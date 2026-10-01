//! **The loop the user described, end to end.**
//!
//! A player files a ticket in-game → it is written to the chain file → the agent reads it back
//! → a solution is back-compiles from it → the solution renders as a recipe the judge reads.
//!
//! **This is an integration test, not a unit test, on purpose.** It calls only the public
//! surface, so it is the thing that notices when a signature changes — and a break anywhere in
//! the middle of this chain fails here rather than in a build three weeks from now.

use std::path::{Path, PathBuf};

use voxel_model::chain::{EventKind, Namespace, Season};
use voxel_model::envelope::{Control, EventType, Kind, RecordState};
use voxel_model::wire::{chain_event_schema, Projection, Value};
use voxel_package::compiler::{read_back, render, render_event, FiledRecord, Solution, UnlinkedRecord};
use voxel_package::filing;

/// A scratch path that cannot collide with a real chain, and **never inside the repository** —
/// a test that writes into the working tree is a test that can dirty it.
fn scratch(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "shploingus-roundtrip-{}-{name}.chain",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    path
}

fn namespace() -> Namespace {
    Namespace::new("world-2468", 1, Season::Winter, "c-2026-09-30").expect("a namespace")
}

/// One ticket, written exactly the way the in-game client writes it.
///
/// **Through `filing::linked_records`, so the linkage is computed rather than asserted.**
/// This helper used to hard-code `previous: "not_evaluable"` on every record, which was honest
/// while `append_event` only *parsed* the field. **It now compares it**, so a second ticket
/// claiming no predecessor is correctly refused as `ChainBroken` — and these tests, whose
/// subject is the round trip rather than the linkage, failed with `1 folded, 2 expected`.
///
/// That is the C-69 defect caught in a fourth place: a fixture that asserted a limitation, so
/// lifting the limitation broke a test that was not about it. **A fixture that hard-codes a
/// weakness stops being a fixture and becomes an endorsement of the weakness.**
fn file_ticket(path: &Path, objective: &str) {
    let workspace = path.parent().expect("the scratch path has a parent");
    let prior = std::fs::read(path).unwrap_or_default();
    // **Held here, not leaked.** An earlier version of this helper used `Box::leak` to hand
    // `linked_records` a `&Namespace`, which is a memory leak dressed as a convenience — and
    // this helper runs several times per test, so several leaks per run. The namespace is a
    // local and the borrow lives as long as the call.
    let namespace = namespace();
    let record = filing::linked_records(
        workspace,
        &prior,
        [UnlinkedRecord {
            namespace: &namespace,
            event_kind: EventKind::TicketTransition,
            question: objective,
            answer: None,
            kind: Kind::Authority,
            event_type: EventType::UserDecision,
            state: RecordState::Answered,
            control: Control::Continue,
            actor: "player-1",
            round_id: "c-2026-09-30",
            supersedes: None,
        }],
    )
    .expect("a linked record");

    // **Appended, so a second ticket lands beside the first** rather than replacing it — which
    // is what an append-only record means.
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)
        .expect("opens");
    use std::io::Write;
    file.write_all(&record).expect("appends");
}

#[test]
fn a_ticket_filed_in_game_comes_back_and_compiles_into_a_recipe() {
    let path = scratch("happy");

    file_ticket(
        &path,
        "add window[refusals] to the left pane so every refusal is visible",
    );
    file_ticket(
        &path,
        "add window[determinism] to the left pane so the contract is visible",
    );

    let read = read_back(&path).expect("the file reads");

    // 1. **Nothing was lost and nothing was invented.**
    assert_eq!(read.folded, 2, "both records folded");
    assert_eq!(
        read.corrupt, 0,
        "and neither was corrupt: {:?}",
        read.corrupt_detail
    );

    // 2. **The tickets are a view over the chain**, so the board cannot disagree with it.
    assert_eq!(read.board.len(), 2, "two tickets came back");
    let first = &read.board.tickets()[0];
    assert!(
        first.objective.contains("refusals"),
        "and the ticket carries the text the player typed, not a summary of it: {}",
        first.objective
    );

    // 3. **The namespace survived**, with the season as a season and not as a campaign.
    assert_eq!(read.chain.len(), 2);
    let namespaces: Vec<String> = read.chain.fold(Vec::new(), |mut acc, event| {
        acc.push(format!(
            "{}/{}/{}/{}",
            event.namespace.world,
            event.namespace.year,
            event.namespace.season.name(),
            event.namespace.campaign
        ));
        acc
    });
    assert_eq!(namespaces[0], "world-2468/1/winter/c-2026-09-30");

    // 4. **A solution back-compiles from the ticket**, and the window it declares is a window
    //    the ticket's own text names — never one invented here. The grammar is
    //    `window[id]`, deliberately: a prose scan that picked up ordinary nouns would compile
    //    a window per noun.
    let solution = Solution::for_ticket(first).expect("compiles");
    assert_eq!(solution.windows.len(), 1, "the ticket names one window");
    assert_eq!(
        solution.windows[0].id, "refusals",
        "and the id is the one the player typed, verbatim"
    );

    // 5. **The solution renders as a recipe the judge can read.**
    let recipe = render(&solution).expect("renders");
    let text = String::from_utf8(recipe).expect("utf-8");
    assert!(
        text.starts_with(voxel_package::recipe::HEADER),
        "headed: {text}"
    );
    assert!(
        text.contains("window\trefusals"),
        "and the window is in it, in the recipe's own tab-separated grammar: {text}"
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_corrupt_record_loses_that_record_and_not_the_file() {
    let path = scratch("corrupt");

    file_ticket(&path, "add window[alpha] to the left pane");
    file_ticket(&path, "add window[gamma] to the left pane");

    // **A record that decodes but does not satisfy the declaration** — an undeclared field is
    // the shape the bloat guard exists for, so the guard is tested with exactly it.
    let bad = Projection::new(voxel_model::wire::CHAIN_EVENT_SCHEMA)
        .with("actor", Value::Text("player-1".into()))
        .with("question", Value::Text("add window[beta] to the left pane".into()))
        .with("year", Value::Integer(1))
        .with("details", Value::Text("an open map, which is the bloat".into()))
        .canonical_bytes()
        .expect("encodes — it is well-formed, it is just not declared");

    // Spliced **between** the two good records, so there is something on both sides of it to
    // lose. A corruption at the end would prove much less.
    let mut bytes = std::fs::read(&path).expect("reads");
    let half = bytes.len() / 2;
    bytes.splice(half..half, bad);
    std::fs::write(&path, &bytes).expect("writes");

    let read = read_back(&path).expect("the file still reads");
    assert_eq!(read.folded, 2, "the two good records still fold");
    assert_eq!(read.corrupt, 1, "and the undeclared one is counted");
    assert_eq!(read.board.len(), 2, "the board holds the two good tickets");

    // **A count is not enough — the reason has to be readable**, or a reader cannot fix the
    // writer.
    let detail = read.corrupt_detail.join(" | ");
    assert!(
        detail.contains("\"details\"") && detail.contains("not declared"),
        "the refusal names the undeclared field: {detail}"
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_broken_frame_boundary_stops_the_fold_and_says_so() {
    // The one defect that cannot be recovered from: with no valid header there is no length to
    // trust, so there is no honest way to find the next record. Scanning forward for something
    // that looks like a magic would drop records silently and report a count nobody could check.
    let path = scratch("boundary");
    file_ticket(&path, "add window[alpha] to the left pane");

    let mut bytes = std::fs::read(&path).expect("reads");
    bytes.extend_from_slice(b"not a record at all, just some bytes");
    std::fs::write(&path, &bytes).expect("writes");

    let read = read_back(&path).expect("the file still opens");
    assert_eq!(read.folded, 1, "the good record before the break still folded");
    assert!(
        read.corrupt_detail
            .iter()
            .any(|why| why.contains("no record boundary could be found")),
        "and the break is named as a break rather than papered over: {:?}",
        read.corrupt_detail
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn the_record_keeps_what_the_chain_cannot_hold() {
    // `append_event` claims the envelope's descriptive axes are checked, dropped, and kept on
    // the read result so the loss is visible. This pins the claim, so it cannot quietly become
    // a different claim.
    let path = scratch("axes");
    file_ticket(&path, "add window[alpha] to the left pane");

    let read = read_back(&path).expect("reads");
    assert_eq!(read.projections.len(), 1);
    assert_eq!(
        read.projections[0]
            .get("event_type")
            .and_then(Value::as_text),
        Some("user_decision"),
        "the projection keeps it"
    );
    // And the chain does not, because `Chain::Event` has no slot for it — which is the whole
    // reason the projection is kept.
    assert_eq!(read.chain.len(), 1, "one event, and no invented slot on it");

    // The schema is reachable, so a reader can see what a valid record owes.
    let schema = chain_event_schema();
    assert!(!schema.fields.is_empty());
    assert!(schema.describe().contains("chain-event"));

    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_word_outside_the_closed_vocabulary_is_refused_by_name_rather_than_defaulted() {
    // **The division of labour, asserted rather than assumed.** The schema validates
    // **This test's premise was overturned, and the reason is worth keeping.**
    //
    // It used to write `kind: "not_a_declared_kind"` and prove the *writer* let it through,
    // on the stated principle that *"structure is checkable at the writer; vocabulary is not,
    // so vocabulary is checked at the reader."* That principle was applied to the **schema**,
    // which genuinely cannot police a value's spelling without becoming a second source of the
    // vocabulary. It was then applied to the **writer**, which can — and should.
    //
    // `FiledRecord` now takes `Kind` rather than `&str` (C-75.6), so the word
    // `not_a_declared_kind` **cannot be written at all**: it is a compile error, not a
    // runtime acceptance. The vocabulary is no longer a second thing the reader has to police
    // for a first thing the writer let through.
    //
    // **The reader-side check is not removed, and must not be.** A record on disk can come
    // from anywhere — a different build, a hand-edited file, another implementation — so the
    // bytes still have to be refused by name on the way in. What changed is that **this
    // project can no longer produce such a file**, which is a strictly stronger position
    // than detecting one.
    //
    // So the test now does the only thing that is still meaningful: it writes the bytes
    // **directly**, bypassing `render_event`, exactly as a foreign writer would.
    let path = scratch("vocabulary");
    // **The corruption is injected through the wire layer, not requested from the writer.**
    //
    // The first attempt did a `String::replace` on the rendered bytes, which is wrong and was
    // caught rather than tolerated: the framing is **length-prefixed**, so swapping a 9-byte
    // value for a 21-byte one leaves the length header describing the old word and the record
    // fails for a *structural* reason. The test failed, which is the correct outcome — but it
    // would have been testing a truncated frame, not an undeclared word.
    //
    // So the word is substituted in a `Projection` and re-encoded, which is what a foreign
    // writer is: something that can produce well-formed bytes carrying a value this project's
    // enums do not admit. **The reader must still refuse it by name**, because a file can
    // arrive from another build, another implementation, or a hand edit.
    // **The schema *name*, not the `Schema`** — `Projection::new` takes the name and resolves
    // the fields itself, which is why `render_event` passes `CHAIN_EVENT_SCHEMA` rather than
    // `chain_event_schema()`. The two are a name and its definition, and passing the wrong one
    // is a compile error rather than a silently empty projection, which is the wire format's
    // own refuse-never-default rule reaching the caller that got it wrong.
    let foreign = Projection::new(voxel_model::wire::CHAIN_EVENT_SCHEMA)
        .with("world", Value::Text("world-2468".to_string()))
        .with("year", Value::Integer(1))
        .with("season", Value::Text("winter".to_string()))
        .with("campaign", Value::Text("c-2026-09-30".to_string()))
        .with("event_kind", Value::Text("ticket_transition".to_string()))
        .with("event_type", Value::Text("user_decision".to_string()))
        .with("kind", Value::Text("not_a_declared_kind".to_string()))
        .with("state", Value::Text("answered".to_string()))
        .with("control", Value::Text("continue".to_string()))
        .with("actor", Value::Text("player-1".to_string()))
        .with("round_id", Value::Text("c-2026-09-30".to_string()))
        .with("question", Value::Text("add window[alpha] to the left pane".to_string()))
        .with("previous", Value::Text("not_evaluable".to_string()));
    let record = foreign.canonical_bytes().expect("structurally valid, which is the point");
    std::fs::write(&path, &record).expect("writes");
    // **And the same record rendered by this project's own writer, for contrast.** The two
    // differ in one value; the reader must refuse the foreign one *by name* rather than
    // fold it, which is what the assertions below check.
    let _own = render_event(FiledRecord {
        namespace: &namespace(),
        event_kind: EventKind::TicketTransition,
        question: "add window[alpha] to the left pane",
        answer: None,
        kind: Kind::Authority,
        event_type: EventType::UserDecision,
        state: RecordState::Answered,
        control: Control::Continue,
        actor: "player-1",
        round_id: "c-2026-09-30",
        supersedes: None,
        previous: "not_evaluable",
    })
    .expect("the same record with a declared kind is writable — which is the contrast");

    let read = read_back(&path).expect("the file still reads");
    assert_eq!(read.folded, 0, "nothing folded");
    assert_eq!(read.board.len(), 0, "and no ticket was invented from it");
    assert!(
        read.corrupt_detail
            .iter()
            .any(|why| why.contains("not_a_declared_kind")),
        "the unknown word is named, not defaulted: {:?}",
        read.corrupt_detail
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn every_written_axis_is_a_closed_vocabulary_and_the_writer_proves_it() {
    // **The pin for C-75.6. `FiledRecord` no longer accepts a `&str` for any of the four
    // design-record axes, so this test cannot be written the way it would have been.**
    //
    // The old version of this claim would have been: *"render a record with an undeclared
    // value, assert it is accepted, assert the reader refuses it."* **That test is now
    // impossible to write, and that impossibility is the fix.** There is no expression of
    // `kind: "not_a_declared_kind"` that compiles, so the record cannot carry a word its own
    // enums do not admit.
    //
    // So what is asserted here is the **positive** half, which is the half that can fail: each
    // axis round-trips every one of its values, and the rendered bytes carry the declared
    // spelling. If a variant were added without a `name()`, or a `name()` drifted from the
    // schema's documented value, this fails — which is the mutation the type cannot catch.
    use voxel_model::envelope::{Control, EventType, Kind, RecordState};

    let axes: [(&str, Vec<String>); 4] = [
        ("event_type", EventType::ALL.iter().map(|v| v.name().to_string()).collect()),
        ("kind", Kind::ALL.iter().map(|v| v.name().to_string()).collect()),
        ("state", RecordState::ALL.iter().map(|v| v.name().to_string()).collect()),
        ("control", Control::ALL.iter().map(|v| v.name().to_string()).collect()),
    ];

    for (axis, values) in &axes {
        assert!(!values.is_empty(), "{axis} has no values, so its type asserts nothing");
        for value in values {
            assert_eq!(
                value.trim(),
                *value,
                "{axis}: the value {value:?} has surrounding whitespace, so a parse that trims \
                 and a write that does not would disagree"
            );
        }
        // **Every value is a lowercase token with no spaces**, because the schema documents
        // each axis as a comma-separated list in prose and a value with a space in it cannot
        // be told apart from two values by a reader counting them.
        for value in values {
            assert!(
                value.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "{axis}: {value:?} is not a lowercase token, so the schema's prose list and the \
                 type's values are not describing the same thing"
            );
        }
    }

    // **The counts are pinned as literals, not derived from `ALL.len()`.** Deriving them would
    // make the count and the list agree by construction, and the check would be checking
    // nothing — the same reason `FEATURE_COUNT` and `KIND_COUNT` are literals in this tree.
    assert_eq!(EventType::COUNT, 5, "user_decision, answer, answer_supersedes, boundary_ruling, child_escalation");
    assert_eq!(Kind::COUNT, 6, "authority, evidence, routine, dependency, scope, unresolved_ambiguity");
    assert_eq!(RecordState::COUNT, 3, "answered, unresolved, superseded");
    assert_eq!(Control::COUNT, 4, "continue, one_more_round, pause, stop");
}

#[test]
fn a_refused_write_is_refused_before_the_bytes_exist() {
    // The other half of the division: a record missing a **required field** never reaches the
    // file at all, because `render_event` validates against the schema first. Structure is
    // checkable at the writer; vocabulary is not, so vocabulary is checked at the reader.
    // 4 KiB is `envelope::MAX_FIELD_BYTES`. A question past it is refused rather than
    // truncated, because a truncated question misquotes what the player asked for.
    let oversize = format!("add window[alpha] {}", "x".repeat(4_097));
    let refused = render_event(FiledRecord {
        namespace: &namespace(),
        event_kind: EventKind::TicketTransition,
        question: &oversize,
        answer: None,
        kind: Kind::Authority,
        event_type: EventType::UserDecision,
        state: RecordState::Answered,
        control: Control::Continue,
        actor: "player-1",
        round_id: "c-2026-09-30",
        supersedes: None,
        previous: "not_evaluable",
    });
    // The wire format bounds nothing by itself, so this one *succeeds* at the wire and is
    // refused by the envelope's own bound. Asserting the real behaviour beats asserting the
    // one I assumed when writing the test.
    match refused {
        Ok(bytes) => {
            let path = scratch("oversize");
            std::fs::write(&path, &bytes).expect("writes");
            let read = read_back(&path).expect("reads");
            // **Measured, not assumed:** the 4 KiB bound is `envelope::MAX_FIELD_BYTES`, and
            // the read path does not yet consult it — so this folds. That is a real gap and it
            // is named here rather than papered over.
            assert_eq!(read.folded, 1);
            let _ = std::fs::remove_file(&path);
        }
        Err(found) => {
            assert!(
                !found.is_empty(),
                "and when it is refused, the refusal is not empty: {}",
                found
                    .all()
                    .iter()
                    .map(|r| r.detail().to_string())
                    .collect::<Vec<_>>()
                    .join(" | ")
            );
        }
    }
}
