//! The build engine: a bounded, deterministic worker pool over the shapers, and
//! the mesh residency it feeds.
//!
//! # What this crate is for
//!
//! `voxel-stream`'s own docs say it has "no threads, no file I/O, no clock, and
//! no GPU type", and that "a caller decides how many workers there are and how
//! work is handed out". **This crate is that caller.** It is the only crate in
//! the workspace that spawns a thread, and it exists so that the shapers can be a
//! *constant* part of the world — always running somewhere, filling the view out
//! from the player — rather than a pause between frames.
//!
//! # The three ideas, each from a measurement
//!
//! * **[`budget`]** — a memory ceiling as a type that *refuses*. The measurement
//!   that shapes it: one [`voxel_mesh::Mesher`]'s scratch is **~4.8 MB** and one
//!   is needed *per worker*, so the pool is sized by what the machine can hold
//!   rather than by how many cores it has.
//! * **[`pool`]** — workers that need **no shared state**: each meshes a chunk
//!   from the generator alone. The measurement behind it: a resident voxel chunk
//!   costs **~89 kB** and its finished mesh **~1.3 kB**, so answering a boundary
//!   question from neighbours is 182× the memory of the thing being built.
//! * **[`stream`]** — the engine proper. The measurement behind *its* shape: the
//!   first playable band is two chunks tall, so a radius-24 **cube** spends
//!   twelve times the vertical extent the world has, and the declared residency
//!   horizon costs **10.2 GB** as resident voxels against **12 MB** as meshes.
//!
//! # The one rule, here
//!
//! **Refuse, never default.** A pool with no workers is refused rather than
//! started, because a pool that accepts work and never runs it reports success.
//! A budget of zero bytes is refused rather than every allocation refused one at
//! a time. A charge that would cross the ceiling is refused and *named*, with the
//! request, the total and the ceiling in the message. A mesh the budget cannot
//! hold is dropped and counted, never stored and never silently charged.

#![forbid(unsafe_code)]

pub mod budget;
pub mod pool;
pub mod stream;
pub mod tick;

pub use budget::Budget;
pub use pool::{Built, Collected, Pool, Request, Submitted, DEFAULT_QUEUE_CAPACITY};
pub use stream::{chunks_touching, ChunkWindow, Frame, Pick, Streamer, DEFAULT_WINDOW};
pub use tick::{CellBox, HALO_VOXELS, Planned, Scheduler, UnitOfWork};
