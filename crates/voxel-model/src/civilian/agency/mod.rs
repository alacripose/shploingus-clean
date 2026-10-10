//! Persistent citizens, governed embodied work, and typed selection plans.

mod action;
mod actor;
mod evidence;
mod selection;
mod task;

pub use crate::world::{PhysicalVerb, WorldTransition};

pub use action::{
    ActionAttempt, ActionReceipt, AgencyError, AgencyRules, Blocker, GovernedWorld,
    SharedPhysicalVerb, SharedWorldTransition, TaskRequirement, VerbPolicy,
};
pub use actor::{
    ActorError, Citizen, CitizenRole, LifecycleEvent, LifecycleState, Needs, PlayerCitizen, SkillId,
};
pub use evidence::{AgencyEvidence, ConsequencePolicy, Evidence, EvidenceQuery, WorkConsequence};
pub use selection::{
    estimate_selection, estimate_selection_bounded, CuboidSelection, EstimateValue,
    MaterialRequirement, PlanId, PlanState, SelectionBudget, SelectionError, SelectionEstimate,
    SelectionPlan, SelectionPreview,
};
pub use task::{TaskStatus, ToolId, WorkTask};

pub use crate::world::{ActorId, ReservationId, TargetRef, TaskId};
