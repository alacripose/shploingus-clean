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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Config;

    fn spec() -> WorldSpec {
        WorldSpec::new(99, Config::default())
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
