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
    // The count moves in deliberate steps as the content pass lands tranches:
    // 21 substances at the seed declaration; 33 after the first tranche (the 12
    // TFC rocks); 42 once TFC's whole classified rock set is in — 21 rocks,
    // corrected from the record's 20; 70 once TFC's ore roster lands as minerals
    // (28 new rows: the three native metals are the metals themselves, and
    // bituminous_coal is the `coal` row); **128 once the technology tree is
    // declared** — the metals, alloys, ceramics, chemicals and workshop products
    // the harvest measures, plus their processes, which is what takes the two
    // seed rungs to the eight-rung ladder; **141 once the molten and solution
    // states are declared** — the seven melts, the three liquors and the
    // leach residue, plus pressed seed oil and its cake; **145 once husbandry,
    // composting and dyeing are declared** — the animals, seed, feed, the
    // compost heap's three grades and the four dye-liquor colours, plus the
    // three processes that make them matter rather than names; **146 once solvent
    // extraction is rowed** — the spent meal, and the one process that washes the
    // press cake with the ethanol the tree already makes.
    assert_eq!(table.tiers.len(), 8);
    assert_eq!(table.substances.len(), 146);
    assert_eq!(table.processes.len(), 79);
    assert_eq!(table.vocabulary.len(), 32);

    // The rows being sound is not the tree being whole. The walk is the proof:
    // every rung reached from the first, every substance obtainable, every
    // process firing — and an edit that orphans any of them fails here.
    let tree = table.tree();
    assert!(
        tree.is_complete(),
        "the shipped tree is not whole: {}",
        tree.report()
    );
    let coverage = tree.coverage();
    assert_eq!(coverage.tiers_reached, coverage.tiers, "{}", tree.report());
    assert_eq!(coverage.substances_available, coverage.substances, "{}", tree.report());
    assert_eq!(coverage.substances_made_produced, coverage.substances_made, "{}", tree.report());
    assert_eq!(coverage.processes_fired, coverage.processes, "{}", tree.report());
    assert_eq!((coverage.tiers, coverage.substances, coverage.processes), (8, 146, 79));
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
