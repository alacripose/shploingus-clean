//! **The content-intake gate — a declared row, consumed in place by reference, judged on named axes.**
//!
//! Derived from `docs/DESIGN-SPEC.md` §2 **F2** (*"source admission and material registry"*, and the
//! one rule inherited whole: **refuse, never default**), from the **U6 — The record and the API layer
//! (the gate)** entry of `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §2 (`admit`/`inspect` returning
//! `Established` — *never a bool* — and the crate-by-axis census with **`Unchecked` never a pass**),
//! from **§5 row 5**'s acceptance bar (*"an unadmitted package refused by name;
//! `Established::ChecksumOnly.establishes_identity() == false`"*), from **§16 row 10**'s row form (*the
//! feature, the source document as `path + digest`, the licence verdict it relies on, the system that
//! carries it (U3–U8), its own acceptance check, and a refusal for a row whose source is refused or
//! whose verdict is absent*), and from `docs/LEDGER.md` **§55.3**, the record's own decision on intake:
//! **integrity and identity are two different statements**, `ChecksumOnly` admits a payload *and
//! reports that no identity was proven*, pinning is the most this repository can honestly offer,
//! `admit` collects **every** problem rather than the first, and a refused package is **not partially
//! admitted**. No `.rs` under `crates/voxel-package/` was read (P7).
//!
//! # What this slice is
//!
//! One capability: **a declared row is admitted, or refused by name, on the record of what was and was
//! not established.** The row *points at* its document — a path and the digest that document must hash
//! to — and the gate reads it **where it lives**. Nothing carries bytes: a row is a declaration, and a
//! declaration that could carry a body would be a second copy of the thing it declares.
//!
//! | the row declares | the gate decides | named refusal when it does not pass |
//! |---|---|---|
//! | the feature it carries | the row names what it is about | `axis_unchecked` |
//! | its source (`path + digest`) | the bytes at that path hash to that digest | `source_absent`, `source_digest_mismatch` |
//! | the licence verdict it relies on | the verdict it relies on permits the source | `axis_unchecked`, `licence_refused` |
//! | the system that carries it | the name is one of U3–U8 | `axis_unchecked`, `carrier_undeclared` |
//! | its own acceptance check | the check exists at its declared reference | `axis_unchecked`, `axis_inactive`, `source_absent`, `source_digest_mismatch` |
//! | the claim it makes | what that claim establishes (a verdict, not a `bool`) | `axis_unchecked`, `pin_mismatch` |
//! | what it requires of the world (`requires_stages`, `requires_materials`) | the world's **own** declarations hold it: U3's roster, U2's registry | `axis_unchecked`, `axis_inactive` (owed), `stage_undeclared`, `unadmitted_material` |
//!
//! # The one rule, and the one state that does not refuse
//!
//! **Only an `Active` axis is a pass**, and `inspect` reports every axis whether it passed or not.
//! `Unchecked` — an axis the gate declares and nobody answered — is a refusal, never a pass; that is
//! the plan's census rule holding in the small. `Inactive` is a checked-and-not-there axis, and it
//! refuses too — *except* for **identity** and **conformance**, the two axes the gate **reports**
//! rather than demands: a `ChecksumOnly` claim is admitted with identity recorded as not established
//! (a checksum is not a signature), and a row that requires nothing of the world is admitted with
//! conformance recorded as nothing asked. Both facts are carried rather than counted, and neither is
//! ever read as a pass. Identity's fact is carried by the **verdict's own shape**
//! ([`Established::ChecksumOnly`], whose `establishes_identity()` is `false`) rather than by a
//! boolean — *"a `true` that means 'the checksum matched' is how a supply-chain guarantee gets
//! believed and does not exist"*. A pin that names another payload is a different fact and refuses by
//! its own name.
//!
//! # The axes, and the ones that are a list of strings
//!
//! An axis here is a **column of the judgement** — the plan's census vocabulary — and it is not U7's
//! civilian axis vocabulary, which is a different thing and is owed. Two axes are deliberately not
//! *checked against a declared list*: the **feature** name (which features exist is the tree of
//! declared rows — U10's, built from `docs/MODS-MAP.md` — and this slice would have to invent it) and
//! the **licence** verdict (deciding a licence is a content decision, so the gate carries the verdict
//! the row relies on and refuses a refused one rather than judging a licence text itself). What the
//! gate does declare, and therefore checks, is its closed carrier set: **U3–U8**, the systems §16 says
//! may carry content derived from a reference.
//!
//! # What this gate still owes (named, not implied)
//!
//! The **document record and its compression** (`Compendium::derive`), the **calibrated structural
//! judge** (`StructuralJudge` + `calibrate`), the **ballot**, the **permission policy matrix**
//! (`policy::decide`), the **recipe/content format**, the **API document** (`apidoc::render`), the
//! **crate × axis census over the tree** (twelve subjects of five axes: its subjects are the tree, and
//! it belongs with the Lane A/B measurement), and **U10's own tree of 91 declared rows**. None of them
//! is stubbed: a plausible verdict from a stub would be exactly the invented default P2 forbids.
//!
//! **The row's conformance against the world's own declarations has since landed** (2026-10-12): the
//! seventh axis judges `requires_stages` against U3's `Declared` and `requires_materials` against U2's
//! `MaterialRegistry` — both **borrowed** through [`WorldDeclarations`], never copied — refusing
//! `stage_undeclared` with the stage named and `unadmitted_material` in the registry's own words.
//! **What is still owed is the served spelling of two states**: a requirement that is declared and
//! **owed**, and one that is declared and names nothing, are reachable through this seam (a caller
//! building a `Row`) but not through a request line — `requires_…=` with nothing after it is refused
//! by the table's own shape rule (`argument_malformed`), exactly as an owed *acceptance check* is
//! unreachable while `check` is a required reference. The wire says so in its own words rather than
//! inventing a spelling: `bridge::gate::answer_admit` carries the paragraph, and the binary prints
//! the owed line.
//!
//! **The wire form of a row has since landed** (2026-10-12): `bridge::TOOLS` declares `admit` with its
//! ten arguments — the two references as `path + digest` each, the feature, the licence verdict, the
//! claim and the carrier as declared words, and the two **optional** lists (`requires_stages`,
//! `requires_materials`) a row uses to declare what it needs of the world — so a row crosses the
//! transport **by reference** and is consumed in place by the seam below. What a row still cannot do
//! over a served line is carry a **pin**: a pin names a signer and a digest an operator states out of
//! band, so `Claim::parse` refuses every claim name but `checksum_only` and the refusal says which
//! claim a line *can* carry.

use std::fmt;
use std::path::{Path, PathBuf};

use crate::digest::Digest;
use crate::material::{MaterialId, MaterialRegistry};
use crate::refusal::{Refusal, RefusalKind, RefusalSet};
use crate::stage::Declared;

// ---------------------------------------------------------------------------
// The declared reference — a document, pointed at rather than carried.
// ---------------------------------------------------------------------------

/// One document **consumed in place by reference**: where it lives, and the digest it must hash to.
/// The bytes are never part of this value, so a row can be declared in a table, printed, compared and
/// carried on the record without dragging a copy of the document behind it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Source {
    path: String,
    digest: Digest,
}

impl Source {
    pub fn new(path: impl Into<String>, digest: Digest) -> Self {
        Source { path: path.into(), digest }
    }

    /// The path, relative to the gate's root, where the document lives — read, never copied.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The digest the bytes at that path must hash to for this reference to be the one it says.
    pub const fn digest(&self) -> Digest {
        self.digest
    }
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.path, self.digest.to_hex())
    }
}

/// **What a row claims about its bytes.** Integrity and identity are different statements and this
/// vocabulary keeps them apart on purpose: a digest *is* the bytes' integrity, and a digest can never
/// establish where the bytes came from — that takes a signer, which takes a signature, which takes a
/// dependency this crate does not have. So a claim is either `ChecksumOnly` (admitted, and the verdict
/// says no identity was proven) or `Pinned` (identity established by **pinning**: an operator named
/// out of band the digest they expect from that signer — a real guarantee, and a different one from a
/// signature, which is why it has a different name).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Claim {
    ChecksumOnly,
    Pinned { signer: String, digest: Digest },
}

impl Claim {
    /// A pinned claim: the signer a payload is attributed to, and the digest an operator expects from
    /// them. The pin carries the payload's digest, not a signature, because there is no signature to
    /// carry.
    pub fn pinned(signer: impl Into<String>, digest: Digest) -> Self {
        Claim::Pinned { signer: signer.into(), digest }
    }

    pub const fn name(&self) -> &'static str {
        match self {
            Claim::ChecksumOnly => "checksum_only",
            Claim::Pinned { .. } => "pinned",
        }
    }

    /// Whether this claim establishes **where the bytes came from** — the same question
    /// [`Established::establishes_identity`] answers about a verdict, asked of the claim that verdict
    /// was read from, so a caller holding either one asks it the same way. `false` for a checksum-only
    /// claim, which is the whole reason neither of them is a `bool`.
    pub const fn establishes_identity(&self) -> bool {
        matches!(self, Claim::Pinned { .. })
    }

    /// The signer this claim attributes the bytes to, if it attributes them to anyone.
    pub fn signer(&self) -> Option<&str> {
        match self {
            Claim::ChecksumOnly => None,
            Claim::Pinned { signer, .. } => Some(signer),
        }
    }

    /// The digest an operator pinned, if this claim is a pin.
    pub fn pin(&self) -> Option<Digest> {
        match self {
            Claim::ChecksumOnly => None,
            Claim::Pinned { digest, .. } => Some(*digest),
        }
    }

    /// The claim a name names, **for a request line and for a verdict's wire form**: only
    /// `ChecksumOnly`. A pin carries a signer and a digest, and it is exactly the statement an
    /// operator makes **out of band** — a request line is not out of band, so a pin is not a name
    /// either of them can carry.
    pub fn parse(name: &str) -> Option<Self> {
        match name.trim() {
            "checksum_only" => Some(Claim::ChecksumOnly),
            _ => None,
        }
    }

    /// **The claim a served verdict's wire form names**, read back from the two facts that form
    /// carries: the signer (absent for a claims-checksum admission) and the digest the bytes were
    /// found at. A **pin**'s digest **is** the digest the bytes were found at when the pin admitted —
    /// had they differed there would be no verdict to serve — so the reader needs no third value.
    /// `None` for a word this vocabulary does not declare, and for a name that contradicts its own
    /// shape (a checksum-only claim that names a signer).
    pub fn from_wire_name(name: &str, signer: Option<&str>, found: Digest) -> Option<Self> {
        match (name.trim(), signer) {
            ("checksum_only", None) => Some(Claim::ChecksumOnly),
            ("pinned", Some(signer)) if !signer.is_empty() => {
                Some(Claim::pinned(signer, found))
            }
            _ => None,
        }
    }
}

/// The **licence verdict a row relies on**. The gate does not decide it — deciding a licence is a
/// content decision, and the plan's rule is that nothing derived may be implemented from a refused
/// reference — so a row states the verdict its source was given, and a `Refused` one is refused before
/// it can carry anything.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Licence {
    Permitted,
    Refused,
}

impl Licence {
    pub const ALL: [Licence; 2] = [Licence::Permitted, Licence::Refused];

    /// How many verdicts the gate declares. Written down, and pinned to [`Licence::ALL`] by a test.
    pub const COUNT: usize = 2;

    pub const fn name(self) -> &'static str {
        match self {
            Licence::Permitted => "permitted",
            Licence::Refused => "refused",
        }
    }

    /// The verdict a name names, or nothing — so a word outside the declared two is refused as
    /// undeclared rather than read as nearly-permitted.
    pub fn parse(name: &str) -> Option<Self> {
        Licence::ALL.iter().copied().find(|licence| licence.name() == name.trim())
    }
}

/// The **carrier**: the unified system that carries what the row declares. Closed, because §16 names
/// the set (U3–U8) and a closed set is what makes *"the gate declares no such carrier"* a fact rather
/// than a guess.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Carrier {
    U3,
    U4,
    U5,
    U6,
    U7,
    U8,
}

impl Carrier {
    pub const ALL: [Carrier; 6] = [
        Carrier::U3,
        Carrier::U4,
        Carrier::U5,
        Carrier::U6,
        Carrier::U7,
        Carrier::U8,
    ];

    /// How many carriers the gate declares. Written down, and pinned to [`Carrier::ALL`] by a test.
    pub const COUNT: usize = 6;

    pub const fn name(self) -> &'static str {
        match self {
            Carrier::U3 => "U3",
            Carrier::U4 => "U4",
            Carrier::U5 => "U5",
            Carrier::U6 => "U6",
            Carrier::U7 => "U7",
            Carrier::U8 => "U8",
        }
    }

    /// The carrier a name names, or nothing — so an undeclared name is refused rather than rounded to
    /// the nearest system.
    pub fn parse(name: &str) -> Option<Self> {
        Carrier::ALL.iter().copied().find(|carrier| carrier.name() == name.trim())
    }
}

/// The row's own **acceptance check**, by reference like everything else: the document that holds it,
/// or the declaration that it is **owed**. A check that does not exist yet is `Owed`, and the gate
/// shows it by name instead of counting it — a row whose own bar cannot be run is not admitted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Acceptance {
    Provided(Source),
    Owed,
}

/// **What a row requires of one of the world's declarations**: named, or owed. The shape mirrors
/// [`Acceptance`] deliberately — one vocabulary for *the bar I bring* and for *the bar the world must
/// already have* — and the two dimensions (stages, materials) are two declarations rather than one
/// list, because a row may require both and a refusal has to say **which** world did not answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Requirement<T> {
    /// The row names what the world must declare, and the owner's own seam is what resolves a name: U3's
    /// roster for a stage, U2's registry for a material id.
    Named(Vec<T>),
    /// The row declares the requirement and cannot name it yet: the bar is **owed**, and an owed bar is
    /// shown by name rather than counted as a pass.
    Owed,
}

/// **What a row requires of the world, in one value**: the two declarations the conformance axis
/// judges. The dimensions stay separate — a stage is a name while a material is an id, and a refusal
/// has to say **which** world did not answer — but they are one declaration *about the row*, so they
/// travel together and no caller can pass one without the other being asked for.
///
/// `Default`/`nothing()` is the row that requires nothing, which is a declaration rather than an
/// omission: it is the state the axis **reports** instead of counting as met.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Requirements {
    /// The stages this row needs the world to run, or `None` when it needs none.
    pub stages: Option<Requirement<String>>,
    /// The materials this row needs the world to admit, or `None` when it needs none.
    pub materials: Option<Requirement<MaterialId>>,
}

impl Requirements {
    /// A row requires nothing of the world: the declaration a row makes by saying nothing, and the one
    /// state of this axis that is reported rather than counted.
    pub const fn nothing() -> Self {
        Requirements { stages: None, materials: None }
    }
}

/// **A declared row**: one thing this gate is asked to admit, carrying only declarations. The feature
/// and the licence are names this row states (see the module header for why the gate does not keep a
/// list of either); the carrier, the source, the acceptance check and the world's own two declarations
/// are the declarations the gate can decide.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    feature: String,
    source: Source,
    licence: Option<Licence>,
    carrier: Option<String>,
    acceptance: Option<Acceptance>,
    claim: Claim,
    requirements: Requirements,
}

impl Row {
    pub fn new(
        feature: impl Into<String>,
        source: Source,
        licence: Option<Licence>,
        carrier: Option<&str>,
        acceptance: Option<Acceptance>,
        claim: Claim,
        requirements: Requirements,
    ) -> Self {
        Row {
            feature: feature.into(),
            source,
            licence,
            carrier: carrier.map(str::to_string),
            acceptance,
            claim,
            requirements,
        }
    }

    pub fn feature(&self) -> &str {
        &self.feature
    }

    pub fn source(&self) -> &Source {
        &self.source
    }

    pub const fn licence(&self) -> Option<Licence> {
        self.licence
    }

    pub fn carrier(&self) -> Option<&str> {
        self.carrier.as_deref()
    }

    pub fn acceptance(&self) -> Option<&Acceptance> {
        self.acceptance.as_ref()
    }

    pub fn claim(&self) -> &Claim {
        &self.claim
    }

    /// Everything this row requires of the world.
    pub const fn requirements(&self) -> &Requirements {
        &self.requirements
    }

    /// The stages this row needs the world to run, or `None` when it needs none.
    pub const fn requires_stages(&self) -> Option<&Requirement<String>> {
        self.requirements.stages.as_ref()
    }

    /// The materials this row needs the world to admit, or `None` when it needs none.
    pub const fn requires_materials(&self) -> Option<&Requirement<MaterialId>> {
        self.requirements.materials.as_ref()
    }
}

// ---------------------------------------------------------------------------
// The axes, and what one decision about one axis is.
// ---------------------------------------------------------------------------

/// One **column of the judgement**: a question the gate asks of every row it is handed. A closed enum,
/// because "which axis was not checked" has to be a value a report can name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Axis {
    /// Does the row name what it carries?
    Feature,
    /// Are the bytes at the declared path the bytes the row describes?
    Source,
    /// Does the verdict the row relies on permit its source?
    Licence,
    /// Is the carrier one of the declared systems (U3–U8)?
    Carrier,
    /// Does the row's own acceptance check exist at its declared reference?
    Acceptance,
    /// What does the row's claim establish — and what does it not?
    Identity,
    /// **Does the world declare what the row requires?** The last column, and the one judged against the
    /// owners' *own* declarations rather than against a list this gate keeps: U3's [`Declared`] roster
    /// and U2's [`MaterialRegistry`] (`docs/LEDGER.md` §55.3 — *"conformance is against the world, not a
    /// list of strings"*). A required stage the roster does not declare is refused
    /// [`RefusalKind::StageUndeclared`] **with the stage named**; a required material the registry never
    /// admitted is refused [`RefusalKind::UnadmittedMaterial`] **with its id named**.
    Conformance,
}

impl Axis {
    /// The columns a census prints, in order. The six a row was first judged on keep their places and a
    /// new column is **appended**, because a verdict's axes are positional: an axis inserted in the
    /// middle would silently relabel a report that already exists.
    pub const ALL: [Axis; 7] = [
        Axis::Feature,
        Axis::Source,
        Axis::Licence,
        Axis::Carrier,
        Axis::Acceptance,
        Axis::Identity,
        Axis::Conformance,
    ];

    /// How many axes the gate declares. Written down, and pinned to [`Axis::ALL`] by a test.
    pub const COUNT: usize = 7;

    pub const fn name(self) -> &'static str {
        match self {
            Axis::Feature => "feature",
            Axis::Source => "source",
            Axis::Licence => "licence",
            Axis::Carrier => "carrier",
            Axis::Acceptance => "acceptance",
            Axis::Identity => "identity",
            Axis::Conformance => "conformance",
        }
    }
}

/// The record's own four states for the question *"was this checked, and what was found?"* — one
/// vocabulary — used for what a feature is and for what a claim establishes, because they are one
/// question asked of two subjects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Establishment {
    /// Checked, and found: the thing is there.
    Active,
    /// Checked, and not there.
    Inactive,
    /// Not checked: nothing was consulted. **Never a pass.**
    Unchecked,
    /// Checked and refused.
    Refused,
}

impl Establishment {
    pub const ALL: [Establishment; 4] = [
        Establishment::Active,
        Establishment::Inactive,
        Establishment::Unchecked,
        Establishment::Refused,
    ];

    /// Whether this state is a pass. **Only `Active` is**, so an unchecked axis can never be read as
    /// one — the rule the plan's census exists to hold.
    pub const fn is_established(self) -> bool {
        matches!(self, Establishment::Active)
    }

    /// The state a name names, or nothing — the reader a verdict's wire form needs, so a state the
    /// writer never produces is refused rather than misread as a nearly-right one.
    pub fn parse(name: &str) -> Option<Self> {
        Establishment::ALL.iter().copied().find(|state| state.name() == name.trim())
    }

    pub const fn name(self) -> &'static str {
        match self {
            Establishment::Active => "active",
            Establishment::Inactive => "inactive",
            Establishment::Unchecked => "unchecked",
            Establishment::Refused => "refused",
        }
    }
}

/// One decision about one axis: **named**, with what it was decided about, how it came out, and (when
/// it did not pass) the refusal it produces. The refusal is carried rather than re-derived from the
/// pair of values, because two different facts can share a state — a check that is owed and a
/// reference that is absent are both `Inactive` and are not the same problem.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    axis: Axis,
    state: Establishment,
    subject: String,
    detail: String,
    /// `None` when nothing is refused: an `Active` axis, and the **one** reported state — the identity
    /// of a checksum-only claim, whose absence of a proof is carried by the verdict's own shape.
    kind: Option<RefusalKind>,
}

impl Finding {
    fn active(axis: Axis, subject: impl Into<String>, detail: impl Into<String>) -> Self {
        Finding {
            axis,
            state: Establishment::Active,
            subject: subject.into(),
            detail: detail.into(),
            kind: None,
        }
    }

    fn unchecked(axis: Axis, subject: impl Into<String>, detail: impl Into<String>) -> Self {
        Finding::not_passing(axis, Establishment::Unchecked, subject, detail, RefusalKind::AxisUnchecked)
    }

    fn inactive(
        axis: Axis,
        subject: impl Into<String>,
        detail: impl Into<String>,
        kind: RefusalKind,
    ) -> Self {
        Finding::not_passing(axis, Establishment::Inactive, subject, detail, kind)
    }

    fn refused(
        axis: Axis,
        subject: impl Into<String>,
        detail: impl Into<String>,
        kind: RefusalKind,
    ) -> Self {
        Finding::not_passing(axis, Establishment::Refused, subject, detail, kind)
    }

    /// The one non-`Active` state that is **reported and does not refuse**: the identity a
    /// `ChecksumOnly` claim cannot establish. Checked, and found absent — which is a fact about the
    /// claim, not a defect in the row.
    fn unproven(axis: Axis, subject: impl Into<String>, detail: impl Into<String>) -> Self {
        Finding {
            axis,
            state: Establishment::Inactive,
            subject: subject.into(),
            detail: detail.into(),
            kind: None,
        }
    }

    fn not_passing(
        axis: Axis,
        state: Establishment,
        subject: impl Into<String>,
        detail: impl Into<String>,
        kind: RefusalKind,
    ) -> Self {
        Finding {
            axis,
            state,
            subject: subject.into(),
            detail: detail.into(),
            kind: Some(kind),
        }
    }

    pub const fn axis(&self) -> Axis {
        self.axis
    }

    pub const fn state(&self) -> Establishment {
        self.state
    }

    pub fn subject(&self) -> &str {
        &self.subject
    }

    pub fn detail(&self) -> &str {
        &self.detail
    }

    /// Whether this axis is a **pass**. Only `Active` is, and this is the one place that is decided.
    pub const fn is_pass(&self) -> bool {
        self.state.is_established()
    }

    /// The refusal this axis produces when it does not pass — `None` for a pass, and for the one
    /// reported state the verdict carries instead.
    pub fn refusal(&self) -> Option<Refusal> {
        self.kind.map(|kind| {
            Refusal::new(
                kind,
                format!("{} axis, subject `{}`: {}", self.axis.name(), self.subject, self.detail),
            )
        })
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} — {} ({})",
            self.axis.name(),
            self.state.name(),
            self.detail,
            self.subject
        )
    }
}

// ---------------------------------------------------------------------------
// The report, and the verdict it may become.
// ---------------------------------------------------------------------------

/// **What the gate found, axis by axis** — every axis, whether it passed or not, collected rather than
/// short-circuited so an author fixes one round of problems instead of one at a time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inspection {
    findings: Vec<Finding>,
    source: Source,
    found: Option<Digest>,
    claim: Claim,
}

impl Inspection {
    /// Every finding, in [`Axis::ALL`] order.
    pub fn findings(&self) -> &[Finding] {
        &self.findings
    }

    /// The finding for one axis, if the gate asked about it. Every axis is always asked about, so this
    /// is `Some` for a report this gate made — and `None` is how a caller writes a report of its own
    /// that the gate did not make.
    pub fn finding(&self, axis: Axis) -> Option<&Finding> {
        self.findings.iter().find(|finding| finding.axis == axis)
    }

    /// The state of one axis.
    pub fn state(&self, axis: Axis) -> Option<Establishment> {
        self.finding(axis).map(Finding::state)
    }

    /// The digest the bytes at the source actually hashed to, when they were read in place.
    pub const fn found(&self) -> Option<Digest> {
        self.found
    }

    /// Every axis that is **not** a pass, as refusals: the whole set, never the first. An empty set
    /// means every axis the gate demanded was established and the claim is what it says.
    pub fn refusals(&self) -> RefusalSet {
        let mut set = RefusalSet::new();
        for finding in &self.findings {
            if let Some(refusal) = finding.refusal() {
                set.push(refusal);
            }
        }
        set
    }

    /// Whether nothing refused. Note what this does **not** say: a row can be admitted with an
    /// identity that was not established, and [`Established::establishes_identity`] is where that is
    /// read. `is_established` means *the gate found nothing to refuse*, not *a `true`*.
    pub fn is_established(&self) -> bool {
        self.refusals().is_empty()
    }

    /// How many axes were **established** — the count a report prints beside the axes the claim could
    /// not establish.
    pub fn axes_established(&self) -> usize {
        self.findings.iter().filter(|finding| finding.is_pass()).count()
    }

    /// The report as text: every axis, its state, what it was about and what was found.
    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "intake report: {} — {} of {} axes established\n",
            self.source,
            self.axes_established(),
            self.findings.len()
        ));
        for finding in &self.findings {
            out.push_str(&format!("  {finding}\n"));
        }
        out
    }

    /// **The verdict.** `Ok` only when nothing refused, and never a boolean: a claims-checksum row is
    /// admitted as [`Established::ChecksumOnly`], whose `establishes_identity()` is `false`, and a
    /// pinned row as [`Established::Pinned`]. A row that refused has no `Established` to reach a
    /// report through, so a caller cannot mistake a refused row for a row admitted with warnings.
    pub fn verdict(&self) -> Result<Established, RefusalSet> {
        let refusals = self.refusals();
        if !refusals.is_empty() {
            return Err(refusals);
        }
        let found = self
            .found
            .expect("an admitted row's source was read in place: its digest is the one declared");
        match &self.claim {
            Claim::ChecksumOnly => Ok(Established::ChecksumOnly {
                source: self.source.clone(),
                found,
                findings: self.findings.clone(),
            }),
            Claim::Pinned { signer, .. } => Ok(Established::Pinned {
                signer: signer.clone(),
                source: self.source.clone(),
                found,
                findings: self.findings.clone(),
            }),
        }
    }
}

/// **What a row became.** Deliberately **not** a `bool`: a row whose bytes are the bytes it describes
/// is admitted and the verdict says **what was proven** — a digest, or a digest and a signer. There is
/// no variant meaning *"true, with a caveat"*, because a `true` that means *"the checksum matched"* is
/// how a guarantee gets believed and does not exist.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Established {
    /// Integrity established — *these are the bytes the row describes* — and **no identity was
    /// proven**: no signer, so nothing to check and nothing to claim.
    ChecksumOnly {
        source: Source,
        found: Digest,
        findings: Vec<Finding>,
    },
    /// Integrity established, and identity established by **pinning**: the payload is the digest an
    /// operator named out of band for that signer. A real guarantee, and a different one from a
    /// signature.
    Pinned {
        signer: String,
        source: Source,
        found: Digest,
        findings: Vec<Finding>,
    },
}

impl Established {
    /// Whether this verdict establishes **where the bytes came from**. `false` for a checksum-only
    /// admission — the value the plan's bar reads, and the reason this type is not a `bool`.
    pub const fn establishes_identity(&self) -> bool {
        matches!(self, Established::Pinned { .. })
    }

    /// The signer this verdict attributes the bytes to, when one was pinned.
    pub fn signer(&self) -> Option<&str> {
        match self {
            Established::ChecksumOnly { .. } => None,
            Established::Pinned { signer, .. } => Some(signer),
        }
    }

    /// The reference this verdict is about.
    pub const fn source(&self) -> &Source {
        match self {
            Established::ChecksumOnly { source, .. } | Established::Pinned { source, .. } => source,
        }
    }

    /// The digest the bytes actually hashed to when they were read in place — read back out of the
    /// document rather than restated from the row.
    pub const fn found(&self) -> Digest {
        match self {
            Established::ChecksumOnly { found, .. } | Established::Pinned { found, .. } => *found,
        }
    }

    /// Every axis, as it was found at admission time.
    pub fn findings(&self) -> &[Finding] {
        match self {
            Established::ChecksumOnly { findings, .. } | Established::Pinned { findings, .. } => {
                findings
            }
        }
    }

    /// The state of one axis in this verdict — how a caller reads a verdict back out as the report it
    /// is, axis by axis, rather than as the one fact it was admitted on.
    pub fn state(&self, axis: Axis) -> Option<Establishment> {
        self.findings()
            .iter()
            .find(|finding| finding.axis() == axis)
            .map(Finding::state)
    }

    pub const fn claim_name(&self) -> &'static str {
        match self {
            Established::ChecksumOnly { .. } => "checksum_only",
            Established::Pinned { .. } => "pinned",
        }
    }

    /// What this row became, in words a reader can check against the document.
    pub fn render(&self) -> String {
        let identity = match self {
            Established::ChecksumOnly { .. } => {
                "identity NOT established (no signer: a checksum is not a signature)".to_string()
            }
            Established::Pinned { signer, .. } => format!("identity established by pinning: {signer}"),
        };
        let established = self.findings().iter().filter(|f| f.is_pass()).count();
        let mut out = format!(
            "admitted: {} — {} ({}), {} of {} axes established\n  {identity}\n",
            self.source(),
            self.claim_name(),
            self.found().to_hex(),
            established,
            self.findings().len()
        );
        for finding in self.findings() {
            out.push_str(&format!("  {finding}\n"));
        }
        out
    }
}

// ---------------------------------------------------------------------------
// The census, and the gate itself.
// ---------------------------------------------------------------------------

/// **The census**: a set of declared rows judged on the gate's axes, cell by cell. It reports; it also
/// refuses — a row with any cell that is not established is not admitted, so *"the gate cannot grade
/// only the new work"* and an unmeasured cell never rides along as a pass.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Census {
    rows: usize,
    admitted: usize,
    cells: Vec<(String, Finding)>,
}

impl Census {
    /// How many declared rows were judged.
    pub const fn rows(&self) -> usize {
        self.rows
    }

    /// How many of them were admitted — *and nothing else*.
    pub const fn admitted(&self) -> usize {
        self.admitted
    }

    /// Every cell: the row's feature, and the finding for one axis.
    pub fn cells(&self) -> &[(String, Finding)] {
        &self.cells
    }

    /// How many cells are in a given state.
    pub fn count(&self, state: Establishment) -> usize {
        self.cells.iter().filter(|(_, finding)| finding.state == state).count()
    }

    /// Every cell that did not pass, as refusals, collected.
    pub fn refusals(&self) -> RefusalSet {
        let mut set = RefusalSet::new();
        for (feature, finding) in &self.cells {
            if let Some(refusal) = finding.refusal() {
                set.refuse(refusal.kind(), format!("`{feature}` {}", refusal.detail()));
            }
        }
        set
    }

    /// Whether every declared row was admitted. A single unmeasured cell makes this `false`.
    pub const fn is_established(&self) -> bool {
        self.admitted == self.rows
    }

    /// The census as a grid, plus its own totals.
    pub fn render(&self) -> String {
        let mut out = format!(
            "census: {} declared row(s) over {} axes ({} cells): {} established, {} inactive, {} \
             unchecked, {} refused — admitted {} of {}\n",
            self.rows,
            Axis::COUNT,
            self.cells.len(),
            self.count(Establishment::Active),
            self.count(Establishment::Inactive),
            self.count(Establishment::Unchecked),
            self.count(Establishment::Refused),
            self.admitted,
            self.rows
        );
        for (feature, finding) in &self.cells {
            out.push_str(&format!("  [{feature}] {finding}\n"));
        }
        out
    }
}

/// **The content-intake gate.** It holds one thing: the **root** the rows' paths are relative to. It
/// holds no document, no digest it was not handed and no verdict it did not just measure — every
/// admission reads the bytes where they live, every time.
#[derive(Clone, Copy, Debug)]
pub struct WorldDeclarations<'a> {
    /// U3's roster: which stages the world runs.
    pub roster: &'a Declared,
    /// U2's registry: which materials the world is made of.
    pub registry: &'a MaterialRegistry,
}

impl PartialEq for WorldDeclarations<'_> {
    /// Two declarations are **one world** exactly when the owners' own vocabularies have the same
    /// identity — the two digests, never a pointer, so two equal registries are equal declarations and
    /// one registry borrowed twice is not two worlds.
    fn eq(&self, other: &Self) -> bool {
        self.roster.digest() == other.roster.digest() && self.registry.digest() == other.registry.digest()
    }
}

impl Eq for WorldDeclarations<'_> {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Intake<'a> {
    root: PathBuf,
    /// The world's own declarations, **borrowed**: conformance is against the world, so this gate keeps
    /// the seams rather than a copy of either vocabulary (`LEDGER` §55.3).
    world: WorldDeclarations<'a>,
}

impl<'a> Intake<'a> {
    /// An intake over a reference root, conforming against the world's own declarations.
    pub fn over(root: impl Into<PathBuf>, world: WorldDeclarations<'a>) -> Self {
        Intake { root: root.into(), world }
    }

    /// Re-pin the root the rows' references resolve against, **keeping** the world this gate conforms
    /// against: the two declarations are separate facts, and moving the content must not move the world.
    pub fn set_root(&mut self, root: impl Into<PathBuf>) {
        self.root = root.into();
    }

    /// The place the rows' references are resolved against — declared once, so a row's path can stay
    /// portable (`docs/MODS-MAP.md`) instead of naming a machine.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// **Inspect**: ask every axis and report every answer, including the ones that did not pass. This
    /// is the cheap door — it fetches nothing it was not asked to and it admits nothing.
    pub fn inspect(&self, row: &Row) -> Inspection {
        let mut findings = Vec::with_capacity(Axis::COUNT);

        // 1. The feature — what the row is about. The gate does not keep the list of features (see the
        //    header): what it demands is that the row names what it carries, because a row that names
        //    nothing is nothing to admit.
        let feature = row.feature().trim();
        findings.push(if feature.is_empty() {
            Finding::unchecked(
                Axis::Feature,
                "(unnamed)",
                "the row names no feature, so there is nothing to admit",
            )
        } else {
            Finding::active(Axis::Feature, feature, "the row names what it carries")
        });

        // 2. The source — consumed **in place, now**. `found` is what the bytes hashed to; the bytes
        //    themselves are dropped the moment the digest exists.
        let found = match self.read_at(row.source()) {
            Ok(bytes) => {
                let digest = Digest::of(&bytes);
                if digest == row.source().digest() {
                    findings.push(Finding::active(
                        Axis::Source,
                        row.source().path(),
                        format!(
                            "read in place: the bytes hash to {}, the row's declared digest",
                            digest.to_hex()
                        ),
                    ));
                } else {
                    findings.push(Finding::refused(
                        Axis::Source,
                        row.source().path(),
                        format!(
                            "the bytes at {} hash to {}, and the row declares {} — these are not the \
                             bytes the row describes",
                            row.source().path(),
                            digest.to_hex(),
                            row.source().digest().to_hex()
                        ),
                        RefusalKind::SourceDigestMismatch,
                    ));
                }
                Some(digest)
            }
            Err(detail) => {
                findings.push(Finding::inactive(
                    Axis::Source,
                    row.source().path(),
                    detail,
                    RefusalKind::SourceAbsent,
                ));
                None
            }
        };

        // 3. The licence verdict the row relies on. The gate carries the verdict; it does not decide
        //    one, and it does not accept a row that relies on a refusal.
        findings.push(match row.licence() {
            Some(Licence::Permitted) => Finding::active(
                Axis::Licence,
                Licence::Permitted.name(),
                "the row relies on a verdict that permits its source",
            ),
            Some(Licence::Refused) => Finding::refused(
                Axis::Licence,
                Licence::Refused.name(),
                "the row relies on a verdict that refuses its source, and nothing derived may be \
                 implemented from a refused reference",
                RefusalKind::LicenceRefused,
            ),
            None => Finding::unchecked(
                Axis::Licence,
                "(none)",
                "the row relies on no licence verdict, so nothing was checked about whether its \
                 source may be used",
            ),
        });

        // 4. The carrier — the declared system that carries what the row declares.
        findings.push(match row.carrier() {
            Some(name) => match Carrier::parse(name) {
                Some(carrier) => Finding::active(
                    Axis::Carrier,
                    carrier.name(),
                    "the gate declares this carrier",
                ),
                None => Finding::refused(
                    Axis::Carrier,
                    name,
                    format!(
                        "the gate declares no carrier `{}`; it declares {}",
                        name.trim(),
                        Carrier::ALL
                            .iter()
                            .map(|carrier| carrier.name())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    RefusalKind::CarrierUndeclared,
                ),
            },
            None => Finding::unchecked(
                Axis::Carrier,
                "(none)",
                "the row names no carrier, so no system was checked as the one that carries it",
            ),
        });

        // 5. The row's own acceptance check.
        findings.push(match row.acceptance() {
            Some(Acceptance::Provided(source)) => match self.read_at(source).map(|bytes| Digest::of(&bytes)) {
                Ok(digest) if digest == source.digest() => Finding::active(
                    Axis::Acceptance,
                    source.path(),
                    format!("read in place: the check is the one declared ({})", digest.to_hex()),
                ),
                Ok(digest) => Finding::refused(
                    Axis::Acceptance,
                    source.path(),
                    format!(
                        "the bytes at {} hash to {}, and the row declares {} — the check is not the \
                         one declared",
                        source.path(),
                        digest.to_hex(),
                        source.digest().to_hex()
                    ),
                    RefusalKind::SourceDigestMismatch,
                ),
                Err(detail) => Finding::inactive(
                    Axis::Acceptance,
                    source.path(),
                    detail,
                    RefusalKind::SourceAbsent,
                ),
            },
            Some(Acceptance::Owed) => Finding::inactive(
                Axis::Acceptance,
                "owed",
                "the row declares its own acceptance check as owed: a bar that cannot be run cannot be \
                 met",
                RefusalKind::AxisInactive,
            ),
            None => Finding::unchecked(
                Axis::Acceptance,
                "(none)",
                "the row declares no acceptance check, so nothing about its bar was checked",
            ),
        });

        // 6. Identity — the axis the gate **reports** rather than demands. A checksum-only claim was
        //    examined and cannot establish origin; the verdict carries that, so this one finding is
        //    reported without refusing.
        findings.push(match row.claim() {
            Claim::ChecksumOnly => Finding::unproven(
                Axis::Identity,
                Claim::ChecksumOnly.name(),
                "the bytes' digest is established and no signer is named: identity is not established, \
                 and the verdict says so rather than a `true`",
            ),
            Claim::Pinned { signer, digest } => match found {
                None => Finding::unchecked(
                    Axis::Identity,
                    signer,
                    "the bytes could not be read, so the pin could not be compared",
                ),
                Some(found) if found == *digest => Finding::active(
                    Axis::Identity,
                    signer,
                    format!(
                        "pinned: the payload is the digest an operator named out of band ({})",
                        digest.to_hex()
                    ),
                ),
                Some(found) => Finding::refused(
                    Axis::Identity,
                    signer,
                    format!(
                        "the payload hashes to {} and the pin for {} names {} — this is not the \
                         release that was pinned",
                        found.to_hex(),
                        signer,
                        digest.to_hex()
                    ),
                    RefusalKind::PinMismatch,
                ),
            },
        });

        // 7. **Conformance**: does the world declare what this row requires? Judged against the owners'
        //    own declarations — U3's roster for a stage, U2's registry for a material — and never against
        //    a list of names this gate keeps (`LEDGER` §55.3). This axis does not short-circuit the
        //    others: a row whose digest is wrong **and** which requires a stage the world does not run
        //    reports both, because `admit` collects every problem rather than the first.
        findings.push(self.conform(row));

        Inspection {
            findings,
            source: row.source().clone(),
            found,
            claim: row.claim().clone(),
        }
    }

    /// **One decision about conformance.** The row says what it needs of the world; the world's own
    /// seams say whether it has it.
    ///
    /// | the row says | this axis |
    /// |---|---|
    /// | nothing | `inactive` *(reported)* — nothing was asked, so nothing is missing |
    /// | a requirement, naming nothing | `unchecked` → `axis_unchecked`: nothing was consulted |
    /// | a requirement it cannot name | `inactive` → `axis_inactive`: the bar is owed |
    /// | named items the world has | `active` |
    /// | a named item the world does not have | `refused`, with **every** missing item named |
    ///
    /// A row that requires nothing is **reported** rather than counted as a pass — the treatment the
    /// identity of a checksum-only claim already gets — while a requirement that names nothing is a
    /// refusal, because a bar nobody can run is not a bar that was met.
    fn conform(&self, row: &Row) -> Finding {
        let stages = row.requires_stages();
        let materials = row.requires_materials();
        if stages.is_none() && materials.is_none() {
            return Finding::unproven(
                Axis::Conformance,
                "(none)",
                "the row requires nothing of the world, so nothing was asked and nothing is missing",
            );
        }
        if matches!(stages, Some(Requirement::Named(names)) if names.is_empty())
            || matches!(materials, Some(Requirement::Named(ids)) if ids.is_empty())
        {
            return Finding::unchecked(
                Axis::Conformance,
                "(unnamed)",
                "the row declares a requirement and names nothing, so nothing about conformance was \
                 checked",
            );
        }
        if matches!(stages, Some(Requirement::Owed)) || matches!(materials, Some(Requirement::Owed)) {
            return Finding::inactive(
                Axis::Conformance,
                "owed",
                "the row declares a requirement owed: a bar that cannot be run cannot be met",
                RefusalKind::AxisInactive,
            );
        }

        // Named: ask the world's own declarations, item by item, and collect **every** item that is not
        // there. A stage is resolved by U3's roster and a material by U2's registry — the owners' seams,
        // never a copy of either vocabulary.
        let mut declared_stages: Vec<&str> = Vec::new();
        let mut missing_stages: Vec<&str> = Vec::new();
        if let Some(Requirement::Named(names)) = stages {
            for name in names {
                if self.world.roster.stage(name).is_some() {
                    declared_stages.push(name);
                } else {
                    missing_stages.push(name);
                }
            }
        }
        let mut admitted_materials: Vec<&str> = Vec::new();
        let mut missing_materials: Vec<MaterialId> = Vec::new();
        if let Some(Requirement::Named(ids)) = materials {
            for id in ids {
                match self.world.registry.material(*id) {
                    Ok(material) => admitted_materials.push(material.name),
                    Err(_) => missing_materials.push(*id),
                }
            }
        }

        if !missing_stages.is_empty() {
            // The refusal names the world's own declaration, the way `carrier_undeclared` names the
            // carriers the gate declares. The **kind** is the first dimension that did not answer,
            // because an axis is one column; when a stage and a material are both missing, the detail
            // carries both rather than the second going unsaid.
            let declared: Vec<&str> = self
                .world
                .roster
                .stages()
                .iter()
                .map(|stage| stage.name)
                .collect();
            let detail = format!(
                "the world declares no stage {} — it declares {}",
                quoted(&missing_stages),
                declared.join(", ")
            );
            let detail = match missing_materials.is_empty() {
                true => detail,
                false => format!(
                    "{detail}; and its registry admits no material {}",
                    ids(&missing_materials)
                ),
            };
            return Finding::refused(
                Axis::Conformance,
                quoted(&missing_stages),
                detail,
                RefusalKind::StageUndeclared,
            );
        }
        if !missing_materials.is_empty() {
            // The registry's **own** refusal, passed through: it is the owner that did not admit the id,
            // and its message already names it.
            let first = self
                .world
                .registry
                .material(missing_materials[0])
                .expect_err("the id is not admitted");
            let detail = match missing_materials.len() {
                1 => first.detail().to_string(),
                _ => format!(
                    "the registry admits none of {} — {}",
                    ids(&missing_materials),
                    first.detail()
                ),
            };
            return Finding::refused(
                Axis::Conformance,
                ids(&missing_materials),
                detail,
                RefusalKind::UnadmittedMaterial,
            );
        }

        Finding::active(
            Axis::Conformance,
            match (declared_stages.is_empty(), admitted_materials.is_empty()) {
                (false, false) => format!(
                    "stages {} and materials {}",
                    quoted(&declared_stages),
                    quoted(&admitted_materials)
                ),
                (false, true) => format!("stages {}", quoted(&declared_stages)),
                (true, false) => format!("materials {}", quoted(&admitted_materials)),
                (true, true) => "(nothing named)".to_string(),
            },
            "the world declares every stage this row requires and admits every material it requires",
        )
    }

    /// **Admit**: inspect, then decide. `Ok(Established)` only when nothing refused; `Err` carrying
    /// **every** refusal otherwise, because a row is not partially admitted.
    pub fn admit(&self, row: &Row) -> Result<Established, RefusalSet> {
        self.inspect(row).verdict()
    }

    /// **The census**: every declared row judged on every axis, reported as a grid and refused as a
    /// whole when any cell is not established.
    pub fn census(&self, rows: &[Row]) -> Census {
        let mut census = Census { rows: rows.len(), admitted: 0, cells: Vec::new() };
        for row in rows {
            let inspection = self.inspect(row);
            if inspection.is_established() {
                census.admitted += 1;
            }
            for finding in inspection.findings() {
                census.cells.push((row.feature().to_string(), finding.clone()));
            }
        }
        census
    }

    /// **Consume a declared reference in place, as bytes** — the door the *document form* of a save is
    /// read through (`specs/006-world-saves-and-seeds/plan.md`'s named-owed document form, landed
    /// 2026-10-12, §28). The bytes are handed over **only** when they are the bytes the reference
    /// declares: a document that is not there refuses [`RefusalKind::SourceAbsent`], and one whose
    /// bytes have moved refuses [`RefusalKind::SourceDigestMismatch`] — by name, with both values in
    /// the message — rather than being parsed into a nearly-right world. This is the same read the
    /// source axis makes ([`Intake::inspect`]); there is **one** `fs::read` site and one rule about
    /// which bytes a reference is, and the two doors differ only in what they hand back (a finding, or
    /// the bytes themselves).
    ///
    /// The bytes are the caller's to interpret and are **not** kept: an intake carries no document, so
    /// a document that changes under a caller's feet is refused by name rather than served from a
    /// copy.
    pub fn read(&self, source: &Source) -> Result<Vec<u8>, RefusalSet> {
        let mut set = RefusalSet::new();
        let bytes = match self.read_at(source) {
            Ok(bytes) => bytes,
            Err(detail) => {
                set.refuse(RefusalKind::SourceAbsent, detail);
                return Err(set);
            }
        };
        let found = Digest::of(&bytes);
        if found != source.digest() {
            set.refuse(
                RefusalKind::SourceDigestMismatch,
                format!(
                    "the bytes at {} hash to {}, and the reference declares {} — these are not the \
                     bytes it describes",
                    source.path(),
                    found.to_hex(),
                    source.digest().to_hex()
                ),
            );
            return Err(set);
        }
        Ok(bytes)
    }

    /// Read one declared reference **in place**: the path, relative to the root, read now, as the
    /// bytes that are there. The **only** place this gate touches a document, so the source axis and
    /// [`Intake::read`] cannot disagree about which bytes a reference is.
    ///
    /// A read that failed is described in **stable** words: the reference as the row declared it, and
    /// the operating system's own classification. Never [`std::io::Error`]'s `Display` and never the
    /// path this host normalized, because a detail is not incidental — it goes into a response, into a
    /// report, and (were it recorded) into the record, where the same world on two hosts would keep
    /// two different records.
    fn read_at(&self, source: &Source) -> Result<Vec<u8>, String> {
        std::fs::read(self.root.join(source.path())).map_err(|error| {
            format!(
                "the declared reference `{}` could not be read in place: {}",
                source.path(),
                io_reason(&error)
            )
        })
    }
}

/// Names in a refusal's own words: backticked and comma-separated, so a missing item is legible in the
/// message rather than merely implied by the kind.
fn quoted(names: &[&str]) -> String {
    names.iter().map(|name| format!("`{name}`")).collect::<Vec<_>>().join(", ")
}

/// Material ids, in the registry's own spelling.
fn ids(ids: &[MaterialId]) -> String {
    ids.iter().map(|id| id.0.to_string()).collect::<Vec<_>>().join(", ")
}

/// Why a filesystem call failed, in the **stable** half of what the operating system said: its
/// classification, spelled here. [`std::io::Error`]'s `Display` is deliberately not used — it is this
/// host's prose (`The system cannot find the file specified. (os error 2)`) and it would make one
/// failure read differently on another host. One spelling for both doors of a declared reference —
/// the read here and the write the document form makes (`save::Document::write`) — so a document that
/// cannot be placed is refused in the same words as one that cannot be read.
pub(crate) fn io_reason(error: &std::io::Error) -> &'static str {
    match error.kind() {
        std::io::ErrorKind::NotFound => "no such document",
        std::io::ErrorKind::PermissionDenied => "permission denied",
        // A direction-neutral word, because this one mapping now answers for **both** doors: a read
        // that failed for some other reason and a write that could not place the document are the
        // same classification question, and "unreadable" was only ever true of the first.
        _ => "unavailable",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_carrier_vocabulary_is_closed_and_parses_both_ways() {
        assert_eq!(Carrier::ALL.len(), Carrier::COUNT);
        for (index, carrier) in Carrier::ALL.iter().enumerate() {
            assert!(
                Carrier::ALL[..index].iter().all(|other| other != carrier),
                "{} is declared twice",
                carrier.name()
            );
            assert_eq!(Carrier::parse(carrier.name()), Some(*carrier));
            assert_eq!(Carrier::parse(&format!(" {} ", carrier.name())), Some(*carrier));
        }
        assert!(Carrier::parse("U9").is_none(), "U9 is admitted by the plan, not declared here");
        assert!(Carrier::parse("voxel-mesh").is_none());
    }

    #[test]
    fn only_an_active_axis_is_a_pass() {
        assert_eq!(Establishment::ALL.len(), 4);
        assert!(Establishment::Active.is_established());
        for state in [
            Establishment::Inactive,
            Establishment::Unchecked,
            Establishment::Refused,
        ] {
            assert!(
                !state.is_established(),
                "{} must never be read as a pass",
                state.name()
            );
        }
        // The axes are declared once each, in a stable order, and each one is named.
        assert_eq!(Axis::ALL.len(), Axis::COUNT);
        for (index, axis) in Axis::ALL.iter().enumerate() {
            assert!(Axis::ALL[..index].iter().all(|other| other != axis));
            assert!(!axis.name().is_empty());
        }
    }

    #[test]
    fn a_source_prints_the_path_and_the_digest_it_must_hash_to() {
        let source = Source::new("docs/MODS-MAP.md", Digest::of(b"a document"));
        assert_eq!(source.path(), "docs/MODS-MAP.md");
        assert_eq!(source.digest(), Digest::of(b"a document"));
        let printed = source.to_string();
        assert!(printed.starts_with("docs/MODS-MAP.md ("));
        assert!(printed.contains(&Digest::of(b"a document").to_hex()));
    }

    #[test]
    fn a_finding_refuses_by_its_own_kind_and_a_pass_refuses_nothing() {
        let pass = Finding::active(Axis::Source, "a.md", "read in place");
        assert!(pass.is_pass());
        assert!(pass.refusal().is_none());

        let unchecked = Finding::unchecked(Axis::Licence, "(none)", "nothing was checked");
        assert!(!unchecked.is_pass());
        assert_eq!(
            unchecked.refusal().expect("an unchecked axis refuses").kind(),
            RefusalKind::AxisUnchecked
        );

        let unproven = Finding::unproven(Axis::Identity, "checksum_only", "no signer");
        assert!(!unproven.is_pass(), "a reported absence of a proof is not a pass");
        assert!(
            unproven.refusal().is_none(),
            "and the verdict carries it instead of a refusal"
        );
        assert_eq!(unproven.state(), Establishment::Inactive);
    }

    #[test]
    fn the_wire_vocabularies_parse_only_what_they_declare() {
        assert_eq!(Licence::ALL.len(), Licence::COUNT);
        for licence in Licence::ALL {
            assert_eq!(Licence::parse(licence.name()), Some(licence));
        }
        assert!(Licence::parse("maybe").is_none());

        for state in Establishment::ALL {
            assert_eq!(Establishment::parse(state.name()), Some(state));
        }
        assert!(Establishment::parse("established").is_none());

        assert_eq!(Claim::parse("checksum_only"), Some(Claim::ChecksumOnly));
        assert!(
            Claim::parse("pinned").is_none(),
            "a pin carries a signer and a digest, and it is made out of band: a request line is not \
             out of band"
        );
    }

    #[test]
    fn a_claim_names_what_it_establishes_and_a_checksum_establishes_no_identity() {
        let checksum = Claim::ChecksumOnly;
        assert_eq!(checksum.name(), "checksum_only");
        assert_eq!(checksum.signer(), None);
        assert_eq!(checksum.pin(), None);

        let pin = Digest::of(b"the release an operator expects");
        let pinned = Claim::pinned("an operator", pin);
        assert_eq!(pinned.name(), "pinned");
        assert_eq!(pinned.signer(), Some("an operator"));
        assert_eq!(pinned.pin(), Some(pin));
    }
}
