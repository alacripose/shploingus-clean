//! **The ballot: what a package must declare for its work to be judgeable.**
//!
//! # Why the work needs a declaration, and not just bytes
//!
//! A package so far says *what to build*. It cannot say **what question the work answers**, and a
//! judgment needs one: the decision layer's three shapes (`choice`, `noul`, `score`) are each a
//! different instrument, and *"does this belong to the recipe?"* is not the same question as *"how
//! much of it is done?"* — different answers, different ladders, and a judge asked the wrong one
//! returns a number that looks fine.
//!
//! So the package carries a **ballot**: the question, the terms it is asked in, and the artifact
//! the answer is about. That is the whole extension, and it is deliberately the smallest one that
//! makes a judgment possible:
//!
//! | field | why it is not optional |
//! |---|---|
//! | `ballot` | the id an answer comes back under, and the row a finding cites |
//! | `claim` | the ticket or ruling the work serves — an unattributed claim is not a claim |
//! | `artifact` | **the bytes the answer is about**, relative to the workspace, so the judgment can re-hash them |
//! | `instruction` | how to judge the state; the model receives this and nothing else about intent |
//! | `kind` | `choice`, `noul` or `score` — the instrument |
//! | `option` / `level` | the terms of the question: a choice's options, a score's ordered rungs |
//! | `presenter` | who is accountable for the presentation (upstream's rule: a presentation with no accountable actor is refused) |
//! | `max-age` | how many ticks an answer stays current — **stale is refused, never grandfathered** |
//! | `passes` / `floor` / `threshold` | **where the answer stops passing** — see below |
//!
//! # The floor, and why the ballot must carry it
//!
//! An answer is not a verdict. *"The model chose *leave it*"* says nothing about whether that is
//! good work until someone declares which answers count, and that declaration is a ruling about the
//! work — not something a judge may invent after seeing the answer. So the ballot carries a floor,
//! per instrument: a `choice` names the option that passes (`passes`), a `score` names the lowest
//! rung that passes (`floor`), and a `noul` may name its threshold (`threshold`, 0.5 by default
//! because a probability of yes has a natural midpoint and a chosen one would need a reason).
//!
//! **The floor is declared before the answer exists.** That ordering is the whole point: a floor
//! chosen after reading the answer is a floor chosen to fit it.
//!
//! # What it refuses, and why refusing is the point
//!
//! A ballot that cannot be judged is refused **here**, at parse, rather than becoming a question
//! the model answers anyway: a `choice` with no options, a `score` over one level, an unknown kind,
//! an empty instruction, a missing artifact or claim. Every one of those is a question with no
//! terms, and a model asked a question with no terms still returns a confident-looking number. That
//! is the failure this parser exists to make impossible.
//!
//! # The format
//!
//! `key\ttab\tvalue` lines, the shape every gate report in this tree uses, under a declared header.
//! A ballot opens with `ballot` and closes when the next one opens, so no terminator can be
//! forgotten:
//!
//! ```text
//! shploingus.ballot/1
//! ballot\tbelongs
//! claim\tT-12
//! artifact\tcrates/voxel-mcp/src/lib.rs
//! kind\tnoul
//! instruction\tdoes this belong to the recipe it claims, with nothing invented?
//! presenter\tala
//! max-age\t1200
//! ```
//!
//! Every `\t` above is a literal tab, the same one the sentence before the block writes: a
//! delimiter and not spacing, so it is written out where nothing can see it. A real tab inside a
//! doc comment is whitespace a reader cannot count, which is why the linter refuses one.

use voxel_model::refusal::{Refusal, RefusalKind, RefusalSet};

/// The declared header. A file that does not carry it is not a ballot file.
pub const HEADER: &str = "shploingus.ballot/1";

/// Every key a ballot term may carry, in one place.
///
/// **Declared so a reader can find where a block ends without knowing the grammar by heart**: the
/// recipe's block reader stops at the first key that is not here, which is how `read_window` decides
/// its own block ended. A new term is one edit here and one in [`parse_body`]'s dispatch, and the
/// test beside this file pins that the two agree.
pub const FIELDS: [&str; 10] = [
    "ballot",
    "claim",
    "artifact",
    "kind",
    "instruction",
    "presenter",
    "max-age",
    "passes",
    "floor",
    "threshold",
];

/// The repeated terms, which may appear more than once in a ballot — kept apart from [`FIELDS`]
/// because a duplicate check on them would refuse a three-option choice.
pub const REPEATED_FIELDS: [&str; 2] = ["option", "level"];

/// The versions this parser accepts.
pub const ACCEPTED_VERSIONS: &[&str] = &[HEADER];

/// How many ballots one file may declare. A bound, not a target: the classifier's cost follows the
/// question structure, so a file that declares thousands is a file nobody meant to run.
pub const MAX_BALLOTS: usize = 256;

/// How many terms one question may carry — the same bound `recipe` puts on its rows.
pub const MAX_TERMS: usize = 32;

/// The most bytes one instruction may carry. Long instructions are not better questions, and a
/// model's context is not free: this is the one term whose size a caller controls.
pub const MAX_INSTRUCTION_BYTES: usize = 4096;

/// The instrument a question is asked with. **The decision layer's own three, by name.**
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    /// Does it belong? A probability of yes.
    Noul,
    /// One option from a fixed set.
    Choice,
    /// A position on an ordered scale, weakest to strongest.
    Score,
}

impl Kind {
    pub const ALL: [Kind; 3] = [Kind::Noul, Kind::Choice, Kind::Score];

    pub const fn name(self) -> &'static str {
        match self {
            Kind::Noul => "noul",
            Kind::Choice => "choice",
            Kind::Score => "score",
        }
    }

    /// Parse a kind name. **Unknown is refused rather than defaulted**, because a defaulted kind is
    /// a question asked with an instrument nobody chose.
    pub fn parse(word: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.name() == word.trim())
            .ok_or_else(|| {
                format!(
                    "{word:?} is not a question kind; the kinds are {}",
                    Self::ALL.map(Kind::name).join(", ")
                )
            })
    }
}

/// One declared ballot: the question, its terms, and what the answer will be about.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ballot {
    /// The id the answer comes back under.
    pub id: String,
    /// The ticket or ruling this work serves.
    pub claim: String,
    /// The artifact the answer is about, relative to the workspace root.
    pub artifact: String,
    /// The instrument.
    pub kind: Kind,
    /// How to judge the state.
    pub instruction: String,
    /// A choice's options, in declaration order. Empty for the other kinds.
    pub options: Vec<String>,
    /// A score's rungs, weakest to strongest. Empty for the other kinds.
    pub levels: Vec<String>,
    /// Who is accountable for the presentation.
    pub presenter: String,
    /// How many ticks an answer stays current. Zero means **the declaration refused to say**, and a
    /// judgment that cannot be aged is refused rather than assumed young.
    pub max_age: u64,
    /// For a `choice`: the option key that counts as passing.
    pub passes: Option<String>,
    /// For a `score`: the lowest rung index that counts as passing.
    pub floor: Option<u32>,
    /// For a `noul`: the probability of yes at or above which the answer passes, **in
    /// thousandths**: 500 is one half, 900 is nine tenths.
    ///
    /// Thousandths rather than a float, and for the reason `opacity` in this repository declares
    /// its own bound the same way: a declared floor is part of a record, two records that say
    /// `0.1 + 0.2` and `0.3` are the same intention and different bytes, and a record with a float
    /// in it cannot be compared for equality at all — which is how this field was caught, because
    /// adding it to a `Recipe` broke that type's `Eq`.
    pub threshold_thousandths: u32,
}

/// The threshold a `noul` passes at when the ballot names none: one half, in thousandths.
pub const DEFAULT_THRESHOLD_THOUSANDTHS: u32 = 500;

/// The top of the threshold scale: a probability of yes, in thousandths.
pub const MAX_THRESHOLD_THOUSANDTHS: u32 = 1000;

impl Ballot {
    /// The terms of this question, as the classifier needs them: one list, in order, whichever kind
    /// declared it — a choice's options or a score's levels. **One accessor for two fields**, so a
    /// caller cannot pass the wrong one and get a question with terms in the wrong roles.
    pub fn terms(&self) -> &[String] {
        match self.kind {
            Kind::Choice => &self.options,
            Kind::Score => &self.levels,
            Kind::Noul => &[],
        }
    }

    /// Whether an answer to this question can be a judgment at all: every field the instrument
    /// needs is present.
    pub fn refusals(&self) -> RefusalSet {
        let mut refusals = RefusalSet::new();
        for (field, value) in [
            ("ballot", &self.id),
            ("claim", &self.claim),
            ("artifact", &self.artifact),
            ("instruction", &self.instruction),
            ("presenter", &self.presenter),
        ] {
            if value.trim().is_empty() {
                refusals.refuse(
                    RefusalKind::MalformedRecord,
                    format!(
                        "{}: `{field}` is empty, and a question with no {field} cannot be answered \
                         as this work's",
                        self.id_or_unnamed()
                    ),
                );
            }
        }
        if self.instruction.len() > MAX_INSTRUCTION_BYTES {
            refusals.refuse(
                RefusalKind::MalformedRecord,
                format!(
                    "{}: the instruction is {} bytes and the limit is {MAX_INSTRUCTION_BYTES}; a \
                     longer question is not a better one",
                    self.id_or_unnamed(),
                    self.instruction.len()
                ),
            );
        }
        match self.kind {
            Kind::Choice if self.options.len() < 2 => {
                refusals.refuse(
                    RefusalKind::MalformedRecord,
                    format!(
                        "{}: a choice over {} option(s) is not a choice; the model would be given \
                         one answer and asked to pick it",
                        self.id_or_unnamed(),
                        self.options.len()
                    ),
                );
            }
            Kind::Score if self.levels.len() < 2 => {
                refusals.refuse(
                    RefusalKind::MalformedRecord,
                    format!(
                        "{}: a score over {} level(s) is not a scale",
                        self.id_or_unnamed(),
                        self.levels.len()
                    ),
                );
            }
            _ => {}
        }
        if self.terms().len() > MAX_TERMS {
            refusals.refuse(
                RefusalKind::MalformedRecord,
                format!(
                    "{}: {} term(s) exceeds the limit of {MAX_TERMS}",
                    self.id_or_unnamed(),
                    self.terms().len()
                ),
            );
        }
        if self.max_age == 0 {
            refusals.refuse(
                RefusalKind::MalformedRecord,
                format!(
                    "{}: `max-age` is 0, which is not a window; an answer nobody declared the age \
                     of is refused rather than assumed fresh",
                    self.id_or_unnamed()
                ),
            );
        }
        // **The floor, per instrument.** An answer with no declared floor can be described and not
        // judged, and a judge that invents the floor has invented a ruling.
        match self.kind {
            Kind::Choice => match &self.passes {
                None => {
                    refusals.refuse(
                        RefusalKind::MalformedRecord,
                        format!(
                            "{}: a choice with no `passes` says which answer is good work only \
                             after someone has seen the answer",
                            self.id_or_unnamed()
                        ),
                    );
                }
                Some(passing) if !self.options.iter().any(|option| option == passing) => {
                    refusals.refuse(
                        RefusalKind::FabricatedFeature,
                        format!(
                            "{}: `passes` names {passing:?}, which is not one of the {} option(s) \
                             declared, so no answer could ever pass",
                            self.id_or_unnamed(),
                            self.options.len()
                        ),
                    );
                }
                Some(_) => {}
            },
            Kind::Score => match self.floor {
                None => {
                    refusals.refuse(
                        RefusalKind::MalformedRecord,
                        format!(
                            "{}: a score with no `floor` has no rung that counts as passing",
                            self.id_or_unnamed()
                        ),
                    );
                }
                Some(floor) if floor as usize >= self.levels.len() => {
                    refusals.refuse(
                        RefusalKind::FabricatedFeature,
                        format!(
                            "{}: `floor` is {floor} and the scale has {} rung(s); a floor above the \
                             top rung is a question nothing can pass",
                            self.id_or_unnamed(),
                            self.levels.len()
                        ),
                    );
                }
                Some(_) => {}
            },
            // The threshold's own bound is enforced at parse, where the text is turned into a
            // number: a value that cannot be a probability never reaches a ballot.
            Kind::Noul => {}
        }
        refusals
    }

    fn id_or_unnamed(&self) -> &str {
        if self.id.trim().is_empty() {
            "<unnamed ballot>"
        } else {
            &self.id
        }
    }

    /// One `key<TAB>value` line per field, in declaration order — **the same text a reader parses**,
    /// so a ballot written and a ballot read cannot disagree.
    pub fn render(&self) -> String {
        let mut out = format!(
            "ballot\t{}\nclaim\t{}\nartifact\t{}\nkind\t{}\ninstruction\t{}\npresenter\t{}\nmax-age\t{}\n",
            self.id,
            self.claim,
            self.artifact,
            self.kind.name(),
            self.instruction,
            self.presenter,
            self.max_age
        );
        for option in &self.options {
            out.push_str(&format!("option\t{option}\n"));
        }
        for level in &self.levels {
            out.push_str(&format!("level\t{level}\n"));
        }
        // The floor is rendered only where it applies, so a reader of the text sees which
        // instrument it was declared for.
        match self.kind {
            Kind::Choice => {
                if let Some(passing) = &self.passes {
                    out.push_str(&format!("passes\t{passing}\n"));
                }
            }
            Kind::Score => {
                if let Some(floor) = self.floor {
                    out.push_str(&format!("floor\t{floor}\n"));
                }
            }
            Kind::Noul => {
                if self.threshold_thousandths != DEFAULT_THRESHOLD_THOUSANDTHS {
                    out.push_str(&format!("threshold\t{}\n", self.threshold_thousandths));
                }
            }
        }
        out
    }
}

/// Close the open ballot, if there is one: refuse it, or take it. **One function, called at both
/// closings** — a second ballot is closed by the next `ballot` line, and the last by the end of the
/// file, and the two paths must not disagree about what a valid ballot is. The first draft
/// duplicated the body and the duplicate-id check existed on only one of them, which its own test
/// caught.
fn close(open: &mut Option<Ballot>, ballots: &mut Vec<Ballot>, refusals: &mut RefusalSet) {
    let Some(done) = open.take() else {
        return;
    };
    let found = done.refusals();
    if !found.is_empty() {
        for refusal in found.all() {
            refusals.push(refusal.clone());
        }
        return;
    }
    if ballots.iter().any(|seen| seen.id == done.id) {
        refusals.refuse(
            RefusalKind::MalformedRecord,
            format!(
                "{}: two ballots share this id, so an answer to one would be read as an answer to the \
                 other",
                done.id
            ),
        );
        return;
    }
    ballots.push(done);
}

/// Parse a ballot file. `Err` is **every** problem, not the first: a caller fixes one round of them.
pub fn parse(text: &str) -> Result<Vec<Ballot>, RefusalSet> {
    let mut refusals = RefusalSet::new();
    // The header is the first non-blank line that carries anything, and it is not optional.
    let mut saw_header = false;
    let mut body_from = 0usize;
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        saw_header = line.trim() == HEADER;
        if !saw_header {
            refusals.refuse(
                RefusalKind::MalformedRecord,
                format!(
                    "the file opens with {:?} and a ballot file opens with {HEADER:?}",
                    line.trim()
                ),
            );
        }
        body_from = index + 1;
        break;
    }
    if !saw_header {
        return Err(refusals);
    }
    let body: String = text.lines().skip(body_from).collect::<Vec<_>>().join("\n");
    parse_body(&body)
}

/// The ballots in a stream of terms, **without a header** — the entry point a recipe uses.
///
/// A recipe declares its own header (`shploingus.recipe/3`), so a ballot block inside one cannot
/// carry a second: two headers in one file are two formats claiming the same bytes. The grammar is
/// the same either way and **this is the same parser**, not a second one — the rule `apidoc` states
/// for prose applies to a format: nothing is summarised here, and nothing is reimplemented.
///
/// **Indentation is presentational**, the rule `read_window` already applies: the terms are written
/// `\tkey\tvalue` so a person can read the block's shape, and the leading tab is not part of the key.
pub fn parse_body(text: &str) -> Result<Vec<Ballot>, RefusalSet> {
    let mut refusals = RefusalSet::new();
    let mut ballots: Vec<Ballot> = Vec::new();
    let mut open: Option<Ballot> = None;

    for (number, line) in text.lines().enumerate() {
        let line = line.trim_end_matches(['\r', '\n']).trim_start_matches('\t');
        if line.trim().is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once('\t') else {
            refusals.refuse(
                RefusalKind::MalformedRecord,
                format!("line {}: {line:?} is not a `key<TAB>value` term", number + 1),
            );
            continue;
        };
        let value = value.to_string();
        match key {
            "ballot" => {
                close(&mut open, &mut ballots, &mut refusals);
                open = Some(Ballot {
                    id: value,
                    claim: String::new(),
                    artifact: String::new(),
                    kind: Kind::Noul,
                    instruction: String::new(),
                    options: Vec::new(),
                    levels: Vec::new(),
                    presenter: String::new(),
                    max_age: 0,
                    passes: None,
                    floor: None,
                    threshold_thousandths: DEFAULT_THRESHOLD_THOUSANDTHS,
                });
            }
            other => {
                let Some(open) = open.as_mut() else {
                    refusals.refuse(
                        RefusalKind::MalformedRecord,
                        format!(
                            "line {}: `{other}` appears before any `ballot` opens",
                            number + 1
                        ),
                    );
                    continue;
                };
                match other {
                    "claim" => open.claim = value,
                    "artifact" => open.artifact = value.replace('\\', "/"),
                    "instruction" => open.instruction = value,
                    "presenter" => open.presenter = value,
                    "option" => open.options.push(value),
                    "level" => open.levels.push(value),
                    "passes" => open.passes = Some(value),
                    "floor" => match value.trim().parse::<u32>() {
                        Ok(rung) => open.floor = Some(rung),
                        Err(_) => {
                            refusals.refuse(
                                RefusalKind::MalformedRecord,
                                format!("{}: {value:?} is not a rung index", open.id),
                            );
                        }
                    },
                    "threshold" => match value.trim().parse::<u32>() {
                        // **Bounded where it is read**, so a value that cannot be a probability
                        // never becomes a ballot that a judge would compare against.
                        Ok(threshold) if threshold <= MAX_THRESHOLD_THOUSANDTHS => {
                            open.threshold_thousandths = threshold;
                        }
                        Ok(threshold) => {
                            refusals.refuse(
                                RefusalKind::MalformedRecord,
                                format!(
                                    "{}: `threshold` is {threshold} and a probability of yes lies \
                                     in 0..={MAX_THRESHOLD_THOUSANDTHS} thousandths",
                                    open.id
                                ),
                            );
                        }
                        Err(_) => {
                            refusals.refuse(
                                RefusalKind::MalformedRecord,
                                format!(
                                    "{}: {value:?} is not a probability in thousandths",
                                    open.id
                                ),
                            );
                        }
                    },
                    "max-age" => match value.trim().parse::<u64>() {
                        Ok(ticks) => open.max_age = ticks,
                        Err(_) => {
                            refusals.refuse(
                                RefusalKind::MalformedRecord,
                                format!(
                                    "{}: `max-age` is {value:?}, which is not a tick count",
                                    open.id
                                ),
                            );
                        }
                    },
                    "kind" => match Kind::parse(&value) {
                        Ok(kind) => open.kind = kind,
                        Err(why) => {
                            refusals.refuse(RefusalKind::MalformedRecord, why);
                        }
                    },
                    unknown => {
                        refusals.refuse(
                            RefusalKind::FabricatedFeature,
                            format!(
                                "{}: `{unknown}` is not a ballot term; the terms are ballot, claim, \
                                 artifact, kind, instruction, presenter, max-age, option, level, \
                                 passes, floor, threshold",
                                open.id
                            ),
                        );
                    }
                }
            }
        }
    }
    // The last ballot closes at the end of the file, through the same function as every other.
    close(&mut open, &mut ballots, &mut refusals);

    if ballots.len() > MAX_BALLOTS {
        refusals.refuse(
            RefusalKind::MalformedRecord,
            format!(
                "{} ballots declared and the limit is {MAX_BALLOTS}; a file that declares this many \
                 questions is a file nobody meant to run",
                ballots.len()
            ),
        );
    }
    if ballots.is_empty() {
        refusals.refuse(
            RefusalKind::NoReading,
            "the file declares no ballot, so there is nothing to judge and nothing to report — not \
             a pass"
                .to_string(),
        );
    }
    refusals.into_result(ballots)
}

/// Read a ballot file off disk, with the reason on failure.
pub fn read(path: &std::path::Path) -> Result<Vec<Ballot>, RefusalSet> {
    let text = std::fs::read_to_string(path).map_err(|failure| {
        let mut refusals = RefusalSet::new();
        refusals.refuse(
            RefusalKind::MalformedRecord,
            format!("{} cannot be read: {failure}", path.display()),
        );
        refusals
    })?;
    parse(&text)
}

/// One refusal of the kind a parsed file would carry, for a caller that has the facts already.
pub fn refuse(kind: RefusalKind, detail: impl Into<String>) -> Refusal {
    Refusal::owned(kind, detail)
}

/// Whether a term's key belongs to a ballot block. The recipe's reader asks this to find where the
/// block ends, so the answer is the same one [`parse_body`] uses rather than a second opinion.
pub fn is_field(key: &str) -> bool {
    FIELDS.contains(&key) || REPEATED_FIELDS.contains(&key)
}

#[cfg(test)]
mod tests {
    use super::*;

    const A_BALLOT: &str = "shploingus.ballot/1\n\
        ballot\tbelongs\n\
        claim\tT-12\n\
        artifact\tcrates/voxel-mcp/src/lib.rs\n\
        kind\tnoul\n\
        instruction\tdoes this belong to the recipe it claims?\n\
        presenter\tala\n\
        max-age\t1200\n";

    #[test]
    fn the_declared_fields_are_the_keys_the_parser_dispatches() {
        // **One list, two readers**, and this asserts they agree: every declared field is a key the
        // parser knows, and every key the parser knows is declared. A term added to the match and
        // not to `FIELDS` would end a block early for the recipe's reader, which is the failure this
        // test exists to make impossible.
        let source = include_str!("ballot.rs");
        for field in FIELDS.iter().chain(REPEATED_FIELDS.iter()) {
            match *field {
                // `ballot` opens a block rather than appearing inside one.
                "ballot" => continue,
                other => assert!(
                    source.contains(&format!("\"{other}\" =>")),
                    "{other} is declared in FIELDS and the parser has no arm for it"
                ),
            }
        }
        for field in ["claim", "artifact", "kind", "instruction", "presenter", "max-age"] {
            assert!(FIELDS.contains(&field), "{field} is parsed and not declared");
        }
        assert!(is_field("option") && is_field("level"), "a repeated term is still a field");
        assert!(!is_field("window"), "a recipe's own key is not a ballot term");
    }

    #[test]
    fn a_body_with_no_header_parses_the_same_terms_a_file_does() {
        // The recipe's entry point: the same grammar, minus the header it already carries.
        let body = A_BALLOT.lines().skip(1).collect::<Vec<_>>().join("\n");
        let from_body = parse_body(&body).expect("a header-less body parses");
        let from_file = parse(A_BALLOT).expect("the whole file parses");
        assert_eq!(from_body, from_file, "the two entry points disagree about the same terms");
        // And the body refuses what a file refuses, with the same reasons.
        let why = parse_body("ballot\tbelongs\nclaim\tT-1\nkind\tnoul\nmax-age\t60\n")
            .expect_err("a body missing its artifact and presenter is refused");
        assert!(format!("{why}").contains("artifact"), "{why}");
    }

    #[test]
    fn a_block_written_with_presentational_indentation_reads_the_same() {
        // A recipe writes block terms `\tkey\tvalue`; the leading tab is not the key.
        let indented = "\tballot\tbelongs\n\tclaim\tT-12\n\tartifact\ta.rs\n\tkind\tnoul\n\tinstruction\tdone?\n\tpresenter\tala\n\tmax-age\t1200\n";
        let ballots = parse_body(indented).expect("an indented block parses");
        assert_eq!(ballots.len(), 1);
        assert_eq!(ballots[0].id, "belongs");
        assert_eq!(ballots[0].claim, "T-12");
    }

    #[test]
    fn a_declared_ballot_round_trips_through_its_own_text() {
        let ballots = parse(A_BALLOT).expect("the declared ballot parses");
        assert_eq!(ballots.len(), 1);
        let ballot = &ballots[0];
        assert_eq!(ballot.id, "belongs");
        assert_eq!(ballot.kind, Kind::Noul);
        assert_eq!(ballot.max_age, 1200);
        assert_eq!(ballot.threshold_thousandths, DEFAULT_THRESHOLD_THOUSANDTHS);
        // Written and read are the same text: render what was parsed and parse it again.
        let again = parse(&format!("{HEADER}\n{}", ballot.render())).expect("its own render parses");
        assert_eq!(&again[0], ballot, "a ballot does not survive its own round trip");
    }

    #[test]
    fn a_question_with_no_terms_is_refused_rather_than_asked() {
        // A choice with no options is the failure that matters: the model is asked to pick and
        // returns a confident-looking key that names nothing.
        let why = parse(
            "shploingus.ballot/1\nballot\tpick\nclaim\tT-1\nartifact\tcrates/x/src/lib.rs\nkind\tchoice\n\
             instruction\twhich of these?\npresenter\tala\nmax-age\t60\n",
        )
        .expect_err("a choice with no options is refused");
        let text = format!("{why}");
        assert!(text.contains("not a choice"), "{text}");

        let why = parse(
            "shploingus.ballot/1\nballot\trung\nclaim\tT-1\nartifact\tcrates/x/src/lib.rs\nkind\tscore\n\
             instruction\thow far?\nlevel\twholly\npresenter\tala\nmax-age\t60\n",
        )
        .expect_err("a one-level scale is refused");
        assert!(format!("{why}").contains("not a scale"), "{why}");

        let why = parse(
            "shploingus.ballot/1\nballot\tvibes\nclaim\tT-1\nartifact\tcrates/x/src/lib.rs\nkind\tfeeling\n\
             instruction\twhat vibe?\npresenter\tala\nmax-age\t60\n",
        )
        .expect_err("an unknown instrument is refused");
        assert!(format!("{why}").contains("is not a question kind"), "{why}");
    }

    #[test]
    fn an_answer_nobody_declared_the_age_of_is_refused() {
        // `max-age` absent is 0, and 0 is not a window: upstream refuses a proof that cannot be
        // aged rather than grandfathering it, and so does this.
        let why = parse(
            "shploingus.ballot/1\nballot\tbelongs\nclaim\tT-1\nartifact\tcrates/x/src/lib.rs\nkind\tnoul\n\
             instruction\tdone?\npresenter\tala\n",
        )
        .expect_err("no max-age is refused");
        assert!(format!("{why}").contains("not a window"), "{why}");
    }

    #[test]
    fn every_problem_is_reported_not_only_the_first() {
        // Two empty fields on one ballot: a caller fixes one round, not two.
        let why = parse(
            "shploingus.ballot/1\nballot\tbelongs\nartifact\tcrates/x/src/lib.rs\nkind\tnoul\n\
             instruction\tdone?\nmax-age\t60\n",
        )
        .expect_err("empty claim and presenter are refused");
        let text = format!("{why}");
        assert!(text.contains("claim"), "{text}");
        assert!(text.contains("presenter"), "{text}");
    }

    #[test]
    fn a_file_that_declares_nothing_is_a_no_reading_and_not_a_pass() {
        let why = parse("shploingus.ballot/1\n").expect_err("an empty file is refused");
        assert!(format!("{why}").contains("nothing to judge"), "{why}");
        let why = parse("not a ballot file\n").expect_err("a foreign file is refused");
        assert!(format!("{why}").contains("opens with"), "{why}");
    }

    #[test]
    fn two_ballots_sharing_an_id_would_make_an_answer_ambiguous_and_are_refused() {
        let two = format!("{HEADER}\n{}{}", &A_BALLOT[HEADER.len() + 1..], &A_BALLOT[HEADER.len() + 1..]);
        let why = parse(&two).expect_err("a repeated id is refused");
        assert!(format!("{why}").contains("share this id"), "{why}");
    }

    #[test]
    fn a_choice_and_a_score_carry_their_terms_in_the_roles_they_declared() {
        let ballots = parse(&format!(
            "{HEADER}\nballot\tpick\nclaim\tT-2\nartifact\ta.rs\nkind\tchoice\noption\tstream it\n\
             option\tleave it\npasses\tstream it\ninstruction\twhich?\npresenter\tala\nmax-age\t60\n\
             ballot\trung\nclaim\tT-2\nartifact\ta.rs\nkind\tscore\nlevel\tnone\nlevel\tsome\n\
             level\tall\nfloor\t2\ninstruction\thow far?\npresenter\tala\nmax-age\t60\n"
        ))
        .expect("two ballots parse");
        assert_eq!(ballots[0].terms(), ["stream it", "leave it"]);
        assert!(ballots[0].levels.is_empty());
        assert_eq!(ballots[0].passes.as_deref(), Some("stream it"));
        assert_eq!(ballots[1].terms(), ["none", "some", "all"]);
        assert!(ballots[1].options.is_empty());
        assert_eq!(ballots[1].floor, Some(2));
    }

    #[test]
    fn a_question_that_does_not_say_what_passes_cannot_produce_a_verdict() {
        // The floor is a ruling about the work, and a judge that invents it after seeing the answer
        // has invented the ruling. So the ballot must carry it, and its absence is refused here.
        let why = parse(
            "shploingus.ballot/1\nballot\tpick\nclaim\tT-2\nartifact\ta.rs\nkind\tchoice\n\
             option\tstream it\noption\tleave it\ninstruction\twhich?\npresenter\tala\nmax-age\t60\n",
        )
        .expect_err("a choice with no passes is refused");
        assert!(format!("{why}").contains("no `passes`"), "{why}");

        let why = parse(
            "shploingus.ballot/1\nballot\trung\nclaim\tT-2\nartifact\ta.rs\nkind\tscore\n\
             level\tnone\nlevel\tall\ninstruction\thow far?\npresenter\tala\nmax-age\t60\n",
        )
        .expect_err("a score with no floor is refused");
        assert!(format!("{why}").contains("no `floor`"), "{why}");

        // A floor above the top rung is a question nothing can pass, and a `passes` naming an
        // option that was never declared is the same defect from the other side.
        let why = parse(
            "shploingus.ballot/1\nballot\trung\nclaim\tT-2\nartifact\ta.rs\nkind\tscore\n\
             level\tnone\nlevel\tall\nfloor\t7\ninstruction\thow far?\npresenter\tala\nmax-age\t60\n",
        )
        .expect_err("a floor past the top rung is refused");
        assert!(format!("{why}").contains("nothing can pass"), "{why}");

        let why = parse(
            "shploingus.ballot/1\nballot\tpick\nclaim\tT-2\nartifact\ta.rs\nkind\tchoice\n\
             option\tyes\noption\tno\npasses\tmaybe\ninstruction\twhich?\npresenter\tala\nmax-age\t60\n",
        )
        .expect_err("a passes naming an undeclared option is refused");
        assert!(format!("{why}").contains("could ever pass"), "{why}");
    }
}
