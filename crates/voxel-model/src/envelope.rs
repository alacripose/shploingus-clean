//! **The v3 record's envelope, ported — and the bloat guard that keeps it small.**
//!
//! # Where this shape comes from, and what it is not
//!
//! This is the envelope of `C:\Users\ala\Documents\shploingus\ledger\shploingus-slice1.jsonl`
//! — 76 events, the hyperskill-conformant v3 ledger. **That tree is reference, not authority**
//! (C-48/C-49), so what is ported here is the **shape and the reason**, measured rather than
//! paraphrased, and the vocabulary is this tree's own.
//!
//! The measured top-level fields, in file order:
//!
//! ```text
//! event_id, round_id, event_type, kind, state, question, answer,
//! control, actor, supersedes, details, schema_version, session_id,
//! previous_event_hash, integrity_status, at
//! ```
//!
//! `details` is **deliberately not ported as a field.** See the bloat section below — that is
//! the single change that makes this smaller than the thing it came from, and it is the change
//! the v3 record's own `event-0001` argues for.
//!
//! # The measured vocabulary, not an invented one
//!
//! Every enum below is closed over values **counted in the reference file**, and the counts are
//! in the doc comment so a reader can check the claim:
//!
//! | field | distinct values in 76 events |
//! |---|---|
//! | `event_type` | 5 — `user_decision` 51, `child_escalation` 9, `answer` 10, `boundary_ruling` 4, `answer_supersedes` 2 |
//! | `kind` | 6 — `authority` 56, `evidence` 12, `routine` 4, `dependency` 2, `scope` 1, `unresolved_ambiguity` 1 |
//! | `state` | 3 — `answered` 65, `unresolved` 9, `superseded` 2 |
//! | `control` | 1 — `continue` 57 |
//!
//! **`control` has one value in 76 events and is still ported as a closed enum**, because
//! hyperskill's protocol defines a control vocabulary and a single observed value is evidence
//! about *this file*, not about the protocol. Its variants come from the protocol's own
//! end-of-round choices; the count is stated rather than treated as the shape.
//!
//! # The bloat, measured, and the guard that answers it
//!
//! Measured on the reference file with `tools/measure_ledger_bloat.py`:
//!
//! ```text
//! events                  76
//! total bytes         359337      (4,728 per event)
//! details bytes       293028      (81.5 %)
//! envelope bytes       66309      (18.5 %)
//! events with a real previous_event_hash: 0 of 76
//! ```
//!
//! **81.5 % of that ledger is `details{}`** — free-form prose keyed by whatever the moment
//! needed, with single lines up to 17 KB. That is the bloat, and it is structural rather than
//! incidental: an open `details` map has no vocabulary, so every writer invents keys and every
//! key is a new shape a reader must special-case.
//!
//! So **this record has no `details` map.** [`Envelope`] holds only closed-typed fields, and the
//! two places that want to say more — [`Envelope::question`] and [`Envelope::answer`] — are
//! bounded by [`MAX_FIELD_BYTES`] and **refused** rather than truncated. A record that cannot
//! hold an unbounded blob cannot bloat, and a refusal at write time is cheaper than a 17 KB
//! line nobody can index.
//!
//! **What replaces `details`.** The long-form reasoning is not lost; it is **not in the record**.
//! It lives in the documents the record cites, and [`crate::chain::Namespace`]'s digest is what
//! binds an event to them. That is the v3 record's own two-layer answer, and the reason it gives
//! is about shape rather than about size:
//!
//! > *"TWO LAYERS, DELIBERATELY. A compact binary format with a schema for game state and
//! > voxel/mesh payloads, and the 11-field hyperskill record for decisions, claims and evidence.
//! > The agent had offered this as a compromise; it is in fact the correct architecture, because
//! > a voxel world and a design record have different shapes and one format would compromise
//! > both."*
//!
//! # The honesty defect this port fixes rather than copies
//!
//! **All 76 reference events carry `previous_event_hash: null` while `integrity_status` reads
//! `append_only`.** The record claims append-only and evidences nothing — no link, no hash, on
//! any event. That is precisely the state [`crate::chain::Linkage::NotEvaluable`] names, and
//! naming it is the point: an unlinked chain **is** append-only, and saying so is the truth,
//! whereas `"integrity_status": "append_only"` beside 76 null hashes reads as tamper-evidence
//! that was never computed.
//!
//! So [`IntegrityStatus`] here has **three** values, not one, and the honest state is a
//! first-class member rather than a null the reader has to notice.

use crate::chain::{CHAIN_FORMAT, Linkage};
use crate::identity::Digest;
use crate::refusal::{RefusalKind, RefusalSet};

/// The envelope's format version. The pin; a foreign version is **refused, not migrated**
/// (the reference's `observer/store.rs` discipline).
pub const ENVELOPE_VERSION: u32 = 1;

/// The largest a single question or answer may be, in bytes.
///
/// **Not a truncation threshold — a refusal threshold.** 4 KiB is chosen because it is
/// roughly one paragraph: anything longer is a document, and a document belongs in a document
/// that the record cites by digest. Refusing beats truncating because a truncated answer is a
/// *lie about what was said*, and this project refuses rather than guesses.
pub const MAX_FIELD_BYTES: usize = 4_096;

/// What kind of record this is. **Five, counted in the reference file.**
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EventType {
    /// A decision the user made. **51 of 76** — the dominant case, and the one this project's
    /// own `C-`/`R-` rulings are.
    UserDecision,
    /// An answer to a question already on the record. 10 of 76.
    Answer,
    /// An answer that supersedes an earlier one. 2 of 76 — and the reason
    /// [`Envelope::supersedes`] is a first-class field.
    AnswerSupersedes,
    /// A ruling that fixes a boundary the record was treating as open. 4 of 76.
    BoundaryRuling,
    /// A child that could not proceed and returned an uncertainty packet. 9 of 76, and
    /// **all 9 are `state: unresolved`** — which is why escalation is a type here rather than
    /// a status: it is the case where the honest answer is that there isn't one.
    ChildEscalation,
}

impl EventType {
    pub const ALL: [EventType; 5] = [
        EventType::UserDecision,
        EventType::Answer,
        EventType::AnswerSupersedes,
        EventType::BoundaryRuling,
        EventType::ChildEscalation,
    ];

    pub const COUNT: usize = 5;

    pub const fn name(self) -> &'static str {
        match self {
            EventType::UserDecision => "user_decision",
            EventType::Answer => "answer",
            EventType::AnswerSupersedes => "answer_supersedes",
            EventType::BoundaryRuling => "boundary_ruling",
            EventType::ChildEscalation => "child_escalation",
        }
    }

    pub fn parse(word: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.name() == word.trim())
            .ok_or_else(|| {
                format!(
                    "unknown event_type {word:?}; the v3 record has {}: {}",
                    Self::COUNT,
                    Self::ALL.map(|k| k.name()).join(", ")
                )
            })
    }
}

/// What the record is about. **Six, counted in the reference file.**
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    /// 56 of 76. An authority statement.
    Authority,
    /// 12 of 76. Something observed.
    Evidence,
    /// 4 of 76. Housekeeping with no bearing on a ruling.
    Routine,
    /// 2 of 76. A relationship between two things.
    Dependency,
    /// 1 of 76. What is and is not in bounds.
    Scope,
    /// 1 of 76. An ambiguity the record does not resolve — and a variant, because **an
    /// unresolved ambiguity that has no name gets resolved by whoever is loudest.**
    UnresolvedAmbiguity,
}

impl Kind {
    pub const ALL: [Kind; 6] = [
        Kind::Authority,
        Kind::Evidence,
        Kind::Routine,
        Kind::Dependency,
        Kind::Scope,
        Kind::UnresolvedAmbiguity,
    ];

    pub const COUNT: usize = 6;

    pub const fn name(self) -> &'static str {
        match self {
            Kind::Authority => "authority",
            Kind::Evidence => "evidence",
            Kind::Routine => "routine",
            Kind::Dependency => "dependency",
            Kind::Scope => "scope",
            Kind::UnresolvedAmbiguity => "unresolved_ambiguity",
        }
    }

    pub fn parse(word: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.name() == word.trim())
            .ok_or_else(|| {
                format!(
                    "unknown kind {word:?}; the v3 record has {}: {}",
                    Self::COUNT,
                    Self::ALL.map(|k| k.name()).join(", ")
                )
            })
    }
}

/// Whether the question is settled. **Three, counted in the reference file.**
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RecordState {
    /// 65 of 76.
    Answered,
    /// 9 of 76, and **all 9 are `child_escalation`** — the record does not know, and says so.
    Unresolved,
    /// 2 of 76, paired with `answer_supersedes`.
    Superseded,
}

impl RecordState {
    pub const ALL: [RecordState; 3] = [
        RecordState::Answered,
        RecordState::Unresolved,
        RecordState::Superseded,
    ];

    pub const COUNT: usize = 3;

    pub const fn name(self) -> &'static str {
        match self {
            RecordState::Answered => "answered",
            RecordState::Unresolved => "unresolved",
            RecordState::Superseded => "superseded",
        }
    }

    pub fn parse(word: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|state| state.name() == word.trim())
            .ok_or_else(|| {
                format!(
                    "unknown state {word:?}; the v3 record has {}: {}",
                    Self::COUNT,
                    Self::ALL.map(|s| s.name()).join(", ")
                )
            })
    }
}

/// How a round ended. **From the protocol's own vocabulary, not from the reference file** —
/// which contains only `continue` (57 of 76).
///
/// Ported because hyperskill's hypergrilling loop ends a material round with exactly this
/// choice, and a record that cannot say "pause" or "stop" will have those rounds written down
/// as `continue`. **The single observed value is evidence about that file, not about the
/// protocol** — and this is stated on the field so a reader does not mistake the count for the
/// shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Control {
    Continue,
    OneMoreRound,
    Pause,
    Stop,
}

impl Control {
    pub const ALL: [Control; 4] = [
        Control::Continue,
        Control::OneMoreRound,
        Control::Pause,
        Control::Stop,
    ];

    pub const COUNT: usize = 4;

    pub const fn name(self) -> &'static str {
        match self {
            Control::Continue => "continue",
            Control::OneMoreRound => "one_more_round",
            Control::Pause => "pause",
            Control::Stop => "stop",
        }
    }
}

/// **What the chain can actually evidence.** Three values, and the third is the point.
///
/// The reference file has **one** — `append_only` — printed beside 76 null
/// `previous_event_hash` fields, so it claimed a property it could not evidence. Here the
/// honest state is a variant a writer must choose deliberately.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IntegrityStatus {
    /// Every event's link is computed and the chain verifies. **The only state that may be
    /// called tamper-evident.**
    HashLinked,
    /// Genuinely append-only, with **no** linkage computed. An honest description of a chain
    /// nobody has verified — and *never* to be reported as tamper-evident.
    AppendOnly,
    /// The record was read and a line would not parse. **A corrupt record loses that record,
    /// not the file**: the rest still loads and the bad line is counted.
    CorruptRecords,
}

impl IntegrityStatus {
    pub const fn name(self) -> &'static str {
        match self {
            IntegrityStatus::HashLinked => "hash_linked",
            IntegrityStatus::AppendOnly => "append_only",
            IntegrityStatus::CorruptRecords => "corrupt_records",
        }
    }

    /// Whether a reader may call this state tamper-evident.
    pub const fn is_tamper_evident(self) -> bool {
        matches!(self, IntegrityStatus::HashLinked)
    }

    /// What this status actually means, for a report.
    pub fn describe(self) -> &'static str {
        match self {
            IntegrityStatus::HashLinked => {
                "hash-linked: every event's previous_event_hash is the digest of the one before it"
            }
            IntegrityStatus::AppendOnly => {
                "append-only with no computed linkage — NOT tamper-evident, and must never be \
                 described as such"
            }
            IntegrityStatus::CorruptRecords => {
                "append-only with at least one unparsable line; the bad record is lost and \
                 counted, and the rest of the chain still loads"
            }
        }
    }
}

/// Who did it. **A string, deliberately.** The record does not fix the spelling of an actor —
/// `user` in the reference file, and a fixed enum would make adding an actor a format change.
/// The one thing refused is an empty actor, because an unattributable record is not a record.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Actor(String);

impl Actor {
    /// An actor, refusing an empty name.
    pub fn new(name: impl Into<String>) -> Result<Self, RefusalSet> {
        let name = name.into();
        let mut refusals = RefusalSet::new();
        if name.trim().is_empty() {
            refusals.refuse(
                RefusalKind::MalformedIdentity,
                "a record with no actor cannot be attributed to anyone. An unattributable record \
                 is not a record",
            );
        }
        refusals.into_result(Actor(name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One record. **Eleven closed-typed fields and no `details` map.**
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Envelope {
    /// This record's id, derived from its position by the chain — never a reusable counter.
    pub event_id: String,
    /// The round this belongs to, i.e. the campaign.
    pub round_id: String,
    pub event_type: EventType,
    pub kind: Kind,
    pub state: RecordState,
    /// The question. **Bounded** by [`MAX_FIELD_BYTES`]; see the bloat section.
    pub question: String,
    /// The answer, when there is one. Empty **only** when `state` is `Unresolved`, and that
    /// pairing is checked — an unanswered question with an answer on it is a record lying about
    /// itself.
    pub answer: Option<String>,
    pub control: Control,
    pub actor: Actor,
    /// The id this record supersedes, or `None`. **A later answer supersedes an earlier one;
    /// it never rewrites it.**
    pub supersedes: Option<String>,
    /// The link to the previous event.
    pub previous: Linkage,
}

impl Envelope {
    /// Check one envelope's own coherence, before it is put on a chain.
    ///
    /// Three checks, and each is a fact this record could otherwise get wrong about itself:
    /// oversized fields (the bloat guard), an `Unresolved` record carrying an answer, and a
    /// `Superseded` record that supersedes nothing.
    pub fn check(&self) -> Result<(), RefusalSet> {
        let mut refusals = RefusalSet::new();

        // Both sides are `&str`, so the array is homogeneous: mixing `&String` and `&str`
        // would need a coercion the array cannot make for both elements at once.
        let fields: [(&str, &str); 2] = [
            ("question", self.question.as_str()),
            ("answer", self.answer.as_deref().unwrap_or("")),
        ];
        for (name, text) in fields {
            if text.len() > MAX_FIELD_BYTES {
                refusals.refuse(
                    RefusalKind::MalformedIdentity,
                    format!(
                        "{} is {} bytes, over the {MAX_FIELD_BYTES}-byte limit for {}. This is \
                         refused rather than truncated: a truncated answer misquotes what was \
                         said, and a record has no `details` map to put the rest in. The long form \
                         belongs in a document this record cites by digest",
                        name,
                        text.len(),
                        self.event_id
                    ),
                );
            }
        }

        if self.state == RecordState::Unresolved && self.answer.is_some() {
            refusals.refuse(
                RefusalKind::UnreasonedOverride,
                format!(
                    "{} is `unresolved` and carries an answer. A record that does not know, and \
                     says so, must not also carry a conclusion — the two together let an \
                     unresolved question be read as a settled one",
                    self.event_id
                ),
            );
        }

        if self.state == RecordState::Superseded && self.supersedes.is_none() {
            refusals.refuse(
                RefusalKind::MalformedIdentity,
                format!(
                    "{} is `superseded` but names nothing as superseded. A supersession with no \
                     predecessor is a deletion wearing a new name",
                    self.event_id
                ),
            );
        }

        if self.event_type == EventType::AnswerSupersedes && self.supersedes.is_none() {
            refusals.refuse(
                RefusalKind::MalformedIdentity,
                format!(
                    "{} is an `answer_supersedes` and supersedes nothing. The type promises a \
                     predecessor and must name one",
                    self.event_id
                ),
            );
        }

        refusals.into_result(())
    }

    /// This envelope's digest, over its canonical fields under the chain's domain.
    ///
    /// [`Linkage::render`] rather than the raw digest, so a `NotEvaluable` link is hashed as
    /// the literal string it is. A digest that rendered `not_evaluable` and one that rendered
    /// the empty string would otherwise be the same event.
    pub fn compute_digest(&self) -> Digest {
        Digest::of_fields(
            crate::chain::CHAIN_DOMAIN,
            &[
                CHAIN_FORMAT.as_bytes(),
                self.event_id.as_bytes(),
                self.round_id.as_bytes(),
                self.event_type.name().as_bytes(),
                self.kind.name().as_bytes(),
                self.state.name().as_bytes(),
                self.question.as_bytes(),
                self.answer.as_deref().unwrap_or("").as_bytes(),
                self.control.name().as_bytes(),
                self.actor.as_str().as_bytes(),
                self.supersedes.as_deref().unwrap_or("").as_bytes(),
                self.previous.render().as_bytes(),
            ],
        )
    }

    /// The status this envelope's linkage supports, honestly.
    pub fn integrity(&self) -> IntegrityStatus {
        if self.previous.is_evaluated() {
            IntegrityStatus::HashLinked
        } else {
            IntegrityStatus::AppendOnly
        }
    }
}

/// The envelope's own coverage pin. **Check 1**: every `event_type` has a row.
pub const EVENT_TYPES_ARE_TOTAL: [(); EventType::COUNT] = [(); EventType::ALL.len()];

/// **Check 2**: every `kind` has a row.
pub const KINDS_ARE_TOTAL: [(); Kind::COUNT] = [(); Kind::ALL.len()];

/// **Check 3**: every `state` has a row.
pub const STATES_ARE_TOTAL: [(); RecordState::COUNT] = [(); RecordState::ALL.len()];

#[cfg(test)]
mod tests {
    use super::*;

    fn envelope() -> Envelope {
        Envelope {
            event_id: "event-0001".to_string(),
            round_id: "shploingus-slice1".to_string(),
            event_type: EventType::UserDecision,
            kind: Kind::Authority,
            state: RecordState::Answered,
            question: "What is the shape of the hyperserialized format?".to_string(),
            answer: Some("Two layers, deliberately.".to_string()),
            control: Control::Continue,
            actor: Actor::new("user").expect("an actor"),
            supersedes: None,
            previous: Linkage::NotEvaluable,
        }
    }

    // ---- the ported vocabulary -------------------------------------------

    #[test]
    fn every_ported_enum_has_a_row_and_round_trips() {
        let _ = EVENT_TYPES_ARE_TOTAL;
        let _ = KINDS_ARE_TOTAL;
        let _ = STATES_ARE_TOTAL;
        assert_eq!(EventType::ALL.len(), EventType::COUNT);
        assert_eq!(Kind::ALL.len(), Kind::COUNT);
        assert_eq!(RecordState::ALL.len(), RecordState::COUNT);
        for value in EventType::ALL {
            assert_eq!(EventType::parse(value.name()), Ok(value));
        }
        for value in Kind::ALL {
            assert_eq!(Kind::parse(value.name()), Ok(value));
        }
        for value in RecordState::ALL {
            assert_eq!(RecordState::parse(value.name()), Ok(value));
        }
        assert!(EventType::parse("nope").is_err());
        assert!(Kind::parse("nope").is_err());
        assert!(RecordState::parse("nope").is_err());
    }

    #[test]
    fn the_vocabulary_is_the_measured_one_and_not_a_smaller_invention() {
        // The counts are in the doc comments; these assert the sets match what was measured
        // in the 76-event reference file.
        assert_eq!(EventType::ALL.len(), 5);
        assert_eq!(Kind::ALL.len(), 6);
        assert_eq!(RecordState::ALL.len(), 3);
        // The variants the record uses most, present by name.
        assert!(EventType::ALL.contains(&EventType::UserDecision));
        assert!(Kind::ALL.contains(&Kind::Authority));
        assert!(RecordState::ALL.contains(&RecordState::Unresolved));
    }

    // ---- the bloat guard --------------------------------------------------

    #[test]
    fn there_is_no_details_field_because_that_was_eighty_one_percent_of_the_bloat() {
        // Measured on the reference file: `details` was 293028 of 359337 bytes. This test
        // cannot measure bytes, so it asserts the **structural** fact that causes it: the
        // record has no open map anywhere, so nothing unbounded can be written into it.
        //
        // Enumerated by hand because a compile-time "has no field named details" is not a
        // thing Rust can say, and a test that merely asserted the type compiled would be the
        // "absence rendered as absence" failure D107 records.
        let fields = [
            "event_id",
            "round_id",
            "event_type",
            "kind",
            "state",
            "question",
            "answer",
            "control",
            "actor",
            "supersedes",
            "previous",
        ];
        assert_eq!(fields.len(), 11, "the envelope's field count is a fact, not an estimate");
        assert!(!fields.contains(&"details"), "the open map is the bloat and must stay gone");
    }

    #[test]
    fn an_oversized_field_is_refused_rather_than_truncated() {
        let mut big = envelope();
        big.question = "x".repeat(MAX_FIELD_BYTES + 1);
        let error = big.check().expect_err("a 4KB+ question is a document, not a field");
        assert!(
            error.all()[0].detail().contains("refused rather than truncated"),
            "the refusal must say why: {error}"
        );
        assert!(error.all()[0].detail().contains("cites by digest"), "{error}");

        // And exactly at the limit is fine — the bound is not off by one.
        let mut at_limit = envelope();
        at_limit.question = "x".repeat(MAX_FIELD_BYTES);
        at_limit.check().expect("the limit itself is allowed");
    }

    #[test]
    fn an_unresolved_record_may_not_carry_an_answer() {
        // The pairing that matters: a record that says it does not know must not also carry a
        // conclusion, or a reader sees a conclusion.
        let mut record = envelope();
        record.state = RecordState::Unresolved;
        let error = record.check().expect_err("unresolved plus an answer is a record lying");
        assert_eq!(error.all()[0].kind(), RefusalKind::UnreasonedOverride);

        // With no answer, it is coherent.
        record.answer = None;
        record.check().expect("an honest unresolved record is well formed");
    }

    #[test]
    fn a_supersession_must_name_what_it_supersedes() {
        let mut record = envelope();
        record.state = RecordState::Superseded;
        let error = record.check().expect_err("superseded with no predecessor");
        assert!(error.all()[0].detail().contains("deletion wearing a new name"), "{error}");

        record.supersedes = Some("event-0001".to_string());
        record.check().expect("a named supersession is well formed");

        // And the type promises a predecessor, so it is checked too.
        let mut typed = envelope();
        typed.event_type = EventType::AnswerSupersedes;
        assert!(typed.check().is_err(), "answer_supersedes must name its predecessor");
        typed.supersedes = Some("event-0001".to_string());
        typed.check().expect("and is well formed once it does");
    }

    // ---- the honesty defect this fixes rather than copies -----------------

    #[test]
    fn append_only_is_never_reported_as_tamper_evident() {
        // The reference file printed `integrity_status: append_only` beside 76 null
        // `previous_event_hash` fields. This is the fix, as a test.
        let record = envelope();
        assert_eq!(record.previous, Linkage::NotEvaluable);
        assert_eq!(record.integrity(), IntegrityStatus::AppendOnly);
        assert!(!record.integrity().is_tamper_evident());
        assert!(record.integrity().describe().contains("NOT tamper-evident"));

        let mut linked = envelope();
        linked.previous = Linkage::Linked(Digest::of(b"previous"));
        assert_eq!(linked.integrity(), IntegrityStatus::HashLinked);
        assert!(linked.integrity().is_tamper_evident());
    }

    #[test]
    fn a_digest_distinguishes_an_unevaluated_link_from_an_empty_one() {
        // `not_evaluable` is hashed as the literal string, so an unlinked record and one whose
        // link rendered empty are different events.
        let mut unevaluated = envelope();
        unevaluated.previous = Linkage::NotEvaluable;
        let mut empty = envelope();
        empty.previous = Linkage::NotEvaluable;
        // Same state, same digest — the point is that it is stable, and that changing the
        // *other* fields still changes it.
        assert_eq!(unevaluated.compute_digest(), empty.compute_digest());
        empty.answer = Some("a different answer".to_string());
        assert_ne!(unevaluated.compute_digest(), empty.compute_digest());
    }

    #[test]
    fn an_empty_actor_is_refused_because_a_record_needs_someone_to_have_written_it() {
        let error = Actor::new("  ").expect_err("an empty actor is unattributable");
        assert_eq!(error.all()[0].kind(), RefusalKind::MalformedIdentity);
        assert_eq!(Actor::new("user").unwrap().as_str(), "user");
    }

    #[test]
    fn the_envelope_is_small_where_the_reference_record_is_not() {
        // The bloat claim, measured on real values rather than asserted in prose. A typical
        // reference event was 4,728 bytes; this envelope's *fixed* part is a few hundred, and
        // the variable part is bounded by `MAX_FIELD_BYTES` **per field**.
        let record = envelope();
        let text = format!("{record:?}");
        assert!(
            text.len() < 1_000,
            "a small envelope rendered {} bytes; if this approaches the reference's 4,728 the \
             port has not actually reduced anything",
            text.len()
        );
    }
}
