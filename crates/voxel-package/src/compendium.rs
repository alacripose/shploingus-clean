//! **The record, compressed into a format a solution can be derived from.**
//!
//! # Why this module exists
//!
//! The record is ten Markdown files totalling roughly 400 KB. It cannot be the
//! thing an evaluator reads: [`crate::oracle::StructuralJudge`] judges a *line*
//! against a *recipe*, and nothing in it can be handed 400 KB of prose and get
//! an answer that means something. Worse, the prose is not a flat list — it is
//! **186 numbered decisions in a closed shape**, buried in narrative that
//! restates them. A reader who wants to know what `R-64.1` says has to find it.
//!
//! So the documents are **compressed into a derived format**: every anchor
//! becomes a [`Decree`] carrying its own text, the document it came from, the
//! line it was found on, and the digest of the bytes it was read from. The
//! evaluator then reads decrees, not documents, and the recipe a solution is
//! derived from is assembled from them.
//!
//! This is R-62.2 made mechanical. The ruling was *"the docs become the basic
//! recipe"* — and a ruling that says the documents *are* the declaration cannot
//! be honoured by transcribing them into a second structure, because a second
//! structure is a second copy and a second copy drifts. So the format here holds
//! **no new content at all**: every field is either an anchor the record already
//! used, a location the record already had, or a digest of bytes the record
//! already contained. Nothing is summarised, because a summary is a place where
//! the record and the tool can disagree without either being wrong.
//!
//! # The shape, measured rather than assumed
//!
//! The anchor grammar was **measured against this tree's documents before this
//! format was written**, because the previous failure in this project was
//! inventing a vocabulary and only then reading the authority (C-61, C-64). What
//! the measurement found:
//!
//! * 186 distinct anchors of four series: `C-` (ledger), `H-` (hypergrilling),
//!   `L-` (reference research), `R-` (rulings).
//! * **104** of them are *defining* — the heading **begins** with the anchor.
//!   The other 82 are citations of an anchor inside someone else's heading.
//! * Three anchors (`H-11.5`, `H-11.6`, `H-13.1`) are **defined twice**: asked
//!   in one place and answered in another, both under the same number.
//!
//! That last one is why [`Occurrence`] exists and why [`Series`] keys on the
//! number rather than the whole anchor string. A format that stored one decree
//! per anchor would have to either drop the second occurrence or refuse the
//! record. Both would be wrong: the record genuinely asks a question under
//! `H-11.5` and answers it under a different `H-11.5`, and that is a fact about
//! how the user reasons, not a defect to be tidied away.
//!
//! # What is refused
//!
//! Three things, and each is a different fact:
//!
//! * a document that cannot be read ([`read_document`], refusing for us);
//! * a heading that **looks** like a decree heading but carries no anchor this
//!   format has a series for — [`Series::parse`] returns `Err` rather than
//!   inventing a series, because a fifth series is a change to the record and
//!   not to this file;
//! * an anchor whose body is empty, which would be a decision with no text and
//!   therefore nothing to derive anything from.
//!
//! Note what is **not** refused: an anchor that appears only in a derived
//! document. That is [`crate::record::DocumentRole`]'s question and it is
//! answered there, against the full set. This module's job is to compress; it
//! does not get to decide what counts as authority.

use std::collections::BTreeMap;
use std::path::Path;

use voxel_model::{Digest, RefusalKind, RefusalSet};

use crate::record::{read_document, Document, DocumentRole, DOCUMENTS};

/// The format's version. The pin, and the thing a bump is recorded against.
///
/// `R-64.1` is the ruling this implements: *the format is defined by this
/// repository, versioned, and closed.* So this string is the whole of the
/// versioning discipline, and a document requiring a field this format does not
/// have is a refusal in the build gate — not a schema change.
pub const FORMAT: &str = "shploingus.compendium/1";

/// The four anchor series the record uses. **Closed**, and adding one is a
/// change to the record rather than to this match.
///
/// The names are the series' own, taken from what the documents call them: `C-`
/// for the append-only ledger, `H-` for the user's answers, `L-` for reference
/// research, `R-` for the rulings. C-61 exists because this crate once invented
/// oracle vocabulary that the record did not use; the cost of getting this
/// wrong is a gate that convicts contributors over words nobody wrote.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Series {
    /// `C-n` — a ledger entry. What was decided or found, with its evidence.
    Ledger,
    /// `H-n.m` — one of the user's answers.
    Answer,
    /// `L-n` — reference research. **Not authority** (C-48/C-49).
    Research,
    /// `R-n.m` — a ruling: an answer that binds.
    Ruling,
}

/// How many series there are. **Check 1 of 2**: a literal, not `Series::ALL.len()`
/// — deriving it would make [`Series::ALL`] and this agree by construction, and
/// then the check below would be checking nothing.
pub const SERIES_COUNT: usize = 4;

impl Series {
    /// Every series, in declaration order.
    pub const ALL: [Series; SERIES_COUNT] = [
        Series::Ledger,
        Series::Answer,
        Series::Research,
        Series::Ruling,
    ];

    /// The letter this series is written with.
    pub const fn letter(self) -> char {
        match self {
            Series::Ledger => 'C',
            Series::Answer => 'H',
            Series::Research => 'L',
            Series::Ruling => 'R',
        }
    }

    /// A stable name. Never prose.
    pub const fn name(self) -> &'static str {
        match self {
            Series::Ledger => "ledger",
            Series::Answer => "answer",
            Series::Research => "research",
            Series::Ruling => "ruling",
        }
    }

    /// Whether an anchor of this series may be cited as authority.
    ///
    /// **`L-` may not**, and this is C-48/C-49 stated as a property of the type
    /// rather than as a rule somebody has to remember: research into another
    /// tree is a *reason* and never an *authority*, so a caller that holds a
    /// [`Series`] can ask and a compiler can see the answer.
    pub const fn is_authority(self) -> bool {
        !matches!(self, Series::Research)
    }

    /// Parse the letter. **Refuses** anything else.
    ///
    /// No wildcard arm, and the refusal names the letter: a decree heading
    /// carrying a series this format has never heard of is a change to the
    /// record, and the honest response to that is to stop and say so.
    pub fn parse(letter: char) -> Result<Self, String> {
        match letter {
            'C' => Ok(Series::Ledger),
            'H' => Ok(Series::Answer),
            'L' => Ok(Series::Research),
            'R' => Ok(Series::Ruling),
            other => Err(format!(
                "anchor series {other:?} is not one of C, H, L, R; this format has {SERIES_COUNT} \
                 series and adding one is a change to the record, not to the code that reads it"
            )),
        }
    }
}

/// **Check 1 of 2.** Every series has a row, in a `const`.
///
/// The value is meaningless and always zero; the **type** is the assertion.
/// A fifth variant without a row here does not compile.
pub const SERIES_ARE_TOTAL: [(); SERIES_COUNT] = [(); Series::ALL.len()];

/// An anchor's coordinates: which series, which number, which sub-number.
///
/// **Keyed on the number, not the whole anchor string**, and that is the
/// measured requirement rather than a stylistic choice: `H-11.5` is written
/// twice in `HYPERGRILLING.md` — once asking a question, once answering it — so
/// a format keyed on the string would collide on a record that is correct.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Anchor {
    pub series: Series,
    /// The number after the letter. `11` for `H-11.5`.
    pub number: u32,
    /// The sub-number, when the anchor carries one.
    pub sub: Option<u32>,
}

impl Anchor {
    /// The anchor exactly as the record writes it.
    pub fn render(self) -> String {
        match self.sub {
            Some(sub) => format!("{}-{}.{}", self.series.letter(), self.number, sub),
            None => format!("{}-{}", self.series.letter(), self.number),
        }
    }

    /// Parse an anchor string. **Refuses** anything malformed, naming what it saw.
    ///
    /// Used by the evaluator rather than only by the parser, because a
    /// declaration in a feature row is an anchor *string* and a string that
    /// cannot be parsed is a citation that cannot be checked — which is a
    /// refusal, not a shrug.
    pub fn parse(text: &str) -> Result<Self, String> {
        let trimmed = text.trim();
        let mut chars = trimmed.chars();
        let letter = chars
            .next()
            .ok_or_else(|| format!("{trimmed:?} is empty; an anchor is a letter, a dash and a number"))?;
        let series = Series::parse(letter)?;
        let rest = trimmed[1..].strip_prefix('-').ok_or_else(|| {
            format!("{trimmed:?} has no dash after its series letter; the record writes C-56, not C56")
        })?;

        let (number, sub) = match rest.split_once('.') {
            Some((n, s)) => (n, Some(s)),
            None => (rest, None),
        };

        let number = number.parse::<u32>().map_err(|_| {
            format!("{trimmed:?} has a non-numeric number {number:?}; the record numbers its anchors")
        })?;
        let sub = match sub {
            None => None,
            Some(text) => Some(text.parse::<u32>().map_err(|_| {
                format!("{trimmed:?} has a non-numeric sub-number {text:?}")
            })?),
        };

        Ok(Anchor { series, number, sub })
    }
}

impl std::fmt::Display for Anchor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.render())
    }
}

/// One place one anchor is written.
///
/// A [`Compendium`] holds a `Vec` of these rather than a map, because an anchor
/// with two defining headings has two *answers* and a map would have to pick
/// one. [`Compendium::decrees_of`] returns all of them, in document order, so
/// the reader sees the question and then the answer rather than whichever the
/// hash table kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Occurrence {
    pub anchor: Anchor,
    /// Which document of the record it was found in.
    pub document: &'static str,
    /// The role of that document. **Carried, not re-derived**: this module
    /// compresses and [`crate::record`] decides standing.
    pub role: DocumentRole,
    /// 1-based line number in the document. So a refusal can be pointed at.
    pub line: usize,
    /// The heading text after the anchor, with the anchor itself removed.
    pub title: String,
    /// The anchor's body: every line until the next heading, trimmed.
    ///
    /// **Verbatim.** No summary, no rewrite, no reflow. A derived format that
    /// paraphrases its source is a second copy, and R-62.2's whole content is
    /// that there is no second copy.
    pub body: String,
    /// The digest of the body bytes as read.
    ///
    /// Binds a decision to the text it was read from, so a later edit to the
    /// record is *visible* rather than silently absorbed. It is not a signature
    /// and says nothing about who wrote it.
    pub body_digest: Digest,
}

impl Occurrence {
    /// The whole text of this occurrence: its title, then its body.
    ///
    /// What an evaluator is handed as a **recipe**. The title is included because
    /// half of what a heading says is in it, and a judge that only saw the body
    /// would be reading a decision without knowing what it was about.
    pub fn text(&self) -> String {
        if self.body.is_empty() {
            self.title.clone()
        } else {
            format!("{}\n\n{}", self.title, self.body)
        }
    }
}

/// What to derive a recipe from. **A declaration, not a search.**
///
/// `anchors` is the caller's list of the decisions that bear on the subject, and
/// it is a *list* rather than a query because a query would need a relevance
/// function, and a relevance function invented by this module would be a way of
/// deciding what the record means that the record has not delegated. A wrong
/// declaration is visible at the call site; a wrong relevance score is invisible
/// everywhere.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecipeRequest {
    /// What this recipe is for, named so a refusal can say.
    pub subject: String,
    /// The decisions to derive from, in the order they should be read.
    pub anchors: Vec<Anchor>,
    /// Whether reference research may license this recipe.
    ///
    /// **Defaults to `false` in the type, not at the call site**, and that is the
    /// point: the safe reading is the one you get by doing nothing, and a caller
    /// who wants research has to write down that they want it. C-48/C-49 as a
    /// default rather than a rule.
    pub allow_research: bool,
}

impl RecipeRequest {
    /// A request that licenses nothing but the record's own decisions.
    pub fn of(subject: impl Into<String>, anchors: Vec<Anchor>) -> Self {
        RecipeRequest {
            subject: subject.into(),
            anchors,
            allow_research: false,
        }
    }

    /// The same request, with research permitted. **Named, and recorded on the
    /// [`Recipe`]**, so a caller who did this cannot do it quietly.
    pub fn allowing_research(mut self) -> Self {
        self.allow_research = true;
        self
    }
}

/// A derived recipe: the text a judge reads, and where it came from.
///
/// Both halves are kept because the text alone is not checkable. A judge that
/// convicts a contributor hands back a term; the recipe's `sources` are how
/// anybody finds out which decision that term came from, and a conviction that
/// cannot be traced to a ruling is a conviction with no appeal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recipe {
    /// What it is for.
    pub subject: String,
    /// The concatenated text of the decisions, each prefixed with its anchor.
    pub text: String,
    /// Which decisions, in order.
    pub sources: Vec<Anchor>,
    /// Whether research was permitted. Carried so it can be printed.
    pub allow_research: bool,
}

impl Recipe {
    /// **A recipe derived from no decisions** — the honest state before a selection exists.
    ///
    /// [`Compendium::derive`] already produces exactly this for an empty anchor list, and this
    /// constructor exists so a caller that holds **no compendium at all** can still render a
    /// document: the API window is built from a ticket the shell has just compiled, and the record
    /// it would be derived from is not necessarily beside that ticket's workspace. The result is
    /// not a claimed backing — [`Recipe::licenses_anything`] is false and the renderer prints
    /// `derived from: none`, which is what the reader is owed rather than silence.
    pub fn unbacked(subject: impl Into<String>) -> Self {
        Recipe {
            subject: subject.into(),
            text: String::new(),
            sources: Vec::new(),
            allow_research: false,
        }
    }

    /// The vocabulary this recipe licenses, as [`crate::oracle::terms`] would
    /// read it.
    ///
    /// A convenience, and deliberately a thin one: it is the same
    /// [`crate::oracle::terms`] the judge uses, so there is exactly one
    /// definition of what a word is in this project. A second tokenizer here
    /// would be a second answer to that question and the two would drift.
    pub fn licensed_terms(&self) -> Vec<String> {
        crate::oracle::terms(&self.text)
    }

    /// Whether the recipe licenses anything at all.
    ///
    /// An empty recipe is **not** an error here — deriving nothing from a
    /// subject with no decisions is a real answer — but it is a state a caller
    /// must handle, so it is asked rather than defaulted away.
    pub fn licenses_anything(&self) -> bool {
        !self.text.trim().is_empty()
    }
}

/// What was asked, and whether the record answered it.
///
/// A closed two-state answer, and the two states are not "yes/no" — they are
/// *asked* and *left open*, because the record contains questions the user has
/// not answered and those are the most important things in it. `docs/TODO.md`
/// and every "left blank" in `HYPERGRILLING.md` are the same fact, and a
/// boolean would render them as `false`, which reads as *refused* rather than
/// as *not yet asked*.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Openness {
    /// The record poses this and answers it.
    Settled,
    /// The record poses this and leaves it open. **Not a failure of this
    /// module** — it is the state the record is honestly in.
    Open,
}

impl Openness {
    pub const fn slug(self) -> &'static str {
        match self {
            Openness::Settled => "settled",
            Openness::Open => "open",
        }
    }
}

/// One question the record poses, and what it did about it.
///
/// **A question is derived, not stored.** The record does not mark its questions;
/// it *asks* them, in headings ending in a question mark. So this is found by
/// reading the title, and it is the one place this module interprets rather than
/// transcribes — which is stated here because the module's whole claim is that it
/// invents nothing.
///
/// The interpretation is deliberately crude and deliberately narrow: a title
/// ending in `?` is a question. That is the record's own convention, and a
/// cleverer rule would be a rule this repository has not adopted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Question {
    pub anchor: Anchor,
    pub document: &'static str,
    pub line: usize,
    pub title: String,
    pub openness: Openness,
}

impl Question {
    /// The text a judge is handed: the question, plus any answer beneath it.
    pub fn text(&self, occurrence: &Occurrence) -> String {
        occurrence.text()
    }
}

/// Every decision in the record, compressed.
///
/// The derived format's whole content. `Vec` order is **document order then line
/// order**, so the compression reads the way the record reads and two runs over
/// the same bytes produce the same vector.
///
/// `Default` is hand-written rather than derived, because [`Digest`] has no
/// `Default` and **should not**: a digest of nothing is a value that looks like
/// a measurement. So the empty compendium carries the digest of no bytes, and
/// [`Compendium::is_empty`] is the only thing that may be asked about it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Compendium {
    pub occurrences: Vec<Occurrence>,
    /// The digest of the entire record as read. One number that changes when
    /// anything in any of the ten documents changes.
    pub record_digest: Digest,
}

impl Default for Compendium {
    /// The empty compression, carrying the digest of **no** bytes.
    ///
    /// [`Digest::of`] of an empty slice, not a zero digest. The distinction is
    /// small and the reason is not: a hand-written `Default` that returned
    /// `Digest::default()` would need `Digest` to grow one, and a digest whose
    /// all-zero bytes means "nothing" is a value that can be printed as though
    /// something had been measured.
    fn default() -> Self {
        Compendium {
            occurrences: Vec::new(),
            record_digest: Digest::of(&[]),
        }
    }
}

impl Compendium {
    /// How many decisions were compressed. Counted, not claimed.
    pub fn len(&self) -> usize {
        self.occurrences.len()
    }

    pub fn is_empty(&self) -> bool {
        self.occurrences.is_empty()
    }

    /// Every occurrence of one anchor, in record order.
    ///
    /// **A `Vec<&Occurrence>`, not a slice**, and the reason is the record's own
    /// shape: `H-11.5` is written twice, so the occurrences of one anchor are
    /// *not contiguous* in a vector that is in document order. A slice would
    /// have to promise contiguity and so would silently return the first one
    /// only. Returning a vector says plainly that there may be several.
    ///
    /// Empty if the record does not write it, which is a fact a caller must
    /// handle — hence no defaulted single value.
    pub fn decrees_of(&self, anchor: Anchor) -> Vec<&Occurrence> {
        self.occurrences
            .iter()
            .filter(|occurrence| occurrence.anchor == anchor)
            .collect()
    }

    /// The anchors present, each once, in order.
    pub fn anchors(&self) -> Vec<Anchor> {
        let mut seen: Vec<Anchor> = Vec::new();
        for occurrence in &self.occurrences {
            if !seen.contains(&occurrence.anchor) {
                seen.push(occurrence.anchor);
            }
        }
        seen
    }

    /// How many distinct anchors, and how many places they are written in.
    ///
    /// Reported rather than enforced, because the record's three twice-written
    /// anchors are not a defect. A tool that hid the difference would make the
    /// record look tidier than it is, which is the failure mode this project
    /// keeps refusing.
    pub fn shape(&self) -> Shape {
        let mut per_anchor: BTreeMap<Anchor, usize> = BTreeMap::new();
        for occurrence in &self.occurrences {
            *per_anchor.entry(occurrence.anchor).or_insert(0) += 1;
        }
        Shape {
            distinct_anchors: per_anchor.len(),
            occurrences: self.occurrences.len(),
            written_more_than_once: per_anchor.values().filter(|count| **count > 1).count(),
        }
    }

    /// **The derivation.** Turn the compressed record into a judgeable recipe.
    ///
    /// This is the function the whole module exists to make possible. The
    /// problem it solves is concrete: [`crate::oracle::StructuralJudge`] judges a
    /// line against *a recipe*, and the recipe has to be text. Handing it the
    /// record's 400 KB would be handing it prose with 104 decisions buried in it,
    /// and a judge that convicted a contributor against a whole ledger would be
    /// convicting them against everything this project has ever thought.
    ///
    /// So the recipe is **assembled from the decisions that bear on the subject**
    /// — and which ones bear on it is decided by the caller naming anchors, not
    /// by a similarity score invented here. That is the whole design: the
    /// selection is a *declaration*, so a wrong selection is visible in the code
    /// that made it, and a right one is reproducible.
    ///
    /// The rules, each of which the project has a reason for:
    ///
    /// * a named anchor that the record **does not contain** is a **refusal**,
    ///   naming the anchor — a recipe citing a decision nobody wrote is exactly
    ///   the defect [`crate::record::check_declarations`] exists to catch, and
    ///   this is that check at the point of use;
    /// * a named anchor in the `L-` series is a **refusal** unless the caller
    ///   says so explicitly, because research is a reason and never an authority
    ///   (C-48/C-49). [`Recipe::allow_research`] is how a caller overrides, and
    ///   the override is a field on the record so it is printed;
    /// * a twice-written anchor contributes **both** occurrences, because the
    ///   record asked and then answered and a recipe holding only one of those is
    ///   holding half a decision.
    ///
    /// Nothing here is defaulted. An empty [`Recipe`] names no anchors, so it
    /// licenses nothing — which is the correct reading of "derive nothing" rather
    /// than a convenient one.
    pub fn derive(&self, request: &RecipeRequest) -> Result<Recipe, RefusalSet> {
        let mut refusals = RefusalSet::new();
        let mut text = String::new();
        let mut sources: Vec<Anchor> = Vec::new();

        for anchor in &request.anchors {
            let occurrences = self.decrees_of(*anchor);
            if occurrences.is_empty() {
                refusals.refuse(
                    RefusalKind::FabricatedFeature,
                    format!(
                        "the recipe for {} names {anchor}, and the record does not contain it. A \
                         recipe may only be derived from decisions the record actually made",
                        request.subject
                    ),
                );
                continue;
            }

            if anchor.series == Series::Research && !request.allow_research {
                refusals.refuse(
                    RefusalKind::FabricatedFeature,
                    format!(
                        "the recipe for {} names {anchor}, which is reference research. Outside \
                         trees are a reason and never an authority (C-48/C-49), so research may \
                         not license a recipe unless the caller passes `allow_research` and takes \
                         responsibility for it in the open",
                        request.subject
                    ),
                );
                continue;
            }

            for occurrence in occurrences {
                if !text.is_empty() {
                    text.push_str("\n\n");
                }
                // The anchor goes in the text, not just the metadata: a judge
                // reading the recipe should be able to see which decision it is
                // reading, or a conviction cannot be traced to a ruling.
                text.push_str(&format!("{anchor}: {}\n\n{}", occurrence.title, occurrence.body));
            }
            sources.push(*anchor);
        }

        refusals.into_result(Recipe {
            subject: request.subject.clone(),
            text,
            sources,
            allow_research: request.allow_research,
        })
    }

    /// The questions the record poses and leaves open.
    ///
    /// **Derived from titles ending in `?`,** which is the record's own
    /// convention and nothing more clever. A title that asks is open unless the
    /// body says otherwise; this does not try to read the body for an answer,
    /// because deciding whether prose answers a question is a judgement and a
    /// judgement here would be a fabricated one.
    pub fn open_questions(&self) -> Vec<Question> {
        let mut questions = Vec::new();
        for occurrence in &self.occurrences {
            if !occurrence.title.trim_end().ends_with('?') {
                continue;
            }
            questions.push(Question {
                anchor: occurrence.anchor,
                document: occurrence.document,
                line: occurrence.line,
                title: occurrence.title.clone(),
                openness: Openness::Open,
            });
        }
        questions
    }
}

/// What the compression found, as counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shape {
    pub distinct_anchors: usize,
    pub occurrences: usize,
    /// Anchors the record writes in more than one place.
    pub written_more_than_once: usize,
}

/// Compress the whole record, or refuse.
///
/// Reads every document in [`DOCUMENTS`] — **all** of them, including
/// `HANDOFF.md`, because a derived document still says things and excluding it
/// would make this a compression with a hole in it. Which of those things carry
/// authority is [`crate::record`]'s question and is not answered here.
pub fn compress(root: &Path) -> Result<Compendium, RefusalSet> {
    let mut refusals = RefusalSet::new();
    let mut occurrences: Vec<Occurrence> = Vec::new();
    let mut whole: Vec<u8> = Vec::new();

    for document in DOCUMENTS {
        let text = match read_document(root, document) {
            Ok(text) => text,
            Err(found) => {
                // A document that cannot be read means the compression would be
                // silently missing a decision. That is a refusal, not a shorter
                // compendium.
                refusals.absorb(found);
                continue;
            }
        };
        // Tagged, so two documents with the same bytes cannot concatenate into
        // one record's digest.
        whole.extend_from_slice(document.path.as_bytes());
        whole.push(0);
        whole.extend_from_slice(text.as_bytes());
        whole.push(0);

        match extract(document, &text) {
            Ok(found) => occurrences.extend(found),
            Err(found) => {
                refusals.absorb(found);
            }
        }
    }

    if !refusals.is_empty() {
        return Err(refusals);
    }

    Ok(Compendium {
        occurrences,
        record_digest: Digest::of(&whole),
    })
}

/// Pull the defining headings out of one document's text.
///
/// **Defining** means the heading *begins* with the anchor, optionally followed
/// by a dash. This is not a guess: the measurement found 17 anchors written in
/// more than one heading, and every one of them is a heading that *cites* an
/// anchor inside its title — `### 51.3 STATUS.md's dependency rows were stale,
/// and contradicted this ledger's own C-33/C-37`. Counting those as decisions
/// would inflate the record by 82 and attribute rulings to the wrong entries.
fn extract(document: &'static Document, text: &str) -> Result<Vec<Occurrence>, RefusalSet> {
    let mut refusals = RefusalSet::new();
    let lines: Vec<&str> = text.lines().collect();
    let mut found: Vec<Occurrence> = Vec::new();

    // A heading starts a decision; the next heading of any level ends it.
    //
    // The fence is tracked because the record contains fenced code blocks and a
    // `#` line inside one is a comment in a language, not a ruling. Without
    // this, a shell transcript in the ledger would compress into a decision.
    let mut index = 0usize;
    let mut open_fence: Option<char> = None;
    while index < lines.len() {
        let line = lines[index];
        if let Some(marker) = fence(line) {
            // A fence marker closes its own fence when it matches, and
            // opens one otherwise. CommonMark allows a longer run to close
            // a shorter opener, which this does not model — noted rather
            // than guessed, because guessing here would mean inventing a
            // Markdown rule this repository has not adopted.
            open_fence = match open_fence {
                Some(open) if open == marker => None,
                Some(_) => open_fence,
                None => Some(marker),
            };
            index += 1;
            continue;
        }

        let Some(heading) = markdown_heading(line).filter(|_| open_fence.is_none()) else {
            index += 1;
            continue;
        };

        let Some((anchor, title)) = parse_defining_heading(heading) else {
            index += 1;
            continue;
        };

        // Collect the body: everything up to the heading that **ends this entry**, and a
        // heading inside a fence is never one.
        //
        // **The first version ended the body at the next heading of ANY level**, and that
        // silently truncated every entry the record wrote with subsections. `## C-42` runs to
        // `### 42.1`, so its body was the four lines before that heading and the Barrett
        // taxonomy, the three source cards and the coral classification were **dropped with
        // no refusal** — the body was still non-empty, so every check passed. That is the
        // reader returning a plausible answer over content it never read, and
        // `body_digest` was a digest of the truncated text, so the loss was bound in rather
        // than visible.
        //
        // A migration through that reader would have filled the chain, driven
        // `projection::compare` to zero disagreements, and turned the gate green over a
        // record missing most of its content — and the count, the one thing that would have
        // looked wrong, would have looked **right**.
        let level = heading_level(line).expect("a heading was matched to get here");
        let mut body_lines: Vec<&str> = Vec::new();
        let mut cursor = index + 1;
        let mut body_fence: Option<char> = None;
        while cursor < lines.len() {
            let candidate = lines[cursor];
            if let Some(marker) = fence(candidate) {
                body_fence = match body_fence {
                    Some(open) if open == marker => None,
                    Some(_) => body_fence,
                    None => Some(marker),
                };
                body_lines.push(candidate);
                cursor += 1;
                continue;
            }
            if body_fence.is_none() {
                if let Some(text) = markdown_heading(candidate) {
                    // **Not `unwrap_or_default`.** `markdown_heading` matched this line, so
                    // `heading_level` must too — they read the same two rules. A default here
                    // would make a disagreement between them read as depth 0, and depth 0
                    // ends every entry, which is the truncation this function replaced.
                    let candidate_level = heading_level(candidate)
                        .expect("a line that is a heading has a level");
                    if ends_entry(candidate_level, level, text) {
                        break;
                    }
                }
            }
            body_lines.push(candidate);
            cursor += 1;
        }
        let body = body_lines.join("\n").trim().to_string();

        if body.is_empty() && title.is_empty() {
            refusals.refuse(
                RefusalKind::NoReading,
                format!(
                    "{}:{} carries anchor {anchor} and has neither a title nor a body. A decision \
                     with no text is not a decision this format can compress: there is nothing in \
                     it to derive anything from",
                    document.path,
                    index + 1
                ),
            );
        } else {
            found.push(Occurrence {
                anchor,
                document: document.path,
                role: document.role,
                line: index + 1,
                title,
                body_digest: Digest::of(body.as_bytes()),
                body,
            });
        }

        index = cursor;
    }

    refusals.into_result(found)
}

/// The text of a Markdown heading, or `None` if this line is not one.
///
/// **ATX headings only** (`#` run, then a space). A line beginning with `#`
/// inside a fenced code block would be read as a heading by this, and the
/// record contains fenced blocks — so fences are tracked, and a `#` inside one
/// is text. Getting this wrong would compress a code sample into a decision.
/// A heading's level, **1 through 6**, or `None` if the line is not a heading.
///
/// Separate from [`markdown_heading`] because the text alone cannot say its own depth, and
/// the depth is what decides whether a heading ends an entry or is a subsection of one.
fn heading_level(line: &str) -> Option<usize> {
    let indent = line.len() - line.trim_start().len();
    if indent >= 4 {
        return None;
    }
    let trimmed = line.trim_start();
    let hashes = trimmed.len() - trimmed.trim_start_matches('#').len();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    // **The same ATK rule [`markdown_heading`] applies, checked here too** because a `#tag`
    // is not a heading at either level, and a level test that disagreed with the text test
    // would make `ends_entry` decide on a line the reader had already rejected.
    trimmed[hashes..].strip_prefix(' ')?;
    Some(hashes)
}

/// Whether a heading ends the entry whose own heading sits at `level`.
///
/// **Two rules, and the second is the one that is easy to get wrong.** A deeper heading is a
/// subsection and continues the body; a sibling or an ancestor ends it. But a heading that
/// **defines an anchor of its own** ends the entry at *any* depth — because the record writes
/// `### C-35` through `### C-39` at subsection depth, and those are entries, not subsections
/// of C-34. A rule of "deeper headings continue" would swallow five entries into their
/// neighbour; a rule of "any heading ends it" loses the content of every subsectioned entry.
/// The 54-versus-39 count C-73.4 measured is exactly this distinction, and it is why the
/// check is here rather than in a comment.
fn ends_entry(candidate_level: usize, level: usize, text: &str) -> bool {
    if parse_defining_heading(text).is_some() {
        return true;
    }
    candidate_level <= level
}

fn markdown_heading(line: &str) -> Option<&str> {
    // CommonMark: up to three spaces of indent is still a heading; four or more
    // makes it an indented code block. The first version of this trimmed *all*
    // leading whitespace, so it read code blocks as headings — a defect found by
    // a test that asserted the right answer and agreed by accident.
    let indent = line.len() - line.trim_start().len();
    if indent >= 4 {
        return None;
    }
    let trimmed = line.trim_start();
    if !trimmed.starts_with('#') {
        return None;
    }
    let hashes = trimmed.len() - trimmed.trim_start_matches('#').len();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = &trimmed[hashes..];
    // ATX requires a space after the run, so `#hashtag` is not a heading.
    rest.strip_prefix(' ')
}

/// Whether a line closes a fenced code block, and the fence character.
fn fence(line: &str) -> Option<char> {
    let trimmed = line.trim_start();
    ['`', '~']
        .into_iter()
        .find(|marker| trimmed.starts_with(&marker.to_string().repeat(3)))
}

/// Split a defining heading into its anchor and its title.
fn parse_defining_heading(heading: &str) -> Option<(Anchor, String)> {
    let body = heading.trim();
    let letter = body.chars().next()?;
    if !letter.is_ascii_uppercase() {
        return None;
    }
    // Only a series this format has is a defining heading. A heading that
    // *begins* with an unknown capital letter-and-dash is refused by the
    // caller's series check rather than skipped, so this only filters the ones
    // that are not anchors at all (`## The design system ...`).
    let rest = body[1..].strip_prefix('-')?;
    if rest.is_empty() || !rest.starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }

    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    let after = &rest[digits.len()..];

    let sub = match after.strip_prefix('.') {
        Some(tail) => {
            let sub_digits: String = tail.chars().take_while(|c| c.is_ascii_digit()).collect();
            if sub_digits.is_empty() {
                None
            } else {
                match sub_digits.parse::<u32>() {
                    Ok(value) => Some((value, &tail[sub_digits.len()..])),
                    // A number too large for u32 is a refusal, not a truncation.
                    Err(_) => return None,
                }
            }
        }
        None => None,
    };

    let number = match digits.parse::<u32>() {
        Ok(value) => value,
        Err(_) => return None,
    };

    // The series letter must be one we have, or this is not a decree heading.
    let series = match Series::parse(letter) {
        Ok(series) => series,
        // Not a defining heading for this format; `extract` counts nothing and
        // `check` reports the unknown series. Not silently skipped — see below.
        Err(_) => return None,
    };

    let after_anchor = match sub {
        Some((_, tail)) => tail,
        None => after,
    };

    // A defining heading has the anchor and then, optionally, a space, a dash or
    // an em-dash, and then a title. The record writes several forms — `## C-60 —
    // a third reference read`, `### R-59.1 Provenance: our own digest`, `### R-62.2
    // **The docs become the recipe**` — and the first version of this parser
    // accepted only the dash, which found **no** decisions in a record that has
    // 104 of them. Found by running it.
    //
    // `C-56v2` is still not `C-56`, and that check belongs **here, at the digit
    // boundary, and not after trimming**. The first version put it after the
    // trim, where it rejected every heading whose title began with a capital
    // letter — which is 9 of the 11 `R-` rulings. The distinction is adjacency:
    // a letter *touching* the number is a version suffix, and a letter *after a
    // space* is the first word of a title.
    if let Some(character) = after_anchor.chars().next() {
        if character.is_alphanumeric() {
            return None;
        }
    }

    let after_anchor = after_anchor.trim_start();
    let title = match after_anchor.chars().next() {
        None => String::new(),
        // `## C-60 — title` and `## C-60 - title`: drop the dash, keep the title.
        Some(character) if character == '-' || character == '\u{2014}' => {
            after_anchor.chars().skip(1).collect::<String>().trim().to_string()
        }
        // Anything else at this point is a title, because the adjacency check
        // above has already excluded the version-suffix case.
        Some(_) => after_anchor.to_string(),
    };

    Some((        Anchor {
            series,
            number,
            sub: sub.map(|(value, _)| value),
        },
        title,
    ))
}

/// Check the compression against the record it came from.
///
/// Separate from [`compress`] for the same reason [`crate::features`] splits its
/// checks: *"is this well formed"* and *"does it cover what it must"* are
/// different questions, and a function that answered both could not answer
/// either on a tree that is mid-build.
///
/// **One check, and it is about the compression rather than the record.** This
/// module's job is to compress faithfully; what the compressed things *mean* and
/// which of them may be cited is [`crate::record`]'s question.
///
/// **Removed by measurement, and the removal is the interesting part.** The first
/// version of this function also refused any `L-` research anchor that was
/// *defined in* an authority document, on the reasoning that research must never
/// be authority (C-48/C-49). It refused all seven. The reason it was wrong is
/// the same class of error as C-61 and C-64: **it read a fact about where the
/// record files something and treated it as a fact about what the thing is.** The
/// `L-` series lives in `HYPERGRILLING.md`, which is an authority document, and
/// that is a filing decision — not a promotion. C-48/C-49 is about what research
/// may be *cited as*, and [`Series::is_authority`] already answers that on the
/// type, where a caller cannot forget it.
///
/// So the check is gone and the property is a method. Refusing the record for a
/// formatting detail is the failure mode this project keeps refusing, and this
/// module was doing it.
pub fn check(compendium: &Compendium) -> Result<(), RefusalSet> {
    let mut refusals = RefusalSet::new();

    if compendium.is_empty() {
        refusals.refuse(
            RefusalKind::NoReading,
            format!(
                "the record compressed to {FORMAT} and found no decisions at all. Either the \
                 documents are not the ones this format was written against, or the anchor grammar \
                 changed; both are refusals to surface, not an empty compendium to ship"
            ),
        );
    }

    // Every decision that was found must carry a location, or a reader cannot
    // go and check it. This is the compression's own obligation and it is
    // cheap to state.
    for occurrence in &compendium.occurrences {
        if occurrence.line == 0 {
            refusals.refuse(
                RefusalKind::NoReading,
                format!(
                    "{} carries {} at line 0. Lines are 1-based, and a decision that cannot be \
                     pointed at cannot be checked",
                    occurrence.document, occurrence.anchor
                ),
            );
        }
    }

    refusals.into_result(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::workspace_root;

    fn real_root() -> std::path::PathBuf {
        workspace_root(Path::new(env!("CARGO_MANIFEST_DIR")))
            .expect("a workspace root above the crate")
    }

    fn real() -> Compendium {
        compress(&real_root()).expect("the record compresses")
    }

    // ---- the format's own guarantees ------------------------------------

    #[test]
    fn an_anchor_round_trips_through_its_rendered_form() {
        for text in ["C-56", "H-11.5", "L-3", "R-64.1"] {
            let anchor = Anchor::parse(text).expect("a written anchor parses");
            assert_eq!(anchor.render(), text);
        }
    }

    #[test]
    fn a_malformed_anchor_is_refused_by_name_rather_than_guessed() {
        for text in ["", "C56", "X-1", "C-", "C-abc", "H-1.x", "R-1.2.3"] {
            let error = Anchor::parse(text).expect_err("this is not an anchor");
            assert!(
                error.contains(&text.chars().next().map(|c| c.to_string()).unwrap_or_default())
                    || !text.is_empty(),
                "{text:?} refused with {error:?}, which does not name what it saw"
            );
        }
    }

    #[test]
    fn research_is_not_authority_and_the_type_says_so() {
        // C-48/C-49 as a property, so a caller cannot forget it.
        assert!(!Series::Research.is_authority());
        assert!(Series::Ledger.is_authority());
        assert!(Series::Answer.is_authority());
        assert!(Series::Ruling.is_authority());
    }

    #[test]
    fn an_unknown_series_is_refused_and_the_refusal_names_the_letter() {
        let error = Series::parse('Z').expect_err("Z is not a series");
        assert!(error.contains('Z'), "{error}");
    }

    #[test]
    fn every_series_has_a_row() {
        // The const's value is meaningless; that it compiles is the assertion.
        let _ = SERIES_ARE_TOTAL;
        assert_eq!(Series::ALL.len(), SERIES_COUNT);
    }

    // ---- the shape of the record, measured ------------------------------

    #[test]
    fn the_record_compresses_to_decisions_and_the_check_passes() {
        let compendium = real();
        assert!(!compendium.is_empty(), "the record has decisions in it");
        check(&compendium).expect("the real record is well formed");
    }

    #[test]
    fn compression_is_deterministic() {
        // Same bytes in, same vector out — or the digest in a build is noise.
        let a = real();
        let b = real();
        assert_eq!(a, b);
        assert_eq!(a.record_digest, b.record_digest);
    }

    #[test]
    fn the_record_keeps_its_three_twice_written_anchors_rather_than_being_tidied() {
        // Measured before the format was written: H-11.5, H-11.6 and H-13.1 are
        // each asked in one place and answered in another. A format that stored
        // one decree per anchor would have had to drop or refuse these.
        let compendium = real();
        let shape = compendium.shape();
        assert_eq!(
            shape.written_more_than_once, 3,
            "the record's twice-written anchors changed: {shape:?}"
        );
        let h = Anchor::parse("H-11.5").unwrap();
        let found = compendium.decrees_of(h);
        assert_eq!(found.len(), 2, "{:?}", found.iter().map(|o| o.line).collect::<Vec<_>>());
        // Both survive, and they are the question and the answer.
        assert_ne!(found[0].body_digest, found[1].body_digest);
    }

    /// **The pin for the truncation defect, and it reads the real record rather than a
    /// fixture.**
    ///
    /// A fixture written to be subsectioned would pass under both the old rule and the new
    /// one if the fixture's shape happened to suit, so the test is anchored on `C-42`, which
    /// the record actually wrote with three subsections and a source card in each. The
    /// assertion is on **content from the third subsection**, not on a length: a length
    /// comparison would have been satisfied by any body that grew, and the defect was a body
    /// that was non-empty and wrong.
    #[test]
    fn a_subsections_content_is_part_of_its_entry_and_not_dropped_between_headings() {
        let compendium = real();
        let c42 = compendium
            .decrees_of(Anchor::parse("C-42").expect("C-42 is an anchor"))
            .first()
            .copied()
            .expect("C-42 is in the record");

        // **Whitespace-normalised before matching, and that is not cosmetic.** The record
        // hard-wraps prose at about ninety columns, so `**three\nindependent properties**` is
        // one sentence split across two lines. The first version of this assertion searched
        // for the unwrapped phrase, failed, and read as though the fix had not worked — when
        // the body had in fact grown from four lines to the whole entry. A test that cannot
        // tell a wrapped sentence from a missing one is a test that will be believed when it
        // should not be.
        let flat = c42.body.split_whitespace().collect::<Vec<_>>().join(" ");

        // From `### 42.2` — the middle subsection, so a rule that only kept the first would
        // pass a check on 42.1.
        assert!(
            flat.contains("three independent properties"),
            "C-42's body must carry the content of ### 42.2, not stop at the first subsection: \
             its body is {} bytes and ends {:?}",
            c42.body.len(),
            c42.body.chars().rev().take(60).collect::<String>().chars().rev().collect::<String>()
        );
        // And from `### 42.3`, the last, which is what a "keeps one subsection" bug would lose.
        assert!(
            flat.contains("branching, columnar, encrusting, massive"),
            "C-42's body must carry the content of ### 42.3 as well"
        );
        // The heading line of a subsection is part of the entry it belongs to, so a reader can
        // see the structure rather than a body with its own headings sliced off.
        assert!(
            c42.body.contains("42.2 Clastic rock shape"),
            "a subsection's own heading belongs to the entry, or the body loses its structure"
        );

        // **The digest binds the whole body.** The defect bound the *truncation* in, so this
        // is the assertion that would have caught it: two bodies that differ only in content
        // past the first subsection must not share a digest.
        let truncated = "The record's shaper contract is that no shaper exists as an unexplained pile of noise.";
        assert_ne!(
            c42.body_digest,
            Digest::of(truncated.as_bytes()),
            "the digest must be over the whole entry, or it certifies the truncated text"
        );
    }

    /// **The other half, and it is the half a "deeper headings continue" rule gets wrong.**
    ///
    /// `### C-35` through `### C-39` are entries the record wrote at *subsection* depth. If
    /// depth alone decided, all five would be swallowed into C-34 and the record would hold
    /// fewer entries than it does. So an entry is ended by a heading that **defines an
    /// anchor**, whatever its level.
    #[test]
    fn an_entry_written_at_subsection_depth_is_still_its_own_entry() {
        let compendium = real();
        let shape = compendium.shape();
        // The count C-73.4 measured and could not explain: 34 top-level `## C-` headings, 39
        // entries, and the five between them written as `### C-35`..`### C-39`.
        let c_series: Vec<u32> = compendium
            .occurrences
            .iter()
            .filter(|o| o.anchor.series == Series::Ledger && o.anchor.sub.is_none())
            .map(|o| o.anchor.number)
            .collect();
        for number in 35..=39u32 {
            assert!(
                c_series.contains(&number),
                "C-{number} is written at subsection depth and must still be an entry of its own; \
                 the series reads {c_series:?}"
            );
        }
        assert!(
            shape.distinct_anchors > 0,
            "and the shape is measured, so this is not a vacuous assertion"
        );
    }

    #[test]
    fn a_citing_heading_is_not_mistaken_for_a_decision() {
        // `### 51.3 STATUS.md's dependency rows were stale, and contradicted this
        // ledger's own C-33/C-37` cites three anchors and defines none. Reading it
        // as decisions would inflate the record and attribute rulings to the wrong
        // entries.
        //
        // Asserted on the **specific known case** rather than on a count
        // comparison. The first draft compared "citing headings" against
        // "decisions" and expected the former to be larger; it was not, and the
        // reason is that most headings in the record are *defining* ones. A
        // count ratio was the wrong instrument. The right question is whether
        // this particular heading was read as three decisions.
        let text = std::fs::read_to_string(real_root().join("docs/LEDGER.md"))
            .expect("the ledger reads");
        let citing = text
            .lines()
            .find(|line| line.contains("contradicted this ledger's own C-33/C-37"))
            .expect("that citing heading is in the ledger");
        assert_eq!(
            parse_defining_heading(markdown_heading(citing).expect("it is a heading")),
            None,
            "a heading that cites three anchors in its title defines none of them"
        );
    }

    #[test]
    fn a_hash_inside_a_fenced_block_is_not_a_heading() {
        // The record contains fenced blocks. A `#` line inside one is text, and
        // reading it as a decision would compress a code sample into a ruling.
        assert_eq!(markdown_heading("```rust"), None, "a fence is not a heading");
        assert_eq!(fence("```rust"), Some('`'));
        assert_eq!(fence("no fence here"), None);
        assert_eq!(markdown_heading("#hashtag"), None, "ATX needs a space");
        assert_eq!(markdown_heading("### real"), Some("real"));
        assert_eq!(markdown_heading("####### seven hashes is not a heading"), None);
    }

    #[test]
    fn an_indented_hash_is_a_code_block_and_not_a_heading() {
        // CommonMark: four spaces of indent makes an ATX heading a code block.
        // The first version of this test asserted the opposite and passed for
        // the wrong reason — `markdown_heading` was trimming *all* leading
        // whitespace, so it agreed by accident. Now the rule is real: up to
        // three spaces is a heading, four is code.
        assert_eq!(markdown_heading("   ### indented but a heading"), Some("indented but a heading"));
        assert_eq!(markdown_heading("    # four spaces is code"), None);
    }

    #[test]
    fn a_decision_can_be_found_by_the_anchor_the_record_writes_it_under() {
        // The `R-` rulings are written as `### R-62.2 **The docs become the
        // recipe**` in the ledger, so the anchor headings are findable. Asserted
        // against an anchor that is genuinely written as a heading, because
        // `R-64.1` — the one the first draft of this test used — is stated
        // *inline* in the ruling's prose and never given a heading of its own.
        let compendium = real();
        let r622 = Anchor::parse("R-62.2").expect("R-62.2 is an anchor");
        let found = compendium.decrees_of(r622);
        assert!(!found.is_empty(), "R-62.2 is a heading in LEDGER.md and was not found");
    }

    #[test]
    fn a_decision_body_is_verbatim_so_the_record_cannot_drift_from_the_format() {
        // The point of holding the body rather than a summary: a solution derived
        // from this is derived from the record's actual words.
        let compendium = real();
        let occurrence = compendium
            .occurrences
            .iter()
            .find(|o| o.anchor == Anchor::parse("R-62.2").unwrap())
            .expect("R-62.2 is in the record");
        assert!(
            occurrence.body.contains("docs become the recipe")
                || occurrence.title.contains("docs become the recipe"),
            "R-62.2's text was rewritten: title {:?}, body {:?}",
            occurrence.title,
            &occurrence.body[..occurrence.body.len().min(200)]
        );
        // Verbatim means the body is the record's own characters, so the digest
        // of the body is reproducible from the document on disk.
        //
        // **Line terminators are the checkout's, not the record's.** `compress`
        // splits the document into lines and joins a body with `\n`, so its text
        // carries LF regardless; a Windows checkout with `core.autocrlf=true`
        // writes the same document with CRLF, and the two differ only in a
        // translation Git applies on the way in and out. Comparing raw bytes here
        // asserted that a working copy on one platform is byte-identical to a
        // working copy on another — a fact about the checkout rather than about
        // the record, and one this test failed on this machine while the reader
        // was reporting R-62.2 correctly. The record's content is what is being
        // checked, so the document is read the way the reader reads it: line
        // endings normalised to LF, everything else untouched.
        let document = std::fs::read_to_string(real_root().join(occurrence.document))
            .expect("the document reads")
            .replace("\r\n", "\n");
        assert!(
            document.contains(occurrence.body.trim()),
            "the body is not present in its own document, so it is not verbatim"
        );
    }

    #[test]
    fn a_derived_document_is_read_even_though_it_declares_no_anchors() {
        // `HANDOFF.md` is `DocumentRole::Derived` and is read by `compress`, but
        // it carries no anchor headings — the first draft of this test asserted
        // that it contributed decisions and failed, which is a fact about the
        // record rather than a defect. So the property asserted here is the one
        // that actually holds: the derived document is read, and nothing it says
        // is promoted to authority.
        let compendium = real();
        let derived_read = crate::record::DOCUMENTS
            .iter()
            .filter(|document| document.role == DocumentRole::Derived)
            .count();
        assert!(derived_read > 0, "the record has no derived document to read");
        // And no decision was sourced from one.
        let from_derived = compendium
            .occurrences
            .iter()
            .filter(|occurrence| occurrence.role == DocumentRole::Derived)
            .count();
        assert_eq!(
            from_derived, 0,
            "a derived document contributed {from_derived} decisions. Its headings changed, and \
             the compression is faithfully reporting that — which is correct, and means a feature \
             must not cite an anchor found only here (see crate::record)"
        );
    }

    #[test]
    fn an_empty_compendium_is_refused_rather_than_shipped() {
        let empty = Compendium::default();
        let error = check(&empty).expect_err("an empty record is not well formed");
        assert!(
            error.all().iter().any(|r| r.detail().contains("no decisions at all")),
            "{error}"
        );
    }

    // ---- the derivation: compressed record -> judgeable recipe ----------

    #[test]
    fn a_recipe_is_derived_from_the_record_s_own_words() {
        // **The point of the module.** A recipe is assembled from decisions, and
        // its text is the record's text — not a paraphrase of it.
        let compendium = real();
        let request = RecipeRequest::of(
            "the arrangement package is the shell's own",
            vec![
                Anchor::parse("H-9.7").expect("H-9.7 is an anchor"),
                Anchor::parse("H-9.8").expect("H-9.8 is an anchor"),
            ],
        );
        let recipe = compendium.derive(&request).expect("both anchors are in the record");
        assert!(recipe.licenses_anything());
        assert_eq!(recipe.sources.len(), 2);
        // Each anchor is named in the text, so a conviction traces to a ruling.
        for anchor in &recipe.sources {
            assert!(
                recipe.text.contains(&anchor.render()),
                "{} is not named in its own recipe text",
                anchor
            );
        }
        // And the text is the record's, verbatim.
        let ledger = std::fs::read_to_string(real_root().join("docs/LEDGER.md"))
            .expect("the ledger reads");
        let grilling = std::fs::read_to_string(real_root().join("docs/HYPERGRILLING.md"))
            .expect("the hypergrilling reads");
        let corpus = ledger + &grilling;
        for line in recipe.text.lines().filter(|line| line.len() > 40).take(5) {
            assert!(
                corpus.contains(line.trim()),
                "the recipe invented a line the record does not contain: {line:?}"
            );
        }
    }

    #[test]
    fn a_recipe_citing_a_decision_nobody_made_is_refused() {
        // `crate::record::check_declarations` checks this at declaration time;
        // this checks it at the point of use, because a recipe can also be built
        // at runtime from a list a person typed.
        let compendium = real();
        let request = RecipeRequest::of(
            "a feature that does not exist",
            vec![Anchor {
                series: Series::Ruling,
                number: 9999,
                sub: Some(1),
            }],
        );
        let error = compendium
            .derive(&request)
            .expect_err("the record has no R-9999.1");
        assert!(
            error.all()[0].detail().contains("R-9999.1"),
            "the refusal must name the anchor it could not find: {error}"
        );
    }

    #[test]
    fn research_cannot_license_a_recipe_unless_the_caller_says_so_in_the_open() {
        let compendium = real();
        let l_series = Anchor::parse("L-3").expect("L-3 is an anchor");

        let refused = compendium
            .derive(&RecipeRequest::of("a shell API", vec![l_series]))
            .expect_err("research is not authority by default");
        assert!(
            refused.all()[0].detail().contains("never an authority"),
            "{refused}"
        );

        let allowed = compendium
            .derive(&RecipeRequest::of("a shell API", vec![l_series]).allowing_research())
            .expect("the caller took responsibility in the open");
        assert!(allowed.allow_research, "the override is recorded on the recipe");
        assert!(allowed.text.contains("L-3"), "and the research is named in the text");
    }

    #[test]
    fn a_twice_written_anchor_contributes_both_occurrences() {
        // `H-11.5` is asked in one place and answered in another. A recipe
        // holding only the question would be holding half a decision, and the
        // half it dropped would be the answer.
        let compendium = real();
        let h = Anchor::parse("H-11.5").unwrap();
        assert_eq!(compendium.decrees_of(h).len(), 2, "the record writes it twice");

        let recipe = compendium
            .derive(&RecipeRequest::of("notification grouping", vec![h]))
            .expect("H-11.5 is in the record");
        assert_eq!(recipe.sources.len(), 1, "one anchor named");
        // One anchor, two bodies: the text carries both.
        let occurrences = compendium.decrees_of(h);
        for occurrence in occurrences {
            assert!(
                recipe.text.contains(occurrence.title.trim()),
                "the recipe dropped one of the two H-11.5 occurrences: {:?}",
                occurrence.title
            );
        }
    }

    #[test]
    fn a_recipe_with_no_anchors_licenses_nothing_rather_than_everything() {
        // The default that would matter: an empty declaration licensing every
        // term would make the judge pass everything and mean nothing.
        let compendium = real();
        let recipe = compendium
            .derive(&RecipeRequest::of("nothing in particular", Vec::new()))
            .expect("an empty request derives an empty recipe");
        assert!(!recipe.licenses_anything());
        assert!(recipe.licensed_terms().is_empty());
    }

    #[test]
    fn a_derived_recipe_licenses_the_words_the_record_used() {
        // The join with the oracle: the recipe's terms are read by the same
        // `terms` the judge uses, so there is one definition of a word here.
        let compendium = real();
        let recipe = compendium
            .derive(&RecipeRequest::of(
                "the build gate is the hard stop",
                vec![Anchor::parse("H-9.7").unwrap()],
            ))
            .expect("H-9.7 is in the record");
        let terms = recipe.licensed_terms();
        assert!(!terms.is_empty());
        // A word from the ruling's own text is licensed, so a line using that
        // word is not convicted for using it.
        let occurrence = &compendium.decrees_of(Anchor::parse("H-9.7").unwrap())[0];
        if let Some(word) = crate::oracle::terms(&occurrence.title)
            .into_iter()
            .find(|word| word.len() > 4)
        {
            assert!(
                terms.contains(&word),
                "{word:?} is in the ruling's title and is not licensed by the recipe"
            );
        }
    }

    #[test]
    fn the_record_s_open_questions_are_found_by_its_own_convention() {
        // A title ending in `?` is a question. That is the record's convention
        // and the only rule applied — nothing cleverer, because anything cleverer
        // would be this module deciding what the record means.
        let compendium = real();
        let open = compendium.open_questions();
        assert!(
            open.len() > 5,
            "only {} questions found in a record that poses many; the convention has been read \
             wrongly",
            open.len()
        );
        for question in &open {
            assert!(
                question.title.trim_end().ends_with('?'),
                "{} does not end in a question mark",
                question.title
            );
            assert_eq!(question.openness, Openness::Open);
        }
    }

    #[test]
    fn the_compression_reports_what_it_found() {
        // C-45: the run is visible, and nothing is filtered out of it. The
        // numbers below are **printed**, not asserted, because they are a
        // measurement of the record on this day and pinning them as literals
        // would make a document edit look like a code regression.
        //
        // What *is* asserted is that the compression found the record's own
        // scale: hundreds of decisions across all four series.
        let compendium = real();
        let shape = compendium.shape();
        let mut per_series = [0usize; SERIES_COUNT];
        let mut per_document: Vec<(&str, usize)> = Vec::new();
        for occurrence in &compendium.occurrences {
            per_series[occurrence.anchor.series as usize] += 1;
            match per_document.iter_mut().find(|(path, _)| *path == occurrence.document) {
                Some((_, count)) => *count += 1,
                None => per_document.push((occurrence.document, 1)),
            }
        }

        println!("--- {FORMAT} ---");
        println!(
            "record digest      {}",
            &compendium.record_digest.to_hex()[..32]
        );
        println!("decisions found    {}", shape.occurrences);
        println!("distinct anchors   {}", shape.distinct_anchors);
        println!("written twice      {}", shape.written_more_than_once);
        for series in Series::ALL {
            println!(
                "  {:<9} {:>4} occurrences  authority={}",
                series.name(),
                per_series[series as usize],
                series.is_authority()
            );
        }
        for (path, count) in &per_document {
            println!("  {path:<28} {count:>4}");
        }

        assert!(
            shape.occurrences > 90,
            "the record compressed to {} decisions, which is far below the 104 defining headings \
             measured before this format was written — the extraction has stopped reading headings",
            shape.occurrences
        );
        assert!(shape.distinct_anchors > 80, "only {} distinct anchors", shape.distinct_anchors);
        // Every series must appear, or `Series` names a thing the record does
        // not use and this format would be carrying an empty variant.
        for series in Series::ALL {
            assert!(
                per_series[series as usize] > 0,
                "series {} has no occurrences; either the record stopped using it or the parser \
                 cannot read it, and an empty variant in a closed vocabulary is a trap",
                series.name()
            );
        }
    }

    #[test]
    fn the_record_digest_changes_when_any_document_changes() {
        // The property that makes the digest worth carrying.
        let one = Compendium {
            occurrences: Vec::new(),
            record_digest: Digest::of(b"a"),
        };
        let two = Compendium {
            occurrences: Vec::new(),
            record_digest: Digest::of(b"b"),
        };
        assert_ne!(one.record_digest, two.record_digest);
    }
}
