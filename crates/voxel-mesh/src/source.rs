//! Where voxels come from, and what lies just outside a chunk.
//!
//! The mesher does not know about worlds, generators or disks. It takes a
//! [`Chunk`](crate::Chunk) and, optionally, a [`ChunkNeighborhood`]. That is the
//! whole interface, and it is what lets this crate be tested with a hand-built
//! four-voxel chunk and then driven by a full shaper pipeline without a line
//! changing in either.
//!
//! # Two traits, because they answer two different questions
//!
//! * [`VoxelSource`] — *"what is in this chunk?"* Answered for a whole chunk at
//!   once, because a generated world computes a **column** at a time (surface,
//!   soil, lithology, caves) and asking it voxel-by-voxel throws that away.
//! * [`ChunkNeighborhood`] — *"what is in the voxel just past my edge?"*
//!   Answered for coordinates **outside** `0..32`, because a face on a chunk's
//!   boundary is only invisible if the chunk next door agrees.

use std::collections::BTreeMap;

use voxel_model::{ChunkPos, LocalPos, Refused, VoxelShape, CHUNK_SIDE};

use crate::octree::SparseVoxelOctree;

/// Something that can produce a chunk's voxels — a generator, a store, a test.
pub trait VoxelSource {
    /// The shape at one voxel.
    fn voxel(&self, chunk: ChunkPos, local: LocalPos) -> Refused<VoxelShape>;

    /// Build a whole chunk.
    ///
    /// The default walks all 32,768 voxels through [`Self::voxel`], which is
    /// **correct but not fast** — and deliberately the default rather than the
    /// requirement, so that implementing the trait is one method and the fast
    /// path is an override a source opts into. A generated world overrides it to
    /// work column-by-column; nothing else has to change.
    fn chunk(&self, pos: ChunkPos) -> Refused<SparseVoxelOctree> {
        let mut tree = SparseVoxelOctree::new();
        for z in 0..CHUNK_SIDE as u32 {
            for y in 0..CHUNK_SIDE as u32 {
                for x in 0..CHUNK_SIDE as u32 {
                    let local = LocalPos { x, y, z };
                    let shape = self.voxel(pos, local)?;
                    if !shape.is_empty() {
                        tree.set_voxel(local, shape);
                    }
                }
            }
        }
        Ok(tree)
    }
}

/// The voxels immediately around a chunk, for culling the chunk's own boundary.
///
/// `local` **may be outside `0..CHUNK_SIDE`** — `-1` and `CHUNK_SIDE` are the
/// coordinates the mesher reads. A neighbourhood that has no chunk there cannot
/// answer, and refusing is the right answer: the alternative is claiming air,
/// which would punch a halo of visible faces around every loaded chunk and look
/// exactly like a bug in the mesher.
pub trait ChunkNeighborhood {
    fn shape(&self, local: [i32; 3]) -> Refused<VoxelShape>;
}

/// A neighbourhood of nothing. The honest answer for an isolated chunk.
///
/// It emits every boundary face, which is correct: an isolated chunk really does
/// have nothing on the other side.
#[derive(Clone, Copy, Debug, Default)]
pub struct AllAir;

impl ChunkNeighborhood for AllAir {
    fn shape(&self, _local: [i32; 3]) -> Refused<VoxelShape> {
        Ok(VoxelShape::empty())
    }
}

/// Any closure can be a neighbourhood, so a caller can inline one.
impl<F> ChunkNeighborhood for F
where
    F: Fn([i32; 3]) -> Refused<VoxelShape>,
{
    fn shape(&self, local: [i32; 3]) -> Refused<VoxelShape> {
        self(local)
    }
}

/// A sparse in-memory set of voxels, relative to one chunk.
///
/// This is the shape an **authored** world takes: a list of places somebody put
/// something, rather than a function. It is `BTreeMap` and not `HashMap`
/// because iteration order must be deterministic (F1), and because a map that
/// enumerates in a fixed order is one you can digest.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct VoxelGrid {
    chunk: ChunkPos,
    cells: BTreeMap<[i32; 3], VoxelShape>,
}

impl VoxelGrid {
    pub fn new(chunk: ChunkPos) -> Self {
        VoxelGrid { chunk, cells: BTreeMap::new() }
    }

    /// The chunk this grid describes.
    ///
    /// Named `position` rather than `chunk` because [`VoxelSource::chunk`] is the
    /// other thing a grid can be asked, and two methods called `chunk` on one
    /// type would make an inherent method shadow a trait method.
    pub fn position(&self) -> ChunkPos {
        self.chunk
    }

    /// Put a shape down. An empty shape removes the cell, so a grid never stores
    /// a voxel that holds nothing.
    pub fn set(&mut self, local: [i32; 3], shape: VoxelShape) {
        if shape.is_empty() {
            self.cells.remove(&local);
        } else {
            self.cells.insert(local, shape);
        }
    }

    pub fn get(&self, local: [i32; 3]) -> VoxelShape {
        self.cells.get(&local).copied().unwrap_or_else(VoxelShape::empty)
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// Every cell, in coordinate order.
    pub fn cells(&self) -> impl Iterator<Item = (&[i32; 3], &VoxelShape)> {
        self.cells.iter()
    }

    /// Adopt this grid as a whole-chunk tree.
    pub fn tree(&self) -> SparseVoxelOctree {
        let mut tree = SparseVoxelOctree::new();
        for (local, shape) in &self.cells {
            if let Ok(pos) = crate::octree::local_from(*local) {
                tree.set_voxel(pos, *shape);
            }
        }
        tree
    }
}

impl VoxelSource for VoxelGrid {
    fn voxel(&self, _chunk: ChunkPos, local: LocalPos) -> Refused<VoxelShape> {
        Ok(self.get([local.x as i32, local.y as i32, local.z as i32]))
    }

    fn chunk(&self, _pos: ChunkPos) -> Refused<SparseVoxelOctree> {
        Ok(self.tree())
    }
}

impl ChunkNeighborhood for VoxelGrid {
    fn shape(&self, local: [i32; 3]) -> Refused<VoxelShape> {
        Ok(self.get(local))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_model::{MaterialId, CHUNK_VOLUME};

    #[test]
    fn an_all_air_neighbourhood_says_air_and_never_refuses() {
        let n = AllAir;
        assert_eq!(n.shape([-1, 0, 0]).unwrap(), VoxelShape::empty());
        assert_eq!(n.shape([32, 32, 32]).unwrap(), VoxelShape::empty());
    }

    #[test]
    fn a_grid_sources_a_chunk_and_out_of_range_coordinates_read_as_air() {
        let mut g = VoxelGrid::new(ChunkPos::new(0, 0, 0));
        g.set([3, 4, 5], VoxelShape::solid(MaterialId(6)));
        let tree = g.chunk(ChunkPos::new(0, 0, 0)).unwrap();
        assert_eq!(tree.solid_count(), 1);
        // The mesher's neighbour read reaches one voxel out; the grid answers.
        assert_eq!(g.shape([-1, 0, 0]).unwrap(), VoxelShape::empty());
        assert_eq!(g.shape([3, 4, 5]).unwrap(), VoxelShape::solid(MaterialId(6)));
    }

    #[test]
    fn setting_an_empty_shape_removes_the_cell() {
        let mut g = VoxelGrid::new(ChunkPos::new(0, 0, 0));
        g.set([1, 1, 1], VoxelShape::solid(MaterialId(6)));
        assert_eq!(g.len(), 1);
        g.set([1, 1, 1], VoxelShape::empty());
        assert!(g.is_empty());
    }

    #[test]
    fn the_default_chunk_walks_every_voxel() {
        // The slow path is the default, and it is measured: 32,768 calls.
        struct CountTo {
            wanted: [i32; 3],
            calls: std::cell::Cell<u64>,
        }
        impl VoxelSource for CountTo {
            fn voxel(&self, _c: ChunkPos, local: LocalPos) -> Refused<VoxelShape> {
                self.calls.set(self.calls.get() + 1);
                if [local.x as i32, local.y as i32, local.z as i32] == self.wanted {
                    Ok(VoxelShape::solid(MaterialId(6)))
                } else {
                    Ok(VoxelShape::empty())
                }
            }
        }
        let s = CountTo { wanted: [31, 31, 31], calls: std::cell::Cell::new(0) };
        let tree = s.chunk(ChunkPos::new(0, 0, 0)).unwrap();
        assert_eq!(s.calls.get(), CHUNK_VOLUME as u64);
        assert_eq!(tree.solid_count(), 1);
    }
}
