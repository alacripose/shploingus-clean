//! Materials: a named set of components in **exact rational ratios summing to 1**.
//!
//! This is the F3 ruling, and it is the strongest thing the record contains. A
//! material's mass and volume *derive arithmetically* from its components, so
//! conservation of mass across a transformation is **provable by construction**
//! rather than audited after the fact.
//!
//! Two consequences the record names explicitly and this module implements:
//!
//! 1. **Exact rationals, never floats.** `0.1 + 0.2 != 0.3` in binary floating
//!    point, and a mass audit that drifts is worse than no audit. A decimal mass
//!    in a file is a **load error**, not a rounding.
//! 2. **The registry is bounded by criterion, not by count.** A row is admitted
//!    by evidence and review state. A missing row is a **visible blocker**, never
//!    an implicit zero — which is why [`MaterialRegistry::material`] refuses with
//!    [`RefusalKind::UnadmittedMaterial`] rather than returning a default.

use crate::refusal::{Refusal, RefusalKind, RefusalSet, Refused};
use core::fmt;
use std::collections::BTreeMap;

/// An exact non-negative rational, always fully reduced.
///
/// Deliberately not `Ord`-friendly for arbitrary use: the operations that matter
/// are `+`, `*`, and `== 1`, and each is checked. Overflow saturates and is
/// caught by the equality tests rather than wrapping into a plausible value.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Ratio {
    num: u64,
    den: u64,
}

impl Default for Ratio {
    /// Zero. **Deliberately not derived**: the derived `Default` would be
    /// `0/0`, which is not a ratio at all and would make `Ratio::default()` a
    /// denominator-zero bug waiting for a caller.
    fn default() -> Self {
        Ratio::ZERO
    }
}

impl Ratio {
    /// Zero. The only ratio that is legal in a *difference* and forbidden in a
    /// component set (a zero-ratio component is a no-op pretending to be a part).
    pub const ZERO: Ratio = Ratio { num: 0, den: 1 };
    /// One, exactly.
    pub const ONE: Ratio = Ratio { num: 1, den: 1 };

    /// Build `num/den`, reduced. `den` must be nonzero or the result is refused.
    pub fn new(num: u64, den: u64) -> Refused<Self> {
        if den == 0 {
            return Err(Refusal::owned(RefusalKind::RatiosDoNotSumToOne, "zero denominator"));
        }
        Ok(Self::unchecked(num, den))
    }

    /// A whole number as a ratio.
    pub const fn from_u64(n: u64) -> Self {
        Ratio { num: n, den: 1 }
    }

    fn unchecked(num: u64, den: u64) -> Self {
        let g = gcd(num, den);
        if g == 0 {
            return Ratio::ZERO;
        }
        Ratio { num: num / g, den: den / g }
    }

    pub const fn numerator(self) -> u64 {
        self.num
    }

    pub const fn denominator(self) -> u64 {
        self.den
    }

    pub const fn is_zero(self) -> bool {
        self.num == 0
    }

    pub const fn is_one(self) -> bool {
        self.num == 1 && self.den == 1
    }

    /// The exact value, for display and for the appearance layer only.
    ///
    /// Never fed back into an exact quantity: `to_f64` exists so a *number can be
    /// shown*, not so a number can be decided with.
    pub fn to_f64(self) -> f64 {
        self.num as f64 / self.den as f64
    }

    /// Exact subtraction, **refusing a negative result**.
    ///
    /// Deliberately not `Sub`. The standard operator would have to either panic or
    /// wrap on `a - b` where `b > a`, and a *negative* quantity of anything — a
    /// mass, a volume, a mole fraction — is not a small number, it is a wrong
    /// one. A negative component ratio is also the one thing
    /// [`RefusalKind::ZeroRatio`]’s sibling cannot express: this is where the
    /// refusal has to be.
    ///
    /// **This operation exists; [`crate::conservation::Mass::checked_sub`] still
    /// refuses and does not call it.** The arithmetic was built here first, over a
    /// common denominator, and the mass-level refusal was then *kept* by ruling:
    /// asked which mattered more, a `WeighingGate::net` or the pinned absence that
    /// says out loud the world cannot yet net two masses, the pin was chosen. So a
    /// ratio subtracts exactly and a mass still does not, and this paragraph is
    /// the honest record of that gap rather than a stale cross-reference.
    ///
    /// `0 - 0` is `Ratio::ZERO`, not a refusal. There is nothing to withdraw from
    /// an empty account *because nothing was withdrawn from it*, and a guard that
    /// refused the zero case would make `0 - 0` the only subtraction in the model
    /// that lies about sign.
    pub fn checked_sub(self, other: Ratio) -> Refused<Self> {
        if other.den == 0 || self.den == 0 {
            return Err(Refusal::owned(RefusalKind::RatiosDoNotSumToOne, "zero denominator"));
        }
        let g = gcd(self.den, other.den);
        let l = (self.den / g).saturating_mul(other.den);
        let a = self.num.saturating_mul(l / self.den);
        let b = other.num.saturating_mul(l / other.den);
        if b > a {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!("{self} - {other} is negative; a quantity may not be withdrawn that was \
                         never deposited"),
            ));
        }
        Ok(Ratio::unchecked(a - b, l))
    }

    /// Whether this ratio is strictly greater than `other`, exactly.
    ///
    /// Cross-multiplied rather than dividing, because the division form loses the
    /// distinction between two nearly-equal exact rationals to `f64` rounding —
    /// which is how a mixture's composition check would start accepting ratios
    /// that do not sum to one.
    pub fn is_greater_than(self, other: Ratio) -> bool {
        self.num.saturating_mul(other.den) > other.num.saturating_mul(self.den)
    }
}

impl core::ops::Add for Ratio {
    type Output = Ratio;

    /// **Exact**, and saturating rather than wrapping: a denominator that
    /// overflows is caught by the sum-to-one test, never silently replaced by a
    /// denominator that happens to divide.
    fn add(self, other: Ratio) -> Ratio {
        let l = lcm(self.den, other.den);
        let a = self.num.saturating_mul(l / self.den);
        let b = other.num.saturating_mul(l / other.den);
        Ratio::unchecked(a.saturating_add(b), l)
    }
}

impl core::ops::AddAssign for Ratio {
    /// Exact, like [`Ratio`]'s `Add`.
    fn add_assign(&mut self, other: Ratio) {
        *self = *self + other;
    }
}

impl core::ops::Mul for Ratio {
    type Output = Ratio;

    /// **Exact.**
    fn mul(self, other: Ratio) -> Ratio {
        Ratio::unchecked(
            self.num.saturating_mul(other.num),
            self.den.saturating_mul(other.den),
        )
    }
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

fn lcm(a: u64, b: u64) -> u64 {
    let g = gcd(a, b);
    if g == 0 {
        return 0;
    }
    // Saturating: a denominator that overflows is caught by the sum test, not
    // silently wrapped into a denominator that happens to divide.
    (a / g).saturating_mul(b)
}

impl fmt::Display for Ratio {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.den == 1 {
            write!(f, "{}", self.num)
        } else {
            write!(f, "{}/{}", self.num, self.den)
        }
    }
}

/// A component's identity inside the registry.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ComponentId(pub u16);

/// A material's identity inside the registry. **Zero is reserved and means
/// empty** — the absence of matter, not a material.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct MaterialId(pub u16);

impl MaterialId {
    /// The absence of matter. Never admitted, never valid as a component.
    pub const EMPTY: MaterialId = MaterialId(0);
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

/// One component's share of a material. The ratio is a **mass fraction** and is
/// exact. Per F3, the UI shows each component on its **own** scale so the numbers
/// are never presented as a set that should total 1 on screen.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Component {
    pub id: ComponentId,
    pub ratio: Ratio,
    pub density_kg_m3: Ratio,
}

/// How a material interacts with light. The appearance layer reads this; the
/// model stores it because a fluid and a rock differ in *kind*, not in colour.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OpticalClass {
    Opaque,
    Transmissive,
}

/// How much of the light arriving at a surface comes back off it, per channel,
/// **exact**.
///
/// Like [`Component::density_kg_m3`] this is a *physical* property, and like it
/// it is an exact rational rather than a float: the recorded value is then a
/// statement somebody can check rather than a number that drifted through a
/// formatting step. A measurement's own spread is the source's business, and
/// [`Reflectance::broadband`] exists so that a single-number albedo is
/// representable without pretending it was measured per channel.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Reflectance {
    pub r: Ratio,
    pub g: Ratio,
    pub b: Ratio,
}

impl Reflectance {
    /// Refuses a channel above 1: a surface that sends back more light than
    /// falls on it is a reflectance that does not mean anything, and clamping it
    /// would make an over-unity measurement look like a bright surface.
    pub fn new(r: Ratio, g: Ratio, b: Ratio) -> Refused<Self> {
        for (channel, value) in [("r", r), ("g", g), ("b", b)] {
            if value.to_f64() > 1.0 {
                return Err(Refusal::owned(
                    RefusalKind::OutOfDeclaredRange,
                    format!(
                        "reflectance channel {channel} is {} and a reflectance is at most 1",
                        value.to_f64()
                    ),
                ));
            }
        }
        Ok(Reflectance { r, g, b })
    }

    /// One number for all three channels — which is what a **broadband** albedo
    /// is, and saying so in the constructor keeps an achromatic value from
    /// looking like a chromatic one that happens to be grey.
    pub fn broadband(value: Ratio) -> Refused<Self> {
        Self::new(value, value, value)
    }

    pub fn linear(self) -> [f64; 3] {
        [self.r.to_f64(), self.g.to_f64(), self.b.to_f64()]
    }
}

/// A reflectance **and where it came from**.
///
/// The two are one value rather than two fields, and that is the whole point: a
/// reflectance with no provenance is exactly the invented number the appearance
/// layer refuses to derive (`docs/LEDGER.md` C-27), so "sourced" is a property of
/// the type and not a convention a later edit can drop. Same discipline as the
/// four-bucket rule in `voxel-ui`, applied to a physical measurement.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SourcedReflectance {
    pub value: Reflectance,
    /// Where the number came from, and — when the value is a *choice inside a
    /// published range* rather than a measurement — that it is.
    pub source: &'static str,
}

/// An admitted material.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Material {
    pub id: MaterialId,
    pub name: String,
    pub components: Vec<Component>,
    pub optical_class: OpticalClass,
    /// What the material reflects, or **a named absence**.
    ///
    /// `None` means the record does not carry a reflectance for this material,
    /// which is the state every material was in until this round — and the
    /// reason the world drew black. It stays representable, because a material
    /// admitted without one is a material nobody has a spectrum for yet, and
    /// inventing a grey for it is the bug this field exists to fix.
    pub reflectance: Option<SourcedReflectance>,
}

impl Material {
    /// Bulk density = Σ(ratio · density), exactly. `kg/m³` as an exact rational.
    pub fn bulk_density_kg_m3(&self) -> Ratio {
        let mut acc = Ratio::ZERO;
        for c in &self.components {
            acc += c.ratio * c.density_kg_m3;
        }
        acc
    }

    /// The sum of the component mass fractions. Exactly 1 for an admitted
    /// material; anything else is the refusal that will be named.
    pub fn ratio_sum(&self) -> Ratio {
        let mut acc = Ratio::ZERO;
        for c in &self.components {
            acc += c.ratio;
        }
        acc
    }
}

/// The admitted component and material set.
///
/// Empty by default and **stays empty unless a row is admitted**, which is the
/// honest representation of F2's measured state: the machinery is settled and
/// complete, the admitted content is zero. A world that needs a material the
/// registry has not admitted gets a refusal, not a grey cube.
#[derive(Clone, Default, Debug)]
pub struct MaterialRegistry {
    components: BTreeMap<ComponentId, String>,
    materials: BTreeMap<MaterialId, Material>,
}

impl MaterialRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Admit a component (a substance that can appear in a material).
    pub fn admit_component(&mut self, id: ComponentId, name: impl Into<String>) -> &mut Self {
        self.components.insert(id, name.into());
        self
    }

    pub fn component_name(&self, id: ComponentId) -> Option<&str> {
        self.components.get(&id).map(String::as_str)
    }

    pub fn admitted_material_count(&self) -> usize {
        self.materials.len()
    }

    /// Admit a material, refusing every reason it cannot be admitted.
    ///
    /// **Every** reason: the set is collected, so a material with three problems
    /// reports three problems in one round rather than one per rebuild.
    pub fn admit_material(&mut self, material: Material) -> Result<(), RefusalSet> {
        let mut set = RefusalSet::new();

        if material.id.is_empty() {
            set.refuse(RefusalKind::UnadmittedMaterial, "material id 0 is reserved for empty");
        }
        if material.components.is_empty() {
            set.refuse(RefusalKind::RatiosDoNotSumToOne, format!("{} has no components", material.name));
        }

        for (i, c) in material.components.iter().enumerate() {
            if c.ratio.is_zero() {
                set.refuse(
                    RefusalKind::ZeroRatio,
                    format!("{} component {i} has ratio 0", material.name),
                );
            }
            if !self.components.contains_key(&c.id) {
                set.refuse(
                    RefusalKind::UnknownComponent,
                    format!("{} component {i} references {:?}", material.name, c.id),
                );
            }
        }

        let sum = material.ratio_sum();
        if !sum.is_one() {
            set.refuse(
                RefusalKind::RatiosDoNotSumToOne,
                format!("{} ratios sum to {sum}, not 1", material.name),
            );
        }

        if !set.is_empty() {
            return Err(set);
        }

        self.materials.insert(material.id, material);
        Ok(())
    }

    /// Look a material up. An unadmitted id is **refused by name**.
    pub fn material(&self, id: MaterialId) -> Refused<&Material> {
        self.materials.get(&id).ok_or_else(|| {
            Refusal::owned(
                RefusalKind::UnadmittedMaterial,
                format!("material {} was never admitted", id.0),
            )
        })
    }

    pub fn materials(&self) -> impl Iterator<Item = &Material> {
        self.materials.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> MaterialRegistry {
        let mut r = MaterialRegistry::new();
        r.admit_component(ComponentId(1), "silica");
        r.admit_component(ComponentId(2), "feldspar");
        r
    }

    #[test]
    fn ratios_are_exact_and_reduce() {
        assert_eq!(Ratio::new(2, 4).unwrap(), Ratio::new(1, 2).unwrap());
        // The float trap: 1/3 + 1/3 + 1/3 is exactly one, where f64 gives
        // 0.9999999999999999.
        let third = Ratio::new(1, 3).unwrap();
        let sum = third + third + third;
        assert!(sum.is_one(), "third + third + third was {sum}");
        assert_eq!(sum, Ratio::ONE);
    }

    #[test]
    fn a_ratio_with_a_zero_denominator_is_refused() {
        let r = Ratio::new(1, 0).unwrap_err();
        assert_eq!(r.kind(), RefusalKind::RatiosDoNotSumToOne);
    }

    #[test]
    fn a_material_admits_when_its_ratios_sum_to_one() {
        let mut r = registry();
        let m = Material {
            id: MaterialId(1),
            name: "granite".into(),
            components: vec![
                Component {
                    id: ComponentId(1),
                    ratio: Ratio::new(2, 3).unwrap(),
                    density_kg_m3: Ratio::from_u64(2650),
                },
                Component {
                    id: ComponentId(2),
                    ratio: Ratio::new(1, 3).unwrap(),
                    density_kg_m3: Ratio::from_u64(2550),
                },
            ],
            optical_class: OpticalClass::Opaque,
            reflectance: None,
        };
        // Bulk density = (2/3)*2650 + (1/3)*2550 = 2616 + 2/3 = 7850/3.
        let density = m.bulk_density_kg_m3();
        assert_eq!(density.numerator(), 7850);
        assert_eq!(density.denominator(), 3);
        assert!(r.admit_material(m).is_ok());
        assert_eq!(r.admitted_material_count(), 1);
    }

    #[test]
    fn a_material_whose_ratios_do_not_sum_to_one_is_refused_by_name() {
        let mut r = registry();
        let m = Material {
            id: MaterialId(1),
            name: "wishful".into(),
            components: vec![Component {
                id: ComponentId(1),
                ratio: Ratio::new(1, 2).unwrap(),
                density_kg_m3: Ratio::from_u64(2000),
            }],
            optical_class: OpticalClass::Opaque,
            reflectance: None,
        };
        let err = r.admit_material(m).unwrap_err();
        assert_eq!(err.len(), 1);
        assert_eq!(err.all()[0].kind(), RefusalKind::RatiosDoNotSumToOne);
        assert_eq!(r.admitted_material_count(), 0);
    }

    #[test]
    fn every_reason_a_material_is_bad_is_reported_at_once() {
        let mut r = registry();
        let m = Material {
            id: MaterialId(1),
            name: "three_problems".into(),
            components: vec![
                Component {
                    id: ComponentId(99), // not admitted
                    ratio: Ratio::ZERO,  // zero ratio
                    density_kg_m3: Ratio::from_u64(1),
                },
                Component {
                    id: ComponentId(1),
                    ratio: Ratio::new(1, 2).unwrap(),
                    density_kg_m3: Ratio::from_u64(1),
                },
            ],
            optical_class: OpticalClass::Opaque,
            reflectance: None,
        };
        let err = r.admit_material(m).unwrap_err();
        // zero ratio, unknown component, and a sum that is not one.
        assert_eq!(err.len(), 3, "refusals were {err}");
    }

    #[test]
    fn an_unadmitted_material_is_refused_rather_than_defaulted() {
        let r = registry();
        let err = r.material(MaterialId(404)).unwrap_err();
        assert_eq!(err.kind(), RefusalKind::UnadmittedMaterial);
    }

    #[test]
    fn material_id_zero_is_empty_and_never_admitted() {
        assert!(MaterialId::EMPTY.is_empty());
        let mut r = registry();
        let m = Material {
            id: MaterialId::EMPTY,
            name: "null".into(),
            components: vec![Component {
                id: ComponentId(1),
                ratio: Ratio::ONE,
                density_kg_m3: Ratio::from_u64(1),
            }],
            optical_class: OpticalClass::Opaque,
            reflectance: None,
        };
        assert!(r.admit_material(m).is_err());
    }
}
