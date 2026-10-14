//! **U12's stepping instrument** — the record's own owed line, measured through the crate's public surface
//! rather than its internals:
//!
//! > *a stepping cellular automaton on the field plane is **owed**: state that carries tick to tick, so a
//! > rained-on or drained column changes the being's next step* (`docs/CLEAN-ROOM-PLAN-2026-10-11.md` §33,
//! > closed by §34)
//!
//! What is measured here is that sentence and nothing beside it: the plane's state **carries** (it takes ticks,
//! it is a pure function of the read and the count, and it starts from the read's own water rather than from a
//! plane of zeros), a column the generator left **dry** is rained on, and **the same being at the same cell
//! takes a different next step** on the plane the world's own routing has stepped than on the plane it was
//! placed in — drinking, in fact, at a column the generator left dry. The binary's own `check_u12_stepping`
//! drives the same behaviour with the same comparison ([`evidence::next_step`]), so the test and the running
//! product measure one thing rather than two descriptions of one.

use std::path::{Path, PathBuf};

use shploingus_clean_room::automaton::{Automaton, RAIN_UM_PER_TICK};
use shploingus_clean_room::being::{self, Act, Being, Organisation, WorldRead};
use shploingus_clean_room::chunk::CubicWorld;
use shploingus_clean_room::evidence::next_step;
use shploingus_clean_room::field::fields_at;
use shploingus_clean_room::process;
use shploingus_clean_room::reference;
use shploingus_clean_room::stage::FieldId;
use shploingus_clean_room::von::{self, Classifier};
use shploingus_clean_room::world::Tick;

/// The declared tick count these measurements advance the plane by — the binary's own
/// `evidence::STEPPING_TICKS`, and the same number the being's record declares as its reach.
const TICKS: usize = 8;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("shploingus-clean-u12-step-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

/// The one world every measurement here steps: the declared fields filled into their **cubic world** (§38)
/// and read through the one reading every consumer shares — plus the world itself, because the law's state
/// *is* its cells.
fn world() -> (CubicWorld, WorldRead) {
    let roster = process::roster().expect("the declared roster");
    let identity = reference::identity().expect("the reference identity");
    let request = process::declared_request();
    let patch = request.patch().expect("the declared request is a patch");
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
    let cubic = CubicWorld::from_surface(patch, &values[0], &values[1], &values[2], &ladder)
        .expect("the declared surface fills a cubic world");
    let world = WorldRead::over_cubic(Organisation::first(), "the first mob", &cubic)
        .expect("the world's own cells are readable");
    (cubic, world)
}

/// **The world's own reading of the law's state** — `over_cubic` of the world the law is stepping (§38).
fn read_of(plane: &Automaton) -> WorldRead {
    WorldRead::over_cubic(Organisation::first(), "the first mob", plane.world())
        .expect("the stepped world's own cells are readable")
}

/// One declared count of ticks, with the law's own pour refusal a panic here rather than a silent default:
/// the declared world's span holds the rain, and a test that did not would be a defect worth seeing.
fn steps(plane: &mut Automaton, count: usize) {
    plane
        .steps(count)
        .expect("the declared world's span holds the law's own water");
}

fn soul(dir: &Path, cell: usize) -> PathBuf {
    let record = dir.join("being.md");
    std::fs::write(
        &record,
        format!(
            "---\nrecord: being\nbeing: the first mob\norganisation: the undertow congregation\ncell: {cell}\n---\n\n\
             this being is a member of the undertow congregation. Its need is water and its acts are its own.\n"
        ),
    )
    .expect("the being's own record is written");
    record
}

fn classifier() -> Classifier {
    Classifier::classified().expect("the sixteen labelled readings calibrate the classifier")
}

/// **The being's own drive over one plane**, up to and including its first drink — the same loop the binary's
/// private `drive` runs, read through the public surface, answering with the acts and the chain digest of that
/// world.
fn drive(world: &WorldRead, dir: &Path, start: usize, reach: usize) -> (Vec<Act>, String) {
    let mut mob = Being::of(
        "the first mob",
        Organisation::first(),
        &soul(dir, start),
        dir.join("drafts"),
        reach,
    )
    .expect("the being is ensouled");
    let classifier = classifier();
    let root = von::root();
    let mut acts = Vec::new();
    for _ in 0..=reach {
        match mob.tick(world, &classifier, &root).expect("the drive is not refused") {
            act @ Act::Drink { .. } => {
                acts.push(act);
                break;
            }
            act => acts.push(act),
        }
    }
    (acts, being::chain_digest(mob.chain()).to_hex())
}

/// The state **carries**: the plane starts from the read's own water, takes the declared ticks, and the state
/// the being reads is the state the law produced.
#[test]
fn the_plane_carries_state_from_tick_to_tick() {
    let (cubic, world) = world();
    let standing_in_the_read = (0..world.cells()).filter(|cell| world.water_stands(*cell)).count();
    let mut plane = Automaton::over(&cubic);
    assert_eq!(plane.ticks(), 0, "the state starts at the world's own tick");
    assert_eq!(plane.tick(Tick(0)), Tick(0));
    assert_eq!(
        plane.standing(),
        standing_in_the_read,
        "the state the law starts from is the cells' own water, not a plane of zeros"
    );
    let sink = being::dry_sink(&world).expect("the world's own law names a dry sink");
    assert!(!world.water_stands(sink), "the sink is dry at the base world");

    steps(&mut plane, TICKS);
    assert_eq!(plane.ticks(), TICKS as u64);
    assert_eq!(plane.tick(Tick(0)), Tick(TICKS as u64), "eight ticks on, the plane stands at Tick(8)");
    assert!(plane.water_stands(sink), "the rained-on column stands water");
    assert!(
        plane.depth_um(sink) > world.depth_um(sink),
        "and it holds more than the filled surface did: {} µm against {}",
        plane.depth_um(sink),
        world.depth_um(sink)
    );
    assert_ne!(
        plane.standing(),
        standing_in_the_read,
        "the plane's own water moved: {} column(s) stood water in the read, {} stand it after {TICKS} tick(s)",
        standing_in_the_read,
        plane.standing()
    );

    // The world the being reads is the law's own state — one water, not two.
    // **No base read is passed**: the law holds the world it steps, and `over_cubic` reads that world's
    // cells, so the only state a stepped read can carry is the law's own — a mismatched patch or routing
    // graph cannot be expressed.
    let stepped = read_of(&plane);
    assert_eq!(stepped.patch, world.patch);
    assert_eq!(stepped.receivers, world.receivers, "one routing graph, not a second one");
    assert_eq!(stepped.depth_um(sink), plane.depth_um(sink), "the read stands the law's water");
    assert_ne!(stepped.depth_um(sink), world.depth_um(sink), "and not the world it stepped from");
}

/// **The bar**: the same being, at the same cell of the same patch, takes a different next step on the plane
/// the world's own routing has stepped. Its need is met on both planes, so what changed is the walk, not the
/// need.
#[test]
fn a_rained_on_column_changes_the_beings_next_step() {
    let (cubic, world) = world();
    let start = being::start_cell(&world, TICKS).expect("the world's own law names a start cell");
    let mut plane = Automaton::over(&cubic);
    steps(&mut plane, TICKS);
    let stepped = read_of(&plane);

    let (base_acts, base_digest) = drive(&world, &scratch("base"), start, TICKS);
    let (stepped_acts, stepped_digest) = drive(&stepped, &scratch("stepped"), start, TICKS);
    let base_next = next_step(&base_acts);
    let stepped_next = next_step(&stepped_acts);
    assert!(
        base_next.1.is_some() && stepped_next.1.is_some(),
        "the need is met on both planes: {base_next:?} and {stepped_next:?}"
    );
    assert_ne!(
        base_digest, stepped_digest,
        "two worlds, two chains: a record of what it did, in the world it stood in"
    );
    // The walk from this cell is the same one on both planes, and that is a **measured** fact rather than a
    // gap in the bar: the ground does not move, so the route a being walks is the world's own and unchanged;
    // what the plane moves is where along that route the water stands. Stated here so the reader is not left
    // to infer that the first mob's own step is the one that changed.
    assert_eq!(
        base_next, stepped_next,
        "the world's own route from this cell is the world's, and it did not move: {base_next:?}"
    );
    assert_ne!(
        base_acts, stepped_acts,
        "what did move is the water it finds there, and the record of the drink carries it"
    );

    // **§33's sentence, measured column by column**: *a rained-on or drained column changes the being's next
    // step*. The walk is taken from every column of the patch on both planes, by the being's own walk
    // ([`being::walk_to_water`]), and the columns whose answer moved are counted — a number, not a claim, and
    // one that would be zero if the plane carried nothing.
    // The walk from every column of the patch, on both planes, by the being's own walk
    // ([`being::walk_to_water`]) — so §33's sentence is a number rather than a claim, and the **direction** of
    // every change is named: a walk the rain made possible, a walk the drain took away, or a walk that moved.
    let mut gained = 0;
    let mut lost = 0;
    let mut moved = 0;
    let mut gained_at = None;
    for cell in 0..world.cells() {
        match (
            being::walk_to_water(&world, cell, TICKS),
            being::walk_to_water(&stepped, cell, TICKS),
        ) {
            (None, Some(_)) => {
                gained += 1;
                gained_at.get_or_insert(cell);
            }
            (Some(_), None) => lost += 1,
            (Some(base), Some(moved_to)) if base != moved_to => moved += 1,
            _ => {}
        }
    }
    let drank = base_next.1.expect("the being drinks on the plane it was placed in");
    println!(
        "U12 stepping walk: {} of {} column(s) walk to water differently on the settled plane than on the \
         plane the being was placed in (reach {TICKS}) — {gained} walk(s) the rain made possible, {lost} the \
         drain took away, {moved} moved; at the being's own cell {start} the walk is {base_next:?} on both, and \
         the water it reaches there moved from {} µm to {} µm",
        gained + lost + moved,
        world.cells(),
        world.depth_um(drank),
        stepped.depth_um(drank)
    );
    assert!(
        gained + lost + moved > 0,
        "the plane's own change must move the being's next step somewhere: {} column(s) of {}",
        gained + lost + moved,
        world.cells()
    );

    // **And the being's own drive agrees with that walk, refusing exactly where the walk has nowhere to go:**
    // the same being, at a column whose route the rain made possible, is refused `water_unreachable` on the
    // plane it was placed in and drinks on the settled one. The refusal is the same one the check's own bar
    // names, so two instruments measure one law.
    let gained_at = gained_at.expect("a column whose walk the rain made possible");
    let refused_dir = scratch("gained-refused");
    let mut refused = Being::of(
        "the first mob",
        Organisation::first(),
        &soul(&refused_dir, gained_at),
        refused_dir.join("drafts"),
        TICKS,
    )
    .expect("the being is ensouled");
    // Its whole reach, one step at a time: the walk is spent and the need is refused by name — the same
    // refusal [`shploingus_clean_room::refusal::RefusalKind::WaterUnreachable`] the check's own bar names.
    let mut base_refusal = None;
    for _ in 0..=TICKS {
        match refused.tick(&world, &classifier(), &von::root()) {
            Ok(_) => {}
            Err(refusal) => {
                base_refusal = Some(refusal.kind());
                break;
            }
        }
    }
    assert_eq!(
        base_refusal,
        Some(shploingus_clean_room::refusal::RefusalKind::WaterUnreachable),
        "column {gained_at}: on the plane it was placed in, the walk reaches no water and the need is refused"
    );
    let (gained_acts, _) = drive(&stepped, &scratch("gained"), gained_at, TICKS);
    assert_eq!(
        next_step(&gained_acts),
        being::walk_to_water(&stepped, gained_at, TICKS)
            .map(|(steps, drank)| (steps, Some(drank)))
            .expect("the rain made this walk possible"),
        "column {gained_at}: and on the settled plane the same being takes the walk's own answer — its \
         count and its column"
    );

    // The water the first mob drinks is the **settled plane's own** reading of the cells — not the depth the
    // generator's filled surface stood at that column, and not one tick of rain: what a plane that has been
    // stepped says about itself, which the filled surface cannot.
    let water_there = stepped.depth_um(drank);
    assert!(stepped.water_stands(drank));
    assert_ne!(
        water_there,
        world.depth_um(drank),
        "it is not what the generator's own filled surface stood there: {} µm at rest against {} µm",
        water_there,
        world.depth_um(drank)
    );
    assert_ne!(
        water_there,
        RAIN_UM_PER_TICK,
        "nor a single tick's rain: the marsh has been thinning since the lake drained into it"
    );
    // **And the read is a float view of the cells, to the micrometre**: `WorldRead` carries water as metres
    // (§35's surface), so its own `depth_um` can answer one micrometre below the cells' exact `water_um` — the
    // truncation §34 measured. The law's state is the authority; the gap is measured at every column here
    // rather than left for a reader to find, and it is bounded by one micrometre.
    let mut low = 0;
    for cell in 0..world.cells() {
        let (read, cells) = (stepped.depth_um(cell), plane.depth_um(cell));
        assert!(
            (read - cells).abs() <= 1,
            "column {cell}: the read answers {read} µm where the cells hold {cells} µm"
        );
        if read < cells {
            low += 1;
        }
    }
    println!(
        "U12 stepping read: the read's own whole micrometres answer below the cells' at {low} of {} \
         column(s), never by more than one",
        world.cells()
    );
    assert!(
        (stepped.depth(drank) - plane.depth_um(drank) as f64 / 1_000_000.0).abs() < 1e-6,
        "and the depth itself is the law's own state: {} m against {} µm",
        stepped.depth(drank),
        plane.depth_um(drank)
    );
}

/// **One water truth** (§35): the stepped read carries the water the law produced, and its surface is derived
/// from that one depth — so the struct cannot answer the water question twice, and a stepped world is no
/// longer indistinguishable from the static one by [`being::surface_digest`], which hashes the water fields.
#[test]
fn the_reads_carry_one_water_truth() {
    let (cubic, world) = world();
    let mut plane = Automaton::over(&cubic);
    steps(&mut plane, TICKS);
    let stepped = read_of(&plane);
    for cell in 0..world.cells() {
        let surface = stepped.water_level[cell] - stepped.elevation[cell];
        assert!(
            (surface - stepped.depth(cell)).abs() < 1e-9,
            "the stepped read's surface is its depth with the ground added, column {cell}: {surface} against {}",
            stepped.depth(cell)
        );
        // And the same identity holds in the read the generator produced, so 'depth is the authority' is a
        // property of the struct rather than of one constructor.
        let base_surface = world.water_level[cell] - world.elevation[cell];
        assert!((base_surface - world.depth(cell)).abs() < 1e-9, "column {cell} at the base read");
    }
    // The defect the audit named: with the generator's surface carried over the stepped state, this digest
    // was the **same** for both worlds — the two reads were interchangeable to anything hashing the water.
    assert_ne!(
        being::surface_digest(&stepped).to_hex(),
        being::surface_digest(&world).to_hex(),
        "a stepped world must not digest as the static one"
    );
    assert_eq!(
        stepped.depth_um(0),
        plane.depth_um(0),
        "and the read's water is the law's own, column by column"
    );
}

/// One law, one answer: the state and the being are functions of the read and the tick count, so two runs
/// produce one state digest and one chain digest — the plan's own determinism bar, on the stepping plane.
#[test]
fn two_states_from_one_read_are_one_digest() {
    let (cubic, world) = world();
    let mut a = Automaton::over(&cubic);
    steps(&mut a, TICKS);
    let mut b = Automaton::over(&cubic);
    steps(&mut b, 1);
    steps(&mut b, 1);
    steps(&mut b, TICKS - 2);
    assert_eq!(a, b, "the state is the count, not the calls");
    assert_eq!(a.digest().to_hex(), b.digest().to_hex(), "two states, one identity");

    let stepped = read_of(&a);
    let start = being::start_cell(&world, TICKS).expect("a start cell");
    let (_, first) = drive(&stepped, &scratch("det-a"), start, TICKS);
    let (_, second) = drive(&stepped, &scratch("det-b"), start, TICKS);
    assert_eq!(first, second, "two runs over one stepped plane, one chain digest");
    assert_eq!(first.len(), 64);
}
