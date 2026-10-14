//! **The save subsystem's own tests** — the layout, the parser and the loader, driven directly.
//!
//! Separate from [`super`] so the module's own text is readable without scrolling past four hundred
//! lines of assertions. The import scope is identical to a `mod tests` nested in it: `use super::*;`
//! brings the vocabulary, and the layout's own items are named explicitly because the tests reach
//! past the parser to build bytes the writer would never emit.

use super::layout::{encode, encode_raw, TRAILER};
use super::*;
use crate::chain::{Chain, EventKind, Namespace};
use crate::digest::Digest;
use crate::intake::{Intake, Source, WorldDeclarations};
use crate::process;
use crate::reference;
use crate::refusal::{RefusalKind, RefusalSet};
use crate::world::{DeclaredPeriod, Domain, Tick, World, WorldIdentity};

fn identity() -> WorldIdentity {
    WorldIdentity::new(0x5EED_BEEF, DeclaredPeriod::seconds(1).expect("one second"))
}

/// The reference world's ruleset, read from its **one** declaration ([`crate::reference`]) rather
/// than repeated here, so this fixture cannot drift from the world the binary measures (C-46).
fn ruleset() -> Ruleset {
    reference::ruleset()
}

fn world_at(tick: u64) -> World {
    World::new(identity()).with_tick(Tick(tick))
}

/// A record with two events, so a save carries something worth carrying.
fn record() -> Chain {
    let mut chain = Chain::new();
    chain
        .append(Namespace::World, EventKind::WorldOpened, "seed=1592639215 period=1")
        .expect("a world event is wirable");
    chain
        .append(Namespace::Record, EventKind::TickAdvanced, "tick=1")
        .expect("a tick event is wirable");
    chain
}

fn kinds(set: &RefusalSet) -> Vec<RefusalKind> {
    set.all().iter().map(|refusal| refusal.kind()).collect()
}

fn details(set: &RefusalSet) -> String {
    set.all()
        .iter()
        .map(|refusal| format!("{}: {}", refusal.kind().name(), refusal.detail()))
        .collect::<Vec<_>>()
        .join(" | ")
}

#[test]
fn the_document_form_asks_the_gates_question_before_its_own() {
    // A root this test owns, and the **one** gate a document is read through: the content gate, over
    // the reference world's own declarations (the same borrows `reference::gate` builds it with).
    let dir = std::env::temp_dir().join("shploingus-save-document-unit");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the fixture root is creatable");
    let roster = process::roster().expect("the reference roster admits");
    let intake = Intake::over(&dir, WorldDeclarations { roster: &roster, registry: reference::registry() });

    // A document that **is** the save the reference declares: written by the one writer, read by the
    // one reader, and the same bytes in both directions.
    let world = world_at(2);
    let bytes = Save::of(&world, &record(), ruleset()).expect("the world writes down");
    let document = Document::write(&dir, "a-world.save", &bytes).expect("placed");
    assert_eq!(document.digest(), Digest::of(&bytes));
    let (save, read_back) = Save::read_document(
        &intake,
        &Source::new(document.path(), document.digest()),
    )
    .expect("the document reads back");
    assert_eq!(read_back, document, "the document read is the document declared");
    assert_eq!(save.bytes(), bytes, "and it parses to the bytes written");
    assert_eq!(
        save.load(ruleset()).expect("it loads").report.truth,
        world.truth_digest(),
        "to the world it was written from"
    );

    // **The gate's question first, and by the gate's own name**: bytes that are not a save at all, whose
    // digest the reference names exactly, are refused `not_a_save` — the document *is* the declared one,
    // so the fault is the layout's and it is U9's name that says so. (The mirror case — a document that
    // is gone, or whose bytes have moved — is refused `source_absent`/`source_digest_mismatch` by the
    // gate, before anything is parsed; `tests/u9_saves.rs` measures both.)
    let junk = b"not a save at all, and its digest is right\n";
    let junk_document = Document::write(&dir, "junk.save", junk).expect("placed");
    let refusals = Save::read_document(
        &intake,
        &Source::new(junk_document.path(), junk_document.digest()),
    )
    .expect_err("these bytes are not a save");
    assert!(refusals.contains(RefusalKind::NotASave), "{refusals}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_save_round_trips_its_bytes_exactly() {
    let bytes = Save::of(&world_at(4), &record(), ruleset()).expect("the world writes down");
    let save = Save::parse(&bytes).expect("and reads back");
    assert_eq!(save.bytes(), bytes, "save -> load -> save is one byte string");
    assert_eq!(Save::parse(&save.bytes()).expect("twice"), save);
    assert_eq!(save.identity().seed, identity().seed);
    assert_eq!(save.identity().period, 1);
    assert_eq!(save.identity().tick, 4);
    assert_eq!(save.ruleset(), ruleset());
    assert_eq!(save.streams().len(), Domain::COUNT);
    assert_eq!(save.record().1, 2);
}

#[test]
fn two_saves_of_one_world_are_one_byte_string() {
    let one = Save::of(&world_at(2), &record(), ruleset()).expect("written");
    let two = Save::of(&world_at(2), &record(), ruleset()).expect("written again");
    assert_eq!(one, two, "nothing in a save depends on the work that produced it");
}

#[test]
fn an_empty_record_is_a_record() {
    let bytes = Save::of(&world_at(0), &Chain::new(), ruleset()).expect("an empty record writes");
    let loaded = Save::parse(&bytes)
        .expect("and reads")
        .load(ruleset())
        .expect("and loads");
    assert_eq!(loaded.report.record_len, 0);
    assert_eq!(loaded.report.record_head, None);
    assert!(loaded.chain.is_empty());
}

#[test]
fn a_loaded_world_carries_the_truth_digest_of_the_world_it_was_written_from() {
    let world = world_at(3);
    let bytes = Save::of(&world, &record(), ruleset()).expect("written");
    let loaded = Save::parse(&bytes)
        .expect("parsed")
        .load(ruleset())
        .expect("loaded");
    assert_eq!(
        loaded.report.truth,
        world.truth_digest(),
        "the loaded world is the world it was written from"
    );
    assert_eq!(loaded.world.identity(), world.identity());
    assert_eq!(loaded.world.tick(), world.tick());
    assert_eq!(loaded.report.identity.tick, 3);
}

#[test]
fn the_loaded_record_is_the_same_chain_and_verifies() {
    let chain = record();
    let bytes = Save::of(&world_at(1), &chain, ruleset()).expect("written");
    let loaded = Save::parse(&bytes)
        .expect("parsed")
        .load(ruleset())
        .expect("loaded");
    assert_eq!(loaded.chain.events(), chain.events());
    assert_eq!(loaded.chain.len(), 2);
    assert_eq!(loaded.chain.head(), chain.head());
    assert_eq!(loaded.report.record_len, 2);
    assert_eq!(loaded.report.record_head, chain.head());
    assert!(loaded.chain.verify().is_ok(), "the loaded record is the chain");
}

#[test]
fn bytes_that_are_not_a_save_are_refused_by_name() {
    for bytes in [b"".as_slice(), b"not a save at all", b"shploingus.save/2 and then"] {
        let refusals = Save::parse(bytes).expect_err("refused");
        assert_eq!(kinds(&refusals), vec![RefusalKind::NotASave]);
    }
    // A version the layout does not declare is refused rather than guessed at.
    let mut wrong_version = Save::of(&world_at(0), &record(), ruleset()).expect("written");
    wrong_version[MAGIC.len()] = 9;
    let refusals = Save::parse(&wrong_version).expect_err("refused");
    assert!(kinds(&refusals).contains(&RefusalKind::SaveTampered), "{}", details(&refusals));
}

#[test]
fn a_truncated_save_is_refused_by_name_and_says_where() {
    let bytes = Save::of(&world_at(0), &record(), ruleset()).expect("written");
    for cut in [20_usize, 40, 100] {
        let refusals = Save::parse(&bytes[..cut]).expect_err("refused");
        assert!(
            kinds(&refusals).contains(&RefusalKind::SaveTruncated),
            "cut at {cut}: {}",
            details(&refusals)
        );
    }
    let refusals = Save::parse(&bytes[..bytes.len() - 8]).expect_err("refused");
    assert!(
        kinds(&refusals)
            .iter()
            .any(|kind| matches!(kind, RefusalKind::SaveTampered | RefusalKind::SaveTruncated)),
        "{}",
        details(&refusals)
    );
}

/// **The declared layout's own header with a count the file cannot pay for** — crafted the way a
/// hostile file arrives: the magic, the fixed header, a trailer-shaped tail, and not one table
/// entry. Nothing here is a hand-copied second copy of the layout; it is the same field order
/// `encode_raw` writes, and it reaches the count, which is the point.
fn crafting_a_count_the_file_cannot_hold(count: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&VERSION.to_le_bytes());
    bytes.extend_from_slice(&ruleset().ruleset.to_le_bytes());
    bytes.extend_from_slice(&ruleset().revision.to_le_bytes());
    bytes.extend_from_slice(&identity().seed.to_le_bytes());
    bytes.extend_from_slice(&1_u64.to_le_bytes()); // the declared period
    bytes.extend_from_slice(&0_u64.to_le_bytes()); // the tick
    bytes.extend_from_slice(&count.to_le_bytes());
    bytes.extend_from_slice(&[0_u8; TRAILER]); // the layout's own edge, and no table
    bytes
}

#[test]
fn a_declared_stream_count_cannot_buy_work_or_memory() {
    // **The count is the only number a file gets to choose about how much work the reader does**,
    // so it must not be able to choose. This input is 97 bytes — the header and a tail — and it
    // holds no table at all; the fault is that, *once*. Before the pass of 2026-10-12 the same
    // header without the tail produced **4,000,010** refusals for a declared 2,000,000 streams,
    // and aborted on an allocation at `u32::MAX`.
    for count in [6, 1_000_000, u32::MAX] {
        let bytes = crafting_a_count_the_file_cannot_hold(count);
        assert_eq!(bytes.len(), 97, "the crafted file is the header, a tail, and no table");
        let refusals = Save::parse(&bytes).expect_err("refused");
        assert_eq!(refusals.len(), 1, "count {count}: {}", details(&refusals));
        assert_eq!(kinds(&refusals), vec![RefusalKind::SaveTruncated], "count {count}");
        assert!(
            details(&refusals).contains(&count.to_string()),
            "the declared count is named: {}",
            details(&refusals)
        );
    }
}

#[test]
fn a_short_file_reports_the_fault_it_can_evidence_and_nothing_else() {
    // **One file, one fault to report.** A truncated save is short — and it is *only* that.
    // Before the pass of 2026-10-12, cutting this save at 65 bytes produced **20** refusals, five
    // of them claiming the save declared no seed for a stream whose table was simply not there,
    // plus a `save_tampered` read out of the file's own header.
    let whole = Save::of(&world_at(0), &record(), ruleset()).expect("written");
    for cut in [18_usize, 40, 65, 100, 200, whole.len() - 1] {
        let refusals = Save::parse(&whole[..cut]).expect_err("refused");
        assert_eq!(refusals.len(), 1, "cut at {cut}: {}", details(&refusals));
        assert_eq!(
            kinds(&refusals),
            vec![RefusalKind::SaveTruncated],
            "cut at {cut}: {}",
            details(&refusals)
        );
    }
}

#[test]
fn a_version_the_layout_does_not_declare_is_refused_by_name() {
    // Reachable only with a trailer that matches the bytes it covers, which is why the older test
    // (a flipped version byte, and a stale trailer) reports the alteration instead. A version this
    // build does not read is refused **by name** rather than guessed at (P2).
    let mut bytes = Save::of(&world_at(0), &record(), ruleset()).expect("written");
    bytes[MAGIC.len()] = 2;
    let at = bytes.len() - TRAILER;
    let trailer = Digest::of(&bytes[..at]);
    bytes[at..].copy_from_slice(trailer.as_bytes());

    let refusals = Save::parse(&bytes).expect_err("refused");
    assert_eq!(kinds(&refusals), vec![RefusalKind::NotASave], "{}", details(&refusals));
    assert!(details(&refusals).contains("version 2"), "{}", details(&refusals));
    assert!(details(&refusals).contains("reads 1"), "{}", details(&refusals));
}

#[test]
fn a_flipped_byte_is_refused_as_tampered_with_both_digests() {
    let mut bytes = Save::of(&world_at(0), &record(), ruleset()).expect("written");
    let at = MAGIC.len() + 24;
    bytes[at] ^= 0x01;
    let refusals = Save::parse(&bytes).expect_err("refused");
    assert_eq!(kinds(&refusals), vec![RefusalKind::SaveTampered]);
    let detail = details(&refusals);
    assert!(
        detail.contains("the trailer says") && detail.contains("the bytes before it say"),
        "both digests are named: {detail}"
    );
    assert_eq!(detail.matches("the trailer says").count(), 1);
}

#[test]
fn a_save_missing_a_declared_stream_is_refused_by_name() {
    let world = world_at(0);
    let missing: Vec<StreamSeed> = Domain::ALL[..4]
        .iter()
        .map(|domain| StreamSeed { domain: *domain, seed: world.stream_seed(*domain) })
        .collect();
    let bytes = encode(
        &SaveIdentity { seed: world.identity().seed, period: 1, tick: 0 },
        ruleset(),
        &missing,
        None,
        0,
        "",
    );
    let refusals = Save::parse(&bytes).expect_err("refused");
    assert_eq!(kinds(&refusals), vec![RefusalKind::SeedUndeclared]);
    assert!(
        details(&refusals).contains(Domain::Detail.name()),
        "the missing stream is named: {}",
        details(&refusals)
    );
}

#[test]
fn a_stream_the_receiver_does_not_declare_is_refused_by_name() {
    let bytes = encode_raw(
        &SaveIdentity { seed: 1, period: 1, tick: 0 },
        ruleset(),
        &[(0, 10), (1, 11), (2, 12), (3, 13), (4, 14), (9, 99)],
        None,
        0,
        "",
    );
    let refusals = Save::parse(&bytes).expect_err("refused");
    assert_eq!(kinds(&refusals), vec![RefusalKind::SeedUndeclared]);
    assert!(
        details(&refusals).contains("tag 9"),
        "the undeclared tag is named: {}",
        details(&refusals)
    );
}

#[test]
fn every_fault_in_a_save_is_collected_not_just_the_first() {
    // Three declared domains are absent and one tag is undeclared: four faults, one pass.
    let bytes = encode_raw(
        &SaveIdentity { seed: 1, period: 1, tick: 0 },
        ruleset(),
        &[(0, 10), (1, 11), (9, 99)],
        None,
        0,
        "",
    );
    let refusals = Save::parse(&bytes).expect_err("refused");
    assert_eq!(refusals.len(), 4, "{}", details(&refusals));
    assert!(kinds(&refusals).iter().all(|kind| *kind == RefusalKind::SeedUndeclared));
    for domain in [Domain::Climate, Domain::Hydrology, Domain::Detail] {
        assert!(details(&refusals).contains(domain.name()), "{}", details(&refusals));
    }
}

#[test]
fn a_stream_seed_that_is_not_the_worlds_own_is_refused_with_both_values() {
    let world = world_at(0);
    let doctored: Vec<StreamSeed> = Domain::ALL
        .iter()
        .map(|domain| {
            let own = world.stream_seed(*domain);
            StreamSeed {
                domain: *domain,
                seed: if *domain == Domain::Geology { own ^ 1 } else { own },
            }
        })
        .collect();
    let bytes = encode(
        &SaveIdentity { seed: world.identity().seed, period: 1, tick: 0 },
        ruleset(),
        &doctored,
        None,
        0,
        "",
    );
    let refusals = Save::parse(&bytes)
        .expect("the layout is intact")
        .load(ruleset())
        .expect_err("but it is not this world");
    assert_eq!(kinds(&refusals), vec![RefusalKind::IdentityMismatch]);
    let detail = details(&refusals);
    assert!(detail.contains(Domain::Geology.name()), "{detail}");
    assert!(
        detail.contains(&world.stream_seed(Domain::Geology).to_string()),
        "the world's own seed is named: {detail}"
    );
}

#[test]
fn a_load_under_another_ruleset_is_refused_with_both() {
    let bytes = Save::of(&world_at(0), &record(), ruleset()).expect("written");
    let save = Save::parse(&bytes).expect("parsed");
    let refusals = save
        .load(Ruleset::new(8, 3))
        .expect_err("another configuration is another world");
    assert_eq!(kinds(&refusals), vec![RefusalKind::IdentityMismatch]);
    let detail = details(&refusals);
    assert!(detail.contains("ruleset 7 revision 3"), "{detail}");
    assert!(detail.contains("ruleset 8 revision 3"), "{detail}");
    // And the same ruleset loads.
    assert!(save.load(ruleset()).is_ok());
}

#[test]
fn consecutive_ticks_share_the_seed_and_differ_in_the_tick() {
    let zero = Save::parse(&Save::of(&world_at(0), &record(), ruleset()).expect("written"))
        .expect("parsed");
    let one = Save::parse(&Save::of(&world_at(1), &record(), ruleset()).expect("written"))
        .expect("parsed");
    assert_eq!(zero.identity().seed, one.identity().seed, "the seed is the world's");
    assert_eq!(zero.identity().period, one.identity().period);
    assert_ne!(zero.identity().tick, one.identity().tick, "the tick moved");
    assert_eq!(zero.load(ruleset()).expect("loads").report.truth, world_at(0).truth_digest());
    assert_ne!(
        zero.load(ruleset()).expect("loads").report.truth,
        one.load(ruleset()).expect("loads").report.truth,
        "a later tick is a different world"
    );
    // The streams are the world's too: a different tick roots them differently.
    assert_ne!(
        zero.streams()[0].seed,
        one.streams()[0].seed,
        "a stream seed is a function of the tick"
    );
}

#[test]
fn a_record_that_is_not_the_declared_one_is_refused_as_diverged() {
    // Coherent bytes — the trailer matches — whose declaration belongs to another record.
    let carried = record();
    let other = {
        let mut chain = Chain::new();
        chain
            .append(Namespace::World, EventKind::WorldOpened, "seed=1 period=1")
            .expect("wirable");
        chain
    };
    let world = world_at(0);
    let streams: Vec<StreamSeed> = Domain::ALL
        .iter()
        .map(|domain| StreamSeed { domain: *domain, seed: world.stream_seed(*domain) })
        .collect();
    let bytes = encode(
        &SaveIdentity { seed: identity().seed, period: 1, tick: 0 },
        ruleset(),
        &streams,
        other.head(),
        other.len() as u64,
        &carried.wire(),
    );
    let refusals = Save::parse(&bytes)
        .expect("the layout is intact")
        .load(ruleset())
        .expect_err("the record is not the one declared");
    assert_eq!(kinds(&refusals), vec![RefusalKind::RecordDiverged]);
    let detail = details(&refusals);
    assert!(detail.contains("2 event(s)") && detail.contains("1 event(s)"), "{detail}");
}

#[test]
fn a_refused_load_produces_no_world_and_no_record() {
    let bytes = Save::of(&world_at(0), &record(), ruleset()).expect("written");
    let save = Save::parse(&bytes).expect("parsed");
    assert!(save.load(Ruleset::new(1, 1)).is_err());

    // A zero declared period is refused by U2's own rule, passed through by name.
    let streams: Vec<StreamSeed> = Domain::ALL
        .iter()
        .map(|domain| StreamSeed { domain: *domain, seed: world_at(0).stream_seed(*domain) })
        .collect();
    let bytes = encode(
        &SaveIdentity { seed: 1, period: 0, tick: 0 },
        ruleset(),
        &streams,
        None,
        0,
        "",
    );
    let refusals = Save::parse(&bytes)
        .expect("the layout is intact")
        .load(ruleset())
        .expect_err("a period of zero is not a world");
    assert_eq!(kinds(&refusals), vec![RefusalKind::ZeroPeriod]);
}
