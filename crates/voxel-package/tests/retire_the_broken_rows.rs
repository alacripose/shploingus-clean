//! **Retiring the old tiler's broken fragments — and the event shape that makes a retirement
//! writable at all.**
//!
//! # The gap this closes (F-8), measured before anything was written
//!
//! C-104.7 named the repair for the **244 broken rows**: *"an append-only record cannot retract
//! them. The repair is a regrade or a retirement through the ticket system — the rows stay as
//! evidence of what that filer wrote."* But the ticket system could not file a retirement:
//! `EventKind::Retirement` had **no writer anywhere**, and `Board::from_chain` dropped a
//! retirement event's contents — it flipped the status to `Complete` and left `retirement:
//! None`, which the fold's own comment calls *closed-but-unretired*. A pass today would have
//! produced 244 status flips with no [`Retirement`] behind them, the exact defect
//! `RefusalKind::UnretiredTicket` exists to refuse.
//!
//! So the first test here is the event shape end to end, on a scratch chain: a retirement filed
//! as a chain event carries its own record, and the fold attaches it.
//!
//! # The pass, and what it refuses to do
//!
//! The deliberate second test selects the broken rows with the reader's own grade — a row whose
//! answer names a document and whose digest **cannot be verified** (`holds != Some(true)`, the
//! 244 tiled fragments that "name line ranges and carry no digest"). For each, it demands a
//! **verified successor**: a later filing of the same ledger entry whose cited sha256 still
//! matches the document. Only then is `Superseded` an honest token. A fragment with no verified
//! successor is **reported and left alone**, not rubber-stamped.
//!
//! The **147 self-report rows are out of scope by ruling**: the record says they "need citations
//! to reach `B`", which is content work on each row, not a retirement.
//!
//! ```text
//! cargo test --offline -p voxel-package --test retire_the_broken_rows -- --ignored --nocapture
//! ```

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use voxel_model::axis::AxisKind;
use voxel_model::chain::{EventKind, Namespace, Season};
use voxel_model::envelope::{Control, EventType, Kind, RecordState};
use voxel_model::ticket::{
    retirement_section, Board, Grounding, ReadBack, Retirement, RetirementReason, Verification,
};
use voxel_package::compiler::UnlinkedRecord;
use voxel_package::features::workspace_root;
use voxel_package::filing;

/// The namespace this record's filings belong to — the same one the other filers use, because
/// there is one record and not one record per document.
fn namespace() -> Namespace {
    Namespace::new("world-2468", 1, Season::Winter, "c-2026-10-02").expect("a namespace")
}

/// The workspace root, found from the process's current directory — the same walk the other
/// filers use.
fn root() -> PathBuf {
    workspace_root(&std::env::current_dir().expect("a current directory"))
        .expect("the workspace root")
}

/// A directory the test owns and removes, so a failed run cannot leave a chain behind.
fn scratch(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("shploingus-retire-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    path
}

/// The first `C-<digits>` a text names — the old tiler's objective is
/// `docs/LEDGER.md entry 32 (## C-63 — …) — part 1/3, lines 2533..=2591`, and the new filer's
/// is the heading itself. Same rule as `file_the_ledger.rs`, so the two filers map a ticket to
/// its entry the same way.
fn first_c_number(text: &str) -> Option<u32> {
    let bytes = text.as_bytes();
    for index in 0..bytes.len().saturating_sub(2) {
        if bytes[index] == b'C' && bytes[index + 1] == b'-' && bytes[index + 2].is_ascii_digit() {
            let digits: String = text[index + 2..]
                .chars()
                .take_while(char::is_ascii_digit)
                .collect();
            if let Ok(number) = digits.parse() {
                return Some(number);
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------------------------
// The event shape, proven on a scratch chain.
// ---------------------------------------------------------------------------------------------

/// **A retirement filed on the chain attaches its own record to the ticket.**
///
/// This is the test F-8 did not have: before the section parser, this chain read back as a
/// ticket flipped to `Complete` with `retirement: None` — a status flip, which is not a
/// retirement. Now the contents travel in the event's own section and the fold runs
/// `ticket::retire` on them, so the status and the record are the same act on read-back too.
#[test]
fn a_retirement_filed_on_the_chain_attaches_its_own_record_to_the_ticket() {
    let workspace = scratch("attach");
    let ns = namespace();

    let first = filing::linked_records(
        &workspace,
        &[],
        [UnlinkedRecord {
            namespace: &ns,
            event_kind: EventKind::TicketTransition,
            question: "[CSE] C-1 — a ticket to retire",
            answer: Some("docs/LEDGER.md lines 1..=2; sha256:00"),
            kind: Kind::Authority,
            event_type: EventType::UserDecision,
            state: RecordState::Answered,
            control: Control::Continue,
            actor: "agent",
            round_id: "test",
            supersedes: None,
        }],
    )
    .expect("the ticket links");
    filing::append(&workspace, &first).expect("the record is filed");

    let board = Board::from_chain(&filing::read(&workspace));
    assert_eq!(board.len(), 1, "one ticket");
    let id = board.tickets()[0].id.clone();

    let retirement = Retirement::new(
        id.clone(),
        RetirementReason::Superseded,
        "a later filing covers this ticket",
        Grounding { inputs_verified: false, note: "nothing of its own to verify".into() },
        Verification {
            acceptance_criteria: Vec::new(),
            checks_run: Vec::new(),
            passed: true,
            regressions_detected: false,
        },
        ReadBack::NotChecked,
    )
    .expect("a coherent retirement");
    let section = retirement_section(&retirement);

    let before = std::fs::read(filing::chain_path(&workspace)).expect("the record reads");
    filing::grow(
        &workspace,
        &before,
        [UnlinkedRecord {
            namespace: &ns,
            event_kind: EventKind::Retirement,
            question: &section,
            answer: None,
            kind: Kind::Authority,
            event_type: EventType::UserDecision,
            state: RecordState::Answered,
            control: Control::Continue,
            actor: "agent",
            round_id: "test",
            supersedes: None,
        }],
    )
    .expect("the retirement files");

    let board = Board::from_chain(&filing::read(&workspace));
    assert!(
        board.malformed().is_empty(),
        "the fold reported the retirement malformed: {}",
        board.malformed().join("; ")
    );
    let ticket = board.tickets().iter().find(|ticket| ticket.id == id).expect("the ticket");
    assert!(ticket.is_closed(), "the retirement closed the ticket");
    let recorded = ticket
        .retirement
        .as_ref()
        .expect("the record travels with the event, not just the status");
    assert_eq!(recorded.reason, RetirementReason::Superseded);
    assert_eq!(recorded.completion_claim, "a later filing covers this ticket");
    assert_eq!(recorded.read_back, ReadBack::NotChecked);
    assert!(
        !ticket.is_validated(),
        "`superseded` is not a validated retirement and must never read as one"
    );
}

// ---------------------------------------------------------------------------------------------
// The pass, and it is deliberate.
// ---------------------------------------------------------------------------------------------

/// **File `Superseded` retirements for the old tiler's broken fragments, on the real record.**
///
/// `#[ignore]`d because it writes to the record every view reads — the same discipline the
/// other filers state. Run it with:
///
/// ```text
/// cargo test --offline -p voxel-package --test retire_the_broken_rows -- --ignored --nocapture
/// ```
///
/// A fragment is retired **only** when a later filing of the same ledger entry has a citation
/// whose sha256 verifies against the document (the reader's own `holds == Some(true)`). A
/// fragment without one is named in the output and left alone, because `Superseded` without a
/// successor is a claim with nothing behind it.
#[test]
#[ignore = "files retirements onto the canonical record; run it deliberately"]
fn retire_the_old_tilers_broken_fragments_on_the_record() {
    let root = root();
    assert!(
        filing::is_filed(&root),
        "the record must be filed first: there is nothing to retire on"
    );

    let board = Board::from_chain(&filing::read(&root));
    let observations = voxel_package::civilians::observe(&root);
    let ladder = observations
        .iter()
        .find(|observation| observation.ladder == AxisKind::TicketEvidence)
        .expect("the ticket-evidence ladder is always reported");
    let rows = &ladder.rows;

    let verified: BTreeSet<&str> = rows
        .iter()
        .filter(|row| row.holds == Some(true))
        .map(|row| row.ticket.as_str())
        .collect();
    let broken: Vec<&voxel_package::civilians::TicketRow> = rows
        .iter()
        .filter(|row| row.document.is_some() && row.holds != Some(true))
        .collect();
    println!(
        "{} row(s): {} verified, {} broken, {} self-report",
        rows.len(),
        verified.len(),
        broken.len(),
        rows.len() - verified.len() - broken.len()
    );

    let mut sections: Vec<(String, String)> = Vec::new();
    let mut already_retired = 0usize;
    let mut unbacked: Vec<String> = Vec::new();
    for row in &broken {
        let Some(ticket) = board
            .tickets()
            .iter()
            .find(|ticket| ticket.id.as_str() == row.ticket)
        else {
            unbacked.push(format!("{}: no ticket on the board", row.ticket));
            continue;
        };
        if ticket.retirement.is_some() {
            already_retired += 1;
            continue;
        }
        let Some(entry) = first_c_number(&ticket.objective) else {
            unbacked.push(format!(
                "{}: the objective names no ledger entry: {:?}",
                row.ticket, ticket.objective
            ));
            continue;
        };
        let successor = board.tickets().iter().find(|candidate| {
            candidate.event_index != ticket.event_index
                && candidate.objective.starts_with(&format!("C-{entry} —"))
                && verified.contains(candidate.id.as_str())
        });
        let Some(successor) = successor else {
            unbacked.push(format!(
                "{}: ledger entry C-{entry} has no verified successor filing",
                row.ticket
            ));
            continue;
        };
        let retirement = Retirement::new(
            ticket.id.clone(),
            RetirementReason::Superseded,
            format!(
                "{} is a tiled fragment of ledger entry C-{entry}; {} files the same entry as \
                 one ticket and its citation's sha256 verifies",
                row.ticket,
                successor.id.as_str()
            ),
            Grounding {
                inputs_verified: true,
                note: format!(
                    "{}'s citation digest was re-verified against the ledger on this pass",
                    successor.id.as_str()
                ),
            },
            Verification {
                acceptance_criteria: Vec::new(),
                checks_run: vec![format!(
                    "{}'s citation digest verified; the fragment names lines and carries no \
                     digest of its own, so there is nothing of its own to check",
                    successor.id.as_str()
                )],
                passed: true,
                regressions_detected: false,
            },
            ReadBack::NotChecked,
        )
        .expect("a coherent supersession");
        sections.push((row.ticket.clone(), retirement_section(&retirement)));
    }

    println!(
        "{} fragment(s) to retire, {} already retired, {} unbacked",
        sections.len(),
        already_retired,
        unbacked.len()
    );
    for reason in &unbacked {
        println!("  unbacked: {reason}");
    }
    if sections.is_empty() {
        println!("nothing to file: every broken fragment is already retired or unbacked");
        return;
    }

    let ns = namespace();
    let records: Vec<UnlinkedRecord<'_>> = sections
        .iter()
        .map(|(_, section)| UnlinkedRecord {
            namespace: &ns,
            event_kind: EventKind::Retirement,
            question: section,
            answer: None,
            kind: Kind::Authority,
            event_type: EventType::UserDecision,
            state: RecordState::Answered,
            control: Control::Continue,
            actor: "agent",
            round_id: "c-2026-10-02",
            supersedes: None,
        })
        .collect();

    let before = std::fs::read(filing::chain_path(&root)).expect("the record reads");
    // **Through the chokepoint**: `grow` links every record to the one before it and refuses to
    // shorten the file, so the retirements join the chain rather than forming a parallel list.
    filing::grow(&root, &before, records).expect("the retirements are filed");
    let after = std::fs::read(filing::chain_path(&root)).expect("the record reads");
    assert!(after.starts_with(&before), "nothing already filed was shortened");

    let board = Board::from_chain(&filing::read(&root));
    let mut attached = 0usize;
    let mut missing: Vec<String> = Vec::new();
    for (ticket_id, _) in &sections {
        match board.tickets().iter().find(|ticket| ticket.id.as_str() == ticket_id) {
            Some(ticket)
                if ticket.is_closed()
                    && ticket.retirement.as_ref().map(|retirement| retirement.reason)
                        == Some(RetirementReason::Superseded) =>
            {
                attached += 1;
            }
            Some(ticket) => missing.push(format!("{ticket_id}: {}", ticket.closing_line())),
            None => missing.push(format!("{ticket_id}: not on the board after the filing")),
        }
    }
    println!(
        "attached {attached} of {} filed retirement(s); {} missing",
        sections.len(),
        missing.len()
    );
    for line in &missing {
        println!("  missing: {line}");
    }
    assert!(missing.is_empty(), "every filed retirement must attach to its ticket");
    assert!(
        board.malformed().is_empty(),
        "the fold reported malformed retirements: {}",
        board.malformed().join("; ")
    );
    let _ = Path::new("");
}
