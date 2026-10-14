//! **U10's end-to-end check: the gameplay tree, driven the way a client drives it.**
//!
//! Derived from `specs/007-u10-gameplay-tree/spec.md` (US1–US3 and SC-001…SC-007),
//! `specs/007-u10-gameplay-tree/contracts/u10-seams.md` (C1, C2, C4) and the plan's own acceptance bar
//! (`docs/CLEAN-ROOM-PLAN-2026-10-11.md` §5 row 10: *"every row names its document, its licence verdict
//! and the system that carries it; a refused reference is refused by name; the tree's first row is
//! playable"*). The tree itself is [`shploingus_clean_room::gameplay`]; what is proved here is that it
//! stands against the record it derives from, that its refusals are the owner's own names, and that one of
//! its rows **crosses the served wire** — one `admit` line, one response record, read by a separate
//! process — with the seven axes intact and the answer checked against a world this process built itself
//! rather than trusted because it arrived over a pipe (P4: evidence over assertion).
//!
//! The rows' documents are the workspace's own (`docs/MODS-MAP.md`, `docs/MODS-MATERIALS.md`,
//! `docs/SHAPERS.md`, `specs/004-u4-mesh-stream-work/quickstart.md`), read **in place** where they live:
//! the server is started in the record's root, so a relative path means what a relative path always means.

use std::collections::BTreeSet;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use shploingus_clean_room::bridge::Answer;
use shploingus_clean_room::digest::Digest;
use shploingus_clean_room::gameplay::{
    self, Disposition, Entry, MapRows, Tree, HARVEST_DIGEST, MAP_DIGEST, MAP_PATH, U4_QUICKSTART_PATH,
};
use shploingus_clean_room::intake::{
    Acceptance, Establishment, Intake, Licence, Row, Source, WorldDeclarations,
};
use shploingus_clean_room::process;
use shploingus_clean_room::reference;
use shploingus_clean_room::refusal::RefusalKind;
use shploingus_clean_room::transport::{self, Response};

/// The record's tree, and the world it is conformed against: U3's roster and U2's registry, the same two
/// declarations the binary serves.
fn tree_and_world<'a>(roster: &'a shploingus_clean_room::stage::Declared) -> (Tree, Intake<'a>) {
    let tree = Tree::of_record().expect("the record's own table is a tree");
    let world = WorldDeclarations { roster, registry: reference::registry() };
    let intake = Intake::over(gameplay::record_root(), world);
    (tree, intake)
}

/// **The tree stands against the record it derives from.** Every reference is one entry, the four
/// dispositions are the record's own words, the census sums to the entry count, one row per technology
/// with every citation kept, and the gate's census counts exactly what was and was not established —
/// admitted rows, an owed bar, and a refused verdict each named.
#[test]
fn the_tree_stands_against_the_record_it_derives_from() {
    let roster = process::roster().expect("the reference roster admits");
    let (tree, intake) = tree_and_world(&roster);

    // One entry per reference, in the record's order, §1's load-bearing class first.
    assert_eq!(tree.entries().len(), 106, "the record's own reference set");
    assert_eq!(tree.first().expect("the tree is not empty").reference(), "MOD-c2me");

    // The census of dispositions, with the record's own figures beside it (SC-001, SC-002).
    let census = tree.declared();
    assert_eq!(census.entries(), 106);
    assert_eq!(census.mapped(), 64);
    assert_eq!(census.open(), 25, "the count this build derives");
    assert_eq!(census.refused(), 5);
    assert_eq!(census.unresolved(), 12);
    assert!(census.sums_to_entries(), "{}", census.render(tree.sections()));

    // An owed reference carries **no row**, and is never admitted — a reference with no feature cannot be
    // a row at all, because `feature` is a required argument of the table's `admit`.
    let owed: Vec<&Entry> = tree.entries().iter().filter(|entry| entry.is_owed()).collect();
    assert_eq!(owed.len(), 37);
    for entry in owed {
        assert!(entry.row().is_none(), "{} is owed and carries a row", entry.reference());
    }

    // Every row names its document, its licence verdict and the system that carries it, and every one of
    // them is admitted or refused **by name** — nothing is silent (SC-003).
    let mut admitted = 0;
    let mut refused = 0;
    for entry in tree.entries() {
        let Some(row) = entry.row() else {
            continue;
        };
        assert!(!row.feature().is_empty());
        assert!(row.licence().is_some());
        assert!(row.carrier().is_some());
        assert!([MAP_PATH, gameplay::HARVEST_PATH].contains(&row.source().path()));
        match intake.admit(row) {
            Ok(_) => admitted += 1,
            Err(refusals) => {
                refused += 1;
                assert!(
                    refusals.contains(RefusalKind::LicenceRefused)
                        || refusals.contains(RefusalKind::AxisInactive),
                    "{} refused by something else: {refusals}",
                    entry.reference()
                );
            }
        }
    }
    assert_eq!(admitted + refused, 69, "64 mapped and 5 refused references declare a row");
    assert!(refused > 0, "a refused reference and an owed bar both refuse by name");

    // The gate's census over the tree's rows: **exactly what was and was not established** — eight rows
    // admitted (the four horizon rows on U4's own bar and the four shaper rows on the shapers'), every
    // other row's own bar owed, and three rows whose verdict refuses them.
    let rows: Vec<Row> = tree.rows().iter().map(|technology| technology.row().clone()).collect();
    let gate = intake.census(&rows);
    assert_eq!(gate.rows(), tree.rows().len());
    assert_eq!(gate.rows(), 43, "one row per technology");
    assert_eq!(gate.admitted(), 8, "the rows whose carrying system already declares their bar");
    assert_eq!(gate.count(Establishment::Unchecked), 0, "no axis of this gate is unchecked");
    assert_eq!(gate.refusals().all().iter().filter(|r| r.kind() == RefusalKind::LicenceRefused).count(), 3);
    assert!(!gate.is_established(), "a census that admits every row would be no census");

    // And the head is playable: a §1 world-shape-and-streaming row whose check is U4's own bar.
    let head = tree.first().expect("the tree is not empty");
    let head_row = head.row().expect("the head declares a row");
    assert_eq!(head_row.carrier(), Some("U4"));
    assert_eq!(
        head_row.acceptance(),
        Some(&Acceptance::Provided(Source::new(
            U4_QUICKSTART_PATH,
            Digest::from_hex(gameplay::U4_QUICKSTART_DIGEST).expect("a declared digest")
        )))
    );
    assert!(intake.admit(head_row).is_ok(), "the tree's first row is playable");
}

/// **A refused reference is refused by name, and its permitted sibling admits** (US2, SC-003). The name is
/// the gate's own (`licence_refused`, raised at the licence axis), passed through rather than re-worded —
/// and a refused row carries no cargo: it requires nothing of the world and its own bar is owed.
#[test]
fn a_refused_reference_is_refused_by_name_while_its_permitted_sibling_admits() {
    let roster = process::roster().expect("the reference roster admits");
    let (tree, intake) = tree_and_world(&roster);

    let refused: Vec<&Entry> =
        tree.entries().iter().filter(|e| e.disposition() == Disposition::Refused).collect();
    assert_eq!(refused.len(), 5);
    assert_eq!(
        refused.iter().map(|e| e.reference()).collect::<Vec<_>>(),
        ["BigGlobe", "lithostitched", "Create-The_Factory_Must_Grow", "MFR", "archipelago"]
    );
    for entry in &refused {
        let row = entry.row().expect("a refused reference carries a row whose verdict refuses it");
        assert_eq!(row.licence(), Some(Licence::Refused));
        let inspection = intake.inspect(row);
        assert_eq!(inspection.state(shploingus_clean_room::intake::Axis::Licence), Some(Establishment::Refused));
        let refusals = match intake.admit(row) {
            Ok(_) => panic!("{} must be refused", entry.reference()),
            Err(refusals) => refusals,
        };
        assert_eq!(refusals.first_named().expect("a named refusal").kind(), RefusalKind::LicenceRefused);
        assert_eq!(refusals.first_named().expect("a named refusal").kind().name(), "licence_refused");
        // No cargo: nothing required of the world, and a bar that cannot be run.
        assert_eq!(row.requires_stages(), None);
        assert_eq!(row.requires_materials(), None);
        assert_eq!(row.acceptance(), Some(&Acceptance::Owed));
    }

    // The permitted sibling: the head's row admits, and its identity is *reported* rather than claimed.
    let here = intake.admit(tree.first().expect("the tree is not empty").row().expect("a row"));
    let here = here.expect("a permitted row admits");
    assert!(!here.establishes_identity(), "a checksum establishes integrity and never origin");
}

/// **A document that is absent, and a document whose bytes have moved, each refuse by name** — over a tree
/// of this test's own, with only the bytes moving underneath the tree's own first row (C3, US2's third
/// scenario).
#[test]
fn an_absent_document_and_a_moved_document_refuse_by_name() {
    let root = gameplay::record_root();
    let fixture = std::env::temp_dir().join("shploingus-u10-gameplay-fixture");
    let _ = std::fs::remove_dir_all(&fixture);
    for path in [MAP_PATH, U4_QUICKSTART_PATH] {
        let target = fixture.join(path);
        std::fs::create_dir_all(target.parent().expect("a document has a directory"))
            .expect("the fixture's directories are made");
        std::fs::copy(root.join(path), &target).expect("the document is copied");
    }

    let roster = process::roster().expect("the reference roster admits");
    let intake = Intake::over(
        &fixture,
        WorldDeclarations { roster: &roster, registry: reference::registry() },
    );
    let tree = Tree::of_record().expect("the record's own table is a tree");
    let head = tree.first().expect("the tree is not empty").row().expect("a row").clone();

    // The same bytes under the same reference: admitted, because the row describes this document.
    assert!(intake.admit(&head).is_ok(), "the copy is the document the row declares");

    // One byte moves under it — same path, same declared digest, another document.
    let bytes = std::fs::read(fixture.join(MAP_PATH)).expect("the fixture's map is readable");
    let mut moved = bytes.clone();
    moved.push(b'\n');
    std::fs::write(fixture.join(MAP_PATH), &moved).expect("the fixture's map can be rewritten");
    let refusals = intake.admit(&head).expect_err("the document is no longer the one the row describes");
    assert_eq!(refusals.first_named().expect("a named refusal").kind(), RefusalKind::SourceDigestMismatch);
    let detail = refusals.first_named().expect("a named refusal").detail();
    assert!(detail.contains(&head.source().digest().to_hex()), "the declared digest is named: {detail}");
    assert!(
        detail.contains(&Digest::of(&moved).to_hex()),
        "and the digest the bytes were found at is named: {detail}"
    );

    // A reference that is not there at all: refused with its own path in the message.
    let absent = Row::new(
        head.feature(),
        Source::new("docs/no-such-document.md", head.source().digest()),
        head.licence(),
        head.carrier(),
        head.acceptance().cloned(),
        head.claim().clone(),
        head.requirements().clone(),
    );
    let refusals = intake.admit(&absent).expect_err("a document that is not there cannot be read");
    assert_eq!(refusals.first_named().expect("a named refusal").kind(), RefusalKind::SourceAbsent);
    assert!(
        refusals.first_named().expect("a named refusal").detail().contains("docs/no-such-document.md"),
        "the absent document's path is named"
    );
}

/// **The shape rules, and the closed disposition vocabulary** (T001, T004, FR-002, FR-004). A mapped entry
/// without a row and an owed entry with one are refused by the closest declared name; a word outside the
/// record's four is not a disposition at all.
#[test]
fn the_entry_shape_and_the_disposition_vocabulary_are_closed() {
    let tree = Tree::of_record().expect("the record's own table is a tree");
    let row = tree.technology("chunk-infrastructure").expect("a row").row().clone();

    let absent = Entry::new("mapped-with-no-row", Disposition::Mapped, None, None)
        .expect_err("a mapped entry carries a row");
    assert_eq!(absent.first_named().expect("a refusal").kind(), RefusalKind::ArgumentAbsent);
    assert!(absent.to_string().contains("owed"), "the owed word is in the message: {absent}");

    let malformed = Entry::new("open-with-a-row", Disposition::Open, Some(row), Some("owed"))
        .expect_err("an owed reference cannot be a row");
    assert_eq!(malformed.first_named().expect("a refusal").kind(), RefusalKind::ArgumentMalformed);

    assert_eq!(Disposition::ALL.len(), Disposition::COUNT);
    assert!(Disposition::parse("nearly").is_none());
    assert_eq!(Disposition::parse("unresolved"), Some(Disposition::Unresolved));
}

/// **A tree row crosses the served wire form as a separate process** (FR-011/FR-012, SC-005): one `admit`
/// line, over the line protocol of `specs/005-u6-record-api/contracts/u6-transport.md`, with the seven axes
/// in the declared order — and the served answer is compared with the one this process derives over the
/// **same line and the same world**, rather than trusted because it crossed a pipe.
#[test]
fn a_tree_row_crosses_the_served_wire_form_as_a_separate_process() {
    let root = gameplay::record_root();
    let roster = process::roster().expect("the reference roster admits");
    let tree = Tree::of_record().expect("the record's own table is a tree");
    let head = tree.first().expect("the tree is not empty").row().expect("a row");
    let line = admit_line(head);

    let mut client = Client::start(&root);
    let response = client.ask(&line);
    client.finish();

    let Response::Ok { receipt, record } = &response else {
        panic!("a tree row must be admitted, not refused: {response:?}");
    };
    assert_eq!(receipt.answer.verb(), "admit");
    let Answer::Admitted { feature, claim, found, axes } = &receipt.answer else {
        panic!("the answer is what the row became: {:?}", receipt.answer);
    };
    assert_eq!(feature, head.feature());
    assert_eq!(found, &head.source().digest(), "the document was read in place and hashed");
    assert_eq!(claim.name(), "checksum_only");
    assert!(!claim.establishes_identity(), "a checksum is integrity and never origin");
    assert_eq!(
        *axes,
        [
            Establishment::Active,
            Establishment::Active,
            Establishment::Active,
            Establishment::Active,
            Establishment::Active,
            Establishment::Inactive,
            Establishment::Inactive,
        ],
        "seven axes, in the gate's declared order: identity and conformance are reported, not demanded"
    );
    // The row promises it changes nothing, and the effect says so.
    assert_eq!(receipt.effect.columns_produced, 0);
    assert!(receipt.effect.appended.is_none());
    assert_eq!(record.events, 0);

    // Correspondence: the same line, in this process, over the same world — one answer value.
    let mut mine = reference::gate(&roster, 1).expect("the reference declaration");
    mine.set_intake_root(&root);
    let here = mine.call(&line).expect("the same line admits here");
    assert_eq!(
        receipt.answer.digest(),
        here.answer.digest(),
        "the served answer is the one derived here"
    );

    // And the tree's own row, admitted in this process over the same root, is the same row.
    let intake = Intake::over(
        gameplay::record_root(),
        WorldDeclarations { roster: &roster, registry: reference::registry() },
    );
    assert!(intake.admit(head).is_ok(), "the served row is a row this world admits");

    // **A licence word outside the declared two is refused by name, with the declared two in the
    // message** — never read as nearly-permitted (FR-006, US2's second scenario). The verdicts are
    // closed, so the word is *undeclared* rather than rounded to the nearest of them.
    let mut client = Client::start(&root);
    let nearly = line.replace("licence=permitted", "licence=maybe");
    let response = client.ask(&nearly);
    assert_eq!(response.refusal_kind(), Some("licence_undeclared"), "{response:?}");
    let Response::Err { detail, record, .. } = &response else {
        panic!("a word outside the two verdicts must refuse: {response:?}");
    };
    for verdict in ["permitted", "refused"] {
        assert!(detail.contains(verdict), "the declared `{verdict}` is named: {detail}");
    }
    assert_eq!(record.events, 1, "the line parsed, so the world refusal is recorded once");
    client.finish();
}

/// The `admit` line for a row, in the table's own declared argument order — the two references as
/// `path + digest` each, and the rest as declared words.
fn admit_line(row: &Row) -> String {
    let (check, check_digest) = match row.acceptance() {
        Some(Acceptance::Provided(source)) => (source.path().to_string(), source.digest().to_hex()),
        other => panic!("a served row declares its own check: {other:?}"),
    };
    format!(
        "admit path={} digest={} feature={} licence={} claim={} carrier={} check={} check_digest={}",
        row.source().path(),
        row.source().digest().to_hex(),
        row.feature(),
        row.licence().expect("a row relies on a licence verdict").name(),
        row.claim().name(),
        row.carrier().expect("a row names the system that carries it"),
        check,
        check_digest
    )
}

/// A real client: the binary as a child process, started **in the record's root**, its stdin as the request
/// pipe and its stdout as the response stream.
struct Client {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}

impl Client {
    fn start(root: &Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_shploingus-clean-room"))
            .arg("serve")
            .current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("the binary starts in serve mode");
        let input = child.stdin.take().expect("the child's stdin is piped");
        let output = BufReader::new(child.stdout.take().expect("the child's stdout is piped"));
        Client { child, input, output }
    }

    fn ask(&mut self, line: &str) -> Response {
        transport::send(&mut self.input, line)
            .unwrap_or_else(|refusal| panic!("`{line}` must cross the protocol: {refusal}"));
        transport::read_response_for(&mut self.output, line)
            .unwrap_or_else(|refusal| panic!("the response to `{line}` must be about it: {refusal}"))
            .expect("the server answers every request before it ends")
    }

    fn finish(self) {
        let Client { mut child, input, output } = self;
        drop(input);
        drop(output);
        let status = child.wait().expect("the child is waited for");
        assert!(status.success(), "the served session must exit cleanly: {status}");
    }
}

/// The record's root is the workspace the tree's documents live in — and the digests the rows declare are
/// the ones on the disk, measured here rather than assumed (the fixture above moves a document and watches
/// the refusal that follows).
#[test]
fn the_declared_digests_are_the_documents_on_the_disk() {
    let root: PathBuf = gameplay::record_root();
    for (path, hex) in [(MAP_PATH, MAP_DIGEST), (gameplay::HARVEST_PATH, HARVEST_DIGEST)] {
        let bytes = std::fs::read(root.join(path)).expect("the record's own document");
        assert_eq!(Digest::of(&bytes), Digest::from_hex(hex).expect("a digest"), "{path}");
    }
}

/// **SC-001, measured from the record instead of restated.** The map's §1–§5 are read **in place** and
/// reduced to the references they name — §1's table, §2's group table plus its dated 2026-10-12 block plus
/// the dated 2026-10-03 Tekkit paragraph, §3's and §4's class rows, with §5's list folding into rows
/// already counted — by the module's **one** parse of those tables
/// ([`shploingus_clean_room::gameplay::MapRows`]), which is also where the census's own split is read from:
/// this test re-derives nothing beside it, it checks it. The set is compared with the tree's entries
/// **both ways**, so a row the map names that the tree drops fails, an entry the map does not name fails,
/// and prose that drifts from the tables fails: the header's own stated count and its per-section split are
/// read out of the same bytes, the split is pinned, and the digits the census prints are asserted to be
/// that derived split — so a row moved across sections fails by name rather than being believed.
#[test]
fn the_maps_own_reference_count_is_the_trees_entry_count() {
    let map = std::fs::read_to_string(gameplay::record_root().join(MAP_PATH))
        .expect("the map is the record's own document");
    // The module's one parse of the map's tables: the references they name, the section that first named
    // each one, and §5's list.
    let rows = MapRows::parse(&map);

    // The Tekkit paragraph names plain mods rather than `code` spans, so its names are read out of that
    // paragraph — asserted present **there** before the parse is trusted to have counted them.
    let tekkit = [
        "IC2",
        "BuildCraft",
        "Railcraft",
        "ComputerCraft",
        "ProjectRed",
        "BluePower",
        "EE 3",
        "ForestryMC",
        "MFR",
        "LogisticsPipes",
        "ironchest",
    ];
    for name in tekkit {
        assert!(
            map.contains(&format!("{name} (")),
            "the dated Tekkit paragraph must name `{name}`, with its licence beside it"
        );
    }

    let tree = Tree::of_record().expect("the record's own table is a tree");
    let entries: BTreeSet<String> =
        tree.entries().iter().map(|entry| entry.reference().to_string()).collect();

    let unentried: Vec<&String> = rows.named().difference(&entries).collect();
    let unnamed: Vec<&String> = entries.difference(rows.named()).collect();
    assert!(unentried.is_empty(), "the map names references the tree does not carry: {unentried:?}");
    assert!(unnamed.is_empty(), "the tree carries entries the map does not name: {unnamed:?}");
    assert_eq!(
        rows.named().len(),
        entries.len(),
        "SC-001: one entry per reference the map's own rows name"
    );

    // SC-001's per-section split: the header states it (10 §1 + 42 §2 + 19 §3 + 35 §4), and it is derived
    // from **where each reference is first named** — so a row moved across sections leaves the total and
    // the set alone and still fails by name.
    let split = rows.sections();
    assert_eq!(split, [10, 42, 19, 35], "the map's own per-section split: {split:?}");
    assert_eq!(
        split.iter().sum::<usize>(),
        rows.named().len(),
        "every reference the map names is first named in one of §1–§4, and §5 adds no reference"
    );
    assert!(
        map.contains("10 in §1, 42 in §2, 19 in §3 and 35 in §4"),
        "the map's header must state its own per-section split, not only its total"
    );

    // **The split the census prints is that split.** The tree read the same rows in place when it was
    // built, and the census line is built from what it read: the digits are the derived ones, never a
    // literal kept beside them — so a row moved across sections moves the printed line, and the pin above
    // fails by name.
    assert_eq!(
        tree.sections(),
        split,
        "the tree's published split is the map's own, read from the document in place"
    );
    let census = tree.declared().render(tree.sections());
    let printed = format!("({} §1 + {} §2 + {} §3 + {} §4)", split[0], split[1], split[2], split[3]);
    assert!(census.contains(&printed), "the census must print the derived split `{printed}`: {census}");

    // The header states its own count too: the prose and the rows cannot drift apart unnoticed.
    let stated = format!("The map's own reference count: {} references", entries.len());
    assert!(map.contains(&stated), "the map's header must state its own count: `{stated}`");

    // And §5 is not a second set of references: every name it carries is a row already counted.
    assert!(!rows.section_5().is_empty(), "the map has a §5, and it lists the URLs that had no row");
    for name in rows.section_5() {
        assert!(rows.names(name), "§5's `{name}` is a reference the tree already carries");
    }
}
