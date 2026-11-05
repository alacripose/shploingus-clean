//! **The LOD ladder: one cell size a tier, and the tiers this tree actually inhabits.**
//!
//! A rung of the terrain ladder is not a distance and it is not a mesh LOD level — it is **the size
//! of the cell the world is measured in**. `tier 2` is a one-metre voxel, `tier 5` is an eight-metre
//! coarse cell, `tier 7` is a whole 32 m chunk as one cell. Geometry and light are the same rung by
//! construction: a face cut at eight metres and the coarse band's share are the same number because
//! they are read from the same census (see [`crate::far_terrain`]).
//!
//! # Why the base is a subvoxel
//!
//! The octree's finest address is a subvoxel — [`BASE_CELL_M`] = 0.25 m — and every other cell in
//! the world is a power of two of it: `Mixed` is 4 × 4 × 4 subvoxels, a voxel is 4 × 4 × 4 of those,
//! a chunk is 32 voxels. So the ladder is **one doubling per tier** from the subvoxel up, and the
//! three cells the tree already measures are tiers rather than free-floating constants:
//!
//! | tier | cell | what measures at it |
//! |---|---|---|
//! | 0 | 0.25 m | the octree's `Mixed` leaf, the subdivision the streamed mesh is cut at |
//! | 2 | 1 m | the fine light band ([`crate::horizon::HORIZON_CELL_M`]) |
//! | 5 | 8 m | the coarse light band **and** the far terrain ([`crate::far_terrain`]) |
//!
//! `tier 31` is **536 870 912 m ≈ 536 871 km** — the top of the address space, past the Earth's
//! diameter. Tiers past 7 are part of the
//! **address**, not a draw distance: they name cells larger than one chunk, which a per-chunk census
//! cannot hold. They are drawn instead from [`crate::surface`]'s own census — **one sample a
//! cell**, the generator's own surface — which needs no chunk at all.
//!
//! # Where this stops
//!
//! A [`crate::far_terrain::Census`] is one chunk's measurement, so it can only carry a tier whose
//! cell divides the chunk: tiers 5 (4 cells a side), 6 (2) and 7 (1). The rungs above 7 are cut by
//! [`crate::surface`] instead, which measures the generator's surface once per cell and reads both
//! the mesh and the band from that one sample.
//!
//! # What each tier's geometry is
//!
//! A tier's ring is an annulus of **cells**, not a distance: `8 ..= 16` cells from the eye's own
//! cell, so rung *t*'s outer edge is rung *t+1*'s inner edge exactly (`8 × 2C = 16 × C`), and the
//! rungs tile the ground from 384 m to [`surface_outer_m`]`(`[`SURFACE_LAST_DRAWN_TIER`]`)` =
//! **67 109 km**. The first rung starts at [`surface_inner_m`]'s own 384 m — the outer edge of
//! tier 7's drawn ring — because its own first whole cell past that edge is cell 6.

/// **How many tiers the ladder has.** One doubling each, from a subvoxel.
pub const TIERS: u32 = 32;

/// **The finest address**: one subvoxel, in metres.
pub const BASE_CELL_M: f64 = 0.25;

/// The octree's own subdivision — the tier the streamed mesh is cut at.
pub const SUBVOXEL_TIER: u32 = 0;

/// One voxel: the tier the streamed chunk's light is measured at.
pub const VOXEL_TIER: u32 = 2;

/// One eight-metre coarse cell: the tier of the coarse light band and the far terrain.
pub const FAR_TIER: u32 = 5;

/// Sixteen metres: the second terrain rung, and the cell its own band measures.
pub const WIDE_TIER: u32 = 6;

/// Sixteen metres, the wide band's cell and tier 6's terrain cell.
pub const WIDE_CELL_M: f32 = 16.0;

/// Wide cells along one chunk axis: a 32 m chunk is two.
pub const WIDE_CELLS_PER_CHUNK: i32 = 2;

/// Thirty-two metres: one cell a chunk — **the last chunk-held tier**.
pub const CHUNK_TIER: u32 = 7;

/// Thirty-two metres, the chunk tier's cell and its band's grain.
pub const CHUNK_CELL_M: f32 = 32.0;

/// Chunk cells along one chunk axis: one.
pub const CHUNK_CELLS_PER_CHUNK: i32 = 1;

/// **Sixty-four metres: one cell a 2 × 2 × 2 group of chunks** — the first rung whose cell is
/// larger than a chunk, measured by [`crate::far_terrain::Census::of_group`].
pub const GROUP_TIER: u32 = 8;

/// The group tier's cell, in metres.
pub const GROUP_CELL_M: f32 = 64.0;

/// Chunks along one edge of a group at this tier: eight chunks, two a side.
pub const GROUP_CHUNKS: i32 = 2;

/// **Where the surface ladder starts**: tier 8, a 64 m cell, the first cell larger than a chunk.
pub const SURFACE_FIRST_TIER: u32 = 8;

/// **One cell in this many sub-cell steps, for the surface rungs' own records.**
///
/// A surface rung's quads are in cell units, so its rectangle's coordinates are whole cells — but
/// its top face stands at the **sample's own height**, a fraction of a cell, which a `u16::MAX`
/// step could not state. `256` a cell is the finest step whose coordinates still fit a rectangle:
/// a 33-cell disc is `33 × 256 = 8 448`, well inside `u16`, and **a tier-8 cell's step is exactly
/// one of the octree's own subvoxels** (64 m / 256 = 0.25 m). See [`crate::surface`].
pub const SURFACE_SUB_CELLS: i32 = 256;

/// **The rung slots the light's rung table has**, one per surface tier — so a band is addressed by
/// its tier's own slot and a rung above the drawn range is an append rather than a new buffer.
pub const SURFACE_RUNG_SLOTS: usize = (TIERS - SURFACE_FIRST_TIER) as usize;

/// **The far plane the drawn ladder needs**, metres: twice the outermost radius
/// [`SURFACE_LAST_DRAWN_TIER`] draws at, so the furthest geometry this client draws is comfortably
/// inside a plane that discards what stands exactly on it (`camera::FAR_M`).
///
/// Twice, and not `1.05`, because a plane is a plane: it only has to be at least as far as the
/// geometry, and the factor is stated here rather than hidden in the camera. The number that comes
/// out is `2³¹ = 2 147 483 648 m` — the world's own `i32` voxel address, in metres, which is what
/// sets the drawn top in the first place.
pub const LADDER_FAR_M: f64 =
    2.0 * (SURFACE_OUTER_CELLS as f64 * (BASE_CELL_M * (1u64 << SURFACE_LAST_DRAWN_TIER) as f64));

/// **The last rung the surface ladder draws**: `28`, a 67 108 864 m cell whose outer radius is
/// **1 073 741 824 m = 1 073 742 km** — 2.8 Earth–Moon distances, 30× geostationary orbit.
///
/// **And it is 28 because that is where the world's own address stops.** The generator's material
/// law is stated in `i32` voxels — [`voxel_shapers::chunk_fields::material_at`] takes `i32` — and a
/// rung's disc reaches `16 × cell` from the eye, so the last tier whose whole disc can be **named**
/// is the last one with `16 × cell < 2³¹`: tier 29's own outer edge is exactly `2 147 483 648 m`,
/// the address space's edge, and tiers 30 and 31 stand past it. A rung out there could be sampled
/// for height (the elevation law is `f64`) and could not be given a material at all.
///
/// So the ladder is drawn from a subvoxel to **1.07 million km**, and the four tiers above the
/// drawn top are **address**: tier 29's cell is 134 217 728 m and its outer radius 2 147 483 648 m
/// — the i32 limit exactly — then 268 435 456 m / 4 294 967 296 m and 536 870 912 m /
/// 8 589 934 592 m at the top.
pub const SURFACE_LAST_DRAWN_TIER: u32 = 28;

/// **Cells from the eye's own to a rung's outer edge**: 16, so a rung's outer radius is
/// `16 × cell` — 1 024 m at tier 8 and 8 589 935 km at tier 31.
pub const SURFACE_OUTER_CELLS: i32 = 16;

/// **The top of the ladder**: `tier 31`'s cell, 0.25 m × 2³¹ = 536 870 912 m ≈ 536 871 km, and the
/// outer radius it would draw at (16 cells) = **8 589 934 592 m ≈ 8 589 935 km** — 22 Earth–Moons,
/// past the Sun's own radius. It is address rather than ground: see
/// [`SURFACE_LAST_DRAWN_TIER`] for why the drawn ladder stops four tiers below it.
pub const TOP_CELL_M: f64 = 536_870_912.0;

/// The drawn surface rungs, tier 8 through [`SURFACE_LAST_DRAWN_TIER`].
pub fn surface_tiers() -> Vec<u32> {
    (SURFACE_FIRST_TIER..=SURFACE_LAST_DRAWN_TIER).collect()
}

/// How many rungs the surface ladder draws.
pub const SURFACE_RUNGS: usize = (SURFACE_LAST_DRAWN_TIER - SURFACE_FIRST_TIER + 1) as usize;

/// **A surface rung's outer radius**, metres: `16 × cell`.
pub fn surface_outer_m(tier: u32) -> f64 {
    f64::from(SURFACE_OUTER_CELLS) * cell_m(tier)
}

/// **A surface rung's inner radius**, metres: the finer rung's outer edge, and **384 m** for the
/// first rung — the outer edge of tier 7's drawn ring (chunks `radius+5 ..= radius+6`, i.e.
/// 320..384 m at the shipped window), so the ladder begins where the chunk-held rungs end and no
/// ground between them is left undrawn.
pub fn surface_inner_m(tier: u32) -> f64 {
    if tier == SURFACE_FIRST_TIER {
        384.0
    } else {
        surface_outer_m(tier) / 2.0
    }
}

/// One metre, the fine band's cell ([`crate::horizon::HORIZON_CELL_M`]).
pub const VOXEL_CELL_M: f32 = 1.0;

/// Eight metres, the coarse band's and the far terrain's cell
/// ([`crate::horizon::HORIZON_FAR_CELL_M`]).
pub const FAR_CELL_M: f32 = 8.0;

/// Voxels along one far cell: a chunk is [`crate::voxel_model::CHUNK_SIDE`] = 32 m.
pub const FAR_CELL_VOXELS: i32 = FAR_CELL_M as i32;

/// Coarse cells along one axis of a far chunk.
pub const FAR_CELLS_PER_CHUNK: i32 = 4;

/// Cells in one far chunk's census.
pub const FAR_CELLS: usize =
    (FAR_CELLS_PER_CHUNK * FAR_CELLS_PER_CHUNK * FAR_CELLS_PER_CHUNK) as usize;

/// Voxels in one far cell, which a solid share is measured out of.
pub const FAR_CELL_VOXELS_TOTAL: u32 = (FAR_CELL_VOXELS * FAR_CELL_VOXELS * FAR_CELL_VOXELS) as u32;

/// Subvoxels along one far cell: the mesher's stride for the far tier.
pub const FAR_CELL_SUBVOXELS: i32 = FAR_CELL_VOXELS * voxel_model::SUBVOXEL_SIDE as i32;

const _: () = assert!(FAR_CELLS == 64);
const _: () = assert!(FAR_CELL_VOXELS_TOTAL == 512);

/// **The cell a tier measures**, in metres.
///
/// # Panics
///
/// A tier outside `0..TIERS` is not a rung: the ladder has a bottom and a top, and a caller that
/// asks past either has the wrong number rather than a large one.
pub fn cell_m(tier: u32) -> f64 {
    assert!(tier < TIERS, "tier {tier} is outside the ladder of {TIERS}");
    BASE_CELL_M * f64::from(1u32 << tier)
}

/// **The tier a cell size names**, or `None` when the size is not a rung.
///
/// Exact, not rounded: a 6 m cell is between tiers, and a caller that wants a rung has to pick one
/// rather than have one invented for it.
pub fn tier_of(cell_m: f64) -> Option<u32> {
    if !(cell_m > 0.0) {
        return None;
    }
    let ratio = cell_m / BASE_CELL_M;
    let tier = ratio.log2();
    if tier.fract() == 0.0 && (0.0..(TIERS as f64)).contains(&tier) {
        Some(tier as u32)
    } else {
        None
    }
}

/// **How many cells of a tier fit one chunk axis**, or `None` when they do not divide it.
///
/// This is the bound on a per-chunk census: tiers 5..=7 give 4, 2 and 1; a coarser tier is smaller
/// than one chunk and cannot be held chunk by chunk.
pub fn cells_per_chunk(tier: u32) -> Option<i32> {
    let cell = cell_m(tier);
    let side = f64::from(voxel_model::CHUNK_SIDE);
    let cells = side / cell;
    if cells >= 1.0 && cells.fract() == 0.0 {
        Some(cells as i32)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ladder_is_thirty_two_doublings_from_a_subvoxel() {
        assert_eq!(cell_m(SUBVOXEL_TIER), 0.25);
        assert_eq!(cell_m(VOXEL_TIER), 1.0);
        assert_eq!(cell_m(FAR_TIER), 8.0);
        // The names the docs use: 256 m at ten, 8.2 km at fifteen, 537 km at the top.
        assert_eq!(cell_m(10), 256.0);
        assert_eq!(cell_m(15), 8_192.0);
        assert_eq!(cell_m(31), 536_870_912.0);
        for tier in 0..TIERS - 1 {
            assert_eq!(
                cell_m(tier + 1),
                cell_m(tier) * 2.0,
                "tier {tier} is not one doubling below {}",
                tier + 1
            );
        }
    }

    #[test]
    fn a_tier_is_the_exact_inverse_of_its_cell() {
        for tier in 0..TIERS {
            assert_eq!(tier_of(cell_m(tier)), Some(tier));
        }
        assert_eq!(tier_of(6.0), None, "a size between rungs is not a rung");
        assert_eq!(tier_of(0.0), None);
        assert_eq!(tier_of(-8.0), None);
    }

    #[test]
    fn the_three_cells_the_tree_measures_are_tiers() {
        assert_eq!(cell_m(FAR_TIER), f64::from(FAR_CELL_M));
        assert_eq!(cell_m(VOXEL_TIER), f64::from(VOXEL_CELL_M));
        assert_eq!(
            cell_m(FAR_TIER),
            f64::from(crate::horizon::HORIZON_FAR_CELL_M),
            "the coarse band and the far terrain left the same tier"
        );
        assert_eq!(
            cell_m(VOXEL_TIER),
            f64::from(crate::horizon::HORIZON_CELL_M),
            "the fine band left the ladder"
        );
        assert_eq!(cells_per_chunk(FAR_TIER), Some(FAR_CELLS_PER_CHUNK));
        assert_eq!(cells_per_chunk(WIDE_TIER), Some(WIDE_CELLS_PER_CHUNK));
        assert_eq!(cell_m(WIDE_TIER), f64::from(WIDE_CELL_M));
        assert_eq!(cells_per_chunk(CHUNK_TIER), Some(CHUNK_CELLS_PER_CHUNK));
        assert_eq!(cell_m(CHUNK_TIER), f64::from(CHUNK_CELL_M));
        assert_eq!(
            cells_per_chunk(8),
            None,
            "a cell larger than a chunk is not chunk-held"
        );
        assert_eq!(
            cells_per_chunk(4),
            Some(8),
            "finer than the far census, still chunk-held"
        );
    }
}
