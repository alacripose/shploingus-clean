//! Measure the water ledger over a transect: **does it close, and on what.**
//!
//! A throwaway measurement, kept because it is the evidence for C-94.6 and because
//! a closure claim with no numbers attached is a claim about a claim. Run with:
//!
//! ```text
//! cargo run -p voxel-shapers --example water-ledger
//! ```
//!
//! It prints the closure error across a grid, the per-class closure, the mixing
//! error, and the volume in each reservoir — so a reader can see that "closed"
//! means "closed at one machine epsilon", not "closed because nothing moved".

use voxel_shapers::spec::{Config, WorldSpec};
use voxel_shapers::water::{ledger_at, Phase, Process, Ratios, Reservoir, Salinity};

fn main() {
    let spec = WorldSpec::new(2024, Config::default());
    let ratios = Ratios::declared().expect("the declared ratios are valid");
    let year = voxel_shapers::water::year_in_ticks();

    println!("world 2024, sea level {} m", spec.config.sea_level_m);
    println!("a year is {year:.0} ticks, so one tick is {:.3e} years", 1.0 / year);
    println!("declared ratios: {ratios:?}\n");

    let mut worst_total = 0.0f64;
    let mut worst_fresh = 0.0f64;
    let mut worst_saline = 0.0f64;
    let mut worst_mixing = 0.0f64;
    let mut columns = 0u64;
    let mut with_ocean = 0u64;
    let mut with_surface = 0u64;
    let mut with_groundwater = 0u64;
    let mut rain_total = 0.0f64;
    let mut runoff_total = 0.0f64;
    let mut volume = [0.0f64; 3];
    let mut phase = Phase::Liquid;

    for tick in [0u64, 4, 20, 24_000] {
        for i in -30..30 {
            let x = i as f64 * 220.0;
            for z in [-1500.0, -500.0, 0.0, 900.0] {
                let l = match ledger_at(&spec, x, z, tick, 10) {
                    Ok(l) => l,
                    Err(e) => {
                        println!("refused at ({x}, {z}) tick {tick}: {e}");
                        continue;
                    }
                };
                columns += 1;
                let scale = l.storage_total_m3().max(1.0);
                worst_total = worst_total.max(l.closure_error_m3().abs() / scale);
                worst_fresh =
                    worst_fresh.max(l.class_closure_error_m3(Salinity::Fresh).abs() / scale);
                worst_saline =
                    worst_saline.max(l.class_closure_error_m3(Salinity::Saline).abs() / scale);
                worst_mixing = worst_mixing.max(l.mixing_error_m3().abs() / scale);

                if l.derived[Reservoir::Ocean.index()].total_m3() > 0.0 {
                    with_ocean += 1;
                }
                if l.derived[Reservoir::Surface.index()].total_m3() > 0.0 {
                    with_surface += 1;
                }
                if l.derived[Reservoir::Groundwater.index()].total_m3() > 0.0 {
                    with_groundwater += 1;
                }
                for r in Reservoir::ALL {
                    volume[r.index()] += l.storage[r.index()].total_m3();
                    phase = l.storage[r.index()].phase;
                }
                rain_total += l.source_m3();
                runoff_total += l.runoff_m3();
            }
        }
    }

    println!("columns checked: {columns}");
    println!("  with ocean       : {with_ocean}");
    println!("  with surface     : {with_surface}");
    println!("  with groundwater : {with_groundwater}\n");

    println!("worst **relative** closure error over the whole sweep");
    println!("  total    : {worst_total:.3e}");
    println!("  fresh    : {worst_fresh:.3e}");
    println!("  saline   : {worst_saline:.3e}");
    println!("  mixing   : {worst_mixing:.3e}");
    println!("  (f64 machine epsilon is {:.3e})\n", f64::EPSILON);

    let sum: f64 = volume.iter().sum();
    println!("total volume held, summed over every column and tick:");
    for r in Reservoir::ALL {
        println!(
            "  {:<11} {:>14.3} m3  ({:>5.2} %)  phase {:?}",
            r.name(),
            volume[r.index()],
            100.0 * volume[r.index()] / sum,
            phase
        );
    }
    println!("  {:<11} {:>14.3} m3\n", "total", sum);

    println!("fluxes summed over the sweep:");
    println!("  precipitation in : {rain_total:.6e} m3  ({})", Process::Precipitation.name());
    println!("  runoff to the sea: {runoff_total:.6e} m3  ({})", Process::Runoff.name());
    println!("  runoff share     : {:.2} % of the rain", 100.0 * runoff_total / rain_total);

    let worst = worst_total.max(worst_fresh).max(worst_saline).max(worst_mixing);
    println!(
        "\nevery column's ledger closed; {worst:.3e} is the largest relative error, about \
         {:.0} machine epsilons",
        worst / f64::EPSILON
    );
}