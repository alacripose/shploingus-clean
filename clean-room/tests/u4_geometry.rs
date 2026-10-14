//! **U4's end-to-end check, driven through the crate's public API.**
//!
//! Derived from `specs/004-u4-mesh-stream-work/spec.md` (three user stories and their acceptance
//! scenarios) and `contracts/u4-seams.md` C1–C4. It exercises the **reference world** — U3's fifteen
//! declared stages, U2's starter material registry, U2's `World` — so the properties are measured on
//! a real world rather than on a fixture: the same world yields the same geometry, a change to the
//! world invalidates exactly the geometry derived from it and nothing else, the work accounting agrees
//! with an **independent recount of the same cells**, and the seam this slice was ordered for — the
//! greedy merge over the cells' own voxel faces — is exercised on **the law's own world** too, where the
//! resident set's key is the cells' own state digest.

use shploingus_clean_room::automaton::Automaton;
use shploingus_clean_room::chunk::CubicWorld;
use shploingus_clean_room::coordinates::{ChunkKey, VoxelPos, CHUNK_SIDE};
use shploingus_clean_room::field::fields_at;
use shploingus_clean_room::material::{MaterialId, MaterialRegistry};
use shploingus_clean_room::mesh::{
    check_horizons, chunk_columns, recount_faces, ring_columns, Axis, Budget, Effort, FieldHorizon,
    Footprint, GeometryKey, Ladder, Layer, Level, Mesh, Mesher, RenderHorizon, ResidencyHorizon,
    Resident, SimulationHorizon, WorkLedger, MESH_RING,
};
use shploingus_clean_room::process;
use shploingus_clean_room::refusal::RefusalKind;
use shploingus_clean_room::stage::{Declared, FieldId, Patch, ProcessRequest};
use shploingus_clean_room::world::{DeclaredPeriod, Tick, World, WorldIdentity};

/// The reference world of this check: named by seed, period and tick (C-46 — every number says which
/// world it is of).
const SEED: u64 = 0x5EED_BEEF;

fn identity() -> WorldIdentity {
    let period = DeclaredPeriod::seconds(1).expect("a one-second period is not zero");
    WorldIdentity::new(SEED, period)
}

fn world_at(tick: u64) -> World {
    World::new(identity()).with_tick(Tick(tick))
}

fn reference_roster() -> Declared {
    process::roster().expect("the reference roster admits")
}

/// The declared palette: the admitted ground materials in id order, laid over the world's six
/// declared lithology classes, with water for the surface layer. A palette thinner than the class
/// count is a **declaration** here, and the mapping is the mesher's to print.
fn reference_ladder(registry: &MaterialRegistry) -> Ladder {
    let ground = [MaterialId(1), MaterialId(2), MaterialId(3), MaterialId(4), MaterialId(6)];
    Ladder::over_palette(registry, &ground, 6, MaterialId(5))
        .expect("the starter registry admits every id in the palette")
}

fn residency(metres: f64) -> ResidencyHorizon {
    ResidencyHorizon::m(metres).expect("a positive residency radius")
}

/// The cells a reading of `side × side` columns at `origin` stands on: the same declared fields the
/// mesher's reading door reads, filled by the one constructor.
fn cube_over(roster: &Declared, ladder: &Ladder, origin: VoxelPos, side: u32) -> CubicWorld {
    let values = fields_at(
        roster,
        identity(),
        Tick(0),
        &[FieldId::Elevation, FieldId::WaterLevel, FieldId::Lithology],
        ProcessRequest::Region { origin, side },
        1,
    )
    .expect("the reference fields answer");
    CubicWorld::from_surface(
        Patch::new(origin, side).expect("a non-empty patch"),
        &values[0],
        &values[1],
        &values[2],
        ladder,
    )
    .expect("the reference surface fills a cubic world")
}

fn chunk_origin(chunk: ChunkKey) -> VoxelPos {
    chunk.origin().expect("the chunk has an origin")
}

/// **The cells one chunk's reading stands on**: the same declared fields the mesher's reading door
/// reads, filled by the one constructor — the world the independent recount is taken of.
fn chunk_cube(roster: &Declared, ladder: &Ladder) -> CubicWorld {
    cube_over(roster, ladder, chunk_origin(ChunkKey::new(0, 0, 0)), CHUNK_SIDE as u32)
}

/// **The world the reading builds for itself**: the chunk's footprint **plus its halo** — the ring the
/// extent contract declares a chunk's faces are decided within.
fn halo_cube(roster: &Declared, ladder: &Ladder) -> CubicWorld {
    let origin = chunk_origin(ChunkKey::new(0, 0, 0));
    cube_over(
        roster,
        ladder,
        VoxelPos::new(origin.x - MESH_RING, origin.y - MESH_RING, origin.z),
        CHUNK_SIDE as u32 + 2 * MESH_RING as u32,
    )
}


#[test]
fn the_reference_world_derives_one_geometry_and_counts_its_own_work() {
    let roster = reference_roster();
    let registry = MaterialRegistry::starter();
    let ladder = reference_ladder(&registry);
    let mesher = Mesher::new(&roster, ladder.clone(), residency(256.0));

    // The four horizons are four declarations — what is drawn is inside what is simulated, which is
    // inside what is kept, which is inside what the fields describe — and the one derived rule
    // between them is checked.
    let render = RenderHorizon::m(128.0).expect("positive");
    let simulation = SimulationHorizon::m(192.0).expect("positive");
    let fields = FieldHorizon::m(1024.0).expect("positive");
    let horizon = residency(256.0);
    check_horizons(render, horizon).expect("the render horizon is inside the residency horizon");
    assert!(render.radius_m() < simulation.radius_m());
    assert!(simulation.radius_m() < horizon.radius_m());
    assert!(horizon.radius_m() < fields.radius_m());
    assert_eq!(
        check_horizons(RenderHorizon::m(300.0).unwrap(), horizon)
            .expect_err("wider than residency")
            .kind(),
        RefusalKind::HorizonMixed
    );

    let world = world_at(0);
    let chunk = ChunkKey::new(0, 0, 0);
    let level = Level::new(0).expect("level 0 is on the ladder");

    let mut resident = Resident::new();
    let mut ledger = WorkLedger::default();
    let mesh = mesher
        .derive(&world, chunk, level, Effort::plain(), &mut resident, &mut ledger)
        .expect("the reference world meshes");
    println!(
        "U4 seed={SEED} period=1s tick={} chunk=({}, {}, {}) level={}: {} cells swept, {} faces → {} \
         quads of {} faces of area, digest {}, merge factor {:.2}",
        world.tick().0,
        chunk.x,
        chunk.y,
        chunk.z,
        level.get(),
        ledger.cells,
        ledger.faces,
        mesh.quads().len(),
        mesh.area(),
        mesh.digest().to_hex(),
        ledger.merge_factor()
    );

    // --- SC-004: the ledger equals an independent recount of the same cells, and of the returned
    // geometry. The recount is taken of **the world the reading reads** — the chunk and its halo — with
    // the chunk's own footprint, which is what the door swept (the extent contract).
    let read = halo_cube(&roster, &ladder);
    let footprint = Footprint::of_chunk(&read, chunk).expect("the chunk's own footprint");
    assert!(
        footprint.halo_owned(&read),
        "the world the reading reads owns the halo it declares"
    );
    assert_eq!(footprint.columns(), chunk_columns());
    let recount = recount_faces(&read, footprint);
    assert_eq!(ledger.columns_swept, recount.columns);
    assert_eq!(ledger.cells, recount.solid_cells);
    assert_eq!(ledger.faces, recount.total_faces());
    assert_eq!(ledger.merged_away, ledger.faces - ledger.quads);
    assert_eq!(ledger.quads, mesh.quads().len() as u64);
    assert_eq!(mesh.area(), ledger.faces, "the rectangles tile every face exactly once");
    assert_eq!(ledger.vertices, mesh.vertices());
    assert_eq!(ledger.indices, mesh.indices());
    assert_eq!(
        ledger.columns_produced,
        ring_columns(),
        "the reading produces the footprint and its halo: 34x34"
    );
    assert_eq!(ledger.samples_read, chunk_columns(), "level 0 reads every column");
    assert_eq!(ledger.misses, 1);
    assert_eq!(ledger.refusals, 0);
    assert!(!mesh.quads().is_empty(), "the reference world has a surface");
    assert!(ledger.faces > ledger.quads, "the merge did something");
    // Every rectangle is inside the chunk's own columns, on one of the three axes, with a face that
    // looks one way or the other and a span a voxel face can have.
    for quad in mesh.quads() {
        assert!(quad.span_u >= 1 && quad.span_v >= 1, "{quad:?}");
        assert!(quad.facing == 1 || quad.facing == -1, "{quad:?}");
        assert!(Axis::ALL.contains(&quad.axis));
        match quad.axis {
            // The chunk's columns are 0..32 in x and y; z runs over the world's own span. A face's
            // first coordinate is always a horizontal axis (y for x faces, x for y and z faces), and
            // a z face's second one is horizontal too.
            Axis::Z => {
                assert!((0..32).contains(&quad.u), "{quad:?}");
                assert!((0..32).contains(&quad.v), "{quad:?}");
                assert!(quad.u + quad.span_u as i32 <= 32 && quad.v + quad.span_v as i32 <= 32, "{quad:?}");
            }
            Axis::Y | Axis::X => {
                assert!((0..32).contains(&quad.u), "{quad:?}");
                assert!(quad.u + quad.span_u as i32 <= 32, "{quad:?}");
            }
        }
        assert!(quad.plane.abs() < 10_000, "a voxel plane, not a field value: {quad:?}");
    }

    // --- SC-001: the worker count is a knob, on the reference world as on a fixture.
    let mut digests = vec![mesh.digest()];
    for workers in [2_usize, 4, 8] {
        let mut fresh = Resident::new();
        let mut fresh_ledger = WorkLedger::default();
        let again = mesher
            .derive(&world, chunk, level, Effort::new(Budget::unlimited(), workers), &mut fresh, &mut fresh_ledger)
            .expect("the same world meshes at any worker count");
        assert_eq!(again.quads(), mesh.quads(), "the worker count changes no quad");
        digests.push(again.digest());
    }
    assert!(
        digests.windows(2).all(|pair| pair[0] == pair[1]),
        "one world, one digest: {digests:?}"
    );

    // --- The level ladder: coarser is fewer samples over the same world, and different geometry.
    let mut coarse_resident = Resident::new();
    let mut coarse_ledger = WorkLedger::default();
    let coarse = mesher
        .derive(
            &world,
            chunk,
            Level::new(2).expect("level 2 is on the ladder"),
            Effort::new(Budget::unlimited(), 2),
            &mut coarse_resident,
            &mut coarse_ledger,
        )
        .expect("a coarse level of the reference world meshes");
    assert_eq!(
        coarse_ledger.columns_produced,
        ring_columns(),
        "the coarse reading reads the halo too"
    );
    assert_eq!(coarse_ledger.samples_read, 64, "8x8 tiles of 4 columns");
    assert_ne!(coarse.digest(), mesh.digest(), "a level is part of the identity");
    assert_eq!(coarse.area(), coarse_ledger.faces, "a coarse level tiles its own faces exactly");
    // A level past the ladder is refused by name rather than clamped to the coarsest.
    assert_eq!(
        Level::new(3).expect_err("past the ladder").kind(),
        RefusalKind::LevelOffLadder
    );
    // --- FR-009: a budget that cannot be met is refused by name with both numbers, and caches
    // nothing. The ceiling is a ceiling on **work**, so it is checked on a cold cache: the request
    // above is already resident, and a hit has no work to budget.
    let mut cold = Resident::new();
    let mut refused_ledger = WorkLedger::default();
    let refused = mesher
        .derive(&world, chunk, level, Effort::new(Budget::new(64, 64), 1), &mut cold, &mut refused_ledger)
        .expect_err("1156 columns cannot fit in 64");
    assert_eq!(refused.kind(), RefusalKind::GeometryBudgetExceeded);
    assert!(refused.detail().contains("1156"), "{refused}");
    assert!(refused.detail().contains("64"), "{refused}");
    assert_eq!(refused_ledger.columns_produced, 0, "refused before the fields were read");
    assert_eq!(refused_ledger.refusals, 1);
    assert!(cold.is_empty(), "a refused request caches nothing");

    // A quad ceiling is checked against the merge that really happened, and the refusal carries the
    // measured need.
    let needed = mesh.quads().len() as u64;
    let mut measured = WorkLedger::default();
    let refused = mesher
        .derive(&world, chunk, level, Effort::new(Budget::new(u64::MAX, 0), 1), &mut cold, &mut measured)
        .expect_err("no rectangle fits in zero");
    assert_eq!(refused.kind(), RefusalKind::GeometryBudgetExceeded);
    assert!(refused.detail().contains(&needed.to_string()), "{refused}");
    assert_eq!(measured.columns_produced, ring_columns(), "the work that led here is counted");
    assert_eq!(measured.quads, needed);
    assert_eq!(measured.faces, ledger.faces);
    assert!(cold.is_empty(), "a refused request caches nothing");

    // And a ceiling that the request fits is not refused: the budget is a ceiling, not a target.
    let fitting = Budget::new(ring_columns(), needed);
    assert!(
        fitting.allows(ring_columns(), needed),
        "the ceiling admits exactly what the request needs"
    );
    let mut roomy = Resident::new();
    let mut roomy_ledger = WorkLedger::default();
    mesher
        .derive(
            &world,
            chunk,
            level,
            Effort::new(fitting, 1),
            &mut roomy,
            &mut roomy_ledger,
        )
        .expect("a budget that fits is met, not exceeded");
    assert_eq!(roomy.len(), 1);

    // --- FR-010: a chunk outside the residency horizon is refused by name.
    let narrow = Mesher::new(&roster, ladder, residency(48.0));
    let mut narrow_resident = Resident::new();
    let mut narrow_ledger = WorkLedger::default();
    let far = narrow
        .derive(
            &world,
            ChunkKey::new(2, 0, 0),
            level,
            Effort::new(Budget::unlimited(), 1),
            &mut narrow_resident,
            &mut narrow_ledger,
        )
        .expect_err("two chunks away is 64 m, past a 48 m horizon");
    assert_eq!(far.kind(), RefusalKind::GeometryOutOfResidency);
    assert!(far.detail().contains("64.0"), "{far}");
    assert!(narrow_resident.is_empty());
}
#[test]
fn the_reference_world_s_geometry_stays_coherent_with_the_world_it_came_from() {
    let roster = reference_roster();
    let registry = MaterialRegistry::starter();
    let ladder = reference_ladder(&registry);
    let mesher = Mesher::new(&roster, ladder, residency(256.0));
    let level = Level::new(0).expect("level 0");
    let chunks = [
        ChunkKey::new(0, 0, 0),
        ChunkKey::new(1, 0, 0),
        ChunkKey::new(0, 1, 0),
    ];

    let world = world_at(0);
    let mut resident = Resident::new();
    let mut ledger = WorkLedger::default();
    let first = mesher
        .derive_many(&world, &chunks, level, Effort::new(Budget::unlimited(), 4), &mut resident, &mut ledger)
        .expect("three chunks of the reference world");
    assert_eq!(resident.len(), 3);
    assert_eq!(ledger.misses, 3);
    assert_eq!(ledger.hits, 0);
    // The declared order of the request is the order of the answer.
    for (mesh, chunk) in first.iter().zip(&chunks) {
        assert_eq!(mesh.key().chunk, *chunk);
        assert_eq!(mesh.key().level, level);
    }

    // --- SC-002: a request in an unchanged world is a hit with **zero** columns produced.
    let before = ledger;
    let warm = mesher
        .derive_many(&world, &chunks, level, Effort::new(Budget::unlimited(), 4), &mut resident, &mut ledger)
        .expect("served from the resident set");
    assert_eq!(warm.iter().map(Mesh::digest).collect::<Vec<_>>(), first.iter().map(Mesh::digest).collect::<Vec<_>>());
    assert_eq!(ledger.hits, before.hits + 3);
    assert_eq!(ledger.misses, before.misses);
    assert_eq!(ledger.columns_produced, before.columns_produced, "a hit reads no columns");
    assert_eq!(ledger.samples_read, before.samples_read);
    assert_eq!(ledger.faces, before.faces, "a hit sweeps no cells");
    assert_eq!(resident.len(), 3);

    // --- FR-007: a view is not truth. Moving the anchor and filling U2's cache changes no key, so
    // the geometry is still a hit and its digest is unchanged.
    let mut viewed = world.clone();
    viewed.set_anchor(ChunkKey::new(-9, 4, 2));
    for x in -2..4 {
        viewed.reside(ChunkKey::new(x, 0, 0));
    }
    assert_eq!(viewed.truth_digest(), world.truth_digest());
    assert_eq!(GeometryKey::new(&viewed, chunks[0], level), first[0].key());
    let still = mesher
        .derive(&viewed, chunks[0], level, Effort::new(Budget::unlimited(), 1), &mut resident, &mut ledger)
        .expect("a resident chunk is served whatever the anchor says");
    assert_eq!(still.digest(), first[0].digest());
    assert_eq!(ledger.hits, before.hits + 4);
    assert_eq!(
        resident.invalidate(viewed.truth_digest()),
        0,
        "a moved anchor invalidates nothing"
    );
    // The horizon still governs **new** work: a chunk that is not resident and is now outside the
    // anchor's horizon is refused by name rather than derived and dropped.
    let far_work = mesher
        .derive(
            &viewed,
            ChunkKey::new(3, 0, 0),
            level,
            Effort::new(Budget::unlimited(), 1),
            &mut resident,
            &mut ledger,
        )
        .expect_err("409 m from the moved anchor, past a 256 m horizon");
    assert_eq!(far_work.kind(), RefusalKind::GeometryOutOfResidency);
    assert_eq!(resident.len(), 3, "the refusal cached nothing");

    // --- A world that has moved serves nothing stale: the same requests are misses and rebuilds.
    let later = world_at(1);
    let mut later_ledger = WorkLedger::default();
    let rebuilt = mesher
        .derive_many(&later, &chunks, level, Effort::new(Budget::unlimited(), 1), &mut resident, &mut later_ledger)
        .expect("three chunks of the new world");
    assert_eq!(later_ledger.hits, 0, "the resident entries are from another world");
    assert_eq!(later_ledger.misses, 3);
    for (old, new) in first.iter().zip(&rebuilt) {
        assert_ne!(old.digest(), new.digest(), "a new tick is a new world");
    }

    // --- FR-006: invalidation removes **exactly** the entries derived from the old state, and keeps
    // every entry derived from the current one.
    assert_eq!(resident.len(), 6);
    assert_eq!(resident.stale(later.truth_digest()), 3);
    assert_eq!(resident.invalidate(later.truth_digest()), 3);
    assert_eq!(resident.len(), 3);
    assert_eq!(resident.stale(later.truth_digest()), 0);
    assert_eq!(resident.invalidate(later.truth_digest()), 0, "nothing else is stale");
    for (mesh, chunk) in rebuilt.iter().zip(&chunks) {
        let served = resident
            .serve(later.truth_digest(), *chunk, level)
            .expect("the current world's geometry is still resident");
        assert_eq!(served.digest(), mesh.digest());
    }
    assert!(
        resident.serve(world.truth_digest(), chunks[0], level).is_none(),
        "the old world's geometry is gone and cannot be served"
    );
}
#[test]
fn the_laws_own_world_meshes_through_the_cells_door_and_re_keys_when_its_water_moves() {
    let roster = reference_roster();
    let registry = MaterialRegistry::starter();
    let ladder = reference_ladder(&registry);
    let mesher = Mesher::new(&roster, ladder.clone(), residency(256.0));
    let level = Level::new(0).expect("level 0");
    let chunk = ChunkKey::new(0, 0, 0);

    // The law's own world: the declared patch's fields, filled by the one constructor — the same world
    // `evidence.rs` hands the law.
    let request = process::declared_request();
    let patch = request.patch().expect("the declared request is a region");
    let values = fields_at(
        &roster,
        identity(),
        Tick(0),
        &[FieldId::Elevation, FieldId::WaterLevel, FieldId::Lithology],
        request,
        1,
    )
    .expect("the declared fields answer");
    let cube = CubicWorld::from_surface(patch, &values[0], &values[1], &values[2], &ladder)
        .expect("the declared surface fills a cubic world");
    let mut law = Automaton::over(&cube);

    let mut resident = Resident::new();
    let mut before_ledger = WorkLedger::default();
    let before = mesher
        .derive_cubic(&cube, chunk, level, Effort::plain(), &mut resident, &mut before_ledger)
        .expect("the law's world meshes through the cells' door");
    let state_before = law.digest();
    assert_eq!(before.key().state, state_before, "the key is the cells' own identity");
    assert_eq!(before_ledger.columns_produced, 0, "the cells' door reads no field plane");
    let footprint = Footprint::of_chunk(&cube, chunk).expect("the chunk's own footprint");
    let recount = recount_faces(&cube, footprint);
    assert_eq!(before_ledger.columns_swept, recount.columns);
    assert_eq!(before_ledger.cells, recount.solid_cells);
    assert_eq!(before_ledger.faces, recount.total_faces());
    assert_eq!(before_ledger.merged_away, before_ledger.faces - before_ledger.quads);
    assert_eq!(before.area(), before_ledger.faces);

    // The render ladder does not coarsen a cell world: a level above zero is refused by name.
    let refused = mesher
        .derive_cubic(&cube, chunk, Level::new(1).expect("level 1"), Effort::plain(), &mut resident, &mut before_ledger)
        .expect_err("the ladder coarsens a reading, not a cell world");
    assert_eq!(refused.kind(), RefusalKind::ParameterOutOfRange);
    assert!(refused.detail().contains("level 0 meshes it"), "{refused}");

    // One tick of the law: rain wets the ground, the water moves, and the state — the key — moves with it.
    let tick = law.step().expect("the law's own pour is not refused");
    assert_eq!(tick, Tick(1));
    let state_after = law.digest();
    assert_ne!(state_after, state_before, "the rain moved the water");
    assert!(
        resident.serve(state_after, chunk, level).is_none(),
        "the older state's geometry is never served for the new one"
    );
    assert_eq!(resident.stale(state_after), 1);
    let mut after_ledger = WorkLedger::default();
    let after = mesher
        .derive_cubic(law.world(), chunk, level, Effort::plain(), &mut resident, &mut after_ledger)
        .expect("the stepped cells mesh");
    assert_eq!(after.key().state, state_after);
    assert_ne!(after.digest(), before.digest());

    // The ground did not move, so its faces are the same set; the water is the law's own state, so its
    // faces are counted where they are.
    let solid = |mesh: &Mesh| -> Vec<_> {
        mesh.quads()
            .iter()
            .copied()
            .filter(|quad| quad.layer == Layer::Solid)
            .collect()
    };
    assert_eq!(solid(&before), solid(&after), "one tick of rain moves no ground");
    let stepped = recount_faces(law.world(), footprint);
    assert_eq!(after_ledger.columns_swept, stepped.columns, "the stepped world is what was swept");
    assert_eq!(after_ledger.cells, stepped.solid_cells);
    assert_eq!(after_ledger.faces, stepped.total_faces());
    assert_eq!(after.area(), 
        after_ledger.faces,
        "the recount of the stepped world is what the ledger counted"
    );
    assert!(stepped.total_faces() > 0, "{stepped:?}");

    // And the resident set is re-keyed: the older state's entry is unreachable and removed, the new one
    // is what is kept.
    assert_eq!(resident.invalidate(state_after), 1);
    assert_eq!(resident.len(), 1);
    assert!(resident.serve(state_after, chunk, level).is_some());
    assert_eq!(resident.stale(state_after), 0);
}
/// A world of `side × side` columns at `origin`, built by the one constructor from a column rule of the
/// test's own: worlds of different widths hold **exactly the same cells** where their columns overlap, so
/// the extent questions can be asked of one world made wider rather than of two different worlds. The
/// reference world answers them too, and does so in `evidence.rs`; a guard has to be able to say *why* a
/// span ends where it ends.
fn slab(
    origin: VoxelPos,
    side: u32,
    ground: impl Fn(i32, i32) -> f64,
    water: f64,
    ladder: &Ladder,
) -> CubicWorld {
    let cells = (side * side) as usize;
    let mut elevation = Vec::with_capacity(cells);
    let mut classes = Vec::with_capacity(cells);
    for cell in 0..cells {
        let x = origin.x + (cell % side as usize) as i32;
        let y = origin.y + (cell / side as usize) as i32;
        elevation.push(ground(x, y));
        classes.push(0.0);
    }
    let water_level = vec![water; cells];
    CubicWorld::from_surface(
        Patch::new(origin, side).expect("a non-empty patch"),
        &elevation,
        &water_level,
        &classes,
        ladder,
    )
    .expect("the fixture surface fills a cubic world")
}

/// A `chunks × chunks` fixture at the origin: `ground(x)` decides each column's elevation.
fn fixture(chunks: u32, ground: impl Fn(i32) -> f64, water: f64, ladder: &Ladder) -> CubicWorld {
    slab(
        VoxelPos::new(0, 0, 0),
        CHUNK_SIDE as u32 * chunks,
        move |x, _y| ground(x),
        water,
        ladder,
    )
}

/// **The extent contract, through the public API.** Four things the audit found unprobed, all of them
/// questions about *what is meshed* rather than about the faces a mesh happens to have:
///
/// * a footprint over a rectangle of chunks is one mesh whose shared boundaries carry no face where the two
///   sides agree — and whose faces are the chunks' own faces summed when they do not;
/// * a chunk's faces do not depend on the world beyond its ring: a world with one ring and a world with two
///   draw the same rectangles, and a world that is only the chunk draws its own edge as a cliff;
/// * the reading and the cells' door, over **one world**, draw the same rectangles;
/// * the span reaches water's own surface: a column standing 31 m of water has its top face inside the
///   footprint's span, with the ceiling a band strictly above it.
#[test]
fn one_footprint_stitches_chunks_and_its_faces_ignore_the_world_beyond_the_ring() {
    let roster = reference_roster();
    let registry = MaterialRegistry::starter();
    let ladder = reference_ladder(&registry);
    let mesher = Mesher::new(&roster, ladder.clone(), residency(256.0));
    let level = Level::new(0).expect("level 0");
    let chunk = ChunkKey::new(0, 0, 0);
    let square = [
        ChunkKey::new(0, 0, 0),
        ChunkKey::new(1, 0, 0),
        ChunkKey::new(0, 1, 0),
        ChunkKey::new(1, 1, 0),
    ];
    let footprint_of = |cube: &CubicWorld, at: ChunkKey| {
        Footprint::of_chunk(cube, at).expect("the chunk's own footprint")
    };
    let mesh_of = |cube: &CubicWorld, at: ChunkKey| -> Mesh {
        let mut ledger = WorkLedger::default();
        mesher
            .derive_cubic(cube, at, level, Effort::plain(), &mut Resident::new(), &mut ledger)
            .expect("the chunk meshes through the cells' door")
    };
    let boundary_quads = |mesh: &Mesh| -> usize {
        mesh.quads()
            .iter()
            .filter(|quad| quad.axis == Axis::X && quad.plane == CHUNK_SIDE)
            .count()
    };

    // --- A smooth boundary between two chunks in one footprint: no rectangle may stand on it. The
    // rectangle is one footprint and one sweep, not four meshes with a seam between them.
    let flat = fixture(2, |_x| 3.0, -10.0, &ladder);
    let mut flat_ledger = WorkLedger::default();
    let flat_region = mesher
        .derive_region(&flat, chunk, 2, level, Effort::plain(), &mut flat_ledger)
        .expect("a rectangle of chunks meshes as one mesh");
    assert_eq!(
        boundary_quads(&flat_region),
        0,
        "a boundary both sides agree at is the world's inside, not a face of it"
    );
    let rectangle = Footprint::over(
        &flat,
        Patch::new(VoxelPos::new(0, 0, 0), CHUNK_SIDE as u32 * 2).expect("a non-empty patch"),
    )
    .expect("the rectangle's own footprint");
    assert_eq!(rectangle.patch().side(), CHUNK_SIDE as u32 * 2, "one footprint, two chunks wide");
    assert_eq!(rectangle.columns(), 4 * chunk_columns(), "a two-by-two rectangle of chunks");
    assert_eq!(flat_region.key().chunk, chunk, "a region is named by the chunk it starts at");
    assert_eq!(
        flat_ledger.columns_swept,
        4 * chunk_columns(),
        "every chunk's columns were swept once, in one pass"
    );
    assert_eq!(flat_region.area(), flat_ledger.faces);

    // --- A step at that boundary: the cliff between the two chunks is a face of the world, and it is swept
    // **once** — the region's faces are the chunks' own faces summed.
    let stepped = fixture(2, |x| if x < CHUNK_SIDE { 3.0 } else { 9.0 }, -10.0, &ladder);
    let mut stepped_ledger = WorkLedger::default();
    let stepped_region = mesher
        .derive_region(&stepped, chunk, 2, level, Effort::plain(), &mut stepped_ledger)
        .expect("a rectangle of chunks meshes as one mesh");
    assert!(
        boundary_quads(&stepped_region) > 0,
        "a step at the boundary is visible: it is a face of the world"
    );
    let band = footprint_of(&stepped, chunk).span();
    let mut alone = 0;
    for at in square {
        assert_eq!(footprint_of(&stepped, at).span(), band, "the fixture's chunks stand in one band");
        alone += mesh_of(&stepped, at).area();
    }
    assert_eq!(
        stepped_region.area(),
        alone,
        "the shared boundaries are meshed once: neither twice nor never"
    );
    assert_eq!(stepped_region.vertices(), stepped_region.quads().len() as u64 * 4);

    // --- The world beyond the ring changes nothing; a world with no ring draws the chunk's own edge.
    //
    // The three worlds hold **exactly the same cells** where their columns overlap, because they are one
    // column rule written over three widths — the extent experiment is *one world made wider*, not a wider
    // field request (the declared roster carries region-dependent stages, so a wider request is a
    // different world, which is why the reading's own door reads only the chunk and its halo).
    let ramp = |x: i32, _y: i32| if x < 0 { 40.0 } else { 3.0 + ((x % 11) as f64) };
    let no_ring = slab(VoxelPos::new(0, 0, 0), CHUNK_SIDE as u32, ramp, 6.0, &ladder);
    let one_ring = slab(
        VoxelPos::new(-MESH_RING, -MESH_RING, 0),
        CHUNK_SIDE as u32 + 2 * MESH_RING as u32,
        ramp,
        6.0,
        &ladder,
    );
    let two_rings = slab(VoxelPos::new(-2, -2, 0), CHUNK_SIDE as u32 + 4, ramp, 6.0, &ladder);
    let no_ring_mesh = mesh_of(&no_ring, chunk);
    let one_ring_mesh = mesh_of(&one_ring, chunk);
    let two_rings_mesh = mesh_of(&two_rings, chunk);
    assert_eq!(
        one_ring_mesh.quads(),
        two_rings_mesh.quads(),
        "one ring is the whole of what a footprint's faces are decided by"
    );
    assert_ne!(
        one_ring_mesh.key().state,
        two_rings_mesh.key().state,
        "widening a world moves its own name: the same rectangles are keyed by two states"
    );
    assert_ne!(
        no_ring_mesh.quads(),
        one_ring_mesh.quads(),
        "a world that is only the chunk owns no halo and shows its own edge"
    );
    assert!(
        no_ring_mesh.area() > one_ring_mesh.area(),
        "the edge a halo hides is faces: {} against {}",
        no_ring_mesh.area(),
        one_ring_mesh.area()
    );
    assert!(!footprint_of(&no_ring, chunk).halo_owned(&no_ring));
    assert!(footprint_of(&one_ring, chunk).halo_owned(&one_ring));
    assert!(footprint_of(&two_rings, chunk).halo_owned(&two_rings));

    // --- One world, two doors: the reading reads the ring, so it draws what the cells' door draws of the
    // world it built for itself.
    let read_world = halo_cube(&roster, &ladder);
    let read_cells = mesh_of(&read_world, chunk);
    let mut reading_ledger = WorkLedger::default();
    let reading = mesher
        .derive(&world_at(0), chunk, level, Effort::plain(), &mut Resident::new(), &mut reading_ledger)
        .expect("the reading of the reference world meshes");
    assert_eq!(
        reading.quads(),
        read_cells.quads(),
        "the reading reads the halo: the same world through both doors is the same list"
    );
    assert_eq!(reading.area(), read_cells.area());
    assert_eq!(reading_ledger.columns_produced, ring_columns(), "the reading reads the ring");
    assert_eq!(reading_ledger.samples_read, chunk_columns(), "and decimates nothing at level 0");
    // The same declared chunk in a world that is **only** the chunk: the reading's old world, and the
    // audit's number. Its faces are the chunk's own edge drawn as a cliff, which is what the halo hides.
    let declared_chunk_only = mesh_of(&chunk_cube(&roster, &ladder), chunk);
    assert!(
        declared_chunk_only.area() > read_cells.area(),
        "the declared chunk with no halo draws its own edge: {} against {}",
        declared_chunk_only.area(),
        read_cells.area()
    );

    // --- Span-edge water: the footprint's span covers the water's own surface, and its ceiling is a band
    // strictly above the highest cell the columns hold.
    let wet = fixture(1, |_x| 3.0, 34.0, &ladder);
    let wet_footprint = footprint_of(&wet, chunk);
    let wet_mesh = mesh_of(&wet, chunk);
    let water_quads: Vec<_> = wet_mesh
        .quads()
        .iter()
        .filter(|quad| quad.layer == Layer::Water)
        .collect();
    assert!(!water_quads.is_empty(), "water's own top face is emitted");
    let water_top = water_quads.iter().map(|quad| quad.plane).max().expect("a water face");
    assert_eq!(water_top, 35, "the water stands to 34 m, so its top face is the plane above it");
    assert_eq!(wet_footprint.floor_z(), 0, "the ground at 3 m is in the band at 0");
    assert_eq!(wet_footprint.ceiling_z(), 64, "a band strictly above the highest cell at 34 m");
    assert!(
        wet_footprint.ceiling_z() > water_top,
        "the span reaches the water's surface and past it: {} > {water_top}",
        wet_footprint.ceiling_z()
    );
    assert_eq!(wet_footprint.cells(), chunk_columns() * 64);
}
