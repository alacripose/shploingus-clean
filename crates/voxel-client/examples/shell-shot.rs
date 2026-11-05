//! Throwaway proof: the shipped shell through the game's own draw path, to PNG.

use voxel_client::hud::{HudRenderer, HudState};
use voxel_client::shell::console::Level;
use voxel_client::{shell, shell::cells, shell::hud};
use voxel_model::Digest;
use voxel_ui::{Palette, Rect};

const SHELL: &str = include_str!("../../../packages/alpha-shell.shspec");
const DEFAULT_W: u32 = 1280;
const DEFAULT_H: u32 = 720;

/// **The window size is the shot's**, so the same camera can be captured at two sizes: `SIZE=1080`
/// draws 1920 x 1080 and `SIZE=720` (the default) draws 1280 x 720, with the map's camera untouched.
fn size() -> (u32, u32) {
    match std::env::var("SIZE").ok().as_deref() {
        Some("1080") => (1920, 1080),
        Some("480") => (480, 440),
        _ => (DEFAULT_W, DEFAULT_H),
    }
}

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

fn main() {
    let (w, h) = size();
    let (device, queue, _adapter) = voxel_client::renderer::offscreen_device().expect("a GPU");
    let package = voxel_package::recipe::compile(SHELL.as_bytes()).expect("compiles");
    let loaded = voxel_package::recipe::load(&package).expect("loads");
    let mut mount = shell::Mount::new(&loaded).expect("mounts");
    mount.set_viewport(Rect::new(0.0, 0.0, w as f64, h as f64));
    // A visual check of the map window's cell frame, with the display package's artifacts.
    if std::env::var("MAP").is_ok() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("the workspace root");
        mount
            .load_package(&root.join("display-packages/alpha-shell/package.shspec"))
            .expect("the display package mounts");
        mount.open("map-plan").expect("the map window opens");
        // **`MAX=1` is the window the whole screen is given to**, which is the size a body whose
        // rectangle came from a declared minimum rather than a resized desktop used to be caught at.
        if std::env::var("MAX").is_ok() {
            mount.toggle_maximize("map-plan");
        }
        // **`PLAN=<token>` is a view selection in the shot**, so a capture can show what the
        // gutter's buttons do to the picture rather than only what a test says they do.
        if let Ok(token) = std::env::var("PLAN") {
            let selected = mount
                .select_feature(&token)
                .expect("the package declares that view");
            println!("plan: {selected}");
        }
    }

    let mode = std::env::args().nth(1).unwrap_or_default();
    if mode != "" {
        for (level, source, text) in [
            (Level::Event, "gate", "16 windows · 3 pinned"),
            (Level::Event, "shell", "shell opened"),
            (
                Level::Warn,
                "laya",
                "laya-0202 refused crates/voxel-package/src/projection.rs",
            ),
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
        shading: voxel_client::shading::ShadingReport {
            materials: 7,
            with_base_colour: 7,
        },
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
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
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
                hud.prepare(&device, &queue, (w, h), 1.0, &runs, &[])
                    .expect("prepares");
                sum[2] += mark.elapsed().as_secs_f64();
                let mark = std::time::Instant::now();
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                hud.draw(&mut encoder, &bench_view);
                sum[3] += mark.elapsed().as_secs_f64();
                let mark = std::time::Instant::now();
                queue.submit(Some(encoder.finish()));
                device
                    .poll(wgpu::PollType::Wait {
                        submission_index: None,
                        timeout: None,
                    })
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
                hud.prepare(&device, &queue, (w, h), 1.0, &changed, &[])
                    .expect("prepares");
                sum += mark.elapsed().as_secs_f64();
            }
            changed_best = changed_best.min(sum / f64::from(rounds));
        }
        println!(
            "shell: prepare, one run changed {:.3} ms over {counter} of {} runs",
            changed_best * 1e3,
            views_elements
                .iter()
                .filter(|element| element.text().is_some())
                .count()
        );
    }
    // **The map window's survey, from the streamer's own voxels.** A live world is settled over the
    // column the frame declares and handed to the window as its ground; the window asks for the
    // plan its camera and body imply and writes that world's own surface over the laws' field. This
    // is the same call the game makes, so the capture is the map and not a stand-in for it.
    let mapped = std::env::var("MAP").is_ok();
    if mapped {
        let declaration = mount
            .frame_of("map-plan")
            .and_then(|frame| frame.map.clone())
            .expect("the package declares a map frame");
        let spec = voxel_shapers::WorldSpec::new(
            voxel_client::game::GAME_SEED,
            voxel_shapers::Config::default(),
        );
        let registry = voxel_shapers::materials::registry().expect("the material registry");
        let config = voxel_mesh::MeshConfig::from_registry(&registry).expect("a mesh config");
        let centre = voxel_model::ChunkPos::new(
            (declaration.centre_xz[0] / 32.0).floor() as i32,
            0,
            (declaration.centre_xz[1] / 32.0).floor() as i32,
        );
        let mut streamer = voxel_work::Streamer::new(
            voxel_stream::WorldSource::new(spec.clone()),
            config,
            std::thread::available_parallelism().map_or(4, |threads| threads.get()),
            512 * 1024 * 1024,
        )
        .expect("a streamer")
        .with_window(voxel_work::ChunkWindow {
            radius_chunks: 2,
            vertical_chunks: 1,
        })
        .expect("a window");
        streamer.set_centre(centre);
        let settled = streamer.settle().expect("the world settles");
        println!(
            "streamer: {} chunks resident over {centre:?}",
            settled.resident
        );
        // **`NEAR=1` is the world's own patch**: the camera close enough that the survey is the
        // resident chunks rather than the laws a few hundred metres out — the picture that shows
        // what the map draws from the world it is standing in.
        if std::env::var("NEAR").is_ok() {
            mount.map.camera_to(voxel_client::map_view::MapCamera {
                centre_m: [
                    f64::from(centre.x) * 32.0 + 16.0,
                    f64::from(centre.z) * 32.0 + 16.0,
                ],
                pixels_per_m: 2.0,
                ..voxel_client::map_view::MapCamera::default()
            });
        }
        // **`FAR=1` is the wide view**: the same middle at a quarter of a pixel per metre, so the
        // body shows kilometres rather than metres and almost none of it is the resident window.
        // It is the view the world's own generator read exists for — a picture whose far ground
        // used to be the elevation law.
        if std::env::var("FAR").is_ok() {
            mount.map.camera_to(voxel_client::map_view::MapCamera {
                centre_m: [
                    f64::from(centre.x) * 32.0 + 16.0,
                    f64::from(centre.z) * 32.0 + 16.0,
                ],
                pixels_per_m: 0.25,
                ..voxel_client::map_view::MapCamera::default()
            });
        }
        // **The player's own pose**: the declared column, looking south-east, so the arrow and the
        // player's cell are in the picture and the window is not centred on them.
        let observe_at = std::time::Instant::now();
        let mut observation = mount.map.observe(
            voxel_client::map_survey::MapWorld {
                ground: Some(&streamer),
                spec: &spec,
                tick: voxel_client::game::GAME_TICK,
            },
            Some([declaration.centre_xz[0], declaration.centre_xz[1], 135.0]),
            1.0,
            &mount.map_chrome(),
        );
        // **The survey is filled the way the game fills it** — one observe per ground write until
        // every sample is the world's own — because a capture of the first band alone would show
        // the law's field beyond it, which is the picture this read exists to remove. The band's
        // own budget is the survey's, so this is the same call the frame loop makes, repeated.
        let mut writes = 1usize;
        let fill_at = std::time::Instant::now();
        while observation.world_samples() < observation.samples && writes < 512 {
            observation = mount.map.observe(
                voxel_client::map_survey::MapWorld {
                    ground: Some(&streamer),
                    spec: &spec,
                    tick: voxel_client::game::GAME_TICK,
                },
                Some([declaration.centre_xz[0], declaration.centre_xz[1], 135.0]),
                voxel_client::map_survey::MapSurvey::GROUND_INTERVAL_S,
                &mount.map_chrome(),
            );
            writes += 1;
        }
        println!(
            "survey: {} of {} samples from the world's own ground — {} resident, {} asked of the generator ({} still the law's){} — {writes} ground write(s) in {:.1} ms",
            observation.world_samples(),
            observation.samples,
            observation.from_chunks,
            observation.from_world,
            observation.samples.saturating_sub(observation.world_samples()),
            if observation.law { " · laws resampled" } else { "" },
            observe_at.elapsed().as_secs_f64() * 1e3,
        );
        println!(
            "survey fill: {:.1} ms for the whole survey",
            fill_at.elapsed().as_secs_f64() * 1e3,
        );
        let camera = mount.map.camera();
        println!(
            "map camera: centre {:?} · {:.4} px/m · bearing {:.0}° · pitch {:.0}°",
            camera.centre_m, camera.pixels_per_m, camera.bearing_deg, camera.pitch_deg
        );
        println!(
            "map rect: body {:?} · window {:?}",
            mount.map.view_rect(&mount.map_chrome()),
            mount.rect_of("map-plan")
        );
        // **The selection is off the survey on purpose**: the route leaves the player and stops at
        // the edge, and the edge indicator says which way the chosen cell lies. A capture that only
        // showed an on-map selection would not exercise the clamp.
        mount.select_map(Some([
            camera.centre_m[0] + declaration.cell_m * 80.0,
            camera.centre_m[1],
        ]));
        // **The pointer, at a known cell**, so the toolbar's readout and the body's cursor outline
        // are in the picture rather than only in a test.
        let at = mount.map.view_rect(&mount.map_chrome()).map(|rect| {
            (
                rect.x + rect.width_dp * 0.25,
                rect.y + rect.height_dp * 0.75,
            )
        });
        if let Some(at) = at {
            mount.map.hover(at, &mount.map_chrome());
        }
        if let Some(terrain) = mount.map.terrain() {
            println!(
                "map: {} × {} samples of {:.1} m over x {:.0}..{:.0} · z {:.0}..{:.0} · player {:?} · arrow {}",
                terrain.columns,
                terrain.rows,
                terrain.cell_m,
                terrain.centre_xz[0] - terrain.columns as f64 / 2.0 * terrain.cell_m,
                terrain.centre_xz[0] + terrain.columns as f64 / 2.0 * terrain.cell_m,
                terrain.centre_xz[1] - terrain.rows as f64 / 2.0 * terrain.cell_row_m,
                terrain.centre_xz[1] + terrain.rows as f64 / 2.0 * terrain.cell_row_m,
                terrain.player_cell(),
                terrain.arrow().glyph()
            );
            println!(
                "map hover: at {at:?} → cell {:?} · cursor {:?}",
                mount.map.cursor_cell(),
                mount
                    .map
                    .facts(&mount.map_chrome())
                    .and_then(|facts| facts.cursor)
            );
        }
    }

    // **The whole shell layer in one call**, the map's own content included — so the number below
    // is the map's raster and the window chrome together, which is the cost a frame pays.
    let compose_at = std::time::Instant::now();
    let composed = hud::frame(&mut mount, &Palette::declared(), viewport, &views).expect("draws");
    println!(
        "composed in {:.1} ms",
        compose_at.elapsed().as_secs_f64() * 1e3
    );
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
                    println!(
                        "text y={:.0} x={:.0} w={:.0} {:?}",
                        rect.y, rect.x, rect.width_dp, text
                    );
                }
            }
        }
        for window in mount.windows() {
            println!(
                "window {} rect {:?} pinnable {}",
                window.spec.id,
                mount.rect_of(&window.spec.id),
                window.spec.pinnable
            );
        }
    }
    // **Both paints, from the same ordered frame**, so the comparison is a picture rather than an
    // argument: `CELLS=1` is the designer's cell surface, and the default is the views' own.
    let cells_on = std::env::var("CELLS").is_ok();
    // **The map is already in this frame**: its ground, its marks, its toolbar and its control
    // gutter are the map window's content, drawn by the mount inside the layer's own window order.
    // This example adds nothing to it, which is the whole point of the change: there is one map
    // renderer and it is the window's.
    let runs = if cells_on || mapped {
        // **The game's own draw path**: every graded frame's cells, and everything else exactly as
        // it was composed. A map's ground is composed ink rather than a palette role — the whole
        // reason it carries a `Paint` of its own — so retinting the frame would show a grey map,
        // and the app never retints one.
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
    hud.prepare(&device, &queue, (w, h), 1.0, &runs, &[])
        .expect("prepares");
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("shot"),
        size: wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let padded =
        (w * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: u64::from(padded) * u64::from(h),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
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
                rows_per_image: Some(h),
            },
        },
        wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(encoder.finish()));
    let slice = readback.slice(..);
    let (sender, receiver) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        })
        .expect("poll");
    receiver.recv().expect("callback").expect("map");
    let data = slice.get_mapped_range().expect("mapped");
    let mut bytes = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h as usize {
        for x in 0..w as usize {
            let i = y * padded as usize + x * 4;
            bytes.extend_from_slice(&data[i..i + 4]);
        }
    }
    drop(data);
    readback.unmap();

    let suffix = if mapped {
        "-map"
    } else if cells_on {
        "-cells"
    } else {
        ""
    };
    let sized = if (w, h) == (DEFAULT_W, DEFAULT_H) {
        String::new()
    } else {
        format!("-{w}x{h}")
    };
    let path = if mode.is_empty() {
        format!("target/shell-shot{suffix}{sized}.png")
    } else {
        format!("target/shell-shot-{mode}{suffix}{sized}.png")
    };
    let file = std::fs::File::create(&path).expect("a file");
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), w, h);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .expect("header")
        .write_image_data(&bytes)
        .expect("pixels");
    println!("wrote {path}");
}
