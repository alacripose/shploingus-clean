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
//! # Two trees, and the side in the sign
//!
//! A tree is a function of the field's tick, the eye's own sample cell and the pixel angle, so the
//! sky is **derived at a cadence and drawn every frame**: swapping one derivation in for the next
//! whole is a cloud that pops at 5 Hz, and every cell whose water crosses [`CLOUD_CUT_WATER`] does
//! it in one step. The two most recent trees are therefore held, and every rectangle says **which of
//! them it belongs to in the sign of its own veil** ([`veil_with_side`]) — the weights are in no
//! geometry at all. The newer tree's weight is one number read from the world's ticks
//! ([`fade_weight`]), and `shading.wgsl` interpolates with it, once per pixel.
//!
//! **A pair at complementary weights composites to one picture, not two.** Two veils faded into the
//! same beam multiply, so a rectangle drawn by both trees composites to `1 - (1 - a·w)(1 - a·(1-w))`
//! — the same opacity to within `a²/4`, which is a twentieth of the cloud's own thickness at the
//! densest core and nothing at a wisp's edge. That is what makes a cross-fade the right instrument
//! here and not a transparency trick: the two pictures are two derivations of *one* sky, so the join
//! has to preserve how much cloud is there, and the weights are what does it.
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
    cloud_transmittance, density_of, CloudField, CLOUD_COLUMNS, CLOUD_COLUMN_M, CLOUD_SAMPLE_M,
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
    if !pixel_angle_rad.is_finite()
        || !distance_m.is_finite()
        || pixel_angle_rad <= 0.0
        || distance_m < 0.0
    {
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

/// **The veil as a rectangle carries it to the shader**: `±(1 + the cell's own opacity)`, positive
/// for the **newer** of the two trees a frame may be joining and negative for the older, with
/// exactly `0` for terrain.
///
/// **The side travels in the sign of the one float a cloud already had**, so a frame that joins two
/// derivations needs no second attribute, no second buffer and no second draw: the shader reads the
/// sign to know which weight applies and does the interpolation itself, once per pixel, from one
/// uniform (`shading.wgsl`'s `fs_main`, `View::screen.w`) — which is what the frame thread would
/// otherwise have to do per rectangle, per frame, in a buffer the GPU then has to be handed again.
///
/// **The opacity is what is carried, and the `1` makes zero mean terrain.** A veil is a
/// *transmittance*, so a cross-fade interpolates the share of the beam the cell takes (`1 - veil`),
/// not the veil: `veil · w` would move a thin wisp three times as far as a dense core for the same
/// weight, a change of shape wearing a fade's clothes. And the offset is not decoration — a
/// transmittance is `0` for a cell that takes the whole beam, so a bare veil would read as terrain
/// at exactly the thickness a cloud is most opaque, which the shipped field reaches. An encoding a
/// cloud can never answer with zero keeps "this is not a cloud" an exact test.
pub fn veil_with_side(veil: f64, older: bool) -> f64 {
    let opacity = 1.0 - veil.clamp(0.0, 1.0);
    let encoded = 1.0 + opacity;
    if older {
        -encoded
    } else {
        encoded
    }
}

/// **How often the cloud is derived**, in ticks — the ladder's own **`Cadence::Sediment`** rung
/// (×20), one rung below the atmosphere's, and read from the ladder rather than restated.
///
/// # Why the cloud is not on the atmosphere's rung
///
/// The tree is a pure function of `(tick, eye, pixel angle)`, so the only thing this number decides
/// is **how stale a cloud the frame draws** — and the derivation is not cheap: `CLOUD_SAMPLES_PER_AXIS²`
/// = **4 225** model calls plus an octree fill over the field, measured at tens of milliseconds a
/// cadence on a thread that competes with the frame for a core ([`super::cloud_cubes`]'s own note on
/// the split). The *light* has to be on the atmosphere's ×4: a shadow's edge is a function of the
/// sun's angle and the eye can see it move. A cloud's *shape* is not like that — it is a picture of
/// something that changes over minutes, and a one-second-old shape is not distinguishable from a
/// fresh one. Deriving it at the atmosphere's rate paid five times for a difference no frame can
/// show, and this is that five back.
///
/// **The number is the ladder's, not a taste constant**: `Cadence::Sediment` is the rung H-8.3
/// declares below `Water`, so a reader can find it and a later edit cannot quietly make the cloud
/// the slowest or the fastest thing in the world.
pub fn cadence_ticks() -> u64 {
    voxel_shapers::stages::Cadence::Sediment
        .period_ticks()
        .max(1)
}

/// **The cross-fade's own period, in ticks** — the rung the tree is *derived* on, read from
/// [`cadence_ticks`] rather than restated.
///
/// The dissolve's length is the cloud's own derivation period and not a taste constant, which is what
/// makes the fade **phase-locked to the cadence** rather than merely about the same length. A newer
/// tree reaches full weight exactly as the next one is due, so at a standstill the picture is always
/// the join of two adjacent derivations and never a third.
pub fn fade_period_ticks() -> u64 {
    cadence_ticks()
}

/// **How much of the sky the newer tree owns**, `0` … `1`, from the world's own clock.
///
/// `arrived_at_tick` is the tick the newer tree **became the newest** — the tick its derivation
/// landed on the frame thread, and not the tick it was built from — and `live_tick` is the tick
/// being drawn. The weight is a pure function of two ticks, so it is the same number for every
/// frame drawn at one tick: the determinism contract the cadence exists to keep, with a fade
/// driven by `Instant` being exactly what would make the cloud a function of the frame rate.
///
/// **Why the anchor is the arrival and not the derivation's own tick.** The key a tree was derived
/// from moves for reasons that are not the tick — the eye crossing a sample cell, the rung changing
/// — and such a tree carries the **same** cadence tick as the one it replaces. Anchored to that
/// tick, an incoming tree's weight was therefore already whatever its predecessor had reached, so
/// it was drawn at full weight on the frame it arrived: the dissolve existed in the shader and
/// never ran, and the sky stepped. Anchored to the arrival, every tree starts its dissolve at zero.
///
/// **The dissolve still ends as the next derivation falls due**, which is the phase lock the
/// cadence is for: a tree that arrived is replaced by the next one `fade_period_ticks()` later, so
/// a worker that keeps up reaches full weight exactly as its successor lands, and one that lags is
/// drawn at full weight rather than served thinner than it is (the `clamp` below). The weight is
/// `0` for a `live_tick` behind its arrival, which is a clock that went backwards: the newest tree
/// is the one in hand, so it starts its own dissolve rather than being served a weight it has not
/// earned.
pub fn fade_weight(arrived_at_tick: u64, live_tick: u64) -> f64 {
    let period = fade_period_ticks() as f64;
    (live_tick.saturating_sub(arrived_at_tick) as f64 / period).clamp(0.0, 1.0)
}

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
        let levels = (CLOUD_DOMAIN_M / cell_m)
            .log2()
            .round()
            .clamp(0.0, CLOUD_LEVELS as f64) as u32;
        let corner = domain_corner_m(field);

        let mut tree = SparseVoxelOctree::with_levels(levels);
        let samples = SamplePyramid::new(field, corner);
        fill(
            &mut tree,
            field,
            &samples,
            corner,
            cell_m,
            levels,
            Ladder {
                eye_m,
                pixel_angle_rad,
            },
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
        (
            crate::cloud_field::CLOUD_SAMPLES_PER_AXIS,
            crate::cloud_field::CLOUD_SAMPLE_STRIDE,
        )
    }
}

/// **The tree's domain**: the cube's low corner in world metres.
///
/// Horizontally it is the field's own centre, so the field sits centred in the cube and the outer
/// band is empty; vertically the centre is **half the tropopause**, so the cube brackets the model's
/// ceiling and a storm has room to reach it. Every axis is snapped so the tree's own lattice lands
/// on whole subvoxels of the record format.
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
            let past = (lo[axis] - eye_m[axis])
                .max(eye_m[axis] - hi[axis])
                .max(0.0);
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
    classify(
        tree,
        field,
        samples,
        FillRegion { corner, cell_m },
        [0, 0, 0],
        levels,
        ladder,
    );
}

/// **The region being filled, as one value.** The corner and the finest cell size do not change
/// between a node and its children, so they travel together; the two things that *do* change —
/// the child's own origin and its level — stay the recursion's own parameters. The split is what
/// keeps the argument list short enough to read without shuffling six invariants per call.
#[derive(Clone, Copy)]
struct FillRegion {
    corner: [f64; 3],
    cell_m: f64,
}

/// **One box, and what the model's own water says about it.**
///
/// The three answers are the whole algorithm: *all of it is cloud* (one region write), *none of it
/// is* (nothing is written — the node is already [`Node::Empty`]), or *it straddles the cut*
/// (subdivide). At the finest level there is nothing left to subdivide with, so the cell's own
/// address decides — asked at the height inside the box where the model's own profile is
/// strongest, which is the same rule the drawn set has always used and the only form of it that can
/// see a layer thinner than the box it is cut into.
fn classify(
    tree: &mut SparseVoxelOctree,
    field: &CloudField,
    samples: &SamplePyramid,
    at: FillRegion,
    origin: [i32; 3],
    level: u32,
    ladder: Ladder,
) {
    let FillRegion { corner, cell_m } = at;
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
    //
    // **And it is asked at the height inside the box where that profile is strongest, not at the
    // box's own centre.** The profile is a single hump over the box's own column, so its maximum
    // over the box is at the layer's own middle — and a layer *thinner than the box* can leave the
    // box's centre outside it entirely. Asking the centre is how a single 64 m column of a 4 m-deep
    // layer drew **no cell at all** (measured: `cells 0, records 0`): the two 8 m boxes that straddle
    // the layer have their centres at `y = ±4`, where the profile is already zero, so every leaf was
    // refused and the sky held nothing. The clamped middle is the same number for a box inside the
    // layer, and the honest one for a box that merely overlaps it.
    let sample = field.at_m((lo[0] + hi[0]) / 2.0, (lo[2] + hi[2]) / 2.0);
    let strongest_y = strongest_y_of(sample, lo[1], hi[1]);
    let wet_here = || density_of(sample, strongest_y) >= CLOUD_CUT_WATER;

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
        classify(tree, field, samples, at, child, level - 1, ladder);
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

/// **The model's water on the cloud domain's own lattice, and the maximum over every doubling of
/// it.**
///
/// # Why the field's own columns, and at their own rate
///
/// A box that is empty has to be *known* empty, and four corner samples are not that. The root box
/// is 16 km across and its corners are outside the field entirely — a corner test prunes the whole
/// sky, which is exactly what the first version of this fill did. A 1 km box's corners can be dry
/// with a cumulus in its middle. So the boxes are bounded by the field's **own columns**, one every
/// [`CLOUD_COLUMN_M`], each carrying its own water, base and top: the wettest column a box covers
/// bounds every value inside it, and a box whose covering columns are all dry is dry. That is the
/// direction that cannot lose a cloud.
///
/// **The rate has to be the columns' own, and that was measured, not assumed.** `CloudField` takes
/// its *coverage* from the nearest sample and only interpolates the layer between samples, so a
/// lattice even twice as coarse as a column steps over a single one. The pyramid was first built on
/// the [`CLOUD_SAMPLE_M`] lattice — one point every second column — and a fixture of **one** cloudy
/// column read **zero wet samples** across the whole domain (`at_m(0, 256)` dry beside a cloud at
/// `z = 300`), so the fill called the sky empty and drew nothing at all. A lattice that cannot see
/// every column is not a bound on the columns. Between columns the layer is interpolated, so a box
/// also depends on the column **past its far face** — which the inclusive blocks below hold.
///
/// # Two aggregates, because a box has to be dismissable in two ways
///
/// `blocks` carries the **water**: the wettest sample under the box, with the base and top of the
/// *intersection* of the wet samples' layers (the highest base, the lowest top), which is what lets
/// a box wholly inside every one of them be written as **one** region however large it is.
///
/// `envelopes` carries the **union of the layers** instead — the lowest base and the highest top of
/// the wet samples under the box — which is what lets a box above the whole deck or below all of it
/// be called empty at once. **The two directions are both needed and neither is the other**: the
/// water says "is there cloud here at all, and is there room for one box", the envelope says "is
/// this box even in the sky the cloud can be in". Without the envelope there is no such test: every
/// large box has *some* wet sample under it, so it can never be called empty on water alone, and the
/// walk then descends to its own floor over the **whole** sky — measured at **29,669,881** boxes
/// visited for a deck a few hundred metres deep, 2.35 s of a frame's own time, to write 65,460
/// leaves.
///
/// # The blocks are inclusive, and anchored at the domain's corner
///
/// `blocks[k]` is the maximum over `2^k + 1` samples a side, from an index that is a multiple of
/// `2^k`, so a box of `2^k` sample intervals is answered by **one cell** — and that cell is exactly
/// the samples the box's own values depend on: its own, **and the one at its far face**, which still
/// reaches 64 m into a cloudy box at half weight.
///
/// Anchoring the lattice at the domain's own corner is what makes "a multiple of `2^k`" true. The
/// first version anchored it at the field's first column *centre* and used the aligned block
/// *containing* the box's corner — the block that *ends* there whenever the corner is not a multiple
/// of the block's width, and it is not. A 512 m box over a single 64 m column was then answered with
/// the samples of the half of the sky beside it: dry, every leaf beneath it pruned, and **a cloud
/// that did not exist in the picture at all** (measured: `cells 0, records 0`).
struct SamplePyramid {
    /// **The inclusive blocks**: `blocks[k]` is the wettest of `2^k + 1` columns a side, with the
    /// base and top of the intersection of their layers.
    blocks: Vec<Vec<[f32; 4]>>,
    /// **The same blocks' layer envelope**: the lowest base and the highest top of the columns under
    /// them that have water at all. `[inf, -inf]` where none has any.
    envelopes: Vec<Vec<[f32; 2]>>,
    /// Blocks a side at each level.
    sides: Vec<i32>,
    /// **The columns themselves**, for a box smaller than one column: there the covering set is the
    /// four columns at the cell's own corners, and no block is that small.
    samples: Vec<[f32; 4]>,
    samples_side: i32,
    /// The domain's own low corner in world metres — column `(0, 0)`, and the lattice every box's
    /// own low corner is a multiple of.
    origin_m: [f64; 2],
}

/// **The layer envelope of one column**, or the empty envelope when it has no water: `[inf, -inf]`.
///
/// **Dry columns are left out on purpose.** Their layer is `[0, 0]`, and a `min`/`max` that admitted
/// them would drag every block's lowest base to the ground — the one number the test below needs to
/// be the cloud's own base.
fn wet_envelope(sample: [f32; 4]) -> [f32; 2] {
    if (sample[0] as f64) < CLOUD_CUT_WATER {
        [f32::INFINITY, f32::NEG_INFINITY]
    } else {
        [sample[1], sample[2]]
    }
}

/// The **union** of two envelopes: the lower base and the higher top.
fn merge_envelope(held: [f32; 2], sample: [f32; 2]) -> [f32; 2] {
    [held[0].min(sample[0]), held[1].max(sample[1])]
}

impl SamplePyramid {
    /// **The pyramid over `field`, anchored at the domain's corner.**
    fn new(field: &CloudField, corner_m: [f64; 3]) -> Self {
        let origin_m = [corner_m[0], corner_m[2]];
        // One **column** a step across the whole domain, **plus the one past its far face**: a box of
        // `2^k` intervals needs the columns up to its own far face, and the outermost box *is* the
        // domain. The step is the column rate and not the model's sample rate: `CloudField` takes its
        // coverage from the nearest column, so a coarser lattice steps over single-column clouds and
        // calls the sky empty (measured: zero wet samples, a cloud not drawn at all).
        let samples_side = (CLOUD_DOMAIN_M / CLOUD_COLUMN_M).round() as i32 + 1;
        let mut samples = vec![[0.0f32; 4]; (samples_side * samples_side) as usize];
        for j in 0..samples_side {
            for i in 0..samples_side {
                let at = [
                    origin_m[0] + i as f64 * CLOUD_COLUMN_M,
                    origin_m[1] + j as f64 * CLOUD_COLUMN_M,
                ];
                samples[(j * samples_side + i) as usize] = field.at_m(at[0], at[1]);
            }
        }
        // **The base level is the pairs.** Level `k` stands for `2^k + 1` samples and the level
        // below it for `2^(k-1) + 1`, so the two halves of a block *share* the sample in the middle
        // — which is why the ordinary `2 x 2` doubling below produces exactly the inclusive block
        // and needs no second table.
        let side = samples_side - 1;
        let mut base = vec![[0.0f32; 4]; (side * side) as usize];
        let mut envelopes = vec![[f32::INFINITY, f32::NEG_INFINITY]; (side * side) as usize];
        for j in 0..side {
            for i in 0..side {
                let at = (j * side + i) as usize;
                let here = samples[(j * samples_side + i) as usize];
                let next = samples[(j * samples_side + i + 1) as usize];
                base[at] = merge_sample(here, next);
                envelopes[at] = merge_envelope(wet_envelope(here), wet_envelope(next));
            }
        }
        let mut pyramid = SamplePyramid {
            blocks: vec![base],
            envelopes: vec![envelopes],
            sides: vec![side],
            samples,
            samples_side,
            origin_m,
        };
        while *pyramid.sides.last().expect("a pyramid has a base") > 1 {
            pyramid.double();
        }
        pyramid
    }

    /// Add the next level: the maximum over each `2 x 2` block of the level below — which is the
    /// maximum over the `2^k + 1` samples that level stands for, because two adjacent blocks share
    /// the sample between them. **Both aggregates double together**: the water as `merge_sample`
    /// states it, the envelope as the union of the four.
    fn double(&mut self) {
        let prev = self.blocks.last().expect("a pyramid has a base");
        let prev_envelopes = self.envelopes.last().expect("a pyramid has a base");
        let prev_side = *self.sides.last().expect("a pyramid has a base");
        let side = (prev_side + 1) / 2;
        let mut next = vec![[0.0f32; 4]; (side * side) as usize];
        let mut next_envelopes = vec![[f32::INFINITY, f32::NEG_INFINITY]; (side * side) as usize];
        for j in 0..side {
            for i in 0..side {
                let mut cell: Option<[f32; 4]> = None;
                let mut envelope = [f32::INFINITY, f32::NEG_INFINITY];
                for dj in 0..2 {
                    for di in 0..2 {
                        let (pi, pj) = (i * 2 + di, j * 2 + dj);
                        if pi >= prev_side || pj >= prev_side {
                            continue;
                        }
                        let at = (pj * prev_side + pi) as usize;
                        let sample = prev[at];
                        cell = Some(match cell {
                            None => sample,
                            Some(held) => merge_sample(held, sample),
                        });
                        envelope = merge_envelope(envelope, prev_envelopes[at]);
                    }
                }
                next[(j * side + i) as usize] = cell.unwrap_or([0.0; 4]);
                next_envelopes[(j * side + i) as usize] = envelope;
            }
        }
        self.blocks.push(next);
        self.envelopes.push(next_envelopes);
        self.sides.push(side);
    }

    /// **The columns covering a box, in both directions** — the water and the layer envelope — or
    /// `None` when the box is not over the field at all.
    ///
    /// The two branches between them are exact for **every** box the tree can hold. A box's side is
    /// `cell_m * 2^k` with `cell_m` of 8, 16 or 32 m, so its span in columns is a power of two or a
    /// power of two over eight: below two columns the four columns at the box's own cell corners are
    /// the covering set (one or two columns a side, and the cell's corners carry two), and at two and
    /// above the inclusive block holds `2^k + 1` — the box's own columns and the one past its far
    /// face. There is no span in between for a box to fall through.
    fn coverage(&self, lo: [f64; 3], hi: [f64; 3]) -> Option<([f32; 4], [f32; 2])> {
        let span = (hi[0] - lo[0]) / CLOUD_COLUMN_M;
        if span >= 2.0 {
            // **One cell of the inclusive pyramid, which is the whole of the box's coverage.**
            //
            // The aligned block *containing* the box's low corner was the first version, and it is
            // the block that *ends* there whenever the corner is not a multiple of the block's width
            // — which, anchored at the field's first column centre, it never is. The second version
            // asked for the two blocks holding the box's ends: it found the cloud, but over-claimed
            // by up to a box width on each side, and since every level inherits its parent's
            // over-claim it walked **24,113,305** boxes to write 69,963.
            //
            // This is neither. The pyramid is anchored at the domain's corner, so `i >> level` names
            // the block that *begins* at the box's corner, and that block holds `2^k + 1` samples:
            // the box's own **and the one at its far face**, whose water still reaches a metre inside
            // it. No cloud lost, and nothing walked that the cloud is not in.
            let intervals = span.round() as i32;
            let level = (intervals.ilog2() as usize).min(self.blocks.len() - 1);
            let side = self.sides[level];
            // **Clamped, not refused.** A box larger than the domain has indices off the lattice, and
            // the honest answer for it is the samples it *does* cover, which is the nearest block
            // there is. Clamping can only over-claim coverage, and an over-claimed box is subdivided
            // rather than believed; a box genuinely beyond the field is caught by the four-corner
            // branch below, where every corner is off the lattice and the answer is `None`.
            let i = ((lo[0] - self.origin_m[0]) / CLOUD_COLUMN_M).round() as i32 >> level;
            let j = ((lo[2] - self.origin_m[1]) / CLOUD_COLUMN_M).round() as i32 >> level;
            let (i, j) = (i.clamp(0, side - 1), j.clamp(0, side - 1));
            let at = (j * side + i) as usize;
            return Some((self.blocks[level][at], self.envelopes[level][at]));
        }
        // **Inside one column cell**: the box is narrower than a column, so it lies within one or two
        // of them a side and the four columns at that cell's own corners bound every point of it.
        let base_side = self.samples_side;
        let i = ((lo[0] - self.origin_m[0]) / CLOUD_COLUMN_M).floor() as i32;
        let j = ((lo[2] - self.origin_m[1]) / CLOUD_COLUMN_M).floor() as i32;
        let mut cell: Option<[f32; 4]> = None;
        let mut envelope = [f32::INFINITY, f32::NEG_INFINITY];
        for dj in 0..2 {
            for di in 0..2 {
                let (si, sj) = (i + di, j + dj);
                if si < 0 || sj < 0 || si >= base_side || sj >= base_side {
                    continue;
                }
                let sample = self.samples[(sj * base_side + si) as usize];
                cell = Some(match cell {
                    None => sample,
                    Some(held) => merge_sample(held, sample),
                });
                envelope = merge_envelope(envelope, wet_envelope(sample));
            }
        }
        cell.map(|sample| (sample, envelope))
    }

    /// **What the model's own samples say about a box.** The whole of the subdivision rule.
    fn claim(&self, lo: [f64; 3], hi: [f64; 3]) -> Claim {
        let Some((sample, envelope)) = self.coverage(lo, hi) else {
            return Claim::Empty;
        };
        if (sample[0] as f64) < CLOUD_CUT_WATER {
            // The aggregate is the *wettest* sample under the box, so every one of them is dry — and
            // a value between two dry samples is dry.
            return Claim::Empty;
        }
        // **A box outside every layer the model has under it is empty, and that is the cheapest box
        // there is.** The envelope is the union of those layers: below its lowest base, a blend of
        // them has no profile at all; above its highest top, likewise. This is the test that keeps
        // the walk inside the deck — on water alone a large box can never be called empty, so the
        // fill descended to its own floor across the whole sky (measured: **29,669,881** boxes for a
        // deck of a few hundred metres, to write 65,460 leaves).
        let (lowest_base, highest_top) = (envelope[0] as f64, envelope[1] as f64);
        if hi[1] <= lowest_base || lo[1] >= highest_top {
            return Claim::Empty;
        }
        // It is one region of cloud when the box lies inside the **intersection** of the covering
        // layers and the wettest covering sample's own profile clears the cut at **both** of the
        // box's ends: the profile is a single hump, so its minimum over the box is at an end.
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

/// One plane's cells: the running **veil sum** and the **count** that turns it into the merged
/// rectangle's own mean opacity, keyed by the cell's `[u, v]` position on the plane.
type Plane = BTreeMap<[i32; 2], (f64, u32)>;

/// The leaves drawn, grouped by **everything a merge must agree on**: the face, the level the
/// leaf is drawn at, the plane it lies on, and the veil's bucket. Named because the key is four
/// fields wide and a reader should not have to count them at every use.
type Groups = BTreeMap<(u32, u32, i32, u32), Plane>;

/// **The height inside a box where the model's own profile is strongest** — the middle of its layer,
/// clamped into the box. The tree's own decision is asked here: whether a box is a cloud at all is a
/// question about the water at a point inside it, and the middle of the layer is the point that
/// answers for the box rather than for one of its corners.
fn strongest_y_of(sample: [f32; 4], lo_y: f64, hi_y: f64) -> f64 {
    ((sample[1] as f64 + sample[2] as f64) / 2.0).clamp(lo_y, hi_y)
}

/// **Where one drawn face belongs, in the record's own subvoxels** — the leaf's own face, on the
/// ladder's lattice.
///
/// **The plane is the leaf's on purpose, and the motion between two states is the veil's.** Placing
/// each face where [`density_of`] equals [`CLOUD_CUT_WATER`] reads better and was tried; it measures
/// **94,727 rectangles against 39,540** on the shipped field, 3.61 MB against 1.51 MB a frame and
/// ~395 ms against ~344 ms to build, and the reason is structural rather than a matter of care: **the
/// plane is part of the merge key**, so a boundary that follows the water stops merging with its
/// neighbour, and a surface that was one long rectangle becomes a patchwork of cells on slightly
/// different planes — which is where the seams came from, at 2.4× the bytes. A plane that is a
/// function of the leaf merges; a plane that is a function of the water cannot.
///
/// So the geometry here is deliberately **independent of the water**: two derivations whose trees
/// are the same produce the same bytes for every water, and a raining column's shape moves because
/// the *next* derivation draws it that way and the two are cross-faded in opacity (C-183) — not
/// because a face was slid off its own plane, which would take the cubes out of a picture whose
/// whole cost argument is that they merge.
fn lattice_plane_sub(
    corner: [f64; 3],
    lo: [f64; 3],
    hi: [f64; 3],
    axis: usize,
    positive: bool,
) -> i32 {
    let face = if positive { hi[axis] } else { lo[axis] };
    ((face - corner[axis]) * SUBVOXEL_SIDE as f64).round() as i32
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
    let mut groups: Groups = BTreeMap::new();
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
        let bucket = ((1.0 - veil) * VEIL_BUCKETS as f64)
            .round()
            .clamp(0.0, VEIL_BUCKETS as f64) as u32;

        for face in 0..6u32 {
            let (axis, positive) = face_axis_and_sign(face);
            let axis = axis as usize;
            // **A face is drawn unless the region behind it is cloud.** The neighbour is the same
            // size as this leaf, one leaf along the axis: a leaf answer means the region is
            // uniformly cloud (hidden) or uniformly empty (drawn), and a branch answer means it is
            // subdivided, so part of this face is exposed and it stays.
            let mut neighbour = origin;
            neighbour[axis] = if positive {
                origin[axis] + half
            } else {
                origin[axis] - half
            };
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
            let away = if positive {
                eye_m[axis] < lo[axis]
            } else {
                eye_m[axis] > hi[axis]
            };
            if away {
                continue;
            }
            let (u_axis, v_axis) = face_axes(face);
            let u = origin[u_axis] / half;
            let v = origin[v_axis] / half;
            // **The plane is the leaf's own face** — see `lattice_plane_sub` for why the crossing is
            // not baked here, and for the numbers that decided it.
            let plane = lattice_plane_sub(corner, lo, hi, axis, positive);
            let entry = groups.entry((face, level, plane, bucket)).or_default();
            let cell = entry.entry([u, v]).or_insert((0.0, 0));
            cell.0 += veil;
            cell.1 += 1;
        }
    });

    let mut drawn = Drawn {
        cells,
        deepest,
        ..Drawn::default()
    };
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
            // Already in subvoxels: `lattice_plane_sub` returns the plane in the record's own unit.
            let plane_sub = plane as f64;
            let holds = |value: f64| value.is_finite() && (0.0..=u16::MAX as f64).contains(&value);
            if !(holds(u_sub) && holds(v_sub) && holds(du_sub) && holds(dv_sub) && holds(plane_sub))
            {
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
                light_y: 0,
                light_z: 0,
                shape_lo: 0,
                shape_hi: 0,
                shape_twist: 0,
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
fn greedy_merge(grid: &Plane) -> Vec<(i32, i32, i32, i32, f64)> {
    let mut used: BTreeSet<[i32; 2]> = BTreeSet::new();
    let mut out = Vec::new();
    for &[u, v] in grid.keys() {
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
            let Some(texel) = field.texel(i, j) else {
                continue;
            };
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
            assert!(
                cell >= previous,
                "the ladder went finer with distance at {step}"
            );
            previous = cell;
        }
    }

    #[test]
    fn a_cloud_thinner_than_its_own_cells_is_still_drawn() {
        /// **One column, four metres of layer — the fixture that found both ways this fill could lose
        /// a cloud, and now the witness that neither has come back.** At the ladder's floor that
        /// cloud is barely a cell of sky, so nothing here survives a bound that samples only where it
        /// is convenient.
        ///
        /// * **A pyramid on the model's own 128 m sample lattice reads the gaps between columns.**
        ///   `CloudField::at_m` takes its *coverage* from the nearest column while it interpolates
        ///   the layer, so a lattice twice the column rate measured **zero wet samples** over the
        ///   whole domain: the fill called the sky empty and drew no rectangle at all.
        /// * **A pyramid anchored anywhere but the domain's corner answers a box with the block that
        ///   *ends* at its corner.** A 512 m box over this column was answered by the dry half of the
        ///   sky beside it, and its whole subtree was pruned — measured as **no cells and no
        ///   rectangles**.
        ///
        /// The leak check is the other half, and it is not decoration: a bound that over-claims
        /// writes cloud where the model has none, so every cell written here is held to the column's
        /// own footprint.
        const I: u32 = 10;
        let layer = [0.8f32, -2.0, 2.0, 1.0];
        let field = field_with(&[(I, I, layer)]);
        // The field's own origin is the corner of its lattice, so column `I` holds
        // `[I·64 m, (I+1)·64 m]` in both directions.
        let (low, high) = (I as f64 * CLOUD_COLUMN_M, (I as f64 + 1.0) * CLOUD_COLUMN_M);

        let cubes = CloudCubes::build(&field, [0.0, 0.0, 0.0], 1.7e-3);
        assert!(
            cubes.cells > 0,
            "a cloud thinner than its own cells wrote no cell at all"
        );
        assert!(
            !cubes.records.is_empty(),
            "{} cells were written and not one rectangle drawn",
            cubes.cells
        );

        let corner = cubes.domain_corner_m;
        let mut walked = 0usize;
        let mut outside = 0usize;
        cubes.tree.visit_leaves(&mut |origin, level, node| {
            if !is_cloud(node) {
                return;
            }
            walked += 1;
            let side = cubes.cell_m * (1i32 << level) as f64;
            let lo = [
                corner[0] + origin[0] as f64 * cubes.cell_m,
                corner[1] + origin[1] as f64 * cubes.cell_m,
                corner[2] + origin[2] as f64 * cubes.cell_m,
            ];
            assert!(
                lo[1] < layer[2] as f64 && lo[1] + side > layer[1] as f64,
                "a cloud cell lies outside the model's own layer: {lo:?} + {side}"
            );
            // Give or take the one cell that straddles the column's own edge.
            let one = cubes.cell_m;
            if lo[0] < low - one
                || lo[0] + side > high + one
                || lo[2] < low - one
                || lo[2] + side > high + one
            {
                outside += 1;
            }
        });
        assert_eq!(
            walked, cubes.cells,
            "the tree's own count and the walk's disagree about the cells"
        );
        assert_eq!(
            outside, 0,
            "{outside} cloud cells were written outside the one cloudy column"
        );
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
        let field = field_with(&[
            (10, 10, texel),
            (11, 10, texel),
            (10, 11, texel),
            (11, 11, texel),
        ]);
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
        let columns: Vec<(u32, u32, [f32; 4])> = (8..12)
            .flat_map(|i| (8..12).map(move |j| (i, j, texel)))
            .collect();
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
        assert!(!cubes.records.is_empty(), "the fixture drew nothing");

        // **No two rectangles on one face and plane overlap**, and every rectangle has area. Both
        // are claims about the merge rather than about the field, and the overlap test is done on
        // the rectangles' own intervals so a big face costs nothing to check.
        let mut planes: BTreeMap<(u32, u32), Vec<[u32; 4]>> = BTreeMap::new();
        for record in &cubes.records {
            let low = record.low();
            let size = record.size();
            assert!(
                size[0] > 0 && size[1] > 0,
                "a rectangle has no area: {record:?}"
            );
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
        assert_eq!(
            cubes.clipped, 0,
            "a rectangle left the record format's own range"
        );
    }

    #[test]
    fn a_wetter_column_is_drawn_as_a_taller_fatter_cloud() {
        // **The emergent shape, from the model's own numbers.** The cut is on
        // `water · sin(π · through)`, so a column holding more water clears it over a wider band of
        // its own layer: the wisp is a lens and the cumulus is a mass, from the same two numbers
        // and no shape this module invented. Measured as the drawn band's height and as the
        // **drawn volume** — see the note on the leaf count below.
        let span = |water: f32| {
            // The fixture's heights are **real-world** metres put through the world's scale,
            // like every other altitude: a bare 1 000/1 800 would now sit above the scaled
            // tropopause and be clipped out of the domain the cubes are sized on.
            let field = field_with(&[(
                10,
                10,
                [
                    water,
                    voxel_shapers::scale::voxels(1_000.0) as f32,
                    voxel_shapers::scale::voxels(1_800.0) as f32,
                    1.0,
                ],
            )]);
            let cubes = CloudCubes::build(&field, [0.0, 20.0, 0.0], 1.7e-3);
            let corner = cubes.domain_corner_m;
            let (mut low, mut high) = (f64::INFINITY, f64::NEG_INFINITY);
            let mut volume = 0.0f64;
            cubes.tree.visit_leaves(&mut |origin, level, node| {
                if !is_cloud(node) {
                    return;
                }
                let side = cubes.cell_m * (1i32 << level) as f64;
                // The drawn mass itself: what the record's own box encloses, which is what "fatter"
                // means on screen. A leaf's own side is its level's, so a merged interior counts once
                // and counts large — the same reason the count below is not the measurement.
                volume += side * side * side;
                let y = corner[1] + origin[1] as f64 * cubes.cell_m;
                low = low.min(y);
                high = high.max(y + side);
            });
            (
                cubes.cells,
                if cubes.cells == 0 { 0.0 } else { high - low },
                volume,
            )
        };
        let (thin_cells, thin_band, thin_volume) = span(0.16);
        let (thick_cells, thick_band, thick_volume) = span(0.5);
        println!(
            "cut band: water 0.16 -> {thin_cells} cells over {thin_band:.0} m ({thin_volume:.3e} m³); \
             water 0.5 -> {thick_cells} cells over {thick_band:.0} m ({thick_volume:.3e} m³)"
        );
        assert!(thin_cells > 0, "the thinnest fixture drew nothing at all");
        // **The leaf count is not the measurement, and the first version of this test was wrong to
        // take it as one.** The fill merges every box whose samples are all wet and whose own address
        // agrees into **one** region — "the interior is one node", which is the whole reason a coarse
        // lattice is affordable — so a denser column is *fewer and larger* leaves, and counting them
        // measures the subdivision rather than the cloud. The denser column read **5 cells against
        // 81** here, which the old assertion called a failure while the drawn mass was larger.
        assert!(
            thick_volume > thin_volume,
            "a wetter column was not drawn larger: {thick_volume:.3e} m³ against {thin_volume:.3e} m³"
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
        let field = field_with(&[
            (10, 10, texel),
            (11, 11, texel),
            (40, 7, [0.3, 900.0, 1_400.0, 2.0]),
        ]);
        let first = CloudCubes::build(&field, [100.0, 30.0, -50.0], 1.7e-3);
        let second = CloudCubes::build(&field, [100.0, 30.0, -50.0], 1.7e-3);
        assert_eq!(first, second);
        assert!(first.cells > 0, "the fixture has no cloud to build");
    }

    /// **The geometry is the ladder's, so it does not move when the water does.**
    ///
    /// One column of cloud with its water thinned continuously — rain's own act — and the drawn
    /// planes read off the records after each step. Two claims, and they are the two the merge and the
    /// frame rate need: every plane is a **face of the tree** (so a flat boundary merges into one
    /// rectangle instead of a patchwork), and **the bytes are the same for every water** (so a
    /// raining column is drawn by the next derivation and cross-faded by the shader, rather than by
    /// re-cutting a tree that costs hundreds of milliseconds).
    #[test]
    fn the_drawn_planes_are_faces_of_the_tree_and_do_not_move_with_the_water() {
        /// **A layer thinner than the cell the ladder draws it at, above the eye** — which is the
        /// case the old geometry could not state at all. The tree's leaf is the whole box around the
        /// layer; the layer's own base sits a little inside it; and the face a player under the cloud
        /// sees is the box's **underside**, whose plane used to be the box's own bottom face and
        /// therefore did not move when the water did, because the box does not move.
        const AT_Z: f64 = 300.0;
        const I: u32 = 10;
        /// **A base off the lattice on purpose.** The leaf the tree draws this layer as is a box of
        /// `cell_m` whose own low face is a multiple of it; a base that sat on one of those planes
        /// would make the drawn crossing and the box's face the same number at the wet end, and the
        /// check could not tell which one it was reading.
        const BASE: f32 = 101.0;
        const TOP: f32 = 105.0;
        const EYE: [f64; 3] = [0.0, 0.0, 0.0];
        let field = |water: f32| -> CloudField {
            let n = CLOUD_COLUMNS as usize;
            let mut columns = vec![[0.0f32; 4]; n * n];
            columns[I as usize * n + I as usize] = [water, BASE, TOP, 1.0];
            CloudField {
                origin_m: [
                    -((I as f64) + 0.5) * CLOUD_COLUMN_M,
                    AT_Z - ((I as f64) + 0.5) * CLOUD_COLUMN_M,
                ],
                columns,
                slab_base_m: BASE as f64,
                slab_top_m: TOP as f64,
                cloudy: 1,
                filled_micros: 0,
            }
        };
        // The underside's own plane, in metres: the record's subvoxels past the domain corner.
        let undersides = |water: f32| -> Vec<f64> {
            let cubes = CloudCubes::build(&field(water), EYE, 1.7e-3);
            let below: Vec<f64> = cubes
                .records
                .iter()
                .filter(|record| record.face == 2)
                .map(|record| {
                    cubes.origins[record.chunk as usize][1]
                        + record.slice as f64 / SUBVOXEL_SIDE as f64
                })
                .collect();
            assert!(
                !below.is_empty(),
                "the fixture drew no underside at water {water}"
            );
            below
        };

        // A sweep that stops while the layer's own middle still clears the cut, so every step is a
        // cloud the model says is there and the assertion below is never reading an empty sky.
        let mut planes_m: Vec<f64> = Vec::new();
        for step in 0..=60 {
            let water = 1.0 - step as f32 * 0.015_825;
            assert!(
                water > CLOUD_CUT_WATER as f32,
                "the sweep walked out of the cloud at {water}"
            );
            let mut drawn = undersides(water);
            drawn.sort_by(|a, b| a.partial_cmp(b).expect("a plane is a number"));
            planes_m.push(drawn[0]);
        }

        // **One plane, for every water in the sweep**, because a leaf's face is a function of the ladder and never of the water.

        // **The plane is a face of the tree, and the water does not touch it.** Every step of the
        // sweep drew the same plane, because a leaf's face is a function of the ladder and never of
        // the water — which is the property that lets the motion be the shader's instead of a
        // rebuild, and the property that stops a flat boundary from splitting into a patchwork.
        for pair in planes_m.windows(2) {
            assert_eq!(
                pair[0], pair[1],
                "the plane moved with the water: {} then {}",
                pair[0], pair[1]
            );
        }
        // **On the ladder's own lattice.** The record's subvoxels are metres from the domain's own
        // corner, so a plane a whole number of cells away from it is a face of the tree and not a
        // slide: this is what the merge keys on, and a merge can only key on the tree.
        let cell = CloudCubes::build(&field(1.0), EYE, 1.7e-3).cell_m;
        let wet = CloudCubes::build(&field(1.0), EYE, 1.7e-3);
        for record in &wet.records {
            let steps = record.slice as f64 / SUBVOXEL_SIDE as f64 / cell;
            assert!(
                (steps - steps.round()).abs() < 1e-9,
                "a plane {steps} cells from the corner is not a face of the tree"
            );
        }
        // **The same tree at another water draws the same planes.** A thinner column is a different
        // sky — its cells are drawn thinner, and its veils fall into other buckets, so the merge
        // regroups — but a *plane* is a face of the tree and the tree has not changed, so the set of
        // planes is identical. That is what leaves the motion to the shader and the rebuild to the
        // derivation's own cadence.
        let planes_of = |cubes: &CloudCubes| -> BTreeSet<(u32, u32)> {
            cubes
                .records
                .iter()
                .map(|record| (record.face, record.slice))
                .collect()
        };
        let thinner = CloudCubes::build(&field(0.6), EYE, 1.7e-3);
        assert_eq!(
            planes_of(&wet),
            planes_of(&thinner),
            "the planes moved with the water, so a slide would cost a rebuild"
        );
        assert_ne!(
            wet.origins, thinner.origins,
            "the veil did not follow the water"
        );
        println!(
            "the underside over a thinning column: {:.3} m held across {} derivations, against a \
             {cell:.3} m cell",
            planes_m[0],
            planes_m.len()
        );
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
        assert!(!cubes.records.is_empty(), "a cloudy field drew no geometry");
        assert_eq!(
            cubes.clipped, 0,
            "a rectangle left the record format's own range"
        );
    }

    #[test]
    fn the_side_rides_the_sign_and_the_fade_interpolates_opacity() {
        // The newer side is positive, the older negative, and the two carry the same magnitude —
        // so the shader's `!= 0.0` is "a cloud" and its `> 0.0` is which one.
        for veil in [0.0, 0.01, 0.2, 0.5, 0.72, 0.95, 1.0] {
            let newer = veil_with_side(veil, false);
            let older = veil_with_side(veil, true);
            assert_eq!(
                newer, -older,
                "the two sides do not share a magnitude at {veil}"
            );
            assert!(newer > 0.0 && older < 0.0, "a side lost its sign at {veil}");
            // A cloud can never answer zero, so "not a cloud" stays an exact test.
            assert!(
                (1.0..=2.0).contains(&newer),
                "the encoding left its own range at {veil}"
            );
            assert_ne!(newer, 0.0);
            // The decode is what the shader does: `1 - (|v| - 1)·w` is the tree's own number at one.
            for weight in [0.0, 0.25, 0.75, 1.0] {
                let faded = 1.0 - (newer - 1.0) * weight;
                assert!(
                    (faded - (1.0 - (1.0 - veil) * weight)).abs() < 1e-12,
                    "a weight of {weight} over a veil of {veil} decoded to {faded}"
                );
                // **Inside the cell's own range, to within a rounding step.** A fade runs from the
                // tree's own transmittance, through clear air, to `1.0` — never past either end —
                // and `1 - (1 - 0.72)` is one ulp under `0.72` in binary, which is the arithmetic and
                // not a fade that leaked.
                assert!(
                    faded <= 1.0 && faded >= veil - 1e-15,
                    "a weight of {weight} left the veil {veil}'s own range: {faded}"
                );
            }
        }
        // The natural wrong path — multiplying the veil — misses half weight by 45 %:
        let right = 1.0 - (veil_with_side(0.95, false) - 1.0) * 0.5;
        let wrong = 0.95 * 0.5;
        assert!(
            (right - 0.975).abs() < 1e-12,
            "half weight is not half the opacity"
        );
        assert!(
            (wrong - right).abs() > 0.4,
            "`veil · w` would have passed this check"
        );
    }

    #[test]
    fn the_fade_is_phase_locked_to_the_cadence_that_derives_the_tree() {
        let period = fade_period_ticks();
        assert_eq!(
            period,
            voxel_shapers::stages::Cadence::Sediment.period_ticks(),
            "the cloud's cadence moved and this fade did not move with it"
        );
        // **And it is the rung below the atmosphere's, not the atmosphere's own.** The whole point of
        // this number is that the cloud is derived *less* often than the light it is drawn against: a
        // fade as long as the rig's cadence would be five dissolves per second of picture for a shape
        // that had not moved.
        assert!(
            period > voxel_shapers::stages::Stage::Atmosphere.cadence().period_ticks(),
            "the cloud is derived as often as the atmosphere, which is the cost this rung exists to \
             avoid"
        );
        let t = 40_000;
        assert_eq!(
            fade_weight(t, t),
            0.0,
            "a tree is the older half of its own first frame"
        );
        assert_eq!(fade_weight(t, t + period / 2), 0.5);
        assert_eq!(
            fade_weight(t, t + period),
            1.0,
            "the fade outlived its cadence"
        );
        // A late derivation is a staler picture, not a slower dissolve.
        assert_eq!(fade_weight(t, t + 10 * period), 1.0);
        // A clock that went backwards cannot be served a negative share.
        assert_eq!(fade_weight(t, t - 1), 0.0);
        // Monotone across the cadence, which is what makes two adjacent trees complementary:
        let mut previous = 0.0;
        for step in 0..=period {
            let weight = fade_weight(t, t + step);
            assert!(weight >= previous, "the fade went backwards at step {step}");
            previous = weight;
        }
        assert!(period > 0, "a period of zero ticks cannot be divided by");
    }

    #[test]
    fn a_tree_that_shares_its_predecessors_cadence_tick_still_dissolves() {
        // **The pop, as a property of two ticks.** A tree is re-derived for reasons that are not
        // the clock — the eye crossing a sample cell, the rung changing — and such a tree is built
        // from the *same* atmosphere cadence tick as the one it replaces. Anchor the dissolve to
        // that tick and the incoming tree is served its predecessor's own weight, which by then is
        // full: it lands at once, with different geometry, and the sky steps.
        let period = fade_period_ticks();
        let cadence_tick = 40_000;
        // The predecessor has been on screen for a whole cadence, so the shared tick reads full.
        assert_eq!(
            fade_weight(cadence_tick, cadence_tick + period),
            1.0,
            "the derivation tick is already spent when the replacement lands"
        );
        // Anchored to the arrival, the replacement starts at nothing and earns its way in over the
        // cadence — the dissolve the shared tick could not express. Both trees use the same
        // cadence tick above; only the anchor differs, which is the whole of the repair.
        let arrived = cadence_tick + 3;
        assert_eq!(
            fade_weight(arrived, arrived),
            0.0,
            "a new tree arrived opaque"
        );
        assert!(
            fade_weight(arrived, arrived + 1) > 0.0,
            "the dissolve never started"
        );
        assert_eq!(
            fade_weight(arrived, arrived + period),
            1.0,
            "the dissolve never finished"
        );
    }
}
