//! **U1 — the client's own controller: a person drives the world the client draws (2026-10-14e).**
//!
//! # What was missing, and what owns it now
//!
//! Until this pass the drawn reading's own eye was a **function of the region's extent** and of nothing else:
//! the residency held the frame's cells, the frame reproduced to the byte, and **no person could move the
//! camera**. This module is the missing half of U1's own bar (*a visible client whose check is the band count
//! and a screenshot*) — *a client a person can drive* — and it owns exactly two things:
//!
//! | concern | owner |
//! |---|---|
//! | **where the eye stands and which way it looks**, and what a person is asking for | [`crate::viewpoint::Viewpoint`] and [`crate::viewpoint::Wish`] — one state, one intent vocabulary |
//! | **the step** that turns one wish into the whole client's next state | [`crate::surface::step_driven`] — the eye moves, the two keys are read off the eye's own new place, the residency is observed there and the frame is drawn from there |
//! | **a session of those steps** — the client's own state carried across a person's input, with what each step cost | [`Driven`] here |
//! | **the two translations from a person to a wish** | [`crate::surface::platform`] (the window's own keys) and `main.rs`'s `drive` mode (the shell's own words) |
//!
//! Nothing else in the crate knows a key or a word: both translations produce a [`Wish`] and both hand it to
//! the same step, so a held key and a typed word cannot move the camera two different ways.
//!
//! # What one step is, and what it is not
//!
//! A step is **the camera's own**: the eye moves ([`crate::viewpoint::FLY_M_PER_FRAME`] along the wish's own
//! unit direction, decomposed on the eye's own basis), the look turns, the pitch is clamped at the parent's
//! own pole, and then the frame is drawn from the eye's own place. It does **not** tick the world's law — the
//! law and the camera are two clocks, and this module drives only the second. The window's own loop ticks the
//! law beside this call (one tick a frame, §36 ruling 3) and the shell's `drive` mode holds it still, which
//! is what makes *the frame comes back byte for byte when the eye comes back* a property of the code rather
//! than of the world happening to settle.
//!
//! # The parent's own controller, read rather than depended on
//!
//! The parent's controller is `crates/voxel-client/src/motion.rs` (`Wish`, `Motion`, `Player`) with its
//! camera in `crates/voxel-client/src/camera.rs`, and its bin translates keys to intent at
//! `crates/shploingus/src/main.rs` (`KeyW`/`KeyS` forward and back, `KeyD`/`KeyA` right and left,
//! `Space`/`ControlLeft` up and down, all at `FLY_M_PER_S`). Those bytes were **read** and are this module's
//! source: the intent's own names, the diagonal rule (one unit direction, so holding two keys is not faster
//! than holding one), the pitch clamp at `MAX_PITCH_RAD`, and *one owner for the eye* — the parent derives
//! its camera from its body every frame rather than moving both, which is why [`crate::drawn::Camera`] is
//! **derived** from a [`Viewpoint`] here ([`crate::surface::frame_of`]) and never kept beside it.
//!
//! They are **not a path dependency**, and the blocker is the crate rather than any signature:
//! `voxel-client` is the only crate that holds `motion.rs`, and it depends on `wgpu`, `glyphon` and
//! `pollster` for its renderer — while this crate's own manifest declares `sha2` and the two ported
//! appearance seams and nothing else. The quoted signature, the command that produced the quote and the
//! smallest next step that would remove the blocker are recorded in
//! `docs/baselines/port-appearance-seam/parent-blocker-2026-10-14e.txt`, with the exact bytes this seam read
//! pinned by path + sha256 in `docs/baselines/port-appearance-seam/pinned-parent-bytes.txt`.
//!
//! # The body (2026-10-14g), and the step-up that cannot fire
//!
//! **The free-flying eye is no longer what a person is.** A [`Viewpoint`] holds a [`crate::body::Body`] and the
//! eye is **derived** from its feet, so `Driven` opens a person who **walks**, stands on the world's own floor,
//! is blocked by the world's own cells, is carried down by the parent's own gravity and stands back up — while
//! flight is a key (`F`, in the window) a person takes rather than what they are. `check_u1_controller` below
//! measures that on the **real surface**: walks from the person's own start over the client's own cells, a jump
//! onto a step, a fall that lands, and a stop the terrain answered for.
//!
//! **`STEP_UP_M` cannot fire on this world, and that is the parent's own finding.** Voxels are one metre cubes,
//! so the smallest real rise a body can meet is a whole metre: the parent's `STEP_UP = 0.55` is armed and never
//! fires, the parent's record says so in as many words (*"every real ledge is a jump"*, `docs/LEDGER.md` §95.5)
//! and leaves the number to the user. So a real ledge is climbed here by the held **jump** — `8.4` m/s of
//! vertical speed clears a metre — and the run prints the walk-climb it measured (zero, every direction)
//! **beside** the jump that did the climbing, rather than letting a number that cannot fire look like one that
//! did.
//!
//! # What is owed, named here rather than implied
//!
//! * **The rest of the parent's walk.** Friction against a surface's own slipperiness, the air-acceleration
//!   cap, air resistance, **sprint and sneak speeds**, step banking for a frame-rate-independent clock, trimping
//!   and sharking, and swimming: the parent's own `Motion` is deeper than the one law [`crate::body`] ports, and
//!   the body's own header names each of them as owed.
//! * **A pointer.** Look is turned by a key; a raw device motion is the parent's door and this client has no
//!   device event of its own.
//! * **A frame-rate-independent step.** One step is one frame, this client's own declared clock; a person on
//!   a slower machine moves a shorter distance in the same wall-clock second. The parent banks frame time;
//!   this client's honest repair is a step per tick with a banked remainder, and it is not built here.
//! * **A world wider than its own horizon.** The residency's horizon **is** the drawn region's own reach
//!   ([`crate::surface::DRAWN_RESIDENCY_M`]), and this world is `4` chunk(s) of a `45.2548` m diagonal — so
//!   every chunk of it is inside the horizon from anywhere a person can stand, and no step can make one
//!   leave. A person driving a world wider than the horizon would evict and re-derive as the anchor moved,
//!   and that is the measurement this client cannot make about itself yet.

use crate::automaton::Automaton;
use crate::body::{self, Body, Move};
use crate::chunk::CubicWorld;
use crate::coordinates::{ChunkKey, VoxelPos, CHUNK_SIDE};
use crate::drawn::Drawn;
use crate::refusal::Refusal;
use crate::stream::Streamer;
use crate::surface::{self, DrawnCounts, DrivenStep};
use crate::viewpoint::{Viewpoint, Wish};

/// **A driven client** — the client's own state, carried across a person's own steps.
///
/// One owner per fact, and none of them is a second copy: the **world** is the law's own state
/// ([`Automaton`]), the **resident set** is the streamer's own ([`Streamer`], whose held cells are the
/// world's own store), and the **eye** is a [`Viewpoint`]. The session adds only what a *session* has: how
/// many steps it has taken, what the residency did over them, and the frames' own counters.
///
/// It draws nothing on [`Driven::open`] (a client that has not been asked for a frame has none), and
/// [`Driven::frame`] is the one way to get the current one — so a caller that only wants to *move* a camera
/// does not pay for a picture, and a caller that prints the picture cannot get one that does not belong to
/// the state it just stepped to.
pub struct Driven {
    plane: Automaton,
    residency: Streamer,
    viewpoint: Viewpoint,
    /// The plan column the **declared view's own eye** stands in — the column [`surface::person_of`] searches
    /// rings about and prefers a level one beside. Kept here so *the person opens in the instrument's own plan
    /// column* is a measured sentence rather than a claim the binary makes about itself (2026-10-14g).
    lens_column: (i32, i32),
    /// The chunk the eye stood over at the last step ([`Viewpoint::chunk`]), and the region its frame was a
    /// picture of ([`Viewpoint::region_origin`]) — read once per step, from the eye's own place.
    anchor: ChunkKey,
    region: ChunkKey,
    counts: DrawnCounts,
    steps: u64,
    derived: u64,
    evicted: u64,
    stale: u64,
}

impl Driven {
    /// **Open a client a person can drive**: the world the client shows ([`surface::client_state`]), the
    /// client's own declared residency of it, and **the person the client is** ([`surface::person_of`]): the
    /// declared view's own plan column with the feet on the world's own ground, **walking** (2026-10-14g). So
    /// the walk starts where the recorded frames' own lens stands in plan, and a person who drives away and
    /// comes back is standing where the reading has always looked from — 1.62 m above a floor rather than at
    /// the instrument's own altitude.
    pub fn open() -> Result<Driven, Refusal> {
        let plane = surface::client_state()?;
        let residency = surface::residency_of(plane.world())?;
        let world = plane.world();
        let mesh = surface::region_mesh(&residency, surface::DRAWN_CHUNK)?;
        let viewpoint = surface::person_of(&mesh, world)?;
        let lens = surface::home_viewpoint(&mesh)?;
        let lens_column = (lens.eye()[0].floor() as i32, lens.eye()[1].floor() as i32);
        Ok(Driven {
            anchor: viewpoint.chunk(world)?,
            region: viewpoint.region_origin(world, surface::DRAWN_CHUNKS)?,
            plane,
            residency,
            viewpoint,
            lens_column,
            counts: DrawnCounts::default(),
            steps: 0,
            derived: 0,
            evicted: 0,
            stale: 0,
        })
    }

    /// The world the law is stepping — the client's own state.
    pub fn plane(&self) -> &Automaton {
        &self.plane
    }

    /// The person's own eye and look.
    pub const fn viewpoint(&self) -> Viewpoint {
        self.viewpoint
    }

    /// The plan column the declared view's own eye stands in, which the person's own column is searched
    /// beside rather than assumed to be.
    pub const fn lens_column(&self) -> (i32, i32) {
        self.lens_column
    }

    /// The chunk the eye stood over at the last step — the anchor the residency was observed at.
    pub const fn anchor(&self) -> ChunkKey {
        self.anchor
    }

    /// The plan corner of the region the last frame was a picture of.
    pub const fn region(&self) -> ChunkKey {
        self.region
    }

    /// The resident set itself — the world's own store, holding the chunks the horizon keeps.
    pub const fn residency(&self) -> &Streamer {
        &self.residency
    }

    /// How many steps this session has taken, and what the residency did over them.
    pub const fn steps(&self) -> u64 {
        self.steps
    }

    pub const fn derived(&self) -> u64 {
        self.derived
    }

    pub const fn evicted(&self) -> u64 {
        self.evicted
    }

    pub const fn stale(&self) -> u64 {
        self.stale
    }

    /// The frames' own counters, summed over every frame this session drew.
    pub const fn counts(&self) -> &DrawnCounts {
        &self.counts
    }

    /// **One step of a person's own input** — the eye moves, the two keys are read off its own new place, the
    /// residency is observed at that anchor and the frame is drawn from the eye's own place
    /// ([`surface::step_driven`], the one door the window's own loop comes through too). The step's own work
    /// is counted here, so a session can print what its input cost rather than describing it.
    pub fn step(&mut self, wish: &Wish) -> Result<DrivenStep, Refusal> {
        let step = surface::step_driven(
            &self.plane,
            &mut self.residency,
            &mut self.viewpoint,
            wish,
            surface::frame_seconds(),
        )?;
        self.anchor = step.anchor;
        self.region = step.region;
        self.steps += 1;
        self.derived += step.stream.derived as u64;
        self.evicted += step.stream.evicted as u64;
        self.stale += step.stream.stale as u64;
        self.counts.add(&step.drawn);
        Ok(step)
    }

    /// **The frame the eye now draws** — [`surface::draw_from`] at this session's own residency, eye and
    /// region: the same call [`Driven::step`] makes, so a frame read here and a frame stepped to are drawn by
    /// one path.
    pub fn frame(&self) -> Result<Drawn, Refusal> {
        surface::draw_from(&self.residency, &self.viewpoint, self.region)
    }

    /// **Where the person is, and how they traverse** — walking or flying, on the ground or in the air
    /// ([`Viewpoint::traversed`]), so a run's own line cannot leave a reader guessing which body moved.
    pub fn traversed(&mut self, traversal: crate::body::Traversal) {
        self.viewpoint.traverse(traversal);
    }

    /// **The person's own place, named** — the eye, the feet, the traversal, the heading and pitch, the chunk
    /// they stand over and the region their frame is a picture of, for a run's own line.
    pub fn described(&self) -> String {
        format!(
            "{}, over chunk ({}, {}, {}) with its frame a picture of the region ({}, {}, {})",
            self.viewpoint.described(),
            self.anchor.x,
            self.anchor.y,
            self.anchor.z,
            self.region.x,
            self.region.y,
            self.region.z
        )
    }
}

/// **How many steps the controller's own measurement walks** — one step west (: `strafe: -1`) is `0.6` m and
/// the declared view's own eye stands **exactly on the plan boundary** at `x = 32.0` (`CLIENT_CHUNKS`'
/// own chunk lattice, `surface::home_viewpoint`'s own placement), so the **first** step already crosses into
/// another chunk of the world's own key law: the eye's own chunk moves from `(1, 1, ·)` to `(0, 1, ·)` and the
/// person's own anchor moves with it. Eight steps carry the eye `4.8` m west and back again, which is enough
/// for the walk to be a walk rather than a step and short enough that every one of its frames can be drawn
/// (a region mesh and a frame cost about a quarter of a second here).
pub const WALK_STEPS: usize = 8;

/// **How high above its own floor a body is dropped**, in metres — the fall this check measures on the world
/// the client draws: high enough that gravity is unmistakably what brought it down, low enough that the fall
/// is a few frames of the client's own clock.
const FALL_FROM_M: f64 = 4.0;

/// **The directions the body's own walk is measured in**, in plan — the four axes and the four diagonals, so a
/// walk that is stopped is stopped by the terrain rather than by one unlucky bearing. Each is the unit
/// direction the body holds for the whole walk.
const PLAN_DIRECTIONS: [(&str, [f64; 2]); 8] = [
    ("north", [0.0, 1.0]),
    ("north-east", [1.0, 1.0]),
    ("east", [1.0, 0.0]),
    ("south-east", [1.0, -1.0]),
    ("south", [0.0, -1.0]),
    ("south-west", [-1.0, -1.0]),
    ("west", [-1.0, 0.0]),
    ("north-west", [-1.0, 1.0]),
];

/// **How many frames one direction's walk is given**: at the client's own frame and `WALK_M_PER_S` that is
/// about `130` m, more than this world's own side, so a walk the terrain does not stop is stopped by the
/// world's own edge — and the run says which of the two it was rather than assuming the flattering one.
const WALK_PROBE_STEPS: usize = 900;

/// **How many frames in a row a grounded body must be refused before its walk is called stopped** — a body
/// pressed against terrain is refused every frame it pushes, so a handful is enough and the count is printed.
const STUCK_FRAMES: usize = 10;

/// **How many frames a jump is given to come down.** A jump at `8.4` m/s in this gravity is aloft for about
/// half a second — fifteen frames — so this is the bound, not the expectation.
const AIRBORNE_FRAMES: usize = 60;

/// **What one direction's walk over the real world did** — the body's own law run against [`body::Cells`] of
/// the world the client draws, with no mesh, no residency and no frame in the way: the plainest way to ask
/// *does the drawn world stop a body, and what does gravity do to it there*.
#[derive(Clone, Copy, Debug, PartialEq)]
struct WalkOverWorld {
    /// The bearing this walk was taken on.
    direction: &'static str,
    /// The unit plan direction it held, in metres per metre — what tells *the world refused the walk where the
    /// body stands* from *the walk reached the world's own edge*, since the feet themselves stay inside it.
    plan: [f64; 2],
    /// How many frames the walk (and its one jump) took.
    frames: usize,
    /// How far the feet travelled in plan, in metres.
    plan_m: f64,
    /// How far the **walk** raised the body through the step-up retry, in metres — the number §95.5 says can
    /// only ever be zero on one-metre voxels.
    walked_up_m: f64,
    /// How far gravity brought the feet down, in metres.
    fell_m: f64,
    /// How many frames a horizontal move was refused outright.
    blocked_frames: usize,
    /// Whether the walk ended **stopped** — refused on [`STUCK_FRAMES`] frames in a row while standing.
    stopped: bool,
    /// The feet at the end of the walk.
    feet: [f64; 3],
    /// Whether the feet ended on the world's own floor.
    grounded: bool,
}

/// **One direction's walk over the world's own cells.**
///
/// The body holds one direction for up to [`WALK_PROBE_STEPS`] frames against [`body::Cells`] of the world the
/// client draws, and the walk ends when the world has refused it on [`STUCK_FRAMES`] frames in a row while it
/// stands — a body pressed against what is there stays stopped, so pushing on is not a measurement. What this
/// measures is **where the world refuses a walk and what gravity does along it**; whether the body can get
/// *up* a step at all is measured separately — [`read_the_surface`] first, to say whether this world holds one,
/// and [`jump_over_the_floor`] for the jump this world's own ledges are taken by — because a bearing that
/// happens to hold no step would say nothing about whether the body could climb one.
fn walk_over(world: &CubicWorld, direction: &'static str, plan: [f64; 2], from: [f64; 3]) -> WalkOverWorld {
    let cells = body::Cells::over(world);
    let dt = surface::frame_seconds();
    let length = (plan[0] * plan[0] + plan[1] * plan[1]).sqrt();
    let along = [plan[0] / length, plan[1] / length, 0.0];
    let walk = Move { along, jump: false };
    let mut body = Body::standing(from);
    let mut over = WalkOverWorld {
        direction,
        plan: [along[0], along[1]],
        frames: 0,
        plan_m: 0.0,
        walked_up_m: 0.0,
        fell_m: 0.0,
        blocked_frames: 0,
        stopped: false,
        feet: from,
        grounded: true,
    };
    let mut stuck = 0;
    for _ in 0..WALK_PROBE_STEPS {
        let step = body.step(dt, &cells, &walk);
        over.frames += 1;
        over.plan_m += step.plan_m;
        over.walked_up_m += step.climbed_m;
        over.fell_m += step.fell_m;
        if step.blocked {
            over.blocked_frames += 1;
        }
        if step.blocked && body.grounded() {
            stuck += 1;
        } else {
            stuck = 0;
        }
        if stuck >= STUCK_FRAMES {
            over.stopped = true;
            break;
        }
    }
    over.feet = body.feet();
    over.grounded = body.grounded();
    over
}

/// **The client's own surface, read column by column** — what the world a body walks *is*, so a run can say
/// whether that world holds a step at all rather than leaving a reader to assume one. A flat world is a fact
/// about this pass's subject, and it is the reason a one-metre step is climbed in the body's own guards beside
/// [`crate::body`] rather than on the drawn world: the drawn world has none.
#[derive(Clone, Copy, Debug, PartialEq)]
struct SurfaceReading {
    /// How many plan columns the world's own patch holds.
    columns: usize,
    /// How many of them a body finds ground in at all.
    with_ground: usize,
    /// The lowest and highest floor the world's own columns have, in metres.
    lowest_m: f64,
    highest_m: f64,
    /// **How many plan neighbours differ in floor height at all** — the world's own count of steps, of any
    /// size. Zero is a flat plain, and it is the count this check reads instead of assuming relief.
    neighbour_steps: usize,
    /// The smallest and largest difference between two plan neighbours' floors, in metres.
    smallest_step_m: f64,
    largest_step_m: f64,
    /// How many columns stand water.
    with_water: usize,
}

/// **The world's own surface, measured** — every plan column's floor ([`body::floor_under`]) and every plan
/// neighbour pair's difference, so *this world holds no step* is a count rather than a claim, and a world that
/// gained relief would move it.
fn read_the_surface(world: &CubicWorld) -> SurfaceReading {
    let cells = body::Cells::over(world);
    let top = f64::from(world.span_z().1);
    let patch = world.patch();
    let origin = patch.origin();
    let side = patch.side() as i32;
    let mut reading = SurfaceReading {
        columns: 0,
        with_ground: 0,
        lowest_m: f64::INFINITY,
        highest_m: f64::NEG_INFINITY,
        neighbour_steps: 0,
        smallest_step_m: f64::INFINITY,
        largest_step_m: 0.0,
        with_water: 0,
    };
    for cell_x in origin.x..origin.x + side {
        for cell_y in origin.y..origin.y + side {
            reading.columns += 1;
            let (x, y) = (f64::from(cell_x) + 0.5, f64::from(cell_y) + 0.5);
            let Some(floor) = body::floor_under(&cells, [x, y], top) else {
                continue;
            };
            reading.with_ground += 1;
            reading.lowest_m = reading.lowest_m.min(floor);
            reading.highest_m = reading.highest_m.max(floor);
            // The water **above** the ground, read through the world's own column: a cell at the floor's own
            // height is the ground itself, so asking the cell under the surface would answer zero for a world
            // standing water on every one of its columns.
            if world
                .column(VoxelPos::new(cell_x, cell_y, 0))
                .is_ok_and(|column| column.water_um > 0)
            {
                reading.with_water += 1;
            }
            // The plan neighbours, east and north — each pair once, so a step is not counted twice — and only
            // where the neighbour is a column this world owns: past the world's own edge there is no floor to
            // compare against, and `Cells` answers solid at every height there, which would read as an
            // enormous step at the world's own rim rather than as the rim it is.
            for (dx, dy) in [(1_i32, 0_i32), (0, 1)] {
                let (cell_x_next, cell_y_next) = (cell_x + dx, cell_y + dy);
                if !(origin.x..origin.x + side).contains(&cell_x_next)
                    || !(origin.y..origin.y + side).contains(&cell_y_next)
                {
                    continue;
                }
                let (nx, ny) = (f64::from(cell_x_next) + 0.5, f64::from(cell_y_next) + 0.5);
                let Some(next) = body::floor_under(&cells, [nx, ny], top) else {
                    continue;
                };
                let difference = (next - floor).abs();
                if difference > 0.0 {
                    reading.neighbour_steps += 1;
                    reading.smallest_step_m = reading.smallest_step_m.min(difference);
                    reading.largest_step_m = reading.largest_step_m.max(difference);
                }
            }
        }
    }
    if reading.with_ground == 0 {
        reading.lowest_m = 0.0;
    }
    if reading.neighbour_steps == 0 {
        reading.smallest_step_m = 0.0;
    }
    reading
}

/// **A body dropped in the air over its own column, brought down by gravity and stopped by the floor** — the
/// fall the walking law has to have, measured on the world the client draws rather than on a fixture. Returns
/// how far it fell, how many frames it was in the air, and how fast it was going when the floor stopped it.
fn fall_from_the_air(world: &CubicWorld, from: [f64; 3], above_m: f64) -> Option<(f64, usize, f64)> {
    let cells = body::Cells::over(world);
    let dt = surface::frame_seconds();
    let floor = surface::ground_under(world, from[0], from[1]).ok()?;
    let mut body = Body::standing([from[0], from[1], floor + above_m]);
    let mut fell = 0.0;
    for frame in 0..AIRBORNE_FRAMES * 4 {
        let step = body.step(dt, &cells, &Move::STILL);
        fell += step.fell_m;
        if step.grounded {
            let speed = body.velocity()[2].abs();
            return Some((fell, frame + 1, speed));
        }
    }
    None
}

/// **A held jump over the world's own floor** — how high the body's own jump reaches above the floor it stands
/// on, and whether gravity brings it back down to that same floor. Returns the height above the floor and
/// whether it landed, or nothing when the jump was never taken.
fn jump_over_the_floor(world: &CubicWorld, from: [f64; 3]) -> Option<(f64, bool)> {
    let cells = body::Cells::over(world);
    let dt = surface::frame_seconds();
    let floor = surface::ground_under(world, from[0], from[1]).ok()?;
    let mut body = Body::standing([from[0], from[1], floor]);
    if !body.step(dt, &cells, &Move { along: [0.0; 3], jump: true }).jumped {
        return None;
    }
    let mut highest = body.feet()[2];
    let mut landed = false;
    for _ in 0..AIRBORNE_FRAMES {
        let step = body.step(dt, &cells, &Move::STILL);
        highest = highest.max(body.feet()[2]);
        if step.grounded {
            landed = true;
            break;
        }
    }
    Some((highest - floor, landed))
}
/// **A walk taken without a picture** — the body's own law over the world's own cells, stepped directly, so a
/// caller that must know *which way this body can walk* pays nothing for frames it will not draw. Returns the
/// plan distance the feet travelled and whether the world refused a step.
fn dry_walk(world: &CubicWorld, from: [f64; 3], along: [f64; 2], frames: usize) -> (f64, bool) {
    let cells = body::Cells::over(world);
    let dt = surface::frame_seconds();
    let request = Move { along: [along[0], along[1], 0.0], jump: false };
    let mut body = Body::standing(from);
    let mut plan_m = 0.0;
    let mut blocked = false;
    for _ in 0..frames {
        let step = body.step(dt, &cells, &request);
        plan_m += step.plan_m;
        blocked |= step.blocked;
    }
    (plan_m, blocked)
}

/// **A wish's own opposite** — every held intent negated, so a walk and its own way back are the same walk in
/// the two directions rather than two different ones.
fn reversed(wish: &Wish) -> Wish {
    Wish {
        forward: -wish.forward,
        strafe: -wish.strafe,
        rise: -wish.rise,
        turn: -wish.turn,
        tilt: -wish.tilt,
        jump: wish.jump,
    }
}

/// **Whether what refused a walk is the world's own extent** — the body's own reach in plan, a step ahead of
/// where it stands: the cells its own box would occupy are read against the world's own plan, and a refusal
/// that came from a place this world does not own is the rim rather than its own relief. (A refusal a body's
/// own *feet* are still well inside the world for is still the rim: the disc is what hits it first.)
fn refused_at_the_worlds_own_extent(world: &CubicWorld, feet: [f64; 3], plan: [f64; 2]) -> bool {
    let reach = body::BODY_RADIUS_M + body::WALK_M_PER_S * surface::frame_seconds();
    let (x, y) = (feet[0] + plan[0] * reach, feet[1] + plan[1] * reach);
    let low_x = (x - body::BODY_RADIUS_M).floor() as i32;
    let high_x = (x + body::BODY_RADIUS_M).floor() as i32;
    let low_y = (y - body::BODY_RADIUS_M).floor() as i32;
    let high_y = (y + body::BODY_RADIUS_M).floor() as i32;
    let patch = world.patch();
    let origin = patch.origin();
    let side = patch.side() as i32;
    let owned = |value: i32, from: i32| (from..from + side).contains(&value);
    (low_x..=high_x).any(|cell_x| {
        (low_y..=high_y).any(|cell_y| !owned(cell_x, origin.x) || !owned(cell_y, origin.y))
    })
}

/// Whether a plan position is inside the world's own patch — the difference between *the world refused the
/// walk where a body stands* and *the walk reached the edge of the world it is drawn from*, which the run
/// prints separately.
fn in_the_worlds_own_plan(world: &CubicWorld, x: f64, y: f64) -> bool {
    let patch = world.patch();
    let origin = patch.origin();
    let side = patch.side() as i32;
    let (xi, yi) = (x.floor() as i32, y.floor() as i32);
    (origin.x..origin.x + side).contains(&xi) && (origin.y..origin.y + side).contains(&yi)
}

/// **The body, on the real surface** — the bars a body must meet on the client's own world rather than on a
/// fixture: **terrain stops the walk**, **gravity brings a walk down onto the floor**, and **a step is climbed**
/// (by the jump the parent's own record says a real ledge needs), with the walk's own climbed distance printed
/// beside it as the zero it is. Returns whether every bar holds.
fn check_body_over_the_world(world: &CubicWorld, home: [f64; 3]) -> bool {
    println!(
        "U1 body: the person's own feet stand at ({:.2}, {:.2}, {:.2}) m — the world's own floor under their own \
         plan column ({:.1}, {:.1}) m is {} — and the body is **walking**: {}",
        home[0],
        home[1],
        home[2],
        home[0],
        home[1],
        match surface::ground_under(world, home[0], home[1]) {
            Ok(floor) => format!("{floor:.2} m"),
            Err(refusal) => format!("refused by name ({})", refusal.kind().name()),
        },
        Body::standing(home).described(),
    );
    let mut walks = Vec::with_capacity(PLAN_DIRECTIONS.len());
    for (direction, plan) in PLAN_DIRECTIONS {
        let over = walk_over(world, direction, plan, home);
        let ended = if in_the_worlds_own_plan(world, over.feet[0], over.feet[1]) {
            "inside the world's own plan"
        } else {
            "past the world's own edge"
        };
        let at_the_edge = refused_at_the_worlds_own_extent(world, over.feet, over.plan);
        println!(
            "U1 body {direction}: {} frame(s) at {:.3} m a frame carried the feet {:.2} m in plan, walked up \
             {:.2} m and fell {:.2} m; refused on {} frame(s); stopped = {} (at the world's own edge = \
             {at_the_edge}); it ends at ({:.2}, {:.2}, {:.2}) m, {ended}, {}",
            over.frames,
            body::WALK_M_PER_S * surface::frame_seconds(),
            over.plan_m,
            over.walked_up_m,
            over.fell_m,
            over.blocked_frames,
            over.stopped,
            over.feet[0],
            over.feet[1],
            over.feet[2],
            if over.grounded { "standing on it" } else { "in the air" },
        );
        walks.push(over);
    }
    let walked_up_m: f64 = walks.iter().map(|walk| walk.walked_up_m).sum();
    let stopped = walks.iter().filter(|walk| walk.stopped).count();
    // **Where a stopped walk was stopped.** The feet stay inside the world's own plan either way; what tells
    // the two apart is the column the body was walking *into*, half a metre ahead of it — where `Cells` answers
    // solid at every height, which is this world's own edge rather than its relief.
    let stopped_at_the_edge = walks
        .iter()
        .filter(|walk| walk.stopped && refused_at_the_worlds_own_extent(world, walk.feet, walk.plan))
        .count();
    // **The step-up measured, not quoted.** The retry is armed at `STEP_UP = 0.55` m, which cannot clear a
    // whole one-metre cell — but a cell's own *corner* is enough for it to fire, and this world's own relief is
    // where it does. So the bar is not *the sum is zero*: it is *the retry fires* (the sum over every direction
    // is above zero) **and** *no frame raises the body by more than one retry per plan axis*, which is what
    // keeps a climb a climb rather than a lift.
    let step_up_fires_and_is_bounded = walked_up_m > 0.0
        && walks
            .iter()
            .all(|walk| walk.walked_up_m <= 2.0 * body::STEP_UP_M * walk.frames as f64 + 1e-9);

    // **What the world's own surface is, read column by column.** A body's own world decides what can be
    // demonstrated on it, and this is the number that decides: *how many of this world's plan neighbours differ
    // in floor height at all*. Zero would be a flat plain, and a flat plain holds no step for a walk to climb —
    // a fact this check would then report rather than a climb it could measure. Read honestly (each column's
    // own floor, each neighbour pair's own difference) this world is **not** flat: 1.00 m of step over 685
    // neighbour pairs, which is what makes the step-up's own firing below a measurement of relief.
    let surface_reading = read_the_surface(world);
    println!(
        "U1 body the world: the client's own surface, read over all {} of its own plan column(s): {} of them \
         hold ground, from {:.2} m to {:.2} m — {} of its plan neighbour pair(s) differ in floor height at all \
         (smallest {:.2} m, largest {:.2} m) — and {} column(s) stand water; **a step of any size is {} in this \
         world**",
        surface_reading.columns,
        surface_reading.with_ground,
        surface_reading.lowest_m,
        surface_reading.highest_m,
        surface_reading.neighbour_steps,
        surface_reading.smallest_step_m,
        surface_reading.largest_step_m,
        surface_reading.with_water,
        if surface_reading.neighbour_steps == 0 {
            "nowhere"
        } else {
            "there"
        },
    );
    let holds_a_step = surface_reading.neighbour_steps > 0;

    // **The fall.** A body placed above its own column, held still, brought down by the world's own gravity and
    // stopped by the world's own floor — the second sentence of this pass's brief, on the real surface.
    let fall = fall_from_the_air(world, home, FALL_FROM_M);
    println!(
        "U1 body the fall: {}",
        match fall {
            Some((fell, frames, speed)) => format!(
                "a body stood {FALL_FROM_M:.2} m above the world's own floor fell **{fell:.2} m** over \
                 {frames} frame(s) of the client's own clock and was stopped by the terrain at {:.2} m, still \
                 moving {speed:.2} m/s when the floor took it — gravity is the parent's own -32.0 m/s² and the \
                 floor is the world's own",
                surface_reading.lowest_m
            ),
            None => "no fall was measured: the body never left the air over the world's own floor".to_string(),
        }
    );
    let fell_and_landed = fall.is_some();

    // **The jump**, which is what this world leaves for a ledge: one metre of voxels is above the step-up's
    // 0.55 m and below the jump's own apex. The parent's record says it in as many words (*every real ledge is
    // a jump*, `docs/LEDGER.md` §95.5) and this is that sentence measured over this world's own floor.
    let jump = jump_over_the_floor(world, home);
    println!(
        "U1 body the jump: {}",
        match jump {
            Some((high, landed)) => format!(
                "the body's held jump left the world's own floor, reached **{high:.4} m** above it and came back \
                 down = {landed}; the continuous apex of `JUMP_M_PER_S = 8.4` in `GRAVITY_M_PER_S2 = -32.0` is \
                 {:.4} m and one frame of this client's own clock carries the body past it, which is the \
                 difference between the arithmetic and the step — so a one-metre rise is a thing this body can \
                 take, **by jumping and not by walking**",
                body::JUMP_APEX_M
            ),
            None => "the jump was never taken: the body was not standing on the world's own floor".to_string(),
        }
    );
    let jumped_and_returned = jump.is_some_and(|(_, landed)| landed);

    println!(
        "U1 body the walk on the real surface: {stopped} of {} direction(s) were refused until the body was \
         stopped ({} of them at **the world's own edge**, where a place outside the world's own plan is solid \
         at every height); the fall ended on the floor ({fell_and_landed}), the jump left the floor and came back \
         ({jumped_and_returned}), and the **walk's own step-up** raised the body a total of {walked_up_m:.2} m \
         over every direction with no frame raising it more than one retry a plan axis — the retry is armed at \
         `STEP_UP = 0.55` m and *does* fire on this world's own relief, which is the parent's own finding \
         (`docs/LEDGER.md` §95.5: a whole metre cannot be stepped up, a cell's own corner can) measured on this \
         world's own cells rather than quoted",
        walks.len(),
        stopped_at_the_edge,
    );
    if !(stopped > 0
        && fell_and_landed
        && jumped_and_returned
        && holds_a_step
        && step_up_fires_and_is_bounded)
    {
        eprintln!(
            "U1 body: the body's own walk is not measured on the real world — walks stopped {stopped}, at the \
             world's own edge {stopped_at_the_edge}, a fall ended on the floor {fell_and_landed}, a jump left \
             and returned {jumped_and_returned}, the world holds a step of its own {holds_a_step}, the step-up \
             raised the body {walked_up_m:.2} m within one retry a plan axis a frame \
             {step_up_fires_and_is_bounded}"
        );
    }
    stopped > 0 && fell_and_landed && jumped_and_returned && holds_a_step && step_up_fires_and_is_bounded
}

/// **U1's controller, measured on the run's own lines** — what a person now gets that they did not before.
///
/// It opens a [`Driven`] client (the client's own world, the client's own residency, the client's own declared
/// view), draws the frame from where that view stands, walks [`WALK_STEPS`] step(s) west and back with the
/// world's law **held**, and prints: the eye before and after, the chunk it stands over before and after,
/// what the residency held and what it had to do at every step, the frames' own digests and pixel counts as
/// they change, and — the round trip — the frame coming back **byte for byte** to the one the declared view
/// draws (`1333` rectangle(s), `147808` of `331776` pixel(s), digest `7816bf6a…`). The bar is the round trip
/// and the anchor: a walk that left the picture where it was, or a camera whose anchor was not the chunk the
/// person stands in, is a controller that is not driven.
pub fn check_u1_controller() -> bool {
    let mut client = match Driven::open() {
        Ok(client) => client,
        Err(refusal) => {
            eprintln!("U1 controller: the client refused by name: {refusal}");
            return false;
        }
    };
    let home_frame = match client.frame() {
        Ok(frame) => frame,
        Err(refusal) => {
            eprintln!("U1 controller: the declared view's own frame refused by name: {refusal}");
            return false;
        }
    };
    let home_viewpoint = client.viewpoint();
    let home_anchor = client.anchor();
    let region = client.region();
    let world_chunks = client.plane().world().chunks();
    println!(
        "U1 controller: the person the client opens as stands at ({:.2}, {:.2}, {:.2}) m, yaw {:.1}°, \
         pitch {:.1}° — its feet on the world's own floor in the plan column ({}, {}), {} the declared view's \
         own column ({}, {}) — and the frame that eye draws is {} rectangle(s), {} of {} pixel(s), digest {}, \
         mesh state {} (the residency's own identity = {})",
        home_viewpoint.eye()[0],
        home_viewpoint.eye()[1],
        home_viewpoint.eye()[2],
        home_viewpoint.yaw_deg(),
        home_viewpoint.pitch_deg(),
        home_viewpoint.feet()[0].floor() as i32,
        home_viewpoint.feet()[1].floor() as i32,
        if (
            home_viewpoint.feet()[0].floor() as i32,
            home_viewpoint.feet()[1].floor() as i32,
        ) == client.lens_column()
        {
            "which is the very column of"
        } else {
            "the column the search for a place to stand chose beside"
        },
        client.lens_column().0,
        client.lens_column().1,
        home_frame.quads(),
        home_frame.shaded(),
        home_frame.pixels_count(),
        home_frame.digest().to_hex(),
        home_frame.mesh_state().to_hex(),
        home_frame.mesh_state() == client.residency().digest(),
    );

    // **Which way the session's own walk goes, measured before a frame is drawn.** The four plan bearings the
    // eye's own basis can ask for are walked dry over the world's own cells (the body's own law, no mesh, no
    // residency, no picture): the one the body walks furthest in becomes the walk below, and all four are
    // printed. A client whose walk went the way the terrain refuses would look like a client that cannot move,
    // and the direction a person walks is a property of the ground rather than of the client.
    let bearings = [
        ("forward", Wish { forward: 1.0, ..Wish::STILL }),
        ("back", Wish { forward: -1.0, ..Wish::STILL }),
        ("right", Wish { strafe: 1.0, ..Wish::STILL }),
        ("left", Wish { strafe: -1.0, ..Wish::STILL }),
    ];
    let mut dry = Vec::with_capacity(bearings.len());
    for (name, wish) in bearings {
        let Some(along) = home_viewpoint.plan_of(&wish) else {
            continue;
        };
        let walked = dry_walk(client.plane().world(), home_viewpoint.feet(), along, WALK_STEPS);
        dry.push((name, wish, along, walked));
    }
    let allowed = body::WALK_M_PER_S * surface::frame_seconds() * WALK_STEPS as f64;
    for (name, _, plan, (plan_m, blocked)) in &dry {
        println!(
            "U1 controller the bearing {name}: {WALK_STEPS} step(s) along ({:.2}, {:.2}) would carry the body \
             {plan_m:.2} m of the {allowed:.2} m its own speed allows{}",
            plan[0],
            plan[1],
            if *blocked {
                ", with the terrain refusing a step"
            } else {
                ""
            },
        );
    }
    let Some((bearing, walk, _bearing_plan, (dry_m, _))) = dry
        .iter()
        .max_by(|a, b| a.3.0.total_cmp(&b.3.0))
        .copied()
    else {
        eprintln!(
            "U1 controller: this eye's own basis asks for no plan direction at all, so there is no bearing to \
             walk and no walk to measure"
        );
        return false;
    };
    let _ = dry_m;
    println!(
        "U1 controller: the walk below goes **{bearing}** — the bearing the body walks furthest in of the four \
         its own basis can ask for, chosen from the ground rather than declared"
    );

    // **The walk.** Eight steps along the bearing the terrain admits, every one of them drawn: the eye moves,
    // the person's own anchor moves with it, and the picture changes with the eye.
    let west = walk;
    let mut walked = Vec::with_capacity(WALK_STEPS);
    for _ in 0..WALK_STEPS {
        match client.step(&west) {
            Ok(step) => walked.push(step),
            Err(refusal) => {
                eprintln!("U1 controller: a step west refused by name: {refusal}");
                return false;
            }
        }
    }
    let away = match walked.last() {
        Some(step) => step,
        None => {
            eprintln!("U1 controller: the walk took no step at all, which is itself a defect");
            return false;
        }
    };
    let away_viewpoint = client.viewpoint();
    let digests: Vec<String> = walked
        .iter()
        .map(|step| step.drawn.digest().to_hex()[..8].to_string())
        .collect();
    let frames_changed = walked
        .iter()
        .filter(|step| step.drawn.digest() != home_frame.digest())
        .count();
    let moved = walked.iter().map(|step| step.movement.moved_m).sum::<f64>();
    let derived: u64 = walked.iter().map(|step| step.stream.derived as u64).sum();
    let evicted: u64 = walked.iter().map(|step| step.stream.evicted as u64).sum();
    let held_whole = walked
        .iter()
        .all(|step| step.stream.resident == world_chunks);
    // **The residency was observed where the step says the eye is**, step by step: the streamer's own answer
    // and the step's own anchor are the same key, which is what makes *the person's own chunk* the anchor
    // rather than a number the caller passed in.
    let anchor_is_the_eyes = walked
        .iter()
        .all(|step| step.stream.anchor == step.anchor);
    println!(
        "U1 controller the walk: {WALK_STEPS} step(s) {bearing} at {:.2} m a step take the eye {:.2} m — from \
         {} to {} — \
         and the person's own chunk with it, from ({}, {}, {}) to ({}, {}, {}); the residency was observed at \
         **the person's own anchor** and held {} of this world's {} chunk(s) at every step (derived {}, \
         evicted {}: the declared horizon is {:.0} m and this world's own diagonal is only {:.4} m, so no \
         camera anywhere in it can make a chunk leave the horizon — and a residency holding a part of the \
         region is refused by the region's own name, so the drawn path's floor is the region itself); the \
         region the frame is a picture of stayed ({}, {}, {}) because this world holds {} chunk(s) a side and \
         the region is {}, so the world's own edge is what holds it; the frames' own digests over the walk: \
         {} ({})",
        body::WALK_M_PER_S * surface::frame_seconds(),
        moved,
        home_viewpoint.described(),
        away_viewpoint.described(),
        home_anchor.x,
        home_anchor.y,
        home_anchor.z,
        client.anchor().x,
        client.anchor().y,
        client.anchor().z,
        away.stream.resident,
        world_chunks,
        derived,
        evicted,
        surface::DRAWN_RESIDENCY_M,
        region_diagonal_m(),
        region.x,
        region.y,
        region.z,
        surface::CLIENT_CHUNKS,
        surface::DRAWN_CHUNKS,
        digests.join(" "),
        frames_changed == WALK_STEPS,
    );

    // **The round trip.** The same number of steps back east, drawn: the eye returns to where it started and
    // the frame must be the declared view's own frame again — bit for bit, not merely similar.
    // **The way back is the same hold, reversed** — and named, so the line reads as the walk a person took
    // rather than as a direction this run preferred.
    let back_bearing = match bearing {
        "forward" => "back",
        "back" => "forward",
        "right" => "left",
        _ => "right",
    };
    let east = reversed(&west);
    let mut returned = None;
    for _ in 0..WALK_STEPS {
        match client.step(&east) {
            Ok(step) => returned = Some(step),
            Err(refusal) => {
                eprintln!("U1 controller: a step east refused by name: {refusal}");
                return false;
            }
        }
    }
    let back = match returned {
        Some(step) => step,
        None => {
            eprintln!("U1 controller: the walk back took no step at all, which is itself a defect");
            return false;
        }
    };
    let back_viewpoint = client.viewpoint();
    let back_frame = &back.drawn;
    let eye_returned = back_viewpoint.eye() == home_viewpoint.eye();
    let look_returned = back_viewpoint.look() == home_viewpoint.look();
    let anchor_returned = client.anchor() == home_anchor;
    let frame_returned = back_frame.digest() == home_frame.digest()
        && back_frame.quads() == home_frame.quads()
        && back_frame.shaded() == home_frame.shaded();
    println!(
        "U1 controller the round trip: {WALK_STEPS} step(s) {back_bearing} put the eye back at {} — the same eye \
         bit for bit = {}, \
         the same look bit for bit = {}, over the same chunk ({}, {}, {}) = {} — and the frame it draws is {} \
         rectangle(s), {} of {} pixel(s), digest {}: the declared view's own frame again = {}",
        back_viewpoint.described(),
        eye_returned,
        look_returned,
        home_anchor.x,
        home_anchor.y,
        home_anchor.z,
        anchor_returned,
        back_frame.quads(),
        back_frame.shaded(),
        back_frame.pixels_count(),
        back_frame.digest().to_hex(),
        frame_returned,
    );
    // **The body, on the real surface** (2026-10-14g) — the law run against the client's own cells over eight
    // walks from where this person stands: terrain that stops the walk, a fall gravity ends on the floor, and a
    // step climbed by the jump a one-metre world leaves (with the walk's own step-up printed as the zero it is).
    let body_walks = check_body_over_the_world(client.plane().world(), home_viewpoint.feet());
    println!(
        "U1 controller the session's own totals: {} frame(s) drawn, {} rectangle(s), {} shaded; the person's \
         own input: {} step(s), {:.2} m moved; the residency: derived {}, evicted {} over {} step(s)",
        client.counts().frames,
        client.counts().quads,
        client.counts().shaded,
        client.steps(),
        moved * 2.0,
        client.derived(),
        client.evicted(),
        client.steps(),
    );

    // **The bar.** A controller that moved the picture and brought it home byte for byte, whose anchor is the
    // chunk the person's own eye stands in, and whose frames are the drawn reading's own (every bar holds) —
    // that is a camera a person can drive, and anything less is not.
    let controller_drives = frames_changed == WALK_STEPS
        && eye_returned
        && look_returned
        && anchor_returned
        && frame_returned
        && held_whole
        && anchor_is_the_eyes
        && body_walks;
    if !controller_drives {
        eprintln!(
            "U1 controller: the controller is not driven — frames changed {frames_changed} of {WALK_STEPS}, \
             eye returned {eye_returned}, look returned {look_returned}, anchor returned {anchor_returned}, \
             frame returned {frame_returned}, the world held whole {held_whole}, every step observed at the \
             eye's own chunk {anchor_is_the_eyes}, the body walks the real world {body_walks}"
        );
    }
    controller_drives
}

/// **The world's own reach**, in metres: the distance between the centres of two chunks that share only a
/// corner — one chunk across and one along, which is the furthest pair this world holds. Printing it beside
/// the declared horizon is what lets a reader see that the horizon has that much slack in it rather than
/// taking *the residency never evicts* on trust.
fn region_diagonal_m() -> f64 {
    let side = f64::from(CHUNK_SIDE);
    (side * side + side * side).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::ResidencyHorizon;
    use crate::refusal::RefusalKind;

    /// **The person's own body is written in the world's own frame** — the crossing a flat world cannot fake.
    ///
    /// The world addresses cells by `(x, y, z)` with `(x, y)` the plan and `z` the span, and a chunk key's
    /// own third number counts that span; so a body's height is `z`, its plan is `(x, y)`, and its eye differs
    /// from its feet on the span and on nothing else. A body written in a rotated frame — its height on the
    /// plan's own axis, its plan coordinates on the span's — still walks a flat plain, still falls under
    /// gravity, still draws a frame, because a flat plain's floor is the same everywhere; what it cannot do is
    /// *name where it is*. So this asserts the naming: the eye is over the feet's own column and above them on
    /// the span, the world's own floor under that same column is where the feet stand, and the chunk the
    /// person's anchor names is the chunk over the column they stand in.
    #[test]
    fn the_person_the_client_opens_as_stands_in_the_column_their_own_anchor_names() {
        let client = Driven::open().expect("a driven client");
        let world = client.plane().world();
        let feet = client.viewpoint().feet();
        let eye = client.viewpoint().eye();
        assert_eq!(
            [eye[0], eye[1]],
            [feet[0], feet[1]],
            "the eye is not over the feet's own plan column: the body's height is not on the span"
        );
        assert_eq!(eye[2], feet[2] + body::EYE_HEIGHT_M, "the eye is not above the feet");
        assert_eq!(
            surface::ground_under(world, feet[0], feet[1]).expect("the world's own floor"),
            feet[2],
            "the feet are not standing on the floor of the column they are in"
        );
        assert!(client.viewpoint().grounded(), "and the body does not know it is grounded");
        // And the anchor is the key over that same column, read through the world's own key law: the chunk a
        // camera stands in is the chunk over its **plan** position, so a body whose plan was not `(x, y)` would
        // name a chunk its feet are not in. (The column itself is the search's own choice rather than the
        // declared view's: this world's relief means the lens's own column is not one whose floor is level out
        // to the body's own reach, so `person_of` takes the nearest that is — which is what the run prints
        // beside both columns.)
        let side = f64::from(crate::coordinates::CHUNK_SIDE);
        assert_eq!(
            client.anchor(),
            crate::viewpoint::chunk_over_plan(
                world,
                (feet[0] / side).floor() as i32,
                (feet[1] / side).floor() as i32,
            )
            .expect("the chunk over the person's own plan column"),
            "the anchor is not the chunk over the column the person's own feet stand in"
        );
    }

    /// **A person's own step moves the eye and the picture, and the person's own chunk is the anchor the
    /// residency is observed at** — the client's own doors, a short walk, and the two facts a controller is:
    /// the eye moved, and the chunk the frame was streamed around is the one the eye stands in.
    #[test]
    fn a_step_moves_the_eye_the_frame_and_the_anchor_a_person_stands_at() {
        let mut client = Driven::open().expect("a driven client");
        let home = client.frame().expect("the declared frame");
        let anchor = client.anchor();
        let step = client
            .step(&Wish { strafe: -1.0, ..Wish::STILL })
            .expect("a step west");
        assert!(step.movement.moved_m > 0.0, "the eye did not move");
        assert_eq!(step.stream.anchor, step.anchor, "the residency's anchor is the step's own");
        assert_eq!(
            client.viewpoint().chunk(client.plane().world()).expect("a chunk"),
            step.anchor,
            "the anchor is not the chunk the person's own eye stands in"
        );
        assert_ne!(step.drawn.digest(), home.digest(), "the picture did not follow the eye");
        // **The anchor moves when the body crosses a chunk's own plan boundary** — and how many steps that
        // takes is the ground's answer rather than this test's: the person stands half a metre west of the
        // world's own `x = 32.0` lattice line, so walking east crosses it, and walking west does not.
        let east = Wish { strafe: 1.0, ..Wish::STILL };
        let mut crossed = step.anchor != anchor;
        for _ in 0..WALK_STEPS {
            let step = client.step(&east).expect("a step east");
            if step.anchor != anchor {
                crossed = true;
                break;
            }
        }
        assert!(
            crossed,
            "{WALK_STEPS} step(s) east did not cross the chunk boundary the person stands beside"
        );
        assert_eq!(
            client.residency().len(),
            client.plane().world().chunks(),
            "the declared horizon holds this world whole"
        );
    }

    /// **A world wider than the horizon would evict as the anchor moved, and this one cannot** — the owed item
    /// named in this module's own header, held as a measurement rather than a sentence: at the world's own
    /// scale (45 m, one metre inside the world's own `45.2548` m diagonal) the *same* anchor the person's own
    /// step reports releases a chunk, derives it back, and serves the rest.
    #[test]
    fn the_anchor_a_person_stands_at_is_the_key_a_tighter_horizon_evicts_on() {
        let mut client = Driven::open().expect("a driven client");
        let world = client.plane().world().clone();
        let anchor = client.anchor();
        // The world's own diagonal is `45.2548` m, so 45 m is the tight horizon `U4 stream` walks at: the
        // chunk diagonally across from the camera's own is one step outside it.
        let mut tight = Streamer::over(&world, ResidencyHorizon::m(45.0).expect("a 45 m horizon"));
        let first = tight.observe(&world, anchor).expect("the first step");
        assert_eq!(
            first.resident, 3,
            "45 m must release the world's own far corner chunk: {first:?}"
        );
        let still = tight.observe(&world, anchor).expect("a still step");
        assert!(still.was_free(), "a still camera's step is free: {still:?}");
        // **The walk crosses where the ground lets it**: east, into the world's own next chunk, and the
        // residency is asked about **that** chunk once the body's own anchor has moved there.
        let east = Wish { strafe: 1.0, ..Wish::STILL };
        let mut walked = None;
        for _ in 0..WALK_STEPS {
            let step = client.step(&east).expect("a step east");
            if step.anchor != anchor {
                walked = Some(step);
                break;
            }
        }
        let step = walked.expect("no step east crossed a chunk boundary");
        let moved = tight.observe(&world, step.anchor).expect("the walked anchor");
        let back = tight.observe(&world, anchor).expect("home again");
        assert!(
            moved.evicted > 0 || moved.derived > 0 || moved.resident != first.resident,
            "walking a chunk cost the tighter residency nothing: {moved:?}"
        );
        assert!(back.derived > 0, "coming back derived nothing: {back:?}");
    }

    /// **A horizon has no metres in it** — the derivation a controller's own tighter horizon stands on is the
    /// streamer's, so a horizon of no metres is refused **by the streamer**, proved here rather than assumed.
    #[test]
    fn a_horizon_of_no_metres_is_not_a_horizon() {
        assert_eq!(
            ResidencyHorizon::m(0.0).err().map(|refusal| refusal.kind()),
            Some(RefusalKind::ParameterOutOfRange),
            "a horizon of no metres is not one"
        );
    }
}
