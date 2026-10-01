//! **The decision layer: one seam for every decision this framework makes.**
//!
//! # The ruling this module implements
//!
//! `HYPERGRILLING.md` **H-14.1**, from the user's words: *"the older agent made laya not the
//! oracle, but laya is the entire decision layer across the entire framwork includin the game's
//! systems, and the compiler itself - so it should be using the full axis swuite, all of the
//! features, making the game compile it's recipe as a package"*.
//!
//! **So the seam is here, and `oracle::Oracle` is one implementation of it** — the model
//! instrument. Before this module the tree had four decision surfaces that each answered in their
//! own code: [`crate::census`] decided five axes per crate against its own readings,
//! [`crate::features`] decided every row with `witness.is_file()`, [`crate::recipe`] decided by
//! parsing, and [`crate::oracle`] decided through a trait. Four surfaces, four vocabularies, and
//! the gate's own report said which one was Laya: **`oracle-is-laya no`**.
//!
//! # One vocabulary, and it is reused rather than invented
//!
//! The answer to a decision is [`Establishment`] — `Active`, `Inactive`, `Unchecked`, `Refused`,
//! which `features.rs` already defines and `census.rs` already reuses *for the stated reason*:
//! *"a second vocabulary for 'we did not look' is how a gate comes to read as greener than it
//! is."* So a subject that was never measured is `Unchecked` and **not** a pass, and a subject
//! that failed is `Refused` **with its reason attached**, because a refusal that names nothing is
//! a refusal nobody can act on.
//!
//! # The seam does not re-decide
//!
//! [`Deterministic`] answers by **calling** [`census::census`], [`features::evaluate`] and
//! [`recipe::compile`]/[`recipe::load`] — never by reimplementing them. A seam that re-derived an
//! answer would be a second implementation, and two implementations of one decision disagree
//! exactly when it matters. This is what makes the equality test in this module meaningful: it
//! asserts the seam and the surfaces cannot drift, because there is one implementation behind
//! both.
//!
//! # One pass, and it collects rather than stopping
//!
//! [`audit`] decides **every** subject in one traversal and returns a [`Pass`] — H-10.1's *"one
//! clean execution pass"*. A decider that cannot be reached refuses, and that refusal is
//! **collected** ([`RefusalSet`]'s rule: *"a decoder that can only report the first problem can
//! only ever be fixed one round-trip at a time"*) while the other subjects are still decided. The
//! caller decides what is fatal; the pass decides nothing by omission.

use std::path::Path;

use voxel_model::{RefusalKind, RefusalSet};

use crate::census::{self, Axis, Crate, AXES, AXIS_COUNT, CRATES, CRATE_COUNT};
use crate::features::{self, Establishment, Feature, FEATURES, FEATURE_COUNT};
use crate::recipe;

/// **What a decision can be about — the closed set.**
///
/// Closed on purpose: a decider calibrated on these subjects was not calibrated on a question
/// outside them, so a new subject is a compile-time change at every `match` rather than a string a
/// caller invents.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Subject {
    /// One cell of the crate census: one crate on one axis.
    Axis { crate_: Crate, axis: Axis },
    /// One declared feature row.
    Feature(Feature),
    /// The shipped recipe, decided by compiling it into a package and loading that package back —
    /// the whole codec, because *"a package that goes through the system validates its place in
    /// the game."*
    Recipe,
    /// One number the record carries in prose, decided by opening the document that carries it.
    ///
    /// **This arm is why `numbers` exists as a subject and not as a private module.** The first
    /// draft of `numbers` was a green island — `observe`, `rule`, `describe`, and *no caller* —
    /// which is `BLD-0023`'s shape one layer out: a check that runs beside the seam rather than
    /// through it, so no subject, no pass and no Panel can see it. Adding the arm is what puts
    /// the record's numbers on the same two-reader path as its crates and its feature rows.
    Number(&'static crate::numbers::NumberClaim),
}

/// How many axis subjects: the whole grid, every crate on every axis.
pub const AXIS_SUBJECT_COUNT: usize = CRATE_COUNT * AXIS_COUNT;

/// **How many subjects there are. A literal, as everywhere else** — so adding an axis, a feature
/// or a number without deciding how it is asked is caught here rather than at the end of a
/// report.
///
/// **80 → 86: `+ CLAIM_COUNT`.** The value is meaningless; the **type** is the assertion, and it
/// is what stopped this arm from being added without the six rows that go with it.
pub const SUBJECT_COUNT: usize = AXIS_SUBJECT_COUNT + FEATURE_COUNT + 1 + crate::numbers::CLAIM_COUNT;

/// The compile-time check that [`SUBJECT_COUNT`] and the vocabulary agree.
pub const SUBJECTS_ARE_TOTAL: [(); SUBJECT_COUNT] =
    [(); CRATE_COUNT * AXIS_COUNT + FEATURE_COUNT + 1 + crate::numbers::CLAIM_COUNT];

impl Subject {
    /// A stable, machine-readable name. **Never prose**, because a name is compared and prose is
    /// read.
    pub fn name(self) -> String {
        match self {
            Subject::Axis { crate_, axis } => {
                format!("axis/{}/{}", crate_.name(), axis.name())
            }
            Subject::Feature(feature) => format!("feature/{}", feature.name()),
            Subject::Recipe => "recipe".to_string(),
            Subject::Number(claim) => format!("number/{}", claim.name),
        }
    }
}

/// **Every subject, in one order.** The grid first, then the features, then the recipe: a reader
/// who wants to know *"did anything at all get decided"* reads the count, and a reader who wants
/// one answer looks it up by name.
pub fn subjects() -> Vec<Subject> {
    let mut out = Vec::with_capacity(SUBJECT_COUNT);
    for crate_ in CRATES {
        for axis in AXES {
            out.push(Subject::Axis { crate_, axis });
        }
    }
    for row in FEATURES {
        out.push(Subject::Feature(row.feature));
    }
    out.push(Subject::Recipe);
    // **The record's numbers, after the recipe and in declaration order** — a reader who wants
    // one answer looks it up by name, and the count beside it is how they know the set is whole.
    for claim in crate::numbers::CLAIMS {
        out.push(Subject::Number(claim));
    }
    out
}

/// One decided subject: the answer, the instrument that answered, and the reason when the answer
/// is not `Active`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Assessment {
    pub subject: String,
    pub state: Establishment,
    /// **Which instrument decided this.** The seam's whole point is that a reader can tell a
    /// measurement from a proposal, so no assessment is anonymous.
    pub instrument: &'static str,
    /// **Whether the instrument reached a decision at all.** `false` means the decider could not
    /// answer — *a missing instrument is not a pass* — and that is a different fact from an
    /// instrument that answered `Refused`. The two passes need the difference: an absence is not a
    /// disagreement.
    pub reached: bool,
    /// Never empty unless the state is `Active` — the same rule [`crate::features::FeatureState`]
    /// follows, for the same reason.
    pub reason: Option<String>,
}

// ---- the rules, and they live here rather than in the readers ----------------------------
//
// **The inversion.** `features::observe` reports which witnesses are on disk and
// `census::observe` reports what it read; *what those facts mean* is decided here, once, so a
// second instrument implementing this seam can be handed the same observations and answer its own
// way. Before this, each reader decided in its own code and the layer could only front the answer.

/// **What a feature row means.** The rule `features::evaluate` used to hold itself.
pub fn rule_for_feature(observation: &crate::features::FeatureObservation) -> (Establishment, String) {
    if observation.present {
        (Establishment::Active, format!("witness present at {}", observation.witness))
    } else {
        (
            Establishment::Unchecked,
            format!(
                "no witness at {}, so {} is not implemented; {}",
                observation.witness,
                observation.feature.name(),
                if observation.required_for_alpha {
                    "it is required for the alpha, so the build refuses"
                } else {
                    "it is pending, so it is displayed and the build continues"
                }
            ),
        )
    }
}

/// **What one crate-on-one-axis observation means.**
///
/// Every arm reproduces the reason text `census::measure` wrote, **verbatim** — the gate's report
/// lines are the record's own words and an inversion is not licence to rephrase them.
/// **The rule for a number the record carries in prose, and where it lives here rather than in
/// the reader.**
///
/// [`crate::numbers`] reports `{readable, line}` and nothing else; **this** decides what that
/// means. The split is C-84.2's — *"the readers report; the layer decides"* — and it is a
/// correction rather than a style: the first draft of `numbers` returned
/// `Reading::{Agrees, Stale, Absent}` **from the reader**, which is three verdicts taken by the
/// thing that opened the file. **A reader that decides cannot be re-decided by a second
/// instrument**, which is the whole reason `decision.rs` exists.
///
/// Four outcomes, and they are four different facts:
///
/// | observation | state | why |
/// |---|---|---|
/// | the document could not be read | **`Refused`** | nothing was measured — D107 |
/// | the sentence is gone | **`Refused`** | the row names prose nobody wrote, so the row is the defect |
/// | the line carries the expected figure | **`Active`** | the document agrees with the record |
/// | the row *declares* the document stale | **`Inactive`** | a known disagreement, reported every build, **not** a build failure |
///
/// **The last row is the one that matters, and it is C-38's practice as a type.** C-34 and C-38
/// both left a stale figure standing in a document the user owns — *"stale in the evidence and
/// untouched in the document"* — and recorded it here instead. **Correcting reserved-area prose
/// is the user's hand**; the layer's job is to name the disagreement in those words, every
/// build, forever, rather than rewriting a document it does not own.
pub fn rule_for_number(
    observation: &crate::numbers::NumberObservation,
) -> (Establishment, Option<String>) {
    let claim = observation.claim;
    if !observation.readable {
        return (
            Establishment::Refused,
            Some(format!(
                "{}: {} could not be read ({}), so this number was not measured. It is neither \
                 agreement nor disagreement — it is the absence of a measurement, and a gate that \
                 counts it as agreement is a gate that is not looking.",
                claim.name,
                claim.document,
                observation.unreadable.as_deref().unwrap_or("no reason given")
            )),
        );
    }
    let Some(line) = &observation.line else {
        return (
            Establishment::Refused,
            Some(format!(
                "{}: the sentence {:?} is not in {}. Either the prose was reworded, in which case \
                 this row must be updated deliberately, or the figure was deleted — and a row \
                 that names a sentence nobody wrote is a row nothing checks.",
                claim.name, claim.sentence, claim.document
            )),
        );
    };
    // **A row that declares the document stale expects it to disagree.** The sentence being
    // present *is* the check; the figure is quoted beside it so a reader sees both numbers.
    if claim.is_stale() {
        return (
            Establishment::Inactive,
            Some(format!(
                "{} still reads {line:?} in {}. The record's figure is {}, and the prose is not \
                 edited here: C-2 is stamped OPEN TO CORRECTION and appearance is the user's area \
                 (C-27, C-38), so the correction is theirs to make. This line prints on every \
                 build until they make it.",
                claim.name,
                claim.document,
                claim.expected(),
            )),
        );
    }
    if line.contains(claim.expects.unwrap_or_default()) {
        return (Establishment::Active, None);
    }
    (
        Establishment::Refused,
        Some(format!(
            "{}: {} carries the sentence but not the figure it is required to carry. The row \
             expects {:?} and the document says {line:?}. A number that drifted from its \
             declaration is the defect this table exists to catch, and it is not caught by \
             asserting a constant against another constant.",
            claim.name,
            claim.document,
            claim.expects.unwrap_or_default(),
        )),
    )
}

pub fn rule_for_axis(
    observation: &crate::census::CrateObservation,
    axis: Axis,
) -> (Establishment, Option<String>) {
    match axis {
        Axis::Named => match observation.named_by_record {
            Some(true) => (Establishment::Active, None),
            Some(false) => (
                Establishment::Refused,
                Some(format!(
                    "no document of the record names {}. A crate the record does not mention has \
                     no declared reason to exist, and a gate that passes it anyway is a gate that \
                     is not looking at the whole repository",
                    observation.crate_.name()
                )),
            ),
            None => (
                Establishment::Refused,
                Some(
                    "the record could not be read, so whether this crate is named is unknown. \
                     Unnamed is not claimed"
                        .to_string(),
                ),
            ),
        },
        Axis::Moduled | Axis::Tested | Axis::Refusing => match observation.sources {
            Some(facts) => {
                let (held, absent) = match axis {
                    Axis::Moduled => (facts.moduled, "declares no `pub mod`"),
                    Axis::Tested => (facts.tested, "contains no `#[test]`"),
                    _ => (facts.refusing, "names no RefusalKind"),
                };
                if held {
                    return (Establishment::Active, None);
                }
                // The three absent texts, each with the record's own reason for *why* it is a
                // state rather than a failure where that is what it is.
                let why = match axis {
                    Axis::Moduled => format!(
                        "{} {absent}. That is legal — a leaf crate can be one file — so this is \
                         reported as `inactive`, not refused: a state, and a fact a reader can \
                         check",
                        observation.crate_.name()
                    ),
                    Axis::Tested => format!(
                        "{} {absent}. C-17 holds the whole client to 'every cargo test actually \
                         visible', and a crate with no test cannot show that it was looked at",
                        observation.crate_.name()
                    ),
                    _ => format!(
                        "{} {absent}. A crate that cannot refuse returns a plausible value where \
                         the record says it should decline; whether this one should is a question \
                         the record has not answered, so none is asserted here",
                        observation.crate_.name()
                    ),
                };
                (Establishment::Inactive, Some(why))
            }
            None => (
                Establishment::Refused,
                Some(format!(
                    "{} has no readable source, so {} could not be measured at all",
                    observation.crate_.name(),
                    axis.name()
                )),
            ),
        },
        Axis::Claimed => match &observation.claim {
            Some(claim) if claim.claimed => {
                let why = match (&claim.witness, claim.line) {
                    (Some(path), Some(line)) => format!("claimed at {path}:{line}"),
                    _ => "claimed, with no locator, which is a claim nobody can check".to_string(),
                };
                (Establishment::Active, Some(why))
            }
            Some(claim) => (Establishment::Inactive, Some(claim.why_unclaimed.clone())),
            None => (
                Establishment::Refused,
                Some(format!(
                    "{} was not measured against the spec at all. An unmeasured cell is not an \
                     unclaimed one",
                    observation.crate_.name()
                )),
            ),
        },
    }
}

impl Assessment {
    /// Whether this assessment says the decision was checked and holds.
    pub fn holds(&self) -> bool {
        self.state == Establishment::Active
    }
}

/// **The seam.** Everything that decides asks this, and nothing decides by asking something else.
///
/// `Err` means **the instrument could not be reached**, which is a refusal and never a pass — the
/// rule `oracle.rs` states as *"a missing instrument is not a pass"*. A decision that was reached
/// and is *no* comes back `Ok` with [`Establishment::Refused`], because a decider that answers
/// "no" has done its job and a decider that cannot answer has not.
pub trait Decider {
    /// Which instrument this is, for the record.
    fn name(&self) -> &'static str;

    /// Decide one subject.
    fn decide(&self, subject: Subject) -> Result<Assessment, RefusalSet>;
}

/// The in-project, deterministic decision layer.
///
/// **It answers by delegating**, so this is a front and not a second implementation: the axes come
/// from [`census::census`], the features from [`features::evaluate`], the recipe from
/// [`recipe::compile`] followed by [`recipe::load`].
pub struct Deterministic<'a> {
    root: &'a Path,
    /// The recipe source, when the caller has one. Absent is a *refusal* for the recipe subject
    /// rather than a silent `Active`: a decider that was handed no source has decided nothing.
    recipe: Option<&'a [u8]>,
}

impl<'a> Deterministic<'a> {
    /// The decider over a workspace, with every subject available except the recipe's source.
    pub fn new(root: &'a Path) -> Self {
        Deterministic { root, recipe: None }
    }

    /// Hand it the recipe's source, so [`Subject::Recipe`] can be decided.
    pub fn with_recipe(mut self, source: &'a [u8]) -> Self {
        self.recipe = Some(source);
        self
    }
}

impl Decider for Deterministic<'_> {
    fn name(&self) -> &'static str {
        "deterministic"
    }

    fn decide(&self, subject: Subject) -> Result<Assessment, RefusalSet> {
        match subject {
            Subject::Axis { crate_, axis } => {
                // **Read, then rule.** `census::observe` reports and `rule_for_axis` decides;
                // going through `census::census` here would be the same pair already applied and
                // would hide where the rule lives.
                let observations = census::observe(self.root)?;
                let assessment = match observations.iter().find(|observed| observed.crate_ == crate_)
                {
                    Some(observed) => {
                        let (state, reason) = rule_for_axis(observed, axis);
                        Assessment {
                            subject: subject.name(),
                            state,
                            instrument: self.name(),
                            reached: true,
                            reason,
                        }
                    }
                    // **An unmeasured crate is not a crate that passed.** The census returned, and
                    // this crate is not in it, so the honest state is `Unchecked` with the fact
                    // named — not a default of `Active`.
                    None => Assessment {
                        subject: subject.name(),
                        state: Establishment::Unchecked,
                        instrument: self.name(),
                        reached: true,
                        reason: Some(format!(
                            "the census returned no observation for {}, and a crate that was not \
                             measured is not a crate that passed",
                            crate_.name()
                        )),
                    },
                };
                Ok(assessment)
            }
            Subject::Feature(feature) => {
                // The same pair as the axis arm: the row's witness is **read** by
                // `features::observe` and **ruled on** by `rule_for_feature`.
                let observations = features::observe(self.root);
                let assessment =
                    match observations.iter().find(|observed| observed.feature == feature) {
                        Some(observed) => {
                            let (state, reason) = rule_for_feature(observed);
                            Assessment {
                                subject: subject.name(),
                                state,
                                instrument: self.name(),
                                reached: true,
                                reason: Some(reason),
                            }
                        }
                        None => Assessment {
                            subject: subject.name(),
                            state: Establishment::Unchecked,
                            instrument: self.name(),
                            reached: true,
                            reason: Some(format!(
                                "the feature table returned no row for {}, so nothing was \
                                 evaluated",
                                feature.name()
                            )),
                        },
                    };
                Ok(assessment)
            }
            Subject::Number(claim) => {
                // **Read, then rule — the same pair as the other two arms.** `numbers::observe`
                // opens the document and reports `{readable, line}`; `rule_for_number` says what
                // that means. Going through `numbers::check_numbers` here would re-run the rule
                // behind the seam and hide where the decision lives, which is the thing
                // `the_seam_answers_what_the_surfaces_answer` exists to prevent.
                let observations = crate::numbers::observe(self.root);
                let assessment = match observations.iter().find(|observed| observed.claim.name == claim.name) {
                    Some(observed) => {
                        let (state, reason) = rule_for_number(observed);
                        Assessment {
                            subject: subject.name(),
                            state,
                            instrument: self.name(),
                            reached: true,
                            reason,
                        }
                    }
                    None => Assessment {
                        subject: subject.name(),
                        state: Establishment::Unchecked,
                        instrument: self.name(),
                        reached: true,
                        reason: Some(format!(
                            "the numbers table returned no row for {}, so nothing was evaluated",
                            claim.name
                        )),
                    },
                };
                Ok(assessment)
            }
            Subject::Recipe => {
                let Some(source) = self.recipe else {
                    let mut refusals = RefusalSet::new();
                    refusals.refuse(
                        RefusalKind::NoReading,
                        "the decision layer was handed no recipe source, so the recipe subject was \
                         not decided. A decider with nothing to read has no reading",
                    );
                    return Err(refusals);
                };
                // **Compile, then load** — the whole codec, because a package that does not load
                // back has not validated its place in the game (H-10.1).
                match recipe::compile(source) {
                    Err(refusals) => Ok(Assessment {
                        subject: subject.name(),
                        state: Establishment::Refused,
                        instrument: self.name(),
                        reached: true,
                        reason: Some(format!("the recipe did not compile: {refusals}")),
                    }),
                    Ok(package) => match recipe::load(&package) {
                        Err(refusals) => Ok(Assessment {
                            subject: subject.name(),
                            state: Establishment::Refused,
                            instrument: self.name(),
                            reached: true,
                            reason: Some(format!(
                                "the compiled package did not load back: {refusals}"
                            )),
                        }),
                        Ok(loaded) => {
                            // The round trip is the decision: a package that loads but does not
                            // carry the source it was compiled from has not validated anything.
                            let carries = loaded.source == source;
                            Ok(Assessment {
                                subject: subject.name(),
                                state: if carries {
                                    Establishment::Active
                                } else {
                                    Establishment::Refused
                                },
                                instrument: self.name(),
                                reached: true,
                                reason: if carries {
                                    None
                                } else {
                                    Some(
                                        "the package loaded, and it does not carry the exact \
                                         recipe source it was compiled from"
                                            .to_string(),
                                    )
                                },
                            })
                        }
                    },
                }
            }
        }
    }
}

/// A decider that cannot be reached. **Refuses every subject, on purpose.**
///
/// Its purpose is the same as [`crate::oracle::UnreachableOracle`]'s: it makes *"the decision layer
/// was skipped"* impossible to mistake for *"the decision layer passed"*.
pub struct Unreachable {
    pub why: String,
}

impl Decider for Unreachable {
    fn name(&self) -> &'static str {
        "unreachable"
    }

    fn decide(&self, subject: Subject) -> Result<Assessment, RefusalSet> {
        Err(one_refusal(&format!(
            "{} could not be decided: {}",
            subject.name(),
            self.why
        )))
    }
}

/// **One pass over every subject.** Returns a [`Pass`] and never stops early.
pub fn audit(decider: &dyn Decider) -> Pass {
    let mut assessments = Vec::with_capacity(SUBJECT_COUNT);
    let mut refusals = RefusalSet::new();
    for subject in subjects() {
        match decider.decide(subject) {
            Ok(assessment) => assessments.push(assessment),
            // **The subject is still assessed**, and its assessment is `Refused` with the
            // instrument's own words — so a subject nobody could decide is visible in the pass
            // instead of being an absence a reader has to notice.
            Err(found) => {
                for refusal in found.all() {
                    refusals.push(voxel_model::Refusal::owned(refusal.kind(), refusal.detail()));
                }
                assessments.push(Assessment {
                    subject: subject.name(),
                    state: Establishment::Refused,
                    instrument: decider.name(),
                    reached: false,
                    reason: Some(format!("the instrument could not decide this: {found}"))
                });
            }
        }
    }
    Pass { decider: decider.name(), assessments, refusals }
}

/// One refusal, collected the same way [`RefusalSet`] collects.
fn one_refusal(detail: &str) -> RefusalSet {
    let mut refusals = RefusalSet::new();
    refusals.refuse(RefusalKind::NoReading, detail);
    refusals
}

/// What one pass decided.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pass {
    /// The instrument that ran. **Recorded**, because a pass with no instrument is not a pass.
    pub decider: &'static str,
    pub assessments: Vec<Assessment>,
    /// Every refusal the instruments met, collected rather than returned at the first one.
    pub refusals: RefusalSet,
}

/// **One subject the two passes answered differently.**
///
/// Preserved rather than synthesized: hyperskill's own rule is that *"a disagreement is preserved
/// and adjudicated rather than silently synthesized"*, and a seam that averaged two instruments
/// would be a third instrument nobody calibrated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Divergence {
    pub subject: String,
    pub system: Establishment,
    pub model: Establishment,
    pub system_reason: Option<String>,
    pub model_reason: Option<String>,
}

/// **Two passes over one subject vocabulary** — the Rust system and the model instrument (H-14.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TwoPass {
    pub system: Pass,
    pub model: Pass,
}

impl TwoPass {
    /// Every subject both instruments **reached** and answered differently.
    ///
    /// **An absence is not a disagreement.** A subject the model never reached is not a place the
    /// two disagree; it is a place only one of them looked, and `refusal.rs`'s rule is that a
    /// recorded absence is a distinct, renderable state. So this compares only answered subjects,
    /// and the model's unreached subjects are counted separately.
    pub fn divergences(&self) -> Vec<Divergence> {
        let mut out = Vec::new();
        for assessment in &self.system.assessments {
            if !assessment.reached {
                continue;
            }
            let Some(model) = self
                .model
                .assessments
                .iter()
                .find(|candidate| candidate.subject == assessment.subject)
            else {
                continue;
            };
            if !model.reached || model.state == assessment.state {
                continue;
            }
            out.push(Divergence {
                subject: assessment.subject.clone(),
                system: assessment.state,
                model: model.state,
                system_reason: assessment.reason.clone(),
                model_reason: model.reason.clone(),
            });
        }
        out
    }

    /// How many subjects the model instrument actually answered.
    pub fn model_decided(&self) -> usize {
        self.model.assessments.iter().filter(|assessment| assessment.reached).count()
    }

    /// **Both passes, then every disagreement**, as the gate's own lines.
    pub fn report(&self) -> String {
        let mut out = self.system.report();
        out.push_str("\n# the decision layer, pass two (the model instrument)\n");
        out.push_str(&format!("model-decider\t{}\n", self.model.decider));
        out.push_str(&format!("model-decided\t{}\n", self.model_decided()));
        out.push_str(&format!(
            "model-unreached\t{}\n",
            self.model.assessments.len() - self.model_decided()
        ));
        let divergences = self.divergences();
        out.push_str(&format!("divergences\t{}\n", divergences.len()));
        for divergence in &divergences {
            out.push_str(&format!(
                "divergence\t{}\tsystem={}\tmodel={}\n",
                divergence.subject,
                divergence.system.slug(),
                divergence.model.slug()
            ));
        }
        out
    }
}

/// **Run both passes over the same subjects.** One traversal each, one vocabulary, one report.
pub fn run(system: &dyn Decider, model: &dyn Decider) -> TwoPass {
    TwoPass { system: audit(system), model: audit(model) }
}

impl Pass {
    /// Every assessment with the given state.
    pub fn in_state(&self, state: Establishment) -> Vec<&Assessment> {
        self.assessments.iter().filter(|assessment| assessment.state == state).collect()
    }

    /// One subject's assessment, by name.
    pub fn of(&self, subject: Subject) -> Option<&Assessment> {
        let name = subject.name();
        self.assessments.iter().find(|assessment| assessment.subject == name)
    }

    /// How many subjects hold.
    pub fn holding(&self) -> usize {
        self.in_state(Establishment::Active).len()
    }

    /// **The pass, as the gate's own lines.** One section, so a reader of the report sees what the
    /// decision layer decided rather than having to infer it from four sections that each decided
    /// something.
    pub fn report(&self) -> String {
        let mut out = String::new();
        out.push_str("\n# the decision layer\n");
        out.push_str(&format!("decider\t{}\n", self.decider));
        out.push_str(&format!("subjects\t{}\n", self.assessments.len()));
        for state in [
            Establishment::Active,
            Establishment::Inactive,
            Establishment::Unchecked,
            Establishment::Refused,
        ] {
            out.push_str(&format!("subject-{}\t{}\n", state.slug(), self.in_state(state).len()));
        }
        // Every subject that does not hold, with its reason — the report a reader actually wants,
        // because "which decision is not settled" is the question and a count is not the answer.
        for assessment in &self.assessments {
            if assessment.holds() {
                continue;
            }
            let reason = assessment.reason.as_deref().unwrap_or("no reason recorded");
            out.push_str(&format!(
                "subject\t{}\t{}\t{}\n",
                assessment.subject,
                assessment.state.slug(),
                reason
            ));
        }
        out
    }
}

// ============================================================================
// Reading a report back
// ============================================================================
//
// **Why a reader at all, and why it lives here.** The build writes the pass into
// `alpha-shell.gate` and `crates/shploingus/src/main.rs` embeds those bytes; UI-SPEC §9 and
// C-53.3 rule that *the same findings are a record the client renders, so a green build and a
// legible panel cannot disagree*. A client that re-derived the answers would be a second run
// that can drift, and C-78.6 already refuses that shape outright — *"parsing the gate's text back
// out would not deliver 'cannot disagree.'"* So the gate's text is read, **by the module that
// writes it**, and `the_report_reads_back_what_the_report_wrote` pins the pair.
//
// **It is not a `Pass`, and it does not pretend to be one.** A report carries the counts, the
// instrument and the reason on every subject that does not hold. It does **not** carry
// `Assessment::instrument` per row, nor `reached` — so a reader that reconstructed a `Pass` would
// be inventing two fields the text never wrote. These types carry exactly what the text holds.

/// One subject the report says does not hold, with the reason it gives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReportedSubject {
    pub subject: String,
    pub state: Establishment,
    pub reason: String,
}

/// What `# the decision layer` said.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReportedPass {
    pub decider: String,
    pub subjects: usize,
    pub active: usize,
    pub inactive: usize,
    pub unchecked: usize,
    pub refused: usize,
    /// **Every subject that does not hold, with its reason** — C-83.5's rule, carried: *"which
    /// decision is not settled* is the question and a count is not the answer".
    pub pending: Vec<ReportedSubject>,
}

impl ReportedPass {
    /// How many hold, by the report's own count rather than by counting the pending rows.
    pub fn holding(&self) -> usize {
        self.active
    }
}

/// What `# the decision layer, pass two` said.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReportedTwo {
    pub model_decider: String,
    pub model_decided: usize,
    pub model_unreached: usize,
    pub divergences: usize,
    /// Each divergence's line, verbatim. **Preserved rather than synthesized** (C-84.3), and kept
    /// as text because a report line is what the writer wrote and a re-typed row could differ.
    pub divergence_rows: Vec<String>,
}

/// The header the first section carries, and the one the second does.
const PASS_HEADER: &str = "# the decision layer";
const TWO_HEADER: &str = "# the decision layer, pass two (the model instrument)";

/// Read the decision layer's two passes out of a gate report.
///
/// **Collects every refusal rather than returning the first**, for the reason `refusal.rs` gives:
/// a decoder that reports one problem can only be fixed one round trip at a time.
///
/// **A missing section is a refusal, never a zero.** D107's rule in its fourth home (`census`,
/// `bloat`, `projection`, `provenance`): four zeros beside each other read as a measurement, and a
/// report with no decision-layer section is *not* a measurement of a project with no decisions.
pub fn read_report(text: &str) -> Result<(ReportedPass, ReportedTwo), RefusalSet> {
    let mut refusals = RefusalSet::new();

    let Some(two_start) = find_line(text, TWO_HEADER) else {
        refusals.refuse(
            RefusalKind::NoReading,
            format!("the text carries no {TWO_HEADER:?} section, so the model's pass is not \
                     measured rather than empty"),
        );
        return Err(refusals);
    };
    let Some(pass_start) = find_line(text, PASS_HEADER) else {
        refusals.refuse(
            RefusalKind::NoReading,
            format!("the text carries no {PASS_HEADER:?} section, so a count of zero here would \
                     read as a measurement of nothing"),
        );
        return Err(refusals);
    };
    // `PASS_HEADER` is a prefix of `TWO_HEADER`, so the first section ends where the second
    // begins. Measured, not assumed: an ordering the writer controls in one place.
    if two_start < pass_start {
        refusals.refuse(
            RefusalKind::CorruptRecord,
            format!(
                "the pass-two section ({TWO_HEADER:?}) appears before the pass-one section \
                 ({PASS_HEADER:?}), which is not the order `report` writes"
            ),
        );
        return Err(refusals);
    }

    let mut pass = ReportedPass {
        decider: String::new(),
        subjects: 0,
        active: 0,
        inactive: 0,
        unchecked: 0,
        refused: 0,
        pending: Vec::new(),
    };
    let mut two = ReportedTwo {
        model_decider: String::new(),
        model_decided: 0,
        model_unreached: 0,
        divergences: 0,
        divergence_rows: Vec::new(),
    };
    let mut seen_decider = false;
    let mut seen_model_decider = false;

    for line in lines_between(text, pass_start, two_start) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let cells: Vec<&str> = line.split('\t').collect();
        match cells[0] {
            "decider" => {
                pass.decider = cells.get(1).copied().unwrap_or_default().to_string();
                seen_decider = true;
            }
            "subjects" => pass.subjects = number(&mut refusals, &cells, 1, line),
            "subject-active" => pass.active = number(&mut refusals, &cells, 1, line),
            "subject-inactive" => pass.inactive = number(&mut refusals, &cells, 1, line),
            "subject-unchecked" => pass.unchecked = number(&mut refusals, &cells, 1, line),
            "subject-refused" => pass.refused = number(&mut refusals, &cells, 1, line),
            "subject" => {
                let Some(state) = cells.get(2).and_then(|word| state_of(word)) else {
                    refusals.refuse(
                        RefusalKind::NoReading,
                        format!(
                            "subject {:?} carries state {:?}, which is not one of the four the \
                             decision layer reports",
                            cells.get(1).copied().unwrap_or(""),
                            cells.get(2).copied().unwrap_or("")
                        ),
                    );
                    continue;
                };
                let subject = cells.get(1).copied().unwrap_or_default().to_string();
                // **A reason is never defaulted to a blank.** The writer substitutes its own
                // words when a subject carries none; a reader that accepted an empty third cell
                // would render a refusal with no reason beside a refused state.
                let reason = match cells.get(3) {
                    Some(reason) if !reason.trim().is_empty() => reason.to_string(),
                    _ => {
                        refusals.refuse(
                            RefusalKind::NoReading,
                            format!(
                                "subject {subject:?} is {} with no reason beside it, and a \
                                 refusal nobody can read is a refusal nobody can fix",
                                state.slug()
                            ),
                        );
                        String::new()
                    }
                };
                pass.pending.push(ReportedSubject { subject, state, reason });
            }
            other => {
                refusals.refuse(
                    RefusalKind::FabricatedFeature,
                    format!(
                        "the decision layer's report carries a key {other:?} that \
                         `Pass::report` never writes. Refused rather than skipped: a key this \
                         reader does not know is a second report format wearing this one's \
                         section header"
                    ),
                );
            }
        }
    }

    for line in lines_between(text, two_start, text.len()) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let cells: Vec<&str> = line.split('\t').collect();
        match cells[0] {
            "model-decider" => {
                two.model_decider = cells.get(1).copied().unwrap_or_default().to_string();
                seen_model_decider = true;
            }
            "model-decided" => two.model_decided = number(&mut refusals, &cells, 1, line),
            "model-unreached" => two.model_unreached = number(&mut refusals, &cells, 1, line),
            "divergences" => two.divergences = number(&mut refusals, &cells, 1, line),
            "divergence" => two.divergence_rows.push(line.to_string()),
            other => {
                refusals.refuse(
                    RefusalKind::FabricatedFeature,
                    format!(
                        "the model pass's report carries a key {other:?} that \
                         `TwoPass::report` never writes"
                    ),
                );
            }
        }
    }

    // **A pass with no instrument is not a pass** (C-84.3's shape), and a reader that defaulted
    // the field would print `decider` with nothing after it.
    if !seen_decider {
        refusals.refuse(
            RefusalKind::NoReading,
            "the decision layer's report names no instrument, so nothing was measured",
        );
    }
    if !seen_model_decider {
        refusals.refuse(
            RefusalKind::NoReading,
            "the model pass names no instrument, so the second pass is not measured",
        );
    }
    // **The counts must add up to what they count.** Four numbers beside a fifth that does not
    // contain them is a report that cannot be rendered without two of them disagreeing, and the
    // anti-fabrication guard is the same one `features::check_features` runs on the table.
    let summed = pass.active + pass.inactive + pass.unchecked + pass.refused;
    if summed != pass.subjects {
        refusals.refuse(
            RefusalKind::DigestMismatch,
            format!(
                "the report counts {} subject(s) and its four states sum to {summed}, so the \
                 numbers cannot both be true",
                pass.subjects
            ),
        );
    }
    // **A divergence the report counts but does not print is a claim with no witness**, which is
    // the same refusal in the other direction.
    if two.divergence_rows.len() != two.divergences {
        refusals.refuse(
            RefusalKind::DigestMismatch,
            format!(
                "the report counts {} divergence(s) and prints {} row(s) of them",
                two.divergences,
                two.divergence_rows.len()
            ),
        );
    }
    // **Every subject that does not hold is printed**, so a pending row the reader cannot see is
    // a decision nobody is shown is unsettled.
    let printed = pass.subjects.saturating_sub(pass.active);
    if pass.pending.len() != printed {
        refusals.refuse(
            RefusalKind::DigestMismatch,
            format!(
                "{} subject(s) do not hold and the report names {} of them, so the client would \
                 render a count the reader cannot check",
                printed,
                pass.pending.len()
            ),
        );
    }

    refusals.into_result((pass, two))
}

/// The state a report word names, and `None` for a word the four states do not cover.
fn state_of(word: &str) -> Option<Establishment> {
    match word {
        "active" => Some(Establishment::Active),
        "inactive" => Some(Establishment::Inactive),
        "unchecked" => Some(Establishment::Unchecked),
        "refused" => Some(Establishment::Refused),
        _ => None,
    }
}

/// One count, refused rather than defaulted to zero.
fn number(refusals: &mut RefusalSet, cells: &[&str], index: usize, line: &str) -> usize {
    match cells.get(index).and_then(|cell| cell.parse::<usize>().ok()) {
        Some(value) => value,
        None => {
            refusals.refuse(
                RefusalKind::NoReading,
                format!("{line:?} does not carry a count at position {index}"),
            );
            0
        }
    }
}

/// The line number of `header`, searching for the whole line so a prefix cannot match the
/// pass-two header when the pass-one one is wanted.
fn find_line(text: &str, header: &str) -> Option<usize> {
    text.lines().position(|line| line.trim_end() == header)
}

/// The lines between two line numbers, exclusive of both.
fn lines_between(text: &str, from: usize, to: usize) -> impl Iterator<Item = &str> {
    text.lines().skip(from + 1).take(to.saturating_sub(from + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> std::path::PathBuf {
        features::workspace_root(Path::new(env!("CARGO_MANIFEST_DIR")))
            .expect("a workspace root above the crate")
    }

    fn recipe_source() -> Vec<u8> {
        std::fs::read(root().join("packages/alpha-shell.shspec")).expect("the shipped recipe")
    }

    fn shipped() -> Deterministic<'static> {
        // Leaked on purpose: the decider borrows its root and its source, and a test's decider
        // outlives its own scope only through `Box::leak`. One allocation per test is cheaper than
        // an owned decider nobody else needs.
        let root: &'static Path = Box::leak(root().into_boxed_path());
        let source: &'static [u8] = Box::leak(recipe_source().into_boxed_slice());
        Deterministic::new(root).with_recipe(source)
    }

    #[test]
    fn the_report_reads_back_exactly_what_the_report_wrote() {
        // **The round trip that `read_report` exists for.** UI-SPEC §9's rule is that a green
        // build and a legible panel cannot disagree; a reader that could drop or invent a row is
        // how they would. So this runs the real pass over the real tree, writes it, reads it, and
        // compares — every count, and every subject that does not hold with its reason.
        let two = run(&shipped(), &Unreachable { why: "the model is not built yet".into() });
        let (read_pass, read_model) = read_report(&two.report()).expect("the report reads back");

        assert_eq!(read_pass.decider, two.system.decider);
        assert_eq!(read_pass.subjects, two.system.assessments.len());
        // **The count is pinned as well as compared.** A round trip alone would agree with any
        // number the writer chose, so it cannot catch the count drifting; C-59.7's rule is that a
        // count is a literal somebody decided, and `SUBJECT_COUNT` is that literal's type. This
        // tree reads **86** today, not the 80 C-83 recorded, because `numbers::CLAIM_COUNT` is
        // inside the vocabulary — and a test that had asserted `80` would be reporting a stale
        // number as though it were the gate's.
        assert_eq!(
            read_pass.subjects,
            SUBJECT_COUNT,
            "the report and the subject vocabulary agree, and the vocabulary is measured"
        );
        assert_eq!(read_pass.active, two.system.in_state(Establishment::Active).len());
        assert_eq!(read_pass.inactive, two.system.in_state(Establishment::Inactive).len());
        assert_eq!(read_pass.unchecked, two.system.in_state(Establishment::Unchecked).len());
        assert_eq!(read_pass.refused, two.system.in_state(Establishment::Refused).len());
        assert_eq!(read_pass.holding(), two.system.holding());

        let mut expected: Vec<(String, Establishment, String)> = two
            .system
            .assessments
            .iter()
            .filter(|assessment| !assessment.holds())
            .map(|assessment| {
                (
                    assessment.subject.clone(),
                    assessment.state,
                    assessment
                        .reason
                        .clone()
                        .unwrap_or_else(|| "no reason recorded".to_string()),
                )
            })
            .collect();
        let mut read: Vec<(String, Establishment, String)> = read_pass
            .pending
            .iter()
            .map(|row| (row.subject.clone(), row.state, row.reason.clone()))
            .collect();
        expected.sort();
        read.sort();
        assert_eq!(read, expected, "every unsettled subject, with the reason beside it");
        assert!(!read.is_empty(), "this tree has unsettled subjects, so the rows are not vacuous");

        // And the model pass, which today reaches nothing — an instrument that is absent rather
        // than an instrument that agreed.
        assert_eq!(read_model.model_decider, two.model.decider);
        assert_eq!(read_model.model_decided, two.model_decided());
        assert_eq!(read_model.divergences, two.divergences().len());
        assert_eq!(
            read_model.model_decided + read_model.model_unreached,
            two.model.assessments.len(),
            "the model's decided and unreached counts partition every subject"
        );
    }

    #[test]
    fn a_key_the_report_never_writes_is_refused_by_name_rather_than_skipped() {
        // **The guard with teeth.** A reader that skipped an unknown key would silently drop a
        // section added later, and a client rendering a stale pass would look exactly like a
        // client rendering the right one.
        let two = run(&shipped(), &Unreachable { why: "the model is not built yet".into() });
        let tampered = two
            .report()
            .replace("subjects\t", "subjects-weighted\t7\\nsubjects\t");
        let refused = read_report(&tampered).expect_err("a key the writer never emits");
        assert!(
            refused.all().iter().any(|refusal| {
                refusal.detail().contains("subjects-weighted")
                    && refusal.detail().contains("never writes")
            }),
            "the refusal names the key and says why: {refused}"
        );
    }

    #[test]
    fn a_report_with_no_decision_layer_is_refused_rather_than_read_as_four_zeros() {
        // D107 in its fifth home: absence rendered as absence. Four zeros beside each other read
        // as a measurement of a project with no decisions.
        let refused = read_report("oracle-kind\tstructural-deterministic\n").expect_err("no layer");
        assert!(
            refused.all().iter().any(|r| r.detail().contains("decision layer")),
            "{refused}"
        );
        assert!(
            !refused.all().iter().any(|r| r.detail().contains("passed")),
            "and it never reports a pass: {refused}"
        );
    }

    #[test]
    fn a_count_that_does_not_add_up_to_what_it_counts_is_refused() {
        // Four numbers beside a fifth that does not contain them is a report that cannot be
        // rendered without two of them disagreeing on screen.
        let text = "\
# the decision layer
decider\tdeterministic
subjects\t9
subject-active\t1
subject-inactive\t1
subject-unchecked\t1
subject-refused\t1
subject\ta/active-state\tunchecked\tno witness
subject\tb/other\tunchecked\tno witness
subject\tc/other\tunchecked\tno witness

# the decision layer, pass two (the model instrument)
model-decider\tunreachable
model-decided\t0
model-unreached\t9
divergences\t0
";
        let refused = read_report(text).expect_err("1 + 1 + 1 + 1 is not 4");
        assert!(
            refused.all().iter().any(|r| r.detail().contains("sum to 4")),
            "{refused}"
        );
    }

    #[test]
    fn a_subject_printed_without_a_reason_is_refused() {
        // The writer substitutes its own words when a subject carries none, so a bare three-cell
        // row is something this reader did not write — and rendering it would put a refusal on
        // screen with no reason beside it.
        let text = "\
# the decision layer
decider\tdeterministic
subjects\t2
subject-active\t1
subject-inactive\t0
subject-unchecked\t1
subject-refused\t0
subject\ta/unchecked-state\tunchecked

# the decision layer, pass two (the model instrument)
model-decider\tunreachable
model-decided\t0
model-unreached\t2
divergences\t0
";
        let refused = read_report(text).expect_err("a reason is not optional");
        assert!(
            refused.all().iter().any(|r| r.detail().contains("no reason beside it")),
            "{refused}"
        );
    }

    #[test]
    fn every_subject_gets_a_decision_from_the_seam() {
        let pass = audit(&shipped());
        assert_eq!(
            pass.assessments.len(),
            SUBJECT_COUNT,
            "**every subject is decided in one pass** — {} subjects, {} assessments",
            SUBJECT_COUNT,
            pass.assessments.len()
        );
        for assessment in &pass.assessments {
            assert_eq!(
                assessment.instrument, "deterministic",
                "{} carries no instrument, so a reader cannot tell a measurement from a proposal",
                assessment.subject
            );
            assert!(
                assessment.holds() || assessment.reason.is_some(),
                "{} is {} and names no reason, which is a refusal nobody can act on",
                assessment.subject,
                assessment.state.slug()
            );
        }
        // The recipe is decided by the whole codec, so a shipped tree holds it.
        assert!(
            pass.of(Subject::Recipe).expect("the recipe subject").holds(),
            "the shipped recipe must compile and load back: {:?}",
            pass.of(Subject::Recipe).map(|assessment| assessment.reason.clone())
        );
    }

    #[test]
    fn the_seam_answers_what_the_surfaces_answer() {
        // **The anti-drift test.** The seam delegates, so its answers and the surfaces' answers are
        // the same implementation read twice — and if anyone ever re-derives one of them inside
        // `Deterministic`, this is what fails.
        let pass = audit(&shipped());
        let workspace = root();

        let measured = census::census(&workspace).expect("the census runs");
        for crate_ in CRATES {
            for axis in AXES {
                let seam = pass.of(Subject::Axis { crate_, axis }).expect("an axis subject");
                let direct = measured
                    .reading_of(crate_)
                    .expect("every crate is measured")
                    .cell(axis);
                assert_eq!(
                    seam.state, direct,
                    "{}/{}: the seam says {} and the census says {}",
                    crate_.name(),
                    axis.name(),
                    seam.state.slug(),
                    direct.slug()
                );
            }
        }

        for state in features::evaluate(&workspace) {
            let seam = pass.of(Subject::Feature(state.feature)).expect("a feature subject");
            assert_eq!(
                seam.state,
                state.state,
                "{}: the seam says {} and the feature table says {}",
                state.feature.name(),
                seam.state.slug(),
                state.state.slug()
            );
        }
    }

    #[test]
    fn an_unreachable_decider_refuses_every_subject_and_nothing_passes() {
        let pass = audit(&Unreachable { why: "the model instrument is not reachable here".into() });
        assert_eq!(pass.in_state(Establishment::Active).len(), 0, "a missing instrument is not a pass");
        assert_eq!(
            pass.in_state(Establishment::Refused).len(),
            SUBJECT_COUNT,
            "every subject is refused, and each one names it"
        );
        assert_eq!(
            pass.refusals.all().len(),
            SUBJECT_COUNT,
            "**the refusals are collected, not returned at the first one** — a pass that stopped \
             early would report one problem and hide {}",
            SUBJECT_COUNT - 1
        );
        assert!(pass.report().contains("decider\tunreachable"));
    }

    #[test]
    fn a_recipe_that_does_not_conform_is_decided_and_not_raised() {
        // **A decision of "no" is an `Ok`.** The instrument was reached; it answered. The caller
        // decides whether that stops a build, and the pass says so either way.
        // The header is the current one, so the refusal under test is the **window block's**
        // missing keys and not a version this build does not read.
        let source: &'static [u8] = b"shploingus.recipe/3\npackage\tghost\tv0\nwindow\tghost\n";
        let decider = Deterministic::new(Box::leak(root().into_boxed_path())).with_recipe(source);
        let pass = audit(&decider);
        let recipe = pass.of(Subject::Recipe).expect("the recipe subject");
        assert_eq!(recipe.state, Establishment::Refused);
        assert!(recipe.reason.is_some(), "a refusal names what it refused");
        assert_eq!(pass.decider, "deterministic", "the pass names the instrument that ran");
    }

    /// A model instrument that reaches every subject and answers `Inactive` to all of them — so
    /// the two passes disagree on purpose, everywhere.
    struct Contrarian;

    impl Decider for Contrarian {
        fn name(&self) -> &'static str {
            "contrarian"
        }

        fn decide(&self, subject: Subject) -> Result<Assessment, RefusalSet> {
            Ok(Assessment {
                subject: subject.name(),
                state: Establishment::Inactive,
                instrument: self.name(),
                reached: true,
                reason: Some("a test instrument that disagrees on purpose".to_string()),
            })
        }
    }

    #[test]
    fn the_readers_report_and_the_layer_rules() {
        // **The inversion, pinned.** `observe` returns facts and this module returns the rule, so
        // the table's own display is that same pair — one rule, applied twice, which is why a
        // change to it cannot make the two disagree.
        let workspace = root();

        for observation in features::observe(&workspace) {
            let (state, reason) = rule_for_feature(&observation);
            let evaluated = features::evaluate(&workspace)
                .into_iter()
                .find(|state| state.feature == observation.feature)
                .expect("every observation has a row");
            assert_eq!(evaluated.state, state, "{}: states differ", observation.feature.name());
            assert_eq!(
                evaluated.reason,
                reason,
                "{}: the rule and the table's display disagree about the reason",
                observation.feature.name()
            );
        }

        let measured = census::census(&workspace).expect("the census runs");
        for observation in census::observe(&workspace).expect("the tree is readable") {
            let reading = measured.reading_of(observation.crate_).expect("a reading per crate");
            for axis in AXES {
                let (state, reason) = rule_for_axis(&observation, axis);
                assert_eq!(
                    reading.cell(axis),
                    state,
                    "{}/{}: the rule and the census disagree about the state",
                    observation.crate_.name(),
                    axis.name()
                );
                assert_eq!(
                    reading.why_at(axis).map(str::to_string),
                    reason,
                    "{}/{}: the rule and the census disagree about the reason",
                    observation.crate_.name(),
                    axis.name()
                );
            }
        }
    }

    #[test]
    fn the_two_passes_are_side_by_side_and_their_disagreements_are_preserved() {
        let two = run(&shipped(), &Contrarian);
        assert_eq!(two.model_decided(), SUBJECT_COUNT, "the contrarian reached every subject");

        let divergences = two.divergences();
        assert!(
            divergences.len() >= 19,
            "a contrarian instrument must disagree with the system somewhere, and it disagreed \
             {} times",
            divergences.len()
        );
        for divergence in &divergences {
            assert_eq!(divergence.model, Establishment::Inactive);
            assert!(
                divergence.model_reason.is_some(),
                "{}: a disagreement carries both instruments' reasons",
                divergence.subject
            );
        }

        // **Neither pass is overwritten by the other.** The system's count is the same before and
        // after the model ran, which is the difference between preserving a disagreement and
        // synthesizing one.
        let alone = audit(&shipped());
        assert_eq!(two.system.holding(), alone.holding());
        assert_eq!(two.system.assessments, alone.assessments);

        let report = two.report();
        assert!(report.contains("pass two"), "the report prints both passes");
        assert!(report.contains("divergence\t"), "and every disagreement");
    }

    #[test]
    fn an_absent_instrument_is_not_a_disagreement() {
        let two = run(&shipped(), &Unreachable { why: "the model is not built yet".into() });
        assert_eq!(two.model_decided(), 0, "an unreachable instrument decides nothing");
        assert!(
            two.divergences().is_empty(),
            "**an absence is not a disagreement** — a subject only one instrument looked at is not \
             a place the two disagree"
        );
        let report = two.report();
        assert!(
            report.contains(&format!("model-unreached\t{SUBJECT_COUNT}")),
            "{report}"
        );
        assert!(!report.contains("divergence\t"), "and it prints no divergence line");
    }

    #[test]
    fn the_subject_vocabulary_is_closed_and_counted() {
        // A literal, checked against the vocabulary it counts — the same mutation test the axis gate
        // relies on. Change a crate to add an axis and this is the compile error that says so.
        assert_eq!(AXIS_SUBJECT_COUNT, 60, "12 crates x 5 axes");
        // **80 → 86.** The six declared numbers join the vocabulary, so they are decided by the
        // seam and printed by the gate beside the other eighty rather than living in a module
        // nothing calls. A literal, so an arm added without its rows does not compile.
        assert_eq!(
            SUBJECT_COUNT, 86,
            "60 axis subjects + 19 features + the recipe + 6 declared numbers"
        );
        assert_eq!(SUBJECTS_ARE_TOTAL.len(), SUBJECT_COUNT);
        let all = subjects();
        assert_eq!(all.len(), SUBJECT_COUNT);
        for subject in &all {
            assert!(!subject.name().is_empty(), "a subject with no name cannot be reported");
        }
        // The names are distinct, so a lookup by name cannot find two subjects.
        let mut names: Vec<String> = all.iter().map(|subject| subject.name()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), SUBJECT_COUNT, "two subjects share a name");
    }
}
