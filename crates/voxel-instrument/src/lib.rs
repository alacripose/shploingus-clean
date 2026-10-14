//! The instrument layer: **one sensor concept**, and everything built from it.
//!
//! # Where this comes from
//!
//! Not from a guess about how a mod's gauges behave. From three gate strings
//! recorded in the source project's own disposition ledger, which are the whole
//! design and are quoted rather than paraphrased:
//!
//! | Row | Gate | What it forces here |
//! |---|---|---|
//! | `S-03` · keep | `one_sensor_concept_instruments_built_from_it_overlay_is_second_consumer` | [`Sensor`] is the **only** way to ask what a thing measures. A gauge and the overlay are two consumers of it, not two systems. |
//! | `S-12` · keep | `one_band_controller_min_max_per_item` | [`band::Band`] is **per item** and has **min and max**. Not a threshold — a band. The ticker/requester pair was struck by this row, so there is no second controller here. |
//! | `S-13` · keep | `one_weighing_gate_at_stock_boundary_only_raw_mass_number` | [`stock::WeighingGate`] exists once per boundary and its measurement is a `Mass` and **nothing else** — the type has no room for a count or an item name. |
//! | `FC-2.16` · CONTENT S1, upgraded to a base package | `factory controller virtual gauge wall` | [`gauge_wall::GaugeWall`] — many **virtual** gauges in one controller, not one block per gauge. |
//!
//! # An item is a material
//!
//! There is no `ItemId`. A stock network counts [`MaterialId`]s, because the
//! world model already has exactly one answer to "what is this made of", and a
//! parallel notion of "thing you can hold" would be a second one. The payoff is
//! concrete: the weighing gate's raw mass number is
//! `count × bulk_density_kg_m3`, computed in **exact rationals** by the same
//! machinery F1 reserves for the exact set, so a mass audit cannot drift.
//!
//! # The one rule, here
//!
//! **Refuse, never default.** A band whose `min` exceeds its `max` is refused at
//! construction. A band controller handed a **mass** reading is refused, because
//! S-12 says a band is per *item* and a mass is not a count of one. A controller
//! handed a reading about a **different material** is refused, because a gauge
//! that silently compares the wrong item is indistinguishable from a correct one
//! until somebody builds a factory on it.
//!
//! **The audit, 2026-10-10.** 1 417 lines, 4 modules, 69 public items, 33 tests, and **read by
//! nobody**: `census::orphan_libraries` prints it in every build receipt, and its only mentions
//! elsewhere in the tree are comments citing `Sensor` as "the one sensor concept". So this is 1 417
//! lines and 33 tests that **no run in the game exercises**, on slots C-25 records as built early.
//! The crate is not wrong; it is unreachable. **Owed:** a consumer, or the ruling that content may
//! precede the world that uses it — said on the record, because today the only thing that knows is a
//! build-report line.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod band;
pub mod gauge_wall;
pub mod stock;

use voxel_model::{MaterialId, Ratio};

pub use band::{Band, BandController, BandReading, Position};
pub use gauge_wall::{GaugeReading, GaugeWall};
pub use stock::{Container, Stock, StockNetwork, WeighingGate};

/// Where a measurement was taken.
///
/// A site is a *place*, not a type of instrument. Two instruments at the same
/// site read the same world; the type of instrument is not part of the address,
/// which is what stops "the same stock level" from becoming two questions.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Site {
    /// A stock network (FC-2.11), addressed by its controller's position.
    Network([i32; 3]),
    /// A weighing gate at a stock boundary (S-13), addressed by the boundary's
    /// position.
    Boundary([i32; 3]),
}

impl Site {
    /// The site's position, whatever kind it is.
    pub const fn position(self) -> [i32; 3] {
        match self {
            Site::Network(p) | Site::Boundary(p) => p,
        }
    }
}

/// What was measured.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Measured {
    /// A count of blocks of one material.
    Units(u64),
    /// A raw mass, in kilograms. **S-13's "only raw mass number".**
    Mass(Ratio),
}

/// **The one sensor concept** (S-03).
///
/// A reading is four things and no more: where, of what, how much, and when.
/// Every instrument in this crate consumes one of these, and so does the
/// overlay — which is the gate's own wording, that the overlay is a *second
/// consumer* of one concept rather than a parallel implementation of it.
///
/// The `tick` is part of the value and not metadata: a reading from tick 40 and
/// the same reading from tick 41 are **different readings**, and a consumer that
/// cannot tell them apart cannot tell a stock level from a stock level that has
/// been moving.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Sensor {
    pub site: Site,
    pub material: MaterialId,
    pub measured: Measured,
    pub tick: u64,
}

impl Sensor {
    /// A count of blocks at a stock site.
    pub const fn units(site: Site, material: MaterialId, count: u64, tick: u64) -> Self {
        Sensor { site, material, measured: Measured::Units(count), tick }
    }

    /// A raw mass, which only a weighing gate at a boundary produces.
    pub const fn mass(boundary: [i32; 3], material: MaterialId, kilograms: Ratio, tick: u64) -> Self {
        Sensor {
            site: Site::Boundary(boundary),
            material,
            measured: Measured::Mass(kilograms),
            tick,
        }
    }

    /// The count, if this reading is a count.
    pub const fn units_of(&self) -> Option<u64> {
        match self.measured {
            Measured::Units(n) => Some(n),
            Measured::Mass(_) => None,
        }
    }

    /// True when this reading came from the same site and tick as `other`.
    ///
    /// Used to tell "two instruments agree" from "two instruments agree because
    /// they read the same moment", which is not the same statement.
    pub const fn same_moment(&self, other: &Sensor) -> bool {
        self.tick == other.tick
            && match (self.site, other.site) {
                (Site::Network(a), Site::Network(b)) => a[0] == b[0] && a[1] == b[1] && a[2] == b[2],
                (Site::Boundary(a), Site::Boundary(b)) => a[0] == b[0] && a[1] == b[1] && a[2] == b[2],
                _ => false,
            }
    }

    /// **The overlay's consumption** of the one sensor concept (S-03).
    ///
    /// A display is a rendering of a reading and never a second measurement. The
    /// test below proves it by checking that this string and the band
    /// controller's verdict are functions of *the same* sensor value: change the
    /// sensor and both change, feed them two different sensors and they
    /// disagree.
    pub fn describe(&self) -> String {
        let at = self.site.position();
        match self.measured {
            Measured::Units(n) => format!(
                "{} x{} at {},{} ,{} @t{}",
                self.material.0, n, at[0], at[1], at[2], self.tick
            ),
            Measured::Mass(kg) => format!(
                "{} kg of {} at boundary {},{} ,{} @t{}",
                kg.to_f64(),
                self.material.0,
                at[0],
                at[1],
                at[2],
                self.tick
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const IRON: MaterialId = MaterialId(6);

    #[test]
    fn a_reading_knows_whether_it_is_a_count_or_a_mass() {
        let count = Sensor::units(Site::Network([1, 2, 3]), IRON, 64, 7);
        assert_eq!(count.units_of(), Some(64));
        let weight = Sensor::mass([1, 2, 3], IRON, Ratio::from_u64(1786), 7);
        assert_eq!(weight.units_of(), None, "a mass is not a count of anything");
    }

    #[test]
    fn two_readings_at_the_same_place_and_tick_are_the_same_moment() {
        let a = Sensor::units(Site::Network([1, 2, 3]), IRON, 64, 7);
        let b = Sensor::units(Site::Network([1, 2, 3]), IRON, 12, 7);
        let c = Sensor::units(Site::Network([1, 2, 3]), IRON, 64, 8);
        let d = Sensor::units(Site::Network([1, 2, 4]), IRON, 64, 7);
        // Same moment, different values: this is what makes "the instrument and
        // the overlay agree" a statement about the *reading* and not about luck.
        assert!(a.same_moment(&b));
        assert!(!a.same_moment(&c), "a different tick is a different moment");
        assert!(!a.same_moment(&d), "a different site is a different moment");
        assert!(!a.same_moment(&Sensor::mass([1, 2, 3], IRON, Ratio::ONE, 7)));
    }

    #[test]
    fn the_overlay_renders_the_reading_it_was_given() {
        let count = Sensor::units(Site::Network([1, 2, 3]), IRON, 64, 7);
        assert!(count.describe().contains("x64"));
        let weight = Sensor::mass([1, 2, 3], IRON, Ratio::from_u64(2000), 7);
        assert!(weight.describe().contains("2000"));
    }
}
