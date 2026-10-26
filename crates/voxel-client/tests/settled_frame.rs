//! **The witness for the shaped-run owner**, on the shipped shell's own pixels.
//!
//! Three claims, and the third is the one that matters:
//!
//! 1. a frame that says what the last one said is *recognised* — nothing is shaped and the GPU is
//!    not touched (the renderer's own two measurements say so);
//! 2. the picture it draws is the picture the frame it repeats drew; and
//! 3. a frame that changes **one run** while reusing the rest draws what a renderer that had never
//!    seen the frame before would have drawn. Claims 1 and 2 pass on a cache that never reuses
//!    anything, so on their own they would be a witness to nothing.
//!
//! # Why this is an integration test and not a test in `hud.rs`
//!
//! Because it creates a GPU device and reads pixels back, and the unit-test binary it would share
//! holds `scene`'s measurement of record-append cost — a wall-clock ratio with enough margin for the
//! code and not for a second thread allocating a frame's textures beside it. Measured: with this
//! witness in that binary, that ratio moved from **1.1×** to **5.6×** and failed; in this binary the
//! GPU work is the only thing on the machine and both checks mean what they say.

use voxel_client::hud::{HudState, HudRenderer};
use voxel_client::shell;
use voxel_model::Digest;
use voxel_ui::{Element, Palette, Rect};

/// The shell the game ships, so the witness is about the interface a player sees.
const SHELL: &str = include_str!("../../../packages/alpha-shell.shspec");
const W: u32 = 1280;
const H: u32 = 720;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

struct Target {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    readback: wgpu::Buffer,
    padded_row_bytes: u32,
}

fn target(device: &wgpu::Device) -> Target {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("settled frame"),
        size: wgpu::Extent3d { width: W, height: H, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let padded_row_bytes = (W * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
        * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("settled frame readback"),
        size: u64::from(padded_row_bytes) * u64::from(H),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    Target { texture, view, readback, padded_row_bytes }
}

/// Draw what `hud` holds into `target` over black, and read the pixels back.
fn shot(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    hud: &mut HudRenderer,
    target: &Target,
) -> Vec<[u8; 4]> {
    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("shot") });
    {
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("backdrop"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target.view,
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
    hud.draw(&mut encoder, &target.view);
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &target.texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &target.readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(target.padded_row_bytes),
                rows_per_image: Some(H),
            },
        },
        wgpu::Extent3d { width: W, height: H, depth_or_array_layers: 1 },
    );
    queue.submit(Some(encoder.finish()));

    let slice = target.readback.slice(..);
    let (sender, receiver) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    device
        .poll(wgpu::PollType::Wait { submission_index: None, timeout: None })
        .expect("poll");
    receiver.recv().expect("the map callback runs").expect("the map succeeds");
    let data = slice.get_mapped_range().expect("the range is mapped");
    let stride = target.padded_row_bytes as usize;
    let mut out = Vec::with_capacity((W * H) as usize);
    for y in 0..H as usize {
        for x in 0..W as usize {
            let i = y * stride + x * 4;
            out.push([data[i], data[i + 1], data[i + 2], data[i + 3]]);
        }
    }
    drop(data);
    target.readback.unmap();
    out
}

/// The shell's own frame, composed from readings a live one would have.
fn elements(mount: &mut shell::Mount) -> Vec<Element> {
    let frame = voxel_work::Frame {
        wanted: 605,
        resident: 605,
        bytes: 132 * 1024 * 1024,
        ceiling: 256 * 1024 * 1024,
        scratch_bytes: 0,
        in_flight: 0,
        ..Default::default()
    };
    let state = HudState {
        frame: &frame,
        adapter: "an offscreen adapter",
        workers: 15,
        fps: Some(78.0),
        position_m: [0.0, 45.0, -160.0],
        grounded: true,
        vertical_m_s: 0.0,
        in_water: false,
        quads: 57_175,
        seed: 2468,
        tick: 9_432,
        digest: Digest::of(b"the resident world"),
        refusal: None,
        shading: voxel_client::shading::ShadingReport { materials: 7, with_base_colour: 7 },
    };
    let views = shell::hud::views(&state, mount.console(), mount.api_view().cloned());
    shell::hud::frame(mount, &Palette::declared(), mount.viewport(), &views)
        .expect("the shell layer lays out")
}

#[test]
fn a_settled_frame_draws_the_same_pixels_as_the_frame_it_repeats() {
    let (device, queue, _adapter) = voxel_client::renderer::offscreen_device().expect("a GPU");
    let package = voxel_package::recipe::compile(SHELL.as_bytes()).expect("the shell compiles");
    let loaded = voxel_package::recipe::load(&package).expect("the shell loads");
    let mut mount = shell::Mount::new(&loaded).expect("the shell mounts");
    mount.set_viewport(Rect::new(0.0, 0.0, f64::from(W), f64::from(H)));
    let frame = elements(&mut mount);
    let target = target(&device);

    let mut hud = HudRenderer::new(&device, &queue, FORMAT);
    hud.prepare(&device, &queue, (W, H), 1.0, &frame, &[]).expect("the shell prepares");
    let runs = frame.iter().filter(|element| element.text().is_some()).count();
    assert!(runs > 10, "the shell's own frame is text: {runs} runs");
    let shaped = hud.shaped_runs();
    println!("the shell's frame: {shaped} shape(s) for {runs} run(s)");
    assert!(
        shaped > 10,
        "the first frame shapes the shell's text: {shaped} of {runs} runs"
    );
    let first = shot(&device, &queue, &mut hud, &target);

    hud.prepare(&device, &queue, (W, H), 1.0, &frame, &[]).expect("the repeat prepares");
    assert_eq!(hud.settled_frames(), 1, "a frame that says the same thing is recognised");
    assert_eq!(hud.shaped_runs(), 0, "and it shapes nothing");
    let repeated = shot(&device, &queue, &mut hud, &target);
    assert_eq!(repeated, first, "a settled frame is the same picture, pixel for pixel");

    // One counter moves, which is the frame the game actually draws.
    // The streamer window's mesh readout, which moves every frame of a live run.
    let mut changed = frame.clone();
    let mut touched = false;
    for element in &mut changed {
        if let Element::Text { text, .. } = element {
            if text.contains(" / 256.0 MB") {
                *text = "133.9 / 256.0 MB".to_string();
                touched = true;
                break;
            }
        }
    }
    assert!(touched, "the shell's streamer window reports its mesh against the ceiling");
    hud.prepare(&device, &queue, (W, H), 1.0, &changed, &[]).expect("the changed frame prepares");
    assert_eq!(hud.shaped_runs(), 1, "one run was shaped, not every run");
    let incremental = shot(&device, &queue, &mut hud, &target);

    let mut fresh = HudRenderer::new(&device, &queue, FORMAT);
    fresh
        .prepare(&device, &queue, (W, H), 1.0, &changed, &[])
        .expect("a renderer that has never drawn this prepares");
    let scratch = shot(&device, &queue, &mut fresh, &target);
    assert_eq!(
        incremental, scratch,
        "reused bands and a from-scratch prepare drew the same pixels"
    );
    assert_ne!(incremental, first, "and the changed run changed the picture");
}
