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

/// **The clock's rate: world ticks per second.**
///
/// The one rate in the tree — `voxel_client::clock::TICK_HZ` is this number rather than a second
/// copy of it, so a stage ladder and a frame clock cannot tick at two speeds wearing one name.
pub const TICKS_PER_SECOND: u64 = 20;

/// Seconds in a real day. The world's day is a *scaled* one; this is the length it is scaled from.
pub const SECONDS_PER_DAY: u64 = 86_400;

/// **Ticks in one in-game day: a real day at the world's own scale.**
///
/// The tree is drawn at one eighth of life — [`crate::scale::REAL_METRES_PER_VOXEL`] is `8.0` —
/// and the clock is scaled with it: `86,400 s / 8 = 10,800 s` of **3 hours**, which at
/// [`TICKS_PER_SECOND`] is **216,000 ticks**.
///
/// It was `24_000`, a hand-rolled number that read as 20 minutes at this clock. That is a day
/// nine times too short for a world whose metre is a voxel's eighth, and nothing derived it, so
/// no scale could be checked against it. The day is now the scale's own statement: one eighth of
/// a day's *seconds*, at the rate the clock already declares. A full day/night cycle therefore
/// takes three hours, and every daily cadence in the tree — weather fronts, growth, the seasonal
/// ramp — keeps its *fraction* of a day and takes three hours of the frame's time.
pub const TICKS_PER_DAY: u64 =
    SECONDS_PER_DAY / crate::scale::REAL_METRES_PER_VOXEL as u64 * TICKS_PER_SECOND;
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

    // The atlas's seasonal-range mechanism (§2): an interior column swings wider than a coastal
    // one at the same latitude and height. Applied to the **amplitude** and not to the base, which
    // is what keeps a maritime margin mild in both directions rather than merely cooler.
    let inland = 1.0 + CONTINENTAL_SEASONAL_GAIN * continentality(spec, x, z);

    TemperatureParts {
        base_c: cfg.base_temperature_c + latitude - lapse + belt_c,
        seasonal_amplitude_c: MAX_SEASONAL_SWING_C * lat_scale * (0.7 + 0.3 * belt01) * inland,
    }
}

/// The peak horizontal temperature variation, in °C, that the climate belt
/// contributes either side of the world's mean. Declared.
const CLIMATE_BELT_C: f64 = 9.0;

/// The frequency the **continentalness** field is read at, as a fraction of the terrain's own
/// `continent_frequency`. Declared: a coastline is the terrain's business, and continentalness is
/// the landmass's *shape* — the same fractal, the same seed, one scale up.
const CONTINENTAL_SCALE: f64 = 0.25;

/// The top of the continental field's own range — the same normalisation the climate belt uses
/// (`fbm · 0.5 + 0.5` reads the field on `[0, 1]`), so `1` is the field's highest ground and not a
/// tuned threshold. The interior gradient runs from the **coastline** ([`crate::terrain::is_regional_sea`]'s
/// own line, `cont = 2 · (sea_level_m − base_elevation_m) / relief_m`) to this top, which is why the
/// mapping needs no interior band to pick: the whole positive range of the field that draws the
/// world's continents *is* the maritime-to-interior gradient.
const CONTINENTAL_TOP: f64 = 1.0;

/// How much of the boundary layer's moisture a **deep interior** column keeps, as a fraction.
///
/// The atlas's moisture trajectory: *"air masses may lose moisture as they travel over land"*. One
/// declared number, applied as `1 − LOSS · continentality`, so the maritime margin is unchanged and
/// the interior keeps 55 % of what the belt would have given it. It is the same *kind* of number as
/// [`ALTITUDE_ATTENUATION`], and for the same reason: a world where every column keeps its belt
/// moisture has no interior, and the atlas's steppe and cold desert are what the loss makes.
const CONTINENTAL_MOISTURE_LOSS: f64 = 0.45;

/// How much **larger** an interior column's seasonal temperature swing is, as a fraction of a
/// maritime column's.
///
/// The atlas's first mechanism and the one its §2 leads with: *"inland surfaces commonly experience
/// hotter summers and colder winters than nearby maritime settings at comparable latitude and
/// elevation."* Declared at the same size as the maritime swing itself, so a deep-interior column
/// swings at 1.6× a coastal one — the ordering the guide states, with the amplitude stated rather
/// than fitted.
const CONTINENTAL_SEASONAL_GAIN: f64 = 0.6;

/// **Continentalness**: how deep inside a landmass a column sits, `[0, 1]` — `0` at the maritime
/// margin, `1` in the deep interior.
///
/// **Source.** `docs/earth_biomes_subbiomes_continentalness_mountains.pdf` §2: the strength of
/// land-interior influence relative to the thermal and moisture buffering of the nearby ocean. The
/// guide is explicit that this is *"not a simple distance-to-coast rule"* — prevailing winds,
/// currents, barriers and latitude all move it — so what is read here is the **landmass's own
/// shape** rather than a geodesic distance: the same continental fractal the terrain law draws its
/// coastlines from ([`crate::terrain::continental_field_m`]'s family and seed), sampled one scale
/// coarser, mapped through the sea line that same law defines.
///
/// **The proxy, stated.** It is a shape measure, not a distance: a peninsula that points into the
/// ocean reads low (correctly) and a plateau beside a narrow inland sea reads high (also correct —
/// a narrow sea buffers far less than an ocean). What it does **not** resolve is a long, thin
/// landmass whose spine is genuinely far from water on both sides but low in the field; the world's
/// own measurement is in this module's tests, which read the proxy against the distance to the
/// nearest sea over a grid rather than asserting the shape is right.
///
/// **Its range in this world, measured.** The default `continent_frequency` is `0.0025`, which is a
/// 400 m base wavelength: this world's landmasses are a few hundred metres across inside a 6 km
/// half-extent, so *almost every land column is near its own coast* and the field's measured deciles
/// over land run `0.000 … 0.03 … 0.15 … 0.63` (`the_continentalness_range_prints`). That is not a
/// defect in the mapping — a high coastline-to-area ratio is what such a world *has* — but it does
/// mean the maritime end is the populated one here, and a world that wants interiors says so by
/// giving `continent_frequency` a value that makes continents bigger than its own horizon.
///
/// **Cost.** One fbm of two octaves — no ring search and no terrain sampling, so a caller may read
/// it per column without a hydro-style budget. Deterministic: seed and position only, no tick.
pub fn continentality(spec: &WorldSpec, x: f64, z: f64) -> f64 {
    let cfg = &spec.config;
    let f = Fractal::new(2, 2.0, 0.5, Kind::Perlin);
    let raw = f.fbm(
        spec.stationary_stage_seed("terrain"),
        Vec3::new(
            x * cfg.continent_frequency * CONTINENTAL_SCALE,
            0.0,
            z * cfg.continent_frequency * CONTINENTAL_SCALE,
        ),
    );
    let coast = 2.0 * (cfg.sea_level_m - cfg.base_elevation_m) / cfg.relief_m;
    ((raw - coast) / (CONTINENTAL_TOP - coast)).clamp(0.0, 1.0)
}

/// The altitude attenuation of boundary-layer moisture — 0.7 is a **declared** fraction: at full
/// relief height the air retains 30 % of the lowland moisture. Stated so that it can be tuned
/// deliberately.
const ALTITUDE_ATTENUATION: f64 = 0.7;

/// The three factors [`moisture`] multiplies, kept apart.
///
/// **Why the product is exposed in parts.** Three different processes move this number — the
/// humidity belt (a field), the altitude loss (the terrain's own height), and the atlas's inland
/// loss (the distance from the ocean) — and a reader that can only see their product cannot tell
/// which one changed when a world's biomes move. It is the same ruling [`TemperatureParts`] carries,
/// for the same reason.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct MoistureParts {
    /// The low-frequency humidity belt, `[0, 1]`.
    pub belt: f64,
    /// What the altitude leaves of it, `[0, 1]`.
    pub altitude_retention: f64,
    /// What the distance from the ocean leaves of it, `[0, 1]` — `1 − loss · continentality`.
    pub continental_retention: f64,
}

/// Boundary-layer moisture in `[0, 1]`.
///
/// Three coupled terms and no second noise field: a low-frequency humidity belt, attenuated by
/// **altitude** (air that has risen and dropped its water is dry, so height and moisture are
/// anti-correlated — which is why a mountain top is bare rock and the valley beside it is not), and
/// attenuated by **continentalness** (the atlas's §2 moisture trajectory: air that has travelled
/// over land has rained on it). The last is the term that makes an interior different from a
/// coastline at the same height and the same belt.
pub fn moisture(spec: &WorldSpec, x: f64, z: f64) -> f64 {
    let parts = moisture_parts(spec, x, z);
    (parts.belt * parts.altitude_retention * parts.continental_retention).clamp(0.0, 1.0)
}

/// The three factors of [`moisture`], read once each.
pub fn moisture_parts(spec: &WorldSpec, x: f64, z: f64) -> MoistureParts {
    let cfg = &spec.config;
    let f = Fractal::new(4, 2.0, 0.5, Kind::Perlin);
    let belt = f.fbm(
        spec.stationary_stage_seed("moisture"),
        Vec3::new(x * cfg.continent_frequency * 1.7, 0.0, z * cfg.continent_frequency * 1.7),
    ) * 0.5
        + 0.5;

    let elev = terrain::elevation_m(spec, x, z);
    let height = ((elev - cfg.sea_level_m).max(0.0) / cfg.relief_m).clamp(0.0, 1.0);

    MoistureParts {
        belt,
        altitude_retention: 1.0 - ALTITUDE_ATTENUATION * height,
        continental_retention: 1.0 - CONTINENTAL_MOISTURE_LOSS * continentality(spec, x, z),
    }
}

/// Everything climate says about one column.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ClimateSample {
    pub temperature_c: f64,
    pub moisture: f64,
    pub season_phase: f64,
    /// How deep inside a landmass the column sits, `[0, 1]` — see [`continentality`].
    pub continentality: f64,
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
        continentality: continentality(spec, x, z),
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
    fn the_continentalness_proxy_tracks_the_distance_to_the_sea() {
        // **The proxy, measured rather than asserted.** `continentality` is the landmass's shape,
        // not a geodesic distance, so the claim worth checking is the one the atlas makes: the
        // maritime margin buffers and the interior does not. The ground truth is read from the
        // terrain itself over a grid — for every land column, the distance to the nearest column
        // `terrain::is_regional_sea` calls sea — and the two ends of the proxy are compared.
        let s = spec();
        const STEP: f64 = 120.0;
        const HALF: i32 = 60;
        let mut sea: Vec<(f64, f64)> = Vec::new();
        let mut land: Vec<(f64, f64, f64)> = Vec::new();
        for i in -HALF..=HALF {
            for j in -HALF..=HALF {
                let (x, z) = (i as f64 * STEP, j as f64 * STEP);
                if terrain::is_regional_sea(&s, x, z) {
                    sea.push((x, z));
                } else {
                    land.push((x, z, continentality(&s, x, z)));
                }
            }
        }
        assert!(!sea.is_empty() && !land.is_empty(), "the grid saw only one kind of ground");
        let nearest_sea = |x: f64, z: f64| {
            sea.iter()
                .map(|(sx, sz)| ((sx - x).powi(2) + (sz - z).powi(2)).sqrt())
                .fold(f64::INFINITY, f64::min)
        };
        land.sort_by(|a, b| a.2.partial_cmp(&b.2).expect("continentality is finite"));
        let n = land.len();
        let mean = |slice: &[(f64, f64, f64)]| {
            slice.iter().map(|(x, z, _)| nearest_sea(*x, *z)).sum::<f64>() / slice.len() as f64
        };
        let (maritime, interior) = (&land[..n / 3], &land[2 * n / 3..]);
        let (near, far) = (mean(maritime), mean(interior));
        assert!(
            far > near,
            "the interior tercile sits {far:.0} m from the sea and the maritime one {near:.0} m — \
             the proxy is not reading the coast at all"
        );
        let (bottom, top) = (
            land.first().expect("non-empty").2,
            land.last().expect("non-empty").2,
        );
        assert!(
            top - bottom > 0.3 && interior.first().expect("non-empty").2 > maritime.last().expect("non-empty").2,
            "the land's continentality runs {bottom:.3} … {top:.3}, which separates too little to be \
             a gradient"
        );
    }

    #[test]
    fn the_continentalness_range_prints() {
        // The house instrument, for the same reason `census::tests::the_prose_frame_prints` is one:
        // the numbers a rule is calibrated on belong in a run anyone can repeat, not in a constant's
        // comment. Deciles of `continentality` over **land** columns of one 12 km world.
        let s = spec();
        let mut values = Vec::new();
        for x in (-6000..6000).step_by(97) {
            for z in (-6000..6000).step_by(101) {
                let (xf, zf) = (f64::from(x), f64::from(z));
                if !terrain::is_regional_sea(&s, xf, zf) {
                    values.push(continentality(&s, xf, zf));
                }
            }
        }
        values.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
        print!("continentality over {} land columns, deciles:", values.len());
        for d in 1..=10 {
            let i = (values.len() * d / 10).min(values.len() - 1);
            print!(" {:.3}", values[i]);
        }
        println!();
        assert!(values.len() > 500, "only {} land columns", values.len());
    }

    #[test]
    fn the_interior_keeps_less_of_the_belt_s_moisture() {
        // And the consequence, measured on pairs the **other two** factors cannot explain. The
        // moisture is a product of three; a pair whose belt and altitude retention agree to within
        // two percent differs only through the continental term, so the drier one must be the inland
        // one. An earlier version of this test compared pairs at equal *elevation*, and it failed
        // 94 % of the time for the honest reason: equal height leaves the belt free to differ, and
        // the belt is the larger term. The comparison has to hold the parts fixed, which is why
        // `moisture_parts` exists.
        let s = spec();
        let mut columns = Vec::new();
        for x in (-6000..6000).step_by(311) {
            for z in (-6000..6000).step_by(367) {
                let (xf, zf) = (f64::from(x), f64::from(z));
                let parts = moisture_parts(&s, xf, zf);
                columns.push((
                    xf,
                    zf,
                    parts.belt,
                    parts.altitude_retention,
                    continentality(&s, xf, zf),
                ));
            }
        }
        let mut pairs = 0;
        for (i, a) in columns.iter().enumerate() {
            for b in columns.iter().skip(i + 1) {
                if (a.2 - b.2).abs() > 0.02
                    || (a.3 - b.3).abs() > 0.02
                    || (a.4 - b.4).abs() < 0.3
                {
                    continue;
                }
                pairs += 1;
                let (dry, wet) = if a.4 > b.4 { (a, b) } else { (b, a) };
                assert!(
                    moisture(&s, dry.0, dry.1) < moisture(&s, wet.0, wet.1),
                    "at ({:.0}, {:.0}) the inland column ({:.2}) kept at least as much moisture as \
                     the maritime one ({:.2}) with the belt and the altitude held level",
                    dry.0,
                    dry.1,
                    dry.4,
                    wet.4
                );
            }
        }
        assert!(pairs > 20, "only {pairs} comparable pairs, so the sweep proved nothing");
    }

    #[test]
    fn the_retention_factors_are_fractions_and_the_product_is_the_moisture() {
        // The identity and the range, over the real world: each factor is a fraction, the maritime
        // margin is untouched at 1.0, and the product is what `moisture` returns. The inland end is
        // read from the world rather than assumed: an earlier version demanded the interior arm
        // cross 0.6 and this world — whose continents are 400 m across — never does, so the arm was
        // a claim about a world nobody had measured. What is asserted is that both ends exist and
        // that the gradient moves the moisture by a **declared fraction of the loss**.
        let s = spec();
        let mut lowest = 1.0f64;
        let mut highest = 0.0f64;
        let mut deepest = 0.0f64;
        for x in (-4000..4000).step_by(173) {
            for z in (-4000..4000).step_by(197) {
                let (xf, zf) = (f64::from(x), f64::from(z));
                let parts = moisture_parts(&s, xf, zf);
                let inland = continentality(&s, xf, zf);
                assert!((0.0..=1.0).contains(&parts.belt));
                assert!((0.0..=1.0).contains(&parts.altitude_retention));
                assert!(
                    (1.0 - CONTINENTAL_MOISTURE_LOSS..=1.0).contains(&parts.continental_retention)
                );
                // The retention **is** the declared relation to the field — no third term hides in
                // here, which is the whole point of reading the parts instead of the product.
                assert_eq!(
                    parts.continental_retention,
                    1.0 - CONTINENTAL_MOISTURE_LOSS * inland
                );
                let product =
                    (parts.belt * parts.altitude_retention * parts.continental_retention).clamp(0.0, 1.0);
                assert_eq!(product, moisture(&s, xf, zf));
                lowest = lowest.min(parts.continental_retention);
                highest = highest.max(parts.continental_retention);
                deepest = deepest.max(inland);
            }
        }
        assert_eq!(highest, 1.0, "the maritime margin was not left untouched");
        // The threshold is the **sweep's own** deepest inland reading, not a number chosen here:
        // `this world's continents are 400 m across`, so the inland end is shallow everywhere and a
        // hand-picked "interior" depth would be a claim about a world nobody measured.
        assert!(
            deepest > 0.3,
            "the sweep never got inland at all (deepest {deepest:.3})"
        );
        assert_eq!(lowest, 1.0 - CONTINENTAL_MOISTURE_LOSS * deepest);
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
