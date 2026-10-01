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
use voxel_model::{MaterialId, Ratio, RefusalKind, RefusalSet, Refused};

/// One recorded change.
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
pub fn hand_change(
    x: i32,
    y: i32,
    z: i32,
    from: MaterialId,
    to: MaterialId,
    reason: &str,
) -> Refused<Modification> {
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
    Ok(Modification { x, y, z, from, to, reason: reason.into(), ledger })
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
        let m = hand_change(4, 5, 6, MaterialId::EMPTY, igneous, "a block placed").unwrap();
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
        let nothing = hand_change(1, 1, 1, MaterialId::EMPTY, MaterialId::EMPTY, "nothing").unwrap();
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
}
