//! **U7's seam: the question, the answer, the ballot, and the four rules that make a non-reading fail.**
//!
//! Derived from **documents** (Constitution P1/P7):
//!
//! | Source | What it declares here |
//! |---|---|
//! | `docs/LEDGER.md` R-59.5, and §59.5 item 9, which states the **four fail-closed rules** | *an uncalibrated run writes nothing; a missing or unreachable oracle is refused; no reading at all is refused; a `not_belongs` reading is refused unless a human's reason is recorded in the same act, and the reason travels with the finding.* Also the closed three-word `Verdict` and the bar: **the majority class of the known-answer set this project actually ships, not a fixed number**. |
//! | `docs/LEDGER.md` §59.5 item 10 | a verdict vocabulary is closed — an undeclared word is refused rather than coerced — and `no_reading` is a **third word**, not a synonym for either of the other two. |
//! | `docs/HYPERGRILLING.md` H-14.1 and H-14.2 | the seam's own shape: the layer is one seam every instrument implements (`Subject`/`Decider`/`Assessment` there, [`StructuralJudge`]/[`Judgement`] here), an answer is **evidence and never authority**, and a `not_belongs` reading needs a human's reason in the same act. |
//! | `patches/PATCH-023-the-ballot-and-the-von-classifier.md` | the ballot's declared fields and its refusals: a choice over fewer than two options, a score over fewer than two rungs, a `passes` naming an undeclared option, a floor above the top rung, an empty claim/artifact/instruction/presenter, a `max-age` of zero — *"not a window"* — and, above all, that **the origin is the proof's to carry, not the claimant's to declare**. |
//! | `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §2 (U7) and §5 row 6 | the interface and the bar: `Question::{Choice,Score,Noul}`, `Verdict::{Belongs,NotBelongs,NoReading}`, `Ballot`, `StructuralJudge`; **a ballot answered with `Origin::Oracle`; the calibration accuracy beats the majority class**. |
//!
//! # Why the floor is a declaration and not a comparison
//!
//! *"The model chose such-and-such"* is not a verdict until someone says which answers count — and a floor
//! chosen after the answer exists is a floor chosen to fit it. So a [`Ballot`] declares its passing option
//! (`Choice`), its lowest passing rung (`Score`) or its threshold (`Noul`) **before** anything is asked, and
//! [`Ballot::declare`] refuses a ballot that leaves it out.
//!
//! # What an answer carries
//!
//! An [`Answer`] is a verdict, the [`Origin`] that gave it, and [`Evidence`]: the artifact's **digest** (what
//! was read, in place), the declared features that held, every option's score, and the calibration's own
//! figures. A model's answer is evidence, not authority — and an answer whose *calibration* does not beat the
//! majority class is not handed out at all ([`RefusalKind::NotEstablished`]), because a judge that scores
//! below the commonest answer in its own known-answer set is worse than always guessing.

use std::path::Path;

use crate::digest::Digest;
use crate::refusal::{Refusal, RefusalKind, RefusalSet};

pub use crate::axis::Origin;

/// **The closed three-word verdict** (§59.5 item 10). Three words and no prose: a verdict that arrived with
/// a sentence attached would be quotable as an argument, and this vocabulary exists to prevent exactly that.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Verdict {
    /// The answer landed on the answer the question says counts.
    Belongs,
    /// It landed elsewhere — and this one needs a human's reason before it can be recorded.
    NotBelongs,
    /// Nothing was read: a shape the question does not apply to, a body carrying nothing to read, or a
    /// reading that came out a tie. **A non-reading is never a pass.**
    NoReading,
}

impl Verdict {
    /// Every word this vocabulary declares.
    pub const ALL: [Verdict; 3] = [Verdict::Belongs, Verdict::NotBelongs, Verdict::NoReading];

    /// The recorded name — the wire word, and the word the gate prints.
    pub const fn name(self) -> &'static str {
        match self {
            Verdict::Belongs => "belongs",
            Verdict::NotBelongs => "not_belongs",
            Verdict::NoReading => "no_reading",
        }
    }

    /// The verdict a word names, or nothing: an undeclared word is refused rather than coerced.
    pub fn parse(word: &str) -> Option<Self> {
        Verdict::ALL.iter().copied().find(|verdict| verdict.name() == word)
    }

    /// Whether this is a verdict at all — `NoReading` is the third word, and it is not a reading of the
    /// thing the question asked about.
    pub const fn is_a_verdict(self) -> bool {
        !matches!(self, Verdict::NoReading)
    }
}

/// **The question, typed** — and there are exactly three instruments, because an answer that is not one of
/// these three cannot be checked against a declared floor at all.
#[derive(Clone, Debug, PartialEq)]
pub enum Question {
    /// Which of these answers is the answer: a closed list, and the option that counts as passing.
    Choice {
        /// The options, in the order the question declares them.
        options: Vec<String>,
        /// The option that counts as passing — declared before the answer exists.
        passes: String,
    },
    /// Where on an ordered ladder of rungs the answer lands, and the lowest rung that passes.
    Score {
        /// The rungs, lowest first.
        levels: Vec<String>,
        /// The lowest rung that counts as passing.
        floor: String,
    },
    /// Whether a proposition holds, against a declared threshold. `0.5` is the natural midpoint of a
    /// probability of yes; any other value is a choice and carries its reason with it.
    Noul {
        /// The threshold a yes has to reach.
        threshold: f64,
    },
}

impl Question {
    /// The instrument's own name: `choice`, `score` or `noul`.
    pub const fn kind(&self) -> &'static str {
        match self {
            Question::Choice { .. } => "choice",
            Question::Score { .. } => "score",
            Question::Noul { .. } => "noul",
        }
    }

    /// A choice over a closed list of options, with the passing option declared up front.
    ///
    /// Refused by name: fewer than two options (a choice of one is not a choice), an option declared twice
    /// (the second would silently be the first), and a `passes` that names no declared option — the floor
    /// has to be *in* the question or it is a floor invented afterwards.
    pub fn choice(options: &[&str], passes: &str) -> Result<Question, RefusalSet> {
        let mut refusals = RefusalSet::new();
        let options: Vec<String> = options.iter().map(|option| option.trim().to_string()).collect();
        if options.len() < 2 {
            refusals.refuse(
                RefusalKind::BallotIncomplete,
                format!(
                    "a choice over {} option(s) chooses nothing: two options are the least a question with \
                     an answer can have",
                    options.len()
                ),
            );
        }
        for (index, option) in options.iter().enumerate() {
            if options[..index].contains(option) {
                refusals.refuse(
                    RefusalKind::BallotIncomplete,
                    format!(
                        "`{option}` is declared twice: two equal options make one answer look like two and \
                         leave the passing one undecidable"
                    ),
                );
            }
            if option.is_empty() {
                refusals.refuse(
                    RefusalKind::BallotIncomplete,
                    "an empty option is not a term of the question".to_string(),
                );
            }
        }
        if !options.iter().any(|option| option == passes.trim()) {
            refusals.refuse(
                RefusalKind::FloorUndeclared,
                format!(
                    "`passes` names `{passes}`, which this question does not offer: a floor outside the \
                     question's own terms is a floor chosen to fit an answer",
                ),
            );
        }
        refusals.into_result(Question::Choice {
            options,
            passes: passes.trim().to_string(),
        })
    }

    /// A score over declared rungs, with the lowest passing rung named up front.
    pub fn score(levels: &[&str], floor: &str) -> Result<Question, RefusalSet> {
        let mut refusals = RefusalSet::new();
        let levels: Vec<String> = levels.iter().map(|level| level.trim().to_string()).collect();
        if levels.len() < 2 {
            refusals.refuse(
                RefusalKind::BallotIncomplete,
                format!(
                    "a score over {} rung(s) cannot land between anything: two rungs are the least an \
                     ordered ladder can have",
                    levels.len()
                ),
            );
        }
        if !levels.iter().any(|level| level == floor.trim()) {
            refusals.refuse(
                RefusalKind::FloorUndeclared,
                format!(
                    "`floor` names `{floor}`, which is not one of the declared rungs: a floor above the top \
                     rung is a floor no answer can reach"
                ),
            );
        }
        refusals.into_result(Question::Score {
            levels,
            floor: floor.trim().to_string(),
        })
    }

    /// A yes/no question against a declared threshold.
    pub fn noul(threshold: f64) -> Result<Question, RefusalSet> {
        let mut refusals = RefusalSet::new();
        if !(threshold > 0.0 && threshold <= 1.0) {
            refusals.refuse(
                RefusalKind::FloorUndeclared,
                format!(
                    "a threshold of {threshold} is not a threshold: 0.5 is a probability of yes's own \
                     midpoint, and any other value has to be inside (0, 1] to mean anything"
                ),
            );
        }
        refusals.into_result(Question::Noul { threshold })
    }

    /// The option that counts as passing, for the two instruments that have one.
    pub fn passes(&self) -> Option<&str> {
        match self {
            Question::Choice { passes, .. } => Some(passes),
            Question::Score { floor, .. } => Some(floor),
            Question::Noul { .. } => None,
        }
    }

    /// The question's own terms, in order — the options or the rungs, and nothing for a `noul`.
    pub fn terms(&self) -> &[String] {
        match self {
            Question::Choice { options, .. } => options,
            Question::Score { levels, .. } => levels,
            Question::Noul { .. } => &[],
        }
    }
}

/// **A ballot: a question about an artifact, with who is accountable and how long the answer stays current.**
#[derive(Clone, Debug, PartialEq)]
pub struct Ballot {
    id: String,
    claim: String,
    artifact: String,
    instruction: String,
    question: Question,
    presenter: String,
}

impl Ballot {
    /// Declare a ballot. Every field is required, and an empty one is refused rather than defaulted: a
    /// ballot with no accountable presenter, no artifact or no instruction is a question nobody can answer.
    /// A `max-age` of zero is refused with the record's own words — *not a window* — because an answer that
    /// expires before it is read is not an answer.
    ///
    /// **The window is declared and checked, but not carried.** This build cannot enforce staleness — no
    /// tick span or world digest reaches this seam (`patches/PATCH-023` §*Not done*) — so the window is
    /// validated at declaration and **not stored**: a value the layer would never read is not one a ballot
    /// should claim to hold, and the enforcement is **owed by name** rather than carried unread.
    #[allow(clippy::too_many_arguments)]
    pub fn declare(
        id: &str,
        claim: &str,
        artifact: &str,
        instruction: &str,
        question: Question,
        presenter: &str,
        max_age_ticks: u64,
    ) -> Result<Ballot, RefusalSet> {
        let mut refusals = RefusalSet::new();
        for (field, value) in [
            ("id", id),
            ("claim", claim),
            ("artifact", artifact),
            ("instruction", instruction),
            ("presenter", presenter),
        ] {
            if value.trim().is_empty() {
                refusals.refuse(
                    RefusalKind::BallotIncomplete,
                    format!("`{field}` is empty: a ballot declares it or it is not a ballot"),
                );
            }
        }
        if max_age_ticks == 0 {
            refusals.refuse(
                RefusalKind::BallotIncomplete,
                "a `max-age` of 0 is not a window: an answer that expires before it can be read is not a \
                 current answer"
                    .to_string(),
            );
        }
        refusals.into_result(Ballot {
            id: id.trim().to_string(),
            claim: claim.trim().to_string(),
            artifact: artifact.trim().to_string(),
            instruction: instruction.trim().to_string(),
            question,
            presenter: presenter.trim().to_string(),
        })
    }

    /// The id the answer comes back under, and the row a finding cites.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The ticket or ruling the work serves — an unattributed claim is not a claim.
    pub fn claim(&self) -> &str {
        &self.claim
    }

    /// The bytes the answer is about, workspace-relative, so the judgment can be re-derived from them.
    pub fn artifact(&self) -> &str {
        &self.artifact
    }

    /// How to judge the state. The instrument reads this and nothing else about intent.
    pub fn instruction(&self) -> &str {
        &self.instruction
    }

    /// The question, typed.
    pub fn question(&self) -> &Question {
        &self.question
    }

    /// Who is accountable for asking.
    pub fn presenter(&self) -> &str {
        &self.presenter
    }
}

/// **What an instrument read off the artifact** — the option it lands on, every option's score, and the
/// declared features that held. The scores travel because a summary would hide how close the second option
/// was; the features travel because the answer has to be re-derivable without the instrument.
#[derive(Clone, Debug, PartialEq)]
pub struct Judgement {
    /// The option the instrument landed on.
    pub chosen: String,
    /// Every declared option with its own score, in the question's order.
    pub scores: Vec<(String, f64)>,
    /// The declared features that held for this artifact, by name.
    pub features: Vec<&'static str>,
}

/// **The calibration's own figures**, carried by every answer and printed by the gate.
///
/// `leave_one_out` is the figure the bar is read against: each reading is answered by a calibration fitted
/// **without** it. `in_sample` is reported beside it because the two together are the honest picture — a
/// calibration that only knows its own training rows has measured nothing about a record it has not seen.
///
/// `leave_one_out_with_absence` is a **second estimator over the same folds**, reported because
/// `docs/LEDGER.md` R-59.5's honesty rule says the measurement is reported either way and because the two
/// answers differ by more than the bar: the instrument answers with the held features read as evidence of
/// presence, and this field records what the same folds score when the absent features are also read as
/// evidence of absence. A figure that is only ever printed for the estimator that won is not a measurement.
#[derive(Clone, Debug, PartialEq)]
pub struct Measurement {
    /// How many labelled readings the set declares.
    pub readings: usize,
    /// How many readings each declared option has — the majority class is read off this.
    pub per_option: Vec<(&'static str, usize)>,
    /// The share of the commonest declared option. A judge that always answers it scores exactly this.
    pub majority: f64,
    /// The fitted calibration's accuracy on the set it was fitted on.
    pub in_sample: f64,
    /// The accuracy with each reading answered by a calibration fitted without it.
    pub leave_one_out: f64,
    /// The same folds, scored with the **absent** features read as evidence of absence too — the estimator
    /// the instrument does *not* answer with, printed because a measurement is reported either way.
    pub leave_one_out_with_absence: f64,
    /// The same folds asked from the record's **claim** rather than its head. A figure that is only ever
    /// printed for the half of the record the instrument answers from is not a measurement of the instrument:
    /// this one is what says whether the answer is in the claim at all, and here it says it is not.
    pub leave_one_out_from_the_claim_only: f64,
    /// One digest over the declared set and the bytes read in place — a moved record makes the calibration
    /// **stale rather than wrong**.
    pub known_answers: Digest,
}

impl Measurement {
    /// **The bar** (`docs/LEDGER.md` R-59.5): the calibration's accuracy beats the majority class.
    ///
    /// Strictly beats: a judge that always answers the commonest option scores *exactly* the majority class,
    /// so a tie is not a pass and the constant judge cannot reach the bar by existing.
    pub fn beats_majority(&self) -> bool {
        self.leave_one_out > self.majority
    }

    /// Whether the instrument may be quoted. An instrument whose calibration does not beat the majority
    /// class is worse than guessing, so its answers are refused by name rather than handed out with a
    /// caveat (`docs/LEDGER.md` R-59.5's standing honesty rule, and the build gate's own
    /// `model-oracle not-established`).
    pub fn quotable(&self) -> bool {
        self.beats_majority()
    }

    /// The figures as one line, for a build report — both estimators, so the reader sees the spread rather
    /// than the flattering half of it.
    pub fn render(&self) -> String {
        format!(
            "{} reading(s), {} — accuracy {:.4} in sample, {:.4} leave-one-out, {:.4} leave-one-out with \
             absence, {:.4} leave-one-out from the claim only, majority class {:.4}, margin {:+.4}",
            self.readings,
            self.per_option
                .iter()
                .map(|(option, count)| format!("{option} {count}"))
                .collect::<Vec<String>>()
                .join(" / "),
            self.in_sample,
            self.leave_one_out,
            self.leave_one_out_with_absence,
            self.leave_one_out_from_the_claim_only,
            self.majority,
            self.leave_one_out - self.majority
        )
    }
}

/// **The seam** — one trait every instrument implements, so the layer has one way to be asked and one way to
/// be replaced. *A missing instrument is not a pass*: [`Unreachable`] implements the same trait and refuses.
pub trait StructuralJudge {
    /// Read the artifact and answer the question. The instrument may refuse — a shape it does not read is
    /// [`RefusalKind::NoReading`], and an instrument that cannot run at all is
    /// [`RefusalKind::OracleUnreachable`].
    fn judge(&self, question: &Question, artifact: &[u8]) -> Result<Judgement, Refusal>;

    /// The calibration's figures, which every answer carries.
    fn measurement(&self) -> &Measurement;
}

/// **An instrument that cannot run** — the seam's own *missing instrument is not a pass* case, declared as a
/// judge so the refusal is a value rather than a comment. It answers nothing and says why.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unreachable {
    /// What is missing, in the words of whoever built the ballot.
    pub what: &'static str,
}

impl StructuralJudge for Unreachable {
    fn judge(&self, _question: &Question, _artifact: &[u8]) -> Result<Judgement, Refusal> {
        Err(Refusal::new(
            RefusalKind::OracleUnreachable,
            format!(
                "{} is not reachable, so nothing was read: an absent instrument refuses rather than \
                 answering, because an unanswered question and a question answered `belongs` are not the \
                 same fact",
                self.what
            ),
        ))
    }

    fn measurement(&self) -> &Measurement {
        &UNREACHABLE_MEASUREMENT
    }
}

/// The figures an unreachable instrument reports: no readings, and therefore nothing to be calibrated
/// against — which is why it can never be quoted.
static UNREACHABLE_MEASUREMENT: Measurement = Measurement {
    readings: 0,
    per_option: Vec::new(),
    majority: 0.0,
    in_sample: 0.0,
    leave_one_out: 0.0,
    leave_one_out_with_absence: 0.0,
    leave_one_out_from_the_claim_only: 0.0,
    known_answers: Digest::from_bytes([0u8; 32]),
};

/// **What an answer carries besides its verdict**: what was read, and what the reading was made of.
#[derive(Clone, Debug, PartialEq)]
pub struct Evidence {
    /// The artifact, as the ballot named it.
    pub artifact: String,
    /// The digest of the bytes read **in place** at that path.
    pub digest: Digest,
    /// The declared features that held, by name — the reading is re-derivable from these.
    pub features: Vec<&'static str>,
    /// Every declared option with its own score, in the question's order.
    pub scores: Vec<(String, f64)>,
    /// The calibration's own figures, so the verdict's standing is not a sentence beside it.
    pub readings: usize,
    /// The majority class the calibration was read against.
    pub majority: f64,
    /// The leave-one-out accuracy the verdict rests on.
    pub leave_one_out: f64,
    /// A human's reason, required for a `not_belongs` reading and carried with the finding.
    pub reason: Option<String>,
}

/// **An answer: the option it chose, the three words, the origin that gave it, and its evidence.**
///
/// **The chosen option lives here, once.** It is the instrument's own answer ([`Judgement::chosen`]), so a
/// caller *reads* the option rather than re-deriving it from the scores beside it — a second argmax would be
/// a second opinion about one reading, and could disagree with the instrument about a tie.
#[derive(Clone, Debug, PartialEq)]
pub struct Answer {
    chosen: Option<String>,
    verdict: Verdict,
    origin: Origin,
    evidence: Evidence,
}

impl Answer {
    /// **The option the instrument chose**, or `None` for a non-reading: there is no option to have chosen,
    /// and `None` is not an empty string pretending to be one.
    pub fn chosen(&self) -> Option<&str> {
        self.chosen.as_deref()
    }

    /// The verdict.
    pub const fn verdict(&self) -> Verdict {
        self.verdict
    }

    /// **The origin is the proof's to carry, not the claimant's to declare**: no ballot field can set it,
    /// and an answer from an instrument is `Origin::Oracle` whatever the ballot's presenter wished.
    pub const fn origin(&self) -> Origin {
        self.origin
    }

    /// What the answer carries.
    pub const fn evidence(&self) -> &Evidence {
        &self.evidence
    }

    /// Whether the answer landed on the answer the ballot says counts — **and a non-reading is never a
    /// pass**, so this refuses rather than answering `false` for a reading that never was one.
    pub fn passed(&self) -> Result<bool, Refusal> {
        if !self.verdict.is_a_verdict() {
            return Err(Refusal::new(
                RefusalKind::NoReading,
                "the answer is a non-reading, and a non-reading is never a pass: there is no verdict here \
                 for a floor to compare, and answering `false` would report a reading that never happened"
                    .to_string(),
            ));
        }
        Ok(self.verdict == Verdict::Belongs)
    }
}

/// **Ask the ballot, and be answered under the four fail-closed rules.**
///
/// 1. **An uncalibrated run writes nothing.** An instrument whose calibration does not beat the majority
///    class of its own known-answer set is refused by name ([`RefusalKind::NotEstablished`]), with both
///    figures in the refusal so the size of the loss is visible.
/// 2. **A missing or unreachable instrument is refused** ([`RefusalKind::OracleUnreachable`]) — never read
///    as a pass.
/// 3. **No reading at all is refused**: a shape the question does not apply to, or a body with nothing in it
///    to read, comes back as [`Verdict::NoReading`], and [`Answer::passed`] refuses to read a floor off it.
/// 4. **A `not_belongs` reading is refused unless a human's reason is recorded in the same act**
///    ([`RefusalKind::ReasonAbsent`]), and the reason travels with the finding when it is.
///
/// **The order the four are checked in is deliberate**, because two of them can be reached by the same
/// instrument and each has to keep its own name: the bytes are read, the instrument is *asked*, and only then
/// is its calibration read. Asking first is what keeps *unreachable* ([`RefusalKind::OracleUnreachable`], the
/// oracle's own words) distinct from *uncalibrated* ([`RefusalKind::NotEstablished`]): an instrument that
/// cannot run reports no readings at all, so checking its figures first would answer every missing oracle with
/// the wrong rule's refusal — and a missing instrument is not the same fact as an uncalibrated one. A
/// *non-reading*, by contrast, is one of the three answered words rather than an instrument's refusal, so it
/// stays a verdict and is decided with the reading, after the calibration has been read.
pub fn answer(
    ballot: &Ballot,
    judge: &dyn StructuralJudge,
    root: &Path,
    reason: Option<&str>,
) -> Result<Answer, RefusalSet> {
    let path = root.join(ballot.artifact());
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            let mut refusals = RefusalSet::new();
            refusals.refuse(
                RefusalKind::SourceAbsent,
                format!(
                    "the artifact `{}` is not there to be read in place at `{}`: {error} — the ballot \
                     judges bytes where they live, so an artifact that is gone is refused rather than \
                     reconstructed from a copy",
                    ballot.artifact(),
                    path.display()
                ),
            );
            return Err(refusals);
        }
    };
    let digest = Digest::of(&bytes);

    // The instrument's own refusal is returned as it comes — it is not an answer this layer could file, even
    // if the calibration were quotable. `NoReading` is excluded here because it is a **verdict** in this
    // vocabulary, and a verdict is an answer.
    let reading = match judge.judge(ballot.question(), &bytes) {
        Ok(judgement) => Some(judgement),
        Err(refusal) if refusal.kind() == RefusalKind::NoReading => None,
        Err(refusal) => {
            let mut refusals = RefusalSet::new();
            refusals.push(refusal);
            return Err(refusals);
        }
    };

    let measurement = judge.measurement();
    if !measurement.quotable() {
        let mut refusals = RefusalSet::new();
        refusals.refuse(
            RefusalKind::NotEstablished,
            format!(
                "the instrument is not calibrated to a standard this layer can quote: {} — {} reading(s) \
                 against a majority class of {:.4}, so the answer is refused rather than handed out with a \
                 caveat (the build gate prints `model-oracle not-established`)",
                measurement.render(),
                measurement.readings,
                measurement.majority
            ),
        );
        return Err(refusals);
    }

    let (chosen, verdict, scores, features) = match reading {
        Some(judgement) => {
            let verdict = verdict_of(ballot.question(), &judgement);
            (Some(judgement.chosen), verdict, judgement.scores, judgement.features)
        }
        None => (None, Verdict::NoReading, Vec::new(), Vec::new()),
    };

    if verdict == Verdict::NotBelongs {
        let reason = reason.map(str::trim).filter(|reason| !reason.is_empty());
        if reason.is_none() {
            let mut refusals = RefusalSet::new();
            refusals.refuse(
                RefusalKind::ReasonAbsent,
                format!(
                    "the reading is `not_belongs` at `{}` and no human's reason is recorded in the same \
                     act: a negative reading is refused rather than filed bare, because the reason is the \
                     part of it a successor can act on",
                    ballot.artifact()
                ),
            );
            return Err(refusals);
        }
    }

    let reason = reason
        .map(str::trim)
        .filter(|reason| !reason.is_empty())
        .map(|reason| reason.to_string());
    Ok(Answer {
        chosen,
        verdict,
        // The origin is the proof's to carry: it is set here, from the seam, and no ballot field can.
        origin: Origin::Oracle,
        evidence: Evidence {
            artifact: ballot.artifact().to_string(),
            digest,
            features,
            scores,
            readings: measurement.readings,
            majority: measurement.majority,
            leave_one_out: measurement.leave_one_out,
            reason,
        },
    })
}

/// The floor's own comparison, per instrument — and a `Score` question has no answer from an instrument that
/// does not read rungs, which is a **shape mismatch** and therefore a non-reading rather than a verdict for
/// the other instrument.
fn verdict_of(question: &Question, judgement: &Judgement) -> Verdict {
    match question {
        Question::Choice { passes, .. } => {
            if &judgement.chosen == passes {
                Verdict::Belongs
            } else {
                Verdict::NotBelongs
            }
        }
        Question::Noul { threshold } => {
            let confidence = judgement
                .scores
                .iter()
                .find(|(option, _)| option == &judgement.chosen)
                .map(|(_, score)| *score)
                .unwrap_or(0.0);
            if confidence >= *threshold {
                Verdict::Belongs
            } else {
                Verdict::NotBelongs
            }
        }
        Question::Score { .. } => Verdict::NoReading,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axis::{AxisKind, Ladder};

    fn a_ballot(question: Question, artifact: &str) -> Ballot {
        Ballot::declare(
            "u7-life",
            "the plan's §5 row 6",
            artifact,
            "read the record's own body and say which stage of its life it is at",
            question,
            "the clean room's build",
            24,
        )
        .expect("the fixture ballot is complete")
    }

    fn the_stage_question() -> Question {
        Question::choice(
            &["proposed", "designed", "verified-internal", "applied-internal", "approved"],
            "designed",
        )
        .expect("five stages, one passing")
    }

    #[test]
    fn the_verdict_vocabulary_is_closed_and_no_reading_is_not_a_verdict() {
        assert_eq!(Verdict::ALL.len(), 3);
        for word in ["belongs", "not_belongs", "no_reading"] {
            assert_eq!(Verdict::parse(word).map(Verdict::name), Some(word));
        }
        assert_eq!(Verdict::parse("maybe"), None);
        assert!(Verdict::Belongs.is_a_verdict());
        assert!(Verdict::NotBelongs.is_a_verdict());
        assert!(!Verdict::NoReading.is_a_verdict());
    }

    #[test]
    fn a_choice_over_fewer_than_two_options_is_no_question() {
        let refused = Question::choice(&["only"], "only").expect_err("one option chooses nothing");
        assert!(refused.contains(RefusalKind::BallotIncomplete), "{refused}");
        let refused = Question::choice(&["a", "a"], "a").expect_err("one option twice is one option");
        assert!(refused.contains(RefusalKind::BallotIncomplete), "{refused}");
    }

    #[test]
    fn a_floor_outside_the_questions_own_terms_is_refused() {
        let refused = the_stage_question_of("shipped").expect_err("a passing option must be declared");
        assert!(refused.contains(RefusalKind::FloorUndeclared), "{refused}");
        assert!(
            refused.all().iter().any(|refusal| refusal.detail().contains("shipped")),
            "{refused}"
        );
    }

    fn the_stage_question_of(passes: &str) -> Result<Question, RefusalSet> {
        Question::choice(
            &["proposed", "designed", "verified-internal", "applied-internal", "approved"],
            passes,
        )
    }

    #[test]
    fn a_score_needs_two_rungs_and_a_reachable_floor() {
        let refused = Question::score(&["mostly"], "mostly").expect_err("one rung scores nothing");
        assert!(refused.contains(RefusalKind::BallotIncomplete), "{refused}");
        let refused = Question::score(&["none", "some"], "all").expect_err("that floor is unreachable");
        assert!(refused.contains(RefusalKind::FloorUndeclared), "{refused}");
        assert_eq!(Question::score(&["none", "some"], "some").unwrap().kind(), "score");
    }

    #[test]
    fn a_threshold_outside_the_interval_is_not_a_threshold() {
        assert!(Question::noul(0.5).is_ok());
        for wrong in [0.0, -0.5, 1.5] {
            let refused = Question::noul(wrong).expect_err("that is not a threshold");
            assert!(refused.contains(RefusalKind::FloorUndeclared), "{wrong}: {refused}");
        }
    }

    #[test]
    fn a_ballot_missing_a_field_or_a_window_is_refused_by_name() {
        for (field, id, claim, artifact, instruction, presenter) in [
            ("id", "", "c", "docs/x.md", "i", "p"),
            ("claim", "u", "", "docs/x.md", "i", "p"),
            ("artifact", "u", "c", "", "i", "p"),
            ("instruction", "u", "c", "docs/x.md", "", "p"),
            ("presenter", "u", "c", "docs/x.md", "i", ""),
        ] {
            let refused = Ballot::declare(id, claim, artifact, instruction, the_stage_question(), presenter, 24)
                .expect_err("an incomplete ballot is refused");
            assert!(refused.contains(RefusalKind::BallotIncomplete), "{field}: {refused}");
            assert!(
                refused.all().iter().any(|refusal| refusal.detail().contains(field)),
                "{field} must be the field named: {refused}"
            );
        }
        let refused = Ballot::declare("u", "c", "docs/x.md", "i", the_stage_question(), "p", 0)
            .expect_err("a max-age of zero is not a window");
        assert!(refused.contains(RefusalKind::BallotIncomplete), "{refused}");
        assert!(
            refused.all().iter().any(|refusal| refusal.detail().contains("not a window")),
            "{refused}"
        );
    }

    /// A judge that reads one declared feature and answers the option its fixture names — the smallest
    /// instrument the seam can be exercised through, and never the shipped one.
    struct Fixture {
        answer: String,
        confidence: f64,
        measurement: Measurement,
    }

    impl StructuralJudge for Fixture {
        fn judge(&self, question: &Question, artifact: &[u8]) -> Result<Judgement, Refusal> {
            if matches!(question, Question::Score { .. }) {
                return Err(Refusal::new(
                    RefusalKind::NoReading,
                    "a score question is not a question this instrument reads".to_string(),
                ));
            }
            if artifact.is_empty() {
                return Err(Refusal::new(
                    RefusalKind::NoReading,
                    "the artifact carries nothing to read".to_string(),
                ));
            }
            let scores = question
                .terms()
                .iter()
                .map(|term| {
                    let value = if term == &self.answer { self.confidence } else { 0.0 };
                    (term.clone(), value)
                })
                .collect();
            Ok(Judgement {
                chosen: self.answer.clone(),
                scores,
                features: vec!["DeclaresChange"],
            })
        }

        fn measurement(&self) -> &Measurement {
            &self.measurement
        }
    }

    fn quotable() -> Measurement {
        Measurement {
            readings: 16,
            per_option: vec![("proposed", 5), ("designed", 4), ("verified-internal", 4)],
            majority: 0.3125,
            in_sample: 0.75,
            leave_one_out: 0.5,
            leave_one_out_with_absence: 0.125,
            leave_one_out_from_the_claim_only: 0.0,
            known_answers: Digest::from_bytes([7u8; 32]),
        }
    }

    /// A scratch directory with one file in it, so the in-place read is exercised for real.
    fn scratch(name: &str, body: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("u7-laya-{}-{}", std::process::id(), name));
        std::fs::create_dir_all(&root).expect("a scratch directory");
        std::fs::write(root.join("artifact.md"), body).expect("the artifact");
        root
    }

    #[test]
    fn an_uncalibrated_instrument_writes_nothing() {
        let root = scratch("uncalibrated", "a body");
        let judge = Fixture {
            answer: "designed".to_string(),
            confidence: 0.9,
            measurement: Measurement {
                leave_one_out: 0.25,
                majority: 0.3125,
                ..quotable()
            },
        };
        let refused = answer(&a_ballot(the_stage_question(), "artifact.md"), &judge, &root, None)
            .expect_err("a calibration below its own majority class is not quotable");
        assert!(refused.contains(RefusalKind::NotEstablished), "{refused}");
        assert!(
            refused.all().iter().any(|refusal| refusal.detail().contains("0.3125")),
            "the refusal must carry both figures: {refused}"
        );
    }

    #[test]
    fn an_unreachable_instrument_is_refused_and_never_read_as_a_pass() {
        let root = scratch("unreachable", "a body");
        let judge = Unreachable { what: "a 757 MiB model" };
        let refused = answer(&a_ballot(the_stage_question(), "artifact.md"), &judge, &root, None)
            .expect_err("a missing instrument is not a pass");
        assert!(refused.contains(RefusalKind::OracleUnreachable), "{refused}");
        assert!(refused.all()[0].detail().contains("757 MiB"), "{refused}");
    }

    #[test]
    fn a_non_reading_is_never_a_pass() {
        let root = scratch("empty", "");
        let judge = Fixture {
            answer: "designed".to_string(),
            confidence: 0.9,
            measurement: quotable(),
        };
        let answered = answer(&a_ballot(the_stage_question(), "artifact.md"), &judge, &root, None)
            .expect("a reading of nothing is still an answer, and it says so");
        assert_eq!(answered.verdict(), Verdict::NoReading);
        assert!(!answered.verdict().is_a_verdict());
        let refused = answered.passed().expect_err("a non-reading has no floor");
        assert_eq!(refused.kind(), RefusalKind::NoReading);
    }

    #[test]
    fn a_shape_mismatch_is_a_non_reading_and_not_the_other_instruments_verdict() {
        let root = scratch("shape", "a body");
        let judge = Fixture {
            answer: "designed".to_string(),
            confidence: 0.9,
            measurement: quotable(),
        };
        let score = Question::score(&["none", "some", "all"], "some").expect("three rungs");
        let answered = answer(&a_ballot(score, "artifact.md"), &judge, &root, None)
            .expect("a score question is answered with a non-reading");
        assert_eq!(answered.verdict(), Verdict::NoReading);
    }

    #[test]
    fn a_negative_reading_needs_a_reason_recorded_in_the_same_act() {
        let root = scratch("negative", "a body");
        let judge = Fixture {
            answer: "proposed".to_string(),
            confidence: 0.9,
            measurement: quotable(),
        };
        let ballot = a_ballot(the_stage_question(), "artifact.md");
        let refused = answer(&ballot, &judge, &root, None)
            .expect_err("a not_belongs reading is refused bare");
        assert!(refused.contains(RefusalKind::ReasonAbsent), "{refused}");
        let answered = answer(&ballot, &judge, &root, Some("the record's body is thin and says so"))
            .expect("with a reason it is filed");
        assert_eq!(answered.verdict(), Verdict::NotBelongs);
        assert_eq!(
            answered.evidence().reason.as_deref(),
            Some("the record's body is thin and says so"),
            "the reason travels with the finding"
        );
        assert!(!answered.passed().expect("a verdict has a floor"));
    }

    #[test]
    fn a_landed_answer_carries_the_oracle_origin_and_its_evidence() {
        let root = scratch("belongs", "a body");
        let judge = Fixture {
            answer: "designed".to_string(),
            confidence: 0.9,
            measurement: quotable(),
        };
        let answered = answer(&a_ballot(the_stage_question(), "artifact.md"), &judge, &root, None)
            .expect("a calibrated instrument answers");
        assert_eq!(answered.verdict(), Verdict::Belongs);
        assert_eq!(answered.origin(), Origin::Oracle);
        assert_eq!(answered.origin().token(), "A");
        assert_eq!(
            <Origin as crate::axis::Ladder>::AXIS,
            AxisKind::TicketEvidence,
            "the origin is the ticket-evidence ladder and nothing else"
        );
        assert!(answered.passed().expect("a verdict has a floor"));
        assert_eq!(answered.evidence().scores.len(), 5, "every option's score travels");
        assert_eq!(answered.evidence().readings, 16);
        assert_eq!(answered.evidence().digest, Digest::of(b"a body"));
    }

    #[test]
    fn an_absent_artifact_is_refused_by_name_with_its_path() {
        let root = scratch("absent", "a body");
        let judge = Fixture {
            answer: "designed".to_string(),
            confidence: 0.9,
            measurement: quotable(),
        };
        let ballot = a_ballot(the_stage_question(), "gone.md");
        let refused = answer(&ballot, &judge, &root, None).expect_err("nothing to read in place");
        assert!(refused.contains(RefusalKind::SourceAbsent), "{refused}");
        assert!(refused.all()[0].detail().contains("gone.md"), "{refused}");
    }
}
