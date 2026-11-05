//! **The walk and the report: the one place a calibration reading is produced from the tree.**
//!
//! `PATCH-042` describes ten axis crates; the matrix is one of them, and it deliberately receives
//! its cells rather than deriving them — [`voxel_axis_matrix::Matrix::assemble`] is a projection
//! and it says so. That left the API window showing a table somebody handed in. This crate is the
//! producer: [`verify`] reads the evidence the tree already holds and emits **one cell for each of
//! the seven segments**, with the source of every row in the cell's own basis.
//!
//! # Four sources, and nothing invented
//!
//! | source | what is read | where |
//! |---|---|---|
//! | [`Source::Workspace`] | the member list and each member's package name | `Cargo.toml` |
//! | [`Source::Census`] | the six-axis census and the missing-docs pins | `voxel-package`, `conformance/missing-docs.txt` |
//! | [`Source::Conformance`] | the prose frame, re-measured on this read | `conformance/prose-frame.txt` |
//! | [`Source::Ladder`] | the axis ladders' coverage rows | `voxel-model/src/axis.rs` |
//!
//! **Every source is a call into the crate that owns the instrument.** There is no second parser
//! here, no cached number, and no file opened for writing. A source that cannot run is kept as a
//! [`SourceReading::Refused`] with its reason, and the cell rows it would have lit stay unlit.
//!
//! # What "verified" means, exactly
//!
//! A cell is one segment's reading on the `evidence_grade` ladder, its five predicate rows, and the
//! requirement names those rows satisfy; the matrix computes the star and the tier. `evidence_grade
//! B` means *read back from the tree*: the census read this tree and its pins are current. `C` means
//! the reading is the crate's own artefact with no current census behind it. **Neither is a claim
//! that the segment behaves correctly** — it is a claim about the evidence being about this tree,
//! which is the only thing a verifier reading files can honestly say.
//!
//! A segment with no witness on disk, or with no row that ran and passed, is **declined**: no cell,
//! so the matrix reports the segment as uncovered. That is the refused state the matrix already has,
//! and it is never a zero-star or a pass. [`Report::lines`] prints every decline with the sources
//! that could not witness it, which is also how a cell the pane cannot fit stays reachable.
//!
//! # Where this sits
//!
//! `voxel-package` owns the census, the conformance instruments and the document renderer;
//! `voxel-axis-matrix` owns the segments, cells, stars, tiers and recipes. This crate depends on
//! both and **no UI crate**, so it cannot become a second place a pane is declared. The shell asks
//! it for cells and draws them through the matrix's own [`voxel_axis_matrix::Cell::value`].
//!
//! # Where its rules are recorded
//!
//! `PATCH-042` §The scrubber and §The axis layers names this crate, its four readers and the rule
//! that a source which cannot witness an axis produces a refusal rather than a pass.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod evidence;
pub mod prose;
pub mod ratchet;
pub mod report;
pub mod source;
pub mod subject;

pub use evidence::{Evidence, SourceReading};
pub use ratchet::{ratchet, Act, Baseline, BaselineRow, Must, Ratchet};
pub use prose::{scrub, Prose, DEFAULT_DOCUMENT};
pub use report::{Decline, Report, PATCH};
pub use source::{Source, SOURCES_ARE_TOTAL, SOURCE_COUNT};
pub use subject::{subject_of, Subject, SUBJECTS};

use std::path::Path;

/// **Read the tree and measure all seven segments**, filing the reading under `ticket`.
///
/// This is the whole public act: the caller names a root and the ticket it is filing on, and gets
/// back the cells it may draw plus the declines it must show. Nothing is written.
pub fn verify(root: &Path, ticket: &str) -> Report {
    Report::of(&Evidence::read(root), ticket)
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_axis_matrix::Segment;
    use voxel_model::axis::AxisKind;

    fn real_root() -> std::path::PathBuf {
        voxel_package::features::workspace_root(Path::new(env!("CARGO_MANIFEST_DIR")))
            .expect("this crate sits inside the workspace")
    }

    #[test]
    fn the_public_act_reads_a_tree_and_answers_with_one_cell_per_segment() {
        let report = verify(&real_root(), "T-0954");
        assert_eq!(report.cells().len(), Segment::ALL.len() + 1);
        assert!(report.declines().is_empty(), "{:?}", report.declines());
        assert!(report.verified());
        for segment in Segment::ALL {
            assert!(
                report
                    .matrix()
                    .cell(segment, AxisKind::EvidenceGrade)
                    .is_some(),
                "{} has no cell",
                segment.name()
            );
        }
        // The report is the whole table, not a sample: eight cells, every source, one tail.
        let lines = report.lines();
        assert!(
            lines.iter().filter(|line| line.contains('★')).count() == Segment::ALL.len() + 1,
            "{lines:#?}"
        );
        assert!(
            lines
                .last()
                .expect("a report has lines")
                .starts_with("sources"),
            "the source line stays last when nothing refused: {lines:#?}"
        );
    }

    #[test]
    fn a_tree_that_cannot_witness_anything_is_declined_rather_than_verified() {
        let scratch =
            std::env::temp_dir().join(format!("voxel-axis-verify-nowhere-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&scratch);
        std::fs::create_dir_all(&scratch).expect("an empty scratch tree");

        let report = verify(&scratch, "T-0954");
        assert!(report.cells().is_empty(), "no witness, no cell");
        assert_eq!(
            report.declines().len(),
            Segment::ALL.len() + 1,
            "seven segments and the artifact format"
        );
        assert!(!report.verified(), "an empty table is not a verified one");
        assert_eq!(
            report.sources().len(),
            SOURCE_COUNT,
            "every source is still named, including the ones that refused"
        );
        let _ = std::fs::remove_dir_all(&scratch);
    }
}
