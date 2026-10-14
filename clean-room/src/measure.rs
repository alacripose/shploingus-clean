//! **U5's instruments: one sensor concept, a band per item, and a gate whose answer is a mass.**
//!
//! Derived from `docs/DESIGN-SPEC.md` §3's slots — **S-03** (instruments), **S-12** (the band
//! controller), **S-13** (weighing) and **FC-2.16** (the gauge wall, upgraded to a base package) —
//! through the four **gate strings** `docs/LEDGER.md` C-25 records for those rows. They are the whole
//! design, so they are quoted rather than paraphrased:
//!
//! | Slot | Gate string | What it forces here |
//! |---|---|---|
//! | S-03 | `one_sensor_concept_instruments_built_from_it_overlay_is_second_consumer` | [`Sensor`] is the **only** way to ask what a thing measures; the wall and any overlay are consumers of it, not parallel instruments. |
//! | S-12 | `one_band_controller_min_max_per_item` | [`Band`] is **per item** and carries **min and max**. A mass is not a count of an item, and a band is not a score, so [`Verdict`] has three words and no number. |
//! | S-13 | `one_weighing_gate_at_stock_boundary_only_raw_mass_number` | [`WeighingGate`] stands at one boundary and its answer is a [`Mass`]: a type with room for a number of grams and **nothing else** — no count, no item name. |
//! | FC-2.16 | `factory controller virtual gauge wall` | [`GaugeWall`] holds **many** gauges in one controller, each *virtual*: a band watching one item at one site. |
//!
//! # An item is a material, and the mass is exact
//!
//! There is no second notion of "thing you can hold": a stock counts [`MaterialId`]s, because this
//! world model has exactly one answer to *what is this made of*. The gate's number is then
//! `count × density` computed by [`Density::grams_for`] in **exact rationals** — the arithmetic F1
//! already reserves for the exact set — so *audited to the gram* is a claim about a fraction and not
//! about a float. One unit of stock is one voxel, and the voxel is **1 m** (`docs/DESIGN-SPEC.md` §5
//! scale contract), so the density's own per-cm³ unit is reached through [`UNITS_TO_CM3`].
//!
//! # Refuse, never default
//!
//! Four refusals, each a case and not a symptom: a mass handed to a band ([`RefusalKind::NotACount`]
//! — S-12's own rule), a band whose min exceeds its max, a reading no gauge watches
//! ([`RefusalKind::Unwatched`]), and **a quantity that cannot be weighed**
//! ([`RefusalKind::NotWeighable`]: a crossing that carries nothing has no mass to give, and an
//! unadmitted material has no density to give it through).

use crate::material::{Density, MaterialId, MaterialRegistry};
use crate::refusal::{Refusal, RefusalKind};

/// The cubic centimetres in one unit of stock.
///
/// Declared, not guessed: `docs/DESIGN-SPEC.md` §5's scale contract is *voxel = 1 m*, and one unit of
/// stock is one voxel. The density's own unit is g/cm³, so a count reaches grams through this.
pub const UNITS_TO_CM3: i64 = 1_000_000;

/// Where a measurement was taken — a **place**, and the kind of place it is.
///
/// The kind is not decoration. S-13's gate string puts the weighing gate *at a stock boundary*, so a
/// boundary is its own address; a count is read out of a stock. Two instruments at the same site read
/// the same world.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Site {
    /// A stock, addressed by its own position.
    Stock([i32; 3]),
    /// A stock boundary (S-13), addressed by its own position.
    Boundary([i32; 3]),
}

impl Site {
    /// The site's position, whatever kind it is.
    pub const fn position(self) -> [i32; 3] {
        match self {
            Site::Stock(p) | Site::Boundary(p) => p,
        }
    }

    /// True when both sites are the same kind at the same position.
    pub const fn same(self, other: Site) -> bool {
        match (self, other) {
            (Site::Stock(a), Site::Stock(b)) | (Site::Boundary(a), Site::Boundary(b)) => {
                a[0] == b[0] && a[1] == b[1] && a[2] == b[2]
            }
            _ => false,
        }
    }
}

/// **A raw mass number**, in grams, as an exact reduced rational — S-13's *"only raw mass number"*.
///
/// The type has no room for a count, an item name or a tick: a caller that wants to know *what*
/// weighed that much has to go back to the reading, which is where that fact belongs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Mass {
    grams_num: i64,
    grams_den: i64,
}

impl Mass {
    /// The mass of `count` units of a material, in grams, by the material's own exact density.
    ///
    /// This is the gate's arithmetic and it is the only constructor: a mass is never invented, only
    /// converted from a count through a density.
    pub fn of(density: Density, count: u64) -> Self {
        let cm3 = (count as i64).saturating_mul(UNITS_TO_CM3);
        let (grams_num, grams_den) = density.grams_for(cm3);
        Mass { grams_num, grams_den }
    }

    /// The exact reduced ratio `(numerator, denominator)`, in grams.
    pub const fn grams(self) -> (i64, i64) {
        (self.grams_num, self.grams_den)
    }

    /// The same number as a float, **for display only** — never for an audit, which is what the
    /// exact ratio is for. The tests' own readback: nothing on the shipped path converts a mass to a
    /// float.
    #[cfg(test)]
    pub fn to_f64(self) -> f64 {
        self.grams_num as f64 / self.grams_den as f64
    }
}

/// What was measured: a count of blocks of one material, or a mass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quantity {
    /// A count of units at a stock.
    Count(u64),
    /// A raw mass, which only a weighing gate at a boundary produces.
    Mass(Mass),
}

/// **The one sensor concept** (S-03): where, of what, how much, and when.
///
/// The tick is part of the *value*, not metadata beside it: a reading from tick 40 and the same
/// reading from tick 41 are different readings, and a consumer that cannot tell them apart cannot
/// tell a stock level from a stock level that has been moving.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sensor {
    site: Site,
    material: MaterialId,
    quantity: Quantity,
    tick: u64,
}

impl Sensor {
    /// A count of units of one material at a stock.
    pub const fn count(site: Site, material: MaterialId, count: u64, tick: u64) -> Self {
        Sensor { site, material, quantity: Quantity::Count(count), tick }
    }

    /// A mass at a boundary — what a [`WeighingGate`] hands on.
    pub const fn mass(boundary: [i32; 3], material: MaterialId, mass: Mass, tick: u64) -> Self {
        Sensor {
            site: Site::Boundary(boundary),
            material,
            quantity: Quantity::Mass(mass),
            tick,
        }
    }

    pub const fn site(&self) -> Site {
        self.site
    }

    pub const fn material(&self) -> MaterialId {
        self.material
    }

    pub const fn quantity(&self) -> Quantity {
        self.quantity
    }

    pub const fn tick(&self) -> u64 {
        self.tick
    }

    /// The count, if this reading is a count. A mass is not a count of anything.
    pub const fn count_of(&self) -> Option<u64> {
        match self.quantity {
            Quantity::Count(n) => Some(n),
            Quantity::Mass(_) => None,
        }
    }

    /// True when this reading came from the same site and the same tick as `other`.
    ///
    /// *Two instruments agree* and *two instruments agree because they read the same moment* are
    /// different statements, and only this tells them apart.
    pub const fn same_moment(&self, other: &Sensor) -> bool {
        self.tick == other.tick && self.site.same(other.site)
    }
}

/// A **band** for one item: a minimum and a maximum, inclusive (S-12).
///
/// Deliberately **not** `Ord`: a band is an interval, and ordering two intervals would put
/// `Ord::min`/`Ord::max` in scope beside this type's own `min`/`max`, which mean the band's own two
/// ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Band {
    min: u64,
    max: u64,
}

impl Band {
    /// A band from two inclusive ends. A band whose min exceeds its max is refused **by name**: it
    /// can never hold a reading, and silently swapping the ends would hide that.
    pub fn of(min: u64, max: u64) -> Result<Self, Refusal> {
        if min > max {
            return Err(Refusal::new(
                RefusalKind::BandInverted,
                format!("a band from {min} to {max} has no inside"),
            ));
        }
        Ok(Band { min, max })
    }

    pub const fn min(&self) -> u64 {
        self.min
    }

    pub const fn max(&self) -> u64 {
        self.max
    }

    /// Where a count falls against this band — one of three words.
    pub const fn admits(&self, count: u64) -> Verdict {
        if count < self.min {
            Verdict::Below
        } else if count > self.max {
            Verdict::Above
        } else {
            Verdict::Within
        }
    }
}

/// **A reading is a band, never a score** (S-12's gate string): three words, and no number between
/// them that a caller could threshold on its own terms.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Verdict {
    /// Below the band's minimum.
    Below,
    /// Inside the band, ends inclusive.
    Within,
    /// Above the band's maximum.
    Above,
}

impl Verdict {
    /// The recorded word a consumer matches on.
    pub const fn word(self) -> &'static str {
        match self {
            Verdict::Below => "below",
            Verdict::Within => "within",
            Verdict::Above => "above",
        }
    }
}

/// What the wall answers: the verdict **and the band it was judged against**, so a consumer can see
/// the judgement's grounds instead of a score it has to re-derive. Never a number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GaugeReading {
    sensor: Sensor,
    band: Band,
    verdict: Verdict,
}

impl GaugeReading {
    pub const fn sensor(&self) -> &Sensor {
        &self.sensor
    }

    pub const fn band(&self) -> Band {
        self.band
    }

    pub const fn verdict(&self) -> Verdict {
        self.verdict
    }

    /// True only when the count landed **inside** the band.
    pub const fn is_satisfied(&self) -> bool {
        matches!(self.verdict, Verdict::Within)
    }

    /// The count this judgement was made on, if the reading was a count at all.
    pub const fn count_of(&self) -> Option<u64> {
        self.sensor.count_of()
    }
}

/// One gauge: a band watching one item at one site. Virtual, because it is a slot in the wall and
/// not a block in the world — the wall's **private element**, fitted, replaced and read through
/// [`GaugeWall`]. A caller reads a judgement, never the gauge, and the reading carries the band it
/// was judged against.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Gauge {
    site: Site,
    material: MaterialId,
    band: Band,
}

/// **The gauge wall** (FC-2.16): many virtual gauges in **one** controller.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GaugeWall {
    gauges: Vec<Gauge>,
}

impl GaugeWall {
    pub const fn new() -> Self {
        GaugeWall { gauges: Vec::new() }
    }

    /// Fit a gauge for one item at one site, and answer the band that was there before — re-fitting is
    /// an adjustment and not a second gauge, so the wall never holds two answers to one question.
    pub fn watch(&mut self, site: Site, material: MaterialId, band: Band) -> Option<Band> {
        if let Some(existing) = self
            .gauges
            .iter_mut()
            .find(|gauge| gauge.site.same(site) && gauge.material == material)
        {
            return Some(std::mem::replace(&mut existing.band, band));
        }
        self.gauges.push(Gauge { site, material, band });
        None
    }

    /// The band watching one item at one site, if the wall carries one — the refit test's own
    /// readback: the shipped readback is [`GaugeWall::read`]'s [`GaugeReading`].
    #[cfg(test)]
    pub fn band(&self, site: Site, material: MaterialId) -> Option<Band> {
        self.gauges
            .iter()
            .find(|gauge| gauge.site.same(site) && gauge.material == material)
            .map(|gauge| gauge.band)
    }

    pub fn len(&self) -> usize {
        self.gauges.len()
    }

    pub fn is_empty(&self) -> bool {
        self.gauges.is_empty()
    }

    /// Judge one reading against the gauge watching its item at its site.
    ///
    /// A reading of a **mass** is refused by name: S-12's band is per *item*, and a mass is not a
    /// count of one. A reading no gauge watches is refused by name too — an unwatched reading is
    /// silence, and this wall does not answer questions nobody asked.
    pub fn read(&self, sensor: &Sensor) -> Result<GaugeReading, Refusal> {
        // **The kind of reading first, and the gauge after.** S-12's rule is about the reading: a
        // mass is not a count of an item whether or not the wall watches that item, so a mass is
        // refused as one before any lookup — the order the rule holds, not the order the code runs.
        let at = sensor.site.position();
        let count = sensor.count_of().ok_or_else(|| {
            Refusal::new(
                RefusalKind::NotACount,
                format!(
                    "material {} at {},{} ,{} was read as a mass; a band is per item and a mass is \
                     not a count of one",
                    sensor.material.0, at[0], at[1], at[2]
                ),
            )
        })?;
        let gauge = self
            .gauges
            .iter()
            .find(|gauge| gauge.site.same(sensor.site) && gauge.material == sensor.material)
            .ok_or_else(|| {
                Refusal::new(
                    RefusalKind::Unwatched,
                    format!(
                        "no gauge watches material {} at {},{} ,{} (wall holds {})",
                        sensor.material.0,
                        at[0],
                        at[1],
                        at[2],
                        self.gauges.len()
                    ),
                )
            })?;
        Ok(GaugeReading { sensor: *sensor, band: gauge.band, verdict: gauge.band.admits(count) })
    }
}

/// What crossed a boundary, as the gate sees it: one material, and how many units of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Crossing {
    pub material: MaterialId,
    pub count: u64,
}

/// **The weighing gate** (S-13): exactly one per boundary, and its answer is a [`Mass`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WeighingGate {
    boundary: [i32; 3],
}

impl WeighingGate {
    pub const fn at(boundary: [i32; 3]) -> Self {
        WeighingGate { boundary }
    }

    pub const fn boundary(&self) -> [i32; 3] {
        self.boundary
    }

    /// Weigh what crossed: the count through the material's own density, exactly.
    ///
    /// **A quantity that cannot be weighed is refused by name.** An unadmitted material has no
    /// density to weigh it through — the registry refuses it with its own name — and a crossing that
    /// carries nothing is [`RefusalKind::NotWeighable`]: there is no quantity there, and answering
    /// `0 g` would be a measurement this gate did not make.
    pub fn weigh(&self, crossed: &Crossing, registry: &MaterialRegistry) -> Result<Mass, Refusal> {
        if crossed.count == 0 {
            return Err(Refusal::new(
                RefusalKind::NotWeighable,
                format!(
                    "nothing crossed the boundary at {},{} ,{}: no units of material {}",
                    self.boundary[0],
                    self.boundary[1],
                    self.boundary[2],
                    crossed.material.0
                ),
            ));
        }
        let material = registry.material(crossed.material)?;
        Ok(Mass::of(material.density, crossed.count))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const QUARTZ: MaterialId = MaterialId(4);
    const STOCK: Site = Site::Stock([4, 60, -9]);

    fn band(min: u64, max: u64) -> Band {
        Band::of(min, max).expect("a band from a lower end to a higher one")
    }

    #[test]
    fn a_band_that_has_no_inside_is_refused_by_name() {
        let refused = Band::of(9, 3).unwrap_err();
        assert_eq!(refused.kind(), RefusalKind::BandInverted);
        assert_eq!(refused.kind().name(), "band_inverted");
    }

    #[test]
    fn a_count_is_judged_by_three_words_and_never_by_a_score() {
        let band = band(4, 8);
        assert_eq!(band.admits(3), Verdict::Below);
        assert_eq!(band.admits(4), Verdict::Within, "the ends are inclusive");
        assert_eq!(band.admits(8), Verdict::Within);
        assert_eq!(band.admits(9), Verdict::Above);
        assert_eq!(Verdict::Within.word(), "within");
    }

    #[test]
    fn a_mass_cannot_be_judged_against_a_band_and_is_refused_by_name() {
        let registry = MaterialRegistry::starter();
        let gate = WeighingGate::at([4, 61, -9]);
        let mass = gate
            .weigh(&Crossing { material: QUARTZ, count: 2 }, &registry)
            .expect("two units of an admitted material weigh");
        let mut wall = GaugeWall::new();
        wall.watch(STOCK, QUARTZ, band(1, 3));
        // The mass belongs at the boundary, and there is no gauge there either — but the *count*
        // refusal is what this test is about, so the wall watches the same site.
        let at_stock = Sensor::mass(STOCK.position(), QUARTZ, mass, 7);
        let refused = wall.read(&at_stock).unwrap_err();
        assert_eq!(refused.kind(), RefusalKind::NotACount);
    }

    #[test]
    fn a_reading_no_gauge_watches_is_refused_by_name() {
        let mut wall = GaugeWall::new();
        wall.watch(STOCK, QUARTZ, band(1, 3));
        let refused = wall
            .read(&Sensor::count(Site::Stock([0, 0, 0]), QUARTZ, 2, 7))
            .unwrap_err();
        assert_eq!(refused.kind(), RefusalKind::Unwatched);
        assert_eq!(refused.kind().name(), "unwatched");
    }

    #[test]
    fn the_wall_holds_many_gauges_and_refitting_one_is_an_adjustment() {
        let mut wall = GaugeWall::new();
        assert!(wall.watch(STOCK, QUARTZ, band(1, 3)).is_none());
        assert!(wall.watch(Site::Stock([0, 0, 0]), MaterialId(3), band(0, 9)).is_none());
        assert_eq!(wall.len(), 2, "one controller, two virtual gauges");
        assert_eq!(wall.watch(STOCK, QUARTZ, band(5, 6)), Some(band(1, 3)));
        assert_eq!(wall.len(), 2, "a refit adjusts the gauge it names");
        assert_eq!(wall.band(STOCK, QUARTZ), Some(band(5, 6)));
        let judged = wall
            .read(&Sensor::count(STOCK, QUARTZ, 5, 8))
            .expect("the wall watches this item at this site");
        assert!(judged.is_satisfied());
        assert_eq!(judged.band(), band(5, 6));
        assert_eq!(judged.count_of(), Some(5));
    }

    #[test]
    fn a_crossing_that_carries_nothing_cannot_be_weighed() {
        let registry = MaterialRegistry::starter();
        let gate = WeighingGate::at([0, 70, 0]);
        let refused = gate
            .weigh(&Crossing { material: QUARTZ, count: 0 }, &registry)
            .unwrap_err();
        assert_eq!(refused.kind(), RefusalKind::NotWeighable);
        assert_eq!(refused.kind().name(), "not_weighable");
        assert!(refused.detail().contains("nothing crossed"));
    }

    #[test]
    fn an_unadmitted_material_has_no_density_to_weigh_it_through() {
        let registry = MaterialRegistry::starter();
        let gate = WeighingGate::at([0, 70, 0]);
        let refused = gate
            .weigh(&Crossing { material: MaterialId(9999), count: 64 }, &registry)
            .unwrap_err();
        assert_eq!(refused.kind(), RefusalKind::UnadmittedMaterial);
    }

    #[test]
    fn the_gates_number_is_exact_and_is_a_mass_and_nothing_else() {
        let registry = MaterialRegistry::starter();
        let gate = WeighingGate::at([0, 70, 0]);
        // Quartz is 53/20 g/cm³ in the starter set; one unit is 10⁶ cm³, so 3 units are exactly
        // 3 × 10⁶ × 53/20 g = 7 950 000 g = 7.95 t.
        let mass = gate
            .weigh(&Crossing { material: QUARTZ, count: 3 }, &registry)
            .expect("three units of quartz weigh");
        assert_eq!(mass.grams(), (7_950_000, 1));
        assert_eq!(mass.to_f64(), 7_950_000.0);
        // Basalt is 3 g/cm³ exactly: 64 units are 1.92e8 g, and the fraction stays exact.
        let basalt = gate
            .weigh(&Crossing { material: MaterialId(3), count: 64 }, &registry)
            .expect("sixty-four units of basalt weigh");
        assert_eq!(basalt.grams(), (192_000_000, 1));
    }

    #[test]
    fn two_instruments_that_read_the_same_moment_are_distinguishable() {
        let a = Sensor::count(STOCK, QUARTZ, 64, 40);
        let b = Sensor::count(STOCK, QUARTZ, 12, 40);
        let c = Sensor::count(STOCK, QUARTZ, 64, 41);
        assert!(a.same_moment(&b), "different values, same site and tick");
        assert!(!a.same_moment(&c), "a different tick is a different moment");
        assert!(!a.same_moment(&Sensor::count(Site::Stock([5, 60, -9]), QUARTZ, 64, 40)));
    }
}
