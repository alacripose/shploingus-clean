//! **The declared stage graph — a stage names its source, its inputs, its output, and its rung.**
//!
//! Derived from `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §2 U3 (*“`Declared` (the world's declared stage
//! set), `Stage`, a shaper's `acts_at()`-required contract, the `ProcessScheduler` driven by a
//! cadence `TickDriver`, and the field plane's `field_at` seam”*) and `docs/SHAPERS.md` §2 (the
//! determinism contract) and §1 (pipeline order is a **stage graph** (F9), not a call chain: *a
//! shaper reads only the stages above it and writes its own declared stage*). The contract is
//! `specs/003-u3-shapers/contracts/u3-seams.md` C1.
//!
//! # Why the declaration is checked rather than asserted
//!
//! Three rules in this file are the whole reason U3's roster is worth having:
//!
//! 1. **A stage that names no paper is refused by name.** `docs/SHAPERS.md` opens with the rule that
//!    no shaper exists as *“an unexplained pile of noise”*; a source is therefore part of the type,
//!    not a comment (Constitution P1).
//! 2. **A stage that reads a stage declared below it is refused by name.** The graph is the contract;
//!    if it were only a convention, the order of effects would live in whoever called it.
//! 3. **A stage's declaration binds at run time.** A body that returns a field it did not declare,
//!    or omits one it did, is refused by the scheduler — a declared output that quietly produces
//!    nothing is the failure mode this system exists to remove.
//!
//! Refusals are the U3 kinds in [`crate::refusal`], and the reason this file's validation collects
//! **all** of them (rather than returning the first) is P2: an author fixes one round of problems.

use std::collections::BTreeMap;

use crate::coordinates::VoxelPos;
use crate::digest::Digest;
use crate::noise::Noise;
use crate::refusal::{Refusal, RefusalKind, RefusalSet};
use crate::world::{mix, Tick, WorldIdentity};

/// The **closed** cadence ladder. Four rungs, and nothing between them: a stage's schedule is a
/// declared rung, never a duration and never a wall-clock interval.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Cadence {
    /// Every tick — the world's own clock.
    PerTick,
    /// The climate rung (slower than the tick).
    Climate,
    /// The sediment rung (slower still).
    Sediment,
    /// The detail rung.
    Detail,
}

impl Cadence {
    pub const ALL: [Cadence; 4] = [Cadence::PerTick, Cadence::Climate, Cadence::Sediment, Cadence::Detail];

    pub const COUNT: usize = 4;

    /// The rung as declared in a document: `0..4`.
    pub const fn rung(self) -> u8 {
        match self {
            Cadence::PerTick => 0,
            Cadence::Climate => 1,
            Cadence::Sediment => 2,
            Cadence::Detail => 3,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Cadence::PerTick => "per_tick",
            Cadence::Climate => "climate",
            Cadence::Sediment => "sediment",
            Cadence::Detail => "detail",
        }
    }

    /// Ticks between runs of a rung. Declared here, once, so two readers of the ladder cannot
    /// disagree about what "the climate rung" means.
    pub const fn ticks_between(self) -> u64 {
        match self {
            Cadence::PerTick => 1,
            Cadence::Climate => 4,
            Cadence::Sediment => 20,
            Cadence::Detail => 64,
        }
    }

    /// A rung from a declared number. A rung **off the ladder** is refused by name here rather than
    /// silently becoming the nearest one.
    pub fn from_rung(rung: u8) -> Result<Self, Refusal> {
        match rung {
            0 => Ok(Cadence::PerTick),
            1 => Ok(Cadence::Climate),
            2 => Ok(Cadence::Sediment),
            3 => Ok(Cadence::Detail),
            other => Err(Refusal::new(
                RefusalKind::CadenceOffLadder,
                format!("rung {other} is not one of the {} declared rungs", Cadence::COUNT),
            )),
        }
    }
}

/// The **closed** set of fields a stage may produce. A field that is not declared here cannot be
/// written, which is what keeps "which stage makes this value" a question with one answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FieldId {
    /// Metres of tectonic uplift, from `tectonics` (Kreischer & Bommes 2019).
    Uplift,
    /// The plate a column belongs to, from `tectonics`.
    PlateId,
    /// The regional lithology class, from `fields` (Shepard 1968 + the F4 ladder ruling).
    Lithology,
    /// The exposed stratigraphic layer index, from `geology` (Allen & Allen 2013).
    Stratigraphy,
    /// Elevation in metres, from `terrain` (Mandelbrot 1982; Musgrave, Kolb & Mace 1989).
    Elevation,
    /// Channel discharge, from `hydrology` (O'Callaghan & Mark 1984; Barnes et al. 2014).
    Discharge,
    /// Strahler stream order, from `hydrology` (Strahler 1952): `1` at a headwater, `+1` where two
    /// streams of equal order meet.
    StreamOrder,
    /// The **filled** water surface in metres, from `hydrology` (Barnes, Lehman & Mulla 2014,
    /// Priority-Flood): equal to elevation where the column is dry, and the spill level where a
    /// depression holds water.
    WaterLevel,
    /// Soil depth in metres, from `substrate` (Heimsath et al. 1997).
    SoilDepth,
    /// Erosion rate, from `erosion` (Whipple & Tucker 1999; Lague 2014).
    ErosionRate,
    /// The **divergence** of nonlinear hillslope transport, from `erosion` (Roering, Kirchner &
    /// Dietrich 1999): positive where the column sheds material, and exactly zero over a closed
    /// region.
    HillslopeFlux,
    /// Air temperature in °C, from `climate` (ICAO/US Standard Atmosphere 1976 lapse rate).
    Temperature,
    /// Surface pressure in pascals, from `atmosphere` (US Standard Atmosphere 1976).
    Pressure,
    /// Precipitation, from `weather` (Smith & Barstad 2004).
    Precipitation,
    /// The biome class of a column, from `ecology` (Holdridge 1947).
    Biome,
    /// Lieth's Miami net primary productivity in g DM m⁻² yr⁻¹, limited by the soil depth, from
    /// `ecology` (Lieth 1975).
    CarryingCapacity,
    /// The boids resultant magnitude of the column's agent in `[0, 1]`, from `ecology`
    /// (Reynolds 1987). Zero where no formation can carry an agent.
    Flock,
    /// Cave occupancy — **owed**: the dissolution field it needs is not one of the declared fields,
    /// so `caves` refuses by name rather than thresholding the flow and calling it a cave.
    CaveOccupancy,
    /// Delta deposition in metres, from `delta` (Gilbert 1885; Hjulström 1935 / Shields 1936): the
    /// load a channel drops where it stops carrying, capped by the accommodation standing water
    /// leaves. **Avulsion** is still owed with its state.
    Deposition,
    /// The canonical domain's share of a column in `[0, 1]`, from `transition` (Shepard 1968): a law
    /// about a *relation between columns*, so a lone site is refused by name.
    Blend,
    /// A recorded modification — `earthworks` is the record, not a generator: it refuses by name.
    RecordedMass,
}

impl FieldId {
    pub const ALL: [FieldId; 21] = [
        FieldId::Uplift,
        FieldId::PlateId,
        FieldId::Lithology,
        FieldId::Stratigraphy,
        FieldId::Elevation,
        FieldId::Discharge,
        FieldId::StreamOrder,
        FieldId::WaterLevel,
        FieldId::SoilDepth,
        FieldId::ErosionRate,
        FieldId::HillslopeFlux,
        FieldId::Temperature,
        FieldId::Pressure,
        FieldId::Precipitation,
        FieldId::Biome,
        FieldId::CarryingCapacity,
        FieldId::Flock,
        FieldId::CaveOccupancy,
        FieldId::Deposition,
        FieldId::Blend,
        FieldId::RecordedMass,
    ];

    pub const COUNT: usize = 21;

    pub const fn name(self) -> &'static str {
        match self {
            FieldId::Uplift => "uplift",
            FieldId::PlateId => "plate_id",
            FieldId::Lithology => "lithology",
            FieldId::Stratigraphy => "stratigraphy",
            FieldId::Elevation => "elevation",
            FieldId::Discharge => "discharge",
            FieldId::StreamOrder => "stream_order",
            FieldId::WaterLevel => "water_level",
            FieldId::SoilDepth => "soil_depth",
            FieldId::ErosionRate => "erosion_rate",
            FieldId::HillslopeFlux => "hillslope_flux",
            FieldId::Temperature => "temperature",
            FieldId::Pressure => "pressure",
            FieldId::Precipitation => "precipitation",
            FieldId::Biome => "biome",
            FieldId::CarryingCapacity => "carrying_capacity",
            FieldId::Flock => "flock",
            FieldId::CaveOccupancy => "cave_occupancy",
            FieldId::Deposition => "deposition",
            FieldId::Blend => "blend",
            FieldId::RecordedMass => "recorded_mass",
        }
    }
}

/// A law's **declared dials**: the coefficients, exponents and thresholds it runs with, and — just
/// as importantly — the ones it needs and the declaration does **not** carry.
///
/// There is no invented default (research.md D6): a dial changes what the world looks like, and the
/// world is the user's. A gap is therefore written down (`with_gaps`) rather than left out, because a
/// silent omission is exactly the kind of absence that looks like a decision later. Both are refused
/// by name: a gap at admission, and any dial a body asks for that the table does not carry.
#[derive(Clone, Copy, Debug)]
pub struct Dials {
    declared: &'static [(&'static str, f64)],
    gaps: &'static [&'static str],
}

impl Dials {
    pub const fn new(declared: &'static [(&'static str, f64)]) -> Self {
        Dials { declared, gaps: &[] }
    }

    /// A declaration with **named gaps**: dials the law needs and this declaration does not fix.
    pub const fn with_gaps(
        declared: &'static [(&'static str, f64)],
        gaps: &'static [&'static str],
    ) -> Self {
        Dials { declared, gaps }
    }

    pub fn get(&self, name: &str) -> Option<f64> {
        self.declared
            .iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| *value)
    }

    /// The dials this declaration knows, followed by the ones it names as unknown.
    pub fn names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.declared
            .iter()
            .map(|(key, _)| *key)
            .chain(self.gaps.iter().copied())
    }

    /// The dials the law needs and the declaration does not carry.
    pub const fn gaps(&self) -> &'static [&'static str] {
        self.gaps
    }

    pub fn len(&self) -> usize {
        self.declared.len() + self.gaps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.declared.is_empty() && self.gaps.is_empty()
    }
}

/// What a stage body returns: one vector per declared output field, in patch order.
#[derive(Clone, Debug, PartialEq)]
pub struct Produced {
    fields: Vec<(FieldId, Vec<f64>)>,
}

impl Produced {
    pub fn one(field: FieldId, values: Vec<f64>) -> Self {
        Produced { fields: vec![(field, values)] }
    }

    pub fn push(&mut self, field: FieldId, values: Vec<f64>) {
        self.fields.push((field, values));
    }

    pub fn field(&self, field: FieldId) -> Option<&[f64]> {
        self.fields
            .iter()
            .find(|(id, _)| *id == field)
            .map(|(_, values)| values.as_slice())
    }

    pub fn fields(&self) -> &[(FieldId, Vec<f64>)] {
        &self.fields
    }
}

/// The columns one run covers: `side × side` voxel columns at `origin.z`. `side == 1` is a **lone
/// site**, and a law that needs neighbours says so instead of guessing one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Patch {
    origin: VoxelPos,
    side: u32,
}

impl Patch {
    /// A patch of `side × side` columns. Zero is refused: an empty request is not a request.
    pub fn new(origin: VoxelPos, side: u32) -> Result<Self, Refusal> {
        if side == 0 {
            return Err(Refusal::new(
                RefusalKind::SiteAlone,
                "a patch of zero columns has no site to evaluate",
            ));
        }
        Ok(Patch { origin, side })
    }

    pub const fn origin(self) -> VoxelPos {
        self.origin
    }

    pub const fn side(self) -> u32 {
        self.side
    }

    pub const fn len(self) -> usize {
        (self.side * self.side) as usize
    }

    /// Never true: [`Patch::new`] refuses side 0, so a patch is at least a lone site. It exists so
    /// `len`'s contract is complete rather than so a caller can ask.
    pub const fn is_empty(self) -> bool {
        self.side == 0
    }

    /// Whether this patch has neighbours at all.
    pub const fn has_neighbours(self) -> bool {
        self.side >= 2
    }

    /// Whether the column at `index` is on the patch's rim. The rim is where the region is open —
    /// water leaves there — so a depression-filling law needs it and a closed-region law does not.
    pub fn is_edge(self, index: usize) -> bool {
        if index >= self.len() {
            return false;
        }
        let (x, y) = (index as u32 % self.side, index as u32 / self.side);
        x == 0 || y == 0 || x + 1 == self.side || y + 1 == self.side
    }

    /// The column at `index`, row-major.
    pub fn position(self, index: usize) -> Option<VoxelPos> {
        if index >= self.len() {
            return None;
        }
        let (x, y) = (index as u32 % self.side, index as u32 / self.side);
        Some(VoxelPos::new(
            self.origin.x + x as i32,
            self.origin.y + y as i32,
            self.origin.z,
        ))
    }

    /// Every column exactly once, in patch order. A body iterates this rather than indexing, so it
    /// never has to guess what an out-of-range index would mean.
    pub fn columns(self) -> impl Iterator<Item = (usize, VoxelPos)> {
        (0..self.len()).filter_map(move |index| self.position(index).map(|at| (index, at)))
    }

    /// The four-neighbour columns of `index`, each with the step taken to reach it. The step is what
    /// a gradient or a flow direction needs, and one place computes it.
    pub fn neighbour_steps(self, index: usize) -> Vec<(usize, i32, i32)> {
        if index >= self.len() {
            return Vec::new();
        }
        let (x, y) = (index as u32 % self.side, index as u32 / self.side);
        let side = self.side as usize;
        let mut out = Vec::with_capacity(4);
        if x > 0 {
            out.push((index - 1, -1, 0));
        }
        if x + 1 < self.side {
            out.push((index + 1, 1, 0));
        }
        if y > 0 {
            out.push((index - side, 0, -1));
        }
        if y + 1 < self.side {
            out.push((index + side, 0, 1));
        }
        out
    }

    pub fn index_of(self, at: VoxelPos) -> Option<usize> {
        let (dx, dy) = (at.x - self.origin.x, at.y - self.origin.y);
        if dx < 0 || dy < 0 || at.z != self.origin.z {
            return None;
        }
        let (x, y) = (dx as u32, dy as u32);
        if x >= self.side || y >= self.side {
            return None;
        }
        Some((y * self.side + x) as usize)
    }
}

/// What the caller asks for: one site, or a region. The distinction is a *contract* here, not a
/// hint — a region-only law asked for a lone site refuses by name (`site_alone`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessRequest {
    Unit { at: VoxelPos },
    Region { origin: VoxelPos, side: u32 },
}

impl ProcessRequest {
    pub fn patch(self) -> Result<Patch, Refusal> {
        match self {
            ProcessRequest::Unit { at } => Patch::new(at, 1),
            ProcessRequest::Region { origin, side } => Patch::new(origin, side),
        }
    }

    pub const fn kind_name(self) -> &'static str {
        match self {
            ProcessRequest::Unit { .. } => "unit",
            ProcessRequest::Region { .. } => "region",
        }
    }
}

/// The values already produced in this run: one vector per field, in patch order. This is the only
/// way a stage sees another stage's work, so a stage cannot sample a field nobody declared.
#[derive(Clone, Debug, PartialEq)]
pub struct FieldValues {
    patch: Patch,
    values: BTreeMap<FieldId, Vec<f64>>,
}

impl FieldValues {
    pub fn new(patch: Patch) -> Self {
        FieldValues { patch, values: BTreeMap::new() }
    }

    pub fn patch(&self) -> Patch {
        self.patch
    }

    /// Store one field. A vector of the wrong length is refused: a partial field is not a field.
    pub fn insert(&mut self, field: FieldId, values: Vec<f64>) -> Result<(), Refusal> {
        if values.len() != self.patch.len() {
            return Err(Refusal::new(
                RefusalKind::OutputUndeclared,
                format!(
                    "{} carries {} samples for a patch of {} columns",
                    field.name(),
                    values.len(),
                    self.patch.len()
                ),
            ));
        }
        self.values.insert(field, values);
        Ok(())
    }

    pub fn get(&self, field: FieldId) -> Option<&[f64]> {
        self.values.get(&field).map(Vec::as_slice)
    }

    pub fn has(&self, field: FieldId) -> bool {
        self.values.contains_key(&field)
    }

    pub fn produced(&self) -> Vec<&'static str> {
        self.values.keys().map(|field| field.name()).collect()
    }
}

/// What a stage body is handed: the request, the world's identity, the tick, its own dials, and the
/// values produced above it.
#[derive(Clone, Copy, Debug)]
pub struct StageContext<'a> {
    pub patch: Patch,
    pub identity: WorldIdentity,
    pub tick: Tick,
    /// The stage's position in the declared order — the order of the effects, and the tag that
    /// keeps two stages' noise fields apart.
    pub ordinal: u32,
    pub stage: &'static str,
    pub dials: &'a Dials,
    pub values: &'a FieldValues,
}

impl<'a> StageContext<'a> {
    /// A dial, or a refusal naming it. **Never** a default.
    pub fn dial(&self, name: &str) -> Result<f64, Refusal> {
        self.dials.get(name).ok_or_else(|| {
            Refusal::new(
                RefusalKind::DialUnset,
                format!("stage {} needs the dial {name}, and no value is declared", self.stage),
            )
        })
    }

    /// An input field, or a refusal naming the field and the stage that should have produced it.
    pub fn read(&self, field: FieldId) -> Result<&'a [f64], Refusal> {
        self.values.get(field).ok_or_else(|| {
            Refusal::new(
                RefusalKind::StageInputAbsent,
                format!("stage {} needs {}, which no declared stage above it produced", self.stage, field.name()),
            )
        })
    }

    /// A field of this stage's own noise, seeded by `(seed, tick, ordinal)`.
    ///
    /// The tick enters here and nowhere else: the world is **rebuilt** for a tick, never perturbed
    /// by it — which is what makes "one digest per `(seed, tick)`" a contract rather than a hope.
    pub fn noise(&self, salt: u64) -> Noise {
        let base = mix(self.identity.seed ^ mix(self.tick.0) ^ mix(u64::from(self.ordinal)));
        Noise::new(base, salt)
    }

    pub fn len(&self) -> usize {
        self.patch.len()
    }

    /// Never true, for the same reason [`Patch::is_empty`] never is: the context carries a patch.
    pub const fn is_empty(&self) -> bool {
        self.patch.is_empty()
    }

    pub fn position(&self, index: usize) -> Option<VoxelPos> {
        self.patch.position(index)
    }

    /// Every column of the request, in patch order.
    pub fn columns(&self) -> impl Iterator<Item = (usize, VoxelPos)> + '_ {
        self.patch.columns()
    }

    /// The four-neighbour columns of `index`, each with the step taken to reach it.
    pub fn neighbour_steps(&self, index: usize) -> Vec<(usize, i32, i32)> {
        self.patch.neighbour_steps(index)
    }

    /// An input field's values together with the patch they belong to.
    /// Whether the request has neighbours at all. A law that needs them refuses without them.
    pub fn require_region(&self) -> Result<(), Refusal> {
        if self.patch.has_neighbours() {
            Ok(())
        } else {
            Err(Refusal::new(
                RefusalKind::SiteAlone,
                format!(
                    "stage {} is a law about a relation between columns: a lone site cannot evaluate it",
                    self.stage
                ),
            ))
        }
    }
}

/// A stage body: pure, and either produces its declared fields or refuses by name.
pub type StageBody = fn(&StageContext<'_>) -> Result<Produced, Refusal>;

/// One declared stage.
#[derive(Clone, Copy, Debug)]
pub struct Stage {
    pub name: &'static str,
    /// The **one** paper (or first-principles ruling) the stage is built from. Empty is refused.
    pub source: &'static str,
    /// The stages whose outputs it consumes — each must be declared **above** it.
    pub reads: &'static [FieldId],
    /// The fields it produces. Empty is refused.
    pub writes: &'static [FieldId],
    pub cadence: Cadence,
    /// Whether the law is about a **relation between columns**. A region-only stage asked for a lone
    /// site is refused by name instead of being handed a neighbourhood nobody declared.
    pub requires_region: bool,
    pub dials: Dials,
    pub body: StageBody,
}

impl Stage {
    pub fn produces(&self, field: FieldId) -> bool {
        self.writes.contains(&field)
    }
}

/// The world's declared stage set: the ordered graph, validated once, and re-validatable by printing.
///
/// `Debug` is derived so a test (or a caller) that expects an admission to fail can report **what it
/// got instead** — an `Ok` — rather than losing the evidence, which is the same reason every refusal
/// carries its detail.
#[derive(Clone, Debug)]
pub struct Declared {
    stages: Vec<Stage>,
}

impl Declared {
    /// Admit a roster: collecting **every** problem, not the first.
    pub fn new(stages: Vec<Stage>) -> Result<Self, RefusalSet> {
        let mut refusals = RefusalSet::new();
        let mut seen_names: Vec<&'static str> = Vec::new();
        let mut claimed: Vec<FieldId> = Vec::new();

        for (ordinal, stage) in stages.iter().enumerate() {
            if stage.source.trim().is_empty() {
                refusals.refuse(
                    RefusalKind::NoProvenance,
                    format!("stage {} names no paper and no ruling", stage.name),
                );
            }
            if stage.writes.is_empty() {
                refusals.refuse(
                    RefusalKind::OutputUndeclared,
                    format!("stage {} declares no output", stage.name),
                );
            }
            if seen_names.contains(&stage.name) {
                refusals.refuse(
                    RefusalKind::DuplicateShaper,
                    format!("stage {} is declared twice", stage.name),
                );
            }
            seen_names.push(stage.name);

            for dial in stage.dials.gaps() {
                refusals.refuse(
                    RefusalKind::DialUnset,
                    format!("stage {} needs the dial {dial} and declares no value", stage.name),
                );
            }

            for field in stage.writes {
                if claimed.contains(field) {
                    refusals.refuse(
                        RefusalKind::DuplicateOutput,
                        format!("{} is declared by two stages; {} claims it again", field.name(), stage.name),
                    );
                }
                claimed.push(*field);
            }

            for field in stage.reads {
                let producer = stages[..ordinal].iter().find(|above| above.produces(*field));
                if producer.is_none() {
                    refusals.refuse(
                        RefusalKind::ReadsBelow,
                        format!(
                            "stage {} reads {}, which no stage declared above it writes",
                            stage.name,
                            field.name()
                        ),
                    );
                }
            }
        }

        refusals.into_result(Declared { stages })
    }

    pub fn stages(&self) -> &[Stage] {
        &self.stages
    }

    pub fn len(&self) -> usize {
        self.stages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.stages.is_empty()
    }

    pub fn stage(&self, name: &str) -> Option<&Stage> {
        self.stages.iter().find(|stage| stage.name == name)
    }

    /// The stage that declares this field as an output, if any.
    pub fn producer_of(&self, field: FieldId) -> Option<&Stage> {
        self.stages.iter().find(|stage| stage.produces(field))
    }

    pub fn ordinal_of(&self, name: &str) -> Option<usize> {
        self.stages.iter().position(|stage| stage.name == name)
    }

    /// The roster's own identity: names, sources, inputs, outputs, rungs, dials and order, in order.
    pub fn digest(&self) -> Digest {
        let mut parts: Vec<Vec<u8>> = Vec::new();
        for stage in &self.stages {
            parts.push(stage.name.as_bytes().to_vec());
            parts.push(stage.source.as_bytes().to_vec());
            parts.push(stage.cadence.rung().to_le_bytes().to_vec());
            for field in stage.reads {
                parts.push(field.name().as_bytes().to_vec());
            }
            parts.push(b"writes".to_vec());
            for field in stage.writes {
                parts.push(field.name().as_bytes().to_vec());
            }
            // The dials are part of the identity: a different dial is a different world.
            for name in stage.dials.names() {
                parts.push(name.as_bytes().to_vec());
                let value = stage.dials.get(name).unwrap_or(f64::NAN);
                parts.push(value.to_bits().to_le_bytes().to_vec());
            }
        }
        let borrowed: Vec<&[u8]> = parts.iter().map(Vec::as_slice).collect();
        Digest::of_parts(&borrowed)
    }
}

/// The **steepest elevation difference to a four-neighbour column**, in metres per metre, as a
/// magnitude (`>= 0`) — a slope, not a direction.
///
/// Derived from `docs/SHAPERS.md` §1's `substrate` (soil depth from slope) and `erosion` entries:
/// both laws take a slope, a slope is a relation between a column and its neighbours, so it is
/// computed here **once** from the patch rather than restated by whichever stage needs it. The
/// *direction* is a different question and belongs to the tool that routes water (`hydrology`'s D8
/// receiver), so this function deliberately does not answer it.
///
/// A column whose neighbours are all higher still has a slope: the gradient does not vanish in a
/// valley, only its sign changes.
pub fn slopes(patch: Patch, elevation: &[f64]) -> Vec<f64> {
    let mut out = vec![0.0; patch.len()];
    for index in 0..patch.len() {
        let (x, y) = (index as u32 % patch.side(), index as u32 / patch.side());
        let mut steepest = 0.0_f64;
        let neighbours = [
            (x > 0).then(|| index - 1),
            (x + 1 < patch.side()).then(|| index + 1),
            (y > 0).then(|| index - patch.side() as usize),
            (y + 1 < patch.side()).then(|| index + patch.side() as usize),
        ];
        for neighbour in neighbours.into_iter().flatten() {
            // One column is one voxel (1 m), so the height difference is already m/m.
            let difference = (elevation[index] - elevation[neighbour]).abs();
            if difference > steepest {
                steepest = difference;
            }
        }
        out[index] = steepest;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body_nothing(_: &StageContext<'_>) -> Result<Produced, Refusal> {
        Err(Refusal::new(RefusalKind::NotAGenerator, "test body"))
    }

    fn stage(
        name: &'static str,
        source: &'static str,
        reads: &'static [FieldId],
        writes: &'static [FieldId],
        dials: Dials,
    ) -> Stage {
        Stage {
            name,
            source,
            reads,
            writes,
            cadence: Cadence::PerTick,
            requires_region: false,
            dials,
            body: body_nothing,
        }
    }

    #[test]
    fn a_stage_without_a_source_is_refused_by_name() {
        let roster = Declared::new(vec![stage("orphan", "  ", &[], &[FieldId::Uplift], Dials::new(&[]))]);
        let refusals = roster.expect_err("a source-less stage must be refused");
        assert!(refusals.contains(RefusalKind::NoProvenance), "{refusals}");
        assert_eq!(refusals.first_named().unwrap().kind().name(), "no_provenance");
    }

    #[test]
    fn a_stage_that_declares_no_output_is_refused_by_name() {
        let roster = Declared::new(vec![stage("stub", "Perlin 1985", &[], &[], Dials::new(&[]))]);
        let refusals = roster.expect_err("a stage with no output must be refused");
        assert!(refusals.contains(RefusalKind::OutputUndeclared), "{refusals}");
    }

    #[test]
    fn a_stage_with_an_unset_dial_and_no_default_is_refused_by_name() {
        let roster = Declared::new(vec![stage(
            "erosion",
            "Whipple & Tucker 1999",
            &[],
            &[FieldId::ErosionRate],
            Dials::with_gaps(&[("k", 0.001)], &["m", "n"]),
        )]);
        let refusals = roster.expect_err("an unset dial must be refused");
        assert!(refusals.contains(RefusalKind::DialUnset), "{refusals}");
        // Both missing dials are named, not just the first.
        let names: Vec<&str> = refusals.all().iter().map(|r| r.detail()).collect();
        assert!(names.iter().any(|d| d.contains('m')), "{refusals}");
        assert!(names.iter().any(|d| d.contains('n')), "{refusals}");
    }

    #[test]
    fn a_stage_reading_below_it_is_refused_by_name() {
        // `terrain` consumes `uplift`, but `tectonics` is declared *after* it: not a graph.
        let roster = Declared::new(vec![
            stage("terrain", "Mandelbrot 1982", &[FieldId::Uplift], &[FieldId::Elevation], Dials::new(&[])),
            stage("tectonics", "Kreischer & Bommes 2019", &[], &[FieldId::Uplift], Dials::new(&[])),
        ]);
        let refusals = roster.expect_err("a backward read must be refused");
        assert!(refusals.contains(RefusalKind::ReadsBelow), "{refusals}");
    }

    #[test]
    fn two_stages_declaring_one_output_are_refused_by_name() {
        let roster = Declared::new(vec![
            stage("terrain", "Mandelbrot 1982", &[], &[FieldId::Elevation], Dials::new(&[])),
            stage("terrain_again", "Musgrave 1989", &[], &[FieldId::Elevation], Dials::new(&[])),
        ]);
        let refusals = roster.expect_err("a duplicated output must be refused");
        assert!(refusals.contains(RefusalKind::DuplicateOutput), "{refusals}");
    }

    #[test]
    fn a_declared_set_round_trips_through_its_printable_order() {
        let roster = Declared::new(vec![
            stage("tectonics", "Kreischer & Bommes 2019", &[], &[FieldId::Uplift], Dials::new(&[])),
            stage(
                "terrain",
                "Mandelbrot 1982",
                &[FieldId::Uplift],
                &[FieldId::Elevation],
                Dials::new(&[("relief_m", 120.0)]),
            ),
        ])
        .expect("a valid graph must admit");
        assert_eq!(roster.len(), 2);
        assert_eq!(roster.ordinal_of("terrain"), Some(1));
        assert_eq!(roster.producer_of(FieldId::Elevation).unwrap().name, "terrain");
        assert!(roster.producer_of(FieldId::Biome).is_none(), "nothing declares biome here");
        // The identity is stable, and a moved dial moves it.
        assert_eq!(roster.digest(), roster.digest());
        let mut changed = roster.clone();
        changed.stages[1].dials = Dials::new(&[("relief_m", 121.0)]);
        assert_ne!(roster.digest(), changed.digest(), "a dial is part of the identity");
    }

    #[test]
    fn a_rung_off_the_closed_ladder_is_refused_by_name() {
        for rung in 0..4u8 {
            assert_eq!(Cadence::from_rung(rung).unwrap().rung(), rung);
        }
        let refused = Cadence::from_rung(4).expect_err("a fifth rung does not exist");
        assert_eq!(refused.kind(), RefusalKind::CadenceOffLadder);
        assert!(refused.detail().contains('4'));
        // The ladder is closed, ordered and declares its own spacing.
        for (index, rung) in Cadence::ALL.iter().enumerate() {
            assert_eq!(rung.rung() as usize, index);
            assert!(rung.ticks_between() >= 1);
        }
    }

    #[test]
    fn a_patch_knows_its_columns_its_neighbours_and_a_lone_site() {
        let origin = VoxelPos::new(-4, 7, 0);
        let patch = Patch::new(origin, 3).expect("a 3x3 patch");
        assert_eq!(patch.len(), 9);
        assert!(patch.has_neighbours());
        assert_eq!(patch.position(0), Some(origin));
        assert_eq!(patch.position(8), Some(VoxelPos::new(-2, 9, 0)));
        assert_eq!(patch.position(9), None);
        assert_eq!(patch.index_of(VoxelPos::new(-3, 8, 0)), Some(4));
        assert_eq!(patch.index_of(VoxelPos::new(-4, 7, 1)), None, "another z is another world");

        let lone = Patch::new(origin, 1).expect("a lone site is a patch of one");
        assert!(!lone.has_neighbours(), "a lone site has no neighbour to borrow");
        assert!(Patch::new(origin, 0).is_err(), "an empty request is not a request");
    }

    #[test]
    fn slopes_measure_the_steepest_difference_to_a_neighbour() {
        let patch = Patch::new(VoxelPos::new(0, 0, 0), 3).expect("a 3x3 patch");
        // A ramp falling to +x, 2 m per column: every column has a neighbour 2 m away.
        let ramp: Vec<f64> = (0..9).map(|i| 9.0 - (i % 3) as f64 * 2.0).collect();
        let measured = slopes(patch, &ramp);
        assert_eq!(measured.len(), 9);
        for slope in &measured {
            assert!((*slope - 2.0).abs() < 1e-12, "expected a 2 m step, got {slope}");
        }

        // The *steepest* of the neighbours wins, and the magnitude is direction-free: a valley
        // column whose neighbours are all higher still has a slope.
        let mut steep = vec![10.0; 9];
        steep[4] = 0.0; // the centre drops 10 m to each of its four neighbours
        let measured = slopes(patch, &steep);
        assert_eq!(measured[4], 10.0, "the pit measures its steepest neighbour");
        assert_eq!(measured[1], 10.0, "and so does every column that borders it");
        assert_eq!(
            measured[0], 0.0,
            "a corner whose neighbours share its own height has no slope to report"
        );

        // A flat patch has no slope at all.
        let flat = slopes(patch, &[5.0; 9]);
        assert!(flat.iter().all(|s| *s == 0.0));
    }

    #[test]
    fn a_field_of_the_wrong_length_is_refused_by_name() {
        let patch = Patch::new(VoxelPos::new(0, 0, 0), 2).expect("a 2x2 patch");
        let mut values = FieldValues::new(patch);
        let refused = values.insert(FieldId::Elevation, vec![1.0, 2.0]).expect_err("3 < 4");
        assert_eq!(refused.kind(), RefusalKind::OutputUndeclared);
        assert!(values.insert(FieldId::Elevation, vec![1.0; 4]).is_ok());
        assert!(values.has(FieldId::Elevation));
        assert_eq!(values.produced(), vec!["elevation"]);
    }
}
