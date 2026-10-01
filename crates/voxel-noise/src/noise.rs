//! The three base noises.
//!
//! * [`perlin3`] — Perlin, K. *An Image Synthesizer.* SIGGRAPH 1985;
//!   *Improving Noise.* SIGGRAPH 2002 (the quintic fade, and the refusal to use
//!   a permutation table whose period is a power of two).
//! * [`value3`] — the lattice value noise Perlin 1985 contrasts his gradient
//!   noise against; kept because several shapers want a *blockier* field than
//!   gradient noise gives.
//! * [`worley3`] — Worley, S. *A Cellular Texture Basis Function.* SIGGRAPH 1996.
//!
//! All three are **seeded by hash, not by a table**, so there is no permutation
//! array to get out of step between threads, and sampling any point in any order
//! gives the same answer (F12).

use crate::point::Vec3;
use crate::rng::{hash3, unit3};

/// Perlin's quintic fade, `6t⁵ − 15t⁴ + 10t³`.
///
/// The 2002 improvement over the original cubic `3t² − 2t³`: the quintic has a
/// zero *second* derivative at the lattice points too, so the seams between cells
/// do not show up as creases in a normal map. That matters here because the mesher
/// computes normals from the field.
#[inline]
fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

#[inline]
fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// Perlin's 12 gradients: the edge midpoints of a cube. The classic set, and it
/// is deliberately **not** the 16-point set some implementations use — the 16
/// version includes axis-aligned duplicates that bias the field toward the axes.
const GRADIENTS: [[f64; 3]; 12] = [
    [1.0, 1.0, 0.0],
    [-1.0, 1.0, 0.0],
    [1.0, -1.0, 0.0],
    [-1.0, -1.0, 0.0],
    [1.0, 0.0, 1.0],
    [-1.0, 0.0, 1.0],
    [1.0, 0.0, -1.0],
    [-1.0, 0.0, -1.0],
    [0.0, 1.0, 1.0],
    [0.0, -1.0, 1.0],
    [0.0, 1.0, -1.0],
    [0.0, -1.0, -1.0],
];

#[inline]
fn gradient(seed: u64, x: i64, y: i64, z: i64) -> Vec3 {
    let g = GRADIENTS[(hash3(seed, x, y, z) % 12) as usize];
    Vec3::new(g[0], g[1], g[2])
}

/// 3D gradient noise, `|v| < 1`.
///
/// **The bound is measured, and the measurement is recorded rather than assumed.**
/// Over a 32³ box at 0.37 spacing the raw field reaches ≈0.98, so no
/// normalisation constant is applied: an earlier revision scaled by `2/√3`
/// (≈1.1547) to "fill the range", which pushed the peak to 1.137 and broke the
/// very bound the comment claimed. A constant chosen to make a field look fuller
/// is a constant chosen to make its bound false.
///
/// Callers that need a guaranteed `[-1, 1]` should use [`Self::clamped`], which
/// states that it clamps.
#[inline]
#[must_use]
pub fn perlin3(seed: u64, p: Vec3) -> f64 {
    let xi = p.x.floor() as i64;
    let yi = p.y.floor() as i64;
    let zi = p.z.floor() as i64;

    let fx = p.x - xi as f64;
    let fy = p.y - yi as f64;
    let fz = p.z - zi as f64;

    let u = fade(fx);
    let v = fade(fy);
    let w = fade(fz);

    let dot = |dx: i64, dy: i64, dz: i64| -> f64 {
        let g = gradient(seed, xi + dx, yi + dy, zi + dz);
        g.dot(Vec3::new(fx - dx as f64, fy - dy as f64, fz - dz as f64))
    };

    let x00 = lerp(dot(0, 0, 0), dot(1, 0, 0), u);
    let x10 = lerp(dot(0, 1, 0), dot(1, 1, 0), u);
    let x01 = lerp(dot(0, 0, 1), dot(1, 0, 1), u);
    let x11 = lerp(dot(0, 1, 1), dot(1, 1, 1), u);

    let y0 = lerp(x00, x10, v);
    let y1 = lerp(x01, x11, v);

    lerp(y0, y1, w)
}

/// [`perlin3`], clamped to `[-1, 1]`.
///
/// The clamp is a **stated** operation, not a hidden one: a caller that needs the
/// bound names it, and a caller that does not gets the unclamped field and the
/// measurement above.
#[inline]
pub fn perlin3_clamped(seed: u64, p: Vec3) -> f64 {
    perlin3(seed, p).clamp(-1.0, 1.0)
}

/// 3D lattice **value** noise in `[-1, 1]`: random values at lattice points,
/// interpolated with the same quintic fade.
#[inline]
pub fn value3(seed: u64, p: Vec3) -> f64 {
    let xi = p.x.floor() as i64;
    let yi = p.y.floor() as i64;
    let zi = p.z.floor() as i64;

    let fx = p.x - xi as f64;
    let fy = p.y - yi as f64;
    let fz = p.z - zi as f64;

    let u = fade(fx);
    let v = fade(fy);
    let w = fade(fz);

    let v_at = |dx: i64, dy: i64, dz: i64| unit3(seed ^ 0xC0FF_EE00, xi + dx, yi + dy, zi + dz) * 2.0 - 1.0;

    let x00 = lerp(v_at(0, 0, 0), v_at(1, 0, 0), u);
    let x10 = lerp(v_at(0, 1, 0), v_at(1, 1, 0), u);
    let x01 = lerp(v_at(0, 0, 1), v_at(1, 0, 1), u);
    let x11 = lerp(v_at(0, 1, 1), v_at(1, 1, 1), u);

    lerp(lerp(x00, x10, v), lerp(x01, x11, v), w)
}

/// 3D cellular (Worley) noise: the distance to the **nearest feature point**,
/// normalised so `0` is at a feature point and `1` is the far corner of a cell.
///
/// F1 only (`Worley 1996`). The 27-neighbour search is required — a 1-cell search
/// misses the feature point of a neighbouring cell that is closer than this
/// cell's own, which is the classic cellular-noise seam.
#[inline]
pub fn worley3(seed: u64, p: Vec3) -> f64 {
    let xi = p.x.floor() as i64;
    let yi = p.y.floor() as i64;
    let zi = p.z.floor() as i64;

    let mut best = f64::INFINITY;
    for dx in -1..=1i64 {
        for dy in -1..=1i64 {
            for dz in -1..=1i64 {
                let (cx, cy, cz) = (xi + dx, yi + dy, zi + dz);
                // One feature point per cell, uniform inside it.
                let f = Vec3::new(
                    cx as f64 + unit3(seed ^ 0x5EED_1234, cx, cy, cz),
                    cy as f64 + unit3(seed ^ 0x5EED_5678, cx, cy, cz),
                    cz as f64 + unit3(seed ^ 0x5EED_9ABC, cx, cy, cz),
                );
                let d = (f - p).length();
                if d < best {
                    best = d;
                }
            }
        }
    }
    // A cell is 1 unit wide and a point can be at most √3/2 ≈ 0.866 from the
    // nearest feature point on average, but the true worst case is √3 ≈ 1.732.
    // Clamped small so the field stays in [0, 1] with the clamp *named*.
    (best / 1.732_050_807_568_877_2).min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_box(f: impl Fn(Vec3) -> f64) -> (f64, f64) {
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        for x in -16..16 {
            for y in -16..16 {
                for z in -16..16 {
                    let v = f(Vec3::new(x as f64 * 0.37, y as f64 * 0.37, z as f64 * 0.37));
                    assert!(v.is_finite(), "non-finite noise at ({x},{y},{z})");
                    lo = lo.min(v);
                    hi = hi.max(v);
                }
            }
        }
        (lo, hi)
    }

    #[test]
    fn perlin_is_deterministic_and_spans_its_range() {
        assert_eq!(
            perlin3(1, Vec3::new(1.1, 2.2, 3.3)),
            perlin3(1, Vec3::new(1.1, 2.2, 3.3))
        );
        let (lo, hi) = sample_box(|p| perlin3(1, p));
        assert!(lo >= -1.0 && hi <= 1.0, "perlin range was [{lo}, {hi}]");
        // And it must actually **use** the range: a field that barely moves is a
        // flat world, and the measured span is the thing that catches it.
        assert!(hi - lo > 1.4, "perlin only spanned {lo}..{hi}");
    }

    #[test]
    fn the_stated_clamp_is_the_stated_bound() {
        for x in -20..20 {
            let p = Vec3::new(x as f64 * 0.37, 1.1, 2.2);
            assert!((-1.0..=1.0).contains(&perlin3_clamped(1, p)));
        }
    }

    #[test]
    fn perlin_is_continuous_so_the_mesher_can_take_normals() {
        // A discontinuity would appear as a crease in every derived normal.
        let mut prev = perlin3(9, Vec3::ZERO);
        let mut x = 0.0;
        while x < 20.0 {
            x += 0.01;
            let v = perlin3(9, Vec3::new(x, 0.0, 0.0));
            assert!((v - prev).abs() < 0.1, "jump of {} at x = {x}", (v - prev).abs());
            prev = v;
        }
    }

    #[test]
    fn perlin_is_zero_at_lattice_points() {
        // Gradient noise is 0 at its lattice points **by construction** — the
        // gradients are dotted with a zero offset there. If this fails, the
        // offsets are wrong.
        for i in -3..3i64 {
            assert!(perlin3(5, Vec3::new(i as f64, i as f64, i as f64)).abs() < 1e-12);
        }
    }

    #[test]
    fn value_noise_is_in_range_and_blocky() {
        let (lo, hi) = sample_box(|p| value3(3, p));
        assert!(lo >= -1.001 && hi <= 1.001, "value range was [{lo}, {hi}]");
        assert!(hi - lo > 1.2);
    }

    #[test]
    fn worley_is_in_unit_range_and_has_zeroes_at_feature_points() {
        let (lo, hi) = sample_box(|p| worley3(7, p));
        assert!(lo >= 0.0 && hi <= 1.0, "worley range was [{lo}, {hi}]");
        assert!(lo < 0.05, "worley never got near a feature point: {lo}");
    }

    #[test]
    fn the_27_neighbour_search_is_actually_needed() {
        // A 1-cell search would leave a visible grid. Sample along a line and
        // check the field never spikes at a cell boundary, which is the seam a
        // too-small search produces.
        let mut prev = worley3(11, Vec3::new(0.0, 0.0, 0.0));
        let mut x = 0.0;
        while x < 10.0 {
            x += 0.05;
            let v = worley3(11, Vec3::new(x, 0.25, 0.25));
            assert!(
                (v - prev).abs() < 0.5,
                "worley jumped {} at x = {x}: the neighbour search is too small",
                (v - prev).abs()
            );
            prev = v;
        }
    }

    #[test]
    fn different_seeds_give_different_fields() {
        let a = perlin3(1, Vec3::new(0.3, 0.7, 1.1));
        let b = perlin3(2, Vec3::new(0.3, 0.7, 1.1));
        assert_ne!(a, b);
    }
}
