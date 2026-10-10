//! **PROTOTYPE — THROWAWAY.** Evidence for C-117; retained so the answer stays checkable.
//!
//! The question: *do the world shapers actually tick over the world?* The record said the
//! shapers were *"still tick-independent"* (`game.rs`, `main.rs`'s module note). **The
//! measurement says otherwise** — the same seed at two spec ticks differs in terrain
//! elevation and in the voxel materials the streamer reads — and what keeps the world
//! still is that the client never moves the generator's tick.

use voxel_shapers::earthworks::Earthworks;
use voxel_shapers::spec::{Config, WorldSpec};
use voxel_shapers::{column, terrain};

fn main() {
    let spec = WorldSpec::new(2468, Config::default());
    let later = spec.at_tick(24_000);

    println!("terrain elevation, same seed, two spec ticks:");
    for (x, z) in [(0.5, 0.5), (16.5, 16.5), (-32.5, 48.5), (120.5, -96.5)] {
        let a = terrain::elevation_m(&spec, x, z);
        let b = terrain::elevation_m(&later, x, z);
        println!(
            "  ({x:>8.1}, {z:>8.1})  tick 0 = {a:>9.3} m   tick 24000 = {b:>9.3} m   delta = {:>8.3} m",
            b - a
        );
    }

    let earthworks = Earthworks::new();
    println!("\nvoxel materials over a 33 x 33 patch, y = surface(0) - 2 .. + 2:");
    let mut compared = 0u32;
    let mut differing = 0u32;
    let mut examples = 0;
    for gx in (-96..=96).step_by(6) {
        for gz in (-96..=96).step_by(6) {
            let x = gx;
            let z = gz;
            let surface =
                terrain::elevation_m(&spec, x as f64 + 0.5, z as f64 + 0.5).floor() as i32;
            for y in (surface - 2)..=(surface + 2) {
                let a = column::material_at(&spec, &earthworks, x, y, z).unwrap();
                let b = column::material_at(&later, &earthworks, x, y, z).unwrap();
                compared += 1;
                if a != b {
                    differing += 1;
                    if examples < 5 {
                        println!("  ({x:>4}, {y:>4}, {z:>4})  tick 0 = {a:?}   tick 24000 = {b:?}");
                        examples += 1;
                    }
                }
            }
        }
    }
    println!("\n{differing} of {compared} voxels differ when the spec tick moves.");
    println!(
        "If this is more than zero, the shapers are NOT tick-independent — the world would \
         reshuffle if the generator ever saw a moving tick."
    );
}
