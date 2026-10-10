//! **The governance record: the civilians' compiler side, over the one chain.**
//!
//! # What this module is, after the correction
//!
//! The user's ruling, 2026-10-02: *"the laya observer is the only fucking citizen"*, and
//! *"we are developing the laya compiler side of the civilians"*. The record said the same
//! thing before the correction did, in three places:
//!
//! * `docs/LEDGER.md` C-95.6 — the floating observer is retained as `laya_observation`,
//!   *"the Laya god-agent's vantage"*, and *"Laya in this tree is the **build-time decision
//!   layer**, not an inhabitant of the world"*;
//! * `architecture/workspace.dsl`'s `civilians` container — *"its evidence is the proof of
//!   work for solutions, graded against `axis.rs`. **No physical mobs**; the records feed
//!   the observer and the sound engine"*;
//! * `civilian/mod.rs`'s inventory rule — the machine's ticket vocabulary **maps onto
//!   `crate::ticket`** or stays reference, and *"it does not arrive beside it."*
//!
//! So the civilian layer's compiler side is **the record**, and it is already this tree's:
//!
//! | the civilians need | this tree owns it, at |
//! |---|---|
//! | the one append-only record, namespaced | [`crate::chain`] |
//! | tickets, statuses, kinds, gates, expectations, retirements, corrections | [`crate::ticket`] |
//! | the board, folded from the chain — never a second store | [`crate::ticket::Board`] |
//! | the ladders evidence is graded on | [`crate::axis`] |
//! | the judge, and the project gate it decides | [`crate::laya`] |
//! | the governed change of the world, and its receipt | [`crate::world`] |
//! | the evidence producer at the commit point | [`crate::observer`] |
//! | who authorised a world write | [`crate::grant`], and `voxel_package::policy`'s one matrix |
//!
//! # What was here, and why it is reference now
//!
//! `gov/mod.rs` was 1,823 lines copied from `ala-cities`: a governor with its own `Op`
//! vocabulary and `ALL_OPS` profile matrix, its own `TicketKind`/`TicketStatus`/
//! `RetirementReason`/`Evidence`/`Ticket`/`Retirement`/`Correction`, a case sampler over
//! `world.citizens`/`world.buildings`/`world.starved()`, and expectations over
//! `world.tile(tile).road`. **Measured: nothing outside `civilian/gov/` named any of it.**
//! It is the **city's** half — the mobs side — and every binding it needed was either a
//! duplicate of a type this tree already owns (the ticket vocabulary, which `Board::from_chain`
//! reads), a second decision matrix (`Governor::authorize` beside `voxel_package::policy`,
//! which `world/governed.rs` refuses: *"the matrix is not restated here… a second action enum
//! in this crate is D-07's two names for one meaning"*), or a world this tree does not have
//! (`crate::sim::World` — the city generator is `voxel-shapers`, read and mapped, never
//! copied, `event-0133`).
//!
//! It therefore stays on disk as `gov/city_reference.rs` — **not declared**,
//! not compiled, exactly as the dirty branch is reference and not a source. The part that was
//! never the city's is [`season`], and it is what remains declared here.
//!
//! # The shape a reader should carry away
//!
//! A civilian act that matters is a **governed change**: an actor, a task, a verb, a claim
//! declared before the change, and a transition identity — committed through
//! [`crate::world::DeltaStore::apply_transition`] under a [`crate::world::WritePermission`]
//! minted from the decision layer's grant. The commit writes an evidence row and leaves a
//! [`crate::world::TransitionReceipt`]; [`crate::observer::recorder`] records the accepted act
//! at that single commit point, and the game side drains those rows to mint the proof of work
//! the gate is graded on. **Nothing in that path is a simulation**, and the observer that
//! watches it is the one citizen.

pub mod season;

pub use season::Season;
