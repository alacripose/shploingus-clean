//! **The matrix: one cell per `(segment, axis)`, a report a person reads, and the recipe it hands
//! out.**
//!
//! # The database is a projection, and this type is the projection
//!
//! `PATCH-042`: *"The DB is a projection, not a second store."* The chain is canonical, the security
//! layer refuses a second record on disk, and this crate therefore **writes nothing**. A [`Matrix`] is
//! built from cells a caller assembled from the chain plus the patch files plus the census, and it is
//! dropped when the workspace root moves. It is a database in the sense that it is addressed by key
//! and answered in rows; it is not a file beside the chain.
//!
//! # The tier is computed, never stored
//!
//! [`Cell::tier`] calls [`voxel_model::axis::certification_tier`] over the requirements the cell
//! declares. There is **no `tier` field to set**, which is `PATCH-042` A7's rule — *the constructor's
//! signature is the test* — applied to certification. A cell that claims platinum holds platinum
//! because `reconstruction_verified` is among its satisfied requirements, and for no other reason.
//! The near-miss is refused: a cell holding no tier cannot be rendered into a recipe, because
//! absence of certification is a state and a recipe is an act.
//!
//! # `build` is where C1 bites
//!
//! [`Matrix::build`] refuses three things at once: a cell that fails its own check, two cells for one
//! pair, and **a segment in the table with no cell at all**. A partial matrix therefore cannot be
//! constructed by accident — `PATCH-042` C1 is a refusal at the door rather than a report someone
//! remembers to read.
//!
//! # What the report prints, and why its last line is a number
//!
//! One line per cell, grouped by segment, then a **measured tail**: how many segments, how many
//! cells, how many unchecked, and which tiers are held. The tail is the line a reader compares
//! between two runs, and it is computed from the cells rather than counted by hand.

use voxel_model::axis::{certification_tier, AxisKind, CertificationTier};
use voxel_model::refusal::RefusalSet;

use crate::reading::Reading;
use crate::recipe::{Fix, Recipe};
use crate::star::{self, Predicate, Star};
use crate::refusal::MatrixRefusal;
use crate::segment::Segment;

/// One cell: a segment's reading on one axis, the star its predicates light, the requirements it
/// satisfies, and the ticket and patch that put it there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cell {
    /// Which system this is about.
    pub segment: Segment,
    /// The reading: ladder, value, basis, and witness or `unchecked`.
    pub reading: Reading,
    /// The star claimed. [`Cell::check`] refuses one the predicates do not light.
    pub star: Star,
    /// The rows the star is counted from. **Named, so a refusal can name the one that did not run.**
    pub predicates: Vec<Predicate>,
    /// The certification requirements this cell satisfies, by their recorded names.
    pub requirements: Vec<&'static str>,
    /// The ticket this cell was read from.
    pub ticket: String,
    /// The patch that states the rule the cell is held to.
    pub patch: &'static str,
}

impl Cell {
    /// A cell with no certification requirements yet. **Bronze is not the default**: an empty
    /// requirement list holds no tier at all, and [`Cell::with_requirements`] is how a cell earns one.
    pub fn new(
        segment: Segment,
        reading: Reading,
        star: Star,
        predicates: Vec<Predicate>,
        ticket: impl Into<String>,
        patch: &'static str,
    ) -> Self {
        Cell {
            segment,
            reading,
            star,
            predicates,
            requirements: Vec::new(),
            ticket: ticket.into(),
            patch,
        }
    }

    /// The same cell, with the requirements it satisfies.
    pub fn with_requirements(mut self, requirements: Vec<&'static str>) -> Self {
        self.requirements = requirements;
        self
    }

    /// The pair this cell is keyed by.
    pub fn key(&self) -> (Segment, AxisKind) {
        (self.segment, self.reading.axis)
    }

    /// **The tier, computed from the requirements.** `None` when they hold none.
    pub fn tier(&self) -> Option<CertificationTier> {
        certification_tier(&self.requirements)
    }

    /// Whether the cell's reading was witnessed. An unwitnessed cell lights no star.
    pub fn is_checked(&self) -> bool {
        self.reading.is_checked()
    }

    /// **Nine ways a cell can lie, checked in the order a reader would ask them.**
    ///
    /// 1. the reading is a reading at all (bare token, no basis) — [`Reading::check`];
    /// 2. the predicates are honest (`passed` without `ran` is refused first);
    /// 3. the star is lit by rows that ran and passed — [`star::require`];
    /// 4. **an unwitnessed cell has no star**, because a star is a measurement and the cell records
    ///    none;
    /// 5. the ticket is present, and 6. the patch is present, because a cell with neither is a value
    ///    someone typed.
    pub fn check(&self) -> Result<(), MatrixRefusal> {
        self.reading.check()?;
        star::require(self.star, &self.predicates)?;
        if !self.is_checked() {
            return Err(MatrixRefusal::StarUnsupported {
                claimed: self.star.name(),
                detail: format!(
                    "the cell at {}/{} is unchecked: it names no witness, and an unchecked cell is \
                     not a pass (PATCH-042 S5). A star over a file nobody opened is a score",
                    self.segment.name(),
                    self.reading.axis.name()
                ),
            });
        }
        if self.ticket.trim().is_empty() {
            return Err(MatrixRefusal::Incomplete { field: "ticket" });
        }
        if self.patch.trim().is_empty() {
            return Err(MatrixRefusal::Incomplete { field: "patch" });
        }
        Ok(())
    }

    /// The cell's facts without its segment: **the half a table's value column holds.**
    ///
    /// Split from [`Cell::render`] rather than duplicated, because a pane that laid these out itself
    /// would be the second place the star is drawn as a number and the second place a missing tier
    /// is spelled. The glyphs, the tier's own word and the reading's own `unchecked` all come from
    /// the types.
    pub fn value(&self) -> String {
        format!(
            "{:<7} {:<9} {:<8} {}",
            self.star.glyphs(),
            self.tier().map_or("no-tier", |tier| tier.as_str()),
            self.ticket,
            self.reading.render()
        )
    }

    /// The cell as one report line: `shell  ★★★☆☆  silver  T-0954  evidence_grade A (witness: …)`.
    pub fn render(&self) -> String {
        format!("{:<8} {}", self.segment.name(), self.value())
    }
}

/// The calibration matrix. **A projection over cells, and it writes nothing.**
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Matrix {
    cells: Vec<Cell>,
}

impl Matrix {
    /// **Assemble a matrix without gating it** — every cell handed in, in the order handed in.
    ///
    /// The two constructors have two jobs and the split is deliberate. [`Matrix::build`] is the
    /// **gate**: it refuses, and a caller that gets a matrix has one it may act on. [`Matrix::assemble`]
    /// is the **display**: a surface that must show the bad rows *beside* the good ones cannot use a
    /// constructor that hands back nothing when one cell is wrong — an API window that went blank
    /// because one segment was unchecked would hide the very defect it exists to show.
    ///
    /// **The gate is not weakened by this.** [`Matrix::verify`] is the same check, on the assembled
    /// value, and [`Matrix::build`] is `verify(assemble(cells))` and nothing else — so a caller that
    /// wants the refusal asks for it, and one that wants the report asks for that. What cannot happen
    /// is a display path silently treating an assembled matrix as verified: `build` is still the only
    /// way to get a matrix out of the gate, and the report's own tail prints the unchecked count.
    pub fn assemble(cells: Vec<Cell>) -> Self {
        Matrix { cells }
    }

    /// **The gate, on an assembled matrix.** Every refusal is collected before returning, so a caller
    /// sees all of the damage rather than the first cell: a bad cell, a duplicate pair, and a segment
    /// with no cell all arrive together.
    pub fn verify(&self) -> Result<(), RefusalSet> {
        let cells = &self.cells;
        let mut refusals = RefusalSet::new();

        let mut seen: Vec<(Segment, AxisKind)> = Vec::with_capacity(cells.len());
        for cell in cells {
            if let Err(refusal) = cell.check() {
                refusals.refuse(refusal.kind(), refusal.detail());
            }
            let key = cell.key();
            if seen.contains(&key) {
                let refusal = MatrixRefusal::DuplicateCell {
                    cell: format!("{}/{}", key.0.name(), key.1.name()),
                };
                refusals.refuse(refusal.kind(), refusal.detail());
            } else {
                seen.push(key);
            }
        }

        for segment in Segment::ALL {
            if !cells.iter().any(|cell| cell.segment == segment) {
                let refusal = MatrixRefusal::SegmentWithoutACell {
                    segment: segment.name(),
                };
                refusals.refuse(refusal.kind(), refusal.detail());
            }
        }

        refusals.into_result(())
    }

    /// **Assemble and gate in one call** — the constructor a caller uses when a refused table is a
    /// refused table, and `verify(assemble(cells))` written once so the two cannot drift.
    pub fn build(cells: Vec<Cell>) -> Result<Self, RefusalSet> {
        let matrix = Matrix::assemble(cells);
        matrix.verify()?;
        Ok(matrix)
    }

    /// How many cells the matrix holds.
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    /// Whether it holds none. **An empty matrix is a matrix nothing measured**, and [`Self::build`]
    /// cannot produce one.
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// The cell for a pair, if it exists.
    pub fn cell(&self, segment: Segment, axis: AxisKind) -> Option<&Cell> {
        self.cells
            .iter()
            .find(|cell| cell.segment == segment && cell.reading.axis == axis)
    }

    /// The segments in the table with no cell. Empty for a matrix that came through [`Self::build`].
    pub fn missing_segments(&self) -> Vec<Segment> {
        Segment::ALL
            .into_iter()
            .filter(|segment| !self.cells.iter().any(|cell| cell.segment == *segment))
            .collect()
    }

    /// The axes read for one segment, in ladder order.
    pub fn axes_of(&self, segment: Segment) -> Vec<AxisKind> {
        let mut axes: Vec<AxisKind> = self
            .cells
            .iter()
            .filter(|cell| cell.segment == segment)
            .map(|cell| cell.reading.axis)
            .collect();
        axes.sort_unstable();
        axes.dedup();
        axes
    }

    /// The cells whose reading names no witness, as `segment/axis` words.
    pub fn unchecked(&self) -> Vec<String> {
        self.cells
            .iter()
            .filter(|cell| !cell.is_checked())
            .map(|cell| format!("{}/{}", cell.segment.name(), cell.reading.axis.name()))
            .collect()
    }

    /// **The report a person reads.** One line per segment's cells, then the measured tail.
    pub fn report(&self) -> Vec<String> {
        let mut lines = Vec::with_capacity(self.cells.len() + 2);
        lines.push(format!(
            "calibration matrix — {} segments, {} cells, {} unchecked",
            Segment::ALL.len(),
            self.cells.len(),
            self.unchecked().len()
        ));
        for segment in Segment::ALL {
            for axis in self.axes_of(segment) {
                if let Some(cell) = self.cell(segment, axis) {
                    lines.push(cell.render());
                }
            }
        }
        let mut tiers: Vec<&str> = self
            .cells
            .iter()
            .map(|cell| cell.tier().map_or("no-tier", |tier| tier.as_str()))
            .collect();
        tiers.sort_unstable();
        tiers.dedup();
        let held: Vec<String> = tiers
            .iter()
            .map(|tier| {
                let count = self
                    .cells
                    .iter()
                    .filter(|cell| cell.tier().map_or("no-tier", |t| t.as_str()) == *tier)
                    .count();
                format!("{tier} {count}")
            })
            .collect();
        lines.push(format!(
            "measured  {} cells · tiers: {} · unchecked: {}",
            self.cells.len(),
            held.join(", "),
            if self.unchecked().is_empty() {
                "0".to_string()
            } else {
                self.unchecked().join(", ")
            }
        ));
        lines
    }

    /// **The recipe line for a cell**, or a refusal naming the pair nobody read.
    pub fn recipe(&self, segment: Segment, axis: AxisKind, fix: Fix) -> Result<Recipe, RefusalSet> {
        match self.cell(segment, axis) {
            Some(cell) => Recipe::of(cell, fix),
            None => {
                let refusal = MatrixRefusal::MissingCell {
                    cell: format!("{}/{}", segment.name(), axis.name()),
                };
                let mut refusals = RefusalSet::new();
                refusals.refuse(refusal.kind(), refusal.detail());
                Err(refusals)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reading::Reading;
    use crate::recipe::{Fix, Site};
    use crate::star::Predicate;
    use voxel_model::axis::{EvidenceGrade, Grade};

    fn cell(segment: Segment, axis: AxisKind, star: Star) -> Cell {
        let reading = if axis == AxisKind::EvidenceGrade {
            Reading::of_ladder(EvidenceGrade::B, "read back from the tree").witnessed("Cargo.toml")
        } else {
            Reading::of_ladder(Grade::B, "a unit grade read from the tree").witnessed("Cargo.toml")
        };
        Cell::new(
            segment,
            reading,
            star,
            vec![
                Predicate::held("site_exists"),
                Predicate::held("ticket_filed"),
                Predicate::held("count_fell"),
            ],
            "T-0954",
            "PATCH-042",
        )
        .with_requirements(vec!["declared_provenance"])
    }

    fn full_table() -> Vec<Cell> {
        let mut cells = Vec::new();
        for (index, segment) in Segment::ALL.into_iter().enumerate() {
            let axis = if index % 2 == 0 {
                AxisKind::EvidenceGrade
            } else {
                AxisKind::Grade
            };
            cells.push(cell(segment, axis, Star::Three));
        }
        cells
    }

    #[test]
    fn a_full_table_builds_and_reports_one_line_per_cell() {
        let cells = full_table();
        let count = cells.len();
        let segments = Segment::ALL.len();
        let matrix = Matrix::build(cells).expect("a full table builds");
        assert_eq!(matrix.len(), count);
        assert!(matrix.missing_segments().is_empty());
        let report = matrix.report();
        assert_eq!(report.len(), 1 + count + 1, "header, cells, tail: {report:?}");
        assert!(
            report[0].contains(&format!("{segments} segments, {count} cells, 0 unchecked")),
            "got {}",
            report[0]
        );
        assert!(
            report
                .last()
                .unwrap()
                .starts_with(&format!("measured  {count} cells")),
            "got {:?}",
            report.last()
        );
        assert!(
            report
                .last()
                .unwrap()
                .contains(&format!("bronze {count}")),
            "got {:?}",
            report.last()
        );
    }

    #[test]
    fn a_segment_with_no_cell_is_refused_by_name() {
        let mut cells = full_table();
        cells.retain(|cell| cell.segment != Segment::Shell);
        let refused = Matrix::build(cells).expect_err("the shell is unmeasured");
        let detail = refused.all()[0].detail();
        assert!(detail.contains("segment shell has no cell"), "got {detail}");
        assert_eq!(refused.all()[0].kind(), voxel_model::refusal::RefusalKind::UncoveredAxis);
    }

    #[test]
    fn two_cells_for_one_pair_are_refused() {
        let mut cells = full_table();
        let duplicate = cell(Segment::Shell, AxisKind::EvidenceGrade, Star::Three);
        cells.push(duplicate);
        let refused = Matrix::build(cells).expect_err("one pair, two numbers");
        assert!(
            refused.all().iter().any(|refusal| refusal.detail().contains("shell/evidence_grade")),
            "got {:?}",
            refused.all()
        );
    }

    #[test]
    fn a_cell_whose_star_its_predicates_do_not_light_is_refused_at_the_door() {
        let mut cells = full_table();
        for cell in &mut cells {
            if cell.segment == Segment::World {
                cell.star = Star::Five;
            }
        }
        let refused = Matrix::build(cells).expect_err("three rows cannot light five stars");
        let detail = refused.all()[0].detail();
        assert!(detail.contains("3 predicates ran and passed"), "got {detail}");
        assert!(detail.contains("five-star rule"), "got {detail}");
    }

    #[test]
    fn an_unwitnessed_cell_has_no_star() {
        let mut cells = full_table();
        for cell in &mut cells {
            if cell.segment == Segment::Pattern {
                cell.reading.witness = None;
            }
        }
        let refused = Matrix::build(cells).expect_err("a star over no witness");
        let detail = refused.all()[0].detail();
        assert!(detail.contains("unchecked"), "got {detail}");
        assert!(detail.contains("S5"), "got {detail}");
    }

    #[test]
    fn the_tier_is_computed_from_the_requirements_and_never_stored() {
        let one = cell(Segment::World, AxisKind::EvidenceGrade, Star::Three);
        assert_eq!(one.tier(), Some(CertificationTier::Bronze));
        let two = one.clone().with_requirements(vec!["declared_provenance", "evidence_present"]);
        assert_eq!(two.tier(), Some(CertificationTier::Silver));
        let none = one.with_requirements(Vec::new());
        assert_eq!(none.tier(), None, "absence of certification is a state");
        let out_of_order = cell(Segment::World, AxisKind::EvidenceGrade, Star::Three)
            .with_requirements(vec!["gate_cleared"]);
        assert_eq!(
            out_of_order.tier(),
            None,
            "a later band without the earlier ones is a maximum, not a threshold"
        );
    }

    #[test]
    fn a_recipe_is_handed_out_for_a_pair_that_has_a_cell() {
        let matrix = Matrix::build(full_table()).unwrap();
        let fix = Fix::new(
            Site::new("crates/voxel-model/src/lib.rs", 1),
            "add the missing contract sentence",
            "missing-docs: one of 954 warnings",
            "cargo check -p voxel-model -> warnings 954 -> 953",
            "conformance/missing-docs.txt",
            953,
        );
        let recipe = matrix
            .recipe(Segment::World, AxisKind::Grade, fix.clone())
            .expect("the world/grade cell exists");
        assert_eq!(recipe.segment, Segment::World);
        assert_eq!(recipe.axis, AxisKind::Grade);
        let text = recipe.render().unwrap();
        assert!(text.contains("segment=world"), "got {text}");
        assert!(text.contains("axis=grade"), "got {text}");

        let refused = matrix
            .recipe(Segment::World, AxisKind::CertificationTier, fix)
            .expect_err("no cell was read for that pair");
        assert!(refused.all()[0].detail().contains("world/certification_tier"), "got {:?}", refused.all());
    }

    #[test]
    fn the_report_groups_by_segment_and_names_what_is_missing() {
        // A partial matrix is refused at `build`, so a caller that holds cells directly is how the
        // report's own accessors are exercised — which is why `unchecked` and `missing_segments` are
        // public and not test-only.
        let cells = full_table();
        let matrix = Matrix {
            cells: cells
                .into_iter()
                .filter(|cell| cell.segment != Segment::Ui)
                .collect(),
        };
        assert_eq!(matrix.unchecked(), Vec::<String>::new());
        assert_eq!(matrix.missing_segments(), vec![Segment::Ui]);
        assert_eq!(matrix.axes_of(Segment::World), vec![AxisKind::Grade]);
        assert_eq!(matrix.axes_of(Segment::Shell), vec![AxisKind::EvidenceGrade]);
    }
}
