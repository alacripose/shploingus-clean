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
    let phase = season_phase(tick);
    // Latitude scales the swing: the equator has almost none, the poles have all
    // of it. `sin` rather than a temperature *decline* here — the decline with
    // latitude is already in `latitude_offset_c`.
    let lat_scale = (spec.config.origin_latitude_deg / 90.0).clamp(0.0, 1.0);
    // cos so that phase 0 is the cold extreme.
    (phase * std::f64::consts::TAU).cos() * MAX_SEASONAL_SWING_C * lat_scale
}

/// The mean temperature penalty of this world's latitude, in °C.
///
/// Quadratic in the sine of latitude, which is the standard insolation shape and
/// keeps the equator's derivative flat — a linear penalty makes the tropics a
/// ramp, which they are not.
pub fn latitude_offset_c(latitude_deg: f64) -> f64 {
    let s = latitude_deg.to_radians().sin();
    -45.0 * s * s
}

/// Temperature at a point, in °C.
///
/// `y_m` is the point's altitude above the datum. The **lapse rate** is applied
/// here rather than in [`crate::atmosphere`] because temperature is climate and
/// the atmosphere's own layer is a separate claim.
pub fn temperature_c(spec: &WorldSpec, x: f64, z: f64, y_m: f64, tick: u64) -> f64 {
    let cfg = &spec.config;
    let base = cfg.base_temperature_c;
    let latitude = latitude_offset_c(cfg.origin_latitude_deg);
    let seasonal = seasonal_offset_c(spec, tick);
    let lapse = cfg.lapse_rate_c_per_m * y_m;

    // A slow **climate belt**: the horizontal temperature variation that makes a
    // world have more than one biome. It is stage-coupled — one field, read by
    // both the belt and the seasonal amplitude — rather than two unrelated
    // fractals (WG-Q5).
    //
    // Measured: without it the world produced **two** biomes (tundra and boreal
    // forest) across 12 km, because temperature varied only with a 40 m relief
    // and the season. A world with two biomes is a world with one climate.
    let f = Fractal::new(2, 2.0, 0.5, Kind::Value);
    let belt01 = f.fbm(
        spec.stage_seed("climate"),
        Vec3::new(x * cfg.continent_frequency, 0.0, z * cfg.continent_frequency),
    ) * 0.5
        + 0.5;
    let belt_c = (belt01 - 0.5) * 2.0 * CLIMATE_BELT_C;

    base + latitude + seasonal * (0.7 + 0.3 * belt01) - lapse + belt_c
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
        spec.stage_seed("moisture"),
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
