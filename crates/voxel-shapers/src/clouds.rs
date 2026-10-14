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
//!
//! **The second source, adopted 2026-10-10.** The user's
//! `docs/cloud_types_fronts_and_altitudes_guide.pdf` — the ten WMO genera, their
//! approximate base and top heights, and the frontal systems that arrange them — is the
//! classification and altitude half of the model, and it is what [`CloudGenus`],
//! [`Envelope`], [`CloudLevel`], [`CloudPhase`], [`FrontKind`] and [`tropopause_m`] are.
//! Its numbers are the guide's own, in metres, and its one rule is quoted where it binds:
//! *"a cloud name describes what is observed; it does not, by itself, prove which weather
//! system produced it"* — so the genus is derived from **this column's own base and
//! depth** and the front is derived from the world's own ascent, and neither is read off
//! the other.
//!
//! **Cubic conformance, 2026-10-10.** The ledger's ruling is that the column is superseded and
//! `column` is a **sampling detail** (C-119.2, C-153): a chunk samples declared fields on their
//! lattices, and the column composition is not the foundation. This module now says the same thing
//! in the plane's own vocabulary — its state is **declared as fields** and read at an address:
//! `CloudWater` (this voxel's own local content, zero where there is no cloud), `CloudBase` and
//! `CloudTop` (the deck above the address, refusing by name where the column makes none), all three
//! produced in [`crate::chunk_fields`] from [`cloud_at`] and [`cloud_column`]. What that buys is the
//! cubic question this module could not answer before: a voxel *above* a deck reads zero water
//! while the column beneath it has a cloud, and a chunk can ask its own voxels instead of slicing a
//! per-column object.
//!
//! **What is still column-shaped, named rather than implied.** [`cloud_column`] derives base, top,
//! genus and water from `(x, z)` and [`column_water_path_m`] integrates that layer in closed form —
//! so the *values* are per-column even though the *contract* is per-address, and every address read
//! re-derives its column. That is a memo's job, not a second model: `voxel_stream`'s `ColumnMemo`
//! already carries this pattern for the material path, and the client's [`crate::clouds`] reads
//! (`voxel-client/src/cloud_field.rs`, `cloud_cubes.rs`) are still per-column callers. Both are owed
//! work rather than settled design, and neither is hidden by this paragraph.
//!
//! **What the guide changed, and what it did not.** The base was, and is, the lifting
//! condensation level; the top was a declared depth and is now, for an organized ascent,
//! **where the shield reaches at this column** — the guide's §9 sequence (*"the deck
//! thickens and lowers"*) read as a function of the world's own onset, so a column at a
//! front's leading edge carries the high thin cloud and a column at its core carries the
//! deep precipitating one. The tropopause became latitude-dependent (§12) and the cloud
//! domain is sized on its mid-latitude value. Measured over a 20 km square
//! (`examples/cloud-profile.rs`): **eight of the ten genera** appear, and 81–89 % of
//! columns land inside their own genus's envelope, which the guide says are typical rather
//! than hard.
//!
//! **What the guide asks for and this does not do, named.**
//!
//! * **One column, one cloud.** A real sky stacks layers — the guide's own §12 warning that
//!   *"a deep convective cloud can cross low, middle, and high levels"* — and a column here
//!   carries one base and one top. So `Stratocumulus` is the one genus the world never
//!   makes: an organized ascent's deck **replaces** the boundary-layer sheet instead of
//!   standing under it, and a warm sector's low deck is exactly that missing second layer.
//! * **No convective instability.** §10's cumulonimbus needs CAPE and overshoot; this model
//!   has lift, so `Cb` is reachable only where a slope's own lift stacks high enough to
//!   clear 8 km above the condensation level, which the world almost never does. Measured:
//!   **0 columns** in the profile's square.
//! * **No cyclone phase, so no occlusion.** Guide §11: an occluded front is a *stage* of a
//!   mid-latitude cyclone — the cold front catching the warm one — and there is no cyclone
//!   here to have stages. See [`OCCLUSION_IS_A_CYCLONE_PHASE`].
//! * **No species or varieties.** `uncinus`, `castellanus`, `lenticularis`, `undulatus`…
//!   are not genera and are not modelled; the guide is explicit that *"not every combination
//!   is allowed"*, and a species is a shape this model cannot yet state.
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
use crate::scale;
use crate::sky;
use crate::spec::WorldSpec;
use crate::terrain;
use crate::weather;
use voxel_model::material::Ratio;

/// The dry adiabatic lapse rate, °C per metre — the PDF's ~9.8 °C/km.
pub const DRY_LAPSE_C_PER_M: f64 = scale::per_voxel(0.0098);

/// The moist adiabatic lapse rate, °C per metre — the PDF's ~4–7 °C/km, declared at the
/// middle of the range. It is carried as a constant even though the cloud model below uses
/// it only in naming the regime: the lift that makes cumulus is buoyant because the parcel
/// stays *warmer* than the moist rate would let the environment fall.
pub const MOIST_LAPSE_C_PER_M: f64 = scale::per_voxel(0.0055);

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
const THERMAL_LIFT_M: f64 = scale::voxels(3_000.0);

/// Metres of lift per unit of upslope, the PDF's mountain route.
///
/// Declared: air forced over a slope keeps rising while the slope does. `upslope` is
/// `∇h · wind` (dimensionless, metres per metre) from [`crate::weather`], and the gain is
/// larger than the thermal term because a windward slope lifts air continuously rather
/// than until a thermal detrains.
const OROGRAPHIC_LIFT_M: f64 = scale::voxels(8_000.0);

/// The weak, widespread ascent that makes stratus, metres.
///
/// Declared: a stable layer's ascent is a small fraction of a thermal's, which is exactly
/// why it makes a sheet instead of a tower (the PDF's stability row). Measured against the
/// night LCL: at 400 m the world had zero night cloud, so the stable layer's reach is
/// declared at 600 and measured again.
const STRATUS_LIFT_M: f64 = scale::voxels(600.0);

/// **The synoptic lift** of organised ascent at full onset, metres.
///
/// The PDF's *low-pressure centres: convergence and large-scale ascent*, and it is read
/// from the world's own weather: [`crate::weather`]'s onset is the weight that says how
/// much of the column's annual rain is falling **now**, and rain falling is itself the
/// large-scale ascent acting. This term is why a raining column is a cloudy one (a front
/// lifts its air whether or not a thermal also happens): without it, measured at noon,
/// only **33 of 299** raining columns had a cloud, so 89 % of the world's rain would have
/// fallen from clear sky — a visible disagreement between two stages' answers.
const SYNOPTIC_LIFT_M: f64 = scale::voxels(3_000.0);

/// The heating (solar altitude / 90°) at or above which the day's cloud is cumulus.
///
/// Declared small: it is not a threshold in the physics, it is the boundary between the
/// convective regime and the radiative one, and a day with any sun at all is convective
/// over land.
const CUMULUS_HEATING: f64 = 0.05;

/// Convective cloud depth at full moisture, metres. Declared.
const CUMULUS_DEPTH_M: f64 = scale::voxels(1_800.0);
/// Stratus depth at full moisture, metres. Declared; layers are shallow.
const STRATUS_DEPTH_M: f64 = scale::voxels(500.0);
/// Fog depth at full moisture, metres. Declared.
const FOG_DEPTH_M: f64 = scale::voxels(120.0);

/// The LCL at or below which the cloud **is the ground**: fog, by the PDF's own sentence
/// *"fog is a cloud at the surface"*.
const FOG_LCL_M: f64 = scale::voxels(25.0);

/// The tropospheric cap, metres — the US Standard Atmosphere's first-layer top, which
/// [`crate::atmosphere`] already cites. A cloud top is refused above it by the model's
/// own layer, not by an arbitrary clamp.
///
/// **The mid-latitude value**, and [`tropopause_m`] is the world's own: the guide (§12)
/// says the tropopause is higher in the tropics and lower near the poles, and a world
/// whose origin is the declared 45° reads this constant exactly. The cloud domain the
/// cloud-cube pass subdivides is sized on this number, so a world moved far from 45°
/// keeps its ceiling here while its own physics moves — stated rather than hidden.
pub const TROPOPAUSE_M: f64 = scale::voxels(11_000.0);

/// The tropopause at the pole, metres: the guide's *"lower near the poles"* (standard
/// atmosphere, ~8 km). See [`tropopause_m`].
pub const TROPOPAUSE_POLES_M: f64 = scale::voxels(8_000.0);

/// The tropopause over the equator, metres: the guide's *"tropical anvils can be higher
/// than comparable mid-latitude anvils"* (standard atmosphere, ~16 km).
pub const TROPOPAUSE_TROPICS_M: f64 = scale::voxels(16_000.0);

/// How far ahead the synoptic field is read when a front's own motion is measured, ticks —
/// six hours, a quarter of the world's day. Declared: the guide's §14 tracks a feature for
/// 10–30 minutes by eye, which is a person watching one cloud, and a boundary's own
/// approach is the slower, synoptic quantity.
const FRONT_DRIFT_TICKS: u64 = climate::TICKS_PER_DAY / 4;

/// The onset change over [`FRONT_DRIFT_TICKS`] below which a boundary is **stationary** —
/// *"a motion description, not a cloud type"* (guide §11). Declared.
const FRONT_STATIONARY_EPS: f64 = 0.02;

/// The cloud base at the **leading edge** of an organized ascent, metres above the ground:
/// the guide's warm-front sequence begins with *"high ice wisps"* well ahead (Ci, base
/// 6–13 km), so the far edge of a shield over a column is its high, thin part.
const SHIELD_LEADING_BASE_M: f64 = scale::voxels(6_500.0);

/// The shield's depth at that leading edge, metres — a veil, not a deck.
const SHIELD_LEADING_DEPTH_M: f64 = scale::voxels(400.0);

/// The shield's depth at the front itself, metres: the guide's nimbostratus *"vertical
/// span: commonly several kilometres, extending through the middle layer"*.
const SHIELD_CORE_DEPTH_M: f64 = scale::voxels(4_500.0);

/// The onset at or above which an organized ascent is the guide's **concentrated** kind,
/// `[0, 1]` of the field's range — the lifting-strength row of §12, and the boundary
/// between a cold front's towers and a warm front's shield.
///
/// Declared, and read against the field rather than picked: [`weather::WET_ONSET`] is 0.18
/// (rain begins) and the field's own maximum is about 0.55, so 0.24 is the upper half of
/// the rainy part of the field — a tower is what a strong lift under an unstable column
/// makes, and most of a wet day is not that.
const COLD_FRONT_ONSET: f64 = 0.24;

/// The onset at which an organized ascent has reached a front's own core, `[0, 1]` of the
/// field's range — the weight at which the deck has thickened to its full depth and its
/// base has come down to the column's own condensation level.
///
/// Declared, and read against the field's measured distribution rather than picked: the
/// onset field's own maximum is about 0.55 and [`weather::WET_ONSET`] is 0.18, so a core
/// at 0.30 is the top third of the wet part of the field — a front's core is the rare
/// case and the shield's approach is the common one, which is what the guide's five-step
/// sequence says it is.
const SHIELD_CORE_ONSET: f64 = 0.30;

/// The base above which a layered cloud is one of the **high** genera (guide §3: *"base
/// often above about 5 km; commonly 6–13 km"*). Declared at the middle of the two, so the
/// high bands the genus envelopes carry are what the selection actually uses.
const HIGH_CLOUD_BASE_M: f64 = scale::voxels(6_000.0);

/// The base above which a layered or elevated convective cloud is a **middle** genus
/// (guide §4: middle clouds *"roughly 2–7 km"*, low clouds *"surface to roughly 2 km"*).
const MIDDLE_CLOUD_BASE_M: f64 = scale::voxels(2_000.0);

/// The depth below which a high cloud is **thin** — the guide's leading-edge `Cirrus` and
/// `Cirrocumulus` against the veil that `Cirrostratus` is.
///
/// Declared just above [`SHIELD_LEADING_DEPTH_M`], and calibrated on the shield rather than
/// chosen: the leading edge of an organized ascent is `400` m deep and thickens toward the
/// front, so a threshold far above the leading depth would make `Cs` unreachable and one at
/// it would make `Ci` unreachable — measured: at `1 200` m the world made **no `Cs` at all**
/// (there is nowhere on the shield both high enough and deep enough), and `600` is the
/// value that puts the wisps at the very edge and the veil behind them.
const HIGH_THIN_DEPTH_M: f64 = scale::voxels(600.0);

/// The depth at or above which a **precipitating** layer is the guide's `Nimbostratus`:
/// *"vertical span: commonly several kilometres"*. Declared at two.
const NIMBOSTRATUS_DEPTH_M: f64 = scale::voxels(2_000.0);

/// The top at or above which a convective column is the guide's `Cumulonimbus`: *"top: may
/// reach 8–16 km"*. Declared at the low end of that band, which is where the genus begins.
const CUMULONIMBUS_TOP_M: f64 = scale::voxels(8_000.0);

/// The upslope magnitude, metres per metre, at or above which a column stands in a
/// **wave** — the structure the guide gives to the element genera: `Altocumulus`'s
/// lenticularis (*"smooth lenses near mountain waves"*) and `Stratocumulus`'s cells
/// (*"air flowing over terrain"*), and `Cirrocumulus`'s ripples.
///
/// Read on the **absolute** value, because the guide's lenticular clouds stand in the lee
/// as well as on the windward slope. Declared: a 2 % slope with the wind on it is a wave a
/// person would name, and the world's terrain is steeper than that in most of its hills.
const WAVE_UPSLOPE: f64 = 0.02;

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
/// **The ascent's own character**, which is the guide's §10/§12 row rather than the hour:
/// a cold front's lift is *"more abrupt"* and builds towers where the column is unstable
/// and moist, while a warm front's is *"broad and comparatively gradual"* and makes a
/// shield — **by day or by night**, which the sun alone cannot say.
///
/// So the test is a comparison of the two lifts this column actually has: the thermal it
/// would get from the sun against the organized ascent the weather is bringing. A summer
/// afternoon with a front's synoptic lift under a strong thermal stays convective (the
/// guide's cold-front case); a winter dawn with the same synoptic lift has no thermal to
/// sharpen it and is layered (the warm-front case).
fn kind_for_column(lcl_m: f64, heating: f64, front: Option<Front>) -> CloudKind {
    if lcl_m <= FOG_LCL_M {
        CloudKind::Fog
    } else if let Some(front) = front {
        // **A front decides the shape before the hour does**, which is the whole of §9 and
        // §10: a shield is a shield at noon, and a cold front's towers do not wait for the
        // sun to heat the ground. Without this, every daytime frontal column read as
        // convective and the guide's five-step sequence never appeared by day at all
        // (measured in `examples/cloud-profile.rs` before the rule: no high genus at noon).
        if front.convective {
            CloudKind::Cumulus
        } else {
            CloudKind::Stratus
        }
    } else if heating >= CUMULUS_HEATING {
        // No front: the guide's ordinary day, where the sun's own thermals are the lift.
        CloudKind::Cumulus
    } else {
        CloudKind::Stratus
    }
}

/// **Which front a column is under**, from the world's own weather — or `None` when its
/// air is not being lifted by one.
///
/// The guide's §11 row for what a front *is*: *"a boundary between air masses"*, and its
/// §9/§10 rows for what each one does. This world carries no cyclone and no air-mass
/// labels, so the two things it does carry are read instead: the **ascent** at the column
/// ([`weather::onset`], the weight saying how much of the column's annual rain is falling
/// now — which *is* the large-scale ascent acting) and **how that ascent is changing**,
/// from the same pure function a quarter-day on. A boundary that is deepening over the
/// column is one arriving; one that is fading is leaving; one that is neither is the
/// guide's stationary front, which is a motion description and not a cloud type.
///
/// `convective` is [`kind_for_column`]'s own answer: it is what decides whether an
/// arriving boundary is the abrupt kind (§10) or the gliding kind (§9).
pub fn front_at(spec: &WorldSpec, x: f64, z: f64, tick: u64) -> Option<Front> {
    let now = weather::onset(spec, x, z, tick);
    if now < weather::SILENT_ONSET {
        return None;
    }
    let ahead = weather::onset(spec, x, z, tick + FRONT_DRIFT_TICKS);
    let advance = ahead - now;
    let air = surface_air(spec, x, z, tick);
    let heating = heating(spec, tick);
    // **The guide's §12 lifting-strength row, as the one test this model can make**: a
    // *gentle, widespread* ascent glides and makes a shield, a *concentrated strong* one
    // under a moist, unstable column can build towers. The on-slope measure of how
    // concentrated the ascent is is its own weight — the onset — against a declared
    // threshold, and both `cold` and `warm` need the column to be moist enough for either.
    let convective = heating >= CUMULUS_HEATING
        && air.relative_humidity >= CLOUD_FLOOR_RH
        && now >= COLD_FRONT_ONSET;
    let kind = if advance.abs() <= FRONT_STATIONARY_EPS {
        FrontKind::Stationary
    } else if convective {
        FrontKind::Cold
    } else {
        FrontKind::Warm
    };
    Some(Front { kind, advance, convective })
}

/// **The genus a column's own state makes** — the guide's bands doing the classifying.
///
/// The guide's rule first: *"a cloud name describes what is observed"*. So the inputs here
/// are all **this column's**, in metres **above its own ground** (the guide's chart is
/// drawn that way, and a cirrus 7 km above a 2 km plateau is at 9 km above the datum): the
/// shape class, the base and top, the depth, and whether the column's ascent is the
/// convective kind. The front is read only for what the guide says it arranges — a
/// **shield's own edge** is high and thin, its **core** is deep (§9's sequence is the
/// shield thickening and lowering), and a cold front's band is towers (§10).
fn genus_for(
    kind: CloudKind,
    base_agl_m: f64,
    top_agl_m: f64,
    depth_m: f64,
    raining: bool,
    waved: bool,
    heating: f64,
) -> CloudGenus {
    match kind {
        // The guide's own note (its fog panel): *"fog is a cloud at or very near the ground"*
        // and is not a genus, so a ground cloud is the genus that describes a ground layer.
        CloudKind::Fog => CloudGenus::Stratus,
        CloudKind::Cumulus => {
            // §4: an elevated convective base is the guide's `Altocumulus castellanus`,
            // *"turrets rising from a common base, evidence of elevated instability"*.
            if base_agl_m >= MIDDLE_CLOUD_BASE_M && top_agl_m < CUMULONIMBUS_TOP_M {
                return CloudGenus::Altocumulus;
            }
            // §6: the top reaching the guide's 8–16 km band is a cumulonimbus, and the
            // vertical extent is what makes one — *"the vertical extent is more important
            // than a single 'cloud height'"*.
            if top_agl_m >= CUMULONIMBUS_TOP_M {
                return CloudGenus::Cumulonimbus;
            }
            CloudGenus::Cumulus
        }
        CloudKind::Stratus => {
            // §4: nimbostratus is the deep precipitating layer, and rain is what makes it
            // one rather than a thick altostratus.
            if raining && depth_m >= NIMBOSTRATUS_DEPTH_M {
                return CloudGenus::Nimbostratus;
            }
            if base_agl_m >= HIGH_CLOUD_BASE_M {
                // §3, whose three high genera differ by *texture*: a rippled thin deck is
                // cirrocumulus (*"tiny white cloudlets, grains, or ripples"* — §8 names
                // gravity waves as what makes one), the plain thin streaks are cirrus, and
                // the thick veil that spreads and lowers is cirrostratus — the guide's own
                // *"spreading veil that thickens and lowers"*, the warm front's first sign.
                if depth_m < HIGH_THIN_DEPTH_M && waved {
                    return CloudGenus::Cirrocumulus;
                }
                if depth_m < HIGH_THIN_DEPTH_M {
                    return CloudGenus::Cirrus;
                }
                return CloudGenus::Cirrostratus;
            }
            if base_agl_m >= MIDDLE_CLOUD_BASE_M {
                // §4: both middle genera live here, and **the wave is what separates them** —
                // altocumulus is the one with elements (*"rounded patches or rolls"*, and
                // §4's `lenticularis` is explicitly *"smooth lenses near mountain waves"*),
                // while altostratus is the *"widespread, grey or blue-grey layer"* with no
                // structure in it. Depth is not the separator it was first written as: a
                // smooth shield is thick too, and using depth made every night deck an
                // `Ac` and no `As` ever (measured in `examples/cloud-profile.rs`).
                if waved {
                    return CloudGenus::Altocumulus;
                }
                return CloudGenus::Altostratus;
            }
            // §5: the low deck, separated the same way — §5 gives stratocumulus' own
            // formation as *"shallow convection, or air flowing over terrain"*, and stratus
            // as the *"smooth, uniform sheet"* that neither of those makes.
            if waved || heating >= CUMULUS_HEATING {
                return CloudGenus::Stratocumulus;
            }
            CloudGenus::Stratus
        }
    }
}

// ===========================================================================
// The genera, their altitudes, and the fronts that arrange them
// ===========================================================================
//
// **Source.** The user's `docs/cloud_types_fronts_and_altitudes_guide.pdf` — a field
// guide to the ten WMO genera, their approximate base and top heights, and the frontal
// systems that organize them into shields and bands. Its own reference basis is the WMO
// International Cloud Atlas, NOAA/NWS and Met Office educational material, and the
// frontal-organization research it cites (Hobbs 1978; Browning 1990; Catto et al. 2012;
// Houze 2014; Markowski & Richardson 2010).
//
// The guide states its levels as **approximate envelopes, not hard boundaries**, and
// repeats one rule that shapes every decision below: *"a cloud name describes what is
// observed; it does not, by itself, prove which weather system produced it."* So the
// genus is derived from **this column's own state** — its base, its depth, its water,
// the phase it reaches — and the front is what arranges a shield or a band across many
// columns. Neither is read off the other.

/// A genus's approximate altitude envelope, metres above the datum.
///
/// Envelopes, and the guide says so twice: *"typical simplified envelopes, not hard
/// boundaries… cloud bases and tops shift with latitude, season, weather, and terrain."*
/// The model uses them as the **band a genus may occupy**, so a picture can say which
/// genus a column is making and be held to the height the guide gives that genus.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Envelope {
    /// Base range, `(low, high)`, metres above the datum.
    pub base_m: (f64, f64),
    /// Top range, `(low, high)`, metres above the datum.
    pub top_m: (f64, f64),
}

impl Envelope {
    /// Whether a base and top both lie inside the envelope — the check the tests make on
    /// every column the model produces.
    pub fn holds(&self, base_m: f64, top_m: f64) -> bool {
        base_m >= self.base_m.0 - 1e-6
            && base_m <= self.base_m.1 + 1e-6
            && top_m >= self.top_m.0 - 1e-6
            && top_m <= self.top_m.1 + 1e-6
            && top_m > base_m
    }
}

/// Why the cloud water is what it is: the guide's phase notes, by level.
///
/// §3: high clouds *"are commonly composed mainly of ice crystals because temperatures
/// are very cold"*. §4: middle clouds are *"liquid droplets, ice crystals, or both,
/// depending on temperature and vertical depth"*. §5: low clouds *"liquid droplets are
/// common"*. §6: a deep convective tower holds *"supercooled water, ice crystals,
/// graupel, and hail"* at once.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum CloudPhase {
    /// Droplets: the low genera.
    Liquid,
    /// Both: the middle genera and the lower part of a deep tower.
    Mixed,
    /// Ice crystals: the high genera, and the glaciated top of a cumulonimbus.
    Ice,
}

/// The guide's four level rows, which is how it groups the ten genera.
///
/// Ordered low to high, so a report can sort by it and a count can be keyed on it.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum CloudLevel {
    /// Surface to about 2 km.
    Low,
    /// Roughly 2–7 km.
    Middle,
    /// Base often above about 5 km, commonly 6–13 km.
    High,
    /// Vertically developed: a low base and a top that may reach 8–16 km.
    Deep,
}

/// **The ten internationally recognized genera**, and the envelope each occupies.
///
/// The guide's own ten, in its own order (high, middle, low, vertically developed). A
/// species (`uncinus`, `castellanus`, `congestus`…) and a variety (`undulatus`,
/// `radiatus`…) are *not* genera and are not modelled yet — named as owed rather than
/// flattened into the genus, because the guide is explicit that *"not every combination
/// is allowed"* and a species is a shape this model cannot yet state.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum CloudGenus {
    Cirrus,
    Cirrostratus,
    Cirrocumulus,
    Altocumulus,
    Altostratus,
    Nimbostratus,
    Stratus,
    Stratocumulus,
    Cumulus,
    Cumulonimbus,
}

impl CloudGenus {
    /// Every genus, hand-written for the reason [`CloudKind::ALL`] is.
    pub const ALL: [CloudGenus; 10] = [
        CloudGenus::Cirrus,
        CloudGenus::Cirrostratus,
        CloudGenus::Cirrocumulus,
        CloudGenus::Altocumulus,
        CloudGenus::Altostratus,
        CloudGenus::Nimbostratus,
        CloudGenus::Stratus,
        CloudGenus::Stratocumulus,
        CloudGenus::Cumulus,
        CloudGenus::Cumulonimbus,
    ];

    /// The genus's official abbreviation, as the guide and every observation use it.
    pub const fn abbreviation(self) -> &'static str {
        match self {
            CloudGenus::Cirrus => "Ci",
            CloudGenus::Cirrostratus => "Cs",
            CloudGenus::Cirrocumulus => "Cc",
            CloudGenus::Altocumulus => "Ac",
            CloudGenus::Altostratus => "As",
            CloudGenus::Nimbostratus => "Ns",
            CloudGenus::Stratus => "St",
            CloudGenus::Stratocumulus => "Sc",
            CloudGenus::Cumulus => "Cu",
            CloudGenus::Cumulonimbus => "Cb",
        }
    }

    /// The genus's name, lower case as the guide writes it.
    pub const fn name(self) -> &'static str {
        match self {
            CloudGenus::Cirrus => "cirrus",
            CloudGenus::Cirrostratus => "cirrostratus",
            CloudGenus::Cirrocumulus => "cirrocumulus",
            CloudGenus::Altocumulus => "altocumulus",
            CloudGenus::Altostratus => "altostratus",
            CloudGenus::Nimbostratus => "nimbostratus",
            CloudGenus::Stratus => "stratus",
            CloudGenus::Stratocumulus => "stratocumulus",
            CloudGenus::Cumulus => "cumulus",
            CloudGenus::Cumulonimbus => "cumulonimbus",
        }
    }

    /// The guide's level row for this genus.
    ///
    /// **Cumulus and cumulonimbus are [`CloudLevel::Deep`]**, which is the guide's own
    /// grouping (§6, *"vertically developed"*) even though its low-cloud row lists their
    /// *bases*: a cumulus base is low and its extent is what makes it a cumulus, and the
    /// guide's own warning — *"a deep convective cloud can cross low, middle, and high
    /// levels"* — is why the level is read from the genus and not from the base.
    pub const fn level(self) -> CloudLevel {
        match self {
            CloudGenus::Cirrus | CloudGenus::Cirrostratus | CloudGenus::Cirrocumulus => {
                CloudLevel::High
            }
            CloudGenus::Altocumulus | CloudGenus::Altostratus | CloudGenus::Nimbostratus => {
                CloudLevel::Middle
            }
            CloudGenus::Stratus | CloudGenus::Stratocumulus => CloudLevel::Low,
            CloudGenus::Cumulus | CloudGenus::Cumulonimbus => CloudLevel::Deep,
        }
    }

    /// Why this genus is ice, liquid, or both (the guide's §3–§6 phase notes).
    pub const fn phase(self) -> CloudPhase {
        match self {
            CloudGenus::Cirrus | CloudGenus::Cirrostratus | CloudGenus::Cirrocumulus => {
                CloudPhase::Ice
            }
            CloudGenus::Altocumulus
            | CloudGenus::Altostratus
            | CloudGenus::Nimbostratus
            | CloudGenus::Cumulonimbus => CloudPhase::Mixed,
            CloudGenus::Stratus | CloudGenus::Stratocumulus | CloudGenus::Cumulus => {
                CloudPhase::Liquid
            }
        }
    }

    /// **The genus's approximate altitude envelope**, metres above the datum.
    ///
    /// Every band below is the guide's own number where the guide gives one, in metres:
    /// high clouds *"base often above about 5 km; commonly 6–13 km"* (Ci, and the guide's
    /// own 6–12 km for Cs and Cc); middle *"often 2–7 km"* (Ac, As); nimbostratus *"base
    /// often low to middle, roughly 0.5–3 km"* with *"a vertical span commonly several
    /// kilometres, extending through the middle layer"*; stratus *"surface to about
    /// 1 km"* and stratocumulus *"surface to about 2 km"*; cumulus *"often 0.5–2.5 km"*
    /// with limited vertical depth; cumulonimbus *"base often around 0.5–2 km, top may
    /// reach 8–16 km and often spreads near the tropopause"*.
    ///
    /// **What the guide does not give, this states as the model's own fill** and not as a
    /// source: the top of a shallow genus (a stratus sheet's own depth, a cumulus humilis
    /// top). Those are declared as the band a *layer* occupies, and the model keeps its
    /// own thinner depths ([`STRATUS_DEPTH_M`] and friends) inside them.
    pub const fn envelope(self) -> Envelope {
        match self {
            // The guide's envelopes are **real-world** altitudes; each passes through the
            // world's scale, so a genus's band stays the same fraction of the sky it is.
            CloudGenus::Cirrus => Envelope {
                base_m: (scale::voxels(6_000.0), scale::voxels(13_000.0)),
                top_m: (scale::voxels(6_200.0), scale::voxels(13_000.0)),
            },
            CloudGenus::Cirrostratus => Envelope {
                base_m: (scale::voxels(6_000.0), scale::voxels(12_000.0)),
                top_m: (scale::voxels(6_200.0), scale::voxels(12_500.0)),
            },
            CloudGenus::Cirrocumulus => Envelope {
                base_m: (scale::voxels(6_000.0), scale::voxels(12_000.0)),
                top_m: (scale::voxels(6_200.0), scale::voxels(12_500.0)),
            },
            CloudGenus::Altocumulus => Envelope {
                base_m: (scale::voxels(2_000.0), scale::voxels(7_000.0)),
                top_m: (scale::voxels(2_200.0), scale::voxels(7_500.0)),
            },
            CloudGenus::Altostratus => Envelope {
                base_m: (scale::voxels(2_000.0), scale::voxels(7_000.0)),
                top_m: (scale::voxels(2_200.0), scale::voxels(8_000.0)),
            },
            CloudGenus::Nimbostratus => Envelope {
                base_m: (scale::voxels(500.0), scale::voxels(3_000.0)),
                top_m: (scale::voxels(3_500.0), scale::voxels(11_000.0)),
            },
            CloudGenus::Stratus => Envelope {
                base_m: (0.0, scale::voxels(1_000.0)),
                top_m: (scale::voxels(120.0), scale::voxels(1_600.0)),
            },
            CloudGenus::Stratocumulus => Envelope {
                base_m: (0.0, scale::voxels(2_000.0)),
                top_m: (scale::voxels(300.0), scale::voxels(2_600.0)),
            },
            CloudGenus::Cumulus => Envelope {
                base_m: (scale::voxels(500.0), scale::voxels(2_500.0)),
                top_m: (scale::voxels(700.0), scale::voxels(3_500.0)),
            },
            CloudGenus::Cumulonimbus => Envelope {
                base_m: (scale::voxels(500.0), scale::voxels(2_000.0)),
                top_m: (scale::voxels(8_000.0), scale::voxels(16_000.0)),
            },
        }
    }

    /// The shape class this genus is drawn as.
    ///
    /// **The classification and the geometry are different questions**, and this is where
    /// they meet: [`CloudKind`] is the shape the cloud-cube pass subdivides (a ground
    /// cloud, a heap, a sheet), while the genus is what an observer would name. A veil at
    /// 8 km and a fog at 10 m are both sheets; the difference the picture shows is their
    /// base and depth, which the column already carries.
    pub const fn shape(self) -> CloudKind {
        match self {
            CloudGenus::Cumulus | CloudGenus::Cumulonimbus => CloudKind::Cumulus,
            _ => CloudKind::Stratus,
        }
    }
}

/// **The fronts of the guide's §§9–11**, as this world's own weather can state them.
///
/// A front is a boundary between air masses (guide §11). What the model has is
/// [`crate::weather`]'s onset — the weight saying how much of a column's annual rain is
/// falling now, which is the large-scale ascent acting — sampled **on the world's own
/// clock**. So a front's *approach* is measurable: the same column's onset a few hours
/// on. Its *character* is the guide's §10 row: a cold front wedges under the warm air and
/// lifts it abruptly (convective), a warm front glides (layered).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum FrontKind {
    /// Warm air replacing cold: a gentle slope, a broad layered shield (§9).
    Warm,
    /// Cold air wedging under warm: sharper lift, towers and bands (§10).
    Cold,
    /// *"A motion description, not a cloud type"* (§11): the boundary is here and not
    /// moving, and repeated lift keeps the sky cloudy.
    Stationary,
}

impl FrontKind {
    /// Every kind, hand-written for the reason [`CloudKind::ALL`] is.
    pub const ALL: [FrontKind; 3] = [FrontKind::Warm, FrontKind::Cold, FrontKind::Stationary];

    pub const fn name(self) -> &'static str {
        match self {
            FrontKind::Warm => "warm front",
            FrontKind::Cold => "cold front",
            FrontKind::Stationary => "stationary front",
        }
    }

    /// Whether this front's ascent is the guide's **layered** kind (§9: *"uplift is broad
    /// and comparatively gradual, cloud tends to occupy a large shield"*) rather than §10's
    /// *"more abrupt"* lifting that deepens cumulus into congestus and cumulonimbus.
    pub const fn is_layered(self) -> bool {
        !matches!(self, FrontKind::Cold)
    }
}

/// **An occlusion is a cyclone phase this world does not carry, and that is stated rather
/// than faked.** The guide (§11) is explicit that an occluded front is a *stage* of a
/// mid-latitude cyclone — *"the faster-moving cold front may catch up with the warm
/// front, the warm air is lifted above the surface"* — so what makes one is the **history**
/// of two fronts over a day, and this model is a pure function of the column and the tick
/// with no cyclone tracking behind it. A column an occlusion would be sitting under is
/// therefore classified as what it locally is: a warm front that has caught up, or a cold
/// front whose warm sector is already aloft.
pub const OCCLUSION_IS_A_CYCLONE_PHASE: &str = "an occluded front is a stage of a cyclone, not a state of one column";

/// A front at a column, if one is there.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Front {
    pub kind: FrontKind,
    /// The onset's own change over [`FRONT_DRIFT_TICKS`], `[0, 1]` of the field's range.
    /// Positive is a boundary deepening over this column — the shield arriving.
    pub advance: f64,
    /// Whether this column's ascent is the convective kind (§10's *"warm, moist air"*
    /// under an unstable column), which is what a cold front sharpens.
    pub convective: bool,
}

/// The tropopause's own height, metres — **latitude-dependent**, because the guide's §12
/// says the top of the troposphere is: *"the upper limit of the troposphere is higher in
/// the tropics and lower near the poles"*, which is why *"tropical anvils can be higher
/// than comparable mid-latitude anvils"*.
///
/// The values are the standard-atmosphere ones the guide's table implies — **16 km** in
/// the tropics, **8 km** at the poles — with a two-term fit that lands the mid-latitudes
/// on **11 km**, which is [`TROPOPAUSE_M`] itself. So a world whose origin is the
/// declared 45° reads exactly the constant the cloud domain is sized on, and a world
/// moved toward the equator or the pole reads its own.
pub fn tropopause_m(spec: &WorldSpec) -> f64 {
    let cos2 = spec.config.origin_latitude_deg.to_radians().cos().powi(2);
    // The polar value at `cos² = 0`, and the tropical one at `cos² = 1`: the two terms
    // below sum to the **span** between the two constants at the equator, so the fit holds
    // whatever the world's scale is. It used to spell the span as a literal `4_000 + 4_000`
    // — the real-world half-span — which stopped summing to the tropics the moment the
    // heights were scaled, and the test caught it rather than restating either number here.
    let half_span = (TROPOPAUSE_TROPICS_M - TROPOPAUSE_POLES_M) * 0.5;
    TROPOPAUSE_POLES_M + half_span * cos2 + half_span * cos2 * cos2
}

/// One column's cloud: the layer it makes, its genus, and how much water it holds.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CloudColumn {
    pub kind: CloudKind,
    /// **What an observer would name it** (the guide's ten genera).
    pub genus: CloudGenus,
    /// The front this column is under, if the world's own weather puts one here.
    pub front: Option<Front>,
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
    genus: CloudGenus,
    front: Option<Front>,
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
    let thermal_m = THERMAL_LIFT_M * heating * air.relative_humidity;
    let orographic_m = OROGRAPHIC_LIFT_M * sample.upslope.max(0.0);
    let lift_m = if air.lcl_m <= FOG_LCL_M {
        // Already saturated at the ground: it is a cloud, and no lift is owed.
        air.lcl_m
    } else if heating >= CUMULUS_HEATING {
        thermal_m + orographic_m + synoptic_m
    } else {
        STRATUS_LIFT_M * air.relative_humidity + synoptic_m
    };
    let front = front_at(spec, x, z, tick);
    let kind = kind_for_column(air.lcl_m, heating, front);
    // **The shield's own depth at this column**, the guide's §9 sequence read as a
    // function of how far into the organized ascent the column stands: at the leading
    // edge only the high thin part is overhead, and at the core the deck has thickened
    // and lowered onto the column's own condensation level.
    let shield = ((now.onset - weather::SILENT_ONSET)
        / (SHIELD_CORE_ONSET - weather::SILENT_ONSET))
        .clamp(0.0, 1.0);
    let (base_m, depth_m) = match kind {
        CloudKind::Fog => (
            air.elevation_m + air.lcl_m,
            FOG_DEPTH_M * air.relative_humidity,
        ),
        CloudKind::Cumulus => (
            air.elevation_m + air.lcl_m,
            // **The depth is the part of the lift above the condensation level**, because
            // that is where the rising air is a cloud rather than a parcel — and it is why
            // the guide's cumulonimbus (top 8–16 km) is reachable at all: a declared depth
            // of two kilometres can never make one, and a column lifted nine can.
            (lift_m - air.lcl_m).max(
                CUMULUS_DEPTH_M * air.relative_humidity * (0.5 + 0.5 * heating),
            ),
        ),
        CloudKind::Stratus => {
            let leading_base = air.elevation_m + SHIELD_LEADING_BASE_M;
            let core_base = air.elevation_m + air.lcl_m;
            let layer = STRATUS_DEPTH_M * air.relative_humidity;
            let base = leading_base * (1.0 - shield) + core_base * shield;
            let depth = layer.max(
                SHIELD_LEADING_DEPTH_M
                    + (SHIELD_CORE_DEPTH_M - SHIELD_LEADING_DEPTH_M) * shield,
            );
            (base, depth)
        }
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
    let top_m = (base_m + depth_m).min(tropopause_m(spec));
    let genus = genus_for(
        kind,
        base_m - air.elevation_m,
        top_m - air.elevation_m,
        top_m - base_m,
        now.is_raining,
        sample.upslope.abs() >= WAVE_UPSLOPE,
        heating,
    );
    ColumnState {
        kind,
        genus,
        front,
        base_m,
        top_m,
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
        genus: state.genus,
        front: state.front,
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
    /// The genus the column it belongs to makes — one cloud, named once.
    pub genus: CloudGenus,
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
        genus: column.genus,
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
        let warm = Some(Front { kind: FrontKind::Warm, advance: 0.02, convective: false });
        let cold = Some(Front { kind: FrontKind::Cold, advance: 0.05, convective: true });
        // The LCLs are **real-world** heights put through the world's scale, so the arms
        // mean the same sky at any scale: ten real metres is ground fog, a real kilometre is
        // a cloud.
        let fog = scale::voxels(10.0);
        let deck = scale::voxels(1_000.0);
        // Saturation at the surface is fog whatever the hour, front or not.
        assert_eq!(kind_for_column(fog, 0.0, None), CloudKind::Fog);
        assert_eq!(kind_for_column(fog, 0.0, warm), CloudKind::Fog);
        // A convective day with no front is cumulus; the radiative regime is stratus.
        assert_eq!(kind_for_column(deck, 0.8, None), CloudKind::Cumulus);
        assert_eq!(kind_for_column(deck, 0.0, None), CloudKind::Stratus);
        // **And a front decides the shape before the hour does** — a shield is a shield at
        // noon and a cold front's towers do not wait for the sun (C-183).
        assert_eq!(kind_for_column(deck, 0.8, warm), CloudKind::Stratus);
        assert_eq!(kind_for_column(deck, 0.0, cold), CloudKind::Cumulus);
    }

    #[test]
    fn the_ten_genera_are_the_guides_ten_and_carry_their_abbreviations() {
        // The WMO's ten genera, by the abbreviations every observation uses. A genus added
        // or dropped without touching this list is the failure: the count is the claim.
        let pairs = [
            (CloudGenus::Cirrus, "Ci", CloudLevel::High),
            (CloudGenus::Cirrostratus, "Cs", CloudLevel::High),
            (CloudGenus::Cirrocumulus, "Cc", CloudLevel::High),
            (CloudGenus::Altocumulus, "Ac", CloudLevel::Middle),
            (CloudGenus::Altostratus, "As", CloudLevel::Middle),
            (CloudGenus::Nimbostratus, "Ns", CloudLevel::Middle),
            (CloudGenus::Stratus, "St", CloudLevel::Low),
            (CloudGenus::Stratocumulus, "Sc", CloudLevel::Low),
            (CloudGenus::Cumulus, "Cu", CloudLevel::Deep),
            (CloudGenus::Cumulonimbus, "Cb", CloudLevel::Deep),
        ];
        assert_eq!(CloudGenus::ALL.len(), 10, "the guide names ten genera");
        for (genus, abbreviation, level) in pairs {
            assert_eq!(genus.abbreviation(), abbreviation);
            assert_eq!(genus.level(), level, "{abbreviation}");
            assert!(
                CloudGenus::ALL.contains(&genus),
                "{abbreviation} is missing from ALL"
            );
            // A high genus is ice and a low or fair-weather one is liquid — the guide's
            // phase notes; a cumulonimbus is the mixed one it says holds *"supercooled
            // water, ice crystals, graupel, and hail"* at once, and the middle genera are
            // *"liquid droplets, ice crystals, or both"*.
            let phase = match genus {
                CloudGenus::Cirrus | CloudGenus::Cirrostratus | CloudGenus::Cirrocumulus => {
                    CloudPhase::Ice
                }
                CloudGenus::Stratus | CloudGenus::Stratocumulus | CloudGenus::Cumulus => {
                    CloudPhase::Liquid
                }
                CloudGenus::Altocumulus
                | CloudGenus::Altostratus
                | CloudGenus::Nimbostratus
                | CloudGenus::Cumulonimbus => CloudPhase::Mixed,
            };
            assert_eq!(genus.phase(), phase, "{abbreviation}");
        }
    }

    #[test]
    fn the_envelopes_are_the_guides_own_altitudes() {
        // The numbers the guide states, checked against the tables rather than restated:
        // high clouds *"commonly 6–13 km"* (Ci) and *"about 6–12 km"* (Cs, Cc); middle
        // clouds *"roughly 2–7 km"* (Ac, As); nimbostratus *"base often low to middle,
        // roughly 0.5–3 km"*; stratus *"surface to about 1 km"*; stratocumulus *"surface
        // to about 2 km"*; cumulus *"often 0.5–2.5 km"*; cumulonimbus *"base often around
        // 0.5–2 km, top may reach 8–16 km"*.
        // The guide's heights are real-world ones and the table stores them scaled, so the
        // assertion converts the same real numbers rather than quoting engine metres — a
        // frozen literal here would pass at one scale and lie at the next.
        let r = |real_m: f64| scale::voxels(real_m);
        let high = CloudGenus::Cirrus.envelope();
        assert_eq!(high.base_m, (r(6_000.0), r(13_000.0)));
        assert_eq!(CloudGenus::Cirrostratus.envelope().base_m, (r(6_000.0), r(12_000.0)));
        assert_eq!(CloudGenus::Cirrocumulus.envelope().base_m, (r(6_000.0), r(12_000.0)));
        assert_eq!(CloudGenus::Altocumulus.envelope().base_m, (r(2_000.0), r(7_000.0)));
        assert_eq!(CloudGenus::Altostratus.envelope().base_m, (r(2_000.0), r(7_000.0)));
        assert_eq!(CloudGenus::Nimbostratus.envelope().base_m, (r(500.0), r(3_000.0)));
        assert_eq!(CloudGenus::Stratus.envelope().base_m, (0.0, r(1_000.0)));
        assert_eq!(CloudGenus::Stratocumulus.envelope().base_m, (0.0, r(2_000.0)));
        assert_eq!(CloudGenus::Cumulus.envelope().base_m, (r(500.0), r(2_500.0)));
        let cb = CloudGenus::Cumulonimbus.envelope();
        assert_eq!(cb.base_m, (r(500.0), r(2_000.0)));
        assert_eq!(cb.top_m, (r(8_000.0), r(16_000.0)));
        // And every envelope can hold a cloud: a band whose top is below its base would be
        // a genus nothing could ever be.
        for genus in CloudGenus::ALL {
            let e = genus.envelope();
            assert!(e.top_m.1 > e.base_m.0, "{} has no room", genus.abbreviation());
            assert!(e.top_m.0 > e.base_m.0, "{}'s top starts below its base", genus.abbreviation());
        }
    }

    #[test]
    fn the_tropopause_falls_from_the_tropics_to_the_poles() {
        let at = |latitude: f64| {
            let mut config = Config::default();
            config.origin_latitude_deg = latitude;
            tropopause_m(&WorldSpec::new(2468, config))
        };
        assert!((at(0.0) - TROPOPAUSE_TROPICS_M).abs() < 1e-6, "{}", at(0.0));
        assert!((at(90.0) - TROPOPAUSE_POLES_M).abs() < 1e-6, "{}", at(90.0));
        // **The world's own 45° is the constant the cloud domain is sized on**, which is
        // what keeps a moved world's ceiling from cutting its own clouds off.
        assert!((at(45.0) - TROPOPAUSE_M).abs() < 1e-6, "{}", at(45.0));
        // And it decreases: toward the pole every step is lower, the guide's §12 row.
        let mut previous = f64::INFINITY;
        for step in 0..=90 {
            let height = at(step as f64);
            assert!(height < previous, "the tropopause rose at {step}°");
            previous = height;
        }
    }

    #[test]
    fn a_clouds_genus_is_its_own_base_and_top_band_and_never_the_shield_it_is_not() {
        // **The classification, measured rather than sampled.** Every column the world makes
        // is classified by its own base and depth, so the guide's bands are load-bearing:
        // a High genus can only be one whose base is in the high band, a Middle one below
        // that, and a Low or Deep one below that — which is what makes `Ci`, `As` and `St`
        // statements about the picture rather than labels on it.
        let spec = spec();
        let tropopause = tropopause_m(&spec);
        let mut levels = std::collections::BTreeMap::new();
        let mut inside = 0usize;
        let mut checked = 0usize;
        for tick in [noon(), midnight()] {
            for i in 0..60 {
                for j in 0..60 {
                    let x = i as f64 * 250.0 - 7_000.0;
                    let z = j as f64 * 250.0 - 7_000.0;
                    let Some(column) = cloud_column(&spec, x, z, tick) else {
                        continue;
                    };
                    checked += 1;
                    *levels.entry(column.genus.level()).or_insert(0usize) += 1;
                    let ground = terrain::elevation_m(&spec, x, z);
                    let base_agl = column.base_m - ground;
                    let top_agl = column.top_m - ground;
                    match column.genus.level() {
                        CloudLevel::High => assert!(base_agl >= HIGH_CLOUD_BASE_M, "{base_agl}"),
                        // **Nimbostratus is the middle genus whose base is not middle**, and
                        // the guide says so itself: *"base often low to middle, roughly
                        // 0.5–3 km"*. It is a Middle genus because of its span, not its base.
                        CloudLevel::Middle => assert!(
                            base_agl >= MIDDLE_CLOUD_BASE_M
                                || column.genus == CloudGenus::Nimbostratus,
                            "{} at {base_agl} m above its ground",
                            column.genus.abbreviation()
                        ),
                        CloudLevel::Low => assert!(
                            base_agl < MIDDLE_CLOUD_BASE_M,
                            "{} at {base_agl} m above its ground",
                            column.genus.abbreviation()
                        ),
                        // The deep genera are chosen from the convective kind, so an
                        // elevated convective base is already `Altocumulus` (a Middle genus).
                        CloudLevel::Deep => assert!(matches!(
                            column.genus,
                            CloudGenus::Cumulus | CloudGenus::Cumulonimbus
                        )),
                    }
                    // Every top is inside the tropopause, whatever the genus.
                    assert!(column.top_m <= tropopause + 1e-6, "{}", column.top_m);
                    // A cumulonimbus is what the guide says it is: a top in the 8–16 km band
                    // and a base in the low band, which is what makes it vertical.
                    if column.genus == CloudGenus::Cumulonimbus {
                        assert!(top_agl >= CUMULONIMBUS_TOP_M - 1e-6, "{top_agl}");
                    }
                    if column.genus.envelope().holds(base_agl, top_agl) {
                        inside += 1;
                    }
                }
            }
        }
        assert!(checked > 0, "the world made no clouds to classify");
        // **A reading, printed**: how many columns of the world land inside their genus's
        // own envelope. The guide says its envelopes are typical and not hard, so this is
        // reported rather than asserted at 100 %; what *is* asserted is that the bands
        // classify (above) and that more than half the world sits where the guide says.
        println!(
            "{checked} columns: {inside} inside their genus's envelope ({} %), levels {levels:?}",
            inside * 100 / checked
        );
        assert!(inside * 100 / checked > 50, "{inside} of {checked} inside");
    }

    #[test]
    fn a_fronts_shield_is_high_and_thin_far_ahead_and_deep_and_low_at_its_core() {
        // The guide's §9 sequence — *"the deck thickens and lowers"* — as a fact about the
        // model: over the world, a column with organized ascent and almost no rain has a
        // high, thin cloud, and a raining one has a low base and a deep layer.
        let spec = spec();
        let mut thin_high = 0usize;
        let mut deep_low = 0usize;
        let mut fronts = std::collections::BTreeMap::new();
        for tick in [noon(), noon() + 3_000, midnight(), midnight() + 9_000] {
            for i in 0..70 {
                for j in 0..70 {
                    let x = i as f64 * 250.0 - 8_000.0;
                    let z = j as f64 * 250.0 - 8_000.0;
                    let Some(column) = cloud_column(&spec, x, z, tick) else {
                        continue;
                    };
                    if let Some(front) = column.front {
                        *fronts.entry(front.kind.name()).or_insert(0usize) += 1;
                        // **A front's cloud is layered when the front is**, which is the
                        // guide's own ascent-character row: only §10's abrupt lifting
                        // deepens cumulus into cumulonimbus.
                        // **A warm front is the guide's shield and nothing else**: §9's
                        // ascent is gentle, so no column under one may be a tower. A
                        // stationary front is only a motion description (§11), so what it
                        // makes depends on its lifting strength and is not asserted here.
                        if front.kind == FrontKind::Warm {
                            assert_ne!(
                                column.genus,
                                CloudGenus::Cumulonimbus,
                                "a warm front at ({x}, {z}) made a cumulonimbus"
                            );
                        }
                    }
                    let ground = terrain::elevation_m(&spec, x, z);
                    let base_agl = column.base_m - ground;
                    if column.top_m - column.base_m < HIGH_THIN_DEPTH_M && base_agl >= HIGH_CLOUD_BASE_M {
                        thin_high += 1;
                    }
                    if column.top_m - column.base_m >= NIMBOSTRATUS_DEPTH_M && base_agl < MIDDLE_CLOUD_BASE_M {
                        deep_low += 1;
                    }
                }
            }
        }
        println!(
            "the sky's own sequence: {thin_high} high thin columns, {deep_low} deep low columns, fronts {fronts:?}"
        );
        assert!(
            thin_high > 0,
            "no column carried the guide's leading-edge cloud: the shield's own top is missing"
        );
        assert!(deep_low > 0, "no column carried a deep low deck");
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
