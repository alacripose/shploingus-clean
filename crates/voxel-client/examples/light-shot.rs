//! **The world's own light, as a picture** — `cargo run --offline --release -p voxel-client
//! --example light-shot` writes `target/light-rig.png` and `target/light-measured.png`.
//!
//! The frame is the game's own: the shipped world, the shipped window, the shipped eye and the
//! shipped tick, meshed by `voxel_work::Streamer` and drawn by the same `WorldPipeline` the window
//! draws with. The two pictures differ in **one** thing — whether a corner asks the world's own
//! field for its light or falls back to the rig — and the numbers below say how much of the frame
//! that is.
//!
//! # Why a picture, when the tests measure the field
//!
//! A shadow is a shape, and `voxel-client`'s tests hold the field's *contract* (the address law,
//! the queue, the staleness) rather than its *look*. This is the frame the user judges the look
//! from: the same geometry, one light each, side by side in `target/`.
//!
//! **It also counts the difference**, so the picture cannot be the only evidence: the corner count
//! whose light is a measurement, the spread of the measured direct band, and the fraction of pixels
//! the two frames disagree about.

use std::cell::RefCell;

use voxel_appearance::sky::SkyInputs;
use voxel_client::field::CellField;
use voxel_client::shading::GpuRig;
use voxel_client::{
    offscreen_device_for, skylight, Camera, Scene, WorldPipeline, DEPTH_FORMAT,
};
use voxel_model::VoxelPos;
use voxel_shapers::{Config, WorldSpec};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;
/// `512 × 4` is a multiple of [`wgpu::COPY_BYTES_PER_ROW_ALIGNMENT`] (256), so the readback's rows
/// need no padding — and a padded row that is read back unpadded is a picture that shears.
const WIDTH: u32 = 512;
const HEIGHT: u32 = 288;

fn main() {
    let spec = WorldSpec::new(voxel_client::GAME_SEED, Config::default());
    let inputs = SkyInputs::default();
    // **The first day's own highest Sun**, found in the model rather than chosen: a shadow is the
    // thing this picture is for, and at the shipped start tick the body is a hand's width above the
    // horizon and every shadow is a smear. This is the same search `measured-light` makes.
    let day = voxel_shapers::climate::TICKS_PER_DAY;
    let tick = (0..day)
        .max_by(|a, b| {
            voxel_shapers::sky::solar_position(&spec, *a)
                .altitude_deg
                .total_cmp(&voxel_shapers::sky::solar_position(&spec, *b).altitude_deg)
        })
        .expect("a tick in the first day");
    let daylight = skylight::daylight_of(&spec, tick, inputs, 0)
        .expect("an atmosphere at the highest Sun");
    let grade = daylight.grade;
    println!(
        "sun: tick {tick}, {:.1} deg up at azimuth {:.1} deg",
        daylight.sun_altitude_deg, daylight.sky.sun_azimuth_deg
    );

    // ---- the world, through the streamer the game streams with ---------------------------
    let source = voxel_stream::WorldSource::new(spec);
    let registry = voxel_shapers::materials::registry().expect("the registry");
    let mesh_config = voxel_mesh::MeshConfig::from_registry(&registry).expect("a mesh config");
    let workers = std::thread::available_parallelism()
        .map(|n| n.get().saturating_sub(1).max(1))
        .unwrap_or(1);
    let mut streamer = voxel_work::Streamer::new(
        source,
        mesh_config,
        workers,
        voxel_client::GAME_CEILING_BYTES,
    )
    .and_then(|s| s.with_window(voxel_client::GAME_WINDOW))
    .expect("the streamer");
    let eye = voxel_client::spawn_eye(&voxel_client::game::World::shipped());
    streamer.set_centre(voxel_model::ChunkPos::new(
        (eye[0] / 32.0).floor() as i32,
        (eye[1] / 32.0).floor() as i32,
        (eye[2] / 32.0).floor() as i32,
    ));
    let settled = streamer.settle().expect("the window settles");
    println!(
        "terrain: eye {eye:?}, {} of {} chunks resident, {:.1} MB",
        settled.resident,
        settled.wanted,
        settled.bytes as f64 / (1024.0 * 1024.0)
    );

    // ---- the field: measured on demand, because this tool has no frame to protect ---------
    //
    // The game measures on a worker and the frame only asks; an example has no worker, so it
    // marches a cell the first time a corner asks for it and answers it from the map afterwards.
    // Same march, same address law, same bands.
    let field =
        RefCell::new(CellField::of(&spec, tick, inputs, 0, None).expect("a field at this tick"));
    let started = std::time::Instant::now();
    let light = |point: [f64; 3]| -> Option<[f32; 6]> {
        let cell = voxel_client::SubvoxelPos::of_point(point)?;
        if let Some(bands) = field.borrow_mut().at(cell) {
            return Some(skylight::exposed_bands(bands, grade));
        }
        field
            .borrow_mut()
            .measure(&streamer, cell)
            .map(|(bands, _)| skylight::exposed_bands(bands, grade))
    };
    let measured_scene = Scene::build_with_light(streamer.meshes(), eye, &light, None, None);
    let held = field.borrow().held();
    let marched = field.borrow().adopted();
    println!(
        "field: {} cell(s) held, {} marched in {:.1} s ({:.2} ms a cell), {} corner(s) measured",
        held,
        marched,
        started.elapsed().as_secs_f64(),
        started.elapsed().as_secs_f64() * 1e3 / marched.max(1) as f64,
        measured_scene
            .vertices
            .iter()
            .filter(|v| v.light_present == 1)
            .count()
    );
    // **The rig frame is the same vertices with the flag cleared.** A face's tessellation follows
    // its light — a face whose samples all agree is four corners, one the light crosses is a grid
    // — so a second build with no field is a different array, and the zip this example used to do
    // compared different faces' corners. Clearing the flag keeps one geometry and asks only the
    // shading question the frame is about: the rig's light against the field's.
    let mut rig_scene = measured_scene.clone();
    for vertex in &mut rig_scene.vertices {
        vertex.light_present = 0;
    }
    // A rewrite, not a build: the clone still names the build the measured scene came from, and
    // the pipeline would draw the buffer it already sent for that id.
    rig_scene.rebuilt();

    // ---- what the corners read ----------------------------------------------------------
    let mut direct: Vec<f32> = Vec::new();
    for measured in &measured_scene.vertices {
        if measured.light_present == 1 {
            direct.push(measured.light_direct[0] + measured.light_direct[1] + measured.light_direct[2]);
        }
    }
    direct.sort_by(f32::total_cmp);
    let mean = direct.iter().map(|v| *v as f64).sum::<f64>() / direct.len().max(1) as f64;
    let dark = direct.iter().filter(|v| **v <= 1e-4).count();
    println!(
        "corners: {} of {} carry a measurement; their direct band runs {:.4} .. {:.4} (mean {:.4}), \
         and {dark} of them ({:.1} %) read nothing at all — the world's own shadow",
        direct.len(),
        measured_scene.vertices.len(),
        direct.first().copied().unwrap_or(0.0),
        direct.last().copied().unwrap_or(0.0),
        mean,
        100.0 * dark as f64 / direct.len().max(1) as f64
    );

    // ---- the two frames ----------------------------------------------------------------
    let (device, queue, adapter, _samples) = offscreen_device_for(FORMAT).expect("a GPU");
    // **One sample**, because this frame is a picture and not an anti-aliasing question: the
    // pipeline, its depth attachment and the target all draw at the same count, which is the
    // property that matters when two frames are compared pixel for pixel.
    let samples = 1;
    println!("adapter: {adapter}, {FORMAT:?} at {samples} sample(s)");
    let field_table = voxel_shapers::materials::shading_field().expect("the shading field");
    let materials = voxel_client::table(&registry, &field_table).expect("every material derives");
    let gpu_rig = GpuRig::from_rig(&skylight::exposed_at(&daylight.rig, grade))
        .expect("the rig fits");
    let mut world = WorldPipeline::new(&device, &queue, FORMAT, &materials, &gpu_rig, samples)
        .expect("the world pipeline");
    let peak = skylight::peak_sun_luminance(&spec, inputs);
    let air = voxel_client::sun::air_at(&spec, eye[0], eye[2], tick);
    world.write_sky_model(&skylight::SkyModel::of(
        &daylight.sky,
        &air,
        peak,
        &voxel_shapers::sky::moon_position(&spec, tick, 0),
    ));

    let colour = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("colour"),
        size: wgpu::Extent3d { width: WIDTH, height: HEIGHT, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("depth"),
        size: wgpu::Extent3d { width: WIDTH, height: HEIGHT, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: samples,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let colour_view = colour.create_view(&wgpu::TextureViewDescriptor::default());
    let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());

    let mut camera = Camera::standard(eye, WIDTH as f64 / HEIGHT as f64);
    // A little below the horizon, so the ground that carries the shadows fills more of the frame
    // than a level eye gives it — this is a picture of the terrain's own light, not of the sky.
    camera.pitch_rad = -0.22;
    let pixel_angle = voxel_client::pixel_angle_rad(camera.fov_y_rad, HEIGHT);
    let mut render = |scene: &Scene, label: &'static str| -> Vec<u8> {
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(label) });
        world.draw(&mut encoder, &colour_view, &depth_view, None, scene, &camera, pixel_angle);
        queue.submit(Some(encoder.finish()));
        read_back(&device, &queue, &colour)
    };
    let rigged = render(&rig_scene, "rig");
    let lit = render(&measured_scene, "measured");
    write_png("target/light-rig.png", &rigged);
    write_png("target/light-measured.png", &lit);

    // ---- and how much of the picture moved ---------------------------------------------
    let (a, tail_a) = rigged.as_chunks::<4>();
    let (b, tail_b) = lit.as_chunks::<4>();
    assert!(tail_a.is_empty() && tail_b.is_empty(), "a frame that is not whole pixels");
    let mut differing = 0usize;
    let mut total = 0i64;
    for (rig, lit) in a.iter().zip(b) {
        if rig[..3] != lit[..3] {
            differing += 1;
            for c in 0..3 {
                total += (rig[c] as i64 - lit[c] as i64).abs();
            }
        }
    }
    println!(
        "frames: {} of {} pixel(s) differ ({:.1} %), mean |Δ| {:.1} of 255 over the differing ones; \
         wrote target/light-rig.png and target/light-measured.png",
        differing,
        a.len(),
        100.0 * differing as f64 / a.len() as f64,
        total as f64 / (differing.max(1) * 3) as f64
    );
}

fn read_back(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture) -> Vec<u8> {
    let bytes_per_row = WIDTH * 4;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("light-shot readback"),
        size: (bytes_per_row * HEIGHT) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("readback") });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(HEIGHT),
            },
        },
        wgpu::Extent3d { width: WIDTH, height: HEIGHT, depth_or_array_layers: 1 },
    );
    queue.submit(Some(encoder.finish()));
    let slice = readback.slice(..);
    let (sender, receiver) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    device
        .poll(wgpu::PollType::Wait { submission_index: None, timeout: None })
        .expect("the readback poll");
    receiver.recv().expect("the map callback").expect("the map");
    let pixels: Vec<u8> = slice.get_mapped_range().expect("the mapped range").to_vec();
    readback.unmap();
    let mut rgba = Vec::with_capacity(pixels.len());
    for pixel in pixels.as_chunks::<4>().0 {
        rgba.extend_from_slice(&[pixel[2], pixel[1], pixel[0], 255]);
    }
    rgba
}

fn write_png(path: &str, rgba: &[u8]) {
    let path = std::path::Path::new(path);
    let file = std::fs::File::create(path).expect("a file to write into target/");
    let mut png = png::Encoder::new(std::io::BufWriter::new(file), WIDTH, HEIGHT);
    png.set_color(png::ColorType::Rgba);
    png.set_depth(png::BitDepth::Eight);
    png.write_header()
        .expect("a PNG header")
        .write_image_data(rgba)
        .expect("the pixels");
}
