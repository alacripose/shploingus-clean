//! **The light horizon: what stands between a cell and the sky, streamed as the world streams.**
//!
//! The frame's light used to be one global beam and one global ambient. A shadow map can answer
//! *"is this pixel behind the world from the Sun?"* for one direction, but it cannot answer the
//! other half of a cell's light — **how much of the dome it can see** — and it has no opinion at
//! all about the air the beam crossed before it reached the world. This module is the second half:
//! a small world-aligned volume of **occupancy**, built from each resident chunk's **subvoxel
//! octree**, that `shading.wgsl` traverses from the fragment toward the frame's beam and toward the
//! sky. The beam itself is still the solver's own `voxel_light` reading, transmitted through the
//! atmosphere (`skylight::WorldLight::beam_rgb` and `sky_radiance_rgb`), and at night it is the
//! Sun's light **reflected off the Moon** (`voxel_light::moonlight`): the march only decides how
//! much of either reaches this cell.
//!
//! # Why a volume and not a per-corner band
//!
//! A per-corner measurement is built on the CPU and arrives a chunk at a time, which is the
//! defect PATCH-074 removed from the picture: the light lagged the geometry. A volume is
//! **streamed**: a chunk that lands stamps its own 32³ voxels, a chunk that leaves leaves air,
//! and the fragment samples whatever is resident *this frame* rather than a number baked into a
//! vertex last second. The march happens where the frame needs it — at the fragment — so no cell
//! the eye is not looking at is ever marched.
//!
//! # The grain, and why it is the octree's own
//!
//! One cell is **one metre**, the octree's voxel, and each cell carries the **64 bits of that
//! voxel's 4×4×4 subvoxel grid** — the octree's leaf shape, not a boolean derived from it. A
//! half-dug voxel therefore shadows exactly the quarters that still hold matter, which is what
//! `voxel_stream::light::transmittance_tree` measures on the CPU and what the player sees when a
//! block is broken: the shadow of a broken corner has a corner missing.
//!
//! The words are **two `u32` per metre rather than one `u64`**, because WGSL has no 64-bit
//! integer: bit `x + 4y + 16z` of the pair is subvoxel `(x, y, z)`, low word first, which is
//! `SubvoxelIndex::flat` in `voxel-model` written twice.
//!
//! # The box, and what is outside it
//!
//! The box is three chunks square — [`HORIZON_CELLS`] × [`HORIZON_CELL_M`] = 96 m — with the eye's
//! chunk in the middle, so it follows the eye one chunk at a time and covers a chunk behind the
//! camera as well as the chunk ahead of it. **Past its edge there is no
//! measurement, and the shader says so by returning `1.0`** rather than treating the edge as
//! clear air: outside the volume the frame's answer is the shadow map's (or the rig's), which is
//! the light the frame had before this field existed and a different claim from "this cell sees
//! the whole sky". The box is inside the shadow map's own 256 m range, so the two terms overlap
//! where both are measured and the near field is the finer of the two.
//!
//! # A stamp belongs to a version of a tree
//!
//! A stamp is a fact about a *tree*, and a tree is a fact about the world record at one moment.
//! `Streamer::edit` records the change, and the rebuilt chunk arrives carrying a new **version**
//! (`Streamer::tree_version`, bumped whenever a chunk's mesh or tree enters or is replaced). This
//! volume keeps the version it stamped each chunk at, so **an edit is a chunk that wants
//! stamping again** and a chunk that has not moved costs one map lookup. A stamp that outlived
//! its tree would be the shadow of a block that is no longer there: break one out of a wall and
//! the hole has to let light through on the frame the new tree lands, not at the next recentre.
//!
//! # One body, or the other
//!
//! `WorldLight` switches the beam from the Sun to the Moon at the solver level, with the Moon's
//! bands being the Sun's own reflected (`voxel_light::moonlight`). The march takes the direction
//! and the spectrum it is handed: **it does not know which body it is lighting from**, and that
//! is deliberate — a second body in this module would be a second atmosphere.

use std::collections::{BTreeMap, BTreeSet};

use voxel_mesh::octree::Node;
use voxel_mesh::SparseVoxelOctree;
use voxel_model::{ChunkPos, CHUNK_SIDE, SUBVOXEL_SIDE};

/// **Metres per volume cell** — the octree's own voxel.
pub const HORIZON_CELL_M: f32 = 1.0;

/// Cells along one edge of the volume: `96`, so the box is [`HORIZON_CELL_M`] × this = 96 m.
pub const HORIZON_CELLS: u32 = 96;

/// **Subvoxels along one edge of a cell** — `SUBVOXEL_SIDE`, the model's own 4.
pub const HORIZON_SUBVOXEL_SIDE: u32 = SUBVOXEL_SIDE;

/// **Metres per subvoxel** — `0.25`, the grain the octree's leaf shapes are authored at.
pub const HORIZON_SUBVOXEL_M: f32 = HORIZON_CELL_M / HORIZON_SUBVOXEL_SIDE as f32;

/// The `u32` words one cell's 64-bit mask occupies: two, because WGSL has no `u64`.
pub const HORIZON_WORDS_PER_CELL: usize = 2;

/// **How far the fragment's beam march walks in the fine band**, metres — the box's own span, so
/// the subvoxel answer is used everywhere it exists and the coarse band takes over only past it.
pub const HORIZON_REACH_M: f32 = 96.0;

/// **How far the dome's own rays walk**, metres: the sky a cell can see is decided by what stands
/// within a few chunks of it, and a dome ray is spent on a share of an ambient rather than on a
/// shadow a player can walk up to.
pub const HORIZON_SKY_REACH_M: f32 = 32.0;

/// **Metres per coarse cell** — the far band's grain, one eighth of the world's chunk.
pub const HORIZON_FAR_CELL_M: f32 = 8.0;

/// Coarse cells along one edge of the far band: 32 × [`HORIZON_FAR_CELL_M`] = 256 m.
pub const HORIZON_FAR_CELLS: u32 = 32;

/// **How far a far-band ray walks**, metres — a quarter of the band's diagonal, so a ray that
/// starts at the eye has the whole declared horizon ahead of it and not just half of one side.
pub const HORIZON_FAR_REACH_M: f32 = 160.0;

/// Chunks along one edge of the far band: 256 m / 32 m.
pub const HORIZON_FAR_SLOTS: i32 = 8;

/// The far band's word: **bits 0–7 are the share of this cell that holds matter** (`0` empty,
/// `255` full), and bit 8 says the cell was stamped.
///
/// **A share rather than a flag**, because a coarse cell is the same octree data at a coarser
/// resolution: eight metres of a chunk holds a *fraction* of matter, and the march lets that
/// fraction of the ray through. That is what makes the far field a lower-detail level of the same
/// volume instead of a wall of eight-metre stones, and it is why the two bands agree where they
/// meet: the coarse share is the mean of the fine cells it covers.
pub const FAR_SHARE: u32 = 0xFF;
/// See [`FAR_SHARE`]: the coarse cell has been looked at.
pub const FAR_MEASURED: u32 = 1 << 8;

/// The three directions the dome is sampled along, in the shader's own order.
///
/// **Three rays, and the count is declared.** A dome is not three rays, and this module says so
/// rather than implying an integral: the sample is the same three directions at every fragment,
/// so two cells' skies are comparable, and the shader's `horizon_sky` is their mean.
pub const SKY_RAYS: [[f32; 3]; 3] = [
    [0.0, 1.0, 0.0],
    [0.707_106_77, 0.707_106_77, 0.0],
    [0.0, 0.707_106_77, -0.707_106_77],
];

/// The volume's uniform: the anchor in the fragment's own eye-relative frame, the cell and the
/// three dimensions, and **which chunk slots have been stamped**.
///
/// **`to_anchor` is `anchor − eye` and not the anchor itself**, because the fragment already has
/// its point relative to the eye (`shading.wgsl`'s `eye_rel`). Keeping the subtraction in a small
/// number rather than a world coordinate is what keeps a metre a metre a kilometre from the
/// origin.
///
/// **The coverage mask exists because a clear cell and an unstamped cell are not the same fact.**
/// Zero means "nothing stands here", which is a measurement; a chunk that has not landed yet has
/// no measurement at all, and a march that read its empty cells as clear air would refuse the
/// shadow map for a whole box of world it had never looked at. The cube is exactly 3 × 3 chunk
/// slots a side (the anchor is a chunk corner), so **one bit per slot** — 27 of the word's 32 —
/// says which of them the volume has looked at.
const UNIFORM_BYTES: u64 = 80;

/// **The streamed occupancy volume, and the two GPU handles the world pass reads it through.**
///
/// The cells are [`HORIZON_WORDS_PER_CELL`] words per metre — the octree's own 4×4×4 occupancy,
/// not a boolean and not one word a cell, because the march's bit test *is* the subvoxel address
/// law and a coarser word would be a second grid the CPU measurement does not have. One cube is
/// 96³ × 8 B = 6.75 MB, uploaded only on a frame whose contents moved.
pub struct HorizonStream {
    /// `HORIZON_WORDS_PER_CELL` words per cell, in `x + n·(y + n·z)` order.
    cells: Vec<u32>,
    /// **The far band**: one [`FAR_SHARE`]/[`FAR_MEASURED`] word per 8 m cell, 32³ = 256 m.
    ///
    /// It is what lets a march reach a kilometre of tunnel at a cost a frame can carry: an empty
    /// coarse cell is one compare for eight metres, and a full one stops the ray. Its grain is the
    /// chunk mesh's own second rung rather than four cells of the near band, so past the fine
    /// band's edge the answer is eight times coarser and arrives eight times cheaper — the same
    /// trade every LOD in this client makes, stated rather than implied.
    far: Vec<u32>,
    /// The world-voxel corner of cell `(0, 0, 0)`.
    anchor: [i32; 3],
    /// The world-voxel corner of far cell `(0, 0, 0)`.
    far_anchor: [i32; 3],
    /// **Chunks stamped into this anchor's cube, and the version of the tree each was stamped
    /// from** — so a chunk is walked once per version of it and an edit is walked again.
    stamped: BTreeMap<(i32, i32, i32), u64>,
    /// Whether the CPU cube has moved since the last upload.
    dirty: bool,
    buffer: wgpu::Buffer,
    far_buffer: wgpu::Buffer,
    uniform: wgpu::Buffer,
}

impl HorizonStream {
    /// The cube, empty and anchored at the origin until a frame recentres it.
    pub fn new(device: &wgpu::Device) -> Self {
        let cells = (HORIZON_CELLS * HORIZON_CELLS * HORIZON_CELLS) as usize;
        let bytes = (cells * HORIZON_WORDS_PER_CELL * 4) as u64;
        let far_cells = (HORIZON_FAR_CELLS * HORIZON_FAR_CELLS * HORIZON_FAR_CELLS) as usize;
        HorizonStream {
            cells: vec![0; cells * HORIZON_WORDS_PER_CELL],
            far: vec![0; far_cells],
            anchor: [0, 0, 0],
            far_anchor: [0, 0, 0],
            stamped: BTreeMap::new(),
            dirty: true,
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("light-horizon"),
                size: bytes,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            far_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("light-horizon-far"),
                size: (far_cells * 4) as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            uniform: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("light-horizon-view"),
                size: UNIFORM_BYTES,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
        }
    }

    /// **The two entries the world pass adds for the horizon** — the cells at binding 10 and the
    /// view at 11, after the shadow's 6–9.
    pub(crate) fn layout_entries() -> [wgpu::BindGroupLayoutEntry; 3] {
        let storage = |binding: u32| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        [
            storage(10),
            wgpu::BindGroupLayoutEntry {
                binding: 11,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            storage(12),
        ]
    }

    /// The bind-group entries that pair with [`HorizonStream::layout_entries`].
    pub(crate) fn bind_entries(&self) -> [wgpu::BindGroupEntry<'_>; 3] {
        [
            wgpu::BindGroupEntry { binding: 10, resource: self.buffer.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 11, resource: self.uniform.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 12, resource: self.far_buffer.as_entire_binding() },
        ]
    }

    /// The world-voxel corner the cube is anchored on.
    pub fn anchor(&self) -> [i32; 3] {
        self.anchor
    }

    /// **Which of the cube's 3 × 3 × 3 chunk slots hold a stamp**, as `sx + 3·(sy + 3·sz)` bits
    /// — the mask the shader reads to tell an empty cell from an unmeasured one.
    pub fn coverage(&self) -> u32 {
        let mut mask = 0u32;
        for ((x, y, z), _version) in &self.stamped {
            let slot = [
                (x * CHUNK_SIDE - self.anchor[0]).div_euclid(CHUNK_SIDE),
                (y * CHUNK_SIDE - self.anchor[1]).div_euclid(CHUNK_SIDE),
                (z * CHUNK_SIDE - self.anchor[2]).div_euclid(CHUNK_SIDE),
            ];
            if slot.iter().any(|v| *v < 0 || *v >= 3) {
                continue;
            }
            mask |= 1 << (slot[0] + 3 * (slot[1] + 3 * slot[2]));
        }
        mask
    }

    /// Whether a chunk's 32 m box touches the cube.
    pub fn covers(&self, pos: ChunkPos) -> bool {
        let (origin, far) = self.box_of(pos);
        origin[0] < far[0] && origin[1] < far[1] && origin[2] < far[2]
    }

    /// **Whether this chunk's cube still needs this version of its tree.**
    ///
    /// True when the chunk is in the box and the version it was stamped at is not the version it
    /// has now — which is every chunk after a recentre, the chunk an edit rebuilt, and no chunk
    /// on an ordinary frame of a still world.
    pub fn wants(&self, pos: ChunkPos, version: u64) -> bool {
        self.covers(pos) && self.stamped.get(&(pos.x, pos.y, pos.z)) != Some(&version)
    }

    /// **Move the cube to the eye's own chunk, and carry what it already knows.**
    ///
    /// Returns whether it moved. The anchor is the eye's chunk **minus one on each axis**, so a
    /// 96 m cube is three chunks a side with the eye's own chunk in the middle: a chunk behind
    /// the camera and the chunk ahead of it are both measured.
    ///
    /// **A crossing shifts the bands; it does not clear them.** The cube is a window on world space,
    /// and stepping it one chunk along moves two thirds of the world it holds. Clearing it and
    /// restamping all twenty-seven chunks is a frame that walks every leaf of three chunks it had
    /// already walked — the stall at a chunk boundary, and the reason the light arrived a chunk at a
    /// time. So the cells are copied to their new addresses, and every chunk still inside the new box
    /// keeps its stamped version. What fell out is forgotten; the chunks that entered are the only
    /// work the next stamp loop has to do.
    pub fn recenter(&mut self, eye_m: [f64; 3]) -> bool {
        let chunk = chunk_of(eye_m);
        let anchor = [
            (chunk.x - 1) * CHUNK_SIDE,
            (chunk.y - 1) * CHUNK_SIDE,
            (chunk.z - 1) * CHUNK_SIDE,
        ];
        if anchor == self.anchor {
            return false;
        }
        // **The far band is centred on the eye's own chunk too**, four chunks out on each axis: a
        // 256 m box with the player in the middle, so a tunnel mouth and the skyline are both in
        // it and nothing the eye can walk toward arrives half a box late.
        let far_anchor = [
            (chunk.x - HORIZON_FAR_SLOTS / 2) * CHUNK_SIDE,
            (chunk.y - HORIZON_FAR_SLOTS / 2) * CHUNK_SIDE,
            (chunk.z - HORIZON_FAR_SLOTS / 2) * CHUNK_SIDE,
        ];
        // Where the old bands' first cells land in the new ones, in their own grains. A chunk is 32
        // m and a coarse cell 8 m, so the far shift is four whole cells on every crossing.
        let cell_shift = [
            self.anchor[0] - anchor[0],
            self.anchor[1] - anchor[1],
            self.anchor[2] - anchor[2],
        ];
        let far_shift = [
            (self.far_anchor[0] - far_anchor[0]) / HORIZON_FAR_CELL_M as i32,
            (self.far_anchor[1] - far_anchor[1]) / HORIZON_FAR_CELL_M as i32,
            (self.far_anchor[2] - far_anchor[2]) / HORIZON_FAR_CELL_M as i32,
        ];
        self.cells = shifted(
            &self.cells,
            HORIZON_CELLS as i32,
            HORIZON_WORDS_PER_CELL,
            cell_shift,
        );
        self.far = shifted(&self.far, HORIZON_FAR_CELLS as i32, 1, far_shift);
        self.anchor = anchor;
        self.far_anchor = far_anchor;
        // **A stamp survives only while its chunk is still in the box.** Its cells moved with the
        // band, so the version it was taken at is still the version standing in them.
        let n = HORIZON_CELLS as i32;
        self.stamped.retain(|key, _| {
            let origin = ChunkPos::new(key.0, key.1, key.2).origin();
            [origin.x, origin.y, origin.z]
                .iter()
                .enumerate()
                .all(|(axis, start)| {
                    *start < anchor[axis] + n && *start + CHUNK_SIDE > anchor[axis]
                })
        });
        self.dirty = true;
        true
    }

    /// **Forget the chunks a resident set no longer holds.**
    ///
    /// A chunk that leaves the window leaves no occluder behind: its trees are gone, so the cells
    /// stamped from them describe a world that is no longer streamed. Both bands are cleared **for
    /// that chunk's own box** — a coarse cell is 8 m and a chunk is four of them, aligned to its
    /// corner, so no neighbour shares one. Clearing the coarse band whole instead (which this did)
    /// was a bug with two faces: it threw away every other chunk's summary, and nothing restamped
    /// them, because their versions had not changed — so the far field went blank on the frame the
    /// window moved and stayed blank on every frame after it.
    ///
    /// Returns whether anything was forgotten.
    pub fn retain(&mut self, resident: &BTreeSet<(i32, i32, i32)>) -> bool {
        let gone: Vec<(i32, i32, i32)> = self
            .stamped
            .keys()
            .copied()
            .filter(|key| !resident.contains(key))
            .collect();
        if gone.is_empty() {
            return false;
        }
        for key in &gone {
            self.stamped.remove(key);
            let base = [key.0 * CHUNK_SIDE, key.1 * CHUNK_SIDE, key.2 * CHUNK_SIDE];
            self.clear_chunk(base);
            self.clear_far_chunk(base);
        }
        self.dirty = true;
        true
    }

    /// **Stamp one resident chunk's octree into the cube, at the octree's own subvoxel grain.**
    ///
    /// A `Uniform(m)` region sets every cell it covers solid — all 64 bits, because one material
    /// filling a voxel is 64 solid subvoxels — and a `Mixed(shape)` leaf sets exactly the bits its
    /// 4×4×4 grid holds matter in. That is the whole of the question the march asks, and it is
    /// asked of the same leaves the mesher reads, so a dug quarter is a hole in the shadow rather
    /// than a whole metre of darkness.
    ///
    /// The chunk's own cells are **cleared first**, because a chunk replaces the one it was: an
    /// edit is served by a new tree for the same chunk, and a stamp that only ever added bits
    /// would keep the block the player broke. Chunks do not overlap, so this costs no neighbour's
    /// cells.
    ///
    /// Returns the number of cells the chunk marked (including cells already carrying a mask).
    pub fn stamp(&mut self, pos: ChunkPos, tree: &SparseVoxelOctree, version: u64) -> usize {
        if !self.wants(pos, version) {
            return 0;
        }
        self.stamped.insert((pos.x, pos.y, pos.z), version);
        let origin = pos.origin();
        let base = [origin.x, origin.y, origin.z];
        self.clear_chunk(base);
        let mut marked = 0usize;
        // **The coarse band first, and measured even where the chunk is all air.** The 4 × 4 × 4
        // far cells this chunk owns are its own: 32 m of chunk is four 8 m cells aligned to the
        // chunk's corner, so clearing them is not a neighbour's loss and a chunk of sky is a
        // measured hole rather than an unstamped one.
        for fy in 0..4 {
            for fz in 0..4 {
                for fx in 0..4 {
                    let corner = [
                        base[0] + fx * HORIZON_FAR_CELL_M as i32,
                        base[1] + fy * HORIZON_FAR_CELL_M as i32,
                        base[2] + fz * HORIZON_FAR_CELL_M as i32,
                    ];
                    if let Some(index) = self.far_index(corner) {
                        self.far[index] = FAR_MEASURED;
                    }
                }
            }
        }
        // **The coarse band's own census**, in solid voxels out of the 512 an 8 m cell holds. Every
        // leaf adds its own volume — `Uniform` all of it, `Mixed` its solid subvoxels — so a cell's
        // share is the mean of the fine band over that cell and the two agree at the handover.
        let mut solids = [[[0u32; 4]; 4]; 4];
        tree.visit_leaves(&mut |local: [i32; 3], level: u32, node: &Node| {
            match node {
                Node::Empty | Node::Branch(_) => {}
                Node::Uniform(_) => {
                    let side = 1i32 << level;
                    add_solid_volume(&mut solids, local, side, (side * side * side) as u32);
                    let words = [u32::MAX, u32::MAX];
                    for y in local[1]..local[1] + side {
                        for z in local[2]..local[2] + side {
                            for x in local[0]..local[0] + side {
                                if self.mark(
                                    [base[0] + x, base[1] + y, base[2] + z],
                                    words,
                                ) {
                                    marked += 1;
                                }
                            }
                        }
                    }
                }
                Node::Mixed(shape) => {
                    let words = subvoxel_words(shape.cells());
                    if words != [0, 0]
                        && self.mark([base[0] + local[0], base[1] + local[1], base[2] + local[2]], words)
                    {
                        marked += 1;
                    }
                    add_solid_volume(
                        &mut solids,
                        local,
                        1,
                        words[0].count_ones() + words[1].count_ones(),
                    );
                }
            }
        });
        for fy in 0..4 {
            for fz in 0..4 {
                for fx in 0..4 {
                    let corner = [
                        base[0] + fx * HORIZON_FAR_CELL_M as i32,
                        base[1] + fy * HORIZON_FAR_CELL_M as i32,
                        base[2] + fz * HORIZON_FAR_CELL_M as i32,
                    ];
                    let solid = solids[fz as usize][fy as usize][fx as usize];
                    let share = (solid * FAR_SHARE / 512).min(FAR_SHARE);
                    if let Some(index) = self.far_index(corner) {
                        self.far[index] |= share;
                    }
                }
            }
        }
        self.dirty = true;
        marked
    }

    /// The coarse cell index of a world corner aligned to the far grid, or `None` outside it.
    fn far_index(&self, world: [i32; 3]) -> Option<usize> {
        let cell = HORIZON_FAR_CELL_M as i32;
        let q = [
            (world[0] - self.far_anchor[0]).div_euclid(cell),
            (world[1] - self.far_anchor[1]).div_euclid(cell),
            (world[2] - self.far_anchor[2]).div_euclid(cell),
        ];
        let n = HORIZON_FAR_CELLS as i32;
        if q.iter().any(|v| *v < 0 || *v >= n) {
            return None;
        }
        Some((q[0] + n * (q[1] + n * q[2])) as usize)
    }

    /// **Whether the coarse cell at a world point holds matter**, or `None` where it is unstamped
    /// or outside the band.
    pub fn far_solid_at(&self, world_m: [f64; 3]) -> Option<bool> {
        Some(self.far_share_at(world_m)? >= 0.5)
    }

    /// **How much of the coarse cell at a world point holds matter** — `Some(0.0)` for a stamped
    /// empty cell, `None` where it is unstamped or outside the band.
    pub fn far_share_at(&self, world_m: [f64; 3]) -> Option<f32> {
        let cell = HORIZON_FAR_CELL_M as f64;
        let corner = [
            ((world_m[0] / cell).floor() as i32) * HORIZON_FAR_CELL_M as i32,
            ((world_m[1] / cell).floor() as i32) * HORIZON_FAR_CELL_M as i32,
            ((world_m[2] / cell).floor() as i32) * HORIZON_FAR_CELL_M as i32,
        ];
        let word = *self.far.get(self.far_index(corner)?)?;
        if word & FAR_MEASURED == 0 {
            return None;
        }
        Some((word & FAR_SHARE) as f32 / FAR_SHARE as f32)
    }

    /// Clear the coarse cells one chunk's box owns: four 8 m cells a side, aligned to its corner.
    fn clear_far_chunk(&mut self, base: [i32; 3]) {
        let step = HORIZON_FAR_CELL_M as i32;
        for fy in 0..4 {
            for fz in 0..4 {
                for fx in 0..4 {
                    let corner = [
                        base[0] + fx * step,
                        base[1] + fy * step,
                        base[2] + fz * step,
                    ];
                    if let Some(index) = self.far_index(corner) {
                        self.far[index] = 0;
                    }
                }
            }
        }
    }

    /// Clear the cells one chunk's box covers inside the cube.
    fn clear_chunk(&mut self, base: [i32; 3]) {
        let n = HORIZON_CELLS as i32;
        let near = [
            base[0].max(self.anchor[0]),
            base[1].max(self.anchor[1]),
            base[2].max(self.anchor[2]),
        ];
        let far = [
            (base[0] + CHUNK_SIDE).min(self.anchor[0] + n),
            (base[1] + CHUNK_SIDE).min(self.anchor[1] + n),
            (base[2] + CHUNK_SIDE).min(self.anchor[2] + n),
        ];
        for y in near[1]..far[1] {
            for z in near[2]..far[2] {
                for x in near[0]..far[0] {
                    if let Some(index) = self.index([x, y, z]) {
                        let at = index * HORIZON_WORDS_PER_CELL;
                        self.cells[at] = 0;
                        self.cells[at + 1] = 0;
                    }
                }
            }
        }
    }

    /// OR one world voxel's mask into its cell, if the cell is inside the cube.
    fn mark(&mut self, world: [i32; 3], words: [u32; 2]) -> bool {
        let Some(index) = self.index(world) else {
            return false;
        };
        let at = index * HORIZON_WORDS_PER_CELL;
        let already = self.cells[at] != 0 || self.cells[at + 1] != 0;
        self.cells[at] |= words[0];
        self.cells[at + 1] |= words[1];
        !already
    }

    /// The cell index of a world voxel, or `None` outside the cube.
    fn index(&self, world: [i32; 3]) -> Option<usize> {
        let q = [
            world[0] - self.anchor[0],
            world[1] - self.anchor[1],
            world[2] - self.anchor[2],
        ];
        let n = HORIZON_CELLS as i32;
        if q.iter().any(|v| *v < 0 || *v >= n) {
            return None;
        }
        Some((q[0] + n * (q[1] + n * q[2])) as usize)
    }

    /// **Whether the cell at a world point holds any matter**, by the cube's own address law.
    ///
    /// The CPU mirror of the shader's voxel lookup, kept for tests and for a caller that has to
    /// say what the volume holds; the march itself is the shader's, because a second march here
    /// would be a second light.
    pub fn solid_at(&self, world_m: [f64; 3]) -> Option<bool> {
        let words = self.mask_at(world_m)?;
        Some(words != [0, 0])
    }

    /// **Whether the subvoxel at a world point is solid** — the address law
    /// `shading.wgsl`'s `horizon_subvoxel_solid` states, transcribed once for a test to hold.
    pub fn subvoxel_at(&self, world_m: [f64; 3]) -> Option<bool> {
        let words = self.mask_at(world_m)?;
        let cell = HORIZON_CELL_M as f64;
        let at: Vec<u32> = (0..3)
            .map(|axis| {
                let fraction = (world_m[axis] / cell).rem_euclid(1.0);
                (fraction * HORIZON_SUBVOXEL_SIDE as f64).floor() as u32
            })
            .collect();
        let ordinal = at[0] + HORIZON_SUBVOXEL_SIDE * (at[1] + HORIZON_SUBVOXEL_SIDE * at[2]);
        let word = words[(ordinal / 32) as usize];
        Some((word >> (ordinal % 32)) & 1 == 1)
    }

    /// The two words of the mask covering a world point, or `None` outside the cube.
    fn mask_at(&self, world_m: [f64; 3]) -> Option<[u32; 2]> {
        let cell = HORIZON_CELL_M as f64;
        let index = self.index([
            (world_m[0] / cell).floor() as i32,
            (world_m[1] / cell).floor() as i32,
            (world_m[2] / cell).floor() as i32,
        ])?;
        let at = index * HORIZON_WORDS_PER_CELL;
        Some([self.cells[at], self.cells[at + 1]])
    }

    /// **Send the cube and the view to the frame**, if either moved.
    ///
    /// The view is written every frame because the eye moves every frame and `to_anchor` is
    /// relative to it; the cube is written only when a chunk stamped it, an edit forgot it or the
    /// cube recentred.
    pub fn write(&mut self, queue: &wgpu::Queue, eye_m: [f64; 3]) {
        if self.dirty {
            let mut bytes = Vec::with_capacity(self.cells.len() * 4);
            for cell in &self.cells {
                bytes.extend_from_slice(&cell.to_le_bytes());
            }
            queue.write_buffer(&self.buffer, 0, &bytes);
            self.dirty = false;
        }
        if self.dirty {
            let mut far = Vec::with_capacity(self.far.len() * 4);
            for cell in &self.far {
                far.extend_from_slice(&cell.to_le_bytes());
            }
            queue.write_buffer(&self.far_buffer, 0, &far);
        }
        let mut view = [0.0f32; 20];
        for axis in 0..3 {
            view[axis] = self.anchor[axis] as f32 - eye_m[axis] as f32;
            view[8 + axis] = self.far_anchor[axis] as f32 - eye_m[axis] as f32;
        }
        view[4] = HORIZON_CELL_M;
        view[5] = HORIZON_CELLS as f32;
        view[6] = HORIZON_CELLS as f32;
        view[7] = HORIZON_CELLS as f32;
        view[12] = HORIZON_FAR_CELL_M;
        view[13] = HORIZON_FAR_CELLS as f32;
        view[14] = HORIZON_FAR_CELLS as f32;
        view[15] = HORIZON_FAR_CELLS as f32;
        let mut bytes = [0u8; UNIFORM_BYTES as usize];
        for (slot, value) in view.iter().enumerate() {
            bytes[slot * 4..slot * 4 + 4].copy_from_slice(&value.to_le_bytes());
        }
        bytes[64..68].copy_from_slice(&self.coverage().to_le_bytes());
        queue.write_buffer(&self.uniform, 0, &bytes);
    }

    fn box_of(&self, pos: ChunkPos) -> ([i32; 3], [i32; 3]) {
        let origin = pos.origin();
        let near = [
            origin.x.max(self.anchor[0]),
            origin.y.max(self.anchor[1]),
            origin.z.max(self.anchor[2]),
        ];
        let far = [
            (origin.x + CHUNK_SIDE).min(self.anchor[0] + HORIZON_CELLS as i32),
            (origin.y + CHUNK_SIDE).min(self.anchor[1] + HORIZON_CELLS as i32),
            (origin.z + CHUNK_SIDE).min(self.anchor[2] + HORIZON_CELLS as i32),
        ];
        (near, far)
    }
}

/// **One band, moved to its new anchor**, zeroing the cells the old anchor did not hold.
///
/// `shift` is where the old band's cell `0` lands in the new one, so a positive component is a band
/// that moved backwards into the cube and a negative one is a band that moved forwards. A shift at
/// least as large as the band is a clear: nothing overlaps, and copying nothing is the honest copy.
///
/// The `x` axis is contiguous in the index, so this is one row copy per `(y, z)` line rather than a
/// cell at a time — 96 MB/s of work would not be, and a crossing is a frame the player is watching.
fn shifted(words: &[u32], side: i32, stride: usize, shift: [i32; 3]) -> Vec<u32> {
    let mut moved = vec![0u32; words.len()];
    if shift.iter().any(|d| d.abs() >= side) {
        return moved;
    }
    let lo = [shift[0].max(0), shift[1].max(0), shift[2].max(0)];
    let hi = [
        (shift[0] + side).min(side),
        (shift[1] + side).min(side),
        (shift[2] + side).min(side),
    ];
    if lo.iter().zip(hi.iter()).any(|(low, high)| low >= high) {
        return moved;
    }
    let run = (hi[0] - lo[0]) as usize * stride;
    for y in lo[1]..hi[1] {
        for z in lo[2]..hi[2] {
            let from = (((y - shift[1]) * side + (z - shift[2])) * side + (lo[0] - shift[0])) as usize
                * stride;
            let to = ((y * side + z) * side + lo[0]) as usize * stride;
            moved[to..to + run].copy_from_slice(&words[from..from + run]);
        }
    }
    moved
}

/// Add one leaf's solid volume to the coarse cells its own region covers, in solid voxels.
///
/// A leaf at level ≤ 3 sits inside one 8 m cell, because it is aligned to its own size; one above
/// that spans several, and each takes its share of the volume. That is the coarse band read from
/// the tree's own structure rather than from its voxels, which is why a 256 m band costs less to
/// stamp than the 96 m one.
fn add_solid_volume(solids: &mut [[[u32; 4]; 4]; 4], local: [i32; 3], side: i32, solid: u32) {
    let cell = HORIZON_FAR_CELL_M as i32;
    let lo = [local[0] / cell, local[1] / cell, local[2] / cell];
    let hi = [
        (local[0] + side - 1) / cell,
        (local[1] + side - 1) / cell,
        (local[2] + side - 1) / cell,
    ];
    let count = ((hi[0] - lo[0] + 1) * (hi[1] - lo[1] + 1) * (hi[2] - lo[2] + 1)) as u32;
    let each = solid / count.max(1);
    for z in lo[2]..=hi[2] {
        for y in lo[1]..=hi[1] {
            for x in lo[0]..=hi[0] {
                if let Some(row) = solids.get_mut(z as usize) {
                    if let Some(plane) = row.get_mut(y as usize) {
                        if let Some(cell) = plane.get_mut(x as usize) {
                            *cell += each;
                        }
                    }
                }
            }
        }
    }
}

/// **One voxel's 4×4×4 occupancy as two words**, in the model's own flat order
/// (`x + 4y + 16z`), low word first.
///
/// A shape with every cell solid is `[u32::MAX, u32::MAX]` and one with no matter at all is
/// `[0, 0]`, which is why a caller can test the two cheap cases before looking at a bit.
fn subvoxel_words(cells: &[voxel_model::MaterialId; 64]) -> [u32; 2] {
    let mut words = [0u32; 2];
    for (ordinal, material) in cells.iter().enumerate() {
        if !material.is_empty() {
            words[ordinal / 32] |= 1 << (ordinal % 32);
        }
    }
    words
}

/// The chunk a world point is in, through the one address law the field already uses.
fn chunk_of(point_m: [f64; 3]) -> ChunkPos {
    let side = CHUNK_SIDE as f64;
    ChunkPos::new(
        (point_m[0] / side).floor() as i32,
        (point_m[1] / side).floor() as i32,
        (point_m[2] / side).floor() as i32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_mesh::octree::SparseVoxelOctree;
    use voxel_model::{MaterialId, SubvoxelIndex};

    fn device() -> (wgpu::Device, wgpu::Queue) {
        let (device, queue, _name) = crate::renderer::offscreen_device().expect("a GPU for the volume");
        (device, queue)
    }

    fn tree_with_roof() -> SparseVoxelOctree {
        let mut tree = SparseVoxelOctree::new();
        // A 4 x 4 m roof, exactly one voxel thick and three metres above the chunk's floor.
        // `set_material_region` aligns its origin to its own level, so the roof is laid a
        // voxel at a time: a region at level 3 starting at y = 3 would snap to y = 0 and
        // become a floor rather than a roof.
        for x in 2..6 {
            for z in 2..6 {
                tree.set_voxel(
                    voxel_model::LocalPos::new(x, 3, z).expect("inside the chunk"),
                    voxel_model::VoxelShape::solid(MaterialId(1)),
                );
            }
        }
        tree
    }

    #[test]
    fn a_chunks_octree_stamps_its_own_metres_and_the_shadow_is_a_cell() {
        let (device, _queue) = device();
        let mut stream = HorizonStream::new(&device);
        stream.recenter([1.0, 1.0, 1.0]);
        assert_eq!(stream.anchor(), [-32, -32, -32]);
        assert!(stream.wants(ChunkPos::new(0, 0, 0), 1));
        let marked = stream.stamp(ChunkPos::new(0, 0, 0), &tree_with_roof(), 1);
        assert_eq!(marked, 16, "the roof is 4 x 4 x 1 m");
        assert_eq!(stream.solid_at([4.0, 3.4, 4.0]), Some(true));
        assert_eq!(stream.solid_at([4.0, 2.4, 4.0]), Some(false));
        assert_eq!(stream.solid_at([-100.0, 0.0, 0.0]), None, "outside is not air");
        assert!(!stream.wants(ChunkPos::new(0, 0, 0), 1), "a version is stamped once");
        assert!(stream.wants(ChunkPos::new(0, 0, 0), 2), "a new version of the tree wants stamping");
        // **The claim the shader reads**: the eye's own chunk is slot `1 + 3·(1 + 3·1)`, and a
        // second chunk sets its own bit rather than speaking for the box.
        assert_eq!(stream.coverage(), 1 << 13, "the stamped slot is not the mask's own bit");
        stream.stamp(ChunkPos::new(1, 0, 0), &tree_with_roof(), 1);
        assert_eq!(stream.coverage(), (1 << 13) | (1 << 14));
        assert!(
            !stream.wants(ChunkPos::new(2, 0, 0), 1) && stream.coverage() & (1 << 15) == 0,
            "a chunk outside the cube claimed a slot"
        );
    }

    /// **The stamp keeps the octree's own 4×4×4 grid, not one bit for the metre.**
    ///
    /// A voxel with one subvoxel column solid is a real thing the player makes by digging: three
    /// quarters of the block are gone and the corner that is left casts the corner's shadow. A
    /// volume that marked the whole metre would shade the dug quarters too, which is exactly the
    /// defect PATCH-070 fixed in the CPU walk and this volume must not reintroduce on the GPU.
    #[test]
    fn a_partly_dug_voxel_carries_its_sixty_four_bits_and_not_one() {
        let (device, _queue) = device();
        let mut stream = HorizonStream::new(&device);
        stream.recenter([1.0, 1.0, 1.0]);
        let mut tree = SparseVoxelOctree::new();
        let voxel = voxel_model::LocalPos::new(5, 2, 5).expect("inside the chunk");
        // One subvoxel column: x = 0, z = 0, all four y. Ordinals 0, 4, 8, 12.
        for y in 0..4u8 {
            tree.set_subvoxel(
                voxel,
                SubvoxelIndex::new(0, y, 0).expect("a subvoxel"),
                MaterialId(7),
            );
        }
        assert_eq!(stream.stamp(ChunkPos::new(0, 0, 0), &tree, 1), 1, "one voxel, one cell");

        assert_eq!(stream.solid_at([5.5, 2.5, 5.5]), Some(true), "the cell is not empty");
        assert_eq!(
            stream.subvoxel_at([5.125, 2.5, 5.125]),
            Some(true),
            "the solid column is not solid: the mask dropped the subvoxel grid"
        );
        for dug in [[5.375, 2.5, 5.125], [5.125, 2.5, 5.375], [5.875, 2.875, 5.875]] {
            assert_eq!(
                stream.subvoxel_at(dug),
                Some(false),
                "a dug subvoxel at {dug:?} is marked solid"
            );
        }
    }

    /// **A crossing carries the cube's knowledge with it**, which is the difference between a
    /// stepped window and a rebuilding one.
    ///
    /// The player walks from chunk 0 into chunk 1: the 96 m box slides one chunk, two thirds of the
    /// world in it is the same world, and the cells stamped from those chunks are the same cells at
    /// new addresses. Clearing and restamping all twenty-seven was a stall at every boundary and a
    /// light that arrived a chunk at a time — the two things the running window reported.
    #[test]
    fn a_crossing_shifts_the_cube_and_keeps_what_it_already_knows() {
        let (device, _queue) = device();
        let mut stream = HorizonStream::new(&device);
        assert!(stream.recenter([1.0, 1.0, 1.0]));
        stream.stamp(ChunkPos::new(0, 0, 0), &tree_with_roof(), 1);
        assert_eq!(stream.solid_at([4.0, 3.4, 4.0]), Some(true), "the roof was not stamped");
        assert!(!stream.recenter([1.0, 1.0, 1.0]), "the same chunk does not move it");

        assert!(stream.recenter([40.0, 1.0, 1.0]), "the next chunk does");
        assert_eq!(
            stream.solid_at([4.0, 3.4, 4.0]),
            Some(true),
            "chunk 0 is still in the box, so its stamp is still the truth about it"
        );
        assert!(!stream.wants(ChunkPos::new(0, 0, 0), 1), "a carried chunk is not walked again");
        assert!(stream.wants(ChunkPos::new(2, 0, 0), 1), "the chunk that entered wants a stamp");
        assert!(!stream.wants(ChunkPos::new(3, 0, 0), 1), "a chunk past the 96 m box is not in it");

        // Two chunks along, the chunk that stamped the roof is out of the box and must be forgotten.
        assert!(stream.recenter([80.0, 1.0, 1.0]));
        assert_eq!(stream.solid_at([4.0, 3.4, 4.0]), None, "a cell outside the cube has no answer");
        assert!(!stream.stamped.contains_key(&(0, 0, 0)), "the stamp outlived its chunk's slot");
    }

    /// **An edit is a new version, and the replaced chunk's cells follow it.**
    ///
    /// The block a stamp was taken from is a fact about one version of one tree. A dig that
    /// removed it does not move the cell the old stamp marked; the chunk has to be stamped again,
    /// or the hole the player just made keeps the shadow of the block that was in it. The second
    /// half is the other side of the same rule: a version already stamped is not walked again.
    #[test]
    fn a_new_version_of_a_tree_restamps_the_cells_the_old_one_marked() {
        let (device, _queue) = device();
        let mut stream = HorizonStream::new(&device);
        stream.recenter([1.0, 1.0, 1.0]);
        stream.stamp(ChunkPos::new(0, 0, 0), &tree_with_roof(), 6);
        assert_eq!(stream.solid_at([4.0, 3.4, 4.0]), Some(true), "the roof was not stamped");
        assert_eq!(stream.stamp(ChunkPos::new(0, 0, 0), &tree_with_roof(), 6), 0, "a held version is not walked again");

        // The player breaks the whole roof: the rebuilt chunk arrives as version 7.
        let empty = SparseVoxelOctree::new();
        assert!(stream.wants(ChunkPos::new(0, 0, 0), 7), "the edit must want stamping");
        assert_eq!(stream.stamp(ChunkPos::new(0, 0, 0), &empty, 7), 0, "nothing is left to mark");
        assert_eq!(
            stream.solid_at([4.0, 3.4, 4.0]),
            Some(false),
            "the roof of the tree before the edit is still standing"
        );
    }

    #[test]
    fn the_shaders_reach_and_the_hosts_are_one_number() {
        let source = include_str!("horizon.wgsl");
        for (what, literal) in [
            ("the march reach", "const HORIZON_REACH_M: f32 = 96.0;"),
            ("the dome reach", "const HORIZON_SKY_REACH_M: f32 = 32.0;"),
            ("the cell size", "const HORIZON_CELL_M: f32 = 1.0;"),
            ("the subvoxel side", "const HORIZON_SUBVOXEL_SIDE: u32 = 4u;"),
            ("the coarse reach", "const HORIZON_FAR_REACH_M: f32 = 160.0;"),
            ("the coarse share", "const FAR_SHARE: u32 = 0xFFu;"),
            ("the coarse stamped bit", "const FAR_MEASURED: u32 = 256u;"),
        ] {
            assert!(source.contains(literal), "the shader's {what} moved without this module's");
        }
        assert_eq!(HORIZON_CELLS as f32 * HORIZON_CELL_M, 96.0);
        assert_eq!(HORIZON_FAR_CELLS as f32 * HORIZON_FAR_CELL_M, 256.0);
        assert_eq!(HORIZON_SUBVOXEL_M, 0.25);
        assert_eq!(SKY_RAYS.len(), 3);
    }

    /// **The coarse band is stamped from the tree's own structure, and it is what makes a long
    /// corridor measurable.**
    ///
    /// The fine band answers to 96 m and the coarse one to 256 m; a wall 60 m away is a fine-band
    /// answer to a ray that starts near it, and a coarse-band one to a ray that is looking down a
    /// tunnel from the eye. Both are stamped from the same leaves — the coarse cell is matter if
    /// any voxel in its eight metres is — and an unstamped coarse cell is `None`, not air.
    #[test]
    fn the_coarse_band_carries_each_cells_share_and_says_where_it_has_not_looked() {
        let (device, _queue) = device();
        let mut stream = HorizonStream::new(&device);
        stream.recenter([1.0, 1.0, 1.0]);
        assert_eq!(stream.far_share_at([4.0, 3.5, 4.0]), None, "nothing is stamped yet");
        stream.stamp(ChunkPos::new(0, 0, 0), &tree_with_roof(), 1);
        // **A share, not a stone**: the roof is sixteen of this cell's 512 voxels.
        let share = stream.far_share_at([4.0, 3.5, 4.0]).expect("the cell is stamped");
        assert!(
            (share - 16.0 / 512.0).abs() < 1.0 / 255.0,
            "a one-voxel roof is {share} of its eight metres, not all of them"
        );
        assert_eq!(stream.far_solid_at([4.0, 3.5, 4.0]), Some(false), "and it is not a wall");
        assert_eq!(
            stream.far_share_at([4.0, 20.0, 4.0]),
            Some(0.0),
            "a measured empty cell reads as air, not as unmeasured"
        );
        assert_eq!(
            stream.far_share_at([200.0, 3.5, 4.0]),
            None,
            "a cell no chunk has stamped claimed to be measured"
        );

        // The other end: a whole 8 m cell of matter is a share of one — one cell, not the chunk.
        let mut full = SparseVoxelOctree::new();
        full.set_material_region([0, 0, 0], 3, MaterialId(1));
        assert_eq!(stream.stamp(ChunkPos::new(0, 0, 0), &full, 2), 512, "eight metres, not one");
        assert_eq!(stream.far_share_at([4.0, 3.5, 4.0]), Some(1.0), "a full cell is a full share");
        assert_eq!(stream.far_solid_at([4.0, 3.5, 4.0]), Some(true));
        assert_eq!(
            stream.far_share_at([12.0, 3.5, 4.0]),
            Some(0.0),
            "the neighbouring cell is its own share and holds nothing"
        );
    }

    /// **A chunk that leaves the window leaves no occluder behind.**
    #[test]
    fn a_chunk_that_is_no_longer_resident_is_forgotten() {
        let (device, _queue) = device();
        let mut stream = HorizonStream::new(&device);
        stream.recenter([1.0, 1.0, 1.0]);
        stream.stamp(ChunkPos::new(0, 0, 0), &tree_with_roof(), 1);
        assert_eq!(stream.solid_at([4.0, 3.4, 4.0]), Some(true));
        let mut resident = BTreeSet::new();
        resident.insert((0, 0, 0));
        assert!(!stream.retain(&resident), "a chunk still resident is not forgotten");
        let empty = BTreeSet::new();
        assert!(stream.retain(&empty), "an evicted chunk must be forgotten");
        assert_eq!(
            stream.solid_at([4.0, 3.4, 4.0]),
            Some(false),
            "the evicted chunk's roof is still standing in the fine band"
        );
        assert_eq!(stream.far_solid_at([4.0, 3.5, 4.0]), None, "the coarse band restreamed");
    }
}
