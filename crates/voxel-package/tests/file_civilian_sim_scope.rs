//! **One ticket, filed deliberately: the sim world is part of the process (event-0136, strike 10).**
//!
//! # What happened, and why this is a ticket rather than a ledger note
//!
//! The ledger already carries the infraction. In the agent's **visible chain of thought** — the
//! host renders internal reasoning, which is recorded as a host limitation and not suppressed —
//! the sim side was about to be *resolved for the milestone*: `BuildingKind`, `MaterialLineage`,
//! `Zone`, `World`, `SIM_HZ`, `Terrain` and `CitizenState` mapped or stubbed rather than read
//! whole. The user caught it:
//!
//! > *"ticket to be logged, an infraction has been detected, reducing scope as according to old
//! > projects decision ledgers"*
//!
//! That is the reduction `event-0127` already ruled out — *"reducing the civilians scope is not
//! allowed"*. The sim world is not incidental to the civilians: the towns tick, the buildings and
//! their kinds are what the jobs and the contracts are about, and the whole thing was named as
//! useful to **the laya observer and the sound engine**. So the work is: read it whole, then port
//! it. `ledger` holds the strike; **this ticket holds the work**, because the chain is what the
//! next reader of the board sees.
//!
//! # The one-shot discipline, kept
//!
//! It writes the canonical record, so it is `#[ignore]`d and run deliberately:
//!
//! ```text
//! cargo test --offline -p voxel-package --test file_civilian_sim_scope -- --ignored --nocapture
//! ```
//!
//! A second run is a **state, not a failure**: the objective's head is looked for on the board
//! first, with the kind marker stripped the way `Board::from_chain` strips it, and the run
//! reports *already on the record* and returns.

use voxel_model::chain::{EventKind, Namespace, Season};
use voxel_model::envelope::{Control, EventType, Kind, RecordState};
use voxel_model::ticket::Board;
use voxel_package::compiler::UnlinkedRecord;
use voxel_package::features::workspace_root;
use voxel_package::filing;

/// The work, in the board's own grammar: `[INT]` is the marker `split_kind` reads.
const OBJECTIVE: &str = "[INT] read the whole ala-cities sim world and port it into the civilian \
                         process, unreduced: sim/mod.rs 5563 lines, citizen.rs, task.rs, rng.rs, \
                         road.rs and terrain.rs; BuildingKind, MaterialLineage, Zone, World, \
                         SIM_HZ, Terrain and CitizenState are part of the process and are read \
                         whole before any port row, never stubbed and never mapped ahead of the read";

/// **The `because` is not a comment.** A ticket whose objective is a bare imperative cannot be
/// checked by a later reader, so the ruling that declares it travels with it.
const BECAUSE: &str = "event-0136 (strike 10): the agent's visible chain of thought proposed \
                       resolving the sim types for the laya-seam milestone instead of reading them \
                       whole first — scope reduction, which event-0127 already ruled out. The user: \
                       ticket to be logged, an infraction has been detected, reducing scope as \
                       according to old projects decision ledgers. The sim world is the towns, \
                       buildings, zones and terrain the civilians live and work in, and the user \
                       named it useful to the laya observer and the sound engine. Related: \
                       event-0133 (the world half is read and mapped, never copied), BLD-0221.";

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
fn file_the_civilian_sim_scope_ruling() {
    let root = workspace_root(&std::env::current_dir().expect("a current directory"))
        .expect("the workspace root");
    assert!(
        filing::is_filed(&root),
        "there is no chain to extend — `grow` extends one and refuses to create one"
    );

    let before = std::fs::read(filing::chain_path(&root)).expect("the record reads");
    let board_before = Board::from_chain(&filing::read(&root));

    // **Already filed is a state, not a failure** — compared on the board's own holdings,
    // because the marker is stripped before the board ever sees the objective.
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
    // this is a property of the code rather than a check — and it is asserted because a property
    // nobody checks is a property that can change.
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
