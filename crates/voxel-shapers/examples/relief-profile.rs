//! Measure the relief at the **client's own seed and spawn**: is the world flat?
//!
//! A throwaway measurement, kept because the report that started it is a
//! screenshot. Run with:
//!
//! ```text
//! cargo run -p voxel-shapers --example relief-profile
//! ```
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
    let spec = WorldSpec::new(SEED, Config::default());
    println!(
        "seed {SEED}, spawn {SPAWN:?}\nconfig: sea {} m, base {} m, relief {} m, continent freq {}\n",
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
}