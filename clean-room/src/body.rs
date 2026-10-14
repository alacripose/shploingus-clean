//! **U1 — the client's own body: where the player is, and the one law that moves it (2026-10-14g).**
//!
//! # The gap this closes, and the one owner it lands in
//!
//! Until this pass the client's eye **was** the player: `crate::viewpoint::Viewpoint` owned an eye and the eye
//! flew. `crates/shploingus/src/main.rs`'s own words were quoted in the record as the clean room's state —
//! *"No physics. Moving is free flight, so the camera can pass through terrain"* — while the parent's **live**
//! client is a body: `walk_player` calls `player.advance(&wish, yaw, dt, &world)` and derives the camera from
//! it (`self.camera.position_m = self.player.interpolated_eye()`), so the eye can never be somewhere the body
//! is not. This module is that body, and it owns exactly one thing: **where the player's feet are, how fast
//! they are moving, and what mode they are moving in**. The eye is **derived** ([`Body::eye`]): a stored eye
//! beside a stored body would be two owners of one fact.
//!
//! # The law, and where every number comes from
//!
//! One step is one frame of this client's own declared clock ([`crate::surface::FRAMES_PER_SECOND`], passed in
//! as `dt_s`), so the numbers below are the parent's own read in this client's units:
//!
//! | number | value | source |
//! |---|---|---|
//! | [`EYE_HEIGHT_M`] | `1.62` | the parent's `EYE_HEIGHT` (`crates/voxel-client/src/motion.rs`) |
//! | [`BODY_HEIGHT_M`] | `1.8` | the parent's `BODY_HEIGHT` — the box a walk is blocked by |
//! | [`BODY_RADIUS_M`] | `0.3` | the parent's `BODY_RADIUS` — the disc a wall stops |
//! | [`STEP_UP_M`] | `0.55` | the parent's `STEP_UP`: a blocked horizontal move is retried standing this much higher, and gravity settles the last centimetres — the alternative (teleporting to the top of the ledge) is not a walk |
//! | [`WALK_M_PER_S`] | `4.317` | the parent's `WALK_SPEED` |
//! | [`JUMP_M_PER_S`] | `8.4` | the parent's `JUMP_SPEED`, the vertical speed a held jump sets |
//! | [`GRAVITY_M_PER_S2`] | `-32.0` | the parent's `GRAVITY_PER_TICK = -1.6` over its `PLAYER_STEP = 1/20 s`, read as the acceleration it is |
//! | [`FLY_M_PER_S`] | `18.0` | the parent's `FLY_M_PER_S` (`crates/shploingus/src/main.rs`), the speed of the flight a person chooses |
//!
//! **What is not ported, named rather than implied.** The parent's reference model is deeper than this: friction
//! against a surface's own slipperiness, an acceleration curve with an air-acceleration cap, air resistance,
//! sprint and sneak speeds, step banking for a frame-rate-independent clock, trimping and sharking. This body
//! walks at a constant declared speed, jumps, falls, is stopped and stands — the smallest law that makes
//! *the eye follows the body* true — and the rest is owed by name.
//!
//! # Gravity, ground and blocking, in the order the parent applies them
//!
//! One [`Body::step`] does, in this order: the jump (if asked and standing), the **horizontal** move (which is
//! what the terrain blocks and what a step is climbed by), the **vertical** move (which is what the ground
//! stops), and **gravity last, applied even when standing** — the parent's own order, kept because moving
//! gravity earlier changes the discrete arc of a jump. A standing body's vertical speed after a step is
//! `GRAVITY_M_PER_S2 * dt_s` and the next step's collision cancels it, which is why a still body does not sink.
//!
//! # Flight is a choice, and walking is the default
//!
//! [`Traversal`] has exactly two values and [`Traversal::Walking`] is the [`Default`]. A flying body ignores
//! ground, gravity and collision — it is the parent's own *"Moving is free flight, so the camera can pass
//! through terrain"*, kept as an **option a person takes** rather than as what a person is. The reading's own
//! observing eye (the declared view the recorded frames are drawn from) is a **flying** body at the place that
//! camera has always stood ([`Body::hovering`]), which is the honest name for it: that eye is not a person, it
//! is the instrument's vantage.

use crate::chunk::CubicWorld;
use crate::coordinates::VoxelPos;
use crate::refusal::Refusal;

/// **The standing eye, in metres, above the feet.** The parent's own `EYE_HEIGHT`. The eye is derived from the
/// feet with this and never stored ([`Body::eye`]).
pub const EYE_HEIGHT_M: f64 = 1.62;

/// **How tall the body is**, in metres: the box a horizontal move must fit in. The parent's own `BODY_HEIGHT`.
pub const BODY_HEIGHT_M: f64 = 1.8;

/// **How wide the body is**, in metres: the radius of the disc a wall stops. The parent's own `BODY_RADIUS`.
pub const BODY_RADIUS_M: f64 = 0.3;

/// **The height a blocked move is retried at**, in metres. The parent's own `STEP_UP`.
pub const STEP_UP_M: f64 = 0.55;

/// **How far a walking body travels in a second**, in metres. The parent's own `WALK_SPEED`.
pub const WALK_M_PER_S: f64 = 4.317;

/// **The vertical speed a jump sets**, in metres per second. The parent's own `JUMP_SPEED`.
pub const JUMP_M_PER_S: f64 = 8.4;

/// **Gravity**, in metres per second squared — the parent's own `GRAVITY_PER_TICK = -1.6` over its
/// `PLAYER_STEP = 1/20 s`, which is this acceleration.
pub const GRAVITY_M_PER_S2: f64 = -32.0;

/// **How far a flying body travels in a second**, in metres. The parent's own `FLY_M_PER_S`, and the number
/// [`crate::viewpoint::FLY_M_PER_FRAME`] is this over the client's declared frame rate.
pub const FLY_M_PER_S: f64 = 18.0;

/// **How high a held jump carries the body above where it stood**, in metres — the apex of [`JUMP_M_PER_S`]
/// under [`GRAVITY_M_PER_S2`]: `8.4² / (2 · 32) = 1.1025` m. Derived from the two numbers rather than tuned,
/// and the height a rise has to be under to be jumped: **one metre of this world's own voxels clears it, two
/// does not**, which is what makes a ledger of what a body can reach an arithmetic fact.
pub const JUMP_APEX_M: f64 = JUMP_M_PER_S * JUMP_M_PER_S / (-2.0 * GRAVITY_M_PER_S2);

/// **How far ahead a walk reads the terrain before it decides to jump**, in metres — inside the body's own
/// step (`0.1439` m a frame) and outside its own disc (`0.3` m), so it is the column the body is walking into
/// and not one beyond it.
pub const JUMP_LOOKAHEAD_M: f64 = 0.5;

/// **The tolerance a body is *standing on* the floor within**, in metres — the parent's own `GROUND_EPS`. A
/// body whose feet are this close to the floor under them is grounded; anything higher is in the air and falls.
pub const GROUND_EPS_M: f64 = 1e-6;

/// **The skin a collision probe keeps off the floor**, in metres. A body resting exactly on a cell's own top
/// must not count that cell as part of itself, and floating-point contact is not exact — so the body's own
/// lowest cell is read from `feet + this`, not from `feet`.
pub const SKIN_M: f64 = 1e-3;

/// **How far a collision probe searches**, in cells — the world's own span is far smaller, so a body over
/// nothing finds nothing rather than searching forever.
pub const PROBE_CELLS: i32 = 128;

/// **A move's own sub-step**, in metres: a step longer than this is cut into sub-steps, so a fast body cannot
/// pass through a cell between two probes. The parent's own `SUBSTEP_M`.
pub const SUBSTEP_M: f64 = 0.25;

/// **How a body traverses the world** — the two ways there are, and walking is the [`Default`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Traversal {
    /// **Walking**: gravity, ground, steps and blocking apply. What a person is by default.
    #[default]
    Walking,
    /// **Flying**: the eye moves freely and passes through terrain. An option a person takes, not a default.
    Flying,
}

impl Traversal {
    /// The word the run prints for this traversal.
    pub const fn name(self) -> &'static str {
        match self {
            Traversal::Walking => "walking",
            Traversal::Flying => "flying",
        }
    }
}

/// **What a body is asked to do in one step**, in the eye's own terms: a unit direction in world space (the
/// wish expressed on the eye's own basis, which is [`crate::viewpoint::Viewpoint`]'s own arithmetic) and
/// whether the vertical intent is a jump. The vertical part of `along` is the flight's own up/down; a jump is
/// its own flag because a walking body jumps rather than being lifted along whatever it happens to look at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Move {
    /// The unit direction the body is asked to travel, in world metres per unit distance.
    pub along: [f64; 3],
    /// Whether the body is asked to jump — the walk's own vertical intent, held like the parent's own.
    pub jump: bool,
}

impl Move {
    /// A move that asks for nothing; a step with it still steps (gravity applies, and a standing body stays).
    pub const STILL: Move = Move { along: [0.0; 3], jump: false };
}

/// **What a body can be stopped by** — whether a cell blocks movement, and whether it is water.
///
/// A trait rather than a direct call into the world so this module's own tests can stand a floor, a step and a
/// wall in a few lines and measure the law without a generator, a window or a mesh — and so the one production
/// implementation ([`Cells`]) is the only place the world is asked. The parent's own `Solidity` is the same
/// seam for the same reason.
pub trait Solidity {
    /// Whether the cell at this integer coordinate blocks movement.
    fn is_solid(&self, at: [i32; 3]) -> bool;

    /// Whether the cell at this integer coordinate holds water. Water is not solid (a body sinks through it),
    /// which is why this is a second question. Nothing in this pass's law reads it yet — the parent's own swim
    /// and water-acceleration branches are owed by name — so a body walks *through* water today.
    fn is_water(&self, _at: [i32; 3]) -> bool {
        false
    }
}

/// **The world's own cells, as what a body is stopped by** — the one production [`Solidity`].
///
/// Three answers, and each is the world's own rather than this adapter's:
///
/// * **a cell this world owns**: its own answer (`CubicWorld::cell`, solid or not);
/// * **a place inside the world's own plan but outside its own chunk layers** (above the terrain, as the eye
///   of a person on a hill is): **air** — the world owns no cell there because there is nothing there;
/// * **a place outside the world's own plan, or below its own span**: **solid** — the world's own edge and its
///   own floor. A place this world owns no cell for is not a place the drawn frame can follow a person to, so
///   a body is stopped by the world's own extent rather than walking out of the world it is drawn from.
pub struct Cells<'a> {
    world: &'a CubicWorld,
}

impl<'a> Cells<'a> {
    /// The world's own cells, read through the body's own question.
    pub const fn over(world: &'a CubicWorld) -> Self {
        Cells { world }
    }

    /// Whether the world's own patch holds this plan column.
    fn inside_plan(&self, x: i32, z: i32) -> bool {
        let patch = self.world.patch();
        let origin = patch.origin();
        let side = patch.side() as i32;
        (origin.x..origin.x + side).contains(&x) && (origin.z..origin.z + side).contains(&z)
    }
}

impl Solidity for Cells<'_> {
    fn is_solid(&self, at: [i32; 3]) -> bool {
        match self.world.cell(VoxelPos::new(at[0], at[1], at[2])) {
            Ok(cell) => cell.is_solid(),
            Err(_) => !self.inside_plan(at[0], at[1]) || at[2] < self.world.span_z().0,
        }
    }

    fn is_water(&self, at: [i32; 3]) -> bool {
        self.world
            .cell(VoxelPos::new(at[0], at[1], at[2]))
            .is_ok_and(|cell| cell.water_um > 0)
    }
}

/// **Where the body is** — the feet, the velocity, and how it traverses. One owner: the eye is derived.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Body {
    /// The body's **feet**, in world metres — the point [`Body::eye`] is measured up from.
    feet: [f64; 3],
    /// Metres per second on each axis. Gravity and a jump are the only things that change it.
    velocity: [f64; 3],
    /// Whether the feet are resting on the floor under them this step.
    grounded: bool,
    /// How this body traverses the world.
    traversal: Traversal,
}

impl Body {
    /// **A standing body**: its feet on `feet`, at rest, walking, and grounded as declared — the first step
    /// corrects that answer against the world's own cells if it is wrong.
    pub const fn standing(feet: [f64; 3]) -> Self {
        Body { feet, velocity: [0.0; 3], grounded: true, traversal: Traversal::Walking }
    }

    /// **The body a place in the air is** — the feet [`EYE_HEIGHT_M`] below `eye`, at rest, **flying**.
    ///
    /// This is how the reading's own observing eye is read as a body ([`crate::surface::home_viewpoint`]):
    /// that camera hovers over the region to frame it and is no person's feet, so its traversal is the one
    /// that does not pretend there is ground under it.
    pub const fn hovering(eye: [f64; 3]) -> Self {
        Body {
            feet: [eye[0], eye[1], eye[2] - EYE_HEIGHT_M],
            velocity: [0.0; 3],
            grounded: false,
            traversal: Traversal::Flying,
        }
    }

    /// The feet, in world metres.
    pub const fn feet(&self) -> [f64; 3] {
        self.feet
    }

    /// **The eye — derived, never stored**: the feet plus [`EYE_HEIGHT_M`]. A stored eye beside a stored body
    /// would be two sources of truth for one fact, which is why the parent derives its own camera every frame.
    pub const fn eye(&self) -> [f64; 3] {
        [self.feet[0], self.feet[1], self.feet[2] + EYE_HEIGHT_M]
    }

    /// Metres per second on each axis.
    pub const fn velocity(&self) -> [f64; 3] {
        self.velocity
    }

    /// Whether the feet rest on the floor under them.
    pub const fn grounded(&self) -> bool {
        self.grounded
    }

    /// How this body traverses the world.
    pub const fn traversal(&self) -> Traversal {
        self.traversal
    }

    /// **Take up a traversal** — the one door the mode changes through, so *walking* and *flying* are a
    /// declared state rather than a flag read beside it. Taking up flight drops the body's own vertical speed:
    /// a fall does not carry into a hover.
    pub fn traverse(&mut self, traversal: Traversal) {
        if traversal != self.traversal {
            self.velocity = [0.0; 3];
        }
        self.traversal = traversal;
    }

    /// **One step of the body's own law.** See the module header for the order and why it is the parent's.
    pub fn step(&mut self, dt_s: f64, solids: &impl Solidity, request: &Move) -> BodyStep {
        if !dt_s.is_finite() || dt_s <= 0.0 {
            // A frame that reports no time (or a negative one) moves nothing rather than moving something
            // plausible: the same rule the parent's own bank keeps.
            return BodyStep::still(self);
        }
        match self.traversal {
            Traversal::Flying => self.step_flying(dt_s, request),
            Traversal::Walking => self.step_walking(dt_s, solids, request),
        }
    }

    /// **Flight**: the eye moves freely, at [`FLY_M_PER_S`], and passes through terrain — the parent's own
    /// free flight, kept as an option. No gravity, no ground, no collision, and no vertical speed to keep.
    fn step_flying(&mut self, dt_s: f64, request: &Move) -> BodyStep {
        let before = self.feet;
        let distance = FLY_M_PER_S * dt_s;
        for axis in 0..3 {
            self.feet[axis] += distance * request.along[axis];
        }
        self.grounded = false;
        BodyStep::of(self, before, false, 0.0)
    }

    /// **A walk**: jump, horizontal move (blocked by terrain, climbed by a step), vertical move (stopped by
    /// ground), then gravity — the parent's own order, with gravity last even when standing.
    fn step_walking(&mut self, dt_s: f64, solids: &impl Solidity, request: &Move) -> BodyStep {
        let before = self.feet;
        let mut jumped = false;
        let mut climbed_m = 0.0;
        let mut blocked = false;

        // 1. The jump, if it is asked for and the body is standing on something.
        if request.jump && self.grounded {
            self.velocity[2] = JUMP_M_PER_S;
            self.grounded = false;
            jumped = true;
        }

        // 2. The horizontal move, sub-stepped so a fast body cannot pass through a cell, and split by axis so
        //    a body walking into a wall slides along it rather than being stopped dead by it.
        let unit = plan_unit(request.along);
        let travel = WALK_M_PER_S * dt_s;
        let substeps = ((travel / SUBSTEP_M).ceil().max(1.0)) as usize;
        let sub_dt = dt_s / substeps as f64;
        for _ in 0..substeps {
            for axis in [0_usize, 1] {
                let outcome = self.move_horizontal(axis, unit[axis] * WALK_M_PER_S * sub_dt, solids);
                climbed_m += outcome.climbed_m;
                blocked |= outcome.blocked;
            }
            self.move_vertical(self.velocity[2] * sub_dt, solids);
        }

        // 3. Gravity **last**, and applied whether or not the body is grounded — the parent's own order. A
        //    standing body's vertical speed is cancelled by the next step's own collision.
        self.velocity[2] += GRAVITY_M_PER_S2 * dt_s;
        self.velocity[0] = 0.0;
        self.velocity[1] = 0.0;
        // The ground is the world's own answer, not a remembered flag: standing is `feet == floor`.
        self.grounded = floor_under(solids, [self.feet[0], self.feet[1]], self.feet[2])
            .is_some_and(|floor| (self.feet[2] - floor).abs() < GROUND_EPS_M);
        BodyStep::of(self, before, jumped, climbed_m).blocked(blocked)
    }

    /// **Move on one horizontal axis, with the step-up retry** — the parent's own rule, in this client's
    /// units: the flat move if the body fits there, otherwise the same move standing [`STEP_UP_M`] higher if
    /// the body fits *there* (which is what makes a step a climb rather than a wall), otherwise nothing.
    fn move_horizontal(&mut self, axis: usize, amount: f64, solids: &impl Solidity) -> MoveOutcome {
        if amount == 0.0 || !amount.is_finite() {
            return MoveOutcome::default();
        }
        let feet = self.feet;
        let mut flat = feet;
        flat[axis] += amount;
        if !disc_hits(solids, flat) {
            self.feet = flat;
            return MoveOutcome::default();
        }
        // Blocked at this height: retry standing `STEP_UP_M` higher. If the body fits there the move is taken
        // **at the raised height** and gravity settles the last centimetres — the alternative, moving the body
        // to the top of the ledge outright, is a teleport rather than a walk.
        let mut raised = flat;
        raised[2] = feet[2] + STEP_UP_M;
        if !disc_hits(solids, raised) {
            self.feet = raised;
            self.grounded = false;
            self.velocity[2] = 0.0;
            return MoveOutcome { climbed_m: raised[2] - feet[2], blocked: false };
        }
        // Blocked outright: stay put and stop pushing into it.
        self.velocity[axis] = 0.0;
        MoveOutcome { climbed_m: 0.0, blocked: true }
    }

    /// **Move along the world's own up** — down until the floor stops the body, up until its head meets a
    /// ceiling. The floor is the world's own highest solid cell under the feet; a body that walks off a ledge
    /// finds none and falls.
    fn move_vertical(&mut self, amount: f64, solids: &impl Solidity) {
        if amount == 0.0 || !amount.is_finite() {
            self.grounded = floor_under(solids, [self.feet[0], self.feet[1]], self.feet[2])
                .is_some_and(|floor| (self.feet[2] - floor).abs() < GROUND_EPS_M);
            return;
        }
        if amount < 0.0 {
            let wanted = self.feet[2] + amount;
            match floor_under(solids, [self.feet[0], self.feet[1]], wanted) {
                Some(floor) if floor > wanted => {
                    // The floor is between the body and where it was going: it stops on the floor.
                    self.feet[2] = floor;
                    self.velocity[2] = 0.0;
                    self.grounded = true;
                }
                _ => {
                    self.feet[2] = wanted;
                    self.grounded = false;
                }
            }
            return;
        }
        let head_from = self.feet[2] + BODY_HEIGHT_M;
        if !ceiling_between(solids, [self.feet[0], self.feet[1]], head_from, head_from + amount) {
            self.feet[2] += amount;
        } else {
            self.velocity[2] = 0.0;
        }
        self.grounded = floor_under(solids, [self.feet[0], self.feet[1]], self.feet[2])
            .is_some_and(|floor| (self.feet[2] - floor).abs() < GROUND_EPS_M);
    }

    /// **The body, named** — where its feet are, whether they are on the ground, and how it traverses.
    pub fn described(&self) -> String {
        format!(
            "a {} body with its feet at ({:.2}, {:.2}, {:.2}) m and its eye at ({:.2}, {:.2}, {:.2}) m, {}",
            self.traversal.name(),
            self.feet[0],
            self.feet[1],
            self.feet[2],
            self.eye()[0],
            self.eye()[1],
            self.eye()[2],
            if self.grounded { "standing on the ground" } else { "in the air" },
        )
    }
}

/// What one horizontal move's own attempt came to, before the step sums them.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct MoveOutcome {
    /// How far the body was raised by the step-up retry, in metres.
    climbed_m: f64,
    /// Whether the move was refused outright by what is there.
    blocked: bool,
}

/// **What one step of a body did** — measured, not described: how far it travelled in plan, how far it rose or
/// fell, how far it was raised by a step, whether it jumped, whether it is standing, whether something stopped
/// it, and how it was traversing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyStep {
    /// The plan distance travelled, in metres.
    pub plan_m: f64,
    /// The vertical distance moved **upward** (a jump's own rise, or a flight's), in metres.
    pub rose_m: f64,
    /// The vertical distance moved **downward**, in metres — a fall, or gravity settling onto a step.
    pub fell_m: f64,
    /// How far a blocked horizontal move was raised to clear what was there, in metres (the step-up retry).
    pub climbed_m: f64,
    /// Whether a jump was taken this step.
    pub jumped: bool,
    /// Whether the feet rest on the floor at the end of the step.
    pub grounded: bool,
    /// Whether what is there refused a horizontal move outright.
    pub blocked: bool,
    /// How the body was traversing for this step.
    pub traversal: Traversal,
}

impl BodyStep {
    /// The step a still body takes: it may still fall, and gravity still applies.
    fn of(body: &Body, before: [f64; 3], jumped: bool, climbed_m: f64) -> Self {
        let (dx, dy, dz) = (
            body.feet[0] - before[0],
            body.feet[1] - before[1],
            body.feet[2] - before[2],
        );
        BodyStep {
            plan_m: (dx * dx + dy * dy).sqrt(),
            rose_m: dz.max(0.0),
            fell_m: (-dz).max(0.0),
            climbed_m,
            jumped,
            grounded: body.grounded,
            blocked: false,
            traversal: body.traversal,
        }
    }

    /// A body that could not step at all — no time passed, so nothing moved.
    fn still(body: &Body) -> Self {
        BodyStep::of(body, body.feet, false, 0.0)
    }

    /// This step with the blocking flag set (the step is built before the flag is known).
    fn blocked(mut self, blocked: bool) -> Self {
        self.blocked = blocked;
        self
    }

    /// Whether this step moved nothing at all — no plan travel, no rise, no fall. What a still camera's own
    /// step is to the controller: it counts, and it costs no derivation.
    pub fn was_still(&self) -> bool {
        self.plan_m == 0.0 && self.rose_m == 0.0 && self.fell_m == 0.0
    }
}

/// **The unit plan direction a move asks for** — the horizontal part of the move's own direction, normalized
/// so a diagonal is not faster than a straight line. A move with no plan part asks for none.
pub fn plan_unit(along: [f64; 3]) -> [f64; 3] {
    let length = (along[0] * along[0] + along[1] * along[1]).sqrt();
    if length <= 0.0 || !length.is_finite() {
        return [0.0; 3];
    }
    [along[0] / length, along[1] / length, 0.0]
}

/// **The floor under a plan position, at or below a height** — the top of the world's own highest solid cell
/// under it, in metres, or nothing when the body is over air.
///
/// **The world's own axes are the ones the cells are addressed in** — `(x, y)` is the plan and `z` the span —
/// so a plan position is a pair and the height is the world's own `z`. A cell at integer `z` occupies
/// `[z, z + 1)`, so a body standing on the cell at `z` has its feet at `z + 1`; a body whose feet are at `f`
/// can be standing on the cell `ceil(f) - 1` at the highest, which is where the search starts and it walks down
/// from there.
pub fn floor_under(solids: &impl Solidity, plan: [f64; 2], height_m: f64) -> Option<f64> {
    let (xi, yi) = (plan[0].floor() as i32, plan[1].floor() as i32);
    let mut z = height_m.ceil() as i32 - 1;
    let low = z - PROBE_CELLS;
    while z > low {
        if solids.is_solid([xi, yi, z]) {
            return Some(f64::from(z) + 1.0);
        }
        z -= 1;
    }
    None
}

/// **Whether the body's own box is inside anything solid** — the disc of [`BODY_RADIUS_M`] around the plan
/// position, over the height `[feet, feet + BODY_HEIGHT_M)`. A body standing exactly on a cell's own top does
/// not count that cell as part of itself, which is what [`SKIN_M`] is for.
pub fn disc_hits(solids: &impl Solidity, feet: [f64; 3]) -> bool {
    let low_x = (feet[0] - BODY_RADIUS_M).floor() as i32;
    let high_x = (feet[0] + BODY_RADIUS_M).floor() as i32;
    let low_y = (feet[1] - BODY_RADIUS_M).floor() as i32;
    let high_y = (feet[1] + BODY_RADIUS_M).floor() as i32;
    let bottom = (feet[2] + SKIN_M).floor() as i32;
    let top = (feet[2] + BODY_HEIGHT_M - SKIN_M).ceil() as i32 - 1;
    (bottom..=top).any(|z| {
        (low_x..=high_x)
            .any(|x| (low_y..=high_y).any(|y| solids.is_solid([x, y, z])))
    })
}

/// **Whether anything solid stands between two heights** — the body's own disc, asked over a vertical range,
/// which is how a ceiling stops an upward move without a second collision routine.
fn ceiling_between(solids: &impl Solidity, plan: [f64; 2], from: f64, to: f64) -> bool {
    let low_x = (plan[0] - BODY_RADIUS_M).floor() as i32;
    let high_x = (plan[0] + BODY_RADIUS_M).floor() as i32;
    let low_y = (plan[1] - BODY_RADIUS_M).floor() as i32;
    let high_y = (plan[1] + BODY_RADIUS_M).floor() as i32;
    let bottom = from.floor() as i32;
    let top = to.ceil() as i32;
    (bottom..=top).any(|z| {
        (low_x..=high_x)
            .any(|x| (low_y..=high_y).any(|y| solids.is_solid([x, y, z])))
    })
}

/// **The one judgement a caller with no world can still make** — a refusal wanting a place to stand, by name.
/// Used where a body must be placed on ground the caller cannot find.
pub fn no_ground_refusal(where_: &str) -> Refusal {
    Refusal::new(
        crate::refusal::RefusalKind::ParameterOutOfRange,
        format!("there is no ground under {where_}: a body cannot stand where the world holds no solid cell"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **A declared fixture**: a floor, a step, a wall and a ceiling, so the law can be measured without a
    /// generator, a mesh or a window. It is written in the world's own axes — a plan position is `(x, y)` and
    /// the span is `z`, which is the frame every cell of the drawn world is addressed in. Cells are half-open
    /// boxes `[z, z + 1)`; `air` is above the floor unless one of the named features is there.
    /// **A fixture's heights are the number of solid cells and so the metres they reach**: `floor: 1` is cells
    /// `z < 1` solid, whose own top is `1.0` m — the world's own voxel law, where a cell at `z` occupies
    /// `[z, z + 1)`.
    struct Fixture {
        /// The floor's own top, in solid cells: everything below this is solid.
        floor: i32,
        /// A step's own top: everything at and below this, at and east of `step_x`, is solid.
        step_top: i32,
        step_x: i32,
        /// A wall: everything at and below this, at and east of `wall_x`, is solid (too tall to step).
        wall_top: i32,
        wall_x: i32,
    }

    impl Default for Fixture {
        fn default() -> Self {
            // The floor stands at `1.0` m, the step is the whole metre above it (`2.0` m — a metre is what this
            // world's own voxels make the smallest real rise), and the wall is two metres above the step.
            Fixture { floor: 1, step_top: 2, step_x: 4, wall_top: 4, wall_x: 9 }
        }
    }

    impl Solidity for Fixture {
        fn is_solid(&self, at: [i32; 3]) -> bool {
            let [x, _y, z] = at;
            if z < self.floor {
                return true;
            }
            if x >= self.wall_x {
                return z < self.wall_top;
            }
            if x >= self.step_x {
                return z < self.step_top;
            }
            false
        }
    }

    /// A move straight east, on the plan, at walking speed.
    const EAST: Move = Move { along: [1.0, 0.0, 0.0], jump: false };

    fn step_n(body: &mut Body, fixtures: &impl Solidity, request: &Move, n: usize) -> BodyStep {
        let mut last = BodyStep::still(body);
        for _ in 0..n {
            last = body.step(1.0 / 30.0, fixtures, request);
        }
        last
    }

    #[test]
    fn the_eye_is_the_feet_plus_the_declared_height() {
        // The one owner: there is no stored eye to disagree with the feet.
        let mut body = Body::standing([3.0, 4.0, 1.0]);
        assert_eq!(body.eye(), [3.0, 4.0, 1.0 + EYE_HEIGHT_M]);
        body.traverse(Traversal::Flying);
        step_n(&mut body, &Fixture::default(), &Move::STILL, 1);
        assert_eq!(body.eye()[2], body.feet()[2] + EYE_HEIGHT_M);
    }

    #[test]
    fn a_standing_body_does_not_sink_and_a_still_step_moves_nothing() {
        // Gravity is applied every step and cancelled by the ground, so a still body's step is still.
        let fixtures = Fixture::default();
        let mut body = Body::standing([2.0, 2.0, 1.0]);
        let before = body.feet();
        let step = step_n(&mut body, &fixtures, &Move::STILL, 20);
        assert_eq!(body.feet(), before, "a standing body stays where it stands");
        assert!(step.grounded, "and it is grounded");
        assert!(step.was_still(), "a still step moves nothing: {step:?}");
        assert!(step.fell_m == 0.0 && step.rose_m == 0.0);
    }

    #[test]
    fn gravity_pulls_a_body_out_of_the_air_onto_the_floor_and_the_eye_follows() {
        // The whole point of a body: the eye is where the feet are, so a fall moves the eye.
        let fixtures = Fixture::default();
        let mut body = Body::standing([2.0, 2.0, 12.0]);
        let eye_start = body.eye();
        let mut fell = 0.0;
        let mut landed = None;
        for _ in 0..120 {
            let step = body.step(1.0 / 30.0, &fixtures, &Move::STILL);
            fell += step.fell_m;
            if step.grounded {
                landed = Some(step);
                break;
            }
        }
        let landed = landed.expect("a falling body lands on the floor");
        assert!(fell > 10.0, "the body fell most of the way: {fell} m");
        assert_eq!(body.feet()[2], 1.0, "the feet stop on the floor's own top");
        assert_eq!(body.eye()[2], 1.0 + EYE_HEIGHT_M, "and the eye follows them");
        assert!(body.eye()[2] < eye_start[2], "which is lower than it was");
        assert!(landed.grounded && landed.traversal == Traversal::Walking);
    }

    #[test]
    fn a_one_metre_step_is_not_walked_up_and_the_step_up_cannot_fire() {
        // **The parent's own finding, asserted here** (`docs/LEDGER.md` §95.5): voxels are one metre, so the
        // smallest real rise is a whole metre and `STEP_UP = 0.55` can never clear it. The walk is stopped at
        // the step, `climbed_m` stays zero however long the body pushes, and the body's own place does not
        // move — a guard on the law this world actually has, rather than on the sub-voxel relief it has not.
        let fixtures = Fixture::default();
        let mut body = Body::standing([2.0, 2.0, 1.0]);
        let mut climbed = 0.0;
        let mut blocked = false;
        for _ in 0..60 {
            let step = body.step(1.0 / 30.0, &fixtures, &EAST);
            climbed += step.climbed_m;
            blocked |= step.blocked;
        }
        assert!(blocked, "a metre-high step refused the walk");
        assert_eq!(climbed, 0.0, "the step-up retry fired on a voxel grid: {climbed} m");
        assert!(
            body.feet()[0] < f64::from(fixtures.step_x),
            "the body walked up a metre-high step: {}",
            body.feet()[0]
        );
        assert_eq!(body.feet()[2], 1.0, "and it is still standing on the floor");
    }

    #[test]
    fn the_held_jump_carries_the_walk_onto_the_step_and_the_eye_climbs_with_it() {
        // What this world leaves for a ledge: the same walk, with the jump the parent's own record says a real
        // ledge needs. The body is refused, jumps holding the direction, lands **on** the step and keeps
        // walking — so the climb is the body's own and the eye follows it up a whole metre.
        let fixtures = Fixture::default();
        let mut body = Body::standing([2.0, 2.0, 1.0]);
        let hop = Move { along: EAST.along, jump: true };
        // Walk east into the step.
        let mut refused = false;
        for _ in 0..60 {
            refused |= body.step(1.0 / 30.0, &fixtures, &EAST).blocked;
            if refused {
                break;
            }
        }
        assert!(refused, "the step refused the walk before the jump");
        let before = body.feet();
        assert!(body.step(1.0 / 30.0, &fixtures, &hop).jumped, "the jump was taken");
        let mut landed = false;
        for _ in 0..60 {
            let step = body.step(1.0 / 30.0, &fixtures, &EAST);
            if step.grounded {
                landed = true;
                break;
            }
        }
        assert!(landed, "the jump came back down");
        assert_eq!(body.feet()[2], 2.0, "onto the step's own top");
        assert!(body.feet()[2] > before[2] + 0.5, "which is a step higher than it was");
        assert_eq!(body.eye()[2], 2.0 + EYE_HEIGHT_M, "and the eye climbed with it");
        // And the walk carries on from the step: the body is on top of it, not pressed against it.
        for _ in 0..30 {
            body.step(1.0 / 30.0, &fixtures, &EAST);
        }
        assert!(body.feet()[0] > f64::from(fixtures.step_x), "the walk carried on over the step");
        assert_eq!(body.feet()[2], 2.0, "and stayed on its own top");
    }

    #[test]
    fn terrain_that_cannot_be_climbed_stops_the_walk_and_the_body_keeps_its_place() {
        // A wall taller than `STEP_UP_M`: the body stops at it and stays stopped, however long it pushes.
        let fixtures = Fixture::default();
        let mut body = Body::standing([6.0, 2.0, 2.0]);
        let mut blocked = false;
        for _ in 0..120 {
            let step = body.step(1.0 / 30.0, &fixtures, &EAST);
            blocked |= step.blocked;
        }
        assert!(blocked, "the wall refused a move");
        let stopped_at = body.feet()[0];
        assert!(
            stopped_at < f64::from(fixtures.wall_x),
            "the body never reached the wall's own column: {stopped_at}"
        );
        assert!(stopped_at < 6.0 + 20.0 * WALK_M_PER_S / 30.0, "it did not pass the wall");
        step_n(&mut body, &fixtures, &EAST, 30);
        assert_eq!(body.feet()[0], stopped_at, "and pushing on changes nothing");
        assert_eq!(body.feet()[2], 2.0, "it is still standing on the floor it stood on");
    }

    #[test]
    fn a_jump_leaves_the_ground_and_gravity_brings_the_body_back() {
        let fixtures = Fixture::default();
        let mut body = Body::standing([2.0, 2.0, 1.0]);
        let jump = Move { along: [0.0; 3], jump: true };
        let step = body.step(1.0 / 30.0, &fixtures, &jump);
        assert!(step.jumped, "the jump was taken");
        assert!(step.rose_m > 0.0, "the body rose: {} m", step.rose_m);
        let mut highest = body.feet()[2];
        let mut landed = false;
        for _ in 0..120 {
            let step = body.step(1.0 / 30.0, &fixtures, &Move::STILL);
            highest = highest.max(body.feet()[2]);
            if step.grounded {
                landed = true;
                break;
            }
        }
        assert!(landed, "gravity brought the body back down");
        assert!(highest > 1.0, "and it was actually in the air: {highest}");
        assert_eq!(body.feet()[2], 1.0);
    }

    #[test]
    fn walking_off_a_ledge_ends_the_grounding_and_the_body_falls() {
        // The floor exists only west of `step_x`: a body that walks off its own edge falls.
        struct Ledge;
        impl Solidity for Ledge {
            fn is_solid(&self, at: [i32; 3]) -> bool {
                at[2] < 0 && at[0] < 4
            }
        }
        let mut body = Body::standing([3.0, 0.0, 0.0]);
        let mut fell = 0.0;
        for _ in 0..30 {
            let step = body.step(1.0 / 30.0, &Ledge, &EAST);
            fell += step.fell_m;
        }
        assert!(fell > 0.0, "the body left the ledge and fell: {fell} m");
        assert!(body.feet()[0] > 4.0, "the body walked off the ledge's own edge");
        assert!(
            body.feet()[0] < 3.0 + 30.0 * WALK_M_PER_S / 30.0 + 0.01,
            "it walked further in thirty frames than the walk's own speed allows: {}",
            body.feet()[0]
        );
        assert!(!body.grounded(), "and it is not standing on anything");
    }

    #[test]
    fn flight_is_an_explicit_option_that_ignores_gravity_and_terrain() {
        // The same body that is stopped by a wall walks through it when it is flying, and does not fall.
        let fixtures = Fixture::default();
        let mut flying = Body::standing([6.0, 2.0, 2.0]);
        flying.traverse(Traversal::Flying);
        assert_eq!(flying.traversal(), Traversal::Flying);
        let before = flying.feet();
        step_n(&mut flying, &fixtures, &EAST, 60);
        assert!(flying.feet()[0] > f64::from(fixtures.wall_x), "flight passes through the wall");
        assert_eq!(flying.feet()[2], before[2], "and it does not fall");
        assert!(!flying.grounded(), "a flying eye is not standing on anything");
        // And taking up flight drops a fall's own speed rather than carrying it into a hover.
        let mut walker = Body::standing([6.0, 2.0, 20.0]);
        step_n(&mut walker, &fixtures, &Move::STILL, 5);
        assert!(walker.velocity()[2] < 0.0, "a walker in the air is falling");
        walker.traverse(Traversal::Flying);
        assert_eq!(walker.velocity(), [0.0; 3], "and flight starts at rest");
    }

    #[test]
    fn stepping_back_down_a_step_lands_on_the_lower_floor() {
        // The other half of a climb: the same move the other way is not a fall out of the world.
        let fixtures = Fixture::default();
        let mut body = Body::standing([6.0, 2.0, 2.0]);
        let west = Move { along: [-1.0, 0.0, 0.0], jump: false };
        let mut fell = 0.0;
        for _ in 0..60 {
            let step = body.step(1.0 / 30.0, &fixtures, &west);
            fell += step.fell_m;
        }
        assert!(body.feet()[0] < f64::from(fixtures.step_x), "the body stepped down west");
        assert!(fell > 0.0, "and it fell the step's own height: {fell} m");
        assert_eq!(body.feet()[2], 1.0, "onto the lower floor's own top");
    }

    #[test]
    fn a_step_with_no_time_in_it_moves_nothing_and_the_body_keeps_its_own_answer() {
        let fixtures = Fixture::default();
        let mut body = Body::standing([2.0, 2.0, 6.0]);
        for dt in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            let step = body.step(dt, &fixtures, &EAST);
            assert!(step.was_still(), "a step of {dt} s moves nothing");
        }
        assert_eq!(body.feet(), [2.0, 2.0, 6.0], "and the body is where it was");
    }
}
