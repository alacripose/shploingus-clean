//! **Does the material pattern flicker when the eye moves?** — two eyes, two steps, one picture.
//!
//! The user's report (2026-10-08): *"there is still visible material flickering when moving on the
//! vertical axis which is now fixed when moving on the horizontal axis"*. This example measures it
//! rather than describing it: the same world, the same camera, drawn from two eyes a stated step
//! apart, with a **vertical** step and a **horizontal** step of the same length, and the per-pixel
//! difference between the frames printed in bands.
//!
//! ```text
//! cargo run --offline --release -p voxel-client --example pattern-flicker
//! ```
//!
//! # What it measures, and why it is the right quantity
//!
//! A step of the eye must move the world across the frame, so *some* pixels change; the question is
//! **how much**, and whether the change is a motion of the pattern or a change of it. A pattern that
//! merely scrolls with the geometry changes by one pixel of pattern per pixel of geometry moved; a
//! pattern whose *rungs* are re-weighted by the step changes everywhere at once. So the numbers here
//! are the difference's **size**: how many pixels changed at all, how many changed by more than one
//! 8-bit level (what the window can show), the mean and the maximum, and the total — the last of
//! which is the "flicker energy" a rung re-weight moves and a scroll does not.
//!
//! **The horizontal step is the control**, and it is the user's own datum: it is fixed, so whatever
//! the vertical step does beyond it is the defect.
//!
//! # The two mechanisms this binary is run against
//!
//! `shading.wgsl` is compiled into the renderer with `include_str!`, so a variant is an edit to that
//! file and a rebuild — done deliberately, twice, and reverted both times:
//!
//! * **The face term** — `cos_face = max(abs(dot(n, v)), FACE_COS_FLOOR)` with
//!   `footprint = distance · pixel_angle / cos_face`. Rising changes `v` for the faces in view, so
//!   it changes the footprint, so it changes every rung's weight; and where the clamp is *crossed*
//!   the footprint's rate changes by up to the 20× the floor allows. Variant: `cos_face = 1.0`
//!   (no face term at all).
//! * **The rung cross-fade** — `octave_weight`'s smoothstep, and the weighted mean it normalises
//!   over. Variant: `weight = 1.0` for every rung (no fade, every rung fully on).
//!
//! **This example measures whatever shader is currently compiled**, and says so on its first line,
//! so a variant run cannot be mistaken for the baseline.

use std::time::Instant;

use voxel_appearance::npr::LightRig;
use voxel_appearance::DetailLattice;
use voxel_client::shading::GpuRig;
use voxel_client::{Camera, Frame, Renderer, Scene};
use voxel_mesh::{Chunk, MeshConfig, Mesher};
use voxel_shapers::{Config, WorldSpec};
use voxel_stream::WorldSource;

/// The frame: 16:9, and big enough that a rung's cell covers several pixels.
const WIDTH: u32 = 640;
const HEIGHT: u32 = 360;

/// How far the eye moves between the two frames of a pair, in metres. **One frame of a slow walk**:
/// the game's own tick advances the eye this far at 0.3 m/s and 60 Hz, so the difference between
/// the two frames is the difference the eye would see between two frames of that walk.
const STEP_M: f64 = 0.05;

/// How many bands the difference is counted in: `[1e-4, 1/255)` — under what an 8-bit window can
/// show — then doublings up to 0.32.
const BANDS: [f32; 6] = [1e-4, 1.0 / 255.0, 0.02, 0.04, 0.08, 0.32];

fn main() {
    // ---- a world, meshed -------------------------------------------------
    let source = WorldSource::new(WorldSpec::new(2468, Config::default()));
    let registry = voxel_shapers::column::materials::registry().expect("the registry");
    let table = voxel_client::shading::table(&registry, &shading_field())
        .expect("every admitted material derives");
    let mesh_config = MeshConfig::from_registry(&registry).expect("a mesh config");

    let mut mesher = Mesher::new();
    let mut meshes = Vec::new();
    let started = Instant::now();
    for cy in 0..2 {
        for cx in 0..3 {
            for cz in 0..3 {
                let pos = voxel_model::ChunkPos::new(cx, cy, cz);
                let Ok(tree) = source.build_chunk(pos) else {
                    continue;
                };
                let chunk = Chunk::from_tree(pos, tree);
                let neighborhood = source.neighborhood(pos);
                if let Ok(mesh) = mesher.build(&chunk, &mesh_config, Some(&neighborhood)) {
                    meshes.push(mesh);
                }
            }
        }
    }
    assert!(!meshes.is_empty(), "the world produced no chunks to draw");
    let mesh_ms = started.elapsed().as_secs_f64() * 1000.0;

    // ---- where the eye stands, from the world itself ----------------------
    //
    // **The vertex format, because the quad format has no vertices to measure.** A record is six
    // `u32`s (C-168) and `Scene::build`'s `vertices` is deliberately empty for it, so a placement
    // read from the quad frame would be `NaN` — which is what this example did on its first run,
    // and it drew two identical frames of nothing.
    let lattice = DetailLattice::new(0);
    let no_light = |_: i32, _: i32, _: i32| None;
    let placed = Scene::build_with_light(meshes.iter(), [0.0; 3], &lattice, &no_light, None, None);
    assert!(!placed.vertices.is_empty(), "the vertex format carried no corners to place the eye");
    let lo = |axis: usize| {
        placed.vertices.iter().map(|v| v.position[axis] as f64).fold(f64::INFINITY, f64::min)
    };
    let hi = |axis: usize| {
        placed
            .vertices
            .iter()
            .map(|v| v.position[axis] as f64)
            .fold(f64::NEG_INFINITY, f64::max)
    };
    // Eye two metres above the terrain's own top, looking **down** at it: the faces this report is
    // about are the horizontal ones, because those are the ones a vertical step moves against.
    let eye = [(lo(0) + hi(0)) / 2.0, hi(1) + 2.0, lo(2) - 24.0];
    let mut camera = Camera::standard(eye, WIDTH as f64 / HEIGHT as f64);
    camera.pitch_rad = -0.5;

    let rig = GpuRig::from_rig(&LightRig::standard()).expect("the standard rig fits");
    let mut renderer = Renderer::new(WIDTH, HEIGHT, &table, &rig).expect("a GPU");
    let pixel_angle = voxel_client::pixel_angle_rad(camera.fov_y_rad, HEIGHT);

    println!(
        "pattern-flicker: {} chunks, {} quads, meshed in {mesh_ms:.0} ms; eye ({:.2}, {:.2}, {:.2}), \
         {pixel_angle:.6} rad a pixel, step {STEP_M} m",
        meshes.len(),
        placed.quads,
        eye[0],
        eye[1],
        eye[2],
    );
    println!(
        "this run measures the shader as compiled — a variant is an edit to `shading.wgsl`, and the \
         numbers below only compare against another run of this binary"
    );

    let still = {
        let scene = Scene::build(meshes.iter(), eye, &lattice, None, None);
        renderer.render(&scene, &camera).expect("a frame")
    };
    // **The step size is the measurement, so it is swept.** Every number this file
    // produced before the sweep was taken at `STEP_M` — one frame of a 0.3 m/s walk —
    // while the user's report is about **jumping**, which moves the eye a metre and
    // sweeps the footprint through several rung stages rather than nudging one. A
    // walk's numbers cannot answer a jump's question. `HOLD` is the camera-held
    // control, run at the smallest step only, because it is a bit-identity check
    // rather than a flicker measurement.
    const WALK_M: f64 = 0.05;
    const FAST_M: f64 = 0.25;
    const JUMP_M: f64 = 1.00;
    for (label, step_m) in [("walk", WALK_M), ("fast", FAST_M), ("jump", JUMP_M)] {
        println!("\n=== {label}: the eye moves {step_m} m ===");
        let mut steps = [("vertical (+y)", 1usize), ("horizontal (+x)", 0usize)];
        for (name, axis) in steps.iter_mut() {
            let mut moved_eye = eye;
            moved_eye[*axis] += step_m;
            let mut moved_camera = Camera::standard(moved_eye, WIDTH as f64 / HEIGHT as f64);
            moved_camera.pitch_rad = camera.pitch_rad;
            // **The scene is rebuilt at the moved eye**, which is what the game does every frame: the
            // records are reused and the origins are re-derived, so the world stands still in the world
            // and the *eye* is what moved (C-170).
            let scene = Scene::build(meshes.iter(), moved_eye, &lattice, None, None);
            let moved = renderer.render(&scene, &moved_camera).expect("a frame");
            report(name, &still, &moved);

            if label == "walk" {
                // **And the same step with the camera held**, which turns out to be the strongest
                // statement of the quad format's own claim rather than the probe this file expected.
                // The scene is the world reckoned from the *moved* eye and it is drawn through the
                // **original** camera: the eye argument `Scene::build` was handed is not in the
                // records (they are chunk-local) and the origins are re-derived at upload from
                // `camera.position_m`, so the two frames are **bit-identical** — 0 of 230,400 pixels
                // differ, on both axes.
                //
                // That is what pins where the eye can possibly enter this format: **twice**, through
                // `origin − eye` (C-170) and through `eye_phase`/`eye_voxel` in `write_camera`, and
                // nowhere else.
                let held = renderer.render(&scene, &camera).expect("a frame");
                report(&format!("{name}, camera held"), &still, &held);
            }
        }
    }
}

/// The difference between two frames, in the terms the question is asked in.
fn report(name: &str, a: &Frame, b: &Frame) {
    let mut changed = 0usize;
    let mut total = 0.0f64;
    let mut worst = 0.0f32;
    // One count per band, plus a last for anything above the top one: `bands[i]` counts the pixels
    // in `(BANDS[i - 1], BANDS[i]]` and `bands[0]` the ones at or under `BANDS[0]` — the pixels that
    // did not move by as much as a float target can tell.
    let mut bands = [0usize; BANDS.len() + 1];
    for (p, q) in a.pixels.iter().zip(b.pixels.iter()) {
        let delta = (0..3).map(|c| (p[c] - q[c]).abs()).fold(0.0f32, f32::max);
        if delta > BANDS[0] {
            changed += 1;
        }
        total += f64::from(delta);
        worst = worst.max(delta);
        match BANDS.iter().position(|edge| delta <= *edge) {
            Some(i) => bands[i] += 1,
            None => bands[BANDS.len()] += 1,
        }
    }
    let pixels = a.pixels.len() as f64;
    println!(
        "{name}: {changed} of {} pixels changed ({:.1} %); total {total:.1}, mean {:.6}, worst \
         {worst:.4}",
        a.pixels.len(),
        100.0 * changed as f64 / pixels,
        total / pixels,
    );
    println!(
        "  bands: {:>8} under {:.0e} | {:>8} ≤1/255 | {:>8} ≤0.02 | {:>8} ≤0.04 | {:>8} ≤0.08 | \
         {:>8} ≤0.32 | {:>8} above",
        bands[0], BANDS[0], bands[1], bands[2], bands[3], bands[4], bands[5], bands[6],
    );
}

/// The world's own shading field, so the material table belongs to this world's registry.
fn shading_field() -> voxel_model::ShadingField {
    voxel_shapers::column::materials::shading_field()
        .expect("the world's shading field declares for the world's own registry")
}
