//! The process runtime: **the sixteen stages as processes**, scheduled at their own
//! cadences, with evidence.
//!
//! The reference's `world/process.rs` supplies the **contract**: a process declares a
//! scope, a simulation class, a cadence, its dependencies, the fields it reads and
//! writes, its conservation obligations, its replay fields, its residency policy and
//! the refusals it can raise. A scheduler runs the due processes in dependency order,
//! records per-process state, and emits evidence whose digests make a replay
//! checkable. That is the shape this module ports.
//!
//! # The roster is the stage roster — all sixteen, not a subset
//!
//! This tree's process decomposition **is** its stage decomposition
//! ([`crate::stages::ALL`], H-8.3). A [`ProcessId`] wraps a [`Stage`]; its cadence is
//! the stage's own rung of the ladder; its dependencies are the stage's own declared
//! couplings. There is no second roster to drift, and a stage added to the graph shows
//! up here as a process that is *due* and then **refuses by name** until it has an
//! admitted body.
//!
//! # What a body is, and what it is not
//!
//! A body ([`ProcessBody`]) is a pure function of the world spec, a [`Site`] and a
//! tick. It calls **the same shaper functions the world generator calls** and returns
//! [`ProcessReading`]s — the field values the stage produces at that site. Nothing
//! here is a second simulation:
//!
//! * `Tectonics` reads [`tectonics::sample`];
//! * `Terrain` reads [`terrain::sample`];
//! * `Geology` reads [`terrain::elevation_m`] and [`geology::stratum_at`];
//! * `Hydrology` reads [`hydrology::sample`];
//! * `Climate` reads [`climate::sample`] — **season included**: the season is the
//!   climate's own [`climate::season_phase`], not a second clock and not a counter;
//! * `Atmosphere` reads [`atmosphere::sample`];
//! * `Weather` reads [`weather::sample`];
//! * `Substrate` reads [`geology::stratum_at`] **at the site's own depth**,
//!   [`terrain::sample`], [`climate::moisture`] and [`substrate::soil_depth_m`],
//!   exactly as the field plane's soil law reads it; a single below-the-regolith
//!   depth is a column summary, not the material truth.
//!
//! Eight more bodies were admitted since: `Delta` reads the shaper's own deposition law
//! over the substrate baseline (its long-term `DeltaState` remains owed and is named),
//! `Transition` eats the **region form** [`ProcessRequest::region`] carries and refuses
//! by name when handed none, `Fields` and `Erosion` read the **unit**
//! [`ProcessRequest::unit`] carries — the former to *check* the ladder's level, the
//! latter through the stream-power law — and `SoilTransport` reads the region form
//! through the nonlinear hillslope-diffusion law (`erosion::hillslope_transport`,
//! Roering et al. 1999) and `Ecology` produces the two capacities the roster declares
//! (`ecology::carrying_capacity`, `ecology::species_suitability`, Lieth 1975 and
//! Holdridge 1947). Every one of them refuses by name when handed no region.
//!
//! The remaining two stages have **no admitted body**, and each refuses with its own
//! named reason ([`owed_body`]) rather than inventing one: cave occupancy is not a
//! declared field, and earthworks is the record rather than a process. The roster is every stage from the first tick, so in a full-roster step the
//! first owed stage in dependency order reports its own debt and the stages downstream
//! of a refusal report `DependencyUnavailable` until it clears. A body that *does*
//! exist must emit exactly the fields its [`ProcessSpec::outputs`] declares — a reading
//! the spec does not declare is refused, so the declaration cannot drift away from the
//! body.
//!
//! # The one adjustment from the reference's contract, stated
//!
//! The reference requires a spec's `inputs`, `outputs` and `replay_fields` to be
//! non-empty (every one of its processes is a state machine exchanging fields). This
//! roster includes stages whose honest I/O is partly or wholly absent today — so
//! validation here checks *consistency* (no duplicate entries, no self-dependency,
//! conservation and refusals declared) and no longer demands non-emptiness. The
//! emptiness is carried visibly in the spec and in the evidence instead of being
//! padded with a placeholder field.
use voxel_model::endian::LeBytes;

use std::collections::{BTreeMap, BTreeSet};

use voxel_model::{Identity, Refusal, RefusalKind, Refused};

use crate::chunk_fields::fixed_from_f64;
use crate::field_sampler::{FieldId, FieldValue, Unit};
use crate::spec::WorldSpec;
use crate::stages::{Cadence, Stage, ALL as ALL_STAGES, STAGE_COUNT};
use crate::{atmosphere, climate, delta, ecology, erosion, geology, hydrology, substrate, tectonics, terrain, transition, weather};

// -------------------------------------------------------------------------------------
// The vocabulary, ported from the reference's world/process.rs
// -------------------------------------------------------------------------------------

/// How much of the world a process is about.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum ProcessScope {
    RegionalField,
    GlobalField,
}

impl ProcessScope {
    pub const fn name(self) -> &'static str {
        match self {
            ProcessScope::RegionalField => "regional-field",
            ProcessScope::GlobalField => "global-field",
        }
    }
}

/// Which simulation family a process belongs to.
///
/// The reference declares three (`Climate`, `Atmosphere`, `Weather`) because its
/// process roster is the climate system. This tree's roster is all sixteen stages, so
/// the classification is extended with the families the roster actually contains —
/// a declared taxonomy, not a judgement about values.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum SimulationClass {
    Structural,
    Terrain,
    Geology,
    Hydrology,
    Atmosphere,
    Climate,
    Weather,
    Erosion,
    Caves,
    Biology,
    Materials,
    Work,
}

impl SimulationClass {
    pub const fn name(self) -> &'static str {
        match self {
            SimulationClass::Structural => "structural",
            SimulationClass::Terrain => "terrain",
            SimulationClass::Geology => "geology",
            SimulationClass::Hydrology => "hydrology",
            SimulationClass::Atmosphere => "atmosphere",
            SimulationClass::Climate => "climate",
            SimulationClass::Weather => "weather",
            SimulationClass::Erosion => "erosion",
            SimulationClass::Caves => "caves",
            SimulationClass::Biology => "biology",
            SimulationClass::Materials => "materials",
            SimulationClass::Work => "work",
        }
    }
}

/// How a process's answer is kept outside its own run.
///
/// Ported verbatim: `Exact` means the answer is the full-resolution one; `Aggregate`
/// means a declared aggregate stands in (and a process may not claim to have run
/// exactly when it was aggregated); `LazyAggregate` is the same, computed on read.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum ResidencyPolicy {
    Exact,
    Aggregate,
    LazyAggregate,
}

impl ResidencyPolicy {
    pub const fn name(self) -> &'static str {
        match self {
            ResidencyPolicy::Exact => "exact",
            ResidencyPolicy::Aggregate => "aggregate",
            ResidencyPolicy::LazyAggregate => "lazy-aggregate",
        }
    }
}

/// What a process promises not to do to the world's books.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum ConservationObligation {
    /// Matter may move; it may not be created or destroyed.
    NoMatterCreationOrDestruction,
    /// The water ledger closes across this process.
    HydrologyWaterBudget,
    /// This process reads biology and does not mutate it.
    NoEcologyMutation,
    /// This process changes no voxel.
    NoVoxelMutation,
}

impl ConservationObligation {
    pub const fn name(self) -> &'static str {
        match self {
            ConservationObligation::NoMatterCreationOrDestruction => {
                "no-matter-creation-or-destruction"
            }
            ConservationObligation::HydrologyWaterBudget => "hydrology-water-budget",
            ConservationObligation::NoEcologyMutation => "no-ecology-mutation",
            ConservationObligation::NoVoxelMutation => "no-voxel-mutation",
        }
    }
}

/// Why a process did not run, or what it refused with. Ported from the reference's
/// `RefusalCondition`, and named so a refusal is quotable without prose.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum RefusalCondition {
    MissingDeclaredField,
    WrongFixedScale,
    NegativeQuantity,
    IdentityMismatch,
    DependencyUnavailable,
    CadenceOverflow,
    NoMatchingBiomeRule,
    MissingSuitability,
    NegativeCarryingCapacity,
    UnadmittedProductionContract,
    ConservationFailure,
    InvalidDestination,
    InvalidInput,
}

impl RefusalCondition {
    pub const fn name(self) -> &'static str {
        match self {
            RefusalCondition::MissingDeclaredField => "missing-declared-field",
            RefusalCondition::WrongFixedScale => "wrong-fixed-scale",
            RefusalCondition::NegativeQuantity => "negative-quantity",
            RefusalCondition::IdentityMismatch => "identity-mismatch",
            RefusalCondition::DependencyUnavailable => "dependency-unavailable",
            RefusalCondition::CadenceOverflow => "cadence-overflow",
            RefusalCondition::NoMatchingBiomeRule => "no-matching-biome-rule",
            RefusalCondition::MissingSuitability => "missing-suitability",
            RefusalCondition::NegativeCarryingCapacity => "negative-carrying-capacity",
            RefusalCondition::UnadmittedProductionContract => "unadmitted-production-contract",
            RefusalCondition::ConservationFailure => "conservation-failure",
            RefusalCondition::InvalidDestination => "invalid-destination",
            RefusalCondition::InvalidInput => "invalid-input",
        }
    }
}

// -------------------------------------------------------------------------------------
// The roster
// -------------------------------------------------------------------------------------

/// A process is a **stage**. One roster, not two.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ProcessId(Stage);

impl ProcessId {
    pub const fn of(stage: Stage) -> Self {
        ProcessId(stage)
    }

    pub const fn stage(self) -> Stage {
        self.0
    }

    pub const fn name(self) -> &'static str {
        self.0.name()
    }
}

/// Every process, in the stage graph's own order. Hand-written for the same reason
/// [`ALL_STAGES`] is: a new stage must not silently miss this list.
pub const ALL: [ProcessId; STAGE_COUNT] = [
    ProcessId(Stage::Tectonics),
    ProcessId(Stage::Fields),
    ProcessId(Stage::Geology),
    ProcessId(Stage::Terrain),
    ProcessId(Stage::Hydrology),
    ProcessId(Stage::Erosion),
    ProcessId(Stage::Climate),
    ProcessId(Stage::Atmosphere),
    ProcessId(Stage::Weather),
    ProcessId(Stage::Caves),
    ProcessId(Stage::Substrate),
    ProcessId(Stage::SoilTransport),
    ProcessId(Stage::Ecology),
    ProcessId(Stage::Delta),
    ProcessId(Stage::Transition),
    ProcessId(Stage::Earthworks),
];

/// The declaration a process makes about itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessSpec {
    pub process: ProcessId,
    pub scope: ProcessScope,
    pub simulation_class: SimulationClass,
    /// The stage's own rung of the ladder — never a second cadence.
    pub cadence: Cadence,
    /// The fields the body reads.
    pub inputs: Vec<FieldId>,
    /// The fields the body produces. A body that emits anything else is refused.
    pub outputs: Vec<FieldId>,
    /// The processes that must run first: the stage graph's own dependency row.
    pub dependencies: Vec<ProcessId>,
    pub conservation: Vec<ConservationObligation>,
    /// The fields a replay must be able to reproduce this process's answer from.
    pub replay_fields: Vec<FieldId>,
    pub residency: ResidencyPolicy,
    pub refusal_conditions: Vec<RefusalCondition>,
}

impl ProcessSpec {
    /// Consistency, not non-emptiness: no duplicate entries, no self-dependency,
    /// conservation and refusals declared. See the module docs for why this departs
    /// from the reference's stricter check.
    pub fn validate(&self) -> Refused<()> {
        let unique = |list: &[FieldId], what: &str| -> Refused<()> {
            let before = list.len();
            let mut seen = BTreeSet::new();
            for field in list {
                if !seen.insert(*field) {
                    return Err(Refusal::owned(
                        RefusalKind::InvalidFieldSpec,
                        format!(
                            "process {} declares {field:?} twice in its {what}",
                            self.process.name()
                        ),
                    ));
                }
            }
            let _ = before;
            Ok(())
        };
        unique(&self.inputs, "inputs")?;
        unique(&self.outputs, "outputs")?;
        unique(&self.replay_fields, "replay fields")?;
        if self.dependencies.contains(&self.process) {
            return Err(Refusal::owned(
                RefusalKind::InvalidFieldSpec,
                format!("process {} depends on itself", self.process.name()),
            ));
        }
        let mut deps = BTreeSet::new();
        for dep in &self.dependencies {
            if !deps.insert(*dep) {
                return Err(Refusal::owned(
                    RefusalKind::InvalidFieldSpec,
                    format!(
                        "process {} declares dependency {} twice",
                        self.process.name(),
                        dep.name()
                    ),
                ));
            }
        }
        if self.conservation.is_empty() {
            return Err(Refusal::owned(
                RefusalKind::InvalidFieldSpec,
                format!(
                    "process {} declares no conservation obligation; a process that promises \
                     nothing is a process nothing can check",
                    self.process.name()
                ),
            ));
        }
        if self.refusal_conditions.is_empty() {
            return Err(Refusal::owned(
                RefusalKind::InvalidFieldSpec,
                format!(
                    "process {} declares no refusals; every process can refuse and the names \
                     are part of its contract",
                    self.process.name()
                ),
            ));
        }
        Ok(())
    }

    /// The identity of this declaration: **everything that can change what the process
    /// does**, length-prefixed so two different declarations cannot collide.
    pub fn identity(&self) -> Identity {
        let mut payload = Vec::new();
        field_str(&mut payload, self.process.name());
        field_u64(&mut payload, self.cadence.period_ticks());
        field_u8(&mut payload, scope_tag(self.scope));
        field_u8(&mut payload, class_tag(self.simulation_class));
        field_u8(&mut payload, residency_tag(self.residency));
        field_list(&mut payload, self.inputs.iter().map(|f| f.as_str()));
        field_list(&mut payload, self.outputs.iter().map(|f| f.as_str()));
        field_list(&mut payload, self.dependencies.iter().map(|d| d.name()));
        field_list(&mut payload, self.conservation.iter().map(|c| c.name()));
        field_list(&mut payload, self.replay_fields.iter().map(|f| f.as_str()));
        field_list(&mut payload, self.refusal_conditions.iter().map(|r| r.name()));
        Identity::of_fields("voxel-shapers/process-spec/v1", &[&payload])
    }
}

/// The full sixteen-process roster, with the stage graph's own couplings.
///
/// Scope, class, conservation and residency are declarations for each stage; the
/// inputs/outputs are exactly what the admitted body reads and produces (and the
/// tests pin that). A stage with no body still gets its row — the roster is bounded by
/// the **stage graph**, not by which bodies happen to be written, so the owed work is
/// visible as processes that refuse by name rather than as stages that silently do not
/// exist.
pub fn process_specs() -> Refused<Vec<ProcessSpec>> {
    let specs = ALL_STAGES
        .iter()
        .map(|stage| spec_for_stage(*stage))
        .collect::<Refused<Vec<_>>>()?;
    Ok(specs)
}

/// The seven declarations one stage's spec is assembled from, in the order the tuple below is
/// destructured — named so that the match arms and the reader agree on what position means what.
type StageDeclarations = (
    ProcessScope,
    SimulationClass,
    Vec<ConservationObligation>,
    ResidencyPolicy,
    Vec<FieldId>,
    Vec<FieldId>,
    Vec<FieldId>,
);

fn spec_for_stage(stage: Stage) -> Refused<ProcessSpec> {
    use ConservationObligation::*;
    use FieldId::*;
    let process = ProcessId::of(stage);
    let (scope, class, conservation, residency, inputs, outputs, replay): StageDeclarations = match stage {
        Stage::Tectonics => (
            ProcessScope::RegionalField,
            SimulationClass::Structural,
            vec![NoVoxelMutation],
            ResidencyPolicy::Exact,
            vec![],
            vec![Tectonics],
            vec![Tectonics],
        ),
        Stage::Fields => (
            ProcessScope::GlobalField,
            SimulationClass::Structural,
            vec![NoVoxelMutation],
            ResidencyPolicy::Exact,
            vec![Tectonics],
            vec![],
            vec![],
        ),
        Stage::Geology => (
            ProcessScope::RegionalField,
            SimulationClass::Geology,
            vec![NoVoxelMutation],
            ResidencyPolicy::Exact,
            vec![Elevation, Tectonics],
            vec![Geology],
            vec![Geology],
        ),
        Stage::Terrain => (
            ProcessScope::RegionalField,
            SimulationClass::Terrain,
            vec![NoVoxelMutation],
            ResidencyPolicy::Exact,
            vec![Tectonics],
            vec![Elevation, Slope],
            vec![Elevation, Slope],
        ),
        Stage::Hydrology => (
            ProcessScope::RegionalField,
            SimulationClass::Hydrology,
            vec![HydrologyWaterBudget],
            ResidencyPolicy::Exact,
            vec![Elevation, Moisture],
            vec![Groundwater],
            vec![Groundwater],
        ),
        Stage::Erosion => (
            ProcessScope::RegionalField,
            SimulationClass::Erosion,
            vec![NoMatterCreationOrDestruction],
            ResidencyPolicy::Exact,
            vec![Elevation, Slope, Soil],
            vec![ErosionResistance],
            vec![ErosionResistance],
        ),
        Stage::Climate => (
            ProcessScope::RegionalField,
            SimulationClass::Climate,
            vec![NoVoxelMutation],
            ResidencyPolicy::Exact,
            vec![Elevation],
            vec![Climate, Temperature, Moisture],
            vec![Climate, Temperature, Moisture],
        ),
        Stage::Atmosphere => (
            ProcessScope::RegionalField,
            SimulationClass::Atmosphere,
            vec![NoVoxelMutation],
            ResidencyPolicy::Exact,
            vec![Temperature],
            vec![Pressure],
            vec![Pressure],
        ),
        Stage::Weather => (
            ProcessScope::RegionalField,
            SimulationClass::Weather,
            vec![NoVoxelMutation],
            ResidencyPolicy::Exact,
            vec![Climate, Temperature, Moisture],
            vec![Precipitation, Wind],
            vec![Precipitation, Wind],
        ),
        Stage::Caves => (
            ProcessScope::RegionalField,
            SimulationClass::Caves,
            vec![NoVoxelMutation],
            ResidencyPolicy::Exact,
            vec![Geology, Groundwater],
            vec![],
            vec![],
        ),
        Stage::Substrate => (
            ProcessScope::RegionalField,
            SimulationClass::Materials,
            vec![NoVoxelMutation],
            ResidencyPolicy::Exact,
            vec![Geology, Slope, Moisture],
            vec![Soil],
            vec![Soil],
        ),
        Stage::SoilTransport => (
            ProcessScope::RegionalField,
            SimulationClass::Erosion,
            vec![NoMatterCreationOrDestruction],
            ResidencyPolicy::Exact,
            vec![Soil, Slope, Precipitation],
            vec![Soil],
            vec![Soil],
        ),
        Stage::Ecology => (
            ProcessScope::RegionalField,
            SimulationClass::Biology,
            vec![NoEcologyMutation],
            ResidencyPolicy::Exact,
            vec![Temperature, Moisture, Precipitation, Soil],
            vec![CarryingCapacity, SpeciesSuitability],
            vec![CarryingCapacity, SpeciesSuitability],
        ),
        Stage::Delta => (
            ProcessScope::RegionalField,
            SimulationClass::Hydrology,
            vec![NoMatterCreationOrDestruction],
            ResidencyPolicy::Exact,
            vec![Groundwater, Soil],
            vec![Soil],
            vec![Soil],
        ),
        Stage::Transition => (
            ProcessScope::RegionalField,
            SimulationClass::Materials,
            vec![NoVoxelMutation],
            ResidencyPolicy::Exact,
            vec![Geology, Soil],
            vec![Geology],
            vec![Geology],
        ),
        Stage::Earthworks => (
            ProcessScope::GlobalField,
            SimulationClass::Work,
            vec![NoMatterCreationOrDestruction],
            ResidencyPolicy::Exact,
            vec![],
            vec![],
            vec![],
        ),
    };
    let spec = ProcessSpec {
        process,
        scope,
        simulation_class: class,
        cadence: stage.cadence(),
        inputs,
        outputs,
        dependencies: stage.depends_on().iter().copied().map(ProcessId::of).collect(),
        conservation,
        replay_fields: replay,
        residency,
        refusal_conditions: vec![
            RefusalCondition::MissingDeclaredField,
            RefusalCondition::DependencyUnavailable,
            RefusalCondition::UnadmittedProductionContract,
            RefusalCondition::ConservationFailure,
        ],
    };
    spec.validate()?;
    Ok(spec)
}

// -------------------------------------------------------------------------------------
// A site, a reading, and the bodies
// -------------------------------------------------------------------------------------

/// Where a process is asked for an answer, in world voxels.
///
/// A `Site` is a **voxel**, not a column: the vertical axis is carried because
/// geology, caves and the atmosphere genuinely read it. The horizontal shapers
/// ignore it, and that is their business, not the runtime's.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Site {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Site {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Site { x, y, z }
    }

    pub fn fx(self) -> f64 {
        self.x as f64 + 0.5
    }

    pub fn fy(self) -> f64 {
        self.y as f64 + 0.5
    }

    pub fn fz(self) -> f64 {
        self.z as f64 + 0.5
    }
}

/// A **region**: the half-open voxel box one unit of work covers, with its level.
///
/// `Site` names a voxel; a structural stage's law is about an **area** — the field
/// ladder is addressed by a level, and the stream-power rate is a law over a
/// drainage area — which is exactly what [`owed_body`] said of `Fields` and
/// `Erosion` ("no world sampler at a site", "a unit/region body before it can
/// run"). This is that region form, borrowed from the caller's `UnitOfWork`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct UnitBox {
    pub min: [i32; 3],
    /// Exclusive upper corner.
    pub max: [i32; 3],
    pub level: u32,
}

impl UnitBox {
    pub const fn new(min: [i32; 3], max: [i32; 3], level: u32) -> Self {
        UnitBox { min, max, level }
    }

    /// The box's side in voxels along x.
    pub fn side(&self) -> i32 {
        self.max[0] - self.min[0]
    }

    /// Voxels in the box.
    pub fn cells(&self) -> i64 {
        (0..3).fold(1i64, |acc, a| acc * (self.max[a] - self.min[a]).max(0) as i64)
    }

    /// The box's centre voxel: the site a region body samples at.
    pub fn centre_site(&self) -> Site {
        Site::new(
            self.min[0] + self.side() / 2,
            self.min[1] + (self.max[1] - self.min[1]) / 2,
            self.min[2] + (self.max[2] - self.min[2]) / 2,
        )
    }

    /// The box's horizontal area in square metres, at one metre a voxel.
    pub fn area_m2(&self) -> f64 {
        (self.max[0] - self.min[0]).max(0) as f64 * (self.max[2] - self.min[2]).max(0) as f64
    }
}

/// One field value a process produced, with its unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProcessReading {
    pub field: FieldId,
    pub unit: Unit,
    pub value: FieldValue,
}

impl ProcessReading {
    pub fn fixed(field: FieldId, unit: Unit, value: f64) -> Refused<Self> {
        let value = FieldValue::Fixed(fixed_from_f64(value)?);
        Ok(ProcessReading { field, unit, value })
    }

    pub fn integer(field: FieldId, unit: Unit, value: i128) -> Self {
        ProcessReading { field, unit, value: FieldValue::Integer(value) }
    }
}

/// A process body: a pure function of the world spec and the request.
pub type ProcessBody = fn(&WorldSpec, &ProcessRequest<'_>) -> Refused<Vec<ProcessReading>>;

/// One neighbour a **region** stage reads: where it is, and how far from the site.
///
/// This is the minimal region form `owed_body(Transition)` names as the thing a
/// blend law needs and a lone site cannot supply: *"a blend law over neighbours and
/// a site alone cannot evaluate it"*. A neighbour carries its own distance because
/// the distance is what the weights are a function of — recomputing it from the
/// coordinates inside a body would let the caller's geometry and the shaper's
/// disagree.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct RegionSample {
    pub site: Site,
    pub distance_m: f64,
}

/// What a process is asked: where, at which tick, the one declared radius the
/// hydrology shaper needs, and the **region** a neighbour-reading stage blends.
///
/// The region is a borrowed slice rather than an owned `Vec` for the same reason
/// the roster is fixed: a request is a *view* of the caller's neighbourhood and
/// copying it would be a second record of it. A site stage ignores it, and that is
/// its business; a stage that needs it and is handed none refuses by name rather
/// than guessing (`transition_body`).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ProcessRequest<'a> {
    pub site: Site,
    pub tick: u64,
    /// The hydrology watershed radius. Explicit because shrinking it shrinks the
    /// watershed rather than approximating the same one — the shaper's own words.
    pub hydro_radius: usize,
    /// The neighbours a region stage reads, by distance. Empty is a site request.
    pub region: &'a [RegionSample],
    /// The **unit** a structural stage reads. `None` is a site request, and a
    /// structural body handed none refuses by name rather than inventing one.
    pub unit: Option<UnitBox>,
}

/// The body a stage has **today**, or `None` with the debt named by [`owed_body`].
pub fn body_of(stage: Stage) -> Option<ProcessBody> {
    match stage {
        Stage::Tectonics => Some(tectonics_body),
        Stage::Geology => Some(geology_body),
        Stage::Terrain => Some(terrain_body),
        Stage::Hydrology => Some(hydrology_body),
        Stage::Climate => Some(climate_body),
        Stage::Atmosphere => Some(atmosphere_body),
        Stage::Weather => Some(weather_body),
        Stage::Substrate => Some(substrate_body),
        Stage::Delta => Some(delta_body),
        Stage::Transition => Some(transition_body),
        Stage::Fields => Some(fields_body),
        Stage::Erosion => Some(erosion_body),
        Stage::SoilTransport => Some(soil_transport_body),
        Stage::Ecology => Some(ecology_body),
        Stage::Caves | Stage::Earthworks => None,
    }
}

/// Why a stage has no body yet. Named, one line per stage, so the refusal quotes the
/// actual owed work rather than "not implemented".
///
/// **Only the two stages that genuinely have none appear here.** The earlier arms for
/// `Fields`, `Erosion`, `Delta`, `Transition`, `SoilTransport` and `Ecology` were the
/// debts of their day; each of those stages has a body now, so the arm would be a stale
/// claim about a stage that runs. `Earthworks` is the third and it is **by design**: it
/// is the modification record, and running it as a generator would make the record a
/// simulation.
pub fn owed_body(stage: Stage) -> &'static str {
    match stage {
        Stage::Caves => {
            "the cave answer (open/flooded/rock) is not one of the nineteen declared fields; \
             cave occupancy's field is owed by name"
        }
        Stage::Earthworks => {
            "earthworks is the modification record, not a field process; it refuses because \
             running it as one would make the record a generator"
        }
        _ => "this stage has a body; owed_body was asked about the wrong one",
    }
}

fn tectonics_body(world: &WorldSpec, request: &ProcessRequest) -> Refused<Vec<ProcessReading>> {
    let sample = tectonics::sample(world, request.site.fx(), request.site.fz());
    Ok(vec![ProcessReading::integer(
        FieldId::Tectonics,
        Unit::Dimensionless,
        sample.plate_id as i128,
    )])
}

fn terrain_body(world: &WorldSpec, request: &ProcessRequest) -> Refused<Vec<ProcessReading>> {
    let sample = terrain::sample(world, request.site.fx(), request.site.fz());
    Ok(vec![
        ProcessReading::fixed(FieldId::Elevation, Unit::Metre, sample.elevation_m)?,
        ProcessReading::fixed(FieldId::Slope, Unit::Ratio, sample.slope)?,
    ])
}

fn geology_body(world: &WorldSpec, request: &ProcessRequest) -> Refused<Vec<ProcessReading>> {
    let site = request.site;
    let surface = terrain::elevation_m(world, site.fx(), site.fz());
    let stratum = geology::stratum_at(world, site.fx(), site.fz(), surface, site.fy());
    Ok(vec![ProcessReading::integer(
        FieldId::Geology,
        Unit::Dimensionless,
        crate::chunk_fields::lithology_code(stratum.lithology),
    )])
}

fn hydrology_body(world: &WorldSpec, request: &ProcessRequest) -> Refused<Vec<ProcessReading>> {
    let sample = hydrology::sample(
        world,
        request.site.fx(),
        request.site.fz(),
        request.hydro_radius,
    );
    // The declared field this stage produces is `Groundwater`; the reading is the
    // water depth the shaper reports at this column. The field's name is the
    // roster's, and the surface-water/groundwater naming is recorded as debt rather
    // than silently renamed.
    Ok(vec![ProcessReading::fixed(
        FieldId::Groundwater,
        Unit::Metre,
        sample.water_depth_m,
    )?])
}

fn climate_body(world: &WorldSpec, request: &ProcessRequest) -> Refused<Vec<ProcessReading>> {
    let sample = climate::sample(world, request.site.fx(), request.site.fz(), request.tick);
    Ok(vec![
        ProcessReading::fixed(FieldId::Climate, Unit::Ratio, sample.season_phase)?,
        ProcessReading::fixed(FieldId::Temperature, Unit::DegreesCelsius, sample.temperature_c)?,
        ProcessReading::fixed(FieldId::Moisture, Unit::Ratio, sample.moisture)?,
    ])
}

fn atmosphere_body(world: &WorldSpec, request: &ProcessRequest) -> Refused<Vec<ProcessReading>> {
    let sample = atmosphere::sample(world, request.site.fy());
    // `AirSample` also reports density and a thin-air flag; neither has a declared
    // field in the nineteen-name roster, so neither is emitted (the omission is
    // recorded rather than encoded as a dimensionless number).
    Ok(vec![ProcessReading::fixed(
        FieldId::Pressure,
        Unit::Pascal,
        sample.pressure_kpa * 1000.0,
    )?])
}

fn weather_body(world: &WorldSpec, request: &ProcessRequest) -> Refused<Vec<ProcessReading>> {
    let sample = weather::sample(world, request.site.fx(), request.site.fz(), request.tick);
    // Precipitation is reported in mm and carried in metres; the bearing is an
    // angle and the unit roster has no radian, so it is `Dimensionless` — the
    // honest absence of a unit rather than a ratio it is not.
    Ok(vec![
        ProcessReading::fixed(
            FieldId::Precipitation,
            Unit::Metre,
            sample.precipitation_mm / 1000.0,
        )?,
        ProcessReading::fixed(FieldId::Wind, Unit::Dimensionless, sample.wind_bearing_rad)?,
    ])
}

fn substrate_body(world: &WorldSpec, request: &ProcessRequest) -> Refused<Vec<ProcessReading>> {
    let site = request.site;
    let surface = terrain::elevation_m(world, site.fx(), site.fz());
    // The stratum is sampled at **the site's own depth**, exactly as the field
    // plane's soil law reads it: a column whose rock changes with depth changes its
    // soil with depth too. A below-the-regolith reference depth would be a column
    // summary, not the material truth this process produces.
    let stratum = geology::stratum_at(world, site.fx(), site.fz(), surface, site.fy());
    let slope = terrain::sample(world, site.fx(), site.fz()).slope;
    let moisture = climate::moisture(world, site.fx(), site.fz());
    let depth = substrate::soil_depth_m(world, stratum.lithology, slope, moisture);
    Ok(vec![ProcessReading::fixed(FieldId::Soil, Unit::Metre, depth)?])
}

/// **Delta**, as a *reading* and not as its long-term state.
///
/// The shaper is a law plus a state (`DeltaState`: accumulation and avulsion). The
/// runtime holds no state, so this body gives the one thing a site can honestly
/// answer — the deposition the delta's own law places at the mouth this tick, added
/// to the substrate baseline the `Substrate` body already computes. **The
/// accumulation and the avulsion remain owed and are named, not simulated by a
/// counter**: a delta that remembered would be a second simulation, which is the
/// thing this module's contract forbids.
fn delta_body(world: &WorldSpec, request: &ProcessRequest<'_>) -> Refused<Vec<ProcessReading>> {
    let site = request.site;
    let surface = terrain::elevation_m(world, site.fx(), site.fz());
    let slope = terrain::sample(world, site.fx(), site.fz()).slope;
    let standing_water_m = hydrology::sample(
        world,
        site.fx(),
        site.fz(),
        request.hydro_radius,
    )
    .water_depth_m;
    let stratum = geology::stratum_at(world, site.fx(), site.fz(), surface, site.fy());
    let moisture = climate::moisture(world, site.fx(), site.fz());
    let baseline_soil_m = substrate::soil_depth_m(world, stratum.lithology, slope, moisture);
    // The shaper's own three steps, never a second velocity or a second threshold.
    let velocity = delta::velocity_m_s(slope, standing_water_m);
    let capacity = delta::transport_capacity(velocity);
    let deposited_m = delta::deposition_m(capacity, standing_water_m, slope);
    Ok(vec![ProcessReading::fixed(
        FieldId::Soil,
        Unit::Metre,
        baseline_soil_m + deposited_m,
    )?])
}

/// The lithology of the stratum at a site, through the same path every other body uses.
fn lithology_at(world: &WorldSpec, site: Site) -> crate::geology::Lithology {
    let surface = terrain::elevation_m(world, site.fx(), site.fz());
    geology::stratum_at(world, site.fx(), site.fz(), surface, site.fy()).lithology
}

/// **Transition**, evaluated over the region form.
///
/// A boundary between two domains cannot be seen from one point, which is exactly
/// why the body was owed: this reads the neighbours the request carries and lets the
/// shaper's own [`transition::blend_class`] decide. Two rules are the module's and
/// are not restated here — the weights are a partition of unity (Shepard 1968), and
/// a **tie refuses** rather than inventing a rock that is half granite.
///
/// A request with **no region** refuses by name: a transition body handed a lone sit
/// cannot answer, and guessing one side would be the plausible substitute this
/// repository's first rule forbids.
fn transition_body(world: &WorldSpec, request: &ProcessRequest<'_>) -> Refused<Vec<ProcessReading>> {
    if request.region.is_empty() {
        return Err(Refusal::new(
            RefusalKind::MissingStage,
            "a transition needs the region form: the request carried no neighbours, so the \
             boundary cannot be evaluated from one site",
        ));
    }
    let distances: Vec<f64> = request.region.iter().map(|n| n.distance_m).collect();
    let weights = transition::shepard_weights(&distances)?;
    let classes: Vec<(crate::geology::Lithology, f64)> = request
        .region
        .iter()
        .zip(weights.iter())
        .map(|(n, w)| (lithology_at(world, n.site), *w))
        .collect();
    let blended = transition::blend_class(&classes)?;
    Ok(vec![ProcessReading::integer(
        FieldId::Geology,
        Unit::Dimensionless,
        crate::chunk_fields::lithology_code(blended),
    )])
}

/// **Fields**, over the region form.
///
/// The ladder carries no per-site scalar — it defines the *level* at which the
/// regional fields are addressed, and its consumers (`terrain`, `geology`,
/// `substrate`) sample it directly. So the body's real work is the **check**: a unit
/// whose level is outside the ladder is refused by name here rather than silently
/// producing a field at a level the ladder does not have. It emits nothing, and that
/// is a declaration, not an omission.
fn fields_body(_world: &WorldSpec, request: &ProcessRequest<'_>) -> Refused<Vec<ProcessReading>> {
    let Some(unit) = request.unit else {
        return Err(Refusal::new(
            RefusalKind::MissingStage,
            "fields needs the region form: the ladder is addressed by a unit's level, and the \
             request carried no unit",
        ));
    };
    if unit.level >= crate::fields::STANDARD_LEVELS {
        return Err(Refusal::owned(
            RefusalKind::IndexOutOfRange,
            format!(
                "field level {} is outside the {}-level ladder",
                unit.level,
                crate::fields::STANDARD_LEVELS
            ),
        ));
    }
    Ok(Vec::new())
}

/// **Erosion**, over the region form — the stream-power law the droplet method does
/// not supply.
///
/// `E = K · A^m · S^n` (Whipple & Tucker 1999; Lague 2014, `m/n ≈ 0.5`). The unit's
/// own horizontal area is the drainage surrogate at this rung, widened by the water
/// the column carries; the reading is the **resistance** the surface presents, the
/// reciprocal of the incision. A unit with no region refuses by name, because the
/// rate is a law about an area.
fn erosion_body(world: &WorldSpec, request: &ProcessRequest<'_>) -> Refused<Vec<ProcessReading>> {
    let Some(unit) = request.unit else {
        return Err(Refusal::new(
            RefusalKind::MissingStage,
            "erosion needs the region form: the stream-power rate is a law about a drainage \
             area, and the request carried no unit",
        ));
    };
    let site = unit.centre_site();
    let slope = terrain::sample(world, site.fx(), site.fz()).slope;
    let water_m = hydrology::sample(world, site.fx(), site.fz(), request.hydro_radius).water_depth_m;
    let drainage_m2 = unit.area_m2() * (1.0 + water_m);
    let incision = erosion::stream_power_rate(drainage_m2, slope);
    let resistance = 1.0 / (1.0 + incision);
    Ok(vec![ProcessReading::fixed(
        FieldId::ErosionResistance,
        Unit::Ratio,
        resistance,
    )?])
}

/// The step a soil-transport body advances, in seconds. **Declared**, because the law is
/// a flux (a rate) and the field is a depth, so the two are only convertible through a
/// duration; this is it, at the `Cadence::Sediment` rung's own rate.
pub const SOIL_TRANSPORT_STEP_S: f64 = 1.0;

/// **The discrete operator `SoilTransport` uses**, stated rather than implied.
///
/// The law (`erosion::hillslope_transport`, Roering, Kirchner & Dietrich 1999) turns a
/// **slope** into a **flux** `q = K·S/(1−(S/S_c)²)`. A region supplies one *pairwise*
/// slope per neighbour — `(z_neighbour − z_site) / distance` — and the site's own soil
/// change is the sum of the directed fluxes, each of which moves material **toward the
/// lower** of the two: an uphill neighbour delivers soil to the site, a downhill one
/// takes it away. Because the two ends of a pair compute the same magnitude with
/// opposite signs, a pair exchanges equal and opposite material by construction, so a
/// **closed** region conserves exactly and an open one exports across its boundary — the
/// same fact `erosion::Erosion::exported_m3` already states for the droplet method.
///
/// The sum is scaled by [`SOIL_TRANSPORT_STEP_S`]. `None` when a pairwise slope is at or
/// above [`erosion::CRITICAL_SLOPE`], where the nonlinear law diverges, or when a
/// neighbour is coincident with the site: the caller refuses **by name** rather than
/// reading a number that is not the law's.
pub fn soil_transport_delta_m(site_elevation_m: f64, neighbours: &[(f64, f64)]) -> Option<f64> {
    if !site_elevation_m.is_finite() {
        return None;
    }
    let mut delta_m = 0.0;
    for (neighbour_elevation_m, distance_m) in neighbours {
        if !neighbour_elevation_m.is_finite() || !distance_m.is_finite() || *distance_m <= 0.0 {
            return None;
        }
        let gradient = (neighbour_elevation_m - site_elevation_m) / distance_m;
        let flux = erosion::hillslope_transport(gradient.abs())?;
        delta_m += if gradient > 0.0 { flux } else { -flux };
    }
    Some(delta_m * SOIL_TRANSPORT_STEP_S)
}

/// **SoilTransport**, over the region form — nonlinear hillslope diffusion.
///
/// `owed_body` used to say *"a material-moving stage with no admitted law yet — the
/// baseline is substrate's and the move is owed"*. The law landed in `erosion.rs`
/// (Roering et al. 1999) and [`soil_transport_delta_m`] is the move, so the body is the
/// substrate baseline plus the move. A request with **no region** refuses by name, and a
/// move that would take more soil than the site holds refuses too rather than clamping —
/// the site cannot export material it does not have.
fn soil_transport_body(
    world: &WorldSpec,
    request: &ProcessRequest<'_>,
) -> Refused<Vec<ProcessReading>> {
    if request.region.is_empty() {
        return Err(Refusal::new(
            RefusalKind::MissingStage,
            "soil transport needs the region form: a hillslope flux is a gradient between \
             neighbours, and a site alone has none",
        ));
    }
    let site = request.site;
    let surface = terrain::elevation_m(world, site.fx(), site.fz());
    let slope = terrain::sample(world, site.fx(), site.fz()).slope;
    let stratum = geology::stratum_at(world, site.fx(), site.fz(), surface, site.fy());
    let moisture = climate::moisture(world, site.fx(), site.fz());
    let baseline_m = substrate::soil_depth_m(world, stratum.lithology, slope, moisture);
    let pairs: Vec<(f64, f64)> = request
        .region
        .iter()
        .map(|n| (terrain::elevation_m(world, n.site.fx(), n.site.fz()), n.distance_m))
        .collect();
    let Some(delta_m) = soil_transport_delta_m(surface, &pairs) else {
        return Err(Refusal::new(
            RefusalKind::OutOfDeclaredRange,
            "soil transport refuses: a pairwise slope is at or beyond the critical slope, \
             or a neighbour is coincident with the site",
        ));
    };
    let after_m = baseline_m + delta_m;
    if after_m < 0.0 {
        return Err(Refusal::owned(
            RefusalKind::OutOfDeclaredRange,
            format!(
                "soil transport refuses: the move would take {:.6} m from a site holding \
                 {baseline_m:.6} m of soil",
                -delta_m
            ),
        ));
    }
    Ok(vec![ProcessReading::fixed(FieldId::Soil, Unit::Metre, after_m)?])
}

/// **Ecology**, the two capacities the roster declares.
///
/// The biome is a *class* and was already a field law; the two fields that had **no law**
/// are the capacities. They are now Lieth's Miami productivity (Lieth 1975) and
/// Holdridge's humidity ratio (Holdridge 1947), read through
/// [`ecology::carrying_capacity`] and [`ecology::species_suitability`]. The inputs are the
/// **same sampled numbers every other body reads** — `holdridge_inputs`' biotemperature
/// and mean annual precipitation, and `substrate`'s soil depth — so a capacity and the
/// ground it describes cannot be reading two different columns.
///
/// `MissingSuitability` and `NegativeCarryingCapacity` are the two refusals the roster
/// already declared for this stage: a climate the capacity law cannot evaluate refuses by
/// name rather than producing a zero that would read as a barren province.
fn ecology_body(world: &WorldSpec, request: &ProcessRequest<'_>) -> Refused<Vec<ProcessReading>> {
    let site = request.site;
    let surface = terrain::elevation_m(world, site.fx(), site.fz());
    let (biotemperature_c, precipitation_mm) =
        ecology::holdridge_inputs(world, site.fx(), site.fz(), surface, request.tick);
    let slope = terrain::sample(world, site.fx(), site.fz()).slope;
    let stratum = geology::stratum_at(world, site.fx(), site.fz(), surface, site.fy());
    let moisture = climate::moisture(world, site.fx(), site.fz());
    let soil_m = substrate::soil_depth_m(world, stratum.lithology, slope, moisture);
    let capacity = ecology::carrying_capacity(biotemperature_c, precipitation_mm, soil_m)
        .ok_or_else(|| {
            Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!(
                    "ecology has no carrying capacity at biotemperature {biotemperature_c} °C, \
                     {precipitation_mm} mm/yr and {soil_m} m of soil"
                ),
            )
        })?;
    let suitability = ecology::species_suitability(biotemperature_c, precipitation_mm, soil_m)
        .ok_or_else(|| {
            Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                "ecology has no species suitability: the site's evapotranspiration, rain or \
                 soil is not evaluable",
            )
        })?;
    Ok(vec![
        ProcessReading::fixed(FieldId::CarryingCapacity, Unit::Ratio, capacity)?,
        ProcessReading::fixed(FieldId::SpeciesSuitability, Unit::Ratio, suitability)?,
    ])
}

/// The watershed radius a field-law request carries. **Declared**: a law about groundwater
/// needs a radius, and the field plane is a per-column read with no caller to supply one,
/// so this is it.
pub const FIELD_PLANE_HYDRO_RADIUS: usize = 4;

/// The stage that **owns** a field: the first stage, in the graph's order, whose declared
/// `outputs` name it.
///
/// This is read from the specs' own declarations rather than kept as a second table, so a
/// field whose arm was added to a spec is found on the next call and a field no stage
/// claims (`SoilFertility`, `Humidity`, `Storms`, `Radiation`) is `None` — and so the
/// plane's producer cannot drift away from the roster's declarations.
pub fn owning_stage(field: FieldId) -> Option<Stage> {
    ALL_STAGES.iter().copied().find(|stage| {
        spec_for_stage(*stage)
            .map(|spec| spec.outputs.contains(&field))
            .unwrap_or(false)
    })
}

/// The 16³ unit at octree level 4 **centred on a site**, for a field law that is about an
/// area.
///
/// Centred so the reading is at the coordinate asked for: `UnitBox::centre_site` is the
/// site again, which is what lets the structural bodies (`Fields`, `Erosion`) answer a
/// per-voxel query without offsetting it by half a unit.
fn unit_at(site: Site) -> UnitBox {
    const HALF: i32 = 8;
    const LEVEL: u32 = 4;
    let min = [site.x - HALF, site.y - HALF, site.z - HALF];
    UnitBox::new(min, [min[0] + 2 * HALF, min[1] + 2 * HALF, min[2] + 2 * HALF], LEVEL)
}

/// The **field plane's production seam**: the value of a field the process roster
/// produces, at a site and a tick.
///
/// This is how the roster stops being a runtime with no caller. `chunk_fields::law_of`
/// routes a declared-but-unproduced field here; this finds the owning stage from that
/// stage's own `outputs` declaration, builds the request the body reads, and returns the
/// reading. Nothing here is a second simulation — the body is the same one the scheduler
/// drives, and the same `execute_body` gate refuses an undeclared output.
///
/// The request carries the field's own **unit** (centred on the site) because the
/// structural bodies are laws about an area, and it carries **no region**, so a
/// neighbour-reading body (`Transition`, `SoilTransport`) refuses by name here rather than
/// being handed a fabricated neighbourhood.
pub fn field_at(world: &WorldSpec, field: FieldId, site: Site, tick: u64) -> Refused<FieldValue> {
    let stage = owning_stage(field).ok_or_else(|| {
        Refusal::owned(
            RefusalKind::Unimplemented,
            format!(
                "field {} is named by no stage's declaration in this roster",
                field.as_str()
            ),
        )
    })?;
    let spec = spec_for_stage(stage)?;
    let request = ProcessRequest {
        site,
        tick,
        hydro_radius: FIELD_PLANE_HYDRO_RADIUS,
        region: &[],
        unit: Some(unit_at(site)),
    };
    let readings = execute_body(&spec, world, &request)?;
    readings
        .iter()
        .find(|reading| reading.field == field)
        .map(|reading| reading.value)
        .ok_or_else(|| {
            Refusal::owned(
                RefusalKind::Unimplemented,
                format!(
                    "stage {} ran but produced no {} reading",
                    stage.name(),
                    field.as_str()
                ),
            )
        })
}

fn execute_body(
    spec: &ProcessSpec,
    world: &WorldSpec,
    request: &ProcessRequest<'_>,
) -> Refused<Vec<ProcessReading>> {
    let stage = spec.process.stage();
    let Some(body) = body_of(stage) else {
        return Err(Refusal::owned(
            RefusalKind::Unimplemented,
            format!(
                "process {} has no admitted body in this cut: {}",
                spec.process.name(),
                owed_body(stage)
            ),
        ));
    };
    let readings = body(world, request)?;
    // The declared outputs are load-bearing: a body that produces a field the spec
    // does not declare has produced something nobody asked for, and the evidence
    // would hide it.
    if let Some(undeclared) = readings
        .iter()
        .find(|reading| !spec.outputs.contains(&reading.field))
    {
        return Err(Refusal::owned(
            RefusalKind::InvalidFieldSpec,
            format!(
                "process {} produced {:?}, which its declaration does not list as an output",
                spec.process.name(),
                undeclared.field
            ),
        ));
    }
    Ok(readings)
}

// -------------------------------------------------------------------------------------
// State and evidence
// -------------------------------------------------------------------------------------

/// What the scheduler knows about one process after its runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessState {
    last_run: Option<u64>,
    run_count: u64,
    last_refusal: Option<RefusalCondition>,
    last_identity: Option<Identity>,
}

impl Default for ProcessState {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessState {
    pub const fn new() -> Self {
        ProcessState { last_run: None, run_count: 0, last_refusal: None, last_identity: None }
    }

    pub const fn last_run(&self) -> Option<u64> {
        self.last_run
    }

    pub const fn run_count(&self) -> u64 {
        self.run_count
    }

    pub const fn last_refusal(&self) -> Option<RefusalCondition> {
        self.last_refusal
    }

    pub const fn last_identity(&self) -> Option<Identity> {
        self.last_identity
    }
}

/// What one process did, or refused to do, at one tick.
///
/// The reference's evidence carries mutating outputs and accounts; this tree's
/// processes are **pure**, so the evidence carries **readings** — the field values the
/// stage produced — and the four identities that make the run quotable:
/// `spec` (what was declared), `input` (what was asked), `output` (what came back),
/// `state` (the tick this answer belongs to), and `replay` (all four, bound).
#[derive(Clone, Debug, PartialEq)]
pub struct ProcessEvidence {
    pub process: ProcessId,
    pub tick: u64,
    pub residency: ResidencyPolicy,
    pub readings: Vec<ProcessReading>,
    pub refusal: Option<RefusalCondition>,
    /// The named reason, present exactly when `refusal` is.
    pub detail: Option<String>,
    pub spec: Identity,
    pub input: Identity,
    pub output: Identity,
    pub state: Identity,
    pub replay: Identity,
}

impl ProcessEvidence {
    pub fn ran(&self) -> bool {
        self.refusal.is_none()
    }

    pub fn reading(&self, field: FieldId) -> Option<&ProcessReading> {
        self.readings.iter().find(|r| r.field == field)
    }
}

// -------------------------------------------------------------------------------------
// The scheduler
// -------------------------------------------------------------------------------------

/// The process registry and its clock. It holds the specs, the per-process state, and
/// the last tick it was asked for — **nothing about the world**, which stays a pure
/// function of the spec and the tick.
#[derive(Clone, Debug, PartialEq)]
pub struct ProcessScheduler {
    specs: Vec<ProcessSpec>,
    state: BTreeMap<ProcessId, ProcessState>,
    last_tick: Option<u64>,
}

impl ProcessScheduler {
    /// Build a scheduler over a roster, refusing duplicates, invalid specs and
    /// dependency cycles **by name**.
    pub fn new(specs: Vec<ProcessSpec>) -> Refused<Self> {
        let mut seen = BTreeSet::new();
        for spec in &specs {
            spec.validate()?;
            if !seen.insert(spec.process) {
                return Err(Refusal::owned(
                    RefusalKind::InvalidFieldSpec,
                    format!("process {} is declared twice", spec.process.name()),
                ));
            }
        }
        let state = specs.iter().map(|spec| (spec.process, ProcessState::new())).collect();
        let scheduler = ProcessScheduler { specs, state, last_tick: None };
        scheduler.validate_acyclic()?;
        Ok(scheduler)
    }

    /// The built-in roster: every stage, in the stage graph's order.
    pub fn world_defaults() -> Refused<Self> {
        Self::new(process_specs()?)
    }

    pub fn specs(&self) -> &[ProcessSpec] {
        &self.specs
    }

    pub fn spec(&self, process: ProcessId) -> Option<&ProcessSpec> {
        self.specs.iter().find(|spec| spec.process == process)
    }

    pub fn state(&self, process: ProcessId) -> Option<&ProcessState> {
        self.state.get(&process)
    }

    pub const fn last_tick(&self) -> Option<u64> {
        self.last_tick
    }

    /// The declared processes **due at `tick`**, in dependency order.
    ///
    /// Ties are broken by the stage graph's own order ([`Stage::index_in_all`]), so the
    /// answer is a function of the roster and not of iteration luck.
    pub fn due(&self, tick: u64) -> Vec<ProcessId> {
        let mut ids: Vec<ProcessId> = self
            .specs
            .iter()
            .filter(|spec| spec.cadence.is_due_at(tick))
            .map(|spec| spec.process)
            .collect();
        ids.sort_by_key(|id| (self.dependency_depth(*id), id.stage().index_in_all()));
        ids
    }

    /// Run every due process at the request's tick, in order, with evidence.
    ///
    /// Refusals are recorded, not thrown: a process whose dependency refused itself
    /// refuses with [`RefusalCondition::DependencyUnavailable`], and a process with no
    /// admitted body refuses with [`RefusalCondition::UnadmittedProductionContract`]
    /// and the named debt in `detail`. The same tick asked twice is idempotent; an
    /// older tick is a refusal, because a schedule that can run backwards is a
    /// schedule that can produce two worlds from one.
    pub fn step(
        &mut self,
        world: &WorldSpec,
        request: &ProcessRequest<'_>,
    ) -> Refused<Vec<ProcessEvidence>> {
        let tick = request.tick;
        if let Some(last) = self.last_tick {
            if tick == last {
                return Ok(Vec::new());
            }
            if tick < last {
                return Err(Refusal::owned(
                    RefusalKind::OutOfDeclaredRange,
                    format!("process tick {tick} is older than the last tick {last}"),
                ));
            }
        }
        let mut evidence = Vec::new();
        for process in self.due(tick) {
            let spec = self
                .spec(process)
                .expect("a due process is a declared process")
                .clone();
            let input = input_identity(&spec, request);
            let state = state_identity(process, tick);
            if let Some(condition) = self.dependency_refusal(&spec, tick) {
                let output = output_identity(&[]);
                let replay = replay_identity(spec.identity(), input, output, state);
                self.record(process, tick, Some(condition), replay);
                evidence.push(ProcessEvidence {
                    process,
                    tick,
                    residency: spec.residency,
                    readings: Vec::new(),
                    refusal: Some(condition),
                    detail: Some(format!(
                        "process {} did not run because a dependency it reads was unavailable",
                        process.name()
                    )),
                    spec: spec.identity(),
                    input,
                    output,
                    state,
                    replay,
                });
                continue;
            }
            match execute_body(&spec, world, request) {
                Ok(readings) => {
                    let output = output_identity(&readings);
                    let replay = replay_identity(spec.identity(), input, output, state);
                    self.record(process, tick, None, replay);
                    evidence.push(ProcessEvidence {
                        process,
                        tick,
                        residency: spec.residency,
                        readings,
                        refusal: None,
                        detail: None,
                        spec: spec.identity(),
                        input,
                        output,
                        state,
                        replay,
                    });
                }
                Err(refusal) => {
                    let condition = refusal_condition(refusal.kind());
                    let output = output_identity(&[]);
                    let replay = replay_identity(spec.identity(), input, output, state);
                    self.record(process, tick, Some(condition), replay);
                    evidence.push(ProcessEvidence {
                        process,
                        tick,
                        residency: spec.residency,
                        readings: Vec::new(),
                        refusal: Some(condition),
                        detail: Some(refusal.detail().to_owned()),
                        spec: spec.identity(),
                        input,
                        output,
                        state,
                        replay,
                    });
                }
            }
        }
        self.last_tick = Some(tick);
        Ok(evidence)
    }

    fn record(
        &mut self,
        process: ProcessId,
        tick: u64,
        refusal: Option<RefusalCondition>,
        replay: Identity,
    ) {
        let state = self
            .state
            .get_mut(&process)
            .expect("a recorded process is a declared process");
        state.last_run = Some(tick);
        state.run_count = state.run_count.saturating_add(1);
        state.last_refusal = refusal;
        state.last_identity = Some(replay);
    }

    /// A dependency refuses if it is not declared, refused on a run, or has never run
    /// while the tick is not the first one. Ported from the reference's
    /// `dependency_refusal`.
    fn dependency_refusal(&self, spec: &ProcessSpec, tick: u64) -> Option<RefusalCondition> {
        for dependency in &spec.dependencies {
            let Some(state) = self.state.get(dependency) else {
                return Some(RefusalCondition::DependencyUnavailable);
            };
            if state.last_refusal.is_some() || (state.last_run.is_none() && tick != 0) {
                return Some(RefusalCondition::DependencyUnavailable);
            }
        }
        None
    }

    fn dependency_depth(&self, process: ProcessId) -> usize {
        self.spec(process)
            .map(|spec| {
                1 + spec
                    .dependencies
                    .iter()
                    .map(|dep| self.dependency_depth(*dep))
                    .max()
                    .unwrap_or(0)
            })
            .unwrap_or(0)
    }

    fn validate_acyclic(&self) -> Refused<()> {
        fn visit(
            process: ProcessId,
            specs: &BTreeMap<ProcessId, &ProcessSpec>,
            seen: &mut BTreeSet<ProcessId>,
            done: &mut BTreeSet<ProcessId>,
        ) -> Refused<()> {
            if done.contains(&process) {
                return Ok(());
            }
            if !seen.insert(process) {
                return Err(Refusal::owned(
                    RefusalKind::InvalidFieldSpec,
                    format!(
                        "process dependency cycle: {} is reachable from its own dependencies",
                        process.name()
                    ),
                ));
            }
            if let Some(spec) = specs.get(&process) {
                for dep in &spec.dependencies {
                    if specs.contains_key(dep) {
                        visit(*dep, specs, seen, done)?;
                    }
                }
            }
            done.insert(process);
            Ok(())
        }
        let specs: BTreeMap<ProcessId, &ProcessSpec> =
            self.specs.iter().map(|spec| (spec.process, spec)).collect();
        let mut seen = BTreeSet::new();
        let mut done = BTreeSet::new();
        for spec in &self.specs {
            visit(spec.process, &specs, &mut seen, &mut done)?;
        }
        Ok(())
    }
}

// -------------------------------------------------------------------------------------
// Identities
// -------------------------------------------------------------------------------------

fn input_identity(spec: &ProcessSpec, request: &ProcessRequest) -> Identity {
    let mut payload = Vec::new();
    payload.extend_from_slice(spec.identity().digest().as_bytes());
    field_u64(&mut payload, request.tick);
    field_i32(&mut payload, request.site.x);
    field_i32(&mut payload, request.site.y);
    field_i32(&mut payload, request.site.z);
    field_u64(&mut payload, request.hydro_radius as u64);
    Identity::of_fields("voxel-shapers/process-input/v1", &[&payload])
}

fn output_identity(readings: &[ProcessReading]) -> Identity {
    let mut payload = Vec::new();
    field_u64(&mut payload, readings.len() as u64);
    for reading in readings {
        field_str(&mut payload, reading.field.as_str());
        field_str(&mut payload, reading.unit.as_str());
        append_value(&mut payload, reading.value);
    }
    Identity::of_fields("voxel-shapers/process-output/v1", &[&payload])
}

fn state_identity(process: ProcessId, tick: u64) -> Identity {
    let mut payload = Vec::new();
    field_str(&mut payload, process.name());
    field_u64(&mut payload, tick);
    Identity::of_fields("voxel-shapers/process-state/v1", &[&payload])
}

fn replay_identity(spec: Identity, input: Identity, output: Identity, state: Identity) -> Identity {
    Identity::of_fields(
        "voxel-shapers/process-replay/v1",
        &[
            spec.digest().as_bytes(),
            input.digest().as_bytes(),
            output.digest().as_bytes(),
            state.digest().as_bytes(),
        ],
    )
}

fn refusal_condition(kind: RefusalKind) -> RefusalCondition {
    match kind {
        RefusalKind::Unimplemented => RefusalCondition::UnadmittedProductionContract,
        RefusalKind::InvalidFieldSpec => RefusalCondition::InvalidInput,
        RefusalKind::NonFiniteValue | RefusalKind::OutOfDeclaredRange => {
            RefusalCondition::InvalidInput
        }
        RefusalKind::MissingStage => RefusalCondition::MissingDeclaredField,
        _ => RefusalCondition::InvalidInput,
    }
}

// -------------------------------------------------------------------------------------
// Encoding helpers: every field length-prefixed, so two declarations cannot collide
// -------------------------------------------------------------------------------------

fn append_value(payload: &mut Vec<u8>, value: FieldValue) {
    match value {
        FieldValue::Integer(n) => {
            field_u8(payload, 0);
            payload.extend_from_slice(&(n as i64).into_le_bytes());
        }
        FieldValue::Fixed(fixed) => {
            field_u8(payload, 1);
            payload.extend_from_slice(&fixed.raw().into_le_bytes());
            payload.extend_from_slice(&fixed.scale().into_le_bytes());
        }
        FieldValue::Rational(ratio) => {
            field_u8(payload, 2);
            payload.extend_from_slice(&ratio.numerator().into_le_bytes());
            payload.extend_from_slice(&ratio.denominator().into_le_bytes());
        }
    }
}

fn field(payload: &mut Vec<u8>, bytes: &[u8]) {
    payload.extend_from_slice(&(bytes.len() as u64).into_le_bytes());
    payload.extend_from_slice(bytes);
}

fn field_str(payload: &mut Vec<u8>, text: &str) {
    field(payload, text.as_bytes());
}

fn field_u8(payload: &mut Vec<u8>, value: u8) {
    payload.push(value);
}

fn field_u64(payload: &mut Vec<u8>, value: u64) {
    payload.extend_from_slice(&value.into_le_bytes());
}

fn field_i32(payload: &mut Vec<u8>, value: i32) {
    payload.extend_from_slice(&value.into_le_bytes());
}

fn field_list<'a>(payload: &mut Vec<u8>, items: impl Iterator<Item = &'a str>) {
    let items: Vec<&str> = items.collect();
    field_u64(payload, items.len() as u64);
    for item in items {
        field_str(payload, item);
    }
}

fn scope_tag(scope: ProcessScope) -> u8 {
    match scope {
        ProcessScope::RegionalField => 0,
        ProcessScope::GlobalField => 1,
    }
}

fn class_tag(class: SimulationClass) -> u8 {
    match class {
        SimulationClass::Structural => 0,
        SimulationClass::Terrain => 1,
        SimulationClass::Geology => 2,
        SimulationClass::Hydrology => 3,
        SimulationClass::Atmosphere => 4,
        SimulationClass::Climate => 5,
        SimulationClass::Weather => 6,
        SimulationClass::Erosion => 7,
        SimulationClass::Caves => 8,
        SimulationClass::Biology => 9,
        SimulationClass::Materials => 10,
        SimulationClass::Work => 11,
    }
}

fn residency_tag(residency: ResidencyPolicy) -> u8 {
    match residency {
        ResidencyPolicy::Exact => 0,
        ResidencyPolicy::Aggregate => 1,
        ResidencyPolicy::LazyAggregate => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk_fields::{self, FieldQuery, FieldSource};
    use crate::spec::Config;
    use voxel_model::coord::VoxelPos;

    fn world() -> WorldSpec {
        WorldSpec::new(2468, Config::default())
    }

    fn request(tick: u64) -> ProcessRequest<'static> {
        ProcessRequest { site: Site::new(12, 24, -7), tick, hydro_radius: 4, region: &[], unit: None }
    }

    fn region_request(tick: u64) -> ProcessRequest<'static> {
        ProcessRequest {
            site: Site::new(12, 24, -7),
            tick,
            hydro_radius: 4,
            region: &[],
            unit: Some(UnitBox::new([0, 0, 0], [16, 16, 16], 4)),
        }
    }

    fn kind_of(readings: &[ProcessReading], field: FieldId) -> f64 {
        let reading = readings
            .iter()
            .find(|r| r.field == field)
            .unwrap_or_else(|| panic!("no reading for {field:?}"));
        match reading.value {
            FieldValue::Fixed(fixed) => chunk_fields::f64_of_fixed(fixed),
            FieldValue::Rational(ratio) => {
                ratio.numerator() as f64 / ratio.denominator() as f64
            }
            FieldValue::Integer(_) => panic!("{field:?} was an integer"),
        }
    }

    #[test]
    fn the_roster_is_every_stage_and_not_a_smaller_one() {
        let specs = process_specs().expect("the roster validates");
        assert_eq!(specs.len(), STAGE_COUNT, "the roster is not the stage graph");
        assert_eq!(specs.len(), 16, "a stage was dropped from the roster");
        for (spec, stage) in specs.iter().zip(ALL_STAGES) {
            assert_eq!(spec.process.stage(), stage);
        }
        assert_eq!(ALL.len(), STAGE_COUNT);
        for id in ALL {
            assert!(specs.iter().any(|spec| spec.process == id), "{id:?} is not declared");
        }
    }

    #[test]
    fn every_process_runs_on_its_stages_own_rung_and_couplings() {
        let specs = process_specs().expect("the roster validates");
        for spec in &specs {
            let stage = spec.process.stage();
            assert_eq!(spec.cadence, stage.cadence(), "{:?} changed its rung", stage);
            let declared: Vec<ProcessId> =
                stage.depends_on().iter().copied().map(ProcessId::of).collect();
            assert_eq!(spec.dependencies, declared, "{stage:?} changed its couplings");
            spec.validate().expect("each spec validates");
        }
    }

    #[test]
    fn the_ranks_are_the_ladder_and_not_a_second_one() {
        // Each stage's rung has to be *the* ladder's own number: a process that
        // restated it could drift from the scheduler in `voxel-work`.
        let specs = process_specs().expect("the roster validates");
        for spec in &specs {
            let expected = match spec.process.stage() {
                Stage::Earthworks | Stage::Transition | Stage::Substrate => 1,
                Stage::Hydrology | Stage::Weather | Stage::Atmosphere => 4,
                Stage::Erosion | Stage::Delta | Stage::SoilTransport => 20,
                Stage::Geology | Stage::Climate | Stage::Caves => climate::TICKS_PER_DAY,
                Stage::Tectonics | Stage::Fields | Stage::Ecology | Stage::Terrain => 1,
            };
            assert_eq!(spec.cadence.period_ticks(), expected, "{:?}", spec.process);
        }
    }

    #[test]
    fn a_specs_identity_is_stable_and_changes_with_a_declaration() {
        let a = process_specs().expect("valid");
        let b = process_specs().expect("valid");
        for (x, y) in a.iter().zip(&b) {
            assert_eq!(x.identity(), y.identity(), "{} moved between builds", x.process.name());
        }
        let mut changed = a[0].clone();
        changed.dependencies.push(ProcessId::of(Stage::Earthworks));
        assert_ne!(
            changed.identity(),
            a[0].identity(),
            "a changed dependency did not change the spec identity"
        );
        let mut changed = a[0].clone();
        changed.outputs.clear();
        assert_ne!(changed.identity(), a[0].identity());
    }

    #[test]
    fn duplicate_processes_are_refused_by_name() {
        let specs = process_specs().expect("valid");
        let mut twice = specs.clone();
        twice.push(specs[0].clone());
        let refused = ProcessScheduler::new(twice).expect_err("a duplicate roster");
        assert!(refused.detail().contains("declared twice"), "{}", refused.detail());
    }

    #[test]
    fn a_dependency_cycle_is_refused_by_name() {
        let mut a = process_specs().expect("valid")[0].clone();
        let mut b = process_specs().expect("valid")[1].clone();
        a.dependencies = vec![b.process];
        b.dependencies = vec![a.process];
        let refused = ProcessScheduler::new(vec![a, b]).expect_err("a cycle was accepted");
        assert!(refused.detail().contains("cycle"), "{}", refused.detail());
    }

    #[test]
    fn a_spec_that_depends_on_itself_is_refused() {
        let mut spec = process_specs().expect("valid")[0].clone();
        spec.dependencies = vec![spec.process];
        let refused = spec.validate().expect_err("self-dependency was accepted");
        assert!(refused.detail().contains("itself"), "{}", refused.detail());
    }

    #[test]
    fn due_order_respects_dependencies_and_is_deterministic() {
        let scheduler = ProcessScheduler::world_defaults().expect("the roster builds");
        let at_zero = scheduler.due(0);
        assert_eq!(at_zero.len(), STAGE_COUNT, "tick 0 is due for every process");
        let position = |stage: Stage| {
            at_zero
                .iter()
                .position(|id| id.stage() == stage)
                .unwrap_or_else(|| panic!("{stage:?} missing from the tick-0 plan"))
        };
        assert!(position(Stage::Tectonics) < position(Stage::Terrain));
        assert!(position(Stage::Terrain) < position(Stage::Hydrology));
        assert!(position(Stage::Climate) < position(Stage::Weather));
        assert!(position(Stage::Substrate) < position(Stage::SoilTransport));
        assert_eq!(at_zero, scheduler.due(0), "the plan moved between two asks");
        // The ×4 rung is absent on tick 1 and present on tick 4; the daily pair
        // waits for the day.
        let at_one = scheduler.due(1);
        assert!(!at_one.contains(&ProcessId::of(Stage::Hydrology)));
        assert!(scheduler.due(4).contains(&ProcessId::of(Stage::Hydrology)));
        assert!(!scheduler.due(20).contains(&ProcessId::of(Stage::Climate)));
        assert!(scheduler
            .due(climate::TICKS_PER_DAY)
            .contains(&ProcessId::of(Stage::Climate)));
    }

    #[test]
    fn an_older_tick_is_refused_and_the_same_tick_is_idempotent() {
        let world = world();
        let mut scheduler = ProcessScheduler::world_defaults().expect("the roster builds");
        scheduler.step(&world, &request(10)).expect("tick 10");
        assert_eq!(scheduler.last_tick(), Some(10));
        let again = scheduler.step(&world, &request(10)).expect("tick 10 again");
        assert!(again.is_empty(), "the same tick ran twice");
        let refused = scheduler.step(&world, &request(9)).expect_err("an older tick");
        assert_eq!(refused.kind(), RefusalKind::OutOfDeclaredRange);
    }

    /// The body's own answer, called directly. This is the interface the scheduler
    /// drives; while the full roster still fails closed at `Fields`, it is the honest
    /// way to test the eight admitted bodies.
    fn body_readings(
        stage: Stage,
        world: &WorldSpec,
        request: &ProcessRequest,
    ) -> Vec<ProcessReading> {
        let spec = spec_for_stage(stage).expect("the spec builds");
        execute_body(&spec, world, request)
            .unwrap_or_else(|refusal| panic!("{stage:?} refused: {}", refusal.detail()))
    }

    #[test]
    fn the_processes_with_bodies_produce_the_shapers_own_values() {
        let world = world();
        let req = request(0);
        let site = req.site;

        // Terrain: exactly `terrain::sample`.
        let terrain = terrain::sample(&world, site.fx(), site.fz());
        assert!((kind_of(&body_readings(Stage::Terrain, &world, &req), FieldId::Elevation) - terrain.elevation_m).abs() < 1e-9);
        assert!((kind_of(&body_readings(Stage::Terrain, &world, &req), FieldId::Slope) - terrain.slope).abs() < 1e-9);

        // Climate: the season is the climate's own phase, not a counter.
        let climate = climate::sample(&world, site.fx(), site.fz(), req.tick);
        assert!((kind_of(&body_readings(Stage::Climate, &world, &req), FieldId::Climate) - climate::season_phase(req.tick)).abs() < 1e-9);
        assert!((kind_of(&body_readings(Stage::Climate, &world, &req), FieldId::Temperature) - climate.temperature_c).abs() < 1e-9);
        assert!((kind_of(&body_readings(Stage::Climate, &world, &req), FieldId::Moisture) - climate.moisture).abs() < 1e-9);

        // Weather: precipitation in metres, bearing carried as itself.
        let weather = weather::sample(&world, site.fx(), site.fz(), req.tick);
        assert!((kind_of(&body_readings(Stage::Weather, &world, &req), FieldId::Precipitation) - weather.precipitation_mm / 1000.0).abs() < 1e-9);
        assert!((kind_of(&body_readings(Stage::Weather, &world, &req), FieldId::Wind) - weather.wind_bearing_rad).abs() < 1e-9);

        // Hydrology: the shaper's own depth at the declared radius.
        let hydro = hydrology::sample(&world, site.fx(), site.fz(), req.hydro_radius);
        assert!((kind_of(&body_readings(Stage::Hydrology, &world, &req), FieldId::Groundwater) - hydro.water_depth_m).abs() < 1e-9);

        // Atmosphere: kPa to Pa.
        let air = atmosphere::sample(&world, site.fy());
        assert!((kind_of(&body_readings(Stage::Atmosphere, &world, &req), FieldId::Pressure) - air.pressure_kpa * 1000.0).abs() < 1e-6);

        // Substrate: the stratum at the site's own depth, as the field plane reads it.
        let surface = terrain::elevation_m(&world, site.fx(), site.fz());
        let stratum = geology::stratum_at(&world, site.fx(), site.fz(), surface, site.fy());
        let slope = terrain::sample(&world, site.fx(), site.fz()).slope;
        let moisture = climate::moisture(&world, site.fx(), site.fz());
        let depth = substrate::soil_depth_m(&world, stratum.lithology, slope, moisture);
        assert!((kind_of(&body_readings(Stage::Substrate, &world, &req), FieldId::Soil) - depth).abs() < 1e-9);

        // Geology: the lithology code of the stratum at the site's own height.
        let here = geology::stratum_at(&world, site.fx(), site.fz(), surface, site.fy());
        let readings = body_readings(Stage::Geology, &world, &req);
        let code = match readings
            .iter()
            .find(|r| r.field == FieldId::Geology)
            .expect("a geology reading")
            .value
        {
            FieldValue::Integer(n) => n,
            other => panic!("Geology is a class code and must be an integer, got {other:?}"),
        };
        assert_eq!(code, chunk_fields::lithology_code(here.lithology));
    }

    #[test]
    fn the_delta_body_is_the_shapers_own_law_over_the_substrate_baseline() {
        // Delta's admitted law reads the standing water and the slope, and its reading
        // is the substrate baseline plus whatever the mouth deposits this tick. The
        // expected value is computed here from the shaper's own functions rather than
        // copied from the body, so the two cannot agree by both being wrong.
        let world = world();
        let req = request(0);
        let site = req.site;
        let surface = terrain::elevation_m(&world, site.fx(), site.fz());
        let slope = terrain::sample(&world, site.fx(), site.fz()).slope;
        let water = hydrology::sample(&world, site.fx(), site.fz(), req.hydro_radius).water_depth_m;
        let stratum = geology::stratum_at(&world, site.fx(), site.fz(), surface, site.fy());
        let moisture = climate::moisture(&world, site.fx(), site.fz());
        let baseline = substrate::soil_depth_m(&world, stratum.lithology, slope, moisture);
        let deposited = delta::deposition_m(
            delta::transport_capacity(delta::velocity_m_s(slope, water)),
            water,
            slope,
        );
        let soil = kind_of(&body_readings(Stage::Delta, &world, &req), FieldId::Soil);
        assert!(
            (soil - (baseline + deposited)).abs() < 1e-9,
            "delta read {soil}, expected {}",
            baseline + deposited
        );
        // And with no standing water the mouth deposits exactly nothing, so the
        // reading is the baseline and not a small number that would silt a pool.
        assert_eq!(delta::deposition_m(1.0, 0.0, slope), 0.0);
    }

    #[test]
    fn a_transition_without_a_region_refuses_by_name_instead_of_guessing() {
        // The defect the owed body named: a blend law over neighbours cannot be
        // evaluated from one site. A request with an empty region must refuse, not
        // pick a side.
        let world = world();
        let req = request(0);
        let refused = execute_body(&spec_for_stage(Stage::Transition).unwrap(), &world, &req)
            .expect_err("a transition answered without a region");
        assert_eq!(refused.kind(), RefusalKind::MissingStage);
        assert!(refused.detail().contains("region"), "{}", refused.detail());
    }

    #[test]
    fn a_transition_over_a_region_answers_the_nearest_domain() {
        // With one neighbour the weight is one, so the answer is that neighbour's own
        // stratum — the shaper's law, read through the region form.
        let world = world();
        let mine = Site::new(12, 24, -7);
        let neighbour = Site::new(20, 24, -7);
        let region = [RegionSample { site: neighbour, distance_m: 1.0 }];
        let req =
            ProcessRequest { site: mine, tick: 0, hydro_radius: 4, region: &region, unit: None };
        let expected = chunk_fields::lithology_code(lithology_at(&world, neighbour));
        let readings = body_readings(Stage::Transition, &world, &req);
        let got = match readings
            .iter()
            .find(|r| r.field == FieldId::Geology)
            .expect("a geology reading")
            .value
        {
            FieldValue::Integer(n) => n,
            other => panic!("Geology is a class code and must be an integer, got {other:?}"),
        };
        assert_eq!(got, expected);
    }

    #[test]
    fn the_structural_bodies_refuse_a_site_request_by_name() {
        // The debt `owed_body` named for `Fields` and `Erosion`: a law about an area
        // cannot be evaluated at a point. A site request must refuse, not guess.
        let world = world();
        let req = request(0); // no unit
        for stage in [Stage::Fields, Stage::Erosion] {
            let refused = execute_body(&spec_for_stage(stage).unwrap(), &world, &req)
                .expect_err("a region stage answered without a region");
            assert_eq!(refused.kind(), RefusalKind::MissingStage, "{stage:?}");
            assert!(refused.detail().contains("region"), "{stage:?}: {}", refused.detail());
        }
    }

    #[test]
    fn the_region_bodies_check_the_ladder_and_apply_the_stream_power_law() {
        let world = world();
        let req = region_request(0);

        // Fields: a legal unit produces nothing — the ladder carries no per-site
        // scalar, and the body declares no output, so an empty reading is the answer.
        let fields = body_readings(Stage::Fields, &world, &req);
        assert!(fields.is_empty(), "fields produced a reading it does not declare");

        // Erosion: the shaper's own stream-power law over the unit's own area, with the
        // expected value recomputed here rather than copied from the body.
        let unit = req.unit.expect("the region request carries a unit");
        let site = unit.centre_site();
        let slope = terrain::sample(&world, site.fx(), site.fz()).slope;
        let water = hydrology::sample(&world, site.fx(), site.fz(), req.hydro_radius).water_depth_m;
        let incision = erosion::stream_power_rate(unit.area_m2() * (1.0 + water), slope);
        let expected = 1.0 / (1.0 + incision);
        let got = kind_of(
            &body_readings(Stage::Erosion, &world, &req),
            FieldId::ErosionResistance,
        );
        assert!((got - expected).abs() < 1e-12, "erosion read {got}, expected {expected}");

        // A unit whose level is outside the ladder is refused by name.
        let mut bad = req;
        bad.unit = Some(UnitBox::new([0, 0, 0], [16, 16, 16], crate::fields::STANDARD_LEVELS));
        let refused = execute_body(&spec_for_stage(Stage::Fields).unwrap(), &world, &bad)
            .expect_err("an out-of-ladder level was accepted");
        assert_eq!(refused.kind(), RefusalKind::IndexOutOfRange);
    }

    #[test]
    fn a_roster_that_opens_mid_day_runs_after_a_single_tick_zero() {
        // The client's clock opens at `GAME_TICK` (`climate::TICKS_PER_DAY * 3 / 8`), and a
        // **fresh** roster refuses a stage whose dependency has never run at a nonzero tick
        // (`dependency_refusal`'s `tick != 0` clause). Stepping tick 0 once is what clears
        // that, and this pins it: the second step — the live mid-day tick — must raise no
        // `DependencyUnavailable` of its own.
        let world = world();
        let mut scheduler = ProcessScheduler::world_defaults().expect("the roster builds");
        scheduler.step(&world, &region_request(0)).expect("tick 0");
        let mid_day = climate::TICKS_PER_DAY * 3 / 8;
        assert_ne!(mid_day, 0, "the client's opening tick is not tick 0");
        let evidence = scheduler.step(&world, &region_request(mid_day)).expect("the mid-day tick");
        let spurious: Vec<&str> = evidence
            .iter()
            .filter(|e| e.refusal == Some(RefusalCondition::DependencyUnavailable))
            .map(|e| e.process.stage().name())
            .collect();
        assert!(
            spurious.is_empty(),
            "a roster opened at tick 0 still refused on a ran dependency: {spurious:?}"
        );
    }

    #[test]
    fn a_unit_request_unblocks_the_structural_region_and_nothing_that_reads_it_refuses_on_it() {
        // A request carrying a unit runs `Fields` and `Erosion`, which runs everything
        // that reads them. The two stages whose bodies want the **region** rather than the
        // unit then refuse their own region debt (not a dependency), and the two stages
        // with no body quote their own debt — so `Ecology`, which reads the structural
        // chain, is finally running and `Transition` is no longer blocked on it.
        let world = world();
        let mut scheduler = ProcessScheduler::world_defaults().expect("the roster builds");
        let evidence = scheduler.step(&world, &region_request(0)).expect("the first step");
        let find = |stage: Stage| {
            evidence
                .iter()
                .find(|e| e.process.stage() == stage)
                .unwrap_or_else(|| panic!("{stage:?} was not in the evidence"))
        };
        let ran = evidence.iter().filter(|e| e.ran()).count();
        assert_eq!(ran, 12, "a unit request ran {ran} stages, not twelve");
        assert!(find(Stage::Fields).ran(), "fields refused a unit request");
        assert!(find(Stage::Erosion).ran(), "erosion refused a unit request");
        assert!(find(Stage::Ecology).ran(), "ecology refused: {:?}", find(Stage::Ecology).detail);
        for stage in [Stage::SoilTransport, Stage::Transition] {
            assert_eq!(
                find(stage).refusal,
                Some(RefusalCondition::MissingDeclaredField),
                "{stage:?} should quote its own region debt: {:?}",
                find(stage).detail
            );
            assert!(find(stage).detail.as_deref().unwrap_or("").contains("region"));
        }
        for stage in [Stage::Caves, Stage::Earthworks] {
            assert_eq!(
                find(stage).refusal,
                Some(RefusalCondition::UnadmittedProductionContract),
                "{stage:?} reads a ran dependency and should quote its own debt"
            );
        }
    }

    #[test]
    fn a_paired_hillslope_flux_is_antisymmetric_and_vanishes_on_a_flat_bed() {
        // The operator's two load-bearing properties: a flat pair moves nothing, and one
        // pair computes equal and opposite changes at its two ends, which is the only
        // reason the move can claim to conserve material.
        assert_eq!(soil_transport_delta_m(10.0, &[(10.0, 1.0)]), Some(0.0));
        // A 0.1 m drop over 1 m is well inside the critical slope of 0.8.
        let downhill = soil_transport_delta_m(10.0, &[(9.9, 1.0)]).expect("a gentle slope");
        assert!(downhill < 0.0, "a downhill neighbour did not take soil: {downhill}");
        let uphill = soil_transport_delta_m(9.9, &[(10.0, 1.0)]).expect("a gentle slope");
        assert!(uphill > 0.0, "an uphill neighbour did not deliver soil: {uphill}");
        assert!(
            (downhill + uphill).abs() < 1e-12,
            "the pair did not conserve: {downhill} + {uphill}"
        );
        // A slope at or beyond the critical slope diverges, so it is a refusal and not a
        // very large number; a coincident neighbour has no gradient to read.
        assert_eq!(soil_transport_delta_m(100.0, &[(0.0, 1.0)]), None);
        assert_eq!(soil_transport_delta_m(10.0, &[(11.0, 0.0)]), None);
    }

    #[test]
    fn soil_transport_without_a_region_refuses_by_name() {
        let world = world();
        let refused = execute_body(
            &spec_for_stage(Stage::SoilTransport).unwrap(),
            &world,
            &request(0),
        )
        .expect_err("soil transport answered without a region");
        assert_eq!(refused.kind(), RefusalKind::MissingStage);
        assert!(refused.detail().contains("region"), "{}", refused.detail());
    }

    #[test]
    fn soil_transport_moves_soil_off_the_substrate_baseline() {
        let world = world();
        let site = Site::new(12, 24, -7);
        // Far neighbours so the pairwise slopes stay well inside the critical slope and
        // the body is exercised on its ordinary path rather than its refusal.
        let region = [
            RegionSample { site: Site::new(13, 24, -7), distance_m: 1_000.0 },
            RegionSample { site: Site::new(11, 24, -6), distance_m: 1_000.0 },
        ];
        let req =
            ProcessRequest { site, tick: 0, hydro_radius: 4, region: &region, unit: None };

        let surface = terrain::elevation_m(&world, site.fx(), site.fz());
        let slope = terrain::sample(&world, site.fx(), site.fz()).slope;
        let stratum = geology::stratum_at(&world, site.fx(), site.fz(), surface, site.fy());
        let moisture = climate::moisture(&world, site.fx(), site.fz());
        let baseline = substrate::soil_depth_m(&world, stratum.lithology, slope, moisture);
        let pairs: Vec<(f64, f64)> = region
            .iter()
            .map(|n| (terrain::elevation_m(&world, n.site.fx(), n.site.fz()), n.distance_m))
            .collect();
        let expected = baseline + soil_transport_delta_m(surface, &pairs).expect("gentle pairs");
        let got = kind_of(&body_readings(Stage::SoilTransport, &world, &req), FieldId::Soil);
        assert!((got - expected).abs() < 1e-12, "soil transport read {got}, expected {expected}");
    }

    #[test]
    fn ecology_reads_the_two_capacities_from_the_shapers_own_numbers() {
        // The expected values are recomputed here from the ecology laws and the same
        // substrate/climate inputs the body reads, so the body and this test cannot agree
        // by both being wrong.
        let world = world();
        let req = request(0);
        let site = req.site;
        let surface = terrain::elevation_m(&world, site.fx(), site.fz());
        let (biotemperature, precipitation) =
            ecology::holdridge_inputs(&world, site.fx(), site.fz(), surface, req.tick);
        let slope = terrain::sample(&world, site.fx(), site.fz()).slope;
        let stratum = geology::stratum_at(&world, site.fx(), site.fz(), surface, site.fy());
        let moisture = climate::moisture(&world, site.fx(), site.fz());
        let soil = substrate::soil_depth_m(&world, stratum.lithology, slope, moisture);

        let readings = body_readings(Stage::Ecology, &world, &req);
        let expected_capacity =
            ecology::carrying_capacity(biotemperature, precipitation, soil).expect("finite");
        let expected_suitability =
            ecology::species_suitability(biotemperature, precipitation, soil).expect("finite");
        let capacity = kind_of(&readings, FieldId::CarryingCapacity);
        let suitability = kind_of(&readings, FieldId::SpeciesSuitability);
        assert!((capacity - expected_capacity).abs() < 1e-12, "capacity {capacity}");
        assert!(
            (suitability - expected_suitability).abs() < 1e-12,
            "suitability {suitability}"
        );
        assert!((0.0..=1.0).contains(&capacity), "capacity out of range: {capacity}");
        assert!((0.0..=1.0).contains(&suitability), "suitability out of range: {suitability}");
    }

    #[test]
    fn the_terrain_and_climate_readings_are_the_field_planes_own_values_where_it_declares_them() {
        // The process layer and the field plane must be one world: where the plane
        // declares a field this roster also produces, the values agree. The bodies are
        // called directly because the full roster cannot step past the unadmitted
        // `Fields` yet; the agreement being pinned is body-versus-plane, not
        // scheduler-versus-plane.
        let world = world();
        let req = request(0);
        let plane = FieldSource::new(&world).expect("the plane builds");
        let coord = VoxelPos::new(req.site.x, req.site.y, req.site.z);

        let compared = [
            (Stage::Terrain, FieldId::Elevation),
            (Stage::Terrain, FieldId::Slope),
            (Stage::Climate, FieldId::Moisture),
            (Stage::Substrate, FieldId::Soil),
            (Stage::Geology, FieldId::Geology),
        ];
        let mut compared_count = 0;
        for (stage, field) in compared {
            let spec = spec_for_stage(stage).expect("the spec builds");
            let readings = execute_body(&spec, &world, &req).expect("the body runs");
            let through_process = readings
                .iter()
                .find(|r| r.field == field)
                .unwrap_or_else(|| panic!("{stage:?} did not produce {field:?}"))
                .value;
            let through_plane = FieldQuery::sample(&plane, field, coord).expect("the plane answers");
            if let (FieldValue::Fixed(a), FieldValue::Fixed(b)) = (through_process, through_plane) {
                assert!(
                    (chunk_fields::f64_of_fixed(a) - chunk_fields::f64_of_fixed(b)).abs() < 1e-9,
                    "{stage:?}/{field:?}: process and plane disagree"
                );
            } else {
                assert_eq!(through_process, through_plane, "{stage:?}/{field:?}");
            }
            compared_count += 1;
        }
        assert_eq!(compared_count, 5);
    }

    #[test]
    fn a_process_with_no_body_refuses_by_name_and_says_what_is_owed() {
        // The roster is every stage from the first tick (the design rule: a process may
        // not be silently activated only when its implementation lands). In the full
        // roster the first owed stage in dependency order reports its own debt and the
        // stages that read it refuse on the unavailable dependency.
        let world = world();
        // `Fields` and `Erosion` now have **region** bodies: a site request (no unit)
        // refuses them by name rather than admitting a law about an area at a point.
        let owed = [Stage::Caves, Stage::Earthworks];
        let mut scheduler = ProcessScheduler::world_defaults().expect("the roster builds");
        let evidence = scheduler.step(&world, &request(0)).expect("the first step");
        assert_eq!(evidence.len(), STAGE_COUNT, "a stage was silently skipped");
        let fields = evidence
            .iter()
            .find(|e| e.process.stage() == Stage::Fields)
            .expect("fields was planned");
        assert_eq!(fields.refusal, Some(RefusalCondition::MissingDeclaredField));
        assert!(
            fields.detail.as_deref().unwrap_or("").contains("region"),
            "fields' region debt is not named: {:?}",
            fields.detail
        );
        for stage in owed.iter().copied() {
            let e = evidence
                .iter()
                .find(|e| e.process.stage() == stage)
                .unwrap_or_else(|| panic!("{stage:?} was not in the evidence"));
            assert!(!e.ran(), "{stage:?} claimed to run with no body");
            // Both read a stage that is itself blocked on the (unit-less) `Fields`, so
            // the refusal they report is the unavailable dependency rather than their
            // own body — the ordering the scheduler is meant to express.
            assert_eq!(
                e.refusal,
                Some(RefusalCondition::DependencyUnavailable),
                "{stage:?} should refuse on the unavailable dependency"
            );
            assert!(e.detail.is_some(), "{stage:?} refused without a reason");
        }
        // And every owed stage's own debt is named when the stage is reached: a
        // one-process roster with the couplings cleared exercises exactly the body
        // admission path, where `UnadmittedProductionContract` is the refusal.
        for stage in owed {
            let mut spec = spec_for_stage(stage).expect("the spec builds");
            spec.dependencies.clear();
            let mut scheduler = ProcessScheduler::new(vec![spec]).expect("the roster builds");
            let evidence = scheduler.step(&world, &request(0)).expect("the first step");
            let e = evidence
                .iter()
                .find(|e| e.process.stage() == stage)
                .unwrap_or_else(|| panic!("{stage:?} was not in the evidence"));
            assert_eq!(
                e.refusal,
                Some(RefusalCondition::UnadmittedProductionContract),
                "{stage:?}"
            );
            let detail = e.detail.as_deref().unwrap_or("");
            assert!(
                detail.contains(stage.name()),
                "{stage:?}'s refusal does not name it: {detail}"
            );
            assert!(
                detail.contains(owed_body(stage)),
                "{stage:?}'s refusal does not quote its debt: {detail}"
            );
        }
    }

    #[test]
    fn a_process_whose_dependency_refused_refuses_without_running() {
        // `SoilTransport` reads `Substrate`, which has a body; but its dependency row
        // also names `Erosion`, which does not. The scheduler must refuse it on the
        // dependency, and the refusal must be the scheduler's, not a body's.
        let world = world();
        let mut scheduler = ProcessScheduler::world_defaults().expect("the roster builds");
        let evidence = scheduler.step(&world, &request(0)).expect("the first step");
        let transport = evidence
            .iter()
            .find(|e| e.process.stage() == Stage::SoilTransport)
            .expect("soil transport was planned");
        assert_eq!(transport.refusal, Some(RefusalCondition::DependencyUnavailable));
        assert!(transport.detail.as_deref().unwrap_or("").contains("dependency"));
        // And the state records it, so a later tick can see the refusal.
        assert_eq!(
            scheduler
                .state(ProcessId::of(Stage::SoilTransport))
                .expect("state")
                .last_refusal(),
            Some(RefusalCondition::DependencyUnavailable)
        );
    }

    #[test]
    fn a_body_that_produces_an_undeclared_field_is_refused() {
        // Teeth for the outputs table: take a spec with a body and empty its outputs.
        // The body would read fine; the *declaration* refuses the reading, which is
        // what keeps the table honest.
        let mut spec = process_specs().expect("valid")[0].clone();
        assert_eq!(spec.process.stage(), Stage::Tectonics);
        spec.outputs.clear();
        let world = world();
        let request = request(0);
        let refused =
            execute_body(&spec, &world, &request).expect_err("an undeclared output was accepted");
        assert!(refused.detail().contains("output"), "{}", refused.detail());
    }

    #[test]
    fn the_evidence_carries_four_distinct_identities_and_a_bound_replay() {
        // The full roster refuses at `Fields` until its body is admitted, so this
        // evidence test runs a roster closed over admitted bodies: `Terrain`'s stage
        // row also names `Fields` and `Geology`, which are not part of what is being
        // pinned here (its body reads the shapers directly).
        let world = world();
        let mut terrain = spec_for_stage(Stage::Terrain).expect("the spec builds");
        terrain.dependencies = vec![ProcessId::of(Stage::Tectonics)];
        let roster = vec![
            spec_for_stage(Stage::Tectonics).expect("the spec builds"),
            terrain,
        ];
        let mut scheduler = ProcessScheduler::new(roster.clone()).expect("the roster builds");
        let evidence = scheduler.step(&world, &request(0)).expect("the first step");
        let terrain = evidence
            .iter()
            .find(|e| e.process.stage() == Stage::Terrain && e.ran())
            .expect("terrain ran");
        assert_ne!(terrain.spec, terrain.input);
        assert_ne!(terrain.input, terrain.output);
        assert_ne!(terrain.output, terrain.state);
        let expected = replay_identity(terrain.spec, terrain.input, terrain.output, terrain.state);
        assert_eq!(terrain.replay, expected, "the replay identity is not the four bound");
        // The same request through a fresh scheduler is the same evidence.
        let mut second = ProcessScheduler::new(roster).expect("the roster builds");
        let again = second.step(&world, &request(0)).expect("the first step");
        assert_eq!(evidence, again, "two identical runs produced different evidence");
    }

    #[test]
    fn the_runtime_changes_nothing_about_the_world() {
        // The conservation claim, over the whole roster: running every due process
        // twice leaves the world spec bit-identical — these bodies read.
        let world = world();
        let before = world;
        let mut scheduler = ProcessScheduler::world_defaults().expect("the roster builds");
        scheduler.step(&world, &request(0)).expect("tick 0");
        scheduler.step(&world, &request(4)).expect("tick 4");
        assert_eq!(before, world, "a process mutated the world spec");
        assert_eq!(
            scheduler.specs().len(),
            STAGE_COUNT,
            "the roster changed under the runs"
        );
    }

    #[test]
    fn the_evidence_names_are_the_rosters_and_the_states_are_per_process() {
        let world = world();
        let mut scheduler = ProcessScheduler::world_defaults().expect("the roster builds");
        let evidence = scheduler.step(&world, &request(0)).expect("the first step");
        assert_eq!(evidence.len(), STAGE_COUNT);
        let mut names: Vec<&str> = evidence.iter().map(|e| e.process.name()).collect();
        let before = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), before, "a process appeared twice in one step");
        let runs = scheduler
            .state(ProcessId::of(Stage::Terrain))
            .expect("state")
            .run_count();
        assert_eq!(runs, 1);
        assert!(scheduler.state(ProcessId::of(Stage::Terrain)).expect("state").last_run() == Some(0));
    }
}
