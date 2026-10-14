//! **U12's acceptance instrument** — `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §5 row 12's own check, measured
//! through the crate's public surface rather than its internals:
//!
//! > *an organisation's member acts only through the one tool table, and an act it may not do is refused by
//! > name and recorded*
//!
//! and this pass's own clause: the being's need is met through the game's **own** cell-based water law (the
//! `hydrology` stage's cells, walked along `d8_receivers`) — read as **one static evaluation at `Tick(0)`**
//! *(2026-10-13, §34: the state that carries tick to tick is built now — `src/automaton.rs`, with its own
//! instrument in `tests/u12_stepping.rs` and the run's own `U12 stepped plane:` lines — so this header's
//! "not a stepping cellular automaton, which is owed" was §33's reading of the tree §32 left, and the static
//! read below is what this file still measures: one evaluation, over the plane as the generator left it)* —
//! its choice is a **laya question read by von**, and every refusal is by name and recorded. Everything here
//! is read off the public API — the same surface the binary's `check_u12` drives — so a test and the running
//! product measure one behaviour rather than two descriptions of one.

use std::path::{Path, PathBuf};

use shploingus_clean_room::axis::{AxisKind, LadderValues};
use shploingus_clean_room::being::{self, Act, Being, Organisation, WorldRead};
use shploingus_clean_room::chunk::CubicWorld;
use shploingus_clean_room::field::fields_at;
use shploingus_clean_room::policy::Action;
use shploingus_clean_room::process;
use shploingus_clean_room::reference;
use shploingus_clean_room::refusal::RefusalKind;
use shploingus_clean_room::stage::FieldId;
use shploingus_clean_room::von::{self, Classifier};
use shploingus_clean_room::world::Tick;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("shploingus-clean-u12-it-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

/// The one world the row's checks read: the declared fields filled into their **cubic world** (§38), read
/// through the one reading every consumer shares — the plane the being's need is met through.
fn world() -> WorldRead {
    let roster = process::roster().expect("the declared roster");
    let identity = reference::identity().expect("the reference identity");
    let request = process::declared_request();
    let patch = request.patch().expect("the declared request is a patch");
    let ladder = reference::ladder().expect("the reference world's own ladder");
    let values = fields_at(
        &roster,
        identity,
        Tick(0),
        &[FieldId::Elevation, FieldId::WaterLevel, FieldId::Lithology],
        request,
        1,
    )
    .expect("the three declared fields answer in one evaluation");
    let cubic = CubicWorld::from_surface(patch, &values[0], &values[1], &values[2], &ladder)
        .expect("the declared surface fills a cubic world");
    WorldRead::over_cubic(Organisation::first(), "the first mob", &cubic)
        .expect("the world's own cells are readable")
}

fn soul(dir: &Path, cell: usize) -> PathBuf {
    let record = dir.join("being.md");
    std::fs::write(
        &record,
        format!(
            "---\nrecord: being\nbeing: the first mob\norganisation: the undertow congregation\ncell: {cell}\n---\n\n\
             this being is a member of the undertow congregation. Its need is water and its acts are its own.\n"
        ),
    )
    .expect("the being's own record is written");
    record
}

fn classifier() -> Classifier {
    Classifier::classified().expect("the sixteen labelled readings calibrate the classifier")
}

/// The row's check: a member acts, and an act it may not do is refused **by name** and **recorded**.
#[test]
fn a_member_acts_and_an_act_it_may_not_do_is_refused_by_name_and_recorded() {
    let dir = scratch("act");
    let world = world();
    let start = being::start_cell(&world, 8).expect("the world's own law names a start cell");
    let mut mob = Being::of(
        "the first mob",
        Organisation::first(),
        &soul(&dir, start),
        dir.join("drafts"),
        8,
    )
    .expect("a being with a claim is ensouled");

    // The charter, read over the whole closed action vocabulary: the member reaches the record and the
    // world's own state, and nothing that writes the world down.
    let rows = mob.organisation().charter_read("the first mob").expect("the charter reads");
    assert_eq!(rows.len(), Action::ALL.len(), "every declared act is asked about");
    let reaches: Vec<&str> = rows.iter().filter(|(_, ok)| *ok).map(|(name, _)| *name).collect();
    assert!(reaches.contains(&"append_event") && reaches.contains(&"read_world"));
    assert!(!reaches.contains(&"save_world") && !reaches.contains(&"delete_content"));

    // The drive: the world's own flow law, then a drink where the world's own water stands.
    let classifier = classifier();
    let root = von::root();
    let mut acts = Vec::new();
    for _ in 0..8 {
        match mob.tick(&world, &classifier, &root).expect("the drive is not refused") {
            act @ Act::Drink { .. } => {
                acts.push(act);
                break;
            }
            act => acts.push(act),
        }
    }
    let walked = acts.iter().filter(|act| matches!(act, Act::Walk { .. })).count();
    let drank = acts.iter().find_map(|act| match act {
        Act::Drink { cell, depth_um } => Some((*cell, *depth_um)),
        _ => None,
    });
    let (cell, depth_um) = drank.expect("the being reaches the world's own water");
    assert!(walked >= 1, "the being walks the game's own flow law rather than being moved");
    assert!(world.water_stands(cell), "it drinks where the world's own filled surface stands");
    assert_eq!(depth_um, world.depth_um(cell), "the depth is the world's own reading");
    assert!(!mob.thirsty(), "the need is a fold over the being's own records, not a meter");
    assert_eq!(mob.cell(), Some(cell), "the being's cell is read off its own records");
    assert_eq!(mob.chain().len(), walked + 1, "one governed event per act, and no others");

    // The act it may not do: refused by name, and on the chain before the caller sees it.
    let before = mob.refusals();
    let denial = mob.attempt_act_it_may_not_do().expect_err("the world write is not the member's");
    assert_eq!(denial.kind(), RefusalKind::PolicyDenied);
    assert!(denial.detail().contains("may not `save_world`"), "the refusal quotes the act: {denial}");
    assert!(denial.detail().contains("declared scopes are"), "the refusal quotes the list: {denial}");
    assert_eq!(mob.refusals(), before + 1, "the denial is recorded on the one chain");
    assert!(mob
        .chain()
        .events()
        .iter()
        .any(|event| event.kind() == shploingus_clean_room::chain::EventKind::RefusalRecorded
            && event.payload().contains("policy_denied")));
    assert!(mob.verify().is_ok(), "the being's own chain re-walks: every link, every digest");
}

/// The three refusals the being's own drive can meet, each by name and each recorded.
#[test]
fn the_world_and_the_record_refuse_the_being_by_name() {
    let dir = scratch("refusals");
    let world = world();
    let classifier = classifier();
    let root = von::root();

    // 1a. It stands where the world's own routing carries nowhere: a dry sink, and the sink's own refusal.
    let sink = being::dry_sink(&world).expect("the world has a dry sink");
    let mut stranded = Being::of(
        "the first mob",
        Organisation::first(),
        &soul(&dir, sink),
        dir.join("stranded"),
        4,
    )
    .expect("the being is ensouled");
    let refusal = stranded.tick(&world, &classifier, &root).expect_err("a need the world cannot meet");
    assert_eq!(refusal.kind(), RefusalKind::WaterUnreachable);
    assert!(refusal.detail().contains("sink"), "the sink's own branch answers: {refusal}");
    assert_eq!(stranded.refusals(), 1, "the refusal is recorded");

    // 1b. Its reach is nothing, on a dry cell the world's own routing *would* carry to water: the reach guard's
    // own refusal, which the sink's branch cannot answer for.
    let reachless = being::start_cell(&world, 8).expect("a dry cell the routing carries");
    let mut spent = Being::of(
        "the first mob",
        Organisation::first(),
        &soul(&dir, reachless),
        dir.join("spent"),
        0,
    )
    .expect("the being is ensouled");
    let refusal = spent.tick(&world, &classifier, &root).expect_err("a reach spent before a step");
    assert_eq!(refusal.kind(), RefusalKind::WaterUnreachable);
    assert!(refusal.detail().contains("whole reach"), "the reach guard's own branch answers: {refusal}");
    assert_eq!(spent.refusals(), 1, "the refusal is recorded");

    // 2. An act below the floor its charter declares: the stricter organisation files it.
    let start = being::start_cell(&world, 8).expect("a start cell");
    let mut guild = Being::of(
        "the first mob",
        Organisation::stricter(),
        &soul(&dir, start),
        dir.join("guild"),
        8,
    )
    .expect("the being is ensouled");
    assert_eq!(Organisation::stricter().floor(), "applied-internal");
    // The same draft the first organisation takes reads below this charter's floor: measured at the seam.
    let own = dir.join("own.md");
    std::fs::write(
        &own,
        format!(
            "---\nrecord: being-act\nbeing: the first mob\ndependencies: []\nact: walk\ncell: {start}\n---\n\n# Change\n\none act.\n"
        ),
    )
    .expect("written");
    let (reading, _, _) = being::stage_of(Organisation::first(), "the first mob", &own, &classifier, &root)
        .expect("the draft is read");
    assert_eq!(reading, Organisation::first().floor());
    assert_ne!(reading, Organisation::stricter().floor(), "the guild's floor is above the reading");
    let refusal = guild.tick(&world, &classifier, &root).expect_err("an act below the floor");
    assert_eq!(refusal.kind(), RefusalKind::StageBelowFloor);
    assert_eq!(guild.refusals(), 1, "the refusal is recorded");

    // 3. A being whose records cannot be read: no claim, so no being — refused with von's own word.
    let hollow = dir.join("hollow.md");
    std::fs::write(&hollow, "record: being\nno fence at all\n").expect("written");
    let refusal = Being::of("the first mob", Organisation::first(), &hollow, dir.join("hollow"), 4)
        .expect_err("a being without a claim is refused");
    assert_eq!(refusal.kind(), RefusalKind::NoReading);
    let claimless = dir.join("claimless.md");
    std::fs::write(&claimless, "---\nrecord: being\nbeing: the first mob\n---\n\n").expect("written");
    let refusal = Being::of("the first mob", Organisation::first(), &claimless, dir.join("claimless"), 4)
        .expect_err("a being with an empty claim is refused");
    assert_eq!(refusal.kind(), RefusalKind::NoReading);
}

/// The organisation's own vocabulary stays **owed**: the office rank and the certification bands are
/// `axis.rs`'s owed ladders, and this pass read the smallest reading rather than inventing a rung.
#[test]
fn the_organisations_vocabulary_stays_owed_by_name() {
    for (axis, document) in [
        (AxisKind::OrgRank, "CIVILIAN-PORT"),
        (AxisKind::Certification, "CIVILIAN-PORT"),
    ] {
        match axis.declared_values() {
            LadderValues::Owed(names) => {
                assert!(names.contains(document), "{} owes its tokens to {names}", axis.name())
            }
            LadderValues::Declared(values) => panic!(
                "{}'s tokens were invented here ({values:?}) — the vocabulary is U2's owed T016",
                axis.name()
            ),
        }
    }
    // And the office a member holds is the organisation's own name for it, never a rung of that ladder.
    assert_eq!(Organisation::first().office(), "tidewarden");
    assert_eq!(Organisation::first().floor(), "verified-internal");
    assert_eq!(Organisation::first().charter(), "docs/CIVILIAN-PORT.md");
    // The office is not a bare ladder token in disguise: no ladder claims the word, so the bare-token rule is
    // not this word's business at all — which is exactly what an organisation's own name for an office is.
    assert!(shploingus_clean_room::axis::refuse_bare(Organisation::first().office()).is_ok());
}

/// Determinism: the run is a pure function of the world's own cells and the being's own record, so two runs
/// produce one chain digest — the plan's own bar, on the being's chain.
#[test]
fn two_runs_of_the_first_mob_produce_one_chain_digest() {
    let world = world();
    let classifier = classifier();
    let root = von::root();
    let start = being::start_cell(&world, 8).expect("a start cell");
    let mut digests = Vec::new();
    for tag in ["a", "b"] {
        let dir = scratch(&format!("det-{tag}"));
        let mut mob = Being::of(
            "the first mob",
            Organisation::first(),
            &soul(&dir, start),
            dir.join("drafts"),
            8,
        )
        .expect("the being is ensouled");
        while !matches!(
            mob.tick(&world, &classifier, &root).expect("the drive is not refused"),
            Act::Drink { .. }
        ) {}
        digests.push(being::chain_digest(mob.chain()).to_hex());
    }
    assert_eq!(digests[0], digests[1], "two runs, one digest: {digests:?}");
    assert_eq!(digests[0].len(), 64);
    // And the world the two runs read is one reading too — the surface read's own digest, which is a reading
    // of a cubic world's cells and not the world's identity (§37, `LEDGER.md` C-189).
    let cells = being::surface_digest(&world).to_hex();
    let again = self::world();
    assert_eq!(being::surface_digest(&again).to_hex(), cells);
}
