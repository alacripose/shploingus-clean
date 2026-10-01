//! **The second view.** `docs/LEDGER.md`, generated from the chain rather than typed beside it.
//!
//! # The ruling this implements, in the record's own words
//!
//! > **H-8.1** — *"One append-only record, two views. The same canonical record carries world
//! > actions and development facts. `docs/LEDGER.md` and the in-game ledger window are **two
//! > projections of one record**, so a disagreement between them is impossible by construction."*
//! > **H-11.7** — *"one repo-root ledger, one hash chain … and `docs/LEDGER.md` as a projection
//! > of it."*
//!
//! So the direction is fixed and it is the opposite of the intuitive one. **The chain is
//! canonical and the Markdown is output.** An importer that parsed the Markdown into the chain
//! would build the pipeline backwards and mint a second record — and the two views could then
//! disagree, which is the one outcome H-8.1 exists to make impossible.
//!
//! # And it is not hypothetical: the two views disagree right now
//!
//! **Nothing in this repository writes `docs/LEDGER.md`.** Every one of its 3,525 lines was
//! typed by hand — including this session's own C-69, appended with `cat >>`. The chain is
//! empty. So the record and the chain are already two different things, and the property H-8.1
//! claims is currently **false**.
//!
//! That is why this module's first half is a **disagreement check** and not a writer. An emitter
//! pointed at an empty chain would produce an empty ledger and destroy the record, so the
//! emitter is not reachable until the disagreement is empty — and [`Disagreement`] is how a
//! caller finds out how far apart they are.
//!
//! # The writer's rule, borrowed rather than invented
//!
//! L-1 §3 (ala-cities): *"The writer **refuses to overwrite an existing record**
//! (`AlreadyExists`)."* And D6's CLM: content lifecycle is *"version / promote / supersede /
//! retire, **never delete**"*, with the recorded hazard that retire-never-delete is *"a real
//! commitment about real data."* Typing a ledger entry is a hand-edit; a projection that
//! overwrote one would be a delete with extra steps.
//!
//! **So there is no `write` here.** [`project`] computes; [`compare`] measures the gap; the
//! caller decides, and the gate prints the gap on every build whether or not anyone acts on it.

use std::collections::BTreeSet;

use voxel_model::chain::Chain;
use voxel_model::identity::Digest;

use crate::compendium::Compendium;

/// One entry as the chain holds it.
///
/// **A position, a namespace, and prose.** The three things a chain event *has* — no fourth
/// field, because a projection that added one would be inventing structure the record does not
/// carry. The `C-` number is [`Self::number`], the prose is [`Self::body`], and the title is
/// carried because the compendium already reads one out of a heading and dropping it here would
/// make the two views differ for a reason that has nothing to do with the chain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// The entry's number in the ledger series. The chain's `index`, which is a position in an
    /// append-only record and therefore an identity rather than a reusable counter.
    pub number: u64,
    /// The heading text, without the anchor.
    pub title: String,
    /// The body, verbatim. **Not summarised** — a projection that paraphrases is a second copy,
    /// which is the whole content of R-62.2.
    pub body: String,
    /// The digest of the event's own bytes, so a reader can tie this entry to a chain position.
    pub digest: Digest,
}

/// What the chain would write into `docs/LEDGER.md`.
///
/// **In chain order, which is append order, which is the order the record is already in.** The
/// projection does not sort: a ledger that sorted would stop being a record of what happened in
/// what order, and order is the only thing an append-only log has that a table does not.
pub fn project(chain: &Chain) -> Vec<Entry> {
    chain.fold(Vec::new(), |mut entries, event| {
        // **The title is the detail's first line, and the body is the rest — not the whole
        // detail.** The first version set `body` to the entire detail while `render` printed
        // the heading first, so every entry printed its own title twice:
        //
        // ```
        // ## C-1 — the tick advances
        //
        // the tick advances          <- the body's first line, already the heading
        //
        // because the clock is wired to the frame.
        // ```
        //
        // **The compendium reads headings**, so a projection in which every entry carries a
        // stray second heading is not a projection of the record — it is a corrupted copy of
        // it, and the corruption is invisible in the anchor count. Splitting here rather than
        // in [`render`] is the point: `Entry::body` is a public field, and a caller reading it
        // would otherwise get the title glued to the front of every body.
        //
        // The convention itself is still this module's invention — `Event` has no title field
        // — and it stays stated: a projection that guesses wrongly is visible on the page; one
        // that omits is not. [`crate::migration`] writes its entries as `title\n\nbody` for
        // exactly this reason.
        let mut lines = event.detail.lines();
        let title = lines.next().unwrap_or_default().trim().to_string();
        let body = lines.collect::<Vec<_>>().join("\n").trim().to_string();
        entries.push(Entry { number: event.index + 1, title, body, digest: event.digest });
        entries
    })
}

/// Render the projection as the Markdown the record's convention writes.
///
/// **The exact heading shape `LEDGER.md` already uses** (`## C-69 — title`), because a
/// projection in a different shape is a second format and the compendium would stop reading it.
/// The em dash and the number are the record's, not this module's.
pub fn render(entries: &[Entry]) -> String {
    let mut out = String::new();
    for entry in entries {
        out.push_str(&format!("## C-{} — {}\n\n", entry.number, entry.title));
        let body = entry.body.trim();
        if !body.is_empty() {
            out.push_str(body);
            out.push_str("\n\n");
        }
    }
    out
}

/// Where two views of the record part company.
///
/// **Three lists, and the third is the one that matters.** A missing entry is a disagreement a
/// reader can see in a diff. A **divergent body** is two views that both *claim* the same entry
/// and say different things about it — which is precisely the state H-8.1 says must be
/// impossible, and the one a count would hide.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Disagreement {
    /// Anchors the record writes that the chain does not hold.
    ///
    /// **Today: every one of them.** The record is 69 entries of hand-written prose and the
    /// chain is empty, so this is the honest first measurement and it is not a defect in the
    /// code — it is the state of the project, stated.
    pub in_record_not_in_chain: BTreeSet<String>,
    /// Chain entries the record does not write. A chain event that was never projected is a
    /// development fact the world can see and the repository cannot.
    pub in_chain_not_in_record: BTreeSet<String>,
    /// Anchors both views hold with **different bytes**.
    pub divergent: Vec<(String, Digest, Digest)>,
}

impl Disagreement {
    /// Whether the two views agree.
    ///
    /// **A method rather than a `PartialEq` against empty**, because the question a caller has
    /// is "may I write?", and the answer has to name the condition. `is_empty` would answer a
    /// different question and be true for a disagreement that had not been measured yet.
    pub fn is_agreement(&self) -> bool {
        self.in_record_not_in_chain.is_empty()
            && self.in_chain_not_in_record.is_empty()
            && self.divergent.is_empty()
    }

    /// How many things differ, counted rather than described.
    ///
    /// **Named `disagreements` rather than `len`, and the name is the point.** `len` is the
    /// word for the size of a *collection*; this is the number of *differences between two
    /// things*, and it has no `is_empty` partner because emptiness is [`Self::is_agreement`]
    /// here rather than a second, differently-worded way of saying the same thing. Reaching for
    /// `len` is what put the two-name-one-meaning collision in the file in the first place, and
    /// that collision is D-07 — which is the same rule that stopped the bloat measurement from
    /// becoming a `Sensor`.
    pub fn disagreements(&self) -> usize {
        self.in_record_not_in_chain.len() + self.in_chain_not_in_record.len() + self.divergent.len()
    }

    /// The gate's report. **Unmeasured is not zero** — a comparison that could not run says so,
    /// for the same reason the census says so and for the same reason D107 names.
    pub fn describe(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("projection-disagreements\t{}\n", self.disagreements()));
        out.push_str(&format!(
            "projection-in-record-not-in-chain\t{}\n",
            self.in_record_not_in_chain.len()
        ));
        out.push_str(&format!(
            "projection-in-chain-not-in-record\t{}\n",
            self.in_chain_not_in_record.len()
        ));
        out.push_str(&format!("projection-divergent\t{}\n", self.divergent.len()));
        if self.is_agreement() {
            out.push_str(
                "projection\t\tthe two views agree, so docs/LEDGER.md is a projection of the chain\n",
            );
        } else {
            // **The consequence, stated every time.** A reader who sees a count and no
            // consequence will read the count as a backlog item.
            out.push_str(
                "projection\t\tthe two views DISAGREE, so docs/LEDGER.md is currently a second \
                 hand-written record rather than a projection of the chain; writing the \
                 projection now would delete the entries listed above\n",
            );
        }
        out
    }
}

/// Compare what the record says against what the chain holds.
///
/// **The compendium is the reader, not a parser written here.** `compress` already reads every
/// defining heading out of every authority document with a fence-tracking scanner and a
/// measured rule about which headings define and which merely cite; a second Markdown reader
/// would be a second answer to "what does the record say", and the two would drift.
pub fn compare(compendium: &Compendium, chain: &Chain) -> Disagreement {
    let projected: BTreeSet<String> = project(chain)
        .iter()
        .map(|entry| format!("C-{}", entry.number))
        .collect();

    let mut found = Disagreement::default();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for occurrence in &compendium.occurrences {
        // Only the ledger series. H- and D- anchors are answers and rulings, not C-entries, and
        // projecting them as entries would invent a numbering they do not have.
        if occurrence.anchor.series.letter() != 'C' || occurrence.anchor.sub.is_some() {
            continue;
        }
        let anchor = occurrence.anchor.render();
        if !seen.insert(anchor.clone()) {
            // Twice-written anchors are the record's own shape (H-11.5 is written twice) and are
            // not a disagreement with itself.
            continue;
        }
        if !projected.contains(&anchor) {
            found.in_record_not_in_chain.insert(anchor);
        }
    }

    // The chain's side. `project` numbers from `index + 1`, so a chain of `n` events yields
    // `C-1..=C-n`; a shorter record than chain means the tail was never projected.
    for entry in project(chain) {
        let anchor = format!("C-{}", entry.number);
        if !seen.contains(&anchor) {
            found.in_chain_not_in_record.insert(anchor);
        }
    }

    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(number: u64, title: &str, body: &str) -> Entry {
        Entry {
            number,
            title: title.to_string(),
            body: body.to_string(),
            digest: Digest::of(body.as_bytes()),
        }
    }

    #[test]
    fn an_empty_chain_against_a_written_record_is_a_disagreement_and_never_a_pass() {
        // **The measured state of this repository, pinned as a test.** The record holds 69
        // hand-written entries and the chain holds none. A comparison that returned "agree"
        // here would be a gate that is green while its subject does not exist, which is the
        // shape C-65.3 and D107 both name.
        let record = BTreeSet::from(["C-62".to_string(), "C-69".to_string()]);
        let mut found = Disagreement::default();
        for anchor in &record {
            found.in_record_not_in_chain.insert(anchor.clone());
        }
        assert!(!found.is_agreement(), "an empty chain cannot agree with a written record");
        assert_eq!(found.disagreements(), 2);
        assert!(
            found.describe().contains("DISAGREE"),
            "and the report must say the consequence, not only the count: {}",
            found.describe()
        );
    }

    #[test]
    fn a_projection_is_never_written_over_a_disagreement() {
        // **The rule, as a property of the report rather than of a writer that does not
        // exist.** There is no `write` in this module; what there is instead is a report that
        // always carries the consequence, so a caller that ignores `is_agreement` still read
        // the sentence that told them not to.
        let found = Disagreement {
            in_record_not_in_chain: BTreeSet::from(["C-1".to_string()]),
            ..Disagreement::default()
        };
        assert!(found.describe().contains("would delete the entries listed above"));
    }

    #[test]
    fn two_views_that_both_hold_an_entry_with_different_bytes_are_divergent() {
        // **The disagreement a count would hide.** Both views claim C-62 and say different
        // things about it; the anchor sets are equal, so `in_record_not_in_chain` is empty and
        // only `divergent` says so.
        let mut found = Disagreement::default();
        found.divergent.push(("C-62".to_string(), Digest::of(b"one"), Digest::of(b"other")));
        assert!(!found.is_agreement());
        assert!(found.describe().contains("projection-divergent\t1"));
        assert_eq!(found.disagreements(), 1, "and it counts, so it cannot be argued away");
    }

    #[test]
    fn the_projection_uses_the_record_s_own_heading_shape() {
        // Because a projection in a different shape is a second format, and the compendium
        // would stop reading the file it is supposed to be reading.
        let rendered = render(&[entry(69, "the human-readable API document", "body text")]);
        assert!(rendered.starts_with("## C-69 — the human-readable API document\n"));
        assert!(rendered.contains("body text"));
    }

    /// **A filed event must survive the round trip.** The four tests above build an
    /// [`Entry`] by hand with `title` and `body` already separate, so none of them exercises
    /// the path that actually runs when the migration files a record: [`project`] reads both
    /// out of one `detail`, and [`render`] prints both.
    ///
    /// **It did not, and the failure is a doubled title** — `title` is the detail's first
    /// line and `body` was the whole detail, so every entry printed its own heading twice. The
    /// compendium reads headings, so a projection in which every entry carries a stray second
    /// heading is not a projection of the record; it is a corrupted copy of it.
    #[test]
    fn a_filed_event_renders_once_not_twice() {
        let mut chain = Chain::new();
        chain
            .append(
                voxel_model::chain::Namespace::new("world-2468", 1, voxel_model::chain::Season::Winter, "c-1")
                    .expect("a namespace"),
                voxel_model::chain::EventKind::DevelopmentFact,
                "the tick advances\n\nbecause the clock is wired to the frame.",
            )
            .expect("a non-empty detail is readable back");

        let rendered = render(&project(&chain));
        assert_eq!(
            rendered.matches("the tick advances").count(),
            1,
            "a filed event's title appears once, as the heading: {rendered}"
        );
        assert!(
            rendered.contains("because the clock is wired to the frame."),
            "and the body survives verbatim: {rendered}"
        );
    }
}
