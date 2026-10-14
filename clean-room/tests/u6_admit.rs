//! **U6's content door, driven the way a client really drives it: as a separate process.**
//!
//! Derived from `specs/005-u6-record-api/contracts/u6-transport.md` and the `admit` row of
//! `bridge::TOOLS`. The binary is the server — its `serve` mode speaks the declared line protocol on
//! stdin/stdout — and what is proved here is that a **row arrives by reference** on that surface: the
//! document stays where it lives and is read there, the answer says what the row became, and a
//! reference that is absent, or whose bytes have moved underneath it, refuses **by its own name** with
//! both values in the message.
//!
//! The root a row's references resolve against is the directory the server runs in — a relative path
//! means what a relative path always means — so the tree this test writes *is* the tree the server
//! reads, and the served answer is checked against one derived in this process rather than trusted
//! because it crossed a pipe (P4: evidence over assertion). Each test owns a directory of its own,
//! because the tests in one binary run at the same time.

use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use shploingus_clean_room::bridge::Answer;
use shploingus_clean_room::digest::Digest;
use shploingus_clean_room::intake::Establishment;
use shploingus_clean_room::process;
use shploingus_clean_room::reference;
use shploingus_clean_room::transport::{self, Response};

const SOURCE: &[u8] = b"# a declared source document\n";
const CHECK: &[u8] = b"#[test] fn the_bar() {}\n";

/// A tree of its own: the two documents a row points at, and the digests of the bytes **that are on
/// the disk** — so a row admitted below is admitted because the documents are what it declares, never
/// because a test wrote a digest it controls into a string.
struct Fixture {
    root: PathBuf,
    source: Digest,
    check: Digest,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("shploingus-u6-admit-{name}"));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("the fixture root is made");
        std::fs::write(root.join("source.md"), SOURCE).expect("the source reaches the disk");
        std::fs::write(root.join("check.md"), CHECK).expect("the check reaches the disk");
        Fixture { root, source: Digest::of(SOURCE), check: Digest::of(CHECK) }
    }

    /// The row, in the table's own declared argument order, pointing at `path` with `digest` — so the
    /// tamper case can hold the **reference** still and move the **bytes**.
    fn row(&self, path: &str, digest: Digest) -> String {
        format!(
            "admit path={path} digest={} feature=a-declared-row licence=permitted claim=checksum_only \
             carrier=U4 check=check.md check_digest={}",
            digest.to_hex(),
            self.check.to_hex()
        )
    }

    /// Move the source's bytes underneath the reference it was admitted under.
    fn tamper(&self) {
        std::fs::write(self.root.join("source.md"), b"# a declared source document!\n")
            .expect("the source can be rewritten");
    }
}

/// A real client: the binary as a child process, started **in the fixture's directory**, its stdin as
/// the request pipe and its stdout as the response stream.
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

    /// Send one request line, then read **the answer to it**: the response must be about the line that
    /// was sent, so an answer to another request is never read as this one's.
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

/// **A row arrives by reference and the client sees what it became.** The served answer is the one
/// this process derives over the same tree, and it says: the document was read where it lives, its
/// bytes hashed to the digest the row declared, every declared axis was established — and no identity
/// was proven, because a checksum is integrity and not origin.
#[test]
fn a_served_row_arrives_by_reference_and_says_what_it_became() {
    let fixture = Fixture::new("admitted");
    let row = fixture.row("source.md", fixture.source);
    let mut client = Client::start(&fixture.root);
    let response = client.ask(&row);
    client.finish();

    let Response::Ok { receipt, record } = &response else {
        panic!("the row must be admitted, not refused: {response:?}");
    };
    assert_eq!(receipt.answer.verb(), "admit");
    let Answer::Admitted { feature, claim, found, axes } = &receipt.answer else {
        panic!("the answer is what the row became: {:?}", receipt.answer);
    };
    assert_eq!(feature, "a-declared-row");
    assert_eq!(found, &fixture.source, "the bytes were read in place and hashed");
    assert_eq!(claim.name(), "checksum_only", "the claim the row declared");
    assert!(
        !claim.establishes_identity(),
        "a checksum establishes integrity and never origin"
    );
    assert_eq!(
        *axes,
        [
            Establishment::Active,
            Establishment::Active,
            Establishment::Active,
            Establishment::Active,
            Establishment::Active,
            // The identity axis is *reported*, not demanded: checked, and found absent — which is a
            // fact about the claim rather than a defect in the row.
            Establishment::Inactive,
            // And so is conformance: this row requires nothing of the world, so nothing was asked and
            // nothing is missing. An axis nothing was asked of is still never read as a pass.
            Establishment::Inactive,
        ],
        "every axis is answered, in the gate's declared order"
    );
    // The row promises it changes nothing, and the effect says so: no work, no append.
    assert_eq!(receipt.effect.columns_produced, 0);
    assert!(receipt.effect.appended.is_none());
    assert_eq!(receipt.effect.events_after, receipt.effect.events_before);
    assert_eq!(record.events, 0, "a served session opens an empty record");

    // Correspondence: this process admits the same row over the same tree, and the answer is the same
    // value — the server read a document where it lives, it did not invent a verdict that happens to
    // have the same shape.
    let roster = process::roster().expect("the reference roster admits");
    let mut mine = reference::gate(&roster, 1).expect("the reference declaration");
    mine.set_intake_root(&fixture.root);
    let here = mine.call(&row).expect("the same row admits here");
    assert_eq!(
        receipt.answer.digest(),
        here.answer.digest(),
        "the served answer digest is the one derived here"
    );
}

/// **An absent document and a moved document refuse by their own names, with both values in the
/// message** — and because both lines parsed, each refusal is recorded, exactly once.
#[test]
fn an_absent_and_a_moved_document_refuse_by_name_over_the_pipe() {
    let fixture = Fixture::new("refused");
    let mut client = Client::start(&fixture.root);

    let absent = client.ask(&fixture.row("gone.md", fixture.source));
    let Response::Err { kind, detail, record, .. } = &absent else {
        panic!("a reference to a document that is not there must refuse: {absent:?}");
    };
    assert_eq!(kind, "source_absent", "the intake's own name for an absent document");
    assert!(
        detail.contains("gone.md"),
        "the absent document's path is named: {detail}"
    );
    assert_eq!(record.events, 1, "a parsed line's refusal is a fact about the world");

    // The bytes move underneath the same reference: same path, same declared digest, another document.
    fixture.tamper();
    let moved = client.ask(&fixture.row("source.md", fixture.source));
    let Response::Err { kind, detail, record, .. } = &moved else {
        panic!("a reference whose document moved must refuse: {moved:?}");
    };
    assert_eq!(kind, "source_digest_mismatch");
    let moved_digest = Digest::of(b"# a declared source document!\n").to_hex();
    assert!(
        detail.contains(&fixture.source.to_hex()),
        "the digest the row declared is in the message: {detail}"
    );
    assert!(
        detail.contains(&moved_digest),
        "and the digest the bytes were found at is too: {detail}"
    );
    assert_eq!(record.events, 2, "the second refusal is recorded too");
    client.finish();
}

/// **A line that does not parse writes nothing.** A reference that leaves the gate's root, or a digest
/// that is not one, is refused by the table's own name before any world is consulted — the split
/// between a parse fault and a world refusal, held on the row that reads a disk.
#[test]
fn a_reference_the_table_does_not_declare_writes_nothing() {
    let fixture = Fixture::new("malformed");
    let mut client = Client::start(&fixture.root);

    let escaped = format!("admit path=../escape digest={} feature=a-declared-row \
                           licence=permitted claim=checksum_only carrier=U4 check=check.md \
                           check_digest={}",
        fixture.source.to_hex(),
        fixture.check.to_hex());
    let response = client.ask(&escaped);
    assert_eq!(
        response.refusal_kind(),
        Some("argument_malformed"),
        "a path that climbs out of the root is not a reference: {response:?}"
    );

    let not_a_digest = fixture.row("source.md", fixture.source).replace(
        &fixture.source.to_hex(),
        "not-a-digest",
    );
    let response = client.ask(&not_a_digest);
    assert_eq!(response.refusal_kind(), Some("argument_malformed"));


    // Neither line reached a world, so neither left a mark on the record.
    let record = client.ask("record");
    assert_eq!(record.record().events, 0, "a parse fault writes nothing");
    client.finish();
}

/// **A served row is conformed against the world the server itself declared.** The last axis asks
/// whether the world holds what the row requires — U3's roster for a stage name, U2's registry for a
/// material id — and every answer is a refusal **by name with the missing item**, never a bare
/// negative: `stage_undeclared` with the stage, the registry's own words for a material, and the
/// table's own shape rule for a requirement that names nothing (a parse fault, which writes nothing at
/// all). Nothing here consults a list the transport keeps: the server's own two declarations decide.
#[test]
fn a_served_row_is_conformed_against_the_worlds_own_declarations() {
    use shploingus_clean_room::intake::Axis;

    let fixture = Fixture::new("conformance");
    let roster = process::roster().expect("the reference roster admits");
    let a_stage = roster.stages()[0].name;
    let row = fixture.row("source.md", fixture.source);
    let mut client = Client::start(&fixture.root);

    // A row that requires a stage the served roster runs and a material the served registry admits:
    // the axis is **established**, and the answer carries it in the gate's declared position.
    let conforming = format!("{row} requires_stages={a_stage} requires_materials=1");
    let receipt = client
        .ask(&conforming)
        .receipt()
        .expect("the row conforms")
        .clone();
    let Answer::Admitted { axes, .. } = &receipt.answer else {
        panic!("the answer is what the row became: {:?}", receipt.answer);
    };
    let conformance = Axis::ALL
        .iter()
        .position(|axis| *axis == Axis::Conformance)
        .expect("the gate declares the axis");
    assert_eq!(
        axes[conformance],
        Establishment::Active,
        "the served roster runs {a_stage} and the served registry admits material 1"
    );

    // A stage the served roster does not run: refused by name, **with the stage in the message**.
    let missing_stage = format!("{row} requires_stages=no-such-stage");
    let refused = client.ask(&missing_stage);
    assert_eq!(refused.refusal_kind(), Some("stage_undeclared"), "{refused:?}");
    let Response::Err { detail, record, .. } = &refused else {
        panic!("a requirement the world does not hold refuses: {refused:?}");
    };
    assert!(
        detail.contains("no-such-stage"),
        "the missing stage is named: {detail}"
    );
    assert_eq!(
        record.events, 1,
        "the line parsed and the content tree is part of the world, so the refusal is recorded"
    );

    // A material the served registry never admitted: the registry's **own** refusal, passed through.
    let missing_material = format!("{row} requires_materials=4242");
    let refused = client.ask(&missing_material);
    assert_eq!(refused.refusal_kind(), Some("unadmitted_material"), "{refused:?}");
    let Response::Err { detail, .. } = &refused else {
        panic!("a material the registry does not admit refuses: {refused:?}");
    };
    assert!(detail.contains("4242"), "the id is named: {detail}");

    // A requirement that names nothing is refused by the **table's shape rule** — nothing is not a
    // value of a declared kind — so it never reaches an axis and writes nothing to the record.
    let unnamed = format!("{row} requires_stages=");
    let refused = client.ask(&unnamed);
    assert_eq!(refused.refusal_kind(), Some("argument_malformed"), "{refused:?}");

    let record = client.ask("record");
    assert_eq!(
        record.record().events,
        2,
        "the two world-level refusals are recorded and the parse fault is not"
    );
    client.finish();
}

/// **One row twice is one answer**, and the second effect differs in nothing but the ledger — the row
/// promises it may change the record, and the record shows it did not.
#[test]
fn one_row_twice_is_one_answer_and_the_record_does_not_move() {
    let fixture = Fixture::new("repeat");
    let row = fixture.row("source.md", fixture.source);
    let mut client = Client::start(&fixture.root);

    let first = client.ask(&row);
    let second = client.ask(&row);
    let record = client.ask("record");
    client.finish();

    let first = first.receipt().expect("the row is admitted");
    let second = second.receipt().expect("and again");
    assert_eq!(
        first.answer_digest(),
        second.answer_digest(),
        "the same row twice is one answer"
    );
    assert!(second.effect.appended.is_none(), "an admission appends nothing");
    assert_eq!(second.effect.events_after, second.effect.events_before);
    assert_eq!(
        record.record().events,
        0,
        "and the record never moved for a row that changes nothing"
    );
}
