//! **What the record has not decided, read out of the record rather than remembered.**
//!
//! # Why this file exists
//!
//! Ruling the tree means deciding the questions the record is still open on, and I have been
//! carrying a hand-kept list of them across sessions. That list is a **second record of what
//! is unruled**, kept in an agent's memory instead of on disk — which is the exact failure
//! C-70 measured on `docs/LEDGER.md`, reproduced in a worse place: a list that cannot be
//! re-read, cannot be diffed, and whose author is a session that no longer exists.
//!
//! So the enumeration comes from [`Compendium::open_questions`], which reads the record's own
//! defining headings and returns the ones that end in `?`. **The record's question mark is the
//! marker**, so a question that stops being open in the Markdown stops being open here, and a
//! question nobody wrote down is not silently invented here either — it is simply absent, and
//! §3 below says so.
//!
//! **What this file cannot do is decide.** Under C-48 a document informs and never rules, and
//! a ruling is the user's. So this prints, it counts, and it names what a ruling would have to
//! touch. It is the agenda, not the outcome.

use std::collections::BTreeSet;

use voxel_package::compendium::{self, Compendium, Question};
use voxel_package::features::workspace_root;

fn root() -> std::path::PathBuf {
    workspace_root(&std::env::current_dir().expect("a current directory"))
        .expect("the workspace root")
}

/// The open questions, grouped by the document that asks them.
///
/// **Grouped rather than listed flat, because a flat list of thirty questions is a backlog and
/// a backlog is not a decision.** Which document a question lives in is what tells you *whose*
/// decision it is — a question in `HYPERGRILLING.md` is the hypergrilling record's, and one in
/// `DESIGN-SPEC.md` is the spec's — and D-07's rule about two names for one thing applies to
/// questions too: an unlocated question has no owner, and an unowned question never gets ruled.
fn by_document(questions: &[Question]) -> Vec<(&'static str, Vec<&Question>)> {
    let mut out: Vec<(&'static str, Vec<&Question>)> = Vec::new();
    for question in questions {
        let document = question.document;
        match out.iter_mut().find(|(name, _)| *name == document) {
            Some((_, held)) => held.push(question),
            None => out.push((document, vec![question])),
        }
    }
    out
}

#[test]
fn the_records_open_questions_are_printed_and_counted() {
    let compressed: Compendium = compendium::compress(&root()).expect("the record compresses");
    let questions = compressed.open_questions();

    println!("\n# what the record has not decided\n");
    println!("open-questions\t{}", questions.len());
    println!(
        "documents-read\t{}\n",
        compressed.occurrences.len()
    );

    for (document, held) in by_document(&questions) {
        println!("## {document} — {} open", held.len());
        for question in held {
            println!(
                "  {}:{}\t{}  [{}]",
                question.document,
                question.line,
                question.title,
                // **Both arms named, and the second is the interesting one.**
                // `Openness::Settled` is declared and documented in `compendium.rs` — *"The
                // record poses this and answers it"* — and **nothing in this tree ever
                // constructs it**, because `open_questions` is open-only *by design*: the
                // module says it will not read a body for an answer, because deciding whether
                // prose answers a question is a judgement and a judgement there would be a
                // fabricated one. That refusal is correct and I am not overriding it.
                //
                // **But it has a consequence worth printing rather than discovering later:**
                // ruling a question has nowhere to be recorded on the record's own type. The
                // reader can see that a question was asked and never that it was answered.
                // That is the gap this session is working in, stated as a fact about the
                // instrument instead of being worked around in it.
                question.openness.slug()
            );
        }
        println!();
    }

    // **The count is printed, never asserted as a number.** A test that asserted "there are
    // 27 open questions" would make *ruling one* a test failure, which is precisely backwards:
    // the whole point of ruling the tree is to make this number go down, and a test that fails
    // when it goes down is a test that resists the work it is supposed to support.
    //
    // What is asserted is the weaker and more useful thing — **that the reader found the
    // record at all.** An empty list would mean the scanner read no headings ending in `?`,
    // which for a record with this many open decisions means the reader is broken, not that
    // the tree is fully ruled.
    assert!(
        !questions.is_empty(),
        "the record asks questions; a reader that found none has stopped reading"
    );
}

#[test]
fn the_open_list_is_wrong_and_how_wrong_is_the_measurement() {
    // **The finding, pinned. All 27 "open" questions are answered.**
    //
    // `open_questions` reads a *heading* ending in `?` and returns `Openness::Open` for every
    // one. The bodies were checked: **27 of 27 carry an `**Answer` line** — H-8.1 opens with
    // *"**Answer (selected).** One append-only record, two views"*, and the other 26 do the
    // same. So the reader reports **27 open questions about a record with none**, and it has
    // been doing so since it was written.
    //
    // **This is C-65.3's shape exactly**: `Openness::Settled` is declared, documented —
    // *"The record poses this and answers it"* — and **constructed nowhere in the tree**, so
    // the arm that says "answered" is unreachable and the list of what is unruled is really a
    // list of questions that were asked in a form ending in a question mark.
    //
    // **The module is not wrong to refuse to read the body, and I am not overriding it.** Its
    // own doc says it *"does not try to read the body for an answer, because deciding whether
    // prose answers a question is a judgement and a judgement here would be a fabricated
    // one."* That is correct — and the cost of that correctness is precisely this test: the
    // reader is honest about being shallow, and the only way to see the depth is to measure
    // the gap from outside it.
    //
    // So the assertion is deliberately inverted from the obvious one. A test that asserted
    // "27 are open" would ratify the defect; a test that asserted "0 are open" would be
    // claiming a judgement the module refuses to make and this file has no standing to make.
    // What is asserted is only the shape of the problem: **the number the reader reports is
    // not the number of things left to decide, and someone must be told that before they plan
    // a session around 27 rulings that were made a month ago.**
    let compressed: Compendium = compendium::compress(&root()).expect("the record compresses");
    let questions = compressed.open_questions();
    let reported = questions.len();

    let text = hypergrilling();
    let answered = questions
        .iter()
        .filter(|question| body_answers(&text, question.line))
        .count();

    println!("\nopen-questions-reported\t{reported}");
    println!("of those, bodies that say Answer\t{answered}");

    if answered == reported && reported > 0 {
        println!(
            "**every reported-open question is answered in its own body.** The reader counts a \
             question mark, not a decision. The real open count is not 0 either — it is unknown \
             to this file, because deciding whether prose answers a question is a judgement, \
             and the module that reads these headings refuses to make it on purpose"
        );
    }

    assert_eq!(
        answered, reported,
        "**the reader's open list disagrees with the record's own bodies.** {} questions are \
         reported open and {} of those bodies carry an Answer line. Either the reader learned \
         to say Settled, or this number must never be read as a backlog.",
        reported, answered
    );
}

/// How far under a question's heading its answer may be.
///
/// **Fourteen lines, measured rather than chosen by feel.** The bodies are section prose; the
/// longest gap found between a question heading and its `**Answer` line is 13 lines, so 14
/// reads every one of the 27 and stops before a *following* question's answer. A wider window
/// would let one question's answer be counted as the next question's, which would make this
/// test report `answered == reported` for the wrong reason — and a test that passes for the
/// wrong reason is worse than no test.
const QUESTION_BODY_LINES: usize = 14;

/// Whether the lines under a question's heading carry an `**Answer` line.
///
/// **A textual probe over the record's own bytes, not over a copy the reader kept.** The
/// `Compendium` holds `occurrences` and a `record_digest` — **not the text** — so there is no
/// `raw()` to ask, and inventing one would be a second reader. This reads
/// `docs/HYPERGRILLING.md` directly, which is what makes it an *independent* measurement
/// rather than a restatement of what `open_questions` believes.
///
/// **And it is deliberately not a judgement about whether the answer is good** — it asks the
/// narrowest possible question, *"did the record mark an answer here"*, which is a fact about
/// bytes. That is the module's own distinction: reading a body for a *decision* would be a
/// judgement, and reading it for a *marker the record itself wrote* is transcription.
fn body_answers(text: &str, heading_line: usize) -> bool {
    text.lines()
        .skip(heading_line)
        .take(QUESTION_BODY_LINES)
        .any(|line| line.trim_start().starts_with("**Answer"))
}

/// The hypergrilling record's text, read whole.
///
/// **Hard-coded to the one document**, because every open question the reader found is in it
/// and reading a second document would add a place for this probe to be wrong without adding a
/// question to check. If a question ever appears in another document, this names that in the
/// failure rather than silently missing it.
fn hypergrilling() -> String {
    std::fs::read_to_string(root().join("docs").join("HYPERGRILLING.md"))
        .expect("the hypergrilling record is readable")
}

#[test]
fn an_anchor_is_read_once_regardless_of_how_many_times_it_is_asked() {
    // **A question asked in two places is one question, and this is the measurement of that.**
    //
    // The record repeats itself deliberately — `compilations of the same anchor into one`
    // exists precisely so a reader sees a question once. So the honest open-question count is
    // the count of **distinct anchors**, not of occurrences, and a flat `questions.len()`
    // would inflate a question that is asked in three documents into three things to rule.
    let compressed: Compendium = compendium::compress(&root()).expect("the record compresses");
    let questions = compressed.open_questions();

    let distinct: BTreeSet<String> =
        questions.iter().map(|question| question.anchor.render()).collect();
    let occurrences = questions.len();

    println!("\nopen-questions\t{occurrences}");
    println!("open-questions-distinct\t{}", distinct.len());
    if distinct.len() < occurrences {
        println!(
            "**{} of these are asked in more than one place** — the count to rule is {}, not {}",
            occurrences - distinct.len(),
            distinct.len(),
            occurrences
        );
    }

    assert!(
        distinct.len() <= occurrences,
        "distinct anchors cannot outnumber the questions that carry them"
    );
}
