//! Refusal — the one rule, as a type.
//!
//! **Refuse, never default.** A value that cannot be validated is refused and
//! named, never silently replaced by a plausible one. A plausible substitute is
//! indistinguishable from a correct one until much later, and by then it has
//! been written into a world.
//!
//! Two properties are load-bearing and both are tested:
//!
//! * every refusal carries **which kind** it is and **what** it refused, so a
//!   caller can tell `NoSigningBackend` from `ForgedSignature` — collapsing
//!   different-in-kind refusals into one is how an unknown publisher becomes
//!   indistinguishable from a forged package;
//! * a [`RefusalSet`] collects **all** the refusals it meets rather than
//!   stopping at the first, because a decoder that can only report the first
//!   problem can only ever be fixed one round-trip at a time.

use core::fmt;
use std::borrow::Cow;

/// Why something was refused.
///
/// The variants are deliberately narrow. A new class of refusal is a new variant
/// and therefore a compile-time change at every `match`, which is the same
/// property the axis gate relies on: a non-exhaustive match cannot be merged.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RefusalKind {
    /// A coordinate left the signed 32-bit space the world is defined over.
    CoordinateOutOfRange,
    /// An index left its level's side (a subvoxel index outside `0..4`, say).
    IndexOutOfRange,
    /// Component ratios did not sum to exactly one.
    RatiosDoNotSumToOne,
    /// A ratio was zero, which would make a component a no-op and a lie.
    ZeroRatio,
    /// A material referenced a component the registry never admitted.
    UnknownComponent,
    /// A material id was used that the registry never admitted.
    UnadmittedMaterial,
    /// A text digest was not `sha256:` followed by 64 lowercase hex digits.
    MalformedIdentity,
    /// A declared stage was absent, which is a blocker and never an omission.
    MissingStage,
    /// A shaper produced a non-finite value (NaN or infinity).
    NonFiniteValue,
    /// A declared range was violated by a value that had a valid range.
    OutOfDeclaredRange,
    /// A shaper of one kind tried to make a change only the other kind may make.
    /// A **material** shaper reached for occupancy, which is a physical shaper's
    /// authority and nobody else's (see `crate::shaper`).
    ShaperKindViolation,
    /// The operation does not exist, and refusing to fake it is the answer.
    ///
    /// Its own kind rather than a reused one, because "this model cannot do that
    /// yet" and "that value is out of range" are different facts: the first is a
    /// gap in the ledger of operations, the second is a bad input, and a caller
    /// that cannot tell them apart will eventually treat a gap as a bad input and
    /// work around it. This is what [`crate::conservation::Mass::checked_sub`]
    /// refuses with while exact subtraction is unbuilt.
    Unimplemented,

    // ---- the signing seam (C-59 R-59.2, R-59.3) ------------------------------
    //
    // These are deliberately **several kinds and not one**. Integrity and
    // authenticity are different facts, and "cannot check" and "checked and the
    // key is unknown" are different facts again. Collapsing any two of them is
    // how an unsigned package comes to look signed, or how an honest unknown
    // publisher comes to look like a forgery.
    //
    // A byte was changed, or the bytes are not the layout this build writes.
    /// **Integrity** failed: a checksum or content digest did not match. A fact
    /// about *corruption*, and deliberately not a statement about signatures — a
    /// reader sent to inspect the signature would find it perfectly good.
    DigestMismatch,
    /// The container carries no signature at all, so nothing in it is
    /// attributable to anyone. An absent signature is not a weak one; it is no
    /// signature.
    UnsignedEnvelope,
    /// A signature names a key this verifier does not hold. **Not** the same as
    /// `SignatureInvalid`: a valid signature by an unknown key is one this
    /// verifier cannot answer for, not one that is wrong.
    UntrustedKey,
    /// A signature that is well formed and by a held key, and is **not** a
    /// signature over these bytes. This is the authenticity failure.
    SignatureInvalid,
    /// A signature named an algorithm this build does not implement, so nothing
    /// was checked. Never collapsed into `SignatureInvalid`: that would send a
    /// reader to inspect a signature this build cannot read.
    UnknownSignatureAlgorithm,
    /// No signing or verifying backend is linked into this build. A missing
    /// instrument is not a pass.
    NoSigningBackend,

    // ---- the conformance oracle (C-59 R-59.5) --------------------------------
    /// The oracle produced no measurement, so nothing it says may be quoted and
    /// nothing is written.
    UncalibratedOracle,
    /// The oracle could not be reached. A missing instrument is not a pass.
    OracleUnreachable,
    /// The oracle read the input and declined to answer. Distinct from
    /// `OracleUnreachable`: this is a reading, and that is not one.
    NoReading,
    /// The oracle read the line as not belonging, and no human reason was
    /// recorded in the same act. The one door through is a recorded reason, and
    /// the reason travels with the finding.
    UnreasonedOverride,

    // ---- the feature evaluators (C-59 R-59.1, R-59.7) ------------------------
    /// A declared feature has no witness: nothing in the tree implements it.
    /// This is what the build prints when it can show what is missing.
    MissingFeature,
    /// A feature row lies about itself: it names a witness that is not there,
    /// duplicates another row, or declares no values. A row that misdescribes
    /// its own feature is worse than no row, because it looks checked.
    FabricatedFeature,

    // ---- the one chain (H-8.1, H-11.7, C-54.1) ---------------------------
    /// The chain does not link: an event's `previous_event_hash` is not the
    /// digest of the event before it. **A fact about bytes and order, and never
    /// about permission** — a hash proves sequence, not authority.
    ChainBroken,
    /// An event claims a `previous_event_hash` of `not_evaluable`, so this chain
    /// is append-only **without tamper-evidence** and must never be described
    /// as tamper-evident. The honesty rule the record makes binding.
    ChainNotEvaluable,
    /// Two processes tried to write one chain. C-54.1 makes one writer a
    /// consequence of "one chain for everything", not an optional discipline.
    ConcurrentWriter,
    /// A record on disk would be overwritten by this write. The writer refuses to
    /// overwrite, ever: an append-only record that can be replaced is not one.
    RecordExists,
    /// One line of the chain could not be parsed. **A corrupt record loses that
    /// record, not the file** — the rest of the chain still folds, and the bad
    /// line is counted and reported rather than skipped.
    CorruptRecord,

    // ---- tickets and governance (UI-SPEC tab 3) --------------------------
    /// A ticket's status is not one of the nine governed values. Carving a tenth
    /// is a change to the record, not to the code that reads it.
    UngovernedStatus,
    /// A ticket claims a terminal state with no retirement record behind it. A
    /// status flip is not a retirement (H-8.1 / the reference's rule).
    UnretiredTicket,
    /// A ticket's evidence is cited but absent, or its grade is claimed and no
    /// evidence supports it. Evidence is on the record or it is nothing.
    UnsubstantiatedEvidence,
    /// A ticket's expectation could not be read back from the world, so the
    /// claim is **unverified** — and closing unverified is required, because
    /// "cannot stall" must never be implemented as "auto-promote to success".
    ExpectationUnread,
}

impl RefusalKind {
    /// A stable, machine-readable name. Never prose, never localised.
    pub const fn name(self) -> &'static str {
        match self {
            RefusalKind::CoordinateOutOfRange => "coordinate_out_of_range",
            RefusalKind::IndexOutOfRange => "index_out_of_range",
            RefusalKind::RatiosDoNotSumToOne => "ratios_do_not_sum_to_one",
            RefusalKind::ZeroRatio => "zero_ratio",
            RefusalKind::UnknownComponent => "unknown_component",
            RefusalKind::UnadmittedMaterial => "unadmitted_material",
            RefusalKind::MalformedIdentity => "malformed_identity",
            RefusalKind::MissingStage => "missing_stage",
            RefusalKind::NonFiniteValue => "non_finite_value",
            RefusalKind::OutOfDeclaredRange => "out_of_declared_range",
            RefusalKind::ShaperKindViolation => "shaper_kind_violation",
            RefusalKind::Unimplemented => "unimplemented",
            RefusalKind::DigestMismatch => "digest_mismatch",
            RefusalKind::UnsignedEnvelope => "unsigned_envelope",
            RefusalKind::UntrustedKey => "untrusted_key",
            RefusalKind::SignatureInvalid => "signature_invalid",
            RefusalKind::UnknownSignatureAlgorithm => "unknown_signature_algorithm",
            RefusalKind::NoSigningBackend => "no_signing_backend",
            RefusalKind::UncalibratedOracle => "uncalibrated_oracle",
            RefusalKind::OracleUnreachable => "oracle_unreachable",
            RefusalKind::NoReading => "no_reading",
            RefusalKind::UnreasonedOverride => "unreasoned_override",
            RefusalKind::MissingFeature => "missing_feature",
            RefusalKind::FabricatedFeature => "fabricated_feature",
            RefusalKind::ChainBroken => "chain_broken",
            RefusalKind::ChainNotEvaluable => "chain_not_evaluable",
            RefusalKind::ConcurrentWriter => "concurrent_writer",
            RefusalKind::RecordExists => "record_exists",
            RefusalKind::CorruptRecord => "corrupt_record",
            RefusalKind::UngovernedStatus => "ungoverned_status",
            RefusalKind::UnretiredTicket => "unretired_ticket",
            RefusalKind::UnsubstantiatedEvidence => "unsubstantiated_evidence",
            RefusalKind::ExpectationUnread => "expectation_unread",
        }
    }
}

/// A single refusal: what kind, and what exactly it refused.
///
/// The detail is a [`Cow`] so that a refusal built in a `const fn` (a coordinate
/// constructor, say) costs nothing, while a refusal that has to name a measured
/// value — the interesting ones — can own its text.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Refusal {
    kind: RefusalKind,
    detail: Cow<'static, str>,
}

impl Refusal {
    /// A refusal whose reason is a compile-time string. `const`, so a `const fn`
    /// constructor can refuse without allocating.
    pub const fn new(kind: RefusalKind, detail: &'static str) -> Self {
        Refusal { kind, detail: Cow::Borrowed(detail) }
    }

    /// A refusal that names a value measured at run time.
    pub fn owned(kind: RefusalKind, detail: impl Into<String>) -> Self {
        Refusal { kind, detail: Cow::Owned(detail.into()) }
    }

    pub const fn kind(&self) -> RefusalKind {
        self.kind
    }

    /// The refusal's reason, naming the value that was refused.
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind.name(), self.detail)
    }
}

impl std::error::Error for Refusal {}

/// Every refusal met on one path, in the order they were met.
///
/// This is not a `Vec<Refusal>` because a `Vec` invites `first()` and the whole
/// point is that the caller sees all of them. The inner field is private and the
/// only way to read it is [`RefusalSet::all`].
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct RefusalSet {
    refusals: Vec<Refusal>,
}

impl RefusalSet {
    pub const fn new() -> Self {
        RefusalSet { refusals: Vec::new() }
    }

    /// Record a refusal and keep going. Returns `&mut Self` so a chain of checks
    /// reads as a list rather than a ladder.
    pub fn push(&mut self, refusal: Refusal) -> &mut Self {
        self.refusals.push(refusal);
        self
    }

    /// Record a refusal built from its parts.
    pub fn refuse(&mut self, kind: RefusalKind, detail: impl Into<String>) -> &mut Self {
        self.push(Refusal::owned(kind, detail))
    }

    /// Fold another set in — how a parent gathers its children's refusals.
    pub fn absorb(&mut self, other: RefusalSet) -> &mut Self {
        self.refusals.extend(other.refusals);
        self
    }

    pub fn all(&self) -> &[Refusal] {
        &self.refusals
    }

    pub fn len(&self) -> usize {
        self.refusals.len()
    }

    pub fn is_empty(&self) -> bool {
        self.refusals.is_empty()
    }

    /// Turn the set into a `Result`, refusing with the whole set as one error.
    pub fn into_result<T>(self, value: T) -> Result<T, RefusalSet> {
        if self.is_empty() {
            Ok(value)
        } else {
            Err(self)
        }
    }

    /// The first refusal, for a caller that genuinely only wants one.
    ///
    /// Named `first_named` rather than `first` so that reaching for it is a
    /// decision rather than a habit.
    pub fn first_named(&self) -> Option<&Refusal> {
        self.refusals.first()
    }
}

impl fmt::Display for RefusalSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} refusal(s): ", self.refusals.len())?;
        for (i, r) in self.refusals.iter().enumerate() {
            if i > 0 {
                write!(f, "; ")?;
            }
            write!(f, "{r}")?;
        }
        Ok(())
    }
}

impl std::error::Error for RefusalSet {}

/// `Result` with a single [`Refusal`].
pub type Refused<T> = Result<T, Refusal>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refusal_names_its_kind_and_its_value() {
        let r = Refusal::owned(RefusalKind::IndexOutOfRange, "subvoxel z = 4 (side 4)");
        assert_eq!(r.kind(), RefusalKind::IndexOutOfRange);
        assert_eq!(r.detail(), "subvoxel z = 4 (side 4)");
        assert_eq!(r.to_string(), "index_out_of_range: subvoxel z = 4 (side 4)");
    }

    #[test]
    fn a_set_collects_every_refusal_rather_than_the_first() {
        // The whole reason RefusalSet exists: three bad values, three refusals.
        let mut set = RefusalSet::new();
        set.refuse(RefusalKind::ZeroRatio, "component 0")
            .refuse(RefusalKind::RatiosDoNotSumToOne, "sum = 1/2")
            .refuse(RefusalKind::UnknownComponent, "component 7");
        assert_eq!(set.len(), 3);
        assert_eq!(set.all()[2].kind(), RefusalKind::UnknownComponent);
    }

    #[test]
    fn an_empty_set_is_ok_and_a_non_empty_set_is_err() {
        let ok: Result<u8, RefusalSet> = RefusalSet::new().into_result(7);
        assert_eq!(ok, Ok(7));

        let mut set = RefusalSet::new();
        set.refuse(RefusalKind::MissingStage, "erosion");
        let err: Result<u8, RefusalSet> = set.into_result(7);
        assert!(err.is_err());
    }

    #[test]
    fn distinct_kinds_are_not_interchangeable() {
        // The D43/D48 lesson in miniature: "cannot verify" and "verified but
        // untrusted" are different in kind, so they are different values.
        assert_ne!(
            RefusalKind::UnknownComponent,
            RefusalKind::UnadmittedMaterial
        );
    }
}
