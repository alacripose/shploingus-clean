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

/// A perspective camera with a yaw/pitch orientation, in metres.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Camera {
    /// Eye position in **world** metres.
    pub position_m: [f64; 3],
    /// Rotation about +Y. `0.0` looks down `+Z`.
    pub yaw_rad: f64,
    /// Positive looks up. Clamped to just under a right angle so the up vector
    /// never collapses.
    pub pitch_rad: f64,
    /// Vertical field of view.
    pub fov_y_rad: f64,
    pub aspect: f64,
    pub near_m: f64,
    pub far_m: f64,
}

/// 89.9°, and it is a **platform** number rather than a design one: a look-at
/// basis is undefined when the view direction is parallel to the up vector, so
/// pitch has to stop short of the pole. `MD1 rule` does not apply to camera
/// geometry, and this is labelled a platform constraint for the four-bucket
/// rule's sake.
pub const MAX_PITCH_RAD: f64 = 1.569_051_2;

impl Camera {
    /// The first-person camera a client starts with.
    pub fn standard(position_m: [f64; 3], aspect: f64) -> Self {
        Camera {
            position_m,
            yaw_rad: 0.0,
            pitch_rad: 0.0,
            fov_y_rad: 70f64.to_radians(),
            aspect,
            near_m: 0.1,
            far_m: 1024.0,
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
        (right, normalise(up).unwrap_or([0.0, 1.0, 0.0]), forward)
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

        let f = 1.0 / (self.fov_y_rad / 2.0).tan();
        let nf = 1.0 / (self.near_m - self.far_m);
        // wgpu clip space is depth 0…1, not -1…1.
        let projection = [
            [(f / self.aspect) as f32, 0.0, 0.0, 0.0],
            [0.0, f as f32, 0.0, 0.0],
            [0.0, 0.0, (self.far_m * nf) as f32, -1.0],
            [0.0, 0.0, (self.near_m * self.far_m * nf) as f32, 0.0],
        ];

        multiply(projection, view)
    }
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
        for (yaw, pitch) in [(0.3, 0.2), (-1.1, -0.4), (2.7, 0.9)] {
            cam.yaw_rad = yaw;
            cam.pitch_rad = pitch;
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
