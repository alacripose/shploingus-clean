//! Atmosphere — pressure and density with altitude.
//!
//! **Source.** The *U.S. Standard Atmosphere*, 1976, and the ICAO standard
//! atmosphere's barometric relation. This is the one shaper whose model is a
//! **published constant table** rather than a fractal, and it is here because two
//! other shapers need it and neither may invent it: [`crate::climate`] needs the
//! lapse rate and [`crate::weather`] needs air density to move moisture.
//!
//! F1 lists *"pressure and flow"* among the six declared **approximate**
//! families, so nothing here is claimed exact.
//!
//! # Why an exponential and not the full layered table
//!
//! The real standard atmosphere is piecewise, with a different lapse rate in each
//! layer. Below ~11 km — which is where this world's entire playable band lives,
//! by three orders of magnitude — a single isothermal exponential is accurate to
//! well under a percent, and it is **one function with one constant** instead of
//! a table whose indexing can be wrong. The approximation is stated, not hidden.

use crate::spec::WorldSpec;

/// Sea-level standard pressure, kPa.
pub const SEA_LEVEL_PRESSURE_KPA: f64 = 101.325;
/// Sea-level standard density, kg/m³.
pub const SEA_LEVEL_DENSITY_KG_M3: f64 = 1.225;
/// The scale height: the altitude over which pressure falls by `1/e`.
pub const SCALE_HEIGHT_M: f64 = 8500.0;
/// Standard gravity, m/s².
pub const GRAVITY_M_S2: f64 = 9.806_65;

/// Pressure at an altitude, in kPa.
///
/// Below the datum the exponential *rises*, which is correct for a shaft dug into
/// a world and is why this is not clamped at sea level.
pub fn pressure_kpa(y_m: f64) -> f64 {
    SEA_LEVEL_PRESSURE_KPA * (-y_m / SCALE_HEIGHT_M).exp()
}

/// Air density at an altitude, kg/m³.
pub fn density_kg_m3(y_m: f64) -> f64 {
    SEA_LEVEL_DENSITY_KG_M3 * (-y_m / SCALE_HEIGHT_M).exp()
}

/// The temperature the standard atmosphere implies at an altitude, before any
/// climate modulation. Ice-point referenced: `0 °C` at the datum.
pub fn standard_temperature_c(y_m: f64, lapse_rate_c_per_m: f64) -> f64 {
    -lapse_rate_c_per_m * y_m
}

/// Everything atmosphere says about one altitude.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct AirSample {
    pub pressure_kpa: f64,
    pub density_kg_m3: f64,
    /// True when the pressure is low enough that a surface would be inhospitable.
    /// Declared as a threshold rather than emergent.
    pub is_thin: bool,
}

/// The pressure below which air is declared thin, in kPa.
///
/// The **Armstrong limit**'s pressure (6.3 kPa) is the physiological threshold at
/// which water boils at body temperature. It is a real published number, and the
/// band this world plays in never approaches it — which is exactly why it is
/// worth having: the day a world does approach it, something should say so.
pub const THIN_AIR_KPA: f64 = 6.3;

/// Sample the atmosphere. `spec` is accepted so that a world can declare a
/// different atmosphere later without changing every call site; today it
/// contributes nothing, and that is stated rather than implied.
pub fn sample(_spec: &WorldSpec, y_m: f64) -> AirSample {
    let pressure_kpa = pressure_kpa(y_m);
    AirSample {
        pressure_kpa,
        density_kg_m3: density_kg_m3(y_m),
        is_thin: pressure_kpa < THIN_AIR_KPA,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Config;

    #[test]
    fn sea_level_matches_the_published_constants() {
        assert!((pressure_kpa(0.0) - 101.325).abs() < 1e-9);
        assert!((density_kg_m3(0.0) - 1.225).abs() < 1e-9);
    }

    #[test]
    fn pressure_and_density_fall_with_altitude_and_rise_below_the_datum() {
        let mut last = f64::INFINITY;
        for y in [-200, -16, 0, 100, 1000, 5000, 11000] {
            let p = pressure_kpa(y as f64);
            assert!(p > 0.0 && p.is_finite());
            if y >= 0 {
                assert!(p < last, "pressure did not fall at y = {y}");
                last = p;
            }
        }
        // Below the datum the exponential rises, which is correct for a shaft.
        assert!(pressure_kpa(-100.0) > pressure_kpa(0.0));
    }

    #[test]
    fn the_scale_height_is_the_scale_height() {
        // At one scale height the pressure is down by exactly 1/e. If this fails,
        // the constant is decorative.
        let ratio = pressure_kpa(SCALE_HEIGHT_M) / SEA_LEVEL_PRESSURE_KPA;
        assert!((ratio - (-1.0f64).exp()).abs() < 1e-12, "ratio was {ratio}");
    }

    #[test]
    fn the_playable_band_is_nowhere_near_thin_air() {
        // C-6's band is z = -16 … +47. The Armstrong limit is 6.3 kPa. Asserting
        // the gap keeps a future scale change from silently moving the world into
        // an atmosphere nothing else models.
        let s = WorldSpec::new(1, Config::default());
        for y in -100..200 {
            assert!(!sample(&s, y as f64).is_thin, "thin air at y = {y}");
        }
        // The Armstrong limit sits at ~25 km in this exponential: 6.3 kPa is
        // `-8500·ln(6.3/101.325)` ≈ 23.6 km. An earlier test used 20 km and
        // failed, because at 20 km the pressure is 9.6 kPa — not thin air yet.
        assert!(!sample(&s, 20_000.0).is_thin);
        assert!(sample(&s, 25_000.0).is_thin);
    }

    #[test]
    fn the_lapse_rate_is_applied_linearly() {
        assert_eq!(standard_temperature_c(1000.0, 0.0065), -6.5);
        assert_eq!(standard_temperature_c(0.0, 0.0065), 0.0);
    }
}
