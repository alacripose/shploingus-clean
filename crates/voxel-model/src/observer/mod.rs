//! The observer: **the civilians' evidence producer, living at the commit point.**
//!
//! The user's ruling of 2026-10-02 — *laya runs the citizens* — is what puts a
//! module here: an act that is decided by laya and committed by the governed
//! path (`crate::world::governed`) leaves a receipt, and the observer is what
//! turns the receipt's own rows into **evidence on record** — the raw material
//! the gate's proof of work is minted from. It is not a second record: every
//! row it holds quotes a committed transition's digest, and the governed path's
//! evidence row is the authority it cites.

pub mod recorder;
