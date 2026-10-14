//! Which declared rocks and ores the world actually draws —
//! `cargo run -p voxel-shapers --example rock-census`.
//!
//! **Before C-172 the solid world was five generic classes** (`IGNEOUS`, `SEDIMENTARY`,
//! `METAMORPHIC`, `BEDROCK`, `REGOLITH`), so the declared rock rows and the ore rows were
//! admitted to the registry and **never drawn** — twenty-two measured rock spectra and
//! every ore spectrum reaching no pixel. This census walks the *same* path the client
//! streams (`chunk_fields::material_at`) over a slab of the world and counts every
//! declared substance below the soil, by name, next to the generic rows that remain.
//!
//! It is the witness that the wiring is real: a registered row that no voxel holds is a
//! row that still reaches no pixel, and this is where that would show.

use std::collections::BTreeMap;

use voxel_model::MaterialId;
use voxel_shapers::chunk_fields;
use voxel_shapers::materials;
use voxel_shapers::corpus;
use voxel_shapers::spec::{Config, WorldSpec};
use voxel_shapers::terrain;

fn main() {
    let spec = WorldSpec::new(2468, Config::default());

    // A 512 m × 512 m window at 16 m steps, each column walked 260 m down from its
    // surface. The ore depth bands stack shallow→deep (coal 2–60, iron 10–120, copper
    // 25–180, tin 40–220 m) and `ore_at` returns the **shallowest** kind whose band holds
    // the depth, so a shallower slab cannot reach tin at all: 150 m reaches copper and
    // stops. 260 m gives every ore kind the wiring can draw the chance to appear.
    let mut declared: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut generic: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut columns = 0usize;
    for x in (-256..256).step_by(16) {
        for z in (-256..256).step_by(16) {
            let surface = terrain::elevation_m(&spec, x as f64 + 0.5, z as f64 + 0.5);
            let top = surface as i32 - 1;
            columns += 1;
            for y in (top - 260)..=top {
                let m = chunk_fields::material_at(&spec, x, y, z).unwrap_or(materials::AIR);
                if let Some(name) = corpus::name_of(m) {
                    *declared.entry(name).or_default() += 1;
                } else if let Some(name) = generic_name(m) {
                    *generic.entry(name).or_default() += 1;
                }
            }
        }
    }

    let total_declared: usize = declared.values().sum();
    println!(
        "rock-census: {columns} columns, {total_declared} declared voxels across {} distinct \
         substances",
        declared.len()
    );
    for (name, count) in &declared {
        println!("  declared\t{count:>8}\t{name}");
    }
    println!("  generic (world rows still drawn): {generic:?}");

    // **The claim, asserted rather than only printed**: the rocks and the ores are both
    // present, or the wiring drew nothing.
    let rocks = corpus::ROCK_KINDS
        .iter()
        .flat_map(|(_, kinds)| kinds.iter())
        .filter(|name| declared.contains_key(*name))
        .count();
    let ores = corpus::ORE_SUBSTANCE
        .iter()
        .filter(|(_, name)| declared.contains_key(name))
        .count();
    println!(
        "  rocks drawn: {rocks}/{}   ores drawn: {ores}/{}",
        corpus::ROCK_KINDS.iter().map(|(_, k)| k.len()).sum::<usize>(),
        corpus::ORE_SUBSTANCE.len()
    );
    assert!(rocks > 0, "no declared rock reached a voxel");
    assert!(ores > 0, "no declared ore reached a voxel");
}

/// The world's own (non-corpus) rows, by name — the two that keep their generic material.
fn generic_name(m: MaterialId) -> Option<&'static str> {
    match m {
        materials::WATER => Some("water"),
        materials::ICE => Some("ice"),
        materials::REGOLITH => Some("regolith"),
        materials::SEDIMENTARY => Some("sedimentary"),
        materials::METAMORPHIC => Some("metamorphic"),
        materials::IGNEOUS => Some("igneous"),
        materials::BEDROCK => Some("bedrock"),
        materials::GRASS => Some("grass"),
        materials::DRY_GRASS => Some("dry grass"),
        materials::LEAF => Some("leaf"),
        materials::CONIFER => Some("conifer"),
        materials::SAND => Some("sand"),
        materials::WOOD => Some("wood"),
        _ => None,
    }
}
