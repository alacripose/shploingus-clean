//! **Tickets and governance, as a view over the one chain.** (UI-SPEC §5 tab 3)
//!
//! # The lineage this joins, and what each one contributes
//!
//! The record is explicit that the past projects **each lacked key features others had**, and
//! that this tree is the union rather than a copy (L-2's map, and the user's framing that the
//! lineage has been *the hardest thing to carry through*). So this module takes from three
//! places and adds nothing of its own:
//!
//! | what | from where | why here |
//! |---|---|---|
//! | `TicketStatus` — **nine** governed values; `TicketKind`; `Expectation`; `Evidence` grades; `RetirementReason` — **14** tokens; the separate `Retirement` record; append-only `Correction` | **`ala-cities/src/gov/mod.rs`**, read directly | the finished ticket model. Its `Expectation` is *"what the world must show, read back from the world, never inferred from the request that filed it"* — the one idea the whole surface turns on |
//! | **deny by default; permission is a property of the action; a grant with no granter or no scope is refused construction; a refusal quotes the list** | **`ala-system/src/security/policy.ts`** (L-5) | **already implemented** as `voxel_package::policy`, and this module **routes through it** rather than inventing a second gate |
//! | **the chain, the namespace, `not_evaluable` honesty, one writer** | **this tree's record** (H-8.1, H-11.7, C-54.1) | the ticket *is* a projection of the chain, so a ticket that disagrees with the chain is impossible by construction |
//!
//! **Nothing is invented here.** Where the record is silent it says so on the field rather than
//! filling the gap.
//!
//! # Why this is a view and not a second store
//!
//! H-8.1: *"**One append-only record, two views** … `docs/LEDGER.md` and the in-game ledger
//! window are two projections of one record, so a disagreement between them is impossible by
//! construction."* UI-SPEC §5 tab 3 is the other window: *"the ledger's tickets, their
//! expectations, their retirements; the observer."*
//!
//! So a [`Ticket`] here is **derived by folding the chain** ([`crate::chain::Chain::fold`]),
//! never stored beside it. [`Board::from_chain`] is the only constructor, and there is no
//! `Board::new`. A board that could be built without the chain would be a second source of
//! truth, which is the exact thing H-8.1 exists to prevent.
//!
//! # The rule that makes retirement more than a status
//!
//! The reference states it and this tree inherits the reason:
//!
//! > **Retirement is a separate `ticket.retirement` event**: a status flip, an
//! > `outcome.recorded`, or a `STATUS.md` projection is *not* a retirement record.
//!
//! [`Board::retire`] therefore **refuses** to move a ticket to a terminal status without a
//! [`Retirement`] behind it — [`RefusalKind::UnretiredTicket`]. And
//! `RetirementReason::is_validated` is true for exactly one token, `completed_and_validated`,
//! so *"we did it and checked"* and *"we stopped without checking"* stay different facts. That
//! distinction is the reason the enum has fourteen members rather than a `bool`.
//!
//! # The board must drain, and closing unverified is required
//!
//! The reference's rule, kept: every ticket reaches a terminal state, and a ticket whose
//! expectation was **not** read back from the world closes as
//! [`RetirementReason::CompletedButUnverified`] — because
//! *"cannot stall must never be implemented as auto-promote to success."*
//! [`Board::retire`] takes the read-back result as an argument rather than inferring it, so
//! that rule cannot be skipped by a caller who forgot.

use crate::chain::{Chain, EventKind, Namespace};
use crate::identity::Digest;
use crate::refusal::{RefusalKind, RefusalSet};

/// A ticket's id. **Derived from the chain, never from a counter** — the reference's rule
/// (*"ids come from the directory, never from a counter"*), for the reason it gives: a
/// process killed mid-run leaves a counter behind its own directory, and trusting it hands
/// out an id that already exists, which an append-only writer would then have to overwrite.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TicketId(String);

impl TicketId {
    /// An id, refusing an empty one.
    pub fn new(value: impl Into<String>) -> Result<Self, RefusalSet> {
        let value = value.into();
        let mut refusals = RefusalSet::new();
        if value.trim().is_empty() {
            refusals.refuse(
                RefusalKind::MalformedIdentity,
                "a ticket with no id cannot be cited. An uncitable ticket is one nobody can close",
            );
        }
        refusals.into_result(TicketId(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The governed status set. **Exactly nine**, from `ala-cities::gov::TicketStatus`.
///
/// Not this tree's invention, and the count is not negotiable by accident: a tenth status is
/// a change to how work is governed, so it is a decision rather than a variant someone adds.
///
/// The order is the reference's, and it reads as the life of a ticket: filed, taken up,
/// working, stopped, awaiting review, then one of four terminal outcomes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TicketStatus {
    Queued,
    Open,
    Active,
    Blocked,
    Review,
    Complete,
    Rejected,
    Handled,
    Dropped,
}

impl TicketStatus {
    pub const ALL: [TicketStatus; 9] = [
        TicketStatus::Queued,
        TicketStatus::Open,
        TicketStatus::Active,
        TicketStatus::Blocked,
        TicketStatus::Review,
        TicketStatus::Complete,
        TicketStatus::Rejected,
        TicketStatus::Handled,
        TicketStatus::Dropped,
    ];

    /// **A literal, on purpose** — the same reason `FEATURE_COUNT` is one. Deriving it from
    /// `ALL.len()` would make the count and the list agree by construction, and the check
    /// below would be checking nothing.
    pub const COUNT: usize = 9;

    pub const fn name(self) -> &'static str {
        match self {
            TicketStatus::Queued => "queued",
            TicketStatus::Open => "open",
            TicketStatus::Active => "active",
            TicketStatus::Blocked => "blocked",
            TicketStatus::Review => "review",
            TicketStatus::Complete => "complete",
            TicketStatus::Rejected => "rejected",
            TicketStatus::Handled => "handled",
            TicketStatus::Dropped => "dropped",
        }
    }

    pub fn parse(word: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|status| status.name() == word.trim())
            .ok_or_else(|| {
                format!(
                    "unknown ticket status {word:?}; the governed set has {}: {}",
                    Self::COUNT,
                    Self::ALL.map(|s| s.name()).join(", ")
                )
            })
    }

    /// Whether no further work is expected.
    ///
    /// **`Blocked` counts as terminal, and that is deliberate.** The reference includes it, and
    /// the reason is the drain rule: a board where a blocked ticket is not terminal can never
    /// drain, and a board that cannot drain is a board that accumulates. Being blocked is a
    /// real end state; pretending it is still progress is not.
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            TicketStatus::Complete
                | TicketStatus::Rejected
                | TicketStatus::Handled
                | TicketStatus::Dropped
                | TicketStatus::Blocked
        )
    }
}

/// **Check 1.** Every status has a row, in a `const`. The value is meaningless; the type is
/// the assertion.
pub const STATUSES_ARE_TOTAL: [(); TicketStatus::COUNT] = [(); TicketStatus::ALL.len()];

/// What a ticket is about. **Four**, from `ala-cities::gov::TicketKind`.
///
/// `Material` is its own kind rather than riding `Build` for the reason the reference gives:
/// so material claims drain and report separately instead of hiding inside placement claims.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TicketKind {
    /// Something to be made.
    Build,
    /// An observed condition worth a decision.
    Case,
    /// Something to be done about the world.
    Intervention,
    /// A claim about what something is made of.
    Material,
}

impl TicketKind {
    pub const ALL: [TicketKind; 4] = [
        TicketKind::Build,
        TicketKind::Case,
        TicketKind::Intervention,
        TicketKind::Material,
    ];

    pub const COUNT: usize = 4;

    /// The id prefix. **`BLD`/`CSE`/`INT`/`MAT`**, from the reference.
    ///
    /// Carried because it is what makes a ticket id readable in a log line, and the reference
    /// uses it for exactly that.
    pub const fn prefix(self) -> &'static str {
        match self {
            TicketKind::Build => "BLD",
            TicketKind::Case => "CSE",
            TicketKind::Intervention => "INT",
            TicketKind::Material => "MAT",
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            TicketKind::Build => "build",
            TicketKind::Case => "case",
            TicketKind::Intervention => "intervention",
            TicketKind::Material => "material",
        }
    }

    pub fn parse(word: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.name() == word.trim())
            .ok_or_else(|| {
                format!(
                    "unknown ticket kind {word:?}; there are {}: {}",
                    Self::COUNT,
                    Self::ALL.map(|k| k.name()).join(", ")
                )
            })
    }

    /// The kind an **id prefix** names, or `None`.
    ///
    /// **Separate from [`TicketKind::parse`] on purpose.** `build` and `BLD` are two spellings
    /// of one kind, and the record's ids use the prefix while the chain's marker is what a
    /// person writes. Having one function that accepted both would make the id's spelling and
    /// the prose spelling interchangeable, and they are not: an id is read by a program and a
    /// word is read by a person.
    pub fn from_prefix(prefix: &str) -> Option<Self> {
        let wanted = prefix.trim();
        Self::ALL.into_iter().find(|kind| kind.prefix() == wanted)
    }
}

/// **Check 2.** Every kind has a row.
pub const KINDS_ARE_TOTAL: [(); TicketKind::COUNT] = [(); TicketKind::ALL.len()];

/// Why work stopped being active. **Fourteen**, from `ala-cities::gov::RetirementReason`.
///
/// *"These are not interchangeable: 'we did it and checked' and 'we stopped without checking'
/// are different facts, and collapsing them is how a record starts lying."* Fourteen members
/// exist because this project has fourteen distinguishable ways for work to end, and a `bool`
/// would merge the ones that matter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RetirementReason {
    CompletedAndValidated,
    CompletedWithKnownRegression,
    CompletedButUnverified,
    CancelledByAuthority,
    Superseded,
    Invalidated,
    BlockedAndClosed,
    Abandoned,
    Duplicate,
    Obsolete,
    ObjectiveWithdrawn,
    AuthorizationExpired,
    SafetyStop,
    SystemShutdown,
}

impl RetirementReason {
    pub const ALL: [RetirementReason; 14] = [
        RetirementReason::CompletedAndValidated,
        RetirementReason::CompletedWithKnownRegression,
        RetirementReason::CompletedButUnverified,
        RetirementReason::CancelledByAuthority,
        RetirementReason::Superseded,
        RetirementReason::Invalidated,
        RetirementReason::BlockedAndClosed,
        RetirementReason::Abandoned,
        RetirementReason::Duplicate,
        RetirementReason::Obsolete,
        RetirementReason::ObjectiveWithdrawn,
        RetirementReason::AuthorizationExpired,
        RetirementReason::SafetyStop,
        RetirementReason::SystemShutdown,
    ];

    /// **A literal.** Same reason as everywhere else.
    pub const COUNT: usize = 14;

    pub const fn name(self) -> &'static str {
        match self {
            RetirementReason::CompletedAndValidated => "completed_and_validated",
            RetirementReason::CompletedWithKnownRegression => "completed_with_known_regression",
            RetirementReason::CompletedButUnverified => "completed_but_unverified",
            RetirementReason::CancelledByAuthority => "cancelled_by_authority",
            RetirementReason::Superseded => "superseded",
            RetirementReason::Invalidated => "invalidated",
            RetirementReason::BlockedAndClosed => "blocked_and_closed",
            RetirementReason::Abandoned => "abandoned",
            RetirementReason::Duplicate => "duplicate",
            RetirementReason::Obsolete => "obsolete",
            RetirementReason::ObjectiveWithdrawn => "objective_withdrawn",
            RetirementReason::AuthorizationExpired => "authorization_expired",
            RetirementReason::SafetyStop => "safety_stop",
            RetirementReason::SystemShutdown => "system_shutdown",
        }
    }

    pub fn parse(word: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|reason| reason.name() == word.trim())
            .ok_or_else(|| {
                format!(
                    "unknown retirement reason {word:?}; the taxonomy has {}: {}",
                    Self::COUNT,
                    Self::ALL.map(|r| r.name()).join(", ")
                )
            })
    }

    /// **True for exactly one token.**
    ///
    /// The reference's own rule, and it is the whole reason this is not a `bool`: only
    /// `completed_and_validated` means the work was both done *and* checked. Every other
    /// "completed" token says something was finished and something was learned that is not
    /// the same fact.
    pub const fn is_validated(self) -> bool {
        matches!(self, RetirementReason::CompletedAndValidated)
    }

    /// Whether this reason means the work **finished**, as opposed to stopped.
    pub const fn is_completion(self) -> bool {
        matches!(
            self,
            RetirementReason::CompletedAndValidated
                | RetirementReason::CompletedWithKnownRegression
                | RetirementReason::CompletedButUnverified
        )
    }
}

/// **Check 3.** Every reason has a row.
pub const REASONS_ARE_TOTAL: [(); RetirementReason::COUNT] = [(); RetirementReason::ALL.len()];

/// **Where the evidence came from.** **Three origins**, from `ala-cities::gov::Evidence`.
///
/// # This was called `EvidenceGrade`, and both halves of that were wrong
///
/// **It is not a grade.** A grade ranks strength; these three are *sources* — an external
/// oracle, a read-back from world state, and a self-report. The type's own `describe` strings
/// already said so (*"A — an external oracle decided this"*, *"C — self-report; evidence, but not
/// authority"*) while its name promised the question it does not ask. `ala-system` draws the
/// distinction explicitly and enforces it: `EvidenceLevel` is **strength**
/// (`direct | strongly_supported | supported | inferred | self_reported | unverified`) and
/// `ClaimOrigin` is **how the claim came to exist** — and `claim.ts` combines the two, because
/// *"a model-generated hypothesis must never be resume-safe regardless of apparent support."*
/// A source and a strength are different axes, and one of them wearing the other's name is the
/// D-07 vocabulary mismatch.
///
/// **And it derived `Ord`, which made the mismatch load-bearing.** With `PartialOrd, Ord` on the
/// derive list, `Oracle < ReadBack < SelfReport` sorted **self-report as the greatest evidence** —
/// exactly backwards from the lineage this type came from, where `axes.py`'s first ladder records
/// that **`A` is strongest** and `ala-system` ranks `direct: 5` above `self_reported: 1`. So a
/// `min()`, a `max()` or a sort over grades would have produced a confidently wrong answer with
/// no refusal anywhere.
///
/// **That is the exact failure `refuse_interchange` exists to prevent**, and it was sitting in a
/// derive list rather than in a comparison, which is why nothing caught it. `axis.rs` rule 1 is
/// *"every ladder is its own type"*; the corollary this type needed was that an **origin is not a
/// ladder and must carry no order at all**. `PartialOrd`/`Ord` are therefore **not** derived: a
/// comparison of two origins is now a compile error rather than a silent inversion.
///
/// **The origin is on the record, which is the point:** evidence filed without one is filed as an
/// assertion, and the reader has no way to tell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EvidenceOrigin {
    /// **A** — an external oracle. Something outside this project decided it.
    Oracle,
    /// **B** — read back from world state. This project looked and measured.
    ReadBack,
    /// **C** — self-report. Somebody said so.
    ///
    /// **Present and named, not omitted.** The reference's rule from L-5 is that *agent
    /// self-report is evidence, not authority*, and the honest way to carry that is a value a
    /// ticket can hold and a reader can see — rather than a system where self-reported evidence
    /// has no representation and so is quietly filed as if it were an oracle's.
    SelfReport,
}

impl EvidenceOrigin {
    pub const ALL: [EvidenceOrigin; 3] = [
        EvidenceOrigin::Oracle,
        EvidenceOrigin::ReadBack,
        EvidenceOrigin::SelfReport,
    ];

    pub const COUNT: usize = 3;

    /// The single letter, as the reference writes it. **A letter, not a rank** — the letters
    /// happen to be alphabetical and nothing more, which is why the type carries no `Ord`.
    pub const fn letter(self) -> &'static str {
        match self {
            EvidenceOrigin::Oracle => "A",
            EvidenceOrigin::ReadBack => "B",
            EvidenceOrigin::SelfReport => "C",
        }
    }

    pub const fn describe(self) -> &'static str {
        match self {
            EvidenceOrigin::Oracle => "A — an external oracle decided this",
            EvidenceOrigin::ReadBack => "B — read back from world state",
            EvidenceOrigin::SelfReport => "C — self-report; evidence, but not authority",
        }
    }

    pub fn parse(word: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|origin| origin.letter() == word.trim())
            .ok_or_else(|| {
                format!(
                    "unknown evidence origin {word:?}; there are {}: A, B, C",
                    Self::COUNT
                )
            })
    }
}

/// **Check 4.** Every origin has a row.
pub const ORIGINS_ARE_TOTAL: [(); EvidenceOrigin::COUNT] = [(); EvidenceOrigin::ALL.len()];

/// What the world must show before this ticket may be called validated.
///
/// **Read back from the world, never inferred from the request that filed the ticket.** That
/// sentence is the reference's and it is the load-bearing rule of the whole surface: an
/// expectation inferred from the request is the request checking itself.
///
/// The variants are **this tree's**, not the city's. A city asserts roads, zones and
/// buildings; a voxel world asserts **voxels, materials and the modification record**, and
/// copying `RoadTiles` would be importing a domain rather than a shape. What is carried is the
/// *shape*: an expectation names world facts, it has a read-back, and it can be asked which
/// coordinates it touches.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expectation {
    /// These coordinates are solid, carrying this material.
    SolidAt { at: [[i32; 3]; 1], material: u16 },
    /// These coordinates are empty.
    EmptyAt { at: Vec<[i32; 3]> },
    /// The material at these coordinates is this one.
    MaterialAt { at: [i32; 3], material: u16 },
    /// The modification record holds an entry for this coordinate.
    ///
    /// **The strongest expectation here, and it is the one this tree can actually check**:
    /// `voxel_shapers::earthworks` states that *an unrecorded modification is invisible*, so
    /// "the record has an entry" is a fact about the record that a reader can verify.
    RecordedAt { at: [i32; 3] },
    /// The world digest is this.
    ///
    /// **Whole-world, and a digest rather than a spot check** — this project already computes
    /// one per frame (`Streamer::digest`), and comparing digests is a whole-world expectation
    /// that costs nothing to check and cannot be satisfied by one lucky voxel.
    WorldDigest(Digest),
    /// Nothing to read back.
    ///
    /// **A real variant, not a null.** A ticket with no world claim says so, which is
    /// different from a ticket whose claim was never checked — and only the second is a
    /// defect.
    None,
}

impl Expectation {
    /// The coordinates this expectation names.
    ///
    /// The reference's `Expectation::tiles()` exists to answer a stale claim's real question:
    /// *did a later recorded action explain this change, or did the world move with nothing on
    /// the record to account for it?* Those are different findings and only one is a defect,
    /// so a reader must be able to ask which coordinates are involved.
    pub fn coordinates(&self) -> Vec<[i32; 3]> {
        match self {
            Expectation::SolidAt { at, .. } => at.to_vec(),
            Expectation::EmptyAt { at } => at.clone(),
            Expectation::MaterialAt { at, .. } => vec![*at],
            Expectation::RecordedAt { at } => vec![*at],
            Expectation::WorldDigest(_) | Expectation::None => Vec::new(),
        }
    }

    /// The declared value, in one word. **The counterpart to [`Self::describe`], and the one a
    /// document prints.**
    ///
    /// `describe` is a sentence and sentences are prose: a renderer that prints one is
    /// summarising, and a summary is a place where the document and the tool can disagree
    /// without either being wrong. A slug is a *label for a declared variant* — the same
    /// word the type already uses — so quoting it adds nothing the record did not say.
    ///
    /// Exhaustive, and the same `slug` convention as [`ReadBack`] below, so a new variant is
    /// a compile error rather than a document that quietly under-reports.
    pub const fn slug(&self) -> &'static str {
        match self {
            Expectation::SolidAt { .. } => "solid-at",
            Expectation::EmptyAt { .. } => "empty-at",
            Expectation::MaterialAt { .. } => "material-at",
            Expectation::RecordedAt { .. } => "recorded-at",
            Expectation::WorldDigest(_) => "world-digest",
            Expectation::None => "none",
        }
    }

    /// What the world must show, in words.
    pub fn describe(&self) -> String {
        match self {
            Expectation::SolidAt { at, material } => {
                format!("material {material} is solid at {:?}", at[0])
            }
            Expectation::EmptyAt { at } => format!("{} coordinates are empty", at.len()),
            Expectation::MaterialAt { at, material } => {
                format!("material {material} is at {:?}", at)
            }
            Expectation::RecordedAt { at } => {
                format!("the modification record holds an entry at {at:?}")
            }
            Expectation::WorldDigest(digest) => {
                format!("the world digest is {}", &digest.to_hex()[..16])
            }
            Expectation::None => "no world expectation (nothing to read back)".to_string(),
        }
    }

    /// Whether there is anything to read back at all.
    pub const fn has_claim(&self) -> bool {
        !matches!(self, Expectation::None)
    }
}

/// What a read-back found. **The result, not a bool**, for the same reason
/// [`crate::chain::Linkage`] is not one: "not checked" and "checked and it did not hold" are
/// different facts and only one of them is a defect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadBack {
    /// Checked, and the world showed it.
    Held,
    /// Checked, and the world did not.
    NotHeld,
    /// **No read-back ran.** Never a pass. This is
    /// [`RefusalKind::ExpectationUnread`] as a value.
    NotChecked,
}

impl ReadBack {
    pub const fn slug(self) -> &'static str {
        match self {
            ReadBack::Held => "held",
            ReadBack::NotHeld => "not_held",
            ReadBack::NotChecked => "not_checked",
        }
    }

    /// Whether this result may license `completed_and_validated`.
    ///
    /// **Only [`ReadBack::Held`].** An unevaluated read-back must not be able to produce a
    /// validated retirement, because that is exactly how an unverified result acquires a
    /// verified-looking label.
    pub const fn validates(self) -> bool {
        matches!(self, ReadBack::Held)
    }
}

/// One piece of evidence, **with its origin on the record**.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Evidence {
    pub id: String,
    pub ticket: TicketId,
    /// Where it came from, **not how strong it is** — see [`EvidenceOrigin`], which is where
    /// the reasoning lives. The field is named for the question it answers.
    pub origin: EvidenceOrigin,
    /// What was observed, in one line.
    pub detail: String,
}

impl Evidence {
    /// Evidence, refusing an id or a detail that records nothing.
    pub fn new(
        id: impl Into<String>,
        ticket: TicketId,
        origin: EvidenceOrigin,
        detail: impl Into<String>,
    ) -> Result<Self, RefusalSet> {
        let id = id.into();
        let detail = detail.into();
        let mut refusals = RefusalSet::new();
        if id.trim().is_empty() {
            refusals.refuse(
                RefusalKind::MalformedIdentity,
                "evidence with no id cannot be cited, and uncitable evidence is an assertion",
            );
        }
        if detail.trim().is_empty() {
            refusals.refuse(
                RefusalKind::NoReading,
                format!("evidence {id:?} records nothing. An empty detail is a claim with no \
                         content, which is the thing an origin is supposed to qualify"),
            );
        }
        refusals.into_result(Evidence {
            id,
            ticket,
            origin,
            detail,
        })
    }
}

/// A ticket's gate, stated **up front** from the objective's own acceptance text.
///
/// The reference's field comment is the reason it exists at all: *"so nobody has to guess
/// afterwards what 'done' was going to mean."*
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Gate {
    /// What must be true for this ticket to be done.
    pub statement: String,
    /// What the world must show. See [`Expectation`].
    pub expectation: Expectation,
}

impl Gate {
    /// A gate, refusing an empty statement.
    pub fn new(statement: impl Into<String>, expectation: Expectation) -> Result<Self, RefusalSet> {
        let statement = statement.into();
        let mut refusals = RefusalSet::new();
        if statement.trim().is_empty() {
            refusals.refuse(
                RefusalKind::NoReading,
                "a gate that states nothing gates nothing. Stating the bar afterwards is how a \
                 ticket gets declared done by whoever moves last",
            );
        }
        refusals.into_result(Gate { statement, expectation })
    }
}

/// A ticket, **derived from the chain**.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ticket {
    pub id: TicketId,
    pub kind: TicketKind,
    pub status: TicketStatus,
    pub objective: String,
    pub gate: Gate,
    /// Who filed it.
    pub actor: String,
    /// The tick it was opened at, from the namespace.
    pub opened_tick: u64,
    /// The chain position it came from — **identity is a position in an append-only record.**
    pub event_index: u64,
    /// The digest of the event that filed it.
    pub digest: Digest,
    /// Evidence filed against it, grades included.
    pub evidence: Vec<Evidence>,
    /// The retirement, when there is one. **`None` while the ticket is live**, and its absence
    /// is what makes [`Board::retire`]'s check meaningful.
    pub retirement: Option<Retirement>,
    /// Set when a correction has been appended beside this record. **Never a mutation of the
    /// retirement** — immutability means *no silent edits*, and a correction is the opposite of
    /// silent.
    pub corrected: Option<String>,
}

impl Ticket {
    /// Whether this ticket is closed.
    pub fn is_closed(&self) -> bool {
        self.status.is_terminal()
    }

    /// Whether this ticket may be called validated.
    ///
    /// **Both** halves must hold: a retirement exists, and it says validated. A ticket with no
    /// retirement is unvalidated however complete it looks, which is the rule that stops a
    /// status flip from standing in for a retirement.
    pub fn is_validated(&self) -> bool {
        // `as_ref`, not `map`: `map` would move the retirement out of a shared reference.
        self.retirement
            .as_ref()
            .map(|retirement| retirement.reason.is_validated())
            .unwrap_or(false)
    }

    /// The closing line, as it reads on the ledger.
    pub fn closing_line(&self) -> String {
        match (&self.retirement, &self.corrected) {
            (Some(retirement), Some(correction)) => {
                format!("{} (corrected: {correction})", retirement.reason.name())
            }
            (Some(retirement), None) => retirement.reason.name().to_string(),
            // **Never a bare status where a retirement should be.** A ticket with no
            // retirement prints its status, and the absence of a reason is itself the signal.
            (None, _) => format!("{} (no retirement recorded)", self.status.name()),
        }
    }
}

/// The retirement record. **Separate from the ticket, on purpose.**
///
/// The reference's rule: a status flip is not a retirement. This type is the thing a terminal
/// status must be backed by, and it carries the grounding and the verification — the two
/// things a status name cannot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Retirement {
    pub ticket: TicketId,
    pub reason: RetirementReason,
    /// What the ticket claimed to deliver.
    pub completion_claim: String,
    /// Whether the inputs were verified, and the note saying so.
    pub grounding: Grounding,
    /// What was checked, and whether it passed.
    pub verification: Verification,
    /// The read-back result that produced this retirement.
    pub read_back: ReadBack,
}

/// Whether the ticket's inputs were themselves verified.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grounding {
    pub inputs_verified: bool,
    pub note: String,
}

/// What was checked, and what happened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Verification {
    pub acceptance_criteria: Vec<String>,
    pub checks_run: Vec<String>,
    pub passed: bool,
    /// **Whether previously working behaviour broke.** Carried on every retirement because
    /// the four-way classification depends on it and a record that omits it forces the reader
    /// to guess.
    pub regressions_detected: bool,
}

impl Retirement {
    /// A retirement, refusing a claim that records nothing.
    ///
    /// **The coherence checks are the point.** Claiming `completed_and_validated` with a
    /// read-back that did not happen, or that did not hold, is refused here — at
    /// construction — rather than discovered later by a reader who trusted the token.
    pub fn new(
        ticket: TicketId,
        reason: RetirementReason,
        completion_claim: impl Into<String>,
        grounding: Grounding,
        verification: Verification,
        read_back: ReadBack,
    ) -> Result<Self, RefusalSet> {
        let completion_claim = completion_claim.into();
        let mut refusals = RefusalSet::new();

        if completion_claim.trim().is_empty() {
            refusals.refuse(
                RefusalKind::NoReading,
                "a retirement with no completion claim records that work stopped, and not what \
                 was done. That is an abandonment wearing a completion's clothes",
            );
        }

        if reason.is_validated() && !read_back.validates() {
            refusals.refuse(
                RefusalKind::ExpectationUnread,
                format!(
                    "{} claims the world was checked and it was not (read-back: {}). A validated \
                     retirement is the one token that means 'done AND checked', so it cannot be \
                     issued on an unchecked or failed read-back",
                    reason.name(),
                    read_back.slug()
                ),
            );
        }

        if verification.regressions_detected && reason == RetirementReason::CompletedAndValidated {
            refusals.refuse(
                RefusalKind::UnsubstantiatedEvidence,
                "a retirement claims `completed_and_validated` while recording a detected \
                 regression. Those are contradictory; the honest token is \
                 `completed_with_known_regression`",
            );
        }

        refusals.into_result(Retirement {
            ticket,
            reason,
            completion_claim,
            grounding,
            verification,
            read_back,
        })
    }

    /// The four-way classification, from the wayfinder reference L-1 records.
    ///
    /// `gain` when it was broken and now works, `regression` when it worked and now does not,
    /// `residual_failure` when it did not work and still does not, `retained` when it worked
    /// and still does. **The before-state is an argument, not a field**, because a record that
    /// carried only an after-state could classify itself as a gain by definition.
    pub fn classify(&self, worked_before: bool, works_now: bool) -> Outcome {
        match (worked_before, works_now) {
            (false, true) => Outcome::Gain,
            (true, false) => Outcome::Regression,
            (false, false) => Outcome::ResidualFailure,
            (true, true) => Outcome::Retained,
        }
    }
}

/// **The four-way outcome**, from `wayfinder-task-retirement` via L-1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Gain,
    Regression,
    ResidualFailure,
    Retained,
}

impl Outcome {
    pub const ALL: [Outcome; 4] = [
        Outcome::Gain,
        Outcome::Regression,
        Outcome::ResidualFailure,
        Outcome::Retained,
    ];

    pub const COUNT: usize = 4;

    pub const fn name(self) -> &'static str {
        match self {
            Outcome::Gain => "gain",
            Outcome::Regression => "regression",
            Outcome::ResidualFailure => "residual_failure",
            Outcome::Retained => "retained",
        }
    }
}

/// **Check 5.** Every outcome has a row.
pub const OUTCOMES_ARE_TOTAL: [(); Outcome::COUNT] = [(); Outcome::ALL.len()];

/// Split a leading `[KIND]` marker off a chain event's detail, yielding the ticket's kind and
/// the text that is actually the ticket.
///
/// **The marker is the convention, and it is stated rather than inferred.** A chain event
/// reads `[BLD] mount window[console]`, and the bracket is how the record says *which kind of
/// thing happened* — the same sentence cannot be a case, an intervention and a material at
/// once.
///
/// **An event with no marker is a [`TicketKind::Build`].** That is a fallback and it is named
/// as one: the alternative is refusing every marker-less event, which would make the read path
/// refuse the ordinary case in exchange for a distinction the writer did not make. A build is
/// what an in-game request for a window *is*, so the fallback is the right reading rather than
/// a convenient one — and the marker is stripped from the objective either way, so the ticket's
/// text is never the chain's bookkeeping.
/// One objective filed more than once, at the positions that prove it.
///
/// **Derived after the fold, not tracked during it.** The fold's merge keys on `id`, and `id`
/// is derived from the chain position — so a duplicate at a *different* position is a
/// different ticket by the fold's own rule, and the only place the collision is visible is
/// after the fact, on the objective itself. Doing it here rather than inline also means the
/// check cannot be bypassed by a second fold path.
fn find_duplicates_of(tickets: &[Ticket]) -> Vec<Duplicate> {
    let mut first_seen: std::collections::BTreeMap<&str, u64> = std::collections::BTreeMap::new();
    let mut found: std::collections::BTreeMap<String, Duplicate> = std::collections::BTreeMap::new();
    for ticket in tickets {
        // **The objective as the board holds it**, which is the marker-stripped text — so a
        // filing that adds or removes a `[BLD]` prefix is still the same work. The earlier
        // one-shot guard compared the *prefixed* constant against this stripped text and so
        // could never match; that is the bug this function exists to make visible.
        match first_seen.get(ticket.objective.as_str()) {
            Some(first) => {
                let entry = found.entry(ticket.objective.clone()).or_insert_with(|| Duplicate {
                    objective: ticket.objective.clone(),
                    first_index: *first,
                    repeated_at: Vec::new(),
                });
                entry.repeated_at.push(ticket.event_index);
            }
            None => {
                first_seen.insert(ticket.objective.as_str(), ticket.event_index);
            }
        }
    }
    found.into_values().collect()
}

fn split_kind(detail: &str) -> (TicketKind, String) {
    if let Some(rest) = detail.strip_prefix('[') {
        if let Some(close) = rest.find(']') {
            if let Some(kind) = TicketKind::from_prefix(&rest[..close]) {
                return (kind, rest[close + 1..].trim_start().to_string());
            }
        }
    }
    (TicketKind::Build, detail.to_string())
}

/// The board: **the tickets, folded from the chain.** A view, never a store.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Board {
    tickets: Vec<Ticket>,
    /// Filings of the **same objective at more than one chain position**, with both positions.
    ///
    /// # Why this exists, and the absence that let it happen
    ///
    /// The fold below merges on `id`, which is `{prefix}-{index}`. Two filings of one objective
    /// at different positions therefore get different ids and **both are pushed** — so there
    /// was no duplicate check anywhere: not on the objective, not on the event digest, not on
    /// anything.
    ///
    /// **That absence is not hypothetical.** This repository's own record was filed with its
    /// 42 work tickets **three times** in one session, because a one-shot guard compared a
    /// marker-prefixed objective against the marker-stripped text the board actually holds and
    /// so could never match. The chain held 184 events, the board read **128** tickets where
    /// 42 was the truth, and **every test stayed green** — because the reader cannot see this
    /// class of defect in its own input.
    ///
    /// The reference answers the same subject by refusing: `patch_store::load_and_verify`
    /// returns *"duplicate base-chain digest"* when a digest repeats. **Refusing here would
    /// make the whole board unreadable on a chain that genuinely holds duplicates**, and a
    /// reader that cannot be read is not a safer reader. So this is
    /// [`crate::chain::Linkage::NotEvaluable`]'s shape: **measured, reported, and never
    /// silently dropped.** Both tickets stay on the board, because hiding one would be a
    /// second record rather than a clean one.
    duplicates: Vec<Duplicate>,
}

/// One objective filed more than once, with the positions that prove it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Duplicate {
    /// The objective, exactly as the board holds it.
    pub objective: String,
    /// The first chain position it was filed at.
    pub first_index: u64,
    /// Every later position it was filed at, in chain order.
    pub repeated_at: Vec<u64>,
}

impl Board {
    /// **The only constructor.** There is deliberately no `Board::new`.
    ///
    /// The board is a projection of the chain (H-8.1), so building one without a chain would
    /// create a second source of truth — the exact failure the ruling exists to prevent. A
    /// caller that wants a board must therefore hand over the record it is a view of.
    pub fn from_chain(chain: &Chain) -> Self {
        // The fold **accumulates owned summaries**, not borrowed `&Event`s. Collecting
        // references out of the fold would not compile — the borrow cannot escape the closure
        // — and it would be wrong anyway: this is a projection, so it should carry the
        // fields it needs and nothing else.
        #[derive(Clone)]
        struct Filed {
            index: u64,
            world: String,
            detail: String,
            digest: Digest,
            kind_is_retirement: bool,
        }

        let filed = chain.fold(Vec::new(), |mut acc, event| {
            if event.kind != EventKind::TicketTransition && event.kind != EventKind::Retirement {
                return acc;
            }
            acc.push(Filed {
                index: event.index,
                world: event.namespace.world.clone(),
                detail: event.detail.clone(),
                digest: event.digest,
                kind_is_retirement: event.kind == EventKind::Retirement,
            });
            acc
        });

        let mut tickets: Vec<Ticket> = Vec::new();
        for event in filed {
            // Only retirement events carry a retirement; a filed ticket is derived from the
            // event that filed it, and the chain event is the authority for both.
            //
            // **The id is in the ticket vocabulary, not a chain-shaped string.** It used to be
            // `{index}-{campaign}`, which is a position and a campaign glued together — so a
            // ticket filed in-game could never be addressed by the id its own kind gives it,
            // and `BLD-0000` and `0-c-2026-09-30` were two names for one ticket. The id is now
            // `{kind prefix}-{position}`, which is the reference's shape and puts identity
            // where the record says identity lives: **a position in an append-only record.**
            let (kind, objective) = split_kind(&event.detail);
            let id = match TicketId::new(format!("{}-{:04}", kind.prefix(), event.index)) {
                Ok(id) => id,
                Err(_) => continue,
            };
            match tickets.iter_mut().find(|ticket| ticket.id == id) {
                Some(existing) => {
                    if event.kind_is_retirement {
                        // The chain records that a retirement happened; the retirement's own
                        // contents come from the event's detail, and a chain that cannot
                        // supply them leaves the ticket closed-but-unretired — which
                        // `Board::retire` will then refuse to call validated.
                        existing.status = TicketStatus::Complete;
                    }
                }
                None => tickets.push(Ticket {
                    id,
                    kind,
                    status: TicketStatus::Queued,
                    objective: objective.clone(),
                    gate: Gate {
                        statement: objective,
                        expectation: Expectation::None,
                    },
                    actor: event.world.clone(),
                    opened_tick: 0,
                    event_index: event.index,
                    digest: event.digest,
                    evidence: Vec::new(),
                    retirement: None,
                    corrected: None,
                }),
            }
        }
        // **Duplicates derived before the move**, so the borrow ends before the board is built.
        let duplicates = find_duplicates_of(&tickets);
        Board { tickets, duplicates }
    }

    /// Filings of one objective at more than one position. **Counted, not hidden.**
    pub fn duplicates(&self) -> &[Duplicate] {
        &self.duplicates
    }

    /// The report the gate prints.
    ///
    /// **A bare ticket count is the absence-as-zero defect D107 names** — 128 is a perfectly
    /// good number for a board holding 42 real tickets and 84 copies of them, and it reads
    /// exactly like 128 real tickets. So the count is printed *with* the duplication, and the
    /// consequence is stated every time rather than left for a reader to infer.
    pub fn describe(&self) -> String {
        let mut out = format!("board-tickets\t{}\nboard-duplicate-filings\t{}\n", self.tickets.len(), self.duplicates.len());
        if self.duplicates.is_empty() {
            out.push_str("board\tevery ticket on this board was filed once\n");
        } else {
            let repeated: usize = self.duplicates.iter().map(|d| d.repeated_at.len()).sum();
            out.push_str(&format!(
                "board\tthe board holds {} tickets, of which {repeated} are repeat filings of {} \
                 distinct objectives. An append-only record cannot retract a filing, so these are \
                 reported rather than removed, and the ticket count above is NOT a count of \
                 distinct pieces of work\n",
                self.tickets.len(),
                self.duplicates.len()
            ));
            // **Every duplicate, in full.** The first version printed five and a count of the
            // rest, and truncated each objective to 72 characters — a reader who saw
            // `board-duplicates-more 37` still could not tell *which* objectives were filed
            // three times without going back to the code. That is the same defect as the
            // session's own: a reader that reports a sample of a defect it can see all of.
            // Forty-two lines on one build is the measurement; truncating it is the bloat.
            for duplicate in &self.duplicates {
                out.push_str(&format!(
                    "board-duplicate\t{} and {}\t{}\n",
                    duplicate.first_index,
                    duplicate.repeated_at
                        .iter()
                        .map(u64::to_string)
                        .collect::<Vec<_>>()
                        .join(", "),
                    duplicate.objective
                ));
            }
            // **The singly-filed tickets are named too, and this is the same defect one level
            // over.** The loop above exists because *a reader who cannot see all of a defect
            // cannot see it* — and then the report named every duplicate and **no** ticket that
            // was filed once. On this board that is four objectives existing only as an
            // arithmetic remainder (`130 - 126`), which is the absence-as-zero rule
            // (D107) wearing the report's own clothes: the work that is *not* duplicated is
            // precisely the work a reader most needs to read.
            //
            // So the accounting is closed rather than sampled: **every** position on the board
            // appears in exactly one of the two lists, and the test that holds it walks the
            // board rather than counting the lines.
            let mut repeated_positions: std::collections::BTreeSet<u64> =
                std::collections::BTreeSet::new();
            for duplicate in &self.duplicates {
                repeated_positions.insert(duplicate.first_index);
                repeated_positions.extend(duplicate.repeated_at.iter().copied());
            }
            for ticket in &self.tickets {
                if repeated_positions.contains(&ticket.event_index) {
                    continue;
                }
                out.push_str(&format!(
                    "board-ticket\t{}\t{}\n",
                    ticket.event_index, ticket.objective
                ));
            }
        }
        out
    }

    pub fn len(&self) -> usize {
        self.tickets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tickets.is_empty()
    }

    /// Every ticket, in chain order.
    pub fn tickets(&self) -> &[Ticket] {
        &self.tickets
    }

    /// One ticket, or `None` — which a caller must handle, because a ticket that was not
    /// found is not a ticket that passed.
    pub fn ticket(&self, id: &TicketId) -> Option<&Ticket> {
        self.tickets.iter().find(|ticket| ticket.id == *id)
    }

    /// The tickets that have not reached a terminal state.
    pub fn open(&self) -> Vec<&Ticket> {
        self.tickets
            .iter()
            .filter(|ticket| !ticket.is_closed())
            .collect()
    }

    /// Whether the board has drained: every ticket terminal.
    ///
    /// The reference's requirement, and the reason `Blocked` counts as terminal — a board
    /// where a blocked ticket is still open can never drain, and a board that cannot drain is
    /// a board that accumulates.
    pub fn has_drained(&self) -> bool {
        self.tickets.iter().all(|ticket| ticket.is_closed())
    }
}

/// File a ticket onto a chain, **routed through the policy chokepoint**.
///
/// The signature takes the decision rather than a `Grant`, because this crate must not depend
/// on `voxel_package` (that would invert the dependency: the chokepoint is *above* the
/// record). The caller obtains the decision from `voxel_package::policy::decide` and passes
/// it here, so **the permission check happens at one place** and this function only records
/// what it was told.
///
/// `allowed` is therefore not this module's opinion. The alternative — a `Grant` parameter —
/// would have meant duplicating the chokepoint here, and L-5's whole point is *"one matrix,
/// one decision function, one audit sink … so that the one call site nobody remembered cannot
/// be the one that fails."*
pub fn file(
    chain: &mut Chain,
    namespace: Namespace,
    kind: TicketKind,
    objective: impl Into<String>,
    gate: Gate,
    allowed: bool,
    denial_reason: &str,
) -> Result<TicketId, RefusalSet> {
    let objective = objective.into();
    let mut refusals = RefusalSet::new();

    if !allowed {
        // **The refusal is on the record.** L-5: every denial writes an audit event, and a
        // refusal nobody can read is a refusal that did not happen.
        refusals.refuse(
            RefusalKind::UntrustedKey,
            format!("filing a ticket was refused by the policy chokepoint: {denial_reason}"),
        );
        if let Err(found) = chain.append(
            namespace,
            EventKind::RefusalRecorded,
            format!("ticket refused: {denial_reason}"),
        ) {
            refusals.absorb(found);
        }
        return Err(refusals);
    }

    if objective.trim().is_empty() {
        refusals.refuse(
            RefusalKind::NoReading,
            "a ticket with no objective records nothing to be done",
        );
    }
    if gate.statement.trim().is_empty() {
        refusals.refuse(
            RefusalKind::NoReading,
            "a ticket with no gate states no bar, so 'done' is whatever the last reader says",
        );
    }
    refusals.into_result(())?;

    let event = chain
        .append(
            namespace,
            EventKind::TicketTransition,
            format!("[{}] {objective}", kind.prefix()),
        )
        .map_err(|found| {
            let mut set = RefusalSet::new();
            set.absorb(found);
            set
        })?
        .clone();

    TicketId::new(format!("{}-{:04}", kind.prefix(), event.index)).map_err(|found| {
        let mut set = RefusalSet::new();
        set.absorb(found);
        set
    })
}

/// Retire a ticket: **the status flip and the retirement record are the same act.**
///
/// Refuses a terminal status with no [`Retirement`] behind it
/// ([`RefusalKind::UnretiredTicket`]) — the rule that stops a status flip standing in for a
/// retirement. And refuses `completed_and_validated` on anything but a
/// [`ReadBack::Held`], which [`Retirement::new`] also checks at construction; both are here so
/// that a caller bypassing the constructor still cannot manufacture one.
pub fn retire(ticket: &mut Ticket, retirement: Retirement) -> Result<(), RefusalSet> {
    let mut refusals = RefusalSet::new();

    if retirement.ticket != ticket.id {
        refusals.refuse(
            RefusalKind::MalformedIdentity,
            format!(
                "a retirement for {} was filed against {}: the record says one thing and the \
                 retirement another",
                retirement.ticket.as_str(),
                ticket.id.as_str()
            ),
        );
    }

    if !retirement.read_back.validates() && retirement.reason.is_validated() {
        refusals.refuse(
            RefusalKind::ExpectationUnread,
            format!(
                "{} cannot be validated on a read-back of {}",
                retirement.reason.name(),
                retirement.read_back.slug()
            ),
        );
    }

    if ticket.is_closed() {
        refusals.refuse(
            RefusalKind::UngovernedStatus,
            format!(
                "{} is already closed as {}. A closed ticket is not reopened by a second \
                 retirement; it is superseded by a new ticket that references it",
                ticket.id.as_str(),
                ticket.status.name()
            ),
        );
    }

    if !refusals.is_empty() {
        return Err(refusals);
    }

    ticket.status = match retirement.reason {
        RetirementReason::CompletedAndValidated
        | RetirementReason::CompletedWithKnownRegression
        | RetirementReason::CompletedButUnverified => TicketStatus::Complete,
        _ => TicketStatus::Handled,
    };
    ticket.retirement = Some(retirement);
    Ok(())
}

/// File a correction beside a ticket. **Append-only, and never a mutation of the retirement.**
///
/// The reference's rule, kept whole: a ticket carries `corrected: Option<String>` and the
/// retirement is never rewritten. A correction that could edit the retirement would make the
/// retirement unfalsifiable.
pub fn correct(ticket: &mut Ticket, correction: impl Into<String>, authorized_by: &str) -> Result<(), RefusalSet> {
    let correction = correction.into();
    let mut refusals = RefusalSet::new();
    if correction.trim().is_empty() {
        refusals.refuse(
            RefusalKind::NoReading,
            "a correction that says nothing corrects nothing",
        );
    }
    if authorized_by.trim().is_empty() {
        refusals.refuse(
            RefusalKind::MalformedIdentity,
            "an unauthorized correction is not a correction. Immutability means no *silent* \
             edits, and 'silent' is the part that needs an author",
        );
    }
    refusals.into_result(())?;
    ticket.corrected = Some(format!("{correction} (by {authorized_by})"));
    Ok(())
}

/// The record's own report of what it holds, for a window that shows it.
///
/// **Derived, never stored**, and it says so on its face — a summary that could disagree with
/// the record is the thing H-8.1 rules out.
pub fn report(board: &Board, chain: &Chain) -> String {
    let mut out = String::new();
    out.push_str("tickets / governance — a view over the one chain\n");
    out.push_str(&format!(
        "chain digest {}\nchain linkage {}\n",
        &chain.digest().to_hex()[..16],
        chain.tamper_evident_description()
    ));
    out.push_str(&format!(
        "tickets {}  open {}  drained {}\n",
        board.len(),
        board.open().len(),
        board.has_drained()
    ));
    for ticket in board.tickets() {
        out.push_str(&format!(
            "  {} {} {} — {}\n",
            ticket.id.as_str(),
            ticket.status.name(),
            ticket.kind.prefix(),
            ticket.objective
        ));
        out.push_str(&format!("      {}\n", ticket.closing_line()));
        if !ticket.evidence.is_empty() {
            for evidence in &ticket.evidence {
                out.push_str(&format!(
                    "      evidence {} origin {} — {}\n",
                    evidence.id, evidence.origin.letter(), evidence.detail
                ));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::Season;

    /// **The pin, and it is the assertion the old reader could not make.**
    ///
    /// Before this change the fold merged on `id` — `{prefix}-{index}` — and had no duplicate
    /// check anywhere, so filing one objective twice at two positions produced two tickets and
    /// **nothing could see it**. That is how 84 duplicates came to sit on this repository's own
    /// record with every test green. The test has no form against the old code because there
    /// was no quantity to assert; the absence *was* the defect.
    #[test]
    fn one_objective_filed_twice_is_reported_with_both_positions() {
        // **The same objective three times at three positions**, so the ids differ and the
        // fold's merge cannot catch them. This is exactly what the one-shot did.
        let mut chain = Chain::linked();
        for _ in 0..3 {
            chain
                .append(
                    namespace(),
                    EventKind::TicketTransition,
                    "[BLD] resize a pane: drag a divider, move the stretch factors",
                )
                .expect("appends");
        }
        let board = Board::from_chain(&chain);

        // **Both filings stay.** Dropping one would be a second record — the board would then
        // disagree with the chain about what was filed. Reported, never hidden.
        assert_eq!(board.len(), 3, "the board holds exactly what the chain holds");
        assert_eq!(board.duplicates().len(), 1, "one objective filed more than once");
        let duplicate = &board.duplicates()[0];
        assert_eq!(duplicate.first_index, 0);
        assert_eq!(duplicate.repeated_at, vec![1, 2], "and both later positions are named");

        // **And the report says the ticket count is not a count of work.** This is the half
        // that would have caught the session's mistake: a bare `3` reads exactly like three
        // tickets of real work.
        let report = board.describe();
        assert!(report.contains("board-duplicate-filings\t1"), "{report}");
        assert!(report.contains("is NOT a count of"), "{report}");
    }

    #[test]
    fn an_unduplicated_board_says_so_rather_than_printing_nothing() {
        // **Absence rendered as absence.** Zero duplicates is a measurement and is stated; a
        // silent empty line would be indistinguishable from a check that did not run.
        let mut chain = Chain::linked();
        for text in ["[BLD] the first thing", "[CSE] a different thing"] {
            chain
                .append(namespace(), EventKind::TicketTransition, text)
                .expect("appends");
        }
        let board = Board::from_chain(&chain);
        assert!(board.duplicates().is_empty(), "two distinct objectives");
        assert!(
            board.describe().contains("every ticket on this board was filed once"),
            "{}",
            board.describe()
        );
    }

    fn namespace() -> Namespace {
        Namespace::new("world-2468", 0, Season::Spring, "C-66").expect("a named namespace")
    }

    fn gate() -> Gate {
        Gate::new("the world shows the dig", Expectation::RecordedAt { at: [1, 2, 3] })
            .expect("a stated gate")
    }

    // ---- the id join, which was wrong and is now pinned ------------------

    #[test]
    fn a_ticket_id_is_its_kind_prefix_and_its_position() {
        // **This is the join the in-game loop needs.** A ticket filed in-game must be
        // addressable by the id its own kind gives it, or `emit` can never find it: the board
        // used to mint `{index}-{campaign}` — `0-C-66` — while the taxonomy says `BLD-0000`.
        // Two names for one ticket is the vocabulary mismatch D-07 names.
        let mut chain = Chain::new();
        chain
            .append(
                namespace(),
                EventKind::TicketTransition,
                "[BLD] mount window[console]",
            )
            .expect("filed");
        chain
            .append(
                namespace(),
                EventKind::TicketTransition,
                "[CSE] the streamer stalls at the horizon",
            )
            .expect("filed");

        let board = Board::from_chain(&chain);
        assert_eq!(board.len(), 2);
        assert_eq!(board.tickets()[0].id.as_str(), "BLD-0000");
        assert_eq!(board.tickets()[1].id.as_str(), "CSE-0001");
        assert_eq!(board.tickets()[0].kind, TicketKind::Build);
        assert_eq!(board.tickets()[1].kind, TicketKind::Case);
    }

    #[test]
    fn the_bracket_marker_is_the_chains_bookkeeping_and_never_the_tickets_text() {
        // The marker says which kind of thing happened; the text after it is the ticket. A
        // player reading their own ticket must not see the chain's bracket.
        let mut chain = Chain::new();
        chain
            .append(
                namespace(),
                EventKind::TicketTransition,
                "[INT] add a window named refusals",
            )
            .expect("filed");
        let board = Board::from_chain(&chain);
        assert_eq!(board.tickets()[0].objective, "add a window named refusals");
        assert_eq!(board.tickets()[0].id.as_str(), "INT-0000");
    }

    #[test]
    fn an_event_with_no_marker_is_a_build_and_the_fallback_is_visible() {
        // **A named fallback, not a silent one.** Refusing every marker-less event would make
        // the read path refuse the ordinary case; a build is what an in-game request for a
        // window *is*. What matters is that the rule is stated and therefore arguable.
        let (kind, objective) = split_kind("add a window named refusals");
        assert_eq!(kind, TicketKind::Build);
        assert_eq!(objective, "add a window named refusals");

        // And an unrecognised marker is *not* a kind: it is left in the text, so a reader can
        // see what was actually written rather than a fallback standing in for it.
        let (kind, objective) = split_kind("[XYZ] something");
        assert_eq!(kind, TicketKind::Build);
        assert_eq!(objective, "[XYZ] something");
    }

    fn retirement(reason: RetirementReason, read_back: ReadBack) -> Retirement {
        Retirement::new(
            TicketId::new("BLD-0001").unwrap(),
            reason,
            "dug the recorded voxel",
            Grounding { inputs_verified: true, note: "seed and tick read".into() },
            Verification {
                acceptance_criteria: vec!["the record holds an entry".into()],
                checks_run: vec!["earthworks.is_recorded".into()],
                passed: true,
                regressions_detected: false,
            },
            read_back,
        )
        .expect("a coherent retirement")
    }

    fn ticket() -> Ticket {
        Ticket {
            id: TicketId::new("BLD-0001").unwrap(),
            kind: TicketKind::Build,
            status: TicketStatus::Queued,
            objective: "dig the recorded voxel".into(),
            gate: gate(),
            actor: "user".into(),
            opened_tick: 0,
            event_index: 0,
            digest: crate::identity::Digest::of(b"event"),
            evidence: Vec::new(),
            retirement: None,
            corrected: None,
        }
    }

    // ---- the ported vocabularies -----------------------------------------

    #[test]
    fn the_governed_status_set_is_nine_and_nothing_else_parses() {
        let _ = STATUSES_ARE_TOTAL;
        assert_eq!(TicketStatus::ALL.len(), TicketStatus::COUNT);
        assert_eq!(TicketStatus::COUNT, 9);
        for status in TicketStatus::ALL {
            assert_eq!(TicketStatus::parse(status.name()), Ok(status));
        }
        let error = TicketStatus::parse("working").expect_err("not governed here");
        assert!(error.contains("9"), "the refusal quotes the set: {error}");
    }

    #[test]
    fn the_retirement_taxonomy_is_fourteen_and_only_one_validates() {
        let _ = REASONS_ARE_TOTAL;
        assert_eq!(RetirementReason::ALL.len(), RetirementReason::COUNT);
        assert_eq!(RetirementReason::COUNT, 14);
        let validating: Vec<&str> = RetirementReason::ALL
            .iter()
            .filter(|reason| reason.is_validated())
            .map(|reason| reason.name())
            .collect();
        assert_eq!(
            validating,
            vec!["completed_and_validated"],
            "exactly one token means done AND checked"
        );
        // And the three "completed" tokens are distinguishable.
        let completed: Vec<&str> = RetirementReason::ALL
            .iter()
            .filter(|reason| reason.is_completion())
            .map(|reason| reason.name())
            .collect();
        assert_eq!(completed.len(), 3, "three ways to finish, one of them validated");
    }

    #[test]
    fn every_kind_prefix_and_origin_round_trips() {
        let _ = KINDS_ARE_TOTAL;
        let _ = ORIGINS_ARE_TOTAL;
        assert_eq!(TicketKind::ALL.len(), 4);
        assert_eq!(EvidenceOrigin::ALL.len(), 3);
        for kind in TicketKind::ALL {
            assert_eq!(TicketKind::parse(kind.name()), Ok(kind));
            assert!(!kind.prefix().is_empty());
        }
        for origin in EvidenceOrigin::ALL {
            assert_eq!(EvidenceOrigin::parse(origin.letter()), Ok(origin));
        }
        // Self-report is named, not omitted: evidence, not authority.
        assert!(EvidenceOrigin::SelfReport.describe().contains("not authority"));
    }

    #[test]
    fn an_origin_is_not_a_grade_and_carries_no_order() {
        // **The defect this rename fixed, pinned so it cannot come back.**
        //
        // `EvidenceGrade` derived `PartialOrd, Ord`, so `Oracle < ReadBack < SelfReport` made a
        // self-report the *greatest* evidence — backwards from the lineage it came from, where
        // `axes.py`'s first ladder records that **A is strongest** and `ala-system` ranks
        // `direct: 5` over `self_reported: 1`. A `max()` over grades was a confident wrong answer
        // with no refusal, which is what `refuse_interchange` exists to prevent.
        //
        // **A test cannot assert the absence of a trait**, so the mechanism is the derive list on
        // the enum and this test holds the *consequence* that matters: the three values are
        // distinct sources, they are not a ranking, and the letters are labels. If someone
        // re-adds `Ord`, the comparisons this type is forbidden from making become possible again
        // and the doc comment on the enum is the only thing warning them — so the warning is
        // here too, where a reader editing the test will see it.
        let all = EvidenceOrigin::ALL;
        assert_eq!(all.len(), 3, "three sources, not a scale");
        // Distinct by construction, and the letters are not an ordering claim: they happen to be
        // alphabetical, which is the whole reason the type must carry no `Ord`.
        let letters: Vec<&str> = all.iter().map(|origin| origin.letter()).collect();
        assert_eq!(letters, vec!["A", "B", "C"]);
        // Each describes where it came from. None of them claims to be stronger or weaker than
        // another, which is the property the old name implied and the derive list enforced.
        for origin in all {
            let text = origin.describe();
            assert!(
                !text.to_lowercase().contains("stronger")
                    && !text.to_lowercase().contains("weaker"),
                "{text:?} must not claim a strength, because an origin is not a grade"
            );
        }
    }

    #[test]
    fn a_blocked_ticket_is_terminal_so_the_board_can_drain() {
        // The drain rule, as a property of the type.
        assert!(TicketStatus::Blocked.is_terminal());
        assert!(TicketStatus::Complete.is_terminal());
        assert!(!TicketStatus::Active.is_terminal());
        assert!(!TicketStatus::Review.is_terminal());
    }

    // ---- retirement is more than a status --------------------------------

    #[test]
    fn a_validated_retirement_cannot_be_issued_on_an_unchecked_read_back() {
        // The check that matters most, and the reason `ReadBack` is not a bool.
        for read_back in [ReadBack::NotChecked, ReadBack::NotHeld] {
            let error = Retirement::new(
                TicketId::new("BLD-0001").unwrap(),
                RetirementReason::CompletedAndValidated,
                "dug it",
                Grounding { inputs_verified: true, note: "n".into() },
                Verification {
                    acceptance_criteria: vec![],
                    checks_run: vec![],
                    passed: true,
                    regressions_detected: false,
                },
                read_back,
            )
            .expect_err("validated means checked");
            assert_eq!(error.all()[0].kind(), RefusalKind::ExpectationUnread);
        }
        // With a held read-back it is fine.
        retirement(RetirementReason::CompletedAndValidated, ReadBack::Held);
    }

    #[test]
    fn a_ticket_cannot_be_called_validated_without_a_retirement() {
        let mut t = ticket();
        assert!(!t.is_validated(), "a live ticket is not validated");

        // **A status flip alone must not validate.** Setting the field by hand is exactly the
        // act the rule forbids, so this is the state under test: `complete` with no
        // retirement behind it.
        //
        // The first draft of this test then called `retire` on that hand-closed ticket and
        // expected success. The compiler was fine and the test failed, because
        // [`retire`] correctly **refuses** a ticket that is already closed — a closed ticket
        // is superseded by a new one, not reopened. The guard was right and the test was
        // wrong, so the second half now retires a *live* ticket.
        t.status = TicketStatus::Complete;
        assert!(
            !t.is_validated(),
            "a status flip is not a retirement: `complete` alone must not validate"
        );
        assert!(
            t.closing_line().contains("no retirement recorded"),
            "and the closing line must say so rather than print a bare status: {}",
            t.closing_line()
        );

        // Retiring a **live** ticket is the path that works.
        let mut live = ticket();
        retire(
            &mut live,
            retirement(RetirementReason::CompletedAndValidated, ReadBack::Held),
        )
        .expect("a proper retirement on a live ticket");
        assert!(live.is_validated());
        assert_eq!(live.status, TicketStatus::Complete);
    }

    #[test]
    fn a_retirement_with_a_detected_regression_cannot_claim_validated() {
        let error = Retirement::new(
            TicketId::new("BLD-0001").unwrap(),
            RetirementReason::CompletedAndValidated,
            "dug it",
            Grounding { inputs_verified: true, note: "n".into() },
            Verification {
                acceptance_criteria: vec![],
                checks_run: vec![],
                passed: true,
                regressions_detected: true,
            },
            ReadBack::Held,
        )
        .expect_err("a regression contradicts 'validated'");
        assert_eq!(error.all()[0].kind(), RefusalKind::UnsubstantiatedEvidence);
        assert!(
            error.all()[0].detail().contains("completed_with_known_regression"),
            "the refusal names the honest token: {error}"
        );
    }

    #[test]
    fn a_closed_ticket_is_not_reopened_by_a_second_retirement() {
        let mut t = ticket();
        retire(&mut t, retirement(RetirementReason::CompletedAndValidated, ReadBack::Held)).unwrap();
        let error = retire(
            &mut t,
            retirement(RetirementReason::CompletedAndValidated, ReadBack::Held),
        )
        .expect_err("a closed ticket stays closed");
        assert_eq!(error.all()[0].kind(), RefusalKind::UngovernedStatus);
    }

    #[test]
    fn a_retirement_for_the_wrong_ticket_is_refused() {
        let mut t = ticket();
        let mut wrong = retirement(RetirementReason::CompletedAndValidated, ReadBack::Held);
        wrong.ticket = TicketId::new("BLD-9999").unwrap();
        let error = retire(&mut t, wrong).expect_err("the ids must agree");
        assert_eq!(error.all()[0].kind(), RefusalKind::MalformedIdentity);
    }

    #[test]
    fn the_four_way_classification_needs_the_before_state_as_an_argument() {
        // A record carrying only an after-state could classify itself as a gain by
        // definition, so the before-state is deliberately not a field.
        let r = retirement(RetirementReason::CompletedAndValidated, ReadBack::Held);
        assert_eq!(r.classify(false, true), Outcome::Gain);
        assert_eq!(r.classify(true, false), Outcome::Regression);
        assert_eq!(r.classify(false, false), Outcome::ResidualFailure);
        assert_eq!(r.classify(true, true), Outcome::Retained);
        assert_eq!(Outcome::ALL.len(), Outcome::COUNT);
    }

    // ---- corrections -----------------------------------------------------

    #[test]
    fn a_correction_is_appended_and_never_rewrites_the_retirement() {
        let mut t = ticket();
        retire(&mut t, retirement(RetirementReason::CompletedAndValidated, ReadBack::Held)).unwrap();
        let before = t.retirement.clone();
        correct(&mut t, "the coordinate was off by one", "user").expect("a correction");
        assert!(t.corrected.as_ref().unwrap().contains("by user"));
        assert_eq!(
            t.retirement, before,
            "a correction must not mutate the retirement, or the retirement is unfalsifiable"
        );
    }

    #[test]
    fn an_unauthorized_or_empty_correction_is_refused() {
        let mut t = ticket();
        assert!(correct(&mut t, "something", "  ").is_err(), "no author is not a correction");
        assert!(correct(&mut t, "  ", "user").is_err(), "no content corrects nothing");
        assert!(t.corrected.is_none());
    }

    // ---- the chokepoint, and the board as a view -------------------------

    #[test]
    fn filing_goes_through_the_chokepoint_and_a_denial_is_on_the_record() {
        let mut chain = Chain::new();
        let error = file(
            &mut chain,
            namespace(),
            TicketKind::Build,
            "dig the recorded voxel",
            gate(),
            false,
            "file_ticket requires a grant",
        )
        .expect_err("the chokepoint denied it");
        assert_eq!(error.all()[0].kind(), RefusalKind::UntrustedKey);
        // **The refusal is an event**, so a denial nobody can read did not happen.
        assert_eq!(chain.len(), 1);
        assert_eq!(chain.fold(0, |n, _| n + 1), 1);
    }

    #[test]
    fn an_allowed_ticket_lands_on_the_chain_and_is_readable_back() {
        let mut chain = Chain::new();
        let id = file(&mut chain, namespace(), TicketKind::Build, "dig it", gate(), true, "")
            .expect("an allowed filing");
        assert!(id.as_str().starts_with("BLD-"), "{}", id.as_str());
        let board = Board::from_chain(&chain);
        assert_eq!(board.len(), 1, "the board is a projection of the chain");
        assert!(!board.has_drained(), "one live ticket means the board has not drained");
    }

    #[test]
    fn a_ticket_with_no_objective_or_no_gate_is_refused_and_leaves_no_trace() {
        let mut chain = Chain::new();
        assert!(file(&mut chain, namespace(), TicketKind::Build, "  ", gate(), true, "").is_err());
        let empty_gate = Gate {
            statement: "   ".into(),
            expectation: Expectation::None,
        };
        assert!(file(&mut chain, namespace(), TicketKind::Build, "do a thing", empty_gate, true, "").is_err());
        assert_eq!(chain.len(), 0, "a refused filing records nothing");
    }

    #[test]
    fn the_report_says_the_chain_is_not_tamper_evident_when_it_is_not() {
        let mut chain = Chain::new();
        file(&mut chain, namespace(), TicketKind::Build, "dig it", gate(), true, "").unwrap();
        let text = report(&Board::from_chain(&chain), &chain);
        assert!(text.contains("NOT tamper-evident"), "{text}");
        assert!(text.contains("a view over the one chain"), "{text}");
    }

    // ---- the expectation, read back --------------------------------------

    #[test]
    fn an_expectation_names_the_coordinates_it_touches() {
        // The reference's `tiles()`, and the reason it exists: a reader must be able to ask
        // which coordinates a stale claim is about.
        let expectation = Expectation::EmptyAt { at: vec![[0, 0, 0], [1, 1, 1]] };
        assert_eq!(expectation.coordinates().len(), 2);
        assert_eq!(Expectation::None.coordinates().len(), 0);
        assert!(!Expectation::None.has_claim());
        assert!(Expectation::WorldDigest(crate::identity::Digest::of(b"w")).has_claim());
    }

    #[test]
    fn evidence_with_no_id_or_no_detail_is_refused() {
        let id = TicketId::new("BLD-0001").unwrap();
        assert!(Evidence::new("", id.clone(), EvidenceOrigin::ReadBack, "saw it").is_err());
        assert!(Evidence::new("ev-1", id, EvidenceOrigin::ReadBack, "   ").is_err());
    }
}
