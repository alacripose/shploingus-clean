//! The governed change path: **the one way a transition becomes a committed change.**
//!
//! `book.rs` is the book — it can commit a batch and it can audit what the batch did. What it
//! cannot do is decide *whether this transition may happen at all*, and that is the gap the
//! civilians hit: `agency/action.rs`'s single commit point has an actor, a task, a verb and a
//! reservation in hand, and this is where they are handed over.
//!
//! # Why this is not called `World`, and not a world
//!
//! `world/mod.rs`'s own table refuses it, in the record's words: a **world**
//! (`World { spec, store, … }`) is refused because *"a fourth `World` beside
//! `voxel-client::game::World`, `voxel-client::world::WorldPipeline` and
//! `voxel_stream::WorldSource` is a parallel world, which is the defect"* (`rule.one-thing`,
//! `forbidden.second-record`). So the world stays the game's (`App::world`) composed with the
//! earthworks record; what lands here is only the **path**, and it is a path *over the book*,
//! not a store beside it.
//!
//! # The order is the design
//!
//! 1. **exactly-once** — a transition this book has already committed is `DuplicateTransition`
//!    and never a second application. This is the check `applied_ids` exists for;
//! 2. **the reservation** — a claimed target must be claimed *by this actor* and *on this
//!    target*. A reservation is a claim declared **before** the change (`docs/SHAPERS.md` §1
//!    row 15), so a change that skips its own claim is refused rather than back-dated;
//! 3. **the commit** — `DeltaStore::apply_with_id`, which clones, validates, audits and restores
//!    the whole store on **any** failure. A half-applied batch is never observable;
//! 4. **the evidence row** — written *after* the commit, carrying the transition's identity, so
//!    the row cannot describe a change that did not happen and a reader can find it by
//!    reservation or by transition.
//!
//! # The authorization, and exactly how far it reaches
//!
//! The user's ruling of 2026-10-02 chose that the **handle takes the decision**: the caller
//! consults Laya's own action matrix (`voxel_package::policy::decide(Action::WriteWorld, grant)`,
//! where `WriteWorld => RequiresGrant` and `decide` is deny-by-default — *"silence is not
//! permission"*), and mints a [`WritePermission`] from the [`Grant`] that decision returned.
//! [`DeltaStore::apply_transition`] cannot be called without one.
//!
//! **The matrix is not restated here, and that is the point of the split.** The *class* decision
//! — that a world write requires a grant — stays in `voxel-package`, where `permission_of` is
//! exhaustive with no wildcard arm. Moving it down would have meant a second action enum in this
//! crate, which is D-07's *"two names for one meaning"* — the exact defect `voxel_model::grant`
//! was created by moving *down* rather than duplicating (*"the type moved down instead of up"*).
//! What this crate owns is the bound: a permission names a granter and a scope, and a scope is
//! *"a bound, not a description"*.
//!
//! **What is not claimed.** A [`WritePermission`] records that a scoped, attributed grant was
//! presented. It is not a signature and it does not verify the grant is genuine — `grant.rs`
//! states why (*"a digest establishes integrity, never identity"*), and this module does not
//! pretend otherwise.

use super::book::{DeltaApplyError, DeltaStore, Evidence};
use super::change::EarthworksEffect;
use super::transition::{ReservationId, TransitionRefusal, WorldTransition};
use super::WorldError;
use crate::grant::Grant;
use crate::identity::Digest;

/// **Proof that the decision layer permitted this change.**
///
/// Minted from the [`Grant`] that `policy::decide(Action::WriteWorld, …)` returned as
/// `Decision::Granted`, and required by [`DeltaStore::apply_transition`]. `Grant` already
/// refuses an empty granter and an empty scope at construction, so there is nothing left to
/// refuse here — a permission that exists is one with an author and a bound.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WritePermission {
    granter: String,
    scope: String,
}

impl WritePermission {
    /// Mint a permission from the grant the decision layer returned.
    ///
    /// **The caller is the one that asked the layer**, and that is deliberate: this crate can
    /// not reach `voxel_package::policy` — it sits below it — and a second matrix here would be
    /// a second answer to a question Laya already answers.
    pub fn granted(grant: &Grant) -> Self {
        Self {
            granter: grant.granter().to_owned(),
            scope: grant.scope().to_owned(),
        }
    }

    /// Who authorised it. A **name**, never key material.
    pub fn granter(&self) -> &str {
        &self.granter
    }

    /// What it authorises, bounded.
    pub fn scope(&self) -> &str {
        &self.scope
    }
}

impl core::fmt::Display for WritePermission {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "{} over {}", self.granter, self.scope)
    }
}

/// **What a committed change left behind** — the civilians' receipt, and Laya's reading.
///
/// Carried whole rather than reduced to a boolean, because every field answers a question a
/// reader asks afterwards: *which act was this* (`transition`), *was a claim spent, and which*
/// (`reservation`), *did the books balance* (`conserved`), *how much moved* (`deltas`), *who
/// allowed it* (`granted_by`), and *what the evidence row says* (`evidence`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransitionReceipt {
    /// The transition's own identity — the exactly-once key.
    pub transition: Digest,
    /// The reservation this change spent, when it claimed one.
    pub reservation: Option<ReservationId>,
    /// How many deltas were committed.
    pub deltas: usize,
    /// Whether the conservation audit found the books balanced.
    ///
    /// **A committed change that did not conserve is not possible** — `apply_with_id` refuses
    /// rather than commits one — so this is a receipt reading, not a warning.
    pub conserved: bool,
    /// The state the change left, by digest. The receipt points at what it describes.
    pub audit_digest: Digest,
    /// The granter the permission named. The receipt attributes the act.
    pub granted_by: String,
    /// The evidence row written for this change. **In the receipt**, not merely in the store, so
    /// a caller holding a receipt holds its own proof.
    pub evidence: Evidence,
}

/// A digest, short enough to read inside a one-line evidence message.
///
/// Local to this module rather than new surface on [`Digest`]: `identity.rs` already owns the
/// one canonical rendering, and a second `short()` on the type would be a second spelling of
/// what a digest is.
fn short(digest: Digest) -> String {
    digest.to_hex().chars().take(12).collect()
}

impl DeltaStore {
    /// **Apply one governed transition, or change nothing.**
    ///
    /// The single entry point. A caller with a [`WorldTransition`], the [`EarthworksEffect`] it
    /// formed, and a [`WritePermission`] from Laya's matrix gets a [`TransitionReceipt`] or a
    /// [`TransitionRefusal`] — and in the refusal case the store is **exactly as it was**
    /// (`apply_with_id` restores it), which is what makes a retry safe.
    pub fn apply_transition(
        &mut self,
        transition: &WorldTransition,
        effect: &EarthworksEffect,
        permitted: &WritePermission,
    ) -> Result<TransitionReceipt, TransitionRefusal> {
        let id = transition.id();

        // 1. Exactly-once. The identity covers the actor, the task, the verb, the target, the
        //    reservation, the epoch and the nonce — so a repeated *intent* is a different act,
        //    and one act can never be applied twice.
        if self.has_applied(id) {
            return Err(TransitionRefusal::DuplicateTransition);
        }

        // 2. The claim, declared before the change.
        //
        // **The order is load-bearing, and it was measured rather than assumed.**
        // `has_reservation` asks *"is there an unspent claim here?"* — it is `false` for a
        // committed one — so asking it first returns `MissingReservation` for a claim that
        // exists and was spent, and three distinct facts collapse into one name. A test caught
        // exactly that (`a_spent_claim_is_named_as_spent_rather_than_missing`). So: spent
        // first, then existence, then ownership, then the target.
        if let Some(reservation) = transition.reservation.as_ref() {
            if self.reservation_committed(reservation) {
                return Err(TransitionRefusal::ReservationCommitted);
            }
            if !self.has_reservation(reservation) {
                return Err(TransitionRefusal::MissingReservation);
            }
            match self.reservation_owner(reservation) {
                Some(owner) if owner == transition.actor.as_str() => {}
                // An owner that is not this actor is a different fact from a claim that does not
                // exist, and the two are named differently.
                Some(_) => return Err(TransitionRefusal::UnownedReservation),
                // Unreachable past the two checks above, and still named rather than defaulted:
                // a fallthrough here would be a claim nobody ever verified.
                None => return Err(TransitionRefusal::MissingReservation),
            }
            // **A reservation is a claim on one target.** Reserving one place and changing
            // another is the defect the claim mechanism exists to prevent, so the two are
            // compared as the record spells them.
            if self.reservation_target(reservation) != Some(transition.target.as_str()) {
                return Err(TransitionRefusal::UnownedReservation);
            }
        }

        // 3. The commit: clone, validate, audit, restore-on-any-failure.
        let audit = self
            .apply_with_id(&effect.deltas, Some(id), transition.reservation.as_ref())
            .map_err(refusal_for_apply_failure)?;

        // 4. The evidence row, written now — never before the commit, so it cannot describe a
        //    change that did not happen. The coordinate is recorded when the target names one,
        //    because a reader asking *where* should not have to parse the target back.
        let where_ = match transition.target.coord() {
            Some(coord) => format!(" at ({},{},{})", coord.x, coord.y, coord.z),
            None => String::new(),
        };
        let evidence = Evidence::for_transition(
            "transition.committed",
            format!(
                "{} {} {}{} under {} ({} to {}), permitted by {} over {}",
                transition.actor.as_str(),
                transition.intent.as_str(),
                transition.target.as_str(),
                where_,
                transition
                    .task
                    .as_ref()
                    .map_or("no task", |task| task.as_str()),
                short(audit.before.digest),
                short(audit.after.digest),
                permitted.granter(),
                permitted.scope()
            ),
            id,
            transition.reservation.clone(),
        );
        self.append_evidence(evidence.clone());

        Ok(TransitionReceipt {
            transition: id,
            reservation: transition.reservation.clone(),
            deltas: effect.deltas.len(),
            conserved: audit.is_conserved(),
            audit_digest: audit.after.digest,
            granted_by: permitted.granter().to_owned(),
            evidence,
        })
    }
}

/// The source's own bridge: an apply failure becomes a *transition* refusal, and the mapping
/// names which kind of failure it was rather than flattening them into one.
///
/// **A conservation failure is not an application failure.** The first is a state the record
/// forbids; the second is a caller that built something malformed. A reader has to be able to
/// tell them apart, which is why they are not one variant.
fn refusal_for_apply_failure(failure: DeltaApplyError) -> TransitionRefusal {
    match failure {
        DeltaApplyError::Invalid(WorldError::ConservationFailure { .. }) => {
            TransitionRefusal::ConservationFailure
        }
        // An audit with violations is the same fact by another route: mass did not balance.
        DeltaApplyError::Audit(_) => TransitionRefusal::ConservationFailure,
        DeltaApplyError::SnapshotUnavailable(_) => TransitionRefusal::SnapshotUnavailable,
        DeltaApplyError::Invalid(_) => TransitionRefusal::ApplicationFailure,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matter::Rational;
    use crate::world::change::{Account, ComponentId, EarthworksEffect, LineageId};
    use crate::world::region::Coord;
    use crate::world::transition::{ActorId, PhysicalVerb, TargetRef};

    const TARGET: Coord = Coord::new(1, 2, 3);

    fn account(name: &str) -> Account {
        Account::new(name).expect("an account id")
    }

    fn component() -> ComponentId {
        ComponentId::new("rock").expect("a component id")
    }

    fn permission() -> WritePermission {
        WritePermission::granted(
            &Grant::new("the record", "region:first-band").expect("a scoped, attributed grant"),
        )
    }

    fn transition(nonce: u64) -> WorldTransition {
        WorldTransition::new(
            ActorId::new("buffy").expect("an actor"),
            None,
            PhysicalVerb::Dig,
            TargetRef::voxel(TARGET),
            None,
            31,
            nonce,
        )
    }

    /// A book holding 10 rock in `substrate` at the target, seeded through the test-only seam.
    fn book() -> DeltaStore {
        let mut store = DeltaStore::new();
        store.seed_for_tests(account("substrate"), component(), Rational::from_integer(10), TARGET);
        store
    }

    fn dig() -> EarthworksEffect {
        EarthworksEffect::excavation(
            TARGET,
            account("substrate"),
            account("spoil:ala"),
            component(),
            Rational::from_integer(4),
            LineageId::from_parts(["dig", "1"]).expect("a lineage"),
        )
        .expect("an excavation forms")
    }

    #[test]
    fn a_committed_change_leaves_a_receipt_and_an_evidence_row() {
        let mut store = book();
        let receipt = store
            .apply_transition(&transition(1), &dig(), &permission())
            .expect("the change commits");
        assert!(receipt.conserved, "a committed change conserved by construction");
        assert_eq!(receipt.granted_by, "the record");
        assert_eq!(receipt.transition, transition(1).id());
        assert_eq!(receipt.deltas, 1, "one transfer moved the matter");
        // The row is in the receipt and in the store, and it is the same row.
        assert_eq!(
            store.evidence_for_transition(receipt.transition),
            vec![receipt.evidence.clone()]
        );
        // And it names where, because the target named a voxel.
        assert!(receipt.evidence.message.contains("(1,2,3)"), "{}", receipt.evidence.message);
    }

    #[test]
    fn a_committed_change_moves_the_mass_and_the_books_balance() {
        let mut store = book();
        store
            .apply_transition(&transition(1), &dig(), &permission())
            .expect("the change commits");
        assert_eq!(
            store.component_amount(&account("substrate"), &component()).unwrap(),
            Rational::from_integer(6)
        );
        assert_eq!(
            store.component_amount(&account("spoil:ala"), &component()).unwrap(),
            Rational::from_integer(4)
        );
        assert_eq!(
            store.conservation_snapshot().unwrap().total_component_mass().unwrap(),
            Rational::from_integer(10),
            "a change moves mass, it does not make it"
        );
    }

    #[test]
    fn the_same_act_is_never_applied_twice() {
        let mut store = book();
        let effect = dig();
        let first = store
            .apply_transition(&transition(7), &effect, &permission())
            .expect("commits once");
        let before = store.clone();
        assert_eq!(
            store.apply_transition(&transition(7), &effect, &permission()),
            Err(TransitionRefusal::DuplicateTransition)
        );
        assert_eq!(store, before, "a refused change changes nothing");
        assert_eq!(
            store.evidence_for_transition(first.transition).len(),
            1,
            "and it leaves no second row"
        );
        // The mass moved once, not twice.
        assert_eq!(
            store.component_amount(&account("spoil:ala"), &component()).unwrap(),
            Rational::from_integer(4)
        );
    }

    #[test]
    fn a_different_nonce_is_a_different_act() {
        let mut store = book();
        let effect = dig();
        store
            .apply_transition(&transition(1), &effect, &permission())
            .expect("the first act commits");
        // Not a duplicate: the identity carries the nonce, so a second act at the same target
        // with the same verb is a second act. Four rock, twice, out of a book holding ten.
        store
            .apply_transition(&transition(2), &effect, &permission())
            .expect("a repeated intent is not a repeated act");
        assert_eq!(
            store.component_amount(&account("spoil:ala"), &component()).unwrap(),
            Rational::from_integer(8)
        );
    }

    #[test]
    fn an_overdraft_is_a_conservation_failure_and_changes_nothing() {
        let mut store = book();
        let overdraft = EarthworksEffect::excavation(
            TARGET,
            account("substrate"),
            account("spoil:ala"),
            component(),
            Rational::from_integer(11),
            LineageId::from_parts(["dig", "1"]).expect("a lineage"),
        )
        .expect("the effect forms; the book is what refuses it");
        let before = store.clone();
        assert_eq!(
            store.apply_transition(&transition(1), &overdraft, &permission()),
            Err(TransitionRefusal::ConservationFailure),
            "eleven rock out of an account holding ten"
        );
        assert_eq!(store, before);
        assert!(store.evidence_for_transition(transition(1).id()).is_empty());
    }

    #[test]
    fn a_claim_must_exist_and_belong_to_the_actor_who_spends_it() {
        let mut store = book();
        let reservation = ReservationId::new("res-1").expect("a reservation id");
        store
            .reserve(reservation.clone(), ActorId::new("someone-else").unwrap(), TargetRef::voxel(TARGET))
            .expect("a claim");
        let mut naming = transition(3);
        naming.reservation = Some(reservation);
        assert_eq!(
            store.apply_transition(&naming, &dig(), &permission()),
            Err(TransitionRefusal::UnownedReservation)
        );

        let mut missing = transition(4);
        missing.reservation = Some(ReservationId::new("never-declared").expect("an id"));
        assert_eq!(
            store.apply_transition(&missing, &dig(), &permission()),
            Err(TransitionRefusal::MissingReservation)
        );
    }

    #[test]
    fn a_claim_on_one_target_does_not_authorize_a_change_to_another() {
        let mut store = book();
        let reservation = ReservationId::new("res-1").expect("a reservation id");
        store
            .reserve(
                reservation.clone(),
                ActorId::new("buffy").unwrap(),
                TargetRef::voxel(Coord::new(9, 9, 9)),
            )
            .expect("a claim on another place");
        let mut elsewhere = transition(5);
        elsewhere.reservation = Some(reservation);
        assert_eq!(
            store.apply_transition(&elsewhere, &dig(), &permission()),
            Err(TransitionRefusal::UnownedReservation),
            "a reservation is a claim on one target, not a licence"
        );
        // And the claim is still there, unspent: a refused change does not consume it.
        assert_eq!(
            store.reservation_target(&ReservationId::new("res-1").unwrap()),
            Some("voxel:9,9,9")
        );
    }

    #[test]
    fn a_spent_claim_is_named_as_spent_rather_than_missing() {
        let mut store = book();
        let reservation = ReservationId::new("res-1").expect("a reservation id");
        store
            .reserve(reservation.clone(), ActorId::new("buffy").unwrap(), TargetRef::voxel(TARGET))
            .expect("a claim");
        let mut claiming = transition(6);
        claiming.reservation = Some(reservation.clone());
        store
            .apply_transition(&claiming, &dig(), &permission())
            .expect("the claimed act commits, spending the claim");
        // A second act naming the spent claim is `ReservationCommitted`, not `Missing` and not
        // `Unowned` — three different facts, three different names.
        let mut again = transition(8);
        again.reservation = Some(reservation);
        assert_eq!(
            store.apply_transition(&again, &dig(), &permission()),
            Err(TransitionRefusal::ReservationCommitted)
        );
    }

    #[test]
    fn a_permission_carries_its_author_and_its_bound() {
        let permitted = permission();
        assert_eq!(permitted.granter(), "the record");
        assert_eq!(permitted.scope(), "region:first-band");
        assert_eq!(permitted.to_string(), "the record over region:first-band");
    }

    #[test]
    fn the_receipt_survives_the_change_it_describes() {
        // The receipt's digest is the state the change left, so a later change does not make the
        // earlier receipt describe the wrong world.
        let mut store = book();
        let first = store
            .apply_transition(&transition(1), &dig(), &permission())
            .expect("commits");
        let after_first = store.conservation_snapshot().unwrap().digest;
        assert_eq!(first.audit_digest, after_first);
        store
            .apply_transition(&transition(2), &dig(), &permission())
            .expect("commits");
        assert_ne!(
            store.conservation_snapshot().unwrap().digest,
            first.audit_digest,
            "the world moved on, and the receipt still points at what it described"
        );
    }
}
