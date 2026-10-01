//! **Are the record's answers real, or only claimed?** The audit the user asked for.
//!
//! # The distinction this file exists to draw
//!
//! C-74 measured that **all 27 of the record's "open" questions carry an `**Answer` line** —
//! so the reader's count of what is unruled is 100% wrong. That established the record
//! *claims* to have answered them. It says nothing about whether the answers are **implemented**,
//! and those are different questions with different consequences:
//!
//! * **Claimed** — the record writes down an answer. Cheap; a paragraph does it.
//! * **Implemented** — the tree does the thing the answer describes, and a reader can check.
//!
//! **A claim without an implementation is the defect this project already found twice** —
//! `provenance_digest unchecked, no witness at …` and the `RESUME_SAFE_LEVELS` array in
//! `ala-system` that `satisfies` proves well-typed while `isResumeSafe` says something else.
//! Both looked fine and both lied. So the 27 answers are audited here rather than trusted.
//!
//! # What the audit checks, and what it cannot
//!
//! **The instrument already exists and this file does not replace it.** `features.rs` maps
//! rulings to witnesses and `check_features` verifies the witness **file exists** — and its
//! own doc is honest that a row's `means` text is *"checked against nothing"*:
//!
//! > *"The field most able to be lied about, which is why `check_features` checks the witness
//! > against the filesystem and this text against nothing. It is here to be read by a person,
//! > and its being unchecked is stated here rather than left for a reader to assume otherwise."*
//!
//! **So a file existing is not a feature implemented**, and this file measures the residue: a
//! question whose answer names a witness the tree does not have, or names **no** witness at
//! all. The second is the common case and the important one — a ruling about a design is not
//! supposed to have a file, and treating that as unimplemented would convict 27 correct
//! answers of not being code.
//!
//! **What this file cannot do, stated plainly:** it cannot judge whether an implemented answer
//! is the *right* answer, or whether a design answer was followed in the parts no file names.
//! It separates *checkable* from *not checkable* and counts both. Anything it counts as
//! unverifiable is a **question for a person, not a defect**.
//!
//! **A first draft of this file carried a `Verifiable` enum** — `Witnessed` / `Unwitnessed` /
//! `Unverifiable` — to name the three outcomes. **Clippy removed it as dead code, correctly:**
//! no answer in the record names a path, so two of the three variants are never constructed
//! and the third is every answer. The enum asserted a discrimination the data does not
//! support, and the counts below say what is true without it.

use std::collections::BTreeSet;

use voxel_package::compendium::{self, Compendium};
use voxel_package::features::{self, Feature, FEATURES};
use voxel_package::features::workspace_root;

fn root() -> std::path::PathBuf {
    workspace_root(&std::env::current_dir().expect("a current directory"))
        .expect("the workspace root")
}

/// The `.rs` / `.wgsl` paths a body names, as witness candidates.
///
/// **A textual probe, deliberately narrow, and its narrowness is the finding.** It looks for a
/// path-shaped token that exists in the tree. It does **not** try to work out whether the file
/// implements the sentence — that is a judgement, and this file has no standing to make one.
/// What it can do is separate the answers that *point at code* from the ones that do not, and
/// the second group is where "claimed but not built" hides.
fn named_paths(body: &str) -> Vec<String> {
    let mut found = BTreeSet::new();
    let bytes: Vec<char> = body.chars().collect();
    let mut index = 0usize;
    while index < bytes.len() {
        // **A path starts at a boundary, not mid-word.** `mod.rs` inside `domino.rs` is not
        // a path, and matching it would invent a witness that does not exist.
        let at_boundary = index == 0
            || !(bytes[index - 1].is_alphanumeric() || bytes[index - 1] == '_');
        if at_boundary && (bytes[index].is_alphanumeric() || bytes[index] == '_') {
            let start = index;
            while index < bytes.len()
                && (bytes[index].is_alphanumeric()
                    || matches!(bytes[index], '/' | '.' | '_' | '-'))
            {
                index += 1;
            }
            let token: String = bytes[start..index].iter().collect();
            if token.contains('/') && (token.ends_with(".rs") || token.ends_with(".wgsl")) {
                found.insert(token);
            }
            continue;
        }
        index += 1;
    }
    found.into_iter().collect()
}

#[test]
fn every_answered_question_is_audited_against_the_tree() {
    let compressed: Compendium = compendium::compress(&root()).expect("the record compresses");
    let questions = compressed.open_questions();
    let text = std::fs::read_to_string(root().join("docs").join("HYPERGRILLING.md"))
        .expect("the hypergrilling record is readable");
    let lines: Vec<&str> = text.lines().collect();

    let mut witnessed = 0usize;
    let mut unwitnessed: Vec<(String, String)> = Vec::new();
    let mut unverifiable: Vec<(String, String)> = Vec::new();

    println!("\n# are the record's answers implemented?\n");

    for question in &questions {
        // The body is the prose under the heading, up to the next heading.
        let start = question.line.min(lines.len().saturating_sub(1));
        let body: String = lines[start..]
            .iter()
            .take(BODY_LINES)
            .skip(1)
            .cloned()
            .collect::<Vec<&str>>()
            .join("\n");

        let answer_line = body
            .lines()
            .find(|line| line.trim_start().starts_with("**Answer"))
            .unwrap_or("");
        // **Only the answer itself is audited, not the whole section.** A section's
        // *consequences* prose names files the answer does not — auditing that would report a
        // design ruling as unimplemented because its follow-up work is described nearby.
        let named = named_paths(answer_line);

        let anchor = question.anchor.render();
        if named.is_empty() {
            println!("{anchor}\tunverifiable\tno path named in the answer");
            unverifiable.push((anchor, question.title.clone()));
            continue;
        }

        let missing: Vec<String> = named
            .iter()
            .filter(|path| !root().join(path).is_file())
            .cloned()
            .collect();
        if missing.is_empty() {
            witnessed += 1;
            println!("{anchor}\twitnessed\t{}", named.join(", "));
        } else {
            println!("{anchor}\t**UNWITNESSED**\tnamed {missing:?} and the tree has none of them");
            unwitnessed.push((anchor, missing.join(", ")));
        }
    }

    println!("\nanswered-questions\t{}", questions.len());
    println!("answer-witnessed\t{witnessed}");
    println!("answer-unwitnessed\t{}", unwitnessed.len());
    println!("answer-unverifiable\t{}", unverifiable.len());

    // **The assertion is the one that matters and it is deliberately narrow.** A test that
    // demanded every answer be witnessed would fail on 20+ correct design rulings, and the fix
    // would be to delete the test. What must hold is only that **the audit ran and classified
    // every question** — so an unclassified answer is a defect in the audit, and an unwitnessed
    // answer is a *result*, printed for a person to rule on.
    //
    // `Unwitnessed` is therefore reported, not asserted empty. Under C-48 a document informs
    // and never rules: whether an answer that names a missing file is a wrong answer or a
    // forward reference to work not yet done is a judgement, and this file does not make it.
    assert_eq!(
        witnessed + unwitnessed.len() + unverifiable.len(),
        questions.len(),
        "every answered question must land in exactly one of the three verdicts: {} witnessed, \
         {} unwitnessed, {} unverifiable, {} questions",
        witnessed,
        unwitnessed.len(),
        unverifiable.len(),
        questions.len()
    );
    // **The audit's first real result, asserted as a result.** No answer in the record names a
    // `.rs` path — measured, not assumed: `grep -A 2 '^\*\*Answer' docs/HYPERGRILLING.md` over
    // all 56 answers returns **zero** paths. **39 of the 56 are `**Answer (user's words).**`
    // followed by a blockquote**, so the answer proper is prose in the user's voice and a path
    // would sit in the *consequences* section below it, not in the answer.
    //
    // **So `witnessed + unwitnessed == 0` is asserted, and it is the finding rather than a
    // placeholder.** It says the witness probe currently discriminates nothing, which is true
    // and is the reason a second probe is owed: the record's authority is currently checked
    // only through `FEATURES`, which reaches 19 features and **5 rulings**, not through the 56
    // answers themselves.
    //
    // **The first draft of this assertion was `assert!(true, …)`.** Clippy refused it, and it
    // was right to: an assertion that is always true while carrying a paragraph of explanation
    // is a comment wearing a test's clothes, and it would have passed forever. This one can
    // fail — the day a ruling starts naming its own witness, it must be updated to say so.
    assert_eq!(
        witnessed + unwitnessed.len(),
        0,
        "**an answer now names a path in the tree.** The witness probe has started \
         discriminating, so `Verifiable::Witnessed` / `Unwitnessed` are no longer dead code and \
         this audit has a real signal to report. Update the counts above and stop claiming the \
         record names no code"
    );
    assert_eq!(
        unverifiable.len(),
        questions.len(),
        "with no answer naming a path, every question is unverifiable by this probe — and \
         that is the gap: the record's answers are audited only as prose"
    );
}

#[test]
fn the_feature_rows_are_the_audits_already_declared_and_measured() {
    // **The audit does not start from zero, and this is the measurement of that.**
    //
    // Five of the 19 feature rows are declared by **H-anchors** — H-8.1, H-8.3, H-9.7, H-9.8,
    // H-12.2 — so the rulings that carry witnesses already carry them, and `check_features`
    // already verifies the file is present. **The gap this file found is narrower and sharper
    // than "rulings are unaudited": a witness file existing is not the answer implemented, and
    // `means` is checked against nothing by the module's own admission.**
    //
    // So the count printed here is *how much of the record's authority is already machine-
    // checked*, and it is printed rather than asserted because it should **grow** as rulings
    // are wired up — an assertion would resist the very work this file exists to enable.
    let anchored: Vec<&features::FeatureRow> =
        FEATURES.iter().filter(|row| row.declared_by.starts_with('H')).collect();

    println!("\nfeature-rows\t{}", FEATURES.len());
    println!("feature-rows-declared-by-an-ruling\t{}", anchored.len());
    for row in &anchored {
        println!("  {}\t{}\t{}", row.declared_by, row.feature.name(), row.means);
    }
    println!(
        "feature-rows-with-a-witness-file\t{}",
        FEATURES.iter()
            .filter(|row| root().join(features::witness_of(row.feature)).is_file())
            .count()
    );

    // **The missing witnesses are REPORTED, not asserted away — and not asserted present
    // either.** The record's own rule is that a feature with no witness is `unchecked` and
    // *pending*: `check_required` stops the build only for `required_for_alpha()`, and the gate
    // prints the rest with their reasons on every build. So four rows are currently `unchecked`
    // because they point into `crates/voxel-client/src/shell/`, **a directory that does not
    // exist**. Asserting they must exist would demand the shell be built before this test
    // passes; asserting nothing would discard the finding.
    //
    // So the assertion is the record's own invariant, not a completion claim: **every
    // ruling-declared row has a witness path that is a real path shape**, whether or not the
    // file is there yet. A row naming `""` or a slug would be a row that lies about itself,
    // and that *is* a defect the record forbids.
    let missing: Vec<&str> = anchored
        .iter()
        .filter(|row| !root().join(features::witness_of(row.feature)).is_file())
        .map(|row| row.feature.name())
        .collect();
    if !missing.is_empty() {
        println!(
            "**{missing:?} are declared by a ruling and have no witness file.** Each points \
             into crates/voxel-client/src/shell/, which does not exist. Under the record's own \
             rule these are *pending*, and the gate prints them with the reason on every build — \
             so this is work not done, stated, not a broken row"
        );
    }
    for row in &anchored {
        let witness = features::witness_of(row.feature);
        assert!(
            witness.contains('/') && witness.ends_with(".rs"),
            "{}: the witness {} is not a path, so the row names nothing anyone can look for",
            row.feature.name(),
            witness
        );
    }

    // And the rows exist at all, so the audit has something to audit.
    assert!(
        !FEATURES.is_empty() && !Feature::ALL.is_empty(),
        "the declared feature rows are the audit's substrate and they must not be empty"
    );
}

/// How far under a question's heading its answer's body is read.
///
/// **The answer line only** — see [`named_paths`]'s caller. Bounded so a long section's
/// follow-up prose cannot contribute a path to an answer that named none.
const BODY_LINES: usize = 14;
