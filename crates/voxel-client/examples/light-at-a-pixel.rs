//! **One ground pixel's light, from two headings and two roofs** — and the numbers the ground's
//! darkness turns on.
//!
//! ```text
//! cargo run --release -p voxel-client --example light-at-a-pixel
//! ```
//!
//! The window cannot answer this: its recorder reads the swapchain back every frame, which drops
//! the loop to a couple of frames a second and leaves the world half streamed, and the demo's own
//! ghost owns the camera's heading, so a pinned `SHPLOINGUS_EYE` places the eye and nothing else.
//! This example renders the **shipped** world offscreen through the same [`Renderer`] at about
//! 17 ms a frame — the same scene, the same rig, the same shader — with the horizon stamped from
//! the same resident octrees the window stamps, and it sets the camera's heading and its stance
//! directly.
//!
//! # The three frames per pose
//!
//! * **the frame**, as shipped;
//! * **the dome alone** — the same rig with its light count zeroed, so `direct` is exactly zero and
//!   the pixel is only what the sky delivers through `world_sky`;
//! * **the dome alone with no horizon stamped** — the same geometry and rig, a fresh pipeline whose
//!   bands hold nothing, where the march has no measurement and answers `1.0` by construction.
//!
//! The third is the denominator of the number this file exists to print — **the share the frame's
//! sky is multiplied by**, or rather the ratio the two dome-only frames give:
//!
//! ```text
//! sky + one bounce = luminance(dome alone) / luminance(dome alone, unmeasured)
//!                  = share + BOUNCE_REFLECTANCE · (1 − share)²
//! ```
//!
//! **It is no longer `share` alone, and the difference is a term rather than a rounding.** A face
//! the horizon can see no sky from measured a share of `0.0`, and `0.0` times any dome is zero: the
//! light that a wall hides has not left the world, it landed on the wall, so the frame adds the sky
//! the *occluder* sees at the occluder's own reflectance. That term is zero wherever the share is
//! one, which is why the open readings do not move; its floor is `BOUNCE_REFLECTANCE`, so a fully
//! enclosed face reads `0.430` here and read `0.000` before the term existed.
//!
//! # Why the obstructed poses matter as much as the open ones
//!
//! A share of `1.000` at every heading of open sky is also what an **unwired** occlusion path looks
//! like: a term that never consults the horizon reports exactly one. So the same differential is run
//! under a canopy and beside a wall, found from the streamed world rather than by eye:
//!
//! * **under a canopy**: a column with air at head height and solid matter above it;
//! * **against a wall**: a column with solid matter within three metres on the side the shader's own
//!   two slanted rays go (`+x`, `−z`), rising past head height.
//!
//! If those come back near `1.0` as well, the occlusion is not reaching the term and raising the
//! ambient constant would light a cave exactly as brightly as open ground. If they fall, the term is
//! wired and the number left to choose is the constant's.
//!
//! The pixel is named, not chosen after the fact: `(width/2, 4·height/5)`, twenty per cent up from
//! the bottom of the frame, which at the shipped eye and pitch is ground a few metres ahead.
//!
//! # The enclosed set, which is where black survived
//!
//! A named pixel in the open says nothing about a walled-in face, and a cave is not something to
//! guess the location of, so the example reports two numbers a pose cannot fake: the **enclosed
//! set** (every pixel within `0.02` of the frame's own floor share — the most enclosed class that
//! view holds — read in the full frame, as a median) and the **darkest geometry pixel in the
//! frame**. The `into a wall` pose is the wall the
//! world actually has, with the heading turned round, so its whole frame stands at the floor share.
//!
//! # The crossing, and the coarse field
//!
//! A picture at a settled eye is the easy case: every chunk that is drawn has been stamped long
//! ago. The black a player reports is at a **chunk crossing**, when the bands shift under the eye and
//! nine chunks leave the cube as nine enter it — so the example then steps the eye one chunk at a
//! time across three boundaries and re-issues the window's own stamp order at each, budgets and all,
//! asking the record at every step whether a chunk the window would draw has a measurement.
//!
//! Beyond the cube the question is different and is asked at the far band's own eight metres: the
//! coarse cell over a distant chunk's surface, stamped or not, and whether it stands in a sky ray's
//! way. Silence there is not black — an unstamped coarse cell returns *clear* — so the far field's
//! debt is shadowing a distant hillside would have cast, and the number of frames of stamping it
//! takes to pay is printed beside it.

use std::collections::BTreeSet;
use std::time::Instant;

use voxel_appearance::sky::SkyInputs;
use voxel_client::{
    game, luminance, skylight, Camera, Frame, GpuRig, MaterialParams, Renderer, Scene,
    SHADOW_LATTICE,
};
use voxel_model::{ChunkPos, CHUNK_SIDE};
use voxel_shapers::{materials, Config, WorldSpec};

/// The window the shipped binary opens.
const WIDTH: u32 = 1280;
/// See [`WIDTH`].
const HEIGHT: u32 = 720;
/// The named pixel: half-width, four fifths down.
const AT_X: u32 = WIDTH / 2;
/// See [`AT_X`].
const AT_Y: u32 = HEIGHT * 4 / 5;
/// Headings tried, in degrees. Eight so the pair that faces the beam and the pair that turns from
/// it are found from `forward · beam` rather than assumed from the camera's yaw convention.
const HEADINGS: [f64; 8] = [0.0, 45.0, 90.0, 135.0, 180.0, 225.0, 270.0, 315.0];
/// Pixels sampled when the occlusion share is summarised over a frame: every eighth pixel, which is
/// a fifth of a million readings — enough for a minimum that is not a coincidence.
const STRIDE: u32 = 8;

type Trees = Vec<(ChunkPos, u64, std::sync::Arc<voxel_mesh::SparseVoxelOctree>)>;

fn main() {
    if let Err(why) = run() {
        eprintln!("light-at-a-pixel: {why}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let started = Instant::now();
    let spec = WorldSpec::new(game::GAME_SEED, Config::default());
    let source = voxel_stream::WorldSource::new(spec);
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
    let eye = game::spawn_eye(&game::World::shipped());
    streamer.set_centre(ChunkPos::new(
        (eye[0] / f64::from(CHUNK_SIDE)).floor() as i32,
        (eye[1] / f64::from(CHUNK_SIDE)).floor() as i32,
        (eye[2] / f64::from(CHUNK_SIDE)).floor() as i32,
    ));
    let settled = streamer.settle().map_err(|e| e.to_string())?;
    let drawn: Trees = streamer
        .drawn_meshes()
        .filter_map(|(pos, _, _)| {
            Some((pos, streamer.tree_version(pos)?, streamer.tree_shared(pos)?))
        })
        .collect();

    // The frame's light: the rig's own beam and ambient at the shipped tick, exactly as the window
    // derives them (`clear` corners, so no per-corner band reaches the pixel).
    let clear = |_: [f64; 3]| Some([1.0f32, 1.0, 1.0, 0.0, 0.0, 0.0]);
    let scene = Scene::build_with_light_lattice(
        streamer.drawn_meshes().map(|(_, _, mesh)| mesh),
        eye,
        &clear,
        None,
        None,
        SHADOW_LATTICE,
    );
    let table: Vec<MaterialParams> = voxel_client::table(
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
    // **The dome alone**: the same ambient radiance, no beam at all, so `beam_known` is false and
    // `direct` stays zero. Nothing else about the frame changes.
    let dome_rig = GpuRig {
        count: [0; 4],
        ..rig
    };
    let beam = rig.lights[0].direction;
    let beam_m = [f64::from(beam[0]), f64::from(beam[1]), f64::from(beam[2])];

    let quads: usize = streamer
        .drawn_meshes()
        .map(|(_, _, mesh)| mesh.quads.len())
        .sum();
    println!(
        "scene: {} chunks, {quads} quads, {} MB of mesh, settled in {:.1} ms",
        drawn.len(),
        settled.bytes / (1024 * 1024),
        started.elapsed().as_secs_f64() * 1000.0,
    );
    println!(
        "rig: beam {:.3} W/m², ambient {:.3} W/m² (ambient is {:.1}% of the beam), beam direction \
         [{:.3}, {:.3}, {:.3}], AMBIENT_FROM_HORIZON {:.2}",
        luminance_of(rig.lights[0].colour) * f64::from(rig.lights[0].direction[3]),
        luminance_of(rig.ambient),
        100.0 * luminance_of(rig.ambient)
            / (luminance_of(rig.lights[0].colour) * f64::from(rig.lights[0].direction[3])),
        beam[0],
        beam[1],
        beam[2],
        skylight::AMBIENT_FROM_HORIZON,
    );

    // ---- the two headings at the shipped eye ---------------------------------------
    let mut sweep = Vec::new();
    for &heading in &HEADINGS {
        let camera = pose(eye, heading);
        let forward = camera.forward();
        let facing = forward[0] * beam_m[0] + forward[1] * beam_m[1] + forward[2] * beam_m[2];
        sweep.push((heading, facing, camera));
    }
    let sunward = *sweep
        .iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).expect("finite"))
        .ok_or("no headings")?;
    let away = *sweep
        .iter()
        .min_by(|a, b| a.1.partial_cmp(&b.1).expect("finite"))
        .ok_or("no headings")?;

    // ---- the two stances, found from the world ------------------------------------
    let canopy = stance(&streamer, eye, Stance::Canopy);
    let wall = stance(&streamer, eye, Stance::Wall);
    let enclosed = stance(&streamer, eye, Stance::Enclosed);
    // **The table's own mean reflectance**, which is the number the second-bounce term is worth if
    // it is to be a measurement rather than a taste: the light that reaches a closed face last
    // landed on terrain, and this is how much of it that terrain gave back.
    let (mut sum, mut count) = (0.0f64, 0usize);
    for row in &table {
        let base = row.base_color_and_present;
        if base[3] > 0.5 {
            sum += 0.2126 * f64::from(base[0])
                + 0.7152 * f64::from(base[1])
                + 0.0722 * f64::from(base[2]);
            count += 1;
        }
    }
    println!(
        "materials: mean base reflectance {:.3} over {count} row(s) with a base colour",
        if count > 0 { sum / count as f64 } else { 0.0 }
    );

    let mut poses: Vec<(String, Camera)> = vec![
        (format!("sunward {:.0}°", sunward.0), sunward.2),
        (format!("away {:.0}°", away.0), away.2),
    ];
    if let Some(at) = canopy {
        poses.push(("under a canopy".to_string(), pose(at, sunward.0)));
    }
    if let Some(at) = wall {
        // Looking away from the wall on the side the shader's slanted rays go, so the wall is behind
        // the eye and still in their way.
        poses.push(("against a wall".to_string(), pose(at, sunward.0)));
    }
    if let Some(at) = wall {
        // **The wall in front of the eye**, not behind it: the same stance, the heading turned
        // round, so the named pixel is a face the horizon can see no sky from. That face is what
        // `0.0 × any dome` used to leave black, and it is the reading the second-bounce term is
        // judged on.
        poses.push(("into a wall".to_string(), pose(at, sunward.0 + 180.0)));
    }
    if let Some(at) = enclosed {
        poses.push(("enclosed".to_string(), pose(at, sunward.0)));
    }

    println!(
        "{:<16} {:>9} {:>9} {:>9} {:>10} {:>9} {:>9}",
        "pose", "frame", "dome", "dome-raw", "sky+1b", "min", "median"
    );
    for (name, camera) in &poses {
        let full = shoot(&scene, camera, &table, &rig, Some(&drawn))?;
        let dome = shoot(&scene, camera, &table, &dome_rig, Some(&drawn))?;
        let bare = shoot(&scene, camera, &table, &dome_rig, None)?;
        let (lit, sky, raw) = (at(&full), at(&dome), at(&bare));
        let stats = shares(&dome, &bare);
        println!(
            "{name:<16} {lit:>9.4} {sky:>9.4} {raw:>9.4} {:>10.3} {:>9.3} {:>9.3}",
            if raw > 0.0 { sky / raw } else { 0.0 },
            stats.0,
            stats.2,
        );
        println!(
            "  occlusion over the frame: min {:.3} at ({}, {}), p10 {:.3}, median {:.3}, \
             {} pixel(s) below 0.90 of {} sampled; enclosed set reads {:.4}",
            stats.0, stats.3 .0, stats.3 .1, stats.1, stats.2, stats.4, stats.5, stats.7,
        );
        // **The reading the second-bounce term is judged on**: the frame's darkest *sky* pixel,
        // named by the share itself rather than by a guess about where a cave is. A share of
        // `0.0` is a face the horizon can see no sky from; before the term it reads zero and the
        // display transform turns that into black wherever the beam misses it too.
        let (dx_at, dz_at) = stats.3;
        let dark = full.pixel(dx_at, dz_at).map(luminance).unwrap_or(0.0);
        let dark_sky = dome.pixel(dx_at, dz_at).map(luminance).unwrap_or(0.0);
        // **The darkest geometry pixel in the whole frame**, which is the question a player asks:
        // is anything in this picture black? A minimum over the frame rather than a named pixel, so
        // two builds can be compared without having to agree on which pixel they are naming — and
        // the term this pass added is judged on exactly it.
        let darkest = full
            .pixels
            .iter()
            .filter(|pixel| pixel[3] >= 0.5)
            .map(|pixel| luminance(*pixel))
            .fold(f64::INFINITY, f64::min);
        println!(
            "  darkest sky: share {:.3} at ({dx_at}, {dz_at}) — frame {dark:.4}, dome {dark_sky:.4}; \
             {} pixel(s) at a share of exactly 0; darkest geometry pixel in the frame {darkest:.4}",
            stats.0, stats.6,
        );
    }
    // ---- the chunk audit: can a drawn chunk still read black? ----------------------
    audit(&streamer, &table, &rig, eye)?;
    crossings(&mut streamer, &table, &rig, eye)?;
    Ok(())
}

/// **One survey of the record against the chunks a frame would draw.**
///
/// Every count is asked of the bands the frame's own horizon holds, never of the generator: the
/// question is what the light can see, not what the world contains.
#[derive(Default, Clone, Copy)]
struct Survey {
    drawn: usize,
    covered: usize,
    lit: usize,
    /// Drawn, inside the cube, and with no fine record — the number the window prints as
    /// `frame: unlit N of M drawn` and refuses from the picture rather than drawing it.
    unlit: usize,
    /// Sampled coarse cells above a drawn chunk's surface the far band has never looked at.
    far_unstamped: usize,
    /// Sampled coarse cells that stand in an up-ray's way: `share >= FAR_BLOCKS`.
    far_blocking: usize,
    /// Sampled fine cells the cube calls matter where the world has air.
    fake: usize,
    /// Whether the band reports matter in the cell the camera itself stands in.
    eye_solid: bool,
}

/// **The window's own per-frame stamp order, with its own budgets** — the sequence a crossing is
/// issued into. The eye's chunk is stamped first and outside the budget, then the fine cube's
/// twenty-seven and the coarse band's sixty-four, because those are the numbers the window spends
/// (`HORIZON_CHUNKS_PER_FRAME`, `HORIZON_FAR_CHUNKS_PER_FRAME`).
fn stamp_frame(world: &mut Renderer, streamer: &voxel_work::Streamer, eye: [f64; 3]) -> bool {
    const NEAR_BUDGET: usize = 27;
    const FAR_BUDGET: usize = 64;
    let moved = world.world_mut().horizon_recenter(eye);
    let resident: BTreeSet<(i32, i32, i32)> = streamer
        .drawn_meshes()
        .map(|(pos, _, _)| (pos.x, pos.y, pos.z))
        .collect();
    world.world_mut().horizon_retain(&resident);
    let own = chunk_of(eye);
    if let (Some(version), Some(tree)) = (streamer.tree_version(own), streamer.tree_shared(own)) {
        if moved || world.world_mut().horizon_wants(own, version) {
            world.world_mut().horizon_stamp(own, &tree, version);
        }
    }
    let (mut near, mut far) = (NEAR_BUDGET, FAR_BUDGET);
    for pos in streamer.tree_chunks() {
        let Some(version) = streamer.tree_version(pos) else {
            continue;
        };
        if !world.world_mut().horizon_wants(pos, version) {
            continue;
        }
        let budget = if world.world_mut().horizon_covers(pos) {
            &mut near
        } else {
            &mut far
        };
        if *budget == 0 {
            continue;
        }
        if let Some(tree) = streamer.tree_shared(pos) {
            world.world_mut().horizon_stamp(pos, &tree, version);
            *budget -= 1;
        }
    }
    moved
}

/// **What the bands answer about the chunks a frame would draw.**
///
/// Three questions, and the third is the one that turns a chunk black rather than dim:
///
/// * **does a drawn chunk have a record at all** — `horizon_lit` inside the cube, the same bit the
///   window's probe counts; outside it, the coarse cell over the chunk's own surface;
/// * **does either band put matter in the air above a drawn chunk** — a sky ray that stops in the
///   first cells it enters leaves a dome share of `0.0` and no ambient to fall back on;
/// * **is the cell under the camera itself mistaken for rock**, which a crossing is the moment for.
///
/// **The grain is each band's own.** A fine cell is a metre, so the cube is asked at 1, 2, 4 and 8 m
/// above the surface; a coarse cell is eight, so the far band is asked at 8, 16 and 24 — a question
/// at its own resolution rather than one it cannot answer. `FAR_BLOCKS` is `shading.wgsl`'s literal:
/// the share at which an eight-metre cell is in a ray's way.
fn survey(world: &mut Renderer, streamer: &voxel_work::Streamer, eye: [f64; 3]) -> Survey {
    const FAR_BLOCKS: f32 = 0.75;
    let solid = |x: i32, y: i32, z: i32| {
        streamer
            .material_at(x, y, z)
            .map(|material| !material.is_empty())
            .unwrap_or(false)
    };
    let mut out = Survey {
        eye_solid: world
            .world_mut()
            .horizon_solid(eye)
            .is_some_and(|solid| solid),
        ..Survey::default()
    };
    for (pos, _, _) in streamer.drawn_meshes() {
        out.drawn += 1;
        let covered = world.world_mut().horizon_covers(pos);
        if covered {
            out.covered += 1;
            if world.world_mut().horizon_lit(pos) {
                out.lit += 1;
            } else {
                out.unlit += 1;
            }
        }
        let base = (
            pos.x * CHUNK_SIDE as i32,
            pos.y * CHUNK_SIDE as i32,
            pos.z * CHUNK_SIDE as i32,
        );
        for (dx, dz) in [(8, 8), (16, 16), (24, 8)] {
            let (x, z) = (base.0 + dx, base.2 + dz);
            // **The surface is the topmost solid with air above it**, scanning down from the
            // chunk's top: a chunk that is rock all the way through has no surface and is skipped,
            // and a canopy is passed because the cell above it is air only if nothing sits on it.
            let ground = (base.1..base.1 + CHUNK_SIDE as i32)
                .rev()
                .find(|&y| solid(x, y, z) && !solid(x, y + 1, z));
            let Some(ground) = ground else { continue };
            if covered {
                for h in [1.0, 2.0, 4.0, 8.0] {
                    let at = [
                        f64::from(x) + 0.5,
                        f64::from(ground) + h,
                        f64::from(z) + 0.5,
                    ];
                    if world.world_mut().horizon_solid(at) == Some(true) {
                        out.fake += 1;
                    }
                }
            } else {
                for h in [8.0, 16.0, 24.0] {
                    let at = [
                        f64::from(x) + 0.5,
                        f64::from(ground) + h,
                        f64::from(z) + 0.5,
                    ];
                    match world.world_mut().horizon_far_share(at) {
                        None => out.far_unstamped += 1,
                        Some(share) if share >= FAR_BLOCKS => out.far_blocking += 1,
                        Some(_) => {}
                    }
                }
            }
        }
    }
    out
}

/// **The crossing, which is where the user sees the black**: the eye steps one chunk at a time and
/// every step is asked the window's own questions before anything is allowed to settle.
///
/// Two frames per step, because they are two different pictures. **The crossing's own frame** is
/// the window's stamp order with its budgets: a chunk still waiting for its stamp is exactly the
/// chunk the window refuses, and the count printed is that refusal. **The settled frame** is one
/// stamp later, with nothing left wanting: a chunk still unlit there is a chunk that lost its
/// measurement to the move — the defect this section exists to catch.
fn crossings(
    streamer: &mut voxel_work::Streamer,
    table: &[MaterialParams],
    rig: &GpuRig,
    eye: [f64; 3],
) -> Result<(), String> {
    let mut world = Renderer::new(8, 8, table, rig).map_err(|e| e.to_string())?;
    world.world_mut().write_no_sky_model();
    let start = chunk_of(eye);
    println!();
    println!("crossings: one chunk a step, in the window's own stamp order and budgets");
    println!(
        "{:<15} {:>6} {:>7} {:>7} {:>6} {:>8} {:>8} {:>9} {:>5} {:>6}",
        "step",
        "drawn",
        "in cube",
        "stamped",
        "unlit",
        "settled",
        "far none",
        "far block",
        "fake",
        "eye"
    );
    let (mut red, mut steps) = (0usize, 0usize);
    let (mut lost, mut blocked, mut coarse_none, mut fine_fake) = (0usize, 0usize, 0usize, 0usize);
    let (mut moved_eye, mut awaited) = (0usize, 0usize);
    for (step, (dx, dz)) in [(0i32, 0i32), (1, 0), (2, 0), (2, 1)]
        .into_iter()
        .enumerate()
    {
        let centre = ChunkPos::new(start.x + dx, start.y, start.z + dz);
        // **The eye well inside its own chunk**, at the height it was shipped at, so the step is a
        // crossing in x and z and never in y.
        let at = [
            f64::from(centre.x) * f64::from(CHUNK_SIDE) + 16.0,
            eye[1],
            f64::from(centre.z) * f64::from(CHUNK_SIDE) + 16.0,
        ];
        if step > 0 {
            streamer.set_centre(centre);
            streamer.settle().map_err(|e| e.to_string())?;
            steps += 1;
        }
        stamp_frame(&mut world, streamer, at);
        let crossing = survey(&mut world, streamer, at);
        stamp_frame(&mut world, streamer, at);
        let settled = survey(&mut world, streamer, at);
        let (drained, frames) = drain_far(&mut world, streamer, at, settled);
        let label = if step == 0 {
            "spawn".to_string()
        } else {
            format!("cross {step}: x{dx:+} z{dz:+}")
        };
        println!(
            "{label:<15} {:>6} {:>7} {:>7} {:>6} {:>8} {:>8} {:>9} {:>5} {:>6}",
            crossing.drawn,
            crossing.covered,
            crossing.lit,
            crossing.unlit,
            settled.unlit,
            settled.far_unstamped,
            settled.far_blocking,
            settled.fake,
            if settled.eye_solid { "rock" } else { "air" },
        );
        println!(
            "  coarse band: {} cell(s) unstamped at the crossing frame, {} after {frames} more \
             frame(s) of the window's sixty-four a frame",
            settled.far_unstamped, drained.far_unstamped
        );
        if settled.unlit > 0
            || settled.fake > 0
            || settled.far_blocking > 0
            || settled.eye_solid
            || drained.far_unstamped > 0
        {
            red += 1;
        }
        lost += settled.unlit;
        fine_fake += settled.fake;
        blocked += settled.far_blocking;
        coarse_none += drained.far_unstamped;
        moved_eye += usize::from(settled.eye_solid);
        awaited += frames;
    }
    println!(
        "crossings: {steps} step(s); {lost} drawn chunk(s) still unlit after settling, {fine_fake} fine \
         cell(s) called matter in open air, {blocked} coarse cell(s) in an up-ray's way, {coarse_none} \
         cell(s) still unstamped after {awaited} frame(s) of draining, {moved_eye} step(s) with rock \
         under the camera"
    );
    if red == 0 {
        println!("crossings: GREEN — every drawn chunk kept its record across every crossing");
    } else {
        println!("crossings: RED — {red} step(s) drew a chunk the records do not light");
    }
    Ok(())
}

/// **Fill the coarse band until it answers again**, at the window's own sixty-four chunks a frame.
///
/// An unstamped coarse cell is not black: `world_far_share` returns clear (`1.0`) for a cell nobody
/// has looked at, so the far field's silence is a *brighter* chunk and never a darker one. What it
/// costs is the shadowing a distant hillside would have cast, and this is how many frames of the
/// window's own stamping it takes before every sampled cell above a drawn chunk has an answer.
fn drain_far(
    world: &mut Renderer,
    streamer: &voxel_work::Streamer,
    eye: [f64; 3],
    from: Survey,
) -> (Survey, usize) {
    const CAP: usize = 32;
    let (mut frames, mut now) = (0usize, from);
    while now.far_unstamped > 0 && frames < CAP {
        stamp_frame(world, streamer, eye);
        frames += 1;
        now = survey(world, streamer, eye);
    }
    (now, frames)
}

/// The chunk an eye stands in — the same floor `horizon_recenter` takes.
fn chunk_of(eye: [f64; 3]) -> ChunkPos {
    ChunkPos::new(
        (eye[0] / f64::from(CHUNK_SIDE)).floor() as i32,
        (eye[1] / f64::from(CHUNK_SIDE)).floor() as i32,
        (eye[2] / f64::from(CHUNK_SIDE)).floor() as i32,
    )
}

/// **The spawn frame's audit**, before any chunk has moved under the eye: one line of the same
/// counts [`crossings`] prints a row at a time.
fn audit(
    streamer: &voxel_work::Streamer,
    table: &[MaterialParams],
    rig: &GpuRig,
    eye: [f64; 3],
) -> Result<(), String> {
    let mut world = Renderer::new(8, 8, table, rig).map_err(|e| e.to_string())?;
    world.world_mut().write_no_sky_model();
    stamp_frame(&mut world, streamer, eye);
    let first = survey(&mut world, streamer, eye);
    let (at, frames) = drain_far(&mut world, streamer, eye, first);
    println!(
        "audit: {} drawn, {} inside the cube, {} stamped, {} unlit; {} fine cell(s) call open air \
         solid; {} coarse cell(s) in an up-ray's way; {} coarse cell(s) still unstamped after {frames} \
         frame(s) of draining",
        at.drawn,
        at.covered,
        at.lit,
        at.unlit,
        at.fake,
        at.far_blocking,
        at.far_unstamped,
    );
    Ok(())
}

/// Which stances the world is asked for.
#[derive(Clone, Copy, Debug)]
enum Stance {
    /// Air at head height, solid matter overhead.
    Canopy,
    /// Solid matter within three metres on `+x` or `−z`, past head height.
    Wall,
    /// **No sky in any direction the shader's rays go**: a roof overhead and matter within three
    /// metres on all four sides at head height. This is the face a measured share of `0.0` leaves
    /// black, and it is the stance the second-bounce term exists for.
    Enclosed,
}

/// **A stance found in the streamed world**, not one guessed by hand: the first column within a few
/// chunks of the eye that satisfies the stance's own condition, at 1.6 m above its ground.
fn stance(streamer: &voxel_work::Streamer, eye: [f64; 3], want: Stance) -> Option<[f64; 3]> {
    let solid = |x: i32, y: i32, z: i32| {
        streamer
            .material_at(x, y, z)
            .map(|material| !material.is_empty())
            .unwrap_or(false)
    };
    let (cx, cz) = (eye[0] as i32, eye[2] as i32);
    let foot = eye[1] as i32 - 1;
    for ring in (2..=32i32).step_by(2) {
        for dx in (-ring..=ring).step_by(2) {
            for dz in (-ring..=ring).step_by(2) {
                if dx.abs() != ring && dz.abs() != ring {
                    continue;
                }
                // The ground is the first solid cell below the eye's own feet.
                let ground = (foot - 8..=foot)
                    .rev()
                    .find(|&y| solid(cx + dx, y, cz + dz));
                let Some(ground) = ground else { continue };
                let head = ground + 2;
                if solid(cx + dx, head, cz + dz) {
                    continue;
                }
                let stands = match want {
                    Stance::Canopy => (head + 1..=ground + 14).any(|y| solid(cx + dx, y, cz + dz)),
                    Stance::Wall => {
                        let tall = |x: i32, z: i32| {
                            (head..=ground + 6).filter(|&y| solid(x, y, z)).count() >= 4
                        };
                        tall(cx + dx + 1, cz + dz)
                            || tall(cx + dx + 2, cz + dz)
                            || tall(cx + dx, cz + dz - 1)
                            || tall(cx + dx, cz + dz - 2)
                    }
                    Stance::Enclosed => {
                        // **A wall within three metres on each of the four sides, and a roof.** The
                        // shader's two slanted sky rays go along `+x` and `−z` and the vertical one
                        // goes up, so a column walled all round and roofed reads a share of `0.0`
                        // whatever else the world is doing.
                        let wall_at =
                            |x: i32, z: i32| (head - 1..=head + 1).any(|y| solid(x, y, z));
                        let mut sides = true;
                        for (ax, az) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
                            sides &= (1..=3).any(|d| wall_at(cx + dx + ax * d, cz + dz + az * d));
                        }
                        sides
                            && !wall_at(cx + dx, cz + dz)
                            && (head + 1..=head + 6).any(|y| solid(cx + dx, y, cz + dz))
                    }
                };
                if stands {
                    let at = [
                        f64::from(cx + dx) + 0.5,
                        f64::from(ground) + 1.6,
                        f64::from(cz + dz) + 0.5,
                    ];
                    println!(
                        "stance {want:?} at [{:.1}, {:.1}, {:.1}] — ground y {ground}, head y {head}",
                        at[0], at[1], at[2]
                    );
                    return Some(at);
                }
            }
        }
    }
    None
}

/// A camera at `eye` looking along `heading`, level.
fn pose(eye: [f64; 3], heading_deg: f64) -> Camera {
    let mut camera = Camera::standard(eye, f64::from(WIDTH) / f64::from(HEIGHT));
    camera.yaw_rad = heading_deg.to_radians();
    camera.pitch_rad = 0.0;
    camera
}

/// One frame, with the horizon stamped (`Some`) or left empty (`None`).
fn shoot(
    scene: &Scene,
    camera: &Camera,
    table: &[MaterialParams],
    rig: &GpuRig,
    drawn: Option<&Trees>,
) -> Result<Frame, String> {
    let mut renderer = Renderer::new(WIDTH, HEIGHT, table, rig).map_err(|e| e.to_string())?;
    renderer.world_mut().write_no_sky_model();
    // **The cube is anchored on this pose's eye before anything is stamped** — `horizon_recenter`
    // is what writes the `HorizonView` the fragment subtracts, and a frame that stamps without it
    // has every march land outside its own cube, where the answer is `1.0` for lack of a
    // measurement. That is what the first version of this example measured: an empty horizon that
    // read exactly like an unoccluded sky.
    renderer.world_mut().horizon_recenter(camera.position_m);
    if let Some(drawn) = drawn {
        for (pos, version, tree) in drawn {
            renderer.world_mut().horizon_stamp(*pos, tree, *version);
        }
    }
    renderer.render(scene, camera).map_err(|e| e.to_string())
}

/// **The occlusion share over a whole frame**: `stamped / unstamped`, sampled on a stride.
///
/// Returns `(min, p10, median, (x, y) of the minimum, below, sampled, zeros, enclosed median)`. A
/// frame whose sky is
/// entirely open reads `1.000` everywhere, which is as true of a working occlusion path in the open
/// as of a path that was never wired — which is why the obstructed poses are here.
fn shares(stamped: &Frame, bare: &Frame) -> (f64, f64, f64, (u32, u32), usize, usize, usize, f64) {
    let mut values: Vec<(f64, u32, u32)> = Vec::new();
    for y in (0..HEIGHT).step_by(STRIDE as usize) {
        for x in (0..WIDTH).step_by(STRIDE as usize) {
            let (Some(a), Some(b)) = (stamped.pixel(x, y), bare.pixel(x, y)) else {
                continue;
            };
            let (a, b) = (luminance(a), luminance(b));
            if b > 1e-6 {
                values.push((a / b, x, y));
            }
        }
    }
    if values.is_empty() {
        return (0.0, 0.0, 0.0, (0, 0), 0, 0, 0, 0.0);
    }
    values.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("finite"));
    let at = |f: f64| values[((values.len() - 1) as f64 * f).round() as usize];
    let min = values[0];
    // **The enclosed set's own brightness**: every pixel within `0.02` of the frame's floor share —
    // the most enclosed class the view holds — read in the *full* frame. This is the number a
    // second-bounce term has to move, and it is a median over a class rather than a named pixel, so
    // two builds are compared on the same kind of face.
    let floor = min.0 + 0.02;
    let mut enclosed: Vec<f64> = values
        .iter()
        .filter(|v| v.0 <= floor)
        .filter_map(|v| stamped.pixel(v.1, v.2).map(luminance))
        .collect();
    enclosed.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    let enclosed_median = if enclosed.is_empty() {
        0.0
    } else {
        enclosed[enclosed.len() / 2]
    };
    (
        min.0,
        at(0.10).0,
        at(0.50).0,
        (min.1, min.2),
        values.iter().filter(|v| v.0 < 0.90).count(),
        values.len(),
        values.iter().filter(|v| v.0 <= 0.0).count(),
        enclosed_median,
    )
}

/// The named pixel's luminance, or `0` where the frame holds nothing (a pixel outside the image is
/// not a measurement and must not read as one).
fn at(frame: &Frame) -> f64 {
    frame.pixel(AT_X, AT_Y).map(luminance).unwrap_or(0.0)
}

/// Rec. 709 luminance of a uniform triple, the same function the frames are read with.
fn luminance_of(rgb: [f32; 4]) -> f64 {
    luminance(rgb)
}
