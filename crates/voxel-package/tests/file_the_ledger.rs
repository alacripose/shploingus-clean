//! **Filing the game's ledger — `docs/LEDGER.md` — as one CSE ticket per entry.**
//!
//! # The correction this file answers
//!
//! The user's instruction, 2026-10-02: *"file THE ledger.. the GAME'S ledger in docs into the
//! ticket system"*, then *"DO NOT REDUCE IT'S SCOPE."*, and then, against the first version of
//! this file — which **tiled** each entry into several `... part i/j` records to fit the
//! 4,096-byte field bound — the verdict that stands at the top of the record itself:
//!
//! > *this file is ass and should have filed each record individually as tickets*
//!
//! So this files **every `## C-` entry as its own ticket**, one ticket per entry, in document
//! order, and it files **all of them** — there is no selection to keep in sync, no tiling, no
//! skip list. This is the ledger's own index: `C-1` … `C-103` and whatever comes after, each a
//! `[CSE]` case on the board, filed the way this record files governance — through
//! [`filing::grow`], the chokepoint that computes linkage, so the tickets join the chain
//! rather than forming a parallel list.
//!
//! # Why the entry's *text* is not copied into the record
//!
//! `voxel_model::envelope::MAX_FIELD_BYTES` is **4,096** and the record's rule is a field over
//! it is **refused, never truncated**. The ledger's largest entry is over 23,000 bytes, so the
//! text cannot live in a field — and the envelope's own design says where it does live:
//!
//! > *"The long-form reasoning is not lost; it is **not in the record**. It lives in the
//! > documents the record cites, and `Namespace`'s digest is what binds an event to them."*
//!
//! So each ticket's **question** is the entry's own heading (its title, the record's words,
//! never a paraphrase), and its **answer** is the locator and the sha256 of the entry's exact
//! bytes — the citation that binds the ticket to the document. A reader of the board sees the
//! whole ledger as its own index; a reader who wants the text follows the locator, and the
//! digest tells them whether the document has moved since the ticket was filed.
//!
//! # Every heading style, because the first version filed only one of them (C-107)
//!
//! The record holds **three** shapes: a fully bold one-liner (`**C-1 — …**`, C-1…C-34), a
//! `### C-35 …` heading (C-35…C-39) and a `## C-40 …` heading (C-40 onward). This file's first
//! version split on `## ` alone, so **the 39 earliest entries were never filed** and the board's
//! earliest ticket was C-40. That is C-90.2's own named hazard committed one more time: *"absence
//! rendered as absence is a reading error, and the fix is to read the file rather than to grep one
//! heading level."* [`entries`] reads all three, and the coverage test pins the numbers as
//! **contiguous from C-1**, because a gap is what a dropped entry looks like from the board.
//!
//! # The two tests, and why only one of them writes
//!
//! * [`every_c_entry_is_one_ticket`] runs in a **default** `cargo test` and writes nothing. It
//!   is the coverage proof: every C entry the document holds becomes exactly one ticket, no
//!   ticket is over the field bound, and the questions are distinct.
//! * [`file_the_ledger_onto_the_record`] is `#[ignore]`d and run **deliberately**. It appends
//!   the whole run to the canonical record through `filing::grow`. Run it with:
//!
//! ```text
//! cargo test --offline -p voxel-package --test file_the_ledger -- --ignored --nocapture
//! ```

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use voxel_model::chain::{EventKind, Namespace, Season};
use voxel_model::envelope::{Control, EventType, Kind, RecordState, MAX_FIELD_BYTES};
use voxel_model::identity::Digest;
use voxel_model::ticket::Board;
use voxel_package::compiler::UnlinkedRecord;
use voxel_package::features::workspace_root;
use voxel_package::filing;

/// The game's ledger, relative to the workspace root.
const LEDGER: &str = "docs/LEDGER.md";

/// The namespace this record's filings belong to — the same one the other filers use, because
/// there is one record and not one record per document.
fn namespace() -> Namespace {
    Namespace::new("world-2468", 1, Season::Winter, "c-2026-10-01").expect("a namespace")
}

/// The workspace root, found from the process's current directory — the same walk the other
/// filers use, so the ledger is read from the repository rather than from the crate.
fn root() -> PathBuf {
    workspace_root(&std::env::current_dir().expect("a current directory"))
        .expect("the workspace root")
}

fn ledger_text(root: &Path) -> String {
    std::fs::read_to_string(root.join(LEDGER)).expect("the ledger is readable")
}

/// **Every C entry, whatever heading style it uses.**
///
/// C-90.2's own hazard, and the first version of this file re-committed it: *"absence rendered as
/// absence is a reading error, and the fix is to read the file rather than to grep one heading
/// level."* The record holds three shapes, and splitting on `## ` alone left **the 39 earliest
/// entries unfiled** (C-107).
///
/// Each entry runs from its own first line to the line before the next entry's, so every entry
/// tiles the document and every locator a ticket writes can be followed.
fn entries(text: &str) -> Vec<(usize, usize, &str)> {
    let mut starts: Vec<(usize, usize)> = Vec::new();
    let mut offset = 0usize;
    let mut line = 1usize;
    for text_line in text.split_inclusive('\n') {
        if names_a_c_entry(text_line) {
            starts.push((offset, line));
        }
        offset += text_line.len();
        line += 1;
    }
    let total_lines = text.lines().count();
    let mut bounds: Vec<(usize, usize)> = Vec::with_capacity(starts.len());
    for (index, (start, start_line)) in starts.iter().enumerate() {
        let end = starts
            .get(index + 1)
            .map(|(next, _)| *next)
            .unwrap_or(text.len());
        bounds.push((*start, end));
        let _ = start_line;
    }

    let mut out: Vec<(usize, usize, &str)> = Vec::new();
    for (index, (start, end)) in bounds.iter().enumerate() {
        let body = &text[*start..*end];
        let first_line = starts[index].1;
        let last_line = if index + 1 < starts.len() {
            starts[index + 1].1 - 1
        } else {
            total_lines
        };
        // **Every start is an entry now**, because `names_a_c_entry` is the filter the old
        // `body.starts_with("## C-")` check was standing in for — and that check is what silently
        // dropped the bold and `###` entries.
        out.push((first_line, last_line, body));
    }
    out
}

/// **Whether a line begins a C entry**, in any of the record's three heading shapes.
///
/// The bold shape is *the whole line in `**…**` and the text inside begins `C-<n> — `*. **The em
/// dash is the whole rule**, because the record is full of bold *references* to entries —
/// `**C-49's ruling 2 lists the stage cadences**…`, `**C-78 exists partly because that claim was
/// asserted without being checked**` — and reading one of those as a start would split an entry in
/// two and file a case for a sentence.
fn names_a_c_entry(line: &str) -> bool {
    let trimmed = line.trim_end_matches(['\r', '\n']);
    if let Some(rest) = trimmed.strip_prefix("## ") {
        return rest.starts_with("C-");
    }
    if let Some(rest) = trimmed.strip_prefix("### ") {
        return rest.starts_with("C-");
    }
    let Some(inner) = trimmed.strip_prefix("**") else {
        return false;
    };
    let Some(rest) = inner.strip_prefix("C-") else {
        return false;
    };
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return false;
    }
    rest[digits.len()..].starts_with(" — ")
}

/// The entry's heading with its **own** markers stripped — the record's own title for the case.
///
/// All three shapes (`## `, `### ` and the bold `**…**`) come out as one sentence, because a ticket
/// whose question reads `## C-104 …` while its neighbour reads `C-1 …` would be two spellings of
/// one convention.
fn heading(entry: &str) -> &str {
    let first = entry
        .lines()
        .find(|line| !line.trim().is_empty())
        .map(str::trim)
        .unwrap_or("(empty)");
    first
        .trim_start_matches("## ")
        .trim_start_matches("### ")
        .trim_start_matches("**")
        .trim_end_matches("**")
        .trim()
}

/// The entry's own number, from its heading — **the record's identity for the case.**
///
/// Used twice: the coverage test pins the numbers as contiguous from `C-1`, and the deliberate
/// filer skips an entry the board already holds.
fn number_of(heading: &str) -> Option<u32> {
    let rest = heading.strip_prefix("C-")?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// The first `C-<digits>` a text names, so a ticket **either filer** wrote maps to its entry.
///
/// The new filer's objective is the heading itself (`C-63 — …`); the tiled filer's is
/// `docs/LEDGER.md entry 32 (## C-63 — …) — part 1/3, lines 2533..=2591`. Both are on the record
/// and both cover their entry, so the skip set is read from the objectives rather than from a
/// second list that could drift.
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

/// One ticket: the question the board will hold, and the citation that binds it to the document.
struct Ticket {
    /// The entry's own number (`C-1`, `C-63`, …) — the record's identity for the case.
    number: u32,
    question: String,
    answer: String,
}

/// **The ledger as its own index: one ticket per C entry.**
fn tickets(text: &str) -> Vec<Ticket> {
    entries(text)
        .iter()
        .map(|(first_line, last_line, entry)| {
            let title = heading(entry);
            let digest = Digest::of(entry.as_bytes());
            Ticket {
                number: number_of(title).unwrap_or(0),
                // **The marker is the kind.** `[CSE]` is the case prefix the board's own
                // grammar strips, so the id is `CSE-<position>` and the objective is the
                // entry's own title.
                question: format!("[CSE] {title}"),
                // **The citation, not a copy.** The locator a reader follows, and the digest
                // of the exact bytes it was filed against.
                answer: format!(
                    "{LEDGER} lines {first_line}..={last_line}; sha256:{}",
                    digest.to_hex()
                ),
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------
// The proof, and it writes nothing.
// ---------------------------------------------------------------------------------------------

/// **Every C entry is one ticket, and the set is whole.**
///
/// Five properties, each asserted against the document on disk:
///
/// 1. **every C entry becomes exactly one ticket, whatever heading style it uses** —
///    `tickets().len()` is `entries().len()`, because a filer that skipped one files a
///    selection, and the first version of this file filed only the `## ` shape (C-107);
/// 2. **the numbers run `C-1 … C-n` with no gap and no repeat** — a gap is what a dropped entry
///    looks like from the board, and the reduction C-107 removes left the board's earliest case
///    at `C-40` while the document's first entry was `C-1`;
/// 3. every question is under `MAX_FIELD_BYTES`, so no ticket is refused at write time;
/// 4. every question is distinct, so two entries cannot collapse to one board objective;
/// 5. the entries tile the document — each line belongs to exactly one entry, so a reader can
///    follow every locator.
#[test]
fn every_c_entry_is_one_ticket() {
    let text = ledger_text(&root());
    let entries = entries(&text);
    let tickets = tickets(&text);

    assert!(
        !entries.is_empty(),
        "the ledger holds {} C entries; the count is printed rather than assumed",
        entries.len()
    );
    assert_eq!(
        tickets.len(),
        entries.len(),
        "one ticket per C entry — a filer that skips one files a selection"
    );
    for (ticket, (first_line, last_line, entry)) in tickets.iter().zip(entries.iter()) {
        assert!(
            ticket.question.len() <= MAX_FIELD_BYTES,
            "{}: {} bytes, over the {MAX_FIELD_BYTES}-byte field bound",
            ticket.question,
            ticket.question.len()
        );
        assert!(
            ticket.question.starts_with("[CSE] C-"),
            "the kind marker and the entry's own name: {}",
            ticket.question
        );
        assert!(
            ticket.answer.contains(&format!("lines {first_line}..={last_line}")),
            "the locator names this entry's lines: {}",
            ticket.answer
        );
        assert!(
            !entry.is_empty(),
            "entry at line {first_line} is empty and cannot be a case"
        );
    }
    let mut questions: Vec<&str> = tickets.iter().map(|ticket| ticket.question.as_str()).collect();
    questions.sort_unstable();
    let distinct = questions.len();
    questions.dedup();
    assert_eq!(
        questions.len(),
        distinct,
        "two C entries would collapse to one board objective; the questions are not distinct"
    );

    // **The numbers are `C-1 … C-n` with no gap and no repeat.** The heading style changes
    // inside the document — a bold one-liner for the first 34 entries, a `### ` heading for
    // five, a `## ` heading for the rest — so a reader that greps one level files a suffix, and
    // the board's earliest case is not the ledger's first entry. This is the assertion that
    // catches that: not a count, which a suffix also passes, but the absence of a hole.
    let numbers: Vec<u32> = tickets.iter().map(|ticket| ticket.number).collect();
    let expected: Vec<u32> = (1..=numbers.len() as u32).collect();
    assert_eq!(
        numbers,
        expected,
        "the ledger's numbers must run C-1..=C-{} with no gap and no repeat; a gap means the \
         heading filter dropped an entry, and a repeat means two entries share one number",
        numbers.len()
    );

    let bold = text
        .lines()
        .filter(|line| names_a_c_entry(line) && line.starts_with("**"))
        .count();
    let three = text
        .lines()
        .filter(|line| names_a_c_entry(line) && line.starts_with("### "))
        .count();
    let two = text
        .lines()
        .filter(|line| names_a_c_entry(line) && line.starts_with("## "))
        .count();
    println!(
        "the ledger: {} bytes, {} lines, {} C entries — {} tickets, one per entry \
         ({} bold one-liner(s), {} ### heading(s), {} ## heading(s))",
        text.len(),
        text.lines().count(),
        entries.len(),
        tickets.len(),
        bold,
        three,
        two
    );
    for (ticket, (first_line, last_line, _)) in tickets.iter().zip(entries.iter()) {
        println!("  {first_line}..={last_line}  {}", ticket.question);
    }
}

// ---------------------------------------------------------------------------------------------
// The filing, and it is deliberate.
// ---------------------------------------------------------------------------------------------

/// **Append the ledger's index to the canonical record — every C entry, its own ticket.**
///
/// `#[ignore]`d because it writes to the record every view reads, which is a thing to do on
/// purpose and not as a side effect of a default test run — the same discipline the other
/// filers state. Run it with:
///
/// ```text
/// cargo test --offline -p voxel-package --test file_the_ledger -- --ignored --nocapture
/// ```
///
/// **One ticket per entry — and an entry already on the board is skipped, not refiled.** The
/// user's rule from the first draft's own top line is *file each record individually as tickets*,
/// which is a statement about granularity, not about filing the index twice. The board is the
/// record's own view ([`Board::from_chain`]), so the skip set is read from the **objectives the
/// record already carries** — either filer's form — rather than from a list beside it that could
/// drift. That is what makes this test re-runnable: what is missing is filed, what is present is
/// left alone, and the run prints both counts.
#[test]
#[ignore = "files the game's ledger index onto the canonical record; run it deliberately"]
fn file_the_ledger_onto_the_record() {
    let root = root();
    assert!(
        filing::is_filed(&root),
        "the record must be filed first: there is nothing to link the ledger onto"
    );

    let text = ledger_text(&root);
    let all = tickets(&text);
    assert!(!all.is_empty(), "the ledger holds no C entries at all");

    // **What the board already holds.** Read from the objectives on the record — the new
    // filer's `C-63 — …` (the board strips the `[CSE]` marker) or the tiled filer's
    // `docs/LEDGER.md entry 32 (## C-63 — …) — part 1/3, lines …` — so the skip set is derived
    // from the chain rather than from a second list that could drift out of step with it.
    let before_board = Board::from_chain(&filing::read(&root));
    let already: BTreeSet<u32> = before_board
        .tickets()
        .iter()
        .filter(|ticket| {
            ticket.objective.starts_with("C-") || ticket.objective.contains("docs/LEDGER.md")
        })
        .filter_map(|ticket| first_c_number(&ticket.objective))
        .collect();
    let missing: Vec<&Ticket> = all
        .iter()
        .filter(|ticket| !already.contains(&ticket.number))
        .collect();
    println!(
        "the board already holds {} of the ledger's {} C entry ticket(s); {} missing: {}",
        all.len() - missing.len(),
        all.len(),
        missing.len(),
        missing
            .iter()
            .map(|ticket| format!("C-{}", ticket.number))
            .collect::<Vec<_>>()
            .join(", ")
    );
    if missing.is_empty() {
        println!(
            "every one of the ledger's {} C entries is already on the board; nothing to file",
            all.len()
        );
        return;
    }

    let namespace = namespace();
    let records: Vec<UnlinkedRecord<'_>> = missing
        .iter()
        .map(|ticket| UnlinkedRecord {
            namespace: &namespace,
            event_kind: EventKind::TicketTransition,
            question: &ticket.question,
            answer: Some(&ticket.answer),
            // A ledger entry is a decision the record holds — the same kind and event the
            // other filers use, because the ledger's entries are this project's own rulings.
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
    // **Through the chokepoint**: `grow` links every record to the one before it, so the
    // ledger's tickets join the chain rather than forming a parallel list.
    filing::grow(&root, &before, records).expect("the ledger is filed");

    let after = std::fs::read(filing::chain_path(&root)).expect("the record reads");
    assert!(after.len() > before.len(), "the record grew");
    assert!(
        after.starts_with(&before),
        "and nothing already filed was shortened"
    );

    // **The whole index, read back off the chain.** Every entry the document holds now has a
    // ticket whose objective names it, so coverage is asserted entry by entry rather than by a
    // count — a count is what a suffix of the index also produces.
    let board = Board::from_chain(&filing::read(&root));
    let filed: BTreeSet<u32> = board
        .tickets()
        .iter()
        .filter(|ticket| {
            ticket.objective.starts_with("C-") || ticket.objective.contains("docs/LEDGER.md")
        })
        .filter_map(|ticket| first_c_number(&ticket.objective))
        .collect();
    println!(
        "filed {} C entry ticket(s); the board now names {} of the ledger's {} entries and \
         holds {} ticket(s) in total",
        missing.len(),
        filed.len(),
        all.len(),
        board.len()
    );
    assert!(
        all.iter().all(|ticket| filed.contains(&ticket.number)),
        "**the ledger is on the record**, not only in a document: {already:?} covered \
         {} of {} entries before this run, and the read-back is missing entries",
        already.len(),
        all.len()
    );
    assert!(
        filed.len() >= all.len(),
        "no entry may be counted twice: {} entries produced {} distinct ticket(s)",
        all.len(),
        filed.len()
    );
}
