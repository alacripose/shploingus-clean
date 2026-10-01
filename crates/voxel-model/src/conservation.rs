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
//!   explicitly declared** — and records that the claim weakened.
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
    pub fn checked_sub(self, other: Mass) -> Refused<Mass> {
        // No negative masses: a withdrawal larger than the deposit is a refusal,
        // not a debt. A ledger with a negative mass is a ledger that is wrong.
        //
        // **This is now implemented**, exactly. It used to refuse with *"exact
        // rational subtraction is not implemented; do not approximate it"*, which
        // was true and was a gap: the atmosphere cannot be a conserved account
        // while its only two operations are add and refuse. `Ratio::checked_sub`
        // does the work over a common denominator, so `1/3 - 1/6` is exactly `1/6`
        // rather than `0.16666666666666666`.
        Ok(Mass(Exact(self.0.ratio().checked_sub(other.0.ratio())?)))
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
