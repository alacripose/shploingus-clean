//! `shploingus` — the only place in this repository that opens a window.
//!
//! # What this adds to `voxel-client`, and what it deliberately does not
//!
//! Everything about *what the world looks like* is `voxel-client`'s, and this
//! bin does not restate any of it: the shader, the vertex layout, the depth
//! test, the material table and the Source 2 terms all come from
//! [`voxel_client::WorldPipeline`], which is the **same** object the offscreen
//! test measures. That matters more than it looks: if this file built its own
//! pipeline, the `shading.wgsl` transcription could pass its test against
//! `npr::shade` while the game drew something else, and nothing would say so.
//!
//! What this file adds is a surface to present to, a depth texture that follows
//! the window size, and input.
//!
//! # The streamer runs every frame, and that is the requirement
//!
//! The user's constraint is that exploration and the concurrent terrain shapers
//! are a **constant** part of the world rather than a loading screen. So
//! [`Streamer::pump`] is called on every frame, not once at start-up: the camera
//! moves, the window centre changes, chunks are submitted, meshed on worker
//! threads, evicted and re-submitted — continuously, while the frame loop keeps
//! drawing whatever is already resident. The frame loop never generates and
//! never meshes; that is what `voxel-work` exists for and the numbers in
//! `docs/STATUS.md` are the reason.
//!
//! # The HUD is on screen, and the window title is not the HUD
//!
//! Until this revision the report was the window title, because the workspace
//! had no text renderer. It has one now: [`voxel_client::hud`] composes four
//! panels from [`voxel_ui`]'s rules and draws them with `glyphon` and a two-
//! triangle panel pipeline, and this file only supplies the *values* and the
//! frame's scale factor. The design decisions — which ink step, how tall a row
//! is, whether the text is legible over a white world — are `voxel-ui`'s, and
//! they are checked there and in `voxel-client`'s offscreen measurement rather
//! than by looking at a window.
//!
//! The title keeps the adapter name and nothing else, so a window that is
//! minimised or captured still says which GPU drew it.
//!
//! # Digging and building are edits to the record, not to a mesh
//!
//! Left button **breaks** the voxel in the crosshair, right button **builds**
//! into the empty voxel the ray entered through, and a number key chooses which
//! of the seven admitted materials is placed. Nothing is edited in a mesh: the
//! change goes into the earthworks record ([`voxel_shapers::earthworks`], whose
//! own rule is *an unrecorded modification is invisible*) through
//! [`voxel_work::Streamer::edit`], which invalidates exactly the chunks the
//! mesher's one-voxel pad reaches and lets the workers rebuild them from the new
//! record. A build already in flight when the edit lands finishes with the old
//! world in its hands and is **discarded by generation stamp** rather than drawn
//! — which is what makes an edit safe under the concurrency the module above
//! describes.
//!
//! # The pointer is locked, and look is a device event
//!
//! The pointer's **owner** is decided where the shell's other decisions are: the
//! windows own it while the shell is shown, the world owns it while the shell is
//! hidden. [`App::sync_pointer_grab`] is the one function that reads that decision
//! and applies **both halves of it** — the grab and the cursor's visibility —
//! because the two set in two places is how they drift apart.
//!
//! Look reads [`DeviceEvent::MouseMotion`], the platform's raw relative motion, and
//! **not** `WindowEvent::CursorMoved`. On this platform (winit 0.30.13, Windows)
//! `CursorGrabMode::Locked` clips the cursor to a 1×1 rectangle at its present
//! position, so its reported position stops changing the instant the lock lands —
//! a look that read those positions would freeze exactly when it is needed. Raw
//! motion keeps arriving while the cursor is pinned, and it arrives whether or not
//! the shell is up, so the same ownership rule gates it. `CursorMoved` still feeds
//! the shell's own pointer — hover, drag, hit-testing — so both sources exist and
//! exactly one of them is the camera's.
//!
//! **The clip lands where the cursor already is**, so the lock alone does not put
//! the pointer at the centre of the screen — measured, not assumed: winit's Windows
//! backend builds the clip rectangle from the cursor's present position. The grab
//! therefore moves the cursor to the window's centre **first**, and the 1×1 clip
//! then pins it there. Without that step the pointer is locked to whichever edge it
//! was last dragged to, which is a lock, but not the one the camera is aimed with.
//!
//! # What is honestly missing
//!
//! * **No physics.** Moving is free flight, so the camera can pass through
//!   terrain. There is no gravity and no collision. **The tick does advance** — it is
//!   `voxel_client::clock`'s `Clock`, advanced by the frame's elapsed seconds and
//!   bounded by `MAX_CATCH_UP` — and it drives the sun, so the light on the world is a
//!   function of it. What the tick does **not** drive is the terrain, and not because the
//!   generator ignores it: `WorldSpec::stage_seed` mixes the tick and terrain/caves/geology
//!   read it (measured — `voxel-shapers/examples/tick-over-world.rs`, LEDGER C-117), but
//!   the streamer's world is `World::shipped()` — the (seed, tick 0) world — and nothing
//!   moves its tick, so the voxels are the same at every tick of the client's clock.
//!   Whether the world should age with the clock, and how (a naive tick move *reshuffles*
//!   it), is C-92.0 item 3.
//! * **No interactive control in the HUD.** Every element is a readout. The
//!   design system's target floor and separation rule are enforced by
//!   `voxel-ui`'s own tests and have no drawn consumer yet, which is recorded as
//!   a debt rather than satisfied by drawing a button that does nothing.
//!
//! **The audit, 2026-10-10.** 2 990 lines of window, input and frame orchestration behind **one**
//! test, in the crate that decides what the player sees. Its build script is the gate: it measures the
//! eighteen crates, the record, the disk and the prose ratchet on every build and refuses the build when
//! any of them is wrong. **Nothing measures this file's frame loop**, which is also the missing half of
//! the "random hitches" report. **Owed:** a frame-timing measurement — an instrument, not a guess —
//! because the one thing a player notices immediately is the one thing the gate cannot see.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeSet;
use std::sync::Arc;

mod capture;
use std::time::{Duration, Instant};

use voxel_appearance::sky::SkyInputs;
use voxel_client::shell::notifications::{write_the_gate_queue, NotificationStream};
use voxel_client::shell::{
    cells as shell_cells,
    changes as shell_changes,
    console::{Key as ShellKey, Level as ConsoleLevel},
    debug as shell_debug,
    hud as shell_hud,
    Button as ShellButton,
    Mount as ShellMount,
    WheelDelta as ShellWheel,
};
use voxel_client::scene::Scene;
use voxel_client::{
    Camera, CameraOverhaul, Clock, GpuRig, HudRenderer, HudState, ShadingReport,
    WorldPipeline, DEPTH_FORMAT,
};
use voxel_model::coord::CHUNK_SIDE;
use voxel_model::{ChunkPos, DisplacementField, Face, MaterialId};
use voxel_mesh::{raycast::Ray, RefineConfig};
use voxel_shapers::materials;
use voxel_shapers::stages::Stage;
use voxel_shapers::WorldSpec;
use voxel_stream::WorldSource;
use voxel_work::Streamer;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{DeviceEvent, DeviceId, ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Window, WindowId};

/// **The demo observation's cruise**, in metres a second.
///
/// Flown along the observer's own forward, and slow enough that the window the eye sees is the
/// window the streamer can keep — this is the streaming surface being exercised, not a flight
/// model. The god's own speed is not stated here: a real pilot states its own wish.
const WALK_CRUISE_M_PER_S: f64 = 6.0;

/// **How fast the demo rises while standing still**, in metres a second —
/// `SHPLOINGUS_LIFT_MM_S`, read once, or `0` for the walk the demo has always done.
///
/// The material report is about the **vertical** axis: a slight change of eye height moved
/// the ground's pattern, and a run that also walks forward changes the scene under the
/// camera at the same time, so no pixel can be attributed to the eye. This holds the ghost
/// still and moves it straight up, which leaves the scene, the tick and the light alone and
/// changes exactly one input.
fn walk_lift_m_s() -> f64 {
    static LIFT: std::sync::OnceLock<f64> = std::sync::OnceLock::new();
    *LIFT.get_or_init(|| {
        std::env::var("SHPLOINGUS_LIFT_MM_S")
            .ok()
            .and_then(|text| text.parse::<f64>().ok())
            .map(|mm| mm / 1000.0)
            .unwrap_or(0.0)
    })
}

/// **A held pose** — `SHPLOINGUS_EYE="x,y,z[,pitch_deg]"`, in metres and degrees.
///
/// The demo's walk is frame-rate driven, so the ghost's path — and every defect that depends on
/// where it is — moves between runs; a frame that went black cannot be asked for again by waiting.
/// With a pose pinned the camera is *placed* instead of flown, which makes such a frame reproducible
/// on demand: the same eye, the same heading, the same tick, every time.
fn walk_eye_pin() -> Option<([f64; 3], Option<f64>)> {
    let text = std::env::var("SHPLOINGUS_EYE").ok()?;
    let mut parts = text.split(',').map(|part| part.trim().parse::<f64>());
    let x = parts.next()?.ok()?;
    let y = parts.next()?.ok()?;
    let z = parts.next()?.ok()?;
    let pitch = parts.next().and_then(|part| part.ok());
    if [x, y, z].iter().all(|value| value.is_finite()) {
        Some(([x, y, z], pitch))
    } else {
        eprintln!("shploingus: SHPLOINGUS_EYE must be finite metres, not {text:?}");
        None
    }
}

/// **What a god asks its observer to do this frame**, in metres a second along the observer's own
/// axes — forward, right, up.
///
/// It is the body's `Wish` without a body: no collider answers it, no integrator smooths it, and
/// nothing in the world can refuse it. That is what the god's vantage is, and it is why the demo
/// observation could be written without touching the player at all.
#[derive(Clone, Copy, Debug, Default)]
struct LayaWish {
    forward_m_s: f64,
    strafe_m_s: f64,
    up_m_s: f64,
}

/// **A Laya ghost** — the observer the demo pilots, and the stand-in the doctrine left open.
///
/// A ghost is one of `CIVILIANS.md`'s seven faces and the one it never settled: a being with no
/// body, so it owns no mesh, stands in no collider, and nothing in the world can stop it. That is
/// exactly the thing a god looks through, and it is why the demo does **not** move the player: the
/// player's body is a collider in the world and dragging it along a script would be a body being
/// flown, not a ghost.
///
/// **It is fake in one named sense.** Laya's own runtime does not exist yet — the decider this
/// tree builds is the build-time one — so the wish comes from the demo rather than from Laya. What
/// is *not* faked is the relationship: the ghost is the position, and the frame's camera is read
/// from it, so the thing the audit could not drive is a being standing somewhere rather than a
/// float moving on its own.
struct LayaGhost {
    /// The record's name, so a report can say who was looking.
    name: &'static str,
    /// Where the ghost stands, in the world's metres. This is the record; the camera is a reading
    /// of it.
    position_m: [f64; 3],
}

impl LayaGhost {
    /// **The ghost that stands where the player's eye does at boot**, so a run with no demo is the
    /// run it always was and a run with one begins from the player's own vantage.
    fn at(eye_m: [f64; 3]) -> Self {
        Self { name: "laya-ghost", position_m: eye_m }
    }
}

/// **How long the demo flies**, in seconds — `SHPLOINGUS_WALK_SECONDS`, refused by name rather
/// than defaulted.
///
/// The demo used to be judged by someone watching a window: the counters printed once a second and
/// a reader compared them. A run that states its own length ends with a verdict instead, which is
/// the difference between an observation and a measurement — and the reason this is an input the
/// run takes rather than a number compiled into it.
fn walk_seconds() -> Option<f64> {
    let text = std::env::var("SHPLOINGUS_WALK_SECONDS").ok()?;
    match text.trim().parse::<f64>() {
        Ok(seconds) if seconds.is_finite() && seconds > 0.0 => Some(seconds),
        _ => {
            eprintln!(
                "shploingus: SHPLOINGUS_WALK_SECONDS must be a positive number of seconds, not \
                 {text:?}"
            );
            std::process::exit(2);
        }
    }
}

/// Radians of camera turn per **unit of pointer motion**, as the device reports it.
///
/// The unit is the platform's own — one raw device count here, which at the platform's
/// default pointer speed moves the cursor by one pixel, so the number did not have to
/// move when the source did. It is a **project value** (the sensitivity a mouse should
/// have in this game) rather than a design-system constant, exactly as it was when the
/// source was cursor pixels.
const LOOK_RAD_PER_MOTION_UNIT: f64 = 0.0022;

/// The platform's double-click **interval**, in milliseconds — the most time that may pass
/// between the first click and the second.
///
/// The port needs a click *count* (the title bar's maximize verdict is the double-click's)
/// and winit reports presses with no count, so the count is made here. The number is the
/// platform's own: Windows' `GetDoubleClickTime`, whose default its `SetDoubleClickTime`
/// documentation names as **500 ms**. It is a **platform translation** rather than a project
/// taste — the reference is a Qt app, and a Qt double-click interval is likewise the
/// platform's value — and it is declared rather than read because this program carries no
/// `winapi` dependency and cannot ask the operator's live mouse settings.
const DOUBLE_CLICK_INTERVAL_MS: u128 = 500;

/// The platform's double-click **rectangle**, in pixels — how far the second click may land
/// from the first and still be a pair.
///
/// Windows' `SM_CXDOUBLECLK`/`SM_CYDOUBLECLK`, whose factory value is **4** (`HKCU\Control
/// Panel\Mouse\DoubleClickWidth`/`DoubleClickHeight`). **Pixels, not dp**: the rectangle is
/// input classification rather than layout, and pixels are the unit the platform reports
/// pointer positions in — [`App::cursor`] holds winit's physical position unconverted.
const DOUBLE_CLICK_SLOP_PX: f64 = 4.0;

/// How far the player can reach to break or build, in metres.
const REACH_M: f64 = 6.0;

/// The seven admitted materials, in the order the number keys choose them.
///
/// One list, so a number key, the console line and the placed material cannot
/// disagree about what "3" means.
const PLACEABLE: [(MaterialId, &str); 7] = [
    (materials::REGOLITH, "regolith"),
    (materials::SEDIMENTARY, "sedimentary"),
    (materials::METAMORPHIC, "metamorphic"),
    (materials::IGNEOUS, "igneous"),
    (materials::BEDROCK, "bedrock"),
    (materials::ICE, "ice"),
    (materials::WATER, "water"),
];

/// Which [`PLACEABLE`] slot a number key chooses, if it is one.
fn selection_of(code: KeyCode) -> Option<usize> {
    Some(match code {
        KeyCode::Digit1 => 0,
        KeyCode::Digit2 => 1,
        KeyCode::Digit3 => 2,
        KeyCode::Digit4 => 3,
        KeyCode::Digit5 => 4,
        KeyCode::Digit6 => 5,
        KeyCode::Digit7 => 6,
        _ => return None,
    })
}

/// The outward voxel step of a face — the direction of
/// [`voxel_model::profile::face_vector`], as integer steps so a placed voxel
/// lands exactly on the grid.
fn outward_step(face: Face) -> [i32; 3] {
    let v = voxel_model::profile::face_vector(face);
    [v[0] as i32, v[1] as i32, v[2] as i32]
}

/// Whether a press is the second of the platform's **double-click pair**: the same button,
/// within [`DOUBLE_CLICK_INTERVAL_MS`], inside [`DOUBLE_CLICK_SLOP_PX`] — the platform's two
/// numbers, checked against the raw cursor position they are measured in.
///
/// Time and the previous press are parameters rather than `Instant::now()` and a field, so
/// the running program ([`App::count_shell_click`]) and the test at the end of this file hold
/// the rule to one function.
fn is_double_click(
    button: ShellButton,
    at: (f64, f64),
    now: Instant,
    previous: Option<(Instant, (f64, f64), ShellButton)>,
) -> bool {
    previous.is_some_and(|(then, there, pressed)| {
        pressed == button
            && now.saturating_duration_since(then).as_millis() <= DOUBLE_CLICK_INTERVAL_MS
            && (at.0 - there.0).abs() <= DOUBLE_CLICK_SLOP_PX
            && (at.1 - there.1).abs() <= DOUBLE_CLICK_SLOP_PX
    })
}

// The world's window, ceiling, seed and tick are `voxel-client::game`'s now, not
// this file's. That is not tidiness: an ignored test in `voxel-client` renders
// the shipped view from those constants and measures whether the world actually
// reaches the screen, and it cannot do that against numbers that live inside a
// binary crate. See `docs/LEDGER.md` C-36.
use voxel_client::game::{spawn_eye, spawn_feet, World, GAME_CEILING_BYTES as CEILING_BYTES, GAME_PITCH_RAD};
use voxel_client::motion::Player;
use voxel_shapers::earthworks::Earthworks;
use voxel_client::game::{GAME_TICK as TICK, GAME_WINDOW as WINDOW};

const SHELL_PACKAGE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/alpha-shell.shpkg"));
const PACKAGE_GATE: &str = include_str!(concat!(env!("OUT_DIR"), "/alpha-shell.gate"));

/// **The refinement's declared rate** (C-44): vertices along one edge of a **block's** face.
///
/// `4` is one sample per subvoxel, so a displaced face is drawn at the stored level's own
/// resolution and every cell of the block is a cell of the picture. The addressed level's
/// own 256 is reachable the same way and is refused by **price** rather than by policy:
/// one block face at 256 samples a side is 131 072 triangles, which is a number to say out
/// loud rather than to spend.
const REFINE_SAMPLES_PER_BLOCK: u16 = 4;

/// The most triangles one merged quad may refine to, and it is **derived rather than
/// chosen**: the largest rectangle a chunk can hold is a whole chunk face,
/// [`voxel_mesh::SUBVOXEL_CHUNK_SIDE`] subvoxels a side, and at the rate above that is
/// `2·n²`. A cap below it would refuse a quad the merge can legitimately emit — a refusal
/// with a name, but of something the world really has; a cap above it would be a number
/// nobody ever spent.
const REFINE_MAX_TRIANGLES_PER_QUAD: u32 = 2
    * voxel_mesh::SUBVOXEL_CHUNK_SIDE as u32
    * voxel_mesh::SUBVOXEL_CHUNK_SIDE as u32;

fn main() {
    // Fail before opening the client if its required data package cannot load.
    // The shell will mount these declarations; a build proof is not a mounted UI.
    let shell = match voxel_package::recipe::load(SHELL_PACKAGE) {
        Ok(shell) => shell,
        Err(errors) => {
            eprintln!("shploingus: required shell package refused: {errors}");
            std::process::exit(1);
        }
    };
    println!("{PACKAGE_GATE}");

    // The loaded package, **mounted**. The gate above already printed every window's
    // evaluation; this constructs the shell's window manager from the same declarations, so
    // what is reported here is what the client can actually draw and move.
    // **The workspace root, found before the shell mounts**, because two things need it now:
    // the console's `file` verb (as before) and the judgement the game asks for below.
    let found_root =
        voxel_package::features::workspace_root(std::path::Path::new(env!("CARGO_MANIFEST_DIR")));
    let mut mount = match ShellMount::new(&shell) {
        Ok(mut mount) => {
            // **The workspace root, found rather than assumed** — the same call the build
            // script makes, and the same reason: every path in the record is relative to it,
            // and an assumed root makes them resolve against the wrong place. It is what lets
            // the console's `file` verb reach `chains/workspace.chain`, and **it is not
            // forced**: a binary run from outside a Cargo workspace has no root, and the verb
            // refuses by name rather than filing somewhere nobody named.
            match &found_root {
                Some(root) => mount.set_workspace(root.clone()),
                None => eprintln!(
                    "shploingus: no workspace root above this crate, so the console's `file` \
                     verb will refuse and there is no project for the judge to open; the rest \
                     of the shell is unaffected"
                ),
            }
            // ---- the display package, and the drawing it declares -----------------
            //
            // **The declaration is not resolved here.** The folder is the source `build.rs`
            // proves the embedded bytes equal, and **mounting the package is what reads
            // `package.cells`** — the same loader call a package arriving mid-run makes. A
            // declaration this build does not read refuses the package before it is adopted, so
            // the embedded arrangement and the drawing in force keep drawing and one sentence
            // says where the declaration was written. The version follows the package, not this
            // file: the frame asks the mount for it.
            if let Some(root) = &found_root {
                if let Err(refusal) =
                    mount.load_package(&root.join("display-packages/alpha-shell/package.shspec"))
                {
                    eprintln!("shploingus: {refusal}");
                }
            }
            // **The gate's pass, onto the console — the client's own second reader**
            // (UI-SPEC §9, C-53.3, `BLD-0019`, `BLD-0026`). The line above already printed it
            // to a terminal, and D98 calls a finding that can only be read there incomplete. So
            // the same bytes the build wrote are read onto the console the player is about to
            // look at, **before the window opens** — and a report this reader cannot understand
            // is refused by name rather than shown as zeroes.
            if let Err(why) = mount.read_gate(PACKAGE_GATE, Some(TICK)) {
                eprintln!("shploingus: the gate report could not be read onto the console: {why}");
            }
            // ---- the recipes the record already holds -----------------------------
            //
            // DSL: `queue -> agent "the recipes the game derived from the solutions"`. **The agent
            // authors nothing and the game gives it the recipes it needs** — and the chain already
            // holds work nobody has handed over: filings made before the inbox existed. So the
            // world-open pass reads the board, and for **every open ticket on it** — the ledger's
            // `CSE` cases filed in another world included, which the old world filter left out
            // (C-105) — hands the recipe over: the package on disk if the ticket already emitted
            // one, and a back-compiled one if it did not. New filings are handed over by `file` as
            // they are made.
            //
            // **It runs before the window opens and it is idempotent**: a filing already in the
            // inbox is skipped, because the inbox is work the agent owes and re-emitting it would
            // rewrite the package and the record under the agent's feet. The counts are printed
            // here as well as logged, because this is the line that says whether anything was
            // owed at all.
            if found_root.is_some() {
                let (handed, skipped, refused) =
                    mount.hand_the_boards_tickets_to_the_agent(Some(TICK));
                println!(
                    "shploingus: the agent's inbox: {handed} recipe(s) handed over, {skipped} \
                     already there, {refused} refused"
                );
            }
            // ---- the board, read back ---------------------------------------------
            //
            // DSL: `chain -> board "tickets read back"`. The evaluator pass above takes the whole
            // board — every open ticket is back-compiled into a solution, emitted as a package
            // and handed to the agent's inbox. This read is the naming half: one console line per
            // ticket, in chain order, and the counts printed here — the ledger's `CSE` cases
            // among them, which is what *"the game and laya should fucking catch the goddamn
            // tickets"* means written down.
            if found_root.is_some() {
                let (tickets, here, elsewhere) = mount.read_the_board(Some(TICK));
                println!(
                    "shploingus: the board: {tickets} ticket(s) on the record — {here} filed in \
                     this console's world, {elsewhere} filed elsewhere and caught too"
                );
            }
            mount
        }
        Err(refusal) => {
            eprintln!("shploingus: the shell package cannot be mounted: {refusal}");
            std::process::exit(1);
        }
    };
    // ---- the judgement, on the game's own clock -----------------------------
    //
    // **The queue's production writer, and it is the game.** Before this the only callers of
    // `Evaluation::write_queue` were tests, so `logs/laya-decisions/` was test residue and the
    // record's `game -> decision` and `decision -> queue` arrows had no writer in production.
    // The pass runs once, when the world opens, at the tick the world opens at; the frame loop
    // below then hands the window one record per tick (H-12.2).
    //
    // **A game with no workspace root does not judge**, which is the same refusal the console's
    // `file` verb makes: a binary started outside a Cargo workspace has no project to open, and
    // inventing a subject set for it would be a second judge. It says so rather than pretending
    // the queue is empty — an empty window reads as a clean pass.
    let (notifications, workspace) = match found_root {
        Some(root) => match write_the_gate_queue(&root) {
            Ok(evaluation) => {
                let written = evaluation.decisions.len();
                let cleared = evaluation.decisions.iter().filter(|d| d.cleared()).count();
                let calibration = if evaluation.calibration.calibration.is_quotable() {
                    "quotable"
                } else {
                    "NOT quotable, so no verdict from this run clears anything"
                };
                mount.log(
                    ConsoleLevel::Event,
                    "laya",
                    format!(
                        "the game asked for this run's judgement at tick {TICK}: {written} \
                         decision(s) written to {}, {cleared} cleared, the judge is {calibration}",
                        voxel_model::laya::queue_dir(&root).display()
                    ),
                    Some(TICK),
                );
                match NotificationStream::open(&root) {
                    Ok(stream) => (Some(stream), Some(root)),
                    Err(why) => {
                        eprintln!("shploingus: the queue cannot be streamed: {why}");
                        (None, Some(root))
                    }
                }
            }
            Err(why) => {
                eprintln!("shploingus: the judgement was refused: {why}");
                (None, Some(root))
            }
        },
        None => (None, None),
    };

    let tabs: Vec<&str> = mount
        .bar()
        .iter()
        .filter(|entry| entry.tab)
        .map(|entry| entry.id.as_str())
        .collect();
    let bar_windows: Vec<&str> = mount
        .bar()
        .iter()
        .filter(|entry| !entry.tab)
        .map(|entry| entry.id.as_str())
        .collect();
    println!(
        "shell mounted: {} windows, {} tabs [{}], {} pinned [{}], bar windows [{}]",
        shell.recipe.windows.len(),
        tabs.len(),
        tabs.join(", "),
        shell.recipe.pinned.len(),
        shell.recipe.pinned.join(", "),
        bar_windows.join(", "),
    );
    for window in &shell.recipe.windows {
        println!(
            "  window\t{}\t{}\t{}\t{}",
            window.id, window.title, window.lane.name(), window.content.name()
        );
    }
    println!("  Escape or F1 hides and shows the shell; a pinned window stays (H-9.8)");
    println!("  F3 draws the debug lanes over the shell, and works with the shell closed (H-7.25, H-7.9)");
    println!("  F4 outlines the world meshes' greedy quads over the world (the mesh wireframe)");
    println!("  W/A/S/D walks, Space jumps — hold it to bunnyhop — Shift sprints, Ctrl sneaks");
    println!("  moving is the Squake model; jumping while sneaking with speed trumps — trimping, and it stays on");

    let event_loop = match EventLoop::new() {
        Ok(loop_) => loop_,
        Err(e) => {
            eprintln!("shploingus: cannot create an event loop: {e}");
            std::process::exit(1);
        }
    };
    // Poll rather than Wait: the world is being streamed in by worker threads
    // whatever the input does, so a frame that only wakes on events would show a
    // world that stops arriving whenever the player stops moving.
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::new(mount, notifications, workspace);
    if let Err(e) = event_loop.run_app(&mut app) {
        eprintln!("shploingus: {e}");
        std::process::exit(1);
    }
}

/// The GPU side, created once on `resumed`.
struct Graphics {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    /// **The multisampled attachment the world pass draws into**, or `None` when the
    /// surface format or the adapter cannot carry four samples. The swapchain's own view is
    /// then the pass's target and there is nothing to resolve — which is the frame this app
    /// drew before the count existed.
    colour_msaa: Option<wgpu::Texture>,
    /// How many samples that attachment (and the depth buffer beside it) has. Held so a
    /// resize rebuilds both at the count the pipeline was built with.
    samples: u32,
    depth: wgpu::Texture,
    depth_view: wgpu::TextureView,
    world: WorldPipeline,
    hud: HudRenderer,
    /// How much of the material table can be shaded. See [`ShadingReport`].
    shading: ShadingReport,
    /// Device-independent pixels per physical pixel, as the window reports it.
    /// The HUD's layout is in dp and this is the only place the two meet.
    scale: f32,
    /// What the adapter calls itself, for the title bar and the world panel.
    adapter: String,
    /// **The frame-level recorder**, off unless `SHPLOINGUS_CAPTURE_DIR` names a folder.
    ///
    /// It reads the **swapchain texture the frame was presented from**, after the world and
    /// the shell were drawn into it, so a recorded frame is the frame the player saw and two
    /// recordings with no skip between them are consecutive presented frames — not samples of
    /// a screen being repainted under an external grabber. Each image is named for its index
    /// and every index is written once, with the wall clock and the eye that drew it, to
    /// `frames.tsv` beside the images (`capture::FrameRecorder`).
    capture: Option<capture::FrameRecorder>,
    /// **The piece of ground the corner trace watches** — a world point, fixed by the first
    /// captured frame and then never moved.
    ///
    /// It has to be the world's rather than the eye's: a digest taken around a moving eye counts
    /// different corners every frame simply because the eye moved, which is indistinguishable in a
    /// trace from the geometry stating something different. Fixed here, a line that changes while the
    /// player stands still is the geometry speaking.
    capture_probe: Option<[f64; 3]>,
}

/// **The piece of ground the corner trace watches**, from the frame's own rectangles: the highest
/// upward face below the eye, in the chunk the eye stands in.
///
/// The ghost flies, so a fixed "1.6 m below the eye" is mid-air and a digest taken there counts
/// nothing; and the terrain is drawn from **records**, not corners, so the nearest corner is a
/// refined leaf or a body rather than the ground. A `PosY` record's plane is its chunk origin plus
/// its slice in subvoxels — `SUBVOXEL_CELL_M` each, the same number `Vertex::position` is in — so
/// the highest such plane below the eye is the surface it is standing over.
///
/// `None` while the frame holds no ground under the eye at all: a capture that opens before the
/// world is resident must not fix the trace on a point in mid-air for the rest of the run.
fn ground_probe(scene: &Scene, eye: [f64; 3]) -> Option<[f64; 3]> {
    if std::env::var_os("SHPLOINGUS_PROBE_DEBUG").is_some() {
        let mut faces = [0usize; 6];
        let mut near = 0usize;
        for record in &scene.records {
            faces[(record.face as usize).min(5)] += 1;
            if let Some(origin) = scene.origins.get(record.chunk as usize) {
                if (origin[0] - (eye[0] / 32.0).floor() * 32.0).abs() < 0.5
                    && (origin[2] - (eye[2] / 32.0).floor() * 32.0).abs() < 0.5
                {
                    near += 1;
                }
            }
        }
        // **The highest surface the frame draws in the eye's own column**, above or below it: a
        // plane over the eye's height is the geometry the camera is standing inside.
        let chunk = [
            (eye[0] / 32.0).floor() * 32.0,
            (eye[2] / 32.0).floor() * 32.0,
        ];
        let mut highest = f64::NEG_INFINITY;
        for record in &scene.records {
            if record.face != 3 {
                continue;
            }
            let Some(origin) = scene.origins.get(record.chunk as usize) else {
                continue;
            };
            if (origin[0] - chunk[0]).abs() > 0.5 || (origin[2] - chunk[1]).abs() > 0.5 {
                continue;
            }
            let plane = origin[1] + f64::from(record.slice) * voxel_client::pattern::SUBVOXEL_CELL_M;
            highest = highest.max(plane);
        }
        eprintln!(
            "probe: eye {eye:?} records {} vertices {} origins {} faces {faces:?} near {near} \
             surface {highest:.2}",
            scene.records.len(),
            scene.vertices.len(),
            scene.origins.len()
        );
    }
    let chunk_x = (eye[0] / f64::from(CHUNK_SIDE)).floor() * f64::from(CHUNK_SIDE);
    let chunk_z = (eye[2] / f64::from(CHUNK_SIDE)).floor() * f64::from(CHUNK_SIDE);
    let mut ground: Option<f64> = None;
    for record in &scene.records {
        // `PosY` is the fourth face in `Face::ALL`'s order (`shading.wgsl`'s own comment on the
        // quad record), so `3` is the only value an upward face can carry here.
        if record.face != 3 {
            continue;
        }
        let Some(origin) = scene.origins.get(record.chunk as usize) else {
            continue;
        };
        if (origin[0] - chunk_x).abs() > 0.5 || (origin[2] - chunk_z).abs() > 0.5 {
            continue;
        }
        let plane = origin[1] + f64::from(record.slice) * voxel_client::pattern::SUBVOXEL_CELL_M;
        if plane <= eye[1] && ground.is_none_or(|highest| plane > highest) {
            ground = Some(plane);
        }
    }
    if let Some(plane) = ground {
        return Some([eye[0], plane, eye[2]]);
    }
    // **No rectangle under the eye**, which is the case for a frame whose terrain is refined: every
    // face is corner geometry and the record array is empty. The nearest corner *below* the eye is
    // then the surface the frame draws under it — below, so a leaf hanging above the eye does not
    // win a tie against the ground the frame is looking down at.
    let mut best: Option<([f64; 3], f64)> = None;
    for vertex in &scene.vertices {
        let Some(origin) = scene.origins.get(vertex.chunk as usize) else {
            continue;
        };
        let world = [
            origin[0] + f64::from(vertex.position[0]),
            origin[1] + f64::from(vertex.position[1]),
            origin[2] + f64::from(vertex.position[2]),
        ];
        if world[1] > eye[1] {
            continue;
        }
        let d = [world[0] - eye[0], world[1] - eye[1], world[2] - eye[2]];
        let squared = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
        if best.is_none_or(|(_, b)| squared < b) {
            best = Some((world, squared));
        }
    }
    best.map(|(world, _)| world)
}

impl Graphics {
    /// `boot` is the rig the pipeline is constructed with. **It is a boot value, not
    /// the rig the window draws with** — every frame replaces it through
    /// [`WorldPipeline::write_rig`] before anything is rendered, which is why it may be
    /// the frame's own derivation rather than a fixture.
    fn new(window: Arc<Window>, boot: &GpuRig) -> Result<Self, String> {
        let size = window.inner_size();
        let (width, height) = (size.width.max(1), size.height.max(1));

        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let surface = instance
            .create_surface(window.clone())
            .map_err(|e| format!("cannot create a surface: {e}"))?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
            ..Default::default()
        }))
        .map_err(|e| format!("no adapter for this surface: {e}"))?;
        let info = adapter.get_info();
        let adapter_name = format!("{} ({:?})", info.name, info.backend);

        // **The adapter's own format capabilities, when it has them.** `Rgba32Float` is the
        // offscreen measurement target and its *guaranteed* WebGPU flags carry no multisample
        // bit, so four samples there — and the resolve that makes them a picture — come from
        // this feature. The window's `sRGB` surface needs it for nothing, so an adapter that
        // lacks it (a browser's WebGPU, say) loses only the offscreen antialiasing.
        let mut required = wgpu::Features::empty();
        if adapter
            .features()
            .contains(wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES)
        {
            required |= wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES;
        }
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("shploingus"),
            required_features: required,
            ..Default::default()
        }))
        .map_err(|e| format!("no device: {e}"))?;
        // The device's own errors, printed with wgpu's text. Without this the window only says
        // "a validation error" and the cause is invisible.
        device.on_uncaptured_error(Arc::new(|e: wgpu::Error| {
            eprintln!("shploingus: wgpu: {e}");
        }));

        let caps = surface.get_capabilities(&adapter);
        // An sRGB surface is what makes a linear shaded value arrive on screen
        // as the colour the model computed, so it is preferred over the first
        // format on offer rather than taken by luck.
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);
        // **The presented image is opaque.** Alpha in this program is a
        // *measurement* channel — the sky writes 0 so "geometry is here" stays
        // true (`sky.wgsl`) — and not window transparency. Compositing the
        // surface with any other mode lets a capture read the sky as
        // see-through: measured in OBS, where the sky was invisible until
        // premultiplied alpha was enabled by hand. The offscreen convention is
        // untouched; this is the swapchain's own setting.
        let alpha_mode = caps
            .alpha_modes
            .iter()
            .copied()
            .find(|m| *m == wgpu::CompositeAlphaMode::Opaque)
            .unwrap_or(caps.alpha_modes[0]);
        let config = wgpu::SurfaceConfiguration {
            // `COPY_SRC` is what the frame recorder reads the presented image through; a
            // surface without it can be drawn and presented but not copied.
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // **How many samples this surface is drawn at**, decided once by the same rule the
        // offscreen renderer uses: four where the colour format takes four samples and a
        // resolve, and one otherwise.
        let samples = voxel_client::world_samples(&adapter, required, format);
        let colour_msaa = colour_msaa_target(&device, width, height, format, samples);
        let (depth, depth_view) = depth_target(&device, width, height, samples);

        // The material table and the light rig are `voxel-client`'s, so the
        // window shades exactly what the offscreen test shades. A registry that
        // cannot be described refuses here rather than shading grey.
        let registry = materials::registry().map_err(|e| format!("the material registry: {e}"))?;
        let field = materials::shading_field()
            .map_err(|e| format!("the world's shading field: {e}"))?;
        let table = voxel_client::table(&registry, &field)
            .map_err(|e| format!("a material the appearance layer cannot derive: {e}"))?;
        let mut world = WorldPipeline::new(&device, &queue, format, &table, boot, samples)
            .map_err(|e| format!("the world pipeline: {e}"))?;
        // **The shipped frame draws the beam's shadow**, from the Sun or Moon the rig names.
        // Off in a measurement frame; on in the picture, which is this one. The range is the
        // window's own horizon, so raising `GAME_WINDOW` raises the shadow with it instead of
        // leaving the far chunks on a measurement the light has moved past.
        world.set_shadows(true);
        world.set_shadow_range(
            (voxel_client::game::GAME_WINDOW.radius_chunks + 1) as f32
                * voxel_model::CHUNK_SIDE as f32,
        );
        let shading = ShadingReport::of(&table);
        if !shading.shades() {
            // Not an error: a state. The world is drawn, and drawn black,
            // because the appearance layer refuses to invent a reflectance and
            // no material's record carries one. Said here and on screen.
            println!(
                "shploingus: 0 of {} materials carry a base colour, so only the rim is drawn — the world will be black",
                shading.materials
            );
        }
        // The HUD's panels and glyphs share the swapchain's format, so glyph
        // coverage and the target agree about sRGB.
        let hud = HudRenderer::new(&device, &queue, format);
        let scale = window.scale_factor() as f32;
        // Built before the struct takes ownership of the device and the config.
        let capture = capture::FrameRecorder::from_env(&device, width, height, format);

        Ok(Graphics {
            window,
            surface,
            device,
            queue,
            config,
            colour_msaa,
            samples,
            depth,
            depth_view,
            world,
            hud,
            scale,
            shading,
            adapter: adapter_name,
            capture,
            capture_probe: None,
        })
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        // **Both attachments, at the count the pipeline was built with.** A resize that rebuilt
        // one and not the other would be a pass whose colour and depth disagree about samples,
        // which wgpu refuses on the first frame after the resize.
        self.colour_msaa =
            colour_msaa_target(&self.device, width, height, self.config.format, self.samples);
        let (depth, view) = depth_target(&self.device, width, height, self.samples);
        self.depth = depth;
        self.depth_view = view;
    }
}

/// The multisampled colour attachment the world pass draws into, or `None` at one sample —
/// where the swapchain's own view *is* the pass's target and there is nothing to resolve into.
fn colour_msaa_target(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    samples: u32,
) -> Option<wgpu::Texture> {
    (samples > 1).then(|| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("colour-msaa"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: samples,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
    })
}

fn depth_target(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    samples: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("depth"),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: samples,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

/// Everything the app owns: the graphics, the world, the shell and the player.
struct App {
    graphics: Option<Graphics>,
    streamer: Option<Streamer>,
    /// **The 256 level's physical declaration, and the rate it is refined at** (C-44).
    ///
    /// Both are declared properties of the pipeline in the sense sea level is, so they are
    /// built once here rather than derived per frame: `Scene::build` reads them, and what
    /// comes back is a frame whose planar rectangles stay 24-byte records and whose bent
    /// faces pay vertices. Empty today, because the world is placed as whole voxels and
    /// declaring relief it does not contain is the one thing this layer may not do — so the
    /// stage is wired in and costs nothing, and the first shaper placed at generation time
    /// reaches the picture without a change here.
    displacement: DisplacementField,
    refine: RefineConfig,
    /// The shell's window manager, mounted from the package `build.rs` compiled. It is not an
    /// `Option`: a window with no shell has no bar, no windows and no HUD — the HUD is the
    /// pinned subset of the same set (H-9.8) — so an app without one is an app with nothing
    /// to draw over the world.
    shell: ShellMount,
    camera: Camera,    /// **The god's ghost**, a being with no body — the demo's observer and, one day, Laya's
    /// vantage. It is a separate record from the player rather than a mode of it: a ghost stands
    /// somewhere and is read, a body stands somewhere and is pushed.
    laya: LayaGhost,
    /// The camera effects ported from CameraOverhaul: the smoothing, the sway and the
    /// velocity tilts. **They shape the frame and never the movement** — their offsets
    /// go onto [`App::render_camera`], a copy, while the look the body reads is
    /// [`App::camera`].
    overhaul: CameraOverhaul,
    /// The camera the frame is drawn with: the player's camera plus this frame's
    /// effect offsets. A second camera rather than a mutation, so nothing that reads
    /// the player's look — the movement, the pick's source angles — can see a
    /// smoothed value.
    render_camera: Camera,
    /// The player's body, which **owns** the camera's position.
    ///
    /// The camera's `position_m` is derived from this every frame rather than moved
    /// independently, so the eye can never be somewhere the body is not — which is
    /// the whole defect the floating observer had, stated as a type.
    player: Player,
    /// The world: **a seed, a tick and a configuration held together** (H-7.6), so
    /// the thing a save names is a value rather than three loose fields.
    ///
    /// **Held rather than rebuilt per frame**, which is the whole of the reason this
    /// field is a [`World`] and not a `GAME_SEED` constant: the tick, the sun, the
    /// sky table, the streamed terrain and the body the player is standing on must
    /// all describe **the same world**, and a spec rebuilt from a constant is a
    /// second source of truth waiting to disagree with the first. Its inner spec is
    /// borrowed through [`App::spec`] rather than duplicated, for the same reason.
    world: World,
    /// The record of changes layered over the generated world. The collision queries
    /// it alongside [`App::world`], because a body must stand on the world as
    /// *changed*, not as generated. Empty, because every edit goes through the
    /// streamer's own record instead.
    earthworks: Earthworks,
    held: BTreeSet<KeyCode>,
    /// Which of [`PLACEABLE`] the next build places.
    selected: usize,
    cursor: Option<(f64, f64)>,
    /// The previous press the shell claimed — when, where and with which button — so the next
    /// press can be classified as a double-click. **The one piece of click state the port does
    /// not hold**: the port's `press_at` takes `double` from its caller, and the platform's
    /// interval and rectangle are the constants declared above. A double ends the sequence,
    /// and so does a press the shell does not claim (`App::window_event` clears it), so a dig
    /// can never pair with a window click.
    last_shell_click: Option<(Instant, (f64, f64), ShellButton)>,
    last: Instant,
    frames: u32,
    /// When the current reporting interval started, so a rate is measured over a
    /// period rather than extrapolated from one frame.
    title_at: Instant,
    /// The last completed measurement, or `None` before the first interval ends.
    fps: Option<f64>,
    workers: usize,
    /// The last time the shell refused to lay out, so a refusal that persists is
    /// printed once rather than sixty times a second.
    shell_refusal: Option<String>,
    /// Whether the debug lanes are drawn. **Off until asked for**: a test surface nobody turned on
    /// is not a default, and the lanes are claims about the client rather than part of it.
    lanes: bool,
    /// Whether the world meshes are outlined as a wireframe over the shaded world — the greedy
    /// merge's own rectangles, read from [`Scene::quad_edge_bytes`]. **Off until asked for**,
    /// like the lanes: it is a diagnostic view of how the meshes are made, not part of the
    /// frame the game draws.
    wireframe: bool,
    /// **Which of the material's layers the world pass draws** — `0` is the shaded picture and
    /// `1..=4` are `shading.wgsl`'s own views (`WorldPipeline::set_debug_layer`): the stretch axis a
    /// face's pattern is shifted along, what each corner states of its own, the ladder's rung and
    /// share, and the footprint it is measured at. **Off until asked for**, like the two views
    /// above: it is a diagnostic picture of the material and not part of the frame the game draws.
    layer: u32,
    /// **The live change regions the vector view draws, read from the world's record.**
    ///
    /// Not remembered at the edit sites: feature `change_view` is *a view over the chain, not a
    /// second record* (C-56), so the regions are rebuilt from the streamer's modification record.
    /// `change_generation` is the record's generation the regions were read at — a frame with no
    /// edit does not copy the record — and `changes_seen` says how many of its entries the regions
    /// cover (the record is append-only, so the new ones are the ones past it).
    changes: Vec<shell_changes::ChangeRegion>,
    change_generation: u64,
    changes_seen: usize,
    /// **The corner refusals the report has already logged.** The pipeline counts *distinct*
    /// refusals, and this is how each one reaches stderr once rather than once a second.
    corner_refusals_seen: u64,
    /// The world's clock, **advanced by the frame loop** rather than held at a constant.
    ///
    /// It was [`TICK`] — a `const 0` — for as long as nothing simulated. The reason
    /// recorded against it was *"a HUD that showed a moving tick would be reporting a
    /// simulation this build does not have"*, and that reason is now false in exactly
    /// the way `voxel_client::clock`'s own module doc predicted: the tick drives the
    /// sun, so the simulation this build has is a **lighting** one.
    clock: Clock,
    /// **The cadence cursor**: the production consumer of `voxel_work::Scheduler`.
    ///
    /// The clock says *when* a tick happens; this says **what is due on it**, from the
    /// one place the ladder is declared. Before it existed the client restated the
    /// ladder per site (`Stage::Atmosphere.is_due_at(tick)` in the frame loop), which is
    /// a second copy of the schedule — and a second copy is the thing that drifts.
    tick_driver: voxel_work::TickDriver,
    /// **The process roster, driven from the running world.**
    ///
    /// `voxel_shapers::process::ProcessScheduler` planned by cadence and had **no caller
    /// outside an example** (`docs/MISSING-SYSTEMS.md` §1). This is that caller: the roster
    /// is stepped once per tick at the eye's own unit and neighbourhood, which is what turns
    /// it from a library into a consumer the running program drives.
    process: voxel_shapers::process::ProcessScheduler,
    /// The tick the roster last stepped, so it runs **once per tick** rather than once per
    /// frame. [`voxel_shapers::process::ProcessScheduler::step`] is idempotent within a tick
    /// and refuses a rewind, so a second call is a no-op and an older one is a refusal — but
    /// a frame-rate call would still be a report about the client rather than the world.
    process_at: Option<u64>,
    /// The last ran/refused summary printed, so a set that has not changed is not a report.
    process_refusals: Option<String>,
    /// The tick the light was last derived at, **or `None` before the first frame**.
    ///
    /// This exists to honour [`voxel_shapers::stages::Stage::Atmosphere`]'s declared
    /// cadence. H-8.3: *"a stage's period is part of the determinism contract, not a perf
    /// knob"*, and the ladder puts the atmosphere on `Cadence::Water` — **every 4 ticks**.
    /// The first version recomputed the rig on **every frame**, which was three kinds of
    /// wrong at once: it contradicted the schedule, it spent a 96-day solar search plus a
    /// `Sky::at` per frame, and it made the world's light a function of the frame rate
    /// rather than of the tick.
    rig_at_tick: Option<u64>,
    /// **The exposure grade the frame's light field is read at** — the `Daylight`'s own, kept
    /// beside the field for the same reason the field is kept: the measurement is physical and the
    /// grade is the frame's, so the two meet once, where the measurement becomes a vertex
    /// ([`voxel_client::skylight::exposed_bands`]). `1.0` until the first `Daylight` lands, which is
    /// the grade a boot frame's own sky is drawn at.
    light_grade: f64,
    /// **The world's own light, per cell, as the frame reads it** ([`voxel_client::CellField`]).
    ///
    /// The rig is a four-direction toy light; this is the measurement. It is rebuilt on the
    /// atmosphere's cadence — the same tick the rig is — and **kept between cadences**, because a
    /// shadow march is hundreds of octree spans and re-marching the visible world every frame would
    /// make the picture's cost the frame rate's. `None` while the atmosphere is refused, which is
    /// the rig's own refusal and not a second one: a world with no sky has no sky light to measure,
    /// and the rig it falls back to is the light this game was lit by before the field existed.
    ///
    /// **A `RefCell` because a corner is where the measurement attaches**: `Scene::build_with_light`
    /// asks a `Fn` for one cell at a time, and the field's own answers are a `&mut` cache. The
    /// borrow is taken and dropped inside that one cell's lookup, so no `&mut` is held across the
    /// scene build.
    light_field: Option<std::cell::RefCell<voxel_client::CellField>>,
    /// **The measurement's own thread** ([`voxel_client::Measurer`]).
    ///
    /// The marches are what the field's numbers are made of, and they do not happen here: the
    /// worker marches the cells this frame asked for, the frame adopts what has landed, answers
    /// the worker's tree questions by handle, and asks for the next batch when it is idle. The
    /// draft this replaced spent those marches **on the frame** — up to 1 024 of them on a rewrite
    /// frame, about ten milliseconds, ten frames a second while the field filled — which is the
    /// one cost a frame held to 8.1 ms cannot carry.
    measurer: Option<voxel_client::Measurer>,
    /// **The beam the ask is composed under**: the spec, the cadence's tick and the sky's inputs,
    /// as the atmosphere last stated them. The cells change every frame; the beam does not.
    light_ask: Option<voxel_client::Ask>,

    /// **The light's own cadence, as the geometry sees it.**
    ///
    /// A rebuild is 228 000 corner asks, nine megabytes of vertices to encode and a buffer to send:
    /// measured at **30 ms a build** in this build of the tree, against a frame held to 8.1 ms. So
    /// the picture's light moves on the **beam's** own step — [`voxel_client::BEAM_STEP_DEG`],
    /// about 1.7 s of this world's clock — and not on the atmosphere's cadence (five times a second,
    /// which measured 12.1 ms a frame). The field behind it is refreshed continuously on its own
    /// thread; this is only how often the world is re-written with it, and it is the field's
    /// declared resolution rather than a compromise with the clock.
    light_epoch: u64,
    /// The beam direction the geometry was last written under, so a cadence that moved the sun by a
    /// sixteenth of a step does not rewrite nine megabytes for it.
    light_built_under: Option<[f64; 3]>,
    /// The field's own counters, summed since the last report: cells measured, corners answered
    /// from the world's own light, corners a step behind it, corners rigged, and hops past a chunk
    /// the streamer does not hold. Reset when they are printed.
    light_totals: [u64; 5],
    /// **The frame's terrain geometry, and the key it was built under.**
    ///
    /// A corner is chunk-local and the eye travels in the origins array, so the world's
    /// rectangles and corners are the same bytes while the world and its light are. The cache
    /// states exactly that: it builds when the key moves and keeps the scene otherwise, and
    /// because a kept scene carries the build id the GPU last uploaded, neither the scene walk
    /// nor the buffer upload happens on a frame that walks through a world that has not changed.
    resident: voxel_client::ResidentScene,
    /// **The frame's own work, and the one place that decides how much of it happens**: a slice of
    /// quads, a slice of upload bytes and a slice of cells every frame, each independent of the
    /// others. A chunk arriving used to mean a full rebuild and a 20 MB upload on one frame — 70 ms
    /// and 143 ms under eight busy cores — because both were all-or-nothing.
    work: voxel_client::FrameWork,
    /// **What the last residency sync did**, for the report: what landed, what left, what was
    /// refreshed and what is still waiting for a frame's budget.
    residency: voxel_client::Residency,
    /// **The demo walk**: the camera is driven forward and the frame records whether the chunk it
    /// stands in was drawn in the frame it became resident. Off unless `SHPLOINGUS_WALK` is set.
    walk: bool,
    /// **Set when the walk first moves** — `walk` and a settled window — so the boot's empty world
    /// is not counted as frames in which the eye's chunk was missing from the scene.
    walk_started: bool,
    walk_frames: u64,
    /// Whether the demo has made its one **hop** — the step that puts the eye in ground the
    /// streamer has not meshed yet, which is the only way a settled window can produce the frame
    /// this change is measured on: a camera standing in a chunk on the frame it becomes resident.
    walk_hopped: bool,
    /// Frames where the eye's chunk was in the streamer and **not** in the scene that was drawn.
    walk_missed: u64,
    /// **Frames where the eye's chunk arrived and was drawn in the same frame**: it was in the
    /// streamer, absent from the scene when the frame began, and held by the scene that frame
    /// drew. This is the claim in its own units — not "nothing was missed", but "every arrival
    /// under the eye arrived drawn".
    walk_landed: u64,
    /// Frames where the eye's chunk was in the drawn scene's reach at all — a chunk still being
    /// meshed is the streamer's latency and not the scene's, and the two are counted apart.
    walk_absent: u64,
    /// The four counts above **since the ghost first flew**, so a bounded run's verdict is the
    /// whole run rather than its last second: `[frames, landed, missed, absent]`.
    walk_totals: [u64; 4],
    /// When the ghost first flew, so a demo run can time itself.
    walk_since: Option<Instant>,
    /// **How long the demo flies**, in seconds — `SHPLOINGUS_WALK_SECONDS`, or `None` for as long
    /// as the window is open. A bounded run is one command and one verdict, which is what makes
    /// the claim checkable instead of watchable.
    walk_for: Option<f64>,
    /// **What the geometry path did since the last report**, as the numbers this change is
    /// argued with: scenes built, corner uploads made (and bytes), and frames that sent nothing
    /// with the bytes they spared. Reset when they are printed.
    geometry: GeometryTally,
    /// **The astronomical bodies the frame draws** (C-184), as records in the world's own units.
    ///
    /// They are held here rather than written into a uniform because a body is **geometry**: the same
    /// array a chunk's rectangles travel in, drawn between the terrain and the cloud, with the phase
    /// coming out of the pixels (`fs_body`'s sphere law). Written from the same `Daylight` the rig and
    /// the sky model are, so the light, the sky and the body in it cannot describe different skies — and
    /// **kept when a tick derives no daylight**, which is `write_rig`'s own hold rather than a black
    /// frame.
    bodies: Vec<voxel_client::quad::BodyRecord>,
    /// **The cloud's own derivation, on its own thread** ([`CloudWorker`]).
    ///
    /// The field and the tree behind it are 4 225 model asks and an octree fill, both pure
    /// functions of the key below. They used to be derived *by the frame that drew them*, every
    /// four ticks, and measured 31–60 ms of every frame with stalls to 389 ms — the stutter the
    /// player reported. The frame now asks and adopts; the cadence, the inputs and the values are
    /// unchanged.
    cloud_worker: CloudWorker,
    /// **The atmosphere's own derivation, one cadence ahead** ([`voxel_client::AtmosphereAhead`]).
    ///
    /// The sky, the rig, the field's solver, the dome and the two bodies are one pure function of
    /// `(spec, tick, inputs, eye)` and cost **19 ms** — `sky-light 14.4 | sky-dome 4.3` on the
    /// shipped binary's cadence frames, against a frame that otherwise draws in 10–11 ms. The next
    /// cadence is known four ticks early, so it is derived on a thread while four frames draw and
    /// the frame that is due **adopts** it. A cadence whose eye or inputs moved since the ask is not
    /// adopted — it is derived here, exactly as before, and counted.
    atmosphere_ahead: voxel_client::AtmosphereAhead,
    /// **The cloud tree in hand, and the key it answers.**
    ///
    /// A tree is a function of the field's tick, of the eye **snapped to the cloud domain's own
    /// sample lattice** ([`voxel_client::CLOUD_SAMPLE_M`]) and of the pixel angle — not of the
    /// frame. Rebuilding it per frame is what made the first octree version the cost it was: a
    /// measured **1,658 ms** of the frame's own time, for a picture whose rectangles were already
    /// cheaper than the lattice it replaced. So it is held the way the rig and the sky table are
    /// held, and rebuilt only when something it is a function of actually moves — and `None` until
    /// the first tree is adopted, which is why a first frame draws no cloud rather than an
    /// invented one.
    cloud_tree: Option<voxel_client::CloudCubes>,
    /// **The tree the one above is replacing**, held for one cadence so the sky can be a *join* of
    /// two derivations rather than a swap between them.
    ///
    /// A tree is asked for once per cloud cadence — [`voxel_client::cloud_cubes::cadence_ticks`],
    /// the ladder's `Cadence::Sediment` — and drawn every frame, so the picture steps when a new
    /// derivation lands rather than continuously: visible as a cloud that pops at the moment a
    /// cell's water crosses the cut, and as a whole sky that jumps when the eye crosses a sample
    /// cell or the rung changes. This is the outgoing half of that step. It is **one** tree and
    /// never a queue:
    /// the picture is the two most recent derivations, and an older one is a staler sky rather than
    /// a third share of the light. The weights are read from the world's own ticks
    /// ([`voxel_client::cloud_cubes::fade_weight`]) so a dropped frame cannot change the fade.
    cloud_tree_previous: Option<voxel_client::CloudCubes>,
    /// **The tick the tree in hand became the tree in hand** — the dissolve's own anchor.
    ///
    /// It used to be the key's tick (the atmosphere's cadence tick), and that was wrong in the
    /// two cases the doc above names. The key moves for reasons that are *not* the tick — the eye
    /// crossing a sample cell or the rung changing — and those derivations carry the **same**
    /// cadence tick as the tree they replace. Read from that tick, the incoming tree's weight was
    /// already what its predecessor's had reached, so it arrived at full weight on the frame it
    /// landed: the cross-fade existed in the shader and never once ran. Measured from the adoption
    /// instead, a tree always starts its dissolve at zero, and the dissolve ends as the next
    /// derivation falls due — which is the phase lock the cadence was for, now kept even when the
    /// worker is a tick or two behind.
    cloud_tree_at_tick: Option<u64>,
    /// What the tree in hand was built from: the field's own tick, the eye snapped to
    /// [`voxel_client::CLOUD_SAMPLE_M`] on each axis, and the pixel angle's own bits.
    cloud_tree_key: Option<CloudKey>,
    /// The sky's **turbidity**, sampled on [`voxel_shapers::stages::Stage::Weather`]'s
    /// cadence — `Cadence::Water`, every 4 ticks — and read from **the stage's rate**:
    /// the onset layer's answer, so a rain spell hazes the air it is falling through
    /// rather than only a climatologically wet world being hazier forever (C-94.7).
    ///
    /// **Held separately from the ozone below because the two stages are not in step.**
    /// Weather runs ×4 and climate runs `Cadence::Daily`, so a single "recompute the sky
    /// when the atmosphere is due" would drag the ozone along at the weather's rate and
    /// make a daily quantity vary four times a second. Each field is sampled on the
    /// cadence of the stage that owns it, and held between.
    turbidity_at: Option<u64>,
    /// The last sampled turbidity, held between weather cadences.
    turbidity: f64,
    /// The sky's **ozone depth**, on [`voxel_shapers::stages::Stage::Climate`]'s cadence —
    /// `Cadence::Daily`, once a world day.
    ozone_at: Option<u64>,
    /// The last sampled ozone depth, held between climate cadences.
    ozone_atm_cm: f64,
    /// **The judgement this run asked for, streamed at the world's tick rate.**
    ///
    /// `None` when there was no workspace root to judge, or when the pass or the queue was
    /// refused — never an empty stream standing in for a pass, because a window with no records
    /// and a window whose records were all refused are different facts.
    notifications: Option<NotificationStream>,
    /// Where the queue was written, so each record's artifact can be re-hashed before it is
    /// shown: a decision is shown against the bytes it was made from or not at all.
    workspace: Option<std::path::PathBuf>,
    /// The tick the last notification was shown at, so the stream advances **once per tick**
    /// rather than once per frame (H-12.2: one rendering per record, one at a time).
    notified_at: Option<u64>,
    /// How many decisions this run has shown.
    notifications_shown: usize,
    /// How many of the console's records have already been printed to this process's output.
    ///
    /// The console is the api layer's record surface (C-54.3) and it is the player's window; this
    /// counter is how the agent reads the same records without opening the window, so the live-edit
    /// loop has a read as well as a write.
    console_at: usize,
    /// How many records the console had dropped when this read last re-anchored.
    console_dropped: usize,
    /// The last shell-drawing reading logged, so it is a record **when it changes** rather than
    /// once per frame: a number that is the same every frame is a noise floor, not a report.
    /// **The frame-timing instrument**, off unless `SHPLOINGUS_FRAMES` asks for it
    /// ([`FrameTiming`]). It is held here rather than in a side table so that a frame's own
    /// report and the frame loop's own state share one owner.
    timing: FrameTiming,
    drawing: Option<String>,
}

/// **The frame's own cost, split by what the frame loop does.**
///
/// This file's audit (2026-10-10) ends with the debt this pays: *"Nothing measures this
/// file's frame loop… **Owed:** a frame-timing measurement — an instrument, not a guess."*
/// The sections are the frame loop's own steps **in the order it runs them**, and each is
/// timed around the code the running program executes rather than around a copy of it.
#[derive(Clone, Copy, Default)]
struct FrameCost {
    /// The clock, the shell's tick, and the process roster — one step per tick.
    roster: Duration,
    /// The atmosphere inputs, `Daylight` derived and the rig written: the solar search and
    /// the light's own upload.
    sky_light: Duration,
    /// **The dome table's 4 096 directional radiance evaluations**, and the two uploads that
    /// follow them.
    sky_dome: Duration,
    /// The judgement stream and the live-reload package watch. *(Not the judge: `voxel-von`
    /// ran in the gate pass, before the window opened.)*
    judge: Duration,
    /// The console's read-back onto this process's own output.
    api: Duration,
    /// The player's step, and the drawn eye placed from it.
    walk: Duration,
    /// The streamer's pump: what it asks for, takes, refuses and evicts.
    stream: Duration,
    /// **Every resident chunk's rectangles** copied into the frame's own record array, and the
    /// ask that hands the cloud's own derivation to its worker. *(The cloud field and its tree
    /// are no longer in this split: they are not this thread's work — see [`CloudWorker`].)*
    ///
    /// **And, when the world's light field is live, this is where every visible corner is
    /// measured**: the shadow marches run inside the scene build, because a corner is the address a
    /// per-cell measurement attaches to. So this step carries the field's cost as well as the
    /// geometry's, and the field's own counters are printed beside the frame-rate line rather than
    /// invented here.
    scene: Duration,
    /// The water query, the camera effects and the change view's own projection.
    camera: Duration,
    /// The swapchain image and the shell: the panels, the lanes, the HUD's preparation.
    shell: Duration,
    /// The passes, the submit and the present — the world's draw, the HUD's, and the wait.
    draw: Duration,
    /// The whole frame, from its first clock read to its present.
    total: Duration,
}

impl FrameCost {
    /// One duration, in milliseconds, as every line of this report prints it.
    fn ms(d: Duration) -> f64 {
        d.as_secs_f64() * 1000.0
    }

    /// The split as one line: **every section**, so a report is the whole of it rather than
    /// the interesting half of it.
    fn report(&self) -> String {
        format!(
            "roster {:.2} | sky-light {:.2} | sky-dome {:.2} | judge {:.2} \
             | api {:.2} | walk {:.2} | stream {:.2} | scene {:.2} \
             | camera {:.2} | shell {:.2} | draw {:.2}",
            Self::ms(self.roster),
            Self::ms(self.sky_light),
            Self::ms(self.sky_dome),
            Self::ms(self.judge),
            Self::ms(self.api),
            Self::ms(self.walk),
            Self::ms(self.stream),
            Self::ms(self.scene),
            Self::ms(self.camera),
            Self::ms(self.shell),
            Self::ms(self.draw),
        )
    }
}

/// **What the geometry path did in one reporting second.**
///
/// Two numbers the frame already knows and one the pipeline keeps: a scene is *built* only when
/// the world or its light moved, and its corners are *sent* only when the build is new. A second
/// of walking through a settled world is `builds == 0, uploads == 0` and every frame in
/// `spared`, with `spared_bytes` the upload the old per-frame path would have made.
#[derive(Default)]
struct GeometryTally {
    builds: u64,
    frames: u64,
    spared_frames: u64,
    spared_bytes: u64,
    uploads: u64,
    upload_bytes: u64,
    /// The last build id seen, so a kept scene is told from a rebuilt one.
    last_build: u64,
    /// What the residency syncs did: chunks appended, dropped and light-refreshed.
    landed: u64,
    left: u64,
    refreshed: u64,
}

impl GeometryTally {
    /// Fold one taken scene into the tally: a new build id is a build.
    fn scene_taken(&mut self, build: u64) {
        if build != self.last_build {
            self.builds += 1;
            self.last_build = build;
        }
        self.frames += 1;
    }

    /// Fold one residency sync into the tally.
    fn residency(&mut self, residency: voxel_client::Residency) {
        self.landed += residency.landed as u64;
        self.left += residency.left as u64;
        self.refreshed += residency.refreshed as u64;
    }

    /// Take the pipeline's own ledger, once a second, and hand it back as a line.
    fn report(&mut self, traffic: voxel_client::CornerTraffic) -> String {
        let uploads = traffic.uploads - self.uploads;
        let bytes = traffic.bytes - self.upload_bytes;
        let spared = traffic.spared_frames - self.spared_frames;
        let spared_bytes = traffic.spared_bytes - self.spared_bytes;
        let line = format!(
            "{} frame(s) | {} scene change(s), {} landed / {} left / {} light-refreshed | {} \
             corner upload(s), {:.2} MB | {spared} frame(s) sent nothing, {:.2} MB not sent",
            self.frames,
            self.builds,
            self.landed,
            self.left,
            self.refreshed,
            uploads,
            bytes as f64 / (1024.0 * 1024.0),
            spared_bytes as f64 / (1024.0 * 1024.0),
        );
        self.uploads = traffic.uploads;
        self.upload_bytes = traffic.bytes;
        self.spared_frames = traffic.spared_frames;
        self.spared_bytes = traffic.spared_bytes;
        self.builds = 0;
        self.frames = 0;
        self.landed = 0;
        self.left = 0;
        self.refreshed = 0;
        line
    }
}

/// **The frame-timing instrument** (`SHPLOINGUS_FRAMES`), which is the audit's owed report.
///
/// Off by default, and **off costs one boolean test per frame**: a player who did not ask for a
/// report does not get one, and an instrument that charges for existing would be measuring
/// itself. Turned on, it prints a line for every frame longer than the stall threshold and
/// folds the mean split into the frame-rate line the loop already prints once a second.
#[derive(Default)]
struct FrameTiming {
    /// Whether `SHPLOINGUS_FRAMES` turned it on. The only field read when it is off.
    on: bool,
    /// What counts as a stall, in milliseconds: `SHPLOINGUS_STALL_MS`, defaulting to one 20 Hz
    /// tick's own period, so a frame that misses a tick says so rather than averaging away.
    stall_ms: f64,
    /// Frames drawn since the last summary.
    frames: u64,
    /// Of those, how many were over the threshold.
    stalls: u64,
    /// The worst frame **of the run**, and the tick it was drawn at.
    worst: Duration,
    worst_at: u64,
    /// The sections' own sums since the last summary.
    sum: FrameCost,
}

impl FrameTiming {
    /// Read the two variables. A malformed threshold leaves the instrument **off and says
    /// why**, because a report made against a threshold the caller did not choose is not
    /// their report.
    fn from_env() -> Self {
        let mut timing = FrameTiming { stall_ms: 25.0, ..FrameTiming::default() };
        match std::env::var("SHPLOINGUS_FRAMES") {
            Ok(text)
                if !matches!(text.trim(), "" | "0" | "off" | "false") =>
            {
                match std::env::var("SHPLOINGUS_STALL_MS") {
                    Err(_) => {}
                    Ok(text) => match text.trim().parse::<f64>() {
                        Ok(ms) if ms.is_finite() && ms > 0.0 => timing.stall_ms = ms,
                        other => {
                            eprintln!(
                                "shploingus: SHPLOINGUS_STALL_MS must be a positive number of \
                                 milliseconds, not {other:?}; the frame timer stays off"
                            );
                            return timing;
                        }
                    },
                }
                timing.on = true;
                println!(
                    "shploingus: SHPLOINGUS_FRAMES is on — a line for every frame over {:.0} ms, \
                     and the mean split on the frame-rate line",
                    timing.stall_ms
                );
            }
            _ => {}
        }
        timing
    }

    /// Take one drawn frame: report it if it stalled, and fold it into the running sums.
    fn take(&mut self, tick: u64, cost: &FrameCost) {
        if !self.on {
            return;
        }
        self.frames += 1;
        self.sum.roster += cost.roster;
        self.sum.sky_light += cost.sky_light;
        self.sum.sky_dome += cost.sky_dome;
        self.sum.judge += cost.judge;
        self.sum.api += cost.api;
        self.sum.walk += cost.walk;
        self.sum.stream += cost.stream;
        self.sum.scene += cost.scene;
        self.sum.camera += cost.camera;
        self.sum.shell += cost.shell;
        self.sum.draw += cost.draw;
        self.sum.total += cost.total;
        if cost.total > self.worst {
            self.worst = cost.total;
            self.worst_at = tick;
        }
        if FrameCost::ms(cost.total) >= self.stall_ms {
            self.stalls += 1;
            println!(
                "stall\tt{tick}\t{:.1} ms — {}",
                FrameCost::ms(cost.total),
                cost.report()
            );
        }
    }

    /// The summary for the frame-rate line: **the mean split**, the worst frame of the run
    /// and how many of the interval's frames were stalls. `None` when it is off or when no
    /// frame was drawn, so an idle window reports nothing rather than a zero.
    fn summary(&mut self) -> Option<String> {
        if !self.on || self.frames == 0 {
            return None;
        }
        let n = self.frames as f64;
        let mean = FrameCost {
            roster: self.sum.roster.div_f64(n),
            sky_light: self.sum.sky_light.div_f64(n),
            sky_dome: self.sum.sky_dome.div_f64(n),
            judge: self.sum.judge.div_f64(n),
            api: self.sum.api.div_f64(n),
            walk: self.sum.walk.div_f64(n),
            stream: self.sum.stream.div_f64(n),
            scene: self.sum.scene.div_f64(n),
            camera: self.sum.camera.div_f64(n),
            shell: self.sum.shell.div_f64(n),
            draw: self.sum.draw.div_f64(n),
            total: self.sum.total.div_f64(n),
        };
        let out = format!(
            "mean {:.1} ms of {} frame(s) — {} | {} stall(s) over {:.0} ms | worst {:.1} ms at t{}",
            FrameCost::ms(mean.total),
            self.frames,
            mean.report(),
            self.stalls,
            self.stall_ms,
            FrameCost::ms(self.worst),
            self.worst_at,
        );
        self.frames = 0;
        self.stalls = 0;
        self.sum = FrameCost::default();
        Some(out)
    }
}

/// **The cloud derivation's own address:** the field's tick (floored to the cloud's own cadence —
/// [`cloud_tick_of`]), the eye snapped to the cloud domain's sample lattice
/// ([`voxel_client::CLOUD_SAMPLE_M`]), and the pixel angle's own bits. The same key the frame loop
/// already used to decide whether the tree had to be rebuilt, so a key still yields exactly one tree.
type CloudKey = (u64, [i64; 3], u32);

/// **The cloud's own derivation, off the frame thread.**
///
/// The cloud field is the model asked once per sample — `CLOUD_SAMPLES_PER_AXIS²` = **4 225**
/// calls to `clouds::cloud_column`, each a terrain, climate, weather and solar evaluation — over a
/// 8 192 m span, and the tree behind it is an octree fill over that field. Both used to run **on
/// the frame thread, every four ticks**, and `SHPLOINGUS_FRAMES=1` measured what that costs: the
/// field **11–21 ms of every frame and 44–139 ms on a cadence frame**, the tree **20–39 ms mean and
/// up to 243 ms**, on a 20 Hz clock whose whole frame may spend 50 ms. That is the stall the player
/// reported, and it is not the roster's — the roster measures 0.07 ms.
///
/// Neither half has to be derived *by the frame that draws it*: both are pure functions of
/// `(tick, eye, pixel angle)` and both are **held between cadences** already. This is the split
/// `voxel_work::Pool` already makes for meshes, made here for the same reason.
///
/// **What this does not change.** The inputs and the values: a key yields one tree whoever computes
/// it. What changed is *which thread pays*, and the price: a tree is adopted a frame or two after its
/// cadence rather than on it, and the hold is now bounded by the derivation instead of by the tick.
/// That is the same held-between-cadences contract the field, the rig and the sky table already keep.
///
/// **And it is derived on the cloud's own rung, not the atmosphere's.** `Cadence::Water`'s four ticks
/// was the cadence this worker was split *off from*, and keeping it bought nothing: the tree is a
/// picture of a shape that changes over minutes, on a thread that competes with the frame for a core.
/// [`voxel_client::cloud_cubes::cadence_ticks`] is the ladder's next rung down and
/// [`cloud_tick_of`] is where the key takes it, so one ask replaces five.
///
/// **Owed, and named rather than implied — and it is one thing, not three.** The renderer is **one
/// system**, and these are its stages rather than separate features:
///
/// 1. the **octree** the world is already in (`voxel-mesh::octree`);
/// 2. its **hierarchy merge** — the backwalk that walks a node up while its boundary stays uniform,
///    equal-material and coplanar, emitting **n-gons** instead of one rectangle per slice
///    (`voxel-mesh::mesh` merges per slice today and never walks the hierarchy);
/// 3. the **subvoxel pass** on those n-gons — the 64 subvoxel base and the 256 lattice, plus the
///    declared displacement, animated as a pure function of `(tick, address)`, which is why rain and
///    moving water need it and not a fluid system of their own (C-161, C-43). **Wired in
///    2026-10-10, so this stage is no longer owed as a call site** (C-182): the frame is handed the
///    world's own declaration ([`materials::displacement_field`]) and a declared rate, a quad the
///    field does not move stays a 24-byte record, and the wiring measures inside the noise floor.
///    What is still owed is the stage's **content**: this world declares no relief, because
///    generation places whole voxels, and the *addressed* amplitude that would let rain, water and
///    the ground's own fractional surface declare at an address does not exist yet;
/// 4. **one material pass** over the result, clouds and water and terrain alike.
///
/// So this worker is **not** a second renderer and must not be grown into one. It exists because the
/// cloud's derivation cannot stay on the frame thread today, and `cloud_field` + `cloud_cubes` +
/// `vs_cloud_ngon` are a **fork of stages 2–4** that has already cost this program two measured
/// regressions (1 658 ms in the first octree cut, and the 31–60 ms per frame this removes). The fork
/// is to be **deleted** when stage 2–4 exist for real — the n-gon shader entry point and the mesher's
/// polygon pass are the same idea implemented twice, and the world must not keep two answers. This
/// buys the frame back; it pays none of that.
struct CloudWorker {
    /// What the worker thread is asked: the key, the field's tick, the eye and the pixel angle.
    asks: std::sync::mpsc::Sender<(CloudKey, u64, [f64; 3], f64)>,
    /// What it has finished, keyed by the ask it answers.
    answers: std::sync::mpsc::Receiver<(CloudKey, voxel_client::CloudCubes)>,
    /// The key asked for and not yet answered, so one cadence's question is asked **once** and a
    /// walking eye does not queue a build per frame.
    in_flight: Option<CloudKey>,
}

impl CloudWorker {
    /// Start the one thread that owns the field and the tree for the life of the program.
    ///
    /// The world spec goes over at the start: it is the whole input, it is `Copy`, and it does not
    /// change — so the worker cannot be asking a different world than the frame is drawing.
    fn new(spec: WorldSpec, report: bool) -> Self {
        let (asks, ask_rx) = std::sync::mpsc::channel::<(CloudKey, u64, [f64; 3], f64)>();
        let (answer_tx, answers) = std::sync::mpsc::channel();
        let spawned = std::thread::Builder::new().name("cloud".to_owned()).spawn(move || {
            while let Ok((key, tick, eye_m, pixel_angle_rad)) = ask_rx.recv() {
                let started = Instant::now();
                // The field is sampled **at the key's own tick and where the eye stands**, which is
                // the same pair the frame loop used: the tick is the cadence's, and the column is
                // the player's, because a field centred on the origin is a field the player walks
                // out of.
                let field = voxel_client::CloudField::sample(&spec, tick, [eye_m[0], eye_m[2]]);
                let field_ms = FrameCost::ms(started.elapsed());
                let started = Instant::now();
                let tree = voxel_client::CloudCubes::build(&field, eye_m, pixel_angle_rad);
                let tree_ms = FrameCost::ms(started.elapsed());
                if report {
                    println!(
                        "cloud\tt{tick}\tfield {field_ms:.1} ms + tree {tree_ms:.1} ms = \
                         {:.1} ms for {} cell(s), {} record(s)",
                        field_ms + tree_ms,
                        tree.cells,
                        tree.records.len()
                    );
                }
                // A closed channel means the program is going: the ask that could not be answered
                // is the last thing this thread ever does, and the frame keeps the last tree.
                if answer_tx.send((key, tree)).is_err() {
                    break;
                }
            }
        });
        match spawned {
            Ok(_) => CloudWorker { asks, answers, in_flight: None },
            // **A machine that cannot spawn one thread is refused by name, not served a still
            // picture.** The alternative — running the derivation on the frame thread again — is
            // the stutter this exists to remove, and silently reverting to it would hide the one
            // fact the player can feel.
            Err(refusal) => {
                eprintln!("shploingus: the cloud worker cannot start: {refusal}");
                std::process::exit(2);
            }
        }
    }

    /// Ask for one key's tree. **A key already in flight is not asked twice**, and a key that is
    /// already the one in hand is not asked at all — the caller compares against
    /// `App::cloud_tree_key` for that, so this only guards the in-flight case.
    fn ask(&mut self, key: CloudKey, tick: u64, eye_m: [f64; 3], pixel_angle_rad: f64) {
        if self.in_flight == Some(key) {
            return;
        }
        if self.asks.send((key, tick, eye_m, pixel_angle_rad)).is_ok() {
            self.in_flight = Some(key);
        }
    }

    /// Whatever has finished since the last call, or `None` when the worker is still working.
    ///
    /// An answer that arrived is no longer in flight, so this clears the in-flight key **when the
    /// key it answers is the one being waited for**: an older key's answer adopted while a newer one
    /// is in flight must not make the frame forget it is waiting.
    fn finished(&mut self) -> Option<(CloudKey, voxel_client::CloudCubes)> {
        match self.answers.try_recv() {
            Ok((key, tree)) => {
                if self.in_flight == Some(key) {
                    self.in_flight = None;
                }
                Some((key, tree))
            }
            Err(_) => None,
        }
    }
}

/// **The sky the same frame's rig was built from**, as a table the shader can read.
///
/// # The atmosphere is the default one and that is a debt, not a claim
///
/// `SkyInputs` is two f64s — turbidity and ozone depth — carrying no source here, which
/// is board ticket *"admit the baseline atmosphere as TICK-ZERO DATA with a source per
/// number, instead of four f64 constants in a shaper"*, still owed. It is written down
/// here as a call site so the admission has somewhere to land.
///
/// # Why this takes the [`voxel_client::skylight::Daylight`] rather than a tick
///
/// Before the sky was drawn there was nothing to keep consistent. Now there is: the
/// background behind the terrain and the light falling on it must come from one
/// evaluation of one atmosphere. `Daylight` carries that [`voxel_appearance::sky::Sky`],
/// so deriving the table from it makes disagreement between the two **impossible rather
/// than unlikely** — which is the only kind of consistency worth having when both are
/// recomputed on a cadence.
///
/// # And why the peak travels out beside the table
///
/// Because the Sun's own body is normalised by it too (C-176). The table is
/// `radiance / peak`, and the drawn disc is `irradiance / (peak · its own solid angle)`, so a
/// caller that drew the body against a differently measured peak would be exposing one pass
/// twice. Returning the number that was used, rather than letting each caller find its own, is
/// the same rule that made this function take a `Daylight` in the first place.
/// **The eye snapped to the cloud's own sample lattice**, one cell index per axis.
///
/// The tree's domain corner is snapped to [`voxel_client::CLOUD_SAMPLE_M`], so two eyes inside
/// one cell build the *identical* tree: this is what makes the held tree a function of the cell
/// rather than of the frame, and it is why a key is a legitimate cache key and not a tolerance.
/// **The tick a cloud is derived from**: the clock's own tick, floored to the cloud's cadence.
///
/// The cloud is not re-derived on the atmosphere's rung any more — [`voxel_client::cloud_cubes::cadence_ticks`]
/// is the ladder's next rung down, and this is where that rung is spent. Two things follow, and both
/// are wanted:
///
/// * the tree's own key stops moving four times a second, so a key that is already in hand is asked
///   for once per cadence rather than once per atmosphere update; and
/// * the field is sampled at *the cadence's own tick* rather than at the live one — which is the rule
///   the ask already followed, since sampling it at the live tick would make the cloud a function of
///   the frame rate rather than of the clock.
///
/// It is a floor rather than a rounding: a tick's cloud is the cloud of the cadence it is *inside* of,
/// so two frames within one cadence name one tree and the fade below spans exactly one period.
/// **One batch, one march per address**: the queue may name a cell twice, and the worker has no
/// memory of an earlier ask. Deliberately not a set that outlives the batch — a refusal is silent,
/// so a cell asked and not answered must stay askable.
fn dedupe_batch<T: std::hash::Hash + Eq + Copy>(cells: &mut Vec<T>) {
    let mut seen = std::collections::HashSet::with_capacity(cells.len());
    cells.retain(|cell| seen.insert(*cell));
}

/// **Squared distance between two points in metres**, as the light queue's own ordering.
///
/// Squared because the order is all it is used for: a square root per cell would be a square root
/// per ask, for nothing.
fn squared_distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    d[0] * d[0] + d[1] * d[1] + d[2] * d[2]
}

fn cloud_tick_of(tick: u64) -> u64 {
    let period = voxel_client::cloud_cubes::cadence_ticks();
    tick - tick % period
}

fn snap_to_sample_lattice(eye_m: [f64; 3]) -> [i64; 3] {
    let cell = voxel_client::CLOUD_SAMPLE_M;
    [
        (eye_m[0] / cell).floor() as i64,
        (eye_m[1] / cell).floor() as i64,
        (eye_m[2] / cell).floor() as i64,
    ]
}

impl App {
    /// **The shell, plus the judgement the game asked for on its own clock.**
    ///
    /// `notifications` is the queue's stream, and `workspace` is where that queue lives, so
    /// each record's artifact can be re-hashed before the record is shown.
    fn new(
        shell: ShellMount,
        notifications: Option<NotificationStream>,
        workspace: Option<std::path::PathBuf>,
    ) -> Self {
        let world = World::shipped();
        // **The cloud worker, and the instrument that reports it.** The worker is built here so its
        // thread starts with the program: the first cadence's derivation then begins before the first
        // frame asks for it rather than after.
        let timing = FrameTiming::from_env();
        let cloud_worker = CloudWorker::new(*world.spec(), timing.on);
        // **The atmosphere's look-ahead starts with the program**, so the first cadence after boot is
        // already being derived while the window opens rather than after it has drawn.
        let atmosphere_ahead = match voxel_client::AtmosphereAhead::new(
            *world.spec(),
            voxel_client::game::GAME_NEW_MOON_TICK,
        ) {
            Ok(ahead) => ahead,
            // **A machine that cannot spawn one thread is refused by name, not served the stall.**
            Err(refusal) => {
                eprintln!("shploingus: the atmosphere worker cannot start: {refusal}");
                std::process::exit(2);
            }
        };
        let mut camera = Camera::standard(spawn_eye(&world), 16.0 / 9.0);
        camera.pitch_rad = GAME_PITCH_RAD;
        App {
            graphics: None,
            streamer: None,
            shell,
            camera,
            laya: LayaGhost::at(camera.position_m),
            overhaul: CameraOverhaul::new(),
            render_camera: camera,
            player: Player::new(spawn_feet(&world)),
            world,
            displacement: materials::displacement_field()
                .expect("the world's displacement field declares"),
            refine: RefineConfig::new(
                REFINE_SAMPLES_PER_BLOCK,
                REFINE_MAX_TRIANGLES_PER_QUAD,
            )
            .expect("the declared refinement rate is inside the addressed level"),
            earthworks: Earthworks::new(),
            held: BTreeSet::new(),
            selected: 0,
            cursor: None,
            last_shell_click: None,
            last: Instant::now(),
            frames: 0,
            title_at: Instant::now(),
            fps: None,
            workers: 0,
            shell_refusal: None,
            lanes: false,
            wireframe: false,
            // **`SHPLOINGUS_LAYER` opens a run on a layer view**, so a capture can be taken of one
            // without a person at the keyboard — the same reason the walk and the recorder take
            // their inputs from the environment. An unparsable value is the shaded picture.
            layer: std::env::var("SHPLOINGUS_LAYER")
                .ok()
                .and_then(|text| text.trim().parse::<u32>().ok())
                .filter(|layer| *layer <= 4)
                .unwrap_or(0),
            changes: Vec::new(),
            change_generation: 0,
            changes_seen: 0,
            corner_refusals_seen: 0,
        clock: Clock::at(TICK),
        tick_driver: voxel_work::TickDriver::full()
            .expect("the default pipeline and its horizons validate"),
        process: voxel_shapers::process::ProcessScheduler::world_defaults()
            .expect("the built-in process roster validates"),
        process_at: None,
        process_refusals: None,
        notifications,
        workspace,
        notified_at: None,
        notifications_shown: 0,
        console_at: 0,
        console_dropped: 0,
        timing,
        drawing: None,            rig_at_tick: None,
            light_grade: 1.0,
        light_field: None,
        measurer: Some(voxel_client::Measurer::new()),
        light_ask: None,
        light_epoch: 0,
        light_built_under: None,
        light_totals: [0; 5],
        resident: {
            // **The shadow's own resolution, asked for by the frame that draws it.** One sample
            // per stored subvoxel is the finest the world's light field can answer; the builder
            // clamps anything else to a divisor of `SUBVOXEL_SIDE`. A scene built before this
            // (or by a test that asks for nothing) keeps the once-per-corner path.
            let mut resident = voxel_client::ResidentScene::new();
            resident.set_shadow_lattice(voxel_client::SHADOW_LATTICE);
            resident
        },
        residency: voxel_client::Residency::default(),
        walk: std::env::var_os("SHPLOINGUS_WALK").is_some(),
        walk_started: false,
        walk_hopped: false,
        walk_frames: 0,
        walk_missed: 0,
        walk_landed: 0,
        walk_absent: 0,
        walk_totals: [0; 4],
        walk_since: None,
        walk_for: walk_seconds(),
        work: voxel_client::FrameWork::stated(),
        geometry: GeometryTally::default(),
        bodies: Vec::new(),
            cloud_tree: None,
            cloud_tree_previous: None,
            cloud_tree_at_tick: None,
            cloud_tree_key: None,
            cloud_worker,
            atmosphere_ahead,
            turbidity_at: None,
            turbidity: voxel_appearance::sky::SkyInputs::default().turbidity,
            ozone_at: None,
            ozone_atm_cm: voxel_appearance::sky::SkyInputs::default().ozone_atm_cm,
        }
    }
}

impl App {
    /// **The sky's two inputs, each sampled on the cadence of the stage that owns it.**
    ///
    /// **This is the join the record orders and the one that was missing.** The sun and
    /// the moon are both lit through the same air, and the turbidity that air carries is
    /// a *weather* quantity — so before this, the sky was evaluated in `SkyInputs::
    /// default()`, which is Preetham's clear air, forever, while the world around it
    /// rained. The two fields are held separately because their stages are not in step:
    ///
    /// | field | stage | cadence |
    /// |---|---|---|
    /// | `turbidity` | [`voxel_shapers::stages::Stage::Weather`] | `Cadence::Water`, ×4 |
    /// | `ozone_atm_cm` | [`voxel_shapers::stages::Stage::Climate`] | `Cadence::Daily` |
    ///
    /// **Both are sampled at the player's own `(x, z)`**, because a sky that is one value
    /// everywhere is a sky with no weather in it — the eye is `self.camera.position_m`.
    ///
    /// The first call samples both, and after that each is held until its own stage is
    /// due again. **A refused weather sample keeps the previous value** rather than
    /// falling back to clear air, because a refusal is not a clear day.
    fn atmosphere(&mut self) -> SkyInputs {
        let tick = self.clock.tick;
        let (x, _, z) = (
            self.camera.position_m[0],
            self.camera.position_m[1],
            self.camera.position_m[2],
        );
        // **The first frame samples both, whatever the ladder says is due**, because a
        // world that opens on clear air because `TICKS_PER_DAY` is not a multiple of
        // anything would be a world whose sky is wrong until tomorrow.
        let first = self.turbidity_at.is_none() || self.ozone_at.is_none();
        // **The stage's own due-check**, not the ladder restated here: `weather::is_due_at`
        // exists as the one seam so the ×4 rung is reachable from the stage it governs
        // (C-94.7). The sample below reads the same stage's rate.
        let weather_due =
            first || (Some(tick) != self.turbidity_at && voxel_shapers::weather::is_due_at(tick));
        // **The driver's due set, not the ladder restated here.** `TickDriver::due_at`
        // reads `Stage::cadence()`, so the ×1 rung's clone (`Stage::Climate` is daily)
        // cannot come to disagree with the schedule the world is planned on.
        let due_now = self.tick_driver.due_at(tick);
        let climate_due = first || (Some(tick) != self.ozone_at && due_now.contains(&Stage::Climate));
        if weather_due || climate_due {
            // **One sample, two fields** — the two stages are read from the same world at
            // the same place, so asking twice would be the same answer twice.
            let sampled = voxel_client::skylight::sky_inputs_at(self.spec(), x, z, tick);
            if weather_due {
                self.turbidity = sampled.turbidity;
                self.turbidity_at = Some(tick);
            }
            if climate_due {
                self.ozone_atm_cm = sampled.ozone_atm_cm;
                self.ozone_at = Some(tick);
            }
        }
        SkyInputs { turbidity: self.turbidity, ozone_atm_cm: self.ozone_atm_cm }
    }

    /// **Drive the process roster at the eye's own unit, once per tick.**
    ///
    /// The unit is the 16³ octree-level-4 box centred on the eye and the region is that
    /// box's four cardinal columns, each at its true distance — the two region forms the
    /// structural and neighbour-reading bodies were owed. It changes **no voxel**: every
    /// body is a pure read of a shaper, which is why `the_runtime_changes_nothing_about_the_world`
    /// can pin that the world spec is bit-identical after a step. What it does give the
    /// running program is the roster's own answer — which stages ran and which refused —
    /// reported **when that answer changes**, so a stage that owes a body is visible here
    /// rather than only in a test.
    ///
    /// **A fresh roster opens at the pipeline's own tick 0.** The clock starts mid-day
    /// (`GAME_TICK`), and `ProcessScheduler` refuses a stage whose dependency has *never run*
    /// at a nonzero tick — so a fresh roster's first step at `GAME_TICK` would report
    /// `DependencyUnavailable` for the daily stages until the next day boundary. Stepping
    /// tick 0 once is exactly what that clause anticipates: the pipeline's first tick is the
    /// whole chain, and the live tick follows it.
    fn drive_processes(&mut self, tick: u64) {
        if self.process_at == Some(tick) {
            return;
        }
        use voxel_shapers::process::{ProcessRequest, RegionSample, Site, UnitBox};
        let site = Site::new(
            self.camera.position_m[0].floor() as i32,
            self.camera.position_m[1].floor() as i32,
            self.camera.position_m[2].floor() as i32,
        );
        const HALF: i32 = 8;
        let unit = UnitBox::new(
            [site.x - HALF, site.y - HALF, site.z - HALF],
            [site.x + HALF, site.y + HALF, site.z + HALF],
            4,
        );
        let region = [
            RegionSample { site: Site::new(site.x + 4, site.y, site.z), distance_m: 4.0 },
            RegionSample { site: Site::new(site.x - 4, site.y, site.z), distance_m: 4.0 },
            RegionSample { site: Site::new(site.x, site.y, site.z + 4), distance_m: 4.0 },
            RegionSample { site: Site::new(site.x, site.y, site.z - 4), distance_m: 4.0 },
        ];
        let spec = *self.spec();
        let mut ticks: Vec<u64> = Vec::new();
        if self.process.last_tick().is_none() && tick != 0 {
            ticks.push(0);
        }
        ticks.push(tick);
        for at in ticks {
            let request = ProcessRequest {
                site,
                tick: at,
                hydro_radius: voxel_shapers::process::FIELD_PLANE_HYDRO_RADIUS,
                region: &region,
                unit: Some(unit),
            };
            match self.process.step(&spec, &request) {
                Ok(evidence) => {
                    self.process_at = Some(at);
                    let summary = summarise_processes(&evidence);
                    if self.process_refusals.as_deref() != Some(summary.as_str()) {
                        println!("process\tt{at}\t{summary}");
                        self.process_refusals = Some(summary);
                    }
                }
                Err(refusal) => {
                    // A refused step is named once and **not retried every frame**: the tick
                    // is recorded so a persistent refusal is one line, not sixty a second.
                    self.process_at = Some(at);
                    eprintln!(
                        "shploingus: the process roster refused tick {at}: {}",
                        refusal.detail()
                    );
                }
            }
        }
    }
}

/// The roster's answer as one line: how many stages ran, and every refusal **by name**.
fn summarise_processes(evidence: &[voxel_shapers::process::ProcessEvidence]) -> String {
    let ran = evidence.iter().filter(|e| e.ran()).count();
    let refused: Vec<String> = evidence
        .iter()
        .filter(|e| !e.ran())
        .map(|e| {
            format!(
                "{}:{}",
                e.process.stage().name(),
                e.refusal.map(|condition| condition.name()).unwrap_or("unknown")
            )
        })
        .collect();
    format!("{ran} of {} ran; refused [{}]", evidence.len(), refused.join(", "))
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // `resumed` can arrive more than once on some platforms; the window is
        // made once and kept.
        if self.graphics.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title("shploingus")
            .with_inner_size(LogicalSize::new(1280.0, 720.0));
        // **The rig the pipeline is constructed with, before any frame has run.** A
        // default atmosphere cannot be refused — `SkyInputs::default()` is a legal
        // turbidity — so this is not a fallback dressed up as a value; it is the world's
        // tick 0 light, and the first `redraw` replaces it with tick 1's.
        // **The boot rig is the world's own weather, not clear air** — sampled through the
        // same cadence-respecting path the frame loop uses, so the first frame is not
        // brighter than the two hundredth for a reason nobody could see.
        let boot_inputs = self.atmosphere();
        // **One evaluation, two consumers.** The rig and the sky both come from this
        // `Daylight`, so the first frame cannot be lit by one sky and backed by
        // another. `rig_at` remains for callers that want only the rig.
        let boot_daylight = match voxel_client::skylight::daylight_of(
            self.spec(),
            self.clock.tick,
            boot_inputs,
            voxel_client::game::GAME_NEW_MOON_TICK,
        ) {
            Some(d) => d,
            None => {
                eprintln!(
                    "shploingus: the default atmosphere was refused, so there is no rig to \
                     construct the pipeline with"
                );
                event_loop.exit();
                return;
            }
        };
        let boot_rig = match GpuRig::from_rig(&voxel_client::skylight::exposed(&boot_daylight.rig))
        {
            Some(gpu) => gpu,
            None => {
                eprintln!("shploingus: the rig does not fit the uniform");
                event_loop.exit();
                return;
            }
        };
        let window = match event_loop.create_window(attributes) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                eprintln!("shploingus: cannot create a window: {e}");
                event_loop.exit();
                return;
            }
        };
        let graphics = match Graphics::new(window, &boot_rig) {
            Ok(g) => g,
            Err(e) => {
                eprintln!("shploingus: {e}");
                event_loop.exit();
                return;
            }
        };
        // The sky, from the same `Daylight` the boot rig came from. Before the sky pass
        // existed this line had nothing to write to and every uncovered pixel was the
        // render pass's clear value.
        //
        // **Both the background and the Sun's body come from this one evaluation**, exactly
        // as the frame loop's own tick does, so the first frame cannot be lit by one sky and
        // backed by another — nor draw a body normalised against a peak the table was not.
        if let Some((model, peak)) = voxel_client::atmosphere::sky_model_at(
            self.spec(),
            &boot_daylight,
            boot_inputs,
            self.clock.tick,
            self.render_camera.position_m,
            voxel_client::game::GAME_NEW_MOON_TICK,
        ) {
            graphics.world.write_sky_model(&model);
            // **The Sun and the Moon as records** (C-184), from this one evaluation: the band table's
            // own declared reflectance lit by this sky's sun, so the first frame's moon is not lit by a
            // sun the frame does not have. What draws them is the scene the frame already builds — a
            // body is geometry now, not a field of a uniform. `None` is the model's own refusal (a Sun
            // that has not risen, a new moon with no lit face), and a frame with neither has no body.
            let eye = self.render_camera.position_m;
            let spec = *self.spec();
            let mut bodies = Vec::new();
            if let Some(sun) = voxel_client::sun::DrawnSun::of(&boot_daylight.sky, peak, eye) {
                bodies.push(sun.body_record_graded(eye, boot_daylight.grade));
            }
            if let Some(moon) = voxel_client::sun::DrawnMoon::of(
                &voxel_shapers::sky::moon_position(
                    &spec,
                    self.clock.tick,
                    voxel_client::game::GAME_NEW_MOON_TICK,
                ),
                &voxel_client::sun::air_at(&spec, eye[0], eye[2], self.clock.tick),
                peak,
                eye,
                boot_daylight.sky.sun_altitude_deg,
                voxel_shapers::sky::solar_position(&spec, self.clock.tick).azimuth_deg,
            ) {
                bodies.push(moon.body_record_graded(eye, boot_daylight.grade));
            }
            self.bodies = bodies;
        } else {
            // **Stated, not omitted**: `sky_model_at` refuses when there is no peak to normalise the
            // transport against, so this frame has no sky rather than no word for one — which is
            // the difference the pipeline refuses a frame over.
            graphics.world.write_no_sky_model();
        }

        // The streamer, with one fewer worker than the machine has, because the
        // frame loop needs a core too.
        //
        // `SHPLOINGUS_WORKERS` overrides it. That is not a convenience: the HUD
        // tells the player that the worker count is a perf knob and that the
        // digest is the world, and a claim on screen that nobody watching can
        // falsify is a claim that has stopped being checkable. Setting this to
        // 1 and to 15 and reading the same digest out of the panel is the
        // falsification, done by hand, in the running program.
        let workers = match std::env::var("SHPLOINGUS_WORKERS") {
            Ok(text) => match text.trim().parse::<usize>() {
                Ok(n) if n >= 1 => {
                    println!("shploingus: SHPLOINGUS_WORKERS={n}");
                    n
                }
                other => {
                    eprintln!("shploingus: SHPLOINGUS_WORKERS must be a positive integer, not {other:?}");
                    event_loop.exit();
                    return;
                }
            },
            Err(_) => std::thread::available_parallelism()
                .map(|n| n.get().saturating_sub(1).max(1))
                .unwrap_or(1),
        };
        let spec = *self.spec();
        let source = WorldSource::new(spec);
        let registry = match materials::registry() {
            Ok(r) => r,
            Err(e) => {
                eprintln!("shploingus: {e}");
                event_loop.exit();
                return;
            }
        };
        let mesh_config = match voxel_mesh::MeshConfig::from_registry(&registry) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("shploingus: the mesh config: {e}");
                event_loop.exit();
                return;
            }
        };
        let streamer = match Streamer::new(source, mesh_config, workers, CEILING_BYTES)
            .and_then(|s| s.with_window(WINDOW))
        {
            Ok(s) => s,
            Err(e) => {
                eprintln!("shploingus: the streamer: {e}");
                event_loop.exit();
                return;
            }
        };
        println!(
            "shploingus on {} — {workers} worker threads, window {}×{} chunks, {} MB ceiling",
            graphics.adapter,
            WINDOW.count(),
            (2 * WINDOW.radius_chunks + 1),
            CEILING_BYTES / (1024 * 1024)
        );
        self.streamer = Some(streamer);
        self.graphics = Some(graphics);
        self.workers = workers;
        self.last = Instant::now();
        self.title_at = Instant::now();
        // **Apply the pointer's owner as soon as a window exists to apply it to.** The
        // focus event that would otherwise do it can arrive before `graphics` does — and
        // is then a no-op — so a world view whose pointer was never grabbed would read as
        // "the mouse is not locked" with nothing on the path to fix it.
        self.sync_pointer_grab();
    }

    /// **The camera's motion source, and it is the device rather than the cursor.**
    ///
    /// [`DeviceEvent::MouseMotion`] is the platform's raw, relative motion. It is what
    /// makes a first-person look work at all here: while the pointer is locked the
    /// cursor is pinned (this platform's `Locked` clips it to a 1×1 rectangle at its
    /// present position), so [`WindowEvent::CursorMoved`] stops carrying anything a
    /// camera could read. Raw motion keeps arriving while the cursor is pinned.
    ///
    /// It also arrives while the shell is up, where it must do nothing — the pointer
    /// belongs to the windows then — so this reads the **same ownership rule**
    /// [`App::sync_pointer_grab`] applies to the grab. One decision, two consumers.
    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        if self.shell.shell_shown() {
            return;
        }
        let DeviceEvent::MouseMotion { delta: (dx, dy) } = event else {
            return;
        };
        self.camera.yaw_rad += dx * LOOK_RAD_PER_MOTION_UNIT;
        self.camera.pitch_rad = (self.camera.pitch_rad - dy * LOOK_RAD_PER_MOTION_UNIT).clamp(
            -voxel_client::camera::MAX_PITCH_RAD,
            voxel_client::camera::MAX_PITCH_RAD,
        );
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(g) = self.graphics.as_mut() {
                    g.resize(size.width, size.height);
                }
            }
            // A window dragged between a 100 % and a 150 % display gets a new
            // scale factor, and the HUD's dp layout has to follow it. Nothing
            // else in this program is DPI-aware, because nothing else is
            // measured in device-independent pixels.
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if let Some(g) = self.graphics.as_mut() {
                    g.scale = scale_factor as f32;
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let PhysicalKey::Code(code) = event.physical_key else {
                    return;
                };
                // **Escape pulls the bar down rather than closing the game** — `event-0004`:
                // *"Escape pulls a horizontal tab strip down across the top edge of the
                // screen"* — and F1 is the dirty shell's own toggle, kept because a player who
                // learned it there should not have to learn a second key. Closing the window is
                // the way out.
                if event.state == ElementState::Pressed
                    && matches!(code, KeyCode::Escape | KeyCode::F1)
                {
                    self.shell.toggle_shell();
                    // The pointer follows the shell, and it has to: the pointer is the
                    // shell's while the windows are up (a click that lands on a window
                    // is that window's, not a dig), so locking it then would steal
                    // every click and make the UI unusable.
                    let shown = self.shell.shell_shown();
                    self.sync_pointer_grab();
                    // Which way the pointer went is printed with it, because "the camera
                    // does not turn" and "the pointer is not the world's" look the same
                    // from inside the window.
                    println!(
                        "shploingus: shell {} — the pointer is {}",
                        if shown { "shown" } else { "hidden" },
                        if shown {
                            "the shell's (free and visible)"
                        } else {
                            "the world's (locked, cursor hidden)"
                        }
                    );
                    return;
                }
                // **The console owns the keyboard while it is focused**, and it is asked second —
                // after the modal Escape/F1 and before F3 and the digit bindings, because a
                // focused line edit takes every key it can use. A console that could not receive
                // a digit would be a command line that cannot type half its arguments.
                if event.state == ElementState::Pressed && self.shell.console_has_focus() {
                    // `event.text` is the platform's **resolved** text, not the physical code, so
                    // a shifted character or a layout other than this one arrives as itself.
                    let key = match code {
                        KeyCode::Enter | KeyCode::NumpadEnter => Some(ShellKey::Enter),
                        KeyCode::Backspace => Some(ShellKey::Backspace),
                        KeyCode::ArrowUp => Some(ShellKey::HistoryUp),
                        KeyCode::ArrowDown => Some(ShellKey::HistoryDown),
                        _ => event.text.and_then(|text| text.chars().next()).map(ShellKey::Char),
                    };
                    if let Some(key) = key {
                        self.shell.key_pressed(key, Some(TICK));
                    }
                    return;
                }
                // **F3 draws the debug lanes: a declared, labelled outline over every piece of the
                // shell's own structure and every affordance on it** (event-0013, H-7.25 — *the
                // lanes on every window are the test surface*). It is on its own key rather than a
                // tab, and it works **with the shell closed**, which is the dependence the vector
                // view replaces (H-7.9).
                if event.state == ElementState::Pressed && code == KeyCode::F3 {
                    self.lanes = !self.lanes;
                    println!(
                        "shploingus: debug lanes {}",
                        if self.lanes { "on" } else { "off" }
                    );
                    return;
                }
                // **F4 outlines the world meshes themselves** — the greedy merge's own quads,
                // which is how a person sees how the meshes are made (the request that arrived
                // with the wireframe ask, 2026-10-04). It is not the edit lanes: the lanes are
                // the change view, this is the geometry. While it is on the world's own
                // materials are withheld and the lines are graded by depth in a rainbow, on
                // the user's own call — white read poorly against the world.
                if event.state == ElementState::Pressed && code == KeyCode::F4 {
                    self.wireframe = !self.wireframe;
                    println!(
                        "shploingus: mesh wireframe {}",
                        if self.wireframe { "on" } else { "off" }
                    );
                    return;
                }
                // **F5 steps through the material's layers** — the shaded picture, then the
                // stretch axis, the corner's own sample, the ladder and the footprint. Each is the
                // same fragments the frame draws, so a face whose pattern is flatter or coarser than
                // its neighbours is visible in the layer that owns the difference rather than
                // inferred from colour.
                if event.state == ElementState::Pressed && code == KeyCode::F5 {
                    self.layer = (self.layer + 1) % 5;
                    println!(
                        "shploingus: material layer {}{}",
                        self.layer,
                        match self.layer {
                            0 => " (shaded)",
                            1 => " (stretch axis)",
                            2 => " (corner sample)",
                            3 => " (ladder)",
                            _ => " (footprint)",
                        }
                    );
                    return;
                }
                if event.state == ElementState::Pressed {
                    if let Some(choice) = selection_of(code) {
                        self.selected = choice;
                        let (_, name) = PLACEABLE[choice];
                        println!("shploingus: building with {name}");
                    }
                    self.held.insert(code);
                } else {
                    self.held.remove(&code);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let at = self.shell_point();
                if state == ElementState::Pressed {
                    // **The shell gets the click first when it is drawn there.** A click that
                    // lands on the bar or a window is the shell's; only a click that lands on
                    // the world is the world's, because digging through a window is a mis-click
                    // rather than a feature.
                    if self.shell.shell_shown() && self.shell.hit(at) {
                        let button = match button {
                            MouseButton::Right => ShellButton::Right,
                            _ => ShellButton::Left,
                        };
                        // **The second press of a pair is answered as the double-click it is.**
                        // The platform reports presses and no counts, so the count is made here
                        // against the two declared platform numbers above, on the raw cursor
                        // position they are measured in; the port's `press_at` takes the
                        // verdict as its `double` flag.
                        let raw = self.cursor.unwrap_or((-1.0, -1.0));
                        if self.count_shell_click(button, raw) {
                            self.shell.pointer_double_clicked(button, at);
                        } else {
                            self.shell.pointer_pressed(button, at);
                        }
                        return;
                    }
                    // A press the shell did not claim cannot be half of a pair with one it did.
                    self.last_shell_click = None;
                    match button {
                        MouseButton::Left => self.break_at_target(),
                        MouseButton::Right => self.build_at_target(),
                        _ => {}
                    }
                } else {
                    self.shell.pointer_released(at);
                }
            }
            // **The wheel belongs to the windows, and only to the windows.** A gesture that
            // landed on the world would be a camera control this shell has not been given; the
            // first surface that asked for one is the console's scrollback. Claimed only when
            // the shell is up and the point is on its chrome, exactly like a click.
            WindowEvent::MouseWheel { delta, .. } => {
                let at = self.shell_point();
                if self.shell.shell_shown() && self.shell.hit(at) {
                    let scale = self.graphics.as_ref().map(|g| g.scale as f64).unwrap_or(1.0);
                    let delta = match delta {
                        // A notched wheel reports lines; positive is away from the operator.
                        MouseScrollDelta::LineDelta(_, lines) => ShellWheel::Lines(lines as f64),
                        // A trackpad reports physical pixels, and every layout in this program is
                        // in device-independent pixels — converted here, the one place that
                        // knows the window's scale factor.
                        MouseScrollDelta::PixelDelta(position) => {
                            ShellWheel::Pixels(position.y / scale.max(1e-6))
                        }
                    };
                    self.shell.wheel(at, delta);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                // The pointer's **position** is the shell's business — hover, drag and
                // hit-testing — and it cannot be the camera's: a locked pointer stops
                // reporting position changes (`device_event` says why), so the camera reads
                // the device's raw motion instead. One source each, so a platform that
                // reports both can never double-turn the view.
                let now = (position.x, position.y);
                let scale = self.graphics.as_ref().map(|g| g.scale as f64).unwrap_or(1.0);
                self.shell.pointer_moved((now.0 / scale.max(1e-6), now.1 / scale.max(1e-6)));
                self.cursor = Some(now);
            }
            WindowEvent::CursorLeft { .. } => {
                self.cursor = None;
                self.shell.pointer_left();
            }
            WindowEvent::RedrawRequested => self.redraw(event_loop),
            // **Regain the grab on focus.** The platform releases it when the window
            // loses focus — correctly, so the player can reach another window — but it
            // does not always hand it back, and a first-person camera whose pointer
            // is free again cannot be turned past the screen edge. Re-asserting the
            // decision we already made is cheaper than detecting the loss.
            WindowEvent::Focused(true) => self.sync_pointer_grab(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // The world is arriving whether or not anybody moves, so the loop keeps
        // asking for frames. A frame only on input would freeze the streaming.
        if let Some(g) = self.graphics.as_ref() {
            g.window.request_redraw();
        }
    }
}

impl App {
    /// The pointer in **device-independent pixels** — the unit every layout in this program
    /// uses — or a point no target can contain when the pointer has left the window.
    fn shell_point(&self) -> (f64, f64) {
        let scale = self.graphics.as_ref().map(|g| g.scale as f64).unwrap_or(1.0);
        match self.cursor {
            Some((x, y)) => (x / scale.max(1e-6), y / scale.max(1e-6)),
            None => (-1.0, -1.0),
        }
    }

    /// **The platform's own double-click test**, made here because winit reports presses and
    /// no counts: two presses of the same button, within [`DOUBLE_CLICK_INTERVAL_MS`] and
    /// inside [`DOUBLE_CLICK_SLOP_PX`] of each other, are one double-click — and the check
    /// runs against the **raw cursor** (pixels), not the dp point the shell is handed.
    ///
    /// A double **ends the sequence**: the click after a pair starts a new one, as the
    /// platform's own sequence does.
    fn count_shell_click(&mut self, button: ShellButton, at: (f64, f64)) -> bool {
        let now = Instant::now();
        let double = is_double_click(button, at, now, self.last_shell_click);
        self.last_shell_click = if double { None } else { Some((now, at, button)) };
        double
    }

    /// **Show the next decision the game's own pass wrote** — at most one per tick (H-12.2).
    ///
    /// The record is decoded whole and its artifact re-hashed before it is shown; a record whose
    /// artifact no longer hashes to what was judged is **refused by name** and the stream stops,
    /// because a decision shown against bytes it was not made from is worse than no decision.
    /// A drained queue says so once and stops asking, rather than printing the same line every
    /// tick for the life of the run.
    fn show_next_notification(&mut self, tick: u64) {
        let Some(root) = self.workspace.clone() else {
            return;
        };
        let Some(stream) = self.notifications.as_mut() else {
            return;
        };
        let next = stream.next_verified(&root);
        match next {
            Ok(Some(notification)) => {
                self.notifications_shown += 1;
                let decision = &notification.decision;
                let state = if decision.cleared() { "cleared" } else { "refused" };
                let finding = decision.findings.first().cloned().unwrap_or_else(|| {
                    "no finding: the artifact carries its proof of work".to_string()
                });
                // **A refused decision is a refusal, and refusals are persistent** (UI-SPEC §7):
                // the level is what puts it in the notifications window and keeps it out of the
                // toast queue. A cleared decision is an event — logged, not loud (C-64.2's
                // predicate is the split, and this is one more place it applies).
                let level = if decision.cleared() {
                    ConsoleLevel::Event
                } else {
                    ConsoleLevel::Warn
                };
                self.shell.log(
                    level,
                    "laya",
                    format!(
                        "{} {state} {} [{}] at tick {tick}: {finding}",
                        notification.notification, decision.path, decision.verdict
                    ),
                    Some(tick),
                );
            }
            Ok(None) => {
                self.notifications = None;
                self.shell.log(
                    ConsoleLevel::Event,
                    "laya",
                    format!(
                        "the queue is drained: {} decision(s) shown by tick {tick}",
                        self.notifications_shown
                    ),
                    Some(tick),
                );
            }
            Err(refused) => {
                self.notifications = None;
                self.shell.log(ConsoleLevel::Warn, "laya", refused, Some(tick));
            }
        }
    }

    /// **The light's own frame**: adopt what the worker has measured, answer the trees it needs,
    /// and hand it the next batch when it has room for one.
    ///
    /// Bounded in every direction, because this runs on the frame: one adoption batch, a fixed
    /// number of tree handles, and one ask. Nothing here waits — a worker still marching is a
    /// frame that asks again next frame, and a worker with nothing to do costs one hash lookup
    /// and one empty receive.
    fn pump_light(&mut self) {
        /// Tree handles answered in one frame: a refcount each, and the worker blocks on them.
        const TREES: usize = 16;

        let Some(measurer) = self.measurer.as_mut() else {
            return;
        };
        if let Some(field) = self.light_field.as_ref() {
            field.borrow_mut().begin_frame();
        }
        // (a) What landed. A measurement is a cell's bands **and the beam they were marched
        // under**, so an answer that arrives after the sun has moved is still an answer about its
        // own beam rather than a claim about this frame's.
        let landed = measurer.adopt(self.work.measure());
        if let Some(field) = self.light_field.as_ref() {
            let mut field = field.borrow_mut();
            for measured in &landed {
                let cell = measured.cell;
                field.adopt(cell, measured.bands, measured.toward);
            }
        }
        // (b) The worker's own questions. The streamer is the world the beams cross, and what
        // crosses to the worker is a refcount on the same tree rather than a copy of it.
        let streamer = self.streamer.as_ref();
        measurer.service(TREES, |pos| streamer.and_then(|streamer| streamer.tree_shared(pos)));
        // (c) The next batch, when the worker has room for one.
        if measurer.busy() {
            return;
        }
        let (Some(template), Some(field)) = (self.light_ask, self.light_field.as_ref()) else {
            return;
        };
        let mut cells = field.borrow_mut().wants(voxel_client::CELLS_PER_ASK);
        // **Nearest first**, so the part of the world the eye is in is measured before the part it
        // is not: the queue is the build's own order, which is mesh order and not the eye's.
        let eye = self.camera.position_m;
        cells.sort_unstable_by(|a, b| {
            squared_distance(a.centre_m(), eye).total_cmp(&squared_distance(b.centre_m(), eye))
        });
        // **One ask, one march per cell — and no memory past this batch.** The worker sends no
        // refusal: a cell whose own chunk is absent when the beam leaves ends the ask with nothing
        // in `Batch.measured`. A set that outlived the ask therefore kept that address out of every
        // later batch, and its corners stayed on the rig for the rest of the run — lit by the
        // unshadowed sun however far the light had since moved.
        dedupe_batch(&mut cells);
        if cells.is_empty() {
            return;
        }
        // **The residency the worker is handed with the ask**: which chunks have a tree in hand.
        // Without it a march discovers the empty sky one frame at a time.
        let (generation, resident) = match self.streamer.as_ref() {
            Some(streamer) => (
                streamer.world_generation(),
                streamer.tree_chunks().collect::<Vec<_>>(),
            ),
            None => (template.generation, Vec::new()),
        };
        measurer.ask_for(voxel_client::Ask { generation, ..template }, cells, resident);
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        if self.graphics.is_none() || self.streamer.is_none() {
            return;
        }

        // ---- time -------------------------------------------------------
        let now = Instant::now();
        let dt = (now - self.last).as_secs_f64().min(0.1);
        self.last = now;
        // **The frame-timing instrument's first mark.** `mark` is moved at each section
        // boundary below, so the report's split is the frame loop's own order rather than a
        // second description of it that can go stale.
        let mut mark = Instant::now();
        let mut cost = FrameCost::default();

        // ---- the clock, and the light it drives --------------------------
        // **Advance before the rig is derived, so the rig is this frame's tick and not
        // last frame's.** `Clock::advance` is bounded by `MAX_CATCH_UP` and counts what
        // it dropped, so a stall costs the world four ticks and says so in `dropped`
        // rather than handing it two hundred.
        let moved = self.clock.advance(dt);
        if self.clock.dropped > 0 {
            // **Once, and then only when it changes.** Sixty times a second is not a
            // report, it is a noise floor.
            let dropped = self.clock.dropped;
            eprintln!("shploingus: the world is {dropped} tick(s) behind — a frame ran long");
        }
        // **The light is derived on the atmosphere stage's declared cadence, not per
        // frame.** `Stage::Atmosphere` is `Cadence::Water` — every 4 ticks (H-8.3) — so
        // the sky is re-solved when that stage is due and *held* in between. Two reasons,
        // and the second is the one that bites: the cost is a 96-day solar search plus a
        // `Sky::at`, and doing that per frame made the world's light a function of the
        // frame rate rather than of the tick, which is exactly what a determinism
        // contract forbids.
        //
        // **The `tick != rig_at_tick` half is load-bearing.** `is_due_at` is
        // `tick.is_multiple_of(4)`, so it is true for tick 12,000 on every frame until
        // the tick moves — without the inequality the stage would be "due" sixty times
        // a second and the cadence would be decorative.
        let tick = self.clock.tick;
        // **The shell's clock, once per frame and idempotent within a tick.** The toast queue is
        // the first transient surface to read it (H-12.2: one at a time, at the world's rate),
        // and a tick that changed nothing changes nothing here either.
        self.shell.set_tick(tick);
        // **The process roster, at the world's own tick.** It runs once per tick (guarded
        // in the method) and reports only when its answer changes; it mutates nothing.
        self.drive_processes(tick);
        cost.roster = mark.elapsed();
        mark = Instant::now();
        let due = Some(tick) != self.rig_at_tick
            && self
                .tick_driver
                .due_at(tick)
                .contains(&voxel_shapers::stages::Stage::Atmosphere);
        if due {
            let cadence_started = Instant::now();
            self.rig_at_tick = Some(tick);
            // **`None` keeps the previous rig**, which is `WorldPipeline::write_rig`'s own
            // refusal semantics too, so a refused atmosphere costs a held frame rather
            // than a black one.
            let inputs = self.atmosphere();
            // The three things the derivation needs, read **before** the graphics borrow: the
            // spec is behind `&self`, and a `&mut self.graphics` held across a `&self` call is
            // the borrow checker's problem rather than the renderer's.
            let spec = *self.spec();
            let eye = self.render_camera.position_m;
            // **The world's own light, on the cadence the rig is** — adopted from the look-ahead
            // when it is this cadence's, derived here when it is not — and **kept between
            // cadences**: the cells the previous field measured are carried over when the beam has
            // not swung past the solver's own step, and thrown away when it has (see
            // `voxel_client::field`'s module doc for why the step is the stencil's own resolution
            // rather than a tuning constant).
            //
            // `None` when the atmosphere is refused, which is **the rig's own refusal**: one
            // `daylight_of`, one solver, the same conditions for both. A field that measured an
            // atmosphere the rig could not describe would be a second opinion about the sky, and
            // this game has exactly one.
            //
            // **Derived four ticks ago when the look-ahead could, here when it could not.** `cadence`
            // takes the worker's answer only when it is the same derivation in every argument — the
            // tick, the inputs and the eye — and derives the cadence on this thread otherwise, which
            // is exactly what this loop did always and is counted rather than hidden.
            let synced = self.atmosphere_ahead.derived_here();
            let mut atmosphere = self.atmosphere_ahead.cadence(
                &spec,
                tick,
                inputs,
                eye,
                voxel_client::game::GAME_NEW_MOON_TICK,
            );
            let adopted = self.atmosphere_ahead.derived_here() == synced;
            let previous = self.light_field.take().map(std::cell::RefCell::into_inner);
            self.light_field = atmosphere.light.take().map(|light| {
                std::cell::RefCell::new(voxel_client::field::CellField::of_light(light, previous))
            });
            // **The beam the worker marches under, stated once per cadence.** The worker derives
            // its own solver from exactly these four arguments — one pure function, so its beams
            // are this field's beams and not a second opinion about the sky — and every cell it
            // measured for the last cadence is due a re-march, because a cell's bands carry the
            // beam they were marched under.
            // **The material's light is the GPU's now** — PATCH-074. `fs_main` evaluates the
            // atmosphere from `sky_model` at the fragment's own normal and gates the frame's one
            // Sun/Moon triple by the per-frame shadow map; both stream with the frame the
            // geometry lands. The CPU field's demand queue measured cells a chunk at a time and
            // arrived long after the mesh, which is why only the measured chunk was lit and why
            // lighting took seconds where shadows took a frame. Nothing asks it for a cell now:
            // the queue stays empty, `pump_light` is one hash lookup and one empty receive, and
            // the scene's light key stops moving with the beam because there are no per-corner
            // bands left in the picture.
            self.light_ask = None;
            // **The cloud field and its tree are asked for in the geometry section below**, not
            // here: the ask needs the pixel angle, and the derivation itself now runs on
            // [`CloudWorker`]'s own thread — 4 225 model calls and an octree fill are not this
            // thread's work to do every four ticks.
            // **One `Sky`, uploaded three times.** The rig, the background and the Sun's body
            // are written from the same `Daylight`, so the light, the sky it is seen against and
            // the body in it cannot describe different atmospheres. Before the sky pass there was
            // only the rig here and nothing to keep in step with it.
            let sky = atmosphere.sky.take();
            let bodies = std::mem::take(&mut atmosphere.bodies);
            if let Some(daylight) = atmosphere.daylight.as_ref() {
                // **The field's own exposure, taken from the one `Daylight` the rig is exposed
                // with** — so a measured corner and a rig-lit one are lit at one grade, and a
                // twilight's measurements are not read at a noon's exposure.
                self.light_grade = daylight.grade;
                if let Some(graphics) = self.graphics.as_mut() {
                    if let Some(gpu) = GpuRig::from_rig(&voxel_client::skylight::exposed_at(
                        &daylight.rig,
                        daylight.grade,
                    )) {
                        graphics.world.write_rig(&gpu);
                    }
                    cost.sky_light = mark.elapsed();
                    mark = Instant::now();
                    match sky {
                        Some((model, _peak)) => {
                            graphics.world.write_sky_model(&model);
                            // **Both bodies, from the same `Sky` and the same normaliser the
                            // background was drawn against** — derived where the rig was, so the
                            // light, the sky and the body in it cannot describe different skies.
                            // One record each (C-184), carried into the frame by the scene build
                            // below rather than by a uniform written here.
                            self.bodies = bodies;
                        }
                        None => {
                            // The boot frame's own refusal, on this tick: no sky, stated — and
                            // **no bodies**, because a body is drawn against this sky and leaving
                            // the last tick's in place would put a sun in a frame that just said it
                            // has none.
                            graphics.world.write_no_sky_model();
                            self.bodies.clear();
                        }
                    }
                }
            }
            cost.sky_dome = mark.elapsed();
            if std::env::var_os("ATMOSPHERE_LEDGER").is_some() {
                println!(
                    "atmosphere\tt{tick}\t{}\t{:.2} ms\tsync {}\tsky-write {:.2} ms",
                    if adopted { "adopted" } else { "derived" },
                    cadence_started.elapsed().as_secs_f64() * 1e3,
                    self.atmosphere_ahead.derived_here(),
                    (cost.sky_light + cost.sky_dome).as_secs_f64() * 1e3,
                );
            }
            mark = Instant::now();
        }
        // **The cadence after this one, asked for now — every frame, and idempotent.** The
        // derivation is a pure function of a tick that has not happened yet, so it can be *had*
        // before it is needed; asking every frame rather than only after an adoption is what makes
        // it survive a stalled frame, where `Clock::advance` can carry the tick past a cadence the
        // look-ahead had already been asked about.
        //
        // **The inputs it will be derived in, as far as they can be known now.** The turbidity is a
        // pure function of the column and the tick, so it is sampled *for* a tick that has not
        // happened; the ozone is the **held** daily value, which is what the frame still holds unless
        // a climate sample lands in between — and then the answer is not adopted and the frame
        // derives the cadence itself, counted.
        let period = voxel_shapers::stages::Stage::Atmosphere
            .cadence()
            .period_ticks();
        let next = (tick / period + 1) * period;
        if !self.atmosphere_ahead.asked(next) {
            let column = self.camera.position_m;
            let eye = self.render_camera.position_m;
            let spec = *self.spec();
            let next_sampled = voxel_client::skylight::sky_inputs_at(&spec, column[0], column[2], next);
            let held_ozone = self.ozone_atm_cm;
            self.atmosphere_ahead.ask(
                next,
                SkyInputs {
                    turbidity: next_sampled.turbidity,
                    ozone_atm_cm: held_ozone,
                },
                eye,
            );
        }
        let _ = moved;

        // ---- the judgement, at the world's rate ----------------------------
        // H-12.2: **one rendering per record, one at a time.** The stream advances once per
        // tick rather than once per frame, so a decision is shown at the rate the world runs
        // at rather than the rate the terminal can print. Every record is verified before it
        // is shown — `sha256(artifact)` against the artifact's own bytes — and a record that
        // does not verify stops the stream **by name** rather than being shown against bytes
        // the decision was not made from.
        if Some(tick) != self.notified_at {
            self.notified_at = Some(tick);
            self.show_next_notification(tick);
        }

        // ---- the live-reload watch -----------------------------------------
        // **The compiler's output root is the loader's search path** (`packages -> game`), so a
        // package emitted while the game runs is adopted here rather than after a restart: that
        // is the loop the UI is fixed in, and a rebuild would throw the running world — the thing
        // the UI is being fixed against — away on every iteration. The watch reads the directory
        // once per tick, adopts only a package whose digest it has not handled, and says on the
        // console what it adopted or refused.
        if let Some(root) = self.workspace.clone() {
            self.shell.watch_packages(&root.join("packages"), Some(tick));
        }

        cost.judge = mark.elapsed();
        mark = Instant::now();

        // ---- the api layer, read back out of the game ----------------------
        // C-54.3: the console, the task viewer, the toast queue and the panel are **one api** — and
        // the console *is* that api's record surface. It lived only in the window, which left the
        // agent's half of the live-edit loop without a read: the shell would adopt an edited
        // prototype and the only trace would be a line on a screen nobody else can open. So every
        // record the console has taken since the last frame is printed with the tick it landed at,
        // in order and verbatim — the same bytes the player sees, never a summary of them.
        //
        // **Once per tick, and guarded by the length**, so a frame at 100 fps does not copy a
        // console that has not changed.
        let console_len = self.shell.console().len();
        if console_len > self.console_at {
            let lines = self.shell.console().lines();
            // **A console that has dropped records has shifted its own indices**, so a counter
            // carried across that shift would print the wrong line — or skip one — silently. The
            // drop is reported and the read re-anchored to the end, which loses the old lines
            // loudly rather than mislabelling the new ones (D107: a gap says it is a gap).
            if self.shell.console().dropped() != self.console_dropped {
                self.console_dropped = self.shell.console().dropped();
                println!(
                    "console\tt- api: the console dropped {} record(s); this read is re-anchored",
                    self.console_dropped
                );
                if lines.len() < self.console_at {
                    self.console_at = lines.len();
                }
            } else if lines.len() < self.console_at {
                // The same shift with no drop reported: refuse to guess rather than index into a
                // buffer that moved, and say which state made the read stop.
                println!(
                    "console\tt- api: the console holds {} record(s) and this read was at {}; the \
                     read stops rather than printing lines it cannot place",
                    lines.len(),
                    self.console_at
                );
            } else {
                while self.console_at < lines.len() {
                    println!("console\t{}", lines[self.console_at]);
                    self.console_at += 1;
                }
            }
        }


        cost.api = mark.elapsed();
        mark = Instant::now();

        // ---- move -------------------------------------------------------
        self.walk_player(dt);
        // **The demo walk begins once the window has settled**, so the run is a world that stays
        // full while it moves rather than a boot that never catches up.
        let settled = self
            .streamer
            .as_ref()
            .is_some_and(|s| s.len() + s.stale_len() == s.window().count() as usize);
        // **The demo flies once the window has settled, and then keeps flying.** `settled` is a
        // predicate on a world that is filling: a ghost that has moved is a window caught
        // mid-refill, so asking it again every frame stops the camera after its first step —
        // which is exactly what the first bounded run did, 5 s in and 3 m short of a chunk
        // boundary. The gate arms the demo; it does not fly it.
        if self.walk && (self.walk_started || settled) {
            if !self.walk_started {
                // **The demo looks level**, because a walk is along the ground and this is the
                // ghost's first step: the boot's own −0.08 rad pitch would sink it 29 m over the
                // minute a bounded run flies, and a camera under the terrain is a run that cannot
                // show what the ground under it does.
                self.camera.pitch_rad = 0.0;
                self.walk_since.get_or_insert_with(Instant::now);
            }
            self.walk_started = true;
            // **One hop, part-way in.** A ghost that only ever walks a settled window never stands
            // in a chunk as its mesh lands — the window is already resident around it — so the
            // frame this change exists to fix would never be asked of the run. Six chunks puts the
            // eye just outside the five-chunk window, and the arrival that follows is the frame
            // the counter below is about.
            // **A pinned pose stays pinned**: no hop, no cruise, no lift — one eye for the whole
            // run, so a frame that went black can be asked for again.
            if let Some((pin, pitch)) = walk_eye_pin() {
                self.laya.position_m = pin;
                self.camera.position_m = pin;
                if let Some(pitch) = pitch {
                    self.camera.pitch_rad = pitch.to_radians();
                }
            } else {
            let hop_at = self.walk_for.map(|limit| limit * 0.4).unwrap_or(20.0);
            if !self.walk_hopped
                && self.walk_since.is_some_and(|since| since.elapsed().as_secs_f64() >= hop_at)
            {
                self.walk_hopped = true;
                self.laya_hop(6.0 * CHUNK_SIDE as f64);
                println!(
                    "walk-hop\t{} moved to a vantage the window does not hold: x={:.0} z={:.0} m",
                    self.laya.name, self.laya.position_m[0], self.laya.position_m[2],
                );
            }
            // **The demo flies the ghost**, not the player's body: `laya_observation` moves the
            // being and derives the eye from it, the body is left standing, and the world streams
            // around the ghost exactly as it does around a player. Teleporting the body instead
            // would have made the demo a collider being dragged through the world, which is not the
            // camera the audit could not drive.
            // **A measurement, not a way to fly**: `SHPLOINGUS_LIFT_MM_S` holds the ghost
            // still and raises it at that rate, so consecutive recorded frames differ in
            // nothing but the eye's height. The walk had no vertical component at all, which
            // is why every earlier capture of this defect was of a constant-height eye.
            let lift = walk_lift_m_s();
            self.laya_observation(
                dt,
                LayaWish {
                    forward_m_s: if lift == 0.0 { WALK_CRUISE_M_PER_S } else { 0.0 },
                    up_m_s: lift,
                    ..LayaWish::default()
                },
            );
            }
        }
        cost.walk = mark.elapsed();
        mark = Instant::now();

        // ---- drive the world --------------------------------------------
        // Every frame, so the shapers are continuous rather than a load step.
        let centre = self.chunk_of_eye();
        let frame = {
            let streamer = self.streamer.as_mut().expect("checked above");
            streamer.set_centre(centre);
            match streamer.pump() {
                Ok(f) => f,
                Err(refusal) => {
                    eprintln!("shploingus: the streamer refused: {refusal}");
                    event_loop.exit();
                    return;
                }
            }
        };

        cost.stream = mark.elapsed();
        mark = Instant::now();

        // ---- geometry ---------------------------------------------------
        // **Built when the world or its light moved, and kept otherwise.** A corner is
        // chunk-local and the eye travels in the origins array, so the terrain is a function of
        // the resident world and the light's own revision — not of the frame. [`SceneCache`]
        // states exactly that pair as its key, and the pipeline keys its own upload on the
        // scene's build id, so a frame that walks through an unchanged world neither walks the
        // meshes nor sends a byte to the GPU.
        //
        // **And the cloud's own cells, appended after the world's** (C-179). They are the
        // world's own rectangles, so they travel in the same buffers and are drawn by the same
        // fragment shader; what the draw splits on is `Scene::cloud_records`. The rung the cells
        // are cut at is the eye's own pixel angle, so the ladder coarsens as the player walks
        // away from the cloud — which is the whole reason it is measured here and not in the
        // field.
        let pixel_angle_rad = self
            .graphics
            .as_ref()
            .map(|g| voxel_client::pixel_angle_rad(self.camera.fov_y_rad, g.config.height.max(1)))
            .unwrap_or(1.7e-3);
        // **The light the frame is built with: the world's own measurements where it has them, and
        // the rig where it does not.**
        //
        // This is the whole of what "the world is lit by measurement" means in the code: the same
        // scene, built by the constructor that asks the world's field at every corner — and by the
        // record-format constructor when there is no field, which is the frame this game drew
        // before the field existed rather than a dark one. `Scene::build_with_light`'s own doc
        // states why the two are constructors rather than a flag, and
        // `renderer::tests::the_quad_format_and_the_vertex_format_draw_the_same_surface` is what
        // holds them to one picture.
        // **The light's own frame, before the scene that draws it**: adopt what the worker has
        // finished, answer its tree questions by handle, and hand it the next batch. This is the
        // whole of the field's cost on this thread — three channel operations and a bounded
        // handshake — and it is timed into the scene stage because it is the scene's light.
        self.pump_light();
        let field = self.light_field.take();
        // **The light's revision is the field's, not the cadence's.** A corner carries light only
        // when the field measured the cell it stands in; a field holding nothing leaves every corner
        // on the rig, which the shader reads from a uniform written whether the geometry was rebuilt
        // or not. So the atmosphere's tick moves this key only when the field has something to say
        // about a corner — without it, every cadence frame walked the resident meshes to rebuild the
        // bytes it already had.
        let held = field.as_ref().map(|f| f.borrow().held()).unwrap_or(0);
        // **The light's key is the cadence, once there are cells to draw with.** The rebuild is
        // also the ask — every corner is asked, and a stale one is queued for a re-march — so the
        // cadence is the rate at which the drawn cells are kept current, and a field holding
        // nothing still moves nothing.
        // ---- the light horizon, streamed from the octrees ----------------
        // **The frame's own occlusion field, moved with the eye and fed by the chunks the
        // streamer holds.** The cube is recentred on the eye's chunk; a crossing shifts it, and
        // the loop below stamps the chunks that are resident and not yet represented, a bounded
        // number per frame. That is what "streamed" means here: the light the fragment marches
        // arrives with the world rather than in one build behind it, and a chunk that has not
        // landed yet reads as unmeasured for the frames it is absent (its direct shadow is still
        // the shadow map's, which is drawn from the geometry that is present).
        // Nine a frame: a crossing only owes the chunks that entered (docs/lighting-sky-and-moon.md,
        // "The horizon's streaming rules"). The coarse band is sixteen summaries a frame — it
        // reaches 256 m each way, so its first fill is the world's whole skyline rather than a
        // crossing's slab.
        const HORIZON_CHUNKS_PER_FRAME: usize = 9;
        const HORIZON_FAR_CHUNKS_PER_FRAME: usize = 16;
        // **Nothing is withheld from the frame over a missing stamp.** A chunk whose cells are not
        // stamped yet now says so (`horizon_stamped`), and an unmeasured ray is the shadow map's
        // answer rather than an invented one — so the chunk is drawn, and it is drawn with a real
        // measurement. Withholding it instead removed the ground under the player for the frames a
        // crossing's slab was still being stamped, which is the black the player saw.
        //
        // **The eye's own chunk is stamped first, outside the frame's budget**, because it is the
        // one chunk the player is standing on.
        let eye_chunk_stamp = self.chunk_of_eye();
        if let Some(graphics) = self.graphics.as_mut() {
            let eye = self.camera.position_m;
            // **The cell under the eye, before and after the crossing**: a band that reports rock
            // there only once it has been shifted is a move bug, and one that reports it before the
            // move is a stamp that was already wrong.
            let solid_before = graphics.world.horizon_solid(eye).is_some_and(|solid| solid);
            let moved = graphics.world.horizon_recenter(eye);
            if moved && std::env::var_os("SHPLOINGUS_PROBE_DEBUG").is_some() {
                eprintln!(
                    "horizon-recenter: eye {eye:?} before {solid_before} after {:?}",
                    graphics.world.horizon_solid(eye)
                );
            }
            if let Some(streamer) = self.streamer.as_ref() {
                // A chunk the window dropped leaves its trees behind; the volume must drop its cells.
                let resident: std::collections::BTreeSet<(i32, i32, i32)> =
                    streamer.tree_chunks().map(|pos| (pos.x, pos.y, pos.z)).collect();
                graphics.world.horizon_retain(&resident);
                // Before the budget: the chunk under the eye.
                //
                // **A crossing re-stamps it whatever the version map says.** A mark measured on
                // the run this was found at (`SHPLOINGUS_EYE`) read *matter* at the eye's own cell
                // while that chunk's octree read air there, and one forced restamp of the chunk
                // corrected the cell in the same frame — so "this version is already stamped" is
                // not a good enough claim for the one chunk the camera is standing in. The cost is
                // one chunk's walk on a chunk boundary, which is not a frame anyone is watching.
                if let (Some(version), Some(tree)) =
                    (streamer.tree_version(eye_chunk_stamp), streamer.tree_shared(eye_chunk_stamp))
                {
                    if moved || graphics.world.horizon_wants(eye_chunk_stamp, version) {
                        graphics.world.horizon_stamp(eye_chunk_stamp, &tree, version);
                    }
                }
                let mut near_budget = HORIZON_CHUNKS_PER_FRAME;
                let mut far_budget = HORIZON_FAR_CHUNKS_PER_FRAME;
                for pos in streamer.tree_chunks() {
                    // An edit bumps the chunk's version, so its stamp follows the new tree.
                    let Some(version) = streamer.tree_version(pos) else {
                        continue;
                    };
                    if !graphics.world.horizon_wants(pos, version) {
                        continue;
                    }
                    // **Two budgets, because the two bands answer different questions.** The fine
                    // cube holds 27 chunks and a crossing owes nine of them; the coarse band
                    // reaches 256 m each way and fills from the same leaves at its own rate. A
                    // chunk still waiting for its stamp is drawn anyway and asked again next
                    // frame: the budget is the cost of the walk, never a reason to hide ground.
                    let budget = if graphics.world.horizon_covers(pos) {
                        &mut near_budget
                    } else {
                        &mut far_budget
                    };
                    if *budget == 0 {
                        continue;
                    }
                    if let Some(tree) = streamer.tree_shared(pos) {
                        graphics.world.horizon_stamp(pos, &tree, version);
                        *budget -= 1;
                    }
                }
                // **What the band actually holds at the eye**, one line a frame, when the probe
                // is asked for: the slots it has looked at and whether the air above the camera
                // is measured as matter. A frame whose ground is black and whose cells above it
                // are solid is a stamp that put rock where there is none.
                if std::env::var_os("SHPLOINGUS_PROBE_DEBUG").is_some() {
                    let eye = self.camera.position_m;
                    // **The world's own answer beside the band's**, from the streamer's record: the
                    // generator and the stamp are two statements about one cell, and a frame whose
                    // ground is black is explained when they disagree.
                    let voxel = |p: [f64; 3]| {
                        streamer
                            .material_at(
                                p[0].floor() as i32,
                                p[1].floor() as i32,
                                p[2].floor() as i32,
                            )
                            .map(|material| !material.is_empty())
                            .unwrap_or(false)
                    };
                    // **What the octree the eye's chunk was stamped from says**, which is the
                    // only statement about that column that is not the band's own: a band that
                    // reports rock where its own tree holds air has lost the address.
                    let tree_says = streamer.tree_shared(eye_chunk_stamp).map(|tree| {
                        let at = |y: f64| {
                            let local = |value: f64| {
                                (value.floor() as i32).rem_euclid(CHUNK_SIDE) as u32
                            };
                            voxel_model::LocalPos::new(
                                local(eye[0]),
                                local(y),
                                local(eye[2]),
                            )
                            .ok()
                            .map(|pos| !tree.voxel(pos).is_empty())
                        };
                        (at(eye[1]), at(eye[1] + 8.0))
                    });
                    eprintln!(
                        "horizon: coverage {:#x} at-eye {:?} +1 {:?} +3 {:?} +8 {:?} world {:?} {:?} tree {:?}",
                        graphics.world.horizon_coverage(),
                        graphics.world.horizon_solid(eye),
                        graphics.world.horizon_solid([eye[0], eye[1] + 1.0, eye[2]]),
                        graphics.world.horizon_solid([eye[0], eye[1] + 3.0, eye[2]]),
                        graphics.world.horizon_solid([eye[0], eye[1] + 8.0, eye[2]]),
                        voxel(eye),
                        voxel([eye[0], eye[1] + 8.0, eye[2]]),
                        tree_says,
                    );
                }
            }
        }

        // ---- the frame's residency change --------------------------------
        // **The scene the player is standing in, changed where the world changed.** A chunk that
        // landed appends its own slice and is drawn this frame; one that left is dropped in the
        // same place. The frame no longer draws the previous scene while a walk rebuilds it, and
        // the walk itself is gone: the eye's chunk never waits for a window's worth of quads.
        //
        // **The material's light is not a scene input any more** (PATCH-074): `fs_main` reads the
        // atmosphere and the frame's shadow map, so the light key does not move with a measured
        // cell and `held` is zero on every frame. It stays in the key so a later pass that gives
        // the material bit a sample again reuses the same rebuild path.
        let light_revision = voxel_client::SceneKey::light_of(held, self.light_epoch);
        // **The demo walk's own check, before the sync it judges**: was the eye's chunk resident
        // in the streamer this frame? If it was, the sync below must draw it in this same frame.
        let eye_chunk = self.chunk_of_eye();
        let eye_was_resident = self
            .streamer
            .as_ref()
            .is_some_and(|streamer| streamer.drawn(eye_chunk).is_some());
        // **Whether the frame *began* with the eye's chunk in the scene it was drawing**: a chunk
        // that was already held is not an arrival, and one that is held after the sync below
        // without being held before it is exactly the arrival this change is about.
        let eye_held_before = self.resident.holds(eye_chunk);
        self.residency = {
            // Each corner asks the field's own measurement of its cell: the march from the sun
            // through this chunk's octree, adopted from the worker.
            //
            // **And a field that has measured nothing is not handed over at all.** The scene
            // chooses the corner format — the whole world as triangles, four corners and six
            // indices a face — whenever it is given a sampler, because a corner is where a
            // measurement attaches and the beam's shadow is cast from corners. A sampler that
            // answers nothing therefore buys no light **and** costs the record format, the shadow
            // casters' own format, for every face of the world: that is the state the shipped
            // world was in, with `records` empty on every frame while `vertices` ran to the tens of
            // thousands. Until the field has an answer, the frame has no measurement to carry and
            // the record is what it draws; the first answered cell moves the frame back to corners
            // on the light revision's own rebuild.
            let measured = field
                .as_ref()
                .is_some_and(|field| field.borrow().answered() > 0 || field.borrow().adopted() > 0);
            let light_fn = field
                .as_ref()
                .filter(|_| measured)
                .map(voxel_client::field::light_at);
            let light: Option<&voxel_client::LightAt> =
                light_fn.as_ref().map(|f| f as &voxel_client::LightAt);
            self.resident.sync(
                self.streamer
                    .as_ref()
                    .expect("checked above")
                    .drawn_meshes(),
                eye_chunk,
                light,
                Some(&self.displacement),
                Some(&self.refine),
                self.work.geometry(),
                light_revision,
            )
        };
        if self.walk_started {
            self.walk_frames += 1;
            self.walk_totals[0] += 1;
            if !self.resident.holds(eye_chunk) {
                self.walk_absent += 1;
                self.walk_totals[3] += 1;
                if eye_was_resident {
                    self.walk_missed += 1;
                    self.walk_totals[2] += 1;
                }
            } else if eye_was_resident && !eye_held_before {
                // **The arrival, drawn on the frame it arrived on**: the streamer had the chunk
                // this frame, the scene did not when the frame began, and the scene that frame
                // drew has it.
                self.walk_landed += 1;
                self.walk_totals[1] += 1;
            }
        }
        let scene = self.resident.take();
        self.geometry.scene_taken(scene.build);
        self.geometry.residency(self.residency);
        // The frame's own tally, folded into the report's running sum and then put back: the
        // field is `App`'s state and the borrow above was the scene build's.
        if let Some(field) = field.as_ref() {
            let field = field.borrow();
            self.light_totals[0] += field.adopted();
            self.light_totals[1] += field.answered();
            self.light_totals[2] += field.stale();
            self.light_totals[3] += field.unmeasured();
            self.light_totals[4] += field.unresolved();
        }
        self.light_field = field;
        cost.scene = mark.elapsed();
        mark = Instant::now();
        // **The tree is derived only when something it is a function of moves.** The key is the
        // field's own tick, the eye snapped to the domain's sample lattice, and the pixel angle —
        // so walking inside one 128 m cell costs no tree at all, and the fill is paid on the cell
        // boundary rather than on every frame — **and on the worker rather than on this thread**.
        // The eye's snap is not an approximation of where
        // the geometry sits: the domain's own corner is snapped to that lattice, so two eyes in one
        // cell build the same tree.
        let ladder_key = (
            cloud_tick_of(tick),
            snap_to_sample_lattice(self.camera.position_m),
            pixel_angle_rad.to_bits(),
        );
        // **Adopt what the worker has finished, then ask for what the frame wants and does not
        // have.** Adopting is unconditional and newest-last: a tree whose key the eye has since
        // walked away from is still the freshest picture of the sky, and the ask it may have
        // displaced is re-asked below rather than waited on.
        //
        // **And the tree it displaces is kept, not dropped.** A swap between two derivations is a
        // cloud that pops 5 times a second; the two are joined for one cadence by their own weights
        // below, which is the whole of the fix for it.
        while let Some((key, tree)) = self.cloud_worker.finished() {
            self.cloud_tree_previous = self.cloud_tree.take();
            self.cloud_tree = Some(tree);
            self.cloud_tree_at_tick = Some(tick);
            self.cloud_tree_key = Some(key);
        }
        if self.cloud_tree_key != Some(ladder_key) {
            // The field's own tick is the **cadence's**, taken from the key rather than from the
            // clock: a key that moved because the eye crossed a sample lattice still describes the
            // field the atmosphere last recomputed, and sampling it at the live tick would make the
            // cloud a function of the frame rate — the thing the atmosphere's cadence exists to
            // prevent.
            self.cloud_worker.ask(
                ladder_key,
                ladder_key.0,
                self.camera.position_m,
                pixel_angle_rad as f64,
            );
        }
        // **The cloud, as a join of the two most recent derivations.** The newer tree's weight is
        // read from the world's clock — the tick it was derived from against the tick being drawn —
        // so the dissolve is phase-locked to the cadence that produced it: the newer tree reaches
        // full weight exactly as the next one falls due, and a tree held after its own cadence
        // ([`CloudWorker`]'s latency) is drawn at full weight rather than served thinner than it
        // is. Nothing here reads a stopwatch, so the frame rate cannot change the sky.
        //
        // **It is one number for the frame, and it is the shader that spends it.** The scene
        // carries the two trees and which of them each rectangle belongs to; the *weights* are
        // `set_cloud_fade`'s below, next to the draw, so a dissolve costs a uniform write rather
        // than a rebuilt veil per rectangle per frame.
        let cloud_fade = voxel_client::cloud_cubes::fade_weight(
            self.cloud_tree_at_tick.unwrap_or(tick),
            tick,
        );
        let scene = match self.cloud_tree.as_ref() {
            Some(to) => match self.cloud_tree_previous.as_ref() {
                Some(from) => scene.with_cloud_fade(from, to),
                // The first tree of a session fades in over its own cadence instead of appearing
                // between two frames: one tree appended, with clear air as the shader's other side.
                None => scene.with_clouds(to),
            },
            None => scene,
        };
        // **And the frame's own bodies** (C-184), each on its own origin: the Sun and the Moon are
        // records in the scene rather than fields of a uniform, so a frame that has one draws it and the
        // pass places it between the terrain and the cloud. They are their own array rather than the
        // records' own suffix, because the cloud already owns that suffix.
        let scene = scene.with_bodies(&self.bodies);

        // Asked here, **before** `self.graphics` is borrowed mutably below and held
        // across the panel: the water query reads the world, and the draw borrows the
        // renderer, and the two cannot overlap inside one expression.
        let in_water = self.eye_is_in_water();

        // ---- the vector change view ----------------------------------------
        // **Read from the world's record and projected through the camera the world is drawn
        // with**, and asked *before* the renderer is borrowed mutably below and held across the
        // draw: the projection reads the record and the camera, and the draw borrows the
        // renderer, and the two cannot overlap in one borrow. Collected every frame, so an
        // outline expires on the record's own clock; drawn only while the debug lanes are on —
        // **with the shell closed as well** (H-7.9).
        let viewport_dp = {
            let g = self.graphics.as_ref().expect("checked above");
            // **The camera's aspect follows the surface, every frame.** The
            // projection was built once with 16:9, so a resized window drew
            // through the old shape and the world stretched with it. Deriving
            // the aspect here — from the same numbers the viewport uses — means
            // no path (Resized, a re-configure after Outdated/Lost, or a
            // ScaleFactorChanged) can leave the projection behind.
            if g.config.height > 0 {
                self.camera.aspect = g.config.width as f64 / g.config.height as f64;
            }
            voxel_ui::Rect::new(
                0.0,
                0.0,
                g.config.width as f64 / g.scale as f64,
                g.config.height as f64 / g.scale as f64,
            )
        };
        // ---- the camera effects, once per frame -----------------------------
        // **The frame is drawn with a copy of the camera plus this frame's offsets.**
        // The player's own camera is what the movement reads next frame, so a smoothed
        // look can never become a physics input that varies with the frame rate.
        let sprinting = self.held.contains(&KeyCode::ShiftLeft)
            || self.held.contains(&KeyCode::ShiftRight);
        let offsets = self.overhaul.update(
            &self.camera,
            self.player.motion.velocity,
            sprinting,
            dt,
        );
        self.render_camera = offsets.applied_to(&self.camera);
        let render_camera = self.render_camera;
        // The viewport is the shell's, not the frame loop's: a maximized window is sized
        // against it, and a resize is a state change rather than a per-frame argument.
        self.shell.set_viewport(viewport_dp);
        let projected = self.change_strokes(&render_camera, viewport_dp);
        let strokes: Vec<voxel_client::hud::Stroke> =
            if self.lanes { projected } else { Vec::new() };
        cost.camera = mark.elapsed();
        mark = Instant::now();

        let graphics = self.graphics.as_mut().expect("checked above");

        // ---- draw -------------------------------------------------------
        // `wgpu 30` returns these as a **status enum** rather than a `Result`,
        // because most of them are not errors: a timeout, an occluded window and
        // a suboptimal-but-usable frame are all "nothing to draw this time".
        let surface_texture = match graphics.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) => t,
            wgpu::CurrentSurfaceTexture::Suboptimal(t) => {
                // Usable, but the surface wants reconfiguring. Do both: present
                // this frame, and fix the configuration for the next one.
                graphics.surface.configure(&graphics.device, &graphics.config);
                t
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                let (w, h) = (graphics.config.width, graphics.config.height);
                graphics.resize(w, h);
                return;
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                // Skip the frame and try again; the world keeps streaming.
                return;
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                eprintln!("shploingus: a validation error inside get_current_texture");
                return;
            }
        };
        // ---- the shell, and the HUD it hosts -----------------------------
        // Composed here, from values that were all measured, and drawn by the shell's own
        // layer: the bar, the windows, their menus — and, while the shell is hidden, the pinned
        // windows, because the HUD **is** the pinned subset of the same window set (H-9.8).
        // A refusal from the design system is a refusal: the world is not drawn with half a
        // shell over it, and the reason is printed once.
        let elements = {
            // Asked before the literal, not inside it: the panel borrows `self` for
            // several other fields at once, and a query in the middle of that is a
            // second borrow of the same thing in the same expression.
            let state = HudState {
                frame: &frame,
                adapter: &graphics.adapter,
                workers: self.workers,
                fps: self.fps,
                position_m: self.camera.position_m,
                // **The body's own state.** A position alone cannot say whether the
                // player is standing on the ground or falling past it — the two print
                // the same three numbers — and a water readout is the only way to see
                // the sea the body is standing in.
                grounded: self.player.motion.grounded,
                vertical_m_s: self.player.motion.velocity[1],
                in_water,
                quads: scene.quads,
                // **The world's seed, not the constant.** The panel's claim is that
                // the world is a function of `(seed, tick)`, and holding a literal
                // here would make it a claim about a constant rather than about the
                // world actually on screen.
                seed: self.world.seed(),
                // **The live tick, not the constant it started from.** The HUD's claim
                // is that the world is a function of `(seed, tick)`; a readout holding a
                // literal `0` was a claim the client could not support.
                tick: self.clock.tick,
                // Measured once from the same table the shader reads, so the
                // panel reports the shading the frame is actually getting.
                shading: graphics.shading,
                digest: self.streamer.as_ref().expect("checked above").digest(),
                refusal: self.streamer.as_ref().expect("checked above").last_refusal(),
            };
            let views = shell_hud::views(
                &state,
                self.shell.console(),
                self.shell.api_view().cloned(),
            );
            match shell_hud::frame(
                &mut self.shell,
                &voxel_ui::Palette::declared(),
                viewport_dp,
                &views,
            ) {
                Ok(mut elements) => {
                    // The lanes are appended to whichever layer is being drawn, so they are on
                    // screen with the shell closed as well as open (H-7.9).
                    if self.lanes {
                        let shown = self.shell.shell_shown();
                        let set = shell_debug::ui_lanes(&self.shell, shown);
                        match shell_debug::ui_lane_elements(&set, &voxel_ui::Palette::declared()) {
                            Ok(lanes) => elements.extend(lanes),
                            Err(refusal) => {
                                eprintln!("shploingus: the debug lanes refused to draw: {refusal}")
                            }
                        }
                    }
                    // **The reading, not an opinion.** The elements this frame drew are measured for
                    // the two hierarchy defects this shell has been reported for — ink no surface
                    // hosts, and ink a later surface covers — and the reading goes onto the console,
                    // which is the api's record surface. So a UI change is a number that has to
                    // move, and the agent editing the shell can read it without seeing the window.
                    // **The shell's one raster act.** Every element — chrome, title bars, panes,
                    // readouts, the console's rows and the API window's lines — is snapped to the
                    // 8 × 16 cell grid here and re-tinted to the Android 4 schema, so the two
                    // decisions a paint can make are made once for the whole interface. The
                    // pointers below still resolve against the declared rectangles: a paint does
                    // not move a window, and the HUD's bands keep each window's cells under the
                    // window drawn after it.
                    // **The cell version is the mounted package's declaration**, read where the
                    // package was mounted: `package.cells` selects version 1 (this raster) or
                    // version 0 (the elements as composed — the old drawing). The mount is asked
                    // per frame rather than copied here, so a package that arrived mid-run has
                    // already replaced the version; one that was refused did not, and the drawing
                    // on screen keeps drawing.
                    // **The content space is the only place the raster may draw.** The bar, the
                    // title bars, the borders and the panels' chrome are passed through as
                    // composed; only a window's content container is addressable in cells.
                    // **Only the frames the mount graded may become cells.** The mask is the
                    // frame's own footprint inside a declared window's content container, so
                    // content the compiler composed is cells when — and only when — a package's
                    // graded frame claims that rect.
                    let frames = self.shell.frame_space();
                    let elements = match shell_cells::raster(
                        &elements,
                        &frames,
                        self.shell.cell_version().number(),
                    ) {
                        Ok(elements) => elements,
                        Err(refusal) => {
                            eprintln!("shploingus: the cell raster refused: {refusal}");
                            elements
                        }
                    };
                    let reading = shell_debug::drawing(&elements).describe();
                    if self.drawing.as_deref() != Some(reading.as_str()) {
                        self.shell.log(
                            ConsoleLevel::Event,
                            "shell",
                            reading.clone(),
                            Some(self.clock.tick),
                        );
                        self.drawing = Some(reading);
                    }
                    // **The windows that laid out are on screen whatever the ones that did not.**
                    // A refusal names the window it belongs to, and it is printed when it changes
                    // rather than sixty times a second.
                    let detail = self.shell.refusals().join("; ");
                    if self.shell_refusal.as_deref() != Some(detail.as_str()) {
                        if !detail.is_empty() {
                            eprintln!("shploingus: the shell refused to lay out: {detail}");
                        }
                        self.shell_refusal = Some(detail);
                    }
                    Some(elements)
                }
                Err(refusal) => {
                    let detail = refusal.to_string();
                    if self.shell_refusal.as_deref() != Some(detail.as_str()) {
                        eprintln!("shploingus: the shell refused to lay out: {detail}");
                        self.shell_refusal = Some(detail);
                    }
                    None
                }
            }
        };
        if elements.is_some() || !strokes.is_empty() {
            if let Err(e) = graphics.hud.prepare(
                &graphics.device,
                &graphics.queue,
                (graphics.config.width, graphics.config.height),
                graphics.scale,
                elements.as_deref().unwrap_or(&[]),
                &strokes,
            ) {
                eprintln!("shploingus: the HUD could not be prepared: {e}");
                event_loop.exit();
                return;
            }
        }

        cost.shell = mark.elapsed();
        mark = Instant::now();
        let view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
        // **The world pass draws into the multisampled attachment and resolves into the
        // swapchain's own view**, which the HUD's pass then loads and draws over. At one sample
        // there is no attachment to resolve from and the swapchain view is the target itself.
        let msaa_view = graphics
            .colour_msaa
            .as_ref()
            .map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()));
        let (world_view, resolve) = match &msaa_view {
            Some(msaa) => (msaa, Some(&view)),
            None => (&view, None),
        };
        let mut encoder = graphics
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("frame") });
        // **The material layer this frame draws** (F5), set with the other per-frame numbers: it
        // is the frame's view, not the world's, so the scene and its material table are untouched.
        graphics.world.set_debug_layer(self.layer);
        // One pixel's angular size for the swapchain's own height — the pattern
        // ladder's footprint scale, which the shader cannot derive for itself
        // (C-163).
        let pixel_angle =
            voxel_client::pattern::pixel_angle_rad(render_camera.fov_y_rad, graphics.config.height);
        // **The sky's own weight, one number for this frame** (C-183). It is set here rather than
        // in the scene because it is not a property of the geometry: the two trees a frame joins
        // are the same bytes for every frame of the dissolve, and the shader interpolates between
        // them per pixel from `View::screen.w`.
        graphics.world.set_cloud_fade(cloud_fade as f32);
        // **How many corner bytes this frame may send**, from the same owner that sized the build:
        // a 20 MB scene is drafted a slice a frame into the back pair while the front pair draws.
        graphics.world.set_upload_budget(self.work.upload());
        if self.wireframe {
            graphics.world.draw_wireframe(
                &mut encoder,
                world_view,
                &graphics.depth_view,
                resolve,
                &scene,
                &render_camera,
                pixel_angle,
            );
        } else {
            graphics.world.draw(
                &mut encoder,
                world_view,
                &graphics.depth_view,
                resolve,
                &scene,
                &render_camera,
                pixel_angle,
            );
        }
        // **The recorder takes the world alone.** A panel opening over the ground is a change
        // of overlay, not of surface, and a measurement of the surface must not have to tell
        // the two apart — so the shell's pass is skipped while `SHPLOINGUS_CAPTURE_DIR` is set.
        if graphics.capture.is_none() && (elements.is_some() || !strokes.is_empty()) {
            // A second pass, loading what the world drew: no depth, no clear.
            graphics.hud.draw(&mut encoder, &view);
        }
        graphics.queue.submit(Some(encoder.finish()));
        // `wgpu 30` presents through the queue, not the texture: the texture is
        // dropped unless it is handed here, and `SurfaceTexture::drop` discards
        // it — which is the safe default rather than a silent no-op.
        // ---- the frame recorder ------------------------------------------------
        // Before the present: what is copied is the texture this frame was drawn into, with
        // the world and the shell both in it. Off unless `SHPLOINGUS_CAPTURE_DIR` is set.
        // **The ground the corner trace watches, chosen on the first frame that has any.** The
        // capture opens before the world is resident, so the first frames of a run hold no geometry
        // at all; choosing then would fix the trace on a point in mid-air for the whole run. Once
        // chosen it is held in the world, so the trace keeps describing that patch of ground as the
        // camera moves over it.
        let capture_probe = if graphics.capture.is_some() {
            if graphics.capture_probe.is_none() {
                graphics.capture_probe = ground_probe(&scene, render_camera.position_m);
            }
            graphics.capture_probe
        } else {
            None
        };
        if let Some(recorder) = graphics.capture.as_mut() {
            recorder.record(
                &graphics.device,
                &graphics.queue,
                &surface_texture.texture,
                render_camera.position_m,
                render_camera.yaw_rad,
                render_camera.pitch_rad,
                self.clock.tick,
            );
            if let Some(probe) = capture_probe {
                recorder.record_corners(&scene, probe, 6.0);
            }
            recorder.record_scene(&scene);
            recorder.next();
        }
        graphics.queue.present(surface_texture);
        graphics.hud.trim();
        // **The two counts the report prints, read before the scene goes back to its owner.**
        let drawn = (scene.quads, scene.clouds.len());
        // **The terrain back to its owner**, with this frame's cloud and bodies stripped on the
        // next `take`: the resident scene keeps the world's part, which is the part that outlives
        // the frame, and the frame's own additions are what the next frame appends again.
        self.resident.put(scene);
        cost.draw = mark.elapsed();
        self.frames += 1;

        // ---- the instrument ---------------------------------------------
        // **The frame's own split, taken from the code the player's frame ran.** A frame that
        // never reached a present — an occluded or timed-out surface, a lost one — returned
        // above and is deliberately **not** counted: this reports frames that were drawn, and a
        // skipped frame that were counted would show up as a free one.
        cost.total = now.elapsed();
        self.timing.take(tick, &cost);

        // ---- report -----------------------------------------------------
        // The rate is measured over an interval and printed; the panels above are
        // the on-screen report, and they say the same things so that a player
        // watching the window and a developer reading a log see one story.
        let elapsed = now.duration_since(self.title_at).as_secs_f64();
        if elapsed >= 1.0 {
            let fps = self.frames as f64 / elapsed;
            self.fps = Some(fps);
            self.title_at = now;
            self.frames = 0;
            // **The instrument's summary, on the line the loop already prints.** It is the
            // mean split rather than a second frame-rate claim: the rate is above, and this
            // is where the rate went.
            if let Some(split) = self.timing.summary() {
                println!("frame-summary\t{split}");
            }
            // **The light field's own report, once a second and in the world's own numbers.** The
            // claim this game now makes — that the world is lit by measurement rather than by a
            // rig — is a claim about *how many corners were measured*, and a claim of that shape
            // has to be readable while the game runs rather than inferred from the picture. So the
            // five counts are printed beside the frame rate: cells measured this second (adopted
            // from the worker), corners answered from a measurement **whose own beam has since
            // moved past the field's step** (`CellField::stale`), corners that fell back to the
            // rig, and hops made past a chunk the streamer does not hold — with the field's whole
            // holding and its queue behind the five, which is how much of the world is measured at
            // all.
            //
            // The stale count is the one that says whether the field is still *covering* the view:
            // it is high while a turning eye's map catches up and zero once it stands still, and it
            // exists because the alternative — dropping the cell and letting the whole map be
            // re-covered — is what left most of a frame on the rig before this pass.
            //
            // **"Rigged" is the honest half and it is why the line carries a second number.** A
            // budget that ran out and a chunk that was not resident both land there, and a frame
            // lit entirely by the rig would report `0 measured` rather than a black screen.
            let [measured, answered, stale, rigged, unresolved] = self.light_totals;
            if let Some(field) = self.light_field.as_ref() {
                let field = field.borrow();
                println!(
                    "light-field\t{measured} cell(s) measured | {answered} corner(s) answered from the \
                     world's own light | {stale} corner(s) a step behind the light | {rigged} corner(s) \
                     rigged | {unresolved} hop(s) past a chunk not resident | {} cell(s) held | {} queued \
                     | {} in flight",
                    field.held(),
                    field.queued(),
                    self.measurer.as_ref().map(|m| m.in_flight()).unwrap_or(0),
                );
            }
            self.light_totals = [0; 5];
            // **The geometry path's own report**, beside the light field's: how many scenes were
            // actually built and what the corner buffers were sent, which is the argument this
            // change makes rather than the claim it makes.
            let traffic = graphics.world.corner_traffic();
            println!("geometry\t{}", self.geometry.report(traffic));
            if self.walk_started {
                println!(
                    "walk\t{} (ghost) {} frame(s) at x={:.0} z={:.0} m | {} chunk arrival(s) \
                     under the eye drawn in the frame they landed | {} frame(s) with the eye's \
                     chunk resident and not drawn | {} frame(s) with the eye's chunk not in the \
                     scene | {} chunk(s) waiting for a frame's budget",
                    self.laya.name,
                    self.walk_frames,
                    self.camera.position_m[0],
                    self.camera.position_m[2],
                    self.walk_landed,
                    self.walk_missed,
                    self.walk_absent,
                    self.residency.pending,
                );
                self.walk_frames = 0;
                self.walk_landed = 0;
                self.walk_missed = 0;
                self.walk_absent = 0;
                // **A bounded demo ends with the claim, not with a keypress.** The verdict is the
                // whole run's counts, and it is a pass only when the ghost actually crossed a
                // chunk and every crossing under it was drawn on its own frame — `landed > 0` is
                // what stops a stalled camera from passing by never asking the question.
                if let (Some(limit), Some(since)) = (self.walk_for, self.walk_since) {
                    let flown = since.elapsed().as_secs_f64();
                    if flown >= limit {
                        let [frames, landed, missed, absent] = self.walk_totals;
                        println!(
                            "walk-verdict\t{} | {} (ghost) flew {flown:.1} s, {frames} frame(s), \
                             {landed} chunk arrival(s) under the eye drawn on their own frame, \
                             {missed} missed, {absent} frame(s) waiting on the streamer, {} at x={:.0} \
                             z={:.0} m",
                            if missed == 0 && landed > 0 { "PASS" } else { "FAIL" },
                            self.laya.name,
                            self.residency.pending,
                            self.camera.position_m[0],
                            self.camera.position_m[2],
                        );
                        event_loop.exit();
                    }
                }
            }
            println!(
                "frame-work\tceiling {} ms | geometry {} quad(s) a frame | upload {} B a frame, \
                 {} B owed | measure {} cell(s) a frame",
                self.work.ceiling().as_millis(),
                self.work.geometry(),
                self.work.upload(),
                graphics.world.upload_pending(),
                self.work.measure(),
            );
            let (records_ms, corners_ms) = graphics.world.upload_split();
            // **And what the two corner pairs cost for never allocating on a frame.** The capacity
            // is stated once at construction and retained for the process, so it belongs on the
            // same line as the uploads that use it: a reader sees 72.0 MB of GPU memory that the
            // mesh residency and the scratch do not count, and a count that says no frame has been
            // the one that grew it.
            let (quad_uploads, quad_bytes, quad_grows) = graphics.world.quad_traffic();
            let (patched, encoded) = graphics.world.quad_path();
            let (encode_ms, transfer_ms) = graphics.world.quad_split();
            // **A corner buffer the device will not hold is named, once.** The pipeline refuses the
            // ask before `create_buffer` and keeps drawing the pair it has; this is where that
            // refusal reaches a person, with both numbers in it, instead of a frame that silently
            // stops drawing over an invalid buffer.
            let refusals = graphics.world.corner_refusals();
            if refusals > self.corner_refusals_seen {
                if let Some(refusal) = graphics.world.corner_refusal() {
                    eprintln!(
                        "shploingus: refused a corner buffer — {refusal}; the frame keeps the \
                         scene it was drawing"
                    );
                }
                self.corner_refusals_seen = refusals;
            }
            println!(
                "upload-split\trecords {records_ms:.2} ms ({:.1} MB in {quad_uploads} upload(s), \
                 {quad_grows} growth(s), {encoded} re-encoded, {patched} record(s) patched) | encode \
                 {encode_ms:.2} ms + transfer {transfer_ms:.2} ms | corners {corners_ms:.2} ms of the \
                 last frame | corner pairs {:.1} MB retained, {} allocation(s) since start, \
                 {} compaction(s), {refusals} refused",
                quad_bytes as f64 / (1024.0 * 1024.0),
                graphics.world.corner_retained_bytes() as f64 / (1024.0 * 1024.0),
                graphics.world.corner_allocations(),
                self.resident.compactions(),
            );
            graphics.window.set_title(&format!("shploingus — {}", graphics.adapter));
            println!(
                "{:.0} fps | {}/{} chunks | {} stale-but-drawn | {} quads | {} corner(s) ({} live), {} index(es) | {:.1}/{:.1} MB mesh | scratch {:.1} MB | refusals-for-budget {} | evicted {} | in flight {} | {} cloud rect(s) | digest {}",
                fps,
                frame.resident,
                frame.wanted,
                frame.stale_drawn,
                drawn.0,
                self.resident.corners(),
                self.resident.live_corners(),
                self.resident.indices(),
                frame.bytes as f64 / (1024.0 * 1024.0),
                frame.ceiling as f64 / (1024.0 * 1024.0),
                frame.scratch_bytes as f64 / (1024.0 * 1024.0),
                frame.refused_for_budget,
                frame.evicted,
                frame.in_flight,
                drawn.1,
                &self.streamer.as_ref().expect("checked above").digest().to_hex()[..16],
            );
        }
    }

    /// **The vector change view, read from the record and projected for this frame**
    /// (H-7.1/H-7.9, feature `change_view`).
    ///
    /// Collection happens whether or not the lanes are on: a change expires on the record's own
    /// clock, and a region first built at observation time while the view was off would draw as
    /// though it had just happened. The read is guarded by the record's generation, so a frame
    /// with no edit does not copy the record.
    fn change_strokes(
        &mut self,
        camera: &Camera,
        viewport: voxel_ui::Rect,
    ) -> Vec<voxel_client::hud::Stroke> {
        let generation = self.streamer.as_ref().map(|streamer| streamer.world_generation());
        if let Some(generation) = generation {
            if generation != self.change_generation {
                let entries = self
                    .streamer
                    .as_ref()
                    .expect("the generation came from the streamer")
                    .modifications();
                let total = entries.len();
                let start = self.changes_seen.min(total);
                // The instant is the tick the read lands on: the record carries no wall time, and
                // inventing one per entry would be a second clock. Two edits in one tick share
                // the instant, and the view keeps both of them.
                let now_ms = self.clock.tick as f64 * voxel_client::clock::tick_millis();
                for entry in entries.into_iter().skip(start) {
                    // **The record's own reason and material**, so the outline says what changed
                    // and not merely where.
                    match shell_changes::ChangeRegion::of(
                        [[entry.x as i64, entry.y as i64, entry.z as i64]],
                        entry.reason,
                        Some(entry.to),
                        now_ms,
                    ) {
                        Ok(region) => self.changes.push(region),
                        Err(refusal) => self.shell.log(
                            ConsoleLevel::Warn,
                            "world",
                            format!("the change view refused a recorded edit: {refusal}"),
                            Some(self.clock.tick),
                        ),
                    }
                }
                self.changes_seen = total;
                self.change_generation = generation;
            }
        }
        let now_ms = self.clock.tick as f64 * voxel_client::clock::tick_millis();
        self.changes.retain(|region| region.is_live(now_ms));
        let projection = shell_changes::Projection::of(camera, viewport);
        shell_changes::outline_strokes(&self.changes, &projection, now_ms)
    }

    /// The voxel the player is looking at, within [`REACH_M`].
    ///
    /// The ray starts at the **eye**, so a camera buried in terrain targets the
    /// voxel it is inside (`entry_face` is `None` there, and a build against it
    /// is skipped rather than guessed at).
    fn target(&self) -> Option<voxel_work::Pick> {
        let streamer = self.streamer.as_ref()?;
        // **The ray starts at the drawn eye and follows the drawn look**, so the pick
        // agrees with the crosshair the player is actually looking through — the
        // effect offsets are small, but a pick that disagreed with the crosshair
        // would be a bug at any size.
        let ray = Ray::new(self.render_camera.position_m, self.render_camera.forward());
        match streamer.pick(&ray, REACH_M) {
            Ok(found) => found,
            Err(refusal) => {
                // A refusal from the generator is not "nothing there": say which.
                eprintln!("shploingus: the pick refused: {refusal}");
                None
            }
        }
    }

    /// Left button: take the voxel out of the world, recorded.
    fn break_at_target(&mut self) {
        let Some(pick) = self.target() else {
            return;
        };
        let streamer = self.streamer.as_mut().expect("the target came from the streamer");
        match streamer.edit(voxel_work::pool::Edit {
            x: pick.voxel[0],
            y: pick.voxel[1],
            z: pick.voxel[2],
            to: MaterialId::EMPTY,
            reason: "the player broke it".into(),
            actor: voxel_model::world::ActorId::player(),
            verb: voxel_model::world::PhysicalVerb::Break,
        }) {
            // **A transient result goes through `announce`, and a refusal stays a record.**
            // `broke material 4 at (…)` is MD1's snackbar class (UI-SPEC §7), so it is one call
            // that writes the console record *and* raises the toast; the refusal is a `Warn`,
            // which the same call logs and deliberately does not toast.
            Ok(m) => self.shell.announce(
                ConsoleLevel::Event,
                "world",
                format!(
                    "broke material {} at ({}, {}, {})",
                    m.from.0, pick.voxel[0], pick.voxel[1], pick.voxel[2]
                ),
                Some(self.clock.tick),
            ),
            Err(refusal) => self.shell.announce(
                ConsoleLevel::Warn,
                "world",
                format!("{refusal}"),
                Some(self.clock.tick),
            ),
        }
    }

    /// Right button: put the selected material into the empty voxel the ray
    /// entered through.
    ///
    /// Building into **air** is the path. Water is *attempted* and the world's edit
    /// path refuses it — a fluid is not a block and displacement is not modelled
    /// (C-154), so the refusal is announced rather than a water voxel silently
    /// deleted, and the rule lives in one place instead of being spelled again
    /// here. Building into anything else is refused with the material's name,
    /// because silently replacing rock with rock is an edit the player did not ask
    /// for.
    fn build_at_target(&mut self) {
        let Some(pick) = self.target() else {
            return;
        };
        let Some(face) = pick.entry_face else {
            self.shell.announce(
                ConsoleLevel::Warn,
                "world",
                "the eye is inside a voxel; nothing to build against",
                Some(self.clock.tick),
            );
            return;
        };
        let step = outward_step(face);
        let at = [
            pick.voxel[0] + step[0],
            pick.voxel[1] + step[1],
            pick.voxel[2] + step[2],
        ];
        let (material, name) = PLACEABLE[self.selected];
        let streamer = self.streamer.as_mut().expect("the target came from the streamer");
        match streamer.material_at(at[0], at[1], at[2]) {
            Err(refusal) => self.shell.announce(
                ConsoleLevel::Warn,
                "world",
                format!("{refusal}"),
                Some(self.clock.tick),
            ),
            // Water goes through the same door as air, so the edit path is the one
            // place the fluid rule lives and its refusal is what the player reads.
            Ok(existing) if existing.is_empty() || existing == materials::WATER => {
                match streamer.edit(voxel_work::pool::Edit {
                    x: at[0],
                    y: at[1],
                    z: at[2],
                    to: material,
                    reason: "the player built here".into(),
                    actor: voxel_model::world::ActorId::player(),
                    verb: voxel_model::world::PhysicalVerb::Place,
                }) {
                    Ok(_) => self.shell.announce(
                        ConsoleLevel::Event,
                        "world",
                        format!("placed {name} at ({}, {}, {})", at[0], at[1], at[2]),
                        Some(self.clock.tick),
                    ),
                    Err(refusal) => self.shell.announce(
                        ConsoleLevel::Warn,
                        "world",
                        format!("{refusal}"),
                        Some(self.clock.tick),
                    ),
                }
            }
            Ok(existing) => self.shell.announce(
                ConsoleLevel::Warn,
                "world",
                format!("{name} cannot go there — material {} is in the way", existing.0),
                Some(self.clock.tick),
            ),
        }
    }

    /// The player's body, and the camera placed from it.
    ///
    /// One call to [`Player::advance`], which banks the frame delta and spends it in
    /// whole 1/20 s steps — one tick of the Squake model, the world's own clock rate
    /// — against the world's own voxels, so the trajectory is a function of how many
    /// steps have run and not of how the frames arrived. The eye is then *read off*
    /// the body, never written to — and read **between the last two steps**, because
    /// physics at 20 Hz drawn at frame rates above it would otherwise stutter.
    ///
    /// Whether the eye is inside a water voxel — one query, asked about the eye
    /// rather than inferred from the body's motion, because a body can be in water
    /// and perfectly still (standing on the sea floor) and can be moving and not be
    /// in it (falling past a fall).
    fn eye_is_in_water(&self) -> bool {
        let eye = self.camera.position_m;
        self.world
            .voxels(self.streamer.as_ref(), &self.earthworks)
            .is_water(
                eye[0].floor() as i32,
                eye[1].floor() as i32,
                eye[2].floor() as i32,
            )
    }

    /// The world's spec, borrowed. Every shaper call takes `&WorldSpec`, and this is
    /// the one place the App turns its [`App::world`] into one — so there is exactly
    /// one world in this program rather than one per call site.
    fn spec(&self) -> &voxel_shapers::WorldSpec {
        self.world.spec()
    }

    /// **Apply the pointer's owner: the shell while it is shown, the world while it is
    /// hidden — grab *and* visibility, from one place.**
    ///
    /// The owner is already decided elsewhere in this file, and this is the one function
    /// that reads the decision, so the two cannot drift — the alternative is a grab set
    /// in one place and released in another, which is how a player ends up locked in a
    /// window they cannot click.
    ///
    /// `Locked` rather than `Confined` because confining still lets the cursor reach the
    /// edge of a window it has already escaped, and because `Locked` is the mode that
    /// matches the camera: this platform implements it by clipping the cursor to a 1×1
    /// rectangle at its **present** position, which is why the camera's motion source is
    /// [`DeviceEvent::MouseMotion`] rather than cursor positions. That same platform
    /// behaviour is why the centre comes first — a 1×1 clip at the edge the pointer was
    /// last dragged to is a lock the player experiences as "the mouse is not locked".
    /// And it makes the visibility half **load-bearing rather than cosmetic** — winit
    /// does not promise `Locked` hides the cursor, and an unhidden one leaves an arrow
    /// pinned on screen while the player looks around.
    ///
    /// A no-op before the window exists, because there is nothing to grab; re-applied on
    /// focus because the platform drops the grab when the window loses it.
    fn sync_pointer_grab(&mut self) {
        let Some(g) = self.graphics.as_ref() else {
            return;
        };
        let locked = !self.shell.shell_shown();
        let mode = if locked {
            CursorGrabMode::Locked
        } else {
            CursorGrabMode::None
        };
        if locked {
            // **The cursor is moved before the clip is built.** winit's Windows backend
            // clips to a 1×1 rectangle around the cursor's position *at the moment the
            // grab is set*, so the grab alone pins the pointer where it already was — a
            // window edge, most of the time. Centring first makes that rectangle land at
            // the middle of the screen, which is what "the mouse is locked" means to a
            // player. It is harmless on platforms whose `Locked` already recentres.
            let size = g.window.inner_size();
            let centre = PhysicalPosition::new((size.width / 2) as i32, (size.height / 2) as i32);
            if let Err(e) = g.window.set_cursor_position(centre) {
                eprintln!("shploingus: cannot centre the pointer before locking it: {e}");
            }
        }
        if let Err(e) = g.window.set_cursor_grab(mode) {
            // Named rather than swallowed: a grab that silently fails is a camera
            // that cannot be turned, and the player would be told the mouse is
            // broken with no reason given.
            eprintln!("shploingus: cannot set the pointer grab to {mode:?}: {e}");
        }
        // The cursor is visible exactly when the pointer is the shell's.
        g.window.set_cursor_visible(!locked);
    }

    /// The collision asks the **streamer's live record** first, through [`Voxels`],
    /// and the generator only as a fallback. The order is load-bearing: the record a
    /// dig or a placement is written into belongs to the `WorldSource` inside the
    /// streamer, so an adapter holding the App's own empty `Earthworks` asks about a
    /// world that stopped being true the moment the player touched it — walking
    /// through blocks just placed, standing on air just dug. The generator is still
    /// the right fallback, and the reason the player cannot fall through the world at
    /// the edge of the streamed window: the terrain is a pure function of
    /// `(seed, tick)`, so a body can stand on a column that has not been meshed yet.
    fn walk_player(&mut self, dt: f64) {
        let wish = {
            let key = |k: KeyCode| if self.held.contains(&k) { 1.0 } else { 0.0 };
            voxel_client::motion::Wish {
                // **The demo does not drive the body.** `SHPLOINGUS_WALK` flies the ghost
                // ([`App::laya_observation`]), and the player is left standing: a body pushed
                // forward by the demo would be a collider being dragged through the world while
                // the camera it is supposed to be the eye of went somewhere else. These four keys
                // are the player's own, and nothing else moves them.
                forward: key(KeyCode::KeyW) - key(KeyCode::KeyS),
                strafe: key(KeyCode::KeyD) - key(KeyCode::KeyA),
                // Ctrl is the sneak key — the reference's sneak, at 2.2 m/s, and
                // the key a trimp is taken from.
                sneak: self.held.contains(&KeyCode::ControlLeft)
                    || self.held.contains(&KeyCode::ControlRight),
                sprint: self.held.contains(&KeyCode::ShiftLeft)
                    || self.held.contains(&KeyCode::ShiftRight),
                jump: self.held.contains(&KeyCode::Space),
            }
        };
        let world = self.world.voxels(self.streamer.as_ref(), &self.earthworks);
        self.player.advance(&wish, self.camera.yaw_rad, dt, &world);
        self.camera.position_m = self.player.interpolated_eye();
    }

    /// **The floating observer, kept for the Laya god and no longer the player's.**
    ///
    /// The user's ruling: the free camera does not become a player *mode*, it becomes
    /// the god-agent's vantage. So it is not deleted — deleting it would throw away
    /// the one camera a god-agent wants, a thing that can be anywhere and touch
    /// nothing — and it is not bound to the player's keys, which are the body's now.
    ///
    /// **It is piloted by a wish, not by the keyboard and not by the body's position.** A body
    /// answers a `Wish` through the collider; a ghost answers this through nothing at all, and
    /// that difference is the whole of the god's vantage. `SHPLOINGUS_WALK` is the first thing to
    /// fly it: the audit could not drive the camera, so the demo drives *the ghost* — the being a
    /// god looks through — and the player's body is left where it stood.
    ///
    /// **The ghost is the position and the camera is its reading.** The move lands on
    /// [`LayaGhost::position_m`] first and only then on the eye, so nothing can move the vantage
    /// without moving the being it belongs to — which is the same rule the player's body already
    /// states for the player's camera, kept for the row the doctrine left open.
    fn laya_observation(&mut self, dt: f64, wish: LayaWish) {
        let forward = self.camera.forward();
        let (right, up, _) = self.camera.basis();
        let mut delta = [0.0f64; 3];
        let mut add = |axis: [f64; 3], scale: f64| {
            for i in 0..3 {
                delta[i] += axis[i] * scale;
            }
        };
        add(forward, wish.forward_m_s * dt);
        add(right, wish.strafe_m_s * dt);
        add(up, wish.up_m_s * dt);
        for (position, d) in self.laya.position_m.iter_mut().zip(delta) {
            *position += d;
        }
        self.camera.position_m = self.laya.position_m;
    }

    /// **Move the ghost to a farther vantage in one step**, on its own heading and level with the
    /// ground it left.
    ///
    /// This is not a body being teleported: a ghost owns no collider and no mesh, and being
    /// anywhere is the whole of the vantage it stands for. It is how the demo asks the one question
    /// a walk along a settled window cannot — *is the chunk I am standing in drawn on the frame its
    /// mesh arrives?* — instead of waiting for a walk to stumble into the answer.
    fn laya_hop(&mut self, metres: f64) {
        let forward = self.camera.forward();
        let length = (forward[0] * forward[0] + forward[2] * forward[2]).sqrt();
        if length > 1e-9 {
            for axis in [0usize, 2] {
                self.laya.position_m[axis] += forward[axis] / length * metres;
            }
        }
        self.camera.position_m = self.laya.position_m;
    }

    fn chunk_of_eye(&self) -> ChunkPos {
        let side = CHUNK_SIDE as f64;
        ChunkPos::new(
            (self.camera.position_m[0] / side).floor() as i32,
            (self.camera.position_m[1] / side).floor() as i32,
            (self.camera.position_m[2] / side).floor() as i32,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use voxel_model::VoxelPos;

    /// **The pair rule is the platform's two numbers and nothing else.** Each bound is checked
    /// at it and past it, because a `<=` written as a `<` is invisible in a running window and
    /// obvious here.
    #[test]
    fn a_press_pairs_only_inside_the_interval_and_the_rectangle() {
        let now = Instant::now();
        let previous = |millis: u64, at: (f64, f64), button: ShellButton| {
            Some((now - Duration::from_millis(millis), at, button))
        };

        // Inside both bounds is a pair.
        assert!(is_double_click(
            ShellButton::Left,
            (102.0, 102.0),
            now,
            previous(200, (100.0, 100.0), ShellButton::Left)
        ));
        // The interval is inclusive at the bound and not one millisecond past it.
        assert!(is_double_click(
            ShellButton::Left,
            (100.0, 100.0),
            now,
            previous(500, (100.0, 100.0), ShellButton::Left)
        ));
        assert!(!is_double_click(
            ShellButton::Left,
            (100.0, 100.0),
            now,
            previous(501, (100.0, 100.0), ShellButton::Left)
        ));
        // The rectangle is inclusive on both axes, and outside it is not a pair.
        let edge = DOUBLE_CLICK_SLOP_PX;
        assert!(is_double_click(
            ShellButton::Left,
            (100.0 + edge, 100.0 - edge),
            now,
            previous(100, (100.0, 100.0), ShellButton::Left)
        ));
        assert!(!is_double_click(
            ShellButton::Left,
            (100.0 + edge + 0.5, 100.0),
            now,
            previous(100, (100.0, 100.0), ShellButton::Left)
        ));
        assert!(!is_double_click(
            ShellButton::Left,
            (100.0, 100.0 - edge - 0.5),
            now,
            previous(100, (100.0, 100.0), ShellButton::Left)
        ));
        // A different button is never a pair, and neither is a first press.
        assert!(!is_double_click(
            ShellButton::Right,
            (100.0, 100.0),
            now,
            previous(100, (100.0, 100.0), ShellButton::Left)
        ));
        assert!(!is_double_click(
            ShellButton::Left,
            (100.0, 100.0),
            now,
            None
        ));
    }

    /// **A refused cell is not blacklisted.** The worker sends no refusal, so the only memory of
    /// what was asked is the batch in hand; a cell it could not measure is offered again by the
    /// field at the next build, and this is the rule that lets it through.
    #[test]
    fn a_batch_is_deduped_and_a_refused_cell_is_asked_again() {
        let once = VoxelPos::new(4, 0, 400);
        let mut batch = vec![once, VoxelPos::new(4, 0, 401), once];
        dedupe_batch(&mut batch);
        assert_eq!(batch.len(), 2, "a duplicated address was marched twice: {batch:?}");

        // The next batch holds the cell the worker refused, and nothing filters it.
        let mut later = vec![once];
        dedupe_batch(&mut later);
        assert_eq!(
            later,
            vec![once],
            "a cell refused by an earlier ask was never asked again"
        );
    }
}
