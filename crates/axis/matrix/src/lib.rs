//! **The calibration matrix: what the game verifies, and the recipe that fixes what it does not.**
//!
//! The user's statement of purpose, 2026-10-21, is this crate's specification:
//!
//! > *"create a calibration matrix through civilians axis db that will provide agents or people human
//! > readable code in the recipe format that can be used to fix the game's code, from the patches and
//! > tickets which should allow for quickly fixing the many hundreds of warning that happen when
//! > compiling the shploingus bin … This will require the full outlined api view in the shell window
//! > because that is how the game is verified through the 5 stars and the bronze-platinum tiers"*
//!
//! # The three parts, and nothing else
//!
//! | part | what it is | where |
//! |---|---|---|
//! | [`Segment`] | the seven systems the game is verified in — shell, ui, game, world, pattern, axis, tooling — closed, with a compile-time totality pin | [`segment`] |
//! | [`Cell`] | one `(segment, axis)` reading: the ladder value, its basis, its witness or `unchecked`, the five-star row, the requirements that compute its tier, and the ticket and patch behind it | [`matrix`] |
//! | [`Recipe`] | the readable line: **what to type, at which `file:line`, why, how it is checked, what moved, and where the witness lives** | [`recipe`] |
//!
//! [`Matrix`] holds the cells, refuses a partial table, and hands out the recipe. That is the whole
//! crate; the act of applying a recipe stays where the ticket-scoped grant is.
//!
//! # What this crate refuses to be
//!
//! * **Not a second store.** The chain is canonical (`voxel_model::chain`) and the security layer
//!   refuses a second record on disk. The matrix is a *projection* built at mount, addressed by key
//!   and dropped when the root moves. Nothing here opens a file for writing.
//! * **Not a score.** A star is the number of [`Predicate`] rows that **ran and passed**; a row that
//!   reports `passed` without `ran` is refused at the row, before any total, because that is exactly
//!   the defect `voxel-package/src/bridge.rs` quotes: *"a star lit by a predicate that did not run is
//!   the defect the five-star rule forbids."*
//! * **Not a second axis vocabulary.** The ladders, the tiers and the bare-token rule come from
//!   [`voxel_model::axis`]; the tier is **computed** by `certification_tier` over the cell's own
//!   requirements and is never stored, so a cell cannot carry a tier its requirements do not hold
//!   (`PATCH-042` A7).
//! * **Not an author.** A recipe cites a ticket that a chain event records (`Recipe::cite`), a site
//!   that exists (`Site::locate`), and a count the check actually reached (`Recipe::verify`). No
//!   ticket, no recipe.
//!
//! # Why the shell window is part of the requirement and not of this crate
//!
//! *"This will require the full outlined api view in the shell window"* — the report this crate
//! produces ([`Matrix::report`]) is one line per cell plus a measured tail, and drawing it is the
//! shell's job. `voxel-client` converts lines to elements; the view layer stays text-only, the same
//! division `voxel-package::apidoc` already keeps. **This crate depends on no UI crate**, so the
//! matrix cannot accidentally become a second place a pane is declared.
//!
//! # Where its rules are recorded
//!
//! `PATCH-042` §*The calibration matrix* states the shape and the gates C1–C6; `PATCH-043` states why
//! the tree is the first `segment = world` cell rather than the first thing to draw.
//!
//! [`PATCH-042`]: ../../../patches/PATCH-042-the-axis-layers-and-the-api-scrubber.md
//! [`PATCH-043`]: ../../../patches/PATCH-043-the-tree-is-the-first-mob.md

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod matrix;
pub mod reading;
pub mod recipe;
pub mod refusal;
pub mod segment;
pub mod star;
pub mod task;

pub use matrix::{Cell, Matrix};
pub use reading::Reading;
pub use recipe::{Fix, Recipe, Site};
pub use refusal::MatrixRefusal;
pub use segment::{Segment, SEGMENT_COUNT};
pub use star::{lit_star, Predicate, Star, FIVE_STAR_RULE, STAR_COUNT};
pub use task::Task;

#[cfg(test)]
mod tests {
    //! **The crate's own contract, exercised the way a caller would.** The module tests cover each
    //! part; these four are the rules that span parts, and they are here because a rule that only
    //! holds inside one module is a rule the composition can break.

    use super::*;
    use voxel_model::axis::{AxisKind, CertificationTier, EvidenceGrade, Grade};
    use voxel_model::refusal::RefusalKind;

    fn papers() -> Vec<Predicate> {
        vec![
            Predicate::held("site_exists"),
            Predicate::held("ticket_filed"),
            Predicate::held("count_fell"),
        ]
    }

    fn cell_for(segment: Segment, axis: AxisKind) -> Cell {
        let reading = match axis {
            AxisKind::EvidenceGrade => Reading::of_ladder(EvidenceGrade::B, "read back from disk")
                .witnessed("conformance/missing-docs.txt"),
            _ => Reading::of_ladder(Grade::B, "a unit grade read from disk")
                .witnessed("conformance/missing-docs.txt"),
        };
        Cell::new(segment, reading, Star::Three, papers(), "T-0954", "PATCH-042")
            .with_requirements(vec!["declared_provenance", "evidence_present"])
    }

    fn table() -> Vec<Cell> {
        Segment::ALL
            .into_iter()
            .map(|segment| cell_for(segment, AxisKind::EvidenceGrade))
            .collect()
    }

    #[test]
    fn the_whole_loop_ends_in_a_line_a_person_can_apply() {
        let matrix = Matrix::build(table()).expect("seven segments, seven readings");
        let fix = Fix::new(
            Site::new("crates/voxel-model/src/lib.rs", 1),
            "state the module's contract in one sentence",
            "missing-docs: voxel-model reports 954 warnings and this line closes one",
            "cargo check -p voxel-model -> warnings 954 -> 953",
            "conformance/missing-docs.txt",
            953,
        );
        let recipe = matrix
            .recipe(Segment::World, AxisKind::EvidenceGrade, fix)
            .expect("the world's evidence cell");
        let text = recipe.render().expect("a complete line");
        for field in ["segment=world", "axis=evidence_grade", "star=★★★☆☆", "tier=silver", "ticket=T-0954"] {
            assert!(text.contains(field), "{field} missing from:\n{text}");
        }
        assert_eq!(recipe.tier, CertificationTier::Silver, "two requirements, silver held");
    }

    #[test]
    fn a_star_cannot_outrun_its_rows_anywhere_in_the_composition() {
        let mut cells = table();
        cells[0].star = Star::Five;
        let refused = Matrix::build(cells).expect_err("three rows, four stars short");
        assert_eq!(refused.all()[0].kind(), RefusalKind::UncalibratedOracle);
        assert!(
            refused.all()[0].detail().contains(FIVE_STAR_RULE.split(':').next().unwrap()),
            "the refusal quotes the rule: {}",
            refused.all()[0].detail()
        );
    }

    #[test]
    fn every_segment_is_measured_by_the_table_a_builder_must_produce() {
        // The type-level pin and the runtime gate agree: `build` cannot accept a table with a hole.
        let mut cells = table();
        cells.retain(|cell| cell.segment != Segment::Tooling);
        let refused = Matrix::build(cells).expect_err("tooling is unmeasured");
        assert!(refused
            .all()
            .iter()
            .any(|refusal| refusal.detail().contains("segment tooling")));
        assert_eq!(Matrix::build(table()).unwrap().missing_segments(), Vec::new());
    }

    #[test]
    fn the_recipe_is_the_only_way_a_cell_becomes_an_act_and_it_carries_the_cell_facts() {
        let matrix = Matrix::build(table()).unwrap();
        let fix = Fix::new(
            Site::new("Cargo.toml", 1),
            "declare the crate",
            "a crate the workspace does not declare cannot be built",
            "cargo check -p voxel-axis-matrix",
            "Cargo.toml",
            0,
        );
        let recipe = matrix
            .recipe(Segment::Axis, AxisKind::EvidenceGrade, fix)
            .expect("the axis segment was read");
        assert_eq!(recipe.segment, Segment::Axis);
        assert_eq!(recipe.tier, CertificationTier::Silver);
    }
}
