//! Coordinates and the settled first-target scale.
//!
//! The scale contract (C-6) is deliberately written as constants rather than as
//! prose in a document, because a scale contract that lives in prose is a scale
//! contract nobody can test:
//!
//! * **voxel** = 1 metre,
//! * **chunk** = 32³ voxels,
//! * **coordinates** = signed 32-bit,
//! * **first playable band** = z ∈ [−16, +47] — the **starting column**, not a world
//!   height.
//!
//! **The vertical extent is open, and that is a ruling (2026-10-10).** The band above was
//! never a decision about how tall the world is; it is where the first playable column
//! begins. Chunks are cubic and streamed in three axes ([`ChunkWindow::vertical_chunks`]
//! is a *budget*, not a ceiling), so nothing here caps how high the world may reach — which
//! is what lets an atmosphere eight times shorter than Earth's still stand kilometres tall,
//! and later lets a mountain do the same. [`VoxelPos::in_playable_band`] is therefore
//! advisory: a spawn helper, enforced nowhere.
//!
//! [`ChunkWindow::vertical_chunks`]: https://docs.rs/voxel-work
//!
//! Chunk and local coordinates use **Euclidean** division and remainder. Plain
//! `/` and `%` in Rust truncate toward zero, which puts voxel `(-1, 0, 0)` in
//! chunk `0` instead of chunk `-1` and folds two chunks' worth of world onto one
//! origin. That mistake is invisible at the origin and catastrophic away from it,
//! so it is a test here.

use crate::refusal::{Refusal, RefusalKind, Refused};

/// Voxels per chunk edge.
pub const CHUNK_SIDE: i32 = 32;
/// Voxels per chunk, as a count.
pub const CHUNK_VOLUME: i64 = (CHUNK_SIDE as i64) * (CHUNK_SIDE as i64) * (CHUNK_SIDE as i64);

/// Subvoxels per voxel edge — the **composition** resolution (C-2/APPEARANCE §2).
/// Stored: 64 cells per voxel.
pub const SUBVOXEL_SIDE: u32 = 4;
/// Subvoxels per voxel, as a count.
pub const SUBVOXEL_COUNT: usize = (SUBVOXEL_SIDE as usize).pow(3);

/// Material detail samples per subvoxel edge — the **detailing** resolution
/// (APPEARANCE §2). **Addressed, never stored.**
///
/// The base the record calls **256**, because one subvoxel *face* carries
/// 16×16 = **256** samples: **16×16×16** within the subvoxel, exactly as the
/// composition base is 4×4×4 within the voxel. The user corrected this from the
/// earlier reading of 256-per-axis on 2026-10-05 (*"256 was supposed to be 16x16x16
/// like the 64 base was 4x4x4"*); the lattice's own cell is therefore 15.6 mm and
/// not 1 mm.
pub const DETAIL_SIDE: u32 = 16;

/// The levels a cell's fill is resolved to — the **fluid** interface's own scale, in
/// 1/`FILL_SIDE`ths of a cell's pore volume (`voxel-shapers::fluidics::FILL_LEVELS`).
///
/// This is **not** the detail lattice, and the two shared one constant (`DETAIL_SIDE`)
/// until 2026-10-05, when the user corrected the detail base to 16 per subvoxel edge
/// and ruled where fluids belong: *"it's the level that the water and fluids should
/// run at, so it can animate down to as small as individual 32x pixels (not 256x
/// because it'd waste performance ESPECIALLY because of the transparency issues)."*
///
/// **And the scale is 32 per block axis, not 64 (C-164).** The same eye-check that
/// floored the drawn pattern ruled the whole visible scale: *"I like the 32x per side
/// of block, so if there's stuff happening below that, then I don't want it."* So a
/// block's fill is resolved at four subvoxels of **eight** — **32** levels, one per
/// 31.25 mm — which is exactly the drawn pattern's finest cell.
pub const FILL_SIDE: u32 = 32;

/// The lowest voxel z in the first playable band (C-6).
pub const BAND_MIN_Z: i32 = -16;
/// The highest voxel z in the first playable band (C-6). One past the last, so
/// `BAND_MIN_Z..BAND_MAX_Z` is the half-open range.
pub const BAND_MAX_Z: i32 = 48;

/// A voxel position in world space. Signed, so the world is unbounded in
/// principle and bounded only by the coordinate type.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct VoxelPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

/// A chunk position. One chunk is [`CHUNK_SIDE`]³ voxels.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct ChunkPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

/// A voxel's offset inside its chunk. Always in `0..CHUNK_SIDE`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct LocalPos {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

impl VoxelPos {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        VoxelPos { x, y, z }
    }

    /// The chunk this voxel belongs to. Euclidean, so negative coordinates
    /// behave.
    pub const fn chunk(&self) -> ChunkPos {
        ChunkPos {
            x: self.x.div_euclid(CHUNK_SIDE),
            y: self.y.div_euclid(CHUNK_SIDE),
            z: self.z.div_euclid(CHUNK_SIDE),
        }
    }

    /// This voxel's offset inside its chunk.
    pub const fn local(&self) -> LocalPos {
        LocalPos {
            x: self.x.rem_euclid(CHUNK_SIDE) as u32,
            y: self.y.rem_euclid(CHUNK_SIDE) as u32,
            z: self.z.rem_euclid(CHUNK_SIDE) as u32,
        }
    }

    /// **The voxel a world point in metres stands in** — the one address law the light's cells
    /// share, and the reason it lives here rather than in the crate that reads it.
    ///
    /// A cell of the world's light is a **voxel**: `floor(p)` per axis, in metres, with the
    /// scale contract above as the whole of the definition. The scene's corner rig, the field's
    /// key and the measurer's addresses are all this function, so a marched cell's light is the
    /// light the corner that named it receives. The draft this replaced had the scene asking at
    /// subvoxel grain (`floor(p / 0.25 m)`) and the field reading those integers as metres, which
    /// put every cell four chunks from the corner's own and measured **zero** of them.
    ///
    /// `None` for a point with a non-finite axis: a NaN corner is a fact about a broken quad and
    /// not a cell, so it reads as unmeasured and the rig lights it. A *finite* point outside the
    /// coordinate contract is clamped to the representable end rather than refused — the clamp is
    /// what keeps the cast from being a saturating conversion whose result depends on the cast.
    pub fn of_point(point_m: [f64; 3]) -> Option<Self> {
        let mut out = [0i32; 3];
        for axis in 0..3 {
            let v = point_m[axis].floor();
            if !v.is_finite() {
                return None;
            }
            out[axis] = v.clamp(i32::MIN as f64 + 1.0, i32::MAX as f64 - 1.0) as i32;
        }
        Some(VoxelPos { x: out[0], y: out[1], z: out[2] })
    }

    /// The centre of this voxel, in metres — **the point a shadow ray leaves from** when the
    /// cell's own light is measured.
    ///
    /// Half a metre in from the cell's low face: the cell is the unit the march charges matter
    /// against, so the ray starts at its middle rather than on a face two neighbours share.
    pub fn centre_m(&self) -> [f64; 3] {
        [
            self.x as f64 + 0.5,
            self.y as f64 + 0.5,
            self.z as f64 + 0.5,
        ]
    }

    /// True when this voxel is inside the first playable band (C-6) — the **starting
    /// column**, not a world height.
    ///
    /// **Advisory, and it is enforced nowhere**: the world's vertical extent is open
    /// (`coord`'s module doc), cubic chunks stream in three axes, and this predicate's only
    /// callers are its own tests. Use it to ask "is this a sensible spawn height", never to
    /// refuse a voxel.
    pub const fn in_playable_band(&self) -> bool {
        self.z >= BAND_MIN_Z && self.z < BAND_MAX_Z
    }

    /// Refuse a coordinate that cannot round-trip through a signed 32-bit index
    /// pair — the M19 evidence line, one layer down.
    pub fn checked_add(self, dx: i32, dy: i32, dz: i32) -> Refused<Self> {
        let x = self.x.checked_add(dx);
        let y = self.y.checked_add(dy);
        let z = self.z.checked_add(dz);
        match (x, y, z) {
            (Some(x), Some(y), Some(z)) => Ok(VoxelPos { x, y, z }),
            _ => Err(Refusal::owned(
                RefusalKind::CoordinateOutOfRange,
                format!("({}, {}, {}) + ({dx}, {dy}, {dz}) overflows i32", self.x, self.y, self.z),
            )),
        }
    }

    /// Manhattan distance, as an exact integer. Never a float.
    pub const fn manhattan(self, other: VoxelPos) -> i64 {
        (self.x as i64 - other.x as i64).abs()
            + (self.y as i64 - other.y as i64).abs()
            + (self.z as i64 - other.z as i64).abs()
    }
}

impl ChunkPos {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        ChunkPos { x, y, z }
    }

    /// The voxel at the chunk's low corner.
    pub const fn origin(&self) -> VoxelPos {
        VoxelPos {
            x: self.x * CHUNK_SIDE,
            y: self.y * CHUNK_SIDE,
            z: self.z * CHUNK_SIDE,
        }
    }

    /// The voxel containing `local`, in this chunk.
    pub const fn voxel_at(&self, local: LocalPos) -> VoxelPos {
        let o = self.origin();
        VoxelPos {
            x: o.x + local.x as i32,
            y: o.y + local.y as i32,
            z: o.z + local.z as i32,
        }
    }
}

impl LocalPos {
    /// Refuse an offset outside the chunk.
    pub const fn new(x: u32, y: u32, z: u32) -> Refused<Self> {
        if x < CHUNK_SIDE as u32 && y < CHUNK_SIDE as u32 && z < CHUNK_SIDE as u32 {
            Ok(LocalPos { x, y, z })
        } else {
            Err(Refusal::new(
                RefusalKind::IndexOutOfRange,
                "local offset outside 0..32",
            ))
        }
    }
}

/// Spread one axis' low 16 bits to every third bit: the software half of a
/// Morton (Z-order) code, after the `zorder` crate's own `index_of`. A loop
/// rather than magic masks, so the bit placement is read directly.
const fn spread3(v: u32) -> u64 {
    let mut out = 0u64;
    let mut i = 0;
    while i < 16 {
        if (v >> i) & 1 == 1 {
            out |= 1u64 << (3 * i);
        }
        i += 1;
    }
    out
}

/// Gather every third bit back into one axis: the inverse of [`spread3`].
const fn gather3(v: u64) -> u32 {
    let mut out = 0u32;
    let mut i = 0;
    while i < 16 {
        if (v >> (3 * i)) & 1 == 1 {
            out |= 1u32 << i;
        }
        i += 1;
    }
    out
}

/// A 3D cell address as one locality-preserving 1D code: Morton (Z-order)
/// order, with `x` in bits `0, 3, 6, …`, `y` in `1, 4, 7, …`, `z` in
/// `2, 5, 8, …` — the `zorder` crate's own bit layout, implemented here so the
/// order is available offline (the crate joins when the network does).
///
/// Axes are `u16`: callers with signed world coordinates bias first (the light
/// field marches chunk-local `0..32`, which needs no bias). Only the low 16
/// bits of each axis enter the code; anything wider must be split by the
/// caller rather than silently truncated.
///
/// The named consumer is the light march queue (`voxel-client/src/field.rs`):
/// unmeasured cells marched in Morton order fill the frame as a spatially
/// coherent sweep rather than in mesh order, so the measured/rig frontier
/// moves like a scan instead of churning. Buffer layout (records, origins) is
/// the second candidate; both stay follow-ups until the frame numbers pick
/// one.
pub const fn morton3(x: u16, y: u16, z: u16) -> u64 {
    spread3(x as u32) | (spread3(y as u32) << 1) | (spread3(z as u32) << 2)
}

/// The cell address a Morton code names: the inverse of [`morton3`].
pub const fn morton3_of(code: u64) -> [u16; 3] {
    [
        gather3(code) as u16,
        gather3(code >> 1) as u16,
        gather3(code >> 2) as u16,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_are_the_settled_contract() {
        assert_eq!(CHUNK_SIDE, 32);
        assert_eq!(CHUNK_VOLUME, 32_768);
        assert_eq!(SUBVOXEL_SIDE, 4);
        assert_eq!(SUBVOXEL_COUNT, 64);
        // The two bases, by the names the record uses: the composition base is
        // 4×4×4 (64) per voxel, the detail base is 16×16×16 (256 per face) per
        // subvoxel. A fluid's fill levels are neither (`FILL_SIDE`).
        assert_eq!(DETAIL_SIDE, 16);
        assert_eq!(DETAIL_SIDE * DETAIL_SIDE, 256);
        // A fluid's fill is resolved at the drawn scale — 32 samples per block axis,
        // four subvoxels of eight — and not at the addressed detail base (C-164).
        assert_eq!(FILL_SIDE, SUBVOXEL_SIDE * 8);
        assert_ne!(DETAIL_SIDE, FILL_SIDE);
    }

    #[test]
    fn negative_voxels_land_in_negative_chunks() {
        // Truncating division would put this in chunk 0. Euclidean puts it in -1.
        let v = VoxelPos::new(-1, -1, -1);
        assert_eq!(v.chunk(), ChunkPos::new(-1, -1, -1));
        assert_eq!(v.local(), LocalPos { x: 31, y: 31, z: 31 });
    }

    #[test]
    fn chunk_and_local_round_trip_across_the_origin() {
        for x in [-33, -32, -31, -1, 0, 1, 31, 32, 33, 1_000_000] {
            let v = VoxelPos::new(x, x, x);
            let back = v.chunk().voxel_at(v.local());
            assert_eq!(back, v, "round trip failed at {x}");
        }
    }

    #[test]
    fn morton_codes_pack_x_y_z_at_bits_0_1_2() {
        // The `zorder` layout: x in bits 0, 3, 6, …, y in 1, 4, 7, …, z in
        // 2, 5, 8, … — so unit steps read 1, 2, 4 and the corner reads 7.
        assert_eq!(morton3(1, 0, 0), 0b001);
        assert_eq!(morton3(0, 1, 0), 0b010);
        assert_eq!(morton3(0, 0, 1), 0b100);
        assert_eq!(morton3(1, 1, 1), 0b111);
        assert_eq!(morton3(2, 0, 0), 0b1_000);
        assert_eq!(morton3(0, 0, 0), 0);
        assert_eq!(morton3(0xFFFF, 0xFFFF, 0xFFFF), 0xFFFFFFFFFFFF);
    }

    #[test]
    fn morton_codes_round_trip() {
        for [x, y, z] in [
            [0, 0, 0],
            [1, 0, 0],
            [0, 31, 17],
            [31, 31, 31],
            [5, 129, 30000],
            [0xFFFF, 0xFFFF, 0xFFFF],
        ] {
            assert_eq!(morton3_of(morton3(x, y, z)), [x, y, z]);
        }
    }

    #[test]
    fn morton_order_keeps_neighbours_close() {
        // Adjacent cells along one axis land a bounded step apart in code
        // space — the locality a march queue spends. A stride across the whole
        // domain is orders of magnitude further.
        let here = morton3(10, 10, 10);
        let neighbours = [
            morton3(11, 10, 10).abs_diff(here),
            morton3(10, 11, 10).abs_diff(here),
            morton3(10, 10, 11).abs_diff(here),
        ];
        let far = morton3(500, 500, 500).abs_diff(here);
        for step in neighbours {
            assert!(step < far, "neighbour step {step} is not local against {far}");
        }
    }

    #[test]
    fn the_playable_band_is_half_open() {
        assert!(VoxelPos::new(0, 0, BAND_MIN_Z).in_playable_band());
        assert!(VoxelPos::new(0, 0, BAND_MAX_Z - 1).in_playable_band());
        assert!(!VoxelPos::new(0, 0, BAND_MAX_Z).in_playable_band());
        assert!(!VoxelPos::new(0, 0, BAND_MIN_Z - 1).in_playable_band());
    }

    #[test]
    fn an_overflowing_coordinate_is_refused_by_name() {
        let r = VoxelPos::new(i32::MAX, 0, 0)
            .checked_add(1, 0, 0)
            .unwrap_err();
        assert_eq!(r.kind(), RefusalKind::CoordinateOutOfRange);
        assert!(r.detail().contains("overflows i32"));
    }

    #[test]
    fn manhattan_distance_is_integer_and_symmetric() {
        let a = VoxelPos::new(-3, 4, 0);
        let b = VoxelPos::new(2, -1, 5);
        assert_eq!(a.manhattan(b), 5 + 5 + 5);
        assert_eq!(a.manhattan(b), b.manhattan(a));
    }

    /// **The address law, held to its own words.** A cell is the voxel a point stands in, its
    /// centre is inside it, and the chunk it names is the chunk that holds the point — the one
    /// property the frame's corner rig, the field's key and the measurer's queue all spend.
    ///
    /// The regression this refuses is the draft's: the scene asked at `floor(p / 0.25 m)` and the
    /// field read those integers as metres, so every cell named here would have been four times
    /// the point's own address, in a chunk four times away, with no world under it. The last
    /// assertion is the sharp one — a subvoxel address and a metre address of the same point are
    /// **not** the same cell, and only one of them is inside the point's own voxel.
    #[test]
    fn a_cell_is_the_voxel_a_point_stands_in() {
        for (point, cell) in [
            ([0.0, 0.0, 0.0], [0, 0, 0]),
            ([0.25, 0.25, 0.25], [0, 0, 0]),
            ([31.999, 0.0, 0.0], [31, 0, 0]),
            ([32.0, 0.0, 0.0], [32, 0, 0]),
            ([-0.001, -0.001, 0.0], [-1, -1, 0]),
            ([-32.0, 7.5, 63.0], [-32, 7, 63]),
            ([1234.9, -9.0, -1.0], [1234, -9, -1]),
        ] {
            let named = VoxelPos::of_point(point).expect("a finite point has a cell");
            assert_eq!([named.x, named.y, named.z], cell, "{point:?} named {named:?}");
            // The centre is inside the cell, and the chunk is the point's own.
            let centre = named.centre_m();
            for axis in 0..3 {
                assert!(
                    centre[axis] > cell[axis] as f64 && centre[axis] < (cell[axis] + 1) as f64,
                    "a centre outside its own cell on axis {axis}: {centre:?}"
                );
            }
            assert_eq!(
                named.chunk(),
                VoxelPos::of_point(centre).expect("a centre").chunk(),
                "a cell and its own centre named different chunks"
            );
        }
        // **A subvoxel address is not a metre address.** `floor(2.0 / 0.25)` is 8, which is a
        // real cell of this world and not the one the point at 2.0 m stands in.
        assert_ne!(
            VoxelPos::of_point([2.0, 0.0, 0.0]),
            Some(VoxelPos::new(8, 0, 0)),
            "the subvoxel grain came back as the light's address law"
        );
        // A non-finite axis is no cell at all; a finite one off the end is clamped, so the cast
        // is deterministic rather than saturating.
        assert_eq!(VoxelPos::of_point([f64::NAN, 0.0, 0.0]), None);
        assert_eq!(VoxelPos::of_point([0.0, f64::INFINITY, 0.0]), None);
        assert_eq!(
            VoxelPos::of_point([1.0e30, 0.0, 0.0]),
            Some(VoxelPos::new(i32::MAX - 1, 0, 0))
        );
        assert_eq!(
            VoxelPos::of_point([-1.0e30, 0.0, 0.0]),
            Some(VoxelPos::new(i32::MIN + 1, 0, 0))
        );
    }

    #[test]
    fn a_local_offset_outside_the_chunk_is_refused() {
        assert_eq!(LocalPos::new(0, 0, 31).unwrap().z, 31);
        let r = LocalPos::new(0, 0, 32).unwrap_err();
        assert_eq!(r.kind(), RefusalKind::IndexOutOfRange);
    }
}
