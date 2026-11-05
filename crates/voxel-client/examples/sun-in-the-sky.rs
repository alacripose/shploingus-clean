//! **The Sun in the sky, as a picture** - `cargo run -p voxel-client --offline --example
//! sun-in-the-sky` writes `target/sun-in-the-sky.png`.
//!
//! # Why this exists when a test already measures the body
//!
//! `renderer::tests::the_sun_is_drawn_as_the_cuboid_the_model_says` compares the drawn pixels
//! against the model's own ray test, which is the stronger claim and the one that can fail. It
//! is also a set of numbers. This is the same frame **written where a person can look at it**,
//! which is what the crate's `png` dev-dependency is declared for and what no example did until
//! now.
//!
//! # Three decisions this tool makes, and none of them belong to the pass
//!
//! * **A telescope's field.** The real Sun is 0.53° across, so at a player's 70° field it is
//!   under one pixel of a 216-line frame. `FIELD_DEG` is 6°, which puts the body at about
//!   nineteen pixels wide - a shape rather than a dot.
//! * **The display transform on.** The renderer's target is `Rgba32Float` and measures the
//!   scene's own radiance, which for this sky means a nearly black picture with a white square
//!   in it. `set_tonemap` and `set_dither` are the two things an 8-bit presentation asks for,
//!   and this file is an 8-bit presentation.
//! * **The `sRGB` encode.** `LinearRgb` is deliberately linear and the pass writes linear
//!   values; a window's surface does the encoding in the GPU. A PNG does not, so the transfer
//!   function is applied here - in the tool that knows what it is writing to, which is where
//!   `voxel-appearance::tonemap` says it belongs.

use voxel_appearance::sky::SkyInputs;
use voxel_client::camera::Camera;
use voxel_client::renderer::Renderer;
use voxel_client::scene::Scene;
use voxel_client::shading::GpuRig;
use voxel_client::skylight::{daylight_of, peak_sun_luminance, sun_direction};
use voxel_client::sun::DrawnSun;
use voxel_shapers::{Config, WorldSpec};

/// The vertical field, degrees: a telescope, for the reason the module doc gives.
const FIELD_DEG: f64 = 6.0;
const WIDTH: u32 = 384;
const HEIGHT: u32 = 216;
/// The shipped client's own start tick - low enough in the sky for the body to be seen against
/// a sky with a gradient in it.
const TICK: u64 = 9_000;

/// One channel of `sRGB`, from a linear value a display transform has already bounded.
fn srgb(v: f32) -> u8 {
    let v = v.clamp(0.0, 1.0) as f64;
    let encoded = if v <= 0.003_130_8 {
        12.92 * v
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    };
    (encoded * 255.0 + 0.5) as u8
}

fn main() {
    let spec = WorldSpec::new(2468, Config::default());
    let inputs = SkyInputs::default();
    let day = daylight_of(&spec, TICK, inputs, 0).expect("a sky at the start tick");
    let peak = peak_sun_luminance(&spec, inputs);
    let eye = [0.0; 3];
    let sun = DrawnSun::of(&day.sky, peak, eye).expect("a body at the start tick");

    // **Aimed at the body**, through the same direction function the rig and the shader use.
    let d = sun_direction(day.sky.sun_altitude_deg, day.sky.sun_azimuth_deg);
    let mut camera = Camera::standard(eye, WIDTH as f64 / HEIGHT as f64);
    camera.fov_y_rad = FIELD_DEG.to_radians();
    camera.yaw_rad = d.x.atan2(d.z);
    camera.pitch_rad = d.y.clamp(-1.0, 1.0).asin();

    let registry = voxel_shapers::materials::registry().expect("the registry");
    let materials = voxel_client::shading::table(
        &registry,
        &voxel_shapers::materials::shading_field().expect("the world's shading field"),
    )
    .expect("every admitted material derives");
    let rig = GpuRig::from_rig(&voxel_client::skylight::exposed(&day.rig)).expect("the rig fits");
    let mut renderer = Renderer::new(WIDTH, HEIGHT, &materials, &rig).expect("a GPU adapter");
    // **The model the pass integrates**, over this column's own air: the Sun's bands through the
    // droplets the world's weather carries, normalised by the world's own peak sun.
    let air = voxel_client::sun::air_at(&spec, eye[0], eye[2], TICK);
    renderer
        .world()
        .write_sky_model(&voxel_client::skylight::SkyModel::of(
            &day.sky,
            &air,
            peak,
            &voxel_shapers::sky::moon_position(&spec, TICK, 0),
        ));
    // **The body is a record in the scene** (C-184), not a field of a uniform: what places it is the
    // same array a chunk's own rectangles travel in.
    let bodies = [sun.body_record(eye)];
    // The two the target's own format would have decided for a window, asked for deliberately:
    // this frame is going to an 8-bit file rather than to a measurement.
    renderer.world_mut().set_tonemap(1.0);
    renderer.world_mut().set_dither(1.0);

    let scene =
        Scene::build_with_detail(Vec::<voxel_mesh::Mesh>::new().iter(), eye).with_bodies(&bodies);
    let frame = renderer.render(&scene, &camera).expect("a frame");

    let mut bytes = Vec::with_capacity((WIDTH * HEIGHT * 4) as usize);
    for pixel in frame.pixels.iter() {
        bytes.push(srgb(pixel[0]));
        bytes.push(srgb(pixel[1]));
        bytes.push(srgb(pixel[2]));
        bytes.push(255);
    }

    // Relative to the directory the example is run from, which for `cargo run` is the crate or
    // workspace root — so `target/` is the build directory that already exists.
    let path = std::path::Path::new("target/sun-in-the-sky.png");
    let file = std::fs::File::create(path).expect("a file to write into target/");
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), WIDTH, HEIGHT);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("a PNG header");
    writer.write_image_data(&bytes).expect("the pixels");
    drop(writer);

    // **And the numbers beside the picture**, so the image is not the only evidence: the body's
    // coverage by the model, the brightest pixel the frame carried, and where the Sun was.
    let bright = frame
        .pixels
        .iter()
        .map(|p| voxel_client::frame::luminance(*p))
        .fold(0.0f64, f64::max);
    let half_width = (sun.body.radius_m / voxel_client::sun::AU_M).atan();
    println!(
        "sun-in-the-sky: tick {TICK}, altitude {:.3}°, azimuth {:.3}°, the face's angular \
         half-width {:.5}° ({:.2} pixels of this field), brightest pixel {bright:.3} of the \
         **display-transformed** frame",
        day.sky.sun_altitude_deg,
        day.sky.sun_azimuth_deg,
        half_width.to_degrees(),
        half_width.to_degrees() / FIELD_DEG * HEIGHT as f64,
    );
    println!(
        "wrote {}",
        std::env::current_dir()
            .map(|d| d.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
            .display()
    );

    /// **And the shape as text.** A PNG is where a person looks and a terminal is where a run is
    /// read, and the shape *is* the claim: a **disc** of the radius the body's own figures give it,
    /// because the phase is a per-pixel normal and a cuboid has one normal per visible face. The
    /// window is the frame's centre, which is where the camera was aimed.
    const CUT: f64 = 0.75;
    const HALF: i32 = 16;
    let (cx, cy) = ((WIDTH / 2) as i32, (HEIGHT / 2) as i32);
    let body = |x: i32, y: i32| {
        frame
            .pixel(x as u32, y as u32)
            .map(|p| voxel_client::frame::luminance(p) > CUT)
            .unwrap_or(false)
    };
    let lit = (cy - HALF..=cy + HALF)
        .flat_map(|y| (cx - HALF..=cx + HALF).map(move |x| (x, y)))
        .filter(|(x, y)| body(*x, *y))
        .count();
    println!(
        "the body, a {HALF}-pixel window round the frame's centre, presented luminance over {CUT} \
         (the whole dome's own peak presents as 0.42):"
    );
    for y in (cy - HALF)..=(cy + HALF) {
        let mut row = String::new();
        for x in (cx - HALF)..=(cx + HALF) {
            row.push(if body(x, y) { '#' } else { '.' });
        }
        println!("  {row}");
    }
    println!("  {lit} pixels over {CUT} in the window");
}
