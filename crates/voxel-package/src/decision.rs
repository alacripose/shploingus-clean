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
//! The answer to a decision is [`Establishment`] — `Active`, `Inactive`, `Stale`, `Unchecked`,
//! `Refused`, which `features.rs` already defines and `census.rs` already reuses *for the stated
//! reason*:
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

use voxel_model::axis::AxisKind;

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
    /// **One ladder of the civilians' evidence.** The DSL's binding sentence for the civilian
    /// container is that *"its evidence is the proof of work for solutions, graded against
    /// `axis.rs`"*, so a civilian subject is one **ladder** — [`AxisKind`], the seven ladders
    /// `axis.rs` declares — and not one aggregate grade.
    ///
    /// **Why one per ladder, measured rather than preferred.** [`Assessment`] carries a single
    /// `state: Establishment`, and `axis.rs`'s rules 51 and 53 refuse both comparing across
    /// ladders and reducing one to a number (`refuse_interchange`, `refuse_reduction`). A single
    /// aggregate subject therefore *cannot carry* seven ladder verdicts: it would have to either
    /// collapse them — refused by the tree's own rules — or keep one and discard six. One
    /// subject per ladder is the only shape that can hold the grading at all.
    Civilian(AxisKind),
    /// **One row of the bridge's surface** — the port's tool table and the four capability names
    /// that partition it.
    ///
    /// **The row is a decided subject, and that is the point rather than a completeness flourish.**
    /// A capability map living beside the seam would be a fifth decision surface, which is the exact
    /// defect this module was created to close (*"the tree had four decision surfaces that each
    /// answered in their own code"*). So the surface is asked here, with every other subject, and
    /// the gate's report carries it.
    ///
    /// **The subject holds the row, not its name**, so a subject naming a tool the surface does not
    /// carry is unrepresentable — the same argument [`Subject::Axis`] makes by holding a `Crate` and
    /// an `Axis` rather than two strings.
    Tool(crate::bridge::Tool),
}

/// How many axis subjects: the whole grid, every crate on every axis.
pub const AXIS_SUBJECT_COUNT: usize = CRATE_COUNT * AXIS_COUNT;

/// How many civilian subjects: **one per ladder**, so the count is `axis.rs`'s own
/// `AXIS_COUNT` and is never a hand-written number that can drift from the ladder set.
pub const CIVILIAN_SUBJECT_COUNT: usize = voxel_model::axis::AXIS_COUNT;

/// **How many subjects there are. A literal, as everywhere else** — so adding an axis, a feature
/// or a number without deciding how it is asked is caught here rather than at the end of a
/// report.
///
/// **80 → 86 → 98 → 105 → 128 → 152: `+ CLAIM_COUNT`, then the sixth census axis, then the civilian
/// ladders, then the bridge's surface, then the four crates the census table was not measuring.**
/// The value is meaningless; the **type** is the assertion, and it is what stopped each arm from
/// being added without the rows that go with it.
///
/// **The last step is the one that happened on its own, and that is the point of deriving it.**
/// `voxel-light`, `voxel-audio`, `voxel-von` and `voxel-mcp` joined the workspace and this
/// vocabulary grew by 4×6 subjects with no edit here, because the grid is the crate table's rather
/// than a second list. A literal would have gone stale silently; a derived count cannot.
///
/// **The bridge's rows are counted from `bridge::TOOLS.len()` rather than written as a literal,
/// and the two cannot be made to disagree**: that table's length *is* `Action::COUNT`, pinned by
/// `bridge::TOOLS_ARE_TOTAL`, so a literal here would be a second place to keep in step for
/// nothing.
pub const SUBJECT_COUNT: usize = AXIS_SUBJECT_COUNT
    + FEATURE_COUNT
    + 1
    + crate::numbers::CLAIM_COUNT
    + CIVILIAN_SUBJECT_COUNT
    + crate::bridge::TOOLS.len();

/// The compile-time check that [`SUBJECT_COUNT`] and the vocabulary agree.
pub const SUBJECTS_ARE_TOTAL: [(); SUBJECT_COUNT] = [(); CRATE_COUNT * AXIS_COUNT
    + FEATURE_COUNT
    + 1
    + crate::numbers::CLAIM_COUNT
    + CIVILIAN_SUBJECT_COUNT
    + crate::bridge::TOOLS.len()];

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
            // The ladder's own recorded name, so a reader who knows `axis.rs` knows this name.
            Subject::Civilian(ladder) => format!("civilian/{}", ladder.name()),
            // The row's own recorded verb, so a reader who knows the matrix's vocabulary knows this
            // name — the tool table is named after `Action` for exactly this reason.
            Subject::Tool(row) => format!("tool/{}", row.name),
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
    // **The civilians' ladders, last and in `axis.rs`'s own order** — one subject per ladder, so
    // the count beside them is how a reader knows the ladder set is whole.
    for ladder in AxisKind::ALL {
        out.push(Subject::Civilian(ladder));
    }
    // **The bridge's surface, last.** One `tool/<name>` subject per row, in the table's own order, so
    // the surface this tree exposes through the port is decided by this seam and reported by the
    // gate rather than being a table the checker in `bridge` is the only thing to have looked at.
    for row in crate::bridge::TOOLS {
        out.push(Subject::Tool(row));
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
    /// follows, for the same reason. **A holding subject may still carry one when it has a catch
    /// to report** — the ticket-evidence ladder does, naming the citations it graded — because a
    /// settled reading can still owe the reader a fact, and the report prints it (`Pass::report`).
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
        // **The doc stage.** The facts come from `census::observe`; the rule is here so a second
        // instrument is handed the same numbers and rules them its own way.
        Axis::Documented => match &observation.prose {
            None => (
                Establishment::Refused,
                Some(format!(
                    "{} has no readable source, so where its rationale lives was not measured",
                    observation.crate_.name()
                )),
            ),
            Some(prose) => {
                // A second record beside the chain is refused whatever the prose share reads:
                // `forbidden.second-record` is not a matter of degree.
                if !prose.invented.is_empty() {
                    return (
                        Establishment::Refused,
                        Some(format!(
                            "{} keeps {} invented record(s) beside the chain: {}. The chain is \
                             the one append-only record, and a store beside it becomes the next \
                             agent's source of truth",
                            observation.crate_.name(),
                            prose.invented.len(),
                            prose.invented.join(", ")
                        )),
                    );
                }
                // **The frame's own share: plain comment only.** Rustdoc is not counted against the
                // frame, because the ruling of 2026-10-10 is that documentation belongs in it. The
                // count below is what decides this axis; the share is carried into the reason and
                // refuses only by being **unmeasured**, because a cell with no reading is not a pass.
                let Some(percent) = prose.plain_percent else {
                    return (
                        Establishment::Refused,
                        Some(format!(
                            "{}'s prose share was not measured ({}), so this axis has no reading",
                            observation.crate_.name(),
                            prose.why_unmeasured.as_deref().unwrap_or("no reason recorded")
                        )),
                    );
                };
                // **The compiler's own count** (since 2026-10-10): the axis is named for what it
                // now reads. Each crate root enables `missing_docs`, so the number the compiler
                // reports is a count of the documentation itself — it falls when docs are written
                // and rises when public items are added without them.
                //
                // **What this replaces, and why.** The arm asked whether the entry file opened with
                // a `//!` block, and every crate had one, so the axis was `active` for all sixteen
                // and could only have turned false if somebody deleted a line at the top of a file.
                // That is a check on the *presence of a claim*, not on the documentation: a header
                // saying a crate documents itself is exactly the kind of evidence this tree refuses
                // everywhere else, and the `//!` blocks were written from the same prose the headers
                // already carried. The user's ruling stands — *the reason for a value belongs beside
                // the value*, where `cargo doc` renders it and a hover shows it — and the count is
                // what says whether that happened, item by item.
                let Some(reading) = &prose.missing_docs else {
                    return (
                        Establishment::Refused,
                        Some(format!(
                            "{} has no line in {}: no instrument asked the compiler about it, so \
                             this crate is unmeasured on the axis it is named for. A crate nobody \
                             counted is not a documented one — the reading is taken by the recipe \
                             in that file's header",
                            observation.crate_.name(),
                            census::MISSING_DOCS_PATH
                        )),
                    );
                };
                // **A pin over part of a crate is not a pin.** A covered file that did not decode is
                // not hashed, so the digest would describe a subset of the crate while reading as
                // though it described the crate — and a reading pinned to some of the bytes cannot
                // say whether it still holds. Refused rather than hashed partially, the same rule
                // `provenance` applies one level up.
                if !prose.undecodable.is_empty() {
                    return (
                        Establishment::Refused,
                        Some(format!(
                            "{} has {} covered file(s) that did not decode as text ({}), so the pin \
                             over its sources would cover a subset of them and could not say whether \
                             the count still holds",
                            observation.crate_.name(),
                            prose.undecodable.len(),
                            prose.undecodable
                                .iter()
                                .map(|path| path.display().to_string())
                                .collect::<Vec<_>>()
                                .join(", ")
                        )),
                    );
                }
                // **The pin is checked before the count is read**, and it is the one part of this
                // axis the gate re-derives itself: the count needs a compiler it cannot run from
                // inside its own build script, but hashing the bytes it was taken over needs nothing
                // but a walk. When those bytes have moved, the number is not this build's answer, so
                // `stale` says exactly that instead of `inactive` repeating a figure about a tree
                // this one is not. Neither a pass nor a failure — the repair is a command, so this
                // does not refuse the door (see `conformance::of`, which reports it beside the
                // refusals it does not gate on).
                if reading.declared != reading.measured {
                    return (
                        Establishment::Stale,
                        Some(format!(
                            "{}'s documentation reading in {} was taken under pin {} and its sources \
                             now digest to {}: the count of {} undocumented public item(s) is about a \
                             tree this one is not, so it is reported as stale rather than repeated as \
                             a measurement. Re-run the recipe in that file's header — the count and \
                             the pin are taken together — and the axis reads the new one",
                            observation.crate_.name(),
                            census::MISSING_DOCS_PATH,
                            pin_short(&reading.declared),
                            pin_short(&reading.measured),
                            reading.missing
                        )),
                    );
                }
                let missing = reading.missing;
                if missing == 0 {
                    return (
                        Establishment::Active,
                        Some(format!(
                            "the compiler reports no undocumented public item in {}: `missing_docs` \
                             is enabled in its crate root, the reading in {} records 0, and its pin \
                             still matches the files on disk. Rustdoc beside the items is what \
                             `cargo doc` renders as this crate's pages and a hover shows on the item \
                             itself",
                            observation.crate_.name(),
                            census::MISSING_DOCS_PATH
                        )),
                    );
                }
                (
                    Establishment::Inactive,
                    Some(format!(
                        "the compiler reports {missing} undocumented public item(s) in {}: \
                         `missing_docs` is enabled in its crate root, the reading in {} records \
                         {missing}, and its pin matches the files on disk. {percent}% of its source is \
                         plain comment beside that (frame {} %), so the rationale is written where a \
                         reader has to stand in the file to find it; the count falls as the rustdoc \
                         that belongs beside each item is written",
                        observation.crate_.name(),
                        census::MISSING_DOCS_PATH,
                        census::PROSE_FRAME_PERCENT
                    )),
                )
            }
        },
    }
}

/// A pin in a reason, short enough to read beside another one.
///
/// **Sixteen hex digits, the same shortening [`voxel_model::identity`] uses in its refusals.** A
/// reason that quotes two 71-character digests is a reason nobody reads, and the full pins are still
/// on the reading — the report's job here is to make it possible to see *that* they differ and *which*
/// moved, and the repair is stated in the same line.
fn pin_short(pin: &voxel_model::identity::Identity) -> String {
    pin.to_string().chars().take("sha256:".len() + 16).collect()
}

impl Assessment {
    /// Whether this assessment says the decision was checked and holds.
    pub fn holds(&self) -> bool {
        self.state == Establishment::Active
    }

    /// Whether this assessment says the reading **was checked and is about this tree**.
    ///
    /// The distinction a caller needs when it is about to act on a number rather than display it:
    /// [`holds`](Self::holds) is `active`, and a `stale` cell is not active either — but the two are
    /// not the same fact, and a caller that cannot tell them apart will either ignore a reading it
    /// could re-derive or trust one that is about a tree that no longer exists.
    pub fn is_stale(&self) -> bool {
        self.state == Establishment::Stale
    }
}

/// **What a civilian ladder's witness means.** The rule the reader does not hold.
///
/// `civilians::observe` reports whether the governed record carries the ladder's witness;
/// **this** says what that is worth. Deliberately the same shape as [`rule_for_feature`],
/// because the two report the same kind of fact: a witness that is there, absent, or
/// unreadable.
///
/// **A present witness is `Active`; an absent one is `Unchecked`; an unreadable record is
/// `Refused`.** The difference between the last two is the whole reason there are three:
/// *"the record could not be read"* is a refusal, and *"the record does not carry this"* is a
/// state a reader can act on. **Neither is a pass** — the same rule `census`'s `None` arm and
/// `features`' missing row already keep.
pub fn rule_for_civilian(
    observation: &crate::civilians::LadderObservation,
) -> (Establishment, Option<String>) {
    // **The second arm, and it exists because the rows landed.** The first arm grades the
    // *instrument* — whether the record has somewhere on this ladder to put evidence at all.
    // The rows are the evidence itself: the tickets' citations, read back from the record with
    // the cited document opened and the cited sha256 checked against the bytes on disk. Only the
    // ticket-evidence ladder carries rows, so only it takes this arm; the other six ladders fall
    // through to the witness reading unchanged.
    if observation.ladder == AxisKind::TicketEvidence {
        if let Some(unread) = &observation.rows_unread {
            return (
                Establishment::Refused,
                Some(format!(
                    "the record's tickets could not be read back, so the citations this ladder \
                     grades were not read at all: {unread}"
                )),
            );
        }
        if !observation.rows.is_empty() {
            use crate::civilians::TicketRow;
            // **The rungs, three ways, and they are three different facts:**
            //
            //   * `holds == Some(true)` — a citation whose document was opened and whose sha256
            //     still matches: **read back from the record** (`ticket-evidence B`);
            //   * an answer that names no citation at all — **self-report** (`ticket-evidence C`),
            //     which is evidence and not authority, and is counted here rather than refused:
            //     the rung exists for exactly this evidence;
            //   * anything else — a citation that could not be checked, or one whose digest no
            //     longer matches the bytes on disk — is **broken**, and a broken read-back is a
            //     refusal that names the ticket, because a claimed B that cannot be verified must
            //     not pass as a C.
            let read_back = observation
                .rows
                .iter()
                .filter(|row| row.holds == Some(true))
                .count();
            let self_report = observation
                .rows
                .iter()
                .filter(|row| row.document.is_none())
                .count();
            let broken: Vec<&TicketRow> = observation
                .rows
                .iter()
                .filter(|row| row.holds != Some(true) && row.document.is_some())
                .collect();
            if broken.is_empty() {
                return (
                    Establishment::Active,
                    Some(format!(
                        "{} ticket row(s) on this ladder: {read_back} read back from the document \
                         they cite (ticket-evidence B) with every cited sha256 still verifying, \
                         {self_report} self-report (ticket-evidence C — evidence, but not \
                         authority), none broken",
                        observation.rows.len()
                    )),
                );
            }
            // **Every broken row is named with its own reason**, and the tail says how many
            // were left unprinted rather than sampling invisibly (`Board::describe`'s own rule).
            let named: Vec<String> = broken
                .iter()
                .take(8)
                .map(|row| {
                    format!(
                        "{}: {}",
                        row.ticket,
                        row.reason.as_deref().unwrap_or("no reason recorded")
                    )
                })
                .collect();
            return (
                Establishment::Refused,
                Some(format!(
                    "{} of {} ticket row(s) are broken — {}{}",
                    broken.len(),
                    observation.rows.len(),
                    named.join("; "),
                    if broken.len() > 8 {
                        format!("; and {} more", broken.len() - 8)
                    } else {
                        String::new()
                    }
                )),
            );
        }
    }
    match (observation.readable, observation.present) {
        (true, true) => (Establishment::Active, None),
        (true, false) => (
            Establishment::Unchecked,
            Some(format!(
                "the governed record carries no witness for {}, so the civilians have nowhere on \
                 that ladder to put evidence and nothing on it is graded: {} does not contain \
                 {:?}. That is a state and not a refusal — the row is readable and this is what \
                 it says",
                observation.ladder.name(),
                observation.witness,
                observation.token
            )),
        ),
        (false, _) => (
            Establishment::Refused,
            Some(format!(
                "{} could not be read, so whether the record carries {}'s witness is unknown. \
                 Unwitnessed is not claimed",
                observation.witness,
                observation.ladder.name()
            )),
        ),
    }
}

/// **What one row of the bridge's surface means.** The rule that `bridge::row_refusals` does not
/// hold, completing the same pair — the reader reports, the layer decides — that
/// [`rule_for_feature`] and [`rule_for_civilian`] complete.
///
/// It is also **why the port's four capability names are a decided subject instead of a table
/// beside the seam.**
///
/// | the row | state | why |
/// |---|---|---|
/// | reports no problem | **`Active`** | the row is coherent: a spellable name, and a capability that agrees with the matrix's class for the act |
/// | reports one | **`Refused`** | **with the row's own words attached**, because a refusal that names nothing is a refusal nobody can act on |
///
/// **`Unchecked` is unreachable here, and that is not an omission.** `Unchecked` means *"this was
/// not measured"*, and a row is a value this build holds — there is no row that could not be read.
/// That state belongs to a subject made of names, which is why [`Subject::Tool`] holds the row
/// itself.
///
/// **`Refused` here is not a build failure, and it does not mean "denied."** The row this tree
/// forbids outright ([`crate::policy::Action::DeleteContent`]) **holds**: it is a coherent row whose
/// class is `always_denied`, and the whole reason it ships is so that calling it is a recorded
/// denial rather than an absent verb nobody can read. What this rules on is *whether the surface is
/// coherent* — never *whether a particular caller may act*, which needs an authority and a grant and
/// is [`crate::bridge::call`]'s question.
pub fn rule_for_tool(row: &crate::bridge::Tool) -> (Establishment, Option<String>) {
    let refusals = crate::bridge::row_refusals(row);
    if refusals.is_empty() {
        // No reason: a coherent row owes the reader nothing, which is `Pass::report`'s own rule
        // (*"a holding subject with no reason is not printed"*) and keeps the gate's body the size
        // of what is actually wrong.
        return (Establishment::Active, None);
    }
    (
        Establishment::Refused,
        Some(format!("{}: {refusals}", row.name)),
    )
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
            Subject::Civilian(ladder) => {
                // The same pair again: `civilians::observe` **reads** the record and
                // `rule_for_civilian` **rules** on the reading. Going through a function that
                // already applied the rule here would hide where the decision lives, which is
                // the thing `the_seam_answers_what_the_surfaces_answer` exists to prevent.
                let observations = crate::civilians::observe(self.root);
                let assessment = match observations
                    .iter()
                    .find(|observed| observed.ladder == ladder)
                {
                    Some(observed) => {
                        let (state, reason) = rule_for_civilian(observed);
                        Assessment {
                            subject: subject.name(),
                            state,
                            instrument: self.name(),
                            reached: true,
                            reason,
                        }
                    }
                    // A ladder this build knows with no row in the table: named, never defaulted
                    // to `Active`.
                    None => Assessment {
                        subject: subject.name(),
                        state: Establishment::Unchecked,
                        instrument: self.name(),
                        reached: true,
                        reason: Some(format!(
                            "the ladder table returned no row for {}, so nothing was evaluated",
                            ladder.name()
                        )),
                    },
                };
                Ok(assessment)
            }
            Subject::Tool(row) => {
                // Read, then rule — the same pair as every other arm. `bridge::row_refusals`
                // reports what is wrong with the row and `rule_for_tool` says what that is worth;
                // going through `bridge::check_surface` here would fold the whole table and lose
                // which row this subject is about, which is the thing the pairs exist to prevent.
                let (state, reason) = rule_for_tool(&row);
                Ok(Assessment {
                    subject: subject.name(),
                    state,
                    instrument: self.name(),
                    reached: true,
                    reason,
                })
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
/// **This is [`crate::oracle::UnreachableOracle`], and that is the ruling rather than a
/// tidy-up.** H-14.1 made Laya the decision layer and `oracle::Oracle` *one implementation of
/// the seam*; this module's own header says so (*"the seam is here, and `oracle::Oracle` is one
/// implementation of it"*), and the DSL names the skipped case in the oracle's component:
/// *"`oracle::Oracle` — one implementation of the seam, with `oracle::UnreachableOracle` for
/// the skipped case"*. A second `Unreachable` here would be two names for one meaning — D-07,
/// the defect that made `voxel_model::grant` move **down** rather than be duplicated.
///
/// So the seam's skipped case is declared where the DSL says it is, and implements
/// [`Decider`] there. Its purpose is unchanged: it makes *"the decision layer was skipped"*
/// impossible to mistake for *"the decision layer passed"*.
pub use crate::oracle::UnreachableOracle as Unreachable;

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
        // **Every state, including `stale`.** A subject whose reading is about another tree is a
        // fact this section has to carry: the client reads these numbers as the gate's answer, and a
        // state the writer omitted would make the counts fail to sum to `subjects` — which the
        // reader refuses, correctly, rather than rendering two numbers that disagree.
        for state in [
            Establishment::Active,
            Establishment::Inactive,
            Establishment::Stale,
            Establishment::Unchecked,
            Establishment::Refused,
        ] {
            out.push_str(&format!("subject-{}\t{}\n", state.slug(), self.in_state(state).len()));
        }
        // Every subject that does not hold, with its reason — the report a reader actually wants,
        // because "which decision is not settled" is the question and a count is not the answer.
        //
        // **And every holding subject that carries a reason**, because a settled subject can
        // still owe the reader a fact: the ticket-evidence ladder holds *with a catch* — the
        // rows it graded — and this is the only surface the client reads (BLD-0026). A holding
        // subject with no reason is not printed, exactly as before.
        for assessment in &self.assessments {
            if assessment.holds() && assessment.reason.is_none() {
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
    /// Readings the report says are **about a tree that has moved**. Carried for the same reason the
    /// others are: the sum below has to close, and a state the reader dropped would be a subject the
    /// panel renders as something else.
    pub stale: usize,
    pub unchecked: usize,
    pub refused: usize,
    /// **Every subject the report printed with a reason** — C-83.5's rule, carried: *"which
    /// decision is not settled* is the question and a count is not the answer". A holding
    /// subject appears here when it carries a reason (the ticket-evidence ladder does), so a
    /// reader of `pending` sees every reasoned row the report wrote, not only the failures.
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
        stale: 0,
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
            "subject-stale" => pass.stale = number(&mut refusals, &cells, 1, line),
            "subject-unchecked" => pass.unchecked = number(&mut refusals, &cells, 1, line),
            "subject-refused" => pass.refused = number(&mut refusals, &cells, 1, line),
            "subject" => {
                let Some(state) = cells.get(2).and_then(|word| state_of(word)) else {
                    refusals.refuse(
                        RefusalKind::NoReading,
                        format!(
                            "subject {:?} carries state {:?}, which is not one of the five the \
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

    // **The section ends where the next one begins.** The gate's report continues after the
    // two passes — `# what this gate is not` carries `integrity` and `identity` — and a reader
    // that ran to the end of the text read those keys as keys `TwoPass::report` never writes.
    // Measured on the console, which refused the build's own report: `logs/live.out` line 307.
    // The writer's keys and subject rows never begin `# `; the writer's sections all do.
    let two_end = next_section(text, two_start);
    for line in lines_between(text, two_start, two_end) {
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
    let summed = pass.active + pass.inactive + pass.stale + pass.unchecked + pass.refused;
    if summed != pass.subjects {
        refusals.refuse(
            RefusalKind::DigestMismatch,
            format!(
                "the report counts {} subject(s) and its five states sum to {summed}, so the \
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
    // a decision nobody is shown is unsettled. **A holding row may appear in `pending` too** —
    // the writer prints a settled subject that carries a catch, the ticket-evidence ladder does —
    // so the count that must close is the *non-holding* rows against `subjects - active`, and
    // the holding rows are named as the extra facts they are.
    let printed = pass.subjects.saturating_sub(pass.active);
    let unsettled = pass
        .pending
        .iter()
        .filter(|row| row.state != Establishment::Active)
        .count();
    if unsettled != printed {
        refusals.refuse(
            RefusalKind::DigestMismatch,
            format!(
                "{printed} subject(s) do not hold and the report names {unsettled} of them, so the \
                 client would render a count the reader cannot check"
            ),
        );
    }

    refusals.into_result((pass, two))
}

/// The state a report word names, and `None` for a word the five states do not cover.
fn state_of(word: &str) -> Option<Establishment> {
    match word {
        "active" => Some(Establishment::Active),
        "inactive" => Some(Establishment::Inactive),
        "stale" => Some(Establishment::Stale),
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

/// Where the section beginning at `start` ends: the first following line that begins a new
/// `# ` section, or the end of the text.
///
/// **A section reader must know where its section stops**, or the next section's keys arrive
/// as rows of this one. That is not hypothetical here: the build's report has a section after
/// the two passes, and running the reader to the end made it refuse the build's own keys.
fn next_section(text: &str, start: usize) -> usize {
    text.lines()
        .enumerate()
        .skip(start + 1)
        .find(|(_, line)| line.starts_with("# "))
        .map(|(index, _)| index)
        .unwrap_or_else(|| text.lines().count())
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_model::identity::Identity;

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
        // tree reads **98** today, not the 80 C-83 recorded, because `numbers::CLAIM_COUNT` and
        // the doc stage's sixth axis are inside the vocabulary — and a test that had asserted
        // `80` would be reporting a stale number as though it were the gate's.
        assert_eq!(
            read_pass.subjects,
            SUBJECT_COUNT,
            "the report and the subject vocabulary agree, and the vocabulary is measured"
        );
        assert_eq!(read_pass.active, two.system.in_state(Establishment::Active).len());
        assert_eq!(read_pass.inactive, two.system.in_state(Establishment::Inactive).len());
        assert_eq!(read_pass.stale, two.system.in_state(Establishment::Stale).len());
        assert_eq!(read_pass.unchecked, two.system.in_state(Establishment::Unchecked).len());
        assert_eq!(read_pass.refused, two.system.in_state(Establishment::Refused).len());
        assert_eq!(read_pass.holding(), two.system.holding());

        // **The writer's own rule, mirrored exactly**: the report prints a subject that does not
        // hold, and a holding subject that carries a reason (the ticket-evidence ladder does).
        let mut expected: Vec<(String, Establishment, String)> = two
            .system
            .assessments
            .iter()
            .filter(|assessment| !assessment.holds() || assessment.reason.is_some())
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
    fn a_section_after_the_two_passes_is_not_read_as_the_model_s_rows() {
        // **The gate's own shape.** The build's report continues after the two passes
        // (`# what this gate is not`), and a reader that ran to the end of the text read those
        // keys as keys this writer never emits — the console refused the build's own report.
        let two = run(&shipped(), &Unreachable { why: "the model is not built yet".into() });
        let mut text = two.report();
        text.push_str("\n# what this gate is not\n");
        text.push_str("integrity\testablished\n");
        text.push_str("identity\tnot-established-no-signature\n");
        let (_, read_model) =
            read_report(&text).expect("the trailing section is not the model's rows");
        assert_eq!(read_model.model_decided, 0);
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
        assert_eq!(AXIS_SUBJECT_COUNT, 96, "16 crates x 6 axes");
        // **80 → 86 → 98 → 105 → 128 → 152.** The six declared numbers join the vocabulary, then
        // the doc stage adds a sixth axis, then the civilian ladders, then the bridge's surface,
        // and then the four crates the census was not measuring: each is decided by the seam and
        // printed by the gate rather than living in a module nothing calls. A literal, so an arm
        // added without its rows does not compile.
        assert_eq!(
            SUBJECT_COUNT, 152,
            "96 axis subjects + 19 features + the recipe + 6 declared numbers + 7 civilian \
             ladders + 23 tool rows — the bridge's surface, so a capability is a decided subject \
             rather than a table beside the seam"
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

    #[test]
    fn every_row_of_the_bridge_s_surface_holds_as_a_decided_subject() {
        // **The change this test exists for.** The port's tool rows are asked of *this* seam, beside
        // the crates, the features, the recipe, the numbers and the ladders — so a capability map
        // cannot become a fifth decision surface answering "may this act proceed?" in its own code,
        // which is the exact defect this module was created to close.
        //
        // The rows are decided **through the seam's own arm** rather than by calling `rule_for_tool`
        // directly, and the root is deliberately not the tree: the tool arm reads its row and nothing
        // else, so a test that needed a workspace here would be hiding a file read behind a table.
        let decider = Deterministic::new(std::path::Path::new("."));
        for row in crate::bridge::TOOLS {
            let subject = Subject::Tool(row);
            assert_eq!(subject.name(), format!("tool/{}", row.name));
            let assessment = decider
                .decide(subject)
                .expect("the tool arm reads its row and refuses nothing");
            assert!(assessment.reached, "{}: the seam did not reach it", subject.name());
            assert_eq!(assessment.instrument, decider.name());
            assert_eq!(
                assessment.state,
                Establishment::Active,
                "{}: {}",
                subject.name(),
                assessment.reason.as_deref().unwrap_or("no reason")
            );
            // A coherent row owes the reader nothing, so a surface with nothing wrong with it adds no
            // line to the gate — `Pass::report` prints what does not hold, and what holds *with a
            // catch*, and nothing else.
            assert!(assessment.reason.is_none(), "{}", subject.name());
        }

        // And the row this tree forbids outright is among them, **holding** — it is a coherent row
        // whose class is `always_denied`, and that coherence is what makes calling it a recorded
        // denial instead of an absent verb nobody can read. `Refused` here would mean the row was
        // malformed rather than forbidden, which is the distinction the whole arm exists to draw.
        let forbidden = crate::bridge::TOOLS
            .iter()
            .find(|row| row.class() == crate::policy::PermissionClass::AlwaysDenied)
            .expect("the vocabulary holds an act the record forbids");
        let assessment = decider
            .decide(Subject::Tool(*forbidden))
            .expect("a decision");
        assert!(
            assessment.holds(),
            "{} is {} and must hold: {:?}",
            forbidden.name,
            forbidden.class().slug(),
            assessment.reason
        );
    }

    #[test]
    fn a_documentation_reading_pinned_to_other_bytes_is_stale_and_never_a_pass() {
        // **The state this rule exists for, and the one it must not be folded into.** The count comes
        // from a command a person runs, so it is true of the bytes it was taken over; when those bytes
        // have moved it is not evidence about this tree. Read as `inactive` it prints an old number as
        // today's gap; read as `active` it is worse — and a stale **zero** is the most dangerous of the
        // three, because zero is the one count that reads as a pass. So the three cases below are the
        // boundary, and the third is why the pin is checked before the count is.
        let observation = |missing: usize, declared: Identity, measured: Identity| {
            census::CrateObservation {
                crate_: Crate::Model,
                named_by_record: Some(true),
                sources: None,
                claim: None,
                prose: Some(census::ProseObservation {
                    measured: true,
                    why_unmeasured: None,
                    files: 1,
                    ruled_files: 1,
                    undecodable: Vec::new(),
                    comment_percent: Some(20),
                    plain_percent: Some(5),
                    missing_docs: Some(census::MissingDocsReading { missing, declared, measured }),
                    document: None,
                    invented: Vec::new(),
                }),
            }
        };
        let pin = Identity::of(b"the tree the count was taken over");
        let moved = Identity::of(b"the tree as it stands now");

        let (state, _) = rule_for_axis(&observation(0, pin, pin), Axis::Documented);
        assert_eq!(state, Establishment::Active, "a matching pin and a zero count hold");

        let (state, why) = rule_for_axis(&observation(907, pin, pin), Axis::Documented);
        assert_eq!(state, Establishment::Inactive, "a matching pin and a count is owed work");
        assert!(
            why.expect("a reason").contains("907"),
            "and the count is named, because it is the thing to lower"
        );

        let (state, why) = rule_for_axis(&observation(0, pin, moved), Axis::Documented);
        assert_eq!(state, Establishment::Stale, "a zero about another tree is not a pass");
        let why = why.expect("a stale reading carries its reason");
        assert!(
            why.contains(&pin_short(&pin)) && why.contains(&pin_short(&moved)),
            "both pins are named, so a reader can see which one moved: {why}"
        );
        assert!(
            why.contains("Re-run the recipe"),
            "and the repair is stated, because the fix is a command and not an edit: {why}"
        );

        // The state is about the pin, not about the size of the count: a stale 907 is stale too, and
        // reporting it as `inactive` would present a figure from another tree as this tree's debt.
        let (state, _) = rule_for_axis(&observation(907, pin, moved), Axis::Documented);
        assert_eq!(state, Establishment::Stale);
        assert_ne!(state, Establishment::Active, "and it is not a pass either");

        // **A crate with no reading at all is refused, not stale** — there is no pin to have moved.
        let mut readingless = observation(0, pin, pin);
        readingless.prose.as_mut().expect("prose").missing_docs = None;
        let (state, _) = rule_for_axis(&readingless, Axis::Documented);
        assert_eq!(state, Establishment::Refused, "no reading is not a moved reading");
    }
}
