//! CameraOverhaul's camera effects: the look's smoothing, sway and velocity tilts.
//!
//! # Where this comes from, and what it is not
//!
//! The reference is `Mirsario/Minecraft-CameraOverhaul` (GPL-3.0), and this module is
//! a **clean-room port of its mechanism**: the effects, the constants and the
//! smoothing arithmetic are cited from its `CameraSystem.java`, `ConfigData.java`,
//! `CameraContext.java`, `MathUtils.java` and `VectorUtils.java` on the `dev` branch,
//! and none of its code was copied (C-48/C-49). It is not a Minecraft mod and this
//! file never touches a `winit` or a `wgpu` type: [`CameraOverhaul::update`] takes the
//! player's camera, the body's velocity and a frame delta, and returns offsets.
//!
//! # The offsets are presentation, and only presentation
//!
//! [`OverhaulOffsets::applied_to`] adds them to a **copy** of the camera and the frame
//! is drawn with that copy. The stored camera — the one the movement reads its yaw
//! from — is never written here. A smoothed look fed back into the movement would be
//! a physics input that varies with the frame rate, which is the one thing this
//! client's tick model exists to prevent.
//!
//! # The effects, named as the reference names them
//!
//! * **Camera smoothing** — mouse smoothing: the drawn yaw and pitch follow the
//!   real angles through `1 - exp(-(16 / smoothing) · dt)`, so the sprint context's
//!   0.6 makes the view lag a fast flick by a few degrees. The reference enables it
//!   only where its context says so, and the walking default is **off** (0.0): this
//!   port therefore smooths nothing while walking, exactly like the reference.
//! * **Sway** — a slow, seeded 2D noise (the reference's simplex, at frequency 0.16
//!   and intensity 0.60°) applied to yaw and pitch, fading in over five seconds when
//!   the player is still and out over 0.75 s when they act. "Acting" is the same
//!   test the reference uses: **the velocity or the look changed since the last
//!   frame**. A dig or a place does not move those and so does not fade the sway;
//!   the reference's extra `notifyOfPlayerAction` hook is not ported.
//! * **Vertical velocity pitch** — pitching with vertical velocity, gated at a 0.4°
//!   threshold on the target (the reference's fix for slime-block shake).
//! * **Forward velocity pitch** — pitching with the forward component of the
//!   velocity: running forward tilts the view down.
//! * **Turning roll** — the accumulated, eased bank into a turn. Fast flicks
//!   saturate it near 1.5°; slow turns barely move it, which is the design.
//! * **Strafing roll** — banking with lateral velocity, damped through the same
//!   horizontal-smoothing factor the forward pitch uses.
//! * **The context crossfade** — walking/sprinting configurations blend at about
//!   8 % per frame (`contextTransitionSmoothing` 0.1), so entering a sprint eases the
//!   smoothing and factors in rather than switching.
//!
//! The reference's swimming/flying/mounts/vehicles contexts and its screen shakes
//! are **not** ported: this client has no swimming state, no flight and no
//! explosions, and a context that cannot be entered is a table of dead numbers.
//!
//! # The signs, stated because a screenshot cannot be a test
//!
//! Pitch: **our pitch is positive looking up, the reference's `eulerRot.x` is
//! positive looking down**, so every pitch offset is negated on the way in
//! ([`PITCH_AXIS`]). That makes a fall tilt the view up and running forward tilt it
//! down, which are the reference's own directions.
//!
//! Roll: **our `roll_rad` is positive when the camera banks left** (`camera.rs`), and
//! the offsets are added in that convention so a right turn banks right — into the
//! turn. The reference's strafing algebra gives a strafe the opposite bank to a turn
//! of the same direction, and this port keeps that algebra exactly as published
//! (`strafing = +rightward × factor`, a double negative in the reference's own
//! left-positive frame). If a bank ever reads as inverted on screen, the axis
//! constants here and in `camera.rs` are the two places it lives, and both are pinned
//! by tests.

use crate::camera::{Camera, MAX_PITCH_RAD};
use crate::clock::TICK_HZ;

/// Metres per second in one block-per-tick.
///
/// The reference's velocities are Minecraft blocks per tick and its factors are sized
/// for those numbers; the body's velocity here is m/s, so it is divided by this on the
/// way into every velocity-driven effect.
const BLOCKS_PER_TICK_TO_M_S: f64 = TICK_HZ as f64;

// ---- the reference's `General` block ----------------------------------------

/// `contextTransitionSmoothing` — how fast a context crossfades, per frame.
const CONTEXT_TRANSITION_SMOOTHING: f64 = 0.1;
/// `turningRollAccumulation`.
const TURNING_ROLL_ACCUMULATION: f64 = 1.0;
/// `turningRollIntensity`.
const TURNING_ROLL_INTENSITY: f64 = 1.25;
/// `turningRollSmoothing`.
const TURNING_ROLL_SMOOTHING: f64 = 1.0;
/// `cameraSwayIntensity`, in degrees.
const CAMERA_SWAY_INTENSITY: f64 = 0.60;
/// `cameraSwayFrequency`.
const CAMERA_SWAY_FREQUENCY: f64 = 0.16;
/// `cameraSwayFadeInDelay`, in seconds.
const CAMERA_SWAY_FADE_IN_DELAY: f64 = 0.15;
/// `cameraSwayFadeInLength`, in seconds.
const CAMERA_SWAY_FADE_IN_LENGTH: f64 = 5.0;
/// `cameraSwayFadeOutLength`, in seconds.
const CAMERA_SWAY_FADE_OUT_LENGTH: f64 = 0.75;
/// The exponent the sway's fade is raised to before it scales the intensity.
const CAMERA_SWAY_FADING_SMOOTHNESS: f64 = 3.0;

// ---- the reference's `CameraSystem` constants -------------------------------

/// `BASE_MOUSE_SMOOTHING`.
const BASE_MOUSE_SMOOTHING: f64 = 16.0;
/// `MOUSE_SMOOTHING_THRESHOLD`: at or below this the smoothing is off entirely.
const MOUSE_SMOOTHING_THRESHOLD: f64 = 0.001;
/// `BASE_VERTICAL_PITCH_SMOOTHING`.
const BASE_VERTICAL_PITCH_SMOOTHING: f64 = 0.000_04;
/// `VERTICAL_PITCH_THRESHOLD`, in degrees, on the **target** offset.
const VERTICAL_PITCH_THRESHOLD_DEG: f64 = 0.4;
/// `BASE_FORWARD_PITCH_SMOOTHING`.
const BASE_FORWARD_PITCH_SMOOTHING: f64 = 0.008;
/// `BASE_TURNING_ROLL_ACCUMULATION`: accumulated per degree of yaw per frame.
const BASE_TURNING_ROLL_ACCUMULATION: f64 = 0.004_8;
/// `BASE_TURNING_ROLL_INTENSITY`, in degrees.
const BASE_TURNING_ROLL_INTENSITY: f64 = 1.25;
/// `BASE_TURNING_ROLL_SMOOTHING`.
const BASE_TURNING_ROLL_SMOOTHING: f64 = 0.082_5;
/// `BASE_STRAFING_ROLL_SMOOTHING`.
const BASE_STRAFING_ROLL_SMOOTHING: f64 = 0.008;

/// The reference's `eulerRot.x` is positive looking down; ours is positive looking
/// up, so pitch offsets carry this sign. See the module header.
const PITCH_AXIS: f64 = -1.0;

/// One context's factors — the reference's `ConfigData.Contextual`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Contextual {
    strafing_roll_factor: f64,
    forward_velocity_pitch_factor: f64,
    vertical_velocity_pitch_factor: f64,
    horizontal_velocity_smoothing_factor: f64,
    vertical_velocity_smoothing_factor: f64,
    camera_smoothing: f64,
}

impl Contextual {
    /// The reference's `Contextual::lerp`, used for the crossfade and nothing else.
    fn lerped(&self, target: &Contextual, t: f64) -> Contextual {
        let mix = |a: f64, b: f64| a + (b - a) * t;
        Contextual {
            strafing_roll_factor: mix(self.strafing_roll_factor, target.strafing_roll_factor),
            forward_velocity_pitch_factor: mix(
                self.forward_velocity_pitch_factor,
                target.forward_velocity_pitch_factor,
            ),
            vertical_velocity_pitch_factor: mix(
                self.vertical_velocity_pitch_factor,
                target.vertical_velocity_pitch_factor,
            ),
            horizontal_velocity_smoothing_factor: mix(
                self.horizontal_velocity_smoothing_factor,
                target.horizontal_velocity_smoothing_factor,
            ),
            vertical_velocity_smoothing_factor: mix(
                self.vertical_velocity_smoothing_factor,
                target.vertical_velocity_smoothing_factor,
            ),
            camera_smoothing: mix(self.camera_smoothing, target.camera_smoothing),
        }
    }
}

/// The reference's `walking` defaults.
const WALKING: Contextual = Contextual {
    strafing_roll_factor: 10.0,
    forward_velocity_pitch_factor: 7.0,
    vertical_velocity_pitch_factor: 2.5,
    horizontal_velocity_smoothing_factor: 1.0,
    vertical_velocity_smoothing_factor: 1.0,
    camera_smoothing: 0.0,
};

/// The reference's `sprinting` overrides: a stronger strafe and forward factor, and
/// the 0.6 mouse smoothing that is off while walking.
const SPRINTING: Contextual = Contextual {
    strafing_roll_factor: 12.5,
    forward_velocity_pitch_factor: 9.5,
    vertical_velocity_pitch_factor: 2.5,
    horizontal_velocity_smoothing_factor: 1.0,
    vertical_velocity_smoothing_factor: 1.0,
    camera_smoothing: 0.6,
};

/// The offsets one frame adds to the drawn camera, in radians.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct OverhaulOffsets {
    /// Added to the drawn yaw.
    pub yaw_rad: f64,
    /// Added to the drawn pitch, in **this port's** convention (positive = up).
    pub pitch_rad: f64,
    /// Added to the drawn roll (positive = the camera banks left).
    pub roll_rad: f64,
}

impl OverhaulOffsets {
    /// The camera the frame is drawn with: the player's camera plus these offsets.
    ///
    /// A copy, because the stored camera is the movement's input and this never
    /// writes it. Pitch is clamped where the look's own pitch is clamped, so a
    /// velocity tilt cannot push the basis through the pole.
    pub fn applied_to(&self, camera: &Camera) -> Camera {
        let mut drawn = *camera;
        drawn.yaw_rad += self.yaw_rad;
        drawn.pitch_rad = (drawn.pitch_rad + self.pitch_rad).clamp(-MAX_PITCH_RAD, MAX_PITCH_RAD);
        drawn.roll_rad += self.roll_rad;
        drawn
    }
}

/// The ported effect state. One per client, updated once per frame.
///
/// Every field is the reference's: the two previous angles and the previous velocity
/// exist to detect "the player acted"; the continuous/smoothed angle pair is the
/// mouse smoothing's own unwrapped stream; and each effect keeps the damped value it
/// is drawn with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraOverhaul {
    time_s: f64,
    context: Contextual,
    prev_yaw_deg: f64,
    prev_pitch_deg: f64,
    prev_velocity: [f64; 3],
    last_action_s: f64,
    turning_roll_target: f64,
    prev_vertical_pitch: f64,
    prev_forward_pitch: f64,
    prev_strafing_roll: f64,
    sway_factor: f64,
    sway_factor_target: f64,
    ms_initialized: bool,
    prev_yaw_norm: f64,
    prev_pitch_norm: f64,
    cont_yaw: f64,
    cont_pitch: f64,
    sm_yaw: f64,
    sm_pitch: f64,
}

impl Default for CameraOverhaul {
    fn default() -> Self {
        Self::new()
    }
}

impl CameraOverhaul {
    /// The effects at rest: walking context, nothing smoothed, nothing added.
    pub fn new() -> Self {
        CameraOverhaul {
            time_s: 0.0,
            context: WALKING,
            prev_yaw_deg: 0.0,
            prev_pitch_deg: 0.0,
            prev_velocity: [0.0; 3],
            last_action_s: f64::NEG_INFINITY,
            turning_roll_target: 0.0,
            prev_vertical_pitch: 0.0,
            prev_forward_pitch: 0.0,
            prev_strafing_roll: 0.0,
            sway_factor: 0.0,
            sway_factor_target: 0.0,
            ms_initialized: false,
            prev_yaw_norm: 0.0,
            prev_pitch_norm: 0.0,
            cont_yaw: 0.0,
            cont_pitch: 0.0,
            sm_yaw: 0.0,
            sm_pitch: 0.0,
        }
    }

    /// Advance the effects by one frame and return what it adds to the drawn camera.
    ///
    /// `velocity_m_s` is the body's velocity — the drawn camera is placed by the
    /// caller and the velocity is what several effects read. `sprinting` picks the
    /// context this frame eases toward. A non-positive or non-finite `dt_s` returns
    /// zero offsets and changes no state: a frame that reports bad time is a bug, and
    /// damping over it would invent motion from it.
    pub fn update(
        &mut self,
        camera: &Camera,
        velocity_m_s: [f64; 3],
        sprinting: bool,
        dt_s: f64,
    ) -> OverhaulOffsets {
        if !dt_s.is_finite() || dt_s <= 0.0 {
            return OverhaulOffsets::default();
        }
        self.time_s += dt_s;

        // ---- the context crossfade --------------------------------------------
        let target = if sprinting { SPRINTING } else { WALKING };
        let smoothing = if CONTEXT_TRANSITION_SMOOTHING > 0.0 {
            damp_step(CONTEXT_TRANSITION_SMOOTHING, dt_s)
        } else {
            1.0
        };
        self.context = self.context.lerped(&target, damp_step(smoothing, dt_s));

        let yaw_deg = camera.yaw_rad.to_degrees();
        let pitch_deg = camera.pitch_rad.to_degrees();

        // ---- "the player acted" ------------------------------------------------
        // The reference's own test, and the only one: the velocity or the look moved
        // since the previous frame. It is what fades the sway out, and what delays
        // its fade-in after the last action.
        if velocity_m_s != self.prev_velocity
            || yaw_deg != self.prev_yaw_deg
            || pitch_deg != self.prev_pitch_deg
        {
            self.last_action_s = self.time_s;
        }

        // ---- camera smoothing (yaw and pitch) ---------------------------------
        let mut smooth_yaw_deg = 0.0;
        let mut smooth_pitch_deg = 0.0;
        if !self.ms_initialized {
            // The first frame only initializes: the continuous stream starts at the
            // angles it finds, so a camera that opens turned is not smeared toward
            // zero.
            self.prev_yaw_norm = yaw_deg;
            self.prev_pitch_norm = pitch_deg;
            self.cont_yaw = yaw_deg;
            self.cont_pitch = pitch_deg;
            self.sm_yaw = yaw_deg;
            self.sm_pitch = pitch_deg;
            self.ms_initialized = true;
        } else {
            // The angles are unwrapped step by step, so a crossing through ±180°
            // does not spike the stream by 360.
            let step_yaw = wrap_degrees(yaw_deg - self.prev_yaw_norm);
            let step_pitch = wrap_degrees(pitch_deg - self.prev_pitch_norm);
            self.prev_yaw_norm = yaw_deg;
            self.prev_pitch_norm = pitch_deg;
            self.cont_yaw += step_yaw;
            self.cont_pitch += step_pitch;

            let smoothing_value = self.context.camera_smoothing.max(0.0);
            if smoothing_value <= MOUSE_SMOOTHING_THRESHOLD {
                self.sm_yaw = self.cont_yaw;
                self.sm_pitch = self.cont_pitch;
            } else {
                let k = BASE_MOUSE_SMOOTHING / smoothing_value;
                let step = 1.0 - (-k * dt_s).exp();
                self.sm_yaw += (self.cont_yaw - self.sm_yaw) * step;
                self.sm_pitch += (self.cont_pitch - self.sm_pitch) * step;
            }
            // The offset is what makes `real + offset` the smoothed angle.
            smooth_yaw_deg = wrap_degrees(self.sm_yaw - yaw_deg);
            smooth_pitch_deg = wrap_degrees(self.sm_pitch - pitch_deg);
        }

        // ---- sway ---------------------------------------------------------------
        let mut sway_yaw_deg = 0.0;
        let mut sway_pitch_deg = 0.0;
        if CAMERA_SWAY_INTENSITY != 0.0 {
            let noise_x = self.time_s * CAMERA_SWAY_FREQUENCY;
            if (self.time_s - self.last_action_s) < CAMERA_SWAY_FADE_IN_DELAY {
                self.sway_factor_target = 0.0;
            } else if self.sway_factor == self.sway_factor_target {
                // Only start a fade-in once the last fade-out has finished.
                self.sway_factor_target = 1.0;
            }
            let fade_length = if self.sway_factor_target > 0.0 {
                CAMERA_SWAY_FADE_IN_LENGTH
            } else {
                CAMERA_SWAY_FADE_OUT_LENGTH
            };
            let fade_step = if fade_length > 0.0 {
                dt_s / fade_length
            } else {
                1.0
            };
            self.sway_factor = step_towards(self.sway_factor, self.sway_factor_target, fade_step);
            let scaled_intensity =
                CAMERA_SWAY_INTENSITY * self.sway_factor.powf(CAMERA_SWAY_FADING_SMOOTHNESS);
            // The reference's own axes' seeds; its third component is multiplied by
            // a zero intensity, so only two are needed here.
            sway_yaw_deg = noise_2d(noise_x, 420.0) * scaled_intensity;
            sway_pitch_deg = noise_2d(noise_x, 1337.0) * scaled_intensity;
        }

        // ---- vertical velocity pitch -------------------------------------------
        let vertical_target_deg = (velocity_m_s[1] / BLOCKS_PER_TICK_TO_M_S)
            * self.context.vertical_velocity_pitch_factor;
        let vertical_target_deg = if vertical_target_deg.abs() < VERTICAL_PITCH_THRESHOLD_DEG {
            0.0
        } else {
            vertical_target_deg
        };
        self.prev_vertical_pitch = damp(
            self.prev_vertical_pitch,
            vertical_target_deg,
            BASE_VERTICAL_PITCH_SMOOTHING * self.context.vertical_velocity_smoothing_factor,
            dt_s,
        );

        // ---- forward velocity pitch, and the strafe's lateral component --------
        let (sin_yaw, cos_yaw) = camera.yaw_rad.sin_cos();
        let forward_speed_bpt =
            (velocity_m_s[0] * sin_yaw + velocity_m_s[2] * cos_yaw) / BLOCKS_PER_TICK_TO_M_S;
        let right_speed_bpt =
            (velocity_m_s[0] * cos_yaw - velocity_m_s[2] * sin_yaw) / BLOCKS_PER_TICK_TO_M_S;
        let forward_target_deg = forward_speed_bpt * self.context.forward_velocity_pitch_factor;
        self.prev_forward_pitch = damp(
            self.prev_forward_pitch,
            forward_target_deg,
            BASE_FORWARD_PITCH_SMOOTHING * self.context.horizontal_velocity_smoothing_factor,
            dt_s,
        );

        // ---- turning roll -------------------------------------------------------
        let yaw_delta_deg = self.prev_yaw_deg - yaw_deg;
        self.turning_roll_target = damp(
            self.turning_roll_target,
            0.0,
            BASE_TURNING_ROLL_SMOOTHING * TURNING_ROLL_SMOOTHING,
            dt_s,
        );
        self.turning_roll_target = (self.turning_roll_target
            + yaw_delta_deg * (BASE_TURNING_ROLL_ACCUMULATION * TURNING_ROLL_ACCUMULATION))
            .clamp(-1.0, 1.0);
        let turning_roll_deg = turning_easing(self.turning_roll_target.abs()).clamp(0.0, 1.0)
            * (BASE_TURNING_ROLL_INTENSITY * TURNING_ROLL_INTENSITY)
            * self.turning_roll_target.signum();

        // ---- strafing roll ------------------------------------------------------
        // The reference computes a left-positive lateral speed and negates it; in this
        // camera's right-positive frame that is a **double negative**, so the target
        // is the rightward speed itself. See the module header.
        let strafing_target_deg = right_speed_bpt * self.context.strafing_roll_factor;
        self.prev_strafing_roll = damp(
            self.prev_strafing_roll,
            strafing_target_deg,
            BASE_STRAFING_ROLL_SMOOTHING * self.context.horizontal_velocity_smoothing_factor,
            dt_s,
        );

        // ---- record this frame for the next one's action test -------------------
        self.prev_velocity = velocity_m_s;
        self.prev_yaw_deg = yaw_deg;
        self.prev_pitch_deg = pitch_deg;

        OverhaulOffsets {
            yaw_rad: (smooth_yaw_deg + sway_yaw_deg).to_radians(),
            pitch_rad: PITCH_AXIS
                * (smooth_pitch_deg
                    + sway_pitch_deg
                    + self.prev_vertical_pitch
                    + self.prev_forward_pitch)
                    .to_radians(),
            roll_rad: (turning_roll_deg + self.prev_strafing_roll).to_radians(),
        }
    }
}

/// The reference's framerate-independent damping: `lerp(source, destination,
/// dampStep(smoothing, dt))`, where `dampStep = 1 - smoothing²^dt`.
fn damp(source: f64, destination: f64, smoothing: f64, dt_s: f64) -> f64 {
    let t = damp_step(smoothing, dt_s);
    source + (destination - source) * t
}

/// The reference's `dampStep`: a smoothing value of `s` decays over a second like
/// `s²`, whatever the frame time.
fn damp_step(smoothing: f64, dt_s: f64) -> f64 {
    1.0 - (smoothing * smoothing).powf(dt_s)
}

/// The reference's `stepTowards`.
fn step_towards(current: f64, target: f64, step: f64) -> f64 {
    if current < target {
        (current + step).min(target)
    } else if current > target {
        (current - step).max(target)
    } else {
        current
    }
}

/// The reference's `wrapDiff`/`unwrapStep`: an angular step wrapped into (-180, 180]
/// so a crossing of the seam does not read as a 360° turn.
///
/// The reference declares this arithmetic twice under two names; one function serves
/// both here.
fn wrap_degrees(delta_deg: f64) -> f64 {
    let mut d = delta_deg % 360.0;
    if d <= -180.0 {
        d += 360.0;
    } else if d > 180.0 {
        d -= 360.0;
    }
    d
}

/// The reference's `turningEasing`: easeInOutCubic.
fn turning_easing(x: f64) -> f64 {
    if x < 0.5 {
        4.0 * x * x * x
    } else {
        1.0 - (-2.0 * x + 2.0).powi(3) / 2.0
    }
}

/// Deterministic 2D gradient noise, for the sway.
///
/// **From scratch, and not the reference's generator**: the reference calls joml's
/// simplex noise with a time-scaled x and a fixed per-axis y (420, 1337, 6969), and
/// the mechanism that matters is *a smooth, seeded, bounded function of time*, not the
/// particular simplex table. This is a Perlin-style lattice with a fixed hash, faded
/// with the quintic curve, and its ±0.707 range is scaled to the simplex-like ±1 the
/// reference's amplitude is sized for.
fn noise_2d(x: f64, y: f64) -> f64 {
    let x0 = x.floor();
    let y0 = y.floor();
    let xf = x - x0;
    let yf = y - y0;
    let (ix, iy) = (x0 as i64, y0 as i64);
    let u = fade(xf);
    let v = fade(yf);
    let n00 = dot_gradient(hash_2d(ix, iy), xf, yf);
    let n10 = dot_gradient(hash_2d(ix + 1, iy), xf - 1.0, yf);
    let n01 = dot_gradient(hash_2d(ix, iy + 1), xf, yf - 1.0);
    let n11 = dot_gradient(hash_2d(ix + 1, iy + 1), xf - 1.0, yf - 1.0);
    let noise = mix(mix(n00, n10, u), mix(n01, n11, u), v);
    noise * std::f64::consts::SQRT_2
}

/// The quintic fade, so the noise is smooth in its second derivative as well.
fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// Linear interpolation, clamp-free by the caller's choice.
fn mix(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// A 2D lattice hash: integer coordinates in, a stable pseudo-random word out.
///
/// `wrapping_mul` and shifts only, so the same coordinates give the same word on
/// every machine and every run — the sway is deterministic, which is what lets a test
/// name a frame.
fn hash_2d(x: i64, y: i64) -> u64 {
    let mut h = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    h ^= h >> 29;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 32;
    h
}

/// One lattice point's gradient dot: eight unit directions at 45°.
fn dot_gradient(hash: u64, x: f64, y: f64) -> f64 {
    const DIAGONAL: f64 = std::f64::consts::FRAC_1_SQRT_2;
    const DIRECTIONS: [[f64; 2]; 8] = [
        [1.0, 0.0],
        [DIAGONAL, DIAGONAL],
        [0.0, 1.0],
        [-DIAGONAL, DIAGONAL],
        [-1.0, 0.0],
        [-DIAGONAL, -DIAGONAL],
        [0.0, -1.0],
        [DIAGONAL, -DIAGONAL],
    ];
    let d = DIRECTIONS[(hash % 8) as usize];
    d[0] * x + d[1] * y
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A camera at the origin, looking down +Z.
    fn cam() -> Camera {
        Camera::standard([0.0, 0.0, 0.0], 16.0 / 9.0)
    }

    const DT: f64 = 1.0 / 60.0;

    /// The offsets are added to a **copy** of the player's camera: the draw is tilted,
    /// the movement's own look is not.
    #[test]
    fn the_offsets_are_added_to_a_copy_and_never_touch_the_players_camera() {
        let camera = Camera::standard([1.0, 2.0, 3.0], 1.0);
        let offsets = OverhaulOffsets {
            yaw_rad: 0.1,
            pitch_rad: 0.05,
            roll_rad: 0.02,
        };
        let drawn = offsets.applied_to(&camera);
        assert_eq!(drawn.yaw_rad, 0.1);
        assert_eq!(drawn.pitch_rad, 0.05);
        assert_eq!(drawn.roll_rad, 0.02);
        assert_eq!(drawn.position_m, camera.position_m);
        assert_eq!(
            camera.yaw_rad, 0.0,
            "the player's camera must not be written"
        );
        assert_eq!(camera.roll_rad, 0.0);
        // A pitch offset cannot push the drawn camera through the pole.
        let extreme = OverhaulOffsets {
            yaw_rad: 0.0,
            pitch_rad: 10.0,
            roll_rad: 0.0,
        };
        assert!(extreme.applied_to(&camera).pitch_rad <= MAX_PITCH_RAD);
    }

    /// **Walking smooths nothing; sprinting lags a fast flick.** The reference's
    /// walking `cameraSmoothing` is 0.0 — so the drawn angle equals the real one
    /// exactly — and its sprinting context is 0.6, which shows.
    #[test]
    fn walking_does_not_smooth_the_look_where_sprinting_does() {
        let mut oh = CameraOverhaul::new();
        let mut camera = cam();
        // A velocity that changes every frame keeps the sway faded out, so what is
        // measured here is the smoothing alone.
        let mut walking_offsets = OverhaulOffsets::default();
        for i in 0..30 {
            camera.yaw_rad += 10.0_f64.to_radians();
            walking_offsets = oh.update(&camera, [i as f64 * 0.001, 0.0, 0.0], false, DT);
        }
        assert_eq!(
            walking_offsets.yaw_rad, 0.0,
            "the walking context must not smooth the look"
        );
        // The same flick, in the sprinting context: the smoothed angle now trails.
        let mut sprint_offsets = OverhaulOffsets::default();
        for i in 0..90 {
            camera.yaw_rad += 10.0_f64.to_radians();
            sprint_offsets = oh.update(&camera, [i as f64 * 0.001, 0.0, 0.0], true, DT);
        }
        assert!(
            sprint_offsets.yaw_rad < -0.05,
            "the sprinting context smoothed by only {} rad",
            sprint_offsets.yaw_rad
        );
    }

    /// A fast flick accumulates a bank that decays away in about a second.
    #[test]
    fn a_fast_right_flick_banks_and_decays() {
        let mut oh = CameraOverhaul::new();
        let mut camera = cam();
        let mut roll = 0.0;
        for i in 0..4 {
            camera.yaw_rad += 45.0_f64.to_radians();
            roll = oh
                .update(&camera, [i as f64 * 0.01, 0.0, 0.0], false, DT)
                .roll_rad;
        }
        assert!(
            roll < -0.015,
            "a 45°/frame right flick banked only {roll} rad"
        );
        // Turning stopped: the bank decays through the reference's 0.0825 smoothing.
        let still = [0.1, -1.568, 0.0];
        for _ in 0..90 {
            roll = oh.update(&camera, still, false, DT).roll_rad;
        }
        assert!(
            roll.abs() < 0.002,
            "the bank was still {roll} rad after 1.5 s"
        );
    }

    /// **The reference's strafing algebra, pinned**: a rightward velocity gives a
    /// positive strafing roll in this frame, the opposite sign to a right *turn*.
    /// The bank on screen is a matter of taste; that the two effects disagree is the
    /// reference's own arithmetic and is not left to drift.
    #[test]
    fn a_right_strafe_rolls_opposite_to_a_right_turn() {
        let camera = cam();
        let mut strafing = CameraOverhaul::new();
        let mut roll = 0.0;
        for _ in 0..60 {
            roll = strafing
                .update(&camera, [4.317, 0.0, 0.0], false, DT)
                .roll_rad;
        }
        assert!(roll > 0.02, "a rightward strafe gave {roll} rad");
        let mut turning = CameraOverhaul::new();
        let mut camera = cam();
        let mut turn_roll = 0.0;
        for i in 0..4 {
            camera.yaw_rad += 45.0_f64.to_radians();
            turn_roll = turning
                .update(&camera, [i as f64 * 0.01, 0.0, 0.0], false, DT)
                .roll_rad;
        }
        assert!(
            turn_roll < 0.0,
            "a right turn rolled {turn_roll} rad — the same way as a right strafe"
        );
    }

    /// **The vertical pitch offset**: the reference's 0.4° threshold means a slow
    /// drift adds nothing and a real fall adds the factor's full tilt — and a fall
    /// tilts the view **up** in this port (the pitch axis is negated).
    #[test]
    fn the_vertical_pitch_ignores_slow_falls_and_follows_fast_ones() {
        let camera = cam();
        // 2 m/s is 0.1 blocks/tick × 2.5 = 0.25° — under the threshold, so zero.
        let mut slow = CameraOverhaul::new();
        let mut slow_pitch = 0.0;
        for i in 0..60 {
            slow_pitch = slow
                .update(&camera, [0.0, -2.0 - i as f64 * 1e-4, 0.0], false, DT)
                .pitch_rad;
        }
        assert_eq!(
            slow_pitch, 0.0,
            "a slow fall tilted the view by {slow_pitch} rad"
        );
        // A fall at 20 m/s is 1 block/tick × 2.5 = 2.5°, and the axis carries it up.
        let mut fast = CameraOverhaul::new();
        let mut fast_pitch = 0.0;
        for i in 0..60 {
            fast_pitch = fast
                .update(&camera, [0.0, -20.0 - i as f64 * 1e-4, 0.0], false, DT)
                .pitch_rad;
        }
        let expected = 2.5_f64.to_radians();
        assert!(
            fast_pitch > 0.0,
            "a fall must tilt the view up here, not down"
        );
        assert!(
            (fast_pitch - expected).abs() < 0.001,
            "a 20 m/s fall gave {fast_pitch} rad, expected {expected}"
        );
    }

    /// The forward pitch follows the reference's 7.0 factor on blocks per tick, and
    /// is **down** in this port — running forward leans the view into the motion.
    #[test]
    fn the_forward_pitch_follows_the_reference_factor() {
        let camera = cam();
        let mut oh = CameraOverhaul::new();
        let mut pitch = 0.0;
        for _ in 0..60 {
            pitch = oh
                .update(&camera, [0.0, -1.568, 4.317], false, DT)
                .pitch_rad;
        }
        // 4.317 m/s = 0.21585 blocks/tick × 7.0 = 1.511°, negated by the pitch axis.
        let expected = -(0.215_85 * 7.0_f64).to_radians();
        assert!(
            (pitch - expected).abs() < 0.001,
            "forward at a walk gave {pitch} rad, expected {expected}"
        );
        assert!(pitch < 0.0, "forward must tilt the view down in this port");
    }

    /// The sway fades in while the player is still and out as soon as they act, and
    /// it moves the drawn yaw and pitch — never the roll.
    #[test]
    fn the_sway_fades_in_while_still_and_out_when_the_player_acts() {
        let mut oh = CameraOverhaul::new();
        let camera = cam();
        // A standing body, at the reference's own stored value — constant, so no
        // frame counts as an action after the first.
        let still = [0.0, -1.568, 0.0];
        let mut biggest = 0.0_f64;
        for _ in 0..360 {
            let offsets = oh.update(&camera, still, false, DT);
            let magnitude =
                (offsets.yaw_rad * offsets.yaw_rad + offsets.pitch_rad * offsets.pitch_rad).sqrt();
            biggest = biggest.max(magnitude);
        }
        assert_eq!(oh.sway_factor, 1.0, "the sway never reached full strength");
        assert!(
            biggest > 0.002,
            "six seconds of sway moved the view by only {biggest} rad"
        );
        // Now act — a turn every frame — and it fades out over 0.75 s.
        let mut turning = cam();
        for _ in 0..90 {
            turning.yaw_rad += 0.01;
            oh.update(&turning, still, false, DT);
        }
        assert!(
            oh.sway_factor < 0.05,
            "the sway was still {} after acting",
            oh.sway_factor
        );
    }

    /// The contexts crossfade rather than switch, through the reference's own double
    /// damping.
    #[test]
    fn the_contexts_crossfade_rather_than_switch() {
        let camera = cam();
        let mut oh = CameraOverhaul::new();
        for i in 0..120 {
            oh.update(&camera, [i as f64 * 0.01, 0.0, 0.0], true, DT);
        }
        assert!(
            oh.context.camera_smoothing > 0.59,
            "the sprinting context did not arrive: {}",
            oh.context.camera_smoothing
        );
        assert!(oh.context.strafing_roll_factor > 12.4);
        for i in 0..120 {
            oh.update(&camera, [i as f64 * 0.01, 0.0, 0.0], false, DT);
        }
        assert!(
            oh.context.camera_smoothing < 0.01,
            "the walking context did not return: {}",
            oh.context.camera_smoothing
        );
    }

    #[test]
    fn a_bad_frame_delta_changes_nothing() {
        let camera = cam();
        let mut oh = CameraOverhaul::new();
        oh.update(&camera, [0.0; 3], false, DT);
        let before = oh;
        for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            let offsets = oh.update(&camera, [0.0; 3], false, bad);
            assert_eq!(offsets, OverhaulOffsets::default());
        }
        assert_eq!(oh, before, "a bad delta moved the effect state");
    }

    /// The noise is deterministic, two seeds are two streams, and the amplitude is
    /// the simplex-like range the reference's intensity is sized for.
    #[test]
    fn the_noise_is_deterministic_and_bounded() {
        assert_eq!(noise_2d(0.37, 420.0), noise_2d(0.37, 420.0));
        assert_ne!(noise_2d(0.37, 420.0), noise_2d(0.37, 1337.0));
        let mut biggest = 0.0_f64;
        for i in 0..2_000 {
            let x = i as f64 * 0.01;
            for seed in [420.0, 1337.0] {
                let n = noise_2d(x, seed);
                assert!(n.is_finite());
                biggest = biggest.max(n.abs());
            }
        }
        assert!(biggest > 0.2, "the noise is suspiciously flat: {biggest}");
        assert!(
            biggest < 1.5,
            "the noise is outside the simplex-like range: {biggest}"
        );
    }

    #[test]
    fn the_turning_easing_is_the_references_cubic() {
        assert_eq!(turning_easing(0.0), 0.0);
        assert_eq!(turning_easing(0.5), 0.5);
        assert_eq!(turning_easing(1.0), 1.0);
        assert!((turning_easing(0.25) - 0.0625).abs() < 1e-12);
    }
}
