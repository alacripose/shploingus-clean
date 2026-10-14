//! **The air group — `climate`, `atmosphere`, `weather`.**
//!
//! Derived from `docs/SHAPERS.md` §1 and §2's rule 4 (*“Fields are stage-coupled, not independent
//! noise (WG-Q5): a climate field must be a function of the terrain above it, not a second unrelated
//! fBm”*) — which is why every stage here **reads `elevation`** and refuses without it.
//!
//! | Stage | Paper | Produces |
//! |---|---|---|
//! | `climate` | Whittaker 1960; Köppen 1884; ICAO/US Standard Atmosphere 1976 (the lapse rate) | `temperature` |
//! | `atmosphere` | US Standard Atmosphere 1976; the barometric formula | `pressure` |
//! | `weather` | Smith & Barstad, *A Linear Theory of Orographic Precipitation*, JAS 2004; Magnus/Tetens saturation form as recorded by Alduchov & Eskridge 1996 | `precipitation` |

use crate::refusal::{Refusal, RefusalKind};
use crate::stage::{FieldId, Produced, StageContext};

/// The declared physical constants of the barometric formula. Declared, not recalled: they are part
/// of what the law means.
const STANDARD_GRAVITY: f64 = 9.806_65;
const MOLAR_MASS_DRY_AIR: f64 = 0.028_964_4;
const UNIVERSAL_GAS_CONSTANT: f64 = 8.314_47;

fn out_of_range(what: &str, value: f64) -> Refusal {
    Refusal::new(RefusalKind::ParameterOutOfRange, format!("{what} = {value}"))
}

/// The discrete terrain gradient at a column, in metres per metre, from its neighbours.
///
/// Each axis's one-sided differences are **averaged over the neighbours on that axis**, so a uniform
/// ramp reports the ramp's own slope rather than a count-dependent multiple of it — and a ridge whose
/// two flanks cancel reports no gradient, which is the truth about a ridge line.
///
/// One place computes it, so the orographic term and any later reader agree about what the slope of
/// a column is.
pub fn terrain_gradient(patch: crate::stage::Patch, elevation: &[f64], index: usize) -> (f64, f64) {
    let mut gx = 0.0;
    let mut gy = 0.0;
    let (mut along_x, mut along_y) = (0.0_f64, 0.0_f64);
    for (neighbour, dx, dy) in patch.neighbour_steps(index) {
        let difference =
            elevation.get(neighbour).copied().unwrap_or(elevation[index]) - elevation[index];
        if dx != 0 {
            gx += difference * f64::from(dx);
            along_x += 1.0;
        }
        if dy != 0 {
            gy += difference * f64::from(dy);
            along_y += 1.0;
        }
    }
    if along_x > 0.0 {
        gx /= along_x;
    }
    if along_y > 0.0 {
        gy /= along_y;
    }
    (gx, gy)
}

/// **`climate`** (ICAO/US Standard Atmosphere 1976 lapse rate): temperature falls with elevation from
/// the declared sea-level temperature. Reads `elevation`.
pub fn climate(ctx: &StageContext<'_>) -> Result<Produced, Refusal> {
    let sea_level = ctx.dial("sea_level_temperature_c")?;
    let lapse = ctx.dial("lapse_rate_c_per_m")?;
    let elevation = ctx.read(FieldId::Elevation)?;
    Ok(Produced::one(
        FieldId::Temperature,
        elevation.iter().map(|height| sea_level - lapse * height).collect(),
    ))
}

/// **`atmosphere`** (US Standard Atmosphere 1976; barometric formula): surface pressure by altitude.
/// Reads `elevation`. An altitude past the model's own domain — where its bracket reaches zero — is
/// refused with the altitude in it, not clamped to the last finite value.
pub fn atmosphere(ctx: &StageContext<'_>) -> Result<Produced, Refusal> {
    let sea_level_pressure = ctx.dial("sea_level_pressure_pa")?;
    if sea_level_pressure <= 0.0 {
        return Err(out_of_range("sea_level_pressure_pa", sea_level_pressure));
    }
    let surface_temperature = ctx.dial("surface_temperature_k")?;
    if surface_temperature <= 0.0 {
        return Err(out_of_range("surface_temperature_k", surface_temperature));
    }
    let lapse = ctx.dial("lapse_k_per_m")?;
    if lapse < 0.0 {
        return Err(out_of_range("lapse_k_per_m", lapse));
    }
    let elevation = ctx.read(FieldId::Elevation)?;
    let mut pressure = Vec::with_capacity(ctx.len());
    for height in elevation {
        let value = if lapse == 0.0 {
            // The isothermal form the same law degenerates to, rather than a division by zero.
            sea_level_pressure
                * (-STANDARD_GRAVITY * MOLAR_MASS_DRY_AIR * height
                    / (UNIVERSAL_GAS_CONSTANT * surface_temperature))
                    .exp()
        } else {
            let bracket = 1.0 - lapse * height / surface_temperature;
            if bracket <= 0.0 {
                return Err(Refusal::new(
                    RefusalKind::ParameterOutOfRange,
                    format!("altitude {height} m is past the barometric model's domain"),
                ));
            }
            let exponent = STANDARD_GRAVITY * MOLAR_MASS_DRY_AIR / (UNIVERSAL_GAS_CONSTANT * lapse);
            sea_level_pressure * bracket.powf(exponent)
        };
        pressure.push(value);
    }
    Ok(Produced::one(FieldId::Pressure, pressure))
}

/// **`weather`** (Smith & Barstad 2004): orographic precipitation over the windward slope, scaled by
/// the air's moisture capacity from `climate`'s temperature via the Magnus/Tetens saturation form
/// (`exp(17.27·T/(T+237.3))`, normalised to one at 0 °C; Alduchov & Eskridge 1996). Reads
/// `elevation` and `temperature` — the second read is the stage-coupling rule made executable.
pub fn weather(ctx: &StageContext<'_>) -> Result<Produced, Refusal> {
    ctx.require_region()?;
    let base = ctx.dial("base_precipitation_mm")?;
    let gain = ctx.dial("orographic_gain")?;
    let wind_x = ctx.dial("wind_x")?;
    let wind_y = ctx.dial("wind_y")?;
    let elevation = ctx.read(FieldId::Elevation)?;
    let temperature = ctx.read(FieldId::Temperature)?;

    let mut precipitation = Vec::with_capacity(ctx.len());
    for (index, celsius) in temperature.iter().enumerate() {
        let celsius = *celsius;
        if celsius <= -237.3 {
            return Err(out_of_range("temperature_c", celsius));
        }
        let capacity = (17.27 * celsius / (celsius + 237.3)).exp();
        let (gx, gy) = terrain_gradient(ctx.patch, elevation, index);
        // Only the upslope component lifts air: the lee side gets the base rate, not a negative one.
        let lifting = (wind_x * gx + wind_y * gy).max(0.0);
        precipitation.push(base + gain * lifting * capacity);
    }
    Ok(Produced::one(FieldId::Precipitation, precipitation))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coordinates::VoxelPos;
    use crate::stage::{Dials, FieldValues, Patch, StageContext};
    use crate::world::{DeclaredPeriod, Tick, WorldIdentity};

    fn patch(side: u32) -> Patch {
        Patch::new(VoxelPos::new(0, 0, 0), side).expect("a patch")
    }

    fn context<'a>(
        patch: Patch,
        stage: &'static str,
        dials: &'a Dials,
        values: &'a FieldValues,
    ) -> StageContext<'a> {
        StageContext {
            patch,
            identity: WorldIdentity::new(3, DeclaredPeriod::seconds(1).expect("a period")),
            tick: Tick(0),
            ordinal: 8,
            stage,
            dials,
            values,
        }
    }

    #[test]
    fn climate_is_a_lapse_rate_over_the_terrain_it_reads() {
        let dials = Dials::new(&[("sea_level_temperature_c", 15.0), ("lapse_rate_c_per_m", 0.0065)]);
        let patch = patch(2);
        let mut values = FieldValues::new(patch);
        values.insert(FieldId::Elevation, vec![0.0, 1000.0, 2000.0, 3000.0]).expect("elevation");
        let climate = climate(&context(patch, "climate", &dials, &values)).expect("temperature");
        let temperature = climate.field(FieldId::Temperature).expect("temperature");
        assert!((temperature[0] - 15.0).abs() < 1e-12);
        assert!((temperature[1] - 8.5).abs() < 1e-12);
        assert!(temperature[3] < temperature[0], "height cools the air, not warms it");
    }

    #[test]
    fn a_stage_that_reads_the_terrain_refuses_without_it() {
        let dials = Dials::new(&[("sea_level_temperature_c", 15.0), ("lapse_rate_c_per_m", 0.0065)]);
        let empty = FieldValues::new(patch(2));
        let refused = climate(&context(patch(2), "climate", &dials, &empty))
            .expect_err("climate without terrain is not a climate");
        assert_eq!(refused.kind(), RefusalKind::StageInputAbsent);
        assert!(refused.detail().contains("elevation"), "{refused}");
    }

    #[test]
    fn pressure_falls_with_altitude_and_refuses_past_its_domain() {
        let dials = Dials::new(&[
            ("sea_level_pressure_pa", 101_325.0),
            ("surface_temperature_k", 288.15),
            ("lapse_k_per_m", 0.0065),
        ]);
        let patch = patch(2);
        let mut values = FieldValues::new(patch);
        values.insert(FieldId::Elevation, vec![0.0, 1000.0, 5000.0, 8848.0]).expect("elevation");
        let pressure = atmosphere(&context(patch, "atmosphere", &dials, &values)).expect("pressure");
        let pressure = pressure.field(FieldId::Pressure).expect("pressure");
        assert!((pressure[0] - 101_325.0).abs() < 1e-9, "sea level pressure is the declared one");
        assert!(pressure[1] < pressure[0] && pressure[2] < pressure[1] && pressure[3] < pressure[2]);

        // Past the model's bracket the law has no value: refused with the altitude, not clamped.
        let mut high = FieldValues::new(patch);
        high.insert(FieldId::Elevation, vec![60_000.0; 4]).expect("elevation");
        let refused = atmosphere(&context(patch, "atmosphere", &dials, &high))
            .expect_err("60000 m is past the barometric model");
        assert_eq!(refused.kind(), RefusalKind::ParameterOutOfRange);
        assert!(refused.detail().contains("60000"), "{refused}");
    }

    #[test]
    fn rain_falls_on_the_windward_slope_and_not_on_the_lee() {
        let dials = Dials::new(&[
            ("base_precipitation_mm", 100.0),
            ("orographic_gain", 2.0),
            ("wind_x", 1.0),
            ("wind_y", 0.0),
        ]);
        let patch = patch(3);
        // A ridge along x = 1 with the wind blowing toward +x: the west flank rises into the wind,
        // the east flank falls away from it, and the ridge line itself has no gradient.
        let mut values = FieldValues::new(patch);
        let elevation: Vec<f64> = vec![0.0, 200.0, 0.0, 0.0, 200.0, 0.0, 0.0, 200.0, 0.0];
        values.insert(FieldId::Elevation, elevation).expect("elevation");
        values.insert(FieldId::Temperature, vec![10.0; 9]).expect("temperature");

        let rain = weather(&context(patch, "weather", &dials, &values)).expect("precipitation");
        let rain = rain.field(FieldId::Precipitation).expect("precipitation");
        assert!(rain[0] > 100.0, "the windward flank gets more than the base rate: {}", rain[0]);
        assert!(
            (rain[2] - 100.0).abs() < 1e-12,
            "the lee slope gets the base rate, not a negative one: {}",
            rain[2]
        );
        assert!(
            (rain[1] - 100.0).abs() < 1e-12,
            "a ridge line has no gradient to lift air: {}",
            rain[1]
        );
        assert!(rain[0] > rain[2]);

        // The gradient reports the ramp's own slope, not a multiple of it.
        let elevation = values.get(FieldId::Elevation).expect("elevation");
        let (gx, gy) = terrain_gradient(patch, elevation, 0);
        assert!((gx - 200.0).abs() < 1e-12 && gy == 0.0, "gradient was ({gx}, {gy})");
    }

    #[test]
    fn weather_refuses_without_the_temperature_it_is_coupled_to() {
        let dials = Dials::new(&[
            ("base_precipitation_mm", 100.0),
            ("orographic_gain", 2.0),
            ("wind_x", 1.0),
            ("wind_y", 0.0),
        ]);
        let patch = patch(2);
        let mut values = FieldValues::new(patch);
        values.insert(FieldId::Elevation, vec![0.0, 1.0, 2.0, 3.0]).expect("elevation");
        let refused = weather(&context(patch, "weather", &dials, &values))
            .expect_err("orographic precipitation is coupled to the temperature");
        assert_eq!(refused.kind(), RefusalKind::StageInputAbsent);
        assert!(refused.detail().contains("temperature"), "{refused}");
    }
}
