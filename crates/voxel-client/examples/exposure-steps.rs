//! **What actually moves between two atmosphere updates**, on both axes that can step: across
//! space with the tick standing still, and across time standing still — `docs/skylight.md` §3.
//!
//! ```text
//! cargo run --offline -p voxel-client --example exposure-steps
//! ```

use voxel_client::skylight::{daylight_of, sky_inputs_at};
use voxel_shapers::{Config, WorldSpec};

/// One rig update's worth of world time: `Stage::Atmosphere` is `Cadence::Water`, every 4
/// ticks, at [`voxel_client::clock::TICK_HZ`].
const CADENCE_TICKS: u64 = 4;

/// The three numbers the frame is actually lit and exposed by.
#[derive(Clone, Copy, Default)]
struct Reading {
    turbidity: f64,
    sun_intensity: f64,
    ambient: f64,
    /// Carried so a step can be attributed to the sun's own position rather than only
    /// counted — a step at the horizon crossing is a different defect from one at noon.
    altitude_deg: f64,
}

fn read(spec: &WorldSpec, x: f64, z: f64, tick: u64) -> Option<Reading> {
    let inputs = sky_inputs_at(spec, x, z, tick);
    let day = daylight_of(spec, tick, inputs, 0)?;
    Some(Reading {
        turbidity: inputs.turbidity,
        sun_intensity: day.rig.lights.first().map_or(0.0, |l| l.intensity),
        ambient: day.rig.ambient.luminance(),
        altitude_deg: day.sun_altitude_deg,
    })
}

/// The largest single-step change in a series, as an absolute and as a fraction of the mean.
fn worst(series: &[Reading], field: fn(&Reading) -> f64) -> (f64, f64) {
    let mut max = 0.0f64;
    for pair in series.windows(2) {
        let step = (field(&pair[1]) - field(&pair[0])).abs();
        if step > max {
            max = step;
        }
    }
    let mean = series.iter().map(field).sum::<f64>() / series.len().max(1) as f64;
    (max, max / mean.max(f64::EPSILON))
}

fn main() {
    let spec = WorldSpec::new(2468, Config::default());
    let start = voxel_client::game::GAME_TICK;
    let per_day = voxel_shapers::climate::TICKS_PER_DAY;
    let seconds = |ticks: u64| ticks as f64 / voxel_client::clock::TICK_HZ as f64;

    println!(
        "the world: seed 2468, a day is {per_day} ticks = {:.0} s of real time at {} Hz",
        seconds(per_day),
        voxel_client::clock::TICK_HZ
    );
    println!(
        "the rig is uploaded on Cadence::Water = every {CADENCE_TICKS} ticks = {:.1} s\n",
        seconds(CADENCE_TICKS)
    );

    // ---- across time: the player stands still -------------------------------------
    //
    // The onset layer is a function of `(x, z, tick)`: the rain moves over a standing player,
    // so this axis can step with no movement at all. It is measured first because if the
    // temporal steps are as large as the spatial ones then walking is not the cause.
    println!("== standing still at the origin, one update per {CADENCE_TICKS} ticks, over a day ==");
    let mut time_series = Vec::new();
    for k in 0..(per_day / CADENCE_TICKS) {
        let tick = start + k * CADENCE_TICKS;
        if let Some(r) = read(&spec, 0.0, 0.0, tick) {
            time_series.push(r);
        }
    }
    let (dt, dt_rel) = worst(&time_series, |r| r.turbidity);
    let (di, di_rel) = worst(&time_series, |r| r.sun_intensity);
    let (da, da_rel) = worst(&time_series, |r| r.ambient);
    println!(
        "  worst step in turbidity:      {dt:>9.4}  ({:>6.1}% of the day's mean)",
        dt_rel * 100.0
    );
    println!(
        "  worst step in sun intensity:  {di:>9.6}  ({:>6.1}% of the day's mean)",
        di_rel * 100.0
    );
    println!(
        "  worst step in ambient:        {da:>9.6}  ({:>6.1}% of the day's mean)",
        da_rel * 100.0
    );

    // How many updates actually step, and by how much. A single step among thousands is a
    // different defect from a rig that moves at that rate every 200 ms.
    let mut stepped = 0usize;
    let mut biggest_at = 0u64;
    let mut biggest = 0.0f64;
    for (k, pair) in time_series.windows(2).enumerate() {
        let step = (pair[1].turbidity - pair[0].turbidity).abs();
        if step > 0.01 {
            stepped += 1;
        }
        if step > biggest {
            biggest = step;
            biggest_at = start + (k as u64 + 1) * CADENCE_TICKS;
        }
    }
    println!(
        "  {} of {} updates moved turbidity by more than 0.01; the largest was at tick {biggest_at} \
         ({:.0} s in)",
        stepped,
        time_series.len().saturating_sub(1),
        seconds(biggest_at - start)
    );

    // Where the worst ambient step is, attributed. The rig's ambient is the sky's dome
    // radiance, so a step in it is a step in the sky and not in any light.
    let mut worst_ambient = (0.0f64, 0u64, 0.0f64, 0.0f64);
    for (k, pair) in time_series.windows(2).enumerate() {
        let step = (pair[1].ambient - pair[0].ambient).abs();
        if step > worst_ambient.0 {
            worst_ambient = (
                step,
                start + (k as u64 + 1) * CADENCE_TICKS,
                pair[0].altitude_deg,
                pair[1].altitude_deg,
            );
        }
    }
    println!(
        "  the ambient's worst step is {:.6} at tick {} ({:.0} s in), sun {:.3} deg -> {:.3} deg",
        worst_ambient.0,
        worst_ambient.1,
        seconds(worst_ambient.1 - start),
        worst_ambient.2,
        worst_ambient.3
    );
    println!(
        "    that is {:.4} after EXPOSURE={} — before the ambient was {:+.4}",
        worst_ambient.0 * voxel_client::skylight::EXPOSURE,
        voxel_client::skylight::EXPOSURE,
        worst_ambient.0 * voxel_client::skylight::EXPOSURE
    );

    // ---- across space: the player walks, the tick stands still --------------------
    println!("\n== walking east at a fixed tick, one update per step of ground covered ==");
    println!("   (the step the player gets is walk speed x {:.1} s of real time)\n", seconds(CADENCE_TICKS));
    println!("  {:>6}  {:>11}  {:>12}  {:>12}  {:>12}", "step m", "max dturb", "% of mean", "max dintens", "max dambient");
    for step_m in [0.25f64, 0.5, 0.86, 1.5, 3.0, 6.0, 12.0] {
        let mut series = Vec::new();
        let mut x = 0.0f64;
        while x < 512.0 {
            // The ground is not flat, but the sky's inputs do not read height — so a straight
            // line in x is a fair probe of every position the player can stand at.
            if let Some(r) = read(&spec, x, 0.0, start) {
                series.push(r);
            }
            x += step_m;
        }
        let (dt, dt_rel) = worst(&series, |r| r.turbidity);
        let (di, _) = worst(&series, |r| r.sun_intensity);
        let (da, _) = worst(&series, |r| r.ambient);
        println!(
            "  {step_m:>6.2}  {dt:>11.4}  {:>11.1}%  {di:>12.6}  {da:>12.6}",
            dt_rel * 100.0
        );
    }

    // ---- and the same line, walked while the tick advances -----------------------
    //
    // The two axes together: the player walks two blocks and a little more in the time one
    // update takes, which is what actually happens in the client.
    println!("\n== walking east as the tick advances, 0.86 m per update ==");
    let mut series = Vec::new();
    let mut x = 0.0f64;
    let mut tick = start;
    while x < 256.0 {
        if let Some(r) = read(&spec, x, 0.0, tick) {
            series.push(r);
        }
        x += 0.86;
        tick += CADENCE_TICKS;
    }
    let (dt, dt_rel) = worst(&series, |r| r.turbidity);
    let (di, di_rel) = worst(&series, |r| r.sun_intensity);
    let (da, da_rel) = worst(&series, |r| r.ambient);
    println!("  worst step in turbidity:      {dt:>9.4}  ({:>6.1}%)", dt_rel * 100.0);
    println!("  worst step in sun intensity:  {di:>9.6}  ({:>6.1}%)", di_rel * 100.0);
    println!("  worst step in ambient:        {da:>9.6}  ({:>6.1}%)", da_rel * 100.0);
}
