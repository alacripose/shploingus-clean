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
    let registry = voxel_shapers::column::materials::registry().expect("the registry");
    let field = voxel_shapers::column::materials::shading_field().expect("the shading field");
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

    let peak = skylight::peak_sun_luminance(&spec, SkyInputs::default());
    let table = skylight::sky_dome_table(&daylight.sky, SkyInputs::default(), peak);
    world.write_sky(&table).expect("the sky table fits");
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
    let eye = [0.0, 20.0, 0.0];
    let mut camera = Camera::standard(eye, WIDTH as f64 / HEIGHT as f64);
    camera.pitch_rad = 0.5;
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
    let scene = Scene::build_with_detail(Vec::<voxel_mesh::Mesh>::new().iter(), eye, &DetailLattice::new(0))
        .with_clouds(&cubes);

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
            texture: &colour,
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
    let pixels: Vec<u8> = slice
        .get_mapped_range()
        .expect("the mapped range")
        .to_vec();
    readback.unmap();

    // `Bgra8UnormSrgb`, so the channels are the other way round from the file's own order.
    let mut rgba = Vec::with_capacity(pixels.len());
    for pixel in pixels.chunks_exact(4) {
        rgba.extend_from_slice(&[pixel[2], pixel[1], pixel[0], 255]);
    }
    let path = std::path::Path::new("target/clouds.png");
    let file = std::fs::File::create(path).expect("a file to write into target/");
    let mut png = png::Encoder::new(std::io::BufWriter::new(file), WIDTH, HEIGHT);
    png.set_color(png::ColorType::Rgba);
    png.set_depth(png::BitDepth::Eight);
    png.write_header()
        .expect("a PNG header")
        .write_image_data(&rgba)
        .expect("the pixels");
    println!(
        "wrote {}",
        std::env::current_dir()
            .map(|d| d.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
            .display()
    );
}
