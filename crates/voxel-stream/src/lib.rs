//! The world as voxels, and the four horizons that decide which voxels exist.
//!
//! This crate is the **link** between the shapers and the mesher. Above it, a
//! world is a `WorldSpec` and a set of papers; below it, a world is a list of
//! quads. Neither side knows about the other, and that is the whole point: the
//! mesher can be tested against four hand-written voxels, the shapers can be
//! tested against nothing at all, and the claim that they work *together* is
//! made here and tested here.
//!
//! # The three modules are three different questions
//!
//! * [`world_source`] — *what is at this voxel?* The shapers' world, answerable
//!   one voxel at a time or one chunk at a time. Both ways reach
//!   `voxel_shapers::chunk_fields`' one material law, and that they agree is
//!   **tested** voxel by voxel rather than asserted.
//! * [`horizons`] — *how far out is this horizon?* Four horizons, strictly
//!   nested, and each with the field-ladder level it reads at. The nesting is
//!   checked; a set that is not nested is four unrelated numbers, not four
//!   horizons.
//! * [`resident`] — *what is loaded, and what does it draw?* The set of live
//!   chunks, the meshes built from them, and the invalidation that a neighbouring
//!   chunk's arrival forces.
//! * [`light`] — *how much of the Sun reached this cell?* A shadow ray through the
//!   world's own octree, at the world's own subvoxel addresses, because a cell's light
//!   is a property of what stands between it and the Sun.
//!
//! # What is deliberately *not* here
//!
//! No threads, no file I/O, no clock, and no GPU type. The resident set is
//! single-threaded and synchronous: a caller decides how many workers there are
//! and how work is handed out, and — per F12 — that decision cannot change the
//! output. The digest tests exist to make that claim checkable rather than
//! aspirational: building the same chunks in a different order must produce the
//! same [`voxel_model::Digest`].
//!
//! # The one rule, here
//!
//! **Refuse, never default.** A horizon set that is not nested is refused at
//! construction. A chunk that is not resident is `None` and not an empty chunk.
//! A mesh built while a neighbour is absent is *named* provisional rather than
//! passed off as final — because "this is what the world looks like" and "this
//! is what the world looks like so far" are different claims, and a renderer
//! that cannot tell them apart cannot be debugged.
//!
//! **The audit, 2026-10-10.** 2 833 lines, 5 modules, 70 public items, 48 tests; read by `work`,
//! `client` and the binary. The cross-crate equality claim — the single-voxel read and the chunk
//! build are one world — is the strongest claim any crate in this workspace makes, and it has its
//! own tests rather than a note saying it should have one. **No finding this pass.**

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod horizons;
pub mod light;
pub mod resident;
pub mod world_source;

pub use horizons::{Horizon, HorizonRule, Horizons};
pub use resident::Resident;
pub use world_source::{WorldNeighborhood, WorldSource};
