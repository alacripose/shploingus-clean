//! The governed transition: **who** acts, **under what**, and **what kind of act** it is.
//!
//! Copied whole from `ala-cities/src/world/transition.rs` (lines 1–183) — the ids, the verb
//! list, the transition and its identity, and the refusal vocabulary. Everything the source
//! computes, this computes; the two substitutions are the tree's own digest and the tree's own
//! coordinate, and both are named where they are made.
//!
//! # Why the ids are newtypes over `String` and not numbers
//!
//! The source's reason, kept because it is a real one: an id that can be *typed* is an id a
//! caller will invent a second spelling of, and the store's own rule is that **ids come from
//! the directory, never from a counter**. A `String` newtype that refuses empty keeps the
//! caller honest about what it is handing over.
//!
//! # The transition's identity, and why the domain string is this tree's
//!
//! The source mints its digest with a hand-rolled SHA-256 and the domain
//! `ALA-CITIES-WORLD-TRANSITION-ID-V2`. This tree has **one** hashing implementation, and
//! `sha2` is named by `voxel-model` and by no other crate (`docs/LEDGER.md:1994`), so the
//! digest here is [`crate::identity::Digest::of_fields`] — a domain-separated sequence with
//! every field **length-prefixed**, which distinguishes `("ab", "c")` from `("a", "bc")`. The
//! source length-prefixed too; this is the same scheme, under the tree's own digest, and the
//! domain is this tree's name for the thing.
//!
//! **A hand edit is not identified this way**, and the difference is deliberate: see
//! [`voxel_shapers::earthworks::hand_change`] — a transition's id covers the actor, the verb,
//! the reservation and the epoch, and a hand edit has none of the last three to distinguish a
//! redo from an undo. Its identity is over its own content instead.
use crate::endian::LeBytes;

use super::region::Coord;
use super::WorldError;
use crate::identity::Digest;
use core::fmt;

/// Who acts. A persistent identity, not a session.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActorId(String);

impl ActorId {
    pub fn new(value: impl Into<String>) -> Result<Self, WorldError> {
        Ok(Self(named_id("actor", value)?))
    }

    /// The declared id of **the person at the console**.
    ///
    /// The DSL declares them — `player = person "The player" "files plain prose tickets at the
    /// console"` — and the ruling of 2026-10-02 is that a hand edit is a **governed act by the
    /// player as an actor**, so a by-hand change is attributable exactly as an agent's is.
    /// There is one spelling of this id and it is here.
    pub fn player() -> Self {
        Self("player".to_owned())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The task a change serves. Absent for work that is not a job — including a hand edit.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskId(String);

impl TaskId {
    pub fn new(value: impl Into<String>) -> Result<Self, WorldError> {
        Ok(Self(named_id("task", value)?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The claim on a target, **declared before the change**.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReservationId(String);

impl ReservationId {
    pub fn new(value: impl Into<String>) -> Result<Self, WorldError> {
        Ok(Self(named_id("reservation", value)?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// What a change is addressed to.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TargetRef(String);

impl TargetRef {
    pub fn new(value: impl Into<String>) -> Result<Self, WorldError> {
        Ok(Self(named_id("target", value)?))
    }

    /// A single voxel.
    pub fn voxel(coord: Coord) -> Self {
        Self(format!("voxel:{},{},{}", coord.x, coord.y, coord.z))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The voxel this target names, when it names one.
    pub(crate) fn coord(&self) -> Option<Coord> {
        let value = self.0.strip_prefix("voxel:")?;
        let mut values = value.split(',').map(str::parse::<i32>);
        Some(Coord::new(
            values.next()?.ok()?,
            values.next()?.ok()?,
            values.next()?.ok()?,
        ))
    }
}

/// The id rule, stated once for the whole governed vocabulary: trim, refuse empty.
///
/// [`change`](super::change)'s ids — `ComponentId`, `Account`, a `LineageId`'s parts — are
/// the same rule, so they call this rather than restating it.
pub(crate) fn named_id(kind: &str, value: impl Into<String>) -> Result<String, WorldError> {
    let value = value.into();
    let value = value.trim();
    if value.is_empty() {
        return Err(WorldError::InvalidTransition {
            reason: format!("{kind} id must not be empty"),
        });
    }
    Ok(value.to_owned())
}

/// What kind of act a change is. **Closed on purpose**, and it grows by decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PhysicalVerb {
    Break,
    Place,
    Gather,
    Dig,
    Fill,
    Use,
    Craft,
}

impl PhysicalVerb {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Break => "break",
            Self::Place => "place",
            Self::Gather => "gather",
            Self::Dig => "dig",
            Self::Fill => "fill",
            Self::Use => "use",
            Self::Craft => "craft",
        }
    }
}

impl fmt::Display for PhysicalVerb {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A governed change of the world, **before** it is applied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldTransition {
    pub actor: ActorId,
    pub task: Option<TaskId>,
    pub intent: PhysicalVerb,
    pub target: TargetRef,
    pub reservation: Option<ReservationId>,
    pub epoch: u64,
    pub nonce: u64,
}

impl WorldTransition {
    pub fn new(
        actor: ActorId,
        task: Option<TaskId>,
        intent: PhysicalVerb,
        target: TargetRef,
        reservation: Option<ReservationId>,
        epoch: u64,
        nonce: u64,
    ) -> Self {
        Self {
            actor,
            task,
            intent,
            target,
            reservation,
            epoch,
            nonce,
        }
    }

    /// This transition's identity, over everything that distinguishes it.
    ///
    /// Domain-separated and length-prefixed, so a transition cannot be confused with another
    /// whose fields merely concatenate the same way. `epoch` and `nonce` are in it because a
    /// repeated *intent* by the same actor on the same target under the same reservation is
    /// **not** the same act — that is what makes exactly-once expressible at all.
    pub fn id(&self) -> Digest {
        Digest::of_fields(
            "shploingus/world-transition/v1",
            &[
                self.actor.as_str().as_bytes(),
                self.task.as_ref().map_or("", TaskId::as_str).as_bytes(),
                self.intent.as_str().as_bytes(),
                self.target.as_str().as_bytes(),
                self.reservation
                    .as_ref()
                    .map_or("", ReservationId::as_str)
                    .as_bytes(),
                &self.epoch.into_le_bytes(),
                &self.nonce.into_le_bytes(),
            ],
        )
    }
}

/// Why a governed change was refused. Copied whole.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransitionRefusal {
    MissingReservation,
    UnownedReservation,
    ReservationCommitted,
    MissingTarget,
    MissingAccount,
    InsufficientResources,
    ConservationFailure,
    InvalidPhase,
    DuplicateTransition,
    ApplicationFailure,
    SnapshotUnavailable,
}

impl TransitionRefusal {
    /// A stable, machine-readable name — the record's own spelling, kept.
    pub const fn name(&self) -> &'static str {
        match self {
            Self::MissingReservation => "missing-reservation",
            Self::UnownedReservation => "unowned-reservation",
            Self::ReservationCommitted => "reservation-committed",
            Self::MissingTarget => "missing-target",
            Self::MissingAccount => "missing-account",
            Self::InsufficientResources => "insufficient-resources",
            Self::ConservationFailure => "conservation-failure",
            Self::InvalidPhase => "invalid-phase",
            Self::DuplicateTransition => "duplicate-transition",
            Self::ApplicationFailure => "application-failure",
            Self::SnapshotUnavailable => "snapshot-unavailable",
        }
    }
}

impl fmt::Display for TransitionRefusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actor() -> ActorId {
        ActorId::new("buffy").unwrap()
    }

    fn transition(epoch: u64, nonce: u64) -> WorldTransition {
        WorldTransition::new(
            actor(),
            None,
            PhysicalVerb::Dig,
            TargetRef::voxel(Coord::new(1, 2, 3)),
            None,
            epoch,
            nonce,
        )
    }

    #[test]
    fn an_id_may_not_be_empty_or_whitespace() {
        assert!(ActorId::new("").is_err());
        assert!(ActorId::new("   ").is_err());
        assert!(TaskId::new("").is_err());
        assert!(ReservationId::new("").is_err());
        assert!(TargetRef::new("").is_err());
        // And it is trimmed rather than kept with the caller's padding.
        assert_eq!(ActorId::new("  buffy  ").unwrap().as_str(), "buffy");
    }

    #[test]
    fn a_transition_carries_its_own_identity() {
        let a = transition(31, 1);
        let b = transition(31, 1);
        let c = transition(31, 2);
        assert_eq!(a.id(), b.id(), "the same act is the same act");
        assert_ne!(a.id(), c.id(), "a different nonce is a different act");
        assert_ne!(a.id(), Digest::from_bytes([0; 32]));
    }

    #[test]
    fn the_identity_separates_fields_that_would_otherwise_concatenate_alike() {
        // The reason `of_fields` length-prefixes: two actors whose names the same way.
        let left = WorldTransition::new(
            ActorId::new("ab").unwrap(),
            None,
            PhysicalVerb::Dig,
            TargetRef::new("c").unwrap(),
            None,
            0,
            0,
        );
        let right = WorldTransition::new(
            ActorId::new("a").unwrap(),
            None,
            PhysicalVerb::Dig,
            TargetRef::new("bc").unwrap(),
            None,
            0,
            0,
        );
        assert_ne!(left.id(), right.id());
    }

    #[test]
    fn the_player_is_an_actor_with_one_declared_spelling() {
        assert_eq!(ActorId::player().as_str(), "player");
        assert_eq!(ActorId::player(), ActorId::player());
    }

    #[test]
    fn a_voxel_target_is_a_voxel_target() {
        let target = TargetRef::voxel(Coord::new(-1, 0, 47));
        assert_eq!(target.as_str(), "voxel:-1,0,47");
        assert_eq!(target.coord(), Some(Coord::new(-1, 0, 47)));
        assert_eq!(TargetRef::new("selection:plan-1").unwrap().coord(), None);
    }

    #[test]
    fn every_refusal_names_itself_as_the_record_spells_it() {
        let all = [
            TransitionRefusal::MissingReservation,
            TransitionRefusal::UnownedReservation,
            TransitionRefusal::ReservationCommitted,
            TransitionRefusal::MissingTarget,
            TransitionRefusal::MissingAccount,
            TransitionRefusal::InsufficientResources,
            TransitionRefusal::ConservationFailure,
            TransitionRefusal::InvalidPhase,
            TransitionRefusal::DuplicateTransition,
            TransitionRefusal::ApplicationFailure,
            TransitionRefusal::SnapshotUnavailable,
        ];
        for refusal in all {
            assert_eq!(refusal.to_string(), refusal.name());
        }
    }
}
