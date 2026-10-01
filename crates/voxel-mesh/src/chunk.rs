//! A chunk: a position, an octree, and whether what was drawn from it is stale.
//!
//! A chunk is deliberately **small in responsibility**. It does not know how to
//! mesh itself (the [`Mesher`](crate::Mesher) holds the scratch, and one mesher
//! serves many chunks on one worker), it does not know where its neighbours are
//! (the streaming layer does), and it does not keep a cached copy of its own
//! mesh. It holds voxels and it knows when they changed.
//!
//! # Editing is not regenerating
//!
//! [`Chunk::generate`] adopts a whole tree from a [`VoxelSource`] and marks the
//! chunk dirty. [`Chunk::set_voxel`] changes one voxel and marks it dirty. Both
//! end the same way, because the mesher neither knows nor needs to know where a
//! voxel came from: the world's own record of *why* a voxel changed is
//! [`Earthworks`](https://docs.rs/voxel-shapers), one layer up, and duplicating
//! that reasoning here would give the world two accounts of the same edit.

use voxel_model::{
    ChunkPos, Digest, LocalPos, MaterialId, Refused, SubvoxelIndex, VoxelShape,
};

use crate::octree::SparseVoxelOctree;
use crate::source::VoxelSource;

/// One chunk's voxels.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Chunk {
    /// Where the chunk is. Signed, so the world is unbounded in principle.
    pub pos: ChunkPos,
    tree: SparseVoxelOctree,
    dirty: bool,
}

impl Chunk {
    /// An empty chunk at `pos`, dirty: it has never been meshed.
    pub fn new(pos: ChunkPos) -> Self {
        Chunk { pos, tree: SparseVoxelOctree::new(), dirty: true }
    }

    /// A chunk that already holds `tree`.
    pub fn from_tree(pos: ChunkPos, tree: SparseVoxelOctree) -> Self {
        Chunk { pos, tree, dirty: true }
    }

    pub fn tree(&self) -> &SparseVoxelOctree {
        &self.tree
    }

    /// Adopt a whole chunk's voxels from a source — the generator, a store, a
    /// save file.
    pub fn generate(&mut self, source: &dyn VoxelSource) -> Refused<()> {
        self.tree = source.chunk(self.pos)?;
        self.dirty = true;
        Ok(())
    }

    /// Change one voxel.
    pub fn set_voxel(&mut self, local: LocalPos, shape: VoxelShape) {
        self.tree.set_voxel(local, shape);
        self.dirty = true;
    }

    /// Change one subvoxel, which is what digging into a block does.
    pub fn set_subvoxel(&mut self, local: LocalPos, at: SubvoxelIndex, material: MaterialId) {
        self.tree.set_subvoxel(local, at, material);
        self.dirty = true;
    }

    pub fn voxel(&self, local: LocalPos) -> VoxelShape {
        self.tree.voxel(local)
    }

    pub fn material_at(&self, local: LocalPos, at: SubvoxelIndex) -> MaterialId {
        self.tree.material_at(local, at)
    }

    pub fn solid_count(&self) -> u64 {
        self.tree.solid_count()
    }

    pub fn leaf_count(&self) -> u64 {
        self.tree.leaf_count()
    }

    /// Bytes this chunk holds: its own struct plus what its tree allocated.
    ///
    /// The struct's own size is counted here and the tree's **heap** is counted
    /// by the tree, so a caller summing over chunks never double-counts the
    /// inline root.
    pub fn bytes(&self) -> usize {
        std::mem::size_of::<Self>() + self.tree.heap_bytes()
    }

    /// Nodes in this chunk's tree. A chunk of one material is **one** node, and
    /// being able to say that with a number is what makes the octree's
    /// canonicalisation auditable rather than merely asserted.
    pub fn node_count(&self) -> u64 {
        self.tree.node_count()
    }

    pub fn is_sparse(&self) -> bool {
        self.tree.is_sparse()
    }

    pub fn is_uniform(&self) -> bool {
        self.tree.is_uniform()
    }

    /// A digest of the **contents**, deliberately not of the position: two
    /// chunks that hold the same voxels share a digest, which is what makes it
    /// usable for de-duplication and for comparing a generated chunk against a
    /// loaded one.
    pub fn content_digest(&self) -> Digest {
        self.tree.digest()
    }

    /// Whether anything has changed since the mesh was last built.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Called once a fresh mesh has been stored, so the flag means what it says.
    pub fn mark_meshed(&mut self) {
        self.dirty = false;
    }

    /// Mark the chunk as needing a new mesh **without changing a voxel**.
    ///
    /// This is for the one case where the voxels did not change but the mesh
    /// must: a neighbouring chunk arrived, so the faces this chunk shares with
    /// it are now cullable and were not before. `set_voxel` would have been the
    /// wrong tool — it would have claimed an edit that did not happen.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::octree::local_from;
    use crate::source::VoxelGrid;
    use voxel_model::CHUNK_SIDE;

    const STONE: MaterialId = MaterialId(6);

    #[test]
    fn a_new_chunk_is_empty_and_dirty() {
        let c = Chunk::new(ChunkPos::new(2, -1, 0));
        assert!(c.is_dirty());
        assert_eq!(c.solid_count(), 0);
        assert!(c.is_uniform());
        assert_eq!(c.pos, ChunkPos::new(2, -1, 0));
    }

    #[test]
    fn writing_a_voxel_dirties_the_chunk_and_marking_it_meshed_clears_it() {
        let mut c = Chunk::new(ChunkPos::new(0, 0, 0));
        c.mark_meshed();
        assert!(!c.is_dirty());
        c.set_voxel(local_from([1, 2, 3]).unwrap(), VoxelShape::solid(STONE));
        assert!(c.is_dirty());
        c.mark_meshed();
        assert!(!c.is_dirty());
        c.set_subvoxel(
            local_from([1, 2, 3]).unwrap(),
            SubvoxelIndex::new(0, 0, 0).unwrap(),
            MaterialId(1),
        );
        assert!(c.is_dirty(), "a subvoxel edit is an edit");
    }

    #[test]
    fn generating_from_a_source_adopts_the_whole_tree() {
        let mut g = VoxelGrid::new(ChunkPos::new(0, 0, 0));
        g.set([4, 4, 4], VoxelShape::solid(STONE));
        g.set([5, 4, 4], VoxelShape::solid(STONE));

        let mut c = Chunk::new(ChunkPos::new(0, 0, 0));
        c.generate(&g).unwrap();
        assert_eq!(c.solid_count(), 2);
        assert_eq!(c.voxel(local_from([4, 4, 4]).unwrap()), VoxelShape::solid(STONE));
        assert!(c.is_dirty());
    }

    #[test]
    fn the_content_digest_ignores_the_position_and_the_edits_change_it() {
        let mut a = Chunk::new(ChunkPos::new(0, 0, 0));
        let mut b = Chunk::new(ChunkPos::new(9, 9, 9));
        let before = a.content_digest();
        assert_eq!(before, b.content_digest(), "empty chunks hold the same voxels");
        a.set_voxel(local_from([0, 0, 0]).unwrap(), VoxelShape::solid(STONE));
        assert_ne!(a.content_digest(), before);
        b.set_voxel(local_from([0, 0, 0]).unwrap(), VoxelShape::solid(STONE));
        assert_eq!(a.content_digest(), b.content_digest());
    }

    #[test]
    fn a_chunk_spans_the_whole_chunk_side() {
        // A guard on the octree's level count: if it ever stopped covering 32,
        // this would be the test that noticed.
        let mut c = Chunk::new(ChunkPos::new(0, 0, 0));
        let far = local_from([CHUNK_SIDE - 1, CHUNK_SIDE - 1, CHUNK_SIDE - 1]).unwrap();
        c.set_voxel(far, VoxelShape::solid(STONE));
        assert_eq!(c.voxel(far), VoxelShape::solid(STONE));
        assert_eq!(c.solid_count(), 1);
    }
}
