//! **Does the world pipeline validate on the window's own surface format?**
//!
//! The offscreen tests draw into `Rgba32Float`. The window draws into an 8-bit `sRGB` surface,
//! and a pipeline can validate on one format and not the other. This builds the same
//! [`WorldPipeline`] on `Bgra8UnormSrgb`, draws a frame into a target of that format, and
//! reports every wgpu validation error the draw produced.
//!
//! ```text
//! cargo run --offline --release -p voxel-client --example surface-format-check
//! ```
//!
//! # And the two pictures this format is the only one that can carry
//!
//! `target/clouds.png` is the shaded frame — the game's own field at the shipped tick, with the
//! veil blended. `target/wireframe.png` is the same frame in the `F4` view, where the world's own
//! materials stand aside and the greedy merge's rectangles are outlined. Both are written because
//! a shape is a picture's question and not an assertion's: the tests measure the geometry, and
//! these are what a person judges it from.

use voxel_appearance::sky::SkyInputs;
use voxel_appearance::DetailLattice;

use voxel_client::shading::GpuRig;
use voxel_client::{offscreen_device, skylight, Camera, Scene, WorldPipeline, DEPTH_FORMAT, GAME_TICK};
use voxel_shapers::{Config, WorldSpec};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;
const WIDTH: u32 = 320;
const HEIGHT: u32 = 180;

fn main() {
    let (device, queue, adapter) = offscreen_device().expect("a GPU");
    println!("adapter: {adapter}, target format {FORMAT:?}");

    let spec = WorldSpec::new(voxel_client::GAME_SEED, Config::default());
    let registry = voxel_shapers::materials::registry().expect("the registry");
    let field = voxel_shapers::materials::shading_field().expect("the shading field");
    let materials = voxel_client::table(&registry, &field).expect("every admitted material derives");

    // Error scopes around the pipeline build, so a refusal is named rather than a panic.
    let build_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let daylight = skylight::daylight_of(&spec, GAME_TICK, SkyInputs::default(), 0)
        .expect("an atmosphere at the shipped tick");
    let gpu_rig = GpuRig::from_rig(&skylight::exposed(&daylight.rig)).expect("the rig fits");
    let mut world = WorldPipeline::new(&device, &queue, FORMAT, &materials, &gpu_rig)
        .expect("the world pipeline builds on the surface format");
    let build_error = pollster::block_on(build_scope.pop());
    println!("pipeline build validation: {build_error:?}");

    let inputs = SkyInputs::default();
    let peak = skylight::peak_sun_luminance(&spec, inputs);
    let air = voxel_client::sun::air_at(&spec, 0.0, 0.0, GAME_TICK);
    world.write_sky_model(&skylight::SkyModel::of(
        &daylight.sky,
        &air,
        peak,
        &voxel_shapers::sky::moon_position(&spec, GAME_TICK, 0),
    ));
    let colour = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("surface-format-check colour"),
        size: wgpu::Extent3d { width: WIDTH, height: HEIGHT, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("surface-format-check depth"),
        size: wgpu::Extent3d { width: WIDTH, height: HEIGHT, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let colour_view = colour.create_view(&wgpu::TextureViewDescriptor::default());
    let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());

    // **And the cloud, because this is the only target the veil blends on.** `veil_blend` is
    // `ALPHA_BLENDING` exactly where `format.is_srgb()`, so the offscreen tests — which draw into
    // `Rgba32Float`, where blending is not a property the format has — never exercise that state.
    // A cloud in this frame is what validates it, and the field is the game's own: sampled where
    // this camera stands, on the shipped tick.
    // **The terrain, because `F4` is one frame with two scales in it.** The game appends the cloud
    // to the world's own records and outlines both, so a debug view of the cloud alone would answer
    // a question nobody asks: what the toggle shows is the meshes of the ground *and* the sky above
    // it, and the hue ramp has to carry both. The streamer below is the game's own path —
    // `voxel_work::Streamer` over `voxel_stream::WorldSource` on its shipped window — so this frame
    // is the frame the toggle draws, not an approximation of it. About twenty seconds of meshing.
    let world_spec = WorldSpec::new(voxel_client::GAME_SEED, Config::default());
    let source = voxel_stream::WorldSource::new(world_spec);
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
        "terrain: eye {eye:?}, {} of {} chunks resident, {} meshes held, {:.1} MB",
        settled.resident,
        settled.wanted,
        streamer.len(),
        settled.bytes as f64 / (1024.0 * 1024.0)
    );

    let mut camera = Camera::standard(eye, WIDTH as f64 / HEIGHT as f64);
    // The game's own field of view and pitch, so the frame is the frame the toggle draws.
    camera.pitch_rad = voxel_client::GAME_PITCH_RAD;
    let pixel_angle = voxel_client::pixel_angle_rad(camera.fov_y_rad, HEIGHT);
    let clouds = voxel_client::CloudField::sample(&spec, GAME_TICK, [eye[0], eye[2]]);
    let cubes = voxel_client::CloudCubes::build(&clouds, eye, pixel_angle as f64);
    println!(
        "cloud: {} cloudy columns -> {} cells, {} records, {} origins, rung {:.1} m",
        clouds.cloudy,
        cubes.cells,
        cubes.records.len(),
        cubes.origins.len(),
        cubes.cell_m
    );
    // The same frame **without** the cloud as well: the `F4` view's own pixels are counted
    // against it below, which is a measurement of the outlines rather than a guess at a colour.
    let bare = Scene::build_with_detail(streamer.meshes(), eye, &DetailLattice::new(0));
    let terrain_records = bare.records.len();
    let scene = bare.clone().with_clouds(&cubes);
    println!(
        "frame: {} terrain rectangles + {} cloud rectangles = {} outlined",
        terrain_records,
        cubes.records.len(),
        scene.records.len()
    );

    let draw_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("check") });
    world.draw(&mut encoder, &colour_view, &depth_view, &scene, &camera, pixel_angle);
    queue.submit(Some(encoder.finish()));
    let draw_error = pollster::block_on(draw_scope.pop());
    println!("draw validation: {draw_error:?}");

    match draw_error {
        None => println!("OK: the world pipeline draws on {FORMAT:?} with no validation error"),
        Some(error) => println!("FAIL: {error}"),
    }

    // ---- and the pixels, so the cloud can be looked at rather than asserted about -------
    //
    // **This is the only frame in the tree with the veil blended**, because blending is a property
    // of the target's format and this is the window's. A PNG of it is what the user judges a
    // cloud's *shape* from: the geometry is measured by the tests, and whether the cut reads as a
    // cloud is a picture's question.
    let shaded = read_back(&device, &queue, &colour);
    write_png("target/clouds.png", &shaded);

    // ---- and the `F4` view, which is the same frame with the world's materials set aside ------
    //
    // **The debug view is a picture's question too, and the pass is the only way to answer it**:
    // what the merge produced is a count until it is outlined. The lines are the records
    // themselves, so this is the one frame in the tree where the greedy merge's own rectangles are
    // the subject — the world's own materials are withheld, and the sky stays so the lines have a
    // horizon to be read against.
    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("wire") });
    world.draw_wireframe(&mut encoder, &colour_view, &depth_view, &scene, &camera, pixel_angle);
    queue.submit(Some(encoder.finish()));
    let wire = read_back(&device, &queue, &colour);
    write_png("target/wireframe.png", &wire);

    // **How many pixels the outlines are, without guessing at a colour.** Two reference frames do
    // it, because this view carries two subjects and they have to be told apart:
    //
    // * **Short of the cloud only** — the same view without it draws the same sky and the same
    //   terrain, so the frames differ exactly where a *cloud* rectangle's border lies.
    // * **Short of every rectangle** — no terrain and no cloud — which is the whole set: this is
    //   where the terrain's own lines are, and they are the majority of them.
    //
    // Both are the same sky, so a difference is a border and not a change of light.
    let mut reference = |scene: &Scene, label: &'static str| {
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(label) });
        world.draw_wireframe(&mut encoder, &colour_view, &depth_view, scene, &camera, pixel_angle);
        queue.submit(Some(encoder.finish()));
        read_back(&device, &queue, &colour)
    };
    let bare_wire = reference(&bare, "bare");
    let nothing = Scene::build_with_detail(
        Vec::<voxel_mesh::Mesh>::new().iter(),
        eye,
        &DetailLattice::new(0),
    );
    let cloud_only = nothing.clone().with_clouds(&cubes);
    let cloud_only_wire = reference(&cloud_only, "cloud-only");
    let nothing_wire = reference(&nothing, "nothing");

    // **`as_chunks` rather than `chunks_exact`**, so the pixel width is in the type: a slice that
    // is not a whole number of pixels is a *tail*, and a tail dropped in silence would be a
    // measurement that skipped its last few bytes.
    let differing = |reference: &[u8]| -> Vec<[u8; 4]> {
        let (wire_pixels, wire_tail) = wire.as_chunks::<4>();
        let (reference_pixels, reference_tail) = reference.as_chunks::<4>();
        assert!(
            wire_tail.is_empty() && reference_tail.is_empty(),
            "a frame that is not a whole number of pixels: {wire_tail:?} / {reference_tail:?} left over"
        );
        wire_pixels
            .iter()
            .zip(reference_pixels)
            .filter(|(wire, reference)| wire[..3] != reference[..3])
            .map(|(wire, _)| *wire)
            .collect()
    };
    let cloud_lines = differing(&bare_wire);
    let terrain_lines = differing(&cloud_only_wire);
    let all_lines = differing(&nothing_wire);

    // **And what the ramp actually did**, because a hue with no range in it is invisible. The
    // outlines split into the warm half of the sweep and the cool half: with a ramp in octaves the
    // terrain (metres to a few hundred) reads warm and the cloud (kilometres) cool, which is the
    // whole point of measuring them separately. The terrain mesh's own linear 4 m to 256 m ramp put
    // the **cloud** entirely in the far end — one flat violet, measured — and widening its far end
    // to the field's 8 km then put the **terrain** entirely in the near end, which is the same
    // fault from the other side.
    let warm = |pixels: &[[u8; 4]]| pixels.iter().filter(|p| p[0] > p[2]).count();
    println!(
        "F4 view: {} rectangles outlined ({} terrain + {} cloud), {} of {} pixels are the outlines \
         themselves ({} terrain + {} cloud); warm/cool is {} / {} over the terrain's lines and \
         {} / {} over the cloud's; wrote target/clouds.png and target/wireframe.png",
        scene.records.len(),
        terrain_records,
        cubes.records.len(),
        all_lines.len(),
        WIDTH * HEIGHT,
        terrain_lines.len(),
        cloud_lines.len(),
        warm(&terrain_lines),
        terrain_lines.len() - warm(&terrain_lines),
        warm(&cloud_lines),
        cloud_lines.len() - warm(&cloud_lines),
    );
}

/// **The colour texture's pixels, in the order a file wants them.** The target is
/// `Bgra8UnormSrgb`, so the channels are the other way round from the file's own order.
fn read_back(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture) -> Vec<u8> {
    let bytes_per_row = WIDTH * 4;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("surface-format-check readback"),
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
