//! **One cell per segment and one for the artifact format, built from the rows that ran, or a
//! decline that names the sources.**
//!
//! # A star is a count of rows, and every row names its source
//!
//! Each segment is measured by five [`Predicate`] rows:
//!
//! | row | source | what it asks |
//! |---|---|---|
//! | `workspace_member` | workspace | is every crate of this segment a declared member? |
//! | `declared_witness` | workspace | is the segment's site and every crate witness on disk? |
//! | `census_measured` | census | was every crate read on all six axes, with no unchecked cell? |
//! | `missing_docs_pin` | census | does every compiler count still describe these bytes? |
//! | `prose_frame` / `ladder_coverage` | conformance / ladder | is the segment within its declared budget (or is the ladder table faithful)? |
//!
//! The star is [`voxel_axis_matrix::lit_star`]'s count — never a number this module chose — and the
//! tier is the cell's own requirements, computed by the matrix. **Nothing here writes a star, a
//! tier or a segment word**: those come from the matrix's own types.
//!
//! # A row that did not run is not a pass, and a segment with no witness gets no cell
//!
//! If no row ran and passed, there is no star to claim and the segment is [`Decline`]d. If the rows
//! lit a star but the segment's own witness is not on disk, it is declined too: a cell whose witness
//! names a file nobody can open is exactly the score `PATCH-042` S5 forbids. A declined segment
//! contributes **no cell at all**, so [`voxel_axis_matrix::Matrix::verify`] reports it as a segment
//! with no cell — the matrix's own refused state, rather than a zero, a one-star, or an `unchecked`
//! reading over a file that is not there.
//!
//! # The reading's grade says how independent the witness is
//!
//! `evidence_grade B` — *read back from world state* — when the census read this tree and its pins
//! are current. `evidence_grade C` — *an artefact with no independent site* — when only the
//! workspace and the witness were available, because then the reading is the crate's own file
//! rather than a second instrument agreeing with it. The grade is computed from which rows lit, so
//! it cannot be raised by a caller.

use std::path::Path;

use voxel_axis_matrix::{lit_star, Cell, Matrix, Predicate, Reading, Segment};
use voxel_model::axis::{AxisKind, EvidenceGrade, Grade};
use voxel_package::census::{Axis as CensusAxis, Establishment, AXES, AXIS_COUNT};

use crate::evidence::{Evidence, SourceReading};
use crate::source::Source;
use crate::subject::{Subject, FORMAT, SUBJECTS};

/// The patch that states the rules a cell is held to: the five-star rule, the unchecked state, and
/// the recipe line. **One word, and the cell carries it.**
pub const PATCH: &str = "PATCH-042";

/// Why a segment has no cell: **which sources were consulted, and what each said.**
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decline {
    /// The segment that was not measured.
    pub segment: Segment,
    /// The sources that could not witness it, in [`Source::ALL`] order.
    pub sources: Vec<Source>,
    /// Every reason, each prefixed with the source that gave it.
    pub detail: String,
}

/// The verifier's answer: the cells it could measure, the declines it had to make, and the sources
/// it consulted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    cells: Vec<Cell>,
    declines: Vec<Decline>,
    sources: Vec<Source>,
    failures: Vec<(Source, String)>,
}

impl Report {
    /// Measure every segment in [`SUBJECTS`] against the evidence, with `ticket` as the ticket the
    /// reading is filed under.
    ///
    /// **The ticket is threaded through rather than invented.** A cell with no ticket is refused by
    /// [`voxel_axis_matrix::Cell::check`], and that is the right refusal: a reading nobody filed is
    /// a value someone typed. The artifact format is a [`Subject`] too, measured and declined by
    /// the same loop, so it cannot become a second report.
    pub fn of(evidence: &Evidence, ticket: &str) -> Self {
        let mut cells = Vec::with_capacity(SUBJECTS.len());
        let mut declines = Vec::new();
        for subject in SUBJECTS.iter().chain(std::iter::once(&FORMAT)) {
            match cell_for(evidence, subject, ticket) {
                Ok(cell) => cells.push(cell),
                Err(decline) => declines.push(decline),
            }
        }
        Report {
            cells,
            declines,
            sources: Source::ALL.to_vec(),
            failures: evidence.refusals(),
        }
    }

    /// The cells that were measured.
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    /// The segments that were declined, with their reasons.
    pub fn declines(&self) -> &[Decline] {
        &self.declines
    }

    /// Every source this report consulted, whether it produced a reading or refused.
    pub fn sources(&self) -> &[Source] {
        &self.sources
    }

    /// The matrix over the measured cells. **Assembled, not gated** — [`Report::verified`] is the
    /// gate, and a display path asks for the cells beside the refusals rather than instead of them.
    pub fn matrix(&self) -> Matrix {
        Matrix::assemble(self.cells.clone())
    }

    /// Whether every segment has a cell, every cell passed the matrix's gate, and no segment was
    /// declined. **A decline is absence of a reading, not a pass**: a format cell that could not be
    /// measured must not leave the report `verified` on the strength of the cells that remain.
    pub fn verified(&self) -> bool {
        self.declines.is_empty() && self.matrix().verify().is_ok()
    }

    /// **The report a console prints.** The matrix's one line per cell and its measured tail, then
    /// a line per decline, then the sources with their sites, then any source that refused.
    ///
    /// A pane bounds its rows; this text does not, which is what keeps a cell the pane could not fit
    /// **reachable** rather than silently dropped.
    pub fn lines(&self) -> Vec<String> {
        let mut lines = self.matrix().report();
        let tail = lines.pop().unwrap_or_default();
        for decline in &self.declines {
            lines.push(format!(
                "declined  {:<8} sources: {} — {}",
                decline.segment.name(),
                names_list(
                    &decline
                        .sources
                        .iter()
                        .map(|source| source.name())
                        .collect::<Vec<_>>()
                ),
                decline.detail
            ));
        }
        let declined: Vec<&str> = self
            .declines
            .iter()
            .map(|decline| decline.segment.name())
            .collect();
        lines.push(format!(
            "{tail} · declined: {}",
            if declined.is_empty() {
                "0".to_string()
            } else {
                declined.join(", ")
            }
        ));
        lines.push(format!(
            "sources   {}",
            Source::ALL
                .iter()
                .map(|source| format!("{}({})", source.name(), source.site()))
                .collect::<Vec<_>>()
                .join(" · ")
        ));
        for (source, detail) in &self.failures {
            lines.push(format!("source-refused  {}: {detail}", source.name()));
        }
        lines
    }
}

/// The four or five rows a segment is measured by.
fn rows_for(evidence: &Evidence, subject: &Subject) -> Vec<Row> {
    if let Some(schema) = subject.format {
        return format_rows(evidence, subject, schema);
    }
    let mut rows = vec![
        workspace_row(evidence, subject),
        witness_row(&evidence.root, subject),
        census_row(evidence, subject),
        pin_row(evidence, subject),
    ];
    rows.push(if subject.segment == Segment::Axis {
        ladder_row(evidence)
    } else {
        frame_row(evidence, subject)
    });
    rows
}

/// Build the cell, or the decline that says why there is none.
///
/// **A format whose reader refused gets no cell**, even though its witness and the census row can
/// still light: a star over those would be a star over a format nothing graded. And a format's
/// grade is `B` only when every one of its rows lit, so one row that did not is a `C` rather than a
/// value this function chose.
fn cell_for(evidence: &Evidence, subject: &Subject, ticket: &str) -> Result<Cell, Decline> {
    let rows = rows_for(evidence, subject);
    let predicates: Vec<Predicate> = rows.iter().map(|row| row.predicate.clone()).collect();
    let witness_held = rows
        .iter()
        .any(|row| row.predicate.name == "declared_witness" && row.lit());

    // **No witness, no cell** — a star over a file nobody can open is the defect the rule forbids.
    // The check is first because the star can be lit by a workspace declaration alone.
    if !witness_held {
        let detail = rows
            .iter()
            .find(|row| row.predicate.name == "declared_witness")
            .map(|row| row.reason.clone())
            .unwrap_or_else(|| "the segment names no witness".to_string());
        return Err(decline(subject, &rows, detail));
    }
    let format = subject.format.is_some();
    if format
        && !rows
            .iter()
            .any(|row| row.predicate.name == "frame_grammar_parses" && row.lit())
    {
        return Err(decline(subject, &rows, unlit_reasons(&rows)));
    }

    let Some(star) = lit_star(&predicates) else {
        return Err(decline(subject, &rows, unlit_reasons(&rows)));
    };

    let census_lit = rows
        .iter()
        .any(|row| row.predicate.name == "census_measured" && row.lit());
    let pin_lit = rows
        .iter()
        .any(|row| row.predicate.name == "missing_docs_pin" && row.lit());

    let complete = rows.iter().all(Row::lit);
    let grade_words = if format {
        if complete {
            "every row lit from the tree"
        } else {
            "a measured row did not light, so the format has no complete reading"
        }
    } else if census_lit && pin_lit {
        "read back from the tree"
    } else {
        "an artefact with no current census reading"
    };

    let mut basis = format!("{grade_words} — {}", unlit_and_lit_reasons(&rows));
    let unlit: Vec<&str> = rows
        .iter()
        .filter(|row| !row.lit())
        .map(|row| row.predicate.name)
        .collect();
    if !unlit.is_empty() {
        basis.push_str(&format!("; not counted: {}", unlit.join(", ")));
    }

    let reading = if format {
        Reading::of_ladder(if complete { Grade::B } else { Grade::C }, basis)
    } else {
        Reading::of_ladder(
            if census_lit && pin_lit {
                EvidenceGrade::B
            } else {
                EvidenceGrade::C
            },
            basis,
        )
    };

    let mut requirements: Vec<&'static str> = Vec::new();
    if format {
        requirements.push("declared_provenance");
        if census_lit {
            requirements.push("evidence_present");
        }
    } else {
        if rows
            .iter()
            .any(|row| row.predicate.name == "workspace_member" && row.lit())
        {
            requirements.push("declared_provenance");
        }
        if witness_held && census_lit {
            requirements.push("evidence_present");
        }
    }

    Ok(Cell::new(
        subject.segment,
        reading.witnessed(subject.site),
        star,
        predicates,
        ticket,
        PATCH,
    )
    .with_requirements(requirements))
}

fn decline(subject: &Subject, rows: &[Row], detail: String) -> Decline {
    let mut sources: Vec<Source> = rows
        .iter()
        .filter(|row| !row.lit())
        .map(|row| row.source)
        .collect();
    sources.sort_unstable();
    sources.dedup();
    Decline {
        segment: subject.segment,
        sources,
        detail,
    }
}

/// **The artifact format's rows**: `display-packages/alpha-shell`'s frame, read by the format's own
/// reader and the census's own table.
///
/// **No second grader.** Every frame row is a fact about what
/// [`voxel_package::cellframe::read`] returned — the same call the mount makes over a package's
/// artifact table — and a reader that refused carries its refusal into the row that did not light.
/// Six rows: the witness, the reader's acceptance, the recipe's row, the window, the grid, and the
/// census. The grade is `B` only when all six lit; a refused reader produces no cell at all.
fn format_rows(evidence: &Evidence, subject: &Subject, schema: &str) -> Vec<Row> {
    let root = &evidence.root;
    let folder = root.join("display-packages/alpha-shell");
    let source = std::fs::read(folder.join("package.shspec"));
    let recipe = source
        .as_ref()
        .ok()
        .and_then(|bytes| voxel_package::recipe::Recipe::parse(bytes).ok());
    let graded = recipe
        .as_ref()
        .map(|recipe| voxel_package::cellframe::read(&folder, recipe));
    let refusal = graded
        .as_ref()
        .and_then(|found| found.as_ref().err())
        .map(|found| {
            found
                .all()
                .iter()
                .map(|refusal| refusal.detail().to_string())
                .collect::<Vec<_>>()
                .join("; ")
        });
    let package = graded.and_then(Result::ok);
    let declared = package.as_ref().and_then(|package| {
        package
            .frames
            .iter()
            .find(|declared| declared.frame.id == "MAP-0001")
    });

    let mut rows = vec![if root.join(subject.site).is_file() {
        Row::held(
            Source::Workspace,
            "declared_witness",
            format!("{} is on disk", subject.site),
        )
    } else {
        Row::failed(
            Source::Workspace,
            "declared_witness",
            format!("{} is not on disk", subject.site),
        )
    }];

    match (recipe.as_ref(), declared) {
        (Some(_), Some(declared)) => rows.push(Row::held(
            Source::Workspace,
            "frame_grammar_parses",
            format!(
                "the format's own reader accepted {} and its record as {}",
                subject.site, declared.frame.id
            ),
        )),
        _ => {
            let why = refusal.clone().unwrap_or_else(|| match recipe {
                None => format!(
                    "{} is not a recipe this build reads",
                    folder.join("package.shspec").display()
                ),
                Some(_) => "the package's frame was not graded".to_string(),
            });
            rows.push(Row::failed(Source::Workspace, "frame_grammar_parses", why));
        }
    }

    match (recipe.as_ref(), declared) {
        (Some(recipe), Some(declared)) => {
            rows.push(
                if recipe.rows.iter().any(|row| row.id == declared.frame.id) {
                    Row::held(
                        Source::Workspace,
                        "recipe_names_the_frame_by_a_row",
                        format!("the recipe names frame {} by a row", declared.frame.id),
                    )
                } else {
                    Row::failed(
                        Source::Workspace,
                        "recipe_names_the_frame_by_a_row",
                        format!("the recipe names frame {} by no row", declared.frame.id),
                    )
                },
            );
            match recipe
                .windows
                .iter()
                .find(|window| window.id == declared.frame.window)
            {
                Some(window) => {
                    rows.push(Row::held(
                        Source::Workspace,
                        "frame_addresses_a_declared_window",
                        format!(
                            "the frame addresses window {:?}, which the package declares",
                            declared.frame.window
                        ),
                    ));
                    let needed_w = declared
                        .frame
                        .cols
                        .saturating_mul(voxel_package::cellframe::CELL_W_DP);
                    let needed_h = declared
                        .frame
                        .rows
                        .saturating_mul(voxel_package::cellframe::CELL_H_DP);
                    if needed_w <= window.geometry[2] && needed_h <= window.geometry[3] {
                        rows.push(Row::held(
                            Source::Workspace,
                            "frame_fits_the_window_extent",
                            format!(
                                "{} × {} cells fit the window's {} × {} dp",
                                declared.frame.cols,
                                declared.frame.rows,
                                window.geometry[2],
                                window.geometry[3]
                            ),
                        ));
                    } else {
                        rows.push(Row::failed(
                            Source::Workspace,
                            "frame_fits_the_window_extent",
                            format!(
                                "{} × {} cells need {} × {} dp and the window is {} × {} dp",
                                declared.frame.cols,
                                declared.frame.rows,
                                needed_w,
                                needed_h,
                                window.geometry[2],
                                window.geometry[3]
                            ),
                        ));
                    }
                }
                None => {
                    rows.push(Row::failed(
                        Source::Workspace,
                        "frame_addresses_a_declared_window",
                        format!(
                            "the frame addresses window {:?}, which the package does not declare",
                            declared.frame.window
                        ),
                    ));
                    rows.push(Row::not_run(
                        Source::Workspace,
                        "frame_fits_the_window_extent",
                        "no declared window to fit".to_string(),
                    ));
                }
            }
        }
        _ => {
            let why = refusal.unwrap_or_else(|| {
                "the package or its frame could not be read, so no frame was graded".to_string()
            });
            for name in [
                "recipe_names_the_frame_by_a_row",
                "frame_addresses_a_declared_window",
                "frame_fits_the_window_extent",
            ] {
                rows.push(Row::not_run(Source::Workspace, name, why.clone()));
            }
        }
    }

    if voxel_package::census::folders::table_names_format(root, schema) {
        rows.push(Row::held(
            Source::Census,
            "census_names_the_format",
            format!("packages/census.tsv names {schema} in a format row"),
        ));
    } else {
        rows.push(Row::failed(
            Source::Census,
            "census_names_the_format",
            format!("packages/census.tsv has no row naming {schema}"),
        ));
    }
    rows
}

fn unlit_reasons(rows: &[Row]) -> String {
    rows.iter()
        .filter(|row| !row.lit())
        .map(|row| format!("{}: {}", row.source.name(), row.reason))
        .collect::<Vec<_>>()
        .join("; ")
}

fn unlit_and_lit_reasons(rows: &[Row]) -> String {
    // **The source is named in the sentence**, not inferred from the row's name: a reader of the
    // cell should not have to know that `prose_frame` is the conformance source's question.
    let lit: Vec<String> = rows
        .iter()
        .filter(|row| row.lit())
        .map(|row| format!("{}: {}", row.source.name(), row.reason))
        .collect();
    if lit.is_empty() {
        unlit_reasons(rows)
    } else {
        lit.join("; ")
    }
}

fn workspace_row(evidence: &Evidence, subject: &Subject) -> Row {
    let names = crate_names(subject);
    match &evidence.workspace {
        SourceReading::Read(members) => {
            let missing: Vec<&str> = names
                .iter()
                .copied()
                .filter(|name| !members.iter().any(|member| member == name))
                .collect();
            if missing.is_empty() {
                Row::held(
                    Source::Workspace,
                    "workspace_member",
                    format!("the workspace manifest declares {}", names_list(&names)),
                )
            } else {
                Row::failed(
                    Source::Workspace,
                    "workspace_member",
                    format!(
                        "the workspace manifest does not declare {}",
                        names_list(&missing)
                    ),
                )
            }
        }
        SourceReading::Refused { detail } => {
            Row::not_run(Source::Workspace, "workspace_member", detail.clone())
        }
    }
}

fn witness_row(root: &Path, subject: &Subject) -> Row {
    let mut witnesses: Vec<&str> = vec![subject.site];
    witnesses.extend(subject.crates.iter().map(|crate_| crate_.witness_of()));
    let missing: Vec<&str> = witnesses
        .iter()
        .copied()
        .filter(|path| !root.join(path).is_file())
        .collect();
    if missing.is_empty() {
        Row::held(
            Source::Workspace,
            "declared_witness",
            format!(
                "every declared witness is on disk: {}",
                names_list(&witnesses)
            ),
        )
    } else {
        Row::failed(
            Source::Workspace,
            "declared_witness",
            format!(
                "these declared witnesses are not on disk: {}",
                names_list(&missing)
            ),
        )
    }
}

fn census_row(evidence: &Evidence, subject: &Subject) -> Row {
    let names = crate_names(subject);
    match &evidence.census {
        SourceReading::Read(census) => {
            let mut missing: Vec<&str> = Vec::new();
            let mut unchecked: Vec<String> = Vec::new();
            for crate_ in subject.crates {
                match census.reading_of(*crate_) {
                    None => missing.push(crate_.name()),
                    Some(reading) => {
                        for (index, cell) in reading.cells.iter().enumerate() {
                            if *cell == Establishment::Unchecked {
                                unchecked.push(format!("{}/{}", crate_.name(), AXES[index].name()));
                            }
                        }
                    }
                }
            }
            if missing.is_empty() && unchecked.is_empty() {
                Row::held(
                    Source::Census,
                    "census_measured",
                    format!(
                        "the census measured all {AXIS_COUNT} axes for {} with no unchecked cell",
                        names_list(&names)
                    ),
                )
            } else {
                let mut detail = Vec::new();
                if !missing.is_empty() {
                    detail.push(format!("no reading for {}", names_list(&missing)));
                }
                if !unchecked.is_empty() {
                    detail.push(format!("unchecked cells: {}", unchecked.join(", ")));
                }
                Row::failed(Source::Census, "census_measured", detail.join("; "))
            }
        }
        SourceReading::Refused { detail } => {
            Row::not_run(Source::Census, "census_measured", detail.clone())
        }
    }
}

fn pin_row(evidence: &Evidence, subject: &Subject) -> Row {
    let documented = documented_index();
    match &evidence.census {
        SourceReading::Read(census) => {
            let mut reasons = Vec::new();
            for crate_ in subject.crates {
                let Some(reading) = census.reading_of(*crate_) else {
                    reasons.push(format!("{} has no census reading", crate_.name()));
                    continue;
                };
                match reading.cells[documented] {
                    // `active` is a current zero and `inactive` a current count; both are readings
                    // of **this** tree, which is what the pin decides.
                    Establishment::Active | Establishment::Inactive => {}
                    state => reasons.push(format!(
                        "{} is {}: {}",
                        crate_.name(),
                        state.slug(),
                        reading.why[documented]
                            .clone()
                            .unwrap_or_else(|| "no reason recorded".to_string())
                    )),
                }
            }
            if reasons.is_empty() {
                Row::held(
                    Source::Census,
                    "missing_docs_pin",
                    format!(
                        "every missing-docs pin matches the bytes on disk for {}",
                        names_list(&crate_names(subject))
                    ),
                )
            } else {
                Row::failed(Source::Census, "missing_docs_pin", reasons.join("; "))
            }
        }
        SourceReading::Refused { detail } => {
            Row::not_run(Source::Census, "missing_docs_pin", detail.clone())
        }
    }
}

fn frame_row(evidence: &Evidence, subject: &Subject) -> Row {
    match &evidence.conformance {
        SourceReading::Read(standings) => {
            let mut missing: Vec<&str> = Vec::new();
            let mut over: Vec<String> = Vec::new();
            let mut pairs: Vec<String> = Vec::new();
            for crate_ in subject.crates {
                match standings.iter().find(|standing| standing.crate_ == *crate_) {
                    None => missing.push(crate_.name()),
                    Some(standing) if standing.measured > standing.allowed => over.push(format!(
                        "{} is {}% of a declared {}%",
                        crate_.name(),
                        standing.measured,
                        standing.allowed
                    )),
                    Some(standing) => pairs.push(format!(
                        "{} {}% of {}",
                        crate_.name(),
                        standing.measured,
                        standing.allowed
                    )),
                }
            }
            if missing.is_empty() && over.is_empty() {
                Row::held(
                    Source::Conformance,
                    "prose_frame",
                    format!("the prose frame holds: {}", pairs.join(", ")),
                )
            } else {
                let mut detail = Vec::new();
                if !missing.is_empty() {
                    detail.push(format!("no prose standing for {}", names_list(&missing)));
                }
                if !over.is_empty() {
                    detail.push(format!("over budget: {}", over.join(", ")));
                }
                Row::failed(Source::Conformance, "prose_frame", detail.join("; "))
            }
        }
        SourceReading::Refused { detail } => {
            Row::not_run(Source::Conformance, "prose_frame", detail.clone())
        }
    }
}

fn ladder_row(evidence: &Evidence) -> Row {
    match &evidence.ladder {
        SourceReading::Read(()) => Row::held(
            Source::Ladder,
            "ladder_coverage",
            format!(
                "all {} ladders declare the tokens their axis declares",
                AxisKind::ALL.len()
            ),
        ),
        SourceReading::Refused { detail } => {
            Row::not_run(Source::Ladder, "ladder_coverage", detail.clone())
        }
    }
}

/// Where [`CensusAxis::Documented`] sits in the census's fixed axis order.
fn documented_index() -> usize {
    AXES.iter()
        .position(|axis| *axis == CensusAxis::Documented)
        .expect("the census declares the documented axis")
}

fn crate_names(subject: &Subject) -> Vec<&'static str> {
    subject.crates.iter().map(|crate_| crate_.name()).collect()
}

/// `a`, `a and b`, `a, b and c` — the sentence a reason uses instead of a debug list.
fn names_list(words: &[&str]) -> String {
    match words {
        [] => "nothing".to_string(),
        [one] => (*one).to_string(),
        [first, second] => format!("{first} and {second}"),
        _ => {
            let (last, rest) = words.split_last().expect("non-empty, checked above");
            format!("{} and {last}", rest.join(", "))
        }
    }
}

/// One predicate, the source that produced it, and the sentence its reason becomes.
struct Row {
    source: Source,
    predicate: Predicate,
    reason: String,
}

impl Row {
    fn held(source: Source, name: &'static str, reason: String) -> Self {
        Row {
            source,
            predicate: Predicate::held(name),
            reason,
        }
    }

    fn failed(source: Source, name: &'static str, reason: String) -> Self {
        Row {
            source,
            predicate: Predicate::failed(name),
            reason,
        }
    }

    fn not_run(source: Source, name: &'static str, reason: String) -> Self {
        Row {
            source,
            predicate: Predicate::not_run(name),
            reason,
        }
    }

    fn lit(&self) -> bool {
        self.predicate.lit()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_axis_matrix::Star;
    use voxel_model::axis::CertificationTier;
    use voxel_package::census::Crate as CensusCrate;

    fn real_root() -> std::path::PathBuf {
        voxel_package::features::workspace_root(Path::new(env!("CARGO_MANIFEST_DIR")))
            .expect("this crate sits inside the workspace")
    }

    #[test]
    fn the_real_tree_measures_every_segment_from_its_own_sources() {
        let evidence = Evidence::read(&real_root());
        let report = Report::of(&evidence, "T-0954");
        assert_eq!(
            report.declines(),
            &[],
            "every segment's sources must witness the checked-in tree: {:?}",
            report.declines()
        );
        assert_eq!(
            report.cells().len(),
            SUBJECTS.len() + 1,
            "one cell per segment, plus the artifact format's own cell"
        );
        assert!(report.verified(), "{:?}", report.matrix().verify().err());

        let format = report
            .matrix()
            .cell(Segment::Tooling, AxisKind::Grade)
            .expect("the artifact format was measured")
            .clone();
        assert_eq!(format.star, Star::Five, "{}", format.value());
        // **The grade is `B` because the census names the format**, not because a caller typed
        // `A`: the format cell's rows are six measured questions (five format, one census).
        assert_eq!(format.reading.token, "grade B");
        assert_eq!(format.predicates.len(), 6, "{}", format.value());
        assert!(
            format
                .reading
                .witness
                .as_deref()
                .is_some_and(|witness| witness.ends_with("frames/MAP-0001.frame")),
            "the format cell names the frame it graded: {}",
            format.value()
        );
        assert_eq!(format.ticket, "T-0954");

        for cell in report
            .cells()
            .iter()
            .filter(|cell| cell.reading.axis == AxisKind::EvidenceGrade)
        {
            let segment = cell.segment.name();
            assert_eq!(
                cell.star,
                Star::Five,
                "{segment}: {} | basis: {}",
                cell.value(),
                cell.reading.basis
            );
            assert_eq!(cell.reading.axis, AxisKind::EvidenceGrade);
            assert_eq!(
                cell.reading.token, "evidence_grade B",
                "{segment} has no current census reading"
            );
            assert!(cell.is_checked(), "{segment} names no witness");
            let witness = cell.reading.witness.as_deref().expect("checked");
            assert!(
                real_root().join(witness).is_file(),
                "{segment}'s witness `{witness}` is not on disk"
            );
            assert_eq!(cell.tier(), Some(CertificationTier::Silver), "{segment}");
            assert_eq!(cell.ticket, "T-0954");
            assert_eq!(cell.patch, PATCH);
            for source in [Source::Workspace, Source::Census] {
                assert!(
                    cell.reading.basis.contains(source.name()),
                    "{segment}'s basis does not name {}: {}",
                    source.name(),
                    cell.reading.basis
                );
            }
            let expected = if cell.segment == Segment::Axis {
                Source::Ladder
            } else {
                Source::Conformance
            };
            assert!(
                cell.reading.basis.contains(expected.name()),
                "{segment}'s basis does not name {}: {}",
                expected.name(),
                cell.reading.basis
            );
        }
        // The axis segment is the one measured by the ladder table instead of the prose frame.
        let matrix = report.matrix();
        let axis = matrix
            .cell(Segment::Axis, AxisKind::EvidenceGrade)
            .expect("the axis segment was measured");
        assert!(
            axis.reading.basis.contains("ladder"),
            "the axis cell names its ladder reading: {}",
            axis.reading.basis
        );
    }

    /// **The format's grade is a reading, not a constant.** The same frame, with a census table
    /// that does not name the format, measures five of its six rows and reads `grade C`; the frame
    /// rows themselves still lit, so the fall is the missing census row and nothing else.
    #[test]
    fn the_format_grade_falls_when_a_measured_row_does_not_light() {
        let source = real_root().join("display-packages/alpha-shell");
        let root =
            std::env::temp_dir().join(format!("voxel-axis-verify-format-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let folder = root.join("display-packages/alpha-shell/frames");
        std::fs::create_dir_all(&folder).expect("a frame folder");
        std::fs::copy(
            source.join("package.shspec"),
            root.join("display-packages/alpha-shell/package.shspec"),
        )
        .expect("the package source");
        for name in ["MAP-0001.frame", "MAP-0001.md"] {
            std::fs::copy(source.join("frames").join(name), folder.join(name))
                .expect("the frame and its record");
        }
        let evidence = Evidence {
            root: root.clone(),
            workspace: SourceReading::Read(
                CensusCrate::ALL
                    .iter()
                    .map(|crate_| crate_.name().to_string())
                    .collect(),
            ),
            census: SourceReading::Refused {
                detail: "the census could not read this tree".to_string(),
            },
            conformance: SourceReading::Refused {
                detail: "the prose frame could not read this tree".to_string(),
            },
            ladder: SourceReading::Read(()),
        };
        let report = Report::of(&evidence, "T-0954");
        let matrix = report.matrix();
        let cell = matrix
            .cell(Segment::Tooling, AxisKind::Grade)
            .expect("the format was measured");
        assert_eq!(cell.reading.token, "grade C", "{}", cell.value());
        assert_eq!(cell.star, Star::Five, "{}", cell.value());
        assert!(
            cell.reading.basis.contains("census_names_the_format"),
            "{}",
            cell.reading.basis
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **A frame the reader refused is not a cell.** The witness and the census row can still light;
    /// the report declines the format by name rather than filing a star over a frame nothing graded.
    #[test]
    fn a_refused_format_frame_declines_instead_of_earning_a_star() {
        let source = real_root().join("display-packages/alpha-shell");
        let root = std::env::temp_dir().join(format!(
            "voxel-axis-verify-refused-frame-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let folder = root.join("display-packages/alpha-shell/frames");
        std::fs::create_dir_all(&folder).expect("a frame folder");
        std::fs::copy(
            source.join("package.shspec"),
            root.join("display-packages/alpha-shell/package.shspec"),
        )
        .expect("the package source");
        std::fs::copy(
            source.join("frames/MAP-0001.frame"),
            folder.join("MAP-0001.frame"),
        )
        .expect("the frame");
        std::fs::write(
            folder.join("MAP-0001.md"),
            "# Change\n\na record with none of the four sections\n",
        )
        .expect("a record that cannot explain its frame");
        let evidence = Evidence {
            root: root.clone(),
            workspace: SourceReading::Read(
                CensusCrate::ALL
                    .iter()
                    .map(|crate_| crate_.name().to_string())
                    .collect(),
            ),
            census: SourceReading::Refused {
                detail: "the census could not read this tree".to_string(),
            },
            conformance: SourceReading::Refused {
                detail: "the prose frame could not read this tree".to_string(),
            },
            ladder: SourceReading::Read(()),
        };
        let report = Report::of(&evidence, "T-0954");
        assert!(
            report
                .matrix()
                .cell(Segment::Tooling, AxisKind::Grade)
                .is_none(),
            "a refused frame has no star"
        );
        let decline = report
            .declines()
            .iter()
            .find(|decline| {
                decline.segment == Segment::Tooling && decline.detail.contains("section")
            })
            .expect("the format is declined by name");
        assert!(
            decline.detail.contains("section"),
            "the decline names what the record is missing: {}",
            decline.detail
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_blank_ticket_is_refused_by_the_matrix_rather_than_papered_over() {
        let evidence = Evidence::read(&real_root());
        let report = Report::of(&evidence, "  ");
        assert_eq!(
            report.cells().len(),
            SUBJECTS.len() + 1,
            "the readings still exist"
        );
        assert!(!report.verified(), "a reading nobody filed is not verified");
        let refused = report.matrix().verify().expect_err("no ticket");
        assert!(
            refused
                .all()
                .iter()
                .any(|refusal| refusal.detail().contains("ticket")),
            "{:?}",
            refused.all()
        );
    }

    /// A tree whose workspace is readable and whose files are not: the shape a declined segment has
    /// to survive. **No file is invented for it** — the root is an empty directory.
    fn hollow_evidence() -> (Evidence, std::path::PathBuf) {
        let root =
            std::env::temp_dir().join(format!("voxel-axis-verify-hollow-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("an empty root");
        let evidence = Evidence {
            root: root.clone(),
            workspace: SourceReading::Read(
                CensusCrate::ALL
                    .iter()
                    .map(|crate_| crate_.name().to_string())
                    .collect(),
            ),
            census: SourceReading::Refused {
                detail: "the census could not read this tree".to_string(),
            },
            conformance: SourceReading::Refused {
                detail: "the prose frame could not read this tree".to_string(),
            },
            ladder: SourceReading::Read(()),
        };
        (evidence, root)
    }

    #[test]
    fn a_segment_with_no_witness_is_declined_and_the_decline_names_its_sources() {
        let (evidence, root) = hollow_evidence();
        let report = Report::of(&evidence, "T-0954");
        assert!(report.cells().is_empty(), "no witness, no cell");
        assert_eq!(
            report.declines().len(),
            SUBJECTS.len() + 1,
            "seven segments and the artifact format"
        );
        assert!(!report.verified());

        let shell = report
            .declines()
            .iter()
            .find(|decline| decline.segment == Segment::Shell)
            .expect("the shell was declined");
        assert!(
            shell.detail.contains("not on disk"),
            "the decline names the absent witness: {}",
            shell.detail
        );
        assert!(
            shell.sources.contains(&Source::Census),
            "the sources it could not read are named: {:?}",
            shell.sources
        );

        // The refused sources are printed with their reasons, and no star is invented for a
        // segment that was never measured.
        let lines = report.lines();
        assert!(
            lines
                .iter()
                .any(|line| line.contains("source-refused  census")),
            "{lines:#?}"
        );
        assert!(
            lines.iter().all(|line| !line.contains('★')),
            "a declined segment has no star: {lines:#?}"
        );
        assert!(
            lines.iter().any(|line| line.contains("declined  shell")),
            "{lines:#?}"
        );
        assert!(
            lines
                .iter()
                .any(|line| line.contains("sources   workspace(Cargo.toml)")),
            "{lines:#?}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_basis_is_written_from_the_rows_that_ran_and_names_what_was_not_counted() {
        // The world's evidence, with the prose frame refused: four rows run, one does not, so the
        // star is four and the basis says which row was left out.
        let root = real_root();
        let mut evidence = Evidence::read(&root);
        evidence.conformance = SourceReading::Refused {
            detail: "the prose frame is not this build's".to_string(),
        };
        let report = Report::of(&evidence, "T-0954");
        let world = report
            .matrix()
            .cell(Segment::World, AxisKind::EvidenceGrade)
            .expect("the world is still measured")
            .clone();
        // Four rows *can* light here — the tree's pins decide whether the pin row is one of them —
        // so the assertion is that the refused source's row is never counted, not a fixed total.
        assert!(world.star <= Star::Four, "{}", world.value());
        assert!(
            world.reading.basis.contains("not counted:")
                && world.reading.basis.contains("prose_frame"),
            "{}",
            world.reading.basis
        );
        assert!(
            world
                .reading
                .basis
                .contains("the census measured all 7 axes"),
            "{}",
            world.reading.basis
        );
    }

    #[test]
    fn a_name_list_reads_as_a_sentence() {
        assert_eq!(names_list(&[]), "nothing");
        assert_eq!(names_list(&["a"]), "a");
        assert_eq!(names_list(&["a", "b"]), "a and b");
        assert_eq!(names_list(&["a", "b", "c"]), "a, b and c");
    }
}
