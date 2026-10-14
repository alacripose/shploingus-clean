//! The port's own measurement and its two gates, each **run** rather than asserted in prose:
//!
//! * [`the_parents_mesher_and_this_crates_derivation_are_compared_on_one_world`] — the whole comparison,
//!   printed (raw material for the record's tables);
//! * [`every_divergence_between_the_two_derivations_is_named`] — the port's gate: every divergence is one
//!   of the named reasons and the accounting is total;
//! * [`the_parents_streaming_and_this_worlds_shape_are_measured_against_each_other`] — the stream half,
//!   measured against this world's own keyed shape (see the stream half's blocker in [`super`]).

use super::*;
use crate::automaton::{Automaton, SETTLE_TICKS};
use crate::mesh::chunk_centre_distance_m;
use crate::process;
use crate::reference;
use crate::stream::Streamer;

/// The client's own world at its own declared rest, through the one declaration of it.
fn world_at_rest() -> Result<CubicWorld, Refusal> {
    let cubic = crate::surface::client_world()?;
    let mut plane = Automaton::over(&cubic);
    plane.steps(SETTLE_TICKS)?;
    Ok(plane.world().clone())
}

/// A roster and a ladder, through the same doors the client uses.
fn declared() -> (Declared, Ladder) {
    let roster = process::roster().map_err(|refusals| {
        refusals.joined().unwrap_or_else(|| {
            Refusal::new(
                RefusalKind::StageUndeclared,
                "the declared roster refused without naming a refusal, which is itself a defect",
            )
        })
    });
    let roster = roster.expect("the declared roster is admitted");
    let ladder = reference::ladder().expect("the declared ladder is admitted");
    (roster, ladder)
}

fn describe(label: &str, world: &CubicWorld, chunk: ChunkKey) {
    let patch = world.patch();
    let (floor_z, ceiling_z) = world.span_z();
    // The wrapped line's own indentation is **inside** the literal, so it is part of the emitted line:
    // it is left at the width the 2026-10-13 capture (`port-mesh-2026-10-13.txt`) carries rather than
    // re-flowed, so the port's printed measurement does not move when the file moves.
    println!(
        "{label}: patch origin ({}, {}, {}) side {}, vertical span {floor_z}..{ceiling_z}, {} chunk(s) 
             of {}³, {} cell(s), {} solid — measured chunk ({}, {}, {})",
        patch.origin().x,
        patch.origin().y,
        patch.origin().z,
        patch.side(),
        world.chunks(),
        crate::coordinates::CHUNK_SIDE,
        world.cells(),
        world.solid(),
        chunk.x,
        chunk.y,
        chunk.z,
    );
}

/// A world of `chunks × chunks` chunk(s) a side, built **through the client's own doors** — the same
/// roster, identity, ladder, request and [`CubicWorld::from_fields`] call
/// [`crate::surface::client_world`] makes, with one declared difference: the patch is wider. It exists
/// because a differential needs a chunk whose neighbourhood the world can answer for, and the client's own
/// world is two chunks a side, so every one of its chunks is a boundary chunk.
fn measurement_world(chunks: u32) -> Result<CubicWorld, Refusal> {
    let roster = process::roster().map_err(|refusals| {
        refusals.joined().unwrap_or_else(|| {
            Refusal::new(
                RefusalKind::StageUndeclared,
                "the declared roster refused without naming a refusal, which is itself a defect",
            )
        })
    })?;
    let identity = reference::identity()?;
    let ladder = reference::ladder()?;
    let origin = process::declared_request().patch()?.origin();
    let side = crate::coordinates::CHUNK_SIDE as u32 * chunks;
    let request = crate::stage::ProcessRequest::Region { origin, side };
    CubicWorld::from_fields(&roster, identity, crate::world::Tick(0), request, &ladder, 1)
}

/// **The port's own measurement**, printed beside the two derivations (P4).
///
/// Two subjects, because the first one measured a fact the second one has to work around:
///
/// * the **client's own world** (2 × 2 chunk(s) a side), on which the parent's mesher **refuses** — the
///   parent reads one voxel outside every face of the chunk it is given, and on a finite patch every
///   chunk is a boundary chunk, so the halo is a place the world does not own. The parent's refusal is
///   the correct answer under its own contract (*"the alternative is claiming air, which would punch a
///   halo of visible faces around every loaded chunk"*), and it is printed here in the parent's own
///   words rather than worked around here.
/// * a **measurement fixture** (3 × 3 chunk(s) a side) built through the same declared doors, whose
///   middle chunk *has* an answerable neighbourhood — the chunk on which the two derivations can be
///   compared at all.
#[test]
fn the_parents_mesher_and_this_crates_derivation_are_compared_on_one_world() {
    let (roster, ladder) = declared();
    let client = world_at_rest().expect("the client's own world is declared");
    let client_chunk =
        client.first_chunk().expect("the client's own world answers for a chunk");
    let fixture = measurement_world(3).expect("the 3 x 3 measurement fixture is admitted");
    let inner = fixture.top_chunk_at(1, 1).expect("the fixture owns its middle chunk");

    for mapping in [Water::FullCube, Water::AtTheParentsOwnResolution] {
        println!("\nport-bridge: water handed over as {}", mapping.name());
        describe("  the client's own world", &client, client_chunk);
        describe("  the 3 x 3 measurement fixture", &fixture, inner);
        for (label, cube, chunk) in [
            ("the client's own world", &client, client_chunk),
            ("the 3 x 3 measurement fixture", &fixture, inner),
        ] {
            for outside in [Outside::World, Outside::WorldOrNothing, Outside::Nothing] {
                match differential(cube, chunk, ladder.clone(), &roster, mapping, outside) {
                    Ok(report) => {
                        println!("  {label}, outside = {outside:?}:\n    {}", report.line());
                        report_divergences(&report);
                    }
                    Err(refusal) => println!(
                        "  {label}, outside = {outside:?}: the parent's own mesher REFUSED the chunk, in \
                         its own words: {}",
                        refusal.detail()
                    ),
                }
            }
        }
    }
}

/// Every divergence, named by kind with its count and its examples (the accounting itself).
fn report_divergences(report: &Differential) {
    if report.only_parent_total == 0 && report.only_clean_total == 0 {
        println!("    the two derivations cover exactly the same unit faces");
        return;
    }
    for line in report.divergence_lines() {
        println!("{line}");
    }
}

/// **The port's own gate, executable** — the claim that decides whether the parent's derivation may
/// become this crate's, over both worlds this module can build (the client's own, and the 3 × 3 fixture
/// whose middle chunk has a neighbourhood the world can answer for), all three declared halos, and both
/// declared water mappings.
///
/// Two assertions, and each is a real claim rather than a print:
///
/// 1. **every divergence is one of the three named reasons** ([`Divergence`]), and the counts sum to the
///    difference between the two face sets, so there is no residual wearing a reason's clothes. A
///    `Divergence::Unnamed` face is a disagreement about geometry both derivations can see, and it is the
///    only kind that could justify one derivation replacing the other.
/// 2. **the comparison is not vacuous:** the parent covered at least one unit face whose far side the
///    world owns, and this crate's own derivation drew at least one face, on every case that ran.
///
/// The gate does **not** pass as *equality* on any case (`agrees()` is false, and the divergence is
/// printed by the measurement): the parent draws the world's own edge as air and this clean room draws it
/// as an edge, and the wetted cells' two readings differ. That is why no module of this crate was
/// replaced by the bridge.
#[test]
fn every_divergence_between_the_two_derivations_is_named() {
    let (roster, ladder) = declared();
    let client = world_at_rest().expect("the client's own world is declared");
    let client_chunk =
        client.first_chunk().expect("the client's own world answers for a chunk");
    let fixture = measurement_world(3).expect("the 3 x 3 measurement fixture is admitted");
    let fixture_chunk =
        fixture.top_chunk_at(1, 1).expect("the fixture owns its middle chunk");

    let mut cases = 0_u64;
    let mut faces_compared = 0_u64;
    let mut named = 0_u64;
    for mapping in [Water::FullCube, Water::AtTheParentsOwnResolution] {
        for outside in [Outside::World, Outside::WorldOrNothing, Outside::Nothing] {
            for (label, cube, chunk) in [
                ("the client's own world", &client, client_chunk),
                ("the 3 x 3 measurement fixture", &fixture, fixture_chunk),
            ] {
                let report = match differential(
                    cube,
                    chunk,
                    ladder.clone(),
                    &roster,
                    mapping,
                    outside,
                ) {
                    Ok(report) => report,
                    // The parent's own contract refuses a chunk whose halo the world cannot answer for;
                    // that refusal is the parent's answer and is not a case the gate can compare.
                    Err(_) => continue,
                };
                cases += 1;
                assert!(
                    report.the_port_gate_passes(),
                    "{label}, {outside:?}, water {mapping:?}: the gate is not met — {} unnamed parent-only \
                     and {} unnamed clean-room-only of {} / {} divergent face(s); the accounting is {} (the \
                     counts sum to {} of {} parent-only and {} of {} clean-room-only)",
                    report.only_parent.of(Divergence::Unnamed),
                    report.only_clean.of(Divergence::Unnamed),
                    report.only_parent_total,
                    report.only_clean_total,
                    if report.only_parent.total() == report.only_parent_total
                        && report.only_clean.total() == report.only_clean_total
                    {
                        "total"
                    } else {
                        "NOT total — a counting defect"
                    },
                    report.only_parent.total(),
                    report.only_parent_total,
                    report.only_clean.total(),
                    report.only_clean_total,
                );
                assert!(
                    report.parent_faces_the_world_owns > 0 && report.clean_faces > 0,
                    "{label}, {outside:?}, water {mapping:?}: the comparison is vacuous — the parent \
                     covered {} owned face(s) and this crate drew {} face(s)",
                    report.parent_faces_the_world_owns,
                    report.clean_faces
                );
                faces_compared += report.parent_faces_the_world_owns as u64;
                named += report.only_parent_total as u64 + report.only_clean_total as u64;
            }
        }
    }
    println!(
        "port-gate: {cases} case(s) compared, {faces_compared} unit face(s) the world owns on the parent's \
         side, {named} divergent face(s) and **every one of them named** — 0 unnamed"
    );
}

/// **The stream half of the seam, measured against this world's own shape.**
///
/// The parent's residency names a cube of `(2r + 1)³` chunks around a viewer and generates any of them on
/// demand ([`crate::port::PARENT_HORIZONS`], pinned by digest). This world owns one finite patch
/// and refuses every position outside it, so the two are not the same object and the numbers say by how
/// much. Nothing here is asserted about the parent — the numbers are printed, and what is asserted is
/// this world's own side of them: it is chunk-addressable, its chunks are whole, and it owns far fewer
/// chunks than any of the parent's four horizons names.
///
/// **And since 2026-10-14c it measures the consumer**, not only the shape: the streamed residency
/// ([`crate::stream`]) is fed by the world's own keyed door at each of the parent's four horizons, and
/// what it holds, what it derives, what it serves and the horizon at which the first chunk is evicted are
/// printed rather than argued — with this world's own far corner (45.2548 m from its first chunk) naming
/// the eviction boundary, because a residency around a finite world is bounded by the world.
#[test]
fn the_parents_streaming_and_this_worlds_shape_are_measured_against_each_other() {
    let client = world_at_rest().expect("the client's own world is declared");
    let fixture = measurement_world(3).expect("the 3 x 3 measurement fixture is admitted");

    for (label, cube) in [
        ("the client's own world", &client),
        ("the 3 x 3 measurement fixture", &fixture),
    ] {
        let patch = cube.patch();
        let (floor_z, ceiling_z) = cube.span_z();
        println!(
            "stream-seam: {label}: patch origin ({}, {}, {}) side {}, vertical span {floor_z}..{ceiling_z} \
             = {} chunk layer(s), {} chunk(s) owned, {} cells, {} solid",
            patch.origin().x,
            patch.origin().y,
            patch.origin().z,
            patch.side(),
            (ceiling_z - floor_z) / CHUNK_SIDE,
            cube.chunks(),
            cube.cells(),
            cube.solid(),
        );
    }
    for (name, radius) in PARENT_HORIZONS {
        println!(
            "stream-seam: the parent's {name} horizon: radius {radius} chunk(s) -> {} chunk(s) named \
             around a viewer; this world owns {} of them ({:.4}%)",
            horizon_chunks(radius),
            client.chunks(),
            100.0 * client.chunks() as f64 / horizon_chunks(radius) as f64,
        );
    }

    // **The streamed residency at each of those four horizons (2026-10-14c).** The parent names a cube of
    // keys per horizon and generates every one of them on demand; this world names four and generates no
    // others, so what a streaming consumer holds is bounded by the world and not by the horizon. The
    // streamer is U4's own and its cells come from **the world's own keyed door** ([`CubicWorld::chunk`],
    // which is what [`crate::stream::Streamer`] takes since 2026-10-14d), one key at a time — the same cells
    // the drawn path draws, not a second derivation's.
    let source = &client;
    let home = client.first_chunk().expect("the client's world has a chunk");
    let mut reached_all = true;
    for (name, radius) in PARENT_HORIZONS {
        let horizon =
            ResidencyHorizon::m(radius as f64 * CHUNK_SIDE as f64).expect("a positive horizon");
        let mut streamer = Streamer::over(source, horizon);
        let step = streamer.observe(source, home).expect("the world answers for its own chunk");
        reached_all &= step.resident == client.chunks();
        println!(
            "stream-seam: the parent's {name} horizon = {:.0} m: the streamer holds {} of this world's {} \
             chunk(s) ({} cell(s), {} derived, {} served) — the horizon is wider than the world, so nothing \
             it names beyond those {} can be held",
            horizon.radius_m(),
            step.resident,
            client.chunks(),
            streamer.cells(),
            step.derived,
            step.served,
            client.chunks(),
        );
    }
    assert!(
        reached_all,
        "every one of this world's chunks must be held at each of the parent's horizons"
    );

    // **The eviction boundary, and a walk across it.** The world's own far corner from the camera's chunk
    // is what bounds a residency over a finite world: at the horizon that just reaches it every chunk is
    // held, one metre inside it the corner chunk is not, and a camera that moves so the corner leaves the
    // horizon pays one derivation to have it back — the residency's own re-request, counted.
    let far = source
        .keys()
        .into_iter()
        .filter_map(|key| chunk_centre_distance_m(home, key))
        .fold(0.0f64, f64::max);
    let reaches = ResidencyHorizon::m(far.ceil()).expect("a positive horizon");
    let inside = ResidencyHorizon::m(far.ceil() - 1.0).expect("a positive horizon");
    let mut at_edge = Streamer::over(source, reaches);
    let at_edge_step = at_edge.observe(source, home).expect("the world answers for its own chunk");
    let mut tight = Streamer::over(source, inside);
    let start = tight.observe(source, home).expect("the world answers for its own chunk");
    let east = ChunkKey::new(home.x + 1, home.y, home.z);
    let moved = tight.observe(source, east).expect("the world answers for its own chunk");
    let back = tight.observe(source, home).expect("the world answers for its own chunk");
    assert_eq!(
        at_edge_step.resident,
        client.chunks(),
        "the horizon that reaches the far corner holds the whole world"
    );
    assert!(
        start.resident < client.chunks(),
        "one metre inside it, the corner chunk is evicted"
    );
    assert!(back.derived >= 1, "a released chunk costs a derivation to come back");
    println!(
        "stream-seam: the first eviction: the world's furthest chunk from its own first chunk ({}, {}, {}) \
         is {far:.4} m away, so the streamer holds {} of {} chunk(s) at {:.0} m and {} at {:.0} m; a walk \
         from there: one chunk east evicts {} and holds {}; back derives {} and serves {}",
        home.x,
        home.y,
        home.z,
        at_edge_step.resident,
        client.chunks(),
        reaches.radius_m(),
        start.resident,
        inside.radius_m(),
        moved.evicted,
        moved.resident,
        back.derived,
        back.served,
    );

    // This world's own side of the mismatch, asserted: every chunk it owns is whole (a full
    // `CHUNK_SIDE³` cell count, so a chunk is addressable as a unit), and the chunks it owns are exactly
    // the ones its own footprint and vertical span name.
    let side_chunks = client.patch().side() as usize / CHUNK_SIDE as usize;
    let layers = ((client.span_z().1 - client.span_z().0) / CHUNK_SIDE) as usize;
    assert_eq!(
        client.chunks(),
        side_chunks * side_chunks * layers,
        "this world's chunks are not the cube its own footprint and span name"
    );
    assert_eq!(
        client.cells(),
        client.chunks() * (CHUNK_SIDE * CHUNK_SIDE * CHUNK_SIDE) as usize,
        "a chunk of this world is not whole, so a chunk key does not address it"
    );

    // And the position outside it is refused **by name**, which is the fact the parent's streamer never
    // has to face: a chunk key it can name is a chunk it can generate.
    let outside = VoxelPos::new(0, 0, client.span_z().1);
    let refusal = client
        .cell(outside)
        .expect_err("a position above this world's own ceiling is not in this world");
    println!(
        "stream-seam: this world's own answer above its ceiling: {}",
        refusal.detail()
    );

    // **The two keyed doors, and what is still not reachable through either (2026-10-14d).** This world
    // answers the parent's residency question — *give me the chunk at this key* — **twice over**: the world's
    // own door ([`CubicWorld::chunk`], which is what [`crate::stream::Streamer`] consumes, one key at a time)
    // and the declared source ([`ChunkSource::chunk`], which derives the declared surface one key at a time).
    // The first is the one the camera's residency and the drawn frame read; the second is a *derivation*, and
    // the two are **not the same cells** — which is why the drawn path takes the first. What still cannot be
    // driven is the parent's own `Resident`: `pub fn new(source: WorldSource)` takes the parent's generator
    // **by type** (`crates/voxel-stream/src/resident.rs`, pinned by sha256 in `pinned-parent-bytes.txt`), so
    // no value built here can be handed to it without editing a parent byte (forbidden) or adopting U3's
    // seam.
    //
    // And the **differential between the two doors**, on this client's own subject: the world this client
    // *shows* is the declared surface **stepped** by the plane's own law ([`world_at_rest`], `SETTLE_TICKS`),
    // while the declared source derives the declared surface. Counting the cells that differ is how this
    // measurement says *which* world each door answers rather than asserting that they agree — and the number
    // it prints is the measured size of the second path the frame no longer reads.
    let (roster, ladder) = declared();
    let identity = reference::identity().expect("the reference identity");
    let declared_world =
        crate::surface::client_world().expect("the client's own world, through its own door");
    let declared_source = crate::chunk::ChunkSource::over(
        &roster,
        identity,
        crate::world::Tick(0),
        crate::stage::ProcessRequest::Region {
            origin: declared_world.patch().origin(),
            side: declared_world.patch().side(),
        },
        &ladder,
        1,
    )
    .expect("the client's own read of the declared surface resolves");
    let keys = declared_source.keys();
    assert_eq!(
        keys.len(),
        declared_world.chunks(),
        "the keyed door names {} chunk(s) and the world's own door builds {}",
        keys.len(),
        declared_world.chunks()
    );
    let mut cells = 0usize;
    let mut stepped = 0usize;
    for key in &keys {
        let derived = declared_source
            .chunk(*key)
            .expect("a key the source names is one it answers, and this world owns four of them");
        assert_eq!(
            derived.cells().len(),
            (CHUNK_SIDE * CHUNK_SIDE * CHUNK_SIDE) as usize,
            "a chunk derived alone is whole"
        );
        let shown = client.chunk(*key).expect("a key this world owns is one it holds");
        stepped += derived
            .cells()
            .iter()
            .zip(shown.cells())
            .filter(|(declared, at_rest)| declared != at_rest)
            .count();
        cells += derived.cells().len();
    }
    println!(
        "stream-seam: the two keyed doors (2026-10-14d): the declared source names {} chunk key(s) and answers \
         any one of them alone ({cells} cell(s), each chunk whole); the world this client *shows* has been \
         stepped by the plane's own law ({SETTLE_TICKS} tick(s)) and is the door the camera's residency and the \
         drawn frame read, so {stepped} of its {cells} cell(s) differ between the two doors — the second path, \
         measured, and not one the frame reads",
        keys.len()
    );
    println!(
        "stream-seam: the parent's own `Resident` still cannot be driven from a per-chunk door — `pub fn \
         new(source: WorldSource)` takes the parent's `WorldSpec` generator by type \
         (crates/voxel-stream/src/resident.rs, pinned by sha256)"
    );
}
