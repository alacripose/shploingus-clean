//! Picking a voxel with a ray — Amanatides & Woo (1987), *A Fast Voxel
//! Traversal Algorithm for Ray Tracing*.
//!
//! The algorithm walks the voxel grid one **plane crossing** at a time, and
//! never visits a voxel the ray does not pass through. That matters here for a
//! reason beyond speed: a naive "check every voxel along the ray" implementation
//! has to decide how many to check, and any bound it picks is a bound that is
//! wrong for some ray. Plane crossings have no bound to pick.
//!
//! # What a hit reports
//!
//! * the voxel,
//! * the **face it was entered through** —
//!   [`Hit::entry_face`] is `None` when the ray *began* inside the voxel,
//!   because at zero distance there is no entry face and inventing one would be
//!   a default pretending to be a measurement,
//! * the subvoxel the entry point lands in, so a dig can address the 4×4×4 grid
//!   rather than the whole block.
//!
//! # One honest limit
//!
//! The hit is resolved to the **voxel** boundary, and the subvoxel is the one
//! containing the entry point. A `Mixed` voxel whose visible geometry is set back
//! from its own boundary — a partial shape — is therefore picked at the boundary
//! it was entered through, which is the subvoxel the ray is *in*. Picking real
//! subvoxel geometry needs a second traversal at four times the resolution
//! inside the hit voxel; that is a declared refinement, not an oversight, and it
//! is the only thing standing between this and exact shape picking.

use voxel_model::{Face, LocalPos, CHUNK_SIDE, SUBVOXEL_SIDE};

use crate::octree::{local_from, SparseVoxelOctree};

/// A ray in **voxel** units, so one unit is one metre. `dir` need not be
/// normalised; [`Hit::distance`] is in units of `dir`'s length.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Ray {
    pub origin: [f64; 3],
    pub dir: [f64; 3],
}

impl Ray {
    pub const fn new(origin: [f64; 3], dir: [f64; 3]) -> Self {
        Ray { origin, dir }
    }
}

/// Where a ray first met matter.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Hit {
    pub local: LocalPos,
    /// The face of `local` the ray came in through, or `None` if the ray started
    /// inside it.
    pub entry_face: Option<Face>,
    /// Which of the voxel's 4×4×4 subvoxels the entry point is in.
    pub subvoxel: voxel_model::SubvoxelIndex,
    /// Distance along `dir`, in voxel units.
    pub distance: f64,
}

/// Whether the ray has left the chunk for good on some axis.
///
/// Without this the octree walk would be bounded by a number chosen in advance,
/// which is the kind of bound that is silently too small. Once an axis is past
/// the far side **in the direction of travel** the ray can never come back, and
/// once a stationary axis sits outside the chunk it never will be inside. Note
/// that an axis *below* zero with a positive step is not an escape: the ray is
/// still on its way in, and a bound that refused it would make every ray that
/// starts outside the chunk miss.
fn escapes(v: &[i32; 3], step: &[i32; 3]) -> bool {
    for i in 0..3 {
        if step[i] > 0 && v[i] >= CHUNK_SIDE {
            return true;
        }
        if step[i] < 0 && v[i] < 0 {
            return true;
        }
        if step[i] == 0 && !(0..CHUNK_SIDE).contains(&v[i]) {
            return true;
        }
    }
    false
}

/// One plane crossing: the voxel entered, the face it was entered through, and
/// the distance along `dir` at which it happened.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Crossing {
    pub voxel: [i32; 3],
    /// `None` for the voxel the ray **starts** in, because at zero distance
    /// there is no entry face and inventing one would be a default pretending to
    /// be a measurement.
    pub entry_face: Option<Face>,
    /// Distance along `dir`, in voxel units.
    pub t: f64,
}

/// The voxels a ray crosses, in order — Amanatides & Woo's traversal, as an
/// iterator so that the **same** walk serves an octree and a world that has no
/// octree to walk (a picking ray asks the generator voxel by voxel).
///
/// [`Crossings::new`] returns `None` for a **degenerate** ray — a NaN
/// coordinate, a non-finite direction, or a zero direction — rather than an
/// empty walk, because "this is not a ray" and "this ray crosses nothing" are
/// different claims and a caller may need to tell them apart.
///
/// A zero component is legal in an otherwise non-zero direction: its axis is
/// never chosen (`t_max` stays infinite), so the walk slides along the grid
/// plane exactly as the geometry says it should.
pub struct Crossings {
    v: [i32; 3],
    step: [i32; 3],
    t_max: [f64; 3],
    t_delta: [f64; 3],
    max_distance: f64,
    t: f64,
    entry_face: Option<Face>,
    started: bool,
    done: bool,
}

impl Crossings {
    /// The per-axis step this walk takes. Constant for the whole walk, and
    /// public because a caller bounding the walk by a **box** (an octree's chunk)
    /// needs it to tell "this axis can never return" from "this axis has not
    /// arrived yet".
    pub const fn step(&self) -> [i32; 3] {
        self.step
    }

    /// The walk of `ray`, or `None` when the ray is degenerate.
    pub fn new(ray: &Ray, max_distance: f64) -> Option<Crossings> {
        for i in 0..3 {
            if !ray.origin[i].is_finite() || !ray.dir[i].is_finite() {
                return None;
            }
        }
        if ray.dir.iter().all(|d| *d == 0.0) || !max_distance.is_finite() || max_distance <= 0.0 {
            return None;
        }

        let mut v = [0i32; 3];
        for (slot, origin) in v.iter_mut().zip(ray.origin.iter()) {
            *slot = origin.floor() as i32;
        }

        let mut step = [0i32; 3];
        let mut t_max = [f64::INFINITY; 3];
        let mut t_delta = [f64::INFINITY; 3];
        for i in 0..3 {
            let d = ray.dir[i];
            if d > 0.0 {
                step[i] = 1;
                t_delta[i] = 1.0 / d;
                t_max[i] = ((v[i] + 1) as f64 - ray.origin[i]) / d;
            } else if d < 0.0 {
                step[i] = -1;
                t_delta[i] = -1.0 / d;
                t_max[i] = (v[i] as f64 - ray.origin[i]) / d;
            }
            // A zero direction leaves `t_max` infinite, so that axis is never
            // chosen — which is exactly right, and needs no special case.
        }

        Some(Crossings {
            v,
            step,
            t_max,
            t_delta,
            max_distance,
            t: 0.0,
            entry_face: None,
            started: false,
            done: false,
        })
    }
}

impl Iterator for Crossings {
    type Item = Crossing;

    fn next(&mut self) -> Option<Crossing> {
        if self.done {
            return None;
        }
        if !self.started {
            self.started = true;
            return Some(Crossing { voxel: self.v, entry_face: None, t: 0.0 });
        }

        // Cross the nearest plane.
        let axis = if self.t_max[0] <= self.t_max[1] && self.t_max[0] <= self.t_max[2] {
            0
        } else if self.t_max[1] <= self.t_max[2] {
            1
        } else {
            2
        };
        self.t = self.t_max[axis];
        self.v[axis] += self.step[axis];
        self.t_max[axis] += self.t_delta[axis];

        // Entering a voxel by moving in `+axis` means coming in through its
        // `-axis` face, and the other way round.
        self.entry_face = Some(if self.step[axis] > 0 {
            match axis {
                0 => Face::NegX,
                1 => Face::NegY,
                _ => Face::NegZ,
            }
        } else {
            match axis {
                0 => Face::PosX,
                1 => Face::PosY,
                _ => Face::PosZ,
            }
        });

        if self.t > self.max_distance {
            self.done = true;
            return None;
        }
        Some(Crossing { voxel: self.v, entry_face: self.entry_face, t: self.t })
    }
}

/// The point `t` along a ray, in voxel units.
fn point_at(origin: [f64; 3], dir: [f64; 3], t: f64) -> [f64; 3] {
    [
        origin[0] + t * dir[0],
        origin[1] + t * dir[1],
        origin[2] + t * dir[2],
    ]
}

/// The subvoxel a point falls in, clamped to the voxel it is in.
///
/// Clamped rather than refused because a point exactly on a voxel boundary
/// lands on fraction `1.0`, which is the next voxel's `0.0` — and a hit is
/// reported at the boundary by construction, so this case is the common one
/// rather than an error.
fn subvoxel_of(point: [f64; 3], voxel: [i32; 3]) -> voxel_model::SubvoxelIndex {
    let mut at = [0u8; 3];
    for i in 0..3 {
        let fraction = (point[i] - voxel[i] as f64) * SUBVOXEL_SIDE as f64;
        let clamped = fraction.floor().clamp(0.0, (SUBVOXEL_SIDE - 1) as f64);
        at[i] = clamped as u8;
    }
    // Constructed directly rather than through `SubvoxelIndex::new`: the clamp
    // above is the proof that these are in `0..4`, and a refusal path here would
    // be a branch that can never be taken pretending to be defensive.
    voxel_model::SubvoxelIndex { x: at[0], y: at[1], z: at[2] }
}

impl SparseVoxelOctree {
    /// The first voxel along `ray`, up to `max_distance` voxels away.
    ///
    /// Returns `None` for a ray that misses, a ray that leaves the chunk, and a
    /// **degenerate** ray — a NaN coordinate or a zero direction — rather than
    /// treating a degenerate ray as one that misses at distance zero.
    pub fn raycast_local(&self, ray: &Ray, max_distance: f64) -> Option<Hit> {
        let walk = Crossings::new(ray, max_distance)?;
        let step = walk.step();
        for crossing in walk {
            if escapes(&crossing.voxel, &step) {
                return None;
            }
            if let Ok(local) = local_from(crossing.voxel) {
                if !self.voxel(local).is_empty() {
                    let point = point_at(ray.origin, ray.dir, crossing.t);
                    return Some(Hit {
                        local,
                        entry_face: crossing.entry_face,
                        subvoxel: subvoxel_of(point, crossing.voxel),
                        distance: crossing.t,
                    });
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::octree::local_from;
    use voxel_model::{MaterialId, VoxelShape};

    const STONE: MaterialId = MaterialId(6);

    fn tree_with(cells: &[[i32; 3]]) -> SparseVoxelOctree {
        let mut t = SparseVoxelOctree::new();
        for c in cells {
            t.set_voxel(local_from(*c).unwrap(), VoxelShape::solid(STONE));
        }
        t
    }

    #[test]
    fn a_ray_down_an_axis_hits_the_voxel_and_names_the_face() {
        let t = tree_with(&[[0, 0, 0]]);
        let hit = t
            .raycast_local(&Ray::new([0.5, 0.5, -5.0], [0.0, 0.0, 1.0]), 100.0)
            .expect("the ray should hit");
        assert_eq!(hit.local, local_from([0, 0, 0]).unwrap());
        assert_eq!(hit.entry_face, Some(Face::NegZ));
        assert!((hit.distance - 5.0).abs() < 1e-9);
        assert_eq!(hit.subvoxel, voxel_model::SubvoxelIndex { x: 2, y: 2, z: 0 });
    }

    #[test]
    fn a_ray_that_misses_returns_nothing() {
        let t = tree_with(&[[0, 0, 0]]);
        // Straight past at y = 10.
        assert!(t
            .raycast_local(&Ray::new([-5.0, 10.5, 0.5], [1.0, 0.0, 0.0]), 100.0)
            .is_none());
    }

    #[test]
    fn a_zero_direction_on_one_axis_does_not_break_the_traversal() {
        // The x direction is exactly zero, so t_max[x] is infinite and the walk
        // must simply never advance x. A `1.0/0.0` that produced a finite value
        // here would step x every iteration and miss everything.
        let t = tree_with(&[[4, 4, 4]]);
        let hit = t
            .raycast_local(&Ray::new([4.5, 4.5, -3.0], [0.0, 0.0, 2.5]), 100.0)
            .unwrap();
        assert_eq!(hit.local, local_from([4, 4, 4]).unwrap());
        assert_eq!(hit.entry_face, Some(Face::NegZ));
        // `dir` has length 2.5, so seven metres of travel is a `distance` of
        // 7 / 2.5 = 2.8 — the distance is in units of the direction's length.
        assert!((hit.distance - 2.8).abs() < 1e-9, "distance was {}", hit.distance);
    }

    #[test]
    fn a_ray_that_begins_inside_reports_no_entry_face() {
        let t = tree_with(&[[3, 3, 3]]);
        let hit = t
            .raycast_local(&Ray::new([3.5, 3.5, 3.5], [1.0, 0.0, 0.0]), 100.0)
            .unwrap();
        assert_eq!(hit.entry_face, None, "no entry face exists at zero distance");
        assert_eq!(hit.distance, 0.0);
    }

    #[test]
    fn the_first_voxel_along_the_ray_wins_not_the_nearest_to_the_origin() {
        let t = tree_with(&[[0, 0, 0], [0, 0, 5]]);
        let hit = t
            .raycast_local(&Ray::new([0.5, 0.5, -1.0], [0.0, 0.0, 1.0]), 100.0)
            .unwrap();
        assert_eq!(hit.local, local_from([0, 0, 0]).unwrap());
    }

    #[test]
    fn max_distance_is_respected() {
        let t = tree_with(&[[0, 0, 0]]);
        let ray = Ray::new([0.5, 0.5, -5.0], [0.0, 0.0, 1.0]);
        assert!(t.raycast_local(&ray, 4.0).is_none(), "the hit is at 5, not 4");
        assert!(t.raycast_local(&ray, 5.5).is_some());
    }

    #[test]
    fn a_ray_pointing_away_from_the_chunk_terminates_immediately() {
        // The test that would hang without the `escapes` check, so it is also
        // the test that the loop has no arbitrary iteration bound.
        let t = tree_with(&[[0, 0, 0]]);
        assert!(t
            .raycast_local(&Ray::new([-10.0, 0.5, 0.5], [-1.0, 0.0, 0.0]), 1e9)
            .is_none());
        assert!(t
            .raycast_local(&Ray::new([0.5, 0.5, 40.0], [0.0, 0.0, 1.0]), 1e9)
            .is_none());
        assert!(t
            .raycast_local(&Ray::new([0.5, -3.0, 0.5], [0.0, 0.0, 0.0]), 1e9)
            .is_none());
    }

    #[test]
    fn a_degenerate_ray_is_none_rather_than_a_hit_at_zero_distance() {
        let t = tree_with(&[[0, 0, 0]]);
        assert!(t
            .raycast_local(&Ray::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]), 10.0)
            .is_none());
        assert!(t
            .raycast_local(&Ray::new([f64::NAN, 0.0, 0.0], [1.0, 0.0, 0.0]), 10.0)
            .is_none());
        assert!(t
            .raycast_local(&Ray::new([0.0, 0.0, 0.0], [f64::INFINITY, 0.0, 0.0]), 10.0)
            .is_none());
    }

    #[test]
    fn the_walk_yields_the_starting_voxel_then_the_crossings_in_order() {
        let ray = Ray::new([0.5, 0.5, 0.5], [1.0, 0.0, 0.0]);
        let walk: Vec<Crossing> = Crossings::new(&ray, 3.5).unwrap().collect();
        assert_eq!(
            walk.iter().map(|c| c.voxel).collect::<Vec<_>>(),
            vec![[0, 0, 0], [1, 0, 0], [2, 0, 0], [3, 0, 0], [4, 0, 0]]
        );
        assert_eq!(walk[0].entry_face, None, "the starting voxel has no entry face");
        assert_eq!(walk[0].t, 0.0);
        assert_eq!(walk[1].entry_face, Some(Face::NegX));
        assert_eq!(walk[1].t, 0.5);
        assert_eq!(walk[2].t, 1.5);
        // A crossing at **exactly** the reach is inside it, and one past it is
        // not — the boundary is inclusive and the test says so.
        assert_eq!(walk.last().unwrap().t, 3.5);
        let walk: Vec<Crossing> = Crossings::new(&ray, 3.4).unwrap().collect();
        assert_eq!(walk.last().unwrap().voxel, [3, 0, 0]);
        assert_eq!(walk.last().unwrap().t, 2.5);
    }

    #[test]
    fn the_walk_stops_at_the_reach_rather_than_running_on() {
        let ray = Ray::new([0.5, 0.5, 0.5], [1.0, 0.0, 0.0]);
        let walk: Vec<Crossing> = Crossings::new(&ray, 2.0).unwrap().collect();
        assert_eq!(walk.iter().map(|c| c.voxel[0]).collect::<Vec<_>>(), vec![0, 1, 2]);
        // And a shorter reach yields fewer.
        let walk: Vec<Crossing> = Crossings::new(&ray, 1.0).unwrap().collect();
        assert_eq!(walk.iter().map(|c| c.voxel[0]).collect::<Vec<_>>(), vec![0, 1]);
    }

    #[test]
    fn a_degenerate_ray_has_no_walk() {
        assert!(Crossings::new(&Ray::new([f64::NAN, 0.0, 0.0], [1.0, 0.0, 0.0]), 5.0).is_none());
        assert!(Crossings::new(&Ray::new([0.0; 3], [0.0; 3]), 5.0).is_none());
        assert!(Crossings::new(&Ray::new([0.0; 3], [1.0, 0.0, 0.0]), 0.0).is_none());
        assert!(Crossings::new(&Ray::new([0.0; 3], [1.0, 0.0, 0.0]), f64::INFINITY).is_none());
    }

    #[test]
    fn the_walk_is_the_walk_the_octree_uses() {
        // The property the refactor exists for: the octree's hit is the walk's
        // first non-empty crossing, at the same distance and through the same
        // face. A second traversal written for the world would not be pinned to
        // this one; this is.
        let t = tree_with(&[[4, 4, 4]]);
        let ray = Ray::new([0.5, 4.5, 4.5], [1.0, 0.0, 0.0]);
        let hit = t.raycast_local(&ray, 100.0).unwrap();
        let crossing = Crossings::new(&ray, 100.0)
            .unwrap()
            .find(|c| c.voxel == [4, 4, 4])
            .expect("the walk crossed the hit voxel");
        assert_eq!(crossing.entry_face, hit.entry_face);
        assert!((crossing.t - hit.distance).abs() < 1e-12);
    }

    #[test]
    fn a_ray_across_a_diagonal_reaches_the_voxel_it_should() {
        // (0,0,0) → (4,4,0) with a 45° ray in the xy plane.
        let t = tree_with(&[[4, 3, 0]]);
        let hit = t
            .raycast_local(&Ray::new([0.5, -0.5, 0.5], [1.0, 1.0, 0.0]), 100.0)
            .unwrap();
        assert_eq!(hit.local, local_from([4, 3, 0]).unwrap());
        // Moving in `+x` or `+y` means entering through the voxel's `-x` or
        // `-y` face. Which of the two depends on which plane crossed first, and
        // on an exact tie the axis order decides — so the test accepts either
        // rather than pinning a tie.
        assert!(matches!(hit.entry_face, Some(Face::NegX) | Some(Face::NegY)));
    }
}
