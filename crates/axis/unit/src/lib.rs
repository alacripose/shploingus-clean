//! **The addressable unit, and the tiling that proves nothing was dropped.**
//!
//! A [`Unit`] is the smallest slice a claim can quote: one paragraph of a document, a public
//! item's doc block, or one chain event's detail. It carries the artifact's id, its index, a
//! half-open character range, a readable locator and the **exact text** at that range.
//!
//! # The audit is not optional
//!
//! [`Unit::tile_text`] builds the spans and then hands them to [`tile`] before it constructs a
//! single unit. A gap, an overlap, an empty span or a span past the end is refused with the
//! position, so removing a paragraph cannot produce a shorter document that still reads as a
//! complete one. The concatenation of every unit is the artifact, character for character; the
//! test below checks that equality rather than a count.
//!
//! # Ids
//!
//! `unit_<sha40>` is the first forty hex digits of a domain-separated sha256 over the artifact id,
//! the index, the range and the text. Two runs over the same bytes produce the same ids, and a
//! moved paragraph moves its id because the range moved — which is what makes a claim id stable
//! only while its subject is.
//!
//! # What this crate does not do
//!
//! It does not decide whether a paragraph is a claim. [`voxel_axis_claim`] does that; this layer
//! only says where the text is and that all of it is addressed.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub use voxel_axis_coverage::{tile, Span, TileError};

use voxel_model::Identity;

/// The domain tag under which a unit id is hashed, so a unit id cannot collide with another kind.
const UNIT_DOMAIN: &str = "shploingus/axis/unit";

/// How many hex digits an id keeps. The record's other ids use forty, and a readable report line
/// is the point of the shorter form.
const ID_DIGITS: usize = 40;

/// One addressable slice of one source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unit {
    /// `unit_` followed by forty hex digits of the unit's own identity.
    pub id: String,
    /// The artifact's id, stable for the bytes it was read from.
    pub artifact: String,
    /// The paragraph's index inside the artifact, from zero.
    pub index: usize,
    /// The first character claimed, inclusive.
    pub start: usize,
    /// One past the last character claimed.
    pub end: usize,
    /// A human-readable locator for the slice.
    pub unit_ref: String,
    /// The source's characters at `[start, end)`, exactly.
    pub text: String,
}

impl Unit {
    /// Tile `text` into paragraphs, refusing unless every character is claimed exactly once.
    ///
    /// A paragraph ends after a blank line and includes that separator; the final paragraph
    /// runs to the end. The separator is therefore claimed once, by the paragraph before it,
    /// rather than being an unowned character or a unit with no content. A blank line is a
    /// paragraph's `\n\n` or a file's `\r\n\r\n`, and both are claimed whole — a CRLF file tiles
    /// into the same paragraphs as its LF twin rather than tiling after every `\n` and leaving
    /// the `\r` to the next paragraph's head.
    pub fn tile_text(artifact: &str, text: &str) -> Result<Vec<Unit>, Vec<TileError>> {
        if artifact.trim().is_empty() {
            return Err(vec![TileError::Empty { index: 0 }]);
        }
        let characters: Vec<char> = text.chars().collect();
        let mut spans = Vec::new();
        let mut start = 0usize;
        let mut index = 0usize;
        while index + 1 < characters.len() {
            let separator = if characters[index] == '\n' && characters[index + 1] == '\n' {
                2
            } else if index + 3 < characters.len()
                && characters[index] == '\r'
                && characters[index + 1] == '\n'
                && characters[index + 2] == '\r'
                && characters[index + 3] == '\n'
            {
                4
            } else {
                index += 1;
                continue;
            };
            spans.push(Span::new(start, index + separator));
            start = index + separator;
            index += separator;
        }
        if start < characters.len() {
            spans.push(Span::new(start, characters.len()));
        }
        tile(characters.len(), &spans)?;
        Ok(spans
            .into_iter()
            .enumerate()
            .map(|(index, span)| {
                let text: String = characters[span.start..span.end].iter().collect();
                let id = Unit::id_of(artifact, index, span, &text);
                Unit {
                    id,
                    artifact: artifact.to_string(),
                    index,
                    start: span.start,
                    end: span.end,
                    unit_ref: format!("{artifact}@{}..{}", span.start, span.end),
                    text,
                }
            })
            .collect())
    }

    /// The deterministic id of one slice.
    fn id_of(artifact: &str, index: usize, span: Span, text: &str) -> String {
        let index = index.to_string();
        let start = span.start.to_string();
        let end = span.end.to_string();
        let digest = Identity::of_fields(
            UNIT_DOMAIN,
            &[
                artifact.as_bytes(),
                index.as_bytes(),
                start.as_bytes(),
                end.as_bytes(),
                text.as_bytes(),
            ],
        )
        .to_string();
        let body: String = digest
            .trim_start_matches("sha256:")
            .chars()
            .take(ID_DIGITS)
            .collect();
        format!("unit_{body}")
    }

    /// The source text this unit claims, in order, as a reader can reassemble it.
    pub fn concatenate(units: &[Unit]) -> String {
        units.iter().map(|unit| unit.text.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = "# One\n\nThe first paragraph.\n\nA second paragraph\nspans two lines.\n";

    #[test]
    fn every_character_is_claimed_exactly_once_and_the_unit_text_is_the_source() {
        let units = Unit::tile_text("artifact_demo", TEXT).expect("the tiling is exact");

        assert_eq!(Unit::concatenate(&units), TEXT, "the units reassemble the source");
        for unit in &units {
            assert_eq!(
                TEXT.chars().skip(unit.start).take(unit.end - unit.start).collect::<String>(),
                unit.text,
                "{} quotes its own range",
                unit.id
            );
            assert!(unit.id.starts_with("unit_") && unit.id.len() == 45, "{}", unit.id);
        }
        let spans: Vec<Span> = units.iter().map(|unit| Span::new(unit.start, unit.end)).collect();
        assert_eq!(tile(TEXT.chars().count(), &spans), Ok(()));
    }

    #[test]
    fn two_runs_over_the_same_bytes_produce_the_same_ids() {
        let first = Unit::tile_text("artifact_demo", TEXT).expect("tiled");
        let second = Unit::tile_text("artifact_demo", TEXT).expect("tiled again");
        assert_eq!(
            first.iter().map(|unit| &unit.id).collect::<Vec<_>>(),
            second.iter().map(|unit| &unit.id).collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_deleted_middle_paragraph_is_refused_rather_than_shortening_the_source() {
        let units = Unit::tile_text("artifact_demo", TEXT).expect("tiled");
        let mut spans: Vec<Span> = units.iter().map(|unit| Span::new(unit.start, unit.end)).collect();
        spans.remove(1);
        let errors = tile(TEXT.chars().count(), &spans).expect_err("the second paragraph is gone");
        assert!(errors.iter().any(|error| matches!(error, TileError::Gap { .. })));
        assert!(errors.iter().any(|error| error.describe().contains("gap at")));
    }

    #[test]
    fn a_crlf_blank_line_ends_a_paragraph_as_one_separator() {
        let units = Unit::tile_text("artifact_crlf", "# One\r\n\r\nThe first.\r\n").expect("tiled");
        assert_eq!(units.len(), 2, "the CRLF blank line is a separator, not two paragraph ends");
        assert_eq!(units[0].text, "# One\r\n\r\n");
        assert_eq!(units[1].text, "The first.\r\n");
        assert_eq!(Unit::concatenate(&units), "# One\r\n\r\nThe first.\r\n");
        assert!(units[0].text.ends_with("\r\n\r\n"), "the separator is claimed whole");
        assert!(!units[1].text.starts_with('\r'), "the next paragraph does not keep the CR");
    }

    #[test]
    fn an_empty_artifact_has_no_units_and_an_empty_artifact_id_is_refused() {
        assert!(Unit::tile_text("artifact_empty", "").expect("an empty text is a valid tiling").is_empty());
        assert!(Unit::tile_text("  ", "text").is_err());
    }
}
