//! **One statement: what the matrix measured, where in the tree it stands, and how it was supported.**
//!
//! | field | where it comes from |
//! |---|---|
//! | `statement` | the cell's own facts — segment, the star's glyphs, the tier's word, the ticket |
//! | `cite` | the cell's witness file, at its first non-blank line |
//! | `reading` | the cell's reading, token and basis unchanged ([`Reading`]) |
//! | `support` | **computed** by [`support_of`], never a field a caller sets |
//!
//! # Why the support is computed
//!
//! `PATCH-042` A7: *"there is no field for a candidate's opinion of its own support."* A statement
//! carries `support_level direct` in its own text so a reader can see it, and the gate recomputes the
//! level from the citation and the reading and **refuses a disagreement** — so the document states
//! the reading and the tree decides it.
//!
//! | the citation resolves? | the reading has a witness? | the witness is the cited file? | support |
//! |---|---|---|---|
//! | yes | yes | yes | `direct` |
//! | yes | yes | no | `strongly_supported` |
//! | yes | no | — | `supported` |
//! | no | yes | — | `inferred` |
//! | no | no | — | `unverified` |
//!
//! # Why the citation is a line of the witness
//!
//! The cell's witness is a file, and `PATCH-042` gate 2 asks every claim to quote a `file:line`. The
//! line is the file's **first non-blank line** — the sentence a reader meets when they open it, which
//! is the file's own account of what it is — and the reproduction section re-reads the whole file by
//! digest, so a citation cannot drift away from its bytes without the gate naming it.

use std::path::Path;

use voxel_axis_matrix::{Cell, Reading, Site};
use voxel_model::axis::{AxisKind, EvidenceLevel};
use voxel_model::refusal::{RefusalKind, RefusalSet};

/// One statement of a package: a sentence, the place it stands on, the ladder reading, and the
/// support this machine could derive for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    statement: String,
    cite: Site,
    reading: Reading,
    support: EvidenceLevel,
}

impl Line {
    /// **The one constructor from a measured cell.** A cell that fails its own check, names no
    /// witness, holds no tier, or cites a file this machine cannot read is refused by name rather
    /// than written with a blank where the fact should be.
    pub fn of(root: &Path, cell: &Cell) -> Result<Line, RefusalSet> {
        if let Err(refusal) = cell.check() {
            return Err(refusal_set(
                refusal.kind(),
                format!("{}: {}", cell.segment.name(), refusal.detail()),
            ));
        }
        let witness = cell
            .reading
            .witness
            .as_deref()
            .filter(|witness| !witness.trim().is_empty())
            .ok_or_else(|| {
                refusal_set(
                    RefusalKind::NoReading,
                    format!(
                        "{}: the cell names no witness, so its statement would cite nothing",
                        cell.segment.name()
                    ),
                )
            })?;
        let tier = cell.tier().ok_or_else(|| {
            refusal_set(
                RefusalKind::UncalibratedOracle,
                format!(
                    "{}: the cell holds no certification tier, and a package statement is a reading \
                     rather than a claim",
                    cell.segment.name()
                ),
            )
        })?;
        let cite = Site::new(witness, first_statement(root, witness)?);
        let support = support_of(root, &cite, &cell.reading);
        let statement = format!(
            "{} {} {} {}",
            cell.segment.name(),
            cell.star.glyphs(),
            tier.as_str(),
            cell.ticket
        );
        let line = Line {
            statement,
            cite,
            reading: cell.reading.clone(),
            support,
        };
        line.check()
            .map_err(|(kind, detail)| refusal_set(kind, detail))?;
        Ok(line)
    }

    /// Read one statement back from the five fields a package line holds.
    ///
    /// The text cannot say how strongly it is supported — the token is read, then immediately
    /// checked against the ladder — and [`crate::Package::gate`] re-derives it from the tree.
    pub fn parse(fields: &[&str]) -> Result<Line, String> {
        let [statement, cite, reading, support, basis] = fields else {
            return Err(format!(
                "a statement is five tab-separated fields (what, where, which ladder, how strongly \
                 supported, why) and this one has {}",
                fields.len()
            ));
        };
        let statement = statement.trim();
        if statement.is_empty() {
            return Err("a statement with no words says nothing".to_string());
        }
        let (path, number) = cite
            .rsplit_once(':')
            .ok_or_else(|| format!("{cite:?} is not a `file:line`"))?;
        let number: u32 = number
            .parse()
            .map_err(|_| format!("{cite:?} has no line number"))?;
        if path.trim().is_empty() || number == 0 {
            return Err(format!(
                "{cite:?} does not name a line (lines are one-based)"
            ));
        }
        let axis = AxisKind::ALL
            .into_iter()
            .find(|axis| reading.starts_with(&format!("{} ", axis.name())))
            .ok_or_else(|| {
                format!(
                    "the reading {reading:?} names no ladder; a token is written `<axis> <member>`"
                )
            })?;
        let line = Line {
            statement: statement.to_string(),
            cite: Site::new(path, number),
            reading: Reading {
                axis,
                token: reading.to_string(),
                basis: basis.to_string(),
                witness: Some(path.to_string()),
            },
            support: EvidenceLevel::parse(
                support.strip_prefix("support_level ").unwrap_or(support),
            )
            .map_err(|refusal| refusal.to_string())?,
        };
        line.check().map_err(|(_, detail)| detail)?;
        Ok(line)
    }

    /// **The three ways a line can fail to be a reading.** Checked at construction and again at the
    /// gate, because a document can arrive from anywhere.
    pub fn check(&self) -> Result<(), (RefusalKind, String)> {
        self.reading
            .check()
            .map_err(|refusal| (refusal.kind(), refusal.detail().to_string()))?;
        let axis = self.reading.axis;
        let qualified = axis
            .declared_values()
            .iter()
            .any(|member| self.reading.token == format!("{} {member}", axis.name()));
        if !qualified {
            return Err((
                RefusalKind::MalformedRecord,
                format!(
                    "`{}` is not a member of the `{}` ladder, whose members are {}",
                    self.reading.token,
                    axis.name(),
                    axis.declared_values()
                        .iter()
                        .map(|member| format!("{} {member}", axis.name()))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ));
        }
        for (field, value) in [
            ("statement", self.statement.as_str()),
            ("basis", self.reading.basis.as_str()),
        ] {
            if value.contains(['\t', '\n', '\r']) {
                return Err((
                    RefusalKind::MalformedRecord,
                    format!(
                        "the {field} carries a tab or a newline, so the package would stop being one \
                         statement per line"
                    ),
                ));
            }
        }
        Ok(())
    }

    /// The statement's words.
    pub fn statement(&self) -> &str {
        &self.statement
    }

    /// The place it stands on.
    pub fn cite(&self) -> &Site {
        &self.cite
    }

    /// The ladder reading, token and basis.
    pub fn reading(&self) -> &Reading {
        &self.reading
    }

    /// How much of the statement this machine could see.
    pub fn support(&self) -> EvidenceLevel {
        self.support
    }

    /// The line as the package writes it: five tab-separated fields, no field optional.
    pub fn render(&self) -> String {
        format!(
            "{}\t{}\t{}\tsupport_level {}\t{}",
            self.statement,
            self.cite.render(),
            self.reading.token,
            self.support.as_str(),
            self.reading.basis
        )
    }
}

/// **The one place a statement's support is decided.** Total over the two facts a line has: whether
/// its citation resolves on this machine, and whether its reading carries a witness.
pub(crate) fn support_of(root: &Path, cite: &Site, reading: &Reading) -> EvidenceLevel {
    let resolves = cite.locate(root).is_ok();
    match (resolves, reading.is_checked()) {
        (true, true) if reading.witness.as_deref() == Some(cite.path.as_str()) => {
            EvidenceLevel::Direct
        }
        (true, true) => EvidenceLevel::StronglySupported,
        (true, false) => EvidenceLevel::Supported,
        (false, true) => EvidenceLevel::Inferred,
        (false, false) => EvidenceLevel::Unverified,
    }
}

/// The line a citation points at: the file's first non-blank line.
fn first_statement(root: &Path, path: &str) -> Result<u32, RefusalSet> {
    let text = std::fs::read_to_string(root.join(path)).map_err(|error| {
        refusal_set(
            RefusalKind::NoReading,
            format!(
                "the statement cites `{path}` and this machine cannot read it ({error}); a citation \
                 into a file nobody can open is not a citation"
            ),
        )
    })?;
    text.lines()
        .position(|line| !line.trim().is_empty())
        .map(|index| index as u32 + 1)
        .ok_or_else(|| {
            refusal_set(
                RefusalKind::NoReading,
                format!("`{path}` is empty, so the statement has no line to cite"),
            )
        })
}

pub(crate) fn refusal_set(kind: RefusalKind, detail: String) -> RefusalSet {
    let mut refusals = RefusalSet::new();
    refusals.refuse(kind, detail);
    refusals
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_axis_matrix::{Predicate, Star};
    use voxel_model::axis::EvidenceGrade;

    fn root() -> std::path::PathBuf {
        voxel_package::features::workspace_root(Path::new(env!("CARGO_MANIFEST_DIR")))
            .expect("this crate sits inside the workspace")
    }

    fn cell(witness: &str) -> Cell {
        let reading =
            Reading::of_ladder(EvidenceGrade::B, "read back from the tree").witnessed(witness);
        Cell::new(
            voxel_axis_matrix::Segment::Shell,
            reading,
            Star::Three,
            vec![
                Predicate::held("workspace_member"),
                Predicate::held("census_measured"),
                Predicate::held("declared_witness"),
            ],
            "T-0954",
            "PATCH-042",
        )
        .with_requirements(vec!["declared_provenance", "evidence_present"])
    }

    #[test]
    fn a_cell_becomes_one_line_citing_its_own_witness_and_carrying_its_own_reading() {
        let line =
            Line::of(&root(), &cell("crates/axis/matrix/src/lib.rs")).expect("a measured cell");
        assert_eq!(line.cite.path, "crates/axis/matrix/src/lib.rs");
        assert_eq!(
            line.cite.line, 1,
            "the file's first line is its own account"
        );
        assert_eq!(line.reading.token, "evidence_grade B");
        assert_eq!(line.support, EvidenceLevel::Direct);
        assert_eq!(line.statement, "shell ★★★☆☆ silver T-0954");
        assert_eq!(line.render().split('\t').count(), 5);
    }

    #[test]
    fn a_cell_with_no_witness_is_refused_rather_than_written_without_a_citation() {
        let reading = Reading::of_ladder(EvidenceGrade::C, "self report");
        let cell = Cell::new(
            voxel_axis_matrix::Segment::World,
            reading,
            Star::One,
            vec![Predicate::held("only")],
            "T-0954",
            "PATCH-042",
        );
        let refused = Line::of(&root(), &cell).expect_err("no witness, no line");
        assert!(
            refused.all()[0].detail().contains("unchecked"),
            "the matrix says why before this crate does: {}",
            refused.all()[0].detail()
        );
    }

    #[test]
    fn a_citation_into_a_file_that_is_not_on_disk_is_refused_by_name() {
        let refused = Line::of(&root(), &cell("crates/axis/matrix/src/no-such-file.rs"))
            .expect_err("a citation that points at nothing");
        assert!(
            refused.all()[0].detail().contains("no-such-file.rs"),
            "{}",
            refused.all()[0].detail()
        );
    }

    #[test]
    fn a_token_that_is_not_on_its_own_ladder_is_refused_by_the_line() {
        let line = Line {
            statement: "shell".to_string(),
            cite: Site::new("Cargo.toml", 1),
            reading: Reading {
                axis: AxisKind::CertificationTier,
                token: "certification_tier plutonium".to_string(),
                basis: "invented".to_string(),
                witness: Some("Cargo.toml".to_string()),
            },
            support: EvidenceLevel::Direct,
        };
        let (kind, detail) = line.check().expect_err("plutonium is not a tier");
        assert_eq!(kind, RefusalKind::MalformedRecord);
        assert!(detail.contains("bronze"), "{detail}");
    }

    #[test]
    fn a_field_with_a_tab_would_stop_being_one_statement_per_line_and_is_refused() {
        let line = Line {
            statement: "shell\tsplit".to_string(),
            cite: Site::new("Cargo.toml", 1),
            reading: Reading::of_ladder(EvidenceGrade::B, "read back").witnessed("Cargo.toml"),
            support: EvidenceLevel::Direct,
        };
        let (kind, detail) = line.check().expect_err("a tab is not a field value");
        assert_eq!(kind, RefusalKind::MalformedRecord);
        assert!(detail.contains("tab"), "{detail}");
    }
}
