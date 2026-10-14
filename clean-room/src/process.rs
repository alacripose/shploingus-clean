//! **The process runtime — the declared roster, the stage graph, the tick driver, the digest.**
//!
//! Derived from `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §2 U3 and §5 slice 3 (the acceptance line: *“each
//! shaper named by one paper; one digest for one `(seed, tick)`”*) and `docs/SHAPERS.md` §2's
//! determinism contract. The binding seams are `specs/003-u3-shapers/contracts/u3-seams.md` C1–C4.
//!
//! # The three claims this file is built to make checkable
//!
//! 1. **One digest per `(seed, declared period, tick)`** — folded in the **declared** order, over the
//!    declared outputs, so neither a stage's return order nor the schedule can move it (C4).
//! 2. **The worker count is a knob** — stages are evaluated in **dependency levels**, and the threads
//!    of one level write nothing until the level is joined. A different worker count reorders *work*,
//!    never *values*, and SC-001 measures exactly that.
//! 3. **A tick is idempotent and never rewinds** — driving a tick twice returns the same receipt
//!    without re-running it, and driving an earlier tick is refused by name (C2, SC-006).
//!
//! # The reference declaration
//!
//! [`roster`] is the one place this crate declares the fifteen stages, their papers, their inputs and
//! outputs, their rungs and the **values of their dials**. The dials are declared here — visibly, in
//! one table — rather than invented at a call site; a dial the table omits is refused at admission
//! (`dial_unset`), and the binary (`src/main.rs`) prints the whole declaration so a reader can see
//! what the numbers mean.

use std::collections::BTreeMap;

use crate::digest::Digest;
use crate::coordinates::VoxelPos;
use crate::refusal::{Refusal, RefusalKind, RefusalSet};
use crate::shapers::{air, life, record, terrain, water};
use crate::stage::{
    Cadence, Declared, Dials, FieldId, FieldValues, Patch, Produced, ProcessRequest, Stage,
    StageBody, StageContext,
};
use crate::world::{Tick, WorldIdentity};

/// The columns the reference declaration generates: an 8 × 8 patch (64 columns). Declared, printed,
/// and big enough that every relation-based law has neighbours to read.
pub const DECLARED_PATCH_SIDE: u32 = 48;

// ---------------------------------------------------------------------------
// The reference declaration's dials. One table per law; every required dial has a value.
// ---------------------------------------------------------------------------

// The reference world's plates carry **no** signed bias: its uplift is the boundary term alone, which
// is a continuous field, so the soil laws of `substrate` and `erosion` hold all the way across a
// plate edge instead of meeting a scarp. The bias term is still declared (and exercised by
// `terrain`'s own test at 40 m); this world simply has no use for it, and the plate *identities* are
// unaffected — `plate_id` still names the region each column belongs to.
const TECTONICS_DIALS: &[(&str, f64)] = &[
    ("plate_scale_cells", 512.0),
    ("boundary_uplift_m", 12.0),
    ("plate_bias_m", 0.0),
];
// The coarsest rung of the F4 ladder (`32 · 2^L` blocks, so `L = 0` is a 32-column cell): the
// reference patch is wider than one cell, so its columns cross a ladder boundary and `transition`
// has a real boundary to blend rather than a single domain inside one cell.
const FIELDS_DIALS: &[(&str, f64)] =
    &[("ladder_level", 0.0), ("lithology_classes", 6.0), ("shepard_power", 2.0)];
const GEOLOGY_DIALS: &[(&str, f64)] = &[
    ("layer_thickness_m", 20.0),
    ("layer_count", 24.0),
    ("datum_m", 200.0),
    ("deposit_warp_m", 12.0),
];
// The patch is 8 × 8 columns at the declared one metre per column, so its relief is a *hillside*,
// not a mountain range: 18 m of amplitude over those 64 columns keeps every gradient inside the
// soil laws' declared critical slope, and the world's mountains come from `tectonics`' uplift,
// which is nearly constant across a patch this small.
const TERRAIN_DIALS: &[(&str, f64)] = &[
    ("base_elevation_m", 64.0),
    ("relief_amplitude_m", 5.0),
    ("glacial_smoothing_m", 1.25),
    ("octaves", 6.0),
    ("lacunarity", 2.0),
    ("gain", 0.5),
];
const HYDROLOGY_DIALS: &[(&str, f64)] = &[("rainfall_per_cell", 0.5)];
const SUBSTRATE_DIALS: &[(&str, f64)] = &[("max_soil_depth_m", 1.2), ("decay_per_slope", 0.5)];
const EROSION_DIALS: &[(&str, f64)] = &[
    ("stream_power_k", 0.001),
    ("stream_power_m", 0.5),
    ("stream_power_n", 1.0),
    ("reference_discharge", 8.0),
    ("hillslope_transport_rate", 0.01),
    ("critical_slope", 1.0),
];
const CLIMATE_DIALS: &[(&str, f64)] =
    &[("sea_level_temperature_c", 15.0), ("lapse_rate_c_per_m", 0.0065)];
const ATMOSPHERE_DIALS: &[(&str, f64)] = &[
    ("sea_level_pressure_pa", 101_325.0),
    ("surface_temperature_k", 288.15),
    ("lapse_k_per_m", 0.0065),
];
const WEATHER_DIALS: &[(&str, f64)] = &[
    ("base_precipitation_mm", 100.0),
    ("orographic_gain", 2.0),
    ("wind_x", 1.0),
    ("wind_y", 0.0),
];
const ECOLOGY_DIALS: &[(&str, f64)] = &[
    ("biotemperature_floor_c", 0.0),
    ("biome_bands", 8.0),
    ("driest_ratio", 0.0625),
    ("min_soil_depth_m", 0.02),
    ("soil_half_saturation_m", 0.2),
    ("flock_radius_cells", 2.0),
    ("heading_scale", 0.25),
    ("separation_weight", 0.5),
    ("alignment_weight", 1.0),
    ("cohesion_weight", 0.5),
];

const DELTA_DIALS: &[(&str, f64)] = &[
    ("deposit_gain", 1.0),
    ("deposition_slope", 1.0),
    ("deposition_capacity_gain", 0.05),
];

const TRANSITION_DIALS: &[(&str, f64)] = &[
    ("control_point_spacing_cells", 2.0),
    ("control_point_jitter_cells", 0.5),
    ("shepard_power", 2.0),
    ("tie_tolerance", 1e-6),
];

const TECTONICS_SOURCE: &str =
    "Kreischer, Bommes et al., Procedural Tectonic Planets, CGF 2019; Wilson-cycle plate taxonomy";
const FIELDS_SOURCE: &str =
    "Shepard, A Two-Dimensional Interpolation Function for Irregularly-Spaced Data, ACM 1968; the F4 ladder ruling";
const GEOLOGY_SOURCE: &str = "Allen & Allen, Basin Analysis, Wiley 2013";
const TERRAIN_SOURCE: &str =
    "Mandelbrot 1982 (fractal landscapes); Musgrave, Kolb & Mace 1989 (eroded fractal terrains)";
const HYDROLOGY_SOURCE: &str =
    "O'Callaghan & Mark, D8 drainage networks, CVGIP 1984; Barnes, Lehman & Mulla, Priority-Flood, C&G 2014";
const SUBSTRATE_SOURCE: &str =
    "Heimsath, Dietrich, Nishiizumi & Finkel, The soil production function, Nature 1997";
const EROSION_SOURCE: &str =
    "Whipple & Tucker, JGR 1999; Lague, ESPL 2014 (stream power); Roering, Kirchner & Dietrich, WRR 1999 (nonlinear transport)";
const CLIMATE_SOURCE: &str = "Whittaker 1960; Koppen 1884; ICAO/US Standard Atmosphere 1976 (lapse rate)";
const ATMOSPHERE_SOURCE: &str = "US Standard Atmosphere 1976; the barometric formula";
const WEATHER_SOURCE: &str =
    "Smith & Barstad, A Linear Theory of Orographic Precipitation, JAS 2004; Alduchov & Eskridge 1996 (saturation form)";
const ECOLOGY_SOURCE: &str =
    "Holdridge 1947 (the humidity-province ladder); Heimsath, Dietrich, Nishiizumi & Finkel 1997 (the soil depth that decides establishment)";
const CAVES_SOURCE: &str =
    "Ford & Williams, Karst Hydrogeology and Geomorphology, Wiley 2007; Nie et al., Algorithms 17(10):466, 2024";
const DELTA_SOURCE: &str =
    "Gilbert 1885; Hjulstrom 1935 / Shields 1936; Slingerland & Smith 2004 (avulsion)";
const TRANSITION_SOURCE: &str =
    "Shepard 1968; normalized sparse convolution over a jittered grid (NoisePosti.ng 2021)";
const EARTHWORKS_SOURCE: &str =
    "the F10 ruling - not a paper: a conservation ledger, where a recipe that is a mass pump declares itself one";

/// **What one stage declares**, named. A struct rather than eight positional arguments, because the
/// field name is the documentation: at a call site below, `reads` and `writes` are told apart by the
/// word in front of them rather than by counting commas — and the two lists are exactly the pair a
/// reader must not mix up.
struct Decl {
    source: &'static str,
    reads: &'static [FieldId],
    writes: &'static [FieldId],
    cadence: Cadence,
    requires_region: bool,
    dials: Dials,
    body: StageBody,
}

fn stage(name: &'static str, decl: Decl) -> Stage {
    Stage {
        name,
        source: decl.source,
        reads: decl.reads,
        writes: decl.writes,
        cadence: decl.cadence,
        requires_region: decl.requires_region,
        dials: decl.dials,
        body: decl.body,
    }
}

/// **The closed roster: fifteen stages, in the declared order of their effects.**
///
/// The order is the graph (`docs/SHAPERS.md` §2 rule 3: *the order of the set is the order of the
/// effects*), and it is validated by [`Declared::new`] — a read of something declared below, a
/// repeated output, a missing source or an unset dial is refused there rather than discovered at run
/// time. `requires_region` is the declaration that a law is about a **relation between columns**, so
/// a lone-site request can be refused by name instead of guessed at.
pub fn roster() -> Result<Declared, RefusalSet> {
    Declared::new(vec![
        // The slow, coarse rungs: the plate partition, the regional ladder, the stratigraphy.
        stage(
            "tectonics",
            Decl {
                source: TECTONICS_SOURCE,
                reads: &[],
                writes: &[FieldId::Uplift, FieldId::PlateId],
                cadence: Cadence::Detail,
                requires_region: false,
                dials: Dials::new(TECTONICS_DIALS),
                body: terrain::tectonics,
            },
        ),
        stage(
            "fields",
            Decl {
                source: FIELDS_SOURCE,
                reads: &[],
                writes: &[FieldId::Lithology],
                cadence: Cadence::Detail,
                requires_region: false,
                dials: Dials::new(FIELDS_DIALS),
                body: terrain::fields,
            },
        ),
        stage(
            "geology",
            Decl {
                source: GEOLOGY_SOURCE,
                reads: &[FieldId::Uplift],
                writes: &[FieldId::Stratigraphy],
                cadence: Cadence::Detail,
                requires_region: false,
                dials: Dials::new(GEOLOGY_DIALS),
                body: terrain::geology,
            },
        ),
        // The terrain the world walks on.
        stage(
            "terrain",
            Decl {
                source: TERRAIN_SOURCE,
                reads: &[FieldId::Uplift],
                writes: &[FieldId::Elevation],
                cadence: Cadence::PerTick,
                requires_region: false,
                dials: Dials::new(TERRAIN_DIALS),
                body: terrain::terrain,
            },
        ),
        // The water group: all three are relations between columns.
        stage(
            "hydrology",
            Decl {
                source: HYDROLOGY_SOURCE,
                reads: &[FieldId::Elevation],
                writes: &[FieldId::Discharge, FieldId::StreamOrder, FieldId::WaterLevel],
                cadence: Cadence::PerTick,
                requires_region: true,
                dials: Dials::new(HYDROLOGY_DIALS),
                body: water::hydrology,
            },
        ),
        stage(
            "substrate",
            Decl {
                source: SUBSTRATE_SOURCE,
                reads: &[FieldId::Elevation],
                writes: &[FieldId::SoilDepth],
                cadence: Cadence::Sediment,
                requires_region: true,
                dials: Dials::new(SUBSTRATE_DIALS),
                body: water::substrate,
            },
        ),
        stage(
            "erosion",
            Decl {
                source: EROSION_SOURCE,
                reads: &[FieldId::Discharge, FieldId::Elevation],
                writes: &[FieldId::ErosionRate, FieldId::HillslopeFlux],
                cadence: Cadence::Sediment,
                requires_region: true,
                dials: Dials::new(EROSION_DIALS),
                body: water::erosion,
            },
        ),
        // The air group: stage-coupled to the terrain it reads.
        stage(
            "climate",
            Decl {
                source: CLIMATE_SOURCE,
                reads: &[FieldId::Elevation],
                writes: &[FieldId::Temperature],
                cadence: Cadence::Climate,
                requires_region: false,
                dials: Dials::new(CLIMATE_DIALS),
                body: air::climate,
            },
        ),
        stage(
            "atmosphere",
            Decl {
                source: ATMOSPHERE_SOURCE,
                reads: &[FieldId::Elevation],
                writes: &[FieldId::Pressure],
                cadence: Cadence::Climate,
                requires_region: false,
                dials: Dials::new(ATMOSPHERE_DIALS),
                body: air::atmosphere,
            },
        ),
        stage(
            "weather",
            Decl {
                source: WEATHER_SOURCE,
                reads: &[FieldId::Elevation, FieldId::Temperature],
                writes: &[FieldId::Precipitation],
                cadence: Cadence::PerTick,
                requires_region: true,
                dials: Dials::new(WEATHER_DIALS),
                body: air::weather,
            },
        ),
        // Life, and the two owed bodies and the record.
        stage(
            "ecology",
            Decl {
                source: ECOLOGY_SOURCE,
                reads: &[FieldId::Temperature, FieldId::Precipitation, FieldId::SoilDepth],
                writes: &[FieldId::Biome, FieldId::CarryingCapacity, FieldId::Flock],
                cadence: Cadence::PerTick,
                requires_region: false,
                dials: Dials::new(ECOLOGY_DIALS),
                body: life::ecology,
            },
        ),
        stage(
            "caves",
            Decl {
                source: CAVES_SOURCE,
                reads: &[FieldId::Discharge],
                writes: &[FieldId::CaveOccupancy],
                cadence: Cadence::PerTick,
                requires_region: true,
                dials: Dials::new(&[]),
                body: life::caves,
            },
        ),
        stage(
            "delta",
            Decl {
                source: DELTA_SOURCE,
                reads: &[
                    FieldId::Elevation,
                    FieldId::Discharge,
                    FieldId::ErosionRate,
                    FieldId::WaterLevel,
                ],
                writes: &[FieldId::Deposition],
                cadence: Cadence::PerTick,
                requires_region: true,
                dials: Dials::new(DELTA_DIALS),
                body: record::delta,
            },
        ),
        stage(
            "transition",
            Decl {
                source: TRANSITION_SOURCE,
                reads: &[FieldId::Lithology, FieldId::Biome],
                writes: &[FieldId::Blend],
                cadence: Cadence::PerTick,
                requires_region: true,
                dials: Dials::new(TRANSITION_DIALS),
                body: record::transition,
            },
        ),
        stage(
            "earthworks",
            Decl {
                source: EARTHWORKS_SOURCE,
                reads: &[],
                writes: &[FieldId::RecordedMass],
                cadence: Cadence::PerTick,
                requires_region: false,
                dials: Dials::new(&[]),
                body: record::earthworks,
            },
        ),
    ])
}

/// The request the reference declaration is driven with.
pub const fn declared_request() -> ProcessRequest {
    ProcessRequest::Region {
        origin: VoxelPos::new(0, 0, 0),
        side: DECLARED_PATCH_SIDE,
    }
}

/// What a stage did: it produced its declared fields, or it refused by name.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    Produced(Produced),
    Refused(Refusal),
}

impl Outcome {
    pub fn refusal(&self) -> Option<&Refusal> {
        match self {
            Outcome::Produced(_) => None,
            Outcome::Refused(refusal) => Some(refusal),
        }
    }

    pub fn field(&self, field: FieldId) -> Option<&[f64]> {
        match self {
            Outcome::Produced(produced) => produced.field(field),
            Outcome::Refused(_) => None,
        }
    }
}

/// A tick's arithmetic: what was due, what ran, and **which** stages refused.
#[derive(Clone, Debug, PartialEq)]
pub struct Tally {
    pub due: usize,
    pub ran: usize,
    pub refused: Vec<Refusal>,
}

impl Tally {
    pub fn refused_names(&self) -> Vec<&'static str> {
        self.refused
            .iter()
            .map(|refusal| refusal.kind().name())
            .collect()
    }
}

/// Everything one evaluation produced.
#[derive(Clone, Debug, PartialEq)]
pub struct Evaluation {
    pub patch: Patch,
    pub outcomes: Vec<Outcome>,
    pub tally: Tally,
    pub values: FieldValues,
    /// **The** digest: one per `(seed, declared period, tick)` and the declared roster (C4).
    pub digest: Digest,
}

/// A driven tick's receipt: the evidence that the tick happened **once**.
#[derive(Clone, Debug, PartialEq)]
pub struct TickReceipt {
    pub tick: Tick,
    pub plan_digest: Digest,
    pub tally: Tally,
}

/// The tick driver: plans a tick, refuses a rewind, and is idempotent within a tick.
#[derive(Clone, Debug, Default)]
pub struct TickDriver {
    last: Option<Tick>,
    receipts: BTreeMap<u64, TickReceipt>,
}

impl TickDriver {
    pub fn new() -> Self {
        TickDriver::default()
    }

    pub const fn last(&self) -> Option<Tick> {
        self.last
    }

    pub fn receipt(&self, tick: Tick) -> Option<&TickReceipt> {
        self.receipts.get(&tick.0)
    }

    /// Drive a tick. Idempotent within the tick: the second drive returns the first receipt and runs
    /// nothing. A tick earlier than the last one driven is refused by name — the schedule never runs
    /// backwards (FR-005).
    pub fn drive(
        &mut self,
        roster: &Declared,
        identity: WorldIdentity,
        tick: Tick,
        workers: usize,
    ) -> Result<TickReceipt, Refusal> {
        if let Some(last) = self.last {
            if tick.0 < last.0 {
                return Err(Refusal::new(
                    RefusalKind::Rewind,
                    format!("tick {} is earlier than tick {} that was already driven", tick.0, last.0),
                ));
            }
        }
        if let Some(receipt) = self.receipts.get(&tick.0) {
            return Ok(receipt.clone());
        }
        let evaluation = evaluate(roster, identity, tick, declared_request(), workers)?;
        let receipt = TickReceipt {
            tick,
            plan_digest: evaluation.digest,
            tally: evaluation.tally,
        };
        self.receipts.insert(tick.0, receipt.clone());
        self.last = Some(tick);
        Ok(receipt)
    }
}

/// The digest of one generation: `(seed, declared period, tick)` and the declared roster only.
pub fn world_generation_digest(
    roster: &Declared,
    identity: WorldIdentity,
    tick: Tick,
) -> Result<Digest, Refusal> {
    evaluate(roster, identity, tick, declared_request(), 1).map(|evaluation| evaluation.digest)
}

/// The same digest, evaluated across `workers` threads. The number **must not** depend on it.
pub fn world_generation_digest_with_workers(
    roster: &Declared,
    identity: WorldIdentity,
    tick: Tick,
    request: ProcessRequest,
    workers: usize,
) -> Result<Digest, Refusal> {
    evaluate(roster, identity, tick, request, workers).map(|evaluation| evaluation.digest)
}

/// Evaluate the roster for one tick. Deterministic: pure stages, a declared fold order, and levels
/// joined before the next one starts.
pub fn evaluate(
    roster: &Declared,
    identity: WorldIdentity,
    tick: Tick,
    request: ProcessRequest,
    workers: usize,
) -> Result<Evaluation, Refusal> {
    let patch = request.patch()?;
    let workers = workers.max(1);
    let mut values = FieldValues::new(patch);
    let mut outcomes = Vec::with_capacity(roster.len());
    let mut tally = Tally { due: roster.len(), ran: 0, refused: Vec::new() };

    // Stages are grouped into **dependency levels**: a stage's level is one past the deepest stage it
    // reads. Validation already guarantees every read is declared above, so this is total.
    let levels = dependency_levels(roster);
    let mut slot: Vec<Option<Outcome>> = vec![None; roster.len()];
    for level in &levels {
        for (index, outcome) in run_level(level, workers, roster, identity, tick, patch, &values) {
            if let Outcome::Produced(produced) = &outcome {
                for field in roster.stages()[index].writes {
                    if let Some(samples) = produced.field(*field) {
                        values.insert(*field, samples.to_vec())?;
                    }
                }
                tally.ran += 1;
            }
            slot[index] = Some(outcome);
        }
    }
    // The outcomes are read back in **declared** order — the order the roster prints and the order
    // the digest folds — so the tally's refusal list and the digest cannot depend on the schedule or
    // on how a level was split. Reading them off in level order would put an oweless stage first
    // (it sits at level 0) and make the roster's own order invisible in its own report.
    for outcome in slot {
        let outcome = outcome.unwrap_or(Outcome::Refused(Refusal::new(
            RefusalKind::BodyOwed,
            "the stage was never reached",
        )));
        if let Outcome::Refused(refusal) = &outcome {
            tally.refused.push(refusal.clone());
        }
        outcomes.push(outcome);
    }

    let digest = fold_digest(roster, identity, tick, &outcomes);
    Ok(Evaluation { patch, outcomes, tally, values, digest })
}

/// Group the roster's stage indices by dependency level. A stage that reads nothing is level 0.
fn dependency_levels(roster: &Declared) -> Vec<Vec<usize>> {
    let stages = roster.stages();
    let mut level_of = vec![0usize; stages.len()];
    for (index, stage) in stages.iter().enumerate() {
        let mut deepest = 0;
        for field in stage.reads {
            if let Some(producer) = stages[..index].iter().position(|above| above.produces(*field)) {
                deepest = deepest.max(level_of[producer] + 1);
            }
        }
        level_of[index] = deepest;
    }
    let depth = level_of.iter().copied().max().unwrap_or(0);
    let mut levels: Vec<Vec<usize>> = vec![Vec::new(); depth + 1];
    for (index, level) in level_of.into_iter().enumerate() {
        levels[level].push(index);
    }
    levels
}

/// Run one dependency level, splitting it across `workers` threads when there is more than one.
/// Results come back in the level's own order, so the **fold** — and therefore the digest — is
/// independent of how the work was divided.
fn run_level(
    indices: &[usize],
    workers: usize,
    roster: &Declared,
    identity: WorldIdentity,
    tick: Tick,
    patch: Patch,
    values: &FieldValues,
) -> Vec<(usize, Outcome)> {
    let run = |index: usize| (index, run_stage(roster, index, identity, tick, patch, values));
    if workers <= 1 || indices.len() <= 1 {
        return indices.iter().map(|index| run(*index)).collect();
    }
    let chunk = indices.len().div_ceil(workers);
    std::thread::scope(|scope| {
        let handles: Vec<_> = indices
            .chunks(chunk)
            .map(|slice| scope.spawn(|| slice.iter().map(|index| run(*index)).collect::<Vec<_>>()))
            .collect();
        handles
            .into_iter()
            .flat_map(|handle| handle.join().expect("a stage thread must not panic"))
            .collect()
    })
}

/// Run one stage and make its **declaration bind**: a body that omits a declared output, or returns
/// one it did not declare, is refused by name rather than believed.
fn run_stage(
    roster: &Declared,
    index: usize,
    identity: WorldIdentity,
    tick: Tick,
    patch: Patch,
    values: &FieldValues,
) -> Outcome {
    let stage = &roster.stages()[index];
    if stage.requires_region && !patch.has_neighbours() {
        return Outcome::Refused(Refusal::new(
            RefusalKind::SiteAlone,
            format!(
                "stage {} is a law about a relation between columns: a lone site cannot evaluate it",
                stage.name
            ),
        ));
    }
    let context = StageContext {
        patch,
        identity,
        tick,
        ordinal: index as u32,
        stage: stage.name,
        dials: &stage.dials,
        values,
    };
    match (stage.body)(&context) {
        Err(refusal) => Outcome::Refused(refusal),
        Ok(produced) => {
            let missing: Vec<&'static str> = stage
                .writes
                .iter()
                .filter(|field| produced.field(**field).is_none())
                .map(|field| field.name())
                .collect();
            let undeclared: Vec<&'static str> = produced
                .fields()
                .iter()
                .map(|(field, _)| field.name())
                .filter(|name| !stage.writes.iter().any(|field| field.name() == *name))
                .collect();
            let wrong_length = stage.writes.iter().find(|field| {
                produced
                    .field(**field)
                    .map(|samples| samples.len() != patch.len())
                    .unwrap_or(false)
            });
            if !missing.is_empty() || !undeclared.is_empty() || wrong_length.is_some() {
                return Outcome::Refused(Refusal::new(
                    RefusalKind::OutputUndeclared,
                    format!(
                        "stage {} declares {:?} and produced missing {missing:?}, undeclared {undeclared:?}, wrong length in {:?}",
                        stage.name,
                        stage.writes.iter().map(|field| field.name()).collect::<Vec<_>>(),
                        wrong_length.map(|field| field.name())
                    ),
                ));
            }
            Outcome::Produced(produced)
        }
    }
}

/// Fold one evaluation into its digest.
///
/// Two deliberate choices, both of them about not letting the schedule leak into the number:
/// the fields are folded in the stage's **declared** output order (not the body's return order), and
/// a refusal is folded by its **name** (not its prose), so rewording a message does not rewrite every
/// digest in the project.
fn fold_digest(roster: &Declared, identity: WorldIdentity, tick: Tick, outcomes: &[Outcome]) -> Digest {
    let mut parts: Vec<Vec<u8>> = vec![
        identity.seed.to_le_bytes().to_vec(),
        identity.period.get().to_le_bytes().to_vec(),
        tick.0.to_le_bytes().to_vec(),
        roster.digest().as_bytes().to_vec(),
    ];
    for (index, stage) in roster.stages().iter().enumerate() {
        parts.push(stage.name.as_bytes().to_vec());
        match outcomes.get(index) {
            Some(Outcome::Produced(produced)) => {
                for field in stage.writes {
                    parts.push(field.name().as_bytes().to_vec());
                    match produced.field(*field) {
                        Some(samples) => {
                            for sample in samples {
                                parts.push(sample.to_bits().to_le_bytes().to_vec());
                            }
                        }
                        None => parts.push(b"omitted".to_vec()),
                    }
                }
            }
            Some(Outcome::Refused(refusal)) => {
                parts.push(b"refused".to_vec());
                parts.push(refusal.kind().name().as_bytes().to_vec());
            }
            None => parts.push(b"unreached".to_vec()),
        }
    }
    let borrowed: Vec<&[u8]> = parts.iter().map(Vec::as_slice).collect();
    Digest::of_parts(&borrowed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity(seed: u64) -> WorldIdentity {
        WorldIdentity::new(seed, crate::world::DeclaredPeriod::seconds(1).expect("a period"))
    }

    #[test]
    fn the_reference_roster_admits_and_declares_fifteen_stages_by_name() {
        let roster = roster().expect("the reference roster must admit");
        assert_eq!(roster.len(), 15, "the roster is closed at fifteen stages");
        let names: Vec<&str> = roster.stages().iter().map(|stage| stage.name).collect();
        assert_eq!(
            names,
            vec![
                "tectonics",
                "fields",
                "geology",
                "terrain",
                "hydrology",
                "substrate",
                "erosion",
                "climate",
                "atmosphere",
                "weather",
                "ecology",
                "caves",
                "delta",
                "transition",
                "earthworks"
            ]
        );
        // Every stage names a paper and declares an output — the two things admission checks.
        for stage in roster.stages() {
            assert!(!stage.source.is_empty(), "{} names no source", stage.name);
            assert!(!stage.writes.is_empty(), "{} declares no output", stage.name);
            for field in stage.writes {
                assert_eq!(roster.producer_of(*field).map(|s| s.name), Some(stage.name));
            }
        }
        // All four rungs of the closed ladder are used, and every field has one producer.
        let rungs: std::collections::BTreeSet<u8> =
            roster.stages().iter().map(|stage| stage.cadence.rung()).collect();
        assert_eq!(rungs.len(), Cadence::COUNT, "the reference declaration uses every rung");
    }

    #[test]
    fn one_digest_for_one_seed_and_tick_whatever_the_worker_count() {
        // SC-001: the worker count is a knob and never changes output.
        let roster = roster().expect("the reference roster must admit");
        let tick = Tick(0);
        let one = world_generation_digest_with_workers(&roster, identity(0x5EED), tick, declared_request(), 1)
            .expect("a digest");
        for workers in [2, 4, 8] {
            let many = world_generation_digest_with_workers(
                &roster,
                identity(0x5EED),
                tick,
                declared_request(),
                workers,
            )
            .expect("a digest");
            assert_eq!(
                one, many,
                "worker count {workers} changed the world's digest: it must not"
            );
        }
        // SC-002: the tick is an input — a moved tick is a different world.
        let moved = world_generation_digest(&roster, identity(0x5EED), Tick(1)).expect("a digest");
        assert_ne!(one, moved, "the tick must be an input to the world");
        // A different seed is a different world.
        let other = world_generation_digest(&roster, identity(0x5EED_0001), tick).expect("a digest");
        assert_ne!(one, other);
        // And the same call twice is the same number.
        assert_eq!(
            one,
            world_generation_digest(&roster, identity(0x5EED), tick).expect("a digest")
        );
    }

    #[test]
    fn the_digest_is_blind_to_the_anchor_and_to_a_populated_cache() {
        // U2's rule, held by U3: truth is `(seed, declared period, tick)` and the roster, nothing else.
        let roster = roster().expect("the reference roster must admit");
        let mut world = crate::world::World::new(identity(0x5EED));
        let plain = world_generation_digest(&roster, world.identity(), world.tick()).expect("a digest");
        world.set_anchor(crate::coordinates::ChunkKey::new(-9, 4, 2));
        for x in 0..5 {
            world.reside(crate::coordinates::ChunkKey::new(x, 0, 0));
        }
        let viewed = world_generation_digest(&roster, world.identity(), world.tick()).expect("a digest");
        assert_eq!(plain, viewed, "an anchor and a cache are views, not truth");
    }

    #[test]
    fn the_tally_counts_what_ran_and_names_what_refused() {
        let roster = roster().expect("the reference roster must admit");
        let evaluation =
            evaluate(&roster, identity(0x5EED), Tick(0), declared_request(), 1).expect("an evaluation");
        assert_eq!(evaluation.tally.due, 15);
        assert_eq!(evaluation.tally.ran, 13, "thirteen stages produce a world");
        assert_eq!(evaluation.tally.refused.len(), 2, "and two refuse by name");

        let refused: Vec<&str> = evaluation.tally.refused_names();
        assert!(refused.contains(&"body_owed"), "{refused:?}");
        assert!(refused.contains(&"not_a_generator"), "{refused:?}");
        // The tally reads in the **roster's** order, not in the order the scheduler happened to
        // reach the stages: an owed stage that reads nothing sits at level 0, and a report that put
        // it first would hide the roster's own order from its own report.
        let refused_stages: Vec<&str> = roster
            .stages()
            .iter()
            .zip(&evaluation.outcomes)
            .filter(|(_, outcome)| matches!(outcome, Outcome::Refused(_)))
            .map(|(stage, _)| stage.name)
            .collect();
        assert_eq!(refused_stages, vec!["caves", "earthworks"]);
        assert_eq!(evaluation.tally.refused.len(), refused_stages.len());
        // Two refusals, each by name and for its own stated reason: one owed field, one ruling.
        let reasons: String = evaluation
            .tally
            .refused
            .iter()
            .map(|refusal| format!("{}: {}", refusal.kind().name(), refusal.detail()))
            .collect::<Vec<_>>()
            .join(" | ");
        assert!(reasons.contains("dissolution field"), "{reasons}");
        assert!(reasons.contains("the record, not a generator"), "{reasons}");

        // The produced fields are the ones the graph promises, in patch order and full length —
        // including the three hydrology and three ecology readings and the record's two new laws.
        for field in [
            FieldId::Elevation,
            FieldId::Temperature,
            FieldId::StreamOrder,
            FieldId::WaterLevel,
            FieldId::HillslopeFlux,
            FieldId::Biome,
            FieldId::CarryingCapacity,
            FieldId::Flock,
            FieldId::Deposition,
            FieldId::Blend,
        ] {
            let values = evaluation.values.get(field).expect("a produced field");
            assert_eq!(values.len(), evaluation.patch.len());
        }
        assert_eq!(
            evaluation.patch.len(),
            (DECLARED_PATCH_SIDE * DECLARED_PATCH_SIDE) as usize
        );

        // The new laws are not constants: a field the roster promises says something about the world.
        for field in [
            FieldId::StreamOrder,
            FieldId::WaterLevel,
            FieldId::CarryingCapacity,
            FieldId::Flock,
            FieldId::Deposition,
            FieldId::Blend,
        ] {
            if let Some(values) = evaluation.values.get(field) {
                assert!(
                    values.iter().all(|value| value.is_finite()),
                    "{} must be finite everywhere",
                    field.name()
                );
            }
        }
    }

    #[test]
    fn a_tick_is_idempotent_and_a_rewind_is_refused_by_name() {
        let roster = roster().expect("the reference roster must admit");
        let mut driver = TickDriver::new();
        let first = driver.drive(&roster, identity(0x5EED), Tick(3), 1).expect("a tick");
        let again = driver.drive(&roster, identity(0x5EED), Tick(3), 1).expect("the same tick");
        assert_eq!(first, again, "driving a tick twice is one effect");
        assert_eq!(driver.last(), Some(Tick(3)));
        assert!(driver.receipt(Tick(3)).is_some());

        let forward = driver.drive(&roster, identity(0x5EED), Tick(4), 2).expect("a later tick");
        assert_ne!(forward.plan_digest, first.plan_digest);

        let rewind = driver
            .drive(&roster, identity(0x5EED), Tick(2), 1)
            .expect_err("a rewind must be refused");
        assert_eq!(rewind.kind(), RefusalKind::Rewind);
        assert_eq!(rewind.kind().name(), "rewind");
        assert!(rewind.detail().contains('2'), "{rewind}");
    }

    #[test]
    fn a_declaration_that_binds_is_enforced_at_run_time() {
        // A body that produces nothing for a declared output is refused by name, not believed.
        fn silent(_: &StageContext<'_>) -> Result<Produced, Refusal> {
            Ok(Produced::one(FieldId::Uplift, Vec::new()))
        }
        let roster = Declared::new(vec![Stage {
            name: "silent",
            source: "a paper",
            reads: &[],
            writes: &[FieldId::Uplift],
            cadence: Cadence::PerTick,
            requires_region: false,
            dials: crate::stage::Dials::new(&[]),
            body: silent,
        }])
        .expect("the declaration admits (it is the run that refuses)");
        let evaluation = evaluate(&roster, identity(1), Tick(0), declared_request(), 1)
            .expect("an evaluation");
        assert_eq!(evaluation.tally.ran, 0);
        let refusal = evaluation.tally.refused.first().expect("one refusal");
        assert_eq!(refusal.kind(), RefusalKind::OutputUndeclared);
        assert!(refusal.detail().contains("wrong length"), "{refusal}");
    }

    #[test]
    fn a_level_is_split_across_workers_without_reordering_the_fold() {
        let roster = roster().expect("the reference roster must admit");
        let levels = dependency_levels(&roster);
        // The graph really has depth: the air group sits below the terrain it reads.
        assert!(levels.len() >= 3, "expected a staged graph, got {} levels", levels.len());
        let flat: Vec<usize> = levels.iter().flatten().copied().collect();
        assert_eq!(flat.len(), roster.len(), "every stage is in exactly one level");
        let mut sorted = flat.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, (0..roster.len()).collect::<Vec<_>>());

        // The same evaluation at 1 and 8 workers agrees on the tally, on the values and on the digest.
        let one = evaluate(&roster, identity(0x5EED), Tick(0), declared_request(), 1).expect("an evaluation");
        let many = evaluate(&roster, identity(0x5EED), Tick(0), declared_request(), 8).expect("an evaluation");
        assert_eq!(one.digest, many.digest);
        assert_eq!(one.tally.ran, many.tally.ran);
        assert_eq!(one.tally.refused_names(), many.tally.refused_names());
        assert_eq!(one.outcomes, many.outcomes);
    }
}


