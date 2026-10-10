//! Earthworks — every change the world did not make itself.
//!
//! **Source.** The F10 ruling, not a paper: *"a recipe that is a mass pump must
//! declare itself one"*, and the conservation half of F1. This is the shaper that
//! exists so that `terrain` stays a **pure function**: anything a player or an
//! agent digs, builds or floods is not terrain, it is a **record**, and the world
//! is the generator's output *composed with* that record.
//!
//! # Why an append-only chain and not a map
//!
//! A `HashMap<Pos, Material>` of "what the world is now" cannot answer the
//! question this project keeps asking — *what changed, and what authorised it?*
//! Every modification here therefore carries:
//!
//! * the material it **replaced** and the material it **placed**;
//! * a **reason**, which is required (an unexplained change is not a record);
//! * a **mass ledger** (F1's exact set), so the change either balances or
//!   declares its pump.
//!
//! And the `from` field is checked against the chain: a modification whose `from`
//! is not what the world currently holds is a modification that **does not
//! connect**, and it is refused. That is what makes the record an audit rather
//! than a wish.

use voxel_model::conservation::{Exact, Ledger, Mass, MassPumpDeclaration, Source};
use voxel_model::world::{ActorId, PhysicalVerb, ReservationId, TaskId};
use voxel_model::{Digest, MaterialId, Ratio, RefusalKind, RefusalSet, Refused};

/// One recorded change.
///
/// # The governed fields, and why they are required rather than carried elsewhere
///
/// `SHAPERS.md` §1 row 15 says what this record is: *"**recorded** player/agent modification of
/// the world."* For a long time it recorded the *what* and the *why* and not the **who** — no
/// actor, no task, no verb, no claim declared before the change, and no identity — which meant
/// no reader could ask the question the record exists to answer: *what changed, and what
/// authorised it?*
///
/// The five fields below close that, and they live **here** rather than in a book beside the
/// record for one reason: `forbidden.second-record`. A parallel ledger of who-did-what drifts
/// from the record it describes, and the drift is invisible until the two disagree. The record
/// is the one place a change is stated, so it is the one place the change is attributed.
///
/// The user's ruling (2026-10-02) chose this over keeping the governed fields beside it, and
/// chose that a hand edit is **a governed act by the player as an actor** — hence
/// [`ActorId::player`] and a [`PhysicalVerb`] on `hand_change`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Modification {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    /// The material this change replaced.
    pub from: MaterialId,
    /// The material it placed. [`MaterialId::EMPTY`] is a removal, and digging is
    /// a legitimate modification.
    pub to: MaterialId,
    /// Why. Required; an empty reason is refused.
    pub reason: String,
    /// The mass ledger for this change.
    pub ledger: Ledger,
    /// **Who** changed it. Never absent: a change nobody is answerable for is not a record.
    pub actor: ActorId,
    /// The task this change serves. Absent for work that is not a job — including a hand edit.
    pub task: Option<TaskId>,
    /// **What kind of act** it was. The verb list is `voxel_model::world::PhysicalVerb`'s and
    /// is closed on purpose, so `break` and `dig` are told apart rather than both being "dug".
    pub verb: PhysicalVerb,
    /// The claim on the target, **declared before the change**. Absent for a hand edit, which
    /// cannot be declared for: a hand is not a plan.
    pub reservation: Option<ReservationId>,
    /// This change's identity, so exactly-once is expressible: [`Earthworks::record`] refuses a
    /// second entry carrying an id the record already holds.
    pub transition: Digest,
}

/// The identity of a by-hand change: **over its own content**, not over a transition's fields.
///
/// A [`voxel_model::world::WorldTransition`]'s id covers actor, verb, reservation, epoch and
/// nonce — and a hand edit has no reservation, epoch or nonce to tell a redo from an undo. So
/// the content is the identity: where, from what, to what, by whom, and how. Two hand edits
/// that differ in any of those are two entries; the same one twice is the same entry, and the
/// chain check in [`Earthworks::record`] refuses the repeat anyway, because a second claim to
/// replace `SOIL` where the record already holds `SOIL` no longer connects.
///
/// Public because a caller that builds an [`Modification`] **by hand** — the streaming and
/// column tests do, and so will any path that needs to control the ledger — must mint the
/// identity the builder would. Two spellings of one identity is how exactly-once stops meaning
/// anything.
pub fn hand_change_id(
    x: i32,
    y: i32,
    z: i32,
    from: MaterialId,
    to: MaterialId,
    actor: &ActorId,
    verb: PhysicalVerb,
) -> Digest {
    Digest::of_fields(
        "shploingus/earthworks/hand-change/v1",
        &[
            &x.to_be_bytes(),
            &y.to_be_bytes(),
            &z.to_be_bytes(),
            &from.0.to_be_bytes(),
            &to.0.to_be_bytes(),
            actor.as_str().as_bytes(),
            verb.as_str().as_bytes(),
        ],
    )
}

/// A hand change, **named rather than positional**.
///
/// Eight positional arguments would be four coordinate-shaped values in a row, two of which
/// share a type: `hand_change(x, y, z, from, to, …)` and `hand_change(x, y, from, z, to, …)`
/// both compile. Naming the fields is not ceremony — it is the difference between a transposed
/// coordinate being a type error and being a hole in the world.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct HandChange {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub from: MaterialId,
    pub to: MaterialId,
    pub reason: String,
    pub actor: ActorId,
    pub verb: PhysicalVerb,
}

/// One voxel of matter moved **by hand** — a dig, or a block placed.
///
/// The mass is the material's **own registry density**, not a chosen number: one
/// voxel is one cubic metre, so `bulk_density_kg_m3` *is* the figure, exactly.
/// A material the registry does not admit (air, and an id nobody registered)
/// contributes **no entry** rather than a zero one, because a zero-mass entry
/// records nothing and [`Ledger::audit`] refuses it.
///
/// The pump is declared because the world has no excavated store modelled yet:
/// the matter a dig removes has to *go* somewhere, and naming that place is the
/// difference between a ledger and an excuse. The day an inventory exists, this
/// is the function that changes, and the mass it recorded is the mass the
/// inventory starts from.
///
/// # Who, and what kind of act
///
/// A hand change **is** a governed act: the player is an actor (`ActorId::player`, the id the
/// DSL's `player = person "The player"` declares), and the verb says which act it was rather
/// than leaving it to the prose in `reason`. Nothing about "the player broke it" needs a
/// reservation, so there is none; a task is `None` for the same reason. The identity is minted
/// from the content — see [`hand_change_id`] — because a hand edit has no epoch or nonce to
/// distinguish a redo from an undo.
pub fn hand_change(change: HandChange) -> Refused<Modification> {
    let HandChange {
        x,
        y,
        z,
        from,
        to,
        reason,
        actor,
        verb,
    } = change;
    // **Water is not a mineable block (C-152.2).** A hand change may neither remove
    // a fluid voxel nor place one: removing it would delete water the world's ledger
    // counts (its mass would have to go somewhere, and the excavated store is not
    // water's home), and placing it would mint a fluid from nothing — the Minecraft
    // source block the user ruled out. The fluid has nowhere to go and nowhere to
    // come from while displacement is unmodelled, so the edit **refuses by name**
    // rather than pretending. `Pool::edit` adds the derived half: a lake is fluid
    // state with no material voxel (`fluid_state`), and an edit into one refuses too.
    for material in [from, to] {
        if crate::column::is_fluid(material) {
            return Err(crate::fluid_state::refuse_edit(x, y, z));
        }
    }
    let registry = crate::column::materials::registry()?;
    let mass_of = |m: MaterialId| -> Option<Ratio> {
        if m.is_empty() {
            return None;
        }
        registry
            .material(m)
            .ok()
            .map(|material| material.bulk_density_kg_m3())
            .filter(|density| !density.is_zero())
    };
    let store = || Source::Named("the excavated store".into());
    let mut ledger = Ledger::new();
    ledger.declare_pump(MassPumpDeclaration {
        reason: format!("manual earthworks: {reason}"),
        source: store(),
    });
    if let Some(mass) = mass_of(from) {
        ledger.withdraw(
            Mass::from_exact(Exact::new(mass)),
            store(),
            format!("one cubic metre of {} removed", from.0),
        );
    }
    if let Some(mass) = mass_of(to) {
        ledger.deposit(
            Mass::from_exact(Exact::new(mass)),
            store(),
            format!("one cubic metre of {} placed", to.0),
        );
    }
    let transition = hand_change_id(x, y, z, from, to, &actor, verb);
    Ok(Modification {
        x,
        y,
        z,
        from,
        to,
        reason,
        ledger,
        actor,
        task: None,
        verb,
        reservation: None,
        transition,
    })
}

/// The ordered record of every change to a world.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct Earthworks {
    entries: Vec<Modification>,
}

impl Earthworks {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entries(&self) -> &[Modification] {
        &self.entries
    }

    /// Record a change.
    ///
    /// Refuses, naming every reason, when the change records nothing, has no
    /// reason, breaks the chain, or carries a ledger that will not audit.
    pub fn record(&mut self, m: Modification) -> Result<(), RefusalSet> {
        let mut set = RefusalSet::new();

        if m.from == m.to {
            set.refuse(
                RefusalKind::OutOfDeclaredRange,
                format!(
                    "modification at ({}, {}, {}) replaces {:?} with itself and records nothing",
                    m.x, m.y, m.z, m.from
                ),
            );
        }
        if m.reason.trim().is_empty() {
            set.refuse(
                RefusalKind::MissingStage,
                format!("modification at ({}, {}, {}) has no reason", m.x, m.y, m.z),
            );
        }

        // **Exactly once.** The identity is on the entry, so a change that arrives twice — a
        // retried governed transition, a hand edit replayed from a queue — is refused rather
        // than recorded twice. This is the whole reason the id is a field and not a comment.
        if m.transition == Digest::from_bytes([0; 32]) {
            set.refuse(
                RefusalKind::MissingStage,
                format!(
                    "modification at ({}, {}, {}) carries no transition identity",
                    m.x, m.y, m.z
                ),
            );
        }
        if self.entries.iter().any(|e| e.transition == m.transition) {
            set.refuse(
                RefusalKind::RecordExists,
                format!(
                    "modification at ({}, {}, {}) carries transition {} that the record already holds",
                    m.x,
                    m.y,
                    m.z,
                    m.transition.to_hex()
                ),
            );
        }

        // The chain must connect: `from` has to be what the world holds now.
        let current = self.material_at(m.x, m.y, m.z, m.from);
        if !self.is_recorded(m.x, m.y, m.z) {
            // First change at this position: `from` is the caller's claim about
            // the generated world, which this record cannot check by itself and
            // therefore must not pretend to. It is accepted, and the acceptance
            // is what `is_recorded` reports afterwards.
        } else if current != m.from {
            set.refuse(
                RefusalKind::OutOfDeclaredRange,
                format!(
                    "modification at ({}, {}, {}) claims to replace {:?} but the record holds {:?}",
                    m.x, m.y, m.z, m.from, current
                ),
            );
        }

        if let Err(ledger_refusals) = m.ledger.audit() {
            set.absorb(ledger_refusals);
        }

        if !set.is_empty() {
            return Err(set);
        }

        self.entries.push(m);
        Ok(())
    }

    /// What the record says is at a position, or `base` (the generated material)
    /// where nothing has been recorded.
    ///
    /// **An unrecorded modification is invisible.** That is the property that
    /// makes the generator pure: if a change is not in the record, the world
    /// behaves as though it never happened, and the discrepancy is a bug in
    /// whatever made the change rather than a silent divergence.
    pub fn material_at(&self, x: i32, y: i32, z: i32, base: MaterialId) -> MaterialId {
        let mut out = base;
        for e in &self.entries {
            if e.x == x && e.y == y && e.z == z {
                out = e.to;
            }
        }
        out
    }

    pub fn is_recorded(&self, x: i32, y: i32, z: i32) -> bool {
        self.entries.iter().any(|e| e.x == x && e.y == y && e.z == z)
    }

    /// The history at a position, oldest first.
    pub fn history(&self, x: i32, y: i32, z: i32) -> Vec<&Modification> {
        self.entries
            .iter()
            .filter(|e| e.x == x && e.y == y && e.z == z)
            .collect()
    }

    /// Audit the whole record: every ledger must still audit.
    pub fn audit(&self) -> Result<(), RefusalSet> {
        let mut set = RefusalSet::new();
        for (i, e) in self.entries.iter().enumerate() {
            if let Err(r) = e.ledger.audit() {
                let mut r = r;
                r.refuse(
                    RefusalKind::OutOfDeclaredRange,
                    format!("entry {i} at ({}, {}, {})", e.x, e.y, e.z),
                );
                set.absorb(r);
            }
        }
        if set.is_empty() {
            Ok(())
        } else {
            Err(set)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STONE: MaterialId = MaterialId(4);
    const SOIL: MaterialId = MaterialId(3);

    fn modification(from: MaterialId, to: MaterialId, reason: &str) -> Modification {
        // A balanced ledger: a pump is declared so `audit` passes, and the reason
        // for it is the digging itself.
        let mut ledger = Ledger::new();
        // **Always** declare the pump. Digging to `EMPTY` still moves mass — it
        // moves it out of the world — and an earlier revision only declared the
        // pump when both sides were non-empty, which made every removal fail its
        // own audit.
        ledger.declare_pump(voxel_model::conservation::MassPumpDeclaration {
            reason: "earthworks moves material within the world".into(),
            source: voxel_model::conservation::Source::World,
        });
        ledger.deposit(
            voxel_model::conservation::Mass::from_exact(voxel_model::conservation::Exact::new(
                voxel_model::Ratio::from_u64(1),
            )),
            voxel_model::conservation::Source::World,
            "one cubic metre",
        );
        Modification {
            x: 1,
            y: 2,
            z: 3,
            from,
            to,
            reason: reason.into(),
            ledger,
            actor: ActorId::player(),
            task: None,
            verb: PhysicalVerb::Place,
            reservation: None,
            transition: hand_change_id(1, 2, 3, from, to, &ActorId::player(), PhysicalVerb::Place),
        }
    }

    #[test]
    fn an_unrecorded_modification_is_invisible() {
        let e = Earthworks::new();
        assert_eq!(e.material_at(1, 2, 3, STONE), STONE);
        assert!(!e.is_recorded(1, 2, 3));
    }

    #[test]
    fn a_recorded_modification_lands() {
        let mut e = Earthworks::new();
        e.record(modification(STONE, SOIL, "built a field")).unwrap();
        assert_eq!(e.material_at(1, 2, 3, STONE), SOIL);
        assert!(e.is_recorded(1, 2, 3));
        // Somewhere else is untouched.
        assert_eq!(e.material_at(9, 9, 9, STONE), STONE);
    }

    #[test]
    fn a_change_that_records_nothing_is_refused() {
        let mut e = Earthworks::new();
        let r = e.record(modification(STONE, STONE, "nothing")).unwrap_err();
        assert!(r.all().iter().any(|x| x.kind() == RefusalKind::OutOfDeclaredRange));
        assert!(e.is_empty());
    }

    #[test]
    fn a_change_with_no_reason_is_refused() {
        let mut e = Earthworks::new();
        let r = e.record(modification(STONE, SOIL, "   ")).unwrap_err();
        assert!(r.all().iter().any(|x| x.kind() == RefusalKind::MissingStage));
        assert!(e.is_empty());
    }

    #[test]
    fn the_chain_must_connect() {
        let mut e = Earthworks::new();
        e.record(modification(STONE, SOIL, "first")).unwrap();
        // The record now holds SOIL here, so a change claiming to replace STONE
        // is a change that does not connect.
        let bad = e.record(modification(STONE, MaterialId(5), "second"));
        assert!(bad.is_err(), "a disconnected modification was accepted");

        // Replacing what is actually there is fine, and the history is kept.
        e.record(modification(SOIL, MaterialId(5), "second")).unwrap();
        let h = e.history(1, 2, 3);
        assert_eq!(h.len(), 2);
        assert_eq!(h[0].to, SOIL);
        assert_eq!(h[1].to, MaterialId(5));
        assert_eq!(e.material_at(1, 2, 3, STONE), MaterialId(5));
    }

    #[test]
    fn digging_to_empty_is_a_legitimate_modification() {
        let mut e = Earthworks::new();
        e.record(modification(STONE, MaterialId::EMPTY, "quarry")).unwrap();
        assert_eq!(e.material_at(1, 2, 3, STONE), MaterialId::EMPTY);
    }

    #[test]
    fn a_hand_change_may_not_remove_or_place_a_fluid() {
        // C-152.2's filed defect, pinned: *"for some reason I can break water
        // blocks which is dumb as hell"*. Neither half of a hand change may be
        // water — a fluid is world state, and the edit refuses instead of deleting
        // it (or minting one, which is the Minecraft source block the user ruled
        // out).
        let water = crate::column::materials::WATER;

        let broken = hand_change(HandChange {
            x: 4,
            y: -3,
            z: 6,
            from: water,
            to: MaterialId::EMPTY,
            reason: "the player broke the sea".into(),
            actor: ActorId::player(),
            verb: PhysicalVerb::Break,
        })
        .unwrap_err();
        assert_eq!(broken.kind(), RefusalKind::FluidIsNotABlock);
        assert!(broken.detail().contains("targets water"), "got {broken}");

        let placed = hand_change(HandChange {
            x: 4,
            y: -3,
            z: 6,
            from: MaterialId::EMPTY,
            to: water,
            reason: "the player poured the sea".into(),
            actor: ActorId::player(),
            verb: PhysicalVerb::Place,
        })
        .unwrap_err();
        assert_eq!(placed.kind(), RefusalKind::FluidIsNotABlock);
        assert!(placed.detail().contains("displacement is not modelled"), "got {placed}");

        // And the refusal is not a general ban: rock still moves by hand.
        hand_change(HandChange {
            x: 4,
            y: -3,
            z: 6,
            from: STONE,
            to: MaterialId::EMPTY,
            reason: "the player quarried".into(),
            actor: ActorId::player(),
            verb: PhysicalVerb::Dig,
        })
        .expect("rock is still a block");
    }

    #[test]
    fn the_record_audits_its_ledgers() {
        let mut e = Earthworks::new();
        e.record(modification(STONE, SOIL, "ok")).unwrap();
        assert!(e.audit().is_ok());
    }

    #[test]
    fn a_hand_change_carries_the_materials_own_mass_and_names_the_store() {
        // One cubic metre of igneous rock, at the registry's own density: the
        // number is the density, and the ledger audits.
        let igneous = crate::column::materials::IGNEOUS;
        let m = hand_change(HandChange {
            x: 4,
            y: 5,
            z: 6,
            from: MaterialId::EMPTY,
            to: igneous,
            reason: "a block placed".into(),
            actor: ActorId::player(),
            verb: PhysicalVerb::Place,
        })
        .unwrap();
        let mut e = Earthworks::new();
        e.record(m.clone()).unwrap();
        assert!(e.audit().is_ok());
        let density = crate::column::materials::registry()
            .unwrap()
            .material(igneous)
            .unwrap()
            .bulk_density_kg_m3();
        let listed: Vec<Ratio> = m
            .ledger
            .entries()
            .iter()
            .map(|entry| entry.amount.exact().ratio())
            .collect();
        assert_eq!(listed, vec![density], "the ledger did not carry the material's own mass");

        // Air contributes **no** entry, and a change from air to air is refused
        // by `record` itself (`from == to` records nothing).
        let nothing = hand_change(HandChange {
            x: 1,
            y: 1,
            z: 1,
            from: MaterialId::EMPTY,
            to: MaterialId::EMPTY,
            reason: "nothing".into(),
            actor: ActorId::player(),
            verb: PhysicalVerb::Place,
        })
        .unwrap();
        assert!(nothing.ledger.entries().is_empty());
        assert!(e.record(nothing).is_err(), "from == to must not record");
    }

    #[test]
    fn recording_is_deterministic_in_order() {
        let build = || -> Earthworks {
            let mut e = Earthworks::new();
            e.record(modification(STONE, SOIL, "a")).unwrap();
            e.record(modification(SOIL, MaterialId(5), "b")).unwrap();
            e
        };
        assert_eq!(build(), build());
    }

    #[test]
    fn every_entry_says_who_did_it_and_what_kind_of_act_it_was() {
        let mut e = Earthworks::new();
        let actor = ActorId::new("buffy").unwrap();
        let m = hand_change(HandChange {
            x: 4,
            y: 5,
            z: 6,
            from: MaterialId::EMPTY,
            to: MaterialId(6),
            reason: "a block placed".into(),
            actor: actor.clone(),
            verb: PhysicalVerb::Place,
        })
        .unwrap();
        assert_eq!(m.actor, actor, "the record names the actor, not a placeholder");
        assert_eq!(m.verb, PhysicalVerb::Place);
        assert_eq!(m.task, None, "a hand edit is not a job");
        assert_eq!(m.reservation, None, "a hand edit cannot be declared for");
        e.record(m.clone()).unwrap();
        assert_eq!(e.entries()[0].actor, actor);
    }

    #[test]
    fn the_same_change_twice_is_refused_by_its_own_identity() {
        let mut e = Earthworks::new();
        let m = modification(STONE, SOIL, "built a field");
        e.record(m.clone()).unwrap();
        let again = e.record(m).unwrap_err();
        assert!(
            again
                .all()
                .iter()
                .any(|r| r.kind() == RefusalKind::RecordExists),
            "a change that arrives twice must be refused, not recorded twice"
        );
        assert_eq!(e.len(), 1);
    }

    #[test]
    fn a_change_with_no_identity_is_refused() {
        let mut e = Earthworks::new();
        let mut m = modification(STONE, SOIL, "built a field");
        m.transition = Digest::from_bytes([0; 32]);
        let refused = e.record(m).unwrap_err();
        assert!(refused
            .all()
            .iter()
            .any(|r| r.kind() == RefusalKind::MissingStage));
        assert!(e.is_empty(), "an unidentified change is not a record");
    }

    #[test]
    fn a_hand_change_is_identified_by_its_content_so_an_undo_is_not_a_redo() {
        let actor = ActorId::player();
        let dig = hand_change_id(1, 2, 3, STONE, MaterialId::EMPTY, &actor, PhysicalVerb::Dig);
        let undo = hand_change_id(1, 2, 3, MaterialId::EMPTY, STONE, &actor, PhysicalVerb::Place);
        let elsewhere = hand_change_id(9, 2, 3, STONE, MaterialId::EMPTY, &actor, PhysicalVerb::Dig);
        assert_ne!(dig, undo, "an undo is not the same act as the dig it undoes");
        assert_ne!(dig, elsewhere, "the position is part of the act");
        assert_eq!(dig, hand_change_id(1, 2, 3, STONE, MaterialId::EMPTY, &actor, PhysicalVerb::Dig));
    }
}
