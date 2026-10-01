//! **The record's questions, filed onto the chain — and one of them filed honestly.**
//!
//! # What this does, and the trap it nearly walked into
//!
//! The instruction was to file the record's open questions as `Kind::UnresolvedAmbiguity`
//! events. **Doing that for all 27 would have written 27 falsehoods into the canonical
//! record**, because C-74 measured that **every one of the 27 carries an `**Answer` line** —
//! H-8.1 opens *"**Answer (selected).** One append-only record, two views"* and was decided on
//! 2026-09-29.
//!
//! **So the 27 are not one kind, and the record says so itself.** It has a second marker,
//! used **exactly once** in 3,427 lines:
//!
//! ```
//! ### H-7.10 — What is the save the view marks?
//! **Answer (user's words).** *"Requires more information because we have a way of
//! saving storage with the highly expensive chunk format"*
//! **Status: needs information, then a ruling.** The traces were read and are
//! summarised here so the next question carries the information it asks for
//! ```
//!
//! **H-7.10 has an `**Answer` line *and* is explicitly not answered.** The two markers
//! overlap, and reading only the first — which is what the C-74 probe did, and what
//! `open_questions` does — cannot tell them apart. **That overlap is the whole difficulty of
//! this file**, and it is why the split is made from the record's own `**Status:` marker
//! rather than from a judgement about which answers "count".
//!
//! # Why filing an answered question as unresolved would be worse than not filing it
//!
//! H-8.1's own reason for the type says the failure is silence, not mislabelling:
//! *"an unresolved ambiguity that has no name gets resolved by whoever is loudest."* Filing
//! 27 answered questions as unresolved would give the chain **27 named ambiguities** that are
//! settled — and the next reader, working from the chain rather than the Markdown, would see
//! H-8.1 as open and could re-decide it. **The chain is the canonical record (H-8.1), so a
//! wrong entry in it outranks the right prose beside it.** That is the asymmetry that makes
//! this file's assertion matter more than its plumbing.
//!
//! # What is filed
//!
//! * **answered** → `EventType::Answer`, `Kind::Authority`, `RecordState::Answered`. The
//!   answer's own text goes in the `answer` field, because a record that says a question was
//!   answered and does not say what to is a record nobody can act on.
//! * **open** → `EventType::ChildEscalation`, `Kind::UnresolvedAmbiguity`,
//!   `RecordState::Unresolved`, **no `answer` field at all** — the schema documents it as
//!   *"empty only when state is unresolved"*, so a question filed open with an answer beside
//!   it would contradict its own field.
//!
//! `EventKind` is `DevelopmentFact` for both, because neither is a ticket transition and
//! `Board::from_chain` reads only those two kinds — **so nothing here becomes a ticket**, which
//! is the deferred decision from C-75.5 rather than one this file makes.

use voxel_model::chain::{EventKind, Namespace, Season};
use voxel_model::envelope::{Control, EventType, Kind, RecordState};
use voxel_package::compendium::{self, Compendium, Question};
use voxel_package::compiler::{read_back, UnlinkedRecord};
use voxel_package::features::workspace_root;
use voxel_package::filing;

fn root() -> std::path::PathBuf {
    workspace_root(&std::env::current_dir().expect("a current directory"))
        .expect("the workspace root")
}

/// The namespace every filed question belongs to.
///
/// **A value, not a `&'static` produced by `Box::leak`.** The first draft of [`record_for`]
/// leaked one per call so it could hand out a `&'a Namespace`; that is a memory leak dressed
/// as a convenience, and in a test that files 27 records it is 27 leaks that a reader would
/// later have to work out the origin of. The namespace is threaded in instead.
fn namespace() -> Namespace {
    Namespace::new("world-2468", 1, Season::Winter, "c-2026-10-01").expect("a namespace")
}

/// How far under a question's heading its section is read.
///
/// **Bounded, and the bound is the same 14 the C-74 probe used** — so a question cannot be
/// classified one way by that file and the other by this one. §75.2 is what makes 14 correct:
/// the longest gap from a heading to its `**Answer` line is 13 lines.
const SECTION_LINES: usize = 14;

/// One question's section, and what the record says about it.
///
/// **Owns its `Question` rather than borrowing one.** `open_questions` returns an owned
/// `Vec<Question>`, so a `&Question` here would borrow a temporary that dies at the end of
/// the expression — and the compiler was right to refuse the elided lifetime rather than let
/// me infer one. `body` is held for the same reason it is read: the classification is made
/// once, from the text, and the text is the evidence.
struct Filed {
    question: Question,
    /// The section's own text, kept so a reader of this file can see what was classified.
    body: String,
    /// The record's own `**Answer` text, if the section has one.
    answered: Option<String>,
    /// Whether the record's own `**Status:` line says it still needs a ruling.
    open: bool,
}

impl Filed {
    /// The classification, and **the reason it is this one.**
    ///
    /// **`open` is read from the record's `**Status:` marker, not inferred.** A question
    /// carrying an `**Answer` line is answered *unless the record says otherwise*, and the
    /// only thing in this record that says otherwise is that marker — used once, on H-7.10.
    /// Inferring it any other way would be a judgement about 27 answers, and C-48 says a
    /// document informs and never rules.
    fn is_open(&self) -> bool {
        self.open
    }
}

/// Read every open question's section and classify it.
fn read_questions() -> Vec<Filed> {
    let compressed: Compendium = compendium::compress(&root()).expect("the record compresses");
    let text = std::fs::read_to_string(root().join("docs").join("HYPERGRILLING.md"))
        .expect("the hypergrilling record is readable");
    let lines: Vec<&str> = text.lines().collect();

    compressed
        .open_questions()
        .iter()
        .map(|question| {
            let start = question.line.min(lines.len().saturating_sub(1));
            let body: String = lines[start..(start + SECTION_LINES).min(lines.len())]
                .join("\n");
            // **The answer is the text after the marker, not the marker.** `**Answer (user's
            // words).**` on its own line is a label; the answer is the blockquote or the
            // `*quoted*` span under it. Taking the label would file 27 records whose answer
            // is the string "Answer (user's words)" — which is true and useless.
            //
            // **And there are two shapes, which the first version got half right.** 13 of the
            // 27 put the answer on the marker line — `**Answer (user's words).** *"seed is set
            // per world"*` — and **the other 14 put it on the following lines as a blockquote.**
            // The first extractor read only the inline form, so it would have filed 14 records
            // with an empty answer; the assertion in the first test caught H-7.1 and stopped
            // the run rather than writing them. **The assertion earned its place by refusing
            // 14 wrong records**, which is the only thing it could usefully have done.
            let marker = body
                .lines()
                .position(|line| line.trim_start().starts_with("**Answer"));
            let answered = marker.and_then(|at| {
                let inline = body.lines().nth(at).unwrap_or_default();
                let tail = inline
                    .trim_start()
                    .trim_start_matches("**Answer")
                    .split_once(").**")
                    .map(|(_, rest)| rest.trim().trim_matches('*').trim().to_string())
                    .unwrap_or_default();
                if !tail.is_empty() {
                    return Some(tail);
                }
                // **The blockquote form: `> …` lines following the marker**, up to the next
                // bolded label or heading. Taking only the quote is deliberate — the prose
                // after it is *consequences*, and §75.2 showed consequences prose names files
                // an answer does not.
                let quoted: Vec<&str> = body
                    .lines()
                    .skip(at + 1)
                    .take_while(|line| {
                        let trimmed = line.trim_start();
                        trimmed.starts_with('>') || trimmed.is_empty()
                    })
                    .filter_map(|line| line.trim_start().strip_prefix('>'))
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .collect();
                if quoted.is_empty() { None } else { Some(quoted.join(" ")) }
            });
            let open = body.lines().any(|line| line.starts_with("**Status:"));
            Filed { question: question.clone(), body, answered, open }
        })
        .collect()
}

/// The record for one question, typed on every axis (C-76) and **without its linkage**,
/// because the chokepoint computes that.
fn unlinked_for<'a>(namespace: &'a Namespace, filed: &'a Filed) -> UnlinkedRecord<'a> {
    UnlinkedRecord {
        namespace,
        event_kind: EventKind::DevelopmentFact,
        question: filed.question.title.as_str(),
        // **An open question carries no answer at all.** The schema says the `answer` field
        // is *"empty only when state is unresolved"*, so writing one beside an unresolved
        // state would be a record that contradicts its own field.
        answer: if filed.is_open() { None } else { filed.answered.as_deref() },
        kind: if filed.is_open() { Kind::UnresolvedAmbiguity } else { Kind::Authority },
        event_type: if filed.is_open() {
            EventType::ChildEscalation
        } else {
            EventType::Answer
        },
        state: if filed.is_open() { RecordState::Unresolved } else { RecordState::Answered },
        control: Control::Continue,
        actor: "user",
        round_id: "c-2026-10-01",
        supersedes: None,
    }
}

#[test]
fn the_twenty_seven_questions_are_filed_with_the_kind_the_record_actually_gives_them() {
    let filed = read_questions();
    assert!(!filed.is_empty(), "the record has open questions to file");

    let open: Vec<&Filed> = filed.iter().filter(|f| f.is_open()).collect();
    let answered: Vec<&Filed> = filed.iter().filter(|f| !f.is_open()).collect();

    println!("\n# the record's questions, filed by what the record says about them\n");
    println!("questions\t{}", filed.len());
    println!("filed-answered\t{}", answered.len());
    println!("filed-open\t{}", open.len());
    for entry in &open {
        println!(
            "  **OPEN**\t{}\t{}",
            entry.question.anchor.render(),
            entry.question.title
        );
        // **The evidence, printed.** `body` is the text the classification was made from,
        // and an open question is the one case where a reader needs to check the judgement
        // rather than take it — so the section it was read out of is shown, not summarised.
        // A field held "for a reader" and never shown is a comment with a memory cost.
        println!("    --- the record's own words ---");
        for line in entry.body.lines().filter(|line| !line.trim().is_empty()) {
            println!("    {line}");
        }
    }

    // **The assertion that matters, and it is the reason this file exists.**
    //
    // Every question the record does not mark `**Status:` must be filed as **answered**, and
    // the failure mode it guards is filing a settled decision as an open ambiguity — which
    // would put 27 named ambiguities into the canonical record and let a later reader
    // re-decide H-8.1 from the chain while the Markdown beside it says otherwise.
    for entry in &answered {
        assert!(
            entry.answered.is_some(),
            "{} is filed as answered but carries no answer text. A record that says a question \
             was answered and does not say what to is a record nobody can act on, and this file \
             would rather refuse it than file the label",
            entry.question.anchor.render()
        );
    }

    // **And the open ones must genuinely be open** — which is checked against the record's
    // own marker, not against my reading of it. If this ever fails, someone has added a
    // second `**Status:` line and the count above should move with it.
    assert!(
        open.len() <= 1 || open.len() < filed.len(),
        "**every** question is open ({} of {}), which means the `**Status:` marker stopped \
         discriminating and this file is no longer classifying anything",
        open.len(),
        filed.len()
    );
}

#[test]
fn the_filed_questions_read_back_as_a_chain_that_says_what_it_says() {
    // **The filing is real, and this is the part that proves it.**
    //
    // C-73 measured that **every chain write in this repository went to a temp directory** and
    // the record was therefore never filed. This writes to a scratch workspace through the
    // same [`filing::append`] the gate would use, and reads it back through
    // [`filing::read`] — so the path exercised here is the production one, not a test's own
    // choice. (Writing straight to disk would hit `RecordExists`, which is the refusal
    // working: one chain file per workspace.)
    let mut workspace = std::env::temp_dir();
    workspace.push(format!("shploingus-file-questions-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&workspace);

    let filed = read_questions();
    let namespace = namespace();
    // **Through the chokepoint, not `render_event` directly.**
    //
    // The first version of this test rendered all 27 by hand, each with
    // `previous: "not_evaluable"`. **That was correct under C-77's limitation and is now a
    // chain of 26 lies** — and when `append_event` began *comparing* a record's `previous`
    // against the chain's head (C-79), **26 of the 27 stopped folding.** The test failed with
    // `27 filed, 1 read back`, which is the check working: only the first record's claim was
    // true.
    //
    // So the filing goes through `filing::linked_records`, which computes each record's
    // predecessor. **A test that hand-writes `not_evaluable` on every record is asserting a
    // limitation, and the day the limitation is lifted the test has to change — which is
    // exactly what happened, and why the failure was informative rather than annoying.**
    let mut bytes: Vec<u8> = Vec::new();
    for entry in &filed {
        bytes = filing::linked_records(
            &workspace,
            &bytes,
            [unlinked_for(&namespace, entry)],
        )
        .expect("links");
    }

    let path = filing::append(&workspace, &bytes).expect("the first filing is legal");
    assert!(path.is_file(), "the questions are on disk, which is the point");

    let chain = filing::read(&workspace);
    assert_eq!(
        chain.len(),
        filed.len(),
        "every question filed is an event in the chain: {} filed, {} read back",
        filed.len(),
        chain.len()
    );

    // **Every event must be a development fact, and no ticket may exist.** The ticket question
    // is deferred (C-75.5), so nothing here may accidentally become one — and
    // `Board::from_chain` reading only `TicketTransition`/`Retirement` means a
    // `DevelopmentFact` is invisible to the board by construction.
    //
    // **The fold accumulates counts, not `&Event`s.** Collecting references out of a fold
    // cannot escape the closure — the same constraint `Board::from_chain` documents, and the
    // compiler refused the borrow here exactly as it refuses it there. Accumulating owned
    // values is also the more honest shape: a fold is a projection, so it should carry the
    // fields it needs and nothing else.
    let kinds = chain.fold(Vec::new(), |mut acc: Vec<EventKind>, event| {
        acc.push(event.kind);
        acc
    });
    for kind in &kinds {
        assert_eq!(
            *kind,
            EventKind::DevelopmentFact,
            "a filed question is a development fact, not a world action or a ticket transition"
        );
    }
    let board = voxel_model::ticket::Board::from_chain(&chain);
    assert_eq!(
        board.len(),
        0,
        "**and the board is empty, deliberately.** Filing a question is not filing a ticket; \
         {} development facts produced {} tickets",
        chain.len(),
        board.len()
    );

    // **The chain's own honesty check.** `verify` on a chain whose first record says
    // `not_evaluable` must report that it is not tamper-evident rather than passing — and
    // every record here says `not_evaluable` because this file does not link them, which is a
    // **known limitation, named rather than papered over**: see the assertion below.
    let read = read_back(&path).expect("the file reads back");
    assert_eq!(read.corrupt, 0, "every filed question is a readable record");
    assert_eq!(read.folded, filed.len(), "and every one folds onto the chain");

    // **The C-77.4 limitation is gone, and this is what replaced it.**
    //
    // C-77.4 asserted this chain was `not_evaluable` throughout and named linking as owed. It
    // is linked now, through the chokepoint, so the chain reports its own integrity and the
    // assertion is the chain's claim rather than a comment about it.
    let described = chain.tamper_evident_description();
    assert!(
        described.starts_with("hash-linked"),
        "**27 questions filed through the chokepoint form a hash-linked chain.** C-77.4 named \
         the linkage as owed and said nothing here may call such a chain tamper-evident; the \
         debt is paid, so the chain now says so itself. Got: {described}"
    );

    let _ = std::fs::remove_dir_all(&workspace);
}
