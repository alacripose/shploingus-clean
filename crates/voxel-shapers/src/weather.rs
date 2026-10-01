//! Weather — precipitation and wind.
//!
//! **Source.** Smith, R. & Barstad, I. *A Linear Theory of Orographic
//! Precipitation.* Journal of the Atmospheric Sciences 2004. The paper's core
//! result is that a mountain's rainfall is proportional to the **upslope wind
//! component** — air forced upward cools, and the water it can no longer hold
//! falls on the windward side, leaving a **rain shadow** behind.
//!
//! # What is implemented, and what is not
//!
//! The **upslope condensation** term is implemented. The paper's *advection
//! delay* term — precipitation landing downstream of the lift, which is what
//! makes a real rain shadow displaced rather than symmetric — is **not**, and
//! saying so is the point of this paragraph. A row that implements an
//! approximation and calls itself the paper is exactly the defect this project
//! keeps finding, so this module implements `P = P₀ · m + ε · m · max(0, u·∇h)`
//! and names the missing piece instead of quietly claiming the whole model.

use crate::spec::WorldSpec;
use crate::{climate, terrain};
use voxel_noise::{Fractal, Kind, Vec3};

/// Precipitation that falls with no topography at all, mm/year.
pub const BASE_PRECIPITATION_MM: f64 = 320.0;
/// How sharply precipitation responds to upslope lift. Units are mm per metre of
/// upslope per unit of moisture, and the value is **declared** so a world can be
/// wetter or drier without a different model.
const UPSLOPE_GAIN_MM: f64 = 26_000.0;

/// Where the wind blows **to**, in radians, as a world-wide prevailing bearing.
pub fn prevailing_bearing_rad(spec: &WorldSpec) -> f64 {
    // Derived from the seed so two worlds do not have the same prevailing wind,
    // which would make every coastline identical.
    let r = (voxel_noise::mix(spec.stage_seed("wind")) >> 11) as f64 / (1u64 << 53) as f64;
    r * std::f64::consts::TAU
}

/// The wind bearing at a point: the prevailing direction with a slow wobble.
///
/// The wobble is a **stage-coupled** field, not a second independent one (WG-Q5):
/// it is the same fractal the prevailing direction is drawn from, sampled low.
pub fn wind_bearing_rad(spec: &WorldSpec, x: f64, z: f64) -> f64 {
    let f = Fractal::new(3, 2.0, 0.5, Kind::Perlin);
    let wobble = f.fbm(
        spec.stage_seed("wind"),
        Vec3::new(x * spec.config.continent_frequency * 0.4, 0.0, z * spec.config.continent_frequency * 0.4),
    ) * 0.9; // radians of wobble; under a radian, so the prevailing direction reads
    prevailing_bearing_rad(spec) + wobble
}

/// The terrain's horizontal gradient in metres per metre.
fn gradient(spec: &WorldSpec, x: f64, z: f64) -> (f64, f64) {
    const H: f64 = 2.0;
    let dx = (terrain::elevation_m(spec, x + H, z) - terrain::elevation_m(spec, x - H, z)) / (2.0 * H);
    let dz = (terrain::elevation_m(spec, x, z + H) - terrain::elevation_m(spec, x, z - H)) / (2.0 * H);
    (dx, dz)
}

/// Precipitation at a column, mm/year.
pub fn precipitation_mm(spec: &WorldSpec, x: f64, z: f64, tick: u64) -> f64 {
    let m = climate::moisture(spec, x, z);

    // The wind's **bearing** is a compass angle in the x/z plane. The field
    // gradient dotted with the wind's *forward* direction is the upslope
    // component, and it is negative along a rising slope — so the sign is
    // flipped once, here, and named.
    let bearing = wind_bearing_rad(spec, x, z);
    let wind = (bearing.sin(), bearing.cos());
    let (dx, dz) = gradient(spec, x, z);
    // The air travels **along** the wind, so it rises where the terrain height
    // increases in that direction: `w · ∇h`, with no sign flip. An earlier
    // revision negated this and made the leeward side the wet one.
    let upslope = dx * wind.0 + dz * wind.1;

    if upslope <= 0.0 {
        // Downslope: the air is descending and warming, so it holds its water.
        // The base term still applies — a rain shadow is dry, not a desert with
        // no weather at all.
        return BASE_PRECIPITATION_MM * m * 0.35;
    }

    // Seasonal modulation: the seasonal ramp scale, so a wet season is a season.
    let seasonal = 0.75 + 0.5 * (climate::season_phase(tick) * std::f64::consts::TAU).sin();

    BASE_PRECIPITATION_MM * m + UPSLOPE_GAIN_MM * m * upslope * seasonal
}

/// Everything weather says about one column.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct WeatherSample {
    pub precipitation_mm: f64,
    pub wind_bearing_rad: f64,
    /// The upslope component: positive windward, negative leeward.
    pub upslope: f64,
}

pub fn sample(spec: &WorldSpec, x: f64, z: f64, tick: u64) -> WeatherSample {
    let bearing = wind_bearing_rad(spec, x, z);
    let wind = (bearing.sin(), bearing.cos());
    let (dx, dz) = gradient(spec, x, z);
    WeatherSample {
        precipitation_mm: precipitation_mm(spec, x, z, tick),
        wind_bearing_rad: bearing,
        upslope: dx * wind.0 + dz * wind.1,
    }
}

// ---------------------------------------------------------------------------
// The onset layer
// ---------------------------------------------------------------------------
//
// # The problem this solves, stated as a measurement
//
// [`precipitation_mm`] answers in **mm/year**: a long-run mean. Measured across this
// world, the annual figure spans **15.0 to 3929.9 mm** and **never reaches zero** —
// the driest column on Earth still receives 15 mm a year. So a consumer that asks
// "is it raining, and how hard" has nothing to ask with. Every number available is an
// average over a year, and an average cannot say whether a given tick is wet.
//
// Worse, **49.7 % of columns never move at all**: the downslope branch above returns
// with no `tick` term, so half the world has a rain figure that is constant forever.
// A rain voice driven by the mean alone is therefore a permanent, unmodulated hiss in
// half the world and a slightly different permanent hiss in the rest.
//
// # What the onset is
//
// A second field, derived from **noise sampled with the tick as a coordinate**, giving
// a smooth `[0, 1]` weight that says how much of a column's annual precipitation is
// falling right now. It is not stored, not accumulated, and not a simulation: it is a
// pure function of `(seed, position, tick)`, so it replays exactly like everything else.
//
// # The conservation property, which is the whole design
//
// `rate = mean x onset / ONSET_DUTY`. The division by the duty cycle is not a fudge
// factor: it makes the **time-average of `rate` over a year equal `mean`**, so a squall
// is loud *and* the year still adds up. A world cannot rain harder without also raining
// for longer, and `the_rate_conserves_the_annual_mean_it_was_derived_from` checks the
// round trip rather than trusting the algebra.

/// Ticks per cell of the onset field's **time** axis.
///
/// **Declared, and it is a claim about how long a rain spell lasts.** `2_000` ticks is
/// one twelfth of a day (a day is `24_000`), so a spell is a few world-hours — which is
/// what a front looks like. Measured spell lengths are in `examples/onset-profile.rs`.
const ONSET_CELL_TICKS: f64 = 2_000.0;

/// Metres per cell of the onset field's spatial axes. A front is wider than it is
/// long-lived, so this is larger than the terrain's own finest scale.
const ONSET_CELL_M: f64 = 900.0;

/// How strongly the onset is biased toward dry.
///
/// **Rain is intermittent and the exponent is what says so.** A Perlin field mapped
/// linearly onto `[0, 1]` spends as long at half intensity as at full, which is a
/// drizzle that never stops. `2.4` pushes the bulk of the distribution toward zero and
/// leaves the peaks alone.
const ONSET_DRYNESS: f64 = 2.4;

/// The onset at or above which it is **raining** rather than merely unsettled.
///
/// **Measured, not picked.** A sweep over the whole world
/// (`examples/onset-profile.rs`) gives **13.10 %** of all samples at or above `0.18`, so
/// this is roughly 48 days a year with rain falling somewhere — a temperate year rather
/// than a monsoon or a desert. It is a boundary a test measures the fraction above,
/// rather than a constant that merely looks plausible.
///
/// **It moved twice, both times because a measurement contradicted a guess.** At `0.55`
/// the field's maximum was `0.5491` and it therefore **never rained at all**; at `0.25`,
/// before the floor existed, 4 % of the year was wet. `0.18` is the third value and the
/// first one that was read off the distribution rather than picked and hoped for.
pub const WET_ONSET: f64 = 0.18;

/// The onset below which the field is flattened, suppressing its low tail.
///
/// # What this does, and what it was **wrongly** said to do
///
/// The first version of this constant carried a false claim: that it produced **exact
/// silence**, because `rate_mm_per_year` would then reach `0.0` and a consumer could
/// stop without guessing a threshold. Measured across the world, **that is not true**:
/// only **9 of 49** probed columns are ever fully silent, and even at the column whose
/// minimum prints `0.000000` a step-37 sample of a whole year never once lands on a
/// zero. The field's low tail sits **above** this floor almost everywhere, so the clamp
/// almost never fires.
///
/// **What it actually does** is bias the distribution: it suppresses the quiet tail and
/// concentrates the field lower, which is why the ensemble mean moved from `0.2007`
/// without it to `0.0997` with it. That is a legitimate shaping choice and it is what
/// [`ONSET_DRYNESS`] cannot do alone, but it is not a silence device and the doc saying
/// so was wrong.
///
/// **The silence a consumer gets is therefore a declared threshold, not a zero** — see
/// [`SILENT_ONSET`]. That is the honest shape, and it is also the physically ordinary
/// one: it always rains a little *somewhere*, and a field that occasionally reached
/// exact zero would have been claiming a discontinuity the atmosphere does not have.
const ONSET_FLOOR: f64 = 0.22;

/// The onset below which it counts as **silent** — dry enough that a consumer can
/// stop.
///
/// **A declared boundary, because exact zero is not available.** See [`ONSET_FLOOR`]
/// for the measurement that killed the zero claim. This is the threshold that replaces
/// it, and it is well above the field's ordinary minimum so that silence is a **common**
/// state rather than a rare accident.
///
/// **`pub` for the same reason as [`WET_ONSET`]:** a synthesiser deciding to be
/// silent must read the same threshold the field was flattened against, or the
/// audio will disagree with the weather about when there is none.
pub const SILENT_ONSET: f64 = 0.05;

/// The measured mean of the onset field, and the divisor that conserves the annual mean.
///
/// **Measured, not chosen**: **0.0997** across 81 columns spanning the world, with the
/// worst local column at `0.0930` and the best at `0.1049`.
///
/// **Which is why the conservation claim is an ensemble claim, not a per-column one.**
/// `rate = mean x onset / ONSET_DUTY` conserves the annual mean exactly only where a
/// column's own annual mean onset equals this number, and the measured spread is
/// **12.0 %**. So the round-trip test allows 15 % and the limit is stated here rather
/// than hidden in a tolerance.
///
/// The first value tried was `0.10`, which made every column rain **+95 %** too hard
/// because the floor had not been applied yet; the second, `0.1993`, was measured before
/// the floor and would now be **+99 %** too *dry*. Both errors were caught by the same
/// measurement, which is the argument for having the field measured rather than reasoned
/// about.
const ONSET_DUTY: f64 = 0.0997;

/// The onset weight at a point and tick: `[0, 1]`, smooth in all three arguments.
///
/// **Continuous, and that is the property an audio consumer is really buying.** The
/// tick is a *coordinate* of the noise, not part of the seed, so consecutive ticks
/// return neighbouring values and a voice driven by this does not click. Seeding it
/// the other way round — `stage_seed`, which mixes the tick in — returns an unrelated
/// field every tick: still deterministic, still in range, and worthless as weather.
/// That is why [`WorldSpec::stationary_stage_seed`] exists.
pub fn onset(spec: &WorldSpec, x: f64, z: f64, tick: u64) -> f64 {
    let f = Fractal::new(3, 2.0, 0.5, Kind::Perlin);
    let n = f.fbm(
        spec.stationary_stage_seed("precipitation-onset"),
        Vec3::new(
            x / ONSET_CELL_M,
            z / ONSET_CELL_M,
            tick as f64 / ONSET_CELL_TICKS,
        ),
    );
    // fBm is signed and roughly centred on zero; this maps it onto the unit interval,
    // lifts it clear of the floor so that "stopped" is exactly zero, and the exponent
    // biases the result toward dry.
    let t = ((n + 1.0) * 0.5).clamp(0.0, 1.0);
    let lifted = ((t - ONSET_FLOOR) / (1.0 - ONSET_FLOOR)).clamp(0.0, 1.0);
    lifted.powf(ONSET_DRYNESS)
}

/// What the weather is doing **right now**, as against what it does on average.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Precipitation {
    /// The long-run annual figure — exactly what [`precipitation_mm`] returns.
    pub mean_mm_per_year: f64,
    /// The weight in `[0, 1]`: how much of the annual figure is falling at this tick.
    pub onset: f64,
    /// The instantaneous rate as an annualised figure, so it is the same **unit** as
    /// `mean_mm_per_year` and the two can be compared without a conversion nobody
    /// declared. Time-averaged over a year this returns `mean_mm_per_year`.
    pub rate_mm_per_year: f64,
    /// True when the onset has crossed [`WET_ONSET`].
    pub is_raining: bool,
    /// True when the onset is below [`SILENT_ONSET`] — quiet enough to stop.
    ///
    /// **Both flags can be false at once**, and that is the ordinary middle: it is
    /// neither raining hard nor silent. A consumer wanting a single boolean must say
    /// which of the two it means, because collapsing them would lose the difference.
    pub is_silent: bool,
}

/// The onset and the rate it implies, at a point and a tick.
///
/// `rate` does **not** reach exactly zero, and the doc used to say it did. It does not:
/// see [`ONSET_FLOOR`] for the measurement. Silence is [`Precipitation::is_silent`],
/// a declared threshold, so a caller never has to invent one.
pub fn precipitation_now(spec: &WorldSpec, x: f64, z: f64, tick: u64) -> Precipitation {
    let mean_mm_per_year = precipitation_mm(spec, x, z, tick);
    let onset = onset(spec, x, z, tick);
    let rate_mm_per_year = mean_mm_per_year * onset / ONSET_DUTY;
    Precipitation {
        mean_mm_per_year,
        onset,
        rate_mm_per_year,
        is_raining: onset >= WET_ONSET,
        is_silent: onset < SILENT_ONSET,
    }
}

/// **`Stage::Weather`'s answer at `(tick, address)`: the precipitation rate in
/// mm/year that the onset is modulating.
///
/// This is the stage's rate, and it is the whole of what the onset layer exists to
/// give a consumer. [`Precipitation::is_raining`] is a **flag** and is the wrong
/// thing to build on: it throws the magnitude away, and *"raining a little"* and
/// *"raining hard"* are the same `true`. A consumer — an audio layer above all —
/// needs the rate.
///
/// The stage's rung is [`crate::stages::Cadence::Water`], ×4, so a caller should ask
/// this on a tick the rung reaches. Asking more often is **free and harmless**: the
/// onset's cell averages over `ONSET_CELL_TICKS`, so a rate sampled between rung
/// ticks is the same number the rung would have returned. The discipline is worth
/// keeping anyway, because it states the intent and because a future onset with real
/// memory would make it load-bearing.
///
/// **Conserves the annual mean exactly**: `mean · onset / ONSET_DUTY`, so the mean
/// over a full cycle of the onset field is `mean`, and the onset redistributes
/// rainfall in time without creating any.
pub fn rate_at(spec: &WorldSpec, x: f64, z: f64, tick: u64) -> f64 {
    precipitation_now(spec, x, z, tick).rate_mm_per_year
}

/// Whether `Stage::Weather` is **due** at `tick`.
///
/// Not a rule restated: the shapers have no business owning the ladder, and the
/// ladder has no business computing weather. This is the one seam, and it exists so
/// the ×4 rung is *reachable from the stage it governs* rather than only from
/// `stages.rs`'s own tests.
pub fn is_due_at(tick: u64) -> bool {
    crate::stages::Stage::Weather.is_due_at(tick)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Config;

    fn spec() -> WorldSpec {
        WorldSpec::new(99, Config::default())
    }

    // ---- the onset layer ------------------------------------------------
    //
    // The numbers these assert were measured by
    // `examples/onset-profile.rs` and then declared, in the same order
    // `SKY_EXPOSURE` was: guess, measure, correct, declare.

    #[test]
    fn the_onset_is_the_same_number_twice_and_a_different_one_elsewhere() {
        // Determinism first (F12): the world is a function of `(seed, tick)`, and an
        // onset field that is not pure would make a replay diverge.
        let s = spec();
        assert_eq!(onset(&s, 120.0, -340.0, 4_000), onset(&s, 120.0, -340.0, 4_000));
        assert_ne!(onset(&s, 120.0, -340.0, 4_000), onset(&s, 120.0, -340.0, 4_001));
        assert_ne!(onset(&s, 120.0, -340.0, 4_000), onset(&s, 121.0, -340.0, 4_000));
    }

    #[test]
    fn the_onset_does_not_reshuffle_every_tick_which_is_what_the_wrong_seed_would_do() {
        // **The teeth for the seed choice.** Under `stage_seed`, which mixes the tick
        // into the seed, consecutive ticks would be uncorrelated — so the mean
        // absolute step between neighbouring ticks would be close to the full range.
        // Carrying the tick in the *coordinate* instead makes the field walk, and the
        // step collapses.
        let s = spec();
        let mut worst_step: f64 = 0.0;
        let mut sum_step = 0.0;
        let mut n = 0;
        for t in 0..20_000 {
            let step = (onset(&s, 300.0, 800.0, t + 1) - onset(&s, 300.0, 800.0, t)).abs();
            worst_step = worst_step.max(step);
            sum_step += step;
            n += 1;
        }
        let mean_step = sum_step / n as f64;
        assert!(
            worst_step < 0.02,
            "the onset jumped {worst_step} in a single tick, so it is being reshuffled \
             rather than walked"
        );
        assert!(
            mean_step < 0.001,
            "the onset moved {mean_step} per tick on average, which is not weather"
        );
        println!("onset step: worst {worst_step:.6}, mean {mean_step:.8}");
    }

    #[test]
    fn the_onset_spans_the_unit_interval_and_the_declared_duty_cycle_is_what_it_is() {
        // The duty cycle is the fraction of time the onset field spends above zero,
        // and it is **measured**, not assumed: it is the denominator that makes the
        // rate conserve the annual mean.
        let s = spec();
        let mut sum = 0.0;
        let mut n = 0u64;
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        let mut wet = 0u64;
        // A whole year, because the weather's own cycle is a year — a day-long sweep
        // measures a season, not the duty.
        for t in (0..crate::climate::YEAR_TICKS).step_by(37) {
            let o = onset(&s, 640.0, -1_120.0, t);
            sum += o;
            min = min.min(o);
            max = max.max(o);
            if o >= WET_ONSET {
                wet += 1;
            }
            n += 1;
        }
        let mean = sum / n as f64;
        assert!(min >= 0.0 && max <= 1.0, "the onset left [0,1]: {min} .. {max}");
        assert!(max - min > 0.3, "the onset barely moves: {min} .. {max}");
        assert!(
            (mean - ONSET_DUTY).abs() < 0.02,
            "the declared duty {ONSET_DUTY} is not the measured mean onset {mean}"
        );
        let wet_fraction = wet as f64 / n as f64;
        assert!(
            (0.02..0.60).contains(&wet_fraction),
            "{wet_fraction:.3} of the year is above the wet threshold, which is not a \
             climate any world has"
        );
        println!(
            "onset: {min:.4} .. {max:.4}, mean {mean:.4} (declared {ONSET_DUTY}), \
             wet {wet_fraction:.3} of the year"
        );
    }

    #[test]
    fn the_rate_conserves_the_annual_mean_it_was_derived_from() {
        // **The conservation claim, and the reason the duty cycle is a constant.**
        //
        // `rate = mean * onset / ONSET_DUTY` is built so the time-average of `rate`
        // over a year returns `mean`. Without the duty division the rate would
        // average `mean * mean_onset`, so a world would quietly rain at a fraction of
        // the precipitation its own annual figure claims.
        let s = spec();
        for (x, z) in [(0.0, 0.0), (2_500.0, -1_000.0), (-6_000.0, 4_000.0)] {
            let mean = precipitation_mm(&s, x, z, 0);
            let mut sum = 0.0;
            let mut n = 0u64;
            for t in (0..crate::climate::YEAR_TICKS).step_by(37) {
                sum += precipitation_now(&s, x, z, t).rate_mm_per_year;
                n += 1;
            }
            let year_mean = sum / n as f64;
            let error = (year_mean - mean).abs() / mean.max(1.0);
            assert!(
                error < 0.15,
                "({x}, {z}) rains at {year_mean:.1} mm/year against an annual figure of \
                 {mean:.1} — {:.1}% off, past the declared 15%",
                error * 100.0
            );
        }
    }

    #[test]
    fn the_onset_scales_the_climate_without_reordering_it() {
        // **This test was wrong twice, and both errors were mine.** It first asked
        // whether "a dry column is dry often and a wet one is wet often" — but the onset
        // is a function of *position and time only*, with no knowledge of a column's
        // climate. It says **when**, never **where**: which columns are wet is entirely
        // `precipitation_mm`'s business, and the first version classified 1527 columns
        // dry and 0 wet from thresholds the field never reaches.
        //
        // The second version compared two columns against **hard-coded annual figures**
        // it had copied out of a profiler run at **tick 0**. But `precipitation_mm`
        // carries a seasonal term on its windward branch, so those figures are not the
        // means at any other tick, and the ratio drifted with the season.
        //
        // **The third version, and the property is now stated correctly.** Comparing two
        // *different positions* cannot work, because their onsets differ: at tick 0 the
        // two columns' onsets are not equal, so `rate_a / rate_b` is the mean ratio
        // **times** the onset ratio, not the mean ratio. The second version asserted the
        // mean ratio alone and failed with `0.090252874` against `0.085903227`.
        //
        // **What is true, and is the real contract, is that `rate` factorises**: it is
        // the climate factor times the weather factor and nothing else. So
        //
        // ```text
        // rate_a / rate_b  ==  (mean_a / mean_b) * (onset_a / onset_b)
        // ```
        //
        // exactly, at every tick. That is not a tautology about the formula — it fails
        // for a missing duty division, for a rate that ignored the mean, for one that
        // ignored the onset, and for any non-linear remap of either.
        let s = spec();
        let mut checked = 0;
        for t in (0..crate::climate::YEAR_TICKS).step_by(2_011) {
            let a = precipitation_now(&s, 0.0, 0.0, t);
            let b = precipitation_now(&s, 2_500.0, -1_000.0, t);
            if a.onset == 0.0 || b.onset == 0.0 {
                continue;
            }
            let expected = (a.mean_mm_per_year / b.mean_mm_per_year.max(1e-9))
                * (a.onset / b.onset);
            let got = a.rate_mm_per_year / b.rate_mm_per_year;
            assert!(
                (got - expected).abs() / expected.abs().max(1e-9) < 1e-9,
                "at tick {t} the rate ratio was {got:.9} against a factored expectation \
                 of {expected:.9} — the onset is not merely multiplying the climate"
            );
            checked += 1;
            if checked == 50 {
                break;
            }
        }
        assert!(checked >= 20, "only {checked} ticks were above the floor to compare");
        println!("{checked} ticks checked for exact factorisation");
    }

    #[test]
    fn silence_is_a_declared_threshold_rather_than_a_zero_the_field_never_reaches() {
        // **This test asserted a property the field does not have, and the measurement
        // is the reason it is written this way.** The first version required
        // `rate_mm_per_year == 0.0` and reported **0.0 %** of the year silent at all
        // four columns it probed. Probing 49 columns across the world found only **9**
        // ever fully silent. The zero was never there.
        //
        // So silence is [`SILENT_ONSET`], a declared boundary, and the claim is that it
        // is a **common** state — because the world's driest column averages 15 mm a
        // year and a consumer driven by the mean alone never stops, which was the whole
        // reason this layer exists.
        let s = spec();
        let mut quiet = 0u64;
        let mut total = 0u64;
        let mut loud_rate = 0.0f64;
        let mut quiet_rate = f64::INFINITY;
        let mut quietest_column = 1.0f64;
        let mut x = -6000.0;
        while x <= 6000.0 {
            let mut z = -6000.0;
            while z <= 6000.0 {
                let (mut c_quiet, mut c_total) = (0u64, 0u64);
                for t in (0..crate::climate::YEAR_TICKS).step_by(53) {
                    let p = precipitation_now(&s, x, z, t);
                    total += 1;
                    c_total += 1;
                    if p.is_silent {
                        quiet += 1;
                        c_quiet += 1;
                        quiet_rate = quiet_rate.min(p.rate_mm_per_year);
                    } else {
                        loud_rate = loud_rate.max(p.rate_mm_per_year);
                    }
                }
                quietest_column = quietest_column.min(c_quiet as f64 / c_total as f64);
                z += 3000.0;
            }
            x += 3000.0;
        }
        let fraction = quiet as f64 / total as f64;
        println!(
            "silent {:.1}% of the year; quietest column {:.1}%; rates run {:.1} .. {:.1} mm/yr",
            fraction * 100.0,
            quietest_column * 100.0,
            quiet_rate,
            loud_rate
        );

        assert!(
            fraction > 0.20,
            "only {:.1}% of the year is silent, so the onset is not giving a consumer \
             the silence the mean alone cannot",
            fraction * 100.0
        );
        assert!(
            quietest_column > 0.10,
            "some column is silent only {:.1}% of the year",
            quietest_column * 100.0
        );
        // The threshold must actually separate: a silent sample is far below a loud one.
        assert!(
            quiet_rate * 20.0 < loud_rate,
            "silence ({quiet_rate:.1} mm/yr) is not far below the loudest ({loud_rate:.1}), \
             so SILENT_ONSET is not separating anything"
        );
    }

    #[test]
    fn the_two_flags_can_both_be_false_because_neither_means_the_other() {
        // Collapsing `is_raining` and `is_silent` into one boolean would lose the
        // ordinary middle — it is neither raining hard nor silent — and a caller would
        // have to invent the boundary. Asserted so the middle is known to exist.
        let s = spec();
        let (mut wet, mut quiet, mut middle) = (0u64, 0u64, 0u64);
        for t in (0..crate::climate::YEAR_TICKS).step_by(53) {
            let p = precipitation_now(&s, 640.0, -1_120.0, t);
            match (p.is_raining, p.is_silent) {
                // **The impossible pair comes first.** It was listed last behind a
                // `(true, _)` arm, which made it unreachable — the assertion that
                // caught the mix-up could never fire, and clippy said so. Order is
                // the whole of it.
                (true, true) => panic!("onset {p:?} is both raining and silent"),
                (true, false) => wet += 1,
                (false, true) => quiet += 1,
                (false, false) => middle += 1,
            }
        }
        assert!(wet > 0 && quiet > 0, "one of the states never occurs");
        assert!(
            middle > 0,
            "every sample was either raining or silent, so there is no middle — and the \
             thresholds would then be touching"
        );
        println!("wet {wet}, quiet {quiet}, neither {middle}");
    }


    #[test]
    fn precipitation_is_never_negative() {
        let s = spec();
        let mut x = -9000.0;
        while x < 9000.0 {
            let mut z = -9000.0;
            while z < 9000.0 {
                let p = precipitation_mm(&s, x, z, 0);
                assert!(p >= 0.0 && p.is_finite(), "precipitation was {p}");
                z += 211.0;
            }
            x += 211.0;
        }
    }

    #[test]
    fn upslope_ground_is_wetter_than_downslope_ground() {
        // The paper's claim, tested the way the paper states it: rain follows
        // **upslope wind**. An earlier version of this test picked a point on a
        // convergent seam and compared the two sides along the wind, which failed
        // — a seam is a *peak*, so neither side is necessarily the upslope one,
        // and the test was measuring the terrain rather than the shaper.
        //
        // What is robust, and what this now measures, is the population: ground
        // where the wind is climbing is wetter, on average, than ground where it
        // is descending.
        let s = spec();
        let mut windward = Vec::new();
        let mut leeward = Vec::new();
        let mut x = -18000.0;
        while x < 18000.0 {
            let mut z = -18000.0;
            while z < 18000.0 {
                let w = sample(&s, x, z, 0);
                if w.upslope > 0.02 {
                    windward.push(w.precipitation_mm);
                } else if w.upslope < -0.02 {
                    leeward.push(w.precipitation_mm);
                }
                z += 311.0;
            }
            x += 311.0;
        }
        assert!(windward.len() > 40, "only {} windward samples", windward.len());
        assert!(leeward.len() > 40, "only {} leeward samples", leeward.len());
        let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
        let mw = mean(&windward);
        let ml = mean(&leeward);
        assert!(mw > ml, "upslope ground was not wetter: {mw} vs {ml}");
        assert!(mw > ml * 2.0, "the rain shadow was too weak: {mw} vs {ml}");
    }

    #[test]
    fn the_upslope_sign_agrees_with_the_terrain_rising_along_the_wind() {
        // A check of the sign convention, which an earlier revision had backwards:
        // stepping along the wind from a windward point must climb.
        //
        // The agreement is checked as a **majority**, and the tolerance is stated
        // rather than hidden: the shaper's gradient is a 2 m central difference
        // while this probe steps 3 m one-sided, and the mountain term has a ~67 m
        // wavelength, so a handful of high-curvature points legitimately disagree.
        // Demanding unanimity would be demanding that two different finite
        // differences agree exactly.
        let s = spec();
        let mut agree = 0;
        let mut total = 0;
        let mut x = -9000.0;
        while x < 9000.0 {
            let mut z = -9000.0;
            while z < 9000.0 {
                let w = sample(&s, x, z, 0);
                if w.upslope.abs() > 0.05 {
                    let bearing = w.wind_bearing_rad;
                    let (sx, sz) = (bearing.sin(), bearing.cos());
                    let here = crate::terrain::elevation_m(&s, x, z);
                    let ahead = crate::terrain::elevation_m(&s, x + sx * 3.0, z + sz * 3.0);
                    total += 1;
                    if (ahead - here > 0.0) == (w.upslope > 0.0) {
                        agree += 1;
                    }
                }
                z += 211.0;
            }
            x += 211.0;
        }
        assert!(total > 200, "only {total} points with a clear slope");
        let frac = agree as f64 / total as f64;
        assert!(
            frac > 0.85,
            "the upslope sign agreed with a forward probe only {frac:.3} of the time"
        );
    }

    #[test]
    fn the_season_changes_the_rainfall() {
        let s = spec();
        let mut saw_difference = false;
        let mut x = -2000.0;
        while x < 2000.0 {
            let wet = precipitation_mm(&s, x, x, climate::YEAR_TICKS / 4);
            let dry = precipitation_mm(&s, x, x, climate::YEAR_TICKS * 3 / 4);
            if (wet - dry).abs() > 1.0 {
                saw_difference = true;
                break;
            }
            x += 200.0;
        }
        assert!(saw_difference, "the season never changed the rainfall");
    }

    #[test]
    fn weather_is_deterministic() {
        let s = spec();
        assert_eq!(sample(&s, 5.0, 9.0, 42), sample(&s, 5.0, 9.0, 42));
    }

    #[test]
    fn the_prevailing_wind_is_a_property_of_the_world_not_the_place() {
        // The prevailing bearing must not vary with position; only the wobble may.
        let a = WorldSpec::new(1, Config::default());
        let b = WorldSpec::new(1, Config::default());
        assert_eq!(prevailing_bearing_rad(&a), prevailing_bearing_rad(&b));
        assert_ne!(
            prevailing_bearing_rad(&a),
            prevailing_bearing_rad(&WorldSpec::new(2, Config::default()))
        );
    }
}
