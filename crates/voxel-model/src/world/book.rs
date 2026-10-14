//! The governed book: the accounts, reservations and evidence a world change is decided against.
//!
//! Transcribed from `ala-cities/src/world/delta.rs` — the store (`DeltaStore`, lines 891–1450),
//! the conservation snapshot (`1450–1560`), the commit path (`apply_with_id`/`apply_validated`,
//! `1560–1830`) and the audit (`audit_snapshots` and the provenance audit, `1830–2125`) — **seams
//! only**, and the runtime-parts half is deliberately **not here**: the source's
//! `WorldFixture`/`WorldFixtureParts`/`fixture_parts`/`restore_runtime_parts` are its persistence
//! mechanism, and this tree's save doctrine is *the record + seed + tick* (HYPERGRILLING
//! **H-7.16**, **H-7.20**; `docs/LEDGER.md` **C-96.1**). The book is **resident working state**;
//! the durable change is the earthworks entry and the durable decision is the chain's, so when a
//! load path lands the book is rebuilt from the record, not restored from a second artifact.
//!
//! # What a book is, and what it is not
//!
//! It is: `accounts` (and the `account_components` key index), `account_coords` (which account
//! owns a coordinate), `occupancy` and `occupancy_provenance`, the `reservations` book, the
//! `evidence` rows, `committed_deltas`, and `applied_ids` — the set that makes **exactly-once**
//! expressible. It is not a record of the world: the voxels are a *projection* of the generator
//! composed with the earthworks record, and this store never computes them.
//!
//! **The ingest is the world's side.** Which account owns a coordinate and what it holds arrives
//! as data — the source fills it from a `WorldFixture`; this tree has no fixture machinery, so a
//! caller (a test today, the game's load path when the switch flips) supplies it, and the store
//! reads it and never assumes it. A coordinate with no account is refused (`MissingAccount` at
//! the handle), never invented.
//!
//! # The governed subset, measured
//!
//! The source's snapshot and audit carry the **generator's** subjects as well: `fluid_mass`,
//! `fluid_species`, `water_cycle_species`, `fields`, `strata`, `populations`, and the lifecycle
//! totals (`active`/`inactive`/`retired`/`aggregate`). Those are fields, hydrology and ecology —
//! `crates/voxel-shapers`' generation, contracted by `docs/SHAPERS.md` — and the machine names
//! **none** of them (measured: `agency/`, `gov/` and `sim/` read `snapshot.digest`,
//! `snapshot.occupancy`, `snapshot.occupancy_provenance`, and `ConservationSnapshot` itself).
//! So the snapshot here is the governed subset — component totals and per-account holdings, the
//! account-to-coordinate ownership, occupancy and its provenance, and the digest over exactly
//! those — and the audit's violations are the governed ones. `docs/CIVILIAN-PORT.md` carries the
//! same split, and `event-0143` is where the vocabulary split landed.
//!
//! # `Evidence` is the world's row, and `ticket::Evidence` is a different subject
//!
//! This was read, not assumed, because the two share a name. The world's row is
//! `{ kind, message, digest, transition_id, reservation }` (`world/delta.rs:753`) — what a
//! transition *did*, and what `laya` will grade. `ticket::Evidence` is
//! `{ id, ticket, origin, detail }` (`crate::ticket`, `ticket.rs:580`) — one piece of evidence
//! **with its origin on the record, citing a ticket**. Neither is a rename of the other: the
//! world's row is a change's own account of itself; the ticket's is a claim filed against a
//! ticket. They are two types because they answer two questions.
use crate::endian::LeBytes;

use std::collections::{BTreeMap, BTreeSet};

use crate::identity::Digest;
use crate::matter::Rational;

use super::change::{canonical_deltas, Account, ComponentId, Delta, Occupancy, OccupancyProvenance};
use super::region::Coord;
use super::transition::ReservationId;
use super::WorldError;

/// One row of world evidence: what a transition did, in one line, with the transition's identity.
///
/// Not [`crate::ticket::Evidence`] — see the module documentation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Evidence {
    pub kind: String,
    pub message: String,
    pub digest: Digest,
    pub transition_id: Option<Digest>,
    pub reservation: Option<ReservationId>,
}

impl Evidence {
    pub fn new(kind: impl Into<String>, message: impl Into<String>) -> Self {
        let kind = kind.into();
        let message = message.into();
        let digest = Digest::of_fields(
            "shploingus/world/evidence/v1",
            &[kind.as_bytes(), message.as_bytes()],
        );
        Self {
            kind,
            message,
            digest,
            transition_id: None,
            reservation: None,
        }
    }

    pub fn for_transition(
        kind: impl Into<String>,
        message: impl Into<String>,
        transition_id: Digest,
        reservation: Option<ReservationId>,
    ) -> Self {
        let mut evidence = Self::new(kind, message);
        evidence.transition_id = Some(transition_id);
        evidence.reservation = reservation;
        evidence
    }
}

/// The governed state of the world's books, as a value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConservationSnapshot {
    pub component_totals: BTreeMap<ComponentId, Rational>,
    pub component_accounts: BTreeMap<(Account, ComponentId), Rational>,
    pub account_coords: BTreeMap<Account, Coord>,
    pub occupancy: BTreeMap<Coord, Occupancy>,
    pub occupancy_provenance: BTreeMap<Coord, OccupancyProvenance>,
    pub digest: Digest,
}

impl ConservationSnapshot {
    /// The total mass held across every account, by component.
    pub fn total_component_mass(&self) -> Result<Rational, WorldError> {
        sum(self.component_totals.values().copied(), "component mass")
    }

    /// The digest this snapshot's **own data** computes, for checking the stored one.
    ///
    /// The source does the same check (`audit_snapshots` recomputes and compares); the difference
    /// is only what is folded in — the governed subset, under this tree's domain string.
    pub fn computed_digest(&self) -> Digest {
        let mut payload = Vec::new();
        for (component, amount) in &self.component_totals {
            append(&mut payload, component.as_str().as_bytes());
            append_rational(&mut payload, *amount);
        }
        for ((account, component), amount) in &self.component_accounts {
            append(&mut payload, account.as_str().as_bytes());
            append(&mut payload, component.as_str().as_bytes());
            append_rational(&mut payload, *amount);
        }
        for (account, coord) in &self.account_coords {
            append(&mut payload, account.as_str().as_bytes());
            append_coord(&mut payload, *coord);
        }
        for (coord, occupancy) in &self.occupancy {
            append_coord(&mut payload, *coord);
            append_rational(&mut payload, occupancy.value());
        }
        for (coord, proof) in &self.occupancy_provenance {
            append_coord(&mut payload, *coord);
            append_occupancy_provenance(&mut payload, proof);
        }
        Digest::of_fields(
            "shploingus/world/conservation-snapshot/v1",
            &[&payload],
        )
    }
}

fn optional_rational(value: Option<Rational>) -> Rational {
    match value {
        Some(value) => value,
        None => Rational::ZERO,
    }
}

fn rational_or_zero(value: Option<&Rational>) -> Rational {
    match value {
        Some(value) => *value,
        None => Rational::ZERO,
    }
}

fn sum(mut values: impl Iterator<Item = Rational>, label: &str) -> Result<Rational, WorldError> {
    values.try_fold(Rational::ZERO, |total, value| {
        total
            .checked_add(&value)
            .ok_or(WorldError::ConservationFailure {
                reason: format!("{label} overflow"),
            })
    })
}

/// What a conservation audit found. The governed violations, whole.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConservationViolation {
    ComponentChanged {
        component: ComponentId,
        before: Rational,
        after: Rational,
    },
    ComponentAccountChanged {
        account: Account,
        component: ComponentId,
        before: Rational,
        after: Rational,
    },
    OccupancyChanged {
        coord: Coord,
        before: Occupancy,
        after: Occupancy,
    },
    OccupancyProvenanceChanged {
        coord: Coord,
        before: Option<Box<OccupancyProvenance>>,
        after: Option<Box<OccupancyProvenance>>,
    },
    MissingOccupancyProvenance {
        coord: Coord,
    },
    OrphanOccupancyProvenance {
        coord: Coord,
    },
    InvalidOccupancyProvenance {
        coord: Coord,
        reason: String,
    },
    SnapshotDigestChanged,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConservationAudit {
    pub before: ConservationSnapshot,
    pub after: ConservationSnapshot,
    pub deltas: Vec<Delta>,
    pub violations: Vec<ConservationViolation>,
}

impl ConservationAudit {
    pub fn is_conserved(&self) -> bool {
        self.violations.is_empty()
    }

    pub fn between(
        before: ConservationSnapshot,
        after: ConservationSnapshot,
        deltas: Vec<Delta>,
    ) -> Self {
        audit_snapshots(before, after, canonical_deltas(&deltas), &[])
    }
}

/// Why a batch could not be committed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeltaApplyError {
    Invalid(WorldError),
    SnapshotUnavailable(WorldError),
    Audit(Box<ConservationAudit>),
}

/// A reservation, as the book keeps it. Private on purpose: the public face is
/// [`DeltaStore::reservation_owner`] and friends, and the source's own snapshot type for it is
/// part of the persistence half this port does not carry.
#[derive(Clone, Debug, PartialEq, Eq)]
struct ReservationRecord {
    owner: String,
    target: String,
    committed: bool,
}

/// The governed book. See the module documentation for what it is and what it is not.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DeltaStore {
    /// The single component ledger; `account_components` is only its typed key index.
    accounts: BTreeMap<Account, BTreeMap<ComponentId, Rational>>,
    account_components: BTreeMap<Account, BTreeSet<ComponentId>>,
    account_coords: BTreeMap<Account, Coord>,
    occupancy: BTreeMap<Coord, Occupancy>,
    occupancy_provenance: BTreeMap<Coord, OccupancyProvenance>,
    evidence: Vec<Evidence>,
    reservations: BTreeMap<String, ReservationRecord>,
    committed_deltas: Vec<Delta>,
    applied_ids: BTreeSet<Digest>,
}

impl DeltaStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// How much of a component an account holds. An account with no row holds zero — the source's
    /// reading, kept: absence is zero here and only here, never a default elsewhere.
    pub fn component_amount(
        &self,
        account: &Account,
        component: &ComponentId,
    ) -> Result<Rational, WorldError> {
        let amount = self
            .accounts
            .get(account)
            .and_then(|values| values.get(component))
            .copied();
        Ok(optional_rational(amount))
    }

    /// The component an account is keyed by — the first, deterministically (a `BTreeSet`).
    pub fn component_at(&self, account: &Account) -> Option<ComponentId> {
        self.account_components
            .get(account)
            .and_then(|values| values.iter().next())
            .cloned()
    }

    pub fn occupancy_at(&self, coord: Coord) -> Occupancy {
        match self.occupancy.get(&coord).copied() {
            Some(value) => value,
            None => Occupancy::empty(),
        }
    }

    pub fn account_for_coord(&self, coord: Coord) -> Option<Account> {
        self.account_coords
            .iter()
            .find(|(_, value)| **value == coord)
            .map(|(account, _)| account.clone())
    }

    /// Has this transition already been committed? The exactly-once check's memory.
    pub fn has_applied(&self, id: Digest) -> bool {
        self.applied_ids.contains(&id)
    }

    pub fn has_reservation(&self, id: &ReservationId) -> bool {
        self.reservations
            .get(id.as_str())
            .is_some_and(|record| !record.committed)
    }

    pub fn has_reservation_for(&self, target: &super::transition::TargetRef) -> bool {
        self.reservations
            .values()
            .any(|record| !record.committed && record.target == target.as_str())
    }

    pub fn reserve(
        &mut self,
        id: ReservationId,
        owner: super::transition::ActorId,
        target: super::transition::TargetRef,
    ) -> Result<(), WorldError> {
        if self.reservations.contains_key(id.as_str()) {
            return Err(WorldError::InvalidTransition {
                reason: format!("reservation {} already exists", id.as_str()),
            });
        }
        self.reservations.insert(
            id.as_str().to_owned(),
            ReservationRecord {
                owner: owner.as_str().to_owned(),
                target: target.as_str().to_owned(),
                committed: false,
            },
        );
        Ok(())
    }

    pub fn reservation_committed(&self, id: &ReservationId) -> bool {
        self.reservations
            .get(id.as_str())
            .is_some_and(|record| record.committed)
    }

    pub fn reservation_owner(&self, id: &ReservationId) -> Option<&str> {
        self.reservations
            .get(id.as_str())
            .filter(|record| !record.committed)
            .map(|record| record.owner.as_str())
    }

    pub fn reservation_target(&self, id: &ReservationId) -> Option<&str> {
        self.reservations
            .get(id.as_str())
            .filter(|record| !record.committed)
            .map(|record| record.target.as_str())
    }

    /// Mark a reservation spent. A reservation may be committed **once**; a second attempt is
    /// refused rather than ignored.
    pub fn consume_reservation(&mut self, id: &ReservationId) -> Result<(), WorldError> {
        let record =
            self.reservations
                .get_mut(id.as_str())
                .ok_or(WorldError::InvalidTransition {
                    reason: "unknown reservation".to_owned(),
                })?;
        if record.committed {
            return Err(WorldError::InvalidTransition {
                reason: "reservation already committed".to_owned(),
            });
        }
        record.committed = true;
        Ok(())
    }

    /// Cancel a claim that was never spent. **A committed reservation can never be cancelled** —
    /// the change it stood for happened, and un-claiming it would rewrite that.
    pub fn cancel_reservation(&mut self, id: &ReservationId) -> Result<(), WorldError> {
        match self.reservations.get(id.as_str()) {
            Some(record) if record.committed => Err(WorldError::InvalidTransition {
                reason: "committed reservation cannot be cancelled".to_owned(),
            }),
            Some(_) => {
                self.reservations.remove(id.as_str());
                Ok(())
            }
            None => Err(WorldError::InvalidTransition {
                reason: "unknown reservation".to_owned(),
            }),
        }
    }

    pub fn evidence(&self) -> &[Evidence] {
        &self.evidence
    }

    pub fn evidence_for(&self, reservation: &ReservationId) -> Vec<Evidence> {
        self.evidence
            .iter()
            .filter(|row| row.reservation.as_ref() == Some(reservation))
            .cloned()
            .collect()
    }

    pub fn evidence_for_transition(&self, id: Digest) -> Vec<Evidence> {
        self.evidence
            .iter()
            .filter(|row| row.transition_id == Some(id))
            .cloned()
            .collect()
    }

    pub(crate) fn append_evidence(&mut self, evidence: Evidence) {
        self.evidence.push(evidence);
    }

    /// The governed snapshot: totals, holdings, ownership, occupancy, provenance, digest.
    pub fn conservation_snapshot(&self) -> Result<ConservationSnapshot, WorldError> {
        let mut component_totals: BTreeMap<ComponentId, Rational> = BTreeMap::new();
        let mut component_accounts = BTreeMap::new();
        for (account, values) in &self.accounts {
            for (component, amount) in values {
                component_accounts.insert((account.clone(), component.clone()), *amount);
                let total = component_totals.get(component).copied();
                let next = match total {
                    Some(value) => value.checked_add(amount),
                    None => Some(*amount),
                }
                .ok_or(WorldError::ConservationFailure {
                    reason: "component total overflow".to_owned(),
                })?;
                component_totals.insert(component.clone(), next);
            }
        }
        let mut snapshot = ConservationSnapshot {
            component_totals,
            component_accounts,
            account_coords: self.account_coords.clone(),
            occupancy: self.occupancy.clone(),
            occupancy_provenance: self.occupancy_provenance.clone(),
            digest: Digest::from_bytes([0; 32]),
        };
        snapshot.digest = snapshot.computed_digest();
        Ok(snapshot)
    }

    /// Commit a batch, or change nothing.
    ///
    /// The whole store is cloned first and restored on **any** failure — the source's discipline,
    /// kept exactly: a half-applied batch is never observable, and the audit runs against
    /// `before`/`after` values, not against a guess.
    pub(crate) fn apply_with_id(
        &mut self,
        deltas: &[Delta],
        transition_id: Option<Digest>,
        reservation: Option<&ReservationId>,
    ) -> Result<ConservationAudit, DeltaApplyError> {
        let original = self.clone();
        let deltas = canonical_deltas(deltas);
        let before = self
            .conservation_snapshot()
            .map_err(DeltaApplyError::SnapshotUnavailable)?;
        if let Err(error) = self.apply_validated(&deltas, transition_id) {
            *self = original;
            return Err(DeltaApplyError::Invalid(error));
        }
        let after = self.conservation_snapshot().map_err(|error| {
            *self = original.clone();
            DeltaApplyError::SnapshotUnavailable(error)
        })?;
        let audit = audit_snapshots(before, after, deltas.clone(), &self.committed_deltas);
        if !audit.violations.is_empty() {
            *self = original;
            return Err(DeltaApplyError::Audit(Box::new(audit)));
        }
        if let Some(reservation) = reservation {
            if let Err(error) = self.consume_reservation(reservation) {
                *self = original;
                return Err(DeltaApplyError::Invalid(error));
            }
        }
        self.committed_deltas.extend(deltas.iter().cloned());
        if let Some(id) = transition_id {
            self.applied_ids.insert(id);
        }
        Ok(audit)
    }

    /// The governed arms of the source's `apply_validated`.
    ///
    /// Note one faithful oddity: the account/occupancy maps are mutated as **locals** and written
    /// back only when every delta validated, while `account_components` (an index, not a value) is
    /// updated in place. That is the source's own structure; `apply_with_id` restores the whole
    /// store on failure, so a half-updated index is never observable.
    /// Seed the book directly, **for tests in sibling modules**.
    ///
    /// `book.rs`'s own tests reach these fields because a test module is a child of the module
    /// that owns them; a sibling module's tests cannot, and duplicating the seeding there would
    /// be a second copy of the book's shape.
    ///
    /// **Test-only on purpose.** The production book gains mass only by **moving** it —
    /// `MatterTransfer` conserves and refuses an overdraft — so the *initial* deposit is the
    /// source's fixture path (`WorldFixture`/`fixture_parts`), which this port does not carry
    /// (H-7.20 excludes the persistence half). A production seeding entry point nothing calls
    /// would be surface invented for a test, so the seam is `#[cfg(test)]` and says so.
    #[cfg(test)]
    pub(crate) fn seed_for_tests(
        &mut self,
        account: Account,
        component: ComponentId,
        amount: Rational,
        coord: Coord,
    ) {
        self.accounts
            .entry(account.clone())
            .or_default()
            .insert(component.clone(), amount);
        self.account_components
            .entry(account.clone())
            .or_default()
            .insert(component);
        self.account_coords.insert(account, coord);
    }

    fn apply_validated(
        &mut self,
        deltas: &[Delta],
        transition_id: Option<Digest>,
    ) -> Result<(), WorldError> {
        let sorted = canonical_deltas(deltas);
        for delta in &sorted {
            delta.validate()?;
        }
        let mut accounts = self.accounts.clone();
        let mut occupancy = self.occupancy.clone();
        let mut provenance = self.occupancy_provenance.clone();
        for delta in &sorted {
            match delta {
                Delta::MatterTransfer {
                    component,
                    from,
                    to,
                    amount,
                    ..
                } => {
                    let source = optional_rational(
                        accounts
                            .get(from)
                            .and_then(|values| values.get(component))
                            .copied(),
                    );
                    let remainder =
                        source
                            .checked_sub(amount)
                            .ok_or(WorldError::ConservationFailure {
                                reason: format!("component account {from:?} underflow"),
                            })?;
                    if remainder.is_negative() {
                        return Err(WorldError::ConservationFailure {
                            reason: format!("component account {from:?} underflow"),
                        });
                    }
                    let destination = optional_rational(
                        accounts.get(to).and_then(|values| values.get(component)).copied(),
                    );
                    let next =
                        destination
                            .checked_add(amount)
                            .ok_or(WorldError::ConservationFailure {
                                reason: "component destination overflow".to_owned(),
                            })?;
                    accounts
                        .entry(from.clone())
                        .or_default()
                        .insert(component.clone(), remainder);
                    accounts
                        .entry(to.clone())
                        .or_default()
                        .insert(component.clone(), next);
                    self.account_components
                        .entry(from.clone())
                        .or_default()
                        .insert(component.clone());
                    self.account_components
                        .entry(to.clone())
                        .or_default()
                        .insert(component.clone());
                }
                Delta::OccupancyChange {
                    coord,
                    before,
                    after,
                    provenance: proof,
                } => {
                    if transition_id != Some(proof.transition_id) {
                        return Err(WorldError::InvalidTransition {
                            reason: "occupancy transition provenance mismatch".to_owned(),
                        });
                    }
                    let matching = sorted.iter().any(|candidate| match candidate {
                        Delta::MatterTransfer {
                            amount,
                            from,
                            to,
                            lineage,
                            ..
                        } => {
                            amount.is_positive()
                                && lineage == &proof.lineage
                                && (from == &proof.account || to == &proof.account)
                                && self.account_coords.get(&proof.account) == Some(coord)
                        }
                        _ => false,
                    });
                    if !matching {
                        return Err(WorldError::InvalidTransition {
                            reason: format!(
                                "occupancy at {} lacks matching positive provenance",
                                super::coord_text(*coord)
                            ),
                        });
                    }
                    let current = match occupancy.get(coord).copied() {
                        Some(value) => value,
                        None => Occupancy::empty(),
                    };
                    if current != *before {
                        return Err(WorldError::InvalidTransition {
                            reason: format!(
                                "occupancy before-state mismatch at {}",
                                super::coord_text(*coord)
                            ),
                        });
                    }
                    occupancy.insert(*coord, *after);
                    provenance.insert(*coord, proof.clone());
                }
                Delta::EvidenceOutput { .. } => {}
            }
        }
        self.accounts = accounts;
        self.occupancy = occupancy;
        self.occupancy_provenance = provenance;
        Ok(())
    }
}

fn audit_provenance_snapshot(
    snapshot: &ConservationSnapshot,
    coord: Coord,
    deltas: &[Delta],
    violations: &mut Vec<ConservationViolation>,
) {
    let occupancy_exists = snapshot.occupancy.contains_key(&coord);
    let proof = snapshot.occupancy_provenance.get(&coord);
    match (occupancy_exists, proof) {
        (false, None) => {}
        (true, None) => {
            violations.push(ConservationViolation::MissingOccupancyProvenance { coord })
        }
        (false, Some(_)) => {
            violations.push(ConservationViolation::OrphanOccupancyProvenance { coord })
        }
        (true, Some(proof)) => {
            let owner_valid = match &proof.fluid_address {
                Some(address) => address.coord() == Some(coord) && address.as_str() == proof.account.as_str(),
                None => snapshot.account_coords.get(&proof.account) == Some(&coord),
            };
            if !owner_valid {
                violations.push(ConservationViolation::InvalidOccupancyProvenance {
                    coord,
                    reason: "provenance does not own the occupancy coordinate".to_owned(),
                });
            }
            if proof.transition_id != Digest::from_bytes([0; 32]) {
                let transfer_matches = deltas.iter().any(|delta| match delta {
                    Delta::MatterTransfer {
                        from,
                        to,
                        amount,
                        lineage,
                        ..
                    } => {
                        amount.is_positive()
                            && lineage == &proof.lineage
                            && proof.fluid_address.is_none()
                            && (from == &proof.account || to == &proof.account)
                            && snapshot.account_coords.get(&proof.account) == Some(&coord)
                    }
                    _ => false,
                });
                if !transfer_matches {
                    violations.push(ConservationViolation::InvalidOccupancyProvenance {
                        coord,
                        reason: "provenance has no matching positive transfer".to_owned(),
                    });
                }
            }
        }
    }
}

fn audit_snapshots(
    before: ConservationSnapshot,
    after: ConservationSnapshot,
    deltas: Vec<Delta>,
    historical_deltas: &[Delta],
) -> ConservationAudit {
    let deltas = canonical_deltas(&deltas);
    let mut audit_deltas = historical_deltas.to_vec();
    audit_deltas.extend(deltas.iter().cloned());
    let audit_deltas = canonical_deltas(&audit_deltas);
    let mut violations = Vec::new();
    if before.computed_digest() != before.digest {
        violations.push(ConservationViolation::SnapshotDigestChanged);
    }
    if after.computed_digest() != after.digest {
        violations.push(ConservationViolation::SnapshotDigestChanged);
    }
    let mut provenance_coords = BTreeSet::new();
    provenance_coords.extend(before.occupancy.keys().copied());
    provenance_coords.extend(before.occupancy_provenance.keys().copied());
    provenance_coords.extend(after.occupancy.keys().copied());
    provenance_coords.extend(after.occupancy_provenance.keys().copied());
    for coord in provenance_coords {
        audit_provenance_snapshot(&before, coord, &audit_deltas, &mut violations);
        audit_provenance_snapshot(&after, coord, &audit_deltas, &mut violations);
    }
    compare_map(
        &mut violations,
        &before.component_totals,
        &after.component_totals,
        |component, left, right| ConservationViolation::ComponentChanged {
            component: component.clone(),
            before: rational_or_zero(left),
            after: rational_or_zero(right),
        },
    );
    let mut account_keys = BTreeSet::new();
    account_keys.extend(before.component_accounts.keys().cloned());
    account_keys.extend(after.component_accounts.keys().cloned());
    for key in account_keys {
        if before.component_accounts.get(&key) != after.component_accounts.get(&key)
            && !audit_deltas.iter().any(|delta| match delta {
                Delta::MatterTransfer {
                    component,
                    from,
                    to,
                    ..
                } => (key.0 == *from || key.0 == *to) && key.1 == *component,
                _ => false,
            })
        {
            violations.push(ConservationViolation::ComponentAccountChanged {
                account: key.0.clone(),
                component: key.1.clone(),
                before: rational_or_zero(before.component_accounts.get(&key)),
                after: rational_or_zero(after.component_accounts.get(&key)),
            });
        }
    }
    let mut occupancy_coords = BTreeSet::new();
    occupancy_coords.extend(before.occupancy.keys().copied());
    occupancy_coords.extend(after.occupancy.keys().copied());
    for coord in occupancy_coords {
        if before.occupancy.get(&coord) != after.occupancy.get(&coord)
            && !audit_deltas.iter().any(
                |delta| matches!(delta, Delta::OccupancyChange { coord: changed, .. } if changed == &coord),
            )
        {
            violations.push(ConservationViolation::OccupancyChanged {
                coord,
                before: match before.occupancy.get(&coord) {
                    Some(value) => *value,
                    None => Occupancy::empty(),
                },
                after: match after.occupancy.get(&coord) {
                    Some(value) => *value,
                    None => Occupancy::empty(),
                },
            });
        }
    }
    let mut provenance_keys = BTreeSet::new();
    provenance_keys.extend(before.occupancy_provenance.keys().copied());
    provenance_keys.extend(after.occupancy_provenance.keys().copied());
    for coord in provenance_keys {
        if before.occupancy_provenance.get(&coord) != after.occupancy_provenance.get(&coord)
            && !audit_deltas.iter().any(
                |delta| matches!(delta, Delta::OccupancyChange { coord: changed, .. } if changed == &coord),
            )
        {
            violations.push(ConservationViolation::OccupancyProvenanceChanged {
                coord,
                before: before
                    .occupancy_provenance
                    .get(&coord)
                    .cloned()
                    .map(Box::new),
                after: after
                    .occupancy_provenance
                    .get(&coord)
                    .cloned()
                    .map(Box::new),
            });
        }
    }
    ConservationAudit {
        before,
        after,
        deltas: audit_deltas,
        violations,
    }
}

fn compare_map<K: Ord + Clone>(
    violations: &mut Vec<ConservationViolation>,
    before: &BTreeMap<K, Rational>,
    after: &BTreeMap<K, Rational>,
    mut make: impl FnMut(&K, Option<&Rational>, Option<&Rational>) -> ConservationViolation,
) {
    let mut keys = BTreeSet::new();
    keys.extend(before.keys().cloned());
    keys.extend(after.keys().cloned());
    for key in keys {
        if before.get(&key) != after.get(&key) {
            violations.push(make(&key, before.get(&key), after.get(&key)));
        }
    }
}

fn append(input: &mut Vec<u8>, bytes: &[u8]) {
    input.extend_from_slice(&(bytes.len() as u64).into_le_bytes());
    input.extend_from_slice(bytes);
}

fn append_rational(input: &mut Vec<u8>, value: Rational) {
    append(input, &value.numerator().into_le_bytes());
    append(input, &value.denominator().into_le_bytes());
}

fn append_coord(input: &mut Vec<u8>, coord: Coord) {
    append(input, &coord.x.into_le_bytes());
    append(input, &coord.y.into_le_bytes());
    append(input, &coord.z.into_le_bytes());
}

/// The source's compact provenance encoding, kept: a fluid address's coordinate is folded into
/// three bytes offset by 128, because this is a digest encoding and not a serialisation of record.
fn append_occupancy_provenance(input: &mut Vec<u8>, proof: &OccupancyProvenance) {
    append(input, proof.transition_id.as_bytes());
    append(input, proof.account.as_str().as_bytes());
    for part in proof.lineage.parts() {
        append(input, part.as_bytes());
    }
    match &proof.fluid_address {
        Some(address) => {
            append(input, address.as_str().as_bytes());
            append(input, &[address.species() as u8]);
            append(input, &[address.phase() as u8]);
            append(
                input,
                &address.coord().map_or([0_u8; 3], |coord| {
                    let mut bytes = [0_u8; 3];
                    bytes[0] = (coord.x + 128) as u8;
                    bytes[1] = (coord.y + 128) as u8;
                    bytes[2] = (coord.z + 128) as u8;
                    bytes
                }),
            );
        }
        None => {
            append(input, b"none");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::change::{EarthworksEffect, LineageId};
    use crate::world::transition::{ActorId, TargetRef};

    fn account(name: &str) -> Account {
        Account::new(name).unwrap()
    }

    fn component() -> ComponentId {
        ComponentId::new("rock").unwrap()
    }

    fn lineage() -> LineageId {
        LineageId::from_parts(["test", "1"]).unwrap()
    }

    /// A book with one substrate account owning one coordinate, holding 10 rock.
    fn book() -> DeltaStore {
        let mut store = DeltaStore::new();
        store
            .accounts
            .entry(account("substrate"))
            .or_default()
            .insert(component(), Rational::from_integer(10));
        store
            .account_components
            .entry(account("substrate"))
            .or_default()
            .insert(component());
        store
            .account_coords
            .insert(account("substrate"), Coord::new(1, 2, 3));
        store
    }

    fn transfer(amount: i128) -> Delta {
        Delta::MatterTransfer {
            component: component(),
            from: account("substrate"),
            to: account("carried:ala"),
            amount: Rational::from_integer(amount),
            lineage: lineage(),
        }
    }

    #[test]
    fn a_matter_transfer_moves_mass_and_conserves_it() {
        let mut store = book();
        let audit = store
            .apply_with_id(&[transfer(3)], Some(Digest::of(b"t1")), None)
            .expect("the transfer is valid");
        assert!(audit.is_conserved());
        assert_eq!(
            store.component_amount(&account("substrate"), &component()).unwrap(),
            Rational::from_integer(7)
        );
        assert_eq!(
            store.component_amount(&account("carried:ala"), &component()).unwrap(),
            Rational::from_integer(3)
        );
        let snapshot = store.conservation_snapshot().unwrap();
        assert_eq!(
            snapshot.total_component_mass().unwrap(),
            Rational::from_integer(10),
            "a transfer moves mass, it does not make it"
        );
        assert!(store.has_applied(Digest::of(b"t1")));
    }

    #[test]
    fn an_overdraft_is_refused_and_nothing_changes() {
        let mut store = book();
        let error = store
            .apply_with_id(&[transfer(11)], Some(Digest::of(b"t1")), None)
            .expect_err("an overdraft is refused");
        assert!(matches!(error, DeltaApplyError::Invalid(_)));
        assert_eq!(
            store.component_amount(&account("substrate"), &component()).unwrap(),
            Rational::from_integer(10),
            "the store is restored on refusal"
        );
        assert!(!store.has_applied(Digest::of(b"t1")));
    }

    #[test]
    fn a_reservation_commits_once_and_a_committed_one_is_never_cancelled() {
        let mut store = book();
        let id = ReservationId::new("agency:ala:1:1").unwrap();
        store
            .reserve(
                id.clone(),
                ActorId::new("ala").unwrap(),
                TargetRef::voxel(Coord::new(1, 2, 3)),
            )
            .unwrap();
        assert!(store.has_reservation(&id));
        assert!(store.has_reservation_for(&TargetRef::voxel(Coord::new(1, 2, 3))));
        assert_eq!(store.reservation_owner(&id), Some("ala"));

        store.consume_reservation(&id).unwrap();
        assert!(store.reservation_committed(&id));
        assert!(store.consume_reservation(&id).is_err(), "once, not twice");
        assert!(
            store.cancel_reservation(&id).is_err(),
            "a committed reservation can never be cancelled"
        );
        assert_eq!(store.reservation_owner(&id), None, "committed is spent");
    }

    #[test]
    fn a_cancelled_reservation_is_gone_and_can_be_reclaimed() {
        let mut store = book();
        let id = ReservationId::new("r1").unwrap();
        store
            .reserve(
                id.clone(),
                ActorId::new("ala").unwrap(),
                TargetRef::voxel(Coord::new(0, 0, 0)),
            )
            .unwrap();
        store.cancel_reservation(&id).unwrap();
        assert!(!store.has_reservation(&id));
        assert!(store.cancel_reservation(&id).is_err(), "cancelling twice is not a no-op");
    }

    #[test]
    fn evidence_rows_are_readable_by_reservation_and_by_transition() {
        let mut store = book();
        let id = Digest::of(b"t1");
        let reservation = ReservationId::new("r1").unwrap();
        store.append_evidence(Evidence::for_transition(
            "transition.committed",
            "committed 1 delta",
            id,
            Some(reservation.clone()),
        ));
        store.append_evidence(Evidence::for_transition(
            "transition.refused",
            "no reservation",
            Digest::of(b"t2"),
            None,
        ));
        assert_eq!(store.evidence_for(&reservation).len(), 1);
        assert_eq!(store.evidence_for_transition(id).len(), 1);
        assert_eq!(store.evidence().len(), 2);
    }

    #[test]
    fn an_occupancy_change_needs_its_matching_transfer() {
        let mut store = book();
        let id = Digest::of(b"t1");
        let coord = Coord::new(1, 2, 3);
        // The world's side of the ingest: this coordinate is occupied, the substrate account owns
        // it, and its baseline provenance carries a zero transition id — the source's own baseline
        // marker, which is why the audit skips the transfer check for it. The book reads this; it
        // never invents it.
        store.occupancy.insert(coord, Occupancy::full());
        store.occupancy_provenance.insert(
            coord,
            OccupancyProvenance {
                transition_id: Digest::from_bytes([0; 32]),
                account: account("substrate"),
                lineage: lineage(),
                fluid_address: None,
            },
        );
        let transfer = transfer(1);
        let occupancy = Delta::OccupancyChange {
            coord,
            before: Occupancy::full(),
            after: Occupancy::empty(),
            provenance: OccupancyProvenance {
                transition_id: id,
                account: account("substrate"),
                lineage: lineage(),
                fluid_address: None,
            },
        };

        let mut without_transfer = book();
        let error = without_transfer
            .apply_with_id(std::slice::from_ref(&occupancy), Some(id), None)
            .expect_err("occupancy without a transfer has no provenance");
        assert!(matches!(error, DeltaApplyError::Invalid(_)));

        let audit = store
            .apply_with_id(&[transfer, occupancy], Some(id), None)
            .expect("the transfer justifies the occupancy");
        assert!(audit.is_conserved());
        let snapshot = store.conservation_snapshot().unwrap();
        assert_eq!(
            snapshot.occupancy.get(&coord).copied(),
            Some(Occupancy::empty())
        );
        assert_eq!(
            snapshot.occupancy_provenance.get(&coord).map(|p| p.transition_id),
            Some(id)
        );
    }

    #[test]
    fn a_snapshot_that_disagrees_with_its_own_digest_is_a_violation() {
        let store = book();
        let before = store.conservation_snapshot().unwrap();
        let mut tampered = before.clone();
        tampered.digest = Digest::from_bytes([0; 32]);
        let audit = ConservationAudit::between(before, tampered, Vec::new());
        assert!(audit
            .violations
            .contains(&ConservationViolation::SnapshotDigestChanged));
    }

    #[test]
    fn an_excavation_is_a_transfer_the_book_can_commit() {
        let mut store = book();
        let effect = EarthworksEffect::excavation(
            Coord::new(1, 2, 3),
            account("substrate"),
            account("spoil:ala"),
            component(),
            Rational::from_integer(4),
            lineage(),
        )
        .unwrap();
        let audit = store
            .apply_with_id(&effect.deltas, Some(Digest::of(b"dig")), None)
            .unwrap();
        assert!(audit.is_conserved());
        assert_eq!(
            store.component_amount(&account("spoil:ala"), &component()).unwrap(),
            Rational::from_integer(4)
        );
    }
}
