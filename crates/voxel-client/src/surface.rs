//! **The surface rungs: the ladder's tiers above a chunk, measured one sample a cell.**
//!
//! A rung whose cell is larger than a chunk cannot be censused chunk by chunk — a 64 m cell spans
//! eight chunks, and the per-chunk census of tiers 5..7 has no cell for it. Building eight chunk
//! octrees to answer one cell is what made that rung cost 2 304 tree builds a ring. This module is
//! the other answer, and it is the generator's own:
//!
//! * **one sample a column**, [`voxel_shapers::terrain::elevation_m`], the same function the chunk
//!   builder uses to decide where the ground is;
//! * the sample's material from [`voxel_shapers::chunk_fields::material_at`], at the voxel the
//!   surface falls in — the same function the chunk builder uses to decide what the ground is made
//!   of;
//! * and **both readers read that one sample**: the drawn top face stands at the sample's own
//!   height inside its cell, and the band word for that cell is the share of it standing above its
//!   floor. A face and the light that answers it cannot disagree, because there is only one number.
//!
//! # The ring
//!
//! A rung draws an **annulus**: `inner ..= 16` cells from the eye's own cell, where `inner` is 6 for
//! the first rung (so it begins at 384 m, the outer edge of tier 7's drawn ring) and 8 for every
//! rung above it. A cell doubling makes `8 × C_t = 16 × C_{t−1}`, so rung *t*'s inner edge **is**
//! rung *t−1*'s outer edge: the rings tile the ground from 384 m to
//! [`crate::lod::surface_outer_m`]`(31)` = 8 589 935 km with no gap and no overlap.
//!
//! The **whole disc** is sampled — `33 × 33` columns, the eye's cell in the middle — and only the
//! annulus is meshed, because the band needs an answer everywhere inside the rung's own reach: a
//! ray from a face at the ring's inner edge crosses ground nearer than the ring, and an unstamped
//! cell there would be an invented clear sky.
//!
//! # What a cell is drawn as, and what its band word is
//!
//! A column's sample is a height: `layer` cells plus `share` of one, and the cell it falls in is
//! drawn on it. But the height a drawn cell **states at its edges** is the corner lattice's, not
//! its own middle's: a roof is the bilinear through the four corners the cell shares with its
//! neighbours, anchored at the low corner, so the two cells either side of an edge state the same
//! two corner heights there and the rung has no seam inside it. **No side face is drawn inside a
//! rung** for exactly that reason; the annulus's inner and outer edges draw none either, because
//! **an unmeasured neighbour is not air**, which is the law every rung here shares.
//!
//! The band word for the cell the sample falls in is `FAR_MEASURED | share × 255`; the cells below
//! it are full (`255`) because the sampled column is matter from its top down, and the cells above
//! are measured air. So the march's own threshold reads the same number the mesh drew: a cell whose
//! sample stands three quarters of a cell tall stops a ray, and one whose sample is a tenth does
//! not.
//!
//! # The near disc is **shaped**, not a plane a cell wide
//!
//! A rung's cell is a block, and a block in this world is voxels: the world's own finest surface
//! denominator is [`voxel_model::profile32`]'s — **32 divisions a block axis**, evaluated and never
//! stored — so the near disc's own cells are drawn from those point samples rather than from one
//! number a cell. Each sample asks the same two laws the cell's own sample does (the elevation and
//! the material at that point), and the block is drawn as **sub-cuboids at the rung's record
//! grain**: a top face a profile cell wide at its own sample's own height, and a wall wherever the
//! neighbouring sample stands lower. The rest of the rung — cells a few pixels across from where
//! the eye stands — keeps the one-sample-a-cell law, which is what that law is for.
//!
//! # Units: a cell is 256 sub-cells in a record
//!
//! The quads are in **cell units** — a 536 870 912 m cell is 2 147 483 648 subvoxels, past what a
//! `u16` rectangle can state — and a cell is cut into [`crate::lod::SURFACE_SUB_CELLS`] `= 256`
//! steps so the top face can stand at the sample's own height inside it. At the first rung that
//! step is exactly one of the octree's own subvoxels (64 m / 256 = 0.25 m). The record carries the
//! **rung** in `light_y` and the **step** in `light_z`; `quad_corner_m` multiplies by the step and
//! the fragment's `world_band_for` reads the rung.
//!
//! # The ladder is a worker, and it fills nearest first
//!
//! Measuring a rung is 1 089 samples of the generator's own surface law — one column in about ten
//! microseconds, so a rung is a few milliseconds and the whole 24-rung ladder under half a second.
//! It is still not a frame's work, so it happens on a thread of its own, **one rung at a time,
//! nearest first**: the first rung arrives alone and the frame draws it, and each rung after it
//! appears as it is measured rather than when the ladder is finished.
//!
//! On a crossing of a rung's own cell the ring is measured again around the new eye cell, and the
//! ring already in hand **stays drawn until the new one lands**: its world extent is fixed, so the
//! union of the rings in hand still covers the ground — the neighbouring rungs' spans cover what a
//! lagging ring has not re-anchored yet — and every face on screen has the measurement it was cut
//! from.

use std::collections::BTreeMap;
use std::sync::mpsc::{Receiver, Sender};

use voxel_mesh::Quad;
use voxel_model::{Face, MaterialId};
use voxel_shapers::{chunk_fields, terrain, WorldSpec};

/// **Columns along one axis of a rung's disc**: 33, so 16 cells either side of the eye's own.
pub const RING_CELLS: i32 = 33;

/// The eye's cell sits at this index of the disc, and the drawn annulus ends at it.
pub const RING_MID: i32 = 16;

/// Where the drawn annulus starts for every rung above the first, in cells from the eye's own.
pub const RING_INNER: i32 = 8;

/// **The first rung's inner cell**: `6`, which is 384 m at 64 m a cell — the first whole cell past
/// tier 7's drawn ring (chunks `radius+5 ..= radius+6`, 320..384 m at the shipped window), so the
/// ladder begins exactly where the chunk-held rungs end.
pub const FIRST_INNER: i32 = 6;

/// **How far the first rung's near disc reaches, in its own cells** — the cells it draws *inside*
/// its hole, and only while the streamed slab stands above the ground (see
/// [`SurfaceRing::near`]).
///
/// Three cells is 224 m at 64 m a cell, so the disc spans 7 x 7 cells around the eye's own and
/// reaches every chunk the shipped window holds: the window is `radius 5` chunks — `|dx|, |dz| <=
/// 5`, whose outer edge is 176 m from the eye's own chunk centre — and the chunk-held rings begin
/// where those end (tier 5's ring, chunks `radius+1 ..= radius+7`). The disc is **at least** one
/// chunk past the window's edge in the worst alignment and at most one of its own cells past the
/// first chunk-held ring, which is the band the hand-over overlaps in; a coarser rung cannot tile
/// a 176 m half-width exactly, and a **gap** is the defect this disc answers while an overlap of
/// one coarse cell over ground both rungs measured is a seam.
pub const NEAR_CELLS: i32 = 3;

/// **How many point samples one drawn cell gets, per axis**: the world's own finest surface
/// denominator, [`voxel_model::profile32::PROFILE_SIDE`], which is the fill denominator too — so
/// "half a block" and "half a trunk" are the same fraction.
pub const PROFILE_PER_CELL: i32 = voxel_model::profile32::PROFILE_SIDE as i32;

/// How many record sub-cell steps one point sample spans: a cell is [`SUB_CELLS`] steps.
pub const PROFILE_STEPS: i32 = SUB_CELLS / PROFILE_PER_CELL;

/// **How far the shaped part of the near disc reaches, in the rung's own cells.**
///
/// The disc is 7 × 7 cells; the cells the eye is standing over — this many either side of its own —
/// are the ones whose interior is worth sampling at the profile's grain, and they are shaped from
/// [`PROFILE_PER_CELL`] samples an axis. The ring of cells outside them keeps the rung's own law of
/// one sample a cell, which is the same law this shape is a refinement of: the hand-over region is
/// the ground under the player, and a cell 200 m out is a few pixels.
pub const NEAR_SHAPED: i32 = 2;

/// **One point-based sample of a drawn block's shape.**
///
/// `plane` is in the rung's own sub-cell steps ([`SurfaceRing::step_m`]), so a sample is expressed
/// in exactly the units the record's rectangles are, and `material` is the ground's own material at
/// that point — the two numbers a face needs, asked of the same two laws the cell's sample asks.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ProfileSample {
    /// Where the ground stands above the disc's floor, in sub-cell steps.
    pub plane: u16,
    /// What the ground is made of there, or `MaterialId::EMPTY` for a point with no measurement.
    pub material: MaterialId,
}

impl ProfileSample {
    /// Whether this point holds a measurement: an unmeasured point is not drawn and not stamped,
    /// the law every rung here shares.
    pub fn measured(&self) -> bool {
        !self.material.is_empty()
    }
}

/// One cell in this many sub-cell steps: [`crate::lod::SURFACE_SUB_CELLS`], the record's own unit.
pub const SUB_CELLS: i32 = crate::lod::SURFACE_SUB_CELLS;

/// **How many voxels the material probe reaches, starting at the sample's own height.**
///
/// The sample's height and the material law's own surface are read half a metre apart — `material_at`
/// samples the voxel's *centre*, and a solid voxel is one whose centre is under its own surface —
/// so the voxel at the sample's height reads as *air above ground* for about **three columns in
/// ten**.
///
/// Measured over every drawn rung's whole disc: the topmost solid voxel is the sample's own for
/// 727–834 of the 1 089 columns and **one below** for the rest, except **one column of one disc**
/// (tier 16) which needed three. Four covers every disc whole — no column of the drawn ladder is
/// left without a material — and a column the probe still finds nothing in is unmeasured: drawn as
/// nothing and stamped as nothing, which is the law every rung here shares.
pub const MATERIAL_PROBES: i32 = 4;

/// **Rungs the frame may adopt from the worker in one call.** A rung is a 139 KB band and a few
/// thousand rectangles; two a frame keeps a burst of deliveries off any single frame's clock.
pub const SURFACE_RUNG_WAVE: usize = 2;

/// **One column's measurement**: the world cell layer its surface falls in, how much of that cell
/// stands above the floor, and what the ground is made of there.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SurfaceColumn {
    /// The world cell layer the surface falls in, in cells from `y = 0`.
    pub layer: i32,
    /// How much of that cell the surface stands above its floor, `0..=1` — the band's own number.
    pub share: f32,
    /// The ground's material at the sample, or `MaterialId::EMPTY` where the generator refused the
    /// point: **an unmeasured column is not drawn and not stamped.**
    pub material: MaterialId,
}

impl Default for SurfaceColumn {
    fn default() -> Self {
        Self {
            layer: 0,
            share: 0.0,
            material: MaterialId::EMPTY,
        }
    }
}

impl SurfaceColumn {
    /// Whether this column holds a measurement at all.
    pub fn measured(&self) -> bool {
        !self.material.is_empty()
    }
}

/// **One rung's disc**: the samples, the mesh and the band cut from them, and where they stand.
#[derive(Clone, PartialEq, Debug)]
pub struct SurfaceRing {
    pub tier: u32,
    pub cell_m: f64,
    /// The disc's low cell, per axis.
    pub origin_cell: [i32; 3],
    /// The eye's own cell — the middle of the disc, and what a crossing is measured against.
    pub eye_cell: [i32; 3],
    /// `33 × 33` columns, in `i + n·j` order.
    pub columns: Vec<SurfaceColumn>,
    /// **The surface at the disc's cell corners** — `(n + 1)²` heights in metres, in `i + (n + 1)·j`
    /// order, where corner `(i, j)` stands at `(origin_cell[0] + i, origin_cell[2] + j) · cell_m`.
    /// The roofs are the bilinear through these four numbers per cell, so two cells that share a
    /// corner share the height at it and meet with no seam. `NaN` where the generator refused.
    pub corner: Vec<f32>,
    /// The annulus's quads, in **sub-cell** units relative to [`SurfaceRing::origin_cell`].
    pub quads: Vec<Quad>,
    /// **The shaper's cell for each quad**, parallel to [`SurfaceRing::quads`]: a roof's
    /// `(x, z, x·z)` gradients, or a wall's `(bottom, top, 0)` along its horizontal axis, in
    /// sub-cells per sub-cell. `[0, 0, 0]` is the flat face the rung drew before the shaper was
    /// carried, and every sub-cuboid the profile's point samples draw is already a step.
    pub shapes: Vec<[f32; 3]>,
    /// The rung's band: `33³` words in `x + 33·(y + 33·z)` order, one cell of the disc each.
    pub words: Vec<u32>,
    /// **Whether this rung also draws the cells inside its own hole** — its **near disc**. Set on
    /// the first rung, and only while the streamed slab stands above the ground the eye is over:
    /// the fine window holds the eye's own column and five chunk layers of it, so a player a
    /// hundred metres up is inside a window made of air, and the ground *within* the window's
    /// footprint is drawn by no rung at all — the chunk-held rings and this ladder both begin
    /// where the window ends. The near disc is that ground, at this rung's own cell, from the
    /// samples the disc already holds.
    pub near: bool,
    /// **The point samples that shape the disc's own cells**, in `u + n·v` order over
    /// `(2·`[`NEAR_SHAPED`]` + 1)²` cells at [`PROFILE_PER_CELL`] samples an axis — the profile32
    /// law asked at each point. Empty until [`SurfaceRing::shape_near`] has measured it, and drawn
    /// only while [`SurfaceRing::near`] is set.
    pub shape: Vec<ProfileSample>,
    /// The side of [`SurfaceRing::shape`], in samples.
    pub shape_side: i32,
    /// How long the samples took, for the fill ledger.
    pub sampled_us: u64,
}

impl SurfaceRing {
    /// The world corner the mesh's coordinates are relative to, in metres.
    pub fn origin_m(&self) -> [f64; 3] {
        [
            self.origin_cell[0] as f64 * self.cell_m,
            self.origin_cell[1] as f64 * self.cell_m,
            self.origin_cell[2] as f64 * self.cell_m,
        ]
    }

    /// **The metres one of this rung's coordinate steps is**: a cell in `SUB_CELLS` steps.
    pub fn step_m(&self) -> f64 {
        self.cell_m / f64::from(SUB_CELLS)
    }

    /// The disc's side in cells.
    pub fn side(&self) -> i32 {
        RING_CELLS
    }

    /// The band's words in one direction.
    pub fn band_side(&self) -> i32 {
        RING_CELLS
    }

    /// The drawn cells: the annulus, and — when [`SurfaceRing::near`] is set on the first rung —
    /// the near disc inside it.
    pub fn drawn(&self, i: i32, j: i32) -> bool {
        let d = (i - RING_MID).abs().max((j - RING_MID).abs());
        if d > RING_MID {
            return false;
        }
        let inner = if self.tier == crate::lod::SURFACE_FIRST_TIER {
            FIRST_INNER
        } else {
            RING_INNER
        };
        d >= inner || (self.near && d <= NEAR_CELLS)
    }

    /// **Mesh or unmesh the near disc**, in place — the cells this rung's own samples already
    /// answer, so the switch is a mesh and never a measurement. Returns whether anything moved,
    /// and does nothing on a rung that has no near disc (every rung but the first, whose hole the
    /// finer rungs would draw twice).
    pub fn set_near(&mut self, near: bool) -> bool {
        if self.tier != crate::lod::SURFACE_FIRST_TIER || self.near == near {
            return false;
        }
        self.near = near;
        (self.quads, self.shapes) = mesh(self);
        true
    }

    /// Whether the disc's own cells have been shaped from point samples.
    pub fn shaped(&self) -> bool {
        !self.shape.is_empty()
    }

    /// **Measure the profile's point samples and shape the disc's own cells.**
    ///
    /// [`NEAR_SHAPED`] cells either side of the eye's own, at [`PROFILE_PER_CELL`] samples an axis,
    /// each one the world's own elevation and material laws at that point — the same two laws the
    /// cell's own sample comes from, asked 32 times finer, which is the whole of "shaped in three
    /// dimensions": the block's interior gradient is measured instead of assumed flat.
    ///
    /// A sample with no measurement is stored as measurementless [`Face`]-less nothing: it is not
    /// drawn and not stamped, exactly as an unmeasured column is not.
    pub fn shape_near(&mut self, spec: &WorldSpec) {
        if self.tier != crate::lod::SURFACE_FIRST_TIER {
            return;
        }
        let cells = NEAR_SHAPED * 2 + 1;
        let n = cells * PROFILE_PER_CELL;
        let first = RING_MID - NEAR_SHAPED;
        let sample_m = self.cell_m / f64::from(PROFILE_PER_CELL);
        let floor_m = self.origin_cell[1] as f64 * self.cell_m;
        let step = self.step_m();
        let started = std::time::Instant::now();
        let mut shape = Vec::with_capacity((n * n) as usize);
        for v in 0..n {
            for u in 0..n {
                let x = (self.origin_cell[0] + first) as f64 * self.cell_m
                    + (f64::from(u) + 0.5) * sample_m;
                let z = (self.origin_cell[2] + first) as f64 * self.cell_m
                    + (f64::from(v) + 0.5) * sample_m;
                let surface_m = terrain::elevation_m(spec, x, z);
                let material =
                    surface_material(spec, x.floor() as i32, z.floor() as i32, surface_m)
                        .unwrap_or(MaterialId::EMPTY);
                let plane = (surface_m - floor_m) / step;
                let plane = if plane.is_finite() && (0.0..=f64::from(u16::MAX)).contains(&plane) {
                    plane.round() as u16
                } else {
                    // Outside the disc's own vertical window: no plane to stand a face on, and no
                    // face is invented for one.
                    u16::MAX
                };
                shape.push(ProfileSample { plane, material });
            }
        }
        self.sampled_us += started.elapsed().as_micros() as u64;
        self.shape = shape;
        self.shape_side = n;
        if self.near {
            (self.quads, self.shapes) = mesh(self);
        }
    }

    /// **Where a column's sample stands, in sub-cells above the disc's floor** — the plane its top
    /// face is drawn on, or `None` when the sample stands outside the disc's own vertical window.
    ///
    /// An unmeasured column has no height, and a sample above or below the window is outside what
    /// this rung measured: 33 cells at the rung's grain, which at every tier above 7 is kilometres
    /// either side of the eye.
    pub fn plane(&self, column: &SurfaceColumn) -> Option<u16> {
        if !column.measured() {
            return None;
        }
        let top = column.layer - self.origin_cell[1];
        if !(0..RING_CELLS).contains(&top) {
            return None;
        }
        let within = (f64::from(column.share) * f64::from(SUB_CELLS)).round() as i32;
        let plane = top * SUB_CELLS + within;
        debug_assert!(
            (0..=RING_CELLS * SUB_CELLS).contains(&plane),
            "plane {plane} off the disc"
        );
        u16::try_from(plane).ok()
    }

    /// **The band word at a disc cell**, in cell coordinates `(i, k, j)` relative to
    /// [`SurfaceRing::origin_cell`].
    pub fn word(&self, i: i32, k: i32, j: i32) -> u32 {
        let n = self.band_side();
        self.words[(i + n * (k + n * j)) as usize]
    }

    /// The column at a disc cell, or `None` outside the disc.
    pub fn column(&self, i: i32, j: i32) -> Option<&SurfaceColumn> {
        let n = self.side();
        if i < 0 || j < 0 || i >= n || j >= n {
            return None;
        }
        Some(&self.columns[(i + n * j) as usize])
    }
}

/// **Measure one rung's disc around the eye.**
///
/// One `elevation_m` a column and one `material_at` at the voxel the surface falls in: nothing here
/// walks a tree, and nothing here is a second reading of the world — these are the generator's own
/// two functions, at the grain the rung draws.
pub fn sample(spec: &WorldSpec, eye_m: [f64; 3], tier: u32) -> SurfaceRing {
    let cell = crate::lod::cell_m(tier);
    let eye_cell = [
        (eye_m[0] / cell).floor() as i32,
        (eye_m[1] / cell).floor() as i32,
        (eye_m[2] / cell).floor() as i32,
    ];
    let origin_cell = [
        eye_cell[0] - RING_MID,
        eye_cell[1] - RING_MID,
        eye_cell[2] - RING_MID,
    ];
    let started = std::time::Instant::now();
    let n = RING_CELLS;
    // **The corner lattice first** — `(n + 1)²` samples, the only heights the roofs read. A cell's
    // column is their mean, so the band and the roof agree at the cell's own middle.
    let mut corner = Vec::with_capacity(((n + 1) * (n + 1)) as usize);
    for j in 0..=n {
        for i in 0..=n {
            let x = (origin_cell[0] + i) as f64 * cell;
            let z = (origin_cell[2] + j) as f64 * cell;
            corner.push(terrain::elevation_m(spec, x, z) as f32);
        }
    }
    let mut columns = Vec::with_capacity((n * n) as usize);
    for j in 0..n {
        for i in 0..n {
            let x = (origin_cell[0] + i) as f64 * cell + cell * 0.5;
            let z = (origin_cell[2] + j) as f64 * cell + cell * 0.5;
            // **The column is the cell's own middle**, as the band and the material have always
            // read it: a corner on the world's rim can stand outside the generator's domain, and a
            // column measured by its middle is still a measured column.
            let surface_m = terrain::elevation_m(spec, x, z);
            let layer = (surface_m / cell).floor() as i32;
            let share = ((surface_m - layer as f64 * cell) / cell).clamp(0.0, 1.0) as f32;
            // **The voxel the surface falls in**, not the cell's middle: a 64 m cell's midpoint is
            // 32 m of air above a 16 m datum, and a census that read it would call every column
            // unmeasured — which is what it did before this line was the surface's own metre. The
            // probe reaches [`MATERIAL_PROBES`] voxels down for the same reason.
            let material = surface_material(spec, x.floor() as i32, z.floor() as i32, surface_m)
                .unwrap_or(MaterialId::EMPTY);
            columns.push(SurfaceColumn {
                layer,
                share,
                material,
            });
        }
    }
    let sampled_us = started.elapsed().as_micros() as u64;
    let ring = SurfaceRing {
        tier,
        cell_m: cell,
        origin_cell,
        eye_cell,
        corner,
        columns,
        quads: Vec::new(),
        shapes: Vec::new(),
        words: Vec::new(),
        near: false,
        shape: Vec::new(),
        shape_side: 0,
        sampled_us,
    };
    let (quads, shapes) = mesh(&ring);
    let words = band(&ring);
    SurfaceRing {
        quads,
        shapes,
        words,
        ..ring
    }
}

/// **The ground's own material at a column**, from the voxel the sample's height falls in — and
/// the two below it, for the one-voxel disagreement [`MATERIAL_PROBES`] states.
///
/// `None` when the generator refused the point, when no voxel in reach holds matter, or when the
/// height is not a number: an unmeasured column is not drawn and not stamped.
pub fn surface_material(spec: &WorldSpec, x: i32, z: i32, surface_m: f64) -> Option<MaterialId> {
    if !surface_m.is_finite() {
        return None;
    }
    let mut y = surface_m.floor() as i32;
    for _ in 0..MATERIAL_PROBES {
        if let Ok(material) = chunk_fields::material_at(spec, x, y, z) {
            if !material.is_empty() {
                return Some(material);
            }
        }
        y -= 1;
    }
    None
}

/// **The rung's surface, from the disc's own samples** — one a cell, and the profile's own point
/// samples where the rung is shaped.
///
/// A cell is drawn when its column holds a measurement, as the **bilinear through the four corner
/// heights it shares with its neighbours**, with no side face at all: neighbouring roofs state the
/// same corner numbers, so there is nothing between them to close. The annulus's inner and outer
/// edges stand open for the same reason they always did — an unmeasured neighbour is not air.
///
/// The near disc's own cells are drawn from [`SurfaceRing::shape`] instead — a top face a profile
/// cell wide at its own sample's height, one wall per boundary where the two sides disagree — and
/// the disc's fringe keeps the cell law. That fringe is a hand-over band, not a second opinion: it
/// is the same territory of ground drawn one grain coarser, and the wall past the shaped block ends
/// on the fringe cell's own **roof**, asked at the sample's point, so the two meet on one number.
/// **A roof's shaper cell, from the other three heights its corners share** — the bilinear anchored
/// at the rectangle's **low corner**, whose own height is the record's `slice` and therefore zero
/// here, in record units per record unit.
///
/// A bilinear has four degrees of freedom. Three ride the record — `shape_lo`, `shape_hi` and the
/// `x·z` twist — and the fourth is the record's own `slice`, which is the low corner's height. So
/// the constant term is zero by construction: at `(0, 0)` the shape adds nothing to `slice`, and at
/// the other three corners the three terms sum to exactly those corners' heights above it. Two
/// cells reading the same corner therefore draw that one integer there, and the edge they share
/// is one line of numbers, not two.
fn roof(c10: f64, c01: f64, c11: f64) -> [f32; 3] {
    let width = f64::from(SUB_CELLS);
    [
        (c10 / width) as f32,
        (c01 / width) as f32,
        ((c11 - c10 - c01) / (width * width)) as f32,
    ]
}

fn mesh(ring: &SurfaceRing) -> (Vec<Quad>, Vec<[f32; 3]>) {
    let n = ring.side();
    let sub = SUB_CELLS as u16;
    let mut quads = Vec::new();
    let mut shapes: Vec<[f32; 3]> = Vec::new();
    // **The corner lattice, in the record's own units** — sub-cells above the disc's low cell. Every
    // roof reads four of these and no other measurement for its height, so the edge two cells share
    // is one line of numbers and cannot split.
    let floor_units = f64::from(ring.origin_cell[1]) * f64::from(SUB_CELLS);
    let corner_units = |i: i32, j: i32| -> Option<f64> {
        let at = (i + (n + 1) * j) as usize;
        let h = *ring.corner.get(at)?;
        h.is_finite()
            .then(|| h as f64 / ring.step_m() - floor_units)
    };
    let height = |i: i32, j: i32| -> Option<u16> {
        let column = ring.column(i, j)?;
        ring.plane(column)
    };
    // **A cell the shape owns is not also drawn from the cell law.** `shaped_at` is the boundary
    // between the two readers of one rung: inside it the profile's samples, outside it one sample
    // a cell.
    let shaped_at = |i: i32, j: i32| -> bool {
        ring.near
            && ring.tier == crate::lod::SURFACE_FIRST_TIER
            && ring.shaped()
            && (i - RING_MID).abs().max((j - RING_MID).abs()) <= NEAR_SHAPED
    };
    for j in 0..n {
        for i in 0..n {
            if !ring.drawn(i, j) || shaped_at(i, j) {
                continue;
            }
            let (Some(plane), Some(here)) = (height(i, j), ring.column(i, j)) else {
                continue;
            };
            // **The cell's four corners are the shared measurement.** The roof is the bilinear
            // through them, anchored at its low corner — so the edge it shares with each neighbour
            // is the same two numbers the neighbour reads, no wall is needed inside a rung, and none
            // is drawn. A cell with an unmeasured corner falls back to its own sample's plane, flat.
            let (slice, shape) = match (
                corner_units(i, j),
                corner_units(i + 1, j),
                corner_units(i, j + 1),
                corner_units(i + 1, j + 1),
            ) {
                (Some(c00), Some(c10), Some(c01), Some(c11)) => {
                    // **Quantized once, at the corner** — the only place two cells can disagree.
                    // Both read the same corner from the same lattice and round it the same way, so
                    // the height each draws there is that one integer, whatever it adds it to.
                    let q00 = c00.round().clamp(0.0, f64::from(u16::MAX));
                    (
                        q00 as u16,
                        roof(
                            c10.round() - q00,
                            c01.round() - q00,
                            c11.round() - q00,
                        ),
                    )
                }
                _ => (plane, [0.0, 0.0, 0.0]),
            };
            quads.push(Quad {
                face: Face::PosY,
                slice,
                uv: [i as u16 * sub, j as u16 * sub],
                extent: [sub, sub],
                material: here.material,
            });
            shapes.push(shape);
        }
    }
    shape_quads(ring, &mut quads, &mut shapes);
    debug_assert_eq!(quads.len(), shapes.len(), "one shaper record a rectangle");
    (quads, shapes)
}

/// **The disc's own cells, drawn from the profile's point samples.**
///
/// Every sample is a cuboid at the rung's record grain: a top face a profile cell wide at its own
/// sample's own height, and a wall at each boundary where the two sides disagree, from the lower
/// height to the higher. The neighbours inside the shaped block are its own samples; past its edge
/// they are the fringe cell's **roof at that point**, read from the same record `mesh` wrote, so a
/// wall that ends on a fringe cell ends on the surface that cell draws — a fringe cell is a roof,
/// not a cuboid, and it has no side of its own to close the step. Past the disc nothing is drawn.
fn shape_quads(ring: &SurfaceRing, quads: &mut Vec<Quad>, shapes: &mut Vec<[f32; 3]>) {
    let n = ring.shape_side;
    if n == 0 || !ring.near || !ring.shaped() {
        return;
    }
    let first = RING_MID - NEAR_SHAPED;
    let sample = |u: i32, v: i32| -> Option<ProfileSample> {
        if !(0..n).contains(&u) || !(0..n).contains(&v) {
            return None;
        }
        let at = ring.shape[(u + n * v) as usize];
        at.measured().then_some(at)
    };
    // **The fringe cells' own roofs**, the records `mesh` has already pushed: a cell's plane and
    // its shaper, keyed by the cell the rectangle stands in.
    let sub = SUB_CELLS as u16;
    let mut roofs = std::collections::HashMap::new();
    for (quad, shape) in quads.iter().zip(shapes.iter()) {
        if quad.face == Face::PosY && quad.extent == [sub, sub] {
            let key = (
                quad.uv[0] as i32 / sub as i32,
                quad.uv[1] as i32 / sub as i32,
            );
            roofs.insert(key, (quad.slice, *shape));
        }
    }
    // **What stands beside a sample of the block at its edge**: the fringe cell's own roof, asked
    // at the sample's point — the same bilinear its record states, so the wall and the roof meet on
    // one number there.
    let fringe = |u: i32, v: i32| -> Option<u16> {
        let i = first + u.div_euclid(PROFILE_PER_CELL);
        let j = first + v.div_euclid(PROFILE_PER_CELL);
        let (slice, s) = *roofs.get(&(i, j))?;
        let dx = f64::from(u.rem_euclid(PROFILE_PER_CELL) * PROFILE_STEPS);
        let dz = f64::from(v.rem_euclid(PROFILE_PER_CELL) * PROFILE_STEPS);
        let h = f64::from(slice)
            + f64::from(s[0]) * dx
            + f64::from(s[1]) * dz
            + f64::from(s[2]) * dx * dz;
        (0.0..=f64::from(u16::MAX))
            .contains(&h)
            .then(|| h.round() as u16)
    };
    // The neighbour's height, and whether it is a fringe cell's **roof** rather than a sample's
    // top face: a sample below this one hides its half behind its own top; a roof has no side at
    // all, so the step past the block is this block's wall to close.
    let neighbour = |u: i32, v: i32| -> Option<(u16, bool)> {
        match sample(u, v) {
            Some(that) => Some((that.plane, false)),
            None if (0..n).contains(&u) && (0..n).contains(&v) => None,
            None => fringe(u, v).map(|that| (that, true)),
        }
    };
    let step = PROFILE_STEPS as u16;
    for v in 0..n {
        for u in 0..n {
            let Some(here) = sample(u, v) else {
                continue;
            };
            let i = first + u.div_euclid(PROFILE_PER_CELL);
            let j = first + v.div_euclid(PROFILE_PER_CELL);
            let ux = (i * SUB_CELLS + u.rem_euclid(PROFILE_PER_CELL) * PROFILE_STEPS) as u16;
            let uz = (j * SUB_CELLS + v.rem_euclid(PROFILE_PER_CELL) * PROFILE_STEPS) as u16;
            quads.push(Quad {
                face: Face::PosY,
                slice: here.plane,
                uv: [ux, uz],
                extent: [step, step],
                material: here.material,
            });
            shapes.push([0.0, 0.0, 0.0]);
            for (du, dv, face) in [
                (-1, 0, Face::NegX),
                (1, 0, Face::PosX),
                (0, -1, Face::NegZ),
                (0, 1, Face::PosZ),
            ] {
                let Some((that, from_roof)) = neighbour(u + du, v + dv) else {
                    continue;
                };
                if that == here.plane || (that > here.plane && !from_roof) {
                    continue;
                }
                // **From the lower of the two to the higher**, whichever side that is: inside the
                // block the neighbour's own top face hides its half, and at the fringe there is no
                // other face, so the step itself is the wall.
                let (low, span) = (that.min(here.plane), that.abs_diff(here.plane));
                let (slice, uv, extent) = match face.axis() {
                    0 => (
                        if du > 0 { ux + step } else { ux },
                        [low, uz],
                        [span, step],
                    ),
                    _ => (
                        if dv > 0 { uz + step } else { uz },
                        [ux, low],
                        [step, span],
                    ),
                };
                quads.push(Quad {
                    face,
                    slice,
                    uv,
                    extent,
                    material: here.material,
                });
                shapes.push([0.0, 0.0, 0.0]);
            }
        }
    }
}

/// **The rung's band, from the disc's own samples.**
///
/// One word a cell: [`crate::horizon::FAR_MEASURED`] with the share of the cell that holds matter
/// — the same word the coarse rungs carry, read by the same walk (`world_band_share`), so a rung
/// above 7 is an offset in the ladder's one buffer rather than a second kind of band.
///
/// The share is the column's own: the cell the sample falls in holds `share` of it, the cells below
/// are full, and the cells above are measured air. A column with no measurement writes **no word at
/// all** — every one of its cells stays unmeasured, which is not air.
fn band(ring: &SurfaceRing) -> Vec<u32> {
    let n = ring.band_side();
    let mut words = vec![0u32; (n * n * n) as usize];
    for j in 0..n {
        for i in 0..n {
            let column = &ring.columns[(i + n * j) as usize];
            if !column.measured() {
                continue;
            }
            // The sample's own cell, in the disc's coordinates. A sample outside the disc's window
            // is clamped rather than dropped: the whole disc's column is inside the terrain when
            // the surface stands above it, and above the terrain when it stands below.
            let top = (column.layer - ring.origin_cell[1]).clamp(0, n);
            let share = (f64::from(column.share) * f64::from(crate::horizon::FAR_SHARE as u32))
                .round() as u32;
            for k in 0..n {
                let word = if k < top {
                    crate::horizon::FAR_MEASURED | crate::horizon::FAR_SHARE
                } else if k == top {
                    crate::horizon::FAR_MEASURED | share.min(crate::horizon::FAR_SHARE)
                } else {
                    crate::horizon::FAR_MEASURED
                };
                words[(i + n * (k + n * j)) as usize] = word;
            }
        }
    }
    words
}

/// One rung's worth of work for the ladder's thread: measure these tiers, nearest first, around
/// this eye — and, when the caller is drawing the near disc, measure the first tier's profile
/// samples too, so the ground under a high eye arrives shaped rather than flat.
struct Job {
    eye_m: [f64; 3],
    tiers: Vec<u32>,
    near: bool,
}

/// **The surface ladder: every rung above a chunk, measured on its own thread.**
///
/// The rungs are held by tier, so a caller reads what is drawn and a crossing replaces one rung
/// rather than the set. `asked` is what the worker was last told to measure: a frame that wants
/// something else sends a new job, and the worker abandons the rung it was on when it sees it — so
/// the ladder always follows the eye rather than a queue of stale rings.
pub struct SurfaceLadder {
    spec: WorldSpec,
    rings: BTreeMap<u32, SurfaceRing>,
    /// **Whether the first rung carries its near disc** — the caller's answer to "does the
    /// streamed slab stand above the ground?", applied to the rung in hand and to every rung the
    /// worker delivers while it is set. See [`SurfaceRing::near`].
    near: bool,
    /// What the worker was last asked for, and around which eye.
    asked: Vec<u32>,
    asked_eye: Option<[f64; 3]>,
    /// The tiers the last [`SurfaceLadder::advance`] adopted, nearest first.
    adopted: Vec<u32>,
    to_worker: Option<Sender<Job>>,
    from_worker: Option<Receiver<SurfaceRing>>,
    thread: Option<std::thread::JoinHandle<()>>,
    measured: u64,
}

impl SurfaceLadder {
    /// **A ladder that measures its rungs on its own thread**, from the world's own spec.
    pub fn new(spec: WorldSpec) -> Self {
        let (to_worker, jobs) = std::sync::mpsc::channel::<Job>();
        let (done, from_worker) = std::sync::mpsc::channel::<SurfaceRing>();
        let worker_spec = spec.clone();
        let thread = std::thread::Builder::new()
            .name("surface-ladder".into())
            .spawn(move || {
                let mut job: Option<Job> = None;
                loop {
                    if job.is_none() {
                        match jobs.recv() {
                            Ok(next) => job = Some(next),
                            Err(_) => return,
                        }
                    }
                    // **A new job supersedes the one in flight at the next rung boundary.** The
                    // sample of a rung is a few milliseconds; the check is between rungs, so a
                    // rung that is being measured is measured out and sent rather than abandoned
                    // half-sampled.
                    let current = job.take().expect("a job");
                    let mut superseded = None;
                    for tier in current.tiers.iter().copied() {
                        match jobs.try_recv() {
                            Ok(newer) => {
                                superseded = Some(newer);
                                break;
                            }
                            Err(_) => {}
                        }
                        let mut ring = sample(&worker_spec, current.eye_m, tier);
                        // **The shaping is the worker's work**, like the sampling it belongs to: it is
                        // 32² points a cell, which is why it is not the frame's.
                        if current.near && tier == crate::lod::SURFACE_FIRST_TIER {
                            ring.set_near(true);
                            ring.shape_near(&worker_spec);
                        }
                        if done.send(ring).is_err() {
                            return;
                        }
                    }
                    job = superseded;
                }
            })
            .expect("a thread for the surface ladder");
        Self {
            spec,
            rings: BTreeMap::new(),
            near: false,
            asked: Vec::new(),
            asked_eye: None,
            adopted: Vec::new(),
            to_worker: Some(to_worker),
            from_worker: Some(from_worker),
            thread: Some(thread),
            measured: 0,
        }
    }

    /// **Whether the first rung draws its near disc**, and a switch for it.
    ///
    /// The caller owns the question — it is about the *streamed* window, which the ladder does not
    /// see. Turning it on **meshes the ring in hand from the samples it already holds**, in this
    /// same frame (a handful of cell tops: the ground under the player is covered before the frame
    /// ends), and the worker then replaces that ring with the **shaped** one — [`NEAR_SHAPED`] cells
    /// of point samples at [`PROFILE_PER_CELL`] an axis, which is not the frame's work. Returns
    /// whether anything moved on this frame.
    pub fn set_near_disc(&mut self, near: bool) -> bool {
        if self.near == near {
            return false;
        }
        self.near = near;
        let mut moved = false;
        for ring in self.rings.values_mut() {
            moved |= ring.set_near(near);
        }
        moved
    }

    /// Whether the first rung is drawing its near disc.
    pub fn near_disc(&self) -> bool {
        self.near
    }

    /// **The tiers stale at this eye**, nearest first: every drawn tier whose ring is missing or
    /// was measured around a different cell — a crossing of a rung's own cell is what moves it —
    /// **and the first tier while its disc is asked for and its shape is not measured**: a ring
    /// already in hand draws the flat disc from that very frame, and the worker replaces it with
    /// the shaped one rather than the ground waiting on the profile's 32² points a cell.
    pub fn wanted(&self, eye_m: [f64; 3]) -> Vec<u32> {
        crate::lod::surface_tiers()
            .into_iter()
            .filter(|tier| {
                let cell = crate::lod::cell_m(*tier);
                let eye_cell = [
                    (eye_m[0] / cell).floor() as i32,
                    (eye_m[1] / cell).floor() as i32,
                    (eye_m[2] / cell).floor() as i32,
                ];
                self.rings.get(tier).is_none_or(|ring| {
                    ring.eye_cell != eye_cell
                        || (self.near && *tier == crate::lod::SURFACE_FIRST_TIER && !ring.shaped())
                })
            })
            .collect()
    }

    /// **Ask for what the eye wants, then take what the worker has finished.**
    ///
    /// Returns the number of rungs adopted; [`SurfaceLadder::adopted`] names them, nearest first,
    /// so a caller stamps each rung's band on the frame that draws it.
    pub fn advance(&mut self, eye_m: [f64; 3], budget: usize) -> usize {
        self.adopted.clear();
        let mut taken = 0usize;
        if let Some(from_worker) = self.from_worker.as_ref() {
            while taken < budget {
                match from_worker.try_recv() {
                    Ok(mut ring) => {
                        // **A delivery the ladder no longer wants is dropped, not drawn twice.** A
                        // job supersedes the one in flight at a rung boundary, so the rung the
                        // worker was already on arrives once for each job that named it; the ring
                        // in hand is the one the disc's own eye cell was measured around.
                        if self.rings.get(&ring.tier).is_some_and(|have| {
                            have.eye_cell == ring.eye_cell && have.shaped() >= ring.shaped()
                        }) {
                            continue;
                        }
                        self.measured += 1;
                        self.adopted.push(ring.tier);
                        // **A rung delivered while the disc is on arrives with it meshed**: the
                        // caller's answer has not changed just because the worker is a rung
                        // behind, and a first rung without the disc would be the hole back for
                        // the frames it takes the ladder to re-anchor.
                        ring.set_near(self.near);
                        self.rings.insert(ring.tier, ring);
                        taken += 1;
                    }
                    Err(_) => break,
                }
            }
        }
        let wanted = self.wanted(eye_m);
        // **The worker is told once per list**, and only when the list or the eye it is about has
        // moved: a frame that repeats the same ask would otherwise queue the same job sixty times a
        // second, and a rung the worker still has queued is not a reason to restart its list.
        // A ladder filling nearest first asks for `8..` and then for `9..`, which is a suffix of
        // what it has already asked for; a crossing asks for the fine tiers again, which is not.
        let same_eye = self.asked_eye == Some(eye_m);
        let still_queued = self.asked.len() >= wanted.len()
            && self.asked[self.asked.len() - wanted.len()..] == wanted[..];
        if !wanted.is_empty() && !(same_eye && still_queued) {
            if let Some(to_worker) = self.to_worker.as_ref() {
                let _ = to_worker.send(Job {
                    eye_m,
                    tiers: wanted.clone(),
                    near: self.near,
                });
            }
            self.asked = wanted;
            self.asked_eye = Some(eye_m);
        }
        taken
    }

    /// The tiers the last [`SurfaceLadder::advance`] adopted, nearest first.
    pub fn adopted(&self) -> &[u32] {
        &self.adopted
    }

    /// Every rung in hand, nearest first.
    pub fn rings(&self) -> impl Iterator<Item = &SurfaceRing> {
        self.rings.values()
    }

    /// How many rungs are in hand.
    pub fn len(&self) -> usize {
        self.rings.len()
    }

    /// Whether no rung has been measured yet.
    pub fn is_empty(&self) -> bool {
        self.rings.is_empty()
    }

    /// How many rungs the ladder draws.
    pub fn total(&self) -> usize {
        crate::lod::SURFACE_RUNGS
    }

    /// The rung at a tier, if it is in hand.
    pub fn ring(&self, tier: u32) -> Option<&SurfaceRing> {
        self.rings.get(&tier)
    }

    /// The rectangles every rung in hand is drawn as.
    pub fn quads(&self) -> usize {
        self.rings.values().map(|ring| ring.quads.len()).sum()
    }

    /// **The sampling time of every rung in hand**, microseconds — the fill a cold start spends on
    /// the generator's own surface law, summed over the rungs currently drawn.
    pub fn sampled_us(&self) -> u64 {
        self.rings.values().map(|ring| ring.sampled_us).sum()
    }

    /// How many rungs the worker has delivered since the ladder was built.
    pub fn measured(&self) -> u64 {
        self.measured
    }
}

impl Drop for SurfaceLadder {
    fn drop(&mut self) {
        // Dropping the sender ends the worker's `recv`, so the thread ends with the ladder.
        self.to_worker.take();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_shapers::{Config, WorldSpec};

    fn spec() -> WorldSpec {
        WorldSpec::new(crate::game::GAME_SEED, Config::default())
    }

    /// The shipped eye: the spawn the game uses, and the tick the harness measures at.
    fn eye() -> [f64; 3] {
        crate::game::spawn_eye(&crate::game::World::shipped())
    }

    /// **Two cells of one rung meet at the height they both measure at the corner they share.**
    ///
    /// The roof is the bilinear through the cell's four corners, so its value on a shared edge is a
    /// function of the two corner heights alone — the same two numbers the neighbour reads. This
    /// walks every adjacent pair the annulus draws and checks both roofs on their shared edge,
    /// which is the seam the cell-plane law left open.
    #[test]
    fn neighbouring_roofs_meet_on_the_corner_they_share() {
        let spec = spec();
        let ring = sample(&spec, eye(), crate::lod::SURFACE_FIRST_TIER);
        let sub = SUB_CELLS as u16;
        let width = f64::from(SUB_CELLS);
        let mut roofs = std::collections::HashMap::new();
        for (quad, shape) in ring.quads.iter().zip(ring.shapes.iter()) {
            if quad.face != Face::PosY || quad.extent != [sub, sub] {
                continue;
            }
            let i = quad.uv[0] as i32 / sub as i32;
            let j = quad.uv[1] as i32 / sub as i32;
            roofs.insert((i, j), (quad.slice, *shape));
        }
        // The record's own law: the bilinear from the rectangle's low corner, on `slice`.
        let height = |(slice, s): (u16, [f32; 3]), dx: f64, dz: f64| {
            f64::from(slice)
                + f64::from(s[0]) * dx
                + f64::from(s[1]) * dz
                + f64::from(s[2]) * dx * dz
        };
        let n = ring.side();
        let corner = |i: i32, j: i32| ring.corner[(i + (n + 1) * j) as usize];
        let mut pairs = 0;
        for (&(i, j), &a) in roofs.iter() {
            for (di, dj) in [(1, 0), (0, 1)] {
                let Some(&b) = roofs.get(&(i + di, j + dj)) else {
                    continue;
                };
                // The disk's rim can stand outside the generator's domain; a cell there falls back
                // to its own middle, and there is no shared corner to agree on.
                let (ei, ej) = (i + i32::from(di != 0), j + i32::from(dj != 0));
                if !corner(ei, ej).is_finite() || !corner(i, j).is_finite() {
                    continue;
                }
                for t in [0.0, width * 0.5, width] {
                    // Each cell reads its own low-corner coordinates, which meet on the edge.
                    let (ax, az) = if di != 0 { (width, t) } else { (t, width) };
                    let (bx, bz) = if di != 0 { (0.0, t) } else { (t, 0.0) };
                    let (ha, hb) = (height(a, ax, az), height(b, bx, bz));
                    assert!(
                        (ha - hb).abs() <= 1e-3,
                        "a seam at ({i}, {j}) and ({}, {}): {ha} != {hb}",
                        i + di,
                        j + dj
                    );
                }
                pairs += 1;
            }
        }
        assert!(pairs > 100, "too few neighbours to be a check: {pairs}");
    }

    /// **Every drawn face stands in a measured cell of its own rung, and its top is the sample's
    /// own height** — the one number both readers read.
    #[test]
    fn a_rungs_faces_come_from_its_own_samples() {
        let spec = spec();
        let ring = sample(&spec, eye(), crate::lod::SURFACE_FIRST_TIER);
        assert_eq!(ring.side(), RING_CELLS);
        assert!(
            !ring.quads.is_empty(),
            "the first surface rung drew nothing"
        );
        let n = ring.side();
        let sub = SUB_CELLS as u16;
        for (index, quad) in ring.quads.iter().enumerate() {
            assert!(
                !quad.material.is_empty(),
                "a face was drawn without a measurement"
            );
            // **The cell behind the face**: a top face is its own cell's, and a wall at a boundary
            // belongs to the cell whose own side it is — the one on the wall's inward side.
            let (i, j) = match quad.face.axis() {
                1 => (
                    quad.uv[0] as i32 / sub as i32,
                    quad.uv[1] as i32 / sub as i32,
                ),
                0 => (
                    quad.slice as i32 - i32::from(quad.face == Face::PosX),
                    quad.uv[1] as i32 / sub as i32,
                ),
                _ => (
                    quad.uv[0] as i32 / sub as i32,
                    quad.slice as i32 - i32::from(quad.face == Face::PosZ),
                ),
            };
            assert!(
                (0..n).contains(&i) && (0..n).contains(&j),
                "a face stands off the rung"
            );
            assert!(
                ring.drawn(i, j),
                "a face was drawn outside the annulus at ({i}, {j})"
            );
            let here = ring.column(i, j).expect("a column of the disc");
            assert!(here.measured(), "a face has no column behind it");
            if quad.face == Face::PosY && quad.extent == [sub, sub] {
                // **The roof's `slice` is its low corner's height**, the corner two neighbours
                // share, and its shaper reproduces the other three — so the record is the bilinear
                // through the corners rather than a plane through the cell's own middle.
                let n = ring.side();
                let corner = |i: i32, j: i32| ring.corner[(i + (n + 1) * j) as usize];
                let (c00, c10, c01, c11) = (
                    corner(i, j),
                    corner(i + 1, j),
                    corner(i, j + 1),
                    corner(i + 1, j + 1),
                );
                if [c00, c10, c01, c11].iter().all(|c| c.is_finite()) {
                    let floor = ring.origin_cell[1] as f64 * f64::from(SUB_CELLS);
                    // The heights the record states are **quantized once, at the corner**.
                    let q = |c: f32| (f64::from(c) / ring.step_m() - floor).round();
                    assert_eq!(
                        f64::from(quad.slice),
                        q(c00),
                        "a top face is not on its own low corner's height"
                    );
                    let shape = ring.shapes[index];
                    let width = f64::from(SUB_CELLS);
                    for (dx, dz, c) in [
                        (0.0, 0.0, c00),
                        (width, 0.0, c10),
                        (0.0, width, c01),
                        (width, width, c11),
                    ] {
                        let drawn = f64::from(quad.slice)
                            + f64::from(shape[0]) * dx
                            + f64::from(shape[1]) * dz
                            + f64::from(shape[2]) * dx * dz;
                        assert!(
                            (drawn - q(c)).abs() <= 1e-3,
                            "the roof misses the corner it shares at ({dx}, {dz})"
                        );
                    }
                }
            }
        }
    }

    /// **The band and the mesh are the same measurement**: the cell a top face stands in carries
    /// the sample's own share, the cells below it are full, and the cells above are measured air.
    #[test]
    fn a_rungs_band_is_its_own_samples() {
        let spec = spec();
        let ring = sample(&spec, eye(), 12);
        assert_eq!(
            ring.words.len(),
            (RING_CELLS * RING_CELLS * RING_CELLS) as usize
        );
        assert!(ring.quads.len() > 0, "tier 12 drew nothing");
        let mut stamped = 0usize;
        for j in 0..RING_CELLS {
            for i in 0..RING_CELLS {
                let column = ring.column(i, j).expect("a column");
                if !column.measured() {
                    continue;
                }
                let top = (column.layer - ring.origin_cell[1]).clamp(0, RING_CELLS);
                for k in 0..RING_CELLS {
                    let word = ring.word(i, k, j);
                    assert_ne!(
                        word & crate::horizon::FAR_MEASURED,
                        0,
                        "a measured column left a cell unmeasured at ({i}, {k}, {j})"
                    );
                    stamped += 1;
                    if k < top {
                        assert_eq!(word & crate::horizon::FAR_SHARE, crate::horizon::FAR_SHARE);
                    } else if k == top {
                        let share = (f64::from(column.share)
                            * f64::from(crate::horizon::FAR_SHARE as u32))
                        .round() as u32;
                        assert_eq!(word & crate::horizon::FAR_SHARE, share);
                    } else {
                        assert_eq!(word & crate::horizon::FAR_SHARE, 0, "air above the surface");
                    }
                }
                // **A top face's cell is a cell the light knows is matter**: its own share, or the
                // measured-empty above it. A face lit by a cell the band calls clear is the
                // disagreement this whole module exists to prevent.
                let plane = ring.plane(column).expect("a drawn column has a plane");
                let face_cell = (plane as i32 / SUB_CELLS).min(RING_CELLS - 1);
                assert_ne!(
                    ring.word(i, face_cell, j) & crate::horizon::FAR_MEASURED,
                    0,
                    "the face's own cell is unmeasured"
                );
            }
        }
        assert!(
            stamped > (RING_CELLS * RING_CELLS) as usize,
            "only {stamped} cells were stamped, so the band is not a disc of columns"
        );
    }

    /// **The disc's window holds every sample of every rung**, so no rung silently drops a column:
    /// the window is 33 cells at the rung's own grain, and the world's relief is 40 m.
    #[test]
    fn every_rungs_samples_stand_inside_its_disc() {
        let spec = spec();
        let eye = eye();
        let mut columns = 0usize;
        for tier in crate::lod::surface_tiers() {
            let ring = sample(&spec, eye, tier);
            for j in 0..RING_CELLS {
                for i in 0..RING_CELLS {
                    let column = ring.column(i, j).expect("a column");
                    assert!(
                        column.measured(),
                        "tier {tier} column ({i}, {j}) has no measurement"
                    );
                    assert!(
                        ring.plane(column).is_some(),
                        "tier {tier} column ({i}, {j}) stands outside the disc's window"
                    );
                    columns += 1;
                }
            }
            assert_eq!(
                ring.words.len(),
                (RING_CELLS * RING_CELLS * RING_CELLS) as usize
            );
        }
        assert_eq!(columns, crate::lod::SURFACE_RUNGS * 33 * 33);
    }

    /// **A rung's cell is its tier's cell, and nothing else** — the mismatch the band and the
    /// record would otherwise be cut at.
    #[test]
    fn a_rungs_cell_is_its_tiers_cell() {
        let spec = spec();
        for tier in crate::lod::surface_tiers() {
            let ring = sample(&spec, eye(), tier);
            assert_eq!(ring.cell_m, crate::lod::cell_m(tier));
            assert_eq!(ring.step_m(), ring.cell_m / 256.0);
        }
    }

    /// **The rungs tile: a rung's inner edge is the finer rung's outer edge**, from tier 7's own
    /// drawn edge to the top of the ladder.
    #[test]
    fn a_rung_begins_where_the_finer_one_ends() {
        let tiers = crate::lod::surface_tiers();
        assert_eq!(tiers.first(), Some(&crate::lod::SURFACE_FIRST_TIER));
        assert_eq!(tiers.last(), Some(&crate::lod::SURFACE_LAST_DRAWN_TIER));
        assert_eq!(
            crate::lod::surface_inner_m(crate::lod::SURFACE_FIRST_TIER),
            (crate::game::GAME_WINDOW.radius_chunks + 7) as f64 * 32.0,
            "the first rung does not begin where tier 7's drawn ring ends"
        );
        for pair in tiers.windows(2) {
            let (fine, coarse) = (pair[0], pair[1]);
            let coarse_inner = crate::lod::cell_m(coarse) * f64::from(RING_INNER);
            let fine_outer = crate::lod::cell_m(fine) * f64::from(RING_MID);
            assert_eq!(
                coarse_inner, fine_outer,
                "tier {coarse}'s inner edge is not tier {fine}'s outer edge"
            );
        }
    }

    /// **The drawn top is the world's own address**, and the tiers above it are stated as address:
    /// the last drawn rung's disc fits in `i32` voxels, the next tier's own outer edge is exactly
    /// `2³¹` m — the address space's edge — and the top of the ladder is 536 871 km a cell.
    #[test]
    fn the_drawn_top_is_where_the_worlds_own_address_stops() {
        let last = crate::lod::SURFACE_LAST_DRAWN_TIER;
        let outer = crate::lod::surface_outer_m(last);
        assert_eq!(
            last, 28,
            "the drawn top moved without this test's own numbers"
        );
        assert_eq!(crate::lod::cell_m(last), 67_108_864.0);
        assert_eq!(outer, 1_073_741_824.0, "16 × 67 108 864 m");
        // **The next tier's disc is exactly the address space**, which is why it is not drawn: the
        // generator's material law names voxels in `i32`, and a rung whose disc reaches `2³¹` m has
        // no voxel it could ask.
        let next = crate::lod::surface_outer_m(last + 1);
        assert_eq!(
            next, 2_147_483_648.0,
            "tier 29's inner disc is exactly 2³¹ m"
        );
        assert!(
            crate::lod::surface_outer_m(last) < 2_147_483_648.0,
            "the drawn top's own disc is inside the address space"
        );
        // **And the ladder's own top is stated**: tier 31's cell and the radius it would draw at.
        assert_eq!(
            crate::lod::cell_m(crate::lod::TIERS - 1),
            crate::lod::TOP_CELL_M
        );
        assert_eq!(crate::lod::surface_outer_m(31), 8_589_934_592.0);
        assert!(
            crate::lod::surface_outer_m(31) > 384_400_000.0,
            "the ladder's own top reaches past the Moon's orbit"
        );
        assert!(
            crate::lod::LADDER_FAR_M == 2_147_483_648.0,
            "the plane the drawn ladder needs is the address space itself"
        );
        assert!(
            crate::camera::FAR_M >= outer,
            "the drawn ladder reaches {outer:.0} m and the far plane is {:.0} m",
            crate::camera::FAR_M
        );
    }

    /// **The ladder fills nearest first, and a rung is a whole rung.** The first call adopts fewer
    /// rungs than the ladder has; the rungs arrive in tier order; each one's band is as wide as its
    /// disc and its cell is its own.
    #[test]
    fn the_ladder_fills_nearest_first_and_a_rung_at_a_time() {
        let mut ladder = SurfaceLadder::new(spec());
        let eye = eye();
        let mut order: Vec<u32> = Vec::new();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
        let mut first_adoption: Option<usize> = None;
        while ladder.len() < ladder.total() && std::time::Instant::now() < deadline {
            // **One rung a call at most**, which is what makes the first adoption a *partial* one:
            // the frame that draws the first rung is a frame that has not waited for the ladder.
            ladder.advance(eye, 1);
            if !ladder.adopted().is_empty() && first_adoption.is_none() {
                first_adoption = Some(ladder.len());
            }
            order.extend_from_slice(ladder.adopted());
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(
            ladder.len(),
            ladder.total(),
            "the ladder did not finish filling"
        );
        assert!(
            first_adoption == Some(1),
            "the first adoption was {first_adoption:?} rungs, not one"
        );
        let mut sorted = order.clone();
        sorted.sort_unstable();
        assert_eq!(order, sorted, "the rungs did not arrive nearest first");
        assert_eq!(order.len(), ladder.total());
        for ring in ladder.rings() {
            assert_eq!(
                ring.words.len(),
                (RING_CELLS * RING_CELLS * RING_CELLS) as usize
            );
            assert_eq!(ring.cell_m, crate::lod::cell_m(ring.tier));
            assert!(!ring.quads.is_empty(), "tier {} drew nothing", ring.tier);
            assert!(ring.sampled_us > 0, "tier {} was not sampled", ring.tier);
        }
        println!(
            "the ladder: {} rungs, {} quads, {:.1} ms of samples",
            ladder.len(),
            ladder.quads(),
            ladder.sampled_us() as f64 / 1000.0,
        );
    }

    /// **A crossing re-measures the rungs it moves, and only those.** Walking one tier-8 cell
    /// leaves every rung above 9 where it was.
    #[test]
    fn a_crossing_only_moves_the_rungs_it_crosses() {
        let mut ladder = SurfaceLadder::new(spec());
        let eye = eye();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
        while ladder.len() < ladder.total() && std::time::Instant::now() < deadline {
            ladder.advance(eye, SURFACE_RUNG_WAVE);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(
            ladder.len(),
            ladder.total(),
            "the ladder did not finish filling"
        );
        assert!(
            ladder.wanted(eye).is_empty(),
            "a settled ladder still wants rungs"
        );
        let moved = [eye[0] + 64.0, eye[1], eye[2]];
        let wanted = ladder.wanted(moved);
        assert_eq!(
            wanted.first(),
            Some(&crate::lod::SURFACE_FIRST_TIER),
            "a 64 m walk does not re-measure the 64 m rung"
        );
        assert!(
            !wanted.contains(&(crate::lod::SURFACE_FIRST_TIER + 2)),
            "a 64 m walk re-measured a rung four cells wide"
        );
    }

    /// **The near disc is the ground the streamed window holds, and it is off unless asked for.**
    ///
    /// The defect it answers is the user's own screenshot: `605 / 605 chunks resident, 9 quads
    /// drawn` at an eye 280 m over the ground. The shipped slab is five chunk *layers* around the
    /// eye, so a player that high stands in a window made of air, and every rung that could draw
    /// the ground *within* the window's footprint begins where the window ends — the chunk-held
    /// rings at `radius + 1` chunks and this ladder at 384 m. So:
    ///
    /// 1. unasked, **no** cell inside the annulus's hole is drawn, and asked, every cell within
    ///    [`NEAR_CELLS`] of the eye's own is;
    /// 2. every chunk the shipped window holds stands inside a drawn near cell, so the slab's
    ///    ground has a rung to hand over to wherever the window's own column cannot reach it.
    #[test]
    fn the_near_disc_is_the_windows_own_ground() {
        let spec = spec();
        let eye = eye();
        let mut ring = sample(&spec, eye, crate::lod::SURFACE_FIRST_TIER);
        let hole = |i: i32, j: i32| (i - RING_MID).abs().max((j - RING_MID).abs()) < FIRST_INNER;
        for j in 0..RING_CELLS {
            for i in 0..RING_CELLS {
                if hole(i, j) {
                    assert!(!ring.drawn(i, j), "the disc was drawn without being asked");
                } else {
                    assert!(ring.drawn(i, j), "the annulus lost a cell");
                }
            }
        }
        let annulus = ring.quads.len();
        assert!(ring.set_near(true), "the switch moved nothing");
        assert!(ring.quads.len() > annulus, "the disc drew no cells");
        for j in 0..RING_CELLS {
            for i in 0..RING_CELLS {
                let d = (i - RING_MID).abs().max((j - RING_MID).abs());
                if hole(i, j) {
                    assert_eq!(ring.drawn(i, j), d <= NEAR_CELLS, "the disc's own edge");
                }
            }
        }
        // **Every chunk of the shipped window stands in a drawn near cell.** The window's chunks
        // are `radius` either side of the eye's own, and a drawn cell is two chunks a side; the
        // cell a chunk stands in is the one its own centre falls in.
        let side = f64::from(voxel_model::CHUNK_SIDE);
        let radius = crate::game::GAME_WINDOW.radius_chunks;
        let eye_chunk = [
            (eye[0] / side).floor() as i32,
            (eye[1] / side).floor() as i32,
            (eye[2] / side).floor() as i32,
        ];
        for dz in -radius..=radius {
            for dx in -radius..=radius {
                let x = (eye_chunk[0] + dx) as f64 * side + side * 0.5;
                let z = (eye_chunk[2] + dz) as f64 * side + side * 0.5;
                let i = (x / ring.cell_m).floor() as i32 - ring.origin_cell[0];
                let j = (z / ring.cell_m).floor() as i32 - ring.origin_cell[2];
                assert!(
                    (0..RING_CELLS).contains(&i) && (0..RING_CELLS).contains(&j),
                    "a window chunk fell outside the disc"
                );
                assert!(
                    ring.drawn(i, j),
                    "chunk ({dx}, {dz}) of the window is in no drawn cell"
                );
            }
        }
        // And the switch is a mesh and nothing else: unasked again, the ring is the annulus it was.
        assert!(ring.set_near(false), "the switch moved nothing");
        assert_eq!(
            ring.quads,
            sample(&spec, eye, crate::lod::SURFACE_FIRST_TIER).quads
        );
    }

    /// **The disc's own cells are shaped by the profile's point samples** — [`PROFILE_PER_CELL`] an
    /// axis, each one the world's own elevation and material at that point, drawn as sub-cuboids at
    /// the rung's record grain: a top face a profile cell wide at its own sample's height, and one
    /// wall per lower neighbour. Nothing is assumed flat inside a shaped cell, and nothing is
    /// invented where a point has no measurement.
    #[test]
    fn the_near_discs_own_cells_are_shaped_from_profile_samples() {
        let spec = spec();
        let eye = eye();
        let mut ring = sample(&spec, eye, crate::lod::SURFACE_FIRST_TIER);
        ring.set_near(true);
        ring.shape_near(&spec);
        let n = (NEAR_SHAPED * 2 + 1) * PROFILE_PER_CELL;
        assert_eq!(
            ring.shape_side, n,
            "the shape is not the block's own sample grid"
        );
        assert_eq!(ring.shape.len(), (n * n) as usize);
        // **One top face a sample, at that sample's own height**, and none of the flat cell tops
        // for the cells the shape owns.
        let flat = ring
            .quads
            .iter()
            .filter(|quad| quad.face == Face::PosY)
            .filter(|quad| quad.extent == [SUB_CELLS as u16, SUB_CELLS as u16])
            .filter(|quad| {
                let i = quad.uv[0] as i32 / SUB_CELLS - RING_MID;
                let j = quad.uv[1] as i32 / SUB_CELLS - RING_MID;
                i.abs().max(j.abs()) <= NEAR_SHAPED
            })
            .count();
        assert_eq!(flat, 0, "a shaped cell was also drawn as one flat cell");
        let step = PROFILE_STEPS as u16;
        let tops: Vec<_> = ring
            .quads
            .iter()
            .filter(|quad| quad.face == Face::PosY && quad.extent == [step, step])
            .collect();
        let measured = ring.shape.iter().filter(|sample| sample.measured()).count();
        assert_eq!(tops.len(), measured, "one top face a measured sample");
        for quad in &tops {
            let u = quad.uv[0] as i32 / PROFILE_STEPS - (RING_MID - NEAR_SHAPED) * PROFILE_PER_CELL;
            let v = quad.uv[1] as i32 / PROFILE_STEPS - (RING_MID - NEAR_SHAPED) * PROFILE_PER_CELL;
            let sample = ring.shape[(u + n * v) as usize];
            assert!(
                sample.measured(),
                "a top face stands over a point with no measurement"
            );
            assert_eq!(
                quad.slice, sample.plane,
                "a top face is not on its own sample's height"
            );
            assert_eq!(
                quad.material, sample.material,
                "a top face wears another point's material"
            );
        }
        // **One wall per lower neighbour**, the same comparison the samples themselves state.
        let at = |u: i32, v: i32| -> Option<u16> {
            if !(0..n).contains(&u) || !(0..n).contains(&v) {
                return None;
            }
            let sample = ring.shape[(u + n * v) as usize];
            sample.measured().then_some(sample.plane)
        };
        let mut expected = 0usize;
        for v in 0..n {
            for u in 0..n {
                let Some(here) = at(u, v) else { continue };
                for (du, dv) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let Some(that) = at(u + du, v + dv) else {
                        continue;
                    };
                    if that < here {
                        expected += 1;
                    }
                }
            }
        }
        let walls = ring
            .quads
            .iter()
            .filter(|quad| quad.face != Face::PosY)
            .filter(|quad| {
                let in_x = quad.uv[0] as i32 / PROFILE_STEPS
                    >= (RING_MID - NEAR_SHAPED) * PROFILE_PER_CELL
                    && (quad.uv[0] as i32) < (RING_MID + NEAR_SHAPED + 1) * SUB_CELLS;
                let in_z = quad.uv[1] as i32 / PROFILE_STEPS
                    >= (RING_MID - NEAR_SHAPED) * PROFILE_PER_CELL
                    && (quad.uv[1] as i32) < (RING_MID + NEAR_SHAPED + 1) * SUB_CELLS;
                in_x && in_z
            })
            .count();
        assert!(
            walls >= expected,
            "{walls} walls stand inside the shaped block, and {expected} neighbours are lower"
        );
        // And the shape is measured once and kept: shaping again is a re-mesh, not a re-read.
        let quads = ring.quads.len();
        ring.shape_near(&spec);
        assert_eq!(
            ring.quads.len(),
            quads,
            "the shape was re-meshed into a different block"
        );
    }

    /// **The step past the shaped block is closed on the block's side.** A fringe cell is a roof
    /// and has no side of its own, so where the block's own sample stands above or below that
    /// roof's edge, the wall has to be the block's — with the two heights as its ends. This walks
    /// the block's four rims and checks every such step has its wall, from the lower height to the
    /// higher one. A wall drawn only downward is the hole this test exists to catch.
    #[test]
    fn the_step_past_the_shaped_block_is_closed() {
        let spec = spec();
        let mut ring = sample(&spec, eye(), crate::lod::SURFACE_FIRST_TIER);
        ring.set_near(true);
        ring.shape_near(&spec);
        let n = ring.shape_side;
        let first = RING_MID - NEAR_SHAPED;
        let sub = SUB_CELLS as u16;
        let step = PROFILE_STEPS as u16;
        // The fringe cells' roofs: the same records `mesh` wrote, asked the same way.
        let mut roofs = std::collections::HashMap::new();
        for (quad, shape) in ring.quads.iter().zip(ring.shapes.iter()) {
            if quad.face == Face::PosY && quad.extent == [sub, sub] {
                roofs.insert(
                    (
                        quad.uv[0] as i32 / sub as i32,
                        quad.uv[1] as i32 / sub as i32,
                    ),
                    (quad.slice, *shape),
                );
            }
        }
        let roof_at = |i: i32, j: i32, u: i32, v: i32| -> Option<u16> {
            let (slice, s) = *roofs.get(&(i, j))?;
            let dx = f64::from(u.rem_euclid(PROFILE_PER_CELL) * PROFILE_STEPS);
            let dz = f64::from(v.rem_euclid(PROFILE_PER_CELL) * PROFILE_STEPS);
            let h = f64::from(slice)
                + f64::from(s[0]) * dx
                + f64::from(s[1]) * dz
                + f64::from(s[2]) * dx * dz;
            (0.0..=f64::from(u16::MAX))
                .contains(&h)
                .then(|| h.round() as u16)
        };
        let walls: std::collections::HashSet<_> = ring
            .quads
            .iter()
            .filter(|quad| quad.face != Face::PosY)
            .map(|quad| (quad.face, quad.slice, quad.uv, quad.extent))
            .collect();
        let mut closed = 0usize;
        for v in 0..n {
            for u in 0..n {
                let at = ring.shape[(u + n * v) as usize];
                if !at.measured() {
                    continue;
                }
                let i = first + u.div_euclid(PROFILE_PER_CELL);
                let j = first + v.div_euclid(PROFILE_PER_CELL);
                let ux = (i * SUB_CELLS + u.rem_euclid(PROFILE_PER_CELL) * PROFILE_STEPS) as u16;
                let uz = (j * SUB_CELLS + v.rem_euclid(PROFILE_PER_CELL) * PROFILE_STEPS) as u16;
                // Only the block's own rim: the neighbour cell past it is a fringe cell.
                let rim = [
                    (u == 0, Face::NegX, (i - 1, j), (u - 1, v)),
                    (u == n - 1, Face::PosX, (i + 1, j), (u + 1, v)),
                    (v == 0, Face::NegZ, (i, j - 1), (u, v - 1)),
                    (v == n - 1, Face::PosZ, (i, j + 1), (u, v + 1)),
                ];
                for (on_rim, face, cell, (nu, nv)) in rim {
                    if !on_rim {
                        continue;
                    }
                    let Some(roof) = roof_at(cell.0, cell.1, nu, nv) else {
                        continue;
                    };
                    if roof == at.plane {
                        continue;
                    }
                    let (low, span) = (roof.min(at.plane), roof.abs_diff(at.plane));
                    let key = match face.axis() {
                        0 => (
                            face,
                            if face == Face::PosX { ux + step } else { ux },
                            [low, uz],
                            [span, step],
                        ),
                        _ => (
                            face,
                            if face == Face::PosZ { uz + step } else { uz },
                            [ux, low],
                            [step, span],
                        ),
                    };
                    assert!(
                        walls.contains(&key),
                        "the step past the block at ({u}, {v}) has no wall: {key:?}"
                    );
                    closed += 1;
                }
            }
        }
        assert!(closed > 100, "only {closed} steps were checked");
    }

    /// **A rung delivered while the disc is asked for arrives shaped.** The worker owns the
    /// profile's 32² points a cell, so the frame that asks for the disc never pays them; a ring
    /// already in hand draws the flat disc from that same frame, and this is the delivery that
    /// replaces it.
    #[test]
    fn a_rung_delivered_while_the_disc_is_asked_for_arrives_shaped() {
        let mut ladder = SurfaceLadder::new(spec());
        let eye = eye();
        ladder.set_near_disc(true);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(180);
        while std::time::Instant::now() < deadline {
            ladder.advance(eye, SURFACE_RUNG_WAVE);
            if ladder
                .ring(crate::lod::SURFACE_FIRST_TIER)
                .is_some_and(|ring| ring.shaped())
            {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let ring = ladder
            .ring(crate::lod::SURFACE_FIRST_TIER)
            .expect("the first rung arrived");
        assert!(ring.shaped(), "the first rung arrived without its shape");
        assert!(ring.near, "the first rung arrived without its disc");
        assert!(
            ring.drawn(RING_MID, RING_MID),
            "the disc is not drawn at the eye's own cell"
        );
        // Every rung above it keeps the one-sample-a-cell law: this is the first rung's own block.
        for tier in crate::lod::surface_tiers()
            .into_iter()
            .filter(|t| *t > crate::lod::SURFACE_FIRST_TIER)
        {
            if let Some(ring) = ladder.ring(tier) {
                assert!(
                    !ring.shaped(),
                    "tier {tier} measured a shape it does not draw"
                );
                assert!(!ring.near, "tier {tier} was given a near disc");
            }
        }
    }
}
