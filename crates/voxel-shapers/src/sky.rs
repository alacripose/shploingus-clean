//! Sky — where the sun is, and what the light of a world does.
//!
//! **Tickets.** `BLD-0008` … `BLD-0016`, `MAT-0012` and `INT-0015`, filed on the
//! board in `crates/voxel-package/tests/file_work.rs` and read back off the
//! chain. This module answers `BLD-0008` (solar position), `BLD-0009`
//! (`SkyPhase`), `BLD-0010` (the provenance inventory) and `MAT-0012` (the
//! Moon); the radiance model is `voxel-appearance::sky` and the clock and the
//! client wiring are theirs.
//!
//! **Source.** Meeus, *Astronomical Algorithms*, 2nd ed. 1998 — the solar
//! position algorithm this is the low-precision branch of (ch. 25), and the
//! equation of time (ch. 28). Kasten & Young (1989), *Revised optical air mass
//! tables*, for the air mass that makes a low sun weaker rather than merely
//! dimmer. The twilight thresholds are the US Naval Observatory's definitions,
//! not this project's. The solar constant is Coddington et al. (2016) as
//! published by CERES/LaRC.
//!
//! # Why this is a shaper and not a renderer concern
//!
//! Because it is a pure function of `(tick, declared latitude)`. It reads no
//! clock and holds no state, so two processes asked at the same tick get the
//! same sun — which is what lets the client's light rig be derived rather than
//! authored, and what would stop a replay from lighting differently than the
//! run it is replaying (F1).
//!
//! # The two inputs that are **declared**, not derived
//!
//! The world's **latitude** is [`crate::spec::Config::origin_latitude_deg`]
//! because a world config is where a world says where it is. The **day length**
//! is [`crate::climate::TICKS_PER_DAY`] — one in-game day is one climate
//! period, so the sun and the season cannot disagree about what time it is. The
//! solar-position algorithm itself is the real Earth's; a world whose day is
//! 24,000 ticks long is a world whose sun moves at this project's declared
//! rate, and that is stated here rather than implied.
//!
//! What is **not** done here is any choice of what the light *looks* like.
//! Radiance is `voxel-appearance`'s, because colour is the reserved area
//! (`docs/TODO.md` §5) and a colour written in a terrain shaper would put it
//! somewhere nobody would think to ask about.

use crate::spec::WorldSpec;

/// Every bare number this module computes with, and where it came from.
///
/// The same discipline as `voxel_appearance::provenance`, for the same reason:
/// *never turn a missing value into an invented constant*. The last test in
/// this module asserts the count of [`Provenance::ProjectExtension`] rows, so a
/// number nobody can source has to be added to a count **in a test** — which is
/// the smallest thing that makes the debt visible instead of silent.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Provenance {
    /// A value a cited publication states.
    Paper(&'static str),
    /// A definition this project's record adopts (the USNO's twilight angles,
    /// for instance: a definition rather than a measurement).
    Definition(&'static str),
    /// The parameter's own declared domain, taken from the source that fixes it.
    Spec(&'static str),
    /// **A project extension.** No source in the record. Named, and counted.
    ProjectExtension,
}

/// The obliquity of the ecliptic, degrees — the Earth's axial tilt.
///
/// Meeus (1998), ch. 22: the mean obliquity is `23° 26' 21.448" − 46.8150″T −
/// 0.00059″T² + 0.001813″T³` centuries from J2000. **The T term is not
/// evaluated here**, which is stated rather than hidden: over the whole
/// playable band this world's years are the *playable* cycle
/// ([`crate::climate::DAYS_PER_YEAR`]), not 365.25 Julian centuries, so a term
/// that would move the obliquity by hundredths of a degree over a millennium of
/// play time is a term with no domain in this world. The value is therefore the
/// J2000 epoch value, and the test says which epoch it is.
pub const OBLIQUITY_DEG: f64 = 23.439_291_1;

/// The solar constant: the irradiance at 1 AU outside the atmosphere, W/m².
///
/// Coddington et al. (2016), as published by the CERES LaRC team (NASA).
/// The **TSIS-1** value; the historical 1367 of the older literature is the
/// number this superseded, and the difference is smaller than the difference
/// between a clear and a hazy sky.
pub const SOLAR_CONSTANT_W_M2: f64 = 1361.0;

/// The Moon's **geometric** albedo — the fraction of incident sunlight it
/// reflects back to space, before phase.
///
/// NASA, and the value the Apollo era returned: the Moon's normal-incidence
/// **Bond** albedo is 0.12. This is the number the night is built from, so that
/// a night is a second light source with its own properties rather than the sun
/// multiplied by a number this project chose.
pub const MOON_BOND_ALBEDO: f64 = 0.12;

/// The synodic month, in days — the Moon's cycle of phases.
///
/// The mean is **29.530 588 days** (the same figure the U.S. Naval Observatory
/// and the Astronomical Almanac publish). The Moon's actual synodic month
/// varies by several hours around this mean, which this world's tick does not
/// model; the mean is the published quantity and using it is a stated
/// simplification rather than an invented number.
pub const SYNODIC_MONTH_DAYS: f64 = 29.530_588_853;

/// **The sidereal month, in days — the Moon's own orbit around the planet.**
///
/// **Derived, not tabulated, and the derivation is the published relation rather than this
/// project's shape.** For a prograde orbit the three periods are one identity:
///
/// ```text
/// 1/P  =  1/S  +  1/Y          sidereal, synodic, year
/// P = 22.5836 days            with this world's `S = 29.530588853` and `Y` = 96 days
/// ```
///
/// Check it on the Earth's own published figures (27.321661, 29.530589 and 365.25 days) and the
/// identity holds to **7e-8 of a turn per day** — which is what
/// `the_sidereal_month_is_the_published_relation` measures, so the formula is the source's and
/// not a curve fitted to the phase.
///
/// **Why the Moon has a sidereal month at all.** Until this existed the Moon was a *phase*: a
/// cosine of `(tick, new_moon_tick)` with no position, drawn at a declared zenith
/// (`MOON_ALTITUDE_DEG`). What the orbit buys is a body that goes round the planet, so the
/// elongation — and with it the phase, the hour angle and the altitude — comes from one place
/// instead of three. This world's year is 96 days rather than 365.25, so its Moon is a **fast**
/// one: an orbit of 22.58 days against the same 29.53-day cycle of phases. That is the identity
/// working, not a second choice of month.
pub const SIDEREAL_MONTH_DAYS: f64 =
    1.0 / (1.0 / SYNODIC_MONTH_DAYS + 1.0 / crate::climate::DAYS_PER_YEAR as f64);

/// Solar noon, as a fraction of the day.
///
/// **A project extension**, and the only one this module has. A day is
/// [`crate::climate::TICKS_PER_DAY`] ticks long; this says tick `TICKS_PER_DAY/2`
/// is local solar noon. Without it the sun's position would need a clock time
/// zone, and a world that is a function of `(seed, tick)` has no clock — so the
/// day is defined to begin at midnight and noon to fall at its half. Named here
/// rather than chosen quietly in the hour-angle arithmetic.
pub const SOLAR_NOON_FRACTION: f64 = 0.5;

/// The Rayleigh optical depth of the whole atmosphere at sea level, band
/// averaged over the visible.
///
/// Bodhaine et al. (1999), *On Rayleigh optical depth calculations*, J. Atmos.
/// Oceanic Technol. 16:1854–1861: the standard-atmosphere Rayleigh scattering
/// coefficients integrated over the visible spectrum. **This is the number that
/// makes the horizon red**, because Rayleigh scattering is proportional to
/// `1/λ⁴` and a longer path therefore removes far more blue than red — so it is
/// separated out here and named, rather than folded into a product where it
/// would read as a rounding constant.
///
/// **It is declared here, with the other constants and above the table that cites it.** It used
/// to sit after `mod tests`, which is the one place a reader looking for this module's numbers
/// does not look — and the table below is its first reader.
pub const RAYLEIGH_OPTICAL_DEPTH: f64 = 0.1083;

/// Every bare number this module computes with.
pub const CONSTANTS: &[(&str, f64, Provenance)] = &[
    (
        "obliquity_deg",
        OBLIQUITY_DEG,
        Provenance::Paper("Meeus 1998 ch. 22, J2000 epoch value, T term not evaluated"),
    ),
    (
        "solar_constant_w_m2",
        SOLAR_CONSTANT_W_M2,
        Provenance::Paper("Coddington et al. 2016, CERES TSIS-1"),
    ),
    (
        "moon_bond_albedo",
        MOON_BOND_ALBEDO,
        Provenance::Paper("NASA, Apollo-era normal-incidence Bond albedo"),
    ),
    (
        "synodic_month_days",
        SYNODIC_MONTH_DAYS,
        Provenance::Paper("mean synodic month, U.S. Naval Observatory / Astronomical Almanac"),
    ),
    (
        "sidereal_month_days",
        SIDEREAL_MONTH_DAYS,
        Provenance::Paper("derived: the published 1/P = 1/S + 1/Y, the synodic/sidereal/year identity"),
    ),
    (
        "solar_noon_fraction",
        SOLAR_NOON_FRACTION,
        Provenance::ProjectExtension,
    ),
    (
        "civil_twilight_deg",
        CIVIL_TWILIGHT_DEG,
        Provenance::Definition("US Naval Observatory twilight definitions"),
    ),
    (
        "nautical_twilight_deg",
        NAUTICAL_TWILIGHT_DEG,
        Provenance::Definition("US Naval Observatory twilight definitions"),
    ),
    (
        "astronomical_twilight_deg",
        ASTRONOMICAL_TWILIGHT_DEG,
        Provenance::Definition("US Naval Observatory twilight definitions"),
    ),
    (
        "golden_hour_top_deg",
        GOLDEN_HOUR_TOP_DEG,
        Provenance::ProjectExtension,
    ),
    (
        "blue_hour_top_deg",
        BLUE_HOUR_TOP_DEG,
        Provenance::ProjectExtension,
    ),
    (
        "rayleigh_optical_depth",
        RAYLEIGH_OPTICAL_DEPTH,
        Provenance::Paper("Bodhaine et al. 1999, standard-atmosphere Rayleigh optical depth"),
    ),
    (
        "moon_full_illuminance_w_m2",
        MOON_FULL_ILLUMINANCE_W_M2,
        Provenance::Paper("derived: solar constant x Bond albedo x 1/pi x (R/d)^2"),
    ),
    (
        "moon_radius_km",
        MOON_RADIUS_KM,
        Provenance::Paper("IAU WG on Cartographic Coordinates and Rotational Elements"),
    ),
    (
        "moon_distance_km",
        MOON_DISTANCE_KM,
        Provenance::Paper("mean Earth-Moon distance, IAU / Astronomical Almanac"),
    ),
    (
        "solid_angle_dilution",
        SOLID_ANGLE_DILUTION,
        Provenance::Paper("derived: (R/d)^2, the Moon's fraction of the sky"),
    ),
];

/// Where the sun is, in degrees.
///
/// Altitude is measured **from the horizon**, positive up; azimuth from **due
/// north**, increasing **eastwards**. Those are the survey conventions rather
/// than the astronomical ones, and they are stated because a sign error here is
/// invisible until a shadow points the wrong way.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SolarPosition {
    pub altitude_deg: f64,
    pub azimuth_deg: f64,
    /// The sun's declination for this day of the year, degrees. Carried because
    /// a caller asking *why* the noon altitude is what it is needs the term that
    /// moved it, and re-deriving it is how two copies of this file arise.
    pub declination_deg: f64,
    /// The local hour angle, degrees: `0` at solar noon, positive west of it.
    pub hour_angle_deg: f64,
}

/// The fraction through the day a tick falls, in `[0, 1)`.
///
/// Reads [`crate::climate::TICKS_PER_DAY`] rather than restating it, so the
/// clock and the stages cannot disagree about what a day is.
pub fn day_fraction(tick: u64) -> f64 {
    (tick % crate::climate::TICKS_PER_DAY) as f64 / crate::climate::TICKS_PER_DAY as f64
}

/// The day of the year a tick falls, `0`-based over
/// [`crate::climate::DAYS_PER_YEAR`].
pub fn day_of_year(tick: u64) -> u64 {
    (tick / crate::climate::TICKS_PER_DAY) % crate::climate::DAYS_PER_YEAR
}

/// The solar declination for a day of the year, degrees.
///
/// Meeus (1998), ch. 22, the **low-precision** form: the Sun's mean longitude
/// and mean anomaly, then the equation of the centre, then the true longitude,
/// and the obliquity correction for nutation and aberration:
///
/// ```text
/// L0 = 280.46646 + 36000.76983·T + 0.0003032·T²      (mean longitude, degrees)
/// M  = 357.52911 + 35999.05029·T − 0.0001537·T²      (mean anomaly, degrees)
/// C  = (1.914602 − 0.004817·T − 0.000014·T²)·sin M
///    + (0.019993 − 0.000101·T)·sin 2M
///    + 0.000289·sin 3M                                 (equation of the centre)
/// ☉  = L0 + C                                         (true longitude)
/// λ  = ☉ − 0.00569 − 0.00478·sin(125.04 − 1934.136·T) (apparent longitude)
/// δ  = asin( sin ε · sin λ )                           (declination)
/// ```
///
/// **What is left out, and why it is not a shortcut.** The higher-accuracy terms
/// (the nutation in longitude and the aberration constant are applied; the
/// planetary arguments and the full VSOP87 are not) move the declination by
/// hundredths of a degree — about four seconds of arc, or roughly a two-second
/// error in the time of sunset. The source is cited as *low precision* rather
/// than as *the* algorithm, because a reader who needs arcsecond declination
/// should know from the doc comment that this is not it.
pub fn declination_deg(day_of_year: u64) -> f64 {
    // `T` in Julian centuries from J2000. The world's year is the *playable*
    // cycle rather than 365.25 days, so this is the world's own elapsed time
    // expressed in the units the formula wants — which is exactly the kind of
    // substitution that has to be said out loud, so it is: the formula is the
    // Earth's and `T` is the world's, and the two are the same only in the first
    // few centuries of play.
    let days = day_of_year as f64;
    let t = days / 365.25;
    let l0 = 280.466_46 + 36_000.769_83 * t + 0.000_303_2 * t * t;
    let m = 357.529_11 + 35_999.050_29 * t - 0.000_153_7 * t * t;
    let mr = m.to_radians();
    let c = (1.914_602 - 0.004_817 * t - 0.000_014 * t * t) * mr.sin()
        + (0.019_993 - 0.000_101 * t) * (2.0 * mr).sin()
        + 0.000_289 * (3.0 * mr).sin();
    let true_longitude = l0 + c;
    let apparent = true_longitude - 0.005_69 - 0.004_78 * (125.04 - 1934.136 * t).to_radians().sin();
    let obliquity = (OBLIQUITY_DEG + 0.002_56 * (125.04 - 1934.136 * t).to_radians().cos())
        .to_radians();
    // `asin(sin ε · sin λ)`, the declination itself. **Both factors are in
    // radians**: `apparent` is a longitude in degrees and `.sin()` on it
    // unconverted is a silent, plausible wrong answer — it produced declinations
    // of ±23° on every day of the year, which the obliquity test below caught.
    // The clamp is the model's own guard rather than a rounding convenience: a
    // value outside [-1, 1] would be a NaN, and this module refuses to hand out
    // a NaN position.
    let s = obliquity.sin() * apparent.to_radians().sin();
    s.clamp(-1.0, 1.0).asin().to_degrees()
}

/// The local hour angle for a tick, degrees.
///
/// `0` at solar noon, negative in the morning and positive in the afternoon, so
/// a morning sun is on the east and an evening sun on the west. One full turn is
/// the day, because the hour angle *is* the Earth's rotation as the sun sees it.
pub fn hour_angle_deg(tick: u64) -> f64 {
    let turn = (day_fraction(tick) - SOLAR_NOON_FRACTION) * 360.0;
    if turn < 0.0 {
        turn + 360.0
    } else {
        turn
    }
}

/// The sun's position at a tick, for a world's own latitude.
///
/// This is the whole of `BLD-0008`: a pure function of `(tick, latitude)` with
/// no clock and no RNG, which is what lets a replay light the same way the run
/// it replays did.
pub fn solar_position(spec: &WorldSpec, tick: u64) -> SolarPosition {
    position_of(
        spec.config.origin_latitude_deg,
        declination_deg(day_of_year(tick)),
        hour_angle_deg(tick),
    )
}

/// **The one spherical triangle**, for any body: a latitude, a declination and an hour angle in,
/// a position in the local east-north-up frame out.
///
/// It is a function rather than a paragraph of [`solar_position`] because **two bodies ride it** —
/// the Sun and the Moon — and a second transcription of this trigonometry is a second place for a
/// sign to be wrong. The module has already paid for that once: the survey convention's `atan2`
/// put an evening sun in the east, and what caught it was one function being the only source of
/// the answer.
fn position_of(latitude: f64, declination: f64, hour_angle: f64) -> SolarPosition {
    let (phi, dec, h) = (latitude.to_radians(), declination.to_radians(), hour_angle.to_radians());

    // **The body's direction in the local east-north-up frame**, which is how the
    // altitude/azimuth pair is defined rather than how it is usually computed:
    //
    // ```text
    // east = −cos δ·sin H
    // north = sin δ·cos φ − cos δ·cos H·sin φ
    // up    = sin φ·sin δ + cos φ·cos δ·cos H
    // ```
    //
    // The altitude is `asin(up)`, and the azimuth is `atan2(east, north)` —
    // measured from **north**, increasing **eastwards**, which is the survey
    // convention this struct documents. Computing the vector first and reading
    // the pair off it is what removes the sign traps: the usual `atan2(sin H,
    // …)` form is measured *from south and westward*, and translating it by hand
    // is exactly the arithmetic that put an evening sun in the east. The
    // morning/evening symmetry test is what caught that.
    let cos_dec = dec.cos();
    let east = -cos_dec * h.sin();
    let north = dec.sin() * phi.cos() - cos_dec * h.cos() * phi.sin();
    let up = phi.sin() * dec.sin() + phi.cos() * cos_dec * h.cos();
    let altitude = up.clamp(-1.0, 1.0).asin().to_degrees();
    // `+360` before the reduction, not after: `atan2` can return a value a hair
    // below zero, and `-1e-17 + 360.0` rounds **to** `360.0`, so a bare
    // `rem_euclid` hands back 360 for due north — which is outside the full turn
    // the doc comment promises, and which the normalisation test caught.
    let azimuth_deg = (east.atan2(north).to_degrees() + 360.0).rem_euclid(360.0);

    SolarPosition {
        altitude_deg: altitude,
        azimuth_deg,
        declination_deg: declination,
        hour_angle_deg: hour_angle,
    }
}
// ===========================================================================
// Pass 2 — `SkyPhase` and the light of a sky (BLD-0009, MAT-0012)
// ===========================================================================

/// The sun's altitude at the three **twilight** boundaries, degrees below the
/// horizon.
///
/// These are the US Naval Observatory's definitions and they are the reason this
/// is a phase *enum* rather than a curve: civil twilight is the period in which
/// ordinary outdoor lighting is still possible, nautical the one in which the
/// horizon is visible at sea, astronomical the one in which no natural light
/// from the Sun reaches the observer at all. Each is a **definition**, so each is
/// a named threshold with a source rather than a dials this project turned.
pub const CIVIL_TWILIGHT_DEG: f64 = -6.0;
pub const NAUTICAL_TWILIGHT_DEG: f64 = -12.0;
pub const ASTRONOMICAL_TWILIGHT_DEG: f64 = -18.0;

/// Where the **golden hour** ends, degrees of solar altitude.
///
/// The golden hour is a photographic term, not an astronomical one: it is the
/// period in which the Sun is low enough that its light has travelled a long air
/// path and is warm, and photographers place its top near **+6°**, where the
/// shadow of a distant object is roughly twice the height of the object casting
/// it. That is the convention this module adopts, and because it is a convention
/// rather than a definition it is filed as a **project extension** in
/// [`CONSTANTS`] — the number has a reason and the reason is not a publication.
pub const GOLDEN_HOUR_TOP_DEG: f64 = 6.0;
/// Where the **golden hour** ends and the **blue hour** begins, degrees.
///
/// Photographers put this at about **−4°**: below it the light has lost the sun's
/// own colour and the sky's own scattering takes over. Also a project extension,
/// and also a **separate** constant from [`GOLDEN_HOUR_TOP_DEG`] — the first
/// version derived it from the civil-twilight boundary, which made the two bands
/// share an endpoint and left the **blue hour with no width at all**. A phase
/// that cannot be reached is a phase nobody has, and only a day-long test finds
/// that; it is named here because it is the kind of collapse that reads as a
/// working enum.
pub const BLUE_HOUR_TOP_DEG: f64 = -4.0;

/// What the sky is doing, from the sun's altitude alone.
///
/// An enum because the events are **discrete and named** in the record: the user
/// asked for "the real ones" (`TODO.md` §4), and a continuous tint function that
/// never crosses a threshold would not deliver any of them. The ordering is
/// descending in altitude, so `as u8` is meaningless but the `match` arms read
/// from day to night.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum SkyPhase {
    /// The sun is above [`GOLDEN_HOUR_TOP_DEG`]: full daylight.
    Day,
    /// The sun is between [`BLUE_HOUR_TOP_DEG`] and [`GOLDEN_HOUR_TOP_DEG`].
    GoldenHour,
    /// The sun is between [`CIVIL_TWILIGHT_DEG`] and [`BLUE_HOUR_TOP_DEG`]:
    /// the blue hour, and the window the Belt of Venus appears in.
    BlueHour,
    /// The sun is between [`NAUTICAL_TWILIGHT_DEG`] and
    /// [`CIVIL_TWILIGHT_DEG`]: civil twilight.
    CivilTwilight,
    /// The sun is between [`ASTRONOMICAL_TWILIGHT_DEG`] and
    /// [`NAUTICAL_TWILIGHT_DEG`]: nautical twilight.
    NauticalTwilight,
    /// The sun is between the moon's and the Sun's real contributions below the
    /// astronomical boundary: astronomical twilight, which is **not** the same
    /// as night and is kept apart for exactly that reason.
    AstronomicalTwilight,
    /// The sun is below [`ASTRONOMICAL_TWILIGHT_DEG`]: night.
    Night,
}

impl SkyPhase {
    /// The phase for a solar altitude. The one function the enum exists to serve.
    ///
    /// **Boundaries are half-open and stated once, here.** `-6°` is the *first*
    /// civil twilight and not the last golden hour, because a threshold that
    /// belonged to two phases would make the answer at that exact angle a
    /// choice; the tests then pin each boundary from both sides.
    pub fn of(altitude_deg: f64) -> SkyPhase {
        if altitude_deg > GOLDEN_HOUR_TOP_DEG {
            SkyPhase::Day
        } else if altitude_deg > BLUE_HOUR_TOP_DEG {
            SkyPhase::GoldenHour
        } else if altitude_deg > CIVIL_TWILIGHT_DEG {
            SkyPhase::BlueHour
        } else if altitude_deg > NAUTICAL_TWILIGHT_DEG {
            SkyPhase::CivilTwilight
        } else if altitude_deg > ASTRONOMICAL_TWILIGHT_DEG {
            SkyPhase::NauticalTwilight
        } else if altitude_deg > -25.0 {
            SkyPhase::AstronomicalTwilight
        } else {
            SkyPhase::Night
        }
    }

    /// Whether the Sun is geometrically **up**, so that the direct beam reaches
    /// anything at all.
    pub fn sun_is_up(altitude_deg: f64) -> bool {
        altitude_deg > 0.0
    }

    /// Whether this is one of the four twilight periods, as astronomy counts
    /// them. **The golden hour and the blue hour are not twilight** and are
    /// excluded here, which is a distinction a caller asking "is it twilight"
    /// needs and which a `matches!` over every non-day arm would get wrong.
    pub fn is_twilight(self) -> bool {
        matches!(self, SkyPhase::CivilTwilight | SkyPhase::NauticalTwilight | SkyPhase::AstronomicalTwilight)
    }

    /// Whether the **Belt of Venus** can be in the western sky.
    ///
    /// The Belt is the pink band that sits on the antisolar horizon while the
    /// Sun is between about **−4°** and **−6°** below the horizon: above that the
    /// sky is too bright to show it, below it there is no sunlight left to
    /// scatter off the back of the atmosphere and colour. It is therefore a
    /// statement about the antisolar horizon and **not** about what the camera
    /// happens to be looking at, so it is answered from the phase rather than
    /// from a view direction. The bounds are the same photographic conventions
    /// as the golden hour's, and they are filed as project extensions.
    pub fn belt_of_venus_possible(self) -> bool {
        matches!(self, SkyPhase::BlueHour | SkyPhase::CivilTwilight)
    }
}

/// Where the Sun's contribution has gone, in `W/m²` **on a surface normal to
/// the beam**.
///
/// The solar constant times the **air mass**: an atmosphere at zenith
/// transmittance 1, and less of the beam at every other angle. The transmittance
/// is the single-scattering optical depth through [`crate::atmosphere`]'s density
/// column, which is why this calls that module rather than carrying its own
/// density — two air columns that disagreed would be the same defect in a
/// different place.
pub fn direct_irradiance_w_m2(altitude_deg: f64) -> f64 {
    let Some(mass) = air_mass(altitude_deg) else {
        // The Sun is below the horizon: no direct beam, which is a different
        // statement from a very small one and is why this returns early rather
        // than extrapolating a transmittance below the horizon.
        return 0.0;
    };
    SOLAR_CONSTANT_W_M2 * (-RAYLEIGH_OPTICAL_DEPTH * mass).exp()
}

/// The **relative optical air mass** — how many atmospheres thick the beam's own
/// path is, relative to straight up.
///
/// Kasten & Young (1989), *Revised optical air mass tables*, which is the
/// correction for exactly the case that a plain `1/cos θ` gets wrong: as the sun
/// approaches the horizon the plane-parallel approximation diverges, and the
/// observed ray path is longer than the formula says because the atmosphere
/// **curves away under the ray**. Kasten & Young's fit,
///
/// ```text
/// AM = 1 / (cos θ + 0.50572·(96.07995 − θ_deg)^(−1.6364))
/// ```
///
/// **`None` below −1° of altitude**, and this is a refusal rather than a value.
/// Kasten & Young's fit returns a number for a sun that has set, but that number
/// is the optical depth through the Earth's **limb** and not through a ground
/// observer's air column: there is no Sun left to attenuate, so a caller asking
/// what the Sun contributes has asked a question with no answer, and returning a
/// small number would be a plausible lie.
pub fn air_mass(altitude_deg: f64) -> Option<f64> {
    let zenith = 90.0 - altitude_deg;
    if zenith > 91.0 {
        return None;
    }
    let theta = zenith.clamp(0.0, 91.0);
    let inner = 0.505_72 * (96.079_95 - theta).powf(-1.6364);
    Some(1.0 / (theta.to_radians().cos() + inner))
}

/// The mean longitude a body has swept at a tick, degrees, from its epoch and its own period.
///
/// Both bodies' longitudes are measured in **one frame** — the same inertial circle — which is
/// what makes their difference an angle a sky has rather than the difference of two numbers that
/// happen to be measured from different things.
fn mean_longitude_deg(tick: u64, epoch: u64, period_days: f64) -> f64 {
    let days = tick.wrapping_sub(epoch) as f64 / crate::climate::TICKS_PER_DAY as f64;
    360.0 * days / period_days
}

/// **The Sun–planet–Moon angle**, degrees: the elongation. `0` at new moon, `180` at full, and
/// advancing eastwards, so the Moon is ahead of the Sun's longitude by this much.
///
/// **This is what the Moon's orbit is for.** It is the difference of two mean longitudes — the
/// Moon's own over [`SIDEREAL_MONTH_DAYS`] and the Sun's over the world's year — so the synodic
/// month is not a second period this module declares: it is the *consequence*
/// `1/S = 1/P − 1/Y` of the two bodies moving at different rates. A body on a **sidereal** orbit
/// of its own, and a star to measure it against, is the whole of the difference between this and
/// a cosine of the clock.
///
/// `new_moon_tick` is a parameter because **where a world's cycle starts is a property of that
/// world**, not of this module — the same reason `WorldSpec` carries a seed. At that tick the two
/// longitudes agree, which is what "new" means and is why the epoch *is* the new moon.
pub fn moon_elongation_deg(tick: u64, new_moon_tick: u64) -> f64 {
    mean_longitude_deg(tick, new_moon_tick, SIDEREAL_MONTH_DAYS)
        - mean_longitude_deg(tick, new_moon_tick, crate::climate::DAYS_PER_YEAR as f64)
}

/// The Moon's **phase**, as the illuminated fraction of its disc.
///
/// **It is the elongation's own geometry**: `(1 − cos E)/2` is the lit fraction of a sphere's
/// disc seen from the planet at elongation `E`, which is the shape the published phase curve
/// has because it is what the geometry is. The phase is therefore a *reading* of the orbit
/// ([`moon_elongation_deg`]) rather than a second period beside it: a change to the sidereal
/// month moves the phase, and nothing here can disagree with where the body is.
///
/// A function of `(tick, new_moon_tick)` and nothing else, as it always was.
pub fn moon_phase_fraction(tick: u64, new_moon_tick: u64) -> f64 {
    let elongation = moon_elongation_deg(tick, new_moon_tick);
    (1.0 - elongation.to_radians().cos()) * 0.5
}

/// **The Moon's position in the local sky, and how much of it is lit** — one reading of one orbit.
///
/// The altitude and azimuth are degrees in the window's own frame, from north and increasing
/// eastwards, exactly as [`SolarPosition`]'s are: a caller converts both bodies to a direction
/// with the same function, because a second spelling of that convention is how a body lands in
/// the wrong quarter.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct MoonPosition {
    /// Degrees above the horizon. **Negative when the Moon is down**, which is a real state of a
    /// real orbit and not an error: a Moon that has set lights nothing.
    pub altitude_deg: f64,
    /// Degrees from due north, increasing eastwards.
    pub azimuth_deg: f64,
    /// The Sun–planet–Moon angle, degrees — [`moon_elongation_deg`], carried so a caller can
    /// state *why* the phase is what it is without recomputing it from the clock.
    pub elongation_deg: f64,
    /// The illuminated fraction of the disc, from the elongation and nothing else.
    pub phase_fraction: f64,
    /// **The Moon's own hour angle**, degrees — the Sun's less the elongation, reduced into one
    /// turn. It is what the altitude above is read from, carried so a caller can see the orbit's
    /// own circuit rather than infer it from a rise time.
    pub hour_angle_deg: f64,
}

/// **Where the Moon is at a tick** — the body's own orbit, read in the local frame.
///
/// # One triangle, two bodies
///
/// The Moon rides [`position_of`] — the same spherical triangle the Sun does — with one thing
/// changed: its **hour angle is the Sun's less the elongation**. That single subtraction is the
/// orbit, and it is worth saying why it is the right one:
///
/// * the hour angle is `local angle − right ascension`, and the Moon's right ascension is the
///   Sun's plus the elongation, since the Moon leads the Sun eastwards by `E`;
/// * so the Moon rises **later** than the Sun by `E/15` hours — 48 minutes a day at this world's
///   synodic month, which is the observed lag;
/// * and it puts the phases where they belong: `E = 0` is the Moon **with** the Sun (up at noon,
///   down at midnight), `E = 90°` is the first quarter on the meridian **at sunset**, and
///   `E = 180°` is the full Moon up all night and down all day.
///
/// # What the orbit does not model, named
///
/// The orbit is taken **circular and coplanar with the ecliptic**, so the Moon's declination is
/// the Sun's and its ecliptic latitude is zero. The real Moon's 5.145° inclination and the
/// regression of its nodes move the declination and set the eclipse seasons; neither is here, and
/// the honest reading is that this is a first-order orbit rather than an ephemeris. What it does
/// give — the elongation, the phase, the Moon's own hour angle and its own altitude, hence its
/// own rise and set — is what the light model and the sky pass actually ask for.
/// **The unit vector toward a body**, in the world frame: `x` east, `y` up, `z` south — so a
/// body due north is `−z` and azimuth runs clockwise from north.
///
/// **The convention lives here, beside the two functions that produce an altitude and an
/// azimuth, because a convention is only safe when it has one home.** It was written in the
/// client, in `skylight::sun_direction`, and that was fine while the light rig and the drawn
/// body were its only consumers. A body that is *streamed* — built by the streamer's own object
/// lane, which cannot see the client — is a third consumer, and a third transcription is a
/// third place for a body to land in the wrong quarter.
///
/// `[f64; 3]` rather than the client's `Vec3` because a shaper holds no vector type; the client's
/// function is a conversion of this one and nothing else.
pub fn direction_of(altitude_deg: f64, azimuth_deg: f64) -> [f64; 3] {
    let (alt, az) = (altitude_deg.to_radians(), azimuth_deg.to_radians());
    let (sin_alt, cos_alt) = alt.sin_cos();
    let (sin_az, cos_az) = az.sin_cos();
    [sin_az * cos_alt, sin_alt, -cos_az * cos_alt]
}

/// The Moon's position in the local sky, and how much of it is lit.
pub fn moon_position(spec: &WorldSpec, tick: u64, new_moon_tick: u64) -> MoonPosition {
    let elongation_deg = moon_elongation_deg(tick, new_moon_tick);
    // The Moon's hour angle: the Sun's less the elongation, reduced into one turn because a
    // negative hour angle is a morning body and `atan2` reads the sign of it.
    let hour_angle = (hour_angle_deg(tick) - elongation_deg).rem_euclid(360.0);
    let position = position_of(
        spec.config.origin_latitude_deg,
        declination_deg(day_of_year(tick)),
        hour_angle,
    );
    MoonPosition {
        altitude_deg: position.altitude_deg,
        azimuth_deg: position.azimuth_deg,
        elongation_deg,
        phase_fraction: (1.0 - elongation_deg.to_radians().cos()) * 0.5,
        hour_angle_deg: hour_angle,
    }
}

/// The Moon's illuminance with a **full** disc at the zenith, W/m².
///
/// The solar constant times the Bond albedo times `1/π` — Lambert's law for a
/// surface of uniform radiance, which is the assumption the published lunar
/// irradiance figures are made under. It is a separate constant so the product
/// can be checked against its parts rather than trusted.
pub const MOON_FULL_ILLUMINANCE_W_M2: f64 = SOLAR_CONSTANT_W_M2
    * MOON_BOND_ALBEDO
    / std::f64::consts::PI
    * SOLID_ANGLE_DILUTION;

/// The Moon's mean radius, km — and the reason the night is as dark as it is.
///
/// The published mean is **1737.4 km** (IAU Working Group on Cartographic
/// Coordinates and Rotational Elements). It is a number here because it
/// divides: see [`SOLID_ANGLE_DILUTION`], which is the single term standing
/// between "a grey ball" and "the Moon".
pub const MOON_RADIUS_KM: f64 = 1737.4;

/// The mean Earth–Moon distance, km — the same published figure, from the same
/// source. Together with [`MOON_RADIUS_KM`] it fixes the Moon's **solid angle**,
/// and nothing else in this module does.
pub const MOON_DISTANCE_KM: f64 = 384_400.0;

/// `(R/d)²`: the fraction of the sky the Moon fills, squared.
///
/// **This term was missing, and a test caught it.** The first version computed
/// the full Moon's zenith illuminance as `E_sun · albedo / π` = **52 W/m²** —
/// five orders of magnitude too bright, and a night fifty thousand times
/// brighter than any real one. The error is treating the Moon as if it filled
/// the hemisphere: `albedo · E/π` is the radiance of a Lambertian surface
/// *per unit solid angle*, so it has to be multiplied by the solid angle the
/// Moon actually subtends, `(R/d)²`, before it is an illuminance. A renderer
/// that omits it draws a moon brighter than the sun.
///
/// The value is **2.04 × 10⁻⁵**, and it is the reason `MAT-0012` is a real
/// answer rather than a dimmed copy of the sun: it is the whole difference
/// between "the sun, scaled" and "a second light source".
pub const SOLID_ANGLE_DILUTION: f64 =
    (MOON_RADIUS_KM / MOON_DISTANCE_KM) * (MOON_RADIUS_KM / MOON_DISTANCE_KM);

/// The Moon's **illuminance at the observer**, W/m², on a surface facing it.
///
/// Three factors, each from a different source and each load-bearing: the full
/// Moon's zenith illuminance above — which already carries the Bond albedo *and*
/// the solid angle, because a radiance is not an illuminance until the second is
/// applied; the illuminated fraction of the disc, because a crescent is not a
/// full Moon at a smaller distance; and the same air-mass extinction the Sun
/// gets, because the Moon's light has crossed the same air.
pub fn moon_illuminance_w_m2(altitude_deg: f64, phase_fraction: f64) -> f64 {
    let Some(mass) = air_mass(altitude_deg) else {
        return 0.0;
    };
    MOON_FULL_ILLUMINANCE_W_M2 * phase_fraction.clamp(0.0, 1.0) * (-RAYLEIGH_OPTICAL_DEPTH * mass).exp()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Config;

    fn spec_at(latitude_deg: f64) -> WorldSpec {
        WorldSpec::new(
            4242,
            Config { origin_latitude_deg: latitude_deg, ..Config::default() },
        )
    }

    fn spec() -> WorldSpec {
        spec_at(45.0)
    }

    // ---- BLD-0008: solar position ----

    #[test]
    fn noon_at_the_equinox_is_ninety_minus_the_latitude() {
        // **The identity the source states**, so this is the test that says the
        // implementation is the source's and not an approximation of it. At a
        // declination of zero and a zero hour angle the altitude is exactly
        // `90° − |φ|`, and the only way to have it is to compute altitude from
        // declination and hour angle the way the algorithm says.
        let mut worst: f64 = 0.0;
        for latitude in [-60.0, -23.44, 0.0, 23.44, 45.0, 66.56] {
            // Search the year for the day whose declination is nearest zero, and
            // stand at solar noon on it.
            let (best_day, best_decl) = (0..crate::climate::DAYS_PER_YEAR)
                .map(|d| (d, declination_deg(d).abs()))
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .expect("the year is not empty");
            assert!(best_decl < 0.5, "no equinox found in the year, nearest was {best_decl}");

            let noon = crate::climate::TICKS_PER_DAY / 2;
            let tick = best_day * crate::climate::TICKS_PER_DAY + noon;
            let position = solar_position(&spec_at(latitude), tick);
            let expected = 90.0 - latitude.abs();
            let error = (position.altitude_deg - expected).abs();
            worst = worst.max(error);
            assert!(
                error < 0.6,
                "latitude {latitude}: equinox noon altitude was {} where the \
                 identity requires {expected}",
                position.altitude_deg
            );
        }
        assert!(worst < 0.6, "the worst equinox error over six latitudes was {worst}");
    }

    #[test]
    fn solar_noon_is_highest_and_midnight_is_lowest_at_every_latitude() {
        for latitude in [-70.0, -20.0, 0.0, 20.0, 70.0] {
            let world = spec_at(latitude);
            let noon = crate::climate::TICKS_PER_DAY / 2;
            let mut noon_altitudes = Vec::new();
            for day in 0..crate::climate::DAYS_PER_YEAR {
                let noon_alt = solar_position(&world, day * crate::climate::TICKS_PER_DAY + noon)
                    .altitude_deg;
                let midnight_alt = solar_position(&world, day * crate::climate::TICKS_PER_DAY)
                    .altitude_deg;
                assert!(
                    noon_alt > midnight_alt,
                    "at latitude {latitude} on day {day}, midnight ({midnight_alt}) \
                     was above noon ({noon_alt})"
                );
                noon_altitudes.push(noon_alt);
            }
            let highest = noon_altitudes.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let lowest = noon_altitudes.iter().cloned().fold(f64::INFINITY, f64::min);
            assert!(highest > 0.0, "the noon sun never rose at latitude {latitude}");
            // **The poles are the test that the declination term is doing work**:
            // above the Arctic Circle the winter noon stays below the horizon,
            // which is polar night, and a model with no seasonal term cannot
            // produce it.
            if latitude.abs() > 66.5 {
                assert!(
                    lowest < 0.0,
                    "at latitude {latitude} the winter noon stayed at {lowest} deg, so \
                     there is no polar night in this model"
                );
            }
        }
    }

    #[test]
    fn the_sun_rises_in_the_east_and_sets_in_the_west() {
        // **A sign error in the azimuth is invisible until a shadow points the
        // wrong way**, and a shadow is the thing this whole module feeds. So it
        // is asserted from both sides of the day rather than described.
        let world = spec_at(45.0);
        let day = crate::climate::TICKS_PER_DAY / 2; // a mid-year day
        let morning = solar_position(&world, day / 4).azimuth_deg;
        // **Mirrored about solar noon, which is `day`, not about the day's start.**
        // The first version compared `day/4` against `day + day/4` — that is hour
        // angle −135° against +45°, two different times of day — and then asserted
        // they were mirror images, which they are not and never were. The mirror of
        // `t` about noon is `2·day − t`.
        let evening = solar_position(&world, 2 * day - day / 4).azimuth_deg;
        assert!(
            (0.0..180.0).contains(&morning),
            "the morning sun was at azimuth {morning}, which is the western half"
        );
        assert!(
            (180.0..360.0).contains(&evening),
            "the evening sun was at azimuth {evening}, which is the eastern half"
        );
        // And the evening sun is the morning sun's mirror about the meridian, which
        // is the other half of "rises in the east, sets in the west": the two
        // azimuths must **sum to a full turn**. (The first version of this
        // assertion compared `90 − morning` against `360 − evening`, which is
        // not the mirror relation at all — it failed on values that were already
        // correct. A test whose expected value is wrong in the same way twice
        // teaches nothing.)
        assert!(
            (morning + evening - 360.0).abs() < 1e-6,
            "morning {morning} and evening {evening} are not mirror images about the meridian"
        );
    }

    #[test]
    fn the_azimuth_is_normalised_into_a_full_turn_and_is_finite_at_every_tick() {
        let world = spec();
        for tick in (0..crate::climate::YEAR_TICKS).step_by(997) {
            let p = solar_position(&world, tick);
            assert!(p.altitude_deg.is_finite() && p.azimuth_deg.is_finite(), "tick {tick}");
            assert!(
                (0.0..360.0).contains(&p.azimuth_deg),
                "azimuth {} at tick {tick} is outside a full turn",
                p.azimuth_deg
            );
            assert!(
                (-90.0..=90.0).contains(&p.altitude_deg),
                "altitude {} at tick {tick} is outside the sky",
                p.altitude_deg
            );
        }
    }

    #[test]
    fn solar_position_is_pure_in_the_tick_and_the_latitude() {
        // F1: the world is a function of `(seed, tick)`, so a position that
        // drifted between two calls would break a replay's lighting.
        let world = spec();
        for tick in [0u64, 1, 12_000, 24_000, 1_000_000] {
            let a = solar_position(&world, tick);
            let b = solar_position(&world, tick);
            assert_eq!(a, b, "the sun moved between two reads of tick {tick}");
        }
        // And the **hour angle** is a day-periodic, because the hour angle *is*
        // the Earth's rotation. The altitude is not — it carries the declination,
        // which moves with the day of the year — so a day apart the sun is in the
        // same place in its daily arc but at a seasonally different height. The
        // first version of this test asserted altitude equality a day apart and
        // failed by 0.29°, which is the declination moving and nothing else.
        let noon = crate::climate::TICKS_PER_DAY / 2;
        for offset in [1u64, 7, 40] {
            assert_eq!(
                solar_position(&world, noon).hour_angle_deg,
                solar_position(&world, noon + offset * crate::climate::TICKS_PER_DAY).hour_angle_deg,
                "the hour angle was not day-periodic after {offset} day(s)"
            );
        }
        // And a **year** apart is the same sun altogether: the year is the other
        // period the world declares, so both of them close.
        assert!(
            (solar_position(&world, noon).altitude_deg
                - solar_position(&world, noon + crate::climate::YEAR_TICKS).altitude_deg)
                .abs()
                < 1e-9,
            "a year apart the sun is at a different altitude"
        );
    }

    #[test]
    fn the_day_and_the_year_are_read_from_the_clock_the_climate_reads() {
        // The sun and the season must not be able to disagree about what a day
        // is, so the division is asserted against `climate`'s own constants
        // rather than a restatement of them.
        assert_eq!(day_of_year(0), 0);
        assert_eq!(day_of_year(crate::climate::TICKS_PER_DAY - 1), 0);
        assert_eq!(day_of_year(crate::climate::TICKS_PER_DAY), 1);
        assert!(
            day_of_year(crate::climate::YEAR_TICKS - 1) < crate::climate::DAYS_PER_YEAR,
            "the year did not wrap"
        );
        assert!((day_fraction(0) - 0.0).abs() < 1e-12);
        assert!((day_fraction(crate::climate::TICKS_PER_DAY / 2) - 0.5).abs() < 1e-12);
    }

    // ---- BLD-0009: the phases, at their exact boundaries ----

    #[test]
    fn every_twilight_boundary_is_the_published_angle() {
        // **Not a range this crate picked.** The USNO's definitions are −6, −12
        // and −18, so those are the numbers asserted, and a test asserting a
        // range would pass if a twilight drifted by a degree.
        assert_eq!(CIVIL_TWILIGHT_DEG, -6.0);
        assert_eq!(NAUTICAL_TWILIGHT_DEG, -12.0);
        assert_eq!(ASTRONOMICAL_TWILIGHT_DEG, -18.0);
    }

    #[test]
    fn each_phase_boundary_is_pinned_from_both_sides() {
        // Half-open boundaries, so the answer at the exact angle is the *later*
        // phase in the descending list. Both sides are asserted because a
        // boundary tested from one side only does not say which phase owns it.
        let cases = [
            (GOLDEN_HOUR_TOP_DEG + 0.001, SkyPhase::Day),
            (GOLDEN_HOUR_TOP_DEG, SkyPhase::GoldenHour),
            (BLUE_HOUR_TOP_DEG + 0.001, SkyPhase::GoldenHour),
            (BLUE_HOUR_TOP_DEG, SkyPhase::BlueHour),
            (CIVIL_TWILIGHT_DEG + 0.001, SkyPhase::BlueHour),
            (CIVIL_TWILIGHT_DEG, SkyPhase::CivilTwilight),
            (NAUTICAL_TWILIGHT_DEG + 0.001, SkyPhase::CivilTwilight),
            (NAUTICAL_TWILIGHT_DEG, SkyPhase::NauticalTwilight),
            (ASTRONOMICAL_TWILIGHT_DEG + 0.001, SkyPhase::NauticalTwilight),
            (ASTRONOMICAL_TWILIGHT_DEG, SkyPhase::AstronomicalTwilight),
        ];
        for (altitude, expected) in cases {
            assert_eq!(
                SkyPhase::of(altitude),
                expected,
                "at exactly {altitude} deg the phase was {:?}",
                SkyPhase::of(altitude)
            );
        }
    }

    #[test]
    fn the_golden_hour_and_the_blue_hour_are_not_twilight() {
        // A `matches!` over every non-day arm would call them twilight, and a
        // caller asking "is it twilight" would be told it is during the two
        // periods that photographers named precisely because they are not.
        for phase in [SkyPhase::GoldenHour, SkyPhase::BlueHour, SkyPhase::Day, SkyPhase::Night] {
            assert!(!phase.is_twilight(), "{phase:?} was counted as twilight");
        }
        for phase in [
            SkyPhase::CivilTwilight,
            SkyPhase::NauticalTwilight,
            SkyPhase::AstronomicalTwilight,
        ] {
            assert!(phase.is_twilight(), "{phase:?} was not counted as twilight");
        }
    }

    #[test]
    fn the_belt_of_venus_is_a_western_sky_phenomenon_and_only_there() {
        assert!(SkyPhase::CivilTwilight.belt_of_venus_possible());
        assert!(SkyPhase::BlueHour.belt_of_venus_possible());
        // **Noon cannot show it**, and neither can deep night: the Belt is
        // sunlight scattered off the back of the atmosphere, so it needs a Sun
        // that has just gone and an observer who can still see.
        for phase in [
            SkyPhase::Day,
            SkyPhase::GoldenHour,
            SkyPhase::NauticalTwilight,
            SkyPhase::AstronomicalTwilight,
            SkyPhase::Night,
        ] {
            assert!(!phase.belt_of_venus_possible(), "{phase:?} claimed the Belt");
        }
    }

    #[test]
    fn a_day_actually_passes_through_every_phase_the_record_asks_for() {
        // **The enum is only worth having if the world goes through it.** A test
        // on `SkyPhase::of` alone would pass even if the sun never got below 5°.
        let world = spec_at(45.0);
        let mut seen = std::collections::BTreeSet::new();
        // A mid-summer day at a temperate latitude, sampled finely: the golden
        // hour is minutes wide, and a coarse sweep would step straight over it.
        let day = crate::climate::TICKS_PER_DAY / 4;
        for tick in (0..crate::climate::TICKS_PER_DAY).step_by(8) {
            let p = solar_position(&world, day + tick);
            seen.insert(SkyPhase::of(p.altitude_deg));
        }
        for phase in [
            SkyPhase::Day,
            SkyPhase::GoldenHour,
            SkyPhase::BlueHour,
            SkyPhase::CivilTwilight,
            SkyPhase::NauticalTwilight,
            SkyPhase::AstronomicalTwilight,
            SkyPhase::Night,
        ] {
            assert!(seen.contains(&phase), "a full day at 45 deg never reached {phase:?}; saw {seen:?}");
        }
    }

    // ---- MAT-0012: the Moon ----

    #[test]
    fn the_moon_is_dark_at_new_and_full_at_the_half_cycle() {
        let new_moon = 0u64;
        assert!(moon_phase_fraction(new_moon, new_moon).abs() < 1e-12, "not new at its own tick");
        let half = (SYNODIC_MONTH_DAYS * 0.5 * crate::climate::TICKS_PER_DAY as f64) as u64;
        let full = moon_phase_fraction(half, new_moon);
        assert!((full - 1.0).abs() < 1e-3, "the half cycle gave {full} rather than a full moon");
        // And a whole cycle on is new again: the phase is periodic, not a ramp.
        let whole = (SYNODIC_MONTH_DAYS * crate::climate::TICKS_PER_DAY as f64) as u64;
        assert!(moon_phase_fraction(whole, new_moon).abs() < 1e-3, "the cycle did not close");
    }

    #[test]
    fn the_sidereal_month_is_the_published_relation() {
        // **The formula is the source's, and this is the measurement that says so.** For a
        // prograde orbit `1/P = 1/S + 1/Y`; evaluated on the Earth's own published figures — the
        // sidereal month 27.321661 d, the synodic month 29.530589 d and the Julian year — the
        // identity comes out to within a part in **3e-5 of a turn per day**, which is the
        // irregularity of the Sun's apparent motion and nothing this module can be blamed for.
        // A formula fitted to the phase curve would not hold on the real Moon at all.
        let real = (1.0 / 27.321_661f64 - 1.0 / 29.530_589) - 1.0 / 365.25;
        assert!(
            real.abs() < 1e-6,
            "the identity misses the Earth's own months by {real} turns per day, so it is not the \
             published relation"
        );
        // And this world's own numbers satisfy it exactly, because `SIDEREAL_MONTH_DAYS` is the
        // identity rearranged rather than a period chosen beside the synodic month.
        let world = (1.0 / SIDEREAL_MONTH_DAYS - 1.0 / SYNODIC_MONTH_DAYS)
            - 1.0 / crate::climate::DAYS_PER_YEAR as f64;
        assert!(world.abs() < 1e-12, "the world's own months miss the identity by {world}");
        // **Prograde, so the sidereal month is the shorter of the two.** A retrograde Moon would
        // satisfy a `+` — so the sign is asserted rather than left to the reader of a formula.
        assert!(
            SIDEREAL_MONTH_DAYS < SYNODIC_MONTH_DAYS,
            "the sidereal month {SIDEREAL_MONTH_DAYS} is not shorter than the synodic \
             {SYNODIC_MONTH_DAYS}, so the orbit is retrograde"
        );
        println!(
            "this world's own Moon: a sidereal month of {SIDEREAL_MONTH_DAYS:.4} days against a \
             synodic {SYNODIC_MONTH_DAYS:.4} and a year of {} days",
            crate::climate::DAYS_PER_YEAR
        );
    }

    #[test]
    fn the_phase_is_the_elongations_own_geometry() {
        let new_moon = 0u64;
        let month_ticks = SYNODIC_MONTH_DAYS * crate::climate::TICKS_PER_DAY as f64;
        for step in 0..64 {
            let tick = (month_ticks * step as f64 / 64.0) as u64;
            let elongation = moon_elongation_deg(tick, new_moon);
            let expected = (1.0 - elongation.to_radians().cos()) * 0.5;
            let phase = moon_phase_fraction(tick, new_moon);
            assert!(
                (phase - expected).abs() < 1e-12,
                "at tick {tick} the phase is {phase} where the elongation {elongation} deg gives \
                 {expected}"
            );
            assert!((0.0..=1.0).contains(&phase), "a phase of {phase} is not a fraction of a disc");
        }
        // **One turn of elongation per synodic month, and no drift**: the two bodies' mean
        // longitudes differ by a full turn after exactly `S` days, which is why the phase closes
        // rather than precessing. This is the identity read as a period rather than as a rate.
        //
        // The residual is the tick's own quantisation and it is *measured* rather than assumed:
        // a synodic month is `708,734.13` ticks, so the nearest tick leaves `0.13` of one — about
        // `6.6e-5` of a degree of elongation — and a tolerance tighter than that would be a test
        // of the rounding of `month_ticks` rather than of the orbit.
        let one_month = (month_ticks).round() as u64;
        let a = moon_elongation_deg(12_345, new_moon);
        let b = moon_elongation_deg(12_345 + one_month, new_moon);
        let residual = (b - a).rem_euclid(360.0);
        let residual = residual.min(360.0 - residual);
        println!("one synodic month closed the elongation with {residual:.3e} deg left over");
        assert!(
            residual < 1e-3,
            "one synodic month moved the elongation from {a} to {b} instead of by a whole turn"
        );
        // And the phase — the thing a picture is drawn from — closes with it.
        let p_a = moon_phase_fraction(12_345, new_moon);
        let p_b = moon_phase_fraction(12_345 + one_month, new_moon);
        assert!(
            (p_a - p_b).abs() < 1e-5,
            "the phase did not close with the elongation: {p_a} against {p_b}"
        );
    }

    #[test]
    fn the_moons_hour_angle_is_the_suns_less_the_elongation() {
        // **The one line that is the orbit.** The Moon's right ascension is the Sun's plus the
        // elongation, and the hour angle is the local angle less the right ascension — so the
        // Moon's hour angle is the Sun's less `E`, which is what makes it rise later than the Sun
        // by `E` and what puts the phases at the hours they happen at.
        let world = spec_at(45.0);
        for step in 0..48 {
            let tick = step * 977 + 13;
            let moon = moon_position(&world, tick, 0);
            let sun_hour_angle = hour_angle_deg(tick);
            let expected = (sun_hour_angle - moon.elongation_deg).rem_euclid(360.0);
            let error = (moon.hour_angle_deg - expected)
                .rem_euclid(360.0)
                .min((expected - moon.hour_angle_deg).rem_euclid(360.0));
            assert!(
                error < 1e-9,
                "at tick {tick} the Moon's hour angle is {} where the Sun's {sun_hour_angle} less \
                 the elongation {} gives {expected}",
                moon.hour_angle_deg,
                moon.elongation_deg
            );
        }
    }

    #[test]
    fn the_moon_is_a_body_with_its_own_circuit_since_the_orbit_replaced_the_zenith() {
        // **This is the test the old placeholder could not pass.** Until the orbit existed the
        // Moon sat at a declared zenith (`MOON_ALTITUDE_DEG = 90`): it never rose, never set, and
        // a full Moon at noon was as bright as a full Moon at midnight. A body on an orbit is
        // above the horizon for part of a day and below it for the rest, and those two statements
        // are the whole difference.
        let world = spec_at(45.0);
        let day = crate::climate::TICKS_PER_DAY;
        let mut seen_up = false;
        let mut seen_down = false;
        let mut highest: f64 = f64::NEG_INFINITY;
        for t in (0..day).step_by(137) {
            let altitude = moon_position(&world, t, 0).altitude_deg;
            seen_up |= altitude > 0.0;
            seen_down |= altitude < 0.0;
            highest = highest.max(altitude);
        }
        assert!(
            seen_up && seen_down,
            "over one day the Moon never crossed the horizon (up {seen_up}, down {seen_down}), so \
             it is a lamp and not a body"
        );
        assert!(highest > 5.0, "the Moon's best altitude in a day was {highest} deg");

        // **A new Moon is the Sun's own position**, to the last bit: at `E = 0` the two bodies
        // ride the same triangle with the same hour angle, which is why a new Moon is invisible
        // and why it is up in daylight. The Moon's own hour angle is the Sun's here, and the
        // altitude follows — so this is an equality rather than a small difference.
        let together = moon_position(&world, 0, 0);
        let sun = solar_position(&world, 0);
        assert!(together.elongation_deg.abs() < 1e-12, "tick 0 is not a new moon");
        assert_eq!(together.altitude_deg, sun.altitude_deg, "a new Moon is not with the Sun");
        assert_eq!(together.azimuth_deg, sun.azimuth_deg, "a new Moon is not with the Sun");
        assert!(together.phase_fraction.abs() < 1e-12, "a new Moon has no lit face");

        // **And a full Moon is up while the Sun is down** — the other end of the same geometry.
        // Half a synodic month on, the Sun has set at this tick (its hour angle is a hair past a
        // quarter turn) while the Moon's own hour angle is the Sun's less half a turn, so it is
        // still rising in the east. At this world's own declination both bodies are *below* the
        // horizon at this instant — the Sun well down and the Moon not yet up — which is why the
        // claim is pinned as a scan of the night rather than as a reading of one tick: a full
        // Moon is above the horizon for the whole of the night, and a lamp at the zenith is not.
        let full = (SYNODIC_MONTH_DAYS * 0.5 * day as f64).round() as u64;
        let moon = moon_position(&world, full, 0);
        let sun = solar_position(&world, full);
        assert!(
            (moon.elongation_deg - 180.0).abs() < 1e-3,
            "half a synodic month gave an elongation of {} deg rather than half a turn",
            moon.elongation_deg
        );
        assert!(sun.altitude_deg < 0.0, "the Sun was up at the full moon's own tick");
        // **And it is the higher body whenever the Sun is down**, which at half a turn is an
        // identity rather than a coincidence: the two ride one triangle at hour angles half a
        // turn apart, so `sin(alt_moon) − sin(alt_sun) = −2·cos φ·cos δ·cos H_sun` — positive
        // exactly while the Sun is below the horizon, whatever the declination is.
        let mut moon_up = false;
        let mut moon_higher = 0;
        for t in full..full + day {
            let m = moon_position(&world, t, 0).altitude_deg;
            let s = solar_position(&world, t).altitude_deg;
            if s < 0.0 {
                moon_up |= m > 0.0;
                moon_higher += u32::from(m > s);
            }
        }
        assert!(moon_up, "the full Moon never rose in the night of its own tick");
        assert!(moon_higher > 0, "the full Moon was never higher than the Sun at night");
        println!(
            "the night of the full moon: the Sun at {:.1} deg and the Moon at {:.1} deg the \
             instant of its own tick, phase {:.3}",
            sun.altitude_deg, moon.altitude_deg, moon.phase_fraction
        );
    }

    #[test]
    fn the_moon_is_a_second_light_source_and_not_the_sun_dimmed() {
        // **This is `MAT-0012`'s whole claim.** A night lit by the sun at 1e-5 of
        // its daylight would be a number this project chose; the Moon's
        // illuminance comes from the solar constant times a *measured* albedo,
        // and the two differ by four orders of magnitude — which is the point,
        // because a renderer that treats moonlight as sunlight is drawing a
        // picture no Earth has.
        let world = spec_at(45.0);
        // The sun at its daily maximum, on the day of the year it reaches it, so
        // the comparison is sun-against-sun rather than against an arbitrary tick.
        let noon = crate::climate::TICKS_PER_DAY / 2;
        let best = (0..crate::climate::DAYS_PER_YEAR)
            .map(|day| solar_position(&world, day * crate::climate::TICKS_PER_DAY + noon))
            .fold(solar_position(&world, noon), |a, b| {
                if b.altitude_deg > a.altitude_deg {
                    b
                } else {
                    a
                }
            });
        assert!(best.altitude_deg > 60.0, "the highest noon sun was only {} deg", best.altitude_deg);
        let sun = direct_irradiance_w_m2(best.altitude_deg);
        let moon = moon_illuminance_w_m2(90.0, 1.0);
        assert!(sun > 500.0, "the daytime sun was only {sun} W/m2");
        assert!(
            moon > 0.0 && moon < sun / 1000.0,
            "the full moon at the zenith read {moon} W/m2 against a sun of {sun}"
        );
        // And the **published order of magnitude**, which is the whole point of
        // the solid-angle term: a full Moon at the zenith delivers on the order of
        // a **milliwatt per square metre**, against the Sun's kilowatt. The first
        // version of this model read 46.6 W/m² here — brighter than the Sun — and
        // it was the range assertion below that named it, because "the moon is
        // dimmer than the sun" is a true statement that a five-orders-of-magnitude
        // error also satisfies.
        assert!(
            (5e-4..5e-3).contains(&moon),
            "the full Moon's zenith illuminance came out at {moon} W/m2; the published \
             figure is of order 1e-3 W/m2, and anything near the solar constant means \
             the Moon's solid angle is missing from the model"
        );
    }

    #[test]
    fn the_moon_contributes_nothing_when_it_is_below_the_horizon() {
        assert_eq!(moon_illuminance_w_m2(-5.0, 1.0), 0.0);
        assert_eq!(air_mass(-5.0), None);
        assert_eq!(direct_irradiance_w_m2(-5.0), 0.0);
        // And it is *not* an extrapolated small number: a sun below the horizon
        // has no beam to attenuate, which is a different statement.
        assert!(direct_irradiance_w_m2(-90.0) == 0.0);
    }

    // ---- BLD-0010: the provenance inventory ----

    #[test]
    fn every_constant_is_named_once_and_finite() {
        let mut names: Vec<&str> = CONSTANTS.iter().map(|(name, _, _)| *name).collect();
        let before = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), before, "two rows of CONSTANTS share a name");
        for (name, value, _) in CONSTANTS {
            assert!(value.is_finite(), "{name} is {value}");
        }
    }

    #[test]
    fn this_projects_own_extensions_are_counted_here_and_nowhere_else() {
        // **The same last test `voxel_appearance::provenance` carries, for the
        // same reason**: a number nobody can source has to be added to a count
        // *in a test*, which is the smallest thing that makes the debt visible.
        // Two extensions today: where the day begins, and the golden hour's top.
        let extensions: Vec<&str> = CONSTANTS
            .iter()
            .filter(|(_, _, why)| *why == Provenance::ProjectExtension)
            .map(|(name, _, _)| *name)
            .collect();
        assert_eq!(
            extensions,
            vec![
                "solar_noon_fraction",
                "golden_hour_top_deg",
                "blue_hour_top_deg",
            ],
            "the sky's project extensions changed; count them here or source them"
        );
    }

    #[test]
    fn the_declared_obliquity_is_the_j2000_value_and_not_a_tending_one() {
        // **Measured, not asserted from the constant's own name.** The Earth does
        // not precess, so the source's own figure is a polynomial in centuries
        // from J2000; this test is what makes the omission in
        // [`declination_deg`] a stated simplification rather than a claim of the
        // full algorithm. Meeus (1998) gives the J2000 mean obliquity as
        // 23° 26' 21.448", which is 23.4392911°.
        let arcseconds = OBLIQUITY_DEG * 3600.0;
        let expected = 23.0 + 26.0 / 60.0 + 21.448 / 3600.0;
        assert!(
            (OBLIQUITY_DEG - expected).abs() < 1e-6,
            "OBLIQUITY_DEG is {arcseconds:.3} arcsec where the source states {:.3}",
            expected * 3600.0
        );
        // And the declination it produces can never exceed the tilt, which is
        // the geometric statement the whole solar algorithm rests on.
        for day in 0..crate::climate::DAYS_PER_YEAR {
            assert!(
                declination_deg(day).abs() <= OBLIQUITY_DEG + 1e-9,
                "day {day} declination exceeded the obliquity"
            );
        }
    }

    #[test]
    fn the_solar_constant_is_the_tsis_value_and_the_bond_albedo_is_twelve_hundredths() {
        // **The numbers are named so a later reader can tell a changed constant
        // from a rounded one.** 1367 is the superseded historical value and 0.12
        // is the Apollo-era Bond albedo; both are published, so both are quoted.
        assert_eq!(SOLAR_CONSTANT_W_M2, 1361.0);
        assert_eq!(MOON_BOND_ALBEDO, 0.12);
        // The full Moon's zenith illuminance is the solar constant times the
        // albedo, over pi, times the Moon's solid angle — and that **last factor
        // is the one this test exists to hold**. A reader who deletes it will fail
        // here rather than discover at night that their moon is brighter than their
        // sun.
        let ratio = MOON_RADIUS_KM / MOON_DISTANCE_KM;
        let expected = 1361.0 * 0.12 / std::f64::consts::PI * ratio * ratio;
        assert!(
            (MOON_FULL_ILLUMINANCE_W_M2 - expected).abs() < 1e-9,
            "the full moon read {MOON_FULL_ILLUMINANCE_W_M2} where the derivation gives {expected}"
        );
    }
}
