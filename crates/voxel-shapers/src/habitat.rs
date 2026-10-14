//! Habitat — the **sub-biome layer**: what terrain does to a regional biome.
//!
//! **Source.** `docs/earth_biomes_subbiomes_terrain.pdf`: *"climate sets the envelope →
//! terrain redistributes energy and water → geology/soil controls storage and access →
//! disturbance selects which vegetation persists → the resulting recurring pattern is
//! what we call a biome or sub-biome."* The same document's method (§10) puts **terrain**
//! — elevation, slope, aspect, relief, position relative to wind and water — between the
//! regional climate and the local vegetation, and its hierarchy is *broad biome →
//! regional ecoregion → local habitat or sub-biome-like patch*. This module is the third
//! rung: a **habitat** is the terrain expression a column takes *inside* its formation.
//!
//! # The rule, in one paragraph
//!
//! A column already knows its formation (C-93/ECO-0001, `ecology::biome_at`) and the
//! fields the material plane carries — `Elevation`, `Slope`, `Moisture` (C-159.10's
//! plane). The habitat is a small closed classification of those: **steep** ground sheds
//! water and soil and shows the rock under it; ground high in the relief behaves like a
//! colder belt (the PDF's *"mountain compresses climate zones"*); flat moist ground is
//! the wet expression; dry or sloping ground is the drier one; everything else is the
//! formation's regional default. Nothing here is a second climate classifier: the
//! inputs are the ones the material paths already read, so this layer costs **no new
//! field producers and no new terrain evaluations**.
//!
//! # What is deliberately not read yet, and where each door is
//!
//! Two of the PDF's terrain variables are not inputs here, because the fields that
//! would carry them have **no producer yet** in this tree's plane — the same honesty the
//! plane applies when it refuses them by name:
//!
//! * **Aspect** (slope orientation → solar exposure). Its natural carrier is the
//!   declared `Radiation` field, which the plane's own docs earmark for the sky/light
//!   pipeline. Until that producer exists, habitat reads slope magnitude only.
//! * **Drainage / flow accumulation** (hollows collect runoff). `hydrology::HydroSample`
//!   already measures it (`flow_cells`, `order`) but at a watershed radius the material
//!   path cannot afford per column; the declared `Groundwater` field is its door.
//!
//! # Stability
//!
//! Every input is a function of position and the world's seed, so a habitat does not
//! move with the tick — the same ruling C-159.8 applied to the fields themselves. The
//! material consequence is read at tick 0 (the `Climate` field carries no tick), so a
//! season cannot repaint the ground.

use crate::spec::WorldSpec;

/// A column's **local terrain expression** — the sub-biome layer, nested inside the
/// regional formation.
///
/// The set is closed and small on purpose: each class must have a material or flora
/// consequence in the shipped world, or it would be a label nobody can see.
#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub enum Habitat {
    /// The formation's regional default — no terrain override applies.
    Lowland,
    /// Flat, moist ground: the PDF's valley/floodplain expression (greener cover).
    Wet,
    /// Sloping or dry ground: the PDF's ridge expression (drier cover).
    Dry,
    /// Steep ground: shed soil, show the lithology's own rock.
    Steep,
    /// High in the relief: the PDF's vertical biome ladder, compressed into a few metres.
    Montane,
}

impl Habitat {
    pub const ALL: [Habitat; 5] = [
        Habitat::Lowland,
        Habitat::Wet,
        Habitat::Dry,
        Habitat::Steep,
        Habitat::Montane,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Habitat::Lowland => "lowland",
            Habitat::Wet => "wet",
            Habitat::Dry => "dry",
            Habitat::Steep => "steep",
            Habitat::Montane => "montane",
        }
    }
}

/// The slope at which ground is **steep**: 0.5 = 26.6°, where the substrate's own
/// soil-depth curve (Heimsath's `E/P₀`, read through `substrate`) has largely shed its
/// soil and a slope reads as a rock face rather than as a covered hillside. Declared,
/// not tuned silently: a world whose relief makes 26.6° impossible shows no `Steep`
/// rather than mislabelling its gentlest ground.
pub const STEEP_SLOPE: f64 = 0.5;

/// The height fraction of `relief_m` above sea level at which a column reads as
/// **montane**. Below the alpine override (`ecology::ALPINE_HEIGHT_ABOVE_RELIEF`, 0.85),
/// which stays the upper bound: montane is the belt between the lowlands and the alpine.
pub const MONTANE_HEIGHT_FRACTION: f64 = 0.45;

/// The moisture at or above which flat ground reads as **wet**.
pub const WET_MOISTURE: f64 = 0.55;

/// The slope at or below which wet ground can still be flat enough to collect.
pub const WET_SLOPE: f64 = 0.12;

/// The moisture at or below which ground reads as **dry**.
pub const DRY_MOISTURE: f64 = 0.35;

/// The slope at or above which ground sheds enough water to read as **dry** even when
/// the regional belt is moist.
pub const DRY_SLOPE: f64 = 0.30;

/// The sub-biome at a column, from **terrain** on top of the regional formation.
///
/// `surface_m` is the column's ground (the `Elevation` field), `slope` the gradient
/// magnitude (`Slope`), `moisture` the climate's own attenuation (`Moisture`). The
/// precedence is the PDF's causal order: a rock face is a rock face whatever the
/// climate; elevation beats local wetness; wetness beats dryness; otherwise the
/// formation's default stands.
///
/// **Pure: four numbers in, one class out, no position and no tick.** As with
/// `ecology::biome_for`, a classifier that consulted anything else would make two
/// columns with identical terrain different habitats.
pub fn habitat_of(spec: &WorldSpec, surface_m: f64, slope: f64, moisture: f64) -> Habitat {
    let above_sea = (surface_m - spec.config.sea_level_m).max(0.0);
    if slope >= STEEP_SLOPE {
        return Habitat::Steep;
    }
    if above_sea >= spec.config.relief_m * MONTANE_HEIGHT_FRACTION {
        return Habitat::Montane;
    }
    if moisture >= WET_MOISTURE && slope <= WET_SLOPE {
        return Habitat::Wet;
    }
    if moisture <= DRY_MOISTURE || slope >= DRY_SLOPE {
        return Habitat::Dry;
    }
    Habitat::Lowland
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Config;

    fn spec() -> WorldSpec {
        WorldSpec::new(2468, Config::default())
    }

    #[test]
    fn every_habitat_is_reachable_by_some_terrain() {
        // A classifier with an unreachable state is a classifier with a dead arm, and the
        // dead arm is always the one a later change assumes works — `ecology`'s own test,
        // applied to the terrain layer. The sweep is over terrain, not over one world:
        // reachability here is a property of the rule, and the world's own census is
        // `examples/habitat-census.rs`.
        let s = spec();
        let mut seen = std::collections::BTreeSet::new();
        let mut above = 0.0;
        while above < 80.0 {
            let mut slope = 0.0;
            while slope < 1.2 {
                let mut moisture = 0.0;
                while moisture <= 1.0 {
                    seen.insert(habitat_of(&s, s.config.sea_level_m + above, slope, moisture));
                    moisture += 0.02;
                }
                slope += 0.02;
            }
            above += 1.0;
        }
        for habitat in Habitat::ALL {
            assert!(seen.contains(&habitat), "{habitat:?} is unreachable by any terrain");
        }
    }

    #[test]
    fn more_moisture_never_gives_a_drier_habitat() {
        // Monotonicity on the wetness axis, below the montane belt and below the steep
        // slope so the sweep stays on one axis. Dry < Lowland < Wet.
        let s = spec();
        let rank = |h: Habitat| match h {
            Habitat::Dry => 0,
            Habitat::Lowland => 1,
            Habitat::Wet => 2,
            _ => unreachable!("the sweep stays off the steep and montane arms"),
        };
        for slope in [0.0, 0.05, 0.1] {
            let mut last = 0;
            let mut moisture = 0.0;
            while moisture <= 1.0 {
                let h = habitat_of(&s, s.config.sea_level_m, slope, moisture);
                let r = rank(h);
                assert!(r >= last, "more moisture gave a drier habitat at {moisture}");
                last = r;
                moisture += 0.01;
            }
        }
    }

    #[test]
    fn steeper_ground_never_gives_a_wetter_habitat() {
        // And the slope axis the other way: as ground steepens it may only get drier,
        // then become a rock face. Wet > Lowland > Dry > Steep is the wetness rank with
        // steep at the bottom.
        let s = spec();
        let rank = |h: Habitat| match h {
            Habitat::Steep => 0,
            Habitat::Dry => 1,
            Habitat::Lowland => 2,
            Habitat::Wet => 3,
            Habitat::Montane => unreachable!("the sweep stays below the montane belt"),
        };
        for moisture in [0.0, 0.3, 0.5, 0.7, 1.0] {
            let mut last = rank(Habitat::Wet);
            let mut slope = 0.0;
            while slope < 1.0 {
                let r = rank(habitat_of(&s, s.config.sea_level_m, slope, moisture));
                assert!(r <= last, "steeper ground got wetter at slope {slope}");
                last = r;
                slope += 0.005;
            }
        }
    }

    #[test]
    fn higher_ground_never_leaves_the_montane_belt() {
        // Elevation is the one-axis ladder: once a column is montane, every higher column
        // with the same terrain is too, until the alpine override takes over in `biome_at`
        // (which is a biome, not a habitat, and is not this function's business).
        let s = spec();
        let mut above = 0.0;
        let mut became = false;
        while above <= s.config.relief_m {
            let h = habitat_of(&s, s.config.sea_level_m + above, 0.05, 0.4);
            if h == Habitat::Montane {
                became = true;
            } else {
                assert!(!became, "the montane belt ended at {above} m above sea");
            }
            above += 0.25;
        }
        assert!(became, "no height in the relief was montane");
    }

    #[test]
    fn the_habitat_is_a_function_of_terrain_and_nothing_else() {
        let s = spec();
        assert_eq!(
            habitat_of(&s, 30.0, 0.1, 0.6),
            habitat_of(&s, 30.0, 0.1, 0.6),
        );
        // And the tick is not an argument: the same world at two ticks is the same
        // terrain, because no input here reads the clock.
        let later = s.at_tick(crate::climate::YEAR_TICKS * 5 + 3);
        assert_eq!(habitat_of(&s, 30.0, 0.1, 0.6), habitat_of(&later, 30.0, 0.1, 0.6));
    }
}
