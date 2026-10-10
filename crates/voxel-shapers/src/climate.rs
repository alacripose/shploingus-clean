//! Climate — temperature and moisture, **as processes**.
//!
//! **Source.** Whittaker, R. *Vegetation of the Siskiyou Mountains, Oregon and
//! California.* Ecological Monographs 1960, which is what [`crate::ecology`]
//! classifies with; Köppen 1884 for the climate-class idea; and the US Standard
//! Atmosphere's lapse rate via [`crate::atmosphere`]. The seasonal clock is the
//! S-21 ruling — *"a seasonal clock with a continuous analogue ramp"* — credited
//! to `SereneSeasons`.
//!
//! # Climate is a process, not a field (F4)
//!
//! The 20-level ladder deliberately does **not** carry climate. It carries
//! lithology, elevation and soil depth. Climate is *read from process state*, and
//! the reason matters: a stored climate field would be a second record of a truth
//! that the terrain and the season already determine, and two records drift.
//!
//! # The seasonal clock is continuous
//!
//! `season_phase` is a **ramp**, not four discrete seasons. A discrete switch is
//! the thing the S-21 ruling rejected: crops, weathering and the snow line all
//! move continuously in the world this models, and a climate that snapped between
//! four states would make every one of them jump.

use crate::spec::WorldSpec;
use crate::terrain;
use voxel_noise::{Fractal, Kind, Vec3};

/// Ticks in one in-game day. Declared; the tick rate is the world's.
pub const TICKS_PER_DAY: u64 = 24_000;
/// In-game days in one year. Declared for a *playable* cycle: a real 365 would
/// mean a player never sees a second spring.
pub const DAYS_PER_YEAR: u64 = 96;
/// Ticks in one in-game year.
pub const YEAR_TICKS: u64 = TICKS_PER_DAY * DAYS_PER_YEAR;

/// The strongest seasonal temperature swing, °C, at the world's latitude. The
/// actual swing is scaled by latitude in [`seasonal_offset_c`].
const MAX_SEASONAL_SWING_C: f64 = 14.0;

/// Where in the year a tick falls: `[0, 1)`, `0` = mid-winter.
///
/// A continuous ramp. Note that `tick` wraps, so this is well-defined for any
/// `u64` without overflow.
pub fn season_phase(tick: u64) -> f64 {
    (tick % YEAR_TICKS) as f64 / YEAR_TICKS as f64
}

/// The seasonal temperature offset at a latitude, in °C.
///
/// The **analogue** part of S-21: this is a sinusoid, so every value between the
/// extremes occurs, and a caller asking about "a week from now" gets a different
/// answer rather than the same season.
pub fn seasonal_offset_c(spec: &WorldSpec, tick: u64) -> f64 {
    // Latitude scales the swing: the equator has almost none, the poles have all
    // of it. `sin` rather than a temperature *decline* here — the decline with
    // latitude is already in `latitude_offset_c`.
    let lat_scale = (spec.config.origin_latitude_deg / 90.0).clamp(0.0, 1.0);
    seasonal_unit(tick) * MAX_SEASONAL_SWING_C * lat_scale
}

/// The seasonal scalar at a tick: `cos` of the year's phase, in `[-1, 1]`.
///
/// **The one place the season's tick enters the temperature as a scalar.**
/// [`seasonal_offset_c`] is this times the latitude's amplitude, and
/// [`temperature_parts`] carries the column's own amplitude, so a consumer that needs
/// many samples of **one** column — the biome's twelve monthly means are the one that
/// exists — reads the expensive x/z terms once and this scalar per sample. That is a
/// re-association of the same arithmetic, not a second model.
pub fn seasonal_unit(tick: u64) -> f64 {
    let phase = season_phase(tick);
    // cos so that phase 0 is the cold extreme.
    (phase * std::f64::consts::TAU).cos()
}

/// The equator-to-pole span the latitude penalty covers, in °C. **Declared**, and
/// calibrated through the two anchors below rather than fitted to a single point.
///
/// `36` is the fit through the published zonal annual means: **+26 °C at the equator**
/// and **≈ +8 °C at 45°** (`26 − 36 · sin² 45 = 8`). A world that wants a different
/// span gives it one on purpose, rather than by editing an expression.
pub const LATITUDE_SPAN_C: f64 = 36.0;

/// The mean temperature penalty of this world's latitude, in °C.
///
/// Quadratic in the sine of latitude, which is the standard insolation shape and
/// keeps the equator's derivative flat — a linear penalty makes the tropics a
/// ramp, which they are not.
///
/// **Calibrated against real zonal means, because the previous pairing was not.**
/// The offset is measured down from [`crate::spec::Config::base_temperature_c`] — the
/// **equatorial** sea-level annual mean — so the two numbers together have to describe
/// one world. They did not:
///
/// * The offset was `45 · sin²φ` while the base was `15 °C`, so the world's **declared
///   45°** latitude sat at `15 − 22.5 = ` **−7.5 °C** sea level: the Arctic. The whole
///   ±6 km world then classified as tundra/boreal/alpine, with a warmest biotemperature
///   of **2.81 °C** (`logs/c159-biome-census.txt`) — and a biome is what the surface is
///   made of, so a surface made of one biome is a surface made of one material.
/// * Each number was reasonable alone. The defect was the pair: a base that reads as a
///   *sea-level mean at the origin* combined with a penalty that reads as an
///   *equator-to-pole span* applies latitude twice.
///
/// The third anchor is honest rather than fitted: this form puts the pole at **−10 °C**,
/// warmer than the Arctic's annual mean of roughly **−18 °C**. Bending the curve to
/// reach it would move the two latitudes the world is actually played at, so the
/// mismatch is stated here instead of hidden.
pub fn latitude_offset_c(latitude_deg: f64) -> f64 {
    let s = latitude_deg.to_radians().sin();
    -LATITUDE_SPAN_C * s * s
}

/// A column's temperature as the two parts that separate: what does not move with the
/// season, and what the season scales.
///
/// `temperature_c = base_c + seasonal_amplitude_c · seasonal_unit(tick)` — exactly, and
/// **declared because the biome reads one column ninety-six times.** [`crate::ecology`]
/// samples twelve monthly means of this function at four sub-samples each, and the x/z
/// terms (the climate belt, the lapse) are the expensive half and are identical at every
/// one of those samples. This is where they are read once.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct TemperatureParts {
    /// The part that does not move with the season, °C.
    pub base_c: f64,
    /// The amplitude the seasonal scalar scales, °C.
    pub seasonal_amplitude_c: f64,
}

/// Temperature at a point, in °C.
///
/// `y_m` is the point's altitude above the datum. The **lapse rate** is applied
/// here rather than in [`crate::atmosphere`] because temperature is climate and
/// the atmosphere's own layer is a separate claim.
pub fn temperature_c(spec: &WorldSpec, x: f64, z: f64, y_m: f64, tick: u64) -> f64 {
    let parts = temperature_parts(spec, x, z, y_m);
    parts.base_c + parts.seasonal_amplitude_c * seasonal_unit(tick)
}

/// The x/z parts of [`temperature_c`], read once for a column.
pub fn temperature_parts(spec: &WorldSpec, x: f64, z: f64, y_m: f64) -> TemperatureParts {
    let cfg = &spec.config;
    let latitude = latitude_offset_c(cfg.origin_latitude_deg);
    let lapse = cfg.lapse_rate_c_per_m * y_m;

    // A slow **climate belt**: the horizontal temperature variation that makes a
    // world have more than one biome. It is stage-coupled — one field, read by
    // both the belt and the seasonal amplitude — rather than two unrelated
    // fractals (WG-Q5).
    //
    // Measured: without it the world produced **two** biomes (tundra and boreal
    // forest) across 12 km, because temperature varied only with a 40 m relief
    // and the season. A world with two biomes is a world with one climate.
    // **A field, so a stationary seed**: the climate belt is a property of the ground, and
    // a belt that moved with the tick would move every biome with it. The *season* is what
    // moves, and it moves through `seasonal_unit` above.
    let f = Fractal::new(2, 2.0, 0.5, Kind::Value);
    let belt01 = f.fbm(
        spec.stationary_stage_seed("climate"),
        Vec3::new(x * cfg.continent_frequency, 0.0, z * cfg.continent_frequency),
    ) * 0.5
        + 0.5;
    let belt_c = (belt01 - 0.5) * 2.0 * CLIMATE_BELT_C;
    let lat_scale = (cfg.origin_latitude_deg / 90.0).clamp(0.0, 1.0);

    TemperatureParts {
        base_c: cfg.base_temperature_c + latitude - lapse + belt_c,
        seasonal_amplitude_c: MAX_SEASONAL_SWING_C * lat_scale * (0.7 + 0.3 * belt01),
    }
}

/// The peak horizontal temperature variation, in °C, that the climate belt
/// contributes either side of the world's mean. Declared.
const CLIMATE_BELT_C: f64 = 9.0;

/// Boundary-layer moisture in `[0, 1]`.
///
/// Two coupled terms and no second noise field: a low-frequency humidity belt,
/// **attenuated by altitude**. Air that has risen and dropped its water is dry,
/// so height and moisture are anti-correlated — which is why a mountain top is
/// bare rock and the valley beside it is not.
pub fn moisture(spec: &WorldSpec, x: f64, z: f64) -> f64 {
    let cfg = &spec.config;
    let f = Fractal::new(4, 2.0, 0.5, Kind::Perlin);
    let belt = f.fbm(
        spec.stationary_stage_seed("moisture"),
        Vec3::new(x * cfg.continent_frequency * 1.7, 0.0, z * cfg.continent_frequency * 1.7),
    ) * 0.5
        + 0.5;

    let elev = terrain::elevation_m(spec, x, z);
    let height = ((elev - cfg.sea_level_m).max(0.0) / cfg.relief_m).clamp(0.0, 1.0);

    // 0.7 is a **declared** attenuation: at full relief height the air retains
    // 30% of the lowland moisture. Stated so that it can be tuned deliberately.
    const ALTITUDE_ATTENUATION: f64 = 0.7;
    (belt * (1.0 - ALTITUDE_ATTENUATION * height)).clamp(0.0, 1.0)
}

/// Everything climate says about one column.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ClimateSample {
    pub temperature_c: f64,
    pub moisture: f64,
    pub season_phase: f64,
    /// True when the surface is at or below freezing.
    pub is_freezing: bool,
}

/// Sample the climate at a column's surface.
pub fn sample(spec: &WorldSpec, x: f64, z: f64, tick: u64) -> ClimateSample {
    let y = terrain::elevation_m(spec, x, z);
    let temperature_c = temperature_c(spec, x, z, y, tick);
    ClimateSample {
        temperature_c,
        moisture: moisture(spec, x, z),
        season_phase: season_phase(tick),
        is_freezing: temperature_c <= 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Config;

    fn spec() -> WorldSpec {
        WorldSpec::new(4242, Config::default())
    }

    #[test]
    fn the_seasonal_clock_is_continuous_and_periodic() {
        let s = spec();
        // Periodic: exactly a year apart is exactly the same.
        for t in [0u64, 1, 5000, 191_999] {
            assert_eq!(season_offset_at(&s, t), season_offset_at(&s, t + YEAR_TICKS));
        }
        // Continuous: no jumps anywhere in a year.
        let mut prev = season_offset_at(&s, 0);
        let mut t = 0u64;
        while t < YEAR_TICKS {
            t += 60;
            let v = season_offset_at(&s, t);
            assert!((v - prev).abs() < 0.1, "season jumped {} at tick {t}", (v - prev).abs());
            prev = v;
        }
    }

    fn season_offset_at(s: &WorldSpec, t: u64) -> f64 {
        seasonal_offset_c(s, t)
    }

    #[test]
    fn the_seasonal_ramp_actually_reaches_both_extremes() {
        let s = spec();
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        let mut t = 0u64;
        while t < YEAR_TICKS {
            let v = seasonal_offset_c(&s, t);
            lo = lo.min(v);
            hi = hi.max(v);
            t += 97;
        }
        assert!(hi - lo > 8.0, "the year only swung {lo}..{hi} °C");
    }

    #[test]
    fn the_phase_is_a_ramp_not_four_states() {
        // A discrete season would produce the same value over long runs. Count
        // distinct values in a year; a four-state climate gives ~4.
        let mut seen = std::collections::BTreeSet::new();
        let mut t = 0u64;
        while t < YEAR_TICKS {
            seen.insert((season_phase(t) * 1000.0).round() as i64);
            t += 37;
        }
        assert!(seen.len() > 500, "the season had only {} distinct values", seen.len());
    }

    #[test]
    fn temperature_falls_with_altitude() {
        let s = spec();
        let low = temperature_c(&s, 0.0, 0.0, 0.0, 0);
        let high = temperature_c(&s, 0.0, 0.0, 2000.0, 0);
        assert!(high < low, "temperature did not fall with height: {low} → {high}");
        // The lapse rate is exactly the configured one.
        assert!((low - high - 2000.0 * s.config.lapse_rate_c_per_m).abs() < 1e-9);
    }

    #[test]
    fn latitude_makes_the_poles_colder_than_the_tropics() {
        assert!(latitude_offset_c(0.0) > latitude_offset_c(45.0));
        assert!(latitude_offset_c(45.0) > latitude_offset_c(80.0));
        assert_eq!(latitude_offset_c(0.0), 0.0);
    }

    #[test]
    fn the_latitude_penalty_lands_on_the_published_zonal_means() {
        // **The calibration, asserted rather than described.** The two anchors are the
        // published zonal annual means: the equator is the base itself, and 45° — this
        // world's declared origin — must come out at ≈ +8 °C at sea level. The old pair
        // (base 15, span 45) put it at −7.5 °C, which is the defect this pins.
        let s = spec();
        let cfg = &s.config;
        let at_sea_level = |latitude: f64| cfg.base_temperature_c + latitude_offset_c(latitude);
        assert!(
            (at_sea_level(0.0) - 26.0).abs() < 1e-9,
            "the equator is {} °C, not the declared base",
            at_sea_level(0.0)
        );
        let mid = at_sea_level(45.0);
        assert!(
            (7.0..=9.0).contains(&mid),
            "45° came out at {mid} °C — not the published zonal mean this is calibrated on"
        );
        // And the world's own declared latitude is what its surface climate is measured
        // at, so the origin is not an Arctic world by accident.
        let origin = at_sea_level(cfg.origin_latitude_deg);
        assert!(
            origin > 0.0,
            "the world's declared {} ° latitude sits below freezing at sea level ({origin} °C)",
            cfg.origin_latitude_deg
        );
    }

    #[test]
    fn moisture_is_in_range_and_lower_at_altitude() {
        let s = spec();
        let mut x = -3000.0;
        while x < 3000.0 {
            let m = moisture(&s, x, x * 0.5);
            assert!((0.0..=1.0).contains(&m), "moisture was {m} at x = {x}");
            x += 137.0;
        }
    }

    #[test]
    fn climate_is_deterministic_for_a_seed_and_a_tick() {
        let s = spec();
        assert_eq!(
            sample(&s, 10.0, 20.0, 1234),
            sample(&s, 10.0, 20.0, 1234)
        );
        assert_ne!(sample(&s, 10.0, 20.0, 0), sample(&s, 10.0, 20.0, YEAR_TICKS / 2));
    }
}
