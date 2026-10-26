//! **The shadow's own resolution, as a picture** — `cargo run --offline --release -p
//! voxel-client --example shadow-lattice` writes `target/shadow-coarse.png` and
//! `target/shadow-fine.png`.
//!
//! Two frames of the **same** one-square-metre face, drawn by the game's own `WorldPipeline` under
//! the game's own sky: the only difference is how many times the face asks the shadow for an
//! answer. `build_with_light` asks once per corner — four samples a metre apart, however large the
//! rectangle — and `build_with_light_lattice` asks at [`SHADOW_LATTICE`], one sample per stored
//! subvoxel, a quarter of a metre.
//!
//! The field is synthetic on purpose and says so: a shadow edge at **0.5 m**, in the middle of the
//! face. That is the shape a once-per-corner face cannot draw — its samples sit on the two ends of
//! the edge and the fragment's blend spans the whole metre — and it is the shape the stored field
//! *can* answer, because `CellField` holds one value per `SUBVOXEL_SIDE = 4` cells. A finer lattice
//! would ask the same cell twice; this one asks each cell once.
//!
//! The pixels are read twice per lattice: once with the half-shadow and once with the face fully
//! lit. The difference is **the shadow's own footprint** — the material's pattern and the rig's
//! shading cancel, so what is left is the light the face was built with, as the screen saw it.
//! The width of its shoulder is the number this example exists to print.
//!
//! # And then the same measurement on the shipped world
//!
//! A synthetic square metre is a clean witness and not the shipped frame (the shipped-view audit).
//! So the example also streams the **game's own window** (`GAME_WINDOW`, `GAME_SEED`, the spawn
//! eye), finds a real terrain panel whose corner light differs, measures the game's own `CellField`
//! at the panel's lattice, and draws the panel through the **same `ResidentScene` the binary
//! runs** at `SHADOW_LATTICE` and at lattice `1` — the identical view the coarse path would have
//! drawn. The difference frame is the shadow's own footprint again, and the shoulder is the
//! number: pixels and metres on real terrain, against the synthetic face's own.
//!
//! **What is measured, and what is only bound.** The light on the measured panel is the game's
//! field, marched cell by cell through the real world. The rest of the window falls back to the
//! rig (`None`) wherever the field holds no cell, because measuring every cell of a 605-chunk
//! window is the binary's worker and not this example's; the panel's own pixels are the subject,
//! and the fallback is stated rather than hidden. The rebuild cost printed beside it is the whole
//! window's — the median of five syncs — with the field asks the lattice spends against lattice 1;
//! a chunk is the unit the binary's own budget lands in, so the per-chunk line is the number a
//! frame pays.

use std::cell::Cell;

use voxel_appearance::sky::SkyInputs;
use voxel_client::shading::GpuRig;
use voxel_client::{
    offscreen_device_for, skylight, Camera, Scene, WorldPipeline, DEPTH_FORMAT, SHADOW_LATTICE,
};
use voxel_mesh::{Mesh, MeshStats, Quad};
use voxel_model::{Face, MaterialId};
use voxel_shapers::{Config, WorldSpec};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;
const WIDTH: u32 = 512;
const HEIGHT: u32 = 288;
/// The face's material: the same id the scene's own light tests use.
const ROCK: MaterialId = MaterialId(6);

fn main() {
    // ---- the sky the face is drawn under: the first day's own highest Sun -----------------
    let spec = WorldSpec::new(voxel_client::GAME_SEED, Config::default());
    let inputs = SkyInputs::default();
    let day = voxel_shapers::climate::TICKS_PER_DAY;
    let tick = (0..day)
        .max_by(|a, b| {
            voxel_shapers::sky::solar_position(&spec, *a)
                .altitude_deg
                .total_cmp(&voxel_shapers::sky::solar_position(&spec, *b).altitude_deg)
        })
        .expect("a tick in the first day");
    let daylight =
        skylight::daylight_of(&spec, tick, inputs, 0).expect("an atmosphere at the highest Sun");
    let grade = daylight.grade;
    println!(
        "sun: tick {tick}, {:.1} deg up at azimuth {:.1} deg",
        daylight.sun_altitude_deg, daylight.sky.sun_azimuth_deg
    );

    // ---- one face, one metre square, its outward face up ---------------------------------
    let mesh = Mesh {
        chunk: voxel_model::ChunkPos::new(0, 0, 0),
        origin: [0, 0, 0],
        quads: vec![Quad {
            face: Face::PosY,
            slice: 4,
            uv: [0, 0],
            extent: [4, 4],
            material: ROCK,
        }],
        loops: Vec::new(),
        stats: MeshStats { solid_voxels: 1, visible_faces: 1, quads: 1, loops: 0 },
    };
    // **The shadow edge is at 0.5 m**, half way across the face, and it is a subvoxel edge: the
    // field stores it at 0.25 m and nothing in the light data is coarser than that.
    let shadow = |point: [f64; 3]| {
        let light = if point[0] < 0.5 { 1.0 } else { 0.0 };
        Some([light, light, light, 0.0, 0.0, 0.0])
    };
    let clear = |_: [f64; 3]| Some([1.0f32, 1.0, 1.0, 0.0, 0.0, 0.0]);
    let eye = [0.5, 2.0, 0.5];

    let asks = Cell::new(0u64);
    let counted = |point: [f64; 3]| {
        asks.set(asks.get() + 1);
        shadow(point)
    };
    let build = |lattice: u32, field: &voxel_client::LightAt<'_>, count: &Cell<u64>| {
        let started = std::time::Instant::now();
        let scene = Scene::build_with_light_lattice([&mesh], eye, field, None, None, lattice);
        (scene, started.elapsed(), count.replace(0))
    };
    let (coarse, coarse_build, coarse_asks) = build(1, &counted, &asks);
    let coarse_clear = Scene::build_with_light([&mesh], eye, &clear, None, None);
    let (fine, fine_build, fine_asks) = build(SHADOW_LATTICE, &counted, &asks);
    let fine_clear =
        Scene::build_with_light_lattice([&mesh], eye, &clear, None, None, SHADOW_LATTICE);
    println!(
        "face: {} corners and {coarse_asks} ask(s) in {:?} ({} B) against {} corners and \
         {fine_asks} ask(s) in {:?} ({} B)",
        coarse.vertices.len(),
        coarse_build,
        coarse.vertices.len() * voxel_client::Vertex::STRIDE as usize,
        fine.vertices.len(),
        fine_build,
        fine.vertices.len() * voxel_client::Vertex::STRIDE as usize,
    );
    println!(
        "cost: {:.1}x the vertices, {:.1}x the samples, {:.2} us an ask, at {:.0} asks and {:.0} \
         corners a square metre",
        fine.vertices.len() as f64 / coarse.vertices.len() as f64,
        fine_asks as f64 / coarse_asks as f64,
        1e6 * fine_build.as_secs_f64() / fine_asks.max(1) as f64,
        fine_asks as f64,
        fine.vertices.len() as f64 / 4.0,
    );

    // ---- the game's own pipeline, and the game's own sky ----------------------------------
    let (device, queue, adapter, _samples) = offscreen_device_for(FORMAT).expect("a GPU");
    let samples = 1;
    println!("adapter: {adapter}, {FORMAT:?} at {samples} sample(s)");
    let registry = voxel_shapers::materials::registry().expect("the registry");
    let field_table = voxel_shapers::materials::shading_field().expect("the shading field");
    let materials = voxel_client::table(&registry, &field_table).expect("every material derives");
    let gpu_rig = GpuRig::from_rig(&skylight::exposed_at(&daylight.rig, grade))
        .expect("the rig fits");
    let mut world = WorldPipeline::new(&device, &queue, FORMAT, &materials, &gpu_rig, samples)
        .expect("the world pipeline");
    let peak = skylight::peak_sun_luminance(&spec, inputs);
    let air = voxel_client::sun::air_at(&spec, eye[0], eye[2], tick);
    world.write_sky_model(&skylight::SkyModel::of(
        &daylight.sky,
        &air,
        peak,
        &voxel_shapers::sky::moon_position(&spec, tick, 0),
    ));

    let colour = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("colour"),
        size: wgpu::Extent3d { width: WIDTH, height: HEIGHT, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("depth"),
        size: wgpu::Extent3d { width: WIDTH, height: HEIGHT, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: samples,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let colour_view = colour.create_view(&wgpu::TextureViewDescriptor::default());
    let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());

    // Straight down at the face from one metre above it: the metre is about 200 px across, so the
    // quarter-metre edge the stored field can resolve is about 50 px and the metre it replaces is
    // about 200.
    let mut camera = Camera::standard(eye, WIDTH as f64 / HEIGHT as f64);
    camera.pitch_rad = -voxel_client::camera::MAX_PITCH_RAD;
    let pixel_angle = voxel_client::pixel_angle_rad(camera.fov_y_rad, HEIGHT);
    let mut render = |scene: &Scene, label: &'static str| -> Vec<u8> {
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(label) });
        world.draw(&mut encoder, &colour_view, &depth_view, None, scene, &camera, pixel_angle);
        queue.submit(Some(encoder.finish()));
        read_back(&device, &queue, &colour)
    };
    let coarse_pixels = render(&coarse, "coarse");
    let coarse_clear_pixels = render(&coarse_clear, "coarse-clear");
    let fine_pixels = render(&fine, "fine");
    let fine_clear_pixels = render(&fine_clear, "fine-clear");
    write_png("shadow-coarse.png", &coarse_pixels);
    write_png("shadow-fine.png", &fine_pixels);

    // ---- what the pixels say: how wide the lit-to-dark edge actually is --------------------
    let coarse_edge = shadow_edge(&coarse_clear_pixels, &coarse_pixels);
    let fine_edge = shadow_edge(&fine_clear_pixels, &fine_pixels);
    for (label, edge) in [("coarse", &coarse_edge), ("fine", &fine_edge)] {
        println!(
            "{label}: the shadow's own footprint fades 90 % to 10 % over {:.1} px = {:.3} m of \
             the face (the face is {:.1} px across), across {:.0} level(s) of grey",
            edge.width_px, edge.width_m, edge.face_px, edge.drop,
        );
    }
    let (a, _) = coarse_pixels.as_chunks::<4>();
    let (b, _) = fine_pixels.as_chunks::<4>();
    let differing = a
        .iter()
        .zip(b)
        .filter(|(coarse, fine)| coarse[..3] != fine[..3])
        .count();
    println!(
        "frames: {differing} of {} pixel(s) differ ({:.1} %); wrote target/shadow-coarse.png and \
         target/shadow-fine.png",
        a.len(),
        100.0 * differing as f64 / a.len().max(1) as f64
    );

    // **The claim, asserted rather than only printed**: the same face, the same shadow, and the
    // fine frame's edge is a fraction of the coarse one's — and below the metre the coarse path
    // could not go under.
    assert!(
        fine_edge.width_m * 2.0 < coarse_edge.width_m,
        "the fine edge is {:.3} m against the coarse {:.3} m: not a finer shadow",
        fine_edge.width_m,
        coarse_edge.width_m,
    );
    assert!(
        fine_edge.width_m < 0.5,
        "the fine edge is {:.3} m: not below the voxel it was meant to resolve",
        fine_edge.width_m
    );

    // ---- and the same measurement on the world the binary draws --------------------------
    shipped(
        &mut world, &device, &queue, &colour, &colour_view, &depth_view, peak, inputs,
    );
}

/// **The shipped scene's own shadow edge**: the game's window, the game's field, the game's own
/// `ResidentScene`, and the same 90 → 10 % shoulder the synthetic face above prints.
#[allow(clippy::too_many_arguments)]
fn shipped(
    world: &mut WorldPipeline,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    colour: &wgpu::Texture,
    colour_view: &wgpu::TextureView,
    depth_view: &wgpu::TextureView,
    peak: f64,
    inputs: SkyInputs,
) {
    use voxel_client::game;
    use voxel_client::ResidentScene;
    use voxel_model::ChunkPos;

    // ---- the world the binary streams ------------------------------------------------
    let spec = WorldSpec::new(game::GAME_SEED, Config::default());
    let source = voxel_stream::WorldSource::new(spec);
    let registry = voxel_shapers::materials::registry().expect("the registry");
    let mesh_config = voxel_mesh::MeshConfig::from_registry(&registry).expect("a mesh config");
    let workers = std::thread::available_parallelism()
        .map(|n| n.get().saturating_sub(1).max(1))
        .unwrap_or(1);
    let mut streamer = voxel_work::Streamer::new(
        source.clone(),
        mesh_config,
        workers,
        game::GAME_CEILING_BYTES,
    )
    .and_then(|s| s.with_window(game::GAME_WINDOW))
    .expect("the streamer");
    let spawn = game::spawn_eye(&game::World::shipped());
    let centre = ChunkPos::new(
        (spawn[0] / 32.0).floor() as i32,
        (spawn[1] / 32.0).floor() as i32,
        (spawn[2] / 32.0).floor() as i32,
    );
    streamer.set_centre(centre);
    let settled = streamer.settle().expect("the shipped window settles");
    println!(
        "\nshipped: {} of {} chunk(s), {:.1} MB, window {:?} at tick {}",
        settled.resident,
        settled.wanted,
        settled.bytes as f64 / (1024.0 * 1024.0),
        game::GAME_WINDOW,
        game::GAME_TICK
    );

    // ---- the game's own hour and field -----------------------------------------------
    let tick = game::GAME_TICK;
    let epoch = game::GAME_NEW_MOON_TICK;
    let day = skylight::daylight_of(&spec, tick, inputs, epoch).expect("the shipped hour");
    let grade = day.grade;
    let field = std::cell::RefCell::new(
        voxel_client::field::CellField::of(&spec, tick, inputs, epoch, None).expect("a field"),
    );
    let marches = Cell::new(0u64);
    let asks = Cell::new(0u64);
    let measure = |point: [f64; 3]| -> Option<[f32; 6]> {
        asks.set(asks.get() + 1);
        let cell = voxel_client::field::SubvoxelPos::of_point(point)?;
        let mut field = field.borrow_mut();
        if let Some(bands) = field.at(cell) {
            return Some(skylight::exposed_bands(bands, grade));
        }
        marches.set(marches.get() + 1);
        let (bands, _) = field.measure(&streamer, cell)?;
        Some(skylight::exposed_bands(bands, grade))
    };
    println!(
        "shipped: sun {:.1} deg up, grade {:.3}; the field is marched on this thread",
        day.sun_altitude_deg, grade
    );

    // ---- a real terrain panel with a light edge on it -------------------------------
    let displacement =
        voxel_shapers::materials::displacement_field().expect("the world's displacement field");
    // The bin's own refinement (`shploingus/src/main.rs:323`, `:331`): printed here so a face the
    // refinement would bend is not mistaken for one the lattice cuts.
    let refine = voxel_mesh::RefineConfig::new(4, 2).expect("the bin's refinement");
    let lattice = SHADOW_LATTICE as f64;
    let scan_started = std::time::Instant::now();
    let mut panel: Option<Panel> = None;
    let mut candidates = 0usize;
    'scan: for mesh in streamer.meshes() {
        // The eye's own neighbourhood: a shadow edge is looked for where the player stands, and
        // the field's march from there is short enough to stay inside the settled window.
        if (mesh.chunk.x - centre.x).abs() > 2 || (mesh.chunk.z - centre.z).abs() > 2 {
            continue;
        }
        let origin = mesh.origin_voxel();
        for quad in &mesh.quads {
            if quad.face != Face::PosY {
                continue;
            }
            // A bent face leaves for the vertex path and never sees the lattice.
            if displacement.entry(quad.material, quad.face).is_some() {
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
            if candidates > 600 {
                break 'scan;
            }
            // The face's four corners: all measured, and not all the same answer.
            let mut corner = [[0.0f32; 6]; 4];
            let mut all = true;
            for (i, c) in corners.iter().enumerate() {
                match measure(*c) {
                    Some(bands) => corner[i] = bands,
                    None => {
                        all = false;
                        break;
                    }
                }
            }
            if !all || corner.iter().all(|bands| bands == &corner[0]) {
                continue;
            }
            let nu = (lu * lattice).round().max(1.0) as usize;
            let nv = (lv * lattice).round().max(1.0) as usize;
            // Three rows across the face: the edge must run along one axis, or the average
            // down the columns smears two edges into one wide one.
            let mut rows: Vec<Vec<f64>> = Vec::new();
            for t in [0.0, 0.5, 1.0] {
                let mut row = Vec::with_capacity(nu + 1);
                for i in 0..=nu {
                    let s = i as f64 / nu as f64;
                    let point = [
                        corners[0][0] + u[0] * s + v[0] * t,
                        corners[0][1] + u[1] * s + v[1] * t,
                        corners[0][2] + u[2] * s + v[2] * t,
                    ];
                    match measure(point) {
                        Some(bands) => row.push(bands_luminance(bands)),
                        None => {
                            all = false;
                            break;
                        }
                    }
                }
                if !all {
                    break;
                }
                rows.push(row);
            }
            if !all {
                continue;
            }
            let (mut low, mut high) = (f64::MAX, f64::MIN);
            for row in &rows {
                for value in row {
                    low = low.min(*value);
                    high = high.max(*value);
                }
            }
            let drop = |row: &Vec<f64>| (row[0] - row[nu]).abs();
            let min_drop = rows.iter().map(drop).fold(f64::INFINITY, f64::min);
            // **A shadow, not a gradient.** A face the sun grazes varies by a few per cent from
            // one end to the other; a face a shadow crosses loses most of its beam at the edge.
            // Half the face's own brightness is the line between the two, and a gradient that
            // passes it is a measurement of nothing dressed as an edge.
            if high <= 0.0 || min_drop < 0.5 * high {
                continue;
            }
            let smear = (0..=nu)
                .map(|i| {
                    let (a, b, c) = (rows[0][i], rows[1][i], rows[2][i]);
                    a.max(b).max(c) - a.min(b).min(c)
                })
                .fold(0.0, f64::max);
            if smear > 0.35 * min_drop {
                continue;
            }
            // The face is the subject: measure its whole lattice once, so the field holds every
            // cell the two scenes below will ask for and no render pays for a march. The grid is
            // not kept here — the panel is drawn through the field, and the field is the record.
            let mut full = corner[0];
            for j in 0..=nv {
                for i in 0..=nu {
                    let (s, t) = (i as f64 / nu as f64, j as f64 / nv as f64);
                    let point = [
                        corners[0][0] + u[0] * s + v[0] * t,
                        corners[0][1] + u[1] * s + v[1] * t,
                        corners[0][2] + u[2] * s + v[2] * t,
                    ];
                    match measure(point) {
                        Some(bands) => {
                            if bands_luminance(bands) > bands_luminance(full) {
                                full = bands;
                            }
                        }
                        None => {
                            all = false;
                            break;
                        }
                    }
                }
                if !all {
                    break;
                }
            }
            if !all {
                continue;
            }
            panel = Some(Panel {
                origin: corners[0],
                u,
                v,
                lu,
                lv,
                nu,
                nv,
                full,
            });
            break 'scan;
        }
    }
    let panel = panel.unwrap_or_else(|| {
        panic!(
            "no shipped terrain panel with a one-axis light edge in {candidates} candidate(s) \
             near the spawn: {} ask(s), {} march(es)",
            asks.get(),
            marches.get()
        )
    });
    println!(
        "shipped: panel {:.2} x {:.2} m at ({:.1}, {:.1}, {:.1}) — {} x {} lattice sample(s); \
         {} ask(s), {} march(es), {:.2} s",
        panel.lu,
        panel.lv,
        panel.origin[0] + panel.u[0] * 0.5 + panel.v[0] * 0.5,
        panel.origin[1] + panel.u[1] * 0.5 + panel.v[1] * 0.5,
        panel.origin[2] + panel.u[2] * 0.5 + panel.v[2] * 0.5,
        panel.nu,
        panel.nv,
        asks.get(),
        marches.get(),
        scan_started.elapsed().as_secs_f64()
    );

    // ---- one camera, three scenes ----------------------------------------------------
    let centre_m = panel.point(0.5, 0.5);
    let eye = [centre_m[0], centre_m[1] + 3.0, centre_m[2]];
    let mut camera = Camera::standard(eye, WIDTH as f64 / HEIGHT as f64);
    camera.pitch_rad = -voxel_client::camera::MAX_PITCH_RAD;
    let pixel_angle = voxel_client::pixel_angle_rad(camera.fov_y_rad, HEIGHT);

    let gpu = GpuRig::from_rig(&skylight::exposed_at(&day.rig, grade)).expect("the rig fits");
    let _ = world.write_rig(&gpu);
    let air = voxel_client::sun::air_at(&spec, centre_m[0], centre_m[2], tick);
    world.write_sky_model(&skylight::SkyModel::of(
        &day.sky,
        &air,
        peak,
        &voxel_shapers::sky::moon_position(&spec, tick, epoch),
    ));

    let panel = &panel;
    // **The frame's own ask**, not the scan's: a cell from the point, the field's answer if it
    // holds one, and the rig where it does not. This is what `shipped_frame` calls once per
    // corner of every lit face, so the build times below are the ones the game pays.
    let build_asks = Cell::new(0u64);
    let lookup = |point: [f64; 3]| -> Option<[f32; 6]> {
        build_asks.set(build_asks.get() + 1);
        let cell = voxel_client::field::SubvoxelPos::of_point(point)?;
        let bands = field.borrow_mut().at(cell)?;
        Some(skylight::exposed_bands(bands, grade))
    };
    let clear = |_: [f64; 3]| Some(panel.full);
    let build = |lattice: u32,
                     light: &dyn Fn([f64; 3]) -> Option<[f32; 6]>|
                     -> (Scene, f64, usize, usize) {
        let started = std::time::Instant::now();
        let mut resident = ResidentScene::new();
        resident.set_shadow_lattice(lattice);
        resident.sync(
            streamer.drawn_meshes(),
            centre,
            Some(light),
            Some(&displacement),
            Some(&refine),
            usize::MAX,
            1,
        );
        let elapsed = started.elapsed().as_secs_f64();
        let (corners, indices) = (resident.corners(), resident.indices());
        (resident.take(), elapsed, corners, indices)
    };
    let mut shoot = |scene: &Scene, label: &'static str| -> Vec<u8> {
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(label) });
        world.draw(&mut encoder, colour_view, depth_view, None, scene, &camera, pixel_angle);
        queue.submit(Some(encoder.finish()));
        read_back(device, queue, colour)
    };

    // **Five builds, the median reported**, because one wall-clock sample on a shared machine is
    // a coin toss: the same lattice measured 151 ms and 201 ms in two consecutive runs here. The
    // first build is dropped — it faults in the eleven megabytes of vertices it allocates, and
    // that fault is the allocator's, not the sampling's.
    let median_build = |lattice: u32| {
        let _ = build(lattice, &lookup);
        let mut times = [0.0f64; 5];
        let mut asks = 0u64;
        let mut last = None;
        for slot in times.iter_mut() {
            build_asks.set(0);
            let (scene, took, corners, indices) = build(lattice, &lookup);
            *slot = took;
            asks = build_asks.get();
            last = Some((scene, corners, indices));
        }
        times.sort_by(f64::total_cmp);
        let (scene, corners, indices) = last.expect("five builds");
        (scene, times[2], times[0], times[4], corners, indices, asks)
    };
    let (fine, fine_s, fine_lo, fine_hi, fine_corners, fine_indices, fine_asks) =
        median_build(SHADOW_LATTICE);
    let fine_pixels = shoot(&fine, "shipped-fine");
    drop(fine);
    let (coarse, coarse_s, coarse_lo, coarse_hi, coarse_corners, coarse_indices, coarse_asks) =
        median_build(1);
    let coarse_pixels = shoot(&coarse, "shipped-coarse");
    drop(coarse);
    let (bright, clear_s, clear_corners, _) = build(SHADOW_LATTICE, &clear);
    let clear_pixels = shoot(&bright, "shipped-clear");
    drop(bright);
    write_png("shipped-shadow-fine.png", &fine_pixels);
    write_png("shipped-shadow-coarse.png", &coarse_pixels);

    let difference = |clear: &[u8], scene: &[u8]| -> Vec<f64> {
        clear
            .chunks(4)
            .zip(scene.chunks(4))
            .map(|(a, b)| {
                (0.2126 * (a[0] as f64 - b[0] as f64)
                    + 0.7152 * (a[1] as f64 - b[1] as f64)
                    + 0.0722 * (a[2] as f64 - b[2] as f64))
                    .max(0.0)
            })
            .collect()
    };
    let fine_edge = panel_edge(&difference(&clear_pixels, &fine_pixels), &camera, panel);
    let coarse_edge = panel_edge(&difference(&clear_pixels, &coarse_pixels), &camera, panel);
    for (label, edge) in [("shipped fine", &fine_edge), ("shipped coarse", &coarse_edge)] {
        println!(
            "{label}: the shadow's own footprint fades 90 % to 10 % over {:.1} px = {:.3} m \
             (the panel is {:.1} px across), across {:.0} level(s) of grey",
            edge.width_px, edge.width_m, edge.face_px, edge.drop
        );
    }
    println!(
        "shipped cost: {:.0} corner(s) / {:.0} index(es) at the lattice in {:.1} ms (median of 5, \
         {:.1}..{:.1}) and {} field ask(s), against {:.0} / {:.0} at lattice 1 in {:.1} ms \
         ({:.1}..{:.1}) and {} ask(s); the clear frame is {:.0} corners in {:.1} ms",
        fine_corners as f64,
        fine_indices as f64,
        fine_s * 1e3,
        fine_lo * 1e3,
        fine_hi * 1e3,
        fine_asks,
        coarse_corners as f64,
        coarse_indices as f64,
        coarse_s * 1e3,
        coarse_lo * 1e3,
        coarse_hi * 1e3,
        coarse_asks,
        clear_corners as f64,
        clear_s * 1e3
    );
    let chunks = settled.resident as f64;
    println!(
        "shipped cost, per chunk: {:.2} ms and {} ask(s) a chunk at the lattice against {:.2} ms \
         and {} at lattice 1 ({:.0} chunk(s))",
        fine_s * 1e3 / chunks,
        (fine_asks as f64 / chunks).round(),
        coarse_s * 1e3 / chunks,
        (coarse_asks as f64 / chunks).round(),
        chunks
    );
    // **The claim, on the shipped world**: the same panel, the same field, and the lattice's own
    // edge is a fraction of the one the corner path draws — below the metre that path cannot go
    // under.
    assert!(
        fine_edge.width_m > 0.0,
        "the shipped panel carried no light edge to measure"
    );
    assert!(
        fine_edge.width_m * 2.0 < coarse_edge.width_m,
        "the shipped panel's fine edge is {:.3} m against the coarse {:.3} m: not a finer shadow",
        fine_edge.width_m,
        coarse_edge.width_m,
    );
    assert!(
        fine_edge.width_m < 0.5,
        "the shipped panel's fine edge is {:.3} m: not below the voxel it was meant to resolve",
        fine_edge.width_m
    );
}

/// **A real terrain panel** the shipped measurement draws: its own basis from the mesh's
/// corners, the lattice samples measured once, and the full-sun bands the clear frame needs.
struct Panel {
    origin: [f64; 3],
    u: [f64; 3],
    v: [f64; 3],
    lu: f64,
    lv: f64,
    nu: usize,
    nv: usize,
    full: [f32; 6],
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

/// **The shoulder of the shadow on one panel, in pixels and in metres.** The difference image is
/// clear minus lit, so the panel's own material and the ambient cancel and what is left is the
/// sun. Samples are projected from the panel's own corners (not interpolated on the screen), and
/// the profile is averaged across its width.
fn panel_edge(difference: &[f64], camera: &Camera, panel: &Panel) -> Edge {
    let matrix = camera.view_projection();
    let rows = 24usize;
    let steps = (panel.nu * 24).max(96);
    let mut profile = vec![0.0f64; steps + 1];
    let mut across_px = 0.0f64;
    for j in 0..=rows {
        let t = j as f64 / rows as f64;
        let mut previous: Option<[f64; 2]> = None;
        for i in 0..=steps {
            let s = i as f64 / steps as f64;
            let xy = project(camera, &matrix, panel.point(s, t));
            profile[i] += sample(difference, xy[0], xy[1]);
            if let Some(last) = previous {
                across_px += ((xy[0] - last[0]).powi(2) + (xy[1] - last[1]).powi(2)).sqrt();
            }
            previous = Some(xy);
        }
    }
    // **Averaged over the rows**, not over the profile's own length: each of the `rows + 1`
    // traversals contributed one sample per step, and dividing by the step count would report a
    // drop twenty-five times smaller than the frame's.
    let n = (rows + 1) as f64;
    let mut profile: Vec<f64> = profile.iter().map(|value| value / n).collect();
    // A three-sample box, the same as the synthetic measurement: one aliased pixel is not a slope.
    let raw = profile.clone();
    for i in 0..raw.len() {
        let lo = i.saturating_sub(1);
        let hi = (i + 1).min(raw.len() - 1);
        profile[i] = (raw[lo] + raw[i] + raw[hi]) / 3.0;
    }
    let margin = (steps / 16).max(1);
    let inner = &profile[margin..profile.len().saturating_sub(margin)];
    let high = inner.iter().cloned().fold(f64::MIN, f64::max);
    let low = inner.iter().cloned().fold(f64::MAX, f64::min);
    let drop = high - low;
    // The scan runs from the lit end, wherever the panel's own u axis puts it.
    let scan: Vec<f64> = if inner[0] >= inner[inner.len() - 1] {
        inner.to_vec()
    } else {
        inner.iter().rev().copied().collect()
    };
    let crossing = |fraction: f64| -> f64 {
        let target = high - fraction * drop;
        for i in 1..scan.len() {
            let (a, b) = (scan[i - 1], scan[i]);
            if a >= target && b <= target {
                let u = (a - target) / (a - b).max(1e-9);
                return (i - 1) as f64 + u;
            }
        }
        (scan.len() - 1) as f64
    };
    let width_samples = (crossing(0.1) - crossing(0.9)).abs();
    let width_s = width_samples / (steps as f64);
    let face_px = across_px / (rows + 1) as f64;
    Edge {
        width_px: width_s * face_px,
        width_m: width_s * panel.lu,
        face_px,
        drop,
    }
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}


fn length(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}

fn bands_luminance(bands: [f32; 6]) -> f64 {
    0.2126 * bands[0] as f64 + 0.7152 * bands[1] as f64 + 0.0722 * bands[2] as f64
}

/// World metres → screen pixels, through the camera's own column-major view-projection. The
/// matrix has no translation (positions are camera-relative), so the eye is subtracted here.
fn project(camera: &Camera, matrix: &[[f32; 4]; 4], point: [f64; 3]) -> [f64; 2] {
    let v = [
        point[0] - camera.position_m[0],
        point[1] - camera.position_m[1],
        point[2] - camera.position_m[2],
    ];
    let mut clip = [0.0f64; 4];
    for (row, slot) in clip.iter_mut().enumerate() {
        for (column, value) in v.iter().enumerate() {
            *slot += matrix[column][row] as f64 * value;
        }
        *slot += matrix[3][row] as f64;
    }
    let w = clip[3];
    [
        (clip[0] / w * 0.5 + 0.5) * WIDTH as f64,
        (1.0 - (clip[1] / w * 0.5 + 0.5)) * HEIGHT as f64,
    ]
}

/// Bilinear read of the difference image at a screen position.
fn sample(image: &[f64], x: f64, y: f64) -> f64 {
    let x = x.clamp(0.0, WIDTH as f64 - 1.0);
    let y = y.clamp(0.0, HEIGHT as f64 - 1.0);
    let (x0, y0) = (x.floor() as u32, y.floor() as u32);
    let (x1, y1) = ((x0 + 1).min(WIDTH - 1), (y0 + 1).min(HEIGHT - 1));
    let (fx, fy) = (x - x0 as f64, y - y0 as f64);
    let at = |xi: u32, yi: u32| image[(yi * WIDTH + xi) as usize];
    at(x0, y0) * (1.0 - fx) * (1.0 - fy)
        + at(x1, y0) * fx * (1.0 - fy)
        + at(x0, y1) * (1.0 - fx) * fy
        + at(x1, y1) * fx * fy
}

/// **One shadow edge, measured off the pixels.**
///
/// The two renders of one lattice differ only in the light the face was built with, so their
/// difference is the shadow and nothing else: the material's pattern, the rig's shading and the
/// sky cancel. The difference is averaged down the columns, and its **equivalent width** is the
/// drop divided by the steepest slope — a linear blend over one metre reads as a metre, and a
/// blend confined to one quarter-metre cell reads as a quarter of one.
struct Edge {
    width_px: f64,
    width_m: f64,
    face_px: f64,
    drop: f64,
}

fn luminance(pixels: &[u8], x: u32, y: u32) -> f64 {
    let at = ((y * WIDTH + x) * 4) as usize;
    0.2126 * pixels[at] as f64 + 0.7152 * pixels[at + 1] as f64 + 0.0722 * pixels[at + 2] as f64
}

fn shadow_edge(clear: &[u8], shadowed: &[u8]) -> Edge {
    let rows = 20..HEIGHT - 20;
    let column = |pixels: &[u8]| -> Vec<f64> {
        (0..WIDTH)
            .map(|x| {
                let mut sum = 0.0;
                for y in rows.clone() {
                    sum += luminance(pixels, x, y);
                }
                sum / rows.len() as f64
            })
            .collect()
    };
    let lit = column(clear);
    let strength: Vec<f64> = column(shadowed)
        .iter()
        .zip(&lit)
        .map(|(shadowed, clear)| (clear - shadowed).max(0.0))
        .collect();
    // A three-pixel box, so a single aliased pixel is not a slope.
    let smooth: Vec<f64> = (0..strength.len())
        .map(|x| {
            let lo = x.saturating_sub(1);
            let hi = (x + 1).min(strength.len() - 1);
            (strength[lo] + strength[x] + strength[hi]) / 3.0
        })
        .collect();
    // **The face's own footprint from the lit render**, not from the shadow: the shadow covers
    // only half of it at the fine lattice, and a face measured by its shadow would be measured
    // differently in the two frames.
    let background = (lit[..32].iter().sum::<f64>() + lit[WIDTH as usize - 32..].iter().sum::<f64>())
        / 64.0;
    let face: Vec<usize> = (0..lit.len()).filter(|x| (lit[*x] - background).abs() > 3.0).collect();
    let (Some(first), Some(last)) = (face.first(), face.last()) else {
        return Edge { width_px: 0.0, width_m: 0.0, face_px: 0.0, drop: 0.0 };
    };
    let (first, last) = (*first, *last);
    let face_px = (last - first) as f64;
    // **The shoulders of the fade, 90 % to 10 % of the drop**, which is the width a viewer reads
    // as the edge. The ramp's two ends are inside the face by construction, so the face's own
    // boundary pixels are not part of the measurement.
    let margin = ((last - first) / 12).max(1);
    let inner = &smooth[first + margin..last.saturating_sub(margin)];
    let high = inner.iter().cloned().fold(f64::MIN, f64::max);
    let low = inner.iter().cloned().fold(f64::MAX, f64::min);
    let drop = (high - low).max(1e-6);
    // **Which way the fade runs**: the camera looks straight down, so the screen's left is not
    // promised to be the face's low x. The scan starts at the lit end, wherever that is.
    let scan: Vec<f64> = if inner[0] >= inner[inner.len() - 1] {
        inner.to_vec()
    } else {
        inner.iter().rev().copied().collect()
    };
    let shoulder = |fraction: f64| {
        scan.iter()
            .position(|v| *v <= high - fraction * drop)
            .unwrap_or(scan.len() - 1)
    };
    let (ten, ninety) = (shoulder(0.1) as f64, shoulder(0.9) as f64);
    let width_px = (ten - ninety).abs();
    Edge { width_px, width_m: (width_px / face_px).min(1.0), face_px, drop }
}

fn read_back(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture) -> Vec<u8> {
    let bytes_per_row = WIDTH * 4;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("shadow-lattice readback"),
        size: (bytes_per_row * HEIGHT) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("readback") });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(HEIGHT),
            },
        },
        wgpu::Extent3d { width: WIDTH, height: HEIGHT, depth_or_array_layers: 1 },
    );
    queue.submit(Some(encoder.finish()));
    let slice = readback.slice(..);
    let (sender, receiver) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    device
        .poll(wgpu::PollType::Wait { submission_index: None, timeout: None })
        .expect("the readback poll");
    receiver.recv().expect("the map callback").expect("the map");
    let pixels: Vec<u8> = slice.get_mapped_range().expect("the mapped range").to_vec();
    readback.unmap();
    let mut rgba = Vec::with_capacity(pixels.len());
    for pixel in pixels.as_chunks::<4>().0 {
        rgba.extend_from_slice(&[pixel[2], pixel[1], pixel[0], 255]);
    }
    rgba
}

/// **Where `target/` is**, honouring `CARGO_TARGET_DIR` the way Cargo itself does: this tree
/// builds into a shared target directory, and a picture written to a `target/` that does not exist
/// is a proof nobody can open.
fn target_path(name: &str) -> std::path::PathBuf {
    let root = std::env::var_os("CARGO_TARGET_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("target"));
    std::fs::create_dir_all(&root).expect("a target directory");
    root.join(name)
}

fn write_png(name: &str, rgba: &[u8]) {
    let path = target_path(name);
    let path = path.as_path();
    let file = std::fs::File::create(path).expect("a file to write into target/");
    let mut png = png::Encoder::new(std::io::BufWriter::new(file), WIDTH, HEIGHT);
    png.set_color(png::ColorType::Rgba);
    png.set_depth(png::BitDepth::Eight);
    png.write_header()
        .expect("a PNG header")
        .write_image_data(rgba)
        .expect("the pixels");
}
