//! **The frame's own light: every cell of the world, measured rather than rigged.**
//!
//! `docs/PLAN-2026-10-10-clean-room-and-the-api-layer.md` names the defect in one line — *"the
//! world is lit by a rig, not by measurement"* — and this module is the second half of the
//! repair. The first half is [`crate::skylight::WorldLight`], which holds the **solver**: the
//! sun and moon's own beams through the spectra, the sky's own bands, and
//! `WorldLight::bands_across` marching those beams chunk by chunk through the resident trees.
//! What was missing was a **field**: the rig's four-direction toy light was the only thing in
//! the frame that knew where the sun was, and no corner of any surface ever asked the world what
//! stood between it and the sky.
//!
//! # What this is, and what it refuses to be
//!
//! [`CellField`] measures `bands_across` at the cell a corner stands in and **caches it for the
//! frame**, under a budget, and it is re-measured from scratch when the beam has swung. It is a
//! field of *measurements* and not a cache of results to be trusted: a cell it has not measured
//! returns `None`, and the caller falls back to the rig. That is the difference between this and
//! the thing it could have been — a second solver with its own staleness rules, whose numbers
//! would be indistinguishable from measurements and wrong.
//!
//! # The budget is a per-frame count and it is visible
//!
//! A frame asks for cells at every corner of every quad it builds — hundreds of thousands of
//! times — and a shadow march is not free. So `CELLS_PER_FRAME` is a hard ceiling: the cells
//! after it in a frame return `None` and are **counted** ([`CellField::unmeasured`]). The count
//! is a number a test can hold, not a silent truncation: it is how the field says "this frame saw
//! the rig here" rather than "the world is dark here".
//!
//! # The beam's own step, and why staleness is **per cell**
//!
//! Between frames the sun moves by a fraction of a degree and the field's answer moves by a
//! fraction of a band. Re-measuring four thousand shadow rays for that is the kind of thrash that
//! makes a correct thing unusable, so each cell records **the beam direction it was measured under**
//! and is re-marched when *that* direction has swung past [`BEAM_STEP_DEG`]. The threshold is the
//! solver's own resolution rather than a tuning constant: [`crate::skylight`]'s dome is a five-ray
//! stencil of a hemisphere, forty-five degrees of solid angle per ray, and the Moon's own half a
//! degree of arc is the smallest thing in the frame that a shadow could be *for*.
//!
//! **The whole map used to be thrown away when the beam swung**, and that is the version this file
//! no longer is. `BEAM_STEP_DEG` of sun motion is about `1.7` seconds of this world's clock (the
//! hour angle sweeps `0.3°/s` at `TICKS_PER_DAY = 24_000`), so the field was emptied and had to
//! re-cover the visible world from nothing every second and a bit — and a frame that re-covers has
//! most of its corners on the rig, which is exactly the picture the field exists to replace. A
//! per-cell step costs nothing extra and means **coverage survives the sun's motion**: the first
//! pass fills the map and every later frame re-marches only the cells whose own beam has moved.
//!
//! **What is still refused is silence about a stale answer.** A cell whose beam has swung and whose
//! re-march did not fit the frame's budget is answered from its last measurement and **counted**
//! ([`CellField::stale`]), so "this frame drew N corners a step behind the light" is a number a
//! caller reads rather than a claim nobody made. The one thing that *is* thrown away wholesale is a
//! change of **body** ([`CellField::beam_is_moon`]): the Sun's beam and the Moon's are different
//! lights, and cells measured against one are not measurements of the other at any step.
//!
//! The kept cells are **not** re-normalised by the new sun's colour, and that is deliberate: a
//! band vector carries the beam's own spectrum, and rescaling it would be this module inventing a
//! partial measurement of a beam it did not march.

use std::collections::HashMap;

use voxel_appearance::sky::SkyInputs;
use voxel_mesh::SparseVoxelOctree;
use voxel_model::{ChunkPos, CHUNK_SIDE};
use voxel_shapers::WorldSpec;

use crate::skylight::WorldLight;

/// Where the frame's world is: the octree of the chunk holding a position.
///
/// **A trait rather than `&Streamer`** so that the field can be measured over any provider of
/// resident trees — the streamer in the game, a fixture in a test, a census in an example — and
/// so that `voxel-client` does not need `voxel-work` to *be* the world. The streamer is the
/// provider the game uses, and its implementation is three lines below.
pub trait ChunkTrees {
    /// The tree of the chunk containing `pos`, or `None` when the chunk is not resident.
    ///
    /// **`None` is a fact and not an error**: a chunk outside the window, or one being rebuilt,
    /// has no matter to charge a beam against. [`WorldLight::bands_across`] is the only caller
    /// that interprets it, and it does so explicitly — see that function's own note on the
    /// difference between the cell's own chunk and a chunk further along the ray.
    fn tree_at(&self, pos: ChunkPos) -> Option<&SparseVoxelOctree>;
}

impl ChunkTrees for voxel_work::Streamer {
    fn tree_at(&self, pos: ChunkPos) -> Option<&SparseVoxelOctree> {
        self.tree(pos)
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

/// How many cells one frame may measure.
///
/// A ceiling with a counter beside it ([`CellField::unmeasured`]) rather than a silent stop, and the
/// number is **measured rather than chosen**: `voxel-client`'s `measured-light` example marches a
/// generated world's own columns at **0.01 ms a cell** (126 cells in 1.0 ms, release, a 160 m
/// reach, in a world whose cells are mostly one hop from rock), so 1 024 cells is about **eight
/// milliseconds** at the very worst — a frame's light can be costly, and it cannot be a stall.
///
/// The alternative was 4 096, which the same measurement puts at **32 ms**: a frame that walked its
/// whole budget would be a stutter the player would report as a freeze, and the honest budget is the
/// one that cannot. What the size buys instead is *latency*: the cells a frame draws are filled at a
/// few thousand a frame, so a surface a player has just turned to face is lit by the rig for its
/// first frames and by the world's own light from then on — a fade in the direction of the truth,
/// with the count in [`CellField::unmeasured`] saying how much of the frame is still the rig.
pub const CELLS_PER_FRAME: usize = 1024;

/// How far the beam may swing before the kept cells are thrown away, in degrees.
///
/// See this module's doc: it is the angle at which the sun's own movement is comparable to the
/// stencil's resolution, so a kept cell is stale by less than the solver's own quantisation.
pub const BEAM_STEP_DEG: f64 = 0.5;

/// One cell's address: its chunk, and its offset inside it.
///
/// The offset is a `u16` because [`CHUNK_SIDE`] is 32 — `32 · 32 · 32` is 32768, which fits — and
/// it is computed from a *voxel* position rather than a float so that two corners on either side
/// of a voxel plane get the same key, and two corners that differ only in the last bits of a
/// float do not get different ones.
type CellKey = (i32, i32, i32, u16);

/// The chunk a world point is in. Euclidean, as everywhere in `voxel-model`.
pub(crate) fn chunk_of(point_m: [f64; 3]) -> ChunkPos {
    let mut out = [0i32; 3];
    for axis in 0..3 {
        let v = (point_m[axis] / CHUNK_SIDE as f64).floor();
        // A non-finite point has no chunk; `i32::MIN` is a key that no real chunk holds, so the
        // provider answers `None` and the cell reads as unmeasured. Refusing here rather than
        // panicking keeps a NaN corner from taking the frame down.
        out[axis] = if v.is_finite() {
            v.clamp(i32::MIN as f64 + 1.0, i32::MAX as f64 - 1.0) as i32
        } else {
            i32::MIN
        };
    }
    ChunkPos::new(out[0], out[1], out[2])
}

/// The cell a world point stands in, and the fields used to name it.
fn key_of(cell: [i32; 3]) -> CellKey {
    let local = [
        (cell[0].rem_euclid(CHUNK_SIDE)) as u32,
        (cell[1].rem_euclid(CHUNK_SIDE)) as u32,
        (cell[2].rem_euclid(CHUNK_SIDE)) as u32,
    ];
    let index = (local[0] + CHUNK_SIDE as u32 * (local[1] + CHUNK_SIDE as u32 * local[2])) as u16;
    (
        cell[0].div_euclid(CHUNK_SIDE),
        cell[1].div_euclid(CHUNK_SIDE),
        cell[2].div_euclid(CHUNK_SIDE),
        index,
    )
}

/// Somewhere to measure from: what the solver is asked about one cell.
pub struct CellField {
    light: WorldLight,
    /// The direction the kept cells were measured with, normalised. `None` until the first
    /// measurement, which is what makes the first frame take the field as it is given.
    toward: Option<[f64; 3]>,
    /// **Which body the kept cells were measured from.** The swing test is a direction test, and
    /// two bodies can share a direction: at the moment the Sun sets and the Moon rises they can
    /// both sit within [`BEAM_STEP_DEG`] of the horizon's same point, and the Sun's beam kept for
    /// the Moon's would be a bright day's light cast by a night's body. So the body is part of
    /// what the cells were measured *against*, exactly as the direction is.
    beam_is_moon: bool,
    /// **The measurement, and the beam direction it was taken under** — per cell, so the staleness
    /// test is this cell's own and not the frame's. A band vector alone would make the step a
    /// property of the field and force the whole map to be dropped to keep it honest.
    cells: HashMap<CellKey, ([f32; 6], [f64; 3])>,
    budget: usize,
    measured: u64,
    reused: u64,
    /// Corners this frame answered from a measurement whose own beam had swung past
    /// [`BEAM_STEP_DEG`]: a *counted* stale answer rather than a silent one.
    stale: u64,
    unmeasured: u64,
    /// Hops whose chunk was absent, summed over this frame's marches: how much of the field is
    /// measuring past a hole in the resident world. Reported rather than hidden, because a field
    /// lit as if an absent chunk were clear air is exactly the plausible wrong answer this
    /// project keeps refusing.
    unresolved: u64,
}

impl CellField {
    /// The frame's field at `tick`, **keeping the cells the previous frame measured when the beam
    /// has not swung past [`BEAM_STEP_DEG`]**.
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
        let toward = light.toward_beam();
        let cells = match previous {
            // **The beam's own step is what the cells are kept for.** The previous field's
            // direction and this one's are dotted, so the swing is exact rather than a
            // subtraction of two angles that a wrap-around could invert.
            Some(before) => match before.toward {
                // **The cells are kept whatever the swing, and this is the change that makes the
                // field cover a frame.** Each cell carries the direction it was measured under and
                // is re-marched on its own ([`CellField::at`]), so a swing no longer empties the map
                // and the frame no longer re-covers the visible world from nothing every second and
                // a half. What is *not* kept is a different body: see the arm below.
                Some(_) if before.beam_is_moon == light.beam_is_moon() => before.cells,
                // **The body changed**, so every kept cell was measured against a different
                // light and none of them is about this one.
                Some(_) => HashMap::new(),
                // A previous field that never measured anything has nothing to keep.
                None => HashMap::new(),
            },
            None => HashMap::new(),
        };
        let beam_is_moon = light.beam_is_moon();
        Some(CellField {
            light,
            toward: Some(toward),
            beam_is_moon,
            cells,
            budget: CELLS_PER_FRAME,
            measured: 0,
            reused: 0,
            stale: 0,
            unmeasured: 0,
            unresolved: 0,
        })
    }

    /// The solver behind this field, for a caller that wants a beam or a sky radiance on its own.
    pub fn light(&self) -> &WorldLight {
        &self.light
    }

    /// **Begin a frame**: the cell budget is restored and the four counters go back to zero.
    ///
    /// The counters are the frame's and not the field's, so a caller that reports "this frame
    /// measured N cells and had to rig M corners" is reading the frame it drew. The **cells** are
    /// not touched: they are what the field is holding, and a frame that began by dropping them
    /// would re-march the whole visible world four times a second for nothing.
    pub fn begin_frame(&mut self) {
        self.budget = CELLS_PER_FRAME;
        self.measured = 0;
        self.reused = 0;
        self.stale = 0;
        self.unmeasured = 0;
        self.unresolved = 0;
    }

    /// **One cell's six bands**, measured if the frame's budget allows it.
    ///
    /// `None` is "the frame has no measurement here": the cell's own chunk is not resident, or
    /// the budget is spent. Either way the caller's fallback is the rig, which is a *different*
    /// light and a declared one, rather than a zero that would read as "this cell is in the
    /// dark".
    pub fn at(&mut self, trees: &dyn ChunkTrees, cell: [i32; 3]) -> Option<[f32; 6]> {
        let key = key_of(cell);
        // The beam this frame's cells are measured against. `None` only for a field that was never
        // given a direction, which `of` cannot build — and then there is no beam to measure.
        let toward = self.toward?;
        if let Some(&(bands, measured_at)) = self.cells.get(&key) {
            // **This cell's own swing**, dotted so the test is one subtraction rather than two
            // angles a wrap-around could invert.
            let swing = (measured_at[0] * toward[0]
                + measured_at[1] * toward[1]
                + measured_at[2] * toward[2])
                .clamp(-1.0, 1.0)
                .acos()
                .to_degrees();
            if swing <= BEAM_STEP_DEG {
                self.reused += 1;
                return Some(bands);
            }
            if self.budget == 0 {
                // **Answered, and counted as stale.** The alternative the field used to take —
                // drop the cell and let the caller fall back to the rig — replaced the world's own
                // light with a different one for most of the frame every second and a half.
                self.stale += 1;
                return Some(bands);
            }
        } else if self.budget == 0 {
            self.unmeasured += 1;
            return None;
        }
        self.budget -= 1;
        let centre = [
            cell[0] as f64 + 0.5,
            cell[1] as f64 + 0.5,
            cell[2] as f64 + 0.5,
        ];
        let (bands, absent) = self.light.bands_across(trees, centre, REACH_M)?;
        self.unresolved += absent as u64;
        self.cells.insert(key, (bands, toward));
        self.measured += 1;
        Some(bands)
    }

    /// Corners this frame answered from a measurement whose own beam has since swung past
    /// [`BEAM_STEP_DEG`] — the field's own statement of how far behind the light the frame is.
    pub fn stale(&self) -> u64 {
        self.stale
    }

    /// Cells measured in this frame.
    pub fn measured(&self) -> u64 {
        self.measured
    }

    /// Cells answered from the frame's own measurements without marching again.
    pub fn reused(&self) -> u64 {
        self.reused
    }

    /// Corners that fell back to the rig: the budget ran out, or the chunk was not resident.
    pub fn unmeasured(&self) -> u64 {
        self.unmeasured
    }

    /// Hops this frame's marches made past a chunk the streamer does not hold.
    pub fn unresolved(&self) -> u64 {
        self.unresolved
    }

    /// How many cells this field is holding, kept or measured this frame.
    pub fn held(&self) -> usize {
        self.cells.len()
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

/// The closure [`crate::scene::Scene::build_with_light`] takes, over a field and a world.
///
/// A free function rather than a method so the field can be borrowed mutably inside the closure
/// while the world is borrowed immutably — the two borrows are what the scene build needs and a
/// method on `CellField` cannot express them both.
pub fn light_at<'a>(
    field: &'a std::cell::RefCell<CellField>,
    trees: &'a dyn ChunkTrees,
) -> impl Fn(i32, i32, i32) -> Option<[f32; 6]> + 'a {
    move |x, y, z| field.borrow_mut().at(trees, [x, y, z])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cell_key_is_one_key_across_its_own_chunk_boundaries() {
        // The defect this refuses: a key built from a *position* rather than an address.
        // `(0, 0, 0)` and `(31, 31, 31)` are different cells of the same chunk and must have
        // different keys; `(31, 0, 0)` and `(31, 0, 0)` reached from a float that landed a
        // billionth under 31.0 must have the same one. The index does the second half: it is
        // computed from integers, so no two floats that floor to the same voxel can disagree.
        let a = key_of([0, 0, 0]);
        let b = key_of([31, 31, 31]);
        assert_ne!(a, b, "two cells of one chunk shared a key");
        assert_eq!(a.0, b.0, "and they are the same chunk");
        assert_eq!(a.3, 0, "the chunk's first cell is the chunk's first index");
        assert_eq!(b.3, (31 + 32 * (31 + 32 * 31)) as u16, "and its last cell is its last");
        // The next chunk's first cell is the same *local* address by construction — the local
        // index of `(0, 31, 31)` — and a different chunk, so the two keys differ. This is the
        // half that a key built from a world position would get wrong: `(32, 31, 31)` and
        // `(31, 31, 31)` are one voxel apart and share no field entry.
        let c = key_of([32, 31, 31]);
        assert_eq!(c.3, (0 + 32 * (31 + 32 * 31)) as u16);
        assert_ne!(b.0, c.0, "a chunk boundary is a different chunk");
        assert_ne!(b, c, "and a different key");
    }

    #[test]
    fn a_negative_cell_is_in_the_chunk_that_holds_it() {
        // Euclidean, as `voxel-model`'s own contract says: `(-1, -1, -1)` is the *last* cell of
        // chunk `(-1, -1, -1)` and not the first of chunk `(0, 0, 0)`. The truncating version of
        // this is a field that lights the wrong chunk at every negative coordinate.
        let last = key_of([-1, -1, -1]);
        assert_eq!(last.0, -1);
        assert_eq!(last.3, (31 + 32 * (31 + 32 * 31)) as u16);
        let first = key_of([-32, 0, 0]);
        assert_eq!(first.0, -1);
        assert_eq!(first.3, 0);
    }

    #[test]
    fn a_point_off_the_world_still_names_a_chunk() {
        // **Nothing here may panic on a NaN.** A corner is a float that came out of a mesh, and
        // a non-finite corner is a fact about a broken quad rather than a licence to take the
        // frame down; the **sentinel** chunk is a chunk no world holds, so any provider answers
        // `None` for it and the cell reads as unmeasured — the rig lights it, which is the light
        // this field falls back to everywhere it has nothing to say.
        assert_eq!(chunk_of([f64::NAN, 0.0, 0.0]).x, i32::MIN, "NaN names the sentinel");
        assert_eq!(chunk_of([f64::INFINITY, 0.0, 0.0]).x, i32::MIN, "so does an infinity");
        assert_eq!(chunk_of([f64::NEG_INFINITY, 0.0, 0.0]).x, i32::MIN);
        // A *finite* point off the world's far end is clamped rather than sentinelled: it is a
        // real chunk address, merely one no window reaches, and the clamp is what keeps the
        // `as i32` below from being a saturating conversion whose result depends on the cast.
        // The bound is one chunk inside the representable end — `i32::MAX - 1` and
        // `i32::MIN + 1` — because the point of the clamp is to be *inside* the range a cast can
        // hold, and a bound that sat on the last representable address would be the boundary case
        // it exists to avoid.
        assert_eq!(chunk_of([1.0e30, 0.0, 0.0]).x, i32::MAX - 1);
        assert_eq!(chunk_of([-1.0e30, 0.0, 0.0]).x, i32::MIN + 1);
        // And the ordinary cases, around zero: 0.0 through 31.999 is chunk 0.
        assert_eq!(chunk_of([0.0, 0.0, 0.0]).x, 0);
        assert_eq!(chunk_of([31.999, 0.0, 0.0]).x, 0);
        assert_eq!(chunk_of([32.0, 0.0, 0.0]).x, 1);
        assert_eq!(chunk_of([-0.001, 0.0, 0.0]).x, -1);
    }

    /// **A frame's coverage survives the sun's motion, and a stale answer is counted rather than
    /// silent.** These are the three ways the cache can answer, and each is one arm of
    /// [`CellField::at`]:
    ///
    /// * a cell whose *own* beam has not moved is answered from the frame's own measurement;
    /// * a cell whose beam **has** swung is re-marched when the budget allows, and otherwise
    ///   answered from its last measurement **and counted** ([`CellField::stale`]) — it is *not*
    ///   dropped. Dropping was what emptied the whole map every time the Sun moved half a degree
    ///   (about 1.7 s of this world's clock, `0.015°/tick` at `TICKS_PER_DAY = 24_000`), which left
    ///   most of a re-covering frame on the rig;
    /// * a change of **body** does drop the map, because the Moon's beam is not a stale Sun's.
    #[test]
    fn the_fields_coverage_survives_a_beam_swing_and_a_stale_answer_is_counted() {
        /// One chunk of clear air, and nothing outside it — a resident tree is all this test needs,
        /// because what it measures is the cache's own bookkeeping and not a radiance.
        struct Air {
            tree: SparseVoxelOctree,
            chunk: ChunkPos,
        }
        impl ChunkTrees for Air {
            fn tree_at(&self, pos: ChunkPos) -> Option<&SparseVoxelOctree> {
                (pos == self.chunk).then_some(&self.tree)
            }
        }

        let world = WorldSpec::new(2468, voxel_shapers::Config::default());
        let inputs = SkyInputs::default();
        let trees = Air { tree: SparseVoxelOctree::with_levels(5), chunk: ChunkPos::new(0, 0, 0) };
        // A cell of the one resident chunk, per index: `x` and `y` walk the chunk and `z` changes
        // with the pass, so a range of cells can be asked without ever repeating one.
        let cell_of = |index: usize, z: i32| {
            [(index % 32) as i32, ((index / 32) % 32) as i32, z]
        };

        // Noon, so the beam has a direction and a swing between two ticks is the Sun's own motion.
        let noon = voxel_shapers::climate::TICKS_PER_DAY / 2;
        let mut field = CellField::of(&world, noon, inputs, 0, None).expect("a field");
        field.begin_frame();
        for index in 0..CELLS_PER_FRAME {
            assert!(field.at(&trees, cell_of(index, 0)).is_some(), "a resident cell");
        }
        assert_eq!(field.measured(), CELLS_PER_FRAME as u64, "the frame marched its budget");
        // The same cells again, at the same tick: the frame's own measurements, no marches.
        field.begin_frame();
        for index in 0..CELLS_PER_FRAME {
            field.at(&trees, cell_of(index, 0)).expect("answered from the frame's own measurements");
        }
        assert_eq!(field.measured(), 0, "a repeated ask marched the octree again");
        assert_eq!(field.reused(), CELLS_PER_FRAME as u64);
        assert_eq!(field.stale(), 0);
        let early = field.toward_beam().expect("a beam direction");

        // **Forty ticks on.** The swing is *checked rather than assumed*: if the clock's rate ever
        // changes, this test says so instead of quietly testing a stationary sun.
        let mut swung =
            CellField::of(&world, noon + 40, inputs, 0, Some(field)).expect("a field at the later tick");
        let later = swung.toward_beam().expect("a beam direction");
        let swing = (early[0] * later[0] + early[1] * later[1] + early[2] * later[2])
            .clamp(-1.0, 1.0)
            .acos()
            .to_degrees();
        assert!(
            swing > BEAM_STEP_DEG,
            "forty ticks moved the beam {swing}°, which is not past the {BEAM_STEP_DEG}° step"
        );
        assert_eq!(swung.held(), CELLS_PER_FRAME, "the swing emptied the map");
        // **A budget already spent, and a cell kept from the earlier beam.** This frame marches
        // cells the map does not hold until the budget is gone, and then asks one the map *does*
        // hold — under a beam it is no longer the light. The arm this exercises is the one that used
        // to be a *drop*, and therefore a corner handed back to the rig.
        swung.begin_frame();
        for index in 0..CELLS_PER_FRAME {
            swung.at(&trees, cell_of(index, 3)).expect("a fresh cell");
        }
        let answered = swung
            .at(&trees, cell_of(0, 0))
            .expect("a swung cell with no budget left is answered, not dropped");
        assert!(
            answered.iter().all(|band| band.is_finite()),
            "a stale answer is not a measurement: {answered:?}"
        );
        assert_eq!(swung.stale(), 1, "a stale answer went uncounted");
        assert_eq!(swung.unmeasured(), 0, "a fresh cell was rigged with the budget still unspent");
        // **And the same cells are re-marched when the frame has the budget for them** — so the
        // coverage is not merely kept, it is brought up to the current light.
        swung.begin_frame();
        for index in 0..CELLS_PER_FRAME {
            swung.at(&trees, cell_of(index, 0)).expect("re-marched under the new beam");
        }
        assert_eq!(swung.measured(), CELLS_PER_FRAME as u64, "a swung cell was not re-marched");
        assert_eq!(swung.stale(), 0, "a cell was called stale with the budget still unspent");

        // **And a different body is a different light.** At tick zero the Sun is down, so the
        // field's beam is the Moon's — not the Sun's by however many degrees — and every kept cell
        // is discarded rather than re-labelled.
        let night = CellField::of(&world, 0, inputs, 0, Some(swung)).expect("a field at midnight");
        assert!(
            night.held() == 0,
            "{} cell(s) measured against the Sun's beam were kept for the Moon's",
            night.held()
        );
    }
}
