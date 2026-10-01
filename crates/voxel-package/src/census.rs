//! **The crates already in the project, evaluated on the same terms as the new
//! ones.**
//!
//! # Why this module exists
//!
//! R-59.7: *"a lot of the missing functions are supposed to be evaluators for
//! the building."* The evaluators built so far — [`crate::features`],
//! [`crate::oracle`], [`crate::record`], [`crate::compendium`] — all describe
//! the **new** work: the package system, the recipe, the gate. They say nothing
//! about the eleven other crates, which are the larger half of this repository
//! and were written before any of this existed.
//!
//! That is a real gap, not a tidiness one. A gate that only grades the code it
//! was written alongside cannot tell a contributor that the crate they are
//! extending is undocumented, untested, or contradicted by the record — because
//! nothing looks. So this module gives the pre-existing crates the same
//! treatment: a declared row per crate, a witness on disk, and an answer.
//!
//! # The `axis.rs` pattern, and why it is here
//!
//! `C:/Users/ala/Documents/shploingus-game/crates/diffusion-core/src/axis.rs`
//! gets coverage right in a way this project could not argue its way into on its
//! own, and this module is that pattern applied to the crate set:
//!
//! * **an exhaustive `match` with no wildcard arm** ([`witness_of`]) — add a
//!   variant to [`Crate`] and the crate stops compiling until someone says
//!   where its witness is. A non-exhaustive match cannot be merged; a failing
//!   test can be deleted;
//! * **a `const` row-count pin** ([`CRATES_ARE_TOTAL`]) — the value is
//!   meaningless and always zero, and the **type** is the assertion. It is a
//!   *literal* rather than `CRATES.len()` because deriving it would make the two
//!   sides agree by construction and the check would check nothing;
//! * **`check_coverage` as the runtime anti-fabrication guard** — because a
//!   pattern is a way of writing, and a way of writing is not a proof that
//!   anyone read the rules.
//!
//! # What a crate is judged on, and why these five
//!
//! Each axis is something that **can be checked from this tree**, so no axis is
//! a matter of opinion. A sixth axis — "is this crate good?" — would be
//! unanswerable and is therefore absent rather than approximated:
//!
//! * [`Axis::Named`] — the record mentions the crate. Checkable: the ten
//!   documents either do or do not contain the name.
//! * [`Axis::Moduled`] — it declares modules. Checkable: `lib.rs` either has
//!   `pub mod` lines or it does not. A crate with none is a leaf of a different
//!   shape, and the axis reports `Unchecked` rather than inventing a verdict.
//! * [`Axis::Tested`] — it has tests. Checkable: `#[test]` either appears or it
//!   does not.
//! * [`Axis::Refusing`] — it can refuse. Checkable: it names a
//!   [`voxel_model::RefusalKind`] or it does not. This is the axis that matters
//!   most and the one most likely to be `Unchecked`, because a crate that
//!   cannot refuse is a crate that returns a plausible value where the record
//!   says it should decline.
//! * [`Axis::Declared`] — the record states what it is **for**. Checkable: the
//!   [`CrateRow::means`] text is read by a person and checked by nobody, and
//!   that is stated on the field rather than left to be assumed.
//!
//! The first three axes are **measured** and the fourth is measured, so a
//! [`Census`] is a fact about the tree rather than an opinion. The fifth is
//! explicitly not, and says so in its own field.
//!
//! # The finding this module exists to produce
//!
//! Running it on this tree finds that **`voxel-ui` and `shploingus` have no
//! `#[test]` at all**, and that is reported by measurement rather than asserted
//! here — see [`census::tests_report`]. It is not fixed by this module. It is
//! *visible*, which is the difference between a gate and a gate that only looks
//! at half the repository.

use std::path::Path;

use voxel_model::{RefusalKind, RefusalSet};


use crate::record::read_document;

/// How many crates the workspace holds. **A literal**, for the reason
/// [`crate::features::FEATURE_COUNT`] is one.
pub const CRATE_COUNT: usize = 12;

/// Every crate in the workspace, in dependency order.
///
/// Dependency order rather than alphabetical, because the point of the list is
/// to read as the build reads: `voxel-model` refuses before anything asks it
/// to, and a reader who sees `voxel-package` first has not been told what it
/// stands on.
pub const CRATES: [Crate; CRATE_COUNT] = [
    Crate::Model,
    Crate::Noise,
    Crate::Mesh,
    Crate::Shapers,
    Crate::Stream,
    Crate::Instrument,
    Crate::Appearance,
    Crate::Work,
    Crate::Ui,
    Crate::Client,
    Crate::Package,
    Crate::Shploingus,
];

/// **Check 2 of 2.** Every crate has a row, in a `const`.
pub const CRATES_ARE_TOTAL: [(); CRATE_COUNT] = [(); CRATES.len()];

/// A crate in this workspace. One variant each.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Crate {
    Model,
    Noise,
    Mesh,
    Shapers,
    Stream,
    Instrument,
    Appearance,
    Work,
    Ui,
    Client,
    Package,
    Shploingus,
}

impl Crate {
    /// Every crate, in dependency order.
    pub const ALL: [Crate; CRATE_COUNT] = CRATES;

    /// The directory name, which is also the crate name.
    pub const fn name(self) -> &'static str {
        match self {
            Crate::Model => "voxel-model",
            Crate::Noise => "voxel-noise",
            Crate::Mesh => "voxel-mesh",
            Crate::Shapers => "voxel-shapers",
            Crate::Stream => "voxel-stream",
            Crate::Instrument => "voxel-instrument",
            Crate::Appearance => "voxel-appearance",
            Crate::Work => "voxel-work",
            Crate::Ui => "voxel-ui",
            Crate::Client => "voxel-client",
            Crate::Package => "voxel-package",
            Crate::Shploingus => "shploingus",
        }
    }

    /// The path a reader opens. Relative to the workspace root.
    pub const fn path(self) -> &'static str {
        match self {
            Crate::Model => "crates/voxel-model",
            Crate::Noise => "crates/voxel-noise",
            Crate::Mesh => "crates/voxel-mesh",
            Crate::Shapers => "crates/voxel-shapers",
            Crate::Stream => "crates/voxel-stream",
            Crate::Instrument => "crates/voxel-instrument",
            Crate::Appearance => "crates/voxel-appearance",
            Crate::Work => "crates/voxel-work",
            Crate::Ui => "crates/voxel-ui",
            Crate::Client => "crates/voxel-client",
            Crate::Package => "crates/voxel-package",
            Crate::Shploingus => "crates/shploingus",
        }
    }

    /// **Check 1 of 2.** The file that witnesses this crate existing.
    ///
    /// Exhaustive with **no wildcard arm**: add a variant to [`Crate`] and this
    /// stops compiling until someone says which file proves the crate is there.
    /// A test can be deleted; this cannot be compiled past.
    pub const fn witness_of(self) -> &'static str {
        match self {
            Crate::Model => "crates/voxel-model/src/lib.rs",
            Crate::Noise => "crates/voxel-noise/src/lib.rs",
            Crate::Mesh => "crates/voxel-mesh/src/lib.rs",
            Crate::Shapers => "crates/voxel-shapers/src/lib.rs",
            Crate::Stream => "crates/voxel-stream/src/lib.rs",
            Crate::Instrument => "crates/voxel-instrument/src/lib.rs",
            Crate::Appearance => "crates/voxel-appearance/src/lib.rs",
            Crate::Work => "crates/voxel-work/src/lib.rs",
            Crate::Ui => "crates/voxel-ui/src/lib.rs",
            Crate::Client => "crates/voxel-client/src/lib.rs",
            Crate::Package => "crates/voxel-package/src/lib.rs",
            Crate::Shploingus => "crates/shploingus/src/main.rs",
        }
    }

    /// Look a crate up by name. **Refused** rather than defaulted, as everywhere.
    pub fn parse(word: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|candidate| candidate.name() == word.trim())
            .ok_or_else(|| format!("unknown crate {word:?}; this project has {CRATE_COUNT}"))
    }
}

/// One crate, its witness, and what the record says it is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CrateRow {
    pub crate_: Crate,
    /// What "working" means for **this** crate, in one line.
    ///
    /// **Unchecked by anything.** The field most able to be lied about, which is
    /// why it is here and why that is stated: it is read by a person, and a
    /// machine reading it is reading an author's account of their own intent.
    pub means: &'static str,
    /// Whether a missing witness stops the build.
    ///
    /// The same split as [`crate::features::Feature::required_for_alpha`]: some
    /// absences are fatal and some are shown. Both are *shown*.
    pub required: bool,
}

/// Every crate, its witness, and what the record says it is for.
///
/// **These `means` are the agent's, and they are not evidence of anything.** They were
/// written by whoever wrote this module, so a check that reads them is reading an author's
/// account of their own intent. [`Axis::Claimed`] is the axis that replaced that circularity,
/// and it reads the *spec's* declaration tables instead. This table is kept because `required`
/// is a real build decision, and deleting the field would hide a decision rather than remove a
/// fiction — but the `means` here are documentation, not a licence, and a test says so.
pub const ROWS: [CrateRow; CRATE_COUNT] = [
    CrateRow { crate_: Crate::Model, means: "the world model: coordinates, identity, materials, and the refusal vocabulary everything else speaks", required: true },
    CrateRow { crate_: Crate::Noise, means: "deterministic noise, so the same seed yields the same world", required: true },
    CrateRow { crate_: Crate::Mesh, means: "chunks become meshes, and a mesh can be refined or raycast", required: true },
    CrateRow { crate_: Crate::Shapers, means: "stages and shapers produce the world, deterministically, from a seed", required: true },
    CrateRow { crate_: Crate::Stream, means: "what is resident, and what is wanted next", required: true },
    CrateRow { crate_: Crate::Instrument, means: "measurement that reports rather than asserts", required: false },
    CrateRow { crate_: Crate::Appearance, means: "how a material looks, and where that answer came from", required: false },
    CrateRow { crate_: Crate::Work, means: "the budget and the pool: work is bounded and dispatched", required: true },
    CrateRow { crate_: Crate::Ui, means: "the design system resolves colour, type and elevation, and refuses unrepresentable values", required: false },
    CrateRow { crate_: Crate::Client, means: "the GPU side, shared with the offscreen test that measures it", required: true },
    CrateRow { crate_: Crate::Package, means: "a package is admitted by validation, never by optimism", required: true },
    CrateRow { crate_: Crate::Shploingus, means: "the only place a window opens, and the only place the gate runs", required: true },
];

/// The row for a crate, or `None` — which is a fact, not a default.
pub fn row_of(crate_: Crate) -> Option<&'static CrateRow> {
    ROWS.iter().find(|row| row.crate_ == crate_)
}

/// The claim for a crate, or `None` — which means it was not measured, and is a
/// different fact from a measured `unclaimed`.
pub fn claim_of(claims: &[Claim], crate_: Crate) -> Option<&Claim> {
    claims.iter().find(|claim| claim.crate_ == crate_)
}

/// The declaration tables this tree's spec uses, by name.
///
/// **One, and the reduction is the finding.** The first version of this listed three —
/// `DESIGN-SPEC.md`, `LEDGER.md` and `STATUS.md` — and reported **twelve of twelve claimed**.
/// Two of those three had to go, for reasons that are the whole reason this check exists:
///
/// * **`LEDGER.md` is append-only and the agent writes to it.** Including it means the agent
///   can declare anything about its own code by writing a sentence — and worse, C-65 already
///   put a **census gap table** in there naming all twelve crates, so the census was reading its
///   own output and congratulating itself. That is the "axis graded against the author's own
///   prose" failure one level up: not a table I typed, but a table my own tool printed.
/// * **`STATUS.md` is derived.** Hyperskill's own protocol says *"STATUS.md is derived and never
///   authorizes a transition"*, and it does exactly what this axis must not do: it lists all
///   twelve workspace members as a fact of the build, which says nothing about whether the spec
///   asked for them.
///
/// So the only source is `DESIGN-SPEC.md`, whose own first line calls it *"the authority for
/// shploingus-clean"*. A spec that names its own tables is checkable; a build report is not a
/// specification, and an append-only log written by the measurer is the worst possible witness.
pub const DECLARATION_TABLES: [&str; 1] = ["docs/DESIGN-SPEC.md"];

/// The measured answer to "does the spec declare this crate?", and where it said so.
///
/// **`Claimed` is a measurement; `Unclaimed` is the record's own word.** D97, in the reference
/// record this tree inherits: *"A file that implements no row is not mis-conformant, it is
/// `unclaimed`, and that is a different refusal."* A crate the spec's tables never name is not
/// a defect and not a failure to conform — it is a thing nobody has claimed yet, which is a
/// different fact and deserves a different word.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Claim {
    pub crate_: Crate,
    /// Whether any declaration table names it.
    pub claimed: bool,
    /// Which document named it, when one did. **Never a bare `true`**, because a claim with no
    /// locator cannot be checked by the person it affects.
    pub witness: Option<String>,
    /// Which line inside that document, so a reader is one click away rather than one grep.
    pub line: Option<usize>,
}

impl Claim {
    /// Whether the spec names this crate.
    pub const fn is_claimed(&self) -> bool {
        self.claimed
    }

    /// The refusal text an unclaimed crate earns, naming the tables that were searched.
    pub fn why_unclaimed(&self) -> String {
        format!(
            "the spec's declaration tables ({}) never name {}. It is **unclaimed**: not \
             mis-conformant, and not a failure — a thing nobody has claimed yet. Either a \
             feature row is owed for it, or it is a crate this project does not need",
            DECLARATION_TABLES.join(", "),
            self.crate_.name()
        )
    }
}

/// Measure every crate against the spec's own declaration tables.
///
/// **A declaration table, not a document.** The first version of this searched each whole
/// document for the crate's name, and it reported **twelve of twelve claimed** — including
/// `shploingus`, "claimed" at `DESIGN-SPEC.md:1`, which is the document's own title line and
/// contains the project's name. A check that cannot fail is not a check, and the failure was
/// silent: the report looked perfect.
///
/// So the search is restricted to **table rows** — a line beginning with `|` — and a row must
/// place the crate in a cell, not merely mention it. A Markdown table is where a spec *declares*
/// something; a heading that titles a crate's own history is a record of what happened, not a
/// statement of what was asked for. Conflating them is how a crate becomes "declared" by the
/// act of having been written down.
pub fn claim_against_spec(root: &Path) -> Result<Vec<Claim>, RefusalSet> {
    let mut refusals = RefusalSet::new();
    let mut claims = Vec::with_capacity(CRATE_COUNT);

    // Each document read **once**, with its lines, so twelve crates do not cause twelve reads
    // of a 400 KB corpus.
    let mut documents: Vec<(String, Vec<String>)> = Vec::new();
    for path in DECLARATION_TABLES {
        let relative = path.trim_start_matches("docs/");
        match std::fs::read_to_string(root.join("docs").join(relative)) {
            Ok(text) => documents.push((
                path.to_string(),
                text.lines().map(str::to_string).collect::<Vec<String>>(),
            )),
            Err(error) => {
                refusals.refuse(
                    RefusalKind::NoReading,
                    format!(
                        "{path} is one of the declaration tables and it could not be read: \
                         {error}. A claim measured against a partial spec is a claim nobody can \
                         check"
                    ),
                );
            }
        }
    }
    if !refusals.is_empty() {
        return Err(refusals);
    }

    for crate_ in Crate::ALL {
        let needle = crate_.name();
        let mut witness: Option<String> = None;
        let mut line: Option<usize> = None;
        for (path, lines) in &documents {
            let found = lines.iter().position(|line| declares_in_row(line, needle));
            if let Some(found) = found {
                witness = Some(path.clone());
                // **1-based**, because that is what an editor and a `grep -n` both say, and a
                // report that counts from zero sends a reader one line off.
                line = Some(found + 1);
                break;
            }
        }
        claims.push(Claim {
            crate_,
            claimed: witness.is_some(),
            witness,
            line,
        });
    }

    refusals.into_result(claims)
}

/// Whether one line of one document **declares** this crate.
///
/// Three conditions, and each is a way the first version was fooled:
///
/// 1. **the line is a table row** — it starts with `|`. A spec declares in tables; a heading
///    that titles a crate's history is a record of what happened, not of what was asked for.
///    Without this, `DESIGN-SPEC.md:1` — the title line, which contains the project's name —
///    "declares" the binary crate;
/// 2. **the crate name is a whole cell**, not a substring. `voxel-model` appearing inside
///    `` `voxel-model::shapers` `` is a module path, which is a mention;
/// 3. **the row is not a header or separator** (`|---|---|`, or a bolded column label), which
///    name the table's shape rather than its contents.
fn declares_in_row(line: &str, needle: &str) -> bool {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') {
        return false;
    }
    // A separator row is `|---|---|` and a header row is bold-labelled; neither carries a
    // declaration, and both match every name-shaped string if you let them.
    if trimmed.starts_with("|-") || trimmed.contains("---") && !trimmed.contains(needle) {
        return false;
    }
    trimmed
        .split('|')
        // The empty cells either side of a leading and trailing `|`.
        .filter(|cell| !cell.trim().is_empty())
        .any(|cell| cell_declares(cell, needle))
}

/// Whether one **cell** names this crate.
///
/// **A cell can name more than one crate**, and the spec's F12 row does exactly that:
/// `` `voxel-mesh`, `voxel-stream` `` in a single cell. Splitting the cell on commas and
/// matching whole tokens is therefore not a detail — without it, two genuinely declared crates
/// read as undeclared, and a check that produces false negatives gets ignored, which is the
/// other way a gate stops working.
/// The row test, exposed so the suite can pin its edges without duplicating it.
///
/// **A copy of this rule in a test is a second rule**, which is the thing D-07 names and the
/// thing this whole module was rewritten to stop. The suite calls the real function.
pub fn declares_in_row_for_test(line: &str, needle: &str) -> bool {
    declares_in_row(line, needle)
}

fn cell_declares(cell: &str, needle: &str) -> bool {
    cell.split(',')
        .map(|token| {
            token
                .trim()
                .trim_matches('`')
                .trim_matches('*')
                .trim()
                .trim_matches('`')
        })
        .any(|token| token == needle || token.starts_with(&format!("{needle}::")))
}

/// The report: one line per crate, the spec's answer and where it came from.
pub fn claim_report(claims: &[Claim]) -> String {
    let mut out = String::new();
    for claim in claims {
        out.push_str(&format!("{:<20}", claim.crate_.name()));
        match (&claim.witness, claim.line) {
            (Some(path), Some(line)) => {
                out.push_str(&format!("claimed\t{path}:{line}\n"))
            }
            _ => out.push_str("unclaimed\t-\n"),
        }
    }
    out
}

/// The five things a crate is measured on.
///
/// **Five, deliberately.** Each is checkable from this tree without judgement.
/// An axis that could not be checked would be an opinion wearing a measurement's
/// clothes, and this repository has refused those before.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Axis {
    /// The record names this crate somewhere.
    Named,
    /// `lib.rs` declares at least one `pub mod`.
    Moduled,
    /// The crate contains at least one `#[test]`.
    Tested,
    /// The crate names at least one [`voxel_model::RefusalKind`].
    Refusing,
    /// **The spec's declaration tables name this crate.** Measured by
    /// [`claim_against_spec`], against `DESIGN-SPEC.md` §2's F-table, `DESIGN-SPEC.md` §3's
    /// slot manifest, and `LEDGER.md` C-5's crate-layout ruling.
    ///
    /// **This axis replaces `Declared`, and the reason is that `Declared` could not fail.** It
    /// read [`CrateRow::means`] — twelve sentences written by the agent that wrote this module —
    /// so a crate the spec never mentioned scored `active` because the author had described it.
    /// An axis graded against the grader's own prose is not a measurement. This one reads the
    /// spec, and a crate the spec does not name comes back `unclaimed`.
    Claimed,
}

/// How many axes. A literal, as everywhere else.
pub const AXIS_COUNT: usize = 5;

/// Every axis, in the order the report prints them.
pub const AXES: [Axis; AXIS_COUNT] = [
    Axis::Named,
    Axis::Moduled,
    Axis::Tested,
    Axis::Refusing,
    Axis::Claimed,
];

/// The coverage row: one cell per axis.
///
/// `axis.rs`'s `COVERAGE` transposed: a `[Axis; AXIS_COUNT]` per crate rather
/// than a column, so adding an axis to [`Axis`] without filling a cell in every
/// row is a compile error.
pub type Coverage = [[Establishment; AXIS_COUNT]; CRATE_COUNT];

/// One measured fact about one crate on one axis.
///
/// **Four states, and the third is the point.** `Unchecked` means *no instrument
/// ran*, which is not a pass — the same distinction
/// [`crate::features::Establishment`] makes, reused rather than reinvented
/// because a second vocabulary for "we did not look" is how a gate comes to read
/// as greener than it is.
pub type Establishment = crate::features::Establishment;

/// What one crate was measured as, per axis.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reading {
    pub crate_: Crate,
    pub cells: [Establishment; AXIS_COUNT],
    /// The evidence, per axis. A `Refused` cell says what it refused, because a
    /// refusal that names nothing is a refusal nobody can act on.
    pub why: [Option<String>; AXIS_COUNT],
}

impl Reading {
    /// One cell.
    pub fn cell(&self, axis: Axis) -> Establishment {
        self.cells[index_of(axis)]
    }

    /// Why that cell is what it is, if it said.
    pub fn why_at(&self, axis: Axis) -> Option<&str> {
        self.why[index_of(axis)].as_deref()
    }

    /// Every axis that is not `Active`, with the reason.
    ///
    /// The report a reader actually wants, because "which of my crates is
    /// incomplete" is the question and a grid of twenty enum values is not the
    /// answer to it.
    pub fn gaps(&self) -> Vec<(Axis, Establishment, Option<&str>)> {
        AXES
            .iter()
            .enumerate()
            .filter(|(_, axis)| self.cell(**axis) != Establishment::Active)
            .map(|(index, _axis)| (axis_ref(index), self.cells[index], self.why[index].as_deref()))
            .collect()
    }
}

fn index_of(axis: Axis) -> usize {
    AXES.iter().position(|candidate| *candidate == axis).expect("every axis is in AXES")
}

fn axis_ref(index: usize) -> Axis {
    AXES[index]
}

/// What the whole workspace was measured as.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Census {
    pub readings: Vec<Reading>,
}

impl Census {
    /// One crate's reading, or `None` — which the caller must handle, because a
    /// crate that was not measured is not a crate that passed.
    pub fn reading_of(&self, crate_: Crate) -> Option<&Reading> {
        self.readings.iter().find(|reading| reading.crate_ == crate_)
    }

    /// The full coverage grid, for the report and for [`check_coverage`].
    pub fn coverage(&self) -> Coverage {
        let mut grid = [[Establishment::Unchecked; AXIS_COUNT]; CRATE_COUNT];
        for reading in &self.readings {
            let row = CRATES
                .iter()
                .position(|candidate| *candidate == reading.crate_)
                .expect("a reading is for a crate in CRATES");
            grid[row] = reading.cells;
        }
        grid
    }

    /// Crates with at least one axis not `Active`.
    pub fn incomplete(&self) -> Vec<&Reading> {
        self.readings
            .iter()
            .filter(|reading| reading.cells.iter().any(|cell| *cell != Establishment::Active))
            .collect()
    }
}

/// **What the census read about one crate — the facts, before any rule.**
///
/// The inversion `crate::decision` records: this module reports what it read, and the decision
/// layer decides what it means (`decision::rule_for_axis`). Nothing here turns a fact into a
/// state, so the rule has one implementation and a second instrument can be handed the same
/// observations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrateObservation {
    pub crate_: Crate,
    /// `Some(true)` when the record names it, `Some(false)` when the record was read and does not,
    /// **`None` when the record could not be read at all** — three facts, and the third is why
    /// "unnamed" is never claimed on a partial corpus.
    pub named_by_record: Option<bool>,
    /// `None` when the crate has no readable source, which is *not* the same as a crate whose
    /// source lacks a `pub mod`.
    pub sources: Option<SourceFacts>,
    /// `None` when this crate was not measured against the spec at all.
    pub claim: Option<ClaimObservation>,
}

/// What a crate's source files were read as containing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceFacts {
    pub moduled: bool,
    pub tested: bool,
    pub refusing: bool,
}

/// The spec's claim about a crate, as read — including the text an unclaimed crate earns, so the
/// decision layer does not re-derive it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimObservation {
    pub claimed: bool,
    pub witness: Option<String>,
    pub line: Option<usize>,
    pub why_unclaimed: String,
}

/// **Read the tree. Facts only.**
///
/// A crate whose files cannot be read gets `None` for its sources, and the decision layer turns
/// that into `Refused` on every axis that depends on them. It does not get skipped: a crate that is
/// absent from the report and a crate that is present and unreadable are different facts, and only
/// the second one is true.
pub fn observe(root: &Path) -> Result<Vec<CrateObservation>, RefusalSet> {
    let mut refusals = RefusalSet::new();

    // The record's text is read once and shared, so ten crates do not cause ten
    // reads of a 400 KB corpus.
    let corpus = read_corpus(root, &mut refusals);

    // The spec's own declaration tables, read once. **This is the axis that can fail** — the
    // other four read the tree, and this one reads what the tree was asked for.
    let claims = match claim_against_spec(root) {
        Ok(claims) => claims,
        Err(found) => {
            refusals.absorb(found);
            Vec::new()
        }
    };

    let observations: Vec<CrateObservation> = Crate::ALL
        .into_iter()
        .map(|crate_| CrateObservation {
            crate_,
            named_by_record: corpus.as_ref().map(|corpus| corpus.contains(crate_.name())),
            // **An empty file list is `None`, not a list of facts that are all false.** The
            // reading this replaces treated "no readable source" and "a source with no `pub mod`"
            // as different answers, and they are: one is a refusal to guess and the other is a
            // measurement.
            sources: read_sources(crate_, root)
                .filter(|files| !files.is_empty())
                .map(|files| {
                    let all = files.iter().map(|(_, text)| text.as_str()).collect::<String>();
                    SourceFacts {
                        moduled: all.contains("pub mod "),
                        tested: all.contains("#[test]"),
                        refusing: all.contains("RefusalKind::"),
                    }
                }),
            claim: claim_of(&claims, crate_).map(|claim| ClaimObservation {
                claimed: claim.is_claimed(),
                witness: claim.witness.clone(),
                line: claim.line,
                why_unclaimed: claim.why_unclaimed(),
            }),
        })
        .collect();

    if !refusals.is_empty() {
        return Err(refusals);
    }
    Ok(observations)
}

/// Measure every crate. **All of them, including the new ones** — the point is
/// one census, not two.
///
/// The reading half is [`observe`]; this function is `observe` plus the rules, and the rules live in
/// `crate::decision` so that this reader and any second instrument cannot answer differently by
/// accident.
pub fn census(root: &Path) -> Result<Census, RefusalSet> {
    let observations = observe(root)?;
    Ok(Census { readings: observations.iter().map(reading_of).collect() })
}

/// One observation, ruled on every axis.
fn reading_of(observation: &CrateObservation) -> Reading {
    let mut cells = [Establishment::Unchecked; AXIS_COUNT];
    let mut why: [Option<String>; AXIS_COUNT] = [const { None }; AXIS_COUNT];
    for (index, axis) in AXES.iter().enumerate() {
        let (state, reason) = crate::decision::rule_for_axis(observation, *axis);
        cells[index] = state;
        why[index] = reason;
    }
    Reading { crate_: observation.crate_, cells, why }
}

/// Every document's text, or `None` if any of them could not be read.
///
/// **All or nothing, on purpose.** A census that searched a partial record would
/// report a crate as unnamed when the document naming it was the one that failed
/// to read, and "the record did not mention this" is a serious enough claim that
/// it is not worth making on a partial corpus.
fn read_corpus(root: &Path, refusals: &mut RefusalSet) -> Option<String> {
    let mut corpus = String::new();
    for document in crate::record::DOCUMENTS {
        match read_document(root, document) {
            Ok(text) => {
                corpus.push_str(document.path);
                corpus.push('\n');
                corpus.push_str(&text);
            }
            Err(found) => {
                refusals.absorb(found);
            }
        }
    }
    if refusals.is_empty() {
        Some(corpus)
    } else {
        None
    }
}

impl Axis {
    /// A stable name. Never prose.
    pub const fn name(self) -> &'static str {
        match self {
            Axis::Named => "named",
            Axis::Moduled => "moduled",
            Axis::Tested => "tested",
            Axis::Refusing => "refusing",
            Axis::Claimed => "claimed",
        }
    }
}

/// Read every `.rs` file in a crate's `src/`, plus its `build.rs` if it has one.
///
/// **`src/*.rs` only, not the whole tree.** Tests in `tests/` are real tests and
/// are counted; a walk over `target/` would count the same test sixty times and
/// make the number meaningless.
fn read_sources(crate_: Crate, root: &Path) -> Option<Vec<(std::path::PathBuf, String)>> {
    let mut files = Vec::new();
    let src = root.join(crate_.path()).join("src");
    let entries = std::fs::read_dir(&src).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            if let Ok(text) = std::fs::read_to_string(&path) {
                files.push((path, text));
            }
        }
    }
    // `build.rs` is the gate. Not counting it would understate the one crate
    // whose build *is* its program.
    let build = root.join(crate_.path()).join("build.rs");
    if let Ok(text) = std::fs::read_to_string(&build) {
        files.push((build, text));
    }
    // A binary crate's tests live in the same `src/` tree, so nothing extra.
    Some(files)
}

/// **The runtime anti-fabrication guard.** `axis.rs`'s `check_coverage`, and the
/// reason the pattern is not merely a way of writing.
///
/// A pattern is a way of writing. `CRATES_ARE_TOTAL` says the table has twelve
/// rows; only this says a **measurement actually happened for all twelve**, and
/// that a cell was not left `Unchecked` by accident.
///
/// An `Unchecked` cell is the dangerous one. It means *no instrument ran*, and a
/// report full of `Unchecked` reads as complete to anyone who does not know the
/// difference. So this refuses it, and names how many cells are unmeasured.
pub fn check_coverage(census: &Census) -> Result<(), RefusalSet> {
    let mut refusals = RefusalSet::new();

    if census.readings.len() != CRATE_COUNT {
        refusals.refuse(
            RefusalKind::UncalibratedOracle,
            format!(
                "the census holds {} readings and this project has {CRATE_COUNT} crates. A census \
                 that did not measure everything cannot be called a census",
                census.readings.len()
            ),
        );
        return Err(refusals);
    }

    let unmeasured: Vec<String> = census
        .readings
        .iter()
        .flat_map(|reading| {
            reading
                .cells
                .iter()
                .enumerate()
                .filter(|(_, cell)| **cell == Establishment::Unchecked)
                .map(move |(index, _)| format!("{}/{}", reading.crate_.name(), axis_ref(index).name()))
        })
        .collect();

    if !unmeasured.is_empty() {
        refusals.refuse(
            RefusalKind::UncalibratedOracle,
            format!(
                "{} of {CRATE_COUNT}×{AXIS_COUNT} cells were never measured: {}. An unmeasured \
                 cell is not a passing cell, and a report that cannot tell them apart is worse \
                 than no report",
                unmeasured.len(),
                unmeasured.join(", ")
            ),
        );
    }

    refusals.into_result(())
}

/// The report a reader sees. One line per crate, one column per axis.
pub fn report(census: &Census) -> String {
    let mut out = String::new();
    out.push_str("crate                ");
    for axis in AXES {
        out.push_str(&format!("{:<9}", axis.name()));
    }
    out.push('\n');
    for reading in &census.readings {
        out.push_str(&format!("{:<20}", reading.crate_.name()));
        for index in 0..AXIS_COUNT {
            out.push_str(&format!("{:<9}", reading.cells[index].slug()));
        }
        out.push('\n');
    }
    out
}

/// **Check 3.** The table's own shape, separate from any measurement.
///
/// The `axis.rs` split: *is the declaration well formed* is answerable on a tree
/// that is mid-build, and *does it measure what it must* is not. Putting both in
/// one function would make the first unanswerable whenever the second fails.
pub fn check_table_shape() -> Result<(), RefusalSet> {
    let mut refusals = RefusalSet::new();

    for index in 0..CRATE_COUNT {
        let expected = CRATES[index];
        let row = ROWS[index];
        if row.crate_ != expected {
            refusals.refuse(
                RefusalKind::FabricatedFeature,
                format!(
                    "crate row {index} is {} but the crate order says it should be {}. The two \
                     tables must agree, or a report reads one row under another's name",
                    row.crate_.name(),
                    expected.name()
                ),
            );
        }
        if row.means.trim().is_empty() {
            refusals.refuse(
                RefusalKind::NoReading,
                format!(
                    "{} has a row with no `means`. A row that states nothing is a row that \
                     licenses anything",
                    row.crate_.name()
                ),
            );
        }
    }

    // Every crate reachable by name, and nothing reachable that is not a crate.
    for crate_ in Crate::ALL {
        if Crate::parse(crate_.name()) != Ok(crate_) {
            refusals.refuse(
                RefusalKind::FabricatedFeature,
                format!("{} does not parse back to itself", crate_.name()),
            );
        }
    }
    if Crate::parse("voxel-envelope").is_ok() {
        refusals.refuse(
            RefusalKind::FabricatedFeature,
            "voxel-envelope parses as a crate, and no such crate exists. R-59.2 requires it across \
             a crate boundary, so it is owed — but a name that resolves before the crate does is \
             a gate reporting on something that is not there"
                .to_string(),
        );
    }

    refusals.into_result(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::workspace_root;

    fn real_root() -> std::path::PathBuf {
        workspace_root(Path::new(env!("CARGO_MANIFEST_DIR")))
            .expect("a workspace root above the crate")
    }

    // ---- the pattern's own guarantees -----------------------------------

    #[test]
    fn every_crate_has_a_row_and_a_witness() {
        let _ = CRATES_ARE_TOTAL;
        check_table_shape().expect("the crate table is well formed");
        for crate_ in Crate::ALL {
            assert!(row_of(crate_).is_some(), "{} has no row", crate_.name());
            assert!(!crate_.witness_of().is_empty());
        }
    }

    #[test]
    fn a_crate_name_round_trips_and_an_unknown_one_is_refused() {
        for crate_ in Crate::ALL {
            assert_eq!(Crate::parse(crate_.name()), Ok(crate_));
        }
        let error = Crate::parse("voxel-envelope").expect_err("that crate does not exist yet");
        assert!(error.contains("12"), "the refusal should say how many there are: {error}");
    }

    #[test]
    fn every_crate_has_a_witness_file_on_disk() {
        // Measured, not asserted: a crate that declares a witness which is not
        // there is a table lying about the tree.
        let root = real_root();
        for crate_ in Crate::ALL {
            let path = root.join(crate_.witness_of());
            assert!(path.exists(), "{} claims a witness at {} and it is not there", crate_.name(), crate_.witness_of());
        }
    }

    // ---- the census, on this tree ---------------------------------------

    #[test]
    fn the_census_measures_every_crate_and_every_cell() {
        let census = census(&real_root()).expect("the workspace is readable");
        assert_eq!(census.readings.len(), CRATE_COUNT);
        check_coverage(&census).expect("no cell went unmeasured");
    }

    #[test]
    fn a_census_is_deterministic() {
        let root = real_root();
        assert_eq!(census(&root).unwrap(), census(&root).unwrap());
    }

    #[test]
    fn the_census_finds_the_untested_crates_rather_than_this_module_asserting_it() {
        // **The finding this module exists to produce.** `shploingus` carries no
        // `#[test]`, which the gate could not see before because nothing looked
        // at it. Measured here, not hard-coded as a claim: if someone adds a test
        // the assertion below fails and that is the correct outcome.
        //
        // **Corrected by measurement.** The first draft of this test asserted
        // that `voxel-ui` was also untested, on the strength of a one-line shell
        // count taken while writing the module header. That count was wrong:
        // `voxel-ui` is a single 1,808-line `lib.rs` carrying 33 tests, and a
        // per-file loop had counted the *files* rather than the tests. The census
        // read the tree properly and disagreed with the note, which is the whole
        // argument for measuring instead of asserting — so the test now says
        // what the tree says.
        let census = census(&real_root()).expect("the workspace is readable");
        let untested: Vec<&str> = census
            .readings
            .iter()
            .filter(|reading| reading.cell(Axis::Tested) != Establishment::Active)
            .map(|reading| reading.crate_.name())
            .collect();
        assert_eq!(
            untested,
            vec!["shploingus"],
            "the set of crates with no test changed. That is a finding to record in the ledger, \
             not a failing expectation to relax"
        );
    }

    #[test]
    fn a_single_file_crate_is_measured_by_its_tests_not_by_its_file_count() {
        // The defect the correction above came from, pinned so it cannot come
        // back: `voxel-ui` is one file with many tests, and any measurement that
        // counts files rather than `#[test]` will call it untested.
        let census = census(&real_root()).expect("the workspace is readable");
        let ui = census.reading_of(Crate::Ui).expect("voxel-ui is measured");
        assert_eq!(ui.cell(Axis::Tested), Establishment::Active);
        let text = std::fs::read_to_string(
            real_root().join("crates/voxel-ui/src/lib.rs"),
        )
        .expect("voxel-ui's single source file reads");
        assert!(
            text.matches("#[test]").count() > 1,
            "voxel-ui has {} tests in one file; if this is 1 the census's reading needs checking",
            text.matches("#[test]").count()
        );
    }

    #[test]
    fn a_crate_that_cannot_refuse_is_reported_as_inactive_with_a_reason() {
        let census = census(&real_root()).expect("the workspace is readable");
        let noise = census.reading_of(Crate::Noise).expect("voxel-noise is measured");
        assert_eq!(noise.cell(Axis::Refusing), Establishment::Inactive);
        assert!(
            noise.why_at(Axis::Refusing).expect("a non-active cell says why").contains("RefusalKind"),
            "the reason must be readable, not a status flip"
        );
    }

    #[test]
    fn an_unmeasured_cell_is_refused_rather_than_reported_as_a_pass() {
        // The anti-fabrication guard, tested by making the failure happen.
        let census = Census {
            readings: Crate::ALL
                .iter()
                .map(|crate_| Reading {
                    crate_: *crate_,
                    cells: [Establishment::Active; AXIS_COUNT],
                    why: [const { None }; AXIS_COUNT],
                })
                .collect(),
        };
        check_coverage(&census).expect("a full census passes");

        let mut short = census.clone();
        short.readings[0].cells[0] = Establishment::Unchecked;
        let error = check_coverage(&short).expect_err("an unmeasured cell is refused");
        assert!(
            error.all().iter().any(|r| r.detail().contains("voxel-model/named")),
            "the refusal must name the cell: {error}"
        );
    }

    #[test]
    fn a_short_census_is_refused_rather_than_being_called_one() {
        let census = Census { readings: Vec::new() };
        let error = check_coverage(&census).expect_err("no readings is not a census");
        assert!(error.all()[0].detail().contains("0 readings"), "{error}");
    }

    #[test]
    fn the_report_prints_every_crate_and_every_axis() {
        let census = census(&real_root()).expect("the workspace is readable");
        let text = report(&census);
        for crate_ in Crate::ALL {
            assert!(text.contains(crate_.name()), "{} is missing from the report", crate_.name());
        }
        for axis in AXES {
            assert!(text.contains(axis.name()), "{} is missing from the header", axis.name());
        }
    }

    #[test]
    fn a_gap_is_reported_with_its_axis_and_reason() {
        let census = census(&real_root()).expect("the workspace is readable");
        let ui = census.reading_of(Crate::Ui).expect("voxel-ui is measured");
        let gaps = ui.gaps();
        assert!(!gaps.is_empty(), "voxel-ui has no gaps, which contradicts the census");
        for (axis, _, why) in &gaps {
            assert!(!axis.name().is_empty());
            assert!(
                why.is_some(),
                "{} is {} and says nothing about why; a non-active cell with no reason is a \
                 status flip",
                ui.crate_.name(),
                axis.name()
            );
        }
    }

    #[test]
    fn an_incomplete_crate_is_reported_as_incomplete_and_not_hidden() {
        let census = census(&real_root()).expect("the workspace is readable");
        // Every crate here has at least one axis that is not active, because the
        // axes are strict. The point is that this is *visible*.
        assert!(
            !census.incomplete().is_empty(),
            "a census where nothing is incomplete would mean the axes measure nothing"
        );
    }

    #[test]
    fn the_census_prints_its_own_measurements() {
        // C-45 again: the report is visible, unfiltered, on every run. A gate
        // whose numbers can only be seen by editing a test is a gate nobody
        // reads twice.
        let census = census(&real_root()).expect("the workspace is readable");
        println!("--- crate census ---");
        print!("{}", report(&census));
        for reading in &census.readings {
            for (axis, state, why) in reading.gaps() {
                println!("  gap {}/{}: {}{}", reading.crate_.name(), axis.name(), state.slug(),
                    why.map(|w| format!(" — {w}")).unwrap_or_default());
            }
        }

        let active = census
            .readings
            .iter()
            .flat_map(|reading| reading.cells.iter())
            .filter(|cell| **cell == Establishment::Active)
            .count();
        println!("cells active {active} of {}", CRATE_COUNT * AXIS_COUNT);

        assert!(active > 0, "a census with no active cells measures nothing");
    }

    #[test]
    fn the_axes_are_five_and_each_names_itself() {
        assert_eq!(AXES.len(), AXIS_COUNT);
        assert_eq!(AXIS_COUNT, 5, "the axis count is a ruling, not an accident");
        for axis in AXES {
            assert!(!axis.name().is_empty());
        }
    }
}
