//! **The compiler: tickets read back off the chain → solutions → recipes → packages.**
//!
//! # Why this module exists at all
//!
//! The build-order audit (`11-build-order.md` §2 F) measured a missing milestone and named it:
//!
//! > *"**the compiler** — records → slice spec → package, **with a real caller for
//! > `emit_package`** … M2 defines the format and nothing in the plan emits against it."*
//!
//! And the same audit's §2 D named the class of defect it belongs to: *"a gate that cannot run
//! looks the same as a gate that passes when nobody runs it."* A format with no emitter is the
//! same shape of hole — `recipe.rs` could compile a hand-written `.shspec` and nothing could
//! produce one from the record.
//!
//! So this is the loop the user described, made executable:
//!
//! ```text
//! user files a ticket in-game
//!   → Chain.append(TicketTransition)            the one append-only record
//!   → Board::from_chain(&chain)                 the tickets/governance view
//!   → Solution::for_ticket(...)                 back-compiled, deterministically
//!   → render(...) → compile(...)                recipe source → package bytes
//!   → verify_provenance(...)                    governance + security audit
//! ```
//!
//! Every arrow is a function here or in a module this crate already owns. **Nothing in the
//! chain is invented vocabulary**, and the audit runs at **every** transition rather than once
//! at the end.
//!
//! # The audit is the chokepoint, not a wrapper
//!
//! L-5's requirement, which this tree already satisfies in [`crate::policy`]:
//!
//! > *"That is the shape of the lineage's other chokepoints … expressed as **one matrix, one
//! > decision function, one audit sink**, with an exhaustiveness check that makes a forgotten
//! > case a compile error."*
//!
//! So [`Auditor::step`] is the **only** way to move between stages, it consults
//! [`crate::policy::decide`] for the action each stage needs, and it **appends every decision to
//! the chain** — allowed *and* denied. L-5's rule is that *"every denial writes an audit
//! event"*, and a denial nobody can read did not happen. A denied stage **stops** the compile:
//! there is no "compile anyway and record it afterwards" path, because a step that proceeds
//! unpermitted is precisely what a chokepoint exists to prevent.
//!
//! **The stages are actions, not booleans.** Each names its [`crate::policy::Action`], so the
//! permission required to compile a solution is the same matrix row as every other action in
//! the project, and a new stage cannot be added without a permission that already exists.
//!
//! # What a "solution" is, and why it is not invented
//!
//! A [`Solution`] is **the set of window declarations a ticket asks for**, plus the record ids
//! it is derived from. It is not a plan, a patch, or a description of code: the record says the
//! recipe is *the package declaration* (L-4, D97 rule 1 — *"the recipe is the package
//! declaration / slice spec … a file is judged against the row it implements"*), so a solution
//! that is not a set of declarations has nothing to compile.
//!
//! **It is derived, never authored.** [`Solution::for_ticket`] reads the ticket's objective and
//! gate off the chain and names the windows from them. The naming is deliberately strict — a
//! window id the ticket does not mention is **never declared**, because a compiler that invents a
//! window is a compiler that can put a window on a player's screen that nobody asked for.
//!
//! **And a solution needs no UI surface, so a ticket that names no window compiles.** This
//! module used to refuse such a ticket outright, and that refusal was a misreading rather than a
//! rule: the user's own words are *"the solutions themselves don't need a UI surface so we're not
//! just magically conjuring new UI elements up, it was a different reading"* (2026-10-01). The
//! strictness is in the **reading** — ordinary words are never promoted into ids — and not in a
//! demand that a solution carry a window it was never asked for. A prose ticket now compiles to a
//! solution whose `windows` is empty, and what the client does with a package that declares no
//! console window is the client's own, separately-stated refusal (`shell::mount::Mount::reload`).
//!
//! # The bloat discipline, carried from `envelope.rs`
//!
//! [`render`] emits the **one-key-per-row** grammar `recipe.rs` already parses, and every value
//! is bounded by that module's own `MAX_VALUE_BYTES`. A solution whose rendering would exceed
//! `MAX_WINDOWS` is **refused**, not truncated: a truncated arrangement is an arrangement that
//! silently lost a window, and the whole point of the gate is that what is mounted is what was
//! asked for.
use voxel_model::endian::LeBytes;

use std::path::Path;

use voxel_model::chain::{Chain, EventKind, Namespace, Season};
use voxel_model::envelope::{Control, EventType, Kind, RecordState};
use voxel_model::ticket::{Board, Expectation, ReadBack, Ticket, TicketId};
use voxel_model::wire::{
    chain_event_schema, CanonicalCodec, Codec, Projection, Value, WIRE_MAGIC,
};
use voxel_model::{Digest, RefusalKind, RefusalSet};

use crate::policy::{self, Action, Decision, Grant};
use crate::recipe::{self, WindowSpec};

/// The compile stages, in order. **Each names the action it needs.**
///
/// A closed list, and the point of naming the [`Action`] on each variant is that **a new stage
/// cannot be added without a permission that already exists in the matrix** — which is
/// L-5's *"one matrix, one decision function"* turned into a type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    /// Read the tickets back off the chain.
    ReadTickets,
    /// Back-compile a solution from a ticket.
    DeriveSolution,
    /// Render the solution as recipe source.
    Render,
    /// Compile the source into package bytes.
    Compile,
    /// Check the package's provenance against the record.
    VerifyProvenance,
}

impl Stage {
    pub const ALL: [Stage; 5] = [
        Stage::ReadTickets,
        Stage::DeriveSolution,
        Stage::Render,
        Stage::Compile,
        Stage::VerifyProvenance,
    ];

    pub const COUNT: usize = 5;

    pub const fn name(self) -> &'static str {
        match self {
            Stage::ReadTickets => "read_tickets",
            Stage::DeriveSolution => "derive_solution",
            Stage::Render => "render",
            Stage::Compile => "compile",
            Stage::VerifyProvenance => "verify_provenance",
        }
    }

    /// **The action this stage needs permitted.** Not a flag — a matrix row.
    ///
    /// Read-tickets and verify-provenance are reads and are `AlwaysAllowed`; the three that
    /// change or produce something are `RequiresGrant`, so compiling a solution on a strength
    /// ticket's authority is refused by the same rule as any other write.
    pub const fn action(self) -> Action {
        match self {
            // **A read is a read.** Reading the tickets is reading governance, and the matrix
            // already says that is always allowed.
            Stage::ReadTickets => Action::ReadGovernance,
            // **The two audit-writing stages, and this is the correction.** Both were mapped
            // onto actions that do not describe them: deriving a solution is not *filing* a
            // ticket, and rendering one is not *compiling* a recipe. What each actually does
            // is read and append its audit event — so `AppendEvent`, which the matrix makes
            // `RequiresGrant`, is the honest answer. Under the old map `DeriveSolution` was
            // `AlwaysAllowed` and `Render` was `AlwaysAllowed`, so **two chain writes needed
            // no authority at all**, and the compiler's own test caught it.
            Stage::DeriveSolution => Action::AppendEvent,
            Stage::Render => Action::AppendEvent,
            // Compiling a declared recipe into a package is a **pure function over text**,
            // and the matrix already rules it always allowed. A compile that needed a grant
            // would mean the build gate needed one on every build.
            Stage::Compile => Action::CompileRecipe,
            Stage::VerifyProvenance => Action::ReadProvenance,
        }
    }
}

/// What the audit decided at a stage. **Recorded on the chain, allowed or denied.**
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditEntry {
    pub stage: Stage,
    pub action: Action,
    pub decision: Decision,
}

/// The audit sink. **One place every stage passes through.**
///
/// Holds the entries so a caller can read the whole audit back, and appends each decision to
/// the chain so it is on the record rather than only in a report.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Auditor {
    entries: Vec<AuditEntry>,
}

impl Auditor {
    pub fn new() -> Self {
        Auditor::default()
    }

    /// Every decision, in order.
    pub fn entries(&self) -> &[AuditEntry] {
        &self.entries
    }

    /// Whether every stage so far was allowed.
    pub fn all_allowed(&self) -> bool {
        self.entries.iter().all(|entry| entry.decision.is_permitted())
    }

    /// Consult the chokepoint for one stage, and **record the decision either way**.
    ///
    /// Returns the decision. A denial is the caller's to act on — and
    /// [`Auditor::gate`] is the one that stops, so a caller cannot proceed past a denial by
    /// ignoring the return value.
    pub fn step(
        &mut self,
        chain: &mut Chain,
        stage: Stage,
        grant: Option<&Grant>,
    ) -> Decision {
        let action = stage.action();
        let decision = policy::decide(action, grant);
        self.entries.push(AuditEntry {
            stage,
            action,
            decision: decision.clone(),
        });
        // **Every decision is an event, and a denial is recorded as a refusal.** L-5: every
        // denial writes an audit event. `AppendEvent` is `AlwaysAllowed` precisely so that
        // recording a refusal can never itself require permission — otherwise a denial could
        // be unreportable, which would make the chokepoint unauditable.
        let _ = chain.append(
            match chain_last_namespace(chain) {
                Some(namespace) => namespace,
                None => match voxel_model::chain::Namespace::new(
                    "shploingus",
                    0,
                    voxel_model::chain::Season::Spring,
                    "compile",
                ) {
                    Ok(namespace) => namespace,
                    Err(_) => return decision,
                },
            },
            if decision.is_permitted() {
                voxel_model::chain::EventKind::DevelopmentFact
            } else {
                voxel_model::chain::EventKind::RefusalRecorded
            },
            format!(
                "compile stage {} needs {}: {}",
                stage.name(),
                action.name(),
                decision.slug()
            ),
        );
        decision
    }

    /// Run one stage, and **stop the whole compile on a denial**.
    ///
    /// This is the only way a stage is entered, and returning `Err` on a denial is the
    /// mechanism by which *"the build must not be capable of forgetting"* (H-10.1) holds for
    /// the compiler and not only for the build gate.
    pub fn gate(
        &mut self,
        chain: &mut Chain,
        stage: Stage,
        grant: Option<&Grant>,
    ) -> Result<(), RefusalSet> {
        let decision = self.step(chain, stage, grant);
        // **`is_permitted`, not `== Decision::Allowed`.** A granted decision is permitted too,
        // and matching on `Allowed` alone would have refused every correctly-granted stage.
        if decision.is_permitted() {
            return Ok(());
        }
        let reason = match &decision {
            Decision::Denied { reason } => reason.clone(),
            other => format!("a decision that is not permitted: {}", other.slug()),
        };
        let mut refusals = RefusalSet::new();
        refusals.refuse(
            RefusalKind::UntrustedKey,
            format!(
                "compile stage {} refused: {reason}. The refusal is on the chain, and the compile \
                 does not continue past it",
                stage.name()
            ),
        );
        Err(refusals)
    }
}

/// The namespace of the most recent event, so an audit entry lands in the right place.
///
/// **Folded, not indexed**, because `Chain` deliberately has no accessor that hands out its
/// events (C-54.1: *fold the chain, never load it whole*).
fn chain_last_namespace(chain: &Chain) -> Option<voxel_model::chain::Namespace> {
    chain.fold(None, |_last, event| Some(event.namespace.clone()))
}

/// One window a solution declares. **This tree's own shape**, from `recipe::WindowSpec`.
///
/// A field list copied from `recipe.rs` would be a second declaration of the same record, and
/// a second declaration is a second thing that can drift — the exact defect L-1's D-07 names
/// as *vocabulary mismatch*. So a solution carries `WindowSpec` values directly and this module
/// adds no window vocabulary of its own.
pub type DeclaredWindow = WindowSpec;

/// A back-compiled solution: what one ticket asks the arrangement to contain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Solution {
    /// The ticket this answers.
    pub ticket: TicketId,
    /// The arrangement name. From the ticket's own objective, never invented.
    pub name: String,
    /// **The rows of the record this solution implements** — the work, as distinct from the
    /// arrangement below. One row today: the ticket's own chain event, carrying the player's
    /// words verbatim. **A ticket that names no window still compiles to a recipe that says what
    /// the work is**, which is the difference between a package the agent can implement from and
    /// a package that declares only a name.
    ///
    /// **The shape is `shploingus-game`'s**: there a compile produces a package and a slice spec,
    /// and the slice spec's `implemented_rows` is what the package implements. The rows live in
    /// the record (`Record { id, title, text }` — *"this is the recipe the oracle judges code
    /// against"*), which is why they are derived here rather than written by hand.
    pub rows: Vec<recipe::Row>,
    /// The windows, in declaration order.
    pub windows: Vec<DeclaredWindow>,
    /// The chain events this was derived from — **so the derivation is traceable**.
    pub record_ids: Vec<String>,
    /// The read-back that would be required for this solution to retire as validated.
    ///
    /// Carried from the ticket's gate, and **not evaluated here**: a compiler does not verify
    /// the world, it produces the declaration whose verification is the ticket's business.
    /// Conflating them is how a compile acquires a validated-looking label.
    pub expectation: Expectation,
}

impl Solution {
    /// **Back-compile a solution from a ticket, or refuse.**
    ///
    /// The naming is strict on purpose: a window is derived from the ticket's own text, and **an id
    /// the ticket does not mention is never declared** — because a compiler that invents a window
    /// can put a window on a player's screen that nobody asked for, and that is the failure the
    /// whole audit chain exists to make impossible.
    ///
    /// **A ticket that names no window is not refused, because a solution needs no UI surface.**
    /// It compiles to a solution that declares no windows: the derivation reads the record and
    /// generates nothing, so an empty window set is what the ticket actually asked for rather than
    /// a gap in it. The refusal that used to stand here demanded a window id to satisfy the
    /// compiler, which is the compiler conjuring up a requirement nobody filed — corrected
    /// 2026-10-01 at the user's ruling, and pinned by [`tests::a_ticket_naming_no_window_compiles_to_a_solution_with_none`].
    pub fn for_ticket(ticket: &Ticket) -> Result<Self, RefusalSet> {
        let mut refusals = RefusalSet::new();

        if ticket.objective.trim().is_empty() {
            refusals.refuse(
                RefusalKind::NoReading,
                format!(
                    "ticket {} has no objective, so there is nothing to compile into a \
                     declaration",
                    ticket.id.as_str()
                ),
            );
        }
        if ticket.gate.statement.trim().is_empty() {
            refusals.refuse(
                RefusalKind::NoReading,
                format!(
                    "ticket {} states no gate, so a solution compiled from it would certify \
                     nothing",
                    ticket.id.as_str()
                ),
            );
        }

        // **A closed ticket is not recompiled.** A solution from retired work would be a second
        // answer to a question the record already closed, and the reference's rule is that a
        // closed ticket is superseded by a new one rather than reopened.
        if ticket.is_closed() {
            refusals.refuse(
                RefusalKind::UngovernedStatus,
                format!(
                    "ticket {} is closed as {}. A closed ticket is not recompiled: a new ticket \
                     references it, which is what keeps the record append-only",
                    ticket.id.as_str(),
                    ticket.status.name()
                ),
            );
        }

        // **All refusals collected, then reported together** — `RefusalSet` is the whole
        // reason this pattern exists. Checked without consuming, because the set is returned
        // below and still needs accumulating.
        if !refusals.is_empty() {
            return Err(refusals);
        }

        // The window ids the ticket's own text names. **Only ids the ticket mentions**, so the
        // derivation is a reading of the record rather than a generation — and a ticket that
        // mentions none yields **no windows**, which is a solution and not a failure. Nothing is
        // conjured to fill the set: an empty `windows` is the honest answer to a ticket that asks
        // for no surface (ruled 2026-10-01).
        let mentioned = mentioned_window_ids(&ticket.objective);
        if mentioned.len() > recipe::MAX_WINDOWS {
            refusals.refuse(
                RefusalKind::OutOfDeclaredRange,
                format!(
                    "ticket {} names {} windows and an arrangement holds at most {}. Refused \
                     rather than truncated: a truncated arrangement silently lost a window",
                    ticket.id.as_str(),
                    mentioned.len(),
                    recipe::MAX_WINDOWS
                ),
            );
            return Err(refusals);
        }

        let windows = mentioned
            .iter()
            .map(|id| DeclaredWindow {
                id: id.clone(),
                // **The title is the id, verbatim.** Not prettified, not title-cased: a title
                // this module invented would be a second thing that could differ from the
                // record, and the record is the authority.
                title: id.clone(),
                content: crate::recipe::Content::Mods,
                lane: crate::recipe::Lane::Control,
                extent_unit: crate::recipe::ExtentUnit::SquarePixels,
                geometry: [0, 0, 800, 600],
                min_size: [40, 40],
                opacity: 1000,
                // **Elevation 0, never a chosen level.** H-13.2 rules that which surface sits at
                // which level is *a declaration in the arrangement package*, so the compiler
                // declares the floor and the arrangement decides. Guessing a level here would
                // be exactly the "a plausible substitute for a missing field" the record
                // refuses.
                elevation: 0,
                theme_role: "surface".to_string(),
                pinnable: true,
                layout_policy: crate::recipe::LayoutPolicy::NONE,
                behavior_policy: crate::recipe::BehaviorPolicy::NONE,
                // **The record id, so the window points at the chain event that declared it.**
                record_id: format!("event-{:04}", ticket.event_index),
            })
            .collect();

        refusals.into_result(Solution {
            ticket: ticket.id.clone(),
            name: ticket.id.as_str().to_string(),
            // **The row is the record's, and its text is the player's own words.** The id is the
            // ticket's own (`BLD-0207`), which is the record's id shape the recipe grammar
            // checks, and the text is the objective verbatim — the same rule the window titles
            // follow below, for the same reason: a paraphrase here is a second thing that can
            // differ from the record.
            rows: vec![recipe::Row {
                id: ticket.id.as_str().to_string(),
                text: ticket.objective.clone(),
            }],
            windows,
            record_ids: vec![format!("event-{:04}", ticket.event_index)],
            expectation: ticket.gate.expectation.clone(),
        })
    }

    /// Whether this solution's expectation, if read back, would validate the ticket.
    ///
    /// Takes the read-back **as an argument**. A compiler that read the world itself would be
    /// claiming a verification it did not perform, and *"cannot stall must never be
    /// implemented as auto-promote to success"* is the rule that stops here.
    pub fn would_validate(&self, read_back: ReadBack) -> bool {
        self.expectation.has_claim() && read_back.validates()
    }
}

/// The window ids a ticket's own text names.
///
/// **Two forms, and the shape of the line is what tells them apart.** A bracketed token —
/// `window[console]`, `window:console`, `window=console` — anywhere in the line, or a line that
/// is **exactly one token**, which is a bare id. `file refusals` declares a window called
/// `refusals`, because that is what a person at a keyboard types.
///
/// **Prose names nothing, and protecting that is the whole reason the grammar is narrow.** A
/// scan that picked up ordinary nouns would compile a window per noun, and a compiler that does
/// that is a compiler nobody can review.
///
/// **One token, rather than "any bare token in the line", is what keeps prose out.** C-67.7
/// recorded this grammar as unruled and named its cost: `window[id]` is a machine grammar and a
/// player will not type brackets. Widening it to every bare token would hand that straight back —
/// `file the console should log the ledger` would declare a window called `the`. **The line's
/// arity is the rule, not the words in it**, which is why no vocabulary list appears anywhere in
/// this file and why adding one would be the defect.
///
/// **What the doc comment on this function used to say, and why it is gone:** it described "a
/// bare token that is already a known window id in the arrangement". This function takes a
/// `&str` and has no arrangement to consult, so that was a rule the code never had — a reader
/// would have gone looking for the lookup. A doc comment that describes a check which is not
/// there is worse than no comment, because it is believed.
fn mentioned_window_ids(objective: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    let bytes: Vec<char> = objective.chars().collect();
    let mut index = 0usize;
    while index < bytes.len() {
        // Bracketed form: `window[console]` or `window:console`.
        if objective_line_starts_window(&bytes, index) {
            let start = index + 7;
            let mut end = start;
            while end < bytes.len() && (bytes[end].is_alphanumeric() || bytes[end] == '_' || bytes[end] == '-') {
                end += 1;
            }
            let id: String = bytes[start..end].iter().collect();
            if !id.is_empty() && !found.contains(&id) {
                found.push(id);
            }
            index = end;
            continue;
        }
        index += 1;
    }
    // **The bare form: the whole objective is one token, so it is an id.** Applied after the
    // bracket scan, and it cannot collide with it — a line holding `window[console]` is a
    // single token but fails [`is_window_id_token`] on the bracket, so the two forms never both
    // fire and the order does not have to matter.
    if let Some(id) = single_token_id(objective) {
        if !found.contains(&id) {
            found.push(id);
        }
    }
    found
}

/// Whether `text` is exactly one token **and** that token is shaped like a window id.
///
/// **Two refusals, and both are the grammar.** A line with any whitespace in it is prose by
/// this rule, whatever its words are; and a single token carrying punctuation is a machine
/// token that failed to parse — `window[console]` — rather than an id. The character set is the
/// recipe's own: [`recipe::MAX_WINDOWS`] arrangements carry ids matching it, so an id accepted
/// here is an id the rest of the pipeline can carry.
fn single_token_id(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed.split_whitespace().count() != 1 {
        return None;
    }
    if !trimmed.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return None;
    }
    Some(trimmed.to_string())
}

/// Whether the text at `index` begins `window` followed by a separator.
fn objective_line_starts_window(chars: &[char], index: usize) -> bool {
    let word: String = chars
        .iter()
        .skip(index)
        .take(6)
        .collect::<String>()
        .to_lowercase();
    if word != "window" {
        return false;
    }
    matches!(chars.get(index + 6), Some('[') | Some(':') | Some('='))
}

/// The package name a back-compiled solution is emitted under.
///
/// **A declared constant, not a derivation.** The recipe's `package` row demands a canonical
/// identifier — lowercase, digits and dashes — and a ticket id is `BLD-0000`, which is
/// neither. Deriving one from the other would mean inventing a name the ticket never carried,
/// and a package's name is the thing a reader trusts it by.
pub const PACKAGE_NAME: &str = "from-ticket";

/// The version a back-compiled solution is emitted under. **Also declared, and deliberately
/// `v0`**: an arrangement compiled from a ticket has not been reviewed by anything, and a
/// version string is the claim that it has.
pub const PACKAGE_VERSION: &str = "v0";

/// Render a solution as **recipe source**, in the one-key-per-row grammar `recipe.rs` parses.
///
/// One function, and it is the only place a recipe's text is written, so the grammar has one
/// producer. Every value is bounded by `recipe::MAX_VALUE_BYTES` and **refused** past it.
///
/// **The grammar is the recipe's, not this module's.** TAB-separated, a header naming the
/// version, a `package` row, then one `window` block per window with its keys indented by a
/// TAB. The version string is [`recipe::HEADER`] rather than a literal, so a version bump
/// cannot leave this function emitting a header the parser refuses.
pub fn render(solution: &Solution) -> Result<Vec<u8>, RefusalSet> {
    let mut refusals = RefusalSet::new();
    let mut out = String::new();

    out.push_str(recipe::HEADER);
    out.push('\n');

    let version = PACKAGE_VERSION;
    check_value("package", PACKAGE_NAME, &mut refusals);
    check_value("package-version", version, &mut refusals);
    out.push_str(&format!("package\t{PACKAGE_NAME}\t{version}\n"));

    // **The rows are written before the windows, because the work is what a package is for.**
    // Each row is the record's id and the record's own words, and the two refusals here are the
    // grammar's: a text past its own declared ceiling, and a text carrying a tab — which would
    // split one value into two and leave the second unread by anything (the parser reads the
    // third field as the whole text and refuses a tab in it).
    for row in &solution.rows {
        if row.text.len() > recipe::MAX_ROW_TEXT_BYTES {
            refusals.refuse(
                RefusalKind::OutOfDeclaredRange,
                format!(
                    "row {} carries {} bytes and a row holds {}. Refused rather than truncated: \
                     a row whose text was cut is a decision whose reason was cut",
                    row.id,
                    row.text.len(),
                    recipe::MAX_ROW_TEXT_BYTES
                ),
            );
        }
        if row.text.contains('\t') {
            refusals.refuse(
                RefusalKind::MalformedIdentity,
                format!(
                    "row {} carries a tab in its text. A tab is the grammar's separator, so the \
                     text would arrive as a second value nothing reads",
                    row.id
                ),
            );
        }
        out.push_str(&format!("row\t{}\t{}\n", row.id, row.text));
    }

    for window in &solution.windows {
        out.push_str(&format!("window\t{}\n", window.id));
        // **Every key the record requires, in the record's order.** A missing key is a
        // refusal at parse time, so a renderer that omitted one would produce a recipe that
        // cannot be read back — which is the round trip this whole module exists to make.
        let rows: [(&str, String); 13] = [
            ("title", window.title.clone()),
            ("content", window.content.name().to_string()),
            ("lane", window.lane.name().to_string()),
            ("extent", window.extent_unit.name().to_string()),
            (
                "geometry",
                format!(
                    "{} {} {} {}",
                    window.geometry[0], window.geometry[1], window.geometry[2], window.geometry[3]
                ),
            ),
            (
                "min-size",
                format!("{} {}", window.min_size[0], window.min_size[1]),
            ),
            ("opacity", window.opacity.to_string()),
            ("elevation", window.elevation.to_string()),
            ("theme-role", window.theme_role.clone()),
            ("pinnable", window.pinnable.to_string()),
            ("layout", window.layout_policy.declared()),
            ("behavior", window.behavior_policy.declared()),
            ("record-id", window.record_id.clone()),
        ];
        for (key, value) in rows {
            check_value(key, &value, &mut refusals);
            out.push_str(&format!("\t{key}\t{value}\n"));
        }
    }

    if !refusals.is_empty() {
        return Err(refusals);
    }
    Ok(out.into_bytes())
}

fn check_value(key: &str, value: &str, refusals: &mut RefusalSet) {
    if value.len() > recipe::MAX_VALUE_BYTES {
        refusals.refuse(
            RefusalKind::OutOfDeclaredRange,
            format!(
                "{key} is {} bytes and a recipe value holds at most {}. Refused rather than \
                 truncated",
                value.len(),
                recipe::MAX_VALUE_BYTES
            ),
        );
    }
}

/// The receipt a compile produces: what was made, from what, and under whose authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Receipt {
    pub name: String,
    /// The digest of the compiled package bytes.
    pub package_digest: Digest,
    /// The digest of the recipe source it was compiled from.
    pub source_digest: Digest,
    /// The chain events the solution was derived from.
    pub record_ids: Vec<String>,
    /// The number of decisions the audit made — allowed and denied both.
    pub audited: usize,
}

/// **The whole loop, with the audit at every transition.**
///
/// This is the `emit_package` the build-order audit found missing. It is deliberately one
/// function rather than four, because **the stages only mean anything together** — a caller
/// that could render without compiling, or compile without verifying provenance, could skip
/// the gate, and a gate that can be stepped around is not a gate.
pub fn emit(
    chain: &mut Chain,
    ticket: &Ticket,
    grant: Option<&Grant>,
) -> Result<(Vec<u8>, Receipt), RefusalSet> {
    let mut auditor = Auditor::new();
    let mut refusals = RefusalSet::new();

    // 1 — read the tickets back. The `board` argument is the view; this stage records that the
    //     read happened, which is the point: the view and the record cannot disagree.
    if let Err(found) = auditor.gate(chain, Stage::ReadTickets, grant) {
        refusals.absorb(found);
        return Err(refusals);
    }
    let board = Board::from_chain(chain);
    if board.ticket(&ticket.id).is_none() {
        refusals.refuse(
            RefusalKind::MalformedIdentity,
            format!(
                "ticket {} is not on this chain, so there is nothing to read back for it",
                ticket.id.as_str()
            ),
        );
        return Err(refusals);
    }

    // 2 — back-compile the solution.
    if let Err(found) = auditor.gate(chain, Stage::DeriveSolution, grant) {
        refusals.absorb(found);
        return Err(refusals);
    }
    let solution = match Solution::for_ticket(ticket) {
        Ok(solution) => solution,
        Err(found) => {
            refusals.absorb(found);
            return Err(refusals);
        }
    };

    // 3 — render, then compile. Two stages because they are two permissions' worth of work,
    //     and a stage list that merged them would be a stage list that could not say which
    //     one was refused.
    if let Err(found) = auditor.gate(chain, Stage::Render, grant) {
        refusals.absorb(found);
        return Err(refusals);
    }
    let source = match render(&solution) {
        Ok(source) => source,
        Err(found) => {
            refusals.absorb(found);
            return Err(refusals);
        }
    };

    if let Err(found) = auditor.gate(chain, Stage::Compile, grant) {
        refusals.absorb(found);
        return Err(refusals);
    }
    let package = match recipe::compile(&source) {
        Ok(package) => package,
        Err(found) => {
            refusals.absorb(found);
            return Err(refusals);
        }
    };

    // 4 — provenance. **A refusal here discards the package**, because a package whose
    //     provenance did not verify is exactly the artifact R-59.1's ruling is about: an
    //     operator states what they expect, and a package that does not match it is not
    //     silently shipped.
    if let Err(found) = auditor.gate(chain, Stage::VerifyProvenance, grant) {
        refusals.absorb(found);
        return Err(refusals);
    }
    if let Err(found) = verify_provenance(&source, &package, &solution) {
        refusals.absorb(found);
        return Err(refusals);
    }

    refusals.into_result((
        package.clone(),
        Receipt {
            name: solution.name.clone(),
            package_digest: Digest::of(&package),
            source_digest: Digest::of(&source),
            record_ids: solution.record_ids.clone(),
            audited: auditor.entries().len(),
        },
    ))
}

/// Check that the compiled package is a faithful reading of the source it claims.
///
/// **Round-trip, not inspection.** The compiler asserts nothing about its own output; it
/// recompiles the source and compares digests, so a claim that is not checked is not made.
pub fn verify_provenance(
    source: &[u8],
    package: &[u8],
    solution: &Solution,
) -> Result<(), RefusalSet> {
    let mut refusals = RefusalSet::new();

    let loaded = match recipe::load(package) {
        Ok(loaded) => loaded,
        Err(found) => {
            refusals.absorb(found);
            return Err(refusals);
        }
    };

    if loaded.source != source {
        refusals.refuse(
            RefusalKind::DigestMismatch,
            "the loaded package does not carry the exact recipe source it was compiled from, so \
             the two files disagree rather than merely existing",
        );
    }
    if loaded.recipe.windows.len() != solution.windows.len() {
        refusals.refuse(
            RefusalKind::FabricatedFeature,
            format!(
                "the solution declared {} windows and the package carries {}. A package that \
                 does not match its declaration is the defect C-64 was recorded for",
                solution.windows.len(),
                loaded.recipe.windows.len()
            ),
        );
    }

    refusals.into_result(())
}

/// What a read-back of a chain file found.
///
/// Named `ChainRead` and not `ReadBack`, because [`ReadBack`] already exists in
/// `voxel_model::ticket` and means something else entirely: the three-state *answer to an
/// expectation*. Two types with one name and two meanings is the vocabulary mismatch D-07
/// names, so the file-level one is qualified by what it reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChainRead {
    /// The chain, folded from the file.
    pub chain: Chain,
    /// The tickets view over it.
    pub board: Board,
    /// **How many records were refused and skipped.** Reported, never hidden: *a corrupt
    /// record loses that record, not the file.*
    pub corrupt: usize,
    /// Every refusal, verbatim. **A count alone is not enough** — a count says how many and
    /// not why, and a reader who cannot see the reason cannot fix the writer.
    pub corrupt_detail: Vec<String>,
    /// How many records were folded onto the chain.
    pub folded: usize,
    /// Every record that was read, in file order. **Kept because the chain does not carry the
    /// envelope's descriptive axes** — see [`append_event`] — so a reader who needs them reads
    /// them here rather than finding them gone.
    pub projections: Vec<Projection>,
}

/// Read the tickets back off a chain file, or refuse.
///
/// **This is how an in-game ticket comes back to the agent.** A user files it in the client, it
/// lands on the chain file, and this is the door it comes out of.
///
/// # Frames, not lines, and that is the format's own doing
///
/// The records are **length-described frames**, so a file is a *concatenation* of records and
/// the reader walks record by record rather than line by line. This is strictly better than a
/// line reader for the rule the reference's corrupt-line recovery exists to serve, because a
/// length prefix says **where the next record starts**: a record that decodes badly is skipped
/// exactly, and everything after it still reads.
///
/// **The one thing that cannot be recovered from is a bad frame boundary**, and that is stated
/// rather than papered over — see [`read_back`].
pub fn read_back(path: &Path) -> Result<ChainRead, RefusalSet> {
    use std::io::Read;

    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .and_then(|mut file| file.read_to_end(&mut bytes))
        .map_err(|error| {
            let mut refusals = RefusalSet::new();
            refusals.refuse(
                RefusalKind::CorruptRecord,
                format!(
                    "{}: the chain could not be read: {error}. A record that cannot be read is a \
                     fact to report, not a chain to guess at",
                    path.display()
                ),
            );
            refusals
        })?;

    fold_bytes(&bytes)
}

/// **The frames that fold, and only those, in one pass.**
///
/// # What this is for, and the measurement that made it necessary
///
/// A chain can hold records that do not link. The doubling defect wrote the file's own history
/// again — and in the live record it did so **more than once**, which the first repair attempt
/// discovered by refusing: cutting the file at the outermost repeat left inner repeats behind and
/// the result would not fold, with **166,406 records refused**. A single cut cannot repair a file
/// that has been doubled twice, and the arithmetic says how it got there: 105,323,302 → 842,609,979
/// → 1,685,220,970 is ×8 and then ×2 of a chain that kept gaining records.
///
/// So the repair is not a cut. It is: **keep every frame that linked and drop every frame that did
/// not**, decided by [`append_event`] — the same function the reader uses to decide it. There is no
/// second opinion about linkage here, and that is the point: a repair that had its own rule could
/// disagree with the reader, and the disagreement would be discovered by whoever read the record
/// next.
///
/// # Why the surviving frames are kept byte for byte
///
/// A surviving frame is written out exactly as it was read, so its own digest — and therefore the
/// `previous` of the record after it — is unchanged. Re-rendering the events would be a new
/// encoding of a record that already has a digest, and the chain's linkage is computed over the
/// bytes that are on disk.
///
/// # What it does not do
///
/// It does not write anything, it does not decide whether a repair should happen, and it does not
/// treat "dropped something" as success. A chain that is already sound compacts to itself with
/// `dropped == 0`, and the caller is the one that must refuse in that case rather than rewriting a
/// sound record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChainCompaction {
    /// The chain as it should have been: the frames that folded, concatenated in order.
    pub bytes: Vec<u8>,
    pub kept: usize,
    pub dropped: usize,
    /// Why each dropped frame was dropped, verbatim. **A count alone cannot be fixed** — the same
    /// rule `ChainRead::corrupt_detail` follows.
    pub dropped_detail: Vec<String>,
}

pub fn compact(bytes: &[u8]) -> Result<ChainCompaction, RefusalSet> {
    let mut chain = Chain::new();
    let mut out: Vec<u8> = Vec::new();
    let mut kept = 0usize;
    let mut dropped = 0usize;
    let mut dropped_detail: Vec<String> = Vec::new();
    let mut at = 0usize;

    while at < bytes.len() {
        let Some(frame) = frame_at(bytes, at) else {
            dropped += 1;
            dropped_detail.push(format!(
                "at byte {at}: no record boundary could be found, so the walk stopped here rather \
                 than scanning forward for something that looked like a record"
            ));
            break;
        };
        let mut refused: Vec<String> = Vec::new();
        match CanonicalCodec.decode(frame) {
            Ok(projection) => {
                let schema = chain_event_schema();
                let validation = projection.validate(&schema);
                if validation.is_empty() {
                    // **The first surviving frame seeds the head**, exactly as it does in the fold.
                    if chain.is_empty() {
                        if let Some(word) = projection.get("previous").and_then(Value::as_text) {
                            match voxel_model::chain::Linkage::parse(word) {
                                Ok(linkage) => chain = voxel_model::chain::Chain::with_head(linkage),
                                Err(_) => {
                                    chain = voxel_model::chain::Chain::with_head(
                                        voxel_model::chain::Linkage::NotEvaluable,
                                    );
                                }
                            }
                        }
                    }
                    if let Err(found) = append_event(&mut chain, &projection) {
                        refused.extend(found.all().iter().map(|r| r.detail().to_string()));
                    }
                } else {
                    refused.extend(
                        validation
                            .all()
                            .iter()
                            .map(|r| format!("at byte {at}: {}", r.detail())),
                    );
                }
            }
            Err(found) => refused.extend(
                found
                    .all()
                    .iter()
                    .map(|r| format!("at byte {at}: {}", r.detail())),
            ),
        }
        if refused.is_empty() {
            out.extend_from_slice(frame);
            kept += 1;
        } else {
            dropped += 1;
            dropped_detail.extend(refused);
        }
        at += frame.len();
    }

    Ok(ChainCompaction {
        bytes: out,
        kept,
        dropped,
        dropped_detail,
    })
}

/// Fold records that are **already in memory**, or refuse.
///
/// **The body of [`read_back`], and it is separated so a writer can agree with a reader.**
/// `filing::linked_records` renders a record, then needs *that record's own digest* as the
/// next one's `previous` — and the only digest a reader will agree with is the one the reader
/// computes. So the writer folds its own output through **this** function rather than
/// recomputing a digest, and there is exactly one implementation of the wire format.
///
/// **Without this, linkage would be a second opinion.** A writer that computed `Digest` itself
/// and a reader that decoded the frame would agree until the first thing they disagreed about
/// — and then the chain would be `ChainBroken` with no way to tell which of the two was wrong.
pub fn fold_bytes(bytes: &[u8]) -> Result<ChainRead, RefusalSet> {
    let mut chain = Chain::new();
    let mut corrupt = 0usize;
    let mut corrupt_detail: Vec<String> = Vec::new();
    let mut folded = 0usize;
    let mut projections: Vec<Projection> = Vec::new();
    let mut at = 0usize;

    while at < bytes.len() {
        // **The one unresynchronisable defect.** Without a valid header there is no length to
        // trust, so there is no honest way to find where the next record begins. Every later
        // byte could be the middle of a record. So this stops the fold and says so, rather
        // than scanning forward for something that looks like a magic — which would silently
        // drop records and report a count nobody could check.
        let Some(frame) = frame_at(bytes, at) else {
            corrupt += 1;
            corrupt_detail.push(format!(
                "at byte {at}: no record boundary could be found, so the fold stopped here rather \
                 than scanning forward for something that looked like a record. {bytes_len} \
                 bytes were read",
                bytes_len = bytes.len()
            ));
            break;
        };

        match CanonicalCodec.decode(frame) {
            Ok(projection) => {
                let schema = chain_event_schema();
                let validation = projection.validate(&schema);
                if validation.is_empty() {
                    // **The first record seeds the chain's head, and only the first.**
                    //
                    // `append_event` compares a record's `previous` against the chain's head,
                    // which means the head has to be right before the first comparison — and
                    // the only thing that can say what it is, is the first record's own
                    // `previous`. A genesis digest means the writer intended a hash-linked
                    // chain; `not_evaluable` means append-only. **Neither is defaulted**, so
                    // the integrity a chain reports is the integrity its writer claimed.
                    if chain.is_empty() {
                        if let Some(word) = projection.get("previous").and_then(Value::as_text) {
                            match voxel_model::chain::Linkage::parse(word) {
                                Ok(linkage) => chain = voxel_model::chain::Chain::with_head(linkage),
                                // `append_event` refuses the unparseable case and records why;
                                // seeding with `NotEvaluable` here would only be the baseline
                                // that refusal is measured against.
                                Err(_) => {
                                    chain =
                                        voxel_model::chain::Chain::with_head(
                                            voxel_model::chain::Linkage::NotEvaluable,
                                        );
                                }
                            }
                        }
                    }
                    match append_event(&mut chain, &projection) {
                        Ok(()) => {
                            folded += 1;
                            projections.push(projection);
                        }
                        Err(found) => {
                            corrupt += 1;
                            corrupt_detail.extend(found.all().iter().map(|r| r.detail().to_string()));
                        }
                    }
                } else {
                    // **A record that decodes but does not satisfy the declaration is
                    // corrupt, and it is skippable** — the frame's own length says where the
                    // next one starts.
                    corrupt += 1;
                    corrupt_detail.extend(
                        validation
                            .all()
                            .iter()
                            .map(|r| format!("at byte {at}: {}", r.detail())),
                    );
                }
            }
            Err(found) => {
                corrupt += 1;
                corrupt_detail.extend(
                    found
                        .all()
                        .iter()
                        .map(|r| format!("at byte {at}: {}", r.detail())),
                );
            }
        }
        at += frame.len();
    }

    Ok(ChainRead {
        board: Board::from_chain(&chain),
        chain,
        corrupt,
        corrupt_detail,
        folded,
        projections,
    })
}

/// Check one closed vocabulary, **by name**, and refuse a word that is not in it.
///
/// The four axes are four different types with four different `ALL` lengths, so they are
/// checked one at a time against a `&[&str]` of names rather than collected into one array —
/// an array of four different slice types does not exist, and coercing them into one would
/// mean a trait object where a plain comparison will do.
fn check_axis(
    refusals: &mut RefusalSet,
    text: &dyn Fn(&str) -> Result<String, RefusalSet>,
    field: &str,
    declared: &[&'static str],
) {
    match text(field) {
        Ok(word) => {
            if !declared.contains(&word.as_str()) {
                refusals.refuse(
                    RefusalKind::UnknownComponent,
                    format!(
                        "{field} is {word:?}, which is not one of the {} declared values ({}). \
                         Refused by name rather than defaulted: a record whose axis is silently \
                         replaced by a plausible one is a record that can be read two ways",
                        declared.len(),
                        declared.join(", ")
                    ),
                );
            }
        }
        Err(found) => {
            refusals.absorb(found);
        }
    }
}

/// One refusal, as a set. A small helper so a `Result<_, String>` from a `parse` becomes a
/// [`RefusalSet`] without every call site repeating the three lines.
fn refusal(kind: RefusalKind, detail: impl Into<String>) -> RefusalSet {
    let mut refusals = RefusalSet::new();
    refusals.refuse(kind, detail);
    refusals
}

/// Everything a writer supplies for one chain event.
///
/// **A struct rather than twelve parameters**, and the reason is the schema: this type's
/// fields *are* the declared fields, one for one, so a reader can hold the struct next to
/// [`chain_event_schema`] and check them against each other. Twelve positional arguments in the
/// same order as the schema is the same information with the labels thrown away, and the labels
/// are the part that stops a caller transposing `state` and `control`.
///
/// # Why four of these are enums and not `&str`
///
/// **`event_type`, `kind`, `state` and `control` were all `&'a str`, and the wire format's
/// refuse-never-default rule did not reach this layer because of it.** Every one of the four
/// vocabularies was *already declared* — [`EventType`] and [`Kind`] counted from the reference
/// file (51/10/2/4/9 of 76, 56/12/4/2/1/1), [`RecordState`] three values, and [`Control`] four
/// from hyperskill's own hypergrilling loop — and the schema's `doc` string for each field
/// spelled the same values out in prose. So the declarations were complete and unread.
///
/// **What that cost, concretely:** a caller could write `"banana"` as an `event_type` and the
/// record would be written, because `render_event` copies the `&str` straight into the
/// projection and the schema checks the field's *type*, not its value. Every one of those four
/// axes was a closed vocabulary in the record and an open string in the writer. That is C-69's
/// *declared list nobody reads is a fiction*, at the layer every record in this project passes
/// through.
///
/// **Typing them is the whole fix, and it is a compile error rather than a runtime refusal on
/// purpose** — the refusal already exists ([`EventType::parse`] and its three siblings each
/// refuse by name) and a refusal at run time is a refusal a caller can forget to handle. A
/// value that is not in the vocabulary **cannot be written down**, so the record cannot hold
/// one.
///
/// **The fifth axis, `event_kind`, was already typed** as [`EventKind`], which is why the
/// omission of the other four is visible as an omission rather than as a design: the same
/// struct, one field typed and four not, with all five vocabularies declared in the same
/// module.
#[derive(Clone, Copy, Debug)]
pub struct FiledRecord<'a> {
    pub namespace: &'a Namespace,
    /// The chain's own kind — the operational axis.
    pub event_kind: EventKind,
    /// The question. **For a `ticket_transition` this is the objective the player typed.**
    pub question: &'a str,
    pub answer: Option<&'a str>,
    // The design record's axes. **Typed, not `&'a str`** — see the note on [`FiledRecord`].
    pub kind: Kind,
    pub event_type: EventType,
    pub state: RecordState,
    pub control: Control,
    pub actor: &'a str,
    pub round_id: &'a str,
    pub supersedes: Option<&'a str>,
    /// The link to the previous record, or the literal `not_evaluable`.
    pub previous: &'a str,
}

/// One record's fields **without its linkage** — the shape a writer can be handed.
///
/// [`FiledRecord`] requires a `previous` the caller has to know, and a caller does not know it:
/// it is the digest of a record that does not exist yet. So the chokepoint takes *this* and
/// fills the linkage in, which is what makes "one writer computes the linkage" enforceable
/// rather than a convention. The alternative — a `&str` on the public type — is exactly the
/// C-76 defect one layer up: a closed value that any string can stand for.
#[derive(Clone, Copy, Debug)]
pub struct UnlinkedRecord<'a> {
    pub namespace: &'a Namespace,
    pub event_kind: EventKind,
    pub question: &'a str,
    pub answer: Option<&'a str>,
    pub kind: Kind,
    pub event_type: EventType,
    pub state: RecordState,
    pub control: Control,
    pub actor: &'a str,
    pub round_id: &'a str,
    pub supersedes: Option<&'a str>,
}

impl<'a> UnlinkedRecord<'a> {
    /// This record, linked to `previous`.
    pub const fn linked(self, previous: &'a str) -> FiledRecord<'a> {
        FiledRecord {
            namespace: self.namespace,
            event_kind: self.event_kind,
            question: self.question,
            answer: self.answer,
            kind: self.kind,
            event_type: self.event_type,
            state: self.state,
            control: self.control,
            actor: self.actor,
            round_id: self.round_id,
            supersedes: self.supersedes,
            previous,
        }
    }
}

/// The digest of the last complete record in `bytes`, or `None` if there is none.
///
/// **A convenience over [`fold_bytes`], and it returns the *head* rather than a re-computed
/// digest** so the answer is by construction the same number a reader will arrive at.
pub fn digest_of_last(bytes: &[u8]) -> Option<String> {
    let read = fold_bytes(bytes).ok()?;
    if read.folded == 0 {
        return None;
    }
    Some(voxel_model::chain::Linkage::Linked(read.chain.digest()).render())
}

/// Render one chain event as a record on disk, or refuse.
///
/// The in-game client calls this when a player files something, and it lives here rather than
/// in the client so that **the writer and the reader cannot disagree about the format** — the
/// same reason the ticket is a projection of the chain and not a second store.
///
/// Every value is checked against the declared schema before it is written, so a record that
/// would be refused on the way in is refused on the way out too, where the mistake is.
pub fn render_event(record: FiledRecord<'_>) -> Result<Vec<u8>, RefusalSet> {
    let namespace = record.namespace;
    let mut projection = Projection::new(voxel_model::wire::CHAIN_EVENT_SCHEMA)
        .with("world", Value::Text(namespace.world.clone()))
        .with("year", Value::Integer(namespace.year as i64))
        .with("season", Value::Text(namespace.season.name().to_string()))
        .with("campaign", Value::Text(namespace.campaign.clone()))
        .with("event_kind", Value::Text(record.event_kind.name().to_string()))
        .with("event_type", Value::Text(record.event_type.name().to_string()))
        .with("kind", Value::Text(record.kind.name().to_string()))
        .with("state", Value::Text(record.state.name().to_string()))
        .with("control", Value::Text(record.control.name().to_string()))
        .with("actor", Value::Text(record.actor.to_string()))
        .with("round_id", Value::Text(record.round_id.to_string()))
        .with("question", Value::Text(record.question.to_string()))
        .with("previous", Value::Text(record.previous.to_string()));
    if let Some(answer) = record.answer {
        projection = projection.with("answer", Value::Text(answer.to_string()));
    }
    if let Some(supersedes) = record.supersedes {
        projection = projection.with("supersedes", Value::Text(supersedes.to_string()));
    }

    // **Checked before written.** A record the reader would refuse is a record the writer must
    // not produce, and finding that out here names the caller's mistake rather than the
    // reader's.
    let refusals = projection.validate(&chain_event_schema());
    if !refusals.is_empty() {
        return Err(refusals);
    }
    projection.canonical_bytes()
}

/// The bytes of the record starting at `at`, or `None` if there is no boundary there.
///
/// A record is `magic · frame_version · schema_len · schema · schema_version · field_count`
/// followed by that many length-described fields. The header is walked rather than assumed,
/// so a boundary is only reported where one genuinely is.
fn frame_at(bytes: &[u8], at: usize) -> Option<&[u8]> {
    let mut cursor = at;

    // magic
    let magic = bytes.get(cursor..cursor + 4)?;
    if magic != WIRE_MAGIC {
        return None;
    }
    cursor += 4;
    // frame version
    cursor += 2;

    // schema name, length-prefixed
    let schema_len = u32_at(bytes, cursor)? as usize;
    cursor += 4;
    // **Assigned, not merely checked.** `checked_add(..)?` as a statement guards the
    // overflow and throws the value away, which leaves the cursor exactly where it was — and
    // then every length after it is read from the middle of a name.
    cursor = cursor.checked_add(schema_len)?;

    // schema version, then field count
    cursor += 2;
    let field_count = u32_at(bytes, cursor)? as usize;
    cursor += 4;

    for _ in 0..field_count {
        let name_len = u32_at(bytes, cursor)? as usize;
        cursor += 4;
        cursor = cursor.checked_add(name_len)?;
        let payload_len = u32_at(bytes, cursor)? as usize;
        cursor += 4;
        cursor = cursor.checked_add(payload_len)?;
    }

    bytes.get(at..cursor)
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    let slice = bytes.get(at..at + 4)?;
    Some(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

/// Fold one validated projection onto the chain.
///
/// # What carries across, and what does not
///
/// The chain event carries the **operational** half — namespace, the chain's own `EventKind`,
/// and the question as the event's detail. The envelope's descriptive axes (`event_type`,
/// `kind`, `state`, `control`, `actor`, `round_id`, `supersedes`) are **checked** by
/// `validate` and then **not carried**, because `chain::Event` has nowhere to put them and
/// adding a slot for them would be a second place the same fact lives.
///
/// **That is recorded rather than hidden.** The consequence is stated plainly: a reader of the
/// folded chain has the ticket text, its namespace and its integrity, and does **not** have
/// the design record's classification. `ChainRead` keeps the projections for anyone who needs
/// them, so the loss is visible instead of assumed away.
fn append_event(chain: &mut Chain, projection: &Projection) -> Result<(), RefusalSet> {
    let mut refusals = RefusalSet::new();

    // Every read is a **refusing read**: a missing field is `None`, and `None` is turned into
    // a refusal here rather than into a default. The schema already proved the field is
    // present, so a `None` at this point means the two disagree — which is worth saying.
    let text = |name: &str| -> Result<String, RefusalSet> {
        match projection.get(name).and_then(Value::as_text) {
            Some(value) => Ok(value.to_string()),
            None => {
                let mut found = RefusalSet::new();
                found.refuse(
                    RefusalKind::CorruptRecord,
                    format!(
                        "field {name:?} is absent or is not text, so this record cannot become an \
                         event. Not defaulted"
                    ),
                );
                Err(found)
            }
        }
    };

    let world = match text("world") {
        Ok(value) => value,
        Err(found) => {
            refusals.absorb(found);
            return Err(refusals);
        }
    };
    let campaign = match text("campaign") {
        Ok(value) => value,
        Err(found) => {
            refusals.absorb(found);
            return Err(refusals);
        }
    };
    let year = match projection.get("year").and_then(Value::as_integer) {
        Some(value) if value >= 0 => value as u64,
        Some(value) => {
            refusals.refuse(
                RefusalKind::CorruptRecord,
                format!("year is {value}, which is not a year. Not clamped to zero"),
            );
            return Err(refusals);
        }
        None => {
            refusals.refuse(
                RefusalKind::CorruptRecord,
                "year is absent or not an integer. Not defaulted to the first year",
            );
            return Err(refusals);
        }
    };
    let season = match text("season").and_then(|word| {
        Season::parse(&word).map_err(|why| refusal(RefusalKind::UnknownComponent, why))
    }) {
        Ok(value) => value,
        Err(found) => {
            refusals.absorb(found);
            return Err(refusals);
        }
    };
    let event_kind = match text("event_kind").and_then(|word| {
        EventKind::parse(&word).map_err(|why| refusal(RefusalKind::UnknownComponent, why))
    }) {
        Ok(value) => value,
        Err(found) => {
            refusals.absorb(found);
            return Err(refusals);
        }
    };
    // **The question is the event's detail, and the reason is the in-game form.** A player
    // filing a ticket types an objective; the record's question is the thing that objective
    // answers. So for a `ticket_transition` they are the same text, and stating that here is
    // what keeps it from being a silent conflation.
    let detail = match text("question") {
        Ok(value) => value,
        Err(found) => {
            refusals.absorb(found);
            return Err(refusals);
        }
    };

    // **The axes are checked even though the chain cannot carry them.** A record whose `kind`
    // is misspelled would otherwise fold cleanly and have its axis silently dropped, which
    // makes a typo invisible on the one path where it could still be caught. The chain has no
    // slot for these fields, so **the check is the only thing standing between a bad word and
    // a record nobody can audit** — it is refused here, and the value is deliberately not
    // carried.
    check_axis(&mut refusals, &text, "kind", &voxel_model::envelope::Kind::ALL.map(|k| k.name()));
    check_axis(
        &mut refusals,
        &text,
        "event_type",
        &voxel_model::envelope::EventType::ALL.map(|k| k.name()),
    );
    check_axis(
        &mut refusals,
        &text,
        "state",
        &voxel_model::envelope::RecordState::ALL.map(|k| k.name()),
    );
    check_axis(
        &mut refusals,
        &text,
        "control",
        &voxel_model::envelope::Control::ALL.map(|k| k.name()),
    );

    // **The linkage is parsed, checked against the chain, and only then believed.**
    //
    // **The first version of this parsed the word and stopped.** The comment above it claimed
    // *"a record that claims a link it does not have is refused before it is folded"*, and the
    // code did not do that: `chain.append` computes `previous` from its own head and **ignores
    // the field entirely**, so a record could claim any predecessor at all and fold cleanly.
    //
    // **That is the C-69 defect in its purest form** — a comment describing behaviour the code
    // does not have — and it was load-bearing: it is the difference between detecting a
    // rewritten record and accepting one. `ChainBroken` existed for exactly this and was
    // unreachable for it.
    //
    // **The first record is the exception, and it is checked, not assumed.** An empty chain has
    // no head to compare against, so the first record *chooses* the chain's integrity: a
    // genesis digest means hash-linked, `not_evaluable` means append-only. That choice is
    // visible in the file rather than defaulted by the reader, which is what
    // `Chain::with_head` exists for.
    let claimed = match text("previous") {
        Ok(word) => match voxel_model::chain::Linkage::parse(&word) {
            Ok(linkage) => linkage,
            Err(why) => {
                refusals.refuse(RefusalKind::ChainBroken, why);
                voxel_model::chain::Linkage::NotEvaluable
            }
        },
        Err(_) => {
            refusals.refuse(
                RefusalKind::ChainBroken,
                "a record with no `previous` field cannot say what it follows, so it is not a \
                 record in a chain",
            );
            voxel_model::chain::Linkage::NotEvaluable
        }
    };
    if !refusals.is_empty() {
        return Err(refusals);
    }

    let namespace = match Namespace::new(world, year, season, campaign) {
        Ok(value) => value,
        Err(found) => {
            refusals.absorb(found);
            return Err(refusals);
        }
    };

    // **And now the comparison, which is the part that was missing.** `chain.is_empty()`
    // distinguishes "this record chose the chain's integrity" from "this record must match
    // what came before it".
    if chain.is_empty() {
        // **The fold seeds the head from this record, so the first record has nothing to be
        // compared against** — and that is not a hole, because the fold already read this
        // record's `previous` to choose the head. Comparing it to itself would be circular.
    } else {
        let expected = chain.digest();
        let expected = voxel_model::chain::Linkage::Linked(expected);
        if claimed != expected {
            refusals.refuse(
                RefusalKind::ChainBroken,
                format!(
                    "this record claims previous={} but the record before it hashes to {}. A \
                     record that names the wrong predecessor is a rewritten record, which is the \
                     one thing a hash chain exists to catch",
                    claimed.render(),
                    expected.render()
                ),
            );
            return Err(refusals);
        }
    }

    match chain.append(namespace, event_kind, detail) {
        Ok(_) => Ok(()),
        Err(found) => {
            refusals.absorb(found);
            Err(refusals)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_model::chain::{Namespace, Season};
    use voxel_model::ticket::{Gate, TicketKind, TicketStatus};
    use voxel_model::RefusalKind as Kind;
    fn namespace() -> Namespace {
        Namespace::new("world-2468", 0, Season::Spring, "C-67").expect("a named namespace")
    }

    fn grant() -> Grant {
        Grant::new("user", "compile a solution for a filed ticket").expect("a scoped grant")
    }

    fn ticket(objective: &str) -> Ticket {
        Ticket {
            id: TicketId::new("BLD-0000").unwrap(),
            kind: TicketKind::Build,
            status: TicketStatus::Open,
            objective: objective.to_string(),
            gate: Gate::new("the arrangement carries the named window", Expectation::None).unwrap(),
            actor: "user".into(),
            opened_tick: 0,
            event_index: 0,
            digest: Digest::of(b"event"),
            evidence: Vec::new(),
            retirement: None,
            corrected: None,
        }
    }

    // ---- the missing milestone, and it runs -------------------------------

    #[test]
    fn the_emitter_produces_a_package_from_a_ticket_on_the_chain() {
        // **The thing 11-build-order.md §2 F says no milestone had.** A format with no emitter.
        let mut chain = Chain::new();
        let ticket = ticket("mount window[console] and window[log]");
        chain
            .append(
                namespace(),
                voxel_model::chain::EventKind::TicketTransition,
                "[BLD] mount window[console] and window[log]",
            )
            .expect("the ticket is on the record");

        let (package, receipt) =
            emit(&mut chain, &ticket, Some(&grant())).expect("an audited compile");
        assert!(!package.is_empty());
        assert_eq!(receipt.name, "BLD-0000");
        assert_eq!(receipt.record_ids.len(), 1);
        // Five stages, five audit entries — allowed or denied, all recorded.
        assert_eq!(receipt.audited, Stage::COUNT);
    }

    #[test]
    fn every_stage_is_audited_and_the_audit_is_on_the_chain() {
        let mut chain = Chain::new();
        chain
            .append(
                namespace(),
                voxel_model::chain::EventKind::TicketTransition,
                "[BLD] mount window[console]",
            )
            .unwrap();
        let before = chain.len();
        let mut auditor = Auditor::new();
        for stage in Stage::ALL {
            auditor.gate(&mut chain, stage, Some(&grant())).expect("each stage audits");
        }
        assert_eq!(auditor.entries().len(), Stage::COUNT);
        assert!(auditor.all_allowed());
        // **Each decision is an event**, so the audit is on the record and not only in a report.
        assert_eq!(chain.len(), before + Stage::COUNT);
    }

    #[test]
    fn a_denied_stage_stops_the_compile_and_is_still_recorded() {
        // **The build must not be capable of forgetting** (H-10.1), for the compiler and not
        // only for the build gate.
        let mut chain = Chain::new();
        let mut auditor = Auditor::new();
        // No grant: `Render` is `AppendEvent`, and appending to the chain is `RequiresGrant`.
        let error = auditor
            .gate(&mut chain, Stage::Render, None)
            .expect_err("a chain write without a grant is refused");
        assert_eq!(error.all()[0].kind(), Kind::UntrustedKey);
        assert!(!auditor.all_allowed(), "the audit records the denial");
        // And the denial is an event — a refusal nobody can read did not happen.
        assert_eq!(chain.len(), 1);
        assert!(chain.fold(false, |_, event| event.kind
            == voxel_model::chain::EventKind::RefusalRecorded));
    }

    #[test]
    fn a_whole_compile_without_a_grant_stops_at_the_first_write_stage() {
        let mut chain = Chain::new();
        let ticket = ticket("mount window[console]");
        chain
            .append(
                namespace(),
                voxel_model::chain::EventKind::TicketTransition,
                "[BLD] mount window[console]",
            )
            .unwrap();
        // `read_tickets` is a read and is allowed; `derive_solution` appends to the chain and
        // needs a grant. So the compile stops at the first stage that writes.
        let error = emit(&mut chain, &ticket, None).expect_err("no grant, no compile");
        assert_eq!(error.all()[0].kind(), Kind::UntrustedKey);
    }

    // ---- the derivation, and its refusal to invent -----------------------

    #[test]
    fn a_solution_names_only_the_windows_the_ticket_mentions() {
        let ticket = ticket("mount window[console] and window[ledger]");
        let solution = Solution::for_ticket(&ticket).expect("a derivation");
        assert_eq!(solution.windows.len(), 2);
        assert_eq!(solution.windows[0].id, "console");
        assert_eq!(solution.windows[1].id, "ledger");
        // The title is the id verbatim — nothing this module prettified.
        assert_eq!(solution.windows[0].title, "console");
        // And the record id points at the event that declared it.
        assert_eq!(solution.windows[0].record_id, "event-0000");
    }

    #[test]
    fn a_ticket_naming_no_window_compiles_to_a_solution_with_none() {
        // **The correction.** This used to be the refusal that mattered — "a compiler that invents
        // a window can put a window on a player's screen that nobody asked for" — and the refusal
        // was the misreading: a solution needs no UI surface, so a ticket that asks for none gets
        // none. What must never happen is the *other* thing: a window appearing because the ticket
        // was prose. So the assertion is that the solution compiles and declares **nothing**.
        let solution = Solution::for_ticket(&ticket("please make the shell nicer"))
            .expect("a solution needs no UI surface, so this compiles");
        assert!(
            solution.windows.is_empty(),
            "no window was named, so none is declared — not one invented to satisfy the compiler: {:?}",
            solution.windows.iter().map(|w| &w.id).collect::<Vec<_>>()
        );
        // And it is still a real solution: the ticket and its event are carried, so the record id
        // points at the chain event that declared it.
        assert_eq!(solution.ticket.as_str(), "BLD-0000");
        assert_eq!(solution.record_ids, vec!["event-0000".to_string()]);
    }

    #[test]
    fn prose_does_not_become_windows() {
        // A compiler that picked up ordinary nouns would emit a window per word, and nobody
        // could review that. **The reading is where the strictness lives**, and it is unchanged by
        // the correction above: ordinary words are not ids, so they yield no windows at all.
        let solution = Solution::for_ticket(&ticket("the console should log the ledger"))
            .expect("prose compiles to a solution with nothing in it");
        assert!(
            solution.windows.is_empty(),
            "ordinary words are not window ids: {:?}",
            solution.windows.iter().map(|w| &w.id).collect::<Vec<_>>()
        );
    }

    // ---- the bare form, which is the ruling C-67.7 was owed ----------------

    #[test]
    fn a_line_that_is_one_token_is_a_bare_window_id() {
        // **What a person actually types.** C-67.7's complaint was exact: `window[id]` is a
        // machine grammar and a player will not type brackets. `file refusals` has to work.
        let solution = Solution::for_ticket(&ticket("refusals")).expect("a bare id is a declaration");
        assert_eq!(solution.windows.len(), 1);
        assert_eq!(solution.windows[0].id, "refusals");
        // And it is a real window, not a stub: the same record id the bracket form gets.
        assert_eq!(solution.windows[0].record_id, "event-0000");
    }

    #[test]
    fn the_bare_form_and_the_bracket_form_declare_the_same_window() {
        // **One grammar, two spellings, one result** — or the bare form is a second dialect that
        // happens to compile, and the two can drift apart without anything noticing.
        let bare = Solution::for_ticket(&ticket("refusals")).expect("a bare id");
        let bracketed = Solution::for_ticket(&ticket("window[refusals]")).expect("a bracketed id");
        assert_eq!(bare.windows.len(), bracketed.windows.len());
        assert_eq!(bare.windows[0].id, bracketed.windows[0].id);
        // And `window=refusals` / `window:refusals` too, because they were already rules and the
        // bare form must not have become the only spelling that works.
        for spelling in ["window=refusals", "window:refusals"] {
            let solution = Solution::for_ticket(&ticket(spelling)).expect(spelling);
            assert_eq!(solution.windows[0].id, "refusals", "{spelling}");
        }
    }

    #[test]
    fn a_prose_line_naming_a_window_in_words_declares_nothing() {
        // **The exact line C-67.7 named as the thing to be ruled on.** Six tokens, so it is prose,
        // so it declares nothing — and it compiles, because nothing was named and a solution needs
        // no UI surface. This is the test that would go red if the bare form were widened to "any
        // bare token in the line": `named` and `refusals` would each become a window, and the
        // solution would stop being empty.
        let solution = Solution::for_ticket(&ticket("add a window named refusals"))
            .expect("six tokens is prose, and prose compiles to no windows");
        assert!(
            solution.windows.is_empty(),
            "six tokens is prose, whatever the words are: {:?}",
            solution.windows.iter().map(|w| &w.id).collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_bare_token_the_recipe_could_not_carry_is_not_a_window_id() {
        // The character set is the recipe's, and this is where that claim is tested rather than
        // asserted. A single token carrying punctuation is a machine token that failed to parse,
        // so it is not quietly promoted into an id.
        assert_eq!(single_token_id("window[refusals]"), None, "a bracketed token is not bare");
        assert_eq!(single_token_id("refusals."), None, "trailing punctuation is not an id");
        assert_eq!(single_token_id(""), None, "an empty objective is not an id");
        assert_eq!(single_token_id("  refusals  "), Some("refusals".to_string()), "padding is not a token");
        assert_eq!(single_token_id("refu sals"), None, "two tokens is prose");
    }

    #[test]
    fn a_closed_ticket_is_not_recompiled() {
        let mut closed = ticket("mount window[console]");
        closed.status = TicketStatus::Complete;
        let error = Solution::for_ticket(&closed).expect_err("a closed ticket is superseded, not reopened");
        assert_eq!(error.all()[0].kind(), Kind::UngovernedStatus);
    }

    #[test]
    fn an_arrangement_over_the_window_ceiling_is_refused_not_truncated() {
        let names: Vec<String> = (0..recipe::MAX_WINDOWS + 1)
            .map(|index| format!("window[w{index}]"))
            .collect();
        let error = Solution::for_ticket(&ticket(&names.join(" ")))
            .expect_err("too many windows");
        assert_eq!(error.all()[0].kind(), Kind::OutOfDeclaredRange);
        assert!(error.all()[0].detail().contains("silently lost a window"), "{error}");
    }

    // ---- the render, and the round trip ----------------------------------

    #[test]
    fn a_rendered_solution_compiles_and_round_trips() {
        let solution = Solution::for_ticket(&ticket("mount window[console]")).unwrap();
        let source = render(&solution).expect("a rendering");
        let package = recipe::compile(&source).expect("it compiles");
        let loaded = recipe::load(&package).expect("and loads back");
        assert_eq!(loaded.source, source, "the package carries its exact source");
        assert_eq!(loaded.recipe.windows.len(), 1);
        assert_eq!(loaded.recipe.windows[0].id, "console");
    }

    #[test]
    fn a_prose_ticket_renders_and_compiles_to_a_package_with_no_window() {
        // **The whole path, for the person who filed it.** A plain-prose ticket has to file and
        // compile: a solution needs no UI surface, so there is nothing here to refuse. The
        // refusal this replaces ("names no window") demanded a window id, and a demand nobody
        // filed is the compiler conjuring up a requirement — the exact failure A10 corrects.
        //
        // **And the other half is pinned with it**: what comes out declares **no** window, so
        // the correction did not smuggle a window onto a player's screen instead. Nothing was
        // conjured in either direction.
        let solution = Solution::for_ticket(&ticket("the console should log the ledger"))
            .expect("plain prose compiles");
        let source = render(&solution).expect("a rendering");
        let package = recipe::compile(&source).expect("the package compiles");
        let loaded = recipe::load(&package).expect("and loads back");
        assert_eq!(loaded.source, source, "the package carries its exact source");
        assert!(
            loaded.recipe.windows.is_empty(),
            "no window was named, so the package declares none: {}",
            loaded.recipe.windows.len()
        );
        // Provenance is unaffected by the arrangement being empty: the two files still have to
        // agree, which is the check a package with nothing in it is most likely to skip.
        verify_provenance(&source, &package, &solution).expect("provenance verifies");
    }

    #[test]
    fn provenance_is_verified_by_round_trip_and_not_by_assertion() {
        let solution = Solution::for_ticket(&ticket("mount window[console]")).unwrap();
        let source = render(&solution).unwrap();
        let package = recipe::compile(&source).unwrap();
        verify_provenance(&source, &package, &solution).expect("an honest compile verifies");

        // A package that does not match its source is refused.
        let other = recipe::compile(&render(
            &Solution::for_ticket(&ticket("mount window[other]")).unwrap(),
        )
        .unwrap())
        .unwrap();
        let error = verify_provenance(&source, &other, &solution)
            .expect_err("two files that merely exist are not two files that agree");
        assert_eq!(error.all()[0].kind(), Kind::DigestMismatch);
    }

    #[test]
    fn a_compiler_does_not_verify_the_world_and_does_not_claim_to() {
        // The separation: the compiler produces the declaration; the ticket's read-back
        // validates it. A compile that read the world would claim a verification it did not
        // perform.
        let solution = Solution::for_ticket(&ticket("mount window[console]")).unwrap();
        assert!(!solution.would_validate(ReadBack::NotChecked));
        assert!(!solution.would_validate(ReadBack::NotHeld));
        // An expectation with nothing in it cannot validate even on a held read-back.
        assert!(!solution.would_validate(ReadBack::Held), "no claim, nothing to validate");
    }

    // ---- the stage table --------------------------------------------------

    #[test]
    fn every_stage_names_an_action_and_every_action_is_one_the_matrix_has() {
        // The reason `Stage::action` exists: a new stage cannot be added without a permission
        // that already exists in the matrix.
        for stage in Stage::ALL {
            let action = stage.action();
            // `decide` answers for every action, and a name it does not know is impossible
            // because `Action` is closed — so this asserts the pair works together.
            let _decision = policy::decide(action, None);
            assert!(!action.name().is_empty(), "{} names its action", stage.name());
            assert!(
                !action.describes().trim().is_empty(),
                "{} must produce a described action: an action nobody can describe is an action \
                 nobody can refuse correctly",
                stage.name()
            );
        }
    }

    #[test]
    fn the_two_reader_stages_are_reads_and_the_two_chain_writes_need_a_grant() {
        // Stated as a property, because it is the audit's shape: reads pass, chain writes do
        // not. **`Compile` is deliberately not in the second list** — compiling a declared
        // recipe into a package is a pure function over text, the matrix rules it
        // `AlwaysAllowed`, and a compile that needed a grant would mean the build gate needed
        // one on every build. The test says what is true rather than what reads symmetrically.
        assert!(policy::decide(Stage::ReadTickets.action(), None) == Decision::Allowed);
        assert!(policy::decide(Stage::VerifyProvenance.action(), None) == Decision::Allowed);
        for stage in [Stage::DeriveSolution, Stage::Render] {
            assert!(
                policy::decide(stage.action(), None) != Decision::Allowed,
                "{} appends to the chain and must need a grant",
                stage.name()
            );
            assert!(
                // **`Granted`, not `Allowed`.** The matrix distinguishes "always allowed"
                // from "allowed because somebody said so", and a test that collapsed the two
                // would say nothing about either.
                matches!(
                    policy::decide(stage.action(), Some(&grant())),
                    Decision::Granted { .. }
                ),
                "{} is permitted under a scoped grant, and the decision says who said so",
                stage.name()
            );
        }
        assert!(
            policy::decide(Stage::Compile.action(), None) == Decision::Allowed,
            "and compiling declared text is always allowed, which is why the gate can run it"
        );
    }
}
