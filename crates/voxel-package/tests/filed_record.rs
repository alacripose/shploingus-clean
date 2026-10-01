//! **The first record that outlives the process that wrote it.**
//!
//! # What this file is for
//!
//! `first_ticket.rs` files a ticket, reads it back, back-compiles it and judges the document
//! it produces. `round_trip.rs` proves the same round trip six ways. **Both write their chain
//! files to a temporary directory and throw them away** — so the project's own record had
//! never been filed, and every view that reads the record had nothing to read:
//!
//! * the chain was empty, so `Board::from_chain` had no tickets and no window could declare
//!   anything;
//! * `projection::compare` reported **36 unbacked entries** on every build, which is H-8.1's
//!   central claim — *"a disagreement between them is impossible by construction"* —
//!   **measurably false**;
//! * the API document had nothing to describe.
//!
//! This file closes that gap **at the smallest possible scale**: one real event, filed to
//! disk, read back through the same path the gate uses, and the disagreement count measured
//! before and after.
//!
//! # Why it does not seed all 36 entries
//!
//! **Because seeding them would be the wrong move and it is worth saying why.** Back-filling
//! the chain by parsing `docs/LEDGER.md` into events builds the pipeline **backwards**: the
//! Markdown would become the input and the chain a copy of it, which mints a second record
//! that can drift from the first — precisely the failure H-8.1 exists to make impossible. The
//! entries the chain does not yet hold are **real history that predates the chain**, and the
//! honest state is the disagreement count, not a fabricated reconciliation.
//!
//! So the number is **expected to stay positive and in fact to grow by one per filing**, and
//! the test asserts the *direction and the width* rather than a total that would have to be
//! edited every time a real entry was filed. That is the difference between a measurement and
//! a snapshot.
//!
//! # The direction, which is the opposite of what this file was first named for
//!
//! The first draft of this test was called `filing_one_real_record_moves_the_disagreement_
//! count_down_by_exactly_one`. **It does not, and it was measured rather than assumed:** 38
//! unbacked entries before, 39 after. Filing a record the Markdown does not write *adds* a
//! disagreement — the chain now holds `C-1` and the record does not write it, so writing the
//! projection would delete a real event. That is not a bug in the filing path; it is the
//! projection gate working exactly as designed, and **it is the reason the emitter stays
//! unreachable until someone rules how pre-chain history becomes chain history.** A test whose
//! name stated the opposite of what it measured would have been the C-69 defect — a comment
//! describing behaviour the code does not have.

use std::path::PathBuf;

use voxel_model::chain::{self, EventKind, Namespace, Season};
use voxel_model::envelope::{Control, EventType, Kind, RecordState};
use voxel_package::compendium::{self, Compendium};
use voxel_package::compiler::{read_back, render_event, FiledRecord, UnlinkedRecord};
use voxel_package::features::workspace_root;
use voxel_package::{filing, projection};

fn root() -> std::path::PathBuf {
    workspace_root(&std::env::current_dir().expect("a current directory"))
        .expect("the workspace root")
}

/// A workspace of the test's own, **named per test**, so a test run never writes into the
/// repository and two tests never share one.
///
/// **The name is the defect this had, and it was found by the code under test.** The first
/// version keyed the directory on the process id alone, so both tests in this file raced for
/// one path — and the *refuse-to-overwrite* rule then failed the first test with a
/// `RecordExists` on a file it had just written. The rule was right and the harness was
/// wrong: a `RefusalSet` reporting `RecordExists` on a scratch path nobody else uses is a
/// fact about a collision, and reading it as "filing is broken" would have been the C-69
/// defect — a refusal mistaken for a defect in the thing under test. Tests in one binary run
/// concurrently on one thread count by default, so the process id is not a unique key.
fn scratch(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("shploingus-filed-record-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    path
}

/// The one real event: the first ticket, in the player's words.
///
/// **The same words `first_ticket.rs` uses, deliberately.** A second, differently-worded
/// ticket would be a second answer to "what is the first ticket", and the two would drift —
/// which is the same two-records problem this project is trying to get rid of.
fn the_first_ticket() -> Vec<u8> {
    let namespace = Namespace::new("world-2468", 1, Season::Winter, "c-2026-10-01")
        .expect("a namespace");
    render_event(FiledRecord {
        namespace: &namespace,
        event_kind: EventKind::TicketTransition,
        question:
            "mount window[api-viewer] and window[ledger] so the Mods window can show what the \
             arrangement declares",
        answer: None,
        kind: Kind::Authority,
        event_type: EventType::UserDecision,
        state: RecordState::Answered,
        control: Control::Continue,
        actor: "player-1",
        round_id: "c-2026-10-01",
        supersedes: None,
        previous: "not_evaluable",
    })
    .expect("a valid record")
}

#[test]
fn filing_one_real_record_makes_the_gap_wider_by_exactly_one_and_names_the_cost() {
    let workspace = scratch("wider");
    let compressed: Compendium = compendium::compress(&root()).expect("the record compresses");

    // **Measured before, not assumed — and on the SAME chain the filing happens to.**
    //
    // The first version read the *real* root here, on the reasoning that it was "the chain as
    // it is". That was true while the real chain was empty, and it stopped being true the
    // moment the migration was filed: `chains/workspace.chain` now holds 56 events and backs
    // all 55 record entries, so `before` reported **0** unbacked while `after` — measured on
    // the one-event scratch chain — reported 55. The subtraction went negative and the test
    // failed on a record that was correct.
    //
    // The lesson is the one this whole file is about: **a "before" read from a different
    // chain than the "after" is not a measurement of a change.** Both sides belong to the
    // chain being written.
    let before = projection::compare(&compressed, &filing::read(&workspace));
    println!("\nbefore filing: {}", before.describe());
    assert!(
        !before.is_agreement(),
        "if the two views already agree there is nothing for this file to demonstrate, and the \
         emitter question in docs/LEDGER.md should be reopened rather than this test quietly \
         passing"
    );
    // **And the real record is now a real chain**, which is the state this file was opened to
    // reach. Asserted here so the scratch test cannot quietly become the only measurement.
    let real = filing::read(&root());
    assert!(
        !real.is_empty(),
        "**the record is filed.** For the whole of this project's existence every chain write \
         went to a temporary directory; `chains/workspace.chain` exists, and this assertion is \
         here so a future reader does not assume the scratch directory is the state of the \
         project"
    );

    let path = filing::append(&workspace, &the_first_ticket()).expect("the first filing is legal");
    assert!(path.exists(), "the record is on disk, which is the whole point");

    // **Read back through the same path the gate uses.** `read_back` is not called directly:
    // [`filing::read`] is, because that is the function a caller — including `build.rs` — will
    // use, and a test that exercised a different path than production would be testing the
    // test's own choice rather than the code's behaviour.
    let filed = filing::read(&workspace);
    assert_eq!(filed.len(), 1, "one event filed, read back as one event");

    let after = projection::compare(&compressed, &filed);
    println!("after filing:  {}", after.describe());

    assert_eq!(
        before.in_record_not_in_chain.len() - after.in_record_not_in_chain.len(),
        0,
        "the real record does not yet write C-1, so filing an event cannot reduce the unbacked \
         count — and that is the honest state, not a failure. Before: {}, after: {}.",
        before.in_record_not_in_chain.len(),
        after.in_record_not_in_chain.len()
    );
    assert_eq!(
        after.in_chain_not_in_record.len(),
        1,
        "**and the cost is named, not hidden.** The chain now holds C-1, which docs/LEDGER.md \
         does not write — so writing the projection now would delete a real event. This is the \
         disagreement that makes the emitter stay unreachable, and it is one entry wide."
    );
    assert!(
        !after.is_agreement(),
        "the two views still disagree, which is true and is the reason the emitter is gated: \
         {} record entries are unbacked and {} chain entries are unwritten",
        after.in_record_not_in_chain.len(),
        after.in_chain_not_in_record.len()
    );

    let _ = std::fs::remove_dir_all(&workspace);
}

#[test]
fn linked_records_make_the_chain_tamper_evident_rather_than_merely_append_only() {
    // **The pin for the linkage, and the assertion is the chain's own claim.**
    //
    // Every other filing test in this file writes `previous: "not_evaluable"` by hand, which is
    // C-77.4's named limitation: those records **deny knowing the one before**, so the chain is
    // append-only *without tamper evidence* and must never be called tamper-evident (D97:
    // *"never call that state tamper-evident"*).
    //
    // `filing::linked_records` is the chokepoint that removes the limitation: **the writer
    // computes the linkage, so a caller cannot opt out of it.** The first record says
    // `not_evaluable` because it genuinely has no predecessor; every record after it says the
    // digest of the one before.
    let workspace = scratch("linked");
    let namespace =
        Namespace::new("world-2468", 1, Season::Winter, "c-2026-10-01").expect("a namespace");
    let mut bytes: Vec<u8> = Vec::new();
    for _ in 0..3 {
        // **`bytes` is passed in and returned grown**, so the second call links to the first
        // call's record rather than to a genesis digest again.
        bytes = filing::linked_records(
            &workspace,
            &bytes,
            [UnlinkedRecord {
                namespace: &namespace,
                event_kind: EventKind::DevelopmentFact,
                question: "does the linkage hold",
                answer: None,
                kind: Kind::Evidence,
                event_type: EventType::UserDecision,
                state: RecordState::Answered,
                control: Control::Continue,
                actor: "user",
                round_id: "c-2026-10-01",
                supersedes: None,
            }],
        )
        .expect("links");
    }
    filing::append(&workspace, &bytes).expect("files");

    let chain = filing::read(&workspace);
    assert_eq!(chain.len(), 3, "three linked records");

    // **The claim, from the chain's own type rather than a re-derivation of it.**
    //
    // **Every record is linked, including the first** — the first links to `Chain::linked()`'s
    // genesis digest, which is a real digest of no bytes rather than a sentinel. The first
    // version of this test asserted the first record was *un*linked, on the reasoning that it
    // has no predecessor. **That reasoning is what made the chain untamper-evident**, because
    // `is_tamper_evident` requires all of them to be linked — the test was pinning the defect.
    let described = chain.tamper_evident_description();
    assert!(
        described.starts_with("hash-linked"),
        "**a chain whose records all name their predecessors must report itself hash-linked** — \
         the whole point of computing the linkage at the writer. Got: {described}"
    );
    assert!(
        !described.contains("not_evaluable") && !described.contains("NOT tamper-evident"),
        "and must never describe itself as unevaluated. Got: {described}"
    );

    // **Every record is linked, checked through `Linkage` rather than by string-matching
    // `previous`** — a string test would pass on a word that parses wrong.
    let linkages: Vec<bool> =
        chain.fold(Vec::new(), |mut acc: Vec<bool>, event| {
            acc.push(event.previous.is_evaluated());
            acc
        });
    assert_eq!(
        linkages,
        vec![true, true, true],
        "**all three, including the first** — the first links to the genesis digest, not to \
         nothing. Got {linkages:?}"
    );

    // **And `verify` agrees**, which is the reader's own check rather than this test's.
    chain::verify(&chain).expect("a linked chain verifies");

    let _ = std::fs::remove_dir_all(&workspace);
}

#[test]
fn a_second_filing_of_the_same_record_refuses_and_keeps_the_first() {
    // **L-1 §3, exercised through the real path rather than a temp file's edge case.** The
    // refusal is what makes `filing::append` safe to point at anything, so it is tested
    // against the same function the gate will call.
    let workspace = scratch("second-refused");
    filing::append(&workspace, &the_first_ticket()).expect("the first filing is legal");

    let refused = filing::append(&workspace, &the_first_ticket())
        .expect_err("the writer refuses to overwrite a record, ever");
    println!("\nsecond filing refused: {refused}");

    assert_eq!(filing::read(&workspace).len(), 1, "still one event: the refusal did not append");
    let read = read_back(&filing::chain_path(&workspace)).expect("the one record reads back");
    assert_eq!(read.chain.len(), 1, "and it is intact");

    let _ = std::fs::remove_dir_all(&workspace);
}
