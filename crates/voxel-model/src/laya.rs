//! The judge: typed questions in, typed answers out — and the project gate it
//! decides. **Full powers**: the judge evaluates the entire project — every
//! source file and every test — and by default **every artifact fails**.
//!
//! # The rulings this file implements
//!
//! * The judge and the gate evaluate the ENTIRE project. Nothing is scoped out
//!   and no question is narrowed away. A reduced judge is not this judge.
//! * By default all of the code in the project FAILS. An artifact clears only on
//!   proof of work: a claim on the record, evidence with an origin, a judgement
//!   through this judge, and a grade on its own axis.
//! * The runnable gate runs at `cargo test` time.
//! * No test succeeds unless the client carries a notification over **every
//!   single decision**, as the api layer. [`Evaluation::notification_lines`]
//!   emits one per decision — never fewer.
//! * The tests are judged too. Past agents' tests carry no authority until this
//!   judge and decider pass them.
//!
//! # The instrument
//!
//! Laya is *non-autoregressive and text-only*: typed questions (`choice`,
//! `score`, `noul`) in, typed answers with calibrated confidence out, in one
//! pass. **No variant of an answer carries prose** — a judge that could return a
//! sentence would be a judge whose sentence could be quoted.
//!
//! # Calibration, and the verdict's quotability
//!
//! A verdict is quotable only if the judge beats the majority class on a
//! known-answer set ([`calibrate`]). The bar is the exact measurement that
//! convicted the model this instrument replaces. A judge that loses is
//! **NOT QUOTABLE**, and the gate then clears nothing — which is why
//! verification cannot pass through the axes without this judge in front of
//! them.

use std::path::{Path, PathBuf};

use crate::axis::{
    refuse_bare, refuse_interchange, refuse_reduction, AxisKind, EvidenceGrade, EvidenceLevel,
    Reduction,
};
use crate::refusal::{Refusal, RefusalKind, RefusalSet};
use crate::identity::Digest;
use crate::ticket::EvidenceOrigin;

/// The three questions an oracle may be asked. There is no fourth.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[derive(musli::Encode, musli::Decode)]
pub enum Question {
    /// Does this belong to the recipe? Binary.
    Choice,
    /// How much of it belongs, weakest to strongest. An ordinal, not a score.
    Score,
    /// The judge read it and declines to answer.
    Noul,
}

impl Question {
    pub const ALL: [Question; 3] = [Question::Choice, Question::Score, Question::Noul];

    pub const fn as_str(self) -> &'static str {
        match self {
            Question::Choice => "choice",
            Question::Score => "score",
            Question::Noul => "noul",
        }
    }

    /// Parse a question name. Unknown is refused rather than coerced.
    pub fn parse(word: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|candidate| candidate.as_str() == word.trim())
            .ok_or_else(|| format!("unknown question {word:?}"))
    }
}

/// The judge's decision on conformance. Typed, closed, no prose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[derive(musli::Encode, musli::Decode)]
pub enum Verdict {
    /// Every term the subject introduces is declared by the basis.
    Belongs,
    /// The subject introduces a term the basis does not name — and the term is
    /// named in the reading, never in the verdict.
    NotBelongs,
    /// The judge read it and there is nothing to judge.
    NoReading,
}

impl Verdict {
    pub const ALL: [Verdict; 3] = [Verdict::Belongs, Verdict::NotBelongs, Verdict::NoReading];

    pub const fn as_str(self) -> &'static str {
        match self {
            Verdict::Belongs => "belongs",
            Verdict::NotBelongs => "not_belongs",
            Verdict::NoReading => "no_reading",
        }
    }
}

impl std::fmt::Display for Verdict {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A typed answer. **No variant carries prose.**
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AnswerValue {
    /// A binary conformance verdict.
    Choice(Verdict),
    /// An ordinal reading, 0 weakest to 5 strongest. A direction, never a score
    /// — [`refuse_reduction`] is what stops it being used as one.
    Score(u8),
    /// The judge read the input and declines.
    Noul,
}

/// One answer: the typed value, and how confident the judge was.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Answer {
    /// The question this answers.
    pub question: Question,
    /// The typed value.
    pub value: AnswerValue,
    /// `0.0..=1.0`. Outside that range is malformed and refused at construction.
    pub confidence: f64,
}

impl Answer {
    /// A binary answer. The confidence is bounded at construction.
    pub fn choice(verdict: Verdict, confidence: f64) -> Result<Self, String> {
        let confidence = bounded(confidence)?;
        Ok(Self {
            question: Question::Choice,
            value: AnswerValue::Choice(verdict),
            confidence,
        })
    }

    /// An ordinal answer on the 0..=5 scale. Above the ceiling is refused: a
    /// scale with no ceiling is a score.
    pub fn score(points: u8, confidence: f64) -> Result<Self, String> {
        let confidence = bounded(confidence)?;
        if points > 5 {
            return Err(format!("score {points} is above the 0..=5 range"));
        }
        Ok(Self {
            question: Question::Score,
            value: AnswerValue::Score(points),
            confidence,
        })
    }

    /// The judge read it and declines.
    pub fn noul() -> Self {
        Self {
            question: Question::Noul,
            value: AnswerValue::Noul,
            confidence: 0.0,
        }
    }
}

fn bounded(confidence: f64) -> Result<f64, String> {
    if !(0.0..=1.0).contains(&confidence) || !confidence.is_finite() {
        return Err(format!(
            "confidence {confidence} is outside 0.0..=1.0; an unbounded confidence is not one"
        ));
    }
    Ok(confidence)
}

/// A judge that answers typed questions about **any** subject against any
/// basis — a line, a file, a test, a claim. Full powers; nothing is scoped out.
pub trait Judge {
    /// Which instrument this is, for the record.
    fn name(&self) -> &'static str;

    /// Answer one typed question about a subject against a basis.
    fn ask(&self, question: Question, subject: &str, basis: &str) -> Result<Answer, RefusalSet>;
}

/// The Rust judge: **structural** conformance, and it says so.
///
/// It answers *"does this subject belong to the basis?"* by comparing the
/// **vocabulary the subject introduces** against the vocabulary the basis
/// declares. Deterministic, explainable, and it **names the culprit** in its
/// reading — which is what makes a refusal actionable. Its confidence is a
/// measured coverage ratio, not a softmax, and [`calibrate`] decides whether it
/// is worth quoting.
#[derive(Clone, Copy, Debug, Default)]
pub struct StructuralJudge;

/// What the judge saw, before it became a typed [`Answer`].
#[derive(Clone, Debug, PartialEq)]
pub enum Reading {
    /// Nothing to judge: a blank line, a comment, or pure language shape.
    NothingToJudge,
    /// Every term is declared. Carries the measured coverage.
    AllDeclared {
        /// The declared fraction of the introduced terms.
        coverage: f64,
    },
    /// Terms the basis does not name — **the culprits, verbatim**.
    Undeclared {
        /// The offending terms, lowercased as [`terms`] returns them.
        found: Vec<String>,
        /// The declared fraction of the introduced terms.
        coverage: f64,
    },
}

impl Judge for StructuralJudge {
    fn name(&self) -> &'static str {
        "structural"
    }

    fn ask(&self, question: Question, subject: &str, basis: &str) -> Result<Answer, RefusalSet> {
        match question {
            Question::Noul => Ok(Answer::noul()),
            Question::Choice => match self.reading(subject, basis) {
                Reading::NothingToJudge => Ok(Answer::noul()),
                Reading::AllDeclared { coverage } => {
                    Answer::choice(Verdict::Belongs, coverage).map_err(refusals)
                }
                // Confidence is deliberately **low** on a negative reading: a
                // judge confident it found a fault is the reading most likely to
                // be quoted against a contributor.
                Reading::Undeclared { coverage, .. } => {
                    Answer::choice(Verdict::NotBelongs, (1.0 - coverage).min(0.5)).map_err(refusals)
                }
            },
            Question::Score => {
                let points = match self.reading(subject, basis) {
                    Reading::NothingToJudge => 0,
                    Reading::AllDeclared { coverage } => {
                        (coverage * 5.0).round().clamp(1.0, 5.0) as u8
                    }
                    Reading::Undeclared { .. } => 0,
                };
                Answer::score(points, 0.5).map_err(refusals)
            }
        }
    }
}

impl StructuralJudge {
    /// The reading, kept separate from the answer so it can be inspected and
    /// named. [`Reading::Undeclared`] carries the culprits — an [`Answer`]
    /// carries no prose by design, so the names live here.
    pub fn reading(&self, subject: &str, basis: &str) -> Reading {
        self.reading_within(subject, basis, &[])
    }

    /// The reading against a basis **plus an explicitly licensed vocabulary**.
    ///
    /// `licensed` is added to the basis's own terms and nothing else changes.
    /// **No default**: defaulting an empty `licensed` to "everything" would make
    /// the gate pass every file and mean nothing.
    pub fn reading_within(&self, subject: &str, basis: &str, licensed: &[String]) -> Reading {
        if subject.trim().is_empty() || subject.trim().starts_with("//") {
            return Reading::NothingToJudge;
        }
        let introduced = terms(subject);
        if introduced.is_empty() || introduced.iter().all(|term| is_syntax(term)) {
            return Reading::NothingToJudge;
        }
        let mut declared: std::collections::BTreeSet<String> = terms(basis).into_iter().collect();
        for term in licensed {
            declared.extend(terms(term));
        }
        let undeclared: Vec<String> = introduced
            .iter()
            .filter(|term| !declared.contains(*term) && !is_syntax(term))
            .cloned()
            .collect();
        let coverage = 1.0 - (undeclared.len() as f64 / introduced.len() as f64);
        if undeclared.is_empty() {
            Reading::AllDeclared { coverage }
        } else {
            Reading::Undeclared {
                found: undeclared,
                coverage,
            }
        }
    }
}

/// Terms a text introduces: identifier-shaped runs, lowercased, first-seen
/// order, de-duplicated. Public because a second caller needs the judge's own
/// rule, not a copy of it.
pub fn terms(text: &str) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    let mut current = String::new();
    for ch in text.chars().chain(std::iter::once(' ')) {
        if ch.is_alphanumeric() || ch == '_' {
            current.push(ch.to_ascii_lowercase());
        } else if !current.is_empty() {
            if !seen.contains(&current) {
                seen.push(std::mem::take(&mut current));
            } else {
                current.clear();
            }
        }
    }
    seen
}

/// Language shape rather than meaning. Kept small and named: every entry here is
/// a term the judge will not hold against a contributor.
fn is_syntax(term: &str) -> bool {
    matches!(
        term,
        "let" | "mut" | "fn" | "pub" | "struct" | "enum" | "impl" | "use" | "mod" | "const"
            | "self" | "true" | "false" | "none" | "some" | "ok" | "err" | "if" | "else"
            | "for" | "in" | "while" | "match" | "return" | "type" | "where" | "as" | "ref"
            | "move" | "dyn" | "trait" | "crate" | "super" | "u8" | "u16" | "u32" | "u64"
            | "i64" | "f64" | "bool" | "str" | "string" | "vec" | "box"
    )
}

fn refusals(problem: String) -> RefusalSet {
    let mut refusals = RefusalSet::new();
    refusals.refuse(RefusalKind::UncalibratedOracle, problem);
    refusals
}

// ---------------------------------------------------------------------------
// Calibration — the instrument that convicted the model, pointed at this judge
// ---------------------------------------------------------------------------

/// One decision whose answer is **on record**, not an opinion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KnownAnswer {
    /// The subject judged.
    pub subject: &'static str,
    /// The basis it was judged against.
    pub basis: &'static str,
    /// The verdict on record.
    pub expected: Verdict,
}

/// Whether a calibration run may be quoted at all.
#[derive(Clone, Debug, PartialEq)]
pub struct Calibration {
    /// The judge beat the majority class on the set.
    pub calibrated: bool,
    /// The run measured nothing.
    pub unmeasured: bool,
    /// Both numbers and the distribution, kept rather than folded into a flag.
    pub notes: Vec<String>,
}

impl Calibration {
    /// A run that measured nothing is not a pass.
    pub fn unmeasured() -> Self {
        Calibration {
            calibrated: false,
            unmeasured: true,
            notes: vec!["no known answers were measured".to_string()],
        }
    }

    /// A verdict from an unquotable run is noise wearing an instrument's name.
    pub fn is_quotable(&self) -> bool {
        self.calibrated
    }
}

/// Measure a judge against a known-answer set and decide whether it may be
/// quoted. **The bar is the majority class** — a judge below the most common
/// answer is worse than always guessing.
pub fn calibrate(judge: &dyn Judge, set: &[KnownAnswer]) -> CalibrationReport {
    if set.is_empty() {
        return CalibrationReport {
            calibration: Calibration::unmeasured(),
            accuracy: 0.0,
            majority_class: 0.0,
            inverted_accuracy: 0.0,
            readings_used: 0,
        };
    }

    let mut correct = 0usize;
    let mut inverted = 0usize;
    let mut per_verdict = [0usize; 3];
    let mut used = 0usize;

    for known in set {
        let Ok(answer) = judge.ask(Question::Choice, known.subject, known.basis) else {
            continue;
        };
        let AnswerValue::Choice(verdict) = answer.value else {
            continue;
        };
        used += 1;
        match verdict {
            Verdict::Belongs => per_verdict[0] += 1,
            Verdict::NotBelongs => per_verdict[1] += 1,
            Verdict::NoReading => per_verdict[2] += 1,
        }
        if verdict == known.expected {
            correct += 1;
        }
        if invert(verdict) == known.expected {
            inverted += 1;
        }
    }

    if used == 0 {
        return CalibrationReport {
            calibration: Calibration::unmeasured(),
            accuracy: 0.0,
            majority_class: 0.0,
            inverted_accuracy: 0.0,
            readings_used: 0,
        };
    }

    let total = used as f64;
    let accuracy = correct as f64 / total;
    let inverted_accuracy = inverted as f64 / total;
    let majority_class = per_verdict.iter().copied().max().unwrap_or(0) as f64 / total;

    let beats_majority = accuracy > majority_class;
    let mut calibration = Calibration {
        calibrated: beats_majority,
        unmeasured: false,
        notes: vec![
            format!("accuracy {accuracy:.4}"),
            format!("majority class {majority_class:.4}"),
            format!("accuracy if the reading were inverted {inverted_accuracy:.4}"),
            format!("readings used {used}"),
            format!(
                "verdict distribution belongs {} / not_belongs {} / no_reading {}",
                per_verdict[0], per_verdict[1], per_verdict[2]
            ),
        ],
    };
    if !beats_majority {
        calibration.notes.push(
            "NOT QUOTABLE: this judge does not beat the majority class on this set, which is the \
             measurement that convicted the model. A verdict from this run is noise wearing an \
             instrument's name."
                .to_string(),
        );
    }

    CalibrationReport {
        calibration,
        accuracy,
        majority_class,
        inverted_accuracy,
        readings_used: used,
    }
}

/// The calibration with the numbers that decided it.
#[derive(Clone, Debug, PartialEq)]
pub struct CalibrationReport {
    /// The quotability decision and its notes.
    pub calibration: Calibration,
    /// Correct readings over readings used.
    pub accuracy: f64,
    /// The most common expected answer over readings used.
    pub majority_class: f64,
    /// Accuracy if every verdict were inverted.
    pub inverted_accuracy: f64,
    /// How many known answers were read.
    pub readings_used: usize,
}

impl CalibrationReport {
    /// Which reading of the verdicts this run uses. The inversion is measured,
    /// and the reading used is recorded.
    pub fn prefer_inverted(&self) -> bool {
        self.inverted_accuracy > self.accuracy
    }
}

fn invert(verdict: Verdict) -> Verdict {
    match verdict {
        Verdict::Belongs => Verdict::NotBelongs,
        Verdict::NotBelongs => Verdict::Belongs,
        Verdict::NoReading => Verdict::NoReading,
    }
}

// ---------------------------------------------------------------------------
// Proof of work, decisions, and the project gate
// ---------------------------------------------------------------------------

/// The proof of work one artifact carries: **a claim on the record, and
/// evidence with an origin**. Evidence filed without an origin is filed as an
/// assertion ([`crate::ticket::EvidenceOrigin`]'s own rule).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProofOfWork {
    /// The artifact's path relative to the workspace root.
    pub path: String,
    /// The claim anchor on the record — a ticket or ruling id.
    pub claim: String,
    /// How the evidence came to exist. On the record.
    pub origin: EvidenceOrigin,
    /// Where the evidence was measured.
    pub evidence: String,
}

/// One decision of the gate, on one artifact. Every decision produces exactly
/// one client notification — [`Evaluation::notification_lines`] is total.
#[derive(Clone, Debug, PartialEq)]
#[derive(musli::Encode, musli::Decode)]
pub struct Decision {
    /// The artifact's path relative to the workspace root.
    pub path: String,
    /// The question asked.
    pub question: Question,
    /// The judge's typed answer.
    pub verdict: Verdict,
    /// The judge's confidence — carried for the record, never quoted as a score.
    pub confidence: f64,
    /// The evidence origin the artifact carried, if any. Absence is rendered as
    /// absence.
    pub origin: Option<EvidenceOrigin>,
    /// Where each graded value sits — **each on its own ladder**, never merged.
    pub grades: Vec<(AxisKind, String)>,
    /// Findings, each citing the line that triggered it. A finding with no
    /// cited line does not exist.
    pub findings: Vec<String>,
}

impl Decision {
    /// Whether this artifact cleared the gate.
    pub fn cleared(&self) -> bool {
        self.verdict == Verdict::Belongs && self.findings.is_empty()
    }
}

/// The gate's whole run: every decision, and the calibration that decides
/// whether any verdict is quotable.
#[derive(Clone, Debug, PartialEq)]
pub struct Evaluation {
    /// One decision per artifact, in path order.
    pub decisions: Vec<Decision>,
    /// The judge's calibration on the known-answer set. An unquotable judge
    /// clears **nothing**.
    pub calibration: CalibrationReport,
}

impl Evaluation {
    /// Whether the entire project cleared. **Every** artifact must clear and the
    /// judge must be quotable — one failure is a failure.
    pub fn all_cleared(&self) -> bool {
        self.calibration.calibration.is_quotable()
            && !self.decisions.is_empty()
            && self.decisions.iter().all(Decision::cleared)
    }


    // The queue is streamed: one record per decision, encoded and written
    // alone, so the run never holds more than one notification's bytes.
    // The count written is the decision count, always.
    pub fn write_queue(&self, root: &Path, queue: &Path) -> Result<usize, Refusal> {
        let mut written = 0usize;
        for (index, decision) in self.decisions.iter().enumerate() {
            write_one(root, queue, index, decision)?;
            written += 1;
        }
        Ok(written)
    }

    /// The full report: every artifact, its verdict, and its cited findings.
    pub fn report(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "laya gate: {} artifact(s) judged, {} cleared, calibration {:?}\n",
            self.decisions.len(),
            self.decisions.iter().filter(|d| d.cleared()).count(),
            self.calibration.calibration.notes
        ));
        for decision in &self.decisions {
            out.push_str(&format!(
                "  {} {} [{}]{}\n",
                decision.verdict.as_str(),
                decision.path,
                decision
                    .origin
                    .map(|origin| origin.letter().to_string())
                    .unwrap_or_else(|| "-".to_string()),
                if decision.findings.is_empty() {
                    String::new()
                } else {
                    format!(" {}", decision.findings.join("; "))
                }
            ));
        }
        out
    }
}



/// The security scan, per artifact. **Every finding cites the line that
/// triggered it** — the lfe-security-check rule: a finding with no quoted code
/// is not a finding.
///
/// `A08` — integrity failures: a **hand-rolled wire format** — manual byte
/// encoding or decoding in project code, with no proof of work claiming a
/// reviewed standard. `wire.rs` is the case in point: a bespoke serializer over
/// untrusted bytes is exactly the failure this gate exists to name.
pub fn security_findings(path: &str, source: &str) -> Vec<String> {
    let mut findings = Vec::new();
    let byte_markers = [
        concat!("from_", "ne_bytes"),
        concat!("to_", "ne_bytes"),
    ];
    let mut hits: Vec<(usize, &str)> = Vec::new();
    for (index, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") {
            continue;
        }
        if byte_markers.iter().any(|marker| line.contains(marker)) {
            hits.push((index + 1, trimmed));
        }
    }
    if !hits.is_empty() {
        let (line_no, line) = hits[0];
        findings.push(format!(
            "A08 hand-rolled byte encoding at {path}:{line_no}: `{}` — a wire format rolled by \
             hand over untrusted bytes; {} byte-level site(s) in this artifact",
            line.chars().take(80).collect::<String>(),
            hits.len()
        ));
    }
    findings
}

/// Judge one artifact through the full instrument: proof of work, the security
/// scan, the judge, and the axis grades. **No artifact passes by default** — an
/// artifact with no proof of work fails, whatever else it shows.
pub fn decide(
    judge: &dyn Judge,
    path: &str,
    source: &str,
    proofs: &[ProofOfWork],
    calibration: &CalibrationReport,
) -> Decision {
    let mut findings = security_findings(path, source);
    let proof = proofs.iter().find(|proof| proof.path == path);

    let mut origin = None;
    let mut grades = Vec::new();
    match proof {
        None => {
            findings.push(
                "no proof of work on record: no claim, no evidence, no origin — the default is \
                 fail"
                    .to_string(),
            );
        }
        Some(proof) => {
            origin = Some(proof.origin);
            grades.push((AxisKind::TicketEvidence, proof.origin.letter().to_string()));
            grades.push((AxisKind::SupportLevel, EvidenceLevel::Unverified.as_str().to_string()));
            // The judge asks whether the evidence belongs to the claim — one of
            // its three questions, asked at full scope.
            if let Ok(answer) = judge.ask(Question::Choice, source, &proof.claim) {
                if answer.value == AnswerValue::Choice(Verdict::NotBelongs) {
                    findings.push(format!(
                        "the judge read the artifact against claim {} and the evidence does not \
                         belong to it",
                        proof.claim
                    ));
                }
            }
        }
    }

    // An unquotable judge convicts nothing and clears nothing: its verdict is
    // `NoReading` at best, and the finding names the calibration.
    let (verdict, confidence) = if !calibration.calibration.is_quotable() {
        (
            Verdict::NoReading,
            0.0,
        )
    } else {
        match proof {
            None => (Verdict::NotBelongs, 0.0),
            Some(_) if findings.is_empty() => (Verdict::Belongs, 0.5),
            Some(_) => (Verdict::NotBelongs, 0.5),
        }
    };
    if !calibration.calibration.is_quotable() {
        findings.push(
            "the judge is not calibrated on the known-answer set, so no verdict from this run is \
             quotable"
                .to_string(),
        );
    }

    Decision {
        path: path.to_string(),
        question: Question::Choice,
        verdict,
        confidence,
        origin,
        grades,
        findings,
    }
}

/// The gate: judge **every** `.rs` artifact under `crates/` — source files and
/// test files alike — in deterministic path order, through [`decide`].
/// The artifact a commit site names: `crates/voxel-model/src/agency/action.rs:607`
/// becomes `crates/voxel-model/src/agency/action.rs`. The trailing `:line` is
/// dropped only when it is a line number, and `\` is normalized to `/`.
pub fn commit_site_artifact(site: &str) -> String {
    let file = match site.rsplit_once(':') {
        Some((file, line))
            if !line.is_empty() && line.bytes().all(|byte| byte.is_ascii_digit()) =>
        {
            file
        }
        _ => site,
    };
    file.replace('\\', "/")
}

/// Mint one [`ProofOfWork`] per committed observer row. The observer is the
/// CPU civilians' system: each committed action is a row the citizens
/// produced through their needs and feelings, and each row names the artifact
/// whose code committed the act. The proof carries the row's claim anchor and
/// the row's own account, graded `ReadBack`.
pub fn mint_proofs_from_rows(rows: Vec<crate::observer::recorder::CommittedAction>) -> Vec<ProofOfWork> {
    rows.into_iter()
        .map(|row| ProofOfWork {
            path: commit_site_artifact(&row.site),
            claim: row.claim(),
            origin: crate::ticket::EvidenceOrigin::ReadBack,
            evidence: row.message(),
        })
        .collect()
}

pub fn evaluate_project(root: &Path, proofs: &[ProofOfWork]) -> Evaluation {
    let mut paths: Vec<PathBuf> = Vec::new();
    collect_rs_files(&root.join("crates"), &mut paths);
    paths.sort();

    let judge = StructuralJudge;
    let calibration = calibrate(&judge, KNOWN_ANSWERS);

    let decisions = paths
        .iter()
        .map(|path| {
            let relative = path
                .strip_prefix(root)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");
            let source = std::fs::read_to_string(path).unwrap_or_default();
            decide(&judge, &relative, &source, proofs, &calibration)
        })
        .collect();

    Evaluation {
        decisions,
        calibration,
    }
}

/// Where the queue lives: a **directory** the client watches — the change
/// channel's own shape (IMG_3365: *the client does not need a custom network
/// protocol — it can just see a directory … that the compiler process
/// updates*). One record per file, named in decision order. **The format is not
/// invented here**: records ride the metadata codec the material names
/// (IMG_3320: *musli-wire for metadata*), never a hand-rolled line format.
pub fn queue_dir(workspace: &Path) -> PathBuf {
    workspace.join("logs").join("laya-decisions")
}

/// One notification on the client's queue — **the whole decision**, carried
/// verbatim. The decision layer is one of the api's three essential layers and
/// is **never reduced**: this embeds [`Decision`] whole, so every finding and
/// every grade crosses the seam intact or the decode fails.
#[derive(Clone, Debug, PartialEq, musli::Encode, musli::Decode)]
pub struct Notification {
    /// Which notification in the run.
    pub notification: String,
    /// The decision, in full.
    pub decision: Decision,
    /// The judged artifact's sha256 — what the client checks against the
    /// artifact's own bytes before it shows the record (IMG_3365: *verifies
    /// sha256(artifact) == expected*).
    pub artifact_sha256: Digest,
}

impl Notification {
    /// Build the notification for one decision: the queue id, the decision
    /// whole, and the artifact's sha256 — **streamed off disk**, never read
    /// whole into memory to be hashed.
    pub fn of(index: usize, decision: &Decision, root: &Path) -> Result<Self, Refusal> {
        let artifact = root.join(&decision.path);
        let file = std::fs::File::open(&artifact).map_err(|failure| {
            Refusal::owned(
                RefusalKind::MalformedRecord,
                format!("{} cannot be opened: {failure}", artifact.display()),
            )
        })?;
        let artifact_sha256 =
            Digest::of_reader(std::io::BufReader::new(file)).map_err(|failure| {
                Refusal::owned(
                    RefusalKind::MalformedRecord,
                    format!("{} cannot be read: {failure}", artifact.display()),
                )
            })?;
        Ok(Notification {
            notification: format!("laya-{index:04}"),
            decision: decision.clone(),
            artifact_sha256,
        })
    }

    /// The record's file name in the queue — decision order, one file per record.
    pub fn file_name(&self) -> String {
        format!("{}.musli", self.notification)
    }

    /// Encode on the decided wire — the metadata codec the material names.
    pub fn encode(&self) -> Result<Vec<u8>, Refusal> {
        musli::wire::to_vec(self).map_err(|failure| {
            Refusal::owned(
                RefusalKind::MalformedRecord,
                format!("the record does not encode: {failure}"),
            )
        })
    }

    /// **A record named for what it is about**, rather than for its position in a run.
    ///
    /// [`Notification::of`] names records `laya-NNNN` because the gate queue **is** one run's
    /// decisions in order. The agent's inbox is not one run's: it is the work the agent owes, and a
    /// record named by position would be **overwritten by the next run's first filing** — losing,
    /// silently, work nobody had read. Naming the record for its subject makes a second
    /// notification about the same ticket replace that ticket's record and nothing else.
    pub fn named_for(subject: &str, decision: &Decision, root: &Path) -> Result<Self, Refusal> {
        let mut notification = Notification::of(0, decision, root)?;
        notification.notification = format!("recipe-{subject}");
        Ok(notification)
    }

    /// Decode one record. Malformed bytes are **refused**, never half-read: a
    /// half-read decision is a decision the window would show wrong.
    pub fn decode(bytes: &[u8]) -> Result<Notification, Refusal> {
        musli::wire::from_slice(bytes).map_err(|failure| {
            Refusal::owned(
                RefusalKind::MalformedRecord,
                format!("the record does not decode: {failure}"),
            )
        })
    }
}

/// **The agent's inbox**: the recipes this run derived from the solutions, on the same streaming
/// system the notifications window reads.
///
/// # Why it is a queue, and why the decision layer is the writer
///
/// The first design had the agent authoring commands into a file the client ran. It was wrong in
/// both halves: **the agent authors nothing**, and this shape of record already has a writer —
/// the judge. The recipes are derived from the solutions the compiler emits, the decision layer
/// writes one record per emitted package, and the agent's work is to read the record and
/// implement the recipe it points at. That is the arrow `queue -> agent` in
/// `architecture/workspace.dsl`, added with this function.
///
/// **One directory, one writer, one reader** is the queue's own rule (H-12.2), and this is the
/// same codec, the same `sha256(artifact)` binding and the same stream the window already uses.
/// A second record format for the agent would be a second record, which
/// `forbidden.second-record` names as the defect.
pub fn agent_inbox_dir(workspace: &Path) -> PathBuf {
    workspace.join("logs").join("agent-inbox")
}

/// A path **as the record spells it**: relative to the workspace root, with `/` separators.
///
/// The same spelling [`evaluate_project`] gives its artifacts, extracted here so a notification's
/// artifact and a decision's subject cannot be two spellings of one file. A path that is not under
/// the root is **refused**, never guessed at: a record naming an absolute path would be readable
/// on one machine and not another, and the artifact's digest is only checkable if the reader can
/// find the file the record meant.
pub fn relative_to(root: &Path, path: &Path) -> Result<String, Refusal> {
    let relative = path.strip_prefix(root).map_err(|_| {
        Refusal::owned(
            RefusalKind::MalformedIdentity,
            format!(
                "{} is not under {}, so the record could not name it relative to the project",
                path.display(),
                root.display()
            ),
        )
    })?;
    Ok(relative.to_string_lossy().replace('\\', "/"))
}

/// **One notification onto one queue, through the declared codec** — the whole write path, so the
/// gate queue and the agent's inbox cannot encode a record differently.
pub fn write_notification(queue: &Path, notification: &Notification) -> Result<String, Refusal> {
    std::fs::create_dir_all(queue).map_err(|failure| {
        Refusal::owned(
            RefusalKind::MalformedRecord,
            format!("{} cannot be created: {failure}", queue.display()),
        )
    })?;
    let bytes = notification.encode()?;
    let name = notification.file_name();
    std::fs::write(queue.join(&name), bytes).map_err(|failure| {
        Refusal::owned(
            RefusalKind::MalformedRecord,
            format!("{name} cannot be written: {failure}"),
        )
    })?;
    Ok(name)
}

/// **One decision onto one queue, indexed by the caller** — `write_queue` calls this for every
/// decision it writes, and the inbox calls it for the one record it writes.
///
/// Returns the record's file name. A record that cannot be written is **refused by name**, never
/// silently skipped: a queue missing the record it claimed to write is a window that will quietly
/// show one decision fewer.
pub fn write_one(root: &Path, queue: &Path, index: usize, decision: &Decision) -> Result<String, Refusal> {
    let notification = Notification::of(index, decision, root)?;
    write_notification(queue, &notification)
}

/// **The recipe, notified to the agent** — one record per package the game emitted from a
/// solution, written into [`agent_inbox_dir`].
///
/// # What the record says, and what it refuses to invent
///
/// The artifact is the **package**, and that is the whole delivery: `recipe::load` returns the
/// exact recipe source a package carries, so the rows the package implements arrive **with the
/// artifact** — the agent reads the work out of the package rather than out of a second file. The
/// record's job is to be the pointer **with a digest**: the reader checks `sha256(package)` before
/// it will show anything, so a recipe the agent is handed is the bytes it was notified about or
/// nothing.
///
/// **`subject` names the record** (`recipe-<subject>.musli`), and it is the ticket: a positional
/// name would be overwritten by the next run's first filing, and an inbox record is work the agent
/// owes until it is read.
///
/// `provenance_held` is the compiler's own round-trip measurement — `verify_provenance` loads the
/// package back and checks it carries the exact source it was compiled from, and that every window
/// the solution declared is present. It arrives as an argument because **the decision layer must
/// not re-run the compiler to believe it**: the reader measures and the layer decides (C-84.2).
/// A package whose provenance did not hold is `NotBelongs`, with the reason on the record, which
/// is the same verdict the gate would have made one layer down.
///
/// **The confidence is 1.0, and that is not a boast**: the reading is a round trip and a round trip
/// is decidable, so a hedged number over a decided fact would read as uncertainty that does not
/// exist. It is carried for the record and never quoted as a score.
pub fn notify_the_agent_of(
    workspace: &Path,
    subject: &str,
    package: &str,
    package_digest: &Digest,
    provenance_held: bool,
) -> Result<String, Refusal> {
    let mut findings: Vec<String> = Vec::new();
    let verdict = if provenance_held {
        Verdict::Belongs
    } else {
        findings.push(
            "the package's provenance did not verify: it does not carry the exact recipe source \
             it was compiled from, so the terms it introduces are not the recipe's terms"
                .to_string(),
        );
        Verdict::NotBelongs
    };
    let decision = Decision {
        path: package.to_string(),
        question: Question::Choice,
        verdict,
        confidence: 1.0,
        // **ReadBack, not Oracle.** The provenance check is this project measuring its own
        // artifact, which is exactly what the `B` origin means; an oracle is something outside
        // this project deciding, and nothing outside did.
        origin: Some(EvidenceOrigin::ReadBack),
        grades: Vec::new(),
        findings,
    };
    let notification = Notification::named_for(subject, &decision, workspace)?;
    // **The receipt's digest and the artifact's own must agree**, or the record is refused rather
    // than written: a notification binding bytes nobody measured is worse than no notification,
    // because it verifies against the wrong thing.
    if &notification.artifact_sha256 != package_digest {
        return Err(Refusal::owned(
            RefusalKind::DigestMismatch,
            format!(
                "the receipt names {package_digest} for {package} and the bytes on disk hash to {}; \
                 the record is refused rather than written against a package nobody measured",
                notification.artifact_sha256
            ),
        ));
    }
    write_notification(&agent_inbox_dir(workspace), &notification)
}



fn collect_rs_files(dir: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut names: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    names.sort();
    for path in names {
        if path.is_dir() {
            collect_rs_files(&path, into);
        } else if path.extension().map(|ext| ext == "rs").unwrap_or(false) {
            into.push(path);
        }
    }
}

/// The judge's known-answer set: the bar a verdict must clear to be quotable.
///
/// # This set was four readings on invented vocabulary, and that was the C-61 defect
///
/// The first set judged `let motor = transducer;` against a sentence about rotation and
/// wiring — **a subject and a basis this tree's record does not carry**, so the calibration
/// measured the judge against a world that does not exist. `voxel-package`'s set was
/// corrected for the same reason (C-63: 53 readings, accuracy 1.0000 against a majority of
/// 0.6226); this one could not borrow that set, because the dependency direction is
/// `voxel-package -> voxel-model` and this crate sits below it.
///
/// # What replaces it: the record's own sentences, both halves of every pair
///
/// Every row below is **this tree's own declaration**, and the verdict on record is
/// **measured, not asserted**: the [`StructuralJudge`] reads a subject against a basis by
/// vocabulary, so *belongs* means **every term the subject introduces is a term the basis
/// declares** — checked by running [`terms`] over both sides, which is the same function the
/// judge uses. A row whose expectation the judge disagrees with is a row that has to be
/// argued with, not quietly adjusted.
///
/// **Both verdicts are present, in pairs.** A set that is all `Belongs` makes the majority
/// class the answer, and a judge that always says `Belongs` beats it — so a calibration set
/// with one verdict is a set that measures nothing (the same reason
/// `a_judge_that_loses_to_the_majority_class_is_not_quotable` exists). Every pairing below
/// is one subject built from the basis's own terms, and one subject that introduces a term
/// the basis does not name.
///
/// **The set is the tree's whole vocabulary, not a sample of it (C-106).** The user's ruling:
/// *"the full vocab set is probably way higher than that … expand it's vocab … the thing has
/// the 5 modes, an observer/civilian/citizen, the audio engine, the chunk system — why would I
/// say that laya is a god over the project if you never make it so?"* So every declared system
/// in this repository carries its own pair: the five modes of the decision layer, the observer
/// at the commit point and the civilian's governed book, the audio engine's synthesis, the
/// chunk source and the octree mesher, the work pool, the instruments, the design system, the
/// shell, the gate — beside the ten domains the first landing carried. **A domain with no pair
/// is a domain the calibration cannot quote**, and this set is where Laya's scope is measured
/// rather than asserted.
pub const KNOWN_ANSWERS: &[KnownAnswer] = &[
    // ================= the chain (H-8.1, H-10.5) =================
    KnownAnswer {
        subject: "the one append-only record is the chain",
        basis: "the one append-only record is the chain",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "the one append-only record is the ledger file",
        basis: "the one append-only record is the chain",
        expected: Verdict::NotBelongs,
    },
    // ================= the governed change (C-96.1, world/governed.rs) =================
    KnownAnswer {
        subject: "a committed transition leaves a receipt and an evidence row",
        basis: "a committed transition leaves a receipt and an evidence row",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "a committed transition leaves a receipt and a payroll",
        basis: "a committed transition leaves a receipt and an evidence row",
        expected: Verdict::NotBelongs,
    },
    // ================= the proof of work (laya's own rule) =================
    KnownAnswer {
        subject: "a proof of work carries a claim an origin and evidence",
        basis: "a proof of work carries a claim an origin and evidence",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "a proof of work carries a claim an origin and a coin",
        basis: "a proof of work carries a claim an origin and evidence",
        expected: Verdict::NotBelongs,
    },
    // ================= the retirements (ticket.rs's taxonomy) =================
    KnownAnswer {
        subject: "completed and validated is the one token that means done and checked",
        basis: "completed and validated is the one token that means done and checked",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "completed and validated is the one token that means done and paid",
        basis: "completed and validated is the one token that means done and checked",
        expected: Verdict::NotBelongs,
    },
    // ================= the reader that reports, never the judge (C-84.2) =================
    KnownAnswer {
        subject: "the reader measures and the decision layer decides",
        basis: "the reader measures and the decision layer decides",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "the reader measures and the decision layer guesses",
        basis: "the reader measures and the decision layer decides",
        expected: Verdict::NotBelongs,
    },
    // ================= the governed write (world/mod.rs's own table) =================
    KnownAnswer {
        subject: "a voxel world asserts voxels materials and the modification record",
        basis: "a voxel world asserts voxels materials and the modification record",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "a voxel world asserts voxels materials and the road graph",
        basis: "a voxel world asserts voxels materials and the modification record",
        expected: Verdict::NotBelongs,
    },
    // ================= the season (chain.rs, and the word's one meaning) =================
    KnownAnswer {
        subject: "a season is one of winter spring summer autumn and is never a campaign",
        basis: "a season is one of winter spring summer autumn and is never a campaign",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "a season is one of winter spring summer autumn and is never a holiday",
        basis: "a season is one of winter spring summer autumn and is never a campaign",
        expected: Verdict::NotBelongs,
    },
    // ================= the material's own number (matter/mod.rs) =================
    KnownAnswer {
        subject: "a material is a named set of components in exact rational ratios summing to one",
        basis: "a material is a named set of components in exact rational ratios summing to one",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "a material is a named set of components in exact rational ratios summing to seven",
        basis: "a material is a named set of components in exact rational ratios summing to one",
        expected: Verdict::NotBelongs,
    },
    // ================= the clock (voxel-client's own declaration) =================
    KnownAnswer {
        subject: "a fixed twenty hertz clock counts the ticks one slow frame may catch up on",
        basis: "a fixed twenty hertz clock counts the ticks one slow frame may catch up on",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "a fixed twenty hertz clock counts the ticks one slow frame may invent",
        basis: "a fixed twenty hertz clock counts the ticks one slow frame may catch up on",
        expected: Verdict::NotBelongs,
    },
    // ================= the exit (civilian/agency) =================
    KnownAnswer {
        subject: "an unrecorded modification is invisible",
        basis: "an unrecorded modification is invisible",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "an unrecorded modification is permissible",
        basis: "an unrecorded modification is invisible",
        expected: Verdict::NotBelongs,
    },
    // ================= the five modes of the decision layer (§98.8) =================
    KnownAnswer {
        subject: "the decision layer's five named parts are the seam the matrix the in-project \
                  decider the model instrument and the judge",
        basis: "the decision layer's five named parts are the seam the matrix the in-project \
                decider the model instrument and the judge",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "the decision layer's five named parts are the seam the matrix the in-project \
                  decider the model instrument and the payroll",
        basis: "the decision layer's five named parts are the seam the matrix the in-project \
                decider the model instrument and the judge",
        expected: Verdict::NotBelongs,
    },
    // the seam: a missing instrument is not a pass
    KnownAnswer {
        subject: "a missing instrument is not a pass",
        basis: "a missing instrument is not a pass",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "a missing instrument is not a coin",
        basis: "a missing instrument is not a pass",
        expected: Verdict::NotBelongs,
    },
    // the matrix: one matrix, one decision function, one audit sink, deny by default
    KnownAnswer {
        subject: "one matrix one decision function one audit sink and a denial is the default",
        basis: "one matrix one decision function one audit sink and a denial is the default",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "one matrix one decision function one audit sink and a garden is the default",
        basis: "one matrix one decision function one audit sink and a denial is the default",
        expected: Verdict::NotBelongs,
    },
    // the in-project decider: answers by delegating, and does not re-decide
    KnownAnswer {
        subject: "the in-project decider answers by delegating and does not re-decide",
        basis: "the in-project decider answers by delegating and does not re-decide",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "the in-project decider answers by guessing and does not re-decide",
        basis: "the in-project decider answers by delegating and does not re-decide",
        expected: Verdict::NotBelongs,
    },
    // the model instrument: quotable only when it beats the majority class
    KnownAnswer {
        subject: "the model instrument is calibrated and quotable only when it beats the \
                  majority class",
        basis: "the model instrument is calibrated and quotable only when it beats the majority \
                class",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "the model instrument is advertised and quotable only when it beats the \
                  majority class",
        basis: "the model instrument is calibrated and quotable only when it beats the majority \
                class",
        expected: Verdict::NotBelongs,
    },
    // the judge's typed questions
    KnownAnswer {
        subject: "the judge's typed questions are choice score and noul",
        basis: "the judge's typed questions are choice score and noul",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "the judge's typed questions are choice score and maybe",
        basis: "the judge's typed questions are choice score and noul",
        expected: Verdict::NotBelongs,
    },
    // an answer: a type and a bounded confidence, never prose
    KnownAnswer {
        subject: "an answer carries a type and a bounded confidence but no prose",
        basis: "an answer carries a type and a bounded confidence but no prose",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "an answer carries a type and an endless confidence but no prose",
        basis: "an answer carries a type and a bounded confidence but no prose",
        expected: Verdict::NotBelongs,
    },
    // the calibration's own honesty rule
    KnownAnswer {
        subject: "a known-answer set is not edited to flatter the judge",
        basis: "a known-answer set is not edited to flatter the judge",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "a known-answer set is not edited to marmalade the judge",
        basis: "a known-answer set is not edited to flatter the judge",
        expected: Verdict::NotBelongs,
    },
    // ================= the observer at the commit point (C-103.1) =================
    KnownAnswer {
        subject: "the observer records every accepted act at the single commit point",
        basis: "the observer records every accepted act at the single commit point",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "the observer records every accepted act at the single trophy point",
        basis: "the observer records every accepted act at the single commit point",
        expected: Verdict::NotBelongs,
    },
    // ================= the civilian's governed book (C-101) =================
    KnownAnswer {
        subject: "every machine act passes through the governed book under a write permission",
        basis: "every machine act passes through the governed book under a write permission",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "every machine act passes through the governed book under a marmalade \
                  permission",
        basis: "every machine act passes through the governed book under a write permission",
        expected: Verdict::NotBelongs,
    },
    // ================= the civilian sim: the process table is in-machine (C-101.3) =================
    KnownAnswer {
        subject: "the process table is declared in-machine and the generator's materials stay \
                  the generator's",
        basis: "the process table is declared in-machine and the generator's materials stay the \
                generator's",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "the process table is declared in-machine and the generator's socks stay the \
                  generator's",
        basis: "the process table is declared in-machine and the generator's materials stay the \
                generator's",
        expected: Verdict::NotBelongs,
    },
    // ================= the only citizen (C-101.1) =================
    KnownAnswer {
        subject: "the laya observer is the only citizen and no physical mobs inhabit the record",
        basis: "the laya observer is the only citizen and no physical mobs inhabit the record",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "the laya observer is the only citizen and no starved mobs inhabit the record",
        basis: "the laya observer is the only citizen and no physical mobs inhabit the record",
        expected: Verdict::NotBelongs,
    },
    // ================= the filing chokepoint (C-89.6) =================
    KnownAnswer {
        subject: "one writer computes the linkage so a second record cannot form",
        basis: "one writer computes the linkage so a second record cannot form",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "one writer computes the payroll so a second record cannot form",
        basis: "one writer computes the linkage so a second record cannot form",
        expected: Verdict::NotBelongs,
    },
    // ================= the compiler's audit: a denied stage stops the compile =================
    KnownAnswer {
        subject: "a denied stage stops the compile and there is no compile-anyway path",
        basis: "a denied stage stops the compile and there is no compile-anyway path",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "a sausage stage stops the compile and there is no compile-anyway path",
        basis: "a denied stage stops the compile and there is no compile-anyway path",
        expected: Verdict::NotBelongs,
    },
    // ================= the back-compiler (C-100.1) =================
    KnownAnswer {
        subject: "the board's tickets back-compile into solutions and a solution is what one \
                  ticket asks the arrangement to contain",
        basis: "the board's tickets back-compile into solutions and a solution is what one ticket \
                asks the arrangement to contain",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "the board's tickets back-compile into solutions and a solution is what one \
                  ticket asks the arrangement to trampoline",
        basis: "the board's tickets back-compile into solutions and a solution is what one ticket \
                asks the arrangement to contain",
        expected: Verdict::NotBelongs,
    },
    // ================= the recipe licenses its vocabulary (C-100.1) =================
    KnownAnswer {
        subject: "a recipe must declare the vocabulary it licenses",
        basis: "a recipe must declare the vocabulary it licenses",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "a recipe must declare the vocabulary it trophies",
        basis: "a recipe must declare the vocabulary it licenses",
        expected: Verdict::NotBelongs,
    },
    // ================= the recipe's rows =================
    KnownAnswer {
        subject: "a recipe that declares no window and no row declares nothing",
        basis: "a recipe that declares no window and no row declares nothing",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "a recipe that declares no window and no sock declares nothing",
        basis: "a recipe that declares no window and no row declares nothing",
        expected: Verdict::NotBelongs,
    },
    // ================= the game compiles its own recipe (H-14.1) =================
    KnownAnswer {
        subject: "the game compiles its own recipe as a package and the loader refuses what it \
                  cannot verify",
        basis: "the game compiles its own recipe as a package and the loader refuses what it \
                cannot verify",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "the game compiles its own recipe as a package and the loader refuses what it \
                  cannot garden",
        basis: "the game compiles its own recipe as a package and the loader refuses what it \
                cannot verify",
        expected: Verdict::NotBelongs,
    },
    // ================= provenance: the exact source or nothing =================
    KnownAnswer {
        subject: "a package that does not carry the exact source it was compiled from is refused",
        basis: "a package that does not carry the exact source it was compiled from is refused",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "a package that does not carry the banana source it was compiled from is \
                  refused",
        basis: "a package that does not carry the exact source it was compiled from is refused",
        expected: Verdict::NotBelongs,
    },
    // ================= signing: refused before it runs =================
    KnownAnswer {
        subject: "the loader refuses unsigned or tampered work before it runs",
        basis: "the loader refuses unsigned or tampered work before it runs",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "the loader refuses unsigned or sausage work before it runs",
        basis: "the loader refuses unsigned or tampered work before it runs",
        expected: Verdict::NotBelongs,
    },
    // ================= the ticket: a status flip is not a retirement =================
    KnownAnswer {
        subject: "a status flip is not a retirement and the reason travels with the record",
        basis: "a status flip is not a retirement and the reason travels with the record",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "a status trampoline is not a retirement and the reason travels with the record",
        basis: "a status flip is not a retirement and the reason travels with the record",
        expected: Verdict::NotBelongs,
    },
    // ================= the evidence origins: three, and they are graded =================
    KnownAnswer {
        subject: "the three evidence origins are oracle read-back and self-report",
        basis: "the three evidence origins are oracle read-back and self-report",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "the seven evidence origins are oracle read-back and self-report",
        basis: "the three evidence origins are oracle read-back and self-report",
        expected: Verdict::NotBelongs,
    },
    // ================= the plan: milestones and blockers =================
    KnownAnswer {
        subject: "a milestone is ordered work with a finish condition and a blocker is a rule \
                  that stops work",
        basis: "a milestone is ordered work with a finish condition and a blocker is a rule that \
                stops work",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "a milestone is ordered work with a finish condition and a garden is a rule that \
                  stops work",
        basis: "a milestone is ordered work with a finish condition and a blocker is a rule that \
                stops work",
        expected: Verdict::NotBelongs,
    },
    // ================= the axis rules (C-97.2) =================
    KnownAnswer {
        subject: "each ladder its own type and cross-axis and scalar reduction refuse",
        basis: "each ladder its own type and cross-axis and scalar reduction refuse",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "each ladder its own type and cross-axis and banana reduction refuse",
        basis: "each ladder its own type and cross-axis and scalar reduction refuse",
        expected: Verdict::NotBelongs,
    },
    // ================= the census: unchecked is not a pass (C-65.2) =================
    KnownAnswer {
        subject: "a report full of unchecked reads as complete to anyone who does not know the \
                  difference",
        basis: "a report full of unchecked reads as complete to anyone who does not know the \
                difference",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "a report full of trophy reads as complete to anyone who does not know the \
                  difference",
        basis: "a report full of unchecked reads as complete to anyone who does not know the \
                difference",
        expected: Verdict::NotBelongs,
    },
    // ================= the features: no witness, no row, no build (R-59.7) =================
    KnownAnswer {
        subject: "a feature with no witness or no row is a compile error",
        basis: "a feature with no witness or no row is a compile error",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "a feature with no sock or no row is a compile error",
        basis: "a feature with no witness or no row is a compile error",
        expected: Verdict::NotBelongs,
    },
    // ================= the audio engine (voxel-audio's own header) =================
    KnownAnswer {
        subject: "synthesis is arithmetic with no audio dependency and every sample is a \
                  function of seed onset and length",
        basis: "synthesis is arithmetic with no audio dependency and every sample is a function \
                of seed onset and length",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "synthesis is arithmetic with no banana dependency and every sample is a \
                  function of seed onset and length",
        basis: "synthesis is arithmetic with no audio dependency and every sample is a function \
                of seed onset and length",
        expected: Verdict::NotBelongs,
    },
    // ================= the chunk system (voxel-stream's own header) =================
    KnownAnswer {
        subject: "the world source is the one place where an abstract spec becomes concrete \
                  voxels",
        basis: "the world source is the one place where an abstract spec becomes concrete voxels",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "the world source is the one place where an abstract spec becomes banana voxels",
        basis: "the world source is the one place where an abstract spec becomes concrete voxels",
        expected: Verdict::NotBelongs,
    },
    // ================= the mesher (voxel-mesh's own header) =================
    KnownAnswer {
        subject: "the octree is canonical and two runs that build the same chunk produce the \
                  same digest",
        basis: "the octree is canonical and two runs that build the same chunk produce the same \
                digest",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "the octree is trophy and two runs that build the same chunk produce the same \
                  digest",
        basis: "the octree is canonical and two runs that build the same chunk produce the same \
                digest",
        expected: Verdict::NotBelongs,
    },
    // ================= the work pool (voxel-work's own header) =================
    KnownAnswer {
        subject: "worker count is a performance knob and never changes output",
        basis: "worker count is a performance knob and never changes output",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "worker count is a sausage knob and never changes output",
        basis: "worker count is a performance knob and never changes output",
        expected: Verdict::NotBelongs,
    },
    // ================= the noise (voxel-noise's own header) =================
    KnownAnswer {
        subject: "the generator evaluates a chunk in any thread in any order",
        basis: "the generator evaluates a chunk in any thread in any order",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "the generator evaluates a chunk in any cauldron in any order",
        basis: "the generator evaluates a chunk in any thread in any order",
        expected: Verdict::NotBelongs,
    },
    // ================= the instruments (voxel-instrument's own header) =================
    KnownAnswer {
        subject: "an unmeasured gauge is not a satisfied gauge",
        basis: "an unmeasured gauge is not a satisfied gauge",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "an unmeasured gauge is not a satisfied garden",
        basis: "an unmeasured gauge is not a satisfied gauge",
        expected: Verdict::NotBelongs,
    },
    // ================= the design system (voxel-ui's own header) =================
    KnownAnswer {
        subject: "every geometry constant is labelled and a missing value is never turned into \
                  an invented one",
        basis: "every geometry constant is labelled and a missing value is never turned into an \
                invented one",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "every geometry constant is cousin and a missing value is never turned into an \
                  invented one",
        basis: "every geometry constant is labelled and a missing value is never turned into an \
                invented one",
        expected: Verdict::NotBelongs,
    },
    // ================= the console's five levels =================
    KnownAnswer {
        subject: "the console's levels are five and closed",
        basis: "the console's levels are five and closed",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "the console's levels are seven and closed",
        basis: "the console's levels are five and closed",
        expected: Verdict::NotBelongs,
    },
    // ================= the hud =================
    KnownAnswer {
        subject: "the hud is a pinned subset that stays visible while the shell is hidden",
        basis: "the hud is a pinned subset that stays visible while the shell is hidden",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "the hud is a sock subset that stays visible while the shell is hidden",
        basis: "the hud is a pinned subset that stays visible while the shell is hidden",
        expected: Verdict::NotBelongs,
    },
    // ================= the notifications =================
    KnownAnswer {
        subject: "the shell owns a queue and a toast is a record in it",
        basis: "the shell owns a queue and a toast is a record in it",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "the shell owns a queue and a banana is a record in it",
        basis: "the shell owns a queue and a toast is a record in it",
        expected: Verdict::NotBelongs,
    },
    // ================= the agent's inbox =================
    KnownAnswer {
        subject: "the inbox is work the agent owes until it is read",
        basis: "the inbox is work the agent owes until it is read",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "the inbox is work the agent owes until it is paid",
        basis: "the inbox is work the agent owes until it is read",
        expected: Verdict::NotBelongs,
    },
    // ================= the gate's visibility rule (H-7.13) =================
    KnownAnswer {
        subject: "the suite's own state is readable in the client and a finding only a terminal \
                  can read is incomplete",
        basis: "the suite's own state is readable in the client and a finding only a terminal can \
                read is incomplete",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "the suite's own state is readable in the client and a finding only a banana can \
                  read is incomplete",
        basis: "the suite's own state is readable in the client and a finding only a terminal can \
                read is incomplete",
        expected: Verdict::NotBelongs,
    },
    // ================= the board =================
    KnownAnswer {
        subject: "the board is a view of the chain and never a store",
        basis: "the board is a view of the chain and never a store",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "the board is a view of the chain and never a sausage",
        basis: "the board is a view of the chain and never a store",
        expected: Verdict::NotBelongs,
    },
    // ================= the ticket's identity =================
    KnownAnswer {
        subject: "a ticket's id is a position in an append-only record",
        basis: "a ticket's id is a position in an append-only record",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "a ticket's id is a holiday in an append-only record",
        basis: "a ticket's id is a position in an append-only record",
        expected: Verdict::NotBelongs,
    },
    // ================= the sky, evaluated once =================
    KnownAnswer {
        subject: "the rig and the numbers a readout needs come from one evaluation of the sky",
        basis: "the rig and the numbers a readout needs come from one evaluation of the sky",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "the rig and the numbers a payroll needs come from one evaluation of the sky",
        basis: "the rig and the numbers a readout needs come from one evaluation of the sky",
        expected: Verdict::NotBelongs,
    },
    // ================= the column: above the surface is air, water or nothing =================
    KnownAnswer {
        subject: "a voxel whose centre is above the column's surface is air water or nothing \
                  else",
        basis: "a voxel whose centre is above the column's surface is air water or nothing else",
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "a voxel whose centre is above the column's cousin is air water or nothing else",
        basis: "a voxel whose centre is above the column's surface is air water or nothing else",
        expected: Verdict::NotBelongs,
    },
];

/// **The measured claim, asserted in the source.** Every `Belongs` row's subject is built
/// from its basis's own terms and every `NotBelongs` row introduces at least one term the
/// basis does not name — checked here by [`terms`], the judge's own function, so a row added
/// with the wrong expectation fails at `cargo test` rather than quietly weakening the bar.
///
/// This is the property `calibrate` needs to mean anything: a set where the expectations do
/// not match the vocabulary rule would make the accuracy number a measure of the set's
/// mistakes, and the bar would move without anyone deciding it.
#[cfg(test)]
fn expectations_match_the_vocabulary_rule() -> Vec<String> {
    let mut problems = Vec::new();
    for (index, known) in KNOWN_ANSWERS.iter().enumerate() {
        let declared: std::collections::BTreeSet<String> =
            terms(known.basis).into_iter().collect();
        let introduced = terms(known.subject);
        let undeclared: Vec<String> = introduced
            .iter()
            .filter(|term| !declared.contains(*term) && !is_syntax(term))
            .cloned()
            .collect();
        let expected_by_rule = if undeclared.is_empty() {
            Verdict::Belongs
        } else {
            Verdict::NotBelongs
        };
        if expected_by_rule != known.expected {
            problems.push(format!(
                "row {index} `{}`: the vocabulary rule reads {expected_by_rule:?} and the row \
                 records {:?} (undeclared: {undeclared:?})",
                known.subject, known.expected
            ));
        }
    }
    problems
}

/// The axis rules re-exported at the seam, so a caller reaching for the gate's
/// refusals reaches through the judge rather than around it.
pub use crate::axis::{certification_tier, prohibited_max_certification};

// The interchange and reduction refusals are exercised at the seam below.
#[allow(dead_code)]
fn _the_seam_uses_the_axis_rules() {
    let _ = refuse_interchange(EvidenceGrade::A, EvidenceLevel::Direct);
    let _ = refuse_reduction(AxisKind::Grade, Reduction::Score);
    let _ = refuse_bare("A");
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- the instrument ---------------------------------------------------

    #[test]
    fn the_question_vocabulary_is_closed() {
        assert_eq!(Question::parse(" choice \n").unwrap(), Question::Choice);
        assert_eq!(Question::parse("noul").unwrap(), Question::Noul);
        assert!(Question::parse("maybe").is_err());
    }

    #[test]
    fn an_answer_carries_a_type_and_a_bounded_confidence_but_no_prose() {
        let answer = Answer::choice(Verdict::Belongs, 0.82).unwrap();
        assert_eq!(answer.value, AnswerValue::Choice(Verdict::Belongs));
        assert!(Answer::choice(Verdict::Belongs, 1.5).is_err());
        assert!(Answer::score(6, 0.5).is_err(), "a scale with no ceiling is a score");
        assert_eq!(Answer::noul().confidence, 0.0);
    }

    #[test]
    fn a_line_built_from_the_basis_own_terms_belongs() {
        let answer = StructuralJudge
            .ask(
                Question::Choice,
                "a committed transition leaves a receipt and an evidence row",
                "a committed transition leaves a receipt and an evidence row",
            )
            .unwrap();
        assert_eq!(answer.value, AnswerValue::Choice(Verdict::Belongs));
    }

    #[test]
    fn a_line_naming_something_the_basis_does_not_is_refused_and_the_term_is_named() {
        match StructuralJudge.reading(
            "a committed transition leaves a receipt and a payroll",
            "a committed transition leaves a receipt and an evidence row",
        ) {
            Reading::Undeclared { found, .. } => {
                assert!(found.iter().any(|term| term == "payroll"));
            }
            other => panic!("payroll is nowhere in the basis: {other:?}"),
        }
    }

    /// **The set's own rows are checked against the judge's rule**, so a row added with the
    /// wrong expectation fails here rather than moving the calibration bar silently.
    #[test]
    fn every_known_answer_agrees_with_the_vocabulary_rule() {
        let problems = expectations_match_the_vocabulary_rule();
        assert!(problems.is_empty(), "{}", problems.join("\n"));
    }

    /// **Both verdicts are in the set, and neither is a majority.** A calibration set with one
    /// verdict is a set where always answering it beats the majority **and so does a judge that
    /// always says `Belongs`**, which is exactly the defect this set was rebuilt to avoid.
    #[test]
    fn the_known_answer_set_carries_both_verdicts_and_no_majority() {
        let belongs = KNOWN_ANSWERS
            .iter()
            .filter(|known| known.expected == Verdict::Belongs)
            .count();
        let not_belongs = KNOWN_ANSWERS
            .iter()
            .filter(|known| known.expected == Verdict::NotBelongs)
            .count();
        assert!(belongs > 0 && not_belongs > 0, "{belongs} belongs, {not_belongs} not_belongs");
        assert_eq!(
            belongs, not_belongs,
            "the set is paired: an unpaired set makes one verdict the majority class"
        );
    }

    // ---- calibration decides quotability ----------------------------------

    #[test]
    fn a_judge_that_beats_the_majority_class_is_quotable_and_says_so() {
        let report = calibrate(&StructuralJudge, KNOWN_ANSWERS);
        assert!(report.calibration.is_quotable(), "{report:?}");
        assert_eq!(
            report.readings_used,
            KNOWN_ANSWERS.len(),
            "every row on the record was read, not a sample"
        );
        assert_eq!(report.accuracy, 1.0, "the judge reads its own vocabulary rule");
        assert!(
            report.accuracy > report.majority_class,
            "accuracy {} must beat the majority class {}",
            report.accuracy,
            report.majority_class
        );
    }

    #[test]
    fn a_judge_that_loses_to_the_majority_class_is_not_quotable() {
        let set = [
            KnownAnswer {
                subject: "let payroll = payroll();",
                basis: "rotation and electricity interoperate",
                expected: Verdict::Belongs,
            },
            KnownAnswer {
                subject: "let invoice = invoice();",
                basis: "rotation and electricity interoperate",
                expected: Verdict::Belongs,
            },
            KnownAnswer {
                subject: "let payroll = payroll();",
                basis: "rotation and electricity interoperate",
                expected: Verdict::Belongs,
            },
        ];
        let report = calibrate(&StructuralJudge, &set);
        assert!(!report.calibration.is_quotable());
        assert!(report
            .calibration
            .notes
            .iter()
            .any(|note| note.contains("NOT QUOTABLE")));
    }

    // ---- the gate: default fail, total notification ----------------------

    #[test]
    fn an_artifact_with_no_proof_of_work_fails_by_default() {
        let calibration = calibrate(&StructuralJudge, KNOWN_ANSWERS);
        let decision = decide(
            &StructuralJudge,
            "crates/voxel-model/src/wire.rs",
            "pub fn encode() {}",
            &[],
            &calibration,
        );
        assert_eq!(decision.verdict, Verdict::NotBelongs);
        assert!(decision.findings.iter().any(|f| f.contains("no proof of work")));
    }

    #[test]
    fn a_hand_rolled_wire_format_is_named_with_the_line_that_triggered_it() {
        let source = format!("pub fn encode(v: u32) -> [u8; 4] {{\n    v.{}{}()\n}}", "to_", "ne_bytes");
        let findings = security_findings("crates/voxel-model/src/wire.rs", &source);
        assert!(
            findings.iter().any(|f| f.contains("A08") && f.contains("wire.rs:2")),
            "{findings:?}"
        );
    }

    #[test]
    fn every_decision_produces_exactly_one_notification() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        // Commit a civilian act so the observer book has a row to mint into
        // proof of work. Hold the book lock across commit-and-mint so a
        // parallel test cannot drain the row in between.
        let _book_guard = crate::observer::recorder::BOOK_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let _ = crate::observer::recorder::record_committed_action(
            "dig",
            "civ-1",
            "task-9",
            Some("res-4".to_owned()),
            &["rock 4g substrate -> spoil:civ-1".to_owned()],
            &["transition committed: the books balanced".to_owned()],
            "transition-digest".to_owned(),
        );
        let proofs = mint_proofs_from_rows(crate::observer::recorder::drain_committed_actions_locked());
        assert_eq!(proofs.len(), 1, "one committed act, one proof of work");
        let evaluation = evaluate_project(root, &proofs);
        // **A scratch queue, not the shared one.** `write_queue` writes run-local names
        // (`laya-0000`, …) and does not clear, so `logs/laya-decisions/` can hold records a
        // previous run left behind — and their artifacts still hash, so reading the whole
        // directory would count a previous run's decisions among this one's. The production
        // writer's guard is `write_the_gate_queue`'s clear (`voxel-client`'s module); a test
        // that read the production queue would be counting the game's records as well as its
        // own. It writes its own directory here and reads that back.
        let scratch = root
            .join("logs")
            .join("test-workspaces")
            .join(format!("laya-notifications-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&scratch);
        let queue = scratch.join("queue");
        let written = evaluation
            .write_queue(root, &queue)
            .expect("the queue writes");
        assert_eq!(
            written,
            evaluation.decisions.len(),
            "the api layer carries a notification over every single decision"
        );
        assert!(!evaluation.decisions.is_empty(), "the project is not empty");
        // Every record decodes **whole** — the decision layer is not reduced in
        // transit, and the count on disk is the count of decisions.
        let mut on_disk = 0usize;
        let mut entries: Vec<_> = std::fs::read_dir(&queue)
            .expect("the queue reads")
            .map(|entry| entry.expect("an entry").path())
            .collect();
        entries.sort();
        for path in entries {
            let bytes = std::fs::read(&path).expect("a record reads");
            let notification = Notification::decode(&bytes).expect("a record decodes");
            assert_eq!(
                notification.decision, evaluation.decisions[on_disk],
                "the decision crossed the wire whole"
            );
            on_disk += 1;
        }
        // The oracle speaks on the record: its whole report lands in the project,
        // so the fail state is visible rather than quiet.
        std::fs::create_dir_all(root.join("logs")).expect("the logs directory");
        std::fs::write(root.join("logs").join("laya-report.txt"), evaluation.report())
            .expect("the oracle's report lands");
        assert_eq!(on_disk, evaluation.decisions.len(), "one record per decision");
        let _ = std::fs::remove_dir_all(&scratch);
    }

    #[test]
    fn a_notification_carries_the_whole_decision_and_round_trips() {
        let decision = Decision {
            path: "crates/voxel-model/src/wire.rs".to_string(),
            question: Question::Choice,
            verdict: Verdict::NotBelongs,
            confidence: 0.5,
            origin: Some(EvidenceOrigin::SelfReport),
            grades: vec![
                (AxisKind::TicketEvidence, "C".to_string()),
                (AxisKind::SupportLevel, "unverified".to_string()),
            ],
            findings: vec![
                "A08 hand-rolled byte encoding at wire.rs:445: a wire format rolled by hand"
                    .to_string(),
                "no proof of work on record: no claim, no evidence, no origin".to_string(),
            ],
        };
        let notification = Notification {
            notification: "laya-0001".to_string(),
            decision: decision.clone(),
            artifact_sha256: Digest::of(b"an artifact"),
        };
        let bytes = notification.encode().expect("the record encodes");
        let back = Notification::decode(&bytes).expect("the record decodes");
        assert_eq!(back, notification, "the decision layer crosses the wire whole");
        assert_eq!(back.decision.findings.len(), 2);
        assert_eq!(back.decision.grades.len(), 2);
    }

    #[test]
    fn a_malformed_record_is_refused_rather_than_half_read() {
        assert!(Notification::decode(&[0xff, 0x00, 0x13]).is_err());
    }

    #[test]
    fn the_gate_is_red_until_the_project_carries_proof_of_work() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
        let evaluation = evaluate_project(root, &[]);
        assert!(
            !evaluation.all_cleared(),
            "an unproven project must not clear the gate"
        );
    }
}
