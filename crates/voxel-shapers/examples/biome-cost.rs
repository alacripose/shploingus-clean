//! How long does one column's biome read cost? — `cargo run -p voxel-shapers
//! --example biome-cost`
//!
//! The ground's material is about to become a reading of the column's biome, and a chunk
//! build asks for a biome for each of its `CHUNK_SIDE²` columns. If a Holdridge read is a
//! millisecond, the ground cannot be biome-driven without moving the sampling somewhere
//! coarser; if it is tens of microseconds, it can. This measures rather than guesses.

use std::time::Instant;

use voxel_shapers::spec::{Config, WorldSpec};
use voxel_shapers::{ecology, terrain};

fn main() {
    let spec = WorldSpec::new(2468, Config::default());
    let n: u64 = 2_000;
    let t0 = Instant::now();
    let mut acc = 0.0;
    for i in 0..n {
        let x = i as f64 * 17.3 - 8000.0;
        let z = ((i * 7) % 997) as f64 * 15.7 - 8000.0;
        let y = terrain::elevation_m(&spec, x, z);
        let (biotemperature, precipitation) = ecology::holdridge_inputs(&spec, x, z, y, 0);
        acc += biotemperature + precipitation;
    }
    let dt = t0.elapsed();
    let per = dt / n as u32;
    let per_chunk = per * (voxel_model::coord::CHUNK_SIDE * voxel_model::coord::CHUNK_SIDE) as u32;
    println!("{n} columns in {dt:?}");
    println!("{per:?} per column, {per_chunk:?} per {0}×{0} chunk of columns (acc {acc:.1})",
        voxel_model::coord::CHUNK_SIDE);
}
