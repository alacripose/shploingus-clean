//! **One cell's reading: the ladder, the value, the basis, and the witness — or the state that says
//! there is no witness.**
//!
//! # The token is qualified by construction, not by discipline
//!
//! Round 5's rule is *"a bare ladder token is a bug"*: `A` is an evidence grade, a unit grade and a
//! ticket origin at once, and `gold` is a certification tier and nothing else but is still a bug bare
//! (`voxel_model::axis::refuse_bare`). A reading that carried `"A"` and an `axis` field side by side
//! would be correct only as long as every caller passed the matching pair, and the first caller to
//! pass `("A", AxisKind::Grade)` would produce a cell that reads as verified and is about a different
//! ladder.
//!
//! So [`Reading::of_ladder`] takes the **ladder member itself** and writes the token from
//! `Ladder::AXIS` and the member's own name — `evidence_grade A`, `certification_tier gold`. The pair
//! cannot disagree, because there is only one of them. [`Reading::of_text`] is for a reading that is
//! not a ladder member at all (a count, a measurement, a `warnings 954 → 953`), and [`Reading::check`]
//! still refuses it if the text happens to *be* a bare ladder token.
//!
//! # A witness or nothing
//!
//! [`Reading::witness`] is `Option<&str>`, and the `None` case is a **first-class state**
//! (`unchecked`), not an empty string. `PATCH-042` S5: *"an `unchecked` cell is not a pass"* — and the
//! matrix refuses a star on an unwitnessed cell rather than printing a star next to a file nobody
//! opened.
//!
//! # The basis
//!
//! Every reading carries `basis: String` — *why* that value, in words — because `PATCH-042` A8 makes
//! the basis the thing that distinguishes a measurement from an assertion. An empty basis is refused
//! by name.

use voxel_model::axis::{refuse_bare, AxisKind, Ladder};

use crate::MatrixRefusal;

/// One axis reading for one segment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reading {
    /// The ladder this value lives on.
    pub axis: AxisKind,
    /// The recorded value, **qualified** when it is a ladder member: `evidence_grade A`.
    pub token: String,
    /// Why this value. Never empty; [`Reading::check`] refuses an empty basis.
    pub basis: String,
    /// The `file:line` (or command output) a reader can open to check the reading. `None` is the
    /// `unchecked` state, and it is not a pass.
    pub witness: Option<String>,
}

impl Reading {
    /// A reading of a ladder member. **The axis and the token are written from the same value**, so
    /// they cannot disagree — the pairing is one fact, not two.
    pub fn of_ladder<L: Ladder>(member: L, basis: impl Into<String>) -> Self {
        Reading {
            axis: L::AXIS,
            token: format!("{} {}", L::AXIS.name(), member.as_str()),
            basis: basis.into(),
            witness: None,
        }
    }

    /// A reading that is not a ladder member: a count, a command's output, a measurement.
    ///
    /// The axis is named by the caller because the value does not carry one, and
    /// [`Reading::check`] still refuses a `token` that is a bare ladder token.
    pub fn of_text(axis: AxisKind, token: impl Into<String>, basis: impl Into<String>) -> Self {
        Reading {
            axis,
            token: token.into(),
            basis: basis.into(),
            witness: None,
        }
    }

    /// The same reading, witnessed. Chainable, so a cell literal reads as one statement.
    pub fn witnessed(mut self, witness: impl Into<String>) -> Self {
        self.witness = Some(witness.into());
        self
    }

    /// Whether a witness was recorded. **`false` is `unchecked`, never `pass`.**
    pub fn is_checked(&self) -> bool {
        self.witness
            .as_deref()
            .is_some_and(|witness| !witness.trim().is_empty())
    }

    /// Refuse the three ways a reading can fail to be a reading: a bare token, no basis, and a
    /// witness that is present but empty (an empty string is not a witness).
    pub fn check(&self) -> Result<(), MatrixRefusal> {
        if refuse_bare(&self.token).is_err() {
            return Err(MatrixRefusal::BareToken {
                token: self.token.clone(),
                axis: self.axis.name(),
            });
        }
        if self.basis.trim().is_empty() {
            return Err(MatrixRefusal::NoBasis {
                cell: self.axis.name().to_string(),
            });
        }
        Ok(())
    }

    /// The reading as one report field: `evidence_grade A (witness: path:12)` or
    /// `evidence_grade A (unchecked)`.
    pub fn render(&self) -> String {
        match self.witness.as_deref() {
            Some(witness) if !witness.trim().is_empty() => {
                format!("{} (witness: {witness})", self.token)
            }
            _ => format!("{} (unchecked)", self.token),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_model::axis::{CertificationTier, EvidenceGrade, Grade};

    #[test]
    fn a_ladder_reading_qualifies_its_own_token() {
        let reading = Reading::of_ladder(EvidenceGrade::A, "external oracle: the byte count");
        assert_eq!(reading.axis, AxisKind::EvidenceGrade);
        assert_eq!(reading.token, "evidence_grade A");
        reading.check().expect("a qualified token is a reading");

        let tier = Reading::of_ladder(CertificationTier::Gold, "four of five requirements");
        assert_eq!(tier.token, "certification_tier gold");
        tier.check().expect("a qualified tier is a reading");
    }

    #[test]
    fn the_same_value_on_two_ladders_gives_two_qualified_tokens() {
        let evidence = Reading::of_ladder(EvidenceGrade::A, "strongest evidence grade");
        let unit = Reading::of_ladder(Grade::A, "a unit grade below S");
        assert_eq!(evidence.axis, AxisKind::EvidenceGrade);
        assert_eq!(unit.axis, AxisKind::Grade);
        assert_ne!(evidence.token, unit.token);
        assert_eq!(unit.token, "grade A");
    }

    #[test]
    fn a_bare_ladder_token_is_refused_even_from_text() {
        let bare = Reading::of_text(AxisKind::Grade, "A", "someone typed the letter");
        let refused = bare.check().expect_err("A is a bare token");
        assert!(refused.detail().contains("bare ladder token"), "got {refused}");
        let gold = Reading::of_text(AxisKind::CertificationTier, "gold", "typed bare");
        assert!(gold.check().is_err(), "gold is claimed by one ladder and still bare");
        let measured = Reading::of_text(AxisKind::Grade, "warnings 954 -> 953", "the check printed");
        measured.check().expect("a count is not a ladder token");
    }

    #[test]
    fn an_empty_basis_is_an_assertion_not_a_reading() {
        let reading = Reading::of_ladder(EvidenceGrade::B, "   ");
        let refused = reading.check().expect_err("no basis");
        assert!(refused.detail().contains("no basis"), "got {refused}");
    }

    #[test]
    fn no_witness_is_unchecked_and_never_a_pass() {
        let reading = Reading::of_ladder(EvidenceGrade::B, "read back from world state");
        assert!(!reading.is_checked());
        assert!(reading.render().ends_with("(unchecked)"), "got {}", reading.render());
        let checked = reading.witnessed("conformance/missing-docs.txt");
        assert!(checked.is_checked());
        assert!(checked.render().contains("witness: conformance/missing-docs.txt"));
    }

    #[test]
    fn an_empty_witness_is_the_unchecked_state_not_a_witness() {
        let reading = Reading::of_ladder(EvidenceGrade::C, "self report").witnessed("");
        assert!(
            !reading.is_checked(),
            "an empty witness names nothing, so the cell is unchecked"
        );
        assert!(reading.render().ends_with("(unchecked)"));
    }
}
