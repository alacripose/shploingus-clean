//! **U7 proved through the public surface**: the seven axes, the judge's three questions, the ballot, the
//! calibrated classifier and the four fail-closed rules.
//!
//! Nothing here reaches inside a module: every value comes back out of the API the binary and a client use.
//! The files this file reads are the record's own — the sixteen labelled `hyperskill/*` readings, the 39
//! records in `patches/`, and copies it writes into a scratch root when a *perturbed* record is what a guard
//! has to bite on. **No file of the tree's own is ever written by this suite.**
//!
//! The bar this file holds is the plan's own (`docs/CLEAN-ROOM-PLAN-2026-10-11.md` §5 row 6 and its U7
//! block): *a ballot answered with `Origin::Oracle`; the calibration accuracy beats the majority class* —
//! with `AxisKind` and the three prohibitions, `Question::{Choice,Score,Noul}`, the three-word `Verdict`,
//! `Ballot`, `StructuralJudge`, and the four fail-closed rules each biting **by name**.

use std::path::{Path, PathBuf};

use shploingus_clean_room::axis::{
    bare_token_axes, check_coverage, refuse_bare, refuse_interchange, refuse_reduction, AxisKind,
    CoverageRow, EvidenceGrade, Ladder, Origin, Reduction, COVERAGE,
};
use shploingus_clean_room::digest::Digest;
use shploingus_clean_room::laya::{
    answer, Ballot, Judgement, Measurement, Question, StructuralJudge, Unreachable, Verdict,
};
use shploingus_clean_room::refusal::{Refusal, RefusalKind};
use shploingus_clean_room::von::{
    claimed_vocabulary, claimed_vocabulary_at, declared_facts, declared_status, head_of, Classifier,
    KNOWN_ANSWERS, STAGES,
};

/// The record the demonstration ballots are about: `PATCH-015` declares `approval: user-confirmed`, a word
/// this corpus carries only for the two `approved` records.
const THE_RECORD: &str = "patches/PATCH-015-hypergrilling.md";

fn record_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn text_of(path: &str) -> String {
    std::fs::read_to_string(record_root().join(path)).expect("a record the calibration names")
}

/// A scratch root with one file in it — the only place a perturbed record is ever written.
fn scratch(name: &str, file: &str, body: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("u7-decision-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&root).expect("a scratch directory");
    std::fs::write(root.join(file), body).expect("the probe");
    root
}

fn a_question() -> Question {
    Question::choice(&STAGES, "approved").expect("five options and one of them passes")
}

fn a_ballot(id: &str, artifact: &str) -> Ballot {
    Ballot::declare(
        id,
        "the plan's §5 row 6",
        artifact,
        "read the record's own head and say which stage of its life it is at",
        a_question(),
        "the clean room's build",
        24,
    )
    .expect("a complete ballot")
}

/// A judge whose calibration **ties** the majority class rather than beating it — the first rule's own case,
/// implemented here rather than reached for inside the crate.
struct Tied {
    figures: Measurement,
}

impl StructuralJudge for Tied {
    fn judge(&self, _question: &Question, _artifact: &[u8]) -> Result<Judgement, Refusal> {
        Ok(Judgement {
            chosen: "proposed".to_string(),
            scores: vec![("proposed".to_string(), 1.0)],
            features: Vec::new(),
        })
    }

    fn measurement(&self) -> &Measurement {
        &self.figures
    }
}

fn tied_judge() -> Tied {
    Tied {
        figures: Measurement {
            readings: 16,
            per_option: vec![("proposed", 5)],
            majority: 5.0 / 16.0,
            in_sample: 5.0 / 16.0,
            leave_one_out: 5.0 / 16.0,
            leave_one_out_with_absence: 0.0,
            leave_one_out_from_the_claim_only: 0.0,
            known_answers: Digest::of_parts(&[]),
        },
    }
}

// --- the seven axes, and the three ways reading one is a bug ------------------------------------------

#[test]
fn the_coverage_table_describes_every_axis_exactly_once_and_refuses_a_row_that_is_untrue() {
    check_coverage(&COVERAGE).expect("every axis is described exactly once");
    assert_eq!(COVERAGE.len(), AxisKind::COUNT, "one row per axis, and no more");

    let forged = CoverageRow {
        axis: AxisKind::EvidenceGrade,
        ladder: "the reference's letter grade",
        values: &["A", "B", "C", "E"],
        source: "a forged row",
    };
    let refusals = check_coverage(&[forged]).expect_err("`E` is not a declared grade");
    assert!(refusals.contains(RefusalKind::FabricatedAxis), "{refusals}");

    let partial: Vec<CoverageRow> = COVERAGE
        .iter()
        .copied()
        .filter(|row| row.axis != AxisKind::OrgRank)
        .collect();
    let refusals =
        check_coverage(&partial).expect_err("an axis with no row is a failure, not a todo");
    assert!(refusals.contains(RefusalKind::UncoveredAxis), "{refusals}");
    assert!(
        refusals.all().iter().any(|refusal| refusal.detail().contains("org_rank")),
        "{refusals}"
    );
}

#[test]
fn a_cross_ladder_comparison_is_refused_and_two_members_of_one_ladder_compare_freely() {
    let refusal = refuse_interchange(EvidenceGrade::A, Origin::Oracle)
        .expect_err("`A` is a grade and an origin, and one ladder is not the other");
    assert_eq!(refusal.kind(), RefusalKind::AxisInterchangeRefused);
    refuse_interchange(EvidenceGrade::A, EvidenceGrade::D).expect("one ladder compares with itself");
    refuse_interchange(Origin::Oracle, Origin::ReadBack).expect("one ladder compares with itself");
}

#[test]
fn no_axis_may_be_rendered_as_a_number() {
    let mut refused = 0;
    for axis in AxisKind::ALL {
        for reduction in [Reduction::Percentage, Reduction::Score] {
            assert_eq!(
                refuse_reduction(axis, reduction).kind(),
                RefusalKind::ScalarReductionRefused
            );
            refused += 1;
        }
    }
    assert_eq!(refused, AxisKind::COUNT * 2, "every axis, both ways of reducing it");
}

#[test]
fn a_bare_ladder_token_is_refused_by_its_own_name() {
    let claimants = bare_token_axes("A");
    assert!(
        claimants.len() > 1,
        "`A` is a declared token of more than one ladder, which is why a bare one is ambiguous"
    );
    let refusal = refuse_bare("A").expect_err("a bare `A` names no ladder");
    assert_eq!(refusal.kind(), RefusalKind::BareTokenRefused);
    assert!(refusal.detail().contains("AMBIGUOUS"), "{refusal}");

    let refusal = refuse_bare("SSS").expect_err("exactly one ladder claims it and that is still a bug");
    assert_eq!(refusal.kind(), RefusalKind::BareTokenRefused);
    assert!(refusal.detail().contains("UNQUALIFIED"), "{refusal}");

    refuse_bare("gold").expect("no declared ladder claims it, so this rule is not about it");
    refuse_bare("grade A").expect("a token with its ladder named is not bare");
}

// --- the ballot, answered by the calibrated classifier ------------------------------------------------

#[test]
fn a_ballot_is_answered_by_the_oracle_and_carries_what_it_read() {
    let classifier = Classifier::classified().expect("the sixteen readings are all there");
    let answered = answer(&a_ballot("u7-the-record", THE_RECORD), &classifier, &record_root(), None)
        .expect("a calibrated instrument answers");

    assert_eq!(answered.origin(), Origin::Oracle, "the origin is the proof's to carry");
    assert_eq!(answered.origin().token(), "A", "a model's answer is the A rung");
    assert_eq!(answered.verdict(), Verdict::Belongs);
    assert_eq!(answered.passed(), Ok(true));

    let evidence = answered.evidence();
    assert!(
        evidence.features.contains(&"approval_is_user_confirmed"),
        "the answer carries what it read: {:?}",
        evidence.features
    );
    assert_eq!(evidence.readings, 16, "the calibration's size travels with the answer");
    assert_eq!(evidence.leave_one_out, classifier.figures().leave_one_out);
    assert!(evidence.digest != Digest::of_parts(&[]), "the artifact's own bytes are digested");
}

#[test]
fn the_calibration_beats_the_majority_class_and_the_claim_alone_does_not() {
    let classifier = Classifier::classified().expect("calibrated");
    let figures = classifier.figures();

    assert_eq!(figures.readings, KNOWN_ANSWERS.len());
    assert!(figures.beats_majority(), "{}", figures.render());
    assert!(figures.quotable(), "{}", figures.render());
    assert!(figures.leave_one_out > figures.majority, "{}", figures.render());
    // The measured finding behind the design: the stage is in the record's head, not in its claim.
    assert!(figures.leave_one_out_from_the_claim_only < figures.majority, "{}", figures.render());
    // Every reading's label is a declared option, and one reading at least per option.
    for stage in STAGES {
        assert!(KNOWN_ANSWERS.iter().any(|(_, label)| *label == stage), "{stage} is uncalibrated");
    }
}

#[test]
fn the_declared_vocabulary_is_read_over_the_whole_tree_and_not_only_the_calibration_set() {
    let vocabulary = claimed_vocabulary().expect("the tree's records are readable");
    assert!(vocabulary.records >= 39, "the corpus is smaller than the tree: {}", vocabulary.records);
    assert!(vocabulary.refusals.is_empty(), "{}", vocabulary.refusals);
    assert_eq!(
        vocabulary.families.len(),
        8,
        "every declared word is read, whether or not a record carries it"
    );
    let outside: usize = vocabulary.families.iter().map(|family| family.outside).sum();
    assert!(
        outside > 0,
        "a claim that only ever held over the calibration set would be a fitted claim"
    );
    let applications = vocabulary
        .families
        .iter()
        .find(|family| family.word == "applied-to-internal-base")
        .expect("the word is in the corpus");
    assert_eq!(
        applications.allowed,
        &["applied-internal", "verified-internal"],
        "the head's word for an application never accompanies a proposed or designed record"
    );
}

#[test]
fn a_record_whose_head_contradicts_its_own_vocabulary_is_refused_by_name() {
    // A record that **is** a record, with a head that contradicts a word of its own vocabulary: it declares
    // an application and reads `designed`, which no record in the tree does.
    let root = std::env::temp_dir().join(format!("u7-decision-{}-contradiction", std::process::id()));
    let directory = root.join("patches");
    std::fs::create_dir_all(&directory).expect("a record directory");
    std::fs::write(
        directory.join("PATCH-999-contradiction.md"),
        "---\npatch_id: PATCH-999\nstatus: designed\noperation: add\napproval: applied-to-internal-base\n\
         source: failed-eval\n---\n\n# Change\n\nsomething\n",
    )
    .expect("the probe");
    let vocabulary = claimed_vocabulary_at(&root).expect("the directory is readable");
    assert_eq!(vocabulary.records, 1);
    assert!(vocabulary.refusals.contains(RefusalKind::ClaimUnverified), "{}", vocabulary.refusals);
    let named = vocabulary
        .refusals
        .all()
        .iter()
        .any(|refusal| refusal.detail().contains("applied-to-internal-base"));
    assert!(named, "the refusal names the word that was contradicted: {}", vocabulary.refusals);

    let absent = claimed_vocabulary_at(&root.join("nowhere")).expect_err("no directory, no corpus");
    assert!(absent.contains(RefusalKind::SourceAbsent), "{absent}");
}

// --- the four fail-closed rules -----------------------------------------------------------------------

#[test]
fn an_uncalibrated_instrument_writes_nothing() {
    let refusals = answer(&a_ballot("u7-uncalibrated", THE_RECORD), &tied_judge(), &record_root(), None)
        .expect_err("a calibration that ties its own majority class is not quotable");
    assert!(refusals.contains(RefusalKind::NotEstablished), "{refusals}");
    assert!(
        refusals.all().iter().any(|refusal| refusal.detail().contains("0.3125")),
        "the refusal carries both figures: {refusals}"
    );
}

#[test]
fn a_missing_instrument_is_refused_rather_than_read_as_a_pass() {
    let refusals = answer(
        &a_ballot("u7-unreachable", THE_RECORD),
        &Unreachable { what: "a model-backed instrument" },
        &record_root(),
        None,
    )
    .expect_err("an absent instrument is not an empty answer");
    assert!(refusals.contains(RefusalKind::OracleUnreachable), "{refusals}");
    assert!(
        refusals.all().iter().any(|refusal| refusal.detail().contains("a model-backed instrument")),
        "{refusals}"
    );
}

#[test]
fn a_non_reading_is_an_answer_with_no_verdict_and_no_floor_can_be_read_off_it() {
    let root = scratch(
        "non-reading",
        "not-a-record.md",
        "# a description, not a record\n\nprose about a thing.\n",
    );
    let classifier = Classifier::classified().expect("calibrated");
    let answered = answer(&a_ballot("u7-non-reading", "not-a-record.md"), &classifier, &root, None)
        .expect("a non-reading is an answer");
    assert_eq!(answered.verdict(), Verdict::NoReading);
    let refusal = answered.passed().expect_err("a non-reading is never a pass");
    assert_eq!(refusal.kind(), RefusalKind::NoReading);
}

#[test]
fn a_not_belongs_reading_needs_a_humans_reason_in_the_same_act() {
    // The same record under a floor it does not clear: the reading is `approved` and the floor says the
    // answer must be `proposed`.
    let question = Question::choice(&STAGES, "proposed").expect("five options");
    let ballot = Ballot::declare(
        "u7-off-the-floor",
        "the plan's §5 row 6",
        THE_RECORD,
        "read the record's own head and say which stage of its life it is at",
        question,
        "the clean room's build",
        24,
    )
    .expect("a complete ballot");
    let classifier = Classifier::classified().expect("calibrated");

    let refusals = answer(&ballot, &classifier, &record_root(), None)
        .expect_err("a negative reading filed bare is a fact a successor cannot act on");
    assert!(refusals.contains(RefusalKind::ReasonAbsent), "{refusals}");

    let answered = answer(
        &ballot,
        &classifier,
        &record_root(),
        Some("the record's own stage is later than the floor this ballot declares"),
    )
    .expect("with a reason the finding is filed");
    assert_eq!(answered.verdict(), Verdict::NotBelongs);
    assert_eq!(answered.passed(), Ok(false));
    assert_eq!(
        answered.evidence().reason.as_deref(),
        Some("the record's own stage is later than the floor this ballot declares"),
        "the reason travels with the finding"
    );
}

#[test]
fn an_artifact_that_is_gone_is_refused_rather_than_reconstructed() {
    let refusals = answer(&a_ballot("u7-absent", "patches/PATCH-000-gone.md"), &tied_judge(), &record_root(), None)
        .expect_err("the ballot judges bytes where they live");
    // Rule 1 is asked first only in the order the rules are *listed*; the artifact is read before the
    // instrument is asked, because a ballot about bytes that are not there has nothing to ask about.
    assert!(refusals.contains(RefusalKind::SourceAbsent), "{refusals}");
    assert!(
        refusals.all().iter().any(|refusal| refusal.detail().contains("PATCH-000-gone.md")),
        "{refusals}"
    );
}

#[test]
fn an_uncalibrated_question_is_refused_by_name_rather_than_answered_on_anothers_calibration() {
    // **The audit's own case, closed at the seam.** A `noul` ballot is asked of the shipped instrument and
    // must not be filed as a verdict carrying the **stage** question's figures: this build ships no
    // known-answer set for a `noul` question, so the question is refused by name (`not_established`) rather
    // than answered on a measurement that was never made for it (`docs/LEDGER.md` R-59.5).
    let classifier = Classifier::classified().expect("calibrated");
    let question = Question::noul(0.5).expect("the natural midpoint");
    let ballot = Ballot::declare(
        "u7-noul-uncalibrated",
        "the plan's §5 row 6",
        THE_RECORD,
        "is this record a decision or a description?",
        question,
        "the clean room's build",
        24,
    )
    .expect("a complete ballot");
    let refusals = answer(&ballot, &classifier, &record_root(), None)
        .expect_err("no `noul` set exists, so the question is refused rather than quoted");
    assert!(refusals.contains(RefusalKind::NotEstablished), "{refusals}");
    assert!(
        refusals.all().iter().any(|refusal| refusal.detail().contains("noul")),
        "the refusal names the question it holds no calibration for: {refusals}"
    );
}

// --- the label is never a feature, and a moved one is stale rather than wrong -------------------------

#[test]
fn a_moved_label_cannot_move_the_reading() {
    let classifier = Classifier::classified().expect("calibrated");
    let text = text_of(THE_RECORD);
    let question = a_question();
    let original = classifier
        .judge(&question, text.as_bytes())
        .expect("the record's head declares the facts this instrument reads");

    // The record's own declared stage, moved in a *copy*: nothing about the claim or the head's other fields
    // changes, so a reading that moved would be a reading of the label.
    let moved = text.replace("status: approved", "status: proposed");
    assert_ne!(moved, text, "the probe must actually move the label");
    assert_eq!(declared_status(&moved), Some("proposed"), "the harness does read the label");
    let moved_reading = classifier
        .judge(&question, moved.as_bytes())
        .expect("the same facts are declared after the label moves");
    assert_eq!(original.chosen, moved_reading.chosen, "the label is not evidence");
    assert_eq!(original.scores, moved_reading.scores, "and it cannot move a score either");
    assert_eq!(original.features, moved_reading.features);

    // ...and a stored label that no longer matches the record is **stale rather than wrong**.
    let root = scratch("stale", "stale.md", &moved);
    let refusals = Classifier::declared_labels(&root, &[("stale.md", "approved")]);
    assert!(refusals.contains(RefusalKind::ClaimUnverified), "{refusals}");
    assert!(
        refusals.all()[0].detail().contains("stale rather than wrong"),
        "{refusals}"
    );
}

#[test]
fn the_sixteen_labelled_readings_are_the_records_own_declared_stages() {
    let refusals = Classifier::labels_describe_the_records();
    assert!(refusals.is_empty(), "the stored labels describe the records: {refusals}");
    for (path, label) in KNOWN_ANSWERS {
        assert_eq!(declared_status(&text_of(path)), Some(label), "{path} does not read `{label}`");
    }
}

#[test]
fn the_head_is_read_through_one_reader_and_the_label_is_not_a_feature() {
    let text = text_of(THE_RECORD);
    let head = head_of(&text).expect("a record has a head");
    assert!(head.contains("approval: user-confirmed"), "the head is the region between the fences");
    let facts = declared_facts(head);
    assert!(facts.contains(&shploingus_clean_room::von::Declared::ApprovalIsAUserConfirmation));
    let named: Vec<&str> = facts.iter().map(|fact| fact.name()).collect();
    assert!(!named.iter().any(|name| name.contains("status")), "{named:?}");
    // A document that is not a record has no head at all, so it declares no stage.
    assert_eq!(head_of("# a description\n"), None);
    assert_eq!(declared_status("# a description\nstatus: approved\n"), None);
}
