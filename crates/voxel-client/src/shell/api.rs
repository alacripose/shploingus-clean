//! **The API window's body: the document, and the matrix's rows, as the two crates render them.**
//!
//! `recipe::Content::ApiViewer` has been a declared window kind since the package format gained it,
//! and until this module it had no view: [`super::mount::declared_content_panel`] drew the
//! *declaration* of the window — its content kind, lane, record id — and said so in its note. The
//! user's requirement is the other thing: *"the full outlined api view in the shell window because
//! that is how the game is verified through the 5 stars and the bronze-platinum tiers"*.
//!
//! # Two sources, and this module is not a third
//!
//! | what the pane shows | who produced it |
//! |---|---|
//! | the subject's outline — *Asked for · Declares · Retiring requires read-back* | [`voxel_package::apidoc::render`], from a back-compiled [`Solution`] and the [`Recipe`] it was derived from |
//! | one row per `(segment, axis)` cell — stars, computed tier, ticket, reading | [`Matrix`]'s own [`Cell::value`] |
//! | every cell with no witness | [`Matrix::unchecked`] |
//! | every refusal the gate made | [`Matrix::verify`], kept as [`Refusal`]s rather than strings |
//!
//! **No string in this file spells a star, a tier or a segment.** `Cell::value` draws the five
//! slots and names the tier the cell's requirements hold; the document is the renderer's own text,
//! headings included. The pane composes those two outputs and nothing else — which is the difference
//! between a window and a second report that could disagree with the first.
//!
//! # The view is gated, the display is not
//!
//! [`Matrix::assemble`] keeps every cell a caller hands in, and [`Matrix::verify`] is the gate over
//! it; [`ApiView::of`] runs the gate and keeps its refusals **beside** the rows rather than throwing
//! the matrix away. A window that went blank because one segment was unchecked would hide the very
//! defect it exists to show, and `PATCH-042` S5's rule is that an unchecked cell is *reported*, never
//! read as a pass.
//!
//! # Where the cells come from, and when the view is dropped
//!
//! The cells are [`voxel_axis_verify::verify`]'s: the console's `file` verb asks it for the tree's
//! readings and hands this view the answers, so no `Reading` is constructed in the shell. A segment
//! the verifier could not witness contributes **no cell**, and the gate's `segment … has no cell`
//! refusal is what the pane shows for it.
//!
//! [`super::mount::Mount::set_api_view`] is the one owner, and
//! [`super::mount::Mount::set_workspace`] drops it: a view computed against one root is not a view of
//! the next, and the patch states that as the invalidation point rather than a refresh button. No
//! file is written here, and the matrix is a projection — the chain stays the record.

use voxel_axis_matrix::{Cell, Matrix, Segment};
use voxel_model::refusal::Refusal;
use voxel_ui::{Panel, Readout};

use super::mount::bounded_panel;

/// What the API window draws: **the document, the matrix, and what the gate refused.**
///
/// Fields are public because a caller may hold it across frames; there is nothing here that the two
/// sources did not produce.
#[derive(Clone, Debug, PartialEq)]
pub struct ApiView {
    /// The text [`voxel_package::apidoc::render`] produced for the selected subject.
    pub document: String,
    /// The cells the verifier measured — never invented here, and never a zero-star for a segment
    /// it could not witness.
    pub matrix: Matrix,
    /// The gate's verdict, kept so the pane can show it. **Empty means verified**, and the panel
    /// prints that as a count rather than as silence.
    pub refusals: Vec<Refusal>,
}

impl ApiView {
    /// A view over the renderer's own document and the verifier's own cells.
    ///
    /// The gate runs here, once, and its refusals are carried rather than returned: the caller wants
    /// a window, and the window is where a refusal is read.
    pub fn of(document: impl Into<String>, cells: Vec<Cell>) -> Self {
        let matrix = Matrix::assemble(cells);
        let refusals = matrix
            .verify()
            .err()
            .map(|set| set.all().to_vec())
            .unwrap_or_default();
        ApiView {
            document: document.into(),
            matrix,
            refusals,
        }
    }

    /// Whether every cell the caller handed in passed the gate. **The panel prints this**, so a
    /// window cannot look verified by having no rows.
    pub fn is_verified(&self) -> bool {
        self.refusals.is_empty() && !self.matrix.is_empty()
    }

    /// The rows the pane draws, in reading order: the document, then the cells, then what is
    /// unchecked, then the refusals.
    ///
    /// **Separated from [`api_panel`] so a test can read the rows without measuring a pane** — and
    /// so the order is asserted where it is decided rather than inferred from a drawing.
    pub fn rows(&self) -> Vec<Readout> {
        let mut rows: Vec<Readout> = Vec::new();

        // **The outline first, then the rows, then the detail.** A `## ` heading is a `section` row
        // so the spine is scanable; the document's body lines are gathered separately and placed
        // last, because they are the one part of the view a short pane may drop. That order is what
        // lets [`api_panel`] keep the headings and the cells while a long document loses its oldest
        // prose — the alternative keeps the prose and hides the rows the window exists for.
        for heading in self.headings() {
            rows.push(Readout::measured("section", heading));
        }
        // The document's own title line leads the pane, and the rest of its prose trails it.
        if let Some(title) = self.title_line() {
            rows.insert(0, Readout::measured("document", title));
        }
        if rows.is_empty() {
            // MD1's rule: an empty view says why. A renderer that produced nothing is a different
            // fact from a document with nothing to say, and both must not read as a broken window.
            rows.push(
                Readout::absent(
                    "document",
                    "no document was rendered for this subject, so there is nothing to read here",
                )
                .expect("the absence names itself"),
            );
        }

        // **The body, then the refusals, then the absences, then the cells.** The pane's bound drops
        // rows from the front of the droppable region, so this order *is* the priority: the document's
        // prose goes first, then the refusal details, then the unchecked names, and a cell — the
        // reading the window exists for — goes last. A pane that held only the spine would still be
        // a window; one that held prose and no cells would not.
        for line in self.body_lines() {
            rows.push(Readout::measured("document", line));
        }

        for refusal in &self.refusals {
            rows.push(Readout::measured(
                refusal.kind().name(),
                refusal.detail().to_string(),
            ));
        }

        // The named absences: *which* cells have no witness, short enough to survive a short pane
        // even when the paragraph that explains them does not.
        for name in self.matrix.unchecked() {
            rows.push(Readout::measured("unchecked", name));
        }

        // **One row per cell, and the cell's own rendering.** The label is the pair's recorded
        // words; the value is [`Cell::value`], so the star glyphs, the tier and the `unchecked`
        // reading are the matrix's.
        for segment in Segment::ALL {
            for axis in self.matrix.axes_of(segment) {
                if let Some(cell) = self.matrix.cell(segment, axis) {
                    rows.push(Readout::measured(
                        &format!("{}/{}", segment.name(), axis.name()),
                        cell.value(),
                    ));
                }
            }
        }

        rows
    }

    /// The document's first non-heading line — the renderer's own title, drawn above the outline.
    fn title_line(&self) -> Option<String> {
        self.document
            .lines()
            .find(|line| {
                let trimmed = line.trim();
                !trimmed.is_empty() && !trimmed.starts_with("## ")
            })
            .map(|line| line.trim_end().to_string())
    }

    /// The document's `## ` sections, in its own order and its own words.
    fn headings(&self) -> Vec<String> {
        self.document
            .lines()
            .filter_map(|line| line.trim_end().strip_prefix("## "))
            .map(str::to_string)
            .collect()
    }

    /// The document's prose, **everything that is neither its title nor a heading** — the part a
    /// short pane may drop.
    fn body_lines(&self) -> Vec<String> {
        let title = self.title_line();
        let mut seen_title = false;
        self.document
            .lines()
            .filter(|line| {
                let trimmed = line.trim_end();
                let trimmed = trimmed.trim();
                !trimmed.is_empty() && !trimmed.starts_with("## ")
            })
            .filter_map(|line| {
                let trimmed = line.trim_end().to_string();
                if !seen_title && Some(&trimmed) == title.as_ref() {
                    seen_title = true;
                    return None;
                }
                Some(trimmed)
            })
            .collect()
    }

    /// **How many rows a short pane must keep**: the title and the outline's own headings — the
    /// spine a reader navigates by. Everything after it is droppable, in the order [`ApiView::rows`]
    /// puts it.
    pub fn header_len(&self) -> usize {
        usize::from(self.title_line().is_some()) + self.headings().len()
    }

    /// The line the panel's note carries: **the measured tail**, computed rather than written.
    pub fn tail(&self) -> String {
        format!(
            "{} segment(s) · {} cell(s) · {} unchecked · {} refusal(s)",
            Segment::ALL.len(),
            self.matrix.len(),
            self.matrix.unchecked().len(),
            self.refusals.len()
        )
    }
}

/// The document for a back-compiled solution, **rendered by the module that owns the words**.
///
/// A thin call rather than a re-export so a caller cannot mistake this for a second renderer: the
/// only thing that happens here is that `apidoc` is asked, and the same function is what the gate's
/// document check reads.
pub fn document_for(
    solution: &voxel_package::compiler::Solution,
    recipe: &voxel_package::compendium::Recipe,
) -> String {
    voxel_package::apidoc::render(solution, recipe)
}

/// The API window's body, bounded to the pane that hosts it.
///
/// **Bounded from the front**, like the console's panel, and for the same reason: the outline's
/// headings are the header and are never dropped, so a long document loses its oldest body lines and
/// the note says how many it is holding. A pane that silently showed the tail of a document would
/// read as a document that began there.
pub fn api_panel(view: &ApiView, available_dp: f64) -> Panel {
    let mut rows = view.rows();
    // **The header is everything but the document's prose**, which [`ApiView::rows`] puts last: the
    // headings a reader navigates by, the cells, the absences and the refusals survive a short pane,
    // and only the body lines are dropped, oldest first.
    let header = view.header_len();
    let tail = view.tail();
    // **The note says how many rows it could not draw and where they are.** A bound that only
    // printed its total would leave a reader unable to tell a complete pane from a cut one; the
    // console logged the matrix's whole report when the view was measured, so the held rows are
    // reachable there rather than gone.
    bounded_panel(
        "api view",
        voxel_package::apidoc::TITLE,
        header,
        &mut rows,
        available_dp,
        |held| {
            if held == 0 {
                tail.clone()
            } else {
                format!("{tail} · {held} row(s) held — the console has the full report")
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_axis_matrix::{Predicate, Reading, Star};
    use voxel_model::axis::{AxisKind, CertificationTier, EvidenceGrade, Grade};

    /// The renderer's own output, and **one of its real headings** — the test asserts the pane keeps
    /// the renderer's words rather than any wording of this module's.
    fn document() -> String {
        format!(
            "{title}\n\n## Asked for\n\n- ticket: BLD-0187\n- windows: 2\n\n\
             ## Declares\n\n### window[api-viewer]\n\n- title: API view\n- content: api-viewer\n",
            title = voxel_package::apidoc::TITLE
        )
    }

    fn populated_cell() -> Cell {
        Cell::new(
            Segment::Shell,
            Reading::of_ladder(EvidenceGrade::B, "read back from the mounted arrangement")
                .witnessed("crates/voxel-client/src/shell/api.rs"),
            Star::Three,
            vec![
                Predicate::held("document_rendered"),
                Predicate::held("cells_assembled"),
                Predicate::held("gate_ran"),
            ],
            "BLD-0187",
            "PATCH-042",
        )
        .with_requirements(vec!["declared_provenance", "evidence_present"])
    }

    fn unchecked_cell() -> Cell {
        Cell::new(
            Segment::World,
            Reading::of_ladder(Grade::C, "a grade with no reading behind it"),
            Star::One,
            vec![Predicate::held("only")],
            "T-0954",
            "PATCH-042",
        )
        .with_requirements(vec!["declared_provenance"])
    }

    #[test]
    fn the_pane_keeps_the_documents_own_sections_and_lines() {
        let view = ApiView::of(document(), Vec::new());
        let rows = view.rows();
        assert_eq!(rows[0].label, "document");
        assert_eq!(rows[0].value, voxel_package::apidoc::TITLE);
        assert!(
            rows.iter().position(|row| row.label == "section") < rows.iter().position(|row| row.value.contains("- windows: 2")),
            "the outline leads and the body trails: {rows:?}"
        );
        assert!(
            rows.iter().any(|row| row.label == "section" && row.value == "Asked for"),
            "the renderer's heading is kept verbatim: {rows:?}"
        );
        assert!(rows.iter().any(|row| row.label == "section" && row.value == "Declares"));
        assert!(
            rows.iter().any(|row| row.value.contains("- windows: 2")),
            "the renderer's lines are not rewritten: {rows:?}"
        );
    }

    #[test]
    fn a_populated_cell_draws_its_own_stars_tier_and_ticket() {
        let view = ApiView::of(document(), vec![populated_cell()]);
        let rows = view.rows();
        let row = rows
            .iter()
            .find(|row| row.label == "shell/evidence_grade")
            .expect("the shell/evidence cell is a row");
        assert_eq!(
            row.value.split_whitespace().next(),
            Some(Star::Three.glyphs().as_str()),
            "the star row is the matrix's own glyphs: {}",
            row.value
        );
        assert!(row.value.contains("silver"), "the computed tier: {}", row.value);
        assert!(row.value.contains("BLD-0187"), "the ticket: {}", row.value);
        assert_eq!(view.matrix.cell(Segment::Shell, AxisKind::EvidenceGrade).unwrap().tier(), Some(CertificationTier::Silver));
    }

    #[test]
    fn an_unchecked_cell_is_named_and_its_refusal_is_carried() {
        let view = ApiView::of(document(), vec![populated_cell(), unchecked_cell()]);
        assert!(!view.is_verified(), "a cell with no witness is not verified");
        let rows = view.rows();
        assert!(
            rows.iter().any(|row| row.label == "unchecked" && row.value == "world/grade"),
            "the unchecked cell is named as such: {rows:?}"
        );
        let refusal = view
            .refusals
            .iter()
            .find(|refusal| refusal.detail().contains("world/grade"))
            .expect("the gate refused it by name");
        assert_eq!(
            refusal.kind(),
            voxel_model::refusal::RefusalKind::UncalibratedOracle
        );
        assert!(
            rows.iter().any(|row| row.label == refusal.kind().name()
                && row.value.contains("unchecked")),
            "the refusal is a row rather than a count: {rows:?}"
        );
        assert!(view.tail().contains("2 cell(s)"), "{}", view.tail());
        assert!(view.tail().contains("1 unchecked"), "{}", view.tail());
    }

    #[test]
    fn a_segment_with_no_cell_is_a_refusal_the_pane_shows() {
        let view = ApiView::of(document(), vec![populated_cell()]);
        assert!(
            view.refusals
                .iter()
                .any(|refusal| refusal.detail().contains("segment ui has no cell")),
            "{:?}",
            view.refusals
        );
        // Seven segments, one cell: six absences, and the pane is not blank for them.
        assert!(view.rows().iter().filter(|row| row.label == "uncovered_axis").count() >= 6);
    }

    #[test]
    fn an_empty_document_says_why_it_is_empty() {
        let view = ApiView::of(String::new(), Vec::new());
        let rows = view.rows();
        assert_eq!(rows[0].label, "document");
        assert!(
            rows[0].value.contains("no document was rendered"),
            "the absence names itself: {rows:?}"
        );
    }

    #[test]
    fn the_panel_is_bounded_to_its_pane_and_says_how_much_it_holds() {
        // A document with more body than a 358 dp pane can show — the api-viewer window's own
        // declared size, 640×440 less the port's chrome.
        let mut long = document();
        for index in 0..40 {
            long.push_str(&format!("- detail {index}\n"));
        }
        let view = ApiView::of(long, vec![populated_cell(), unchecked_cell()]);
        let panel = api_panel(&view, 358.0);
        assert!(
            panel.height_dp() <= 358.0,
            "a {} dp panel in a 358 dp pane",
            panel.height_dp()
        );
        assert!(panel.note.is_some(), "and it says what it is holding");
        // **The spine survives and the prose does not** — the order `rows` documents, and the
        // api-viewer window's own 358 dp pane holds the title and two headings with room to spare.
        assert!(
            panel.rows.iter().any(|row| row.label == "document" && row.value == voxel_package::apidoc::TITLE),
            "the title is never dropped: {:?}",
            panel.rows
        );
        assert!(panel.rows.iter().any(|row| row.label == "section"));
        let tall = api_panel(&view, 4000.0);
        assert!(tall.note.is_none(), "nothing is held when the pane is tall");
        assert_eq!(tall.title, voxel_package::apidoc::TITLE);
    }
}
