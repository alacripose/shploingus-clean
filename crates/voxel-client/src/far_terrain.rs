//! **The far field: the world beyond the residency window, at eight metres a cell.**
//!
//! The streamed window ends and, until now, so did the world: a chunk that left the window left no
//! geometry behind, so the horizon was sky however far the terrain actually ran. The coarse band
//! already answers `HORIZON_FAR_CELL_M`-sized cells for light; this is the same rung carrying
//! **geometry**, so a frame draws a low-density cubic surface where the fine path has stopped.
//!
//! # One census, two readers
//!
//! A far chunk's surface and its coarse light are the same measurement: each 8 m cell's **solid
//! share out of the 512 voxels it holds**, from the chunk's own subvoxel octree. [`census`] is that
//! one walk. [`coarse_mesh`] draws a cell as solid at a share of one half, and
//! [`crate::horizon::HorizonStream::stamp_far`] writes the same census into the band as a
//! [`crate::horizon::FAR_SHARE`] fraction, so the face a fragment sees and the march that lights it
//! cannot disagree about which cells are rock.
//!
//! # The cell is the mesher's own, at four times the stride
//!
//! The rung is not a second mesher. A quad is still a [`voxel_mesh::Quad`] — a face, a plane and a
//! rectangle in the face's own `(u, v)` basis — and it still comes out as a [`voxel_mesh::Mesh`],
//! so the scene, the record format and `vs_quad` need no new kind of geometry. What differs is the
//! **stride**: a coarse cell is `FAR_CELL_VOXELS` voxels, so every slice, `uv` and extent is a
//! multiple of [`FAR_CELL_SUBVOXELS`] (32 subvoxels — eight metres), and a merged rectangle is a
//! rectangle of those cells. [`crate::scene::Scene::with_far`] carries that stride into the record's
//! spare slot, which is how the material ladder is told to read the rung the face was cut at rather
//! than the subvoxel rung it drew at.
//!
//! # What a cell is, and what is at its boundary
//!
//! A cell with at least half its volume in matter is **solid**; below that it is air. That is a
//! real quantisation, not a smoothed one: a hillside whose surface crosses a cell in its lower half
//! loses that cell, so the far skyline is up to eight metres coarse. The alternative — drawing a
//! cell that is one tenth rock as a full eight-metre stone — raises a second, invented surface, and
//! the band's own threshold already says what half-solid means (`far_solid_at`).
//!
//! **A face between two far chunks is decided from both censuses**, so a chunk boundary has no
//! seam of its own. Where one side's census is not in hand — the ring's outer edge, or the boundary
//! against the streamed window — the face is **not drawn**: the slice ends there rather than raising
//! a wall of eight-metre stones around the world. The window's own outer wall is what the fine
//! mesher draws, and this field starts beyond it.
//!
//! # The ring, and why it is a worker
//!
//! Far chunks are generated from the same [`voxel_stream::WorldSource`] the streamer uses — the
//! generator is the storage, and a second generator would be a second world. Generating one is a
//! tree build (tens of milliseconds) followed by a 64-cell census; the stack is generated on its
//! own thread, in [`FAR_CENSUS_LANES`] lanes, so a crossing does not spend those milliseconds on the
//! frame. **The lanes are a wall-clock lever and nothing else**: a chunk's tree is a pure function
//! of the spec, so the census, the mesh and the band words are identical at any lane count.
//!
//! The field is **`FAR_STACK` layers above and below the eye's chunk**, two chunks deep beyond the streamed
//! window, and every chunk it will need is censused — a ring of margin horizontally and one layer
//! vertically that the mesh never draws — before any is meshed, so a neighbour's boundary is known
//! when a face asks for it.
//!
//! # The flat band at the horizon is the sea, and it is not a stand-in
//!
//! A 60 m frame over the shipped world shows a **flat, uniform band** between the near ground and
//! the horizon. It is water: at the eye's own column the ground reads **13.78 m** against a sea
//! level of **16.00 m** (`terrain::elevation_m`, seed 2468, `Config::default()`), and the same
//! band fills the lower frame when the eye stands over the ocean at z = −1 000 m, 60 m up. Water
//! is level by law, so its merged faces are flat wherever they are drawn — near or far, at any
//! rung — and no plane carrier exists in this module, in [`crate::surface`] or in the renderer to
//! retire. What draws past the window is these tiers and `surface`'s rungs; a frame that shows
//! terrain missing would show **sky**, not a plane (the outer edge draws no wall, by the rule
//! above). The elevations are the evidence, and a caller can reproduce them with
//! `terrain::elevation_m(&WorldSpec::new(2468, Config::default()), 0.5, z)` at z = −100.5 and
//! −1 000.5.
//!
//! **This is a slice, and its edges are named.** The stack is bounded at one chunk each side of the
//! eye because the world's surface spans about −22 … +54 m; a relief more than a chunk past the
//! eye, or a cave deeper than the stack's floor, is not drawn. **One far ring is drawn, at tier 5**
//! ([`crate::lod::FAR_TIER`]); tiers 6 and 7 are cuttable — [`Census::of`] takes a tier and the
//! mesher reads the stride from it — but no light band measures at them yet, so nothing draws them.
//! The far mesh is the generator's own world, so it does not carry
//! [`voxel_shapers::earthworks::Earthworks`] edits — an
//! edit is a record of what the player changed, and the player cannot reach beyond the window this
//! field starts outside of.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};

use voxel_mesh::octree::Node;
use voxel_mesh::{Mesh, MeshStats, Quad, SparseVoxelOctree};
use voxel_model::{ChunkPos, Face, MaterialId, CHUNK_SIDE, SUBVOXEL_SIDE};
use voxel_shapers::WorldSpec;
use voxel_stream::WorldSource;

/// Metres per far cell — the coarse band's own grain ([`crate::horizon::HORIZON_FAR_CELL_M`]).
///
/// One number, two readers. A mesh that quantised at a different grain than the light would draw
/// its cells at heights the march did not agree with. The number itself is owned by
/// [`crate::lod`], so the band, the terrain and the ladder cannot drift apart.
pub const FAR_CELL_M: f64 = crate::lod::FAR_CELL_M as f64;

/// Voxels along a far cell: a chunk is 32 m, so four cells fit one axis.
pub const FAR_CELL_VOXELS: i32 = crate::lod::FAR_CELL_VOXELS;

/// Cells along one axis of a far chunk at the far tier.
pub const FAR_CELLS_SIDE: i32 = crate::lod::FAR_CELLS_PER_CHUNK;

/// Subvoxels along a far cell: the mesher's stride for this rung.
pub const FAR_CELL_SUBVOXELS: i32 = crate::lod::FAR_CELL_SUBVOXELS;

/// Cells in one far chunk's census at the far tier: `4³`.
pub const FAR_CELLS: usize = crate::lod::FAR_CELLS;

/// **How many lanes build the far census at once.**
///
/// A far chunk's tree is a pure function of the spec, so the censuses are independent: the value,
/// the mesh and the band words are identical to one lane's, and the only thing the lane count buys
/// is wall-clock. Four leaves most of a sixteen-core machine to the streamer and the frame; the
/// measured first fill is in the patch.
pub const FAR_CENSUS_LANES: usize = 4;

/// **How many group layers a group-spanning field draws**: the eye's group and the one below it.
///
/// A group cell is the whole group, so this is 128 m of column at a 64 m grain — the world's
/// −22 … +54 m surface with room to spare. One layer would be 64 m and would cut the valleys under
/// an eye near the top of its group; three would rebuild 50% more chunks for ground the finer
/// fields already draw.
pub const GROUP_STACK: i32 = 1;

/// **How many chunks a worker censuses, meshes and delivers at a time.**
///
/// The first delivery is the point of the field: a wave is the smallest batch whose geometry is
/// worth a frame, and eight of them is roughly a second of tree builds in four lanes. A whole ring
/// per delivery is what made the horizon arrive all at once.
pub const FAR_WAVE: usize = 8;

/// **How many chunk layers the far field draws above and below the eye's chunk.**
///
/// The world's declared surface spans roughly −22 … +54 m (a 16 m datum and 40 m of relief), so
/// **one layer each side** of the eye's chunk — −32 … +64 m while the eye is in the surface layer —
/// covers its relief without stacking the whole column. The stack moves with the eye: a player who
/// climbs a chunk carries it up, and the field re-centres on the new layer rather than pinning the
/// world to y = 0. Deeper cave systems and anything more than a chunk past the relief are outside
/// it, and that edge is named in the module docs rather than implied.
pub const FAR_STACK: i32 = 1;

/// Voxels in one far cell, which a solid share is measured out of.
pub const FAR_CELL_VOXELS_TOTAL: u32 = crate::lod::FAR_CELL_VOXELS_TOTAL;

/// **One 8 m cell's census**: how many of its 512 voxels hold matter, and the material at the top
/// of the column that does.
///
/// The material is the **topmost solid voxel's**, not the majority's: a surface cell is mostly rock
/// and soil under a thin grass layer, and a majority vote would paint the distant hills brown. What
/// a viewer sees of a cell is its top, so that is what the cell is coloured by.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CoarseCell {
    /// Solid voxels out of [`CoarseCell::voxels`].
    pub solids: u32,
    /// **How many voxels this cell was measured out of**: 512 at eight metres, 4096 at sixteen.
    ///
    /// The cell carries its own denominator rather than reading the far tier's constant, because
    /// "half solid" is a claim about *this* cell: a tier's solver must not be able to draw a
    /// 16 m cell at an 8 m threshold simply because the module's constant is the smaller one.
    pub voxels: u32,
    /// The local voxel `y` of the highest solid cell, `-1` when nothing is solid.
    pub top_voxel: i32,
    /// The material of the highest solid subvoxel.
    pub material: MaterialId,
}

impl Default for CoarseCell {
    fn default() -> Self {
        // An unmeasured cell: no solids, and **no denominator** — [`CoarseCell::solid`] refuses to
        // call it air or matter until a census has said what it was measured out of.
        Self {
            solids: 0,
            voxels: 0,
            top_voxel: -1,
            material: MaterialId::EMPTY,
        }
    }
}

impl CoarseCell {
    /// **Whether the cell is occupancy at this rung** — the *band's* threshold: at least half its
    /// volume, which is where a light beam's march stops on it.
    ///
    /// It is **not** the geometry's rule any more; see [`CoarseCell::holds_matter`].
    pub fn solid(&self) -> bool {
        self.voxels > 0 && self.solids * 2 >= self.voxels
    }

    /// **Whether the cell holds matter at all** — the geometry's own threshold, and the reason it
    /// exists.
    ///
    /// The world is voxels at whatever cell a rung measures them in, so a cell that holds matter is
    /// a **cuboid** whose height is that matter's own top — never nothing. The two tests were one
    /// until this pass, and what the old gate cost is measured
    /// (`measure_what_the_draw_gate_costs`, eight real chunks at eight metres): **251 cells hold
    /// matter, 27 of them below half, and 26 of those 27 are the surface cell the eye sees**. Every
    /// one of those was drawn as nothing at all while the band stamped its share — a hole in the
    /// ground the light had already called solid. See
    /// `a_partly_full_cell_is_a_low_cuboid_not_a_hole`.
    pub fn holds_matter(&self) -> bool {
        self.solids > 0
    }

    /// The share of the cell that holds matter, in `0..=1` — the coarse band's own number.
    pub fn share(&self) -> f32 {
        if self.voxels == 0 {
            0.0
        } else {
            self.solids as f32 / self.voxels as f32
        }
    }
}

/// **One chunk's census at one tier**, in `x + side·(y + side·z)` order.
///
/// The census owns the tier it was measured at, and everything that reads it — [`coarse_mesh`],
/// the coarse band — reads the cell size from here rather than from the module's far-tier
/// constant. That is what lets a second tier exist without a second walk: at tier 6 the same walk
/// yields a `2³` census of 16 m cells, and the same mesher cuts it on a 64-subvoxel stride.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Census {
    tier: u32,
    side: i32,
    cells: Vec<CoarseCell>,
}

impl Default for Census {
    fn default() -> Self {
        Self::empty(crate::lod::FAR_TIER)
    }
}

impl Census {
    /// **An unmeasured census at a tier**: the right shape, every cell carrying its own volume but
    /// no solids. Used as the empty value and as the seed [`Census::of`] fills.
    pub fn empty(tier: u32) -> Census {
        let cell_m = crate::lod::cell_m(tier);
        assert_eq!(
            cell_m.fract(),
            0.0,
            "tier {tier} is not a whole number of metres"
        );
        let side = crate::lod::cells_per_chunk(tier)
            .unwrap_or_else(|| panic!("tier {tier} is not a whole number of cells per chunk"));
        let voxels = (cell_m as u32).pow(3);
        let cell = CoarseCell {
            voxels,
            ..CoarseCell::default()
        };
        Census {
            tier,
            side,
            cells: vec![cell; (side * side * side) as usize],
        }
    }

    /// **The one walk a chunk's geometry and its light both read, at a tier.**
    ///
    /// Each leaf adds its solid volume to every coarse cell it overlaps, and records the material
    /// at the top of its column. A `Uniform` leaf is all solid, a `Mixed` one counts its non-empty
    /// subvoxels, and a `Branch` is not a leaf (the walk visits its children). The arithmetic is the
    /// coarse band's own — see the module docs for why there is not a second copy.
    pub fn of(tree: &SparseVoxelOctree, tier: u32) -> Census {
        let mut census = Census::empty(tier);
        // The cell's **side**, in voxels — not its volume, which is the share's denominator.
        let cell = census.cell_m() as i32;
        let side = census.side;
        tree.visit_leaves(&mut |local: [i32; 3], level: u32, node: &Node| {
            match node {
                Node::Empty | Node::Branch(_) => {}
                Node::Uniform(material) => {
                    let span = 1i32 << level;
                    let top = local[1] + span - 1;
                    // The coarse cells this leaf overlaps. A leaf is aligned to its own size, so
                    // this is a divide rather than a sweep.
                    let lo = [local[0] / cell, local[1] / cell, local[2] / cell];
                    let hi = [
                        (local[0] + span - 1) / cell,
                        (local[1] + span - 1) / cell,
                        (local[2] + span - 1) / cell,
                    ];
                    for z in lo[2]..=hi[2] {
                        for y in lo[1]..=hi[1] {
                            for x in lo[0]..=hi[0] {
                                let at = (x + side * (y + side * z)) as usize;
                                // The leaf's overlap with this cell, in voxels on each axis —
                                // exact, so the shares across a chunk sum to its solid volume.
                                let overlap = |leaf: i32, leaf_side: i32, index: i32| {
                                    (leaf + leaf_side).min((index + 1) * cell)
                                        - leaf.max(index * cell)
                                };
                                let volume = overlap(local[0], span, x)
                                    * overlap(local[1], span, y)
                                    * overlap(local[2], span, z);
                                let target = &mut census.cells[at];
                                target.solids += volume as u32;
                                if top > target.top_voxel {
                                    target.top_voxel = top;
                                    target.material = *material;
                                }
                            }
                        }
                    }
                }
                Node::Mixed(shape) => {
                    let solid: Vec<(usize, MaterialId)> = shape
                        .cells()
                        .iter()
                        .enumerate()
                        .filter(|(_, material)| !material.is_empty())
                        .map(|(ordinal, material)| (ordinal, *material))
                        .collect();
                    if solid.is_empty() {
                        return;
                    }
                    let at = [local[0] / cell, local[1] / cell, local[2] / cell];
                    let index = (at[0] + side * (at[1] + side * at[2])) as usize;
                    // The 64 cells are `x + 4y + 16z` (`SubvoxelIndex::flat`), so the topmost
                    // solid subvoxel is the one with the largest `y` — `ordinal / 4 mod 4`.
                    let (_, material) = solid
                        .iter()
                        .max_by_key(|(ordinal, _)| (ordinal / 4) % 4)
                        .copied()
                        .expect("non-empty");
                    let target = &mut census.cells[index];
                    target.solids += solid.len() as u32;
                    let top = local[1];
                    if top > target.top_voxel {
                        target.top_voxel = top;
                        target.material = material;
                    }
                }
            }
        });
        census
    }

    /// The tier this census was measured at.
    pub fn tier(&self) -> u32 {
        self.tier
    }

    /// The cell size this census states, in metres.
    pub fn cell_m(&self) -> f32 {
        crate::lod::cell_m(self.tier) as f32
    }

    /// Cells along one axis of the chunk.
    pub fn side(&self) -> i32 {
        self.side
    }

    /// Voxels one cell of this census holds.
    pub fn cell_voxels(&self) -> u32 {
        self.cells.first().map_or(0, |cell| cell.voxels)
    }

    /// The cell at a local coarse index, or `None` outside the chunk.
    pub fn get(&self, at: [i32; 3]) -> Option<&CoarseCell> {
        if at.iter().any(|v| *v < 0 || *v >= self.side) {
            return None;
        }
        Some(&self.cells[(at[0] + self.side * (at[1] + self.side * at[2])) as usize])
    }

    /// Whether a local coarse cell is matter **at the band's threshold** — half a cell. The
    /// geometry's own question is [`CoarseCell::holds_matter`], or `None` outside the chunk.
    pub fn solid(&self, at: [i32; 3]) -> Option<bool> {
        self.get(at).map(CoarseCell::solid)
    }

    /// Every cell's share, in the band's order — what [`crate::horizon::HorizonStream::stamp_far`]
    /// writes.
    pub fn shares(&self) -> impl Iterator<Item = f32> + '_ {
        self.cells.iter().map(CoarseCell::share)
    }

    /// The cells' own view, for a reader that wants the material as well as the share.
    pub fn cells(&self) -> &[CoarseCell] {
        &self.cells
    }

    /// **The same voxels measured at a coarser chunk-held tier** — `factor` cell widths per new
    /// cell. Each new cell is the sum of the `factor³` cells it covers, so its share is the mean of
    /// theirs and its top is the highest of theirs: the 16 m reading of a chunk whose 8 m census is
    /// already in hand, with no second tree walk and no second world.
    ///
    /// This is how the far ring feeds the wide band: a tier-6 ray that crosses the 8 m ring must
    /// meet its terrain rather than an unmeasured cell, and summing is the same measurement the
    /// census would have made at 16 m.
    /// **One census over a group of chunks**, at a tier whose cell is the whole group.
    ///
    /// A cell larger than a chunk cannot be held by [`Census::of`] — `cells_per_chunk` has no
    /// answer for it — so this is the owner that spans them: each of the `span³` chunks is censused
    /// on its own last chunk-held rung (one 32 m cell a chunk), and the group's cell is the sum of
    /// them. **A sum of measurements, not a second reading of the world**: the solids, the top and
    /// the material come from the same walks the finer rungs make, which is what lets a 64 m face
    /// and the 64 m band answer with one number.
    ///
    /// `origin` is the group's low chunk; every tree must lie inside `origin .. origin + span`.
    ///
    /// # Panics
    ///
    /// A tier whose cell is not `span` chunks, or a set of trees that is not `span³` chunks inside
    /// the group, is a caller error rather than a coarse census.
    pub fn of_group(
        origin: ChunkPos,
        span: i32,
        tier: u32,
        trees: &[(ChunkPos, SparseVoxelOctree)],
    ) -> Census {
        assert!(span > 1, "a group spans more than one chunk");
        let cell_m = crate::lod::cell_m(tier);
        assert_eq!(
            cell_m,
            f64::from(span * CHUNK_SIDE),
            "a {span}-chunk group is not tier {tier}"
        );
        assert_eq!(
            trees.len(),
            (span * span * span) as usize,
            "a {span}³ group needs {} censuses, not {}",
            span * span * span,
            trees.len()
        );
        let mut solids = 0u32;
        let mut top_voxel = -1i32;
        let mut material = MaterialId::EMPTY;
        for (pos, tree) in trees {
            assert!(
                (0..span).contains(&(pos.x - origin.x))
                    && (0..span).contains(&(pos.y - origin.y))
                    && (0..span).contains(&(pos.z - origin.z)),
                "{pos:?} is outside the group at {origin:?}"
            );
            let one = Census::of(tree, crate::lod::CHUNK_TIER);
            let cell = one.get([0, 0, 0]).expect("a chunk-held census is one cell");
            solids += cell.solids;
            let top = (pos.y - origin.y) * CHUNK_SIDE + cell.top_voxel;
            if cell.top_voxel >= 0 && top > top_voxel {
                top_voxel = top;
                material = cell.material;
            }
        }
        Census {
            tier,
            side: 1,
            cells: vec![CoarseCell {
                solids,
                voxels: (cell_m as u32).pow(3),
                top_voxel,
                material,
            }],
        }
    }

    pub fn aggregate(&self, factor: i32) -> Census {
        assert!(factor > 0, "a coarser tier is at least one cell wide");
        assert_eq!(
            self.side % factor,
            0,
            "the factor does not divide the census's side"
        );
        let side = self.side / factor;
        let cell_voxels = (self.cell_voxels() as i32 * factor * factor * factor) as u32;
        let cell = CoarseCell {
            voxels: cell_voxels,
            ..CoarseCell::default()
        };
        let mut cells = vec![cell; (side * side * side) as usize];
        for z in 0..side {
            for y in 0..side {
                for x in 0..side {
                    let target = &mut cells[(x + side * (y + side * z)) as usize];
                    for dz in 0..factor {
                        for dy in 0..factor {
                            for dx in 0..factor {
                                let at = [x * factor + dx, y * factor + dy, z * factor + dz];
                                let source = self.get(at).expect("inside the census");
                                target.solids += source.solids;
                                if source.top_voxel > target.top_voxel {
                                    target.top_voxel = source.top_voxel;
                                    target.material = source.material;
                                }
                            }
                        }
                    }
                }
            }
        }
        Census {
            tier: self.tier + factor.trailing_zeros(),
            side,
            cells,
        }
    }
}

/// **The one walk a far chunk's geometry and its light both read**, at the far tier.
///
/// The walk itself is [`Census::of`]; this is the far tier's name for it, and the only one the
/// shipped field uses.
pub fn census(tree: &SparseVoxelOctree) -> Census {
    Census::of(tree, crate::lod::FAR_TIER)
}

/// **One chunk's surface at the tier its census was measured at**, as the mesher's own `Mesh`.
///
/// `outside` answers whether a coarse cell outside this chunk is solid — `Some` when the caller
/// has the neighbouring census in hand and `None` when it does not. **An unknown neighbour is not
/// air**: no face is drawn into a cell nobody has measured, so the ring's outer edge ends in open
/// air rather than in a wall of eight-metre stones. Cells inside the chunk come from `census`.
pub fn coarse_mesh(
    census: &Census,
    pos: ChunkPos,
    outside: impl Fn([i32; 3]) -> Option<bool>,
) -> Mesh {
    /// What a neighbouring cell says about the face between it and this one.
    ///
    /// **Matter is `solids > 0`, not the band's half-cell** ([`CoarseCell::holds_matter`]): a
    /// neighbour the ground cuts in half is a block beside this one, so the wall between them runs
    /// from the neighbour's own measured top to this cell's, and the rung stays a field of cuboids
    /// at its cell size rather than a set of tiles with holes between them.
    enum Neighbour {
        /// Measured empty — the denominator says this cell was walked and holds nothing: the wall
        /// runs from this cell's floor.
        Air,
        /// Measured matter, with the top of its own cell in subvoxels from this chunk's floor.
        Solid(u16),
        /// Nobody has measured it, so no face is drawn into it.
        Unknown,
    }
    let side = census.side();
    let cell_m = census.cell_m() as i32;
    let stride = (cell_m * SUBVOXEL_SIDE as i32) as u16;
    let voxel = (stride as i32 / cell_m) as u16;
    let mut quads: Vec<Quad> = Vec::new();
    let mut visible_faces = 0u64;
    // **A cell's top is its own measurement, inside its own cell.** `top_voxel` is the highest
    // solid voxel of that cell, clamped to it; a cell that recorded none falls back to its fill
    // share, so a partly solid cell is a low cuboid rather than a plane at the cell's ceiling.
    let top_in = |cell: &CoarseCell, y: i32| -> u16 {
        let rel = cell.top_voxel + 1 - y * cell_m;
        let voxels = if rel > 0 {
            rel.min(cell_m)
        } else {
            let share = cell.solids.min(cell.voxels) as f32 / cell.voxels.max(1) as f32;
            (share * cell_m as f32).round() as i32
        };
        (voxels.clamp(1, cell_m) as u16) * voxel
    };
    let neighbour = |at: [i32; 3]| -> Neighbour {
        match census.get(at) {
            Some(cell) if cell.holds_matter() => {
                Neighbour::Solid(at[1] as u16 * stride + top_in(cell, at[1]))
            }
            // A cell with a denominator and no solids is a cell the walk found empty.
            Some(cell) if cell.voxels > 0 => Neighbour::Air,
            Some(_) => Neighbour::Unknown,
            None => match outside(at) {
                Some(true) => Neighbour::Unknown,
                Some(false) => Neighbour::Air,
                None => Neighbour::Unknown,
            },
        }
    };
    for z in 0..side {
        for y in 0..side {
            for x in 0..side {
                let at = [x, y, z];
                let Some(cell) = census.get(at) else { continue };
                if !cell.holds_matter() || cell.material.is_empty() {
                    continue;
                }
                let here = y as u16 * stride + top_in(cell, y);
                let material = cell.material;
                // The cell's own roof, at the height it measured: one rectangle a cell.
                quads.push(Quad {
                    face: Face::PosY,
                    slice: here,
                    uv: [x as u16 * stride, z as u16 * stride],
                    extent: [stride, stride],
                    material,
                });
                visible_faces += 1;
                // The walls of the cuboid, wherever a measured neighbour stands lower.
                for (face, step) in [
                    (Face::NegX, [-1, 0, 0]),
                    (Face::PosX, [1, 0, 0]),
                    (Face::NegZ, [0, 0, -1]),
                    (Face::PosZ, [0, 0, 1]),
                ] {
                    let base = match neighbour([x + step[0], y + step[1], z + step[2]]) {
                        Neighbour::Air => y as u16 * stride,
                        Neighbour::Solid(other) if other < here => other,
                        _ => continue,
                    };
                    let height = here.saturating_sub(base);
                    if height == 0 {
                        continue;
                    }
                    let plane = match face {
                        Face::NegX => x as u16 * stride,
                        Face::PosX => (x as u16 + 1) * stride,
                        Face::NegZ => z as u16 * stride,
                        _ => (z as u16 + 1) * stride,
                    };
                    let (uv, extent) = match face {
                        Face::NegX | Face::PosX => ([base, z as u16 * stride], [height, stride]),
                        _ => ([x as u16 * stride, base], [stride, height]),
                    };
                    quads.push(Quad {
                        face,
                        slice: plane,
                        uv,
                        extent,
                        material,
                    });
                    visible_faces += 1;
                }
            }
        }
    }
    let solid_voxels = census.cells().iter().map(|cell| cell.solids as u64).sum();
    Mesh {
        chunk: pos,
        origin: {
            let origin = pos.origin();
            [origin.x, origin.y, origin.z]
        },
        stats: MeshStats {
            solid_voxels,
            visible_faces,
            quads: quads.len() as u64,
            loops: 0,
        },
        quads,
        loops: Vec::new(),
    }
}

/// `voxel_mesh::mesh::axes`, restated here because that one is private: the two
/// coordinates a face's `(u, v)` rectangle is measured in.
const fn face_axes(axis: usize) -> (usize, usize) {
    match axis {
        0 => (1, 2),
        1 => (0, 2),
        _ => (0, 1),
    }
}

/// **Every position a field draws**: its ring, in every layer from `−FAR_STACK` to `+FAR_STACK`.
pub(crate) fn stack_between(radius: i32, centre: ChunkPos, near: i32, far: i32) -> Vec<ChunkPos> {
    let mut stack = Vec::new();
    for dy in -FAR_STACK..=FAR_STACK {
        stack.extend(ring_between(radius, centre, dy, near, far));
    }
    stack
}

/// The tier-5 field's ring: `radius+1 ..= radius+2` from `centre`, one layer.
#[cfg(test)]
pub(crate) fn stack_at(radius: i32, centre: ChunkPos) -> Vec<ChunkPos> {
    stack_between(radius, centre, 1, 2)
}

/// **A group tier's ring**: whole groups at `near ..= far` group offsets from the group the eye is
/// in, in the eye's group layer and [`GROUP_STACK`] below it.
///
/// The positions are the groups' **low chunks**, so every coordinate is a multiple of `span` and a
/// census, a mesh and a stamp all name the same cell. A group is never half in a ring: the cell is
/// the group, and a ring that cut one would draw half a measurement.
pub(crate) fn group_stack_between(
    centre: ChunkPos,
    span: i32,
    near: i32,
    far: i32,
) -> Vec<ChunkPos> {
    let group = [
        centre.x.div_euclid(span),
        centre.y.div_euclid(span),
        centre.z.div_euclid(span),
    ];
    let mut ring = Vec::new();
    for dy in -GROUP_STACK..=0 {
        for dz in -far..=far {
            for dx in -far..=far {
                if dx.abs().max(dz.abs()) >= near {
                    ring.push(ChunkPos::new(
                        (group[0] + dx) * span,
                        (group[1] + dy) * span,
                        (group[2] + dz) * span,
                    ));
                }
            }
        }
    }
    ring
}

/// The positions drawn in **one chunk layer**, `radius+near ..= radius+far` chunks from `centre`.
///
/// The offsets are how a second field is placed beyond the first without a second mechanism: tier
/// 6 passes `(3, 4)` and draws the arc outside tier 5's `(1, 2)` ring, in the same chunks, from the
/// same generator, with a different cell.
pub(crate) fn ring_between(
    radius: i32,
    centre: ChunkPos,
    dy: i32,
    near: i32,
    far: i32,
) -> Vec<ChunkPos> {
    let inner = radius + near;
    let outer = radius + far;
    let mut ring = Vec::new();
    for dz in -outer..=outer {
        for dx in -outer..=outer {
            let distance = dx.abs().max(dz.abs());
            if distance >= inner {
                ring.push(ChunkPos::new(centre.x + dx, centre.y + dy, centre.z + dz));
            }
        }
    }
    ring
}

/// The tier-5 field's ring in one layer.
#[cfg(test)]
pub(crate) fn ring_at(radius: i32, centre: ChunkPos, dy: i32) -> Vec<ChunkPos> {
    ring_between(radius, centre, dy, 1, 2)
}

/// **Everything a job censuses**, so every face a drawn chunk asks for has an answer.
///
/// A drawn chunk's side faces need the chunk beside it, and its top and bottom faces need the chunk
/// above or below it. So each drawn layer is censused with the ring *and* one horizontal step past
/// it, and the layer beyond the stack is censused at the ring's own columns. Nothing diagonal is
/// ever asked for, because a face's neighbour is one axis away.
#[cfg(test)]
pub(crate) fn census_for(radius: i32, centre: ChunkPos) -> Vec<ChunkPos> {
    census_between(radius, centre, 1, 2)
}

/// **Everything a field's jobs census**, for a ring `near..=far` chunks out: that ring and one
/// horizontal step past it in every drawn layer, and the ring's own columns one layer above and
/// below the stack (the top and bottom faces).
#[cfg(test)]
pub(crate) fn census_between(radius: i32, centre: ChunkPos, near: i32, far: i32) -> Vec<ChunkPos> {
    let mut census = Vec::new();
    for dy in -FAR_STACK..=FAR_STACK {
        census.extend(ring_between(radius, centre, dy, near, far + 1));
    }
    for dy in [-FAR_STACK - 1, FAR_STACK + 1] {
        census.extend(ring_between(radius, centre, dy, near, far));
    }
    census
}

/// One tier-5 layer's ring and one horizontal step past it.
#[cfg(test)]
pub(crate) fn census_at(radius: i32, centre: ChunkPos, dy: i32) -> Vec<ChunkPos> {
    ring_between(radius, centre, dy, 1, 3)
}

/// **One far chunk that is ready to draw**: its surface, the census it was cut from, and the
/// version of the world that produced it.
#[derive(Clone, PartialEq, Debug)]
pub struct FarChunk {
    pub pos: ChunkPos,
    pub version: u64,
    pub mesh: Mesh,
    pub census: Census,
}

/// One unit of work for the far thread.
struct Job {
    epoch: u64,
    /// Positions to mesh, nearest first — the worker censuses whatever each wave needs as it goes.
    mesh: Vec<ChunkPos>,
}

/// What the far thread sends back.
enum Done {
    Chunk(FarChunk),
    Finished { epoch: u64 },
}

/// **The far ring: what is drawn, what is being built, and the thread that builds it.**
///
/// A ring is scheduled when the eye enters a new chunk. Positions already drawn stay drawn, so a
/// crossing pays for the arc that entered and nothing else; the census cache and the drawn set are
/// what make a far chunk a thing the player walks past rather than a thing the frame rebuilds.
pub struct FarTerrain {
    /// Every far chunk currently drawn, by position.
    chunks: BTreeMap<ChunkPos, FarChunk>,
    /// The positions the current ring wants, so a late delivery of a chunk the eye has left is
    /// dropped rather than drawn behind the camera.
    wanted: BTreeSet<ChunkPos>,
    /// **Whether a job is being worked on**, and whether a newer eye arrived while it was.
    ///
    /// One job at a time is what keeps the field honest under a walk: generating a ring is seconds
    /// of CPU, and queueing a job per crossing spent all of them on stale rings whose chunks were
    /// then rejected. A crossing during a job records `reschedule`; the worker finishes what it has, the
    /// main thread adopts whatever is still wanted, and the next job is measured against the eye's
    /// **newest** chunk rather than a queue of old ones.
    job: Option<u64>,
    reschedule: bool,
    /// The chunks adopted by the most recent [`FarTerrain::advance`], for the caller to stamp into
    /// the band. Cleared at the start of each advance.
    adopted: Vec<FarChunk>,
    to_worker: Option<Sender<Job>>,
    from_worker: Option<Receiver<Done>>,
    thread: Option<std::thread::JoinHandle<()>>,
    eye: Option<ChunkPos>,
    epoch: u64,
    radius: i32,
    /// **The tier this field cuts at**: a census and a mesh grain, not a second field type.
    tier: u32,
    /// **Chunks along one edge of a cell**, 1 for a chunk-held rung and `2ⁿ` for a group tier.
    ///
    /// It is the one number that separates "census a chunk" from "census the group this cell
    /// spans": the ring, the neighbour lookup and the census build all step by it.
    span: i32,
    /// The ring's first and last offset past the streamed window, in chunks.
    near: i32,
    far: i32,
    builds: u64,
}

impl FarTerrain {
    /// **A far field around a streamed window of `radius` chunks**, generated from the world's own
    /// spec on its own thread. The tier-5 ring, `radius+1 ..= radius+2` chunks out.
    pub fn new(spec: WorldSpec, radius: i32) -> Self {
        Self::at_tier(spec, radius, crate::lod::FAR_TIER, 1, 2)
    }

    /// **A field at any chunk-held tier, in any ring outside the window.**
    ///
    /// The second rung is the same mechanism: tier 6 builds the same chunks from the same spec,
    /// censuses them at 16 m and draws them at `radius+near ..= radius+far` — outside the tier-5
    /// ring, so the two fields meet at a chunk boundary and neither double-draws a chunk.
    pub fn at_tier(spec: WorldSpec, radius: i32, tier: u32, near: i32, far: i32) -> Self {
        assert!(
            crate::lod::cells_per_chunk(tier).is_some(),
            "tier {tier} is not chunk-held: a cell larger than a chunk needs a composite census"
        );
        assert!(
            far > near && near > 0,
            "a ring must be a band outside the window"
        );
        Self::field(spec, radius, tier, 1, near, far)
    }

    /// **A field whose cell spans a group of chunks** — the first rung past the chunk-held ones.
    ///
    /// `span` chunks a side, `near..=far` in **group offsets** from the group the eye is in, and a
    /// census per group rather than per chunk ([`Census::of_group`]). The ring is one group thick at
    /// tier 8, so it begins where tier 7's 32 m ring ends, and its cells are whole groups: a drawn
    /// face belongs to the group it is cut from, which is what makes it a rung.
    pub fn at_group(
        spec: WorldSpec,
        radius: i32,
        tier: u32,
        span: i32,
        near: i32,
        far: i32,
    ) -> Self {
        assert!(span > 1, "a group spans more than one chunk");
        assert_eq!(
            crate::lod::cell_m(tier),
            f64::from(span * CHUNK_SIDE),
            "a {span}-chunk group is not tier {tier}"
        );
        assert!(
            crate::lod::cells_per_chunk(tier).is_none(),
            "tier {tier} is chunk-held and belongs in at_tier"
        );
        assert!(
            far >= near && near > 0,
            "a group ring must be outside the window"
        );
        Self::field(spec, radius, tier, span, near, far)
    }

    fn field(spec: WorldSpec, radius: i32, tier: u32, span: i32, near: i32, far: i32) -> Self {
        let (to_worker, jobs) = std::sync::mpsc::channel::<Job>();
        let (done, from_worker) = std::sync::mpsc::channel::<Done>();
        let thread = std::thread::Builder::new()
            .name("far-terrain".into())
            .spawn(move || {
                // **The census cache lives on the worker** because the worker is the only reader
                // that needs a neighbour's answer: a chunk's mesh asks for the cells just outside
                // it, and those are positions that are not drawn (the margin) or are drawn but
                // were built for another crossing. 64 cells each is a few hundred bytes, so the
                // cache is kept for the session rather than evicted under a walk.
                let mut censused: BTreeMap<ChunkPos, Census> = BTreeMap::new();
                while let Ok(job) = jobs.recv() {
                    // **A wave at a time, nearest first.** The census cache is the worker's own: a
                    // neighbour is asked for when the face that needs it is about to be cut, not a
                    // whole ring ahead. That is what makes the field arrive as it is measured
                    // instead of all at once after the last chunk of the ring.
                    let drawn: BTreeSet<ChunkPos> = job.mesh.iter().copied().collect();
                    let mut queue: Vec<ChunkPos> = job.mesh.clone();
                    while !queue.is_empty() {
                        let take = FAR_WAVE.min(queue.len());
                        let wave: Vec<ChunkPos> = queue.drain(..take).collect();
                        // What this wave has to know: itself, and — for a chunk-held tier — the
                        // six neighbours a face can look into. A group field censuses only the
                        // groups it draws, so its seams are answered where both sides exist and
                        // its outer edge ends without a wall.
                        let mut need: Vec<ChunkPos> = Vec::with_capacity(wave.len() * 7);
                        for pos in &wave {
                            need.push(*pos);
                            for step in [
                                [span, 0, 0],
                                [-span, 0, 0],
                                [0, span, 0],
                                [0, -span, 0],
                                [0, 0, span],
                                [0, 0, -span],
                            ] {
                                let neighbour = ChunkPos::new(
                                    pos.x + step[0],
                                    pos.y + step[1],
                                    pos.z + step[2],
                                );
                                if span == 1 || drawn.contains(&neighbour) {
                                    need.push(neighbour);
                                }
                            }
                        }
                        need.sort();
                        need.dedup();
                        need.retain(|pos| !censused.contains_key(pos));
                        if !need.is_empty() {
                            // **The censuses are independent, so they are built in lanes.** Each
                            // lane owns its slice: no shared mutation, and the same values a
                            // single lane would produce — a lane count changes wall-clock and
                            // nothing else.
                            let lanes = FAR_CENSUS_LANES.min(need.len());
                            let per = need.len().div_ceil(lanes);
                            let built: Vec<(ChunkPos, Census)> = std::thread::scope(|scope| {
                                let mut handles = Vec::new();
                                for slice in need.chunks(per) {
                                    let spec = spec.clone();
                                    handles.push(scope.spawn(move || {
                                        let source = WorldSource::new(spec);
                                        slice
                                            .iter()
                                            .filter_map(|pos| {
                                                if span == 1 {
                                                    source
                                                        .build_chunk(*pos)
                                                        .ok()
                                                        .map(|tree| (*pos, Census::of(&tree, tier)))
                                                } else {
                                                    // **The group census is the sum of its
                                                    // chunks' own censuses**: every chunk of the
                                                    // cell is measured at the last chunk-held rung,
                                                    // and the group's cell is their total.
                                                    let mut trees = Vec::new();
                                                    for dy in 0..span {
                                                        for dz in 0..span {
                                                            for dx in 0..span {
                                                                let at = ChunkPos::new(
                                                                    pos.x + dx,
                                                                    pos.y + dy,
                                                                    pos.z + dz,
                                                                );
                                                                trees.push((
                                                                    at,
                                                                    source.build_chunk(at).ok()?,
                                                                ));
                                                            }
                                                        }
                                                    }
                                                    Some((
                                                        *pos,
                                                        Census::of_group(*pos, span, tier, &trees),
                                                    ))
                                                }
                                            })
                                            .collect::<Vec<_>>()
                                    }));
                                }
                                handles
                                    .into_iter()
                                    .flat_map(|handle| handle.join().expect("a census lane"))
                                    .collect()
                            });
                            censused.extend(built);
                        }
                        for pos in &wave {
                            let Some(here) = censused.get(pos) else {
                                continue;
                            };
                            // **A neighbour outside the census is not air and is not solid — it
                            // is unknown**, and a face into it is not drawn.
                            let side = here.side();
                            let outside = |at: [i32; 3]| -> Option<bool> {
                                let off = [
                                    at[0].div_euclid(side),
                                    at[1].div_euclid(side),
                                    at[2].div_euclid(side),
                                ];
                                let local = [
                                    at[0].rem_euclid(side),
                                    at[1].rem_euclid(side),
                                    at[2].rem_euclid(side),
                                ];
                                let neighbour = ChunkPos::new(
                                    pos.x + off[0] * span,
                                    pos.y + off[1] * span,
                                    pos.z + off[2] * span,
                                );
                                censused
                                    .get(&neighbour)
                                    .and_then(|census| census.solid(local))
                            };
                            let mesh = coarse_mesh(here, *pos, outside);
                            if done
                                .send(Done::Chunk(FarChunk {
                                    pos: *pos,
                                    version: job.epoch,
                                    mesh,
                                    census: here.clone(),
                                }))
                                .is_err()
                            {
                                return;
                            }
                        }
                    }
                    let _ = done.send(Done::Finished { epoch: job.epoch });
                }
            })
            .ok();
        let (to_worker, from_worker) = match thread {
            Some(_) => (Some(to_worker), Some(from_worker)),
            None => (None, None),
        };
        FarTerrain {
            chunks: BTreeMap::new(),
            wanted: BTreeSet::new(),
            job: None,
            reschedule: false,
            adopted: Vec::new(),
            to_worker,
            from_worker,
            thread,
            eye: None,
            epoch: 0,
            radius,
            tier,
            span,
            near,
            far,
            builds: 0,
        }
    }

    /// The tier this field cuts its census and its mesh at.
    pub fn tier(&self) -> u32 {
        self.tier
    }

    /// **Move the ring to the eye's chunk**, scheduling what entered it.
    ///
    /// The ring is `radius+1 ..= radius+2` chunks from the eye in each of `FAR_STACK` layers above
    /// and below it, and it is censused one chunk further out — and one layer further up and down —
    /// so every drawn face knows both sides. A crossing retains the chunks still in the stack, so
    /// the work is the leading arc, not the field.
    pub fn set_eye(&mut self, centre: ChunkPos) -> bool {
        if self.eye == Some(centre) {
            return false;
        }
        self.eye = Some(centre);
        self.epoch += 1;
        self.wanted = self.ring(centre).into_iter().collect();
        self.chunks.retain(|pos, _| self.wanted.contains(pos));
        self.schedule();
        true
    }

    /// The positions the field draws: a chunk ring `radius+near ..= radius+far` in every stack
    /// layer, or — for a group tier — whole groups at `near ..= far` group offsets.
    fn ring(&self, centre: ChunkPos) -> Vec<ChunkPos> {
        if self.span == 1 {
            stack_between(self.radius, centre, self.near, self.far)
        } else {
            group_stack_between(centre, self.span, self.near, self.far)
        }
    }

    /// **Ask the thread for what the ring is missing**, if it is not already working.
    ///
    /// Positions are ordered by distance from the eye, so the worker delivers the near edge of the
    /// field first and the rest follows wave by wave. The list is not filtered by what was asked
    /// before: the thread's own cache is the record of what it has built, and a superseded job may
    /// have left positions it never reached. Re-asking a cached position costs one map lookup.
    fn schedule(&mut self) {
        if self.job.is_some() {
            self.reschedule = true;
            return;
        }
        let Some(centre) = self.eye else {
            return;
        };
        let mut mesh: Vec<ChunkPos> = self
            .ring(centre)
            .into_iter()
            .filter(|pos| !self.chunks.contains_key(pos))
            .collect();
        mesh.sort_by_key(|pos| {
            (
                (pos.x - centre.x).abs().max((pos.z - centre.z).abs()),
                (pos.y - centre.y).abs(),
            )
        });
        if mesh.is_empty() {
            return;
        }
        if let Some(worker) = &self.to_worker {
            if worker
                .send(Job {
                    epoch: self.epoch,
                    mesh,
                })
                .is_ok()
            {
                self.job = Some(self.epoch);
                self.reschedule = false;
            }
        }
    }

    /// **Adopt what the thread has finished**, at most `budget` chunks this frame.
    ///
    /// Returns the chunks adopted on this call — the caller stamps each one's census into the light
    /// band, which is the only step that has to happen on the frame's thread. A delivery for a
    /// chunk the eye has already left is dropped; it will be asked for again if the ring returns.
    pub fn advance(&mut self, budget: usize) -> &[FarChunk] {
        self.adopted.clear();
        let Some(receiver) = &self.from_worker else {
            return &self.adopted;
        };
        let mut finished = None;
        while self.adopted.len() < budget {
            match receiver.try_recv() {
                Ok(Done::Chunk(chunk)) => {
                    if self.wanted.contains(&chunk.pos) && !self.chunks.contains_key(&chunk.pos) {
                        self.chunks.insert(chunk.pos, chunk.clone());
                        self.builds += 1;
                        self.adopted.push(chunk);
                    }
                }
                Ok(Done::Finished { epoch }) => finished = Some(epoch),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
        // **A finished job is the only moment a newer ring is scheduled**, so a walk cannot build up
        // a queue of rings whose chunks would be rejected when they arrived.
        if let Some(epoch) = finished {
            if self.job == Some(epoch) {
                self.job = None;
                if self.reschedule {
                    self.schedule();
                }
            }
        }
        &self.adopted
    }

    /// The far chunks currently drawn.
    pub fn chunks(&self) -> impl Iterator<Item = &FarChunk> {
        self.chunks.values()
    }

    /// Their meshes, for the frame's record path.
    pub fn meshes(&self) -> impl Iterator<Item = &Mesh> {
        self.chunks.values().map(|chunk| &chunk.mesh)
    }

    /// The positions currently drawn — what the light volume must keep stamped.
    pub fn positions(&self) -> impl Iterator<Item = ChunkPos> + '_ {
        self.chunks.keys().copied()
    }

    /// How many far chunks are drawn.
    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    /// **How many far chunks the current ring wants** — the number [`FarTerrain::len`] reaches
    /// when the ring has fully arrived. A caller waiting on a first fill compares the two rather
    /// than guessing the ring's size from the window's.
    pub fn scheduled(&self) -> usize {
        self.wanted.len()
    }

    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    /// Far chunks adopted since the field was created.
    pub fn builds(&self) -> u64 {
        self.builds
    }

    /// The quads the far field draws, the number the frame's cost is proportional to.
    pub fn quads(&self) -> usize {
        self.chunks
            .values()
            .map(|chunk| chunk.mesh.quads.len())
            .sum()
    }
}

impl Drop for FarTerrain {
    fn drop(&mut self) {
        // Close the job channel, then wait for the thread to finish the chunk it is on. Dropping
        // the receiver first would make the thread's send fail and return, which is the same end
        // without the wait; keeping both is what makes the shutdown deterministic.
        self.to_worker = None;
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        self.from_worker = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_shapers::{Config, WorldSpec};

    fn world() -> WorldSource {
        WorldSource::new(WorldSpec::new(crate::game::GAME_SEED, Config::default()))
    }

    #[test]
    fn the_census_sums_to_the_chunks_own_voxel_count() {
        let source = world();
        let pos = ChunkPos::new(1, 0, 1);
        let tree = source.build_chunk(pos).expect("a chunk of the world");
        let census = census(&tree);
        let solids: u64 = census.cells().iter().map(|cell| cell.solids as u64).sum();
        assert_eq!(
            solids,
            tree.solid_count(),
            "the census must account for every solid voxel: {} counted, {} in the tree",
            solids,
            tree.solid_count()
        );
    }

    #[test]
    fn a_far_chunk_is_meshed_at_the_eight_metre_cell_and_has_surface() {
        let source = world();
        let pos = ChunkPos::new(3, 0, 3);
        let tree = source.build_chunk(pos).expect("a chunk of the world");
        let census = census(&tree);
        let mesh = coarse_mesh(&census, pos, |_| None);
        assert!(
            !mesh.quads.is_empty(),
            "a surface chunk has faces at the coarse rung"
        );
        let stride = FAR_CELL_SUBVOXELS as u16;
        let side = census.side() as u16;
        for quad in &mesh.quads {
            // **Every rectangle is a cell's own roof or wall**, at the height that cell measured:
            // no plane may stand above the census and no wall may leave its cell's footprint.
            assert!(
                quad.slice <= side * stride,
                "a coarse quad stands above the census"
            );
            assert!(quad.extent[0] > 0 && quad.extent[1] > 0);
            match quad.face {
                Face::PosY => {
                    assert_eq!(quad.extent, [stride, stride], "a roof is one cell");
                    assert_eq!(quad.uv[0] % stride, 0, "a roof starts on a coarse cell");
                    assert_eq!(quad.uv[1] % stride, 0, "a roof starts on a coarse cell");
                }
                _ => {
                    assert_eq!(quad.slice % stride, 0, "a wall stands on a cell boundary");
                    assert!(quad.extent[0] <= stride * side && quad.extent[1] <= stride * side);
                }
            }
        }
        assert_eq!(mesh.origin, {
            let origin = pos.origin();
            [origin.x, origin.y, origin.z]
        });
        // **A roof a cell that holds matter, and no roof for a cell that holds none**: the rung
        // scales with cells, and a cell's roof is its own measurement at its own top.
        let matter = census
            .cells()
            .iter()
            .filter(|cell| cell.holds_matter())
            .count();
        let roofs = mesh
            .quads
            .iter()
            .filter(|quad| quad.face == Face::PosY)
            .count();
        assert_eq!(
            roofs, matter,
            "a cell with matter has one roof, and an empty cell none"
        );
        let partial = census
            .cells()
            .iter()
            .filter(|cell| cell.holds_matter() && !cell.solid())
            .count();
        // **The gate's own cost is measured rather than asserted here**: whether *this* chunk has a
        // cell below half is a fact about the ground under it, and
        // `measure_what_the_draw_gate_costs` is the test that counts them over eight chunks (27 of
        // 251 cells, 26 of them surface).
        println!(
            "chunk {pos:?} at eight metres: {matter} cells hold matter, {partial} of them below half"
        );
    }

    /// **How much matter the cells a rung draws actually hold**, chunk by chunk — the measurement
    /// the draw gate's own change is held to. A cell the ground cuts through is a cell that must
    /// still be drawn: it is the surface, and the band stamps its share whether the gate drew it or
    /// not.
    ///
    /// `cargo test -p voxel-client --lib measure_what_the_draw_gate_costs -- --ignored --nocapture`
    #[test]
    #[ignore = "measurement, not an assertion: run with `cargo test <filter> -- --ignored --nocapture`"]
    fn measure_what_the_draw_gate_costs() {
        let source = world();
        let mut chunks = 0usize;
        let mut matter = 0usize;
        let mut below_half = 0usize;
        let mut surface_holes = 0usize;
        for pos in [
            ChunkPos::new(0, 0, 0),
            ChunkPos::new(1, 0, 1),
            ChunkPos::new(3, 0, 3),
            ChunkPos::new(-3, 0, 2),
            ChunkPos::new(5, 0, -4),
            ChunkPos::new(-6, 1, 6),
            ChunkPos::new(9, 2, -9),
            ChunkPos::new(-12, -1, 11),
        ] {
            let Ok(tree) = source.build_chunk(pos) else {
                continue;
            };
            let census = census(&tree);
            chunks += 1;
            let side = census.side();
            let mut here_matter = 0usize;
            let mut here_half = 0usize;
            let mut here_holes = 0usize;
            for z in 0..side {
                for x in 0..side {
                    // The column's own topmost matter: the cell the surface is drawn for.
                    let top = (0..side)
                        .rev()
                        .find(|y| census.get([x, *y, z]).is_some_and(CoarseCell::holds_matter));
                    for y in 0..side {
                        let Some(cell) = census.get([x, y, z]) else {
                            continue;
                        };
                        if !cell.holds_matter() {
                            continue;
                        }
                        here_matter += 1;
                        if !cell.solid() {
                            here_half += 1;
                            if top == Some(y) {
                                here_holes += 1;
                            }
                        }
                    }
                }
            }
            println!(
                "{pos:?} at {} m: {here_matter} cells hold matter, {here_half} below half, \
                 {here_holes} of them the surface the eye sees",
                census.cell_m()
            );
            matter += here_matter;
            below_half += here_half;
            surface_holes += here_holes;
        }
        println!(
            "{chunks} chunks: {matter} cells hold matter, {below_half} below half ({:.2} per cent), \
             {surface_holes} of them surface",
            100.0 * below_half as f64 / matter.max(1) as f64
        );
    }

    /// **The range the field draws and the range it censuses, stated as numbers.**
    ///
    /// The peak test in `horizon.rs` proves the layer is *drawn and lit* on a real summit; this
    /// pins the geometry that gets it there: three layers, the same ring in each, and a census that
    /// reaches one layer past the stack at the ring's own columns (the top and bottom faces) and
    /// one horizontal chunk past it in every drawn layer (the side faces). A diagonal neighbour is
    /// never asked for, so it is never paid for.
    #[test]
    fn the_stack_reaches_one_layer_above_and_below_the_eye() {
        let centre = ChunkPos::new(0, 0, 0);
        let radius = 5;
        let stack = stack_at(radius, centre);
        assert_eq!(
            stack.len(),
            (2 * FAR_STACK + 1) as usize * 104,
            "three layers of one ring"
        );
        for dy in -FAR_STACK..=FAR_STACK {
            let layer = ring_at(radius, centre, dy);
            assert_eq!(layer.len(), 104, "the ring's own size");
            assert!(
                layer.iter().all(|pos| pos.y == centre.y + dy),
                "a layer holds one y"
            );
            assert!(
                layer.iter().all(|pos| stack.contains(pos)),
                "the stack lost a layer it draws"
            );
        }
        assert!(
            !stack.contains(&ChunkPos::new(6, FAR_STACK + 1, 0)),
            "the stack drew a layer beyond its bound"
        );

        let census = census_for(radius, centre);
        assert!(
            census.contains(&ChunkPos::new(6, FAR_STACK + 1, 0)),
            "the layer above the stack is not censused: the top faces would be unknown"
        );
        assert!(
            census.contains(&ChunkPos::new(8, 0, 0)),
            "the horizontal margin is not censused: the side faces would be unknown"
        );
        assert!(
            !census.contains(&ChunkPos::new(8, FAR_STACK + 1, 0)),
            "the census paid for a diagonal no face asks for"
        );
    }

    /// **The far pair is drawn by the real pass, and the ladder is told its cell size.**
    ///
    /// This is the end of the slice's wiring that a census test cannot see: the records leave
    /// [`crate::Scene::with_far`], reach the GPU through the far buffer pair, are expanded by
    /// `vs_quad` and shaded by `fs_main`. A geometry probe (the frame's centre must be covered) and
    /// a record probe (the spare slot carries eight metres) together say the field is *drawn* and
    /// not only built.
    #[test]
    fn a_far_mesh_is_drawn_and_states_the_cell_it_was_cut_at() {
        use voxel_appearance::npr::LightRig;

        let registry = voxel_shapers::materials::registry().expect("the registry");
        let field = voxel_shapers::materials::shading_field().expect("the field");
        let table = crate::shading::table(&registry, &field).expect("the material table");
        let rig = LightRig::standard();
        let gpu_rig = crate::shading::GpuRig::from_rig(&rig).expect("the standard rig");

        // A 16 m block of rock on the chunk's floor: two coarse cells a side, all fully solid.
        let mut tree = SparseVoxelOctree::new();
        tree.set_material_region([0, 0, 0], 4, voxel_shapers::materials::REGOLITH);
        let pos = ChunkPos::new(0, 0, 0);
        let census = census(&tree);
        let mesh = coarse_mesh(&census, pos, |_| None);
        assert!(!mesh.quads.is_empty(), "a solid block has a surface");

        let scene = crate::Scene::default().with_far(std::iter::once(&mesh));
        assert_eq!(scene.far.len(), mesh.quads.len(), "one record a rectangle");
        assert_eq!(scene.far_origins.len(), 1, "one origin a far chunk");
        let carried = f32::from_bits(scene.far[0].light_y);
        assert_eq!(
            carried, FAR_CELL_M as f32,
            "the record does not carry its cell size"
        );

        let eye = [8.0, 8.0, -24.0];
        let mut camera = crate::Camera::standard(eye, 1.0);
        camera.yaw_rad = 0.0;
        camera.pitch_rad = 0.0;
        let mut renderer = crate::Renderer::new(96, 96, &table, &gpu_rig).expect("a GPU");
        renderer.world_mut().write_no_sky_model();
        renderer.world_mut().horizon_recenter(eye);
        renderer.world_mut().horizon_stamp_far(pos, &census, 1);
        let frame = renderer.render(&scene, &camera).expect("a frame");
        let centre = frame.pixel(48, 48).expect("the centre is in range");
        assert_eq!(
            centre[3], 1.0,
            "the far surface does not cover the frame's centre"
        );
        let brightest = centre[0].max(centre[1]).max(centre[2]);
        assert!(
            brightest > 0.005,
            "the far surface is drawn black ({centre:?}): the record path shades nothing"
        );
    }

    /// **The tier-6 ring is outside the far ring and inside the next one.**
    ///
    /// The placement, not the build: `radius+3 ..= radius+4` shares no chunk with the tier-5
    /// `radius+1 ..= radius+2`, so no ground is drawn twice, and it stops one chunk short of where a
    /// third field would begin. With the window at radius 5 that is 256–320 m.
    #[test]
    fn the_second_ring_lies_outside_the_first() {
        let centre = ChunkPos::new(0, 0, 0);
        let outer = ring_between(5, centre, 0, 3, 4);
        assert!(!outer.is_empty(), "the second ring is empty");
        for dz in -9i32..=9 {
            for dx in -9i32..=9 {
                let distance = dx.abs().max(dz.abs());
                let pos = ChunkPos::new(dx, 0, dz);
                assert_eq!(
                    outer.contains(&pos),
                    distance == 8 || distance == 9,
                    "the second ring at ({dx}, {dz}) is not 8..=9 chunks out"
                );
            }
        }
        assert!(
            !ring_at(5, centre, 0).iter().any(|pos| outer.contains(pos)),
            "the two rings draw the same chunk"
        );
    }

    /// **An aggregated census is the same voxels measured at the coarser tier.**
    ///
    /// Two independent routes to tier 6: walk the tree at 16 m, or sum the 8 m census the far mesh
    /// already used. Every cell's share and the total matter must agree, or one of the two is a
    /// second reading of the world rather than the same one at another grain.
    #[test]
    fn a_census_aggregated_to_tier_six_is_the_same_measurement() {
        let source = world();
        let pos = ChunkPos::new(3, 0, 3);
        let tree = source.build_chunk(pos).expect("a chunk of the world");
        let eight = census(&tree);
        let summed = eight.aggregate(2);
        let direct = Census::of(&tree, crate::lod::WIDE_TIER);
        assert_eq!(summed.tier(), direct.tier());
        assert_eq!(summed.side(), direct.side());
        assert_eq!(summed.cell_voxels(), direct.cell_voxels());
        assert_eq!(summed.cells().len(), direct.cells().len());
        for (a, b) in summed.cells().iter().zip(direct.cells()) {
            assert_eq!(a.solids, b.solids, "a summed 16 m cell holds another count");
            assert_eq!(a.share(), b.share(), "a summed 16 m cell has another share");
        }
        let total: u64 = summed.cells().iter().map(|c| c.solids as u64).sum();
        assert_eq!(
            total,
            tree.solid_count(),
            "the aggregate lost or invented matter"
        );
    }

    /// **A group ring is made of whole groups, and it starts past the chunk rungs.**
    ///
    /// Placement, not a build: every position is a group's low chunk (a multiple of the span), it
    /// lies at the ring's own group distance, and the nearest one begins at chunk 12 — the first
    /// chunk after tier 7's 10..=11 ring. A half group here would be a cell that is not the cell.
    #[test]
    fn a_group_ring_is_whole_groups_past_the_chunk_ring() {
        let centre = ChunkPos::new(0, 0, 0);
        let ring = group_stack_between(centre, 2, 6, 6);
        assert!(!ring.is_empty(), "the group ring is empty");
        for pos in &ring {
            assert_eq!(pos.x.rem_euclid(2), 0, "{pos:?} is half a group on x");
            assert_eq!(pos.y.rem_euclid(2), 0, "{pos:?} is half a group on y");
            assert_eq!(pos.z.rem_euclid(2), 0, "{pos:?} is half a group on z");
            let distance = (pos.x / 2).abs().max((pos.z / 2).abs());
            assert_eq!(distance, 6, "{pos:?} is not on the sixth group ring");
            assert!(
                pos.y == 0 || pos.y == -2,
                "{pos:?} is outside the two group layers"
            );
            assert!(
                pos.x.abs().max(pos.z.abs()) >= 12,
                "{pos:?} overlaps tier 7's chunk ring"
            );
        }
    }

    /// **A group census is the sum of its chunks' own measurements.**
    ///
    /// The tier above the chunk-held ladder has no per-chunk census to read: its cell *is* the
    /// group. This is that measurement — eight chunks' one-cell censuses added into one 64 m cell —
    /// and the test holds it to the same voxels, not to a second walk of the world.
    #[test]
    fn a_group_census_is_the_sum_of_its_chunks() {
        let source = world();
        let origin = ChunkPos::new(2, 0, 2);
        let mut trees = Vec::new();
        let mut expected = 0u64;
        for dy in 0..2 {
            for dz in 0..2 {
                for dx in 0..2 {
                    let pos = ChunkPos::new(origin.x + dx, origin.y + dy, origin.z + dz);
                    let tree = source.build_chunk(pos).expect("a chunk of the world");
                    expected += Census::of(&tree, crate::lod::CHUNK_TIER).cells()[0].solids as u64;
                    trees.push((pos, tree));
                }
            }
        }
        let group = Census::of_group(origin, 2, crate::lod::GROUP_TIER, &trees);
        assert_eq!(group.side(), 1, "a group is one cell");
        assert_eq!(group.cell_m(), 64.0);
        assert_eq!(group.cell_voxels(), 64 * 64 * 64);
        assert_eq!(
            group.cells()[0].solids as u64,
            expected,
            "the group lost or invented the chunks' matter"
        );
    }

    /// **A group cell draws no wall into a neighbour nobody measured, and still draws its roof.**
    #[test]
    fn a_group_cell_draws_nothing_into_an_unmeasured_neighbour() {
        let voxels = 64 * 64 * 64;
        let census = Census {
            tier: crate::lod::GROUP_TIER,
            side: 1,
            cells: vec![CoarseCell {
                solids: voxels,
                voxels,
                top_voxel: 63,
                material: MaterialId(1),
            }],
        };
        let pos = ChunkPos::new(0, 0, 0);
        let open = coarse_mesh(&census, pos, |_| None);
        assert_eq!(
            open.quads
                .iter()
                .filter(|quad| quad.face != Face::PosY)
                .count(),
            0,
            "a group drew {} wall(s) into unmeasured neighbours",
            open.quads.len()
        );
        assert_eq!(
            open.quads.len(),
            1,
            "the cell's own roof is its measurement"
        );
        let measured = coarse_mesh(&census, pos, |_| Some(false));
        assert_eq!(
            measured.quads.len(),
            5,
            "a lone measured group is a roof and four walls; its floor is not a surface"
        );
    }

    /// **The ring arrives as it is measured, not all at once.**
    ///
    /// The worker delivers in waves: the first chunk adopted is evidence that the field's geometry
    /// exists while the rest of the ring is still being censused. A whole-ring delivery would fail
    /// this with `first == scheduled`.
    #[test]
    fn the_first_far_chunk_arrives_before_the_ring_is_complete() {
        let spec = WorldSpec::new(crate::game::GAME_SEED, Config::default());
        let mut field = FarTerrain::at_tier(spec, 0, crate::lod::FAR_TIER, 1, 2);
        field.set_eye(ChunkPos::new(0, 0, 0));
        let started = std::time::Instant::now();
        let mut arrived = 0usize;
        while started.elapsed() < std::time::Duration::from_secs(120) {
            if !field.advance(4).is_empty() {
                arrived = field.len();
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(arrived > 0, "no far chunk arrived");
        assert!(
            arrived < field.scheduled(),
            "the whole ring arrived at once ({arrived} of {})",
            field.scheduled()
        );
    }

    #[test]
    fn a_cell_at_or_above_half_is_solid_and_below_is_air() {
        let mut half = CoarseCell::default();
        half.voxels = FAR_CELL_VOXELS_TOTAL;
        half.solids = FAR_CELL_VOXELS_TOTAL / 2;
        assert!(
            half.solid(),
            "half a cell is the band's own solid threshold"
        );
        half.solids -= 1;
        assert!(!half.solid(), "one voxel below the threshold is air");
    }

    /// **A census is a tier's measurement, not the far tier's.**
    ///
    /// This is the structure a second rung needs: the same walk, the same mesher, a different
    /// cell. At sixteen metres a chunk is `2³` cells of 4096 voxels, every solid voxel is still
    /// accounted for, and every quad lands on a 64-subvoxel stride. Nothing in the light path is
    /// exercised here — that is the band's own tier, still uninhabited — but the geometry no
    /// longer has to be rewritten to inhabit it.
    /// **A far cell is a cuboid at the height it measured, not a plane at its ceiling.**
    #[test]
    fn a_far_cell_is_a_cuboid_at_the_height_it_measured() {
        let mut tree = SparseVoxelOctree::new();
        tree.set_material_region([8, 8, 8], 3, voxel_shapers::materials::REGOLITH);
        // The cell's top voxel layer carved off, so its height is inside the cell rather than at
        // its ceiling: 448 of 512 voxels, still matter, and a top the cell has to state.
        for x in 0..8 {
            for z in 0..8 {
                tree.set_material_region([8 + x, 15, 8 + z], 0, MaterialId::EMPTY);
            }
        }
        let pos = ChunkPos::new(0, 0, 0);
        let census = census(&tree);
        let cell = census.get([1, 1, 1]).expect("the cell");
        assert!(cell.solid(), "448 of 512 voxels is matter at this rung");
        assert_eq!(cell.solids, 448);
        assert_eq!(
            cell.top_voxel, 14,
            "the cell records the chunk-local voxel its matter tops at"
        );
        let mesh = coarse_mesh(&census, pos, |_| None);
        let roofs: Vec<_> = mesh
            .quads
            .iter()
            .filter(|quad| quad.face == Face::PosY)
            .collect();
        assert_eq!(roofs.len(), 1, "one roof for one solid cell");
        assert_eq!(
            roofs[0].slice,
            FAR_CELL_SUBVOXELS as u16 + 7 * SUBVOXEL_SIDE as u16,
            "the roof stands at the cell's own top voxel, not at its ceiling"
        );
        let walls: Vec<_> = mesh
            .quads
            .iter()
            .filter(|quad| quad.face != Face::PosY)
            .collect();
        assert_eq!(
            walls.len(),
            4,
            "four walls, because every neighbour is measured air"
        );
        for wall in walls {
            let height = match wall.face {
                Face::NegX | Face::PosX => wall.extent[0],
                _ => wall.extent[1],
            };
            assert_eq!(
                height,
                7 * SUBVOXEL_SIDE as u16,
                "a wall runs from the cell's floor to the roof it measured"
            );
        }
    }

    #[test]
    fn a_census_can_be_measured_at_another_tier() {
        let source = world();
        let pos = ChunkPos::new(3, 0, 3);
        let tree = source.build_chunk(pos).expect("a chunk of the world");
        let tier = crate::lod::FAR_TIER + 1;
        let census = Census::of(&tree, tier);
        assert_eq!(census.tier(), tier);
        assert_eq!(census.side(), 2, "sixteen-metre cells make a 2³ census");
        assert_eq!(census.cell_voxels(), 4096);
        assert!(
            census.cells().iter().all(|cell| cell.solids <= 4096),
            "a 16 m cell holds at most 4096 voxels: the denominator and the walk disagree"
        );
        let solids: u64 = census.cells().iter().map(|cell| cell.solids as u64).sum();
        assert_eq!(
            solids,
            tree.solid_count(),
            "the tier's census must still account for every solid voxel"
        );
        let mesh = coarse_mesh(&census, pos, |_| None);
        assert!(
            !mesh.quads.is_empty(),
            "a surface chunk has faces at the coarser rung"
        );
        let mut scene = crate::Scene::default();
        scene.add_far_at(std::iter::once(&mesh), crate::lod::WIDE_CELL_M);
        assert_eq!(
            f32::from_bits(scene.far[0].light_y),
            crate::lod::WIDE_CELL_M,
            "a tier-6 record must state sixteen metres, not the far tier's eight"
        );
        let stride = 16 * SUBVOXEL_SIDE as u16;
        for quad in &mesh.quads {
            assert!(
                quad.slice <= 2 * stride,
                "a 16 m quad stands above its census"
            );
            if quad.face == Face::PosY {
                assert_eq!(quad.extent, [stride, stride], "a roof is one 16 m cell");
                assert_eq!(quad.uv[0] % stride, 0);
                assert_eq!(quad.uv[1] % stride, 0);
            } else {
                assert_eq!(
                    quad.slice % stride,
                    0,
                    "a 16 m wall stands on a cell boundary"
                );
            }
        }
        let matter = census
            .cells()
            .iter()
            .filter(|cell| cell.holds_matter())
            .count();
        let roofs = mesh
            .quads
            .iter()
            .filter(|quad| quad.face == Face::PosY)
            .count();
        assert_eq!(
            roofs, matter,
            "a cell with matter has one roof, and an empty cell none"
        );
    }

    /// **A partly full cell is a low cuboid, not a hole.**
    ///
    /// The world is voxels at whatever cell a rung measures them in ([`crate::lod`]), so a cell
    /// that holds any matter is a **block**: its roof at the height that matter tops at, its walls
    /// where a measured neighbour stands lower. The old gate was [`CoarseCell::solid`] — half a
    /// cell — and a *surface* cell is exactly the cell the ground's own surface cuts through: **26
    /// of the 251 matter-holding cells of eight measured chunks are below half and are the surface**
    /// (`measure_what_the_draw_gate_costs`), every one of them drawn as **nothing** while the band
    /// had already stamped its share.
    #[test]
    fn a_partly_full_cell_is_a_low_cuboid_not_a_hole() {
        let voxels = FAR_CELL_VOXELS_TOTAL;
        let mut part = CoarseCell::default();
        part.voxels = voxels;
        part.solids = voxels / 4;
        part.top_voxel = 3;
        part.material = MaterialId(1);
        assert!(
            !part.solid(),
            "a quarter of a cell is below the band's own threshold"
        );
        assert!(
            part.holds_matter(),
            "and it is still matter, which is what the geometry asks"
        );
        // Its own block, at its own top: five voxels' worth of cuboid inside an eight-metre cell.
        let census = Census {
            tier: crate::lod::FAR_TIER,
            side: 1,
            cells: vec![part],
        };
        let mesh = coarse_mesh(&census, ChunkPos::new(0, 0, 0), |_| None);
        let roofs: Vec<_> = mesh
            .quads
            .iter()
            .filter(|quad| quad.face == Face::PosY)
            .collect();
        assert_eq!(roofs.len(), 1, "a partly full cell is still a block");
        assert_eq!(
            roofs[0].slice,
            4 * SUBVOXEL_SIDE as u16,
            "its roof is its own top voxel, not the cell's ceiling"
        );
        assert_eq!(
            roofs[0].extent,
            [FAR_CELL_SUBVOXELS as u16, FAR_CELL_SUBVOXELS as u16],
            "and it is a cell wide, like every other block of the rung"
        );
        // **And it is a block beside its neighbours, not air**: the same census with every
        // neighbour measured empty draws the four walls of the cuboid, from the cell's floor up.
        let alone = coarse_mesh(&census, ChunkPos::new(0, 0, 0), |_| Some(false));
        let walls: Vec<_> = alone
            .quads
            .iter()
            .filter(|quad| quad.face != Face::PosY)
            .collect();
        assert_eq!(
            walls.len(),
            4,
            "a lone partly full cell is a cuboid with four walls"
        );
        for wall in walls {
            let height = match wall.face {
                Face::NegX | Face::PosX => wall.extent[0],
                _ => wall.extent[1],
            };
            assert_eq!(
                height,
                4 * SUBVOXEL_SIDE as u16,
                "the wall runs from the cell's floor"
            );
        }
    }
}
