//! **U7's classifier: the structure a record's own body declares, read as a stage of its life.**
//!
//! Derived from **documents** (Constitution P1/P7):
//!
//! | Source | What it declares here |
//! |---|---|
//! | `docs/PLAN-2026-10-10-clean-room-and-the-api-layer.md` Lane C | what the classifier is *for*: nothing in the tree could tell a **decision** from a **description**, and a decision is a text with *"a subject, an evidence, a rollback"*. Those three are three of the declared [`Feature`]s, and they are the whole of the second instrument ([`Classifier::judge`] on a `noul` question). |
//! | `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §2 (U7) | the **calibration set**: *the sixteen shared `hyperskill/*` ids at five life stages, which are **labelled examples**, not code*. In this tree those sixteen are `patches/PATCH-001` … `PATCH-016` — the records whose `target` is `hyperskill/*` and which name no path in this tree at all (`patches/PATCH-025-the-patch-coverage-join.md`, `patch-unnamed-records 16`) — each carrying its own **stage** in the one field this module's features never read. |
//! | `patches/PATCH-039-the-corpus-beyond-this-tree.md` | the five stages, named in the record's own order: `proposed`, `designed`, `verified-internal`, `applied-internal`, `approved`. |
//! | `docs/LEDGER.md` R-59.5 and §59.5 item 12 | the bar and the honesty rule: **the calibration bar is the majority class, not a fixed number**, the known-answer set **may not be edited to manufacture a pass**, and the measurement is reported either way. |
//! | `docs/LEDGER.md` C-102 §102.2 | why the majority class is *measured*: an unpaired set is how a constant judge comes to beat the bar. Here the bar is read **strictly**, so a judge that always answers the commonest stage scores exactly the majority class and does not beat it. |
//! | `docs/LEDGER.md` R-59.7 | the pattern every declared table here follows: an exhaustive match, a total pin, a guard that compares a claim against the thing it claims. |
//! | `docs/HYPERGRILLING.md` H-14.1 | *the model instrument is still calibrated and quotable only when it beats the majority class* — [`crate::laya::Measurement::quotable`]. |
//!
//! # The two readers, and why there are two
//!
//! **The classifier never sees a record's head.** [`body_of`] returns the bytes after the frontmatter fence
//! and nothing else, so no feature can reach the field that carries the label: the reading is of the
//! record's own *claim*, and a test perturbs the label to prove the reading does not move.
//!
//! The **harness** reads the head for one purpose: [`declared_status`] lets the calibration check that a
//! stored label is still the stage the record declares for itself, so a record whose stage moves makes this
//! calibration **stale rather than wrong** — [the convention]: the tree's other readings.
//!
//! [the convention]: Classifier::labels_describe_the_records
//!
//! # The measured fact about the set, which the report does not smooth over
//!
//! **Two of the sixteen open with a byte-order mark** (`PATCH-007`, `PATCH-008`) — a fence is a fence with
//! or without it, and a reader that missed this read two records as bodiless and would have reported a
//! calibration over fourteen. The BOM is stripped **once**, in one place ([`body_of`]), because two readers
//! disagreeing about where a body starts is the second-parser failure this register forbids.
//!
//! And the corpus is thin: the sixteen bodies differ in almost nothing but length (measured — every one
//! declares its four sections and quotes its own vocabulary). So the fitted table is reported **with the
//! feature that carries it named**, and the figure below is a reading of sixteen records rather than a claim
//! about records in general.

use std::path::{Path, PathBuf};

use crate::digest::Digest;
use crate::gameplay::record_root;
use crate::laya::{Judgement, Measurement, Question, StructuralJudge};
use crate::refusal::{Refusal, RefusalKind, RefusalSet};

/// **The five life stages a decision passes through** — the question's options, in the order the chain runs
/// them (`patches/PATCH-039-the-corpus-beyond-this-tree.md`).
pub const STAGES: [&str; 5] = [
    "proposed",
    "designed",
    "verified-internal",
    "applied-internal",
    "approved",
];

/// **The sixteen labelled readings** — the calibration set the plan names, declared as `(path, stage)`.
///
/// The path is what is read **in place**; the stage is the label, and the harness checks it against the
/// record's own declared stage rather than trusting it
/// ([`Classifier::labels_describe_the_records`]). These sixteen are the whole set: *the known-answer set may
/// not be edited to manufacture a pass* (`docs/LEDGER.md` R-59.5), so it is a `const` with no constructor
/// that could grow it at measurement time.
pub const KNOWN_ANSWERS: [(&str, &str); 16] = [
    ("patches/PATCH-001-scientific-method-loop.md", "proposed"),
    ("patches/PATCH-002-digital-paper-evidence.md", "proposed"),
    ("patches/PATCH-003-portable-theory-project.md", "proposed"),
    ("patches/PATCH-004-adaptive-context-budget.md", "proposed"),
    ("patches/PATCH-005-documentation-diagram-tickets.md", "proposed"),
    ("patches/PATCH-006-diagram-freshness-lifecycle.md", "designed"),
    ("patches/PATCH-007-retirement-before-completion.md", "verified-internal"),
    ("patches/PATCH-008-concise-first-response.md", "verified-internal"),
    ("patches/PATCH-009-exact-rollback-evidence.md", "applied-internal"),
    ("patches/PATCH-010-context-budget-integrity.md", "verified-internal"),
    ("patches/PATCH-011-honest-host-process-evidence.md", "verified-internal"),
    ("patches/PATCH-012-standalone-publication.md", "designed"),
    ("patches/PATCH-013-hyperskill-identity.md", "designed"),
    ("patches/PATCH-014-sparse-semantic-diffusion.md", "designed"),
    ("patches/PATCH-015-hypergrilling.md", "approved"),
    ("patches/PATCH-016-mantis-security-core.md", "approved"),
];

/// **The declared features** — every one of them a structural fact of the record format itself, read off the
/// body. The list is closed and it is the *only* list: the classifier's numbers are a function of these
/// predicates, so a reader can re-derive any answer without the classifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Feature {
    /// The body declares `# Change` — Lane C's *subject*.
    DeclaresChange,
    /// The body declares `# Acceptance` — what the work will be held to.
    DeclaresAcceptance,
    /// The body declares `# Evidence` — Lane C's *evidence*.
    DeclaresEvidence,
    /// The body declares `# Rollback` — Lane C's *rollback*, the part a bare description lacks.
    DeclaresRollback,
    /// The body says what is **owed** (`owed`, or a `# Not done` heading) — a record that knows its own gap.
    StatesWhatIsOwed,
    /// The body cites **another record** by id (`PATCH-…`).
    CitesAnotherRecord,
    /// The body names a **path** in this tree.
    NamesAPath,
    /// The body carries a **table row** — a figure set out to be read rather than described.
    CarriesATable,
    /// The body carries a **command** (`cargo …`): something a reader can re-run.
    CarriesACommand,
    /// The body reports a **figure**: a `=` with digits after it, or an arrow between two things.
    ReportsAFigure,
    /// The body is **long** (30 lines or more).
    LongBody,
    /// The body is **short** (20 lines or fewer).
    ShortBody,
    /// The body carries a **subheading** (`## …`).
    HasASubheading,
    /// The body **quotes the record's own vocabulary** in backticks.
    QuotesTheRecord,
}

/// How many features this build declares. Written down, and pinned to [`Feature::ALL`] by a test.
pub const FEATURE_COUNT: usize = 14;

impl Feature {
    /// Every feature, in declaration order — the order the table's columns are in.
    pub const ALL: [Feature; FEATURE_COUNT] = [
        Feature::DeclaresChange,
        Feature::DeclaresAcceptance,
        Feature::DeclaresEvidence,
        Feature::DeclaresRollback,
        Feature::StatesWhatIsOwed,
        Feature::CitesAnotherRecord,
        Feature::NamesAPath,
        Feature::CarriesATable,
        Feature::CarriesACommand,
        Feature::ReportsAFigure,
        Feature::LongBody,
        Feature::ShortBody,
        Feature::HasASubheading,
        Feature::QuotesTheRecord,
    ];

    /// The recorded name — the column a report prints.
    pub const fn name(self) -> &'static str {
        match self {
            Feature::DeclaresChange => "declares_change",
            Feature::DeclaresAcceptance => "declares_acceptance",
            Feature::DeclaresEvidence => "declares_evidence",
            Feature::DeclaresRollback => "declares_rollback",
            Feature::StatesWhatIsOwed => "states_what_is_owed",
            Feature::CitesAnotherRecord => "cites_another_record",
            Feature::NamesAPath => "names_a_path",
            Feature::CarriesATable => "carries_a_table",
            Feature::CarriesACommand => "carries_a_command",
            Feature::ReportsAFigure => "reports_a_figure",
            Feature::LongBody => "long_body",
            Feature::ShortBody => "short_body",
            Feature::HasASubheading => "has_a_subheading",
            Feature::QuotesTheRecord => "quotes_the_record",
        }
    }

    /// What the feature reads, in one line — this is what a reader checks against the bytes.
    pub const fn describes(self) -> &'static str {
        match self {
            Feature::DeclaresChange => "the body declares a `# Change` section",
            Feature::DeclaresAcceptance => "the body declares a `# Acceptance` section",
            Feature::DeclaresEvidence => "the body declares a `# Evidence` section",
            Feature::DeclaresRollback => "the body declares a `# Rollback` section",
            Feature::StatesWhatIsOwed => "the body says what is owed (`owed`, or `# Not done`)",
            Feature::CitesAnotherRecord => "the body cites another record by id (`PATCH-…`)",
            Feature::NamesAPath => "the body names a path in this tree",
            Feature::CarriesATable => "the body carries a table row",
            Feature::CarriesACommand => "the body carries a command a reader can re-run",
            Feature::ReportsAFigure => "the body reports a figure",
            Feature::LongBody => "the body runs to 30 lines or more",
            Feature::ShortBody => "the body runs to 20 lines or fewer",
            Feature::HasASubheading => "the body carries a `## …` subheading",
            Feature::QuotesTheRecord => "the body quotes the record's own vocabulary in backticks",
        }
    }

    /// **The exhaustive match.** A feature with no predicate here fails to compile, which is the point: a
    /// column nobody reads is a number in a table that looks measured.
    pub fn holds(self, body: &str) -> bool {
        let mut lines = body.lines();
        match self {
            Feature::DeclaresChange => lines.any(|line| heading(line) == Some("Change")),
            Feature::DeclaresAcceptance => lines.any(|line| heading(line) == Some("Acceptance")),
            Feature::DeclaresEvidence => lines.any(|line| heading(line) == Some("Evidence")),
            Feature::DeclaresRollback => lines.any(|line| heading(line) == Some("Rollback")),
            Feature::StatesWhatIsOwed => {
                lines.any(|line| heading(line) == Some("Not done"))
                    || body.to_lowercase().contains("owed")
            }
            Feature::CitesAnotherRecord => body.contains("PATCH-"),
            Feature::NamesAPath => ["crates/", "docs/", "src/", "clean-room/"]
                .iter()
                .any(|root| body.contains(root)),
            Feature::CarriesATable => lines.any(|line| line.trim_start().starts_with("| ")),
            Feature::CarriesACommand => body.contains("cargo "),
            Feature::ReportsAFigure => lines.any(|line| {
                line.contains('→')
                    || line
                        .split('=')
                        .skip(1)
                        .any(|after| after.trim_start().chars().next().is_some_and(|c| c.is_ascii_digit()))
            }),
            Feature::LongBody => body.lines().count() >= 30,
            Feature::ShortBody => body.lines().count() <= 20,
            Feature::HasASubheading => lines.any(|line| line.starts_with("## ") && heading(line).is_some()),
            Feature::QuotesTheRecord => body.contains('`'),
        }
    }
}

/// **The head's own declared facts** — the second, smaller feature list, and the one the stage question is
/// answered from.
///
/// Every variant reads one **key the record format declares** (the corpus of `patches/*.md` is where the
/// format is written down: 39 records, each opening with the same keys) and either its word from that key's
/// own declared vocabulary, or a fact about its shape. The **label's key is not here**: `status` is read by
/// [`declared_status`] for the staleness check and by no feature, so the reading cannot restate the answer.
///
/// **Why the head, and not only the claim.** The first draft of this module read the *body* alone, on the
/// reasoning that a reading of the claim is the honest thing to measure. The measurement said otherwise:
/// over the sixteen labelled records the body-only classifier scores **0.0000** leave-one-out against a
/// majority class of **0.3125**, because the sixteen claims are structurally the same document (measured —
/// every one declares the same four sections and quotes the record's own vocabulary, and only their length
/// varies). The stage is not in the claim; it is in the record's own life facts, which is exactly what
/// `patches/PATCH-039-the-corpus-beyond-this-tree.md` says of this corpus: *"the same id at five different
/// stages of its life, across five trees … answerable per record, from ids and heads alone"*. Both readings
/// are measured and both are printed ([`Measurement`]); the classifier answers from the head.
///
/// **Each word-feature is a vocabulary entry, not a fitted value.** The declared words are the ones the
/// corpus uses, and each is read over **all 39 records in the tree**, not only the sixteen labelled ones:
/// [`claimed_vocabulary`] is that reading, printed by the gate, and it fails by name if a record's own head
/// accompanies a word with a stage the word's claim does not allow.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Declared {
    /// `operation: add` — the head declares the work as an addition.
    OperationIsAdd,
    /// `operation: strengthen` — a follow-up cut against a base that already carries the work.
    OperationIsStrengthen,
    /// `operation: replace` — something is being taken out and put back.
    OperationIsReplace,
    /// `live_mutation: true` — the record says the change has already touched a live tree.
    LiveMutationDeclared,
    /// `dependencies: []` — a record that stands on nothing.
    DeclaresNoDependency,
    /// `approval: design-approved`.
    ApprovalIsADesignApproval,
    /// `approval: applied-to-internal-base` — the approval names an application.
    ApprovalNamesAnApplication,
    /// `approval: user-selected`.
    ApprovalIsAUserSelection,
    /// `approval: user-confirmed`.
    ApprovalIsAUserConfirmation,
    /// `approval: user-directed`.
    ApprovalIsAUserDirection,
    /// The head's `source` names a user instruction (the vocabulary's `user-…` family).
    SourceIsAUserInstruction,
    /// The head's `source` names a failed evaluation — work that came out of a problem rather than a plan.
    SourceIsAFailedEvaluation,
}

/// How many facts the head declares. Written down, and pinned to [`Declared::ALL`] by a test.
pub const DECLARED_COUNT: usize = 12;

impl Declared {
    /// Every declared fact, in declaration order — the head's columns, before the claim's.
    pub const ALL: [Declared; DECLARED_COUNT] = [
        Declared::OperationIsAdd,
        Declared::OperationIsStrengthen,
        Declared::OperationIsReplace,
        Declared::LiveMutationDeclared,
        Declared::DeclaresNoDependency,
        Declared::ApprovalIsADesignApproval,
        Declared::ApprovalNamesAnApplication,
        Declared::ApprovalIsAUserSelection,
        Declared::ApprovalIsAUserConfirmation,
        Declared::ApprovalIsAUserDirection,
        Declared::SourceIsAUserInstruction,
        Declared::SourceIsAFailedEvaluation,
    ];

    /// The recorded name — the column a report prints.
    pub const fn name(self) -> &'static str {
        match self {
            Declared::OperationIsAdd => "operation_is_add",
            Declared::OperationIsStrengthen => "operation_is_strengthen",
            Declared::OperationIsReplace => "operation_is_replace",
            Declared::LiveMutationDeclared => "live_mutation_declared",
            Declared::DeclaresNoDependency => "declares_no_dependency",
            Declared::ApprovalIsADesignApproval => "approval_is_design_approved",
            Declared::ApprovalNamesAnApplication => "approval_is_applied_to_internal_base",
            Declared::ApprovalIsAUserSelection => "approval_is_user_selected",
            Declared::ApprovalIsAUserConfirmation => "approval_is_user_confirmed",
            Declared::ApprovalIsAUserDirection => "approval_is_user_directed",
            Declared::SourceIsAUserInstruction => "source_is_a_user_instruction",
            Declared::SourceIsAFailedEvaluation => "source_is_a_failed_evaluation",
        }
    }

    /// What the fact reads, in one line — what a reader checks against the bytes.
    pub const fn describes(self) -> &'static str {
        match self {
            Declared::OperationIsAdd => "the head declares `operation: add`",
            Declared::OperationIsStrengthen => "the head declares `operation: strengthen`",
            Declared::OperationIsReplace => "the head declares `operation: replace`",
            Declared::LiveMutationDeclared => "the head declares `live_mutation: true`",
            Declared::DeclaresNoDependency => "the head declares no dependency (`dependencies: []`)",
            Declared::ApprovalIsADesignApproval => "the head declares `approval: design-approved`",
            Declared::ApprovalNamesAnApplication => {
                "the head declares `approval: applied-to-internal-base`"
            }
            Declared::ApprovalIsAUserSelection => "the head declares `approval: user-selected`",
            Declared::ApprovalIsAUserConfirmation => "the head declares `approval: user-confirmed`",
            Declared::ApprovalIsAUserDirection => "the head declares `approval: user-directed`",
            Declared::SourceIsAUserInstruction => {
                "the head's `source` names a user instruction (`user-…`)"
            }
            Declared::SourceIsAFailedEvaluation => {
                "the head's `source` names a failed evaluation (`failed-eval…`)"
            }
        }
    }

    /// **The exhaustive match.** A declared fact with no predicate here fails to compile, which is the
    /// point: a column nobody reads is a number in a table that looks measured.
    pub fn holds(self, head: &str) -> bool {
        let word = |key: &str, expected: &str| head_field(head, key) == Some(expected);
        match self {
            Declared::OperationIsAdd => word("operation", "add"),
            Declared::OperationIsStrengthen => word("operation", "strengthen"),
            Declared::OperationIsReplace => word("operation", "replace"),
            Declared::LiveMutationDeclared => word("live_mutation", "true"),
            Declared::DeclaresNoDependency => head_field(head, "dependencies").is_none_or(|value| {
                let value = value.trim().trim_start_matches('[').trim_end_matches(']');
                value.trim().is_empty()
            }),
            Declared::ApprovalIsADesignApproval => word("approval", "design-approved"),
            Declared::ApprovalNamesAnApplication => word("approval", "applied-to-internal-base"),
            Declared::ApprovalIsAUserSelection => word("approval", "user-selected"),
            Declared::ApprovalIsAUserConfirmation => word("approval", "user-confirmed"),
            Declared::ApprovalIsAUserDirection => word("approval", "user-directed"),
            Declared::SourceIsAUserInstruction => {
                head_field(head, "source").is_some_and(|value| value.starts_with("user"))
            }
            Declared::SourceIsAFailedEvaluation => {
                head_field(head, "source").is_some_and(|value| value.starts_with("failed-eval"))
            }
        }
    }
}

/// The declared facts that hold for one head, in declaration order.
pub fn declared_facts(head: &str) -> Vec<Declared> {
    Declared::ALL
        .iter()
        .copied()
        .filter(|fact| fact.holds(head))
        .collect()
}

/// **The words the record format declares, and what each claims about a stage.**
///
/// Each row is one word of the format's own vocabulary (`operation`'s words, `approval`'s words, and the two
/// `source` families) with the stages a record carrying it may read. The claims are the ones
/// [`claimed_vocabulary`] measures over **every** record in `patches/` — the tree's 39, not only the sixteen
/// the calibration set carries — so a word whose meaning drifts, or a record whose head contradicts its own
/// vocabulary, is **refused by name** with the path, the word and the stage it read.
pub const VOCABULARY_CLAIMS: [(&str, &str, &[&str]); 8] = [
    (
        "strengthen",
        "a follow-up is applied-internal, verified-internal or applied",
        &["applied-internal", "verified-internal", "applied"],
    ),
    ("replace", "a replacement is a designed change", &["designed"]),
    (
        "design-approved",
        "a design approval is proposed or designed",
        &["proposed", "designed"],
    ),
    (
        "applied-to-internal-base",
        "a record whose approval names an application is applied-internal or verified-internal",
        &["applied-internal", "verified-internal"],
    ),
    ("user-confirmed", "a user confirmation is approved", &["approved"]),
    ("user-selected", "a user selection is designed", &["designed"]),
    (
        "user-directed",
        "a user direction is applied",
        &["applied"],
    ),
    (
        "failed-eval",
        "work that came out of a failed evaluation is applied-internal or verified-internal",
        &["applied-internal", "verified-internal"],
    ),
];

/// One declared word's reading: the word, the claim it makes, the stages it allows, and how many records in
/// the corpus carry it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Family {
    /// The declared word.
    pub word: &'static str,
    /// What the word claims about the stage, in one sentence.
    pub claim: &'static str,
    /// The stages a record carrying the word may read.
    pub allowed: &'static [&'static str],
    /// How many records in the corpus carry the word **and** were read against this claim.
    pub seen: usize,
    /// How many of those are **outside** the sixteen the calibration carries. A claim that only ever held
    /// over the calibration set would be a fitted claim; this is the count that says whether the word's
    /// meaning was measured on records the classifier was not calibrated on.
    pub outside: usize,
}

/// **The reading of the format's words over the tree's own records.**
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Vocabulary {
    /// How many records were read — the corpus, not the calibration set.
    pub records: usize,
    /// One row per declared word, in [`VOCABULARY_CLAIMS`]'s order.
    pub families: Vec<Family>,
    /// Every word a record contradicted, each named: the path, the word, the stage it read.
    pub refusals: RefusalSet,
}

impl Vocabulary {
    /// The families a record actually carries — the ones with a count behind them, for the gate's line.
    pub fn carried(&self) -> Vec<&Family> {
        self.families.iter().filter(|family| family.seen > 0).collect()
    }
}

/// **Read every record in `patches/` and check each declared word against its claim.**
///
/// Refused ([`RefusalKind::SourceAbsent`]) only when the directory itself cannot be read: a record that
/// contradicts a claim is a *finding* carried in [`Vocabulary::refusals`], because a reading that stops at
/// the first bad record measures one record rather than the corpus.
pub fn claimed_vocabulary() -> Result<Vocabulary, RefusalSet> {
    claimed_vocabulary_at(&record_root())
}

/// The same read over an explicit root — the seam a test drives a perturbed record through without touching
/// the tree's own.
pub fn claimed_vocabulary_at(root: &Path) -> Result<Vocabulary, RefusalSet> {
    let directory = root.join("patches");
    let entries = match std::fs::read_dir(&directory) {
        Ok(entries) => entries,
        Err(error) => {
            let mut refusals = RefusalSet::new();
            refusals.refuse(
                RefusalKind::SourceAbsent,
                format!(
                    "the record directory `{}` is not there to be read in place: {error} — the words this \
                     reading measures are the records' own, so a corpus that cannot be read is refused \
                     rather than reconstructed from a list",
                    directory.display()
                ),
            );
            return Err(refusals);
        }
    };

    let mut paths: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("PATCH-") && name.ends_with(".md"))
        })
        .collect();
    paths.sort();

    let mut families: Vec<Family> = VOCABULARY_CLAIMS
        .iter()
        .map(|(word, claim, allowed)| Family {
            word,
            claim,
            allowed,
            seen: 0,
            outside: 0,
        })
        .collect();
    let mut vocabulary = Vocabulary::default();

    for path in &paths {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("a record")
            .to_string();
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) => {
                vocabulary.refusals.refuse(
                    RefusalKind::SourceAbsent,
                    format!("`{name}` is in the record directory and could not be read: {error}"),
                );
                continue;
            }
        };
        let Some(head) = head_of(&text) else {
            vocabulary.refusals.refuse(
                RefusalKind::NoReading,
                format!(
                    "`{name}` is named like a record and opens with no frontmatter fence, so it declares no \
                     head for this reading to check: a document that is not a record is not a counterexample, \
                     it is a non-reading"
                ),
            );
            continue;
        };
        let Some(stage) = declared_status(&text) else {
            vocabulary.refusals.refuse(
                RefusalKind::ClaimUnverified,
                format!(
                    "`{name}` declares no stage of its own, so the words its head carries cannot be read \
                     against anything: a record with no stage is a reading this check cannot make"
                ),
            );
            continue;
        };
        vocabulary.records += 1;
        for family in families.iter_mut() {
            let word = family.word;
            let carried = head_field(head, "operation") == Some(word)
                || head_field(head, "approval") == Some(word)
                || head_field(head, "source").is_some_and(|source| source.starts_with(word));
            if !carried {
                continue;
            }
            family.seen += 1;
            if !KNOWN_ANSWERS
                .iter()
                .any(|(labelled, _)| *labelled == format!("patches/{name}"))
            {
                family.outside += 1;
            }
            if !family.allowed.contains(&stage) {
                vocabulary.refusals.refuse(
                    RefusalKind::ClaimUnverified,
                    format!(
                        "`{name}` carries the declared word `{word}` and reads `{stage}`: the word claims \
                         {} — the claim is a property of the format, measured over every record in the tree, \
                         and a record that contradicts it is refused by name rather than counted as \
                         agreement",
                        family.claim
                    ),
                );
            }
        }
    }

    vocabulary.families = families;
    Ok(vocabulary)
}

/// The `# Heading` a line is, or nothing — the record format's own section heading. A `## ` subheading is
/// **not** one, which is why the subheading predicate asks about the prefix as well as this.
fn heading(line: &str) -> Option<&str> {
    let line = line.trim_end();
    line.strip_prefix("# ")
        .map(str::trim)
        .filter(|name| !name.is_empty())
}

/// **A record's two regions, in one read** — the head between the frontmatter's fences, and the body after
/// them. There is **one** reader of the fence in this module, so the head's facts and the body's claim cannot
/// come to disagree about where the record starts.
///
/// This is **not a parser**: it reads no key and knows no value. `None` when the text does not open with a
/// fence or has no closing one — a document that is not a record at all, and therefore has neither a head
/// nor a claim.
///
/// A leading byte-order mark is stripped **here and only here**: two of the sixteen readings open with one
/// (`PATCH-007`, `PATCH-008`, measured), and a reader that treated those two as bodiless would report a
/// calibration over fourteen records while printing sixteen.
pub fn regions(text: &str) -> Option<(&str, &str)> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut offset = 0usize;
    let mut opened = None;
    for line in text.split_inclusive('\n') {
        if line.trim_end_matches(['\n', '\r']).trim_end() == "---" {
            match opened {
                None => opened = Some(offset + line.len()),
                Some(start) => return Some((&text[start..offset], &text[offset + line.len()..])),
            }
        } else if opened.is_none() {
            // The first line is not a fence, so this text is not a record and has no regions.
            return None;
        }
        offset += line.len();
    }
    None
}

/// **Where a record's body starts** — the bytes after the frontmatter's closing fence.
pub fn body_of(text: &str) -> Option<&str> {
    regions(text).map(|(_, body)| body)
}

/// **Where a record's head is** — the bytes between the fences.
pub fn head_of(text: &str) -> Option<&str> {
    regions(text).map(|(head, _)| head)
}

/// **The value a head declares for one key**, first non-empty occurrence. `None` when the head does not
/// declare the key at all — which is the honest answer for a record whose head omits a field, and the reason
/// a feature read from an absent key never holds.
pub fn head_field<'a>(head: &'a str, key: &str) -> Option<&'a str> {
    let prefix = format!("{key}:");
    head.lines()
        .filter_map(|line| line.trim_end_matches('\r').strip_prefix(prefix.as_str()))
        .map(str::trim)
        .find(|value| !value.is_empty())
}

/// **The stage a record declares for itself** — the *harness's* reader, used for one thing only: checking
/// that a stored label still describes the record it was read from. It is [`head_field`] over [`head_of`]
/// and nothing else, and it is never a feature: the classifier reads the head's *other* fields, which is
/// what makes the reading a reading rather than a restatement.
pub fn declared_status(text: &str) -> Option<&str> {
    head_field(head_of(text)?, "status")
}

/// The features that hold for one body, in declaration order.
pub fn features_of(body: &str) -> Vec<Feature> {
    Feature::ALL
        .iter()
        .copied()
        .filter(|feature| feature.holds(body))
        .collect()
}

/// The head's columns, first in the declared feature space.
const HEAD_COLUMNS: usize = DECLARED_COUNT;

/// The claim's columns, after them. One space, two halves, so the same estimator can be asked both readings
/// and the two answers are comparable line for line.
const CLAIM_COLUMNS: usize = FEATURE_COUNT;

/// Every column this classifier can read: the head's declared facts, then the claim's own features.
const COLUMNS: usize = HEAD_COLUMNS + CLAIM_COLUMNS;

/// **One labelled reading**: the stage its record declares, the facts its head declares, and the features its
/// claim carries. Both lists are kept, because the two questions this instrument answers read different
/// halves of the record and because the measurement reports both readings rather than the flattering one.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Reading {
    stage: &'static str,
    declared: Vec<Declared>,
    claim: Vec<Feature>,
}

impl Reading {
    /// The head's columns, in declaration order — the query the stage question is answered from.
    fn head_columns(&self) -> Vec<usize> {
        self.declared.iter().map(|fact| declared_index(*fact)).collect()
    }

    /// The claim's columns, offset into the same space — the query the reported diagnostic is measured with.
    fn claim_columns(&self) -> Vec<usize> {
        self.claim.iter().map(|feature| HEAD_COLUMNS + feature_index(*feature)).collect()
    }

    /// **The columns a moving label could move.** The label's own key is read by no feature, so this is the
    /// list the label-perturbation test compares across a moved `status:`.
    fn columns(&self) -> Vec<usize> {
        let mut columns = self.head_columns();
        columns.extend(self.claim_columns());
        columns
    }
}

/// **The classifier**: a calibration fitted on declared readings, and the two questions it answers.
///
/// The fitted state is **the rows themselves**, and the counts are recomputed from them. That is what makes
/// leave-one-out honest: a fold removes one reading's row and recounts, with no fitted parameter anywhere
/// that could have seen the answer.
#[derive(Clone, Debug, PartialEq)]
pub struct Classifier {
    options: Vec<&'static str>,
    rows: Vec<Reading>,
    measurement: Measurement,
}

/// **Which features count as evidence.** A declared feature is a property of the body, checked once and
/// reported **when it holds** — so the instrument reads the held features, and a feature that does not hold
/// is simply silent about this body.
///
/// Reading the features that do *not* hold as evidence of absence as well is the other textbook form, and
/// over this corpus (fourteen declared features against sixteen readings, the commonest option holding five)
/// it inverts the one separation the set has: `LongBody` holds for both `approved` records, so its
/// absent-feature term penalises `approved` far more than the classes it must beat and the folds collapse onto
/// the majority option. Both figures are measured at calibration and **both are printed** ([`Measurement`]);
/// the instrument answers with the held features, and the other figure stands on the record beside it rather
/// than being dropped for being unflattering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Estimator {
    /// The features that hold are the evidence — the estimator the instrument answers with.
    Held,
    /// The features that do not hold are read as evidence of absence too.
    HeldAndAbsent,
}

impl Classifier {
    /// **Calibrate against the declared readings**, reading each artifact in place.
    ///
    /// Refused by name: an artifact that is not there ([`RefusalKind::SourceAbsent`]); a reading whose label
    /// is not one of the question's options ([`RefusalKind::OptionUncalibrated`], naming the label); an
    /// option **no reading calibrates** (the same refusal — a five-way question whose set never names one of
    /// its answers cannot be measured for that answer); and a record whose body cannot be read at all
    /// ([`RefusalKind::NoReading`], naming the path).
    pub fn calibrate(root: &Path, readings: &[(&str, &str)]) -> Result<Classifier, RefusalSet> {
        let mut refusals = RefusalSet::new();
        let mut rows: Vec<Reading> = Vec::with_capacity(readings.len());
        let mut digest_parts: Vec<Vec<u8>> = Vec::new();

        for (path, label) in readings {
            let bytes = match std::fs::read(root.join(path)) {
                Ok(bytes) => bytes,
                Err(error) => {
                    refusals.refuse(
                        RefusalKind::SourceAbsent,
                        format!(
                            "the reading `{path}` is not there to be read in place: {error} — a calibration \
                             is measured over the records this tree actually carries, and a missing one is \
                             refused rather than dropped from the count"
                        ),
                    );
                    continue;
                }
            };
            digest_parts.push(path.as_bytes().to_vec());
            digest_parts.push(label.as_bytes().to_vec());
            digest_parts.push(Digest::of(&bytes).as_bytes().to_vec());

            let Some(stage) = STAGES.iter().find(|stage| *stage == label).copied() else {
                refusals.refuse(
                    RefusalKind::OptionUncalibrated,
                    format!(
                        "the reading `{path}` is labelled `{label}`, which is not one of this question's \
                         options ({}) — a label outside the options cannot be scored, and rounding it to the \
                         nearest declared one would invent the answer",
                        STAGES.join(", ")
                    ),
                );
                continue;
            };
            let text = String::from_utf8_lossy(&bytes);
            let Some((head, body)) = regions(&text) else {
                refusals.refuse(
                    RefusalKind::NoReading,
                    format!(
                        "`{path}` is not a record at all: it opens with no frontmatter fence, so it has \
                         neither the head this instrument reads nor a claim, and an unreadable document is a \
                         non-reading rather than a stage"
                    ),
                );
                continue;
            };
            if body.trim().is_empty() {
                refusals.refuse(
                    RefusalKind::NoReading,
                    format!(
                        "`{path}` has no body after its frontmatter fence, so there is no claim here for \
                         the second instrument to read: a record with no claim cannot be a calibration \
                         reading, because one of the two questions this set calibrates would have nothing \
                         to measure"
                    ),
                );
                continue;
            }
            rows.push(Reading {
                stage,
                declared: declared_facts(head),
                claim: features_of(body),
            });
        }
        if !refusals.is_empty() {
            return Err(refusals);
        }

        let options: Vec<&'static str> = STAGES.to_vec();
        for option in &options {
            if !rows.iter().any(|reading| reading.stage == *option) {
                refusals.refuse(
                    RefusalKind::OptionUncalibrated,
                    format!(
                        "the option `{option}` has no reading in the known-answer set: the calibration \
                         cannot be measured for an answer it has never been shown, and a figure over the \
                         other {} would be a measurement of a question nobody asked",
                        options.len() - 1
                    ),
                );
            }
        }
        if !refusals.is_empty() {
            return Err(refusals);
        }

        let probe = Classifier {
            options: options.clone(),
            rows: rows.clone(),
            measurement: Measurement {
                readings: rows.len(),
                per_option: Vec::new(),
                majority: 0.0,
                in_sample: 0.0,
                leave_one_out: 0.0,
                leave_one_out_with_absence: 0.0,
                leave_one_out_from_the_claim_only: 0.0,
                known_answers: Digest::of_parts(&[]),
            },
        };
        let (counts, _, _) = probe.fit(None);
        let most = counts.iter().copied().max().unwrap_or(0);
        let majority = most as f64 / rows.len().max(1) as f64;
        // **The four figures, all of them over the same folds.** The answering reading is the head's facts;
        // the claim-only figure is the diagnostic that says where the stage is *not* (measured 0.0000 over
        // these sixteen); and the absence variant is the other textbook estimator, reported because the
        // record says a measurement is reported either way.
        let (in_correct, in_asked) = probe.accuracy(None, Estimator::Held, HEAD_COLUMNS);
        let (out_correct, out_asked) = probe.accuracy_all_folds(Estimator::Held, HEAD_COLUMNS);
        let (absent_correct, absent_asked) =
            probe.accuracy_all_folds(Estimator::HeldAndAbsent, HEAD_COLUMNS);
        let (claim_correct, claim_asked) = probe.accuracy_from_the_claim();
        let parts: Vec<&[u8]> = digest_parts.iter().map(|part| part.as_slice()).collect();

        Ok(Classifier {
            options: options.clone(),
            rows,
            measurement: Measurement {
                readings: readings.len(),
                per_option: options
                    .iter()
                    .zip(counts.iter())
                    .map(|(option, count)| (*option, *count))
                    .collect(),
                majority,
                in_sample: in_correct as f64 / in_asked.max(1) as f64,
                leave_one_out: out_correct as f64 / out_asked.max(1) as f64,
                leave_one_out_with_absence: absent_correct as f64 / absent_asked.max(1) as f64,
                leave_one_out_from_the_claim_only: claim_correct as f64 / claim_asked.max(1) as f64,
                known_answers: Digest::of_parts(&parts),
            },
        })
    }

    /// **Calibrate against the record's own sixteen readings**, at [`record_root`].
    pub fn classified() -> Result<Classifier, RefusalSet> {
        Classifier::calibrate(&record_root(), &KNOWN_ANSWERS)
    }

    /// The options this instrument answers between.
    pub fn options(&self) -> &[&'static str] {
        &self.options
    }

    /// The calibration's figures.
    pub fn figures(&self) -> &Measurement {
        &self.measurement
    }

    /// How many readings the fitted table holds.
    pub fn readings(&self) -> usize {
        self.rows.len()
    }

    /// **The check that makes a stored label stale rather than wrong**: every reading's stored stage against
    /// the stage the record declares for itself. An empty set means the calibration still describes the tree.
    pub fn labels_describe_the_records() -> RefusalSet {
        Classifier::declared_labels(&record_root(), &KNOWN_ANSWERS)
    }

    /// The same check over an explicit root and an explicit set — the seam a test drives a moved label
    /// through without touching the record's own.
    pub fn declared_labels(root: &Path, readings: &[(&str, &str)]) -> RefusalSet {
        let mut refusals = RefusalSet::new();
        for (path, label) in readings {
            match std::fs::read_to_string(root.join(path)) {
                Ok(text) => match declared_status(&text) {
                    Some(stage) if stage == *label => {}
                    Some(stage) => refusals.refuse(
                        RefusalKind::ClaimUnverified,
                        format!(
                            "`{path}` declares itself `{stage}` and this calibration stores `{label}`: the \
                             reading is stale rather than wrong, so the record is re-read and the set is \
                             **not** edited to fit it"
                        ),
                    ),
                    None => refusals.refuse(
                        RefusalKind::ClaimUnverified,
                        format!(
                            "`{path}` declares no status at all, so the stored label `{label}` has no \
                             witness in the record"
                        ),
                    ),
                },
                Err(error) => refusals.refuse(
                    RefusalKind::SourceAbsent,
                    format!("`{path}` could not be read to check its own declared stage: {error}"),
                ),
            }
        }
        refusals
    }

    /// The counts and per-column counts of each option, with one reading's row removed when `skip` names it.
    fn fit(&self, skip: Option<usize>) -> (Vec<usize>, Vec<[usize; COLUMNS]>, usize) {
        let mut counts = vec![0usize; self.options.len()];
        let mut holds = vec![[0usize; COLUMNS]; self.options.len()];
        let mut total = 0usize;
        for (index, reading) in self.rows.iter().enumerate() {
            if Some(index) == skip {
                continue;
            }
            let slot = self
                .options
                .iter()
                .position(|option| *option == reading.stage)
                .expect("a fitted row's stage is one of the options");
            counts[slot] += 1;
            total += 1;
            for column in reading.columns() {
                holds[slot][column] += 1;
            }
        }
        (counts, holds, total)
    }

    /// The log-posterior of one option for one query, over the fold that excludes `skip` — a Laplace-smoothed
    /// naive Bayes over the declared feature space, priors included: an artifact that carries no evidence at
    /// all falls back to the prior rather than to a guess, which is what a calibration is *for*.
    ///
    /// [`Estimator`] decides whether a column the query does not hold is read as well; `space` is how many
    /// columns may be read that way, so the absence variant is measured over one half of the record at a time
    /// rather than over both at once. The prior is in the sum either way, so what falls back is the prior and
    /// not a coin.
    fn posterior(
        &self,
        slot: usize,
        query: &[usize],
        skip: Option<usize>,
        estimator: Estimator,
        space: usize,
    ) -> f64 {
        let (counts, holds, total) = self.fit(skip);
        let options = self.options.len() as f64;
        let count = counts[slot];
        let mut log = ((count as f64 + 1.0) / (total as f64 + options)).ln();
        // **The columns this reading may speak about**, taken as a slice so the column index and its count
        // cannot drift apart: `space` is the half of the record being read, and the absence variant is
        // measured over one half at a time rather than over both at once.
        for (index, held) in holds[slot][..space.min(COLUMNS)].iter().enumerate() {
            let present = query.contains(&index);
            if !present && estimator == Estimator::Held {
                continue;
            }
            let rate = (*held as f64 + 1.0) / (count as f64 + 2.0);
            let weight = if present { rate } else { 1.0 - rate };
            if weight > 0.0 {
                log += weight.ln();
            }
        }
        log
    }

    /// Every option with its own score, in the question's order.
    fn scored(
        &self,
        query: &[usize],
        skip: Option<usize>,
        estimator: Estimator,
        space: usize,
    ) -> Vec<(String, f64)> {
        self.options
            .iter()
            .enumerate()
            .map(|(slot, option)| {
                ((*option).to_string(), self.posterior(slot, query, skip, estimator, space))
            })
            .collect()
    }

    /// The option the score lands on, or a **non-reading** — a tie is not an answer, and picking one would
    /// invent a boundary the record does not have (`docs/LEDGER.md` C-67's `DomainTie` reasoning, one layer
    /// over).
    fn reading(
        &self,
        query: &[usize],
        skip: Option<usize>,
        estimator: Estimator,
        space: usize,
    ) -> Result<String, Refusal> {
        let scores = self.scored(query, skip, estimator, space);
        let best = scores
            .iter()
            .map(|(_, score)| *score)
            .fold(f64::NEG_INFINITY, f64::max);
        let leaders: Vec<&String> = scores
            .iter()
            .filter(|(_, score)| (score - best).abs() < 1e-12)
            .map(|(option, _)| option)
            .collect();
        if leaders.len() != 1 {
            return Err(Refusal::new(
                RefusalKind::NoReading,
                format!(
                    "the reading is a tie between {}: two options supported equally is not an answer, and \
                     picking one would invent the boundary the record does not have",
                    leaders
                        .iter()
                        .map(|option| option.as_str())
                        .collect::<Vec<&str>>()
                        .join(" and ")
                ),
            ));
        }
        Ok(leaders[0].clone())
    }

    /// The fitted calibration's own recall: how many readings it answers correctly with every row present.
    fn accuracy(&self, skip: Option<usize>, estimator: Estimator, space: usize) -> (usize, usize) {
        let mut correct = 0usize;
        let mut asked = 0usize;
        for (index, reading) in self.rows.iter().enumerate() {
            if Some(index) == skip {
                continue;
            }
            asked += 1;
            if self
                .reading(&reading.head_columns(), skip, estimator, space)
                .map(|chosen| chosen == reading.stage)
                .unwrap_or(false)
            {
                correct += 1;
            }
        }
        (correct, asked)
    }

    /// **Leave-one-out**: every reading answered by a calibration fitted without it. This is the figure the
    /// bar is read against, because it is the only one that measures the instrument on a record it has not
    /// been shown. The query is always the **head's** columns — the half the stage question is answered from
    /// — so this is one measurement per estimator, not two questions added together.
    fn accuracy_all_folds(&self, estimator: Estimator, space: usize) -> (usize, usize) {
        let mut correct = 0usize;
        let mut asked = 0usize;
        for index in 0..self.rows.len() {
            let reading = &self.rows[index];
            asked += 1;
            if self
                .reading(&reading.head_columns(), Some(index), estimator, space)
                .map(|chosen| chosen == reading.stage)
                .unwrap_or(false)
            {
                correct += 1;
            }
        }
        (correct, asked)
    }

    /// **The claim-only reading** — the same folds, but asked from the record's *claim* instead of its head.
    /// It is measured because it is the finding: the sixteen claims are the same document and this reading
    /// scores 0.0000 against the same majority class, which is what moved the stage question to the head.
    fn accuracy_from_the_claim(&self) -> (usize, usize) {
        let mut correct = 0usize;
        let mut asked = 0usize;
        for index in 0..self.rows.len() {
            let reading = &self.rows[index];
            asked += 1;
            if self
                .reading(&reading.claim_columns(), Some(index), Estimator::Held, COLUMNS)
                .map(|chosen| chosen == reading.stage)
                .unwrap_or(false)
            {
                correct += 1;
            }
        }
        (correct, asked)
    }

    /// **Which declared feature carries a stage** — the fitted rate of each column for the option it most
    /// favours, printed by the gate so the figure is not a number without a mechanism. A column that **no**
    /// reading holds is not a carrier (its lift is an artefact of Laplace smoothing over an empty column, and
    /// printing it would name a mechanism that does not exist), and neither is a column whose best lift is
    /// zero. Ties in rate are reported for the option they most favour.
    pub fn carriers(&self) -> Vec<(&'static str, &'static str, f64)> {
        let (counts, holds, total) = self.fit(None);
        let mut carried = Vec::new();
        for index in 0..COLUMNS {
            if holds.iter().all(|row| row[index] == 0) {
                continue;
            }
            let mut best: Option<(usize, f64)> = None;
            for slot in 0..self.options.len() {
                let rate = (holds[slot][index] as f64 + 1.0) / (counts[slot] as f64 + 2.0);
                let overall = ((holds.iter().map(|row| row[index]).sum::<usize>() as f64) + 1.0)
                    / (total as f64 + 2.0);
                let lift = rate - overall;
                if best.is_none_or(|(_, best_lift)| lift > best_lift) {
                    best = Some((slot, lift));
                }
            }
            if let Some((slot, lift)) = best {
                if lift > 0.0 {
                    carried.push((self.options[slot], column_name(index), lift));
                }
            }
        }
        carried
    }
}

impl StructuralJudge for Classifier {
    /// **Answer the question** — and a shape this instrument does not read is a **non-reading**, never a
    /// verdict for the other instrument.
    fn judge(&self, question: &Question, artifact: &[u8]) -> Result<Judgement, Refusal> {
        let text = String::from_utf8_lossy(artifact);
        match question {
            Question::Choice { options, .. } => {
                if options.len() != self.options.len()
                    || options
                        .iter()
                        .zip(self.options.iter())
                        .any(|(asked, declared)| asked != declared)
                {
                    return Err(Refusal::new(
                        RefusalKind::NoReading,
                        format!(
                            "the question asks between {} and this instrument is calibrated between {}: a \
                             question it is not calibrated for is a non-reading, not an answer",
                            options.join(", "),
                            self.options.join(", ")
                        ),
                    ));
                }
            }
            // **The uncalibrated question is refused by name, not answered.** Lane C's three-part reading
            // (a subject, an evidence, a rollback) is computable from the claim, but this build ships **no
            // known-answer set for a `noul` question** — so there is no figure to beat a majority class and
            // nothing for an answer to rest on. Answering here would file a verdict carrying the *stage*
            // question's calibration: a measurement never made for this question, quotable on a standing it
            // did not earn (`docs/LEDGER.md` R-59.5). The set is owed; until it exists the question is
            // refused rather than answered.
            Question::Noul { threshold: _ } => {
                return Err(Refusal::new(
                    RefusalKind::NotEstablished,
                    "this instrument is calibrated for the stage question and for nothing else: a `noul` \
                     question has no known-answer set in this tree, so an answer would carry the stage \
                     question's figures and be quotable on a measurement never made for it — the \
                     uncalibrated question is refused by name (`not_established`), and its calibration set \
                     is owed"
                        .to_string(),
                ));
            }
            // A score question's rungs are owed by name, and an instrument that does not read rungs says so
            // with the third word rather than with another instrument's verdict.
            Question::Score { .. } => {
                return Err(Refusal::new(
                    RefusalKind::NoReading,
                    "this instrument answers a choice between stages and the three-part question; a score \
                     question's rungs are owed, and a verdict for another instrument would be the wrong \
                     answer wearing the right shape"
                        .to_string(),
                ));
            }
        }

        let Some(head) = head_of(&text) else {
            return Err(Refusal::new(
                RefusalKind::NoReading,
                "the artifact is not a record: it opens with no frontmatter fence, so it has no head and \
                 there is nothing here this question is about"
                    .to_string(),
            ));
        };
        let facts = declared_facts(head);
        if facts.is_empty() {
            return Err(Refusal::new(
                RefusalKind::NoReading,
                "the record's head declares none of the facts this instrument reads: a head with no \
                 declared operation, approval or source is a non-reading, because a guess at a stage would \
                 be an invented one"
                    .to_string(),
            ));
        }
        let columns: Vec<usize> = facts.iter().map(|fact| declared_index(*fact)).collect();
        let chosen = self.reading(&columns, None, Estimator::Held, HEAD_COLUMNS)?;
        Ok(Judgement {
            chosen,
            scores: self.scored(&columns, None, Estimator::Held, HEAD_COLUMNS),
            features: facts.iter().map(|fact| fact.name()).collect(),
        })
    }

    fn measurement(&self) -> &Measurement {
        &self.measurement
    }
}

/// The index of a declared fact in the declared order — the one place a fact becomes a column.
fn declared_index(fact: Declared) -> usize {
    Declared::ALL
        .iter()
        .position(|candidate| *candidate == fact)
        .expect("a fact is one of the declared facts")
}

/// The index of a claim feature in the declared order — the one place a feature becomes a column. It is
/// offset into the shared space by its caller, so the two halves cannot collide.
fn feature_index(feature: Feature) -> usize {
    Feature::ALL
        .iter()
        .position(|candidate| *candidate == feature)
        .expect("a feature is one of the declared features")
}

/// The name of a column in the shared space — the head's facts first, then the claim's features, which is the
/// order these columns were declared in rather than a second list that could drift from it.
fn column_name(index: usize) -> &'static str {
    match Declared::ALL.get(index) {
        Some(fact) => fact.name(),
        None => Feature::ALL
            .get(index.saturating_sub(HEAD_COLUMNS))
            .map(|feature| feature.name())
            .unwrap_or("undefclared"),
    }
}

/// The record root the readings are read from — [`record_root`]'s, because there is one answer to *where the
/// record's documents live* and a second one would be a second register.
pub fn root() -> PathBuf {
    record_root()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axis::{Ladder, Origin};
    use crate::laya::{answer, Ballot, Verdict};

    fn text_of(path: &str) -> String {
        std::fs::read_to_string(root().join(path)).expect("a record the calibration names")
    }

    fn scratch(name: &str, file: &str, body: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("u7-von-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&root).expect("a scratch directory");
        std::fs::write(root.join(file), body).expect("the probe");
        root
    }

    #[test]
    fn every_declared_feature_has_a_predicate_and_a_reading() {
        assert_eq!(Feature::ALL.len(), FEATURE_COUNT);
        for (index, feature) in Feature::ALL.iter().enumerate() {
            assert!(
                Feature::ALL[..index].iter().all(|other| other != feature),
                "{} is declared twice",
                feature.name()
            );
            assert!(!feature.describes().is_empty());
            assert!(
                feature.name().bytes().all(|byte| byte.is_ascii_lowercase() || byte == b'_'),
                "{} is not a wire-shaped name",
                feature.name()
            );
            // Every predicate answers, on a body with nothing in it.
            let _ = feature.holds("");
            let _ = feature.holds("# Change\n\nsomething\n");
        }
    }

    #[test]
    fn every_declared_fact_has_a_predicate_and_a_column() {
        assert_eq!(Declared::ALL.len(), DECLARED_COUNT);
        let mut columns: Vec<&str> = Vec::new();
        for (index, fact) in Declared::ALL.iter().enumerate() {
            assert!(
                Declared::ALL[..index].iter().all(|other| other != fact),
                "{} is declared twice",
                fact.name()
            );
            assert!(!fact.describes().is_empty());
            // **The label's key is not a feature.** No declared fact reads `status`, so the reading cannot
            // restate the answer it predicts — the one property that makes this a classifier at all.
            assert!(
                !fact.describes().contains("status") && !fact.name().contains("status"),
                "{} reads the label's own key",
                fact.name()
            );
            assert_eq!(declared_index(*fact), index, "the columns follow the declaration order");
            assert_eq!(column_name(index), fact.name());
            columns.push(fact.name());
            // Every predicate answers, on an empty head and on a head with nothing declared in it.
            let _ = fact.holds("");
            let _ = fact.holds("title: a record\npatch_id: PATCH-999\n");
        }
        assert!(
            !columns.contains(&"status"),
            "a column named after the label would make this a restatement"
        );
        assert_eq!(
            column_name(HEAD_COLUMNS),
            Feature::ALL[0].name(),
            "the claim's first column follows the head's last"
        );
    }

    #[test]
    fn a_declared_word_holds_only_for_its_own_value() {
        let head = "operation: add\napproval: design-approved\nsource: user-feedback\n\
                    dependencies: [PATCH-001]\nlive_mutation: false\n";
        let facts = declared_facts(head);
        assert!(facts.contains(&Declared::OperationIsAdd));
        assert!(!facts.contains(&Declared::OperationIsStrengthen));
        assert!(facts.contains(&Declared::ApprovalIsADesignApproval));
        assert!(!facts.contains(&Declared::ApprovalNamesAnApplication));
        assert!(facts.contains(&Declared::SourceIsAUserInstruction));
        assert!(!facts.contains(&Declared::SourceIsAFailedEvaluation));
        assert!(!facts.contains(&Declared::LiveMutationDeclared));
        assert!(!facts.contains(&Declared::DeclaresNoDependency));
        // `dependencies: []` is the format's own way of declaring that a record stands on nothing.
        assert!(declared_facts("dependencies: []\n").contains(&Declared::DeclaresNoDependency));
        assert!(declared_facts("live_mutation: true\n").contains(&Declared::LiveMutationDeclared));
        assert!(
            declared_facts("source: failed-eval-and-user-governance\n")
                .contains(&Declared::SourceIsAFailedEvaluation)
        );
    }

    #[test]
    fn a_body_is_what_follows_the_fence_and_a_bom_is_stripped_once() {
        let plain = "---\nstatus: proposed\n---\n\n# Change\n\nsomething\n";
        assert_eq!(body_of(plain).map(str::trim), Some("# Change\n\nsomething"));
        let bom = "\u{feff}---\nstatus: proposed\n---\n\n# Change\n";
        assert_eq!(body_of(bom).map(str::trim), Some("# Change"));
        assert_eq!(body_of("no fence here\n# Change\n"), None);
        assert_eq!(body_of("---\nstatus: proposed\n"), None, "no closing fence, no body");
        assert_eq!(declared_status(plain), Some("proposed"));
        assert_eq!(declared_status(bom), Some("proposed"));
        // **A document with no fence has no head**, so it declares no stage either: the head is the region
        // between the fences, and a `status:` line outside a record is a line of prose.
        assert_eq!(declared_status("no fence\nstatus: proposed\n"), None);
        assert_eq!(head_of(plain).map(str::trim), Some("status: proposed"));
        assert_eq!(head_of("no fence\nstatus: proposed\n"), None);
        assert_eq!(head_field("status: proposed\nstatus: designed\n", "status"), Some("proposed"));
        assert_eq!(head_field("title: a record\n", "status"), None);
    }

    #[test]
    fn two_of_the_sixteen_records_open_with_a_byte_order_mark() {
        let marked: Vec<&str> = KNOWN_ANSWERS
            .iter()
            .filter(|(path, _)| text_of(path).starts_with('\u{feff}'))
            .map(|(path, _)| *path)
            .collect();
        assert_eq!(
            marked,
            vec![
                "patches/PATCH-007-retirement-before-completion.md",
                "patches/PATCH-008-concise-first-response.md"
            ],
            "the measured fact body_of's doc comment states"
        );
        let mut bodied = 0;
        for (path, _) in KNOWN_ANSWERS {
            let text = text_of(path);
            let body = body_of(&text).expect("every declared reading has a body");
            assert!(!body.trim().is_empty(), "{path} has an empty body");
            bodied += 1;
        }
        assert_eq!(bodied, 16, "a calibration over fewer than the declared sixteen is a loss");
    }

    #[test]
    fn the_label_is_never_a_feature_so_the_reading_cannot_see_it() {
        let path = KNOWN_ANSWERS[0].0;
        let text = text_of(path);
        let body = body_of(&text).expect("a body");
        assert!(!body.contains("status:"), "the label's own field is in the body");
        assert!(!body.contains("patch_id:"), "the head leaked into the body");
        let head = head_of(&text).expect("a head");
        // **The claim's features cannot see the head at all**, and the head's facts cannot see the label:
        // perturbing `status:` moves neither list, which is the whole of what makes the reading a reading.
        let moved = text.replace("status: proposed", "status: approved");
        let moved_body = body_of(&moved).expect("the fence is where it was");
        assert_eq!(
            features_of(body),
            features_of(moved_body),
            "a moved label must not move the claim's features"
        );
        assert_eq!(
            declared_facts(head),
            declared_facts(head_of(&moved).expect("the fence is where it was")),
            "a moved label must not move the head's declared facts"
        );
        assert_eq!(declared_status(&moved), Some("approved"), "the harness does read the label");
    }

    #[test]
    fn the_stage_is_in_the_head_and_not_in_the_claim() {
        // **The measurement that moved the stage question to the head**, pinned so a later edition cannot
        // quietly forget it: the record's claim alone does not beat the majority class on this corpus, and
        // the head does. Both figures are printed by the gate; the bar is read against the head's.
        let classifier = Classifier::classified().expect("the sixteen readings are all there");
        let figures = classifier.figures();
        assert!(
            figures.leave_one_out_from_the_claim_only < figures.majority,
            "the claim-only reading is the finding and it does not beat the majority class: {}",
            figures.render()
        );
        assert_eq!(figures.leave_one_out_from_the_claim_only, 0.0, "{}", figures.render());
        assert!(
            figures.leave_one_out > figures.majority,
            "the head's reading is the one the bar is read against: {}",
            figures.render()
        );
    }

    #[test]
    fn the_declared_vocabulary_is_read_over_every_record_in_the_tree_not_only_the_sixteen() {
        // The features are the format's own declared words. The claim each word makes about a stage is
        // measured over **all** the records in `patches/`, so a word whose meaning drifts — or a record
        // whose head contradicts its own vocabulary — is refused by name rather than counted as agreement.
        let corpus = claimed_vocabulary().expect("the tree's records are readable");
        assert!(corpus.records >= 39, "the corpus is smaller than the tree: {}", corpus.records);
        assert!(corpus.refusals.is_empty(), "{}", corpus.refusals);
        for family in &corpus.families {
            assert!(family.seen > 0, "{} was never read", family.word);
            assert!(
                family.seen >= family.outside,
                "{}: more readings outside the calibration set than in the corpus",
                family.word
            );
        }
        let outside: usize = corpus.families.iter().map(|family| family.outside).sum();
        assert!(
            outside > 0,
            "every claim held only over the calibration set, which would make them fitted claims"
        );
        // The one rule that makes `applied-to-internal-base` mean what the head's word says: no record whose
        // approval names an application reads as `proposed` or `designed`.
        let applications = corpus
            .families
            .iter()
            .find(|family| family.word == "applied-to-internal-base")
            .expect("the word is in the corpus");
        assert_eq!(
            applications.claim,
            "a record whose approval names an application is applied-internal or verified-internal"
        );
    }

    #[test]
    fn every_stage_is_named_by_at_least_one_of_the_sixteen_readings() {
        for stage in STAGES {
            let count = KNOWN_ANSWERS.iter().filter(|(_, label)| *label == stage).count();
            assert!(count > 0, "{stage} has no reading, so it cannot be calibrated for");
        }
        let total: usize = KNOWN_ANSWERS
            .iter()
            .filter(|(_, label)| STAGES.contains(label))
            .count();
        assert_eq!(total, KNOWN_ANSWERS.len(), "every reading's label is a declared stage");
    }

    #[test]
    fn the_declared_labels_are_the_records_own_stages() {
        assert!(
            Classifier::labels_describe_the_records().is_empty(),
            "{}",
            Classifier::labels_describe_the_records()
        );
    }

    #[test]
    fn the_calibration_beats_its_own_majority_class() {
        let classifier = Classifier::classified().expect("the sixteen readings are all there");
        let figures = classifier.figures();
        assert_eq!(figures.readings, 16);
        assert_eq!(figures.per_option.len(), STAGES.len());
        assert!(
            (figures.majority - 5.0 / 16.0).abs() < 1e-12,
            "the commonest stage is `proposed` at 5 of 16: {}",
            figures.majority
        );
        assert!(
            figures.in_sample >= 0.5,
            "a calibration that cannot recall its own readings has learned nothing: {}",
            figures.render()
        );
        assert!(
            figures.beats_majority(),
            "the bar is the majority class and this calibration does not beat it: {}",
            figures.render()
        );
        assert!(figures.quotable(), "{}", figures.render());
    }

    #[test]
    fn a_constant_judge_scores_exactly_the_majority_class_and_never_beats_it() {
        // The bar is read strictly on purpose: the commonest answer, answered always, scores the majority
        // class itself — so a judge with no signal ties the bar and does not clear it.
        let majority = 5.0 / 16.0;
        let constant = Measurement {
            readings: 16,
            per_option: vec![("proposed", 5)],
            majority,
            in_sample: majority,
            leave_one_out: majority,
            leave_one_out_with_absence: 0.0,
            leave_one_out_from_the_claim_only: 0.0,
            known_answers: Digest::from_bytes([1u8; 32]),
        };
        assert!(!constant.beats_majority(), "a tie is not a beat");
        assert!(!constant.quotable());
    }

    #[test]
    fn a_reading_whose_label_is_not_an_option_is_refused_rather_than_rounded() {
        let mut readings = KNOWN_ANSWERS.to_vec();
        readings[0] = (readings[0].0, "shipped");
        let refused = Classifier::calibrate(&root(), &readings).expect_err("that is not a stage");
        assert!(refused.contains(RefusalKind::OptionUncalibrated), "{refused}");
        assert!(
            refused.all().iter().any(|refusal| refusal.detail().contains("shipped")),
            "{refused}"
        );
    }

    #[test]
    fn an_option_no_reading_calibrates_is_refused() {
        let readings: Vec<(&str, &str)> = KNOWN_ANSWERS
            .iter()
            .copied()
            .filter(|(_, label)| *label != "approved")
            .collect();
        let refused = Classifier::calibrate(&root(), &readings).expect_err("five options, four readings");
        assert!(refused.contains(RefusalKind::OptionUncalibrated), "{refused}");
        assert!(
            refused.all().iter().any(|refusal| refusal.detail().contains("approved")),
            "{refused}"
        );
    }

    #[test]
    fn an_absent_record_is_refused_and_named() {
        let refused = Classifier::calibrate(&root(), &[("patches/gone.md", "proposed")])
            .expect_err("nothing to read in place");
        assert!(refused.contains(RefusalKind::SourceAbsent), "{refused}");
        assert!(refused.all()[0].detail().contains("gone.md"), "{refused}");
    }

    #[test]
    fn a_record_with_no_body_is_refused_as_a_reading_it_cannot_make() {
        let root = scratch("bodiless", "bodiless.md", "---\nstatus: proposed\n");
        let refused = Classifier::calibrate(&root, &[("bodiless.md", "proposed")])
            .expect_err("a head with no body is nothing to read");
        assert!(refused.contains(RefusalKind::NoReading), "{refused}");
    }

    #[test]
    fn a_moved_stage_makes_the_calibration_stale_rather_than_wrong() {
        let root = scratch(
            "moved",
            "moved.md",
            "---\nstatus: approved\n---\n\n# Change\n\nsomething\n",
        );
        let refusals = Classifier::declared_labels(&root, &[("moved.md", "proposed")]);
        assert!(refusals.contains(RefusalKind::ClaimUnverified), "{refusals}");
        assert!(
            refusals.all()[0].detail().contains("stale rather than wrong"),
            "{refusals}"
        );
    }

    #[test]
    fn the_classifier_answers_the_stage_of_a_real_record_with_the_oracle_origin() {
        // The record is `PATCH-015`, whose head declares `approval: user-confirmed` — a word this corpus
        // carries only for the two `approved` records, so the reading is `approved` and the ballot passes.
        // What the instrument gets *wrong* is the measurement's business and is printed by the gate; what
        // this test holds is that a real record is answered, by the oracle, with what it read.
        let classifier = Classifier::classified().expect("calibrated");
        let question = Question::choice(&STAGES, "approved").expect("five options");
        let ballot = Ballot::declare(
            "u7-life-of-PATCH-015",
            "the plan's §5 row 6",
            "patches/PATCH-015-hypergrilling.md",
            "read the record's own head and say which stage of its life it is at",
            question,
            "the clean room's build",
            24,
        )
        .expect("a complete ballot");
        let answered = answer(&ballot, &classifier, &root(), None).expect("a calibrated answer");
        assert_eq!(answered.origin(), Origin::Oracle);
        assert_eq!(answered.origin().token(), "A");
        assert_eq!(answered.verdict(), Verdict::Belongs, "{}", classifier.figures().render());
        assert!(
            answered
                .evidence()
                .features
                .contains(&"approval_is_user_confirmed"),
            "the answer carries what it read: {:?}",
            answered.evidence().features
        );
        assert_eq!(answered.evidence().readings, 16);
        assert_eq!(answered.evidence().leave_one_out, classifier.figures().leave_one_out);
    }

    #[test]
    fn a_document_that_is_not_a_record_is_a_non_reading() {
        let classifier = Classifier::classified().expect("calibrated");
        let question = Question::choice(&STAGES, "designed").expect("five options");
        let ballot = Ballot::declare(
            "u7-life-of-a-readme",
            "the plan's §5 row 6",
            "readme-probe.md",
            "read the record's own body",
            question,
            "the clean-room's build",
            24,
        )
        .expect("a complete ballot");
        let root = scratch(
            "readme",
            "readme-probe.md",
            "# a description, not a record\n\nprose about a thing.\n",
        );
        let answered = answer(&ballot, &classifier, &root, None).expect("a non-reading is an answer");
        assert_eq!(answered.verdict(), Verdict::NoReading);
    }

    #[test]
    fn a_noul_question_is_refused_by_name_because_no_set_calibrates_it() {
        // Lane C's three-part reading is computable, but this build ships **no known-answer set** for a
        // `noul` question — so there is nothing for an answer to rest on, and the question is refused by
        // name rather than answered with the *stage* question's figures (`docs/LEDGER.md` R-59.5: an
        // uncalibrated instrument writes nothing). The instrument's own reading is exercised where it is a
        // reading — `declared_facts`/`features_of` below — and the quotable path is the one closed here.
        let classifier = Classifier::classified().expect("calibrated");
        let question = Question::noul(0.5).expect("the natural midpoint");
        let record = text_of("patches/PATCH-023-the-ballot-and-the-von-classifier.md");
        let refusal = classifier
            .judge(&question, record.as_bytes())
            .expect_err("no `noul` set exists, so the question is refused");
        assert_eq!(refusal.kind(), RefusalKind::NotEstablished);
        assert!(refusal.detail().contains("noul"), "{refusal}");
    }
}
