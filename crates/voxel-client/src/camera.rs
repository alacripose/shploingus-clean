//! A first-person camera, and the one rule it exists to keep.
//!
//! # Why the matrices are hand-written
//!
//! This crate has two dependencies and both are named in `Cargo.toml`. A
//! look-at and a perspective matrix are about sixty lines of arithmetic that
//! either works or is caught by a test, so `glam` is not a dependency here —
//! not on principle, but because the arithmetic is small enough to read and
//! because every added dependency is a replay-affecting input.
//!
//! # The rule
//!
//! **The camera is `f64`, and geometry reaches the GPU already relative to it.**
//! The world is `i32` voxels out to ±2,000,000 m. An `f32` metre has 24 bits of
//! mantissa: at 2,000,000 m its spacing is 0.25 m, which is a whole subvoxel. So
//! the chunk origin is added in `f64` and the eye is subtracted in `f64`, and
//! only the *difference* is narrowed to `f32`. That difference is the size of
//! what is on screen, so it keeps full precision no matter how far out the
//! player is. `Mesh::corner_positions_m` stays chunk-relative for the same
//! reason (`LEDGER.md`), and this is the other half of that decision.

/// **How a camera projects its frame.** The map window's camera is orthographic: a world map is
/// read by scale, so two cells of the same size must stay the same size wherever they stand in the
/// view. The first-person camera is perspective and is unchanged.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Projection {
    /// The lens: a fov and an aspect. Every game frame before the map surface.
    Perspective,
    /// **A stated half-height in metres**, whatever the target's pixels are. The width follows
    /// from the aspect, so the map's scale is one number and never a second fov.
    Orthographic {
        /// Half the view's height at the camera's plane, metres.
        half_height_m: f64,
    },
}

/// A camera with a yaw/pitch orientation, in metres.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Camera {
    /// Eye position in **world** metres.
    pub position_m: [f64; 3],
    /// Rotation about +Y. `0.0` looks down `+Z`.
    pub yaw_rad: f64,
    /// Positive looks up. Clamped to just under a right angle so the up vector
    /// never collapses.
    pub pitch_rad: f64,
    /// Rotation about the view direction, in radians. `0.0` is upright.
    ///
    /// **Positive banks the camera to the left** — `up` tilts toward `-right`. It is
    /// the convention [`crate::camera_overhaul`]'s roll offsets are added in, and it
    /// is the one place those effects attach to geometry. The player's own look never
    /// writes it: the stored camera is upright and only the **drawn** camera carries a
    /// roll.
    pub roll_rad: f64,
    /// Vertical field of view.
    pub fov_y_rad: f64,
    pub aspect: f64,
    pub near_m: f64,
    pub far_m: f64,
    /// **The projection this frame is drawn through.** [`Projection::Perspective`] is the lens
    /// every game frame uses; an orthographic projection is the map's.
    pub projection: Projection,
}

/// 89.9°, and it is a **platform** number rather than a design one: a look-at
/// basis is undefined when the view direction is parallel to the up vector, so
/// pitch has to stop short of the pole. `MD1 rule` does not apply to camera
/// geometry, and this is labelled a platform constraint for the four-bucket
/// rule's sake.
pub const MAX_PITCH_RAD: f64 = 1.569_051_2;

/// **The far clip plane, metres — the clouds' own declared reach, read rather than retyped.**
///
/// It was `1024`, and that number was chosen for a frame whose **only** geometry was terrain: the
/// widest horizon in the tree is `voxel_stream::horizons`' `Horizon::Field` at 96 chunks = 3,072 m,
/// and the render horizon the window actually streams is 8 chunks = **256 m**, so nothing on screen
/// was ever within sight of a 1 km plane.
///
/// **A cloud is not inside a horizon, and it is above the viewer.** The slab's base is around 120 m
/// and its top is the world's tropopause — every column is capped there — on a field spanning
/// 8,192 m and following the eye, so the furthest cloud there can be is the field's own corner with
/// the tropopause above it, and that is the reach this plane is set to.
///
/// The default is therefore **derived**, not chosen twice: it is the furthest thing the frame can
/// draw, so the plane cannot be narrower than what it is meant to show, and a caller that wants
/// another plane sets [`Camera::far_m`].
/// A test holds the two together, and the depth cost of the wider range is nil — the depth
/// derivative at a given distance is `near / d²` to first order and does not read `far` at all, so
/// a 32 m chunk is resolved to millimetres at 256 m exactly as it was at `1024`.
///
/// **The plane is also where a drawn astronomical body is plotted, and that is not where the body
/// stands.** Its own distance is the world's own ([`crate::sun`] carries the figures through
/// `voxel_shapers::scale`): 4.805e7 m for the Moon and 1.87e10 m for the Sun. No `[0,1]`
/// `Depth32Float` buffer can hold those — at 1.87e10 m the depth is `1 − near/d = 5.3e-12`, which
/// rounds to the plane's own `1.0` in `f32`, and a body out there is discarded by a `Less` test. So
/// `vs_body` plots the record's **direction** at [`BODY_PLOT_M`] and scales the half-extent by the same
/// ratio, which leaves the subtended angle exactly the world's own — the claim `renderer`'s body
/// test makes per pixel, and one that holds at any plot distance.
///
/// **And the ladder is now the furthest thing drawn, so the plane is the further of the two.** The
/// surface rungs ([`crate::lod`]) draw ground out to `8 589 935 km`, and geometry past this plane
/// is discarded whatever its own tier says — a ladder that reaches the Moon and a plane at 12.5 km
/// would draw one rung and cut the other twenty-three. [`crate::lod::LADDER_FAR_M`] is twice the
/// top rung's own outer radius, so the furthest cell stands well inside a plane that discards what
/// is exactly on it; the cloud field's 12 500 m is *inside* that, so the sky is still held. The two
/// are held together by `the_far_plane_reaches_the_cloud_fields_own_corner` (the sky) and
/// `surface::tests::the_top_of_the_ladder_is_drawn` (the ground).
pub const FAR_M: f64 = crate::lod::LADDER_FAR_M;

/// **How far a drawn astronomical body is plotted**, metres — [`FAR_M`]'s own reach, exactly.
///
/// **It is not a distance the body stands at.** The record carries the world's own engine distance
/// (4.805e7 m for the Moon, 1.87e10 m for the Sun); `vs_body` plots that direction here and scales the
/// half-extent by the same ratio, so what a body subtends is the world's own angle at *any* plot.
///
/// **And it is the plane's own number because the depth buffer cannot hold a body's.** At 1.87e10 m an
/// `f32` depth is exactly `1.0`, so `vs_body` writes the plane's depth by hand — `clip.z = clip.w`, which
/// is the plane exactly and carries no rounding — and the body's pipeline tests `LessEqual`. That is what
/// makes a body at the plane drawn wherever the frame drew nothing, and a hill in front of it still hide
/// it. The first attempt plotted a thousandth inside the plane and tested `Less`: the depth there is
/// `1 − n/(f·(1−ε)) ≈ 1 − 8e-9`, inside `f32`'s own step at `1.0`, so the disc appeared only where the
/// corners' rounding happened to land below the clear value — a Moon cut into a crescent by arithmetic.
pub const BODY_PLOT_M: f64 = FAR_M;

impl Camera {
    /// The first-person camera a client starts with.
    pub fn standard(position_m: [f64; 3], aspect: f64) -> Self {
        Camera {
            position_m,
            yaw_rad: 0.0,
            pitch_rad: 0.0,
            roll_rad: 0.0,
            fov_y_rad: 70f64.to_radians(),
            aspect,
            near_m: 0.1,
            far_m: FAR_M,
            projection: Projection::Perspective,
        }
    }

    /// **The map's camera**: orthographic, looking down at `pitch_rad` so the horizon is in the
    /// frame rather than a plan drawn from directly above.
    ///
    /// The eye is the **centre of the view**, not a stand point: an orthographic projection has no
    /// vanishing point to walk toward, so moving the eye slides the picture rather than changing
    /// what is in it. `yaw_rad` 0 looks down +Z, and a negative pitch looks down.
    pub fn orthographic(
        position_m: [f64; 3],
        aspect: f64,
        half_height_m: f64,
        yaw_rad: f64,
        pitch_rad: f64,
    ) -> Self {
        Camera {
            position_m,
            yaw_rad,
            pitch_rad: pitch_rad.clamp(-MAX_PITCH_RAD, MAX_PITCH_RAD),
            roll_rad: 0.0,
            fov_y_rad: 0.0,
            aspect,
            near_m: 1.0,
            far_m: FAR_M,
            projection: Projection::Orthographic {
                half_height_m: half_height_m.max(0.001),
            },
        }
    }

    /// The world units one pixel covers at this camera's plane. **The map's pattern scale**, and
    /// for a perspective camera the angular size [`crate::pattern::pixel_angle_rad`] answers;
    /// an orthographic frame has no angle, so it states its own metres per pixel.
    pub fn metres_per_pixel(&self, height_px: u32) -> f64 {
        match self.projection {
            Projection::Perspective => {
                2.0 * (self.fov_y_rad / 2.0).tan() / f64::from(height_px.max(1))
            }
            Projection::Orthographic { half_height_m } => {
                2.0 * half_height_m / f64::from(height_px.max(1))
            }
        }
    }

    /// Unit direction the camera looks along, in world space.
    pub fn forward(&self) -> [f64; 3] {
        let (sy, cy) = self.yaw_rad.sin_cos();
        let (sp, cp) = self.pitch_rad.sin_cos();
        [sy * cp, sp, cy * cp]
    }

    /// The orthonormal basis `(right, up, forward)`.
    ///
    /// The pitch clamp is applied here rather than stored, so a caller that
    /// writes a raw pitch still gets a usable basis instead of a `NaN`.
    pub fn basis(&self) -> ([f64; 3], [f64; 3], [f64; 3]) {
        let pitch = self.pitch_rad.clamp(-MAX_PITCH_RAD, MAX_PITCH_RAD);
        let (sy, cy) = self.yaw_rad.sin_cos();
        let (sp, cp) = pitch.sin_cos();
        let forward = [sy * cp, sp, cy * cp];
        // `right = world_up x forward`, not `forward x world_up`.
        //
        // The order matters and the first version of this had it the other way:
        // `forward x up` at yaw zero gives `-X`, and then `right x up` is
        // `-forward` — a **left-handed** set, which the orthonormality test
        // caught. With `up x forward` the triple is right-handed: at yaw zero it
        // is exactly `(X, Y, Z)`.
        let right = cross([0.0, 1.0, 0.0], forward);
        let right = normalise(right).unwrap_or([1.0, 0.0, 0.0]);
        let up = cross(forward, right);
        let up = normalise(up).unwrap_or([0.0, 1.0, 0.0]);
        let (right, up) = rolled(right, up, self.roll_rad);
        (right, up, forward)
    }

    /// Camera-relative world point → clip space, ready for WGSL as `mat4x4<f32>`.
    ///
    /// Column-major, because that is what a WGSL matrix is. The view matrix
    /// carries **no translation**: positions handed to the GPU are already
    /// relative to the eye, so the view is a pure rotation.
    pub fn view_projection(&self) -> [[f32; 4]; 4] {
        let (right, up, forward) = self.basis();

        // a point p, camera-relative → camera space, right-handed, looking down -Z
        let view = [
            [right[0] as f32, up[0] as f32, -forward[0] as f32, 0.0],
            [right[1] as f32, up[1] as f32, -forward[1] as f32, 0.0],
            [right[2] as f32, up[2] as f32, -forward[2] as f32, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];

        // **One projection, chosen by the frame's own declaration.** wgpu clip space is depth
        // 0…1, not -1…1, in both arms.
        let projection = match self.projection {
            Projection::Perspective => {
                let f = 1.0 / (self.fov_y_rad / 2.0).tan();
                let nf = 1.0 / (self.near_m - self.far_m);
                [
                    [(f / self.aspect) as f32, 0.0, 0.0, 0.0],
                    [0.0, f as f32, 0.0, 0.0],
                    [0.0, 0.0, (self.far_m * nf) as f32, -1.0],
                    [0.0, 0.0, (self.near_m * self.far_m * nf) as f32, 0.0],
                ]
            }
            Projection::Orthographic { half_height_m } => {
                let half = half_height_m.max(0.001);
                let half_w = half * self.aspect.max(0.001);
                // View space looks down −Z: `z = −near` answers 0 and `z = −far` answers 1.
                let span = (self.far_m - self.near_m).max(0.001);
                [
                    [(1.0 / half_w) as f32, 0.0, 0.0, 0.0],
                    [0.0, (1.0 / half) as f32, 0.0, 0.0],
                    [0.0, 0.0, (-1.0 / span) as f32, 0.0],
                    [0.0, 0.0, (-self.near_m / span) as f32, 1.0],
                ]
            }
        };

        multiply(projection, view)
    }
}

/// Rotate the camera's `(right, up)` pair about the view direction.
///
/// Positive `roll_rad` tilts `up` toward `-right`: the camera **banks left**, and the
/// world therefore appears to rotate clockwise. See [`Camera::roll_rad`].
fn rolled(right: [f64; 3], up: [f64; 3], roll_rad: f64) -> ([f64; 3], [f64; 3]) {
    if roll_rad == 0.0 {
        return (right, up);
    }
    let (sin, cos) = roll_rad.sin_cos();
    let mut rotated_right = [0.0; 3];
    let mut rotated_up = [0.0; 3];
    for axis in 0..3 {
        rotated_right[axis] = right[axis] * cos + up[axis] * sin;
        rotated_up[axis] = up[axis] * cos - right[axis] * sin;
    }
    (rotated_right, rotated_up)
}

/// Column-major `a · b`.
fn multiply(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut out = [[0.0f32; 4]; 4];
    for (c, column) in out.iter_mut().enumerate() {
        for (r, cell) in column.iter_mut().enumerate() {
            let mut sum = 0.0f32;
            for k in 0..4 {
                sum += a[k][r] * b[c][k];
            }
            *cell = sum;
        }
    }
    out
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// `None` rather than a default when the vector is degenerate: a zero-length
/// basis vector is not a basis, and inventing `+X` for it would hide the fact
/// that the orientation is broken.
fn normalise(v: [f64; 3]) -> Option<[f64; 3]> {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if !len.is_finite() || len == 0.0 {
        return None;
    }
    Some([v[0] / len, v[1] / len, v[2] / len])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(cam: &Camera, p: [f64; 3]) -> [f64; 4] {
        let m = cam.view_projection();
        let rel = [
            p[0] - cam.position_m[0],
            p[1] - cam.position_m[1],
            p[2] - cam.position_m[2],
        ];
        // column-major, so m[col][row]
        let mut out = [0.0f64; 4];
        for (r, cell) in out.iter_mut().enumerate() {
            *cell = (0..4)
                .map(|k| m[k][r] as f64 * if k == 3 { 1.0 } else { rel[k] })
                .sum();
        }
        out
    }

    #[test]
    fn yaw_zero_looks_down_positive_z() {
        let cam = Camera::standard([0.0, 0.0, 0.0], 1.0);
        assert_eq!(cam.forward(), [0.0, 0.0, 1.0]);
    }

    #[test]
    fn the_basis_is_orthonormal_and_right_handed() {
        let mut cam = Camera::standard([0.0, 0.0, 0.0], 1.0);
        for (yaw, pitch, roll) in [(0.3, 0.2, 0.0), (-1.1, -0.4, 0.5), (2.7, 0.9, -1.2)] {
            cam.yaw_rad = yaw;
            cam.pitch_rad = pitch;
            cam.roll_rad = roll;
            let (r, u, f) = cam.basis();
            let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
            for v in [r, u, f] {
                assert!((dot(v, v) - 1.0).abs() < 1e-12, "not unit: {v:?}");
            }
            assert!(dot(r, u).abs() < 1e-12);
            assert!(dot(r, f).abs() < 1e-12);
            assert!(dot(u, f).abs() < 1e-12);
            // right x up == forward is the right-handed statement.
            let c = cross(r, u);
            for i in 0..3 {
                assert!((c[i] - f[i]).abs() < 1e-12, "not right-handed");
            }
        }
    }

    /// **The roll convention, pinned**: positive roll banks the camera left, so the
    /// up vector tilts toward `-right` and the right vector tilts up. The camera
    /// effects are added in this convention, and they cannot be read back from a
    /// screenshot in a test, so the sign is fixed here.
    #[test]
    fn a_positive_roll_banks_the_camera_to_the_left() {
        let mut cam = Camera::standard([0.0, 0.0, 0.0], 1.0);
        cam.roll_rad = 0.3;
        let (right, up, forward) = cam.basis();
        assert!(up[0] < 0.0, "up tilted toward {:.3} on X, not left", up[0]);
        assert!(
            right[1] > 0.0,
            "right tilted toward {:.3} on Y, not up",
            right[1]
        );
        // Rolling about the view direction does not move the view direction.
        assert_eq!(forward, [0.0, 0.0, 1.0]);
        // And a zero roll is exactly the unrolled basis.
        let upright = Camera::standard([0.0, 0.0, 0.0], 1.0);
        assert_eq!(
            upright.basis(),
            ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0])
        );
    }

    #[test]
    fn a_point_straight_ahead_lands_at_the_centre_of_the_screen() {
        let cam = Camera::standard([0.0, 0.0, 0.0], 1.0);
        let clip = project(&cam, [0.0, 0.0, 10.0]);
        assert!(clip[3] > 0.0, "the point must be in front of the eye");
        assert!(clip[0].abs() < 1e-9, "x was {}", clip[0]);
        assert!(clip[1].abs() < 1e-9, "y was {}", clip[1]);
    }

    #[test]
    fn the_depth_range_is_zero_to_one_and_never_reversed() {
        let cam = Camera::standard([0.0, 0.0, 0.0], 1.0);
        let at_near = project(&cam, [0.0, 0.0, cam.near_m]);
        let at_far = project(&cam, [0.0, 0.0, cam.far_m]);
        let z_near = at_near[2] / at_near[3];
        let z_far = at_far[2] / at_far[3];
        assert!(z_near.abs() < 1e-5, "near mapped to {z_near}");
        assert!((z_far - 1.0).abs() < 1e-5, "far mapped to {z_far}");
        // and monotonic in between
        let mid = project(&cam, [0.0, 0.0, 5.0]);
        let z_mid = mid[2] / mid[3];
        assert!(z_near < z_mid && z_mid < z_far);
    }

    /// **A drawn body is plotted inside the far plane, and the angle it subtends is the world's own.**
    ///
    /// The record carries the body at the distance `voxel_shapers::scale`'s ruling gives it — far
    /// outside the plane — and `vs_body` plots that direction at [`FAR_M`] with the half-extent scaled
    /// by the same ratio. This holds both halves: the plotted body projects at the plane's own depth —
    /// the value the vertex stage writes by hand, so no rounding can strand it — and the angle it
    /// subtends is the world's own figure, which is also the real one, because the ruling divides radius
    /// and distance by the same eight.
    #[test]
    fn a_drawn_body_is_plotted_on_the_far_plane_and_keeps_the_angle_it_has() {
        use voxel_shapers::scale;

        let cam = Camera::standard([0.0, 0.0, 0.0], 1.0);
        // **The plot is the projection's own plane**, so the depth `vs_body` writes by hand is the
        // projection's own the body's pipeline tests against.
        assert_eq!(
            BODY_PLOT_M, FAR_M,
            "a body plotted at {BODY_PLOT_M} m is not on the {FAR_M} m plane "
        );
        let clip = project(&cam, [0.0, 0.0, BODY_PLOT_M]);
        assert!(clip[3] > 0.0, "the plotted body is behind the eye");
        let z = clip[2] / clip[3];
        // **And this is the defect the constant exists to survive, in one number.** The projection's own
        // depth at the plane is `1.0` to within half of `f32`'s step at `1.0` — measured here at
        // `0.999999987`, which is `1.3e-8` under it against a step of `6e-8` — so a depth *computed* for a
        // body on the plane arrives as exactly the value the pass clears depth to, and a `Less` test
        // discards it. That is why no frame had a Moon in it. `vs_body` therefore writes `clip.w` — the
        // plane exactly, with no rounding — and the body's pipeline tests `LessEqual`, so a body at the
        // plane is drawn wherever the frame drew nothing while a hill in front of it still hides it.
        assert!(
            1.0 - z < f32::EPSILON as f64,
            "the projection's own depth at the {FAR_M:.0} m plane is {z}, which is more than half an f32 \
             step ({} below 1.0), so a computed depth there would be distinguishable after all and this \
             test is no longer evidence for the hand-written one",
            f32::EPSILON as f64
        );
        // The Moon's own figures (IAU radius, mean Earth-Moon distance), through the world's ruling
        // and then through the plot: the same number from both orders.
        let (real_half_m, real_distance_m) = (1_737_400.0, 3.844e8);
        let half_m = scale::voxels(real_half_m);
        let distance_m = scale::voxels(real_distance_m);
        let plotted_half_m = half_m * FAR_M / distance_m;
        let plotted_angle = 2.0 * (plotted_half_m / FAR_M).atan().to_degrees();
        let real_angle = 2.0 * (real_half_m / real_distance_m).atan().to_degrees();
        println!(
            "plotted at {FAR_M:.0} m: a {plotted_half_m:.2} m half-extent from an engine half-extent \
             of {half_m:.0} at {distance_m:.3e} m subtends {plotted_angle:.4} deg against the Moon's \
             own {real_angle:.4} deg"
        );
        assert!(
            (plotted_angle - real_angle).abs() < 1e-9,
            "the plotted body subtends {plotted_angle} deg where the world's own figures say \
             {real_angle}"
        );
        // **And the Sun's own disc, which is the same rule at a thousand times the distance.**
        let (sun_half_m, sun_distance_m) = (6.957e8, 1.495_978_707e11);
        let sun_plotted = scale::voxels(sun_half_m) * FAR_M / scale::voxels(sun_distance_m);
        let sun_angle = 2.0 * (sun_plotted / FAR_M).atan().to_degrees();
        assert!(
            (sun_angle - 0.533).abs() < 0.01,
            "the plotted Sun subtends {sun_angle} deg, not the half a degree it does"
        );
        // **And the reason the plot is not the body's own distance.** A body out at its own engine
        // distance cannot be held by a `[0,1]` `Depth32Float` buffer at all: `1 − near/d` is smaller
        // than one `f32` step from the plane's own `1.0`, so the depth test discards it. Stated as a
        // measurement rather than as a claim about the shader, and it is why this plane is the plot.
        let step = (f32::EPSILON / 2.0) as f64;
        let moon_depth = (1.0f32 - 0.1f32 / distance_m as f32) as f64;
        assert!(
            (1.0 - moon_depth) < step,
            "the Moon's own distance did fit the depth buffer after all: `1 − near/d` is {:.3e} \
             against one step of {step:.3e} near the plane, so the plot is avoidable noise",
            1.0 - moon_depth
        );
    }

    /// **The far plane reaches the clouds, measured on the shipped field.** The plane's own
    /// comment says why it is not `1024` any more; this is the check that it stays wide enough,
    /// on the field the game actually samples: the furthest cell the frame can draw is a corner of
    /// the slab, and it has to be in front of the plane or the cloud layer simply ends as the
    /// camera pitches up.
    #[test]
    fn the_far_plane_reaches_the_cloud_fields_own_corner() {
        use crate::cloud_field::{CloudField, CLOUD_SPAN_M};
        use voxel_shapers::{Config, WorldSpec};

        let cam = Camera::standard([0.0, 0.0, 0.0], 1.6);
        let spec = WorldSpec::new(crate::GAME_SEED, Config::default());
        let half = CLOUD_SPAN_M / 2.0;
        // Centred on the eye, so the corner is `half` away on both horizontal axes — the furthest
        // a drawn cell can be — with the field's own measured slab top above it.
        let field = CloudField::sample(&spec, crate::GAME_TICK, [0.0, 0.0]);
        assert!(
            field.cloudy > 0,
            "the shipped field is clear, so this proves nothing"
        );
        let top = field.slab_top_m.max(0.0);
        let reach = (half * half + top * top + half * half).sqrt();
        // **The bound is the declared sky, not this tick's weather**: a column is capped at the tropopause.
        let declared =
            (half * half + voxel_shapers::clouds::TROPOPAUSE_M.powi(2) + half * half).sqrt();
        assert!(
            cam.far_m >= declared,
            "the default far plane is {:.0} m and a column at the tropopause is {declared:.0} m away, \
             so the sky is clipped as the camera pitches up",
            cam.far_m
        );
        println!(
            "far plane {:.0} m; the shipped field's slab top is {top:.0} m, its far corner \
             {reach:.0} m from the eye, and a column at the tropopause would be {declared:.0} m away",
            cam.far_m
        );
        // **The claim is about clipping, so it is made on the projection** and not on two lengths:
        // the corner has to be in front of the eye and inside the far plane, or the pipeline
        // discards it whatever the distance arithmetic says.
        let clip = project(&cam, [half, top, half]);
        assert!(clip[3] > 0.0, "the field's far corner is behind the eye");
        let z = clip[2] / clip[3];
        assert!(
            z < 1.0,
            "the field's far corner is past the far plane: z {z} with a {reach:.0} m reach and a \
             {:.0} m plane",
            cam.far_m
        );
        assert!(
            cam.far_m >= reach,
            "the far plane is {:.0} m and the field's far corner is {reach:.0} m away",
            cam.far_m
        );
        // And the plane this replaced was inside that reach — which is the defect: a cumulus
        // overhead is 2 km of view distance against a 1,024 m plane.
        assert!(
            1024.0 < reach,
            "the shipped field reaches {reach:.0} m, so the old plane would have held it"
        );
    }

    #[test]
    fn moving_the_eye_far_out_does_not_move_the_picture() {
        // The whole point of camera-relative geometry: the same *relative*
        // arrangement must produce the same clip coordinates at the origin and
        // at two million metres. An `f32` world would fail this.
        let near = Camera::standard([0.0, 0.0, 0.0], 1.6);
        let far = Camera::standard([2_000_000.0, 1_200_000.0, -2_000_000.0], 1.6);
        let a = project(&near, [1.0, 2.0, 10.0]);
        let b = project(&far, [2_000_001.0, 1_200_002.0, -1_999_990.0]);
        for i in 0..4 {
            assert!((a[i] - b[i]).abs() < 1e-6, "component {i}: {a:?} vs {b:?}");
        }
    }

    #[test]
    fn a_point_behind_the_eye_has_a_negative_w() {
        let cam = Camera::standard([0.0, 0.0, 0.0], 1.0);
        assert!(project(&cam, [0.0, 0.0, -1.0])[3] < 0.0);
    }
}
