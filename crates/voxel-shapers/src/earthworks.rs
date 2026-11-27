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
use voxel_model::endian::LeBytes;
use voxel_model::profile32;

use voxel_model::conservation::{Exact, Ledger, Mass, MassPumpDeclaration, Source};
use voxel_model::world::{ActorId, PhysicalVerb, ReservationId, TaskId};
use voxel_model::{Digest, MaterialId, Ratio, RefusalKind, RefusalSet, Refused, VoxelShape};

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
            &x.into_le_bytes(),
            &y.into_le_bytes(),
            &z.into_le_bytes(),
            &from.0.into_le_bytes(),
            &to.0.into_le_bytes(),
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
        if crate::materials::is_fluid(material) {
            return Err(crate::fluid_state::refuse_edit(x, y, z));
        }
    }
    let registry = crate::materials::registry()?;
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

    /// **How far up and down the record reaches in one column**, in voxels: the lowest and highest
    /// `y` it has changed, or `None` where it has changed nothing.
    ///
    /// A reader that asks *"where is this column's surface?"* without a chunk in hand — the map's
    /// far probe ([`voxel_stream::WorldSource::column_surface`]) — starts its walk from the
    /// generator's own ceiling, which is the law's elevation plus the tallest tree. **An edit can
    /// stand above that ceiling** (a raised mound, a placed block) **or cut below the generator's
    /// floor** (a pit), and a probe that trusted the law would answer with the world it was before
    /// the change. This is the record's own answer to how far the column can differ, read from the
    /// entries rather than kept as a second index beside them.
    pub fn column_span(&self, x: i32, z: i32) -> Option<(i32, i32)> {
        self.entries
            .iter()
            .filter(|entry| entry.x == x && entry.z == z)
            .fold(None, |span: Option<(i32, i32)>, entry| {
                Some(match span {
                    None => (entry.y, entry.y),
                    Some((low, high)) => (low.min(entry.y), high.max(entry.y)),
                })
            })
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

// -------------------------------------------------------------------------------------
// The profile32 cell: the field record this shaper produces
// -------------------------------------------------------------------------------------
//
// **Earthworks is a terrain shaper, and its product is a cell.** It is not a book beside
// the world and it is not a generator of material: it answers, at a voxel, *which part of
// that voxel the world holds* — the `profile32` cell the shader draws — and the
// append-only record above is where the changes it makes are stated.
//
// The cell is **one plane per voxel, not a stored 32³ grid**, which is dual contouring's
// own record: a cell is its cuts, and the interior is the law those cuts define. (Ju,
// Losasso, Schaefer & Warren, *Dual contouring of Hermite data*, ACM Transactions on
// Graphics 21(3), 2002, 339–346 — <https://doi.org/10.1145/566570.566586>.) The cut is
// placed from the world's **own** surface law and leans by that surface's gradient, so the
// ground is a ramp at 1/32 of a block instead of a staircase of whole ones.
//
// That earthworks is a *terrain-forming* process rather than decoration is the reading of
// the artificial-ground literature: humans move more rock and soil than any other
// geomorphic agent, and the ground they make is a mapped geological unit. (Price, Ford,
// Cooper & Neal, *Humans as major geological and geomorphological agents in the
// Anthropocene: the significance of artificial ground in Great Britain*, Philosophical
// Transactions of the Royal Society A 369(1938), 2011, 1056–1084 —
// <https://doi.org/10.1098/rsta.2010.0296>; Hooke, *On the history of humans as geomorphic
// agents*, Geology 28(9), 2000, 843–846 —
// <https://doi.org/10.1130/0091-7613(2000)28<843:OTHOHA>2.0.CO;2>.) Which is why the
// stage's product is a cell record and its changes are mass-ledgered entries: the work is
// real terrain, and the record is what makes it auditable.

/// **The profile32 cell a voxel holds** — the record this shaper writes at an address.
///
/// The order is the world's own:
///
/// * a voxel the **record** names is whole if the record placed something and empty if it
///   removed it — an earthwork is per voxel, and a placed block is not a partial one;
/// * empty air is an empty cell;
/// * a voxel wholly under the surface law is a whole block;
/// * a voxel wholly above it but not empty — the sea's skin, a tree's organ, a block placed
///   mid-air — is a whole block too, because none of them is the ground's own profile;
/// * the voxel the surface law **cuts** is the cell: a Y cut at the surface's own height in
///   the block, leaning by the surface's gradient, so a ramp is one plane and not 32 steps.
///
/// The cell is a **pure function of the world and the record**, so the fragment that draws
/// it and the process that reports it cannot disagree.
pub fn cell_at(
    spec: &crate::spec::WorldSpec,
    entries: &[Modification],
    x: i32,
    y: i32,
    z: i32,
) -> Refused<profile32::Cell> {
    // **The record's last word at a voxel it names.** The entries are the chain in order,
    // so the newest is the last one; a hand edit is per voxel and replaces the whole block.
    if let Some(entry) = entries
        .iter()
        .rev()
        .find(|entry| entry.x == x && entry.y == y && entry.z == z)
    {
        return Ok(if entry.to.is_empty() {
            profile32::Cell::floor(0.0)
        } else {
            profile32::Cell::solid()
        });
    }
    let material = crate::chunk_fields::material_at(spec, x, y, z)?;
    if material.is_empty() {
        return Ok(profile32::Cell::floor(0.0));
    }
    let surface = crate::terrain::elevation_m(spec, x as f64 + 0.5, z as f64 + 0.5);
    let floor = y as f64;
    if floor + 1.0 <= surface {
        return Ok(profile32::Cell::solid());
    }
    if floor >= surface {
        return Ok(profile32::Cell::solid());
    }
    let at = surface - floor;
    let (dx, dz) = surface_gradient(spec, x, z);
    // **Solid below the plane** (`inside = -1`), which is the ground: a floor at the
    // surface's own height, leaning across the other two axes.
    Ok(profile32::Cell::solid().with(
        profile32::Axis::Y,
        profile32::AxisCut::leaning(at, -1.0, [dx, dz]),
    ))
}

/// **How much of the block the cell occupies**, in `[0, 1]` — the reading the process
/// reports and the number a 1/32 fill is stated in.
///
/// For the single Y cut this shaper produces, the volume is the plane's own average over
/// the block, and a leaning plane's average is its height at the centre: the fill is the
/// cut's `at` (solid below) or `1 - at` (solid above). No 32³ walk is done here —
/// [`profile32::Cell::volume`] is the witness for an arbitrary cell, not a frame cost.
pub fn fill(cell: &profile32::Cell) -> f64 {
    let (at, solid_above) = match cell.cuts[profile32::Axis::Y.index()] {
        Some(cut) => (cut.at, cut.inside > 0.0),
        // No Y cut: every profile cell is inside, which is a whole block.
        None => return 1.0,
    };
    let value: f64 = if solid_above { 1.0 - at } else { at };
    value.clamp(0.0, 1.0)
}

/// **The cell as a voxel shape** — the cubic world format's record of it.
///
/// A voxel in this world is a 4×4×4 grid of subvoxels ([`voxel_model::VoxelShape`]), and
/// that grid is what the mesher reads and the renderer draws. So the shaper's cell does
/// not stay a column beside the world: it **is** the surface voxel's shape, sampled at the
/// centre of each subvoxel — `(i + 0.5) / 4` on each axis — with the block's own material
/// on the inside of every cut and nothing outside. A subvoxel is drawn when its centre is
/// inside the cell, the same point sample the profile grid uses at its own resolution.
///
/// The sampling is the format's, not a second law: [`profile32::Cell::coverage`] decides
/// inside/outside at a point, so a shape and the cell it was made from cannot disagree.
/// A whole block stays whole, an empty cell stays empty, and the cut block becomes the
/// stepped top the mesher draws.
pub fn voxel_shape(material: MaterialId, cell: &profile32::Cell) -> VoxelShape {
    let mut shape = VoxelShape::empty();
    if material.is_empty() {
        return shape;
    }
    let side = voxel_model::SUBVOXEL_SIDE;
    for w in 0..side {
        for v in 0..side {
            for u in 0..side {
                let at = [
                    (u as f64 + 0.5) / side as f64,
                    (v as f64 + 0.5) / side as f64,
                    (w as f64 + 0.5) / side as f64,
                ];
                if cell.coverage(at) > 0.5 {
                    shape.set(
                        voxel_model::SubvoxelIndex { x: u as u8, y: v as u8, z: w as u8 },
                        material,
                    );
                }
            }
        }
    }
    shape
}

/// **The ground's gradient at a column**, in block units per block unit: one central
/// difference of the same surface law the cell is cut from, so the lean and the cut cannot
/// be two different readings of the slope.
fn surface_gradient(spec: &crate::spec::WorldSpec, x: i32, z: i32) -> (f64, f64) {
    let h = |dx: i32, dz: i32| {
        crate::terrain::elevation_m(spec, (x + dx) as f64 + 0.5, (z + dz) as f64 + 0.5)
    };
    ((h(1, 0) - h(-1, 0)) * 0.5, (h(0, 1) - h(0, -1)) * 0.5)
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
        let water = crate::materials::WATER;

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
        let igneous = crate::materials::IGNEOUS;
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
        let density = crate::materials::registry()
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

    fn world() -> crate::spec::WorldSpec {
        crate::spec::WorldSpec::new(2468, crate::spec::Config::default())
    }

    /// The voxel the surface law cuts, and the fraction of it the law fills.
    fn surface_voxel(spec: &crate::spec::WorldSpec, x: i32, z: i32) -> (i32, f64) {
        let surface = crate::terrain::elevation_m(spec, x as f64 + 0.5, z as f64 + 0.5);
        (surface.floor() as i32, surface - surface.floor())
    }

    fn record_at(x: i32, y: i32, z: i32, to: MaterialId) -> Modification {
        let from = if to.is_empty() { SOIL } else { MaterialId::EMPTY };
        let mut ledger = Ledger::new();
        ledger.declare_pump(voxel_model::conservation::MassPumpDeclaration {
            reason: "the test's own earthwork".into(),
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
            x,
            y,
            z,
            from,
            to,
            reason: "the test's own earthwork".into(),
            ledger,
            actor: ActorId::player(),
            task: None,
            verb: PhysicalVerb::Place,
            reservation: None,
            transition: hand_change_id(x, y, z, from, to, &ActorId::player(), PhysicalVerb::Place),
        }
    }

    #[test]
    fn a_voxel_under_the_surface_is_a_whole_block_and_one_above_it_is_empty() {
        let spec = world();
        let (floor, _) = surface_voxel(&spec, 12, -7);
        let below = cell_at(&spec, &[], 12, floor - 2, -7).expect("the shaper answers");
        assert_eq!(fill(&below), 1.0, "a voxel under the surface is whole");
        let above = cell_at(&spec, &[], 12, floor + 3, -7).expect("the shaper answers");
        assert_eq!(fill(&above), 0.0, "air above the surface is an empty cell");
    }

    #[test]
    fn the_surface_voxel_holds_the_fraction_the_law_fills_and_leans_with_the_ground() {
        let spec = world();
        // A column whose surface is well inside its voxel, so the cut is a real one and not
        // a whole block or an empty one at the edges of the search.
        let (x, z) = (0..512)
            .map(|step| (40 + step, -95))
            .find(|(x, z)| {
                let (_, fraction) = surface_voxel(&spec, *x, *z);
                (0.15..=0.85).contains(&fraction)
            })
            .expect("a column with a partial surface voxel");
        let (floor, fraction) = surface_voxel(&spec, x, z);
        let cell = cell_at(&spec, &[], x, floor, z).expect("the shaper answers");
        assert!(
            (fill(&cell) - fraction).abs() < 1e-9,
            "the cell holds {} of the block, the law fills {fraction}",
            fill(&cell)
        );
        let cut = cell.cuts[profile32::Axis::Y.index()].expect("the surface voxel is cut");
        assert!(cut.inside < 0.0, "the ground is below the plane, not above it");
        assert!((cut.at - fraction).abs() < 1e-9);
        // The lean is the surface's own gradient: the same central difference the cell's
        // height came from, so the plane and the cut cannot be two readings of one slope.
        let h = |x: i32| crate::terrain::elevation_m(&spec, x as f64 + 0.5, z as f64 + 0.5);
        assert!((cut.lean[0] - (h(x + 1) - h(x - 1)) * 0.5).abs() < 1e-12);
    }

    #[test]
    fn a_recorded_voxel_is_whole_and_a_recorded_break_is_empty() {
        let spec = world();
        let (floor, _) = surface_voxel(&spec, 4, 9);
        let placed = [record_at(4, floor + 6, 9, SOIL)];
        let cell = cell_at(&spec, &placed, 4, floor + 6, 9).expect("the shaper answers");
        assert_eq!(fill(&cell), 1.0, "a placed block is a whole voxel");
        let broken = [record_at(4, floor - 1, 9, MaterialId::EMPTY)];
        let cell = cell_at(&spec, &broken, 4, floor - 1, 9).expect("the shaper answers");
        assert_eq!(fill(&cell), 0.0, "a recorded break is an empty cell");
    }

    #[test]
    fn the_cell_is_the_voxel_shape_the_mesher_draws() {
        // A 1/2 cut fills the two lower subvoxel layers and none of the upper two: the
        // shape is sampled at the subvoxel centres, so the plane's own coverage decides
        // every cell — there is no second rounding rule in the mesher.
        let cell = profile32::Cell::floor(0.5);
        let shape = voxel_shape(SOIL, &cell);
        let mut filled = 0;
        for z in 0..4u8 {
            for x in 0..4u8 {
                for y in 0..4u8 {
                    let cell = shape.get(voxel_model::SubvoxelIndex { x, y, z });
                    let expected = if y < 2 { SOIL } else { MaterialId::EMPTY };
                    assert_eq!(cell, expected, "subvoxel ({x}, {y}, {z})");
                    filled += usize::from(!cell.is_empty());
                }
            }
        }
        assert_eq!(filled, 32, "a half block is 32 of the 64 subvoxels");
        // The whole block stays whole, and air stays empty — the shape is the cell's
        // answer, not a rule of its own.
        assert!(!voxel_shape(SOIL, &profile32::Cell::solid()).is_empty());
        assert!(voxel_shape(SOIL, &profile32::Cell::floor(0.0)).is_empty());
    }

    #[test]
    fn the_fill_is_a_profile_axis_count_and_never_leaves_the_block() {
        let spec = world();
        for step in 0..64 {
            let (x, z) = (step * 3, -41);
            let (floor, _) = surface_voxel(&spec, x, z);
            for y in [floor - 1, floor, floor + 1] {
                let cell = cell_at(&spec, &[], x, y, z).expect("the shaper answers");
                let fill = fill(&cell);
                assert!((0.0..=1.0).contains(&fill), "fill {fill} left the block");
                let count = (fill * f64::from(profile32::PROFILE_SIDE)).round();
                assert!(
                    (0.0..=f64::from(profile32::PROFILE_SIDE)).contains(&count),
                    "{count} profile cells is not a fill"
                );
            }
        }
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
