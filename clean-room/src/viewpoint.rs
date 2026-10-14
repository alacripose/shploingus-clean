//! **U1 — the client's own viewpoint: the camera a person drives.**
//!
//! # One owner, one vocabulary, and the drawn camera derived from it
//!
//! This module owns exactly two things: **where the eye stands and which way it looks** ([`Viewpoint`]) and
//! **what a person is asking for** ([`Wish`]).
//!
//! **The eye is derived from a body (2026-10-14g).** A [`Viewpoint`] holds a [`Body`] — the feet, the
//! velocity and how it traverses ([`crate::body`]) — and the look, and [`Viewpoint::eye`] is the body's own
//! eye ([`Body::eye`]): the feet plus [`crate::body::EYE_HEIGHT_M`]. A stored eye beside a stored body would
//! be two owners of one fact, which is the seam the parent keeps for the same reason (*"the camera's
//! `position_m` is derived from this every frame rather than moved independently, so the eye can never be
//! somewhere the body is not"*). Walking is the traversal a body is by default and flight is an option a
//! person takes, and the reading's own observing eye is a **flying** body at the place its camera has always
//! stood ([`Body::hovering`]) — that eye is an instrument's vantage, not a person's feet. Nothing here knows a key or a word — the window's own message
//! loop (`crate::surface::platform`) and the shell's own command line are the two translations from input to
//! a [`Wish`] — so a held key and a typed word cannot move the camera two different ways: they are one
//! intent, written by one door ([`Viewpoint::step`]).
//!
//! The drawn reading's own camera is **derived** from this state rather than kept beside it
//! ([`crate::drawn::Camera::standing`]), which is the parent's own rule for the same seam. The parent keeps
//! the body and derives the camera from it every frame — *"the camera's `position_m` is derived from this
//! every frame rather than moved independently, so the eye can never be somewhere the body is not"*
//! (`crates/voxel-client/src/main.rs`) — and it names its input as intent for the same reason this module
//! does: *"Named as intent so nothing here knows a `winit` `KeyCode`; the bin is the only translation"*
//! (`crates/voxel-client/src/motion.rs`).
//!
//! # The parent's controller is read, not depended on, and why
//!
//! The parent's own controller seam is `crates/voxel-client/src/motion.rs` (`Wish`, `Motion`, `Player`) with
//! its camera in `crates/voxel-client/src/camera.rs`, and this pass read those bytes to be the **source** of
//! this module's shape: the intent vocabulary, yaw/pitch clamping short of the pole, and one owner for the
//! eye. It is not a **path dependency**, and the blocker is the crate rather than the signature:
//! `voxel-client` is the only crate that holds that module, and it depends on `wgpu`, `glyphon` and
//! `pollster` for its renderer — so declaring it would add three third-party crates to a manifest whose own
//! rule is that it declares `sha2` alone (`Cargo.toml`), while the recorded seam requires the ported call to
//! be reachable without them. The measurement and the smallest next step that would remove the blocker are in
//! `docs/baselines/port-appearance-seam/parent-blocker-2026-10-14e.txt`; the parent's own free-flight speed
//! is the one number this module re-uses from it, and it is named where it is declared.
//!
//! # The camera's own chunk, and the region it stands at the corner of
//!
//! A person's own place decides two things, and this module is the single owner of both:
//!
//! * **the chunk the camera keeps** ([`Viewpoint::chunk`]) — the world's own key the eye stands over, which
//!   is the anchor `crate::stream::Streamer::observe` is asked about. It moves when the person crosses a
//!   chunk's own plan boundary, which is what makes a walk cost a derivation and a stop cost none.
//! * **the region the frame is a picture of** ([`Viewpoint::region_origin`]) — the plan corner of the
//!   rectangle of chunks the drawn reading stitches, one chunk back from the camera's own and clamped into
//!   the world's own chunks, so the eye stands at the region's own corner column and the region follows the
//!   person until the world's own edge stops it.
//!
//! Both are read off the world's own key law ([`CubicWorld::keys`]) rather than declared again, and the
//! world's own finite extent is what bounds them: a person who walks off the world keeps the last chunk it
//! owns rather than being answered with a region that does not exist.
//!
//! # The numbers, and where each comes from
//!
//! * [`FLY_M_PER_FRAME`] — the parent's own free-flight speed at this client's own declared frame rate:
//!   **18 m/s** (`crates/shploingus/src/main.rs`, `FLY_M_PER_S`) over **30 frame(s)/s**
//!   ([`crate::surface::FRAMES_PER_SECOND`]), because a frame here **is** a tick
//!   ([`crate::surface::DECLARED_FRAMES`]) and a person's move is one step of that clock.
//! * [`TURN_DEG_PER_FRAME`] — a declared quarter turn a second, in a frame's own units. The parent's look is
//!   a mouse device (`LOOK_RAD_PER_MOTION_UNIT`), and this client's is a key: there is no parent number to
//!   read, so this one is declared here and printed with every frame rather than implied.
//! * [`MAX_PITCH_DEG`] — the parent's own pole: `crates/voxel-client/src/camera.rs` stops pitch at
//!   `MAX_PITCH_RAD = 1.5690512`, which is 89.9°, *"a **platform** number rather than a design one"*: a
//!   look-at basis is undefined when the view runs along the world's own up.
//! * [`REGION_BASE_Z`] — the drawn region's own declared base level. A region is a **plan** rectangle of
//!   chunks: the mesh comes from the columns it covers and the span those columns imply, so the region's key
//!   carries this declared level and **not** the camera's own chunk level — the same number the drawn
//!   reading declared before a person could drive it.
//!
//! # What is owed, named here rather than implied
//!
//! * **A body.** This is a free-flying eye, which is what the parent's own client is (*"No physics. Moving is
//!   free flight, so the camera can pass through terrain"*, `crates/shploingus/src/main.rs`): no collider, no
//!   gravity, no ground. The parent's Squake body (`motion.rs`) is assigned **no row** in the clean room by
//!   the record itself (`docs/baselines/u12-first-mob/final-stdout.txt`, the `resquake` entry), so walking a
//!   person through the world's own cells is a named gap rather than a silent import.
//! * **A pointer.** Look is turned by a key here; a raw device motion is the parent's door and this client
//!   has no device event of its own.
//! * **A frame-rate-independent step.** One step is one frame, the client's own declared clock; a person on a
//!   slower machine moves a shorter distance in the same wall-clock second. The parent banks frame time
//!   (`MAX_DT`) and this client's clock is its frame, so the honest repair is a step per *tick* with a
//!   banked remainder, and it is not built here.

use crate::body::{Body, BodyStep, Move, Solidity, Traversal};
use crate::coordinates::{ChunkKey, CHUNK_SIDE};
use crate::chunk::CubicWorld;
use crate::drawn::Camera;
use crate::refusal::{Refusal, RefusalKind};

/// **How far one step of held input moves a flying eye**, in metres.
///
/// The parent's own free-flight speed — `FLY_M_PER_S: f64 = 18.0`, `crates/shploingus/src/main.rs` — over
/// this client's declared frame rate (30, [`crate::surface::FRAMES_PER_SECOND`]): **0.6 m a frame**. Since
/// 2026-10-14g this is **the flight's own** step and not the person's: a walking body travels
/// [`crate::body::WALK_M_PER_S`] a second (`0.1439` m a frame) and flight is the option a person takes, so a
/// run that prints *how far the eye moved* prints what the body did rather than this number.
pub const FLY_M_PER_FRAME: f64 = crate::body::FLY_M_PER_S / crate::surface::FRAMES_PER_SECOND as f64;

/// **How far one step of held turn input turns the eye**, in degrees.
///
/// A declared quarter turn a second at the client's own 30 frame(s)/s: 3° a frame. The parent's look is a
/// **device** (a mouse's motion, `LOOK_RAD_PER_MOTION_UNIT`) and there is no parent number for a key, so this
/// one is this client's, declared and printed.
pub const TURN_DEG_PER_FRAME: f64 = 3.0;

/// **The pole a look stops short of**, in degrees — the parent's own `MAX_PITCH_RAD = 1.5690512`
/// (`crates/voxel-client/src/camera.rs`), which is 89.9° to the tenth.
///
/// A look-at basis is undefined when the view runs along the world's own up, and inventing a roll to fix it
/// would be inventing a camera, so the pitch is **clamped** here and the clamp is the platform's number
/// rather than this module's taste.
pub const MAX_PITCH_DEG: f64 = 89.9;

/// **The drawn region's own base level.** The region a frame is a picture of is a **plan** rectangle of
/// chunks (see [`Viewpoint::region_origin`]), and the mesh comes from the columns that rectangle covers and
/// the span **those** columns imply ([`crate::mesh::Footprint`]), so the region's key carries this declared
/// level rather than the camera's own chunk level: a person who flies a hundred metres up does not name a
/// different region for answering the same columns.
pub const REGION_BASE_Z: i32 = 0;

/// **The world's own up** — the axis the cells are addressed in ([`crate::coordinates::CHUNK_SIDE`]'s third
/// axis, the span): what a person rises along and what a turn turns about.
const UP: [f64; 3] = [0.0, 0.0, 1.0];

/// **What a person is asking for this step, in intent rather than in keys.**
///
/// Each component is a *held* direction in `[-1, 1]`: `forward` along the eye's own look, `strafe` to its
/// own right, `rise` along the world's own up, `turn` about it and `tilt` across it. The three moves are
/// **normalised together**, so holding forward-and-right is not a faster diagonal — the parent's own rule
/// for the same quantity (*"which is what stops walking forward-and-right being 41 % faster than walking
/// forward"*, `motion.rs`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Wish {
    /// Along the direction the eye looks: `+1` forward, `-1` back.
    pub forward: f64,
    /// To the eye's own right: `+1` right, `-1` left.
    pub strafe: f64,
    /// Along the world's own up: `+1` up, `-1` down.
    pub rise: f64,
    /// About the world's own up: `+1` turns right, `-1` left.
    pub turn: f64,
    /// Across it: `+1` tilts up, `-1` down.
    pub tilt: f64,
    /// **Whether the walk is asked to jump** — the walking body's own vertical intent, held like the parent's
    /// own `jump`. A flying body ignores this and rises along `rise` instead, which is why the two are separate
    /// intents rather than one: a body jumps, and an eye flies.
    pub jump: bool,
}

impl Wish {
    /// A wish that asks for nothing — the eye stays where it is, and the step is still a step (the run
    /// counts it, and a still camera's step is free).
    pub const STILL: Wish = Wish {
        forward: 0.0,
        strafe: 0.0,
        rise: 0.0,
        turn: 0.0,
        tilt: 0.0,
        jump: false,
    };

    /// Whether this wish asks for nothing at all — a held jump is something, even with no direction held.
    pub fn is_still(&self) -> bool {
        self.forward == 0.0
            && self.strafe == 0.0
            && self.rise == 0.0
            && self.turn == 0.0
            && self.tilt == 0.0
            && !self.jump
    }

    /// The move as one unit-length direction in the eye's own frame: `(forward, strafe, rise)`, normalised
    /// **together** so no combination of held directions is a faster one. A wish of no length is no move.
    fn unit_move(&self) -> Option<[f64; 3]> {
        let length =
            (self.forward * self.forward + self.strafe * self.strafe + self.rise * self.rise).sqrt();
        if length <= 0.0 || !length.is_finite() {
            return None;
        }
        Some([self.forward / length, self.strafe / length, self.rise / length])
    }
}

/// **Where the eye stands and which way it looks** — the client's one camera state.
///
/// The eye is in the world's own metres; the look is a **unit vector** in the world's own coordinates,
/// because that is what the drawn camera's basis is built from ([`crate::drawn::Camera::standing`]): a
/// viewpoint that returned yaw and pitch would re-derive the direction through two transcendental functions,
/// and a client whose frame must come back **byte for byte** when the eye comes back cannot afford a
/// direction that differs in its last bits for having been turned into angles and back.
///
/// Yaw and pitch are still what a person reads and turns by — they are [`Viewpoint::yaw_deg`],
/// [`Viewpoint::pitch_deg`] and [`Viewpoint::described`], **derived** from the look rather than stored
/// beside it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewpoint {
    /// **The body** — the feet, the velocity and the traversal. The one owner of where the person is.
    body: Body,
    look: [f64; 3],
}

/// **What one step of driving did** — the distance the eye moved, the distance its look turned, the wish that
/// was read, and what the **body** did ([`BodyStep`]: plan travel, rise, fall, a step climbed, a jump, whether
/// it stands, whether it was blocked), so a run prints what a person's own input did rather than a sentence
/// about input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Step {
    pub wish: Wish,
    /// How far the eye moved, in metres.
    pub moved_m: f64,
    /// How far the look turned, in degrees.
    pub turned_deg: f64,
    /// **What the body did** — the feet's own movement, in the body's own words.
    pub body: BodyStep,
}

impl Step {
    /// Whether this step moved or turned the eye at all.
    pub fn was_still(&self) -> bool {
        self.moved_m == 0.0 && self.turned_deg == 0.0
    }
}

impl Viewpoint {
    /// **The viewpoint a camera stands at** — its own eye, and the direction of its own look (`toward` is the
    /// direction *from* the world toward the eye, so the look is its negation, exactly).
    ///
    /// This is how the client's declared instrument view becomes a viewpoint: the drawn reading's own camera
    /// is built first (`Camera::over`, the standoff eye and the look at the region's own centre) and this
    /// reads it. So *home* is the recorded view **by construction**, and a person who drives away and comes
    /// back is standing where the reading has always stood.
    pub fn of(camera: &Camera) -> Viewpoint {
        let look = camera.toward();
        Viewpoint {
            body: Body::hovering(camera.eye),
            look: [-look[0], -look[1], -look[2]],
        }
    }

    /// **A body and a look** — the one door a person's own viewpoint is built through, so a viewpoint always
    /// owns a body rather than an eye that could disagree with one. The look must be a unit direction; a look
    /// that is not one is repaired to the world's own `+y` here rather than making a basis out of nothing.
    pub fn new(body: Body, look: [f64; 3]) -> Viewpoint {
        Viewpoint { body, look: unit(look).unwrap_or(UP) }
    }

    /// The body — where the person is, and how they traverse.
    pub const fn body(&self) -> Body {
        self.body
    }

    /// The feet's own place, in the world's metres.
    pub const fn feet(&self) -> [f64; 3] {
        self.body.feet()
    }

    /// Whether the feet rest on the ground under them.
    pub const fn grounded(&self) -> bool {
        self.body.grounded()
    }

    /// How this person traverses the world.
    pub const fn traversal(&self) -> Traversal {
        self.body.traversal()
    }

    /// **Take up a traversal** — walking or flying, through the body's own door.
    pub fn traverse(&mut self, traversal: Traversal) {
        self.body.traverse(traversal);
    }

    /// **The eye's own place, in the world's metres — derived from the feet, never stored.**
    pub const fn eye(&self) -> [f64; 3] {
        self.body.eye()
    }

    /// The unit direction the eye looks along.
    pub const fn look(&self) -> [f64; 3] {
        self.look
    }

    /// **The heading**, in degrees: `0` looks along the world's own `+y`, `90` along `+x`. Derived from the
    /// look for a person to read, never stored.
    pub fn yaw_deg(&self) -> f64 {
        self.look[0].atan2(self.look[1]).to_degrees()
    }

    /// **The pitch**, in degrees above the world's own plan: what a tilt moves, clamped to [`MAX_PITCH_DEG`].
    pub fn pitch_deg(&self) -> f64 {
        self.look[2].clamp(-1.0, 1.0).asin().to_degrees()
    }

    /// **Where the eye stands and which way it looks**, for a reader of a run's own line — the two facts this
    /// type owns, and nothing else: *which chunk it stands over* is a question the world's own key law answers
    /// ([`Viewpoint::chunk`]) and a session prints beside this, so a reader sees the eye and the anchor as two
    /// answers rather than one string that happens to hold both.
    pub fn described(&self) -> String {
        format!(
            "the person's own eye ({:.2}, {:.2}, {:.2}) m over the feet at ({:.2}, {:.2}, {:.2}) m — {} and {} 
             — yaw {:.1}°, pitch {:.1}°",
            self.eye()[0],
            self.eye()[1],
            self.eye()[2],
            self.feet()[0],
            self.feet()[1],
            self.feet()[2],
            self.traversal().name(),
            if self.grounded() { "standing on the ground" } else { "in the air" },
            self.yaw_deg(),
            self.pitch_deg()
        )
    }

    /// **The plan direction a wish asks for** — the wish's own unit move projected on the eye's own basis and
    /// read in the world's own plan, as a unit direction `[x, y]`. Nothing when the wish asks for no plan
    /// motion at all (a look straight down and a held rise, say). A caller that must know which way a body will
    /// walk — the controller's own choice of a bearing the terrain admits, say — asks this rather than taking
    /// the client's own arithmetic apart and re-deriving the basis beside it.
    pub fn plan_of(&self, wish: &Wish) -> Option<[f64; 2]> {
        let unit = wish.unit_move()?;
        let (right, up) = self.basis();
        let mut along = [0.0; 3];
        for (axis, out) in along.iter_mut().enumerate() {
            *out = unit[0] * self.look[axis] + unit[1] * right[axis] + unit[2] * up[axis];
        }
        let plan = crate::body::plan_unit(along);
        let length = (plan[0] * plan[0] + plan[1] * plan[1]).sqrt();
        if length <= 0.0 || !length.is_finite() {
            return None;
        }
        Some([plan[0] / length, plan[1] / length])
    }

    /// **One step of driving** — the move along the eye's own basis, then the turn about the world's own up,
    /// then the tilt within the pole.
    ///
    /// The move is [`FLY_M_PER_FRAME`] along the wish's own unit direction, decomposed on the eye's own basis:
    /// `forward` and `strafe` along the look and its right, `rise` along the world's own up. The turn rotates
    /// the look about the world's own up — which leaves the pitch's own *sine* where it was, so a turn is a
    /// heading and nothing else — and the tilt moves the pitch alone, **stopping at the pole**
    /// ([`MAX_PITCH_DEG`]) by clamping the pitch *before* the rotation is taken rather than after it: a look
    /// that rotated first would carry the pitch past the pole, which is a **flipped heading**, not a clamped
    /// one, and a person holding "tilt up" against the pole would watch the world spin round behind them.
    /// **One step of a person's own input** — the wish is read as a unit direction on the eye's own basis,
    /// **the body takes the step** ([`Body::step`]: jump, walk, block, climb, fall, gravity — the walking law;
    /// or the free move when the body flies), and then the look turns.
    ///
    /// `dt_s` is the client's own step of time (one frame, [`crate::surface::FRAMES_PER_SECOND`]), and `solids`
    /// is what stops the body: for a person in the world that is the world's own cells
    /// ([`crate::body::Cells`]), which is what makes *the drawn world stops it* true rather than asserted.
    /// The eye itself is never moved here: it is [`Body::eye`], read after the body's own step.
    pub fn step(&mut self, wish: &Wish, dt_s: f64, solids: &impl Solidity) -> Step {
        let moved_start = self.body.eye();
        let along = match wish.unit_move() {
            Some(unit) => {
                let (right, up) = self.basis();
                let mut along = [0.0; 3];
                for (axis, out) in along.iter_mut().enumerate() {
                    *out = unit[0] * self.look[axis] + unit[1] * right[axis] + unit[2] * up[axis];
                }
                along
            }
            None => [0.0; 3],
        };
        let body = self
            .body
            .step(dt_s, solids, &Move { along, jump: wish.jump });
        let before = self.look;
        if wish.turn != 0.0 && wish.turn.is_finite() {
            self.look = rotate_about(self.look, UP, wish.turn * TURN_DEG_PER_FRAME);
        }
        if wish.tilt != 0.0 && wish.tilt.is_finite() {
            self.tilt_deg(wish.tilt * TURN_DEG_PER_FRAME);
        }
        Step {
            wish: *wish,
            moved_m: distance_between(moved_start, self.body.eye()),
            turned_deg: angle_between(before, self.look),
            body,
        }
    }

    /// **Tilt the look by `degrees`, stopping at the pole** — the pitch moves and the heading does not: the
    /// wanted pitch is `pitch + degrees` clamped to [`MAX_PITCH_DEG`], and the look is rebuilt from the plan
    /// direction it already had (scaled, never rotated), so a person at the pole stays there instead of being
    /// carried over it. A look with no plan direction at all (exactly on the world's own up) has no heading to
    /// keep and takes the world's own `+y` as one, which is the same repair [`Viewpoint::basis`] makes.
    fn tilt_deg(&mut self, degrees: f64) {
        let plan = (self.look[0] * self.look[0] + self.look[1] * self.look[1]).sqrt();
        let pitch = self.look[2].clamp(-1.0, 1.0).asin().to_degrees();
        let wanted = (pitch + degrees).clamp(-MAX_PITCH_DEG, MAX_PITCH_DEG);
        let (sin, cos) = wanted.to_radians().sin_cos();
        if plan <= 0.0 {
            self.look = [0.0, cos, sin];
            return;
        }
        let scale = cos / plan;
        self.look = [self.look[0] * scale, self.look[1] * scale, sin];
    }

    /// **The unit direction a person looking straight ahead moves along, and its right** — the eye's own
    /// basis, with the world's own up as the reference: a look that runs along the pole gets the world's own
    /// `+x` as its right rather than a `NaN` (the same repair the parent's `Camera::basis` makes with its own
    /// fallback).
    fn basis(&self) -> ([f64; 3], [f64; 3]) {
        let right = unit(cross(UP, self.look)).unwrap_or([1.0, 0.0, 0.0]);
        let up = cross(self.look, right);
        (right, up)
    }

    /// **The world's own chunk this eye stands over** — the anchor the residency is observed at.
    ///
    /// The rule, in one sentence: the chunk whose own **plan cell** holds the eye's plan position, and when
    /// the eye stands off the world's own plan (above it, or past its edge) the world's own chunk nearest it
    /// in plan, ties in key order. A world that owns no chunk at all has nowhere for a camera to stand and
    /// says so by name.
    pub fn chunk(&self, world: &CubicWorld) -> Result<ChunkKey, Refusal> {
        let (cell_x, cell_y) = chunk_of_place(self.eye()[0], self.eye()[1]);
        chunk_over_plan_beside(world, cell_x, cell_y, self.eye()[0], self.eye()[1])
    }

    /// **The drawn region's own footprint origin** — the plan corner of the `chunks × chunks` rectangle of
    /// chunks the frame is a picture of, one chunk back from the camera's own chunk in both plan axes and
    /// clamped into the world's own chunks, at the region's own declared base level ([`REGION_BASE_Z`]).
    ///
    /// One chunk back is the **declared** reading of *the camera stands at the region's own corner*: the
    /// eye's own plan position is the corner of the four chunks the region covers, which is where this
    /// client's declared instrument view already stood (`docs/baselines/port-appearance-seam/README.txt` §11:
    /// the drawn reading's region is `2 × 2` chunks and the recorded frame is that rectangle). The clamp is
    /// the world's own finiteness: past the world's edge the region stops following rather than naming a
    /// rectangle the world does not own.
    pub fn region_origin(&self, world: &CubicWorld, chunks: u32) -> Result<ChunkKey, Refusal> {
        let anchor = self.chunk(world)?;
        let keys = world.keys();
        let span = chunks.max(1) as i32 - 1;
        let min_x = keys.iter().map(|key| key.x).min().unwrap_or(anchor.x);
        let max_x = keys.iter().map(|key| key.x).max().unwrap_or(anchor.x);
        let min_y = keys.iter().map(|key| key.y).min().unwrap_or(anchor.y);
        let max_y = keys.iter().map(|key| key.y).max().unwrap_or(anchor.y);
        let x = (anchor.x - 1).clamp(min_x, (max_x - span).max(min_x));
        let y = (anchor.y - 1).clamp(min_y, (max_y - span).max(min_y));
        Ok(ChunkKey::new(x, y, REGION_BASE_Z))
    }
}

/// **The world's own chunk over one plan cell** — the one key law the camera's own chunk and the declared
/// view's own anchor are both read through, so *the chunk a camera stands in* has a single answer for a
/// person driving it and for the client's declared view.
///
/// The cell's own chunks are the answer when the world owns any of them (the nearest to the eye in plan when
/// it owns several, ties in key order); otherwise the world's own chunk nearest the plan position given, which
/// is what a camera above or past the world's own edge gets instead of a refusal — the world is finite, and
/// the honest answer to *which of your chunks am I over* is its own nearest one.
pub fn chunk_over_plan(world: &CubicWorld, cell_x: i32, cell_y: i32) -> Result<ChunkKey, Refusal> {
    let centre_x = f64::from(cell_x) * f64::from(CHUNK_SIDE) + f64::from(CHUNK_SIDE) / 2.0;
    let centre_y = f64::from(cell_y) * f64::from(CHUNK_SIDE) + f64::from(CHUNK_SIDE) / 2.0;
    chunk_over_plan_beside(world, cell_x, cell_y, centre_x, centre_y)
}

/// The same rule, with the plan position the tie-break is measured from — the eye's own place.
fn chunk_over_plan_beside(
    world: &CubicWorld,
    cell_x: i32,
    cell_y: i32,
    x: f64,
    y: f64,
) -> Result<ChunkKey, Refusal> {
    let keys = world.keys();
    if keys.is_empty() {
        return Err(Refusal::new(
            RefusalKind::ParameterOutOfRange,
            "the world this camera stands in holds no chunk: there is no camera to stand in one",
        ));
    }
    let over = keys
        .iter()
        .copied()
        .filter(|key| key.x == cell_x && key.y == cell_y)
        .min_by(|a, b| plan_to_eye(*a, x, y).total_cmp(&plan_to_eye(*b, x, y)));
    if let Some(nearest) = over {
        return Ok(nearest);
    }
    keys.iter()
        .copied()
        .min_by(|a, b| plan_to_eye(*a, x, y).total_cmp(&plan_to_eye(*b, x, y)))
        .ok_or_else(|| {
            Refusal::new(
                RefusalKind::ParameterOutOfRange,
                "the world this camera stands in holds no chunk: there is no camera to stand in one",
            )
        })
}

/// The plan chunk a position's own column falls in — the world's own chunk lattice, asked of metres.
fn chunk_of_place(x: f64, y: f64) -> (i32, i32) {
    let side = f64::from(CHUNK_SIDE);
    let cell = |value: f64| {
        if value.is_finite() {
            (value / side).floor() as i32
        } else {
            0
        }
    };
    (cell(x), cell(y))
}

/// How far a chunk's own plan cell centre stands from a plan position, in metres — the distance the nearest
/// chunk is chosen by, and the plan cell as a whole is used so a chunk is in or out as one (the same shape
/// the residency horizon's own rule takes).
fn plan_to_eye(key: ChunkKey, x: f64, y: f64) -> f64 {
    let half = f64::from(CHUNK_SIDE) / 2.0;
    let centre_x = f64::from(key.x) * f64::from(CHUNK_SIDE) + half;
    let centre_y = f64::from(key.y) * f64::from(CHUNK_SIDE) + half;
    let (dx, dy) = (x - centre_x, y - centre_y);
    dx * dx + dy * dy
}

/// A unit vector, or nothing when there is no direction to have.
fn unit(direction: [f64; 3]) -> Option<[f64; 3]> {
    let length =
        (direction[0] * direction[0] + direction[1] * direction[1] + direction[2] * direction[2]).sqrt();
    if length <= 0.0 || !length.is_finite() {
        return None;
    }
    Some([direction[0] / length, direction[1] / length, direction[2] / length])
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Rotate `direction` about `axis` (a unit vector) by `degrees`, right-handed about the axis.
fn rotate_about(direction: [f64; 3], axis: [f64; 3], degrees: f64) -> [f64; 3] {
    let Some(axis) = unit(axis) else {
        return direction;
    };
    let angle = degrees.to_radians();
    let (sin, cos) = angle.sin_cos();
    let dot = direction[0] * axis[0] + direction[1] * axis[1] + direction[2] * axis[2];
    let crossed = cross(axis, direction);
    unit([
        direction[0] * cos + crossed[0] * sin + axis[0] * dot * (1.0 - cos),
        direction[1] * cos + crossed[1] * sin + axis[1] * dot * (1.0 - cos),
        direction[2] * cos + crossed[2] * sin + axis[2] * dot * (1.0 - cos),
    ])
    .unwrap_or(direction)
}

fn distance_between(from: [f64; 3], to: [f64; 3]) -> f64 {
    let (dx, dy, dz) = (to[0] - from[0], to[1] - from[1], to[2] - from[2]);
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// The angle between two unit directions, in degrees.
fn angle_between(a: [f64; 3], b: [f64; 3]) -> f64 {
    let dot = (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]).clamp(-1.0, 1.0);
    dot.acos().to_degrees()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automaton::Automaton;
    use crate::body::Body;
    use crate::drawn::Camera;
    use crate::mesh::ResidencyHorizon;
    use crate::stream::Streamer;
    use crate::surface;

    /// **The air a hovering body flies through.** The guards below are about the **eye** — the basis, the
    /// diagonal rule, the pole, the chunk and the region — and a flying body ignores what it is stopped by, so
    /// they hand the step a fixture that blocks nothing rather than a world they are not measuring: the walking
    /// law has its own guards beside [`crate::body`], on the real world, in `controller`.
    struct Air;

    impl Solidity for Air {
        fn is_solid(&self, _at: [i32; 3]) -> bool {
            false
        }
    }

    /// One frame of the client's own clock — the step of time every step here is taken over
    /// ([`crate::surface::frame_seconds`], the one door to it).
    const DT: f64 = 1.0 / crate::surface::FRAMES_PER_SECOND as f64;

    /// **One step of a wish, in the air** — `viewpoint.step(wish)` before 2026-10-14g, named for what it is
    /// now that the step takes the time it is over and what stops it.
    fn fly(viewpoint: &mut Viewpoint, wish: &Wish) -> Step {
        viewpoint.step(wish, DT, &Air)
    }

    /// **A viewpoint hovered at a place** — the running eye the region guards need, built through the body's
    /// own door (`Body::hovering`, a flying body is not a person's feet) rather than by writing an eye that a
    /// body could disagree with.
    fn hovered(at: [f64; 3], look: [f64; 3]) -> Viewpoint {
        Viewpoint::new(Body::hovering(at), look)
    }

    /// **The client's own three quarters** — the state the law is stepping, the client's own residency of it,
    /// and the declared view read as a viewpoint: exactly what `controller::Driven::open` builds, built here
    /// through the same doors so a guard measures the client the window drives.
    fn client() -> (Automaton, Streamer, Viewpoint) {
        let plane = surface::client_state().expect("the client's shown state");
        let residency = surface::residency_of(plane.world()).expect("the declared residency");
        let mesh = surface::region_mesh(&residency, surface::DRAWN_CHUNK).expect("the region mesh");
        let viewpoint = surface::home_viewpoint(&mesh).expect("the declared view");
        (plane, residency, viewpoint)
    }

    /// The declared view's own state alone, for the guards that only need the eye and the world.
    fn world_and_home() -> (CubicWorld, Viewpoint) {
        let (plane, _, viewpoint) = client();
        (plane.world().clone(), viewpoint)
    }

    /// Steps taken to cross one chunk's own plan boundary at [`FLY_M_PER_FRAME`] a step — the walk's own
    /// length, measured from the two declared numbers rather than chosen.
    fn steps_across_a_chunk() -> usize {
        (f64::from(CHUNK_SIDE) / FLY_M_PER_FRAME).ceil() as usize
    }

    /// **One owner for the eye: the drawn camera home *is* a viewpoint, and the camera derived from that
    /// viewpoint is the camera home stood at** — bit for bit, which is what lets the recorded frame come back
    /// when a person returns rather than merely look like it did.
    #[test]
    fn the_declared_view_is_a_viewpoint_and_the_camera_derived_from_it_is_the_same_camera() {
        let (plane, residency, viewpoint) = client();
        let mesh = surface::region_mesh(&residency, surface::DRAWN_CHUNK).expect("the region mesh");
        let over = Camera::over(&mesh, surface::view().toward, crate::drawn::DRAWN_FOV_DEG)
            .expect("the declared view");
        assert_eq!(viewpoint.eye(), over.eye, "the eye is the camera's own");
        let forward = unit([
            over.target[0] - over.eye[0],
            over.target[1] - over.eye[1],
            over.target[2] - over.eye[2],
        ])
        .expect("the camera looks at something");
        assert_eq!(viewpoint.look(), over.look, "the look is the camera's own, bit for bit");
        // The eye-to-target direction *is* that look: the camera's own `over` computes it from the two places
        // and stores it, so reading it back must not lose a bit — a second computation of one quantity is a
        // different quantity, which is why this is compared against the camera's own field above.
        for (axis, expected) in forward.iter().enumerate() {
            assert!(
                (viewpoint.look()[axis] - expected).abs() < 1e-15,
                "the look is not the eye's own forward on axis {axis}"
            );
        }
        // **The instrument view stands off the declared direction and looks back down it**: the declared
        // `View.toward` is raised `VIEW_ALTITUDE_DEG` from the plan, so the look that returns along it is that
        // much short of straight down — 75° below the plan for the declared 15°.
        let pitch = viewpoint.pitch_deg();
        let below_plan = 90.0 - surface::VIEW_ALTITUDE_DEG;
        assert!(
            (pitch + below_plan).abs() < 1e-9,
            "the declared view's own pitch is {pitch}, not -{below_plan}"
        );
        // And the derivation is a round trip: the camera the person's own state stands at reads back as the
        // same viewpoint, look included.
        let standing = Camera::standing(
            &mesh,
            viewpoint.eye(),
            viewpoint.look(),
            crate::drawn::DRAWN_FOV_DEG,
        )
        .expect("the camera the person's own state stands at");
        assert_eq!(standing.eye, viewpoint.eye(), "the standing camera moved the eye");
        assert_eq!(Viewpoint::of(&standing), viewpoint, "the round trip lost a bit");
        // Where the declared view stands: exactly on the chunk lattice's own plan boundary (`x = 32.0`), which
        // is why the world's own key law and this module agree about the chunk the eye stands over.
        assert_eq!(viewpoint.eye()[0], f64::from(CHUNK_SIDE), "the declared eye's own x");
        assert_eq!(
            viewpoint.chunk(plane.world()).expect("a chunk"),
            crate::viewpoint::chunk_over_plan(plane.world(), 1, 1).expect("the declared anchor"),
            "the eye's own chunk is not the chunk the declared view stands at"
        );
    }

    /// **A held direction moves the eye exactly one step's length** — and three held directions together are
    /// the same length as one, which is the diagonal rule the parent declares for its own input.
    #[test]
    fn a_step_moves_the_eye_the_declared_distance_and_a_diagonal_is_not_faster() {
        let (_, start) = world_and_home();
        let forward = Wish { forward: 1.0, ..Wish::STILL };
        let mut still = start;
        assert!(fly(&mut still, &Wish::STILL).moved_m == 0.0, "a still step moves nothing");
        let mut one = start;
        let step = fly(&mut one, &forward);
        assert!(
            (step.moved_m - FLY_M_PER_FRAME).abs() < 1e-12,
            "one step moved {} m, not {FLY_M_PER_FRAME}",
            step.moved_m
        );
        let diagonal = Wish { forward: 1.0, strafe: 1.0, rise: 1.0, ..Wish::STILL };
        let mut both = start;
        let step = fly(&mut both, &diagonal);
        assert!(
            (step.moved_m - FLY_M_PER_FRAME).abs() < 1e-12,
            "a three-way diagonal moved {} m",
            step.moved_m
        );
    }

    /// **The pitch stops at the parent's own pole, and the heading survives it**: holding three hundred tilts
    /// past the pole leaves the pitch at [`MAX_PITCH_DEG`], the look a unit vector, and the heading exactly
    /// what it was.
    #[test]
    fn the_pitch_stops_at_the_pole_and_the_heading_survives() {
        let (_, mut viewpoint) = world_and_home();
        let heading = viewpoint.yaw_deg();
        let tilt_up = Wish { tilt: 1.0, ..Wish::STILL };
        for _ in 0..300 {
            fly(&mut viewpoint, &tilt_up);
        }
        let pitch = viewpoint.pitch_deg();
        assert!(
            (pitch - MAX_PITCH_DEG).abs() < 1e-9,
            "300 tilts left the pitch at {pitch}, not {MAX_PITCH_DEG}"
        );
        assert!(
            (viewpoint.yaw_deg() - heading).abs() < 1e-6,
            "the heading moved with the clamp: {heading} -> {}",
            viewpoint.yaw_deg()
        );
        let look = viewpoint.look();
        assert!((length(look) - 1.0).abs() < 1e-12, "the look is not a unit vector");
        // The heading survived the clamp: `yaw_deg` compares the plan direction, which the tilt scales and
        // never rotates — so the drift is the arithmetic's own ulp per step, not a turn.
        // And down the other way.
        let tilt_down = Wish { tilt: -1.0, ..Wish::STILL };
        for _ in 0..600 {
            fly(&mut viewpoint, &tilt_down);
        }
        let pitch = viewpoint.pitch_deg();
        assert!(
            (pitch + MAX_PITCH_DEG).abs() < 1e-9,
            "600 tilts down left the pitch at {pitch}"
        );
    }

    /// **A still camera's own chunk is the one the world's own key law names for its place** — and the anchor
    /// moves exactly when the eye crosses a chunk's plan boundary, which is what makes a walk cost a
    /// derivation.
    #[test]
    fn the_anchor_is_the_chunk_the_eye_stands_over_and_it_moves_at_the_chunk_boundary() {
        let (world, start) = world_and_home();
        let here = start.chunk(&world).expect("the world owns a chunk here");
        // `strafe` is the look's own right, and this eye's heading is `180°` (it looks along `-y`), so a
        // negative strafe is the world's own `-x`.
        let west = Wish { strafe: -1.0, ..Wish::STILL };
        let mut viewpoint = start;
        // **The declared eye stands exactly on the boundary** (`x = 32.0`), so the very first step west is
        // already the next chunk's; a whole chunk's worth of steps is the other side of it by construction.
        fly(&mut viewpoint, &west);
        assert_ne!(
            viewpoint.chunk(&world).expect("a chunk"),
            here,
            "one step west did not cross the plan boundary the eye stands on"
        );
        let mut viewpoint = start;
        for _ in 0..steps_across_a_chunk() {
            fly(&mut viewpoint, &west);
        }
        let across = viewpoint.chunk(&world).expect("a chunk");
        assert_ne!(across, here, "a whole chunk of steps stayed in the same chunk");
        assert_eq!(across.x, here.x - 1, "a whole chunk west is one chunk west, in plan");
        assert_eq!(across.y, here.y);
    }

    /// **The region the frame draws always holds the camera's own chunk and always stands inside the
    /// residency** — wherever the eye is, including past the world's own edge. This is the guard that makes
    /// *the frame is a picture of the cells the residency holds* true of a driven camera: the residency is
    /// declared at the region's own reach, so the region can never be refused by its own name.
    #[test]
    fn the_region_always_holds_the_cameras_chunk_and_sits_inside_the_residency() {
        let (world, home) = world_and_home();
        let horizon = ResidencyHorizon::m(surface::DRAWN_RESIDENCY_M).expect("the declared horizon");
        let mut checked = 0;
        for (x, y) in [
            (32.0, 62.92),
            (63.9, 62.92),
            (-400.0, 62.92),
            (400.0, 400.0),
            (0.0, 0.0),
            (31.9, 30.1),
            (-40.0, -40.0),
        ] {
            let viewpoint = hovered([x, y, 185.90], home.look());
            let anchor = viewpoint.chunk(&world).expect("an anchor");
            let origin = viewpoint.region_origin(&world, surface::DRAWN_CHUNKS).expect("a region");
            for dx in 0..surface::DRAWN_CHUNKS as i32 {
                for dy in 0..surface::DRAWN_CHUNKS as i32 {
                    let key = ChunkKey::new(origin.x + dx, origin.y + dy, anchor.z);
                    assert!(
                        horizon.keeps(anchor, key),
                        "the region's own chunk {key:?} at ({x}, {y}) is outside the \
                         {:.4} m the residency keeps around {anchor:?}",
                        horizon.radius_m()
                    );
                    assert!(
                        world.holds(key),
                        "the region's own chunk {key:?} is not one this world owns"
                    );
                    checked += 1;
                }
            }
            // And the region's own rectangle covers the camera's own chunk's plan cell.
            assert!(
                (origin.x..origin.x + surface::DRAWN_CHUNKS as i32).contains(&anchor.x)
                    && (origin.y..origin.y + surface::DRAWN_CHUNKS as i32).contains(&anchor.y),
                "the region {origin:?} does not cover the camera's own chunk {anchor:?}"
            );
        }
        assert!(checked >= 20, "only {checked} chunk(s) were checked");
    }

    /// **A driven frame is the residency's own cells, and coming home is the recorded frame again** — the one
    /// measurement this pass is asked for, as a guard: the frame drawn from an eye that walked away and back
    /// is the frame the declared view draws, **digest for digest**, with the recorded frame's own counts
    /// (`1333` rectangle(s), `147808` of `331776` pixel(s)) standing as the home frame's identity.
    #[test]
    fn a_walk_away_and_back_draws_the_recorded_frame_again() {
        let (plane, mut residency, mut viewpoint) = client();
        let home = surface::draw_from(&residency, &viewpoint, surface::DRAWN_CHUNK)
            .expect("the declared view's own frame");
        assert_eq!(home.quads(), 1333, "the recorded frame's own rectangles");
        assert_eq!(home.shaded(), 147808, "the recorded frame's own drawn pixel(s)");
        assert_eq!(home.pixels_count(), 331776, "the frame's own size");
        // **The walk is the client's own step door**, one step at a time, drawing every step: the eye moves,
        // the residency is observed at the person's own anchor and the frame is drawn from where they stand.
        let west = Wish { strafe: -1.0, ..Wish::STILL };
        let mut walked = Vec::new();
        for _ in 0..steps_across_a_chunk() {
            walked.push(
                surface::step_driven(
                    &plane,
                    &mut residency,
                    &mut viewpoint,
                    &west,
                    surface::frame_seconds(),
                )
                .expect("a step west"),
            );
        }
        let away = walked.last().expect("a step").drawn.clone();
        assert_ne!(away.digest(), home.digest(), "a walk did not move the picture");
        assert_ne!(away.shaded(), home.shaded(), "a walk did not move the covered pixel(s)");
        assert!(
            walked.iter().all(|step| step.stream.resident == plane.world().chunks()),
            "the declared horizon stopped holding this world whole"
        );
        let mut returned = Vec::new();
        let east = Wish { strafe: 1.0, ..Wish::STILL };
        for _ in 0..steps_across_a_chunk() {
            returned.push(
                surface::step_driven(
                    &plane,
                    &mut residency,
                    &mut viewpoint,
                    &east,
                    surface::frame_seconds(),
                )
                .expect("a step east"),
            );
        }
        let back = &returned.last().expect("a step").drawn;
        assert_eq!(
            back.digest(),
            home.digest(),
            "coming home drew a different picture: {} rectangle(s), {} of {} pixel(s)",
            back.quads(),
            back.shaded(),
            back.pixels_count()
        );
        assert_eq!(back.quads(), 1333);
        assert_eq!(back.shaded(), 147808);
        assert_eq!(viewpoint.eye(), surface::home_viewpoint(
            &surface::region_mesh(&residency, surface::DRAWN_CHUNK).expect("the region mesh")
        ).expect("the declared view").eye());
    }

    /// **An eye inside the world it draws counts the rectangles it does not see** — the state the drawn
    /// reading's third bar exists for, and the one a person reaches by flying **down** into the geometry they
    /// are drawing: the declared eye stands `185.90` m up, the region's own columns span `64..96` m, and a
    /// person who has flown down to `85` m is inside them. Every rectangle the near plane cuts is *unseen* and
    /// costs exactly its own two triangles, so the accounting is **total**: nothing can be dropped between the
    /// eye and the picture without a count going wrong. (The declared view has none of them — the eye stands
    /// above everything — which is why this guard's own subject is a placed eye.)
    #[test]
    fn an_eye_inside_the_region_counts_every_rectangle_it_does_not_see() {
        let (_, residency, viewpoint) = client();
        let viewpoint = hovered([32.0, 32.0, 85.0], viewpoint.look());
        let frame = surface::draw_from(&residency, &viewpoint, surface::DRAWN_CHUNK)
            .expect("a frame from inside the region");
        assert!(
            frame.unseen() > 0,
            "no rectangle was behind the eye's own near plane: the eye is not inside the region"
        );
        assert!(
            frame.shaded() > 0,
            "the frame from inside the region drew nothing at all"
        );
        assert_eq!(
            frame.clipped(),
            2 * frame.unseen(),
            "an unseen rectangle cost more or fewer than its own two triangles: {} against {}",
            frame.clipped(),
            2 * frame.unseen()
        );
    }

    /// **The residency's own key is the chunk the person's own eye stands in** — the streamer's own work
    /// through the client's own step door: a still step is free, a step that crosses a chunk's plan boundary
    /// asks the residency about **that** chunk, and the step's own answer carries the anchor it was asked
    /// about, so a camera and the set it streams cannot disagree about where the eye is.
    #[test]
    fn every_step_observes_the_residency_at_the_chunk_the_eye_now_stands_in() {
        let (plane, mut residency, mut viewpoint) = client();
        let world = plane.world();
        let first = surface::step_driven(
            &plane,
            &mut residency,
            &mut viewpoint,
            &Wish::STILL,
            surface::frame_seconds(),
        )
        .expect("a still step");
        assert!(first.was_free(), "a still step cost a derivation: {first:?}");
        assert_eq!(first.anchor, viewpoint.chunk(world).expect("a chunk"));
        let west = Wish { strafe: -1.0, ..Wish::STILL };
        let mut steps = 0;
        loop {
            let step = surface::step_driven(
                &plane,
                &mut residency,
                &mut viewpoint,
                &west,
                surface::frame_seconds(),
            )
            .expect("a step west");
            steps += 1;
            assert_eq!(
                step.stream.anchor,
                step.anchor,
                "the residency was observed somewhere other than the step's own anchor"
            );
            assert_eq!(step.anchor, viewpoint.chunk(world).expect("a chunk"));
            if step.anchor != first.anchor {
                break;
            }
            assert!(steps < steps_across_a_chunk() + 2, "the walk never reached another chunk");
        }
        assert_eq!(steps, 1, "the declared eye stands on the boundary, so one step west is one chunk");
    }

    fn length(v: [f64; 3]) -> f64 {
        (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
    }
}
