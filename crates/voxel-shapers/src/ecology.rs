//! Ecology — biomes from climate, and flocking as a behaviour.
//!
//! **Sources.**
//! * Holdridge, L. *Determination of World Plant Formations from Simple Climatic
//!   Data.* Science 1947 — a plant formation is a function of **biotemperature**
//!   and **precipitation**, and of nothing else. That is the whole reason this
//!   shaper can be built at all: climate is already computed by
//!   [`crate::climate`] and [`crate::weather`], so a biome is a *reading* of two
//!   numbers rather than a third generator.
//! * Reynolds, C. *Flocks, Herds and Schools: A Distributed Behavioral Model.*
//!   SIGGRAPH 1987 — **boids**: separation, alignment and cohesion, from which
//!   flocking emerges without anything telling the flock where to go.
//!
//! # Flocking is a behaviour, not a species (S-23)
//!
//! The slot ruling is explicit: *"flocking is a behaviour on any agent, nothing
//! craftable."* So [`flock_step`] takes **boids**, not `Birds`. A mod author or a
//! later shaper can put a flock on anything that moves, and no creature type gets
//! a private flocking implementation.

use crate::spec::WorldSpec;
use crate::{climate, weather};
use voxel_noise::Vec3;

/// A plant formation. The names are Holdridge's, shortened.
#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub enum Biome {
    PolarDesert,
    Tundra,
    BorealForest,
    CoolTemperateForest,
    TemperateForest,
    Grassland,
    Desert,
    SubtropicalScrub,
    TropicalDryForest,
    TropicalRainforest,
    Alpine,
}

impl Biome {
    pub const ALL: [Biome; 11] = [
        Biome::PolarDesert,
        Biome::Tundra,
        Biome::BorealForest,
        Biome::CoolTemperateForest,
        Biome::TemperateForest,
        Biome::Grassland,
        Biome::Desert,
        Biome::SubtropicalScrub,
        Biome::TropicalDryForest,
        Biome::TropicalRainforest,
        Biome::Alpine,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Biome::PolarDesert => "polar_desert",
            Biome::Tundra => "tundra",
            Biome::BorealForest => "boreal_forest",
            Biome::CoolTemperateForest => "cool_temperate_forest",
            Biome::TemperateForest => "temperate_forest",
            Biome::Grassland => "grassland",
            Biome::Desert => "desert",
            Biome::SubtropicalScrub => "subtropical_scrub",
            Biome::TropicalDryForest => "tropical_dry_forest",
            Biome::TropicalRainforest => "tropical_rainforest",
            Biome::Alpine => "alpine",
        }
    }

    /// The three-value grouping the record calls the **biome triple** (FC-5.6),
    /// which is a coarse reading of the same climate and not a second classifier.
    pub const fn triple(self) -> (BiomeClass, BiomeClass, BiomeClass) {
        match self {
            Biome::PolarDesert | Biome::Tundra | Biome::Alpine => (
                BiomeClass::Cold,
                BiomeClass::Dry,
                BiomeClass::Open,
            ),
            Biome::BorealForest | Biome::CoolTemperateForest => (
                BiomeClass::Cold,
                BiomeClass::Wet,
                BiomeClass::Forest,
            ),
            Biome::TemperateForest => (
                BiomeClass::Temperate,
                BiomeClass::Wet,
                BiomeClass::Forest,
            ),
            Biome::Grassland => (
                BiomeClass::Temperate,
                BiomeClass::Mesic,
                BiomeClass::Open,
            ),
            Biome::Desert => (BiomeClass::Hot, BiomeClass::Dry, BiomeClass::Open),
            Biome::SubtropicalScrub => (
                BiomeClass::Hot,
                BiomeClass::Mesic,
                BiomeClass::Open,
            ),
            Biome::TropicalDryForest => (
                BiomeClass::Hot,
                BiomeClass::Mesic,
                BiomeClass::Forest,
            ),
            Biome::TropicalRainforest => (
                BiomeClass::Hot,
                BiomeClass::Wet,
                BiomeClass::Forest,
            ),
        }
    }
}

/// One axis of the biome triple.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BiomeClass {
    Cold,
    Temperate,
    Hot,
    Dry,
    Mesic,
    Wet,
    Open,
    Forest,
}

/// The biome a climate supports.
///
/// **Pure: two numbers in, one formation out, no position and no noise.** That is
/// deliberate — a classifier that consulted position would make two columns with
/// identical climate different biomes, and the whole point of deriving biomes from
/// climate is that it cannot.
///
/// # Holdridge's *inputs*, sampled from the ticked shapers
///
/// Holdridge classifies on **biotemperature** — the mean of twelve monthly means, with
/// values below 0 °C clamped to 0 and above 30 °C clamped to 30 — and on **mean annual**
/// precipitation. Those are not raw samples of the moment: [`holdridge_inputs`] reads
/// [`crate::climate::temperature_c`] and [`crate::weather::precipitation_mm`] at twelve
/// declared months of the tick's own world year, clamps each monthly mean, and hands the
/// aggregates to this function.
///
/// **This used to be otherwise, and the defect was measured.** The call site passed
/// `clim.temperature_c` and `wx.precipitation_mm` *instantaneously* (`docs/LEDGER.md`
/// §93.2): four of four probed columns changed biome within one world year and each
/// returned — `(0, 0)` ran `boreal_forest ↔ tundra`, twice — because the classifier is
/// Holdridge's *scheme* and was being fed one tick of a season. The ruling of 2026-10-02
/// was to aggregate before classifying, and the stability is now structural: the
/// `Climate` field the world's material law reads carries **no tick**, so there is
/// no route by which a season could reach the ground.
///
/// **The values are still the ticked functions'.** Nothing here re-derives a climate
/// model: the twelve samples are tick coordinates, so the world's shapers stay sampled and
/// ticked exactly as they were, and the instantaneous samples are untouched for every
/// consumer that wants *this tick*.
pub fn biome_for(temperature_c: f64, precipitation_mm: f64) -> Biome {
    // **The arms are the mounted package's**, not this function's: [`crate::biome_library::library`]
    // holds them with the atlas's own label, system and structure on each arm, and this reader
    // exists so that every caller of the formation keeps its signature. Cold is cold regardless of
    // rain — below freezing most of the year only the driest and the hardest formations survive —
    // and that is the first arm of the package rather than a branch here.
    crate::biome_library::climate_biome(temperature_c, precipitation_mm)
}

/// Ticks in one twelfth of a world year — **read from the climate module's own clock**,
/// so the sampling month and the year cannot drift apart.
pub const MONTH_TICKS: u64 = climate::YEAR_TICKS / 12;

/// How many ticked samples make one month's mean.
///
/// **Declared, and it is a resolution rather than a count.** The seasonal term is a
/// sinusoid with a one-year period, so sampling a month at its four quarter-midpoints
/// under-resolves it by `sinc(π/24) ≈ 0.991` — under 1 % of the seasonal swing, against
/// the classifier's 5 °C arms. One sample a month would read the sine at a point and call
/// it a mean; four is the least that can be defended as one.
pub const MONTH_SAMPLES: u64 = 4;

/// **Holdridge's two numbers, sampled from the ticked shapers.**
///
/// Twelve monthly means of [`climate::temperature_c`], each **clamped to `[0, 30]`** —
/// the clamp is the whole reason biotemperature differs from an annual mean — then
/// averaged; and the mean annual total of [`weather::precipitation_mm`], taken as the
/// mean of the twelve monthly means.
///
/// **Every sample is a tick coordinate**, and the twelve months are the months of the
/// world year the tick falls in. The cycle is periodic, so the aggregate does not move
/// with the tick within a year and does not differ between years; the shapers stay
/// sampled and ticked, and the biome becomes the regional fact Holdridge describes.
pub fn holdridge_inputs(spec: &WorldSpec, x: f64, z: f64, y_m: f64, tick: u64) -> (f64, f64) {
    let base = (tick / climate::YEAR_TICKS) * climate::YEAR_TICKS;

    // **The x/z terms are read once, not once per sample.** Both shapers separate into a
    // column factor and a tick scalar — `temperature_c` is `base_c + amplitude · unit(tick)`
    // and `precipitation_mm` is `dry + gain · seasonal_gain(tick)` — so the twelve monthly
    // means below evaluate the column's climate once and the scalars ninety-six times. The
    // monthly mean is the same arithmetic reassociated (`mean(a + b·u) = a + b·mean(u)`),
    // which is why the sampled rule and the sampled numbers both stand: this measured
    // **202 µs per column** before it existed (`examples/biome-cost.rs`), and a 32×32
    // chunk of columns is a chunk build's whole budget.
    let temperature = climate::temperature_parts(spec, x, z, y_m);
    let precipitation = weather::precipitation_parts(spec, x, z);
    let mut biotemperature_sum = 0.0;
    let mut precipitation_sum = 0.0;

    for month in 0..12u64 {
        let mut temperature_sum = 0.0;
        let mut precipitation_month = 0.0;
        for sample in 0..MONTH_SAMPLES {
            // The sample sits at the middle of its sub-interval, so a month is sampled
            // symmetrically and neither edge of it is double-counted.
            let offset = (2 * sample + 1) * MONTH_TICKS / (2 * MONTH_SAMPLES);
            let at = base + month * MONTH_TICKS + offset;
            temperature_sum +=
                temperature.base_c + temperature.seasonal_amplitude_c * climate::seasonal_unit(at);
            precipitation_month += precipitation.dry_mm
                + precipitation.wet_gain_mm * weather::seasonal_gain(at);
        }
        let monthly_mean = temperature_sum / MONTH_SAMPLES as f64;
        // Holdridge's clamp is on the *monthly mean*, and it is why a column whose winter
        // is far below freezing still has a biotemperature near zero rather than negative.
        biotemperature_sum += monthly_mean.clamp(0.0, 30.0);
        precipitation_sum += precipitation_month / MONTH_SAMPLES as f64;
    }

    (biotemperature_sum / 12.0, precipitation_sum / 12.0)
}

/// Above this height, a column is alpine regardless of its latitude's climate.
/// Declared, because "a mountain is its own biome" is a decision.
pub const ALPINE_HEIGHT_ABOVE_RELIEF: f64 = 0.85;

/// The biome at a column, including the **altitude override**.
///
/// The two climate numbers are [`holdridge_inputs`]' — **biotemperature** and **mean
/// annual precipitation** — and not an instantaneous sample; see the note on
/// [`biome_for`].
pub fn biome_at(
    spec: &WorldSpec,
    elevation_m: f64,
    biotemperature_c: f64,
    mean_annual_precipitation_mm: f64,
) -> Biome {
    let above = (elevation_m - spec.config.sea_level_m).max(0.0);
    if above > spec.config.relief_m * ALPINE_HEIGHT_ABOVE_RELIEF {
        // Alpine is not a Holdridge formation — it is what altitude does to one.
        // The override is a declared rule and is applied *after* the classifier,
        // so the classifier stays a pure function of climate.
        return Biome::Alpine;
    }
    biome_for(biotemperature_c, mean_annual_precipitation_mm)
}

// -------------------------------------------------------------------------------------
// The two capacities the process roster declares and nothing produced
// -------------------------------------------------------------------------------------
//
// `FieldId::CarryingCapacity` and `FieldId::SpeciesSuitability` are declared in the
// nineteen-name roster and had **no law** (`process.rs::owed_body(Ecology)`: *"neither
// carrying capacity nor species suitability has an admitted law here"*). The two
// functions below admit one, from a named source, so the stage can run.
//
// **Sources.**
// * Lieth, H. *Modeling the primary productivity of the world.* In: Primary
//   Productivity of the Biosphere, Springer 1975 — the **Miami model**: annual net
//   primary productivity from mean annual temperature and precipitation, two curves,
//   and the **minimum** of the two is the limiting one (Liebig's law).
// * Holdridge, L. *Determination of World Plant Formations from Simple Climatic Data.*
//   Science 1947 — the **potential evapotranspiration** is linear in biotemperature,
//   `PET = 58.93 · T_bio` mm/yr, and the **humidity province** is the ratio `P / PET`.

/// The Miami model's saturation constant, in g/m²/yr (Lieth 1975). It is the model's
/// own number, not a tuned dial.
pub const MIAMI_MAX_NPP_G_M2_YR: f64 = 3000.0;
/// Holdridge's PET coefficient: `PET_mm = 58.93 · biotemperature_C` (Holdridge 1947).
pub const HOLDGRIDGE_PET_COEFFICIENT: f64 = 58.93;
/// The soil depth at which soil stops limiting productivity. **Declared**: it is this
/// tree's reading of Heimsath et al. 1997's soil-production baseline, not the paper's.
pub const REFERENCE_SOIL_DEPTH_M: f64 = 0.5;

/// Potential evapotranspiration, in mm/yr (Holdridge 1947).
///
/// `None` for a biotemperature that is not finite or is below zero — the coefficient is
/// defined on Holdridge's clamped `[0, 30]` biotemperature, and a negative PET is a
/// number somebody could divide by.
pub fn potential_evapotranspiration_mm(biotemperature_c: f64) -> Option<f64> {
    if !biotemperature_c.is_finite() || biotemperature_c < 0.0 {
        return None;
    }
    Some(HOLDGRIDGE_PET_COEFFICIENT * biotemperature_c)
}

/// Annual net primary productivity, in g/m²/yr — **Lieth's Miami model**.
///
/// `NPP_T = 3000 / (1 + e^{1.315 − 0.119·T})` and
/// `NPP_P = 3000 · (1 − e^{−0.000664·P})`, and the answer is their **minimum** because
/// the scarcer of the two resources limits growth.
///
/// `None` when either input is not finite: a productivity that cannot be computed is not
/// zero productivity, and a zero here would read as a barren province.
pub fn miami_npp_g_m2_yr(biotemperature_c: f64, mean_annual_precipitation_mm: f64) -> Option<f64> {
    if !biotemperature_c.is_finite() || !mean_annual_precipitation_mm.is_finite() {
        return None;
    }
    if mean_annual_precipitation_mm < 0.0 {
        return None;
    }
    let by_temperature = MIAMI_MAX_NPP_G_M2_YR / (1.0 + (1.315 - 0.119 * biotemperature_c).exp());
    let by_precipitation =
        MIAMI_MAX_NPP_G_M2_YR * (1.0 - (-0.000664 * mean_annual_precipitation_mm).exp());
    Some(by_temperature.min(by_precipitation))
}

/// **Carrying capacity**, as the dimensionless ratio a region can stock: the Miami
/// productivity as a fraction of the model's own maximum, limited by soil.
///
/// The soil term is `min(1, depth / REFERENCE_SOIL_DEPTH_M)`, so a site with less than the
/// reference soil carries proportionally less, and one with more does not carry more
/// than the climate allows. In `[0, 1]`.
///
/// `None` when the productivity cannot be computed or the soil depth is not finite/
/// negative — the refusal [`crate::process`] turns into `NegativeCarryingCapacity`.
pub fn carrying_capacity(
    biotemperature_c: f64,
    mean_annual_precipitation_mm: f64,
    soil_depth_m: f64,
) -> Option<f64> {
    if !soil_depth_m.is_finite() || soil_depth_m < 0.0 {
        return None;
    }
    let npp = miami_npp_g_m2_yr(biotemperature_c, mean_annual_precipitation_mm)?;
    let soil_factor = (soil_depth_m / REFERENCE_SOIL_DEPTH_M).clamp(0.0, 1.0);
    Some((npp / MIAMI_MAX_NPP_G_M2_YR) * soil_factor)
}

/// **Species suitability**, as the dimensionless fit of a site to its own province in
/// `[0, 1]`: perfect when the humidity ratio is exactly 1, falling to zero in both a
/// desert and a superhumid bog.
///
/// Holdridge's humidity ratio is `r = P / PET`. This reading is **this tree's declared
/// shape over that ratio** — `fit = min(r, 1/r)`, multiplied by the same soil factor as
/// [`carrying_capacity`] — and is named as such rather than attributed to a paper; the
/// ratio itself is Holdridge 1947's, and no species model is claimed.
///
/// `None` on a non-finite input, a negative PET (impossible from
/// [`potential_evapotranspiration_mm`]), or a negative soil depth.
pub fn species_suitability(
    biotemperature_c: f64,
    mean_annual_precipitation_mm: f64,
    soil_depth_m: f64,
) -> Option<f64> {
    if !soil_depth_m.is_finite() || soil_depth_m < 0.0 {
        return None;
    }
    let pet = potential_evapotranspiration_mm(biotemperature_c)?;
    if pet <= 0.0 {
        // At a biotemperature of zero there is no evapotranspiration to divide by, so the
        // humidity ratio is undefined rather than infinite; the site is at the cold end
        // and its suitability is the soil-limited minimum.
        return Some((soil_depth_m / REFERENCE_SOIL_DEPTH_M).clamp(0.0, 1.0));
    }
    if !mean_annual_precipitation_mm.is_finite() || mean_annual_precipitation_mm < 0.0 {
        return None;
    }
    let ratio = mean_annual_precipitation_mm / pet;
    if ratio <= 0.0 {
        return Some(0.0);
    }
    let fit = ratio.min(1.0 / ratio).clamp(0.0, 1.0);
    let soil_factor = (soil_depth_m / REFERENCE_SOIL_DEPTH_M).clamp(0.0, 1.0);
    Some(fit * soil_factor)
}

/// A point with a velocity — a boid, or anything that wants to flock.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Boid {
    pub pos: Vec3,
    pub vel: Vec3,
}

/// The boid rules' weights and radii. All declared.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct FlockParams {
    pub max_speed: f64,
    /// Inside this radius, boids push apart.
    pub separation_radius: f64,
    /// Inside this radius, boids are neighbours for alignment and cohesion.
    pub neighbour_radius: f64,
    pub separation_weight: f64,
    pub alignment_weight: f64,
    pub cohesion_weight: f64,
    pub dt: f64,
}

impl Default for FlockParams {
    fn default() -> Self {
        FlockParams {
            max_speed: 8.0,
            separation_radius: 2.0,
            neighbour_radius: 12.0,
            separation_weight: 1.6,
            alignment_weight: 0.9,
            cohesion_weight: 0.8,
            dt: 0.05,
        }
    }
}

/// Advance every boid one step.
///
/// **Deterministic**: accelerations are computed into a scratch buffer in index
/// order and then applied, so no boid sees a neighbour that has already moved
/// this step and the result does not depend on iteration order (F12).
///
/// Returns how many boids were actually moved — a boid with no neighbours in
/// range and no separation pressure does not move, and reporting that count keeps
/// the caller from assuming motion.
pub fn flock_step(boids: &mut [Boid], p: &FlockParams) -> usize {
    let n = boids.len();
    let mut accel = vec![Vec3::ZERO; n];

    for i in 0..n {
        let mut sep = Vec3::ZERO;
        let mut ali = Vec3::ZERO;
        let mut coh = Vec3::ZERO;
        let mut sep_n = 0u32;
        let mut nb_n = 0u32;

        for j in 0..n {
            if i == j {
                continue;
            }
            let d = boids[j].pos - boids[i].pos;
            let dist = d.length();
            if dist > p.neighbour_radius || dist <= 0.0 {
                continue;
            }
            nb_n += 1;
            ali += boids[j].vel;
            coh += boids[j].pos;
            if dist < p.separation_radius {
                // Push **apart**: the direction is `i − j`, not `j − i`. An
                // earlier revision accumulated `d`, which points from this boid
                // toward its neighbour, and so drove overlapping boids *together*
                // — an attractor disguised as a repulsion. The test caught it.
                sep += (boids[i].pos - boids[j].pos) * (1.0 / dist);
                sep_n += 1;
            }
        }

        let mut a = Vec3::ZERO;
        if sep_n > 0 {
            a += sep.normalise() * p.separation_weight;
        }
        if nb_n > 0 {
            let ali = ali / nb_n as f64;
            a += (ali - boids[i].vel) * p.alignment_weight;
            let coh = coh / nb_n as f64;
            let to_centre = coh - boids[i].pos;
            a += to_centre.normalise() * p.cohesion_weight;
        }
        accel[i] = a;
    }

    let mut moved = 0;
    for i in 0..n {
        let before = boids[i].pos;
        let mut vel = boids[i].vel + accel[i] * p.dt;
        // Clamp to max speed. A flock that can accelerate without limit is not a
        // flock, it is a projectile.
        let speed = vel.length();
        if speed > p.max_speed {
            vel = vel * (p.max_speed / speed);
        }
        boids[i].vel = vel;
        boids[i].pos += vel * p.dt;
        if boids[i].pos != before {
            moved += 1;
        }
    }
    moved
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Config;

    fn spec() -> WorldSpec {
        WorldSpec::new(314, Config::default())
    }

    #[test]
    fn the_classifier_covers_every_biome_for_some_climate() {
        // A classifier with an unreachable state is a classifier with a dead arm,
        // and the dead arm is always the one a later change assumes works.
        let mut seen = std::collections::BTreeSet::new();
        let mut t = -30.0;
        while t < 40.0 {
            let mut p = 0.0;
            while p < 3000.0 {
                let b = biome_for(t, p);
                if seen.insert(b) {
                    assert!(!b.name().is_empty());
                }
                p += 25.0;
            }
            t += 1.0;
        }
        for b in Biome::ALL {
            // **Alpine is the one exception, and it is deliberate.** It is not a
            // Holdridge formation — it is what *altitude* does to one — so it is
            // applied by `biome_at` as an override and `biome_for` must never
            // return it. `altitude_overrides_latitude` covers that path.
            if b == Biome::Alpine {
                continue;
            }
            assert!(seen.contains(&b), "{b:?} is unreachable by any climate");
        }
        assert!(!seen.contains(&Biome::Alpine), "biome_for returned Alpine directly");
    }

    #[test]
    fn warmer_or_wetter_never_gives_a_more_arid_biome() {
        // Monotonicity in precipitation, which is the property that makes the
        // classifier a classifier rather than a lookup with holes.
        let dry_order = |b: Biome| match b.triple().1 {
            BiomeClass::Dry => 0,
            BiomeClass::Mesic => 1,
            BiomeClass::Wet => 2,
            _ => 1,
        };
        for t in [-3.0, 8.0, 16.0, 25.0] {
            let mut last = 0;
            let mut p = 0.0;
            while p < 2500.0 {
                let w = dry_order(biome_for(t, p));
                assert!(w >= last, "at {t} °C, more rain gave a drier biome at {p} mm");
                last = w;
                p += 50.0;
            }
        }
    }

    #[test]
    fn altitude_overrides_latitude() {
        let s = spec();
        let tropical = biome_at(&s, s.config.sea_level_m, 26.0, 1500.0);
        assert_eq!(tropical, Biome::TropicalRainforest);
        let high = biome_at(
            &s,
            s.config.sea_level_m + s.config.relief_m * 0.95,
            26.0,
            1500.0,
        );
        assert_eq!(high, Biome::Alpine, "a high mountain kept its lowland biome");
    }

    #[test]
    fn a_biome_is_a_function_of_climate_and_nothing_else() {
        // Two calls, same climate, no position argument to disagree about.
        assert_eq!(biome_for(10.0, 700.0), biome_for(10.0, 700.0));
    }

    #[test]
    fn productivity_rises_with_temperature_and_with_rain_and_is_limited_by_the_scarcer_one() {
        // Liebig's minimum: the answer is exactly the smaller of the two curves, so a
        // warmer world never produces less and a colder one never more.
        assert!(miami_npp_g_m2_yr(20.0, 1000.0) > miami_npp_g_m2_yr(10.0, 1000.0));
        assert!(miami_npp_g_m2_yr(20.0, 1500.0) > miami_npp_g_m2_yr(20.0, 500.0));
        let rain_limited = miami_npp_g_m2_yr(30.0, 50.0).expect("finite");
        let temperature_limited = miami_npp_g_m2_yr(30.0, 4000.0).expect("finite");
        assert!(
            rain_limited < temperature_limited,
            "the scarcer resource was not the limiting one: {rain_limited} vs {temperature_limited}"
        );
        // The temperature curve saturates below the model's own maximum.
        assert!(temperature_limited <= MIAMI_MAX_NPP_G_M2_YR);
        assert!(miami_npp_g_m2_yr(25.0, 2000.0).expect("finite") <= MIAMI_MAX_NPP_G_M2_YR);
    }

    #[test]
    fn a_non_finite_climate_gives_no_capacity_rather_than_a_zero() {
        // Zero is a value somebody could believe; a refusal is not.
        assert!(miami_npp_g_m2_yr(f64::NAN, 1000.0).is_none());
        assert!(miami_npp_g_m2_yr(20.0, f64::INFINITY).is_none());
        assert!(miami_npp_g_m2_yr(20.0, -1.0).is_none());
        assert!(carrying_capacity(20.0, 1000.0, f64::NAN).is_none());
        assert!(carrying_capacity(20.0, 1000.0, -1.0).is_none());
    }

    #[test]
    fn soil_limits_capacity_but_never_raises_it_above_the_climate() {
        let deep = carrying_capacity(22.0, 1600.0, 10.0).expect("finite");
        let reference = carrying_capacity(22.0, 1600.0, REFERENCE_SOIL_DEPTH_M).expect("finite");
        let thin = carrying_capacity(22.0, 1600.0, 0.05).expect("finite");
        assert!((deep - reference).abs() < 1e-12, "extra soil raised capacity above the climate");
        assert!(thin < reference, "thin soil did not lower capacity");
        assert!(thin >= 0.0, "capacity went negative");
    }

    #[test]
    fn suitability_peaks_at_the_humid_province_and_falls_away_both_ways() {
        // The ratio `P / PET` is Holdridge's; the peak at 1 is this tree's declared shape.
        // PET at 20 °C is 1178.6 mm, so 1178.6 mm of rain is the optimum.
        let pet = potential_evapotranspiration_mm(20.0).expect("finite");
        let at_optimum = species_suitability(20.0, pet, REFERENCE_SOIL_DEPTH_M).expect("finite");
        let desert = species_suitability(20.0, pet * 0.1, REFERENCE_SOIL_DEPTH_M).expect("finite");
        let bog = species_suitability(20.0, pet * 10.0, REFERENCE_SOIL_DEPTH_M).expect("finite");
        assert!((at_optimum - 1.0).abs() < 1e-12, "the optimum was not 1: {at_optimum}");
        assert!(desert < at_optimum && bog < at_optimum, "suitability did not fall away");
        assert!(desert < 0.2 && bog < 0.2, "the tails were not small: {desert}, {bog}");
        // A colder province has a smaller PET, so the same rain is wetter there.
        assert!(species_suitability(5.0, 400.0, REFERENCE_SOIL_DEPTH_M).expect("finite") > 0.0);
    }

    #[test]
    fn a_frozen_column_has_no_pet_to_divide_by_and_is_read_from_soil_alone() {
        // At a biotemperature of zero the humidity ratio is undefined, and the honest
        // answer is the soil-limited reading rather than an infinite province.
        assert_eq!(potential_evapotranspiration_mm(0.0), Some(0.0));
        let suitability = species_suitability(0.0, 1000.0, REFERENCE_SOIL_DEPTH_M).expect("finite");
        assert!((suitability - 1.0).abs() < 1e-12, "a frozen column read {suitability}");
        assert!(potential_evapotranspiration_mm(-1.0).is_none());
    }

    #[test]
    fn the_biotemperature_clamps_each_monthly_mean_before_averaging_it() {
        // Holdridge's clamp is the whole reason biotemperature differs from an annual
        // mean: a month below freezing contributes 0, not its own negative value. So a
        // column whose year is mostly sub-zero must have a biotemperature **above** the
        // raw annual mean of the same ticked function — and inside Holdridge's range.
        let s = spec();
        let (x, z) = (0.0, 0.0);
        let y = crate::terrain::elevation_m(&s, x, z);
        let (biotemperature, mean_annual_precipitation) = holdridge_inputs(&s, x, z, y, 0);
        assert!(
            (0.0..=30.0).contains(&biotemperature),
            "biotemperature left Holdridge's range: {biotemperature}"
        );
        assert!(mean_annual_precipitation >= 0.0, "precipitation was negative");

        // The raw annual mean of the same ticked function, for the comparison.
        let mut raw = 0.0;
        let mut samples = 0u32;
        let mut t = 0u64;
        while t < climate::YEAR_TICKS {
            raw += climate::temperature_c(&s, x, z, y, t);
            samples += 1;
            t += climate::YEAR_TICKS / 48;
        }
        let raw_mean = raw / samples as f64;
        assert!(
            biotemperature >= raw_mean - 1e-9,
            "the clamp lowered biotemperature below the raw mean: {biotemperature} < {raw_mean}"
        );
    }

    #[test]
    fn the_hoisted_monthly_means_agree_with_the_unhoisted_shaper_calls() {
        // The one difference between `holdridge_inputs` and a loop that calls the full
        // shapers at every sample is the reassociation `a + b·u` versus the expanded
        // expression. This measures that the difference is a few ulps and that the
        // classification cannot move across it.
        let s = spec();
        let (x, z) = (1234.5, -678.5);
        let y = crate::terrain::elevation_m(&s, x, z);
        let tick = 17_000u64;
        let (biotemperature, precipitation) = holdridge_inputs(&s, x, z, y, tick);

        let base = (tick / climate::YEAR_TICKS) * climate::YEAR_TICKS;
        let mut biotemperature_sum = 0.0;
        let mut precipitation_sum = 0.0;
        for month in 0..12u64 {
            let mut temperature_sum = 0.0;
            let mut precipitation_month = 0.0;
            for sample in 0..MONTH_SAMPLES {
                let offset = (2 * sample + 1) * MONTH_TICKS / (2 * MONTH_SAMPLES);
                let at = base + month * MONTH_TICKS + offset;
                temperature_sum += climate::temperature_c(&s, x, z, y, at);
                precipitation_month += weather::precipitation_mm(&s, x, z, at);
            }
            biotemperature_sum += (temperature_sum / MONTH_SAMPLES as f64).clamp(0.0, 30.0);
            precipitation_sum += precipitation_month / MONTH_SAMPLES as f64;
        }
        let direct = (biotemperature_sum / 12.0, precipitation_sum / 12.0);
        assert!(
            (biotemperature - direct.0).abs() < 1e-9,
            "biotemperature {} vs {}",
            biotemperature,
            direct.0
        );
        assert!(
            (precipitation - direct.1).abs() < 1e-6,
            "precipitation {} vs {}",
            precipitation,
            direct.1
        );
        assert_eq!(
            biome_at(&s, y, biotemperature, precipitation),
            biome_at(&s, y, direct.0, direct.1),
            "the reassociation moved a column's biome"
        );
    }

    #[test]
    fn holdridge_inputs_are_sampled_and_do_not_move_within_a_year() {
        // The aggregate is a reading of the **ticked** shapers, so it is the same at
        // every tick of a year and the same in a later year (the cycle is periodic) —
        // and it still separates two different columns.
        let s = spec();
        let y = 0.0;
        let a = holdridge_inputs(&s, 400.0, -300.0, y, 0);
        let b = holdridge_inputs(&s, 400.0, -300.0, y, climate::YEAR_TICKS - 1);
        assert_eq!(a, b, "the aggregate moved with the tick inside one year");
        let c = holdridge_inputs(&s, 400.0, -300.0, y, climate::YEAR_TICKS * 5 + 12);
        assert_eq!(a, c, "two different world years disagreed about one column's climate");
        let other = holdridge_inputs(&s, -4000.0, 300.0, y, 0);
        assert_ne!(a, other, "two different columns had the same Holdridge inputs");
    }

    #[test]
    fn boids_stay_within_their_speed_limit_and_never_go_nan() {
        let mut boids: Vec<Boid> = (0..30)
            .map(|i| Boid {
                pos: Vec3::new(i as f64 * 3.0, 0.0, (i % 5) as f64 * 2.0),
                vel: Vec3::new(1.0, 0.0, 0.0),
            })
            .collect();
        let p = FlockParams::default();
        for _ in 0..60 {
            flock_step(&mut boids, &p);
            for b in &boids {
                assert!(b.pos.is_finite() && b.vel.is_finite());
                assert!(b.vel.length() <= p.max_speed + 1e-9, "speed escaped the clamp");
            }
        }
    }

    #[test]
    fn cohesion_pulls_a_scattered_flock_together() {
        // Two boids well within neighbour range must approach each other.
        let mut boids = vec![
            Boid { pos: Vec3::new(-3.0, 0.0, 0.0), vel: Vec3::ZERO },
            Boid { pos: Vec3::new(3.0, 0.0, 0.0), vel: Vec3::ZERO },
        ];
        let before = (boids[0].pos - boids[1].pos).length();
        for _ in 0..40 {
            flock_step(&mut boids, &FlockParams::default());
        }
        let after = (boids[0].pos - boids[1].pos).length();
        assert!(after < before, "the pair did not converge: {before} → {after}");
    }

    #[test]
    fn separation_pushes_overlapping_boids_apart() {
        // Two boids inside the separation radius must drive apart, and the
        // separation weight is large enough to beat cohesion at that distance.
        let mut boids = vec![
            Boid { pos: Vec3::new(-0.2, 0.0, 0.0), vel: Vec3::ZERO },
            Boid { pos: Vec3::new(0.2, 0.0, 0.0), vel: Vec3::ZERO },
        ];
        let before = (boids[0].pos - boids[1].pos).length();
        for _ in 0..5 {
            flock_step(&mut boids, &FlockParams::default());
        }
        let after = (boids[0].pos - boids[1].pos).length();
        assert!(after > before, "overlapping boids did not separate: {before} → {after}");
    }

    #[test]
    fn flocking_is_deterministic() {
        let make = || -> Vec<Boid> {
            (0..12)
                .map(|i| Boid {
                    pos: Vec3::new(i as f64, (i % 3) as f64, 0.0),
                    vel: Vec3::new(0.5, 0.0, 0.0),
                })
                .collect()
        };
        let p = FlockParams::default();
        let mut a = make();
        let mut b = make();
        for _ in 0..20 {
            flock_step(&mut a, &p);
            flock_step(&mut b, &p);
        }
        assert_eq!(a, b);
    }
}
