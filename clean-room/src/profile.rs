//! **The shape laws — a body of revolution, with the normal it *has*.**
//!
//! Derived from `docs/DESIGN-SPEC.md` §1.1 (a shape is a **law**, and the 4×4×4 pattern is a *sample*
//! of it) and §2 F3 (the stored lattice is a sample of an analytic form). The published laws named
//! here are the ones `docs/MISSING-SYSTEMS.md` and `LEDGER.md` cite for this project's shapes:
//! **Szymczak, Ladd, Lipar & Pekarovič 2025** for the stalagmite's `Da` law, **Requicha 1980** for
//! the half-space geometry of a flat cap, and the classical surfaces of revolution (cylinder,
//! frustum, spheroid).
//!
//! # The correction this module exists to hold
//!
//! `LEDGER.md` records a real bug: a caller used a **taper's declared radius** where it needed the
//! radius the body **has** at that height — the silhouette. For a spheroid and a stalagmite those are
//! different numbers, and the bug made both read as cylinders. So there is **one** radius method,
//! [`Profile::silhouette_radius`], and it is defined to be the silhouette. A shape with no exact
//! normal is **refused by name** rather than approximated (Constitution P2).
//!
//! # The stalagmite: the law's exact consequences, and a recorded substitution
//!
//! The paper's tangent law `cos θ(r) = Da · √(1 − (r/R)²)` fixes the normal by radius, but its
//! meridian — height as a function of radius — is an **elliptic integral**: no elementary closed
//! form exists, and this module refuses to approximate one (P2). The law's **exact** consequences
//! are kept: the cap radius `R · √(1 − Da⁻²)` where `cos θ` reaches 1, a **radial** normal at the
//! base rim, and a **vertical** normal at the cap rim. Between those points the body is a declared
//! **elliptical flank** whose silhouette and normals are read from the same curve, so they cannot
//! describe two different shapes — an earlier revision drew a linear silhouette while taking the
//! law's normals, and the two disagreed. This substitution is recorded, in the style of C-10, and a
//! test pins the one-body property and asserts the paper's printed cap radius `R · √(1 − Da⁻¹)`
//! does **not** reproduce.

use crate::refusal::RefusalKind;

/// The world's up axis. `z` is up.
pub const UP: [f64; 3] = [0.0, 0.0, 1.0];

/// A body of revolution. The height fraction `t` runs from the base (`0`) to the axis (`1`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Profile {
    /// A flat disc cap of this radius; every normal is the axis.
    Flat { radius: f64 },
    /// A right cylinder: a constant silhouette over its height.
    Cylinder { radius: f64, height: f64 },
    /// A cone frustum: base and top radii over its height.
    Frustum { base_radius: f64, top_radius: f64, height: f64 },
    /// A centred spheroid: the silhouette is its own cap inverted, never the base axis.
    Spheroid { semi_axis_r: f64, semi_axis_z: f64 },
    /// A stalagmite from the Szymczak et al. (2025) tangent law over its `Da` and basal radius.
    Stalagmite { da: f64, radius: f64 },
}

fn normalize(v: [f64; 3]) -> Option<[f64; 3]> {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if !len.is_finite() || len <= 0.0 {
        return None;
    }
    Some([v[0] / len, v[1] / len, v[2] / len])
}

/// The stalagmite's cap radius at the law's own value: where `Da · √(1 − (r/R)²)` reaches `1`.
/// Callers validate first (`Da > 1`), so the radicand is non-negative.
fn stalagmite_cap(da: f64, radius: f64) -> f64 {
    radius * (1.0 - 1.0 / (da * da)).max(0.0).sqrt()
}

impl Profile {
    /// Refuse a shape that is not a shape — a non-finite parameter, a non-positive radius or height,
    /// or a `Da` the stalagmite law is not defined for.
    pub fn validate(&self) -> Result<(), RefusalKind> {
        let positive = |x: f64| x.is_finite() && x > 0.0;
        match *self {
            Profile::Flat { radius } => {
                if !positive(radius) {
                    return Err(RefusalKind::MalformedProfile);
                }
            }
            Profile::Cylinder { radius, height } => {
                if !positive(radius) || !height.is_finite() || height <= 0.0 {
                    return Err(RefusalKind::MalformedProfile);
                }
            }
            Profile::Frustum { base_radius, top_radius, height } => {
                // A cone is the `top_radius = 0` case and is a real shape; a *negative* radius is
                // not, and is refused.
                if !positive(base_radius)
                    || !top_radius.is_finite()
                    || top_radius < 0.0
                    || !height.is_finite()
                    || height <= 0.0
                {
                    return Err(RefusalKind::MalformedProfile);
                }
            }
            Profile::Spheroid { semi_axis_r, semi_axis_z } => {
                if !positive(semi_axis_r) || !positive(semi_axis_z) {
                    return Err(RefusalKind::MalformedProfile);
                }
            }
            Profile::Stalagmite { da, radius } => {
                // The law needs `Da > 1`: at `Da = 1` the cap radius is zero and the tangent law's
                // regime degenerates, and below it there is no cap at all.
                if !positive(radius) || !da.is_finite() || da <= 1.0 {
                    return Err(RefusalKind::MalformedProfile);
                }
            }
        }
        Ok(())
    }

    /// The body's total height. A flat cap has none.
    pub fn height(&self) -> Result<f64, RefusalKind> {
        self.validate()?;
        Ok(match *self {
            Profile::Flat { .. } => 0.0,
            Profile::Cylinder { height, .. } | Profile::Frustum { height, .. } => height,
            Profile::Spheroid { semi_axis_z, .. } => 2.0 * semi_axis_z,
            // The stalagmite's height is declared as its basal radius: the law's own height is the
            // elliptic integral this module refuses to approximate, so it is not claimed.
            Profile::Stalagmite { radius, .. } => radius,
        })
    }

    /// The radius the body **has** at height fraction `t` — the **silhouette**, not a taper's
    /// declared radius. This is the method the recorded bug was about.
    pub fn silhouette_radius(&self, t: f64) -> Result<f64, RefusalKind> {
        self.validate()?;
        if !t.is_finite() || !(0.0..=1.0).contains(&t) {
            return Err(RefusalKind::ParameterOutOfRange);
        }
        Ok(match *self {
            Profile::Flat { radius } => radius,
            Profile::Cylinder { radius, .. } => radius,
            Profile::Frustum { base_radius, top_radius, .. } => {
                base_radius + (top_radius - base_radius) * t
            }
            Profile::Spheroid { semi_axis_r, .. } => {
                // z = semi_z * (2t-1); the silhouette is the ellipse's own cap.
                let u = 2.0 * t - 1.0;
                semi_axis_r * (1.0 - u * u).max(0.0).sqrt()
            }
            Profile::Stalagmite { da, radius } => {
                // The elliptical flank from the base rim (t = 0, r = R) to the law's cap radius
                // (t = 1); the normal below is read from this same curve, exactly.
                let cap = stalagmite_cap(da, radius);
                cap + (radius - cap) * (1.0 - t * t).max(0.0).sqrt()
            }
        })
    }

    /// The **exact** outward unit normal at height fraction `t` and azimuth `phi` (radians).
    pub fn normal(&self, t: f64, phi: f64) -> Result<[f64; 3], RefusalKind> {
        self.validate()?;
        if !t.is_finite() || !(0.0..=1.0).contains(&t) || !phi.is_finite() {
            return Err(RefusalKind::ParameterOutOfRange);
        }
        let (cos_phi, sin_phi) = (phi.cos(), phi.sin());
        match *self {
            // Every point of a flat cap has the axis as its normal.
            Profile::Flat { .. } => Ok(UP),
            // A cylinder's side: purely radial.
            Profile::Cylinder { .. } => Ok([cos_phi, sin_phi, 0.0]),
            Profile::Frustum { base_radius, top_radius, height } => {
                // Meridian tangent (dr/dz, 1); outward normal ∝ (1, −dr/dz).
                let dr_dz = (top_radius - base_radius) / height;
                let n = normalize([1.0, -dr_dz, 0.0]).ok_or(RefusalKind::MalformedProfile)?;
                Ok([n[0] * cos_phi, n[0] * sin_phi, n[1]])
            }
            Profile::Spheroid { semi_axis_r, semi_axis_z } => {
                let u = 2.0 * t - 1.0;
                let z = semi_axis_z * u;
                let r = self.silhouette_radius(t)?;
                // The ellipsoid's gradient at (r, z) is (r / a², 0, z / c²).
                let radial = r / (semi_axis_r * semi_axis_r);
                let axial = z / (semi_axis_z * semi_axis_z);
                let n = normalize([radial, axial, 0.0]).ok_or(RefusalKind::MalformedProfile)?;
                Ok([n[0] * cos_phi, n[0] * sin_phi, n[1]])
            }
            Profile::Stalagmite { da, radius } => {
                // The flank is the ellipse ((r − cap)/(R − cap))² + (z/H)² = 1 with `z = H · t`
                // (`H` the declared height); its gradient is the exact normal of the same curve
                // the silhouette is read from.
                let cap = stalagmite_cap(da, radius);
                let r = self.silhouette_radius(t)?;
                let radial = (r - cap) / ((radius - cap) * (radius - cap));
                let vertical = t / radius;
                let n = normalize([radial, vertical, 0.0]).ok_or(RefusalKind::MalformedProfile)?;
                Ok([n[0] * cos_phi, n[0] * sin_phi, n[1]])
            }
        }
    }

    /// The stalagmite's flat-cap radius, where the law's `cos θ` reaches `1`. `None` for other
    /// shapes, so a caller cannot ask a cylinder for a cap that is a law's property.
    pub fn stalagmite_cap_radius(&self) -> Option<f64> {
        match *self {
            Profile::Stalagmite { da, radius } if da > 1.0 => Some(stalagmite_cap(da, radius)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(v: [f64; 3]) -> bool {
        ((v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt() - 1.0).abs() < 1e-12
    }

    #[test]
    fn a_malformed_profile_is_refused_by_name_and_not_accepted() {
        let bad = [
            Profile::Flat { radius: 0.0 },
            Profile::Cylinder { radius: -1.0, height: 2.0 },
            Profile::Cylinder { radius: 1.0, height: -2.0 },
            Profile::Cylinder { radius: 1.0, height: 0.0 },
            Profile::Frustum { base_radius: 1.0, top_radius: -1.0, height: 1.0 },
            Profile::Spheroid { semi_axis_r: f64::NAN, semi_axis_z: 1.0 },
            Profile::Stalagmite { da: 1.0, radius: 1.0 },
            Profile::Stalagmite { da: 0.5, radius: 1.0 },
        ];
        for profile in bad {
            assert_eq!(
                profile.validate().unwrap_err(),
                RefusalKind::MalformedProfile,
                "{profile:?} must be refused"
            );
            // And a malformed shape cannot be sampled at all.
            assert!(profile.silhouette_radius(0.5).is_err());
            assert!(profile.normal(0.5, 0.0).is_err());
        }
        // A **cone** (`top_radius = 0`) is a real shape and is accepted; a negative one is not.
        assert!(Profile::Frustum { base_radius: 1.0, top_radius: 0.0, height: 1.0 }
            .validate()
            .is_ok());

        // A height fraction or azimuth outside its domain is refused, not clamped.
        let good = Profile::Cylinder { radius: 1.0, height: 2.0 };
        assert_eq!(
            good.silhouette_radius(1.5).unwrap_err(),
            RefusalKind::ParameterOutOfRange
        );
        assert_eq!(
            good.normal(0.5, f64::NAN).unwrap_err(),
            RefusalKind::ParameterOutOfRange
        );
    }

    #[test]
    fn a_cylinder_is_radial_and_a_flat_cap_points_up() {
        let cylinder = Profile::Cylinder { radius: 0.5, height: 1.0 };
        assert_eq!(cylinder.silhouette_radius(0.0).unwrap(), 0.5);
        assert_eq!(cylinder.silhouette_radius(1.0).unwrap(), 0.5);
        let n = cylinder.normal(0.3, std::f64::consts::FRAC_PI_2).unwrap();
        assert!((n[0] - 0.0).abs() < 1e-12 && (n[1] - 1.0).abs() < 1e-12 && n[2].abs() < 1e-12);

        let flat = Profile::Flat { radius: 1.0 };
        assert_eq!(flat.normal(0.0, 1.234).unwrap(), UP);
        assert_eq!(flat.height().unwrap(), 0.0);
    }

    #[test]
    fn a_frustum_normal_is_exact() {
        // A cone narrowing upward: dr/dz = -1, so the outward normal is 45° up-and-outward.
        let cone = Profile::Frustum { base_radius: 1.0, top_radius: 0.0, height: 1.0 };
        let n = cone.normal(0.5, 0.0).unwrap();
        let expected = 1.0 / 2.0_f64.sqrt();
        assert!((n[0] - expected).abs() < 1e-12, "{n:?}");
        assert!((n[2] - expected).abs() < 1e-12, "{n:?}");
        assert!(unit(n));

        // A cylinder-shaped frustum has a radial normal.
        let straight = Profile::Frustum { base_radius: 1.0, top_radius: 1.0, height: 2.0 };
        assert_eq!(straight.normal(0.5, 0.0).unwrap(), [1.0, 0.0, 0.0]);
    }

    #[test]
    fn a_spheroid_uses_its_silhouette_and_not_a_base_radius() {
        let spheroid = Profile::Spheroid { semi_axis_r: 1.0, semi_axis_z: 2.0 };
        // Equator: the widest silhouette, radial normal.
        assert!((spheroid.silhouette_radius(0.5).unwrap() - 1.0).abs() < 1e-12);
        assert_eq!(spheroid.normal(0.5, 0.0).unwrap(), [1.0, 0.0, 0.0]);
        // The poles: zero silhouette, axis normal.
        assert!(spheroid.silhouette_radius(1.0).unwrap().abs() < 1e-12);
        assert_eq!(spheroid.normal(1.0, 0.0).unwrap(), UP);
        assert_eq!(spheroid.normal(0.0, 0.0).unwrap(), [0.0, 0.0, -1.0]);
        // **The correction:** away from the equator the silhouette is smaller than the semi-axis,
        // so a caller that used `semi_axis_r` as the radius would build a cylinder.
        let t = 0.9; // u = 0.8, so the silhouette is sqrt(1 - 0.64) = 0.6, not the semi-axis 1.0
        let silhouette = spheroid.silhouette_radius(t).unwrap();
        assert!((silhouette - 0.6).abs() < 1e-12, "silhouette = {silhouette}");
        assert!((silhouette - 1.0).abs() > 0.3, "the base radius does not reproduce");
        // Normals are unit and the height is the full axis.
        assert!(unit(spheroid.normal(t, 0.7).unwrap()));
        assert!((spheroid.height().unwrap() - 4.0).abs() < 1e-12);
    }

    #[test]
    fn a_stalagmite_keeps_the_laws_exact_consequences_and_its_printed_figure_does_not_reproduce() {
        let da = 2.0;
        let radius = 1.0;
        let stalagmite = Profile::Stalagmite { da, radius };

        // The base is a vertical wall: the normal is radial.
        let base = stalagmite.normal(0.0, 0.0).unwrap();
        assert!((base[0] - 1.0).abs() < 1e-12 && base[2].abs() < 1e-12, "{base:?}");

        // The cap radius is the law's own value, and the top edge meets the axis normal.
        let cap_radius = stalagmite.stalagmite_cap_radius().unwrap();
        let expected = (1.0 - 1.0 / (da * da)).sqrt();
        assert!((cap_radius - expected).abs() < 1e-12, "cap radius = {cap_radius}");
        assert_eq!(stalagmite.normal(1.0, 0.0).unwrap(), UP);
        assert!((stalagmite.silhouette_radius(1.0).unwrap() - cap_radius).abs() < 1e-12);

        // The flank is monotone: radial at the base, straightening toward the cap.
        let low = stalagmite.normal(0.05, 0.0).unwrap();
        let high = stalagmite.normal(0.5, 0.0).unwrap();
        assert!(low[0] > high[0], "the flank should straighten toward the top: {low:?} {high:?}");
        assert!(unit(low) && unit(high));

        // **The discrepancy, recorded:** the paper's printed cap radius R·√(1 − 1/Da) does not
        // reproduce, and the difference is not a rounding error.
        let printed = (1.0 - 1.0 / da).sqrt();
        assert!((cap_radius - printed).abs() > 0.1, "printed figure reproduced: {printed}");

        // A cylinder has no cap radius, and asking is `None`, not a guess.
        assert!(Profile::Cylinder { radius: 1.0, height: 1.0 }.stalagmite_cap_radius().is_none());
    }

    #[test]
    fn a_stalagmite_silhouette_and_normal_belong_to_one_body() {
        let s = Profile::Stalagmite { da: 2.0, radius: 1.0 };
        let cap = s.stalagmite_cap_radius().unwrap();
        let height = s.height().unwrap();

        // The top edge is the law's cap radius and its normal is the axis.
        assert!((s.silhouette_radius(1.0).unwrap() - cap).abs() < 1e-12);
        assert_eq!(s.normal(1.0, 0.0).unwrap(), UP);

        // On the flank the normal is perpendicular to the meridian's own tangent — the silhouette
        // and the normal are read from one curve (the audit's failing case: they were not).
        for t in [0.05, 0.25, 0.5, 0.75, 0.95] {
            let eps = 1e-6;
            let dr_dt = (s.silhouette_radius(t + eps).unwrap() - s.silhouette_radius(t - eps).unwrap())
                / (2.0 * eps);
            let dz_dt = height; // z climbs with the height fraction
            let n = s.normal(t, 0.0).unwrap();
            let dot = n[0] * dr_dt + n[2] * dz_dt;
            assert!(dot.abs() < 1e-6, "t={t}: n.tangent = {dot}");
        }

        // **The audit's case, pinned:** a flat normal at a height the silhouette says is wider than
        // the cap is the contradiction this test forbids.
        let n = s.normal(0.5, 0.0).unwrap();
        let flat = n == UP;
        assert!(
            !(flat && (s.silhouette_radius(0.5).unwrap() - cap).abs() > 1e-9),
            "flat normal with a silhouette below the cap"
        );
    }
}
