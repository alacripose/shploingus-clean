use crate::matter::Rational;
use crate::world::{ActorId, Digest, ReservationId, TargetRef};

use super::PlanId;

pub use crate::world::Evidence;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConsequencePolicy {
    pub owner: ActorId,
    pub consent_granted: bool,
    pub compensation: Rational,
}

impl ConsequencePolicy {
    pub fn fixture(owner: ActorId, compensation: Rational) -> Self {
        Self {
            owner,
            consent_granted: true,
            compensation,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkConsequence {
    pub target: TargetRef,
    pub owner: ActorId,
    pub consent_granted: bool,
    pub compensation: Rational,
}

impl WorkConsequence {
    pub fn from_policy(target: TargetRef, policy: &ConsequencePolicy) -> Self {
        Self {
            target,
            owner: policy.owner.clone(),
            consent_granted: policy.consent_granted,
            compensation: policy.compensation,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvidenceQuery {
    Transition(Digest),
    Reservation(ReservationId),
    Plan(PlanId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgencyEvidence {
    pub evidence: Evidence,
    pub plan_id: Option<PlanId>,
}
