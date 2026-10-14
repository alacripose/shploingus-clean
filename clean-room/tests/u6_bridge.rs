//! **U6's end-to-end check, driven the way a caller drives it.**
//!
//! Derived from `specs/005-u6-record-api/spec.md`'s three user stories and from
//! `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §2 U6 (*"a tool call's `Receipt` naming the checks that
//! ran"*). Every test here goes through the crate's **public surface** — `bridge::parse`,
//! `Gate::call`, `Receipt::verify`, `Owners`, `Chain`, `World` — on the reference declaration
//! ([`process::roster`]), so what is proved is what a caller can do.

use shploingus_clean_room::bridge::{
    Answer, ArgKind, CheckName, CheckOutcome, Claim, Gate, Owners, Receipt, TOOLS, TOOL_COUNT, Verb,
    WIRE_PART,
};
use shploingus_clean_room::digest::Digest;
use shploingus_clean_room::chain::{Chain, EventKind, Namespace};
use shploingus_clean_room::mesh::Budget;
use shploingus_clean_room::process;
use shploingus_clean_room::reference;
use shploingus_clean_room::refusal::RefusalKind;
use shploingus_clean_room::stage::Declared;
use shploingus_clean_room::world::Tick;

/// A gate over the **reference world** — the seed and period the plan records, a residency horizon wide
/// enough to keep the chunks a test asks for, and the palette that maps every declared lithology class
/// onto an admitted material — built through the one place that declaration is written down. A gate
/// assembled here instead would be a second, similar world, and a test that proved something about it
/// would prove nothing about the world the binary serves.
fn gate_over<'a>(roster: &'a Declared, workers: usize) -> Gate<'a> {
    reference::gate(roster, workers).expect("the reference declaration is admitted")
}

/// The gate the reference world's own check uses, with the world's opening on the record — appended
/// through U2's writer, because the record is the world's memory and its genesis is a world event.
fn opened_gate(roster: &Declared) -> Gate<'_> {
    let mut gate = gate_over(roster, 1);
    gate.chain_mut()
        .append(Namespace::World, EventKind::WorldOpened, "seed=1592639215 period=1s")
        .expect("a wirable payload");
    gate
}

/// **A declared row, in the wire form**, over a fixture this test writes itself: the two documents the
/// row points at **by reference**, and the line that points at them. The digests are computed from the
/// bytes that are on the disk, so the row is admitted because the documents **are** what it declares
/// rather than because a test wrote a digest it controls into a string.
fn admission(root: &std::path::Path) -> String {
    let _ = std::fs::remove_dir_all(root);
    std::fs::create_dir_all(root).expect("the fixture root is made");
    let source = b"# a declared source document\n";
    let check = b"#[test] fn the_bar() {}\n";
    std::fs::write(root.join("source.md"), source).expect("the fixture reaches the disk");
    std::fs::write(root.join("check.md"), check).expect("the fixture reaches the disk");
    format!(
        "admit path=source.md digest={} feature=a-declared-row licence=permitted claim=checksum_only \
         carrier=U4 check=check.md check_digest={}",
        Digest::of(source).to_hex(),
        Digest::of(check).to_hex()
    )
}

/// Exercise **one** declared row: call it, hold its receipt to the row's own declaration (every check
/// the row declares ran and passed, no claim outside the row's list, a claim where the row declares
/// `claims_hold`), and verify it against the owners. Returns the receipt, and records the line in
/// `called` — so *every row of the table was reached* is a value the caller can assert rather than a
/// list someone wrote twice.
fn exercise_row(
    gate: &mut Gate<'_>,
    line: &str,
    verb: &'static str,
    called: &mut Vec<(String, &'static str)>,
) -> Receipt {
    let receipt = gate
        .call(line)
        .unwrap_or_else(|refusal| panic!("`{line}` was refused by name: {refusal}"));
    assert_eq!(receipt.answer.verb(), verb, "{line}");
    assert_eq!(receipt.request, line, "the canonical line is the request");
    let row = shploingus_clean_room::bridge::tool(line.split(' ').next().expect("a verb"))
        .expect("a declared verb");
    assert_eq!(
        receipt.checks.len(),
        row.checks.len(),
        "{line} must record every check its row declares"
    );
    for check in &receipt.checks {
        assert!(
            matches!(check.outcome, CheckOutcome::Passed),
            "{line}: {} did not pass: {}",
            check.name.name(),
            check.subject
        );
    }
    // A row that declares `claims_hold` must make a claim — otherwise that check would be about
    // nothing. **Three** rows declare none, and say so by making none: an admission's evidence is the
    // digest the seam read, and a save document's is the bytes at the reference — a file is not one of
    // the owners this gate's claims are re-checked against, so those rows make no claim and are
    // re-established instead.
    assert_eq!(
        !receipt.claims.is_empty(),
        row.checks.contains(&CheckName::ClaimsHold),
        "{line}: claims are a row's declaration, not its habit"
    );
    // The declaration is not fiction, in either direction: every claim a receipt carries is one its row
    // declares, and every claim the row declares is claimed by a call that earned it (the derivation
    // below appends, so it earns `event_is` too).
    for claim in &receipt.claims {
        assert!(
            row.claims.contains(&claim.name()),
            "{line}: `{}` is not one of the claims `{}` declares: {:?}",
            claim.name(),
            row.name,
            row.claims
        );
    }
    for declared in row.claims {
        assert!(
            receipt.claims.iter().any(|claim| claim.name() == *declared),
            "{line}: `{declared}` is declared by `{}` and never claimed",
            row.name
        );
    }
    receipt
        .verify(&gate.owners())
        .unwrap_or_else(|refusals| panic!("{line} must verify: {refusals}"));
    assert!(
        receipt.effect.refusals == 0,
        "{line} refused something it should have answered"
    );
    called.push((line.to_string(), verb));
    receipt
}

/// Every declared verb is served, every answer verifies, and every check the row declares passed:
/// the table is the surface, and nothing off it is reachable.
#[test]
fn every_declared_verb_answers_and_every_answer_verifies() {
    let roster = process::roster().expect("the reference roster must admit");
    let mut gate = opened_gate(&roster);
    let root = std::env::temp_dir().join("shploingus-u6-bridge-admit");
    let admission = admission(&root);
    gate.set_intake_root(&root);

    let mut called: Vec<(String, &'static str)> = Vec::new();
    for (line, verb) in [
        ("truth", "truth"),
        ("fields field=elevation at=0,0,0 side=8", "fields"),
        ("geometry chunk=0,0,0 level=0", "geometry"),
        ("held", "held"),
        ("record", "record"),
        (admission.as_str(), "admit"),
    ] {
        exercise_row(&mut gate, line, verb, &mut called);
    }

    // **U9's document form, through this table's own two rows** (2026-10-12, §28): a world written down
    // at the gate's own root, and that document read back **by reference**. The `load` line is handed
    // the digest the `save` line's own answer carries — the row reports what it placed, rather than a
    // test writing a digest it controls into a string. The two lines are adjacent on purpose: a `save`
    // writes, so a document read against an older digest would be a *moved* document, which is the
    // refusal the document form has a name for and a different test's subject.
    let saved = exercise_row(&mut gate, "save path=a-world.save", "save", &mut called);
    let document = match &saved.answer {
        Answer::Saved(facts) => facts.digest,
        other => panic!("a write answers with the document it placed, not {other:?}"),
    };
    let loaded = exercise_row(
        &mut gate,
        &format!("load path=a-world.save digest={}", document.to_hex()),
        "load",
        &mut called,
    );

    assert_eq!(called.len(), TOOL_COUNT, "every row of the table is exercised");
    // The rows the table declares are exactly the ones called, in both directions.
    for tool in &TOOLS {
        assert!(
            called.iter().any(|(line, _)| line.split(' ').next() == Some(tool.name)),
            "{} is declared and never called",
            tool.name
        );
    }

    // **And the document form is what those two rows say it is.** The world written down is the world
    // this gate serves (the identity and the record agree), and the document read back is that same
    // world — the saved and loaded facts are one value, which is what makes the round trip a measured
    // thing rather than a written-down one.
    let (saved_facts, loaded_facts) = match (&saved.answer, &loaded.answer) {
        (Answer::Saved(saved_facts), Answer::Loaded(loaded_facts)) => (saved_facts, loaded_facts),
        other => panic!("the two save rows answer with their own documents, not {other:?}"),
    };
    assert_eq!(saved_facts, loaded_facts, "the document reads back as itself");
    assert_eq!(saved_facts.seed, gate.world().identity().seed);
    assert_eq!(saved_facts.period_s, gate.world().identity().period.get());
    assert_eq!(saved_facts.tick, gate.world().tick().0);
    assert_eq!(saved_facts.ruleset, gate.ruleset());
    assert_eq!(saved_facts.truth, gate.world().truth_digest());
    assert_eq!(
        saved_facts.len,
        std::fs::metadata(root.join("a-world.save"))
            .expect("the document is on the disk")
            .len() as usize,
        "the length a save reports is the document's own"
    );
    // The record the document carries is the record this gate had **when it was written** — and the
    // write and the read are not events about the world, so the gate's record is still that record
    // here: one derivation earlier, and nothing since.
    assert_eq!(
        saved_facts.record_len,
        gate.chain().len(),
        "a write is not an event about the world"
    );
    assert_eq!(
        saved_facts.record_head,
        gate.chain().head(),
        "the document carries the record's head"
    );
    assert_eq!(
        saved_facts.record_head,
        Some(gate.chain().events()[saved_facts.record_len - 1].digest()),
        "and that head is the event at the document's own length"
    );

    // A declared geometry call is the only one that changed the record: three reads appended
    // nothing, and the derivation appended exactly one event.
    let derived = gate
        .chain()
        .events()
        .iter()
        .filter(|event| event.kind() == EventKind::GeometryDerived)
        .count();
    assert_eq!(derived, 1, "one derivation, one event");
    assert_eq!(gate.chain().len(), 2, "the world's opening and the derivation");
    gate.chain().verify().expect("the record still verifies");
}

/// One request twice is one answer; the two **effects** differ, and the record shows what changed.
#[test]
fn one_request_twice_is_one_answer_and_the_second_changes_nothing() {
    let roster = process::roster().expect("the reference roster must admit");
    let mut gate = opened_gate(&roster);

    let first = gate.call("geometry chunk=0,0,0 level=0").expect("a derivation");
    assert!(first.effect.did_work(), "the first call derives");
    assert_eq!(first.effect.hits, 0);
    assert_eq!(first.effect.misses, 1);
    assert!(first.effect.changed_the_record());
    let appended = first.effect.appended.expect("a derivation is recorded");
    assert_eq!(first.effect.events_after, first.effect.events_before + 1);
    // The receipt claims the event it appended, and that claim is checked against the chain.
    assert!(first
        .claims
        .iter()
        .any(|claim| claim.name() == "event_is" && claim.describe().contains(&appended.to_hex())));

    let second = gate.call("geometry chunk=0,0,0 level=0").expect("a hit");
    assert_eq!(
        first.answer_digest(),
        second.answer_digest(),
        "one request twice is one answer"
    );
    assert_eq!(second.effect.columns_produced, 0, "a hit does no work");
    assert_eq!(second.effect.hits, 1);
    assert_eq!(second.effect.misses, 0);
    assert!(second.effect.appended.is_none(), "a hit appends nothing");
    assert!(!second.effect.changed_the_record());
    assert_eq!(
        second.effect.events_after, first.effect.events_after,
        "the record moved once, for the change"
    );
    assert_eq!(second.effect.held_after, first.effect.held_after);
    assert!(first.effect.render().contains("appended "));
    assert!(second.effect.render().contains("appended nothing"));
    assert!(
        !second.claims.iter().any(|claim| claim.name() == "event_is"),
        "a hit makes no claim about the record's length"
    );

    // The answer is what the answer describes, not what produced it: the same request through gates
    // with different worker counts is one answer digest (U3's knob is a knob).
    for workers in [2_usize, 4, 8] {
        let mut other = gate_over(&roster, workers);
        let receipt = other.call("geometry chunk=0,0,0 level=0").expect("a derivation");
        assert!(receipt.effect.did_work());
        assert_eq!(
            receipt.answer_digest(),
            first.answer_digest(),
            "the worker count moved the answer at {workers} workers"
        );
    }
}

/// A request that does not parse writes nothing; a request the world cannot honour is refused by the
/// owner's own name **and recorded**, because that refusal is a fact about the world.
#[test]
fn a_parse_fault_writes_nothing_and_a_world_refusal_is_recorded_by_name() {
    let roster = process::roster().expect("the reference roster must admit");
    let mut gate = opened_gate(&roster);
    let before = gate.chain().len();

    // A line that names no declared verb, and one that names an argument no row declares.
    for line in ["abracadabra", "truth field=elevation", "geometry chunk=0,0 level=0"] {
        let refused = gate.call(line).expect_err(line);
        assert!(
            matches!(
                refused.kind(),
                RefusalKind::ToolUndeclared
                    | RefusalKind::ArgumentUndeclared
                    | RefusalKind::ArgumentMalformed
            ),
            "{line} was refused as {}",
            refused.kind().name()
        );
        assert_eq!(
            gate.chain().len(),
            before,
            "{line}: a line that does not parse is not an event about the world"
        );
    }

    // A chunk outside the declared residency horizon: U4's own name, passed through, and recorded.
    let refused = gate
        .call("geometry chunk=9,9,0 level=0")
        .expect_err("nine chunks away is outside the horizon");
    assert_eq!(refused.kind(), RefusalKind::GeometryOutOfResidency);
    assert_eq!(refused.kind().name(), "geometry_out_of_residency");
    assert_eq!(gate.chain().len(), before + 1);
    let recorded = gate.chain().events().last().expect("the recorded refusal");
    assert_eq!(recorded.kind(), EventKind::RefusalRecorded);
    assert_eq!(recorded.namespace(), Namespace::Record);
    assert!(recorded.payload().contains("geometry_out_of_residency"), "{}", recorded.payload());
    assert!(recorded.payload().contains("chunk=9,9,0"), "{}", recorded.payload());
    // The record carries the **decision** — the canonical request and the refusal's name — and not the
    // refusal's detail: a detail is prose for the caller, and the chain hashes the payload into the
    // record's own identity, so this host's wording must stay out of it.
    assert!(
        !recorded.payload().contains("chunk (9, 9, 0)"),
        "the record carries the request, not the explanation: {}",
        recorded.payload()
    );
    assert_eq!(gate.resident().len(), 0, "nothing was cached");

    // A level that is not on U4's ladder: also the owner's name, also recorded, also cached nothing.
    let refused = gate
        .call("geometry chunk=0,0,0 level=9")
        .expect_err("level 9 is off the ladder");
    assert_eq!(refused.kind(), RefusalKind::LevelOffLadder);
    assert_eq!(gate.chain().len(), before + 2);
    assert_eq!(gate.resident().len(), 0);

    // A budget that cannot be met: U4 refuses by name with its numbers, and the gate passes it on. The
    // request declares 1156 columns — the chunk's own 1024 plus the halo the reading reads.
    let mut narrow = opened_gate(&roster);
    narrow.set_budget(Budget::new(64, 64));
    let refused = narrow
        .call("geometry chunk=0,0,0 level=0")
        .expect_err("1156 columns cannot fit in 64");
    assert_eq!(refused.kind(), RefusalKind::GeometryBudgetExceeded);
    assert!(refused.detail().contains("1156"), "{refused}");
    assert_eq!(narrow.resident().len(), 0, "nothing cached");
    assert_eq!(narrow.chain().len(), 2, "the opening and the recorded refusal");

    // A field name no declared field declares: a fact about the world's vocabulary, so recorded.
    let refused = gate
        .call("fields field=nope at=0,0,0 side=8")
        .expect_err("nope is not a declared field");
    assert_eq!(refused.kind(), RefusalKind::FieldUndeclared);
    assert_eq!(gate.chain().len(), before + 3);

    gate.chain().verify().expect("every refusal left the record intact");
}

/// Every declared argument is required: dropping one is refused `argument_absent`, for every row,
/// straight from the declaration.
#[test]
fn every_declared_argument_is_required_by_the_table_unless_it_is_declared_optional() {
    let roster = process::roster().expect("the reference roster must admit");
    let a_stage = roster.stages()[0].name;
    let mut gate = opened_gate(&roster);
    // The `admit` row's `path`/`digest` arguments are only *values* if the documents they point at
    // exist: so this test writes them, pins the gate's root where they live, and gives the row each
    // digest's own bytes rather than a placeholder that happens to look like a digest.
    let root = std::env::temp_dir().join("shploingus-u6-bridge-args");
    let admission = admission(&root);
    gate.set_intake_root(&root);
    let source_digest = admission
        .split("digest=")
        .nth(1)
        .and_then(|rest| rest.split(' ').next())
        .expect("the fixture line carries the source's digest")
        .to_string();
    let check_digest = admission
        .split("check_digest=")
        .nth(1)
        .expect("the fixture line carries the check's digest")
        .to_string();
    let mut checked = 0;
    // **The two save rows are given their own document.** A `save` writes one, and the `load` that
    // follows is handed **that document's own digest** — read off the save row's answer rather than
    // declared here, so the line points at bytes that are on the disk and are the ones it names. A
    // load of some other file would be rightly refused (`source_digest_mismatch`), which is a
    // different test's subject and not a row that cannot answer.
    let mut document_digest = String::new();

    for tool in &TOOLS {
        let full: Vec<String> = tool
            .args
            .iter()
            .map(|argument| {
                let value: &str = match argument.kind {
                    ArgKind::Chunk | ArgKind::Voxel => "0,0,0",
                    ArgKind::Level => "0",
                    ArgKind::Field => "elevation",
                    ArgKind::Side => "8",
                    // The two save rows have a document of their own: the `save` row writes it, and
                    // the `load` row is handed that document's own digest.
                    ArgKind::Path if tool.verb == Verb::Save || tool.verb == Verb::Load => "a-world.save",
                    ArgKind::Path if argument.name == "path" => "source.md",
                    ArgKind::Path => "check.md",
                    ArgKind::Digest if tool.verb == Verb::Load => document_digest.as_str(),
                    ArgKind::Digest if argument.name == "digest" => source_digest.as_str(),
                    ArgKind::Digest => check_digest.as_str(),
                    ArgKind::Name if argument.name == "feature" => "a-declared-row",
                    ArgKind::Name if argument.name == "licence" => "permitted",
                    ArgKind::Name if argument.name == "claim" => "checksum_only",
                    ArgKind::Name => "U4",
                    // A list is one value: the stage the reference roster runs, and a material the
                    // reference registry admits.
                    ArgKind::Names if argument.name == "requires_stages" => a_stage,
                    ArgKind::Names => "1",
                };
                format!("{}={}", argument.name, value)
            })
            .collect();
        let line = format!("{} {}", tool.name, full.join(" "));
        let receipt = gate
            .call(line.trim_end())
            .unwrap_or_else(|refusal| panic!("`{line}` is a declared request: {refusal}"));
        // The `save` row reports the document it placed; the next row is `load`, and it is handed
        // exactly that digest above.
        if let Answer::Saved(facts) = &receipt.answer {
            document_digest = facts.digest.to_hex();
        }

        for (dropped, argument) in tool.args.iter().enumerate() {
            let without: Vec<&String> = full
                .iter()
                .enumerate()
                .filter(|(index, _)| *index != dropped)
                .map(|(_, value)| value)
                .collect();
            let line = format!("{} {}", tool.name, without
                .iter()
                .map(|value| value.as_str())
                .collect::<Vec<_>>()
                .join(" "));
            if argument.required {
                let refused = gate
                    .call(line.trim_end())
                    .expect_err("an absent required argument");
                assert_eq!(
                    refused.kind(),
                    RefusalKind::ArgumentAbsent,
                    "{line} should be absent, not {}",
                    refused.kind().name()
                );
            } else {
                // An **optional** argument is a declaration, not a silence: a row that says nothing
                // about what it requires of the world requires nothing, and is admitted. This is the
                // only shape in which `admit` trades length for meaning, and it is declared in the
                // table rather than decided at dispatch.
                gate.call(line.trim_end())
                    .unwrap_or_else(|refusal| panic!("`{line}` omits an optional argument: {refusal}"));
            }
            checked += 1;
        }
    }
    // The surface itself: the five rows this spec declared first contribute five arguments, the
    // `admit` row contributes the ten it declares — eight required and two optional — and U9's two
    // save rows contribute three between them (the reference a save declares, and the reference plus
    // its digest a load is handed). A literal, so adding an argument to a row is a decision someone
    // has to make here as well as in the table.
    assert_eq!(checked, 18, "five on the first five rows, ten on `admit`, three on the save rows");
}

/// The answer corresponds to the world it describes — **read, not recomputed**. The same receipt
/// verified against a world that has since moved, or after its geometry was evicted, refuses by name
/// with both values.
#[test]
fn an_answer_is_checked_against_the_world_and_not_against_itself() {
    let roster = process::roster().expect("the reference roster must admit");
    let mut gate = opened_gate(&roster);

    let receipt = gate.call("geometry chunk=0,0,0 level=0").expect("a derivation");
    receipt.verify(&gate.owners()).expect("it holds where it was made");

    // A moved world: the truth claim and the held-geometry claim both fail, and each names both the
    // recorded and the found value.
    let moved = gate.world().with_tick(Tick(1));
    let owners = Owners::new(
        &moved,
        &roster,
        gate.resident(),
        gate.chain(),
        1,
        gate.ruleset(),
        gate.intake(),
    );
    let failures = receipt.verify(&owners).expect_err("a moved world");
    assert!(failures.contains(RefusalKind::ClaimUnverified));
    assert!(
        failures.len() >= 2,
        "the truth and the geometry are both checked: {failures}"
    );
    fn names(set: &shploingus_clean_room::refusal::RefusalSet, claim: &str) -> bool {
        set.all().iter().any(|refusal| refusal.detail().contains(claim))
    }
    assert!(names(&failures, "truth_is"), "{failures}");
    assert!(names(&failures, "mesh_is_held"), "{failures}");
    assert!(failures.to_string().contains("recorded"), "{failures}");
    assert!(failures.to_string().contains("found"), "{failures}");

    // Evicting the geometry in the **same** world: nothing about the world moved, but the answer
    // claimed the gate holds this mesh — so the claim is re-read rather than re-derived, and it no
    // longer holds.
    let removed = gate.resident_mut().invalidate(moved.truth_digest());
    assert!(removed >= 1, "the tick-0 geometry is stale at tick 1");
    let failures = receipt.verify(&gate.owners()).expect_err("the geometry is gone");
    assert_eq!(failures.len(), 1, "{failures}");
    assert!(failures.contains(RefusalKind::ClaimUnverified));
    assert!(names(&failures, "mesh_is_held"), "{failures}");
    assert!(
        !names(&failures, "truth_is"),
        "the world did not move, so the truth claim still holds: {failures}"
    );

    // A field's claim is the one claim that is **re-evaluated**, not read: another tick is another
    // function, so the same request over the moved world answers different values, and the claim
    // says so with both digests rather than passing on its own copy of them.
    let fields = gate.call("fields field=elevation at=0,0,0 side=8").expect("a read");
    fields
        .verify(&gate.owners())
        .expect("the values are the world's own");
    let moved_owners = Owners::new(
        &moved,
        &roster,
        gate.resident(),
        gate.chain(),
        1,
        gate.ruleset(),
        gate.intake(),
    );
    let failures = fields
        .verify(&moved_owners)
        .expect_err("another tick is another function");
    assert_eq!(failures.len(), 1, "{failures}");
    let detail = failures.first_named().expect("one refusal").detail().to_string();
    assert!(detail.contains("field_is"), "{detail}");
    assert!(detail.contains("recorded values"), "{detail}");
    assert!(detail.contains("found"), "{detail}");

    // The record is untouched by all of this: the receipt's event claim still holds.
    let receipt = gate.call("record").expect("a read");
    receipt.verify(&gate.owners()).expect("the record is what it says");
    assert_eq!(
        receipt.answer,
        Answer::Record { events: 2, head: gate.chain().head() }
    );
    gate.chain().verify().expect("the record still verifies");
}

/// **A receipt is the evidence for one answer of one row**, and the public wire form is where that is
/// checked: a receipt whose evidence field was emptied, or which carries another row's claim, refuses
/// by name when it is reloaded — rather than being read back and verified over nothing.
#[test]
fn a_receipt_that_lost_its_evidence_refuses_by_name_over_the_public_wire_form() {
    let roster = process::roster().expect("the reference roster must admit");
    let mut gate = opened_gate(&roster);
    let receipt = gate.call("geometry chunk=0,0,0 level=0").expect("a derivation");
    let wire = receipt.wire();

    // The declared external form: the version, the request, the answer, the claims, the checks and the
    // effect, one part each.
    let fields = wire.split(WIRE_PART).collect::<Vec<_>>();
    assert_eq!(fields.len(), 6, "{wire}");

    // Emptied evidence is not evidence: both fields are the row's declaration, and a receipt without
    // them does not read back at all (before the binding it read back and verified vacuously).
    for (index, field) in [(3_usize, "the claims"), (4, "the checks")] {
        let mut stripped = fields.clone();
        stripped[index] = "";
        let refused = Receipt::from_wire(&stripped.join(&WIRE_PART.to_string()))
            .expect_err("a receipt without its evidence");
        assert_eq!(refused.kind(), RefusalKind::ReceiptMismatch, "{field}: {refused}");
        assert_eq!(refused.kind().name(), "receipt_mismatch");
    }

    // Another row's claim is not this row's evidence, even though it holds in the same world: a
    // `held_is` claim re-checks against the resident set, which has nothing to do with this answer.
    let held = gate.call("held").expect("a read");
    let mut crossbound = fields.clone();
    let foreign = held
        .wire()
        .split(WIRE_PART)
        .nth(3)
        .expect("the held receipt's claims")
        .to_string();
    crossbound[3] = &foreign;
    let refused = Receipt::from_wire(&crossbound.join(&WIRE_PART.to_string()))
        .expect_err("a `geometry` receipt may not claim `held_is`");
    assert_eq!(refused.kind(), RefusalKind::ReceiptMismatch, "{refused}");
    assert!(refused.detail().contains("held_is"), "{refused}");

    // The two ends agree: the receipt the gate handed out is the receipt the wire form reads back, and
    // it verifies against the world it was made in.
    let reloaded = Receipt::from_wire(&wire).expect("its own wire form reads back");
    assert_eq!(reloaded, receipt, "the served receipt is the receipt");
    reloaded
        .verify(&gate.owners())
        .expect("and it holds where it was made");
}

/// The chain the gate holds is **U2's own**, reached through its own writer: an event appended by a
/// caller and an event appended by the gate live in one verified chain, and the gate's ticket view is
/// U2's, not a restatement of it.
#[test]
fn the_record_is_the_chain_and_not_a_second_copy_of_it() {
    let roster = process::roster().expect("the reference roster must admit");
    let mut gate = opened_gate(&roster);

    let ticket = gate
        .chain_mut()
        .file_ticket("the gate's answer must match the world it names")
        .expect("a wirable subject");
    gate.chain_mut().resolve_ticket(ticket).expect("filed, so resolvable");

    let ours = gate.call("geometry chunk=0,0,0 level=0").expect("a derivation");
    let len = gate.chain().len();

    // The record the caller reads is one chain: U2's events, the ticket, and the gate's own appends.
    let wire = gate.chain().wire();
    let reparsed = Chain::from_wire(&wire).expect("the wire form round-trips");
    assert_eq!(&reparsed, gate.chain());
    assert_eq!(reparsed.len(), len);
    assert_eq!(reparsed.head(), ours.effect.appended);

    // And a receipt verified against the *parsed* chain is a receipt verified against the record's
    // own wire form — the same owners, read back from the wire.
    let owners = Owners::new(
        gate.world(),
        &roster,
        gate.resident(),
        &reparsed,
        1,
        gate.ruleset(),
        gate.intake(),
    );
    ours.verify(&owners).expect("the record reads back as the one it was made in");

    // A claim of a hand-built receipt is checked against the chain, not against the receipt: an
    // `event_is` naming a position the chain does not have is refused by name.
    let forged = Claim::EventIs { seq: 99, recorded: ours.effect.appended.expect("a derivation") };
    let refusal = forged.check(&owners).expect_err("no such event");
    assert_eq!(refusal.kind(), RefusalKind::ClaimUnverified);
    assert!(refusal.detail().contains("no longer in the record"), "{refusal}");
}
