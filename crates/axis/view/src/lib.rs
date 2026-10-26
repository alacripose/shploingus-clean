//! **The document, where every line resolves to a claim or is refused.**
//!
//! A report that explains itself has started summarising. So a [`Line`] is not a prose sentence
//! somebody typed beside a claim: it **is** the claim's quote, its kind and its id, and
//! [`Document::validate`] re-checks all three. A line added without a claim is refused by name.
//!
//! The three sections are fixed:
//!
//! * **Asked for** — permissions and principles;
//! * **Declares** — definitions and facts;
//! * **Gates** — obligations, prohibitions, conditions and procedures.
//!
//! An empty section prints `none`, because a section that vanished would read as one that was
//! never declared. There is deliberately no fourth section: a section no producer can fill is a
//! placeholder, not a reading.
//! Rendering is deterministic: the same claims in the same order produce the same bytes. There is
//! no timestamp, no hash of the host and no iteration over a map.
//!
//! # What it is not
//!
//! It is not the API window and it does not draw. `voxel-client` turns lines into elements; this
//! crate depends on no UI crate, so the document cannot become a second place a pane is declared.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeSet;

use voxel_axis_claim::Claim;
use voxel_axis_ladder::ProseKind;

/// The three fixed sections, in printed order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SectionKind {
    /// What the text asked for.
    AskedFor,
    /// What the text declares.
    Declares,
    /// What the text gates.
    Gates,
}

impl SectionKind {
    /// Every section, in printed order.
    pub const ALL: [SectionKind; 3] = [
        SectionKind::AskedFor,
        SectionKind::Declares,
        SectionKind::Gates,
    ];

    /// The section's printed title.
    pub const fn title(self) -> &'static str {
        match self {
            SectionKind::AskedFor => "Asked for",
            SectionKind::Declares => "Declares",
            SectionKind::Gates => "Gates",
        }
    }

    /// Which section a prose kind belongs in.
    const fn of(kind: ProseKind) -> SectionKind {
        match kind {
            ProseKind::Permission | ProseKind::Principle => SectionKind::AskedFor,
            ProseKind::Definition | ProseKind::Fact => SectionKind::Declares,
            ProseKind::Obligation
            | ProseKind::Prohibition
            | ProseKind::Condition
            | ProseKind::Procedure => SectionKind::Gates,
        }
    }
}

/// One line of a document: the claim it quotes, verbatim.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    /// The rendered statement. **Always the claim's quote**, never a summary of it.
    pub text: String,
    /// The claim's own kind.
    pub kind: ProseKind,
    /// The claim's id, so a reader can trace the line back to its unit.
    pub claim: String,
}

impl Line {
    /// A line for a claim, with no rewriting.
    pub fn of(claim: &Claim) -> Line {
        Line {
            text: claim.quote.clone(),
            kind: claim.kind,
            claim: claim.id.clone(),
        }
    }
}

/// One section: a fixed heading and the lines that resolve into it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Section {
    /// Which of the three this is.
    pub kind: SectionKind,
    /// The lines, in claim order.
    pub lines: Vec<Line>,
}

/// A document of claims in three sections.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Document {
    /// The document's title, printed first.
    pub title: String,
    /// The three sections, always present.
    pub sections: Vec<Section>,
}

/// Why a document is not traceable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ViewRefusal {
    /// A line names a claim id the document does not carry.
    Unresolved {
        /// The claim id the line names.
        claim: String,
    },
    /// A line's text is not its claim's quote.
    Paraphrased {
        /// The claim id whose quote was rewritten.
        claim: String,
    },
    /// A line's kind is not its claim's kind.
    KindDiverged {
        /// The claim id whose kind moved.
        claim: String,
    },
    /// A claim the document carries has no line.
    MissingLine {
        /// The claim id with no line.
        claim: String,
    },
}

impl ViewRefusal {
    /// A one-line reason that names the id, not only the class.
    pub fn describe(&self) -> String {
        match self {
            ViewRefusal::Unresolved { claim } => {
                format!("line names {claim}, which is not a claim in this document")
            }
            ViewRefusal::Paraphrased { claim } => {
                format!("line text is not {claim}'s own quote")
            }
            ViewRefusal::KindDiverged { claim } => {
                format!("line kind is not {claim}'s own kind")
            }
            ViewRefusal::MissingLine { claim } => {
                format!("{claim} is carried by the document but has no line")
            }
        }
    }
}

impl Document {
    /// Build the three sections from claims. Every claim is placed by its kind, so no line can
    /// be invented here.
    pub fn from_claims(title: impl Into<String>, claims: &[Claim]) -> Document {
        let mut sections: Vec<Section> = SectionKind::ALL
            .into_iter()
            .map(|kind| Section {
                kind,
                lines: Vec::new(),
            })
            .collect();
        for claim in claims {
            let section = sections
                .iter_mut()
                .find(|section| section.kind == SectionKind::of(claim.kind))
                .expect("every prose kind belongs to one declared section");
            section.lines.push(Line::of(claim));
        }
        Document {
            title: title.into(),
            sections,
        }
    }

    /// **Every line resolves to a claim, and every claim has its line.**
    ///
    /// The set is built from the claims, so a duplicated claim id is a duplicate line rather than
    /// an ambiguity. A line whose id is absent, whose text was rewritten or whose kind moved is
    /// refused by the claim's id; a claim the document carries with no line at all is refused by
    /// its own id, so a dropped paragraph cannot render as a shorter document.
    pub fn validate(&self, claims: &[Claim]) -> Result<(), ViewRefusal> {
        let known: BTreeSet<&str> = claims.iter().map(|claim| claim.id.as_str()).collect();
        let mut represented: BTreeSet<&str> = BTreeSet::new();
        for section in &self.sections {
            for line in &section.lines {
                if !known.contains(line.claim.as_str()) {
                    return Err(ViewRefusal::Unresolved {
                        claim: line.claim.clone(),
                    });
                }
                let claim = claims
                    .iter()
                    .find(|claim| claim.id == line.claim)
                    .expect("the set was built from these claims");
                if line.text != claim.quote {
                    return Err(ViewRefusal::Paraphrased {
                        claim: line.claim.clone(),
                    });
                }
                if line.kind != claim.kind {
                    return Err(ViewRefusal::KindDiverged {
                        claim: line.claim.clone(),
                    });
                }
                represented.insert(line.claim.as_str());
            }
        }
        if let Some(missing) = claims
            .iter()
            .find(|claim| !represented.contains(claim.id.as_str()))
        {
            return Err(ViewRefusal::MissingLine {
                claim: missing.id.clone(),
            });
        }
        Ok(())
    }

    /// Render the document, after checking it against its claims.
    pub fn render_checked(&self, claims: &[Claim]) -> Result<String, ViewRefusal> {
        self.validate(claims)?;
        Ok(self.render())
    }

    /// Render the document deterministically.
    ///
    /// An empty section prints `none`, because a section that vanished would read as one that was
    /// never declared. No line is wrapped by this layer; a bounded pane is the pane's business and
    /// the measured tail is the shell's.
    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str(&self.title);
        out.push('\n');
        for section in &self.sections {
            out.push('\n');
            out.push_str("## ");
            out.push_str(section.kind.title());
            out.push('\n');
            if section.lines.is_empty() {
                out.push_str("none\n");
                continue;
            }
            for line in &section.lines {
                out.push_str(&format!(
                    "{}  {}\t{}\n",
                    line.kind.as_str(),
                    line.claim,
                    line.text
                        .chars()
                        .map(|character| match character {
                            '\r' | '\n' => ' ',
                            other => other,
                        })
                        .collect::<String>()
                ));
            }
        }
        out
    }

    /// How many lines the document carries.
    pub fn len(&self) -> usize {
        self.sections.iter().map(|section| section.lines.len()).sum()
    }

    /// Whether the document carries no lines at all. **Not an error**: a text can be all gaps.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_axis_unit::Unit;

    fn claims(texts: &[&str]) -> Vec<Claim> {
        texts
            .iter()
            .flat_map(|text| Unit::tile_text("artifact_test", text).expect("tiled"))
            .map(|unit| Claim::derive(&unit))
            .collect()
    }

    #[test]
    fn every_line_resolves_to_a_claim_and_the_render_is_deterministic() {
        let claims = claims(&[
            "The gate must refuse a missing witness.",
            "A witness is a file that exists.",
            "The gate may read a witness.",
        ]);
        let document = Document::from_claims("API view — test", &claims);
        document.validate(&claims).expect("every line is its claim's own quote");
        let first = document.render();
        let second = document.render();
        assert_eq!(first, second, "the same claims produce the same bytes");
        assert_eq!(document.len(), 3);
        assert!(first.contains("## Gates"));
        assert!(first.contains(&claims[0].id));
    }

    #[test]
    fn a_claim_without_a_line_is_refused_by_its_id() {
        let claims = claims(&[
            "The gate must refuse a missing witness.",
            "A witness is a file that exists.",
        ]);
        let mut document = Document::from_claims("API view — test", &claims);
        let declares = document
            .sections
            .iter_mut()
            .find(|section| section.kind == SectionKind::Declares)
            .expect("the fact belongs to Declares");
        declares.lines.retain(|line| line.claim != claims[1].id);

        let refused = document
            .validate(&claims)
            .expect_err("the claim is carried by the document but has no line");
        assert!(matches!(
            &refused,
            ViewRefusal::MissingLine { claim } if claim == &claims[1].id
        ));
        assert!(refused.describe().contains(&claims[1].id));
        assert!(refused.describe().contains("no line"));
    }

    #[test]
    fn a_crlf_quote_renders_without_a_carriage_return() {
        let unit = Unit::tile_text("artifact_crlf", "A claim.\r\n\r\nA second claim.\r\n")
            .expect("the paragraphs tile")
            .remove(0);
        assert!(unit.text.contains('\r'));
        let claim = Claim::derive(&unit);
        let document = Document::from_claims("title", &[claim.clone()]);
        let rendered = document
            .render_checked(&[claim])
            .expect("the line resolves to its claim");
        assert!(!rendered.contains('\r'), "a carriage return reached the page: {rendered:?}");
        assert!(rendered.contains("A claim."));
    }

    #[test]
    fn a_line_added_without_a_claim_is_refused_by_name() {
        let claims = claims(&["The gate must refuse a missing witness."]);
        let mut document = Document::from_claims("API view — test", &claims);
        document.sections[0].lines.push(Line {
            text: "a sentence nobody claimed".to_string(),
            kind: ProseKind::Fact,
            claim: "claim_0000000000000000000000000000000000000000".to_string(),
        });
        let refused = document.validate(&claims).expect_err("the line is not traceable");
        assert!(matches!(&refused, ViewRefusal::Unresolved { .. }));
        assert!(refused.describe().contains("claim_0000"));
    }

    #[test]
    fn a_paraphrased_line_and_a_moved_kind_are_both_refused_by_claim_id() {
        let claims = claims(&["The gate must refuse a missing witness."]);
        let mut paraphrased = Document::from_claims("API view — test", &claims);
        paraphrased.sections[2].lines[0].text = "The gate refuses missing witnesses.".to_string();
        let refused = paraphrased.validate(&claims).expect_err("a paraphrase is not a quote");
        assert!(matches!(&refused, ViewRefusal::Paraphrased { .. }));
        assert!(refused.describe().contains(&claims[0].id));

        let mut moved = Document::from_claims("API view — test", &claims);
        moved.sections[2].lines[0].kind = ProseKind::Fact;
        let refused = moved.validate(&claims).expect_err("the kind moved");
        assert!(matches!(&refused, ViewRefusal::KindDiverged { .. }));
    }

    #[test]
    fn the_three_sections_are_always_present_in_order_and_an_empty_one_prints_none() {
        let document = Document::from_claims("title", &[]);
        assert!(document.is_empty());
        let kinds: Vec<SectionKind> = document.sections.iter().map(|section| section.kind).collect();
        assert_eq!(kinds, SectionKind::ALL);
        let rendered = document.render();
        for kind in SectionKind::ALL {
            assert!(
                rendered.contains(&format!("## {}\nnone", kind.title())),
                "an empty section prints none: {rendered:?}"
            );
        }
    }
}
