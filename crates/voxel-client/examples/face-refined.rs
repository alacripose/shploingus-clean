//! **The refined path, on the shipped surface.** A real world built by the shipped source, meshed
//! by the shipped mesher, drawn by the shipped renderer — with one thing turned on that the world
//! currently leaves off: a **displacement law**, so these faces leave the 24-byte record format
//! for the vertex format and are drawn by `vs_main`.
//!
//! The subjects are walls of the **generated world**: a trunk's two vertical sides (`WOOD`, whose
//! shaper is `Bark`, the directional material the user's report was about) and a rock face's two
//! turns (`Brushed`, where the face's basis and the world's disagree on a wall turned along `x`).
//! Each is seen from an eye a player would stand at: 1.7 m above the bottom of the face, 2.0 m off
//! it, looking level. A candidate face is **screened** — the frame is compared with the law at nine
//! points and the face is only measured if the picture is that face — so a face whose eye is
//! buried in terrain is rejected rather than measured.
//!
//! **The frame is the shipped one.** `Scene::build_with_light` is the constructor the window uses
//! whenever its light field is bound, and it puts **every** quad on the vertex format; the field
//! itself is left unbound here (`light_present = 0`), which is the shader's own documented
//! fallback to the rig — so one light source lights the surface and `shade_cpu` can be its
//! reference. The sky model is off and the tonemap is zero for the same reason.
//!
//! **Before and after are two runs of this one binary.** `SurfacePattern::stretch_axis` is the
//! whole law: the run labelled `before` carries its world-axis table — `Bark` along `z`, `Brushed`
//! along `x`, exactly what `stencil` hardcoded before the fix — and the run labelled `after` the
//! face-relative one. World, meshes, eye, scene and shader are identical between the two, so the
//! difference between the PNGs is the fix and nothing else.
//!
//! Usage: `cargo run --example face-refined -- before` (then `-- after`)

use std::fmt::Write as _;

use voxel_appearance::npr::LightRig;
use voxel_appearance::openpbr::OpenPbrInputs;
use voxel_client::camera::Camera;
use voxel_client::pattern::{address_at, eye_footprint_m, pixel_angle_rad, Pattern};
use voxel_client::shading;
use voxel_client::{Frame, GpuRig, Renderer, Scene};
use voxel_mesh::{Chunk, Mesh, MeshConfig, Mesher, Quad, RefineConfig};
use voxel_model::{ChunkPos, DisplacementField, Face, MaterialId, SUBVOXEL_SIDE};
use voxel_noise::Vec3;
use voxel_shapers::materials::{self, SurfacePattern};
use voxel_shapers::{gaia, Config, WorldSpec};
use voxel_stream::WorldSource;

/// 1025 is odd, so pixel 512 is the frame's exact centre.
const SIDE: u32 = 1025;
/// A player's field of view, narrowed: a trunk is 0.5 m of a 2.3 m tall view at 2 m.
const FIELD_DEG: f64 = 60.0;
/// A player's eye above the bottom of the face.
const EYE_HEIGHT_M: f64 = 1.7;
/// How far in front of the face the eye stands.
const STAND_OFF_M: f64 = 2.0;
/// The largest half-width of the sampled patch on a face, in metres.
const BAND_M: f64 = 0.32;
/// A bare amplitude of half a subvoxel: the whole face moves out as one plane, so the refinement
/// emits triangles without bending the face off the plane its normal names.
const LIFT_SUBVOXELS: f64 = 0.5;
/// The world the faces are taken from.
const SEED: u64 = 2468;

fn srgb(v: f32) -> u8 {
    let v = v.clamp(0.0, 1.0) as f64;
    let encoded = if v <= 0.003_130_8 {
        12.92 * v
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    };
    (encoded * 255.0 + 0.5) as u8
}

fn write_png(frame: &Frame, path: &str) {
    let mut bytes = Vec::with_capacity((SIDE * SIDE * 4) as usize);
    for pixel in frame.pixels.iter() {
        bytes.push(srgb(pixel[0]));
        bytes.push(srgb(pixel[1]));
        bytes.push(srgb(pixel[2]));
        bytes.push(255);
    }
    let file = std::fs::File::create(path).expect("a file to write into target/");
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), SIDE, SIDE);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("a PNG header");
    writer.write_image_data(&bytes).expect("the pixels");
    drop(writer);
    println!("wrote {path}");
}

/// A world point's pixel, through the camera's own matrix — the same `view_projection` the
/// vertex stage multiplies by, so a sample lands where the fragment did.
fn project(camera: &Camera, p: [f64; 3]) -> Option<(f64, f64)> {
    let m = camera.view_projection();
    let rel = [
        p[0] - camera.position_m[0],
        p[1] - camera.position_m[1],
        p[2] - camera.position_m[2],
    ];
    let mut clip = [0.0f64; 4];
    for (r, cell) in clip.iter_mut().enumerate() {
        *cell = (0..4)
            .map(|k| m[k][r] as f64 * if k == 3 { 1.0 } else { rel[k] })
            .sum();
    }
    if clip[3] <= 1.0e-9 {
        return None;
    }
    Some((
        (clip[0] / clip[3] + 1.0) * 0.5 * SIDE as f64,
        (1.0 - clip[1] / clip[3]) * 0.5 * SIDE as f64,
    ))
}

/// Bilinear read of the red channel, or `None` off the edge of the frame.
fn sample(frame: &Frame, x: f64, y: f64) -> Option<f64> {
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let at = |i: f64, j: f64| -> Option<f64> {
        if i < 0.0 || j < 0.0 || i > (SIDE - 1) as f64 || j > (SIDE - 1) as f64 {
            return None;
        }
        frame.pixel(i as u32, j as u32).map(|p| p[0] as f64)
    };
    let (a, b) = (at(x0, y0)?, at(x0 + 1.0, y0)?);
    let (c, d) = (at(x0, y0 + 1.0)?, at(x0 + 1.0, y0 + 1.0)?);
    let top = a * (1.0 - fx) + b * fx;
    let bottom = c * (1.0 - fx) + d * fx;
    Some(top * (1.0 - fy) + bottom * fy)
}

/// `mesh::axes`: axis 0 is (y, z), axis 1 is (x, z), axis 2 is (x, y) — the two coordinates a face
/// on that axis varies across, in that order.
fn face_axes(axis: usize) -> (usize, usize) {
    match axis {
        0 => (1, 2),
        1 => (0, 2),
        _ => (0, 1),
    }
}

fn median(mut v: Vec<f64>) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    v.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    v[v.len() / 2]
}

/// **One vertical face of the world, in world metres.**
struct Wall {
    label: String,
    face: Face,
    /// The plane's own world coordinate on `face.axis()`, in metres.
    plane_m: f64,
    /// The in-plane axis that is not `y`: the face's width.
    across_axis: usize,
    across_range: (f64, f64),
    /// The face's height, along `y`.
    height_range: (f64, f64),
    normal: [f64; 3],
    /// The outward displacement the declared law applies to this face, in metres.
    lift_m: f64,
    material: MaterialId,
    surface: SurfacePattern,
    inputs: OpenPbrInputs,
}

impl Wall {
    /// A point on the face **as drawn**: the rectangle moved out by the declared amplitude.
    fn point(&self, across: f64, y: f64) -> [f64; 3] {
        let mut p = [0.0f64; 3];
        p[self.face.axis() as usize] = self.plane_m + self.lift_m;
        p[self.across_axis] = across;
        p[1] = y;
        p
    }

    /// The plane, in the units `SurfacePattern::stencil` takes.
    fn plane(&self) -> Option<u32> {
        Some(self.face.axis() as u32)
    }

    fn slug(&self) -> String {
        let name = match self.surface {
            SurfacePattern::Bark => "bark",
            SurfacePattern::Brushed => "brushed",
            SurfacePattern::Speckle => "speckle",
            SurfacePattern::Foliage => "foliage",
        };
        let turn = match self.face {
            Face::PosX => "posx",
            Face::NegX => "negx",
            Face::PosZ => "posz",
            _ => "negy",
        };
        format!("{name}-{turn}")
    }
}

/// The crate's reading of one world point: the colour `shade_cpu` makes of the deviation this
/// fragment shows, under the plane the law is asked for.
struct Law<'a> {
    pattern: Pattern<'a>,
    rig: LightRig,
    eye: [f64; 3],
    pixel_angle: f64,
}

impl Law<'_> {
    fn red(&self, wall: &Wall, p: [f64; 3], plane: Option<u32>) -> Option<f64> {
        let n = wall.normal;
        let d = [p[0] - self.eye[0], p[1] - self.eye[1], p[2] - self.eye[2]];
        let distance = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        if distance <= 0.0 {
            return None;
        }
        let facing = (n[0] * d[0] + n[1] * d[1] + n[2] * d[2]).abs() / distance;
        let footprint = eye_footprint_m(distance, self.pixel_angle);
        let deviation = self
            .pattern
            .at(wall.material, p, footprint, wall.surface)
            .expect("a finite point");
        let varied = wall
            .inputs
            .with_detail(deviation)
            .expect("a legal deviation");
        let view = Vec3::new(-d[0] / distance, -d[1] / distance, -d[2] / distance);
        shading::shade_cpu(&varied, Vec3::new(n[0], n[1], n[2]), view, 0.0, &self.rig)
            .ok()
            .map(|(c, _)| c.r)
    }
}

/// Where the eye stands and what it sees of one face.
struct Shot {
    eye: [f64; 3],
    camera: Camera,
    across_mid: f64,
    eye_y: f64,
    band: f64,
    frame: Frame,
}

/// Build the shipped frame for one face at a player's eye, and report how far the picture is from
/// the law **of this binary's table** on a nine-point grid — a face whose eye is buried in
/// terrain fails that test, and is not measured.
fn shoot(
    wall: &Wall,
    meshes: &[Mesh],
    displacement: &DisplacementField,
    refine: &RefineConfig,
    renderer: &mut Renderer,
    lattice: &voxel_appearance::DetailLattice,
    field: &voxel_model::ShadingField,
    pixel_angle: f64,
) -> (Shot, f64) {
    let (across_lo, across_hi) = wall.across_range;
    let (y_lo, y_hi) = wall.height_range;
    let across_mid = 0.5 * (across_lo + across_hi);
    // The band shrinks to the face when a face is only a step tall.
    let band = BAND_M
        .min(0.4 * (y_hi - y_lo))
        .min(0.4 * (across_hi - across_lo))
        .max(0.02);
    // The eye: 1.7 m above the bottom of the face, 2 m off it, looking level — where a player
    // stands at the foot of a trunk or a step. Clamped so the sampled band is on the face.
    let eye_y = (y_lo + EYE_HEIGHT_M).clamp(y_lo + band + 0.02, y_hi - band - 0.02);
    let on_face = wall.point(across_mid, eye_y);
    let eye = [
        on_face[0] + wall.normal[0] * STAND_OFF_M,
        on_face[1],
        on_face[2] + wall.normal[2] * STAND_OFF_M,
    ];
    let mut camera = Camera::standard(eye, 1.0);
    camera.fov_y_rad = FIELD_DEG.to_radians();
    let towards = [on_face[0] - eye[0], 0.0, on_face[2] - eye[2]];
    camera.yaw_rad = towards[0].atan2(towards[2]);
    camera.pitch_rad = 0.0;

    let scene = Scene::build_with_light(
        meshes.iter(),
        eye,
        &(|_: [f64; 3]| None),
        Some(displacement),
        Some(refine),
    );
    let frame = renderer.render(&scene, &camera).expect("a frame");
    let law = Law {
        pattern: Pattern::new(lattice, field),
        rig: LightRig::standard(),
        eye,
        pixel_angle,
    };
    let mut errs = Vec::new();
    for i in 0..9 {
        for j in 0..9 {
            let across = across_mid + band * (2.0 * (i as f64 + 0.5) / 9.0 - 1.0);
            let y = eye_y + band * (2.0 * (j as f64 + 0.5) / 9.0 - 1.0);
            let p = wall.point(across, y);
            let Some(drawn) = project(&camera, p).and_then(|(x, y)| sample(&frame, x, y)) else {
                continue;
            };
            if let Some(want) = law.red(wall, p, wall.plane()) {
                errs.push((drawn - want).abs());
            }
        }
    }
    let shot = Shot {
        eye,
        camera,
        across_mid,
        eye_y,
        band,
        frame,
    };
    (shot, median(errs))
}

fn main() {
    let phase = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "after".to_string());
    assert!(
        phase == "before" || phase == "after",
        "usage: face-refined [before|after] — the label of the `stretch_axis` table this binary carries"
    );
    let spec = WorldSpec::new(SEED, Config::default());

    // ---- a trunk of the real world -------------------------------------------------------
    let mut tree = None;
    'search: for cell_x in 0..96 {
        for cell_z in 0..96 {
            if let Some(found) = voxel_shapers::gaia::tree_of_cell(&spec, cell_x, cell_z) {
                tree = Some(found);
                break 'search;
            }
        }
    }
    let tree = tree.expect("no tree in the first 96x96 cells of this world");
    println!(
        "tree: trunk at ({}, {}), ground top y={}, {} voxels tall, canopy {:?}",
        tree.base_x, tree.base_z, tree.base_y, tree.trunk_height, tree.canopy_material
    );

    let source = WorldSource::new(spec);
    let registry = materials::registry().expect("the registry");
    let mesh_config = MeshConfig::from_registry(&registry).expect("a mesh config");
    let mut mesher = Mesher::new();
    let side = voxel_model::coord::CHUNK_SIDE;
    let mut meshes: Vec<Mesh> = Vec::new();
    let (cx0, cz0) = (tree.base_x.div_euclid(side), tree.base_z.div_euclid(side));
    let cy0 = tree.base_y.div_euclid(side);
    for cy in cy0..=(tree.base_y + tree.trunk_height).div_euclid(side) {
        for cx in (cx0 - 3)..=(cx0 + 3) {
            for cz in (cz0 - 3)..=(cz0 + 3) {
                let pos = ChunkPos::new(cx, cy, cz);
                let Ok(owned) = source.build_chunk(pos) else {
                    continue;
                };
                let chunk = Chunk::from_tree(pos, owned);
                let neighborhood = source.neighborhood(pos);
                if let Ok(mesh) = mesher.build(&chunk, &mesh_config, Some(&neighborhood)) {
                    meshes.push(mesh);
                }
            }
        }
    }
    assert!(
        !meshes.is_empty(),
        "the world produced no chunks around the trunk"
    );
    println!(
        "meshed {} chunks, {} quads, {} of them wood",
        meshes.len(),
        meshes.iter().map(|m| m.quads.len()).sum::<usize>(),
        meshes
            .iter()
            .flat_map(|m| m.quads.iter())
            .filter(|q| q.material == materials::WOOD)
            .count()
    );

    // ---- the world's shading declaration and the GPU -------------------------------------
    let field = materials::shading_field().expect("the shading field declares");
    let table = shading::table(&registry, &field).expect("every admitted material derives");
    let rig = LightRig::standard();
    let gpu_rig = GpuRig::from_rig(&rig).expect("the standard rig fits");
    let mut renderer = Renderer::new(SIDE, SIDE, &table, &gpu_rig).expect("a GPU");
    renderer.world_mut().write_no_sky_model();
    renderer.world_mut().set_tonemap(0.0);
    println!("adapter: {}", renderer.adapter);

    // ---- the declared displacement: a bare amplitude on every face of the two materials ---
    //
    // This is the part the world does not declare yet (`materials::displacement_field` is empty
    // because generation places whole voxels), and it is what puts these faces on the vertex path.
    // A bare amplitude translates a plane, so the face stays flat and the only thing the stencil
    // has to decide is which **basis** it reads the grain in.
    // **Every material of the registry**, so the whole frame is the vertex path rather than a
    // mixture: a material the field does not name keeps its 24-byte record and is drawn by
    // `vs_quad`, and a face sampled as if it had moved would then be measured a fifth of a metre
    // in front of where it is.
    let mut displacement = DisplacementField::none();
    let mut laws = 0usize;
    for face in [
        Face::PosX,
        Face::NegX,
        Face::PosY,
        Face::NegY,
        Face::PosZ,
        Face::NegZ,
    ] {
        for material in registry.materials() {
            if displacement
                .declare(material.id, face, LIFT_SUBVOXELS)
                .is_ok()
            {
                laws += 1;
            }
        }
    }
    println!("declared displacement on {laws} (material, face) pairs, half a subvoxel each");
    let refine = RefineConfig::new(4, 4096).expect("4 samples a block is inside the level");
    let lattice = voxel_appearance::DetailLattice::new(field.domain());
    let pixel_angle = pixel_angle_rad(FIELD_DEG.to_radians(), SIDE) as f64;

    // ---- the subjects --------------------------------------------------------------------
    for (surface, face) in [
        (SurfacePattern::Bark, Face::PosZ),
        (SurfacePattern::Bark, Face::PosX),
        (SurfacePattern::Brushed, Face::PosX),
        (SurfacePattern::Brushed, Face::PosZ),
    ] {
        let mut accepted = false;
        for candidate in pick_walls(&meshes, surface, face, &registry) {
            let (shot, screen) = shoot(
                &candidate,
                &meshes,
                &displacement,
                &refine,
                &mut renderer,
                &lattice,
                &field,
                pixel_angle,
            );
            if !(screen < 5.0e-3) {
                println!(
                    "  {} ({} {:?}): rejected — the frame is {screen:.1e} off this face's law",
                    candidate.label, candidate.material.0, surface
                );
                continue;
            }
            accepted = true;
            let wall = &candidate;
            println!(
                "\n=== {} ({} {:?}): a player's eye {:.1} m off the face, {:.2} m above its foot ===",
                wall.label,
                wall.material.0,
                surface,
                STAND_OFF_M,
                shot.eye_y - wall.height_range.0
            );
            let slug = wall.slug();
            write_png(
                &shot.frame,
                &format!("target/face-refined-{slug}-{phase}.png"),
            );

            // ---- the stencil's own cell along each in-plane axis -------------------------
            //
            // One subvoxel of `at` is sixteen base cells and the floor rung's mask halves the
            // count, so a stretched axis reads a quarter of the crossings a square one does.
            let cell_mm = |axis: usize, plane: Option<u32>| -> f64 {
                let mut crossings = 0usize;
                let mut previous: Option<(u16, u16, u16)> = None;
                for i in 0..16u16 {
                    let mut a = [8u16; 3];
                    a[axis] = i;
                    let at =
                        voxel_model::DetailIndex::new(a[0], a[1], a[2]).expect("inside the side");
                    let s = wall.surface.stencil(at, plane);
                    let q = (s.x >> 1, s.y >> 1, s.z >> 1);
                    if let Some(p) = previous {
                        if p != q {
                            crossings += 1;
                        }
                    }
                    previous = Some(q);
                }
                250.0 / crossings.max(1) as f64
            };
            println!(
                "  cell: across (world axis {}) {:.1} mm on the face / {:.1} mm on the corner fallback; \
                 up {:.1} / {:.1} mm",
                wall.across_axis,
                cell_mm(wall.across_axis, wall.plane()),
                cell_mm(wall.across_axis, None),
                cell_mm(1, wall.plane()),
                cell_mm(1, None)
            );

            // ---- every sampled pixel against the law ------------------------------------
            let law = Law {
                pattern: Pattern::new(&lattice, &field),
                rig: LightRig::standard(),
                eye: shot.eye,
                pixel_angle,
            };
            const GRID: usize = 41;
            let mut dump = String::new();
            let mut face_err = Vec::new();
            let mut fallback_err = Vec::new();
            for i in 0..GRID {
                for j in 0..GRID {
                    let across =
                        shot.across_mid + shot.band * (2.0 * (i as f64 + 0.5) / GRID as f64 - 1.0);
                    let y = shot.eye_y + shot.band * (2.0 * (j as f64 + 0.5) / GRID as f64 - 1.0);
                    let p = wall.point(across, y);
                    let drawn =
                        project(&shot.camera, p).and_then(|(x, y)| sample(&shot.frame, x, y));
                    writeln!(&mut dump, "{}", drawn.unwrap_or(f64::NAN)).expect("a dump");
                    let Some(drawn) = drawn else { continue };
                    if let Some(want) = law.red(wall, p, wall.plane()) {
                        face_err.push((drawn - want).abs());
                    }
                    if let Some(want) = law.red(wall, p, None) {
                        fallback_err.push((drawn - want).abs());
                    }
                }
            }
            std::fs::write(format!("target/face-refined-{slug}-{phase}.txt"), dump)
                .expect("a dump file");
            println!(
                "  {} points: median |frame − law on the face| {:.2e}, |frame − corner fallback| {:.2e}",
                face_err.len(),
                median(face_err),
                median(fallback_err)
            );
            break;
        }
        if !accepted {
            println!("  no {surface:?} face on {face:?} was found in view");
        }
    }
}

/// The candidate faces for one subject, largest first: a face has to be tall enough for an eye
/// and a band to sit on it, and the largest rectangle is the one most likely to be in view.
fn pick_walls(
    meshes: &[Mesh],
    surface: SurfacePattern,
    face: Face,
    registry: &voxel_model::MaterialRegistry,
) -> Vec<Wall> {
    let mut found: Vec<(u32, &Mesh, &Quad)> = Vec::new();
    for mesh in meshes {
        for quad in &mesh.quads {
            if quad.face != face || materials::pattern_of(quad.material) != surface {
                continue;
            }
            if !matches!(face, Face::PosX | Face::NegX | Face::PosZ | Face::NegZ) {
                continue;
            }
            let corners = mesh.corner_positions_m(quad);
            let hi_y = corners
                .iter()
                .map(|c| c[1] as f64)
                .fold(f64::NEG_INFINITY, f64::max);
            let lo_y = corners
                .iter()
                .map(|c| c[1] as f64)
                .fold(f64::INFINITY, f64::min);
            let tall = 0.6;
            if hi_y - lo_y < tall {
                continue;
            }
            found.push((quad.area(), mesh, quad));
        }
    }
    found.sort_by(|a, b| b.0.cmp(&a.0));
    found
        .into_iter()
        .take(20)
        .map(|(area, mesh, quad)| {
            let o = mesh.origin_voxel();
            let world: Vec<[f64; 3]> = mesh
                .corner_positions_m(quad)
                .iter()
                .map(|c| {
                    [
                        o[0] as f64 + c[0] as f64,
                        o[1] as f64 + c[1] as f64,
                        o[2] as f64 + c[2] as f64,
                    ]
                })
                .collect();
            let axis = face.axis() as usize;
            let (u_axis, w_axis) = face_axes(axis);
            let across_axis = if u_axis == 1 { w_axis } else { u_axis };
            let span = |which: usize| -> (f64, f64) {
                let lo = world.iter().map(|c| c[which]).fold(f64::INFINITY, f64::min);
                let hi = world
                    .iter()
                    .map(|c| c[which])
                    .fold(f64::NEG_INFINITY, f64::max);
                (lo, hi)
            };
            let n = quad.normal();
            let name = registry
                .material(quad.material)
                .map(|m| m.name.as_str())
                .unwrap_or("?");
            Wall {
                label: format!(
                    "{face:?} {:.1}x{:.1} m at {:.1},{:.1} ({name})",
                    area as f64 / (SUBVOXEL_SIDE * SUBVOXEL_SIDE) as f64,
                    span(1).1 - span(1).0,
                    span(0).0,
                    span(2).0
                ),
                face,
                plane_m: span(axis).0,
                across_axis,
                across_range: span(across_axis),
                height_range: span(1),
                normal: [n[0] as f64, n[1] as f64, n[2] as f64],
                lift_m: LIFT_SUBVOXELS / SUBVOXEL_SIDE as f64,
                material: quad.material,
                surface,
                inputs: shading::inputs_for(registry, quad.material).expect("the material derives"),
            }
        })
        .collect()
}
