//! The player's body: a colliding cylinder, moved by **the Squake model**.
//!
//! # What replaced what
//!
//! This file used to hold this repository's own movement: a drag-plus-acceleration
//! walk with `GROUND_ACCEL == GROUND_FRICTION`. It now holds the Quake-style
//! movement of the reference `polina4096/resquake` at commit `1169c96` (Unlicense),
//! read as reference and re-implemented from its mechanism — no code copied, and no
//! number trusted to memory (C-48/C-49). The model is: build a wish direction from
//! the input, multiply it by a base speed, apply ground friction as a
//! multiplication, then a Quake dot-product acceleration (clamped so the wish speed
//! is *exactly* reached), an air acceleration capped on the **projection** of the
//! velocity (the thing that makes speed gain in the air possible at all), gravity
//! last, and a jump that only exists while the key is held.
//!
//! # One tick of the reference is one step here
//!
//! The reference integrates on Minecraft's 20 Hz tick, and its constants are quoted
//! in that tick: `0.05` seconds, "blocks per tick" speeds, and per-tick deltas. This
//! world's clock is already 20 Hz (`clock::TICK_HZ`), so this file runs **one
//! reference tick per step** rather than re-deriving the model at another rate. That
//! is a decision and not a convenience: the reference's ground pair (friction
//! `× 0.6` per tick against an acceleration that saturates) has its equilibrium at
//! exactly the wish speed *because* of the discrete coupling, and re-scaling both to
//! 60 Hz re-derives them to a different equilibrium — measured, not feared:
//! `× slip^(1/3)` against a `dt`-scaled acceleration settles at 1.77× the wish speed.
//! So: one step is 1/20 s, and speeds are quoted from the reference in blocks/tick
//! and converted at the boundary (1 block/tick = 20 m/s).
//!
//! # What is kept from the old file, and why
//!
//! The **collision** is this repository's, unchanged in spirit: the body is a
//! cylinder, the sweep is split into substeps no longer than one voxel
//! ([`SUBSTEP_M`]), the floor and ceiling probes are bounded, and the step-up retry
//! is the ruled 0.55 m. The reference collides with Minecraft's AABB; an AABB catches
//! on stair corners this cylinder does not, and the no-tunnelling argument
//! ([`tests::no_speed_gets_the_body_through_a_one_voxel_wall`]) is stated for this
//! collider, so the collider stays.
//!
//! # The mechanics, on the record
//!
//! * **Bunnyhopping** — the jump key held on the ground means `onGroundForReal` is
//!   false, so the tick's friction is skipped entirely and the air rules run instead.
//!   With the reference's jump cooldown disabled, a held key re-launches the body on
//!   every landing. This is the mechanic, not a bug: the old file's
//!   "a held jump launches one jump" rule is *replaced* by the model.
//! * **Trimping stays enabled** (`TRIMPING_ENABLED`), the user's standing ruling:
//!   *"definitely keep trimping enabled, rat king was added to deadlock and he's got
//!   trimping"* (2026-10-04). Jump while sneaking with more than the base speed and
//!   the horizontal is traded for vertical.
//! * **Sharking** — holding jump while touching water, above the reference's speed
//!   floor and with no ceiling, skims the surface with no gravity and never sinks
//!   through it. When the shark conditions are not met the reference falls back to
//!   *vanilla Minecraft swimming*, which this client does not have; the substitute
//!   is the ordinary air/ground rules (and the sea floor still stands), stated here
//!   rather than hidden.
//! * **Speed caps** — the reference's 544-threshold soft and hard caps exist in this
//!   file and are inert at their own values, because its `uncappedBunnyhop` default
//!   (true) disables them; the mechanism and its arithmetic are tested anyway.
//!
//! # One deliberate non-port
//!
//! The reference applies **two** 0.2 blocks/tick horizontal terms on a sprinting
//! jump: vanilla's own forward boost, and a mod term of `+sin(yaw)·0.2 /
//! −cos(yaw)·0.2`, which in Minecraft's yaw convention is that same boost
//! *negated* — they cancel. This world has no vanilla boost to cancel, so the port
//! adds nothing on a sprint jump, which is the reference's observable behaviour.
//!
//! # `f64`, and one path from keys to position
//!
//! Still `f64`, still for the reason above: the world is `i32` voxels out to
//! ±2,000,000 m. [`Player::advance`] is still the only way a wish becomes a position,
//! and it still banks the frame delta and spends it in whole steps, so the
//! trajectory is a function of how many steps have run. What is **new** is that the
//! drawn eye no longer has to be the last step's: [`Player::interpolated_eye`]
//! renders between the two most recent steps, so 20 Hz physics does not look like
//! 20 Hz stutter — and never moves the body.

/// The rate the player's motion is integrated at, in steps per second.
///
/// **The reference's tick, and this world's clock.** The Squake model is defined on
/// Minecraft's 20 Hz tick; `crate::clock::TICK_HZ` is the same 20, and a test pins
/// the two together so the player can never silently move to a different rate than
/// the world.
pub const PLAYER_HZ: f64 = 20.0;
/// One player step, in seconds — one tick of the reference.
pub const PLAYER_STEP: f64 = 1.0 / PLAYER_HZ;

/// The largest frame delta the player's motion is banked over.
///
/// **Kept from the previous file** (spec `input.rs::MAX_DT`). A frame that stalls for
/// half a second cannot be integrated in one step. Clamped rather than dropped, so a
/// stall costs the player time instead.
pub const MAX_DT: f64 = 0.25;

/// Top walking speed, in metres per second.
///
/// **The reference's base speed**: Minecraft's 0.21585 blocks/tick × 20, which is
/// 4.317 m/s and is also the spec's chosen walk. The reference arrives at the same
/// number by `movementSpeed × 2.15`.
pub const WALK_SPEED: f64 = 4.317;

/// What sprinting multiplies [`WALK_SPEED`] by.
///
/// **The reference's sprint**, which is Minecraft's +30 % attribute boost read
/// through the same `× 2.15`: 0.13 × 2.15 blocks/tick = 5.612 m/s. The previous
/// file's 1.6 was the spec's "sustainable ratio"; the model being ported has 1.3.
pub const SPRINT_MULTIPLIER: f64 = 1.3;

/// The speed a sneaking body walks at, in metres per second.
///
/// **The reference's sneak**: `movementSpeed × 1.10` = 0.11 blocks/tick = 2.2 m/s.
/// Not vanilla Minecraft's 0.03 blocks/tick — resquake overrides the sneak speed,
/// and the model is the reference's.
pub const SNEAK_SPEED: f64 = 2.2;

/// Upward speed a jump starts with, in metres per second.
///
/// **The reference's jump**, which is vanilla Minecraft's 0.42 blocks/tick × 20.
pub const JUMP_SPEED: f64 = 8.4;

/// Downward velocity one step adds, in metres per second per tick.
///
/// **The reference's gravity**: `−0.08` blocks/tick per tick. It is applied *every*
/// step, on the ground as well as in the air, exactly as the reference does — the
/// collision cancels it on the next step, and a standing body's stored vertical
/// speed is therefore `GRAVITY_PER_TICK * AIR_RESISTANCE` and not zero. That is
/// measured in the tests rather than commented away.
pub const GRAVITY_PER_TICK: f64 = -1.6;

/// What the vertical velocity is multiplied by each step.
///
/// **The reference's air resistance**, verbatim: `0.9800000190734863` — Minecraft's
/// `0.98f` widened to `f64` through resquake's own literal.
pub const AIR_RESISTANCE: f64 = 0.980_000_019_073_486_3;

/// The ground's slipperiness, per step.
///
/// **The reference's default block**: Minecraft's 0.6. Friction here is
/// `velocity *= slipperiness` — a multiplication, not a subtraction, and it is the
/// reference's whole ground friction.
pub const SLIPPERINESS: f64 = 0.6;

/// The reference's ground acceleration.
///
/// `accelSpeed = ACCELERATION * wishspeed / slipperiness * PLAYER_STEP`, clamped to
/// the speed still missing. With the defaults the clamp almost always binds, which
/// is why the ground speed lands on the wish speed *exactly*.
pub const ACCELERATION: f64 = 10.0;

/// The reference's air acceleration — the same 10.
pub const AIR_ACCELERATION: f64 = 10.0;

/// The most the air acceleration may move the velocity's **projection** in one step,
/// in metres per second.
///
/// **The reference's `maxAAccPerTick`**: 0.050 blocks/tick, which is 1.0 m/s. In the
/// air, the wish speed used for the "speed still missing" test is capped at this,
/// while the acceleration itself is computed from the *uncapped* wish speed — that
/// asymmetry is the whole of Quake air control and the reason speed can be gained by
/// strafing in the air.
pub const MAX_AIR_ACCELERATION: f64 = 1.0;

/// The factor that scales the ground acceleration by `1 / slipperiness³`.
///
/// **The reference's constant**, verbatim: `0.16277136`. It is what makes a slick
/// surface accelerate *harder* rather than weaker — the counterpart to the friction
/// that same surface multiplies away.
pub const SLIPPERINESS_ACCEL_FACTOR: f64 = 0.162_771_36;

/// Whether trimping is on. **The user's standing ruling, and it stays on.**
pub const TRIMPING_ENABLED: bool = true;

/// What a trimp multiplies the speed bonus into upward velocity by, and divides the
/// horizontal by.
///
/// **The reference's `trimpMultiplier`**: 1.4.
pub const TRIMP_MULTIPLIER: f64 = 1.4;

/// Whether sharking is on. **The reference's default**, kept.
pub const SHARKING_ENABLED: bool = true;

/// How much horizontal speed is kept per step while sharking.
///
/// **The reference's `sharkingFriction`**: 0.995.
pub const SHARKING_FRICTION: f64 = 0.995;

/// What a downward vertical velocity is multiplied by when a held-jump body is
/// about to enter water.
///
/// **The reference's `sharkingSurfaceTension`**: 0.8.
pub const SHARKING_SURFACE_TENSION: f64 = 0.8;

/// The horizontal speed at or above which sharking replaces sinking, in m/s.
///
/// **The reference's `0.078` blocks/tick**, converted.
pub const SHARK_MIN_SPEED: f64 = 1.56;

/// The horizontal speed above which sharking applies its friction, in m/s.
///
/// **The reference's `0.090` blocks/tick**, converted.
pub const SHARK_FRICTION_SPEED: f64 = 1.8;

/// The horizontal speed above which sharking uses the air acceleration, in m/s.
///
/// **The reference's `0.098` blocks/tick**, converted.
pub const SHARK_AIR_ACCEL_SPEED: f64 = 1.96;

/// The soft speed cap, as a multiple of the base speed.
///
/// **The reference's `softCapThreshold`**: 544.0 — a Quake figure, inert in this
/// model at its own value (`544 × 4.317 ≈ 2349 m/s`) and disabled outright by
/// [`UNCAPPED_BUNNYHOP`]. Kept because the mechanism is part of the model.
pub const SOFT_CAP_THRESHOLD: f64 = 544.0;

/// The hard speed cap, as a multiple of the base speed. **The reference's**, 544.0.
pub const HARD_CAP_THRESHOLD: f64 = 544.0;

/// How much of the speed over the soft cap survives, when the soft cap is on.
/// **The reference's `softCapDegen`**: 0.65.
pub const SOFT_CAP_DEGEN: f64 = 0.65;

/// Whether the caps are disabled, as the reference defaults them.
pub const UNCAPPED_BUNNYHOP: bool = true;

/// How high the eye sits above the player's feet, in metres.
///
/// **Chosen, not derived** (spec `input.rs::EYE_HEIGHT`). 1.62 m is the eye height
/// of a 1.75 m person, and it is one half of a single assumption about how big a
/// person is — the other half being [`BODY_HEIGHT`].
pub const EYE_HEIGHT: f64 = 1.62;

/// Where the eye sits when the body is sneaking, in metres.
///
/// **Minecraft's crouching eye height**: 1.27 m against the standing [`EYE_HEIGHT`]
/// of 1.62 — a 0.35 m drop, and the same number the standing eye came from, so a
/// crouch cannot invent a different person.
///
/// **The collider is deliberately not shrunk** (Minecraft's crouching hitbox is
/// 1.5 m tall): this world has no sub-block crawl spaces, and moving [`BODY_HEIGHT`]
/// would move the sharking ceiling rule and every collision test with it. The eye is
/// what moves.
///
/// The user asked for this after playing C-150: *"the camera doesn't shift downwards
/// when you crouch"*.
pub const SNEAK_EYE_HEIGHT: f64 = 1.27;

/// The body's height, feet to crown, in metres.
///
/// **Chosen, not derived**, and it is a *ruling* rather than the spec's: the spec
/// has an eye height and no body at all. 1.8 m is the standing height of the person
/// [`EYE_HEIGHT`] describes.
pub const BODY_HEIGHT: f64 = 1.8;

/// The body's radius, in metres.
///
/// **Chosen, not derived**, and a ruling: a vertical cylinder rather than a box,
/// because a box catches on the corners of a stair step while walking diagonally
/// along one. 0.3 m is a shoulder's half-width.
pub const BODY_RADIUS: f64 = 0.3;

/// The tallest rise the body walks up without jumping, in metres.
///
/// **Chosen by the user, superseding the spec**, which has no step-up at all. 0.55 m
/// is climbed *smoothly*: the horizontal move is retried at this height and, if it
/// fits, gravity settles the last few centimetres.
///
/// Note the consequence, which is a measurement rather than an opinion: this world's
/// voxels are 1 m cubes, so **0.55 m cannot climb a single block.** A ledge is
/// climbed with a jump, and the step-up is for the sub-block relief of a slope.
pub const STEP_UP: f64 = 0.55;

/// The largest distance one collision substep may move the body, in metres.
///
/// **Derived, and it is a bound rather than a tuning value.** The world's thinnest
/// solid feature is one voxel — 1 m — and the per-substep test is made at the
/// substep's *endpoint*. A substep that cannot cross a voxel therefore cannot pass
/// through one, whatever the speed. This is the number that makes the no-tunnelling
/// claim a theorem instead of a hope.
pub const SUBSTEP_M: f64 = 0.25;

/// How far below the feet the floor probe will look, in voxels.
const FLOOR_PROBE_VOXELS: i32 = 64;

/// How far above the crown the ceiling probe will look, in voxels.
const CEILING_PROBE_VOXELS: i32 = 64;

/// How close to the floor counts as standing on it, in metres.
const GROUND_EPS: f64 = 1e-6;

/// What the player is asking for this step, in intent rather than in keys.
///
/// Named as intent so nothing here knows a `winit` `KeyCode`; the bin is the only
/// translation. `forward` and `strafe` are in [-1, 1] before the diagonal
/// normalisation, which is what stops walking forward-and-right being 41 % faster
/// than walking forward.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Wish {
    /// Along the direction the camera faces: `+1` forward, `-1` back.
    pub forward: f64,
    /// To the camera's right: `+1` right, `-1` left.
    pub strafe: f64,
    /// Held: sneak, at [`SNEAK_SPEED`] — and the key trimping wants.
    pub sneak: bool,
    /// Held: run rather than walk.
    pub sprint: bool,
    /// Held: jump. **Held, not pressed**: the reference's jump has no cooldown, and
    /// holding it is bunnyhopping.
    pub jump: bool,
}

impl Wish {
    /// A wish that asks for nothing, so a still player still has friction.
    pub const STILL: Wish = Wish {
        forward: 0.0,
        strafe: 0.0,
        sneak: false,
        sprint: false,
        jump: false,
    };
}

/// Where the body is and how it is moving.
///
/// The **feet**, not the eye: the eye is derived ([`eye`]) because a stored eye and
/// a stored body are two sources of truth for one fact.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Motion {
    /// The body's feet, in world metres.
    pub feet: [f64; 3],
    /// Metres per second on each axis.
    pub velocity: [f64; 3],
    /// Whether the feet are resting on something this step.
    pub grounded: bool,
}

impl Motion {
    /// A body standing on `feet`, at rest.
    pub fn standing(feet: [f64; 3]) -> Self {
        Motion { feet, velocity: [0.0; 3], grounded: true }
    }
}

/// The standing eye, in world metres: [`EYE_HEIGHT`] above the feet.
///
/// The pose-free spelling, for callers that have a place but not a body (the spawn).
pub fn eye(feet: [f64; 3]) -> [f64; 3] {
    eye_for(feet, false)
}

/// The eye for a pose: [`EYE_HEIGHT`] standing, [`SNEAK_EYE_HEIGHT`] sneaking.
pub fn eye_for(feet: [f64; 3], sneaking: bool) -> [f64; 3] {
    let height = if sneaking { SNEAK_EYE_HEIGHT } else { EYE_HEIGHT };
    [feet[0], feet[1] + height, feet[2]]
}

/// Whether a voxel blocks movement, and whether it is water.
///
/// A trait rather than a direct call into the world so this file's tests can stand a
/// floor and a wall in a few lines and measure the movement without a generator, a
/// GPU or a window — and so the one production implementation is the only place the
/// world is asked.
pub trait Solidity {
    /// Whether the voxel at this integer coordinate blocks movement.
    fn is_solid(&self, x: i32, y: i32, z: i32) -> bool;
    /// Whether the voxel at this integer coordinate is water.
    ///
    /// Defaults to `false` so a test double that has no water does not have to say
    /// so; the client's own adapter answers from the live record. Water is not solid
    /// (a body sinks through it), which is why this is a second question and not a
    /// material of [`Solidity::is_solid`].
    fn is_water(&self, _x: i32, _y: i32, _z: i32) -> bool {
        false
    }
}

/// The player's motion, with the accumulator that makes it frame-rate independent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Player {
    /// Where the body is.
    pub motion: Motion,
    /// The body as it was before the most recent step.
    ///
    /// The other end of the interval [`Player::interpolated_eye`] draws between:
    /// physics runs at 20 Hz and frames do not, so the drawn eye is a position
    /// *between two steps* rather than the last one. Presentation only — nothing
    /// reads this back into the motion.
    pub previous: Motion,
    /// Whether the most recent **step** ran with the sneak key held.
    ///
    /// Part of the pose (it is what [`SNEAK_EYE_HEIGHT`] hangs on), and updated only
    /// when a step runs — so the drawn eye follows the body's tick and not the frame
    /// rate, exactly as the position does.
    pub sneaking: bool,
    /// Whether the step before that one did: the other end of the drawn eye's crouch.
    pub previous_sneaking: bool,
    /// Unspent seconds, banked from frames that arrived faster than [`PLAYER_STEP`].
    pub banked_s: f64,
}

impl Player {
    /// A player whose body is standing on `feet`.
    pub fn new(feet: [f64; 3]) -> Self {
        let motion = Motion::standing(feet);
        Player {
            motion,
            previous: motion,
            sneaking: false,
            previous_sneaking: false,
            banked_s: 0.0,
        }
    }

    /// The eye at the **last step**, in world metres — the physics number, in the
    /// pose that step ran with.
    pub fn eye(&self) -> [f64; 3] {
        eye_for(self.motion.feet, self.sneaking)
    }

    /// The eye the frame is **drawn** at: between the two most recent steps.
    ///
    /// The bank fires a step once half a step has arrived (rounding to nearest), so
    /// the newest state is on average half a step ahead of the wall clock; adding
    /// that half step puts the drawn eye on the wall clock without ever drawing past
    /// the newest step, which is why this interpolates and does not extrapolate. No
    /// step has run yet → both ends are the spawn, and the answer is the spawn.
    pub fn interpolated_eye(&self) -> [f64; 3] {
        let alpha = (self.banked_s / PLAYER_STEP + 0.5).clamp(0.0, 1.0);
        let mut feet = [0.0; 3];
        for (axis, out) in feet.iter_mut().enumerate() {
            let (from, to) = (self.previous.feet[axis], self.motion.feet[axis]);
            *out = from + (to - from) * alpha;
        }
        // The **pose's height is interpolated too**, so a crouch eases over a step
        // rather than snapping the drawn eye down 0.35 m in one frame. The physics
        // eye is still the step's own pose; this is the drawn one.
        let from = if self.previous_sneaking { SNEAK_EYE_HEIGHT } else { EYE_HEIGHT };
        let to = if self.sneaking { SNEAK_EYE_HEIGHT } else { EYE_HEIGHT };
        [feet[0], feet[1] + from + (to - from) * alpha, feet[2]]
    }

    /// Bank a frame's delta and spend it in whole [`PLAYER_STEP`]s.
    ///
    /// A non-finite or non-positive delta banks nothing: a frame that reports
    /// negative elapsed time is a bug, and a clamp would turn it into plausible
    /// movement. The bank is bounded by [`MAX_DT`] twice over — the incoming delta
    /// and the bank — so a stalled frame costs the player time rather than queueing a
    /// burst of catch-up steps.
    ///
    /// Steps are **rounded to the nearest** whole step rather than truncated to one:
    /// truncating makes 144 frames of `1/144 s` sum to a hair under a second and fire
    /// fewer steps than 60 frames of `1/60 s`, so a second of walking would differ by
    /// one step's travel purely because of rounding. The leftover carries.
    pub fn advance(&mut self, wish: &Wish, yaw_rad: f64, dt_s: f64, world: &impl Solidity) {
        if !dt_s.is_finite() || dt_s <= 0.0 {
            return;
        }
        self.banked_s = (self.banked_s + dt_s.min(MAX_DT)).min(MAX_DT);
        while self.banked_s + PLAYER_STEP * 0.5 >= PLAYER_STEP {
            self.banked_s -= PLAYER_STEP;
            self.previous = self.motion;
            self.previous_sneaking = self.sneaking;
            self.sneaking = wish.sneak;
            step(&mut self.motion, wish, yaw_rad, world);
        }
    }
}

/// Advance one body by exactly one reference tick.
///
/// The only function in this file that moves anything. The order is the reference's:
/// the wish direction first, the jump, then water, then ground-or-air, then the move
/// — and **gravity last**, applied even when grounded.
pub fn step(m: &mut Motion, wish: &Wish, yaw_rad: f64, world: &impl Solidity) {
    let (wish_x, wish_z) = wish_direction(wish, yaw_rad);
    let moving = wish.forward != 0.0 || wish.strafe != 0.0;
    let wishspeed = if moving { base_speed(wish) } else { 0.0 };
    // The jump key *held* is not the ground, even while the body is standing on it:
    // this is the whole friction skip that makes bunnyhopping work.
    let on_ground_for_real = m.grounded && !wish.jump;

    if wish.jump && m.grounded {
        jump(m, wish);
    }

    let sharking = SHARKING_ENABLED
        && disc_in_water(world, m.feet[0], m.feet[2], m.feet[1], m.feet[1] + BODY_HEIGHT)
        && wish.jump
        && ceiling_over(world, m.feet[0], m.feet[2], m.feet[1] + BODY_HEIGHT).is_none()
        && horizontal_speed(m.velocity) >= SHARK_MIN_SPEED;
    if sharking {
        // The reference's shark branch does its own move and skips gravity; a body
        // at the surface is never pulled under it.
        shark(m, wishspeed, wish_x, wish_z, world);
        return;
    }

    if on_ground_for_real {
        // Friction is a multiplication by the surface's slipperiness, and it runs
        // whether or not the player is asking to move.
        let slip = SLIPPERINESS;
        m.velocity[0] *= slip;
        m.velocity[2] *= slip;
        if wishspeed != 0.0 {
            let acceleration = ACCELERATION * (SLIPPERINESS_ACCEL_FACTOR / (slip * slip * slip));
            accelerate(m, wishspeed, wish_x, wish_z, acceleration, slip);
        }
    } else {
        air_accelerate(m, wishspeed, wish_x, wish_z, AIR_ACCELERATION);
        // Surface tension: a held-jump body falling *into* water is slowed before it
        // enters, which is what makes sharking an entry and not an accident.
        if SHARKING_SURFACE_TENSION > 0.0
            && wish.jump
            && m.velocity[1] < 0.0
            && falling_into_water(world, m)
        {
            m.velocity[1] *= SHARKING_SURFACE_TENSION;
        }
    }

    move_body(m, world);
    gravity(m);
}

/// The unit wish direction in world space, from the input and the yaw.
///
/// The reference normalises the input so a diagonal is not faster; below unit length
/// it is scaled **up** to the unit circle (its `if speed < 1.0 { 1.0 }`), which is
/// what makes a slight diagonal a full-speed diagonal.
fn wish_direction(wish: &Wish, yaw_rad: f64) -> (f64, f64) {
    let length_sq = wish.forward * wish.forward + wish.strafe * wish.strafe;
    let scale = if length_sq <= 0.0 {
        1.0
    } else {
        let length = length_sq.sqrt();
        if length < 1.0 {
            1.0
        } else {
            1.0 / length
        }
    };
    let forward = wish.forward * scale;
    let strafe = wish.strafe * scale;
    let (sin_yaw, cos_yaw) = yaw_rad.sin_cos();
    (forward * sin_yaw + strafe * cos_yaw, forward * cos_yaw - strafe * sin_yaw)
}

/// The speed the wish is asking for, in m/s.
fn base_speed(wish: &Wish) -> f64 {
    if wish.sneak {
        SNEAK_SPEED
    } else if wish.sprint {
        WALK_SPEED * SPRINT_MULTIPLIER
    } else {
        WALK_SPEED
    }
}

/// The speed trimping measures excess against, and the caps scale: **sprint-aware,
/// sneak-blind** — exactly the reference's `getBaseSpeedMax`.
fn base_max_speed(wish: &Wish) -> f64 {
    if wish.sprint {
        WALK_SPEED * SPRINT_MULTIPLIER
    } else {
        WALK_SPEED
    }
}

/// Horizontal speed, in m/s.
fn horizontal_speed(velocity: [f64; 3]) -> f64 {
    (velocity[0] * velocity[0] + velocity[2] * velocity[2]).sqrt()
}

/// The jump, and everything the reference hangs off it.
///
/// **No sprint-jump boost is added, and that is the port of two 0.2 blocks/tick
/// terms that cancel in the reference** (see the module header). What remains is the
/// vertical launch, the soft cap, trimping, and the hard cap.
fn jump(m: &mut Motion, wish: &Wish) {
    m.velocity[1] = JUMP_SPEED;
    let speed = horizontal_speed(m.velocity);
    let max_base = base_max_speed(wish);
    apply_soft_cap(m, max_base, speed);
    let trimmed = try_trimp(m, wish, max_base);
    if !trimmed {
        apply_hard_cap(m, max_base);
    }
}

/// Trade horizontal speed for vertical, when a sneaking body jumps faster than its
/// base speed allows.
///
/// **The reference's trimp, on the user's ruling that it stays enabled.** The bonus
/// saturates at twice the base speed (`speed / base * 0.5` clamped to 1), the
/// horizontal is divided by [`TRIMP_MULTIPLIER`], and the vertical is *set* — not
/// added — to `bonus * speed * multiplier`, which is why a trimp overrides the
/// jump's 8.4 m/s rather than stacking on it.
fn try_trimp(m: &mut Motion, wish: &Wish, max_base_speed: f64) -> bool {
    if !TRIMPING_ENABLED || !wish.sneak {
        return false;
    }
    let speed = horizontal_speed(m.velocity);
    if speed <= max_base_speed {
        return false;
    }
    let bonus = (speed / max_base_speed * 0.5).min(1.0);
    if TRIMP_MULTIPLIER > 0.0 {
        let inverse = 1.0 / TRIMP_MULTIPLIER;
        m.velocity[0] *= inverse;
        m.velocity[2] *= inverse;
        m.velocity[1] = bonus * speed * TRIMP_MULTIPLIER;
    }
    true
}

/// The multiplier the reference's soft cap applies to a speed over the soft cap.
///
/// Separated from the motion so the arithmetic is testable: over the cap the excess
/// is degenerated by `degen`, and a `degen` of exactly 1 is no cap at all.
fn degenerated_speed(speed: f64, cap: f64, degen: f64) -> f64 {
    if speed > cap && degen != 1.0 {
        ((speed - cap) * degen + cap) / speed
    } else {
        1.0
    }
}

/// The reference's soft cap, applied on a jump. Inert while [`UNCAPPED_BUNNYHOP`].
fn apply_soft_cap(m: &mut Motion, base_speed: f64, speed: f64) {
    let (percent, degen) = if UNCAPPED_BUNNYHOP {
        (1.0, 1.0)
    } else {
        (SOFT_CAP_THRESHOLD, SOFT_CAP_DEGEN)
    };
    let multiplier = degenerated_speed(speed, base_speed * percent, degen);
    m.velocity[0] *= multiplier;
    m.velocity[2] *= multiplier;
}

/// The reference's hard cap, applied when a jump was **not** a trimp.
fn apply_hard_cap(m: &mut Motion, base_speed: f64) {
    let cap = if UNCAPPED_BUNNYHOP { f64::INFINITY } else { base_speed * HARD_CAP_THRESHOLD };
    let speed = horizontal_speed(m.velocity);
    if speed > cap && cap != 0.0 {
        let multiplier = cap / speed;
        m.velocity[0] *= multiplier;
        m.velocity[2] *= multiplier;
    }
}

/// The reference's ground acceleration: a dot-product seek of the wish speed.
///
/// `accelSpeed` is computed from the wish speed and the slipperiness and then
/// **clamped to the speed still missing**, so the seek cannot overshoot: at the
/// reference's defaults the clamp usually binds and the walk speed is reached in one
/// or two steps, exactly.
fn accelerate(
    m: &mut Motion,
    wishspeed: f64,
    wish_x: f64,
    wish_z: f64,
    acceleration: f64,
    slipperiness: f64,
) {
    let current = m.velocity[0] * wish_x + m.velocity[2] * wish_z;
    let add_speed = wishspeed - current;
    if add_speed <= 0.0 {
        return;
    }
    let accel_speed = (acceleration * wishspeed / slipperiness * PLAYER_STEP).min(add_speed);
    m.velocity[0] += accel_speed * wish_x;
    m.velocity[2] += accel_speed * wish_z;
}

/// The reference's air acceleration, and the reason strafe-jumping gains speed.
///
/// The wish speed used to test whether anything is missing is capped at
/// [`MAX_AIR_ACCELERATION`] — a body already moving faster than the cap in the wish
/// direction gains nothing — while `accelSpeed` is computed from the **uncapped**
/// wish speed. The cap binds, so each step moves the velocity's projection toward the
/// cap by at most the amount it is short of it.
fn air_accelerate(m: &mut Motion, wishspeed_initial: f64, wish_x: f64, wish_z: f64, accel: f64) {
    let wishspeed = wishspeed_initial.min(MAX_AIR_ACCELERATION);
    let current = m.velocity[0] * wish_x + m.velocity[2] * wish_z;
    let add_speed = wishspeed - current;
    if add_speed <= 0.0 {
        return;
    }
    let accel_speed = (accel * wishspeed_initial * PLAYER_STEP).min(add_speed);
    m.velocity[0] += accel_speed * wish_x;
    m.velocity[2] += accel_speed * wish_z;
}

/// The reference's shark: friction, acceleration, its own move, and **never down**.
fn shark(m: &mut Motion, wishspeed: f64, wish_x: f64, wish_z: f64, world: &impl Solidity) {
    let speed = horizontal_speed(m.velocity);
    if speed > SHARK_FRICTION_SPEED {
        m.velocity[0] *= SHARKING_FRICTION;
        m.velocity[2] *= SHARKING_FRICTION;
    }
    if speed > SHARK_AIR_ACCEL_SPEED {
        air_accelerate(m, wishspeed, wish_x, wish_z, AIR_ACCELERATION);
    } else {
        accelerate(m, wishspeed, wish_x, wish_z, ACCELERATION, 1.0);
    }
    move_body(m, world);
    if m.velocity[1] < 0.0 {
        m.velocity[1] = 0.0;
    }
}

/// Integrate the velocity through the collision, in substeps no longer than
/// [`SUBSTEP_M`] — which is what makes tunnelling impossible rather than unlikely.
fn move_body(m: &mut Motion, world: &impl Solidity) {
    let travel = m.velocity[0]
        .abs()
        .max(m.velocity[1].abs())
        .max(m.velocity[2].abs())
        * PLAYER_STEP;
    let substeps = (travel / SUBSTEP_M).ceil().max(1.0) as usize;
    let sub_dt = PLAYER_STEP / substeps as f64;
    for _ in 0..substeps {
        // Horizontal first, then vertical, so that a body which walks into a wall
        // is stopped by the wall rather than being lifted over it by the floor.
        move_horizontal(m, 0, m.velocity[0] * sub_dt, world);
        move_horizontal(m, 2, m.velocity[2] * sub_dt, world);
        move_vertical(m, m.velocity[1] * sub_dt, world);
    }
}

/// The reference's gravity, and the last thing a step does.
///
/// Applied on the ground as well as in the air, exactly as the reference applies it:
/// a standing body's vertical speed after a step is `GRAVITY_PER_TICK *
/// AIR_RESISTANCE`, and the next step's collision cancels it. Moving it before the
/// move would change the jump's discrete arc, so it is not moved.
fn gravity(m: &mut Motion) {
    m.velocity[1] = (m.velocity[1] + GRAVITY_PER_TICK) * AIR_RESISTANCE;
}

/// Move on one horizontal axis, with the step-up retry.
fn move_horizontal(m: &mut Motion, axis: usize, amount: f64, world: &impl Solidity) {
    if amount == 0.0 {
        return;
    }
    let feet = m.feet;
    let mut flat = feet;
    flat[axis] += amount;
    if !disc_hits(world, flat[0], flat[2], flat[1], flat[1] + BODY_HEIGHT) {
        m.feet = flat;
        // Walking off a ledge ends the grounding: the floor under the new position
        // is no longer under the feet.
        if let Some(floor) = floor_under(world, m.feet[0], m.feet[2], m.feet[1]) {
            m.grounded = m.grounded && (m.feet[1] - floor).abs() < GROUND_EPS;
        } else {
            m.grounded = false;
        }
        return;
    }

    // Blocked at this height. Retry standing [`STEP_UP`] higher: if the body fits
    // there, the move is taken *at the raised height* and gravity settles the last
    // few centimetres. That is what makes the step smooth — the alternative, moving
    // the body to the top of the ledge outright, is a teleport.
    let mut raised = flat;
    raised[1] = feet[1] + STEP_UP;
    if !disc_hits(world, raised[0], raised[2], raised[1], raised[1] + BODY_HEIGHT) {
        m.feet = raised;
        m.grounded = false;
        m.velocity[1] = 0.0;
        return;
    }

    // Blocked outright: stay put and stop pushing into it, so the body does not
    // accumulate velocity against a wall it cannot pass.
    m.velocity[axis] = 0.0;
}

/// Move vertically, against the floor below and the ceiling above.
fn move_vertical(m: &mut Motion, amount: f64, world: &impl Solidity) {
    let [x, _, z] = m.feet;
    let target = m.feet[1] + amount;
    if amount <= 0.0 {
        match floor_under(world, x, z, target) {
            Some(floor) if target <= floor => {
                m.feet[1] = floor;
                m.velocity[1] = 0.0;
                m.grounded = true;
            }
            _ => {
                m.feet[1] = target;
                m.grounded = false;
            }
        }
    } else if let Some(ceiling) = ceiling_over(world, x, z, target + BODY_HEIGHT) {
        let highest = ceiling - BODY_HEIGHT;
        if target >= highest {
            m.feet[1] = highest;
            // Left as zero rather than reversed: the body is against a ceiling and
            // gravity takes it from here, which is the same rule a jump has.
            m.velocity[1] = 0.0;
        } else {
            m.feet[1] = target;
        }
        m.grounded = false;
    } else {
        m.feet[1] = target;
        // **A body that moved up is not on the ground**, and the reference's
        // `isOnGround` is recomputed by its collision every tick for exactly this
        // reason: a jump that left the flag set would let the next tick's ground
        // friction bite in mid-air.
        m.grounded = false;
    }
}

/// Whether the body's disc touches any voxel the predicate accepts.
///
/// The test is exact for a vertical cylinder against axis-aligned boxes: the
/// closest point on each candidate voxel's footprint, compared against
/// [`BODY_RADIUS`]. Touching (`==`) is *not* an overlap, so a body resting exactly
/// on a surface is not reported as inside it.
fn disc_any<W: Solidity>(
    world: &W,
    x: f64,
    z: f64,
    y_lo: f64,
    y_hi: f64,
    accepts: impl Fn(&W, i32, i32, i32) -> bool,
) -> bool {
    if y_hi <= y_lo {
        return false;
    }
    let x0 = (x - BODY_RADIUS).floor() as i32;
    let x1 = (x + BODY_RADIUS).floor() as i32;
    let z0 = (z - BODY_RADIUS).floor() as i32;
    let z1 = (z + BODY_RADIUS).floor() as i32;
    let y0 = y_lo.floor() as i32;
    // `ceil` of a hair less than the top, so a span ending exactly on a voxel
    // boundary does not drag in the voxel above it.
    let y1 = (y_hi - f64::EPSILON).ceil() as i32;
    for ix in x0..=x1 {
        for iz in z0..=z1 {
            let dx = axis_gap(x, ix as f64);
            let dz = axis_gap(z, iz as f64);
            if dx * dx + dz * dz >= BODY_RADIUS * BODY_RADIUS {
                continue;
            }
            for iy in y0..y1 {
                let top = iy as f64;
                if top < y_hi && top + 1.0 > y_lo && accepts(world, ix, iy, iz) {
                    return true;
                }
            }
        }
    }
    false
}

/// Whether the body's disc overlaps any solid voxel between `y_lo` and `y_hi`.
fn disc_hits(world: &impl Solidity, x: f64, z: f64, y_lo: f64, y_hi: f64) -> bool {
    disc_any(world, x, z, y_lo, y_hi, |w, x, y, z| w.is_solid(x, y, z))
}

/// Whether the body's disc touches any water voxel between `y_lo` and `y_hi`.
fn disc_in_water(world: &impl Solidity, x: f64, z: f64, y_lo: f64, y_hi: f64) -> bool {
    disc_any(world, x, z, y_lo, y_hi, |w, x, y, z| w.is_water(x, y, z))
}

/// Whether the body's box, **offset by one step's travel**, would enter water — the
/// reference's `containsFluid(boundingBox.offset(velocity))`.
fn falling_into_water(world: &impl Solidity, m: &Motion) -> bool {
    let dx = m.velocity[0] * PLAYER_STEP;
    let dy = m.velocity[1] * PLAYER_STEP;
    let dz = m.velocity[2] * PLAYER_STEP;
    disc_in_water(
        world,
        m.feet[0] + dx,
        m.feet[2] + dz,
        m.feet[1] + dy,
        m.feet[1] + dy + BODY_HEIGHT,
    )
}

/// The distance from `p` to the unit interval starting at `lo`, or zero inside it.
///
/// The two terms are the distance to the interval's **near** edge, so the `max`
/// picks whichever side `p` is on and gives zero when it is between them. Getting
/// the order of these two subtractions backwards reports half a voxel for a point
/// in the middle of a voxel, which is not a small error: it makes every collision
/// test miss, and a body then falls through the world.
fn axis_gap(p: f64, lo: f64) -> f64 {
    (lo - p).max(p - (lo + 1.0)).max(0.0)
}

/// The highest solid surface under the body's disc at or below `from_y`.
///
/// Scans downward from `from_y` and stops at the first solid voxel, so the answer is
/// the nearest floor rather than the deepest one. Bounded at
/// [`FLOOR_PROBE_VOXELS`], which is far deeper than any single step can fall.
fn floor_under(world: &impl Solidity, x: f64, z: f64, from_y: f64) -> Option<f64> {
    let start = from_y.floor() as i32;
    // `.rev()` is load-bearing: the scan must run from the feet *downwards*, so the
    // first solid voxel it meets is the **highest** one. Ascending, it returns the
    // deepest solid voxel in the probe window instead — which is not a floor at all,
    // and leaves a standing body convinced it is in mid-air over a 64 m shaft.
    for iy in (start - FLOOR_PROBE_VOXELS + 1..=start).rev() {
        let top = iy as f64;
        if disc_hits(world, x, z, top, top + 1.0) {
            return Some(top + 1.0);
        }
    }
    None
}

/// The lowest solid underside above the body's crown.
fn ceiling_over(world: &impl Solidity, x: f64, z: f64, from_y: f64) -> Option<f64> {
    let start = from_y.floor() as i32;
    for iy in start..=(start + CEILING_PROBE_VOXELS) {
        let top = iy as f64;
        if disc_hits(world, x, z, top, top + 1.0) {
            return Some(top);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    /// A world that is solid at or below one height, everywhere: a floor and nothing
    /// else. The height is a `Cell` so a test can build a step without rebuilding the
    /// world.
    struct Floor {
        top: Cell<i32>,
    }

    impl Solidity for Floor {
        fn is_solid(&self, _x: i32, y: i32, _z: i32) -> bool {
            y <= self.top.get()
        }
    }

    /// A floor at 0 plus a wall standing at `wall_x` that is **exactly one voxel
    /// thick**, so a body approaching it from the west meets the thinnest solid
    /// feature the world has — and, just as importantly, has somewhere to land if it
    /// clears it.
    ///
    /// The first version of this was `x >= wall_x`, a half-space rather than a
    /// wall, and it made the tunnelling test unfalsifiable: with solid ground
    /// extending to infinity there is nothing beyond the wall to pass *into*, so no
    /// amount of speed can demonstrate the failure the test claims to guard.
    struct Wall {
        wall_x: i32,
    }

    impl Solidity for Wall {
        fn is_solid(&self, x: i32, y: i32, _z: i32) -> bool {
            y <= 0 || (x >= self.wall_x && x < self.wall_x + 1 && y <= 8)
        }
    }

    /// A floor with a range of water voxels above it: the reference's two water
    /// questions ([`Solidity::is_solid`] and [`Solidity::is_water`]) answered from
    /// one world.
    struct Sea {
        floor_top: i32,
        water: (i32, i32),
    }

    impl Solidity for Sea {
        fn is_solid(&self, _x: i32, y: i32, _z: i32) -> bool {
            y <= self.floor_top
        }
        fn is_water(&self, _x: i32, y: i32, _z: i32) -> bool {
            y >= self.water.0 && y <= self.water.1
        }
    }

    fn walk(wish: &Wish, steps: usize, world: &impl Solidity) -> Motion {
        let mut m = Motion::standing([0.5, 1.0, 0.5]);
        for _ in 0..steps {
            step(&mut m, wish, 0.0, world);
        }
        m
    }

    const FORWARD: Wish = Wish { forward: 1.0, ..Wish::STILL };

    /// **The step is the world clock's tick**, so the player can never move at a rate
    /// the world does not run on.
    #[test]
    fn the_step_is_the_world_clocks_tick() {
        // Read as values rather than compared inline: a constant expression in an
        // `assert_eq!` is one the linter is right to call constant, and reading it
        // out is the same claim that cannot be folded away.
        let rate = PLAYER_HZ as u32;
        let step = PLAYER_STEP;
        assert_eq!(rate, crate::clock::TICK_HZ);
        assert_eq!(step, crate::clock::tick_seconds());
    }

    /// **The constants are the reference's own numbers, in its own units** — blocks
    /// per tick — converted by the 20 Hz tick rather than re-invented here.
    #[test]
    fn the_models_constants_are_the_references_own_numbers() {
        // Minecraft's walk is 0.21585 blocks/tick; resquake reaches it as
        // movementSpeed × 2.15. The pair is walked rather than compared inline, so
        // each constant is read as a value and the linter cannot call the assertion
        // constant.
        let numbers = [
            (WALK_SPEED, 0.215_85 * 20.0),
            (SNEAK_SPEED, 0.11 * 20.0),
            (JUMP_SPEED, 0.42 * 20.0),
            (GRAVITY_PER_TICK, -0.08 * 20.0),
            (MAX_AIR_ACCELERATION, 0.050 * 20.0),
            (SHARK_MIN_SPEED, 0.078 * 20.0),
            (SHARK_FRICTION_SPEED, 0.090 * 20.0),
            (SHARK_AIR_ACCEL_SPEED, 0.098 * 20.0),
        ];
        for (from_the_model, from_the_reference) in numbers {
            assert!(
                (from_the_model - from_the_reference).abs() < 1e-12,
                "the model's {from_the_model} is not the reference's {from_the_reference}"
            );
        }
        // Air resistance is Minecraft's 0.98f widened through the reference's own
        // literal; the slipperiness is the reference's default block.
        let widened = AIR_RESISTANCE as f32;
        assert_eq!(widened, 0.98f32);
        let slipperiness = SLIPPERINESS;
        assert_eq!(slipperiness, 0.6);
    }

    #[test]
    fn the_body_rests_on_the_floor_instead_of_falling_through_it() {
        let world = Floor { top: Cell::new(0) };
        let m = walk(&Wish::STILL, 300, &world);
        // Voxel 0's top face is 1.0 m, so 1.0 is standing *on* it, not inside it.
        assert!((m.feet[1] - 1.0).abs() < 1e-9, "feet were {}", m.feet[1]);
        assert!(m.grounded, "the body should be resting, not falling");
        // **Gravity is applied on the ground too**, exactly as the reference's
        // `applyGravity` does, so the stored vertical speed is the per-tick fall and
        // not zero. Measured, because it is the one line that makes a standing body
        // look like a falling one in a HUD.
        let expected = GRAVITY_PER_TICK * AIR_RESISTANCE;
        assert!(
            (m.velocity[1] - expected).abs() < 1e-12,
            "standing vertical speed {} m/s, expected the reference's {expected}",
            m.velocity[1]
        );
    }

    #[test]
    fn the_body_never_ends_a_step_inside_a_solid_voxel() {
        let world = Floor { top: Cell::new(0) };
        let m = walk(&Wish::STILL, 300, &world);
        assert!(
            !disc_hits(&world, m.feet[0], m.feet[2], m.feet[1], m.feet[1] + BODY_HEIGHT),
            "the body came to rest inside the world at {:?}",
            m.feet
        );
    }

    #[test]
    fn the_body_cannot_walk_into_a_solid_wall() {
        let world = Wall { wall_x: 5 };
        let m = walk(&FORWARD, 600, &world);
        // The cylinder's skin stops one radius short of the wall's face at x 5.
        let limit = 5.0 - BODY_RADIUS;
        assert!(m.feet[0] <= limit + 1e-9, "reached x {} past {}", m.feet[0], limit);
    }

    #[test]
    fn no_speed_gets_the_body_through_a_one_voxel_wall() {
        // The tunnelling claim, at three speeds: the old observer's 18 m/s, and
        // two the game has no business reaching.
        //
        // **The start offset is swept**, because a single fixed offset passed with
        // `SUBSTEP_M` set to 5.0 — a substep five times a voxel — from one phase the
        // body happened to land inside the wall. A claim that cannot fail is not a
        // claim. So the wall is approached from every phase of a substep.
        for speed in [18.0_f64, 300.0, 5_000.0] {
            for step_no in 0..40 {
                let start = 0.1 + step_no as f64 * 0.125;
                let world = Wall { wall_x: 5 };
                let mut m = Motion::standing([start, 1.0, 0.5]);
                m.velocity[0] = speed;
                for _ in 0..400 {
                    step(&mut m, &Wish::STILL, 0.0, &world);
                    assert!(
                        m.feet[0] < 5.0,
                        "at {speed} m/s from x {start} the body reached x {}, past the wall at 5",
                        m.feet[0]
                    );
                }
            }
        }
    }

    #[test]
    fn the_walk_speed_is_the_declared_one_and_not_a_multiple_of_it() {
        let world = Floor { top: Cell::new(0) };
        let m = walk(&FORWARD, 300, &world);
        // The *speed*, not one component of it: at yaw 0 a forward wish is +Z, so
        // asserting on `velocity[0]` would be asserting that the player walks
        // sideways, and would read 0.0 for a body moving at exactly the right pace.
        let speed = horizontal_speed(m.velocity);
        assert!(
            (speed - WALK_SPEED).abs() < 1e-12,
            "settled at {speed} m/s, not the declared {WALK_SPEED}",
        );
        // The clamp is what makes it exact: the acceleration cannot spend more than
        // the speed still missing, so the seek lands *on* the wish speed rather than
        // near it — the old file's drag/accel pair settled at `a / k` and this one
        // deliberately does not have that coupling.
        let sprinted = walk(&Wish { sprint: true, ..FORWARD }, 300, &world);
        let sprint_speed = horizontal_speed(sprinted.velocity);
        assert!(
            (sprint_speed - WALK_SPEED * SPRINT_MULTIPLIER).abs() < 1e-12,
            "sprint settled at {sprint_speed} m/s"
        );
    }

    /// The reference's friction is `velocity *= slipperiness` per tick — a
    /// multiplication with no stop-speed floor, so a released key coasts to a
    /// geometric stop rather than arriving at one. Measured rather than described.
    #[test]
    fn a_released_key_costs_exactly_the_slipperiness_each_step() {
        let world = Floor { top: Cell::new(0) };
        let mut m = walk(&FORWARD, 30, &world);
        let before = horizontal_speed(m.velocity);
        for _ in 0..10 {
            step(&mut m, &Wish::STILL, 0.0, &world);
        }
        let after = horizontal_speed(m.velocity);
        let expected = before * SLIPPERINESS.powi(10);
        assert!(
            (after - expected).abs() < 1e-12,
            "after ten released steps {after} m/s, expected {expected}"
        );
        // Half a second of coasting from a walk leaves about 2.6 cm/s — the
        // reference's stop is quick and never exactly zero.
        assert!(after > 0.0 && after < 0.05, "stopped at {after} m/s");
    }

    #[test]
    fn a_diagonal_walk_is_not_faster_than_a_straight_one() {
        let world = Floor { top: Cell::new(0) };
        let diagonal = Wish { forward: 1.0, strafe: 1.0, ..Wish::STILL };
        let a = walk(&FORWARD, 600, &world);
        let b = walk(&diagonal, 600, &world);
        let speed = horizontal_speed;
        assert!(
            (speed(a.velocity) - speed(b.velocity)).abs() < 1e-12,
            "straight {} m/s, diagonal {} m/s",
            speed(a.velocity),
            speed(b.velocity)
        );
    }

    #[test]
    fn the_walk_direction_is_where_the_camera_looks() {
        // Yaw 0 looks down +Z, so forward must move +Z and nothing on X.
        let world = Floor { top: Cell::new(0) };
        let m = walk(&FORWARD, 300, &world);
        assert!(m.feet[2] > 1.0, "did not walk forward: {:?}", m.feet);
        assert!((m.feet[0] - 0.5).abs() < 1e-9, "drifted sideways: {:?}", m.feet);

        // A quarter turn walks +X instead.
        let mut turned = Motion::standing([0.5, 1.0, 0.5]);
        for _ in 0..300 {
            step(&mut turned, &FORWARD, std::f64::consts::FRAC_PI_2, &world);
        }
        assert!(turned.feet[0] > 1.0, "did not walk right: {:?}", turned.feet);
        // Not `1e-9`: `cos(PI/2)` is 6.1e-17, not zero, so a quarter turn carries a
        // residue of forward intent. The claim under test is that the walk is
        // *transverse*, so the tolerance is set against the motion it is
        // perpendicular to — and it is tight enough to catch a sign error.
        assert!(
            (turned.feet[2] - 0.5).abs() < 1e-9,
            "drifted {:.6} m forward while turning right: {:?}",
            (turned.feet[2] - 0.5).abs(),
            turned.feet
        );
    }

    /// Sneaking is the reference's 0.11 blocks/tick, and it is the key trimping
    /// wants — measured here so the trimp tests below have a base to exceed.
    #[test]
    fn sneaking_walks_at_the_references_sneak_speed() {
        let world = Floor { top: Cell::new(0) };
        let m = walk(&Wish { sneak: true, ..FORWARD }, 300, &world);
        let speed = horizontal_speed(m.velocity);
        assert!(
            (speed - SNEAK_SPEED).abs() < 1e-12,
            "sneaked at {speed} m/s, not the declared {SNEAK_SPEED}"
        );
    }

    /// **The jump's discrete arc**, computed from the two constants rather than
    /// asserted in prose: the launch is 0.42 blocks/tick, gravity is applied *after*
    /// the move, and the vertical is damped by 0.98 each tick. The recurrence below
    /// is that arithmetic; the measured apex has to be it.
    #[test]
    fn a_jump_reaches_the_apex_the_reference_arithmetic_describes() {
        let world = Floor { top: Cell::new(0) };
        let mut m = Motion::standing([0.5, 1.0, 0.5]);
        let ground = m.feet[1];
        step(&mut m, &Wish { jump: true, ..Wish::STILL }, 0.0, &world);
        let mut apex = m.feet[1];
        let mut steps_total = 1usize;
        let mut steps_rising = 1usize;
        for _ in 0..600 {
            step(&mut m, &Wish::STILL, 0.0, &world);
            steps_total += 1;
            if m.velocity[1] > 0.0 {
                steps_rising = steps_total;
            }
            apex = apex.max(m.feet[1]);
            if m.grounded {
                break;
            }
        }
        let mut expected_apex = 0.0;
        let mut v = JUMP_SPEED;
        while v > 0.0 {
            expected_apex += v * PLAYER_STEP;
            v = (v + GRAVITY_PER_TICK) * AIR_RESISTANCE;
        }
        assert!(
            (apex - ground - expected_apex).abs() < 1e-9,
            "apex {} m above launch, the recurrence says {expected_apex:.6} (Minecraft's published jump is 1.2522)",
            apex - ground
        );
        // Five steps of rising: the arc is short at 20 Hz, which is the reason the
        // drawn eye interpolates rather than following the last step.
        assert_eq!(steps_rising, 5, "rose for {steps_rising} steps");
        // And it comes back down to exactly where it left.
        assert!((m.feet[1] - ground).abs() < 1e-9, "landed at {}", m.feet[1]);
        assert!(m.grounded, "a jump should end on the ground");
    }

    /// **Holding the jump key re-launches on every landing** — the reference's
    /// bunnyhop, and the deliberate replacement of the old file's "a held key
    /// launches one jump" rule: the reference's jump cooldown is disabled, so the
    /// key held at a landing jumps again on the next step.
    #[test]
    fn holding_the_jump_key_re_launches_on_every_landing() {
        let world = Floor { top: Cell::new(0) };
        let held = Wish { jump: true, ..Wish::STILL };
        let mut p = Player::new([0.5, 1.0, 0.5]);
        let mut launches = 0usize;
        for _ in 0..600 {
            let was_grounded = p.motion.grounded;
            p.advance(&held, 0.0, PLAYER_STEP, &world);
            if was_grounded && p.motion.velocity[1] > JUMP_SPEED * 0.5 {
                launches += 1;
            }
        }
        assert!(
            launches >= 20,
            "a held key launched only {launches} jumps in thirty seconds"
        );
    }

    /// **The friction skip**, which is the whole of why a held jump is fast: one
    /// grounded step with the key held keeps the horizontal speed, and the same step
    /// with it released pays the slipperiness.
    #[test]
    fn a_held_jump_skips_the_ground_friction_a_released_one_keeps() {
        let world = Floor { top: Cell::new(0) };
        let mut held = Motion { feet: [0.5, 1.0, 0.5], velocity: [0.0, 0.0, 8.0], grounded: true };
        step(&mut held, &Wish { jump: true, ..FORWARD }, 0.0, &world);
        assert!(
            (horizontal_speed(held.velocity) - 8.0).abs() < 1e-12,
            "a held jump cost {} m/s of speed",
            8.0 - horizontal_speed(held.velocity)
        );
        let mut released = Motion { feet: [0.5, 1.0, 0.5], velocity: [0.0, 0.0, 8.0], grounded: true };
        step(&mut released, &FORWARD, 0.0, &world);
        assert!(
            (horizontal_speed(released.velocity) - 8.0 * SLIPPERINESS).abs() < 1e-12,
            "a released step kept {} m/s, expected {}",
            horizontal_speed(released.velocity),
            8.0 * SLIPPERINESS
        );
    }

    /// The reference's sprint jump adds **nothing** horizontal: its own 0.2
    /// blocks/tick term cancels vanilla's, and this world has no vanilla term to
    /// bring back. Pinned because it is a deliberate non-port (module header).
    #[test]
    fn a_sprint_jump_adds_no_horizontal_boost() {
        let world = Floor { top: Cell::new(0) };
        let sprint_speed = WALK_SPEED * SPRINT_MULTIPLIER;
        let mut m = Motion {
            feet: [0.5, 1.0, 0.5],
            velocity: [0.0, 0.0, sprint_speed],
            grounded: true,
        };
        let wish = Wish { forward: 1.0, sprint: true, jump: true, ..Wish::STILL };
        step(&mut m, &wish, 0.0, &world);
        assert!(
            (horizontal_speed(m.velocity) - sprint_speed).abs() < 1e-12,
            "the sprint jump changed the horizontal speed to {}",
            horizontal_speed(m.velocity)
        );
        let expected = (JUMP_SPEED + GRAVITY_PER_TICK) * AIR_RESISTANCE;
        assert!((m.velocity[1] - expected).abs() < 1e-12);
    }

    /// **Trimping, on the user's ruling that it stays enabled**: jump while sneaking
    /// with more than the base speed and the horizontal is divided by 1.4 while the
    /// vertical is *set* to `bonus × speed × 1.4` — which beats a plain jump, and
    /// only when both conditions hold.
    #[test]
    fn a_sneaking_jump_over_the_base_speed_trimp_and_it_stays_enabled() {
        let trimping = TRIMPING_ENABLED;
        assert!(trimping, "the user's ruling: rat king trimping stays on");
        let world = Floor { top: Cell::new(0) };
        let speed = 8.0;
        let wish = Wish { forward: 1.0, sneak: true, jump: true, ..Wish::STILL };
        let mut m = Motion { feet: [0.5, 1.0, 0.5], velocity: [0.0, 0.0, speed], grounded: true };
        step(&mut m, &wish, 0.0, &world);
        let bonus = (speed / WALK_SPEED * 0.5).min(1.0);
        let expected_horizontal = speed / TRIMP_MULTIPLIER;
        let expected_vertical = (bonus * speed * TRIMP_MULTIPLIER + GRAVITY_PER_TICK) * AIR_RESISTANCE;
        assert!(
            (horizontal_speed(m.velocity) - expected_horizontal).abs() < 1e-12,
            "horizontal {} m/s, expected {expected_horizontal}",
            horizontal_speed(m.velocity)
        );
        assert!(
            (m.velocity[1] - expected_vertical).abs() < 1e-12,
            "vertical {} m/s, expected {expected_vertical}",
            m.velocity[1]
        );
        assert!(m.velocity[1] > JUMP_SPEED, "a trimp must beat a plain jump");

        // Not sneaking: the same jump is a plain jump, no speed traded.
        let mut plain = Motion { feet: [0.5, 1.0, 0.5], velocity: [0.0, 0.0, speed], grounded: true };
        step(&mut plain, &Wish { forward: 1.0, jump: true, ..Wish::STILL }, 0.0, &world);
        assert!((horizontal_speed(plain.velocity) - speed).abs() < 1e-12);
        let plain_vertical = (JUMP_SPEED + GRAVITY_PER_TICK) * AIR_RESISTANCE;
        assert!((plain.velocity[1] - plain_vertical).abs() < 1e-12);

        // Sneaking but *under* the base speed: no excess to trade.
        let mut slow = Motion { feet: [0.5, 1.0, 0.5], velocity: [0.0, 0.0, 4.0], grounded: true };
        step(&mut slow, &wish, 0.0, &world);
        assert!((slow.velocity[1] - plain_vertical).abs() < 1e-12);
        assert!((horizontal_speed(slow.velocity) - 4.0).abs() < 1e-12);
    }

    /// **The air acceleration's asymmetry**: the "speed missing" test is against a
    /// wish speed capped at 1 m/s, so the velocity's projection saturates there —
    /// and nothing decays it in the air, which is why air speed persists.
    #[test]
    fn the_air_acceleration_caps_the_projection_and_never_decays() {
        let world = Floor { top: Cell::new(-64) };
        let mut m = Motion { feet: [0.5, 40.0, 0.5], velocity: [0.0; 3], grounded: false };
        step(&mut m, &FORWARD, 0.0, &world);
        assert!(
            (horizontal_speed(m.velocity) - MAX_AIR_ACCELERATION).abs() < 1e-12,
            "one air step gave {} m/s, expected the cap {MAX_AIR_ACCELERATION}",
            horizontal_speed(m.velocity)
        );
        for _ in 0..20 {
            step(&mut m, &FORWARD, 0.0, &world);
        }
        assert!(
            (horizontal_speed(m.velocity) - MAX_AIR_ACCELERATION).abs() < 1e-12,
            "twenty more air steps gave {} m/s",
            horizontal_speed(m.velocity)
        );
    }

    /// **Sharking**: holding jump at the surface, faster than the reference's floor
    /// and with no ceiling, skims the water and **never sinks** — while the same body
    /// without the key falls through it.
    #[test]
    fn sharking_holds_a_held_jump_at_the_surface_and_never_sinks() {
        let world = Sea { floor_top: 0, water: (1, 1) };
        let mut shark = Motion { feet: [0.5, 1.0, 0.5], velocity: [0.0, 0.0, 4.0], grounded: false };
        step(&mut shark, &Wish { forward: 1.0, jump: true, ..Wish::STILL }, 0.0, &world);
        assert_eq!(shark.velocity[1], 0.0, "the shark's vertical must be clamped to zero");
        assert!(
            (horizontal_speed(shark.velocity) - 4.0 * SHARKING_FRICTION).abs() < 1e-12,
            "shark friction gave {} m/s",
            horizontal_speed(shark.velocity)
        );
        assert!(
            (shark.feet[1] - 1.0).abs() < 1e-9,
            "the shark left the surface: {}",
            shark.feet[1]
        );
        assert!(shark.velocity[2] > 0.0, "the shark stopped moving forward");

        let mut sinking = Motion { feet: [0.5, 1.0, 0.5], velocity: [0.0, 0.0, 4.0], grounded: false };
        step(&mut sinking, &FORWARD, 0.0, &world);
        assert!(sinking.velocity[1] < 0.0, "a released jump did not sink");
    }

    /// A held-jump body **falling into** water keeps 80 % of its fall for that step —
    /// the reference's surface tension — while gravity still runs, so the number
    /// after the step is the reference's own arithmetic.
    #[test]
    fn a_held_jump_falling_into_water_keeps_eighty_percent_of_its_fall() {
        let world = Sea { floor_top: 0, water: (1, 1) };
        let mut m = Motion { feet: [0.5, 2.3, 0.5], velocity: [0.0, -10.0, 0.0], grounded: false };
        step(&mut m, &Wish { jump: true, ..Wish::STILL }, 0.0, &world);
        let expected = (-10.0 * SHARKING_SURFACE_TENSION + GRAVITY_PER_TICK) * AIR_RESISTANCE;
        let without = (-10.0 + GRAVITY_PER_TICK) * AIR_RESISTANCE;
        assert!(
            (m.velocity[1] - expected).abs() < 1e-12,
            "vertical {} m/s, expected {expected} (without the tension it would be {without})",
            m.velocity[1]
        );
    }

    /// The reference's caps, and their arithmetic, with the measured fact that at the
    /// reference's own defaults they are **inert**: `uncappedBunnyhop` is true, and
    /// even switched on the threshold is 544× the walk speed.
    #[test]
    fn the_reference_caps_are_inert_at_their_own_thresholds() {
        let uncapped = UNCAPPED_BUNNYHOP;
        let quake_figure = SOFT_CAP_THRESHOLD * WALK_SPEED;
        assert!(uncapped, "the reference's default, and this port keeps it");
        assert!(quake_figure > 2_000.0, "the soft cap is a Quake figure");
        let mut m = Motion { feet: [0.5, 1.0, 0.5], velocity: [500.0, 0.0, 500.0], grounded: true };
        let before = m.velocity;
        let over_the_cap = horizontal_speed(m.velocity);
        apply_soft_cap(&mut m, WALK_SPEED, over_the_cap);
        apply_hard_cap(&mut m, WALK_SPEED);
        assert_eq!(m.velocity, before, "the caps changed a 707 m/s velocity");
        // The degeneracy arithmetic itself, off the default path:
        assert!((degenerated_speed(100.0, 50.0, SOFT_CAP_DEGEN) - 0.825).abs() < 1e-12);
        assert_eq!(degenerated_speed(50.0, 50.0, SOFT_CAP_DEGEN), 1.0);
        assert_eq!(degenerated_speed(100.0, 50.0, 1.0), 1.0);
    }

    /// **The drawn eye interpolates and never extrapolates.** After two frames of
    /// half a step, one step has run and the eye is exactly halfway between the
    /// states either side of it — the answer to 20 Hz physics at a frame rate that
    /// is not 20 — and no frame can draw past the newest step.
    #[test]
    fn the_interpolated_eye_stays_between_the_last_two_steps() {
        let world = Floor { top: Cell::new(0) };
        let mut p = Player::new([0.5, 1.0, 0.5]);
        assert_eq!(p.interpolated_eye(), p.eye(), "before any step, the eye is the spawn's");
        p.advance(&FORWARD, 0.0, PLAYER_STEP * 0.5, &world);
        // The first half-step fired a step, and the drawn eye is on the state before
        // it — the lag the interpolation pays for smoothness.
        assert_eq!(p.interpolated_eye(), eye(p.previous.feet));
        p.advance(&FORWARD, 0.0, PLAYER_STEP * 0.5, &world);
        let (a, b) = (p.previous.feet, p.motion.feet);
        assert_ne!(a, b, "the body did not move, so this test proves nothing");
        let half: [f64; 3] = std::array::from_fn(|axis| (a[axis] + b[axis]) / 2.0);
        let expected = eye(half);
        let drawn = p.interpolated_eye();
        for (axis, value) in drawn.iter().enumerate() {
            assert!(
                (value - expected[axis]).abs() < 1e-12,
                "axis {axis}: drawn {value} not the midpoint {}",
                expected[axis]
            );
        }
        // And under a spread of frame times, the drawn eye never leaves the segment
        // between the two steps.
        for (i, dt) in [0.013, 0.021, 0.007, 0.05, 0.041].iter().cycle().take(200).enumerate() {
            p.advance(&FORWARD, 0.0, *dt, &world);
            let drawn = p.interpolated_eye();
            for (axis, value) in drawn.iter().enumerate() {
                // The eye is the feet with `EYE_HEIGHT` added on Y and nothing on X
                // or Z, so the bound is lifted only on the axis it belongs to.
                let lift = if axis == 1 { EYE_HEIGHT } else { 0.0 };
                let (lo, hi) = (
                    p.previous.feet[axis].min(p.motion.feet[axis]) + lift,
                    p.previous.feet[axis].max(p.motion.feet[axis]) + lift,
                );
                assert!(
                    *value >= lo - 1e-9 && *value <= hi + 1e-9,
                    "frame {i}: axis {axis} drawn {value} outside [{lo}, {hi}]"
                );
            }
        }
    }

    #[test]
    fn the_trajectory_is_a_function_of_the_step_count_and_not_of_the_framing() {
        // The reason the bank exists. Sixty frames of 1/60 s and one hundred and
        // forty-four frames of 1/144 s are the same second, so the same second of
        // walking ends in the same place.
        let world = Floor { top: Cell::new(0) };
        let mut coarse = Player::new([0.5, 1.0, 0.5]);
        for _ in 0..60 {
            coarse.advance(&FORWARD, 0.0, 1.0 / 60.0, &world);
        }
        let mut fine = Player::new([0.5, 1.0, 0.5]);
        for _ in 0..144 {
            fine.advance(&FORWARD, 0.0, 1.0 / 144.0, &world);
        }
        for axis in 0..3 {
            assert!(
                (coarse.motion.feet[axis] - fine.motion.feet[axis]).abs() < 1e-9,
                "axis {axis}: 60 fps ended at {}, 144 fps at {}",
                coarse.motion.feet[axis],
                fine.motion.feet[axis]
            );
        }
    }

    #[test]
    fn a_stalled_or_backwards_frame_moves_nothing() {
        let world = Floor { top: Cell::new(0) };
        let mut p = Player::new([0.5, 1.0, 0.5]);
        let start = p.motion.feet;
        for bad in [-1.0, 0.0, f64::NAN, f64::INFINITY] {
            p.advance(&FORWARD, 0.0, bad, &world);
        }
        assert_eq!(p.motion.feet, start, "a bad delta moved the player");
    }

    #[test]
    fn a_stalled_frame_costs_time_rather_than_queueing_a_burst() {
        let world = Floor { top: Cell::new(0) };
        let mut p = Player::new([0.5, 1.0, 0.5]);
        p.advance(&FORWARD, 0.0, 30.0, &world);
        assert!(
            p.banked_s <= MAX_DT,
            "banked {} s from one frame, over the {MAX_DT} s ceiling",
            p.banked_s
        );
    }

    #[test]
    fn a_body_in_a_shallow_sea_comes_to_rest_on_the_sea_floor() {
        // A surface below sea level, with water above it that the world reports as
        // *not* solid — and that is not sharked here, because the wish has no jump.
        // The body must sink through the water and stand on the floor, not float on
        // the water and not fall through the floor.
        let world = Sea { floor_top: 13, water: (14, 15) };
        let mut m = Motion::standing([0.5, 16.83, -95.5]);
        for _ in 0..600 {
            step(&mut m, &Wish::STILL, 0.0, &world);
        }
        assert!((m.feet[1] - 14.0).abs() < 1e-9, "came to rest at {}", m.feet[1]);
        assert!(m.grounded);
        // And the head is under the 16 m waterline, which is the fact that made the
        // seed worth changing: this is a submerged spawn, not a dry one.
        let head = m.feet[1] + BODY_HEIGHT;
        assert!(head < 16.0, "the head is at {head} m, above the waterline");
    }

    #[test]
    fn a_step_up_of_this_size_cannot_fire_on_a_voxel_grid_at_all() {
        // **Measured, and it is a finding rather than a test of behaviour.** This
        // world's solid features are 1 m voxels, so the smallest rise a body can
        // meet is a whole metre. The ruled [`STEP_UP`] is 0.55 m, which is less than
        // a voxel, so on this world the step-up can never fire at all: it is
        // unreachable code, and every real ledge is a jump.
        //
        // Asserted rather than left implicit, because an absence claim that is only
        // true is worth nothing — this makes it true *on the record*, and if the
        // step-up is ever raised to 1 m this test fails and says why.
        let step_up = STEP_UP;
        assert!(
            step_up < 1.0,
            "STEP_UP is {step_up} m, which now reaches a voxel — this test's premise has changed"
        );
        let world = Floor { top: Cell::new(0) };
        let m = walk(&FORWARD, 600, &world);
        assert!(m.feet[2] > 1.0, "flat ground should not be climbed: {:?}", m.feet);
        assert!(
            (m.feet[1] - 1.0).abs() < 1e-9,
            "a flat floor must not raise the body: {}",
            m.feet[1]
        );
    }

    #[test]
    fn a_one_voxel_ledge_still_needs_a_jump() {
        /// The measured consequence of the user's 0.55 m: in a world of 1 m cubes it
        /// cannot climb a block, so ledges are a jump. Stated as a test because it is
        /// a fact about the world, not a preference.
        struct Block;
        impl Solidity for Block {
            fn is_solid(&self, _x: i32, y: i32, z: i32) -> bool {
                let top = if z >= 2 { 9 } else { 8 };
                y <= top
            }
        }
        let world = Block;
        let m = walk(&FORWARD, 600, &world);
        assert!(
            m.feet[2] < 2.0,
            "walked up a full block to z {} without jumping",
            m.feet[2]
        );
        // With a jump it gets up — through `Player`, so the jump is one press. The
        // press is timed by *distance to the wall*, not by frame index: the arc rises
        // for about five steps and covers about 1.1 m, so a jump taken at the start
        // of the run lands back on the floor long before the body arrives. The
        // window where the body is over the step at the top of its arc is measured
        // by the walk-speed tests to be about 1.1–1.4 m out.
        let mut jumping = Player::new([0.5, 9.0, 0.5]);
        let mut pressed = false;
        for _ in 0..600 {
            let gap = 2.0 - jumping.motion.feet[2];
            let wish = if !pressed && (0.8..=1.4).contains(&gap) {
                pressed = true;
                Wish { forward: 1.0, jump: true, ..Wish::STILL }
            } else {
                FORWARD
            };
            jumping.advance(&wish, 0.0, PLAYER_STEP, &world);
        }
        assert!(pressed, "never reached the window to jump from");
        assert!(
            jumping.motion.feet[2] > 2.5,
            "a jump did not clear the block: {:?}",
            jumping.motion.feet
        );
        assert!(
            (jumping.motion.feet[1] - 10.0).abs() < 1e-9,
            "cleared the block but not onto it: {}",
            jumping.motion.feet[1]
        );
    }

    #[test]
    fn the_eye_is_the_eye_height_above_the_feet() {
        let feet = [1.0, 14.0, -95.5];
        assert_eq!(eye(feet), [1.0, 14.0 + EYE_HEIGHT, -95.5]);
        assert_eq!(eye_for(feet, true), [1.0, 14.0 + SNEAK_EYE_HEIGHT, -95.5]);
        // The crouch is Minecraft's own 0.35 m, read as values rather than folded.
        let standing = EYE_HEIGHT;
        let crouching = SNEAK_EYE_HEIGHT;
        assert!(
            (standing - crouching - 0.35).abs() < 1e-12,
            "a crouch drops the eye {} m, not Minecraft's 0.35",
            standing - crouching
        );
    }

    /// **The crouch lowers the eye and not the body** (the user's report after
    /// playing C-150: *the camera doesn't shift downwards when you crouch*). The feet
    /// do not move, the pose lowers the eye, the drawn eye eases between the two
    /// poses instead of snapping, and releasing the key stands it back up.
    #[test]
    fn crouching_lowers_the_eye_and_not_the_body() {
        let world = Floor { top: Cell::new(0) };
        let mut p = Player::new([0.5, 1.0, 0.5]);
        p.advance(&FORWARD, 0.0, PLAYER_STEP, &world);
        assert_eq!(p.eye()[1], 1.0 + EYE_HEIGHT, "the standing eye moved before any crouch");
        let crouch = Wish { sneak: true, ..FORWARD };
        p.advance(&crouch, 0.0, PLAYER_STEP, &world);
        assert_eq!(p.motion.feet[1], 1.0, "the body must not move for a crouch");
        assert_eq!(
            p.eye()[1],
            1.0 + SNEAK_EYE_HEIGHT,
            "the crouched eye is the sneak height"
        );
        // The drawn eye is strictly between the poses: the crouch eases over the step.
        let drawn = p.interpolated_eye()[1];
        assert!(
            drawn > 1.0 + SNEAK_EYE_HEIGHT && drawn <= 1.0 + EYE_HEIGHT,
            "the drawn eye {drawn} is not between the poses"
        );
        // And releasing the key stands it back up.
        p.advance(&FORWARD, 0.0, PLAYER_STEP, &world);
        assert_eq!(p.eye()[1], 1.0 + EYE_HEIGHT, "the eye did not stand back up");
    }
}

