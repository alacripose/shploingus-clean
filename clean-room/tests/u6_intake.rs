//! **The end-to-end check for U6's content gate — a declared row, through the public surface, with
//! real files on disk.**
//!
//! Derived from `specs/005-u6-record-api/spec.md` (its `admit`/`inspect` boundary) and
//! `specs/005-u6-record-api/plan.md` (the content gate's first slice). Everything here goes through
//! the **public** API: this test is one of the two doors (`src/main.rs`'s evidence is the other), so a
//! capability that only exists inside the crate's tests would not be measured by it. Each test writes
//! its documents under the system temp directory in a directory of its own, so a parallel run cannot
//! collide and no fixture is ever committed.

use std::path::{Path, PathBuf};

use shploingus_clean_room::digest::Digest;
use shploingus_clean_room::intake::{
    Acceptance, Axis, Carrier, Claim, Establishment, Intake, Licence, Requirement, Requirements, Row,
    Source, WorldDeclarations,
};
use shploingus_clean_room::material::MaterialId;
use shploingus_clean_room::process;
use shploingus_clean_room::reference;
use shploingus_clean_room::refusal::RefusalKind;
use shploingus_clean_room::stage::Declared;

/// A directory this test owns, fresh each run.
fn fixture(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("shploingus-u6-intake-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the fixture directory is creatable");
    dir
}

/// A declared reference to a document **written where it lives**: the path is relative to the gate's
/// root and the digest is read off the bytes rather than restated by hand.
fn declare(dir: &Path, name: &str, bytes: &[u8]) -> Source {
    std::fs::write(dir.join(name), bytes).expect("the document reaches the disk");
    Source::new(name, Digest::of(bytes))
}

/// A row that is complete except for what each test varies: a permitted licence, a declared carrier
/// and a provided acceptance check.
fn row(dir: &Path, feature: &str, claim: Claim) -> Row {
    let source = declare(dir, "source.md", b"# a declared source document\n");
    let acceptance = Acceptance::Provided(declare(dir, "check.md", b"#[test] fn the_bar() {}\n"));
    Row::new(
        feature,
        source,
        Some(Licence::Permitted),
        Some("U4"),
        Some(acceptance),
        claim,
        Requirements::nothing(),
    )
}

/// **An intake over the world**: the caller's tree, conformed against the two declarations the binary
/// itself serves — the roster (the stages a row may require), which the caller owns for the test, and
/// the process's one reference registry (the materials it may require). A roster assembled here would
/// be a second, similar world, and a conformance verdict about *it* would say nothing about the world a
/// served row meets.
fn intake_over<'a>(dir: &'a Path, roster: &'a Declared) -> Intake<'a> {
    Intake::over(dir, WorldDeclarations { roster, registry: reference::registry() })
}

/// The reference roster, admitted by name: the world every intake in this file conforms against. A
/// refusal here is the file's own failure and says which name refused.
fn the_world() -> Declared {
    process::roster().expect("the reference roster admits")
}

#[test]
fn an_admitted_document_reports_what_it_became() {
    let dir = fixture("admitted");
    let roster = the_world();
    let intake = intake_over(&dir, &roster);

    // A checksum-only row: integrity is established and **no identity is proven** — the verdict says
    // which, rather than a boolean that means "the checksum matched".
    let admitted = intake
        .admit(&row(&dir, "a declared feature", Claim::ChecksumOnly))
        .expect("a complete row is admitted");
    assert!(!admitted.establishes_identity(), "a checksum is not a signature");
    assert_eq!(admitted.signer(), None);
    assert_eq!(admitted.claim_name(), "checksum_only");
    assert_eq!(
        admitted.found(),
        Digest::of(b"# a declared source document\n"),
        "the digest is read back out of the document, not restated from the row"
    );

    // Every axis is reported, and this report is the whole of what the gate knows.
    assert_eq!(admitted.findings().len(), Axis::COUNT);
    for axis in Axis::ALL {
        assert!(
            admitted.findings().iter().any(|finding| finding.axis() == axis),
            "the report omits the {} axis",
            axis.name()
        );
    }
    let identity = admitted
        .findings()
        .iter()
        .find(|finding| finding.axis() == Axis::Identity)
        .expect("the identity axis is reported");
    assert_eq!(identity.state(), Establishment::Inactive);
    assert!(!identity.is_pass(), "an unproven identity is not a pass");
    assert!(
        identity.refusal().is_none(),
        "and it does not refuse: the verdict carries it"
    );
    let rendered = admitted.render();
    assert!(rendered.contains("identity NOT established"), "{rendered}");
    assert!(rendered.contains("5 of 7 axes established"), "{rendered}");

    // The same document, with a pin an operator named out of band: now identity **is** established,
    // and it is a different guarantee with a different name.
    let pin = Digest::of(b"# a declared source document\n");
    let pinned = intake
        .admit(&row(
            &dir,
            "a declared feature",
            Claim::pinned("an operator", pin),
        ))
        .expect("a pinned row is admitted");
    assert!(pinned.establishes_identity());
    assert_eq!(pinned.signer(), Some("an operator"));
    assert_eq!(pinned.claim_name(), "pinned");
    assert_eq!(
        pinned.state(Axis::Identity),
        Some(Establishment::Active),
        "identity included: a pin an operator stated out of band establishes it"
    );
    // The **one** axis that is not a pass is the one this row said nothing about: it requires nothing
    // of the world, so conformance is checked and reported rather than counted. Being reported is not
    // being refused — the row is admitted — and it is never read as a pass either.
    let not_passing: Vec<&str> = pinned
        .findings()
        .iter()
        .filter(|finding| !finding.is_pass())
        .map(|finding| finding.axis().name())
        .collect();
    assert_eq!(not_passing, vec!["conformance"], "{pinned:?}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_bytes_are_read_where_they_live_and_never_carried() {
    let dir = fixture("in-place");
    let roster = the_world();
    let intake = intake_over(&dir, &roster);
    let declared = b"the bytes the row describes";
    let acceptance = Acceptance::Provided(declare(&dir, "check.md", b"#[test] fn the_bar() {}\n"));
    let row = Row::new(
        "a declared feature",
        declare(&dir, "source.md", declared),
        Some(Licence::Permitted),
        Some("U4"),
        Some(acceptance),
        Claim::ChecksumOnly,
        Requirements::nothing(),
    );
    assert_eq!(row.source().path(), "source.md");
    assert!(intake.admit(&row).is_ok(), "the document is where the row says");

    // One byte different, and the **same row** is a different document: the gate reads it where it
    // lives, so a document that changed under a row's feet is refused rather than served from a copy.
    let changed = b"the bytes the row describes!";
    std::fs::write(dir.join("source.md"), changed).expect("the document changes");
    let refusals = intake
        .admit(&row)
        .expect_err("the row is no longer the row's document");
    assert!(refusals.contains(RefusalKind::SourceDigestMismatch), "{refusals}");
    let detail: String = refusals
        .all()
        .iter()
        .map(|refusal| refusal.detail().to_string())
        .collect::<Vec<String>>()
        .join(" | ");
    assert!(detail.contains(&Digest::of(declared).to_hex()), "{detail}");
    assert!(detail.contains(&Digest::of(changed).to_hex()), "{detail}");

    // And gone entirely, there is nothing to consume: the gate carries no bytes, so it cannot rebuild
    // the document it was pointed at.
    std::fs::remove_file(dir.join("source.md")).expect("the document is removed");
    let refusals = intake.admit(&row).expect_err("nothing to read in place");
    assert!(refusals.contains(RefusalKind::SourceAbsent), "{refusals}");
    // The refusal is **host-independent in its words**: it names the reference as the row declared it
    // and the operating system's classification in this crate's spelling — never this host's own error
    // text (`… cannot find the file specified. (os error 2)`) and never the path this host normalized.
    // A detail is not incidental: it reaches a response, a report and (through a refusal's record)
    // the record's own identity.
    let absent = refusals
        .all()
        .iter()
        .find(|refusal| refusal.kind() == RefusalKind::SourceAbsent)
        .expect("the absent source is named");
    assert!(absent.detail().contains("source.md"), "{absent}");
    assert!(!absent.detail().contains("os error"), "{absent}");
    assert!(
        !absent.detail().contains(&dir.display().to_string()),
        "the detail names the declared reference, not this host's path: {absent}"
    );
    assert_eq!(
        intake.inspect(&row).state(Axis::Source),
        Some(Establishment::Inactive),
        "checked, and not there"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_undeclared_name_and_an_unchecked_axis_are_two_different_refusals() {
    let dir = fixture("undeclared");
    let roster = the_world();
    let intake = intake_over(&dir, &roster);

    // One row that answers the source and nothing else, and names a carrier the gate does not declare.
    let source = declare(&dir, "source.md", b"# a declared source document\n");
    let row = Row::new(
        "a declared feature",
        source,
        None,
        Some("U9"),
        None,
        Claim::ChecksumOnly,
        Requirements::nothing(),
    );
    let inspection = intake.inspect(&row);

    // The gate declares the axes and the carriers, so "not declared" and "not checked" are two facts
    // with two names — and both are in this one report.
    assert_eq!(inspection.state(Axis::Source), Some(Establishment::Active));
    assert_eq!(inspection.state(Axis::Carrier), Some(Establishment::Refused));
    assert_eq!(inspection.state(Axis::Licence), Some(Establishment::Unchecked));
    assert_eq!(inspection.state(Axis::Acceptance), Some(Establishment::Unchecked));
    let refusals = inspection.refusals();
    assert!(refusals.contains(RefusalKind::CarrierUndeclared), "{refusals}");
    assert!(refusals.contains(RefusalKind::AxisUnchecked), "{refusals}");
    assert_ne!(
        RefusalKind::CarrierUndeclared.name(),
        RefusalKind::AxisUnchecked.name(),
        "an undeclared name is not an unchecked axis"
    );
    assert!(
        refusals.len() >= 3,
        "one undeclared carrier and two unchecked axes: {refusals}"
    );

    // The undeclared refusal says which name was not declared *and* what the gate does declare, and
    // the unchecked one says what was never consulted.
    let words: String = refusals
        .all()
        .iter()
        .map(|refusal| format!("{}: {}", refusal.kind().name(), refusal.detail()))
        .collect::<Vec<String>>()
        .join(" | ");
    assert!(words.contains("U9"), "{words}");
    assert!(words.contains("U8"), "{words}");
    assert!(words.contains("licence"), "{words}");

    // The carrier set is a value a caller can read, and it is closed.
    assert!(Carrier::parse("U4").is_some());
    assert!(Carrier::parse("U9").is_none(), "U9 is admitted by the plan, not a declared carrier");
    assert_eq!(Carrier::ALL.len(), Carrier::COUNT);

    let report = inspection.render();
    assert!(report.contains("carrier refused"), "{report}");
    assert!(report.contains("unchecked"), "{report}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_refused_row_is_not_partially_admitted_and_every_failing_axis_reports() {
    let dir = fixture("refused");
    let roster = the_world();
    let intake = intake_over(&dir, &roster);

    // Every axis fails, and in a different way: no feature, a source that is not there, a refused
    // licence, an undeclared carrier, an owed check, a pin that names another release, and two
    // requirements the world does not declare.
    let row = Row::new(
        "   ",
        Source::new("missing.md", Digest::of(b"a document that is not there")),
        Some(Licence::Refused),
        Some("NOPE"),
        Some(Acceptance::Owed),
        Claim::pinned("an operator", Digest::of(b"a different release")),
        // Two requirements the world does not declare: this row requires both dimensions and neither
        // is held, and each is named rather than folded into one negative.
        Requirements {
            stages: Some(Requirement::Named(vec!["no-such-stage".to_string()])),
            materials: Some(Requirement::Named(vec![MaterialId(4242)])),
        },
    );
    let refusals = intake.admit(&row).expect_err("nothing here is admitted");

    // The checks are independent and none short-circuits: **every** failing axis is named, so an
    // author fixes one round of them.
    assert!(refusals.contains(RefusalKind::AxisUnchecked), "the unnamed feature: {refusals}");
    assert!(refusals.contains(RefusalKind::SourceAbsent), "{refusals}");
    assert!(refusals.contains(RefusalKind::LicenceRefused), "{refusals}");
    assert!(refusals.contains(RefusalKind::CarrierUndeclared), "{refusals}");
    assert!(refusals.contains(RefusalKind::AxisInactive), "the owed check: {refusals}");
    assert!(refusals.contains(RefusalKind::StageUndeclared), "the missing stage: {refusals}");
    assert!(refusals.len() >= 7, "seven failing axes: {refusals}");

    // And the owed check is displayed by name rather than counted as a pass.
    let owed = intake
        .inspect(&row)
        .finding(Axis::Acceptance)
        .expect("the acceptance axis is reported")
        .clone();
    assert_eq!(owed.state(), Establishment::Inactive);
    assert!(owed.subject().contains("owed"), "{owed}");

    // A pin that names another payload refuses by its own name, and says both digests.
    let source = declare(&dir, "source.md", b"# a declared source document\n");
    let mismatched = Row::new(
        "a declared feature",
        source,
        Some(Licence::Permitted),
        Some("U4"),
        Some(Acceptance::Owed),
        Claim::pinned("an operator", Digest::of(b"a different release")),
        Requirements::nothing(),
    );
    let refusals = intake.admit(&mismatched).expect_err("the pin is not the payload");
    assert!(refusals.contains(RefusalKind::PinMismatch), "{refusals}");
    let words: String = refusals
        .all()
        .iter()
        .map(|refusal| refusal.detail().to_string())
        .collect::<Vec<String>>()
        .join(" | ");
    assert!(words.contains(&Digest::of(b"a different release").to_hex()), "{words}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_census_never_passes_an_unchecked_cell() {
    let dir = fixture("census");
    let roster = the_world();
    let intake = intake_over(&dir, &roster);

    let complete = row(&dir, "a complete row", Claim::ChecksumOnly);
    let silent = Row::new(
        "a row that answers nothing but its source",
        declare(&dir, "silent.md", b"# a second declared source\n"),
        None,
        None,
        None,
        Claim::ChecksumOnly,
        Requirements::nothing(),
    );
    let census = intake.census(&[complete, silent]);

    assert_eq!(census.rows(), 2);
    assert_eq!(census.cells().len(), 2 * Axis::COUNT, "one cell per row and axis");
    assert_eq!(
        census.admitted(),
        1,
        "a row with an unchecked axis is not admitted, so the gate cannot grade only what it checked"
    );
    assert!(!census.is_established());
    assert!(census.count(Establishment::Unchecked) >= 3);
    assert!(census.refusals().contains(RefusalKind::AxisUnchecked), "{}", census.refusals());

    let grid = census.render();
    assert!(grid.contains("census: 2 declared row(s) over 7 axes"), "{grid}");
    assert!(grid.contains("admitted 1 of 2"), "{grid}");
    assert!(grid.contains("[a complete row]"), "{grid}");
    assert!(grid.contains("unchecked"), "{grid}");

    let _ = std::fs::remove_dir_all(&dir);
}

/// **Conformance is against the world's own declarations.** A row that requires a stage the roster runs
/// and a material the registry admits conforms; a stage nobody declares refuses `stage_undeclared`
/// **with the stage named**, and a material the registry never admitted refuses with the registry's own
/// words. Neither is a bare negative, and neither is a list this gate keeps.
#[test]
fn conformance_is_decided_against_the_worlds_own_declarations() {
    let dir = fixture("conformance");
    let roster = the_world();
    let intake = intake_over(&dir, &roster);
    let a_stage = roster.stages()[0].name;
    let requiring = |requirements: Requirements| {
        let acceptance =
            Acceptance::Provided(declare(&dir, "check.md", b"#[test] fn the_bar() {}\n"));
        Row::new(
            "a declared feature",
            declare(&dir, "source.md", b"# a declared source document\n"),
            Some(Licence::Permitted),
            Some("U4"),
            Some(acceptance),
            Claim::ChecksumOnly,
            requirements,
        )
    };

    // The world declares both, so the axis is **established** — and it says which two things it
    // resolved rather than that it is satisfied.
    let conforming = requiring(Requirements {
        stages: Some(Requirement::Named(vec![a_stage.to_string()])),
        materials: Some(Requirement::Named(vec![MaterialId(1)])),
    });
    let inspection = intake.inspect(&conforming);
    assert_eq!(
        inspection.state(Axis::Conformance),
        Some(Establishment::Active),
        "the roster runs the stage and the registry admits the material"
    );
    let finding = inspection.finding(Axis::Conformance).expect("the axis is reported");
    assert!(finding.subject().contains(a_stage), "{finding}");
    assert!(inspection.refusals().is_empty(), "{}", inspection.refusals());

    // A stage nobody declares: refused by **name**, with the stage and the roster's own names in the
    // message — never a bare "not conforming".
    let stage_missing = requiring(Requirements {
        stages: Some(Requirement::Named(vec!["no-such-stage".to_string()])),
        ..Requirements::nothing()
    });
    let refusals = intake.admit(&stage_missing).expect_err("no such stage is declared");
    assert!(refusals.contains(RefusalKind::StageUndeclared), "{refusals}");
    let words = refusals
        .first_named()
        .expect("the refusal is named")
        .detail()
        .to_string();
    assert!(words.contains("no-such-stage"), "{words}");
    assert!(words.contains(a_stage), "the roster's own names are in it: {words}");

    // A material the registry never admitted: the refusal is the **registry's own**, passed through.
    let material_missing = requiring(Requirements {
        materials: Some(Requirement::Named(vec![MaterialId(4242)])),
        ..Requirements::nothing()
    });
    let refusals = intake.admit(&material_missing).expect_err("the registry has no such material");
    assert!(refusals.contains(RefusalKind::UnadmittedMaterial), "{refusals}");
    assert!(refusals.first_named().expect("named").detail().contains("4242"), "{refusals}");

    // The states that are not a pass stay distinct: requiring nothing is *reported* (checked, and
    // nothing to find), a bar the row declares and owes is inactive under the axis's own name, and one
    // it declares and leaves unnamed is unchecked — the axis nothing consulted. None of the three is
    // ever a pass.
    let states = [
        (requiring(Requirements::nothing()), Establishment::Inactive, false),
        (
            requiring(Requirements {
                stages: Some(Requirement::Owed),
                ..Requirements::nothing()
            }),
            Establishment::Inactive,
            true,
        ),
        (
            requiring(Requirements {
                stages: Some(Requirement::Named(Vec::new())),
                ..Requirements::nothing()
            }),
            Establishment::Unchecked,
            true,
        ),
    ];
    for (row, state, refuses) in states {
        let inspection = intake.inspect(&row);
        assert_eq!(inspection.state(Axis::Conformance), Some(state), "{inspection:?}");
        let axis = inspection.finding(Axis::Conformance).expect("reported");
        assert!(!axis.is_pass(), "not a pass: {axis}");
        assert_eq!(
            inspection.refusals().contains(RefusalKind::AxisInactive)
                || inspection.refusals().contains(RefusalKind::AxisUnchecked),
            refuses,
            "a bar that cannot be run refuses; nothing required does not: {inspection:?}"
        );
    }

    let _ = std::fs::remove_dir_all(&dir);
}
