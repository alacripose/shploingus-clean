//! **The reference world — the one declaration two processes compare answers through.**
//!
//! The transport frames lines and the gate answers them; neither decides *which* world is being asked.
//! That decision is this module's, and it is written down **once**, so the binary's `serve` mode, its
//! own evidence and its tests cannot drift into three similar worlds: two processes can compare
//! answers only about the **same** world, and comparing two nearly-identical worlds would prove
//! nothing.
//!
//! Nothing here owns state. [`gate`] builds a fresh [`Gate`] over the caller's roster, and
//! [`serve_stdio`] hands that gate to `transport::serve` — a session's record therefore opens
//! **empty**, so what a client reads back is exactly what its own requests recorded. The framing never
//! learns which palette or horizon is in play, and this module never learns how a line is framed.
//!
//! Derived from the **U6** entry of `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §2 and §5's slice 5
//! (*"the record/gate with `bridge::TOOLS` served over stdio"*). Nothing under `crates/` was read
//! (P7); the legacy transport is a witness that the tool bridge is *served*, not a source for how.

use std::io::{BufRead, Write};
use std::sync::OnceLock;

use crate::bridge::Gate;
use crate::material::{MaterialId, MaterialRegistry};
use crate::mesh::{Ladder, ResidencyHorizon};
use crate::policy;
use crate::refusal::Refusal;
use crate::save::Ruleset;
use crate::stage::Declared;
use crate::transport::{self, Session};
use crate::world::{DeclaredPeriod, WorldIdentity};

/// **The reference world's identity**, declared once. A session's two sides, the binary's evidence
/// and its tests compare answers only about the **same** world, and two nearly-identical worlds would
/// prove nothing — so the identity is a value here rather than a constant repeated at each call site.
/// One second is the declared period, and a zero period is refused by name (U2) rather than defaulted.
pub fn identity() -> Result<WorldIdentity, Refusal> {
    let period = DeclaredPeriod::seconds(1).map_err(|kind| {
        Refusal::new(
            kind,
            "the reference identity's declared period is one second, which is never zero",
        )
    })?;
    Ok(WorldIdentity::new(0x5EED_BEEF, period))
}

/// **The reference world's ruleset identity**, declared once beside its identity. A save names the
/// ruleset it was written under and a load declares the one it loads into, so the binary's evidence
/// and its tests must agree on this number exactly as they agree on the seed: a save carrying a
/// ruleset nobody declared is a save of another world (`LEDGER.md` C-96.1, `docs/GAIA.md` §5c/§11).
/// It is a value here rather than a literal at each call site for the reason [`identity`] is.
pub const fn ruleset() -> Ruleset {
    Ruleset::new(7, 3)
}

/// The world's **one material registry**, admitted once for the process: the ladder's palette picks
/// materials out of it and an admission *conforms* a row's required material ids against it, so both
/// must be the same declaration or a gate would be meshing one world and judging another. It lives
/// here for the reason [`identity`] does — the reference declaration is written down once — and it is
/// a borrow rather than a copy because the gate holds the world's declarations rather than a copy of
/// them. Nothing mutates it (`MaterialRegistry` has no `&mut self` constructor here), so every reader
/// in the process sees one value.
pub fn registry() -> &'static MaterialRegistry {
    static REGISTRY: OnceLock<MaterialRegistry> = OnceLock::new();
    REGISTRY.get_or_init(MaterialRegistry::starter)
}

/// **The reference world's own ladder** — the palette laid over the world's six declared lithology
/// classes, drawn from the one registry, with water for the surface layer. Declared here, beside the
/// identity and the ruleset, because it is a fact about *this* world: the mesher that draws a chunk's
/// geometry and U1's own client map a class through the same mapping, so a class cannot be meshed as one
/// material and drawn as another (2026-10-13, §36 — the palette was written out inside `gate()`; two older
/// call sites, `shell.rs`'s geometry pane and `evidence.rs`'s U4 measurement, spell the same five ids
/// locally and folding them in is a two-line, output-preserving follow-up owed).
pub fn ladder() -> Result<Ladder, Refusal> {
    let palette = [MaterialId(1), MaterialId(2), MaterialId(3), MaterialId(4), MaterialId(6)];
    Ladder::over_palette(registry(), &palette, 6, MaterialId(5))
}

/// **Who the reference session acts as, and with what authority** — declared once, beside the identity
/// and the ruleset, for the reason those are: the binary, its evidence and its tests must act as the
/// **same** actor or a denial would be a fact about one session rather than about the table.
///
/// Three grants, one per scope, each **attributed** — and the granter is named as the **document that
/// authorises the act** rather than as a role, because in this tree authority is always traced to a
/// document (P1: every module names the document it was derived from) and a grant whose author is a
/// document is a grant a reader can go and read. That is a **reading** rather than a transcription:
/// `docs/HYPERGRILLING.md` §L-5 requires an identified granter and does not say what kind of thing one
/// is, and this build's answer is *the ruling that says the act may happen*. An unattributed or
/// unscoped grant is refused at construction, so this declaration cannot quietly become a wildcard.
pub fn caller() -> Result<policy::Caller, Refusal> {
    let grants = vec![
        // The record's write, and the derivation whose only record effect it is.
        policy::Grant::of("specs/005-u6-record-api/spec.md", "record")?,
        // Admitting a declared row through the content gate.
        policy::Grant::of("specs/005-u6-record-api/spec.md", "content")?,
        // Writing a world down at a declared reference, and reading one back from it.
        policy::Grant::of("specs/006-world-saves-and-seeds/spec.md", "world")?,
    ];
    policy::Caller::of("operator", grants)
}

/// **The declared reference world**: the material registry its palette is drawn from, the residency
/// horizon and the identity a session's two sides must agree on. The world is the caller's; this is the
/// reference declaration the binary, its evidence and its tests share, so a served answer and a locally
/// derived one are answers about the same world rather than about two similar ones.
pub fn gate(roster: &Declared, workers: usize) -> Result<Gate<'_>, Refusal> {
    let registry = registry();
    let ladder = ladder()?;
    let residency = ResidencyHorizon::m(256.0)?;
    let mut gate = Gate::new(
        roster,
        registry,
        ladder,
        residency,
        identity()?,
        ruleset(),
        caller()?,
    );
    // The one declared knob a session sets at construction: how many workers a derivation may use.
    gate.set_workers(workers);
    Ok(gate)
}

/// The stdio session the binary's `serve` mode runs: the reference world, over the given streams. The
/// record opens **empty**, so what a client reads back is exactly what its own requests recorded.
///
/// A row's content references resolve against the **directory the server runs in**: the gate's root is
/// `.` unless a caller pins one with [`Gate::set_intake_root`], which is the definition of a relative
/// path rather than a policy this module invents — a client that wants the server to read a tree it
/// chose starts it there (the way the separate-process tests do). Nothing else about a session's
/// answers depends on where it runs.
pub fn serve_stdio(
    roster: &Declared,
    workers: usize,
    input: &mut impl BufRead,
    output: &mut impl Write,
) -> Result<Session, Refusal> {
    let mut session_gate = gate(roster, workers)?;
    transport::serve(&mut session_gate, input, output)
}

