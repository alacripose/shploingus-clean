//! The band controller: **one**, with **min and max**, **per item** (`S-12`).
//!
//! Not a threshold. Three words in that gate each forbid something a reader
//! would otherwise assume, and each is enforced by a type or a refusal:
//!
//! * **"one"** — there is no second controller. `S-12` was recorded as *striking*
//!   the ticker-and-requester pair (`FC-2.8`, verdict `NO`, horizon `never`), and
//!   its survivor is `FC-2.15`, the min/max band. So this module has a controller
//!   and not a family of them, and there is no requester to go with it.
//! * **"min/max"** — a [`Band`] has two ends. A one-ended band is a band with its
//!   other end at infinity, which is available as [`Band::at_least`] and is
//!   therefore a *case* of a band rather than a different instrument.
//! * **"per item"** — a band is about a **count of one material**, so a
//!   controller handed a [`Measured::Mass`] refuses. This is the refusal that
//!   matters most: a mass is not a count of one item, so a band that accepted one
//!   would be a band whose `min` and `max` meant something else on Tuesday.
//!
//! # The band's ends are inclusive, and that is a decision
//!
//! `min` and `max` are **both inside** the band: `min <= value <= max` is
//! [`Position::Inside`]. A player setting 64 means "64 is enough", and a bound
//! that excluded its own end would make every well-chosen round number wrong by
//! one. It is pinned by a test at both ends rather than left to a comment.

use voxel_model::{MaterialId, Refusal, RefusalKind, Refused};

use crate::{Measured, Sensor};

/// Where a value sits relative to a band.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Position {
    /// Strictly below `min`.
    Below,
    /// At or between the ends. **Both ends are inside.**
    Inside,
    /// Strictly above `max`.
    Above,
}

impl Position {
    pub const fn name(self) -> &'static str {
        match self {
            Position::Below => "below",
            Position::Inside => "inside",
            Position::Above => "above",
        }
    }

    /// Whether the band is satisfied.
    pub const fn is_satisfied(self) -> bool {
        matches!(self, Position::Inside)
    }
}

/// A closed interval of counts: **min and max, both inclusive**.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Band {
    min: u64,
    max: u64,
}

impl Band {
    /// Refuse a band whose ends are the wrong way round.
    ///
    /// `min == max` is **allowed** and means "exactly this many" — a band of one
    /// value is the strictest useful setting and forbidding it would push a
    /// legitimate case onto a second instrument.
    pub const fn new(min: u64, max: u64) -> Refused<Self> {
        if min > max {
            return Err(Refusal::new(
                RefusalKind::OutOfDeclaredRange,
                "band min is above band max",
            ));
        }
        Ok(Band { min, max })
    }

    /// A band of exactly one value.
    pub const fn exactly(value: u64) -> Self {
        Band { min: value, max: value }
    }

    /// Everything at or above `min`, with no upper end.
    ///
    /// The upper end is `u64::MAX`, and it is a real upper end rather than an
    /// absent one: a count of 2⁶⁴ blocks is not reachable, so the band is
    /// unbounded **in the world** while still being a closed interval in the
    /// type. That is what lets `at_least` be a case of `Band` instead of a
    /// second concept.
    pub const fn at_least(min: u64) -> Self {
        Band { min, max: u64::MAX }
    }

    pub const fn min(&self) -> u64 {
        self.min
    }

    pub const fn max(&self) -> u64 {
        self.max
    }

    /// Where `value` sits. **Both ends are inside.**
    pub const fn position(&self, value: u64) -> Position {
        if value < self.min {
            Position::Below
        } else if value > self.max {
            Position::Above
        } else {
            Position::Inside
        }
    }

    /// How many distinct counts this band accepts, as a count. `u64::MAX` for an
    /// open-ended band, which is the honest answer — the interval really does
    /// hold that many values.
    pub const fn width(&self) -> u64 {
        self.max - self.min + 1
    }
}

/// A band watching one material — **one controller, per item** (`S-12`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BandController {
    pub material: MaterialId,
    pub band: Band,
}

/// A controller's verdict on one reading.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BandReading {
    pub material: MaterialId,
    pub band: Band,
    /// The count the band was compared against.
    pub value: u64,
    pub position: Position,
    pub tick: u64,
}

impl BandReading {
    /// Whether the sensor reading's material and site match.
    pub fn is_satisfied(&self) -> bool {
        self.position.is_satisfied()
    }
}

impl BandController {
    pub const fn new(material: MaterialId, band: Band) -> Self {
        BandController { material, band }
    }

    /// Compare one reading against the band.
    ///
    /// Two refusals, and both are the gate rather than caution:
    ///
    /// * a reading about a **different material** — a gauge that compares the
    ///   wrong item is indistinguishable from a correct one until a factory is
    ///   built on it;
    /// * a reading that is a **mass** — `S-12` says min/max **per item**, and a
    ///   mass is not a count of one item. Accepting it would silently give the
    ///   band's ends a unit they were never given.
    pub fn evaluate(&self, reading: &Sensor) -> Refused<BandReading> {
        if reading.material != self.material {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!(
                    "controller watches material {} but the reading is of material {}",
                    self.material.0, reading.material.0
                ),
            ));
        }
        let value = match reading.measured {
            Measured::Units(n) => n,
            Measured::Mass(_) => {
                return Err(Refusal::owned(
                    RefusalKind::OutOfDeclaredRange,
                    format!(
                        "controller for material {} was handed a mass reading; a band is \
                         min/max per item and a mass is not a count of one",
                        self.material.0
                    ),
                ))
            }
        };
        Ok(BandReading {
            material: self.material,
            band: self.band,
            value,
            position: self.band.position(value),
            tick: reading.tick,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Site;
    use voxel_shapers::column::materials;

    const IRON: MaterialId = materials::IGNEOUS;

    #[test]
    fn a_band_with_its_ends_the_wrong_way_round_is_refused_by_name() {
        assert!(Band::new(10, 4).is_err());
        assert_eq!(
            Band::new(10, 4).unwrap_err().kind(),
            RefusalKind::OutOfDeclaredRange
        );
        assert!(Band::new(4, 10).is_ok());
        // An exact band is a band, not an error: "exactly 64" is the strictest
        // useful setting.
        assert_eq!(Band::exactly(64).width(), 1);
    }

    #[test]
    fn both_ends_of_a_band_are_inside_it() {
        // The decision, pinned at both boundaries. A bound that excluded its own
        // end would make every round number a player picks wrong by one.
        let b = Band::new(10, 20).unwrap();
        assert_eq!(b.position(9), Position::Below);
        assert_eq!(b.position(10), Position::Inside, "min is inside");
        assert_eq!(b.position(15), Position::Inside);
        assert_eq!(b.position(20), Position::Inside, "max is inside");
        assert_eq!(b.position(21), Position::Above);
    }

    #[test]
    fn an_open_ended_band_is_a_case_of_a_band_not_a_second_concept() {
        let b = Band::at_least(64);
        assert_eq!(b.position(63), Position::Below);
        assert_eq!(b.position(64), Position::Inside);
        assert_eq!(b.position(u64::MAX), Position::Inside);
        assert!(b.width() > 0, "width must not overflow at the open end");
    }

    #[test]
    fn a_controller_refuses_a_mass_reading_because_a_band_is_per_item() {
        // S-12's third word, enforced. A mass is not a count of one item, so the
        // band's ends would have a unit they were never given.
        use voxel_model::Ratio;
        let c = BandController::new(IRON, Band::at_least(64));
        let mass = Sensor::mass([0, 0, 0], IRON, Ratio::from_u64(9999), 1);
        let r = c.evaluate(&mass).unwrap_err();
        assert!(r.detail().contains("per item"), "got {r}");
    }

    #[test]
    fn a_controller_refuses_a_reading_about_a_different_material() {
        let c = BandController::new(IRON, Band::at_least(64));
        let other = Sensor::units(Site::Network([0, 0, 0]), materials::WATER, 999, 1);
        let r = c.evaluate(&other).unwrap_err();
        assert!(r.detail().contains("watches material"), "got {r}");
    }

    #[test]
    fn a_controller_reads_the_three_positions_and_carries_the_tick() {
        let c = BandController::new(IRON, Band::new(32, 64).unwrap());
        for (count, expected) in [(0, Position::Below), (32, Position::Inside), (64, Position::Inside), (65, Position::Above)] {
            let s = Sensor::units(Site::Network([0, 0, 0]), IRON, count, 12);
            let r = c.evaluate(&s).unwrap();
            assert_eq!(r.position, expected, "count {count}");
            assert_eq!(r.tick, 12, "the verdict must carry the moment it read");
            assert_eq!(r.value, count);
        }
    }

    #[test]
    fn the_same_reading_gives_the_same_verdict_twice() {
        // Determinism at the instrument layer: no clock, no order, no cache.
        let c = BandController::new(IRON, Band::new(32, 64).unwrap());
        let s = Sensor::units(Site::Network([1, 2, 3]), IRON, 40, 5);
        assert_eq!(c.evaluate(&s).unwrap(), c.evaluate(&s).unwrap());
    }
}
