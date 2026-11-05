//! **What a frame costs, measured where nothing else is moving.**
//!
//! The app's title line reports fps between a walk, a stream and a window compositor, and on this
//! machine consecutive runs of the *same build* with the *same pinned camera* read 92, 112, 94 then
//! 64, 49, 38 — so that number cannot separate a shader change from a background process. This
//! example takes the measurement out of the app: the shipped window is streamed once and settled,
//! the camera is placed and never moved, and every frame is rendered through the same
//! [`voxel_client::Renderer`] the window uses — including its readback, so the time is the whole
//! frame and not a submission.
//!
//! Three batches of frames are timed and each batch's **median** is printed, because a mean over a
//! machine that is doing something else is a number about the machine. The median of the three
//! medians is the number a change has to move.
//!
//! **The far field is measured the same way, against the frame without it.** The world past the
//! streamed window is generated at the eight-metre rung and drawn from its own record pair
//! ([`voxel_client::FarTerrain`]); the difference between the two medians on the same scene, camera
//! and machine is that field's price. The harness prints the chunks and rectangles it added, so a
//! delta cannot be read without knowing what it bought.
//!
//! **The quarter-size batch is the breakdown.** The same scene at a quarter of the pixels: a frame
//! whose time falls with the pixel count is fragment-bound, and one whose time barely moves is
//! paying for geometry, uploads or the driver. That is enough to tell "the per-fragment light march
//! is the cost" from "the scene upload is the cost" without a GPU timestamp query, which this
//! adapter's feature set does not require and `offscreen_device` does not ask for.
//!
//! ```text
//! cargo run --release -p voxel-client --example frame-cost
//! ```

use std::time::{Duration, Instant};

use voxel_appearance::sky::SkyInputs;
use voxel_client::{
    game, skylight, Camera, FarTerrain, GpuRig, MaterialParams, Renderer, Scene, SurfaceLadder,
    SHADOW_LATTICE,
};
use voxel_model::{ChunkPos, CHUNK_SIDE};
use voxel_shapers::{materials, Config, WorldSpec};

/// The window the binary opens, in pixels.
const FULL: (u32, u32) = (1280, 720);
/// A quarter of the pixels, for the breakdown.
const QUARTER: (u32, u32) = (640, 360);
/// Frames per batch: enough for a median, few enough to run three times.
const FRAMES: usize = 120;
/// Frames before the clock starts, so the first-frame pipeline state is not in the measurement.
const WARMUP: usize = 24;

fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(|a, b| a.partial_cmp(b).expect("finite milliseconds"));
    let middle = values.len() / 2;
    if values.len() % 2 == 0 {
        (values[middle - 1] + values[middle]) / 2.0
    } else {
        values[middle]
    }
}

fn percentile(sorted: &[f64], fraction: f64) -> f64 {
    let at = ((sorted.len() as f64 - 1.0) * fraction).round() as usize;
    sorted[at]
}

fn main() {
    if let Err(why) = run() {
        eprintln!("frame-cost: {why}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    // **The shipped world, streamed and settled**: the same spec, window and ceiling the binary
    // uses, so the geometry under the measurement is the geometry the player sees.
    let spec = WorldSpec::new(game::GAME_SEED, Config::default());
    let source = voxel_stream::WorldSource::new(spec.clone());
    let registry = materials::registry().map_err(|e| e.to_string())?;
    let mesh_config =
        voxel_mesh::MeshConfig::from_registry(&registry).map_err(|e| e.to_string())?;
    let workers = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    let mut streamer =
        voxel_work::Streamer::new(source, mesh_config, workers, game::GAME_CEILING_BYTES)
            .and_then(|s| s.with_window(game::GAME_WINDOW))
            .map_err(|e| e.to_string())?;
    let spawn = game::spawn_eye(&game::World::shipped());
    let centre = ChunkPos::new(
        (spawn[0] / f64::from(CHUNK_SIDE)).floor() as i32,
        (spawn[1] / f64::from(CHUNK_SIDE)).floor() as i32,
        (spawn[2] / f64::from(CHUNK_SIDE)).floor() as i32,
    );
    streamer.set_centre(centre);
    let settled = streamer.settle().map_err(|e| e.to_string())?;
    // **The horizon's own data**, so the measured frame is the frame the game draws: a renderer that
    // stamps nothing answers every march with `1.0` for lack of a measurement, which reads exactly
    // like a shader whose marches are free. Measured without it, this harness attributed 0 ms to
    // work it had switched off.
    let trees: Vec<(ChunkPos, u64, std::sync::Arc<voxel_mesh::SparseVoxelOctree>)> = streamer
        .drawn_meshes()
        .filter_map(|(pos, _, _)| {
            Some((pos, streamer.tree_version(pos)?, streamer.tree_shared(pos)?))
        })
        .collect();

    // **The frame's light is the rig's** (`clear` corners), so the measurement is the shader's
    // own per-fragment work: the material ladder, the two horizon bands and the shadow map.
    let clear = |_: [f64; 3]| Some([1.0f32, 1.0, 1.0, 0.0, 0.0, 0.0]);
    let built = Instant::now();
    let scene = Scene::build_with_light_lattice(
        streamer.drawn_meshes().map(|(_, _, mesh)| mesh),
        spawn,
        &clear,
        None,
        None,
        SHADOW_LATTICE,
    );
    let scene_ms = built.elapsed().as_secs_f64() * 1000.0;

    let table = voxel_client::table(
        &registry,
        &materials::shading_field().map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let day = skylight::daylight_of(
        &spec,
        game::GAME_TICK,
        SkyInputs::default(),
        game::GAME_NEW_MOON_TICK,
    )
    .ok_or("the shipped hour is not a legal sky")?;
    let exposed = skylight::exposed_at(&day.rig, day.grade);
    let rig: GpuRig = GpuRig::from_rig(&exposed).ok_or("the rig does not fit the uniform")?;

    let camera = Camera::standard(spawn, 2.0);
    let quads: usize = streamer
        .drawn_meshes()
        .map(|(_, _, mesh)| mesh.quads.len())
        .sum();
    let chunks = streamer.drawn_meshes().count();

    println!(
        "scene: {chunks} chunks, {quads} quads, built in {scene_ms:.1} ms, {} MB mesh, {} MB of trees",
        settled.bytes / (1024 * 1024),
        streamer.tree_bytes() / (1024 * 1024),
    );

    // ---- the far field ------------------------------------------------------
    // **Generated by the field's own thread, not by this harness**: the ring, its census cache and
    // its mesher are the ones the game runs, so the measurement is of the shipped path and not of a
    // stand-in that happens to produce the same rectangles.
    let far_started = Instant::now();
    let mut far = FarTerrain::new(spec, game::GAME_WINDOW.radius_chunks);
    far.set_eye(centre);
    let deadline = Instant::now() + std::time::Duration::from_secs(180);
    while far.len() < far.scheduled() && Instant::now() < deadline {
        far.advance(usize::MAX);
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    let far_ms = far_started.elapsed().as_secs_f64() * 1000.0;
    let far_quads = far.quads();
    println!(
        "far field: {} of {} chunks, {} quads, generated in {far_ms:.0} ms ({} census lanes)",
        far.len(),
        far.scheduled(),
        far_quads,
        voxel_client::far_terrain::FAR_CENSUS_LANES,
    );
    if far.len() < far.scheduled() {
        println!("far field: INCOMPLETE — the ring did not arrive before the deadline");
    }
    let far_scene = scene.clone().with_far(far.meshes());

    // ---- tier 6, the rung past the far ring ---------------------------------
    // The same field API, a coarser tier and a ring one band further out. Its fill is timed the
    // same way: the wait a player pays, not the worker's own clock.
    let wide_started = Instant::now();
    let mut wide = FarTerrain::at_tier(
        spec.clone(),
        game::GAME_WINDOW.radius_chunks,
        voxel_client::lod::WIDE_TIER,
        3,
        4,
    );
    wide.set_eye(centre);
    let deadline = Instant::now() + std::time::Duration::from_secs(180);
    while wide.len() < wide.scheduled() && Instant::now() < deadline {
        wide.advance(usize::MAX);
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    let wide_ms = wide_started.elapsed().as_secs_f64() * 1000.0;
    println!(
        "tier 6: {} of {} chunks, {} quads, generated in {wide_ms:.0} ms",
        wide.len(),
        wide.scheduled(),
        wide.quads(),
    );
    if wide.len() < wide.scheduled() {
        println!("tier 6: INCOMPLETE — the ring did not arrive before the deadline");
    }
    let mut both_scene = far_scene.clone();
    both_scene.add_far_at(wide.meshes(), voxel_client::lod::WIDE_CELL_M);

    // ---- tier 7, the last chunk-held rung ----------------------------------
    let chunk_started = Instant::now();
    let mut chunk_band = FarTerrain::at_tier(
        spec.clone(),
        game::GAME_WINDOW.radius_chunks,
        voxel_client::lod::CHUNK_TIER,
        5,
        6,
    );
    chunk_band.set_eye(centre);
    let deadline = Instant::now() + std::time::Duration::from_secs(180);
    while chunk_band.len() < chunk_band.scheduled() && Instant::now() < deadline {
        chunk_band.advance(usize::MAX);
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    let chunk_ms = chunk_started.elapsed().as_secs_f64() * 1000.0;
    println!(
        "tier 7: {} of {} chunks, {} quads, generated in {chunk_ms:.0} ms",
        chunk_band.len(),
        chunk_band.scheduled(),
        chunk_band.quads(),
    );
    if chunk_band.len() < chunk_band.scheduled() {
        println!("tier 7: INCOMPLETE — the ring did not arrive before the deadline");
    }
    let mut ladder_scene = both_scene.clone();
    ladder_scene.add_far_at(chunk_band.meshes(), voxel_client::lod::CHUNK_CELL_M);

    // ---- the surface ladder, every rung above a chunk ------------------------
    // **One sample a cell, measured on the ladder's own thread**: the same worker the game pumps,
    // so this is the shipped fill and not a stand-in. The per-rung sample time is the worker's own
    // clock (`SurfaceRing::sampled_us`), and the wall time is what a cold start waits for.
    let ladder_started = Instant::now();
    let mut surface = SurfaceLadder::new(spec.clone());
    let deadline = Instant::now() + std::time::Duration::from_secs(180);
    while surface.len() < surface.total() && Instant::now() < deadline {
        surface.advance(spawn, usize::MAX);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let ladder_ms = ladder_started.elapsed().as_secs_f64() * 1000.0;
    println!(
        "surface ladder: {} of {} rungs, {} quads, {:.1} ms of samples, {ladder_ms:.0} ms of wall ",
        surface.len(),
        surface.total(),
        surface.quads(),
        surface.sampled_us() as f64 / 1000.0,
    );
    for ring in surface.rings() {
        println!(
            "  tier {:>2}: cell {:>14.0} m, outer {:>16.0} m, {:.2} ms, {:>5} quads, {} band words",
            ring.tier,
            ring.cell_m,
            voxel_client::lod::surface_outer_m(ring.tier),
            ring.sampled_us as f64 / 1000.0,
            ring.quads.len(),
            ring.words.len(),
        );
    }
    println!(
        "  band memory: {} slots × {} words = {:.2} MB of the ladder's one coarse buffer",
        voxel_client::horizon::SURFACE_SLOTS,
        voxel_client::horizon::SURFACE_BAND_WORDS,
        voxel_client::horizon::HorizonStream::surface_band_bytes() as f64 / (1024.0 * 1024.0),
    );
    if surface.len() < surface.total() {
        println!("surface ladder: INCOMPLETE — the rungs did not arrive before the deadline");
    }
    let mut rung_scene = ladder_scene.clone();
    if !surface.is_empty() {
        rung_scene.add_surface(surface.rings());
    }

    let mut medians: Vec<f64> = Vec::new();
    for run in 0..3 {
        let ms = batch(
            &scene, &camera, &table, &rig, &trees, None, None, None, None, FULL,
        )?;
        let sorted = {
            let mut s = ms.clone();
            s.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
            s
        };
        let med = median(ms);
        println!(
            "run {run}: median {med:.2} ms, p10 {:.2}, p90 {:.2} at {}x{}",
            percentile(&sorted, 0.10),
            percentile(&sorted, 0.90),
            FULL.0,
            FULL.1,
        );
        medians.push(med);
    }
    let quarter = median(batch(
        &scene, &camera, &table, &rig, &trees, None, None, None, None, QUARTER,
    )?);
    let full = median(medians);
    println!(
        "median of medians: {full:.2} ms ({:.1} fps) at {}x{}; {quarter:.2} ms at {}x{} \
         (a quarter of the pixels, ratio {:.2})",
        1000.0 / full,
        FULL.0,
        FULL.1,
        QUARTER.0,
        QUARTER.1,
        full / quarter,
    );

    // ---- and the same frame with the far field drawn -----------------------
    let mut far_medians: Vec<f64> = Vec::new();
    for run in 0..3 {
        let med = median(batch(
            &far_scene,
            &camera,
            &table,
            &rig,
            &trees,
            Some(&far),
            None,
            None,
            None,
            FULL,
        )?);
        println!("far run {run}: median {med:.2} ms at {}x{}", FULL.0, FULL.1);
        far_medians.push(med);
    }
    let far_quarter = median(batch(
        &far_scene,
        &camera,
        &table,
        &rig,
        &trees,
        Some(&far),
        None,
        None,
        None,
        QUARTER,
    )?);
    let far_full = median(far_medians);
    println!(
        "with the far field: {far_full:.2} ms ({:.1} fps) at {}x{}; {far_quarter:.2} ms at {}x{} \
         (ratio {:.2}); the field's price {:.2} ms over the same scene",
        1000.0 / far_full,
        FULL.0,
        FULL.1,
        QUARTER.0,
        QUARTER.1,
        far_full / far_quarter,
        far_full - full,
    );
    // ---- and the same frame with every live rung ---------------------------
    let mut ladder_medians: Vec<f64> = Vec::new();
    for run in 0..3 {
        let med = median(batch(
            &ladder_scene,
            &camera,
            &table,
            &rig,
            &trees,
            Some(&far),
            Some(&wide),
            Some(&chunk_band),
            None,
            FULL,
        )?);
        println!(
            "all live rungs {run}: median {med:.2} ms at {}x{}",
            FULL.0, FULL.1
        );
        ladder_medians.push(med);
    }
    let ladder_quarter = median(batch(
        &ladder_scene,
        &camera,
        &table,
        &rig,
        &trees,
        Some(&far),
        Some(&wide),
        Some(&chunk_band),
        None,
        QUARTER,
    )?);
    let ladder_full = median(ladder_medians);
    println!(
        "with all live rungs: {ladder_full:.2} ms ({:.1} fps) at {}x{}; {ladder_quarter:.2} ms at {}x{}; \
         tiers 6+7's price {:.2} ms over the far field's frame",
        1000.0 / ladder_full,
        FULL.0,
        FULL.1,
        QUARTER.0,
        QUARTER.1,
        ladder_full - far_full,
    );

    // ---- and every rung above a chunk, drawn out of the one coarse buffer --
    let mut rung_medians: Vec<f64> = Vec::new();
    for run in 0..3 {
        let med = median(batch(
            &rung_scene,
            &camera,
            &table,
            &rig,
            &trees,
            Some(&far),
            Some(&wide),
            Some(&chunk_band),
            Some(&surface),
            FULL,
        )?);
        println!(
            "all {} rungs {run}: median {med:.2} ms at {}x{}",
            surface.len(),
            FULL.0,
            FULL.1
        );
        rung_medians.push(med);
    }
    let rung_quarter = median(batch(
        &rung_scene,
        &camera,
        &table,
        &rig,
        &trees,
        Some(&far),
        Some(&wide),
        Some(&chunk_band),
        Some(&surface),
        QUARTER,
    )?);
    let rung_full = median(rung_medians);
    println!(
        "with the surface ladder: {rung_full:.2} ms ({:.1} fps) at {}x{}; {rung_quarter:.2} ms at {}x{}; \
         the {} rungs' own price {:.2} ms over tiers 6+7's frame",
        1000.0 / rung_full,
        FULL.0,
        FULL.1,
        QUARTER.0,
        QUARTER.1,
        surface.len(),
        rung_full - ladder_full,
    );
    Ok(())
}

/// **One batch of timed frames**, each one a full render with its readback.
fn batch(
    scene: &Scene,
    camera: &Camera,
    table: &[MaterialParams],
    rig: &GpuRig,
    trees: &[(ChunkPos, u64, std::sync::Arc<voxel_mesh::SparseVoxelOctree>)],
    far: Option<&FarTerrain>,
    wide: Option<&FarTerrain>,
    chunk_band: Option<&FarTerrain>,
    surface: Option<&SurfaceLadder>,
    (width, height): (u32, u32),
) -> Result<Vec<f64>, String> {
    let mut renderer = Renderer::new(width, height, table, rig).map_err(|e| e.to_string())?;
    // **No sky, stated rather than absent.** The sky is its own pass and its own copy; a frame that
    // states none still pays the world pass's full per-fragment cost, and the measurement is that
    // pass. The door refuses a frame that never said either way, which is why this line exists.
    renderer.world_mut().write_no_sky_model();
    // **The cube anchored on this frame's eye, then the resident chunks stamped into it** — the two
    // calls the window makes every frame, and the difference between a frame with marches and one
    // without (`horizon_recenter` writes the `HorizonView` a fragment subtracts; without it every
    // march lands outside its own cube and answers `1.0`).
    renderer.world_mut().horizon_recenter(camera.position_m);
    for (pos, version, tree) in trees {
        renderer.world_mut().horizon_stamp(*pos, tree, *version);
    }
    // **The far field's own light**, from the census the mesh was cut from — the same stamp the
    // window makes for a far chunk on the frame it adopts one. Without it the far faces would be
    // lit by the shadow map alone, which is the stand-in the coarse band exists to replace.
    if let Some(far) = far {
        for chunk in far.chunks() {
            renderer
                .world_mut()
                .horizon_stamp_far(chunk.pos, &chunk.census, chunk.version);
            // The app also feeds the far ring's voxels into the 16 m band, so a tier-6 beam
            // crossing the ring meets it; the measurement must include that write.
            if wide.is_some() {
                renderer.world_mut().horizon_stamp_wide(
                    chunk.pos,
                    &chunk.census.aggregate(2),
                    chunk.version,
                );
            }
        }
    }
    if let Some(wide) = wide {
        for chunk in wide.chunks() {
            renderer
                .world_mut()
                .horizon_stamp_wide(chunk.pos, &chunk.census, chunk.version);
            if chunk_band.is_some() {
                renderer.world_mut().horizon_stamp_chunk_band(
                    chunk.pos,
                    &chunk.census.aggregate(2),
                    chunk.version,
                );
            }
        }
    }
    if let Some(chunk_band) = chunk_band {
        for chunk in chunk_band.chunks() {
            renderer
                .world_mut()
                .horizon_stamp_chunk_band(chunk.pos, &chunk.census, chunk.version);
        }
    }
    // **And each surface rung's own band**, from the very samples its faces were cut from — the
    // stamp the window makes on the frame it adopts a rung. Without it a rung's faces would be lit
    // by the finest band that happens to cover them, which is a measurement of another grain.
    if let Some(surface) = surface {
        for ring in surface.rings() {
            renderer.world_mut().horizon_stamp_surface(ring);
        }
    }
    for _ in 0..WARMUP {
        renderer.render(scene, camera).map_err(|e| e.to_string())?;
    }
    let mut times: Vec<f64> = Vec::with_capacity(FRAMES);
    for _ in 0..FRAMES {
        // A frame that is asked for and never used: the readback in `render` is the sync, so this
        // is the whole frame's time and not the submission's.
        let started: Instant = Instant::now();
        let frame = renderer.render(scene, camera).map_err(|e| e.to_string())?;
        let elapsed: Duration = started.elapsed();
        std::hint::black_box(&frame);
        times.push(elapsed.as_secs_f64() * 1000.0);
    }
    Ok(times)
}
