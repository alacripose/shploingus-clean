//! **The gate — one closed table of verbs, and answers that carry their own evidence.**
//!
//! Derived from `docs/DESIGN-SPEC.md` §2 **F1** (*"replay inputs are recorded, not derived"* — the
//! world and the record are one truth; a modification nobody recorded is invisible) and **F2**
//! (admission is a decision, never a bool), and from the **U6 — The record and the API layer (the
//! gate)** entry of `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §2 (`admit/inspect` returning `Established`,
//! `bridge::TOOLS`, and *"`bridge::call` returning a `Receipt` naming which checks ran"*). No `.rs`
//! under `crates/voxel-package/` or `crates/voxel-mcp/` was read (P7). The one rule is inherited
//! whole: **refuse, never default**.
//!
//! # The four boundaries this slice exists to hold
//!
//! 1. **The owners answer; the gate dispatches.** World truth is U2's, the fields are U3's stages, the
//!    geometry is U4's mesh. This slice reads them through their own seams — [`World::truth_digest`],
//!    [`field_at`], [`Mesher::derive`], [`Resident::serve`], [`Chain`] — and keeps **no second copy**
//!    of any of them. A capability it needs below it is added at the owner (the geometry event kind
//!    lives in `chain.rs`, where the record lives).
//! 2. **An answer carries the evidence for itself.** A [`Receipt`] names the checks that ran and the
//!    claims the answer makes, and every claim is re-checked **against the thing it describes**
//!    ([`Claim::check`]). A field's values are the one claim that is *re-evaluated* rather than read,
//!    because a field has no stored form: the world is a function of `(seed, period, tick)` and a
//!    function is checked by evaluating it. The `admit` row is the one row that makes **no claim**,
//!    and it says so by not declaring [`CheckName::ClaimsHold`]: its evidence is the digest the intake
//!    measured on the bytes it read in place, and a file is not one of the owners a claim here is
//!    re-checked against — so it claims nothing rather than something nothing could check. A receipt
//!    is **bound to the row it names** ([`Receipt::bind_to_row`]): the answer is that verb's answer,
//!    the claims are what the row's declaration allows, and the checks are exactly the ones the row
//!    declares — so a receipt whose evidence was emptied is refused `receipt_mismatch` by both readers
//!    rather than read back and verified over nothing.
//! 3. **A row with no claim is re-established, not believed.** An admission carries no evidence to
//!    re-check, so [`Receipt::verify`] asks the content gate's own door again: the request is read as
//!    the row it declares ([`Request::row`]), the intake is asked what that row is *now*, and every
//!    carried fact — the feature, the claim, the document digest, each axis's state, and the effect an
//!    admission cannot have had — must be what the request and the world establish, or the receipt
//!    refuses `receipt_mismatch`; a document or a declaration that is not reachable at verification
//!    time refuses by the owner's own name (`source_absent`, `source_digest_mismatch`,
//!    `stage_undeclared`, `unadmitted_material`) instead of passing. **What this still owes:** the five
//!    older rows' answers are **not** re-established from their requests — their claims are their
//!    evidence, and re-deriving `fields` or `geometry` would perform work and move the owners (the
//!    ledger, the resident set, the record), which is a read-only re-evaluation seam `spec.md` and the
//!    LEDGER do not design; so for those rows an answer field edited to contradict the claim beside it
//!    is still not refused by this comparison. The rendered evidence prints the same limit.
//! 4. **The record records changes, not reads.** A call that changes the record appends through the
//!    chain's one writer; a call that changes nothing appends nothing. A world-level refusal *is* a
//!    fact about the world, so it is recorded; a request that does not parse against the declared
//!    table is not an event about anything, so it writes nothing at all. What a recorded refusal
//!    carries is the **decision** — the canonical request and the refusal's name — and never its
//!    detail: the chain hashes the payload into the record's own identity, and a detail is prose that
//!    may quote this host (an operating system's error text, a path as this host normalized it), which
//!    would make one world's record depend on which machine refused it.
//!
//! 5. **Every act passes one decision function before it dispatches** ([`crate::policy`],
//!    `docs/HYPERGRILLING.md` §L-5). `Gate::call` consults `policy::decide` at **exactly one** call
//!    site, with the act taken from the row's own [`Tool::action`] — so the act is the row's
//!    declaration rather than the gate's guess, a door cannot be added without naming its act, and
//!    *"the one call site nobody remembered cannot be the one that fails"* has one place to be. A
//!    denial is refused **by name** and **recorded** through the chain's one writer, for the reason
//!    boundary 4 gives: a refusal nobody can read is a refusal that did not happen.
//!
//! # Why the table is a value rather than a `match`
//!
//! A closed table is checkable: the verbs are a closed enum, the arguments are declared with their
//! kinds, and the checks each verb runs are named in the row that runs them. That is what makes
//! "an interface declared, not described" (F2) a property of the code rather than a claim about it.
//!
//! # The five concerns, and where each one lives
//!
//! The slice is one public surface (`crate::bridge::…`, unchanged) over five files, so a reader can
//! hold one concern at a time:
//!
//! | file | what it owns |
//! |---|---|
//! | [`table`] | the declared surface: argument kinds, check names, the closed rows of `TOOLS` |
//! | [`request`] | one declared line, and its canonical identity |
//! | [`receipt`] | an answer with its claims, its effect, the receipt, and the owners it is verified against |
//! | [`wire`] | the receipt's **external form**: the version, the separators, and the round trip |
//! | [`gate`] | the machinery: the world, the record, the resident geometry, and `call` |
//!
//! A client that only *reads* answers — a transport, a verifier, a test — needs [`receipt`] and
//! [`wire`]; only the server side needs [`gate`]. The separators live in [`wire`] alone, so the
//! framing never has to know another module's format by heart.

use std::fmt;

use crate::chain::{Chain, EventKind, Namespace};
use crate::coordinates::{ChunkKey, VoxelPos};
use crate::digest::Digest;
use crate::field::field_at;
// U2's material vocabulary, in the gate's own scope: a row may **require** materials of the world
// (`requires_materials`), and the thing that decides whether the world has one is the registry the
// gate was handed rather than a list this module keeps.
use crate::material::{MaterialId, MaterialRegistry};
// The content gate's vocabulary — everything a `admit` row's answer is *about*. The intake's `Claim`
// is deliberately **not** imported: this module has a `Claim` of its own (what an answer asserts), and
// a glob import would make one name mean two things at every use site.
use crate::intake::{
    Acceptance, Axis, Establishment, Intake, Licence, Requirement, Requirements, Row, Source,
    WorldDeclarations,
};
use crate::mesh::{Budget, Effort, Ladder, Level, Mesh, Mesher, Resident, ResidencyHorizon, WorkLedger};
use crate::policy;
use crate::refusal::{Refusal, RefusalKind, RefusalSet};
// U9's written form, reached through this table's two save rows (2026-10-12, §28): the `save` row
// writes a document through U9's one encoder and its one writer, and the `load` row reads one in place
// through the **content gate above** and U9's one parser and loader. The ruleset identity travels with
// the gate because it is what a save names and a load declares — never a default.
use crate::save::{Document, Ruleset, Save};
use crate::stage::{Declared, FieldId, ProcessRequest};
use crate::world::{World, WorldIdentity};

mod gate;
mod receipt;
mod request;
mod table;
mod wire;

pub use gate::*;
pub use receipt::*;
pub use request::*;
pub use table::*;
pub use wire::*;

#[cfg(test)]
mod tests;
