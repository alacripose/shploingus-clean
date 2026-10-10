//! `Rational` — the exact number the civilian economy is written in.
//!
//! # Copied whole, from `ala-cities` `src/matter/properties.rs`
//!
//! **Everything of the type is the original's**: a signed, normalized exact rational over
//! `i128`, its construction boundary, `from_integer`/`from_grams`, the four checked
//! operations, `try_cmp` (which *refuses* rather than falling back when cross-multiplication
//! would overflow), and `round_to_multiple`. The civilians are written in this number
//! everywhere — `Needs { nourishment, rest, safety }`, a consequence's compensation, a skill
//! threshold, a selection risk, a delta's amount — and a port that swapped it for this
//! tree's [`crate::material::Ratio`] would have to edit every call site of the machine, which
//! is the hand-rolling the port forbids.
//!
//! # What the seam changes, and only this
//!
//! The original's error type is `MatterError`, which lives in `matter` — the material
//! science this tree already owns a face of, and which the civilians do not use. So the
//! **refusals are mapped onto this tree's one refusal** ([`RefusalKind`]):
//!
//! | `ala-cities` | here | why |
//! |---|---|---|
//! | `MatterError::InvalidDenominator` | [`RefusalKind::RatiosDoNotSumToOne`] | a denominator `<= 0` is not a rational |
//! | `MatterError::ArithmeticOverflow` | [`RefusalKind::OutOfDeclaredRange`] | the value left the declared range of the type |
//! | `MatterError::RoundingToleranceExceeded` | [`RefusalKind::OutOfDeclaredRange`] | the rounding left the declared tolerance |
//! | `MatterError::NonNegativeQuantity` | [`RefusalKind::OutOfDeclaredRange`] | a quantity that may not be negative was |
//!
//! # Why not `Ratio`
//!
//! `Ratio` (`crate::material`) is **unsigned** (`u64/u64`) and has no subtraction; the
//! civilians' arithmetic is signed (`checked_sub` is how a deficit is expressed) and its
//! magnitude is `i128` (measured: a snapshot fixture uses `Rational::from_integer(i128::MAX)`
//! to make an account overflow on purpose). Two different numbers, and one name for each: the
//! tree keeps its own and this is the port's, side by side and declared.

use crate::refusal::{Refusal, RefusalKind, Refused};
use core::cmp::Ordering;

/// An exact normalized rational quantity. Construction is the only public ingestion
/// boundary; the normalized components are private.
///
/// Verbatim from the original, with `MatterError` mapped onto [`Refusal`] — see the module
/// documentation for the table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rational {
    numerator: i128,
    denominator: i128,
}

impl Rational {
    pub const ZERO: Self = Self {
        numerator: 0,
        denominator: 1,
    };

    pub fn new(numerator: i128, denominator: i128) -> Refused<Self> {
        if denominator <= 0 {
            return Err(Refusal::owned(
                RefusalKind::RatiosDoNotSumToOne,
                format!("denominator {denominator} is not positive, so this is not a rational"),
            ));
        }
        if numerator == 0 {
            return Ok(Self::ZERO);
        }
        let divisor = gcd(numerator.unsigned_abs(), denominator as u128);
        Ok(Self {
            numerator: numerator / divisor as i128,
            denominator: denominator / divisor as i128,
        })
    }

    pub const fn from_integer(value: i128) -> Self {
        Self {
            numerator: value,
            denominator: 1,
        }
    }

    pub const fn from_grams(grams: i128) -> Self {
        Self {
            numerator: grams,
            denominator: 1,
        }
    }

    pub const fn zero() -> Self {
        Self::ZERO
    }

    pub const fn is_zero(&self) -> bool {
        self.numerator == 0
    }

    pub const fn is_positive(&self) -> bool {
        self.numerator > 0
    }

    pub const fn is_negative(&self) -> bool {
        self.numerator < 0
    }

    pub const fn numerator(&self) -> i128 {
        self.numerator
    }

    pub const fn denominator(&self) -> i128 {
        self.denominator
    }

    pub fn checked_add(&self, rhs: &Self) -> Option<Self> {
        let left = self
            .denominator
            .checked_mul(rhs.numerator)?
            .checked_add(rhs.denominator.checked_mul(self.numerator)?)?;
        let right = self.denominator.checked_mul(rhs.denominator)?;
        Self::new(left, right).ok()
    }

    pub fn checked_sub(&self, rhs: &Self) -> Option<Self> {
        let negated = Self::new(rhs.numerator.checked_neg()?, rhs.denominator).ok()?;
        self.checked_add(&negated)
    }

    pub fn checked_mul(&self, rhs: &Self) -> Option<Self> {
        Self::new(
            self.numerator.checked_mul(rhs.numerator)?,
            self.denominator.checked_mul(rhs.denominator)?,
        )
        .ok()
    }

    pub fn checked_div(&self, rhs: &Self) -> Option<Self> {
        if rhs.is_zero() {
            return None;
        }
        Self::new(
            self.numerator.checked_mul(rhs.denominator)?,
            self.denominator.checked_mul(rhs.numerator)?,
        )
        .ok()
    }

    /// Compares two normalized rationals without silently falling back when
    /// cross multiplication would overflow.
    pub fn try_cmp(&self, other: &Self) -> Refused<Ordering> {
        let left = self
            .numerator
            .checked_mul(other.denominator)
            .ok_or_else(|| {
                Refusal::owned(
                    RefusalKind::OutOfDeclaredRange,
                    "comparing two rationals overflowed i128, so the order is not known",
                )
            })?;
        let right = other.numerator.checked_mul(self.denominator).ok_or_else(|| {
            Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                "comparing two rationals overflowed i128, so the order is not known",
            )
        })?;
        Ok(left.cmp(&right))
    }

    /// Rounds to a declared quantum, using the nearest multiple and accepting
    /// ties toward zero. The distance check is in the original quantity units.
    pub fn round_to_multiple(&self, quantum: &Self, tolerance: &Self) -> Refused<Self> {
        let overflow = || {
            Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                "rounding a rational overflowed i128",
            )
        };
        if quantum.is_zero() || quantum.is_negative() || tolerance.is_negative() {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                "rounding quantum and tolerance must be a positive and a non-negative quantity",
            ));
        }
        let ratio_numerator = self.numerator.checked_mul(quantum.denominator).ok_or_else(overflow)?;
        let ratio_denominator = self
            .denominator
            .checked_mul(quantum.numerator)
            .ok_or_else(overflow)?;
        let quotient = ratio_numerator.checked_div(ratio_denominator).ok_or_else(overflow)?;
        let remainder = ratio_numerator.checked_rem(ratio_denominator).ok_or_else(overflow)?;
        let absolute_remainder = remainder.unsigned_abs();
        if absolute_remainder != 0 && absolute_remainder > i128::MAX as u128 {
            return Err(overflow());
        }
        let distance = Self::new(absolute_remainder as i128, ratio_denominator)?
            .checked_mul(quantum)
            .ok_or_else(overflow)?;
        if distance.try_cmp(tolerance)? == Ordering::Greater {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!(
                    "rounding {self:?} to a multiple of {quantum:?} moves it {distance:?}, past the \
                     declared tolerance {tolerance:?}"
                ),
            ));
        }
        let doubled = absolute_remainder.saturating_mul(2);
        let rounded = if doubled > ratio_denominator as u128 {
            if remainder > 0 {
                quotient.checked_add(1)
            } else {
                quotient.checked_sub(1)
            }
        } else {
            Some(quotient)
        }
        .ok_or_else(overflow)?;
        quantum.checked_mul(&Self::from_integer(rounded)).ok_or_else(overflow)
    }

    /// Refuses a quantity that may not be negative, naming the field it was.
    pub fn require_nonnegative(&self, field: &str) -> Refused<()> {
        if self.is_negative() {
            Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!("{field} is {}, and a negative quantity is a wrong one", self.numerator),
            ))
        } else {
            Ok(())
        }
    }
}

fn gcd(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_zero_numerator_normalizes_to_exactly_zero() {
        let value = Rational::new(0, 7).expect("a rational");
        assert_eq!(value, Rational::ZERO);
        assert!(value.is_zero());
    }

    #[test]
    fn a_non_positive_denominator_is_refused_rather_than_flipped() {
        for denominator in [0, -1, i128::MIN] {
            let refusal = Rational::new(1, denominator).expect_err("not a rational");
            assert_eq!(refusal.kind(), RefusalKind::RatiosDoNotSumToOne);
        }
    }

    #[test]
    fn equal_values_reduce_to_the_same_components() {
        assert_eq!(Rational::new(2, 4).unwrap(), Rational::new(1, 2).unwrap());
        assert_eq!(Rational::new(-2, 4).unwrap(), Rational::new(-1, 2).unwrap());
    }

    #[test]
    fn arithmetic_is_exact_and_keeps_the_sign() {
        let third = Rational::new(1, 3).unwrap();
        let total = third.checked_add(&third).unwrap().checked_add(&third).unwrap();
        assert_eq!(total, Rational::from_integer(1));
        // A subtraction that goes negative is representable, which is what a deficit is.
        let deficit = Rational::ZERO.checked_sub(&Rational::from_integer(5)).unwrap();
        assert!(deficit.is_negative());
        assert_eq!(deficit.numerator(), -5);
        assert_eq!(deficit.denominator(), 1);
    }

    #[test]
    fn a_comparison_that_would_overflow_refuses_rather_than_guessing() {
        let big = Rational::from_integer(i128::MAX);
        let small = Rational::new(1, i128::MAX).unwrap();
        let refusal = big.try_cmp(&small).expect_err("cross multiplication overflows");
        assert_eq!(refusal.kind(), RefusalKind::OutOfDeclaredRange);
    }

    #[test]
    fn multiplication_and_division_are_inverse_on_exact_values() {
        let value = Rational::new(3, 7).unwrap();
        let scaled = value.checked_mul(&Rational::from_integer(7)).unwrap();
        assert_eq!(scaled, Rational::from_integer(3));
        assert_eq!(value.checked_div(&value).unwrap(), Rational::from_integer(1));
        assert!(value.checked_div(&Rational::ZERO).is_none());
    }

    #[test]
    fn a_required_non_negative_quantity_names_the_field_it_was() {
        let negative = Rational::from_integer(-1);
        let refusal = negative.require_nonnegative("nourishment").expect_err("refused");
        assert!(refusal.detail().contains("nourishment"), "{}", refusal.detail());
        assert!(Rational::ZERO.require_nonnegative("nourishment").is_ok());
    }

    #[test]
    fn rounding_lands_on_the_nearest_multiple_or_refuses_past_the_tolerance() {
        let value = Rational::new(7, 2).unwrap(); // 3.5
        let quantum = Rational::from_integer(1);
        let tolerance = Rational::new(1, 2).unwrap();
        // Ties go toward zero, as the original states.
        assert_eq!(
            value.round_to_multiple(&quantum, &tolerance).unwrap(),
            Rational::from_integer(3)
        );
        // A tolerance that cannot cover the distance is a refusal, not a silent move.
        let too_tight = Rational::new(1, 100).unwrap();
        assert!(value.round_to_multiple(&quantum, &too_tight).is_err());
    }
}
