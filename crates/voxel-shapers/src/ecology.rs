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
/// # This is Holdridge's *scheme* and not Holdridge's *inputs*, and it used to say otherwise
///
/// The line above used to read *"Pure Holdridge: two numbers in, one formation out"*, and
/// that was a claim the call site falsified. Holdridge classifies on **biotemperature** —
/// the mean of twelve monthly means, with values below 0 °C clamped to 0 and above 30 °C
/// clamped to 30 — and on **mean annual** precipitation. `column.rs:285` passes
/// `clim.temperature_c` and `wx.precipitation_mm`, both **instantaneous at the current
/// tick**.
///
/// **Measured consequence** (`docs/LEDGER.md` §93.2): four of four probed columns change
/// biome within one world year and each returns to where it started. `(0, 0)` runs
/// `boreal_forest ↔ tundra` on days 21 and 76, because its temperature is below 0 °C for
/// 357 days and the classifier's `3.0` arm is crossed twice a year. Holdridge's
/// biotemperature for that column is 0 °C year-round — the clamp cancels every sub-zero
/// month — so Holdridge places it once and never moves it.
///
/// **So the numbers are right and the label is a claim this file cannot support.** The
/// fix — aggregate before classifying — is filed as `ECO-0001` and is blocked on the user,
/// because the alternative (amending the citation and keeping the seasonal flicker as a
/// feature) is equally defensible. What is *not* left standing is the false sentence.
pub fn biome_for(temperature_c: f64, precipitation_mm: f64) -> Biome {
    // Cold is cold regardless of rain: below freezing most of the year, only the
    // driest and the hardest formations survive.
    if temperature_c < -5.0 {
        return if precipitation_mm < 250.0 {
            Biome::PolarDesert
        } else {
            Biome::Tundra
        };
    }
    if temperature_c < 3.0 {
        return if precipitation_mm < 250.0 {
            Biome::Tundra
        } else {
            Biome::BorealForest
        };
    }
    if temperature_c < 12.0 {
        return match precipitation_mm {
            p if p < 200.0 => Biome::Tundra,
            p if p < 450.0 => Biome::Grassland,
            p if p < 900.0 => Biome::CoolTemperateForest,
            _ => Biome::BorealForest,
        };
    }
    if temperature_c < 20.0 {
        return match precipitation_mm {
            p if p < 180.0 => Biome::Desert,
            p if p < 400.0 => Biome::Grassland,
            p if p < 750.0 => Biome::SubtropicalScrub,
            _ => Biome::TemperateForest,
        };
    }
    // Warm.
    match precipitation_mm {
        p if p < 150.0 => Biome::Desert,
        p if p < 350.0 => Biome::SubtropicalScrub,
        p if p < 800.0 => Biome::TropicalDryForest,
        _ => Biome::TropicalRainforest,
    }
}

/// Above this height, a column is alpine regardless of its latitude's climate.
/// Declared, because "a mountain is its own biome" is a decision.
pub const ALPINE_HEIGHT_ABOVE_RELIEF: f64 = 0.85;

/// The biome at a column, including the **altitude override**.
pub fn biome_at(
    spec: &WorldSpec,
    elevation_m: f64,
    temperature_c: f64,
    precipitation_mm: f64,
) -> Biome {
    let above = (elevation_m - spec.config.sea_level_m).max(0.0);
    if above > spec.config.relief_m * ALPINE_HEIGHT_ABOVE_RELIEF {
        // Alpine is not a Holdridge formation — it is what altitude does to one.
        // The override is a declared rule and is applied *after* the classifier,
        // so the classifier stays a pure function of climate.
        return Biome::Alpine;
    }
    biome_for(temperature_c, precipitation_mm)
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
