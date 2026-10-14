//! **The end-to-end check for U9 — a written world, through the public surface and a real file.**
//!
//! Derived from `specs/006-world-saves-and-seeds/spec.md` (three user stories) and its
//! `contracts/u9-seams.md` (C1–C3). Everything here goes through the **public** API: this test is one
//! of the two doors (`src/main.rs`'s evidence is the other), so a capability that only exists inside
//! the crate's tests would not be measured by it. Each test uses its **own** file name under the
//! system temp directory, so a parallel run cannot collide.

use std::path::PathBuf;

use shploingus_clean_room::bridge::{Answer, DocumentFacts, Gate};
use shploingus_clean_room::chain::{Chain, EventKind, Namespace};
use shploingus_clean_room::digest::Digest;
use shploingus_clean_room::intake::{Intake, Source, WorldDeclarations};
use shploingus_clean_room::process;
use shploingus_clean_room::reference;
use shploingus_clean_room::refusal::RefusalKind;
use shploingus_clean_room::save::{Document, Ruleset, Save};
use shploingus_clean_room::transport::{self, Response};
use shploingus_clean_room::world::{Domain, Tick, World, WorldIdentity};

/// The one reference identity (`reference::identity`), so these tests and the binary's evidence are
/// about the **same** world rather than about two similar ones.
fn identity() -> WorldIdentity {
    reference::identity().expect("the reference identity's period is one second")
}

/// The reference world's ruleset, read from its **one** declaration ([`reference`]) rather than
/// repeated here, so this test and the binary's evidence cannot disagree about which world a save is.
fn ruleset() -> Ruleset {
    reference::ruleset()
}

/// A record with two events: a world opened and a tick advanced.
fn record() -> Chain {
    let mut chain = Chain::new();
    chain
        .append(Namespace::World, EventKind::WorldOpened, "seed=1592639215 period=1s")
        .expect("a world event is wirable");
    chain
        .append(Namespace::Record, EventKind::TickAdvanced, "tick=3")
        .expect("a tick event is wirable");
    chain
}

/// A file name this test owns.
fn path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("shploingus-u9-{name}.save"))
}

#[test]
fn a_written_world_reads_back_as_the_same_world_through_a_file() {
    let world = World::new(identity()).with_tick(Tick(3));
    let live = World::new(identity());
    let chain = record();

    let bytes = Save::of(&world, &chain, ruleset()).expect("the world writes down");
    let file = path("round-trip");
    std::fs::write(&file, &bytes).expect("the save reaches the disk");
    let read_back = std::fs::read(&file).expect("and comes back");
    assert_eq!(read_back, bytes, "a file is the same bytes it was written from");

    let save = Save::parse(&read_back).expect("the file parses");
    assert_eq!(save.bytes(), bytes, "save -> load -> save is one byte string");

    let loaded = save.load(ruleset()).expect("and loads");
    assert_eq!(
        loaded.report.truth,
        world.truth_digest(),
        "the loaded world is the world it was written from"
    );
    assert_ne!(
        loaded.report.truth,
        live.truth_digest(),
        "and it is not the world at another tick"
    );
    assert_eq!(loaded.world.identity(), world.identity());
    assert_eq!(loaded.world.tick(), Tick(3));
    assert_eq!(loaded.chain.events(), chain.events(), "the record is the record");
    assert_eq!(loaded.report.record_head, chain.head());
    assert_eq!(loaded.report.record_len, chain.len());
    assert!(loaded.chain.verify().is_ok(), "the loaded record verifies");

    // **One digest per world at 1/2/4/8 workers** — the instrument U3 measured, applied to the
    // identity the save carries: the rebuilt world is the ticked world.
    let roster = process::roster().expect("the reference roster admits");
    let mut digests = Vec::new();
    for workers in [1_usize, 2, 4, 8] {
        let digest = process::world_generation_digest_with_workers(
            &roster,
            loaded.world.identity(),
            loaded.world.tick(),
            process::declared_request(),
            workers,
        )
        .expect("the reference roster generates");
        digests.push(digest);
    }
    assert!(
        digests.windows(2).all(|pair| pair[0] == pair[1]),
        "the worker count moves no digest: {digests:?}"
    );
    let live_digest = process::world_generation_digest(&roster, world.identity(), world.tick())
        .expect("the live world generates");
    assert_eq!(
        digests[0], live_digest,
        "a world rebuilt from a save is the world that wrote it"
    );

    let _ = std::fs::remove_file(&file);
}

#[test]
fn a_save_carries_five_stream_seeds_and_the_load_checks_every_one() {
    let world = World::new(identity()).with_tick(Tick(3));
    let bytes = Save::of(&world, &record(), ruleset()).expect("written");
    let save = Save::parse(&bytes).expect("parsed");

    // **The seed systems, in the plural.** One per declared domain, each the world's own — not one
    // dial renamed.
    assert_eq!(save.streams().len(), Domain::COUNT);
    for domain in Domain::ALL {
        let declared = save
            .streams()
            .iter()
            .find(|stream| stream.domain == domain)
            .unwrap_or_else(|| panic!("the save declares the {} stream", domain.name()));
        assert_eq!(
            declared.seed,
            world.stream_seed(domain),
            "the {} stream's seed is the world's own",
            domain.name()
        );
    }
    // The five are distinct: five streams with five seeds, not one number repeated.
    let mut seeds: Vec<u64> = save.streams().iter().map(|stream| stream.seed).collect();
    seeds.sort_unstable();
    seeds.dedup();
    assert_eq!(seeds.len(), Domain::COUNT, "five streams are five seeds");

    // And the loaded report carries them, so a caller can compare two worlds' streams.
    let loaded = save.load(ruleset()).expect("loaded");
    assert_eq!(loaded.report.streams, save.streams());

    // A load under another configuration is another world, and it says so with both identities.
    let refusals = save
        .load(Ruleset::new(8, 3))
        .expect_err("another ruleset is refused");
    assert!(refusals.contains(RefusalKind::IdentityMismatch));
    let detail: String = refusals
        .all()
        .iter()
        .map(|refusal| refusal.detail().to_string())
        .collect::<Vec<String>>()
        .join(" | ");
    assert!(detail.contains("ruleset 7 revision 3"), "{detail}");
    assert!(detail.contains("ruleset 8 revision 3"), "{detail}");
}

#[test]
fn consecutive_ticks_share_the_seed_and_differ_in_the_tick() {
    let zero = Save::parse(
        &Save::of(&World::new(identity()), &record(), ruleset()).expect("written at tick 0"),
    )
    .expect("parsed");
    let three = Save::parse(
        &Save::of(&World::new(identity()).with_tick(Tick(3)), &record(), ruleset())
            .expect("written at tick 3"),
    )
    .expect("parsed");

    assert_eq!(zero.identity().seed, three.identity().seed, "the seed is a world's");
    assert_eq!(zero.identity().period, three.identity().period);
    assert_ne!(zero.identity().tick, three.identity().tick, "the tick is not the seed");

    let zero_truth = zero.load(ruleset()).expect("tick 0 loads").report.truth;
    let three_truth = three.load(ruleset()).expect("tick 3 loads").report.truth;
    assert_ne!(zero_truth, three_truth, "a later tick is a different world");
    assert_eq!(zero_truth, World::new(identity()).truth_digest());
}

#[test]
fn bytes_that_cannot_be_trusted_are_refused_by_name_through_the_public_surface() {
    let bytes = Save::of(&World::new(identity()), &record(), ruleset()).expect("written");

    // Not a save at all.
    let junk = path("junk");
    std::fs::write(&junk, b"not a save at all").expect("written");
    let read = std::fs::read(&junk).expect("read");
    let refusals = Save::parse(&read).expect_err("refused");
    assert!(refusals.contains(RefusalKind::NotASave), "{refusals}");

    // A file shorter than the record it declares.
    let short = path("short");
    std::fs::write(&short, &bytes[..60]).expect("written");
    let read = std::fs::read(&short).expect("read");
    let refusals = Save::parse(&read).expect_err("refused");
    assert!(refusals.contains(RefusalKind::SaveTruncated), "{refusals}");

    // One byte altered.
    let mut altered = bytes.clone();
    altered[40] ^= 0x01;
    let refusals = Save::parse(&altered).expect_err("refused");
    assert!(refusals.contains(RefusalKind::SaveTampered), "{refusals}");

    // **No refusal yields a world.** Each of the three above returned an `Err`; there is no
    // "loaded with warnings" and no partially loaded world for a caller to trip over.
    let _ = std::fs::remove_file(&junk);
    let _ = std::fs::remove_file(&short);
}

/// A root this test owns, fresh each run.
fn root(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("shploingus-u9-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the fixture root is creatable");
    dir
}

/// **The document form, through the library's own door — which is U6's content gate.** A save is
/// written where it is declared to live and read back **by reference** (`path + digest`): the bytes
/// come from `Intake::read`, so an absent document and one whose bytes have moved are refused by the
/// gate's own names before anything is parsed, and a document that is not the save it claims is
/// refused by U9's.
#[test]
fn a_save_document_is_written_at_a_reference_and_read_back_through_the_content_gate() {
    let dir = root("document");
    let world = World::new(identity()).with_tick(Tick(3));
    let chain = record();
    let bytes = Save::of(&world, &chain, ruleset()).expect("the world writes down");

    // The one writer. What it hands back is the reference, not the bytes.
    let document = Document::write(&dir, "the-world.save", &bytes).expect("the document is placed");
    assert_eq!(document.path(), "the-world.save");
    assert_eq!(document.digest(), Digest::of(&bytes));
    assert_eq!(document.len(), bytes.len());
    assert_eq!(
        std::fs::read(dir.join("the-world.save")).expect("the document is on the disk"),
        bytes,
        "the document is the bytes, at the path the reference names"
    );

    // The one reader, through the gate.
    let roster = process::roster().expect("the reference roster admits");
    let intake = Intake::over(
        &dir,
        WorldDeclarations { roster: &roster, registry: reference::registry() },
    );
    let source = Source::new(document.path(), document.digest());
    let (save, read_back) = Save::read_document(&intake, &source).expect("the document is read");
    assert_eq!(read_back, document, "the document read is the document declared");
    assert_eq!(save.bytes(), bytes, "and what it parses to is the bytes written");
    let loaded = save.load(ruleset()).expect("the document loads");
    assert_eq!(loaded.report.truth, world.truth_digest());

    // **Absent**: the gate's own name, and the reference is named in the words.
    let absent = Save::read_document(&intake, &Source::new("gone.save", document.digest()))
        .expect_err("there is no such document");
    assert!(absent.contains(RefusalKind::SourceAbsent), "{absent}");
    assert!(absent.to_string().contains("gone.save"), "{absent}");

    // **Moved**: the reference is right and the bytes at it are not the bytes it describes — both
    // digests in the message, because a mismatch a caller cannot see is a mismatch a caller cannot fix.
    let moved_bytes = b"# not the save at all\n";
    std::fs::write(dir.join("the-world.save"), moved_bytes).expect("the fixture is written");
    let moved = Save::read_document(&intake, &source).expect_err("the bytes have moved");
    assert!(moved.contains(RefusalKind::SourceDigestMismatch), "{moved}");
    assert!(moved.to_string().contains(&document.digest().to_hex()), "{moved}");
    assert!(moved.to_string().contains(&Digest::of(moved_bytes).to_hex()), "{moved}");

    // **Altered inside its own trailer**: the reference names these bytes exactly, and they are not the
    // save they claim — U9's own name for that, reached through the same door.
    let mut altered = bytes.clone();
    let last = altered.len() - 1;
    altered[last] ^= 0x01;
    let altered_document = Document::write(&dir, "altered.save", &altered).expect("placed");
    let tampered = Save::read_document(
        &intake,
        &Source::new(altered_document.path(), altered_document.digest()),
    )
    .expect_err("the save is not the save it claims");
    assert!(tampered.contains(RefusalKind::SaveTampered), "{tampered}");

    // **And a document that cannot be placed is refused by its own name** rather than reported as a
    // save: the root here is a *file*, so the reference under it cannot be written.
    let unmade = std::env::temp_dir().join("shploingus-u9-document-as-a-file");
    std::fs::write(&unmade, b"not a directory\n").expect("the fixture is written");
    let refused = Document::write(&unmade, "the-world.save", &bytes)
        .expect_err("a document under a file is not writable");
    assert!(refused.contains(RefusalKind::DocumentUnwritable), "{refused}");
    assert!(refused.to_string().contains("the-world.save"), "{refused}");

    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_file(&unmade);
}

/// One served line, through the transport the binary's `serve` mode uses: one request in, one response
/// record out — read back with the crate's own reader, so what is asserted is what crosses the
/// protocol rather than what the gate happens to hold.
fn session(gate: &mut Gate<'_>, line: &str) -> Response {
    let input = format!("{line}\n");
    let mut output: Vec<u8> = Vec::new();
    transport::serve(gate, &mut input.as_bytes(), &mut output).expect("a session");
    let mut reader: &[u8] = &output;
    transport::read_response(&mut reader)
        .expect("a response this build's reader accepts")
        .expect("the transport answers every request")
}

/// **`save`/`load` are rows of the one closed table** (landed 2026-10-12, §28), so a client reaches
/// them the way it reaches every other verb: over the served line, one request in and one response
/// record out. The write's own answer carries the digest, and the read is handed **that** reference.
#[test]
fn a_world_is_written_and_read_back_over_the_served_line() {
    let dir = root("document-served");
    let roster = process::roster().expect("the reference roster admits");
    let mut gate = reference::gate(&roster, 1).expect("the reference declaration is admitted");
    // **The gate's own root**: a declared reference is relative to the door's root, so the document is
    // written where the reader will look — one declaration, not a path a caller hopes about.
    gate.set_intake_root(&dir);

    let saved = session(&mut gate, "save path=served.save");
    let written = saved.receipt().expect("the row answered").clone();
    assert_eq!(written.request, "save path=served.save");
    assert!(
        written.verify(&gate.owners()).is_ok(),
        "the write's receipt is re-established from its request"
    );
    let written_facts: DocumentFacts = match &written.answer {
        Answer::Saved(facts) => facts.clone(),
        other => panic!("a write answers with the document it placed, not {other:?}"),
    };
    assert_eq!(written_facts.seed, gate.world().identity().seed);
    assert_eq!(written_facts.ruleset, gate.ruleset());

    // The reference the second line carries is the digest the first line's answer declared — read off
    // the wire, not out of the gate.
    let load_line = format!("load path=served.save digest={}", written_facts.digest.to_hex());
    let loaded = session(&mut gate, &load_line);
    let read_back = loaded.receipt().expect("the row answered").clone();
    assert_eq!(read_back.request, load_line);
    assert!(
        read_back.verify(&gate.owners()).is_ok(),
        "the read's receipt is re-established too"
    );
    let read_facts = match &read_back.answer {
        Answer::Loaded(facts) => facts.clone(),
        other => panic!("a read answers with the world those bytes are of, not {other:?}"),
    };
    assert_eq!(read_facts, written_facts, "the document reads back as itself");
    assert_eq!(
        read_facts.truth,
        gate.world().truth_digest(),
        "and the world it carries is the world the gate serves"
    );

    // **A document that is not there is refused by the gate's own name** — and the refusal is about the
    // line it answers, so a client sees which reference it was.
    let missing = format!("load path=elsewhere.save digest={}", written_facts.digest.to_hex());
    let refused = session(&mut gate, &missing);
    assert_eq!(refused.refusal_kind(), Some("source_absent"), "{refused:?}");
    assert_eq!(refused.request(), missing);

    // Neither the write nor the read is an event about the world: the record is the record.
    assert_eq!(gate.chain().len(), 1, "the refusal was the only thing recorded");
    assert!(gate.chain().verify().is_ok(), "and the record verifies");

    let _ = std::fs::remove_dir_all(&dir);
}
