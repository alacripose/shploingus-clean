//! Coordinates and the settled first-target scale.
//!
//! The scale contract (C-6) is deliberately written as constants rather than as
//! prose in a document, because a scale contract that lives in prose is a scale
//! contract nobody can test:
//!
//! * **voxel** = 1 metre,
//! * **chunk** = 32³ voxels,
//! * **coordinates** = signed 32-bit,
//! * **first playable band** = z ∈ [−16, +47], and *not* the long-term boundary.
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

    /// True when this voxel is inside the first playable band (C-6).
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

    #[test]
    fn a_local_offset_outside_the_chunk_is_refused() {
        assert_eq!(LocalPos::new(0, 0, 31).unwrap().z, 31);
        let r = LocalPos::new(0, 0, 32).unwrap_err();
        assert_eq!(r.kind(), RefusalKind::IndexOutOfRange);
    }
}
