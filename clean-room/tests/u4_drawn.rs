//! **U4's drawn reading, measured (2026-10-13, §41).**
//!
//! Derived from `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §41's own seam — *the mesh's own rectangles reach
//! pixels, under one camera with depth* — and from the U4 entry of §2 (the mesher's geometry, the resident
//! set, the work ledger). Every claim here is measured **without a window**, on the frame the drawn reading
//! produces from the **client's own world** ([`surface::client_world`], §42): what the drawing loop counted against what the mesh
//! declared, what the camera's own arithmetic puts in front of the frustum, and an **independent recount**
//! of the pixels and depths over a declared sample — the same discipline [`crate::mesh::recount_faces`] uses
//! for faces, because a second implementation of one rule is evidence and a second call of one implementation
//! is not.
//!
//! The window's own run (`cargo run --offline -- window`) draws through **this same call**
//! ([`surface::draw_plane`]) and prints its agreement with [`surface::drawn_frame`], so the bars below are the
//! bars the window's frames are held to.

use shploingus_clean_room::automaton::Automaton;
use shploingus_clean_room::coordinates::{VoxelPos, CHUNK_SIDE};
use shploingus_clean_room::drawn;
use shploingus_clean_room::material::MaterialId;
use shploingus_clean_room::presentation::{self, BAR_MINIMUM_BANDS};
use shploingus_clean_room::reference;
use shploingus_clean_room::surface;

/// **The world the drawn reading draws**: the client's own world, through the one declaration of it
/// ([`surface::client_world`]), so the guards here and the window's frames stand on the same world — stepped
/// `ticks` times by the plane's own law. Since §42 that world is a **whole number of chunks** (`2 × 2` of
/// them, `64 × 64` columns), because the drawn reading draws the **stitched region** and a region needs whole
/// chunks of world under it.
fn plane_after(ticks: usize) -> Automaton {
    let cubic = surface::client_world().expect("the client's own world is declared");
    let mut plane = Automaton::over(&cubic);
    plane.steps(ticks).expect("the law steps");
    plane
}

/// **The client's own residency of a state** — the camera's chunk of this world ([`surface::residency_of`]),
/// which is what the drawn path takes its cells from: the guards here hand the drawn door a *residency* and
/// not a plane, so a guard cannot draw cells no camera holds.
fn residency_of(plane: &Automaton) -> shploingus_clean_room::stream::Streamer {
    surface::residency_of(plane.world()).expect("the client's own world has a chunk to stand in")
}

/// The drawn frame of a state, through **the client's own residency of it** — the same door the window's loop
/// and the evidence run use, so a guard here is a guard of the drawn path rather than of a second one.
fn drawn_of(plane: &Automaton) -> Result<drawn::Drawn, shploingus_clean_room::refusal::Refusal> {
    // The declared view's own eye of the declared region — the frame this suite has always measured. Since
    // 2026-10-14e the drawn door also returns the eye it stood at, which the controller's own guards use.
    surface::draw_plane(&residency_of(plane), surface::DRAWN_CHUNK).map(|(frame, _)| frame)
}

/// The region's own mesh as the drawn path derives it — over the residency of this state, so the mesh a guard
/// measures is the mesh the frame was drawn from.
fn mesh_of(plane: &Automaton) -> Result<shploingus_clean_room::mesh::Mesh, shploingus_clean_room::refusal::Refusal> {
    surface::region_mesh(&residency_of(plane), surface::DRAWN_CHUNK)
}

/// The client's own camera over a mesh — the one call the drawn path makes, so the guards here and the
/// window's frames stand on one camera.
fn camera_over(mesh: &shploingus_clean_room::mesh::Mesh) -> drawn::Camera {
    drawn::Camera::over(mesh, surface::view().toward, drawn::DRAWN_FOV_DEG)
        .expect("the client's own region has an extent")
}

fn unit(v: [f64; 3]) -> [f64; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    [v[0] / len, v[1] / len, v[2] / len]
}

#[test]
fn the_drawn_frame_is_the_meshes_own_rectangles_to_pixels() {
    // The first bar: the frame's own counts are the mesh's own declaration, nothing is clipped, the picture
    // is a picture, and the mesh it came from is keyed by the law's own state.
    let drawing = surface::drawn_frame().expect("the drawn frame is admitted");
    let frame = &drawing.last;
    let (declared_quads, declared_vertices, declared_triangles, declared_indices) = frame.declared();
    println!(
        "U4 drawn: {} rectangle(s), {} corner(s), {} triangle(s), {} index(es); {:?}",
        frame.quads(),
        frame.vertices(),
        frame.triangles(),
        frame.indices(),
        frame.declared()
    );
    assert_eq!(frame.quads(), declared_quads, "the mesh declared {declared_quads} rectangle(s)");
    assert_eq!(frame.vertices(), declared_vertices);
    assert_eq!(frame.triangles(), declared_triangles);
    assert_eq!(frame.indices(), declared_indices);
    assert!(
        frame.counts_agree(),
        "the drawing loop's own counts are the mesh's own: {:?} against {:?}",
        frame.declared(),
        (frame.quads(), frame.vertices(), frame.triangles(), frame.indices())
    );
    assert!(frame.quads() > 0, "the client's own region holds geometry");
    assert_eq!(
        frame.clipped(),
        0,
        "the camera stands off so the whole extent is inside its own frustum"
    );
    assert!(frame.shaded() > 0, "rectangles reached pixels");
    assert!(
        frame.coverage() > 0.0 && frame.coverage() < 1.0,
        "a frame that covered every pixel would be a frame with no frame in it: {}",
        frame.coverage()
    );
    assert!(
        frame.bands() >= BAR_MINIMUM_BANDS,
        "the FMA bar on the frame's own {}, measured {}",
        frame.shaded(),
        frame.bands()
    );
    let (near, far) = frame.depth_extent().expect("something was drawn");
    let camera = frame.camera();
    assert!(
        near >= camera.near && far <= camera.far,
        "the depths kept are inside the camera's own planes: {near}..{far} against {}..{}",
        camera.near,
        camera.far
    );
    assert_eq!(
        frame.mesh_state(),
        drawing.state,
        "the frame's mesh is keyed by the state the law is in"
    );
    assert!(
        drawing.bars_hold(),
        "the drawn reading's own bars: {}",
        drawing.bars_described()
    );
    println!("U4 drawn: {}", drawing.described());
}

#[test]
fn the_drawn_frame_is_a_function_of_the_state_alone() {
    // The second bar: one state, one frame — drawn twice, and stepped to the same tick by another path. And
    // the law's own step moves both the state the mesh is keyed by and the picture itself.
    let settled = plane_after(shploingus_clean_room::automaton::SETTLE_TICKS);
    let in_two_halves = {
        let mut half = plane_after(shploingus_clean_room::automaton::SETTLE_TICKS / 2);
        half.steps(shploingus_clean_room::automaton::SETTLE_TICKS / 2)
            .expect("the law steps");
        half
    };
    let one = drawn_of(&settled).expect("the settled world draws");
    let twice = drawn_of(&settled).expect("the same world draws again");
    let by_another_path = drawn_of(&in_two_halves).expect("the same state draws");
    println!(
        "U4 drawn: settled frame {}, again {}, two paths {}",
        one.digest().to_hex(),
        twice.digest().to_hex(),
        by_another_path.digest().to_hex()
    );
    assert_eq!(one.digest(), twice.digest(), "one mesh, one camera, one picture");
    assert_eq!(
        one.digest(),
        by_another_path.digest(),
        "the frame follows the state and not the path the law took to it"
    );
    assert_eq!(one.mesh_state(), settled.digest(), "the mesh's key is the state");

    let placed = plane_after(0);
    let at_placement = drawn_of(&placed).expect("the placed world draws");
    println!(
        "U4 drawn: where it was placed: frame {}, mesh {}, state {}, {} rectangle(s); at rest: frame {}, \
         mesh {}, state {}, {} rectangle(s)",
        at_placement.digest().to_hex(),
        at_placement.mesh().to_hex(),
        at_placement.mesh_state().to_hex(),
        at_placement.quads(),
        one.digest().to_hex(),
        one.mesh().to_hex(),
        one.mesh_state().to_hex(),
        one.quads()
    );
    assert_ne!(
        at_placement.mesh_state(),
        one.mesh_state(),
        "the law's own step moves the state the mesh is keyed by"
    );
    assert_ne!(
        at_placement.mesh(),
        one.mesh(),
        "and it moves the geometry the mesh holds"
    );
    assert_ne!(
        at_placement.digest(),
        one.digest(),
        "and the picture follows the state: {} rectangle(s) at rest against {} where it was placed",
        one.quads(),
        at_placement.quads()
    );
}

#[test]
fn the_camera_stands_the_regions_own_extent_in_front_of_its_own_frustum() {
    // The third bar: the camera's own arithmetic, checked against the mesh's own bounds rather than trusted —
    // the eye's distance, its direction, its planes, and every corner of every rectangle inside the frame.
    let plane = plane_after(shploingus_clean_room::automaton::SETTLE_TICKS);
    let mesh = mesh_of(&plane).expect("the client's own region meshes");
    // **The camera the frame is drawn under is the client's own view, derived from the person's state**
    // (2026-10-14e): the drawn path no longer keeps the `over` placement beside the eye, it *derives* the
    // camera from the viewpoint the declared view reads as (`surface::home_viewpoint`). So the guard is the
    // standoff placement itself — the eye, the look, the radius, the planes and the up — and the one field
    // that differs by design: the target is the person's own ray `LOOK_M` ahead rather than the extent's
    // centre, which is what makes the camera *the person's* rather than the extent's.
    let declared = camera_over(&mesh);
    let frame = drawn_of(&plane).expect("the client's own region draws");
    let camera = frame.camera();
    assert_eq!(camera.eye, declared.eye, "the frame's own eye is the declared view's");
    assert_eq!(camera.look, declared.look, "the frame's own look is the declared view's");
    assert_eq!(camera.radius, declared.radius, "the region's own radius");
    assert_eq!(camera.near, declared.near, "the region's own near plane");
    assert_eq!(camera.far, declared.far, "the region's own far plane");
    assert_eq!(camera.up, declared.up, "the world's own up");
    assert!(
        (camera.distance() - drawn::LOOK_M).abs() < 1e-12,
        "the frame's own target is not {} m along the person's own look: {}",
        drawn::LOOK_M,
        camera.distance()
    );
    let (low, high) = drawn::bounds(&mesh).expect("the mesh holds rectangles");
    let target = [
        (low[0] + high[0]) / 2.0,
        (low[1] + high[1]) / 2.0,
        (low[2] + high[2]) / 2.0,
    ];
    let radius = ((high[0] - low[0]).powi(2) + (high[1] - low[1]).powi(2) + (high[2] - low[2]).powi(2))
        .sqrt()
        / 2.0;
    let toward = surface::view().toward;
    let distance = radius / (drawn::DRAWN_FOV_DEG / 2.0).to_radians().sin();
    assert!((camera.radius - radius).abs() < 1e-12, "{} against {radius}", camera.radius);
    for axis in 0..3 {
        let expected = target[axis] + toward[axis] * distance;
        assert!(
            (camera.eye[axis] - expected).abs() < 1e-9,
            "the eye stands off along the view direction on axis {axis}: {} against {expected}",
            camera.eye[axis]
        );
    }
    assert!(camera.near > 0.0 && camera.far > distance);
    let size = drawn::DRAWN_SIZE;
    // **The extent's own centre is where the eye looks**, so it projects to the middle of the frame and its
    // camera-space depth is the standoff: the look is derived from the eye-and-centre pair, and this is that
    // fact measured on the camera rather than asserted about it.
    let (x, y, depth) = camera
        .project(target, size)
        .expect("a projection")
        .expect("the centre is in front of the eye");
    assert!((x - f64::from(size) / 2.0).abs() < 1e-6 && (y - f64::from(size) / 2.0).abs() < 1e-6);
    assert!((depth - distance).abs() < 1e-6, "{depth} against the standoff {distance}");
    let mut corners = 0_u64;
    for quad in mesh.quads() {
        for corner in drawn::corners(quad) {
            corners += 1;
            let (x, y, depth) = camera
                .project(corner, size)
                .expect("a projection")
                .expect("every corner is in front of the eye");
            assert!(depth >= camera.near && depth <= camera.far, "{depth}");
            assert!(
                (0.0..=f64::from(size)).contains(&x) && (0.0..=f64::from(size)).contains(&y),
                "a corner outside the frame at ({x}, {y})"
            );
        }
    }
    println!(
        "U4 drawn: {} corner(s) inside the frustum; camera {}",
        corners,
        camera.described()
    );
    assert_eq!(corners, mesh.vertices());
}

/// **An independent recount of one frame** — the projection, the barycentric test, the `1/z` depth and the
/// transfer function written out again in this file, sharing no code with `drawn.rs`'s own loop, and run over
/// a **declared sample** of pixels rather than the whole frame. For every sampled pixel it answers the same
/// two questions the frame answers: *is any rectangle here, and which one is nearest*.
///
/// Returns `(sampled, covered-by-the-recount)` for the sample, with the frame's own buffer to be compared
/// pixel by pixel here.
fn recount(mesh: &shploingus_clean_room::mesh::Mesh, frame: &drawn::Drawn, sample: u32) -> (u64, u64) {
    let camera = frame.camera();
    let size = frame.size();
    let light = surface::light();
    let registry = reference::registry();
    let exposure = surface::EXPOSURE;
    // Each rectangle's four corners, projected once, with its own shaded colour — the picture's raw material.
    let mut quads = Vec::with_capacity(mesh.quads().len());
    for quad in mesh.quads() {
        let material = registry
            .material(quad.material)
            .expect("the mesh's own materials are admitted");
        // **The contract's own one conversion** (2026-10-14f), so this recount shades exactly what the frame
        // shaded and a second copy of the conversion cannot drift from it.
        let present = presentation::surface_of(material, surface::ROUGHNESS, surface::METALNESS)
            .expect("the mesh's own materials carry colour");
        let corners = drawn::corners(quad);
        let centre = [
            (corners[0][0] + corners[2][0]) / 2.0,
            (corners[0][1] + corners[2][1]) / 2.0,
            (corners[0][2] + corners[2][2]) / 2.0,
        ];
        let view = unit([
            camera.eye[0] - centre[0],
            camera.eye[1] - centre[1],
            camera.eye[2] - centre[2],
        ]);
        let shaded = presentation::shade(
            &present,
            drawn::normal_of(quad),
            &light,
            &presentation::View { toward: view },
        )
        .expect("the recount shades the same rectangle the frame shaded");
        let projected: Vec<(f64, f64, f64)> = corners
            .iter()
            .map(|corner| {
                camera
                    .project(*corner, size)
                    .expect("a projection")
                    .unwrap_or((f64::NAN, f64::NAN, f64::NAN))
            })
            .collect();
        quads.push((projected, presentation::encode(shaded, exposure)));
    }

    let mut sampled = 0_u64;
    let mut covered = 0_u64;
    let mut y = sample / 2;
    while y < size {
        let mut x = sample / 2;
        while x < size {
            sampled += 1;
            let point = (f64::from(x) + 0.5, f64::from(y) + 0.5);
            let mut best: Option<(f64, [u8; 4])> = None;
            for (projected, colour) in &quads {
                if projected.iter().any(|corner| corner.2.is_nan()) {
                    continue;
                }
                for triangle in drawn::INDICES {
                    let a = projected[triangle[0]];
                    let b = projected[triangle[1]];
                    let c = projected[triangle[2]];
                    // The bounding box, then the barycentric coverage: the same two triangles the frame
                    // submitted, tested by arithmetic written out here.
                    if point.0 < a.0.min(b.0).min(c.0)
                        || point.0 > a.0.max(b.0).max(c.0)
                        || point.1 < a.1.min(b.1).min(c.1)
                        || point.1 > a.1.max(b.1).max(c.1)
                    {
                        continue;
                    }
                    let area = (b.0 - a.0) * (c.1 - a.1) - (c.0 - a.0) * (b.1 - a.1);
                    if area == 0.0 {
                        continue;
                    }
                    let w0 = ((b.1 - c.1) * (point.0 - c.0) + (c.0 - b.0) * (point.1 - c.1)) / area;
                    let w1 = ((c.1 - a.1) * (point.0 - c.0) + (a.0 - c.0) * (point.1 - c.1)) / area;
                    let w2 = 1.0 - w0 - w1;
                    if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                        continue;
                    }
                    let z = 1.0 / (w0 / a.2 + w1 / b.2 + w2 / c.2);
                    if best.map(|(depth, _)| z < depth).unwrap_or(true) {
                        best = Some((z, *colour));
                    }
                }
            }
            let theirs = frame.depth_at(x, y);
            let theirs_pixel = frame.pixel(x, y).expect("a pixel of the frame");
            match (theirs, best) {
                (None, None) => {}
                (Some(depth), Some((mine, colour))) => {
                    covered += 1;
                    assert!(
                        (depth - mine).abs() < 1e-9,
                        "at ({x}, {y}) the frame kept depth {depth} and the recount {mine}"
                    );
                    assert_eq!(
                        &theirs_pixel[0..3],
                        &colour[0..3],
                        "at ({x}, {y}) the frame's pixel is not the nearest rectangle's own colour"
                    );
                }
                (Some(depth), None) => panic!(
                    "at ({x}, {y}) the frame kept depth {depth} and the recount found no rectangle there"
                ),
                (None, Some((mine, _))) => panic!(
                    "at ({x}, {y}) the recount found a rectangle at depth {mine} and the frame kept none"
                ),
            }
            x += sample;
        }
        y += sample;
    }
    (sampled, covered)
}

#[test]
fn every_drawn_pixel_is_the_nearest_rectangle_the_mesh_holds() {
    // The fourth bar, and the strongest one: the frame's own depth buffer and pixels against an **independent
    // recount** of the same mesh over a declared sample of pixels (every sixteenth pixel on each axis, so the
    // sample is the whole frame's own grid rather than its centre). The functions compared are the ones the
    // picture is made of — the projection, the coverage test, the depth test and the transfer function — so a
    // recount that agrees is a second implementation agreeing, not one implementation called twice.
    const SAMPLE: u32 = 16;
    let plane = plane_after(shploingus_clean_room::automaton::SETTLE_TICKS);
    let mesh = mesh_of(&plane).expect("the client's own region meshes");
    let frame = drawn_of(&plane).expect("the client's own region draws");
    let (sampled, covered) = recount(&mesh, &frame, SAMPLE);
    println!(
        "U4 drawn: the independent recount sampled {sampled} pixel(s) and found {covered} covered; the frame \
         itself kept {} of {}",
        frame.shaded(),
        frame.pixels_count()
    );
    assert!(sampled > 0, "the sample names pixels");
    assert!(covered > 0, "the mesh reached the sampled pixels");
    assert!(
        covered * 100 > sampled,
        "the sample is covered at all: {covered} of {sampled}"
    );
}

#[test]
fn one_cells_material_moves_the_frame_and_nothing_else_does() {
    // **The pass's own acceptance (2026-10-14f): the cell is the single owner of its material.** Two worlds
    // that differ in **one cell's material** and in nothing else are drawn through the same door, and the
    // picture moves with that cell — because the material a frame draws is the cell's own and not a class
    // ladder's second answer.
    //
    // The cell is chosen from the world's own cells: the **tallest dry surface cell** inside the region whose
    // material is not the one about to be written, so the face that changes is a face the eye can see (the
    // tallest column in the region is occluded by nothing in it), and the material written is another
    // admitted one (quartz against whatever the cell holds), so the pixels that change are a colour the
    // record carries rather than a plausible grey.
    let plane = plane_after(shploingus_clean_room::automaton::SETTLE_TICKS);
    let before = drawn_of(&plane).expect("the client's own region draws");
    let world = plane.world();
    let base = surface::DRAWN_CHUNK.origin().expect("the region's own origin is a place");
    let side = CHUNK_SIDE * surface::DRAWN_CHUNKS as i32;
    let written = MaterialId(4);
    let mut chosen: Option<(i32, i32, i32, MaterialId)> = None;
    for ty in 0..side {
        for tx in 0..side {
            let at = VoxelPos::new(base.x + tx, base.y + ty, base.z);
            let Ok(column) = world.column(at) else {
                continue;
            };
            let (Some(material), false) = (column.material, column.water_stands()) else {
                continue;
            };
            let other = if material == written { MaterialId(1) } else { written };
            if chosen.is_none_or(|(best, _, _, _)| column.surface_z > best) {
                chosen = Some((column.surface_z, at.x, at.y, other));
            }
        }
    }
    let (ground_z, x, y, other) = chosen.expect("the region holds a dry surface cell");
    let cell = VoxelPos::new(x, y, ground_z);
    let held = world.material_at(cell).expect("the cell is the world's own");
    let mut changed = world.clone();
    changed
        .place_material(cell, Some(other))
        .expect("the cell is the world's own");
    assert_eq!(changed.material_at(cell).expect("the cell is there"), Some(other));
    let residency = surface::residency_of(&changed).expect("the changed world has a chunk to stand in");
    let (after, _) = surface::draw_plane(&residency, surface::DRAWN_CHUNK)
        .expect("the changed world draws through the client's own door");
    println!(
        "U4 drawn: one cell ({}, {}, {}) written {} -> {}: frame {} -> {} ({} -> {} pixels drawn), mesh {} -> \
         {}, state {} -> {}",
        cell.x,
        cell.y,
        cell.z,
        held.map(|id| id.0).unwrap_or(0),
        other.0,
        before.digest().to_hex(),
        after.digest().to_hex(),
        before.shaded(),
        after.shaded(),
        before.mesh().to_hex(),
        after.mesh().to_hex(),
        before.mesh_state().to_hex(),
        after.mesh_state().to_hex()
    );
    assert_ne!(
        before.mesh_state(),
        after.mesh_state(),
        "the world's own identity covers its cells, so a written cell moves the state"
    );
    assert_eq!(
        after.mesh_state(),
        changed.digest(),
        "and the frame is keyed by the cells it was drawn from"
    );
    // **The geometry itself moves**, measured on the rectangles rather than on a digest that frames the
    // state: `Mesh::digest` covers the geometry key, so it would move with the cells alone. The claim here
    // is that the *merge* broke where the material changed — the same faces, cut into different rectangles —
    // and that one of them is drawn in the material that was written.
    let before_mesh = mesh_of(&plane).expect("the client's own region meshes");
    let mesh = surface::region_mesh(&residency, surface::DRAWN_CHUNK).expect("the changed region meshes");
    assert_ne!(
        before_mesh.quads(),
        mesh.quads(),
        "the geometry follows the cell: the face that changed is a different rectangle set"
    );
    assert!(
        mesh.quads().iter().any(|quad| quad.material == other),
        "the written material is drawn: no rectangle of the changed mesh holds it"
    );
    assert_ne!(
        before.digest(),
        after.digest(),
        "**and so does the picture** — the material a drawn frame shows is the cell's own"
    );
    let (declared_quads, _, _, _) = after.declared();
    assert_eq!(after.quads(), declared_quads, "and the frame still counts the mesh's own rectangles");
}
