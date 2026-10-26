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

pub mod folders;

use std::collections::BTreeMap;
use std::path::Path;

use voxel_model::identity::{Digest, Identity};
use voxel_model::{RefusalKind, RefusalSet};


use crate::bloat;
use crate::provenance;
use crate::record::read_document;

/// How many crates the workspace holds. **A literal**, for the reason
/// [`crate::features::FEATURE_COUNT`] is one.
pub const CRATE_COUNT: usize = 29;

/// Every crate in the workspace, in dependency order.
///
/// Dependency order rather than alphabetical, because the point of the list is
/// to read as the build reads: `voxel-model` refuses before anything asks it
/// to, and a reader who sees `voxel-package` first has not been told what it
/// stands on.
///
/// **This list was twelve and the workspace holds sixteen.** `voxel-light`,
/// `voxel-audio`, `voxel-mcp` and `voxel-von` were added to the manifest and
/// never added here, so four members were measured by **nothing** — not by the
/// census, not by the conformance verdict, not by the patch coverage, which all
/// iterate this list. The count was a literal that nothing compared against the
/// manifest, so the blind spot was silent: the report looked complete because it
/// was complete **about the twelve it knew**. The list is now every workspace
/// member, and [`crate::census::tests::every_workspace_member_is_in_the_table`]
/// reads the manifest and refuses when the two disagree, so the next crate
/// cannot be added without being tracked.
pub const CRATES: [Crate; CRATE_COUNT] = [
    Crate::Model,
    Crate::Ladder,
    Crate::Coverage,
    Crate::Unit,
    Crate::Claim,
    Crate::View,
    Crate::Civilians,
    Crate::Being,
    Crate::Matrix,
    Crate::Verify,
    Crate::Artifact,
    Crate::Security,
    Crate::AxisMcp,
    Crate::Balance,
    Crate::Noise,
    Crate::Light,
    Crate::Mesh,
    Crate::Shapers,
    Crate::Audio,
    Crate::Stream,
    Crate::Instrument,
    Crate::Appearance,
    Crate::Work,
    Crate::Ui,
    Crate::Client,
    Crate::Package,
    Crate::Von,
    Crate::Mcp,
    Crate::Shploingus,
];

/// **Check 2 of 2.** Every crate has a row, in a `const`.
pub const CRATES_ARE_TOTAL: [(); CRATE_COUNT] = [(); CRATES.len()];

/// A crate in this workspace. One variant each.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Crate {
    Model,
    /// `crates/axis/ladder` — the axis vocabulary the four prose layers share (PATCH-042).
    Ladder,
    /// `crates/axis/coverage` — the coverage table and the exact-partition audit (PATCH-042).
    Coverage,
    /// `crates/axis/unit` — the addressable paragraph, with a deterministic id (PATCH-042).
    Unit,
    /// `crates/axis/claim` — the quote, its kind and its computed support (PATCH-042).
    Claim,
    /// `crates/axis/view` — the three-section document with no untraceable line (PATCH-042).
    View,
    /// `crates/axis/civilians` — the civilian gates over the model's ported systems (PATCH-042).
    Civilians,
    /// `crates/axis/being` — the being substrate (PATCH-043).
    Being,
    /// `crates/axis/matrix` — the calibration matrix: segments, cells, stars, tiers and recipes
    /// (PATCH-042 §The calibration matrix).
    Matrix,
    /// `crates/axis/verify` — the walk and the report: the one place a matrix reading is produced
    /// from the census, the conformance files, the workspace's witnesses and the ladders
    /// (PATCH-042 §The scrubber).
    Verify,
    /// `crates/axis/artifact` — the package as a value: cited and laddered statements, the pin over
    /// its bytes, the signing seam, and the reproduction claim (PATCH-042, the five requirements).
    Artifact,
    /// `crates/axis/security` — the trust boundary's key: a signing key held outside the checkout,
    /// the signature over a payload or a build digest, and the refusals that name what is missing.
    Security,
    /// `crates/axis/mcp` — the authority an agent-facing surface speaks through: the verified matrix
    /// and its package, the recipe for a named ticket, and an apply that decides only under a
    /// ticket-scoped grant signed by the security layer (PATCH-044).
    AxisMcp,
    /// `crates/axis/balance` — the CPU/GPU balance census: each crate's Rust and WGSL measured from
    /// the tree, and the declared visual inventory with the renderer every item reaches or the
    /// count that reaches none (PATCH-045).
    Balance,
    Noise,
    Light,
    Mesh,
    Shapers,
    Audio,
    Stream,
    Instrument,
    Appearance,
    Work,
    Ui,
    Client,
    Package,
    Von,
    Mcp,
    Shploingus,
}

impl Crate {
    /// Every crate, in dependency order.
    pub const ALL: [Crate; CRATE_COUNT] = CRATES;

    /// The directory name, which is also the crate name.
    pub const fn name(self) -> &'static str {
        match self {
            Crate::Model => "voxel-model",
            Crate::Ladder => "voxel-axis-ladder",
            Crate::Coverage => "voxel-axis-coverage",
            Crate::Unit => "voxel-axis-unit",
            Crate::Claim => "voxel-axis-claim",
            Crate::View => "voxel-axis-view",
            Crate::Civilians => "voxel-axis-civilians",
            Crate::Being => "voxel-axis-being",
            Crate::Matrix => "voxel-axis-matrix",
            Crate::Verify => "voxel-axis-verify",
            Crate::Artifact => "voxel-axis-artifact",
            Crate::Security => "voxel-axis-security",
            Crate::AxisMcp => "voxel-axis-mcp",
            Crate::Balance => "voxel-axis-balance",
            Crate::Noise => "voxel-noise",
            Crate::Light => "voxel-light",
            Crate::Mesh => "voxel-mesh",
            Crate::Shapers => "voxel-shapers",
            Crate::Audio => "voxel-audio",
            Crate::Stream => "voxel-stream",
            Crate::Instrument => "voxel-instrument",
            Crate::Appearance => "voxel-appearance",
            Crate::Work => "voxel-work",
            Crate::Ui => "voxel-ui",
            Crate::Client => "voxel-client",
            Crate::Package => "voxel-package",
            Crate::Von => "voxel-von",
            Crate::Mcp => "voxel-mcp",
            Crate::Shploingus => "shploingus",
        }
    }

    /// The path a reader opens. Relative to the workspace root.
    pub const fn path(self) -> &'static str {
        match self {
            Crate::Model => "crates/voxel-model",
            Crate::Ladder => "crates/axis/ladder",
            Crate::Coverage => "crates/axis/coverage",
            Crate::Unit => "crates/axis/unit",
            Crate::Claim => "crates/axis/claim",
            Crate::View => "crates/axis/view",
            Crate::Civilians => "crates/axis/civilians",
            Crate::Being => "crates/axis/being",
            Crate::Matrix => "crates/axis/matrix",
            Crate::Verify => "crates/axis/verify",
            Crate::Artifact => "crates/axis/artifact",
            Crate::Security => "crates/axis/security",
            Crate::AxisMcp => "crates/axis/mcp",
            Crate::Balance => "crates/axis/balance",
            Crate::Noise => "crates/voxel-noise",
            Crate::Light => "crates/voxel-light",
            Crate::Mesh => "crates/voxel-mesh",
            Crate::Shapers => "crates/voxel-shapers",
            Crate::Audio => "crates/voxel-audio",
            Crate::Stream => "crates/voxel-stream",
            Crate::Instrument => "crates/voxel-instrument",
            Crate::Appearance => "crates/voxel-appearance",
            Crate::Work => "crates/voxel-work",
            Crate::Ui => "crates/voxel-ui",
            Crate::Client => "crates/voxel-client",
            Crate::Package => "crates/voxel-package",
            Crate::Von => "crates/voxel-von",
            Crate::Mcp => "crates/voxel-mcp",
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
            Crate::Ladder => "crates/axis/ladder/src/lib.rs",
            Crate::Coverage => "crates/axis/coverage/src/lib.rs",
            Crate::Unit => "crates/axis/unit/src/lib.rs",
            Crate::Claim => "crates/axis/claim/src/lib.rs",
            Crate::View => "crates/axis/view/src/lib.rs",
            Crate::Civilians => "crates/axis/civilians/src/lib.rs",
            Crate::Being => "crates/axis/being/src/lib.rs",
            Crate::Matrix => "crates/axis/matrix/src/lib.rs",
            Crate::Verify => "crates/axis/verify/src/lib.rs",
            Crate::Artifact => "crates/axis/artifact/src/lib.rs",
            Crate::Security => "crates/axis/security/src/lib.rs",
            Crate::AxisMcp => "crates/axis/mcp/src/lib.rs",
            Crate::Balance => "crates/axis/balance/src/lib.rs",
            Crate::Noise => "crates/voxel-noise/src/lib.rs",
            Crate::Light => "crates/voxel-light/src/lib.rs",
            Crate::Mesh => "crates/voxel-mesh/src/lib.rs",
            Crate::Shapers => "crates/voxel-shapers/src/lib.rs",
            Crate::Audio => "crates/voxel-audio/src/lib.rs",
            Crate::Stream => "crates/voxel-stream/src/lib.rs",
            Crate::Instrument => "crates/voxel-instrument/src/lib.rs",
            Crate::Appearance => "crates/voxel-appearance/src/lib.rs",
            Crate::Work => "crates/voxel-work/src/lib.rs",
            Crate::Ui => "crates/voxel-ui/src/lib.rs",
            Crate::Client => "crates/voxel-client/src/lib.rs",
            Crate::Package => "crates/voxel-package/src/lib.rs",
            Crate::Von => "crates/voxel-von/src/lib.rs",
            Crate::Mcp => "crates/voxel-mcp/src/lib.rs",
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
    CrateRow { crate_: Crate::Ladder, means: "the axis vocabulary the layers above share: nine ladders, their tokens, the bare-token rule and the coverage pin", required: true },
    CrateRow { crate_: Crate::Coverage, means: "the coverage gate and the partition audit: every ladder has a row, and a text's spans claim every character exactly once", required: true },
    CrateRow { crate_: Crate::Unit, means: "the addressable paragraph: a deterministic unit id, its exact range and the source slice it quotes", required: true },
    CrateRow { crate_: Crate::Claim, means: "one claim per unit: the exact quote, a prose kind with its basis, and a support only derive can compute", required: true },
    CrateRow { crate_: Crate::View, means: "the three-section document: every line resolves to a claim and every carried claim has its line, and an empty section prints none rather than disappearing", required: true },
    CrateRow { crate_: Crate::Civilians, means: "the civilian gates over the model's ported systems: a selection's need, budget and estimate, a task's planned material and checked completion, a witnessed ladder and an arrangement that round-trips", required: true },
    CrateRow { crate_: Crate::Being, means: "one being record, seven faces: a civilian, a mob, a plant and a ghost are the same substrate before they are anything else", required: true },
    CrateRow { crate_: Crate::Matrix, means: "the calibration matrix: every segment read on every axis it has, a star lit only by a predicate that ran, a tier computed from the cell's own requirements, and a recipe line a person can apply", required: true },
    CrateRow { crate_: Crate::Verify, means: "the walk and the report: one reading per segment produced from the census and its pins, the conformance files, the workspace's declared witnesses and the ladders — with the source named, and a refusal rather than a pass where a source cannot witness", required: true },
    CrateRow { crate_: Crate::Artifact, means: "the package as a value: plain-text statements each at a file:line and on a ladder, the pin over its bytes, the signing seam it does not implement, and the reproduction claim that refuses rather than promises", required: true },
    CrateRow { crate_: Crate::Security, means: "the trust boundary's key: a signing key held outside the checkout and refused inside it, the HMAC-SHA256 signature over a package's bytes or a build digest, and the four refusals that keep an unknown key, an unknown algorithm and a moved byte apart", required: true },
    CrateRow { crate_: Crate::AxisMcp, means: "the authority an agent-facing surface speaks through: the verified matrix and its package, a recipe for a named ticket, and an apply that decides only under a ticket-scoped grant the security layer signed — refusing no credential, another ticket, another cell, an expired grant and an unwitnessed cell", required: true },
    CrateRow { crate_: Crate::Balance, means: "the CPU/GPU balance census: every crate's Rust and WGSL measured from the tree, and a declared visual inventory held to the source it names — with the items no renderer reaches counted and printed rather than omitted", required: true },
    CrateRow { crate_: Crate::Noise, means: "deterministic noise, so the same seed yields the same world", required: true },
    CrateRow { crate_: Crate::Light, means: "the band table and the real sun: full-spectrum radiometry over declared bands", required: true },
    CrateRow { crate_: Crate::Mesh, means: "chunks become meshes, and a mesh can be refined or raycast", required: true },
    CrateRow { crate_: Crate::Shapers, means: "stages and shapers produce the world, deterministically, from a seed", required: true },
    CrateRow { crate_: Crate::Audio, means: "rain as synthesis, not playback: a pure function of tick and address, with no device to open", required: false },
    CrateRow { crate_: Crate::Stream, means: "what is resident, and what is wanted next", required: true },
    CrateRow { crate_: Crate::Instrument, means: "measurement that reports rather than asserts", required: false },
    CrateRow { crate_: Crate::Appearance, means: "how a material looks, and where that answer came from", required: false },
    CrateRow { crate_: Crate::Work, means: "the budget and the pool: work is bounded and dispatched", required: true },
    CrateRow { crate_: Crate::Ui, means: "the design system resolves colour, type and elevation, and refuses unrepresentable values", required: false },
    CrateRow { crate_: Crate::Client, means: "the GPU side, shared with the offscreen test that measures it", required: true },
    CrateRow { crate_: Crate::Package, means: "a package is admitted by validation, never by optimism", required: true },
    CrateRow { crate_: Crate::Von, means: "the classifier port: the gate's questions answered by a model this project did not write, and never took on faith", required: false },
    CrateRow { crate_: Crate::Mcp, means: "the gate's read surface: a tool server over numbers the package already computed, so a tool call cannot disagree with the build", required: false },
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

/// The seven things a crate is measured on.
///
/// **Seven, deliberately.** Each is checkable from this tree without judgement.
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
    /// **The doc stage: whether this crate is documented, as a measurement.** It reads the
    /// **compiler's own count** ([`MISSING_DOCS_PATH`]): how many public items `missing_docs` —
    /// enabled in every crate root — reports without a doc comment, produced by the recipe in this
    /// module's header. `0` is `active`/`documented`; more is `inactive-with-reason` naming the
    /// count; **a crate with no line in the reading is refused**, because no instrument asked the
    /// compiler about it and *not measured is not a pass*; and a crate whose declared **pin** no
    /// longer matches the bytes on disk is
    /// [`Establishment::Stale`](crate::features::Establishment::Stale) — the count is about a tree
    /// this one is not. That last state is the difference between a reading and a number: the count
    /// cannot be re-taken from inside the gate's own build script, but the bytes it was taken over
    /// can be hashed there, so a reading that has drifted is named rather than repeated.
    ///
    /// **Why the compiler and not a header.** Until 2026-10-10 this axis asked whether the entry
    /// file opened with a `//!` block, and every crate had one — so the axis was `active` for all
    /// sixteen and could only turn false if someone deleted a line at the top of a file. That is a
    /// check on the *presence of a claim*, not on the documentation. `missing_docs` counts the thing
    /// the axis is named for, per item, and falls when docs are written and rises when they are not.
    /// The user's ruling stands: the documentation belongs in the crate's own rustdoc, next to the
    /// values it explains. What changed is that the gate now asks the compiler whether it is there.
    /// A crate keeping an invented record beside the chain ([`bloat::INVENTED_RECORD_EXTENSIONS`]) is
    /// refused whatever its documentation count reads.
    Documented,
    /// **Whether a call site exists outside this crate's own sources and tests.** The question the
    /// six above could not ask: a crate can be named, moduled, tested, refusing, claimed and
    /// documented and still be reached by nothing. `voxel-audio` and `voxel-instrument` are the
    /// live subjects; a re-export is not a call, and an inline `#[cfg(test)]` module is not a caller.
    Exercised,
}

/// How many axes. A literal, as everywhere else.
pub const AXIS_COUNT: usize = 7;

/// Every axis, in the order the report prints them.
pub const AXES: [Axis; AXIS_COUNT] = [
    Axis::Named,
    Axis::Moduled,
    Axis::Tested,
    Axis::Refusing,
    Axis::Claimed,
    Axis::Documented,
    Axis::Exercised,
];

/// The declared share of a crate's source that may be comment before the census calls it
/// documentation rather than code.
///
/// **A declared frame, not a preference.** It is measured on the share of a crate that is *plain*
/// comment — **rustdoc is excluded** ([`bloat::Prose::plain_percent`]), because the user's ruling of
/// 2026-10-10 is that documentation belongs in the crate's own rustdoc, next to the values it
/// explains. A frame counted over rustdoc would charge every crate for writing the thing it is owed.
/// The tree measures 26% comment overall, so this frame sits below current practice and the stage
/// reports every crate's plain share against it (3–29%). What the frame **refuses** is a crate whose
/// plain commentary has risen past its declared share — commentary, not the rustdoc this project is
/// owed. A frame set to whatever the tree already does would be a check that cannot fail.
///
/// **The only reader of this frame is [`check_prose_frame`].** [`Axis::Documented`] does *not* consult
/// it: that axis reads the compiler's own count ([`MISSING_DOCS_PATH`]), because a share of bytes
/// cannot say whether any given item has a doc comment.
pub const PROSE_FRAME_PERCENT: usize = 10;

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
    /// `None` when the crate has no readable source, so the prose share was never measured.
    /// **Not the same as "no prose"**, which is a measured fact.
    pub prose: Option<ProseObservation>,
    /// **Every call site outside this crate's own sources and tests**, as `path:line`.
    ///
    /// `None` when the crate has no readable source; `Some(vec![])` is the measured fact that
    /// nothing outside it calls it. A re-export is not a call, and a line inside an inline
    /// `#[cfg(test)]` module is not a caller — those are excluded by [`exercised_callers`].
    pub exercised: Option<Vec<String>>,
}

/// **The compiler's documentation count for one crate, and the pin that dates it.**
///
/// **A count without a pin is a number about an unknown tree.** `missing_docs` is read by a command
/// a person runs (`conformance/missing-docs.txt` carries the recipe), so unlike the prose ratchet the
/// gate cannot take it again from inside its own build script. What the gate *can* do is check
/// whether the bytes exist that the count was taken over: `declared` is the pin written beside the
/// count, `measured` is the same digest over the files on disk now, and the two differing is what
/// [`Establishment::Stale`](crate::features::Establishment::Stale) renders. The comparison is
/// digest-to-digest over the tree's one hashing scheme rather than a string comparison, the same
/// rule [`crate::civilians`] applies to a citation.
///
/// Both pins are reported so the reason can name them: a reader who is told *stale* and not told
/// which digest moved cannot tell a real edit from a corrupted reading.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissingDocsReading {
    /// How many public items `missing_docs` reported without a doc comment, as declared.
    pub missing: usize,
    /// The pin declared beside the count: `sha256:` over the crate's covered files, as they were
    /// when the count was taken.
    pub declared: Identity,
    /// The same pin over the bytes on disk now. Equal to `declared` exactly when the count was taken
    /// over the tree as it stands.
    pub measured: Identity,
}

/// What a crate's source files were read as containing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceFacts {
    pub moduled: bool,
    pub tested: bool,
    pub refusing: bool,
}/// Where a crate's rationale was found, and where it was not. Facts only; the rule lives in
/// `crate::decision`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProseObservation {
    /// False means the share below is not a measurement.
    pub measured: bool,
    /// Why there is no measurement. Present exactly when `measured` is false.
    pub why_unmeasured: Option<String>,
    /// Text files the share was measured over — every covered file in the crate's directory
    /// that decoded. **The denominator is every covered file, not a chosen subset.**
    pub files: usize,
    /// Every covered file in the crate's directory, whether or not it carried the share.
    pub ruled_files: usize,
    /// Covered files in the crate's directory that did not decode as UTF-8. **Named, never
    /// dropped** — a file that was not read is a fact the report carries, not a silence.
    pub undecodable: Vec<std::path::PathBuf>,
    /// Comment bytes as a whole percent of the crate's text, **rustdoc included**. `None` when
    /// unmeasured.
    pub comment_percent: Option<usize>,
    /// **The share the prose frame is about**: comment bytes that are *not* rustdoc, as a whole
    /// percent. Rustdoc is documentation and is measured apart, because the two answer to different
    /// rules (the user's ruling, 2026-10-10).
    pub plain_percent: Option<usize>,
    /// **The compiler's documentation reading for this crate, and the bytes it was taken over**: how
    /// many public items `missing_docs` reports without a doc comment, and the pin that says whether
    /// the count is still about *this* tree ([`MISSING_DOCS_PATH`]). `None` means **no line asked the
    /// compiler about this crate**, which is not a pass — it is [`Axis::Documented`]'s `unmeasured`
    /// refusal, by name.
    pub missing_docs: Option<MissingDocsReading>,
    /// A document under `docs/` that **defines** this crate — a heading whose text begins with
    /// its name (`heading_defines`, measured by that private rule). A passing mention is not a definition
    /// and does not count.
    pub document: Option<String>,
    /// Invented records that name this crate (`bloat::invented_stores`). Reported, never narrowed.
    pub invented: Vec<String>,
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

    // **Walked once for the whole census**, not once per crate: the covered set, the docs corpus
    // and the invented-record check are properties of the tree, and twelve walks would be twelve
    // answers to one question.
    let covered = provenance::covered_files(root);
    // **One walk for the caller question**, beside the one walk for the sources: a crate that is
    // reached from another crate is a fact about the workspace, not about the crate's own files.
    let callers = exercised_callers(root, &covered);
    let docs = read_docs(root);
    let invented = bloat::invented_stores(root);

    // The compiler's own documentation count and its pin, read once for the whole census (one
    // file, sixteen crates). **An absent file is deliberately not a refusal here**: the axis refuses
    // each crate by name for having no reading, which is a refusal a reader can act on, where one
    // "the file is missing" would hide which crates are unmeasured behind which one is.
    let missing_docs = read_missing_docs(root, &mut refusals);

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
        .map(|crate_| {
            // Read once: a second read would be a second answer to what the crate contains, and
            // the fields below are questions about the same bytes.
            let sources = read_sources(&covered, crate_, root);
            CrateObservation {
            crate_,
            named_by_record: corpus.as_ref().map(|corpus| corpus.contains(crate_.name())),
            prose: sources.as_ref().map(|sources| {
                let prose = bloat::measure_prose(&sources.read);
                // **The pin, over the same bytes the count was taken over** — the files just read,
                // not a second walk. This is the one thing that makes a snapshot taken by hand
                // checkable from inside a build script: hashing is cheap and needs no compiler.
                let measured = missing_docs_pin(sources, root);
                ProseObservation {
                    measured: prose.measured,
                    why_unmeasured: prose.why_unmeasured.clone(),
                    files: prose.files,
                    ruled_files: sources.covered.len(),
                    undecodable: sources.undecodable.clone(),
                    comment_percent: prose.comment_percent(),
                    plain_percent: prose.plain_percent(),
                    missing_docs: missing_docs.get(crate_.name()).map(|(missing, declared)| {
                        MissingDocsReading { missing: *missing, declared: *declared, measured }
                    }),
                    document: defined_by_document(&docs, crate_),
                    invented: invented
                        .iter()
                        .filter(|store| store.path.contains(crate_.name()))
                        .map(|store| store.path.clone())
                        .collect(),
                }
            }),
            // **An empty file list is `None`, not a list of facts that are all false.** The
            // reading this replaces treated "no readable source" and "a source with no `pub mod`"
            // as different answers, and they are: one is a refusal to guess and the other is a
            // measurement.
            sources: sources
                .as_ref()
                .filter(|sources| !sources.read.is_empty())
                .map(|sources| {
                    let all = sources
                        .read
                        .iter()
                        .map(|(_, text)| text.as_str())
                        .collect::<String>();
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
            exercised: sources.as_ref().map(|_| {
                callers
                    .get(crate_.name())
                    .cloned()
                    .unwrap_or_default()
            }),
            }
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
            Axis::Documented => "documented",
            Axis::Exercised => "exercised",
        }
    }
}

/// A document under `docs/` carrying prose about a crate, or `None`.
///
/// **A table row is not prose about a crate.** [`declares_in_row`] exists because a row is a
/// *declaration*; a line naming the crate that is not a row is a sentence someone wrote about it.
/// **This is what the oracle's `Documented` instrument reads a crate's prose against**; the axis
/// itself no longer consults a heading — it reads the compiler's count — so this rule decides which
/// document a crate's commentary belongs to, not whether the crate is documented. It reuses this
/// module's own rule rather than writing a second one, so the two cannot drift.
fn defined_by_document(docs: &[(String, Vec<String>)], crate_: Crate) -> Option<String> {
    let name = crate_.name();
    docs.iter()
        .find(|(_, lines)| lines.iter().any(|line| heading_defines(line, name)))
        .map(|(file, _)| file.clone())
}

/// Whether one line is a heading whose text **begins** with `name`.
///
/// **A mention is not a definition.** This is the grammar `crate::compendium` measured for
/// anchors (C-61): a heading defines the thing it begins with, while a heading that cites a name
/// in its title is a *reference* to it — and reading those as definitions would have inflated
/// that record by 82. A crate named once in a 7,000-line ledger is not documented; a crate with a
/// section of its own is. Without this rule a heading reading cannot fail, which is the defect
/// [`Axis::Claimed`] already exists to avoid.
fn heading_defines(line: &str, name: &str) -> bool {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('#') {
        return false;
    }
    let text = trimmed.trim_start_matches('#').trim_start();
    let Some(rest) = text.strip_prefix(name) else {
        return false;
    };
    // A whole word, so `voxel-mesher` does not define `voxel-mesh`. Any whitespace separates,
    // which is also what makes a heading with a trailing space or newline count.
    rest.is_empty()
        || rest.starts_with(char::is_whitespace)
        || rest.starts_with(':')
        || rest.starts_with('-')
        || rest.starts_with('\u{2014}')
        || rest.starts_with('`')
}

/// A crate's files, split by whether they could be read as text.
///
/// **Total, and that is the type's whole job.** A walk that returned only what it could decode
/// would hide the files it could not, which is the silence this census exists to refuse.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrateSources {
    /// Every covered file in the crate's directory.
    pub covered: Vec<std::path::PathBuf>,
    /// The ones that decoded, with their text.
    pub read: Vec<(std::path::PathBuf, String)>,
    /// Covered files that did not decode. Named, never dropped.
    pub undecodable: Vec<std::path::PathBuf>,
}

/// Every covered file in a crate's directory, split by readability.
///
/// **The universe is [`provenance::covered_files`], not a list chosen here.** C-71.5 decided the
/// allowlist (`COVERED = rs/md/toml/wgsl`, `SKIPPED = target/.git`) for the working-tree digest,
/// and a second list in this module would be a second rule about what the project is. So this
/// walks what that module walks — a crate's `src/`, its **`tests/`**, its `build.rs`, its
/// `Cargo.toml` — and the tests in `tests/` are read, not skipped.
///
/// `None` means the crate's directory holds no covered file at all, which is a fact about the
/// tree rather than a failure to look.
/// **`covered` is passed in, not walked here.** The walk is one property of the tree, so twelve
/// crates must not cause twelve of it; a per-crate walk is the same answer computed twelve times.
/// **Every caller of every crate, found once.** A crate's own name is its directory name; Rust
/// spells it with underscores, so both spellings are accepted. The scan skips this crate's own
/// files, any inline `#[cfg(test)]` module, comment lines, and `pub use` re-exports — a re-export
/// is a name, not a call. A macro-generated call site is still invisible to a line reader; that is
/// the limit stated in the axis's own doc, not a claim that it was measured.
pub(crate) fn exercised_callers(
    root: &Path,
    covered: &[std::path::PathBuf],
) -> BTreeMap<&'static str, Vec<String>> {
    let mut callers: BTreeMap<&'static str, Vec<String>> =
        Crate::ALL.iter().map(|crate_| (crate_.name(), Vec::new())).collect();
    for path in covered {
        if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
            continue;
        }
        let relative = path.strip_prefix(root).unwrap_or(path);
        let relative = relative.to_string_lossy().replace('\\', "/");
        let Some(owner) = Crate::ALL.into_iter().find(|crate_| {
            relative == crate_.path() || relative.starts_with(&format!("{}/", crate_.path()))
        }) else {
            continue;
        };
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let production = production_lines(&text);
        for crate_ in Crate::ALL {
            if crate_ == owner {
                continue;
            }
            let module = crate_.name().replace('-', "_");
            if let Some((line, _)) = production
                .iter()
                .find(|(_, line)| calls_crate(line, &module))
            {
                let sites = callers.entry(crate_.name()).or_default();
                if sites.len() < 5 {
                    sites.push(format!("{relative}:{line}"));
                }
            }
        }
    }
    callers
}

/// Lines outside an inline test module: the caller question is about production code, not about a
/// test that names a crate in order to test the thing that names it.
fn production_lines(text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut skipping = false;
    let mut depth = 0i32;
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        if !skipping && trimmed.contains("#[cfg(test)]") {
            skipping = true;
            depth = brace_delta(line);
            // The attribute may sit on its own line; the opening brace is read on the next one. A
            // complete one-line module is not skipped, because there is nothing after its `}`.
            if line.contains('{') && line.contains('}') {
                skipping = false;
            }
            continue;
        }
        if skipping {
            let delta = brace_delta(line);
            depth += delta;
            if delta != 0 && depth <= 0 {
                skipping = false;
            }
            continue;
        }
        if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
            continue;
        }
        out.push((index + 1, line.to_string()));
    }
    out
}

/// Braces that belong to code, not to a string, a character literal or a comment. The test module
/// is skipped by matching its braces, so a `}` inside a fixture string must not end the skip early.
fn brace_delta(line: &str) -> i32 {
    let chars: Vec<char> = line.chars().collect();
    let mut depth = 0;
    let mut index = 0;
    let mut string = false;
    let mut character = false;
    let mut block_comment = false;
    let mut escaped = false;
    while index < chars.len() {
        let current = chars[index];
        let next = chars.get(index + 1).copied();
        if block_comment {
            if current == '*' && next == Some('/') {
                block_comment = false;
                index += 1;
            }
        } else if string || character {
            if escaped {
                escaped = false;
            } else if current == '\\' {
                escaped = true;
            } else if (string && current == '"') || (character && current == '\'') {
                string = false;
                character = false;
            }
        } else if current == '/' && next == Some('/') {
            break;
        } else if current == '/' && next == Some('*') {
            block_comment = true;
            index += 1;
        } else if current == '"' {
            string = true;
        } else if current == '\'' {
            character = true;
        } else if current == '{' {
            depth += 1;
        } else if current == '}' {
            depth -= 1;
        }
        index += 1;
    }
    depth
}

/// A crate module named as a path segment, not as a word in prose and not as a re-export.
fn calls_crate(line: &str, module: &str) -> bool {
    let trimmed = line.trim_start();
    if trimmed.starts_with("pub use ") || trimmed.starts_with("//") {
        return false;
    }
    let needle = format!("{module}::");
    if let Some(at) = line.find(&needle) {
        let before = line[..at].chars().next_back();
        if !before.is_some_and(|character| character.is_alphanumeric() || character == '_') {
            return true;
        }
    }
    trimmed.starts_with(&format!("use {module} "))
        || trimmed.starts_with(&format!("use {module};"))
        || trimmed.starts_with(&format!("extern crate {module};"))
}

pub(crate) fn read_sources(
    covered: &[std::path::PathBuf],
    crate_: Crate,
    root: &Path,
) -> Option<CrateSources> {
    let directory = root.join(crate_.path());
    let mine: Vec<std::path::PathBuf> = covered
        .iter()
        .filter(|path| path.starts_with(&directory))
        .cloned()
        .collect();
    if mine.is_empty() {
        return None;
    }
    let mut read = Vec::new();
    let mut undecodable = Vec::new();
    for path in &mine {
        match std::fs::read_to_string(path) {
            Ok(text) => read.push((path.clone(), text)),
            Err(_) => undecodable.push(path.clone()),
        }
    }
    Some(CrateSources { covered: mine, read, undecodable })
}

/// **The pin over a crate's covered files: what makes the compiler's count a checkable reading.**
///
/// The count in [`MISSING_DOCS_PATH`] is taken by a command a person runs, and a build script cannot
/// run the compiler from inside itself. Hashing the bytes it was taken over *is* available there —
/// over the same files the prose measure already read, so it costs one pass of sha256 and no second
/// walk. [`MissingDocsReading`] carries the declared pin and this one, and the axis renders their
/// difference as `stale`.
///
/// Three decisions are inside, and each is deliberate:
///
/// * **The relative path is in the digest, forward-slashed.** A rename or a move moves the pin even
///   when the bytes are identical (the rule [`provenance::measure`] states for the tree digest), and
///   keeping the path relative means the pin does not move when the checkout moves — the difference
///   between a digest of the project and a digest of the directory the project happens to sit in.
/// * **The fields are sorted by path**, because [`Digest::of_fields`] is order-sensitive and a walk's
///   order is a property of the filesystem, not of the tree.
/// * **The universe is every covered file in the crate's directory** — its `src/`, its `tests/`, its
///   `build.rs`, its `Cargo.toml` — rather than a subset chosen here. A chosen subset would be a
///   second rule about which bytes the reading is about, and this module already refuses that for the
///   prose denominator. The cost is honest: an edit anywhere under a crate's directory makes its
///   count stale, including an edit to a test, which cannot change the count. The reading *was* taken
///   over a tree that edit changed, and saying so is cheaper than a rule that guesses which edits
///   matter.
fn missing_docs_pin(sources: &CrateSources, root: &Path) -> Identity {
    let mut parts: Vec<Vec<u8>> = Vec::with_capacity(sources.read.len() * 2);
    let mut read: Vec<&(std::path::PathBuf, String)> = sources.read.iter().collect();
    read.sort_by(|(left, _), (right, _)| left.cmp(right));
    for (path, text) in read {
        let relative = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        parts.push(relative.into_bytes());
        parts.push(text.as_bytes().to_vec());
    }
    let fields: Vec<&[u8]> = parts.iter().map(|part| part.as_slice()).collect();
    Identity::from_digest(Digest::of_fields(MISSING_DOCS_DOMAIN, &fields))
}

/// Every Markdown document under `docs/`, as lines, **read once for the whole census** — twelve
/// crates asking the same question of the same corpus is twelve reads of one corpus.
fn read_docs(root: &Path) -> Vec<(String, Vec<String>)> {
    let Ok(entries) = std::fs::read_dir(root.join("docs")) else {
        return Vec::new();
    };
    let mut names: Vec<std::path::PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    names.sort();
    names
        .into_iter()
        .filter(|path| path.extension().and_then(|extension| extension.to_str()) == Some("md"))
        .filter_map(|path| {
            let text = std::fs::read_to_string(&path).ok()?;
            Some((
                path.file_name()?.to_str()?.to_string(),
                text.lines().map(str::to_string).collect(),
            ))
        })
        .collect()
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

/// **The workspace's own manifest, read against this table.**
///
/// A literal count is a number a reader can see, and `CRATE_COUNT` is one for that reason. But a
/// literal that **nothing compares against the tree** is a list that falls behind the workspace
/// silently, and this one did: `voxel-light`, `voxel-audio`, `voxel-mcp` and `voxel-von` were
/// added to `Cargo.toml` and measured by nothing at all — the census, the conformance verdict and
/// the patch coverage all iterate [`CRATES`], so four members were absent from every report and
/// the reports read as complete.
///
/// `check_table_shape` cannot close this: it compares one table against another, and both tables
/// are in this file. The witness has to be the **manifest**, which is the thing that changes when
/// somebody adds a crate — so this reads it, and refuses by name.
///
/// A crate in the manifest and not here is measured by nothing. A crate here and not in the
/// manifest is a report about a tree this is not. Both are refusals, and the two are named
/// separately because they are different mistakes with different fixes.
pub fn check_against_manifest(root: &Path) -> Result<(), RefusalSet> {
    let mut refusals = RefusalSet::new();
    let manifest = root.join("Cargo.toml");
    let text = match std::fs::read_to_string(&manifest) {
        Ok(text) => text,
        Err(error) => {
            refusals.refuse(
                RefusalKind::NoReading,
                format!(
                    "the workspace manifest could not be read ({error}), so the crate table was \
                     compared against nothing. A table that is right about a manifest nobody read \
                     is an assertion"
                ),
            );
            return Err(refusals);
        }
    };
    let declared = workspace_member_packages(root, &text);
    if declared.is_empty() {
        refusals.refuse(
            RefusalKind::NoReading,
            "the workspace manifest was read and its `members` array came back empty. Nothing was \
             compared, and an empty comparison passes every table"
                .to_string(),
        );
        return Err(refusals);
    }

    let measured: Vec<&'static str> = Crate::ALL.iter().map(|crate_| crate_.name()).collect();
    let unmeasured: Vec<&String> = declared
        .iter()
        .filter(|member| !measured.contains(&member.as_str()))
        .collect();
    if !unmeasured.is_empty() {
        refusals.refuse(
            RefusalKind::UncalibratedOracle,
            format!(
                "these workspace members are declared in Cargo.toml and in no census table, so \
                 nothing measures them: {unmeasured:?}. Add a `Crate` variant, a row with its \
                 witness, and its place in `CRATES`"
            ),
        );
    }

    let ghost: Vec<&&'static str> = measured
        .iter()
        .filter(|name| !declared.iter().any(|member| member == *name))
        .collect();
    if !ghost.is_empty() {
        refusals.refuse(
            RefusalKind::FabricatedFeature,
            format!(
                "these census rows name crates the workspace does not declare: {ghost:?}. A table \
                 with rows for absent crates reports on a tree that is not this one"
            ),
        );
    }

    refusals.into_result(())
}

/// **Each crate's declared witness, on disk, with `required` deciding what a missing one costs.**
///
/// The row table has carried a `required` flag since it was written, and **nothing read it**:
/// `row_of` had no caller outside this module, so the flag was prose in a struct field rather than
/// the build decision its own doc comment claims. A crate whose witness file was moved or deleted
/// was caught by a *test* in this file and by nothing in the gate — and the file that *is* the
/// crate, by this table's own definition, is not a thing to notice in a suite and shrug at in a
/// build.
///
/// The division is [`crate::features::check_required`]'s, deliberately reused rather than
/// reinvented, because a second vocabulary for "this absence is fatal and that one is shown" is
/// how two gates come to disagree about one tree:
///
/// * a **required** crate with no witness refuses the build;
/// * an **optional** crate with no witness is returned and printed. Shown, never silent — the
///   failure mode of a table like this is not a wrong verdict, it is a line nobody prints.
///
/// An absent row is treated as required: a crate the table does not describe is not a crate the
/// build may quietly skip, and [`check_table_shape`] refuses that case separately.
pub fn check_witnesses(root: &Path) -> Result<Vec<String>, RefusalSet> {
    let mut refusals = RefusalSet::new();
    let mut shown = Vec::new();
    for crate_ in Crate::ALL {
        let witness = crate_.witness_of();
        if root.join(witness).is_file() {
            continue;
        }
        let required = row_of(crate_).map(|row| row.required).unwrap_or(true);
        if required {
            refusals.refuse(
                RefusalKind::NoReading,
                format!(
                    "{} declares `{witness}` as its witness and that file is not there. Its row \
                     says `required`, so this is a refusal and not a gap: a crate the build cannot \
                     do without is not a crate to report and move past",
                    crate_.name()
                ),
            );
        } else {
            shown.push(format!("{}\t{witness}", crate_.name()));
        }
    }
    refusals.into_result(shown)
}

/// The `members` array of a workspace manifest, as the paths it lists.
///
/// **The tree's own manifest, and not a TOML dependency.** This crate carries no external
/// dependency and its rule is that a resolve is not a use, so the parse is the narrow one the
/// question needs: the `members = [ … ]` array, quoted, one path per line.
///
/// It returns an empty list rather than guessing when the array is absent, because **"no members
/// were read" and "the workspace holds no members" are different facts**, and every caller here
/// refuses on the first.
pub fn workspace_member_paths(manifest: &str) -> Vec<String> {
    let Some(start) = manifest.find("members = [") else {
        return Vec::new();
    };
    let rest = &manifest[start + "members = [".len()..];
    let end = rest.find(']').unwrap_or(rest.len());
    rest[..end]
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let open = line.find('"')?;
            let close = line[open + 1..].find('"')?;
            Some(line[open + 1..open + 1 + close].to_string())
        })
        .collect()
}

/// The same list, as bare crate names.
///
/// The last segment of the path, which this workspace's `crates/<name>` layout makes equal to the
/// package name — and which [`check_against_manifest`] holds against [`Crate::name`], so the
/// convention is checked rather than assumed.
pub fn workspace_members(manifest: &str) -> Vec<String> {
    workspace_member_paths(manifest)
        .iter()
        .filter_map(|path| path.rsplit('/').next())
        .map(str::to_string)
        .collect()
}

/// **The package names of the workspace members**, read from each member's own manifest.
///
/// **Why the directory name is not the witness.** A crate's directory and its package need not
/// agree, and `PATCH-042`'s axis folder makes them disagree on purpose: `crates/axis/being` holds
/// the package `voxel-axis-being`, because the folder groups the layers and the package names the
/// layer. Comparing the census table against directory names would refuse every crate in that
/// folder — ten of them in `PATCH-042` — for a spelling rather than for a missing row.
///
/// So the member's `[package] name = "…"` line is the reading, and the directory name is the
/// **fallback only when the manifest cannot be read**. A fallback is not a silent default: a member
/// whose manifest is unreadable is named as a workspace member the table cannot match, which is the
/// same refusal the caller already reports.
pub fn workspace_member_packages(root: &Path, manifest: &str) -> Vec<String> {
    workspace_member_paths(manifest)
        .iter()
        .map(|path| {
            let manifest = root.join(path).join("Cargo.toml");
            std::fs::read_to_string(manifest)
                .ok()
                .and_then(|text| package_name(&text))
                .unwrap_or_else(|| {
                    path.rsplit('/').next().unwrap_or(path).to_string()
                })
        })
        .collect()
}

/// The `name = "…"` line of a `[package]` section, and `None` if the section has none.
fn package_name(manifest: &str) -> Option<String> {
    let mut in_package = false;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_package = trimmed == "[package]";
            continue;
        }
        if !in_package {
            continue;
        }
        let Some(rest) = trimmed.strip_prefix("name") else {
            continue;
        };
        let Some(rest) = rest.trim_start().strip_prefix('=') else {
            continue;
        };
        let Some(rest) = rest.trim().strip_prefix('"') else {
            continue;
        };
        if let Some(end) = rest.find('"') {
            return Some(rest[..end].to_string());
        }
    }
    None
}

/// Where a crate's prose budget is declared: one `crate<TAB>percent` line per crate.
///
/// **A file of numbers you can argue with**, which is the whole point. The record already ruled that
/// explanation belongs in `docs/` — that is what [`PROSE_FRAME_PERCENT`] states, and what the
/// `documented` axis refuses on. But while the sixteen sections were owed, that axis refused
/// **sixteen of sixteen crates and all sixteen were accepted**: it printed every build and blocked
/// nothing, and a check that cannot fail stops being read, so the code's own 19–50 % comment
/// headers became the example every later file matched. That is how "the house rule is heavy docs
/// in the code" outlived the ruling against it — not by anyone deciding it, but by a rule that
/// fired into a list that said yes. **The user's ruling of 2026-10-10 moved the home and emptied that
/// list**: documentation belongs in each crate's own rustdoc, where `cargo doc` and a hover put the
/// reason for a value next to the value. (The axis itself stopped reading prose on 2026-10-10:
/// [`Axis::Documented`] now reads the compiler's own `missing_docs` count in
/// [`MISSING_DOCS_PATH`], because asking whether a `//!` block was present answered *has a claim*,
/// not *is documented*.) What is left for this ratchet is the share that is **plain comment** and not
/// rustdoc, measured apart since that same ruling, which makes it the only check on prose that is
/// not documentation.
///
/// So the frame is a **ratchet** instead of a threshold: a crate's declared number is what it
/// measured on the day the frame was declared, the share **may fall and may not rise**, and it does
/// not have to reach 10 % today. That changes what an agent is asked to do from *fix 12 000 lines of
/// existing prose* (which nobody will do, and which the axis stopped asking for the day the ruling
/// landed) to **do not make it worse** (which is checkable every build, on every edit, with no
/// retrofit owed).
///
/// The inverse of the acceptance list, and deliberately: an accepted refusal says *this is known and
/// tolerated*. A budget says *this is the most you may have*. Only the second one stops growth.
pub const PROSE_FRAME_PATH: &str = "conformance/prose-frame.txt";

/// Where the compiler's own documentation reading is declared: one `crate<TAB>missing<TAB>pin` line
/// per crate, `missing` being how many public items `missing_docs` reports without a doc comment and
/// `pin` being the digest of the bytes that count was taken over ([`MISSING_DOCS_DOMAIN`]).
///
/// **The `documented` axis's source, and it is a measurement.** [`Axis::Documented`] reads the
/// reading: `0` is `active`, more is `inactive` with the count in the reason, a crate with **no line
/// is refused** — because a `//!` header is a claim that documentation exists while this is a count
/// of the documentation that does — and a line whose **pin no longer matches the files on disk is
/// `stale`**, because the count is a reading of a tree and the tree has moved. The recipe that
/// produces the file is in the head of `conformance/missing-docs.txt`; the compiler step is not
/// re-run per build, because the census runs inside `shploingus`'s build script and invoking the
/// compiler from there would recurse into the build that is running. So unlike [`PROSE_FRAME_PATH`],
/// which the gate re-measures every build, this reading is taken by a command a person runs and read
/// back by the gate — and the pin is what keeps that from meaning *the gate repeats whatever number
/// it was last handed*: the count is only read as this tree's answer while the bytes it was taken
/// over are still the bytes on disk.
pub const MISSING_DOCS_PATH: &str = "conformance/missing-docs.txt";

/// The domain tag folded in ahead of a documentation pin's payload.
///
/// [`Digest::of_fields`] length-prefixes every field under a tag, and the tag is what keeps a pin of
/// a crate's sources from being mistakable for a pin of a chain, a package or the working tree —
/// the same discipline [`provenance::DOMAIN`] records.
pub const MISSING_DOCS_DOMAIN: &str = "shploingus/missing-docs";

/// One crate's standing against its declared prose budget.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProseStanding {
    pub crate_: Crate,
    /// The measured share of the crate's source that is comment, as a whole percent.
    pub measured: usize,
    /// The most it may be. Declared in [`PROSE_FRAME_PATH`].
    pub allowed: usize,
}

impl ProseStanding {
    /// How much prose could be added before the budget is reached. **Printed, never silent**: a
    /// budget with slack in it is a budget that can be lowered, and a reader who cannot see the slack
    /// does not know there is a decision left to make.
    pub const fn slack(&self) -> usize {
        self.allowed.saturating_sub(self.measured)
    }
}

/// The ratchet's whole rule, as one function, so a test pins the boundary instead of re-implementing
/// it — a second copy of a rule is a second rule, which is the failure this module was rewritten for.
///
/// `None` means within budget. `Some(why)` is the refusal.
fn over_budget(crate_: Crate, measured: usize, allowed: usize) -> Option<String> {
    if measured <= allowed {
        return None;
    }
    Some(format!(
        "{} is {measured}% comment and its declared budget is {allowed}%. The share may fall and may \
         not rise: the explanation this crate needs belongs in a document under `docs/`, where a \
         reader looks for it, and prose in a code artifact is documentation written where nobody \
         reads it. Move the explanation rather than arguing the number up — a budget raised because \
         the code grew is not a budget, it is a record of what happened",
        crate_.name()
    ))
}

/// **The ratchet, measured.** Every crate's prose share against its declared budget.
///
/// **Fatal on a rise, and silent on the debt that is already there** — the whole design. A door that
/// demanded 10 % today would refuse every build until a 12 000-line retrofit landed, which is how a
/// gate teaches everyone to route around it; that is what the acceptance list is doing right now for
/// this very axis. A door that says *not worse than yesterday* refuses exactly the edits this
/// project keeps having to apologise for, and asks for nothing else.
///
/// Three ways to fail, and they are different facts:
///
/// * a crate with **no line** in the frame has no budget, and a crate outside the ratchet is free to
///   inflate — so the next crate added would be too, silently;
/// * a crate whose sources **cannot be read** is not within budget: *not measured* is not a pass, and
///   this vocabulary exists precisely so the two never look alike;
/// * a crate **over its number**, which is the refusal the rule is for.
///
/// The frame values come from this function's own measurement — [`check_prose_frame`] prints every
/// crate's share, so the numbers in the file are a reading rather than a guess.
pub fn check_prose_frame(root: &Path) -> Result<Vec<ProseStanding>, RefusalSet> {
    let mut refusals = RefusalSet::new();
    let declared = read_prose_frame(root, &mut refusals);
    if !refusals.is_empty() {
        return Err(refusals);
    }

    let covered = provenance::covered_files(root);
    let mut standings = Vec::with_capacity(CRATE_COUNT);
    for crate_ in Crate::ALL {
        let name = crate_.name();
        let Some(allowed) = declared.get(name).copied() else {
            refusals.refuse(
                RefusalKind::UncalibratedOracle,
                format!(
                    "{name} has no line in {PROSE_FRAME_PATH}, so it has no prose budget, so nothing \
                     bounds it. A crate outside the ratchet is a crate free to inflate — and the next \
                     crate added would be too, silently"
                ),
            );
            continue;
        };
        let measured = match read_sources(&covered, crate_, root) {
            Some(sources) if !sources.read.is_empty() => {
                let prose = bloat::measure_prose(&sources.read);
                // **The plain share, not the whole comment share.** Rustdoc is documentation this
                // project wants next to the values it explains, so a ratchet that counted it would
                // charge every crate for writing the thing it is owed.
                match prose.plain_percent() {
                    Some(percent) => percent,
                    None => {
                        refusals.refuse(
                            RefusalKind::NoReading,
                            format!(
                                "{name}'s prose share was not measured: {}. Not measured is not \
                                 within budget",
                                prose
                                    .why_unmeasured
                                    .clone()
                                    .unwrap_or_else(|| "no reason recorded".to_string())
                            ),
                        );
                        continue;
                    }
                }
            }
            _ => {
                refusals.refuse(
                    RefusalKind::NoReading,
                    format!(
                        "{name} has no readable source, so its prose share is not a measurement. An \
                         unreadable crate is not a crate within budget"
                    ),
                );
                continue;
            }
        };
        if let Some(why) = over_budget(crate_, measured, allowed) {
            refusals.refuse(RefusalKind::UncalibratedOracle, why);
        }
        standings.push(ProseStanding { crate_, measured, allowed });
    }

    refusals.into_result(standings)
}

/// The declared budgets, read from [`PROSE_FRAME_PATH`].
///
/// Refuses on a line that is not `crate<TAB>percent`, on a number that does not parse, and on a name
/// that is not a crate in this workspace — **a budget for a crate that does not exist is a decision
/// about nothing**, and leaving it in the file makes the frame look fuller than it is.
fn read_prose_frame(
    root: &Path,
    refusals: &mut RefusalSet,
) -> std::collections::BTreeMap<String, usize> {
    let mut declared = std::collections::BTreeMap::new();
    let text = match std::fs::read_to_string(root.join(PROSE_FRAME_PATH)) {
        Ok(text) => text,
        Err(error) => {
            refusals.refuse(
                RefusalKind::NoReading,
                format!(
                    "{PROSE_FRAME_PATH} could not be read ({error}), so **no** crate has a prose \
                     budget and the ratchet is silent. That is the exact state this file exists to \
                     end, so an unreadable frame is a refusal and not an empty one"
                ),
            );
            return declared;
        }
    };
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let number = index + 1;
        let Some((name, percent)) = line.split_once('\t') else {
            refusals.refuse(
                RefusalKind::FabricatedFeature,
                format!(
                    "{PROSE_FRAME_PATH}:{number} is not `crate<TAB>percent`: {line:?}. A frame line \
                     nobody can parse is a budget nobody is held to"
                ),
            );
            continue;
        };
        let name = name.trim();
        if Crate::parse(name).is_err() {
            refusals.refuse(
                RefusalKind::FabricatedFeature,
                format!(
                    "{PROSE_FRAME_PATH}:{number} budgets `{name}`, which is not a crate in this \
                     workspace"
                ),
            );
            continue;
        }
        match percent.trim().parse::<usize>() {
            Ok(percent) => {
                declared.insert(name.to_string(), percent);
            }
            Err(_) => {
                refusals.refuse(
                    RefusalKind::FabricatedFeature,
                    format!(
                        "{PROSE_FRAME_PATH}:{number} declares {percent:?} as a percentage for \
                         `{name}`"
                    ),
                );
            }
        }
    }
    declared
}

/// The declared documentation reading, by crate: the compiler's count **and the pin it was taken
/// under** (see [`MISSING_DOCS_PATH`]).
///
/// **Absence is not silence.** A crate with no line comes back with no entry and [`Axis::Documented`]
/// refuses it by name; this reader deliberately does not invent a zero for it, because a zero is the
/// measurement of a crate that *is* documented and the two facts are opposite.
///
/// **A malformed line is worse than an absent one**, because it would credit one crate and refuse
/// another out of a file that is not well formed, so a line that is not `crate<TAB>missing<TAB>pin`,
/// a count that does not parse, a pin that is not `sha256:<64 lowercase hex>`, a name that is not a
/// crate in this workspace, and a crate declared twice are each a refusal of the whole census.
///
/// **The pin is required, and a missing one is a refusal rather than a default.** A count with no
/// pin is a number about an unknown tree: there is nothing to compare it against, so the axis would
/// have no choice but to read it as current — the exact failure this column exists to end. That is
/// [`provenance`]'s rule applied one level down: a digest that cannot be taken is refused loudly
/// rather than replaced by a digest of nothing.
fn read_missing_docs(
    root: &Path,
    refusals: &mut RefusalSet,
) -> std::collections::BTreeMap<String, (usize, Identity)> {
    let mut missing = std::collections::BTreeMap::new();
    let Ok(text) = std::fs::read_to_string(root.join(MISSING_DOCS_PATH)) else {
        return missing;
    };
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let number = index + 1;
        let Some((name, rest)) = line.split_once('\t') else {
            refusals.refuse(
                RefusalKind::FabricatedFeature,
                format!(
                    "{MISSING_DOCS_PATH}:{number} is not `crate<TAB>missing<TAB>pin`: {line:?}. A \
                     reading nobody can parse is a reading that would credit a crate by accident"
                ),
            );
            continue;
        };
        let name = name.trim();
        if Crate::parse(name).is_err() {
            refusals.refuse(
                RefusalKind::FabricatedFeature,
                format!(
                    "{MISSING_DOCS_PATH}:{number} reads `{name}`, which is not a crate in this \
                     workspace"
                ),
            );
            continue;
        }
        let Some((count, pin)) = rest.split_once('\t') else {
            refusals.refuse(
                RefusalKind::FabricatedFeature,
                format!(
                    "{MISSING_DOCS_PATH}:{number} declares a count for `{name}` with no pin. A \
                     reading with no pin cannot be told apart from one taken over a different tree, \
                     so it would be read as this build's answer; the pin comes from the same recipe \
                     that produces the count"
                ),
            );
            continue;
        };
        let Ok(count) = count.trim().parse::<usize>() else {
            refusals.refuse(
                RefusalKind::FabricatedFeature,
                format!("{MISSING_DOCS_PATH}:{number} declares {count:?} as a count for `{name}`"),
            );
            continue;
        };
        let Ok(declared) = Identity::parse(pin.trim()) else {
            refusals.refuse(
                RefusalKind::FabricatedFeature,
                format!(
                    "{MISSING_DOCS_PATH}:{number} pins `{name}` to {:?}, which is not \
                     `sha256:<64 lowercase hex>`: a pin nothing can check pins nothing",
                    pin.trim()
                ),
            );
            continue;
        };
        if missing.insert(name.to_string(), (count, declared)).is_some() {
            refusals.refuse(
                RefusalKind::FabricatedFeature,
                format!(
                    "{MISSING_DOCS_PATH}:{number} reads `{name}` twice, so the file holds two \
                     answers for one crate"
                ),
            );
        }
    }
    missing
}

/// **The dependency discipline's second rule, measured: is it actually USED?**
///
/// Every manifest in this workspace carries the same two-line discipline in a comment — *1. does it
/// do something we need? 2. is it actually USED? (a resolve is not a use)* — and **nothing checked
/// the second rule**. It was not checkable by any axis: the six the census has measure what a crate
/// *is* (named, moduled, tested, refusing, claimed, documented) and none asks who *reads* it, so a
/// crate no one depends on scored exactly like `voxel-model`.
///
/// It found two, and neither was a surprise to the record — both were built *ahead* of the world
/// that would use them (C-25's early slots) with the debt stated in prose and measured nowhere:
///
/// * `voxel-audio` — the rain's synthesis, whose own manifest answers rule 2 with *"yes —
///   voxel-client plays what this returns"*. **No crate in the workspace depends on it and no code
///   calls it**, so that sentence is false, and it was false when it was written;
/// * `voxel-instrument` — the sensor concept, the band controller and the gauge wall (S-03, S-12,
///   S-13), 1,417 lines whose only other mentions in the tree are comments citing
///   `voxel_instrument::Sensor` as "the one sensor concept".
///
/// The rule, written down so it can be argued with rather than inferred from this function:
///
/// * a member **another member depends on** is used. Dev-dependencies count — a library read by
///   tests is read by something in this tree — and *any* `path = "…"` edge counts, wherever it
///   appears in a member's manifest;
/// * a member with a **binary target** is an entry point. `shploingus` opens the window and
///   `voxel-mcp` is a server; neither needs a consumer to be legitimate, and calling them orphans
///   would make this list noise that a reader learns to skip;
/// * every other member is an orphan, and **an orphan is a resolve without a use**.
///
/// **Reported, never refused.** An orphan is a fact about this tree and the decision is a reader's:
/// wire it, delete it, or say on the record why a library nothing calls is deliberate. A door here
/// would refuse every build until a content crate is scheduled, which is how a gate teaches
/// everyone to route around it.
pub fn orphan_libraries(root: &Path) -> Result<Vec<String>, RefusalSet> {
    let mut refusals = RefusalSet::new();
    let manifest = root.join("Cargo.toml");
    let text = match std::fs::read_to_string(&manifest) {
        Ok(text) => text,
        Err(error) => {
            refusals.refuse(
                RefusalKind::NoReading,
                format!(
                    "the workspace manifest could not be read ({error}), so nothing could be asked \
                     which crate reads which. An empty orphan list would read as a clean tree"
                ),
            );
            return Err(refusals);
        }
    };
    let paths = workspace_member_paths(&text);
    // **Package names, not directory names.** A dependency line names a package
    // (`voxel-axis-being`), so the witness that decides whether it was read must be the package's
    // own name too — otherwise a crate in a grouped folder looks orphaned while everything that
    // depends on it was counted against nothing.
    let names = workspace_member_packages(root, &text);
    if names.is_empty() {
        refusals.refuse(
            RefusalKind::NoReading,
            "the workspace manifest names no members, so nothing was compared".to_string(),
        );
        return Err(refusals);
    }

    // **A dependency line names a directory, and a directory is not a package.**
    //
    // `dependent_members` reads the last segment of every `path = "…"`, which is the *folder*: a
    // crate in a grouped folder is depended on as `path = "../axis/matrix"` and the segment is
    // `matrix`, while its package is `voxel-axis-matrix`. Comparing those two strings reported a
    // wired-up crate as an orphan, which is the exact defect the comment on `names` above says the
    // package names were introduced to prevent — fixed on the names side and left standing on the
    // dependency side. The map is the join, and it is built from the manifest's own pairs.
    let directory_to_package: Vec<(&str, &String)> = paths
        .iter()
        .zip(names.iter())
        .filter_map(|(path, name)| {
            path.rsplit('/').next().map(|directory| (directory, name))
        })
        .collect();

    let mut depended: Vec<String> = Vec::new();
    let mut unreadable: Vec<String> = Vec::new();
    for (index, path) in paths.iter().enumerate() {
        let own = root.join(path).join("Cargo.toml");
        match std::fs::read_to_string(&own) {
            Ok(own) => depended.extend(
                dependent_members(&own)
                    .into_iter()
                    .filter_map(|directory| {
                        directory_to_package
                            .iter()
                            .find(|(folder, _)| *folder == directory)
                            .map(|(_, name)| (*name).clone())
                    })
                    .filter(|name| names.contains(name)),
            ),
            // **A member whose manifest cannot be read is not an orphan**, it is unmeasured, and
            // the two are different facts: one says nobody reads this crate, the other says nobody
            // looked. The second is a refusal and never a line in the list.
            Err(_) => unreadable.push(names[index].clone()),
        }
    }
    if !unreadable.is_empty() {
        refusals.refuse(
            RefusalKind::NoReading,
            format!(
                "these members' manifests could not be read, so whether anything depends on them is \
                 unknown: {unreadable:?}"
            ),
        );
    }

    let mut orphans: Vec<String> = names
        .iter()
        .enumerate()
        .filter(|(_, name)| !depended.contains(name))
        .filter(|(_, name)| !unreadable.contains(name))
        .filter(|(index, _)| !has_binary_target(root, &paths[*index]))
        .map(|(_, name)| name.clone())
        .collect();
    orphans.sort();

    refusals.into_result(orphans)
}

/// Every member this manifest points at with a `path = "…"` edge, as bare names.
///
/// **Section-blind on purpose.** A dependency, a dev-dependency and a build-dependency are three
/// different claims about *when* a crate is read, and the question here is only whether it is read
/// at all — so splitting them would be a more precise answer to a question nobody asked, and the
/// one that matters (`is this library dead?`) would still be `no` in all three cases.
///
/// **It is also comment-blind, and that is a limitation rather than a design.** A manifest that
/// quoted `path = "../somewhere"` inside a comment would be read as depending on `somewhere`, which
/// would hide an orphan rather than invent one. No manifest in this workspace does (checked), and a
/// comment-blind scan is still the right shape here: the alternative is a TOML dependency for one
/// line, and a false *negative* on this question is a missing report, not a wrong one.
fn dependent_members(manifest: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = manifest;
    while let Some(at) = rest.find("path = \"") {
        rest = &rest[at + "path = \"".len()..];
        let Some(end) = rest.find('"') else { break };
        if let Some(name) = rest[..end].rsplit('/').next() {
            if !name.is_empty() {
                found.push(name.to_string());
            }
        }
        rest = &rest[end..];
    }
    found
}

/// Whether this member is an entry point: a `src/main.rs`, or a declared `[[bin]]`.
///
/// Both, because a manifest can point a binary target at any file, and a check that only looked at
/// the conventional path would call `voxel-mcp` an orphan the day someone moved its `main`.
fn has_binary_target(root: &Path, member_path: &str) -> bool {
    let directory = root.join(member_path);
    if directory.join("src/main.rs").is_file() {
        return true;
    }
    std::fs::read_to_string(directory.join("Cargo.toml"))
        .map(|manifest| manifest.contains("[[bin]]"))
        .unwrap_or(false)
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
    fn a_missing_witness_is_fatal_when_the_row_says_required_and_shown_when_it_does_not() {
        // **The flag's reader, checked.** `CrateRow::required` was written as a build decision and
        // consulted by nothing; the only thing that noticed a missing witness was a test in this
        // file. So this exercises the division itself on a scratch tree rather than trusting the
        // real tree's sixteen present files to say the wiring works.
        let scratch = std::env::temp_dir().join(format!("census-witness-{}", std::process::id()));
        let write = |crates: &[Crate]| {
            let _ = std::fs::remove_dir_all(&scratch);
            for crate_ in crates {
                let path = scratch.join(crate_.witness_of());
                std::fs::create_dir_all(path.parent().expect("a crate directory"))
                    .expect("a crate directory");
                std::fs::write(&path, "// a witness\n").expect("a witness file");
            }
        };
        let is_required = |crate_: Crate| row_of(crate_).expect("every crate has a row").required;

        // The required half complete and the optional half absent: the call returns, and names
        // **exactly** the optional absences. A shown list that dropped one would be the same
        // silence as no list at all.
        let required: Vec<Crate> = Crate::ALL.into_iter().filter(|c| is_required(*c)).collect();
        write(&required);
        let shown = check_witnesses(&scratch).expect("no required witness is missing");
        let optional: Vec<&'static str> = Crate::ALL
            .into_iter()
            .filter(|c| !is_required(*c))
            .map(|c| c.name())
            .collect();
        assert_eq!(
            shown.len(),
            optional.len(),
            "every optional absence is shown: {shown:?} against {optional:?}"
        );
        for name in &optional {
            assert!(
                shown.iter().any(|line| line.starts_with(&format!("{name}\t"))),
                "{name} is absent and not shown: {shown:?}"
            );
        }

        // And which half is which is not a detail: remove one **required** witness and the same
        // call refuses, naming the crate.
        let mut with_a_hole = required.clone();
        with_a_hole.retain(|crate_| *crate_ != Crate::Package);
        write(&with_a_hole);
        let found = check_witnesses(&scratch).expect_err("a required witness is missing");
        let joined = found
            .all()
            .iter()
            .map(|refusal| refusal.detail().to_string())
            .collect::<Vec<_>>()
            .join("; ");
        assert!(
            joined.contains("voxel-package"),
            "the absent required witness is named: {joined}"
        );

        let _ = std::fs::remove_dir_all(&scratch);
    }

    #[test]
    fn the_dependency_disciplines_second_rule_is_measured_and_finds_the_crates_nothing_reads() {
        // **Rule 2 as a measurement.** Every manifest in this workspace answers *"is it actually
        // USED? (a resolve is not a use)"* in a comment, and nothing checked the answer — not the
        // six axes, which measure what a crate is and never who reads it. The orphans this finds
        // are the finding, so they are named here: pinning one means wiring it up, deleting it, or
        // ruling it deliberate, and each is visible as a change to this line. **`voxel-axis-mcp`
        // is deliberately absent**: `voxel-client` depends on it and calls it from the mount path,
        // so the authority an agent speaks through is read by the surface that serves the player.
        // **`voxel-axis-civilians` is absent for the same reason since the contract pass**:
        // `voxel-package::decision` now consumes its `LadderReading`, so the ladder reading has
        // its caller even though the `Arrangement` half still waits for the `Mount` adapter.
        let orphans = orphan_libraries(&real_root()).expect("the manifests are readable");
        println!("\norphan libraries: {orphans:?}\n");
        assert_eq!(
            orphans,
            // Sorted as `orphan_libraries` sorts them.
            vec![
                // The two long-standing orphans: a synth and a meter with no caller.
                "voxel-audio".to_string(),
                // **The balance census, and it is read by a person and by the shell's console.**
                // `PATCH-045` makes it a report crate: its answer is a reading, not a service, so
                // no crate imports it and its absence from every manifest is the design rather
                // than an unwired edge. It leaves this list the day a surface consumes its lines.
                "voxel-axis-balance".to_string(),
                // **The being substrate, and its absence has a reason and a date.** `PATCH-043`
                // creates it first and Gaia second, so between those two commits nothing reads it
                // — a library no code reads *yet* rather than one nothing will. It leaves this
                // list the commit `voxel-gaia` depends on it, which is the next slice.
                "voxel-axis-being".to_string(),
                "voxel-instrument".to_string(),
            ],
            "the set of libraries nothing in the workspace depends on changed. If a crate was \
             wired up or removed, update this list with the reason; if one appeared, it is a \
             library no code reads"
        );
    }

    #[test]
    fn an_orphan_is_a_library_an_entry_point_is_not_and_an_unreadable_manifest_is_neither() {
        // The three distinctions the rule rests on, on a scratch workspace — because the real tree
        // can only show the case it happens to be in, and a rule checked only against its current
        // answer is a rule that stops being checked the moment it is wrong.
        let scratch = std::env::temp_dir().join(format!("census-orphans-{}", std::process::id()));
        let write = |relative: &str, text: &str| {
            let path = scratch.join(relative);
            std::fs::create_dir_all(path.parent().expect("a directory")).expect("a directory");
            std::fs::write(&path, text).expect("a file");
        };
        let clean = || {
            let _ = std::fs::remove_dir_all(&scratch);
        };

        // `used` is read by `reader`; `dead` is read by nothing; `tool` is an entry point whose
        // manifest does not exist to be read, which is *not* the same as being an orphan.
        clean();
        write(
            "Cargo.toml",
            "[workspace]\nmembers = [\n    \"crates/reader\",\n    \"crates/used\",\n    \"crates/dead\",\n    \"crates/tool\",\n]\n",
        );
        write("crates/reader/Cargo.toml", "[dependencies]\nused = { path = \"../used\" }\n");
        write("crates/used/Cargo.toml", "[package]\nname = \"used\"\n");
        write("crates/dead/Cargo.toml", "[package]\nname = \"dead\"\n");
        write("crates/tool/Cargo.toml", "[package]\nname = \"tool\"\n");
        write("crates/tool/src/main.rs", "fn main() {}\n");
        // `reader` is itself read by nobody and is not an entry point, so it is an orphan too —
        // the rule is about the graph, not about importance.
        let found = orphan_libraries(&scratch).expect("the scratch manifests are readable");
        assert_eq!(found, vec!["dead".to_string(), "reader".to_string()], "{found:?}");

        // A member manifest that cannot be read is unmeasured, which is a refusal — never a name
        // in the list, because "nothing reads this" and "nobody looked" are different facts.
        std::fs::remove_file(scratch.join("crates/used/Cargo.toml")).expect("removing a manifest");
        let found = orphan_libraries(&scratch).expect_err("an unreadable member manifest refuses");
        let joined = found
            .all()
            .iter()
            .map(|refusal| refusal.detail().to_string())
            .collect::<Vec<_>>()
            .join("; ");
        assert!(
            joined.contains("used"),
            "the member whose manifest could not be read is named: {joined}"
        );

        // And a workspace manifest that cannot be read is a refusal rather than an empty list,
        // because an empty orphan list is exactly what a clean tree looks like.
        let found = orphan_libraries(&scratch.join("no-such-tree"))
            .expect_err("an unreadable workspace is refused");
        assert!(found
            .all()
            .iter()
            .any(|refusal| refusal.detail().contains("which crate reads which")));

        clean();
    }

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
    fn the_prose_frame_prints_every_crates_share_so_the_numbers_are_a_reading() {
        // **Where the budgets come from.** The frame in `conformance/prose-frame.txt` is a set of
        // declared numbers, and a declared number nobody measured is a guess wearing a ruling's
        // clothes. So this prints the measurement every run: run it with `--nocapture` and the file
        // is authored from the output. C-45's rule — a number that only exists when something breaks
        // is a number nobody checks.
        let root = real_root();
        let covered = crate::provenance::covered_files(&root);
        println!("--- crate\tplain%(the frame)\trustdoc%\tall-comment% ---");
        for crate_ in Crate::ALL {
            let measured = read_sources(&covered, crate_, &root)
                .filter(|sources| !sources.read.is_empty())
                .map(|sources| bloat::measure_prose(&sources.read));
            match measured {
                Some(prose) => println!(
                    "{}\t{}\t{}\t{}",
                    crate_.name(),
                    prose.plain_percent()
                        .map_or("not-measured".to_string(), |share| share.to_string()),
                    prose.doc_percent()
                        .map_or("not-measured".to_string(), |share| share.to_string()),
                    prose.comment_percent()
                        .map_or("not-measured".to_string(), |share| share.to_string()),
                ),
                None => println!("{}\tnot-measured", crate_.name()),
            }
        }
    }

    #[test]
    fn the_missing_docs_pins_print_so_the_reading_can_be_re_pinned() {
        // **Where the third column of `conformance/missing-docs.txt` comes from.** The compiler's count
        // cannot be taken from here — that is the recipe in that file's header, and it invokes rustc —
        // but the pin can, because it is a hash of the bytes on disk and needs no compiler at all. Run
        // this with `--nocapture` and the file is authored from the output: the count from the compiler
        // run, the pin from here, both taken after the last edit to the crate. A pin nobody can
        // reproduce by hand is a claim wearing a digest.
        let root = real_root();
        let covered = crate::provenance::covered_files(&root);
        println!("--- crate\tsha256: (the pin for conformance/missing-docs.txt) ---");
        for crate_ in Crate::ALL {
            match read_sources(&covered, crate_, &root) {
                Some(sources) => {
                    println!("{}\t{}", crate_.name(), missing_docs_pin(&sources, &root))
                }
                None => println!("{}\tnot-measured", crate_.name()),
            }
        }
    }

    #[test]
    fn a_documentation_pin_moves_when_the_bytes_move_and_not_when_the_checkout_does() {
        // **The two properties the pin exists for, on a scratch tree.** Determinism: one tree hashes
        // the same way twice. Sensitivity: a changed byte changes the digest, so an edit under the
        // crate cannot leave a reading looking current. And the path inside it is **relative**, so the
        // pin is a property of the project rather than of the directory the project happens to sit in
        // — the same rule `provenance::measure` records for the tree digest.
        let scratch = std::env::temp_dir().join(format!("census-pin-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&scratch);
        std::fs::create_dir_all(scratch.join("crates/voxel-noise/src")).expect("a crate directory");
        std::fs::write(scratch.join("crates/voxel-noise/src/lib.rs"), "pub fn a() {}\n")
            .expect("a source file");

        let pin = |root: &Path| {
            let covered = crate::provenance::covered_files(root);
            let sources = read_sources(&covered, Crate::Noise, root).expect("covered files");
            missing_docs_pin(&sources, root)
        };
        let first = pin(&scratch);
        assert_eq!(first, pin(&scratch), "two reads of one tree must agree");

        std::fs::write(scratch.join("crates/voxel-noise/src/lib.rs"), "pub fn ab() {}\n")
            .expect("an edit");
        assert_ne!(
            first,
            pin(&scratch),
            "an edit under the crate must move the pin, or a stale reading reads as current"
        );

        // A tree that *moves* keeps its pins: the same bytes at another path hash the same, because
        // the path folded in is relative to the root.
        let elsewhere = std::env::temp_dir().join(format!("census-pin-moved-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&elsewhere);
        std::fs::create_dir_all(elsewhere.join("crates/voxel-noise/src")).expect("a crate directory");
        std::fs::copy(
            scratch.join("crates/voxel-noise/src/lib.rs"),
            elsewhere.join("crates/voxel-noise/src/lib.rs"),
        )
        .expect("the same bytes elsewhere");
        assert_eq!(
            pin(&scratch),
            pin(&elsewhere),
            "the pin is a property of the project, not of the directory it sits in"
        );

        let _ = std::fs::remove_dir_all(&scratch);
        let _ = std::fs::remove_dir_all(&elsewhere);
    }

    #[test]
    fn a_reading_line_is_refused_for_every_way_it_can_be_wrong() {
        // **The reader's whole contract, one case per way it can lie.** A malformed line is worse than
        // an absent one — it credits one crate while the file is not well formed — and a line with no
        // pin at all is the specific lie this column exists to prevent: a count that would be read as
        // this build's answer because there is nothing to compare it against.
        let scratch = std::env::temp_dir().join(format!("census-missing-docs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&scratch);
        std::fs::create_dir_all(scratch.join("conformance")).expect("a scratch conformance dir");
        let pin = Identity::of(b"a pin a test can type").to_string();
        let verdict = |text: &str| -> (std::collections::BTreeMap<String, (usize, Identity)>, String) {
            std::fs::write(scratch.join(MISSING_DOCS_PATH), text).expect("the reading file");
            let mut refusals = RefusalSet::new();
            let read = read_missing_docs(&scratch, &mut refusals);
            let joined = refusals
                .all()
                .iter()
                .map(|refusal| refusal.detail().to_string())
                .collect::<Vec<_>>()
                .join("; ");
            (read, joined)
        };

        // A well-formed line reads, pin and all, and comments and blanks are ignored.
        let (read, refused) =
            verdict(&format!("# a note\n\nvoxel-noise\t15\t{pin}\n"));
        assert!(refused.is_empty(), "a well-formed reading refuses nothing: {refused}");
        assert_eq!(read.get("voxel-noise"), Some(&(15, Identity::parse(&pin).unwrap())));

        let (read, refused) = verdict("voxel-noise\t15\n");
        assert!(read.is_empty(), "a line with no pin is not a reading: {read:?}");
        assert!(refused.contains("with no pin"), "{refused}");

        let (_, refused) = verdict("voxel-noise\t15\tabc\n");
        assert!(refused.contains("pins `voxel-noise` to"), "{refused}");

        // A pin in the wrong case is a different string, and the identity parser refuses it — the
        // rule the tree states once and applies everywhere (`Identity::parse`).
        let (_, refused) = verdict(&format!("voxel-noise\t15\t{}\n", pin.to_uppercase()));
        assert!(refused.contains("pins `voxel-noise` to"), "{refused}");

        let (_, refused) = verdict(&format!("voxel-noise\t15\t{pin}\textra\n"));
        assert!(refused.contains("pins `voxel-noise` to"), "{refused}");

        let (_, refused) = verdict(&format!("voxel-noise\tfifteen\t{pin}\n"));
        assert!(refused.contains("as a count"), "{refused}");

        let (_, refused) = verdict(&format!("voxel-fabricated\t15\t{pin}\n"));
        assert!(refused.contains("not a crate in this workspace"), "{refused}");

        let (_, refused) = verdict(&format!("voxel-noise\t15\t{pin}\nvoxel-noise\t3\t{pin}\n"));
        assert!(refused.contains("twice"), "{refused}");

        let (_, refused) = verdict(&format!("voxel-noise 15 {pin}\n"));
        assert!(refused.contains("is not `crate<TAB>missing<TAB>pin`"), "{refused}");

        // **An absent file refuses nothing here, deliberately**: [`Axis::Documented`] refuses each
        // crate by name for having no line, which is a refusal a reader can act on, where one "the file
        // is missing" would hide which crates are unmeasured behind which one is.
        std::fs::remove_file(scratch.join(MISSING_DOCS_PATH)).expect("removing the reading");
        let mut refusals = RefusalSet::new();
        let read = read_missing_docs(&scratch, &mut refusals);
        assert!(read.is_empty() && refusals.is_empty(), "absence is not silence here");

        let _ = std::fs::remove_dir_all(&scratch);
    }

    #[test]
    fn the_real_tree_is_within_every_declared_prose_budget() {
        // **The ratchet's state on the real tree, as a test and not only as a build-script
        // behaviour** — for the reason [`crate::conformance`]'s real-tree test exists: building
        // `shploingus` needs the GPU stack, so a crate that went over its declared number would
        // refuse a build that nobody running this crate's tests ever starts, and the only place the
        // refusal would be visible is on somebody else's machine. The declaration to change is
        // [`PROSE_FRAME_PATH`]; the refusal names the crate and both numbers.
        //
        // It also pins the count: sixteen members, sixteen standings. A crate outside the ratchet is
        // a crate free to inflate, which is the state `check_prose_frame` refuses rather than passes.
        let standings = check_prose_frame(&real_root())
            .unwrap_or_else(|found| panic!("a crate is over its declared budget: {}", RefusalSet::to_string(&found)));
        assert_eq!(standings.len(), CRATE_COUNT, "a crate with no budget is a crate with no ceiling");
    }

    #[test]
    fn the_prose_ratchet_refuses_a_rise_and_allows_a_fall() {
        // The boundary, pinned on the rule's own function rather than a copy of it. A budget is
        // allowed to be met exactly, may be under-run, and may not be exceeded — and the last one is
        // the only refusal, which is what makes this affordable to keep switched on.
        assert!(over_budget(Crate::Client, 10, 10).is_none(), "meeting the budget exactly is within it");
        assert!(over_budget(Crate::Client, 4, 10).is_none(), "under the budget is within it");
        let refused = over_budget(Crate::Client, 11, 10).expect("one percent over is over");
        assert!(
            refused.contains("11%") && refused.contains("10%") && refused.contains("voxel-client"),
            "the refusal carries both numbers and the crate, or nobody can act on it: {refused}"
        );
        assert!(
            refused.contains("may fall and may not rise"),
            "and it states the rule, because the reader's first instinct will be to raise the \
             number: {refused}"
        );
    }

    #[test]
    fn a_frame_line_that_is_not_a_budget_is_refused_by_name() {
        // Three ways a frame line can be wrong, each with its own refusal: a line that is not
        // `crate<TAB>percent`, a budget for a crate that does not exist (a decision about nothing),
        // and a missing file (which would make the ratchet silent, the state this file ends).
        let scratch = std::env::temp_dir().join(format!("census-prose-frame-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&scratch);
        std::fs::create_dir_all(scratch.join("conformance")).expect("a scratch conformance dir");
        let write = |text: &str| {
            std::fs::write(scratch.join(PROSE_FRAME_PATH), text).expect("the frame file");
        };
        let verdict = |text: &str| -> String {
            write(text);
            let mut refusals = RefusalSet::new();
            let declared = read_prose_frame(&scratch, &mut refusals);
            let _ = declared;
            refusals
                .all()
                .iter()
                .map(|refusal| refusal.detail().to_string())
                .collect::<Vec<_>>()
                .join("; ")
        };

        let joined = verdict("voxel-client no-percent-sign\n");
        assert!(joined.contains("is not `crate<TAB>percent`"), "{joined}");

        let joined = verdict("voxel-fabricated\t10\n");
        assert!(
            joined.contains("not a crate in this workspace"),
            "a budget for a crate that does not exist: {joined}"
        );

        let joined = verdict("voxel-client\tten\n");
        assert!(joined.contains("as a percentage"), "{joined}");

        // A well-formed line parses, comments and blanks are ignored, and the file's absence is its
        // own refusal rather than an empty frame.
        let mut refusals = RefusalSet::new();
        write("# a note\n\nvoxel-client\t26\n");
        let declared = read_prose_frame(&scratch, &mut refusals);
        assert!(refusals.is_empty(), "a well-formed frame refuses nothing");
        assert_eq!(declared.get("voxel-client"), Some(&26));

        let _ = std::fs::remove_file(scratch.join(PROSE_FRAME_PATH));
        let mut refusals = RefusalSet::new();
        let declared = read_prose_frame(&scratch, &mut refusals);
        assert!(declared.is_empty());
        assert!(
            refusals
                .all()
                .iter()
                .any(|refusal| refusal.detail().contains("the ratchet is silent")),
            "an absent frame refuses rather than accepting nothing quietly"
        );

        let _ = std::fs::remove_dir_all(&scratch);
    }

    #[test]
    fn every_workspace_member_is_in_the_table() {
        // **The blind spot, closed by reading the manifest instead of trusting a literal.**
        // `voxel-light`, `voxel-audio`, `voxel-mcp` and `voxel-von` were in `Cargo.toml` and in
        // no census table, so every report that iterates `CRATES` — the census, the conformance
        // verdict, the patch coverage — measured twelve of sixteen members and printed a complete
        // looking table. Nothing compared the count against the tree, and that is the whole
        // failure: a table is only a census if something checks its edges.
        //
        // So: the manifest is the witness, `check_against_manifest` is the comparison, and this
        // test runs it on the real tree. The gate runs the same function, which is the point — a
        // check that only exists in the suite is a check the build does not make.
        let root = real_root();
        match check_against_manifest(&root) {
            Ok(()) => {}
            Err(found) => panic!(
                "the crate table disagrees with the workspace manifest: {}",
                found
                    .all()
                    .iter()
                    .map(|refusal| refusal.detail().to_string())
                    .collect::<Vec<_>>()
                    .join("; ")
            ),
        }

        let manifest = std::fs::read_to_string(root.join("Cargo.toml")).expect("the manifest");
        let declared = workspace_members(&manifest);
        assert_eq!(
            declared.len(),
            CRATE_COUNT,
            "the manifest declares {} members and this table is {CRATE_COUNT} rows; one of the \
             two moved",
            declared.len()
        );
    }

    #[test]
    fn the_manifest_check_can_fail_and_names_what_it_found() {
        // **A check that cannot fail is not a check.** This tree learned that the expensive way:
        // the claim axis once searched whole documents for crate names and reported twelve of
        // twelve claimed, including the binary at the spec's own title line. So the refusal is
        // exercised here on a manifest that really is short of a row, and on one that really does
        // name a crate the workspace does not hold.
        //
        // The manifests are written into a temporary directory **with a real `Cargo.toml` in
        // it**, because the refusal this asserts is about reading a manifest, and a check fed a
        // string is not the check the build runs.
        let scratch = std::env::temp_dir().join(format!("census-manifest-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&scratch);
        std::fs::create_dir_all(&scratch).expect("a scratch directory");

        // One member the table does not know, and the table's own members left out on purpose:
        // both refusals at once, so neither can hide behind the other.
        std::fs::write(
            scratch.join("Cargo.toml"),
            "[workspace]\nmembers = [\n    \"crates/voxel-model\",\n    \"crates/voxel-fabricated\",\n]\n",
        )
        .expect("a manifest");
        let found = check_against_manifest(&scratch).expect_err("a short table must be refused");
        let all: Vec<String> = found
            .all()
            .iter()
            .map(|refusal| refusal.detail().to_string())
            .collect();
        let joined = all.join("; ");
        assert!(
            joined.contains("voxel-fabricated"),
            "the unmeasured member is named: {joined}"
        );
        assert!(
            joined.contains("voxel-shapers") && joined.contains("does not declare"),
            "and the rows that name crates the workspace does not hold are named too: {joined}"
        );

        // A manifest that cannot be read at all is a third fact, not a pass.
        let found = check_against_manifest(&scratch.join("no-such-tree"))
            .expect_err("an unreadable manifest is refused");
        assert!(
            found
                .all()
                .iter()
                .any(|refusal| refusal.detail().contains("compared against nothing")),
            "an unreadable manifest refuses rather than passing quietly"
        );

        let _ = std::fs::remove_dir_all(&scratch);
    }

    #[test]
    fn a_crate_name_round_trips_and_an_unknown_one_is_refused() {
        for crate_ in Crate::ALL {
            assert_eq!(Crate::parse(crate_.name()), Ok(crate_));
        }
        let error = Crate::parse("voxel-envelope").expect_err("that crate does not exist yet");
        // **Pinned against the count rather than a typed number.** This read "contains 12" until
        // the workspace grew to sixteen and the message moved with it; a test that has to be
        // edited because a number it does not own changed is a test that will one day be edited
        // to the wrong number.
        assert!(
            error.contains(&CRATE_COUNT.to_string()),
            "the refusal should say how many crates there are: {error}"
        );
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
        // **The finding this module exists to produce.** `shploingus` carried no
        // `#[test]`, which the gate could not see before because nothing looked
        // at it. Measured here, not hard-coded as a claim.
        //
        // **That finding is closed (C-118).** `shploingus` gained a test — the
        // double-click classifier's — and the census reads **no** untested crate,
        // so the assertion below says `[]`. Should a crate lose its tests, this
        // fails again and that is the finding, not a value to relax.
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
        assert!(
            untested.is_empty(),
            "the set of crates with no test changed. That is a finding to record in the ledger, \
             not a failing expectation to relax: {untested:?}"
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
    fn a_heading_defines_only_what_it_begins_with() {
        assert!(heading_defines("## voxel-noise", "voxel-noise"));
        assert!(heading_defines("# voxel-noise — the noise", "voxel-noise"));
        assert!(heading_defines("### voxel-noise\n", "voxel-noise"));
        // **A prefix of a longer name is a different crate**, and counting it would let one
        // crate's section document another.
        assert!(!heading_defines("## voxel-mesher", "voxel-mesh"));
        // **A heading that cites the name in its title is a reference, not a definition** —
        // the 82 headings C-61 excluded from the record for exactly this reason.
        assert!(!heading_defines("## What voxel-noise does", "voxel-noise"));
        // And prose is not a heading at all.
        assert!(!heading_defines("voxel-noise is described below.", "voxel-noise"));
    }

    #[test]
    fn the_axes_are_seven_and_each_names_itself() {
        assert_eq!(AXES.len(), AXIS_COUNT);
        assert_eq!(AXIS_COUNT, 7, "the axis count is a ruling, not an accident");
        for axis in AXES {
            assert!(!axis.name().is_empty());
        }
    }

    #[test]
    fn a_reexport_and_an_inline_test_are_not_callers() {
        // The two limits the axis states, pinned where they can fail: a re-export names a crate
        // without calling it, and a test that names a crate is testing the caller, not being one.
        assert!(!calls_crate("pub use voxel_audio::Scene;", "voxel_audio"));
        let production = production_lines(
            "use voxel_audio::Scene;\n\
             #[cfg(test)]\n\
             mod tests {\n\
                 use voxel_audio::Scene;\n\
             }\n",
        );
        assert_eq!(production.len(), 1, "only the production import survives: {production:?}");
        assert!(calls_crate(&production[0].1, "voxel_audio"));
    }

    #[test]
    fn the_exercised_axis_measures_a_real_crate_and_names_the_crates_nothing_calls() {
        let root = real_root();
        let covered = provenance::covered_files(&root);
        let callers = exercised_callers(&root, &covered);
        assert!(
            !callers.get(Crate::Model.name()).expect("a key for every crate").is_empty(),
            "voxel-model is reached by other crates; a zero here would mean the scan stopped"
        );
        assert!(
            callers.get(Crate::Audio.name()).expect("a key for every crate").is_empty(),
            "voxel-audio has no call site outside its own sources and tests: {:?}",
            callers.get(Crate::Audio.name())
        );
        let census = census(&root).expect("the real tree is measurable");
        let audio = census.reading_of(Crate::Audio).expect("a reading");
        assert_eq!(audio.cell(Axis::Exercised), Establishment::Inactive);
        assert!(audio.why_at(Axis::Exercised).is_some());
    }
}
