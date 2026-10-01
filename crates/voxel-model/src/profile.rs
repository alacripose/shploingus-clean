//! **The shapes a block can be**, as analytic profiles.
//!
//! A shaper that says "a pillar" or "a stalagmite" has to *mean* something, and
//! this module is where the meaning lives. Every profile here is a **function** —
//! a cross-section radius, or a protruding surface and its slope — so a whole
//! family of shapes comes from one law, and the 4×4×4 pattern a shaper writes is a
//! **sample** of that law rather than a hand-drawn bitmap.
//!
//! That direction matters more than it looks. A hand-drawn pattern is a shape
//! somebody picked; a sampled law is a shape with a *parameter*, and the parameter
//! is derived from the world. `Profile::Stalagmite` has exactly one parameter — the
//! Damköhler number — and the world computes it from the drip rate and the base
//! radius. Nothing about the shape is chosen, which is the same discipline
//! C-39 applied to colour.
//!
//! # The surfaces are surfaces of revolution, and that is the whole trick
//!
//! Every profile here describes a body whose cross-section is a circle, and the
//! exact outward normal of such a body is available in closed form:
//!
//! * A body `r = R(z)` (a **column**, axis along one of the block's axes) has
//!   `normal ∝ r̂ − R′(z)·ẑ`. The horizontal part is radial and the axial part is
//!   the taper — which is why a frustum and a cylinder differ by one term.
//! * A body `z = h(r)` (a **cap**, a surface of revolution about the face's own
//!   normal) has `normal ∝ n̂ − h′(r)·r̂`.
//!
//! Neither needs a constant of adjustment, which is what lets "fake wrapping" be
//! *correct* rather than plausible: on a `RoundColumn` every normal this module
//! returns is the normal of the cylinder or cone the pattern describes.
//!
//! # The stalagmite, and the one parameter
//!
//! **Szymczak, P., Ladd, A. J. C., Lipar, M. & Pekarovič, D. (2025), *Shapes of
//! ideal stalagmites*, PNAS 122(42) e2513263122** solve Franke's (1965) growth
//! model analytically for a surface of revolution that grows upward without
//! changing shape. Their Eq. 8 gives the **tangent angle** directly:
//!
//! ```text
//! cos θ(r) = Da · √(1 − (r/R)²)          Da = U₀/U = πR²k/Q
//! ```
//!
//! and `θ(R) = π/2`, so the sides are vertical at the maximum radius. The
//! Damköhler number `Da` is the **only** shape parameter and it classifies the
//! whole family into three forms, all of them observed in the wild:
//!
//! | `Da` | the form | why |
//! |---|---|---|
//! | `> 1` | **flat top** | `cos θ` would exceed 1 near the axis, so the solution is capped by a flat disc |
//! | `= 1` | **columnar** | the cap is **exactly a hemisphere** — see [`Profile::Stalagmite`]'s tests |
//! | `< 1` | **conical**, pointed | the cap never flattens; lower `Da` is taller and sharper |
//!
//! ## A correction to the paper's own printed figure, recorded not inherited
//!
//! The paper states the flat-top cap radius as `R_c = R·√(1 − Da⁻¹)`. That does
//! **not** follow from its own Eq. 8/10: with `cos θ = Da·√(1 − (r/R)²)`, the
//! condition `cos θ ≤ 1` gives
//!
//! ```text
//! Da·√(1 − s²) ≤ 1  ⇒  s² ≥ 1 − 1/Da²  ⇒  R_c = R·√(1 − 1/Da²)
//! ```
//!
//! This module implements the **tangent law it can verify** and derives `R_c` from
//! it, so the code is internally consistent and the discrepancy is recorded here
//! rather than silently resolved in the paper's favour. It is the same shape as
//! C-10 and C-28: a figure in a source disagreed with that source's own arithmetic,
//! and the correction is written down. Whether the printed `Da⁻¹` is a typo for
//! `Da⁻²` or the tangent law is quoted with `Da` where the paper means `1/Da` is
//! **open**, and it is named as open.
//!
//! # What is not implemented, stated
//!
//! The paper's closed form for `z(r)` is an incomplete elliptic integral of the
//! third kind (their Eq. 11). This module integrates their Eq. 10 instead —
//! numerically, under a substitution that removes the endpoint singularity, at a
//! declared step count. That is a **quadrature of the same law**, not a different
//! law, but it is not their closed form and it is not claimed to be.
//!
//! The **stalactite** of Short, Baygents & Goldstein (2005) is a different
//! free-boundary solution and is *not* implemented here: [`Profile::Frustum`] with
//! an input tip radius can approximate one, and the approximation is named where it
//! is used rather than dressed as the ideal.

use crate::coord::SUBVOXEL_SIDE;
use crate::refusal::{Refusal, RefusalKind, Refused};
use crate::subvoxel::Face;

/// The largest radius a cross-section may have: half a block, because that is a
/// circle **inscribed** in the block's square.
pub const MAX_RADIUS: f64 = 0.5;

/// The smallest radius a profile may declare. Not zero: a radius of zero is not a
/// shape, it is a line, and the pattern it produces is empty.
pub const MIN_RADIUS: f64 = 0.02;

/// Simpson intervals for the quadrature of the tangent law. Declared rather than
/// chosen per call, so the number is one constant with one test.
pub const QUADRATURE_INTERVALS: usize = 64;

/// Which of the three forms a stalagmite law produces. **Derived** from `Da`, never
/// declared — see the module header.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Regime {
    /// A plane: nothing protrudes, nothing bends.
    Flat,
    /// `Da < 1`: pointed, taller than the columnar form.
    Conical,
    /// `Da = 1`: the cap is exactly a hemisphere.
    Columnar,
    /// `Da > 1`: a flat disc cap of a derived radius, with the law outside it.
    FlatTop,
}

impl Regime {
    pub const fn name(self) -> &'static str {
        match self {
            Regime::Flat => "flat",
            Regime::Conical => "conical",
            Regime::Columnar => "columnar",
            Regime::FlatTop => "flat-top",
        }
    }
}

/// A body whose cross-section is a circle, described by a law rather than a bitmap.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Profile {
    /// A plane. Nothing protrudes anywhere, so nothing wraps.
    Flat,
    /// A circular column with a **flat** end — the mathematically plain cylinder.
    Cylinder { radius: f64 },
    /// A circular column with a **rounded** end: the same cross-section as
    /// [`Profile::Cylinder`] and a cap that is a hemisphere. This is the shape a
    /// **pillar** reads as, and the reason it exists as a separate variant is that
    /// "round" is a property of the *end* and not of the side.
    RoundColumn { radius: f64 },
    /// A **frustum**: the radius varies linearly along the axis, so a
    /// `Frustum { low_radius, high_radius }` with `high < low` is a tapering
    /// stalactite-like spike and one with `high > low` is a mound.
    Frustum { low_radius: f64, high_radius: f64 },
    /// A **spheroid** — a surface of revolution whose cross-section is elliptical:
    /// `z(r) = height·√(1 − (r/radius)²)`.
    ///
    /// This one variant carries three of the four **Zingg (1935)** form classes,
    /// which is why it is not called a dome. With `height = radius` it is a
    /// hemisphere — an **equant** particle; with `height < radius` it is a disc
    /// (**oblate**); with `height > radius` it is a rod (**prolate**). The fourth
    /// class, **bladed**, is *not* a surface of revolution and is therefore **not
    /// representable here** — a limitation, stated rather than papered over with a
    /// tri-axial ellipsoid whose normal this module could not compute exactly.
    Spheroid { radius: f64, height: f64 },
    /// **The invariant stalagmite of Szymczak et al. (2025)**: one parameter, three
    /// forms. See the module header for the law and for the correction to the
    /// paper's printed cap radius.
    Stalagmite { da: f64, radius: f64 },
}

impl Profile {
    /// A stalagmite from a Damköhler number and a base radius, refusing a
    /// non-positive or non-finite `Da` — both make the law meaningless and neither
    /// has a sane substitute.
    pub fn stalagmite(da: f64, radius: f64) -> Refused<Self> {
        if !da.is_finite() || da <= 0.0 {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!("a stalagmite's Damköhler number {da} must be finite and positive"),
            ));
        }
        check_radius(radius)?;
        Ok(Profile::Stalagmite { da, radius })
    }

    /// The Damköhler number from the world's own quantities: `Da = πR²k/Q`.
    ///
    /// This is the paper's Eq. 9, and it is here so that no caller invents a `Da`:
    /// a cave's drip rate `Q` (m³/s) and a base radius `R` (block units) are
    /// **world** facts, and the shape follows. `k` is the precipitation rate
    /// constant the paper's ref 11 gives, `1.3 × 10⁻⁵ cm/s`.
    pub fn da_from_flow(base_radius: f64, flow_m3_s: f64) -> Refused<f64> {
        check_radius(base_radius)?;
        if !flow_m3_s.is_finite() || flow_m3_s <= 0.0 {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!("a drip rate of {flow_m3_s} m³/s is not a flow"),
            ));
        }
        // k in metres per second, from the paper's 1.3e-5 cm/s.
        const K_M_S: f64 = 1.3e-7;
        // R in metres, from block units of one metre.
        let radius_m = base_radius;
        Ok(std::f64::consts::PI * radius_m * radius_m * K_M_S / flow_m3_s)
    }

    /// The cross-section radius at an axial coordinate in `[0, 1]`, where 0 is the
    /// body's low end.
    ///
    /// A linear taper for [`Profile::Frustum`], a constant for the columns, and for
    /// a stalagmite the radius *is* the base radius, because the taper is in the
    /// **cap** — which is exactly the structural difference between a stalagmite and
    /// a cone.
    pub fn radius_at(&self, along: f64) -> f64 {
        match *self {
            Profile::Flat => 0.0,
            Profile::Cylinder { radius } | Profile::RoundColumn { radius } => radius,
            Profile::Spheroid { radius, .. } => radius,
            Profile::Frustum { low_radius, high_radius } => {
                let t = along.clamp(0.0, 1.0);
                low_radius + (high_radius - low_radius) * t
            }
            Profile::Stalagmite { radius, .. } => radius,
        }
    }

    /// **The radius of the body's cross-section at an axial fraction `t` in
    /// `[0, 1]`**, measuring `t` from the body's *low* end.
    ///
    /// This is not the same function as [`Profile::radius_at`], and conflating them
    /// was a real bug: `radius_at` is the radius a *taper* declares, while this is
    /// the radius the body **actually has** at that height — which for a spheroid and
    /// for a stalagmite is the whole shape and not the base.
    ///
    /// * A **cylinder** and a **round column** keep a constant cross-section: their
    ///   shape is in their **end**, and a block of one is the same disc everywhere.
    /// * A **frustum** is its taper.
    /// * A **spheroid** is **centred in the block** — a boulder rests on the block's
    ///   centre and its form is symmetric about it — so its section is
    ///   `radius·√(1 − ((t − ½)/height)²)`, which is zero outside it.
    /// * A **stalagmite**'s block *is its cap*: the section is the **inverse** of the
    ///   cap law, so at height `t·apex` the radius is the `r` where `h(r)` reaches
    ///   that height. This is the correction that makes a stalagmite a *shaped
    ///   silhouette* rather than a cylinder with a spike on top, and it is what
    ///   correction 3 asked for: the same law, sampled, not a different radius.
    ///
    /// The inversion is a bisection on the cap's monotone height, which is why it is
    /// deterministic and why it is not a table. It runs on `[plateau, radius]` where
    /// `plateau` is the flat cap's own radius, because **inside the flat disc the law
    /// is a plateau**: the silhouette at the very top of a flat-topped stalagmite is
    /// the disc, not the vanishing root of a bisection. Zero `plateau` (conical and
    /// columnar forms) makes that interval the whole body.
    ///
    /// A **taller** speleothem is a *stack*: the blocks below the top carry a
    /// [`Profile::RoundColumn`] body and the topmost block carries the cap, so the
    /// cap is one block of a column rather than a scaled copy of it. Choosing which
    /// block is the top is the world's job, not this function's.
    pub fn cross_section_radius(&self, t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        match *self {
            Profile::Flat => 0.0,
            Profile::Cylinder { radius } | Profile::RoundColumn { radius } => radius,
            Profile::Frustum { low_radius, high_radius } => low_radius + (high_radius - low_radius) * t,
            Profile::Spheroid { radius, height } => {
                if height <= 0.0 {
                    return 0.0;
                }
                let z = (t - 0.5) / height;
                let inner = 1.0 - z * z;
                if inner <= 0.0 {
                    0.0
                } else {
                    radius * inner.sqrt()
                }
            }
            Profile::Stalagmite { radius, .. } => {
                let apex = self.cap_height_total();
                if apex <= 0.0 {
                    return 0.0;
                }
                // The flat cap's own radius, and zero for the other two regimes.
                // The plateau is returned **exactly** at the top instead of being
                // approached: inside the disc the law's height is constant, so the
                // bisection has no root there and a bisection would walk to zero.
                let plateau = self.flat_cap_radius();
                if t >= 1.0 {
                    return plateau;
                }
                let target = t * apex;
                // `cap_height` is **strictly decreasing** on `[plateau, radius]`:
                // the apex is at the axis and the surface falls to nothing at the rim.
                // So the inverse on that interval is a bisection for the largest `r`
                // whose height still reaches `target`, which **is** the silhouette.
                let (mut lo, mut hi) = (plateau, radius);
                for _ in 0..48 {
                    let mid = 0.5 * (lo + hi);
                    if self.cap_height(mid) > target {
                        lo = mid;
                    } else {
                        hi = mid;
                    }
                }
                0.5 * (lo + hi)
            }
        }
    }

    /// `dR/d(along)` — the taper's slope, and the term that turns a cylinder's
    /// normal into a frustum's.
    pub fn radius_slope(&self, _along: f64) -> f64 {
        match *self {
            Profile::Frustum { low_radius, high_radius } => high_radius - low_radius,
            _ => 0.0,
        }
    }

    /// The largest cross-section radius anywhere on the body.
    pub fn max_radius(&self) -> f64 {
        match *self {
            Profile::Flat => 0.0,
            Profile::Cylinder { radius } | Profile::RoundColumn { radius } => radius,
            Profile::Spheroid { radius, .. } => radius,
            Profile::Frustum { low_radius, high_radius } => low_radius.max(high_radius),
            Profile::Stalagmite { radius, .. } => radius,
        }
    }

    /// True when nothing protrudes: a plane, and the gate that keeps flat rock flat.
    pub fn is_flat(&self) -> bool {
        matches!(self, Profile::Flat)
    }

    /// Which form this profile produces. `Da` decides for a stalagmite; everything
    /// else is its own kind.
    pub fn regime(&self) -> Regime {
        match *self {
            Profile::Flat => Regime::Flat,
            Profile::Stalagmite { da, .. } => {
                if da > 1.0 {
                    Regime::FlatTop
                } else if da < 1.0 {
                    Regime::Conical
                } else {
                    Regime::Columnar
                }
            }
            _ => Regime::Columnar,
        }
    }

    /// The radius of a stalagmite's flat cap, **derived from the tangent law**:
    /// `R·√(1 − 1/Da²)` where the law's `cos θ` first reaches 1.
    ///
    /// Zero when `Da ≤ 1`, which is the statement that a columnar or conical
    /// stalagmite has no flat region — its cap reaches the axis.
    pub fn flat_cap_radius(&self) -> f64 {
        match *self {
            Profile::Stalagmite { da, radius } if da > 1.0 => {
                radius * (1.0 - 1.0 / (da * da)).max(0.0).sqrt()
            }
            _ => 0.0,
        }
    }

    // ------------------------------------------------------------------ the cap

    /// The **protrusion** of the cap at radius `r`: how far the surface stands out
    /// from the face plane, in block units.
    ///
    /// Zero outside the body's own radius, because there is no surface there — the
    /// pattern has already fallen away and the normal is radial.
    pub fn cap_height(&self, r: f64) -> f64 {
        if r < 0.0 || !r.is_finite() {
            return 0.0;
        }
        match *self {
            Profile::Flat | Profile::Cylinder { .. } | Profile::Frustum { .. } => 0.0,
            Profile::RoundColumn { radius } => {
                if r >= radius {
                    0.0
                } else {
                    (radius * radius - r * r).max(0.0).sqrt()
                }
            }
            Profile::Spheroid { radius, height } => {
                if r >= radius {
                    0.0
                } else {
                    let t = r / radius;
                    height * (1.0 - t * t).max(0.0).sqrt()
                }
            }
            Profile::Stalagmite { da, radius } => {
                if r >= radius {
                    return 0.0;
                }
                let s = (r / radius).min(1.0);
                let psi0 = s.asin();
                // The flat-top case: inside `R_c` the tangent law gives `cos θ > 1`,
                // which is unphysical, and the solution is a flat disc whose height
                // is the height at `R_c`. `R_c/R = √(1 − 1/Da²)`, so in `ψ` the
                // lower limit is `asin(R_c/R)` — **not** `asin(1/Da)`, which is the
                // one-letter error that makes the flat cap's height drift with `r`
                // instead of being constant. Caught by the hemisphere test below
                // disagreeing at the cap's edge.
                let psi_start = if da > 1.0 {
                    let s_flat = (1.0 - 1.0 / (da * da)).max(0.0).sqrt();
                    psi0.max(s_flat.asin())
                } else {
                    psi0
                };
                // `z(r) = (R/Da) ∫ √(1 − Da²cos²ψ) dψ` from `ψ0` to `π/2`, which is
                // Eq. 10 under `s = sin ψ` — the substitution that removes the
                // `1/√(1−s²)` endpoint singularity so a plain Simpson rule converges.
                let scale = radius / da;
                let (lo, hi) = (psi_start, std::f64::consts::FRAC_PI_2);
                if hi <= lo {
                    return 0.0;
                }
                let n = QUADRATURE_INTERVALS;
                let h = (hi - lo) / n as f64;
                let integrand = |psi: f64| {
                    let c = psi.cos();
                    (1.0 - da * da * c * c).max(0.0).sqrt()
                };
                let mut total = integrand(lo) + integrand(hi);
                for i in 1..n {
                    let weight = if i % 2 == 1 { 4.0 } else { 2.0 };
                    total += weight * integrand(lo + i as f64 * h);
                }
                scale * total * h / 3.0
            }
        }
    }

    /// `dh/dr` — the cap's slope, which is what bends the shading normal.
    ///
    /// `−∞` at the body's rim in the limit, which is the honest value (the surface
    /// there *is* vertical); callers that need a finite number use
    /// [`Profile::cap_tangency`], which normalises.
    pub fn cap_slope(&self, r: f64) -> f64 {
        match *self {
            Profile::Flat | Profile::Cylinder { .. } | Profile::Frustum { .. } => 0.0,
            Profile::RoundColumn { radius } => {
                if r <= 0.0 {
                    0.0
                } else if r >= radius {
                    f64::NEG_INFINITY
                } else {
                    -r / (radius * radius - r * r).sqrt()
                }
            }
            Profile::Spheroid { radius, height } => {
                if r <= 0.0 {
                    0.0
                } else if r >= radius {
                    f64::NEG_INFINITY
                } else {
                    let t = r / radius;
                    // z = h·√(1 − t²)  ⇒  dz/dr = −(h/radius)·t/√(1 − t²).
                    -(height / radius) * t / (1.0 - t * t).sqrt()
                }
            }
            Profile::Stalagmite { da, radius } => {
                if r <= 0.0 || r >= radius {
                    return if r >= radius { f64::NEG_INFINITY } else { 0.0 };
                }
                let s = r / radius;
                let cos_theta = da * (1.0 - s * s).max(0.0).sqrt();
                if cos_theta >= 1.0 {
                    // Inside the flat cap: the disc is flat, so the slope is zero.
                    return 0.0;
                }
                // tan θ = √(1/cos²θ − 1), and dz/dr = −tan θ.
                if cos_theta <= 0.0 {
                    return f64::NEG_INFINITY;
                }
                -((1.0 / (cos_theta * cos_theta) - 1.0).max(0.0)).sqrt()
            }
        }
    }

    /// The cap's tangency at radius `r`, as `(axial, radial)` unit components:
    /// `normalise(1, −h′(r))`. Finite for every input, including the rim, which is
    /// why this is the form callers use.
    pub fn cap_tangency(&self, r: f64) -> (f64, f64) {
        let slope = -self.cap_slope(r);
        if !slope.is_finite() {
            // The surface is vertical: the normal is purely radial.
            return (0.0, 1.0);
        }
        let length = (1.0 + slope * slope).sqrt();
        (1.0 / length, slope / length)
    }

    /// The total height of the cap above its base — the apex height.
    pub fn cap_height_total(&self) -> f64 {
        self.cap_height(0.0)
    }

    // -------------------------------------------------------------- the normals

    /// The exact outward normal of a **column**'s side at a point on `face`.
    ///
    /// `across` is the in-plane coordinate across the column, in block units, where
    /// `0.5` is the point where the column's surface meets the face; `along` is the
    /// in-plane coordinate *along* the column, which is what a taper varies with.
    ///
    /// # The law, which is one line
    ///
    /// A body `r = R(z)` has `normal ∝ r̂ − R′(z)·ẑ`. On `face`, a point at
    /// `t = across − 0.5` off the axis sits where the circle is
    /// `√(R² − t²)` out along the face normal, so `r̂` resolves into
    /// `(√(R²−t²)/R)` along the face normal and `(t/R)` along the in-plane axis.
    /// Outside the body `|t| ≥ R` the surface has turned vertical and the normal is
    /// **purely radial** — the limit, not a clamp.
    pub fn column_normal(&self, face: Face, across: f64, along: f64) -> Refused<[f64; 3]> {
        if !across.is_finite() || !along.is_finite() {
            return Err(Refusal::new(
                RefusalKind::NonFiniteValue,
                "a column normal was asked for at a non-finite coordinate",
            ));
        }
        let n = face_vector(face);
        if self.is_flat() || self.max_radius() <= 0.0 {
            return Ok(n);
        }
        let radius = self.radius_at(along);
        let t = across - 0.5;
        let (across_axis, along_axis) = in_plane_axes(face);

        if radius <= 0.0 || t.abs() >= radius {
            // The surface is vertical here: no component along the face normal.
            let sign = if t < 0.0 { -1.0 } else { 1.0 };
            return Ok(scale(across_axis, sign));
        }

        let cos_phi = ((radius * radius - t * t).max(0.0)).sqrt() / radius;
        let sin_phi = t / radius;
        let taper = self.radius_slope(along);
        let out = [
            cos_phi * n[0] + sin_phi * across_axis[0] - taper * along_axis[0],
            cos_phi * n[1] + sin_phi * across_axis[1] - taper * along_axis[1],
            cos_phi * n[2] + sin_phi * across_axis[2] - taper * along_axis[2],
        ];
        unit(out).ok_or_else(|| {
            Refusal::owned(
                RefusalKind::NonFiniteValue,
                format!("a column normal degenerated at across {across}, along {along}"),
            )
        })
    }

    /// The exact outward normal of a **cap** at face coordinates `(u, v)` in
    /// `[0, 1]`, in the face's own basis.
    ///
    /// `normal ∝ n̂ − h′(r)·r̂` with `r` measured from the face's centre — the
    /// surface-of-revolution normal. At the centre the bend is zero, so a cap is
    /// shaded exactly as a plane at its middle, which is what a real cap is.
    pub fn cap_normal(&self, face: Face, u: f64, v: f64) -> Refused<[f64; 3]> {
        let n = face_vector(face);
        if !(0.0..=1.0).contains(&u) || !u.is_finite() || !(0.0..=1.0).contains(&v) || !v.is_finite() {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!("face coordinate ({u}, {v}) outside [0, 1]"),
            ));
        }
        if self.is_flat() || self.max_radius() <= 0.0 {
            return Ok(n);
        }
        let (u_axis, v_axis) = in_plane_axes(face);
        let du = u - 0.5;
        let dv = v - 0.5;
        let r = (du * du + dv * dv).sqrt();
        if r <= 0.0 {
            return Ok(n);
        }
        let (axial, radial) = self.cap_tangency(r);
        let out = [
            axial * n[0] + (radial / r) * (du * u_axis[0] + dv * v_axis[0]),
            axial * n[1] + (radial / r) * (du * u_axis[1] + dv * v_axis[1]),
            axial * n[2] + (radial / r) * (du * u_axis[2] + dv * v_axis[2]),
        ];
        unit(out).ok_or_else(|| {
            Refusal::owned(
                RefusalKind::NonFiniteValue,
                format!("a cap normal degenerated at ({u}, {v})"),
            )
        })
    }

    /// **The discrete pattern, sampled from the law.** Which of a face's `n × n`
    /// cells fall **outside** the cross-section the profile describes.
    ///
    /// This is the function that makes a shaper honest: `pillar`, `stalagmite` and
    /// `frustum` do not each carry a bitmap, they sample *this*. `cells` is the
    /// grid side (4 for the stored subvoxel level, and the same function serves any
    /// finer grid), and `along`/`axial` locate the slice of the body the face is at.
    ///
    /// A cell is outside when the distance from its centre to the **face centre**
    /// exceeds the profile's radius at that slice. Returned in the same order
    /// [`crate::subvoxel`]'s face iterator uses — `u` outside, `v` inside — because
    /// two functions that describe one pattern must agree about its order.
    pub fn outward_cells(&self, cells: u32, radius: f64) -> Vec<bool> {
        let side = cells.max(1);
        let mut out = Vec::with_capacity((side * side) as usize);
        for ui in 0..side {
            for vi in 0..side {
                let u = (ui as f64 + 0.5) / side as f64;
                let v = (vi as f64 + 0.5) / side as f64;
                let du = u - 0.5;
                let dv = v - 0.5;
                let r = (du * du + dv * dv).sqrt();
                out.push(r > radius);
            }
        }
        out
    }

    /// How many cells of a `cells × cells` face are outside the body — the number
    /// a test asserts and a report prints.
    pub fn outward_count(&self, cells: u32, radius: f64) -> usize {
        self.outward_cells(cells, radius).iter().filter(|o| **o).count()
    }

    /// The slice of the body a face sits at, so a per-face pattern knows which
    /// radius to sample. `cell` is the subvoxel coordinate along the axis
    /// perpendicular to `face`.
    ///
    /// The mapping is the one thing that has to be *decided* about a sampled
    /// pattern, so it is decided here, once, and stated: cell `i` of `n` samples the
    /// body at axial fraction `(i + 0.5)/n`.
    pub fn axial_fraction(&self, cell: u8, cells: u32) -> f64 {
        (cell as f64 + 0.5) / cells.max(1) as f64
    }
}

/// The stored subvoxel grid side, so a caller does not have to spell `4`.
pub const SUBVOXEL_CELLS: u32 = SUBVOXEL_SIDE;

fn check_radius(radius: f64) -> Refused<()> {
    if !radius.is_finite() || radius < MIN_RADIUS || radius > MAX_RADIUS {
        return Err(Refusal::owned(
            RefusalKind::OutOfDeclaredRange,
            format!(
                "a radius of {radius} is outside [{MIN_RADIUS}, {MAX_RADIUS}]; a body that does not fit its block is not a block shape"
            ),
        ));
    }
    Ok(())
}

/// The outward unit vector of a face.
pub fn face_vector(face: Face) -> [f64; 3] {
    let (x, y, z) = face.normal();
    [x as f64, y as f64, z as f64]
}

/// The two in-plane unit axes of a face, in the same `(u, v)` order
/// [`crate::subvoxel::VoxelShape::face_cells`] walks them.
pub fn in_plane_axes(face: Face) -> ([f64; 3], [f64; 3]) {
    match face.axis() {
        // u = y, v = z
        0 => ([0.0, 1.0, 0.0], [0.0, 0.0, 1.0]),
        // u = x, v = z
        1 => ([1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
        // u = x, v = y
        _ => ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
    }
}

/// The in-plane axis a **column's** taper runs along, which is the one that is
/// *not* the axis the cross-section lies in.
///
/// For a column whose axis is world `z`, a side face's cross-section lies in the
/// horizontal pair, so the axial in-plane axis is `v` on the x-faces (v = z) and
/// `u` is horizontal; on the y-faces the same; and on the z-faces the cross-section
/// is the horizontal plane and the column has no taper *within* the face. Stated as
/// a function with a test because getting it wrong rotates the taper ninety degrees
/// and looks like a lighting bug.
pub fn column_cross_axis(face: Face) -> bool {
    // `true` when the *first* in-plane axis (u) crosses the column, `false` when the
    // second (v) does. A column's axis is world z, so:
    //   face.axis() == 0 (x faces): u = y (cross), v = z (axial)  -> true
    //   face.axis() == 1 (y faces): u = x (cross), v = z (axial)  -> true
    //   face.axis() == 2 (z faces): the face is the cross-section itself -> false
    face.axis() != 2
}

fn scale(v: [f64; 3], s: f64) -> [f64; 3] {
    [v[0] * s, v[1] * s, v[2] * s]
}

fn unit(v: [f64; 3]) -> Option<[f64; 3]> {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if length == 0.0 || !length.is_finite() {
        return None;
    }
    Some([v[0] / length, v[1] / length, v[2] / length])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_unit(n: [f64; 3]) {
        let length = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        assert!((length - 1.0).abs() < 1e-12, "not a unit vector: {n:?}");
        assert!(n.iter().all(|c| c.is_finite()), "non-finite normal {n:?}");
    }

    #[test]
    fn a_da_of_one_is_exactly_the_hemisphere_and_that_is_the_papers_own_columnar_case() {
        // The strongest check available on the law: at Da = 1 the quadrature must
        // reproduce sqrt(R^2 - r^2) in closed form. If the substitution, the
        // quadrature or the law were wrong, this would be the first thing to fail —
        // and it is a claim the paper makes ("columnar ... hemispherical") that is
        // checkable in eight lines.
        let radius = 0.5;
        let p = Profile::stalagmite(1.0, radius).unwrap();
        assert_eq!(p.regime(), Regime::Columnar);
        assert_eq!(p.flat_cap_radius(), 0.0);
        // The tolerance is the quadrature's **own** error and not a fudge: for a
        // smooth integrand Simpson's rule at 64 intervals is good to about
        // `(π/2)(π/128)⁴/180 ≈ 3.2e-9` here, times the `R/Da = 0.5` scale. Stating it
        // is what keeps the 1e-9-versus-1e-8 question from being decided by whoever
        // ran the test last.
        const QUADRATURE_TOLERANCE: f64 = 1e-8;
        for i in 0..=10 {
            let r = radius * i as f64 / 10.0;
            let exact = (radius * radius - r * r).max(0.0).sqrt();
            let got = p.cap_height(r);
            assert!(
                (got - exact).abs() < QUADRATURE_TOLERANCE,
                "r = {r}: quadrature {got} vs closed form {exact}"
            );
        }
        assert!((p.cap_height_total() - radius).abs() < QUADRATURE_TOLERANCE);
    }

    #[test]
    fn the_daekohler_number_derives_the_three_forms_and_nothing_declares_them() {
        let radius = 0.3;
        let conical = Profile::stalagmite(0.5, radius).unwrap();
        let columnar = Profile::stalagmite(1.0, radius).unwrap();
        let flat = Profile::stalagmite(2.0, radius).unwrap();
        assert_eq!(conical.regime(), Regime::Conical);
        assert_eq!(columnar.regime(), Regime::Columnar);
        assert_eq!(flat.regime(), Regime::FlatTop);

        // Below Da = 1 the form is taller: the paper's conical stalagmites.
        assert!(
            conical.cap_height_total() > columnar.cap_height_total(),
            "a conical stalagmite should be taller than a columnar one"
        );
        // And it keeps getting taller as Da falls, which is the "pointed" direction.
        let sharper = Profile::stalagmite(0.25, radius).unwrap();
        assert!(sharper.cap_height_total() > conical.cap_height_total());

        // Above Da = 1 a flat cap appears, of a radius the tangent law derives.
        assert!(flat.flat_cap_radius() > 0.0);
        // Rc = R sqrt(1 - 1/Da^2), which for Da = 2 is R sqrt(3)/2.
        assert!((flat.flat_cap_radius() - radius * 3.0f64.sqrt() / 2.0).abs() < 1e-12);
        // Inside the flat cap the surface is flat: slope zero, height the apex's.
        let apex = flat.cap_height_total();
        for r in [0.0, 0.05, 0.1, flat.flat_cap_radius() * 0.9, flat.flat_cap_radius() - 1e-12] {
            assert_eq!(flat.cap_slope(r), 0.0, "r = {r} should be on the flat disc");
            assert!((flat.cap_height(r) - apex).abs() < 1e-9, "r = {r}");
        }
        // Outside the cap the law takes over and the surface *falls*, so the flat
        // disc is a disc and not a plateau that never ends.
        let outside = flat.flat_cap_radius() + 0.01;
        assert!(flat.cap_height(outside) < apex);
        assert!(flat.cap_slope(outside) < 0.0);
        // And the paper's printed Rc = R·√(1 − Da⁻¹) is *not* what the tangent law
        // gives, which is R·√(1 − Da⁻²). The two agree only where both vanish, at
        // Da = 1. Asserted in closed form with no threshold, so the discrepancy is
        // recorded in a test rather than in a comment that can be tidied away — and
        // so that "make it match the paper" is a visible change to a failing
        // expectation rather than a quiet edit.
        for da in [2.0f64, 4.0, 10.0] {
            let p = Profile::stalagmite(da, radius).unwrap();
            let derived = radius * (1.0 - 1.0 / (da * da)).sqrt();
            let printed = radius * (1.0 - 1.0 / da).sqrt();
            assert!(
                (p.flat_cap_radius() - derived).abs() < 1e-12,
                "Da = {da}: the code must implement the law it can verify"
            );
            assert!(derived > printed, "Da = {da}");
        }
    }

    #[test]
    fn the_cap_normal_is_the_face_normal_at_the_centre_and_radial_at_the_rim() {
        // "Round" as a measurement rather than a vibe: zero bend at the axis, a
        // full turn to radial at the rim, and monotone in between.
        let p = Profile::RoundColumn { radius: 0.5 };
        for face in Face::ALL {
            let centre = p.cap_normal(face, 0.5, 0.5).unwrap();
            assert_eq!(centre, face_vector(face), "{face:?}");
            let rim = p.cap_normal(face, 1.0, 0.5).unwrap();
            assert_unit(rim);
            let axis = face.axis() as usize;
            assert!(rim[axis].abs() < 1e-12, "{face:?} rim still axial: {rim:?}");
        }
        // Monotone: the axial component never rises as the radius grows.
        let mut last = f64::INFINITY;
        for i in 0..=20 {
            let u = 0.5 + 0.5 * i as f64 / 20.0;
            let n = p.cap_normal(Face::PosZ, u, 0.5).unwrap();
            assert!(n[2].abs() <= last + 1e-12, "u = {u}");
            last = n[2].abs();
        }
    }

    #[test]
    fn a_flat_profile_returns_the_face_normal_for_every_input() {
        // The gate. A plane must not wrap — bending a plane's normal would make
        // every flat surface in the world look curved, which is a worse error than
        // a flat pillar because it is everywhere.
        let p = Profile::Flat;
        assert!(p.is_flat());
        assert_eq!(p.regime(), Regime::Flat);
        for face in Face::ALL {
            for u in [0.0, 0.25, 0.5, 1.0] {
                assert_eq!(p.cap_normal(face, u, u).unwrap(), face_vector(face));
                assert_eq!(p.column_normal(face, u, u).unwrap(), face_vector(face));
            }
        }
        assert_eq!(p.cap_height(0.0), 0.0);
        assert_eq!(p.cap_slope(0.1), 0.0);
    }

    #[test]
    fn a_columns_side_normal_is_exact_for_a_cylinder_and_for_a_frustum() {
        let cyl = Profile::RoundColumn { radius: 0.4 };
        // At across = 0.5 the column touches the face, so the normal is the face's.
        for face in Face::ALL {
            let n = cyl.column_normal(face, 0.5, 0.5).unwrap();
            assert_unit(n);
            assert_eq!(n, face_vector(face), "{face:?}");
        }
        // At the rim the normal has turned fully radial.
        let n = cyl.column_normal(Face::PosX, 0.9, 0.5).unwrap();
        assert_unit(n);
        assert!(n[0].abs() < 1e-12, "the rim should be radial: {n:?}");
        assert!(n[1].abs() > 0.99);
        // Half way across at t = R/2 the tilt is 30 degrees: sin(phi) = 1/2.
        let n = cyl.column_normal(Face::PosX, 0.5 + 0.2, 0.5).unwrap();
        assert!((n[1] - 0.5).abs() < 1e-12, "sin(phi) should be 0.5, got {}", n[1]);

        // A frustum adds the taper term: the normal leans against the direction it
        // widens in. A body that widens with `along` leans its normal *down*.
        let f = Profile::Frustum { low_radius: 0.2, high_radius: 0.5 };
        let n = f.column_normal(Face::PosX, 0.5, 0.0).unwrap();
        assert_unit(n);
        // On the +x face the axial in-plane axis is z, and the radius grows with it,
        // so the normal must acquire a −z component.
        assert!(n[2] < 0.0, "expected a downward lean, got {n:?}");
        // Reversed taper leans the other way, by the same amount.
        let g = Profile::Frustum { low_radius: 0.5, high_radius: 0.2 };
        let m = g.column_normal(Face::PosX, 0.5, 0.0).unwrap();
        assert!((m[2] + n[2]).abs() < 1e-12);
    }

    #[test]
    fn the_radial_sample_of_a_face_is_a_disc_and_the_corner_pattern_is_its_coarse_form() {
        // The pattern `pillar` used to hardcode — four corners — is *derived* here
        // as the inscribed disc sampled on a 4x4 grid, which is the whole point of
        // sampling a law instead of drawing a bitmap.
        let p = Profile::RoundColumn { radius: MAX_RADIUS };
        assert_eq!(p.outward_count(4, MAX_RADIUS), 4, "the four corners of the disc");
        let cells = p.outward_cells(4, MAX_RADIUS);
        let idx = |u: usize, v: usize| 4 * u + v;
        for (u, v) in [(0usize, 0usize), (3, 0), (0, 3), (3, 3)] {
            assert!(cells[idx(u, v)], "({u},{v}) should be outside the disc");
        }
        for (u, v) in [(1usize, 1usize), (2, 2), (1, 2), (2, 1)] {
            assert!(!cells[idx(u, v)], "({u},{v}) should be inside the disc");
        }
        // A smaller disc takes more of the face: the taper change, measured.
        assert_eq!(p.outward_count(4, 0.39), 12, "a 0.39 radius leaves 12 cells outside");
        // The four innermost cell centres sit at `(0.125, 0.125)` from the face's
        // centre, so their radius is exactly `√2/8 ≈ 0.17678` — the smallest disc
        // that can swallow them. Above it they stay inside; below it they do not.
        const INNERMOST_CELL_RADIUS: f64 = 0.176_776_695_629_132_96;
        assert_eq!(p.outward_count(4, 0.18), 12, "0.18 is still outside the inner four");
        assert_eq!(p.outward_count(4, INNERMOST_CELL_RADIUS - 1e-9), 16);
        assert_eq!(p.outward_count(4, INNERMOST_CELL_RADIUS + 1e-9), 12);
        // And on a finer grid the count approaches the area outside the circle:
        // (1 - pi/4) * n^2.
        let n = 64u32;
        let count = p.outward_count(n, MAX_RADIUS) as f64;
        let expected = (1.0 - std::f64::consts::PI / 4.0) * (n * n) as f64;
        assert!(
            (count - expected).abs() / expected < 0.02,
            "a {n}x{n} sample gave {count}, the area says {expected}"
        );
    }

    #[test]
    fn a_taper_makes_the_pattern_differ_from_layer_to_layer() {
        // Correction 3, measured: a stalagmite is *not* a cylinder, so its pattern
        // cannot be one bitmap reused four times.
        let p = Profile::Frustum { low_radius: 0.5, high_radius: 0.2 };
        let counts: Vec<usize> = (0..4)
            .map(|cell| {
                let along = p.axial_fraction(cell as u8, 4);
                p.outward_count(4, p.radius_at(along))
            })
            .collect();
        assert!(
            counts.windows(2).all(|w| w[1] >= w[0]),
            "a taper should remove cells monotonically: {counts:?}"
        );
        assert_ne!(counts[0], counts[3], "the base and the tip must differ");
        // The base is the full radius and so is the corner pattern; the tip is thin.
        assert_eq!(counts[0], 4);
        assert!(counts[3] > counts[0]);
    }

    #[test]
    fn the_daekohler_number_comes_from_the_world_and_not_from_a_constant() {
        // Eq. 9 of the paper, from a base radius and a drip rate. A slower drip on a
        // wide base gives a larger Da and therefore a flatter top — which is the
        // physics the paper states, and the reason the shape is derived.
        let slow = Profile::da_from_flow(0.4, 1e-9).unwrap();
        let fast = Profile::da_from_flow(0.4, 1e-6).unwrap();
        assert!(slow > fast, "a slower drip should give a larger Da");
        assert!(Profile::da_from_flow(0.4, 0.0).is_err());
        assert!(Profile::da_from_flow(0.4, -1.0).is_err());
        assert!(Profile::da_from_flow(0.9, 1e-6).is_err(), "a radius past the block");
        // And the derived value classifies a real case: a slow drip onto a 0.4-wide
        // base is in the flat-top regime, a fast one is conical.
        assert_eq!(Profile::stalagmite(slow, 0.4).unwrap().regime(), Regime::FlatTop);
        assert_eq!(Profile::stalagmite(fast, 0.4).unwrap().regime(), Regime::Conical);
    }

    #[test]
    fn a_degenerate_profile_is_refused_rather_than_clamped() {
        assert!(Profile::stalagmite(0.0, 0.4).is_err());
        assert!(Profile::stalagmite(-1.0, 0.4).is_err());
        assert!(Profile::stalagmite(f64::NAN, 0.4).is_err());
        assert!(Profile::stalagmite(1.0, 0.0).is_err());
        assert!(Profile::stalagmite(1.0, 0.6).is_err(), "wider than its block");
        assert!(Profile::stalagmite(1.0, f64::INFINITY).is_err());
        assert!(Profile::stalagmite(1.0, MAX_RADIUS).is_ok());
    }

    #[test]
    fn every_normal_of_every_profile_is_a_unit_vector_everywhere() {
        // Normalisation is what makes the unbounded slope at a rim safe, so it is
        // checked on a grid that walks through the rim of every profile.
        let profiles = [
            Profile::RoundColumn { radius: 0.4 },
            Profile::Cylinder { radius: 0.5 },
            Profile::Spheroid { radius: 0.3, height: 0.3 },
            Profile::Spheroid { radius: 0.4, height: 0.15 },
            Profile::Frustum { low_radius: 0.5, high_radius: 0.15 },
            Profile::Stalagmite { da: 0.6, radius: 0.45 },
            Profile::Stalagmite { da: 2.5, radius: 0.5 },
        ];
        for p in profiles {
            for face in Face::ALL {
                for i in 0..=16 {
                    for j in 0..=16 {
                        let (u, v) = (i as f64 / 16.0, j as f64 / 16.0);
                        assert_unit(p.cap_normal(face, u, v).unwrap());
                        assert_unit(p.column_normal(face, u, v).unwrap());
                    }
                }
            }
        }
    }

    #[test]
    fn a_coordinate_outside_the_face_is_refused() {
        let p = Profile::RoundColumn { radius: 0.4 };
        assert!(p.cap_normal(Face::PosZ, 1.5, 0.5).is_err());
        assert!(p.cap_normal(Face::PosZ, 0.5, -0.001).is_err());
        assert!(p.cap_normal(Face::PosZ, f64::NAN, 0.5).is_err());
        assert!(p.column_normal(Face::PosZ, f64::INFINITY, 0.5).is_err());
        // The exact edges are inside.
        assert!(p.cap_normal(Face::PosZ, 1.0, 1.0).is_ok());
    }

    #[test]
    fn the_column_axis_and_the_face_basis_agree_about_which_way_is_up() {
        // Getting this wrong rotates a taper ninety degrees and reads as a lighting
        // bug for a week, so the cross-versus-axial choice is a function with a test.
        assert!(column_cross_axis(Face::PosX) && column_cross_axis(Face::NegX));
        assert!(column_cross_axis(Face::PosY) && column_cross_axis(Face::NegY));
        assert!(!column_cross_axis(Face::PosZ) && !column_cross_axis(Face::NegZ));
        // And the basis handed back is the model's own face order.
        for face in Face::ALL {
            let (u_axis, _) = in_plane_axes(face);
            assert_unit(u_axis);
            assert_unit(face_vector(face));
        }
        assert_eq!(SUBVOXEL_CELLS, 4);
    }

    #[test]
    fn the_cross_section_is_the_silhouette_and_not_the_base_radius() {
        // The distinction that was a real bug. A stalagmite's *base* radius is
        // constant, but its **cross-section** narrows upward, and it is the section
        // that decides which cells the body occupies.
        let radius = 0.5;
        let conical = Profile::stalagmite(0.4, radius).unwrap();
        let flat = Profile::stalagmite(3.0, radius).unwrap();

        // Both declare the same constant base radius...
        assert_eq!(conical.radius_at(0.0), conical.radius_at(1.0));
        assert_eq!(flat.radius_at(0.0), flat.radius_at(1.0));
        // ...and their sections differ, and narrow upward.
        let c_base = conical.cross_section_radius(0.0);
        let c_top = conical.cross_section_radius(1.0);
        assert!(c_base > c_top, "a conical stalagmite narrows: {c_base} -> {c_top}");
        assert!(c_base > 0.4, "the base reaches most of the radius, got {c_base}");
        assert!(c_top < 0.15, "the tip is thin, got {c_top}");

        // A flat-topped one keeps its full width much further up, and its very top
        // is the flat cap's own radius.
        assert!(flat.cross_section_radius(0.9) > conical.cross_section_radius(0.9));
        assert!((flat.cross_section_radius(1.0) - flat.flat_cap_radius()).abs() < 1e-6);

        // A cylinder keeps its section everywhere; a frustum is its taper; a
        // spheroid is centred and vanishes outside its own height.
        let cyl = Profile::Cylinder { radius: 0.3 };
        for t in [0.0, 0.5, 1.0] {
            assert_eq!(cyl.cross_section_radius(t), 0.3);
        }
        let f = Profile::Frustum { low_radius: 0.2, high_radius: 0.5 };
        assert_eq!(f.cross_section_radius(0.0), 0.2);
        assert_eq!(f.cross_section_radius(1.0), 0.5);
        let s = Profile::Spheroid { radius: 0.4, height: 0.2 };
        assert!((s.cross_section_radius(0.5) - 0.4).abs() < 1e-12, "widest at the block's centre");
        assert_eq!(s.cross_section_radius(0.0), 0.0, "the spheroid ends before the block does");
        assert_eq!(s.cross_section_radius(1.0), 0.0);
    }

    #[test]
    fn the_axial_fraction_samples_cell_centres_and_not_edges() {
        // The one thing about a sampled pattern that has to be decided, decided once
        // and pinned: cell `i` of `n` samples at `(i + 0.5)/n`.
        let p = Profile::Flat;
        assert_eq!(p.axial_fraction(0, 4), 0.125);
        assert_eq!(p.axial_fraction(1, 4), 0.375);
        assert_eq!(p.axial_fraction(3, 4), 0.875);
        assert_eq!(p.axial_fraction(0, 1), 0.5);
    }
}
