//! U6's transport, driven the way a client really drives it: **as a separate process**.
//!
//! Derived from `specs/005-u6-record-api/contracts/u6-transport.md`. The binary is the server — its
//! `serve` mode speaks the declared line protocol on stdin/stdout — and every answer read back is
//! checked against a world, resident and record this test built itself, so a served answer is
//! believed because it **corresponds**, never because it crossed a pipe (P4: evidence over
//! assertion). The session covers a request that succeeds, one the world refuses by its owner's
//! name, one that does not parse the table at all, and the same request twice producing one answer.

use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::Duration;

use shploingus_clean_room::bridge::Receipt;
use shploingus_clean_room::digest::Digest;
use shploingus_clean_room::process;
use shploingus_clean_room::reference;
use shploingus_clean_room::transport::{self, Response};

/// A directory this test owns, fresh each run — the tree a served row's references resolve against.
fn fixture(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("shploingus-u6-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the fixture directory is creatable");
    dir
}

/// One part of a receipt's wire form (0 the tag, 1 the request, 2 the answer, 3 the claims, 4 the
/// checks, 5 the effect) — the separator is read from the crate's own declaration, never respelled.
fn part(wire: &str, index: usize) -> String {
    wire.split(shploingus_clean_room::bridge::WIRE_PART)
        .nth(index)
        .expect("the receipt has that part")
        .to_string()
}

/// The same receipt with one part replaced.
fn with_part(wire: &str, index: usize, new: &str) -> String {
    wire.split(shploingus_clean_room::bridge::WIRE_PART)
        .enumerate()
        .map(|(at, piece)| if at == index { new.to_string() } else { piece.to_string() })
        .collect::<Vec<String>>()
        .join(&shploingus_clean_room::bridge::WIRE_PART.to_string())
}

/// A real client: the binary as a child process, its stdin as the request pipe and its stdout as the
/// response stream. Nothing here knows what the answers should be — that is what the tests check.
struct Client {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}

impl Client {
    /// Start the real binary in its `serve` mode and open both pipes.
    fn start() -> Self {
        Self::start_in(Path::new("."))
    }

    /// The same, with the server running **in a tree of its own**: a row's references are relative to
    /// the process's directory, so a client that sends one has to say where the bytes are.
    fn start_in(dir: &Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_shploingus-clean-room"))
            .arg("serve")
            .current_dir(dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("the binary starts in serve mode");
        let input = child.stdin.take().expect("the child's stdin is piped");
        let output = BufReader::new(child.stdout.take().expect("the child's stdout is piped"));
        Client { child, input, output }
    }

    /// Send one request line through the protocol's own framing.
    fn send(&mut self, line: &str) {
        transport::send(&mut self.input, line)
            .unwrap_or_else(|refusal| panic!("`{line}` must cross the protocol: {refusal}"));
    }

    /// Read **the answer to `line`**: the response must be about the line that was sent, or the client
    /// refuses it by name — an answer to a different request is never read as this one's.
    fn read_for(&mut self, line: &str) -> Response {
        transport::read_response_for(&mut self.output, line)
            .unwrap_or_else(|refusal| panic!("the response to `{line}` must be about it: {refusal}"))
            .expect("the server answers every request before it ends")
    }

    /// **Send one request, read its answer, then send the next.** Writing a whole session before
    /// reading any of it blocks as soon as the server's answers fill the pipe buffer, and this client
    /// is the one every later pass inherits.
    fn talk(&mut self, lines: &[&str]) -> Vec<Response> {
        lines
            .iter()
            .map(|line| {
                self.send(line);
                self.read_for(line)
            })
            .collect()
    }

    /// Close the request pipe — the server reads its input to its end — and wait for a clean exit.
    fn finish(self) {
        let Client { mut child, input, output } = self;
        drop(input);
        drop(output);
        let status = child.wait().expect("the child is waited for");
        assert!(status.success(), "the served session must exit cleanly: {status}");
    }
}

/// A successful session: every receipt read off the pipe re-checks against a world this test built,
/// and serves the answer this test derives — correspondence rather than trust.
#[test]
fn a_served_receipt_is_checked_against_this_world_and_not_trusted_for_arriving() {
    let roster = process::roster().expect("the reference roster admits");
    let mut mine = reference::gate(&roster, 1).expect("the reference declaration");
    let mut client = Client::start();

    let session = [
        "truth",
        "fields field=elevation at=0,0,0 side=8",
        "geometry chunk=0,0,0 level=0",
        "held",
        "record",
    ];
    let responses = client.talk(&session);

    let mut derived = Vec::new();
    for (index, line) in session.iter().enumerate() {
        // This process performs the same call first, so its world, resident and record are at the
        // same step the server was when it answered.
        let here = mine.call(line).expect("the same request derives here");
        let Response::Ok { receipt, record } = &responses[index] else {
            panic!("`{line}` must be answered, not refused: {:?}", responses[index]);
        };
        receipt.verify(&mine.owners()).unwrap_or_else(|refusals| {
            panic!("`{line}` must verify against the world it was made in: {refusals}")
        });
        assert_eq!(
            receipt.answer.digest(),
            here.answer.digest(),
            "`{line}`: the served answer is the one derived here"
        );
        assert_eq!(record.events, mine.chain().len(), "`{line}`: the served record length");
        assert_eq!(record.head, mine.chain().head(), "`{line}`: the served record head");
        derived.push(here);
    }

    // The derivation's actual geometry is this process's geometry, value for value — the server read
    // U4's seam, it did not invent a mesh that merely hashes the same way.
    assert_eq!(responses[2].receipt().expect("geometry answered").answer, derived[2].answer);
    assert!(matches!(derived[2].answer.verb(), "geometry"));

    client.finish();
}

/// **An admission is re-established from its request, never believed.** It is the one row that makes no
/// claim, so a receipt for it carries no evidence a claim-check could re-run: its request's references
/// and its requirements, and its answer's axis states, are all carried values. The receipt below is a
/// real one read off the pipe; every edit is made here, and each doctored receipt must refuse **by the
/// gate's own name** rather than verify. Before this behaviour existed every one of these edits
/// verified, so this test is red on the older gate.
#[test]
fn a_doctored_admission_refuses_by_name_while_an_honest_one_verifies() {
    let roster = process::roster().expect("the reference roster admits");
    let mut mine = reference::gate(&roster, 1).expect("the reference declaration");
    let root = fixture("transport-admission");
    // **The door the verifier reads the document through is this process's too**: an admission is
    // re-established by asking the intake again, so the root must be the tree the request names.
    mine.set_intake_root(&root);
    let source: &[u8] = b"# a declared source document\n";
    let check: &[u8] = b"#[test] fn the_bar() {}\n";
    std::fs::write(root.join("source.md"), source).expect("the source reaches the disk");
    std::fs::write(root.join("check.md"), check).expect("the check reaches the disk");
    let stage = roster.stages()[0].name;
    let digest = Digest::of(source).to_hex();
    let check_digest = Digest::of(check).to_hex();
    let line = format!(
        "admit path=source.md digest={digest} feature=a-declared-row licence=permitted \
         claim=checksum_only carrier=U4 check=check.md check_digest={check_digest} \
         requires_stages={stage}"
    );

    // The server runs **in the row's own tree**, so `source.md` means there what it means here.
    let mut client = Client::start_in(&root);
    let responses = client.talk(&[&line]);
    let served = match &responses[0] {
        Response::Ok { receipt, .. } => (**receipt).clone(),
        other => panic!("the admission must be answered, not refused: {other:?}"),
    };

    // The control: the receipt the server actually sent verifies where its document lives, and it is
    // the answer **this** process derives for the same row.
    served
        .verify(&mine.owners())
        .expect("an honest admission verifies against the world its document lives in");
    assert_eq!(
        served.answer.digest(),
        mine.call(&line)
            .expect("the same row derives here")
            .answer
            .digest(),
        "the served admission is the one derived here"
    );

    // Every edit below is to one value the receipt carries, and each one is something the request and
    // the world do not support.
    let wire = served.wire();
    let request = part(&wire, 1);
    let answer = part(&wire, 2);
    let effect = part(&wire, 5);
    let zeros = "0000000000000000000000000000000000000000000000000000000000000000";
    let required = format!("requires_stages={stage}");
    let edit_request = |from: &str, to: &str| with_part(&wire, 1, &request.replace(from, to));
    let cases: [(&str, String, &str); 8] = [
        (
            "a stage the roster does not declare",
            edit_request(&required, "requires_stages=no-such-stage"),
            "stage_undeclared",
        ),
        (
            "a material the registry never admitted",
            edit_request(&required, "requires_materials=4242"),
            "unadmitted_material",
        ),
        (
            "the requirements stripped",
            edit_request(&format!(" {required}"), ""),
            "receipt_mismatch",
        ),
        (
            "the document moved",
            edit_request("path=source.md", "path=gone.md"),
            "source_absent",
        ),
        (
            "the document's digest zeroed",
            edit_request(&format!("digest={digest}"), &format!("digest={zeros}")),
            "source_digest_mismatch",
        ),
        (
            "a carried axis state flipped",
            with_part(&wire, 2, &answer.replace("conformance:active", "conformance:inactive")),
            "receipt_mismatch",
        ),
        (
            "another carried axis state flipped",
            with_part(&wire, 2, &answer.replace("identity:inactive", "identity:active")),
            "receipt_mismatch",
        ),
        (
            "an effect an admission cannot have had",
            with_part(&wire, 5, &effect.replace("events=0:0", "events=2:3")),
            "receipt_mismatch",
        ),
    ];
    for (label, doctored, expected) in cases {
        assert_ne!(doctored, wire, "{label}: the edit has to change the receipt");
        let receipt = Receipt::from_wire(&doctored).unwrap_or_else(|refusal| {
            panic!("{label}: a doctored receipt must still read as one: {refusal}")
        });
        let refusals = match receipt.verify(&mine.owners()) {
            Ok(()) => panic!("{label}: a doctored admission must refuse rather than verify"),
            Err(refusals) => refusals,
        };
        let first = refusals.first_named().expect("a named refusal");
        assert_eq!(first.kind().name(), expected, "{label}: {first}");
    }

    // **A receipt that cannot be re-established refuses rather than passes.** This verifier's door
    // holds an empty tree, so the document the request names is not reachable from it — and that is
    // the seam's own name for it, not a silent pass.
    let mut blind = reference::gate(&roster, 1).expect("a second reference declaration");
    blind.set_intake_root(fixture("transport-elsewhere"));
    let refusals = served
        .verify(&blind.owners())
        .expect_err("a document that is not there cannot be re-established");
    assert_eq!(
        refusals.first_named().expect("a named refusal").kind().name(),
        "source_absent"
    );

    client.finish();
}

/// A world refusal is named by its owner and recorded; a line that does not parse the table is named
/// by the table and writes nothing — and the record read back off the pipe is this process's record.
#[test]
fn a_world_refusal_is_named_and_recorded_while_a_line_that_does_not_parse_writes_nothing() {
    let roster = process::roster().expect("the reference roster admits");
    let mut mine = reference::gate(&roster, 1).expect("the reference declaration");
    let mut client = Client::start();

    let session = [
        "geometry chunk=1,0,0 level=0", // a derivation: one event
        "geometry chunk=9,9,0 level=0", // the world refuses, by its owner's name
        "geometry chunk=0,0 level=0",   // a value that is not a chunk
        "abracadabra",                  // a verb the table does not declare
        "record",                       // what the record holds, read back
    ];
    let responses = client.talk(&session);

    // The refusals keep the vocabulary of the check that decided them: U4's horizon decision for the
    // world, the table's own names for the two lines that never reach a world.
    assert_eq!(responses[1].refusal_kind(), Some("geometry_out_of_residency"));
    assert_eq!(responses[2].refusal_kind(), Some("argument_malformed"));
    assert_eq!(responses[3].refusal_kind(), Some("tool_undeclared"));

    // A world-level refusal is a fact about the world: recorded, exactly once. A line that does not
    // parse writes nothing at all.
    assert_eq!(responses[0].record().events, 1, "the derivation's one event");
    assert_eq!(responses[1].record().events, 2, "the world refusal is recorded");
    assert_eq!(responses[2].record().events, 2, "the argument fault writes nothing");
    assert_eq!(responses[3].record().events, 2, "the undeclared verb writes nothing");

    // This process replays the same four calls: every refusal matches, and the record the client was
    // told is the record this process holds.
    for (index, line) in session[..4].iter().enumerate() {
        let reply = mine.call(line);
        match (&responses[index], reply) {
            (Response::Ok { .. }, Ok(_)) => {}
            (Response::Err { kind, .. }, Err(refusal)) => {
                assert_eq!(kind, refusal.kind().name(), "`{line}`: the reason matches this world's");
            }
            (response, reply) => panic!("`{line}`: served {response:?}, derived {reply:?}"),
        }
    }
    assert_eq!(responses[4].receipt().expect("the record is read").answer.verb(), "record");
    assert_eq!(responses[4].record().events, mine.chain().len(), "the served record length");
    assert_eq!(responses[4].record().head, mine.chain().head(), "the served record head");

    client.finish();
}

/// The same request twice over a pipe is one answer with two effects, and the second is a hit.
#[test]
fn the_same_request_twice_over_a_pipe_is_one_answer_and_the_second_is_a_hit() {
    let roster = process::roster().expect("the reference roster admits");
    let mut mine = reference::gate(&roster, 1).expect("the reference declaration");
    let mut client = Client::start();

    let session = ["geometry chunk=3,0,0 level=0", "geometry chunk=3,0,0 level=0", "held"];
    let responses = client.talk(&session);

    let first = responses[0].receipt().expect("the derivation is answered");
    let second = responses[1].receipt().expect("the repeat is answered");
    assert_eq!(
        first.answer.digest(),
        second.answer.digest(),
        "the same request twice is one answer"
    );
    assert!(first.effect.appended.is_some(), "the derivation appends its event");
    assert_eq!(second.effect.columns_produced, 0, "the hit does no column work");
    assert_eq!(second.effect.hits, 1);
    assert!(second.effect.appended.is_none(), "a hit appends nothing");
    assert_eq!(second.effect.events_after, second.effect.events_before);
    assert_eq!(
        responses[0].record().events,
        responses[1].record().events,
        "the record moved once, for the derivation"
    );

    // Both calls replayed here: the hit's claims verify against this process's own held geometry.
    for line in &session[..2] {
        let _ = mine.call(line);
    }
    second
        .verify(&mine.owners())
        .expect("the hit's claims must hold here, where the same mesh is held");
    let held = responses[2].receipt().expect("held is answered");
    held.verify(&mine.owners()).expect("held_is must hold here");
    assert_eq!(
        held.answer.digest(),
        mine.call("held").expect("held derives here").answer.digest(),
        "the held set the server answered about is the one this process built"
    );

    client.finish();
}

/// The request every line of the long session sends, how many lines there are, and the pipe buffer a
/// client must out-run. A write-everything client blocks only when the session is longer than the pipe
/// buffers on **both** sides — the requests it is still writing and the answers it has not read yet — so
/// both volumes below clear 64 KiB: 12 000 lines is 72 KiB of requests, and the answers are megabytes.
const LONG_REQUEST: &str = "truth";
const LONG_SESSION: usize = 12_000;
const PIPE_BUFFER: usize = 64 * 1024;

/// **A long session must finish or fail, never hang.** This client reads each answer before sending
/// the next request, so the server is never left blocked on a full pipe; a client that wrote all of
/// these first would deadlock, and the bounded wait below turns that deadlock into a named failure
/// instead of a test run that never returns.
#[test]
fn a_long_session_completes_instead_of_blocking_on_the_pipe() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_shploingus-clean-room"))
        .arg("serve")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("the binary starts in serve mode");
    let mut input = child.stdin.take().expect("the child's stdin is piped");
    let mut output = BufReader::new(child.stdout.take().expect("the child's stdout is piped"));

    // Driven on its own thread so the wait can be **bounded**: a pipe deadlock parks this thread for
    // good, and only a timeout tells that apart from a slow run.
    let (sender, waited) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut answered = 0usize;
        let mut bytes = 0usize;
        let mut fault: Option<String> = None;
        for _ in 0..LONG_SESSION {
            if let Err(refusal) = transport::send(&mut input, LONG_REQUEST) {
                fault = Some(format!("`{LONG_REQUEST}` must cross the protocol: {refusal}"));
                break;
            }
            match transport::read_response_for(&mut output, LONG_REQUEST) {
                Ok(Some(Response::Ok { receipt, .. })) => {
                    bytes += receipt.wire().len();
                    answered += 1;
                }
                other => {
                    fault = Some(format!("a read must be answered: {other:?}"));
                    break;
                }
            }
        }
        let _ = sender.send((answered, bytes, fault));
    });

    let (answered, bytes, fault) = match waited.recv_timeout(Duration::from_secs(60)) {
        Ok(counts) => counts,
        Err(_) => {
            let _ = child.kill();
            panic!(
                "the long session blocked: {LONG_SESSION} requests did not finish in 60 s — a client \
                 that writes its whole session before reading deadlocks once the answers fill the pipe"
            );
        }
    };
    if let Some(fault) = fault {
        let _ = child.kill();
        panic!("the long session failed: {fault}");
    }

    // The driver's end closed the request pipe, so the server sees the end of its input and exits.
    let status = child.wait().expect("the child is waited for");
    assert!(status.success(), "the long session must exit cleanly: {status}");
    assert_eq!(answered, LONG_SESSION, "every request was answered");
    assert!(
        bytes > PIPE_BUFFER,
        "the answers ({bytes} bytes) must exceed one pipe buffer, or this test does not exercise the \
         shape it is here for"
    );
    assert!(
        LONG_SESSION * (LONG_REQUEST.len() + 1) > PIPE_BUFFER,
        "the requests ({} bytes) must exceed one pipe buffer too: a client that writes them all first \
         blocks on its own writes once the server stops reading",
        LONG_SESSION * (LONG_REQUEST.len() + 1)
    );
}

/// **A response about a different line is refused by name, with both lines in the message.** The bytes
/// are exactly what a served refusal looks like — same fields, same framing — except that the echo
/// names a request this client never sent. The reader is the client's own, so this is the real
/// surface: an answer to someone else's request must never be read as an answer to this one.
#[test]
fn a_response_whose_echo_names_another_request_is_refused_by_name() {
    let tampered = "status\terr\nrequest\trecord\nrecord\t0:-\nkind\ttool_undeclared\n\
                    detail\tnothing was declared\n\n";
    let mut stream: &[u8] = tampered.as_bytes();
    let refusal = transport::read_response_for(&mut stream, "truth")
        .expect_err("a response about another line is not this line's answer");
    assert_eq!(refusal.kind().name(), "malformed_wire");
    let detail = refusal.detail();
    assert!(detail.contains("truth"), "the expected line is named: {detail}");
    assert!(detail.contains("record"), "the echoed line is named: {detail}");

    // And the same bytes are this client's answer when the client did send `record` — the check
    // refuses mismatches, not responses.
    let mut stream: &[u8] = tampered.as_bytes();
    assert!(transport::read_response_for(&mut stream, "record")
        .expect("the line it names")
        .is_some());
}
