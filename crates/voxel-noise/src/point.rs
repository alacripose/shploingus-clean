//! A minimal `f64` vector.
//!
//! Nine operations in forty lines rather than a dependency: the world generator
//! needs `+`, `-`, `*`, `dot`, `length` and a couple of constructors, and a
//! vector library is a large surface to add to a crate whose whole argument is
//! that its arithmetic is auditable. This is also why `glam` was refused in the
//! tree this one serves.

use core::ops::{Add, AddAssign, Div, Mul, Neg, Sub};

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub const ZERO: Vec3 = Vec3 { x: 0.0, y: 0.0, z: 0.0 };

    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Vec3 { x, y, z }
    }

    pub const fn splat(v: f64) -> Self {
        Vec3 { x: v, y: v, z: v }
    }

    /// A voxel position as a sample point. The `+ 0.5` is deliberate and belongs
    /// here rather than at each call site: a voxel's field value is its **centre**,
    /// and a generator that samples corners produces a world offset by half a
    /// block from the one the mesher draws.
    pub fn at_voxel(x: i32, y: i32, z: i32) -> Self {
        Vec3 { x: x as f64 + 0.5, y: y as f64 + 0.5, z: z as f64 + 0.5 }
    }

    pub fn dot(self, o: Vec3) -> f64 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }

    pub fn length(self) -> f64 {
        self.dot(self).sqrt()
    }

    pub fn normalise(self) -> Self {
        let l = self.length();
        if l == 0.0 {
            Vec3::ZERO
        } else {
            self / l
        }
    }

    pub fn lerp(self, o: Vec3, t: f64) -> Self {
        self + (o - self) * t
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }
}

impl Add for Vec3 {
    type Output = Vec3;
    fn add(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}

impl AddAssign for Vec3 {
    fn add_assign(&mut self, o: Vec3) {
        *self = *self + o;
    }
}

impl Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}

impl Mul<f64> for Vec3 {
    type Output = Vec3;
    fn mul(self, s: f64) -> Vec3 {
        Vec3::new(self.x * s, self.y * s, self.z * s)
    }
}

impl Div<f64> for Vec3 {
    type Output = Vec3;
    fn div(self, s: f64) -> Vec3 {
        Vec3::new(self.x / s, self.y / s, self.z / s)
    }
}

impl Neg for Vec3 {
    type Output = Vec3;
    fn neg(self) -> Vec3 {
        Vec3::new(-self.x, -self.y, -self.z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voxel_sample_points_are_at_the_centre() {
        assert_eq!(Vec3::at_voxel(0, 0, 0), Vec3::new(0.5, 0.5, 0.5));
        assert_eq!(Vec3::at_voxel(-1, 2, -3), Vec3::new(-0.5, 2.5, -2.5));
    }

    #[test]
    fn arithmetic_is_the_obvious_thing() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 5.0, 6.0);
        assert_eq!(a + b, Vec3::new(5.0, 7.0, 9.0));
        assert_eq!(b - a, Vec3::new(3.0, 3.0, 3.0));
        assert_eq!(a * 2.0, Vec3::new(2.0, 4.0, 6.0));
        assert_eq!(a.dot(b), 32.0);
        assert!((a.length() - 14.0f64.sqrt()).abs() < 1e-12);
    }

    #[test]
    fn normalising_zero_is_zero_rather_than_nan() {
        // 0/0 is NaN, and a NaN normal propagates into a NaN world silently.
        assert_eq!(Vec3::ZERO.normalise(), Vec3::ZERO);
        assert!(Vec3::ZERO.normalise().is_finite());
    }
}
