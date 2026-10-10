//! **The cloud as a sparse voxel octree** — the model's own water, subdivided where it changes and
//! merged where it does not, recorded as the world's own rectangles so it is drawn by the one
//! `shading.wgsl` fragment path.
//!
//! # One octree, and the domain it is over is the atmosphere
//!
//! The cloud is **not a second lattice**. It is [`voxel_mesh`]'s own canonical
//! [`SparseVoxelOctree`] — the same nodes, the same canonicalisation, the same walk the world's
//! mesher rasterises — asked to span the **atmosphere of the cuboid globe** rather than one 32 m
//! chunk: [`CLOUD_DOMAIN_M`] is a cube 16,384 m a side, placed so it brackets the model's own
//! ceiling ([`TROPOPAUSE_M`], 11 km), and the tree is asked for [`CLOUD_LEVELS`] levels so its
//! finest cell is [`CLOUD_FINEST_CELL_M`] = 8 m. Nothing about the tree's span is baked in: the
//! level count is the tree's own field, which is what lets a 16 km cube of sky and a 32 m chunk of
//! terrain be the same type with the same merge rule.
//!
//! **The bound is the model's, not a count.** The recursion stops for a box that lies at or above
//! the tropopause because there is no condensed water above it, not because a row budget ran out.
//! A storm cumulus is therefore as tall as the model makes it, and the reason the sky is not
//! filled with cells is the field's own water.
//!
//! # Detail is where the water changes; the interior is one node
//!
//! A box whose own corners are all above [`CLOUD_CUT_WATER`] is written as **one region** in one
//! write; a box that is all below it is not written at all; a box that straddles the cut
//! **subdivides**, down to a single cell. That is the whole of the shape: the cloud's silhouette
//! gets finer and finer cells because that is where the model's profile crosses the cut, and the
//! inside of a cumulus is a handful of large regions because the model says one thing there. The
//! alternative — a fixed per-column grid of equal boxes — spends the same geometry on the inside
//! of a cloud as on its edge, which is why the first pictures were columns of slabs.
//!
//! # The merge is `mesh.rs`'s, over leaves instead of a voxel plane
//!
//! A leaf's face is drawn unless the region on the other side of it is cloud of the same size —
//! which for the same-size neighbour is the tree's own [`SparseVoxelOctree::region`], the operation
//! [`SparseVoxelOctree::set_material_region`] is the inverse of. Faces that survive are
//! greedy-merged (rows first, then columns) exactly as the terrain's mesher merges its own, so a
//! large face is one rectangle and the surface is the number of rectangles the surface's own shape
//! asks for.
//!
//! The merge only joins faces of the **same level** and the same **veil bucket** (64 buckets of
//! opacity), so the soft edge is not averaged into the core: a cloud's edge cells are its
//! subdivision, and the veil still travels with them.
//!
//! # The veil, and where it travels
//!
//! Each rectangle carries the **mean veil of the leaves under it** — `cloud_transmittance` of the
//! water path the model's own profile puts through the leaf — in the origin's `w`, the one float of
//! that `vec4` the world never used (C-179). `0` is terrain (nothing is attenuated) and a positive
//! value is cloud, because `cloud_transmittance` is strictly below `1` for any column holding
//! water; a column holding none is never written to the tree at all. The cells a beam crosses
//! therefore **multiply** to the column's own number rather than applying it once each.
//!
//! # The walls the eye can see
//!
//! A leaf's face is also dropped when it points away from the eye. The cloud state tests depth
//! without writing it, so a wall the eye cannot see is not free: it composites on a display and it
//! *wins* on a target that cannot blend, because it is drawn after the wall in front of it. The eye
//! is already a parameter of [`CloudCubes::build`], so the test is one comparison per axis — and a
//! player *inside* a leaf keeps both walls on that axis, so flying through a cloud still shows the
//! cell they are in.

use std::collections::{BTreeMap, BTreeSet};

use voxel_mesh::{Node, SparseVoxelOctree};
use voxel_model::{Face, MaterialId, SUBVOXEL_SIDE};
use voxel_shapers::clouds::TROPOPAUSE_M;

use crate::cloud_field::{
    cloud_transmittance, density_of, CloudField, CLOUD_COLUMNS, CLOUD_COLUMN_M,
    CLOUD_SAMPLES_PER_AXIS, CLOUD_SAMPLE_M,
};
use crate::quad::{basis_for, QuadRecord};

/// **The drawn floor**: one pixel of a block at 32 per side, in metres. It is the user's own
/// number (C-164: *"we do not go further than 32 pixels per side of block"*) and the finest rung
/// the ladder has, so no cell is ever drawn finer than it.
pub const MIN_CELL_M: f64 = 31.25 / 1000.0;

/// The ladder's top for a cloud, metres. The plan's own cap: above it the n-gon merge is
/// affordable but the cell stops being a cell.
pub const MAX_CELL_M: f64 = 32.0;

/// How many rungs the cloud's ladder has: `MIN_CELL_M · 2^octave`, so octave 10 is
/// [`MAX_CELL_M`] exactly — 31.25 mm × 1024 = 32 m.
pub const RUNGS: u32 = 10;

/// The cell size of one rung, metres — the geometric ladder [`rung_for`] climbs.
pub fn rung_cell_m(octave: u32) -> f64 {
    let octave = octave.min(RUNGS);
    MIN_CELL_M * (1u64 << octave) as f64
}

/// **The rung drawn at an eye distance**: the finest rung whose cell is more than four times the
/// pixel's own footprint — the ladder's silence rule (`pattern::octave_weight`), inverted.
///
/// The rule is the model's and not a taste: a rung is drawn while the pixel is under a quarter of
/// its cell, so a cell never shows a detail the screen cannot resolve. The floor and the cap are
/// [`MIN_CELL_M`] and [`MAX_CELL_M`], which is why the answer is clamped rather than extrapolated
/// — a cloud at 8 km would otherwise ask for a 150 m cell, and one inside the player's eye for a
/// cell below the drawn floor.
pub fn rung_for(pixel_angle_rad: f64, distance_m: f64) -> u32 {
    // **An unmeasurable input must not buy the fine end.** This function is a *bound on cost*, so
    // a NaN angle, a NaN or infinite distance, or a non-positive angle is answered with the
    // **coarsest** rung rather than the floor: the floor is the most expensive cell there is, and
    // handing it to an unmeasurable eye would be a refusal dressed as a measurement. Only a
    // genuine distance of zero — an eye inside the cloud, whose footprint really is nothing —
    // lands on the floor.
    if !pixel_angle_rad.is_finite() || !distance_m.is_finite() || pixel_angle_rad <= 0.0 || distance_m < 0.0 {
        return RUNGS;
    }
    let footprint_m = pixel_angle_rad * distance_m.max(0.0);
    let wanted = 4.0 * footprint_m;
    if wanted <= MIN_CELL_M {
        return 0;
    }
    let octave = (wanted / MIN_CELL_M).log2().ceil();
    if octave <= 0.0 {
        0
    } else {
        (octave as u32).min(RUNGS)
    }
}

/// **Where a cloud's edge is cut**, as the model's own local water at the cell.
///
/// `clouds`' doc is explicit that this is the appearance layer's decision and not the model's
/// (*"a rendered cloud's edge is where the appearance layer chooses to cut it, not a step in this
/// function"*), so it is declared here rather than inferred anywhere. It is now also the tree's
/// **subdivision rule**: a box is one region when all of its own corners are on one side of this
/// number, and it subdivides when they are not — so this one number sets where the cloud's detail
/// is spent.
pub const CLOUD_CUT_WATER: f64 = 0.05;

/// **The cloud's domain**: one cube, this many metres a side.
///
/// It is the atmosphere's own reach and not a chunk's: the widest thing a cloud can be is as tall
/// as the tropopause, so the tree has to bracket it, and the field it is filled from reaches
/// [`crate::cloud_field::CLOUD_SPAN_M`] = 8,192 m around the eye — which sits well inside this cube,
/// with the outer band left [`Node::Empty`] at no cost (one node).
pub const CLOUD_DOMAIN_M: f64 = 16_384.0;

/// **How many levels the cloud's tree has** — the domain at `2^11` cells, so its finest cell is
/// [`CLOUD_FINEST_CELL_M`]. The world's tree has 5 ([`voxel_mesh::OCTREE_LEVELS`]); the same type
/// serves both because the span is the tree's own field.
pub const CLOUD_LEVELS: u32 = 11;

/// **The finest cell the cloud's tree can draw**, metres: the domain at [`CLOUD_LEVELS`] levels.
///
/// 8 m is where the ladder meets the tree: the rung at 1 km with the game's own pixel angle
/// (1.70 mrad) is 8 m, it is a whole number of subvoxels at the record's own quantum (32), and it
/// keeps every coordinate the record format can hold, which the tests hold rather than assume.
pub const CLOUD_FINEST_CELL_M: f64 = CLOUD_DOMAIN_M / (1u64 << CLOUD_LEVELS) as f64;

/// **The tree's own material for a cloud cell.** The octree stores materials, and a cloud cell has
/// to be *something*: this id marks the cell **inside the tree** and goes no further. The rectangle
/// the cloud is drawn through carries the empty id, because what the fragment draws there is the
/// veil's business and not a material's (`cloud_field::cloud_inputs` is that appearance).
pub const CLOUD_CELL: MaterialId = MaterialId(1);

/// Opacity buckets the merge may join across. A rectangle's leaves are merged only when they agree
/// on the veil to this resolution, so the edge of a cloud is not averaged into its core — 64
/// buckets is finer than the display can resolve at eight bits and coarse enough that a face still
/// merges into one rectangle.
pub const VEIL_BUCKETS: u32 = 16;

/// **The cloud's cells, as the world's rectangles.**
///
/// `records` are the merged visible faces; `origins` are one per record, all at the domain's own
/// corner (the record's subvoxels carry the position) with that rectangle's mean veil in `w`.
/// `records[*].chunk` indexes `origins`. The reported counts are what the tests print rather than
/// assert.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CloudCubes {
    pub records: Vec<QuadRecord>,
    /// `[x, y, z, veil]` per rectangle — the domain's corner in metres, and this rectangle's own
    /// share of the beam: the rectangles a beam crosses multiply to the column's transmittance.
    pub origins: Vec<[f64; 4]>,
    /// The domain's own low corner, world metres — what the records' subvoxels are relative to.
    pub domain_corner_m: [f64; 3],
    /// The drawn lattice's finest cell, metres.
    pub cell_m: f64,
    /// How many levels the tree used: the domain at `2^levels` cells a side.
    pub subdivisions: usize,
    /// How many cloud cells (leaves) the tree holds.
    pub cells: usize,
    /// The deepest the tree subdivided, in levels below the root.
    pub rows_max: usize,
    /// Rectangles the record format could not hold. **Zero is the contract** — the domain and the
    /// finest cell are chosen so no coordinate overflows — and a non-zero value is a reported
    /// failure rather than silent wraparound.
    pub clipped: usize,
    /// **The tree itself.** The geometry's own source, kept because the surface's claims are
    /// claims about *this*: how far it subdivided, where it stayed uniform, and whether anything
    /// was written above the model's ceiling.
    pub tree: SparseVoxelOctree,
}

impl CloudCubes {
    /// **The cloud, as the octree the model's own water asks for.**
    ///
    /// The eye's own rung sets the tree's *leaf*; the atmosphere sets its *domain*. A distant cloud
    /// therefore costs fewer levels over the same sky rather than a smaller sky — the bound on cost
    /// is the resolution, never the extent.
    pub fn build(field: &CloudField, eye_m: [f64; 3], pixel_angle_rad: f64) -> Self {
        // **The rung is asked about the *nearest* cloud, not the field's own centre.** The field
        // follows the eye and is 8,192 m across, so its centre is always about 4 km away — which
        // would pin every frame at the cap and leave the ladder's own rungs unreachable in the
        // game. What the eye can resolve is decided by the nearest thing it is looking at, so that
        // is the distance the ladder is asked about; a clear field has nothing to resolve and falls
        // to the cap.
        let distance = nearest_cloudy_column_m(field, eye_m).unwrap_or(f64::INFINITY);
        let rung = rung_for(pixel_angle_rad, distance);
        let cell_m = rung_cell_m(rung).clamp(CLOUD_FINEST_CELL_M, MAX_CELL_M);
        // The domain is fixed by the atmosphere and the *levels* are what the rung changes, so
        // `domain / 2^levels == cell` is exact for every rung of the ladder (a power of two).
        let levels = (CLOUD_DOMAIN_M / cell_m).log2().round().clamp(0.0, CLOUD_LEVELS as f64) as u32;
        let corner = domain_corner_m(field);

        let mut tree = SparseVoxelOctree::with_levels(levels);
        let samples = SamplePyramid::new(field);
        fill(
            &mut tree,
            field,
            &samples,
            corner,
            cell_m,
            levels,
            Ladder { eye_m, pixel_angle_rad },
        );
        let drawn = drawn_from(&tree, field, corner, cell_m, eye_m, levels);

        CloudCubes {
            records: drawn.records,
            origins: drawn.origins,
            domain_corner_m: corner,
            cell_m,
            subdivisions: levels as usize,
            cells: drawn.cells,
            rows_max: drawn.deepest,
            clipped: drawn.clipped,
            tree,
        }
    }

    /// The rows the field's own model would be asked for, as the field's sample lattice reads it —
    /// reported so a picture can say whether the model or the drawing is quiet.
    pub fn sample_lattice() -> (u32, u32) {
        (crate::cloud_field::CLOUD_SAMPLES_PER_AXIS, crate::cloud_field::CLOUD_SAMPLE_STRIDE)
    }
}

/// **The tree's domain**: the cube's low corner in world metres.
///
/// Horizontally it is the field's own centre, so the field sits centred in the cube and the outer
/// band is empty; vertically the centre is **half the tropopause**, so the cube brackets the model's
/// ceiling and a storm has room to reach it. Every axis is snapped to the cell so the tree's own
/// lattice lands on whole subvoxels of the record format.
fn domain_corner_m(field: &CloudField) -> [f64; 3] {
    let n = CLOUD_COLUMNS;
    let first = field.column_centre_m(0, 0);
    let last = field.column_centre_m(n - 1, n - 1);
    let half = CLOUD_DOMAIN_M / 2.0;
    // **Snapped to the model's own sample rate**, not to the drawn cell: a multiple of
    // `CLOUD_SAMPLE_M` is a multiple of every cell the ladder can ask for, so the tree's boxes stay
    // aligned to the model's lattice as the rung coarsens — which is what lets [`SamplePyramid`]
    // answer "could there be cloud under this box?" as an aligned block of the model's own samples
    // rather than as a handful of corner guesses.
    let snap = |metres: f64| (metres / CLOUD_SAMPLE_M).floor() * CLOUD_SAMPLE_M;
    [
        snap((first[0] + last[0]) / 2.0 - half),
        snap(TROPOPAUSE_M / 2.0 - half),
        snap((first[1] + last[1]) / 2.0 - half),
    ]
}

/// **Fill the tree from the field**, one box at a time.
/// **The eye's own resolution, per box.**
///
/// The ladder was a single cell size for the whole field, which is the one thing a tree over a 16 km
/// cube cannot afford: the nearest cumulus asks for 8 m leaves and then *every* cloud in the sky —
/// four kilometres away — is cut to 8 m too, and the leaf count becomes the surface area of the
/// atmosphere rather than of what the eye can see. A tree can ask per box, and it is the same
/// declared rule (`rung_for`'s own silence rule) evaluated at that box's own distance: a box the eye
/// cannot resolve is **one** region, decided by its own centre.
///
/// The tree's own floor is still the rung at the nearest cloud, so a near box cannot be drawn finer
/// than the ladder asked and a far one is coarser for free.
#[derive(Clone, Copy, Debug)]
struct Ladder {
    eye_m: [f64; 3],
    pixel_angle_rad: f64,
}

impl Ladder {
    /// The coarsest cell the eye can be said to resolve at `distance` — never finer than the tree's
    /// own finest cell.
    fn floor_m(&self, distance_m: f64, cell_m: f64) -> f64 {
        let rung = rung_cell_m(rung_for(self.pixel_angle_rad, distance_m));
        rung.clamp(cell_m, MAX_CELL_M)
    }

    /// How far the eye is from a box, metres — zero when it is inside it.
    fn distance_m(lo: [f64; 3], hi: [f64; 3], eye_m: [f64; 3]) -> f64 {
        let mut sum = 0.0;
        for axis in 0..3 {
            let past = (lo[axis] - eye_m[axis]).max(eye_m[axis] - hi[axis]).max(0.0);
            sum += past * past;
        }
        sum.sqrt()
    }
}

fn fill(
    tree: &mut SparseVoxelOctree,
    field: &CloudField,
    samples: &SamplePyramid,
    corner: [f64; 3],
    cell_m: f64,
    levels: u32,
    ladder: Ladder,
) {
    classify(tree, field, samples, corner, cell_m, [0, 0, 0], levels, ladder);
}

/// **One box, and what the model's own water says about it.**
///
/// The three answers are the whole algorithm: *all of it is cloud* (one region write), *none of it
/// is* (nothing is written — the node is already [`Node::Empty`]), or *it straddles the cut*
/// (subdivide). At the finest level there is nothing left to subdivide with, so the cell's own
/// address decides — the model's profile at the point the cell is, which is the same rule the
/// drawn set has always used.
fn classify(
    tree: &mut SparseVoxelOctree,
    field: &CloudField,
    samples: &SamplePyramid,
    corner: [f64; 3],
    cell_m: f64,
    origin: [i32; 3],
    level: u32,
    ladder: Ladder,
) {
    let side_m = cell_m * (1i32 << level) as f64;
    let lo = [
        corner[0] + origin[0] as f64 * cell_m,
        corner[1] + origin[1] as f64 * cell_m,
        corner[2] + origin[2] as f64 * cell_m,
    ];
    let hi = [lo[0] + side_m, lo[1] + side_m, lo[2] + side_m];

    // **The bound is the model's own ceiling.** At and above the tropopause the model condenses
    // nothing (`clouds` clamps every column's top to it), so the whole box above it is empty and
    // the recursion stops without asking the field at all. This is the atmosphere of the cuboid
    // globe doing the bounding — not a row count.
    if lo[1] >= TROPOPAUSE_M {
        return;
    }

    // **The model's own profile at this box's own address**, which is the decision every path below
    // ends in: a cell holds cloud exactly where the field's own water is above the cut *there*.
    let centre = [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0, (lo[2] + hi[2]) / 2.0];
    let wet_here = || field.density_at_m(centre[0], centre[2], centre[1]) >= CLOUD_CUT_WATER;

    // **What the model's own samples say about this box — the whole of the subdivision rule.**
    match samples.claim(lo, hi) {
        // Every sample under the box is dry, and a value between two dry samples is dry, so nothing
        // under this box can be cloud. The recursion stops on one lookup: this is what keeps a clear
        // sky nearly free instead of 2,000 levels of subdivision.
        Claim::Empty => return,
        // Every sample under it is wet **and** the box is inside the narrowest of their layers: one
        // region write for a whole box of cloud, which is what makes a coarse lattice affordable.
        //
        // **And the box's own address has to agree.** The aggregate is taken over a *block*, and a
        // block's edge is where the samples just outside it live — so a box at the block's edge can
        // be called wet on a neighbour's water. That claim is harmless as a reason to look closer and
        // wrong as a reason to write: it filled the sky beside a cumulus with regions whose own water
        // path was zero, which is geometry that draws nothing (a veil of 1) for a full record each.
        // Where the box's own address is dry the box *straddles* after all, and the subdivision
        // below spends its detail finding out where.
        Claim::Cloud if wet_here() => {
            tree.set_material_region(origin, level, CLOUD_CELL);
            return;
        }
        Claim::Cloud | Claim::Straddles => {}
    }
    // **The eye's own rung, at this box's own distance.** A box the eye cannot resolve is decided
    // here, by its centre, as one region — which is what keeps the leaf count proportional to what
    // is on screen rather than to the surface of the whole sky. The eye inside the box is distance
    // zero, the floor is then the tree's own finest cell, and the box is subdivided to it: the
    // cloud the player is standing in is the one cloud that must be drawn at full detail.
    let side_m = hi[0] - lo[0];
    let distance_m = Ladder::distance_m(lo, hi, ladder.eye_m);
    if side_m <= ladder.floor_m(distance_m, cell_m) {
        if wet_here() {
            tree.set_material_region(origin, level, CLOUD_CELL);
        }
        return;
    }
    // Straddles the cut: the detail goes here, and only here.
    let half = 1i32 << (level - 1);
    for octant in 0..8u8 {
        let child = [
            origin[0] + if octant & 1 != 0 { half } else { 0 },
            origin[1] + if octant & 2 != 0 { half } else { 0 },
            origin[2] + if octant & 4 != 0 { half } else { 0 },
        ];
        classify(tree, field, samples, corner, cell_m, child, level - 1, ladder);
    }
}

/// **Merge two aggregated samples**: the wettest water, and the base and top of the **wet** ones
/// only.
///
/// A sample with no water has `[0, 0, 0]` for its layer — which is not a layer at all, and letting
/// its zero narrow the top of the wet sample beside it would put every box's claim at
/// `Straddles` and spend the whole tree on a cloud that was never in doubt.
fn merge_sample(held: [f32; 4], sample: [f32; 4]) -> [f32; 4] {
    let wet = |at: [f32; 4]| (at[0] as f64) >= CLOUD_CUT_WATER;
    match (wet(held), wet(sample)) {
        (true, true) => [
            held[0].max(sample[0]),
            held[1].max(sample[1]),
            held[2].min(sample[2]),
            sample[3],
        ],
        (false, true) => sample,
        (true, false) => held,
        (false, false) => [held[0].max(sample[0]), held[1], held[2], held[3]],
    }
}

/// What the model's own samples say about a box.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Claim {
    /// Every sample under the box is dry: nothing under it can be cloud.
    Empty,
    /// Every sample under it is wet and the box lies inside all of their layers: it is one region.
    Cloud,
    /// The cut runs through the box, so this is where its detail is spent.
    Straddles,
}

/// **The model's water on its own lattice, and the maximum over every doubling of it.**
///
/// # Why the field's lattice and not the box's own corners
///
/// A box that is empty has to be *known* empty, and four corner samples are not that. The root box
/// is 16 km across and its corners are outside the field entirely — a corner test prunes the whole
/// sky, which is exactly what the first version of this fill did. A 1 km box's corners can be dry
/// with a cumulus in its middle. What the field *does* hold is its water at the rate the model was
/// asked — one sample every [`CLOUD_SAMPLE_M`], each carrying its own column's water, base and top
/// — and the value between samples is a convex combination of them, so **the maximum over the
/// samples covering a box bounds every value inside it**, and a box whose covering samples are all
/// dry is dry. That is the direction that cannot lose a cloud.
///
/// # And the pyramid, because a box can be 16 km across
///
/// Above two samples a side the covering samples of a box are an **aligned block** of the lattice:
/// the tree's corner is snapped to the sample rate and every cell is a power of two of it, so
/// `levels[k]` — the maximum over each `2^k × 2^k` block — answers a box in one lookup however big
/// it is. The channels aggregate in opposite directions so the aggregate is one synthetic sample
/// standing for the whole block: water and base take the **maximum** (the wettest and the highest),
/// the top the **minimum** (the lowest). A claim made about that sample therefore holds for every
/// sample under it.
struct SamplePyramid {
    /// `levels[0]` is the samples themselves; each level above it stands for twice the side.
    levels: Vec<Vec<[f32; 4]>>,
    /// Samples a side at each level.
    sides: Vec<i32>,
    /// The field's own origin in world metres — sample `(0, 0)`, which is its first column.
    origin_m: [f64; 2],
}

impl SamplePyramid {
    /// The pyramid over `field`, from the model's own samples.
    fn new(field: &CloudField) -> Self {
        let side = CLOUD_SAMPLES_PER_AXIS as i32;
        let first = field.column_centre_m(0, 0);
        let origin_m = [first[0], first[1]];
        let mut samples = vec![[0.0f32; 4]; (side * side) as usize];
        for j in 0..side {
            for i in 0..side {
                let at = [
                    origin_m[0] + i as f64 * CLOUD_SAMPLE_M,
                    origin_m[1] + j as f64 * CLOUD_SAMPLE_M,
                ];
                samples[(j * side + i) as usize] = field.at_m(at[0], at[1]);
            }
        }
        let mut pyramid = SamplePyramid { levels: vec![samples], sides: vec![side], origin_m };
        while *pyramid.sides.last().expect("a pyramid has a base") > 1 {
            pyramid.double();
        }
        pyramid
    }

    /// Add the next level: the maximum over each `2 × 2` block of the level below.
    fn double(&mut self) {
        let prev = self.levels.last().expect("a pyramid has a base");
        let prev_side = *self.sides.last().expect("a pyramid has a base");
        let side = (prev_side + 1) / 2;
        let mut next = vec![[0.0f32; 4]; (side * side) as usize];
        for j in 0..side {
            for i in 0..side {
                let mut cell: Option<[f32; 4]> = None;
                for dj in 0..2 {
                    for di in 0..2 {
                        let (pi, pj) = (i * 2 + di, j * 2 + dj);
                        if pi >= prev_side || pj >= prev_side {
                            continue;
                        }
                        let sample = prev[(pj * prev_side + pi) as usize];
                        cell = Some(match cell {
                            None => sample,
                            Some(held) => merge_sample(held, sample),
                        });
                    }
                }
                next[(j * side + i) as usize] = cell.unwrap_or([0.0; 4]);
            }
        }
        self.levels.push(next);
        self.sides.push(side);
    }

    /// The samples covering a box, aggregated into one synthetic sample — or `None` when the box is
    /// not over the field at all.
    fn covering(&self, lo: [f64; 3], hi: [f64; 3]) -> Option<[f32; 4]> {
        let span = (hi[0] - lo[0]) / CLOUD_SAMPLE_M;
        if span >= 2.0 {
            // **An aligned block.** The box spans whole samples and begins on a sample boundary, so
            // one pyramid cell is the whole of its coverage however large the box is.
            let level = ((span.round() as i32).ilog2() as usize).min(self.levels.len() - 1);
            let side = self.sides[level];
            // **Clamped, not refused.** A box larger than the field has its own block index off the
            // lattice — the root is twice the field's span — and the honest answer for it is the
            // samples it *does* cover, which is the nearest block there is. Clamping can only
            // over-claim coverage, and an over-claimed box is subdivided rather than believed; a
            // box genuinely beyond the field is caught by the four-corner branch below, where every
            // corner is off the lattice and the answer is `None`.
            let i = (((lo[0] - self.origin_m[0]) / CLOUD_SAMPLE_M).round() as i32 >> level)
                .clamp(0, side - 1);
            let j = (((lo[2] - self.origin_m[1]) / CLOUD_SAMPLE_M).round() as i32 >> level)
                .clamp(0, side - 1);
            return Some(self.levels[level][(j * side + i) as usize]);
        }
        // **Inside one sample cell**: every point of the box is a combination of the four samples at
        // that cell's own corners, so those four are the bound.
        let base_side = self.sides[0];
        let i = ((lo[0] - self.origin_m[0]) / CLOUD_SAMPLE_M).floor() as i32;
        let j = ((lo[2] - self.origin_m[1]) / CLOUD_SAMPLE_M).floor() as i32;
        let mut cell: Option<[f32; 4]> = None;
        for dj in 0..2 {
            for di in 0..2 {
                let (si, sj) = (i + di, j + dj);
                if si < 0 || sj < 0 || si >= base_side || sj >= base_side {
                    continue;
                }
                let sample = self.levels[0][(sj * base_side + si) as usize];
                cell = Some(match cell {
                    None => sample,
                    Some(held) => merge_sample(held, sample),
                });
            }
        }
        cell
    }

    /// **What the model's own samples say about a box.** The whole of the subdivision rule.
    fn claim(&self, lo: [f64; 3], hi: [f64; 3]) -> Claim {
        let Some(sample) = self.covering(lo, hi) else {
            return Claim::Empty;
        };
        if (sample[0] as f64) < CLOUD_CUT_WATER {
            // The aggregate is the *wettest* sample under the box, so every one of them is dry — and
            // a value between two dry samples is dry.
            return Claim::Empty;
        }
        // It is one region of cloud when the box lies inside the narrowest covering layer and the
        // wettest covering sample's own profile clears the cut at **both** of the box's ends: the
        // profile is a single hump, so its minimum over the box is at an end.
        let (base, top) = (sample[1] as f64, sample[2] as f64);
        let inside = lo[1] >= base && hi[1] <= top;
        let ends = density_of(sample, lo[1]) >= CLOUD_CUT_WATER
            && density_of(sample, hi[1]) >= CLOUD_CUT_WATER;
        if inside && ends {
            Claim::Cloud
        } else {
            Claim::Straddles
        }
    }
}

/// The cloud the tree holds, as records.
fn drawn_from(
    tree: &SparseVoxelOctree,
    field: &CloudField,
    corner: [f64; 3],
    cell_m: f64,
    eye_m: [f64; 3],
    levels: u32,
) -> Drawn {
    // **The visible faces, grouped by everything the merge must agree on**: the face, the level the
    // leaf is drawn at, the plane it lies on, and the veil to a bucket. Grouping by level is what
    // makes the merge sound — two leaves of different sizes are not the same rectangle however
    // close they are — and grouping by the veil's bucket is what keeps a cloud's edge from being
    // averaged into its core.
    let mut groups: BTreeMap<(u32, u32, i32, u32), BTreeMap<[i32; 2], (f64, u32)>> = BTreeMap::new();
    let mut cells = 0usize;
    let mut deepest = 0usize;

    tree.visit_leaves(&mut |origin, level, node| {
        if !is_cloud(node) {
            return;
        }
        cells += 1;
        deepest = deepest.max((levels - level) as usize);
        let half = 1i32 << level;
        let side_m = cell_m * half as f64;
        let lo = [
            corner[0] + origin[0] as f64 * cell_m,
            corner[1] + origin[1] as f64 * cell_m,
            corner[2] + origin[2] as f64 * cell_m,
        ];
        let hi = [lo[0] + side_m, lo[1] + side_m, lo[2] + side_m];
        let veil = cell_veil(field, lo, hi);
        let bucket = ((1.0 - veil) * VEIL_BUCKETS as f64).round().clamp(0.0, VEIL_BUCKETS as f64) as u32;

        for face in 0..6u32 {
            let (axis, positive) = face_axis_and_sign(face);
            let axis = axis as usize;
            // **A face is drawn unless the region behind it is cloud.** The neighbour is the same
            // size as this leaf, one leaf along the axis: a leaf answer means the region is
            // uniformly cloud (hidden) or uniformly empty (drawn), and a branch answer means it is
            // subdivided, so part of this face is exposed and it stays.
            let mut neighbour = origin;
            neighbour[axis] = if positive { origin[axis] + half } else { origin[axis] - half };
            // **Outside the tree is empty, and that is a different question from "what region is
            // there?"** A coordinate off the tree's edge would otherwise descend into the root by
            // the bit test — the same node again, cloud — and hide the face that is genuinely on
            // the outside of the domain.
            let edge = 1i32 << levels;
            let outside = neighbour.iter().any(|at| *at < 0 || *at + half > edge);
            if !outside && is_cloud(tree.region(neighbour, level)) {
                continue;
            }
            // **A wall pointing away from the eye is not drawn** (see the module doc).
            let away = if positive { eye_m[axis] < lo[axis] } else { eye_m[axis] > hi[axis] };
            if away {
                continue;
            }
            let (u_axis, v_axis) = face_axes(face);
            let u = origin[u_axis] / half;
            let v = origin[v_axis] / half;
            let plane = if positive { origin[axis] + half } else { origin[axis] };
            let entry = groups.entry((face, level, plane, bucket)).or_default();
            let cell = entry.entry([u, v]).or_insert((0.0, 0));
            cell.0 += veil;
            cell.1 += 1;
        }
    });

    let mut drawn = Drawn { cells, deepest, ..Drawn::default() };
    for ((face, level, plane, _bucket), grid) in groups {
        let side_voxels = 1i32 << level;
        for (u0, v0, du, dv, veil) in greedy_merge(&grid) {
            // **Metres to the record's own subvoxels.** The tree's coordinates count cells of
            // `side_voxels` voxels; a voxel is `cell_m` metres; and the record format divides by
            // `SUBVOXEL_SIDE` to get metres back, so this is the one conversion between the tree's
            // lattice and the format's — and it is exact, because both are whole numbers for every
            // rung of the ladder.
            let sub = |voxels: i32| voxels as f64 * cell_m * SUBVOXEL_SIDE as f64;
            let u_sub = sub(u0 * side_voxels);
            let v_sub = sub(v0 * side_voxels);
            let du_sub = sub(du * side_voxels);
            let dv_sub = sub(dv * side_voxels);
            let plane_sub = sub(plane);
            let holds = |value: f64| value.is_finite() && (0.0..=u16::MAX as f64).contains(&value);
            if !(holds(u_sub) && holds(v_sub) && holds(du_sub) && holds(dv_sub) && holds(plane_sub)) {
                drawn.clipped += 1;
                continue;
            }
            let index = drawn.origins.len() as u32;
            drawn.origins.push([corner[0], corner[1], corner[2], veil]);
            drawn.records.push(QuadRecord {
                face,
                material: MaterialId::EMPTY.0 as u32,
                slice: plane_sub as u32,
                uv: (u_sub as u32) | ((v_sub as u32) << 16),
                extent: (du_sub as u32) | ((dv_sub as u32) << 16),
                chunk: index,
            });
        }
    }
    drawn
}

/// What one build drew, before it is packaged as [`CloudCubes`].
#[derive(Clone, Debug, Default)]
struct Drawn {
    records: Vec<QuadRecord>,
    origins: Vec<[f64; 4]>,
    cells: usize,
    deepest: usize,
    clipped: usize,
}

/// True when a node is a cloud cell — a uniform region that is not empty.
///
/// A branch is never cloud *as a region*: the tree collapses eight identical children on every
/// write, so a subdivided region whose cells are all cloud would have become one `Uniform` node.
/// That is what makes this the exact answer to "is the region behind this face cloud?".
fn is_cloud(node: &Node) -> bool {
    matches!(node, Node::Uniform(material) if !material.is_empty())
}

/// The veil one leaf carries: `cloud_transmittance` of the water path the model's own profile puts
/// through it, asked at the leaf's own column — the same law the light sampler walks.
fn cell_veil(field: &CloudField, lo: [f64; 3], hi: [f64; 3]) -> f64 {
    let sample = field.at_m((lo[0] + hi[0]) / 2.0, (lo[2] + hi[2]) / 2.0);
    cloud_transmittance(CloudField::cell_water_path_m(sample, lo[1], hi[1]))
}

/// **The greedy merge**, over a set of coplanar cells: rows first, then columns, growing a
/// rectangle while every cell under it is present.
///
/// The same rule `voxel_mesh::mesh` merges the terrain's surface with, over the tree's leaves
/// instead of a voxel plane. The output carries the **mean veil** of the cells it joined, so a
/// merged rectangle stands for its own leaves' opacity rather than for one of them.
fn greedy_merge(grid: &BTreeMap<[i32; 2], (f64, u32)>) -> Vec<(i32, i32, i32, i32, f64)> {
    let mut used: BTreeSet<[i32; 2]> = BTreeSet::new();
    let mut out = Vec::new();
    for (&[u, v], _) in grid.iter() {
        if used.contains(&[u, v]) {
            continue;
        }
        let mut du = 1;
        while grid.contains_key(&[u + du, v]) && !used.contains(&[u + du, v]) {
            du += 1;
        }
        let mut dv = 1;
        'grow: loop {
            for k in 0..du {
                let at = [u + k, v + dv];
                if !grid.contains_key(&at) || used.contains(&at) {
                    break 'grow;
                }
            }
            dv += 1;
        }
        let mut sum = 0.0;
        let mut count = 0u32;
        for vv in v..v + dv {
            for uu in u..u + du {
                let (veil, leaves) = grid[&[uu, vv]];
                sum += veil;
                count += leaves;
                used.insert([uu, vv]);
            }
        }
        out.push((u, v, du, dv, sum / count as f64));
    }
    out
}

/// `Face::ALL`'s own order — 0 NegX, 1 PosX, 2 NegY, 3 PosY, 4 NegZ, 5 PosZ — as the axis it
/// lies on and whether it is the positive one. `vs_quad`'s `face_axis` is the same table.
fn face_axis_and_sign(face: u32) -> (u32, bool) {
    match face {
        0 => (0, false),
        1 => (0, true),
        2 => (1, false),
        3 => (1, true),
        4 => (2, false),
        _ => (2, true),
    }
}

/// The two world axes a face's own rectangle is spanned by, in the order the record stores them —
/// `vs_quad`'s own table read the other way round, so a rectangle's `(u, v)` here and the basis the
/// shader reconstructs are the same two axes by construction.
fn face_axes(face: u32) -> (usize, usize) {
    let (axis, _) = face_axis_and_sign(face);
    match axis {
        0 => (1, 2),
        1 => (0, 2),
        _ => (0, 1),
    }
}

/// **Which face of `Face::ALL` this record's normal is**, read back from the record so a test can
/// hold the encoding to the enum rather than to a comment.
pub fn face_of(record: &QuadRecord) -> Face {
    Face::ALL[record.face as usize]
}

/// The normal a cell face presents, in world axes. `basis_for` is the world's own table, so this
/// is a read rather than a second statement of the six directions.
pub fn cell_normal(record: &QuadRecord) -> [f64; 3] {
    basis_for(face_of(record))
}

/// **The nearest cloudy column's distance from the eye**, metres, or `None` for a clear field.
///
/// Measured to the column's own **box** and not to its centre, because the distance is a bound on
/// what the eye resolves and a 64 m column's near corner is up to 45 m closer than its middle. The
/// eye is clamped into the box on each axis first, which is the closest point of the box and is a
/// one-line answer that needs no case analysis: inside it, every axis contributes nothing.
fn nearest_cloudy_column_m(field: &CloudField, eye_m: [f64; 3]) -> Option<f64> {
    let n = CLOUD_COLUMNS;
    let mut best: Option<f64> = None;
    for j in 0..n {
        for i in 0..n {
            let Some(texel) = field.texel(i, j) else { continue };
            if texel[0] <= 0.0 {
                continue;
            }
            let [x, z] = field.column_centre_m(i, j);
            // The distance to the column's own box, one axis at a time: how far the eye is past
            // the box's edge, and nothing when it is already inside on that axis.
            let half = CLOUD_COLUMN_M / 2.0;
            let past = |centre: f64, eye: f64| ((eye - centre).abs() - half).max(0.0);
            let span = |lo: f64, hi: f64, eye: f64| (lo - eye).max(eye - hi).max(0.0);
            let dx = past(x, eye_m[0]);
            let dz = past(z, eye_m[2]);
            let dy = span(texel[1] as f64, texel[2] as f64, eye_m[1]);
            let distance = (dx * dx + dy * dy + dz * dz).sqrt();
            best = Some(best.map_or(distance, |b: f64| b.min(distance)));
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A field with cloudy columns at the given `(i, j)` addresses, on their own layers.
    fn field_with(cells: &[(u32, u32, [f32; 4])]) -> CloudField {
        let n = CLOUD_COLUMNS as usize;
        let mut columns = vec![[0.0f32; 4]; n * n];
        let mut cloudy = 0usize;
        let mut base = f64::INFINITY;
        let mut top = f64::NEG_INFINITY;
        for (i, j, texel) in cells {
            columns[*j as usize * n + *i as usize] = *texel;
            cloudy += 1;
            base = base.min(texel[1] as f64);
            top = top.max(texel[2] as f64);
        }
        CloudField {
            origin_m: [0.0, 0.0],
            columns,
            slab_base_m: if cloudy == 0 { 0.0 } else { base },
            slab_top_m: if cloudy == 0 { 0.0 } else { top },
            cloudy,
            filled_micros: 0,
        }
    }

    #[test]
    fn the_ladder_coarsens_with_distance_and_is_floored_and_capped() {
        // **The plan's own table, as arithmetic.** One pixel is 1.70 mrad at 720p with a 70°
        // vertical field of view (`pattern::pixel_angle_rad`), so the rung drawn at 500 m is the
        // finest cell over 4 × 0.85 m = 3.4 m — 4 m — and at 4,000 m it is 32 m.
        let angle = 1.70e-3;
        assert_eq!(rung_cell_m(rung_for(angle, 500.0)), 4.0);
        assert_eq!(rung_cell_m(rung_for(angle, 1_000.0)), 8.0);
        assert_eq!(rung_cell_m(rung_for(angle, 2_000.0)), 16.0);
        assert_eq!(rung_cell_m(rung_for(angle, 4_000.0)), 32.0);
        // The floor: an eye inside a cloud resolves nothing finer than the drawn floor.
        assert_eq!(rung_cell_m(rung_for(angle, 0.0)), MIN_CELL_M);
        // The cap: a cloud at 40 km would ask for 256 m, and the ladder stops at 32.
        assert_eq!(rung_cell_m(rung_for(angle, 40_000.0)), MAX_CELL_M);
        // Monotone: moving away never draws a finer cell.
        let mut previous = 0.0;
        for step in 1..200 {
            let cell = rung_cell_m(rung_for(angle, step as f64 * 50.0));
            assert!(cell >= previous, "the ladder went finer with distance at {step}");
            previous = cell;
        }
    }

    #[test]
    fn a_clear_field_draws_nothing() {
        let field = field_with(&[]);
        let cubes = CloudCubes::build(&field, [0.0, 20.0, 0.0], 1.7e-3);
        assert!(cubes.records.is_empty(), "a clear field emitted geometry");
        assert!(cubes.origins.is_empty());
        assert_eq!(cubes.cells, 0);
        assert_eq!(
            cubes.tree.node_count(),
            1,
            "a clear sky is not the tree it should be: one node"
        );
    }

    #[test]
    fn the_tree_spans_the_atmosphere_and_subdivides_only_where_the_water_crosses_the_cut() {
        // **The domain is the atmosphere's, the detail is the water's.** One cumulus over a 64 m
        // column in a 16 km cube: the tree must bracket the model's own ceiling, its cloud cells
        // must sit inside the model's layer, and the *inside* of the cumulus must be a handful of
        // large regions rather than one node per cell — which is the whole reason the geometry is
        // an octree and not a grid of boxes.
        let texel = [0.4f32, 1_000.0, 1_800.0, 1.0];
        let field = field_with(&[(10, 10, texel), (11, 10, texel), (10, 11, texel), (11, 11, texel)]);
        let cubes = CloudCubes::build(&field, [0.0, 20.0, 0.0], 1.7e-3);
        assert!(
            cubes.subdivisions <= CLOUD_LEVELS as usize,
            "the tree is deeper than the domain's own span: {}",
            cubes.subdivisions
        );
        assert!(cubes.cells > 0, "the fixture drew no cloud");

        // The domain brackets the tropopause, and it is the cube the constant says it is.
        let corner = cubes.domain_corner_m;
        assert!(
            corner[1] < 0.0 && corner[1] + CLOUD_DOMAIN_M > TROPOPAUSE_M,
            "the domain does not bracket the model's own ceiling: {corner:?}"
        );
        assert_eq!(
            CLOUD_DOMAIN_M / (1u64 << cubes.subdivisions) as f64,
            cubes.cell_m,
            "the domain and the level count disagree about the cell size"
        );

        // Every cloud leaf is inside the model's own layer and below its ceiling, and the interior
        // is compressed: the cells are far fewer than the finest lattice the box could hold.
        let mut biggest = 0i32;
        cubes.tree.visit_leaves(&mut |origin, level, node| {
            if !is_cloud(node) {
                return;
            }
            let side = cubes.cell_m * (1i32 << level) as f64;
            let lo = [
                corner[0] + origin[0] as f64 * cubes.cell_m,
                corner[1] + origin[1] as f64 * cubes.cell_m,
                corner[2] + origin[2] as f64 * cubes.cell_m,
            ];
            assert!(
                lo[1] < TROPOPAUSE_M,
                "a cloud cell was written above the model's own ceiling: {lo:?}"
            );
            assert!(
                lo[1] + side > texel[1] as f64 && lo[1] < texel[2] as f64,
                "a cloud cell lies outside the model's own layer: {lo:?} + {side}"
            );
            biggest = biggest.max(1 << level);
        });
        assert!(
            biggest > 1,
            "every cell is one voxel: the tree did not compress the cumulus's interior at all"
        );
    }

    #[test]
    fn the_geometry_is_the_surfaces_rectangles_and_not_the_cells_faces() {
        // **The merge is what makes this affordable.** A cumulus of thousands of cells has at most
        // six faces each; the drawn geometry must be far fewer rectangles than that, because the
        // tree's interior faces are hidden and the surface's coplanar ones are merged. Measured
        // rather than asserted as a magic number: the count is printed.
        let texel = [0.5f32, 1_000.0, 1_800.0, 1.0];
        let columns: Vec<(u32, u32, [f32; 4])> =
            (8..12).flat_map(|i| (8..12).map(move |j| (i, j, texel))).collect();
        let field = field_with(&columns);
        let cubes = CloudCubes::build(&field, [0.0, 20.0, 0.0], 1.7e-3);
        println!(
            "cloud surface: {} cells -> {} rectangles ({} per cell), {} origins",
            cubes.cells,
            cubes.records.len(),
            cubes.records.len() as f64 / cubes.cells.max(1) as f64,
            cubes.origins.len()
        );
        let faces = cubes.cells * 6;
        assert!(
            cubes.records.len() < faces,
            "the merge drew {} rectangles for {faces} possible faces: nothing was merged",
            cubes.records.len()
        );
        assert!(cubes.records.len() > 0, "the fixture drew nothing");

        // **No two rectangles on one face and plane overlap**, and every rectangle has area. Both
        // are claims about the merge rather than about the field, and the overlap test is done on
        // the rectangles' own intervals so a big face costs nothing to check.
        let mut planes: BTreeMap<(u32, u32), Vec<[u32; 4]>> = BTreeMap::new();
        for record in &cubes.records {
            let low = record.low();
            let size = record.size();
            assert!(size[0] > 0 && size[1] > 0, "a rectangle has no area: {record:?}");
            let rect = [
                low[0] as u32,
                low[1] as u32,
                low[0] as u32 + size[0] as u32,
                low[1] as u32 + size[1] as u32,
            ];
            let at = planes.entry((record.face, record.slice)).or_default();
            for other in at.iter() {
                let overlap = rect[0] < other[2]
                    && other[0] < rect[2]
                    && rect[1] < other[3]
                    && other[1] < rect[3];
                assert!(
                    !overlap,
                    "two rectangles overlap on face {} plane {}: {rect:?} against {other:?}",
                    record.face, record.slice
                );
            }
            at.push(rect);
        }
        assert_eq!(cubes.clipped, 0, "a rectangle left the record format's own range");
    }

    #[test]
    fn a_wetter_column_is_drawn_as_a_taller_fatter_cloud() {
        // **The emergent shape, from the model's own numbers.** The cut is on
        // `water · sin(π · through)`, so a column holding more water clears it over a wider band of
        // its own layer: the wisp is a lens and the cumulus is a mass, from the same two numbers
        // and no shape this module invented. Measured as cells and as the drawn band's height.
        let span = |water: f32| {
            let field = field_with(&[(10, 10, [water, 1_000.0, 1_800.0, 1.0])]);
            let cubes = CloudCubes::build(&field, [0.0, 20.0, 0.0], 1.7e-3);
            let corner = cubes.domain_corner_m;
            let (mut low, mut high) = (f64::INFINITY, f64::NEG_INFINITY);
            cubes.tree.visit_leaves(&mut |origin, level, node| {
                if !is_cloud(node) {
                    return;
                }
                let side = cubes.cell_m * (1i32 << level) as f64;
                let y = corner[1] + origin[1] as f64 * cubes.cell_m;
                low = low.min(y);
                high = high.max(y + side);
            });
            (cubes.cells, if cubes.cells == 0 { 0.0 } else { high - low })
        };
        let (thin_cells, thin_band) = span(0.16);
        let (thick_cells, thick_band) = span(0.5);
        println!(
            "cut band: water 0.16 -> {thin_cells} cells over {thin_band:.0} m; water 0.5 -> \
             {thick_cells} cells over {thick_band:.0} m"
        );
        assert!(thin_cells > 0, "the thinnest fixture drew nothing at all");
        assert!(
            thick_cells >= thin_cells,
            "a wetter column was not drawn fatter: {thick_cells} against {thin_cells}"
        );
        assert!(
            thick_band > thin_band,
            "a wetter column's drawn band was not taller: {thick_band} against {thin_band}"
        );
    }

    #[test]
    fn the_cells_edge_is_fainter_than_its_core() {
        // **A cloud's edge is soft because its water is thin there.** The veil a cell carries is
        // `cloud_transmittance` of the water path *it* stands for, so the cell at the layer's own
        // middle is the most opaque and the outermost drawn cells are the faintest — which is what
        // a hard-edged stack of equal boxes cannot be. The band is monotone in the water, so the
        // middle's veil is the smallest of the set.
        let field = field_with(&[(10, 10, [0.4f32, 1_000.0, 1_800.0, 1.0])]);
        let cubes = CloudCubes::build(&field, [0.0, 20.0, 0.0], 1.7e-3);
        assert!(!cubes.origins.is_empty(), "the fixture drew nothing");
        let veils: Vec<f64> = cubes.origins.iter().map(|origin| origin[3]).collect();
        let faintest = veils.iter().cloned().fold(f64::INFINITY, f64::min);
        let thickest = veils.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        println!("veils {faintest:.3} .. {thickest:.3}");
        assert!(
            faintest < 1.0 && thickest <= 1.0,
            "a cloud rectangle lets more than the whole beam through: {faintest} .. {thickest}"
        );
        // The band's own middle is the opaque part: a cloud has a core, and most of its rectangles
        // attenuate the beam rather than passing it.
        let attenuating = veils.iter().filter(|veil| **veil < 0.95).count();
        assert!(
            attenuating * 2 >= veils.len(),
            "only {attenuating} of {} rectangles attenuate at all: the cloud is a shell of nothing",
            veils.len()
        );
        assert!(
            thickest - faintest > 0.0,
            "every rectangle carries the same veil: the edge is not softer than the core"
        );
    }

    #[test]
    fn the_same_field_and_eye_build_the_same_cubes() {
        // Determinism is a contract (F1): no clock, no RNG, no allocation order.
        let texel = [0.5f32, 1_000.0, 1_800.0, 1.0];
        let field = field_with(&[(10, 10, texel), (11, 11, texel), (40, 7, [0.3, 900.0, 1_400.0, 2.0])]);
        let first = CloudCubes::build(&field, [100.0, 30.0, -50.0], 1.7e-3);
        let second = CloudCubes::build(&field, [100.0, 30.0, -50.0], 1.7e-3);
        assert_eq!(first, second);
        assert!(first.cells > 0, "the fixture has no cloud to build");
    }

    /// **What the shipped world's clouds cost as geometry, and as time** — reported, not asserted,
    /// for the reason the field's own cost test is: nobody has ruled what a cloud is allowed to
    /// cost, so a threshold here would be an invented number wearing an assertion's clothes.
    #[test]
    fn the_shipped_fields_clouds_are_reported_as_geometry() {
        use voxel_shapers::spec::{Config, WorldSpec};
        let spec = WorldSpec::new(2468, Config::default());
        let tick = voxel_shapers::climate::TICKS_PER_DAY / 2;
        let field = CloudField::sample(&spec, tick, [0.0, 0.0]);
        if field.cloudy == 0 {
            return;
        }
        let started = std::time::Instant::now();
        let cubes = CloudCubes::build(&field, [0.0, 20.0, 0.0], 1.7e-3);
        let built_ms = started.elapsed().as_secs_f64() * 1000.0;
        let bytes = cubes.records.len() as u64 * QuadRecord::SIZE + cubes.origins.len() as u64 * 16;
        println!(
            "cloud octree: {} cloudy columns of {} -> {} cells at {} levels ({:.1} m leaves, {} \
             deep), {} rectangles, {:.2} MB per frame, {:.1} ms to build",
            field.cloudy,
            CLOUD_COLUMNS * CLOUD_COLUMNS,
            cubes.cells,
            cubes.subdivisions,
            cubes.cell_m,
            cubes.rows_max,
            cubes.records.len(),
            bytes as f64 / (1024.0 * 1024.0),
            built_ms,
        );
        assert!(cubes.records.len() > 0, "a cloudy field drew no geometry");
        assert_eq!(cubes.clipped, 0, "a rectangle left the record format's own range");
    }
}
