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
//! # What is honestly missing
//!
//! * **No physics.** Moving is free flight, so the camera can pass through
//!   terrain. There is no gravity and no collision. **The tick does advance** — it is
//!   `voxel_client::clock`'s `Clock`, advanced by the frame's elapsed seconds and
//!   bounded by `MAX_CATCH_UP` — and it drives the sun, so the light on the world is a
//!   function of it. What the tick does **not** yet drive is the terrain: the shapers
//!   are still tick-independent (C-92.0 item 3 admits that scope and has not landed
//!   it), so a world is still the same voxels at every tick.
//! * **No pointer lock.** Look is driven by `CursorMoved` deltas, so the cursor
//!   stays visible and can leave the window. Pointer grab is a platform detail
//!   worth doing properly rather than approximately.
//! * **No interactive control in the HUD.** Every element is a readout. The
//!   design system's target floor and separation rule are enforced by
//!   `voxel-ui`'s own tests and have no drawn consumer yet, which is recorded as
//!   a debt rather than satisfied by drawing a button that does nothing.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Instant;

use voxel_appearance::sky::SkyInputs;
use voxel_client::shell::{
    console::Key as ShellKey, debug as shell_debug, hud as shell_hud, Button as ShellButton,
    Mount as ShellMount,
};
use voxel_client::{
    Camera, Clock, GpuRig, HudRenderer, HudState, Scene, ShadingReport, WorldPipeline, DEPTH_FORMAT,
};
use voxel_model::coord::CHUNK_SIDE;
use voxel_model::{ChunkPos, Face, MaterialId};
use voxel_mesh::raycast::Ray;
use voxel_shapers::column::materials;
use voxel_shapers::stages::Stage;
use voxel_shapers::{Config, WorldSpec};
use voxel_stream::WorldSource;
use voxel_work::Streamer;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

/// How fast the camera flies, in metres a second.
const FLY_M_PER_S: f64 = 18.0;
const LOOK_RAD_PER_PIXEL: f64 = 0.0022;

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

// The world's window, ceiling, seed and tick are `voxel-client::game`'s now, not
// this file's. That is not tidiness: an ignored test in `voxel-client` renders
// the shipped view from those constants and measures whether the world actually
// reaches the screen, and it cannot do that against numbers that live inside a
// binary crate. See `docs/LEDGER.md` C-36.
use voxel_client::game::{spawn_eye, GAME_CEILING_BYTES as CEILING_BYTES, GAME_PITCH_RAD};
use voxel_client::game::{GAME_SEED as SEED, GAME_TICK as TICK, GAME_WINDOW as WINDOW};

const SHELL_PACKAGE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/alpha-shell.shpkg"));
const PACKAGE_GATE: &str = include_str!(concat!(env!("OUT_DIR"), "/alpha-shell.gate"));

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
    let mount = match ShellMount::new(&shell) {
        Ok(mut mount) => {
            // **The workspace root, found rather than assumed** — the same call the build
            // script makes, and the same reason: every path in the record is relative to it,
            // and an assumed root makes them resolve against the wrong place. It is what lets
            // the console's `file` verb reach `chains/workspace.chain`, and **it is not
            // forced**: a binary run from outside a Cargo workspace has no root, and the verb
            // refuses by name rather than filing somewhere nobody named.
            match voxel_package::features::workspace_root(std::path::Path::new(env!(
                "CARGO_MANIFEST_DIR"
            ))) {
                Some(root) => mount.set_workspace(root),
                None => eprintln!(
                    "shploingus: no workspace root above this crate, so the console's `file` \
                     verb will refuse; the rest of the shell is unaffected"
                ),
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
            mount
        }
        Err(refusal) => {
            eprintln!("shploingus: the shell package cannot be mounted: {refusal}");
            std::process::exit(1);
        }
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
    let mut app = App::new(mount);
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

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("shploingus"),
            ..Default::default()
        }))
        .map_err(|e| format!("no device: {e}"))?;

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
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let (depth, depth_view) = depth_target(&device, width, height);

        // The material table and the light rig are `voxel-client`'s, so the
        // window shades exactly what the offscreen test shades. A registry that
        // cannot be described refuses here rather than shading grey.
        let registry = materials::registry().map_err(|e| format!("the material registry: {e}"))?;
        let table = voxel_client::table(&registry)
            .map_err(|e| format!("a material the appearance layer cannot derive: {e}"))?;
        let world = WorldPipeline::new(&device, &queue, format, &table, boot)
            .map_err(|e| format!("the world pipeline: {e}"))?;
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

        Ok(Graphics {
            window,
            surface,
            device,
            queue,
            config,
            depth,
            depth_view,
            world,
            hud,
            scale,
            shading,
            adapter: adapter_name,
        })
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        let (depth, view) = depth_target(&self.device, width, height);
        self.depth = depth;
        self.depth_view = view;
    }
}

fn depth_target(
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("depth"),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
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
    /// The shell's window manager, mounted from the package `build.rs` compiled. It is not an
    /// `Option`: a window with no shell has no bar, no windows and no HUD — the HUD is the
    /// pinned subset of the same set (H-9.8) — so an app without one is an app with nothing
    /// to draw over the world.
    shell: ShellMount,
    camera: Camera,
    held: BTreeSet<KeyCode>,
    /// Which of [`PLACEABLE`] the next build places.
    selected: usize,
    cursor: Option<(f64, f64)>,
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
    /// The world's clock, **advanced by the frame loop** rather than held at a constant.
    ///
    /// It was [`TICK`] — a `const 0` — for as long as nothing simulated. The reason
    /// recorded against it was *"a HUD that showed a moving tick would be reporting a
    /// simulation this build does not have"*, and that reason is now false in exactly
    /// the way `voxel_client::clock`'s own module doc predicted: the tick drives the
    /// sun, so the simulation this build has is a **lighting** one.
    clock: Clock,
    /// The world the sky is derived from. Held rather than rebuilt per frame because the
    /// tick, the sun and the streamed terrain must all describe **the same world**, and a
    /// spec rebuilt from a constant each frame is a second source of truth waiting to
    /// disagree with the first.
    spec: WorldSpec,
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
    /// The sky's **turbidity**, sampled on [`voxel_shapers::stages::Stage::Weather`]'s
    /// cadence — `Cadence::Water`, every 4 ticks.
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
fn sky_at(
    spec: &WorldSpec,
    daylight: &voxel_client::skylight::Daylight,
    inputs: SkyInputs,
) -> Option<Vec<voxel_appearance::LinearRgb>> {
    let peak = voxel_client::skylight::peak_sun_luminance(spec, inputs);
    if !(peak > 0.0) || !peak.is_finite() {
        return None;
    }
    Some(voxel_client::skylight::sky_table(&daylight.sky, inputs, peak))
}

impl App {
    fn new(shell: ShellMount) -> Self {
        let mut camera = Camera::standard(spawn_eye(), 16.0 / 9.0);
        camera.pitch_rad = GAME_PITCH_RAD;
        App {
            graphics: None,
            streamer: None,
            shell,
            camera,
            held: BTreeSet::new(),
            selected: 0,
            cursor: None,
            last: Instant::now(),
            frames: 0,
            title_at: Instant::now(),
            fps: None,
            workers: 0,
            shell_refusal: None,
            lanes: false,
            clock: Clock::at(TICK),
            spec: WorldSpec::new(SEED, Config::default()),
            rig_at_tick: None,
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
        let weather_due =
            first || (Some(tick) != self.turbidity_at && Stage::Weather.is_due_at(tick));
        let climate_due = first || (Some(tick) != self.ozone_at && Stage::Climate.is_due_at(tick));
        if weather_due || climate_due {
            // **One sample, two fields** — the two stages are read from the same world at
            // the same place, so asking twice would be the same answer twice.
            let sampled = voxel_client::skylight::sky_inputs_at(&self.spec, x, z, tick);
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
            &self.spec,
            self.clock.tick,
            boot_inputs,
            0,
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
        if let Some(table) = sky_at(&self.spec, &boot_daylight, boot_inputs) {
            graphics.world.write_sky(&table);
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
        let spec = self.spec;
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
                    println!(
                        "shploingus: shell {}",
                        if self.shell.shell_shown() { "shown" } else { "hidden" }
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
                    // Look is already active; the click exists so that a window
                    // that has lost focus can be brought back without the camera
                    // jumping, since the delta is measured from the last seen
                    // position.
                    self.cursor = None;
                    // **The shell gets the click first when it is drawn there.** A click that
                    // lands on the bar or a window is the shell's; only a click that lands on
                    // the world is the world's, because digging through a window is a mis-click
                    // rather than a feature.
                    if self.shell.shell_shown() && self.shell.hit(at) {
                        self.shell.pointer_pressed(
                            match button {
                                MouseButton::Right => ShellButton::Right,
                                _ => ShellButton::Left,
                            },
                            at,
                        );
                        return;
                    }
                    match button {
                        MouseButton::Left => self.break_at_target(),
                        MouseButton::Right => self.build_at_target(),
                        _ => {}
                    }
                } else {
                    self.shell.pointer_released(at);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let now = (position.x, position.y);
                let scale = self.graphics.as_ref().map(|g| g.scale as f64).unwrap_or(1.0);
                self.shell.pointer_moved((now.0 / scale.max(1e-6), now.1 / scale.max(1e-6)));
                // The camera looks only while the shell is hidden. With the shell up the pointer
                // belongs to the windows — dragging one is not a camera move — and when the
                // shell goes down the delta is measured from the last seen position, so the
                // world does not jump.
                if !self.shell.shell_shown() {
                    if let Some((lx, ly)) = self.cursor {
                        let dx = now.0 - lx;
                        let dy = now.1 - ly;
                        self.camera.yaw_rad += dx * LOOK_RAD_PER_PIXEL;
                        self.camera.pitch_rad = (self.camera.pitch_rad - dy * LOOK_RAD_PER_PIXEL)
                            .clamp(
                                -voxel_client::camera::MAX_PITCH_RAD,
                                voxel_client::camera::MAX_PITCH_RAD,
                            );
                    }
                }
                self.cursor = Some(now);
            }
            WindowEvent::CursorLeft { .. } => {
                self.cursor = None;
                self.shell.pointer_left();
            }
            WindowEvent::RedrawRequested => self.redraw(event_loop),
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

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        if self.graphics.is_none() || self.streamer.is_none() {
            return;
        }

        // ---- time -------------------------------------------------------
        let now = Instant::now();
        let dt = (now - self.last).as_secs_f64().min(0.1);
        self.last = now;

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
        let due = Some(tick) != self.rig_at_tick
            && voxel_shapers::stages::Stage::Atmosphere.is_due_at(tick);
        if due {
            self.rig_at_tick = Some(tick);
            // **`None` keeps the previous rig**, which is `WorldPipeline::write_rig`'s own
            // refusal semantics too, so a refused atmosphere costs a held frame rather
            // than a black one.
            let inputs = self.atmosphere();
            // **One `Sky`, uploaded twice.** The rig and the background are written
            // from the same `Daylight`, so the light and the sky it is seen against
            // cannot describe different atmospheres. Before the sky pass there was
            // only the rig here and nothing to keep in step with it.
            if let Some(daylight) = voxel_client::skylight::daylight_of(&self.spec, tick, inputs, 0)
            {
                if let Some(graphics) = self.graphics.as_ref() {
                    if let Some(gpu) =
                        GpuRig::from_rig(&voxel_client::skylight::exposed(&daylight.rig))
                    {
                        graphics.world.write_rig(&gpu);
                    }
                    if let Some(table) = sky_at(&self.spec, &daylight, inputs) {
                        let _ = graphics.world.write_sky(&table);
                    }
                }
            }
        }
        let _ = moved;


        // ---- move -------------------------------------------------------
        self.fly(dt);

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

        // ---- geometry ---------------------------------------------------
        // Rebuilt each frame because the residency changes each frame. The
        // vertex buffers are re-uploaded per frame inside the pipeline — a named
        // cost, see `voxel_client::world`.
        let scene = Scene::build(
            self.streamer.as_ref().expect("checked above").meshes(),
            self.camera.position_m,
        );

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
            let scale = graphics.scale;
            let (pw, ph) = (graphics.config.width as f64, graphics.config.height as f64);
            let viewport_dp = voxel_ui::Rect::new(0.0, 0.0, pw / scale as f64, ph / scale as f64);
            // The viewport is the shell's, not the frame loop's: a maximized window is sized
            // against it, and a resize is a state change rather than a per-frame argument.
            self.shell.set_viewport(viewport_dp);
            let state = HudState {
                frame: &frame,
                adapter: &graphics.adapter,
                workers: self.workers,
                fps: self.fps,
                position_m: self.camera.position_m,
                quads: scene.quads,
                seed: SEED,
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
            let views = shell_hud::views(&state, self.shell.console());
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
        if let Some(elements) = &elements {
            if let Err(e) = graphics.hud.prepare(
                &graphics.device,
                &graphics.queue,
                (graphics.config.width, graphics.config.height),
                graphics.scale,
                elements,
            ) {
                eprintln!("shploingus: the HUD could not be prepared: {e}");
                event_loop.exit();
                return;
            }
        }

        let view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = graphics
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("frame") });
        graphics
            .world
            .draw(&mut encoder, &view, &graphics.depth_view, &scene, &self.camera);
        if elements.is_some() {
            // A second pass, loading what the world drew: no depth, no clear.
            graphics.hud.draw(&mut encoder, &view);
        }
        graphics.queue.submit(Some(encoder.finish()));
        // `wgpu 30` presents through the queue, not the texture: the texture is
        // dropped unless it is handed here, and `SurfaceTexture::drop` discards
        // it — which is the safe default rather than a silent no-op.
        graphics.queue.present(surface_texture);
        graphics.hud.trim();
        self.frames += 1;

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
            graphics.window.set_title(&format!("shploingus — {}", graphics.adapter));
            println!(
                "{:.0} fps | {}/{} chunks | {} stale-but-drawn | {} quads | {:.1}/{:.1} MB mesh | scratch {:.1} MB | refusals-for-budget {} | evicted {} | in flight {} | digest {}",
                fps,
                frame.resident,
                frame.wanted,
                frame.stale_drawn,
                scene.quads,
                frame.bytes as f64 / (1024.0 * 1024.0),
                frame.ceiling as f64 / (1024.0 * 1024.0),
                frame.scratch_bytes as f64 / (1024.0 * 1024.0),
                frame.refused_for_budget,
                frame.evicted,
                frame.in_flight,
                &self.streamer.as_ref().expect("checked above").digest().to_hex()[..16],
            );
        }
    }

    /// The voxel the player is looking at, within [`REACH_M`].
    ///
    /// The ray starts at the **eye**, so a camera buried in terrain targets the
    /// voxel it is inside (`entry_face` is `None` there, and a build against it
    /// is skipped rather than guessed at).
    fn target(&self) -> Option<voxel_work::Pick> {
        let streamer = self.streamer.as_ref()?;
        let ray = Ray::new(self.camera.position_m, self.camera.forward());
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
        match streamer.edit(pick.voxel[0], pick.voxel[1], pick.voxel[2], MaterialId::EMPTY, "the player broke it") {
            Ok(m) => println!(
                "shploingus: broke material {} at ({}, {}, {})",
                m.from.0, pick.voxel[0], pick.voxel[1], pick.voxel[2]
            ),
            Err(refusal) => eprintln!("shploingus: {refusal}"),
        }
    }

    /// Right button: put the selected material into the empty voxel the ray
    /// entered through.
    ///
    /// Building into **water** is allowed — that is how a floor is laid — and
    /// building into anything else is refused with the material's name, because
    /// silently replacing rock with rock is an edit the player did not ask for.
    fn build_at_target(&mut self) {
        let Some(pick) = self.target() else {
            return;
        };
        let Some(face) = pick.entry_face else {
            println!("shploingus: the eye is inside a voxel; nothing to build against");
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
            Err(refusal) => eprintln!("shploingus: {refusal}"),
            Ok(existing) if existing.is_empty() || existing == materials::WATER => {
                match streamer.edit(at[0], at[1], at[2], material, "the player built here") {
                    Ok(_) => println!("shploingus: placed {name} at ({}, {}, {})", at[0], at[1], at[2]),
                    Err(refusal) => eprintln!("shploingus: {refusal}"),
                }
            }
            Ok(existing) => println!(
                "shploingus: {name} cannot go there — material {} is in the way",
                existing.0
            ),
        }
    }

    /// Free flight. No gravity, no collision — said plainly in the module
    /// comment rather than left for the player to discover.
    fn fly(&mut self, dt: f64) {
        let forward = self.camera.forward();
        let (right, up, _) = self.camera.basis();
        let mut delta = [0.0f64; 3];
        let mut add = |axis: [f64; 3], scale: f64| {
            for i in 0..3 {
                delta[i] += axis[i] * scale;
            }
        };
        let speed = if self.held.contains(&KeyCode::ShiftLeft) { FLY_M_PER_S * 4.0 } else { FLY_M_PER_S };
        if self.held.contains(&KeyCode::KeyW) {
            add(forward, speed * dt);
        }
        if self.held.contains(&KeyCode::KeyS) {
            add(forward, -speed * dt);
        }
        if self.held.contains(&KeyCode::KeyD) {
            add(right, speed * dt);
        }
        if self.held.contains(&KeyCode::KeyA) {
            add(right, -speed * dt);
        }
        if self.held.contains(&KeyCode::Space) {
            add(up, speed * dt);
        }
        if self.held.contains(&KeyCode::ControlLeft) {
            add(up, -speed * dt);
        }
        for (position, d) in self.camera.position_m.iter_mut().zip(delta) {
            *position += d;
        }
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
