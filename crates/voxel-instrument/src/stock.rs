//! Stock: what is where, and the one gate at a boundary.
//!
//! A [`Stock`] is a count per material. A [`Container`] is a stock at a place. A
//! [`StockNetwork`] is several containers read as one addressable store — which
//! is `FC-2.11`'s *"network grid one addressable store"*, and which is the whole
//! reason a gauge reads a **total** rather than a container.
//!
//! # One gate, and why that is a constraint and not a simplification
//!
//! `S-13` says `one_weighing_gate_at_stock_boundary_only_raw_mass_number`. Two
//! halves, and both are load-bearing:
//!
//! * **one gate per boundary** — `FC-2.17`'s weighing and inspection gate is one
//!   instrument at one place, not a per-material family of meters. Nothing in
//!   [`WeighingGate`] is generic over a material.
//! * **only a raw mass number** — `WeighingGate::measure` returns a
//!   [`Mass`] and the type has no field for a count, an item, or a breakdown. The
//!   constraint is in the signature rather than in a comment, so a future caller
//!   cannot ask the gate for something it was ruled not to report.
//!
//! # Zero is an answer; an unadmitted material is not
//!
//! [`Stock::count`] answers `0` for a material it holds none of. That is not a
//! default standing in for a measurement — it is the arithmetic identity, and a
//! container genuinely holds zero of most things in the world.
//!
//! [`StockNetwork::count`] is different: it takes the registry and **refuses** a
//! material nobody admitted, because a total of zero over a material that does
//! not exist looks exactly like a total of zero over a material that does, and
//! a factory built on the second is a factory built on nothing.

use std::collections::BTreeMap;

use voxel_model::conservation::{Exact, Mass};
use voxel_model::{MaterialId, MaterialRegistry, Ratio, Refusal, RefusalKind, Refused};

/// A count of blocks per material.
///
/// A block is one cubic metre, which is what makes `count × density` a mass with
/// no unit of volume left over.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Stock {
    counts: BTreeMap<MaterialId, u64>,
}

impl Stock {
    pub fn new() -> Self {
        Self::default()
    }

    /// How many blocks of `material` this holds. **Absence is zero**, which is a
    /// fact about a container: a chest holds no lead ingots.
    pub fn count(&self, material: MaterialId) -> u64 {
        self.counts.get(&material).copied().unwrap_or(0)
    }

    /// Set a count. Zero removes the entry rather than storing one, so two
    /// stocks that hold the same things are the same value.
    pub fn set(&mut self, material: MaterialId, count: u64) {
        if count == 0 {
            self.counts.remove(&material);
        } else {
            self.counts.insert(material, count);
        }
    }

    /// Add to a count. **Refuses overflow** rather than wrapping: a chest whose
    /// contents silently became 0 because it passed 2⁶⁴ is the kind of defect a
    /// player finds and a test does not, unless the arithmetic refuses.
    pub fn add(&mut self, material: MaterialId, count: u64) -> Refused<()> {
        let next = self.count(material).checked_add(count).ok_or_else(|| {
            Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!(
                    "stock of material {} would pass 2^64: {} + {count}",
                    material.0,
                    self.count(material)
                ),
            )
        })?;
        self.set(material, next);
        Ok(())
    }

    /// Remove from a count, refusing to remove what is not there. A negative
    /// stock is not a small error; it is a stock that does not describe anything.
    pub fn remove(&mut self, material: MaterialId, count: u64) -> Refused<()> {
        let held = self.count(material);
        let next = held.checked_sub(count).ok_or_else(|| {
            Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!("cannot remove {count} of material {} from {held}", material.0),
            )
        })?;
        self.set(material, next);
        Ok(())
    }

    /// Every material held, in id order. Never insertion order (F1).
    pub fn materials(&self) -> impl Iterator<Item = MaterialId> + '_ {
        self.counts.keys().copied()
    }

    /// How many distinct materials are held.
    pub fn len(&self) -> usize {
        self.counts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.counts.is_empty()
    }

    /// The total number of blocks, whatever they are of.
    pub fn total_units(&self) -> u64 {
        self.counts.values().sum()
    }
}

/// A stock at a place.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Container {
    pub pos: [i32; 3],
    pub stock: Stock,
}

impl Container {
    pub fn new(pos: [i32; 3], stock: Stock) -> Self {
        Container { pos, stock }
    }
}

/// Several containers read as **one addressable store** (`FC-2.11`).
///
/// A `BTreeMap` and not a `HashMap`, for the reason every map in this project is
/// ordered: the aggregate must not depend on the order containers were added,
/// and a digest over an unordered iteration is a digest of the insertion order
/// wearing a world's name.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct StockNetwork {
    containers: BTreeMap<[i32; 3], Stock>,
}

impl StockNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a container. Adding one where a container already is **refuses**
    /// rather than replacing: silently dropping a chest's contents because two
    /// chests share a coordinate is a loss of matter with no record of it.
    pub fn add_container(&mut self, pos: [i32; 3], stock: Stock) -> Refused<()> {
        if self.containers.contains_key(&pos) {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!("a container is already at {}, {}, {}", pos[0], pos[1], pos[2]),
            ));
        }
        self.containers.insert(pos, stock);
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.containers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.containers.is_empty()
    }

    pub fn positions(&self) -> impl Iterator<Item = [i32; 3]> + '_ {
        self.containers.keys().copied()
    }

    pub fn container(&self, pos: [i32; 3]) -> Option<&Stock> {
        self.containers.get(&pos)
    }

    /// The total count of a material across the network.
    ///
    /// Refuses a material the registry never admitted, and refuses an aggregate
    /// that overflows. Both are the same failure the individual `Stock` refuses,
    /// one level up: a total that is wrong is worse than a total that is absent.
    pub fn count(&self, registry: &MaterialRegistry, material: MaterialId) -> Refused<u64> {
        registry.material(material)?;
        let mut total = 0u64;
        for (pos, stock) in &self.containers {
            let here = stock.count(material);
            total = total.checked_add(here).ok_or_else(|| {
                Refusal::owned(
                    RefusalKind::OutOfDeclaredRange,
                    format!(
                        "network total of material {} overflows 2^64 at container {}, {}, {}",
                        material.0, pos[0], pos[1], pos[2]
                    ),
                )
            })?;
        }
        Ok(total)
    }

    /// The whole network as one stock, refusing an overflow.
    pub fn aggregate(&self, registry: &MaterialRegistry) -> Refused<Stock> {
        let mut seen: Vec<MaterialId> = Vec::new();
        for stock in self.containers.values() {
            for m in stock.materials() {
                if !seen.contains(&m) {
                    seen.push(m);
                }
            }
        }
        seen.sort_unstable();

        let mut out = Stock::new();
        for material in seen {
            let total = self.count(registry, material)?;
            out.set(material, total);
        }
        Ok(out)
    }

    /// The exact mass of a material across the network.
    ///
    /// `count × bulk_density_kg_m3`, in exact rationals through
    /// [`voxel_model::conservation`] — the same machinery F1 reserves for the
    /// exact set, so this number cannot drift and cannot be a float that is
    /// almost right.
    pub fn mass_of(&self, registry: &MaterialRegistry, material: MaterialId) -> Refused<Mass> {
        let material_ref = registry.material(material)?;
        let count = self.count(registry, material)?;
        Ok(mass_of_count(count, material_ref.bulk_density_kg_m3()))
    }

    /// The exact mass of everything in the network.
    pub fn total_mass(&self, registry: &MaterialRegistry) -> Refused<Mass> {
        let aggregate = self.aggregate(registry)?;
        let mut total = Mass::from_exact(Exact::new(Ratio::ZERO));
        for material in aggregate.materials() {
            total = total + self.mass_of(registry, material)?;
        }
        Ok(total)
    }
}

/// The mass of `count` blocks of a material of the given density.
fn mass_of_count(count: u64, density_kg_m3: Ratio) -> Mass {
    Mass::from_exact(Exact::new(Ratio::from_u64(count)) * Exact::new(density_kg_m3))
}

/// **One weighing gate at a stock boundary** (`S-13`).
///
/// It reports a raw mass and nothing else. It has no material parameter on
/// `measure`, no count, and no breakdown — because the rulings in this project
/// are gates, and a gate that a later caller can walk around by asking a
/// different question is not a gate.
///
/// # It cannot report a net, and that is a finding
///
/// A gate that weighs both directions obviously *should* be able to report the
/// net. It cannot, and the reason is one layer down: [`voxel_model`] has no exact
/// rational **subtraction** on a mass. `Ratio` has `Add`, `Mul` and its own
/// `checked_sub`, but `Mass::checked_sub` refuses unconditionally with *"exact
/// rational subtraction is not implemented; do not approximate it"*.
///
/// So a gate reports its two gross masses and there is no `net` method — not
/// because netting is unimportant, but because the only way to produce one today
/// would be to cross the exact/approximate boundary to get it, and the exact set
/// is the one place this project refuses to approximate.
///
/// **This was ruled, not overlooked.** `Ratio::checked_sub` was written, which
/// made the mass-level operation a two-line delegation, and the delegation was
/// then reverted: the user chose the pinned absence over a green `net`. The pin
/// below measures the gap rather than assuming it, so the day the ruling goes the
/// other way the test fails and the method gets written.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct WeighingGate {
    boundary: [i32; 3],
}

impl WeighingGate {
    /// The one gate at a boundary.
    pub const fn new(boundary: [i32; 3]) -> Self {
        WeighingGate { boundary }
    }

    pub const fn boundary(&self) -> [i32; 3] {
        self.boundary
    }

    /// The raw mass of what crossed, in one direction.
    ///
    /// Direction is the **caller's** to name, because the gate does not have an
    /// inside: it is a boundary, and which side is inward is a fact about the
    /// building, not about the instrument. Call it twice with the two stocks to
    /// get both.
    pub fn measure(&self, crossed: &Stock, registry: &MaterialRegistry) -> Refused<Mass> {
        let mut total = Mass::from_exact(Exact::new(Ratio::ZERO));
        for material in crossed.materials() {
            let m = registry.material(material)?;
            total = total + mass_of_count(crossed.count(material), m.bulk_density_kg_m3());
        }
        Ok(total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_shapers::materials;

    // The starter registry lives in `voxel-shapers`. It is used here rather than
    // rebuilt, so the instrument layer cannot invent a material the world does
    // not have.
    fn registry() -> MaterialRegistry {
        voxel_shapers::materials::registry().unwrap()
    }

    #[test]
    fn a_stock_holds_counts_and_absence_is_zero() {
        let mut s = Stock::new();
        assert_eq!(s.count(MaterialId(6)), 0);
        s.set(MaterialId(6), 4);
        assert_eq!(s.count(MaterialId(6)), 4);
        s.set(MaterialId(6), 0);
        assert!(s.is_empty(), "a zero count is not a held material");
    }

    #[test]
    fn adding_and_removing_refuse_the_impossible_rather_than_wrapping() {
        let mut s = Stock::new();
        s.add(MaterialId(6), 10).unwrap();
        assert_eq!(s.count(MaterialId(6)), 10);

        let r = s.remove(MaterialId(6), 11).unwrap_err();
        assert!(r.detail().contains("cannot remove"), "got {r}");
        assert_eq!(s.count(MaterialId(6)), 10, "a refused removal changed nothing");

        s.set(MaterialId(6), u64::MAX);
        let r = s.add(MaterialId(6), 1).unwrap_err();
        assert!(r.detail().contains("2^64"), "got {r}");
        assert_eq!(s.count(MaterialId(6)), u64::MAX);
    }

    #[test]
    fn the_network_total_is_over_every_container() {
        // The fact the whole gauge system rests on: a gauge reads the network,
        // not the chest it happens to be nailed to.
        let reg = registry();
        let mut net = StockNetwork::new();
        let mut a = Stock::new();
        a.set(materials::IGNEOUS, 62);
        let mut b = Stock::new();
        b.set(materials::IGNEOUS, 2);
        net.add_container([0, 0, 0], a).unwrap();
        net.add_container([40, 0, 0], b).unwrap();
        assert_eq!(net.count(&reg, materials::IGNEOUS).unwrap(), 64);
        assert_eq!(net.container([0, 0, 0]).unwrap().count(materials::IGNEOUS), 62);
    }

    #[test]
    fn two_containers_at_one_place_are_refused() {
        let mut net = StockNetwork::new();
        net.add_container([1, 1, 1], Stock::new()).unwrap();
        let r = net.add_container([1, 1, 1], Stock::new()).unwrap_err();
        assert!(r.detail().contains("already"), "got {r}");
    }

    #[test]
    fn the_aggregate_does_not_depend_on_the_order_containers_were_added() {
        let reg = registry();
        let mut forward = StockNetwork::new();
        let mut backward = StockNetwork::new();
        let entries: Vec<([i32; 3], MaterialId, u64)> = vec![
            ([0, 0, 0], materials::IGNEOUS, 7),
            ([1, 0, 0], materials::REGOLITH, 3),
            ([2, 0, 0], materials::IGNEOUS, 11),
            ([3, 0, 0], materials::BEDROCK, 2),
        ];
        for (pos, m, n) in &entries {
            let mut s = Stock::new();
            s.set(*m, *n);
            forward.add_container(*pos, s).unwrap();
        }
        for (pos, m, n) in entries.iter().rev() {
            let mut s = Stock::new();
            s.set(*m, *n);
            backward.add_container(*pos, s).unwrap();
        }
        assert_eq!(
            forward.aggregate(&reg).unwrap(),
            backward.aggregate(&reg).unwrap()
        );
    }

    #[test]
    fn an_unadmitted_material_is_refused_by_the_registry_not_counted_as_zero() {
        // The difference between "this network holds none of it" and "this
        // network cannot hold it" — and the second must not look like the first.
        let reg = registry();
        let net = StockNetwork::new();
        let r = net.count(&reg, MaterialId(500)).unwrap_err();
        assert_eq!(r.kind(), voxel_model::RefusalKind::UnadmittedMaterial);
    }

    #[test]
    fn an_overflowing_network_total_is_refused_and_names_the_container() {
        let reg = registry();
        let mut net = StockNetwork::new();
        for (i, pos) in [[0, 0, 0], [1, 0, 0], [2, 0, 0]].into_iter().enumerate() {
            let mut s = Stock::new();
            s.set(materials::IGNEOUS, if i == 2 { u64::MAX } else { u64::MAX / 2 });
            net.add_container(pos, s).unwrap();
        }
        let r = net.count(&reg, materials::IGNEOUS).unwrap_err();
        assert!(r.detail().contains("overflows"), "got {r}");
        assert!(r.detail().contains("2, 0, 0"), "the failing container should be named: {r}");
    }

    #[test]
    fn mass_is_exact_and_is_count_times_the_admitted_density() {
        let reg = registry();
        let mut net = StockNetwork::new();
        let mut s = Stock::new();
        s.set(materials::IGNEOUS, 4);
        net.add_container([0, 0, 0], s).unwrap();

        let density = reg.material(materials::IGNEOUS).unwrap().bulk_density_kg_m3();
        let mass = net.mass_of(&reg, materials::IGNEOUS).unwrap();
        assert_eq!(
            mass.exact().ratio(),
            Ratio::from_u64(4) * density,
            "mass is not exactly count x density"
        );
        // Four cubic metres of 2900 kg/m³ rock.
        assert!((mass.exact().ratio().to_f64() - 11_600.0).abs() < 1e-9);
    }

    #[test]
    fn the_gate_reports_one_raw_mass_per_direction() {
        let reg = registry();
        let gate = WeighingGate::new([8, 0, 0]);
        let mut inward = Stock::new();
        inward.set(materials::IGNEOUS, 3);
        let mut outward = Stock::new();
        outward.set(materials::IGNEOUS, 1);

        // 3 m³ of 2900 kg/m³.
        let in_mass = gate.measure(&inward, &reg).unwrap();
        assert!((in_mass.exact().ratio().to_f64() - 8700.0).abs() < 1e-9);
        // 1 m³ the other way, and the gate is not asked for a difference.
        let out_mass = gate.measure(&outward, &reg).unwrap();
        assert!((out_mass.exact().ratio().to_f64() - 2900.0).abs() < 1e-9);
    }

    #[test]
    fn the_model_has_no_exact_subtraction_so_a_net_mass_cannot_be_reported() {
        // This is the gap that stopped `WeighingGate::net` being written, and it
        // is *measured* here rather than assumed. When exact subtraction lands,
        // this test fails and the net becomes possible — which is the point of
        // pinning an absence with a test instead of a comment.
        let big = Mass::from_exact(Exact::new(Ratio::from_u64(10)));
        let small = Mass::from_exact(Exact::new(Ratio::from_u64(1)));
        let r = big.checked_sub(small).unwrap_err();
        assert!(
            r.detail().contains("not implemented"),
            "checked_sub changed behaviour: {r}"
        );
    }

    #[test]
    fn a_gate_weighs_mixed_matter_as_one_number() {
        // "Only a raw mass number" means the gate does not report a breakdown —
        // and this checks the *sum* is right, which is all it does report.
        let reg = registry();
        let gate = WeighingGate::new([0, 0, 0]);
        let mut s = Stock::new();
        s.set(materials::IGNEOUS, 1);
        s.set(materials::WATER, 1);
        let total = gate.measure(&s, &reg).unwrap();
        let expected = 2900.0 + 1000.0;
        assert!(
            (total.exact().ratio().to_f64() - expected).abs() < 1e-9,
            "got {}",
            total.exact().ratio().to_f64()
        );
    }

    #[test]
    fn total_mass_is_the_mass_of_everything() {
        let reg = registry();
        let mut net = StockNetwork::new();
        let mut a = Stock::new();
        a.set(materials::IGNEOUS, 2);
        let mut b = Stock::new();
        b.set(materials::WATER, 3);
        net.add_container([0, 0, 0], a).unwrap();
        net.add_container([9, 0, 0], b).unwrap();
        let total = net.total_mass(&reg).unwrap();
        assert!((total.exact().ratio().to_f64() - (5800.0 + 3000.0)).abs() < 1e-9);
    }
}
