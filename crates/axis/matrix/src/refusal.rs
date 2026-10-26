//! **The refusals this crate makes, each naming the value that was refused, and the central kind it
//! reports as.**
//!
//! `voxel_model::RefusalKind` is the workspace's vocabulary, and this crate adds no variant to it: a
//! central variant is a change to every exhaustive match in the tree, and the fact a matrix gate
//! reports is almost always one the centre already has a word for — a row that lies about itself is
//! [`RefusalKind::FabricatedFeature`], a segment nothing measured is
//! [`RefusalKind::UncoveredAxis`], a bare ladder token is [`RefusalKind::BareTokenRefused`].
//!
//! So this file is a **translation**, not a second vocabulary: one local name per refusal, in words a
//! reader can act on, and [`MatrixRefusal::kind`] mapping it onto the centre. A caller that filters
//! by kind gets the tree's word; a reader gets the sentence.
//!
//! **One refusal is deliberately absent.** There is no "tier claimed but held" *state*: the tier is
//! computed by [`voxel_model::axis::certification_tier`] and never stored, so a cell cannot carry a
//! tier that its requirements do not hold. The near-miss — a caller that computes a tier from its own
//! requirements and hands the number to the report — is [`crate::Matrix::build`]'s subject, and the
//! remedy is the same as `PATCH-042`'s A7: *the constructor's signature is the test*.

use voxel_model::refusal::{Refusal, RefusalKind};

/// What the matrix refuses, and why — in the crate's own words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MatrixRefusal {
    /// A cell's reading carries a bare ladder token — `A`, `gold`, `active` — instead of naming its
    /// ladder. Round 5's rule, delegated to [`voxel_model::axis::refuse_bare`] so there is one
    /// implementation of it.
    BareToken {
        /// The token that was offered.
        token: String,
        /// The ladder it was being read as.
        axis: &'static str,
    },
    /// A reading declared no basis. A value with no basis is an assertion, not a measurement
    /// (`PATCH-042` A8, applied to the matrix's cells).
    NoBasis {
        /// The cell, in `segment/axis` form.
        cell: String,
    },
    /// A cell declared a star and its predicates do not light it — the five-star rule's defect,
    /// quoted in `voxel-package/src/bridge.rs`: *"a star lit by a predicate that did not run is the
    /// defect the five-star rule forbids."*
    StarUnsupported {
        /// The star that was claimed.
        claimed: &'static str,
        /// How many predicates actually ran and passed, and which ones did not.
        detail: String,
    },
    /// A predicate reported `passed` while reporting that it never ran. **Its own refusal rather
    /// than a quiet "does not count"**, because this is the exact defect the star rule exists for and
    /// it must be visible at the predicate, not only in a total.
    PredicateDidNotRun {
        /// The predicate's name.
        predicate: &'static str,
    },
    /// A segment in the table has no cell, so nothing measures it. `PATCH-042` C1.
    SegmentWithoutACell {
        /// The segment, by its word.
        segment: &'static str,
    },
    /// A `(segment, axis)` pair has no cell, so a recipe was asked for a reading nobody made.
    ///
    /// Its own refusal rather than [`MatrixRefusal::SegmentWithoutACell`]: the segment *has* cells,
    /// and the axis is the hole. Sending a reader to "the shell is unmeasured" when the shell is
    /// measured on four axes and not a fifth is the wrong file.
    MissingCell {
        /// The pair, in `segment/axis` form.
        cell: String,
    },
    /// Two cells describe the same `(segment, axis)` pair, so one of them describes what the other
    /// already describes.
    DuplicateCell {
        /// The pair, in `segment/axis` form.
        cell: String,
    },
    /// A word was offered that the segment table does not declare.
    UnknownSegment {
        /// The word.
        word: String,
    },
    /// A word was offered that the star table does not declare.
    UnknownStar {
        /// The word.
        word: String,
    },
    /// A recipe field was empty. The recipe has six fields and a line missing one is not a shorter
    /// recipe, it is a different, unappliable document.
    Incomplete {
        /// The field that was empty.
        field: &'static str,
    },
    /// The site a recipe names does not exist, or does not reach the line it cites. `PATCH-042` C2.
    SiteAbsent {
        /// The path, relative to the workspace root.
        path: String,
        /// The cited line.
        line: u32,
        /// How many lines the file actually has (`0` when it could not be read).
        lines: usize,
    },
    /// A recipe cited a ticket that no chain event records. `PATCH-042` C2: *no invented ticket*.
    UncitedTicket {
        /// The ticket, as it was written in the line.
        ticket: String,
    },
    /// The check's receipt did not show the movement the recipe claimed. `PATCH-042` C4.
    CountDidNotFall {
        /// The count the recipe claimed the tree would reach.
        claimed: u32,
        /// The count the check actually printed.
        measured: u32,
    },
}

impl MatrixRefusal {
    /// The central kind this refusal reports as. **Total, and by `match`** — a new refusal has to
    /// choose a kind rather than inherit a default.
    pub const fn kind(&self) -> RefusalKind {
        match self {
            MatrixRefusal::BareToken { .. } => RefusalKind::BareTokenRefused,
            MatrixRefusal::NoBasis { .. } => RefusalKind::UncalibratedOracle,
            MatrixRefusal::StarUnsupported { .. } => RefusalKind::UncalibratedOracle,
            MatrixRefusal::PredicateDidNotRun { .. } => RefusalKind::UncalibratedOracle,
            MatrixRefusal::SegmentWithoutACell { .. } => RefusalKind::UncoveredAxis,
            MatrixRefusal::MissingCell { .. } => RefusalKind::UncoveredAxis,
            MatrixRefusal::DuplicateCell { .. } => RefusalKind::FabricatedFeature,
            MatrixRefusal::UnknownSegment { .. } => RefusalKind::UndeclaredName,
            MatrixRefusal::UnknownStar { .. } => RefusalKind::UndeclaredName,
            MatrixRefusal::Incomplete { .. } => RefusalKind::MalformedRecord,
            MatrixRefusal::SiteAbsent { .. } => RefusalKind::MissingFeature,
            MatrixRefusal::UncitedTicket { .. } => RefusalKind::UnsubstantiatedEvidence,
            MatrixRefusal::CountDidNotFall { .. } => RefusalKind::UnsubstantiatedEvidence,
        }
    }

    /// The sentence a report prints. **Every arm names the value**, because a refusal that says
    /// "invalid" sends its reader to the wrong file.
    pub fn detail(&self) -> String {
        match self {
            MatrixRefusal::BareToken { token, axis } => format!(
                "bare ladder token {token:?} read as {axis}; a bare token belongs to more than one \
                 ladder and means something different on each, so write the ladder with the token"
            ),
            MatrixRefusal::NoBasis { cell } => format!(
                "{cell}: the reading declared no basis. A value with no basis is an assertion, and \
                 an assertion is not a cell"
            ),
            MatrixRefusal::StarUnsupported { claimed, detail } => {
                format!("{claimed} claimed and {detail}")
            }
            MatrixRefusal::PredicateDidNotRun { predicate } => format!(
                "{predicate} reported `passed` while reporting that it never ran — a star lit by a \
                 predicate that did not run is the defect the five-star rule forbids"
            ),
            MatrixRefusal::SegmentWithoutACell { segment } => format!(
                "segment {segment} has no cell, so nothing in the matrix measures it. A segment the \
                 table declares and the matrix does not read is a system nobody verified"
            ),
            MatrixRefusal::MissingCell { cell } => format!(
                "{cell}: no cell was built for this pair, so there is no reading to turn into a \
                 recipe. A recipe is a cell's act, not a line someone typed"
            ),
            MatrixRefusal::DuplicateCell { cell } => format!(
                "{cell} appears twice; the second row describes what the first already described, so \
                 one of the two numbers is not about this tree"
            ),
            MatrixRefusal::UnknownSegment { word } => format!(
                "unknown segment {word:?}; the segments are shell, ui, game, world, pattern, axis and \
                 tooling"
            ),
            MatrixRefusal::UnknownStar { word } => format!(
                "unknown star {word:?}; a star is ★1 to ★5 and an absence of stars is not a star"
            ),
            MatrixRefusal::Incomplete { field } => format!(
                "the recipe line has no {field}. Every field is a fact a reader needs: what to type, \
                 why, how it is checked, where the witness is"
            ),
            MatrixRefusal::SiteAbsent { path, line, lines } => format!(
                "the recipe cites {path}:{line} and that file has {lines} lines; a line that points \
                 at nothing is a line nobody can apply"
            ),
            MatrixRefusal::UncitedTicket { ticket } => format!(
                "the recipe cites ticket {ticket} and no chain event records it. A recipe is a \
                 ticket's act, not a sentence someone typed"
            ),
            MatrixRefusal::CountDidNotFall { claimed, measured } => format!(
                "the recipe claimed the check would reach {claimed} and it printed {measured}; a \
                 star that rises on a count that did not move is a star lit by a claim"
            ),
        }
    }

    /// This refusal as the centre's own record, for a caller that has a [`RefusalSet`].
    ///
    /// [`RefusalSet`]: voxel_model::refusal::RefusalSet
    pub fn as_refusal(&self) -> Refusal {
        Refusal::owned(self.kind(), self.detail())
    }
}

impl std::fmt::Display for MatrixRefusal {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.detail())
    }
}

impl std::error::Error for MatrixRefusal {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_refusal_names_the_value_it_refused() {
        let cases: [MatrixRefusal; 13] = [
            MatrixRefusal::MissingCell {
                cell: "shell/org_rank".to_string(),
            },
            MatrixRefusal::BareToken {
                token: "gold".to_string(),
                axis: "certification_tier",
            },
            MatrixRefusal::NoBasis {
                cell: "world/evidence_grade".to_string(),
            },
            MatrixRefusal::StarUnsupported {
                claimed: "★4",
                detail: "3 predicates ran and passed".to_string(),
            },
            MatrixRefusal::PredicateDidNotRun {
                predicate: "warnings_fell",
            },
            MatrixRefusal::SegmentWithoutACell { segment: "shell" },
            MatrixRefusal::DuplicateCell {
                cell: "ui/grade".to_string(),
            },
            MatrixRefusal::UnknownSegment {
                word: "renderer".to_string(),
            },
            MatrixRefusal::UnknownStar {
                word: "six".to_string(),
            },
            MatrixRefusal::Incomplete { field: "witness" },
            MatrixRefusal::SiteAbsent {
                path: "crates/nope.rs".to_string(),
                line: 12,
                lines: 3,
            },
            MatrixRefusal::UncitedTicket {
                ticket: "T-0000".to_string(),
            },
            MatrixRefusal::CountDidNotFall {
                claimed: 953,
                measured: 954,
            },
        ];
        for refusal in cases {
            let detail = refusal.detail();
            assert!(!detail.is_empty(), "{refusal:?} printed nothing");
            assert!(
                detail.len() > 20,
                "{refusal:?} is too short to act on: {detail}"
            );
            assert_eq!(refusal.as_refusal().kind(), refusal.kind());
        }
    }

    #[test]
    fn the_kind_mapping_is_the_one_the_report_filters_by() {
        assert_eq!(
            MatrixRefusal::BareToken {
                token: "A".to_string(),
                axis: "grade"
            }
            .kind(),
            RefusalKind::BareTokenRefused
        );
        assert_eq!(
            MatrixRefusal::SegmentWithoutACell { segment: "world" }.kind(),
            RefusalKind::UncoveredAxis
        );
        assert_eq!(
            MatrixRefusal::UncitedTicket {
                ticket: "T-1".to_string()
            }
            .kind(),
            RefusalKind::UnsubstantiatedEvidence
        );
    }
}
