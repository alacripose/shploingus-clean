//! The axes, and every prohibition the rules state as a predicate.
//!
//! # Provenance: derived from the sources, not from the rederivation
//!
//! Spec: `AGENTS.md` §19.1–19.3 and binding rules 51–55 in
//! `references/Docs-A/ticket-ledger-engine-prototype/seasons/impl/godplan/axes.py`,
//! the round-5 bare-token rule, §17 rules 3–4, and §25.1. The sixth and seventh
//! ladders come from the security/gov systems: `EvidenceLevel` is ala-system's
//! strength scale (`support.ts`), and [`crate::ticket::EvidenceOrigin`] is
//! `ala-cities::gov::Evidence` — registered here by bridge, never redefined.
//!
//! The dirty tree's `diffusion-core/src/axis.rs` is a **rederivation** of these
//! systems and was not up to snuff with its own source: it flattened
//! `ProceduralGate`'s predicate set into a status enum, dropped rule 55's
//! `Unit`/`advance`/`graduate` invariants, kept no certification evaluation
//! (only the refusal of `max`), and let single-claimant bare tokens pass where
//! the source refuses every one. What is here follows the source.
//!
//! # The rules are the point: refusals, not warnings
//!
//! Every operation whose misuse the rules forbid **refuses**. A warning is a
//! control the caller can ignore, and §17 rule 3 already sets the standard: an
//! unaccounted value makes the output REFUSED, not stored with a caveat.
//!
//! 1. **Rule 51 — every ladder is its own type**, so two ladders cannot be
//!    compared without [`refuse_interchange`] noticing.
//! 2. **Rule 52 — certification is a threshold over the whole vector**, never a
//!    maximum: `[A, D, D, D]` would certify a body of work on one strong claim
//!    while three critical components stay unresolved.
//! 3. **Rule 53 — a ladder never renders as a number.** Validity is not a
//!    quantity and a status is not a score; [`refuse_reduction`] names the
//!    reduction attempted and refuses it, for every axis.
//! 4. **Rule 55 — a transition needs its destination.** `advance` requires a
//!    destination grade exists; `graduate` requires that none does; a terminal
//!    unit moves no further. State and event stay distinct.
//! 5. **Round 5 — a bare ladder token is a bug.** Ambiguous (two ladders claim
//!    it) and unqualified (one claims it) are two distinct failures, and
//!    conflating them was itself a bug.
//!
//! §17 rule 4 and §25.1 hold the floor and the ceiling: absence of
//! certification is a **state** (`None`), never a floor — treating an unobserved
//! metric as `0` is the same class of error — and a ladder whose top is
//! reachable by iterating has made the top a quantity, so office rank CEO is
//! reached by **election** and [`OrgRank::next`] refuses to walk there.
//!
//! # The defect the bare-token rule found, recorded so it is not re-merged
//!
//! Section 19.1 enumerates three axes and §19.2 adds the `F..SSS` grade ladder —
//! whose tokens **overlap** with [`EvidenceGrade`]: `A`, `B`, `C`, `D` are valid
//! on both and mean different things. Evidence grade `A` is the **strongest**;
//! grade `A` sits **below `S`**. Implementing the round-5 rule is what found the
//! fourth ladder. A reader who sees two sevens of axes anywhere in this project
//! must not reconcile them.

use std::collections::BTreeMap;

use crate::refusal::{Refusal, RefusalKind, RefusalSet};
use crate::ticket::EvidenceOrigin;

/// Which ladder a value belongs to. **There is no default axis**: a value that
/// cannot be placed on one is refused rather than assumed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(musli::Encode, musli::Decode)]
pub enum AxisKind {
    /// Per claim: how strong the evidence makes it. **`A` is the strongest.**
    EvidenceGrade,
    /// Per cluster or campaign: an ordinal threshold evaluation. Never a maximum.
    CertificationTier,
    /// Per action: named predicates with statuses. **Not a scalar.**
    ProceduralGate,
    /// Per unit: `F < E < D < C < B < A < S < SS < SSS`. **`A` sits below `S`.**
    Grade,
    /// Per office grant: fourteen rungs, and the top is reached by **election**.
    OrgRank,
    /// Per assertion: the strength of its support (ala-system `support.ts`).
    SupportLevel,
    /// Per ticket evidence: how the evidence came to exist
    /// ([`crate::ticket::EvidenceOrigin`], `ala-cities::gov::Evidence`).
    TicketEvidence,
    /// Per prose unit: what kind of statement it is (the scrubber's `prose_kind` ladder).
    ///
    /// **A kind, not a rank.** The eight labels carry no order, and the type
    /// deliberately derives no `Ord`; ranking one against another is the
    /// representation rule 51 forbids.
    ProseKind,
    /// Per observed thing: whether it is established (the census's own five states).
    Establishment,
}

/// How many axes this build knows. **Written down rather than derived** — the
/// chain of pins in [`COVERAGE_IS_TOTAL`] and [`AxisKind::declared_values`] is
/// what closes the gap a hand-written count leaves.
pub const AXIS_COUNT: usize = 9;

impl AxisKind {
    /// Every axis this build knows.
    pub const ALL: [AxisKind; AXIS_COUNT] = [
        AxisKind::EvidenceGrade,
        AxisKind::CertificationTier,
        AxisKind::ProceduralGate,
        AxisKind::Grade,
        AxisKind::OrgRank,
        AxisKind::SupportLevel,
        AxisKind::TicketEvidence,
        AxisKind::ProseKind,
        AxisKind::Establishment,
    ];

    /// The recorded name.
    pub const fn name(self) -> &'static str {
        match self {
            AxisKind::EvidenceGrade => "evidence_grade",
            AxisKind::CertificationTier => "certification_tier",
            AxisKind::ProceduralGate => "procedural_gate",
            AxisKind::Grade => "grade",
            AxisKind::OrgRank => "org_rank",
            AxisKind::SupportLevel => "support_level",
            AxisKind::TicketEvidence => "ticket_evidence",
            AxisKind::ProseKind => "prose_kind",
            AxisKind::Establishment => "establishment",
        }
    }

    /// The tokens this ladder's members are recorded as.
    ///
    /// **Exhaustive on purpose** — a new axis with no ladder behind it does not
    /// compile. Deliberately a `match` over `self` and not a lookup into
    /// [`COVERAGE`]: if it read the coverage table, a fabricated row would agree
    /// with itself and nothing could catch it.
    pub const fn declared_values(self) -> &'static [&'static str] {
        match self {
            AxisKind::EvidenceGrade => EvidenceGrade::TOKENS,
            AxisKind::CertificationTier => CertificationTier::TOKENS,
            AxisKind::ProceduralGate => GateStatus::TOKENS,
            AxisKind::Grade => Grade::TOKENS,
            AxisKind::OrgRank => OrgRank::TOKENS,
            AxisKind::SupportLevel => EvidenceLevel::TOKENS,
            AxisKind::TicketEvidence => crate::ticket::EvidenceOrigin::TOKENS,
            AxisKind::ProseKind => ProseKind::TOKENS,
            AxisKind::Establishment => Establishment::TOKENS,
        }
    }

    /// Rule 53's one-liner, per axis: no ladder is a quantity.
    pub const fn is_a_quantity(self) -> bool {
        false
    }
}

impl std::fmt::Display for AxisKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.name())
    }
}

/// A value that lives on **one** ladder and compares only within it.
pub trait Ladder: Copy {
    /// The axis this ladder belongs to. The one thing every implementor declares.
    const AXIS: AxisKind;

    /// The recorded name of this member.
    fn as_str(self) -> &'static str;
}

/// Rule 51 — refuse a comparison between two values on different ladders.
///
/// **`Ok` means the two are on the same ladder, nothing more.** Which member
/// outranks which is the ladder's own business; a caller must choose a ladder
/// before it chooses a direction.
pub fn refuse_interchange<A: Ladder, B: Ladder>(left: A, right: B) -> Result<(), Refusal> {
    if A::AXIS == B::AXIS {
        return Ok(());
    }
    Err(Refusal::owned(
        RefusalKind::AxisInterchangeRefused,
        format!(
            "{} / {}: {} is not interchangeable with {}; comparing across ladders is what \
             makes a bare token mean two things, and neither is a number",
            left.as_str(),
            right.as_str(),
            A::AXIS.name(),
            B::AXIS.name()
        ),
    ))
}

/// How a ladder was asked to be reduced to a number. Its own type, so adding a
/// fourth way of quantifying a ladder is a visible change.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Reduction {
    /// Rendered as a percentage.
    Percentage,
    /// Rendered as a score.
    Score,
    /// Aggregated by taking the maximum.
    Maximum,
}

impl Reduction {
    /// The recorded name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Reduction::Percentage => "as_percentage",
            Reduction::Score => "as_score",
            Reduction::Maximum => "maximum",
        }
    }
}

impl std::fmt::Display for Reduction {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Rule 53 (and rule 52's sharper case) — refuse to reduce a ladder to a
/// quantity. **Every axis, not only the two the source happened to guard.**
pub fn refuse_reduction(axis: AxisKind, reduction: Reduction) -> Refusal {
    let reason = match reduction {
        Reduction::Maximum if axis == AxisKind::CertificationTier => {
            "certification is a threshold evaluation over the whole vector, not a maximum: \
             [A, D, D, D] would certify a body of work on one strong claim while three \
             critical components stay unresolved (rule 52)"
        }
        Reduction::Percentage => {
            "validity is not a quantity; a ladder never renders as a percentage, and \
             legal_sufficiency = 87% is a REFUSED value (rule 53)"
        }
        Reduction::Score => "a status is not a score (rule 53)",
        Reduction::Maximum => {
            "the maximum is the wrong aggregation for a requirement gate: one strong member \
             does not make the rest true"
        }
    };
    Refusal::owned(
        RefusalKind::ScalarReductionRefused,
        format!("{} {} — {}", axis.name(), reduction, reason),
    )
}

// ---------------------------------------------------------------------------
// The ladders. Each is its own type; none shares a representation.
// ---------------------------------------------------------------------------

/// Per claim: how strong the evidence makes it. **`A` is the strongest.**
///
/// This ladder's `A` is the strongest value while [`Grade`]'s `A` is below `S` —
/// the same character, opposite directions. That is the sharpest argument in the
/// tree for typing the ladders apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EvidenceGrade {
    /// External oracle, or a record equivalent to one.
    A,
    /// Read back from world state.
    B,
    /// Self-report, or an artefact with no independent site.
    C,
    /// Declared but unsupported.
    D,
}

impl EvidenceGrade {
    /// Every grade, in the source's order.
    pub const ALL: [EvidenceGrade; 4] = [
        EvidenceGrade::A,
        EvidenceGrade::B,
        EvidenceGrade::C,
        EvidenceGrade::D,
    ];

    /// The recorded tokens.
    pub const TOKENS: &'static [&'static str] = &["A", "B", "C", "D"];

    /// The recorded name.
    pub const fn as_str(self) -> &'static str {
        match self {
            EvidenceGrade::A => "A",
            EvidenceGrade::B => "B",
            EvidenceGrade::C => "C",
            EvidenceGrade::D => "D",
        }
    }

    /// Parse a recorded name. Unknown is refused rather than defaulted.
    pub fn parse(word: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|candidate| candidate.as_str() == word.trim())
            .ok_or_else(|| format!("unknown evidence grade {word:?}"))
    }

    /// How strong, weakest to strongest. **A direction, not a score.**
    pub const fn strength(self) -> u8 {
        match self {
            EvidenceGrade::D => 1,
            EvidenceGrade::C => 2,
            EvidenceGrade::B => 3,
            EvidenceGrade::A => 4,
        }
    }
}

impl Ladder for EvidenceGrade {
    const AXIS: AxisKind = AxisKind::EvidenceGrade;

    fn as_str(self) -> &'static str {
        self.as_str()
    }
}

impl std::fmt::Display for EvidenceGrade {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "evidence grade {}", self.as_str())
    }
}

/// Per cluster or campaign: an ordinal threshold evaluation.
///
/// **Five members, and `diamond` is real** — the source's comment carries the
/// defect class: *"an enum listing four is a defect."* An omitted rung is
/// invisible in a summary and wrong in every comparison.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CertificationTier {
    /// Provenance declared.
    Bronze,
    /// Provenance declared and evidence present.
    Silver,
    /// Plus no open contradiction.
    Gold,
    /// Plus the gate cleared.
    Diamond,
    /// Plus reconstruction verified.
    Platinum,
}

impl CertificationTier {
    /// Every tier, in the source's order.
    pub const ALL: [CertificationTier; 5] = [
        CertificationTier::Bronze,
        CertificationTier::Silver,
        CertificationTier::Gold,
        CertificationTier::Diamond,
        CertificationTier::Platinum,
    ];

    /// The recorded tokens.
    pub const TOKENS: &'static [&'static str] =
        &["bronze", "silver", "gold", "diamond", "platinum"];

    /// The recorded name.
    pub const fn as_str(self) -> &'static str {
        match self {
            CertificationTier::Bronze => "bronze",
            CertificationTier::Silver => "silver",
            CertificationTier::Gold => "gold",
            CertificationTier::Diamond => "diamond",
            CertificationTier::Platinum => "platinum",
        }
    }

    /// Parse a recorded name.
    pub fn parse(word: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|candidate| candidate.as_str() == word.trim())
            .ok_or_else(|| format!("unknown certification tier {word:?}"))
    }

    /// The ordinal. **Explicit, never inferred from the way a tier is written.**
    pub const fn ordinal(self) -> u8 {
        match self {
            CertificationTier::Bronze => 1,
            CertificationTier::Silver => 2,
            CertificationTier::Gold => 3,
            CertificationTier::Diamond => 4,
            CertificationTier::Platinum => 5,
        }
    }
}

impl Ladder for CertificationTier {
    const AXIS: AxisKind = AxisKind::CertificationTier;

    fn as_str(self) -> &'static str {
        self.as_str()
    }
}

impl std::fmt::Display for CertificationTier {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "certification tier {}", self.as_str())
    }
}

/// One predicate's status on a [`ProceduralGate`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GateStatus {
    /// Every blocking predicate is clear.
    Pass,
    /// A predicate failed.
    Fail,
    /// No predicate applies to this action.
    NotApplicable,
    /// A predicate was clear once and its window has closed.
    Expired,
    /// A predicate is stopping the action.
    Blocked,
}

impl GateStatus {
    /// Every status.
    pub const ALL: [GateStatus; 5] = [
        GateStatus::Pass,
        GateStatus::Fail,
        GateStatus::NotApplicable,
        GateStatus::Expired,
        GateStatus::Blocked,
    ];

    /// The recorded tokens.
    pub const TOKENS: &'static [&'static str] =
        &["PASS", "FAIL", "NOT_APPLICABLE", "EXPIRED", "BLOCKED"];

    /// The recorded name.
    pub const fn as_str(self) -> &'static str {
        match self {
            GateStatus::Pass => "PASS",
            GateStatus::Fail => "FAIL",
            GateStatus::NotApplicable => "NOT_APPLICABLE",
            GateStatus::Expired => "EXPIRED",
            GateStatus::Blocked => "BLOCKED",
        }
    }

    /// Parse a recorded name.
    pub fn parse(word: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|candidate| candidate.as_str() == word.trim())
            .ok_or_else(|| format!("unknown gate status {word:?}; the tokens are uppercase"))
    }

    /// Whether this status stops the action. [`GateStatus::NotApplicable`] does
    /// **not** block — separating it is why the statuses are named, not counted.
    pub const fn blocking(self) -> bool {
        matches!(
            self,
            GateStatus::Fail | GateStatus::Expired | GateStatus::Blocked
        )
    }

    /// Whether nothing is stopping the action.
    pub const fn cleared(self) -> bool {
        !self.blocking()
    }
}

impl Ladder for GateStatus {
    const AXIS: AxisKind = AxisKind::ProceduralGate;

    fn as_str(self) -> &'static str {
        self.as_str()
    }
}

/// Per action: **a set of named predicates with statuses**, and the policy they
/// are evaluated under. Rule 53's subject.
///
/// The source's shape, kept whole: `predicates: Mapping[str, GateStatus]` plus
/// `policy`. A gate is not a status word — flattening it to one is how "which
/// predicate failed" became unaskable.
///
/// **No numeric accessor exists.** The source offers `as_percentage`/`as_score`
/// only so they can raise; here the reductions are refused by name through
/// [`refuse_reduction`] and there is nothing else to call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProceduralGate {
    /// The named predicates and their statuses.
    pub predicates: BTreeMap<String, GateStatus>,
    /// The policy the predicates are evaluated under.
    pub policy: String,
}

impl ProceduralGate {
    /// The predicate names that stop the action, sorted — the report is a list
    /// of names, never a count.
    pub fn blocking(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .predicates
            .iter()
            .filter(|(_, status)| status.blocking())
            .map(|(name, _)| name.clone())
            .collect();
        names.sort();
        names
    }

    /// Whether nothing is stopping the action.
    pub fn cleared(&self) -> bool {
        self.predicates.values().all(|status| status.cleared())
    }
}

impl std::fmt::Display for ProceduralGate {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "procedural gate {} under {}",
            if self.cleared() { "PASS" } else { "BLOCKED" },
            self.policy
        )
    }
}

/// Per unit: `F < E < D < C < B < A < S < SS < SSS`. **`A` sits below `S`.**
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Grade {
    F,
    E,
    D,
    C,
    B,
    A,
    S,
    SS,
    SSS,
}

impl Grade {
    /// Every grade, in the source's order. **Nine levels** — a rung added or
    /// reordered is invisible in a summary, so the count is asserted.
    pub const ALL: [Grade; 9] = [
        Grade::F,
        Grade::E,
        Grade::D,
        Grade::C,
        Grade::B,
        Grade::A,
        Grade::S,
        Grade::SS,
        Grade::SSS,
    ];

    /// The recorded tokens.
    pub const TOKENS: &'static [&'static str] = &["F", "E", "D", "C", "B", "A", "S", "SS", "SSS"];

    /// The recorded name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Grade::F => "F",
            Grade::E => "E",
            Grade::D => "D",
            Grade::C => "C",
            Grade::B => "B",
            Grade::A => "A",
            Grade::S => "S",
            Grade::SS => "SS",
            Grade::SSS => "SSS",
        }
    }

    /// Parse a recorded name.
    pub fn parse(word: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|candidate| candidate.as_str() == word.trim())
            .ok_or_else(|| format!("unknown unit grade {word:?}"))
    }

    /// Position in [`Self::ALL`], weakest to strongest. **A direction, not a
    /// score** — [`refuse_reduction`] is what stops it being used as one.
    pub const fn rank(self) -> u8 {
        match self {
            Grade::F => 0,
            Grade::E => 1,
            Grade::D => 2,
            Grade::C => 3,
            Grade::B => 4,
            Grade::A => 5,
            Grade::S => 6,
            Grade::SS => 7,
            Grade::SSS => 8,
        }
    }

    /// The ceiling, named rather than compared by ordinal at every call site.
    pub const fn is_the_ceiling(self) -> bool {
        matches!(self, Grade::SSS)
    }
}

impl Ladder for Grade {
    const AXIS: AxisKind = AxisKind::Grade;

    fn as_str(self) -> &'static str {
        self.as_str()
    }
}

impl std::fmt::Display for Grade {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "grade {}", self.as_str())
    }
}

/// Rule 55 — state and event kept distinct. [`Unit::advance`] and
/// [`Unit::graduate`] are **events**; this is the state they read and write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitStatus {
    /// The unit is still moving.
    Active,
    /// `graduate` ended it. Nothing moves a terminal unit.
    Terminal,
}

/// A unit on the grade ladder. Rule 55's two invariants:
///
/// * `advance` requires that a destination grade **exists**;
/// * `graduate` requires that **no** destination grade exists — it is valid only
///   at [`Grade::SSS`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unit {
    /// Where the unit stands.
    pub grade: Grade,
    /// Whether it is still moving.
    pub status: UnitStatus,
}

impl Unit {
    /// An active unit at a grade.
    pub const fn new(grade: Grade) -> Self {
        Unit {
            grade,
            status: UnitStatus::Active,
        }
    }

    /// Rule 55 — advance requires that a destination grade **exists**.
    pub fn advance(&mut self) -> Result<(), Refusal> {
        if self.status == UnitStatus::Terminal {
            return Err(Refusal::owned(
                RefusalKind::DestinationInvariantRefused,
                "advance on a terminal unit; graduate already ended it".to_string(),
            ));
        }
        if self.grade.is_the_ceiling() {
            return Err(Refusal::owned(
                RefusalKind::DestinationInvariantRefused,
                format!(
                    "advance({}) REJECTED — no destination grade exists above the ceiling; \
                     graduate is the only event left",
                    self.grade.as_str()
                ),
            ));
        }
        self.grade = Grade::ALL[self.grade.rank() as usize + 1];
        Ok(())
    }

    /// Rule 55 — graduate requires that **no** destination grade exists.
    pub fn graduate(&mut self) -> Result<(), Refusal> {
        if self.status == UnitStatus::Terminal {
            return Err(Refusal::owned(
                RefusalKind::DestinationInvariantRefused,
                "graduate on an already-terminal unit".to_string(),
            ));
        }
        if !self.grade.is_the_ceiling() {
            return Err(Refusal::owned(
                RefusalKind::DestinationInvariantRefused,
                format!(
                    "graduate({}) REJECTED — a destination grade exists; use advance for a \
                     transition and graduate only for terminal completion",
                    self.grade.as_str()
                ),
            ));
        }
        self.status = UnitStatus::Terminal;
        Ok(())
    }
}

/// Per office grant: fourteen rungs, and the top is reached by **election**.
///
/// The organization plan's leash: *membership never implies authority and
/// capability never implies authority* — holdings are recorded as grants, and
/// *"organizational history is a chain of grants, never a mutated rank field."*
/// A director is not "more craftsman" than a manager: this ladder extends no
/// other, and comparing it against one is rule 51's defect.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OrgRank {
    Applicant,
    Associate,
    Member,
    SeniorMember,
    Lead,
    Manager,
    SeniorManager,
    Director,
    SeniorDirector,
    VicePresident,
    SeniorVicePresident,
    ChiefOperatingOfficer,
    President,
    ChiefExecutiveOfficer,
}

impl OrgRank {
    /// Every rank, in the source's order.
    pub const ALL: [OrgRank; 14] = [
        OrgRank::Applicant,
        OrgRank::Associate,
        OrgRank::Member,
        OrgRank::SeniorMember,
        OrgRank::Lead,
        OrgRank::Manager,
        OrgRank::SeniorManager,
        OrgRank::Director,
        OrgRank::SeniorDirector,
        OrgRank::VicePresident,
        OrgRank::SeniorVicePresident,
        OrgRank::ChiefOperatingOfficer,
        OrgRank::President,
        OrgRank::ChiefExecutiveOfficer,
    ];

    /// The recorded tokens.
    pub const TOKENS: &'static [&'static str] = &[
        "applicant",
        "associate",
        "member",
        "senior_member",
        "lead",
        "manager",
        "senior_manager",
        "director",
        "senior_director",
        "vice_president",
        "senior_vice_president",
        "chief_operating_officer",
        "president",
        "chief_executive_officer",
    ];

    /// The recorded name.
    pub const fn as_str(self) -> &'static str {
        match self {
            OrgRank::Applicant => "applicant",
            OrgRank::Associate => "associate",
            OrgRank::Member => "member",
            OrgRank::SeniorMember => "senior_member",
            OrgRank::Lead => "lead",
            OrgRank::Manager => "manager",
            OrgRank::SeniorManager => "senior_manager",
            OrgRank::Director => "director",
            OrgRank::SeniorDirector => "senior_director",
            OrgRank::VicePresident => "vice_president",
            OrgRank::SeniorVicePresident => "senior_vice_president",
            OrgRank::ChiefOperatingOfficer => "chief_operating_officer",
            OrgRank::President => "president",
            OrgRank::ChiefExecutiveOfficer => "chief_executive_officer",
        }
    }

    /// Parse a recorded name.
    pub fn parse(word: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|candidate| candidate.as_str() == word.trim())
            .ok_or_else(|| format!("unknown office rank {word:?}"))
    }

    /// The ordinal. **Explicit, never inferred from the name.**
    pub const fn ordinal(self) -> u8 {
        match self {
            OrgRank::Applicant => 1,
            OrgRank::Associate => 2,
            OrgRank::Member => 3,
            OrgRank::SeniorMember => 4,
            OrgRank::Lead => 5,
            OrgRank::Manager => 6,
            OrgRank::SeniorManager => 7,
            OrgRank::Director => 8,
            OrgRank::SeniorDirector => 9,
            OrgRank::VicePresident => 10,
            OrgRank::SeniorVicePresident => 11,
            OrgRank::ChiefOperatingOfficer => 12,
            OrgRank::President => 13,
            OrgRank::ChiefExecutiveOfficer => 14,
        }
    }

    /// The ceiling, named.
    pub const fn is_ceo(self) -> bool {
        matches!(self, OrgRank::ChiefExecutiveOfficer)
    }

    /// The rung above, or a **refusal** (rule 55's shape, §25.1's reason).
    ///
    /// There is no `advance()` into [`OrgRank::ChiefExecutiveOfficer`]: the top
    /// office is admitted by **election by the board** — a different *kind* of
    /// promotion. [`OrgRank::President`] refuses too: a ladder that could reach
    /// the top by iterating would make the top a quantity.
    pub fn next(self) -> Result<OrgRank, Refusal> {
        match self {
            OrgRank::ChiefExecutiveOfficer => Err(Refusal::owned(
                RefusalKind::DestinationInvariantRefused,
                "office rank chief_executive_officer is the ceiling — no destination rank exists \
                 above it, and the top office is admitted by election, not by advancing"
                    .to_string(),
            )),
            OrgRank::President => Err(Refusal::owned(
                RefusalKind::DestinationInvariantRefused,
                "advance(president) REJECTED — the rung above president is chief_executive_officer, \
                 reachable only by a promotion event whose kind is `election` with the board as the \
                 constituency; a ladder that could reach the top by iterating would make the top a \
                 quantity (§25.1)"
                    .to_string(),
            )),
            _ => Ok(Self::ALL[self.ordinal() as usize]),
        }
    }
}

impl Ladder for OrgRank {
    const AXIS: AxisKind = AxisKind::OrgRank;

    fn as_str(self) -> &'static str {
        self.as_str()
    }
}

impl std::fmt::Display for OrgRank {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "office rank {}", self.as_str())
    }
}

/// Per assertion: the strength of its support — ala-system's `support.ts` scale,
/// registered rather than redefined.
///
/// **The ordering is ala-system's and is not re-derived here.** `support.ts` has
/// not been read for this build, so no rank values are carried: a missing value
/// is never turned into an invented constant (§17 rule 4's class). The tokens
/// are the ladder; a reader wanting direction reads the source that owns it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EvidenceLevel {
    Direct,
    StronglySupported,
    Supported,
    Inferred,
    SelfReported,
    Unverified,
}

impl EvidenceLevel {
    /// Every level.
    pub const ALL: [EvidenceLevel; 6] = [
        EvidenceLevel::Direct,
        EvidenceLevel::StronglySupported,
        EvidenceLevel::Supported,
        EvidenceLevel::Inferred,
        EvidenceLevel::SelfReported,
        EvidenceLevel::Unverified,
    ];

    /// The recorded tokens.
    pub const TOKENS: &'static [&'static str] = &[
        "direct",
        "strongly_supported",
        "supported",
        "inferred",
        "self_reported",
        "unverified",
    ];

    /// The recorded name.
    pub const fn as_str(self) -> &'static str {
        match self {
            EvidenceLevel::Direct => "direct",
            EvidenceLevel::StronglySupported => "strongly_supported",
            EvidenceLevel::Supported => "supported",
            EvidenceLevel::Inferred => "inferred",
            EvidenceLevel::SelfReported => "self_reported",
            EvidenceLevel::Unverified => "unverified",
        }
    }

    /// Parse a recorded name.
    pub fn parse(word: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|candidate| candidate.as_str() == word.trim())
            .ok_or_else(|| format!("unknown support level {word:?}"))
    }
}

impl Ladder for EvidenceLevel {
    const AXIS: AxisKind = AxisKind::SupportLevel;

    fn as_str(self) -> &'static str {
        self.as_str()
    }
}

/// The seventh ladder already exists in this crate as
/// [`crate::ticket::EvidenceOrigin`] — registered by bridge, never redefined. A
/// second origin type beside it would be two representations of one scale.
impl Ladder for EvidenceOrigin {
    const AXIS: AxisKind = AxisKind::TicketEvidence;

    fn as_str(self) -> &'static str {
        self.letter()
    }
}

// ---------------------------------------------------------------------------
// Rule 52 — certification is a threshold over the whole vector
// ---------------------------------------------------------------------------

/// One named requirement in a certification band, with the optional
/// per-requirement evidence bar the source declares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Requirement {
    /// The requirement's recorded name.
    pub name: &'static str,
    /// The evidence bar this requirement holds, when it holds one. The source's
    /// table carries names only; a bar is declared per requirement, never assumed.
    pub minimum_evidence: Option<EvidenceGrade>,
    /// Whether failing this requirement blocks the band.
    pub critical: bool,
}

/// The requirement bands, in ladder order. A tier is **held** when every
/// requirement up to and including its band has been satisfied — a threshold
/// evaluation, never a maximum.
pub fn requirements(tier: CertificationTier) -> &'static [&'static str] {
    match tier {
        CertificationTier::Bronze => &["declared_provenance"],
        CertificationTier::Silver => &["declared_provenance", "evidence_present"],
        CertificationTier::Gold => &[
            "declared_provenance",
            "evidence_present",
            "no_open_contradiction",
        ],
        CertificationTier::Diamond => &[
            "declared_provenance",
            "evidence_present",
            "no_open_contradiction",
            "gate_cleared",
        ],
        CertificationTier::Platinum => &[
            "declared_provenance",
            "evidence_present",
            "no_open_contradiction",
            "gate_cleared",
            "reconstruction_verified",
        ],
    }
}

/// Rule 52 — the highest tier whose requirements **all** hold, or `None`.
///
/// **`None` rather than bronze-by-default**: absence of certification is a
/// state, and inventing a floor is the same class of error as treating an
/// unobserved metric as `0` (§17 rule 4).
pub fn certification_tier(satisfied: &[&str]) -> Option<CertificationTier> {
    let mut held: Option<CertificationTier> = None;
    for tier in CertificationTier::ALL {
        if requirements(tier)
            .iter()
            .all(|requirement| satisfied.contains(requirement))
        {
            held = Some(tier);
        } else {
            break;
        }
    }
    held
}

/// Rule 52 as an executable refusal — so the prohibition is testable rather than
/// written down. A caller that reaches for the maximum over a metric set gets
/// this, not a low tier.
pub fn prohibited_max_certification() -> Refusal {
    Refusal::owned(
        RefusalKind::AggregationRefused,
        "certification_tier != max(evidence_grade): a maximum is the wrong aggregation for a \
         requirement gate. [A, D, D, D] would certify a body of work on one strong claim while \
         three critical components stay unresolved (rule 52)"
            .to_string(),
    )
}

// ---------------------------------------------------------------------------
// Round 5 — a bare ladder token is a bug
// ---------------------------------------------------------------------------

/// Every axis that claims a bare token. **Many-valued on purpose** — a
/// single-valued map would let one axis win a contested token, which is the
/// failure mode the rule exists to prevent.
///
/// The map covers the ladders whose tokens are **recorded as bare tokens**
/// (letters and names): evidence grade, certification tier, unit grade, office
/// rank, and the ticket-evidence letters. Gate statuses and support levels are
/// quoted as statuses and words, never as bare ladder tokens, and the source's
/// own map excludes them.
pub fn bare_token_axes(token: &str) -> Vec<AxisKind> {
    let token = token.trim();
    let mut claimants = Vec::new();
    for (axis, tokens) in [
        (AxisKind::EvidenceGrade, EvidenceGrade::TOKENS),
        (AxisKind::CertificationTier, CertificationTier::TOKENS),
        (AxisKind::Grade, Grade::TOKENS),
        (AxisKind::OrgRank, OrgRank::TOKENS),
        (AxisKind::TicketEvidence, EvidenceOrigin::TOKENS),
        (AxisKind::ProseKind, ProseKind::TOKENS),
        (AxisKind::Establishment, Establishment::TOKENS),
    ] {
        if tokens.contains(&token) {
            claimants.push(axis);
        }
    }
    claimants
}

/// Tokens that more than one ladder claims. A non-empty result is a vocabulary
/// defect, kept visible rather than resolved.
pub fn ambiguous_tokens() -> BTreeMap<String, Vec<AxisKind>> {
    let mut contested = BTreeMap::new();
    for tokens in [
        EvidenceGrade::TOKENS,
        CertificationTier::TOKENS,
        Grade::TOKENS,
        OrgRank::TOKENS,
        EvidenceOrigin::TOKENS,
        ProseKind::TOKENS,
        Establishment::TOKENS,
    ] {
        for token in tokens {
            let claimants = bare_token_axes(token);
            if claimants.len() > 1 {
                contested.insert((*token).to_string(), claimants);
            }
        }
    }
    contested
}

/// Refuse a bare ladder token, and say **which** failure it is.
///
/// Two distinct failures, and conflating them was itself a bug: a token claimed
/// by two ladders is **ambiguous**, while a token claimed by one is merely
/// **unqualified**. `gold` is the second kind; calling it ambiguous sends a
/// reader looking for a collision that does not exist.
///
/// **A token no ladder claims is not this rule's business** — it is not a ladder
/// token at all (the source returns here rather than refusing).
pub fn refuse_bare(token: &str) -> Result<(), Refusal> {
    let claimants = bare_token_axes(token);
    match claimants.as_slice() {
        [] => Ok(()),
        [only] => Err(Refusal::owned(
            RefusalKind::BareTokenRefused,
            format!(
                "bare ladder token {:?} is UNQUALIFIED — it belongs to {} and to no other ladder, \
                 but a bare ladder token is a bug. Write '{} {}'",
                token.trim(),
                only.name(),
                only.name(),
                token.trim()
            ),
        )),
        several => Err(Refusal::owned(
            RefusalKind::BareTokenRefused,
            format!(
                "bare ladder token {:?} is AMBIGUOUS — claimed by {}; write the ladder with the \
                 token, e.g. 'evidence grade A' or 'grade A'",
                token.trim(),
                several
                    .iter()
                    .map(|axis| axis.name())
                    .collect::<Vec<_>>()
                    .join(" or ")
            ),
        )),
    }
}

// ---------------------------------------------------------------------------
// The prose-kind ladder: what a sentence is, never how strong it is
// ---------------------------------------------------------------------------

/// What kind of statement a prose unit makes.
///
/// The eight labels come from `agent_god_files/prose_logic/src/claim.rs`'s `AtomKind`, and
/// **there is no order**: a definition does not outrank a condition, and a permission is not a
/// weaker obligation. The type therefore derives no `Ord`, which is the same rule
/// [`EvidenceOrigin`](crate::ticket::EvidenceOrigin) follows for the same reason.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[derive(musli::Encode, musli::Decode)]
pub enum ProseKind {
    /// A term is given its meaning.
    Definition,
    /// A duty the text imposes.
    Obligation,
    /// An act the text forbids.
    Prohibition,
    /// An act the text allows.
    Permission,
    /// A statement of what is the case.
    Fact,
    /// A rule or rule-like premise.
    Principle,
    /// A condition on another statement.
    Condition,
    /// An ordered step in a procedure.
    Procedure,
}

impl ProseKind {
    /// Every kind, in the declaration's order.
    pub const ALL: [ProseKind; 8] = [
        ProseKind::Definition,
        ProseKind::Obligation,
        ProseKind::Prohibition,
        ProseKind::Permission,
        ProseKind::Fact,
        ProseKind::Principle,
        ProseKind::Condition,
        ProseKind::Procedure,
    ];

    /// The recorded tokens.
    pub const TOKENS: &'static [&'static str] = &[
        "definition",
        "obligation",
        "prohibition",
        "permission",
        "fact",
        "principle",
        "condition",
        "procedure",
    ];

    /// The recorded name. **A label, not a rank.**
    pub const fn as_str(self) -> &'static str {
        match self {
            ProseKind::Definition => "definition",
            ProseKind::Obligation => "obligation",
            ProseKind::Prohibition => "prohibition",
            ProseKind::Permission => "permission",
            ProseKind::Fact => "fact",
            ProseKind::Principle => "principle",
            ProseKind::Condition => "condition",
            ProseKind::Procedure => "procedure",
        }
    }

    /// Parse a recorded name. An unknown kind is refused rather than defaulted.
    pub fn parse(word: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|candidate| candidate.as_str() == word.trim())
            .ok_or_else(|| format!("unknown prose kind {word:?}"))
    }
}

impl Ladder for ProseKind {
    const AXIS: AxisKind = AxisKind::ProseKind;

    fn as_str(self) -> &'static str {
        self.as_str()
    }
}

impl std::fmt::Display for ProseKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// The establishment ladder: checked, and what the check found
// ---------------------------------------------------------------------------

/// The state of one thing that was supposed to be checked.
///
/// These five states were declared in `voxel-package/src/features.rs` for the feature rows and
/// the census cells. They are a ladder, not a score: `active` is not "better" than `inactive`
/// in the way a number would be, and `unchecked` is never a pass. The type moved here so the
/// axis set has one declaration of it; `features` re-exports this type rather than keeping a
/// second copy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(musli::Encode, musli::Decode)]
pub enum Establishment {
    /// Checked, and it holds.
    Active,
    /// Checked, and it is not under this project's root.
    Inactive,
    /// **Checked, and what it read has since changed.**
    Stale,
    /// **No instrument ran.** Never checked. Not a pass.
    Unchecked,
    /// Checked, and it failed — or it could not be checked.
    Refused,
}

impl Establishment {
    /// Every state, in the census's order.
    pub const ALL: [Establishment; 5] = [
        Establishment::Active,
        Establishment::Inactive,
        Establishment::Stale,
        Establishment::Unchecked,
        Establishment::Refused,
    ];

    /// The recorded tokens.
    pub const TOKENS: &'static [&'static str] =
        &["active", "inactive", "stale", "unchecked", "refused"];

    /// The recorded name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Establishment::Active => "active",
            Establishment::Inactive => "inactive",
            Establishment::Stale => "stale",
            Establishment::Unchecked => "unchecked",
            Establishment::Refused => "refused",
        }
    }

    /// The same recorded name under the name the census adopted.
    pub const fn slug(self) -> &'static str {
        self.as_str()
    }

    /// The rendering. A state with no rendering is a blank row, and a blank row reads as
    /// "nothing to report".
    pub const fn describe(self) -> &'static str {
        match self {
            Establishment::Active => "active — checked, and it holds",
            Establishment::Inactive => {
                "inactive — checked, and not under this project's root. A grant is a recorded \
                 decision, not a toggle"
            }
            Establishment::Stale => {
                "stale — checked, and the bytes it was checked over have since changed, so the \
                 reading describes a tree this one is not. Re-derive it rather than reading it as \
                 an answer about this build"
            }
            Establishment::Unchecked => {
                "unchecked — no instrument ran, so this was never checked. A missing instrument \
                 is not a pass"
            }
            Establishment::Refused => {
                "refused — checked, and it failed, or it could not be checked"
            }
        }
    }

    /// Parse a recorded name. An unknown state is refused rather than defaulted.
    pub fn parse(word: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|candidate| candidate.as_str() == word.trim())
            .ok_or_else(|| format!("unknown establishment state {word:?}"))
    }
}

impl Ladder for Establishment {
    const AXIS: AxisKind = AxisKind::Establishment;

    fn as_str(self) -> &'static str {
        self.as_str()
    }
}

impl std::fmt::Display for Establishment {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// The coverage gate: a ladder with no representation fails the build
// ---------------------------------------------------------------------------

/// One ladder, its axis, its recorded tokens, and where it was measured from. A
/// row with no source is a row nobody can check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CoverageRow {
    /// The axis this ladder belongs to.
    pub axis: AxisKind,
    /// The type's name in this crate.
    pub ladder: &'static str,
    /// The tokens the ladder's members are recorded as.
    pub values: &'static [&'static str],
    /// Where this ladder was measured from.
    pub source: &'static str,
}

/// Every ladder this project knows, and where it came from. **This table is the
/// gate's subject**: a ladder with no row fails the build via
/// [`COVERAGE_IS_TOTAL`], and a row that misdescribes its ladder is refused at
/// runtime by [`check_coverage`].
pub const COVERAGE: [CoverageRow; AXIS_COUNT] = [
    CoverageRow {
        axis: AxisKind::EvidenceGrade,
        ladder: "EvidenceGrade",
        values: EvidenceGrade::TOKENS,
        source: "references/Docs-A/ticket-ledger-engine-prototype/seasons/impl/godplan/axes.py \
                 (EvidenceGrade, rule 19.1)",
    },
    CoverageRow {
        axis: AxisKind::CertificationTier,
        ladder: "CertificationTier",
        values: CertificationTier::TOKENS,
        source: "references/Docs-A/ticket-ledger-engine-prototype/seasons/impl/godplan/axes.py \
                 (CertificationTier, TIER_REQUIREMENTS)",
    },
    CoverageRow {
        axis: AxisKind::ProceduralGate,
        ladder: "GateStatus",
        values: GateStatus::TOKENS,
        source: "references/Docs-A/ticket-ledger-engine-prototype/seasons/impl/godplan/axes.py \
                 (GateStatus over ProceduralGate's predicates, rules 19.1 and 53)",
    },
    CoverageRow {
        axis: AxisKind::Grade,
        ladder: "Grade",
        values: Grade::TOKENS,
        source: "references/Docs-A/ticket-ledger-engine-prototype/seasons/impl/godplan/axes.py \
                 (Grade, GRADE_ORDER, rule 19.2)",
    },
    CoverageRow {
        axis: AxisKind::OrgRank,
        ladder: "OrgRank",
        values: OrgRank::TOKENS,
        source: "references/Docs-A/ticket-ledger-engine-prototype/seasons/impl/godplan/axes.py \
                 (OrgRank) and seasons/documents/organizations.md rules 101-108",
    },
    CoverageRow {
        axis: AxisKind::SupportLevel,
        ladder: "EvidenceLevel",
        values: EvidenceLevel::TOKENS,
        source: "references/ala-system/src/evidence/support.ts (EvidenceLevel), registered not \
                 redefined",
    },
    CoverageRow {
        axis: AxisKind::TicketEvidence,
        ladder: "EvidenceOrigin",
        values: EvidenceOrigin::TOKENS,
        source: "crates/voxel-model/src/ticket.rs (EvidenceOrigin), from ala-cities::gov::Evidence; \
                 D is this tree's derivation rung, not upstream's",
    },
    CoverageRow {
        axis: AxisKind::ProseKind,
        ladder: "ProseKind",
        values: ProseKind::TOKENS,
        source: "agent_god_files/prose_logic/src/claim.rs (AtomKind), eight labels with no order",
    },
    CoverageRow {
        axis: AxisKind::Establishment,
        ladder: "Establishment",
        values: Establishment::TOKENS,
        source: "crates/voxel-model/src/axis.rs (Establishment), moved from voxel-package's \
                 features.rs; the census's five cells already carried these states",
    },
];

/// The table is as wide as the axis set. **Compile-checked, not tested** — a new
/// axis without a coverage row does not compile. `pub` and deliberately: a
/// private constant whose *type* is the check is dead code, and dead code is
/// what a tidy-up deletes. The value is meaningless; the **type** is the
/// assertion.
pub const COVERAGE_IS_TOTAL: [(); AXIS_COUNT] = [(); COVERAGE.len()];

/// Check a coverage table for the three ways it can lie: an empty row, a row
/// that misdescribes its ladder (the anti-fabrication check), and a duplicated
/// or missing axis. Returns every problem at once.
pub fn check_coverage(rows: &[CoverageRow]) -> Result<(), RefusalSet> {
    let mut refusals = RefusalSet::new();

    for row in rows {
        if row.values.is_empty() {
            refusals.refuse(
                RefusalKind::UncoveredAxis,
                format!("{}: a coverage row that declares no values describes no ladder", row.axis),
            );
            continue;
        }
        let declared = row.axis.declared_values();
        if declared != row.values {
            refusals.refuse(
                RefusalKind::UncoveredAxis,
                format!(
                    "{} ({}): the row claims {:?} but the ladder's members are {:?}; a row that \
                     does not describe its ladder is worse than no row, because it looks checked",
                    row.axis, row.ladder, row.values, declared
                ),
            );
        }
        let mut distinct: Vec<&str> = row.values.to_vec();
        distinct.sort_unstable();
        distinct.dedup();
        if distinct.len() != row.values.len() {
            refusals.refuse(
                RefusalKind::UncoveredAxis,
                format!("{}: a coverage row lists the same token twice", row.axis),
            );
        }
    }

    for axis in AxisKind::ALL {
        if !rows.iter().any(|row| row.axis == axis) {
            refusals.refuse(
                RefusalKind::UncoveredAxis,
                format!("{}: this axis has no coverage row, so nothing asserts its members", axis),
            );
        }
    }

    let mut axes: Vec<AxisKind> = rows.iter().map(|row| row.axis).collect();
    axes.sort_unstable();
    axes.dedup();
    if axes.len() != rows.len() {
        refusals.refuse(
            RefusalKind::UncoveredAxis,
            "coverage table: two rows claim one axis, so one of them describes what the other \
             already describes",
        );
    }

    if refusals.is_empty() {
        Ok(())
    } else {
        Err(refusals)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- rule 51: cross-axis comparison refuses -----------------------------

    #[test]
    fn a_support_level_may_not_be_compared_to_an_evidence_grade() {
        let refused = refuse_interchange(EvidenceLevel::Direct, EvidenceGrade::A)
            .expect_err("support level and evidence grade are different ladders");
        assert_eq!(refused.kind(), RefusalKind::AxisInterchangeRefused);
        assert!(refused.detail().contains("support_level"), "got {refused}");
        assert!(refused.detail().contains("evidence_grade"), "got {refused}");
    }

    #[test]
    fn an_origin_letter_may_not_be_compared_to_a_unit_grade() {
        // `B` is "read back from world state" and `B` is also a unit grade.
        let refused = refuse_interchange(EvidenceOrigin::ReadBack, Grade::B)
            .expect_err("ticket evidence and unit grade are different ladders");
        assert_eq!(refused.kind(), RefusalKind::AxisInterchangeRefused);
    }

    #[test]
    fn two_members_of_one_ladder_compare_freely() {
        assert!(refuse_interchange(EvidenceGrade::A, EvidenceGrade::D).is_ok());
    }

    #[test]
    fn the_two_a_values_point_in_opposite_directions() {
        assert_eq!(EvidenceGrade::A.strength(), 4, "evidence grade A is strongest");
        assert_eq!(Grade::A.rank(), 5, "unit grade A sits below S");
        assert!(Grade::S.rank() > Grade::A.rank());
    }

    // -- rule 52: certification is a threshold, never a maximum -------------

    #[test]
    fn no_requirements_holds_no_tier_rather_than_a_floor() {
        // §17 rule 4: absence of certification is a state, not bronze.
        assert_eq!(certification_tier(&[]), None);
    }

    #[test]
    fn the_bands_are_a_threshold_ladder_over_the_whole_vector() {
        assert_eq!(
            certification_tier(&["declared_provenance"]),
            Some(CertificationTier::Bronze)
        );
        assert_eq!(
            certification_tier(&["declared_provenance", "evidence_present"]),
            Some(CertificationTier::Silver)
        );
        // A later band satisfied without an earlier one holds nothing: the
        // evaluation is a threshold, not a scorecard.
        assert_eq!(certification_tier(&["gate_cleared", "reconstruction_verified"]), None);
        let all = [
            "declared_provenance",
            "evidence_present",
            "no_open_contradiction",
            "gate_cleared",
            "reconstruction_verified",
        ];
        assert_eq!(certification_tier(&all), Some(CertificationTier::Platinum));
    }

    #[test]
    fn a_maximum_over_a_metric_set_is_refused_with_the_sources_own_example() {
        let refused = prohibited_max_certification();
        assert_eq!(refused.kind(), RefusalKind::AggregationRefused);
        assert!(refused.detail().contains("[A, D, D, D]"), "got {refused}");
    }

    // -- rule 53: a ladder never renders as a number ------------------------

    #[test]
    fn a_percentage_and_a_score_are_refused_for_every_axis() {
        for axis in AxisKind::ALL {
            let percentage = refuse_reduction(axis, Reduction::Percentage);
            assert_eq!(percentage.kind(), RefusalKind::ScalarReductionRefused);
            assert!(percentage.detail().contains("not a quantity"), "got {percentage}");
            let score = refuse_reduction(axis, Reduction::Score);
            assert!(score.detail().contains("not a score"), "got {score}");
        }
    }

    #[test]
    fn no_ladder_is_a_quantity() {
        assert!(AxisKind::ALL.iter().all(|axis| !axis.is_a_quantity()));
    }

    // -- rule 55: a transition needs its destination ------------------------

    #[test]
    fn advance_moves_one_rung_and_graduate_ends_only_at_the_ceiling() {
        let mut unit = Unit::new(Grade::F);
        unit.advance().expect("F advances");
        assert_eq!(unit.grade, Grade::E);
        let premature = unit.graduate().expect_err("E has a destination");
        assert_eq!(premature.kind(), RefusalKind::DestinationInvariantRefused);
        assert!(premature.detail().contains("use advance"), "got {premature}");
    }

    #[test]
    fn advance_at_the_ceiling_refuses_because_no_destination_exists() {
        let mut unit = Unit::new(Grade::SSS);
        let refused = unit.advance().expect_err("SSS has no destination");
        assert!(refused.detail().contains("no destination"), "got {refused}");
        unit.graduate().expect("SSS graduates");
        assert_eq!(unit.status, UnitStatus::Terminal);
        let again = unit.graduate().expect_err("a terminal unit graduates once");
        assert!(again.detail().contains("already-terminal"), "got {again}");
        let moved = unit.advance().expect_err("a terminal unit moves no further");
        assert!(moved.detail().contains("terminal"), "got {moved}");
    }

    #[test]
    fn the_office_ceiling_is_reached_by_election_not_by_iterating() {
        assert_eq!(OrgRank::ALL.len(), 14);
        assert!(OrgRank::ChiefExecutiveOfficer.is_ceo());
        assert_eq!(OrgRank::Member.next().expect("member advances"), OrgRank::SeniorMember);
        let president = OrgRank::President.next().expect_err("president does not advance");
        assert!(president.detail().contains("election"), "got {president}");
        let ceo = OrgRank::ChiefExecutiveOfficer
            .next()
            .expect_err("the ceiling has no destination");
        assert!(ceo.detail().contains("ceiling"), "got {ceo}");
    }

    // -- round 5: a bare ladder token is a bug ------------------------------

    #[test]
    fn a_contested_token_is_ambiguous_and_names_every_claimant() {
        let claimants = bare_token_axes("A");
        assert_eq!(
            claimants,
            vec![
                AxisKind::EvidenceGrade,
                AxisKind::Grade,
                AxisKind::TicketEvidence
            ],
            "A is valid on three ladders here and means three things"
        );
        let refused = refuse_bare("A").expect_err("a contested token may not be read bare");
        assert_eq!(refused.kind(), RefusalKind::BareTokenRefused);
        assert!(refused.detail().contains("AMBIGUOUS"), "got {refused}");
        assert!(!refused.detail().contains("UNQUALIFIED"), "got {refused}");
    }

    #[test]
    fn a_single_claimant_is_unqualified_not_ambiguous() {
        // The distinction the source calls out: `gold` belongs to one ladder and
        // is still a bug bare.
        let refused = refuse_bare("gold").expect_err("a bare ladder token is a bug");
        assert!(refused.detail().contains("UNQUALIFIED"), "got {refused}");
        assert!(!refused.detail().contains("AMBIGUOUS"), "got {refused}");
    }

    #[test]
    fn a_token_no_ladder_claims_is_not_this_rules_business() {
        assert!(refuse_bare("diamond_plus").is_ok());
        assert!(refuse_bare("the").is_ok());
    }

    #[test]
    fn the_contested_set_is_visible_rather_than_resolved() {
        let contested = ambiguous_tokens();
        assert!(contested.contains_key("A"));
        assert!(contested.contains_key("C"), "C is evidence grade, grade and origin");
        assert!(!contested.contains_key("gold"), "gold is unqualified, not contested");
    }

    // -- the coverage gate --------------------------------------------------

    #[test]
    fn the_coverage_table_describes_every_axis_exactly_once() {
        assert_eq!(COVERAGE_IS_TOTAL.len(), AxisKind::ALL.len());
        check_coverage(&COVERAGE).expect("the declared table must describe every axis once");
    }

    #[test]
    fn a_fabricated_row_is_refused_because_it_looks_checked() {
        let forged = CoverageRow {
            axis: AxisKind::EvidenceGrade,
            ladder: "EvidenceGrade",
            values: &["A", "B", "C", "E"],
            source: "invented by a test",
        };
        let refused = check_coverage(&[forged]).expect_err("a forged row must be refused");
        assert!(refused.all()[0].detail().contains("looks checked"), "got {refused}");
    }

    #[test]
    fn a_ladder_with_no_representation_is_a_failure_not_a_todo() {
        let partial: Vec<CoverageRow> = COVERAGE
            .iter()
            .copied()
            .filter(|row| row.axis != AxisKind::OrgRank)
            .collect();
        let refused = check_coverage(&partial).expect_err("a missing row must be refused");
        assert!(refused.all().iter().any(|r| r.detail().contains("org_rank")));
    }

    // -- the registration stays one scale per ladder ------------------------

    #[test]
    fn the_ticket_evidence_ladder_is_the_crates_existing_origin() {
        assert_eq!(
            <EvidenceOrigin as Ladder>::AXIS,
            AxisKind::TicketEvidence
        );
        assert_eq!(
            EvidenceOrigin::ALL.iter().map(|o| o.letter()).collect::<Vec<_>>(),
            EvidenceOrigin::TOKENS
        );
    }

    #[test]
    fn every_ladder_declares_all_of_its_tokens() {
        assert_eq!(EvidenceGrade::ALL.len(), EvidenceGrade::TOKENS.len());
        assert_eq!(CertificationTier::ALL.len(), CertificationTier::TOKENS.len());
        assert_eq!(GateStatus::ALL.len(), GateStatus::TOKENS.len());
        assert_eq!(Grade::ALL.len(), Grade::TOKENS.len());
        assert_eq!(OrgRank::ALL.len(), OrgRank::TOKENS.len());
        assert_eq!(EvidenceLevel::ALL.len(), EvidenceLevel::TOKENS.len());
    }

    #[test]
    fn parsing_refuses_an_unknown_token_rather_than_defaulting() {
        // Whitespace on input is trimmed, as `EvidenceOrigin::parse` trims; a
        // token that does not exist is refused rather than defaulted.
        assert!(EvidenceGrade::parse("E").is_err(), "E is a unit grade");
        assert_eq!(
            CertificationTier::parse(" platinum ").map(|tier| tier.as_str()),
            Ok("platinum")
        );
        assert!(CertificationTier::parse("platinum_plus").is_err());
        assert!(Grade::parse("sss").is_err(), "the tokens are uppercase on purpose");
        assert!(GateStatus::parse("pass").is_err());
    }
}
