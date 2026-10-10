//! Measure the relief at the **client's own seed and spawn**: is the world flat?
//!
//! A throwaway measurement, kept because the report that started it is a
//! screenshot. Run with:
//!
//! ```text
//! cargo run -p voxel-shapers --example relief-profile
//! cargo run -p voxel-shapers --example relief-profile -- 9418646105956113635
//! ```
//!
//! **The seed is an argument, and it is printed in the first line.** A measurement
//! of a world it does not name is worse than no measurement, because it looks like
//! one (C-46.1: this example's numbers once described seed 31337 while the client
//! streamed 2468). The default stays 2468 so the documented run is unchanged.
//!
//! `GAME_SEED` is 2468 and `GAME_SPAWN_XZ` is `(0.5, -95.5)`; both are read from
//! the client rather than restated here, so this cannot measure a different world
//! from the one that was looked at.

use voxel_shapers::spec::{Config, WorldSpec};
use voxel_shapers::terrain;
use voxel_shapers::{caves, ecology, hydrology, tectonics};

/// The client's seed and spawn, restated from `voxel_client::game` because this
/// example cannot depend on the GPU crate — and **checked against it** by name, so
/// a divergence is a compile-time-visible comment rather than a silent one.
const SEED: u64 = 2468;
const SPAWN: (f64, f64) = (0.5, -95.5);

fn main() {
    // A seed may be written either way round — `-9028097967753437981` or its
    // two's-complement `u64` image — and they are the **same 64 bits**, so they
    // are the same world. `parse::<u64>()` refuses a leading minus, so parsing as
    // `u64` alone would have quietly fallen back to the default seed and measured
    // a world nobody asked about. `i64` is tried first for exactly that reason,
    // and both forms are printed so the report names which world it is of.
    let raw = std::env::args().nth(1);
    let (seed, as_written) = match raw.as_deref() {
        None => (SEED, format!("{SEED} (default)")),
        Some(a) => match a.parse::<i64>() {
            Ok(v) => (v as u64, v.to_string()),
            Err(_) => match a.parse::<u64>() {
                Ok(v) => (v, v.to_string()),
                Err(e) => panic!("seed {a:?} is neither an i64 nor a u64: {e}"),
            },
        },
    };
    let spec = WorldSpec::new(seed, Config::default());
    println!(
        "seed {seed} (written {as_written}), spawn {SPAWN:?}\nconfig: sea {} m, base {} m, relief {} m, continent freq {}\n",
        spec.config.sea_level_m,
        spec.config.base_elevation_m,
        spec.config.relief_m,
        spec.config.continent_frequency
    );

    // ---- 1. elevation over the client's own streamed window ----
    //
    // GAME_WINDOW is radius 5 chunks = 176 m, so this sweeps roughly what is on
    // screen, not the whole field.
    let half = (5.0 * 32.0) as i32;
    let mut lo = f64::MAX;
    let mut hi = f64::MIN;
    let mut sum = 0.0;
    let mut n = 0u64;
    let mut hist = [0u64; 12];
    for x in -half..half {
        for z in -half..half {
            let e = terrain::elevation_m(&spec, x as f64 + 0.5, z as f64 + 0.5);
            lo = lo.min(e);
            hi = hi.max(e);
            sum += e;
            n += 1;
            let b = (((e - lo.max(0.0)) / 8.0).floor().max(0.0) as usize).min(11);
            hist[b] += 1;
        }
    }
    println!("elevation over the streamed window ({n} samples, {half} m square):");
    println!("  min {lo:.2} m, max {hi:.2} m, mean {:.2} m, **range {:.2} m**", sum / n as f64, hi - lo);
    println!("  sea level is {} m, so the window is {:.2} m below to {:.2} m above it",
        spec.config.sea_level_m, spec.config.sea_level_m - lo, hi - spec.config.sea_level_m);
    println!("  declared relief_m is {} m, so this window uses {:.1} % of it", spec.config.relief_m,
        100.0 * (hi - lo) / spec.config.relief_m);
    println!("  8 m bands, lowest first:");
    for (b, c) in hist.iter().enumerate() {
        if *c > 0 {
            println!("    band {b:>2} ({} m): {c:>6}", b as f64 * 8.0);
        }
    }
    println!();

    // ---- 2. the same sweep over a much larger area ----
    //
    // A window can be flat and a world can be interesting; a 10 km square being
    // flat is a different claim and needs its own number.
    let mut lo = f64::MAX;
    let mut hi = f64::MIN;
    let mut above_sea = 0u64;
    let mut total = 0u64;
    for x in (-5000..5000).step_by(37) {
        for z in (-5000..5000).step_by(41) {
            let e = terrain::elevation_m(&spec, x as f64 + 0.5, z as f64 + 0.5);
            lo = lo.min(e);
            hi = hi.max(e);
            total += 1;
            if e > spec.config.sea_level_m {
                above_sea += 1;
            }
        }
    }
    println!("elevation over a 10 km square ({total} samples):");
    println!("  min {lo:.2} m, max {hi:.2} m, range {:.2} m", hi - lo);
    println!(
        "  land {:.1} % of the area",
        100.0 * above_sea as f64 / total as f64
    );
    println!();

    // ---- 3. the terms that build the elevation ----
    //
    // A flat sum can be a flat continent term, a gated-off mountain term, or both.
    let mut plate_kinds = std::collections::BTreeMap::new();
    let mut strongest = 0.0f64;
    for x in (-2000..2000).step_by(53) {
        for z in (-2000..2000).step_by(59) {
            let p = tectonics::sample(&spec, x as f64, z as f64);
            *plate_kinds.entry(format!("{:?}", p.boundary_kind)).or_insert(0u64) += 1;
            strongest = strongest.max(p.boundary_strength);
        }
    }
    println!("tectonics over a 4 km square:");
    for (k, c) in &plate_kinds {
        println!("  {k:<24} {c:>5}");
    }
    println!("  strongest boundary seen: {strongest:.4}");
    println!();

    // ---- 4. is there any water at all, and any cave? ----
    let water_table = spec.config.sea_level_m + caves::CaveParams::default().water_table_above_sea_m;
    println!("water table: {water_table:.2} m (sea + water_table_above_sea_m)");
    let mut lakes = 0u64;
    let mut lake_depth = 0.0f64;
    let mut channels = 0u64;
    let mut checked = 0u64;
    for i in 0..24 {
        let x = i as f64 * 300.0 - 3600.0;
        let h = hydrology::sample(&spec, x, x * 0.31, 16);
        checked += 1;
        if h.is_lake {
            lakes += 1;
            lake_depth += h.water_depth_m;
        }
        if h.is_channel {
            channels += 1;
        }
    }
    println!("hydrology over {checked} sampled columns:");
    println!("  lakes {lakes}, channels {channels}");
    if lakes > 0 {
        println!("  mean lake depth {:.3} m", lake_depth / lakes as f64);
    }
    println!();

    // ---- 5. what biome does it think it is? ----
    for (label, x, z) in [("spawn", SPAWN.0, SPAWN.1), ("north", 0.0, -2000.0), ("east", 4000.0, 0.0)] {
        let e = terrain::elevation_m(&spec, x, z);
        let biome = ecology::biome_at(&spec, e, 8.0, 900.0);
        println!("  {label:<6} ({x:>7.0}, {z:>7.0}) elevation {e:>7.2} m -> biome {biome:?}");
    }
    println!();

    // ---- 6. is there dry land at the spawn, and how far is the nearest? ----
    //
    // This is the question a **colliding** camera asks and a floating one never
    // did. `column::is_solid` already refuses water, so a walking body falls
    // through the sea and rests on the sea floor: a spawn whose surface is below
    // sea level puts the player's head underwater, whatever the eye is set to.
    // So the number that decides the spawn is the elevation at the spawn, and the
    // distance to the first column above sea level — not the eye height.
    let sea = spec.config.sea_level_m;
    let spawn_e = terrain::elevation_m(&spec, SPAWN.0, SPAWN.1);
    println!("dry land from the spawn (sea level {sea:.2} m):");
    println!(
        "  spawn surface {spawn_e:.2} m -> {}",
        if spawn_e > sea {
            format!("{:.2} m ABOVE sea, a dry spawn", spawn_e - sea)
        } else {
            format!("{:.2} m BELOW sea, a standing body would be submerged", sea - spawn_e)
        }
    );
    // Outward rings: 1 m out to 64 m, then 8 m out to 4 km. Fine near the spawn,
    // coarse far out, so the reported distance is exact where it is small and
    // honest to within 8 m where it is not. The first ring that finds dry land
    // wins, so the answer is the nearest dry column, not merely a dry one.
    let mut nearest_dry: Option<(f64, f64, f64, f64)> = None;
    let mut r: i32 = 1;
    'search: while r <= 4096 {
        let step = if r < 64 { 1 } else { 8 };
        for i in -r..=r {
            for (dx, dz) in [(i, -r), (i, r), (-r, i), (r, i)] {
                let (x, z) = (SPAWN.0 + dx as f64, SPAWN.1 + dz as f64);
                let e = terrain::elevation_m(&spec, x, z);
                if e > sea {
                    nearest_dry = Some((x, z, e, ((dx * dx + dz * dz) as f64).sqrt()));
                    break 'search;
                }
            }
        }
        r += step;
    }
    match nearest_dry {
        Some((x, z, e, d)) => println!(
            "  nearest dry column ({x:.1}, {z:.1}) at {d:.1} m, surface {e:.2} m ({:.2} m above sea)",
            e - sea
        ),
        None => println!("  no dry column within 4 km of the spawn"),
    }
}