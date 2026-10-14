//! Habitat census — the sub-biome layer, counted and drawn.
//! `cargo run -p voxel-shapers --example habitat-census`
//!
//! Reads the world the client streams (seed 2468) straight from the shapers — the same
//! slope, moisture and habitat the material law reads (C-160). Prints:
//!
//! * deciles of the two terrain inputs (slope, moisture) over the spawn window, so the
//!   declared thresholds in `voxel_shapers::habitat` can be read against the world;
//! * a letter map of the spawn window — the sub-biome layer as the ground sees it;
//! * habitat counts for the window and for the whole ±6 km world.
//!
//! The material consequence is the neighbouring example's business: run `surface-census`
//! to see the covers this layer moves.

use std::collections::BTreeMap;

use voxel_shapers::habitat::{self, Habitat};
use voxel_shapers::spec::{Config, WorldSpec};
use voxel_shapers::{climate, terrain};

/// The three inputs the sub-biome layer reads, taken from the shapers themselves.
///
/// These are the same three calls `chunk_fields` makes when it classifies a column's
/// habitat — read here directly rather than through a world representation kept
/// alongside the shapers.
///
/// Returns `(slope, moisture, habitat)`.
fn habitat_at(spec: &WorldSpec, x: f64, z: f64) -> (f64, f64, Habitat) {
    let surface = terrain::elevation_m(spec, x, z);
    let slope = terrain::sample(spec, x, z).slope;
    let moisture = climate::moisture(spec, x, z);
    (slope, moisture, habitat::habitat_of(spec, surface, slope, moisture))
}

fn main() {
    let spec = WorldSpec::new(2468, Config::default());

    let step = 16i32;
    let half = 512i32;

    // ---- the inputs, against the world -----------------------------------------
    let mut slopes = Vec::new();
    let mut moistures = Vec::new();
    for z in (-half..=half).step_by(step as usize) {
        for x in (-half..=half).step_by(step as usize) {
            let (slope, moisture, _) = habitat_at(&spec, x as f64 + 0.5, z as f64 + 0.5);
            slopes.push(slope);
            moistures.push(moisture);
        }
    }
    println!(
        "terrain inputs, spawn window ±{half} m at {step} m ({} columns):",
        slopes.len()
    );
    print_deciles("slope   ", &mut slopes);
    print_deciles("moisture", &mut moistures);

    // ---- the map ---------------------------------------------------------------
    println!("\nsub-biome (habitat), spawn window ±{half} m (column each {step} m):");
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for z in (-half..=half).step_by(step as usize) {
        let mut row = String::new();
        for x in (-half..=half).step_by(step as usize) {
            let (_, _, h) = habitat_at(&spec, x as f64 + 0.5, z as f64 + 0.5);
            *counts.entry(h.name()).or_default() += 1;
            row.push(letter_of(h));
        }
        println!("  {row}");
    }
    println!("  L=lowland  W=wet  D=dry  S=steep  M=montane");
    println!("  window counts: {counts:?}");

    // ---- the whole world -------------------------------------------------------
    let mut world: BTreeMap<&str, usize> = BTreeMap::new();
    let mut x = -6000i32;
    while x <= 6000 {
        let mut z = -6000i32;
        while z <= 6000 {
            let (_, _, h) = habitat_at(&spec, x as f64 + 0.5, z as f64 + 0.5);
            *world.entry(h.name()).or_default() += 1;
            z += 300;
        }
        x += 300;
    }
    println!("whole ±6 km world at 300 m: {world:?}");
}

fn letter_of(habitat: Habitat) -> char {
    match habitat {
        Habitat::Lowland => 'L',
        Habitat::Wet => 'W',
        Habitat::Dry => 'D',
        Habitat::Steep => 'S',
        Habitat::Montane => 'M',
    }
}

/// The `p10 p25 p50 p75 p90` deciles of a sample, printed as one line.
fn print_deciles(name: &str, values: &mut [f64]) {
    values.sort_by(|a, b| a.partial_cmp(b).expect("terrain samples are finite"));
    let at = |p: f64| values[((values.len() - 1) as f64 * p).round() as usize];
    println!(
        "  {name}: p10 {:.3}  p25 {:.3}  p50 {:.3}  p75 {:.3}  p90 {:.3}",
        at(0.10),
        at(0.25),
        at(0.50),
        at(0.75),
        at(0.90),
    );
}
