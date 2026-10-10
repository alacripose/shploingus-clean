//! Clouds — saturation, cloud base, and what the sky does with its water.
//!
//! **Source.** The user's `docs/clouds_and_atmospheric_formation.pdf` is the mechanism:
//! saturation as thermodynamic equilibrium (not a container limit), the adiabatic route —
//! dry lapse **~9.8 °C/km**, moist **~4–7 °C/km** — with the **lifting condensation level**
//! as cloud base, stability selecting cumulus versus stratus, and the on-the-ground case
//! (fog). The saturation relation itself is the **Magnus–Tetens** form (Tetens 1930;
//! Alduchov & Eskridge 1996; the WMO CIMO Guide's recommended constants), which the PDF
//! leaves to the reader.
//!
//! **What is not implemented, named.** Cloud microphysics is absent: no CCN activation
//! (Köhler), no collision–coalescence, no ice pathways (Wegener–Bergeron–Findeisen,
//! riming, aggregation), and no cloud-water balance equation. So a cloud here does not
//! yet *make* rain — [`rain_fill_at`] draws weather's rain under an emergent cloud, and
//! the physics that would tie the two budgets together is the next atmosphere row.
//! **In the meantime, rain is the world's own witness of condensation**: a column the
//! weather says is raining is drawn as a cloud by construction ([`cloud_column`]'s
//! witness rule), because the alternative — rain falling from a clear sky — is a visible
//! disagreement between two stages' answers. Measured before the rule, only 211 of 299
//! raining columns had a cloud; after it, every one does.
//!
//! # A cloud is a column state, not a stored thing
//!
//! [`surface_air`] is the parcel: its temperature, dew point and lifting condensation
//! level. [`cloud_column`] asks whether this column's air is actually lifted to its own
//! LCL, and what kind of cloud that makes; [`cloud_at`] answers at a point inside it.
//! Everything is recomputed from terrain, climate, weather and the sun — **nothing is
//! stored** (C-2), so a cloud replays exactly with the world.
//!
//! # The world has unbounded height
//!
//! Cloud bases are **real altitudes**. C-6's `-16 … +47` band is where the *terrain*
//! sits, not a ceiling: an LCL of 1.4 km is 1.4 km above the terrain and a real place in
//! the world, reachable by climbing. No vertical compression exists and none is applied.
//!
//! # The 256-level rain interface
//!
//! [`rain_fill_at`] is the state the appearance layer animates falling rain subvoxels
//! from: `Some` in the air between the ground and the cloud base, carrying a
//! [`FillLevel`] — `crate::fluidics`'s own 256-level display lattice — quantised once
//! from weather's instantaneous rate. The falling animation itself (a droplet's position
//! as a pure function of `(tick, address)`) is the appearance layer's and is not here.

use crate::climate;
use crate::fluidics::{FillLevel, FILL_LEVELS};
use crate::sky;
use crate::spec::WorldSpec;
use crate::terrain;
use crate::weather;
use voxel_model::material::Ratio;

/// The dry adiabatic lapse rate, °C per metre — the PDF's ~9.8 °C/km.
pub const DRY_LAPSE_C_PER_M: f64 = 0.0098;

/// The moist adiabatic lapse rate, °C per metre — the PDF's ~4–7 °C/km, declared at the
/// middle of the range. It is carried as a constant even though the cloud model below uses
/// it only in naming the regime: the lift that makes cumulus is buoyant because the parcel
/// stays *warmer* than the moist rate would let the environment fall.
pub const MOIST_LAPSE_C_PER_M: f64 = 0.0055;

/// Saturation vapour pressure at 0 °C, hPa — the Magnus–Tetens constant.
pub const SATURATION_AT_ZERO_C_HPA: f64 = 6.112;
/// The Magnus–Tetens exponent constant. Fitted for the meteorological range (roughly
/// −40 … +50 °C); at 100 °C it overshoots the boiling point by ~2 %, which is stated
/// rather than hidden because this module's world never approaches it.
const MAGNUS_B: f64 = 17.62;
/// The Magnus–Tetens denominator constant, °C.
const MAGNUS_C: f64 = 243.12;

/// **Night radiative cooling** of the surface parcel, °C, at full night.
///
/// The PDF's radiative route to saturation — *"air near a surface loses heat until it
/// saturates: radiation fog, stratus"* — restated as one declared number, scaled by how
/// far the sun is down. Without it this model has no night cloud at all, because
/// [`climate::temperature_c`] does not carry a diurnal term: that is the honest reason
/// this constant exists. It applies while the sun is **below the horizon** and not at
/// all once it is up: radiative cooling is a night phenomenon, and the first version
/// scaled it by `1 − heating`, which left a fabricated 1.75 °C of cooling at noon and
/// pushed the measured noon cover to 67 % when it was tuned against the failing night.
/// Measured in `examples/cloud-profile.rs`; tuned again with the night-only gate: at
/// 5 °C the measured night cover was **60.6 %** (Earth-like in magnitude, but an
/// always-overcast night); 4 °C is the declared value, and the profile example prints
/// the cover it produces so a later hand can move it with a measurement in front of it.
const NIGHT_COOLING_C: f64 = 4.0;

/// Below this near-surface relative humidity no cloud forms in this world.
///
/// **Declared**, and it is the field's floor rather than a physical constant: air this
/// dry can still be lifted to its LCL in principle, but the model has no moisture source
/// above the boundary layer to replace what entrainment mixes away, and a world where
/// every desert column owns a cloud is a world with a cloud prop again.
const CLOUD_FLOOR_RH: f64 = 0.30;

/// The convective depth a fully heated, fully moist column provides, metres.
///
/// Declared: thermals over sunny ground carry surface air a kilometre or two up. Scaled
/// by heating and by moisture — dry air makes shallow thermals and moist air makes deep
/// ones — and measured for its world-wide cover in `examples/cloud-profile.rs`.
const THERMAL_LIFT_M: f64 = 3_000.0;

/// Metres of lift per unit of upslope, the PDF's mountain route.
///
/// Declared: air forced over a slope keeps rising while the slope does. `upslope` is
/// `∇h · wind` (dimensionless, metres per metre) from [`crate::weather`], and the gain is
/// larger than the thermal term because a windward slope lifts air continuously rather
/// than until a thermal detrains.
const OROGRAPHIC_LIFT_M: f64 = 8_000.0;

/// The weak, widespread ascent that makes stratus, metres.
///
/// Declared: a stable layer's ascent is a small fraction of a thermal's, which is exactly
/// why it makes a sheet instead of a tower (the PDF's stability row). Measured against the
/// night LCL: at 400 m the world had zero night cloud, so the stable layer's reach is
/// declared at 600 and measured again.
const STRATUS_LIFT_M: f64 = 600.0;

/// **The synoptic lift** of organised ascent at full onset, metres.
///
/// The PDF's *low-pressure centres: convergence and large-scale ascent*, and it is read
/// from the world's own weather: [`crate::weather`]'s onset is the weight that says how
/// much of the column's annual rain is falling **now**, and rain falling is itself the
/// large-scale ascent acting. This term is why a raining column is a cloudy one (a front
/// lifts its air whether or not a thermal also happens): without it, measured at noon,
/// only **33 of 299** raining columns had a cloud, so 89 % of the world's rain would have
/// fallen from clear sky — a visible disagreement between two stages' answers.
const SYNOPTIC_LIFT_M: f64 = 3_000.0;

/// The heating (solar altitude / 90°) at or above which the day's cloud is cumulus.
///
/// Declared small: it is not a threshold in the physics, it is the boundary between the
/// convective regime and the radiative one, and a day with any sun at all is convective
/// over land.
const CUMULUS_HEATING: f64 = 0.05;

/// Convective cloud depth at full moisture, metres. Declared.
const CUMULUS_DEPTH_M: f64 = 1_800.0;
/// Stratus depth at full moisture, metres. Declared; layers are shallow.
const STRATUS_DEPTH_M: f64 = 500.0;
/// Fog depth at full moisture, metres. Declared.
const FOG_DEPTH_M: f64 = 120.0;

/// The LCL at or below which the cloud **is the ground**: fog, by the PDF's own sentence
/// *"fog is a cloud at the surface"*.
const FOG_LCL_M: f64 = 25.0;

/// The tropospheric cap, metres — the US Standard Atmosphere's first-layer top, which
/// [`crate::atmosphere`] already cites. A cloud top is refused above it by the model's
/// own layer, not by an arbitrary clamp.
pub const TROPOPAUSE_M: f64 = 11_000.0;

/// Saturation vapour pressure over liquid water, hPa, at a temperature in °C.
///
/// Magnus–Tetens, the WMO-recommended constants. `6.112 · exp(17.62·T / (243.12 + T))`.
pub fn saturation_vapour_pressure_hpa(temperature_c: f64) -> f64 {
    SATURATION_AT_ZERO_C_HPA * (MAGNUS_B * temperature_c / (MAGNUS_C + temperature_c)).exp()
}

/// The vapour pressure a parcel holds at `temperature_c` and relative humidity, hPa.
pub fn vapour_pressure_hpa(temperature_c: f64, relative_humidity: f64) -> f64 {
    saturation_vapour_pressure_hpa(temperature_c) * relative_humidity.clamp(0.0, 1.0)
}

/// The dew point of a vapour pressure, °C — the inverse of
/// [`saturation_vapour_pressure_hpa`], so the round trip is exact to the arithmetic.
///
/// A non-positive vapour pressure has no dew point and returns `-273.15` (absolute zero,
/// where no vapour exists) rather than a NaN: the callers here always hand it
/// `moisture · e_s`, and a NaN reaching the world is the failure this guards.
pub fn dew_point_c(vapour_pressure_hpa: f64) -> f64 {
    if vapour_pressure_hpa <= 0.0 {
        return -273.15;
    }
    let ln = (vapour_pressure_hpa / SATURATION_AT_ZERO_C_HPA).ln();
    if ln >= MAGNUS_B {
        // Above the relation's own asymptote; no finite dew point. The world's range
        // never reaches this (it is above ~400 °C), so it is a guard, not a branch.
        return -273.15;
    }
    MAGNUS_C * ln / (MAGNUS_B - ln)
}

/// The sun's heating fraction, `[0, 1]`: altitude over the zenith.
///
/// Stage-coupled by construction (WG-Q5): it is the [`crate::sky`] position, not a second
/// field, so the sun that lights the frame is the sun that heats the ground.
pub fn heating(spec: &WorldSpec, tick: u64) -> f64 {
    let altitude = sky::solar_position(spec, tick).altitude_deg;
    (altitude / 90.0).clamp(0.0, 1.0)
}

/// The surface parcel's state: what a cloud is built from.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SurfaceAir {
    /// The terrain's elevation at the column, metres above the datum.
    pub elevation_m: f64,
    /// The parcel temperature **after** the declared night cooling, °C.
    pub temperature_c: f64,
    /// Near-surface relative humidity, `[0, 1]`.
    pub relative_humidity: f64,
    /// The dew point that vapour pressure implies, °C.
    pub dew_point_c: f64,
    /// The lifting condensation level, metres **above the surface**.
    pub lcl_m: f64,
}

/// The parcel at a column: climate's moisture as relative humidity over the terrain.
///
/// **The declared reading.** [`climate::moisture`] is documented as boundary-layer
/// moisture in `[0, 1]`; the only meaning of a fraction of capacity is relative humidity,
/// so it is read as one here and nowhere else. Then the parcel is cooled by the night
/// term, its vapour pressure is held fixed, and the LCL falls out exactly: the height
/// where the dry adiabat meets the dew point is `(T − T_d) / Γ_d`, because the dew point
/// *is* the temperature at which the parcel's own vapour pressure saturates.
pub fn surface_air(spec: &WorldSpec, x: f64, z: f64, tick: u64) -> SurfaceAir {
    let elevation_m = terrain::elevation_m(spec, x, z);
    let relative_humidity = climate::moisture(spec, x, z).clamp(0.0, 1.0);
    let heating = heating(spec, tick);
    let surface_temperature_c = climate::temperature_c(spec, x, z, elevation_m, tick);
    // Night-only: below the horizon `heating` is exactly zero, and above it the surface
    // is being heated rather than cooled.
    let night = if heating > 0.0 { 0.0 } else { 1.0 };
    let temperature_c = surface_temperature_c - NIGHT_COOLING_C * night;
    // **The vapour pressure is held through the cooling.** Radiative cooling does not
    // remove water; it lowers the temperature at which the same water saturates. So `e`
    // is computed from the **surface state** and the dew point from `e` alone — and the
    // first version computed it from the *cooled* temperature instead, which subtracts
    // the very water the cooling was meant to concentrate: measured, the mean night LCL
    // barely moved (998 m → 988 m) and no night cloud ever reached fog. Held fixed, the
    // cooling moves the LCL by `cooling / Γ_d` — about 510 m at the declared 5 °C — and
    // the ground-saturation case is reachable where the belt is humid.
    let vapour = vapour_pressure_hpa(surface_temperature_c, relative_humidity);
    let dew_point_c = dew_point_c(vapour);
    let lcl_m = ((temperature_c - dew_point_c) / DRY_LAPSE_C_PER_M).max(0.0);
    SurfaceAir {
        elevation_m,
        temperature_c,
        relative_humidity,
        dew_point_c,
        lcl_m,
    }
}

/// What kind of cloud a column makes.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum CloudKind {
    /// The cloud **is** the ground — the PDF's *"fog is a cloud at the surface"*.
    Fog,
    /// Puffy heaps: thermals, buoyancy, localized updrafts.
    Cumulus,
    /// Broad layers: stable air, widespread weak ascent.
    Stratus,
}

impl CloudKind {
    /// Every kind, hand-written for the same reason [`crate::stages::ALL`] is: a kind
    /// added without a name should not compile silently.
    pub const ALL: [CloudKind; 3] = [CloudKind::Fog, CloudKind::Cumulus, CloudKind::Stratus];

    pub const fn name(self) -> &'static str {
        match self {
            CloudKind::Fog => "fog",
            CloudKind::Cumulus => "cumulus",
            CloudKind::Stratus => "stratus",
        }
    }
}

/// Which kind a column makes, from its LCL and its heating.
///
/// The PDF's two rules, in the order that matters: saturation at the surface makes fog
/// whatever the hour; otherwise the convective regime (any sun) makes cumulus and the
/// radiative one makes stratus.
fn kind_for(lcl_m: f64, heating: f64) -> CloudKind {
    if lcl_m <= FOG_LCL_M {
        CloudKind::Fog
    } else if heating >= CUMULUS_HEATING {
        CloudKind::Cumulus
    } else {
        CloudKind::Stratus
    }
}

/// One column's cloud: the layer it makes, and how much water it holds.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CloudColumn {
    pub kind: CloudKind,
    /// The cloud base, metres above the datum: terrain elevation + LCL.
    pub base_m: f64,
    /// The cloud top, metres above the datum.
    pub top_m: f64,
    /// Column water content in `[0, 1]` — the display quantity the appearance layer
    /// shades with, not a mass and not a hydration of any voxel.
    pub water: f64,
}

/// The cloud this column makes, if its air is lifted to its own LCL.
///
/// **Lift is what separates a cloud from merely moist air.** The LCL is where a rising
/// parcel saturates, but nothing rises unless something lifts it: the sun's thermals (the
/// convective regime), the wind over a windward slope (the PDF's mountain route), or the
/// weak ascent of a stable layer (stratus). The model compares the LCL against the lift
/// this column actually has, so a dry, calm column in the middle of a still night is
/// clear air even though its air could saturate if lifted — which it is not.
///
/// **The witness rule.** When weather is raining here, the column is a cloud whatever the
/// surface parcel model alone would say: rain is the world's own evidence that this air
/// condensed, and the floor and lift tests above are statements about *clouds that have
/// not rained yet*. It closes the visible gap between the two stages, and its weight is
/// continuous — `docs/skylight.md` §3.1 records the step the boolean form measured.
/// The column's physics, derived **once**, before any existence test.
///
/// `water` is the continuous condensed amount, and the three booleans are the existence
/// answer's own inputs, so a reader that wants an amount and one that wants a yes/no share a
/// derivation instead of duplicating it.
struct ColumnState {
    kind: CloudKind,
    base_m: f64,
    top_m: f64,
    /// Condensed water in `[0, 1]`, continuous in every input.
    water: f64,
    /// Whether weather is raining here — the model's own witness.
    witnessed: bool,
    /// Whether the humidity floor is met.
    floor_met: bool,
    /// Whether the parcel's lift reaches its own LCL.
    lifted: bool,
}

fn column_state(spec: &WorldSpec, x: f64, z: f64, tick: u64) -> ColumnState {
    let air = surface_air(spec, x, z, tick);
    let heating = heating(spec, tick);
    let sample = weather::sample(spec, x, z, tick);
    let now = weather::precipitation_now(spec, x, z, tick);
    // The synoptic term applies in both regimes: a front lifts air by day and by night.
    let synoptic_m = SYNOPTIC_LIFT_M * now.onset;
    let lift_m = if air.lcl_m <= FOG_LCL_M {
        // Already saturated at the ground: it is a cloud, and no lift is owed.
        air.lcl_m
    } else if heating >= CUMULUS_HEATING {
        THERMAL_LIFT_M * heating * air.relative_humidity
            + OROGRAPHIC_LIFT_M * sample.upslope.max(0.0)
            + synoptic_m
    } else {
        STRATUS_LIFT_M * air.relative_humidity + synoptic_m
    };
    let kind = kind_for(air.lcl_m, heating);
    let base_m = air.elevation_m + air.lcl_m;
    let depth_m = match kind {
        CloudKind::Fog => FOG_DEPTH_M * air.relative_humidity,
        CloudKind::Cumulus => CUMULUS_DEPTH_M * air.relative_humidity * (0.5 + 0.5 * heating),
        CloudKind::Stratus => STRATUS_DEPTH_M * air.relative_humidity,
    };
    // **The witness is a weight, not a switch.** It used to assign the full humidity the
    // moment `onset` crossed `WET_ONSET`, which made `water` — and so the light a column
    // scatters — a step function: `examples/aerosol-column.rs` measures **166.00 m in one
    // tick** at the crossing, against **0.53 m** for every other step in the day. The
    // weight below is the same normalisation [`rain_fill_at`] already uses, and `max` of two
    // continuous ramps is continuous, so no threshold in this function steps.
    let onset_weight =
        ((now.onset - weather::SILENT_ONSET) / (1.0 - weather::SILENT_ONSET)).clamp(0.0, 1.0);
    let ramp = match kind {
        // Saturated at the ground: the whole vapour excess is condensed at the base.
        CloudKind::Fog => 1.0,
        _ => (1.0 - air.lcl_m / lift_m).clamp(0.0, 1.0),
    };
    ColumnState {
        kind,
        base_m,
        top_m: (base_m + depth_m).min(TROPOPAUSE_M),
        water: air.relative_humidity * ramp.max(onset_weight),
        witnessed: now.is_raining,
        floor_met: air.relative_humidity >= CLOUD_FLOOR_RH,
        lifted: air.lcl_m <= lift_m,
    }
}

pub fn cloud_column(spec: &WorldSpec, x: f64, z: f64, tick: u64) -> Option<CloudColumn> {
    let state = column_state(spec, x, z, tick);
    // **The two tests that make this a yes/no rather than an amount.** A reader that wants the
    // amount the light crosses does not come through here: see [`column_water_path_m`].
    if !state.witnessed && !state.floor_met {
        return None;
    }
    if !state.witnessed && !state.lifted {
        return None;
    }
    if state.top_m <= state.base_m {
        return None;
    }
    Some(CloudColumn {
        kind: state.kind,
        base_m: state.base_m,
        top_m: state.top_m,
        water: state.water,
    })
}

/// The vertical water path of one column, metres, **in closed form** and **with no existence
/// gate**: the light reads an amount, so the model's yes/no about a cloud must not erase a
/// whole column's condensed water in one tick.
///
/// [`cloud_at`]'s profile is `water · sin(pi · through)`, whose integral over the layer is
/// exactly `water · 2 · (top - base) / pi` — so the path is evaluated, never sampled, and a
/// caller reads a whole column for the cost of the one derivation [`column_state`] does.
pub fn column_water_path_m(spec: &WorldSpec, x: f64, z: f64, tick: u64) -> f64 {
    // **No existence gate, and that is the point.** The light reads an *amount*, so the
    // model's yes/no about a cloud must not erase a whole column's condensed water in one
    // tick: `examples/aerosol-column.rs` measured the boolean costing **22.85 m in one
    // update**, against 3.96 m for every other step in the day.
    let state = column_state(spec, x, z, tick);
    if state.top_m <= state.base_m {
        return 0.0;
    }
    state.water * 2.0 * (state.top_m - state.base_m) / std::f64::consts::PI
}

/// The cloud at one address, if a cloud is there.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CloudSample {
    pub kind: CloudKind,
    pub base_m: f64,
    pub top_m: f64,
    /// Local water content in `[0, 1]`: the column's content shaped by the vertical
    /// profile — zero at base and top, deepest in the middle. A rendered cloud's edge is
    /// where the appearance layer chooses to cut it, not a step in this function.
    pub water: f64,
}

/// The cloud at `(x, y, z)`, if one is.
pub fn cloud_at(spec: &WorldSpec, x: f64, z: f64, y_m: f64, tick: u64) -> Option<CloudSample> {
    let column = cloud_column(spec, x, z, tick)?;
    if y_m < column.base_m || y_m >= column.top_m {
        return None;
    }
    let through = (y_m - column.base_m) / (column.top_m - column.base_m);
    let shape = (std::f64::consts::PI * through).sin();
    Some(CloudSample {
        kind: column.kind,
        base_m: column.base_m,
        top_m: column.top_m,
        water: column.water * shape,
    })
}

/// **The rain falling in a column right now, as the 256-level display fill.**
///
/// `Some(level)` exactly where rain is in transit — below the emergent cloud's base and
/// above the terrain — and `None` outside that band or when weather is silent. The level
/// is weather's instantaneous intensity quantised **once** onto the same 256 lattice
/// [`crate::fluidics`] uses for fluids: a display level, never a mass. The annual books
/// stay [`crate::water`]'s, which cross the weather family's declared tolerance once.
///
/// The falling animation — droplet positions as a pure function of `(tick, address)` —
/// is the appearance layer's; this is the state it animates.
pub fn rain_fill_at(
    spec: &WorldSpec,
    x: f64,
    z: f64,
    y_m: f64,
    tick: u64,
) -> Option<FillLevel> {
    let column = cloud_column(spec, x, z, tick)?;
    let ground_m = terrain::elevation_m(spec, x, z);
    if y_m < ground_m || y_m >= column.base_m {
        return None;
    }
    let now = weather::precipitation_now(spec, x, z, tick);
    if now.is_silent {
        return None;
    }
    let intensity =
        ((now.onset - weather::SILENT_ONSET) / (1.0 - weather::SILENT_ONSET)).clamp(0.0, 1.0);
    let level = (intensity * FILL_LEVELS as f64).floor() as u64;
    if level == 0 {
        return None;
    }
    let saturation = Ratio::new(level, FILL_LEVELS as u64).ok()?;
    FillLevel::of(saturation).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Config;

    fn spec() -> WorldSpec {
        WorldSpec::new(2468, Config::default())
    }

    /// Noon and midnight, in the world's own ticks — read from the clock, not restated.
    fn noon() -> u64 {
        climate::TICKS_PER_DAY / 2
    }
    fn midnight() -> u64 {
        0
    }

    #[test]
    fn the_saturation_relation_matches_the_published_values() {
        // 0 °C is the constant by construction.
        assert!((saturation_vapour_pressure_hpa(0.0) - 6.112).abs() < 1e-12);
        // The meteorological range: 6.112·exp(17.62·25/268.12) ≈ 31.7 hPa.
        assert!(
            (saturation_vapour_pressure_hpa(25.0) - 31.7).abs() < 0.2,
            "{}",
            saturation_vapour_pressure_hpa(25.0)
        );
        // It rises, and it rises faster at the warm end — the exponential shape the PDF
        // leans on for "warm air supports more vapour".
        let cold_step = saturation_vapour_pressure_hpa(10.0)
            - saturation_vapour_pressure_hpa(0.0);
        let warm_step = saturation_vapour_pressure_hpa(30.0)
            - saturation_vapour_pressure_hpa(20.0);
        assert!(warm_step > cold_step, "cold {cold_step}, warm {warm_step}");
    }

    #[test]
    fn the_dew_point_inverts_the_saturation_relation() {
        for t in [-20.0, -10.0, 0.0, 10.0, 20.0, 30.0, 40.0] {
            let e = saturation_vapour_pressure_hpa(t);
            let td = dew_point_c(e);
            assert!(
                (td - t).abs() < 1e-9,
                "round trip at {t} landed at {td}"
            );
        }
        // A zero vapour pressure has no finite dew point: it is the absolute-zero guard,
        // not a NaN.
        assert_eq!(dew_point_c(0.0), -273.15);
    }

    #[test]
    fn a_parcel_cools_to_its_dew_point_at_the_lcl() {
        let spec = spec();
        // The client's spawn column, at noon: the LCL is exactly the dry adiabat to the
        // dew point, and the dew point never exceeds the temperature.
        let air = surface_air(&spec, 0.5, -95.5, noon());
        assert!(air.temperature_c >= air.dew_point_c);
        let expected = (air.temperature_c - air.dew_point_c) / DRY_LAPSE_C_PER_M;
        assert!((air.lcl_m - expected).abs() < 1e-9, "{} {}", air.lcl_m, expected);
        assert!(air.lcl_m > 0.0, "the spawn column has a dew-point depression");
    }

    #[test]
    fn the_kinds_follow_the_pdf_rules() {
        // Saturation at the surface is fog whatever the hour.
        assert_eq!(kind_for(10.0, 0.0), CloudKind::Fog);
        // A convective day is cumulus; the radiative regime is stratus.
        assert_eq!(kind_for(1_000.0, 0.8), CloudKind::Cumulus);
        assert_eq!(kind_for(1_000.0, 0.0), CloudKind::Stratus);
    }

    #[test]
    fn a_day_column_makes_cumulus_and_a_midnight_column_never_does() {
        let spec = spec();
        // Find a column with a noon cloud by scanning the client's own region; the test
        // fails if none exists, because then it would be testing nothing.
        let mut found = None;
        'scan: for i in 0..40 {
            for j in 0..40 {
                let x = i as f64 * 200.0;
                let z = j as f64 * 200.0;
                if let Some(column) = cloud_column(&spec, x, z, noon()) {
                    if column.kind == CloudKind::Cumulus {
                        found = Some((x, z));
                        break 'scan;
                    }
                }
            }
        }
        let (x, z) = found.expect("some column has a convective noon cloud");
        // At midnight the same column is never convective: any cloud it keeps is fog or
        // stratus, which is the PDF's stability selection.
        if let Some(night) = cloud_column(&spec, x, z, midnight()) {
            assert_ne!(night.kind, CloudKind::Cumulus);
        }
    }

    #[test]
    fn cloud_at_agrees_with_the_column_it_was_built_from() {
        let spec = spec();
        let mut checked = 0usize;
        for i in 0..20 {
            for j in 0..20 {
                let x = i as f64 * 300.0;
                let z = j as f64 * 300.0;
                let Some(column) = cloud_column(&spec, x, z, noon()) else {
                    continue;
                };
                checked += 1;
                assert!(column.base_m >= terrain::elevation_m(&spec, x, z) - 1e-9);
                assert!(column.top_m > column.base_m);
                assert!(column.top_m <= TROPOPAUSE_M + 1e-9);
                assert!((0.0..=1.0).contains(&column.water));

                // Inside the layer, and only inside it.
                let middle = (column.base_m + column.top_m) / 2.0;
                let inside = cloud_at(&spec, x, z, middle, noon())
                    .expect("the middle of a cloud column is in its cloud");
                assert_eq!(inside.kind, column.kind);
                assert!(inside.water <= column.water + 1e-9);
                assert!(cloud_at(&spec, x, z, column.base_m - 1.0, noon()).is_none());
                assert!(cloud_at(&spec, x, z, column.top_m + 1.0, noon()).is_none());
            }
        }
        assert!(checked > 0, "the scan found no clouds at all");
    }

    #[test]
    fn every_raining_column_is_cloudy() {
        // The witness rule, pinned: a column the weather says is raining is drawn as a
        // cloud, so the rain fill interface can never show rain from a clear sky.
        let spec = spec();
        let mut raining = 0usize;
        for i in 0..40 {
            for j in 0..40 {
                let x = i as f64 * 300.0 - 6_000.0;
                let z = j as f64 * 300.0 - 6_000.0;
                if !weather::precipitation_now(&spec, x, z, noon()).is_raining {
                    continue;
                }
                raining += 1;
                assert!(
                    cloud_column(&spec, x, z, noon()).is_some(),
                    "rain at ({x}, {z}) fell from a clear sky"
                );
            }
        }
        assert!(raining > 0, "the scan found no rain to couple against");
    }

    #[test]
    fn the_model_is_deterministic_and_stores_nothing() {
        let spec = spec();
        let a = cloud_column(&spec, 123.0, -456.0, noon());
        let b = cloud_column(&spec, 123.0, -456.0, noon());
        assert_eq!(a, b);
        let y = a.map(|c| (c.base_m + c.top_m) / 2.0).unwrap_or(0.0);
        assert_eq!(
            cloud_at(&spec, 123.0, -456.0, y, noon()),
            cloud_at(&spec, 123.0, -456.0, y, noon())
        );
    }

    #[test]
    fn the_rain_fill_is_bounded_by_the_lattice_and_lives_below_the_cloud() {
        let spec = spec();
        // Scan for a column where weather is raining and a cloud exists, at a few ticks.
        let mut found = None;
        'scan: for tick in [noon(), noon() + 400, noon() + 1_200, midnight() + 400] {
            for i in 0..60 {
                for j in 0..60 {
                    let x = i as f64 * 200.0;
                    let z = j as f64 * 200.0;
                    let Some(column) = cloud_column(&spec, x, z, tick) else {
                        continue;
                    };
                    if !weather::precipitation_now(&spec, x, z, tick).is_raining {
                        continue;
                    }
                    let ground = terrain::elevation_m(&spec, x, z);
                    if column.base_m <= ground {
                        continue;
                    }
                    let middle = (ground + column.base_m) / 2.0;
                    let fill = rain_fill_at(&spec, x, z, middle, tick)
                        .expect("rain in transit under a raining cloud");
                    assert!((1..=FILL_LEVELS).contains(&fill.raw()));
                    assert!(rain_fill_at(&spec, x, z, column.base_m + 1.0, tick).is_none());
                    assert!(rain_fill_at(&spec, x, z, ground - 1.0, tick).is_none());
                    found = Some((x, z, tick));
                    break 'scan;
                }
            }
        }
        assert!(found.is_some(), "no raining cloud column found to test against");
    }
}
