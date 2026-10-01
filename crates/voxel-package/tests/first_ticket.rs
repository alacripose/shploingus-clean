//! **The first ticket, as the worked example** — and the calibration it demonstrates.
//!
//! The user asked for four things at once, and this suite is all four:
//!
//! 1. **a test** — this file, and the calibration it pins;
//! 2. **real user-side documentation**, part of the Mods API view, which did not exist;
//! 3. **the judge calibrated against that documentation** — not by making the judge write prose
//!    (its own reference rules that out) but by **running the same instrument over every line
//!    the renderer produced**;
//! 4. **the first ticket as the example** — a real ticket, filed in-game, read back off the
//!    chain, back-compiled into a solution, documented, and judged clean.
//!
//! # Why the judge does not write the document
//!
//! The reference judge's header: *"A judge that could return a sentence would be a judge whose
//! sentence could be quoted."* [`voxel_package::oracle::Verdict`] is three words and carries no
//! prose for exactly that reason. So the document is rendered from **declarations** — the
//! record's anchor titles and the schema's per-field docs — and the calibration is that the
//! renderer **survives the judge that convicts contributors**.

use std::path::{Path, PathBuf};

use voxel_model::chain::{EventKind, Namespace, Season};
use voxel_model::envelope::{Control, EventType, Kind, RecordState};
use voxel_model::wire::Projection;
use voxel_package::apidoc::{self, DocumentVerdict};
use voxel_package::compendium::{self, Anchor, Compendium, Recipe, RecipeRequest};
use voxel_package::compiler::{read_back, render_event, FiledRecord, Solution};
use voxel_package::features::workspace_root;
use voxel_package::oracle::Calibration;

fn root() -> PathBuf {
    workspace_root(&std::env::current_dir().expect("a current directory"))
        .expect("the workspace root")
}

fn namespace() -> Namespace {
    Namespace::new("world-2468", 1, Season::Winter, "c-2026-10-01").expect("a namespace")
}

/// **The first ticket, in the player's words.** The objective names its windows in the
/// compiler's `window[id]` grammar, because that grammar is the one thing about the in-game
/// form that is still unruled — and a ticket that does not parse would make this example a
/// demonstration of nothing.
const FIRST_TICKET: &str = "mount window[api-viewer] and window[ledger] so the Mods window can \
                            show what the arrangement declares";

/// The record anchors this document is derived from. **Named here, not searched** — a relevance
/// score invented by the test would be a way of deciding what the record means that the record
/// has not delegated, and a wrong declaration is visible while a wrong score is invisible.
const FIRST_TICKET_ANCHORS: [&str; 3] = ["C-62", "C-64", "C-68"];

/// The same three, parsed. **A parse, not a hand-built struct**, so a malformed anchor is a
/// compile-time-visible failure here rather than a citation nothing can check later.
fn first_ticket_anchors() -> Vec<Anchor> {
    FIRST_TICKET_ANCHORS
        .iter()
        .map(|text| Anchor::parse(text).unwrap_or_else(|why| panic!("{text}: {why}")))
        .collect()
}

fn scratch(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "shploingus-first-ticket-{}-{name}.chain",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    path
}

/// File the first ticket the way the in-game client files one.
fn file_first_ticket(path: &Path) {
    let record = render_event(FiledRecord {
        namespace: &namespace(),
        event_kind: EventKind::TicketTransition,
        question: FIRST_TICKET,
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
    .expect("a valid record");
    let mut file = std::fs::File::create(path).expect("creates");
    use std::io::Write;
    file.write_all(&record).expect("writes");
}

/// The recipe the document is measured against, derived from the record — not written here.
fn derived_recipe() -> Recipe {
    let compressed: Compendium = compendium::compress(&root()).expect("the record compresses");
    compressed
        .derive(&RecipeRequest::of(
            "the first ticket's API view",
            first_ticket_anchors(),
        ))
        .expect("a recipe from decisions the record holds")
}

fn calibration() -> Calibration {
    let report = voxel_package::oracle::calibrate_structural();
    assert!(
        report.calibration.is_quotable(),
        "the judge must beat the majority class before anything it says may be quoted: {:?}",
        report.calibration
    );
    report.calibration
}

#[test]
fn the_first_ticket_is_documented_and_the_document_survives_the_judge() {
    // ---- 1. filed in-game, on the chain ---------------------------------
    let path = scratch("first");
    file_first_ticket(&path);

    // ---- 2. read back off the chain -------------------------------------
    let read = read_back(&path).expect("the chain reads");
    assert_eq!(read.corrupt, 0, "nothing corrupt: {:?}", read.corrupt_detail);
    assert_eq!(read.board.len(), 1, "one ticket came back");
    let ticket = &read.board.tickets()[0];
    assert_eq!(ticket.id.as_str(), "BLD-0000", "and it has a real id");
    assert!(
        ticket.objective.contains("api-viewer"),
        "carrying the player's own words: {}",
        ticket.objective
    );

    // ---- 3. back-compiled into a solution -------------------------------
    let solution = Solution::for_ticket(ticket).expect("the ticket compiles");
    assert_eq!(
        solution.windows.len(),
        2,
        "two windows, and both were named in the ticket — none invented"
    );

    // ---- 4. documented ---------------------------------------------------
    let recipe = derived_recipe();
    assert!(
        apidoc::has_sources(&recipe),
        "the document is derived from decisions on the record, not from nothing"
    );
    let document = apidoc::render(&solution, &recipe);

    // **Printed always.** C-45: the run is the user's to read, and the whole point of this
    // example is that a human can read what the system produced.
    println!("\n{document}\n");

    // ---- 5. the calibration: judged by the same instrument ---------------
    let verdict: DocumentVerdict = apidoc::check_against(&document, &recipe, &solution, &calibration())
        .expect("the document is clean");

    assert!(
        verdict.convicted.is_empty(),
        "nothing in the document reads as undeclared: {:?}",
        verdict.convicted
    );
    assert!(
        verdict.is_substantive(),
        "and the document is not vacuous: {verdict:?}\n\n{document}"
    );
    assert!(
        verdict.judged > 0,
        "the judge actually read it, rather than there being nothing to read"
    );
    // **The branch that could not execute before, pinned.** This document's headings and blank
    // lines are `NothingToJudge`; `lint` turns each into a `NoReading` *refusal*, and
    // `check` re-counts those as declines. The first version counted them from a
    // `Verdict::NoReading` that `lint` never returns, so `declined` was always zero and the
    // counter was decoration. An assertion of `> 0` is the only thing that makes the fix real.
    assert!(
        verdict.declined > 0,
        "headings and blank lines are declined rather than convicted, and a decline is counted \
         as a decline: {verdict:?}\n\n{document}"
    );
    println!(
        "\ncalibration: {} lines, {} judged, {} declined, {} convicted",
        verdict.lines, verdict.judged, verdict.declined, verdict.convicted.len(),
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn the_document_carries_its_anchors_so_a_reader_can_check_them() {
    // **A document that cannot be traced to a ruling is a document with no appeal.** Every
    // source anchor must appear in the rendered text, spelled the way the record spells it.
    let path = scratch("anchors");
    file_first_ticket(&path);
    let read = read_back(&path).expect("reads");
    let solution = Solution::for_ticket(&read.board.tickets()[0]).expect("compiles");
    let recipe = derived_recipe();
    let document = apidoc::render(&solution, &recipe);

    assert_eq!(
        apidoc::sources_of(&recipe).len(),
        FIRST_TICKET_ANCHORS.len(),
        "three decisions, all authority"
    );
    for source in apidoc::sources_of(&recipe) {
        assert!(
            document.contains(&source.render()),
            "the document names {} so a reader can go and check it",
            source.render()
        );
    }

    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_document_with_no_decision_behind_it_says_so() {
    // **Never silent.** "Derived from nothing" and "we forgot to record the derivation" print
    // identically otherwise, and the first is a legitimate state worth showing.
    let path = scratch("unbacked");
    file_first_ticket(&path);
    let read = read_back(&path).expect("reads");
    let solution = Solution::for_ticket(&read.board.tickets()[0]).expect("compiles");

    let compressed: Compendium = compendium::compress(&root()).expect("compresses");
    // **An anchor the record does not hold is a refusal, not an empty recipe** — so the
    // unbacked case is built by hand, which is the only way to reach it.
    let unbacked = Recipe {
        subject: "an unbacked document".to_string(),
        text: String::new(),
        sources: Vec::new(),
        allow_research: false,
    };
    assert!(!apidoc::has_sources(&unbacked));
    let document = apidoc::render(&solution, &unbacked);
    assert!(
        document.contains("derived from: none"),
        "and the document says so on its face: {document}"
    );
    // **Stronger than a substring.** An unbacked document must also *survive the judge* — if
    // the honest statement of "no sources" were undeclared, the renderer would have to either
    // license function words or go silent, and both are worse. This is the assertion that
    // caught the first wording.
    let verdict = apidoc::check_against(&document, &unbacked, &solution, &calibration())
        .expect("an unbacked document is still a document; it just says so");
    assert!(
        verdict.is_substantive(),
        "and it is not a refusal in place of a document: {verdict:?}\n\n{document}"
    );

    let _ = compressed;
    let _ = std::fs::remove_file(&path);
}

#[test]
fn the_calibration_can_fail_and_that_is_the_whole_point() {
    // **The defect this whole module exists to prevent, pinned as a test.** Three ways the
    // check could pass without the document being any good, each refused:
    //
    // * an **uncalibrated** oracle — `lint` refuses before the judge is consulted, so an
    //   unmeasured run cannot certify a document;
    // * an **empty** document — every line is `no_reading`, so nothing is convicted, so a
    //   renderer that emits nothing would pass. [`apidoc::check`] refuses on `judged == 0`;
    // * a document whose prose **invents a term the record does not license** — which is the
    //   one this is really for.
    let recipe = derived_recipe();

    // 1. Uncalibrated.
    let uncalibrated = Calibration::unmeasured();
    assert!(
        !uncalibrated.is_quotable(),
        "an unmeasured run is not quotable — that is the precondition, and it is false here"
    );
    // The three bad documents are judged against the **real** solution's vocabulary, so the
    // failures below are the failures claimed and not an artefact of a stripped vocabulary.
    let path = scratch("calibration");
    file_first_ticket(&path);
    let read = read_back(&path).expect("reads");
    let solution = Solution::for_ticket(&read.board.tickets()[0]).expect("compiles");

    let refused = apidoc::check_against(
        "window[api-viewer]\n",
        &recipe,
        &solution,
        &uncalibrated,
    )
    .expect_err("an uncalibrated oracle certifies nothing");
    assert!(
        refused
            .all()
            .iter()
            .any(|r| r.detail().contains("not been calibrated")),
        "and it says why: {refused}"
    );

    // 2. Empty.
    let empty = apidoc::check_against("", &recipe, &solution, &calibration())
        .expect_err("an empty document is not a clean document");
    assert!(
        empty
            .all()
            .iter()
            .any(|r| r.detail().contains("convicted nothing")),
        "a document that is perfect and a document that is empty print identically, and this \
         is the only place they differ: {empty}"
    );

    // 3. Invented vocabulary. **This is the one that matters**: a renderer that writes prose
    //    the record never said is exactly the failure the whole project refuses, and here it
    //    is caught by the same instrument that convicts a contributor.
    //
    //    **What the instrument can and cannot say, measured.** The refusal names the *line* —
    //    `line 1`, `line 2` — and the verdict, but it does **not** name the culprit term,
    //    because `Oracle::judge` returns a three-word `Verdict` and the culprits live in
    //    `Reading::Undeclared`, which never leaves the judge. An earlier version of this test
    //    asserted the culprits were named and it failed; the honest response is to assert what
    //    is true and record the gap, not to add a prose channel to the judge for a test.
    //    `apidoc::check` itself does carry the offending **line text** in
    //    `DocumentVerdict::convicted`, which is why the assertion below can reach it.
    let invented = "## quantum flux capacitor recalibration\n- turbo encabulate the widgets\n";
    let caught = apidoc::check_against(invented, &recipe, &solution, &calibration())
        .expect_err("invented vocabulary is convicted");
    let reported = caught
        .all()
        .iter()
        .map(|r| r.detail().to_string())
        .collect::<Vec<_>>()
        .join(" | ");
    assert!(
        reported.contains("line 1") && reported.contains("line 2"),
        "and the refusal names which line, not merely that something was refused: {reported}"
    );
    // Every line of this document is invented, so every line must be convicted — a check that
    // refused only the first would be a check that stops reading.
    assert_eq!(
        caught.all().len(),
        2,
        "one refusal per invented line, no more and no fewer: {reported}"
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_document_is_not_the_judges_sentence() {
    // **The rule that shapes this module, asserted rather than assumed.** The verdict type
    // carries no prose, so nothing the judge says can be quoted into a document — and a
    // document is assembled only from declarations and the renderer's own declared vocabulary.
    let recipe = derived_recipe();
    // **The real vocabulary, gathered from where it is declared** — the renderer's own list,
    // the grammar's closed enums, and the subject's own words. Spelled out by hand here it
    // would be a fourth copy of a list that three declarations already hold.
    let path = scratch("not-the-sentence");
    file_first_ticket(&path);
    let read = read_back(&path).expect("reads");
    let solution = Solution::for_ticket(&read.board.tickets()[0]).expect("compiles");
    let mut declared = recipe.licensed_terms();
    for phrase in apidoc::declared_vocabulary(&solution) {
        declared.extend(voxel_package::oracle::terms(&phrase));
    }
    assert!(
        !declared.is_empty(),
        "the renderer has a declared vocabulary, so its connective tissue is auditable"
    );
    // And the document's own headings are drawn from that vocabulary, not invented at the
    // call site.
    for heading in ["Asked for", "Declares", "Read-back required to retire"] {
        assert!(
            voxel_package::oracle::terms(heading)
                .iter()
                .all(|term| declared.contains(term)),
            "the heading {heading:?} uses only declared vocabulary"
        );
    }
    let _ = Projection::new(voxel_model::wire::CHAIN_EVENT_SCHEMA);
    let _ = std::fs::remove_file(&path);
}
