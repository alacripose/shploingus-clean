//! **The human-readable API document** — the text the Mods window's API view displays.
//!
//! # The surface exists and is empty
//!
//! `recipe::Content::ApiViewer` is a **declared** window kind, and `recipe.rs` already records
//! the problem in its own words: *"a single 18-column tab-separated line is unreadable to a
//! human and unreviewable by a machine."* So the kind is in the vocabulary, the slot is in
//! `packages/alpha-shell.shspec`, and **nothing produces the text it is for.** This module is
//! that producer, and it is the user-side documentation the record says the API view carries.
//!
//! # Why the judge does not write this
//!
//! The reference judge's own header rules it out: *"A judge that could return a sentence would
//! be a judge whose sentence could be quoted."* And this tree inherits that — [`Verdict`] is
//! three words and carries no prose, on purpose.
//!
//! So the document is **not** the judge's output. It is rendered from **declarations**: the
//! record's own anchor titles, and the schema's own per-field `doc`s. Nothing is summarised and
//! nothing is invented, because a summary is a place where the document and the tool can
//! disagree without either being wrong (C-62.2's reason, applied to prose).
//!
//! # The calibration, and it is the whole point
//!
//! [`check`] runs **the same instrument** — [`crate::oracle::lint`] over
//! [`crate::oracle::StructuralJudge`] — over **every line of the document this module just
//! rendered**, and refuses if any line reads as not belonging to the recipe.
//!
//! **That is what "calibrate the judge to make human-readable documents" means here, and it is
//! not the judge making them.** It means: a renderer that invents a word the record does not
//! license is **convicted by the same judge that convicts a contributor**. The prose is not
//! trusted because it is prose; it is checked because it is checkable, and a document that
//! cannot survive its own instrument is a document nobody should read.
//!
//! This is the property C-65.3's defects were all versions of — a check that cannot fail is
//! worse than no check — aimed at the one artefact nobody was checking.

use std::path::Path;

use voxel_model::{RefusalKind, RefusalSet};

use crate::compendium::{Anchor, Recipe};
use crate::compiler::Solution;
use crate::oracle::{lint, Calibration, LintRequest, Oracle, StructuralJudge};

/// The document's title. **A constant, not a derivation** — a title is a label, and a label
/// this module invented would be a second thing that could differ from the package it
/// documents.
///
/// **Every word in it is in [`RENDERER_VOCABULARY`]**, and that is not an accident: the first
/// version read *"what this arrangement declares"*, and the judge convicted `what` and `this`.
/// Which is the check working. The honest response was to shorten the title, not to license
/// two function words.
pub const TITLE: &str = "API view — declared arrangement";

/// The human-readable document for one back-compiled solution.
///
/// **Three sections, and each is traceable.** A reader who wants to know why a window exists
/// gets the record's own anchor, not this module's opinion of it — a conviction that cannot be
/// traced to a ruling is a conviction with no appeal (`compendium::Recipe::sources`).
///
/// # Nothing in here is a sentence this module wrote
///
/// **The first version ended with a paragraph explaining itself** — *"Verified by reading the
/// world back, not by assertion: this document records what was declared, never that it was
/// checked."* It was good prose and the judge convicted fourteen of its words, because the
/// record never said it.
///
/// Two ways to answer that, and only one is honest. The tempting one is to add those words to
/// [`RENDERER_VOCABULARY`**, which would have made the check pass and the check **mean
/// nothing** — a vocabulary long enough to license the document licenses the next document
/// too. So the paragraph is **cut**, and what replaced it is [`Expectation::slug`]: the
/// declared variant's own name, which the record already uses.
///
/// **A document that explains itself is a document that has started summarising.** If a reader
/// needs to know that this text records declarations rather than checks, that belongs in the
/// field `RENDERER_VOCABULARY` and in the test that prints the document — not in prose a judge
/// has to be bribed to accept.
pub fn render(solution: &Solution, recipe: &Recipe) -> String {
    let mut out = String::new();
    out.push_str(TITLE);
    out.push('\n');
    out.push('\n');

    // ---- what was asked for -------------------------------------------------
    out.push_str("## Asked for\n\n");
    out.push_str(&format!(
        "- ticket: {}\n- windows: {}\n",
        solution.ticket.as_str(),
        solution.windows.len()
    ));
    for source in &recipe.sources {
        out.push_str(&format!("- derived from: {}\n", source.render()));
    }
    if recipe.sources.is_empty() {
        // **Never silent.** A document with no sources says so, because "derived from nothing"
        // and "we forgot to record the derivation" print identically otherwise.
        //
        // **`none`, and only `none`.** The first wording was *"no decision on the record — this
        // document is unbacked"*, which the judge convicted on five of its words: `no`, `on`,
        // `the`, `this`, `is`. Every one of them is a function word, and licensing function
        // words is the same move as licensing the paragraph — a vocabulary that stops
        // discriminating. `none` is declared (it is what an empty [`LayoutPolicy`] renders as),
        // and under a list of sources that is empty it says exactly one thing to a reader.
        out.push_str("- derived from: none\n");
    }
    if recipe.allow_research {
        // Carried and printed, so research cannot be licensed quietly (`RecipeRequest`).
        out.push_str("- reference research: permitted\n");
    }
    out.push('\n');

    // ---- what it declares ---------------------------------------------------
    out.push_str("## Declares\n\n");
    for window in &solution.windows {
        out.push_str(&format!("### window[{}]\n\n", window.id));
        out.push_str(&format!("- title: {}\n", window.title));
        out.push_str(&format!("- content: {}\n", window.content.name()));
        out.push_str(&format!("- lane: {}\n", window.lane.name()));
        out.push_str(&format!("- extent: {}\n", window.extent_unit.name()));
        out.push_str(&format!(
            "- geometry: {} {} {} {}\n",
            window.geometry[0],
            window.geometry[1],
            window.geometry[2],
            window.geometry[3]
        ));
        out.push_str(&format!(
            "- min-size: {} {}\n",
            window.min_size[0],
            window.min_size[1]
        ));
        out.push_str(&format!("- opacity: {}\n", window.opacity));
        out.push_str(&format!("- elevation: {}\n", window.elevation));
        out.push_str(&format!("- pinnable: {}\n", window.pinnable));
        out.push_str(&format!("- layout: {}\n", window.layout_policy.declared()));
        out.push_str(&format!("- behavior: {}\n", window.behavior_policy.declared()));
        out.push_str(&format!("- record-id: {}\n", window.record_id));
        out.push('\n');
    }

    // ---- what would retire it ----------------------------------------------
    // **The declared variant, not `describe`'s sentence** — the reason is at the top of this
    // function, and `Expectation::slug` exists for exactly this caller.
    out.push_str("## Retiring requires read-back\n\n");
    out.push_str(&format!(
        "- expectation: {}\n- read-back: required\n",
        solution.expectation.slug()
    ));
    out
}

/// The vocabulary this module's own prose needs, declared rather than assumed.
///
/// **This is the interesting list, and it is deliberately short.** Every word here is one
/// `render` puts in the document that the record's anchors do not supply, so it is listed and
/// therefore checkable. A renderer whose connective tissue is invisible is a renderer whose
/// vocabulary nobody audits.
///
/// **And it is a budget, not a whitelist to grow.** The failure it invites is a document that
/// reads smoothly and licenses nothing — and the symmetric failure is worse: a list long enough
/// to hold the renderer's prose is a list that stops discriminating, which is the same as no
/// check. The first run of [`check`] convicted fourteen words of a paragraph this module had
/// written; the answer was to **delete the paragraph**, not to license it. When a word is
/// missing here the question is *should the document say it at all*, and usually the answer is
/// no.
pub const RENDERER_VOCABULARY: [&str; 1] = [
    "api view arrangement declares declared asked for ticket window windows title content lane extent geometry min size opacity elevation pinnable layout behavior record id read back required requires retiring retire derived from decision expectation permitted unbacked",
];

/// Every word a rendered document prints that the record's anchors do not supply, gathered
/// from the places that declare them.
///
/// **Four sources, and none of them is this module's imagination:**
///
/// * [`RENDERER_VOCABULARY`] — the connective prose (headings, "derived from", "read-back");
/// * the **recipe's own closed vocabularies** — the window field names and the enum values
///   (`lane`, `content`, `extent`, …), which `recipe.rs` declares and which the renderer
///   prints verbatim ([`crate::recipe::declared_window_vocabulary`]);
/// * the **wire schema's own field names** — a `record-id` like `event-0000` is a wire value,
///   so `event` is declared by the schema that projects it, not by this module;
/// * the **solution's own words** — the ticket id, each window's id and title, which the
///   ticket is the authority for. A document that quotes its subject is not inventing when it
///   repeats the subject's name.
pub fn declared_vocabulary(solution: &Solution) -> Vec<String> {
    let mut words: Vec<String> = RENDERER_VOCABULARY.iter().map(|s| s.to_string()).collect();
    words.push(crate::recipe::declared_window_vocabulary());
    words.push(declared_wire_vocabulary());
    words.push(solution.ticket.as_str().to_string());
    for window in &solution.windows {
        words.push(window.id.clone());
        words.push(window.title.clone());
    }
    words
}

/// The wire schema's field names, read out of the schema, **and the words inside them**.
///
/// **Derived, like everything else in this module's vocabulary.** A `record-id` of `event-0000`
/// puts the word `event` in a document and the first run convicted it, correctly: the schema's
/// field is `event_kind`, and [`crate::oracle::terms`] keeps `_` inside a term, so the schema as
/// written licenses `event_kind` and not `event`.
///
/// Two ways to close that gap. One is to add `event` to [`RENDERER_VOCABULARY`], which is a
/// hand-typed word asserting that the renderer is entitled to it. The other is to notice that
/// the schema declares the word `event` — inside `event_kind` — and that a rendered value
/// reaches it by a different separator. **So the words are split back out**, by the same rule
/// [`crate::oracle::terms`] applies to any other identifier.
///
/// **This is not a loophole, and the distinction is worth stating.** It licenses the *words a
/// declared field name is built from*. It does not license a word no declaration contains,
/// and a field added to the schema brings its own words with it. The measured test is the one
/// in `recipe.rs`: the vocabulary must not contain a term no declaration produced.
pub fn declared_wire_vocabulary() -> String {
    let schema = voxel_model::wire::chain_event_schema();
    let mut words: Vec<&str> = Vec::new();
    for field in &schema.fields {
        words.push(field.name);
        // `event_kind` declares `event` and `kind`; a value printed as `event-0000` reaches the
        // first of them. Pushed whole **and** split, because both spellings occur in documents.
        words.extend(field.name.split('_'));
    }
    words.join(" ")
}

/// Check a rendered document against the recipe, with the same instrument that judges code.
///
/// **Every line, and every refusal, collected.** Three of [`lint`]'s four rules are inherited
/// exactly as they stand:
///
/// * **an uncalibrated oracle cannot produce a quotable finding** — so an uncalibrated run
///   refuses rather than passing a document nobody measured;
/// * **`not_belongs` without a reason is refused** — the caller must say why it overrode;
/// * **an absent finding is a finding's non-existence** — which is why a document that produces
///   *no* findings at all is refused, because a renderer emitting nothing would otherwise pass
///   as clean.
///
/// # The fourth rule is the one this function had to reinterpret, and the reason is measured
///
/// `lint`'s rule 3 turns `no_reading` into a **refusal**, and this module's first version
/// documented a branch that could never execute — it counted declines from a
/// `Verdict::NoReading` that `lint` will never hand back, under a comment claiming *"reaching
/// here means the line was answered."* Reaching there meant the opposite. That is C-65.3's
/// shape exactly: a branch that cannot execute, described as though it could.
///
/// Blank lines and `## headings` are `NothingToJudge` — correctly, since a heading has no
/// vocabulary to check. A document is **mostly** those, so counting each as a failure would
/// make every document unreadable, and a check nobody can pass is not a check.
///
/// **So a decline is caught by its own [`RefusalKind`] and re-counted, not re-licensed.** By
/// kind, never by substring: matching on `detail()` would make the classification depend on
/// prose, and prose is the one thing in this project that changes without a compile error.
/// `judged == 0` still refuses, so a document of *nothing but* declines is caught, and
/// [`DocumentVerdict::is_substantive`] catches one that is *mostly* declines.
pub fn check(
    document: &str,
    recipe: &Recipe,
    vocabulary: &[String],
    calibration: &Calibration,
    oracle: &dyn Oracle,
) -> Result<DocumentVerdict, RefusalSet> {
    let mut refusals = RefusalSet::new();
    let mut judged = 0usize;
    let mut declined = 0usize;
    let mut convicted: Vec<String> = Vec::new();

    // **The recipe the judge reads is the anchors' titles, not the decisions' whole bodies.**
    //
    // The first version passed `recipe.text` — every anchor's full body, which for three
    // anchors is several kilobytes of ledger prose. It worked and it was **unusable**: one
    // refusal quoted the entire recipe, and a document with forty lines produced forty
    // multi-kilobyte refusals. A report a reader cannot read is a report some readers will
    // skip.
    //
    // The titles are also the *right* answer rather than a smaller one. A document's authority
    // is **which decisions it was derived from**; the bodies are what the decisions said, and
    // a renderer that paraphrases a body is summarising, which C-62.2 forbids. So the judge is
    // handed the anchors and nothing else.
    let authority = recipe
        .sources
        .iter()
        .map(|anchor| anchor.render())
        .collect::<Vec<_>>()
        .join("\n");

    // **And the renderer's declared vocabulary is licensed, because it is declared.**
    // `RENDERER_VOCABULARY` exists precisely so the connective prose has an auditable source;
    // a declared list that is not passed to the judge is a list that licenses nothing, and the
    // first run of this function proved it by convicting the renderer's own headings.
    //
    // `vocabulary` carries the rest — the grammar's closed enums, the wire schema's field
    // names, and the solution's own words — so a caller supplies them from where they are
    // declared rather than this module guessing.
    let mut licensed = recipe.licensed_terms();
    licensed.extend(crate::oracle::terms(&RENDERER_VOCABULARY.join(" ")));
    for phrase in vocabulary {
        licensed.extend(crate::oracle::terms(phrase));
    }

    for (number, line) in document.lines().enumerate() {
        let request = LintRequest {
            subject: format!("{TITLE} line {}", number + 1),
            line: line.to_string(),
            recipe: authority.clone(),
            licensed: licensed.clone(),
        };
        match lint(oracle, &request, calibration, None) {
            Ok(_finding) => {
                // Rule 3 turns a decline into an `Err`, so a reading arriving here *was*
                // answered. Counted as judged, which is what it is.
                judged += 1;
            }
            Err(found) if is_a_decline(&found) => {
                // **Neither a pass nor a failure** — the instrument saying this line has no
                // vocabulary to judge. Counted, and refused only if it is all there is.
                declined += 1;
            }
            Err(found) => {
                // **Collected, not returned.** A reader fixing a document needs every bad line
                // in one pass, which is the whole reason `RefusalSet` exists.
                refusals.absorb(found);
                convicted.push(format!("line {}: {line}", number + 1));
            }
        }
    }

    if !refusals.is_empty() {
        return Err(refusals);
    }

    // **The vacuity check.** A document that convicted nothing is either wholly licensed or
    // wholly empty, and only one of those is a document. Refusing here is what stops an empty
    // renderer from passing.
    if judged == 0 {
        refusals.refuse(
            RefusalKind::NoReading,
            format!(
                "{TITLE} produced no line the judge could read at all, so it convicted nothing \
                 because there was nothing in it. A document that is perfect and a document that \
                 is empty print identically, and this is the only place they differ"
            ),
        );
        return Err(refusals);
    }

    Ok(DocumentVerdict {
        lines: document.lines().count(),
        judged,
        declined,
        convicted,
    })
}

/// Whether a refusal is the judge's *"I decline to read this"* rather than a conviction.
///
/// **By kind, never by substring.** Matching on `detail()` would make the classification depend
/// on prose — and prose is the one thing in this project that changes without a compile error.
/// Every refusal in the set must be a decline, because a set that mixed the two would be
/// absorbed whole and a conviction would be counted as a decline.
fn is_a_decline(refusals: &RefusalSet) -> bool {
    !refusals.is_empty()
        && refusals
            .all()
            .iter()
            .all(|refusal| refusal.kind() == RefusalKind::NoReading)
}

/// What the check measured. **The counts, and the empty list of convictions** — a
/// [`DocumentVerdict`] exists only when nothing was convicted, so its type is the claim.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentVerdict {
    pub lines: usize,
    /// Lines the judge answered.
    pub judged: usize,
    /// Lines the judge **declined** — headings, blanks, and anything else with no vocabulary
    /// to check. Not a pass: `no evidence` is not `evidence of correctness`.
    pub declined: usize,
    /// Always empty when this value exists. **Carried anyway**, so a caller that wants the
    /// evidence has it in hand rather than having to take the type's word.
    pub convicted: Vec<String>,
}

impl DocumentVerdict {
    /// Whether the document was worth reading at all: something was judged, and it was not
    /// mostly declines.
    ///
    /// **A majority, not an equality.** A document of headings and blank lines declines
    /// constantly and is still a document; requiring `declined == 0` would refuse the empty
    /// heading shell that the vacuity check already catches, and would force a renderer to pad
    /// itself with judgeable lines to satisfy a gate. Half is the line: past it the document
    /// is mostly something the instrument cannot read.
    pub const fn is_substantive(&self) -> bool {
        self.judged > 0 && self.declined * 2 <= self.lines
    }
}

/// The judge this module calibrates against, named once so the gate and the tests cannot pick
/// different instruments.
pub fn structural() -> StructuralJudge {
    StructuralJudge
}

/// Whether the record has anything to derive documentation from, for a given solution's anchors.
///
/// **A convenience, and a refusing one.** A document derived from no anchor is unbacked, and
/// [`render`] prints that fact — this only lets a caller ask first.
pub fn has_sources(recipe: &Recipe) -> bool {
    !recipe.sources.is_empty()
}

/// The anchors a solution's document is derived from, for a caller that navigates.
pub fn sources_of(recipe: &Recipe) -> &[Anchor] {
    &recipe.sources
}

/// Check a document against the recipe, the solution's own vocabulary, and the judge.
pub fn check_against(
    document: &str,
    recipe: &Recipe,
    solution: &Solution,
    calibration: &Calibration,
) -> Result<DocumentVerdict, RefusalSet> {
    check(
        document,
        recipe,
        &declared_vocabulary(solution),
        calibration,
        &structural(),
    )
}

/// The unused-root guard: a caller that has no workspace cannot claim a document is backed.
///
/// **Deliberately a function rather than a `PathBuf` field**, because holding a root is not
/// holding a spec, and the record's rule is that a derived document must not extend the record.
pub fn requires_a_record(root: &Path) -> bool {
    root.join("docs").join("DESIGN-SPEC.md").is_file()
}
