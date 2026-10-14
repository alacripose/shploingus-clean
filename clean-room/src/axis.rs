//! **U7's axis vocabulary: seven ladders, one type each, and the three ways reading one is a bug.**
//!
//! Derived from **documents**, not from the witness's source (Constitution P1/P7):
//!
//! | Source | What it declares here |
//! |---|---|
//! | `docs/LEDGER.md` §59.5 item 9 (`[A, D, D, D]`) and C-73 §73.1 | the grade ladder's tokens, and that **an origin is not a ladder**: the letters are labels, they happen to be alphabetical, and **no value carries an order**. Rule 1 is *every ladder is its own type*. |
//! | `docs/LEDGER.md` C-97.2's inherited-systems list | the prohibition this module exists for: **each ladder its own type; a cross-axis and a scalar reduction refuse**. |
//! | `docs/LEDGER.md` C-103 §103.1 | the ticket-evidence ladder: `ReadBack` is its **B** rung, registered by bridge — *the machine looked at its own book and the book answered*. |
//! | `docs/LEDGER.md` R-59.7 | the pattern the coverage table copies: `declared_values()` is an **exhaustive match**, `COVERAGE_IS_TOTAL` pins the table's length **by its type**, and `check_coverage` is the anti-fabrication guard that compares each row against the axis's own declaration *rather than against itself*. |
//! | `docs/CIVILIAN-PORT.md` (the `gov/mod.rs` and `resume.rs` rows) and `docs/GAIA.md` | the ladders this tree does **not** declare: the office rank and the certification/resume bands are U12's vocabulary (U2's owed T016), so they are **owed by name** here and never invented. |
//! | `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §2 (U7) | the interface: `AxisKind` plus `refuse_interchange`/`refuse_reduction`/`refuse_bare`. |
//!
//! The behavioural witness is the legacy `axis.rs`'s **test module** (Constitution P7: *its tests and its
//! measured numbers say what the contract is*), which is where the third prohibition's two words come from:
//! a token two ladders claim is **AMBIGUOUS**, and a token one ladder claims is **UNQUALIFIED** — and a token
//! no ladder claims is not this rule's business at all.
//!
//! # An owed ladder is a value, not a silence
//!
//! Four of the seven ladders have no declared token list **in this tree**. The tempting move is to type out
//! a plausible list from memory; that is exactly the substitution Constitution P2 refuses, because a ladder
//! that is *nearly* right reads as checked. So [`AxisKind::declared_values`] answers either
//! [`LadderValues::Declared`] — the tokens, which [`check_coverage`] then holds every row to — or
//! [`LadderValues::Owed`] naming **the document that would declare it**. An owed ladder is printed as owed
//! and never counted as covered.

use crate::refusal::{Refusal, RefusalKind, RefusalSet};

/// **The seven axes** — the closed vocabulary of ladders this project reads a decision on.
///
/// Seven, not six and not five: the plan's §2 U7 entry says *the seven axes*, and the witness's own test
/// module enumerates exactly this set. The order is the declaration order and is the order the gate prints,
/// so a reader can compare two runs line by line.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AxisKind {
    /// How strong a piece of evidence is: the reference's letter ladder.
    EvidenceGrade,
    /// A *unit* of work's grade, `F` … `SSS`, whose ceiling is a different thing from evidence strength.
    Grade,
    /// A certification tier, over the whole vector rather than its best part.
    Certification,
    /// An office rank — reached by election at the top, never by iterating.
    OrgRank,
    /// Where a claim's evidence came from: the three origin letters this project's records carry.
    TicketEvidence,
    /// A gate's status word.
    GateStatus,
    /// The strength scale of a support level.
    EvidenceLevel,
}

/// How many axes this build declares. Written down, and pinned to [`AxisKind::ALL`] by the length of the
/// table type.
pub const AXIS_COUNT: usize = 7;

/// The declared tokens of one ladder — or **the document that would declare them**.
///
/// There is deliberately no third case and no empty `Declared`: a ladder with no tokens is not an empty
/// ladder, it is a ladder this tree has not read, and saying so by name is the difference between an owed
/// value and a fabricated one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LadderValues {
    /// The tokens, in the ladder's own order, exactly as the declaring document states them.
    Declared(&'static [&'static str]),
    /// No declaration is in this tree. The string names **the document that would have to declare it**, so
    /// that "ours is missing" is checkable rather than a shrug.
    Owed(&'static str),
}

impl AxisKind {
    /// Every axis this build declares, in declaration order.
    pub const ALL: [AxisKind; AXIS_COUNT] = [
        AxisKind::EvidenceGrade,
        AxisKind::Grade,
        AxisKind::Certification,
        AxisKind::OrgRank,
        AxisKind::TicketEvidence,
        AxisKind::GateStatus,
        AxisKind::EvidenceLevel,
    ];

    /// How many axes this build declares. The test pins it to [`AxisKind::ALL`]'s length.
    pub const COUNT: usize = AXIS_COUNT;

    /// The recorded name — the wire word, and the word the gate prints.
    pub const fn name(self) -> &'static str {
        match self {
            AxisKind::EvidenceGrade => "evidence_grade",
            AxisKind::Grade => "grade",
            AxisKind::Certification => "certification",
            AxisKind::OrgRank => "org_rank",
            AxisKind::TicketEvidence => "ticket_evidence",
            AxisKind::GateStatus => "gate_status",
            AxisKind::EvidenceLevel => "evidence_level",
        }
    }

    /// The axis a declared name names, or nothing — an unfamiliar word is refused rather than guessed at.
    pub fn parse(name: &str) -> Option<Self> {
        AxisKind::ALL.iter().copied().find(|axis| axis.name() == name)
    }

    /// **The exhaustive match** (R-59.7's first enforcement): a new axis variant with no answer here
    /// **fails to compile**, because a red test can be deleted and a non-exhaustive match cannot be merged.
    pub const fn declared_values(self) -> LadderValues {
        match self {
            // The reference's own example in `docs/LEDGER.md` §59.5 item 9 is `[A, D, D, D]` — four
            // letters, `A` strongest — and C-73 measured a `strength()` of 4 for `A`. Two documents, one
            // four-token ladder, quoted rather than remembered.
            AxisKind::EvidenceGrade => LadderValues::Declared(&["A", "B", "C", "D"]),
            // `docs/LEDGER.md` §59.5 item 9: *"§19.2 adds the `F..SSS` grade ladder"* — nine rungs, `F`
            // the floor and `SSS` the ceiling. The eight intermediates are the letters between.
            AxisKind::Grade => LadderValues::Declared(&["F", "E", "D", "C", "B", "A", "S", "SS", "SSS"]),
            // The ticket-evidence letters, declared in C-73 §73.1 (`Oracle < ReadBack < SelfReport` was
            // the *defect* — an origin has no order) and C-103 §103.1 (the B rung).
            AxisKind::TicketEvidence => LadderValues::Declared(&["A", "B", "C"]),
            // U12's own vocabulary, owed here by name — and by path, so the debt is checkable.
            AxisKind::Certification => LadderValues::Owed(
                "the certification bands: docs/CIVILIAN-PORT.md's `resume.rs` row (allowed_strength, \
                 claims) — U12's vocabulary, U2's owed T016",
            ),
            AxisKind::OrgRank => LadderValues::Owed(
                "the office ladder: docs/CIVILIAN-PORT.md's `gov/mod.rs` row — U12's vocabulary, U2's \
                 owed T016",
            ),
            AxisKind::GateStatus => LadderValues::Owed(
                "the gate's own status words, which are quoted as words and never as bare ladder tokens \
                 (docs/LEDGER.md §59.5 item 9's distinction) — no document in this tree fixes the list",
            ),
            AxisKind::EvidenceLevel => LadderValues::Owed(
                "the support-level strength scale: `ala-system`'s `support.ts`, cited in docs/LEDGER.md \
                 §59.5 item 9 — U12's vocabulary, U2's owed T016",
            ),
        }
    }

    /// **No ladder is a quantity.** There is no axis here on which a percentage or a score would be a
    /// reading of the same thing in more convenient units — which is what [`refuse_reduction`] refuses.
    pub const fn is_a_quantity(self) -> bool {
        false
    }

    /// Whether this build can *read* the ladder at all: a declared token list, and nothing else.
    pub const fn is_declared(self) -> bool {
        matches!(self.declared_values(), LadderValues::Declared(_))
    }
}

/// **One ladder's values, as their own type** — rule 1, and the reason `refuse_interchange` has anything to
/// notice. A value that arrives as a bare [`String`] has no ladder, and this trait is what gives it one.
pub trait Ladder {
    /// The axis this value belongs to.
    const AXIS: AxisKind;
    /// Every value of this ladder, in declaration order and in the declared spelling.
    const TOKENS: &'static [&'static str];
    /// The token this value *is*.
    fn token(self) -> &'static str;
    /// What the token means, in this ladder's own words — **never** a claim about being stronger or
    /// weaker than another value. That is the property C-73 §73.1 records the absence of an order *for*.
    const DESCRIPTIONS: &'static [&'static str];
}

/// How strong a piece of evidence is: the reference's letter ladder, and **not** an order of importance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EvidenceGrade {
    A,
    B,
    C,
    D,
}

impl EvidenceGrade {
    /// Every grade, in the ladder's declared order.
    pub const ALL: [EvidenceGrade; 4] = [
        EvidenceGrade::A,
        EvidenceGrade::B,
        EvidenceGrade::C,
        EvidenceGrade::D,
    ];

    /// The grade a token names, or nothing. An unknown token is refused, never defaulted to the floor.
    pub fn parse(token: &str) -> Option<Self> {
        let token = token.trim();
        EvidenceGrade::ALL.iter().copied().find(|grade| grade.token() == token)
    }
}

impl Ladder for EvidenceGrade {
    const AXIS: AxisKind = AxisKind::EvidenceGrade;
    const TOKENS: &'static [&'static str] = &["A", "B", "C", "D"];
    fn token(self) -> &'static str {
        match self {
            EvidenceGrade::A => "A",
            EvidenceGrade::B => "B",
            EvidenceGrade::C => "C",
            EvidenceGrade::D => "D",
        }
    }
    const DESCRIPTIONS: &'static [&'static str] = &[
        "the strongest grade the reference's ladder states, and still a grade rather than a quantity",
        "a grade one letter down the same ladder",
        "a grade two letters down the same ladder",
        "the floor of the grade ladder, which is a grade and not an absence",
    ];
}

/// A **unit's** grade — `F` … `SSS`. Its ceiling is `SSS`, and reaching it is `graduate`, not a number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Grade {
    F,
    E,
    D,
    C,
    B,
    A,
    S,
    SS,
    SSS,
}

impl Grade {
    /// Every rung, floor first.
    pub const ALL: [Grade; 9] = [
        Grade::F,
        Grade::E,
        Grade::D,
        Grade::C,
        Grade::B,
        Grade::A,
        Grade::S,
        Grade::SS,
        Grade::SSS,
    ];

    /// The rung a token names, or nothing — the tokens are uppercase on purpose, so `sss` is refused.
    pub fn parse(token: &str) -> Option<Self> {
        let token = token.trim();
        Grade::ALL.iter().copied().find(|grade| grade.token() == token)
    }

    /// The rung one above this one, or nothing at the ceiling — *a transition needs its destination*
    /// (`docs/LEDGER.md` §59.5 item 9's rule 55, which the witness's tests name). The top rung has no
    /// destination, and answering with the top rung again would be a move that moved nothing.
    pub fn above(self) -> Option<Self> {
        let index = Grade::ALL.iter().position(|rung| *rung == self)?;
        Grade::ALL.get(index + 1).copied()
    }
}

impl Ladder for Grade {
    const AXIS: AxisKind = AxisKind::Grade;
    const TOKENS: &'static [&'static str] = &["F", "E", "D", "C", "B", "A", "S", "SS", "SSS"];
    fn token(self) -> &'static str {
        match self {
            Grade::F => "F",
            Grade::E => "E",
            Grade::D => "D",
            Grade::C => "C",
            Grade::B => "B",
            Grade::A => "A",
            Grade::S => "S",
            Grade::SS => "SS",
            Grade::SSS => "SSS",
        }
    }
    const DESCRIPTIONS: &'static [&'static str] = &[
        "the floor of the unit ladder",
        "one rung above the floor",
        "two rungs above the floor",
        "three rungs above the floor",
        "four rungs above the floor",
        "five rungs above the floor",
        "the first of the three top rungs",
        "the second of the three top rungs",
        "the ceiling, which is reached by graduating rather than by a score",
    ];
}

/// **Where an answer's evidence came from** — the ticket-evidence ladder, and the one ladder this slice
/// gives a type to, because every ballot's answer carries one.
///
/// It derives **no** `PartialOrd`/`Ord`, and that is the point (`docs/LEDGER.md` C-73 §73.1): a sort over
/// origins once produced a confidently wrong answer with no refusal anywhere, because three *sources* were
/// wearing a grade's name. An origin is not a ladder and must carry no order at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Origin {
    /// A machine answered: an oracle, in the record's own word — the rung a classifier's answer gets.
    Oracle,
    /// The machine looked at its own book and the book answered (`docs/LEDGER.md` C-103 §103.1).
    ReadBack,
    /// A party described its own work. Self-report is a source, not a weak oracle.
    SelfReport,
}

impl Origin {
    /// Every origin, in the ladder's declared order.
    pub const ALL: [Origin; 3] = [Origin::Oracle, Origin::ReadBack, Origin::SelfReport];

    /// The origin a letter names, or nothing — case matters, because a name that has to be guessed at is
    /// not a name this ladder declares.
    pub fn parse(token: &str) -> Option<Self> {
        Origin::ALL.iter().copied().find(|origin| origin.token() == token)
    }

    /// The three letters, in the same order as [`Origin::ALL`] — the ladder's own tokens, for a reader.
    pub const TOKENS: [&'static str; 3] = ["A", "B", "C"];
}

impl Ladder for Origin {
    const AXIS: AxisKind = AxisKind::TicketEvidence;
    const TOKENS: &'static [&'static str] = &["A", "B", "C"];
    fn token(self) -> &'static str {
        match self {
            Origin::Oracle => "A",
            Origin::ReadBack => "B",
            Origin::SelfReport => "C",
        }
    }
    const DESCRIPTIONS: &'static [&'static str] = &[
        "a machine answered the question about the artifact",
        "the machine read its own book back and the book answered",
        "a party described its own work",
    ];
}

/// **Rule 51, as a function**: two values from different ladders may not be compared.
///
/// `Ok` when both values are on one ladder — two members of one ladder compare freely and always did. The
/// refusal names **both ladders and both tokens**, because the failure it prevents is a token that comes to
/// mean two things at once: `A` is a grade *and* an origin, and a comparison across them is not a
/// comparison of anything.
pub fn refuse_interchange<A: Ladder, B: Ladder>(left: A, right: B) -> Result<(), Refusal> {
    if A::AXIS == B::AXIS {
        return Ok(());
    }
    Err(Refusal::new(
        RefusalKind::AxisInterchangeRefused,
        format!(
            "`{}` on the {} ladder and `{}` on the {} ladder: two ladders are not one scale, and reading \
             one ladder's token on another's is how a bare word comes to mean two things while looking \
             like a measurement",
            left.token(),
            A::AXIS.name(),
            right.token(),
            B::AXIS.name()
        ),
    ))
}

/// The two reductions a ladder refuses: rendering it as a percentage, or as a score.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Reduction {
    /// A share of a whole, which a vocabulary of words does not have.
    Percentage,
    /// A number that could be compared, averaged or thresholded — the shape a ladder is *not*.
    Score,
}

/// **A ladder never renders as a number** (`docs/LEDGER.md` C-97.2's inherited rule).
///
/// This returns a [`Refusal`] rather than a `Result`, because there is no input for which the answer is
/// *yes*: the function exists so that a caller who wants a number has to write the refusal into its own
/// code path. A percentage of a word is not a word, and a score is what a reader invents when it wants to
/// sort what a ladder deliberately does not order.
pub fn refuse_reduction(axis: AxisKind, reduction: Reduction) -> Refusal {
    let word = match reduction {
        Reduction::Percentage => "a percentage",
        Reduction::Score => "a score",
    };
    Refusal::new(
        RefusalKind::ScalarReductionRefused,
        format!(
            "the {} ladder is not a quantity, so it has {word}: a reduction would be a number this \
             vocabulary never stated, and a number is what a later reader would sort, average and \
             threshold as if it had been measured",
            axis.name()
        ),
    )
}

/// **Every axis that claims a bare token** — many-valued on purpose, because the contested set is the
/// finding: asking which *one* ladder owns `A` would answer a different question from the one that matters.
pub fn bare_token_axes(token: &str) -> Vec<AxisKind> {
    AxisKind::ALL
        .iter()
        .copied()
        .filter(|axis| match axis.declared_values() {
            LadderValues::Declared(values) => values.contains(&token),
            LadderValues::Owed(_) => false,
        })
        .collect()
}

/// **Round 5's rule: a bare ladder token is a bug**, in one of two ways.
///
/// A token **two or more declared ladders** claim is [`AMBIGUOUS`]: nothing in the text says which ladder
/// the reader meant, so the reading is a guess. A token **exactly one** ladder claims is [`UNQUALIFIED`]:
/// the ladder is inferable and the token is still bare, which is the case the source calls out separately
/// — the answer happens to be right and the *reading* is still not a reading. A token no declared ladder
/// claims is not this rule's business at all, and passing it is not an endorsement: it says only that this
/// rule has nothing to say.
///
/// [`AMBIGUOUS`]: RefusalKind::BareTokenRefused
/// [`UNQUALIFIED`]: RefusalKind::BareTokenRefused
pub fn refuse_bare(token: &str) -> Result<(), Refusal> {
    let claimants = bare_token_axes(token);
    match claimants.len() {
        0 => Ok(()),
        1 => Err(Refusal::new(
            RefusalKind::BareTokenRefused,
            format!(
                "`{token}` is a token of the {} ladder and is written with no ladder named: UNQUALIFIED — \
                 the reading is inferable and is still not a reading, because the next reader of the same \
                 word may infer the other one",
                claimants[0].name()
            ),
        )),
        _ => {
            let names: Vec<&'static str> = claimants.iter().map(|axis| axis.name()).collect();
            Err(Refusal::new(
                RefusalKind::BareTokenRefused,
                format!(
                    "`{token}` is claimed by {} ladders here ({}): AMBIGUOUS — a bare token that means \
                     {} things at once is a bug in the text, not a value to pick from",
                    names.len(),
                    names.join(", "),
                    names.len()
                ),
            ))
        }
    }
}

/// One row of the coverage table: what an axis's ladder is, and **where the values come from**.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CoverageRow {
    /// The axis this row describes.
    pub axis: AxisKind,
    /// The ladder's own name, as the declaring document spells it.
    pub ladder: &'static str,
    /// The values the row claims. Empty for an owed ladder, and empty is the *only* honest claim then.
    pub values: &'static [&'static str],
    /// The document that declares the values — for an owed ladder, the document that would have to.
    pub source: &'static str,
}

/// **The coverage table** — one row per axis, and the four owed axes say so.
pub const COVERAGE: [CoverageRow; AXIS_COUNT] = [
    CoverageRow {
        axis: AxisKind::EvidenceGrade,
        ladder: "the reference's letter grade",
        values: &["A", "B", "C", "D"],
        source: "docs/LEDGER.md §59.5 item 9 (`[A, D, D, D]`) with C-73 §73.1",
    },
    CoverageRow {
        axis: AxisKind::Grade,
        ladder: "the `F`…`SSS` unit grade",
        values: &["F", "E", "D", "C", "B", "A", "S", "SS", "SSS"],
        source: "docs/LEDGER.md §59.5 item 9: *\"§19.2 adds the `F..SSS` grade ladder\"*",
    },
    CoverageRow {
        axis: AxisKind::TicketEvidence,
        ladder: "the ticket-evidence letters",
        values: &["A", "B", "C"],
        source: "docs/LEDGER.md C-73 §73.1 and C-103 §103.1",
    },
    CoverageRow {
        axis: AxisKind::Certification,
        ladder: "the certification bands",
        values: &[],
        source: "owed: docs/CIVILIAN-PORT.md's `resume.rs` row (U12's vocabulary)",
    },
    CoverageRow {
        axis: AxisKind::OrgRank,
        ladder: "the office ladder",
        values: &[],
        source: "owed: docs/CIVILIAN-PORT.md's `gov/mod.rs` row (U12's vocabulary)",
    },
    CoverageRow {
        axis: AxisKind::GateStatus,
        ladder: "the gate's status words",
        values: &[],
        source: "owed: no document in this tree fixes the list (docs/LEDGER.md §59.5 item 9)",
    },
    CoverageRow {
        axis: AxisKind::EvidenceLevel,
        ladder: "the support-level strength scale",
        values: &[],
        source: "owed: `ala-system`'s `support.ts`, cited in docs/LEDGER.md §59.5 item 9",
    },
];

/// **The total pin** (R-59.7's second enforcement). The constant's *value* is meaningless; its **type** is
/// the assertion: a new axis with no coverage row fails to compile.
pub const COVERAGE_IS_TOTAL: [(); AXIS_COUNT] = [(); COVERAGE.len()];

/// **The anti-fabrication guard** (R-59.7's third enforcement): each row is compared against **the axis's
/// own declaration**, never against itself, so a row that is well formed and simply untrue is caught.
///
/// It refuses, by name: a row that claims no values for a ladder this build declares (an uncovered axis
/// wearing a row), a row for an axis this build does not declare the values of but claims some anyway (a
/// ladder that looks checked and is not in this tree), a row whose values are not the axis's own, two rows
/// for one axis, and an axis with no row at all. It collects **every** problem rather than the first.
pub fn check_coverage(rows: &[CoverageRow]) -> Result<(), RefusalSet> {
    let mut refusals = RefusalSet::new();
    for row in rows {
        if row.values.is_empty() {
            if let LadderValues::Declared(values) = row.axis.declared_values() {
                refusals.refuse(
                    RefusalKind::UncoveredAxis,
                    format!(
                        "the row for `{}` declares no values, and this build declares {}: a ladder with no \
                         representation is a failure rather than a todo",
                        row.axis.name(),
                        values.len()
                    ),
                );
            }
        } else {
            match row.axis.declared_values() {
                LadderValues::Declared(values) => {
                    if values != row.values {
                        refusals.refuse(
                            RefusalKind::FabricatedAxis,
                            format!(
                                "the row for `{}` claims {row_values:?} and the axis declares {values:?}: a \
                                 row that is well formed and untrue looks checked, which is worse than an \
                                 empty one",
                                row.axis.name(),
                                row_values = row.values,
                            ),
                        );
                    }
                    if has_duplicate(row.values) {
                        refusals.refuse(
                            RefusalKind::FabricatedAxis,
                            format!(
                                "the row for `{}` names one value twice: a ladder is a set of tokens and a \
                                 repeated token is a second value that was never declared",
                                row.axis.name()
                            ),
                        );
                    }
                }
                LadderValues::Owed(document) => refusals.refuse(
                    RefusalKind::FabricatedAxis,
                    format!(
                        "the row for `{}` claims {} values for a ladder this tree does not declare — {document} \
                         — so the row cannot be checked against anything and is refused rather than counted",
                        row.axis.name(),
                        row.values.len(),
                    ),
                ),
            }
        }
    }

    let mut claimed: Vec<AxisKind> = rows.iter().map(|row| row.axis).collect();
    claimed.sort_unstable();
    claimed.dedup();
    if claimed.len() != rows.len() {
        refusals.refuse(
            RefusalKind::UncoveredAxis,
            "two rows describe one axis, so one of them describes what the other already describes"
                .to_string(),
        );
    }
    for axis in AxisKind::ALL {
        if !rows.iter().any(|row| row.axis == axis) {
            refusals.refuse(
                RefusalKind::UncoveredAxis,
                format!(
                    "`{}` has no coverage row: an axis whose representation is missing is refused by name \
                     rather than left to be noticed",
                    axis.name()
                ),
            );
        }
    }

    if refusals.is_empty() {
        Ok(())
    } else {
        Err(refusals)
    }
}

/// Whether a list names one token twice.
fn has_duplicate(values: &[&'static str]) -> bool {
    values
        .iter()
        .enumerate()
        .any(|(index, value)| values[..index].contains(value))
}

/// What the tree can say about its own seven axes, as counted values: how many ladders are declared, and
/// **which are owed** — named, so the gate prints a debt rather than a gap.
pub fn coverage_summary() -> (usize, Vec<&'static str>) {
    let declared = AxisKind::ALL.iter().filter(|axis| axis.is_declared()).count();
    let owed = AxisKind::ALL
        .iter()
        .filter(|axis| !axis.is_declared())
        .map(|axis| axis.name())
        .collect();
    (declared, owed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_seven_axes_are_closed_and_every_name_round_trips() {
        assert_eq!(AxisKind::ALL.len(), AxisKind::COUNT);
        for (index, axis) in AxisKind::ALL.iter().enumerate() {
            assert!(
                AxisKind::ALL[..index].iter().all(|other| other != axis),
                "{} is declared twice",
                axis.name()
            );
            assert_eq!(AxisKind::parse(axis.name()), Some(*axis));
        }
        assert_eq!(AxisKind::parse("not_an_axis"), None);
    }

    #[test]
    fn three_ladders_are_declared_and_four_are_owed_by_name() {
        let (declared, owed) = coverage_summary();
        assert_eq!(declared, 3);
        assert_eq!(
            owed,
            vec!["certification", "org_rank", "gate_status", "evidence_level"]
        );
        // **An owed ladder names its document**: a debt without a source is a guess.
        for axis in AxisKind::ALL {
            if let LadderValues::Owed(document) = axis.declared_values() {
                assert!(
                    document.starts_with("the") || document.starts_with("no document"),
                    "{} names no document: {document}",
                    axis.name()
                );
                assert!(document.len() > 20, "{} is owed to nobody in particular", axis.name());
            }
        }
    }

    #[test]
    fn a_token_two_ladders_claim_is_ambiguous_and_every_claimant_is_named() {
        let claimants = bare_token_axes("A");
        assert_eq!(
            claimants,
            vec![
                AxisKind::EvidenceGrade,
                AxisKind::Grade,
                AxisKind::TicketEvidence
            ],
            "A is a token of three declared ladders here and means three things"
        );
        let refusal = refuse_bare("A").expect_err("a contested token may not be read bare");
        assert_eq!(refusal.kind(), RefusalKind::BareTokenRefused);
        assert!(refusal.detail().contains("AMBIGUOUS"), "{refusal}");
        for axis in claimants {
            assert!(refusal.detail().contains(axis.name()), "{refusal}");
        }
    }

    #[test]
    fn a_token_one_ladder_claims_is_unqualified_and_still_a_bug() {
        let refusal = refuse_bare("SSS").expect_err("a bare ladder token is a bug");
        assert_eq!(refusal.kind(), RefusalKind::BareTokenRefused);
        assert!(refusal.detail().contains("UNQUALIFIED"), "{refusal}");
        assert!(!refusal.detail().contains("AMBIGUOUS"), "{refusal}");
        assert!(refusal.detail().contains("grade"), "{refusal}");
    }

    #[test]
    fn a_token_no_declared_ladder_claims_is_not_this_rules_business() {
        // `gold` is a *certification* token, and this build does not declare that ladder — so the rule has
        // nothing to say about it, and saying nothing is not the same as endorsing it.
        assert!(refuse_bare("gold").is_ok());
        assert!(refuse_bare("the").is_ok());
    }

    #[test]
    fn comparing_across_ladders_refuses_and_names_both_of_them() {
        let refusal = refuse_interchange(EvidenceGrade::A, Origin::Oracle)
            .expect_err("a grade and an origin are different ladders");
        assert_eq!(refusal.kind(), RefusalKind::AxisInterchangeRefused);
        assert!(refusal.detail().contains("evidence_grade"), "{refusal}");
        assert!(refusal.detail().contains("ticket_evidence"), "{refusal}");
        // A unit grade and a ticket origin both carry `B`, and the refusal says so by token.
        assert!(refusal.detail().contains("`A`"), "{refusal}");
        let same_letter = refuse_interchange(Origin::ReadBack, Grade::B)
            .expect_err("`B` is two different things on two ladders");
        assert!(same_letter.detail().contains("`B`"), "{same_letter}");
    }

    #[test]
    fn two_values_of_one_ladder_compare_freely() {
        assert!(refuse_interchange(EvidenceGrade::A, EvidenceGrade::D).is_ok());
        assert!(refuse_interchange(Origin::Oracle, Origin::SelfReport).is_ok());
        assert!(refuse_interchange(Grade::F, Grade::SSS).is_ok());
    }

    #[test]
    fn no_ladder_is_a_quantity_and_every_reduction_refuses() {
        let mut refusals = 0;
        for axis in AxisKind::ALL {
            assert!(!axis.is_a_quantity(), "{} claims to be a quantity", axis.name());
            let percentage = refuse_reduction(axis, Reduction::Percentage);
            assert_eq!(percentage.kind(), RefusalKind::ScalarReductionRefused);
            assert!(percentage.detail().contains("not a quantity"), "{percentage}");
            let score = refuse_reduction(axis, Reduction::Score);
            assert!(score.detail().contains("not a score") || score.detail().contains("a score"), "{score}");
            refusals += 2;
        }
        assert_eq!(refusals, AxisKind::COUNT * 2);
    }

    #[test]
    fn an_origin_carries_no_order_and_claims_none() {
        // A test cannot assert the *absence* of a trait, so it holds the consequences that matter: three
        // distinct sources, letters that are labels, and no description that ranks one against another.
        assert_eq!(Origin::ALL.len(), 3);
        assert_eq!(
            Origin::ALL.iter().map(|origin| origin.token()).collect::<Vec<_>>(),
            Origin::TOKENS.to_vec()
        );
        for description in <Origin as Ladder>::DESCRIPTIONS {
            let lowered = description.to_lowercase();
            for word in ["strong", "weak", "greater", "less", "better", "worse", "rank"] {
                assert!(
                    !lowered.contains(word),
                    "an origin's own description ranks it: {description}"
                );
            }
        }
        assert_eq!(Origin::parse("b"), None, "the letters are uppercase on purpose");
    }

    #[test]
    fn the_coverage_table_describes_every_axis_exactly_once() {
        assert_eq!(COVERAGE_IS_TOTAL.len(), AxisKind::ALL.len());
        check_coverage(&COVERAGE).expect("the declared table must describe every axis once");
        let owed: Vec<&str> = COVERAGE
            .iter()
            .filter(|row| row.values.is_empty())
            .map(|row| row.axis.name())
            .collect();
        assert_eq!(owed.len(), 4);
    }

    #[test]
    fn a_fabricated_row_is_refused_because_it_looks_checked() {
        let forged = CoverageRow {
            axis: AxisKind::EvidenceGrade,
            ladder: "the reference's letter grade",
            values: &["A", "B", "C", "E"],
            source: "invented by a test",
        };
        let refused = check_coverage(&[forged]).expect_err("a forged row must be refused");
        assert!(refused.contains(RefusalKind::FabricatedAxis), "{refused}");
        assert!(
            refused
                .all()
                .iter()
                .any(|refusal| refusal.detail().contains("looks checked")),
            "{refused}"
        );

        // The same forgery for an **owed** ladder: a row cannot cover what this tree does not declare.
        let claiming_an_owed = CoverageRow {
            axis: AxisKind::OrgRank,
            ladder: "the office ladder",
            values: &["member", "senior"],
            source: "invented by a test",
        };
        let refused = check_coverage(&[claiming_an_owed]).expect_err("an owed ladder has no values");
        assert!(refused.contains(RefusalKind::FabricatedAxis), "{refused}");
    }

    #[test]
    fn an_axis_with_no_row_is_a_failure_not_a_todo() {
        let partial: Vec<CoverageRow> = COVERAGE
            .iter()
            .copied()
            .filter(|row| row.axis != AxisKind::OrgRank)
            .collect();
        let refused = check_coverage(&partial).expect_err("a missing row must be refused");
        assert!(refused.contains(RefusalKind::UncoveredAxis), "{refused}");
        assert!(
            refused.all().iter().any(|refusal| refusal.detail().contains("org_rank")),
            "{refused}"
        );
        // And a row for one axis twice is the same class of hole.
        let mut doubled = partial.clone();
        doubled.push(COVERAGE[0]);
        doubled.push(COVERAGE[6]);
        let refused = check_coverage(&doubled).expect_err("two rows for one axis describe one thing");
        assert!(refused.contains(RefusalKind::UncoveredAxis), "{refused}");
    }

    #[test]
    fn a_rung_above_the_ceiling_has_no_destination() {
        // Rule 55's predicate, as the ladder itself states it: a transition needs somewhere to go.
        assert_eq!(Grade::F.above(), Some(Grade::E));
        assert_eq!(Grade::SS.above(), Some(Grade::SSS));
        assert_eq!(Grade::SSS.above(), None, "the ceiling has no destination");
    }
}
