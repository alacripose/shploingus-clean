//! **PROTOTYPE — THROWAWAY.** Measures the onset field so its constants are set by
//! measurement rather than guessed. Keep or delete; the numbers belong in a test.
//!
//! Run: `cargo run -p voxel-shapers --example onset-profile`

use voxel_shapers::climate::YEAR_TICKS;
use voxel_shapers::spec::{Config, WorldSpec};
use voxel_shapers::weather;

/// A copy of the field, so the profile reports what the constants describe rather
/// than re-deriving the mapping a second time and disagreeing with it.
fn onset_at(spec: &WorldSpec, x: f64, z: f64, tick: u64) -> f64 {
    weather::onset(spec, x, z, tick)
}

/// Where does the onset actually reach exactly zero, and how often?
///
/// The silence test needs this rather than a guess: a field can have a measured
/// minimum of `0.0000` at one column and never touch zero at four others.
fn silence(spec: &WorldSpec) {
    println!("\n== where the onset reaches exactly zero ==");
    let step = 3_000u64;
    let mut cols = 0u32;
    let mut silent_cols = 0u32;
    let mut best = 0.0f64;
    let mut best_at = (0.0, 0.0);
    let mut worst = 1.0f64;
    let mut worst_at = (0.0, 0.0);
    let mut gx = -6000.0;
    while gx <= 6000.0 {
        let mut gz = -6000.0;
        while gz <= 6000.0 {
            let mut silent = 0u64;
            let mut n = 0u64;
            for t in (0..YEAR_TICKS).step_by(step as usize) {
                n += 1;
                if onset_at(spec, gx, gz, t) == 0.0 {
                    silent += 1;
                }
            }
            let frac = silent as f64 / n as f64;
            if frac > 0.0 {
                silent_cols += 1;
            }
            if frac > best {
                best = frac;
                best_at = (gx, gz);
            }
            if frac < worst {
                worst = frac;
                worst_at = (gx, gz);
            }
            cols += 1;
            gz += 2000.0;
        }
        gx += 2000.0;
    }
    println!("  {silent_cols} of {cols} columns are ever fully silent");
    println!("  most silent: {best:.1}% at ({:.0}, {:.0})", best_at.0, best_at.1);
    println!("  least silent: {worst:.1}% at ({:.0}, {:.0})", worst_at.0, worst_at.1);

    // The four columns the silence test currently probes.
    println!("\n  the four columns the test probes:");
    for (x, z) in [(0.0, 0.0), (2_500.0, -1_000.0), (-6_000.0, 4_000.0), (5_500.0, 5_500.0)] {
        let mut silent = 0u64;
        let mut n = 0u64;
        let mut min = f64::INFINITY;
        for t in (0..YEAR_TICKS).step_by(37) {
            let o = onset_at(spec, x, z, t);
            min = min.min(o);
            n += 1;
            if o == 0.0 {
                silent += 1;
            }
        }
        println!(
            "  ({x:>6.0}, {z:>6.0})  silent {:>5.1}%   min onset {min:.6}",
            100.0 * silent as f64 / n as f64
        );
    }
}

fn main() {
    let spec = WorldSpec::new(20_260_930, Config::default());
    let (x, z) = (640.0, -1_120.0);

    // ---- distribution over a whole year --------------------------------
    let mut values: Vec<f64> = Vec::new();
    for t in 0..YEAR_TICKS {
        values.push(onset_at(&spec, x, z, t));
    }
    let mut sorted = values.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let pct = |p: f64| sorted[((sorted.len() - 1) as f64 * p) as usize];

    println!("== the onset field over one year, at ({x}, {z}) ==\n");
    println!("  min {:.4}   max {:.4}   mean {mean:.4}", sorted[0], sorted[sorted.len() - 1]);
    println!("  p10 {:.4}   p50 {:.4}   p90 {:.4}   p99 {:.4}", pct(0.10), pct(0.50), pct(0.90), pct(0.99));
    println!("  declared ONSET_DUTY is {} — measured mean is {mean:.4}", 0.10);

    // ---- how wet, for a sweep of thresholds ----------------------------
    println!("\n== fraction of the year above a threshold ==");
    for t in [0.30, 0.40, 0.45, 0.50, 0.55, 0.60, 0.70] {
        let wet = values.iter().filter(|v| **v >= t).count();
        println!("  onset >= {t:.2}:  {:>6.2}% of the year", 100.0 * wet as f64 / values.len() as f64);
    }

    // ---- spell lengths at the declared threshold -----------------------
    println!("\n== spell lengths at onset >= 0.55 ==");
    for threshold in [0.45, 0.55, 0.65] {
        let mut spells: Vec<u64> = Vec::new();
        let mut run = 0u64;
        for v in &values {
            if *v >= threshold {
                run += 1;
            } else if run > 0 {
                spells.push(run);
                run = 0;
            }
        }
        if run > 0 {
            spells.push(run);
        }
        if spells.is_empty() {
            println!("  threshold {threshold:.2}: it never rains");
            continue;
        }
        let hours = |t: f64| t / 24_000.0 * 24.0;
        let longest = *spells.iter().max().unwrap();
        let mean = spells.iter().sum::<u64>() as f64 / spells.len() as f64;
        println!(
            "  threshold {threshold:.2}: {} spells/year, mean {:.0} ticks ({:.1} h), \
             longest {longest} ticks ({:.1} h)",
            spells.len(),
            mean,
            hours(mean),
            hours(longest as f64)
        );
    }

    // ---- continuity ----------------------------------------------------
    println!("\n== continuity between adjacent ticks ==");
    let mut worst: f64 = 0.0;
    let mut sum = 0.0;
    for t in 0..200_000 {
        let step = (onset_at(&spec, x, z, t + 1) - onset_at(&spec, x, z, t)).abs();
        worst = worst.max(step);
        sum += step;
    }
    println!("  worst step {worst:.6}, mean step {:.8}", sum / 200_000.0);
    println!("  (the tests assert worst < 0.02 and mean < 0.001)");

    // ---- the teeth: what the wrong seed does ----------------------------
    println!("\n== what stage_seed would have given, instead ==");
    use voxel_noise::{Fractal, Kind, Vec3};
    let f = Fractal::new(3, 2.0, 0.5, Kind::Perlin);
    let mut worst: f64 = 0.0;
    let mut sum = 0.0;
    for t in 0..200_000 {
        let at = |tick: u64| {
            let s = spec.at_tick(tick);
            let n = f.fbm(
                s.stage_seed("precipitation-onset"),
                Vec3::new(x / 900.0, z / 900.0, tick as f64 / 2_000.0),
            );
            (((n + 1.0) * 0.5).clamp(0.0, 1.0)).powf(2.4)
        };
        let step = (at(t + 1) - at(t)).abs();
        worst = worst.max(step);
        sum += step;
    }
    println!("  worst step {worst:.6}, mean step {:.8} — uncorrelated, not weather", sum / 200_000.0);

    // ---- conservation ---------------------------------------------------
    println!("\n== does the rate conserve the annual mean? ==");
    for (px, pz) in [(0.0, 0.0), (2_500.0, -1_000.0), (-6_000.0, 4_000.0)] {
        let mean = weather::precipitation_mm(&spec, px, pz, 0);
        let mut sum_rate = 0.0;
        for t in (0..YEAR_TICKS).step_by(37) {
            sum_rate += weather::precipitation_now(&spec, px, pz, t).rate_mm_per_year;
        }
        let year = sum_rate / ((YEAR_TICKS + 36) / 37) as f64;
        println!(
            "  ({px:>6.0}, {pz:>6.0})  annual {mean:>8.1} -> time-averaged rate {year:>8.1} mm  \
             ({:+.2}%)",
            100.0 * (year - mean) / mean.max(1.0)
        );
    }

    // ---- how much does the LOCAL mean onset vary? ------------------------
    //
    // This is the number that decides the conservation tolerance. `rate = mean x
    // onset / ONSET_DUTY` conserves the annual mean **in expectation over the
    // ensemble**; at any one column it conserves it exactly only if that column's
    // own annual mean onset equals the declared duty. So the spread is not noise in
    // the measurement, it is the width of the claim being made.
    println!("\n== the local annual mean onset, across the world ==");
    let mut locals: Vec<f64> = Vec::new();
    let mut gx = -6000.0;
    while gx <= 6000.0 {
        let mut gz = -6000.0;
        while gz <= 6000.0 {
            let mut sum = 0.0;
            for t in (0..YEAR_TICKS).step_by(211) {
                sum += onset_at(&spec, gx, gz, t);
            }
            locals.push(sum / ((YEAR_TICKS + 210) / 211) as f64);
            gz += 1500.0;
        }
        gx += 1500.0;
    }
    let mut sorted_locals = locals.clone();
    sorted_locals.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let ensemble = locals.iter().sum::<f64>() / locals.len() as f64;
    println!(
        "  {} columns: min {:.4}  p10 {:.4}  median {:.4}  p90 {:.4}  max {:.4}",
        locals.len(),
        sorted_locals[0],
        sorted_locals[locals.len() / 10],
        sorted_locals[locals.len() / 2],
        sorted_locals[locals.len() * 9 / 10],
        sorted_locals[sorted_locals.len() - 1]
    );
    println!("  ensemble mean {ensemble:.4}");
    let spread = (sorted_locals[sorted_locals.len() - 1] - sorted_locals[0]) / ensemble;
    println!("  worst-case departure from the ensemble mean: {:.1}% of it", spread * 100.0);

    // ---- a finer threshold sweep ----------------------------------------
    silence(&spec);

    println!("\n== fine sweep, averaged over the world rather than one column ==");
    for t in [0.08, 0.10, 0.12, 0.14, 0.15, 0.16, 0.18, 0.20, 0.22, 0.25] {
        let mut wet = 0u64;
        let mut total = 0u64;
        let mut sx = -3000.0;
        while sx <= 3000.0 {
            let mut sz = -3000.0;
            while sz <= 3000.0 {
                for tick in (0..YEAR_TICKS).step_by(97) {
                    total += 1;
                    if onset_at(&spec, sx, sz, tick) >= t {
                        wet += 1;
                    }
                }
                sz += 3000.0;
            }
            sx += 3000.0;
        }
        println!("  onset >= {t:.2}:  {:>6.2}% of all samples", 100.0 * wet as f64 / total as f64);
    }
}
