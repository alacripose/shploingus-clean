//! **The evidence — one function per built system, each measuring its own acceptance bar.**
//!
//! Derived from `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §4 (the spec-kit structure) and Constitution P4
//! (evidence over assertion): running the binary is the cheapest proof that the new project stands on
//! its own. What each function prints is a **measurement**, not a claim — every number is read back
//! out of the system it belongs to, and a bar that is not met makes the function return `false` so
//! the binary can exit non-zero.
//!
//! It lives beside the registry it measures ([`crate::systems`]) rather than inside the binary, so a
//! system's bar is measured the same way whoever calls it, and the binary stays a thin consumer with
//! two ways in: print the evidence, or serve a session. Nothing here is imported from the legacy tree;
//! the crate's only dependency is `sha2`, for content identity.

use crate::audio::{ONSET_CELL_M, ONSET_CELL_TICKS, ONSET_DUTY, RAIN_GAIN};
use crate::automaton::{
    attribute_routes, Automaton, PIT_KEEP_PERCENT, RAIN_UM_PER_TICK, SETTLE_TICKS,
};
use crate::being;
use crate::axis::{
    self, refuse_bare, refuse_interchange, refuse_reduction, AxisKind, CoverageRow, EvidenceGrade,
    Ladder as AxisLadder, Origin, Reduction, COVERAGE,
};
use crate::bridge::{Answer, CheckOutcome, DocumentFacts, Owners, Receipt, TOOLS, TOOL_COUNT};
use crate::chain::{Chain, EventKind, Namespace, TicketId};
use crate::coordinates::{ChunkKey, SubvoxelIndex, VoxelPos, CHUNK_SIDE};
use crate::digest::Digest;
use crate::feeling::{Family, Feeling, RADIUS_PRESENT, RADIUS_RIM, RADIUS_STRONG};
use crate::chunk::{CubicWorld, CHUNK_CELLS};
use crate::field::{field_at, fields_at};
use crate::light::{
    night_sky_radiance, to_lux, to_moon_lux, twilight_knee_deg, twilight_lux, Droplets, Solver,
    Spectra, State, CHANNEL_COUNT, FOG_THRESHOLD_LWC_G_M3, NIGHT_SKY_ILLUMINANCE_LX,
    SOLAR_CONSTANT_W_M2, TWILIGHT_LUX_AT_HORIZON, VISIBLE_HI_NM, VISIBLE_LO_NM,
};
use crate::measure::{Band, Crossing, GaugeWall, Sensor, Site, WeighingGate};
use crate::material::{MaterialId, MaterialRegistry};
use crate::mesh::{
    check_horizons, chunk_centre_distance_m, recount_faces, ring_columns, Axis, Budget, Effort,
    FieldHorizon, Footprint, Ladder, Layer, Level, Mesher, RenderHorizon, ResidencyHorizon, Resident,
    SimulationHorizon, WorkLedger, MESH_RING,
};
use crate::presentation::{band_count, sweep_fixture, BAR_MINIMUM_BANDS, BAND_RESOLUTION};
use crate::process::{self, Outcome};
use crate::profile::{Profile, UP};
use crate::reference;
use crate::refusal::RefusalKind;
use crate::shell;
use crate::intake::Source;
use crate::policy;
use crate::save::{Ruleset, Save};
use crate::shaper::{apply, CellChange, ShaperKind, ShaperSet, StoredLattice};
use crate::stage::{Declared, FieldId, Patch, ProcessRequest};
use crate::stream::Streamer;
use crate::surface;
use crate::laya::{self, Ballot, Judgement, Measurement, Question, StructuralJudge, Verdict};
use crate::transport;
use crate::von::{self, Classifier, STAGES};
use crate::world::{DeclaredPeriod, Domain, Tick, World, WorldIdentity};

/// U1 — the FMA bar: at least three distinct luminance bands on a shaded surface.
pub fn check_u1() -> bool {
    let samples = match sweep_fixture() {
        Ok(samples) => samples,
        Err(refusal) => {
            eprintln!("U1 fixture refused by name: {}", refusal.name());
            return false;
        }
    };
    let bands = band_count(&samples);
    println!(
        "U1 bar: {bands} distinct luminance bands over {} samples (>= {BAR_MINIMUM_BANDS} required, \
         resolution {BAND_RESOLUTION})",
        samples.len()
    );
    bands >= BAR_MINIMUM_BANDS
}

/// **U1's visible client (2026-10-13, §36)** — the frame, drawn from the world the tree already builds, at
/// the plane's own rest. What can be measured without a window is measured here: the frame's own size and
/// column count, the **FMA bar read on that frame's own samples**, its digest and the world's. The window
/// itself is the **eye's** check (`LEDGER.md` C-40), and the binary's `window` mode is where it is made:
/// it presents the frame, reads the window's own pixels back, and prints what it presented.
///
/// **Which frame, said out loud (2026-10-13, §41).** Since §41 the window presents the **drawn reading** —
/// the mesh's own rectangles under one camera with depth ([`crate::drawn`], whose subject `surface::region_mesh`
/// derives) — while the frame measured **here** is the **plan reading** ([`surface::Surface::render`]: one
/// `shade` call per column, no depth). Both are kept, each with one owner and each **named on the line it is
/// printed on**, because a run that prints two frame digests and labels one is a run whose reader cannot tell
/// which reading moved.
pub fn check_u1_client() -> bool {
    let measured = match surface::measured_frame() {
        Ok(measured) => measured,
        Err(refusal) => {
            eprintln!("U1 client refused by name: {refusal}");
            return false;
        }
    };
    let deepest = match measured.deepest {
        Some((cell, depth)) => format!("{depth} µm at column {cell}"),
        None => "no column stands water".to_string(),
    };
    println!(
        "U1 client: the **plan reading** — kept and named, and the evidence run's own frame (the window \
         presents the drawn reading, §41): the world the being reads, at the plane's own rest ({} tick(s)) — \
         {} of {} column(s) stand water, deepest {deepest} — draws {} x {} pixel(s) from {} shaded column(s) \
         through the contract's own one shade call",
        measured.ticks,
        measured.standing,
        measured.columns,
        measured.width,
        measured.height,
        measured.columns
    );
    println!(
        "U1 client: the plan reading's own frame: its {} x {} pixel(s) measure {} distinct luminance band(s) (>= {BAR_MINIMUM_BANDS} \
         required, resolution {BAND_RESOLUTION}); frame digest {}, surface read digest {}, plane state \
         digest {}; channels 0x{:02x}..0x{:02x}",
        measured.width,
        measured.height,
        measured.bands,
        measured.frame.to_hex(),
        measured.world.to_hex(),
        measured.state.to_hex(),
        measured.dark,
        measured.bright
    );
    println!(
        "U1 client: the window is the eye's own check (C-40): `cargo run --offline -- window` opens one, \
         presents {} frame(s) — one tick of the plane's own law each — at {} frame(s)/s, and there draws the \
         **drawn reading** (the mesh's own rectangles under one camera with depth, §41) where this plan \
         reading is the run's own frame; it reads the window's own pixels back, prints what it presented, and \
         exits",
        surface::DECLARED_FRAMES,
        surface::FRAMES_PER_SECOND
    );
    measured.bands >= BAR_MINIMUM_BANDS && measured.dark < measured.bright && measured.columns > 0
}

/// U2 — world identity and the cache: determinism measured by digest.
pub fn check_u2_world() -> bool {
    let period = DeclaredPeriod::seconds(1).expect("a one-second declared period is not zero");
    let identity = WorldIdentity::new(0x5EED_2468, period);
    let world = World::new(identity);

    let truth = world.truth_digest();
    let truth_again = World::new(identity).truth_digest();
    let mut viewed = World::new(identity);
    viewed.set_anchor(ChunkKey::new(-7, 3, 12));
    for x in -3..3 {
        viewed.reside(ChunkKey::new(x, 0, 0));
    }
    let truth_viewed = viewed.truth_digest();
    let truth_ticked = world.with_tick(Tick(1)).truth_digest();

    let keys = [ChunkKey::new(1, 0, 0), ChunkKey::new(-1, 2, 0), ChunkKey::new(0, 0, 5)];
    let mut forward = World::new(identity);
    for key in keys {
        forward.reside(key);
    }
    let mut backward = World::new(identity);
    for key in keys.iter().rev() {
        backward.reside(*key);
    }
    let plan: Vec<ChunkKey> = (0..64).map(|i| ChunkKey::new(i, -(i % 7), i / 4)).collect();

    let probe = VoxelPos::new(-33, 5, -1);
    let round_trip = probe
        .chunk()
        .voxel(probe.within_chunk())
        .map(|back| back == probe)
        .unwrap_or(false);
    let registry = MaterialRegistry::starter();
    let unadmitted_refused = registry
        .material(MaterialId(9999))
        .err()
        .map(|r| r.kind() == RefusalKind::UnadmittedMaterial)
        .unwrap_or(false);

    println!("U2 world identity: seed={} period={}s tick={}", identity.seed, period.get(), world.tick().0);
    println!("U2 truth digest:       {} (anchor+cache equal: {})", truth.to_hex(), truth_viewed == truth);
    println!("U2 truth (tick 1):     {} (differs: {})", truth_ticked.to_hex(), truth_ticked != truth);
    println!("U2 residency order-free: {} / {}", forward.residency_digest().to_hex(), backward.residency_digest().to_hex());
    println!("U2 plan order-free:    {} / {}", world.plan_digest(plan.iter().copied()).to_hex(), world.plan_digest(plan.iter().rev().copied()).to_hex());
    println!("U2 materials:          {} admitted (digest {})", registry.len(), registry.digest().to_hex());
    println!("U2 coordinate probe:   recompose ok = {round_trip}, chunks reached = {}", probe.chunks_reached_by().map(|c| c.len()).unwrap_or(0));
    let sample_a = world.stage_seed(Domain::Terrain, VoxelPos::new(3, 4, 5));
    let sample_b = world.stage_seed(Domain::Terrain, VoxelPos::new(3, 4, 5));
    println!("U2 sampled terrain seed: {sample_a} (pure: {})", sample_a == sample_b);

    // §37's slice runs **before** the bar is composed, so a failing earlier conjunct cannot short-circuit it
    // out of the run: the cubic world's own lines are printed, and the bar reads the slice's own answer.
    let cubic_ok = cubic_cells();

    truth == truth_again
        && truth_viewed == truth
        && truth_ticked != truth
        && forward.residency_digest() == backward.residency_digest()
        && world.plan_digest(plan.iter().copied()) == world.plan_digest(plan.iter().rev().copied())
        && round_trip
        && unadmitted_refused
        && sample_a == sample_b
        && cubic_ok
}

/// **§37's slice and §38's move, measured on the run's own world**: the cubic world's cells and their one
/// identity, the reading the column world survives as, and the water law **stepping the cells themselves**
/// — one state, one tick, no column plane beside them — with the route difference §37 left open **measured
/// and attributed** rather than asserted.
///
/// Everything here is compared against the **same** declared fields the rest of the run reads, so the two
/// readings describe one world: the generator-fed read is the field path §34–§37 used, and the cubic read
/// is the cells'. The depths must agree at every column (the water is carried in whole micrometres by
/// both), the surface may move by less than one voxel (that is the quantisation the cubic world is made
/// of), and **why** the two routing graphs differ is decomposed per column — a defect in the reading would
/// show up as an unexplained column, and the bar allows none.
///
/// The law's account itself lives beside the law ([`crate::automaton`]), where the decomposition, its
/// vocabulary and its guards are; this function is the run's **printer** for it, which is the departure
/// from this file's habit of carrying each slice's prose and is named rather than silent.
fn cubic_cells() -> bool {
    let roster = process::roster().expect("the declared roster must admit");
    let identity = reference::identity().expect("the reference identity");
    let request = process::declared_request();
    let patch = request.patch().expect("the declared request is a region");
    let ladder = reference::ladder().expect("the reference world's own ladder");
    let values = fields_at(
        &roster,
        identity,
        Tick(0),
        &[FieldId::Elevation, FieldId::WaterLevel, FieldId::Lithology],
        request,
        1,
    )
    .expect("the three declared fields answer in one evaluation");

    // The field path, exactly as §34–§36 read it — the comparison's own raw field, no longer a plane the
    // being is driven over.
    let column_read = being::WorldRead::of(
        being::Organisation::first(),
        "the first mob",
        &roster,
        identity,
        Tick(0),
        request,
        1,
    )
    .expect("the generator's own world read");

    // The cubic world, its one identity, and the reading of it.
    let cubic = CubicWorld::from_surface(patch, &values[0], &values[1], &values[2], &ladder)
        .expect("the declared surface fills a cubic world");
    let again = CubicWorld::from_surface(patch, &values[0], &values[1], &values[2], &ladder)
        .expect("the same surface fills the same world");
    let cubic_read = being::WorldRead::over_cubic(
        being::Organisation::first(),
        "the first mob",
        &cubic,
    )
    .expect("the cells are readable");

    let columns = cubic.columns().expect("every column of the patch reads");
    // The water is compared **in whole micrometres on both sides** — the unit §35 made the authority — so the
    // reading's own float conversion (`depth_um`, which truncates) is not what is being measured here.
    let exact = (0..column_read.cells())
        .filter(|cell| column_read.depth_um(*cell) == columns[*cell].water_um)
        .count();
    // And the *reading*'s own answer, through the read's float field and its micrometre conversion: the one
    // place a single micrometre can be lost (§34's own truncation, measured rather than assumed).
    let read_exact = (0..column_read.cells())
        .filter(|cell| cubic_read.depth_um(*cell) == columns[*cell].water_um)
        .count();
    let read_worst = (0..column_read.cells())
        .map(|cell| (cubic_read.depth_um(cell) - columns[cell].water_um).abs())
        .max()
        .unwrap_or(0);
    let worst_quantisation = (0..column_read.cells())
        .map(|cell| (cubic_read.elevation[cell] - column_read.elevation[cell]).abs())
        .fold(0.0_f64, f64::max);

    // **§38's move: the law steps the cells themselves.** One state (the cubic world's own cells), one tick
    // (the law's synchronous pass, with the graph read from those same cells), and no column plane beside
    // them: the state's identity is the world's own, and two runs of it are one identity.
    let cell_plane = match settle(&cubic) {
        Ok(plane) => plane,
        Err(refusal) => {
            eprintln!("U2 cubic step: the law's own pour is refused by name: {refusal}");
            return false;
        }
    };
    let second = match settle(&cubic) {
        Ok(plane) => plane,
        Err(refusal) => {
            eprintln!("U2 cubic step: the law's own pour is refused by name: {refusal}");
            return false;
        }
    };
    let one_state = second.digest() == cell_plane.digest();

    // The reading of the world the law actually stepped — `over_cubic` of the law's own state — must answer
    // the cells' water at every column, which is what makes it the *stepped* read rather than a base one.
    let stepped_read = being::WorldRead::over_cubic(
        being::Organisation::first(),
        "the first mob",
        cell_plane.world(),
    )
    .expect("the stepped cells are readable");
    let stepped_answers_the_cells = (0..patch.len())
        .filter(|cell| stepped_read.depth_um(*cell) == cell_plane.depth_um(*cell))
        .count();

    // **§37's open question, per column**: the generator's own float surface against the cells' — the one
    // drawing law on each (`d8_receivers`), every disagreement examined and counted under its cause. The
    // direct count beside it is the same figure computed a second way, so the decomposition cannot quietly
    // lose a column.
    let cell_surfaces: Vec<f64> = cubic
        .water_columns()
        .iter()
        .map(|column| column.surface_z as f64)
        .collect();
    let attribution = attribute_routes(patch, &column_read.elevation, &cell_surfaces);
    let receivers_differ = (0..column_read.cells())
        .filter(|cell| column_read.receivers[*cell] != cubic_read.receivers[*cell])
        .count();

    // The declared place: the deepest column the settled **cell** plane names, read both ways.
    let (deepest_cell, deepest_um) = cell_plane
        .deepest()
        .expect("the settled plane stands water somewhere");
    let side = patch.side() as usize;
    let origin = patch.origin();
    let (px, py) = (
        origin.x + (deepest_cell % side) as i32,
        origin.y + (deepest_cell / side) as i32,
    );
    let there = columns
        .get(deepest_cell)
        .copied()
        .expect("the deepest column is one of the patch's");
    // Both sides in metres: the reading's own micrometres against the generator's own float surface.
    let there_surface_m = there.surface_um as f64 / 1_000_000.0;
    let delta = (there_surface_m - column_read.elevation[deepest_cell]).abs();
    // The water **at that column, in the state the cubic world was built from** — the settled law's own
    // depth there is a later state, which the step line below prints; comparing against it would be
    // comparing one world's Tick(0) with another world's rest.
    let column_tick0_um = column_read.depth_um(deepest_cell);

    let digest = cubic.digest();
    println!(
        "U2 cubic cells:       {} chunk(s) × {CHUNK_CELLS} cell(s) = {} cell(s) hashed over the declared \
         patch's {} column(s); {} of them solid, air named rather than defaulted; the world's own identity \
         {}",
        cubic.chunks(),
        cubic.cells(),
        column_read.cells(),
        cubic.solid(),
        digest.to_hex(),
    );
    println!(
        "U2 surface read:      one reading of that patch, out of the cells: {cubic_digest}; out of the \
         generator's own fields: {column_digest} — the reading moved with the quantisation and is a reading \
         either way, never the world's identity",
        cubic_digest = being::surface_digest(&cubic_read).to_hex(),
        column_digest = being::surface_digest(&column_read).to_hex(),
    );
    println!(
        "U2 cubic reading:     at the settled cell plane's own deepest column (column {deepest_cell} → \
         ({px}, {py})): the generator's Tick(0) column reads surface {:.4} m and {column_tick0_um} µm of \
         water; the cells return surface {there_surface_m:.4} m (the voxel that contains the ground, Δ \
         {delta:.4} m < 1 m) and {} µm of water, over {} solid cell(s) (the cell plane settles to \
         {deepest_um} µm at that column, a later state); the water round-trips exactly at {exact} of {} \
         column(s) (in whole micrometres, both sides), and the reading answers the cells' own water at \
         {read_exact} of them, never by more than {read_worst} µm (the read's own µm conversion, §34); the \
         routing graph differs at {receivers_differ} column(s) — the surface the cells return is quantised \
         to the world's own voxel, and the generator's own float surface is not",
        column_read.elevation[deepest_cell],
        there.water_um,
        there.solid,
        column_read.cells(),
    );
    println!(
        "U2 route attribution: the cells' own graph against the generator's float surface, column by \
         column: {} of {} column(s) route differently — {} whose whole downhill drop is inside their own \
         voxel (the cells see a pit where the generator drained), {} whose generator winner is inside its \
         own voxel while the cells route to the steepest candidate they can see, {} tied at the steepest \
         quantised drop (the law's own scan order decides between equals); {} wrong way (the cells route \
         where the generator pitted — impossible while the voxel's floor is monotone) and {} unexplained \
         (a difference the voxel cannot account for, which is exactly what a defect in the reading would \
         look like) — the difference is the voxel, not the reading",
        attribution.differs(),
        attribution.columns,
        attribution.swallowed,
        attribution.inside_the_voxel,
        attribution.tied_at_the_top,
        attribution.wrong_way,
        attribution.unexplained,
    );
    println!(
        "U2 cubic step:        the law steps the cells themselves — {SETTLE_TICKS} tick(s) over {} \
         column(s), rain {RAIN_UM_PER_TICK} µm per tick and a pit keeping {PIT_KEEP_PERCENT}%: water stands \
         on {} of {} column(s) at rest, deepest {} µm at column {}, and the state's one identity (the \
         world's own) is {}; the poured state is a function of the cells and the count (two runs agree = \
         {one_state}), and the reading of that world answers the cells' own water at \
         {stepped_answers_the_cells} of {} column(s)",
        patch.len(),
        cell_plane.standing(),
        patch.len(),
        cell_plane.deepest().map(|(_, um)| um).unwrap_or(0),
        cell_plane.deepest().map(|(cell, _)| cell).unwrap_or(0),
        cell_plane.digest().to_hex(),
        patch.len(),
    );

    // The bar: the water is the same water column by column, the surface moves less than one voxel, the
    // world's identity is a deterministic function of the cells, the law steps the cells deterministically
    // and its state's identity is the world's own, **the reading of the stepped world is the state's own**
    // (one water, not two), and **every route difference is the voxel's** — the measurement §37 left owed,
    // with the direct count and the decomposition required to agree.
    exact == column_read.cells()
        && worst_quantisation < 1.0
        && delta < 1.0
        && digest == again.digest()
        && one_state
        && cell_plane.standing() > 0
        && stepped_answers_the_cells == patch.len()
        && attribution.is_the_voxel()
        && attribution.differs() == receivers_differ
}

/// Step one cubic world to the law's own rest — the run's two call sites share this, so "settled" means one
/// thing in this file, and a pour the world cannot carry is the law's own refusal passed through.
fn settle(world: &CubicWorld) -> Result<Automaton, crate::refusal::Refusal> {
    let mut plane = Automaton::over(world);
    plane.steps(SETTLE_TICKS)?;
    Ok(plane)
}

/// The **cells** one chunk's reading stands on: the same three declared fields the mesher's reading door
/// reads, filled by the one constructor it fills them with — the world the U4 recount is taken of, so the
/// recount is of the cells the derivation swept and not of a second world built beside them.
fn chunk_cube(
    roster: &Declared,
    world: &World,
    ladder: &Ladder,
    chunk: ChunkKey,
) -> Result<CubicWorld, crate::refusal::Refusal> {
    let origin = chunk.origin().map_err(|kind| {
        crate::refusal::Refusal::new(
            kind,
            format!("chunk ({}, {}, {}) has no expressible origin", chunk.x, chunk.y, chunk.z),
        )
    })?;
    let side = CHUNK_SIDE as u32;
    let values = fields_at(
        roster,
        world.identity(),
        world.tick(),
        &[FieldId::Elevation, FieldId::WaterLevel, FieldId::Lithology],
        ProcessRequest::Region { origin, side },
        1,
    )?;
    CubicWorld::from_surface(Patch::new(origin, side)?, &values[0], &values[1], &values[2], ladder)
}

/// The same, over a chunk's **halo**: the patch of `32 + 2·MESH_RING` columns square around `chunk`, and
/// the cells the declared fields imply for it. This is the world the reading's door builds for itself, and
/// it is built here from the same fields by the same constructor so the cells' door can be handed an
/// identical one — one world, two doors, which is the comparison the extent contract is measured by.
fn halo_cube(
    roster: &Declared,
    world: &World,
    ladder: &Ladder,
    chunk: ChunkKey,
) -> Result<CubicWorld, crate::refusal::Refusal> {
    let origin = chunk.origin().map_err(|kind| {
        crate::refusal::Refusal::new(
            kind,
            format!("chunk ({}, {}, {}) has no expressible origin", chunk.x, chunk.y, chunk.z),
        )
    })?;
    let side = CHUNK_SIDE as u32 + 2 * MESH_RING as u32;
    let values = fields_at(
        roster,
        world.identity(),
        world.tick(),
        &[FieldId::Elevation, FieldId::WaterLevel, FieldId::Lithology],
        ProcessRequest::Region {
            origin: VoxelPos::new(origin.x - MESH_RING, origin.y - MESH_RING, origin.z),
            side,
        },
        1,
    )?;
    CubicWorld::from_surface(
        Patch::new(
            VoxelPos::new(origin.x - MESH_RING, origin.y - MESH_RING, origin.z),
            side,
        )?,
        &values[0],
        &values[1],
        &values[2],
        ladder,
    )
}

/// The same, over a rectangle of `chunks` chunks starting at `origin` in x and one chunk in y: the world a
/// stitched region is meshed from.
fn region_cube(
    roster: &Declared,
    world: &World,
    ladder: &Ladder,
    origin: ChunkKey,
    chunks: u32,
) -> Result<CubicWorld, crate::refusal::Refusal> {
    let base = origin.origin().map_err(|kind| {
        crate::refusal::Refusal::new(
            kind,
            format!("chunk ({}, {}, {}) has no expressible origin", origin.x, origin.y, origin.z),
        )
    })?;
    let side = CHUNK_SIDE as u32 * chunks;
    let values = fields_at(
        roster,
        world.identity(),
        world.tick(),
        &[FieldId::Elevation, FieldId::WaterLevel, FieldId::Lithology],
        ProcessRequest::Region { origin: base, side },
        1,
    )?;
    CubicWorld::from_surface(Patch::new(base, side)?, &values[0], &values[1], &values[2], ladder)
}

/// **The laws the reference world must actually show**, measured on the generated fields rather than
/// asserted about them: a drainage network that branches, water standing in a basin, a delta that
/// deposits there, a blend that crosses between two domains, a province that carries life, a flock
/// with a resultant — and a hillslope half that conserves exactly over the closed region.
pub fn check_u3_laws(evaluation: &process::Evaluation) -> bool {
    let field = |id: FieldId| evaluation.values.get(id).unwrap_or(&[]);
    let elevation = field(FieldId::Elevation);
    let order = field(FieldId::StreamOrder);
    let water = field(FieldId::WaterLevel);
    let deposition = field(FieldId::Deposition);
    let blend = field(FieldId::Blend);
    let capacity = field(FieldId::CarryingCapacity);
    let flock = field(FieldId::Flock);
    let flux = field(FieldId::HillslopeFlux);

    let spread = |values: &[f64]| {
        let low = values.iter().copied().fold(f64::INFINITY, f64::min);
        let high = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        (low, high)
    };
    let (order_low, order_high) = spread(order);
    let submerged: Vec<f64> = elevation
        .iter()
        .zip(water)
        .map(|(ground, level)| level - ground)
        .collect();
    let flooded = submerged.iter().filter(|depth| **depth > 0.0).count();
    let deepest = submerged.iter().copied().fold(0.0_f64, f64::max);
    let deposited: f64 = deposition.iter().sum();
    let depositing = deposition.iter().filter(|depth| **depth > 0.0).count();
    let (blend_low, blend_high) = spread(blend);
    let (_, capacity_high) = spread(capacity);
    let (_, flock_high) = spread(flock);
    let flux_total: f64 = flux.iter().sum();
    let flux_magnitude: f64 = flux.iter().map(|q| q.abs()).sum();

    println!(
        "U3 law readings: stream order {order_low:.0}-{order_high:.0}; {flooded} columns hold water \
         (deepest {deepest:.3} m); deposition {deposited:.4} m over {depositing} columns; blend \
         {blend_low:.3}-{blend_high:.3}; capacity up to {capacity_high:.1} g m^-2 yr^-1; flock up to \
         {flock_high:.3}"
    );
    println!(
        "U3 conservation: hillslope flux sums to {flux_total:.3e} over a region of magnitude \
         {flux_magnitude:.3e} (cancels: {})",
        flux_total.abs() <= 1e-9 * flux_magnitude.max(1.0)
    );

    // Every reading is a fact about this world, and each one is a bar: a network with a junction, a
    // basin that holds water, a delta that fills it, a blend that crosses, a living province, and a
    // closed region whose hillslope exchanges cancel.
    order_low >= 1.0
        && order_high >= 2.0
        && flooded >= 1
        && deepest > 0.0
        && deposited > 0.0
        && depositing >= 1
        && blend_high - blend_low > 0.5
        && capacity_high > 0.0
        && flock_high > 0.0
        && flux_total.abs() <= 1e-9 * flux_magnitude.max(1.0)
        && flux_magnitude > 0.0
}

/// How many of the fifteen declared stages produce a world: every one but the two deliberate
/// refusals — `caves` (a field that is not declared) and `earthworks` (the record, not a generator).
const LANDED_STAGES: usize = 13;

/// The refusals that are **decisions**: a stage that stays refused by name, for its own stated
/// reason, rather than a body that returned a plausible zero.
const DELIBERATE_REFUSALS: usize = 2;

/// U3 — the declared roster, the shaper stages, and the worker knob.
///
/// The evidence `specs/003-u3-shapers/spec.md` asks for, **printed** rather than asserted in private:
/// the declaration itself, one digest at 1, 2, 4 and 8 workers (SC-001), the tally of what ran and
/// what refused *by name*, a later tick moving the digest (SC-002), a refused rewind (SC-006), and
/// the single `field_at` seam (FR-007). It also prints what this slice still owes, because an owed
/// stage is a fact about the build and not something to discover from a missing line.
pub fn check_u3() -> bool {
    let roster = match process::roster() {
        Ok(roster) => roster,
        Err(refusals) => {
            eprintln!("U3 roster refused by name: {refusals}");
            return false;
        }
    };
    let period = DeclaredPeriod::seconds(1).expect("a one-second declared period is not zero");
    let identity = WorldIdentity::new(0x5EED_BEEF, period);
    let tick = Tick(0);
    let request = process::declared_request();

    println!(
        "U3 declared roster: {} stages, declaration digest {}",
        roster.len(),
        roster.digest().to_hex()
    );
    for stage in roster.stages() {
        println!(
            "U3   {:<11} rung {} ({:<8}) reads {:?} writes {:?}",
            stage.name,
            stage.cadence.rung(),
            stage.cadence.name(),
            stage.reads.iter().map(|field| field.name()).collect::<Vec<_>>(),
            stage.writes.iter().map(|field| field.name()).collect::<Vec<_>>()
        );
    }

    // SC-001: the worker count is a pure performance knob, so the digest is one number.
    let mut digests: Vec<String> = Vec::new();
    for workers in [1_usize, 2, 4, 8] {
        match process::world_generation_digest_with_workers(&roster, identity, tick, request, workers)
        {
            Ok(digest) => {
                println!("U3 digest at {workers} worker(s): {}", digest.to_hex());
                digests.push(digest.to_hex());
            }
            Err(refusal) => {
                eprintln!("U3 digest at {workers} worker(s) refused: {refusal}");
                return false;
            }
        }
    }
    let worker_independent = digests.windows(2).all(|pair| pair[0] == pair[1]);

    let evaluation = match process::evaluate(&roster, identity, tick, request, 1) {
        Ok(evaluation) => evaluation,
        Err(refusal) => {
            eprintln!("U3 evaluation refused: {refusal}");
            return false;
        }
    };
    println!(
        "U3 tally at tick {}: {}/{} stages ran, {} refused by name: {:?}",
        tick.0,
        evaluation.tally.ran,
        evaluation.tally.due,
        evaluation.tally.refused.len(),
        evaluation.tally.refused_names()
    );
    let mut refusals: Vec<(&str, RefusalKind)> = Vec::new();
    for (stage, outcome) in roster.stages().iter().zip(&evaluation.outcomes) {
        match outcome {
            Outcome::Produced(produced) => println!(
                "U3   ran      {:<11} -> {:?}",
                stage.name,
                produced.fields().iter().map(|(field, _)| field.name()).collect::<Vec<_>>()
            ),
            Outcome::Refused(refusal) => {
                println!(
                    "U3   refused  {:<11} [{}] {}",
                    stage.name,
                    refusal.kind().name(),
                    refusal.detail()
                );
                refusals.push((stage.name, refusal.kind()));
            }
        }
    }
    let refused_by = |name: &str, kind: RefusalKind| {
        refusals.iter().any(|(stage, k)| *stage == name && *k == kind)
    };
    // The two refusals that are decisions rather than debt: one owed field, one ruling. Every other
    // stage of the fifteen must have produced its declared outputs.
    let caves_owed = refused_by("caves", RefusalKind::BodyOwed);
    let earthworks_by_design = refused_by("earthworks", RefusalKind::NotAGenerator);
    println!(
        "U3 deliberate refusals: caves owed (the dissolution field) = {caves_owed}; earthworks \
         bodyless by design (the record) = {earthworks_by_design}"
    );

    // SC-002 and C2: a later tick is a different world, a tick driven twice is one effect, and a
    // rewind is refused by name.
    let later = process::world_generation_digest(&roster, identity, Tick(1))
        .map(|digest| digest.to_hex())
        .unwrap_or_default();
    let tick_moves_the_world = later != digests[0];
    let mut driver = process::TickDriver::new();
    let once = driver.drive(&roster, identity, Tick(3), 2);
    let twice = driver.drive(&roster, identity, Tick(3), 4);
    let idempotent = matches!((&once, &twice), (Ok(a), Ok(b)) if a == b);
    let rewind = driver
        .drive(&roster, identity, Tick(2), 1)
        .expect_err("a rewind must be refused");
    let rewind_named = rewind.kind() == RefusalKind::Rewind;
    println!("U3 tick 1 digest: {later} (moves the world: {tick_moves_the_world})");
    println!(
        "U3 tick 3 driven twice is one receipt: {idempotent}; rewind refused by name = \
         {rewind_named} ({rewind})"
    );

    // FR-007: one seam, the value comes from the stage that declares it, and a region-only law asked
    // at a lone site is refused by name rather than answered.
    let elevation = field_at(&roster, identity, tick, FieldId::Elevation, request, 2);
    let seam_reads = elevation
        .as_ref()
        .map(|values| values.len() == evaluation.patch.len())
        .unwrap_or(false);
    let region_only = field_at(
        &roster,
        identity,
        tick,
        FieldId::Discharge,
        ProcessRequest::Unit { at: VoxelPos::new(0, 0, 0) },
        1,
    )
    .expect_err("a region-only law asked at a lone site is refused");
    let region_named = region_only.kind() == RefusalKind::RegionRequired;
    println!(
        "U3 field_at: elevation from {}, {} columns = {seam_reads}; region-only discharge at a lone \
         site refused by name = {region_named} ({region_only})",
        roster
            .producer_of(FieldId::Elevation)
            .map(|stage| stage.name)
            .unwrap_or("nobody"),
        elevation.as_ref().map(Vec::len).unwrap_or(0)
    );

    let landed = evaluation.tally.ran;
    println!(
        "U3 owed after this slice: caves (the dissolution field it reads) and `delta`'s avulsion \
         state — the two refusals named above are decisions, not debt — {landed} of {} stages \
         landed",
        roster.len()
    );

    worker_independent
        && landed == LANDED_STAGES
        && tick_moves_the_world
        && idempotent
        && rewind_named
        && evaluation.tally.refused.len() == DELIBERATE_REFUSALS
        && caves_owed
        && earthworks_by_design
        && check_u3_laws(&evaluation)
        && seam_reads
        && region_named
}

/// U4 — the geometry a world's fields imply, kept coherent with the world it came from, with its work
/// counted where it happens.
///
/// The evidence `specs/004-u4-mesh-stream-work/spec.md` asks for, **printed** rather than asserted in
/// private, and every line names the world it is of — seed, declared period, tick, chunk and level
/// (C-46): one digest per world at 1/2/4/8 workers (SC-001), a hit that does no work and a rebuild
/// whose digest moved, an invalidation that removes exactly the stale entries (SC-002), the merge
/// factor measured on the reference world (SC-003), the ledger against an independent recount of the
/// returned mesh (SC-004), and the two refusals with their numbers (SC-005).
pub fn check_u4() -> bool {
    let roster = match process::roster() {
        Ok(roster) => roster,
        Err(refusals) => {
            eprintln!("U4 roster refused by name: {refusals}");
            return false;
        }
    };
    let registry = MaterialRegistry::starter();
    // The declared palette: the starter registry's admitted ground materials, laid over the world's
    // six declared lithology classes, with water for the surface layer. A palette thinner than the
    // class count is a declaration, and it is printed rather than implied.
    let palette = [MaterialId(1), MaterialId(2), MaterialId(3), MaterialId(4), MaterialId(6)];
    let ladder = match Ladder::over_palette(&registry, &palette, 6, MaterialId(5)) {
        Ok(ladder) => ladder,
        Err(refusal) => {
            eprintln!("U4 ladder refused by name: {refusal}");
            return false;
        }
    };

    // The four horizons, as four types; only the one derived rule between them can be a refusal.
    let render = RenderHorizon::m(128.0).expect("a positive render horizon");
    let simulation = SimulationHorizon::m(192.0).expect("a positive simulation horizon");
    // **The client's own declared horizon**, read from where it is declared rather than restated here
    // (2026-10-14e): it is the drawn region's own reach, so a literal here would let the stream gate and the
    // drawn path measure two different residencies under one name.
    let residency = ResidencyHorizon::m(surface::DRAWN_RESIDENCY_M)
        .expect("a positive residency horizon");
    let fields = FieldHorizon::m(1024.0).expect("a positive field horizon");
    let horizons_ordered = check_horizons(render, residency).is_ok()
        && render.radius_m() < simulation.radius_m()
        && simulation.radius_m() < residency.radius_m()
        && residency.radius_m() < fields.radius_m();
    let mixed = check_horizons(RenderHorizon::m(300.0).expect("positive"), residency)
        .expect_err("a render horizon wider than residency is refused");

    let period = DeclaredPeriod::seconds(1).expect("a one-second declared period is not zero");
    let identity = WorldIdentity::new(0x5EED_BEEF, period);
    let world = World::new(identity);
    let chunk = ChunkKey::new(0, 0, 0);
    let level = Level::new(0).expect("level 0 is on the render ladder");
    let mesher = Mesher::new(&roster, ladder.clone(), residency);

    println!(
        "U4 render ladder: {} levels (rectangles up to {} columns); palette {} classes over {} \
         admitted ground materials: {}; water = {}",
        Level::ALL.len(),
        Level::ALL.last().map(|level| level.tile()).unwrap_or(1),
        ladder.classes(),
        palette.len(),
        ladder
            .mapping()
            .iter()
            .map(|(class, id)| format!("{class}->{}", id.0))
            .collect::<Vec<_>>()
            .join(" "),
        ladder.water().0
    );
    println!(
        "U4 horizons (four types): render {:.0} m < simulation {:.0} m < residency {:.0} m < field \
         {:.0} m, ordered = {horizons_ordered}",
        render.radius_m(),
        simulation.radius_m(),
        residency.radius_m(),
        fields.radius_m()
    );

    let mut resident = Resident::new();
    let mut ledger = WorkLedger::default();
    let mesh = match mesher.derive(&world, chunk, level, Effort::new(Budget::unlimited(), 1), &mut resident, &mut ledger) {
        Ok(mesh) => mesh,
        Err(refusal) => {
            eprintln!("U4 derive refused by name: {refusal}");
            return false;
        }
    };
    // The extent this door meshed, **measured rather than asserted**: the world the reading builds for
    // itself is built here too, from the same fields by the same constructor, so the footprint printed is
    // the one the door swept. Its halo is owned by construction — that is what reading the ring buys.
    let halo_world = match halo_cube(&roster, &world, &ladder, chunk) {
        Ok(cube) => cube,
        Err(refusal) => {
            eprintln!("U4 reading: the chunk's halo refused by name: {refusal}");
            return false;
        }
    };
    let halo_footprint = match Footprint::of_chunk(&halo_world, chunk) {
        Ok(footprint) => footprint,
        Err(refusal) => {
            eprintln!("U4 reading: the chunk's footprint refused by name: {refusal}");
            return false;
        }
    };
    let halo_owned = halo_footprint.halo_owned(&halo_world);
    println!(
        "U4 reading seed={} period={}s tick={} chunk=({}, {}, {}) level={}: footprint {}×{} + ring {} \
         ({} column(s) in the halo, {} produced, {} samples read), span {}..{} ({} band(s), {} cells), \
         halo owned = {halo_owned}; {} cells swept → {} faces → {} merged away → {} quads ({} merged, \
         factor {:.2}), vertices {}, triangles {}, digest {}, keyed by the reading's own state {}",
        identity.seed,
        period.get(),
        world.tick().0,
        chunk.x, chunk.y, chunk.z,
        level.get(),
        halo_footprint.patch().side(),
        halo_footprint.patch().side(),
        MESH_RING,
        halo_footprint.halo_columns(),
        ledger.columns_produced,
        ledger.samples_read,
        halo_footprint.floor_z(),
        halo_footprint.ceiling_z(),
        (halo_footprint.ceiling_z() - halo_footprint.floor_z()) / CHUNK_SIDE,
        halo_footprint.cells(),
        ledger.cells,
        ledger.faces,
        ledger.merged_away,
        mesh.quads().len(),
        ledger.rectangles_merged,
        ledger.merge_factor(),
        mesh.vertices(),
        mesh.triangles(),
        mesh.digest().to_hex(),
        mesh.key().state.to_hex()
    );

    // **One world, two doors.** The audit measured 3 703 faces through this door and 3 047 through the
    // cells' for the same 11 054 swept cells, because the reading's world *was* the chunk: its own edge was
    // drawn as a cliff. The comparison is now made in the one world the reading really reads — the chunk
    // and its ring, built here from the same fields by the same constructor and handed to the cells' door —
    // and the two doors' quad lists are compared face by face rather than counted.
    let mut halo_resident = Resident::new();
    let mut halo_ledger = WorkLedger::default();
    let halo_cells = match mesher.derive_cubic(
        &halo_world,
        chunk,
        level,
        Effort::new(Budget::unlimited(), 1),
        &mut halo_resident,
        &mut halo_ledger,
    ) {
        Ok(mesh) => mesh,
        Err(refusal) => {
            eprintln!("U4 doors: the world the reading reads refused by name: {refusal}");
            return false;
        }
    };
    let doors_agree = halo_cells.quads() == mesh.quads()
        && halo_cells.area() == mesh.area()
        && halo_cells.vertices() == mesh.vertices()
        && halo_ledger.faces == ledger.faces
        && halo_ledger.cells == ledger.cells;

    // SC-004: the ledger against an **independent recount of the same cells** — the naive sweep beside the
    // mesher's own, so a miscount shows as two numbers rather than as one — and against the request's own
    // columns and the returned geometry.
    let recount = recount_faces(&halo_world, halo_footprint);
    let recount_agrees = halo_ledger.columns_swept == recount.columns
        && halo_ledger.cells == recount.solid_cells
        && halo_ledger.faces == recount.total_faces()
        && halo_ledger.merged_away == halo_ledger.faces - halo_ledger.quads
        && halo_ledger.quads == halo_cells.quads().len() as u64
        && halo_cells.area() == halo_ledger.faces
        && halo_ledger.vertices == halo_cells.vertices()
        && halo_ledger.indices == halo_cells.indices()
        && halo_ledger.columns_produced == 0
        && halo_ledger.samples_read == 0
        && halo_cells.vertices() == 4 * halo_cells.quads().len() as u64
        && halo_cells.indices() == 6 * halo_cells.quads().len() as u64;
    println!(
        "U4 face recount: ledger == an independent sweep of the same cells = {recount_agrees}: {} \
         column(s) swept, {} solid cell(s), {} solid + {} water faces, {} merged away into {} quads \
         carrying {} faces of area",
        recount.columns,
        recount.solid_cells,
        recount.solid_faces,
        recount.water_faces,
        halo_ledger.merged_away,
        halo_ledger.quads,
        halo_cells.area()
    );
    println!(
        "U4 one world, two doors: the same chunk of the world the reading reads through both doors = \
         {doors_agree}; the reading swept {} cell(s) in {} faces, the cells' door {} cell(s) in {} faces, and \
         their rectangles are {} ({} quads); only the name differs — the reading keys by the world's \
         reading {} , the cells by the world's own state {}",
        ledger.cells,
        ledger.faces,
        halo_ledger.cells,
        halo_ledger.faces,
        if halo_cells.quads() == mesh.quads() { "the same list, byte for byte" } else { "NOT the same" },
        mesh.quads().len(),
        mesh.key().state.to_hex(),
        halo_cells.key().state.to_hex()
    );

    // SC-001: one digest at 1, 2, 4 and 8 workers.
    let mut worker_digests = Vec::new();
    for workers in [1_usize, 2, 4, 8] {
        let mut fresh = Resident::new();
        let mut fresh_ledger = WorkLedger::default();
        match mesher.derive(&world, chunk, level, Effort::new(Budget::unlimited(), workers), &mut fresh, &mut fresh_ledger) {
            Ok(mesh) => worker_digests.push(mesh.digest()),
            Err(refusal) => {
                eprintln!("U4 derive at {workers} worker(s) refused: {refusal}");
                return false;
            }
        }
    }
    let worker_knob = worker_digests.windows(2).all(|pair| pair[0] == pair[1]);
    println!(
        "U4 worker knob: digest {} at 1/2/4/8 workers is one value = {worker_knob}",
        worker_digests.first().map(|digest| digest.to_hex()).unwrap_or_default()
    );

    // SC-002: a hit does no work; a moved world is rebuilt and never served stale; invalidation removes
    // exactly the entries of the world that is gone.
    let before = ledger;
    let hit = match mesher.derive(&world, chunk, level, Effort::new(Budget::unlimited(), 4), &mut resident, &mut ledger) {
        Ok(mesh) => mesh,
        Err(refusal) => {
            eprintln!("U4 re-ask refused: {refusal}");
            return false;
        }
    };
    let hit_is_free = ledger.hits == before.hits + 1
        && ledger.columns_produced == before.columns_produced
        && ledger.faces == before.faces
        && ledger.quads == before.quads
        && hit.digest() == mesh.digest();
    // What the hit cost: read **before** the world moves, so the rebuild's own columns and faces are
    // not folded into the hit's count.
    let hit_columns = ledger.columns_produced - before.columns_produced;
    let hit_faces = ledger.faces - before.faces;
    let later = world.with_tick(Tick(1));
    let stale_not_served = resident.serve(later.truth_digest(), chunk, level).is_none();
    let rebuilt = match mesher.derive(&later, chunk, level, Effort::new(Budget::unlimited(), 1), &mut resident, &mut ledger) {
        Ok(mesh) => mesh,
        Err(refusal) => {
            eprintln!("U4 derive at tick 1 refused: {refusal}");
            return false;
        }
    };
    let digest_moved = rebuilt.digest() != mesh.digest();
    let held_before = resident.len();
    let removed = resident.invalidate(later.truth_digest());
    let kept = resident.len();
    println!(
        "U4 seed={} cache: re-ask was a hit producing {hit_columns} more columns, {} more faces and 0 \
         more quads (free: {hit_is_free}); at tick {} the tick-0 entry is served = {} (never stale); \
         rebuild digest {} moved = {digest_moved}; invalidate removed {removed} of {held_before} and kept \
         {kept} current",
        identity.seed,
        hit_faces,
        later.tick().0,
        !stale_not_served,
        rebuilt.digest().to_hex()
    );

    // A coarse level: fewer samples over the same world, and a different identity.
    let coarse_level = Level::new(2).expect("level 2 is on the render ladder");
    let mut coarse_resident = Resident::new();
    let mut coarse_ledger = WorkLedger::default();
    let coarse = match mesher.derive(
        &world,
        chunk,
        coarse_level,
        Effort::new(Budget::unlimited(), 2),
        &mut coarse_resident,
        &mut coarse_ledger,
    ) {
        Ok(mesh) => mesh,
        Err(refusal) => {
            eprintln!("U4 level 2 refused: {refusal}");
            return false;
        }
    };
    let coarser = coarse_ledger.samples_read < coarse_ledger.columns_produced
        && coarse.digest() != mesh.digest()
        && coarse_ledger.columns_produced == ring_columns()
        && coarse.area() == coarse_ledger.faces;
    println!(
        "U4 seed={} tick={} chunk=({}, {}, {}) level={}: read {} of {} produced columns, {} cells swept \
         → {} faces → {} quads, digest {} (a level is part of the identity: {coarser})",
        identity.seed,
        world.tick().0,
        chunk.x, chunk.y, chunk.z,
        coarse_level.get(),
        coarse_ledger.samples_read,
        coarse_ledger.columns_produced,
        coarse_ledger.cells,
        coarse_ledger.faces,
        coarse.quads().len(),
        coarse.digest().to_hex()
    );

    // **The cells' own door.** The same mesher over a world that already holds cells: the declared patch's
    // own fields, filled by the one constructor, handed to the law — and the mesh keyed by the cells'
    // identity. The numbers are printed **before and after one tick of the law**, because the two questions
    // the record asks of them are different: the ground does not move, so its faces are the same set, and
    // the water does, so the key the resident set holds is a different one.
    let cells_request = process::declared_request();
    let cells_patch = match cells_request.patch() {
        Ok(patch) => patch,
        Err(refusal) => {
            eprintln!("U4 cells: the declared request is not a region: {refusal}");
            return false;
        }
    };
    let cells_fields = match fields_at(
        &roster,
        identity,
        Tick(0),
        &[FieldId::Elevation, FieldId::WaterLevel, FieldId::Lithology],
        cells_request,
        1,
    ) {
        Ok(values) => values,
        Err(refusal) => {
            eprintln!("U4 cells: the declared fields refused by name: {refusal}");
            return false;
        }
    };
    let cells_world = match CubicWorld::from_surface(
        cells_patch,
        &cells_fields[0],
        &cells_fields[1],
        &cells_fields[2],
        &ladder,
    ) {
        Ok(cube) => cube,
        Err(refusal) => {
            eprintln!("U4 cells: the declared surface refused by name: {refusal}");
            return false;
        }
    };
    let mut law = Automaton::over(&cells_world);
    let footprint = match Footprint::of_chunk(&cells_world, chunk) {
        Ok(footprint) => footprint,
        Err(refusal) => {
            eprintln!("U4 cells: the chunk's own footprint refused by name: {refusal}");
            return false;
        }
    };
    let cells_halo_owned = footprint.halo_owned(&cells_world);
    let mut cells_resident = Resident::new();
    let mut cells_ledger = WorkLedger::default();
    let cell_mesh_before = match mesher.derive_cubic(
        &cells_world,
        chunk,
        level,
        Effort::new(Budget::unlimited(), 1),
        &mut cells_resident,
        &mut cells_ledger,
    ) {
        Ok(mesh) => mesh,
        Err(refusal) => {
            eprintln!("U4 cells: the law's own world refused by name: {refusal}");
            return false;
        }
    };
    let state_before = law.digest();
    let cells_recount = recount_faces(&cells_world, footprint);
    let cells_recount_agrees = cells_ledger.columns_swept == cells_recount.columns
        && cells_ledger.cells == cells_recount.solid_cells
        && cells_ledger.faces == cells_recount.total_faces()
        && cells_ledger.columns_produced == 0
        && cell_mesh_before.area() == cells_ledger.faces
        && cell_mesh_before.key().state == state_before;
    println!(
        "U4 cells seed={} tick=0 chunk=({}, {}, {}) level={}: world patch {}×{} ({} column(s), {} under \
         this chunk); footprint {}×{} span {}..{} ({} band(s), {} cell(s)), halo owned = \
         {cells_halo_owned}; keyed by the cells' state {} → {} cells swept → {} faces ({} solid + {} water) \
         → {} merged away → {} quads ({} merged, factor {:.2}), vertices {}, triangles {}, digest {}; \
         recount of the same cells agrees = {cells_recount_agrees}",
        identity.seed,
        chunk.x, chunk.y, chunk.z,
        level.get(),
        cells_patch.side(),
        cells_patch.side(),
        cells_patch.len(),
        cells_ledger.columns_swept,
        footprint.patch().side(),
        footprint.patch().side(),
        footprint.floor_z(),
        footprint.ceiling_z(),
        (footprint.ceiling_z() - footprint.floor_z()) / CHUNK_SIDE,
        footprint.cells(),
        state_before.to_hex(),
        cells_ledger.cells,
        cells_ledger.faces,
        cells_recount.solid_faces,
        cells_recount.water_faces,
        cells_ledger.merged_away,
        cell_mesh_before.quads().len(),
        cells_ledger.rectangles_merged,
        cells_ledger.merge_factor(),
        cell_mesh_before.vertices(),
        cell_mesh_before.triangles(),
        cell_mesh_before.digest().to_hex()
    );

    // **The extent contract, measured on one chunk in three worlds.** The same chunk, the same span, three
    // different worlds around it. A world that is only the chunk owns no halo and draws the chunk's own edge
    // as a cliff — this is the number the audit measured through the reading's door. A world with one ring
    // owns the halo and draws the chunk's own surface. The law's own world owns part of the ring — this
    // chunk's east and north columns, not its west and south — so it draws exactly the two edges its own
    // patch ends at.
    let chunk_only_world = match chunk_cube(&roster, &world, &ladder, chunk) {
        Ok(cube) => cube,
        Err(refusal) => {
            eprintln!("U4 extent: the chunk's own cells refused by name: {refusal}");
            return false;
        }
    };
    let chunk_only_footprint = match Footprint::of_chunk(&chunk_only_world, chunk) {
        Ok(footprint) => footprint,
        Err(refusal) => {
            eprintln!("U4 extent: the chunk-only footprint refused by name: {refusal}");
            return false;
        }
    };
    let mut only_ledger = WorkLedger::default();
    let chunk_only_mesh = match mesher.derive_cubic(
        &chunk_only_world,
        chunk,
        level,
        Effort::plain(),
        &mut Resident::new(),
        &mut only_ledger,
    ) {
        Ok(mesh) => mesh,
        Err(refusal) => {
            eprintln!("U4 extent: the chunk-only world refused by name: {refusal}");
            return false;
        }
    };
    let chunk_only_halo_owned = chunk_only_footprint.halo_owned(&chunk_only_world);
    let halo_halo_owned = halo_footprint.halo_owned(&halo_world);
    println!(
        "U4 extent contract seed={} tick=0 chunk=({}, {}, {}) level={}: one footprint {}×{} span {}..{} \
         ({} band(s), {} cell(s)) in three worlds — a world that is only the chunk (halo owned = \
         {chunk_only_halo_owned}): {} faces; the chunk + {} ring (halo owned = {halo_halo_owned}): {} faces, \
         the same list as the reading's = {doors_agree}; the law's own world (halo owned = \
         {cells_halo_owned}): {} faces",
        identity.seed,
        chunk.x, chunk.y, chunk.z,
        level.get(),
        footprint.patch().side(),
        footprint.patch().side(),
        footprint.floor_z(),
        footprint.ceiling_z(),
        (footprint.ceiling_z() - footprint.floor_z()) / CHUNK_SIDE,
        footprint.cells(),
        chunk_only_mesh.area(),
        MESH_RING,
        halo_cells.area(),
        cell_mesh_before.area()
    );

    // **The stitched mesh**: a rectangle of two chunks in one mesh, from the cells' door — one footprint, one
    // sweep, one merge. The boundary between the two chunks is inside the footprint, so no rectangle may
    // stand on it: a face there would be the inside of the world, seen from between two chunks. The region's
    // faces are then the two chunks' own faces summed, because the shared boundary contributes nothing —
    // which is what makes a mesh of chunks a mesh of the world.
    let region_world = match region_cube(&roster, &world, &ladder, chunk, 2) {
        Ok(cube) => cube,
        Err(refusal) => {
            eprintln!("U4 region: the rectangle's own cells refused by name: {refusal}");
            return false;
        }
    };
    let region_origin = match chunk.origin() {
        Ok(origin) => origin,
        Err(kind) => {
            eprintln!("U4 region: the chunk has no expressible origin: {kind:?}");
            return false;
        }
    };
    let region_patch = match Patch::new(region_origin, CHUNK_SIDE as u32 * 2) {
        Ok(patch) => patch,
        Err(refusal) => {
            eprintln!("U4 region: the rectangle's own footprint refused by name: {refusal}");
            return false;
        }
    };
    let region_footprint = match Footprint::over(&region_world, region_patch) {
        Ok(footprint) => footprint,
        Err(refusal) => {
            eprintln!("U4 region: the rectangle's footprint refused by name: {refusal}");
            return false;
        }
    };
    let mut region_ledger = WorkLedger::default();
    let region = match mesher.derive_region(
        &region_world,
        chunk,
        2,
        level,
        Effort::new(Budget::unlimited(), 1),
        &mut region_ledger,
    ) {
        Ok(mesh) => mesh,
        Err(refusal) => {
            eprintln!("U4 region: the stitched mesh refused by name: {refusal}");
            return false;
        }
    };
    // The seam's own planes, in the world's voxel coordinates (a quad's `plane` is that coordinate: the
    // plane just outside the cell that shows the face), so both sides of the seam name the same plane.
    let seam_x = region_origin.x + CHUNK_SIDE;
    let seam_y = region_origin.y + CHUNK_SIDE;
    let boundary_quads = region
        .quads()
        .iter()
        .filter(|quad| {
            (quad.axis == Axis::X && quad.plane == seam_x)
                || (quad.axis == Axis::Y && quad.plane == seam_y)
        })
        .count();
    // The same rectangle's chunks, meshed one by one over the same world. Their own faces sum to the
    // region's — the shared boundaries are drawn once in either reading — and they hold one band, so the
    // region's span is theirs. The boundary planes are the one place both doors must agree face for face:
    // the two sides of the seam differ in height (that is what a declared world's ground does), so the
    // faces there are real cliffs, and the chunk that owns each side is the one that shows it.
    let mut alone_faces = 0;
    let mut spans_agree = true;
    let mut boundary_in_chunks = 0;
    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
        let at = ChunkKey::new(chunk.x + dx, chunk.y + dy, chunk.z);
        let mut one_ledger = WorkLedger::default();
        let one = match mesher.derive_cubic(
            &region_world,
            at,
            level,
            Effort::plain(),
            &mut Resident::new(),
            &mut one_ledger,
        ) {
            Ok(mesh) => mesh,
            Err(refusal) => {
                eprintln!(
                    "U4 region: the chunk at ({}, {}, {}) alone refused by name: {refusal}",
                    at.x, at.y, at.z
                );
                return false;
            }
        };
        spans_agree &= match Footprint::of_chunk(&region_world, at) {
            Ok(footprint) => footprint.span() == region_footprint.span(),
            Err(_) => false,
        };
        // This chunk's own faces on the seam's planes — the same physical seam, read from one side by each
        // chunk, and a world coordinate so both sides name the same plane.
        boundary_in_chunks += one
            .quads()
            .iter()
            .filter(|quad| {
                (quad.axis == Axis::X && quad.plane == seam_x)
                    || (quad.axis == Axis::Y && quad.plane == seam_y)
            })
            .count();
        alone_faces += one.area();
    }
    let region_sums = region.area() == alone_faces && spans_agree;
    let boundary_agrees = boundary_in_chunks == boundary_quads;
    println!(
        "U4 region seed={} tick=0 origin=({}, {}, {}) {} chunk(s) level={}: footprint {}×{} span {}..{} \
         ({} band(s), {} cell(s)) → {} cells swept → {} faces → {} merged away → {} quads, vertices {}, \
         digest {}; the shared boundaries at x={} and y={} carry {} rectangle(s), the cliffs the two \
         sides really show; the {} chunk(s) carry the same {} on those planes = {boundary_agrees}; the {} \
         chunk(s) meshed one by one over the same world draw {} faces, the region {} = {region_sums}",
        identity.seed,
        chunk.x, chunk.y, chunk.z,
        2,
        level.get(),
        region_footprint.patch().side(),
        region_footprint.patch().side(),
        region_footprint.floor_z(),
        region_footprint.ceiling_z(),
        (region_footprint.ceiling_z() - region_footprint.floor_z()) / CHUNK_SIDE,
        region_footprint.cells(),
        region_ledger.cells,
        region_ledger.faces,
        region_ledger.merged_away,
        region.quads().len(),
        region.vertices(),
        region.digest().to_hex(),
        seam_x,
        seam_y,
        boundary_quads,
        4,
        boundary_in_chunks,
        4,
        alone_faces,
        region.area()
    );

    // One tick of the declared law, over the cells themselves.
    let stepped_tick = match law.step() {
        Ok(tick) => tick,
        Err(refusal) => {
            eprintln!("U4 cells: the law's own pour refused by name: {refusal}");
            return false;
        }
    };
    let state_after = law.digest();
    let cells_state_moved = state_after != state_before;
    let cells_stale_not_served = cells_resident.serve(state_after, chunk, level).is_none();
    let cells_stale = cells_resident.stale(state_after);
    let mut cells_after_ledger = WorkLedger::default();
    let cell_mesh_after = match mesher.derive_cubic(
        law.world(),
        chunk,
        level,
        Effort::new(Budget::unlimited(), 1),
        &mut cells_resident,
        &mut cells_after_ledger,
    ) {
        Ok(mesh) => mesh,
        Err(refusal) => {
            eprintln!("U4 cells: the stepped world refused by name: {refusal}");
            return false;
        }
    };
    let stepped_recount = recount_faces(law.world(), footprint);
    let ground_same = cell_mesh_before
        .quads()
        .iter()
        .filter(|quad| quad.layer == Layer::Solid)
        .eq(cell_mesh_after.quads().iter().filter(|quad| quad.layer == Layer::Solid));
    let cells_rekeyed = cell_mesh_after.key().state == state_after
        && cell_mesh_before.key().state == state_before
        && state_after != state_before;
    let cells_removed = cells_resident.invalidate(state_after);
    let cells_kept = cells_resident.len();
    println!(
        "U4 cells after one tick of the law (tick {}): state {} moved = {cells_state_moved}; the older \
         state's entry served for it = {} (never stale), stale = {cells_stale}; rebuild: {} cells swept → \
         {} faces ({} solid + {} water) → {} merged away → {} quads, vertices {}, triangles {}, digest {}, \
         keyed by the new state = {cells_rekeyed}; the same ground faces = {ground_same}; invalidate \
         removed {cells_removed}, kept {cells_kept}",
        stepped_tick.0,
        state_after.to_hex(),
        !cells_stale_not_served,
        cells_after_ledger.cells,
        cells_after_ledger.faces,
        stepped_recount.solid_faces,
        stepped_recount.water_faces,
        cells_after_ledger.merged_away,
        cell_mesh_after.quads().len(),
        cell_mesh_after.vertices(),
        cell_mesh_after.triangles(),
        cell_mesh_after.digest().to_hex()
    );

    // **The camera's own residency, over the world the client shows (2026-10-14d).** This closes the defect
    // the last pass named with numbers: the residency streamed the **declared** surface while the drawn frame
    // showed the **law's state**, so the two disagreed in 3 673 of 131 072 cell(s) — a residency feeding
    // nothing the picture shows. It now streams the shown state ([`surface::client_state`], the world the
    // drawn frame draws), holds it in the world's own store, and the drawn door takes the residency itself,
    // so what a camera holds **is** what the picture is. Measured below: the held cells against the shown
    // state (0 differing) and against the declared surface (the 3 673 the port measured, through the
    // residency now), the eviction boundary, a walk, and the frame path following the residency through it.
    let shown = match surface::client_state() {
        Ok(plane) => plane,
        Err(refusal) => {
            eprintln!("U4 stream: the client's own shown state refused by name: {refusal}");
            return false;
        }
    };
    let shown_world = shown.world();
    let declared_world = match surface::client_world() {
        Ok(world) => world,
        Err(refusal) => {
            eprintln!("U4 stream: the client's own declared world refused by name: {refusal}");
            return false;
        }
    };
    let home = match shown_world.first_chunk() {
        Some(key) => key,
        None => {
            eprintln!("U4 stream: the world this camera stands in has no chunk");
            return false;
        }
    };
    let mut streamer = Streamer::over(shown_world, residency);
    let first_step = match streamer.observe(shown_world, home) {
        Ok(step) => step,
        Err(refusal) => {
            eprintln!("U4 stream: the world's own door refused a chunk: {refusal}");
            return false;
        }
    };
    let still_step = match streamer.observe(shown_world, home) {
        Ok(step) => step,
        Err(refusal) => {
            eprintln!("U4 stream: the world's own door refused a chunk: {refusal}");
            return false;
        }
    };
    let mut shown_differing = 0usize;
    let mut declared_differing = 0usize;
    let mut covered = 0usize;
    for key in streamer.keys() {
        let held = streamer.chunk(key).expect("a held key has a payload");
        let shown_chunk = match shown_world.chunk(key) {
            Ok(chunk) => chunk,
            Err(refusal) => {
                eprintln!("U4 stream: a held chunk is not the shown world's: {refusal}");
                return false;
            }
        };
        let declared_chunk = match declared_world.chunk(key) {
            Ok(chunk) => chunk,
            Err(refusal) => {
                eprintln!("U4 stream: a held chunk is not the declared world's: {refusal}");
                return false;
            }
        };
        covered += held.cells().len();
        for (held_cell, (shown_cell, declared_cell)) in
            held.cells().iter().zip(shown_chunk.cells().iter().zip(declared_chunk.cells()))
        {
            if held_cell != shown_cell {
                shown_differing += 1;
            }
            if held_cell != declared_cell {
                declared_differing += 1;
            }
        }
    }
    let stream_cells_equal = shown_differing == 0 && covered == shown_world.cells();
    // The one number this pass must *reproduce* rather than re-measure: the port measured **3 673** of these
    // 131 072 cell(s) differing between the declared surface and the world the client shows. A residency over
    // the shown state reaches the same number from the other side, and a change to what the world is would
    // move it — which is a defect to name, not to smooth over.
    let stream_declared_gap_is_the_ports = declared_differing == 3673;
    println!(
        "U4 stream seed={} tick={} horizon {:.0} m: the camera's own chunk ({}, {}, {}) holds {} of this \
         world's {} chunk(s) — {covered} cell(s) over {} derivation(s), of the state the client shows ({}); a \
         camera that does not move derives {} more and serves {} of {} ({} cell(s) held), and the held cells \
         are **the shown world's own**: {} differing of {covered}, while against the **declared surface the \
         fields make** {} of {covered} cell(s) differ — the law's own {SETTLE_TICKS} tick(s) inside the \
         source, and the 3 673 of 131 072 the port measured, reached from the residency's own side: {}; \
         residency identity {}",
        identity.seed,
        shown.ticks(),
        residency.radius_m(),
        home.x,
        home.y,
        home.z,
        first_step.resident,
        shown_world.chunks(),
        first_step.derived,
        first_step.state.to_hex(),
        still_step.derived,
        still_step.served,
        still_step.resident,
        streamer.cells(),
        shown_differing,
        declared_differing,
        stream_declared_gap_is_the_ports,
        streamer.digest().to_hex(),
    );

    // **Where the first chunk is evicted, and what a walk back costs.** A residency over a finite world is
    // bounded by the world and not by the horizon: the furthest chunk from the camera's own chunk sets the
    // boundary, and past it the streamer holds what the horizon keeps rather than what it would like.
    let furthest = shown_world
        .keys()
        .into_iter()
        .filter_map(|key| chunk_centre_distance_m(home, key))
        .fold(0.0f64, f64::max);
    let reaches = ResidencyHorizon::m(furthest.ceil()).expect("a positive horizon");
    let tight = ResidencyHorizon::m(furthest.ceil() - 1.0).expect("a positive horizon");
    let mut at_edge = Streamer::over(shown_world, reaches);
    let at_edge_step = match at_edge.observe(shown_world, home) {
        Ok(step) => step,
        Err(refusal) => {
            eprintln!("U4 stream: the world's own door refused a chunk: {refusal}");
            return false;
        }
    };
    let mut walking = Streamer::over(shown_world, tight);
    let walked_from = match walking.observe(shown_world, home) {
        Ok(step) => step,
        Err(refusal) => {
            eprintln!("U4 stream: the world's own door refused a chunk: {refusal}");
            return false;
        }
    };
    let east = ChunkKey::new(home.x + 1, home.y, home.z);
    let walked_to = match walking.observe(shown_world, east) {
        Ok(step) => step,
        Err(refusal) => {
            eprintln!("U4 stream: the world's own door refused a chunk: {refusal}");
            return false;
        }
    };
    let walked_back = match walking.observe(shown_world, home) {
        Ok(step) => step,
        Err(refusal) => {
            eprintln!("U4 stream: the world's own door refused a chunk: {refusal}");
            return false;
        }
    };
    let stream_holds_all = first_step.resident == shown_world.chunks()
        && at_edge_step.resident == shown_world.chunks()
        && at_edge.cells() == shown_world.cells();
    let stream_free_when_still = still_step.derived == 0 && still_step.served == first_step.resident;
    let stream_evicts_and_re_requests = walked_from.resident < shown_world.chunks()
        && walked_to.evicted >= 1
        && walked_back.derived >= 1
        && walked_back.served >= 1
        && walked_back.resident == walked_from.resident;
    println!(
        "U4 stream first eviction: the furthest chunk this world names from its own first chunk ({}, {}, \
         {}) is {furthest:.4} m away, so the streamer holds {} of {} chunk(s) at {:.0} m and {} at {:.0} m — \
         the eviction boundary is the world's own extent, and the {} m horizon's wider reach has no chunk of \
         this world to fill it with",
        home.x,
        home.y,
        home.z,
        at_edge_step.resident,
        shown_world.chunks(),
        reaches.radius_m(),
        walked_from.resident,
        tight.radius_m(),
        residency.radius_m(),
    );
    println!(
        "U4 stream a walk at {:.0} m: the camera's own chunk holds {}; one chunk east evicts {} and holds \
         {}; back derives {} and serves {} of {} — a released chunk costs one derivation and a kept chunk \
         none",
        tight.radius_m(),
        walked_from.resident,
        walked_to.evicted,
        walked_to.resident,
        walked_back.derived,
        walked_back.served,
        walked_back.resident,
    );

    // **The frame path follows the residency** — `surface::draw_plane` takes the residency itself, so a frame
    // is a picture of the cells a camera holds and of nothing else. Four measurements on one world: the frame
    // the declared residency draws (whose mesh state is that residency's own identity); the frame a residency
    // of the **declared** surface draws instead — the same keys, the cells before the law moved them, a
    // visibly different picture; and the frames of a residency holding a part of the region (3 of 4 chunk(s))
    // and of one holding none, which the region refuses **by name**, carrying the count it does hold, rather
    // than drawing a picture of less than the region.
    let stream_frame = match surface::draw_plane(&streamer, surface::DRAWN_CHUNK) {
        Ok((frame, _)) => frame,
        Err(refusal) => {
            eprintln!("U4 stream: the frame drawn from the residency refused by name: {refusal}");
            return false;
        }
    };
    let placed = match surface::residency_of(&declared_world) {
        Ok(residency) => residency,
        Err(refusal) => {
            eprintln!("U4 stream: a residency of the declared surface refused by name: {refusal}");
            return false;
        }
    };
    let placed_frame = match surface::draw_plane(&placed, surface::DRAWN_CHUNK) {
        Ok((frame, _)) => frame,
        Err(refusal) => {
            eprintln!("U4 stream: the declared surface's own frame refused by name: {refusal}");
            return false;
        }
    };
    let narrow_refusal = surface::draw_plane(&walking, surface::DRAWN_CHUNK).err();
    let away = ChunkKey::new(home.x + 9, home.y + 9, home.z + 7);
    let mut empty = Streamer::over(shown_world, residency);
    let empty_step = match empty.observe(shown_world, away) {
        Ok(step) => step,
        Err(refusal) => {
            eprintln!("U4 stream: the world's own door refused a chunk: {refusal}");
            return false;
        }
    };
    let empty_refusal = surface::draw_plane(&empty, surface::DRAWN_CHUNK).err();
    let stream_frame_matches_the_residency = stream_frame.mesh_state() == streamer.digest();
    let stream_placed_frame_differs = placed_frame.digest() != stream_frame.digest()
        && placed_frame.quads() < stream_frame.quads();
    let stream_narrower_refused = narrow_refusal
        .as_ref()
        .map(|refusal| refusal.kind() == RefusalKind::ParameterOutOfRange)
        .unwrap_or(false);
    let stream_empty_refused = empty_step.resident == 0
        && empty_refusal
            .as_ref()
            .map(|refusal| refusal.kind() == RefusalKind::ParameterOutOfRange)
            .unwrap_or(false);
    println!(
        "U4 stream the frame follows the residency: the residency's {} chunk(s) of the shown state draw the \
         drawn reading's own picture — {} rectangle(s), frame digest {}, mesh state {} = the residency's own \
         identity = {} ({}) — while the same keys holding the **declared** surface draw where the world was \
         placed: {} rectangle(s), frame digest {} ({} fewer rectangle(s), a different picture: the 3 673 \
         moved cell(s) are {} rectangle(s) of it); a residency holding 3 of 4 chunk(s) is refused by the \
         region's own name: [{}] {}; one holding none is refused the same way: [{}] {}",
        streamer.len(),
        stream_frame.quads(),
        stream_frame.digest().to_hex(),
        stream_frame.mesh_state().to_hex(),
        streamer.digest().to_hex(),
        stream_frame_matches_the_residency,
        placed_frame.quads(),
        placed_frame.digest().to_hex(),
        stream_frame.quads() - placed_frame.quads(),
        stream_frame.quads() - placed_frame.quads(),
        narrow_refusal
            .as_ref()
            .map(|refusal| refusal.kind().name())
            .unwrap_or("drawn"),
        narrow_refusal
            .as_ref()
            .map(|refusal| refusal.detail().to_string())
            .unwrap_or_else(|| "—".to_string()),
        empty_refusal
            .as_ref()
            .map(|refusal| refusal.kind().name())
            .unwrap_or("drawn"),
        empty_refusal
            .as_ref()
            .map(|refusal| refusal.detail().to_string())
            .unwrap_or_else(|| "—".to_string()),
    );

    // SC-005: the two refusals that carry their numbers. Both on a cold cache, because a budget is a
    // ceiling on **work** and a hit has none.
    let mut cold = Resident::new();
    let mut cold_ledger = WorkLedger::default();
    let budget_refusal = mesher
        .derive(
            &world,
            chunk,
            level,
            Effort::new(Budget::new(64, 64), 1),
            &mut cold,
            &mut cold_ledger,
        )
        .expect_err("1024 columns cannot fit in a 64-column budget");
    println!(
        "U4 budget refusal by name: [{}] {} (nothing cached: {})",
        budget_refusal.kind().name(),
        budget_refusal.detail(),
        cold.is_empty()
    );

    let narrow = Mesher::new(&roster, ladder, ResidencyHorizon::m(48.0).expect("positive"));
    let mut narrow_resident = Resident::new();
    let mut narrow_ledger = WorkLedger::default();
    let far = ChunkKey::new(2, 0, 0);
    let horizon_refusal = narrow
        .derive(&world, far, level, Effort::new(Budget::unlimited(), 1), &mut narrow_resident, &mut narrow_ledger)
        .expect_err("two chunks away is past a 48 m horizon");
    println!(
        "U4 residency refusal by name: [{}] {} (nothing cached: {})",
        horizon_refusal.kind().name(),
        horizon_refusal.detail(),
        narrow_resident.is_empty()
    );
    println!(
        "U4 horizons mixed refused by name: [{}] {}",
        mixed.kind().name(),
        mixed.detail()
    );
    println!(
        "U4 owed after this seam: the displaced 256x surface (a non-planar rectangle must be subdivided \
         and triangulated), a stitched mesh wider than one rectangle (a region's own residency, because the \
         resident set's unit is a chunk and a level), a strided field request (U3's vocabulary), the \
         per-chunk memory figure and water's own side faces (this seam shows a water column's top face and \
         declares its sides its ground's faces) — and **a streamed residency policy came off this list \
         2026-10-14c**: `src/stream.rs` keeps the chunks around a camera's own chunk, derives each on the \
         spot through the keyed door, evicts what the horizon releases and re-derives what comes back \
         (a ceiling on what may be resident at once is still owed)"
    );

    // **The drawn reading, with no window** — §41's own seam, and the evidence that the mesher's
    // geometry reaches pixels: the world's own chunk's mesh, the camera over its own extent, the frame, all
    // through the **same call** the window's loop makes. What is measured here is exactly what the window's
    // last frame draws (the window's own report prints its agreement with this frame), so the drawn path's
    // guards can bite without a display.
    let drawn = match surface::drawn_frame() {
        Ok(drawn) => drawn,
        Err(refusal) => {
            eprintln!("U4 drawn: the drawn frame is refused by name: {refusal}");
            return false;
        }
    };
    println!("U4 drawn: {}", drawn.described());
    let drawn_bars = drawn.bars_hold();
    println!("U4 drawn: the drawn reading's own bars: {}", drawn.bars_described());

    horizons_ordered
        && mixed.kind() == RefusalKind::HorizonMixed
        && recount_agrees
        && worker_knob
        && hit_is_free
        && stale_not_served
        && digest_moved
        && removed == 1
        && kept == 1
        && coarser
        && doors_agree
        && halo_halo_owned
        && !chunk_only_halo_owned
        && !cells_halo_owned
        && region_sums
        && boundary_agrees
        && cells_recount_agrees
        && cells_state_moved
        && cells_stale_not_served
        && cells_stale == 1
        && cells_rekeyed
        && ground_same
        && cells_removed == 1
        && cells_kept == 1
        && budget_refusal.kind() == RefusalKind::GeometryBudgetExceeded
        && cold.is_empty()
        && horizon_refusal.kind() == RefusalKind::GeometryOutOfResidency
        && narrow_resident.is_empty()
        && ledger.refusals == 0
        && cells_ledger.refusals == 0
        && cells_after_ledger.refusals == 0
        && stream_holds_all
        && stream_free_when_still
        && stream_evicts_and_re_requests
        && stream_cells_equal
        && stream_declared_gap_is_the_ports
        && stream_frame_matches_the_residency
        && stream_frame.digest() == drawn.last.digest()
        && stream_placed_frame_differs
        && stream_narrower_refused
        && stream_empty_refused
        && mesh.quads().len() > 1
        && cell_mesh_before.quads().len() > 1
        && drawn_bars
}

/// One receipt, printed the way a caller reads it: the request, the answer and its digest, the checks
/// that ran, the claims the answer makes, and what the call changed.
fn print_receipt(receipt: &Receipt) {
    println!("U6 call `{}`", receipt.request);
    println!(
        "U6   answer   {}  [{}]",
        receipt.answer.render(),
        receipt.answer.digest().to_hex()
    );
    let checks = receipt
        .checks
        .iter()
        .map(|check| {
            let outcome = match &check.outcome {
                CheckOutcome::Passed => "passed".to_string(),
                CheckOutcome::Refused(refusal) => format!("refused: {refusal}"),
            };
            format!("{}[{outcome}]", check.name.name())
        })
        .collect::<Vec<_>>()
        .join(" ");
    println!("U6   checks   {checks}");
    let claims = receipt
        .claims
        .iter()
        .map(|claim| format!("{} ({})", claim.name(), claim.describe()))
        .collect::<Vec<_>>()
        .join("; ");
    println!(
        "U6   claims   {}",
        if claims.is_empty() {
            "none — this row declares no `claims_hold`, and makes no claim nothing here could re-check"
                .to_string()
        } else {
            claims
        }
    );
    println!("U6   effect   {}", receipt.effect.render());
}

/// U6 — the record and the API layer: the closed table, receipts that carry their own evidence, one
/// request twice one answer, both refusal classes, and an answer checked against the world it names.
///
/// The evidence `specs/005-u6-record-api/spec.md` asks for, **printed** rather than asserted in
/// private: every declared verb called through the public surface with every claim verified
/// (SC-001), the same request twice printing one answer with two effects (SC-002), a world refusal
/// named and recorded while a parse fault writes nothing (SC-003), an archived receipt verified
/// against the world it was made in and refused against a world that moved (SC-004), and the effect
/// of every call saying what changed (SC-005).
pub fn check_u6() -> bool {
    let roster = match process::roster() {
        Ok(roster) => roster,
        Err(refusals) => {
            eprintln!("U6 roster refused by name: {refusals}");
            return false;
        }
    };
    // The gate comes from the **one** place the reference declaration is written down, so the
    // binary's evidence, its `serve` mode and the tests cannot drift into three similar worlds.
    let mut gate = match reference::gate(&roster, 1) {
        Ok(gate) => gate,
        Err(refusal) => {
            eprintln!("U6 the reference declaration is refused by name: {refusal}");
            return false;
        }
    };
    let (seed, period_s) =
        (gate.world().identity().seed, gate.world().identity().period.get());
    if let Err(refusal) = gate.chain_mut().append(
        Namespace::World,
        EventKind::WorldOpened,
        format!("seed={seed} period={period_s}s"),
    ) {
        eprintln!("U6 could not open the record: {refusal}");
        return false;
    }

    println!("U6 tool table: {} declared verbs, closed and bijective", TOOL_COUNT);
    for tool in &TOOLS {
        let args = if tool.args.is_empty() {
            "no arguments".to_string()
        } else {
            tool.args
                .iter()
                .map(|argument| format!("{}:{}", argument.name, argument.kind.name()))
                .collect::<Vec<_>>()
                .join(", ")
        };
        println!(
            "U6   {:<9} {args:<38} owner {:<20} record {:<5} checks {}",
            tool.name,
            tool.owner,
            tool.changes_record,
            tool.checks
                .iter()
                .map(|check| check.name())
                .collect::<Vec<_>>()
                .join("+")
        );
        println!("U6   {:<9} answers: {}", "", tool.answers);
    }

    // **The row by reference.** The `admit` row's two documents are written here, under a root of
    // their own, and the gate's root is pinned where they live — so the request below points at bytes
    // that are on a disk rather than at a digest this report made up.
    let intake_root = std::env::temp_dir().join("shploingus-u6-intake-wire");
    let _ = std::fs::remove_dir_all(&intake_root);
    if let Err(error) = std::fs::create_dir_all(&intake_root) {
        eprintln!("U6 intake: the served row's fixture root could not be made: {error}");
        return false;
    }
    let source_bytes = b"# a declared source document\n";
    let check_bytes = b"#[test] fn the_bar() {}\n";
    for (name, bytes) in [("source.md", &source_bytes[..]), ("check.md", &check_bytes[..])] {
        if let Err(error) = std::fs::write(intake_root.join(name), bytes) {
            eprintln!("U6 intake: {name} could not be written: {error}");
            return false;
        }
    }
    let source_digest = Digest::of(source_bytes);
    let check_digest = Digest::of(check_bytes);
    gate.set_intake_root(&intake_root);

    // One spelling of the row, so the calls below differ only in what the disk holds at the path.
    let row_line = |path: &str, digest: &Digest| {
        format!(
            "admit path={path} digest={} feature=a-declared-row licence=permitted claim=checksum_only \
             carrier=U4 check=check.md check_digest={}",
            digest.to_hex(),
            check_digest.to_hex()
        )
    };

    // SC-001: every declared verb, through the public surface, with every claim verified — and
    // **U9's document form through this table's own two rows** (2026-10-12, §28), which arrive here
    // the way a client reaches them: the `save` row writes a world down at the gate's own root, and the
    // `load` row is handed **the digest that write reported**, so the reference the second line carries
    // is the document the first line placed rather than a digest this report made up.
    let mut calls: Vec<String> = vec![
        "truth".to_string(),
        "fields field=elevation at=0,0,0 side=8".to_string(),
        "geometry chunk=0,0,0 level=0".to_string(),
        "held".to_string(),
        "record".to_string(),
        row_line("source.md", &source_digest),
    ];
    calls.push("save path=u6-world.save".to_string());
    let mut every_answer_verified = true;
    let mut saved_facts: Option<DocumentFacts> = None;
    let mut loaded_facts: Option<DocumentFacts> = None;
    // The `load` line does not exist until the `save` line has answered: it is handed **that**
    // document's digest. That is also why the rows run in one pass with the list growing behind them —
    // a document read against an older digest would be a *moved* document, which is the refusal the
    // document form has a name for and not what this measurement is about.
    let mut index = 0;
    while index < calls.len() {
        let line = calls[index].clone();
        let receipt = match gate.call(&line) {
            Ok(receipt) => receipt,
            Err(refusal) => {
                eprintln!("U6 call `{line}` refused by name: {refusal}");
                every_answer_verified = false;
                index += 1;
                continue;
            }
        };
        print_receipt(&receipt);
        match receipt.verify(&gate.owners()) {
            Ok(()) => println!("U6   verified every claim re-checked against the owners it describes"),
            Err(refusals) => {
                eprintln!("U6   NOT VERIFIED: {refusals}");
                every_answer_verified = false;
            }
        }
        match &receipt.answer {
            Answer::Saved(facts) => {
                calls.push(format!("load path=u6-world.save digest={}", facts.digest.to_hex()));
                saved_facts = Some(facts.clone());
            }
            Answer::Loaded(facts) => loaded_facts = Some(facts.clone()),
            _ => {}
        }
        index += 1;
    }
    every_answer_verified &= calls.len() == TOOL_COUNT;
    // **What the two rows are for, in one line**: the document is where the gate's own door looks, its
    // digest is the digest the write reported, and the world read back is the world that was written —
    // same identity, same record, and the truth this process derives for itself. A zero here is a
    // failure, so the two rows are held to their declaration rather than to the report's word.
    let (saved_facts, loaded_facts) = match (saved_facts, loaded_facts) {
        (Some(saved), Some(loaded)) => (saved, loaded),
        _ => {
            eprintln!("U6 the two save rows did not both answer");
            return false;
        }
    };
    let document_round_trip = saved_facts == loaded_facts
        && loaded_facts.seed == gate.world().identity().seed
        && loaded_facts.period_s == gate.world().identity().period.get()
        && loaded_facts.tick == gate.world().tick().0
        && loaded_facts.ruleset == gate.ruleset()
        && loaded_facts.truth == gate.world().truth_digest()
        && loaded_facts.record_len == gate.chain().len()
        && loaded_facts.record_head == gate.chain().head();
    println!(
        "U6 document rows: `save` wrote {} at {} ({}), `load` read them back as seed={} period={}s \
         tick={} truth={} — the gate's own {}, and the write and the read are one answer = \
         {document_round_trip}",
        loaded_facts.len,
        loaded_facts.path,
        loaded_facts.digest.to_hex(),
        loaded_facts.seed,
        loaded_facts.period_s,
        loaded_facts.tick,
        loaded_facts.truth.to_hex(),
        gate.world().truth_digest().to_hex()
    );
    every_answer_verified &= document_round_trip;

    // **An admission's receipt is re-established, not believed.** It is the one row that makes no
    // claim, so its receipt carries no evidence a claim-check could re-run: the two references, the
    // requirements and the answer's axis states are all carried values. The row is admitted once more
    // — this time requiring a stage the world runs — and the receipt is doctored four ways and handed
    // back to verification, each time through [`Receipt::from_wire`] and `verify` exactly as a client
    // would. Every doctored receipt must refuse **by the owner's own name** and the honest one must
    // still verify; a certificate a writer could edit into an admission that never happened is not
    // evidence.
    let a_stage = roster.stages()[0].name;
    let requiring_line = format!(
        "{} requires_stages={a_stage}",
        row_line("source.md", &source_digest)
    );
    let reestablished = match gate.call(&requiring_line) {
        Ok(receipt) => receipt,
        Err(refusal) => {
            eprintln!("U6 receipts: the requiring row was refused by name: {refusal}");
            return false;
        }
    };
    let honest_verifies = reestablished.verify(&gate.owners()).is_ok();
    let refusal_of = |wire: &str| -> String {
        match Receipt::from_wire(wire) {
            Err(refusal) => refusal.kind().name().to_string(),
            Ok(receipt) => match receipt.verify(&gate.owners()) {
                Ok(()) => "verified".to_string(),
                Err(refusals) => refusals
                    .first_named()
                    .map(|refusal| refusal.kind().name().to_string())
                    .unwrap_or_else(|| "unnamed".to_string()),
            },
        }
    };
    let wire = reestablished.wire();
    let undeclared_stage =
        refusal_of(&wire.replace(&format!("requires_stages={a_stage}"), "requires_stages=no-such-stage"));
    let requirements_stripped =
        refusal_of(&wire.replace(&format!(" requires_stages={a_stage}"), ""));
    let state_flipped = refusal_of(&wire.replace("conformance:active", "conformance:inactive"));
    // A verifier whose own tree holds no such document cannot re-establish the receipt at all, and
    // refuses by the seam's name for that rather than passing it.
    let elsewhere = std::env::temp_dir().join("shploingus-u6-receipt-elsewhere");
    let _ = std::fs::remove_dir_all(&elsewhere);
    let _ = std::fs::create_dir_all(&elsewhere);
    let unreachable_document = match reference::gate(&roster, 1) {
        Ok(mut blind) => {
            blind.set_intake_root(&elsewhere);
            match reestablished.verify(&blind.owners()) {
                Ok(()) => "verified".to_string(),
                Err(refusals) => refusals
                    .first_named()
                    .map(|refusal| refusal.kind().name().to_string())
                    .unwrap_or_else(|| "unnamed".to_string()),
            }
        }
        Err(refusal) => {
            eprintln!("U6 receipts: a second reference gate was refused by name: {refusal}");
            return false;
        }
    };
    let receipts_hold = honest_verifies
        && undeclared_stage == "stage_undeclared"
        && requirements_stripped == "receipt_mismatch"
        && state_flipped == "receipt_mismatch"
        && unreachable_document == "source_absent";
    println!(
        "U6 receipts: an admission re-established from its request — an honest one verifies = \
         {honest_verifies}; a stage nobody declares [{undeclared_stage}]; the requirements stripped \
         [{requirements_stripped}]; a carried axis flipped [{state_flipped}]; a verifier whose tree \
         holds no such document [{unreachable_document}] (every one as declared: {receipts_hold})"
    );
    // **What this still owes**, named rather than implied: the five older rows' answers are not
    // re-established from their requests. Their claims are their evidence, and re-deriving `fields` or
    // `geometry` would perform work and move the owners (U4's ledger, the resident set, U2's record) —
    // a read-only re-evaluation seam `spec.md` and the LEDGER do not design — so for those rows an
    // answer field edited to contradict the claim beside it is still not refused by that comparison.
    println!(
        "U6 receipts still OWED: the five older rows' answers are not re-established from their \
         requests — their claims are their evidence, and re-deriving `fields` or `geometry` would \
         perform work and move the owners (the ledger, the resident set, the record), which the spec \
         and the LEDGER do not design as a read-only seam — so an answer field edited against its own \
         claim is not refused by that comparison for those rows."
    );

    // **A reference that is not the document it names refuses by its own name, with both values** —
    // the way the save's digest mismatch does. An absent document and a document whose bytes have
    // moved are two different facts, so they have two names, and because both lines **parsed** each
    // refusal is recorded like any other world-level refusal.
    let before_reference = gate.chain().len();
    let absent = match gate.call(&row_line("gone.md", &source_digest)) {
        Ok(_) => {
            eprintln!("U6 a reference to a document that is not there was admitted");
            return false;
        }
        Err(refusal) => refusal,
    };
    let moved_bytes = Digest::of(b"# a declared source document!\n");
    if let Err(error) = std::fs::write(intake_root.join("source.md"), b"# a declared source document!\n")
    {
        eprintln!("U6 the source document could not be rewritten: {error}");
        return false;
    }
    let tampered = match gate.call(&row_line("source.md", &source_digest)) {
        Ok(_) => {
            eprintln!("U6 a reference whose document had moved was admitted");
            return false;
        }
        Err(refusal) => refusal,
    };
    let absent_named =
        absent.kind() == RefusalKind::SourceAbsent && absent.detail().contains("gone.md");
    let both_values = tampered.kind() == RefusalKind::SourceDigestMismatch
        && tampered.detail().contains(&source_digest.to_hex())
        && tampered.detail().contains(&moved_bytes.to_hex());
    let recorded_reference_refusals = gate.chain().len() == before_reference + 2;
    let reference_still_verifies = gate.chain().verify().is_ok();
    println!(
        "U6 intake by reference: absent [{}] {} (the path named: {absent_named}); the bytes moved \
         [{}] {} (both digests in the message: {both_values}); both lines parsed, so both refusals are \
         recorded = {recorded_reference_refusals}, and the record still verifies = \
         {reference_still_verifies}",
        absent.kind().name(),
        absent.detail(),
        tampered.kind().name(),
        tampered.detail()
    );

    // SC-002: one request twice is one answer, while the two effects differ and the record moved once.
    let miss = match gate.call("geometry chunk=1,0,0 level=0") {
        Ok(receipt) => receipt,
        Err(refusal) => {
            eprintln!("U6 repeat derivation refused: {refusal}");
            return false;
        }
    };
    let hit = match gate.call("geometry chunk=1,0,0 level=0") {
        Ok(receipt) => receipt,
        Err(refusal) => {
            eprintln!("U6 repeat hit refused: {refusal}");
            return false;
        }
    };
    let one_answer = miss.answer_digest() == hit.answer_digest();
    let effects_differ = miss.effect.columns_produced == ring_columns()
        && miss.effect.appended.is_some()
        && hit.effect.columns_produced == 0
        && hit.effect.appended.is_none()
        && hit.effect.hits == 1;
    let moved_once = miss.effect.events_after == miss.effect.events_before + 1
        && hit.effect.events_after == miss.effect.events_after;
    println!(
        "U6 seed={} tick={} repeat [`geometry chunk=1,0,0 level=0`]: one answer digest {} \
         (identical: {one_answer}); the first did {} columns of work and appended {}, the second did \
         {} and appended {} (effects differ: {effects_differ}); the record moved once for the change: \
         {moved_once}",
        seed,
        gate.world().tick().0,
        miss.answer_digest().to_hex(),
        miss.effect.columns_produced,
        miss.effect.appended.is_some(),
        hit.effect.columns_produced,
        hit.effect.appended.is_some()
    );

    // SC-003: a parse fault writes nothing; a world refusal is named and recorded exactly once.
    let before = gate.chain().len();
    let parse_fault = match gate.call("geometry chunk=0,0 level=0") {
        Ok(_) => {
            eprintln!("U6 a malformed chunk was answered");
            return false;
        }
        Err(refusal) => refusal,
    };
    let untouched = gate.chain().len() == before;
    println!(
        "U6 parse refusal by name: [{}] {} (the record is untouched: {untouched})",
        parse_fault.kind().name(),
        parse_fault.detail()
    );

    let held_before = gate.resident().len();
    let world_fault = match gate.call("geometry chunk=9,9,0 level=0") {
        Ok(_) => {
            eprintln!("U6 a chunk outside the horizon was answered");
            return false;
        }
        Err(refusal) => refusal,
    };
    let grew_by_one = gate.chain().len() == before + 1;
    let names_it = gate
        .chain()
        .events()
        .last()
        .map(|event| {
            event.kind() == EventKind::RefusalRecorded
                && event.payload().contains(world_fault.kind().name())
        })
        .unwrap_or(false);
    let cached_nothing = gate.resident().len() == held_before;
    println!(
        "U6 world refusal by name: [{}] {} -> exactly one event recorded = {grew_by_one}, naming it \
         = {names_it}, nothing cached = {cached_nothing}",
        world_fault.kind().name(),
        world_fault.detail()
    );

    // SC-004: the answer corresponds to the world it names — verified in it, refused against a world
    // that has moved, and refused when the geometry it claimed to hold is evicted (read, not
    // recomputed: the truth claim still holds, because the world did not move).
    let holds_where_made = miss.verify(&gate.owners()).is_ok();
    let moved = gate.world().with_tick(Tick(1));
    let owners = Owners::new(
        &moved,
        &roster,
        gate.resident(),
        gate.chain(),
        1,
        gate.ruleset(),
        gate.intake(),
    );
    let moved_failures = miss.verify(&owners).expect_err("a moved world");
    let refused_moved = moved_failures.contains(RefusalKind::ClaimUnverified);
    println!(
        "U6 correspondence: the archived receipt verifies in the world it was made in = \
         {holds_where_made}; against tick {} it refuses [claim_unverified] = {refused_moved}",
        moved.tick().0
    );
    for refusal in moved_failures.all() {
        println!("U6   {refusal}");
    }
    let removed = gate.resident_mut().invalidate(moved.truth_digest());
    let after_evict = miss.verify(&gate.owners()).expect_err("the held geometry is gone");
    let mesh_only = after_evict.len() == 1
        && after_evict
            .first_named()
            .map(|refusal| refusal.detail().contains("mesh_is_held"))
            .unwrap_or(false);
    println!(
        "U6 read, not recomputed: evicting {removed} stale entr(ies) leaves the archived answer's \
         held-geometry claim failing while its truth claim still holds ({mesh_only})"
    );

    let record_verifies = gate.chain().verify().is_ok();

    // SC-005: the effect says what the call changed, on the record itself.
    let record_after = match gate.call("record") {
        Ok(receipt) => receipt,
        Err(refusal) => {
            eprintln!("U6 record refused: {refusal}");
            return false;
        }
    };
    let record_still_verified = record_after.verify(&gate.owners()).is_ok();
    println!(
        "U6 record: {} events, head {}, verifies = {record_verifies}, a `record` read verifies = \
         {record_still_verified} and changed {} event(s)",
        gate.chain().len(),
        gate.chain()
            .head()
            .map(|digest| digest.to_hex())
            .unwrap_or_else(|| "none".to_string()),
        record_after.effect.events_after - record_after.effect.events_before
    );
    // The transport: a **separate process** serves the table, and every served answer is checked
    // against this process's own world rather than trusted because it arrived over a pipe.
    let transport_ok = check_u6_transport(&roster);
    // The content gate's first slice: a declared row consumed **in place by reference**, judged on
    // named axes, admitted as `Established` — and the remainder of the gate still owed.
    let intake_ok = check_u6_intake();
    // The policy chokepoint: one decision function every served row passes through, deny by default,
    // scoped and attributed grants, and a refusal that quotes the list (2026-10-12, §29).
    let policy_ok = check_u6_policy();

    println!(
        "U6 still owed after this slice: the content gate's remainder — the document record and its \
         compression, the calibrated structural judge, the ballot, the recipe format, the API document, \
         the crate-by-axis census over the tree, and U10's own tree of declared rows; the bijection of \
         the served table against the legacy action vocabulary (a witness check P7 forbids reading the \
         legacy for); and the permission policy matrix's **remainder** — the self-describing protocol \
         an autonomous actor would discover the verbs through, and the tool-surface narrowing over a \
         mod-supplied tool list, neither of which this build has a caller for"
    );

    every_answer_verified
        && one_answer
        && effects_differ
        && moved_once
        && parse_fault.kind() == RefusalKind::ArgumentMalformed
        && untouched
        && world_fault.kind() == RefusalKind::GeometryOutOfResidency
        && grew_by_one
        && names_it
        && cached_nothing
        && holds_where_made
        && refused_moved
        && moved_failures.len() >= 2
        && removed >= 1
        && mesh_only
        && record_verifies
        && record_still_verified
        && transport_ok
        && intake_ok
        && policy_ok
        && absent_named
        && both_values
        && recorded_reference_refusals
        && reference_still_verifies
        // The re-established receipts are a **bar**, not a printed line: a doctored admission that
        // verified would make this false and the binary exit non-zero.
        && receipts_hold
}

/// U6's transport evidence: **a separate process** speaks the declared line protocol, and every
/// answer it hands back is checked against a world this process built itself — a served answer is not
/// trusted because it arrived over a pipe. The session covers a request that succeeds, one the world
/// refuses by its owner's name, one that does not parse the table at all, and the same request twice
/// (one answer, two effects); a parent-side replay of the same lines keeps this process's world,
/// resident and record in step, so every served receipt, refusal and record entry is compared with
/// the one derived here.
pub fn check_u6_transport(roster: &Declared) -> bool {
    use std::process::{Command, Stdio};

    // The session, in order: a read, a derivation, a world refusal, a parse fault, the same request
    // again (now a hit), the resident, and the record.
    let session = [
        "truth",
        "geometry chunk=2,0,0 level=0",
        "geometry chunk=9,9,0 level=0",
        "abracadabra",
        "geometry chunk=2,0,0 level=0",
        "held",
        "record",
    ];

    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(error) => {
            eprintln!("U6 transport: this binary's own path is unknown: {error}");
            return false;
        }
    };
    let mut child = match Command::new(&exe)
        .arg("serve")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            eprintln!("U6 transport: `serve` could not be spawned: {error}");
            return false;
        }
    };

    // **Send one line, read its answer, then send the next.** The session below is short enough that a
    // client writing all of it before reading any of it would survive — but only because it is short:
    // once the server's answers fill the pipe buffer, a write-everything client blocks forever, and a
    // hang is worse than a failure. The loop below is the shape later passes inherit.
    let mut every_request_sent = true;
    let mut input = match child.stdin.take() {
        Some(input) => input,
        None => {
            eprintln!("U6 transport: the child has no stdin");
            return false;
        }
    };
    let mut output = match child.stdout.take() {
        Some(stdout) => std::io::BufReader::new(stdout),
        None => {
            eprintln!("U6 transport: the child has no stdout");
            return false;
        }
    };

    // This process's own world, replayed call for call — the thing a served answer must correspond
    // to. The same reference declaration, so the comparison is two derivations of one world.
    let mut mine = match reference::gate(roster, 1) {
        Ok(gate) => gate,
        Err(refusal) => {
            eprintln!("U6 transport: the reference declaration is refused by name: {refusal}");
            return false;
        }
    };

    let mut responses = Vec::new();
    let mut correspondence = true;
    for line in session {
        if let Err(refusal) = transport::send(&mut input, line) {
            eprintln!("U6 transport: `{line}` could not be sent: {refusal}");
            every_request_sent = false;
            continue;
        }
        // Read **the answer to this line**, matched to it: a response about a different request can
        // never be counted as this one's.
        let response = match transport::read_response_for(&mut output, line) {
            Ok(Some(response)) => response,
            Ok(None) => {
                eprintln!("U6 transport: the server ended before answering `{line}`");
                return false;
            }
            Err(refusal) => {
                eprintln!("U6 transport: the response to `{line}` is malformed: {refusal}");
                return false;
            }
        };
        let reply = mine.call(line);

        // The record the client was told is the record this process holds after the same call, and
        // an `ok` receipt's claims re-check against this process's owners — answer digest included.
        let wire = response.record();
        let record_agrees = wire.events == mine.chain().len() && wire.head == mine.chain().head();
        let answer_agrees = match (&response, &reply) {
            (transport::Response::Ok { receipt, .. }, Ok(derived)) => {
                receipt.verify(&mine.owners()).is_ok()
                    && receipt.answer.digest() == derived.answer.digest()
            }
            (transport::Response::Err { kind, .. }, Err(refusal)) => kind == refusal.kind().name(),
            _ => false,
        };
        correspondence &= record_agrees && answer_agrees;
        responses.push(response);
    }
    // Close the pipe — the server reads its input to its end — and only then wait for it.
    drop(input);
    if responses.len() != session.len() {
        eprintln!(
            "U6 transport: {} request(s) were sent and {} answer(s) read",
            session.len(),
            responses.len()
        );
        return false;
    }
    let status = child.wait();
    let clean_exit = matches!(&status, Ok(status) if status.success());
    if !clean_exit {
        eprintln!("U6 transport: the served session did not exit cleanly: {status:?}");
    }

    // The session's own shape: five answered, two refused; the world's refusal named by its owner
    // and recorded, the parse fault named by the table and writing nothing.
    let answered = responses
        .iter()
        .filter(|response| response.receipt().is_some())
        .count();
    let world_named = responses[2].refusal_kind() == Some("geometry_out_of_residency");
    let parse_named = responses[3].refusal_kind() == Some("tool_undeclared");
    let world_grew_by_one = responses[2].record().events == responses[1].record().events + 1;
    let parse_wrote_nothing = responses[3].record().events == responses[2].record().events;

    // The same request twice over the pipe: one answer, two effects, and the hit appends nothing.
    let (miss, hit) = match (&responses[1], &responses[4]) {
        (
            transport::Response::Ok { receipt: miss, .. },
            transport::Response::Ok { receipt: hit, .. },
        ) => (miss, hit),
        _ => {
            eprintln!("U6 transport: the repeated request was not answered twice");
            return false;
        }
    };
    let one_answer = miss.answer.digest() == hit.answer.digest();
    let one_append = miss.effect.appended.is_some()
        && miss.effect.events_after == miss.effect.events_before + 1
        && hit.effect.columns_produced == 0
        && hit.effect.appended.is_none()
        && hit.effect.events_after == hit.effect.events_before;
    let served_record_is_ours = responses[6].record().events == mine.chain().len()
        && responses[6].record().head == mine.chain().head();

    println!(
        "U6 transport: `serve` — a separate process answered {answered}/{} request(s) with {correspondence} \
         correspondence; clean exit = {clean_exit}, every request sent = {every_request_sent}",
        session.len()
    );
    if let transport::Response::Err { kind, detail, .. } = &responses[2] {
        println!(
            "U6   world refusal [{kind}] {detail} -> recorded (the record grew by one: {world_grew_by_one})"
        );
    }
    if let transport::Response::Err { kind, detail, .. } = &responses[3] {
        println!(
            "U6   parse fault   [{kind}] {detail} -> writes nothing (the record held: {parse_wrote_nothing})"
        );
    }
    println!(
        "U6   repeat      one answer digest {} (identical: {one_answer}); the first appended exactly \
         one event, the second did 0 columns and appended none (one append: {one_append})",
        miss.answer.digest().to_hex()
    );
    println!(
        "U6   correspondence: every served receipt verified against this process's own world, \
         resident and record = {correspondence}; the served record is this process's record = \
         {served_record_is_ours}; world refusal named = {world_named}, parse fault named = {parse_named}"
    );

    every_request_sent
        && clean_exit
        && responses.len() == session.len()
        && answered == 5
        && world_named
        && parse_named
        && world_grew_by_one
        && parse_wrote_nothing
        && one_answer
        && one_append
        && served_record_is_ours
        && correspondence
}

/// U6's content gate — the intake slice: a declared row **consumed in place by reference**, judged on
/// named axes, admitted as `Established` (**never a bool**), with the undeclared and the unchecked
/// paths refused under their own names and the census refusing an unmeasured cell.
///
/// The subject is a document this run **writes itself**, under a directory of its own in the system
/// temp directory, so what is measured is a real file read where it lives rather than a fixture
/// committed to the tree. The rows hold a path and a digest and never the bytes; the one measurement
/// that proves it is the row that is admitted, then refused, when one byte of the document changes
/// underneath it.
pub fn check_u6_intake() -> bool {
    use crate::intake::{
        Acceptance, Axis, Claim, Establishment, Intake, Licence, Requirement, Requirements, Row,
        Source, WorldDeclarations,
    };

    // The world's **own** two declarations: the roster whose stages a row may require, and the registry
    // whose materials it may require. They are the reference declarations (`reference::gate` is built
    // over exactly these), so what is measured here is conformance against the world the binary serves.
    let roster = match crate::process::roster() {
        Ok(roster) => roster,
        Err(refusals) => {
            eprintln!("U6 intake: the reference roster is refused by name: {refusals}");
            return false;
        }
    };
    let registry = crate::reference::registry();

    let root = std::env::temp_dir().join("shploingus-u6-intake-evidence");
    let _ = std::fs::remove_dir_all(&root);
    if let Err(error) = std::fs::create_dir_all(&root) {
        eprintln!("U6 intake: the fixture root could not be made: {error}");
        return false;
    }
    let source_bytes = b"# a declared source document\n";
    let check_bytes = b"#[test] fn the_bar() {}\n";
    let write = |name: &str, bytes: &[u8]| -> Source {
        std::fs::write(root.join(name), bytes).expect("the fixture reaches the disk");
        Source::new(name, Digest::of(bytes))
    };
    let source = write("source.md", source_bytes);
    let check = write("check.md", check_bytes);
    let row = |feature: &str, claim: Claim| {
        Row::new(
            feature,
            source.clone(),
            Some(Licence::Permitted),
            Some("U4"),
            Some(Acceptance::Provided(check.clone())),
            claim,
            Requirements::nothing(),
        )
    };

    let intake = Intake::over(&root, WorldDeclarations { roster: &roster, registry });

    // 1. An admitted row, and what it became: integrity established, and **no identity proven**.
    let admitted = match intake.admit(&row("a declared row", Claim::ChecksumOnly)) {
        Ok(admitted) => admitted,
        Err(refusals) => {
            eprintln!("U6 intake: a complete row was refused: {refusals}");
            return false;
        }
    };
    let checksum_only = !admitted.establishes_identity();
    println!(
        "U6 intake: admitted {} as {} — found {}, identity established = {}",
        admitted.source().path(),
        admitted.claim_name(),
        admitted.found().to_hex(),
        admitted.establishes_identity()
    );
    for finding in admitted.findings() {
        println!("U6 intake:   {finding}");
    }

    // 2. The same document, pinned: identity established — by pinning, which is a real guarantee and
    //    not a signature.
    let pin = Digest::of(source_bytes);
    let pinned = match intake.admit(&row("a declared row", Claim::pinned("an operator", pin))) {
        Ok(pinned) => pinned,
        Err(refusals) => {
            eprintln!("U6 intake: a pinned row was refused: {refusals}");
            return false;
        }
    };
    let pinned_identity = pinned.establishes_identity();
    // **Every axis that was demanded is established** — and the ones that are not demanded are named
    // rather than folded in. One state is neither a pass nor a refusal: *checked, and reported* (a
    // checksum's identity, and now a row that requires nothing of the world). It is carried, it is
    // printed, and it is never counted as met.
    let reported: Vec<&str> = pinned
        .findings()
        .iter()
        .filter(|finding| !finding.is_pass())
        .map(|finding| finding.axis().name())
        .collect();
    let every_axis_established = pinned
        .findings()
        .iter()
        .all(|finding| finding.is_pass() || finding.refusal().is_none());
    println!(
        "U6 intake: pinned by {} — identity established = {pinned_identity}, every demanded axis \
         established = {every_axis_established}, reported rather than counted = {reported:?}",
        pinned.signer().unwrap_or("-")
    );

    // 3. **In place, never copied**: one byte of the document changes and the same row is refused,
    //    with both digests named. The row was never told to re-read anything; the gate reads where the
    //    document lives, every time.
    std::fs::write(root.join("source.md"), b"# a declared source document!\n")
        .expect("the fixture changes");
    let changed = intake
        .admit(&row("a declared row", Claim::ChecksumOnly))
        .expect_err("the document is no longer the one the row describes");
    let read_in_place = changed.contains(RefusalKind::SourceDigestMismatch) && changed.len() == 1;
    println!(
        "U6 intake: after one byte changes, the same row refuses = {read_in_place}: {}",
        changed
            .first_named()
            .map(|refusal| refusal.to_string())
            .unwrap_or_else(|| "-".to_string())
    );
    std::fs::write(root.join("source.md"), source_bytes).expect("the fixture is restored");

    // 4. **The undeclared path and the unchecked path, in one report and under two names.** The gate
    //    declares its carriers (U3-U8) and its axes; a carrier outside them is undeclared, and an axis
    //    nobody answered is unchecked. They are different facts and they are named differently.
    let partial = Row::new(
        "a declared row",
        Source::new("source.md", Digest::of(source_bytes)),
        None,
        Some("U9"),
        None,
        Claim::ChecksumOnly,
        Requirements::nothing(),
    );
    let inspection = intake.inspect(&partial);
    let refusals = inspection.refusals();
    let undeclared = refusals.contains(RefusalKind::CarrierUndeclared);
    let unchecked = refusals.contains(RefusalKind::AxisUnchecked);
    println!(
        "U6 intake: undeclared carrier = {undeclared}, unchecked axis = {unchecked}, distinct names = \
         {}",
        RefusalKind::CarrierUndeclared.name() != RefusalKind::AxisUnchecked.name()
    );
    for refusal in refusals.all() {
        println!("U6 intake:   {refusal}");
    }
    let states = |axis: Axis| {
        inspection
            .state(axis)
            .map(Establishment::name)
            .unwrap_or("-")
    };
    println!(
        "U6 intake:   states — source {}, licence {}, carrier {}, acceptance {}, identity {}, \
         conformance {}",
        states(Axis::Source),
        states(Axis::Licence),
        states(Axis::Carrier),
        states(Axis::Acceptance),
        states(Axis::Identity),
        states(Axis::Conformance)
    );

    // 5. Each of the remaining names, earned by the thing it describes.
    let owed = Row::new(
        "a declared row",
        source.clone(),
        Some(Licence::Permitted),
        Some("U4"),
        Some(Acceptance::Owed),
        Claim::ChecksumOnly,
        Requirements::nothing(),
    );
    let owed_refused = intake
        .admit(&owed)
        .expect_err("a check that is owed cannot be met")
        .contains(RefusalKind::AxisInactive);
    let licence_refused = intake
        .admit(&Row::new(
            "a declared row",
            source.clone(),
            Some(Licence::Refused),
            Some("U4"),
            Some(Acceptance::Provided(check.clone())),
            Claim::ChecksumOnly,
            Requirements::nothing(),
        ))
        .expect_err("a refused reference carries nothing")
        .contains(RefusalKind::LicenceRefused);
    let source_gone = intake
        .admit(&Row::new(
            "a declared row",
            Source::new("missing.md", Digest::of(source_bytes)),
            Some(Licence::Permitted),
            Some("U4"),
            Some(Acceptance::Provided(check.clone())),
            Claim::ChecksumOnly,
            Requirements::nothing(),
        ))
        .expect_err("there is nothing to consume in place")
        .contains(RefusalKind::SourceAbsent);
    let pin_mismatch = intake
        .admit(&row(
            "a declared row",
            Claim::pinned("an operator", Digest::of(b"a different release")),
        ))
        .expect_err("the payload is not the release that was pinned")
        .contains(RefusalKind::PinMismatch);
    println!(
        "U6 intake: refused by name — owed check [axis_inactive] = {owed_refused}, refused licence \
         [licence_refused] = {licence_refused}, absent source [source_absent] = {source_gone}, pinned \
         other payload [pin_mismatch] = {pin_mismatch}"
    );

    // 6. **Conformance is against the world's own declarations, not against a list of strings.** A row
    //    that requires a stage the roster *runs* and a material the registry *admits* conforms; a stage
    //    nobody declared refuses `stage_undeclared` **with the stage named**, and a material the
    //    registry never admitted refuses with the registry's own words — a refusal by name, with the
    //    specific item, rather than a bare negative. The states that are not a pass stay visibly
    //    distinct: a row that requires nothing is *reported* and never counted as met, a bar the row
    //    declares and **owes** is `inactive` under the axis's own name, and one it declares and leaves
    //    unnamed is `unchecked` — the axis nothing consulted.
    let a_stage = roster
        .stages()
        .first()
        .map(|stage| stage.name)
        .expect("the reference roster declares at least one stage");
    let requiring = |requirements: Requirements| {
        Row::new(
            "a declared row",
            source.clone(),
            Some(Licence::Permitted),
            Some("U4"),
            Some(Acceptance::Provided(check.clone())),
            Claim::ChecksumOnly,
            requirements,
        )
    };
    let conforming = intake
        .inspect(&requiring(Requirements {
            stages: Some(Requirement::Named(vec![a_stage.to_string()])),
            materials: Some(Requirement::Named(vec![MaterialId(1)])),
        }))
        .state(Axis::Conformance);
    let missing_stage = intake
        .admit(&requiring(Requirements {
            stages: Some(Requirement::Named(vec!["no-such-stage".to_string()])),
            ..Requirements::nothing()
        }))
        .expect_err("the world declares no such stage");
    let missing_material = intake
        .admit(&requiring(Requirements {
            materials: Some(Requirement::Named(vec![MaterialId(4242)])),
            ..Requirements::nothing()
        }))
        .expect_err("the world admits no such material");
    // Named, and the **specific item** is in the words: the stage's own name, and the registry's own
    // refusal (which is the owner's message, passed through rather than restated here).
    let stage_named = missing_stage.contains(RefusalKind::StageUndeclared)
        && missing_stage
            .first_named()
            .map(|refusal| refusal.detail().contains("no-such-stage"))
            .unwrap_or(false);
    let material_named = missing_material.contains(RefusalKind::UnadmittedMaterial)
        && missing_material
            .first_named()
            .map(|refusal| refusal.detail().contains("4242"))
            .unwrap_or(false);
    // The three non-pass states, one row each: nothing required, a declaration owed, a declaration
    // that names nothing. They are three different facts and the axis says which one it met.
    let nothing_required = intake
        .inspect(&requiring(Requirements::nothing()))
        .state(Axis::Conformance);
    let owed_bar = intake
        .inspect(&requiring(Requirements {
            stages: Some(Requirement::Owed),
            ..Requirements::nothing()
        }))
        .state(Axis::Conformance);
    let unnamed_bar = intake
        .inspect(&requiring(Requirements {
            stages: Some(Requirement::Named(Vec::new())),
            ..Requirements::nothing()
        }))
        .state(Axis::Conformance);
    let states_distinct = nothing_required == Some(Establishment::Inactive)
        && owed_bar == Some(Establishment::Inactive)
        && unnamed_bar == Some(Establishment::Unchecked);
    let conformance_holds = conforming == Some(Establishment::Active)
        && stage_named
        && material_named
        && states_distinct;
    println!(
        "U6 intake: conformance against the world: requiring stage {a_stage} and material 1 = {}; a \
         stage nobody declares [stage_undeclared] = {stage_named}; a material the registry never \
         admitted [unadmitted_material] = {material_named}",
        conforming.map(Establishment::name).unwrap_or("-")
    );
    println!(
        "U6 intake: conformance states: requires nothing = {}, declared and owed = {}, declared and \
         unnamed = {} (distinct = {states_distinct})",
        nothing_required.map(Establishment::name).unwrap_or("-"),
        owed_bar.map(Establishment::name).unwrap_or("-"),
        unnamed_bar.map(Establishment::name).unwrap_or("-")
    );
    println!(
        "U6 intake: conformance still OWED over a served line — the wire cannot spell an owed or \
         unnamed requirement: a `requires_…` key with nothing after it is refused by the table's own \
         shape rule (`argument_malformed`, nothing is not a value of a declared kind), exactly as an \
         owed acceptance check is unreachable while `check` is a required reference. Both states are \
         reachable through the intake's seam, and are measured above."
    );

    // 7. **The census never passes an unmeasured cell.** Two declared rows, one complete and one that
    //    answered only its source: the incomplete row is not admitted, so the gate cannot grade only
    //    what it happened to check.
    let census = intake.census(&[
        row("a complete row", Claim::ChecksumOnly),
        partial.clone(),
    ]);
    let census_holds = census.rows() == 2
        && census.cells().len() == 2 * Axis::COUNT
        && census.admitted() == 1
        && !census.is_established()
        && census.count(Establishment::Unchecked) == 2
        && census.count(Establishment::Refused) == 1
        && census.refusals().contains(RefusalKind::AxisUnchecked);
    println!(
        "U6 intake: census: {} declared row(s), {} cells, {} established / {} inactive / {} unchecked \
         / {} refused, admitted {} of {}",
        census.rows(),
        census.cells().len(),
        census.count(Establishment::Active),
        census.count(Establishment::Inactive),
        census.count(Establishment::Unchecked),
        census.count(Establishment::Refused),
        census.admitted(),
        census.rows()
    );
    for (feature, finding) in census.cells() {
        if !finding.is_pass() {
            println!("U6 intake:   [{feature}] {finding}");
        }
    }

    let _ = std::fs::remove_dir_all(&root);

    checksum_only
        && pinned_identity
        && every_axis_established
        && read_in_place
        && undeclared
        && unchecked
        && owed_refused
        && licence_refused
        && source_gone
        && pin_mismatch
        && conformance_holds
        && census_holds
}

/// U2 — the shape laws and the shaper taxonomy.
pub fn check_u2_shapes() -> bool {
    // The stalagmite's `Da` law: a vertical base, a flat cap, and the law's own cap radius.
    let stalagmite = Profile::Stalagmite { da: 2.0, radius: 1.0 };
    let base = stalagmite.normal(0.0, 0.0).unwrap_or([0.0, 0.0, 0.0]);
    let cap = stalagmite.normal(1.0, 0.0).unwrap_or([0.0, 0.0, 0.0]);
    let cap_radius = stalagmite.stalagmite_cap_radius().unwrap_or(f64::NAN);
    let law_cap = (1.0 - 1.0 / 4.0_f64).sqrt();
    let printed_cap = (1.0 - 1.0 / 2.0_f64).sqrt();

    // A malformed shape is refused, not accepted.
    let malformed_refused = Profile::Stalagmite { da: 1.0, radius: 1.0 }
        .validate()
        .err()
        .map(|r| r == RefusalKind::MalformedProfile)
        .unwrap_or(false);

    // The taxonomy: a material shaper cannot change occupancy; the 2×2 grid is fully reachable.
    let mut lattice = StoredLattice::empty();
    let cell = SubvoxelIndex::new(1, 2, 3).expect("a stored cell");
    let physical_filled = apply(ShaperKind::Physical, &mut lattice, cell, CellChange::Fill(MaterialId(4))).is_ok();
    let occupancy_after_fill = lattice.occupancy_count();
    let material_refused = apply(ShaperKind::Material, &mut lattice, cell, CellChange::Empty)
        .err()
        .map(|r| r.kind() == RefusalKind::MaterialShaperCannotChangeOccupancy)
        .unwrap_or(false);
    let occupancy_after_refusal = lattice.occupancy_count();
    let grid = ShaperSet::full().coverage();
    let grid_full = grid.iter().flatten().all(|count| *count >= 1);

    println!(
        "U2 stalagmite:  base normal = ({:.3}, {:.3}, {:.3}) [radial: {}], cap normal = ({:.0}, {:.0}, {:.0}) [up: {}]",
        base[0], base[1], base[2], (base[0] - 1.0).abs() < 1e-9,
        cap[0], cap[1], cap[2], cap == UP
    );
    println!(
        "U2 cap radius:  law {cap_radius:.4} = sqrt(1-1/Da^2) {law_cap:.4}; printed figure {printed_cap:.4} does not reproduce: {}",
        (cap_radius - printed_cap).abs() > 0.1
    );
    println!("U2 malformed profile refused: {malformed_refused}");
    println!(
        "U2 shaper:      physical fill = {physical_filled} (occupancy {occupancy_after_fill}); \
         material empty refused = {material_refused} (occupancy {occupancy_after_refusal}); \
         2x2 coverage = {grid:?} full = {grid_full}"
    );

    (base[0] - 1.0).abs() < 1e-9
        && cap == UP
        && (cap_radius - law_cap).abs() < 1e-12
        && (cap_radius - printed_cap).abs() > 0.1
        && malformed_refused
        && physical_filled
        && occupancy_after_fill == 1
        && material_refused
        && occupancy_after_refusal == 1
        && grid_full
}

/// U2 — the append-only chain: links, tickets, and a wire form that round-trips.
pub fn check_u2_chain() -> bool {
    let mut chain = Chain::new();
    let _ = chain.append(Namespace::World, EventKind::WorldOpened, "seed=2468");
    let _ = chain.append(Namespace::World, EventKind::TickAdvanced, "tick=1");
    let ticket = chain.file_ticket("the spawn surface reads as one pale material");
    let ticket_id = ticket.unwrap_or(TicketId(u64::MAX));
    let resolved = chain.resolve_ticket(ticket_id).is_ok();
    let double_resolve_refused = chain
        .resolve_ticket(ticket_id)
        .err()
        .map(|r| r.kind() == RefusalKind::TicketAlreadyResolved)
        .unwrap_or(false);
    let unknown_refused = chain
        .resolve_ticket(TicketId(999))
        .err()
        .map(|r| r.kind() == RefusalKind::UnknownTicket)
        .unwrap_or(false);
    let verified = chain.verify().is_ok();

    // The wire form round-trips exactly, and a tampered payload is refused.
    let wire = chain.wire();
    let round_trips = Chain::from_wire(&wire).map(|parsed| parsed == chain).unwrap_or(false);
    let tampered_refused = Chain::from_wire(&wire.replacen("tick=1", "tick=7", 1))
        .err()
        .map(|set| set.contains(RefusalKind::ChainBroken))
        .unwrap_or(false);

    println!(
        "U2 chain:       {} events, head {}, verified = {verified}",
        chain.len(),
        chain.head().map(|d| d.to_hex()).unwrap_or_else(|| "none".to_string())
    );
    println!(
        "U2 chain:       ticket {} filed+resolved = {resolved}, double-resolve refused = {double_resolve_refused}, \
         unknown refused = {unknown_refused}",
        ticket_id
    );
    println!("U2 chain wire:  {round_trips} exact round trip, tamper refused = {tampered_refused}");

    chain.len() == 4
        && resolved
        && double_resolve_refused
        && unknown_refused
        && verified
        && round_trips
        && tampered_refused
}

/// U9 — the written world: a save that reloads to the world it was written from, with the **five**
/// seed systems the mission names in the plural carried rather than one dial renamed.
///
/// The evidence `specs/006-world-saves-and-seeds/spec.md` asks for, **printed** rather than asserted
/// in private: a save written to a real file and read back with its byte length, the five declared
/// stream seeds each checked against the world's own, the loaded world's truth digest against the
/// live world's (SC-001/SC-002), one generation digest at 1/2/4/8 workers (SC-001), the record it
/// carries and declares (SC-006), and the refusals the **public** surface can reach (SC-005). Two
/// faults this build refuses — `seed_undeclared` and `record_diverged` — cannot be produced through
/// the public surface at all (the writer will not emit a save it would refuse), so they are named as
/// pinned by the crate's own writer tests instead of being silently omitted.
pub fn check_u9() -> bool {
    let roster = match process::roster() {
        Ok(roster) => roster,
        Err(refusals) => {
            eprintln!("U9 roster refused by name: {refusals}");
            return false;
        }
    };
    let identity = match reference::identity() {
        Ok(identity) => identity,
        Err(refusal) => {
            eprintln!("U9 reference identity refused by name: {refusal}");
            return false;
        }
    };

    let tick = Tick(3);
    let live = World::new(identity);
    let world = live.with_tick(tick);
    let mut chain = Chain::new();
    for (namespace, kind, payload) in [
        (
            Namespace::World,
            EventKind::WorldOpened,
            format!("seed={} period={}s", identity.seed, identity.period.get()),
        ),
        (
            Namespace::Record,
            EventKind::TickAdvanced,
            format!("tick={}", tick.0),
        ),
    ] {
        if let Err(refusal) = chain.append(namespace, kind, payload) {
            eprintln!("U9 could not append to the reference record: {refusal}");
            return false;
        }
    }

    // The reference world's ruleset, from its one declaration — the same value the save-load checks
    // below and the crate's own tests use, so a mismatch here is a real one rather than a typo.
    let ruleset = reference::ruleset();
    let bytes = match Save::of(&world, &chain, ruleset) {
        Ok(bytes) => bytes,
        Err(refusals) => {
            eprintln!("U9 could not write the world down: {refusals}");
            return false;
        }
    };

    // **A real file, written and read back.** "A world written down" is a file, not a buffer in the
    // process that wrote it.
    let path = std::env::temp_dir().join("shploingus-u9.save");
    if let Err(error) = std::fs::write(&path, &bytes) {
        eprintln!("U9 could not write {path:?}: {error}");
        return false;
    }
    let read_back = match std::fs::read(&path) {
        Ok(read_back) => read_back,
        Err(error) => {
            eprintln!("U9 could not read {path:?} back: {error}");
            return false;
        }
    };
    let save = match Save::parse(&read_back) {
        Ok(save) => save,
        Err(refusals) => {
            eprintln!("U9 could not parse its own save: {refusals}");
            return false;
        }
    };
    let loaded = match save.load(ruleset) {
        Ok(loaded) => loaded,
        Err(refusals) => {
            eprintln!("U9 could not load its own save: {refusals}");
            return false;
        }
    };

    let exact = read_back == bytes && save.bytes() == bytes;
    let truth_agrees = loaded.report.truth == world.truth_digest();
    let record_agrees = loaded.chain.events() == chain.events()
        && loaded.chain.head() == chain.head()
        && loaded.report.record_len == chain.len()
        && loaded.report.record_head == chain.head();
    let streams_agree = save
        .streams()
        .iter()
        .all(|stream| stream.seed == world.stream_seed(stream.domain));

    println!(
        "U9 save:          {} bytes at {path:?} (the file and the re-encoding are the bytes written = {exact})",
        bytes.len()
    );
    println!(
        "U9 identity:      seed={} period={}s tick={}  ruleset {} revision {}",
        save.identity().seed,
        save.identity().period,
        save.identity().tick,
        save.ruleset().ruleset,
        save.ruleset().revision
    );
    for stream in save.streams() {
        println!(
            "U9 stream seed:   {:<9} {} (= the world's own: {})",
            stream.domain.name(),
            stream.seed,
            stream.seed == world.stream_seed(stream.domain)
        );
    }
    println!(
        "U9 streams:       {} of {} declared, each the world's own = {streams_agree}",
        save.streams().len(),
        Domain::COUNT
    );
    println!(
        "U9 record:        {} event(s), head {} — declared and carried agree = {record_agrees}",
        loaded.report.record_len,
        loaded
            .report
            .record_head
            .map(|digest| digest.to_hex())
            .unwrap_or_else(|| "-".to_string())
    );
    println!(
        "U9 loaded truth:  {} (the live world at this tick: {} — equal = {truth_agrees}, and the previous tick differs = {})",
        loaded.report.truth.to_hex(),
        world.truth_digest().to_hex(),
        live.truth_digest() != loaded.report.truth
    );

    // **One digest per world at 1/2/4/8 workers** — the instrument U3 measured and U4 repeated,
    // applied to the identity the save carries: the rebuilt world is the ticked world (SC-001).
    let mut digests = Vec::new();
    let mut workers_agree = true;
    for workers in [1_usize, 2, 4, 8] {
        match process::world_generation_digest_with_workers(
            &roster,
            loaded.world.identity(),
            loaded.world.tick(),
            process::declared_request(),
            workers,
        ) {
            Ok(digest) => {
                println!("U9 rebuilt at {workers} worker(s): {}", digest.to_hex());
                if let Some(first) = digests.first() {
                    workers_agree &= *first == digest;
                }
                digests.push(digest);
            }
            Err(refusal) => {
                eprintln!("U9 generation at {workers} worker(s) refused: {refusal}");
                workers_agree = false;
            }
        }
    }
    let rebuilt_is_ticked = !digests.is_empty() && truth_agrees;
    println!(
        "U9 worker knob:   {} digest(s) at 1/2/4/8 workers, one value = {workers_agree}; rebuilt == ticked = {rebuilt_is_ticked}",
        digests.len()
    );

    // **The refusals the public surface reaches**, each by its own name.
    let not_a_save = Save::parse(b"not a save at all");
    let truncated = Save::parse(&bytes[..60]);
    let mut altered = bytes.clone();
    altered[40] ^= 0x01;
    let tampered = Save::parse(&altered);
    let other_world = save.load(Ruleset::new(8, 3));
    let named = |result: &Result<Save, crate::refusal::RefusalSet>, kind: RefusalKind| match result {
        Ok(_) => false,
        Err(refusals) => refusals.contains(kind),
    };
    let not_a_save = named(&not_a_save, RefusalKind::NotASave);
    let truncated = named(&truncated, RefusalKind::SaveTruncated);
    let tampered = named(&tampered, RefusalKind::SaveTampered);
    let another_ruleset = match &other_world {
        Ok(_) => false,
        Err(refusals) => refusals.contains(RefusalKind::IdentityMismatch),
    };
    println!(
        "U9 refusals:      not_a_save = {not_a_save}, save_truncated = {truncated}, save_tampered = {tampered}, \
         identity_mismatch (another ruleset) = {another_ruleset}"
    );
    println!(
        "U9 refusals:      seed_undeclared and record_diverged are pinned by the crate's own writer tests \
         (src/save/tests.rs) — the public writer will not emit a save it would refuse"
    );

    // **The document form** — landed 2026-10-12 (§28), and measured here as the claim it was owed on:
    // `plan.md` says the form *"waits on U6's content gate"* and §16.2 rules that *"the gate becomes a
    // **hard** prerequisite only for the other form of a save: one that crosses the API as a document
    // consumed in place by reference"*. What follows is that whole claim: a world is written down at a
    // declared reference, read back **through the gate's own reader**, and the two save rows of the one
    // tool table are the doors — so there is no path by which a save document is read without the gate.
    let mut door = match reference::gate(&roster, 1) {
        Ok(gate) => gate,
        Err(refusal) => {
            eprintln!("U9 document: the reference declaration is refused by name: {refusal}");
            return false;
        }
    };
    // The world the replay form above was written from, reached through the gate's own seams: the same
    // identity, the same tick, the same two events — so the two forms' numbers are comparable rather
    // than merely both true.
    for _ in 0..tick.0 {
        door.world_mut().advance();
    }
    for (namespace, kind, payload) in [
        (
            Namespace::World,
            EventKind::WorldOpened,
            format!("seed={} period={}s", identity.seed, identity.period.get()),
        ),
        (Namespace::Record, EventKind::TickAdvanced, format!("tick={}", tick.0)),
    ] {
        if let Err(refusal) = door.chain_mut().append(namespace, kind, payload) {
            eprintln!("U9 document: could not append to the door's record: {refusal}");
            return false;
        }
    }
    let document_root = std::env::temp_dir().join("shploingus-u9-document");
    let _ = std::fs::remove_dir_all(&document_root);
    if let Err(error) = std::fs::create_dir_all(&document_root) {
        eprintln!("U9 document: the reference root could not be made: {error}");
        return false;
    }
    door.set_intake_root(&document_root);

    let written = match door.call("save path=the-world.save") {
        Ok(receipt) => receipt,
        Err(refusal) => {
            eprintln!("U9 document: the `save` row refused by name: {refusal}");
            return false;
        }
    };
    let written_facts = match &written.answer {
        Answer::Saved(facts) => facts.clone(),
        other => {
            eprintln!("U9 document: `save` answered with {other:?}, not a written document");
            return false;
        }
    };
    let read_back = match door.call(&format!(
        "load path=the-world.save digest={}",
        written_facts.digest.to_hex()
    )) {
        Ok(receipt) => receipt,
        Err(refusal) => {
            eprintln!("U9 document: the `load` row refused by name: {refusal}");
            return false;
        }
    };
    let read_facts = match &read_back.answer {
        Answer::Loaded(facts) => facts.clone(),
        other => {
            eprintln!("U9 document: `load` answered with {other:?}, not a loaded document");
            return false;
        }
    };
    // The **library's** own door onto the same document — U9's document form read through the gate the
    // rows just went through, so "one reader" is a measurement and not a description.
    let (library_save, library_document) = match Save::read_document(
        door.intake(),
        &Source::new(written_facts.path.as_str(), written_facts.digest),
    ) {
        Ok((save, document)) => (save, document),
        Err(refusals) => {
            eprintln!("U9 document: the library's own door refused by name: {refusals}");
            return false;
        }
    };
    let receipts_hold = written.verify(&door.owners()).is_ok()
        && read_back.verify(&door.owners()).is_ok();
    let one_value = read_facts == written_facts;
    let loaded_is_live = read_facts.truth == door.world().truth_digest()
        && read_facts.record_head == door.chain().head()
        && read_facts.record_len == door.chain().len();
    let library_agrees = library_document.digest() == written_facts.digest
        && library_document.len() == written_facts.len
        && library_save.identity().seed == written_facts.seed
        && library_save.bytes().len() == written_facts.len;
    println!(
        "U9 document:      `save` wrote {} byte(s) at {} ({}), seed={} period={}s tick={} ruleset {} \
         revision {} — its receipt verifies (claims re-established from the request, not believed) = \
         {receipts_hold}",
        written_facts.len,
        written_facts.path,
        written_facts.digest.to_hex(),
        written_facts.seed,
        written_facts.period_s,
        written_facts.tick,
        written_facts.ruleset.ruleset,
        written_facts.ruleset.revision,
    );
    println!(
        "U9 document:      `load` read them back by reference: truth={} — the live world's {} (equal = \
         {loaded_is_live}); the write and the read are one value = {one_value}; the library's own door \
         reads the same document = {library_agrees}",
        read_facts.truth.to_hex(),
        door.world().truth_digest().to_hex(),
    );

    // **Three refusals by name, each through the same door.** An absent document, a document that is
    // not the one the reference names (the bytes moved), and bytes that *are* the declared ones and are
    // not the save they claim (the trailer disagrees) — the first two the content gate's own names, the
    // third U9's, and every one of them recorded on the door's record because each line parsed.
    let before_document_refusals = door.chain().len();
    let absent = match door.call(&format!(
        "load path=gone.save digest={}",
        written_facts.digest.to_hex()
    )) {
        Ok(_) => {
            eprintln!("U9 document: a reference to a document that is not there was loaded");
            return false;
        }
        Err(refusal) => refusal,
    };
    let mut altered_bytes = match Save::of(door.world(), door.chain(), ruleset) {
        Ok(bytes) => bytes,
        Err(refusals) => {
            eprintln!("U9 document: the world could not be written down for the tamper: {refusals}");
            return false;
        }
    };
    let last = altered_bytes.len() - 1;
    altered_bytes[last] ^= 0x01;
    let altered_digest = Digest::of(&altered_bytes);
    for (name, bytes) in [
        ("tampered.save", &altered_bytes),
        ("the-world.save", &altered_bytes),
    ] {
        if let Err(error) = std::fs::write(document_root.join(name), bytes) {
            eprintln!("U9 document: {name} could not be written: {error}");
            return false;
        }
    }
    let altered = match door.call(&format!(
        "load path=tampered.save digest={}",
        altered_digest.to_hex()
    )) {
        Ok(_) => {
            eprintln!("U9 document: bytes altered inside the save's own trailer were loaded");
            return false;
        }
        Err(refusal) => refusal,
    };
    let moved = match door.call(&format!(
        "load path=the-world.save digest={}",
        written_facts.digest.to_hex()
    )) {
        Ok(_) => {
            eprintln!("U9 document: a document whose bytes had moved was loaded");
            return false;
        }
        Err(refusal) => refusal,
    };
    let absent_named = absent.kind() == RefusalKind::SourceAbsent
        && absent.detail().contains("gone.save");
    let altered_named = altered.kind() == RefusalKind::SaveTampered;
    let moved_named = moved.kind() == RefusalKind::SourceDigestMismatch
        && moved.detail().contains(&written_facts.digest.to_hex())
        && moved.detail().contains(&altered_digest.to_hex());
    let three_named = absent_named && altered_named && moved_named;
    let recorded_refusals = door.chain().len() == before_document_refusals + 3;
    let record_still_verifies = door.chain().verify().is_ok();
    println!(
        "U9 document refusals: an absent document [{}], bytes altered inside the save's own trailer \
         [{}], and a document that is not the one the reference names [{}] — each by its own name = \
         {three_named}, each recorded = {recorded_refusals}, and the record still verifies = \
         {record_still_verifies}",
        absent.kind().name(),
        altered.kind().name(),
        moved.kind().name()
    );
    println!(
        "U9 owed:          a save of a **resident set** (a save is the record, not the residency — F1), \
         compression, and a versioned migration path"
    );

    exact
        && truth_agrees
        && record_agrees
        && streams_agree
        && workers_agree
        && rebuilt_is_ticked
        && not_a_save
        && truncated
        && tampered
        && another_ruleset
        && receipts_hold
        && one_value
        && loaded_is_live
        && library_agrees
        && three_named
        && recorded_refusals
        && record_still_verifies
}

/// **U10 — the gameplay tree the mods references imply.** One **entry per reference** in
/// `docs/MODS-MAP.md` §1–§5 with the record's own four-word disposition for each, and one **row per
/// technology** under the record's double-cover rule — every row U6's own row, whose document is read
/// **in place** and admitted through the gate the way an `admit` row is.
///
/// What is measured is the plan's own bar (`docs/CLEAN-ROOM-PLAN-2026-10-11.md` §5 row 10): *"every row
/// names its document, its licence verdict and the system that carries it; a refused reference is refused
/// by name; the tree's first row is playable"* — with the census counting exactly what **was** and was not
/// established, an owed reference never admitted, and the four parts the record has not decided printed as
/// **owed**.
pub fn check_u10() -> bool {
    use crate::gameplay::{
        Disposition, Entry, Technology, Tree, HARVEST_DIGEST, MAP_DIGEST, MAP_PATH, SHAPERS_DIGEST,
        SHAPERS_PATH, U4_QUICKSTART_DIGEST, U4_QUICKSTART_PATH,
    };
    use crate::intake::{Acceptance, Intake, Row, Source, WorldDeclarations};

    let tree = match Tree::of_record() {
        Ok(tree) => tree,
        Err(refusals) => {
            eprintln!("U10 the tree is refused by name: {refusals}");
            return false;
        }
    };

    println!(
        "U10 tree: {} entries, one per reference in `{MAP_PATH}` §1–§5 in the record's own order, and {} \
         rows, one per technology",
        tree.entries().len(),
        tree.rows().len()
    );
    println!(
        "U10 documents: `{MAP_PATH}` ({}) · `docs/MODS-MATERIALS.md` ({}) · `{SHAPERS_PATH}` ({}) · \
         `{U4_QUICKSTART_PATH}` ({}) — each row reads its own where it lives, never a copy",
        &MAP_DIGEST[..16],
        &HARVEST_DIGEST[..16],
        &SHAPERS_DIGEST[..16],
        &U4_QUICKSTART_DIGEST[..16]
    );

    let census = tree.declared();
    // The census prints the map's own per-section split, and that split is the map's: it was derived from
    // the §1–§5 rows where they live when the tree was built (`tree.sections()`), never written down here.
    println!("U10 census: {}", census.render(tree.sections()));
    for entry in tree.entries() {
        println!("U10 entry: {}", entry.render());
    }

    let double_covered = tree.double_covered();
    println!(
        "U10 rows: {} technologies, one row each; {} carry more than one citation (the record's own \
         double-cover rule) and {} declare their own acceptance check as owed",
        tree.rows().len(),
        double_covered.len(),
        tree.rows_owed().len()
    );
    for technology in tree.rows() {
        println!("U10 row: {}", technology.render());
    }

    // **The gate.** The tree's rows are U6's own rows, admitted the way any row is: the document read
    // where it lives, the licence verdict the row relies on, the carrier declared, and its own check at
    // its own reference — while the last two axes are **reported** rather than demanded (a checksum
    // establishes no origin, and a row that requires nothing of the world asked nothing).
    let roster = match process::roster() {
        Ok(roster) => roster,
        Err(refusals) => {
            eprintln!("U10 the reference roster is refused by name: {refusals}");
            return false;
        }
    };
    let registry = reference::registry();
    let root = crate::gameplay::record_root();
    let intake = Intake::over(&root, WorldDeclarations { roster: &roster, registry });
    let rows: Vec<Row> = tree.rows().iter().map(|technology| technology.row().clone()).collect();
    let gate = intake.census(&rows);
    let summary = gate.render();
    println!(
        "U10 gate: {}",
        summary.lines().next().unwrap_or("(no census)").trim_start_matches("census: ")
    );
    let named: Vec<String> = RefusalKind::ALL
        .iter()
        .filter_map(|kind| {
            let count = gate.refusals().all().iter().filter(|refusal| refusal.kind() == *kind).count();
            match count {
                0 => None,
                count => Some(format!("{}×{count}", kind.name())),
            }
        })
        .collect();
    println!(
        "U10 refusals by name: {} — every row the world refused is named by the owner's own name",
        named.join(", ")
    );

    // The bar's first clause: **every row names its document, its licence verdict and the system that
    // carries it.**
    let declared: Vec<&Row> = tree.entries().iter().filter_map(Entry::row).collect();
    let every_row_names_its_declaration = declared.iter().all(|row| {
        !row.source().path().is_empty() && row.licence().is_some() && row.carrier().is_some()
    });
    println!(
        "U10 bar: {} row(s) declared by {} entr(ies); every row names its document, its licence verdict \
         and its carrier = {every_row_names_its_declaration}",
        declared.len(),
        tree.entries().len()
    );

    // The bar's second clause: **a refused reference is refused by name** — the gate's own
    // `licence_refused`, passed through rather than re-worded — while its permitted sibling admits.
    let refused_entry = tree.entries().iter().find(|entry| entry.disposition() == Disposition::Refused);
    let refused_by_name = match refused_entry.and_then(Entry::row).map(|row| intake.admit(row)) {
        Some(Err(refusals)) => refusals.contains(RefusalKind::LicenceRefused),
        _ => false,
    };
    let head_row = tree.first().and_then(Entry::row);
    let head = head_row.map(|row| intake.admit(row));
    let head_admitted = matches!(head, Some(Ok(_)));
    println!(
        "U10 bar: the refused reference `{}` is refused by name (`licence_refused`) = {refused_by_name}; \
         the permitted head `{}` admits = {head_admitted}",
        refused_entry.map(Entry::reference).unwrap_or("(none)"),
        tree.first().map(Entry::reference).unwrap_or("(none)")
    );
    if let Some(Ok(established)) = &head {
        println!(
            "U10 bar: the head's row was admitted as {} — read in place at {}, identity established = {}",
            established.claim_name(),
            established.found().to_hex(),
            established.establishes_identity()
        );
    }

    // **An owed reference is never admitted** — it cannot be, because a reference with no feature is not
    // a row at all (`feature` is a required argument of the table's `admit`) — and the census counts it.
    let owed_never_admitted = tree.entries().iter().filter(|entry| entry.is_owed()).all(|entry| entry.row().is_none());
    println!(
        "U10 owed references: {} of {} entries are named owed and carry no row (never admitted = \
         {owed_never_admitted})",
        tree.entries().iter().filter(|entry| entry.is_owed()).count(),
        tree.entries().len()
    );

    // **The tree over a tree of this process's own.** The row is the tree's own first row; only the bytes
    // it points at move, and both ends of the fact are named.
    let fixture = std::env::temp_dir().join("shploingus-u10-tree-evidence");
    let _ = std::fs::remove_dir_all(&fixture);
    let copy = |path: &str| -> std::io::Result<()> {
        let target = fixture.join(path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(root.join(path), &target).map(|_| ())
    };
    if let Err(error) = copy(MAP_PATH).and_then(|()| copy(U4_QUICKSTART_PATH)) {
        eprintln!("U10 the fixture could not be made: {error}");
        return false;
    }
    let fixture_intake = Intake::over(&fixture, WorldDeclarations { roster: &roster, registry });
    let same_row_admits = head_row.map(|row| fixture_intake.admit(row).is_ok()).unwrap_or(false);
    let mut moved = std::fs::read(fixture.join(MAP_PATH)).unwrap_or_default();
    moved.push(b'\n');
    if std::fs::write(fixture.join(MAP_PATH), &moved).is_err() {
        eprintln!("U10 the fixture could not be moved");
        return false;
    }
    let moved = head_row.map(|row| fixture_intake.admit(row));
    let (moved_refused, moved_detail) = match &moved {
        Some(Err(refusals)) => (
            refusals.contains(RefusalKind::SourceDigestMismatch),
            refusals.first_named().map(|refusal| refusal.detail().to_string()).unwrap_or_default(),
        ),
        _ => (false, "(admitted, which no moved document may be)".to_string()),
    };
    let absent = head_row.map(|row| {
        Row::new(
            row.feature(),
            Source::new("docs/not-a-document.md", row.source().digest()),
            row.licence(),
            row.carrier(),
            row.acceptance().cloned(),
            row.claim().clone(),
            row.requirements().clone(),
        )
    });
    let absent = absent.map(|row| fixture_intake.admit(&row));
    let (absent_refused, absent_detail) = match &absent {
        Some(Err(refusals)) => (
            refusals.contains(RefusalKind::SourceAbsent),
            refusals.first_named().map(|refusal| refusal.detail().to_string()).unwrap_or_default(),
        ),
        _ => (false, "(admitted, which no absent document may be)".to_string()),
    };
    println!(
        "U10 in place: the same row over the same bytes admits = {same_row_admits}; one byte moves and \
         the same row refuses `source_digest_mismatch` = {moved_refused} ({moved_detail}); a reference \
         that is not there refuses `source_absent` = {absent_refused} ({absent_detail})"
    );

    // **The double-cover rule, and the one name it is owed.** An older mod and its Create-addon
    // descendant are two citations on **one** row; a second row for a technology the tree already holds is
    // refused by the closest declared name, with the exact name the record has not given printed as owed.
    let coking = tree.technology("coking");
    let coking_is_one_row = coking.is_some_and(|technology| {
        technology.cited_by() == ["Railcraft", "Create-Pantographs-and-Wires"]
            && tree.rows().iter().filter(|other| other.feature() == "coking").count() == 1
    });
    let mut grown = tree.clone();
    let mut duplicate_refused = false;
    let mut gap = String::new();
    if let Some(held) = coking {
        let row = held.row();
        // The descendant declaring its **own** bar for the same rule — which is exactly what the record's
        // rule forbids.
        let second = Row::new(
            row.feature(),
            row.source().clone(),
            row.licence(),
            row.carrier(),
            Some(Acceptance::Provided(Source::new(
                SHAPERS_PATH,
                Digest::from_hex(SHAPERS_DIGEST).expect("a declared digest is a digest"),
            ))),
            row.claim().clone(),
            row.requirements().clone(),
        );
        match Entry::new("a-descendant", Disposition::Mapped, Some(second), None) {
            Ok(entry) => match grown.declare(entry) {
                Ok(()) => eprintln!("U10 a second row for `coking` was accepted, which it may not be"),
                Err(refusals) => {
                    duplicate_refused = refusals.contains(RefusalKind::DuplicateOutput);
                    gap = refusals
                        .first_named()
                        .map(|refusal| refusal.detail().to_string())
                        .unwrap_or_default();
                }
            },
            Err(shape) => {
                eprintln!("U10 the duplicate-row witness could not be built: {shape}");
                return false;
            }
        }
    }
    println!(
        "U10 double cover: `coking` is one row cited by Railcraft and Create-Pantographs-and-Wires = \
         {coking_is_one_row}; {} row(s) carry more than one citation; a second row for a held technology \
         is refused = {duplicate_refused} — the closest declared name is `duplicate_output` and the \
         **exact name is owed**: {gap}",
        double_covered.len()
    );
    let grown_kept_one_row = grown.rows().len() == tree.rows().len();
    println!(
        "U10 double cover: the refused second row grew no row ({grown_kept_one_row}); the record's own \
         witnesses: {}",
        double_covered
            .iter()
            .map(|technology: &&Technology| format!("{}={}", technology.feature(), technology.cited_by().join("+")))
            .collect::<Vec<_>>()
            .join(", ")
    );

    for part in Tree::owed() {
        println!("U10 OWED: {part}");
    }

    census.sums_to_entries()
        && every_row_names_its_declaration
        && refused_by_name
        && head_admitted
        && owed_never_admitted
        && same_row_admits
        && moved_refused
        && absent_refused
        && coking_is_one_row
        && duplicate_refused
        && grown_kept_one_row
        && gate.rows() == tree.rows().len()
}

/// **U5** — instrumentation, light and audio.
///
/// Row 5 of the plan's §5, read against the record's own carrying slots (`docs/DESIGN-SPEC.md` §3:
/// S-03, S-12, S-13, FC-2.16), `docs/light-solver.md` and `docs/LEDGER.md` C-94. The bar's own
/// clauses: one sensor concept with the wall as a consumer of it, a band per item carrying min and
/// max, a weighing gate at a boundary whose answer is a mass and nothing else, a virtual gauge wall
/// in one controller, `Solver`/`Spectra`, the audio seam, and **a quantity that cannot be weighed is
/// refused by name**.
pub fn check_u5() -> bool {
    let registry = MaterialRegistry::starter();
    let quartz = MaterialId(4);
    let basalt = MaterialId(3);

    // --- the instruments (S-03, S-12, S-13, FC-2.16) ---------------------------------------------
    let stock = Site::Stock([12, 64, -3]);
    let boundary = WeighingGate::at([12, 65, -3]);
    let band = match Band::of(32, 96) {
        Ok(band) => band,
        Err(refusal) => {
            eprintln!("U5: the fixture's own band is refused by name: {refusal}");
            return false;
        }
    };
    let mut wall = GaugeWall::new();
    wall.watch(stock, quartz, band);
    let small = match Band::of(0, 8) {
        Ok(band) => band,
        Err(refusal) => {
            eprintln!("U5: the fixture's second band is refused by name: {refusal}");
            return false;
        }
    };
    wall.watch(Site::Stock([0, 64, 0]), basalt, small);
    wall.watch(Site::Boundary(boundary.boundary()), quartz, band);

    let readings = [
        (64u64, "within"),
        (31, "below"),
        (97, "above"),
    ];
    let mut verdicts = Vec::new();
    for (count, expected) in readings {
        match wall.read(&Sensor::count(stock, quartz, count, 7)) {
            Ok(reading) => {
                let word = reading.verdict().word();
                let mark = if word == expected { "" } else { " WRONG" };
                verdicts.push(format!("{count}→{word}{mark}"));
            }
            Err(refusal) => {
                eprintln!("U5: the wall refused a reading it watches: {refusal}");
                return false;
            }
        }
    }
    let gauges = wall.len();
    let band_ok = wall
        .read(&Sensor::count(stock, quartz, 64, 7))
        .map(|reading| reading.band() == band && reading.is_satisfied() && reading.count_of() == Some(64))
        .unwrap_or(false)
        && verdicts.iter().all(|verdict| !verdict.contains("WRONG"));

    let crossed = Crossing { material: quartz, count: 3 };
    let mass = boundary.weigh(&crossed, &registry);
    let basalt_mass = boundary.weigh(&Crossing { material: basalt, count: 64 }, &registry);
    let (mass_ok, mass_line) = match (&mass, &basalt_mass) {
        (Ok(quartz_mass), Ok(basalt_mass)) => (
            quartz_mass.grams() == (7_950_000, 1) && basalt_mass.grams() == (192_000_000, 1),
            format!(
                "3 units of quartz = {}/{} g exactly; 64 units of basalt = {}/{} g",
                quartz_mass.grams().0,
                quartz_mass.grams().1,
                basalt_mass.grams().0,
                basalt_mass.grams().1
            ),
        ),
        _ => (false, "the gate refused a crossing it must weigh".to_string()),
    };

    // **A quantity that cannot be weighed is refused by name** — and so are the other four cases,
    // each in its own name rather than one symptom for all of them.
    let nothing = boundary.weigh(&Crossing { material: quartz, count: 0 }, &registry).is_err();
    let nothing_name = boundary
        .weigh(&Crossing { material: quartz, count: 0 }, &registry)
        .err()
        .map(|refusal| refusal.kind() == RefusalKind::NotWeighable)
        .unwrap_or(false);
    let unadmitted = boundary
        .weigh(&Crossing { material: MaterialId(9999), count: 1 }, &registry)
        .err()
        .map(|refusal| refusal.kind() == RefusalKind::UnadmittedMaterial)
        .unwrap_or(false);
    let mass_reading = mass
        .as_ref()
        .map(|mass| Sensor::mass(stock.position(), quartz, *mass, 7))
        .unwrap_or_else(|_| Sensor::count(stock, quartz, 0, 7));
    let not_a_count = wall
        .read(&mass_reading)
        .err()
        .map(|refusal| refusal.kind() == RefusalKind::NotACount)
        .unwrap_or(false);
    let unwatched = wall
        .read(&Sensor::count(Site::Stock([40, 64, 40]), quartz, 4, 7))
        .err()
        .map(|refusal| refusal.kind() == RefusalKind::Unwatched)
        .unwrap_or(false);
    let inverted = Band::of(9, 3)
        .err()
        .map(|refusal| refusal.kind() == RefusalKind::BandInverted)
        .unwrap_or(false);
    let refusals_ok = nothing && nothing_name && unadmitted && not_a_count && unwatched && inverted;

    println!(
        "U5 instruments: {gauges} virtual gauges in one controller, band {}..={} — {}",
        band.min(),
        band.max(),
        verdicts.join(", ")
    );
    println!("U5 weighing gate: {mass_line} (a mass, and nothing else)");
    println!(
        "U5 refusals by name: not_weighable={nothing_name} (nothing crossed), unadmitted_material={unadmitted}, \
         not_a_count={not_a_count} (a mass at a band), unwatched={unwatched}, band_inverted={inverted}"
    );

    // --- the light (Solver / Spectra) --------------------------------------------------------------
    let spectra = Spectra::standard();
    let band_total: f64 = spectra.sun_bands().iter().sum();
    let flat = spectra.response(&[1.0; CHANNEL_COUNT]);
    let sun = spectra.sun_response();
    let flat_spread = flat.iter().copied().fold(f64::MIN, f64::max)
        - flat.iter().copied().fold(f64::MAX, f64::min);
    let sun_spread = sun.iter().copied().fold(f64::MIN, f64::max)
        - sun.iter().copied().fold(f64::MAX, f64::min);

    let noon = Solver::new(Spectra::standard(), State::clear(60.0, -30.0, 0.0));
    let sky = noon.sky_radiance(40.0);
    let sky_colour = spectra.response(&sky);
    let fog = Solver::new(
        Spectra::standard(),
        State {
            droplets: Some(Droplets::fog(FOG_THRESHOLD_LWC_G_M3)),
            ..State::clear(60.0, -30.0, 0.0)
        },
    );
    let beam_clear: f64 = noon.direct_beam().iter().sum();
    let beam_fog: f64 = fog.direct_beam().iter().sum();
    let sky_clear: f64 = sky.iter().sum();
    let sky_fog: f64 = fog.sky_radiance(40.0).iter().sum();
    let visibility = Droplets::fog(FOG_THRESHOLD_LWC_G_M3).visibility_km();
    let blue_over_red = sky_colour[2] / sky_colour[0];
    // **The calibration's own target, pinned as a literal** (the published `1 361 W/m²`): the bands
    // are scaled *to* `SOLAR_CONSTANT_W_M2`, so comparing the two to each other passes for any
    // constant; the pin is what makes a moved constant fail by name here.
    let constant_pin = (band_total - SOLAR_CONSTANT_W_M2).abs() < 1e-9
        && (SOLAR_CONSTANT_W_M2 - 1_361.0).abs() < 1e-9;
    let light_ok = constant_pin
        && flat_spread < 1e-5
        && sun_spread < 0.05
        && blue_over_red > 1.0
        && beam_fog < beam_clear
        && sky_fog > sky_clear;
    // Empty on a good run — the stdout stays byte-stable — and a named marker when the pin fails.
    let pin_mark = if constant_pin { "" } else { " WRONG (the calibration's published target moved)" };
    println!(
        "U5 spectra: {CHANNEL_COUNT} bands over {VISIBLE_LO_NM:.0}–{VISIBLE_HI_NM:.0} nm, {band_total:.6} W/m² \
         (the solar constant){pin_mark}; flat spectrum → ({:.5}, {:.5}, {:.5}) spread {flat_spread:.2e}; the Sun → \
         ({:.3}, {:.3}, {:.3})",
        flat[0], flat[1], flat[2], sun[0], sun[1], sun[2]
    );
    println!(
        "U5 sky: Sun 60°, ray 40° up → ({:.3}, {:.3}, {:.3}) — blue over red {blue_over_red:.1}×, from λ^-4.09 alone",
        sky_colour[0], sky_colour[1], sky_colour[2]
    );
    println!(
        "U5 fog: {FOG_THRESHOLD_LWC_G_M3} g/m³ → {visibility:.3} km visibility; the beam {beam_clear:.2} → \
         {beam_fog:.2} W/m² while the sky {sky_clear:.4} → {sky_fog:.4} — light moved into the line of sight"
    );

    // The twilight table, the reference's own five rows, each beside what the solver reads.
    let mut twilight_ok = twilight_lux(0.0) == TWILIGHT_LUX_AT_HORIZON;
    let mut twilight_line = vec![format!("{:.1} lx at 0°", twilight_lux(0.0))];
    for (altitude, printed, printed_lx) in [
        (-2.0, "~80", 80.0),
        (-4.0, "~15", 15.0),
        (-6.0, "~2.5", 2.5),
        (-12.0, "~0.01", 0.01),
        (-18.0, "~0.001", 0.001),
    ] {
        let measured = twilight_lux(altitude);
        // "To the table's own rounding" is the record's own claim: each row inside a factor of two of
        // the value the reference prints.
        let within = measured <= printed_lx * 2.0 && measured >= printed_lx * 0.5;
        twilight_ok &= within;
        twilight_line.push(format!("{measured:.4} at {altitude}° (prints {printed})"));
    }
    let knee = twilight_knee_deg();
    println!("U5 twilight: {}", twilight_line.join(", "));
    println!(
        "U5 night: the curve meets the night sky's own {NIGHT_SKY_ILLUMINANCE_LX} lx at {knee:.2}°, before \
         astronomical twilight's −18°; the floor as a radiance is {:.3e} W/m²/sr",
        night_sky_radiance()
    );

    let up = Solver::new(Spectra::standard(), State::clear(0.1, -30.0, 0.0));
    let down = Solver::new(Spectra::standard(), State::clear(-0.1, -30.0, 0.0));
    let night = Solver::new(Spectra::standard(), State::clear(-18.0, -30.0, 0.0));
    let beam_up: f64 = up.direct_beam().iter().sum();
    let beam_down: f64 = down.direct_beam().iter().sum();
    let night_sky: f64 = night.sky_radiance(40.0).iter().sum();
    let beam_ok = up.direct_beam().iter().sum::<f64>() > 0.0
        && down.direct_beam() == [0.0; CHANNEL_COUNT]
        && night_sky > night_sky_radiance() * CHANNEL_COUNT as f64;
    println!(
        "U5 beam: {beam_up:.4} W/m² at +0.1°, exactly {beam_down:.1} at −0.1° (not small: zero), and the sky at \
         −18° reads {night_sky:.3e} against the night sky's own {:.3e}",
        night_sky_radiance() * CHANNEL_COUNT as f64
    );

    let full = Solver::new(Spectra::standard(), State::clear(0.0, 90.0, 1.0));
    let moon_w_m2: f64 = full.moonlight().iter().sum();
    let moon_photopic = to_lux(moon_w_m2);
    let moon_lunar = to_moon_lux(moon_w_m2);
    let doubled = Solver::new(Spectra::standard().with_sun_factor(2.0), State::clear(0.0, 90.0, 1.0));
    let doubled_w_m2: f64 = doubled.moonlight().iter().sum();
    let crescent = Solver::new(Spectra::standard(), State::clear(0.0, 90.0, 0.1));
    let crescent_w_m2: f64 = crescent.moonlight().iter().sum();
    let occluded = Solver::new(Spectra::standard(), State::clear(0.0, 90.0, 0.0));
    let moon_ok = (2.5e-3..3.5e-3).contains(&moon_w_m2)
        && (doubled_w_m2 - 2.0 * moon_w_m2).abs() < 1e-12
        && (crescent_w_m2 - moon_w_m2 * 0.1).abs() < 1e-15
        && occluded.moonlight() == [0.0; CHANNEL_COUNT];
    println!(
        "U5 moon: the full Moon {moon_w_m2:.4e} W/m² ({moon_photopic:.2} lx at 683 lm/W, {moon_lunar:.2} lx at the \
         Moon's own ≈100 lm/W), doubling the Sun doubles it ({doubled_w_m2:.4e}), a tenth-lit crescent is exactly a \
         tenth ({crescent_w_m2:.4e}), an occluded Moon is exactly zero"
    );
    println!(
        "U5 moon geometry: Ω = {:.4e} sr = π(R/d)²; the legacy's own (R/d)² = {:.4e} is π too dim, named rather \
         than inherited",
        spectra.moon_solid_angle_sr(),
        spectra.moon_solid_angle_sr() / std::f64::consts::PI
    );
    let sun_over_moon = band_total / moon_w_m2;
    println!("U5 sun ÷ moon: {sun_over_moon:.3e} (the record's own six orders: 4.5e5)");

    // --- the audio seam (LEDGER C-93.5 / C-94) -----------------------------------------------------
    let noise = crate::noise::Noise::new(0x5EED_0A57, 3);
    let measured_duty = crate::audio::duty(&noise, 32, 8);
    let weather = crate::audio::onset(&noise, 7_000, [1_234.0, -55.0]);
    let next_window = crate::audio::onset(&noise, 7_000 + ONSET_CELL_TICKS, [1_234.0, -55.0]);
    // **This field's own duty, pinned to the figure it measures** (the field is deterministic). The
    // ×4 clause below is the declared same-order sanity around the record's own figure; the pin is
    // what makes a moved threshold or a changed field a named failure — C-94's constants are read as
    // values in `tests/u5_measure.rs`.
    let duty_pin = (measured_duty - 0.0482).abs() < 1e-4;
    let audio_ok = duty_pin
        && measured_duty > ONSET_DUTY / 4.0
        && measured_duty < ONSET_DUTY * 4.0
        && weather.gain() >= 0.0
        && weather.gain() <= 1.0
        && weather.onset() != next_window.onset();
    let duty_mark = if duty_pin { "" } else { " WRONG (the field's own pinned duty moved)" };
    println!(
        "U5 audio: the onset read at (1234, −55) t7000 = {:.4} (gain {:.3}, raining {}); one value per {} ticks and \
         per {} m; this field's own duty {measured_duty:.4} against the record's own measured {ONSET_DUTY}{duty_mark}",
        weather.onset(),
        weather.gain(),
        weather.is_raining(),
        ONSET_CELL_TICKS,
        ONSET_CELL_M
    );

    // --- the voices (U5's own bar: the feeling-wheel-driven bed and the rain) ----------------------
    //
    // The wheel's own reading, then one buffer out of it. The bed's level is the reading's radius (the
    // record's own *intensity*) and the rain's is the onset field's own gain; the samples are bounded by
    // the mixer's two declared ceilings rather than by a promise; and the buffer's digest is the identity
    // of what was rendered, so a moved tone, a level or a field is a different buffer and not a
    // different-sounding one. The reading is deliberately a **drifted** one, so the print shows the peak
    // leaving the family's own band.
    let feeling = match Feeling::parse("powerful", RADIUS_STRONG as i64, -1) {
        Ok(feeling) => feeling,
        Err(refusal) => {
            eprintln!("U5: the voices' own fixture is refused: {refusal}");
            return false;
        }
    };
    // **A wet column is found in the field rather than assumed, and its absence would be said out loud.**
    // The onset field is a function of place and tick, this world rains about a twentieth of the time, and
    // the scan below is bounded: the first cell in 64 x 64 that the field reads as wet at this tick is the
    // one the rain voice is rendered from, and a field with no wet cell would fail here by name rather than
    // printing silence as if it were a measurement.
    let mut wet_cell = None;
    'cells: for cell_x in 0..64 {
        for cell_z in 0..64 {
            let at = [cell_x as f64 * ONSET_CELL_M, cell_z as f64 * ONSET_CELL_M];
            let sample = crate::audio::onset(&noise, 7_000, at);
            if sample.gain() > 0.0 {
                wet_cell = Some((cell_x, cell_z, sample));
                break 'cells;
            }
        }
    }
    let (wet_x, wet_z, wet) = match wet_cell {
        Some(cell) => cell,
        None => {
            eprintln!("U5: the field held no wet cell in 64 x 64 at t7000: the rain voice has nothing to read");
            return false;
        }
    };
    let voiced = match crate::audio::voice(Some(&feeling), &wet, &noise, crate::audio::VOICE_FRAMES) {
        Ok(voiced) => voiced,
        Err(refusal) => {
            eprintln!("U5: the voice refused a reading it must voice: {refusal}");
            return false;
        }
    };
    // **No state, or a state the record cannot calibrate, is refused by name** — four names, four
    // subjects: the reading that is not there, the word that is not declared, the reading off the ladder
    // (the wheel's own `carrying-the-loss`), and the reading that contradicts itself.
    let absent = matches!(
        crate::audio::voice(None, &wet, &noise, crate::audio::VOICE_FRAMES),
        Err(refusal) if refusal.kind() == RefusalKind::FeelingAbsent
    );
    let undeclared = matches!(
        Feeling::parse("furious", RADIUS_PRESENT as i64, 0),
        Err(refusal) if refusal.kind() == RefusalKind::FeelingUndeclared
    );
    let uncalibrated = matches!(
        Feeling::parse("sad", RADIUS_RIM as i64, -4),
        Err(refusal) if refusal.kind() == RefusalKind::FeelingUncalibrated
    );
    let contradictory = matches!(
        Feeling::of(&[Family::Mad, Family::Joyful], RADIUS_PRESENT as i64, 0),
        Err(refusal) if refusal.kind() == RefusalKind::FeelingContradictory
    );
    let tones = crate::audio::band_tones();
    let level_ok = voiced.peak() <= crate::audio::LEVEL_CEILING && voiced.rms() < voiced.peak();
    let rain_ok = voiced.rain_level() > 0.0 && voiced.rain_level() == wet.gain() * RAIN_GAIN;
    let voices_ok = level_ok && rain_ok && absent && undeclared && uncalibrated && contradictory;
    let (ladder_low, ladder_high) = feeling.ladder();
    println!(
        "U5 voices: {} ({}) anchored at {:.0} nm, radius {} and drift {} → the peak in band {} of 12 \
         (excursion {}), {} bands sounding, ladder {}..={}; the bed at {:.4} of {:.1}",
        feeling.family().name(),
        feeling.family().hue(),
        feeling.family().anchor_nm(),
        feeling.radius(),
        feeling.drift(),
        feeling.peak_band(),
        feeling.excursion(),
        voiced.sounding_bands(),
        ladder_low,
        ladder_high,
        voiced.bed_level(),
        crate::audio::BED_GAIN
    );
    let bounded = if level_ok { "(bounded)" } else { "WRONG (over the mixer's own ceiling)" };
    println!(
        "U5 rain: the field's first wet cell at t7000 is ({wet_x}, {wet_z}) = {:.4} → gain {:.4} and the rain voice \
         at {:.4} of {:.1}{}",
        wet.onset(),
        wet.gain(),
        voiced.rain_level(),
        RAIN_GAIN,
        if rain_ok { "" } else { " WRONG (the rain does not equal the field's own gain)" }
    );
    println!(
        "U5 buffer: {} frames at {:.0} Hz, {} channel, the twelve bands voiced {:.2}–{:.2} Hz; peak {:.6} and rms \
         {:.6} inside the {:.1} ceiling {bounded}; digest {}",
        voiced.frames(),
        crate::audio::SAMPLE_RATE_HZ,
        voiced.channels(),
        tones[0],
        tones[CHANNEL_COUNT - 1],
        voiced.peak(),
        voiced.rms(),
        crate::audio::LEVEL_CEILING,
        short_digest(voiced.digest())
    );
    println!(
        "U5 voices refused by name: feeling_absent={absent} (no reading at all), \
         feeling_undeclared={undeclared} (`furious` is a tertiary state's word), \
         feeling_uncalibrated={uncalibrated} (the wheel's own `carrying-the-loss`, Sad at the rim with \
         drift −4: there is no band below 400 nm to voice it with), feeling_contradictory={contradictory} \
         (mad and joyful at once)"
    );
    band_ok && mass_ok && refusals_ok && light_ok && twilight_ok && beam_ok && moon_ok && audio_ok && voices_ok
}
/// **U7's own bar — row 6 of the plan's §5: a ballot answered with `Origin::Oracle`; the calibration accuracy
/// beats the majority class.**
///
/// Read against the plan's §2 U7 entry (the interface) and the documents it names: `docs/LEDGER.md` R-59.5
/// and §59.5 item 9 for the four fail-closed rules and the bar, C-73 §73.1 for the origin that carries no
/// order, C-97.2's inherited axis rules, R-59.7 for the coverage table's total pin and its anti-fabrication
/// guard, `patches/PATCH-023-the-ballot-and-the-von-classifier.md` for the ballot's own fields and its
/// refusals, and `patches/PATCH-039-the-corpus-beyond-this-tree.md` for the five stages the sixteen labelled
/// records sit at.
///
/// Every clause the bar names is a printed value, and every refusal is printed **by name** as `true` or
/// `false` rather than as a count: a count of four refusals does not say which four.
pub fn check_u7() -> bool {
    // --- the seven axes, and the three ways reading one is a bug ----------------------------------
    let (declared, owed) = axis::coverage_summary();
    let covered = axis::check_coverage(&COVERAGE).is_ok();
    let forged = CoverageRow {
        axis: AxisKind::EvidenceGrade,
        ladder: "the reference's letter grade",
        values: &["A", "B", "C", "E"],
        source: "the evidence's own forged row",
    };
    let fabricated = axis::check_coverage(&[forged])
        .err()
        .map(|refusals| refusals.contains(RefusalKind::FabricatedAxis))
        .unwrap_or(false);
    let partial: Vec<CoverageRow> = COVERAGE
        .iter()
        .copied()
        .filter(|row| row.axis != AxisKind::OrgRank)
        .collect();
    let uncovered = axis::check_coverage(&partial)
        .err()
        .map(|refusals| refusals.contains(RefusalKind::UncoveredAxis))
        .unwrap_or(false);

    let interchange = refuse_interchange(EvidenceGrade::A, Origin::Oracle)
        .err()
        .map(|refusal| refusal.kind() == RefusalKind::AxisInterchangeRefused)
        .unwrap_or(false);
    let same_ladder_free = refuse_interchange(EvidenceGrade::A, EvidenceGrade::D).is_ok();
    let mut reductions = 0;
    for kind in AxisKind::ALL {
        for reduction in [Reduction::Percentage, Reduction::Score] {
            if refuse_reduction(kind, reduction).kind() == RefusalKind::ScalarReductionRefused {
                reductions += 1;
            }
        }
    }
    let claimants = axis::bare_token_axes("A");
    let ambiguous = refuse_bare("A")
        .err()
        .map(|refusal| {
            refusal.kind() == RefusalKind::BareTokenRefused && refusal.detail().contains("AMBIGUOUS")
        })
        .unwrap_or(false);
    let unqualified = refuse_bare("SSS")
        .err()
        .map(|refusal| {
            refusal.kind() == RefusalKind::BareTokenRefused && refusal.detail().contains("UNQUALIFIED")
        })
        .unwrap_or(false);
    let not_this_rules_business = refuse_bare("gold").is_ok();

    let owed_names = owed.join(", ");
    let owed_count = owed.len();
    println!(
        "U7 axes: {} ladders — {declared} declared and {owed_count} owed by name ({owed_names}) — the \
         coverage table describes every axis exactly once = {covered}; a forged row refuses \
         `fabricated_axis` = {fabricated}, an axis with no row refuses `uncovered_axis` = {uncovered}",
        AxisKind::COUNT
    );
    println!(
        "U7 prohibitions: a cross-ladder comparison refuses `axis_interchange_refused` = {interchange} \
         (two members of one ladder compare freely = {same_ladder_free}); {reductions} reductions refused \
         ({} axes × 2); a bare `A` is AMBIGUOUS across {} declared ladders = {ambiguous}, a bare `SSS` is \
         UNQUALIFIED = {unqualified}, and a token no declared ladder claims is not this rule's business = \
         {not_this_rules_business}",
        AxisKind::COUNT,
        claimants.len()
    );

    // --- the ballot, answered by the classifier ---------------------------------------------------
    let classifier = match Classifier::classified() {
        Ok(classifier) => classifier,
        Err(refusals) => {
            eprintln!("U7: the calibration could not be taken over the sixteen readings: {refusals}");
            return false;
        }
    };
    let figures = classifier.figures().clone();
    let labels_current = Classifier::labels_describe_the_records().is_empty();
    // **The vocabulary, read over every record in the tree rather than over the calibration set**: the
    // words the head features are made of, each against the stage the records carry it at. It is printed
    // because it is the evidence that the features are the format's own and not a list fitted to the sixteen.
    let vocabulary = match von::claimed_vocabulary() {
        Ok(vocabulary) => vocabulary,
        Err(refusals) => {
            eprintln!("U7: the record corpus could not be read for its vocabulary: {refusals}");
            return false;
        }
    };
    let outside: usize = vocabulary.families.iter().map(|family| family.outside).sum();
    let vocabulary_ok = vocabulary.refusals.is_empty()
        && vocabulary.records >= von::KNOWN_ANSWERS.len()
        && outside > 0;
    let families: Vec<String> = vocabulary
        .carried()
        .into_iter()
        .map(|family| format!("{}×{}(+{} outside)", family.word, family.seen, family.outside))
        .collect();
    println!(
        "U7 vocabulary: {} declared word(s) read over {} record(s) in place — {}; every word's claim held \
         = {vocabulary_ok}, and {outside} of the readings are records the calibration was **not** taken over \
         ({})",
        vocabulary.families.len(),
        vocabulary.records,
        families.join(", "),
        short_digest(figures.known_answers)
    );

    let question = match Question::choice(&STAGES, "approved") {
        Ok(question) => question,
        Err(refusals) => {
            eprintln!("U7: the stage question is refused by name: {refusals}");
            return false;
        }
    };
    let ballot = match Ballot::declare(
        "u7-life-of-PATCH-015",
        "the plan's §5 row 6",
        "patches/PATCH-015-hypergrilling.md",
        "read the record's own head and say which stage of its life it is at",
        question,
        "the clean room's build",
        24,
    ) {
        Ok(ballot) => ballot,
        Err(refusals) => {
            eprintln!("U7: the demonstration ballot is refused by name: {refusals}");
            return false;
        }
    };
    let answered = match laya::answer(&ballot, &classifier, &von::root(), None) {
        Ok(answered) => answered,
        Err(refusals) => {
            eprintln!("U7: the ballot was not answered: {refusals}");
            return false;
        }
    };
    let oracle = answered.origin() == Origin::Oracle && AxisLadder::token(answered.origin()) == "A";
    let evidence = answered.evidence();
    let features = evidence.features.len();
    println!(
        "U7 calibration: {} — the bar is the majority class, read strictly (leave-one-out {:.4} > majority \
         {:.4}) = {}; the stored labels still describe the records = {labels_current}; the set's own digest \
         {}",
        figures.render(),
        figures.leave_one_out,
        figures.majority,
        figures.beats_majority(),
        short_digest(figures.known_answers)
    );
    println!(
        "U7 ballot: `{}` over `{}` → `{}`, origin {} ({:?}) = {oracle}; the reading is `{}`; {features} \
         declared feature(s) held; {} option(s) scored; the evidence is {} (read in place)",
        ballot.id(),
        ballot.artifact(),
        answered.verdict().name(),
        answered.origin().token(),
        answered.origin(),
        evidence
            .scores
            .iter()
            .max_by(|left, right| left.1.total_cmp(&right.1))
            .map(|(option, _)| option.as_str())
            .unwrap_or("—"),
        evidence.scores.len(),
        short_digest(evidence.digest)
    );
    let carried: Vec<String> = classifier
        .carriers()
        .into_iter()
        .map(|(option, feature, lift)| format!("{feature}→{option} {lift:+.2}"))
        .collect();
    println!(
        "U7 reading: the declared feature(s) that carry a stage are {} — the figure is a reading of {} \
         record(s), so the mechanism is named rather than implied",
        if carried.is_empty() { "none".to_string() } else { carried.join(", ") },
        figures.readings
    );

    // --- the four fail-closed rules, each proved to bite by name ----------------------------------
    let not_established = laya::answer(&ballot, &Demonstration, &von::root(), None)
        .err()
        .map(|refusals| refusals.contains(RefusalKind::NotEstablished))
        .unwrap_or(false);
    let unreachable = laya::answer(
        &ballot,
        &laya::Unreachable { what: "a model-backed instrument" },
        &von::root(),
        None,
    )
    .err()
    .map(|refusals| refusals.contains(RefusalKind::OracleUnreachable))
    .unwrap_or(false);

    // The third rule's own case: a document that is **not a record at all** is asked the same question, and
    // the answer is a non-reading rather than a verdict for some stage. The ballot names the probe, because a
    // ballot is a question about *an artifact* and pointing rule 3 at the answered ballot's record would
    // demonstrate nothing.
    let probe_root = std::env::temp_dir().join(format!("shploingus-clean-u7-{}", std::process::id()));
    let probe = probe_root.join("not-a-record.md");
    let non_record = Question::choice(&STAGES, "approved")
        .ok()
        .and_then(|question| {
            Ballot::declare(
                "u7-life-of-a-description",
                "the plan's §5 row 6",
                "not-a-record.md",
                "read a document that is not a record and say which stage of its life it is at",
                question,
                "the clean room's build",
                24,
            )
            .ok()
        });
    let no_reading = non_record.is_some()
        && std::fs::create_dir_all(&probe_root).is_ok()
        && std::fs::write(&probe, "# a description, not a record\n\nprose about a thing.\n").is_ok()
        && non_record
            .map(|ballot| {
                laya::answer(&ballot, &classifier, &probe_root, None)
                    .map(|answered| {
                        answered.verdict() == Verdict::NoReading
                            && answered.passed().err().map(|refusal| refusal.kind())
                                == Some(RefusalKind::NoReading)
                    })
                    .unwrap_or(false)
            })
            .unwrap_or(false);
    let _ = std::fs::remove_file(&probe);

    // The same record under a floor it does not clear: the reading is `approved`, the floor says the answer
    // must be `proposed`, so the reading is `not_belongs` — the fourth rule's own case.
    let off_floor = Question::choice(&STAGES, "proposed").ok().and_then(|question| {
        Ballot::declare(
            "u7-life-of-PATCH-015-against-proposed",
            "the plan's §5 row 6",
            "patches/PATCH-015-hypergrilling.md",
            "read the record's own head and say which stage of its life it is at",
            question,
            "the clean room's build",
            24,
        )
        .ok()
    });
    let (reason_refused, reason_recorded) = match off_floor {
        Some(ballot) => {
            let refused = laya::answer(&ballot, &classifier, &von::root(), None)
                .err()
                .map(|refusals| refusals.contains(RefusalKind::ReasonAbsent))
                .unwrap_or(false);
            let filed = laya::answer(
                &ballot,
                &classifier,
                &von::root(),
                Some("the record's own stage is earlier than the floor"),
            )
            .map(|answered| {
                answered.verdict() == Verdict::NotBelongs && answered.evidence().reason.is_some()
            })
            .unwrap_or(false);
            (refused, filed)
        }
        None => (false, false),
    };
    println!(
        "U7 fail-closed: an uncalibrated instrument writes nothing [`not_established`] = {not_established}; \
         a missing instrument is refused [`oracle_unreachable`] = {unreachable}; a non-reading is never a \
         pass [`no_reading`] = {no_reading}; a `not_belongs` reading needs a human's reason in the same act \
         [`reason_absent`] = {reason_refused}, and with one it is filed carrying the reason = \
         {reason_recorded}"
    );

    figures.quotable()
        && figures.beats_majority()
        && figures.leave_one_out_from_the_claim_only < figures.majority
        && vocabulary_ok
        && labels_current
        && oracle
        && answered.verdict() == Verdict::Belongs
        && covered
        && fabricated
        && uncovered
        && interchange
        && same_ladder_free
        && reductions == AxisKind::COUNT * 2
        && ambiguous
        && unqualified
        && not_this_rules_business
        && not_established
        && unreachable
        && no_reading
        && reason_refused
        && reason_recorded
}

/// A judge whose calibration does **not** beat the majority class — the declared demonstration of the first
/// fail-closed rule, so the gate proves the refusal bites rather than asserting that it would.
struct Demonstration;

impl StructuralJudge for Demonstration {
    fn judge(
        &self,
        _question: &Question,
        _artifact: &[u8],
    ) -> Result<Judgement, crate::refusal::Refusal> {
        Ok(Judgement {
            chosen: "designed".to_string(),
            scores: vec![("designed".to_string(), 1.0)],
            features: Vec::new(),
        })
    }

    fn measurement(&self) -> &Measurement {
        &DEMONSTRATION_MEASUREMENT
    }
}

/// A calibration that answers exactly what always guessing the commonest stage answers: it ties the majority
/// class, which is a loss rather than a pass.
static DEMONSTRATION_MEASUREMENT: Measurement = Measurement {
    readings: 16,
    per_option: Vec::new(),
    majority: 0.3125,
    in_sample: 0.3125,
    leave_one_out: 0.3125,
    leave_one_out_with_absence: 0.3125,
    leave_one_out_from_the_claim_only: 0.0,
    known_answers: Digest::from_bytes([0u8; 32]),
};

/// The first eight hex digits of a digest — enough to compare two runs by eye, and named as a prefix rather
/// than printed as if it were the whole digest.
fn short_digest(digest: Digest) -> String {
    digest.to_hex().chars().take(8).collect()
}
/// **U8 — the shell: a visibly drawn frame, and the one pointer-ownership decision.**
///
/// The bar is `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §5 row 7's own check — *a visibly drawn frame;
/// pointer ownership read once* — measured rather than asserted: the frame is drawn, its borders and its
/// bar are found in the drawn text, its digest is printed so two runs can be compared, and the pointer
/// decision is read **once** and handed **by value** to the drawing (the signature enforces that, rather
/// than a comment promising it). The interactive form is the binary's own `shell` mode: `cargo run
/// --offline -- shell` puts the frame on screen and redraws it per command, reading real system state.
pub fn check_u8() -> bool {
    let recipe = shell::Recipe::declared();
    let mut mount = shell::Mount::of(&recipe);
    let area = shell::declared_area();
    // **Read once**: the decision is a value here, and the drawing receives that value — it cannot
    // re-read it, so the glyph and the grab cannot disagree.
    mount.set_grab(true);
    let pointer = mount.read_pointer();
    let drawn = match shell::frame(&mount, area, pointer) {
        Ok(drawn) => drawn,
        Err(refusal) => {
            eprintln!("U8 frame refused by name: {refusal}");
            return false;
        }
    };
    let counts = shell::bucket_counts();
    let visible =
        drawn.contains('┌') && drawn.contains('│') && drawn.contains("shploingus — the shell");
    let pointer_once = pointer.visible == (pointer.owner == shell::PointerOwner::Shell)
        && mount.read_pointer() == pointer;
    println!(
        "U8 shell frame:  {} × {} cells — {} windows, {} panes, layer={}, pointer_owner={}, glyph={}",
        area.cols,
        area.rows,
        mount.windows().len(),
        mount.panes().len(),
        mount.layer().name(),
        pointer.owner.name(),
        pointer.visible
    );
    println!(
        "U8 frame digest: {} (a visibly drawn frame = {visible}, pointer ownership read once = \
         {pointer_once})",
        shell::frame_digest(&drawn).to_hex()
    );
    println!(
        "U8 constants:    {} admitted (md1-rule {}, md1-example {}, platform-translation {}, \
         project-extension {}); an unadmitted name is refused by name",
        shell::CONSTANTS.len(),
        counts[0],
        counts[1],
        counts[2],
        counts[3]
    );
    visible && pointer_once && drawn.lines().count() == area.rows
}
/// U6's policy chokepoint — **one decision function, measured** (2026-10-12, §29).
///
/// The evidence `docs/HYPERGRILLING.md` §L-5 and `docs/LEDGER.md` R-62.3 ask for, **printed** rather
/// than asserted in private: the closed matrix and its classes; the four faults refused at
/// **construction** (`grant_unattributed`, `grant_unscoped`, and the unnamed actor); a denial that
/// **quotes the list**; and the one call site — the reference operator serves every gated row, and an
/// actor holding nothing is refused **by name** with the refusal **recorded**, which is the half of the
/// bar the record cares about (*a refusal nobody can read is a refusal that did not happen*).
pub fn check_u6_policy() -> bool {
    // **The matrix, as values.** Closed, counted, and every row readable — a row with nothing to say is
    // refused rather than printed.
    let matrix = policy::check_matrix();
    let matrix_ok = matrix.is_ok();
    let count = |wanted: fn(policy::PermissionClass) -> bool| {
        policy::Action::ALL
            .iter()
            .filter(|action| wanted(policy::permission_of(**action)))
            .count()
    };
    let allowed = count(|class| class == policy::PermissionClass::AlwaysAllowed);
    let gated = count(|class| matches!(class, policy::PermissionClass::Gated(_)));
    let denied = count(|class| matches!(class, policy::PermissionClass::AlwaysDenied(_)));
    let rejected = count(|class| class == policy::PermissionClass::NeverStore);
    let classes_ok = allowed + gated + denied + rejected == policy::ACTION_COUNT;
    println!(
        "U6 policy: {} act(s), closed ({} of them allowed — the reads and the checks everything \
         depends on; {} gated; {} denied unconditionally; {} a rejection), every row describes itself \
         = {matrix_ok}, and the classes account for every act = {classes_ok}",
        policy::ACTION_COUNT,
        allowed,
        gated,
        denied,
        rejected
    );
    // Deny by default, at the vocabulary: an act outside the closed list is not a nearly-right act, and
    // the words a caller might try are refused by the same rule the table's undeclared verbs are.
    let outside = ["everything", "execute", "*", "all"];
    let unrepresentable = outside
        .iter()
        .all(|word| policy::Action::parse(word).is_none() && policy::Scope::parse(word).is_none());
    println!(
        "U6 policy: deny by default — `{}` are not acts and not scopes, and a missing matrix entry is \
         a **compile error** rather than a runtime deny (the matrix's `match` has no wildcard arm) = \
         {unrepresentable}",
        outside.join("`, `")
    );

    // **The four faults refused at construction.** A grant with no granter, a grant whose scope is
    // unbounded, a grant whose scope is undeclared, and a caller naming no actor.
    let unattributed = policy::Grant::of("", "record").expect_err("no granter");
    let unbounded = policy::Grant::of("operator", "everything").expect_err("no scope");
    let undeclared = policy::Grant::of("operator", "worlds").expect_err("not a scope");
    let unnaming = policy::Caller::of("", Vec::new()).expect_err("no actor");
    let constructions_ok = unattributed.kind() == RefusalKind::GrantUnattributed
        && unbounded.kind() == RefusalKind::GrantUnscoped
        && undeclared.kind() == RefusalKind::GrantUnscoped
        && unnaming.kind() == RefusalKind::GrantUnattributed
        && unbounded.detail().contains("everything");
    println!(
        "U6 policy construction: a grant with no granter [{}], a scope that is `everything` [{}], a \
         scope nobody declares [{}], and a caller with no actor [{}] — each refused at construction = \
         {constructions_ok}",
        unattributed.kind().name(),
        unbounded.kind().name(),
        undeclared.kind().name(),
        unnaming.kind().name()
    );

    let roster = match process::roster() {
        Ok(roster) => roster,
        Err(refusals) => {
            eprintln!("U6 policy roster refused by name: {refusals}");
            return false;
        }
    };
    let mut gate = match reference::gate(&roster, 1) {
        Ok(gate) => gate,
        Err(refusal) => {
            eprintln!("U6 policy: the reference declaration was refused by name: {refusal}");
            return false;
        }
    };
    let root = std::env::temp_dir().join("shploingus-u6-policy");
    let _ = std::fs::remove_dir_all(&root);
    if let Err(error) = std::fs::create_dir_all(&root) {
        eprintln!("U6 policy: the reference root could not be made: {error}");
        return false;
    }
    gate.set_intake_root(&root);

    // **The operator's session: every gated act is served.** The caller is the reference declaration's
    // own ([`reference::caller`]), so this is the same actor the served session and the tests act as.
    let operator = gate.caller().actor().to_string();
    let mut served = 0usize;
    let mut saved = None;
    for line in [
        "truth",
        "held",
        "record",
        "geometry chunk=0,0,0 level=0",
        "save path=policy-world.save",
    ] {
        match gate.call(line) {
            Ok(receipt) => {
                served += 1;
                if let Answer::Saved(facts) = &receipt.answer {
                    saved = Some(facts.clone());
                }
            }
            Err(refusal) => {
                eprintln!("U6 policy: the operator's `{line}` was refused by name: {refusal}");
                return false;
            }
        }
    }
    let facts = match saved {
        Some(facts) => facts,
        None => {
            eprintln!("U6 policy: the `save` row answered no document");
            return false;
        }
    };
    if let Err(refusal) = gate.call(&format!(
        "load path=policy-world.save digest={}",
        facts.digest.to_hex()
    )) {
        eprintln!("U6 policy: the operator's `load` was refused by name: {refusal}");
        return false;
    }
    served += 1;
    let served_ok = served == 6;
    println!(
        "U6 policy: the reference `{operator}` serves every gated act — `save` wrote {} byte(s), \
         `load` read them back, and `geometry` appended — {served} of 6 calls answered = {served_ok}",
        facts.len
    );

    // **An actor holding nothing is refused by name, and the refusal is recorded.** The reads still
    // answer — a read is not an act anyone can be kept out of — while the four gated acts refuse
    // `policy_denied` **before they dispatch**, so a denial is a fact about the table rather than about
    // the document or the chunk that happened to be named.
    let before = gate.chain().len();
    let reader = match policy::Caller::of("reader", Vec::new()) {
        Ok(reader) => reader,
        Err(refusal) => {
            eprintln!("U6 policy: the reader declaration was refused by name: {refusal}");
            return false;
        }
    };
    gate.set_caller(reader);
    let mut read_ok = true;
    for line in ["truth", "held", "record"] {
        if let Err(refusal) = gate.call(line) {
            eprintln!("U6 policy: the reader's `{line}` was refused by name: {refusal}");
            read_ok = false;
        }
    }
    let gated_rows = [
        "geometry chunk=1,0,0 level=0",
        "admit path=x.md digest=0000000000000000000000000000000000000000000000000000000000000000 \
         feature=f licence=permitted claim=checksum_only carrier=U4 check=c.md \
         check_digest=0000000000000000000000000000000000000000000000000000000000000000",
        "save path=reader.save",
        "load path=policy-world.save digest=0000000000000000000000000000000000000000000000000000000000000000",
    ];
    let mut refusal_names = Vec::new();
    let mut quoted = String::new();
    for line in gated_rows {
        match gate.call(line) {
            Ok(_) => {
                eprintln!("U6 policy: the reader was served `{line}` — the chokepoint did not bite");
                return false;
            }
            Err(refusal) => {
                if quoted.is_empty() {
                    quoted = refusal.detail().to_string();
                }
                refusal_names.push(refusal.kind().name().to_string());
            }
        }
    }
    let all_denied = refusal_names.iter().all(|name| name == "policy_denied");
    let recorded = gate.chain().len() == before + gated_rows.len();
    let record_verifies = gate.chain().verify().is_ok();
    let quotes_the_list = quoted.contains("`reader`")
        && quoted.contains("no scopes")
        && quoted.contains("the declared scopes are");
    println!(
        "U6 policy: `reader` holds nothing — its three reads answer = {read_ok}; its four gated acts \
         refuse [{}] = {all_denied}, each one **recorded** (the record moved by {} of {}) = {recorded}, \
         and the record still verifies = {record_verifies}",
        refusal_names.join(", "),
        gate.chain().len() - before,
        gated_rows.len()
    );
    println!("U6 policy refusal, quoting the list: {quoted}");

    // The two unconditional denials hold even for an actor that holds **every** declared scope, and a
    // rejection is not a denial.
    let mut all = Vec::new();
    for scope in ["record", "content", "world"] {
        match policy::Grant::of("the operator's own declaration", scope) {
            Ok(grant) => all.push(grant),
            Err(refusal) => {
                eprintln!("U6 policy: a declared scope was refused by name: {refusal}");
                return false;
            }
        }
    }
    let everything = match policy::Caller::of("root", all) {
        Ok(root) => root,
        Err(refusal) => {
            eprintln!("U6 policy: the all-scopes declaration was refused by name: {refusal}");
            return false;
        }
    };
    let deletion = policy::decide(&everything, policy::Action::DeleteContent).expect_err("denied");
    let external = policy::decide(&everything, policy::Action::WriteExternal).expect_err("denied");
    let storage = policy::decide(&everything, policy::Action::PersistMaterial).expect_err("rejected");
    let unconditional_ok = deletion.kind() == RefusalKind::PolicyDenied
        && external.kind() == RefusalKind::PolicyDenied
        && storage.kind() == RefusalKind::NeverStore
        && everything.holds(policy::Scope::Record)
        && everything.holds(policy::Scope::Content)
        && everything.holds(policy::Scope::World);
    println!(
        "U6 policy: an actor holding all {} scopes still cannot `delete_content` [{}] or \
         `write_external` [{}], and `persist_material` is a rejection rather than a permission [{}] = \
         {unconditional_ok}",
        policy::Scope::COUNT,
        deletion.kind().name(),
        external.kind().name(),
        storage.kind().name()
    );

    matrix_ok
        && classes_ok
        && unrepresentable
        && constructions_ok
        && served_ok
        && read_ok
        && all_denied
        && recorded
        && record_verifies
        && quotes_the_list
        && unconditional_ok
}
/// **The run's own world, built once per check** — the declared fields (elevation, water level, lithology)
/// in one evaluation, filled into their cubic world by the world's own build. Every check that reads *the
/// world* reads this one shape of it, so two functions cannot describe two similar worlds.
fn declared_cubic(
    roster: &Declared,
    identity: WorldIdentity,
) -> Result<CubicWorld, crate::refusal::Refusal> {
    let request = process::declared_request();
    let patch = request.patch()?;
    let ladder = reference::ladder()?;
    let values = fields_at(
        roster,
        identity,
        Tick(0),
        &[FieldId::Elevation, FieldId::WaterLevel, FieldId::Lithology],
        request,
        1,
    )?;
    CubicWorld::from_surface(patch, &values[0], &values[1], &values[2], &ladder)
}

/// **U12 — the first mob: a member of an organisation, its need met through the world's own cells.**
///
/// The bar is `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §5 row 12's own check — *an organisation's member acts
/// only through the one tool table, and an act it may not do is refused by name and recorded* — plus this
/// pass's own clause: the being's need is met through the game's **own** cell-based water law, and since
/// §38 the plane it is met through is the **cubic world's own** — the world the declared fields are filled
/// into, read at `Tick(0)` through the one reading every consumer shares ([`being::WorldRead::over_cubic`])
/// and walked along the shaper's own `d8_receivers` — with [`check_u12_stepping`] beside it measuring the
/// same being on the plane the law's own ticks have stepped. Its choice is a **laya question read by von**,
/// and every refusal is by name and recorded. Nothing here is a second path: the world is read through the
/// one world's own cells, the flow direction is the shaper's own `d8_receivers`, the question is the
/// calibrated stage question, and the acts go on U2's one `Chain`.
pub fn check_u12() -> bool {
    let organisation = being::Organisation::first();
    let member: &'static str = "the first mob";
    let dir = std::env::temp_dir().join(format!("shploingus-clean-u12-run-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(&dir);

    // --- the charter, read over the whole closed action vocabulary --------------------------------
    let rows = match organisation.charter_read(member) {
        Ok(rows) => rows,
        Err(refusal) => {
            eprintln!("U12: the charter is refused by name: {refusal}");
            return false;
        }
    };
    let allowed = rows.iter().filter(|(_, ok)| *ok).count();
    let denied = rows.len() - allowed;
    println!(
        "U12 organisation: `{}`, charter `{}`, office `{}` — its rank ladder stays owed by name \
         (`AxisKind::OrgRank`); the charter reads all {} declared action(s): {allowed} allowed, {denied} \
         refused by name",
        organisation.name(),
        organisation.charter(),
        organisation.office(),
        rows.len()
    );

    // --- the world's own cells, and the being's own record -----------------------------------------
    let roster = match process::roster() {
        Ok(roster) => roster,
        Err(refusals) => {
            eprintln!("U12: the declared roster is refused by name: {refusals}");
            return false;
        }
    };
    let identity = match reference::identity() {
        Ok(identity) => identity,
        Err(refusal) => {
            eprintln!("U12: the reference identity is refused by name: {refusal}");
            return false;
        }
    };
    // **The cubic world's own cells (§38)**: the being's plane is the world's own reading — the cubic world
    // the declared fields are filled into — rather than the generator-fed column field beside it.
    let cubic = match declared_cubic(&roster, identity) {
        Ok(cubic) => cubic,
        Err(refusal) => {
            eprintln!("U12: the declared fields do not fill a cubic world: {refusal}");
            return false;
        }
    };
    let world = match being::WorldRead::over_cubic(organisation, member, &cubic) {
        Ok(world) => world,
        Err(refusal) => {
            eprintln!("U12: the world's own cells are refused by name: {refusal}");
            return false;
        }
    };
    // **Where the world's own law puts the first mob**: the first dry cell whose flow path reaches standing
    // water within the being's reach. The coordinate is the world's answer rather than one chosen here, and it
    // is written into the being's own record as its birthplace.
    let Some(start) = being::start_cell(&world, 8) else {
        eprintln!("U12: the world's own law names no dry cell whose flow path reaches water");
        return false;
    };
    let Some(sink) = being::dry_sink(&world) else {
        eprintln!("U12: the world's own law carries every dry cell somewhere");
        return false;
    };
    let record = dir.join("being.md");
    let being_record = format!(
        "---\nrecord: being\nbeing: {member}\norganisation: {}\ncell: {start}\n---\n\nthis being is a member of \
         {}. Its need is water, its acts are its own, and what it is, is what its records say.\n",
        organisation.name(),
        organisation.name()
    );
    if let Err(error) = std::fs::write(&record, being_record) {
        eprintln!("U12: the being's record cannot be written: {error}");
        return false;
    }
    let classifier = match Classifier::classified() {
        Ok(classifier) => classifier,
        Err(refusals) => {
            eprintln!("U12: the sixteen readings do not calibrate: {refusals}");
            return false;
        }
    };
    let root = von::root();

    // --- the drive: ticks along the game's own flow law until the need is met ----------------------
    let (acts, digest, reading, stage_digest, features) =
        match drive(organisation, member, &record, &dir.join("drafts"), 8, &world, &classifier, &root) {
            Ok(parts) => parts,
            Err(refusal) => {
                eprintln!("U12: the first mob's drive is refused by name: {refusal}");
                return false;
            }
        };
    let walked = acts.iter().filter(|act| matches!(act, being::Act::Walk { .. })).count();
    let Some((cell, depth_um)) = acts.iter().find_map(|act| match act {
        being::Act::Drink { cell, depth_um } => Some((*cell, *depth_um)),
        _ => None,
    }) else {
        eprintln!("U12: the mob's drive did not reach the world's own water");
        return false;
    };
    // The being's own records, re-read for the numbers a reader checks: the need is a fold, so this is the
    // same question the model asks — not a second counter agreeing with it.
    let mut mob = match being::Being::of(member, organisation, &record, dir.join("drafts-again"), 8) {
        Ok(mob) => mob,
        Err(refusal) => {
            eprintln!("U12: the being is refused by name on construction: {refusal}");
            return false;
        }
    };
    for _ in 0..8 {
        match mob.tick(&world, &classifier, &root) {
            Ok(being::Act::Drink { .. }) => break,
            Ok(_) => {}
            Err(refusal) => {
                eprintln!("U12: the re-run's drive is refused by name: {refusal}");
                return false;
            }
        }
    }
    let still_thirsty = mob.thirsty();
    let chain_ok = mob.verify().is_ok();
    let one_digest = being::chain_digest(mob.chain()) == digest;
    let governed = mob.chain().len();
    let recorded = mob.refusals();
    println!(
        "U12 first mob:    cell {start} of {} — {walked} step(s) along the game's own flow law (the plane it \
         stands in is the cubic world's own cells, read at Tick(0) through the one reading every consumer \
         shares, §38; the same world after the law's own ticks is the next check's plane), then drank at cell \
         {cell} ({:.6} m of the world's own water); the need reads met = {}; the one chain carries {governed} \
         event(s) ({recorded} refusal(s)) and verifies = {chain_ok}",
        world.cells(),
        depth_um as f64 / 1_000_000.0,
        !still_thirsty
    );
    println!(
        "U12 stage question: reads `{reading}` at the charter's floor `{}` (origin the proof's own) over {} — \
         the being's draft read in place, digest {}; the declared fact(s) that carried it: {}",
        organisation.floor(),
        short_digest(stage_digest),
        short_digest(digest),
        features.join(", ")
    );

    // --- an act the office does not reach: refused by name, and recorded ---------------------------
    let denial = mob.attempt_act_it_may_not_do().err();
    let denial_name = denial.as_ref().map(|refusal| refusal.kind().name()).unwrap_or("allowed");
    let quotes_the_list = denial
        .as_ref()
        .map(|refusal| refusal.detail().contains("may not") && refusal.detail().contains("declared scopes are"))
        .unwrap_or(false);
    let denial_recorded = mob.refusals() == recorded + 1;
    println!(
        "U12 governed act: `save_world` is refused `{denial_name}` to this member, the refusal quotes the \
         list = {quotes_the_list}, and it is on the chain = {denial_recorded}"
    );

    // --- the three refusals this being's own drive can meet, each by name --------------------------
    let stranded_record = dir.join("stranded.md");
    let _ = std::fs::write(
        &stranded_record,
        format!(
            "---\nrecord: being\nbeing: {member}\ncell: {sink}\n---\n\nthis being stands where the world's own \
             routing carries nowhere, and its reach is nothing.\n"
        ),
    );
    let mut stranded = match being::Being::of(member, organisation, &stranded_record, dir.join("stranded"), 0) {
        Ok(being) => being,
        Err(refusal) => {
            eprintln!("U12: the stranded being is refused by name: {refusal}");
            return false;
        }
    };
    let unreachable = stranded
        .tick(&world, &classifier, &root)
        .err()
        .map(|refusal| refusal.kind() == RefusalKind::WaterUnreachable && stranded.refusals() == 1)
        .unwrap_or(false);
    // 1b. Its reach is nothing, on a dry cell the world's own routing *would* carry to water: the reach guard's
    //     own refusal, which the sink's branch cannot answer for.
    let mut short_reach = match being::Being::of(member, organisation, &record, dir.join("short"), 0) {
        Ok(being) => being,
        Err(refusal) => {
            eprintln!("U12: the short-reach being is refused by name: {refusal}");
            return false;
        }
    };
    let reach_spent = short_reach
        .tick(&world, &classifier, &root)
        .err()
        .map(|refusal| refusal.kind() == RefusalKind::WaterUnreachable && short_reach.refusals() == 1)
        .unwrap_or(false);
    let mut guild =
        match being::Being::of(member, being::Organisation::stricter(), &record, dir.join("guild"), 8) {
            Ok(being) => being,
            Err(refusal) => {
                eprintln!("U12: the guild's being is refused by name: {refusal}");
                return false;
            }
        };
    let below_floor = guild
        .tick(&world, &classifier, &root)
        .err()
        .map(|refusal| refusal.kind() == RefusalKind::StageBelowFloor && guild.refusals() == 1)
        .unwrap_or(false);
    let headless = dir.join("no-claim.md");
    let _ = std::fs::write(&headless, "record: being\nno fence at all\n");
    let unsouled = being::Being::of(member, organisation, &headless, dir.join("hollow"), 4)
        .err()
        .map(|refusal| refusal.kind() == RefusalKind::NoReading)
        .unwrap_or(false);
    println!(
        "U12 refusals: a need the world cannot meet where the being may reach is refused \
         `water_unreachable` and recorded = {unreachable} (a dry sink) and = {reach_spent} (a reach spent on \
         a cell the routing would carry); an act below the charter's own floor ({}) is refused \
         `stage_below_floor` and recorded = {below_floor}; a being whose records cannot be read is refused \
         `no_reading` = {unsouled}",
        being::Organisation::stricter().floor()
    );
    println!(
        "U12 determinism:  two runs of the first mob produce one chain digest = {one_digest} ({}){}",
        short_digest(digest),
        if one_digest { "" } else { " — THE RUNS DIVERGED" }
    );

    rows.len() == policy::Action::ALL.len()
        && allowed < rows.len()
        && !still_thirsty
        && walked > 0
        && world.water_stands(cell)
        && reading == organisation.floor()
        && chain_ok
        && quotes_the_list
        && denial_recorded
        && unreachable
        && reach_spent
        && below_floor
        && unsouled
        && one_digest
}

/// **The declared count of ticks the stepped-plane check advances** — the game's own routing, driven this
/// many times over the cells, and the same number the being's own record declares as its reach. Eight ticks
/// of the world's law against eight steps of the being's.
pub const STEPPING_TICKS: usize = 8;

/// **U12 — the cubic world's own cells stepped: the being's need met through the game's own automata over
/// time.**
///
/// The bar is this pass's own clause, read against the three dated sentences it closes: §32 landed the being's
/// need as **one static evaluation** of the roster at `Tick(0)`; §33 recorded that a *stepping cellular
/// automaton is **owed*** — *state that carries tick to tick, so a rained-on or drained column changes the
/// being's next step*; and §37 ruled that the world is the cubic chunk, so this pass moves that law **onto the
/// cells** (§38). [`crate::automaton`] is that state and that law, and this check is the number a reader can
/// hold it to: **the same being, at the same cell of the same patch, taking its next step twice** — on the
/// cells at `Tick(0)`, and on the cells at rest — with the columns that hold water before and after, the
/// state's digest, and the pit the rain reaches. Nothing here is a second needs model or a second drive: the
/// being is the same [`being::Being`] built from the same record, and its acts come from the same [`drive`].
///
/// **§37's own question, printed by this check as well.** The reading the law steps is the surface the cells
/// return, and the last pass measured it to route differently from the generator's float surface at 2 033 of
/// 2 304 columns. The last line of this check prints [`crate::automaton::attribute_routes`]'s decomposition —
/// the columns whose reading is the declared voxel, and which voxel effect each difference is — so the
/// answer (*the voxel's, not a defect's*) is a measurement in the output rather than a claim in a document.
///
/// **The settled world, not a passing one (§35).** §34's line printed eight ticks, which the audit measured
/// to be the generator's lake still in flight. §38 moved this law onto the cells, so the settled figures are
/// the **cells'** own state's — the movement from the column-fed plane §37 printed (a different graph, and a
/// state that began from a truncating float conversion) is recorded in §38 rather than carried here as if it
/// still held. The line prints the short-horizon transient *beside* the settled figures, the being is driven
/// over the settled world (the world it actually lives in), and its **need** is printed as its own value on
/// both planes rather than being left to be read out of the act.
pub fn check_u12_stepping() -> bool {
    let organisation = being::Organisation::first();
    let member: &'static str = "the first mob";
    let reach = STEPPING_TICKS;
    let dir = std::env::temp_dir().join(format!(
        "shploingus-clean-u12-stepping-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(&dir);

    let roster = match process::roster() {
        Ok(roster) => roster,
        Err(refusals) => {
            eprintln!("U12 stepping: the declared roster is refused by name: {refusals}");
            return false;
        }
    };
    let identity = match reference::identity() {
        Ok(identity) => identity,
        Err(refusal) => {
            eprintln!("U12 stepping: the reference identity is refused by name: {refusal}");
            return false;
        }
    };
    // **The cubic world's own cells (§38)**, the same one build every check reads: the being's two planes
    // are this world at `Tick(0)` and this world after the law's own ticks — not a column field beside it.
    let cubic = match declared_cubic(&roster, identity) {
        Ok(cubic) => cubic,
        Err(refusal) => {
            eprintln!("U12 stepping: the declared fields do not fill a cubic world: {refusal}");
            return false;
        }
    };
    let world = match being::WorldRead::over_cubic(organisation, member, &cubic) {
        Ok(world) => world,
        Err(refusal) => {
            eprintln!("U12 stepping: the world's own cells are refused by name: {refusal}");
            return false;
        }
    };
    let Some(start) = being::start_cell(&world, reach) else {
        eprintln!("U12 stepping: the world's own law names no dry cell whose flow path reaches water");
        return false;
    };
    let Some(sink) = being::dry_sink(&world) else {
        eprintln!("U12 stepping: the world's own law carries every dry cell somewhere");
        return false;
    };
    let record = dir.join("being.md");
    let being_record = format!(
        "---\nrecord: being\nbeing: {member}\norganisation: {}\ncell: {start}\n---\n\nthis being is a member of \
         {}. Its need is water, its acts are its own, and what it is, is what its records say.\n",
        organisation.name(),
        organisation.name()
    );
    if let Err(error) = std::fs::write(&record, being_record) {
        eprintln!("U12 stepping: the being's record cannot be written: {error}");
        return false;
    }
    let classifier = match Classifier::classified() {
        Ok(classifier) => classifier,
        Err(refusals) => {
            eprintln!("U12 stepping: the sixteen readings do not calibrate: {refusals}");
            return false;
        }
    };
    let root = von::root();

    // --- the state the cells are in, the state they pass through, and the state they settle into ------
    let mut plane = crate::automaton::Automaton::over(&cubic);
    let standing_at_zero = plane.standing();
    let deepest_at_zero = describe_deepest(plane.deepest());
    let sink_at_zero = plane.depth_um(sink);
    // The short horizon §34 printed, kept for contrast: it is the generator's lake still in flight.
    let mut early = crate::automaton::Automaton::over(&cubic);
    if let Err(refusal) = early.steps(STEPPING_TICKS) {
        eprintln!("U12 stepping: the law's own pour is refused by name: {refusal}");
        return false;
    }
    let (standing_at_eight, deepest_at_eight, total_at_eight) =
        (early.standing(), describe_deepest(early.deepest()), early.total_um());
    // **The world the being actually lives in**: the cells at rest, and the world's own reading of them.
    if let Err(refusal) = plane.steps(SETTLE_TICKS) {
        eprintln!("U12 stepping: the law's own pour is refused by name: {refusal}");
        return false;
    }
    let state_digest = plane.digest();
    let stepped = match being::WorldRead::over_cubic(organisation, member, plane.world()) {
        Ok(stepped) => stepped,
        Err(refusal) => {
            eprintln!("U12 stepping: the stepped world's own cells are refused by name: {refusal}");
            return false;
        }
    };
    println!(
        "U12 settled plane: {SETTLE_TICKS} tick(s) of the game's own routing over {} column(s), rain {} µm per \
         tick and a pit keeping {}% — water stands on {standing_at_zero} column(s) at Tick(0) ({deepest_at_zero}) \
         and on {standing_at_eight} at Tick({STEPPING_TICKS}) ({deepest_at_eight}, {total_at_eight} µm on the \
         plane: the generator's lake still passing), then on {} at rest ({}; {} µm on the plane, unmoved by \
         {SETTLE_TICKS} → {} tick(s)); the dry sink at column {sink} holds {sink_at_zero} µm at Tick(0) and {} µm \
         at rest; state digest {}",
        world.cells(),
        crate::automaton::RAIN_UM_PER_TICK,
        crate::automaton::PIT_KEEP_PERCENT,
        plane.standing(),
        describe_deepest(plane.deepest()),
        plane.total_um(),
        SETTLE_TICKS + 500,
        plane.depth_um(sink),
        short_digest(state_digest)
    );
    // And at rest means at rest: five hundred more ticks move nothing, which the line above states rather
    // than implies.
    let mut later = plane.clone();
    if let Err(refusal) = later.steps(500) {
        eprintln!("U12 stepping: the law's own pour is refused by name: {refusal}");
        return false;
    }
    let at_rest = later.total_um() == plane.total_um() && later.deepest() == plane.deepest();

    // --- the same being, the same cell, two planes ------------------------------------------------
    let base = drive(
        organisation,
        member,
        &record,
        &dir.join("drafts-base"),
        reach,
        &world,
        &classifier,
        &root,
    );
    let moved = drive(
        organisation,
        member,
        &record,
        &dir.join("drafts-stepped"),
        reach,
        &stepped,
        &classifier,
        &root,
    );
    let (base_acts, base_digest) = match base {
        Ok((acts, digest, _, _, _)) => (acts, digest),
        Err(refusal) => {
            eprintln!("U12 stepping: the being's drive on the unstepped plane is refused: {refusal}");
            return false;
        }
    };
    let (stepped_acts, stepped_digest) = match moved {
        Ok((acts, digest, _, _, _)) => (acts, digest),
        Err(refusal) => {
            eprintln!("U12 stepping: the being's drive on the stepped plane is refused: {refusal}");
            return false;
        }
    };
    let base_next = next_step(&base_acts);
    let stepped_next = next_step(&stepped_acts);
    let base_need = drank_at(&base_acts);
    let stepped_need = drank_at(&stepped_acts);
    let met_base = base_need.is_some();
    let met_stepped = stepped_need.is_some();
    // **§33's sentence, measured column by column** rather than read off the one cell the being stands in: the
    // walk to water is taken from every column of the patch on both planes, by the being's own walk
    // ([`being::walk_to_water`]), and the direction of every change is named — a walk the rain made possible,
    // one the drain took away, one that moved.
    let (mut gained, mut lost, mut moved) = (0_usize, 0_usize, 0_usize);
    let mut read_low = 0_usize;
    for cell in 0..world.cells() {
        match (
            being::walk_to_water(&world, cell, reach),
            being::walk_to_water(&stepped, cell, reach),
        ) {
            (None, Some(_)) => gained += 1,
            (Some(_), None) => lost += 1,
            (Some(base), Some(moved_to)) if base != moved_to => moved += 1,
            _ => {}
        }
        if stepped.depth_um(cell) < plane.depth_um(cell) {
            read_low += 1;
        }
    }
    println!(
        "U12 stepping:    one being, one cell ({start}), two planes — at Tick(0) its need reads met = \
         {met_base} ({base_need_where}, next step {base_next}); over the settled plane at rest its need reads \
         met = {met_stepped} ({stepped_need_where}, next step {stepped_next})",
        base_need_where = describe_drink(base_need),
        base_next = describe_next(base_next),
        stepped_need_where = describe_drink(stepped_need),
        stepped_next = describe_next(stepped_next)
    );
    println!(
        "U12 stepping walk: {} of {} column(s) walk to water differently on the settled plane than on the plane \
         the being was placed in (reach {reach}) — {gained} walk(s) the rain made possible, {lost} the drain took \
         away, {moved} moved; at the being's own cell {start} the walk is {} on both, and the water it reaches \
         at column {} stands {} µm on the settled plane; the read's own whole micrometres answer below the \
         cells' at {read_low} of {} column(s), never by more than one",
        gained + lost + moved,
        world.cells(),
        describe_next(base_next),
        stepped_next.1.unwrap_or(start),
        stepped.depth_um(stepped_next.1.unwrap_or(start)),
        world.cells()
    );

    // --- one law, one answer: the state and the being are functions of the world and the count --------
    let mut again = crate::automaton::Automaton::over(&cubic);
    if let Err(refusal) = again.steps(SETTLE_TICKS) {
        eprintln!("U12 stepping: the law's own pour is refused by name: {refusal}");
        return false;
    }
    let one_state = again == plane && again.digest() == state_digest;
    let again_base = drive(
        organisation,
        member,
        &record,
        &dir.join("drafts-base-again"),
        reach,
        &world,
        &classifier,
        &root,
    );
    let again_stepped = drive(
        organisation,
        member,
        &record,
        &dir.join("drafts-stepped-again"),
        reach,
        &stepped,
        &classifier,
        &root,
    );
    let (one_beat_base, one_beat_stepped) = match (again_base, again_stepped) {
        (Ok((_, first, _, _, _)), Ok((_, second, _, _, _))) => {
            (first == base_digest, second == stepped_digest)
        }
        (Err(refusal), _) | (_, Err(refusal)) => {
            eprintln!("U12 stepping: the re-run's drive is refused by name: {refusal}");
            return false;
        }
    };
    // The two planes give the being two different runs, which is the point rather than a defect: the chain
    // is a record of what it did in the world it stood in, and the world moved.
    let two_worlds = base_digest != stepped_digest;
    println!(
        "U12 stepping determinism: two states from the same read and {SETTLE_TICKS} tick(s) are one digest = \
         {one_state} ({}); the same being driven twice over the unstepped plane is one chain digest = \
         {one_beat_base} ({}) and twice over the settled plane is one digest = {one_beat_stepped} ({}); the two \
         planes give two chains = {two_worlds} ({} at Tick(0), {} at rest)",
        short_digest(state_digest),
        short_digest(base_digest),
        short_digest(stepped_digest),
        short_digest(base_digest),
        short_digest(stepped_digest)
    );

    // --- §37's own question, answered by measurement rather than by preference ----------------------
    // The reading the law steps is the surface the cells return; the generator's own surface is the field
    // path. Every differing column is decomposed by cause, and the reading itself is checked against the
    // declared quantisation at **every** column first — the count a defect in the reading would show up in.
    let fields = match being::WorldRead::of(
        organisation,
        member,
        &roster,
        identity,
        Tick(0),
        process::declared_request(),
        1,
    ) {
        Ok(fields) => fields,
        Err(refusal) => {
            eprintln!("U12 stepping: the generator's own field plane is refused by name: {refusal}");
            return false;
        }
    };
    let cells: Vec<f64> = cubic
        .water_columns()
        .iter()
        .map(|column| column.surface_z as f64)
        .collect();
    let attribution = crate::automaton::attribute_routes(cubic.patch(), &fields.elevation, &cells);
    println!(
        "U12 route attribution: {} of {} column(s) route differently from the generator's own float surface — \
         {} swallowed whole in their own voxel, {} with the generator's winner inside its own voxel, {} tied at \
         the steepest quantised drop; {} wrong way, {} unexplained; the reading is the declared voxel at {} \
         column(s) and not at {} column(s) — {} ({})",
        attribution.differs(),
        attribution.columns,
        attribution.swallowed,
        attribution.inside_the_voxel,
        attribution.tied_at_the_top,
        attribution.wrong_way,
        attribution.unexplained,
        attribution.quantised,
        attribution.not_the_voxel,
        if attribution.is_the_voxel() {
            "a faithful consequence of the voxel's quantisation"
        } else {
            "A DEFECT IN THE READING"
        },
        short_digest(being::surface_digest(&world))
    );

    // The bar, one clause per line: the state carries (it took the declared ticks, moved the pit, and has come
    // to rest), the reading the law steps is the declared voxel at every column, the being's need is met **on
    // the settled plane**, and the plane's own change **moves the walk to water somewhere** — the columns it
    // moved are counted above, which is the whole of what "the plane carries tick to tick" buys.
    attribution.is_the_voxel()
        && plane.ticks() == SETTLE_TICKS as u64
        && at_rest
        && stepped.depth_um(sink) > sink_at_zero
        && met_base
        && met_stepped
        && gained + lost + moved > 0
        && one_state
        && one_beat_base
        && one_beat_stepped
        && two_worlds
}

/// The being's **next step** as one comparable pair: how many steps it walks before it drinks, and the column
/// it drinks at. Comparing the pair across two planes is how "the column's change changed its next step" is
/// measured rather than asserted.
pub fn next_step(acts: &[being::Act]) -> (usize, Option<usize>) {
    let walked = acts
        .iter()
        .take_while(|act| !matches!(act, being::Act::Drink { .. }))
        .filter(|act| matches!(act, being::Act::Walk { .. }))
        .count();
    (walked, drank_at(acts).map(|(cell, _)| cell))
}

/// **Where an act list drinks, and how deep the water it found there is** — one scan, one owner: the run's line
/// and [`next_step`] both read this rather than each walking the acts for themselves (the audit's duplicated
/// drink-scan, removed in §35).
pub fn drank_at(acts: &[being::Act]) -> Option<(usize, i64)> {
    acts.iter().find_map(|act| match act {
        being::Act::Drink { cell, depth_um } => Some((*cell, *depth_um)),
        being::Act::Walk { .. } => None,
    })
}

/// One cell state, worded for the run's own line: the deepest column and what it holds.
fn describe_deepest(deepest: Option<(usize, i64)>) -> String {
    match deepest {
        Some((cell, depth)) => format!("deepest {depth} µm at column {cell}"),
        None => "no water standing".to_string(),
    }
}

fn describe_next(next: (usize, Option<usize>)) -> String {
    match next {
        (0, Some(cell)) => format!("drink where it stands (column {cell})"),
        (0, None) => "nothing to do".to_string(),
        (steps, Some(cell)) => format!("{steps} step(s) then drink (column {cell})"),
        (steps, None) => format!("{steps} step(s)"),
    }
}

fn describe_drink(drink: Option<(usize, i64)>) -> String {
    match drink {
        Some((cell, depth_um)) => format!("at column {cell}, {:.6} m of the world's own water", depth_um as f64 / 1_000_000.0),
        None => "nowhere".to_string(),
    }
}

/// **What one drive leaves behind**, named once rather than spelled out in a return type: the acts the being
/// took, its chain's own digest, the reading its last draft earned at the charter's own floor, that draft's own
/// digest, and the declared facts that carried the reading — the last of which the run prints by name, so the
/// fact the floor rests on is visible in the output and not only in the draft.
type Drive = (Vec<being::Act>, Digest, String, Digest, Vec<&'static str>);

/// The first mob's own run: build the being, drive it until its need is met or its reach is spent, and answer
/// with its acts, its chain's digest, the stage reading of its last draft and that draft's own digest. Private
/// to the evidence because it is *the run* rather than a surface: a second caller with its own loop would be a
/// second drive.
#[allow(clippy::too_many_arguments)]
fn drive(
    organisation: being::Organisation,
    member: &'static str,
    record: &std::path::Path,
    drafts: &std::path::Path,
    reach: usize,
    world: &being::WorldRead,
    classifier: &Classifier,
    root: &std::path::Path,
) -> Result<Drive, crate::refusal::Refusal> {
    let mut mob = being::Being::of(member, organisation, record, drafts.to_path_buf(), reach)?;
    let mut acts = Vec::new();
    let mut last_draft = mob.draft_path();
    for _ in 0..=reach {
        last_draft = mob.draft_path();
        match mob.tick(world, classifier, root)? {
            act @ being::Act::Drink { .. } => {
                acts.push(act);
                break;
            }
            act => acts.push(act),
        }
    }
    let (reading, stage_digest, features) =
        being::stage_of(organisation, member, &last_draft, classifier, root)?;
    Ok((acts, being::chain_digest(mob.chain()), reading, stage_digest, features))
}
