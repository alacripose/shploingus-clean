//! **The column water path, measured** — so the sky's turbidity divides by a number the
//! world gave rather than one this file chose. `docs/skylight.md` §3.
//!
//! ```text
//! cargo run --offline -p voxel-shapers --example aerosol-column
//! ```

use voxel_shapers::{clouds, Config, WorldSpec};

fn main() {
    let spec = WorldSpec::new(2468, Config::default());
    let per_day = voxel_shapers::climate::TICKS_PER_DAY;

    let mut paths: Vec<f64> = Vec::new();
    let mut cloudy = 0usize;

    // A line across the world at four hours of the day, so the sample carries both the
    // spatial field and the diurnal cycle that drives the lift.
    for quarter in 0..4u64 {
        let tick = quarter * per_day / 4;
        let mut x = 0.0f64;
        while x < 1024.0 {
            let path_m = clouds::column_water_path_m(&spec, x, 0.0, tick);
            if path_m > 0.0 {
                cloudy += 1;
            }
            paths.push(path_m);
            x += 4.0;
        }
    }

    paths.sort_by(f64::total_cmp);
    let at = |q: f64| paths[((paths.len() - 1) as f64 * q) as usize];
    let mean = paths.iter().sum::<f64>() / paths.len() as f64;

    println!("{} columns, seed 2468", paths.len());
    println!(
        "  water path m:  min {:.1}  mean {:.1}  p90 {:.1}  p99 {:.1}  max {:.1}",
        at(0.0),
        mean,
        at(0.90),
        at(0.99),
        at(1.0)
    );
    println!(
        "  cloudy columns: {cloudy} of {} ({:.1} %) — the rest read the clear-air anchor",
        paths.len(),
        100.0 * cloudy as f64 / paths.len() as f64
    );

    // What a candidate reference would make of it, through **the shape the sky already
    // uses**: `T = CLEAR_AIR_TURBIDITY * (1 + WET_TURBIDITY_GAIN * path / reference)`.
    println!("\n  turbidity each reference would give (clear 2.0, gain 3.0):");
    println!("  {:>9}  {:>8}  {:>8}  {:>8}", "ref m", "median", "p90", "max");
    for reference in [25.0, 50.0, 100.0, 200.0, 400.0] {
        let t = |path: f64| 2.0 * (1.0 + 3.0 * path / reference);
        println!(
            "  {reference:>9.0}  {:>8.2}  {:>8.2}  {:>8.2}",
            t(at(0.5)),
            t(at(0.90)),
            t(at(1.0))
        );
    }

    // ---- and the one the sky actually steps on -------------------------
    //
    // The instrument found a `9.9598` turbidity step at the origin at tick 14428. Which
    // mechanism makes it: the `None` gate, or the witness rule's full-strength assignment?
    // This prints the onset, the boolean it crosses, and the path either side.
    let mut series: Vec<(u64, f64, f64, bool)> = Vec::new();
    for k in 0..(per_day / 4) {
        let tick = k * 4;
        let path_m = clouds::column_water_path_m(&spec, 0.0, 0.0, tick);
        let now = voxel_shapers::weather::precipitation_now(&spec, 0.0, 0.0, tick);
        series.push((tick, path_m, now.onset, now.is_raining));
    }
    let mut steps: Vec<(f64, usize)> = series
        .windows(2)
        .enumerate()
        .map(|(i, w)| ((w[1].1 - w[0].1).abs(), i))
        .collect();
    steps.sort_by(|a, b| b.0.total_cmp(&a.0));
    println!("\n  the origin's five largest water-path steps in a day:");
    println!(
        "  {:>7}  {:>9}  {:>8}  {:>8}  {:>8}  {:>8}",
        "tick", "step m", "onset a", "onset b", "rain a", "rain b"
    );
    for (step, i) in steps.iter().take(5) {
        let (_, _, onset_a, rain_a) = series[*i];
        let (tick_b, _, onset_b, rain_b) = series[*i + 1];
        println!(
            "  {tick_b:>7}  {step:>9.2}  {onset_a:>8.4}  {onset_b:>8.4}  {rain_a:>8}  {rain_b:>8}"
        );
    }
}
