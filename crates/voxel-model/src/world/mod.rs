//! The **governed change** of the world: who changed it, under what, and what kind of act.
//!
//! # Why this module is at the crate root and not under `civilian/`
//!
//! Because the *record* has to name it, and the record is not a civilian file.
//! `voxel-shapers::earthworks` widens `Modification` with `ActorId`, `TaskId`,
//! `ReservationId`, `PhysicalVerb` and a transition digest, so `voxel-shapers` must be able
//! to import these **today**. Putting them under `civilian/` would have meant the machine's
//! seam deciding where the world's record lives — and it would have left the whole port
//! undeclared behind them. At the root they are importable immediately, and the machine's own
//! `use crate::world::{…}` lines resolve with **no edit at all**.
//!
//! # The world this is a change *to*, in the tree's own words
//!
//! Four sources agree, and none of them is this module's invention:
//!
//! * `voxel-shapers/src/column.rs` — *"A world is `generator(x, y, z)` **composed with** the
//!   `Earthworks` record"*, and *"**a saved world is a seed, a tick and a list of changes**"*;
//! * `docs/LEDGER.md` C-96.1 — the value is `voxel_client::game::World`, *"**a seed, a tick and
//!   a configuration held together**"*, and *"**a save is record + seed + tick**"*;
//! * `voxel-shapers/src/stages.rs` — `Earthworks` is a declared **stage**, and player edits run
//!   at the **×1** rung, *"part of the determinism contract, not a performance knob"*;
//! * `docs/SHAPERS.md` §1 row 15 — earthworks is *"**recorded** player/agent modification of the
//!   world, each entry carrying its mass ledger"*.
//!
//! So the world is **not** this module's, and not the civilians': it is the game's
//! (`App::world`), composed with the earthworks record, ticked at that record's own rung. What
//! lives here is only the **vocabulary** a governed change is spelled in — plus the rule that
//! the change is recorded where the world already keeps its changes.
//!
//! # Three things this module deliberately does *not* become
//!
//! | not this | because |
//! |---|---|
//! | a **world** (`World { spec, store, … }`) | `forbidden.second-record` and `rule.one-thing` (`architecture/workspace.dsl`): the world is `App::world` composed with the record, and a fourth `World` beside `voxel-client::game::World`, `voxel-client::world::WorldPipeline` and `voxel_stream::WorldSource` is a parallel world, which is the defect. |
//! | a **`delta`** module | `docs/DESIGN-SPEC.md:79` (F10) assigns `delta` to `voxel-shapers::{delta,earthworks}` — the sedimentary delta, a *different subject under the same name*. |
//! | a **new crate** | `rule.zero-erosion` in the DSL, and rule 2: the DSL is changed first (it was — see below). |
//!
//! # What is mapped, and what is therefore absent
//!
//! `ala-cities`' `world/` is **15 modules and 15,892 lines** of world *generation*. That
//! generator is this tree's `crates/voxel-shapers`, contracted by `docs/SHAPERS.md`, and it is
//! **read and mapped, never copied** (`event-0133`, strike 9). So:
//!
//! * `world/region.rs`'s `Coord`, `ChunkCoord`, `CHUNK_SIZE` and the band constants are this
//!   tree's own `coord::VoxelPos`, `coord::ChunkPos`, `coord::CHUNK_SIDE`,
//!   `coord::BAND_MIN_Z..BAND_MAX_Z` — re-exported under the machine's names, not duplicated
//!   (`event-0131`, the season, is what duplicating a thing the tree owns costs);
//! * `world/identity.rs`'s hand-rolled SHA-256 is **not copied**: `sha2` is named by
//!   `voxel-model` and by no other crate (`docs/LEDGER.md:1994`, *"the dependency boundary is
//!   the point"*), so [`Digest`] here is [`crate::identity::Digest`];
//! * the generator's own delta variants (`FluidTransfer`, `FieldChange`, `FieldOutput`,
//!   `EcologyChange`, `Stratum`) and its `ProcessOutput` bridge are the **generator's**, and
//!   are not admitted here: measured, the civilian machine names **none** of them.
//!
//! # The DSL was changed first (rule 2)
//!
//! `architecture/workspace.dsl` gained a **`world` component under `game`** and the arrow
//! **`civilians -> world`** — *"the governed change: actor, task, verb, reservation and
//! transition id, recorded in the earthworks entry"* — **before** this module and the widening
//! it serves. Validated: 0 errors, 0 warnings. The user's ruling of 2026-10-02 chose to
//! **widen the record** rather than keep the governed fields beside it, and chose that a hand
//! edit is **a governed act by the player as an actor** — which is why [`ActorId::player`]
//! exists and why `voxel_shapers::earthworks::hand_change` takes an actor and a verb.

pub mod book;
pub mod change;
pub mod governed;
pub mod region;
pub mod transition;

pub use crate::identity::Digest;
pub use book::{
    ConservationAudit, ConservationSnapshot, ConservationViolation, DeltaApplyError, DeltaStore,
    Evidence,
};
pub use change::{
    Account, ComponentId, Delta, EarthworksEffect, EarthworksError, FluidAddress, LineageId,
    Occupancy, OccupancyProvenance,
};
pub use governed::{TransitionReceipt, WritePermission};
pub use region::{ChunkCoord, Coord, SignedRegion, FIRST_BAND_MAX_Z, FIRST_BAND_MIN_Z};
pub use transition::{
    ActorId, PhysicalVerb, ReservationId, TargetRef, TaskId, TransitionRefusal, WorldTransition,
};

use core::error::Error;
use core::fmt;

/// What a governed change can be refused for.
///
/// The governed half of `ala-cities/src/world/mod.rs`'s enum, kept whole. The other half of
/// that enum — `InvalidIdentity`, `IdentityMismatch`, `UnsupportedLineage`, `InvalidProjection`,
/// `UnadmittedFieldSpec`, `InvalidFieldSpec`, `FieldDataMissing`, `InvalidStageInput`,
/// `ProcessRefused` — is the **generator's** refusal vocabulary (projection, stages, fields,
/// process scheduling) and belongs to `voxel-shapers`, which has its own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorldError {
    /// A digest that should identify something is missing.
    DigestMismatch { component: &'static str },
    /// A coordinate, or a region, outside the first playable band.
    OutOfRegionAccess { region: SignedRegion, coord: Coord },
    /// A change that could not be formed — an empty id, a negative quantity, a broken chain.
    InvalidTransition { reason: String },
    /// The change would not conserve mass.
    ConservationFailure { reason: String },
}

impl WorldError {
    /// A stable, machine-readable name. Never prose, never localised — the same rule
    /// [`crate::refusal::RefusalKind::name`] follows.
    pub const fn kind_name(&self) -> &'static str {
        match self {
            Self::DigestMismatch { .. } => "digest_mismatch",
            Self::OutOfRegionAccess { .. } => "out_of_region_access",
            Self::InvalidTransition { .. } => "invalid_transition",
            Self::ConservationFailure { .. } => "conservation_failure",
        }
    }
}

/// How a coordinate is written in a refusal.
///
/// The tree's [`crate::coord::VoxelPos`] deliberately carries no `Display` — `Debug` is a view
/// and a coordinate is not a string — so the port formats it **here**, rather than adding
/// surface to a type it does not own, for its own convenience. The source's `Coord` had a
/// `Display` because `Coord` was that tree's type; this is the same text, produced where the
/// need is.
pub(crate) fn coord_text(coord: Coord) -> String {
    format!("({},{},{})", coord.x, coord.y, coord.z)
}

impl fmt::Display for WorldError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DigestMismatch { component } => {
                write!(formatter, "{component} digest is missing")
            }
            Self::OutOfRegionAccess { region, coord } => write!(
                formatter,
                "coordinate {} is outside region {}..={}",
                coord_text(*coord),
                coord_text(region.min),
                coord_text(region.max)
            ),
            Self::InvalidTransition { reason } => {
                write!(formatter, "invalid world transition: {reason}")
            }
            Self::ConservationFailure { reason } => {
                write!(formatter, "conservation failure: {reason}")
            }
        }
    }
}

impl Error for WorldError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_refusal_has_a_stable_name() {
        let errors = [
            WorldError::DigestMismatch { component: "source" },
            WorldError::ConservationFailure {
                reason: "underflow".to_owned(),
            },
            WorldError::InvalidTransition {
                reason: "empty id".to_owned(),
            },
        ];
        for error in errors {
            assert!(!error.kind_name().is_empty());
            // A name is a token, never prose: no spaces, no capitals.
            assert!(
                error
                    .kind_name()
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c == '_'),
                "{} is prose, not a name",
                error.kind_name()
            );
        }
    }
}
