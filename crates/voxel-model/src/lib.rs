//! The Shploingus world model.
//!
//! Everything that is **true about the world** lives here, and nothing that is
//! about *drawing* it does. There is no GPU type, no file handle and no clock in
//! this crate, which is what makes the world's determinism claim auditable: the
//! model cannot consult anything outside `(seed, tick, declared period)` because
//! it has no way to.
//!
//! # The shape
//!
//! * [`coord`] — signed coordinates, the settled scale contract, Euclidean chunk
//!   arithmetic.
//! * [`subvoxel`] — the **4×4×4** composition grid (stored) and the **256×**
//!   detail lattice (addressed, never stored).
//! * [`shaper`] — the **voxel shapers**: two kinds (physical, which may change a
//!   block's silhouette, and material, which may not) crossed with two levels
//!   (the stored 4×4×4 grid and the addressed 256 lattice). The kind is enforced by
//!   two traits and two writers; the level is declared and *verified* at
//!   declaration time.
//! * [`profile`] — **the shapes a block can be**, as analytic surfaces of
//!   revolution: a cylinder, a frustum, a dome, and the invariant **stalagmite** of
//!   Szymczak et al. (2025), whose one parameter comes from the world's drip rate.
//! * [`material`] — a material as a named set of components in **exact rational
//!   ratios summing to 1**, and a registry that refuses rather than defaulting.
//! * [`conservation`] — the **exact** set (mass, volume) and the boundary that
//!   separates it from the approximate families.
//! * [`identity`] — `sha256:` content identity and its strict parser.
//! * [`refusal`] — **refuse, never default**, as a type that collects every
//!   refusal rather than the first.
//! * [`chain`] — the one append-only, hash-chained record, namespaced by
//!   world/year/season/campaign, with `not_evaluable` as a first-class state.
//! * [`envelope`] — the v3 record's closed-typed envelope, with **no `details`
//!   map** and 4 KiB bounds refused rather than truncated.
//! * [`ticket`] — the ticket/governance taxonomy, **a projection of the chain**
//!   rather than a second store.
//! * [`wire`] — a declared schema and a codec behind a trait: every field
//!   length-described, decode collecting, encode refusing.
//!
//! # The one rule
//!
//! **Refuse, never default.** A value that cannot be validated is refused and
//! named, not silently replaced by a plausible one — because a plausible
//! substitute is indistinguishable from a correct one until much later, and by
//! then it has been written into a world.

// Measured: this crate contains zero `unsafe`. That is a property worth having,
// and a property that is worth defending rather than holding by coincidence.
// `forbid` rather than `deny`, so no `#[allow]` anywhere can buy an exception.
#![forbid(unsafe_code)]

pub mod chain;
pub mod conservation;
pub mod envelope;
pub mod grant;
pub mod ticket;
pub mod coord;
pub mod identity;
pub mod material;
pub mod profile;
pub mod refusal;
pub mod shaper;
pub mod subvoxel;
pub mod wire;

pub use coord::{
    ChunkPos, LocalPos, VoxelPos, BAND_MAX_Z, BAND_MIN_Z, CHUNK_SIDE, CHUNK_VOLUME, DETAIL_SIDE,
    SUBVOXEL_COUNT, SUBVOXEL_SIDE,
};
pub use identity::{Digest, Identity};
pub use material::{
    Component, ComponentId, Material, MaterialId, MaterialRegistry, OpticalClass, Ratio,
    Reflectance, SourcedReflectance,
};
pub use profile::{
    column_cross_axis, face_vector, in_plane_axes, Profile, Regime, MAX_RADIUS,
};
pub use refusal::{Refusal, RefusalKind, RefusalSet, Refused};
pub use shaper::{
    DisplacementField, DisplacementWriter, Level, LevelMask, MaterialShaper, Neighbourhood,
    PhysicalShaper, ShadingField, ShadingWriter, ShapeWriter, Shaper, ShaperKind, ShaperSet,
    SlotWriter, Tally, VoxelContext, MAX_DISPLACEMENT_SUBVOXELS,
};
pub use subvoxel::{DetailIndex, Face, SubvoxelIndex, VoxelShape};
