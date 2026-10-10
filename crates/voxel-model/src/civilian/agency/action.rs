use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::matter::Rational;
// **The governed book, not a world.** `DeltaStore::apply_transition` is the one way a governed
// change commits (`world/governed.rs`), and it cannot be called without a `WritePermission`
// minted from the decision layer's grant. The world itself is the game's — a seed, a tick and
// a configuration composed with the earthworks record — so this machine holds the **book** it
// commits into and nothing else.
use crate::world::{
    Account, ActorId, ComponentId, ConservationSnapshot, Coord, Delta, DeltaStore, Digest,
    EarthworksEffect, Evidence, LineageId, Occupancy, OccupancyProvenance, PhysicalVerb,
    ReservationId, SignedRegion, TargetRef, TaskId, TransitionReceipt, TransitionRefusal,
    WorldError, WorldTransition, WritePermission,
};

use super::{
    AgencyEvidence, Citizen, ConsequencePolicy, CuboidSelection, EstimateValue, EvidenceQuery,
    MaterialRequirement, PlanId, PlanState, SelectionBudget, SelectionError, SelectionEstimate,
    SelectionPlan, SelectionPreview, TaskStatus, ToolId, WorkConsequence, WorkTask,
};

pub use crate::world::{
    PhysicalVerb as SharedPhysicalVerb, WorldTransition as SharedWorldTransition,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionAttempt {
    pub actor: ActorId,
    pub task: Option<TaskId>,
    pub verb: PhysicalVerb,
    pub target: TargetRef,
    pub tool: Option<ToolId>,
    pub epoch: u64,
    pub nonce: u64,
}

impl ActionAttempt {
    pub fn transition(&self, reservation: Option<ReservationId>) -> WorldTransition {
        WorldTransition::new(
            self.actor.clone(),
            self.task.clone(),
            self.verb,
            self.target.clone(),
            reservation,
            self.epoch,
            self.nonce,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionReceipt {
    pub accepted: bool,
    pub blockers: Vec<Blocker>,
    pub reservations: Vec<ReservationId>,
    pub deltas: Vec<Delta>,
    pub evidence: Vec<Evidence>,
    pub consequences: Vec<WorkConsequence>,
    pub transition_id: Digest,
    pub read_back: Option<ConservationSnapshot>,
}

/// **What the governed book did**, as the machine reads it back — the source's `PhysicalResult`,
/// bound to [`TransitionReceipt`] instead of to the city's `World`.
///
/// The receipt already answers every question this shape asks: *was a claim spent* (`reservation`),
/// *how much moved* (`deltas`), *did the books balance* (`conserved`), *who allowed it*
/// (`granted_by`), *what the evidence row says* (`evidence`). So this carries the receipt's own
/// answers rather than a second reading of the book.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PhysicalOutcome {
    pub accepted: bool,
    pub deltas: Vec<Delta>,
    pub evidence: Vec<Evidence>,
    pub read_back: Option<ConservationSnapshot>,
}

impl PhysicalOutcome {
    /// Read a committed transition back. **Accepted is not a parameter:** a receipt that exists
    /// is a change the book committed, and `conserved` is the audit's own reading.
    pub(crate) fn from_receipt(receipt: &TransitionReceipt, effect: &EarthworksEffect) -> Self {
        Self {
            accepted: true,
            deltas: effect.deltas.clone(),
            evidence: vec![receipt.evidence.clone()],
            // The receipt points at the state the change left by digest; the snapshot itself is
            // read by the caller that wants it (`Self::read_back` callers already ask the book),
            // so nothing is reconstructed here that the book can answer.
            read_back: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Blocker {
    ActorMissing,
    ActorInactive,
    CapabilityMissing(PhysicalVerb),
    PolicyMissing,
    TaskRequired,
    ToolRequired,
    UnexpectedTask,
    UnexpectedTool,
    PolicyDrift,
    TaskMissing,
    TaskNotAssigned,
    TaskNotReady,
    SkillMissing,
    AuthorityMissing,
    MaterialNotAdmitted,
    MaterialAdmissionStale,
    InsufficientResources,
    ToolMissing,
    ToolNotOwned,
    ToolNotRequired,
    SiteInvalid,
    SiteOutsideDomain,
    SiteOutsideReach,
    OccupancyMismatch,
    ReservationConflict,
    ConsequenceMissing,
    InvalidConsequence,
    SelectionContractChanged,
    TransitionRefused(TransitionRefusal),
}

impl fmt::Display for Blocker {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ActorMissing => formatter.write_str("actor is not registered"),
            Self::ActorInactive => {
                formatter.write_str("actor lifecycle does not permit participation")
            }
            Self::CapabilityMissing(_) => {
                formatter.write_str("actor role lacks the physical capability")
            }
            Self::PolicyMissing => formatter.write_str("verb policy is not configured"),
            Self::TaskRequired => formatter.write_str("verb policy requires a task"),
            Self::ToolRequired => formatter.write_str("verb policy requires a tool"),
            Self::UnexpectedTask => formatter.write_str("task is not allowed by the verb policy"),
            Self::UnexpectedTool => formatter.write_str("tool is not allowed by the verb policy"),
            Self::PolicyDrift => formatter.write_str("verb policy changed after planning"),
            Self::TaskMissing => formatter.write_str("task is not registered"),
            Self::TaskNotAssigned => formatter.write_str("task is assigned to another actor"),
            Self::TaskNotReady => formatter.write_str("task is not ready"),
            Self::SkillMissing => formatter.write_str("actor lacks the required task skill"),
            Self::AuthorityMissing => formatter.write_str("actor has no verb authority"),
            Self::MaterialNotAdmitted => {
                formatter.write_str("material has no draft fixture admission")
            }
            Self::MaterialAdmissionStale => {
                formatter.write_str("draft material admission is stale")
            }
            Self::InsufficientResources => {
                formatter.write_str("aggregate material is insufficient")
            }
            Self::ToolMissing => formatter.write_str("tool is not registered"),
            Self::ToolNotOwned => formatter.write_str("tool is owned by another actor"),
            Self::ToolNotRequired => formatter.write_str("attempt tool does not match the task"),
            Self::SiteInvalid => formatter.write_str("action target is not a voxel site"),
            Self::SiteOutsideDomain => {
                formatter.write_str("action target is outside the world domain")
            }
            Self::SiteOutsideReach => formatter.write_str("action target is outside actor reach"),
            Self::OccupancyMismatch => {
                formatter.write_str("target occupancy does not support the verb")
            }
            Self::ReservationConflict => {
                formatter.write_str("target already has an active reservation")
            }
            Self::ConsequenceMissing => {
                formatter.write_str("ownership consequence is not declared")
            }
            Self::InvalidConsequence => formatter.write_str("ownership consequence is invalid"),
            Self::SelectionContractChanged => {
                formatter.write_str("selection prerequisites changed after planning")
            }
            Self::TransitionRefused(refusal) => {
                write!(formatter, "physical transition refused: {refusal}")
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AgencyRules {
    pub site_reach: u32,
    pub max_selection_cells: u64,
    pub max_selection_work: u64,
}

impl Default for AgencyRules {
    fn default() -> Self {
        Self {
            site_reach: 1,
            max_selection_cells: 4_096,
            max_selection_work: 8_192,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskRequirement {
    NoTask,
    Required { task: TaskId, tool: Option<ToolId> },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerbPolicy {
    pub task: TaskRequirement,
    pub work_per_cell: u64,
}

impl VerbPolicy {
    pub const fn new(task: TaskRequirement, work_per_cell: u64) -> Self {
        Self {
            task,
            work_per_cell,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DraftMaterialAdmission {
    coord: Coord,
    component: ComponentId,
    snapshot_digest: Digest,
    account: Account,
    occupancy: Occupancy,
    provenance: OccupancyProvenance,
    amount: Rational,
}

#[derive(Debug)]
pub struct GovernedWorld {
    /// **The governed book a change commits into**, and the permission that lets it. Both are
    /// held here rather than passed per call, because the commit point is the one place a
    /// change happens and a caller that could forget the permission could write a world.
    store: DeltaStore,
    /// What the decision layer permitted, minted from the grant `policy::decide` returned.
    /// `None` means nothing may commit — the deny-by-default state, not an oversight.
    permission: Option<WritePermission>,
    region: SignedRegion,
    rules: AgencyRules,
    actors: BTreeMap<ActorId, Citizen>,
    tasks: BTreeMap<TaskId, WorkTask>,
    tools: BTreeMap<ToolId, ActorId>,
    authorities: BTreeSet<(ActorId, PhysicalVerb)>,
    verb_policies: BTreeMap<(ActorId, PhysicalVerb), VerbPolicy>,
    draft_materials: BTreeMap<TargetRef, DraftMaterialAdmission>,
    selection_risks: BTreeMap<(TargetRef, PhysicalVerb), Rational>,
    consequences: BTreeMap<(TargetRef, PhysicalVerb), ConsequencePolicy>,
    plans: BTreeMap<PlanId, SelectionPlan>,
    planned_targets: BTreeMap<TargetRef, PlanId>,
    evidence: Vec<AgencyEvidence>,
    next_plan: u64,
    #[cfg(debug_assertions)]
    selection_batch_refusal_for_test: bool,
}

impl GovernedWorld {
    /// A machine over the book the caller read, with nothing permitted yet.
    ///
    /// **`permission` starts absent and that is the point:** the decision layer's matrix is
    /// consulted by the caller (`voxel_package::policy::decide(Action::WriteWorld, grant)`) and
    /// the grant it returns is handed in through [`Self::permit`]. A machine constructed with a
    /// permission would be a machine whose authority was assumed at construction.
    pub fn new(store: DeltaStore, region: SignedRegion, rules: AgencyRules) -> Self {
        Self {
            store,
            permission: None,
            region,
            rules,
            actors: BTreeMap::new(),
            tasks: BTreeMap::new(),
            tools: BTreeMap::new(),
            authorities: BTreeSet::new(),
            verb_policies: BTreeMap::new(),
            draft_materials: BTreeMap::new(),
            selection_risks: BTreeMap::new(),
            consequences: BTreeMap::new(),
            plans: BTreeMap::new(),
            planned_targets: BTreeMap::new(),
            evidence: Vec::new(),
            next_plan: 1,
            #[cfg(debug_assertions)]
            selection_batch_refusal_for_test: false,
        }
    }

    /// Hand the machine the decision layer's permission. The grant was minted above this crate
    /// — `world/governed.rs` states why the matrix is not restated here.
    pub fn permit(&mut self, permission: WritePermission) {
        self.permission = Some(permission);
    }

    /// The book, for a reader. **Read-only:** a caller that wants a change goes through
    /// [`Self::apply_action`] or [`Self::execute_plan`], which are the commit points.
    pub fn store(&self) -> &DeltaStore {
        &self.store
    }

    pub fn register_actor(&mut self, actor: Citizen) -> Result<(), AgencyError> {
        actor.validate()?;
        if self.actors.contains_key(actor.id()) {
            return Err(AgencyError::DuplicateActor(actor.id().clone()));
        }
        self.actors.insert(actor.id().clone(), actor);
        Ok(())
    }

    pub fn actor(&self, id: &ActorId) -> Option<&Citizen> {
        self.actors.get(id)
    }

    pub fn retire_actor(&mut self, id: &ActorId, reason: &str) -> Result<(), AgencyError> {
        self.actor_mut(id)?.retire(reason)?;
        Ok(())
    }

    pub fn kill_actor(&mut self, id: &ActorId, reason: &str) -> Result<(), AgencyError> {
        self.actor_mut(id)?.die(reason)?;
        Ok(())
    }

    pub fn reenter_actor(&mut self, id: &ActorId, reason: &str) -> Result<(), AgencyError> {
        self.actor_mut(id)?.reenter(reason)?;
        Ok(())
    }

    pub fn move_actor(&mut self, id: &ActorId, position: Coord) -> Result<(), AgencyError> {
        self.actor_mut(id)?.move_to(position)?;
        Ok(())
    }

    fn actor_mut(&mut self, id: &ActorId) -> Result<&mut Citizen, AgencyError> {
        self.actors
            .get_mut(id)
            .ok_or_else(|| AgencyError::ActorNotFound(id.clone()))
    }

    pub fn register_task(&mut self, task: WorkTask) -> Result<(), AgencyError> {
        if self.tasks.contains_key(&task.id) {
            return Err(AgencyError::DuplicateTask(task.id));
        }
        if task.minimum_skill.is_negative() {
            return Err(AgencyError::InvalidTask);
        }
        self.tasks.insert(task.id.clone(), task);
        Ok(())
    }

    pub fn register_tool(&mut self, tool: ToolId, owner: ActorId) -> Result<(), AgencyError> {
        if !self.actors.contains_key(&owner) {
            return Err(AgencyError::ActorNotFound(owner));
        }
        self.tools.insert(tool, owner);
        Ok(())
    }

    pub fn grant_authority(
        &mut self,
        actor: ActorId,
        verb: PhysicalVerb,
    ) -> Result<(), AgencyError> {
        if !self.actors.contains_key(&actor) {
            return Err(AgencyError::ActorNotFound(actor));
        }
        self.authorities.insert((actor, verb));
        Ok(())
    }

    pub fn revoke_authority(
        &mut self,
        actor: ActorId,
        verb: PhysicalVerb,
    ) -> Result<(), AgencyError> {
        if !self.actors.contains_key(&actor) {
            return Err(AgencyError::ActorNotFound(actor));
        }
        self.authorities.remove(&(actor, verb));
        Ok(())
    }

    pub fn set_verb_policy(
        &mut self,
        actor: ActorId,
        verb: PhysicalVerb,
        policy: VerbPolicy,
    ) -> Result<(), AgencyError> {
        if !self.actors.contains_key(&actor) {
            return Err(AgencyError::ActorNotFound(actor));
        }
        if policy.work_per_cell == 0 {
            return Err(AgencyError::InvalidVerbPolicy);
        }
        self.verb_policies.insert((actor, verb), policy);
        Ok(())
    }

    #[cfg(debug_assertions)]
    #[doc(hidden)]
    pub fn refuse_next_selection_batch_for_test(&mut self) {
        self.selection_batch_refusal_for_test = true;
    }

    pub fn admit_draft_material(
        &mut self,
        target: TargetRef,
        component: ComponentId,
    ) -> Result<(), AgencyError> {
        let coord = target_coord(&target).ok_or(AgencyError::InvalidMaterialFixture)?;
        let snapshot = self.store.conservation_snapshot()?;
        let provenance = snapshot
            .occupancy_provenance
            .get(&coord)
            .cloned()
            .ok_or(AgencyError::InvalidMaterialFixture)?;
        let occupancy = snapshot
            .occupancy
            .get(&coord)
            .copied()
            .ok_or(AgencyError::InvalidMaterialFixture)?;
        let amount = *snapshot
            .component_accounts
            .get(&(provenance.account.clone(), component.clone()))
            .ok_or(AgencyError::InvalidMaterialFixture)?;
        self.draft_materials.insert(
            target,
            DraftMaterialAdmission {
                coord,
                component,
                snapshot_digest: snapshot.digest,
                account: provenance.account.clone(),
                occupancy,
                provenance,
                amount,
            },
        );
        Ok(())
    }

    pub fn register_selection_risk(
        &mut self,
        target: TargetRef,
        verb: PhysicalVerb,
        risk: Rational,
    ) -> Result<(), AgencyError> {
        if risk.is_negative() {
            return Err(AgencyError::InvalidRiskFixture);
        }
        self.selection_risks.insert((target, verb), risk);
        Ok(())
    }

    pub fn set_consequence(
        &mut self,
        target: TargetRef,
        verb: PhysicalVerb,
        consequence: ConsequencePolicy,
    ) -> Result<(), AgencyError> {
        if consequence.compensation.is_negative() {
            return Err(AgencyError::InvalidConsequence);
        }
        if !self.actors.contains_key(&consequence.owner) {
            return Err(AgencyError::ActorNotFound(consequence.owner));
        }
        self.consequences.insert((target, verb), consequence);
        Ok(())
    }

    pub fn conservation_snapshot(&self) -> Result<ConservationSnapshot, WorldError> {
        self.store.conservation_snapshot()
    }

    pub fn projection_digest(&self) -> Result<Digest, WorldError> {
        self.store
            .conservation_snapshot()
            .map(|snapshot| snapshot.digest)
    }

    pub fn apply_action(&mut self, attempt: ActionAttempt) -> Result<ActionReceipt, WorldError> {
        let before = self.store.conservation_snapshot()?;
        let transition_id = attempt.transition(None).id();
        let blockers = self.preflight(&attempt, None, &before);
        if !blockers.is_empty() {
            return Ok(self.refused_receipt(before, transition_id, blockers, None, None));
        }
        let reservation = ReservationId::new(format!(
            "agency:{}:{}:{}",
            attempt.actor.as_str(),
            attempt.epoch,
            attempt.nonce
        ))
        .map_err(|error| WorldError::InvalidTransition {
            reason: error.to_string(),
        })?;
        if self.store.has_reservation_for(&attempt.target)
            || self
                .store
                .reserve(
                    reservation.clone(),
                    attempt.actor.clone(),
                    attempt.target.clone(),
                )
                .is_err()
        {
            return Ok(self.refused_receipt(
                before,
                transition_id,
                vec![Blocker::ReservationConflict],
                None,
                None,
            ));
        }
        let transition = attempt.transition(Some(reservation.clone()));
        let transition_id = transition.id();
        let Some(permission) = self.permission.clone() else {
            let _ = self.store.cancel_reservation(&reservation);
            return Ok(self.refused_receipt(
                before,
                transition_id,
                vec![Blocker::AuthorityMissing],
                Some(reservation),
                None,
            ));
        };
        // **The machine forms the effect; the book commits it.** `EarthworksEffect::excavation`
        // is the governed vocabulary for a dig/gather/break, and the receipt it returns is the
        // record of what happened — never a boolean invented here.
        let effect = self.effect_for(&attempt, &transition_id)?;
        let receipt = match self.store.apply_transition(&transition, &effect, &permission) {
            Ok(receipt) => receipt,
            Err(refusal) => {
                let _ = self.store.cancel_reservation(&reservation);
                return Ok(self.refused_receipt(
                    before,
                    transition_id,
                    vec![Blocker::TransitionRefused(refusal)],
                    Some(reservation),
                    None,
                ));
            }
        };
        let physical = PhysicalOutcome::from_receipt(&receipt, &effect);
        let consequence = self
            .consequences
            .get(&(attempt.target.clone(), attempt.verb))
            .map(|policy| WorkConsequence::from_policy(attempt.target.clone(), policy));
        let consequences = consequence.into_iter().collect::<Vec<_>>();
        let mut receipt_evidence = physical.evidence.clone();
        receipt_evidence.push(Evidence::for_transition(
            "action.accepted",
            format!("{} completed at work boundary", attempt.verb),
            transition_id,
            Some(reservation.clone()),
        ));
        // The observer's row is the one the game drains to mint proof of work, so it carries
        // the receipt's own identity and the deltas the book actually committed.
        let delta_lines: Vec<String> = physical
            .deltas
            .iter()
            .map(|delta| format!("{delta:?}"))
            .collect();
        let evidence_lines: Vec<String> = receipt_evidence
            .iter()
            .map(|row| format!("{}: {}", row.kind, row.message))
            .collect();
        for row in &receipt_evidence {
            self.record_evidence(row.clone(), None);
        }
        for consequence in &consequences {
            let row = Evidence::for_transition(
                "consequence.recorded",
                format!(
                    "target={}; owner={}; consent={}; compensation={}/{}",
                    consequence.target.as_str(),
                    consequence.owner.as_str(),
                    consequence.consent_granted,
                    consequence.compensation.numerator(),
                    consequence.compensation.denominator()
                ),
                transition_id,
                Some(reservation.clone()),
            );
            receipt_evidence.push(row.clone());
            self.record_evidence(row, None);
        }
        let receipt = ActionReceipt {
            accepted: true,
            blockers: Vec::new(),
            reservations: vec![reservation.clone()],
            deltas: physical.deltas.clone(),
            evidence: receipt_evidence.clone(),
            consequences,
            transition_id,
            read_back: physical.read_back,
        };

        // A committed governed action is recorded from the real receipt, at the single
        // commit point, so no test can omit recording a real action by forgetting to opt
        // in. Built from `physical.deltas` rather than from anything the caller supplied.
        //
        // Only accepted actions are recorded. A refusal has no delta and no authority, so
        // recording it as a ticket would put a non-event into the authority-bearing class.
        crate::observer::recorder::record_committed_action(
            attempt.verb.as_str(),
            attempt.actor.as_str(),
            attempt
                .task
                .as_ref()
                .map(|t| t.as_str())
                .unwrap_or("-"),
            Some(reservation.as_str().to_owned()),
            &delta_lines,
            &evidence_lines,
            receipt.transition_id.to_hex(),
        );

        Ok(receipt)
    }

    fn preflight(
        &self,
        attempt: &ActionAttempt,
        allowed_reservation: Option<&ReservationId>,
        snapshot: &ConservationSnapshot,
    ) -> Vec<Blocker> {
        let mut blockers = Vec::new();
        let Some(actor) = self.actors.get(&attempt.actor) else {
            blockers.push(Blocker::ActorMissing);
            return blockers;
        };
        if !actor.can_participate() {
            blockers.push(Blocker::ActorInactive);
        }
        if !actor.supports(attempt.verb) {
            blockers.push(Blocker::CapabilityMissing(attempt.verb));
        }
        match self
            .verb_policies
            .get(&(attempt.actor.clone(), attempt.verb))
        {
            None => blockers.push(Blocker::PolicyMissing),
            Some(policy) => match &policy.task {
                TaskRequirement::Required { task, tool } => {
                    if attempt.task.as_ref() != Some(task) {
                        blockers.push(Blocker::TaskRequired);
                    }
                    if attempt.tool != *tool {
                        blockers.push(if tool.is_some() {
                            Blocker::ToolRequired
                        } else {
                            Blocker::UnexpectedTool
                        });
                    }
                }
                TaskRequirement::NoTask => {
                    if attempt.task.is_some() {
                        blockers.push(Blocker::UnexpectedTask);
                    }
                    if attempt.tool.is_some() {
                        blockers.push(Blocker::UnexpectedTool);
                    }
                }
            },
        }
        self.check_task_tool(attempt, actor, &mut blockers);
        if !self
            .authorities
            .contains(&(attempt.actor.clone(), attempt.verb))
        {
            blockers.push(Blocker::AuthorityMissing);
        }
        if material_sensitive(attempt.verb) {
            match self.draft_materials.get(&attempt.target) {
                None => blockers.push(Blocker::MaterialNotAdmitted),
                Some(admission) if !material_current(admission, snapshot) => {
                    blockers.push(Blocker::MaterialAdmissionStale)
                }
                Some(_) => {}
            }
        }
        if !self
            .consequences
            .contains_key(&(attempt.target.clone(), attempt.verb))
        {
            blockers.push(Blocker::ConsequenceMissing);
        }
        if let Some(tool) = &attempt.tool {
            match self.tools.get(tool) {
                None => blockers.push(Blocker::ToolMissing),
                Some(owner) if owner != &attempt.actor => blockers.push(Blocker::ToolNotOwned),
                Some(_) => {}
            }
        }
        let Some(coord) = target_coord(&attempt.target) else {
            blockers.push(Blocker::SiteInvalid);
            return blockers;
        };
        if !self.region.contains(coord) {
            blockers.push(Blocker::SiteOutsideDomain);
        }
        if manhattan(actor.position(), coord) > i64::from(self.rules.site_reach) {
            blockers.push(Blocker::SiteOutsideReach);
        }
        let occupancy = snapshot
            .occupancy
            .get(&coord)
            .copied()
            .unwrap_or_else(Occupancy::empty);
        if !occupancy_matches(attempt.verb, occupancy) {
            blockers.push(Blocker::OccupancyMismatch);
        }
        if matches!(attempt.verb, PhysicalVerb::Fill | PhysicalVerb::Place) {
            if let Some(admission) = self.draft_materials.get(&attempt.target) {
                let source = Account::new(format!("carried:{}", attempt.actor.as_str()))
                    .expect("actor id is a valid account suffix");
                let available = snapshot
                    .component_accounts
                    .get(&(source, admission.component.clone()))
                    .copied()
                    .unwrap_or(Rational::ZERO);
                if matches!(
                    available.try_cmp(&Rational::from_integer(1)),
                    Ok(std::cmp::Ordering::Less)
                ) {
                    blockers.push(Blocker::InsufficientResources);
                }
            }
        }
        if self.store.has_reservation_for(&attempt.target)
            || (allowed_reservation.is_none() && self.planned_targets.contains_key(&attempt.target))
        {
            blockers.push(Blocker::ReservationConflict);
        }
        blockers
    }

    fn check_task_tool(
        &self,
        attempt: &ActionAttempt,
        actor: &Citizen,
        blockers: &mut Vec<Blocker>,
    ) {
        if let Some(task_id) = &attempt.task {
            let Some(task) = self.tasks.get(task_id) else {
                blockers.push(Blocker::TaskMissing);
                return;
            };
            if task.actor != *actor.id() {
                blockers.push(Blocker::TaskNotAssigned);
            }
            if task.status != TaskStatus::Ready {
                blockers.push(Blocker::TaskNotReady);
            }
            if let Some(skill) = &task.required_skill {
                if actor.skills().get(skill).is_none_or(|value| {
                    !matches!(
                        value.try_cmp(&task.minimum_skill),
                        Ok(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
                    )
                }) {
                    blockers.push(Blocker::SkillMissing);
                }
            }
            if task.required_tool != attempt.tool {
                blockers.push(Blocker::ToolNotRequired);
            }
        }
    }

    pub fn estimate_selection(
        &self,
        selection: CuboidSelection,
        verb: PhysicalVerb,
    ) -> Result<SelectionEstimate, AgencyError> {
        let actor = self
            .actors
            .keys()
            .next()
            .cloned()
            .ok_or(AgencyError::ActorNotFound(ActorId::new("estimate-actor")?))?;
        self.estimate_for_actor(actor, selection, verb)
    }

    fn estimate_for_actor(
        &self,
        actor: ActorId,
        selection: CuboidSelection,
        verb: PhysicalVerb,
    ) -> Result<SelectionEstimate, AgencyError> {
        if !self.region.contains(selection.min) {
            return Err(SelectionError::OutsideDomain(selection.min).into());
        }
        if !self.region.contains(selection.max) {
            return Err(SelectionError::OutsideDomain(selection.max).into());
        }
        let policy = self
            .verb_policies
            .get(&(actor.clone(), verb))
            .cloned()
            .ok_or(AgencyError::Refused {
                blockers: vec![Blocker::PolicyMissing],
            })?;
        let mut estimate = super::selection::estimate_selection_bounded(
            &self.store,
            selection,
            verb,
            SelectionBudget {
                max_cells: self.rules.max_selection_cells,
                max_work: self.rules.max_selection_work,
                work_per_cell: policy.work_per_cell,
            },
        )?;
        if material_sensitive(verb) {
            let snapshot = self.store.conservation_snapshot()?;
            for target in &estimate.targets {
                let target_ref = TargetRef::voxel(*target);
                match self.draft_materials.get(&target_ref) {
                    None => {
                        return Err(AgencyError::Refused {
                            blockers: vec![Blocker::MaterialNotAdmitted],
                        })
                    }
                    Some(admission) if !material_current(admission, &snapshot) => {
                        return Err(AgencyError::Refused {
                            blockers: vec![Blocker::MaterialAdmissionStale],
                        })
                    }
                    Some(_) => {}
                }
            }
        }
        if matches!(estimate.fluid, EstimateValue::Unavailable(_)) {
            return Err(SelectionError::EstimateUnavailable("target fluid provenance").into());
        }
        let mut risk = Rational::ZERO;
        for target in &estimate.targets {
            let target_ref = TargetRef::voxel(*target);
            let risk_value = self
                .selection_risks
                .get(&(target_ref, verb))
                .copied()
                .ok_or(SelectionError::EstimateUnavailable("draft risk source"))?;
            risk = risk
                .checked_add(&risk_value)
                .ok_or(SelectionError::ArithmeticOverflow)?;
        }
        estimate.risk = EstimateValue::Known(risk);
        Ok(estimate)
    }

    pub fn create_selection_plan(
        &mut self,
        actor: ActorId,
        selection: CuboidSelection,
        verb: PhysicalVerb,
    ) -> Result<SelectionPlan, AgencyError> {
        let estimate = self.estimate_for_actor(actor.clone(), selection, verb)?;
        let policy = self
            .verb_policies
            .get(&(actor.clone(), verb))
            .cloned()
            .ok_or(AgencyError::Refused {
                blockers: vec![Blocker::PolicyMissing],
            })?;
        let (task, tool) = match &policy.task {
            TaskRequirement::NoTask => (None, None),
            TaskRequirement::Required { task, tool } => (Some(task.clone()), tool.clone()),
        };
        let snapshot = self.store.conservation_snapshot()?;
        let mut blockers = Vec::new();
        let mut component = None;
        for target in &estimate.targets {
            if !self.region.contains(*target) {
                return Err(SelectionError::OutsideDomain(*target).into());
            }
            let target_ref = TargetRef::voxel(*target);
            if self.planned_targets.contains_key(&target_ref) {
                blockers.push(Blocker::ReservationConflict);
            }
            let attempt = ActionAttempt {
                actor: actor.clone(),
                task: task.clone(),
                verb,
                target: target_ref.clone(),
                tool: tool.clone(),
                epoch: 0,
                nonce: self.next_plan,
            };
            let target_blockers = self.preflight(&attempt, None, &snapshot);
            let material_refused = target_blockers.iter().any(|blocker| {
                matches!(
                    blocker,
                    Blocker::MaterialNotAdmitted | Blocker::MaterialAdmissionStale
                )
            });
            blockers.extend(target_blockers);
            if material_sensitive(verb) && !material_refused {
                let admission = self
                    .draft_materials
                    .get(&target_ref)
                    .expect("material preflight requires an admission");
                if let MaterialRequirement::Component {
                    component: expected_component,
                    ..
                } = &estimate.material_requirement
                {
                    let expected_amount = estimate
                        .target_amounts
                        .iter()
                        .find(|(coord, _)| coord == target)
                        .map(|(_, amount)| *amount)
                        .ok_or(SelectionError::UnsupportedCell(*target))?;
                    if &admission.component != expected_component
                        || admission.amount != expected_amount
                    {
                        blockers.push(Blocker::SelectionContractChanged);
                    }
                }
                match &component {
                    None => component = Some(admission.component.clone()),
                    Some(existing) if existing == &admission.component => {}
                    Some(_) => blockers.push(Blocker::SelectionContractChanged),
                }
            }
        }
        if matches!(verb, PhysicalVerb::Fill | PhysicalVerb::Place) {
            if let Some(component) = component {
                let source = Account::new(format!("carried:{}", actor.as_str()))?;
                let available = snapshot
                    .component_accounts
                    .get(&(source, component))
                    .copied()
                    .unwrap_or(Rational::ZERO);
                if matches!(
                    available.try_cmp(&estimate.required_material),
                    Ok(std::cmp::Ordering::Less)
                ) {
                    blockers.push(Blocker::InsufficientResources);
                }
            }
        }
        if !blockers.is_empty() {
            return Err(AgencyError::Refused { blockers });
        }
        let id = PlanId::new(format!("selection:{}", self.next_plan))?;
        let reservation = ReservationId::new(format!("{id}:plan"))?;
        let reservation_target = TargetRef::new(format!("selection:{id}"))?;
        self.next_plan = self
            .next_plan
            .checked_add(1)
            .ok_or(AgencyError::PlanLimit)?;
        self.store.reserve(
            reservation.clone(),
            actor.clone(),
            reservation_target.clone(),
        )?;
        let transition_id = WorldTransition::new(
            actor.clone(),
            task.clone(),
            verb,
            reservation_target,
            Some(reservation.clone()),
            0,
            id.as_u64(),
        )
        .id();
        let preview = SelectionPreview {
            verb,
            bounds: selection,
            targets: estimate.targets.clone(),
            estimate: estimate.clone(),
        };
        let plan = SelectionPlan {
            id,
            actor,
            task,
            tool,
            capability: verb,
            policy: policy.clone(),
            selection,
            targets: estimate.targets,
            reservations: vec![reservation.clone()],
            preview,
            state: PlanState::Reserved,
        };
        let created = Evidence::for_transition(
            "selection.created",
            format!("{} cells reserved", plan.reservations.len()),
            transition_id,
            Some(reservation),
        );
        self.record_evidence(created, Some(plan.id));
        for target in &plan.targets {
            self.planned_targets
                .insert(TargetRef::voxel(*target), plan.id);
        }
        self.plans.insert(plan.id, plan.clone());
        Ok(plan)
    }

    pub fn execute_plan(&mut self, id: PlanId) -> Result<ActionReceipt, WorldError> {
        let plan = self
            .plans
            .get(&id)
            .cloned()
            .ok_or(WorldError::InvalidTransition {
                reason: format!("unknown selection plan {id}"),
            })?;
        if plan.state != PlanState::Reserved {
            return Err(WorldError::InvalidTransition {
                reason: "selection plan is not reserved".to_owned(),
            });
        }
        let before = self.store.conservation_snapshot()?;
        let reservation = plan.reservations[0].clone();
        let reservation_target = TargetRef::new(format!("selection:{id}"))?;
        let transition = WorldTransition::new(
            plan.actor.clone(),
            plan.task.clone(),
            plan.capability,
            reservation_target.clone(),
            Some(reservation.clone()),
            0,
            id.as_u64(),
        );
        let transition_id = transition.id();
        let mut blockers = Vec::new();
        if self
            .verb_policies
            .get(&(plan.actor.clone(), plan.capability))
            != Some(&plan.policy)
        {
            blockers.push(Blocker::PolicyDrift);
        }
        for target in &plan.targets {
            let attempt = ActionAttempt {
                actor: plan.actor.clone(),
                task: plan.task.clone(),
                verb: plan.capability,
                target: TargetRef::voxel(*target),
                tool: plan.tool.clone(),
                epoch: 0,
                nonce: id.as_u64(),
            };
            blockers.extend(self.preflight(&attempt, Some(&reservation), &before));
        }
        match self.estimate_for_actor(plan.actor.clone(), plan.selection, plan.capability) {
            Ok(current) if current == plan.preview.estimate => {}
            Ok(_) | Err(_) => blockers.push(Blocker::SelectionContractChanged),
        }
        if !blockers.is_empty() {
            return Ok(self.refused_receipt(
                before,
                transition_id,
                blockers,
                Some(reservation),
                Some(id),
            ));
        }
        let deltas = self.build_selection_deltas(&plan, transition_id)?;
        #[cfg(debug_assertions)]
        let deltas = if self.selection_batch_refusal_for_test {
            self.selection_batch_refusal_for_test = false;
            let mut deltas = deltas;
            deltas.push(Delta::MatterTransfer {
                component: ComponentId::new("selection-test-missing")?,
                from: Account::new("selection-test-missing")?,
                to: Account::new(format!("carried:{}", plan.actor.as_str()))?,
                amount: Rational::from_integer(1),
                lineage: LineageId::from_parts(["selection-test", "physical-refusal"])?,
            });
            deltas
        } else {
            deltas
        };
        // **One governed transition, through the book's one entry point.** A plan commits as a
        // single act — the effect is the whole batch, so a plan that reserved twelve cells is
        // one transition with twelve deltas, and the receipt describes all of them.
        let effect = EarthworksEffect { deltas };
        let Some(permission) = self.permission.clone() else {
            return Ok(ephemeral_refused_receipt(
                before,
                transition_id,
                vec![Blocker::AuthorityMissing],
                reservation,
            ));
        };
        let receipt = match self.store.apply_transition(&transition, &effect, &permission) {
            Ok(receipt) => receipt,
            Err(refusal) => {
                return Ok(ephemeral_refused_receipt(
                    before,
                    transition_id,
                    vec![Blocker::TransitionRefused(refusal)],
                    reservation,
                ));
            }
        };
        let physical = PhysicalOutcome::from_receipt(&receipt, &effect);
        let consequences = plan
            .targets
            .iter()
            .filter_map(|target| {
                let target_ref = TargetRef::voxel(*target);
                self.consequences
                    .get(&(target_ref.clone(), plan.capability))
                    .map(|policy| WorkConsequence::from_policy(target_ref, policy))
            })
            .collect::<Vec<_>>();
        let mut evidence = physical.evidence;
        evidence.push(Evidence::for_transition(
            "action.accepted",
            format!("{} selection committed atomically", plan.capability),
            transition_id,
            Some(reservation.clone()),
        ));
        for consequence in &consequences {
            evidence.push(Evidence::for_transition(
                "consequence.recorded",
                format!(
                    "target={}; owner={}",
                    consequence.target.as_str(),
                    consequence.owner.as_str()
                ),
                transition_id,
                Some(reservation.clone()),
            ));
        }
        for row in &evidence {
            self.record_evidence(row.clone(), Some(id));
        }
        self.plans.get_mut(&id).unwrap().state = PlanState::Committed;
        self.clear_plan_targets(id, &plan);
        Ok(ActionReceipt {
            accepted: true,
            blockers: Vec::new(),
            reservations: vec![reservation],
            deltas: physical.deltas,
            evidence,
            consequences,
            transition_id,
            read_back: physical.read_back,
        })
    }

    fn build_selection_deltas(
        &self,
        plan: &SelectionPlan,
        transition_id: Digest,
    ) -> Result<Vec<Delta>, WorldError> {
        let snapshot = self.store.conservation_snapshot()?;
        let mut deltas = Vec::new();
        let lineage =
            LineageId::from_parts(["selection", plan.actor.as_str(), &plan.id.to_string()])?;
        for target in &plan.targets {
            let target_ref = TargetRef::voxel(*target);
            match plan.capability {
                PhysicalVerb::Break | PhysicalVerb::Gather | PhysicalVerb::Dig => {
                    let admission = self.draft_materials.get(&target_ref).ok_or(
                        WorldError::InvalidTransition {
                            reason: "selection material admission is missing".to_owned(),
                        },
                    )?;
                    let provenance = snapshot.occupancy_provenance.get(target).ok_or(
                        WorldError::InvalidTransition {
                            reason: "selection occupancy provenance is missing".to_owned(),
                        },
                    )?;
                    let before = snapshot
                        .occupancy
                        .get(target)
                        .copied()
                        .unwrap_or_else(Occupancy::empty);
                    let destination = Account::new(format!(
                        "{}:{}",
                        if plan.capability == PhysicalVerb::Dig {
                            "spoil"
                        } else {
                            "carried"
                        },
                        plan.actor.as_str()
                    ))?;
                    deltas.push(Delta::MatterTransfer {
                        component: admission.component.clone(),
                        from: admission.account.clone(),
                        to: destination,
                        amount: admission.amount,
                        lineage: lineage.clone(),
                    });
                    deltas.push(Delta::OccupancyChange {
                        coord: *target,
                        before,
                        after: Occupancy::empty(),
                        provenance: OccupancyProvenance {
                            transition_id,
                            account: admission.account.clone(),
                            lineage: lineage.clone(),
                            fluid_address: provenance.fluid_address.clone(),
                        },
                    });
                }
                PhysicalVerb::Fill | PhysicalVerb::Place => {
                    let admission = self.draft_materials.get(&target_ref).ok_or(
                        WorldError::InvalidTransition {
                            reason: "selection material admission is missing".to_owned(),
                        },
                    )?;
                    let before = snapshot
                        .occupancy
                        .get(target)
                        .copied()
                        .unwrap_or_else(Occupancy::empty);
                    deltas.push(Delta::MatterTransfer {
                        component: admission.component.clone(),
                        from: Account::new(format!("carried:{}", plan.actor.as_str()))?,
                        to: admission.account.clone(),
                        amount: Rational::from_integer(1),
                        lineage: lineage.clone(),
                    });
                    deltas.push(Delta::OccupancyChange {
                        coord: *target,
                        before,
                        after: Occupancy::full(),
                        provenance: OccupancyProvenance {
                            transition_id,
                            account: admission.account.clone(),
                            lineage: lineage.clone(),
                            fluid_address: None,
                        },
                    });
                }
                PhysicalVerb::Use | PhysicalVerb::Craft => {
                    deltas.push(Delta::EvidenceOutput {
                        message: format!("selection work boundary {transition_id:?}"),
                        lineage: lineage.clone(),
                    });
                }
            }
        }
        Ok(deltas)
    }

    pub fn cancel_plan(&mut self, id: PlanId) -> Result<(), AgencyError> {
        let plan = self
            .plans
            .get(&id)
            .cloned()
            .ok_or(AgencyError::UnknownPlan(id))?;
        if plan.state != PlanState::Reserved {
            return Err(AgencyError::PlanNotCancellable(id));
        }
        for reservation in &plan.reservations {
            self.store.cancel_reservation(&reservation)?;
        }
        self.plans.get_mut(&id).unwrap().state = PlanState::Cancelled;
        self.clear_plan_targets(id, &plan);
        let evidence = Evidence::new(
            "selection.cancelled",
            "all reservations cancelled before physical effect",
        );
        let transition_id = evidence.digest;
        let associated = Evidence::for_transition(
            evidence.kind,
            evidence.message,
            transition_id,
            plan.reservations.first().cloned(),
        );
        self.record_evidence(associated, Some(id));
        Ok(())
    }

    pub fn has_plan_reservation(&self, id: PlanId) -> Result<bool, AgencyError> {
        let plan = self.plans.get(&id).ok_or(AgencyError::UnknownPlan(id))?;
        Ok(plan
            .reservations
            .iter()
            .any(|reservation| self.store.has_reservation(reservation)))
    }

    pub fn evidence_for(&self, query: EvidenceQuery) -> Vec<AgencyEvidence> {
        self.evidence
            .iter()
            .filter(|row| match &query {
                EvidenceQuery::Transition(id) => row.evidence.transition_id == Some(*id),
                EvidenceQuery::Reservation(id) => row.evidence.reservation.as_ref() == Some(id),
                EvidenceQuery::Plan(id) => row.plan_id == Some(*id),
            })
            .cloned()
            .collect()
    }

    pub fn evidence_for_plan(&self, id: PlanId) -> Vec<Evidence> {
        self.evidence_for(EvidenceQuery::Plan(id))
            .into_iter()
            .map(|row| row.evidence)
            .collect()
    }

    pub fn world_evidence_for(&self, id: &ReservationId) -> Vec<Evidence> {
        self.store.evidence_for(id)
    }

    /// **The governed effect one act is made of** — the machine's own formation of the change,
    /// in the book's vocabulary.
    ///
    /// The commit point has an actor, a task, a verb and a target in hand; this is where those
    /// become the [`EarthworksEffect`] the book commits. A verb outside the effect vocabulary is
    /// [`Blocker::InvalidConsequence`], named rather than defaulted — the tree's own rule for a
    /// closed list ("a verb arrives with an implementation, not before").
    fn effect_for(
        &self,
        attempt: &ActionAttempt,
        transition_id: &Digest,
    ) -> Result<EarthworksEffect, WorldError> {
        let coord = target_coord(&attempt.target).ok_or(WorldError::InvalidTransition {
            reason: "the act's target is not a coordinate".to_owned(),
        })?;
        let admission = self
            .draft_materials
            .get(&attempt.target)
            .ok_or(WorldError::InvalidTransition {
                reason: "the act's material was never admitted".to_owned(),
            })?;
        let lineage = LineageId::from_parts([
            attempt.verb.as_str(),
            attempt.actor.as_str(),
            &transition_id.to_hex(),
        ])?;
        match attempt.verb {
            PhysicalVerb::Break | PhysicalVerb::Gather | PhysicalVerb::Dig => {
                let spoil = Account::new(format!(
                    "{}:{}",
                    if attempt.verb == PhysicalVerb::Dig {
                        "spoil"
                    } else {
                        "carried"
                    },
                    attempt.actor.as_str()
                ))?;
                EarthworksEffect::excavation(
                    coord,
                    admission.account.clone(),
                    spoil,
                    admission.component.clone(),
                    admission.amount,
                    lineage,
                )
            }
            PhysicalVerb::Fill | PhysicalVerb::Place => {
                let carried = Account::new(format!("carried:{}", attempt.actor.as_str()))?;
                EarthworksEffect::placement(
                    carried,
                    admission.account.clone(),
                    admission.component.clone(),
                    admission.amount,
                    lineage,
                )
                .map_err(|error| WorldError::InvalidTransition {
                    reason: error.to_string(),
                })
            }
            // **`Use` and `Craft` are not earthworks.** They consume no material, so there is no
            // effect to form and no delta to commit; the honest answer is a refusal that names
            // the verb, not an empty effect that commits nothing while claiming it did.
            PhysicalVerb::Use | PhysicalVerb::Craft => Err(WorldError::InvalidTransition {
                reason: format!(
                    "`{}` is not an earthwork: it moves no material, so it has no effect to commit",
                    attempt.verb.as_str()
                ),
            }),
        }
    }

    fn clear_plan_targets(&mut self, id: PlanId, plan: &SelectionPlan) {
        let targets = plan
            .targets
            .iter()
            .copied()
            .map(TargetRef::voxel)
            .collect::<BTreeSet<_>>();
        self.planned_targets
            .retain(|target, owner| *owner != id || !targets.contains(target));
    }

    pub fn plan(&self, id: PlanId) -> Result<&SelectionPlan, AgencyError> {
        self.plans.get(&id).ok_or(AgencyError::UnknownPlan(id))
    }

    fn record_evidence(&mut self, evidence: Evidence, plan_id: Option<PlanId>) {
        self.evidence.push(AgencyEvidence { evidence, plan_id });
    }

    fn refused_receipt(
        &mut self,
        before: ConservationSnapshot,
        transition_id: Digest,
        blockers: Vec<Blocker>,
        reservation: Option<ReservationId>,
        plan_id: Option<PlanId>,
    ) -> ActionReceipt {
        let message = blockers
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ");
        let evidence = Evidence::for_transition(
            "action.refused",
            message,
            transition_id,
            reservation.clone(),
        );
        self.record_evidence(evidence.clone(), plan_id);
        ActionReceipt {
            accepted: false,
            blockers,
            reservations: reservation.into_iter().collect(),
            deltas: Vec::new(),
            evidence: vec![evidence],
            consequences: Vec::new(),
            transition_id,
            read_back: Some(before),
        }
    }
}

fn ephemeral_refused_receipt(
    before: ConservationSnapshot,
    transition_id: Digest,
    blockers: Vec<Blocker>,
    reservation: ReservationId,
) -> ActionReceipt {
    let message = blockers
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ");
    let evidence = Evidence::for_transition(
        "action.refused",
        message,
        transition_id,
        Some(reservation.clone()),
    );
    ActionReceipt {
        accepted: false,
        blockers,
        reservations: vec![reservation],
        deltas: Vec::new(),
        evidence: vec![evidence],
        consequences: Vec::new(),
        transition_id,
        read_back: Some(before),
    }
}

fn material_current(admission: &DraftMaterialAdmission, snapshot: &ConservationSnapshot) -> bool {
    snapshot.digest == admission.snapshot_digest
        && snapshot.occupancy.get(&admission.coord).copied() == Some(admission.occupancy)
        && snapshot.occupancy_provenance.get(&admission.coord) == Some(&admission.provenance)
        && snapshot
            .component_accounts
            .get(&(admission.account.clone(), admission.component.clone()))
            .copied()
            == Some(admission.amount)
}

fn material_sensitive(verb: PhysicalVerb) -> bool {
    !matches!(verb, PhysicalVerb::Use | PhysicalVerb::Craft)
}

fn occupancy_matches(verb: PhysicalVerb, occupancy: Occupancy) -> bool {
    match verb {
        PhysicalVerb::Break | PhysicalVerb::Gather | PhysicalVerb::Dig => {
            occupancy == Occupancy::full()
        }
        PhysicalVerb::Fill | PhysicalVerb::Place => occupancy == Occupancy::empty(),
        PhysicalVerb::Use | PhysicalVerb::Craft => true,
    }
}

pub(crate) fn target_coord(target: &TargetRef) -> Option<Coord> {
    let value = target.as_str().strip_prefix("voxel:")?;
    let mut values = value.split(',').map(str::parse::<i32>);
    Some(Coord::new(
        values.next()?.ok()?,
        values.next()?.ok()?,
        values.next()?.ok()?,
    ))
}

fn manhattan(a: Coord, b: Coord) -> i64 {
    i64::from(a.x.abs_diff(b.x)) + i64::from(a.y.abs_diff(b.y)) + i64::from(a.z.abs_diff(b.z))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgencyError {
    DuplicateActor(ActorId),
    ActorNotFound(ActorId),
    DuplicateTask(TaskId),
    InvalidTask,
    InvalidVerbPolicy,
    InvalidMaterialFixture,
    InvalidRiskFixture,
    InvalidConsequence,
    PlanLimit,
    UnknownPlan(PlanId),
    PlanNotCancellable(PlanId),
    Refused { blockers: Vec<Blocker> },
    Selection(SelectionError),
    World(WorldError),
}

impl fmt::Display for AgencyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateActor(id) => {
                write!(formatter, "actor {} is already registered", id.as_str())
            }
            Self::ActorNotFound(id) => write!(formatter, "actor {} is not registered", id.as_str()),
            Self::DuplicateTask(id) => {
                write!(formatter, "task {} is already registered", id.as_str())
            }
            Self::InvalidTask => formatter.write_str("task requirements are invalid"),
            Self::InvalidVerbPolicy => {
                formatter.write_str("verb policy work per cell must be positive")
            }
            Self::InvalidMaterialFixture => {
                formatter.write_str("draft material fixture does not match target provenance")
            }
            Self::InvalidRiskFixture => {
                formatter.write_str("selection risk fixture cannot be negative")
            }
            Self::InvalidConsequence => {
                formatter.write_str("consequence compensation cannot be negative")
            }
            Self::PlanLimit => formatter.write_str("selection plan id limit reached"),
            Self::UnknownPlan(id) => write!(formatter, "selection plan {id} is unknown"),
            Self::PlanNotCancellable(id) => {
                write!(formatter, "selection plan {id} is not cancellable")
            }
            Self::Refused { blockers } => {
                let message = blockers
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("; ");
                write!(formatter, "selection refused: {message}")
            }
            Self::Selection(error) => write!(formatter, "{error}"),
            Self::World(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for AgencyError {}

impl From<super::ActorError> for AgencyError {
    fn from(error: super::ActorError) -> Self {
        Self::Selection(SelectionError::Actor(error))
    }
}

impl From<SelectionError> for AgencyError {
    fn from(error: SelectionError) -> Self {
        Self::Selection(error)
    }
}

impl From<WorldError> for AgencyError {
    fn from(error: WorldError) -> Self {
        Self::World(error)
    }
}
