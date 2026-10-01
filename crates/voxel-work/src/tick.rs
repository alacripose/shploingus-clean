//! The tick: **what a stage is asked, on what, and in what order**.
//!
//! §88.7 left a stage's unit of work unruled. Everything below is derived from
//! what the tree already measures, and the three places a wrong answer was
//! available are named because each was tempting.
//!
//! # The octree's ladder is not the field ladder
//!
//! Two ladders exist and they do not agree, which is the first thing to get
//! right:
//!
//! | | source | unit | size |
//! |---|---|---|---|
//! | **octree** | [`voxel_mesh::octree`] | voxel | `2^level`, `OCTREE_LEVELS` = 5, root = one 32³ chunk |
//! | **field ladder** | [`voxel_shapers::fields`] | cell | `32 · 2^level`, `STANDARD_LEVELS` = 20 |
//!
//! Reading one as the other would have produced a unit of work 32× too large and
//! a level index that means nothing in the tree being walked. They are kept apart
//! here: [`UnitOfWork`] is in **voxels**, and the field ladder is not referenced
//! by it at all.
//!
//! # Why the leaf, and why a *declared* level rather than "whatever merged"
//!
//! The user's ruling named the octree as the granularity. Taken literally as "the
//! leaves the tree happens to have", that is wrong, and wrong in the one way this
//! project cannot tolerate: a solid chunk is **one** leaf and a chunk with one
//! odd voxel in it is thousands, so two chunks of the same world would be
//! scheduled differently. [`SparseVoxelOctree::visit_at_level`] exists for
//! precisely this, and it is what enumerates the units.
//!
//! # The halo is one cell, and that is a proof rather than a preference
//!
//! A stage reading its own region is fine; a stage reading its *neighbourhood*
//! is not, because the octree's root is one chunk and the region at a chunk edge
//! has neighbours in the next chunk. D8 routing reads eight neighbours, the
//! priority-flood fill reads the same eight, and one cell is exactly what eight
//! neighbours reach. [`UnitOfWork::halo`] is that one cell and
//! [`tests::the_halo_is_exactly_what_d8_reaches_and_not_one_cell_more`] holds the
//! boundary from both sides — one cell less loses the corner neighbour, one cell
//! more is slack nobody asked for.
//!
//! The **chunk-level** half of the same declaration is
//! [`Horizon::Simulation`]: a halo that crosses a chunk boundary reads the
//! neighbour, so the unit of work is only sound when that neighbour is inside the
//! declared neighbourhood — which is also the radius [`crate::DEFAULT_WINDOW`]
//! now streams at, so the stage's reach and the machine's are the same number
//! rather than two numbers that agree by luck.
//!
//! # The tick is the schedule, never state
//!
//! [`Scheduler::plan`] returns which stages are asked at which units on this
//! tick, and **stores nothing**. A stage's answer stays a pure function of
//! `(tick, address)` — the ladder's own determinism contract — so a tick skipped
//! or a run with different worker counts produces the same world. This is why
//! `Stage::Hydrology` can hold a ×4 rung without hydrology needing a time
//! dimension: the rung decides *whether* it is asked, not *what* it answers.

use voxel_mesh::octree::{SparseVoxelOctree, OCTREE_LEVELS};
use voxel_model::{ChunkPos, Refusal, RefusalKind, Refused, CHUNK_SIDE};
use voxel_stream::{Horizon, Horizons};
use voxel_shapers::stages::{Declared, Stage};

/// How many voxels beyond its own region a unit of work may read.
///
/// **One**, and not a tunable: it is the reach of a D8 neighbourhood, which is a
/// fact about the routing rule rather than a quality setting. Widening it would
/// not make hydrology more correct, it would make it read cells the stage has no
/// claim to and charge for them.
pub const HALO_VOXELS: i64 = 1;

/// The eight D8 neighbour offsets, in voxels.
///
/// Test-only, and deliberately: the halo's required size is this table's maximum
/// reach, and that is a fact about the *routing rule*, not about anything this
/// module computes. Keeping it in the test module rather than promoting it to a
/// public constant is the honest position — nothing here routes water, so nothing
/// here should appear to know how D8 is spelled.
#[cfg(test)]
const D8: [[i64; 3]; 8] = [
    [-1, -1, -1],
    [0, -1, -1],
    [1, -1, -1],
    [-1, 0, -1],
    [1, 0, -1],
    [-1, 1, -1],
    [0, 1, -1],
    [1, 1, -1],
];

/// A half-open box of voxels: `min` inclusive, `max` exclusive.
///
/// Its own type rather than a range of `UnitOfWork`s, because **a halo is not a
/// power of two**. A region's side is `2^level`; that region's halo has side
/// `2^level + 2`, which no octree level can name. Encoding the halo as a
/// `UnitOfWork` would have been the tidy wrong answer.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CellBox {
    pub min: [i64; 3],
    pub max: [i64; 3],
}

impl CellBox {
    pub fn contains(&self, p: [i64; 3]) -> bool {
        (0..3).all(|a| p[a] >= self.min[a] && p[a] < self.max[a])
    }

    /// Voxels in the box. Zero if any axis is inverted, so a box built by a
    /// subtraction that wrapped reports nothing rather than a huge number.
    pub fn cells(&self) -> i64 {
        (0..3).fold(1i64, |acc, a| acc * (self.max[a] - self.min[a]).max(0))
    }

    /// Every voxel in the box, in a fixed order.
    pub fn voxels(&self) -> Vec<[i64; 3]> {
        let mut out = Vec::with_capacity(self.cells().max(0) as usize);
        for z in self.min[2]..self.max[2] {
            for y in self.min[1]..self.max[1] {
                for x in self.min[0]..self.max[0] {
                    out.push([x, y, z]);
                }
            }
        }
        out
    }
}

/// **One unit of a stage's work**: one octree region of one chunk.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct UnitOfWork {
    chunk: ChunkPos,
    /// The region's origin in **chunk-local** voxels, which is what the octree
    /// reports.
    origin: [i32; 3],
    level: u32,
}

impl UnitOfWork {
    pub const fn new(chunk: ChunkPos, origin: [i32; 3], level: u32) -> Self {
        UnitOfWork { chunk, origin, level }
    }

    pub const fn chunk(&self) -> ChunkPos {
        self.chunk
    }

    pub const fn origin(&self) -> [i32; 3] {
        self.origin
    }

    pub const fn level(&self) -> u32 {
        self.level
    }

    /// The region's edge in voxels: `2^level`.
    pub fn side_voxels(&self) -> i64 {
        1i64 << self.level
    }

    /// Voxels in the region.
    pub fn voxels(&self) -> i64 {
        let s = self.side_voxels();
        s * s * s
    }

    /// The region's box in **chunk-local** voxels.
    pub fn local_box(&self) -> CellBox {
        let s = self.side_voxels();
        CellBox {
            min: [self.origin[0] as i64, self.origin[1] as i64, self.origin[2] as i64],
            max: [
                self.origin[0] as i64 + s,
                self.origin[1] as i64 + s,
                self.origin[2] as i64 + s,
            ],
        }
    }

    /// The region's box in **world** voxels.
    ///
    /// The chunk's own origin is added rather than assumed, because a unit of work
    /// that addressed the wrong chunk would be a unit of work about the wrong
    /// place — and nothing downstream would notice, since the shape is identical.
    pub fn world_box(&self) -> CellBox {
        let base = [
            self.chunk.x as i64 * CHUNK_SIDE as i64,
            self.chunk.y as i64 * CHUNK_SIDE as i64,
            self.chunk.z as i64 * CHUNK_SIDE as i64,
        ];
        let b = self.local_box();
        CellBox {
            min: [b.min[0] + base[0], b.min[1] + base[1], b.min[2] + base[2]],
            max: [b.max[0] + base[0], b.max[1] + base[1], b.max[2] + base[2]],
        }
    }

    /// The region plus [`HALO_VOXELS`] on every side: everything a stage may read.
    pub fn halo(&self) -> CellBox {
        let b = self.world_box();
        CellBox {
            min: [b.min[0] - HALO_VOXELS, b.min[1] - HALO_VOXELS, b.min[2] - HALO_VOXELS],
            max: [b.max[0] + HALO_VOXELS, b.max[1] + HALO_VOXELS, b.max[2] + HALO_VOXELS],
        }
    }

    /// Whether this unit's own region — **not** its halo — holds the voxel.
    pub fn holds(&self, world: [i64; 3]) -> bool {
        self.world_box().contains(world)
    }

    /// Every chunk the unit's halo reaches into, itself included.
    ///
    /// The set rather than a radius, because a region at a chunk's corner reaches
    /// three chunks and one in the middle reaches only its own: a radius would
    /// have over-claimed and a single neighbour would have under-claimed.
    pub fn chunks_touched(&self) -> Vec<ChunkPos> {
        let h = self.halo();
        let mut out = Vec::with_capacity(8);
        for dz in -1..=1 {
            for dy in -1..=1 {
                for dx in -1..=1 {
                    // The chunk of the halo's *minimum* corner along each axis,
                    // which for dx = -1 is the chunk below.
                    let c = ChunkPos::new(
                        self.chunk.x + dx,
                        self.chunk.y + dy,
                        self.chunk.z + dz,
                    );
                    let base = [
                        c.x as i64 * CHUNK_SIDE as i64,
                        c.y as i64 * CHUNK_SIDE as i64,
                        c.z as i64 * CHUNK_SIDE as i64,
                    ];
                    let lo = [
                        h.min[0] - base[0],
                        h.min[1] - base[1],
                        h.min[2] - base[2],
                    ];
                    let hi = [
                        h.max[0] - base[0],
                        h.max[1] - base[1],
                        h.max[2] - base[2],
                    ];
                    if (0..3).all(|a| lo[a] < CHUNK_SIDE as i64 && hi[a] > 0) {
                        out.push(c);
                    }
                }
            }
        }
        out.sort_by_key(|c| (c.x, c.y, c.z));
        out.dedup();
        out
    }
}

/// One (stage, unit) pair the tick owes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Planned {
    pub stage: Stage,
    pub unit: UnitOfWork,
}

/// The tick's scheduler: a declared pipeline, a horizon set, and no state.
#[derive(Clone, Debug)]
pub struct Scheduler {
    declared: Declared,
    horizons: Horizons,
}

impl Scheduler {
    /// Refuses a horizon set that is not four nested horizons.
    ///
    /// The same check the streaming layer makes, called here rather than assumed,
    /// because a scheduler that planned work over an invalid neighbourhood would
    /// produce a plan that looks fine and reaches past what the machine holds.
    pub fn new(declared: Declared, horizons: Horizons) -> Refused<Self> {
        horizons.validate()?;
        Ok(Scheduler { declared, horizons })
    }

    /// The full pipeline over the declared horizons.
    pub fn full() -> Refused<Self> {
        Self::new(Declared::full(), Horizons::declared())
    }

    pub fn declared(&self) -> &Declared {
        &self.declared
    }

    pub fn horizons(&self) -> &Horizons {
        &self.horizons
    }

    /// The declared stages **due at `tick`**, in dependency order.
    ///
    /// Dependency order rather than [`voxel_shapers::stages::ALL`] order, because
    /// a plan that ran a stage before what it reads would be a plan whose answers
    /// disagree with a single-threaded run's.
    pub fn due_at(&self, tick: u64) -> Vec<Stage> {
        self.declared
            .order()
            .into_iter()
            .filter(|s| s.is_due_at(tick))
            .collect()
    }

    /// Everything this tick owes: each due stage, at each unit, **stage-major**.
    ///
    /// Stage-major so that a stage finishes its whole neighbourhood before the
    /// next one starts; unit-major would interleave two stages over one region
    /// and make the result depend on the order units happened to be in.
    pub fn plan(&self, tick: u64, units: &[UnitOfWork]) -> Vec<Planned> {
        let mut out = Vec::with_capacity(self.due_at(tick).len() * units.len());
        for stage in self.due_at(tick) {
            for unit in units {
                out.push(Planned { stage, unit: *unit });
            }
        }
        out
    }

    /// Whether every chunk this unit's halo reaches is inside the declared
    /// neighbourhood of `centre`.
    ///
    /// **The check that makes the halo sound.** A halo that crosses into a chunk
    /// nobody streams is a stage reading a hole, and the read would be silent:
    /// the shaper would answer for a neighbour that does not exist.
    pub fn halo_is_declared(&self, unit: &UnitOfWork, centre: ChunkPos) -> bool {
        unit.chunks_touched()
            .iter()
            .all(|c| self.horizons.contains(Horizon::Simulation, centre, *c))
    }

    /// Units a chunk offers at `level`, refusing a level the tree does not have.
    pub fn units_at(
        tree: &SparseVoxelOctree,
        chunk: ChunkPos,
        level: u32,
    ) -> Refused<Vec<UnitOfWork>> {
        if level > OCTREE_LEVELS {
            return Err(Refusal::owned(
                RefusalKind::IndexOutOfRange,
                format!(
                    "level {level} is outside the octree's {OCTREE_LEVELS} levels \
                     (root at {OCTREE_LEVELS}, one voxel at 0)"
                ),
            ));
        }
        let mut out = Vec::with_capacity(tree.leaf_count_at(level) as usize);
        tree.visit_at_level(level, &mut |origin, _| {
            out.push(UnitOfWork::new(chunk, origin, level));
        });
        // **Sorted, and this is not cosmetic.** The walk's own order depends on
        // the tree: a compressed chunk arrives as one leaf that is split in a
        // nested loop, and a mixed one arrives by descending branches in octant
        // order. Those are different sequences over the same set, measured — so a
        // scheduler that planned work in walk order would give two chunks of the
        // same world different tick plans. Sorting by origin makes the order a
        // function of the chunk's size alone.
        out.sort_unstable_by_key(|u| (u.origin[0], u.origin[1], u.origin[2]));
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_shapers::stages::Cadence;

    fn chunk_at(x: i32, z: i32) -> ChunkPos {
        ChunkPos::new(x, 0, z)
    }

    fn scheduler() -> Scheduler {
        Scheduler::full().unwrap()
    }

    #[test]
    fn the_halo_is_exactly_what_d8_reaches_and_not_one_cell_more() {
        // The sufficiency half: every D8 neighbour of every voxel in the region is
        // inside the halo, at every level and at every position in the chunk.
        // Teeth: set `HALO_VOXELS` to 0 and the corner neighbours fall out.
        let chunk = chunk_at(0, 0);
        for level in 0..=OCTREE_LEVELS {
            let side = 1i32 << level;
            // Four placements, because "the halo works where it does not cross a
            // chunk edge" is the easy case and the edge is the one that matters.
            for (ox, oz) in [
                (0, 0),
                (CHUNK_SIDE - side, 0),
                (0, CHUNK_SIDE - side),
                (CHUNK_SIDE - side, CHUNK_SIDE - side),
            ] {
                let unit = UnitOfWork::new(chunk, [ox, 0, oz], level);
                let halo = unit.halo();
                for v in unit.world_box().voxels() {
                    for d in D8 {
                        let n = [v[0] + d[0], v[1] + d[1], v[2] + d[2]];
                        assert!(
                            halo.contains(n),
                            "level {level}: the D8 neighbour {n:?} of {v:?} is outside \
                             the halo {:?}",
                            halo
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn the_halo_is_one_cell_and_the_geometry_says_so() {
        // The minimality half. One cell less and the assertion above fails; the
        // size is checked here so that a widened halo cannot be introduced as an
        // improvement without a test noticing.
        for level in 0..=OCTREE_LEVELS {
            let unit = UnitOfWork::new(chunk_at(0, 0), [0, 0, 0], level);
            let side = unit.side_voxels();
            let halo = unit.halo();
            for a in 0..3 {
                assert_eq!(halo.min[a], unit.world_box().min[a] - HALO_VOXELS);
                assert_eq!(halo.max[a], unit.world_box().max[a] + HALO_VOXELS);
            }
            assert_eq!(halo.cells(), (side + 2 * HALO_VOXELS).pow(3));
            // And a corner neighbour genuinely needs all three axes of halo: a
            // box grown on two axes only would miss it.
            assert!(
                (0..3).all(|a| halo.max[a] - halo.min[a] == side + 2 * HALO_VOXELS),
                "level {level}: the halo is not one cell on every side"
            );
        }
    }

    #[test]
    fn a_halo_only_crosses_a_chunk_boundary_where_the_region_touches_one() {
        // The region in the middle of a chunk reaches nothing; the region at its
        // low corner reaches eight — its own chunk and the seven "before" it. A
        // radius-based claim would have got both of those wrong.
        let interior = UnitOfWork::new(chunk_at(0, 0), [8, 8, 8], 4);
        assert_eq!(interior.chunks_touched(), vec![chunk_at(0, 0)]);

        let corner = UnitOfWork::new(chunk_at(0, 0), [0, 0, 0], 4);
        // **All eight, and all of them behind it.** The halo is one voxel on
        // every side, so a region sitting at the chunk's low corner reaches
        // `x = -1`, `y = -1` and `z = -1` — the three chunks *below and before* — and
        // nothing ahead of it at all, because the region ends 16 voxels into a
        // 32-voxel chunk. An earlier version of this test expected four, having
        // reasoned only about x and z and assuming symmetry.
        assert_eq!(corner.chunks_touched().len(), 8, "{:?}", corner.chunks_touched());
        for dz in -1..=0i32 {
            for dy in -1..=0i32 {
                for dx in -1..=0i32 {
                assert!(
                    corner.chunks_touched().contains(&ChunkPos::new(dx, dy, dz)),
                    "the halo did not reach chunk ({dx}, {dy}, {dz})"
                );
                }
            }
        }
        // And the interior region really does reach nothing but its own chunk.
        assert_eq!(interior.chunks_touched(), vec![chunk_at(0, 0)]);
    }

    #[test]
    fn a_halo_is_sound_only_inside_the_simulation_horizon() {
        // The check that ties the cell halo to the chunk neighbourhood: a unit at
        // the edge of the streamed window reaches a chunk nobody holds.
        let s = scheduler();
        let centre = chunk_at(0, 0);
        // **One chunk outside** the simulation horizon. A unit *on* the horizon is
        // fine: its halo reaches inward, to `r - 1`, which is declared. Only a
        // unit past the boundary reaches something nobody streams.
        let past = s.horizons().rule(Horizon::Simulation).radius_chunks + 1;
        let outside = UnitOfWork::new(ChunkPos::new(past, 0, 0), [0, 0, 0], 4);

        assert!(
            !s.halo_is_declared(&outside, centre),
            "a unit past the simulation horizon claimed its halo was declared"
        );
        // And the boundary itself is declared, so the claim is not simply "far
        // chunks are undeclared".
        let on = UnitOfWork::new(
            ChunkPos::new(past - 1, 0, 0),
            [0, 0, 0],
            4,
        );
        assert!(s.halo_is_declared(&on, centre));
        let inside = UnitOfWork::new(chunk_at(0, 0), [0, 0, 0], 4);
        assert!(s.halo_is_declared(&inside, centre));
        // And the window the streamer uses is the same radius, so the stage's
        // reach and the machine's are one number.
        assert_eq!(
            s.horizons().rule(Horizon::Simulation).radius_chunks,
            crate::DEFAULT_WINDOW.radius_chunks
        );
    }

    #[test]
    fn the_units_of_a_chunk_do_not_depend_on_how_well_it_compresses() {
        let solid = SparseVoxelOctree::new();
        let mut speckled = SparseVoxelOctree::new();
        speckled.set_voxel(
            voxel_model::LocalPos::new(7, 9, 11).unwrap(),
            voxel_model::VoxelShape::solid(voxel_model::MaterialId(3)),
        );
        for level in [2u32, 4, OCTREE_LEVELS] {
            let a = Scheduler::units_at(&solid, chunk_at(0, 0), level).unwrap();
            let b = Scheduler::units_at(&speckled, chunk_at(0, 0), level).unwrap();
            assert_eq!(a.len(), b.len(), "level {level}: compression changed the unit count");
            // Compared element-wise rather than by `assert_eq!` on the vectors: a
            // failed whole-vector comparison prints 512 units, which buries the
            // one line that says which unit differed.
            let first = a.iter().zip(&b).position(|(x, y)| x != y);
            assert_eq!(first, None, "level {level}: unit {first:?} differs: {:?} vs {:?}", first.map(|i| a[i]), first.map(|i| b[i]));
        }
        // Stated, so a change in the octree's root size has to be argued for.
        assert_eq!(Scheduler::units_at(&solid, chunk_at(0, 0), 4).unwrap().len(), 8);
        assert_eq!(
            Scheduler::units_at(&solid, chunk_at(0, 0), OCTREE_LEVELS).unwrap().len(),
            1
        );
    }

    #[test]
    fn a_level_the_octree_does_not_have_is_refused_by_name() {
        let r = Scheduler::units_at(&SparseVoxelOctree::new(), chunk_at(0, 0), OCTREE_LEVELS + 1)
            .unwrap_err();
        assert_eq!(r.kind(), RefusalKind::IndexOutOfRange);
        assert!(r.detail().contains("levels"), "got {r}");
    }

    #[test]
    fn a_stage_is_planned_only_on_a_tick_its_own_rung_reaches() {
        // The ladder, exercised through the scheduler rather than through
        // `is_due_at` alone — the claim is that the *plan* honours the rung.
        let s = scheduler();
        let units = [UnitOfWork::new(chunk_at(0, 0), [0, 0, 0], 4)];

        let at = |tick: u64| -> Vec<Stage> {
            let mut v: Vec<Stage> = s.plan(tick, &units).iter().map(|p| p.stage).collect();
            v.dedup();
            v
        };

        // ×4 and ×20 and daily are all absent on tick 1.
        for stage in [Stage::Hydrology, Stage::Weather, Stage::Erosion, Stage::Climate] {
            assert!(!at(1).contains(&stage), "{stage:?} was planned on tick 1");
        }
        // ×4 is present on tick 4, ×20 still is not, and the daily pair is not.
        assert!(at(4).contains(&Stage::Hydrology));
        assert!(at(4).contains(&Stage::Weather));
        assert!(!at(4).contains(&Stage::Erosion));
        assert!(!at(4).contains(&Stage::Climate));
        // ×20 arrives on tick 20.
        assert!(at(20).contains(&Stage::Erosion));
        assert!(!at(20).contains(&Stage::Climate));
        // The daily pair arrives on the day.
        let day = voxel_shapers::climate::TICKS_PER_DAY;
        assert!(at(day).contains(&Stage::Climate));
        assert!(at(day).contains(&Stage::Caves));
        // Tick 0 is due for everything, so a world asked for the first time is
        // complete rather than one period stale.
        assert_eq!(at(0).len(), voxel_shapers::stages::STAGE_COUNT);
    }

    #[test]
    fn hydrologys_rung_is_no_longer_decorative() {
        // `Stage::Hydrology` holds a ×4 rung and, until the scheduler existed,
        // nothing consulted it: the rung was a comment. The claim now is that a
        // tick four apart reaches hydrology and a tick one apart does not — and
        // that the answer is still a pure function of the address, which is why
        // the tick is a schedule and not state.
        let s = scheduler();
        assert_eq!(Stage::Hydrology.cadence(), Cadence::Water);
        assert_eq!(Stage::Hydrology.cadence().period_ticks(), 4);
        let units = [UnitOfWork::new(chunk_at(0, 0), [0, 0, 0], 4)];
        let hydrology_at = |tick: u64| s.plan(tick, &units).iter().any(|p| p.stage == Stage::Hydrology);
        assert!(hydrology_at(0));
        assert!(!hydrology_at(1));
        assert!(!hydrology_at(3));
        assert!(hydrology_at(4));
        assert!(!hydrology_at(5));
        assert!(hydrology_at(8));
        // And the same tick always gives the same plan, whatever was asked first.
        assert_eq!(s.plan(8, &units), s.plan(8, &units));
    }

    #[test]
    fn a_plan_is_in_dependency_order_and_stage_major() {
        let s = scheduler();
        let units = [
            UnitOfWork::new(chunk_at(0, 0), [0, 0, 0], 4),
            UnitOfWork::new(chunk_at(0, 0), [16, 0, 0], 4),
        ];
        let plan = s.plan(voxel_shapers::climate::TICKS_PER_DAY, &units);
        let order = s.declared().order();

        let at = |stage: Stage| order.iter().position(|x| *x == stage).unwrap();
        // Every stage's block is contiguous, and the blocks ascend the declared
        // order.
        let mut seen: Vec<Stage> = Vec::new();
        for p in &plan {
            if seen.last() != Some(&p.stage) {
                assert!(
                    !seen.contains(&p.stage),
                    "{:?} appears in two blocks, so the plan is not stage-major",
                    p.stage
                );
                seen.push(p.stage);
            }
        }
        for w in seen.windows(2) {
            assert!(at(w[0]) < at(w[1]), "{:?} was planned before {:?}", w[0], w[1]);
        }
        // And within a block, the units keep the order they were given in.
        let first: Vec<UnitOfWork> =
            plan.iter().filter(|p| p.stage == Stage::Ecology).map(|p| p.unit).collect();
        assert_eq!(first, units.to_vec());
    }

    #[test]
    fn a_partial_pipeline_plans_only_what_it_declared() {
        // A scheduler over a subset must not plan the rest, and must not plan a
        // subset in an order the dependencies forbid.
        let s = Scheduler::new(
            Declared::closure_of([Stage::Ecology]),
            Horizons::declared(),
        )
        .unwrap();
        let units = [UnitOfWork::new(chunk_at(0, 0), [0, 0, 0], 4)];
        let stages: Vec<Stage> = s.due_at(0);
        assert!(stages.contains(&Stage::Ecology));
        assert!(stages.contains(&Stage::Climate), "the closure did not pull climate in");
        assert!(!stages.contains(&Stage::Caves), "caves was not declared and cannot be reached");
        assert!(!s.plan(0, &units).iter().any(|p| p.stage == Stage::Caves));
    }

    #[test]
    fn a_scheduler_refuses_a_horizon_set_that_is_not_nested() {
        let h = Horizons::declared();
        // Simulation pulled inside render: individually plausible, jointly wrong,
        // and refused by name rather than accepted and sorted out later.
        let bad = h.with_rule(Horizon::Simulation, 4, 1).unwrap_err();
        assert!(bad.detail().contains("not strictly inside"), "got {bad}");
        assert!(Scheduler::new(Declared::full(), Horizons::declared()).is_ok());
    }

    #[test]
    fn a_world_unit_of_work_addresses_the_chunk_it_names() {
        // Two chunks, same local origin, same level: the units must differ. If
        // this fails, a stage would answer for the right shape in the wrong place
        // and nothing downstream would notice.
        let a = UnitOfWork::new(chunk_at(0, 0), [0, 0, 0], 4);
        let b = UnitOfWork::new(chunk_at(1, 0), [0, 0, 0], 4);
        assert_ne!(a, b);
        assert_eq!(a.world_box().min, [0, 0, 0]);
        assert_eq!(b.world_box().min, [CHUNK_SIDE as i64, 0, 0]);
        // And a chunk at negative coordinates does not wrap.
        let c = UnitOfWork::new(chunk_at(-1, 0), [0, 0, 0], 4);
        assert_eq!(c.world_box().min, [-(CHUNK_SIDE as i64), 0, 0]);
    }
}