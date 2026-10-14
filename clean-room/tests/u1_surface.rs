//! **U1's visible client, measured (2026-10-13, §36).**
//!
//! Derived from `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §5 row 1's check — *a **visible** client (never
//! `nodraw`)*, whose own check is *the band count and a screenshot* (`LEDGER.md` C-40) — and from
//! `clean-room/specs/001-presentation/tasks.md` T009. Every claim below is measured on the frame's own
//! pixels or on the window's own device; the one test that needs a window says so in its name, opens it for
//! two frames and closes it, and refuses rather than passing silently on a machine that cannot show one.

use shploingus_clean_room::automaton::{Automaton, SETTLE_TICKS};
use shploingus_clean_room::being::{self, WorldRead};
use shploingus_clean_room::chunk::CubicWorld;
use shploingus_clean_room::field::fields_at;
use shploingus_clean_room::material::{Density, Material, MaterialId, MaterialRegistry};
use shploingus_clean_room::mesh::Ladder;
use shploingus_clean_room::presentation::{self, BAR_MINIMUM_BANDS};
use shploingus_clean_room::process;
use shploingus_clean_room::reference;
use shploingus_clean_room::refusal::RefusalKind;
use shploingus_clean_room::stage::{FieldId, ProcessRequest};
use shploingus_clean_room::surface::{self, Surface};
use shploingus_clean_room::world::Tick;

/// The one world every claim here reads: the declared fields filled into their **cubic world** at
/// `Tick(0)` (U12's world since §38), read through the one reading every consumer shares — the same read
/// the binary's evidence and its window use.
///
/// The cells come back **beside** the reading because they are the one owner of what a column is made of
/// (2026-10-14f): the frames below are drawn from the read and take every material from these cells, so a
/// claim about a column's material is a claim about a cell and not about a class plane.
fn world_and_cells() -> (CubicWorld, WorldRead) {
    let roster = process::roster().expect("the declared roster");
    let identity = reference::identity().expect("the reference identity");
    let request = process::declared_request();
    assert!(matches!(request, ProcessRequest::Region { .. }), "the declared request is a region");
    let patch = request.patch().expect("the declared request is a patch");
    let ladder = ladder();
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
    let world = WorldRead::over_cubic(being::Organisation::first(), "the first mob", &cubic)
        .expect("the world's own cells are readable");
    (cubic, world)
}

/// **The world's own reading of the law's state** — `over_cubic` of the world the law is stepping, the one
/// reading every consumer shares (§38).
fn read_of(plane: &Automaton) -> WorldRead {
    WorldRead::over_cubic(being::Organisation::first(), "the first mob", plane.world())
        .expect("the stepped world's own cells are readable")
}

/// One tick of the law, with the refusal its own pour can make a panic here rather than a silent default:
/// the declared world's span holds the rain, and a test that did not would be a defect worth seeing.
fn step(plane: &mut Automaton, count: usize) {
    plane
        .steps(count)
        .expect("the declared world's span holds the law's own water");
}

fn ladder() -> Ladder {
    reference::ladder().expect("the reference world's own ladder")
}

#[test]
fn the_frame_is_a_pure_function_of_the_world_and_its_declared_view() {
    let (cubic, world) = world_and_cells();
    let ladder = ladder();
    let registry = reference::registry();
    let light = surface::light();
    let view = surface::view();
    let a = Surface::render(&world, &cubic, ladder.water(), registry, &light, &view, surface::ZOOM)
        .expect("the frame is drawn");
    let b = Surface::render(&world, &cubic, ladder.water(), registry, &light, &view, surface::ZOOM)
        .expect("the frame is drawn again");
    assert_eq!(a, b, "the same world and the same declared view are one frame");
    assert_eq!(a.digest().to_hex(), b.digest().to_hex());

    // And the frame **follows the world's own state**: the law moved the cells, so the picture moves.
    let mut plane = Automaton::over(&cubic);
    step(&mut plane, SETTLE_TICKS);
    let stepped = Surface::render(
        &read_of(&plane),
        plane.world(),
        ladder.water(),
        registry,
        &light,
        &view,
        surface::ZOOM,
    )
    .expect("the stepped world draws");
    assert_ne!(
        stepped.digest().to_hex(),
        a.digest().to_hex(),
        "the settled plane is not the plane the being was placed in"
    );
    assert_eq!(
        a.width(),
        world.patch.side() * surface::ZOOM,
        "the frame's width is the patch's own side times the declared zoom"
    );
    assert_eq!(a.height(), a.width(), "the patch is square and so is its frame");
    assert_eq!(a.pixels().len(), (a.width() * a.height() * 4) as usize);
}

#[test]
fn the_frame_shows_at_least_three_luminance_bands_and_is_not_a_flat_fill() {
    let (cubic, _world) = world_and_cells();
    let mut plane = Automaton::over(&cubic);
    step(&mut plane, SETTLE_TICKS);
    let frame = Surface::render(
        &read_of(&plane),
        plane.world(),
        ladder().water(),
        reference::registry(),
        &surface::light(),
        &surface::view(),
        surface::ZOOM,
    )
    .expect("the frame is drawn");
    let bands = frame.bands();
    assert_eq!(
        bands,
        presentation::band_count(frame.samples()),
        "the band count is the contract's own instrument, read on this frame's samples"
    );
    assert!(
        bands >= BAR_MINIMUM_BANDS,
        "the FMA bar requires >={BAR_MINIMUM_BANDS} bands, measured {bands}"
    );
    let (dark, bright) = frame.range();
    assert!(dark < bright, "a drawn frame is not a flat fill: {dark:?}..{bright:?}");
}

#[test]
fn the_material_of_a_column_is_the_cell_s_own_and_water_is_the_palette_s() {
    // The name moved with the code (2026-10-14f): this test was `the_material_of_a_column_is_the_ladder_s_own_
    // mapping_and_water_is_its_own` until the appearance path stopped reading the class ladder. The ladder
    // still draws the one thing a cell cannot answer for — water — and the columns above still name the
    // classes they were once shaded from. `docs/baselines/u1-visible-client/proof.txt` records the old name
    // at the line numbers it was bitten at, and is not rewritten: it is a capture of a past pass.
    let (cubic, world) = world_and_cells();
    let ladder = ladder();
    let registry = reference::registry();
    let light = surface::light();
    let view = surface::view();
    let frame = Surface::render(&world, &cubic, ladder.water(), registry, &light, &view, surface::ZOOM)
        .expect("the frame is drawn");
    // One dry column and one wet column of the world's own choosing, not of this test's.
    let dry = (0..world.cells())
        .find(|cell| !world.water_stands(*cell))
        .expect("the patch has dry columns");
    let wet = (0..world.cells())
        .find(|cell| world.water_stands(*cell))
        .expect("the patch has wet columns");
    let dry_at = world.patch.position(dry).expect("a column of the patch");
    // **The column's material is the cell's own** — read out of the world's cells, and deliberately *not*
    // out of a class: the ladder is not in this path at all (2026-10-14f).
    let dry_material = registry
        .material(cubic.column(dry_at).expect("the column reads").material.expect("a surface cell"))
        .expect("an admitted material");
    let water = registry.material(ladder.water()).expect("the ladder's water is admitted");
    // The column's own shaded sample is the contract's own `shade` over that material's sourced colour:
    // recomputed here from the material record rather than compared with a constant.
    let expected = |material: &Material, normal: [f64; 3]| {
        presentation::shade(
            &presentation::surface_of(material, surface::ROUGHNESS, surface::METALNESS)
                .expect("the starter set is sourced"),
            normal,
            &light,
            &view,
        )
        .expect("a unit normal and an admitted reflectance")
    };
    assert_eq!(
        frame.samples()[dry],
        expected(dry_material, surface::normal_of(&world, dry, false)),
        "a dry column is shaded from its own cell's material and its own terrain"
    );
    assert_eq!(
        frame.samples()[wet],
        expected(water, [0.0, 0.0, 1.0]),
        "a wet column is water's own sourced colour, drawn as a level surface"
    );
    // **And the frame follows a cell**: the same world with one column's surface cell re-materialled draws
    // the second material's colour at that column and nowhere else. This is the claim the pass exists for —
    // *the cell is the single owner* — measured on the frame's own bytes rather than on the code's shape.
    let other = ladder.mapping().iter().map(|(_, id)| *id).find(|id| *id != dry_material.id);
    let other = other.expect("the ladder admits more than one material");
    let mut changed = cubic.clone();
    let ground_z = cubic.column(dry_at).expect("the column reads").surface_z;
    changed
        .place_material(
            shploingus_clean_room::coordinates::VoxelPos::new(dry_at.x, dry_at.y, ground_z),
            Some(other),
        )
        .expect("the cell is the world's own");
    let after = Surface::render(
        &WorldRead::over_cubic(being::Organisation::first(), "the first mob", &changed)
            .expect("the changed world is readable"),
        &changed,
        ladder.water(),
        registry,
        &light,
        &view,
        surface::ZOOM,
    )
    .expect("the changed world draws");
    println!(
        "U1 cell-owned material: one cell re-materialled {} -> {}: frame {} -> {}, sample at column \
         {dry} {:?} -> {:?}",
        dry_material.name,
        registry.material(other).expect("admitted").name,
        frame.digest().to_hex(),
        after.digest().to_hex(),
        frame.samples()[dry],
        after.samples()[dry]
    );
    assert_ne!(
        frame.digest().to_hex(),
        after.digest().to_hex(),
        "one cell's material is the frame's own: the picture follows the cell"
    );
    assert_ne!(
        frame.samples()[dry],
        after.samples()[dry],
        "the re-materialled column is drawn differently"
    );
    for cell in 0..world.cells() {
        if cell == dry {
            continue;
        }
        assert_eq!(
            frame.samples()[cell],
            after.samples()[cell],
            "and no other column moved: column {cell}"
        );
    }
    // The relief is visible **inside one material**: two dry columns holding the same material, on slopes
    // that differ, are drawn differently — which is the FMA bar's own point, and which a picture drawn from
    // material alone could not show.
    let mut shaded_by_terrain = false;
    for cell in 0..world.cells() {
        if world.water_stands(cell) {
            continue;
        }
        let Some(at) = world.patch.position(cell) else {
            continue;
        };
        let held = cubic.column(at).expect("the column reads").material;
        let Some(neighbour) = (0..world.cells()).find(|other| {
            let Some(neighbour_at) = world.patch.position(*other) else {
                return false;
            };
            !world.water_stands(*other)
                && cubic.column(neighbour_at).expect("the column reads").material == held
                && other != &cell
        }) else {
            continue;
        };
        if surface::normal_of(&world, cell, false) != surface::normal_of(&world, neighbour, false)
            && frame.samples()[cell] != frame.samples()[neighbour]
        {
            shaded_by_terrain = true;
            break;
        }
    }
    assert!(
        shaded_by_terrain,
        "the world's own relief reaches the frame: two columns of one material, on different slopes, are \
         drawn differently"
    );
}

#[test]
fn a_material_that_derives_no_colour_refuses_the_frame_by_name() {
    let (cubic, world) = world_and_cells();
    let mut registry = MaterialRegistry::new();
    registry
        .admit(Material {
            id: MaterialId(1),
            name: "unrecorded",
            density: Density::per_cm3(3, 2).expect("a positive density"),
            reflectance: None,
        })
        .expect("one admitted material");
    let ladder = Ladder::over_palette(&registry, &[MaterialId(1)], 6, MaterialId(1))
        .expect("the ladder's one class and its water are admitted");
    let refusal = Surface::render(
        &world,
        &cubic,
        ladder.water(),
        &registry,
        &surface::light(),
        &surface::view(),
        surface::ZOOM,
    )
    .expect_err("a material with no reflectance derives no colour");
    assert_eq!(refusal.kind(), RefusalKind::NoReflectance);
    assert_eq!(refusal.kind().name(), "no_reflectance");
    assert!(
        refusal.detail().contains("unrecorded"),
        "the material is named: {refusal}"
    );
}

#[test]
fn a_zoom_of_none_and_cells_that_are_not_the_read_s_own_are_refused_by_name() {
    let (cubic, world) = world_and_cells();
    let ladder = ladder();
    let registry = reference::registry();
    let light = surface::light();
    let view = surface::view();
    let refusal = Surface::render(&world, &cubic, ladder.water(), registry, &light, &view, 0)
        .expect_err("a zoom of no pixels draws nothing");
    assert_eq!(refusal.kind(), RefusalKind::ParameterOutOfRange);
    assert!(refusal.detail().contains("zoom"), "{refusal}");

    // **The read and the cells are one world or the frame is refused.** The guard this pass kept is not a
    // count of an array: the cells a frame takes its materials from must be the cells the read was taken
    // from, so a world of another patch is refused by name rather than shaded over this read.
    let shifted = CubicWorld::from_fields(
        &process::roster().expect("the declared roster"),
        reference::identity().expect("the reference identity"),
        Tick(0),
        ProcessRequest::Region {
            origin: shploingus_clean_room::coordinates::VoxelPos::new(
                cubic.patch().origin().x + 1,
                cubic.patch().origin().y,
                cubic.patch().origin().z,
            ),
            side: cubic.patch().side(),
        },
        &ladder,
        1,
    )
    .expect("the world one column over is a world");
    let refusal = Surface::render(
        &world,
        &shifted,
        ladder.water(),
        registry,
        &light,
        &view,
        surface::ZOOM,
    )
    .expect_err("a frame cannot take its materials from another patch's cells");
    assert_eq!(refusal.kind(), RefusalKind::ParameterOutOfRange);
    assert!(refusal.detail().contains("column"), "{refusal}");
}

#[test]
fn a_read_and_cells_that_stand_different_water_are_refused_by_name() {
    // **The read and the cells are held together about the water.** The read is a reading *of* the cells, so
    // a read taken before the law moved and cells taken after it are two different worlds: the frame is
    // refused by name rather than shaded — one world's surface over another world's ground. The subject is
    // the placed world's own read against the settled world's own cells.
    let (cubic, world) = world_and_cells();
    let ladder = ladder();
    let mut plane = Automaton::over(&cubic);
    step(&mut plane, SETTLE_TICKS);
    let refusal = Surface::render(
        &world,
        plane.world(),
        ladder.water(),
        reference::registry(),
        &surface::light(),
        &surface::view(),
        surface::ZOOM,
    )
    .expect_err("the placed read and the settled cells do not stand the same water");
    assert_eq!(refusal.kind(), RefusalKind::ParameterOutOfRange);
    assert!(refusal.detail().contains("water"), "{refusal}");
    println!("U1 read and cells: {refusal}");
}

#[test]
fn the_frames_own_bytes_are_the_screenshot_the_bar_asks_for() {
    let (cubic, world) = world_and_cells();
    let frame = Surface::render(
        &world,
        &cubic,
        ladder().water(),
        reference::registry(),
        &surface::light(),
        &surface::view(),
        surface::ZOOM,
    )
    .expect("the frame is drawn");
    let path = std::env::temp_dir().join(format!(
        "shploingus-clean-u1-frame-{}.ppm",
        std::process::id()
    ));
    frame.write_ppm(&path).expect("the frame is written");
    let written = std::fs::read(&path).expect("the written frame reads back");
    let header = format!("P6\n{} {}\n255\n", frame.width(), frame.height());
    assert!(
        written.starts_with(header.as_bytes()),
        "a P6 file's own header, at the frame's own size"
    );
    let body = &written[header.len()..];
    assert_eq!(body.len(), (frame.width() * frame.height() * 3) as usize);
    // The file's first pixel is the surface's first pixel, **in RGB order** — the screenshot is the bytes
    // the window was handed, and not a second drawing of the world.
    let pixel = frame.pixel(0, 0).expect("the frame has a first pixel");
    assert_eq!(&body[0..3], &[pixel[2], pixel[1], pixel[0]]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn the_declared_frame_count_is_the_horizon_the_run_settles_at() {
    // *A frame is a tick*: the client's declared count is the plane's own settle horizon, so the window's
    // last frame and the evidence run's own frame are the same world — pinned here without a window.
    assert_eq!(
        surface::DECLARED_FRAMES,
        SETTLE_TICKS as u32,
        "the client's declared count is the plane's own rest horizon"
    );
    let (cubic, world) = world_and_cells();
    let ladder = ladder();
    let registry = reference::registry();
    let light = surface::light();
    let view = surface::view();
    let mut walked = Automaton::over(&cubic);
    step(&mut walked, surface::DECLARED_FRAMES as usize);
    let mut settled = Automaton::over(&cubic);
    step(&mut settled, SETTLE_TICKS);
    let drawn = Surface::render(
        &read_of(&walked),
        walked.world(),
        ladder.water(),
        registry,
        &light,
        &view,
        surface::ZOOM,
    )
    .expect("the walked world draws");
    let at_rest = Surface::render(
        &read_of(&settled),
        settled.world(),
        ladder.water(),
        registry,
        &light,
        &view,
        surface::ZOOM,
    )
    .expect("the settled world draws");
    assert_eq!(
        drawn.digest().to_hex(),
        at_rest.digest().to_hex(),
        "the frame the window ends on is the frame the evidence draws"
    );
    assert_eq!(walked.ticks(), SETTLE_TICKS as u64);
    assert!(walked.total_um() > 0, "the plane holds water");
    assert!(
        walked.standing() * 2 > world.cells(),
        "and the world it comes to rest in is the marsh the run's own lines name"
    );
    // The law is what moved the picture: one tick of it is enough to change the frame.
    let mut one = Automaton::over(&cubic);
    step(&mut one, 1);
    assert_eq!(one.ticks(), 1);
    let after_one = Surface::render(
        &read_of(&one),
        one.world(),
        ladder.water(),
        registry,
        &light,
        &view,
        surface::ZOOM,
    )
    .expect("the world after one tick draws");
    let placed = Surface::render(&world, &cubic, ladder.water(), registry, &light, &view, surface::ZOOM)
        .expect("the world the being was placed in draws");
    assert_ne!(
        after_one.digest().to_hex(),
        placed.digest().to_hex(),
        "the picture follows the plane's own law, not the frame count"
    );
}

#[test]
fn the_presented_frame_reaches_the_window_s_own_pixels() {
    // The one test that needs a desktop: it opens a **real** window for two frames — a tick of the plane's
    // own law each — reads the window's own client area back through the declared format, and asserts every
    // one of its pixels is the surface's own byte. A machine with no window interface fails this by name
    // rather than passing invisibly (C-40).
    let report = match surface::present_world(2, None) {
        Ok(report) => report,
        Err(refusal) => panic!("the client is refused by name: {refusal}"),
    };
    assert_eq!(report.presented, 2, "two frames were presented");
    assert_eq!(
        report.drawing.counts.frames, 2,
        "every presented frame is a drawn frame, and the counts are the run's own"
    );
    assert_eq!(
        report.drawing.ticks, 2,
        "one tick of the plane's own law per frame"
    );
    let frame = &report.drawing.last;
    assert_eq!(
        report.pixels,
        (frame.size() * frame.size()) as usize * report.presented as usize,
        "every pixel of the client area, every frame"
    );
    assert_eq!(
        report.agreeing, report.pixels,
        "the window's own client area is the drawn frame's own bytes, at every pixel"
    );
    assert_eq!(
        frame.size(),
        48 * surface::ZOOM,
        "the drawn frame is the client's declared size, so one window shows either reading"
    );
    assert!(
        frame.quads() > 0 && frame.counts_agree(),
        "the frame draws the mesh's own rectangles: {} submitted",
        frame.quads()
    );
    assert!(frame.bands() >= BAR_MINIMUM_BANDS);
    assert!(
        report.drawing.bars_hold(),
        "the drawn reading's own bars: {}",
        report.drawing.bars_described()
    );
}
