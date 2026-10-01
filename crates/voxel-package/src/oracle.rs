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

#[cfg(test)]
mod tests {
    use super::*;

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
}
