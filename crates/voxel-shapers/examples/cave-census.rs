//! Whether the world actually has caves, and at what threshold —
//! `cargo run -p voxel-shapers --example cave-census`.
//!
//! The karst shaper ([`voxel_shapers::caves`]) is real and its own tests carve voids at
//! `CaveParams::default()` (threshold 0.40). The **world** uses `spec.config.cave_threshold`
//! instead, and this census asks the only question that matters: how much of the underground
//! comes out as open air or flooded void, per threshold, through the same
//! `chunk_fields::material_at` the client streams.
//!
//! It also prints, per lithology, the **maximum dissolution the model can produce** — the
//! number that says which rock classes *can* carve at all, and which are excluded by
//! arithmetic rather than by taste.

use std::collections::BTreeMap;

use voxel_shapers::caves::{self, CaveParams};
use voxel_shapers::chunk_fields;
use voxel_shapers::materials;
use voxel_shapers::geology::{self, Lithology};
use voxel_shapers::spec::{Config, WorldSpec};
use voxel_shapers::terrain;

/// The thresholds to compare. 0.40 is `CaveParams::default()`; 0.62 is the world's own.
const SWEEP: [f64; 8] = [0.30, 0.35, 0.40, 0.45, 0.50, 0.55, 0.609, 0.62];

fn main() {
    let base = Config::default();
    println!(
        "cave-census: world threshold {} (CaveParams::default() is {})",
        base.cave_threshold,
        CaveParams::default().threshold
    );

    // ---- the ceiling each rock class can reach, threshold-independent ---------------
    let spec = WorldSpec::new(2468, base);
    let params = caves::CaveParams::for_spec(&spec);
    let mut ceiling: BTreeMap<&'static str, f64> = BTreeMap::new();
    for lith in Lithology::ALL {
        ceiling.insert(lith.name(), f64::MIN);
    }
    // A wide window: the water table is a **global constant** (sea level + 2 m), so whether
    // a cave is open or flooded is decided by how high the ground above it stands. A small
    // lowland window can only ever report flooded voids.
    let window: Vec<(i32, i32)> = (-1024..1024)
        .step_by(32)
        .flat_map(|x| (-1024..1024).step_by(32).map(move |z| (x, z)))
        .collect();
    for &(x, z) in &window {
        let surface = terrain::elevation_m(&spec, x as f64 + 0.5, z as f64 + 0.5);
        for y in (surface as i32 - 90)..=(surface as i32 - 1) {
            let (fx, fy, fz) = (x as f64 + 0.5, y as f64 + 0.5, z as f64 + 0.5);
            let lith = geology::stratum_at(&spec, fx, fz, surface, fy).lithology;
            let d = caves::dissolution(&spec, lith, fx, fy, fz, &params);
            let slot = ceiling.get_mut(lith.name()).expect("a lithology row");
            *slot = slot.max(d);
        }
    }
    println!("  max dissolution the model can produce, by rock class:");
    for (name, max) in &ceiling {
        println!("    {name:<14} {max:.3}");
    }

    // ---- what each threshold actually carves ---------------------------------------
    println!("  threshold   open-air cave     flooded void   (underground voxels)");
    for threshold in SWEEP {
        let config = Config { cave_threshold: threshold, ..base };
        let spec = WorldSpec::new(2468, config);
        let (mut open, mut flooded, mut rock) = (0usize, 0usize, 0usize);
        for &(x, z) in &window {
            let surface = terrain::elevation_m(&spec, x as f64 + 0.5, z as f64 + 0.5);
            let top = surface as i32 - 1;
            for y in (top - 90)..=top {
                let m = chunk_fields::material_at(&spec, x, y, z).unwrap_or(materials::AIR);
                if m == materials::AIR {
                    open += 1;
                } else if m == materials::WATER {
                    flooded += 1;
                } else {
                    rock += 1;
                }
            }
        }
        let total = open + flooded + rock;
        let mark = if (threshold - base.cave_threshold).abs() < f64::EPSILON {
            "  <- shipped"
        } else if (threshold - CaveParams::default().threshold).abs() < f64::EPSILON {
            "  <- shaper default"
        } else {
            ""
        };
        println!(
            "    {threshold:<9} {:>6} ({:>6.3}%)   {:>6} ({:>6.3}%)   {total}{mark}",
            open,
            100.0 * open as f64 / total as f64,
            flooded,
            100.0 * flooded as f64 / total as f64,
        );
    }

    // ---- a contiguous slab, because the sweep's 32 m steps under-sample a 42 m
    // fracture network: every voxel of a 64 m x 64 m block, 90 m down.
    println!("  contiguous 64x64 slab, 90 m down (every voxel, no sampling):");
    for threshold in [base.cave_threshold, CaveParams::default().threshold] {
        let config = Config { cave_threshold: threshold, ..base };
        let spec = WorldSpec::new(2468, config);
        let (mut open, mut flooded, mut rock) = (0usize, 0usize, 0usize);
        for x in 0..64 {
            for z in 0..64 {
                let surface = terrain::elevation_m(&spec, x as f64 + 0.5, z as f64 + 0.5);
                let top = surface as i32 - 1;
                for y in (top - 90)..=top {
                    let m = chunk_fields::material_at(&spec, x, y, z).unwrap_or(materials::AIR);
                    if m == materials::AIR {
                        open += 1;
                    } else if m == materials::WATER {
                        flooded += 1;
                    } else {
                        rock += 1;
                    }
                }
            }
        }
        let total = open + flooded + rock;
        println!(
            "    threshold {threshold}: open {open} ({:.3}%), flooded {flooded} ({:.3}%), rock {rock} of {total}",
            100.0 * open as f64 / total as f64,
            100.0 * flooded as f64 / total as f64
        );
    }

    // ---- high ground, because the water table is the thing that decides open vs flooded,
    // and the spawn sits near sea level. Find the highest column in a wide sweep, then walk
    // a contiguous slab there and report open caves as a fraction of the **vadoze zone**
    // (the rock above the water table), which is the only rock a cave can be open in.
    let water_table = spec.config.sea_level_m + params.water_table_above_sea_m;
    let mut highest: (i32, i32, f64) = (0, 0, f64::MIN);
    for x in (-8000..8000).step_by(250) {
        for z in (-8000..8000).step_by(250) {
            let e = terrain::elevation_m(&spec, x as f64 + 0.5, z as f64 + 0.5);
            if e > highest.2 {
                highest = (x, z, e);
            }
        }
    }
    let (hx, hz, he) = highest;
    println!(
        "  highest column in +-8 km: ({hx}, {hz}) at {he:.1} m (water table {water_table:.1} m, \
         {:.1} m of vadoze rock), contiguous 64x64 slab:",
        he - water_table
    );
    for threshold in [base.cave_threshold, CaveParams::default().threshold] {
        let config = Config { cave_threshold: threshold, ..base };
        let spec = WorldSpec::new(2468, config);
        let (mut open, mut flooded, mut vadoze_rock, mut phreatic_rock) = (0usize, 0usize, 0usize, 0usize);
        for x in hx..hx + 64 {
            for z in hz..hz + 64 {
                let surface = terrain::elevation_m(&spec, x as f64 + 0.5, z as f64 + 0.5);
                let top = surface as i32 - 1;
                for y in (top - 90)..=top {
                    let m = chunk_fields::material_at(&spec, x, y, z).unwrap_or(materials::AIR);
                    let above = (y as f64 + 0.5) >= water_table;
                    if m == materials::AIR {
                        open += 1;
                    } else if m == materials::WATER {
                        flooded += 1;
                    } else if above {
                        vadoze_rock += 1;
                    } else {
                        phreatic_rock += 1;
                    }
                }
            }
        }
        let vadoze = open + vadoze_rock;
        println!(
            "    threshold {threshold}: open {open} of {vadoze} vadoze voxels ({:.1}%), flooded {flooded}, \
             solid rock {vadoze_rock} above / {phreatic_rock} below the table",
            100.0 * open as f64 / vadoze.max(1) as f64
        );
    }
}
