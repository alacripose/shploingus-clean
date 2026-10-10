//! **PROTOTYPE — THROWAWAY.** Evidence for a ticket; retained for C-93.2.
//!
//! C-93.2's question — *does a column's biome drift across a year when `biome_at` is
//! fed the per-tick values `column.rs` actually passes it?* — was answered: **4 of 4**
//! columns changed biome within one year, and each returned.
//!
//! **ECO-0001 has since ruled the fix: aggregate before classifying.** So this tool
//! now re-asks the same question under the ruled semantics — the classifier is fed
//! [`ecology::holdridge_inputs`], sampled from the ticked shapers, exactly as
//! `column.rs` feeds it — and the expected answer is **zero** drifting columns. The
//! C-93.2 probes are kept so the before and after compare like for like.

use voxel_shapers::climate::{DAYS_PER_YEAR, TICKS_PER_DAY};
use voxel_shapers::ecology::{self, Biome};
use voxel_shapers::spec::{Config, WorldSpec};
use voxel_shapers::terrain;

fn main() {
    let spec = WorldSpec::new(20_260_930, Config::default());

    let probes: [(f64, f64); 4] =
        [(0.0, 0.0), (1200.0, -800.0), (-3000.0, 2500.0), (6000.0, 6000.0)];

    let mut drifting = 0usize;

    for (x, z) in probes {
        let elevation = terrain::elevation_m(&spec, x, z);
        let mut seen: Vec<Biome> = Vec::new();
        let mut first: Option<Biome> = None;

        for day in 0..DAYS_PER_YEAR {
            let tick = day * TICKS_PER_DAY;
            let (biotemperature_c, mean_annual_precipitation_mm) =
                ecology::holdridge_inputs(&spec, x, z, elevation, tick);
            let b = ecology::biome_at(
                &spec,
                elevation,
                biotemperature_c,
                mean_annual_precipitation_mm,
            );
            if first.is_none() {
                first = Some(b);
            }
            if !seen.contains(&b) {
                seen.push(b);
            }
        }

        let start = first.expect("the loop runs 96 days");
        if seen.len() > 1 {
            drifting += 1;
        }
        let (bt, map) = ecology::holdridge_inputs(&spec, x, z, elevation, 0);
        println!(
            "({x:>7.0}, {z:>7.0})  day 0 = {:<20} -> {} distinct over the year   \
             biotemperature {bt:>6.3} °C  MAP {map:>8.1} mm",
            start.name(),
            seen.len(),
        );
    }

    println!("\n{drifting} of {} columns changed biome within one year.", probes.len());
    println!(
        "C-93.2 measured 4 of 4 before the aggregate landed; 0 of {} is the ruled \
         behaviour (ECO-0001).",
        probes.len()
    );
}
