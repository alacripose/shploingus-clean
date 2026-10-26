//! Throwaway proof: the shipped shell through the game's own draw path, to PNG.

use voxel_client::hud::{HudState, HudRenderer};
use voxel_client::shell::console::Level;
use voxel_client::{shell, shell::cells, shell::hud};
use voxel_model::Digest;
use voxel_ui::{Palette, Rect};

const SHELL: &str = include_str!("../../../packages/alpha-shell.shspec");
const W: u32 = 1280;
const H: u32 = 720;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

fn main() {
    let (device, queue, _adapter) = voxel_client::renderer::offscreen_device().expect("a GPU");
    let package = voxel_package::recipe::compile(SHELL.as_bytes()).expect("compiles");
    let loaded = voxel_package::recipe::load(&package).expect("loads");
    let mut mount = shell::Mount::new(&loaded).expect("mounts");
    mount.set_viewport(Rect::new(0.0, 0.0, W as f64, H as f64));

    let mode = std::env::args().nth(1).unwrap_or_default();
    if mode != "" {
        for (level, source, text) in [
            (Level::Event, "gate", "16 windows · 3 pinned"),
            (Level::Event, "shell", "shell opened"),
            (Level::Warn, "laya", "laya-0202 refused crates/voxel-package/src/projection.rs"),
            (Level::Event, "world", "605 chunks wanted · 291 resident"),
        ] {
            mount.log(level, source, text.to_string(), Some(0));
        }
        let _ = mount.open("console");
    }
    if mode == "overlap" {
        let _ = mount.open("api-viewer");
        let _ = mount.open("inventory");
    }

    let frame = voxel_work::Frame {
        wanted: 605,
        resident: 291,
        bytes: 71 * 1024 * 1024,
        ceiling: 256 * 1024 * 1024,
        scratch_bytes: 72 * 1024 * 1024,
        in_flight: 324,
        ..Default::default()
    };
    let state = HudState {
        frame: &frame,
        adapter: "NVIDIA GeForce RTX 2070 SUPER (Vulkan)",
        workers: 15,
        fps: Some(11.4),
        position_m: [0.0, 45.0, -160.0],
        grounded: true,
        vertical_m_s: 0.0,
        in_water: false,
        quads: 43_794,
        seed: 2468,
        tick: 9_259,
        digest: Digest::of(b"the resident world"),
        refusal: None,
        shading: voxel_client::shading::ShadingReport { materials: 7, with_base_colour: 7 },
    };
    let views = hud::views(&state, mount.console(), mount.api_view().cloned());
    let viewport = mount.viewport();
    if std::env::var("BENCH").is_ok() {
        // The shell's whole frame, split the way the app's own instrument splits it: the compose,
        // the raster, the HUD's preparation, the encoding, and the wait for the GPU.
        let rounds = 60u32;
        let palette = Palette::declared();
        let bench = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bench"),
            size: wgpu::Extent3d { width: W, height: H, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let bench_view = bench.create_view(&wgpu::TextureViewDescriptor::default());
        let views_elements = hud::frame(&mut mount, &palette, viewport, &views).expect("draws");
        let mut hud = HudRenderer::new(&device, &queue, FORMAT);
        let mut best = [f64::INFINITY; 5];
        for _ in 0..3 {
            let mut sum = [0.0f64; 5];
            for _ in 0..rounds {
                let mark = std::time::Instant::now();
                let elements = hud::frame(&mut mount, &palette, viewport, &views).expect("draws");
                sum[0] += mark.elapsed().as_secs_f64();
                let mark = std::time::Instant::now();
                let runs = cells::frames_in(&elements, &mount.frame_space());
                sum[1] += mark.elapsed().as_secs_f64();
                let mark = std::time::Instant::now();
                hud.prepare(&device, &queue, (W, H), 1.0, &runs, &[]).expect("prepares");
                sum[2] += mark.elapsed().as_secs_f64();
                let mark = std::time::Instant::now();
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                hud.draw(&mut encoder, &bench_view);
                sum[3] += mark.elapsed().as_secs_f64();
                let mark = std::time::Instant::now();
                queue.submit(Some(encoder.finish()));
                device
                    .poll(wgpu::PollType::Wait { submission_index: None, timeout: None })
                    .expect("poll");
                sum[4] += mark.elapsed().as_secs_f64();
            }
            for (slot, value) in best.iter_mut().zip(sum) {
                *slot = slot.min(value / f64::from(rounds));
            }
        }
        println!(
            "shell: compose {:.3} · cells {:.3} · prepare {:.3} · encode {:.3} · submit+wait {:.3} ms",
            best[0] * 1e3,
            best[1] * 1e3,
            best[2] * 1e3,
            best[3] * 1e3,
            best[4] * 1e3
        );
        // **The frame a game actually draws**: one readout's number moves and the 44 runs beside it
        // are the ones the last frame already shaped. The loop above repeats one frame verbatim, so
        // it measures the settled path; this measures the other one, which is where the shell spends
        // its time while the world moves.
        let counter = views_elements
            .iter()
            .position(|element| element.text().is_some_and(|text| text.contains('/')))
            .expect("the shell's own frame has a counter in it");
        let mut changed_best = f64::INFINITY;
        for _ in 0..3 {
            let mut sum = 0.0f64;
            for round in 0..rounds {
                let mut changed = cells::retinted(&views_elements);
                if let voxel_ui::Element::Text { text, .. } = &mut changed[counter] {
                    *text = format!("{} / 605", 291 - round % 7);
                }
                let mark = std::time::Instant::now();
                hud.prepare(&device, &queue, (W, H), 1.0, &changed, &[]).expect("prepares");
                sum += mark.elapsed().as_secs_f64();
            }
            changed_best = changed_best.min(sum / f64::from(rounds));
        }
        println!(
            "shell: prepare, one run changed {:.3} ms over {counter} of {} runs",
            changed_best * 1e3,
            views_elements.iter().filter(|element| element.text().is_some()).count()
        );
    }
    let composed = hud::frame(&mut mount, &Palette::declared(), viewport, &views).expect("draws");
    let headers: Vec<&str> = mount
        .windows()
        .filter(|window| window.draws_pane_headers())
        .map(|window| window.spec.id.as_str())
        .collect();
    println!("pane headers drawn: {headers:?}");
    assert!(headers.is_empty());

    if std::env::var("DUMP").is_ok() {
        for element in &composed {
            if let voxel_ui::Element::Text { rect, text, .. } = element {
                if (495.0..525.0).contains(&rect.y) {
                    println!("text y={:.0} x={:.0} w={:.0} {:?}", rect.y, rect.x, rect.width_dp, text);
                }
            }
        }
        for window in mount.windows() {
            println!("window {} rect {:?} pinnable {}", window.spec.id, mount.rect_of(&window.spec.id), window.spec.pinnable);
        }
    }
    // **Both paints, from the same ordered frame**, so the comparison is a picture rather than an
    // argument: `CELLS=1` is the designer's cell surface, and the default is the views' own.
    let cells_on = std::env::var("CELLS").is_ok();
    let runs = if cells_on {
        cells::frames_in(&composed, &mount.frame_space())
    } else {
        cells::retinted(&composed)
    };
    println!(
        "composed {} elements · {} elements out ({})",
        composed.len(),
        runs.len(),
        if cells_on { "cells" } else { "views" }
    );

    let mut hud = HudRenderer::new(&device, &queue, FORMAT);
    hud.prepare(&device, &queue, (W, H), 1.0, &runs, &[]).expect("prepares");
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("shot"),
        size: wgpu::Extent3d { width: W, height: H, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let padded = (W * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
        * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: u64::from(padded) * u64::from(H),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("backdrop"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    }
    hud.draw(&mut encoder, &view);
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: Some(H),
            },
        },
        wgpu::Extent3d { width: W, height: H, depth_or_array_layers: 1 },
    );
    queue.submit(Some(encoder.finish()));
    let slice = readback.slice(..);
    let (sender, receiver) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    device
        .poll(wgpu::PollType::Wait { submission_index: None, timeout: None })
        .expect("poll");
    receiver.recv().expect("callback").expect("map");
    let data = slice.get_mapped_range().expect("mapped");
    let mut bytes = Vec::with_capacity((W * H * 4) as usize);
    for y in 0..H as usize {
        for x in 0..W as usize {
            let i = y * padded as usize + x * 4;
            bytes.extend_from_slice(&data[i..i + 4]);
        }
    }
    drop(data);
    readback.unmap();

    let suffix = if cells_on { "-cells" } else { "" };
    let path = if mode.is_empty() {
        format!("target/shell-shot{suffix}.png")
    } else {
        format!("target/shell-shot-{mode}{suffix}.png")
    };
    let file = std::fs::File::create(&path).expect("a file");
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), W, H);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header().expect("header").write_image_data(&bytes).expect("pixels");
    println!("wrote {path}");
}
