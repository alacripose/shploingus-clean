//! **The civilian process, ported whole from ala-cities. Not re-derived.**
//!
//! # What this module is, and the ruling that put it here
//!
//! The user, 2026-10-02: *"civilians need the entire process (just not the physical mobs,
//! probably useful for the laya observer and sound engine though)"*, and — after the agent's
//! chain of thought proposed a trimmed version — *"Reducing the civilians scope is not allowed,
//! you are killing them by doing so as I do not think you have a full grasp on what the
//! civilians actually did"*. The infraction is `event-0127` (illegal-word occurrence 12, strike
//! 7) and the work is `BLD-0221` on the chain.
//!
//! # The method: copied whole, and the precedent for it
//!
//! `shploingus-game`'s `legacyledger/mod.rs` did this once already and states why:
//!
//! > *"Six files copied whole from `references/Docs-A/core/` … **118.3 KB**, and every line of
//! > the state machine, the evidence ladder and the boundary enforcement is the original's."*
//!
//! So these are the ala-cities files, byte for byte:
//!
//! ```text
//! source: <ala-cities-design-package>/src/
//!   agency/{action.rs 1407, actor.rs 464, evidence.rs 55, selection.rs 380, task.rs 64, mod.rs 26}
//!   gov/mod.rs 1823                  (gov/season.rs 416 was copied, then removed — event-0131)
//!   matter/{rational.rs 314, mod.rs 135}     (Rational copied whole; fluids.rs 12–72 only)
//!   sim/{citizen.rs 157, task.rs 343, mod.rs 16}
//!   ------------------------------------------------------ 5,184 staged, + this mod.rs (128)
//! ```
//!
//! # The inventory that must precede every copy
//!
//! **Before a block is copied, this tree is inventoried for what it already owns** — because
//! copying a system the tree has is two defects at once (a second type, and usually a second
//! record beside the chain), and the second one is the worse. `gov/season.rs` was the first
//! instance and is removed; the season section below has the mapping.
//!
//! The next block is named and already measured, so it does not repeat: the `legacyledger`
//! half's `ticket.rs` and `gov.rs` collide with `voxel_model::ticket` — `TicketId` (`62`),
//! `TicketStatus` (`91`), `TicketKind` (`175`), `EvidenceOrigin` (`380`), `Ticket` (`653`),
//! `Board` (`913`). That half **maps** onto the tree's ticket vocabulary or stays reference;
//! it does not arrive beside it.
//!
//! **Nothing here has been rewritten by hand, and nothing may be.** The system is intricate —
//! a governor deciding before the world changes, 14 retirement reasons, per-actor-verb policy,
//! a 30-variant blocker enum, consequence ownership and compensation, selection plans that
//! reserve and estimate under a declared budget — and a hand-rolled summary of it would be the
//! plausible-substitute failure this project exists to prevent. What changes in the port is the
//! **seam**, never the machine.
//!
//! # The seam, measured from the sources rather than guessed
//!
//! Every `use crate::…` in the set above resolves to one of four things:
//!
//! | import | where it lands |
//! |---|---|
//! | `crate::gov::{Correction, Evidence, Retirement, Ticket}` | **inside this set** — no work. `gov::Season` is **not**: see the season section below |
//! | `crate::matter::Rational` | **landed at the crate root**: `src/matter/rational.rs`, copied whole, refusals mapped onto `RefusalKind`, and **declared** — the governed store's own delta vocabulary names it, so it cannot sit behind this switch (`event-0143`). Not a rename of the unsigned `material::Ratio` |
//! | `crate::materials::generated::{Process, Rational}` | still **owed**, one import site (`sim/task.rs:43`): `materials/*` is 7,405 lines of material science beside this tree's own `voxel_model::material`, read whole first, then mapped (`event-0133`) |
//! | `crate::world::{…}` (20 names) | **landed and owed**: the governed-change vocabulary now lives at the crate root, `voxel_model::world` (`event-0138`), and the generator's record imports it; the `World` handle and its row types are the next landing, measured in `docs/CIVILIAN-PORT.md`. `Digest` already exists (`identity.rs:18`) |
//! | `crate::sim::{BuildingKind, World, Zone, SIM_HZ}` | the city world — the mobs side; read whole before any decision, never copied beside `voxel-shapers` |
//!
//! **The world half is read and mapped, never copied** (`event-0133`, strike 9). `crate::world`,
//! `crate::matter` and `crate::materials` are the ala-cities **world generation**, and this tree
//! owns its own: `crates/voxel-shapers`, contracted by `docs/SHAPERS.md`, with
//! `voxel-shapers::earthworks` as the recorded-modification commit point. What the ported calls
//! need from the world is the **governed half** — actor, verb, reservation, transition identity, a
//! receipt and evidence rows. The record now carries the first four (the `Modification` widening,
//! exactly-once at the record, `event-0138`); the receipt and the evidence rows are the `World`
//! handle, the next landing. `docs/CIVILIAN-PORT.md` has the measured surface and the map.
//!
//! # The season is mapped, not copied — and this was got wrong first
//!
//! `gov/season.rs` was copied in the first pass and has been **removed**, because the clean
//! tree already owns the season and the record forbids this use of the word. The user, 2026-10-02:
//! *"So you know we have our own season system? you should have been more careful with your copy
//! over."* — `event-0131`, strike 8.
//!
//! `voxel_model::chain::Season` (`chain.rs:87`) is the **world's year** — four variants, `at_tick`
//! total and deterministic — and its own refusal text rules out what was copied, in the record's
//! words: *"A season is one of winter, spring, summer, autumn — the world's own year, **never a
//! governance period, a campaign or a historical boundary** (H-10.5)."* The copied type was
//! exactly that governance period (`SeasonMeta { id, timezone, opened_at_unix_ms, purpose,
//! campaigns, counters }`), and it carried a **second record store** beside the chain
//! (`index.json`, `<period>/season.json`, `events.jsonl`, `tickets/`, `retirements/`, `evidence/`,
//! `corrections/`) — which `forbidden.second-record` refuses: *"the chain is the one append-only
//! record, and a scratch ledger beside it drifts."* So the copy was two defects, and the fix is a
//! mapping, never a rename:
//!
//! | ala-cities | this tree |
//! |---|---|
//! | the world's season | `chain::Season` — already owned, untouched |
//! | the governance period (`SeasonMeta.id`, `campaigns`) | the namespace's **`campaign`** (`chain.rs:166`) — `world / year / season / campaign` |
//! | the store (`tickets/`, `retirements/`, `evidence/`, `corrections/`, `events.jsonl`, `index.json`) | **the chain** — `chains/workspace.chain`, the one record |
//!
//! **The governor's machine is untouched; its storage calls are the seam, and they are measured**
//! rather than guessed — `gov/mod.rs` 721–752 (`Season::open`, `Season::attach`, `read_tickets`,
//! `read_retirements`, `read_evidence`, `read_corrections`), 827, 847, 868, 886, 894, 915–916, 975,
//! 1112–1124, 1147–1156 (`append_event`, `next_id`, `write_ticket`, `write_correction`,
//! `write_evidence`). `gov/mod.rs:21`'s `pub mod season;` stays because the machine is byte-identical;
//! **the file at that path is to be *written* as the chain-backed seam, not copied** — and two of
//! its behaviours are load-bearing and must survive the rewrite:
//!
//! * **ids come from the directory, never from a counter** — a process killed mid-run leaves a
//!   counter behind its own directory, and an id that already exists would overwrite history;
//! * **a corrupt record loses that record, not the file** — a log that refuses to load because of
//!   one bad line is a log nobody can use.
//!
//! # The other half, inventoried: **reference, not copied**
//!
//! `shploingus-game/crates/diffusion-core/src/legacyledger/` (3,028 lines) was inventoried file by
//! file against this tree — `docs/CIVILIAN-PORT.md` holds the table and the verdicts. It is not a
//! civilians file: it is the **Bonsai agent daemon's governance ledger** (`agent:bonsai-agentd`,
//! AWSO_BSPT's console mailbox and activity log, the lawmaker lesson's §16 schema via
//! `agent/src/gov_ledger.py`, constants into `C:\Users\ala\Bonsai-demo\agent_uploads\`).
//! **Nothing in it is copied as code.** Two shape-blocks map — the §16 envelope (`gov.rs`) onto
//! `voxel_model::{ticket, envelope, grant}` plus `axis.rs`, and the obligation book (`kinds.rs`:
//! chat | question | commission, and *a new kind may raise what is owed, never lower it*) onto the
//! filing path — one file belongs to another layer (`resume.rs` is the résumé layer, which owes
//! its own block), and the other four are reference. The half's one open question, its JSON
//! boundary, is answered by `event-0129`: one codec, `CanonicalCodec`.
//!
//! # It is not declared in `lib.rs` yet, and that is deliberate
//!
//! `pub mod civilian;` is added to `crates/voxel-model/src/lib.rs` **when the set compiles**,
//! and not before. Until then these files sit on disk, unread by the compiler, so the port
//! cannot make the tree red while the seam is being satisfied. **The switch is one line, and it
//! is the last step rather than the first.**

pub mod agency;
pub mod gov;
pub mod sim;
