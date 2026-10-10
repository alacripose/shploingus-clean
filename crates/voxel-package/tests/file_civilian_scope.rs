//! **One ticket, filed deliberately: the civilians scope is not reduced (event-0127).**
//!
//! # Why this file exists rather than a row in `file_work.rs`
//!
//! `file_work.rs` files its 42 rows as a set and **refuses a partial state** — measure it and
//! it demands all-or-none, because an append-only record plus a non-idempotent writer is
//! unrecoverable and the duplicates it already carries are the proof. So a *new* ticket cannot
//! be added there without either re-filing the 42 or loosening a guard that exists for a
//! measured reason. This is the third shape: **one ticket, one writer, one deliberate run**,
//! which is what `file_questions.rs` and `file_reference_gaps.rs` already are.
//!
//! # What is filed, and why it is a ticket rather than a ledger note
//!
//! The user's ruling, 2026-10-02, after seeing `minimal` in the agent's chain of thought:
//!
//! > *"Reducing the civilians scope is not allowed, you are killing them by doing so as I do
//! > not think you have a full grasp on what the civilians actually did"*
//!
//! The ledger carries the infraction (`event-0127`, illegal-word occurrence 12, strike 7).
//! **The ticket carries the work** — the record's own split: the chain is what the game reads,
//! so a ruling that never becomes a ticket is a ruling the next reader of the board cannot see.
//!
//! # The one-shot discipline, kept
//!
//! It writes the canonical record, so it is `#[ignore]`d and run deliberately:
//!
//! ```text
//! cargo test --offline -p voxel-package --test file_civilian_scope -- --ignored --nocapture
//! ```
//!
//! A second run is a **state, not a failure**: the objective's head is looked for on the board
//! first, with the kind marker stripped the way `Board::from_chain` strips it, and the run
//! reports *already on the record* and returns. `filing::grow` still refuses a `prior` that is
//! not the bytes on disk, so a stale read cannot append beside a changed chain.

use voxel_model::chain::{EventKind, Namespace, Season};
use voxel_model::envelope::{Control, EventType, Kind, RecordState};
use voxel_model::ticket::Board;
use voxel_package::compiler::UnlinkedRecord;
use voxel_package::features::workspace_root;
use voxel_package::filing;

/// The work, in the board's own grammar: `[INT]` is the marker `split_kind` reads, and the
/// rest is what a later reader checks against the record.
const OBJECTIVE: &str = "[INT] port the whole civilian process into laya, with its scope unreduced: \
                         citizens, organizations, contracts, jobs, commissions, evidence and \
                         grading, and security — read whole before any port row lands";

/// **The `because` is not a comment.** A ticket whose objective is a bare imperative cannot be
/// checked by a later reader, so the ruling that declares it travels with it.
const BECAUSE: &str = "event-0127 (illegal word occurrence 12, strike 7), the user's ruling: \
                       reducing the civilians scope is not allowed — the agent's chain of thought \
                       had proposed the minimal real wiring, a civilian port of record shapes plus \
                       a jobs tick. The whole process is ported: GOD_AGENTS sections 27-30, the \
                       ala-cities src/agency, src/gov and src/sim, and the store's own records \
                       (civilian, job, claim, ticket, evidence, genesis, income, promotions, \
                       retirements, metrics, sources, bills) are read as the shape before the port.";

/// The namespace every filed ticket in this record belongs to.
fn namespace() -> Namespace {
    Namespace::new("world-2468", 1, Season::Winter, "c-2026-10-01").expect("a namespace")
}

/// The head the board will hold, **with the marker stripped by the board's own rule**.
fn head() -> String {
    OBJECTIVE.trim_start_matches("[INT] ").chars().take(48).collect()
}

#[test]
#[ignore = "files one ticket onto the canonical record; run it deliberately, once"]
fn file_the_civilian_scope_ruling() {
    let root = workspace_root(&std::env::current_dir().expect("a current directory"))
        .expect("the workspace root");
    assert!(
        filing::is_filed(&root),
        "there is no chain to extend — `grow` extends one and refuses to create one"
    );

    let before = std::fs::read(filing::chain_path(&root)).expect("the record reads");
    let board_before = Board::from_chain(&filing::read(&root));

    // **Already filed is a state, not a failure.** The comparison is on the board's own
    // holdings, because the marker is stripped before the board ever sees the objective — a
    // check against the constant would be false for every run (`file_work.rs` paid for that).
    let head = head();
    if board_before
        .tickets()
        .iter()
        .any(|ticket| ticket.objective.contains(head.as_str()))
    {
        println!(
            "this ruling is already on the record at {} (the board holds {} tickets in total) — \
             nothing to do",
            filing::chain_path(&root).display(),
            board_before.len()
        );
        return;
    }

    let namespace = namespace();
    let record = UnlinkedRecord {
        namespace: &namespace,
        event_kind: EventKind::TicketTransition,
        question: OBJECTIVE,
        answer: Some(BECAUSE),
        kind: Kind::Authority,
        event_type: EventType::UserDecision,
        state: RecordState::Answered,
        control: Control::Continue,
        actor: "agent",
        round_id: "c-2026-10-01",
        supersedes: None,
    };
    filing::grow(&root, &before, std::iter::once(record)).expect("the ruling is filed");

    // **The record grew and kept everything.** `grow` opens for append and never for write, so
    // this is a property of the code rather than a check — and it is asserted because a
    // property nobody checks is a property that can change.
    let after = std::fs::read(filing::chain_path(&root)).expect("the record reads");
    assert!(after.len() > before.len(), "the record grew");
    assert!(
        after.starts_with(&before),
        "and nothing already filed was shortened"
    );

    let board = Board::from_chain(&filing::read(&root));
    assert!(
        board
            .tickets()
            .iter()
            .any(|ticket| ticket.objective.contains(head.as_str())),
        "**the ruling is on the record now**, not in this test's own memory"
    );
    println!(
        "filed: {:?}\nthe board holds {} tickets",
        OBJECTIVE,
        board.len()
    );
}
