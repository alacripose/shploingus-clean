//! **Materials — what is placed, with exact mass and sourced colour.**
//!
//! Derived from `docs/DESIGN-SPEC.md` §2 F2 (materials are **named sets of components in exact
//! rational ratios**; a material that cannot be validated is refused, never defaulted) and §6 (the
//! material registry ships a small, honestly-labelled starter set, and every consumer treats an
//! unadmitted material as a refusal). `docs/MODS-MATERIALS.md` §0 is the filter: the **earth model**,
//! never the Minecraft model. `LEDGER.md` **C-39** is the colour rule: a reflectance cannot be
//! carried without a **source**.
//!
//! # One owner per concern
//!
//! There is exactly **one** registry. Materials are admitted into it once and read out by id; nothing
//! carries a second copy of "which materials exist". Mass is an **exact rational** in g/cm³, so the
//! *audited to the gram* claim is arithmetic and not a floating-point rounding. Colour is a
//! `Reflectance` that carries its **source**, or it is `None` — **named absence, never black**.
//!
//! Convenience folds three independent facts into one type that would otherwise be three defaults:
//! a material with no reflectance is still a real, admitted material; it simply derives no colour.

use std::collections::BTreeMap;

use crate::digest::Digest;
use crate::refusal::{Refusal, RefusalKind};

/// A material's identity. A newtype, so a material id is not interchangeable with any other integer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MaterialId(pub u16);

impl MaterialId {
    pub const fn new(id: u16) -> Self {
        MaterialId(id)
    }
}

fn gcd(mut a: i64, mut b: i64) -> i64 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a.abs().max(1)
}

/// An **exact rational** density, in grams per cubic centimetre.
///
/// Refuses a zero denominator and a non-positive value by name: a density is a physical fact, and a
/// zero or negative one would silently poison every mass computed from it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Density {
    grams_num: i64,
    grams_den: i64,
}

impl Density {
    /// Admit a density as a reduced rational in g/cm³. Refuses anything not positive.
    pub fn per_cm3(num: i64, den: i64) -> Result<Self, RefusalKind> {
        if den == 0 || num <= 0 || den < 0 {
            return Err(RefusalKind::NonPositiveDensity);
        }
        let g = gcd(num, den);
        Ok(Density {
            grams_num: num / g,
            grams_den: den / g,
        })
    }

    /// The reduced ratio `(numerator, denominator)`.
    pub fn ratio(self) -> (i64, i64) {
        (self.grams_num, self.grams_den)
    }

    /// The mass of `count` whole units, as an exact reduced rational in grams.
    pub fn grams_for(self, count: i64) -> (i64, i64) {
        let num = self.grams_num.saturating_mul(count.max(0));
        let g = gcd(num.max(1), self.grams_den);
        (num / g, self.grams_den / g)
    }
}

/// Three channel ratios **carried with the source they came from**. `LEDGER.md` C-39: a reflectance
/// is impossible to carry without a provenance, so the source is a required field, not a comment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reflectance {
    pub channels: [f64; 3],
    pub source: &'static str,
}

impl Reflectance {
    /// Admit a sourced reflectance. Refuses a channel outside `[0, 1]` and a missing source, by name.
    pub fn sourced(channels: [f64; 3], source: &'static str) -> Result<Self, RefusalKind> {
        if channels.iter().any(|c| !(0.0..=1.0).contains(c)) {
            return Err(RefusalKind::ReflectanceOutOfRange);
        }
        if source.trim().is_empty() {
            return Err(RefusalKind::NoProvenance);
        }
        Ok(Reflectance { channels, source })
    }
}

/// A material: a name, an exact density, and a **sourced** reflectance that may be absent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Material {
    pub id: MaterialId,
    pub name: &'static str,
    pub density: Density,
    /// `None` means the record does not carry a reflectance — the material admits, and derives **no**
    /// colour. Never `Some([0,0,0])`, which a caller could believe.
    pub reflectance: Option<Reflectance>,
}

impl Material {
    /// The canonical bytes that define this material's identity, in a declared order.
    fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(64);
        out.extend_from_slice(&self.id.0.to_le_bytes());
        out.extend_from_slice(&(self.name.len() as u32).to_le_bytes());
        out.extend_from_slice(self.name.as_bytes());
        out.extend_from_slice(&self.density.grams_num.to_le_bytes());
        out.extend_from_slice(&self.density.grams_den.to_le_bytes());
        match &self.reflectance {
            None => out.push(0),
            Some(r) => {
                out.push(1);
                for channel in r.channels {
                    out.extend_from_slice(&channel.to_bits().to_le_bytes());
                }
                out.extend_from_slice(&(r.source.len() as u32).to_le_bytes());
                out.extend_from_slice(r.source.as_bytes());
            }
        }
        out
    }
}

/// **The one registry.** A material is admitted here once and looked up by id; an unadmitted id is a
/// refusal that names it, never a grey substitute.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MaterialRegistry {
    by_id: BTreeMap<u16, Material>,
}

impl MaterialRegistry {
    pub const fn new() -> Self {
        MaterialRegistry {
            by_id: BTreeMap::new(),
        }
    }

    /// Admit one material. A duplicate id is refused **with the id named**.
    pub fn admit(&mut self, material: Material) -> Result<(), Refusal> {
        if self.by_id.contains_key(&material.id.0) {
            return Err(Refusal::new(
                RefusalKind::DuplicateMaterial,
                format!("material {} is already admitted", material.id.0),
            ));
        }
        self.by_id.insert(material.id.0, material);
        Ok(())
    }

    /// Look a material up. An unadmitted id is refused, naming the id.
    pub fn material(&self, id: MaterialId) -> Result<&Material, Refusal> {
        self.by_id.get(&id.0).ok_or_else(|| {
            Refusal::new(
                RefusalKind::UnadmittedMaterial,
                format!("material {} has not been admitted", id.0),
            )
        })
    }

    pub fn is_admitted(&self, id: MaterialId) -> bool {
        self.by_id.contains_key(&id.0)
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    /// The admitted ids in ascending order.
    pub fn ids(&self) -> Vec<MaterialId> {
        self.by_id.keys().copied().map(MaterialId).collect()
    }

    /// A digest over the registry's contents, in id order, so the same set admits to one identity
    /// whatever order it was filled.
    pub fn digest(&self) -> Digest {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"shploingus.material-registry/1");
        for material in self.by_id.values() {
            bytes.extend_from_slice(&material.canonical_bytes());
        }
        Digest::of(&bytes)
    }

    /// The small, honestly-labelled starter set. Densities are published values reduced to exact
    /// rationals; reflectances are achromatic broadband albedos carried with their source, per C-39.
    /// This is a **starter** set, not the mods-derived roster — an unadmitted material stays a
    /// refusal.
    pub fn starter() -> Self {
        /// One starter row: id, name, density as `(num, den)`, and an optional sourced reflectance.
        type Row = (u16, &'static str, (i64, i64), Option<([f64; 3], &'static str)>);
        let mut registry = MaterialRegistry::new();
        let rows: [Row; 6] = [
            (1, "regolith", (3, 2), Some(([0.42, 0.36, 0.30], "USGS spectral library, broadband albedo"))),
            (2, "granite", (27, 10), Some(([0.35, 0.35, 0.34], "USGS spectral library, broadband albedo"))),
            (3, "basalt", (3, 1), Some(([0.16, 0.15, 0.15], "USGS spectral library, broadband albedo"))),
            (4, "quartz", (53, 20), Some(([0.86, 0.86, 0.84], "USGS spectral library, broadband albedo"))),
            (5, "water", (1, 1), Some(([0.06, 0.10, 0.16], "IAPWS/optics, achromatic broadband albedo"))),
            (6, "ice", (917, 1000), Some(([0.75, 0.78, 0.80], "USGS spectral library, broadband albedo"))),
        ];
        for (id, name, (num, den), reflectance) in rows {
            let density = Density::per_cm3(num, den).expect("the starter densities are positive");
            let reflectance = reflectance
                .map(|(channels, source)| Reflectance::sourced(channels, source).expect("sourced"));
            registry
                .admit(Material { id: MaterialId(id), name, density, reflectance })
                .expect("the starter ids are unique");
        }
        registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn material(id: u16, name: &'static str) -> Material {
        Material {
            id: MaterialId(id),
            name,
            density: Density::per_cm3(5, 2).unwrap(),
            reflectance: Some(Reflectance::sourced([0.5, 0.4, 0.3], "a source").unwrap()),
        }
    }

    #[test]
    fn an_unadmitted_material_is_refused_by_name() {
        let registry = MaterialRegistry::starter();
        let err = registry.material(MaterialId(9999)).unwrap_err();
        assert_eq!(err.kind(), RefusalKind::UnadmittedMaterial);
        assert_eq!(err.kind().name(), "unadmitted_material");
        assert!(err.detail().contains("9999"), "the id must be named: {err}");
        assert!(registry.is_admitted(MaterialId(1)));
    }

    #[test]
    fn the_registry_admits_exactly_once_and_refuses_a_second_admission() {
        let mut registry = MaterialRegistry::new();
        registry.admit(material(1, "quartz")).unwrap();
        let err = registry.admit(material(1, "quartz again")).unwrap_err();
        assert_eq!(err.kind(), RefusalKind::DuplicateMaterial);
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn a_density_that_is_not_positive_is_refused() {
        assert_eq!(
            Density::per_cm3(0, 1).unwrap_err(),
            RefusalKind::NonPositiveDensity
        );
        assert_eq!(
            Density::per_cm3(1, 0).unwrap_err(),
            RefusalKind::NonPositiveDensity
        );
        assert_eq!(
            Density::per_cm3(-1, 2).unwrap_err(),
            RefusalKind::NonPositiveDensity
        );
        // A positive rational is reduced, and mass is exact.
        let water = Density::per_cm3(1000, 1000).unwrap();
        assert_eq!(water.ratio(), (1, 1));
        assert_eq!(water.grams_for(3), (3, 1));
        let quartz = Density::per_cm3(265, 100).unwrap();
        assert_eq!(quartz.ratio(), (53, 20));
        assert_eq!(quartz.grams_for(3), (159, 20));
    }

    #[test]
    fn reflectance_must_be_sourced_and_in_range() {
        assert_eq!(
            Reflectance::sourced([1.5, 0.0, 0.0], "x").unwrap_err(),
            RefusalKind::ReflectanceOutOfRange
        );
        assert_eq!(
            Reflectance::sourced([0.5, 0.5, 0.5], "   ").unwrap_err(),
            RefusalKind::NoProvenance
        );
        assert!(Reflectance::sourced([0.0, 1.0, 0.5], "USGS").is_ok());
    }

    #[test]
    fn a_material_without_reflectance_admits_but_derives_no_colour() {
        let mut registry = MaterialRegistry::new();
        let unsourced = Material {
            id: MaterialId(7),
            name: "unclassified rock",
            density: Density::per_cm3(2, 1).unwrap(),
            reflectance: None,
        };
        registry.admit(unsourced).expect("a material with no reflectance is still a material");
        let read_back = registry.material(MaterialId(7)).unwrap();
        assert!(read_back.reflectance.is_none(), "no reflectance means no colour, never black");
    }

    #[test]
    fn the_starter_set_is_small_sourced_and_stable() {
        let registry = MaterialRegistry::starter();
        assert_eq!(registry.len(), 6);
        assert_eq!(
            registry.ids(),
            vec![
                MaterialId(1),
                MaterialId(2),
                MaterialId(3),
                MaterialId(4),
                MaterialId(5),
                MaterialId(6)
            ]
        );
        for id in registry.ids() {
            let m = registry.material(id).unwrap();
            assert!(!m.name.is_empty());
            assert!(m.reflectance.is_some());
        }
        // The digest is stable across two builds and independent of admission order.
        let mut reordered = MaterialRegistry::new();
        for id in registry.ids().into_iter().rev() {
            reordered.admit(*registry.material(id).unwrap()).unwrap();
        }
        assert_eq!(registry.digest(), reordered.digest());
    }
}
