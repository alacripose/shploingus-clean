//! The gauge wall: **many virtual gauges in one controller** (`FC-2.16`).
//!
//! The gate string is `factory controller virtual gauge wall`, and the load-bearing
//! word is **virtual**. A wall is *one* block whose gauges are entries in a map,
//! not one block per gauge. That is not a memory optimisation; it is a statement
//! about where the reading comes from. Every gauge on the wall consumes the same
//! [`Sensor`] concept that [`crate::band`] consumes, so a wall cannot become a
//! second measuring system that agrees with the band controller by coincidence.
//!
//! # A gauge's address is the thing it reads, not its slot on the wall
//!
//! A [`GaugeId`] is a `(site, material)` pair — the sensor the gauge watches.
//! There is deliberately no gauge *index*: an index would let the same sensor
//! carry two gauges, and two gauges on one sensor are two answers to one
//! question. Adding a second gauge where one already is **refuses**, for the
//! same reason two containers at one coordinate refuse in [`crate::stock`].
//!
//! # The four defaults this module refuses to take
//!
//! * **A reading no gauge watches is not a satisfied reading.** A wall that
//!   reported `true` for a sensor it does not watch would be indistinguishable
//!   from one that watches it and finds it in band.
//! * **An unmeasured gauge is not a satisfied gauge.** [`GaugeWall::all_satisfied`]
//!   refuses when a gauge on the wall has no reading in the sweep it was given.
//!   This is `S-13`'s asymmetry: an unreadable instrument is a *finding*, and the
//!   honest report is "I was not told", never "all clear".
//! * **A missing gauge is not a zero.** [`GaugeWall::remove`] refuses to remove a
//!   gauge that is not there, because a typo'd address would otherwise read as a
//!   successful removal.
//! * **A band gauge cannot watch a boundary at all.** `S-12`'s band is min/max
//!   **per item**, and `S-13`'s boundary reports **only a raw mass** — so a gauge
//!   placed there would be an instrument whose every reading is refused. It is
//!   refused **at construction** instead of on every read. *This one was found by
//!   a test, not reasoned out first:* the mass test below was written expecting
//!   the refusal to arrive from [`crate::band`] one layer down, and the routing
//!   refused it earlier, because `Sensor::mass` addresses a [`Site::Boundary`]
//!   and the gauge had been added at a [`Site::Network`]. The misconfiguration
//!   was detectable at `add`, which is where it is now caught.

use std::collections::{BTreeMap, BTreeSet};

use voxel_model::{MaterialId, Refusal, RefusalKind, RefusalSet, Refused};

use crate::band::{Band, BandController, BandReading, Position};
use crate::{Sensor, Site};

/// **Which sensor a virtual gauge watches.**
///
/// A place and a material, and nothing else. The type is `Ord` because a wall's
/// gauges are held in a `BTreeMap`: the wall must not depend on the order gauges
/// were added, and a digest over an unordered iteration is a digest of the
/// insertion history wearing a controller's name.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct GaugeId {
    pub site: Site,
    pub material: MaterialId,
}

impl GaugeId {
    pub const fn new(site: Site, material: MaterialId) -> Self {
        GaugeId { site, material }
    }
}

/// A controller's verdict on one reading, **and which gauge reached it**.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct GaugeReading {
    /// The gauge that spoke: the sensor it watches.
    pub gauge: GaugeId,
    /// The band controller's verdict, unchanged from [`BandController`].
    pub verdict: BandReading,
}

impl GaugeReading {
    pub fn is_satisfied(&self) -> bool {
        self.verdict.is_satisfied()
    }

    pub fn position(&self) -> Position {
        self.verdict.position
    }
}

/// **One controller, many virtual gauges** (`FC-2.16`).
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct GaugeWall {
    at: [i32; 3],
    gauges: BTreeMap<GaugeId, Band>,
}

impl GaugeWall {
    /// The wall's own position. It is a *controller*, so its address is a place
    /// in the world even though the gauges it holds are not blocks.
    pub const fn new(at: [i32; 3]) -> Self {
        GaugeWall { at, gauges: BTreeMap::new() }
    }

    pub const fn at(&self) -> [i32; 3] {
        self.at
    }

    /// Put a gauge on the wall, watching `site` for `material`.
    ///
    /// Two refusals, and the first is the interesting one:
    ///
    /// * a **boundary** site — `S-12`'s band is min/max per *item* and `S-13`'s
    ///   boundary reports only a raw *mass*, so a gauge placed there would refuse
    ///   every reading it ever took. Catching it here means an impossible
    ///   instrument cannot be built, rather than being built and then never
    ///   reading;
    /// * a sensor that **already has a gauge** — replacing a band would silently
    ///   change what the factory is asking for, and the wall would look the same
    ///   before and after.
    pub fn add(&mut self, site: Site, material: MaterialId, band: Band) -> Refused<()> {
        if let Site::Boundary(p) = site {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!(
                    "a band is min/max per item (S-12) and the boundary at {}, {}, {} reports \
                     only a raw mass (S-13): a band gauge cannot watch a boundary",
                    p[0], p[1], p[2]
                ),
            ));
        }
        let id = GaugeId::new(site, material);
        if self.gauges.contains_key(&id) {
            let p = site.position();
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!(
                    "a gauge already watches material {} at {}, {}, {}; a second gauge on one \
                     sensor is a second answer to one question",
                    material.0, p[0], p[1], p[2]
                ),
            ));
        }
        self.gauges.insert(id, band);
        Ok(())
    }

    /// Take a gauge off the wall, returning the band it was watching with.
    ///
    /// Refuses a gauge that is not there: a removal that silently did nothing
    /// makes a typo indistinguishable from a success.
    pub fn remove(&mut self, site: Site, material: MaterialId) -> Refused<Band> {
        let id = GaugeId::new(site, material);
        self.gauges.remove(&id).ok_or_else(|| {
            let p = site.position();
            Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!(
                    "no gauge watches material {} at {}, {}, {}",
                    material.0, p[0], p[1], p[2]
                ),
            )
        })
    }

    pub fn len(&self) -> usize {
        self.gauges.len()
    }

    pub fn is_empty(&self) -> bool {
        self.gauges.is_empty()
    }

    /// Every gauge, in **id order** — never insertion order.
    pub fn gauges(&self) -> impl Iterator<Item = (GaugeId, Band)> + '_ {
        self.gauges.iter().map(|(id, band)| (*id, *band))
    }

    /// The band watching one sensor, if the wall has one.
    pub fn band(&self, site: Site, material: MaterialId) -> Option<Band> {
        self.gauges.get(&GaugeId::new(site, material)).copied()
    }

    /// Compare one reading against the gauge that watches its sensor.
    ///
    /// Refuses a reading **no gauge watches**, rather than answering `false`.
    /// "Not in band" and "not mine" are different facts, and a wall that merged
    /// them would let a factory be built on a sensor nothing is reading.
    ///
    /// The routing is by `(site, material)`, which is why a **mass** reading
    /// still gets refused one layer down rather than being filtered out here:
    /// it addresses a real gauge, and then that gauge's band says, per `S-12`,
    /// that a mass is not a count of one item.
    pub fn evaluate(&self, reading: &Sensor) -> Refused<GaugeReading> {
        let gauge = GaugeId::new(reading.site, reading.material);
        let band = self.gauges.get(&gauge).ok_or_else(|| {
            let p = reading.site.position();
            Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!(
                    "no gauge watches material {} at {}, {}, {}; a reading the wall does not \
                     watch is not a satisfied reading",
                    reading.material.0, p[0], p[1], p[2]
                ),
            )
        })?;
        let verdict = BandController::new(gauge.material, *band).evaluate(reading)?;
        Ok(GaugeReading { gauge, verdict })
    }

    /// Compare many readings, reporting **every** refusal rather than the first
    /// (`RefusalSet`'s rule), and the readings that were fine.
    ///
    /// This is the difference from calling [`GaugeWall::evaluate`] in a loop: a
    /// sweep over a hundred sensors does not cost a hundred round trips to find
    /// out what is misconfigured.
    pub fn sweep(&self, readings: &[Sensor]) -> Result<Vec<GaugeReading>, RefusalSet> {
        let mut out = Vec::with_capacity(readings.len());
        let mut refusals = RefusalSet::new();
        for reading in readings {
            match self.evaluate(reading) {
                Ok(g) => out.push(g),
                Err(e) => {
                    refusals.push(e);
                }
            }
        }
        refusals.into_result(out)
    }

    /// Whether **every gauge on the wall** is satisfied by this sweep.
    ///
    /// Refuses, naming the gauge, when a gauge on the wall has no reading here.
    /// An instrument that was not read has not reported "in band" — it has not
    /// reported, and a controller that turns that into `true` is the exact
    /// failure this project's one rule exists to prevent. The wall is this
    /// method's only consumer of that distinction; `evaluate` cannot see it,
    /// because one reading says nothing about the gauges that did not speak.
    pub fn all_satisfied(&self, readings: &[Sensor]) -> Refused<bool> {
        let mut seen: BTreeSet<GaugeId> = BTreeSet::new();
        let mut all = true;
        for reading in readings {
            let gauge_reading = self.evaluate(reading)?;
            seen.insert(gauge_reading.gauge);
            all &= gauge_reading.is_satisfied();
        }
        if let Some(missing) = self.gauges.keys().find(|id| !seen.contains(id)) {
            let p = missing.site.position();
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!(
                    "gauge on material {} at {}, {}, {} has no reading in this sweep; an \
                     unmeasured gauge is not a satisfied one",
                    missing.material.0, p[0], p[1], p[2]
                ),
            ));
        }
        Ok(all)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Measured;
    use voxel_model::{MaterialRegistry, Ratio};
    use voxel_shapers::materials;

    const NETWORK: Site = Site::Network([0, 0, 0]);
    const OTHER: Site = Site::Network([40, 0, 0]);

    fn registry() -> MaterialRegistry {
        materials::registry().unwrap()
    }

    fn count(site: Site, material: MaterialId, n: u64) -> Sensor {
        Sensor::units(site, material, n, 1)
    }

    #[test]
    fn one_controller_holds_many_gauges_and_reads_each_by_its_sensor() {
        // The word "virtual", measured: two gauges, one block.
        let mut wall = GaugeWall::new([8, 0, 0]);
        wall.add(NETWORK, materials::IGNEOUS, Band::at_least(64)).unwrap();
        wall.add(OTHER, materials::WATER, Band::at_least(16)).unwrap();
        assert_eq!(wall.len(), 2, "two gauges, one controller");
        assert_eq!(wall.at(), [8, 0, 0]);

        let rock = wall.evaluate(&count(NETWORK, materials::IGNEOUS, 64)).unwrap();
        assert!(rock.is_satisfied());
        assert_eq!(rock.gauge.site, NETWORK);

        let water = wall.evaluate(&count(OTHER, materials::WATER, 15)).unwrap();
        assert_eq!(water.position(), Position::Below);
        assert_eq!(water.gauge.material, materials::WATER);
    }

    #[test]
    fn two_gauges_on_one_sensor_are_refused_rather_than_replacing_the_band() {
        let mut wall = GaugeWall::new([0, 0, 0]);
        wall.add(NETWORK, materials::IGNEOUS, Band::at_least(64)).unwrap();
        let r = wall
            .add(NETWORK, materials::IGNEOUS, Band::at_least(1))
            .unwrap_err();
        assert!(r.detail().contains("already watches"), "got {r}");
        // The refusal changed nothing: the first band is still the one in force.
        assert_eq!(wall.band(NETWORK, materials::IGNEOUS).unwrap().min(), 64);
        assert_eq!(wall.len(), 1);
    }

    #[test]
    fn removing_a_gauge_that_is_not_there_is_refused_rather_than_silent() {
        let mut wall = GaugeWall::new([0, 0, 0]);
        let r = wall.remove(NETWORK, materials::IGNEOUS).unwrap_err();
        assert!(r.detail().contains("no gauge watches"), "got {r}");
    }

    #[test]
    fn a_reading_no_gauge_watches_is_refused_not_answered_false() {
        // "not in band" and "not mine" are different facts.
        let mut wall = GaugeWall::new([0, 0, 0]);
        wall.add(NETWORK, materials::IGNEOUS, Band::at_least(64)).unwrap();
        let r = wall
            .evaluate(&count(NETWORK, materials::WATER, 999))
            .unwrap_err();
        assert!(r.detail().contains("no gauge watches"), "got {r}");
        assert!(r.detail().contains("0, 0, 0"), "the site should be named: {r}");

        // Same material, a different place: still not this wall's sensor.
        let r = wall.evaluate(&count(OTHER, materials::IGNEOUS, 999)).unwrap_err();
        assert!(r.detail().contains("40, 0, 0"), "got {r}");
    }

    #[test]
    fn a_band_gauge_cannot_watch_a_boundary_because_a_boundary_reports_only_mass() {
        // Found by measurement: `Sensor::mass` addresses a Boundary, so a mass
        // could never reach a Network gauge. The refusal belongs at construction.
        let mut wall = GaugeWall::new([8, 0, 0]);
        let r = wall
            .add(
                Site::Boundary([8, 0, 0]),
                materials::IGNEOUS,
                Band::at_least(64),
            )
            .unwrap_err();
        assert!(r.detail().contains("per item"), "got {r}");
        assert!(r.detail().contains("raw mass"), "got {r}");
        assert_eq!(wall.len(), 0, "a refused gauge is not on the wall");

        // And the mass reading itself is refused by routing, naming the site.
        let mass = Sensor::mass([8, 0, 0], materials::IGNEOUS, Ratio::from_u64(9999), 1);
        let r = wall.evaluate(&mass).unwrap_err();
        assert!(r.detail().contains("no gauge watches"), "got {r}");
    }

    #[test]
    fn a_mass_reaching_a_watched_sensor_is_still_refused_one_layer_down() {
        // The caller can name a Network site and hand over a mass anyway — the
        // fields are public, so this is reachable. The routing does not launder
        // it: the band refuses it, on S-12's own words.
        let mut wall = GaugeWall::new([0, 0, 0]);
        wall.add(NETWORK, materials::IGNEOUS, Band::at_least(64)).unwrap();
        let mass = Sensor {
            site: NETWORK,
            material: materials::IGNEOUS,
            measured: Measured::Mass(Ratio::from_u64(9999)),
            tick: 1,
        };
        let r = wall.evaluate(&mass).unwrap_err();
        assert!(r.detail().contains("per item"), "got {r}");
    }

    #[test]
    fn a_sweep_reports_every_refusal_rather_than_the_first() {
        let mut wall = GaugeWall::new([0, 0, 0]);
        wall.add(NETWORK, materials::IGNEOUS, Band::at_least(64)).unwrap();
        let readings = [
            count(NETWORK, materials::IGNEOUS, 64),   // fine
            count(NETWORK, materials::WATER, 1),      // no such gauge
            Sensor::mass([0, 0, 0], materials::IGNEOUS, Ratio::from_u64(1), 1), // a boundary
        ];
        let refusals = wall.sweep(&readings).unwrap_err();
        assert_eq!(refusals.len(), 2, "both bad readings named: {refusals}");
    }

    #[test]
    fn the_wall_refuses_to_call_an_unmeasured_gauge_satisfied() {
        // The asymmetry: the wall knows a gauge is dark; a single reading cannot.
        let mut wall = GaugeWall::new([0, 0, 0]);
        wall.add(NETWORK, materials::IGNEOUS, Band::at_least(64)).unwrap();
        wall.add(OTHER, materials::WATER, Band::at_least(16)).unwrap();

        // Only the rock gauge reported, and it is comfortably in band.
        let r = wall
            .all_satisfied(&[count(NETWORK, materials::IGNEOUS, 64)])
            .unwrap_err();
        assert!(r.detail().contains("no reading in this sweep"), "got {r}");
        assert!(r.detail().contains("40, 0, 0"), "the dark gauge should be named: {r}");
    }

    #[test]
    fn the_wall_is_satisfied_only_when_every_gauge_is() {
        let mut wall = GaugeWall::new([0, 0, 0]);
        wall.add(NETWORK, materials::IGNEOUS, Band::new(32, 64).unwrap()).unwrap();
        wall.add(OTHER, materials::WATER, Band::new(1, 8).unwrap()).unwrap();

        let in_band = [
            count(NETWORK, materials::IGNEOUS, 64),
            count(OTHER, materials::WATER, 1),
        ];
        assert!(wall.all_satisfied(&in_band).unwrap());

        let one_short = [
            count(NETWORK, materials::IGNEOUS, 31),
            count(OTHER, materials::WATER, 1),
        ];
        assert!(!wall.all_satisfied(&one_short).unwrap(), "one gauge below is not all");
    }

    #[test]
    fn the_wall_iterates_gauges_in_id_order_not_insertion_order() {
        // F1's ordering rule, at the instrument layer: the wall's contents do not
        // remember the order they were put in.
        let mut forward = GaugeWall::new([0, 0, 0]);
        let mut backward = GaugeWall::new([0, 0, 0]);
        let entries = [
            (NETWORK, materials::IGNEOUS, 64u64),
            (NETWORK, materials::WATER, 8),
            (OTHER, materials::BEDROCK, 2),
        ];
        for (site, m, n) in entries {
            forward.add(site, m, Band::at_least(n)).unwrap();
        }
        for (site, m, n) in entries.iter().rev() {
            backward.add(*site, *m, Band::at_least(*n)).unwrap();
        }
        assert_eq!(
            forward.gauges().collect::<Vec<_>>(),
            backward.gauges().collect::<Vec<_>>()
        );
    }

    #[test]
    fn an_unadmitted_material_still_has_to_be_admitted_somewhere() {
        // The wall does not check the registry itself — an instrument is not the
        // authority on what exists — so this pins the fact that a gauge *can* be
        // put on a sensor whose material the world never admitted, which is why
        // `stock` (and not this module) is where the registry check lives.
        let reg = registry();
        let mut wall = GaugeWall::new([0, 0, 0]);
        wall.add(NETWORK, MaterialId(500), Band::at_least(1)).unwrap();
        assert!(reg.material(MaterialId(500)).is_err());
    }
}
