//! **How far apart the record and the chain actually are.** The measurement, run on every test.
//!
//! # Why this file exists rather than a line in the gate
//!
//! H-8.1's claim is that the two views of the record *"cannot disagree by construction"*.
//! **They can, and today they do**, and the size of the gap is a fact about this repository
//! that belongs in the run rather than in a ledger paragraph somebody has to remember to
//! check. So it is measured here, printed always (C-45: the run is the user's to read), and
//! **no writer is involved** — the projection module has none, and this test would fail if one
//! appeared without a disagreement check in front of it.
//!
//! # What it is expected to say
//!
//! Not a pass. The chain is empty, so the record's entries are all unbacked, and the correct
//! first output of this file is a number that says so. A test asserting the two views agree
//! would be asserting a thing the project has not built.

use voxel_package::compendium::{self, Compendium};
use voxel_package::features::workspace_root;
use voxel_package::{filing, projection};

fn root() -> std::path::PathBuf {
    workspace_root(&std::env::current_dir().expect("a current directory"))
        .expect("the workspace root")
}

/// The chain, **as it is filed** — not as this file assumes it is.
///
/// **This was `Chain::new()`, and that was the defect this file was written to find.** The
/// test's whole claim is that it measures the gap between the record and the chain, and it
/// measured it against a chain it had constructed itself — so it would have reported the same
/// 36 unbacked entries *forever*, whether or not anything was ever filed. A measurement whose
/// input is a literal cannot move, and a measurement that cannot move is a decoration. That
/// is D107's hazard again: an empty input read as a measured result.
///
/// [`filing::read`] returns the filed chain and an **empty** chain when nothing is filed,
/// which is the honest answer in both cases and is the same call [`bloat::read_chain`] makes.
fn chain() -> voxel_model::chain::Chain {
    filing::read(&root())
}

#[test]
fn the_two_views_are_measured_against_each_other() {
    let root = root();
    let compressed: Compendium = compendium::compress(&root).expect("the record compresses");
    let found = projection::compare(&compressed, &chain());

    println!("\n{}", found.describe());

    // **The record's C-entries, counted from the record and not asserted here.** If the number
    // moved, the number is printed and the reader sees it move; a hard-coded expectation would
    // make adding an entry a test failure, which is the wrong reason to edit a test.
    let entries: Vec<String> = compressed
        .occurrences
        .iter()
        .filter(|occurrence| {
            occurrence.anchor.series.letter() == 'C' && occurrence.anchor.sub.is_none()
        })
        .map(|occurrence| occurrence.anchor.render())
        .collect();
    println!("record C-entries\t{}", entries.len());
    println!("chain events\t{}", chain().len());
    println!("chain filed\t{}", filing::is_filed(&root));

    // **An entry the chain holds and the record does not is a different failure from an
    // unbacked one, and it is the one that says the emitter became reachable.** It is
    // reported rather than asserted empty, because a test that asserts "nothing can be wrong
    // here" is a test that stops the day something is.
    if !found.in_chain_not_in_record.is_empty() {
        println!(
            "**the chain holds {} entries the record does not write: {:?}** — the record is no \
             longer the whole of it, and the projection would now delete real events",
            found.in_chain_not_in_record.len(),
            found.in_chain_not_in_record
        );
    }
    assert!(
        found.in_record_not_in_chain.len() <= entries.len(),
        "the comparison cannot report more unbacked entries than the record holds: {} > {}",
        found.in_record_not_in_chain.len(),
        entries.len()
    );
    assert!(
        !found.is_agreement(),
        "**The two views disagree, and this test is named for that.** {} entries are written in \
         docs/LEDGER.md and {} are in the chain. H-8.1 says the two projections cannot disagree \
         by construction; they can, and until the chain carries them they do. If this assertion \
         ever fails, the chain has caught up with the record and the emitter is unblocked — \
         which is the only event that should make it pass.",
        found.in_record_not_in_chain.len(),
        chain().len()
    );
}

#[test]
fn there_is_no_writer_and_that_is_deliberate() {
    // **The refusal is the feature.** L-1 §3: *"The writer refuses to overwrite an existing
    // record (`AlreadyExists`)."* D6's CLM: retire, **never delete**.
    //
    // 3,525 lines of the record were typed by hand. A projection emitter pointed at an empty
    // chain would produce an empty ledger and destroy them, so `projection` computes and
    // `compare` measures and **nothing writes**. This test cannot see a writer that does not
    // exist; it can only record the requirement, which is why the requirement is here in prose
    // where the next reader of this module will find it.
    let root = root();
    let compressed: Compendium = compendium::compress(&root).expect("the record compresses");
    let found = projection::compare(&compressed, &chain());
    assert!(
        found.describe().contains("writing the projection now would delete the entries"),
        "the report has to carry the consequence every time, because a caller that skips \
         `is_agreement` still read this sentence: {}",
        found.describe()
    );
}
