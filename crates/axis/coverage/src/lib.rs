//! **Coverage, and the one audit that catches a dropped paragraph.**
//!
//! Two questions, and they are different:
//!
//! * **Does every ladder have a row?** The answer is [`COVERAGE`], [`COVERAGE_IS_TOTAL`] and
//!   [`check_coverage`], which compare each row against the axis's own declaration rather than
//!   against itself. A forged row — one that lists values the ladder does not have — is refused by
//!   name.
//! * **Does a text's tiling claim every character exactly once?** [`tile`] takes the source's
//!   character count and the spans a reader produced, and refuses a **gap**, an **overlap**, an
//!   **empty** span and a span **past the end**, each by position. A deleted middle paragraph is
//!   therefore a named failure at its own offset, not a shorter document.
//!
//! # Why the tiling audit lives beside coverage
//!
//! The coverage table and the partition audit answer the same shape of question one level apart:
//! a ladder set with a missing row and a text with a missing character are both "the table looked
//! complete because nothing compared it to its subject". The prose layer's `unit` crate consumes
//! [`tile`] so there is one audit, not one per renderer.
//!
//! # What it does not do
//!
//! It does not read a file, parse Markdown, or decide where a paragraph begins. A caller that
//! cannot say what its spans are has not produced a tiling yet; this module only judges the spans
//! it is handed.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub use voxel_axis_ladder::{
    check_coverage, CoverageRow, AXIS_COUNT, COVERAGE, COVERAGE_IS_TOTAL,
};

/// One half-open character range, `[start, end)`.
///
/// Character offsets rather than bytes, because the unit layer addresses what a reader sees and
/// a multi-byte codepoint must not be split by an arithmetic mistake in this crate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Span {
    /// The first character claimed.
    pub start: usize,
    /// One past the last character claimed.
    pub end: usize,
}

impl Span {
    /// A span over `[start, end)`.
    pub const fn new(start: usize, end: usize) -> Self {
        Span { start, end }
    }

    /// Whether the span claims no characters at all.
    pub const fn is_empty(self) -> bool {
        self.start >= self.end
    }

    /// How many characters it claims, saturating at zero for a reversed span.
    pub const fn len(self) -> usize {
        self.end.saturating_sub(self.start)
    }
}

/// A way a tiling failed, with the position that makes it actionable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TileError {
    /// No span claims the character at this offset.
    Gap {
        /// The first unclaimed character.
        at: usize,
    },
    /// Two spans claim the character at this offset.
    Overlap {
        /// The first character claimed twice.
        at: usize,
    },
    /// A span claims no characters.
    Empty {
        /// The span's index in the supplied slice.
        index: usize,
    },
    /// A span ends past the source's character count.
    PastEnd {
        /// The span's index in the supplied slice.
        index: usize,
        /// One past the last character the span claims.
        end: usize,
        /// The source's character count.
        limit: usize,
    },
}

impl TileError {
    /// A one-line description that names the position, never only the class.
    pub fn describe(&self) -> String {
        match self {
            TileError::Gap { at } => format!("gap at {at}: no unit claims this character"),
            TileError::Overlap { at } => format!("overlap at {at}: two units claim this character"),
            TileError::Empty { index } => format!("empty unit at index {index}: a span claims nothing"),
            TileError::PastEnd { index, end, limit } => format!(
                "unit {index} ends at {end}, past the end of the {limit}-character source"
            ),
        }
    }
}

/// **The partition audit.** `spans` must claim every character in `[0, chars_total)` exactly once.
///
/// The spans are sorted by start before the walk, so an input order is not a claim about coverage.
/// Every failure is collected rather than the first, because a tiling with two gaps is two things
/// to fix and stopping at the first hides the second.
pub fn tile(chars_total: usize, spans: &[Span]) -> Result<(), Vec<TileError>> {
    let mut errors = Vec::new();
    for (index, span) in spans.iter().enumerate() {
        if span.is_empty() {
            errors.push(TileError::Empty { index });
        }
        if span.end > chars_total {
            errors.push(TileError::PastEnd {
                index,
                end: span.end,
                limit: chars_total,
            });
        }
    }
    let mut ordered: Vec<Span> = spans.iter().copied().filter(|span| !span.is_empty()).collect();
    ordered.sort_by_key(|span| (span.start, span.end));
    let mut expected = 0usize;
    for span in ordered {
        if span.start > expected {
            errors.push(TileError::Gap { at: expected });
        } else if span.start < expected {
            errors.push(TileError::Overlap { at: span.start });
        }
        expected = expected.max(span.end);
    }
    if expected < chars_total {
        errors.push(TileError::Gap { at: expected });
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_one_table_is_total_and_a_forged_row_is_refused_by_name() {
        assert_eq!(COVERAGE.len(), AXIS_COUNT);
        assert_eq!(COVERAGE_IS_TOTAL.len(), AXIS_COUNT);
        check_coverage(&COVERAGE).expect("the declared table must describe every axis once");

        let mut forged = COVERAGE[0];
        forged.values = &["A", "B", "C", "E"];
        let refused = check_coverage(&[forged]).expect_err("a row must describe its own ladder");
        assert!(
            refused.all().iter().any(|refusal| refusal.detail().contains("evidence_grade")),
            "the refusal names the axis: {refused:?}"
        );
    }

    #[test]
    fn a_partition_that_claims_every_character_is_accepted() {
        let spans = [Span::new(0, 3), Span::new(3, 7), Span::new(7, 8)];
        assert_eq!(tile(8, &spans), Ok(()));
    }

    #[test]
    fn a_deleted_middle_paragraph_is_a_gap_at_its_own_offset() {
        let spans = [Span::new(0, 3), Span::new(7, 8)];
        let errors = tile(8, &spans).expect_err("the middle paragraph is missing");
        assert_eq!(errors, vec![TileError::Gap { at: 3 }]);
        assert!(errors[0].describe().contains("gap at 3"));
    }

    #[test]
    fn a_doubled_range_is_an_overlap_at_its_own_offset() {
        let spans = [Span::new(0, 4), Span::new(3, 8)];
        let errors = tile(8, &spans).expect_err("the same characters are claimed twice");
        assert_eq!(errors, vec![TileError::Overlap { at: 3 }]);
    }

    #[test]
    fn an_empty_unit_and_a_unit_past_the_end_are_named_by_position() {
        let spans = [Span::new(0, 2), Span::new(2, 2), Span::new(2, 9)];
        let errors = tile(8, &spans).expect_err("two separate refusals");
        assert!(errors.contains(&TileError::Empty { index: 1 }));
        assert!(errors.contains(&TileError::PastEnd { index: 2, end: 9, limit: 8 }));
        assert!(errors.iter().all(|error| error.describe().contains(' ')));
    }

    #[test]
    fn an_input_order_is_not_a_claim_about_coverage() {
        let spans = [Span::new(6, 8), Span::new(0, 3), Span::new(3, 6)];
        assert_eq!(tile(8, &spans), Ok(()));
    }
}
