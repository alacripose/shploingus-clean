//! **PROTOTYPE — THROWAWAY.** Evidence for a ticket; delete after filing.
//!
//! Question: *does a column's biome drift across a year when `biome_at` is fed the
//! per-tick values `column.rs` actually passes it?*
//!
//! Holdridge 1947 classifies on **biotemperature** and **mean annual precipitation**.
//! `column.rs:285` passes `clim.temperature_c` and `wx.precipitation_mm` — both
//! instantaneous at the current tick. If that makes the biome move through the year,
//! the citation and the implementation disagree.

use voxel_shapers::climate::{self, DAYS_PER_YEAR, TICKS_PER_DAY};
use voxel_shapers::ecology::{self, Biome};
use voxel_shapers::spec::{Config, WorldSpec};
use voxel_shapers::terrain;
use voxel_shapers::weather;

fn main() {
    let spec = WorldSpec::new(20_260_930, Config::default());

    let probes: [(f64, f64); 4] = [(0.0, 0.0), (1200.0, -800.0), (-3000.0, 2500.0), (6000.0, 6000.0)];

    let mut drifting = 0usize;
    let mut total = 0usize;

    for (x, z) in probes {
        let elevation = terrain::elevation_m(&spec, x, z);
        let mut seen: Vec<Biome> = Vec::new();
        let mut first: Option<(u64, Biome)> = None;
        let mut changes: Vec<(u64, Biome, Biome)> = Vec::new();

        for day in 0..DAYS_PER_YEAR {
            let tick = day * TICKS_PER_DAY;
            let clim = climate::sample(&spec, x, z, tick);
            let wx = weather::sample(&spec, x, z, tick);
            let b = ecology::biome_at(&spec, elevation, clim.temperature_c, wx.precipitation_mm);

            if first.is_none() {
                first = Some((tick, b));
            }
            match seen.last() {
                Some(&prev) if prev != b => changes.push((tick, prev, b)),
                None => seen.push(b),
                Some(_) => {}
            }
            if !seen.contains(&b) {
                seen.push(b);
            }
            total += 1;
        }

        let (start_tick, start_biome) = first.unwrap();
        let _ = start_tick;
        if seen.len() > 1 {
            drifting += 1;
        }
        println!(
            "({x:>7.0}, {z:>7.0})  day 0 = {:<20} -> {} distinct over the year{}",
            start_biome.name(),
            seen.len(),
            if changes.is_empty() {
                String::new()
            } else {
                format!(
                    "; first change on day {} ({} -> {})",
                    changes[0].0 / TICKS_PER_DAY,
                    changes[0].1.name(),
                    changes[0].2.name()
                )
            }
        );
        for (tick, from, to) in changes.iter().take(3) {
            println!(
                "        day {:>3}: {:<20} -> {}",
                tick / TICKS_PER_DAY,
                from.name(),
                to.name()
            );
        }
    }

    println!("\n{drifting} of {} columns changed biome within one year.", probes.len());
    let _ = total;

    // And the classification's own inputs, so the drift can be attributed.
    println!("\nwhat actually moved:");
    for (x, z) in probes {
        let elevation = terrain::elevation_m(&spec, x, z);
        let mut t_min = f64::INFINITY;
        let mut t_max = f64::NEG_INFINITY;
        let mut p_min = f64::INFINITY;
        let mut p_max = f64::NEG_INFINITY;
        for day in 0..DAYS_PER_YEAR {
            let tick = day * TICKS_PER_DAY;
            let clim = climate::sample(&spec, x, z, tick);
            let wx = weather::sample(&spec, x, z, tick);
            let _ = elevation;
            t_min = t_min.min(clim.temperature_c);
            t_max = t_max.max(clim.temperature_c);
            p_min = p_min.min(wx.precipitation_mm);
            p_max = p_max.max(wx.precipitation_mm);
        }
        println!(
            "({x:>7.0}, {z:>7.0})  temperature {t_min:>7.2} .. {t_max:>7.2} °C   \
             precipitation {p_min:>8.1} .. {p_max:>8.1} mm"
        );
    }
}
