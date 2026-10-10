//! The sparse voxel octree, and the greedy visible mesh built from it.
//!
//! This crate answers one question — *what is drawn* — from one input, the
//! world's own voxels. It contains no GPU type, no window and no clock: a `Mesh`
//! here is a list of quads in subvoxel coordinates, and turning that into
//! triangles is somebody else's job ([`crate::Mesh`] says why).
//!
//! # The two things C-9 says have to be one pass
//!
//! A voxel is not a cube with an id. It is a **4×4×4 grid of subvoxels**
//! ([`voxel_model::VoxelShape`]), so the surface is not the boundary of a 32³
//! grid — it is the boundary of a **128³** grid, and the interesting geometry
//! (half-soil blocks, stairs, a corner of ore) lives at that second resolution.
//!
//! So the mesher does not mesh voxels and then patch subvoxels in. It rasterises
//! the whole chunk to subvoxel resolution in **one** walk of the octree
//! ([`octree`]), and greedy-merges the 128³ surface in **one** pass ([`mesh`]).
//! Nothing is emitted twice and nothing needs a second stage to fix up.
//!
//! The consequence worth naming: a region of uniform material merges to **one
//! large quad**, because greedy merging cannot tell a 128×128 flat plane of one
//! material from a plane of one material. Uniform worlds get fast *for free*,
//! and detailed voxels show their detail — from the same algorithm, with no
//! level-of-detail switch and therefore no seam where the two paths meet.
//!
//! # The octree is canonical, and that is what makes a digest mean anything
//!
//! Two runs that build the same chunk must produce the same digest, and a digest
//! over a *tree* only means that if the tree is a function of its contents rather
//! than of its insertion history. So [`octree::Node`] is **canonicalised on every
//! write**: eight identical `Uniform` children collapse to one, eight `Empty`
//! children collapse to `Empty`, and a `Mixed` leaf with one material in all 64
//! cells *becomes* that material.
//!
//! The compression and the digest are therefore the same mechanism. This is
//! tested directly: the same voxels inserted in two different orders produce
//! equal digests, and a fully uniform chunk reports a single leaf.
//!
//! # The one rule, here
//!
//! **Refuse, never default.** A voxel outside `0..32`, a subvoxel outside
//! `0..4` and a source that cannot answer are all refusals with names. The
//! mesher never invents a material for a voxel nobody could produce — an
//! unanswerable voxel is an error, not air, because air is a *claim* about the
//! world and the mesher is not entitled to make it.

#![forbid(unsafe_code)]

pub mod chunk;
pub mod mesh;
pub mod refine;
pub mod octree;
pub mod raycast;
pub mod source;

/// Subvoxels per chunk edge: `CHUNK_SIDE × SUBVOXEL_SIDE = 32 × 4`.
pub const SUBVOXEL_CHUNK_SIDE: i32 = voxel_model::CHUNK_SIDE * voxel_model::SUBVOXEL_SIDE as i32;

pub use chunk::Chunk;
pub use mesh::{Loop, Mesh, MeshConfig, MeshStats, Mesher, Quad};
pub use octree::{Node, SparseVoxelOctree, OCTREE_LEVELS};
pub use refine::{refine, RefineConfig, RefineStats, RefinedFace, Triangle, Vertex};
pub use raycast::{Crossing, Crossings, Hit, Ray};
pub use source::{AllAir, ChunkNeighborhood, VoxelGrid, VoxelSource};
