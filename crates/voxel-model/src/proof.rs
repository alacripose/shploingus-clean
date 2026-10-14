//! The ticking proof of work: a claim, the bytes it was made from, the ticks it ran over, and the
//! world digest it ended at. A proof is current only inside its declared window; stale is refused,
//! nothing is grandfathered, and the bytes must hash to what the proof names.

use std::path::Path;

use crate::admission::{Admission, AdmissionClass, Grade, admits_with, thresholds};
use crate::axis::AxisKind;
use crate::refusal::{Refusal, RefusalKind, RefusalSet};
use crate::identity::Digest;
use crate::ticket::EvidenceOrigin;

pub const ARTIFACT_SCHEMA: u16 = 1;

pub fn artifact_digest(bytes: &[u8]) -> Digest {
    Digest::of(bytes)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proof {
    pub artifact: String,
    pub artifact_digest: Digest,
    pub claim: String,
    pub origin: EvidenceOrigin,
    pub evidence: String,
    pub tick_span: (u64, u64),
    pub world_digest: Digest,
    pub presenter: String,
    pub ticket: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProofWindow {
    pub current_tick: u64,
    pub max_age_ticks: u64,
}

impl ProofWindow {
    pub fn is_current(&self, proof: &Proof) -> Result<(), Refusal> {
        let (start, end) = proof.tick_span;
        if start > end {
            return Err(Refusal::owned(
                RefusalKind::ChainNotEvaluable,
                format!(
                    "{}: tick span {start}..{end} is inverted, so it never happened",
                    proof.artifact
                ),
            ));
        }
        if end > self.current_tick {
            return Err(Refusal::owned(
                RefusalKind::ChainNotEvaluable,
                format!(
                    "{}: the span ends at {end} and the world stands at {}, so the proof cannot be \
                     backdated past the head",
                    proof.artifact, self.current_tick
                ),
            ));
        }
        let age = self.current_tick - end;
        if age > self.max_age_ticks {
            return Err(Refusal::owned(
                RefusalKind::ChainNotEvaluable,
                format!(
                    "{}: the proof is {age} ticks old and the window is {}, so it is refused rather \
                     than grandfathered",
                    proof.artifact, self.max_age_ticks
                ),
            ));
        }
        Ok(())
    }
}

pub fn verify_bytes(root: &Path, proof: &Proof) -> Result<(), Refusal> {
    let path = root.join(&proof.artifact);
    if !path.is_file() {
        return Err(Refusal::owned(
            RefusalKind::UnsubstantiatedEvidence,
            format!("{}: no evidence yet - the artifact is not on disk", proof.artifact),
        ));
    }
    let bytes = std::fs::read(&path).map_err(|why| {
        Refusal::owned(
            RefusalKind::UnsubstantiatedEvidence,
            format!("{}: the artifact cannot be read: {why}", proof.artifact),
        )
    })?;
    if artifact_digest(&bytes) != proof.artifact_digest {
        return Err(Refusal::owned(
            RefusalKind::DigestMismatch,
            format!(
                "{}: the artifact's bytes do not hash to what the proof names; the proof is refused \
                 rather than shown against bytes it was not made from",
                proof.artifact
            ),
        ));
    }
    Ok(())
}

pub fn admit_proof(
    class: AdmissionClass,
    proof: &Proof,
    grades: &[Grade<'_>],
    root: &Path,
    window: &ProofWindow,
) -> Admission {
    let mut refusals = RefusalSet::new();
    if let Err(refusal) = verify_bytes(root, proof) {
        refusals.push(refusal);
    }
    if let Err(refusal) = window.is_current(proof) {
        refusals.push(refusal);
    }
    if proof.presenter.trim().is_empty() {
        refusals.refuse(
            RefusalKind::UnsubstantiatedEvidence,
            format!(
                "{}: no presenter names who is acting; a presentation with no accountable actor is \
                 refused",
                proof.artifact
            ),
        );
    }
    if grades
        .iter()
        .any(|grade| grade.axis == AxisKind::TicketEvidence)
    {
        refusals.refuse(
            RefusalKind::DigestMismatch,
            format!("{}: the origin is the proof's to carry, not the claimant's to declare", class.name()),
        );
    }
    let mut all: Vec<Grade<'_>> = grades.to_vec();
    all.push(Grade {
        axis: AxisKind::TicketEvidence,
        token: proof.origin.letter(),
    });
    let floors = admits_with(class, thresholds(class), &all);
    for refusal in floors.refusals.all() {
        refusals.push(refusal.clone());
    }
    Admission { class, refusals }
}