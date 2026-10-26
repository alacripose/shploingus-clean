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

use voxel_model::{
    Face, LocalPos, MaterialId, SubvoxelIndex, CHUNK_SIDE, SUBVOXEL_SIDE,
};

use crate::octree::{local_from, Node, SparseVoxelOctree, SUBVOXEL_SIDE_I32};

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

    /// **The ray's own path through this tree, one occupied node at a time** — the walk that
    /// makes the tree an accelerator rather than a lookup table.
    ///
    /// [`Self::raycast_local`] steps the grid a voxel at a time and asks the tree whether each
    /// one is empty. That answers *picking* (one hit, a metre away) and it is the wrong cost for
    /// a **march**: a beam from a ridge to the Sun, or a shadow ray from a cave floor, crosses
    /// hundreds of voxels of which nearly all are nothing, and asking about each one spends the
    /// walk on the space *between* matter.
    ///
    /// This walk descends the hierarchy instead. A node's box is tested against the ray **once**,
    /// and:
    ///
    /// * [`Node::Empty`] is skipped whole — the cost of an empty metre is the cost of an empty
    ///   chunk;
    /// * [`Node::Uniform`] is yielded as **one span** however many voxels it covers — a block of
    ///   one material is one question, and the merge that built it is what makes that true;
    /// * [`Node::Mixed`] is a leaf voxel, so it is one span by definition;
    /// * [`Node::Branch`] is descended, nearest child first, and only the children the ray's slab
    ///   actually crosses are visited at all.
    ///
    /// The spans are **in order and never overlap**: each is the ray's own intersection with the
    /// box of a non-empty node. **The gap between two spans is empty space** — the walk yields no
    /// span for it, which is the whole of the saving — and `exit_t` and the next `entry_t` are the
    /// two ends of that gap rather than a beat the walk missed.
    ///
    /// **A `Mixed` leaf that is not uniform is reported cell by cell**, at [`SUBVOXEL_SIDE`] per
    /// voxel: the stretch is split at the grid the matter is stored in, so `material_at_entry` is
    /// never one of sixty-four cells speaking for the metre. See [`Spans::split`]. A caller accumulating along the
    /// path (a transmittance, a distance to first solid) therefore charges the spans and lets the
    /// gaps cost nothing, instead of reconstructing the geometry from them.
    ///
    /// `None` for a degenerate ray — a NaN coordinate, a zero direction, a reach that is not a
    /// reach — by the same rule as [`Crossings::new`]: an unmeasurable ray is not a ray that
    /// crosses nothing.
    pub fn spans(&self, ray: &Ray, max_distance: f64) -> Option<Spans<'_>> {
        Spans::new(self.root(), self.levels(), ray, max_distance)
    }

    /// The first voxel along `ray`, found by **descending the hierarchy**.
    ///
    /// The same hit [`Self::raycast_local`] reports — the same voxel, entry face, subvoxel and
    /// distance, so a caller may swap one for the other — at the cost of the tree rather than the
    /// grid. `the_two_walks_find_the_same_voxel` holds the two against each other over a battery
    /// of rays; `the_hierarchical_walk_touches_far_less_of_the_grid` measures what the descent
    /// buys.
    pub fn raycast_accelerated(&self, ray: &Ray, max_distance: f64) -> Option<Hit> {
        for span in self.spans(ray, max_distance)? {
            if !span.occupied() {
                continue;
            }
            let local = local_from(span.voxel).ok()?;
            return Some(Hit {
                local,
                entry_face: span.entry_face,
                subvoxel: span.at,
                distance: span.entry_t,
            });
        }
        None
    }
}

/// **One stretch of a ray's path under one node** — a voxel at level 0, and `2^level` voxels per
/// edge above it.
///
/// The span carries the node it came from rather than a material, because a leaf's answer is a
/// shape and not one material: [`Span::material_at_entry`] is the single question a ray asks of
/// it, and a caller with a second question (a subvoxel's own blocking) asks the node.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Span<'a> {
    /// The node the path is under: `Empty` never appears (it yields no span).
    pub node: &'a Node,
    /// The **first voxel** of the span: the one the ray entered at [`Span::entry_t`].
    pub voxel: [i32; 3],
    /// The face that voxel was entered through, or `None` when the ray began inside the span.
    pub entry_face: Option<Face>,
    /// Where the span begins, in units of `dir`'s length.
    pub entry_t: f64,
    /// Where it ends: the distance at which the ray leaves the span's own node, so consecutive
    /// spans meet exactly and `exit_t - entry_t` is the stretch of path this node owns.
    pub exit_t: f64,
    /// The node's level: `0` is one voxel, `L` covers `2^L` per edge.
    pub level: u32,
    /// The subvoxel of [`Span::voxel`] the ray entered through — the 4×4×4 address a dig or a
    /// material question needs.
    pub at: SubvoxelIndex,
}

impl Span<'_> {
    /// Whether there is any matter under this node at all.
    ///
    /// The node's own emptiness, not the entry cell's: a `Mixed` voxel with holes is still a
    /// voxel with a surface, and a ray that enters one has met matter. That is the rule
    /// [`SparseVoxelOctree::raycast_local`] applies to a whole voxel, stated here once.
    pub fn occupied(&self) -> bool {
        match self.node {
            Node::Empty => false,
            Node::Uniform(material) => !material.is_empty(),
            Node::Mixed(shape) => !shape.is_empty(),
            // A branch is never yielded: the walk descends it.
            Node::Branch(_) => false,
        }
    }

    /// What the span holds at the point the ray entered it: a `Uniform` node's material, a
    /// `Mixed` leaf voxel's own cell there, or `None` for matter that is absent.
    ///
    /// **A `Uniform` node answers for every voxel under it**, which is not a shortcut: the node
    /// exists only because all `2^(3*level)` voxels beneath it hold that one material, which is
    /// what the tree's own merge rule guarantees.
    pub fn material_at_entry(&self) -> Option<MaterialId> {
        match self.node {
            Node::Empty => None,
            Node::Uniform(material) => (!material.is_empty()).then_some(*material),
            Node::Mixed(shape) => {
                let material = shape.get(self.at);
                (!material.is_empty()).then_some(material)
            }
            Node::Branch(_) => None,
        }
    }
}

/// One node still to visit: its box, and where the ray enters it.
#[derive(Clone, Copy)]
struct Pending<'a> {
    node: &'a Node,
    /// The box's corner, in local voxel coordinates.
    min: [i32; 3],
    /// The level of the box: it covers `2^level` voxels per edge from `min`.
    level: u32,
    near: f64,
    far: f64,
    entry_face: Option<Face>,
}

/// One cell of a leaf being subdivided: the stretch of the ray inside one subvoxel, and the leaf
/// it belongs to.
#[derive(Clone, Copy)]
struct Cell<'a> {
    node: &'a Node,
    voxel: [i32; 3],
    at: SubvoxelIndex,
    near: f64,
    far: f64,
    entry_face: Option<Face>,
}

/// The walk [`SparseVoxelOctree::spans`] returns. See it for what a span is.
pub struct Spans<'a> {
    ray: Ray,
    stack: Vec<Pending<'a>>,
    /// The cells of the leaf currently being subdivided, in reverse so that a pop yields the
    /// nearest. Only ever holds one leaf's cells — the stack is not touched until they are spent.
    cells: Vec<Cell<'a>>,
}

impl<'a> Spans<'a> {
    fn new(root: &'a Node, levels: u32, ray: &Ray, max_distance: f64) -> Option<Self> {
        let ray = *ray;
        for i in 0..3 {
            if !ray.origin[i].is_finite() || !ray.dir[i].is_finite() {
                return None;
            }
        }
        if ray.dir.iter().all(|d| *d == 0.0) || !max_distance.is_finite() || max_distance <= 0.0 {
            return None;
        }
        let min = [0i32; 3];
        let stack = match slab(&ray, min, levels, max_distance) {
            Some((near, far, entry_face)) => {
                vec![Pending { node: root, min, level: levels, near, far, entry_face }]
            }
            None => Vec::new(),
        };
        Some(Spans { ray, stack, cells: Vec::new() })
    }

    /// **One `Mixed` leaf's stretch, as the cells the ray actually crosses.**
    ///
    /// A leaf's answer is sixty-four subvoxels and [`Span::material_at_entry`] is one of them —
    /// the one the ray entered through. Reported as a single metre-wide span, that cell speaks for
    /// the rest, and both directions of the error are the same mistake: a beam through a dug
    /// corner passes the rock beside it, and a beam through a solid corner is charged for rock it
    /// never met. *What does this stretch of the path hold* is this walk's question, so it is
    /// answered here rather than by each consumer re-walking the grid for itself.
    ///
    /// **A uniform leaf is not split.** All sixty-four cells hold the same material, so the entry
    /// cell's answer *is* the leaf's for the whole metre and a split would be arithmetic spent
    /// arriving where the caller already was; a dig or a mix is the only thing that makes a leaf
    /// non-uniform, and this is the check for it. What stays coarse at the split is a leaf of two
    /// materials — the entry cell answers for the metre — which is the same answer the whole-metre
    /// span gave and is exact while `Matter::blocking` is per-material and no record declares a
    /// per-material extinction.
    fn split(
        &mut self,
        node: &'a Node,
        voxel: [i32; 3],
        entry_face: Option<Face>,
        near: f64,
        far: f64,
    ) {
        let side = SUBVOXEL_SIDE_I32;
        let cell_m = 1.0 / side as f64;
        let entry = point_at(self.ray.origin, self.ray.dir, near);
        let mut cell = [0i32; 3];
        for axis in 0..3 {
            let local = ((entry[axis] - voxel[axis] as f64) * side as f64).floor();
            cell[axis] = (local as i32).clamp(0, side - 1);
        }
        // A line through a 4×4×4 grid crosses at most three planes per axis, so at most ten
        // cells; sixteen is the bound the array is written against and `far` ends the walk anyway.
        let mut runs = [([0i32; 3], 0.0f64, 0.0f64, None); 16];
        let mut count = 0usize;
        let mut t = near;
        let mut face = entry_face;
        while count < runs.len() {
            let mut end = far;
            let mut crossed: Option<usize> = None;
            for axis in 0..3 {
                let d = self.ray.dir[axis];
                if d == 0.0 {
                    continue;
                }
                let ahead = if d > 0.0 { (cell[axis] + 1) as f64 } else { cell[axis] as f64 };
                let plane = voxel[axis] as f64 + ahead * cell_m;
                let candidate = (plane - self.ray.origin[axis]) / d;
                if candidate < end {
                    end = candidate;
                    crossed = Some(axis);
                }
            }
            // A plane crossed at exactly the entry is a zero-length run, not a cell.
            if end > t + 1e-12 {
                runs[count] = (cell, t, end, face);
                count += 1;
            }
            if end >= far {
                break;
            }
            let Some(axis) = crossed else { break };
            t = end;
            cell[axis] += if self.ray.dir[axis] > 0.0 { 1 } else { -1 };
            if !(0..side).contains(&cell[axis]) {
                break;
            }
            // The face this cell is entered through, by the same rule `Crossings` writes.
            face = Some(match (axis, self.ray.dir[axis] > 0.0) {
                (0, true) => Face::NegX,
                (0, false) => Face::PosX,
                (1, true) => Face::NegY,
                (1, false) => Face::PosY,
                (2, true) => Face::NegZ,
                _ => Face::PosZ,
            });
        }
        for (cell, near, far, entry_face) in runs[..count].iter().rev() {
            // The clamp above is the proof these are in `0..4`, exactly as `subvoxel_of`'s is.
            self.cells.push(Cell {
                node,
                voxel,
                at: SubvoxelIndex {
                    x: cell[0].clamp(0, side - 1) as u8,
                    y: cell[1].clamp(0, side - 1) as u8,
                    z: cell[2].clamp(0, side - 1) as u8,
                },
                near: *near,
                far: *far,
                entry_face: *entry_face,
            });
        }
    }
}

impl<'a> Iterator for Spans<'a> {
    type Item = Span<'a>;

    fn next(&mut self) -> Option<Span<'a>> {
        loop {
            // A leaf's own cells come first: they were pushed last and belong to the leaf the
            // stack has already yielded.
            if let Some(cell) = self.cells.pop() {
                return Some(Span {
                    node: cell.node,
                    voxel: cell.voxel,
                    entry_face: cell.entry_face,
                    entry_t: cell.near,
                    exit_t: cell.far,
                    level: 0,
                    at: cell.at,
                });
            }
            let Some(pending) = self.stack.pop() else {
                return None;
            };
            match pending.node {
                Node::Empty => continue,
                Node::Mixed(shape) => {
                    let entry_t = pending.near.max(0.0);
                    let point = point_at(self.ray.origin, self.ray.dir, entry_t);
                    let voxel = first_voxel(point, pending.min, pending.level);
                    let first = shape.cells()[0];
                    let uniform = shape.cells().iter().all(|material| *material == first);
                    if !uniform && pending.level == 0 {
                        self.split(pending.node, voxel, pending.entry_face, entry_t, pending.far);
                        continue;
                    }
                    return Some(Span {
                        node: pending.node,
                        voxel,
                        entry_face: pending.entry_face,
                        entry_t,
                        exit_t: pending.far,
                        level: pending.level,
                        at: subvoxel_of(point, voxel),
                    });
                }
                Node::Uniform(_) => {
                    let entry_t = pending.near.max(0.0);
                    let point = point_at(self.ray.origin, self.ray.dir, entry_t);
                    let voxel = first_voxel(point, pending.min, pending.level);
                    return Some(Span {
                        node: pending.node,
                        voxel,
                        entry_face: pending.entry_face,
                        entry_t,
                        exit_t: pending.far,
                        level: pending.level,
                        at: subvoxel_of(point, voxel),
                    });
                }
                Node::Branch(children) => {
                    if pending.level == 0 {
                        continue;
                    }
                    let level = pending.level - 1;
                    // The crossed children, nearest first: one node's octants are disjoint boxes,
                    // so their intervals cannot interleave and the first is the first matter.
                    let mut crossed = [None; 8];
                    let mut count = 0usize;
                    for (index, child) in children.iter().enumerate() {
                        if matches!(child, Node::Empty) {
                            continue;
                        }
                        let min = child_min(pending.min, level, index);
                        if let Some((near, far, entry_face)) =
                            slab(&self.ray, min, level, pending.far)
                        {
                            crossed[count] =
                                Some((near, Pending { node: child, min, level, near, far, entry_face }));
                            count += 1;
                        }
                    }
                    for i in 1..count {
                        let mut j = i;
                        while j > 0 {
                            let (left, right) = (
                                crossed[j - 1].expect("filled before the sort").0,
                                crossed[j].expect("filled before the sort").0,
                            );
                            if left <= right {
                                break;
                            }
                            crossed.swap(j - 1, j);
                            j -= 1;
                        }
                    }
                    for slot in crossed[..count].iter().rev() {
                        if let Some((_, child)) = slot {
                            self.stack.push(*child);
                        }
                    }
                }
            }
        }
    }
}

/// The box of a child octant: `min` shifted by the octant's own bit per axis.
///
/// x is bit 0, y bit 1, z bit 2 — the order `octree::octant_index` writes, because a second
/// spelling of it here is a second chance to get the octants wrong.
fn child_min(min: [i32; 3], level: u32, index: usize) -> [i32; 3] {
    let half = 1i32 << level;
    let mut out = min;
    for axis in 0..3 {
        if index & (1 << axis) != 0 {
            out[axis] += half;
        }
    }
    out
}

/// The slab test: where the ray `(origin, dir)` is inside the box `min .. min + 2^level`.
///
/// `Some((near, far, entry_face))` when it is inside for some stretch within `max_distance`,
/// with `entry_face` naming the box's face the ray crossed when it came from outside — and
/// `None` when the ray began inside, because at zero distance there is no entry face.
///
/// A **zero direction** on an axis is the parallel case and is handled as one: the ray is either
/// inside that slab for its whole length or outside it forever, and there is no division to do.
/// This is [`Crossings`]' own rule for a zero component, arrived at directly.
///
/// **Public because one question is asked from outside this crate**: a light marching *across*
/// chunks has to know where the ray leaves each chunk's own box, and `far` here is exactly that.
/// `voxel_client::field` is the caller, and the alternative to this function is a second
/// intersection routine in the client that must agree with this one — the defect the module
/// comment above is about.
pub fn slab(ray: &Ray, min: [i32; 3], level: u32, max_distance: f64) -> Option<(f64, f64, Option<Face>)> {
    let size = (1i32 << level) as f64;
    let mut near = 0.0f64;
    let mut far = max_distance;
    let mut axis: Option<(usize, bool)> = None;
    for i in 0..3 {
        let lo = min[i] as f64;
        let hi = lo + size;
        let d = ray.dir[i];
        let o = ray.origin[i];
        if !d.is_finite() || !o.is_finite() || !hi.is_finite() {
            return None;
        }
        if d == 0.0 {
            if o < lo || o > hi {
                return None;
            }
            continue;
        }
        let a = (lo - o) / d;
        let b = (hi - o) / d;
        let (t0, t1) = if a <= b { (a, b) } else { (b, a) };
        if t0 > 0.0 && t0 > near {
            axis = Some((i, d > 0.0));
        }
        near = near.max(t0);
        far = far.min(t1);
        if near > far {
            return None;
        }
    }
    if far <= 0.0 {
        return None;
    }
    let entry_face = axis.map(|(i, positive)| match (i, positive) {
        (0, true) => Face::NegX,
        (0, false) => Face::PosX,
        (1, true) => Face::NegY,
        (1, false) => Face::PosY,
        (_, true) => Face::NegZ,
        (_, false) => Face::PosZ,
    });
    Some((near, far, entry_face))
}

/// The first voxel of a span: the voxel of `point`, clamped to the span's own box.
///
/// The clamp is the boundary case rather than an error path: a point exactly on the box's far
/// plane floors into the next voxel, which is not in this span, and the voxel the ray is in is
/// the last one that is.
fn first_voxel(point: [f64; 3], min: [i32; 3], level: u32) -> [i32; 3] {
    let high = (1i32 << level) - 1;
    let mut out = [0i32; 3];
    for axis in 0..3 {
        let floored = point[axis].floor();
        let clamped = if !floored.is_finite() {
            min[axis] as f64
        } else {
            floored.clamp(min[axis] as f64, (min[axis] + high) as f64)
        };
        out[axis] = clamped as i32;
    }
    out
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

    // ---- the hierarchical walk: the two walks must agree, and the descent must cost less ----

    /// Deterministic rays, spread over the chunk and over the axis-aligned and diagonal cases,
    /// from a tiny linear congruential generator. A hand-written list would test the cases its
    /// author thought of; this one also crosses the tree at the angles nobody picks.
    fn a_battery(count: usize, seed: u64) -> Vec<Ray> {
        let mut state = seed | 1;
        let mut next = move || {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((state >> 33) as f64 / (1u64 << 31) as f64) * 2.0 - 1.0
        };
        (0..count)
            .map(|_| {
                Ray::new(
                    [
                        next() * 15.5 + 16.0,
                        next() * 15.5 + 16.0,
                        next() * 15.5 + 16.0,
                    ],
                    [next(), next(), next()],
                )
            })
            .collect()
    }

    /// A chunk with all three node kinds in it, and **dense enough that a battery actually
    /// crosses matter**: three quadrants filled by `Uniform` nodes of two materials, one quadrant
    /// empty, and `Mixed` leaves — scattered voxels and a voxel with a hole in it — in the gaps.
    fn a_mixed_chunk() -> SparseVoxelOctree {
        let mut tree = SparseVoxelOctree::new();
        tree.set_material_region([0, 0, 0], 4, STONE);
        tree.set_material_region([16, 0, 0], 4, MaterialId(9));
        tree.set_material_region([0, 0, 16], 4, STONE);
        for cell in [[0, 20, 0], [7, 31, 3], [16, 20, 0], [31, 16, 31], [2, 17, 2]] {
            tree.set_voxel(local_from(cell).unwrap(), VoxelShape::solid(STONE));
        }
        let mut holed = VoxelShape::solid(STONE);
        holed.set(voxel_model::SubvoxelIndex { x: 0, y: 0, z: 0 }, MaterialId::EMPTY);
        tree.set_voxel(local_from([24, 20, 24]).unwrap(), holed);
        tree
    }

    /// **A leaf that is not uniform is answered cell by cell.** A `Mixed` voxel's answer is
    /// sixty-four cells, and a span that reported one of them for the whole metre left every
    /// consumer either wrong (a beam through the dug corner passing the rock) or re-walking the
    /// grid for itself. The split is at the grid the matter is stored in, so the spans come out
    /// quarter of a metre long and in order — and a uniform leaf is still the one span it was.
    #[test]
    fn a_leaf_with_a_hole_is_reported_cell_by_cell() {
        let mut tree = SparseVoxelOctree::new();
        let voxel = local_from([4, 4, 4]).expect("a voxel");
        let mut shape = VoxelShape::solid(STONE);
        // The bottom cell dug out, so the ray enters the voxel through emptiness.
        shape.set(SubvoxelIndex { x: 0, y: 0, z: 0 }, MaterialId::EMPTY);
        tree.set_voxel(voxel, shape);

        let ray = Ray::new([4.125, 3.5, 4.125], [0.0, 1.0, 0.0]);
        let spans: Vec<Span> = tree.spans(&ray, 8.0).unwrap().collect();
        assert_eq!(spans.len(), 4, "a holed voxel was reported as {} span(s)", spans.len());
        for span in &spans {
            assert_eq!(span.voxel, [4, 4, 4], "a cell left its own voxel");
            assert_eq!(span.level, 0, "a cell claimed a level");
            assert!(
                span.exit_t - span.entry_t <= 0.25 + 1e-12,
                "a cell spanned {} m",
                span.exit_t - span.entry_t
            );
            assert!(span.entry_t >= 0.0 && span.exit_t > span.entry_t);
        }
        assert_eq!(spans[0].material_at_entry(), None, "the dug cell was not empty");
        assert!(
            spans[1..].iter().all(|s| s.material_at_entry() == Some(STONE)),
            "a cell above the dig did not hold the voxel's material"
        );
        // And the split did not move: the four cells are the four quarters of the metre.
        assert!((spans[0].entry_t - 0.5).abs() < 1e-12, "entry at {}", spans[0].entry_t);
        assert!((spans[3].exit_t - 1.5).abs() < 1e-12, "exit at {}", spans[3].exit_t);

        // **A uniform leaf is not split**: the entry cell answers for the whole metre and the
        // walk does not spend a crossing per quarter on a block nobody has dug into.
        let mut solid = SparseVoxelOctree::new();
        solid.set_voxel(voxel, VoxelShape::solid(STONE));
        let one: Vec<Span> = solid.spans(&ray, 8.0).unwrap().collect();
        assert_eq!(one.len(), 1, "a solid voxel was split into {} spans", one.len());
        assert!((one[0].exit_t - one[0].entry_t - 1.0).abs() < 1e-12);
    }

    /// **The acceleration may not answer a different question.** Every ray of the battery gives
    /// the same voxel, entry face, subvoxel and distance from both walks, and the battery must
    /// *hit* — two walks that agree about missing prove nothing.
    #[test]
    fn the_two_walks_find_the_same_voxel() {
        let tree = a_mixed_chunk();
        let mut hits = 0usize;
        for ray in a_battery(400, 0x5eed_1234) {
            let stepped = tree.raycast_local(&ray, 200.0);
            let descended = tree.raycast_accelerated(&ray, 200.0);
            assert_eq!(
                stepped.map(|h| (h.local, h.entry_face, h.subvoxel)),
                descended.map(|h| (h.local, h.entry_face, h.subvoxel)),
                "the two walks disagree about {ray:?}"
            );
            match (stepped, descended) {
                (Some(stepped), Some(descended)) => assert!(
                    (stepped.distance - descended.distance).abs() < 1e-9,
                    "the walks agree on the voxel and not on the distance: {} against {} in \
                     {ray:?}",
                    stepped.distance,
                    descended.distance
                ),
                (None, None) => {}
                _ => unreachable!("the two walks already agreed on which of them hit"),
            }
            if stepped.is_some() {
                hits += 1;
            }
        }
        assert!(hits > 150, "the battery barely hit anything: {hits} of 400");
    }

    /// **The measurement the acceleration exists for**: a ray straight through an empty chunk, from
    /// one face to the other — the grid walk visits every voxel it crosses, the tree walk only the
    /// boxes that are not empty — and then the same ray with one solid voxel at the far end, where
    /// both must find that voxel and the tree still must not walk the space between.
    #[test]
    fn the_hierarchical_walk_touches_far_less_of_the_grid() {
        let mut tree = SparseVoxelOctree::new();
        let ray = Ray::new([0.5, 0.5, 0.5], [1.0, 0.0, 0.0]);
        let stepped = Crossings::new(&ray, 32.0).unwrap().count();
        let descended = tree.spans(&ray, 32.0).unwrap().count();
        assert!(stepped >= 32, "the grid walk crossed the whole chunk: {stepped}");
        assert_eq!(descended, 0, "an empty chunk has no occupied span to report");

        tree.set_voxel(local_from([31, 0, 0]).unwrap(), VoxelShape::solid(STONE));
        let hit = tree.raycast_accelerated(&ray, 32.0).expect("the far voxel");
        assert_eq!(hit.local, local_from([31, 0, 0]).unwrap());
        let descended = tree.spans(&ray, 32.0).unwrap().count();
        assert!(
            descended <= 4,
            "the descent should cost the nodes it enters, not the voxels it passes: {descended} \
             spans against {stepped} voxels"
        );
        println!(
            "a ray across an empty chunk: {stepped} voxels stepped, {descended} spans descended"
        );
    }

    /// **A block of one material is one question.** Eight voxels per edge, one material: one node,
    /// so one span — and the span says so by its level rather than by a count of voxels. The
    /// region's origin is **aligned to its own level** (`8 = 2^3`), which is what makes it one
    /// node: an unaligned region is written through the node covering the aligned region it is in.
    #[test]
    fn a_solid_block_is_one_span_however_many_voxels_it_covers() {
        let mut tree = SparseVoxelOctree::new();
        tree.set_material_region([8, 0, 0], 3, STONE);
        let ray = Ray::new([0.5, 4.5, 4.5], [1.0, 0.0, 0.0]);
        let spans: Vec<Span> = tree.spans(&ray, 40.0).unwrap().collect();
        assert_eq!(spans.len(), 1, "a block of one material is one span");
        assert_eq!(spans[0].level, 3);
        assert_eq!(spans[0].material_at_entry(), Some(STONE));
        assert_eq!(spans[0].voxel, [8, 4, 4], "the span begins at the block's own voxel");
        assert_eq!(spans[0].entry_face, Some(Face::NegX));
        assert!((spans[0].entry_t - 7.5).abs() < 1e-12, "entry at {}", spans[0].entry_t);
        assert!((spans[0].exit_t - 15.5).abs() < 1e-12, "exit at {}", spans[0].exit_t);
        assert_eq!(
            tree.raycast_accelerated(&ray, 40.0).unwrap().local,
            local_from([8, 4, 4]).unwrap()
        );
    }

    #[test]
    fn the_spans_are_ordered_disjoint_and_their_gaps_are_empty() {
        let tree = a_mixed_chunk();
        let mut checked = 0usize;
        for ray in a_battery(120, 0xabcd_0f0f) {
            let spans: Vec<Span> = tree.spans(&ray, 200.0).unwrap().collect();
            for pair in spans.windows(2) {
                let (end, next) = (pair[0].exit_t, pair[1].entry_t);
                assert!(
                    end <= next + 1e-12,
                    "two spans overlap in {ray:?}: ends at {end}, next begins at {next}"
                );
                for crossing in Crossings::new(&ray, 200.0).expect("the battery's rays are rays") {
                    if crossing.t > end && crossing.t < next {
                        let local = local_from(crossing.voxel).expect("inside the chunk");
                        assert!(
                            tree.voxel(local).is_empty(),
                            "the walk stepped over {:?}, which is not empty, in {ray:?}",
                            crossing.voxel
                        );
                    }
                }
            }
            for span in &spans {
                assert!(span.exit_t > span.entry_t, "an empty span in {ray:?}");
                assert!(span.entry_t >= 0.0, "a span before the origin in {ray:?}");
                assert!(span.occupied(), "an empty node was yielded in {ray:?}");
            }
            checked += spans.len();
        }
        assert!(checked > 50, "the battery crossed too little to be a test: {checked}");
    }

    #[test]
    fn the_descent_is_bounded_by_the_reach_and_by_the_chunk() {
        let tree = a_mixed_chunk();
        let ray = Ray::new([0.5, 0.5, 0.5], [1.0, 0.0, 0.0]);
        let spans: Vec<Span> = tree.spans(&ray, 1e9).unwrap().collect();
        assert!(!spans.is_empty());
        for span in &spans {
            assert!(span.exit_t <= 32.0, "a span left the chunk at {}", span.exit_t);
        }
        assert_eq!(tree.spans(&Ray::new([0.5, 40.5, 0.5], [1.0, 0.0, 0.0]), 40.0).unwrap().count(), 0);
        assert!(tree.spans(&Ray::new([0.0; 3], [0.0; 3]), 10.0).is_none());
        assert!(tree.spans(&Ray::new([f64::NAN, 0.0, 0.0], [1.0, 0.0, 0.0]), 10.0).is_none());
        assert!(tree.spans(&Ray::new([0.0; 3], [1.0, 0.0, 0.0]), f64::INFINITY).is_none());
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
