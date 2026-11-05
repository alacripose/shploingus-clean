//! **The frame's own light: every cell of the world, measured rather than rigged.**
//!
//! `docs/PLAN-2026-10-10-clean-room-and-the-api-layer.md` names the defect in one line — *"the
//! world is lit by a rig, not by measurement"*. The solver is [`crate::skylight::WorldLight`]: the
//! sun and moon's own beams through the spectra, the sky's own bands, and
//! `WorldLight::bands_across` marching those beams chunk by chunk through the resident trees. This
//! module is the field the frame reads: an **address law**, a map of measurements under it, and the
//! queue of cells the frame asked for and does not have yet.
//!
//! # The address law is one law
//!
//! A cell is a **subvoxel** — the voxel `floor(p)` names, and which of that voxel's `4×4×4` cells
//! the point is in — and its address *is* its key ([`SubvoxelPos`]). The scene's corner rig asks
//! with it, the ray leaves at [`SubvoxelPos::centre_m`], and there is no second conversion
//! anywhere in this path.
//!
//! **The grain is the frame's shadow resolution, and the data is the ceiling on it.** Terrain
//! stores `4×4×4` cells per voxel, so a quarter-voxel cell is the finest address a shadow can be
//! measured at; the material *pattern*'s finer lattice (`DETAIL_SIDE = 16` per subvoxel) is
//! shading-only and carries no occupancy to occlude with, and the `1/32` in `FILL_SIDE` is a
//! fluid's pore volume. Sampling finer than the stored cells would be a claim about a shape
//! nobody kept.
//!
//! **Units are the trap this address was repaired for once, and the type is the guard.** The
//! draft asked the field at subvoxel grain (`floor(p / 0.25 m)`) while the field read those
//! integers as *metres*, so every corner named a cell four chunks from its own and the shipped
//! binary marched **zero** of the 1.1 M corners it rigged. The division now happens once, in
//! [`SubvoxelPos::of_point`], and both halves of the address travel together — there is no integer
//! here that can be read as a metre by mistake.
//!
//! # Measurement is not the frame's work
//!
//! [`CellField`] **does not march**. A build asks [`CellField::at`] and gets a measurement or
//! nothing; a cell it has not measured is queued ([`CellField::queued`]) and returned as `None`,
//! which is the rig's own light and a *different* one ([`CellField::unmeasured`]). The marches
//! happen on [`crate::measure::Measurer`]'s own thread and arrive through [`CellField::adopt`], so
//! nothing a frame does grows with the world, the beam or the queue — the property the shipped
//! frame is held to (8.1 ms settled, ~312 MB).
//!
//! **The budget this replaced was a per-frame count of 1 024 cells** — about ten milliseconds of
//! marches on the very frame that rebuilt the scene, ten of those a second while the field filled.
//! A cell now arrives when the worker has it.
//!
//! # The beam's own step, and why staleness is **per cell**
//!
//! Each cell records **the beam direction it was measured under** and is stale when *that*
//! direction has swung past [`BEAM_STEP_DEG`] — the solver's own resolution rather than a tuning
//! constant: [`crate::skylight`]'s dome is a five-ray stencil of a hemisphere, and the Moon's own
//! half a degree of arc is the smallest thing in the frame that a shadow could be *for*. That is
//! about 1.7 seconds of this world's clock.
//!
//! **What is refused is silence about a stale answer.** A stale cell is answered from its last
//! measurement and **counted** ([`CellField::stale`]) — and *queued again*, so the frame's own
//! demand is what refreshes it: a whole-field re-march on every step spent its asks on cells nobody
//! was looking at, and took long enough to be overtaken by the next step. What *is* dropped
//! wholesale is a change of **body** ([`CellField::beam_is_moon`]): the Sun's beam and the Moon's
//! are different lights, and cells measured against one are not measurements of the other at any
//! step.
//!
//! The kept cells are **not** re-normalised by the new sun's colour, and that is deliberate: a
//! band vector carries the beam's own spectrum, and rescaling it would be this module inventing a
//! partial measurement of a beam it did not march.

use std::collections::HashMap;
use std::sync::Arc;

use voxel_appearance::sky::SkyInputs;
use voxel_mesh::SparseVoxelOctree;
use voxel_model::{ChunkPos, SubvoxelIndex, VoxelPos, SUBVOXEL_SIDE};
use voxel_shapers::WorldSpec;

use crate::skylight::WorldLight;

/// Where the frame's world is: the octree of the chunk holding a position.
///
/// **A handle rather than a borrow**, and that is what lets a measurement happen off the frame:
/// the streamer keeps its trees behind `Arc` and this hands the measurer the same allocation
/// rather than a copy of it, so a cell measured on another thread costs a refcount and no bytes.
/// A trait rather than `&Streamer` so the field can be measured over any provider of resident
/// trees — the streamer in the game, a fixture in a test, a census in an example.
pub trait ChunkTrees {
    /// The tree of the chunk containing `pos`, or `None` when the chunk is not resident.
    ///
    /// **`None` is a fact and not an error**: a chunk outside the window, or one being rebuilt,
    /// has no matter to charge a beam against. [`WorldLight::bands_across`] is the only caller
    /// that interprets it, and it does so explicitly — a cell whose own chunk is absent is
    /// unmeasured, and a hop past an absent chunk is **counted** as unresolved rather than read
    /// as clear air.
    fn tree_at(&self, pos: ChunkPos) -> Option<Arc<SparseVoxelOctree>>;
}

impl ChunkTrees for voxel_work::Streamer {
    fn tree_at(&self, pos: ChunkPos) -> Option<Arc<SparseVoxelOctree>> {
        self.tree_shared(pos)
    }
}

/// How far a shadow ray marches, in metres.
///
/// **Five chunks**, and it is a statement about the world's own scale rather than a performance
/// dial: a chunk is 32 m, the first playable column's relief is tens of metres, and a beam that
/// reaches further than this is a beam whose occlusion is usually a mountain range's *silhouette*
/// and not a nearby wall. The value is a ceiling on the march and never a claim about the light
/// beyond it — a ray that leaves the last chunk with transmittance left over reports that
/// remainder and no more, which is what `reach_m` means everywhere in this project.
pub const REACH_M: f64 = 160.0;

/// How many unmeasured or stale cells the field will hold in its queue before it stops recording.
///
/// A bound rather than a policy: 65 536 addresses are sixteen asks of [`crate::CELLS_PER_ASK`], and
/// the resident window's own surface is about 82 000 cells — so the backlog holds most of a
/// window and the rest arrives as the queue drains rather than being dropped. Past the cap the
/// field records nothing new and the next build records it again: the queue drains and refills
/// rather than growing for as long as the player walks. The first version held 262 144 and charged
/// the working set four megabytes for duplicates it never needed.
const WANTED_CAP: usize = 65_536;

/// How far the beam may swing before a kept cell is re-measured, in degrees.
///
/// See this module's doc: it is the angle at which the sun's own movement is comparable to the
/// stencil's resolution, so a kept cell is stale by less than the solver's own quantisation.
pub const BEAM_STEP_DEG: f64 = 0.5;

/// [`BEAM_STEP_DEG`]'s cosine, so a cell's staleness is **one dot product** rather than an
/// `acos` per corner: a build asks at every corner of every quad, and the inverse trigonometric
/// function on that path was milliseconds of a rewrite frame. [`the_step_is_the_cosine_of_its_angle`]
/// holds the two forms equal.
const BEAM_STEP_COS: f64 = 0.999_961_923_064_171_3;

/// The chunk a world point is in — **the cell's own chunk**, through the one address law.
///
/// A non-finite point names the sentinel chunk `i32::MIN`, which no world holds, so the provider
/// answers `None` and the cell reads as unmeasured rather than taking a panic or a zero.
pub(crate) fn chunk_of(point_m: [f64; 3]) -> ChunkPos {
    match VoxelPos::of_point(point_m) {
        Some(cell) => cell.chunk(),
        None => ChunkPos::new(i32::MIN, i32::MIN, i32::MIN),
    }
}

/// **A multiply-rotate hash for cell addresses**, because the frame makes one lookup per corner.
///
/// The resident window is 228 000 corners a build, and `std`'s default hasher — written for
/// adversary-resistant keys, not for three integers — cost **29 ms of a 30 ms build** when this
/// pass first measured it. Three mixes are enough for a key whose three words *are* the identity.
#[derive(Default, Clone, Copy)]
struct CellHash(u64);

impl std::hash::Hasher for CellHash {
    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.write_u32(*byte as u32);
        }
    }
    fn write_u32(&mut self, value: u32) {
        self.0 = (self.0.rotate_left(5) ^ value as u64).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
    fn write_i32(&mut self, value: i32) {
        self.write_u32(value as u32);
    }
    fn finish(&self) -> u64 {
        self.0
    }
}

/// **One subvoxel's address**: the voxel it is in, and which of that voxel's cells.
///
/// The two halves travel together because they are one address: an integer triple alone was read
/// as metres by one end of this path and as quarter-metres by the other, and every cell it named
/// was four chunks away. [`SubvoxelPos::of_point`] is the only place the division happens.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SubvoxelPos {
    /// The voxel the cell is in — `voxel-model`'s own address law.
    pub voxel: VoxelPos,
    /// Which of the voxel's `4×4×4` cells.
    pub at: SubvoxelIndex,
}

impl SubvoxelPos {
    /// The address of one cell of one voxel, from its two halves.
    pub const fn new(voxel: VoxelPos, at: SubvoxelIndex) -> Self {
        SubvoxelPos { voxel, at }
    }

    /// **The cell a world point stands in** — the one address law, and the one division.
    ///
    /// `None` for a point with a non-finite axis, for the reason [`VoxelPos::of_point`] refuses
    /// one: a NaN corner is a broken quad and the rig's light is the honest answer for it.
    pub fn of_point(point_m: [f64; 3]) -> Option<Self> {
        let voxel = VoxelPos::of_point(point_m)?;
        let side = SUBVOXEL_SIDE as f64;
        let mut at = [0u8; 3];
        for axis in 0..3 {
            let origin = [voxel.x, voxel.y, voxel.z][axis] as f64;
            // Clamped rather than refused: a corner exactly on a cell's boundary lands on
            // fraction `1.0`, which is the next cell's `0.0`, and that is the common case rather
            // than an error — the same rule `raycast::subvoxel_of` applies to a hit.
            at[axis] = ((point_m[axis] - origin) * side)
                .floor()
                .clamp(0.0, side - 1.0) as u8;
        }
        Some(SubvoxelPos {
            voxel,
            at: SubvoxelIndex {
                x: at[0],
                y: at[1],
                z: at[2],
            },
        })
    }

    /// **The point a shadow ray leaves from**: the centre of the cell.
    ///
    /// A quarter of a metre in from the cell's low face, so the march starts in the middle of the
    /// address it charges rather than on a face two cells share.
    pub fn centre_m(&self) -> [f64; 3] {
        let size = 1.0 / SUBVOXEL_SIDE as f64;
        [
            self.voxel.x as f64 + (self.at.x as f64 + 0.5) * size,
            self.voxel.y as f64 + (self.at.y as f64 + 0.5) * size,
            self.voxel.z as f64 + (self.at.z as f64 + 0.5) * size,
        ]
    }
}

impl From<VoxelPos> for SubvoxelPos {
    /// **The cell a per-voxel caller means**: the one the voxel's own centre stands in.
    ///
    /// A caller that only ever had voxel addresses — an example measuring one cell per voxel, a
    /// census counting them — asks for the cell at the middle of the voxel rather than picking a
    /// corner of it, and the pick is stated here once instead of at each call site. The frame does
    /// not use this: [`SubvoxelPos::of_point`] is its address, and it names the corner's own cell.
    fn from(voxel: VoxelPos) -> Self {
        SubvoxelPos::of_point(voxel.centre_m()).expect("a voxel's centre is a cell")
    }
}

/// The map of measurements, under [`CellHash`].
type CellMap = HashMap<SubvoxelPos, Kept, std::hash::BuildHasherDefault<CellHash>>;

/// One measurement, **and the beam direction it was taken under**.
///
/// A band vector alone would make the step a property of the field and force the whole map to be
/// dropped to keep it honest.
struct Kept {
    bands: [f32; 6],
    toward: [f64; 3],
}

/// Somewhere the frame reads the world's light from.
pub struct CellField {
    light: WorldLight,
    /// The direction the kept cells are being measured with, normalised. `None` until the solver
    /// gives one, which is the only case in which the field has no beam to measure at all.
    toward: Option<[f64; 3]>,
    /// **Which body the kept cells were measured from.** The swing test is a direction test, and
    /// two bodies can share a direction: at the moment the Sun sets and the Moon rises they can
    /// both sit within [`BEAM_STEP_DEG`] of the horizon's same point, and the Sun's beam kept for
    /// the Moon's would be a bright day's light cast by a night's body.
    beam_is_moon: bool,
    /// The measurement, per cell address. **The address is the key**: a voxel is its own name, so
    /// there is no packing to compute per corner and no arithmetic per lookup.
    cells: CellMap,
    /// Cells a build asked for and this field has no measurement for.
    ///
    /// **A `Vec` rather than a set**, because a build asks one corner at a time — 228 000 of them
    /// for the resident window — and a hash insert on every miss put this pass's own bookkeeping
    /// on the frame at milliseconds a build. A push is a few nanoseconds; the duplicates it admits
    /// are a repeated march on the worker's clock, which is the clock this design exists to spend.
    wanted: std::collections::VecDeque<SubvoxelPos>,
    /// Cells whose measurement is a step behind the light — still answered, and queued to be
    /// brought up. Kept apart from [`CellField::wanted`] because they *are* held: the miss queue
    /// filters against the map, and a stale cell would be filtered out of the very queue that
    /// exists to refresh it.
    behind: std::collections::VecDeque<SubvoxelPos>,
    adopted: u64,
    answered: u64,
    /// Corners this frame answered from a measurement whose own beam had swung past
    /// [`BEAM_STEP_DEG`]: a *counted* stale answer rather than a silent one.
    stale: u64,
    unmeasured: u64,
    /// Hops whose chunk was absent, summed over the measurements this field adopted: how much of
    /// the field is measuring past a hole in the resident world. Reported rather than hidden,
    /// because a field lit as if an absent chunk were clear air is exactly the plausible wrong
    /// answer this project keeps refusing.
    unresolved: u64,
}

impl CellField {
    /// The frame's field at `tick`, **keeping the cells the previous field measured when the beam
    /// has not changed body**.
    ///
    /// `None` when the solver cannot be built at this tick at all — a degenerate latitude or an
    /// unnormalisable beam — in which case the caller keeps the rig and the frame is lit the way
    /// it was before this module existed, which is the honest fallback rather than a black frame.
    pub fn of(
        spec: &WorldSpec,
        tick: u64,
        inputs: SkyInputs,
        new_moon_tick: u64,
        previous: Option<CellField>,
    ) -> Option<Self> {
        let light = WorldLight::of(spec, tick, inputs, new_moon_tick)?;
        Some(Self::of_light(light, previous))
    }

    /// The same field, **from a solver the caller already has**.
    ///
    /// [`CellField::of`] derives the solver and then carries the previous cells over; a caller that
    /// has the tick's `WorldLight` already — the atmosphere, which derives it once for the rig, the
    /// beams and the frame — should not derive it a second time to wrap it. What this carries over
    /// is exactly what `of` does: the cells, unless the body changed.
    pub fn of_light(light: WorldLight, previous: Option<CellField>) -> Self {
        let toward = light.toward_beam();
        let beam_is_moon = light.beam_is_moon();
        // **The backlog is carried across a cadence, and that is not housekeeping.** A cadence
        // builds a new field five times a second while a scene build — which is what *fills* the
        // queue — happens on the beam's step, twice a second at most. Dropping the queues here let
        // each cadence wipe most of a build's demand before the measurer could be asked for it,
        // and the field stalled at a quarter of the window with nothing queued to explain it.
        let (cells, wanted, behind) = match previous {
            // **A different body is a different light**: every kept cell was measured against the
            // Sun's beam and none of them is about the Moon's, so the map is dropped rather than
            // re-labelled. The backlog is addresses, not beams, and survives.
            Some(before) if before.beam_is_moon == beam_is_moon => {
                (before.cells, before.wanted, before.behind)
            }
            Some(before) => (CellMap::default(), before.wanted, before.behind),
            None => (
                CellMap::default(),
                std::collections::VecDeque::new(),
                std::collections::VecDeque::new(),
            ),
        };
        CellField {
            light,
            toward: Some(toward),
            beam_is_moon,
            cells,
            wanted,
            behind,
            adopted: 0,
            answered: 0,
            stale: 0,
            unmeasured: 0,
            unresolved: 0,
        }
    }

    /// The solver behind this field, for a caller that wants a beam or a sky radiance on its own.
    pub fn light(&self) -> &WorldLight {
        &self.light
    }

    /// **Begin a frame**: the four counters go back to zero.
    ///
    /// The counters are the frame's and not the field's, so a caller that reports "this frame
    /// answered N corners and had to rig M" is reading the frame it drew. The **cells** are not
    /// touched: they are what the field is holding, and a frame that began by dropping them would
    /// re-measure the whole visible world four times a second for nothing.
    pub fn begin_frame(&mut self) {
        self.adopted = 0;
        self.answered = 0;
        self.stale = 0;
        self.unmeasured = 0;
        self.unresolved = 0;
    }

    /// **One cell's six bands, if the field has measured it.** A cell is a **subvoxel**:
    /// [`SubvoxelPos`] is the address, so a corner inside a voxel is measured at its own quarter
    /// and not answered from the voxel's centre.
    ///
    /// `None` is "no measurement here": the cell is queued, or it is past the world, or the beam
    /// has changed body. The caller's fallback is the rig, which is a *different* light and a
    /// declared one, rather than a zero that would read as "this cell is in the dark". A missing
    /// cell is **recorded**, and so is a stale one, so the frame's own demand is what the measurer
    /// is asked about.
    pub fn at(&mut self, cell: SubvoxelPos) -> Option<[f32; 6]> {
        // The beam this field is measured against. `None` only for a field that was never given a
        // direction, which `of_light` cannot build — and then there is no beam to measure.
        let toward = self.toward?;
        let Some(kept) = self.cells.get(&cell) else {
            self.unmeasured += 1;
            if self.wanted.len() < WANTED_CAP {
                self.wanted.push_back(cell);
            }
            return None;
        };
        // **This cell's own swing**, dotted and compared against the step's cosine.
        let dot =
            kept.toward[0] * toward[0] + kept.toward[1] * toward[1] + kept.toward[2] * toward[2];
        if dot < BEAM_STEP_COS {
            // Answered, counted, and queued: the ref-bands are near enough to draw and not near
            // enough to keep.
            self.stale += 1;
            if self.behind.len() < WANTED_CAP {
                self.behind.push_back(cell);
            }
        } else {
            self.answered += 1;
        }
        Some(kept.bands)
    }

    /// **Take the cells to measure next**, up to `limit`: the stale ones first, then the missing.
    ///
    /// Drained from the **front**, because the queue is a backlog and not a stack: the first
    /// version popped from the back, which is the most recently pushed cell, and since every build
    /// pushes its corners in mesh order that served the same tail of the same chunk over and over
    /// — the field grew for ten seconds and then stopped with fifty thousand cells still missing.
    ///
    /// Both queues are self-healing: a cell whose measurement never comes — an absent chunk — is
    /// recorded again by the next build that asks for it.
    pub fn wants(&mut self, limit: usize) -> Vec<SubvoxelPos> {
        let mut out = Vec::with_capacity(limit);
        while out.len() < limit {
            let Some(cell) = self.behind.pop_front() else {
                break;
            };
            out.push(cell);
        }
        while out.len() < limit {
            let Some(cell) = self.wanted.pop_front() else {
                break;
            };
            // A miss that was measured since is not a miss.
            if !self.cells.contains_key(&cell) {
                out.push(cell);
            }
        }
        out
    }

    /// **Take one measurement**, under the beam direction it was marched with.
    ///
    /// The direction travels with the value because the worker derives its own solver from the
    /// same `(spec, tick, inputs)` triple the frame's field was derived from — one function of one
    /// argument set, not a second opinion about the sky — and a batch that arrives after the frame
    /// has moved on is still a measurement of *its* beam.
    pub fn adopt(&mut self, cell: SubvoxelPos, bands: [f32; 6], toward: [f64; 3]) {
        self.cells.insert(cell, Kept { bands, toward });
        self.adopted += 1;
    }

    /// **March one cell on this thread** and keep the measurement.
    ///
    /// The frame never calls this — that is [`crate::measure::Measurer`]'s job — but a test, an
    /// example and a census have no worker and no frame to protect, and they need the same march
    /// the worker performs rather than a second copy of it.
    pub fn measure(
        &mut self,
        trees: &dyn ChunkTrees,
        cell: SubvoxelPos,
    ) -> Option<([f32; 6], usize)> {
        let toward = self.toward?;
        let (bands, absent) = self.light.bands_across(trees, cell.centre_m(), REACH_M)?;
        self.unresolved += absent as u64;
        self.adopt(cell, bands, toward);
        Some((bands, absent))
    }

    /// Corners this frame answered from a measurement whose own beam has since swung past
    /// [`BEAM_STEP_DEG`] — the field's own statement of how far behind the light the frame is.
    pub fn stale(&self) -> u64 {
        self.stale
    }

    /// Measurements this frame received.
    pub fn adopted(&self) -> u64 {
        self.adopted
    }

    /// Corners answered from a measurement under the current beam.
    pub fn answered(&self) -> u64 {
        self.answered
    }

    /// Corners that fell back to the rig: no measurement for the cell.
    pub fn unmeasured(&self) -> u64 {
        self.unmeasured
    }

    /// Hops this field's measurements made past a chunk the provider does not hold.
    pub fn unresolved(&self) -> u64 {
        self.unresolved
    }

    /// How many cells this field is holding.
    pub fn held(&self) -> usize {
        self.cells.len()
    }

    /// How many cells are queued for measurement and not yet asked for.
    pub fn queued(&self) -> usize {
        self.wanted.len() + self.behind.len()
    }

    /// The direction the beam came from when the kept cells were measured.
    ///
    /// Exposed because it is the whole of this field's staleness story: a caller that wants to
    /// report *how stale* the field is has the direction rather than a flag, and can measure it
    /// against the solver's current one itself.
    pub fn toward_beam(&self) -> Option<[f64; 3]> {
        self.toward
    }
}

/// The closure [`crate::scene::Scene::build_with_light`] takes, over a field.
///
/// A free function rather than a method so the field can be borrowed mutably inside the closure
/// while the scene's own borrows are held — a method on `CellField` cannot express that.
pub fn light_at<'a>(
    field: &'a std::cell::RefCell<CellField>,
) -> impl Fn([f64; 3]) -> Option<[f32; 6]> + 'a {
    move |point_m| {
        let cell = SubvoxelPos::of_point(point_m)?;
        field.borrow_mut().at(cell)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One chunk of clear air, and nothing outside it — a resident tree is all these tests need,
    /// because what they measure is the field's own bookkeeping and not a radiance.
    struct Air {
        tree: Arc<SparseVoxelOctree>,
        chunk: ChunkPos,
    }
    impl ChunkTrees for Air {
        fn tree_at(&self, pos: ChunkPos) -> Option<Arc<SparseVoxelOctree>> {
            (pos == self.chunk).then(|| Arc::clone(&self.tree))
        }
    }

    fn air() -> Air {
        Air {
            tree: Arc::new(SparseVoxelOctree::with_levels(5)),
            chunk: ChunkPos::new(0, 0, 0),
        }
    }

    fn a_field(tick: u64) -> CellField {
        let world = WorldSpec::new(2468, voxel_shapers::Config::default());
        CellField::of(&world, tick, SkyInputs::default(), 0, None).expect("a field")
    }

    /// The step's cosine is the cosine of the step — one number written twice, checked.
    #[test]
    fn the_step_is_the_cosine_of_its_angle() {
        let cosine = BEAM_STEP_DEG.to_radians().cos();
        assert!(
            (BEAM_STEP_COS - cosine).abs() < 1e-15,
            "the constant is {BEAM_STEP_COS} and cos({BEAM_STEP_DEG}°) is {cosine}"
        );
    }

    /// **A cell is its own address**, and the law is `voxel-model`'s: every point inside one voxel
    /// names one cell, and the cell's own chunk is the point's.
    #[test]
    fn a_cell_is_its_own_key_and_the_chunk_is_the_points_own() {
        let left = VoxelPos::of_point([3.0, 0.0, 0.0]).expect("a cell");
        let right = VoxelPos::of_point([3.999_999, 0.0, 0.0]).expect("a cell");
        assert_eq!(left, right, "two corners in one voxel named two cells");
        assert_eq!(left.chunk(), ChunkPos::new(0, 0, 0));
        // Euclidean, as `voxel-model`'s own contract says: `(-1, -1, -1)` is the *last* cell of
        // chunk `(-1, -1, -1)` and not the first of chunk `(0, 0, 0)`.
        assert_eq!(VoxelPos::new(-1, -1, -1).chunk(), ChunkPos::new(-1, -1, -1));
        assert_eq!(VoxelPos::new(-32, 0, 0).chunk(), ChunkPos::new(-1, 0, 0));
        // **A voxel is not the address.** The key is a quarter-voxel cell, so two points inside
        // one voxel are two cells — the grain the frame's shadow edge is drawn at — and a point
        // and itself are one.
        assert_ne!(
            SubvoxelPos::of_point([2.1, 0.0, 0.0]),
            SubvoxelPos::of_point([2.9, 0.0, 0.0]),
            "one voxel's quarters were one cell"
        );
        assert_eq!(
            SubvoxelPos::of_point([2.1, 0.0, 0.0]),
            SubvoxelPos::of_point([2.2, 0.0, 0.0]),
            "one cell's own quarter named two cells"
        );
        // And the per-voxel caller's cell is the one its centre stands in, not a corner of it.
        let middle = SubvoxelPos::from(VoxelPos::new(2, 0, 0));
        assert_eq!((middle.at.x, middle.at.y, middle.at.z), (2, 2, 2));
        let centre = middle.centre_m();
        assert_eq!(
            SubvoxelPos::of_point(centre),
            Some(middle),
            "the centre left its own cell"
        );
    }

    #[test]
    fn a_point_off_the_world_still_names_a_chunk() {
        // **Nothing here may panic on a NaN.** A corner is a float that came out of a mesh, and a
        // non-finite corner is a fact about a broken quad rather than a licence to take the frame
        // down; the **sentinel** chunk is a chunk no world holds, so any provider answers `None`
        // for it and the cell reads as unmeasured — the rig lights it, which is the light this
        // field falls back to everywhere it has nothing to say.
        assert_eq!(
            chunk_of([f64::NAN, 0.0, 0.0]).x,
            i32::MIN,
            "NaN names the sentinel"
        );
        assert_eq!(
            chunk_of([f64::INFINITY, 0.0, 0.0]).x,
            i32::MIN,
            "so does an infinity"
        );
        assert_eq!(chunk_of([f64::NEG_INFINITY, 0.0, 0.0]).x, i32::MIN);
        // And the ordinary cases, around zero: 0.0 through 31.999 is chunk 0.
        assert_eq!(chunk_of([0.0, 0.0, 0.0]).x, 0);
        assert_eq!(chunk_of([31.999, 0.0, 0.0]).x, 0);
        assert_eq!(chunk_of([32.0, 0.0, 0.0]).x, 1);
        assert_eq!(chunk_of([-0.001, 0.0, 0.0]).x, -1);
    }

    /// **A cell the field has not measured is queued, answered as `None`, and counted — and the
    /// measurement that lands later answers it without a march.**
    ///
    /// The three arms of [`CellField::at`] are one arm each: a miss (queued), a hit under the
    /// measured beam (answered), and a hit whose beam has swung (answered, **counted stale**, and
    /// queued to be brought up).
    #[test]
    fn a_miss_is_queued_and_the_measurement_that_lands_answers_it() {
        let trees = air();
        let mut field = a_field(voxel_shapers::climate::TICKS_PER_DAY / 2);
        let cell = SubvoxelPos::from(VoxelPos::new(4, 0, 0));
        field.begin_frame();
        assert_eq!(
            field.at(cell),
            None,
            "an unmeasured cell answered with a light"
        );
        assert_eq!(field.unmeasured(), 1, "a rigged corner went uncounted");
        assert_eq!(field.queued(), 1, "the frame's demand was not recorded");
        let wanted = field.wants(16);
        assert_eq!(wanted, vec![cell], "the queue did not name the cell");
        assert_eq!(field.queued(), 0, "the queue was not drained");

        // The march, off this thread in the game and here for the test.
        let (bands, _absent) = field.measure(&trees, cell).expect("a resident cell");
        assert_eq!(field.held(), 1);
        field.begin_frame();
        assert_eq!(
            field.at(cell),
            Some(bands),
            "a measured cell lost its bands"
        );
        assert_eq!(field.answered(), 1, "a hit was not counted");
        assert_eq!(field.unmeasured(), 0, "a hit was counted as a miss");
        assert_eq!(field.queued(), 0, "a current cell was queued again");

        // **A cell whose own chunk is not resident is not measured at all**, which is the rig's
        // fallback and not a zero.
        let far = SubvoxelPos::from(VoxelPos::new(4, 0, 40));
        assert_eq!(
            field.measure(&trees, far),
            None,
            "a cell past the world measured"
        );
    }

    /// **The field's coverage survives the sun's motion, a stale answer is counted, and the stale
    /// cell is queued again.** The offset **swings the beam past the step** (checked rather than
    /// assumed); the kept cells are answered nonetheless, and the queue is what the caller hands
    /// the measurer to bring them up.
    ///
    /// **The offset is read from the clock, not stated as forty ticks.** A day is
    /// `TICKS_PER_DAY` long and the beam crosses `360°` once in it, so the ticks that move it past
    /// `BEAM_STEP_DEG` are `TICKS_PER_DAY · step / 360` — times `1.5`, so the assertion below is met
    /// with room to spare. Forty was this world's number while a day was short; after the day
    /// length changed it moved the beam `0.061°`, and the test failed claiming the other two arms
    /// of `at` were wrong when it was the fixture that had stopped swinging the beam.
    #[test]
    fn the_fields_coverage_survives_a_beam_swing_and_a_stale_answer_is_counted() {
        let trees = air();
        let noon = voxel_shapers::climate::TICKS_PER_DAY / 2;
        let mut field = a_field(noon);
        let mut cells = Vec::new();
        for index in 0..64 {
            let cell = SubvoxelPos::from(VoxelPos::new((index % 8) as i32, 0, (index / 8) as i32));
            field.measure(&trees, cell).expect("a resident cell");
            cells.push(cell);
        }
        assert_eq!(field.held(), 64);
        let early = field.toward_beam().expect("a beam direction");

        let world = WorldSpec::new(2468, voxel_shapers::Config::default());
        let day = voxel_shapers::climate::TICKS_PER_DAY as f64;
        let ticks_past_the_step = (day * (BEAM_STEP_DEG / 360.0) * 1.5).ceil() as u64;
        let mut moved = CellField::of(
            &world,
            noon + ticks_past_the_step,
            SkyInputs::default(),
            0,
            Some(field),
        )
        .expect("a field at the later tick");
        let later = moved.toward_beam().expect("a beam direction");
        let swing = (early[0] * later[0] + early[1] * later[1] + early[2] * later[2])
            .clamp(-1.0, 1.0)
            .acos()
            .to_degrees();
        assert!(
            swing > BEAM_STEP_DEG,
            "{ticks_past_the_step} ticks moved the beam {swing}°, which is not past the \
             {BEAM_STEP_DEG}° step"
        );
        assert_eq!(moved.held(), 64, "the swing emptied the map");
        moved.begin_frame();
        for cell in &cells {
            assert!(moved.at(*cell).is_some(), "a kept cell was dropped");
        }
        assert_eq!(moved.stale(), 64, "a swung cell was answered as current");
        assert_eq!(moved.answered(), 0, "a swung cell was counted as current");
        // **And the stale cells are the queue**, so the caller's next ask refreshes exactly what
        // the frame is drawing rather than the whole world.
        let queued = moved.wants(128);
        assert_eq!(
            queued.len(),
            64,
            "the stale cells were not queued for a re-march"
        );
        assert!(
            queued.iter().all(|cell| cells.contains(cell)),
            "the queue named a stranger"
        );

        // **A different body is a different light.** At tick zero the Sun is down, so the field's
        // beam is the Moon's — not the Sun's by however many degrees — and every kept cell is
        // discarded rather than re-labelled.
        let night =
            CellField::of(&world, 0, SkyInputs::default(), 0, Some(moved)).expect("a field");
        assert_eq!(
            night.held(),
            0,
            "the Sun's cells were kept for the Moon's beam"
        );
    }
}
