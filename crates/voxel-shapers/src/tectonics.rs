//! Tectonics — the plate partition that geology, terrain and deposits all hang off.
//!
//! **Source.** Kreischer, Bommes et al. *Procedural Tectonic Planets.* Computer
//! Graphics Forum 2019, for the plate-partition-plus-boundary-taxonomy shape; the
//! boundary classes (convergent / divergent / transform) are the standard Wilson
//! cycle vocabulary. The implementation here is a **2D Voronoi partition over a
//! jittered grid** — the same structure Worley (1996) uses for cellular noise,
//! with the cell identity promoted from a distance to a *plate*.
//!
//! # Why a partition and not a noise field
//!
//! A mountain range made of noise is a mountain range with no reason. A range made
//! by pushing two plates together has a *reason*, and the reason is visible in the
//! world: the uplift is high in a band along the boundary and falls off with
//! distance from it, so the ranges form lines rather than blobs. That is the
//! difference the paper is about, and it is the difference the terrain shaper
//! downstream depends on.
//!
//! **Determinism.** Every value is a pure function of `(seed, x, z)`. The
//! 3×3-neighbour search is the same one [`voxel_noise::worley3`] needs, and for
//! the same reason: a 1-cell search leaves a visible grid at plate seams.

use crate::spec::WorldSpec;
use voxel_noise::{mix, unit2, Vec3};

/// What a plate boundary is doing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BoundaryKind {
    /// Plates moving together: **uplift** (orogeny).
    Convergent,
    /// Plates moving apart: **subsidence** (rifting).
    Divergent,
    /// Plates sliding past: neither, but the crust is broken.
    Transform,
}

impl BoundaryKind {
    /// The sign this boundary applies to its uplift, as a multiplier on the
    /// boundary strength. Transform boundaries break rock without moving it
    /// vertically, which is why their multiplier is zero and not a small number.
    pub const fn uplift_sign(self) -> f64 {
        match self {
            BoundaryKind::Convergent => 1.0,
            BoundaryKind::Divergent => -0.35,
            BoundaryKind::Transform => 0.0,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            BoundaryKind::Convergent => "convergent",
            BoundaryKind::Divergent => "divergent",
            BoundaryKind::Transform => "transform",
        }
    }
}

/// What tectonics says about one column of the world.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct PlateSample {
    /// The plate this column sits on.
    pub plate_id: u32,
    /// Distance to the nearest plate's feature point, in metres.
    pub nearest_m: f64,
    /// Distance to the **second** nearest, in metres. The gap between the two is
    /// the boundary proximity signal.
    pub second_m: f64,
    /// How close to a boundary this column is: `1` on the seam, falling to `0`
    /// with an exponential length scale of `config.plate_boundary_width_m`.
    ///
    /// **This is an absolute falloff, not a ratio**, and that distinction was
    /// measured rather than assumed. Normalising the distance-to-seam by the
    /// plate cell size gives ≈0.5 *everywhere*, because a Voronoi cell has no
    /// deep interior relative to its own diameter — so a ratio cannot reach
    /// zero, and the terrain shaper built on it grew mountains in the middle of
    /// stable plates. A belt has a **width**, and a width is a length.
    pub boundary_strength: f64,
    /// The kind of boundary, meaningful only where `boundary_strength` is
    /// non-trivial.
    pub boundary_kind: BoundaryKind,
    /// Vertical displacement this plate geometry contributes, in metres.
    pub uplift_m: f64,
}

impl PlateSample {
    /// True when the column is close enough to a boundary for the boundary to
    /// matter. `0.5` is a **declared** proximity cut-off, not a tuned one: it is
    /// `ln(2) ≈ 0.69` of a belt half-width in from the seam.
    pub const BOUNDARY_PROXIMITY: f64 = 0.5;

    pub fn on_boundary(&self) -> bool {
        self.boundary_strength >= Self::BOUNDARY_PROXIMITY
    }
}

/// The jittered-grid cell a point falls in, and the feature point inside it.
fn feature_point(spec: &WorldSpec, i: i64, j: i64) -> Vec3 {
    let seed = spec.stage_seed("tectonics");
    let cell = spec.config.plate_cell_size_m;
    // Jitter is confined to [0.15, 0.85] of the cell so that plates keep a
    // positive size: jitter over the full cell lets two feature points collide
    // and one plate vanish, which is a partition that is not a partition.
    let jx = 0.15 + 0.7 * unit2(seed ^ 0x11, i, j);
    let jz = 0.15 + 0.7 * unit2(seed ^ 0x22, i, j);
    Vec3::new((i as f64 + jx) * cell, 0.0, (j as f64 + jz) * cell)
}

fn plate_id_of(spec: &WorldSpec, i: i64, j: i64) -> u32 {
    let n = spec.config.plate_count.max(1) as u64;
    (mix(spec.stage_seed("tectonics") ^ ((i as u64) << 32) ^ (j as u64)) % n) as u32
}

/// Sample tectonics at a horizontal position. `y` is ignored: plates are a
/// horizontal partition, and pretending otherwise would give a column two plates.
pub fn sample(spec: &WorldSpec, x: f64, z: f64) -> PlateSample {
    let cell = spec.config.plate_cell_size_m;
    let ci = (x / cell).floor() as i64;
    let cj = (z / cell).floor() as i64;

    let p = Vec3::new(x, 0.0, z);

    // (distance, feature point) for the two nearest features.
    let mut nearest = (f64::INFINITY, Vec3::ZERO);
    let mut second = (f64::INFINITY, Vec3::ZERO);
    let mut nearest_cell = (0i64, 0i64);
    let mut second_cell = (0i64, 0i64);

    for di in -1..=1i64 {
        for dj in -1..=1i64 {
            let (i, j) = (ci + di, cj + dj);
            let f = feature_point(spec, i, j);
            // Horizontal distance only: this is a 2D partition.
            let dx = f.x - p.x;
            let dz = f.z - p.z;
            let d = (dx * dx + dz * dz).sqrt();
            if d < nearest.0 {
                second = nearest;
                second_cell = nearest_cell;
                nearest = (d, f);
                nearest_cell = (i, j);
            } else if d < second.0 {
                second = (d, f);
                second_cell = (i, j);
            }
        }
    }

    let plate_id = plate_id_of(spec, nearest_cell.0, nearest_cell.1);
    let other = plate_id_of(spec, second_cell.0, second_cell.1);

    // **Distance to the seam**, exactly: for two feature points A and B with
    // \|pA\| = d1 and \|pB\| = d2 and \|AB\| = L, the perpendicular bisector is
    // at `(d2² − d1²) / (2L)` from p. That is the Voronoi edge, and it needs no
    // tuning constant — which is the whole reason to prefer it to a ratio.
    let (fa, fb) = (nearest.1, second.1);
    let l = ((fb.x - fa.x).powi(2) + (fb.z - fa.z).powi(2)).sqrt();
    let edge_m = if l > 1e-9 {
        ((second.0 * second.0 - nearest.0 * nearest.0) / (2.0 * l)).abs()
    } else {
        f64::INFINITY
    };

    // The belt's width is a **declared length**, so the falloff is a physical
    // distance rather than a normalised coordinate that can never reach zero.
    let proximity = (-edge_m / spec.config.plate_boundary_width_m).exp().clamp(0.0, 1.0);

    // The boundary's kind is a property of the **plate pair**, not of the point:
    // two plates meeting are convergent along their whole shared edge. Hashing
    // the pair (order-independent) is what makes that true.
    let (a, b) = if plate_id <= other { (plate_id, other) } else { (other, plate_id) };
    let kind = match mix(spec.stage_seed("tectonics") ^ 0xB0_1234 ^ ((a as u64) << 20) ^ (b as u64)) % 3 {
        0 => BoundaryKind::Convergent,
        1 => BoundaryKind::Divergent,
        _ => BoundaryKind::Transform,
    };

    let uplift = proximity * kind.uplift_sign() * spec.config.relief_m * 0.45;

    PlateSample {
        plate_id,
        nearest_m: nearest.0,
        second_m: second.0,
        boundary_strength: proximity,
        boundary_kind: kind,
        uplift_m: uplift,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Config;

    fn spec() -> WorldSpec {
        WorldSpec::new(2026, Config::default())
    }

    #[test]
    fn sampling_is_deterministic() {
        let s = spec();
        let a = sample(&s, 123.4, 567.8);
        let b = sample(&s, 123.4, 567.8);
        assert_eq!(a, b);
    }

    #[test]
    fn a_plate_id_is_stable_across_a_plate_interior() {
        // Move a little inside one plate and the plate must not change.
        let s = spec();
        let origin = sample(&s, 0.0, 0.0);
        let near = sample(&s, 5.0, 5.0);
        assert_eq!(origin.plate_id, near.plate_id, "plate changed 7 m from the origin");
    }

    #[test]
    fn boundary_strength_rises_toward_a_seam() {
        // Walk across the world and check that **somewhere** the boundary
        // strength approaches 1, and that most places it does not. If the
        // partition were empty or degenerate, one of these fails.
        let s = spec();
        let mut max = 0.0f64;
        let mut interior = 0usize;
        let mut total = 0usize;
        let mut x = -20000.0;
        while x < 20000.0 {
            let p = sample(&s, x, 17.0);
            assert!((0.0..=1.0).contains(&p.boundary_strength));
            max = max.max(p.boundary_strength);
            if p.boundary_strength < 0.3 {
                interior += 1;
            }
            total += 1;
            x += 13.0;
        }
        assert!(max > 0.95, "no seam was ever reached: max {max}");
        let frac = interior as f64 / total as f64;
        assert!(frac > 0.4, "the world is mostly seam: interior fraction {frac}");
    }

    #[test]
    fn convergent_boundaries_raise_and_divergent_ones_lower() {
        // The uplift sign is the shaper's whole contribution to terrain, so it
        // is checked against the kind rather than against a magic number.
        let s = spec();
        let mut saw_uplift = false;
        let mut saw_subsidence = false;
        let mut x = -30000.0;
        while x < 30000.0 && !(saw_uplift && saw_subsidence) {
            let p = sample(&s, x, 33.0);
            if p.boundary_strength > 0.9 {
                match p.boundary_kind {
                    BoundaryKind::Convergent => {
                        assert!(p.uplift_m > 0.0);
                        saw_uplift = true;
                    }
                    BoundaryKind::Divergent => {
                        assert!(p.uplift_m < 0.0);
                        saw_subsidence = true;
                    }
                    BoundaryKind::Transform => assert_eq!(p.uplift_m, 0.0),
                }
            }
            x += 11.0;
        }
        assert!(saw_uplift, "no convergent boundary was found to check");
        assert!(saw_subsidence, "no divergent boundary was found to check");
    }

    #[test]
    fn the_partition_has_no_seam_grid() {
        // The 3x3 search is what prevents a visible grid at cell edges. A 1-cell
        // search shows up as a discontinuity in the nearest-feature distance at
        // every cell boundary. Check the field is continuous along a line.
        let s = spec();
        let mut prev = sample(&s, 0.0, 0.0).nearest_m;
        let mut x = 0.0;
        while x < 20000.0 {
            x += 5.0;
            let d = sample(&s, x, 0.0).nearest_m;
            assert!(
                (d - prev).abs() < 80.0,
                "nearest-feature distance jumped {} at x = {x}: the neighbour search is too small",
                (d - prev).abs()
            );
            prev = d;
        }
    }

    #[test]
    fn different_seeds_partition_differently() {
        let a = sample(&WorldSpec::new(1, Config::default()), 400.0, 400.0);
        let b = sample(&WorldSpec::new(2, Config::default()), 400.0, 400.0);
        assert_ne!(a.plate_id, b.plate_id);
    }
}
