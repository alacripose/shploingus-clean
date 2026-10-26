//! **The civilian systems' gates, on the axis side of the seam.**
//!
//! The systems themselves are `voxel-model::civilian` — the ported agency, governor, selection
//! planner and task machine — and this crate re-exports them rather than keeping a second copy.
//! The dependency direction settled on 2026-10-25 is why: the systems are written through
//! `voxel_model::world`'s governed book, `matter` and `observer`, and moving their files to a
//! crate that depends on those modules would invert the model's own layer. What belongs here is
//! the **reading and the rule** the axis layers need, not a second machine.
//!
//! Four gates are stated, each with a live subject in the ported code:
//!
//! * a **selection** cites a need, has an estimate, and spends within its budget — the four
//!   conditions `agency/selection.rs` declares, checked before any work;
//! * a **task** has its material planned before the work and cannot complete by declaration —
//!   `sim/task.rs`'s own two sentences;
//! * a **ladder reading** is **held**, **absent** or **unreadable**, and this crate owns that
//!   three-way answer. `voxel-package::decision::rule_for_civilian` consumes it and maps it to the
//!   decision layer's `Active` / `Unchecked` / `Refused`; an absent witness is a state, not a
//!   refusal, because the record was readable and said so;
//! * an **arrangement** round-trips through `read`/`write` without dropping, adding or moving a
//!   pane, and a refused write leaves the running arrangement untouched — the host loop's R1/R2
//!   stated over the seam rather than over the client.
//!
//! # What is owed, and named
//!
//! The 5,471-line physical relocation PATCH-042 describes is **not** this pass. The systems are
//! already compiled in the model; a move that changes no behaviour but makes every axis crate
//! depend back on the model's world vocabulary is a directory change, not a dependency direction.
//! The client-side `Mount` adapter for [`Arrangement`] is still owed, because this crate may not
//! depend on `voxel-client`; the ladder reading already has its caller in `voxel-package`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub use voxel_model::civilian;
pub use voxel_model::civilian::agency::{
    PlanState, SelectionBudget, SelectionEstimate, SelectionPlan,
};
pub use voxel_model::civilian::sim::task::{Task, Verb};

use voxel_axis_ladder::AxisKind;

/// A need a selection serves. **Never empty**, because a selection with no need is a selection
/// nobody asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Need(String);

impl Need {
    /// Record a need, refusing an empty one.
    pub fn new(text: impl Into<String>) -> Result<Self, CivilianRefusal> {
        let text = text.into();
        if text.trim().is_empty() {
            return Err(CivilianRefusal::EmptyNeed);
        }
        Ok(Need(text))
    }

    /// The need as written.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Whether a task's completion was checked or merely declared.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Completion {
    /// The verb's own check ran and passed.
    Checked,
    /// Somebody said so.
    Declared,
}

/// A selection, its need, its budget and the estimate that must precede the work.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selection {
    /// What the work is for.
    pub need: Need,
    /// The plan the civilians produced.
    pub plan: SelectionPlan,
    /// The declared ceiling.
    pub budget: SelectionBudget,
    /// The estimate, when the planner produced one.
    pub estimate: Option<SelectionEstimate>,
}

impl Selection {
    /// **The four conditions, refused by name.** A selection without a need, without an estimate,
    /// without an atomic task, or over budget is not a work item.
    pub fn verify(&self) -> Result<(), CivilianRefusal> {
        if self.need.as_str().trim().is_empty() {
            return Err(CivilianRefusal::EmptyNeed);
        }
        let Some(estimate) = &self.estimate else {
            return Err(CivilianRefusal::MissingEstimate);
        };
        if self.plan.task.is_none() {
            return Err(CivilianRefusal::NoAtomicTask);
        }
        if self.plan.targets.is_empty() {
            return Err(CivilianRefusal::NoTargets);
        }
        if !matches!(self.plan.state, PlanState::Reserved | PlanState::Committed) {
            return Err(CivilianRefusal::PlanNotLive {
                state: self.plan.state,
            });
        }
        let cells = self
            .plan
            .selection
            .cells()
            .map_err(|error| CivilianRefusal::Selection(error.to_string()))?;
        if cells > self.budget.max_cells {
            return Err(CivilianRefusal::BudgetExceeded {
                requested: cells,
                maximum: self.budget.max_cells,
            });
        }
        if estimate.work_ticks > self.budget.max_work {
            return Err(CivilianRefusal::BudgetExceeded {
                requested: estimate.work_ticks,
                maximum: self.budget.max_work,
            });
        }
        Ok(())
    }
}

/// **A task's two sentences, as gates.**
///
/// The material is planned before the work starts, and completion is the verb's check rather than
/// a claim. A `Build` whose planned draw is empty, or whose draw does not account for the mass it
/// says it needs, is refused; a task closed as [`Completion::Declared`] is refused.
pub fn verify_task(task: &Task, completion: Completion) -> Result<(), CivilianRefusal> {
    if task.kind.trim().is_empty() {
        return Err(CivilianRefusal::EmptyTaskKind);
    }
    if matches!(task.verb, Verb::Make | Verb::Gather) && task.process.trim().is_empty() {
        return Err(CivilianRefusal::EmptyProcess);
    }
    if matches!(task.verb, Verb::Build) {
        if task.material.is_empty() {
            return Err(CivilianRefusal::MaterialNotPlanned);
        }
        let planned = task.lineage_total();
        if planned != task.requires_g {
            return Err(CivilianRefusal::MaterialMismatch {
                planned,
                required: task.requires_g,
            });
        }
    }
    if completion == Completion::Declared {
        return Err(CivilianRefusal::CompletionByDeclaration);
    }
    Ok(())
}

/// **A ladder reading's answer, once its witness has been checked.**
///
/// **The absence of a witness is a state, not a refusal.** The record was readable and says it
/// does not carry this ladder's token; that is `Absent`, the fact the decision layer renders as
/// `Unchecked`. An unreadable witness is the refusal, because the record could not say anything.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LadderReadingState {
    /// The witness was readable and carries the ladder's token.
    Held,
    /// The witness was readable and does not carry the ladder's token.
    Absent,
}

/// **A ladder reading's one owner: readable + present is `Held`, readable + absent is `Absent`,
/// unreadable is a refusal.**
///
/// `voxel-package`'s reader supplies the two facts; this type owns what they mean. The package's
/// decision layer maps [`LadderReadingState`] to `Establishment` and never re-decides whether an
/// absent witness is a refusal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LadderReading {
    /// Which ladder was read.
    pub ladder: AxisKind,
    /// The witness file, relative to the workspace root.
    pub witness: String,
    /// Whether the witness could be opened at all.
    pub readable: bool,
    /// Whether the witness carries the ladder's token.
    pub present: bool,
}

impl LadderReading {
    /// The reading, or the refusal that says why it could not be read.
    ///
    /// A reading that cannot say *where* is refused rather than treated as absent. An absent
    /// token is **not** this error: it is [`LadderReadingState::Absent`].
    pub fn verify(&self) -> Result<LadderReadingState, CivilianRefusal> {
        if self.witness.trim().is_empty() {
            return Err(CivilianRefusal::UnwitnessedLadder {
                ladder: self.ladder.name(),
            });
        }
        if !self.readable {
            return Err(CivilianRefusal::UnreadableLadder {
                ladder: self.ladder.name(),
                witness: self.witness.clone(),
            });
        }
        if self.present {
            Ok(LadderReadingState::Held)
        } else {
            Ok(LadderReadingState::Absent)
        }
    }
}

/// One pane of an arrangement, as the host reads and writes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pane {
    /// The window's own id.
    pub window: String,
    /// The lane the window occupies.
    pub lane: String,
    /// The pane's extent in device pixels.
    pub extent: (u32, u32),
    /// The content kind, as the recipe declares it.
    pub content: String,
}

impl Pane {
    fn valid(&self) -> bool {
        !self.window.trim().is_empty()
            && !self.lane.trim().is_empty()
            && !self.content.trim().is_empty()
            && self.extent.0 > 0
            && self.extent.1 > 0
    }
}

/// **The forward and backward halves of the mount loop, over a seam rather than a client.**
///
/// The client's `Mount` owns the windows and the frame. This type owns the two operations the
/// civilians' host needs: read the live panes, and write a new set atomically. An invalid set is
/// refused before the current one is touched, which is the R2 rule stated where it can be tested
/// without a GPU.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Arrangement {
    panes: Vec<Pane>,
}

impl Arrangement {
    /// Build an arrangement, refusing an empty or duplicated window list.
    pub fn of(panes: Vec<Pane>) -> Result<Self, CivilianRefusal> {
        Arrangement::validate(&panes)?;
        Ok(Arrangement { panes })
    }

    /// **R1's read half.** The live panes, in their declared order.
    pub fn read(&self) -> &[Pane] {
        &self.panes
    }

    /// **R1 and R2's write half.** Validate first, then replace; a refused set leaves the running
    /// arrangement untouched.
    pub fn write(&mut self, panes: Vec<Pane>) -> Result<(), CivilianRefusal> {
        Arrangement::validate(&panes)?;
        self.panes = panes;
        Ok(())
    }

    fn validate(panes: &[Pane]) -> Result<(), CivilianRefusal> {
        if panes.is_empty() {
            return Err(CivilianRefusal::EmptyArrangement);
        }
        let mut seen = std::collections::BTreeSet::new();
        for pane in panes {
            if !pane.valid() {
                return Err(CivilianRefusal::InvalidPane {
                    window: pane.window.clone(),
                });
            }
            if !seen.insert(pane.window.clone()) {
                return Err(CivilianRefusal::DuplicateWindow {
                    window: pane.window.clone(),
                });
            }
        }
        Ok(())
    }
}

/// Why a civilian gate refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CivilianRefusal {
    /// The selection cites no need.
    EmptyNeed,
    /// The selection has no estimate.
    MissingEstimate,
    /// The selection names no atomic task.
    NoAtomicTask,
    /// The selection targets no cell.
    NoTargets,
    /// The plan's state is not live.
    PlanNotLive {
        /// The state the plan was in.
        state: PlanState,
    },
    /// The plan or its estimate exceeds the declared budget.
    BudgetExceeded {
        /// What was requested.
        requested: u64,
        /// The declared ceiling.
        maximum: u64,
    },
    /// The plan's selection could not be measured.
    Selection(String),
    /// A task has no kind.
    EmptyTaskKind,
    /// A make or gather has no process.
    EmptyProcess,
    /// A build has no planned material.
    MaterialNotPlanned,
    /// A build's planned draw does not account for what it says it needs.
    MaterialMismatch {
        /// The planned grams.
        planned: i64,
        /// The declared requirement.
        required: i64,
    },
    /// A task closed by declaration rather than by its verb's check.
    CompletionByDeclaration,
    /// A ladder reading names no witness.
    UnwitnessedLadder {
        /// The ladder's name.
        ladder: &'static str,
    },
    /// A ladder reading's witness could not be read.
    UnreadableLadder {
        /// The ladder's name.
        ladder: &'static str,
        /// The witness that could not be read.
        witness: String,
    },
    /// An arrangement has no panes.
    EmptyArrangement,
    /// A pane has no window, lane, content or positive extent.
    InvalidPane {
        /// The offending window id.
        window: String,
    },
    /// Two panes name one window.
    DuplicateWindow {
        /// The duplicated window id.
        window: String,
    },
}

impl CivilianRefusal {
    /// A one-line reason, never only the class.
    pub fn describe(&self) -> String {
        match self {
            CivilianRefusal::EmptyNeed => "a selection with no need is work nobody asked for".to_string(),
            CivilianRefusal::MissingEstimate => "the plan has no estimate; estimate-before-work is the rule".to_string(),
            CivilianRefusal::NoAtomicTask => "the plan names no task, so its work is not atomic".to_string(),
            CivilianRefusal::NoTargets => "the plan targets no cell".to_string(),
            CivilianRefusal::PlanNotLive { state } => format!("the plan is {state:?}, not reserved or committed"),
            CivilianRefusal::BudgetExceeded { requested, maximum } => format!("the plan spends {requested} and the budget is {maximum}"),
            CivilianRefusal::Selection(reason) => reason.clone(),
            CivilianRefusal::EmptyTaskKind => "the task names no kind".to_string(),
            CivilianRefusal::EmptyProcess => "a make or gather names no process".to_string(),
            CivilianRefusal::MaterialNotPlanned => "a build has no planned material".to_string(),
            CivilianRefusal::MaterialMismatch { planned, required } => format!("the plan accounts for {planned} g and the task requires {required} g"),
            CivilianRefusal::CompletionByDeclaration => "a task cannot be completed by declaring it complete".to_string(),
            CivilianRefusal::UnwitnessedLadder { ladder } => format!("{ladder} has no witness"),
            CivilianRefusal::UnreadableLadder { ladder, witness } => format!("{ladder}'s witness {witness} could not be read"),
            CivilianRefusal::EmptyArrangement => "an arrangement with no panes is not a mount".to_string(),
            CivilianRefusal::InvalidPane { window } => format!("pane {window:?} is missing a window, lane, content or extent"),
            CivilianRefusal::DuplicateWindow { window } => format!("window {window:?} is declared twice"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_model::civilian::agency::{
        CuboidSelection, PlanId, SelectionPreview, TaskRequirement, VerbPolicy,
    };
    use voxel_model::civilian::sim::task::Stage;
    use voxel_model::world::{ActorId, Coord, PhysicalVerb, TaskId};

    fn plan(state: PlanState) -> SelectionPlan {
        let bounds = CuboidSelection::new(Coord::new(0, 0, 0), Coord::new(0, 0, 0)).unwrap();
        let estimate = SelectionEstimate {
            verb: PhysicalVerb::Place,
            cells: 1,
            intersected_cells: 1,
            work_ticks: 2,
            mass_before: voxel_model::matter::Rational::ZERO,
            mass_after: voxel_model::matter::Rational::ZERO,
            required_material: voxel_model::matter::Rational::ZERO,
            material_requirement: voxel_model::civilian::agency::MaterialRequirement::NoMaterial("test"),
            target_amounts: Vec::new(),
            fluid: voxel_model::civilian::agency::EstimateValue::Known(
                voxel_model::matter::Rational::ZERO,
            ),
            risk: voxel_model::civilian::agency::EstimateValue::Known(
                voxel_model::matter::Rational::ZERO,
            ),
            targets: vec![Coord::new(0, 0, 0)],
        };
        SelectionPlan {
            id: PlanId::new("plan").unwrap(),
            actor: ActorId::new("actor").unwrap(),
            task: Some(TaskId::new("task").unwrap()),
            tool: None,
            capability: PhysicalVerb::Place,
            policy: VerbPolicy::new(
                TaskRequirement::Required {
                    task: TaskId::new("task").unwrap(),
                    tool: None,
                },
                2,
            ),
            selection: bounds,
            targets: vec![Coord::new(0, 0, 0)],
            reservations: Vec::new(),
            preview: SelectionPreview {
                verb: PhysicalVerb::Place,
                bounds,
                targets: vec![Coord::new(0, 0, 0)],
                estimate,
            },
            state,
        }
    }

    fn selection(estimate: Option<SelectionEstimate>) -> Selection {
        Selection {
            need: Need::new("a wall the citizen needs").unwrap(),
            plan: plan(PlanState::Reserved),
            budget: SelectionBudget::default(),
            estimate,
        }
    }

    #[test]
    fn a_selection_with_a_need_a_budget_and_an_estimate_is_accepted() {
        let plan = plan(PlanState::Reserved);
        let estimate = plan.preview.estimate.clone();
        assert!(selection(Some(estimate)).verify().is_ok());
    }

    #[test]
    fn a_selection_without_an_estimate_or_without_a_task_is_refused_by_name() {
        let refused = selection(None).verify().expect_err("no estimate");
        assert_eq!(refused, CivilianRefusal::MissingEstimate);

        let mut plan = plan(PlanState::Reserved);
        plan.task = None;
        let mut with_task = selection(Some(plan.preview.estimate.clone()));
        with_task.plan = plan;
        let refused = with_task.verify().expect_err("no task");
        assert_eq!(refused, CivilianRefusal::NoAtomicTask);
    }

    #[test]
    fn a_task_without_planned_material_or_closed_by_declaration_is_refused() {
        let task = Task {
            id: 1,
            verb: Verb::Build,
            kind: "wall".to_string(),
            site: 4,
            family: "stone".to_string(),
            requires_g: 12,
            material: Vec::new(),
            process: String::new(),
            substance: String::new(),
            fetch_from: None,
            stage: Stage::Working,
            work_remaining: 2,
            claimed_by: None,
            opened_tick: 0,
        };
        assert_eq!(
            verify_task(&task, Completion::Checked).expect_err("no material"),
            CivilianRefusal::MaterialNotPlanned
        );
        let mut checked = task;
        checked.material = vec![voxel_model::civilian::sim::task::PlannedDraw {
            component: "stone".to_string(),
            grams: 12,
        }];
        assert!(verify_task(&checked, Completion::Checked).is_ok());
        assert_eq!(
            verify_task(&checked, Completion::Declared).expect_err("declared completion"),
            CivilianRefusal::CompletionByDeclaration
        );
    }

    #[test]
    fn a_ladder_reading_is_held_absent_or_refused_and_absent_is_not_a_refusal() {
        let good = LadderReading {
            ladder: AxisKind::EvidenceGrade,
            witness: "crates/voxel-model/src/axis.rs".to_string(),
            readable: true,
            present: true,
        };
        assert_eq!(good.verify(), Ok(LadderReadingState::Held));

        let mut absent = good.clone();
        absent.present = false;
        assert_eq!(absent.verify(), Ok(LadderReadingState::Absent));

        let mut unreadable = good.clone();
        unreadable.readable = false;
        assert_eq!(
            unreadable.verify(),
            Err(CivilianRefusal::UnreadableLadder {
                ladder: AxisKind::EvidenceGrade.name(),
                witness: unreadable.witness.clone(),
            })
        );

        let mut unwitnessed = good;
        unwitnessed.witness.clear();
        assert!(matches!(
            unwitnessed.verify(),
            Err(CivilianRefusal::UnwitnessedLadder { .. })
        ));
    }

    #[test]
    fn an_arrangement_round_trips_and_a_refused_write_leaves_it_untouched() {
        let pane = Pane {
            window: "alpha".to_string(),
            lane: "left".to_string(),
            extent: (640, 440),
            content: "ApiViewer".to_string(),
        };
        let mut arrangement = Arrangement::of(vec![pane.clone()]).expect("one pane");
        assert_eq!(arrangement.read(), &[pane.clone()]);
        let before = arrangement.clone();
        let refused = arrangement.write(vec![Pane {
            window: "alpha".to_string(),
            content: String::new(),
            ..pane.clone()
        }]);
        assert!(refused.is_err());
        assert_eq!(arrangement, before, "the running arrangement survived the refusal");
        let other = Pane { window: "beta".to_string(), ..pane };
        arrangement.write(vec![other.clone()]).expect("a valid replacement");
        assert_eq!(arrangement.read(), &[other]);
    }
}
