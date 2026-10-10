//! Conservation: the **exact** set, and the boundary it must not be crossed by.
//!
//! F1 names six declared **approximate** families (temperature; pressure and
//! flow; process rates and cadences; weather, atmosphere and climate; ecology
//! population counts; regional field interpolation) against a **must-remain-exact**
//! set (**mass, volume, component lineage, fluid volume and mass, lifecycle
//! counters**). The stated risk is precise:
//!
//! > *if a tolerance-bearing quantity ever feeds mass or volume, the
//! > audited-to-the-gram claim silently weakens.*
//!
//! So the boundary is a **type and a refusal**, not a comment:
//!
//! * [`Exact`] holds a [`Ratio`] and is the only thing [`Mass`] and [`Volume`]
//!   are built from;
//! * [`Approximate`] holds an `f64` and **cannot reach** a `Mass` without going
//!   through [`Mass::from_approximate`], which **refuses unless a tolerance is
//!   explicitly declared** — and records that the claim weakened;
//! * [`ApproximateFamily`] names *which* family a value belongs to and carries
//!   this tree's declared [`ToleranceClass`] for it, so every refusal answers
//!   *which family's* tolerance is missing — and **pressure and flow may gate
//!   exact movement but can never feed mass**.
//!
//! F10's own ruling is here too: *an assembly recipe is a mass pump unless it
//! declares itself one.* A [`Ledger::pump`] without a [`MassPumpDeclaration`] is
//! refused.

use crate::material::Ratio;
use crate::refusal::{Refusal, RefusalKind, RefusalSet, Refused};
use core::fmt;

/// A quantity that is exact by construction.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub struct Exact(Ratio);

impl Exact {
    pub const fn new(ratio: Ratio) -> Self {
        Exact(ratio)
    }
    pub const fn ratio(self) -> Ratio {
        self.0
    }
}

impl core::ops::Add for Exact {
    type Output = Exact;
    fn add(self, other: Exact) -> Exact {
        Exact(self.0 + other.0)
    }
}

impl core::ops::Mul for Exact {
    type Output = Exact;
    fn mul(self, other: Exact) -> Exact {
        Exact(self.0 * other.0)
    }
}

/// A quantity that carries a tolerance. **Never** feeds an exact quantity
/// silently.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Approximate {
    pub value: f64,
}

impl Approximate {
    pub fn new(value: f64) -> Refused<Self> {
        if !value.is_finite() {
            return Err(Refusal::owned(
                RefusalKind::NonFiniteValue,
                format!("approximate quantity {value} is not finite"),
            ));
        }
        Ok(Approximate { value })
    }
}

/// F1's six declared **approximate** families (`docs/DESIGN-SPEC.md` §2.1), each
/// with this tree's declared [`ToleranceClass`].
///
/// The list is the design spec's own, and it is closed: a seventh family is a
/// decision about the world's physics, not a convenience. [`ApproximateFamily::ALL`]
/// is the covered list and the tests count it.
///
/// # Why a class and not a number
///
/// Most families have no tolerance this tree may honestly declare yet — a number
/// with no measurement behind it is the fabrication the discipline refuses. What
/// *can* be declared is each family's **class**: whether, and how, its values may
/// meet exact quantities at all. Declaring a crossing is editing one arm of
/// [`ApproximateFamily::tolerance_class`] with its reason in the doc — the one
/// place tolerances are written, so every refusal downstream names both the family
/// it refuses and what that family currently declares.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum ApproximateFamily {
    /// Temperature.
    Temperature,
    /// Pressure and flow.
    PressureAndFlow,
    /// Process rates and cadences for erosion and tectonics.
    ProcessRatesAndCadences,
    /// Weather, atmosphere and climate.
    WeatherAtmosphereClimate,
    /// Ecology population counts and coupling.
    EcologyPopulation,
    /// Regional field interpolation.
    RegionalFieldInterpolation,
}

/// What this tree declares about one approximate family's values meeting exact
/// quantities. See [`ApproximateFamily`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ToleranceClass {
    /// No tolerance is declared: the family's values may not feed mass at all, and
    /// the refusal names the family. This is the honest default — an empty column
    /// is not a zero.
    NoCrossing,
    /// A declared comparison tolerance: the family's values may **decide** exact
    /// movement (a direction, a stop) and may not feed mass. The tolerance is in
    /// the unit the comparison is stated in, and the comparison must state it.
    Gating(Ratio),
    /// A declared crossing tolerance: a value of this family may cross into mass
    /// through [`Mass::from_approximate_family`], and the caller has said out loud
    /// that the claim weakens.
    Crossing(Ratio),
}

impl ApproximateFamily {
    /// Every family, in the design spec's order.
    ///
    /// Hand-written on purpose: this is the list a test counts, so a seventh
    /// family cannot appear without a test failing first.
    pub const ALL: [ApproximateFamily; 6] = [
        ApproximateFamily::Temperature,
        ApproximateFamily::PressureAndFlow,
        ApproximateFamily::ProcessRatesAndCadences,
        ApproximateFamily::WeatherAtmosphereClimate,
        ApproximateFamily::EcologyPopulation,
        ApproximateFamily::RegionalFieldInterpolation,
    ];

    /// The stable, machine-readable name.
    pub const fn name(self) -> &'static str {
        match self {
            ApproximateFamily::Temperature => "temperature",
            ApproximateFamily::PressureAndFlow => "pressure and flow",
            ApproximateFamily::ProcessRatesAndCadences => "process rates and cadences",
            ApproximateFamily::WeatherAtmosphereClimate => "weather, atmosphere and climate",
            ApproximateFamily::EcologyPopulation => "ecology population",
            ApproximateFamily::RegionalFieldInterpolation => "regional field interpolation",
        }
    }

    /// The tolerance class this tree declares for the family.
    ///
    /// One arm per family, and each arm says why. This is the **only** place a
    /// tolerance may be written down; it is `const` so a caller that depends on a
    /// class (the fluidics gate) reads it at compile time and the build fails if
    /// the declaration it needs is withdrawn.
    pub const fn tolerance_class(self) -> ToleranceClass {
        match self {
            // Nothing converts a temperature into mass; phase changes will decide
            // *which* exact movement happens, and their threshold question is theirs
            // to bring when they land.
            ApproximateFamily::Temperature => ToleranceClass::NoCrossing,
            // **Declared: one millimetre of head.** Read from the reference's
            // hydrology mechanism — `HYDROLOGY_TOLERANCE = 1` at
            // `HYDROLOGY_SCALE = 1000`
            // (`references/Docs-A/ala-cities/src/world/hydrology.rs`), where a
            // pressure difference at or below the tolerance authorises no flow.
            // The reference is reference, not authority (C-48/C-49): the mechanism
            // is cited and this tree declares the number. A head difference of one
            // millimetre moves no water.
            ApproximateFamily::PressureAndFlow => {
                ToleranceClass::Gating(Ratio::from_parts(1, 1000))
            }
            // Erosion and tectonics rates move material, which is mass-bearing, so
            // the first rate that crosses into mass must declare its tolerance in
            // this arm first. Nothing does yet.
            ApproximateFamily::ProcessRatesAndCadences => ToleranceClass::NoCrossing,
            // **Declared: the rain crosses.** Precipitation deposits water mass, and
            // this is the first crossing this project genuinely needs (C-152.2's own
            // sentence). The crossing's value is a rainfall rate, and its unit is
            // **kilograms per square metre per year** — the same number as millimetres
            // of annual rain, because one millimetre of water over one square metre is
            // exactly one kilogram. `Mass::from_approximate` keeps whole kilograms, so
            // the tolerance declared here is **one kilogram per square metre per year**
            // (one millimetre of annual rain), and the rounding is at most half of it.
            // The deposit is `voxel-shapers::water`'s precipitation step, which takes
            // the approximation **once, at the crossing**, and slices the crossed
            // annual figure per tick in exact arithmetic from there.
            ApproximateFamily::WeatherAtmosphereClimate => {
                ToleranceClass::Crossing(Ratio::from_u64(1))
            }
            // A population is a count, and a count that manufactures mass is the
            // exact failure the boundary exists to catch.
            ApproximateFamily::EcologyPopulation => ToleranceClass::NoCrossing,
            // An interpolated field sample decides *which* cell a fact belongs to;
            // it never decides how much mass there is.
            ApproximateFamily::RegionalFieldInterpolation => ToleranceClass::NoCrossing,
        }
    }
}

/// Mass, in kilograms, exact.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub struct Mass(Exact);

impl Mass {
    pub const ZERO: Mass = Mass(Exact(Ratio::ZERO));

    pub const fn from_exact(e: Exact) -> Self {
        Mass(e)
    }

    pub const fn exact(self) -> Exact {
        self.0
    }

    /// A **refusing** subtraction, so it is deliberately not `Sub`.
    ///
    /// `a - b` panics or wraps in the standard operator; this returns a refusal
    /// when `b > a`, because a negative mass is not a mass. Implementing `Sub`
    /// would hide that behind an operator that cannot refuse.
    /// **It refuses unconditionally, and that is a ruling rather than an
    /// oversight.** The arithmetic is not the obstacle: `Ratio::checked_sub`
    /// exists and does exactly the work over a common denominator, so `1/3 - 1/6`
    /// could already be exactly `1/6`. The obstacle was removed and the refusal
    /// kept anyway.
    ///
    /// Asked which of the two mattered more — a `WeighingGate::net` that reports
    /// inward minus outward, or the pinned absence that says out loud that the
    /// world cannot yet report a net mass at all — the user chose the pin. So
    /// this is **load-bearing debt, not a gap**, and the closed water ledger owes
    /// a netting operation before it can be built. `voxel-instrument`'s
    /// `the_model_has_no_exact_subtraction_so_a_net_mass_cannot_be_reported` pins
    /// this refusal, and it is green because the refusal is here.
    ///
    /// The message names both masses so a log can show what was refused without
    /// the caller re-deriving them.
    pub fn checked_sub(self, other: Mass) -> Refused<Mass> {
        Err(Refusal::owned(
            RefusalKind::Unimplemented,
            format!(
                "exact rational subtraction is not implemented; do not approximate it \
                 ({self:?} - {other:?})"
            ),
        ))
    }

    pub fn is_zero(self) -> bool {
        self.0.ratio().is_zero()
    }

    /// Cross the exact/approximate boundary **deliberately**.
    ///
    /// Refuses when no tolerance is declared. When one is, the mass is the
    /// rounded value and the caller has *said out loud* that the claim weakens.
    pub fn from_approximate(value: Approximate, declared_tolerance: Option<Ratio>) -> Refused<Mass> {
        match declared_tolerance {
            None => Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!(
                    "approximate value {} may not feed mass without a declared tolerance (F1)",
                    value.value
                ),
            )),
            Some(t) => {
                if t.is_zero() {
                    return Err(Refusal::owned(
                        RefusalKind::ZeroRatio,
                        "a declared tolerance of zero is an exact claim about an inexact value",
                    ));
                }
                // Integer kilograms only: a rounding to sub-kilogram precision
                // would be a second, undeclared approximation.
                let kg = value.value.round();
                if kg < 0.0 || kg > u64::MAX as f64 {
                    return Err(Refusal::owned(
                        RefusalKind::OutOfDeclaredRange,
                        format!("approximate mass {kg} outside representable range"),
                    ));
                }
                Ok(Mass(Exact(Ratio::from_u64(kg as u64))))
            }
        }
    }

    /// Cross the boundary **by family**: the tolerance is the family's own
    /// declared one, and no caller-supplied number can cross here.
    ///
    /// This is deliberately narrower than [`Mass::from_approximate`]: a family
    /// that only [`ToleranceClass::Gating`]s movement *refuses*, because pressure
    /// may steer water but may not become water; a family with no declared
    /// tolerance refuses by name; and only a family that declares
    /// [`ToleranceClass::Crossing`] reaches the tolerance-based door. The refusal
    /// names the family, which is the point of the vocabulary: *"may not feed mass
    /// without a declared tolerance"* becomes *"temperature may not feed mass"*.
    pub fn from_approximate_family(
        family: ApproximateFamily,
        value: Approximate,
    ) -> Refused<Mass> {
        match family.tolerance_class() {
            ToleranceClass::NoCrossing => Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!(
                    "approximate value {} of family `{}` may not feed mass: no tolerance is \
                     declared for that family (F1)",
                    value.value,
                    family.name()
                ),
            )),
            ToleranceClass::Gating(tolerance) => Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!(
                    "approximate value {} of family `{}` gates exact movement at {tolerance} \
                     but may not feed mass (F1)",
                    value.value,
                    family.name()
                ),
            )),
            ToleranceClass::Crossing(tolerance) => Self::from_approximate(value, Some(tolerance)),
        }
    }
}

impl core::ops::Add for Mass {
    type Output = Mass;
    fn add(self, other: Mass) -> Mass {
        Mass(self.0 + other.0)
    }
}

impl fmt::Display for Mass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} kg", self.0.ratio())
    }
}

/// Volume, in cubic metres, exact.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub struct Volume(Exact);

impl Volume {
    pub const fn from_exact(e: Exact) -> Self {
        Volume(e)
    }
    pub const fn exact(self) -> Exact {
        self.0
    }
}

/// Where a mass came from, or went. A named source is what lets a ledger balance
/// **without** inventing matter.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Source {
    /// Drawn from, or returned to, the atmosphere (F3: damage adds mass from a
    /// declared source).
    Atmosphere,
    /// The world substrate itself.
    World,
    /// A named producer or consumer.
    Named(String),
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Source::Atmosphere => f.write_str("atmosphere"),
            Source::World => f.write_str("world"),
            Source::Named(n) => write!(f, "{n}"),
        }
    }
}

/// A declared mass pump — F10's requirement that a recipe which **creates** mass
/// says so.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct MassPumpDeclaration {
    pub reason: String,
    pub source: Source,
}

/// One movement of matter.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Entry {
    pub amount: Mass,
    pub source: Source,
    pub note: String,
}

/// An append-only mass ledger for one transformation.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct Ledger {
    entries: Vec<Entry>,
    pump: Option<MassPumpDeclaration>,
}

impl Ledger {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record matter entering the transformation.
    pub fn deposit(&mut self, amount: Mass, source: Source, note: impl Into<String>) -> &mut Self {
        self.entries.push(Entry { amount, source, note: note.into() });
        self
    }

    /// Record matter leaving it.
    pub fn withdraw(&mut self, amount: Mass, source: Source, note: impl Into<String>) -> &mut Self {
        // A withdrawal is recorded as an entry with the source it returns to.
        self.entries.push(Entry { amount, source, note: note.into() });
        self
    }

    /// Declare that this transformation **creates** matter, and name the source.
    pub fn declare_pump(&mut self, declaration: MassPumpDeclaration) -> &mut Self {
        self.pump = Some(declaration);
        self
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// Fail when the ledger moves mass it did not declare.
    ///
    /// The check is deliberately weak on arithmetic (exact subtraction is not
    /// implemented) and deliberately strong on **declaration**: a ledger whose
    /// entries are non-empty and which has no pump declaration is *not* claimed
    /// to balance.
    pub fn audit(&self) -> Result<(), RefusalSet> {
        let mut set = RefusalSet::new();
        for e in &self.entries {
            if e.amount.is_zero() {
                set.refuse(
                    RefusalKind::ZeroRatio,
                    format!("a zero-mass entry ({}) records nothing", e.note),
                );
            }
        }
        if !self.entries.is_empty() && self.pump.is_none() {
            set.refuse(
                RefusalKind::OutOfDeclaredRange,
                "a transformation that moves mass must either balance or declare a pump (F10)",
            );
        }
        if set.is_empty() {
            Ok(())
        } else {
            Err(set)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_family_list_is_exactly_the_six_the_ruling_declares() {
        // F1 names six approximate families and no seventh. If this fails, a
        // family was invented — which is the one thing this vocabulary must not
        // contain.
        assert_eq!(ApproximateFamily::ALL.len(), 6);
        let mut names: Vec<&str> = ApproximateFamily::ALL.iter().map(|f| f.name()).collect();
        let before = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), before, "two approximate families share a name");
        for family in ApproximateFamily::ALL {
            let _ = family.tolerance_class();
        }
    }

    #[test]
    fn pressure_and_flow_gates_at_one_millimetre_of_head_and_may_not_feed_mass() {
        // The declared value is the reference's hydrology mechanism, cited in
        // `tolerance_class`: tolerance 1 at scale 1000. Cited, never copied (C-48).
        match ApproximateFamily::PressureAndFlow.tolerance_class() {
            ToleranceClass::Gating(tolerance) => {
                assert_eq!(tolerance, Ratio::new(1, 1000).unwrap())
            }
            other => panic!("pressure and flow should gate, it declares {other:?}"),
        }
        let refusal = Mass::from_approximate_family(
            ApproximateFamily::PressureAndFlow,
            Approximate::new(3.5).unwrap(),
        )
        .unwrap_err();
        assert!(refusal.detail().contains("pressure and flow"));
        assert!(refusal.detail().contains("may not feed mass"));
    }

    #[test]
    fn the_weather_family_crosses_because_rain_deposits_mass() {
        // The declaration and its unit, pinned: the crossing's tolerance is one
        // kilogram per square metre per year — one millimetre of annual rain — and
        // the crossing rounds to whole kilograms, so the approximation is bounded by
        // half a millimetre of annual rain. This is the crossing C-152 deliberately
        // left undeclared until the rain deposit landed (C-154).
        match ApproximateFamily::WeatherAtmosphereClimate.tolerance_class() {
            ToleranceClass::Crossing(tolerance) => {
                assert_eq!(tolerance, Ratio::from_u64(1));
                assert_eq!(tolerance.numerator(), 1, "the tolerance is in kg/m2/yr");
                assert_eq!(tolerance.denominator(), 1);
            }
            other => panic!("weather should cross, it declares {other:?}"),
        }
        let crossed = Mass::from_approximate_family(
            ApproximateFamily::WeatherAtmosphereClimate,
            Approximate::new(900.4).unwrap(),
        )
        .unwrap();
        assert_eq!(crossed.exact().ratio(), Ratio::from_u64(900));
        // And the quantum is a whole kilogram: a column under half a millimetre of
        // annual rain crosses to nothing, which is the honest floor of the crossing
        // rather than a silent zero.
        let below = Mass::from_approximate_family(
            ApproximateFamily::WeatherAtmosphereClimate,
            Approximate::new(0.4).unwrap(),
        )
        .unwrap();
        assert_eq!(below.exact().ratio(), Ratio::ZERO);
    }

    #[test]
    fn a_family_that_is_not_crossing_refuses_mass_and_the_refusal_names_it() {
        // The point of the vocabulary: "may not feed mass without a declared
        // tolerance" becomes an answer about *which* family's tolerance is
        // missing. This walks every family, so declaring a crossing later cannot
        // leave this test silently covering less.
        for family in ApproximateFamily::ALL {
            if matches!(family.tolerance_class(), ToleranceClass::Crossing(_)) {
                continue;
            }
            let refusal =
                Mass::from_approximate_family(family, Approximate::new(1.0).unwrap()).unwrap_err();
            assert!(
                refusal.detail().contains(family.name()),
                "the refusal for {} did not name the family: {refusal:?}",
                family.name()
            );
        }
    }

    #[test]
    fn an_approximate_quantity_cannot_feed_mass_without_a_declared_tolerance() {
        let approx = Approximate::new(1234.7).unwrap();
        let r = Mass::from_approximate(approx, None).unwrap_err();
        assert_eq!(r.kind(), RefusalKind::OutOfDeclaredRange);
        assert!(r.detail().contains("may not feed mass"));
    }

    #[test]
    fn with_a_declared_tolerance_the_boundary_is_crossed_deliberately() {
        let approx = Approximate::new(1234.7).unwrap();
        let m = Mass::from_approximate(approx, Some(Ratio::new(1, 100).unwrap())).unwrap();
        // Rounded to whole kilograms — a second undeclared approximation is not
        // allowed to creep in.
        assert_eq!(m.exact().ratio(), Ratio::from_u64(1235));
    }

    #[test]
    fn a_zero_tolerance_is_an_exact_claim_about_an_inexact_value() {
        let approx = Approximate::new(1.0).unwrap();
        let r = Mass::from_approximate(approx, Some(Ratio::ZERO)).unwrap_err();
        assert_eq!(r.kind(), RefusalKind::ZeroRatio);
    }

    #[test]
    fn a_non_finite_approximation_is_refused_at_construction() {
        assert!(Approximate::new(f64::NAN).is_err());
        assert!(Approximate::new(f64::INFINITY).is_err());
        assert!(Approximate::new(1.0).is_ok());
    }

    #[test]
    fn exact_arithmetic_is_exact() {
        // 1/3 kg three times is exactly 1 kg. In f64 it is 0.9999999999999999.
        let third = Mass::from_exact(Exact::new(Ratio::new(1, 3).unwrap()));
        let total = third + third + third;
        assert_eq!(total.exact().ratio(), Ratio::ONE);
    }

    #[test]
    fn a_ledger_that_moves_mass_must_declare_a_pump_or_balance() {
        let mut l = Ledger::new();
        l.deposit(Mass::from_exact(Exact::new(Ratio::from_u64(5))), Source::World, "ore");
        let err = l.audit().unwrap_err();
        assert_eq!(err.len(), 1);
        assert!(err.all()[0].detail().contains("declare a pump"));

        l.declare_pump(MassPumpDeclaration {
            reason: "smelting adds oxygen from atmosphere".into(),
            source: Source::Atmosphere,
        });
        assert!(l.audit().is_ok());
    }

    #[test]
    fn an_empty_ledger_balances_trivially() {
        assert!(Ledger::new().audit().is_ok());
    }

    #[test]
    fn a_zero_mass_entry_records_nothing_and_is_refused() {
        let mut l = Ledger::new();
        l.declare_pump(MassPumpDeclaration { reason: "none".into(), source: Source::World });
        l.deposit(Mass::ZERO, Source::World, "nothing");
        let err = l.audit().unwrap_err();
        assert_eq!(err.all()[0].kind(), RefusalKind::ZeroRatio);
    }

    #[test]
    fn an_exact_withdrawal_larger_than_the_deposit_is_refused_not_defaulted() {
        let a = Mass::from_exact(Exact::new(Ratio::from_u64(1)));
        let b = Mass::from_exact(Exact::new(Ratio::from_u64(2)));
        assert!(a.checked_sub(b).is_err());
    }
}
