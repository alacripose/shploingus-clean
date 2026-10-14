//! Measure the cloud model on the client's own seed: is there cloud cover, and where?
//!
//! The declared constants in [`voxel_shapers::clouds`] are claims about this world, so
//! this example measures them rather than trusting them. Run with:
//!
//! ```text
//! cargo run -p voxel-shapers --example cloud-profile
//! cargo run -p voxel-shapers --example cloud-profile -- 9418646105956113635
//! ```
//!
//! It reports, on a 20 km square: the LCL distribution (the cloud base above terrain),
//! the cloud counts by kind at noon and at midnight, and the coupling between emergent
//! cloud and weather's rain — the fraction of raining columns that have a cloud to fall
//! from, which is the number the rain fill interface depends on.

use voxel_shapers::climate::TICKS_PER_DAY;
use voxel_shapers::clouds::{self, CloudKind};
use voxel_shapers::spec::{Config, WorldSpec};
use voxel_shapers::terrain;
use voxel_shapers::weather;

/// The client's seed and spawn, restated from `voxel_client::game` because this example
/// cannot depend on the GPU crate — and checked against it by name, as `relief-profile`
/// does, so a divergence is a visible comment rather than a silent one.
const SEED: u64 = 2468;

/// The sampled square's half-extent and step, metres.
const HALF_EXTENT_M: f64 = 10_000.0;
const STEP_M: f64 = 250.0;
/// Columns per axis: `(2 · HALF_EXTENT / STEP) + 1`.
const GRID: usize = 81;

fn main() {
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
    let noon = TICKS_PER_DAY / 2;
    let midnight = 0u64;

    println!(
        "seed {seed} (written {as_written})\nconfig: sea {} m, base {} m, relief {} m\n",
        spec.config.sea_level_m, spec.config.base_elevation_m, spec.config.relief_m
    );

    // ---- 1. the LCL distribution, noon vs midnight ----
    //
    // The LCL is the height above the terrain at which a surface parcel saturates. It
    // moves with the night cooling, which is the point of printing both hours.
    for (label, tick) in [("noon", noon), ("midnight", midnight)] {
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        let mut sum = 0.0;
        let mut count = 0usize;
        for i in 0..GRID {
            for j in 0..GRID {
                let x = i as f64 * STEP_M - HALF_EXTENT_M;
                let z = j as f64 * STEP_M - HALF_EXTENT_M;
                let air = clouds::surface_air(&spec, x, z, tick);
                min = min.min(air.lcl_m);
                max = max.max(air.lcl_m);
                sum += air.lcl_m;
                count += 1;
            }
        }
        println!(
            "LCL over a 20 km square at {label} ({count} columns): min {min:.0} m, mean {:.0} m, max {max:.0} m",
            sum / count as f64
        );
    }

    // ---- 2. the cloud counts by kind, and by the guide's genus ----
    //
    // The genus is the classification the world's own base and depth make, so this is the
    // reading of whether the field guide's ten genera are what the sky is actually made of
    // — and of which of them the world never makes, which is a finding rather than a gap.
    for (label, tick) in [("noon", noon), ("midnight", midnight)] {
        let mut fog = 0usize;
        let mut cumulus = 0usize;
        let mut stratus = 0usize;
        let mut base_min = f64::INFINITY;
        let mut base_max = f64::NEG_INFINITY;
        let mut top_max = f64::NEG_INFINITY;
        let mut water_sum = 0.0;
        let mut columns = 0usize;
        let mut genera: std::collections::BTreeMap<&'static str, usize> =
            std::collections::BTreeMap::new();
        let mut fronts: std::collections::BTreeMap<&'static str, usize> =
            std::collections::BTreeMap::new();
        let mut inside = 0usize;
        for i in 0..GRID {
            for j in 0..GRID {
                let x = i as f64 * STEP_M - HALF_EXTENT_M;
                let z = j as f64 * STEP_M - HALF_EXTENT_M;
                if let Some(column) = clouds::cloud_column(&spec, x, z, tick) {
                    match column.kind {
                        CloudKind::Fog => fog += 1,
                        CloudKind::Cumulus => cumulus += 1,
                        CloudKind::Stratus => stratus += 1,
                    }
                    *genera.entry(column.genus.abbreviation()).or_insert(0) += 1;
                    if let Some(front) = column.front {
                        *fronts.entry(front.kind.name()).or_insert(0) += 1;
                    }
                    let ground = voxel_shapers::terrain::elevation_m(&spec, x, z);
                    if column
                        .genus
                        .envelope()
                        .holds(column.base_m - ground, column.top_m - ground)
                    {
                        inside += 1;
                    }
                    base_min = base_min.min(column.base_m);
                    base_max = base_max.max(column.base_m);
                    top_max = top_max.max(column.top_m);
                    water_sum += column.water;
                    columns += 1;
                }
            }
        }
        println!(
            "clouds at {label}: {columns} of {} columns (fog {fog}, cumulus {cumulus}, stratus {stratus})",
            GRID * GRID,
        );
        if columns > 0 {
            println!(
                "  base {base_min:.0} … {base_max:.0} m, top to {top_max:.0} m, mean water {:.2}",
                water_sum / columns as f64
            );
            println!("  genera: {genera:?}");
            println!("  fronts: {fronts:?}");
            println!(
                "  inside their genus's envelope: {inside} of {columns} ({} %)",
                inside * 100 / columns
            );
        } else {
            println!("  (none)");
        }
    }

    // ---- 3. the cloud / rain coupling ----
    //
    // The rain fill interface draws rain only under a cloud. If raining columns without
    // clouds are common, the world would see rain from clear sky in those places.
    let mut raining = 0usize;
    let mut raining_with_cloud = 0usize;
    for i in 0..GRID {
        for j in 0..GRID {
            let x = i as f64 * STEP_M - HALF_EXTENT_M;
            let z = j as f64 * STEP_M - HALF_EXTENT_M;
            let now = weather::precipitation_now(&spec, x, z, noon);
            if !now.is_raining {
                continue;
            }
            raining += 1;
            if clouds::cloud_column(&spec, x, z, noon).is_some() {
                raining_with_cloud += 1;
            }
        }
    }
    println!(
        "coupling at noon: {raining_with_cloud} of {raining} raining columns have a cloud"
    );

    // ---- 4. the world-wide sweep: does the moisture belt ever saturate? ----
    //
    // Fog needs the parcel to saturate at the ground, which needs the moisture belt to
    // approach 1. A local sample cannot say whether that happens *anywhere*, so this
    // sweep measures the belt's own maximum and counts the kinds over 100 km.
    let mut max_moisture = 0.0f64;
    let mut max_at = (0.0f64, 0.0f64);
    let mut wide_fog = 0usize;
    let mut wide_cumulus = 0usize;
    let mut wide_stratus = 0usize;
    for i in 0..101 {
        for j in 0..101 {
            let x = i as f64 * 1_000.0 - 50_000.0;
            let z = j as f64 * 1_000.0 - 50_000.0;
            let moisture = voxel_shapers::climate::moisture(&spec, x, z);
            if moisture > max_moisture {
                max_moisture = moisture;
                max_at = (x, z);
            }
            // At midnight: fog is the night regime in this model, so a noon sweep
            // could not see it even where it exists.
            if let Some(column) = clouds::cloud_column(&spec, x, z, midnight) {
                match column.kind {
                    CloudKind::Fog => wide_fog += 1,
                    CloudKind::Cumulus => wide_cumulus += 1,
                    CloudKind::Stratus => wide_stratus += 1,
                }
            }
        }
    }
    println!(
        "world sweep (100 km, 1 km step): moisture max {max_moisture:.3} at {max_at:?}"
    );
    println!(
        "  clouds at midnight over 10201 columns: fog {wide_fog}, cumulus {wide_cumulus}, stratus {wide_stratus}"
    );
    let wettest = clouds::surface_air(&spec, max_at.0, max_at.1, midnight);
    println!(
        "  wettest column at midnight: RH {:.3}, T {:.1} °C (dew {:.1} °C), night LCL {:.0} m, cloud {:?}",
        wettest.relative_humidity,
        wettest.temperature_c,
        wettest.dew_point_c,
        wettest.lcl_m,
        clouds::cloud_column(&spec, max_at.0, max_at.1, midnight).map(|c| c.kind),
    );

    // ---- 5. one rainy column's day, to see the cycle move ----
    //
    // Scans for a raining column and prints its cloud kind every 3 world-hours, so the
    // diurnal change (cumulus by day, fog or stratus by night) is visible as data.
    'find: for i in 0..GRID {
        for j in 0..GRID {
            let x = i as f64 * STEP_M - HALF_EXTENT_M;
            let z = j as f64 * STEP_M - HALF_EXTENT_M;
            if !weather::precipitation_now(&spec, x, z, noon).is_raining {
                continue;
            }
            println!("\nrainy column ({x:.0}, {z:.0}) over one day:");
            for step in 0..8 {
                let tick = step * (TICKS_PER_DAY / 8);
                let kind = clouds::cloud_column(&spec, x, z, tick)
                    .map(|c| c.kind.name())
                    .unwrap_or("clear");
                println!(
                    "  tick {tick:>6} ({}): {kind}",
                    if step == 4 { "noon" } else { "" }
                );
            }
            println!(
                "  surface on the ground: {:.1} m, LCL noon {:.0} m",
                terrain::elevation_m(&spec, x, z),
                clouds::surface_air(&spec, x, z, noon).lcl_m
            );
            break 'find;
        }
    }
}
