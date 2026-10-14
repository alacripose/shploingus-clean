//! **Coordinates and placement — the world's noun table, with one algebra.**
//!
//! Derived from `docs/DESIGN-SPEC.md` §1.1 (voxel 1 m, subvoxel 4×4×4 stored, detail 256× addressed),
//! §1.2 (chunk 32³, signed 32-bit coordinates, the **20-level** ladder) and §5 (the scale contract).
//! `LEDGER.md` **C-10** is the correction this module pins: the record's own justification for the
//! ladder treated `1 048 576` **metres** as `1.05` **kilometres**, so the correct reading is
//! implemented and the wrong figure is asserted **not** to reproduce.
//!
//! # One algebra, and the seam it closes
//!
//! A chunk, a region and a voxel are three views of one coordinate. They are defined here **once**:
//! the chunk mapping is Euclidean (so `x = -1` is chunk `-1`, not chunk `0`), a region key is the
//! same idea at a ladder level, and the **face relations** — which chunks a coordinate's change
//! reaches — are computed from the within-chunk offset rather than restated by the streamer. Two
//! statements of "which chunks does this voxel touch" is two answers to one question; this is the
//! only one.
//!
//! `z` is **up** (`LEDGER.md` C-138).

use crate::refusal::RefusalKind;

/// Voxels per chunk edge. The settled first-target contract.
pub const CHUNK_SIDE: i32 = 32;
/// Subvoxel cells per voxel axis: the **stored** lattice.
pub const SUBVOXEL_AXIS: u8 = 4;
/// Detail samples per subvoxel axis: the **addressed** lattice, never stored dense.
pub const DETAIL_AXIS: u16 = 256;
/// Levels of the regional field ladder.
pub const LADDER_LEVELS: u8 = 20;
/// The voxel's side, in metres. 1 m, and the unit of collision, ownership and the ledger.
pub const VOXEL_SIDE_M: f64 = 1.0;

/// A voxel coordinate. Signed 32-bit, `z` up.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VoxelPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl VoxelPos {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        VoxelPos { x, y, z }
    }

    /// The chunk this coordinate falls in. **Euclidean**, so the sign is preserved.
    pub fn chunk(self) -> ChunkKey {
        ChunkKey {
            x: self.x.div_euclid(CHUNK_SIDE),
            y: self.y.div_euclid(CHUNK_SIDE),
            z: self.z.div_euclid(CHUNK_SIDE),
        }
    }

    /// The offset within its chunk, `0..32` on each axis.
    pub fn within_chunk(self) -> WithinChunk {
        WithinChunk {
            x: self.x.rem_euclid(CHUNK_SIDE) as u8,
            y: self.y.rem_euclid(CHUNK_SIDE) as u8,
            z: self.z.rem_euclid(CHUNK_SIDE) as u8,
        }
    }

    /// **Which chunks a change at this coordinate reaches.** The containing chunk, plus one
    /// neighbour per chunk-boundary face the coordinate sits on — at most three, and **never** a
    /// diagonal, because a face is never determined by a corner alone (`docs/CLEAN-ROOM-PLAN-2026-10-11.md`
    /// §2 U4's pinned finding). A face that leads past the coordinate system refuses: the chunk
    /// beyond the last representable coordinate cannot be mapped back to a voxel, so it is not a
    /// chunk of this world — never a key whose `origin()` overflows.
    pub fn chunks_reached_by(self) -> Result<Vec<ChunkKey>, RefusalKind> {
        let base = self.chunk();
        let offset = self.within_chunk();
        let mut out = vec![base];
        let axes = [
            (offset.x, 0usize),
            (offset.y, 1usize),
            (offset.z, 2usize),
        ];
        for (local, axis) in axes {
            let neighbour = if local == 0 {
                Some(base.shift(axis, -1)?)
            } else if local as i32 == CHUNK_SIDE - 1 {
                Some(base.shift(axis, 1)?)
            } else {
                None
            };
            if let Some(key) = neighbour {
                // The world's edge: a reached chunk must be expressible as a voxel origin.
                key.origin()?;
                out.push(key);
            }
        }
        Ok(out)
    }
}

/// An offset within a chunk, `0..32` on each axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WithinChunk {
    pub x: u8,
    pub y: u8,
    pub z: u8,
}

impl WithinChunk {
    pub fn new(x: u8, y: u8, z: u8) -> Result<Self, RefusalKind> {
        if [x, y, z].iter().any(|v| *v as i32 >= CHUNK_SIDE) {
            return Err(RefusalKind::ChunkOffsetOutOfRange);
        }
        Ok(WithinChunk { x, y, z })
    }

    /// The flat cell index within the chunk, `0..32768`.
    pub fn flat(self) -> u32 {
        self.x as u32 + CHUNK_SIDE as u32 * (self.y as u32 + CHUNK_SIDE as u32 * self.z as u32)
    }
}

/// The chunk's signed integer key: the unit of ownership and of streaming.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChunkKey {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl ChunkKey {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        ChunkKey { x, y, z }
    }

    /// The coordinate at a chunk's own minimum corner. Refuses on overflow.
    pub fn origin(self) -> Result<VoxelPos, RefusalKind> {
        Ok(VoxelPos {
            x: self
                .x
                .checked_mul(CHUNK_SIDE)
                .ok_or(RefusalKind::CoordinateOverflow)?,
            y: self
                .y
                .checked_mul(CHUNK_SIDE)
                .ok_or(RefusalKind::CoordinateOverflow)?,
            z: self
                .z
                .checked_mul(CHUNK_SIDE)
                .ok_or(RefusalKind::CoordinateOverflow)?,
        })
    }

    /// Recompose a coordinate from this chunk and an offset. The inverse of [`VoxelPos::chunk`] +
    /// [`VoxelPos::within_chunk`].
    pub fn voxel(self, offset: WithinChunk) -> Result<VoxelPos, RefusalKind> {
        let origin = self.origin()?;
        Ok(VoxelPos {
            x: origin
                .x
                .checked_add(offset.x as i32)
                .ok_or(RefusalKind::CoordinateOverflow)?,
            y: origin
                .y
                .checked_add(offset.y as i32)
                .ok_or(RefusalKind::CoordinateOverflow)?,
            z: origin
                .z
                .checked_add(offset.z as i32)
                .ok_or(RefusalKind::CoordinateOverflow)?,
        })
    }

    /// Whether this chunk contains a coordinate.
    pub fn contains(self, pos: VoxelPos) -> bool {
        pos.chunk() == self
    }

    /// The neighbour one chunk along `axis` (`0 = x, 1 = y, 2 = z`) in `direction` (`±1`). A zero
    /// direction, or a step past the signed range, is refused.
    pub fn shift(self, axis: usize, direction: i32) -> Result<ChunkKey, RefusalKind> {
        if direction == 0 || direction.abs() != 1 {
            return Err(RefusalKind::CoordinateOverflow);
        }
        let mut key = self;
        let field = match axis {
            0 => &mut key.x,
            1 => &mut key.y,
            2 => &mut key.z,
            _ => return Err(RefusalKind::CoordinateOverflow),
        };
        *field = field
            .checked_add(direction)
            .ok_or(RefusalKind::CoordinateOverflow)?;
        Ok(key)
    }

    /// The six face-adjacent neighbours.
    pub fn face_neighbors(self) -> Result<[ChunkKey; 6], RefusalKind> {
        Ok([
            self.shift(0, -1)?,
            self.shift(0, 1)?,
            self.shift(1, -1)?,
            self.shift(1, 1)?,
            self.shift(2, -1)?,
            self.shift(2, 1)?,
        ])
    }
}

/// A cell in the stored 4×4×4 subvoxel lattice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubvoxelIndex {
    pub i: u8,
    pub j: u8,
    pub k: u8,
}

impl SubvoxelIndex {
    pub fn new(i: u8, j: u8, k: u8) -> Result<Self, RefusalKind> {
        if [i, j, k].iter().any(|v| *v >= SUBVOXEL_AXIS) {
            return Err(RefusalKind::SubvoxelOutOfRange);
        }
        Ok(SubvoxelIndex { i, j, k })
    }

    /// The cell index within the voxel, `0..64`.
    pub fn flat(self) -> u8 {
        self.i + SUBVOXEL_AXIS * (self.j + SUBVOXEL_AXIS * self.k)
    }

    /// The inverse of [`SubvoxelIndex::flat`].
    pub fn from_flat(cell: u8) -> Result<Self, RefusalKind> {
        if cell >= SUBVOXEL_AXIS * SUBVOXEL_AXIS * SUBVOXEL_AXIS {
            return Err(RefusalKind::SubvoxelOutOfRange);
        }
        let i = cell % SUBVOXEL_AXIS;
        let j = (cell / SUBVOXEL_AXIS) % SUBVOXEL_AXIS;
        let k = cell / (SUBVOXEL_AXIS * SUBVOXEL_AXIS);
        Ok(SubvoxelIndex { i, j, k })
    }
}

/// A sample in the addressed 256× detail lattice. **Never stored dense** — this is an *address*.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DetailIndex {
    pub i: u16,
    pub j: u16,
    pub k: u16,
}

impl DetailIndex {
    pub fn new(i: u16, j: u16, k: u16) -> Result<Self, RefusalKind> {
        if [i, j, k].iter().any(|v| *v >= DETAIL_AXIS) {
            return Err(RefusalKind::DetailOutOfRange);
        }
        Ok(DetailIndex { i, j, k })
    }

    /// The flat address, `0..16 777 216`.
    pub fn flat(self) -> u32 {
        self.i as u32 + DETAIL_AXIS as u32 * (self.j as u32 + DETAIL_AXIS as u32 * self.k as u32)
    }
}

/// The extent of a ladder level, in blocks. Level `L` spans `32 · 2^L`.
pub fn ladder_extent_blocks(level: u8) -> Result<u64, RefusalKind> {
    if level >= LADDER_LEVELS {
        return Err(RefusalKind::LadderOutOfRange);
    }
    Ok((CHUNK_SIDE as u64) << level)
}

/// A region key at a declared ladder level. Regions partition the world the same way chunks do, one
/// scale up.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RegionKey {
    pub level: u8,
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

impl RegionKey {
    pub fn of(pos: VoxelPos, level: u8) -> Result<Self, RefusalKind> {
        let extent = ladder_extent_blocks(level)? as i64;
        Ok(RegionKey {
            level,
            x: (pos.x as i64).div_euclid(extent),
            y: (pos.y as i64).div_euclid(extent),
            z: (pos.z as i64).div_euclid(extent),
        })
    }

    pub fn extent_blocks(&self) -> Result<u64, RefusalKind> {
        ladder_extent_blocks(self.level)
    }

    pub fn contains(&self, pos: VoxelPos) -> bool {
        RegionKey::of(pos, self.level).map(|k| k == *self).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinates_round_trip_across_the_origin() {
        let mut checked = 0u32;
        for x in -70..=70i32 {
            for y in -70..=70i32 {
                for z in -70..=70i32 {
                    let pos = VoxelPos::new(x, y, z);
                    let chunk = pos.chunk();
                    let offset = pos.within_chunk();
                    let back = chunk.voxel(offset).unwrap();
                    assert_eq!(back, pos, "round trip failed at {x},{y},{z}");
                    assert!(chunk.contains(pos));
                    checked += 1;
                }
            }
        }
        println!("coordinate round trip: {checked} coordinates checked exactly");
        assert_eq!(checked, 141 * 141 * 141);
    }

    #[test]
    fn a_negative_coordinate_lands_in_a_negative_chunk() {
        assert_eq!(VoxelPos::new(-1, 0, 0).chunk(), ChunkKey::new(-1, 0, 0));
        assert_eq!(VoxelPos::new(-1, 0, 0).within_chunk().x, 31);
        assert_eq!(VoxelPos::new(-32, 0, 0).chunk(), ChunkKey::new(-1, 0, 0));
        assert_eq!(VoxelPos::new(-33, 0, 0).chunk(), ChunkKey::new(-2, 0, 0));
        assert_eq!(VoxelPos::new(31, 0, 0).chunk(), ChunkKey::new(0, 0, 0));
        assert_eq!(VoxelPos::new(32, 0, 0).chunk(), ChunkKey::new(1, 0, 0));
    }

    #[test]
    fn a_subvoxel_or_detail_index_out_of_range_is_refused_by_name() {
        assert_eq!(
            SubvoxelIndex::new(4, 0, 0).unwrap_err(),
            RefusalKind::SubvoxelOutOfRange
        );
        assert_eq!(
            DetailIndex::new(0, 0, 256).unwrap_err(),
            RefusalKind::DetailOutOfRange
        );
        // All 64 cells are addressable and invertible.
        for cell in 0..64u8 {
            let index = SubvoxelIndex::from_flat(cell).unwrap();
            assert_eq!(index.flat(), cell);
        }
        assert!(SubvoxelIndex::from_flat(64).is_err());
        assert_eq!(DetailIndex::new(255, 255, 255).unwrap().flat(), 16_777_215);
    }

    #[test]
    fn the_ladder_arithmetic_is_the_corrected_one() {
        assert_eq!(ladder_extent_blocks(0).unwrap(), 32);
        assert_eq!(ladder_extent_blocks(19).unwrap(), 16_777_216);
        // The correction C-10: 16 777 216 metres is 16 777.216 km, and the record's "16.8 km" does
        // not reproduce.
        let km = ladder_extent_blocks(19).unwrap() as f64 / 1000.0;
        assert!((km - 16_777.216).abs() < 1e-6, "level 19 is {km} km");
        assert!((km - 16.8).abs() > 1000.0, "the wrong figure must not reproduce");
        // And the record's level-15 claim of "1.05 km" is likewise off by 1000.
        let km15 = ladder_extent_blocks(15).unwrap() as f64 / 1000.0;
        assert!((km15 - 1_048.576).abs() < 1e-6);
        assert!((km15 - 1.05).abs() > 100.0);
        // Past the ladder is refused.
        assert_eq!(
            ladder_extent_blocks(20).unwrap_err(),
            RefusalKind::LadderOutOfRange
        );
    }

    #[test]
    fn a_region_key_partitions_and_contains() {
        let level = 3u8; // 256 blocks
        let a = RegionKey::of(VoxelPos::new(0, 0, 0), level).unwrap();
        let b = RegionKey::of(VoxelPos::new(255, 255, 255), level).unwrap();
        let c = RegionKey::of(VoxelPos::new(256, 0, 0), level).unwrap();
        assert_eq!(a, b, "0..255 are one region at level 3");
        assert_ne!(a, c, "256 begins the next region along x");
        assert!(a.contains(VoxelPos::new(255, 255, 255)));
        assert!(!a.contains(VoxelPos::new(256, 0, 0)));
        // A negative coordinate is its own region, not region zero.
        let neg = RegionKey::of(VoxelPos::new(-1, 0, 0), level).unwrap();
        assert_eq!(neg.x, -1);
        assert_eq!(neg.extent_blocks().unwrap(), 256);
        assert_eq!(
            RegionKey::of(VoxelPos::new(0, 0, 0), 20).unwrap_err(),
            RefusalKind::LadderOutOfRange
        );
    }

    #[test]
    fn a_corner_voxel_reaches_three_chunks_and_no_diagonal() {
        // An interior voxel reaches only its own chunk.
        let middle = VoxelPos::new(5, 5, 5).chunks_reached_by().unwrap();
        assert_eq!(middle.len(), 1);
        assert_eq!(middle[0], ChunkKey::new(0, 0, 0));

        // A voxel on two faces (an edge) reaches its own chunk plus two neighbours: three, never
        // the diagonal's four.
        let edge = VoxelPos::new(0, 0, 5).chunks_reached_by().unwrap();
        assert_eq!(edge.len(), 3, "an edge voxel reaches three chunks: {edge:?}");
        assert!(edge.contains(&ChunkKey::new(0, 0, 0)));
        assert!(edge.contains(&ChunkKey::new(-1, 0, 0)));
        assert!(edge.contains(&ChunkKey::new(0, -1, 0)));
        // The diagonal neighbour is not among them.
        assert!(!edge.contains(&ChunkKey::new(-1, -1, 0)));

        // A voxel at the positive far face reaches the positive neighbour.
        let far = VoxelPos::new(31, 5, 5).chunks_reached_by().unwrap();
        assert!(far.contains(&ChunkKey::new(1, 0, 0)));

        // The six face neighbours are distinct and never diagonals.
        let neighbors = ChunkKey::new(0, 0, 0).face_neighbors().unwrap();
        assert_eq!(neighbors.len(), 6);
        assert!(neighbors.iter().all(|k| {
            let delta = (k.x.abs(), k.y.abs(), k.z.abs());
            delta == (1, 0, 0) || delta == (0, 1, 0) || delta == (0, 0, 1)
        }));
    }

    #[test]
    fn a_within_chunk_offset_out_of_range_is_refused() {
        assert!(WithinChunk::new(31, 31, 31).is_ok());
        assert_eq!(
            WithinChunk::new(32, 0, 0).unwrap_err(),
            RefusalKind::ChunkOffsetOutOfRange
        );
        // flat = x + 32*(y + 32*z): the top cell of the z column is 32*32*31.
        assert_eq!(WithinChunk::new(0, 0, 31).unwrap().flat(), 31 * 1024);
        assert_eq!(WithinChunk::new(1, 2, 3).unwrap().flat(), 1 + 32 * (2 + 32 * 3));
    }

    #[test]
    fn the_world_edge_reaches_no_chunk_that_cannot_be_mapped() {
        // At the last and first representable coordinates the outward face has no chunk of this
        // world: the query refuses, rather than returning a key whose origin cannot be expressed.
        assert_eq!(
            VoxelPos::new(i32::MAX, 5, 5).chunks_reached_by().unwrap_err(),
            RefusalKind::CoordinateOverflow
        );
        assert_eq!(
            VoxelPos::new(i32::MIN, 5, 5).chunks_reached_by().unwrap_err(),
            RefusalKind::CoordinateOverflow
        );
        // One step inside, the same query succeeds and **every** reached chunk maps back.
        for pos in [VoxelPos::new(i32::MAX - 31, 5, 5), VoxelPos::new(i32::MIN + 32, 5, 5)] {
            let reached = pos.chunks_reached_by().unwrap();
            assert!(
                reached.iter().all(|key| key.origin().is_ok()),
                "{pos:?} reached an unmappable chunk: {reached:?}"
            );
        }
    }
}
