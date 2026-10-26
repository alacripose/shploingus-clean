//! **The GPU's beam shadow against the CPU's own ray** — one frame, one wall, one sun.
//!
//! The app draws the beam's shadow with `sun_shadow`'s two cascaded depth maps; the CPU field
//! draws its own by marching the world's trees. This probe puts a known wall on a known ground
//! under a 10.6° sun of its own (the shipped case reads the app's hour) and asks both the same
//! question at the same world metres: *is this patch of ground behind the wall?* The CPU answer is exact — a ray against the
//! wall's plane and its extent — and the GPU answer is the rendered frame through `Renderer`, the
//! offscreen device and pipeline the client uses, not a sampler test.
//!
//! Two rows are measured, one inside each cascade, and a third frame with the wall removed checks
//! that the sampling's bias does not darken the ground it is standing on.
//!
//! # And then the scene the binary itself draws
//!
//! A wall on a private floor is a clean witness and not the shipped frame. So the second case
//! streams **the game's own window** (`GAME_SEED`, `GAME_TICK`, `GAME_WINDOW`), finds a real terrain
//! panel the field says a shadow crosses, builds it through [`Scene::build_with_light_lattice`] with
//! a **clear** corner light, and renders it with shadows on at the range the binary sets — computed
//! from `GAME_WINDOW` and `CHUNK_SIDE`, not written here. The CPU witness is the game's own
//! [`CellField`] on that panel; the comparison is the two edges' positions in metres.
//!
//! The same window's **open ground** is sampled too: an open solid +Y face away from the panel,
//! a grid across it in the same shipped frame, against `shade_cpu` with the face's own pattern.
//! Two private frames of that face — one with the map off and one with it on, both with no
//! vertex light field so the shader takes its rig branch — separate self-shadowing (acne) from
//! another object's cast shadow in the full frame. The face's own map is the acne witness.
//!
//! `cargo run -p voxel-client --release --example gpu-shadow-check`

use std::cell::RefCell;

use voxel_appearance::npr::LightRig;
use voxel_appearance::DetailLattice;
use voxel_appearance::sky::SkyInputs;
use voxel_client::camera::MAX_PITCH_RAD;
use voxel_client::field::{CellField, SubvoxelPos};
use voxel_client::game;
use voxel_client::shading::{self, GpuLight, GpuRig};
use voxel_client::pattern::{address_at, eye_footprint_m, pixel_angle_rad, Pattern, SUBVOXEL_CELL_M};
use voxel_client::{
    skylight, Camera, Frame, MaterialParams, Renderer, Scene, NEAR_RANGE_M, SHADOW_BIAS_M,
    SHADOW_LATTICE, SHADOW_RESOLUTION,
};
use voxel_mesh::{Mesh, MeshStats, Quad};
use voxel_model::{ChunkPos, Face, MaterialId, CHUNK_SIDE};
use voxel_noise::Vec3;
use voxel_shapers::{materials, Config, WorldSpec};

/// Quarter-metres per metre: a face's `slice`, `uv` and `extent` are subvoxel coordinates.
const SUB: u16 = 4;
/// The wall's plane on ground that starts at `x = 0`.
const WALL_X: f64 = 10.5;
/// The two rows, metres down the view: one in the near cascade, one in the far.
const NEAR_ROW_Z: f64 = 22.0;
const FAR_ROW_Z: f64 = 45.0;

/// **The range the binary sets**, from the same constants the binary reads: its window's horizon.
/// `shploingus/src/main.rs` writes exactly this expression into `WorldPipeline::set_shadow_range`.
fn shipped_shadow_range() -> f32 {
    (game::GAME_WINDOW.radius_chunks + 1) as f32 * CHUNK_SIDE as f32
}

fn mesh(faces: &[(Face, u16, [u16; 2], [u16; 2])]) -> Mesh {
    Mesh {
        chunk: ChunkPos::new(0, 0, 0),
        origin: [0, 0, 0],
        quads: faces
            .iter()
            .map(|(face, slice, uv, extent)| Quad {
                face: *face,
                slice: *slice,
                uv: *uv,
                extent: *extent,
                material: MaterialId(0),
            })
            .collect(),
        loops: Vec::new(),
        stats: MeshStats {
            solid_voxels: 1,
            visible_faces: faces.len() as u64,
            quads: faces.len() as u64,
            loops: 0,
        },
    }
}

/// **The controlled scene's sun**: `10.6°` above the horizon, from `+X`. The shipped case does
/// not use this — it reads the app's own hour — so this constant and the tolerance below are the
/// synthetic wall's, not a shipped value restated.
const WALL_SUN_ELEVATION_DEG: f64 = 10.6;

fn toward_sun() -> [f64; 3] {
    let elevation = WALL_SUN_ELEVATION_DEG.to_radians();
    [elevation.cos(), elevation.sin(), 0.0]
}

struct Probe {
    frame: Frame,
    camera: Camera,
    width: u32,
    height: u32,
}

/// One material for the private floor and wall; the shipped case passes the registry's table.
fn wall_material() -> MaterialParams {
    MaterialParams {
        base_color_and_present: [0.7, 0.7, 0.7, 1.0],
        rough_metal: [0.9, 1.0, 0.0, 0.0],
        ..MaterialParams::default()
    }
}

impl Probe {
    fn new(
        scene: &Scene,
        rig: &GpuRig,
        materials: &[MaterialParams],
        camera: Camera,
        shadows: bool,
    ) -> Self {
        let (width, height) = (640u32, 320u32);
        let mut renderer = Renderer::new(width, height, materials, rig).expect("a GPU");
        renderer.world_mut().write_no_sky_model();
        // **The shipped binary's range**, from its own constants. The renderer's default is the
        // seam test's 256 m, which would make the far cascade coarser than the frame this
        // mission names.
        renderer.world_mut().set_shadow_range(shipped_shadow_range());
        renderer.world_mut().set_shadows(shadows);
        let frame = renderer.render(scene, &camera).expect("the frame");
        Self { frame, camera, width, height }
    }

    /// Where a world point lands, or `None` when it is behind the eye or outside the frame.
    /// `pixel` clips for the edge profiles, which are all inside; the strip scan must not read a
    /// clamped pixel as if it were the point's own.
    fn projected(&self, point: [f64; 3]) -> Option<(f64, f64)> {
        let matrix = self.camera.view_projection();
        let v = [
            point[0] - self.camera.position_m[0],
            point[1] - self.camera.position_m[1],
            point[2] - self.camera.position_m[2],
        ];
        let mut clip = [0.0f64; 4];
        for (row, slot) in clip.iter_mut().enumerate() {
            for (column, value) in v.iter().enumerate() {
                *slot += matrix[column][row] as f64 * value;
            }
            *slot += matrix[3][row] as f64;
        }
        if clip[3].abs() < 1e-12 {
            return None;
        }
        let x = clip[0] / clip[3] * 0.5 + 0.5;
        let y = 1.0 - (clip[1] / clip[3] * 0.5 + 0.5);
        if !(0.0..=1.0).contains(&x) || !(0.0..=1.0).contains(&y) {
            return None;
        }
        Some((x * f64::from(self.width), y * f64::from(self.height)))
    }

    fn pixel(&self, point: [f64; 3]) -> (u32, u32) {
        let (x, y) = self.projected(point).unwrap_or((0.0, 0.0));
        (
            (x as u32).min(self.width - 1),
            (y as u32).min(self.height - 1),
        )
    }

    /// The pixel a point lands on, only when that pixel is inside the frame.
    fn visible(&self, point: [f64; 3]) -> Option<(u32, u32)> {
        let (x, y) = self.projected(point)?;
        Some((
            (x as u32).min(self.width - 1),
            (y as u32).min(self.height - 1),
        ))
    }

    /// The single pixel a point lands on — for an edge profile, where a three-by-three maximum
    /// would walk the measured crossing toward the lit side.
    fn at(&self, point: [f64; 3]) -> f64 {
        let (x, y) = self.pixel(point);
        self.frame.luminance_at(x, y).unwrap_or(0.0)
    }

    /// The brightest of the three-by-three pixels around a point: the probe's own guard against
    /// a single dither or pattern cell standing in for the surface.
    fn luminance(&self, point: [f64; 3]) -> f64 {
        let (x, y) = self.pixel(point);
        let mut best = 0.0f64;
        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                let sx = (x as i32 + dx).clamp(0, self.width as i32 - 1) as u32;
                let sy = (y as i32 + dy).clamp(0, self.height as i32 - 1) as u32;
                best = best.max(self.frame.luminance_at(sx, sy).unwrap_or(0.0));
            }
        }
        best
    }
}

/// **The CPU's own answer**: the ray from a ground point toward the sun meets the wall's plane,
/// and the wall is solid where that hit is on it (`y` in 0..1, `z` in 20..60).
fn cpu_shadowed(x: f64, z: f64, sun: [f64; 3]) -> bool {
    let t = (WALL_X - x) / sun[0];
    if t <= 0.0 {
        return false;
    }
    let y = t * sun[1];
    (0.0..=1.0).contains(&y) && (20.0..=60.0).contains(&z)
}

/// The dark interval in one frame on one ground row, and the frame's own lit level there.
fn measure(probe: &Probe, z: f64, sun: [f64; 3]) -> ((f64, f64), (f64, f64), f64) {
    let lit = probe.luminance([WALL_X + 1.0, 0.0, z]);
    let mut cpu = (f64::INFINITY, f64::NEG_INFINITY);
    let mut gpu = (f64::NAN, f64::NAN);
    let mut x = WALL_X - 6.5;
    while x <= WALL_X + 1.5 {
        let value = probe.luminance([x, 0.0, z]);
        if cpu_shadowed(x, z, sun) {
            cpu.0 = cpu.0.min(x);
            cpu.1 = cpu.1.max(x);
        }
        if lit > 0.0 && value < lit * 0.5 {
            gpu.0 = if gpu.0.is_nan() { x } else { gpu.0.min(x) };
            gpu.1 = if gpu.1.is_nan() { x } else { gpu.1.max(x) };
        }
        x += 0.05;
    }
    (cpu, gpu, lit)
}

fn main() {
    let sun = toward_sun();
    let floor = mesh(&[(Face::PosY, 0, [0, 0], [64 * SUB, 64 * SUB])]);
    let wall = mesh(&[(
        Face::PosX,
        (WALL_X * f64::from(SUB)) as u16,
        [0, 20 * SUB],
        [1 * SUB, 40 * SUB],
    )]);
    let light = |_: [f64; 3]| Some([1.0f32, 1.0, 1.0, 0.0, 0.0, 0.0]);
    let build = |meshes: &[&Mesh]| {
        Scene::build_with_light_lattice(
            meshes.iter().copied(),
            [0.0, 1.7, 0.0],
            &light,
            None,
            None,
            SHADOW_LATTICE,
        )
    };
    let with_wall = build(&[&floor, &wall]);
    let without_wall = build(&[&floor]);

    let mut rig = GpuRig::default();
    rig.count[0] = 1;
    rig.lights[0] = GpuLight {
        direction: [sun[0] as f32, sun[1] as f32, sun[2] as f32, 1.0],
        colour: [1.0, 1.0, 1.0, 0.0],
    };

    let camera = || {
        let mut camera = Camera::standard([0.0, 1.7, 0.0], 2.0);
        camera.pitch_rad = -0.08;
        camera
    };
    let materials = [wall_material()];
    let shadowed = Probe::new(&with_wall, &rig, &materials, camera(), true);
    println!(
        "gpu-shadow-check: 640 x 320, sun {:.1}° up, wall x {WALL_X:.1} m, shadows on, range {:.0} m",
        WALL_SUN_ELEVATION_DEG,
        shipped_shadow_range()
    );
    let mut failures: Vec<String> = Vec::new();
    let mut report = Vec::new();
    let near_texel = 2.0 * f64::from(NEAR_RANGE_M) / f64::from(SHADOW_RESOLUTION);
    let far_texel = 2.0 * f64::from(shipped_shadow_range()) / f64::from(SHADOW_RESOLUTION);
    for (label, z, footprint_m) in [("near", NEAR_ROW_Z, near_texel), ("far", FAR_ROW_Z, far_texel)] {
        let (cpu, gpu, lit) = measure(&shadowed, z, sun);
        println!(
            "{label:4} row z {z:4.0} m: CPU shadow x {:+.2}..{:+.2} m; \
             GPU {:+.2}..{:+.2} m (lit {lit:.4}, cascade's own texel {footprint_m:.3} m)",
            cpu.0, cpu.1, gpu.0, gpu.1
        );
        report.push((label, cpu, gpu, footprint_m));
    }

    // **A shadow is not the only thing the bias may not draw.** The same row with the wall gone
    // must be the plane's own light; a self-shadowed receiver would show here and nowhere else.
    let open = Probe::new(&without_wall, &rig, &materials, camera(), true);
    let open_lit = open.luminance([WALL_X + 1.0, 0.0, NEAR_ROW_Z]);
    let mut open_min = f64::INFINITY;
    let mut x = WALL_X - 6.5;
    while x <= WALL_X + 1.5 {
        open_min = open_min.min(open.luminance([x, 0.0, NEAR_ROW_Z]));
        x += 0.05;
    }
    println!(
        "no wall: the same ground row runs {open_min:.4}..{open_lit:.4} — \
         {:.1}% of its own lit level at the darkest",
        100.0 * open_min / open_lit.max(f64::MIN_POSITIVE)
    );
    if open_min <= open_lit * 0.9 {
        failures.push(format!(
            "the open ground shadows itself: {open_min:.4} against {open_lit:.4}"
        ));
    }

    // **The claim.** The map is an orthographic view down the scene's beam, so a displacement
    // of `d` metres along the light moves a ground edge by `d / tan(elevation)` metres along x.
    // Every term below is the cascade's own, not a fitted allowance: the comparison's ±1 texel
    // plus the edge's half-texel rasterisation is 1.5 texels; the shader's single 2 cm normal
    // offset is `SHADOW_BIAS_M / tan`; one pixel at 22-45 m is under 0.10 m, and it is also the
    // measurement's resolution in the negative direction. The midpoint is half a texel plus that
    // same pixel. The base is the contact edge, one texel wider than a free edge because the
    // texel holding the wall's foot also holds the ground in front of it; that texel is the
    // `contact_m` below, not an asserted 0.98 m sliver. Before the fix the near shadow began
    // 2.35 m late and the far drew nothing.
    let elevation = WALL_SUN_ELEVATION_DEG.to_radians();
    let bias_m = f64::from(SHADOW_BIAS_M) / elevation.tan();
    let pixel_m = 0.1;
    for (label, cpu, gpu, texel_m) in report {
        if !gpu.0.is_finite() {
            failures.push(format!("{label}: the frame drew no shadow at all"));
            continue;
        }
        // Signed erosion: positive means the GPU's edge sits inside the CPU's. A shadow that
        // starts earlier than the CPU's or ends later is a displaced receiver and is refused.
        let tip = gpu.0 - cpu.0;
        let base = cpu.1 - gpu.1;
        let footprint_m = 1.5 * texel_m / elevation.tan() + bias_m + pixel_m;
        let contact_m = texel_m / elevation.tan();
        let base_limit = footprint_m + contact_m;
        let midpoint = 0.5 * (gpu.0 + gpu.1) - 0.5 * (cpu.0 + cpu.1);
        let midpoint_limit = 0.5 * texel_m / elevation.tan() + pixel_m;
        let overlap = (gpu.1.min(cpu.1) - gpu.0.max(cpu.0)).max(0.0);
        let covered = overlap / (cpu.1 - cpu.0);
        println!(
            "{label:4} tip {tip:+.2} m (≤ {footprint_m:.2}), base {base:+.2} m \
             (≤ {base_limit:.2}), midpoint {midpoint:+.2} m (≤ {midpoint_limit:.2}), \
             covers {:.0}% of the CPU's span",
            100.0 * covered
        );
        if !(tip >= -pixel_m
            && base >= -pixel_m
            && tip <= footprint_m
            && base <= base_limit
            && midpoint.abs() <= midpoint_limit
            && covered >= 0.25)
        {
            failures.push(format!(
                "{label}: the GPU's shadow spans {:+.2}..{:+.2} m where the CPU's spans \
                 {:+.2}..{:+.2} m — tip {tip:+.2}/{footprint_m:.2}, base {base:+.2}/{base_limit:.2}, \
                 midpoint {midpoint:+.2}/{midpoint_limit:.2}, coverage {:.0}%",
                gpu.0,
                gpu.1,
                cpu.0,
                cpu.1,
                100.0 * covered
            ));
        }
    }

    if let Err(message) = shipped_case() {
        failures.push(message);
    }
    assert!(
        failures.is_empty(),
        "gpu-shadow-check, {} failure(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn length(a: [f64; 3]) -> f64 {
    (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()
}

/// The 50 % crossing of a profile that runs from a lit end to a shadowed one, as a fraction of
/// the profile's own length. `None` when there is no edge to find (a flat face, or no drop).
fn crossing(profile: &[f64], n: usize) -> Option<f64> {
    let first = profile[0];
    let last = profile[n];
    let high = first.max(last);
    let low = first.min(last);
    if high <= 0.0 || high - low < 0.5 * high {
        return None;
    }
    let target = high - 0.5 * (high - low);
    // Oriented from the lit end, so the interpolation always runs down the drop.
    let mut oriented: Vec<(f64, f64)> = (0..=n)
        .map(|i| (i as f64 / n as f64, profile[i]))
        .collect();
    if first < last {
        oriented.reverse();
    }
    for i in 1..=n {
        let (s0, a) = oriented[i - 1];
        let (s1, b) = oriented[i];
        if a >= target && b <= target {
            let f = (a - target) / (a - b).max(1e-9);
            return Some(s0 + (s1 - s0) * f);
        }
    }
    None
}

/// A real terrain face on the shipped window, its own basis, and the field's edge on it.
struct Panel {
    origin: [f64; 3],
    u: [f64; 3],
    v: [f64; 3],
    lu: f64,
    lv: f64,
    cpu_s: f64,
    material: MaterialId,
    chunk: ChunkPos,
    quad: Quad,
}

impl Panel {
    fn point(&self, s: f64, t: f64) -> [f64; 3] {
        [
            self.origin[0] + self.u[0] * s + self.v[0] * t,
            self.origin[1] + self.u[1] * s + self.v[1] * t,
            self.origin[2] + self.u[2] * s + self.v[2] * t,
        ]
    }
}

/// **The scene the binary streams, with its own sun and range.** A window panel the field says a
/// shadow crosses is found, built with a clear corner light so the only shadow in the frame is
/// the map's, and its CPU edge — the game's own [`CellField`] — is compared with the rendered
/// edge. The range is `GAME_WINDOW`'s horizon, computed by [`shipped_shadow_range`].
fn shipped_case() -> Result<(), String> {
    let spec = WorldSpec::new(game::GAME_SEED, Config::default());
    let source = voxel_stream::WorldSource::new(spec);
    let registry = materials::registry().expect("the world's registry");
    let mesh_config = voxel_mesh::MeshConfig::from_registry(&registry).expect("a mesh config");
    let workers = std::thread::available_parallelism()
        .map(|n| n.get().saturating_sub(1).max(1))
        .unwrap_or(1);
    let mut streamer = voxel_work::Streamer::new(
        source,
        mesh_config,
        workers,
        game::GAME_CEILING_BYTES,
    )
    .and_then(|s| s.with_window(game::GAME_WINDOW))
    .expect("the streamer");
    let spawn = game::spawn_eye(&game::World::shipped());
    let centre = ChunkPos::new(
        (spawn[0] / f64::from(CHUNK_SIDE)).floor() as i32,
        (spawn[1] / f64::from(CHUNK_SIDE)).floor() as i32,
        (spawn[2] / f64::from(CHUNK_SIDE)).floor() as i32,
    );
    streamer.set_centre(centre);
    let settled = streamer.settle().expect("the shipped window settles");

    let tick = game::GAME_TICK;
    let epoch = game::GAME_NEW_MOON_TICK;
    let inputs = SkyInputs::default();
    let day = skylight::daylight_of(&spec, tick, inputs, epoch).expect("the shipped hour");
    let grade = day.grade;
    let field = RefCell::new(
        CellField::of(&spec, tick, inputs, epoch, None).expect("the game's field"),
    );
    let luminance = |bands: [f32; 6]| {
        0.2126 * bands[0] as f64 + 0.7152 * bands[1] as f64 + 0.0722 * bands[2] as f64
    };
    let ask = |point: [f64; 3]| -> Option<[f32; 6]> {
        let cell = SubvoxelPos::of_point(point)?;
        let mut field = field.borrow_mut();
        if let Some(bands) = field.at(cell) {
            return Some(bands);
        }
        field.measure(&streamer, cell).map(|(bands, _)| bands)
    };

    let displacement = voxel_shapers::materials::displacement_field().expect("a displacement field");
    let refine = voxel_mesh::RefineConfig::new(4, 2).expect("the bin's refinement");
    let mut panel: Option<Panel> = None;
    let mut candidates = 0usize;
    'scan: for mesh in streamer.meshes() {
        if (mesh.chunk.x - centre.x).abs() > 1 || (mesh.chunk.z - centre.z).abs() > 1 {
            continue;
        }
        let origin = mesh.origin_voxel();
        for quad in &mesh.quads {
            if quad.face != Face::PosY || displacement.entry(quad.material, quad.face).is_some() {
                continue;
            }
            let local = mesh.corner_positions_m(quad);
            let corners = local.map(|p| {
                [
                    origin[0] as f64 + p[0] as f64,
                    origin[1] as f64 + p[1] as f64,
                    origin[2] as f64 + p[2] as f64,
                ]
            });
            let u = sub(corners[1], corners[0]);
            let v = sub(corners[3], corners[0]);
            let (lu, lv) = (length(u), length(v));
            if !(1.5..=6.0).contains(&lu) || lv < 0.5 {
                continue;
            }
            candidates += 1;
            if candidates > 160 {
                break 'scan;
            }
            let n = 32usize;
            let mut profile = Vec::with_capacity(n + 1);
            let mut complete = true;
            for i in 0..=n {
                let s = i as f64 / n as f64;
                let point = [
                    corners[0][0] + u[0] * s + v[0] * 0.5,
                    corners[0][1] + u[1] * s + v[1] * 0.5,
                    corners[0][2] + u[2] * s + v[2] * 0.5,
                ];
                match ask(point) {
                    Some(bands) => profile.push(luminance(bands)),
                    None => {
                        complete = false;
                        break;
                    }
                }
            }
            if !complete {
                continue;
            }
            let Some(cpu_s) = crossing(&profile, n) else {
                continue;
            };
            panel = Some(Panel {
                origin: corners[0],
                u,
                v,
                lu,
                lv,
                cpu_s,
                material: quad.material,
                chunk: mesh.chunk,
                quad: *quad,
            });
            break 'scan;
        }
    }
    let Some(panel) = panel else {
        return Err(format!(
            "no shipped terrain panel with a shadow across it near the spawn after {candidates} \
             candidate(s)"
        ));
    };

    // **The frame's only shadow is the map's**: the corners are built clear, and the same sun the
    // field was measured under is the rig's.
    let clear = |_: [f64; 3]| Some([1.0f32, 1.0, 1.0, 0.0, 0.0, 0.0]);
    let scene = Scene::build_with_light_lattice(
        streamer.drawn_meshes().map(|(_, _, mesh)| mesh),
        spawn,
        &clear,
        Some(&displacement),
        Some(&refine),
        SHADOW_LATTICE,
    );
    let table = voxel_client::table(
        &registry,
        &materials::shading_field().expect("the world's shading field"),
    )
    .expect("a material table");
    let exposed: LightRig = skylight::exposed_at(&day.rig, grade);
    let rig = GpuRig::from_rig(&exposed).expect("the rig fits");

    let centre_m = panel.point(0.5, 0.5);
    let height = (panel.lv / 1.2).max(3.0);
    let mut camera = Camera::standard(
        [centre_m[0], centre_m[1] + height, centre_m[2]],
        2.0,
    );
    camera.pitch_rad = -MAX_PITCH_RAD;
    let probe = Probe::new(&scene, &rig, &table, camera, true);

    let n = 96usize;
    let mut rendered = vec![0.0f64; n + 1];
    for t in [0.25, 0.5, 0.75] {
        for (i, value) in rendered.iter_mut().enumerate() {
            *value += probe.at(panel.point(i as f64 / n as f64, t)) / 3.0;
        }
    }
    let Some(gpu_s) = crossing(&rendered, n) else {
        return Err(format!(
            "the shipped frame drew no shadow edge on the panel at ({:.1}, {:.1}, {:.1})",
            centre_m[0], centre_m[1], centre_m[2]
        ));
    };
    let cpu_m = panel.cpu_s * panel.lu;
    let gpu_m = gpu_s * panel.lu;
    // **The tolerance is the two instruments' own grains, added rather than chosen**: the field
    // answers per `SUBVOXEL_CELL_M` cell, one near-cascade texel projects
    // `2·NEAR_RANGE_M/SHADOW_RESOLUTION / tan(sun)` metres on the ground, the shader's single
    // 2 cm normal offset moves the edge `SHADOW_BIAS_M / tan(sun)`, and a pixel is 0.02 m. The
    // sun is the frame's own `day.sun_altitude_deg`, not a restated 10.6. A displacement larger
    // than their sum is not either instrument's grain — it is a shadow the field would call wrong.
    let elevation = day.sun_altitude_deg.to_radians();
    let cell_m = SUBVOXEL_CELL_M;
    let texel_m = 2.0 * f64::from(NEAR_RANGE_M) / f64::from(SHADOW_RESOLUTION);
    let texel_ground_m = texel_m / elevation.tan();
    let bias_ground_m = f64::from(SHADOW_BIAS_M) / elevation.tan();
    let tolerance = cell_m + texel_ground_m + bias_ground_m + 0.02;
    let panel_row = &table[panel.material.0 as usize];
    println!(
        "shipped: {} chunk(s), {:.1} MB, sun {:.1}° up at {:.0} m range; panel {:.2} x {:.2} m \
         at ({:.1}, {:.1}, {:.1}), material {} (transmission {:.2}, base {:.3})",
        settled.resident,
        settled.bytes as f64 / (1024.0 * 1024.0),
        day.sun_altitude_deg,
        shipped_shadow_range(),
        panel.lu,
        panel.lv,
        centre_m[0],
        centre_m[1],
        centre_m[2],
        panel.material.0,
        panel_row.rough_metal[3],
        panel_row.base_color_and_present[0]
    );
    println!(
        "shipped: field edge {cpu_m:.2} m, rendered edge {gpu_m:.2} m — {:.2} m apart; the bound \
         is field cell {cell_m:.2} + map texel {texel_ground_m:.2} + bias {bias_ground_m:.2} + \
         pixel 0.02 = {tolerance:.2} m",
        (gpu_m - cpu_m).abs()
    );
    if (gpu_m - cpu_m).abs() > tolerance {
        return Err(format!(
            "the shipped frame's shadow edge is {:.2} m from the field's ({gpu_m:.2} against \
             {cpu_m:.2}), more than the {tolerance:.2} m the field's cell, the map's texel and the \
             bias add up to",
            (gpu_m - cpu_m).abs()
        ));
    }

    // **Acne is invisible on the panel, so it is measured on open ground.** The panel is a
    // shadow crossing; a self-shadowed patch elsewhere in the window would never enter its
    // profile. Every solid +Y face more than 10 m from the panel is a candidate; the **open**
    // one with the largest area is the strip, and a grid across it is read from the one shipped
    // frame `strip_probe` draws. The expected value is the CPU witness's own `shade_cpu` for
    // that face — its material, the shader's pattern deviation at this address and footprint,
    // its +Y normal, this frame's rig — and the map-free column is the same face rendered alone
    // with the map off. Two private frames settle acne without the full scene's occluders: both
    // hold only this face and have no vertex light field, so the shader's rig branch draws the
    // rig's sun; with the map off it is the unshadowed beam, and with the map on any darkening
    // is the face shadowing itself. A tree or a panel in front of the full frame shows up in
    // `full/map-free` and is a cast shadow, not acne. No depth offset is re-added to make a
    // number look clean.
    struct Strip {
        point: [f64; 3],
        pixel: (u32, u32),
        material: MaterialId,
        field: f64,
        rendered: f64,
        expected: f64,
        unshadowed: f64,
        self_lit: f64,
        ambient: f64,
    }
    // The open, largest real +Y face is the ground the strip stands on. The window merges its
    // flat ground into one quad per chunk, so a face is a chunk-sized patch of the shipped
    // terrain rather than a 2 m fragment at a seam; the grid below samples across it.
    let mut best: Option<(Panel, f64)> = None;
    let mut candidates = 0usize;
    for mesh in streamer.meshes() {
        for quad in &mesh.quads {
            if quad.face != Face::PosY || displacement.entry(quad.material, quad.face).is_some() {
                continue;
            }
            // **Solid ground, not the sea.** The window's +Y faces are mostly the water
            // surface, whose transmissive branch is not the ground's shading at all; the strip
            // asks the land. A patterned row is allowed — the CPU below applies the same
            // pattern deviation the shader's row does.
            let row = &table[quad.material.0 as usize];
            if row.rough_metal[3] > 0.0 || row.base_color_and_present[3] < 0.5 {
                continue;
            }
            let local = mesh.corner_positions_m(quad);
            let origin = mesh.origin_voxel();
            let corners = local.map(|p| {
                [
                    origin[0] as f64 + p[0] as f64,
                    origin[1] as f64 + p[1] as f64,
                    origin[2] as f64 + p[2] as f64,
                ]
            });
            let u = sub(corners[1], corners[0]);
            let v = sub(corners[3], corners[0]);
            let (lu, lv) = (length(u), length(v));
            if lu.min(lv) < 1.0 {
                continue;
            }
            let point = [
                0.25 * (corners[0][0] + corners[1][0] + corners[2][0] + corners[3][0]),
                0.25 * (corners[0][1] + corners[1][1] + corners[2][1] + corners[3][1]),
                0.25 * (corners[0][2] + corners[1][2] + corners[2][2] + corners[3][2]),
            ];
            let (dx, dz) = (point[0] - centre_m[0], point[2] - centre_m[2]);
            if dx * dx + dz * dz < 100.0 {
                continue;
            }
            candidates += 1;
            if candidates > 400 {
                break;
            }
            let Some(bands) = ask(point) else {
                continue;
            };
            let field = luminance(bands);
            let area = lu * lv;
            // Open first, then large: a chunk-sized face under a canopy is not the ground the
            // strip is for, and the field is the same witness that says which face is lit.
            if best
                .as_ref()
                .map_or(true, |(p, brightest)| (field, area) > (*brightest, p.lu * p.lv))
            {
                best = Some((
                    Panel {
                        origin: corners[0],
                        u,
                        v,
                        lu,
                        lv,
                        cpu_s: 0.0,
                        material: quad.material,
                        chunk: mesh.chunk,
                        quad: *quad,
                    },
                    field,
                ));
            }
        }
    }
    let Some((face, _face_field)) = best else {
        return Err(format!(
            "no +Y face at least 10 m from the panel among {candidates} candidate(s), so the \
             shipped window's open ground was not measured"
        ));
    };
    let face_centre = face.point(0.5, 0.5);
    let face_distance = {
        let (dx, dz) = (face_centre[0] - centre_m[0], face_centre[2] - centre_m[2]);
        (dx * dx + dz * dz).sqrt()
    };
    // The camera stands directly over the face, high enough that the whole face is inside the
    // vertical field of view, with the field of view read rather than assumed.
    let span = face.lu.max(face.lv);
    let mut strip_camera = Camera::standard([face_centre[0], face_centre[1], face_centre[2]], 2.0);
    let camera_height = span / (2.0 * (0.5 * strip_camera.fov_y_rad).tan()) + 2.0;
    strip_camera.position_m = [face_centre[0], face_centre[1] + camera_height, face_centre[2]];
    strip_camera.pitch_rad = -MAX_PITCH_RAD;
    let strip_probe = Probe::new(&scene, &rig, &table, strip_camera, true);

    // **The map's own answer, with the beam kept.** A private scene of this one face at the
    // same world place, with **no light field** (`None`), so the shader takes its rig branch:
    // with shadows off the frame is the rig's sun with visibility 1, not a vertex-light
    // fallback. The material and the law are the full scene's; the only difference is whether
    // the map multiplies the beam, so a deficit is the map and not the shading model's own
    // agreement with the CPU (which is the `expected` column).
    let face_mesh = Mesh {
        chunk: face.chunk,
        origin: {
            let o = face.chunk.origin();
            [o.x, o.y, o.z]
        },
        quads: vec![face.quad],
        loops: Vec::new(),
        stats: MeshStats::default(),
    };
    let face_scene = Scene::build_with_light(
        std::iter::once(&face_mesh),
        face_centre,
        &|_: [f64; 3]| None,
        None,
        None,
    );
    let unshadowed = Probe::new(&face_scene, &rig, &table, strip_camera, false);
    // **The face's own map.** The scene holds this one face, so the only thing its map can cast
    // on it is itself: a darkening here is self-shadowing (acne), not a tree or the panel. This
    // is the map-side raster depth bias on its own, which is the question.
    let self_shadowed = Probe::new(&face_scene, &rig, &table, strip_camera, true);
    // The same face under the same ambient with **no light at all**: everything but the beam.
    // The difference is the sun's own share of this ground, so the floor below is the loss one
    // dark tap in the map's nine would cost — a derived number, not a chosen one.
    let mut no_lights = rig;
    no_lights.count[0] = 0;
    let ambient_only = Probe::new(&face_scene, &no_lights, &table, strip_camera, false);

    // A grid across the face, inset from its edges: a strip of the shipped ground, not one
    // pixel standing for it. `open_level` is the brightest field reading in the grid, and a
    // point is kept only when its own field reaches 90 % of it — the map is then the only
    // thing that can darken it.
    let steps = 5usize;
    let mut points: Vec<([f64; 3], f64)> = Vec::new();
    let mut open_level = f64::MIN;
    for i in 0..steps {
        for j in 0..steps {
            let s = 0.1 + 0.8 * i as f64 / (steps - 1) as f64;
            let t = 0.1 + 0.8 * j as f64 / (steps - 1) as f64;
            let point = face.point(s, t);
            let Some(bands) = ask(point) else {
                continue;
            };
            let field = luminance(bands);
            open_level = open_level.max(field);
            points.push((point, field));
        }
    }
    // The CPU's own law, including the material's pattern: the shader's row carries the
    // pattern's gains, so the base inputs alone are a different surface on a patterned row.
    let shading_field = materials::shading_field().expect("the world's shading field");
    let lattice = DetailLattice::new(shading_field.domain());
    let pattern = Pattern::new(&lattice, &shading_field);
    let surface = materials::pattern_of(face.material);
    let pixel_angle = f64::from(pixel_angle_rad(strip_camera.fov_y_rad, strip_probe.height));
    if open_level <= 0.0 {
        return Err(format!(
            "the shipped window's largest open land face at ({:.1}, {:.1}, {:.1}) is in shadow \
             at every one of its {} grid points; the CPU field never calls it open, so it is not \
             the open-ground witness",
            face_centre[0], face_centre[1], face_centre[2], points.len()
        ));
    }
    let mut strip: Vec<Strip> = Vec::new();
    for (point, field) in points {
        if field < 0.9 * open_level {
            continue;
        }
        let Some(pixel) = strip_probe.visible(point) else {
            continue;
        };
        let Some(rendered) = strip_probe
            .frame
            .pixel(pixel.0, pixel.1)
            .filter(|p| Frame::is_geometry(*p))
            .map(voxel_client::luminance)
        else {
            continue;
        };
        let Some(unshadowed_pixel) = unshadowed
            .frame
            .pixel(pixel.0, pixel.1)
            .filter(|p| Frame::is_geometry(*p))
            .map(voxel_client::luminance)
        else {
            continue;
        };
        let Some(self_lit) = self_shadowed
            .frame
            .pixel(pixel.0, pixel.1)
            .filter(|p| Frame::is_geometry(*p))
            .map(voxel_client::luminance)
        else {
            continue;
        };
        let Some(ambient_pixel) = ambient_only
            .frame
            .pixel(pixel.0, pixel.1)
            .filter(|p| Frame::is_geometry(*p))
            .map(voxel_client::luminance)
        else {
            continue;
        };
        let (from_eye, distance) = {
            let d = sub(strip_camera.position_m, point);
            (d, length(d).max(1e-9))
        };
        let view = Vec3::new(from_eye[0] / distance, from_eye[1] / distance, from_eye[2] / distance);
        let inputs = match shading::inputs_for(&registry, face.material) {
            Ok(inputs) => inputs,
            Err(_) => continue,
        };
        // The fragment's own pattern deviation, from the same address and footprint the shader
        // uses. A row with no declared gain returns `DetailSample::NONE`, so this is the
        // identity on an unpatterned material rather than a second branch.
        let facing = (point[1] - strip_camera.position_m[1]) / distance;
        let footprint = eye_footprint_m(distance, pixel_angle);
        let Ok(deviation) =
            pattern.at(face.material, point, footprint, surface)
        else {
            continue;
        };
        let Ok(varied) = inputs.with_detail(deviation) else {
            continue;
        };
        // The face's own open fraction, from the row the GPU shades: a face that is part hole
        // receives that much less sky, and the CPU is asked the same question.
        let open = f64::from(table[face.material.0 as usize].emission_rgb_open[3]);
        let Ok((want, _)) = shading::shade_cpu(
            &varied,
            Vec3::new(0.0, 1.0, 0.0),
            view,
            open,
            &exposed,
        ) else {
            continue;
        };
        let expected = want.luminance();
        if rendered <= 0.0 || expected <= 0.0 {
            continue;
        }
        strip.push(Strip {
            point,
            pixel,
            material: face.material,
            field,
            rendered,
            expected,
            unshadowed: unshadowed_pixel,
            self_lit,
            ambient: ambient_pixel,
        });
    }
    if strip.len() < 8 {
        return Err(format!(
            "the shipped strip on the {:.1} x {:.1} m face at ({:.1}, {:.1}, {:.1}) kept only {} \
             open point(s), fewer than the eight the strip needs",
            face.lu, face.lv, face_centre[0], face_centre[1], face_centre[2], strip.len()
        ));
    }
    let mut worst = (f64::INFINITY, [0.0f64; 3], 0.0f64, 0.0f64);
    // (map ratio, point, one tap's share of this ground, the floor a full tap falls under)
    let mut worst_map = (f64::INFINITY, [0.0f64; 3], 0.0f64, 0.0f64);
    for s in &strip {
        let ratio = s.rendered / s.expected;
        if ratio < worst.0 {
            worst = (ratio, s.point, s.rendered, s.expected);
        }
        // **One dark tap in nine**, as a share of this pixel: the map averages nine comparisons,
        // so a self-shadowed tap removes one ninth of the beam's own contribution. The beam's
        // contribution is `unshadowed - ambient`, measured on the same face in the same frame.
        let sun_share = ((s.unshadowed - s.ambient) / s.unshadowed).clamp(0.0, 1.0);
        let one_tap = sun_share / 9.0;
        // **The face's own map**, not the full frame's: the full frame can put a tree in front
        // of the pixel, which is a cast shadow and a different question. A full dark tap costs
        // `one_tap`; 99 % of it is the floor, the last 1 % being the two private frames' own
        // repeatability (their ratio agrees to 4e-4).
        let map = s.self_lit / s.unshadowed;
        if map < worst_map.0 {
            worst_map = (map, s.point, one_tap, 1.0 - 0.99 * one_tap);
        }
    }
    println!(
        "shipped strip: {:.1} x {:.1} m +Y face at ({:+.1}, {:+.1}, {:+.1}), {face_distance:.1} m \
         from the panel; {} open point(s), open level {open_level:.4}, camera {camera_height:.1} m \
         up; rendered/CPU min {:.4} (mean {:.4}), full/map-free min {:.4} (mean {:.4}); the \
         face's own map min {:.4} (mean {:.4}) against a floor of 99 % of one dark tap in nine, \
         {:.4}",
        face.lu,
        face.lv,
        face_centre[0],
        face_centre[1],
        face_centre[2],
        strip.len(),
        worst.0,
        strip.iter().map(|s| s.rendered / s.expected).sum::<f64>() / strip.len() as f64,
        strip.iter().map(|s| s.rendered / s.unshadowed).fold(f64::INFINITY, f64::min),
        strip.iter().map(|s| s.rendered / s.unshadowed).sum::<f64>() / strip.len() as f64,
        worst_map.0,
        strip.iter().map(|s| s.self_lit / s.unshadowed).sum::<f64>() / strip.len() as f64,
        worst_map.3
    );
    for s in &strip {
        let sun_share = ((s.unshadowed - s.ambient) / s.unshadowed).clamp(0.0, 1.0);
        println!(
            "shipped strip ({:+.1}, {:+.1}, {:+.1}) px ({}, {}), material {}: field {:.4} \
             ({:.0}% of open), rendered {:.4}, CPU expected {:.4}, ratio {:.4}, map-free {:.4}, \
             full/map-free {:.4}, self {:.4}, self/map-free {:.4}, sun share {:.4}, one tap {:.4}",
            s.point[0],
            s.point[1],
            s.point[2],
            s.pixel.0,
            s.pixel.1,
            s.material.0,
            s.field,
            100.0 * s.field / open_level.max(f64::MIN_POSITIVE),
            s.rendered,
            s.expected,
            s.rendered / s.expected,
            s.unshadowed,
            s.rendered / s.unshadowed,
            s.self_lit,
            s.self_lit / s.unshadowed,
            sun_share,
            sun_share / 9.0
        );
    }
    if worst_map.0 < worst_map.3 {
        return Err(format!(
            "the shipped map self-shadows its own open face at ({:.1}, {:.1}, {:.1}): the face \
             alone with the map on draws {:.4} of the same face with it off, under the {:.4} \
             floor — one dark tap in nine would cost {:.4} of the beam, so the map removed more \
             than a single sample's share",
            worst_map.1[0],
            worst_map.1[1],
            worst_map.1[2],
            worst_map.0,
            worst_map.3,
            worst_map.2
        ));
    }
    Ok(())
}
