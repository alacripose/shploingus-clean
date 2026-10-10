//! The fluid surface: **what the sky does when it meets water.**
//!
//! The user's ruling (`LEDGER.md` C-157.6) is the reason this module exists:
//! *"the sun and it's moon refletions is atually core to the rendering of the world
//! itself, and the materials"*. A rock is lit **by** the sky; a water surface **is**
//! the sky, seen off a dielectric boundary — so the reflection of the directional sky
//! is not a decoration on the fluid, it is most of the fluid's appearance. A fluid
//! drawn without it is a matte sheet, which is what a reported defect looked like:
//! the sea was there and read as a hole, because nothing about it said *water*.
//!
//! # What is here, and what the caller brings
//!
//! This module owns exactly three things:
//!
//! * [`reflected_ray`] — the mirror direction, from the view and the normal. Pure
//!   geometry, so it is the one place the reflection's geometry can be wrong.
//! * [`schlick`] — the share of the radiance the boundary reflects at one angle,
//!   `F(cos θ)`, Schlick's approximation (Schlick 1994) at the same exponent the
//!   rim term uses.
//! * [`surface_radiance`] — the mix: `F · L_reflected + (1 − F) · body`.
//!
//! The caller brings `L_reflected` — the radiance arriving **from the mirror
//! direction** — and in this repository that is read from the same directional sky
//! table the sky pass draws (`voxel_client::skylight::sky_dome_lookup`). That is
//! deliberate: one sky, one table, one evaluation (C-157.6). It is also why this
//! module never mentions the atmosphere.
//!
//! # Absent, not simulated
//!
//! Refraction, absorption and the volume below the surface are **not** here. The
//! transmitted side is the caller's `body` radiance; a depth-dependent body needs
//! `transmission_depth`, which the material record refuses rather than defaults.
//! The mirror ray can also point below the horizon — at a steep face the water
//! reflects the world, not the sky — and the caller decides what that means; the
//! client treats it as no reflection rather than as a bright horizon.
//!
//! # The numbers
//!
//! [`WATER_IOR`] and [`WATER_F0`] live in [`crate::provenance`] with every other
//! constant, and are re-exported here so a caller has one import for the fluid's
//! physics.

use crate::openpbr::LinearRgb;
use crate::provenance::RIM_SCHLICK_EXPONENT;
use voxel_noise::Vec3;

pub use crate::provenance::{WATER_F0, WATER_IOR};

/// The mirror direction of `view` about `normal`, unit length.
///
/// `view` points from the surface **toward the eye**, the convention
/// [`crate::npr::SurfaceFrame`] uses, so the incident ray is `−view` and the
/// reflection is `−view + 2·(view·normal)·normal` — WGSL's own `reflect` arithmetic
/// with an incident vector of `−view`. The client's shader spells the same
/// expression, and the GPU test that draws water and compares it against this
/// module is what holds the two together.
pub fn reflected_ray(view: Vec3, normal: Vec3) -> Vec3 {
    let (v, n) = (view.normalise(), normal.normalise());
    (-v + n * (2.0 * v.dot(n))).normalise()
}

/// Fresnel reflectance at `cos_theta`, by Schlick's approximation:
///
/// ```text
/// F(cos θ) = F0 + (1 − F0) · (1 − cos θ)⁵
/// ```
///
/// `cos_theta` is the cosine between the view ray and the surface normal, clamped
/// into `[0, 1]` — the declared domain of the parameter, the same reason
/// `npr::shade` clamps. The exponent is the one in [`crate::provenance`], shared
/// with the rim term: the same approximation at the same power, applied to a
/// different boundary.
pub fn schlick(cos_theta: f64, f0: f64) -> f64 {
    let c = cos_theta.clamp(0.0, 1.0);
    f0 + (1.0 - f0) * (1.0 - c).powi(RIM_SCHLICK_EXPONENT)
}

/// **The radiance leaving a fluid surface**: the sky reflected by Fresnel, over
/// the body the surface transmits.
///
/// ```text
/// L = F(cos θ) · L_reflected + (1 − F(cos θ)) · body
/// ```
///
/// The two ends are the two physical limits and both are checked by tests: head-on
/// the surface is `2 %` mirror and `98 %` body; at grazing incidence it is a mirror.
/// That is the behaviour a flat matte sheet cannot express, and it is the whole
/// reason water reads as water as the player walks past it.
pub fn surface_radiance(body: LinearRgb, reflected: LinearRgb, cos_theta: f64) -> LinearRgb {
    let f = schlick(cos_theta, WATER_F0);
    body * (1.0 - f) + reflected * f
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reflected_ray_is_the_mirror_of_the_view() {
        // **What a mirror does, asserted geometrically.** The incident ray (`−view`)
        // arrives down-and-south and leaves up-and-south: the vertical flips, the
        // horizontal does not.
        let view = Vec3::new(0.0, 1.0, -1.0).normalise();
        let up = Vec3::new(0.0, 1.0, 0.0);
        let r = reflected_ray(view, up);
        assert!(
            (r.length() - 1.0).abs() < 1e-12,
            "the ray is not unit: {r:?}"
        );
        assert!(
            (r.y - view.y).abs() < 1e-12,
            "the mirror ray should keep the eye-side vertical: {r:?}"
        );
        assert!(
            (r.z + view.z).abs() < 1e-12,
            "the horizontal direction should reverse: {r:?}"
        );
        assert!(
            r.y > 0.0,
            "a mirror ray seen from above must point at the sky: {r:?}"
        );
        // The angle of incidence equals the angle of reflection.
        let (a, b) = (r.dot(up).acos(), view.dot(up).acos());
        assert!((a - b).abs() < 1e-12, "incidence {b} != reflection {a}");
    }

    #[test]
    fn a_vertical_mirror_sends_the_ray_back_along_the_view() {
        // The degenerate case that catches a sign error: a mirror facing the eye
        // reflects the ray straight back at it.
        let view = Vec3::new(0.0, 0.0, -1.0);
        let facing = Vec3::new(0.0, 0.0, -1.0);
        let r = reflected_ray(view, facing);
        assert!((r - view).length() < 1e-12, "expected {view:?}, got {r:?}");
    }

    #[test]
    fn the_fresnel_ends_are_the_two_physical_limits() {
        // Head-on: the water's own normal-incidence reflectance, 2 %.
        let head_on = schlick(1.0, WATER_F0);
        assert!((head_on - WATER_F0).abs() < 1e-12, "head-on gave {head_on}");
        assert!(
            (WATER_F0 - 0.0204).abs() < 1e-3,
            "F0 is {WATER_F0}, not the water figure"
        );
        // Grazing: a mirror. The approximation reaches exactly 1 at 90°.
        let grazing = schlick(0.0, WATER_F0);
        assert!((grazing - 1.0).abs() < 1e-12, "grazing gave {grazing}");
        // And it is monotone in between — **falling as the view becomes more
        // head-on** (`cos θ` grows), which is the direction the physics runs: no
        // bump that would read as a band.
        let mut last = f64::INFINITY;
        for i in 0..=100 {
            let f = schlick(i as f64 / 100.0, WATER_F0);
            assert!(f <= last, "not monotone at cos theta = {i}/100");
            last = f;
        }
    }

    #[test]
    fn the_surface_is_the_body_head_on_and_the_sky_at_grazing() {
        let body = LinearRgb::new(0.02, 0.05, 0.10);
        let sky = LinearRgb::new(0.30, 0.45, 0.80);
        // Head-on the fluid is almost all body — transparent-looking, as water is.
        let head = surface_radiance(body, sky, 1.0);
        assert!(
            (head.luminance() - body.luminance()).abs() < 0.01,
            "head-on the surface should be nearly the body: {head:?}"
        );
        // Grazing it is nearly all sky — the mirror a player sees along the water.
        let grazing = surface_radiance(body, sky, 0.0);
        assert!(
            (grazing.luminance() - sky.luminance()).abs() < 0.01,
            "grazing the surface should be nearly the sky: {grazing:?}"
        );
        // And the mix is a blend, not a swap: the middle is between the two.
        let middle = surface_radiance(body, sky, 0.5);
        assert!(middle.luminance() > head.luminance() && middle.luminance() < grazing.luminance());
    }
}
