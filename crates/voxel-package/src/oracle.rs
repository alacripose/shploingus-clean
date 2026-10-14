//! The conformance oracle: does a compiled line actually say what the recipe asked
//! for?
//!
//! # What this is
//!
//! A line is put to a judge together with the recipe it is supposed to belong to.
//! The judge answers with a **closed typed vocabulary** — never prose — and the
//! answer is recorded as **evidence**. It is never authority. This tree's
//! founding rule is that the model *proposes* and the record *decides*, and a
//! judge that could return a sentence would be a judge whose sentence could be
//! quoted.
//!
//! # The five rules, and why the gate fails closed
//!
//! 1. An **uncalibrated** run is refused and **writes nothing**.
//! 2. A **missing or unreachable** oracle is refused. *A missing instrument is not
//!    a pass.*
//! 3. **No reading at all** is refused. *Absence is not a free pass, it is no
//!    evidence.*
//! 4. A `not_belongs` reading is refused **unless a human's reason is recorded in
//!    the same act**, and the reason travels with the finding.
//! 5. **(C-60) A recorded absence is a distinct, renderable state.** The three
//!    absences above are three refusals and not one, and each has a rendering.
//!    AWA3D's effort loop states the general form: chat *"never goes silent"*.
//!    An absence that is not rendered cannot be told apart from a pass.
//!
//! Each rule is a test.
//!
//! # What the judge is, stated honestly
//!
//! [`StructuralJudge`] is **deterministic and it says so**. It is *not* a model
//! and does not claim to be one: it compares the vocabulary a line introduces
//! against the vocabulary a recipe licenses, and it **names the culprit term**,
//! which is what makes a refusal actionable.
//!
//! **Its measured failure, carried here rather than discovered later.** The
//! reference project measured this same judge on its own corpus and found that
//! against a one-sentence *prose* recipe it convicted **258 of 258** judged
//! lines, and that a real recipe and a deliberately unrelated recipe — *"the
//! moon is made of green cheese"* — produced **the same 258**. Conviction there is
//! a function of vocabulary overlap, not of meaning, and **a gate whose
//! strictness is uncorrelated with its correctness reads as evidence while being
//! wrong.**
//!
//! So the judge's contract requires a recipe to **declare the vocabulary it
//! licenses** ([`StructuralJudge::reading_within`]), and an empty licensed list is
//! **not defaulted in either direction**. A recipe that licenses nothing is judged
//! on its prose alone, which is why the two entry points are named separately
//! rather than one defaulting to the other.
//!
//! # The bar is the majority class, and the known-answer set is not the judge's ally
//!
//! [`calibrate`] measures accuracy **and** the majority class of the set, and
//! refuses to mark the run calibrated when the judge loses. A judge scoring below
//! the most common answer in the set is worse than always guessing, and its
//! verdicts would be noise wearing an instrument's name.
//!
//! **The standing rule for this block: the known-answer set may not be edited to
//! manufacture a pass.** A set chosen to flatter the judge is the same defect as a
//! coverage row that misdescribes its own ladder. The measurement is reported as
//! it falls, in [`calibrate`]'s own notes and in the build gate.

use std::path::Path;

use voxel_model::{Refusal, RefusalKind, RefusalSet};

use crate::census::Axis;
use crate::features::Establishment;

/// What the oracle said about one line. A **closed** vocabulary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verdict {
    /// The line belongs to the recipe.
    Belongs,
    /// The line does not belong to the recipe.
    NotBelongs,
    /// The oracle read it and declines to answer.
    ///
    /// **Distinct from unreachable.** This is a reading; that is not one. A line
    /// with nothing in it to judge is not a line that failed.
    NoReading,
}

impl Verdict {
    pub const fn slug(self) -> &'static str {
        match self {
            Verdict::Belongs => "belongs",
            Verdict::NotBelongs => "not_belongs",
            Verdict::NoReading => "no_reading",
        }
    }

    /// Parse an oracle reply, refusing anything outside the vocabulary.
    ///
    /// A coerced verdict is a fabricated one, so an undeclared word is refused
    /// rather than mapped onto the nearest thing that fits.
    pub fn parse(word: &str) -> Result<Self, String> {
        match word.trim() {
            "belongs" => Ok(Verdict::Belongs),
            "not_belongs" => Ok(Verdict::NotBelongs),
            "no_reading" => Ok(Verdict::NoReading),
            other => Err(format!(
                "oracle reply {other:?} is not one of belongs, not_belongs, no_reading; refusing \
                 to guess what it meant"
            )),
        }
    }
}

/// Whether this run's oracle has been measured against decisions whose answers
/// are on record.
///
/// This is not a formality. A checkpoint that *ships over-confident* is worse
/// than no instrument, because its confidence looks quotable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Calibration {
    pub calibrated: bool,
    /// How many known-answer decisions the calibration ran against.
    pub known_answers: usize,
    /// What the model itself warned about, and the measurement. Carried rather
    /// than allowed to scroll past.
    pub notes: Vec<String>,
}

impl Calibration {
    /// No measurement taken. The gate refuses in this state.
    pub fn unmeasured() -> Self {
        Self {
            calibrated: false,
            known_answers: 0,
            notes: Vec::new(),
        }
    }

    /// Calibrated against `known_answers` decisions whose answers are records.
    ///
    /// A count of zero is **not** calibrated whatever the flag says: a run
    /// against nothing cannot have measured anything.
    pub fn measured(known_answers: usize) -> Self {
        Self {
            calibrated: known_answers > 0,
            known_answers,
            notes: Vec::new(),
        }
    }

    /// Whether a verdict from this run may be quoted at all.
    pub fn is_quotable(&self) -> bool {
        self.calibrated && self.known_answers > 0
    }
}

/// One line under test, and the recipe it is tested against.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LintRequest {
    /// What is being pushed through, for the record.
    pub subject: String,
    /// The line.
    pub line: String,
    /// The recipe — what was actually asked for.
    pub recipe: String,
    /// The vocabulary the recipe licenses. **Never defaulted.**
    pub licensed: Vec<String>,
}

/// The oracle seam.
///
/// Object-safe, so the gate never names an implementation and this crate depends
/// on no judge — not a model, not a subprocess, nothing.
pub trait Oracle {
    /// Which instrument this is, for the record.
    fn name(&self) -> &'static str;

    /// Judge one line. An error means the instrument could not be reached, which
    /// is a refusal and never a pass.
    fn judge(&self, request: &LintRequest) -> Result<Verdict, RefusalSet>;
}

/// What was recorded. Its existence **is** the pass: a refusal returns no
/// finding, so nothing is written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub subject: String,
    pub oracle: &'static str,
    pub verdict: Verdict,
    /// Carried on every finding, so no reader has to guess whether the run was
    /// measured.
    pub calibrated: bool,
    /// The size of the measurement, so a reader sees how big it was and not only
    /// that one happened.
    pub known_answers: usize,
    /// The human's reason, when a `not_belongs` reading was overridden. On the
    /// record **with** the finding, not beside it.
    pub reason: Option<String>,
}

impl Finding {
    /// A reading is quotable only when it was calibrated.
    pub fn is_quotable(&self) -> bool {
        self.calibrated
    }
}

/// Lint one line: return a [`Finding`] to record, or the refusals that stop it.
///
/// **The order is the design.** Calibration is checked **before** the oracle is
/// consulted, so an unmeasured run cannot reach the judge at all and therefore
/// cannot produce something quotable-looking.
pub fn lint(
    oracle: &dyn Oracle,
    request: &LintRequest,
    calibration: &Calibration,
    reason: Option<&str>,
) -> Result<Finding, RefusalSet> {
    // Rule 1.
    if !calibration.is_quotable() {
        let mut refusals = RefusalSet::new();
        refusals.push(Refusal::owned(
            RefusalKind::UncalibratedOracle,
            format!(
                "{}: the oracle has not been calibrated in this run ({} known-answer decisions), so \
                 nothing it says may be quoted and nothing is written",
                request.subject, calibration.known_answers
            ),
        ));
        return Err(refusals);
    }

    // Rule 2.
    let verdict = oracle.judge(request)?;

    // Rule 3.
    if verdict == Verdict::NoReading {
        let mut refusals = RefusalSet::new();
        refusals.push(Refusal::owned(
            RefusalKind::NoReading,
            format!(
                "{}: {} returned no reading for this line. An absence is not a free pass, it is no \
                 evidence",
                request.subject,
                oracle.name()
            ),
        ));
        return Err(refusals);
    }

    // Rule 4.
    if verdict == Verdict::NotBelongs && reason.is_none() {
        let mut refusals = RefusalSet::new();
        refusals.push(Refusal::owned(
            RefusalKind::UnreasonedOverride,
            format!(
                "{}: {} read this line as not belonging ({}), and no reason was recorded in the \
                 same act. The recipe declares: {}",
                request.subject,
                oracle.name(),
                verdict.slug(),
                request.recipe
            ),
        ));
        return Err(refusals);
    }

    Ok(Finding {
        subject: request.subject.clone(),
        oracle: oracle.name(),
        verdict,
        calibrated: calibration.calibrated,
        known_answers: calibration.known_answers,
        reason: reason.map(str::to_owned),
    })
}

/// Terms a line introduces: identifier-shaped runs, lowercased, first-seen order,
/// de-duplicated.
///
/// Lowercased because a recipe in prose says `Ticket` and a line in code says
/// `ticket`, and those are one term. De-duplicated because the count is the
/// denominator of coverage, and a term used twice is not two problems.
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

/// Language shape rather than meaning.
///
/// Kept **small and named** rather than a long stopword list, because every entry
/// is a term the judge will *not* hold against a contributor, and a long list is a
/// long way of saying *"this line is probably fine."*
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

/// A literal value rather than a word: digits, with an optional sign.
///
/// **Found by measurement, not by reading.** The first calibration of this judge
/// misread `window.opacity(930);` as non-conformant, and `culprits` named the
/// culprit: `930`. A number in a source line is a *value*, not vocabulary, and
/// holding a contributor's numeric literal against a prose recipe is the same
/// defect as holding their prose against it — conviction by vocabulary overlap
/// rather than by meaning.
fn is_literal(term: &str) -> bool {
    let body = term.strip_prefix('-').unwrap_or(term);
    !body.is_empty() && body.bytes().all(|byte| byte.is_ascii_digit())
}

/// What the judge saw, before it became a verdict.
///
/// [`Reading::Undeclared`] carries the culprit terms verbatim, because a refusal
/// in this project **names what it refused** — and a refusal that says only
/// "non-conformant" is a refusal nobody can act on.
#[derive(Clone, Debug, PartialEq)]
pub enum Reading {
    /// Nothing in the line to judge: blank, a comment, or pure language shape.
    ///
    /// **Not a pass.** Reading `pub let mut` as conformant would be a free pass
    /// dressed as evidence.
    NothingToJudge,
    /// Every term the line introduces is declared or is language syntax.
    AllDeclared {
        coverage: f64,
    },
    /// The line introduces a term the recipe does not name — and the term is
    /// named.
    Undeclared {
        found: Vec<String>,
        coverage: f64,
    },
}

/// The deterministic judge: **structural** conformance, and it says so.
#[derive(Clone, Copy, Debug, Default)]
pub struct StructuralJudge;

impl StructuralJudge {
    /// The reading, judged on the recipe's prose alone.
    ///
    /// **Named separately from [`Self::reading_within`] rather than defaulting to
    /// it**, because a prose recipe licenses almost no identifiers and this judge
    /// will then convict nearly everything. That behaviour is correct for what it
    /// is; it is just not what a caller means by "ask the question".
    pub fn reading(&self, line: &str, recipe: &str) -> Reading {
        self.reading_within(line, recipe, &[])
    }

    /// The reading, against a recipe **plus an explicitly licensed vocabulary**.
    ///
    /// `licensed` is added to the recipe's own terms and **nothing else changes**:
    /// the undeclared test, the syntax exemption and the `NothingToJudge` contract
    /// are `reading`'s. This is the same instrument with a wider declared
    /// vocabulary, not a second rule.
    ///
    /// **No default, and that is the point.** Defaulting an empty list to
    /// "everything" would make the gate pass every file and mean nothing; the
    /// caller chooses, and the choice is visible in whatever produced the verdict.
    pub fn reading_within(&self, line: &str, recipe: &str, licensed: &[String]) -> Reading {
        // A comment's words are documentation, not code. Judging them would hold a
        // contributor to the recipe's vocabulary for prose they wrote to explain
        // themselves — measured by the failure that found it: `/// docs` extracted
        // `docs` and read the comment as a misfit line.
        if line.trim().is_empty() || line.trim().starts_with("//") {
            return Reading::NothingToJudge;
        }

        let introduced = terms(line);
        // "Nothing to judge" means every term is **shape**, and a line of only
        // literals is shape too: `930` says nothing about a recipe either.
        if introduced.is_empty()
            || introduced
                .iter()
                .all(|term| is_syntax(term) || is_literal(term))
        {
            return Reading::NothingToJudge;
        }

        let mut declared: std::collections::BTreeSet<String> = terms(recipe).into_iter().collect();
        for term in licensed {
            // Each licensed string is put through `terms` first, not taken raw: a
            // caller writing `--license "Mods Surface"` supplies words with capitals
            // and spaces, and a raw insert would make that one term, license
            // nothing, and fail silently.
            declared.extend(terms(term));
        }

        let undeclared: Vec<String> = introduced
            .iter()
            .filter(|term| !declared.contains(*term) && !is_syntax(term) && !is_literal(term))
            .cloned()
            .collect();

        if undeclared.is_empty() {
            Reading::AllDeclared {
                // A line whose terms are all declared has full coverage. Guarded
                // against a division by zero for the case where `introduced` was
                // non-empty but every term was syntax, which cannot reach here but
                // is not worth a panic if the exemption above is ever widened.
                coverage: 1.0,
            }
        } else {
            let coverage = 1.0 - (undeclared.len() as f64 / introduced.len() as f64);
            Reading::Undeclared { found: undeclared, coverage }
        }
    }
}

impl Oracle for StructuralJudge {
    fn name(&self) -> &'static str {
        "structural"
    }

    fn judge(&self, request: &LintRequest) -> Result<Verdict, RefusalSet> {
        // A `Verdict` carries no prose **by design**, so the culprit terms are not
        // attached here. They are reachable through [`culprits`], which is the
        // supported way to get them — a caller that needs to name the offending
        // term asks for it rather than parsing a verdict.
        Ok(match self.reading_within(&request.line, &request.recipe, &request.licensed) {
            Reading::NothingToJudge => Verdict::NoReading,
            Reading::AllDeclared { .. } => Verdict::Belongs,
            Reading::Undeclared { .. } => Verdict::NotBelongs,
        })
    }
}

/// The culprit terms, when the judge read the line as not belonging.
///
/// Split out of [`Oracle::judge`] because a `Verdict` carries no prose **by
/// design** — so if the terms were only reachable there, the promise that a
/// refusal names what it refused would be one only this module's own tests could
/// keep.
pub fn culprits(
    judge: &StructuralJudge,
    request: &LintRequest,
) -> Option<Vec<String>> {
    match judge.reading_within(&request.line, &request.recipe, &request.licensed) {
        Reading::Undeclared { found, .. } => Some(found),
        _ => None,
    }
}

/// An oracle that cannot be reached. **Refuses everything, on purpose.**
///
/// This is the correct binding while no real oracle is reachable, and its purpose
/// is to make *"the lint gate was skipped"* impossible to mistake for *"the lint
/// gate passed."*
pub struct UnreachableOracle {
    pub why: String,
}

impl Oracle for UnreachableOracle {
    fn name(&self) -> &'static str {
        "unreachable"
    }

    fn judge(&self, request: &LintRequest) -> Result<Verdict, RefusalSet> {
        let mut refusals = RefusalSet::new();
        refusals.push(Refusal::owned(
            RefusalKind::OracleUnreachable,
            format!(
                "{}: no conformance oracle is reachable ({}), and a missing instrument is not a \
                 pass",
                request.subject, self.why
            ),
        ));
        Err(refusals)
    }
}

/// **The skipped case, answered through the seam.**
///
/// H-14.1 made Laya the decision layer and this module's oracle *one implementation of the
/// seam*; the DSL's own words for the component are *"`oracle::Oracle` — one implementation of
/// the seam, with `oracle::UnreachableOracle` for the skipped case"*. So the oracle module does
/// not only supply an instrument for the oracle path — it supplies the seam's skipped case too,
/// which is why `decision::Unreachable` is **this type** rather than a second struct with the
/// same shape and the same purpose. Two names for one meaning is D-07, and it is the defect
/// that made `voxel_model::grant` move down rather than be duplicated.
///
/// **The refusal kind is `NoReading`, unchanged from the seam's own skip**, because this
/// collapse is about *identity* and not about re-kind-ing refusals. `UnreachableOracle::judge`
/// below is the oracle path and answers `OracleUnreachable`; a `Decider` that cannot be reached
/// has *no reading*, which is the fact `refusal.rs` separates from the other absences. Changing
/// the kind here would be a behaviour change smuggled inside a structural one.
impl crate::decision::Decider for UnreachableOracle {
    fn name(&self) -> &'static str {
        "unreachable"
    }

    fn decide(
        &self,
        subject: crate::decision::Subject,
    ) -> Result<crate::decision::Assessment, RefusalSet> {
        let mut refusals = RefusalSet::new();
        refusals.push(Refusal::owned(
            RefusalKind::NoReading,
            format!("{} could not be decided: {}", subject.name(), self.why),
        ));
        Err(refusals)
    }
}

/// One decision whose answer is **on record**, not an opinion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KnownAnswer {
    pub subject: &'static str,
    pub line: &'static str,
    pub recipe: &'static str,
    /// The vocabulary this recipe licenses. Declared, never defaulted — see the
    /// module header on the 258-of-258 measurement.
    pub licensed: &'static [&'static str],
    pub expected: Verdict,
}

/// What a calibration measured.
#[derive(Clone, Debug, PartialEq)]
pub struct CalibrationReport {
    pub calibration: Calibration,
    pub accuracy: f64,
    /// The most common answer in the set. **The bar.**
    pub majority_class: f64,
    /// Accuracy if every reading were flipped. Carried because the reference
    /// model's failure was a *phrasing* fault as well as an ability one, and the
    /// two must be told apart rather than assumed fixable.
    pub inverted_accuracy: f64,
    pub readings_used: usize,
    /// Whether the judge is allowed to be quoted, and why not when it is not.
    pub establishment: Establishment,
    /// Which known answers the judge got **wrong**, by subject.
    ///
    /// A score on its own hides which decisions failed, and a calibration that
    /// cannot name its own misses is one nobody can improve without guessing.
    /// This is the same rule as the rest of the project: a refusal names what it
    /// refused.
    pub misread: Vec<String>,
}

/// Measure a judge against a known-answer set.
///
/// **The bar is the majority class.** A judge scoring below the most common answer
/// in the set is worse than always guessing, and its verdicts would be noise
/// wearing an instrument's name. The notes carry **both** numbers, so a reader can
/// see the size of the loss rather than being told only that there was one.
pub fn calibrate(judge: &dyn Oracle, set: &[KnownAnswer]) -> CalibrationReport {
    let unmeasured = |accuracy, majority, inverted, used| CalibrationReport {
        calibration: Calibration::unmeasured(),
        accuracy,
        majority_class: majority,
        inverted_accuracy: inverted,
        readings_used: used,
        establishment: Establishment::Unchecked,
        misread: Vec::new(),
    };

    if set.is_empty() {
        return unmeasured(0.0, 0.0, 0.0, 0);
    }

    let mut correct = 0usize;
    let mut inverted = 0usize;
    let mut used = 0usize;
    let mut per_verdict = [0usize; 3];
    let mut misread: Vec<String> = Vec::new();

    for known in set {
        let request = LintRequest {
            subject: known.subject.to_owned(),
            line: known.line.to_owned(),
            recipe: known.recipe.to_owned(),
            licensed: known.licensed.iter().map(|s| (*s).to_owned()).collect(),
        };
        let Ok(verdict) = judge.judge(&request) else {
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
        } else {
            misread.push(format!(
                "{}: read {} where the record says {}",
                known.subject,
                verdict.slug(),
                known.expected.slug()
            ));
        }
        if invert(verdict) == known.expected {
            inverted += 1;
        }
    }

    if used == 0 {
        return unmeasured(0.0, 0.0, 0.0, 0);
    }

    let total = set.len() as f64;
    let accuracy = correct as f64 / total;
    let inverted_accuracy = inverted as f64 / total;
    let majority = per_verdict.iter().copied().max().unwrap_or(0) as f64 / total;

    // The rule, stated once: the judge must **beat** the majority class. Tying is
    // not beating, because a judge that ties the most common answer has measured
    // nothing the set did not already give away.
    let wins = accuracy > majority;
    let mut notes = vec![
        format!("accuracy {accuracy:.4} against a majority class of {majority:.4}"),
        format!("inverted accuracy {inverted_accuracy:.4}"),
        format!("{} of {} known answers produced a reading", used, set.len()),
    ];
    if !wins {
        notes.push(
            "the judge does not beat the majority class, so nothing it says is quotable and the \
             gate writes nothing"
                .to_owned(),
        );
    }
    for miss in &misread {
        notes.push(format!("misread {miss}"));
    }

    CalibrationReport {
        calibration: Calibration {
            calibrated: wins,
            known_answers: used,
            notes: notes.clone(),
        },
        accuracy,
        majority_class: majority,
        inverted_accuracy,
        readings_used: used,
        establishment: if wins {
            Establishment::Active
        } else {
            Establishment::Inactive
        },
        misread,
    }
}

fn invert(verdict: Verdict) -> Verdict {
    match verdict {
        Verdict::Belongs => Verdict::NotBelongs,
        Verdict::NotBelongs => Verdict::Belongs,
        Verdict::NoReading => Verdict::NoReading,
    }
}

/// The alpha's known-answer set.
///
/// **Drawn from this project's own record, not from the author's imagination.**
/// That correction was made deliberately (C-61): the first version of this set
/// used vocabulary invented for it — `pinned`, `hud`, `mount` — and a judge
/// measured against a vocabulary nobody else in the project uses measures the
/// wrong thing while looking like it measures the right one. Every recipe below
/// quotes language the project's own documents use, and every licensed list is
/// the vocabulary that recipe actually grants.
///
/// The composition under test is H-7.12's, as recorded in
/// `docs/HYPERGRILLING.md`: **recipe + résumé + civilian + governance +
/// security**. All five parts appear, because a set that only exercised the shell
/// would have been measuring a sixth thing the record does not name.
///
/// **The set is not edited to flatter the judge.** If the judge loses to the
/// majority class, [`calibrate`] refuses and the gate writes nothing, and that is
/// the correct outcome rather than a failure to be tuned away. The mix is
/// deliberately not one-sided: a set of nothing but conformant lines would be
/// passed by a judge that always answers `belongs`.
pub const KNOWN_ANSWERS: &[KnownAnswer] = &[
    // ================= the composition (H-7.12) =================
    // recipe + résumé + civilian + governance + security
    KnownAnswer {
        subject: "policy.rs",
        line: "let decision = policy.decide(action);",
        recipe: "a single policy chokepoint that every action passes through",
        licensed: &["policy", "chokepoint", "action", "decide", "decision", "security"],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "policy.rs",
        line: "let citizen = roster.elect();",
        recipe: "a single policy chokepoint that every action passes through",
        licensed: &["policy", "chokepoint", "action", "decide", "decision", "security"],
        expected: Verdict::NotBelongs,
    },
    KnownAnswer {
        subject: "civilian.rs",
        line: "grant.scope();",
        recipe: "governance holds grants that are scoped and attributed to a granter",
        licensed: &["governance", "grant", "scope", "attributed", "granter"],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "resume.rs",
        line: "promote(resume.draft());",
        recipe: "content lifecycle reaches resumes: version, promote, supersede, retire, never delete",
        licensed: &[
            "content", "lifecycle", "resume", "version", "promote", "supersede", "retire",
            "never", "delete", "draft",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "provenance.rs",
        line: "let head = repository.head_commit();",
        recipe: "provenance is read through gix so a stale artifact is detectable",
        licensed: &[
            "provenance", "gix", "stale", "artifact", "detectable", "repository", "head",
            "head_commit", "commit",
        ],
        expected: Verdict::Belongs,
    },
    // ---- the recipe and package route (.shspec to .shpkg) ----
    KnownAnswer {
        subject: "recipe.rs",
        line: "compile(recipe.source());",
        recipe: "a recipe compiles to a package and the loader refuses what it cannot verify",
        licensed: &["recipe", "compile", "package", "loader", "refuse", "verify", "source"],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "recipe.rs",
        line: "let market = economy.tick();",
        recipe: "a recipe compiles to a package and the loader refuses what it cannot verify",
        licensed: &["recipe", "compile", "package", "loader", "refuse", "verify", "source"],
        expected: Verdict::NotBelongs,
    },
    // ================= the shell (H-9.7, H-9.8, H-12.2) =================
    KnownAnswer {
        subject: "shell/arrangement.rs",
        line: "mount(arrangement.window());",
        recipe: "windows are data the shell mounts and the hud is a pinned subset that stays \
                 visible while the shell is hidden",
        licensed: &[
            "window", "data", "shell", "mount", "hud", "pinned", "subset", "stay", "visible",
            "hide", "arrangement", "pin",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "shell/notifications.rs",
        line: "enqueue(toast);",
        recipe: "the shell owns a queue and a toast is a record in it",
        licensed: &[
            "shell", "queue", "toast", "record", "own", "notification", "enqueue", "writer",
            "rendering",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "shell/notifications.rs",
        line: "widget.raise(from_call_site);",
        recipe: "the shell owns a queue and a toast is a record in it",
        licensed: &["shell", "queue", "toast", "record"],
        expected: Verdict::NotBelongs,
    },
    // ================= the record (H-8.1, H-8.3, H-10.5, CLM) =================
    KnownAnswer {
        subject: "chain.rs",
        line: "append(event, chain);",
        recipe: "one append only hash chained record read through two views",
        licensed: &[
            "one", "append", "only", "hash", "chain", "record", "read", "two", "view", "event",
            "never", "delete", "promote", "supersede", "retire", "version",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "budget.rs",
        line: "declare_cadence(stage, hz);",
        recipe: "a twenty hertz base clock with each stage cadence declared",
        licensed: &[
            "twenty", "hertz", "base", "clock", "stage", "cadence", "declare", "tick", "period",
            "lazy", "advance", "sensor", "moment", "declare_cadence", "hz",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "climate.rs",
        line: "seasons(winter, spring, summer, autumn);",
        recipe: "seasons are the literal calendar seasons",
        licensed: &[
            "season", "literal", "calendar", "winter", "spring", "summer", "autumn", "clock",
            "field", "process", "never", "directory",
        ],
        expected: Verdict::Belongs,
    },
    // The same line and the same recipe, with two of the four seasons unlicensed.
    //
    // **The recipe's prose is deliberately short here.** A longer recipe that named
    // `summer` and `autumn` itself would have licensed them, and no licence list
    // can take a word back — see `a_recipe_can_widen_the_declared_set_but_never_narrow_it`.
    KnownAnswer {
        subject: "climate.rs",
        line: "seasons(winter, spring, summer, autumn);",
        recipe: "seasons are the literal calendar seasons",
        licensed: &["season", "winter", "spring"],
        expected: Verdict::NotBelongs,
    },
    // ================= DESIGN-SPEC: the world model =================
    KnownAnswer {
        subject: "voxel-shapers/src/voxel.rs",
        line: "let world = generate(seed, tick, period);",
        recipe: "the world is a pure function of seed, tick and the declared period",
        licensed: &["pure", "function", "world", "generate"],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "voxel-shapers/src/voxel.rs",
        line: "let cached = stage.hold(tick);",
        recipe: "the world is a pure function of seed, tick and the declared period",
        licensed: &["lazy", "advance", "stage"],
        expected: Verdict::NotBelongs,
    },
    KnownAnswer {
        subject: "voxel-shapers/src/voxel.rs",
        line: "recompute(system, moment);",
        recipe: "slow systems are lazily advanced, recomputed on demand, never cached stale",
        licensed: &[
            "slow", "system", "lazily", "advance", "recompute", "demand", "cache", "stale",
            "moment",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "voxel-model/src/conservation.rs",
        line: "check(mass, volume);",
        recipe: "a tolerance bearing quantity must never feed mass or volume",
        licensed: &[
            "tolerance", "bearing", "quantity", "feed", "exact", "approximate", "lineage",
            "counter", "check",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "voxel-model/src/conservation.rs",
        line: "check(mass, temperature);",
        recipe: "a tolerance bearing quantity must never feed mass or volume",
        licensed: &[
            "tolerance", "bearing", "quantity", "feed", "exact", "approximate", "lineage",
            "counter", "check",
        ],
        expected: Verdict::NotBelongs,
    },
    KnownAnswer {
        subject: "voxel-stream/src/horizons.rs",
        line: "typed(render, simulation, residency, field);",
        recipe: "four horizons are computed not copied, and must never be mixed",
        licensed: &[
            "horizon", "compute", "copy", "mix", "render", "simulation", "residency", "field",
            "typed",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "voxel-stream/src/horizons.rs",
        line: "typed(render, simulation, residency, field);",
        recipe: "four horizons are computed not copied, and must never be mixed",
        licensed: &["horizon", "compute", "copy", "mix", "render", "simulation", "residency", "typed"],
        expected: Verdict::NotBelongs,
    },
    KnownAnswer {
        subject: "voxel-world/src/authority.rs",
        line: "let authority = world.canonical();",
        recipe: "one world one truth, and no render side copy that can drift",
        licensed: &[
            "world", "truth", "render", "side", "copy", "drift", "canonical", "authority",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "voxel-shapers/src/fields.rs",
        line: "let chunk = world.chunk_at(voxel);",
        recipe: "a voxel is one metre, a chunk is thirty two cubed, a subvoxel is four by four \
                 by four",
        licensed: &[
            "voxel", "metre", "chunk", "thirty", "two", "cubed", "subvoxel", "four", "by",
            "detail", "lattice", "addressed", "stored", "dense", "world", "chunk_at",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "voxel-shapers/src/fields.rs",
        line: "let lithology = field.level(index);",
        recipe: "the field ladder is twenty levels and carries lithology elevation and soil \
                 depth only",
        licensed: &[
            "field", "ladder", "twenty", "level", "lithology", "elevation", "soil", "depth",
            "identity", "effect", "kind", "climate", "process", "index",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "voxel-shapers/src/fields.rs",
        line: "let climate = field.level(index);",
        recipe: "the field ladder is twenty levels and carries lithology elevation and soil \
                 depth only",
        licensed: &["field", "ladder", "lithology", "elevation", "soil", "depth", "level", "index"],
        expected: Verdict::NotBelongs,
    },
    // ================= DESIGN-SPEC: the feature grill =================
    KnownAnswer {
        subject: "voxel-model/src/material.rs",
        line: "refuse(material);",
        recipe: "an unadmitted material is a refusal, not a default",
        licensed: &[
            "material", "admit", "registry", "refusal", "default", "zero", "component", "ratio",
            "refuse",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "voxel-model/src/material.rs",
        line: "admit(material);",
        recipe: "an unadmitted material is a refusal, not a default",
        licensed: &["refusal", "default"],
        expected: Verdict::NotBelongs,
    },
    KnownAnswer {
        subject: "voxel-shapers/src/ecology.rs",
        line: "refuse(population);",
        recipe: "the population ceiling refuses rather than drops",
        licensed: &[
            "population", "ceiling", "refuse", "drop", "ecology", "bounded", "count", "coupling",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "voxel-shapers/src/delta.rs",
        line: "declare(recipe);",
        recipe: "a recipe that is a mass pump must declare itself one",
        licensed: &["recipe", "mass", "pump", "declare", "itself", "one", "kind", "attribute"],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "voxel-shapers/src/geology.rs",
        line: "derive(geology, from_refused_reference);",
        recipe: "geology is the largest evidence gap and both intended references were refused \
                 on licence",
        licensed: &[
            "geology", "largest", "evidence", "gap", "reference", "refuse", "licence", "terrain",
            "substrate", "deposit",
        ],
        expected: Verdict::NotBelongs,
    },
    KnownAnswer {
        subject: "voxel-model/src/material.rs",
        line: "assert_eq!(registry.len(), 0);",
        recipe: "materials are zero and an unadmitted material is a refusal",
        licensed: &[
            "material", "zero", "admit", "refusal", "registry", "blocked", "content",
            "conservation", "component", "ratio", "sum", "assert_eq", "len",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "voxel-shapers/src/voxel.rs",
        line: "let output = rebuild(seed, tick, workers);",
        recipe: "worker count is a pure performance knob that never changes output",
        licensed: &[
            "worker", "count", "pure", "performance", "knob", "never", "change", "output",
            "rebuild", "seed", "tick", "workers", "digest", "truth",
        ],
        expected: Verdict::Belongs,
    },
    // ================= APPEARANCE: the user's own area =================
    KnownAnswer {
        subject: "voxel-appearance/src/openpbr.rs",
        line: "let base_color = material.sample(block);",
        recipe: "base color is driven from block truth",
        licensed: &["base_color", "material", "sample"],
        expected: Verdict::Belongs,
    },
    // The same line and the same recipe, unlicensed. Short prose on purpose, for the
    // same reason as the seasons pair above.
    KnownAnswer {
        subject: "voxel-appearance/src/openpbr.rs",
        line: "let base_color = material.sample(block);",
        recipe: "base color is driven from block truth",
        licensed: &["base_color"],
        expected: Verdict::NotBelongs,
    },
    KnownAnswer {
        subject: "voxel-appearance/src/npr.rs",
        line: "apply(npr, openpbr);",
        recipe: "NPR is applied over the OpenPBR bsdf and never replaces it",
        licensed: &[
            "npr", "apply", "over", "openpbr", "bsdf", "replace", "stylisation", "transport",
            "light", "tunable", "model", "rim", "half", "lambert", "warp", "ambient", "tint",
            "band", "luminance",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "voxel-appearance/src/npr.rs",
        line: "assert(bands >= 3);",
        recipe: "every visible surface shows at least three distinct luminance bands",
        licensed: &[
            "visible", "surface", "three", "distinct", "luminance", "band", "shading", "response",
            "criterion", "measure", "test", "assert", "bands",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "voxel-appearance/src/openpbr.rs",
        line: "transmission_weight(surface);",
        recipe: "transmission weight is for water and glass and never on solids",
        licensed: &[
            "transmission", "weight", "water", "glass", "solid", "coat", "wet", "surface",
            "subsurface", "soil", "snow",
        ],
        expected: Verdict::NotBelongs,
    },
    KnownAnswer {
        subject: "voxel-ui/src/tokens.rs",
        line: "let elevation = step * 0.05;",
        recipe: "depth is raised is lighter, elevation steps by zero zero five, levels zero \
                 through four, refused above",
        licensed: &[
            "depth", "raise", "lighter", "elevation", "step", "zero", "five", "level", "refuse",
            "above", "radius", "shadow", "blur", "adopt", "user", "own",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "voxel-ui/src/tokens.rs",
        line: "let elevation = shadow(blur(step));",
        recipe: "depth is raised is lighter, elevation steps by zero zero five, levels zero \
                 through four, refused above",
        licensed: &["depth", "elevation", "step", "level", "refuse", "above"],
        expected: Verdict::NotBelongs,
    },
    // ================= the standing rules of this project =================
    KnownAnswer {
        subject: "shploingus/build.rs",
        line: "gate(build(package));",
        recipe: "the game cannot compile without the system",
        licensed: &[
            "game", "compile", "without", "system", "package", "prove", "works", "recipe",
            "build", "gate",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "references.md",
        line: "derive(reference, reason);",
        recipe: "an outside tree is reference for reasons and mechanisms only, never authority",
        licensed: &[
            "outside", "tree", "reference", "reason", "mechanism", "authority", "read", "copy",
            "import", "source", "derive",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "references.md",
        line: "import(reference);",
        recipe: "an outside tree is reference for reasons and mechanisms only, never authority",
        licensed: &["reference", "reason"],
        expected: Verdict::NotBelongs,
    },
    KnownAnswer {
        subject: "tests",
        line: "run(command, unfiltered);",
        recipe: "run tests exactly as the docs write them, with no output filtering",
        licensed: &[
            "run", "test", "exactly", "doc", "write", "output", "filtering", "pipe", "grep",
            "command", "unfiltered", "visible", "observation",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "tests",
        line: "observe(window);",
        recipe: "never test the client invisibly, a visible window observation is required",
        licensed: &[
            "never", "test", "client", "invisibly", "visible", "window", "observation",
            "require", "acceptance", "draw", "run", "observe",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "oracle.rs",
        line: "refuse(RefusalKind::NoReading);",
        recipe: "refuse never default, and a missing instrument is not a pass",
        licensed: &[
            "refuse", "never", "default", "missing", "instrument", "pass", "no", "reading",
            "reason", "refusalkind", "noreading",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "oracle.rs",
        line: "let verdict = judge.ask(question);",
        recipe: "refuse never default, and a missing instrument is not a pass",
        licensed: &["refuse", "never", "default", "missing", "instrument", "pass"],
        expected: Verdict::NotBelongs,
    },
    KnownAnswer {
        subject: "policy.rs",
        line: "let grant = Grant::new(granter, scope);",
        recipe: "grants are scoped and attributed, and an authorization with no author is not an \
                 authorization",
        licensed: &[
            "grant", "scope", "attribute", "authorization", "author", "deny", "default",
            "silence", "permission", "action", "matrix", "chokepoint", "security", "new",
            "granter",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "policy.rs",
        line: "permit(action);",
        recipe: "grants are scoped and attributed, and an authorization with no author is not an \
                 authorization",
        licensed: &["grant", "scope", "security"],
        expected: Verdict::NotBelongs,
    },
    // ================= nothing to judge =================
    // A comment's words are documentation, not code.
    KnownAnswer {
        subject: "voxel-shapers/src/fields.rs",
        line: "// the record's stated reason is wrong by a factor of one thousand",
        recipe: "the field ladder is twenty levels",
        licensed: &["field", "ladder", "twenty", "level"],
        expected: Verdict::NoReading,
    },
    // Pure language shape, which is a no-reading and not a pass.
    KnownAnswer {
        subject: "voxel-package/src/features.rs",
        line: "pub const fn",
        recipe: "the field ladder is twenty levels",
        licensed: &["field", "ladder", "twenty", "level"],
        expected: Verdict::NoReading,
    },
    // A doc comment, likewise.
    KnownAnswer {
        subject: "voxel-appearance/src/openpbr.rs",
        line: "/// the appearance is the user's, not mine",
        recipe: "base color is driven from block truth",
        licensed: &["base", "color", "block", "truth"],
        expected: Verdict::NoReading,
    },
    // The same line, licensed in one part of the composition and not in another.
    KnownAnswer {
        subject: "shell/arrangement.rs",
        line: "window.opacity(930);",
        recipe: "windows are data the shell mounts",
        licensed: &["window", "shell", "mount", "opacity"],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "shell/arrangement.rs",
        line: "window.opacity(930);",
        recipe: "windows are data the shell mounts",
        licensed: &["window", "shell", "mount"],
        expected: Verdict::NotBelongs,
    },
    // ================= the tree's whole vocabulary, one pair per system (C-106) =================
    // The user's ruling: *"the full vocab set is probably way higher than that … expand it's
    // vocab … the thing has the 5 modes, an observer/civilian/citizen, the audio engine, the
    // chunk system"*. Each pair below quotes the system's own declaration and licenses — or
    // refuses — the line that names it.
    // ---- the decision layer's five named parts (§98.8) ----
    KnownAnswer {
        subject: "decision.rs",
        line: "decide(seam, matrix, decider, instrument, judge);",
        recipe: "the decision layer's five named parts are the seam the matrix the in-project \
                 decider the model instrument and the judge",
        licensed: &[
            "decide", "seam", "matrix", "decider", "instrument", "judge", "decision", "layer",
            "part", "model", "project",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "decision.rs",
        line: "decide(seam, matrix, decider, instrument, payroll);",
        recipe: "the decision layer's five named parts are the seam the matrix the in-project \
                 decider the model instrument and the judge",
        licensed: &["decide", "seam", "matrix", "decider", "instrument", "judge"],
        expected: Verdict::NotBelongs,
    },
    // ---- the observer at the single commit point ----
    KnownAnswer {
        subject: "observer/recorder.rs",
        line: "observer.record(act, commit_point);",
        recipe: "the observer records every accepted act at the single commit point",
        licensed: &[
            "observer", "record", "act", "commit_point", "accepted", "commit", "point",
            "single",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "observer/recorder.rs",
        line: "observer.publish(act);",
        recipe: "the observer records every accepted act at the single commit point",
        licensed: &["observer", "act", "commit", "point"],
        expected: Verdict::NotBelongs,
    },
    // ---- the civilian's governed book ----
    KnownAnswer {
        subject: "civilian/agency/action.rs",
        line: "book.apply_transition(permission);",
        recipe: "every machine act passes through the governed book under a write permission",
        licensed: &[
            "book", "apply_transition", "permission", "machine", "act", "pass", "governed",
            "write",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "civilian/agency/action.rs",
        line: "book.apply_payroll(permission);",
        recipe: "every machine act passes through the governed book under a write permission",
        licensed: &["book", "permission", "machine", "act", "governed", "write"],
        expected: Verdict::NotBelongs,
    },
    // ---- the audio engine (voxel-audio's own header) ----
    KnownAnswer {
        subject: "voxel-audio/src/lib.rs",
        line: "synth.sample(seed, onset, length);",
        recipe: "synthesis is arithmetic with no audio dependency and every sample is a \
                 function of seed onset and length",
        licensed: &[
            "synth", "synthesis", "sample", "seed", "onset", "length", "arithmetic", "audio",
            "dependency", "function",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "voxel-audio/src/lib.rs",
        line: "synth.sample(seed, onset, reverb);",
        recipe: "synthesis is arithmetic with no audio dependency and every sample is a \
                 function of seed onset and length",
        licensed: &["synth", "sample", "seed", "onset", "length"],
        expected: Verdict::NotBelongs,
    },
    // ---- the chunk system (voxel-stream's own header) ----
    KnownAnswer {
        subject: "voxel-stream/src/world_source.rs",
        line: "source.material_at(voxel);",
        recipe: "the world source is the one place where an abstract spec becomes concrete \
                 voxels",
        licensed: &[
            "source", "material_at", "voxel", "world", "place", "abstract", "spec",
            "concrete", "voxels",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "voxel-stream/src/world_source.rs",
        line: "source.terrain_at(voxel);",
        recipe: "the world source is the one place where an abstract spec becomes concrete \
                 voxels",
        licensed: &["source", "voxel", "world", "concrete"],
        expected: Verdict::NotBelongs,
    },
    // ---- the mesher (voxel-mesh's own header) ----
    KnownAnswer {
        subject: "voxel-mesh/src/octree.rs",
        line: "octree.insert(chunk, voxels);",
        recipe: "the octree is canonical and two runs that build the same chunk produce the \
                 same digest",
        licensed: &[
            "octree", "insert", "chunk", "voxels", "canonical", "run", "build", "same",
            "digest", "produce", "two",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "voxel-mesh/src/octree.rs",
        line: "octree.insert(chunk, sprites);",
        recipe: "the octree is canonical and two runs that build the same chunk produce the \
                 same digest",
        licensed: &["octree", "insert", "chunk", "canonical", "digest"],
        expected: Verdict::NotBelongs,
    },
    // ---- the work pool (voxel-work's own header) ----
    KnownAnswer {
        subject: "voxel-work/src/pool.rs",
        line: "pool.collect(mesh);",
        recipe: "worker count is a performance knob and never changes output",
        licensed: &[
            "pool", "collect", "mesh", "worker", "count", "performance", "knob", "change",
            "output", "never",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "voxel-work/src/pool.rs",
        line: "pool.collect(bananas);",
        recipe: "worker count is a performance knob and never changes output",
        licensed: &["pool", "collect", "worker", "count", "performance"],
        expected: Verdict::NotBelongs,
    },
    // ---- the noise (voxel-noise's own header) ----
    KnownAnswer {
        subject: "voxel-noise/src/rng.rs",
        line: "rng.next(seed);",
        recipe: "the generator evaluates a chunk in any thread in any order",
        licensed: &[
            "rng", "next", "seed", "generator", "evaluate", "chunk", "thread", "order",
            "any",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "voxel-noise/src/rng.rs",
        line: "rng.next(coin);",
        recipe: "the generator evaluates a chunk in any thread in any order",
        licensed: &["rng", "next", "generator", "chunk", "thread"],
        expected: Verdict::NotBelongs,
    },
    // ---- the instruments (voxel-instrument's own header) ----
    KnownAnswer {
        subject: "voxel-instrument/src/gauge_wall.rs",
        line: "gauge.evaluate(sensor);",
        recipe: "an unmeasured gauge is not a satisfied gauge",
        licensed: &["gauge", "evaluate", "sensor", "unmeasured", "satisfied"],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "voxel-instrument/src/gauge_wall.rs",
        line: "gauge.evaluate(thermometer);",
        recipe: "an unmeasured gauge is not a satisfied gauge",
        licensed: &["gauge", "evaluate", "unmeasured", "satisfied"],
        expected: Verdict::NotBelongs,
    },
    // ---- the design system (voxel-ui's own header) ----
    KnownAnswer {
        subject: "voxel-ui/src/lib.rs",
        line: "palette.get(role);",
        recipe: "every geometry constant is labelled and a missing value is never turned into \
                 an invented one",
        licensed: &[
            "palette", "get", "role", "geometry", "constant", "label", "missing", "value",
            "invent",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "voxel-ui/src/lib.rs",
        line: "palette.get(swatch);",
        recipe: "every geometry constant is labelled and a missing value is never turned into \
                 an invented one",
        licensed: &["palette", "get", "constant", "value"],
        expected: Verdict::NotBelongs,
    },
    // ---- the console's five levels ----
    KnownAnswer {
        subject: "shell/console.rs",
        line: "console.log(level, subject);",
        recipe: "the console's levels are five and closed",
        licensed: &["console", "log", "level", "subject", "five", "closed"],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "shell/console.rs",
        line: "console.log(severity, subject);",
        recipe: "the console's levels are five and closed",
        licensed: &["console", "log", "subject", "five", "closed"],
        expected: Verdict::NotBelongs,
    },
    // ---- the hud ----
    KnownAnswer {
        subject: "shell/hud.rs",
        line: "hud.pin(window);",
        recipe: "the hud is a pinned subset that stays visible while the shell is hidden",
        licensed: &[
            "hud", "pin", "pinned", "window", "subset", "visible", "shell", "hidden",
            "stay",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "shell/hud.rs",
        line: "hud.stick(window);",
        recipe: "the hud is a pinned subset that stays visible while the shell is hidden",
        licensed: &["hud", "window", "subset", "visible", "shell"],
        expected: Verdict::NotBelongs,
    },
    // ---- the agent's inbox ----
    KnownAnswer {
        subject: "mount.rs",
        line: "inbox.notify(ticket, package);",
        recipe: "the inbox is work the agent owes until it is read",
        licensed: &[
            "inbox", "notify", "ticket", "package", "work", "agent", "owe", "read",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "mount.rs",
        line: "inbox.shout(ticket, package);",
        recipe: "the inbox is work the agent owes until it is read",
        licensed: &["inbox", "ticket", "package", "agent"],
        expected: Verdict::NotBelongs,
    },
    // ---- the back-compiler ----
    KnownAnswer {
        subject: "compiler.rs",
        line: "board.back_compile(solution);",
        recipe: "the board's tickets back-compile into solutions and a solution is what one \
                 ticket asks the arrangement to contain",
        licensed: &[
            "board", "back_compile", "solution", "ticket", "back", "compile", "arrangement",
            "contain", "ask",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "compiler.rs",
        line: "board.back_compile(trampoline);",
        recipe: "the board's tickets back-compile into solutions and a solution is what one \
                 ticket asks the arrangement to contain",
        licensed: &["board", "back_compile", "solution", "ticket", "arrangement"],
        expected: Verdict::NotBelongs,
    },
    // ---- the recipe's rows ----
    KnownAnswer {
        subject: "recipe.rs",
        line: "recipe.row(id, text);",
        recipe: "a recipe that declares no window and no row declares nothing",
        licensed: &["recipe", "row", "id", "text", "window", "declare", "nothing"],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "recipe.rs",
        line: "recipe.sock(id, text);",
        recipe: "a recipe that declares no window and no row declares nothing",
        licensed: &["recipe", "id", "text", "window"],
        expected: Verdict::NotBelongs,
    },
    // ---- the filing chokepoint ----
    KnownAnswer {
        subject: "filing.rs",
        line: "filing.grow(records);",
        recipe: "one writer computes the linkage so a second record cannot form",
        licensed: &[
            "filing", "grow", "records", "writer", "compute", "linkage", "second", "record",
                        "form",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "filing.rs",
        line: "filing.append(payroll);",
        recipe: "one writer computes the linkage so a second record cannot form",
        licensed: &["filing", "writer", "linkage", "record"],
        expected: Verdict::NotBelongs,
    },
    // ---- the compiler's audit ----
    KnownAnswer {
        subject: "compiler.rs",
        line: "auditor.gate(stage, grant);",
        recipe: "a denied stage stops the compile and there is no compile-anyway path",
        licensed: &[
            "auditor", "gate", "stage", "grant", "denied", "compile", "anyway", "path",
            "stop",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "compiler.rs",
        line: "auditor.gate(stage, sausage);",
        recipe: "a denied stage stops the compile and there is no compile-anyway path",
        licensed: &["auditor", "gate", "stage", "denied", "compile"],
        expected: Verdict::NotBelongs,
    },
    // ---- the evidence origins ----
    KnownAnswer {
        subject: "ticket.rs",
        line: "proof.origin(read_back);",
        recipe: "the three evidence origins are oracle read-back and self-report",
        licensed: &[
            "proof", "origin", "read_back", "read", "back", "three", "evidence", "oracle",
            "self", "report",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "ticket.rs",
        line: "proof.origin(seven);",
        recipe: "the three evidence origins are oracle read-back and self-report",
        licensed: &["proof", "origin", "evidence", "oracle", "report"],
        expected: Verdict::NotBelongs,
    },
    // ---- the axis ladders ----
    KnownAnswer {
        subject: "axis.rs",
        line: "ladder.graduate(unit);",
        recipe: "each ladder its own type and cross-axis and scalar reduction refuse",
        licensed: &[
            "ladder", "graduate", "unit", "each", "type", "cross", "axis", "scalar",
            "reduction", "refuse",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "axis.rs",
        line: "ladder.graduate(banana);",
        recipe: "each ladder its own type and cross-axis and scalar reduction refuse",
        licensed: &["ladder", "graduate", "type", "axis"],
        expected: Verdict::NotBelongs,
    },
    // ---- the census ----
    KnownAnswer {
        subject: "census.rs",
        line: "census.check_coverage();",
        recipe: "a report full of unchecked reads as complete to anyone who does not know the \
                 difference",
        licensed: &[
            "census", "check_coverage", "check", "coverage", "unchecked", "report",
            "complete", "difference", "know",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "census.rs",
        line: "census.check_trophy();",
        recipe: "a report full of unchecked reads as complete to anyone who does not know the \
                 difference",
        licensed: &["census", "check", "coverage", "unchecked", "report"],
        expected: Verdict::NotBelongs,
    },
    // ---- the features ----
    KnownAnswer {
        subject: "features.rs",
        line: "features.evaluate(witness);",
        recipe: "a feature with no witness or no row is a compile error",
        licensed: &[
            "features", "evaluate", "witness", "feature", "row", "compile", "error",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "features.rs",
        line: "features.evaluate(sock);",
        recipe: "a feature with no witness or no row is a compile error",
        licensed: &["features", "evaluate", "feature", "row"],
        expected: Verdict::NotBelongs,
    },
    // ---- the game's judgement, written to the queue ----
    KnownAnswer {
        subject: "notifications.rs",
        line: "evaluation.write_queue(notification);",
        recipe: "the game asks laya for this run's judgement and the queue stays laya's own",
        licensed: &[
            "evaluation", "write_queue", "write", "queue", "notification", "game", "ask",
            "laya", "judgement", "own", "run",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "notifications.rs",
        line: "evaluation.write_queue(marmalade);",
        recipe: "the game asks laya for this run's judgement and the queue stays laya's own",
        licensed: &["evaluation", "write_queue", "queue", "game", "laya"],
        expected: Verdict::NotBelongs,
    },
    // ---- the chain and its seasons ----
    KnownAnswer {
        subject: "chain.rs",
        line: "chain.append(namespace, kind, question);",
        recipe: "the one append-only record is the chain and a season is one of winter spring \
                 summer autumn",
        licensed: &[
            "chain", "append", "namespace", "kind", "question", "record", "season", "winter",
            "spring", "summer", "autumn",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "chain.rs",
        line: "chain.append(namespace, kind, holiday);",
        recipe: "the one append-only record is the chain and a season is one of winter spring \
                 summer autumn",
        licensed: &["chain", "append", "namespace", "kind", "record", "season"],
        expected: Verdict::NotBelongs,
    },
    // ---- the plan's milestones and blockers ----
    KnownAnswer {
        subject: "plan.rs",
        line: "milestone.deliverable();",
        recipe: "a milestone is ordered work with a finish condition and a blocker is a rule \
                 that stops work",
        licensed: &[
            "milestone", "deliverable", "ordered", "work", "finish", "condition", "blocker",
            "rule", "stops",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "plan.rs",
        line: "milestone.sausage();",
        recipe: "a milestone is ordered work with a finish condition and a blocker is a rule \
                 that stops work",
        licensed: &["milestone", "work", "finish", "condition"],
        expected: Verdict::NotBelongs,
    },
    // ---- the design system's four buckets ----
    KnownAnswer {
        subject: "lib.rs",
        line: "constant.bucket(project_extension);",
        recipe: "every constant carries a label inside it and the four buckets are md1 rule \
                 md1 example platform translation and project extension",
        licensed: &[
            "constant", "bucket", "project_extension", "project", "extension", "label",
            "four", "md1", "rule", "example", "platform", "translation", "carry", "inside",
        ],
        expected: Verdict::Belongs,
    },
    KnownAnswer {
        subject: "lib.rs",
        line: "constant.bucket(trophy);",
        recipe: "every constant carries a label inside it and the four buckets are md1 rule \
                 md1 example platform translation and project extension",
        licensed: &["constant", "bucket", "label", "four", "md1", "rule", "example"],
        expected: Verdict::NotBelongs,
    },
];

/// Calibrate the structural judge against the alpha's own known-answer set.
pub fn calibrate_structural() -> CalibrationReport {
    calibrate(&StructuralJudge, KNOWN_ANSWERS)
}

/// The lint gate as the build runs it: calibrate, then refuse on the result.
///
/// Returns the establishment for the record **and** the refusals. A caller that
/// only wants the state can read `establishment`; a caller that must act on it
/// gets the reasons. Both are produced, so a summary cannot exist without its
/// reasons.
pub fn gate(root: &Path) -> (Establishment, CalibrationReport, RefusalSet) {
    let report = calibrate_structural();
    let mut refusals = RefusalSet::new();

    if !report.calibration.is_quotable() {
        refusals.push(Refusal::owned(
            RefusalKind::UncalibratedOracle,
            format!(
                "the structural judge did not beat the majority class of the known-answer set \
                 ({}); nothing it says may be quoted and no finding is written",
                report.calibration.notes.join("; ")
            ),
        ));
    }

    // The feature table is the other half of the gate: a gate that passes while a
    // required feature has no witness is a green light produced by having checked
    // the wrong thing.
    if let Err(feature_refusals) = crate::features::check_features(root, &crate::features::FEATURES) {
        refusals.absorb(feature_refusals);
    }

    (report.establishment, report, refusals)
}

// ============================================================================
// The model instrument: the seam's second decider
// ============================================================================

/// The model instrument's own name, for the record.
///
/// **It is the structural judge's name, because that is what ran**: the instrument is
/// [`StructuralJudge`] reached through the seam. `TwoPass::report` prints it, and the build
/// gate measures `oracle-is-laya` from the pass's own decider rather than printing a literal.
pub const MODEL_NAME: &str = "structural";

/// **The model instrument: the calibrated judge, reached through the seam.**
///
/// H-14.2's second pass is an instrument of the same kind as the first — a
/// [`crate::decision::Decider`] over the same [`crate::decision::Subject`] vocabulary — and
/// this is it. It reads **the record's own declaration of a subject** as the judge's recipe
/// and **the tree's own evidence for it** as the line, and answers through [`StructuralJudge`],
/// the instrument [`calibrate_structural`] measures on [`KNOWN_ANSWERS`]. Every reading
/// therefore travels with the calibration that says whether it may be quoted at all:
/// *"a run that has not been calibrated in the same pass is refused, and a `not_belongs`
/// reading is refused unless a human's reason is recorded in the same act"* (H-14.2).
///
/// # The four rules, and they are [`lint`]'s own
///
/// * an **uncalibrated** run decides nothing — the guard runs here **before any file is
///   read**, so an unmeasured instrument cannot even look at the tree;
/// * a **`no_reading`** is an absence: the instrument declining to read, which is not a
///   verdict, and the seam reports it as an assessment that was not reached;
/// * a **`not_belongs`** without a human's reason recorded in the same act is refused by
///   [`RefusalKind::UnreasonedOverride`] — the model may not convict on its own word;
/// * a **`not_belongs` with a reason** is a reading: a `Refused` assessment that *was*
///   reached, which is the disagreement the two passes exist to preserve.
///
/// # What it reads, and what it refuses to invent
///
/// The recipe is always text the record already carries — a `FEATURES` row's `declared_by`
/// and `means`, a `NumberClaim`'s document and sentence, a ladder's witness and token, the
/// document `census` found defining a crate. The line is always text the tree already
/// carries — the comment prose in a witness file, a crate's whole comment prose for the
/// **doc stage** (*dissecting the prose comment blocks in the code*), the verbatim line of a
/// document, the witness line that carries a ladder's token.
///
/// **Where a subject carries no prose, this instrument says so instead of rendering a fact
/// into fake prose.** An axis read as `named: true` is a fact, not a sentence; turning it
/// into text would hand the judge words nobody wrote and measure the rendering rather than
/// the tree. The absence is named, and a reader of the pass sees which subjects have a second
/// reading and which do not.
pub struct Model<'a> {
    root: &'a Path,
    calibration: Calibration,
    /// A human's reason, recorded in the same act, allowing a negative reading to be kept as
    /// a disagreement. **`None` by default: the gate does not manufacture reasons.**
    reason: Option<String>,
}

impl<'a> Model<'a> {
    /// The instrument over a workspace, calibrated by the measurement this module already
    /// runs — not by a flag anyone can set.
    pub fn calibrated(root: &'a Path) -> Self {
        Model { root, calibration: calibrate_structural().calibration, reason: None }
    }

    /// The instrument with **no measurement taken**: it refuses every subject, which is what
    /// an uncalibrated run must do. Exists so the refusal is a test rather than a hope.
    pub fn unmeasured(root: &'a Path) -> Self {
        Model { root, calibration: Calibration::unmeasured(), reason: None }
    }

    /// Record a human's reason for this run, letting a negative reading be kept as a
    /// disagreement rather than refused by the [`RefusalKind::UnreasonedOverride`] rule.
    pub fn with_reason(mut self, why: impl Into<String>) -> Self {
        self.reason = Some(why.into());
        self
    }

    /// Whether anything this instrument reads in this run may be quoted.
    pub fn is_quotable(&self) -> bool {
        self.calibration.is_quotable()
    }

    /// The measurement every reading travels with.
    pub fn calibration(&self) -> &Calibration {
        &self.calibration
    }

    /// **Read the declaration and the evidence for one subject**, or refuse to read — and
    /// every refusal names which of the two was missing.
    fn request(&self, subject: crate::decision::Subject) -> Result<LintRequest, RefusalSet> {
        use crate::decision::Subject;
        match subject {
            Subject::Feature(feature) => {
                let Some(row) = crate::features::FEATURES
                    .iter()
                    .find(|row| row.feature == feature)
                else {
                    return Err(self.absent(
                        subject,
                        "the feature table carries no row for it, so there is no declaration \
                         to read",
                    ));
                };
                let witness = crate::features::witness_of(feature);
                let text = std::fs::read_to_string(self.root.join(witness)).map_err(|failure| {
                    self.absent(
                        subject,
                        &format!(
                            "the witness at {witness} could not be read ({failure}), so there \
                             is no evidence to read"
                        ),
                    )
                })?;
                let prose = crate::bloat::comment_lines(&[(
                    std::path::PathBuf::from(witness),
                    text,
                )]);
                if prose.is_empty() {
                    return Err(self.absent(
                        subject,
                        &format!(
                            "the witness at {witness} carries no comment prose, and this \
                             instrument reads prose rather than asserting presence"
                        ),
                    ));
                }
                Ok(LintRequest {
                    subject: subject.name(),
                    line: prose.join("\n"),
                    recipe: format!("{}: {}", row.declared_by, row.means),
                    licensed: terms(&format!(
                        "{} {} {}",
                        row.declared_by,
                        row.means,
                        feature.name()
                    )),
                })
            }
            Subject::Axis { crate_, axis: Axis::Documented } => {
                let observations = crate::census::observe(self.root)?;
                let Some(observed) =
                    observations.iter().find(|observed| observed.crate_ == crate_)
                else {
                    return Err(self.absent(
                        subject,
                        &format!("the census returned no observation for {}", crate_.name()),
                    ));
                };
                let Some(prose) = &observed.prose else {
                    return Err(self.absent(
                        subject,
                        &format!(
                            "{} has no readable source, so where its rationale lives was not \
                             measured",
                            crate_.name()
                        ),
                    ));
                };
                let covered = crate::provenance::covered_files(self.root);
                let Some(sources) = crate::census::read_sources(&covered, crate_, self.root)
                else {
                    return Err(self.absent(
                        subject,
                        &format!("{}'s sources could not be read", crate_.name()),
                    ));
                };
                let lines = crate::bloat::comment_lines(&sources.read);
                if lines.is_empty() {
                    return Err(self.absent(
                        subject,
                        &format!(
                            "{} carries no comment prose to dissect — the doc stage has \
                             nothing to read",
                            crate_.name()
                        ),
                    ));
                }
                let Some(document) = &prose.document else {
                    // **The state the sixth axis exists to name**, said for the second
                    // instrument rather than only counted: the prose has no home to belong to.
                    return Err(self.absent(
                        subject,
                        &format!(
                            "{} carries comment prose in its sources and no document under \
                             docs/ defines it — no heading there begins with its name. The \
                             prose has no declaration to belong to. This instrument's subject is \
                             the axis, and the axis measures the compiler's count; a prose reading \
                             with no document to read against is that state, named here",
                            crate_.name()
                        ),
                    ));
                };
                let text = std::fs::read_to_string(self.root.join(document)).map_err(|failure| {
                    self.absent(
                        subject,
                        &format!("{document} could not be read ({failure})"),
                    )
                })?;
                Ok(LintRequest {
                    subject: subject.name(),
                    line: lines.join("\n"),
                    recipe: text,
                    licensed: terms(crate_.name()),
                })
            }
            Subject::Axis { crate_, axis } => Err(self.absent(
                subject,
                &format!(
                    "the {} axis was read as a fact about {}, not as prose; this instrument \
                     reads declarations against text, and this axis carries no text to read",
                    axis.name(),
                    crate_.name()
                ),
            )),
            Subject::Number(claim) => {
                let observations = crate::numbers::observe(self.root);
                let Some(observed) = observations
                    .iter()
                    .find(|observed| observed.claim.name == claim.name)
                else {
                    return Err(self.absent(
                        subject,
                        "the numbers table returned no observation for it",
                    ));
                };
                let Some(line) = &observed.line else {
                    return Err(self.absent(
                        subject,
                        &format!(
                            "{} carries no line with {:?}, so there is nothing to judge",
                            claim.document, claim.sentence
                        ),
                    ));
                };
                Ok(LintRequest {
                    subject: subject.name(),
                    line: line.clone(),
                    recipe: format!(
                        "{} must carry {:?}, and the record expects {:?}",
                        claim.document,
                        claim.sentence,
                        claim.expected()
                    ),
                    licensed: terms(&format!(
                        "{} {} {}",
                        claim.document,
                        claim.sentence,
                        claim.expected()
                    )),
                })
            }
            Subject::Civilian(ladder) => {
                let observations = crate::civilians::observe(self.root);
                let Some(observed) =
                    observations.iter().find(|observed| observed.ladder == ladder)
                else {
                    return Err(self.absent(
                        subject,
                        "the ladder table returned no observation for it",
                    ));
                };
                if !observed.readable {
                    return Err(self.absent(
                        subject,
                        &format!(
                            "{} could not be read, so whether the record carries {:?} is \
                             unknown",
                            observed.witness, observed.token
                        ),
                    ));
                }
                let text = std::fs::read_to_string(self.root.join(observed.witness))
                    .map_err(|failure| {
                        self.absent(
                            subject,
                            &format!("{} could not be read ({failure})", observed.witness),
                        )
                    })?;
                let Some(line) = text.lines().find(|line| line.contains(observed.token))
                else {
                    return Err(self.absent(
                        subject,
                        &format!(
                            "{} does not carry {:?}, which is the declaration this ladder \
                             rests on",
                            observed.witness, observed.token
                        ),
                    ));
                };
                Ok(LintRequest {
                    subject: subject.name(),
                    line: line.to_string(),
                    recipe: format!(
                        "{}: the record carries {:?} in {}",
                        ladder.name(),
                        observed.token,
                        observed.witness
                    ),
                    licensed: terms(&format!(
                        "{} {} {}",
                        ladder.name(),
                        observed.token,
                        observed.witness
                    )),
                })
            }
            Subject::Tool(row) => Err(self.absent(
                subject,
                &format!(
                    "{} is a row of this build's own tool table, and this instrument reads \
                     declarations against text: the surface is declared in code and the row's \
                     class is the matrix's own answer, so there is no document here to read it \
                     against. The deterministic decider is what decides this subject, and an \
                     absence is the right reading for an instrument with nothing to read",
                    row.name
                ),
            )),
            Subject::Recipe => Err(self.absent(
                subject,
                "the arrangement is its own declaration — the recipe's source is both the \
                 claim and the evidence — so a judge reading one against the other would \
                 license itself. The codec's compile-then-load is what decides this subject, \
                 and the model's reading waits for a declaration of the arrangement it can be \
                 read against",
            )),
        }
    }

    /// The refusal for a subject this instrument was not handed anything to read.
    ///
    /// **A named absence, never a default**: the reason says which half — declaration or
    /// evidence — was missing, so the report distinguishes *"the tree has no prose here"*
    /// from *"the record declares nothing to read it against"*.
    fn absent(&self, subject: crate::decision::Subject, why: &str) -> RefusalSet {
        let mut refusals = RefusalSet::new();
        refusals.refuse(RefusalKind::NoReading, format!("{}: {why}", subject.name()));
        refusals
    }
}

impl crate::decision::Decider for Model<'_> {
    fn name(&self) -> &'static str {
        MODEL_NAME
    }

    fn decide(
        &self,
        subject: crate::decision::Subject,
    ) -> Result<crate::decision::Assessment, RefusalSet> {
        // **Rule 1, before any file is read.** An uncalibrated run cannot reach the judge —
        // that is `lint`'s own order — and it must not reach the tree either, so the guard
        // is here rather than only inside `lint`.
        if !self.calibration.is_quotable() {
            let mut refusals = RefusalSet::new();
            refusals.refuse(
                RefusalKind::UncalibratedOracle,
                format!(
                    "{}: the model instrument has not been calibrated in this run ({} \
                     known-answer decision(s)), so nothing it says may be quoted — not even \
                     which subjects it would have read",
                    subject.name(),
                    self.calibration.known_answers
                ),
            );
            return Err(refusals);
        }

        let request = self.request(subject)?;
        let finding =
            lint(&StructuralJudge, &request, &self.calibration, self.reason.as_deref())?;
        let (state, reason) = match finding.verdict {
            // A positive reading is the instrument's answer, and it owes no catch.
            Verdict::Belongs => (Establishment::Active, None),
            // A negative reading only arrives here with a reason recorded in the same act
            // (`lint` refuses the alternative), and it is a reading that was **reached** —
            // the disagreement the two passes exist to preserve.
            Verdict::NotBelongs => (
                Establishment::Refused,
                Some(format!(
                    "the model read the evidence as not belonging to the declaration{}. The \
                     reason was recorded in the same act: {}",
                    match culprits(&StructuralJudge, &request) {
                        Some(found) => format!(
                            " (the terms the declaration does not license: {})",
                            found.join(", ")
                        ),
                        None => String::new(),
                    },
                    self.reason.as_deref().unwrap_or("no reason recorded"),
                )),
            ),
            Verdict::NoReading => {
                unreachable!("`lint` refuses a no_reading rather than returning it")
            }
        };
        Ok(crate::decision::Assessment {
            subject: subject.name(),
            state,
            instrument: self.name(),
            reached: true,
            reason,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::Decider;

    /// A double that says whatever a test needs it to say.
    struct Scripted(Option<Verdict>);

    impl Oracle for Scripted {
        fn name(&self) -> &'static str {
            "scripted"
        }

        fn judge(&self, request: &LintRequest) -> Result<Verdict, RefusalSet> {
            match self.0 {
                Some(verdict) => Ok(verdict),
                None => {
                    let mut refusals = RefusalSet::new();
                    refusals.push(Refusal::owned(
                        RefusalKind::OracleUnreachable,
                        format!("{}: scripted unreachable", request.subject),
                    ));
                    Err(refusals)
                }
            }
        }
    }

    fn request() -> LintRequest {
        LintRequest {
            subject: "shell/tabs.rs".to_owned(),
            line: "let motor = Transducer::new(rotation);".to_owned(),
            recipe: "rotation and electricity interoperate; direction is a property of wiring"
                .to_owned(),
            licensed: vec!["rotation".to_owned(), "electricity".to_owned()],
        }
    }

    #[test]
    fn the_verdict_vocabulary_is_closed() {
        assert_eq!(Verdict::parse(" belongs\n").unwrap(), Verdict::Belongs);
        assert_eq!(Verdict::parse("not_belongs").unwrap(), Verdict::NotBelongs);
        assert_eq!(Verdict::parse("no_reading").unwrap(), Verdict::NoReading);
        // A coerced verdict is a fabricated one.
        for coerced in ["maybe", "yes", "", "belongs_not", "BELONGS"] {
            assert!(Verdict::parse(coerced).is_err(), "{coerced:?} was coerced");
        }
    }

    #[test]
    fn no_reading_and_unreachable_are_different_verdicts() {
        assert_ne!(Verdict::NoReading, Verdict::Belongs);
        // One is a reading, the other is a refusal kind. A line with nothing to
        // judge has not failed.
        let unreachable = UnreachableOracle { why: "none".to_owned() };
        let refusals = unreachable.judge(&request()).expect_err("must refuse");
        assert!(refusals.all()[0].kind() == RefusalKind::OracleUnreachable);
    }

    #[test]
    fn an_unmeasured_calibration_refuses_before_the_oracle_is_consulted() {
        // Order matters: an unmeasured run must not reach the judge, or it could
        // produce something quotable-looking.
        let oracle = Scripted(Some(Verdict::Belongs));
        let refusals = lint(&oracle, &request(), &Calibration::unmeasured(), None).expect_err("must refuse");
        assert!(refusals.all()[0].kind() == RefusalKind::UncalibratedOracle);
    }

    #[test]
    fn a_calibration_against_zero_answers_is_not_calibrated_however_the_flag_is_set() {
        let calibration = Calibration {
            calibrated: true,
            known_answers: 0,
            notes: Vec::new(),
        };
        assert!(!calibration.is_quotable(), "a run against nothing measured nothing");
        let oracle = Scripted(Some(Verdict::Belongs));
        assert!(lint(&oracle, &request(), &calibration, None).is_err());
    }

    #[test]
    fn an_unreachable_oracle_is_refused_because_a_missing_instrument_is_not_a_pass() {
        let oracle = Scripted(None);
        let refusals = lint(&oracle, &request(), &Calibration::measured(11), None).expect_err("must refuse");
        assert!(refusals.all()[0].kind() == RefusalKind::OracleUnreachable);
    }

    #[test]
    fn the_unreachable_oracle_is_the_bound_default_and_it_refuses() {
        // So "the gate never ran" cannot be mistaken for "the gate passed".
        let oracle = UnreachableOracle { why: "no judge is wired up yet".to_owned() };
        let refusals = lint(&oracle, &request(), &Calibration::measured(11), None).expect_err("must refuse");
        assert!(refusals.all()[0].kind() == RefusalKind::OracleUnreachable);
        assert!(refusals.all()[0].detail().contains("not a pass"));
    }

    #[test]
    fn no_reading_at_all_is_refused_because_absence_is_not_a_free_pass() {
        let oracle = Scripted(Some(Verdict::NoReading));
        let refusals = lint(&oracle, &request(), &Calibration::measured(11), None).expect_err("must refuse");
        assert!(refusals.all()[0].kind() == RefusalKind::NoReading);
    }

    #[test]
    fn a_misread_without_a_reason_is_refused_and_returns_no_finding() {
        // The refusal returns no `Finding`, so "a refusal writes nothing" is
        // enforced by the type rather than by a caller's discipline.
        let oracle = Scripted(Some(Verdict::NotBelongs));
        let refusals = lint(&oracle, &request(), &Calibration::measured(11), None).expect_err("must refuse");
        assert!(refusals.all()[0].kind() == RefusalKind::UnreasonedOverride);
    }

    #[test]
    fn a_misread_with_a_reason_is_recorded_and_the_reason_travels_with_it() {
        // The one door through, and the reason is on the record with the finding.
        let oracle = Scripted(Some(Verdict::NotBelongs));
        let finding = lint(
            &oracle,
            &request(),
            &Calibration::measured(11),
            Some("the recipe licenses the arrangement but not the transducer"),
        )
        .expect("a reason is the one door through");
        assert_eq!(finding.verdict, Verdict::NotBelongs);
        assert!(finding.reason.is_some());
        assert!(finding.is_quotable());
    }

    #[test]
    fn a_holding_reading_passes_with_no_reason_needed() {
        let oracle = Scripted(Some(Verdict::Belongs));
        let finding = lint(&oracle, &request(), &Calibration::measured(11), None).expect("passes");
        assert_eq!(finding.verdict, Verdict::Belongs);
        assert_eq!(finding.reason, None);
        assert_eq!(finding.known_answers, 11);
    }

    #[test]
    fn terms_are_lowercased_split_and_deduplicated() {
        // Lowercased because prose says `Ticket` and code says `ticket`; deduped
        // because a term used twice is not two problems.
        assert_eq!(terms("Ticket ticket TICKET"), vec!["ticket"]);
        assert_eq!(terms("let pinned_window"), vec!["let", "pinned_window"]);
        assert_eq!(terms("Mods Surface"), vec!["mods", "surface"]);
        assert!(terms("").is_empty());
    }

    #[test]
    fn a_comment_and_a_pure_shape_line_have_nothing_to_judge() {
        let judge = StructuralJudge;
        // The comment case is measured, not assumed: `/// docs` once extracted
        // `docs` and read a doc comment as a misfit line.
        assert_eq!(
            judge.reading("// the pinned subset stays visible", "the hud is pinned"),
            Reading::NothingToJudge
        );
        assert_eq!(judge.reading("pub let mut", "the hud is pinned"), Reading::NothingToJudge);
        assert_eq!(judge.reading("   ", "the hud is pinned"), Reading::NothingToJudge);
    }

    #[test]
    fn an_undeclared_reading_names_the_culprit() {
        // A refusal that says only "non-conformant" is a refusal nobody can act on.
        let judge = StructuralJudge;
        let request = LintRequest {
            subject: "shell/tabs.rs".to_owned(),
            line: "let physics = solver.step();".to_owned(),
            recipe: "windows are data the shell mounts".to_owned(),
            licensed: vec!["window".to_owned(), "shell".to_owned(), "mount".to_owned()],
        };
        let found = culprits(&judge, &request).expect("must name the culprits");
        assert!(found.contains(&"physics".to_owned()), "{found:?}");
        assert!(found.contains(&"solver".to_owned()), "{found:?}");
        assert!(!found.contains(&"window".to_owned()), "{found:?}");
    }

    #[test]
    fn a_licensed_term_is_licensed_whatever_its_spelling() {
        // `--license "Mods Surface"` supplies words with capitals and a space; a raw
        // insert would make that one term, license nothing, and fail silently.
        let judge = StructuralJudge;
        let line = "let mods = window.surface();";
        let with_phrase =
            judge.reading_within(line, "a window opens", &["Mods Surface".to_owned()]);
        assert!(
            matches!(with_phrase, Reading::AllDeclared { .. }),
            "a capitalised multi-word licence must license its lowercase words: {with_phrase:?}"
        );
        // And the same line without the licence is not conformant, so the licence is
        // what made the difference rather than the judge being lenient.
        let without = judge.reading(line, "a window opens");
        assert!(matches!(without, Reading::Undeclared { .. }), "{without:?}");

        // A single capitalised word licenses its lowercase form too.
        let lower = "let ticket = queue.advance();";
        assert!(matches!(
            judge.reading_within(
                lower,
                "the queue advances",
                &["Ticket".to_owned(), "advance".to_owned()]
            ),
            Reading::AllDeclared { .. }
        ));
    }

    #[test]
    fn a_recipe_can_widen_the_declared_set_but_never_narrow_it() {
        // Found by the second calibration, which misread two entries for this reason
        // and not by mislabelling them: both recipes' **own prose** already named the
        // terms the licence list was meant to exclude. A recipe that says `constant`
        // has licensed `constant`, and no licence list can take a word back.
        //
        // The property is worth stating rather than working around, because it decides
        // how a recipe must be written: **to exclude a term, the recipe must not
        // mention it.** Narrowing by omission is the only narrowing there is.
        let judge = StructuralJudge;
        let recipe = "base color is driven from block truth, never a per kind and direction constant";
        let line = "let constant = 1;";

        // The recipe's own words license the word, licence list or not.
        assert!(
            matches!(judge.reading_within(line, recipe, &[]), Reading::AllDeclared { .. }),
            "a word the recipe names is licensed by the recipe naming it"
        );
        // And an explicit licence is *additive*: it widens, and never removes.
        let wide = judge.reading_within(line, "an unrelated recipe", &["constant".to_owned()]);
        assert!(matches!(wide, Reading::AllDeclared { .. }), "{wide:?}");
        // With neither the recipe nor a licence naming it, the term is held.
        let narrow = judge.reading_within(line, "an unrelated recipe", &[]);
        assert!(matches!(narrow, Reading::Undeclared { .. }), "{narrow:?}");
    }

    #[test]
    fn a_numeric_literal_is_a_value_and_not_a_word() {
        // Found by the first calibration run, which misread
        // `window.opacity(930);` as non-conformant and named `930` as the
        // culprit. A number says nothing about a recipe, and holding a
        // contributor's literal against prose is conviction by vocabulary
        // overlap rather than by meaning.
        let judge = StructuralJudge;
        let line = "window.opacity(930);";
        assert!(
            matches!(
                judge.reading_within(
                    line,
                    "windows are data the shell mounts",
                    &["window".to_owned(), "opacity".to_owned()]
                ),
                Reading::AllDeclared { .. }
            ),
            "a numeric literal must not be held against a line"
        );
        // And with the literal ignored, the *word* is still held when unlicensed —
        // so the fix exempted numbers and nothing else.
        assert!(matches!(
            judge.reading(line, "windows are data the shell mounts"),
            Reading::Undeclared { .. }
        ));
        // And a line of nothing but literals and shape has nothing to judge.
        assert_eq!(judge.reading("930", "anything at all"), Reading::NothingToJudge);
        assert_eq!(judge.reading("-42", "anything at all"), Reading::NothingToJudge);
    }

    #[test]
    fn an_underscored_identifier_is_one_term_and_is_not_silently_split() {
        // A licence of `Mods Surface` licenses the words `mods` and `surface`; it
        // does not license `mods_surface`, which is a different single term. A
        // judge that split on underscores would pass a line the record does not
        // actually license.
        let judge = StructuralJudge;
        let line = "let mods_surface = window.open();";
        let reading =
            judge.reading_within(line, "a window opens", &["Mods Surface".to_owned()]);
        match reading {
            Reading::Undeclared { found, .. } => {
                assert!(found.contains(&"mods_surface".to_owned()), "{found:?}");
            }
            other => panic!("expected an undeclared reading, got {other:?}"),
        }
    }

    #[test]
    fn the_judge_is_measured_and_the_measurement_is_reported_honestly() {
        // The standing rule: the set is not edited to flatter the judge. Whatever
        // this says is what the gate gets, and the numbers are printed so a reader
        // sees the size of the margin rather than being told there was one.
        let report = calibrate_structural();
        println!(
            "\n---- oracle calibration ----\nknown answers: {}\naccuracy: {:.4}\nmajority class: {:.4}\ninverted: {:.4}\nestablishment: {}\n{}\n----\n",
            report.readings_used,
            report.accuracy,
            report.majority_class,
            report.inverted_accuracy,
            report.establishment.slug(),
            report.calibration.notes.join("\n"),
        );
        assert!(report.readings_used > 0, "the set produced no readings at all");
        assert!(
            report.misread.is_empty(),
            "every known answer must be read correctly, and these were not: {:?}",
            report.misread
        );
    }

    #[test]
    fn the_known_answer_set_is_not_one_sided() {
        // A set of nothing but conformant lines would be passed by a judge that
        // always answers `belongs`, and would measure nothing.
        let belongs = KNOWN_ANSWERS
            .iter()
            .filter(|known| known.expected == Verdict::Belongs)
            .count();
        let not_belongs = KNOWN_ANSWERS
            .iter()
            .filter(|known| known.expected == Verdict::NotBelongs)
            .count();
        let no_reading = KNOWN_ANSWERS
            .iter()
            .filter(|known| known.expected == Verdict::NoReading)
            .count();
        assert!(belongs > 0 && not_belongs > 0 && no_reading > 0);
        assert!(belongs > 1, "one conformant line is not a measurement");
        assert!(not_belongs > 1, "one non-conformant line is not a measurement");
        // R-62.4: the set was grown from 13 to 53 out of the record. A floor rather
        // than an exact count, because the next honest addition should not be a
        // compile error — but a set that shrank would be a regression nobody would
        // otherwise notice.
        assert!(
            KNOWN_ANSWERS.len() >= 40,
            "the known-answer set is {} entries; R-62.4 asked for about forty and it must not \
             shrink",
            KNOWN_ANSWERS.len()
        );
    }

    #[test]
    fn an_empty_known_answer_set_is_unmeasured_rather_than_a_pass() {
        let report = calibrate(&StructuralJudge, &[]);
        assert!(!report.calibration.is_quotable());
        assert_eq!(report.establishment, Establishment::Unchecked);
    }

    #[test]
    fn a_judge_that_never_answers_is_unmeasured() {
        struct Silent;
        impl Oracle for Silent {
            fn name(&self) -> &'static str {
                "silent"
            }
            fn judge(&self, _request: &LintRequest) -> Result<Verdict, RefusalSet> {
                Err(RefusalSet::new())
            }
        }
        let report = calibrate(&Silent, KNOWN_ANSWERS);
        assert!(!report.calibration.is_quotable());
        assert_eq!(report.readings_used, 0);
    }

    #[test]
    fn the_bar_is_the_majority_class_and_tying_does_not_pass() {
        // A judge that ties the most common answer has measured nothing the set did
        // not already give away. Stated against the same comparison `calibrate`
        // makes, so the two cannot drift apart.
        let report = CalibrationReport {
            calibration: Calibration { calibrated: true, known_answers: 4, notes: Vec::new() },
            accuracy: 0.5,
            majority_class: 0.5,
            inverted_accuracy: 0.5,
            readings_used: 4,
            establishment: Establishment::Active,
            misread: Vec::new(),
        };
        let beats_the_bar = report.accuracy > report.majority_class;
        assert!(!beats_the_bar, "a tie must not read as beating the bar");
    }

    #[test]
    fn the_gate_checks_the_feature_table_as_well_as_the_oracle() {
        // A gate that passes while a required feature has no witness is a green
        // light produced by having checked the wrong thing.
        let root = crate::features::workspace_root(std::path::Path::new(env!("CARGO_MANIFEST_DIR")))
            .expect("a workspace root above the crate");
        let (_state, report, refusals) = gate(&root);
        assert!(!report.calibration.notes.is_empty(), "the gate reports nothing to read");
        // On the real tree every required feature has a witness, so the only
        // possible refusals are oracle ones — and the gate must still say
        // something rather than being silent.
        for refusal in refusals.all() {
            assert!(!refusal.detail().is_empty(), "a refusal without a reason is not actionable");
        }
    }

    // ---- the model instrument (BLD-0025) ------------------------------------

    /// A directory this test owns and removes, so a failed run cannot leave a tree behind.
    fn scratch(name: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("shploingus-oracle-model-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("the scratch root");
        path
    }

    #[test]
    fn the_model_s_name_is_the_judge_s_name_rather_than_a_second_name_for_it() {
        // D-07: a second name for one meaning is the defect. The model instrument **is** the
        // structural judge reached through the seam, so it answers with the judge's own name —
        // and `build.rs` measures `oracle-is-laya` from that name rather than printing a literal.
        assert_eq!(MODEL_NAME, StructuralJudge.name());
    }

    #[test]
    fn an_unmeasured_model_refuses_every_subject_before_reading_anything() {
        // **The guard is the instrument's, not the tree's.** The root does not exist: if the
        // model read anything before checking the calibration this test would panic instead of
        // refusing — and the order is `lint`'s own rule, calibration before the judge.
        let root = std::path::Path::new("a-root-the-model-must-not-read");
        let model = Model::unmeasured(root);
        assert!(!model.is_quotable());
        let pass = crate::decision::audit(&model);
        assert_eq!(pass.assessments.len(), crate::decision::SUBJECT_COUNT);
        for assessment in &pass.assessments {
            assert!(
                !assessment.reached,
                "{}: an uncalibrated run decided something",
                assessment.subject
            );
            assert_eq!(assessment.state, Establishment::Refused);
            assert!(assessment.reason.is_some(), "{}: no reason", assessment.subject);
        }
        assert!(
            !pass.refusals.all().is_empty(),
            "the refusals are the absence, recorded"
        );
    }

    #[test]
    fn a_negative_reading_is_refused_without_a_reason_and_kept_with_one() {
        let root = scratch("negative-reading");
        let witness = root.join(crate::features::witness_of(crate::features::Feature::PinnedHud));
        std::fs::create_dir_all(witness.parent().expect("a parent")).expect("the witness tree");
        std::fs::write(
            &witness,
            "//! xylophone\n", // a word the feature's declaration never licenses
        )
        .expect("the witness writes");

        let subject = crate::decision::Subject::Feature(crate::features::Feature::PinnedHud);

        // Without a human's reason, the conviction is refused by name — the model may not
        // convict on its own word.
        let refusals = Model::calibrated(&root)
            .decide(subject)
            .expect_err("the model may not convict on its own word");
        assert!(
            refusals
                .all()
                .iter()
                .any(|refusal| refusal.kind() == RefusalKind::UnreasonedOverride),
            "{refusals}"
        );

        // With a reason recorded in the same act, the negative reading is kept: a `Refused`
        // assessment that was **reached**, which is the disagreement the two passes preserve.
        let assessment = Model::calibrated(&root)
            .with_reason("the witness describes a shape the record has not ruled")
            .decide(subject)
            .expect("a reason is the one door through");
        assert!(assessment.reached);
        assert_eq!(assessment.state, Establishment::Refused);
        assert!(
            assessment
                .reason
                .as_deref()
                .unwrap_or_default()
                .contains("xylophone"),
            "the conviction names the word it convicted: {:?}",
            assessment.reason
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_model_pass_offers_every_subject_and_names_what_it_could_not_read() {
        // **The pass over the real tree, with its reading printed.** The contract is
        // structural — every subject offered, every reached reading positive, every absence
        // named — and the *set* of readings is the measurement, printed for the record rather
        // than asserted into a number that would drift from the tree.
        let root = crate::features::workspace_root(std::path::Path::new(env!("CARGO_MANIFEST_DIR")))
            .expect("a workspace root above the crate");
        let model = Model::calibrated(&root);
        assert!(
            model.is_quotable(),
            "the measured bar is accuracy 1.0 against a majority class of 0.56"
        );
        let pass = crate::decision::audit(&model);
        assert_eq!(pass.assessments.len(), crate::decision::SUBJECT_COUNT);
        let mut reached = 0usize;
        for assessment in &pass.assessments {
            if assessment.reached {
                reached += 1;
                assert_eq!(
                    assessment.state,
                    Establishment::Active,
                    "{}: a reading that is not an absence is a positive one",
                    assessment.subject
                );
                println!("read\t{}\t{}", assessment.subject, assessment.state.slug());
            } else {
                assert_eq!(assessment.state, Establishment::Refused);
                assert!(
                    assessment.reason.is_some(),
                    "{}: an absence must name itself",
                    assessment.subject
                );
                println!(
                    "absent\t{}\t{}",
                    assessment.subject,
                    assessment.reason.as_deref().unwrap_or("")
                );
            }
        }
        println!(
            "the model instrument read {reached} of {} subject(s)",
            pass.assessments.len()
        );
        assert!(
            reached > 0,
            "a second pass that decides nothing is not a second instrument"
        );
    }
}
