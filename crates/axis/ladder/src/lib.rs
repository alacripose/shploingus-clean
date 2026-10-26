//! **The axis vocabulary, as the layers above see it.**
//!
//! PATCH-042 asked for ten axis crates and placed the vocabulary in the first of them. The
//! dependency direction this tree settled on on 2026-10-25 is narrower and is the whole of this
//! crate's shape:
//!
//! * **`voxel-model` sits under every axis crate.** Its `admission`, `laya` and `proof` modules
//!   already consume the ladders, so moving the vocabulary out of the model would make the model
//!   depend on the crate that depends on it — a cycle, not a layer.
//! * **The axis layer never depends on the model-bearing `voxel-von` or the MCP server.** A
//!   classifier's answer arrives as `EvidenceOrigin::Oracle`; a window that draws a ladder does not
//!   need to load an ONNX session to do it.
//!
//! So this crate is the **axis-layer name for the vocabulary**, not a second copy of it. It
//! re-exports the model's one `AxisKind`, its nine ladder types, the coverage table and the
//! refusals, and it names the two ladders PATCH-042 asked for, which the model's vocabulary
//! gained: [`ProseKind`] (a kind, with no order) and [`Establishment`] (checked, stale, unchecked
//! or refused). `voxel-package`'s feature rows now re-export [`Establishment`] from here rather
//! than keeping a second five-state enum.
//!
//! # What this crate refuses
//!
//! Every refusal is the model's own, re-exported rather than restated:
//!
//! * a comparison across two ladders ([`refuse_interchange`]);
//! * a ladder rendered as a percentage or a score, or a certification taken as a maximum
//!   ([`refuse_reduction`], [`prohibited_max_certification`]);
//! * a bare token (`A`, `fact`, `active`) ([`refuse_bare`]);
//! * an axis without a coverage row ([`check_coverage`]).
//!
//! # What it deliberately is not
//!
//! Not a parser, not a document, and not a walk. Those are the layers above: [`voxel_axis_unit`]
//! addresses bytes, [`voxel_axis_claim`] derives claims, [`voxel_axis_view`] renders them, and
//! `voxel-axis-verify` composes the report. The vocabulary stays here so none of them has to
//! re-declare a token.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub use voxel_model::axis::{
    ambiguous_tokens, bare_token_axes, certification_tier, check_coverage, prohibited_max_certification,
    refuse_bare, refuse_interchange, refuse_reduction, requirements, CoverageRow, Establishment,
    EvidenceGrade, EvidenceLevel, GateStatus, Grade, Ladder, OrgRank, ProseKind, Reduction,
    Requirement, AXIS_COUNT, COVERAGE, COVERAGE_IS_TOTAL,
};
pub use voxel_model::axis::AxisKind;
pub use voxel_model::ticket::EvidenceOrigin;

/// How many ladders this layer has, named once so a caller does not write a literal.
pub const LADDER_COUNT: usize = AXIS_COUNT;

/// Every ladder's recorded name, in the axis set's order.
///
/// The list is derived from [`AxisKind::ALL`] rather than written down, so an axis added without a
/// name is a compile error in the vocabulary rather than a shorter list here.
pub fn ladder_names() -> Vec<&'static str> {
    AxisKind::ALL.iter().map(|axis| axis.name()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_layer_has_nine_ladders_and_every_one_has_a_coverage_row() {
        assert_eq!(LADDER_COUNT, 9);
        assert_eq!(ladder_names().len(), AXIS_COUNT);
        assert_eq!(COVERAGE.len(), AXIS_COUNT);
        assert_eq!(COVERAGE_IS_TOTAL.len(), AXIS_COUNT);
        check_coverage(&COVERAGE).expect("the one coverage table describes every ladder once");
    }

    #[test]
    fn the_two_prose_ladders_carry_their_own_tokens_and_no_rank() {
        assert_eq!(ProseKind::ALL.len(), ProseKind::TOKENS.len());
        assert_eq!(Establishment::ALL.len(), Establishment::TOKENS.len());
        for kind in ProseKind::ALL {
            assert_eq!(ProseKind::parse(kind.as_str()), Ok(kind));
        }
        for state in Establishment::ALL {
            assert_eq!(Establishment::parse(state.slug()), Ok(state));
        }
        let _ = ProseKind::Fact;
    }

    #[test]
    fn a_bare_token_is_named_by_the_ladder_that_claims_it() {
        assert_eq!(
            bare_token_axes("A"),
            vec![AxisKind::EvidenceGrade, AxisKind::Grade, AxisKind::TicketEvidence]
        );
        let fact = refuse_bare("fact").expect_err("a bare prose kind is unqualified");
        assert!(fact.detail().contains("UNQUALIFIED"));
        assert!(fact.detail().contains("prose_kind"));
        let active = refuse_bare("active").expect_err("a bare establishment state is unqualified");
        assert!(active.detail().contains("UNQUALIFIED"));
        assert!(refuse_bare("the").is_ok(), "not a ladder token at all");
    }

    #[test]
    fn the_derivation_rung_is_declared_and_its_letter_is_its_own() {
        assert_eq!(EvidenceOrigin::Derived.letter(), "D");
        assert_eq!(EvidenceOrigin::ALL.len(), 4);
        assert_eq!(
            EvidenceOrigin::ALL.iter().map(|origin| origin.letter()).collect::<Vec<_>>(),
            EvidenceOrigin::TOKENS
        );
    }
}
