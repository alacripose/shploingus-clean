//! **The port: this clean room's own cells, handed to the parent's own mesher — and the measurement that
//! decides whether the parent's derivation may become this crate's.**
//!
//! Derived from `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §43 (the record's port order: appearance/frame, then
//! the mesher, the world's truth, content and the shell) and from `docs/APPEARANCE.md`'s parent crates'
//! own interface documents — specifically `crates/voxel-mesh/src/source.rs`, whose two traits exist so that
//! *"the mesher can be tested with a hand-built four-voxel chunk and then driven by a full shaper pipeline
//! without a line changing in either"*. That is the hook this port uses, and it is the parent's.
//!
//! Declared under **Constitution P7 v2.0.0** (2026-10-13): a port declares its `path` dependency and pins
//! the parent bytes it read by `path + digest`. The pin is
//! `docs/baselines/port-appearance-seam/pinned-parent-bytes.txt`, verified with `sha256sum -c`.
//!
//! # The four concerns, and where each one lives (2026-10-14, the structure pass)
//!
//! | file | what it owns |
//! |---|---|
//! | [`mapping`] | how the two matter models meet: the parent's subvoxel resolution, the one id bijection, this room's refusal as the parent's own, and [`Water`] |
//! | [`adapter`] | the parent-facing **seam**: this clean room's world as the parent's `VoxelSource` ([`HandedOver`]) and a chunk's neighbourhood as the parent's `ChunkNeighborhood` ([`Halo`]) |
//! | [`differential`] | the **measurement**: two derivations' unit faces, every divergence named by reason, the parent's four declared horizon sizes, and the comparison itself ([`differential`]) |
//! | [`tests`] | the two gates and the stream figure, **run** rather than argued |
//!
//! Only a caller that would *drive* the parent's mesher needs [`adapter`]; only a reader of the measurement
//! needs [`differential`]. Nothing outside this directory needs either — this port is a **measurement**, not
//! a dependency of the client — and the module boundary is what keeps that true: the seam and its verdict
//! can no longer be read as one thing, because they are not one thing.
//!
//! It is **not** wired into the client's geometry: whether it may be is what the measurement decides, and
//! the decision is recorded in `docs/baselines/port-appearance-seam/README.txt` §7 rather than assumed here.
//!
//! # The stream half of the seam, and its exact blocker
//!
//! [`Resident`](crate::mesh::Resident) in this crate is the clean room's own residency, and the parent's is
//! `voxel_stream::Resident` (`crates/voxel-stream/src/resident.rs`, pinned below). The parent's is **not**
//! a port candidate for this world, and the reason is in its own signature rather than in a judgement:
//!
//! ```text
//! pub struct Resident { source: WorldSource, ... }            // resident.rs
//! pub fn new(source: WorldSource) -> Refused<Self>            // resident.rs
//! pub struct WorldSource { spec: WorldSpec, ... }             // world_source.rs
//! ```
//!
//! A `Resident` can only stream a `WorldSource`, and a `WorldSource` is a `voxel_shapers::WorldSpec` — the
//! parent's **generator**. There is no trait, no generic parameter and no injection point between the
//! resident set and the generator, so the parent's residency cannot be driven by this clean room's cells
//! without editing a parent byte (forbidden here) or adopting the parent's generator (a different seam —
//! U3, not U4). `Resident::ensure_horizon` is the other half of the same mismatch: it names a **cube** of
//! `(2r + 1)³` chunks around a centre and generates any of them on demand, which is a question this world
//! could not be asked while it owned one finite patch, materialised whole, and refused every position
//! outside it ([`CubicWorld::cell`](crate::chunk::CubicWorld::cell)).
//!
//! **Half of that is no longer true, and the half that is is the parent's (2026-10-14).** The keyed question
//! *is* now answerable one chunk at a time: [`ChunkSource::chunk`](crate::chunk::ChunkSource::chunk) derives
//! one chunk's cells from the declared fields on its own, so *give me the chunk at this key* is a question
//! **this** world answers — while its whole-patch door
//! ([`CubicWorld::from_fields`](crate::chunk::CubicWorld::from_fields),
//! [`CubicWorld::chunk`](crate::chunk::CubicWorld::chunk),
//! [`CubicWorld::top_chunk_at`](crate::chunk::CubicWorld::top_chunk_at)) answers the same law at the
//! whole-patch scope. What still cannot be driven is `voxel_stream::Resident` itself, and the reason remains
//! its own signature — `new(source: WorldSource)` takes the parent's generator **by type** — so the mismatch
//! now stands on the parent's side of the seam rather than in this world's shape. Both numbers, the keyed
//! door's reach and the parent's four horizons, are printed by the measurement test.

use std::collections::{BTreeMap, BTreeSet};

use voxel_mesh::mesh::{MeshConfig, Quad as ParentQuad};
use voxel_mesh::source::{ChunkNeighborhood, VoxelSource};
use voxel_mesh::{Chunk as ParentChunk, Mesher as ParentMesher};
use voxel_model::{
    ChunkPos, LocalPos, MaterialId as ParentMaterialId, Refusal as ParentRefusal,
    RefusalKind as ParentRefusalKind, Refused as ParentRefused, SubvoxelIndex, VoxelShape,
};

use crate::chunk::CubicWorld;
use crate::coordinates::{ChunkKey, VoxelPos, CHUNK_SIDE};
use crate::material::MaterialId;
use crate::mesh::{
    Axis, Budget, Effort, Ladder, Layer, Level, Mesh, Mesher, ResidencyHorizon, WorkLedger,
};
use crate::refusal::{Refusal, RefusalKind};
use crate::stage::Declared;

mod adapter;
mod differential;
mod mapping;
#[cfg(test)]
mod tests;

pub use adapter::*;
pub use differential::*;
pub use mapping::*;

