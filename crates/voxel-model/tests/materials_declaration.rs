//! The declaration and the committed table are one thing, checked end to end.
//!
//! The unit tests prove the gate refuses each defect; these prove the shipped
//! declaration passes it and that `src/materials/generated.rs` is exactly what
//! the generator writes — so a hand-edited or stale table is a test failure as
//! well as a build failure (the digest gate in `build.rs`).

use std::fs;
use std::path::PathBuf;

use voxel_model::materials::{declaration, generated, schema::Table};

fn read(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("materials")
        .join(name);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} must be readable: {error}", path.display()))
}

fn declared_table() -> Table {
    let tiers = declaration::parse_tiers("tiers.tsv", &read("tiers.tsv")).expect("tiers parse");
    let substances = declaration::parse_substances("substances.tsv", &read("substances.tsv"))
        .expect("substances parse");
    let processes = declaration::parse_processes("processes.tsv", &read("processes.tsv"))
        .expect("processes parse");
    let vocabulary = declaration::parse_vocabulary("vocabulary.tsv", &read("vocabulary.tsv"))
        .expect("vocabulary parse");
    Table::new(tiers, substances, processes, vocabulary)
}

#[test]
fn the_shipped_declaration_gates_clean() {
    let table = declared_table();
    let defects = table.gate();
    assert!(
        defects.is_empty(),
        "the shipped declaration is refused: {defects}"
    );
    assert_eq!(table.tiers.len(), 2);
    // The count moves in deliberate steps as the content pass lands tranches:
    // 21 at the seed declaration; 33 after the first tranche (the 12 TFC rocks);
    // 42 once TFC's whole classified rock set is in — 21 rocks, corrected from
    // the record's 20; 70 once TFC's ore roster lands as minerals (28 new rows:
    // the three native metals are the metals themselves, and bituminous_coal is
    // the `coal` row).
    assert_eq!(table.substances.len(), 70);
    assert_eq!(table.processes.len(), 11);
    assert_eq!(table.vocabulary.len(), 1);
    // The consumer rule is live: every unconsumed row states its reason, and the
    // reasons are open work rather than defects.
    assert!(
        !table.open_reasons().is_empty(),
        "the open reasons are part of the record"
    );
    // The worked chain's end, exactly: 3295 g in, 1 hatchet @ 2900 g + 395 g out.
    let assemble = table
        .processes
        .iter()
        .find(|process| process.name == "assemble the hatchet")
        .expect("the chain's end is declared");
    assert_eq!(
        table.grams_total(&assemble.inputs),
        table.grams_total(&assemble.outputs)
    );
}

#[test]
fn the_committed_table_is_exactly_the_gated_declaration() {
    assert_eq!(
        generated::table(),
        declared_table(),
        "src/materials/generated.rs disagrees with the declaration; run `cargo run -p \
         voxel-model --bin materials-generator`"
    );
}

#[test]
fn the_digest_record_matches_the_declaration() {
    let files = [
        ("tiers.tsv", read("tiers.tsv")),
        ("substances.tsv", read("substances.tsv")),
        ("processes.tsv", read("processes.tsv")),
        ("vocabulary.tsv", read("vocabulary.tsv")),
    ];
    let refs: Vec<(&str, &str)> = files
        .iter()
        .map(|(name, text)| (*name, text.as_str()))
        .collect();
    let digest = declaration::declaration_digest(&refs);
    let record = read("digest.txt");
    assert!(
        record.contains(&digest.to_hex()),
        "digest.txt does not record the declaration's digest; run the generator\nrecord: {record}"
    );
}
