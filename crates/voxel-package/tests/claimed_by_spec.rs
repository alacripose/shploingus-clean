//! **The whole project, measured against the spec's own declaration tables.**
//!
//! This is the consuming pass the record asks for and that nothing was doing: every crate in
//! the workspace is measured against *what the spec says it is for*, and a crate the spec never
//! names comes back **unclaimed** — the record's own word, and a different fact from
//! mis-conformance.
//!
//! **It reads the spec, not a table of prose about the spec.** That is the whole point, and
//! the previous version of the census got it wrong: its `Declared` axis read twelve sentences
//! the agent had written, so a crate nobody declared scored `active` because the author had
//! described it. An axis graded against the grader's own prose cannot fail.

use voxel_package::census::{claim_against_spec, claim_report, Crate, Claim, DECLARATION_TABLES};
use voxel_package::features::workspace_root;

fn root() -> std::path::PathBuf {
    workspace_root(&std::env::current_dir().expect("a current directory"))
        .expect("the workspace root")
}

#[test]
fn the_whole_project_against_the_spec() {
    let claims = claim_against_spec(&root()).expect("the spec's tables are readable");

    // Every crate is measured. **Not a sample** — a census that reads ten of twelve is a
    // census of the parts the author liked.
    assert_eq!(
        claims.len(),
        Crate::ALL.len(),
        "every crate is measured, because the point of a census is the ones you did not look at"
    );

    // Printed **always**, not only on failure. C-45: the run is the user's to read, and a
    // number that only exists when something breaks is a number nobody checks.
    println!("\n# the spec's declaration tables\n");
    for table in DECLARATION_TABLES {
        println!("table\t{table}");
    }
    println!("\n# every crate, measured against them\n");
    print!("{}", claim_report(&claims));

    let unclaimed: Vec<&Claim> = claims.iter().filter(|c| !c.is_claimed()).collect();
    println!("\nclaimed\t{}", claims.len() - unclaimed.len());
    println!("unclaimed\t{}", unclaimed.len());
    for claim in &unclaimed {
        println!("unclaimed-by\t{}", claim.why_unclaimed());
    }
    println!();
}

#[test]
fn a_claim_carries_its_locator_so_a_reader_is_one_click_away() {
    // **A claim with no locator cannot be checked by the person it affects**, which makes it
    // an assertion rather than a measurement. Every claimed crate therefore names a document
    // and a line.
    for claim in claim_against_spec(&root()).expect("readable") {
        if claim.is_claimed() {
            assert!(
                claim.witness.is_some() && claim.line.is_some(),
                "{} is claimed with no document:line, so nobody can check the claim",
                claim.crate_.name()
            );
            assert!(
                claim.line.expect("checked above") >= 1,
                "{} is claimed at line {}, and line numbers here are 1-based because that is \
                 what an editor and a `grep -n` both say",
                claim.crate_.name(),
                claim.line.expect("checked above")
            );
        }
    }
}

#[test]
fn an_unclaimed_crate_is_not_called_mis_conformant() {
    // D97, verbatim from the record this tree inherits: *"A file that implements no row is not
    // mis-conformant, it is `unclaimed`, and that is a different refusal."* The refusal text
    // must therefore say **unclaimed** and must not claim the crate is broken.
    for claim in claim_against_spec(&root()).expect("readable") {
        if !claim.is_claimed() {
            let why = claim.why_unclaimed();
            assert!(
                why.contains("unclaimed"),
                "an unclaimed crate says so: {why}"
            );
            assert!(
                !why.contains("mis-conformant") || why.contains("not mis-conformant"),
                "and it must not be called a conformance failure, which is a different fact: {why}"
            );
        }
    }
}

#[test]
fn the_spec_declares_fewer_crates_than_the_workspace_holds() {
    // **The finding, pinned so it cannot be quietly closed.** If a future ruling gives every
    // crate a row, this test fails and the ledger entry that made it true gets rewritten —
    // which is the correct way for that to happen: by a decision on the record, not by an
    // agent editing a table until the gate goes green.
    let claims = claim_against_spec(&root()).expect("readable");
    let unclaimed: Vec<&str> = claims
        .iter()
        .filter(|c| !c.is_claimed())
        .map(|c| c.crate_.name())
        .collect();

    println!("\nunclaimed crates: {unclaimed:?}\n");
    assert!(
        !unclaimed.is_empty(),
        "every crate is claimed, so the spec has been extended past what this test was written \
         against. That is a ruling on the record and this test should be updated with it"
    );
    assert!(
        unclaimed.contains(&"voxel-work") && unclaimed.contains(&"voxel-ui"),
        "and specifically the two the census already reported gaps on: {unclaimed:?}"
    );
}

#[test]
fn the_check_can_fail_and_that_is_the_whole_point() {
    // **The bug this suite exists to catch, pinned as a test.** The first version searched
    // whole documents for a crate name and reported twelve of twelve claimed — including
    // `shploingus` at `DESIGN-SPEC.md:1`, which is the document's *title line* and contains
    // the project's name. A perfect score from a check that cannot fail is the most dangerous
    // thing a gate can print, because it looks like success.
    //
    // So: three ways a name appears in that document that are **not** a declaration, and each
    // must come back unclaimed by the row test.
    let root = root();
    let spec = std::fs::read_to_string(root.join("docs/DESIGN-SPEC.md")).expect("readable");

    // 1. The title line. It contains "shploingus" because the project is called shploingus.
    let title = spec.lines().next().expect("a first line");
    assert!(
        title.contains("shploingus") && !declares_here(title, "shploingus"),
        "the title line mentions the project name and must not declare the binary crate"
    );

    // 2. A prose mention outside any table row.
    let prose = spec
        .lines()
        .find(|line| line.contains("voxel-instrument") && !line.trim_start().starts_with('|'))
        .expect("voxel-instrument is mentioned in prose");
    assert!(
        !declares_here(prose, "voxel-instrument"),
        "a prose mention is a mention, not a declaration: {prose}"
    );

    // 3. A table row that names a crate inside a longer cell is still a declaration — the
    //    inverse check, because a rule that refuses everything is as useless as one that
    //    accepts everything.
    let f12 = spec
        .lines()
        .find(|line| line.contains("F12") && line.contains("voxel-mesh"))
        .expect("the F12 row");
    assert!(
        declares_here(f12, "voxel-mesh") && declares_here(f12, "voxel-stream"),
        "a cell naming two crates declares both, or two declared crates read as undeclared: {f12}"
    );
}

/// The row test, **reached through the module's own function** rather than re-implemented, so
/// this suite pins the real rule instead of a copy that can drift from it.
fn declares_here(line: &str, needle: &str) -> bool {
    voxel_package::census::declares_in_row_for_test(line, needle)
}
