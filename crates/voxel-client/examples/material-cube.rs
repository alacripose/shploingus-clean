//! **One cube, drawn by the game's own renderer, one picture per material layer.**
//!
//! The user, 2026-10-25: *"create a debug view for viewing detailed gpu highlights for the different
//! layers that happen in the material, create a debug probe viewer that renders out a cube with the
//! game's actual rendering for viewing frames"*.
//!
//! ```text
//! cargo run --offline --release -p voxel-client --example material-cube
//! ```
//!
//! # What it is, and what it is not
//!
//! It is [`Renderer`] — the same pipeline, the same `shading.wgsl`, the same material table the
//! window builds (`shading::table` over the shipped registry) — pointed at **one block**: six
//! quads, one metre, either plain or refined into the pillar's own cap. The eye is fixed and the
//! cube is the only geometry in the scene, so a layer view is a statement about the material and
//! nothing else: no streaming, no LOD ladder moving under it, no second light.
//!
//! It is **not** a CPU transcription of the shader. The pixels come off the adapter, through
//! `WorldPipeline::set_debug_layer`'s uniform channel, so a layer that is wrong here is wrong in the
//! window too.
//!
//! # The layers
//!
//! | layer | what the picture states |
//! |---|---|
//! | `0` | the shaded material — what the game draws |
//! | `1` | the **stretch axis** the pattern's stencil shifts along (red x, green y, blue z, magenta = no axis) |
//! | `2` | what the **corner** states of its own (red roughness, green reflectance, blue = no sample) |
//! | `3` | the **ladder**: its own share as brightness, the dominant rung as hue |
//! | `4` | the **cell address** the pattern is read at, hashed |
//!
//! # The assertions, and why they are here rather than in a test
//!
//! A layer view that nobody checks is a view wired to the wrong channel — the same failure `F4` had
//! before `Renderer::render_wireframe` existed. So this example **asserts** the two claims the layers
//! exist to make, and exits non-zero when one fails:
//!
//! 1. **Every corner of the plain cube states an axis** (layer 1 has no magenta pixels), and the
//!    three visible sides state three different axes.
//! 2. **The refined cube's cut corners state the face they were cut from** too — the defect that
//!    made a face flip between square and stretched cells between two frames — so layer 1 over the
//!    refined cube has no magenta either.
//!
//! The images land in `target/material-cube/`, one PPM per layer per cube.

use voxel_client::camera::Camera;
use voxel_client::renderer::Renderer;
use voxel_client::scene::Scene;
use voxel_client::shading::{self, GpuRig};
use voxel_appearance::npr::LightRig;
use voxel_mesh::{Mesh, MeshStats, Quad, RefineConfig};
use voxel_model::{ChunkPos, DisplacementField, Face, MaterialId, Profile};

/// The shipped rock, which is the material the scene's own tests cut this cube from.
const ROCK: MaterialId = MaterialId(6);

/// Four subvoxels a block (`voxel_model::coord`'s base rate), so a face's plane is `0` or `4`.
const BLOCK_SUBVOXELS: u16 = 4;

/// The target. 320² is enough for a one-metre cube to fill the frame with a pixel to spare, and
/// small enough that a stack of five layers still reads back in well under a second.
const SIDE: u32 = 320;

/// The eye, in metres, looking at the cube's centre from above and in front.
const EYE: [f64; 3] = [1.55, 1.35, -1.55];

/// The cube's centre — the point the eye is turned toward.
const TARGET: [f64; 3] = [0.5, 0.5, 0.5];

fn main() {
    let registry = voxel_shapers::materials::registry().expect("the shipped registry");
    let field = voxel_shapers::materials::shading_field().expect("the world's shading field");
    let table = shading::table(&registry, &field).expect("the world's material table");
    let rig = LightRig::standard();
    let gpu_rig = GpuRig::from_rig(&rig).expect("the standard rig fits one uniform");

    let out = std::path::Path::new("target/material-cube");
    std::fs::create_dir_all(out).expect("the output directory");

    let mut renderer = Renderer::new(SIDE, SIDE, &table, &gpu_rig).expect("a GPU");
    // **A frame with no sky, stated as none.** The layer views are about the material, and a sky
    // would put a second thing in the picture that a census could count; the shader refuses to draw
    // a frame that never said which of the two it is, which is what this call is.
    renderer.world_mut().write_no_sky_model();
    println!("adapter: {}", renderer.adapter);
    println!("material rows: {}", renderer.material_rows());

    // **The eye's orientation is derived from where it stands and what it looks at**, rather than
    // typed as a yaw and a pitch that could disagree with `EYE`.
    let camera = looking_at(EYE, TARGET);

    // ---- the plain cube: six records, the format a face of the world is drawn in ----------
    let plain = Scene::build([&a_cube()], [0.0; 3], None, None);
    assert_eq!(plain.quads, 6, "a cube is six faces");

    // ---- the refined cube: the same six, with the pillar's cap cutting the roof ----------
    let mut cut = DisplacementField::none();
    cut.declare_profile(ROCK, Face::PosY, Profile::RoundColumn { radius: 0.5 })
        .expect("a pillar's cap is inside its own block and is declarable");
    let config = RefineConfig::new(4, 4096).expect("4 samples per block is inside the level");
    let refined = Scene::build([&a_cube()], [0.0; 3], Some(&cut), Some(&config));
    assert_eq!(refined.bent_quads, 1, "the law reached the roof and no other face");
    assert!(
        !refined.vertices.is_empty(),
        "a bent quad is carried as corners, not as a record"
    );

    let mut failures = Vec::new();
    for (name, scene) in [("plain", &plain), ("refined", &refined)] {
        for layer in 0..=4u32 {
            renderer.world_mut().set_debug_layer(layer);
            let frame = renderer.render(scene, &camera).expect("a frame");
            let path = out.join(format!("{name}-layer-{layer}.ppm"));
            write_ppm(&path, &frame);
            println!("{name} layer {layer}: {}", path.display());
            if layer == 1 {
                let axes = axis_census(&frame);
                println!(
                    "  axis census: x {} y {} z {} none {} (covered {} px)",
                    axes[0], axes[1], axes[2], axes[3], axes[4]
                );
                if axes[3] > 0 {
                    failures.push(format!(
                        "{name}: {} corner(s) state no stretch axis (magenta) — layer 1",
                        axes[3]
                    ));
                }
                if axes[4] > 0 && axes[0] + axes[1] + axes[2] == 0 {
                    failures.push(format!("{name}: layer 1 covered no pixel at all"));
                }
                if axes[0] + axes[1] + axes[2] < 2 {
                    failures.push(format!(
                        "{name}: only {} axis/axes on a cube with three visible sides",
                        axes[0] + axes[1] + axes[2]
                    ));
                }
            }
        }
    }
    renderer.world_mut().set_debug_layer(0);

    if failures.is_empty() {
        println!("\nevery corner of both cubes states the face it was cut from");
    } else {
        for failure in &failures {
            eprintln!("FAILED: {failure}");
        }
        std::process::exit(1);
    }
}

/// A one-metre block at the world origin: six faces, four subvoxels a side.
fn a_cube() -> Mesh {
    // The plane: a negative face lies on its cell's low plane, a positive one on its high plane.
    let quads = Face::ALL
        .into_iter()
        .map(|face| Quad {
            face,
            slice: if face.is_positive() { BLOCK_SUBVOXELS } else { 0 },
            uv: [0, 0],
            extent: [BLOCK_SUBVOXELS, BLOCK_SUBVOXELS],
            material: ROCK,
        })
        .collect();
    Mesh {
        chunk: ChunkPos::new(0, 0, 0),
        origin: [0, 0, 0],
        quads,
        loops: Vec::new(),
        stats: MeshStats {
            solid_voxels: 1,
            visible_faces: 6,
            quads: 6,
            loops: 0,
        },
    }
}

/// An eye at `eye` turned to look at `target`, upright, with the crate's own standard lens.
fn looking_at(eye: [f64; 3], target: [f64; 3]) -> Camera {
    let mut camera = Camera::standard(eye, (SIDE as f64) / (SIDE as f64));
    let d = [target[0] - eye[0], target[1] - eye[1], target[2] - eye[2]];
    let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    let f = [d[0] / len, d[1] / len, d[2] / len];
    camera.yaw_rad = f[0].atan2(f[2]);
    camera.pitch_rad = f[1].asin();
    camera
}

/// How many covered pixels state each stretch axis, and how many state none.
///
/// The classification is the shader's own block, read back through the picture it wrote: layer 1
/// paints a saturated red, green or blue from `in.plane_axis`, and magenta where the corner has no
/// face. Anything else is the sky of a clear frame (zero, since no sky is stated here).
fn axis_census(frame: &voxel_client::frame::Frame) -> [usize; 5] {
    let mut counts = [0usize; 5];
    for pixel in &frame.pixels {
        let [r, g, b, a] = *pixel;
        if a < 0.5 {
            continue;
        }
        // A quantised read of a saturated swatch: the layer writes exactly 0 or 1 per channel, and
        // the resolve blends only where an edge crosses the pixel — so an edge pixel is read by its
        // dominant channel.
        let (r, g, b) = (r > 0.5, g > 0.5, b > 0.5);
        let class = match (r, g, b) {
            (true, false, false) => Some(0),
            (false, true, false) => Some(1),
            (false, false, true) => Some(2),
            (true, false, true) => Some(3),
            _ => None,
        };
        match class {
            Some(class) => {
                counts[class] += 1;
                counts[4] += 1;
            }
            // The layer paints nothing at all outside geometry, so a black pixel is the frame's
            // own backdrop rather than a cube pixel that states nothing.
            None if r || g || b => counts[4] += 1,
            None => {}
        }
    }
    counts
}

/// A binary PPM of the frame, display-encoded from the linear value the material computed.
///
/// The offscreen target is a measurement, not a display (no tonemap, no dither), so the encode is
/// here: clamp, then the inverse of the sRGB transfer's linear segment — the same curve the window's
/// `sRGB` surface applies, written out so an image opened in anything looks like the window's.
fn write_ppm(path: &std::path::Path, frame: &voxel_client::frame::Frame) {
    let mut bytes = Vec::with_capacity((frame.width * frame.height * 3) as usize + 32);
    bytes.extend_from_slice(format!("P6\n{} {}\n255\n", frame.width, frame.height).as_bytes());
    for pixel in &frame.pixels {
        for channel in 0..3 {
            let linear = pixel[channel].clamp(0.0, 1.0) as f64;
            let encoded = if linear <= 0.0031308 {
                12.92 * linear
            } else {
                1.055 * linear.powf(1.0 / 2.4) - 0.055
            };
            bytes.push((encoded * 255.0 + 0.5).clamp(0.0, 255.0) as u8);
        }
    }
    std::fs::write(path, bytes).expect("the image");
}
