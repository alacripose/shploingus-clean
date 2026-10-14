//! What the ground is made of — `cargo run -p voxel-shapers --example surface-census`
//!
//! Prints, over the spawn window and then over the whole 12 km world, the **surface
//! material** the client will stream: `chunk_fields::material_at` at the topmost solid
//! voxel of each column, exactly the path `voxel_stream::WorldSource` uses. The ASCII map
//! is the eyeline: before C-159.10 every letter below was `r`, one pale playa for every
//! formation.

use std::collections::BTreeMap;

use voxel_shapers::chunk_fields;
use voxel_shapers::materials;
use voxel_shapers::spec::{Config, WorldSpec};
use voxel_shapers::terrain;
use voxel_model::MaterialId;

fn main() {
    let spec = WorldSpec::new(2468, Config::default());

    // The map: ±512 m at 16 m steps, the ground the eye sees first.
    let step = 16i32;
    let half = 512i32;
    println!("surface materials, spawn window ±512 m (column each {step} m):");
    let mut legend: BTreeMap<char, &str> = BTreeMap::new();
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for z in (-half..=half).step_by(step as usize) {
        let mut row = String::new();
        for x in (-half..=half).step_by(step as usize) {
            let (letter, name) = surface_at(&spec, x, z);
            legend.insert(letter, name);
            *counts.entry(name).or_default() += 1;
            row.push(letter);
        }
        println!("  {row}");
    }
    let mut legend: Vec<(char, &str)> = legend.into_iter().collect();
    legend.sort();
    let legend: Vec<String> = legend.iter().map(|(c, n)| format!("{c}={n}")).collect();
    println!("  {}", legend.join("  "));
    println!("  window counts: {counts:?}");

    // And the whole world, sampled where the biome census samples.
    let mut world: BTreeMap<&str, usize> = BTreeMap::new();
    let mut x = -6000i32;
    while x <= 6000 {
        let mut z = -6000i32;
        while z <= 6000 {
            let (_, name) = surface_at(&spec, x, z);
            *world.entry(name).or_default() += 1;
            z += 300;
        }
        x += 300;
    }
    println!("whole ±6 km world at 300 m: {world:?}");
}

/// The letter and name of the topmost solid voxel's material at a column.
fn surface_at(spec: &WorldSpec, x: i32, z: i32) -> (char, &'static str) {
    let surface_m = terrain::elevation_m(spec, x as f64 + 0.5, z as f64 + 0.5);
    // A metre below the surface: the topsoil, where the biome's own material lives.
    let y = (surface_m - 1.0).floor() as i32;
    let material = chunk_fields::material_at(spec, x, y, z).unwrap_or(materials::AIR);
    let name = name_of(material);
    (letter_of(material), name)
}

fn name_of(m: MaterialId) -> &'static str {
    match m {
        materials::WATER => "water",
        materials::ICE => "ice",
        materials::REGOLITH => "regolith",
        materials::SEDIMENTARY => "sedimentary",
        materials::METAMORPHIC => "metamorphic",
        materials::IGNEOUS => "igneous",
        materials::BEDROCK => "bedrock",
        materials::GRASS => "grass",
        materials::DRY_GRASS => "dry grass",
        materials::LEAF => "leaf",
        materials::CONIFER => "conifer",
        materials::SAND => "sand",
        _ => "air",
    }
}

fn letter_of(m: MaterialId) -> char {
    match m {
        materials::WATER => '~',
        materials::ICE => '*',
        materials::REGOLITH => 'r',
        materials::SEDIMENTARY => 's',
        materials::METAMORPHIC => 'm',
        materials::IGNEOUS => 'i',
        materials::BEDROCK => 'b',
        materials::GRASS => 'g',
        materials::DRY_GRASS => 'd',
        materials::LEAF => 'l',
        materials::CONIFER => 'c',
        materials::SAND => 'S',
        _ => '.',
    }
}
