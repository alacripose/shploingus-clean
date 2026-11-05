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
//!    //! The box is three chunks square — [`HORIZON_CELLS`] × [`HORIZON_CELL_M`] = 96 m — with the eye's
//! chunk in the middle, so it follows the eye one chunk at a time and covers a chunk behind the
//! camera as well as the chunk ahead of it. **Past its edge the coarse band answers**, 8 m cells
//! over [`HORIZON_FAR_CELLS`] × [`HORIZON_FAR_CELL_M`] = 512 m, so a chunk the shadow map still
//! reaches is lit by a measurement at its own rung rather than falling back to the unshadowed sky.
//! A cell that is **unstamped** in either band is no measurement at all, and the shader says so
//! rather than treating it as clear air: outside both bands the frame's answer is the shadow
//! map's, which is the light the frame had before this field existed and a different claim from
//! "this cell sees the whole sky".
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
//! **And a stamp belongs to a rung.** The version is one half of what a walk covers; the other is
//! *which bands the walk wrote*. A chunk first seen eight chunks out is stamped with none of its
//! metres in the cube — the coarse band gets its 8 m summary and the fine loop is skipped — so a
//! chunk can be current at a version and have never been walked at 1 m. A record that carried the
//! version alone said "stamped" for a band the walk had not reached, and the chunk the player then
//! walked into read as measured, empty air rather than as unmeasured. The same is true in the other
//! direction: a crossing shifts the cells and cuts the ones that fall out of the volume, so the
//! record loses those bands at the crossing and the chunk is walked again on the way back in.
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
pub const HORIZON_CELL_M: f32 = crate::lod::VOXEL_CELL_M;

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
pub const HORIZON_FAR_CELL_M: f32 = crate::lod::FAR_CELL_M;

/// Coarse cells along one edge of the far band: 64 × [`HORIZON_FAR_CELL_M`] = 512 m.
///
/// **As wide as the shadow's far cascade is deep** (256 m each way), because this is the light's
/// own coarse rung and the two answer for the same distance: a drawn chunk the shadow map still
/// reaches must be one the coarse band has looked at, or its ambient has no measurement at exactly
/// the range the geometry is still real.
pub const HORIZON_FAR_CELLS: u32 = 64;

/// **How far a far-band ray walks**, metres — a quarter of the band's diagonal, so a ray that
/// starts at the eye has the whole declared horizon ahead of it and not just half of one side.
pub const HORIZON_FAR_REACH_M: f32 = 512.0;

/// Chunks along one edge of the far band: 512 m / 32 m, half of them each side of the eye's chunk.
pub const HORIZON_FAR_SLOTS: i32 = 16;

/// **The wide band's cell** — sixteen metres, tier 6's grain ([`crate::lod::WIDE_CELL_M`]).
///
/// The second terrain rung is drawn at this cell, so its light is measured at it: a 16 m face lit
/// by an 8 m march would be asking a question finer than the geometry can answer.
pub const HORIZON_WIDE_CELL_M: f32 = crate::lod::WIDE_CELL_M;

/// Wide cells along one edge: 64 × 16 m = 1024 m.
pub const HORIZON_WIDE_CELLS: u32 = 64;

/// **How far a wide-band ray walks**: the band's whole box, so the rung past 320 m still has an
/// answer and a mountain at 900 m is not lit by an unmeasured sky.
pub const HORIZON_WIDE_REACH_M: f32 = 1024.0;

/// Chunks along one edge of the wide band: 1024 m / 32 m, half each side of the eye's chunk.
pub const HORIZON_WIDE_SLOTS: i32 = 32;

/// **The chunk tier's band**: one 32 m cell per chunk, 64³ over 2048 m.
pub const HORIZON_CHUNK_CELL_M: f32 = crate::lod::CHUNK_CELL_M;
pub const HORIZON_CHUNK_CELLS: u32 = 64;
pub const HORIZON_CHUNK_REACH_M: f32 = 2048.0;
pub const HORIZON_CHUNK_SLOTS: i32 = 64;

/// **The surface rungs' bands**: one 33³ grid a rung, in the ladder's one coarse buffer.
///
/// A rung above a chunk is measured one sample a cell by [`crate::surface`], and its band is that
/// **same sample** — the share of the cell it stands in, the cells below it full and the cells
/// above it measured air — so the words are the same [`FAR_SHARE`]/[`FAR_MEASURED`] the coarse
/// rungs carry and the shader walks them with the same `world_band_share`. A rung is an offset
/// into the buffer, not a binding and not a second walk.
pub const SURFACE_BAND_CELLS: u32 = crate::surface::RING_CELLS as u32;

/// The words one rung's band takes: `33³`.
pub const SURFACE_BAND_WORDS: usize =
    (SURFACE_BAND_CELLS * SURFACE_BAND_CELLS * SURFACE_BAND_CELLS) as usize;

/// **The rung slots the light's table has**, one per surface tier ([`crate::lod`]): a rung above
/// the drawn range would be an append into a slot that is already there.
pub const SURFACE_SLOTS: usize = crate::lod::SURFACE_RUNG_SLOTS;

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

/// **Which of the two bands a stamp actually wrote**, as the record's own bits.
///
/// The version alone is not the fact a caller needs. A chunk first seen eight chunks out is
/// stamped with none of its metres in the cube, so [`HorizonStream::stamp`] skips the fine loop and
/// writes only its 8 m summary; two crossings later that same chunk is the one the player is
/// standing in, at the *same* version of the *same* tree*. A record that remembered only the
/// version answered "this is stamped" for a band that had never been walked, and the chunk under
/// the player then read as **measured air** — `Some(false)`, a claim, where the truth was
/// `None`, no measurement at all.
const BAND_NEAR: u8 = 1;
const BAND_FAR: u8 = 1 << 1;
const BAND_WIDE: u8 = 1 << 2;
const BAND_CHUNK: u8 = 1 << 3;

/// **What one chunk's stamp covers**: the version of the tree it was walked from, and the bands
/// whose cells that walk actually wrote at that version.
///
/// Both halves are needed and neither implies the other. The version changes when the tree does;
/// the bands change when the *eye* does, because a crossing shifts the cells and whatever the shift
/// cut out of the volume is no longer stamped in it. See [`HorizonStream::recenter`].
#[derive(Clone, Copy, Debug)]
struct Stamp {
    version: u64,
    bands: u8,
}

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

/// **One surface rung's band, as the light reads it**: the grid's own anchor, the rung's cell, and
/// the words.
///
/// `anchor_cell` is the world cell the band's `(0, 0, 0)` stands in, at the rung's own grain, and
/// `words` is `33³` in `x + 33·(y + 33·z)` order — the same order the coarse bands use, so the
/// march's own index law is untouched. `live` says a rung has been stamped at this anchor; the
/// slot exists from the start, so a rung appears in the table by being stamped rather than by the
/// table growing.
#[derive(Clone, Debug)]
pub struct SurfaceBand {
    pub tier: u32,
    pub cell_m: f32,
    pub anchor_cell: [i32; 3],
    pub words: Vec<u32>,
    live: bool,
    dirty: bool,
}

impl SurfaceBand {
    fn empty(tier: u32) -> Self {
        let cell_m = crate::lod::cell_m(tier) as f32;
        Self {
            tier,
            cell_m,
            anchor_cell: [0, 0, 0],
            words: vec![0u32; SURFACE_BAND_WORDS],
            live: false,
            dirty: false,
        }
    }

    /// **The rung's own reach**: 16 cells, the outer edge of the annulus it draws.
    pub fn reach_m(&self) -> f32 {
        crate::lod::SURFACE_OUTER_CELLS as f32 * self.cell_m
    }

    /// Whether a rung has been measured into this slot.
    pub fn live(&self) -> bool {
        self.live
    }
}

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
/// **The rung table's own stride**: `to_anchor` and `params` — four `f32` each, the WGSL struct's
/// own 32 bytes. `params` is `(cell_m, cells, reach_m, word offset)`.
const SURFACE_ENTRY_FLOATS: usize = 8;

/// The uniform's bytes: the ten `vec4` of the bands and the mask, then one entry a rung slot.
const UNIFORM_BYTES: u64 = (40 + SURFACE_ENTRY_FLOATS * SURFACE_SLOTS) as u64 * 4;

/// **The streamed occupancy volume, and the two GPU handles the world pass reads it through.**
///
/// **Two bands are two rungs of a declared thirty-two** (`docs/lod-ladder.md`): one address step per
/// doubling, from the 0.25 m subvoxel through the 8 m voxel and the 256 m chunk up to the top of a
/// 1/8-Earth address space. The rungs above 8 m do not exist yet, and the doc states what each one
/// owes: a band's own summaries aggregated again, the terrain meshed at the same grain, and the
/// per-cell measurement the cadence tick rebuilds so a fragment samples one value rather than
/// marching a hundred cells.
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
    /// **The wide band**: one [`FAR_SHARE`]/[`FAR_MEASURED`] word per 16 m cell, 64³ = 1024 m.
    ///
    /// Tier 6's terrain is cut at sixteen metres and drawn beyond the far ring; this is the band
    /// that measured it. Same word, same walk, twice the cell — a rung, not a mechanism.
    wide: Vec<u32>,
    /// **The chunk-tier band**: one 32 m cell per chunk, 64³ = 2048 m. The last chunk-held rung.
    chunk: Vec<u32>,
    /// **The surface rungs' bands**, one slot per surface tier, in the same coarse buffer after
    /// the three above. A slot is stamped by [`HorizonStream::stamp_surface`] with the very
    /// samples the rung's mesh was cut from.
    surfaces: Vec<SurfaceBand>,
    /// The world-voxel corner of cell `(0, 0, 0)`.
    anchor: [i32; 3],
    /// The world-voxel corner of far cell `(0, 0, 0)`.
    far_anchor: [i32; 3],
    /// The world-voxel corner of wide cell `(0, 0, 0)`.
    wide_anchor: [i32; 3],
    /// The world-voxel corner of chunk-tier cell `(0, 0, 0)`.
    chunk_anchor: [i32; 3],
    /// **Chunks stamped into this volume, and what each stamp covers** — the version of the tree
    /// it was walked from and the bands it wrote — so a chunk is walked once per version *and per
    /// rung*, and an edit or a crossing is walked again.
    stamped: BTreeMap<(i32, i32, i32), Stamp>,
    /// Whether the CPU cube has moved since the last upload.
    dirty: bool,
    buffer: wgpu::Buffer,
    /// **Every coarse rung, concatenated**: far, then wide, then the chunk tier. One binding for
    /// the ladder rather than one per rung — see [`HorizonStream::bind_entries`].
    coarse_buffer: wgpu::Buffer,
    uniform: wgpu::Buffer,
}

impl HorizonStream {
    /// The cube, empty and anchored at the origin until a frame recentres it.
    pub fn new(device: &wgpu::Device) -> Self {
        let cells = (HORIZON_CELLS * HORIZON_CELLS * HORIZON_CELLS) as usize;
        let bytes = (cells * HORIZON_WORDS_PER_CELL * 4) as u64;
        let far_cells = (HORIZON_FAR_CELLS * HORIZON_FAR_CELLS * HORIZON_FAR_CELLS) as usize;
        let wide_cells = (HORIZON_WIDE_CELLS * HORIZON_WIDE_CELLS * HORIZON_WIDE_CELLS) as usize;
        let chunk_cells =
            (HORIZON_CHUNK_CELLS * HORIZON_CHUNK_CELLS * HORIZON_CHUNK_CELLS) as usize;
        HorizonStream {
            cells: vec![0; cells * HORIZON_WORDS_PER_CELL],
            far: vec![0; far_cells],
            wide: vec![0; wide_cells],
            chunk: vec![0; chunk_cells],
            // **Every surface tier has a slot**, drawn or not: the ladder's own table is
            // `TIERS - 8` wide so a rung above the drawn range is a data change rather than a
            // buffer resize.
            surfaces: (crate::lod::SURFACE_FIRST_TIER..crate::lod::TIERS)
                .map(SurfaceBand::empty)
                .collect(),
            anchor: [0, 0, 0],
            far_anchor: [0, 0, 0],
            wide_anchor: [0, 0, 0],
            chunk_anchor: [0, 0, 0],
            stamped: BTreeMap::new(),
            dirty: true,
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("light-horizon"),
                size: bytes,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            coarse_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("light-horizon-coarse-bands"),
                size: ((far_cells + wide_cells + chunk_cells + SURFACE_SLOTS * SURFACE_BAND_WORDS)
                    * 4) as u64,
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

    /// **The entries the world pass adds for the horizon** — the fine cells at binding 10, the view
    /// at 11, and one storage buffer a coarse rung, after the shadow's 6–9.
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
    ///
    /// **One storage buffer for every coarse rung**: the bands are concatenated and each rung's
    /// offset rides its own `params.w`, so adding a rung appends words instead of spending a
    /// binding. WebGPU gives a fragment stage eight storage buffers; five were already gone to
    /// four bands and the fine one, which is a ceiling the ladder cannot grow under.
    pub(crate) fn bind_entries(&self) -> [wgpu::BindGroupEntry<'_>; 3] {
        [
            wgpu::BindGroupEntry {
                binding: 10,
                resource: self.buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 11,
                resource: self.uniform.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 12,
                resource: self.coarse_buffer.as_entire_binding(),
            },
        ]
    }

    /// The world-voxel corner the cube is anchored on.
    pub fn anchor(&self) -> [i32; 3] {
        self.anchor
    }

    /// **Which of the cube's 3 × 3 × 3 chunk slots have been walked at the fine rung**, as
    /// `sx + 3·(sy + 3·sz)` bits — the mask the shader reads to tell an empty cell from an
    /// unmeasured one.
    ///
    /// **A record is not a walk.** A chunk that was stamped while it was outside the cube has a
    /// record whose only band is the coarse one: its 8 m summary exists and the fine loop never ran.
    /// Counting that record here set the bit anyway, and the frame then read a slot full of zeros as
    /// *measured air* — the chunk beside the player, or the chunk in front of them, answering "the
    /// whole sky is visible from every cell in me" instead of "nobody has looked at me at 1 m yet".
    /// The shadow map answers the second; the first is an invented measurement. So the bit means
    /// what `horizon_stamped` says it means: this slot's metres have been looked at.
    pub fn coverage(&self) -> u32 {
        let mut mask = 0u32;
        for ((x, y, z), stamp) in &self.stamped {
            if stamp.bands & BAND_NEAR == 0 {
                continue;
            }
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

    /// Whether a chunk's 32 m box touches the **fine** cube — the only box whose unstamped cells
    /// a frame may not draw through.
    pub fn covers(&self, pos: ChunkPos) -> bool {
        let origin = pos.origin();
        in_box(
            [origin.x, origin.y, origin.z],
            self.anchor,
            HORIZON_CELLS as i32,
        )
    }

    /// **Whether one chunk's slot in the fine cube has been light-sampled** — the same bit the
    /// shader reads through [`Self::coverage`], asked about a chunk instead of packed into a word.
    ///
    /// `false` is three facts and a caller may treat them as one: the chunk is **outside** the
    /// cube (the coarse band is its answer, not a refusal), the chunk is inside and has not been
    /// walked yet, or a crossing cut its metres out of the volume. The last two are the ones a
    /// frame may not draw through, because drawing them means drawing a chunk whose every ray
    /// answered "unmeasured" — the shadow map's light with none of the horizon's shadowing, which
    /// reads as a chunk that is lit by a sun it cannot see.
    pub fn lit(&self, pos: ChunkPos) -> bool {
        let origin = pos.origin();
        let slot = [
            (origin.x - self.anchor[0]).div_euclid(CHUNK_SIDE),
            (origin.y - self.anchor[1]).div_euclid(CHUNK_SIDE),
            (origin.z - self.anchor[2]).div_euclid(CHUNK_SIDE),
        ];
        if slot.iter().any(|v| *v < 0 || *v >= 3) {
            return false;
        }
        self.coverage() & (1 << (slot[0] + 3 * (slot[1] + 3 * slot[2]))) != 0
    }

    /// Whether a chunk's 32 m box touches the **coarse** band.
    pub fn covers_far(&self, pos: ChunkPos) -> bool {
        let origin = pos.origin();
        let span = (HORIZON_FAR_CELLS as f32 * HORIZON_FAR_CELL_M) as i32;
        in_box([origin.x, origin.y, origin.z], self.far_anchor, span)
    }

    /// **The bands a chunk touches right now** — the only ones a stamp of it could write.
    fn bands(&self, pos: ChunkPos) -> u8 {
        let mut bands = 0;
        if self.covers(pos) {
            bands |= BAND_NEAR;
        }
        if self.covers_far(pos) {
            bands |= BAND_FAR;
        }
        bands
    }

    /// **Whether this chunk's volume still needs this version of its tree, in a band it has not
    /// been walked in.**
    ///
    /// Two questions, and the old one asked only the first. The **version** answers "is this the
    /// tree I stamped?" — an edit bumps it and the chunk wants a walk. The **bands** answer "did
    /// that walk reach the rung this chunk is at now?" — a chunk is stamped into the coarse band
    /// alone while it is outside the cube, and into the fine band alone is impossible (a fine stamp
    /// writes both), but the reverse happens on every crossing: leaving the cube cuts a chunk's
    /// metres out of the volume, so the record has to forget the fine band with them.
    ///
    /// So a chunk wants a walk when it touches either band and touches a band its stamp does not
    /// cover at that version. On an ordinary frame of a still world it wants nothing.
    pub fn wants(&self, pos: ChunkPos, version: u64) -> bool {
        let bands = self.bands(pos);
        if bands == 0 {
            return false;
        }
        match self.stamped.get(&(pos.x, pos.y, pos.z)) {
            Some(stamp) => stamp.version != version || stamp.bands & bands != bands,
            None => true,
        }
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
        // **The far band is centred on the eye's own chunk too**, eight chunks out on each axis: a
        // 512 m box with the player in the middle, so a tunnel mouth and the skyline are both in
        // it and nothing the eye can walk toward arrives half a box late.
        let far_anchor = [
            (chunk.x - HORIZON_FAR_SLOTS / 2) * CHUNK_SIDE,
            (chunk.y - HORIZON_FAR_SLOTS / 2) * CHUNK_SIDE,
            (chunk.z - HORIZON_FAR_SLOTS / 2) * CHUNK_SIDE,
        ];
        // **The wide band is centred on the same chunk**, thirty-two chunks each way: 1 km of world
        // in the eye's frame, which is the rung past the far ring's 320 m edge.
        let wide_anchor = [
            (chunk.x - HORIZON_WIDE_SLOTS / 2) * CHUNK_SIDE,
            (chunk.y - HORIZON_WIDE_SLOTS / 2) * CHUNK_SIDE,
            (chunk.z - HORIZON_WIDE_SLOTS / 2) * CHUNK_SIDE,
        ];
        // **The chunk tier is centred on the same chunk too**, sixty-four chunks each way: 2 km of
        // world at one cell a chunk. It is the reach the rung above the 1 km band needs.
        let chunk_anchor = [
            (chunk.x - HORIZON_CHUNK_SLOTS / 2) * CHUNK_SIDE,
            (chunk.y - HORIZON_CHUNK_SLOTS / 2) * CHUNK_SIDE,
            (chunk.z - HORIZON_CHUNK_SLOTS / 2) * CHUNK_SIDE,
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
        let wide_shift = [
            (self.wide_anchor[0] - wide_anchor[0]) / HORIZON_WIDE_CELL_M as i32,
            (self.wide_anchor[1] - wide_anchor[1]) / HORIZON_WIDE_CELL_M as i32,
            (self.wide_anchor[2] - wide_anchor[2]) / HORIZON_WIDE_CELL_M as i32,
        ];
        let chunk_shift = [
            (self.chunk_anchor[0] - chunk_anchor[0]) / HORIZON_CHUNK_CELL_M as i32,
            (self.chunk_anchor[1] - chunk_anchor[1]) / HORIZON_CHUNK_CELL_M as i32,
            (self.chunk_anchor[2] - chunk_anchor[2]) / HORIZON_CHUNK_CELL_M as i32,
        ];
        self.cells = shifted(
            &self.cells,
            HORIZON_CELLS as i32,
            HORIZON_WORDS_PER_CELL,
            cell_shift,
        );
        self.far = shifted(&self.far, HORIZON_FAR_CELLS as i32, 1, far_shift);
        self.wide = shifted(&self.wide, HORIZON_WIDE_CELLS as i32, 1, wide_shift);
        self.chunk = shifted(&self.chunk, HORIZON_CHUNK_CELLS as i32, 1, chunk_shift);
        self.anchor = anchor;
        self.far_anchor = far_anchor;
        self.wide_anchor = wide_anchor;
        self.chunk_anchor = chunk_anchor;
        // **A stamp survives while its chunk is still in either band — and only in the bands the
        // shift left it in.** The cells moved with the bands, so the version it was taken at is
        // still the version standing in them; a band whose cells the shift *cut* is not, so the
        // record loses that band here rather than claiming metres that are no longer in the volume.
        // This is the half that makes the far band cheap: the coarse summary of a chunk the fine
        // cube has left behind survives every crossing, and the fine band it lost does not.
        let far_span = (HORIZON_FAR_CELLS as f32 * HORIZON_FAR_CELL_M) as i32;
        let wide_span = (HORIZON_WIDE_CELLS as f32 * HORIZON_WIDE_CELL_M) as i32;
        let chunk_span = (HORIZON_CHUNK_CELLS as f32 * HORIZON_CHUNK_CELL_M) as i32;
        self.stamped.retain(|key, stamp| {
            let origin = ChunkPos::new(key.0, key.1, key.2).origin();
            let origin = [origin.x, origin.y, origin.z];
            let mut kept = 0;
            if in_box(origin, anchor, HORIZON_CELLS as i32) {
                kept |= BAND_NEAR;
            }
            if in_box(origin, far_anchor, far_span) {
                kept |= BAND_FAR;
            }
            if in_box(origin, wide_anchor, wide_span) {
                kept |= BAND_WIDE;
            }
            if in_box(origin, chunk_anchor, chunk_span) {
                kept |= BAND_CHUNK;
            }
            stamp.bands &= kept;
            kept != 0
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
            self.clear_wide_chunk(base);
            self.clear_chunk_band(base);
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
    /// **Both bands, always** — a chunk outside the 96 m cube has no fine cells to mark and still
    /// stamps its 8 m summary, which is the rung a far fragment is lit at. The fine loop is skipped
    /// there so a far chunk costs its leaves rather than its voxels.
    ///
    /// The chunk's own cells are **cleared first**, because a chunk replaces the one it was: an
    /// edit is served by a new tree for the same chunk, and a stamp that only ever added bits
    /// would keep the block the player broke. Chunks do not overlap, so this costs no neighbour's
    /// cells.
    ///
    /// Returns the number of cells the chunk marked (including cells already carrying a mask).
    pub fn stamp(&mut self, pos: ChunkPos, tree: &SparseVoxelOctree, version: u64) -> usize {
        // **The caller owns "does this need stamping".** `wants` is the cheap question and every
        // caller asks it; re-asking it here made a forced stamp a no-op, which is how a chunk whose
        // cells a crossing had emptied kept a version record claiming they were there.
        //
        // **The record says which bands this walk wrote**, computed before anything moves, because
        // the answer is exactly what the two loops below are allowed to write: the coarse loop
        // writes wherever the chunk is in the coarse band, the fine loop only inside the cube.
        let bands = self.bands(pos);
        self.stamped
            .insert((pos.x, pos.y, pos.z), Stamp { version, bands });
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
        // **The coarse band's own census**, in solid voxels out of the 512 an 8 m cell holds —
        // the same walk [`crate::far_terrain::census`] gives the far mesh, so the face a fragment
        // sees and the share the march reads cannot disagree about which cells are rock.
        let census = crate::far_terrain::census(tree);
        let in_near = self.covers(pos);
        tree.visit_leaves(&mut |local: [i32; 3], level: u32, node: &Node| {
            if !in_near {
                return;
            }
            match node {
                Node::Empty | Node::Branch(_) => {}
                Node::Uniform(_) => {
                    let side = 1i32 << level;
                    let words = [u32::MAX, u32::MAX];
                    for y in local[1]..local[1] + side {
                        for z in local[2]..local[2] + side {
                            for x in local[0]..local[0] + side {
                                if self.mark([base[0] + x, base[1] + y, base[2] + z], words) {
                                    marked += 1;
                                }
                            }
                        }
                    }
                }
                Node::Mixed(shape) => {
                    let words = subvoxel_words(shape.cells());
                    if words != [0, 0]
                        && self.mark(
                            [base[0] + local[0], base[1] + local[1], base[2] + local[2]],
                            words,
                        )
                    {
                        marked += 1;
                    }
                }
            }
        });
        self.write_far(pos, &census);
        self.dirty = true;
        marked
    }

    /// **Write one chunk's coarse band from a census** — the far field's own stamp.
    ///
    /// A far chunk is outside the fine cube by construction, so this writes the 4 × 4 × 4 coarse
    /// cells and records that the **coarse** band was walked; the fine band is left alone, and a
    /// chunk the cube covers needs [`HorizonStream::stamp`] for its metres. Every word comes from
    /// the same [`crate::far_terrain::census`] the far mesh's faces come from.
    ///
    /// Returns the number of coarse cells the chunk owns inside the band.
    pub fn stamp_far(
        &mut self,
        pos: ChunkPos,
        census: &crate::far_terrain::Census,
        version: u64,
    ) -> usize {
        // The band is one tier and the census says which tier it was measured at: a 16 m census
        // written into an 8 m band would put four times the world in every cell, so it is refused
        // rather than resampled into a grain the walk did not use.
        assert_eq!(
            census.tier(),
            crate::lod::FAR_TIER,
            "the coarse band is the far tier; a census at tier {} is a different band",
            census.tier()
        );
        let key = (pos.x, pos.y, pos.z);
        let bands = self.stamped.get(&key).map_or(0, |stamp| stamp.bands) | BAND_FAR;
        self.stamped.insert(key, Stamp { version, bands });
        let written = self.write_far(pos, census);
        self.dirty = true;
        written
    }

    /// **Write one chunk's wide band from a tier-6 census** — the second terrain rung's own stamp.
    ///
    /// The same law as [`HorizonStream::stamp_far`], one rung up: the census must be the wide tier
    /// (a 16 m census into a 16 m band), the chunk's `2 × 2 × 2` cells are written from the same
    /// shares the tier-6 mesh's faces come from, and the record says the **wide** band was walked.
    ///
    /// Returns the number of wide cells the chunk owns inside the band.
    pub fn stamp_wide(
        &mut self,
        pos: ChunkPos,
        census: &crate::far_terrain::Census,
        version: u64,
    ) -> usize {
        assert_eq!(
            census.tier(),
            crate::lod::WIDE_TIER,
            "the wide band is tier {}; a census at tier {} is a different band",
            crate::lod::WIDE_TIER,
            census.tier()
        );
        let key = (pos.x, pos.y, pos.z);
        let bands = self.stamped.get(&key).map_or(0, |stamp| stamp.bands) | BAND_WIDE;
        self.stamped.insert(key, Stamp { version, bands });
        let written = self.write_wide(pos, census);
        self.dirty = true;
        written
    }

    /// The wide band's own `side³` words for a chunk, from one census.
    fn write_wide(&mut self, pos: ChunkPos, census: &crate::far_terrain::Census) -> usize {
        let origin = pos.origin();
        let base = [origin.x, origin.y, origin.z];
        let mut written = 0usize;
        let side = census.side();
        let voxels = census.cell_voxels();
        for fy in 0..side {
            for fz in 0..side {
                for fx in 0..side {
                    let corner = [
                        base[0] + fx * HORIZON_WIDE_CELL_M as i32,
                        base[1] + fy * HORIZON_WIDE_CELL_M as i32,
                        base[2] + fz * HORIZON_WIDE_CELL_M as i32,
                    ];
                    let solid = census
                        .get([fx, fy, fz])
                        .map(|cell| cell.solids)
                        .unwrap_or(0);
                    let share = (solid * FAR_SHARE / voxels.max(1)).min(FAR_SHARE);
                    if let Some(index) = self.wide_index(corner) {
                        self.wide[index] = FAR_MEASURED | share;
                        written += 1;
                    }
                }
            }
        }
        written
    }

    /// The wide cell index of a world corner aligned to the wide grid, or `None` outside it.
    fn wide_index(&self, world: [i32; 3]) -> Option<usize> {
        let cell = HORIZON_WIDE_CELL_M as i32;
        let q = [
            (world[0] - self.wide_anchor[0]).div_euclid(cell),
            (world[1] - self.wide_anchor[1]).div_euclid(cell),
            (world[2] - self.wide_anchor[2]).div_euclid(cell),
        ];
        let n = HORIZON_WIDE_CELLS as i32;
        if q.iter().any(|v| *v < 0 || *v >= n) {
            return None;
        }
        Some((q[0] + n * (q[1] + n * q[2])) as usize)
    }

    /// **How much of the wide cell at a world point holds matter** — `Some(0.0)` for a stamped
    /// empty cell, `None` where it is unstamped or outside the band.
    pub fn wide_share_at(&self, world_m: [f64; 3]) -> Option<f32> {
        let cell = HORIZON_WIDE_CELL_M as f64;
        let corner = [
            ((world_m[0] / cell).floor() as i32) * HORIZON_WIDE_CELL_M as i32,
            ((world_m[1] / cell).floor() as i32) * HORIZON_WIDE_CELL_M as i32,
            ((world_m[2] / cell).floor() as i32) * HORIZON_WIDE_CELL_M as i32,
        ];
        let word = *self.wide.get(self.wide_index(corner)?)?;
        if word & FAR_MEASURED == 0 {
            return None;
        }
        Some((word & FAR_SHARE) as f32 / FAR_SHARE as f32)
    }

    /// The coarse band's own 4 × 4 × 4 words for a chunk, from one census.
    fn write_far(&mut self, pos: ChunkPos, census: &crate::far_terrain::Census) -> usize {
        let origin = pos.origin();
        let base = [origin.x, origin.y, origin.z];
        let mut written = 0usize;
        let side = census.side();
        let voxels = census.cell_voxels();
        for fy in 0..side {
            for fz in 0..side {
                for fx in 0..side {
                    let corner = [
                        base[0] + fx * HORIZON_FAR_CELL_M as i32,
                        base[1] + fy * HORIZON_FAR_CELL_M as i32,
                        base[2] + fz * HORIZON_FAR_CELL_M as i32,
                    ];
                    let solid = census
                        .get([fx, fy, fz])
                        .map(|cell| cell.solids)
                        .unwrap_or(0);
                    // The denominator is the cell's own, not the far tier's constant: a share is
                    // a fraction of the cell that was measured.
                    let share = (solid * FAR_SHARE / voxels.max(1)).min(FAR_SHARE);
                    if let Some(index) = self.far_index(corner) {
                        self.far[index] = FAR_MEASURED | share;
                        written += 1;
                    }
                }
            }
        }
        written
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

    /// **Write one chunk's chunk-tier band from a tier-7 census** — the last chunk-held rung.
    ///
    /// A 32 m cell is exactly one chunk, so this is one word per chunk for the band's own grain.
    /// The census must be tier 7; a 16 m or 8 m census is refused rather than resampled.
    pub fn stamp_chunk_band(
        &mut self,
        pos: ChunkPos,
        census: &crate::far_terrain::Census,
        version: u64,
    ) -> usize {
        assert_eq!(
            census.tier(),
            crate::lod::CHUNK_TIER,
            "the chunk band is tier {}; a census at tier {} is a different band",
            crate::lod::CHUNK_TIER,
            census.tier()
        );
        let key = (pos.x, pos.y, pos.z);
        let bands = self.stamped.get(&key).map_or(0, |stamp| stamp.bands) | BAND_CHUNK;
        self.stamped.insert(key, Stamp { version, bands });
        let written = self.write_chunk_band(pos, census);
        self.dirty = true;
        written
    }

    /// The chunk band's `side³` words for a chunk, from one census.
    fn write_chunk_band(&mut self, pos: ChunkPos, census: &crate::far_terrain::Census) -> usize {
        let origin = pos.origin();
        let base = [origin.x, origin.y, origin.z];
        let mut written = 0usize;
        let side = census.side();
        let voxels = census.cell_voxels();
        for fy in 0..side {
            for fz in 0..side {
                for fx in 0..side {
                    let corner = [
                        base[0] + fx * HORIZON_CHUNK_CELL_M as i32,
                        base[1] + fy * HORIZON_CHUNK_CELL_M as i32,
                        base[2] + fz * HORIZON_CHUNK_CELL_M as i32,
                    ];
                    let solid = census
                        .get([fx, fy, fz])
                        .map(|cell| cell.solids)
                        .unwrap_or(0);
                    let share = (solid * FAR_SHARE / voxels.max(1)).min(FAR_SHARE);
                    if let Some(index) = self.chunk_band_index(corner) {
                        self.chunk[index] = FAR_MEASURED | share;
                        written += 1;
                    }
                }
            }
        }
        written
    }

    /// The chunk band's cell index of a world corner aligned to its grid, or `None`.
    fn chunk_band_index(&self, world: [i32; 3]) -> Option<usize> {
        let cell = HORIZON_CHUNK_CELL_M as i32;
        let q = [
            (world[0] - self.chunk_anchor[0]).div_euclid(cell),
            (world[1] - self.chunk_anchor[1]).div_euclid(cell),
            (world[2] - self.chunk_anchor[2]).div_euclid(cell),
        ];
        let n = HORIZON_CHUNK_CELLS as i32;
        if q.iter().any(|v| *v < 0 || *v >= n) {
            return None;
        }
        Some((q[0] + n * (q[1] + n * q[2])) as usize)
    }

    /// The chunk band's share at a world point, or `None` outside or unstamped.
    pub fn chunk_share_at(&self, world_m: [f64; 3]) -> Option<f32> {
        let cell = HORIZON_CHUNK_CELL_M as f64;
        let corner = [
            ((world_m[0] / cell).floor() as i32) * HORIZON_CHUNK_CELL_M as i32,
            ((world_m[1] / cell).floor() as i32) * HORIZON_CHUNK_CELL_M as i32,
            ((world_m[2] / cell).floor() as i32) * HORIZON_CHUNK_CELL_M as i32,
        ];
        let word = *self.chunk.get(self.chunk_band_index(corner)?)?;
        if word & FAR_MEASURED == 0 {
            return None;
        }
        Some((word & FAR_SHARE) as f32 / FAR_SHARE as f32)
    }

    /// **Where a surface rung's words begin in the ladder's one coarse buffer.**
    ///
    /// The three chunk-held bands first, then one slot a surface tier: the table's own slot is
    /// the tier's, so a band's address is a function of the rung and not of the order rungs were
    /// measured in.
    pub fn surface_offset(&self, slot: usize) -> usize {
        let far_cells = (HORIZON_FAR_CELLS * HORIZON_FAR_CELLS * HORIZON_FAR_CELLS) as usize;
        let wide_cells = (HORIZON_WIDE_CELLS * HORIZON_WIDE_CELLS * HORIZON_WIDE_CELLS) as usize;
        let chunk_cells =
            (HORIZON_CHUNK_CELLS * HORIZON_CHUNK_CELLS * HORIZON_CHUNK_CELLS) as usize;
        far_cells + wide_cells + chunk_cells + slot * SURFACE_BAND_WORDS
    }

    /// The slot a tier's band lives in, or `None` for a tier below the surface ladder.
    fn surface_slot(tier: u32) -> Option<usize> {
        (crate::lod::SURFACE_FIRST_TIER..crate::lod::TIERS)
            .contains(&tier)
            .then(|| (tier - crate::lod::SURFACE_FIRST_TIER) as usize)
    }

    /// **Write one surface rung's band from the ring it was cut from** — the rung's own stamp.
    ///
    /// The ring's `words` are the very samples its mesh's faces were cut from ([`crate::surface`]
    /// measures one sample a column and reads both from it), so a face and the march that lights it
    /// cannot disagree about which cells hold matter.
    ///
    /// **A tier that is not the ring's cell is refused rather than resampled**: a 64 m band must
    /// not be written from a ring measured at 128 m, because the walk would then put the wrong
    /// ground under every ray it answers.
    ///
    /// Returns the number of words written.
    pub fn stamp_surface(&mut self, ring: &crate::surface::SurfaceRing) -> usize {
        assert_eq!(
            ring.cell_m,
            crate::lod::cell_m(ring.tier),
            "a surface band is measured at its tier's cell: tier {} carries {:.0} m, and the ring \
             states {:.0} m",
            ring.tier,
            crate::lod::cell_m(ring.tier),
            ring.cell_m,
        );
        let slot = Self::surface_slot(ring.tier).unwrap_or_else(|| {
            panic!(
                "tier {} is not a surface rung: the ladder's rungs are {}..{}",
                ring.tier,
                crate::lod::SURFACE_FIRST_TIER,
                crate::lod::TIERS - 1,
            )
        });
        assert_eq!(
            ring.words.len(),
            SURFACE_BAND_WORDS,
            "a rung's band is {} words, one a disc cell",
            SURFACE_BAND_WORDS
        );
        let band = &mut self.surfaces[slot];
        band.tier = ring.tier;
        band.cell_m = ring.cell_m as f32;
        band.anchor_cell = ring.origin_cell;
        band.words.copy_from_slice(&ring.words);
        band.live = true;
        band.dirty = true;
        ring.words.len()
    }

    /// **How much of one surface rung's cell at a world point holds matter** — `Some(0.0)` for a
    /// stamped clear cell, `None` outside the rung's disc or where the rung has not been measured.
    ///
    /// The shader's own walk is the GPU's; this is the same band read on the CPU, which is what a
    /// test can hold a drawn face against.
    pub fn surface_share_at(&self, tier: u32, world_m: [f64; 3]) -> Option<f32> {
        let slot = Self::surface_slot(tier)?;
        let band = self.surfaces.get(slot)?;
        if !band.live {
            return None;
        }
        let cell = f64::from(band.cell_m);
        let n = SURFACE_BAND_CELLS as i32;
        let q = [
            (world_m[0] / cell).floor() as i32 - band.anchor_cell[0],
            (world_m[1] / cell).floor() as i32 - band.anchor_cell[1],
            (world_m[2] / cell).floor() as i32 - band.anchor_cell[2],
        ];
        if q.iter().any(|v| *v < 0 || *v >= n) {
            return None;
        }
        let word = band.words[(q[0] + n * (q[1] + n * q[2])) as usize];
        if word & FAR_MEASURED == 0 {
            return None;
        }
        Some((word & FAR_SHARE) as f32 / FAR_SHARE as f32)
    }

    /// **The bytes the ladder's bands take**, of which the surface rungs are this many: the
    /// cost of the rungs above a chunk, stated where a budget can read it.
    pub fn surface_band_bytes() -> u64 {
        (SURFACE_SLOTS * SURFACE_BAND_WORDS * 4) as u64
    }

    /// The rungs measured into the table, nearest first, and their cells in metres.
    pub fn surface_bands(&self) -> Vec<(u32, f32)> {
        self.surfaces
            .iter()
            .filter(|band| band.live)
            .map(|band| (band.tier, band.cell_m))
            .collect()
    }

    /// Clear the chunk-tier cells one chunk's box owns: the chunk itself, aligned to its corner.
    fn clear_chunk_band(&mut self, base: [i32; 3]) {
        if let Some(index) = self.chunk_band_index(base) {
            self.chunk[index] = 0;
        }
    }

    /// Clear the wide cells one chunk's box owns: two 16 m cells a side, aligned to its corner.
    fn clear_wide_chunk(&mut self, base: [i32; 3]) {
        let step = HORIZON_WIDE_CELL_M as i32;
        for fy in 0..2 {
            for fz in 0..2 {
                for fx in 0..2 {
                    let corner = [
                        base[0] + fx * step,
                        base[1] + fy * step,
                        base[2] + fz * step,
                    ];
                    if let Some(index) = self.wide_index(corner) {
                        self.wide[index] = 0;
                    }
                }
            }
        }
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
        // **Both bands, under one flag.** The coarse band used to be uploaded in a second
        // `dirty` block, after the first had already cleared it: the far buffer never left its
        // zeroed state, so every coarse cell on the GPU read as unstamped and the coarse rung
        // answered nothing for the whole run.
        if self.dirty {
            let mut bytes = Vec::with_capacity(self.cells.len() * 4);
            for cell in &self.cells {
                bytes.extend_from_slice(&cell.to_le_bytes());
            }
            queue.write_buffer(&self.buffer, 0, &bytes);
            // **The ladder's words, concatenated in rung order**: the shader adds each rung's
            // offset from `params.w`, so a new band is an append and not a new binding.
            let mut coarse =
                Vec::with_capacity((self.far.len() + self.wide.len() + self.chunk.len()) * 4);
            for band in [&self.far, &self.wide, &self.chunk] {
                for cell in band {
                    coarse.extend_from_slice(&cell.to_le_bytes());
                }
            }
            queue.write_buffer(&self.coarse_buffer, 0, &coarse);
            self.dirty = false;
        }
        // **A re-measured rung uploads its own slice and nothing else.** A rung above a chunk is
        // re-measured on every crossing of its own cell — the 64 m rung every 64 m walked — and
        // re-sending the whole ladder's 2.4 MB of bands for it would be a copy the frame can feel.
        for index in 0..self.surfaces.len() {
            if !self.surfaces[index].dirty {
                continue;
            }
            let words = std::mem::take(&mut self.surfaces[index].words);
            let mut bytes = Vec::with_capacity(words.len() * 4);
            for word in &words {
                bytes.extend_from_slice(&word.to_le_bytes());
            }
            queue.write_buffer(
                &self.coarse_buffer,
                (self.surface_offset(index) * 4) as u64,
                &bytes,
            );
            self.surfaces[index].words = words;
            self.surfaces[index].dirty = false;
        }
        let mut view = [0.0f32; 40 + SURFACE_ENTRY_FLOATS * SURFACE_SLOTS];
        for axis in 0..3 {
            view[axis] = self.anchor[axis] as f32 - eye_m[axis] as f32;
            view[8 + axis] = self.far_anchor[axis] as f32 - eye_m[axis] as f32;
            view[16 + axis] = self.wide_anchor[axis] as f32 - eye_m[axis] as f32;
            view[24 + axis] = self.chunk_anchor[axis] as f32 - eye_m[axis] as f32;
        }
        view[4] = HORIZON_CELL_M;
        view[5] = HORIZON_CELLS as f32;
        view[6] = HORIZON_CELLS as f32;
        view[7] = HORIZON_CELLS as f32;
        view[12] = HORIZON_FAR_CELL_M;
        view[13] = HORIZON_FAR_CELLS as f32;
        view[14] = HORIZON_FAR_CELLS as f32;
        view[15] = HORIZON_FAR_CELLS as f32;
        view[20] = HORIZON_WIDE_CELL_M;
        view[21] = HORIZON_WIDE_CELLS as f32;
        view[22] = HORIZON_WIDE_CELLS as f32;
        view[23] = HORIZON_WIDE_CELLS as f32;
        view[28] = HORIZON_CHUNK_CELL_M;
        view[29] = HORIZON_CHUNK_CELLS as f32;
        view[30] = HORIZON_CHUNK_CELLS as f32;
        view[31] = HORIZON_CHUNK_CELLS as f32;
        // **Each rung's word offset in the one coarse buffer** — `offsets` in `HorizonView`.
        let far_words = (HORIZON_FAR_CELLS * HORIZON_FAR_CELLS * HORIZON_FAR_CELLS) as f32;
        let wide_words = (HORIZON_WIDE_CELLS * HORIZON_WIDE_CELLS * HORIZON_WIDE_CELLS) as f32;
        view[32] = 0.0;
        view[33] = far_words;
        view[34] = far_words + wide_words;
        // **`offsets.w` is how far the rung table reaches**: the slot past the highest stamped
        // rung, so a fragment's own lookup walks the rungs that exist and not the twenty-four the
        // table can hold.
        let live = self
            .surfaces
            .iter()
            .rposition(|band| band.live)
            .map_or(0, |slot| slot + 1);
        view[35] = live as f32;
        // **The rung table**: one entry a slot, `(to_anchor, params)` — where the rung's grid
        // starts in this frame's eye-relative frame, its own cell and reach, and where its words
        // begin in the ladder's one coarse buffer.
        for slot in 0..self.surfaces.len() {
            let band = &self.surfaces[slot];
            if !band.live {
                continue;
            }
            let base = 40 + slot * SURFACE_ENTRY_FLOATS;
            let anchor = [
                band.anchor_cell[0] as f64 * f64::from(band.cell_m),
                band.anchor_cell[1] as f64 * f64::from(band.cell_m),
                band.anchor_cell[2] as f64 * f64::from(band.cell_m),
            ];
            for axis in 0..3 {
                view[base + axis] = (anchor[axis] - eye_m[axis]) as f32;
            }
            view[base + 4] = band.cell_m;
            view[base + 5] = SURFACE_BAND_CELLS as f32;
            view[base + 6] = band.reach_m();
            view[base + 7] = self.surface_offset(slot) as f32;
        }
        let mut bytes = [0u8; UNIFORM_BYTES as usize];
        for (slot, value) in view.iter().enumerate() {
            bytes[slot * 4..slot * 4 + 4].copy_from_slice(&value.to_le_bytes());
        }
        bytes[144..148].copy_from_slice(&self.coverage().to_le_bytes());
        queue.write_buffer(&self.uniform, 0, &bytes);
    }
}

/// Whether a chunk's own box touches `anchor .. anchor + span` on every axis.
fn in_box(origin: [i32; 3], anchor: [i32; 3], span: i32) -> bool {
    (0..3)
        .all(|axis| origin[axis] < anchor[axis] + span && origin[axis] + CHUNK_SIDE > anchor[axis])
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
            let from = (((y - shift[1]) * side + (z - shift[2])) * side + (lo[0] - shift[0]))
                as usize
                * stride;
            let to = ((y * side + z) * side + lo[0]) as usize * stride;
            moved[to..to + run].copy_from_slice(&words[from..from + run]);
        }
    }
    moved
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
        let (device, queue, _name) =
            crate::renderer::offscreen_device().expect("a GPU for the volume");
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
        assert_eq!(
            stream.solid_at([-100.0, 0.0, 0.0]),
            None,
            "outside is not air"
        );
        assert!(
            !stream.wants(ChunkPos::new(0, 0, 0), 1),
            "a version is stamped once"
        );
        assert!(
            stream.wants(ChunkPos::new(0, 0, 0), 2),
            "a new version of the tree wants stamping"
        );
        // **The claim the shader reads**: the eye's own chunk is slot `1 + 3·(1 + 3·1)`, and a
        // second chunk sets its own bit rather than speaking for the box.
        assert_eq!(
            stream.coverage(),
            1 << 13,
            "the stamped slot is not the mask's own bit"
        );
        stream.stamp(ChunkPos::new(1, 0, 0), &tree_with_roof(), 1);
        assert_eq!(stream.coverage(), (1 << 13) | (1 << 14));
        assert!(
            stream.wants(ChunkPos::new(2, 0, 0), 1) && stream.coverage() & (1 << 15) == 0,
            "a chunk outside the fine cube claimed one of its slots"
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
        assert_eq!(
            stream.stamp(ChunkPos::new(0, 0, 0), &tree, 1),
            1,
            "one voxel, one cell"
        );

        assert_eq!(
            stream.solid_at([5.5, 2.5, 5.5]),
            Some(true),
            "the cell is not empty"
        );
        assert_eq!(
            stream.subvoxel_at([5.125, 2.5, 5.125]),
            Some(true),
            "the solid column is not solid: the mask dropped the subvoxel grid"
        );
        for dug in [
            [5.375, 2.5, 5.125],
            [5.125, 2.5, 5.375],
            [5.875, 2.875, 5.875],
        ] {
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
        assert_eq!(
            stream.solid_at([4.0, 3.4, 4.0]),
            Some(true),
            "the roof was not stamped"
        );
        assert!(
            !stream.recenter([1.0, 1.0, 1.0]),
            "the same chunk does not move it"
        );

        assert!(stream.recenter([40.0, 1.0, 1.0]), "the next chunk does");
        assert_eq!(
            stream.solid_at([4.0, 3.4, 4.0]),
            Some(true),
            "chunk 0 is still in the box, so its stamp is still the truth about it"
        );
        assert!(
            !stream.wants(ChunkPos::new(0, 0, 0), 1),
            "a carried chunk is not walked again"
        );
        assert!(
            stream.wants(ChunkPos::new(2, 0, 0), 1),
            "the chunk that entered wants a stamp"
        );
        // **Past the fine box, the coarse band still owns the chunk.** Chunk 3 is 96 m away and
        // wants its 8 m summary; chunk 9 is past the 512 m band and wants nothing.
        assert!(
            stream.wants(ChunkPos::new(3, 0, 0), 1),
            "a chunk in the coarse band was dropped"
        );
        assert!(
            !stream.wants(ChunkPos::new(9, 0, 0), 1),
            "a chunk past both bands wants a stamp"
        );

        // **And it stamps only its summary**: no fine cell of chunk 3 is in the cube, so the walk
        // marks none, while the coarse cells it covers read as measured and full.
        let mut far_only = SparseVoxelOctree::new();
        far_only.set_material_region([0, 0, 0], 5, MaterialId(1));
        assert_eq!(
            stream.stamp(ChunkPos::new(3, 0, 0), &far_only, 1),
            0,
            "a chunk outside the fine cube marked one of its cells"
        );
        assert_eq!(
            stream.far_share_at([100.0, 4.0, 4.0]),
            Some(1.0),
            "a chunk past the fine box stamped no coarse summary: far chunks have no light rung"
        );

        assert!(stream.recenter([80.0, 1.0, 1.0]));
        assert_eq!(
            stream.solid_at([4.0, 3.4, 4.0]),
            None,
            "a cell outside the cube has no answer"
        );
        assert!(
            stream.far_share_at([4.0, 3.5, 4.0]).is_some(),
            "a chunk still in the coarse band lost its summary at the crossing"
        );
        assert!(
            stream.stamped.contains_key(&(0, 0, 0)) && !stream.wants(ChunkPos::new(0, 0, 0), 1),
            "a carried coarse stamp was thrown away and would be walked again"
        );
    }

    /// **The crossing walked the other way.** A shift has two signs and the test above only asks
    /// for one: the eye stepping to −x moves the band by +32 m, and a band that carried the wrong
    /// sign would leave its cells a chunk off its own anchor — the rock that is in front of the
    /// player answering for the air he is standing in.
    #[test]
    fn a_crossing_toward_negative_x_carries_the_cube_the_same_way() {
        let (device, _queue) = device();
        let mut stream = HorizonStream::new(&device);
        assert!(stream.recenter([1.0, 1.0, 1.0]));
        stream.stamp(ChunkPos::new(0, 0, 0), &tree_with_roof(), 1);
        assert!(
            stream.recenter([-1.0, 1.0, 1.0]),
            "the step back did not move the box"
        );
        assert_eq!(
            stream.solid_at([4.0, 3.4, 4.0]),
            Some(true),
            "the roof is at a different address after the band moved toward −x"
        );
        assert_eq!(
            stream.solid_at([4.0, 2.4, 4.0]),
            Some(false),
            "the cell under the roof took the roof's matter"
        );
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
        assert_eq!(
            stream.solid_at([4.0, 3.4, 4.0]),
            Some(true),
            "the roof was not stamped"
        );
        assert!(
            !stream.wants(ChunkPos::new(0, 0, 0), 6),
            "a held version is not walked again"
        );
        assert_eq!(
            stream.stamp(ChunkPos::new(0, 0, 0), &tree_with_roof(), 6),
            16,
            "a forced stamp is a walk: the caller asked for the cells, not for an opinion"
        );

        // The player breaks the whole roof: the rebuilt chunk arrives as version 7.
        let empty = SparseVoxelOctree::new();
        assert!(
            stream.wants(ChunkPos::new(0, 0, 0), 7),
            "the edit must want stamping"
        );
        assert_eq!(
            stream.stamp(ChunkPos::new(0, 0, 0), &empty, 7),
            0,
            "nothing is left to mark"
        );
        assert_eq!(
            stream.solid_at([4.0, 3.4, 4.0]),
            Some(false),
            "the roof of the tree before the edit is still standing"
        );
    }

    /// **A chunk walked at the coarse rung is not a chunk walked at the fine one.**
    ///
    /// The sequence is the ordinary walk, not a contrivance: a chunk is first seen eight chunks
    /// out, so its stamp lands with none of its metres in the cube and writes only its 8 m summary;
    /// two crossings later the player is standing in it, at the **same version** of the same tree.
    /// The record used to carry the version alone, so `wants` answered "stamped" for a band that
    /// had never been walked: the slot's coverage bit was set, every cell was zero, and the chunk
    /// under the player read as **measured air** — `Some(false)`, which is a claim about the world,
    /// rather than `None`, which is the truth about the volume.
    #[test]
    fn a_chunk_stamped_at_the_coarse_rung_is_walked_again_when_it_enters_the_fine_cube() {
        let (device, _queue) = device();
        let mut stream = HorizonStream::new(&device);
        stream.recenter([1.0, 1.0, 1.0]);
        let far_chunk = ChunkPos::new(7, 0, 0);
        assert!(
            !stream.covers(far_chunk) && stream.covers_far(far_chunk),
            "the chunk this test walks into is not outside the fine cube and inside the coarse one"
        );
        assert_eq!(
            stream.stamp(far_chunk, &tree_with_roof(), 1),
            0,
            "a chunk outside the fine cube wrote a metre into it"
        );
        assert!(
            !stream.wants(far_chunk, 1),
            "the coarse summary did not land: the chunk will be walked again for the wrong reason"
        );

        // The walk only changes the chunk's distance from the eye. The tree is untouched.
        assert!(
            stream.recenter([7.0 * 32.0 + 1.0, 1.0, 1.0]),
            "the walk did not move the band"
        );
        assert!(
            stream.covers(far_chunk),
            "the chunk did not enter the fine cube"
        );
        assert_eq!(
            stream.coverage() & (1 << 13),
            0,
            "a slot whose only walk was the 8 m summary told the shader its metres were measured"
        );
        assert!(
            stream.wants(far_chunk, 1),
            "a chunk summarised at 8 m answered for a 1 m band it was never walked in"
        );

        // What the frame's own loop does with a `wants`, and what the march then reads.
        let roof = [7.0 * 32.0 + 4.0, 3.4, 4.0];
        stream.stamp(far_chunk, &tree_with_roof(), 1);
        assert_ne!(
            stream.coverage() & (1 << 13),
            0,
            "the slot stayed unmeasured after the chunk was walked at 1 m"
        );
        assert_eq!(
            stream.solid_at(roof),
            Some(true),
            "the chunk the player walked into measures as empty air: its roof was never stamped \
             at 1 m"
        );
        assert_eq!(
            stream.solid_at([roof[0], 2.4, 4.0]),
            Some(false),
            "the cell under the roof took the roof's matter"
        );
    }

    /// **A crossing owes the whole slab that enters, not the chunk the player is standing in.**
    ///
    /// This is the player's own report — "chunks outside the one the player is in aren't
    /// sampling" — written as a sequence. The slab ahead is the slab every player walks into, and
    /// it is summarised at 8 m while it is still outside the cube, one ordinary frame's work. The
    /// frame loop stamps the eye's chunk **whatever the record says** (a crossing re-stamps it) and
    /// every other chunk only when `wants` says so, so a record that answered for a band it never
    /// wrote left the nine chunks ahead unmeasured on the very frame they became the ground under
    /// the player's feet: one chunk sampled, twenty-six falling back to the shadow map.
    #[test]
    fn a_crossing_stamps_every_chunk_of_the_slab_that_enters_the_cube() {
        let (device, _queue) = device();
        let mut stream = HorizonStream::new(&device);
        assert!(stream.recenter([1.0, 1.0, 1.0]));

        // The slab ahead, summarised while it is outside the cube — and nothing else about it
        // changes between here and the crossing below.
        for pos in (-1..=1).flat_map(|y| (-1..=1).map(move |z| ChunkPos::new(2, y, z))) {
            assert!(
                stream.covers_far(pos) && !stream.covers(pos),
                "chunk {pos:?} of the slab ahead is not outside the cube and inside the coarse band"
            );
            stream.stamp(pos, &tree_with_roof(), 1);
            assert!(!stream.wants(pos, 1), "the slab's summary did not land");
        }

        // One chunk forward. The slab is inside the cube and its trees are untouched.
        assert!(
            stream.recenter([33.0, 1.0, 1.0]),
            "the walk did not move the box"
        );
        let mut wanted = 0;
        for y in -1..=1 {
            for z in -1..=1 {
                let pos = ChunkPos::new(2, y, z);
                assert!(stream.covers(pos), "chunk {pos:?} did not enter the cube");
                if stream.wants(pos, 1) {
                    wanted += 1;
                }
            }
        }
        assert_eq!(
            wanted,
            9,
            "the crossing left {} of the nine entering chunks unsampled",
            9 - wanted
        );

        // Nine a frame is the crossing's budget, so the slab is walked on the frame it enters.
        for y in -1..=1 {
            for z in -1..=1 {
                let pos = ChunkPos::new(2, y, z);
                if stream.wants(pos, 1) {
                    stream.stamp(pos, &tree_with_roof(), 1);
                }
            }
        }
        for y in -1..=1 {
            for z in -1..=1 {
                let roof = [
                    2.0 * 32.0 + 4.0,
                    y as f64 * 32.0 + 3.4,
                    z as f64 * 32.0 + 4.0,
                ];
                assert_eq!(
                    stream.solid_at(roof),
                    Some(true),
                    "chunk {y},{z} of the slab the player walked into measures as empty air"
                );
            }
        }
    }

    /// **A band the crossing cut is not a band the chunk is stamped in.**
    ///
    /// The other direction of the same lie, and the one a player makes every time they turn
    /// around: a chunk walked while the cube held it, left behind in the coarse band, and walked
    /// back into at the same version. Leaving cut its metres out of the volume — that cut is what
    /// makes a crossing nine stamps instead of twenty-seven — so the record has to lose the fine
    /// band with them. It did not, and the chunk the player returned to read as measured air under
    /// a coverage bit that said it had been looked at.
    #[test]
    fn a_band_the_crossing_cut_is_walked_again_when_the_chunk_returns() {
        let (device, _queue) = device();
        let mut stream = HorizonStream::new(&device);
        stream.recenter([1.0, 1.0, 1.0]);
        stream.stamp(ChunkPos::new(0, 0, 0), &tree_with_roof(), 1);
        assert_eq!(
            stream.solid_at([4.0, 3.4, 4.0]),
            Some(true),
            "the roof was not stamped"
        );

        // Four chunks out: the fine cube no longer holds chunk 0, and its metres are cut.
        assert!(stream.recenter([4.0 * 32.0 + 1.0, 1.0, 1.0]));
        assert!(
            !stream.covers(ChunkPos::new(0, 0, 0)) && stream.covers_far(ChunkPos::new(0, 0, 0)),
            "the crossing did not leave this chunk in the coarse band alone"
        );
        assert_eq!(
            stream.solid_at([4.0, 3.4, 4.0]),
            None,
            "a cell outside the cube has no answer"
        );

        // And back, at the same version: the fine band owes a walk it was never asked for.
        assert!(stream.recenter([1.0, 1.0, 1.0]));
        assert!(
            stream.wants(ChunkPos::new(0, 0, 0), 1),
            "a chunk whose fine cells the crossing cut was treated as walked when it came back"
        );
        stream.stamp(ChunkPos::new(0, 0, 0), &tree_with_roof(), 1);
        assert_eq!(
            stream.solid_at([4.0, 3.4, 4.0]),
            Some(true),
            "the chunk the player walked back into measures as empty air"
        );
    }

    /// **A whole cube is a frame's budget, and a crossing owes only the slab.**
    ///
    /// The player's report — *"chunks outside the one the player is in aren't sampling"* — is a
    /// statement about which slots are lit, so it is written here as one, slot by slot: nothing is
    /// lit before it is walked, a first fill of the cube lights all twenty-seven in one frame's
    /// work, and the crossing after it lights the nine that entered **in that same frame** while
    /// the eighteen carried by `recenter` stay lit. That budget is what makes the frame's refusal
    /// cheap to hold: a chunk that is unlit after a crossing is a chunk the budget could not have
    /// reached, and with a cube's worth of budget there is no such chunk.
    #[test]
    fn a_whole_cube_is_a_frames_budget_and_a_crossing_owes_only_its_slab() {
        let (device, _queue) = device();
        let mut stream = HorizonStream::new(&device);
        stream.recenter([1.0, 1.0, 1.0]);
        let cube = |xs: [i32; 3]| -> Vec<ChunkPos> {
            let mut out = Vec::new();
            for x in xs {
                for y in -1..=1 {
                    for z in -1..=1 {
                        out.push(ChunkPos::new(x, y, z));
                    }
                }
            }
            out
        };

        for pos in cube([-1, 0, 1]) {
            assert!(stream.covers(pos), "{pos:?} is not in the fine cube");
            assert!(!stream.lit(pos), "{pos:?} is lit before anything walked it");
        }
        assert!(
            !stream.lit(ChunkPos::new(4, 0, 0)),
            "a chunk outside the cube read as lit"
        );

        // The first fill, at the frame loop's budget: every chunk of the cube wants a walk.
        let mut walked = 0;
        for pos in cube([-1, 0, 1]) {
            assert!(stream.wants(pos, 1), "{pos:?} did not want its first walk");
            stream.stamp(pos, &tree_with_roof(), 1);
            walked += 1;
        }
        assert_eq!(walked, 27, "the cube is not three chunks a side");
        for pos in cube([-1, 0, 1]) {
            assert!(
                stream.lit(pos),
                "{pos:?} was walked and is still unlit: the frame would refuse it"
            );
        }
        assert_eq!(stream.coverage().count_ones(), 27, "the mask lost a slot");

        // One chunk forward: nine enter, eighteen are carried, and one frame's budget covers it.
        assert!(
            stream.recenter([33.0, 1.0, 1.0]),
            "the walk did not move the box"
        );
        let mut owed = 0;
        for pos in cube([0, 1, 2]) {
            assert!(stream.covers(pos), "{pos:?} did not enter the cube");
            if stream.wants(pos, 1) {
                owed += 1;
                stream.stamp(pos, &tree_with_roof(), 1);
            }
        }
        assert_eq!(
            owed, 9,
            "a crossing owes {owed} walks, not the nine of the slab"
        );
        for pos in cube([0, 1, 2]) {
            assert!(
                stream.lit(pos),
                "{pos:?} is in the cube, was owed at most one walk, and is unlit"
            );
        }
        assert_eq!(
            stream.coverage().count_ones(),
            27,
            "the crossing lost a slot"
        );
    }

    #[test]
    fn the_shaders_reach_and_the_hosts_are_one_number() {
        let source = include_str!("horizon.wgsl");
        for (what, literal) in [
            ("the march reach", "const HORIZON_REACH_M: f32 = 96.0;"),
            ("the dome reach", "const HORIZON_SKY_REACH_M: f32 = 32.0;"),
            ("the cell size", "const HORIZON_CELL_M: f32 = 1.0;"),
            (
                "the subvoxel side",
                "const HORIZON_SUBVOXEL_SIDE: u32 = 4u;",
            ),
            (
                "the coarse reach",
                "const HORIZON_FAR_REACH_M: f32 = 512.0;",
            ),
            ("the coarse share", "const FAR_SHARE: u32 = 0xFFu;"),
            ("the coarse stamped bit", "const FAR_MEASURED: u32 = 256u;"),
            (
                "the surface table's width",
                "const SURFACE_RUNG_SLOTS: u32 = 24u;",
            ),
        ] {
            assert!(
                source.contains(literal),
                "the shader's {what} moved without this module's"
            );
        }
        assert_eq!(HORIZON_CELLS as f32 * HORIZON_CELL_M, 96.0);
        assert_eq!(HORIZON_FAR_CELLS as f32 * HORIZON_FAR_CELL_M, 512.0);
        assert_eq!(HORIZON_SUBVOXEL_M, 0.25);
        assert_eq!(SKY_RAYS.len(), 3);
        // **The rung table's own width, both sides of the literal above.** The uniform is one entry
        // a surface tier, and the ladder's own count of tiers above a chunk is the same number.
        assert_eq!(SURFACE_SLOTS, 24);
        assert_eq!(
            SURFACE_SLOTS,
            crate::lod::TIERS as usize - crate::lod::SURFACE_FIRST_TIER as usize
        );
        assert_eq!(SURFACE_SLOTS, horizon_slots());
        assert_eq!(
            SURFACE_BAND_CELLS as i32,
            crate::surface::RING_CELLS,
            "a rung's band is its own disc"
        );
        assert_eq!(
            UNIFORM_BYTES,
            ((40 + SURFACE_ENTRY_FLOATS * SURFACE_SLOTS) * 4) as u64,
            "the uniform is the ten vec4 of the bands plus one entry a rung"
        );
    }

    /// The slots a ladder of every surface tier needs, counted the way [`HorizonStream::new`]
    /// builds them.
    fn horizon_slots() -> usize {
        (crate::lod::SURFACE_FIRST_TIER..crate::lod::TIERS).count()
    }

    /// **A surface rung stamps the samples its own faces were cut from**, and the band answers a
    /// point the way the mesh drew it: the share in the sample's own cell, full below it, measured
    /// air above it, and nothing outside the disc.
    #[test]
    fn a_surface_rung_stamps_its_own_samples_into_the_ladders_one_buffer() {
        let (device, _queue) = device();
        let mut stream = HorizonStream::new(&device);
        let eye = crate::game::spawn_eye(&crate::game::World::shipped());
        stream.recenter(eye);
        assert_eq!(stream.surface_bands(), Vec::new(), "nothing is stamped yet");
        assert_eq!(
            stream.surface_share_at(crate::lod::SURFACE_FIRST_TIER, eye),
            None,
            "an unstamped rung answered a ray"
        );
        let spec =
            voxel_shapers::WorldSpec::new(crate::GAME_SEED, voxel_shapers::Config::default());
        let tier = crate::lod::SURFACE_FIRST_TIER;
        let ring = crate::surface::sample(&spec, eye, tier);
        assert_eq!(stream.stamp_surface(&ring), SURFACE_BAND_WORDS);
        assert_eq!(stream.surface_bands(), vec![(tier, ring.cell_m as f32)]);
        // The disc's own middle band, where every column is drawn.
        let (i, j) = (crate::surface::RING_MID, crate::surface::RING_MID - 12);
        let column = ring.column(i, j).expect("a column of the disc");
        assert!(column.measured(), "the sampled column has no material");
        assert!(ring.drawn(i, j), "the sampled column is inside the annulus");
        let cell = ring.cell_m;
        let x = (ring.origin_cell[0] + i) as f64 * cell + cell * 0.5;
        let z = (ring.origin_cell[2] + j) as f64 * cell + cell * 0.5;
        let at = |k: i32| (ring.origin_cell[1] + k) as f64 * cell + cell * 0.5;
        let top = column.layer - ring.origin_cell[1];
        let share = stream
            .surface_share_at(tier, [x, at(top), z])
            .expect("the cell the sample stands in is measured");
        assert!(
            (share - column.share).abs() < 1.0 / 255.0,
            "the band says {share} where the mesh's own sample says {}",
            column.share
        );
        assert_eq!(
            stream.surface_share_at(tier, [x, at(top + 2), z]),
            Some(0.0),
            "the cells above a drawn face are measured air"
        );
        assert_eq!(
            stream.surface_share_at(tier, [x, at(top - 2), z]),
            Some(1.0),
            "the cells below a drawn face are the column's own matter"
        );
        assert_eq!(
            stream.surface_share_at(tier, [x + 40.0 * cell, at(top), z]),
            None,
            "a cell no rung has stamped claimed to be measured"
        );
        // **And the slot's words are the ladder's own offsets apart**: one buffer, addressed by
        // each rung's entry, which is what makes a rung an append rather than a binding.
        let far_words = (HORIZON_FAR_CELLS * HORIZON_FAR_CELLS * HORIZON_FAR_CELLS) as usize;
        let wide_words = (HORIZON_WIDE_CELLS * HORIZON_WIDE_CELLS * HORIZON_WIDE_CELLS) as usize;
        let chunk_words =
            (HORIZON_CHUNK_CELLS * HORIZON_CHUNK_CELLS * HORIZON_CHUNK_CELLS) as usize;
        assert_eq!(
            stream.surface_offset(0),
            far_words + wide_words + chunk_words
        );
        assert_eq!(
            stream.surface_offset(1) - stream.surface_offset(0),
            SURFACE_BAND_WORDS
        );
        assert_eq!(
            stream.surface_offset(SURFACE_SLOTS - 1) + SURFACE_BAND_WORDS,
            far_words + wide_words + chunk_words + SURFACE_SLOTS * SURFACE_BAND_WORDS
        );
    }

    /// **A ring whose cell is not its tier's is refused, not resampled into the band.**
    #[test]
    #[should_panic(expected = "measured at its tier's cell")]
    fn a_surface_band_refuses_a_ring_at_the_wrong_cell() {
        let (device, _queue) = device();
        let mut stream = HorizonStream::new(&device);
        let spec =
            voxel_shapers::WorldSpec::new(crate::GAME_SEED, voxel_shapers::Config::default());
        let eye = crate::game::spawn_eye(&crate::game::World::shipped());
        let mut ring = crate::surface::sample(&spec, eye, crate::lod::SURFACE_FIRST_TIER);
        ring.cell_m *= 2.0;
        stream.stamp_surface(&ring);
    }

    /// **A tier below the surface ladder is not a rung of it.**
    #[test]
    #[should_panic(expected = "is not a surface rung")]
    fn a_surface_band_refuses_a_tier_below_the_ladder() {
        let (device, _queue) = device();
        let mut stream = HorizonStream::new(&device);
        let ring = crate::surface::SurfaceRing {
            tier: crate::lod::CHUNK_TIER,
            cell_m: crate::lod::cell_m(crate::lod::CHUNK_TIER),
            origin_cell: [0; 3],
            eye_cell: [0; 3],
            corner: Vec::new(),
            columns: Vec::new(),
            quads: Vec::new(),
            shapes: Vec::new(),
            words: vec![0; SURFACE_BAND_WORDS],
            near: false,
            shape: Vec::new(),
            shape_side: 0,
            sampled_us: 0,
        };
        stream.stamp_surface(&ring);
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
        assert_eq!(
            stream.far_share_at([4.0, 3.5, 4.0]),
            None,
            "nothing is stamped yet"
        );
        stream.stamp(ChunkPos::new(0, 0, 0), &tree_with_roof(), 1);
        // **A share, not a stone**: the roof is sixteen of this cell's 512 voxels.
        let share = stream
            .far_share_at([4.0, 3.5, 4.0])
            .expect("the cell is stamped");
        assert!(
            (share - 16.0 / 512.0).abs() < 1.0 / 255.0,
            "a one-voxel roof is {share} of its eight metres, not all of them"
        );
        assert_eq!(
            stream.far_solid_at([4.0, 3.5, 4.0]),
            Some(false),
            "and it is not a wall"
        );
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
        assert_eq!(
            stream.stamp(ChunkPos::new(0, 0, 0), &full, 2),
            512,
            "eight metres, not one"
        );
        assert_eq!(
            stream.far_share_at([4.0, 3.5, 4.0]),
            Some(1.0),
            "a full cell is a full share"
        );
        assert_eq!(stream.far_solid_at([4.0, 3.5, 4.0]), Some(true));
        assert_eq!(
            stream.far_share_at([12.0, 3.5, 4.0]),
            Some(0.0),
            "the neighbouring cell is its own share and holds nothing"
        );
    }

    /// **The far field's mesh stands where its own band is measured.**
    ///
    /// This is the vertical slice's claim in one test: a chunk beyond the residency window yields a
    /// coarse surface, its census stamps that chunk's coarse cells, and **every face the mesh draws
    /// has a measurement in front of it**, and **stands in measured matter** — the lit fraction
    /// the slice promises is every face, not a subset. A face whose cell answered `None` would be
    /// the far field's version of a chunk lit by no horizon at all — exactly the stand-in this
    /// band exists to remove — and a face whose own cell is empty would be geometry the band says
    /// is air.
    #[test]
    fn a_far_chunks_surface_is_lit_by_the_coarse_band_it_stamped() {
        let (device, _queue) = device();
        let source = voxel_stream::WorldSource::new(voxel_shapers::WorldSpec::new(
            crate::game::GAME_SEED,
            voxel_shapers::Config::default(),
        ));
        let pos = ChunkPos::new(4, 0, 4);
        let tree = source.build_chunk(pos).expect("a chunk of the world");
        let census = crate::far_terrain::census(&tree);
        let mesh = crate::far_terrain::coarse_mesh(&census, pos, |_| None);
        assert!(
            !mesh.quads.is_empty(),
            "the far chunk has no surface to light"
        );

        let mut stream = HorizonStream::new(&device);
        let origin = pos.origin();
        let centre = [
            origin.x as f64 + CHUNK_SIDE as f64 * 0.5,
            origin.y as f64 + CHUNK_SIDE as f64 * 0.5,
            origin.z as f64 + CHUNK_SIDE as f64 * 0.5,
        ];
        stream.recenter(centre);
        assert_eq!(
            stream.stamp_far(pos, &census, 7),
            64,
            "a chunk owns four coarse cells a side"
        );
        let subvoxel_m = 1.0 / SUBVOXEL_SIDE as f64;
        let mut drawn = 0usize;
        let mut measured = 0usize;
        let mut in_matter = 0usize;
        for quad in &mesh.quads {
            let corners = quad.corners();
            let mut at = [0.0f64; 3];
            for corner in corners {
                for axis in 0..3 {
                    at[axis] += corner[axis] as f64 * subvoxel_m;
                }
            }
            let normal = quad.normal();
            let sample = [
                origin.x as f64 + at[0] / 4.0 + normal[0] as f64 * 0.5,
                origin.y as f64 + at[1] / 4.0 + normal[1] as f64 * 0.5,
                origin.z as f64 + at[2] / 4.0 + normal[2] as f64 * 0.5,
            ];
            // The face's own side of the plane: the coarse cell the surface stands in. The
            // band's share is `solid × 255 / 512`, so a cell the mesh drew (at or above half
            // solid) reads at least 127/255 here.
            let inward = [
                sample[0] - normal[0] as f64,
                sample[1] - normal[1] as f64,
                sample[2] - normal[2] as f64,
            ];
            drawn += 1;
            if stream.far_share_at(sample).is_some() {
                measured += 1;
            }
            if stream.far_share_at(inward).is_some_and(|share| share > 0.0) {
                in_matter += 1;
            }
        }
        assert!(drawn > 0, "the mesh drew nothing to light");
        assert_eq!(
            measured, drawn,
            "{measured} of {drawn} far faces have a coarse answer: the rest are lit by no measurement"
        );
        assert_eq!(
            in_matter, drawn,
            "{in_matter} of {drawn} far faces stand in measured matter: the band and the mesh disagree about where the surface is, so the lit fraction is not one"
        );
    }

    /// **The peak the one-layer slice clipped, drawn by the stack and lit by the same band.**
    ///
    /// The world's highest column is 54 m at (1792, 1888) and its surface reaches y = 59: the layer
    /// **above** the eye's chunk. A ring that drew only the eye's layer never asked for that chunk,
    /// so the top of the peak was simply absent from the horizon. The stack asks for it, and every
    /// face it cuts there is measured by the coarse band the census stamped — the same guarantee the
    /// eye-layer test makes, checked on the geometry the slice used to lose.
    #[test]
    fn the_stack_draws_a_peak_the_single_layer_would_clip() {
        let (device, _queue) = device();
        let source = voxel_stream::WorldSource::new(voxel_shapers::WorldSpec::new(
            crate::game::GAME_SEED,
            voxel_shapers::Config::default(),
        ));
        let peak = ChunkPos::new(56, 1, 59);
        let tree = source.build_chunk(peak).expect("the peak's chunk");
        let census = crate::far_terrain::census(&tree);
        let highest = census
            .cells()
            .iter()
            .map(|cell| cell.top_voxel)
            .max()
            .expect("a cell");
        assert!(
            highest > 0,
            "the peak's chunk holds no matter above its own floor: this is not the peak"
        );
        let mesh = crate::far_terrain::coarse_mesh(&census, peak, |_| None);
        assert!(
            !mesh.quads.is_empty(),
            "the peak has a coarse surface to draw"
        );

        // Six chunks west, the peak sits on the ring's own column, one layer up.
        let centre = ChunkPos::new(50, 0, 59);
        assert!(
            !crate::far_terrain::ring_at(5, centre, 0).contains(&peak),
            "the single-layer ring already draws it: this test proves nothing"
        );
        assert!(
            crate::far_terrain::stack_at(5, centre).contains(&peak),
            "the stack did not reach the layer the peak crosses"
        );

        let mut stream = HorizonStream::new(&device);
        let origin = peak.origin();
        stream.recenter([
            origin.x as f64 + CHUNK_SIDE as f64 * 0.5,
            origin.y as f64 + CHUNK_SIDE as f64 * 0.5,
            origin.z as f64 + CHUNK_SIDE as f64 * 0.5,
        ]);
        assert_eq!(
            stream.stamp_far(peak, &census, 9),
            64,
            "the peak's coarse cells were not stamped"
        );
        let subvoxel_m = 1.0 / SUBVOXEL_SIDE as f64;
        let mut drawn = 0usize;
        let mut lit = 0usize;
        for quad in &mesh.quads {
            let mut at = [0.0f64; 3];
            for corner in quad.corners() {
                for axis in 0..3 {
                    at[axis] += corner[axis] as f64 * subvoxel_m;
                }
            }
            let normal = quad.normal();
            let sample = [
                origin.x as f64 + at[0] / 4.0 + normal[0] as f64 * 0.5,
                origin.y as f64 + at[1] / 4.0 + normal[1] as f64 * 0.5,
                origin.z as f64 + at[2] / 4.0 + normal[2] as f64 * 0.5,
            ];
            drawn += 1;
            if stream.far_share_at(sample).is_some() {
                lit += 1;
            }
        }
        assert!(drawn > 0, "the peak drew nothing");
        assert_eq!(
            lit, drawn,
            "{lit} of {drawn} peak faces have a coarse answer: the drawn summit is unlit"
        );
    }

    /// **Tier 6's surface stands where its own band is measured.**
    ///
    /// The rung past the far ring is the same vertical slice one tier up: a chunk beyond the
    /// window yields a 16 m mesh, its census stamps the wide band, and every face the mesh draws
    /// has a measured 16 m cell in front of it **and** stands in measured matter. A face answered
    /// by the 8 m band would be a lie about the grain it was cut at; a face with no answer would be
    /// the unlit distance the rung exists to remove.
    #[test]
    fn a_tier_six_surface_is_lit_by_the_wide_band_it_stamped() {
        let (device, _queue) = device();
        let source = voxel_stream::WorldSource::new(voxel_shapers::WorldSpec::new(
            crate::game::GAME_SEED,
            voxel_shapers::Config::default(),
        ));
        let pos = ChunkPos::new(10, 0, 10);
        let tree = source.build_chunk(pos).expect("a chunk of the world");
        let census = crate::far_terrain::Census::of(&tree, crate::lod::WIDE_TIER);
        assert_eq!(census.side(), 2, "tier 6 is two cells a chunk axis");
        let mesh = crate::far_terrain::coarse_mesh(&census, pos, |_| None);
        assert!(
            !mesh.quads.is_empty(),
            "the tier-6 chunk has no surface to light"
        );

        let mut stream = HorizonStream::new(&device);
        let origin = pos.origin();
        let centre = [
            origin.x as f64 + CHUNK_SIDE as f64 * 0.5,
            origin.y as f64 + CHUNK_SIDE as f64 * 0.5,
            origin.z as f64 + CHUNK_SIDE as f64 * 0.5,
        ];
        stream.recenter(centre);
        assert_eq!(
            stream.stamp_wide(pos, &census, 11),
            8,
            "a tier-6 chunk owns eight wide cells (2³)"
        );
        let subvoxel_m = 1.0 / SUBVOXEL_SIDE as f64;
        let mut drawn = 0usize;
        let mut measured = 0usize;
        let mut in_matter = 0usize;
        for quad in &mesh.quads {
            let mut at = [0.0f64; 3];
            for corner in quad.corners() {
                for axis in 0..3 {
                    at[axis] += corner[axis] as f64 * subvoxel_m;
                }
            }
            let normal = quad.normal();
            let sample = [
                origin.x as f64 + at[0] / 4.0 + normal[0] as f64 * 0.5,
                origin.y as f64 + at[1] / 4.0 + normal[1] as f64 * 0.5,
                origin.z as f64 + at[2] / 4.0 + normal[2] as f64 * 0.5,
            ];
            let inward = [
                sample[0] - normal[0] as f64,
                sample[1] - normal[1] as f64,
                sample[2] - normal[2] as f64,
            ];
            drawn += 1;
            if stream.wide_share_at(sample).is_some() {
                measured += 1;
            }
            if stream
                .wide_share_at(inward)
                .is_some_and(|share| share > 0.0)
            {
                in_matter += 1;
            }
        }
        assert!(drawn > 0, "the tier-6 mesh drew nothing to light");
        assert_eq!(
            measured, drawn,
            "{measured} of {drawn} tier-6 faces have a wide answer"
        );
        assert_eq!(
            in_matter, drawn,
            "{in_matter} of {drawn} tier-6 faces stand in measured matter"
        );
    }

    /// **A band refuses a census of another grain**, in both directions: an 8 m stamp cannot write
    /// a 16 m band and a 16 m stamp cannot write the 8 m one. Silence here would put four times
    /// the world in a cell and shade it as if the grain were right.
    #[test]
    #[should_panic(expected = "the wide band is tier 6")]
    fn the_wide_band_refuses_a_coarse_census() {
        let (device, _queue) = device();
        let source = voxel_stream::WorldSource::new(voxel_shapers::WorldSpec::new(
            crate::game::GAME_SEED,
            voxel_shapers::Config::default(),
        ));
        let pos = ChunkPos::new(2, 0, 2);
        let tree = source.build_chunk(pos).expect("a chunk of the world");
        let coarse = crate::far_terrain::census(&tree);
        let mut stream = HorizonStream::new(&device);
        stream.recenter([64.0, 16.0, 64.0]);
        stream.stamp_wide(pos, &coarse, 1);
    }

    /// The other direction: an 8 m band refuses a 16 m census.
    #[test]
    #[should_panic(expected = "the coarse band is the far tier")]
    fn the_coarse_band_refuses_a_wide_census() {
        let (device, _queue) = device();
        let source = voxel_stream::WorldSource::new(voxel_shapers::WorldSpec::new(
            crate::game::GAME_SEED,
            voxel_shapers::Config::default(),
        ));
        let pos = ChunkPos::new(2, 0, 2);
        let tree = source.build_chunk(pos).expect("a chunk of the world");
        let wide = crate::far_terrain::Census::of(&tree, crate::lod::WIDE_TIER);
        let mut stream = HorizonStream::new(&device);
        stream.recenter([64.0, 16.0, 64.0]);
        stream.stamp_far(pos, &wide, 1);
    }

    /// **Tier 7's surface stands where its own band is measured.**
    ///
    /// The last chunk-held rung: one 32 m cell a chunk, stamped into a 32 m band that is one word a
    /// chunk. Every face the mesh draws must have a measured 32 m cell in front of it and measured
    /// matter behind it — the same contract as tiers 5 and 6, at the last grain a per-chunk census
    /// can hold.
    #[test]
    fn a_tier_seven_surface_is_lit_by_the_chunk_band_it_stamped() {
        let (device, _queue) = device();
        let source = voxel_stream::WorldSource::new(voxel_shapers::WorldSpec::new(
            crate::game::GAME_SEED,
            voxel_shapers::Config::default(),
        ));
        let pos = ChunkPos::new(10, 0, 10);
        let tree = source.build_chunk(pos).expect("a chunk of the world");
        let census = crate::far_terrain::Census::of(&tree, crate::lod::CHUNK_TIER);
        assert_eq!(census.side(), 1, "tier 7 is one cell a chunk axis");
        assert_eq!(census.cell_voxels(), 32 * 32 * 32);
        // Every neighbour measured air, so the chunk's own cube draws all six faces.
        let mesh = crate::far_terrain::coarse_mesh(&census, pos, |_| Some(false));
        assert!(
            !mesh.quads.is_empty(),
            "the tier-7 chunk has no surface to light"
        );

        let mut stream = HorizonStream::new(&device);
        let origin = pos.origin();
        let centre = [
            origin.x as f64 + CHUNK_SIDE as f64 * 0.5,
            origin.y as f64 + CHUNK_SIDE as f64 * 0.5,
            origin.z as f64 + CHUNK_SIDE as f64 * 0.5,
        ];
        stream.recenter(centre);
        assert_eq!(
            stream.stamp_chunk_band(pos, &census, 13),
            1,
            "a tier-7 chunk owns one cell"
        );
        let subvoxel_m = 1.0 / SUBVOXEL_SIDE as f64;
        let mut drawn = 0usize;
        let mut measured = 0usize;
        let mut in_matter = 0usize;
        for quad in &mesh.quads {
            let mut at = [0.0f64; 3];
            for corner in quad.corners() {
                for axis in 0..3 {
                    at[axis] += corner[axis] as f64 * subvoxel_m;
                }
            }
            let normal = quad.normal();
            let inward = [
                origin.x as f64 + at[0] / 4.0 - normal[0] as f64 * 0.5,
                origin.y as f64 + at[1] / 4.0 - normal[1] as f64 * 0.5,
                origin.z as f64 + at[2] / 4.0 - normal[2] as f64 * 0.5,
            ];
            drawn += 1;
            // A 32 m cell **is** the chunk, so the face's own cell is the one behind it: half a
            // metre past the plane is the neighbour's cell, which this chunk's stamp does not own.
            if stream.chunk_share_at(inward).is_some() {
                measured += 1;
            }
            if stream
                .chunk_share_at(inward)
                .is_some_and(|share| share > 0.0)
            {
                in_matter += 1;
            }
        }
        assert!(drawn > 0, "the tier-7 mesh drew nothing to light");
        assert_eq!(
            measured, drawn,
            "{measured} of {drawn} tier-7 faces have a 32 m answer"
        );
        assert_eq!(
            in_matter, drawn,
            "{in_matter} of {drawn} tier-7 faces stand in measured matter"
        );
    }

    /// The chunk band refuses a 16 m census.
    #[test]
    #[should_panic(expected = "the chunk band is tier 7")]
    fn the_chunk_band_refuses_a_wide_census() {
        let (device, _queue) = device();
        let source = voxel_stream::WorldSource::new(voxel_shapers::WorldSpec::new(
            crate::game::GAME_SEED,
            voxel_shapers::Config::default(),
        ));
        let pos = ChunkPos::new(2, 0, 2);
        let tree = source.build_chunk(pos).expect("a chunk of the world");
        let wide = crate::far_terrain::Census::of(&tree, crate::lod::WIDE_TIER);
        let mut stream = HorizonStream::new(&device);
        stream.recenter([64.0, 16.0, 64.0]);
        stream.stamp_chunk_band(pos, &wide, 1);
    }

    /// **The census a far mesh is cut from and the band a stamp writes are one measurement.**
    ///
    /// The mesh's faces and the march's shares must be the same walk. Two streams, one stamped
    /// from the tree and one from the census the mesh used: every coarse cell they own answers the
    /// same share, so a hill drawn solid cannot let a ray through two cells further along.
    #[test]
    fn the_census_a_far_mesh_uses_writes_the_same_band_words_as_its_tree() {
        let (device, _queue) = device();
        let source = voxel_stream::WorldSource::new(voxel_shapers::WorldSpec::new(
            crate::game::GAME_SEED,
            voxel_shapers::Config::default(),
        ));
        let pos = ChunkPos::new(2, 0, 3);
        let tree = source.build_chunk(pos).expect("a chunk of the world");
        let census = crate::far_terrain::census(&tree);
        let mut from_tree = HorizonStream::new(&device);
        let mut from_census = HorizonStream::new(&device);
        let origin = pos.origin();
        let centre = [
            origin.x as f64 + 16.0,
            origin.y as f64 + 16.0,
            origin.z as f64 + 16.0,
        ];
        from_tree.recenter(centre);
        from_census.recenter(centre);
        from_tree.stamp(pos, &tree, 1);
        from_census.stamp_far(pos, &census, 1);
        for z in 0..4 {
            for y in 0..4 {
                for x in 0..4 {
                    let at = [
                        origin.x as f64 + x as f64 * HORIZON_FAR_CELL_M as f64 + 4.0,
                        origin.y as f64 + y as f64 * HORIZON_FAR_CELL_M as f64 + 4.0,
                        origin.z as f64 + z as f64 * HORIZON_FAR_CELL_M as f64 + 4.0,
                    ];
                    assert_eq!(
                        from_tree.far_share_at(at),
                        from_census.far_share_at(at),
                        "the two stamps disagree at coarse cell {x},{y},{z}"
                    );
                }
            }
        }
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
        assert!(
            !stream.retain(&resident),
            "a chunk still resident is not forgotten"
        );
        let empty = BTreeSet::new();
        assert!(stream.retain(&empty), "an evicted chunk must be forgotten");
        assert_eq!(
            stream.solid_at([4.0, 3.4, 4.0]),
            Some(false),
            "the evicted chunk's roof is still standing in the fine band"
        );
        assert_eq!(
            stream.far_solid_at([4.0, 3.5, 4.0]),
            None,
            "the coarse band restreamed"
        );
    }
}
