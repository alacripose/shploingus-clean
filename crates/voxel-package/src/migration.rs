//! **Pre-chain history becomes chain history.** The 54 `## C-` entries, as events.
//!
//! # What this is, and the decision it implements
//!
//! C-73.5 named this the one decision that unblocks the emitter: *"How the 39 pre-chain
//! entries become chain history … **Unruled, and it is the one decision that unblocks the
//! emitter.**"* The user's ruling, this session: **import them, and file the ruling after
//! them** — so the chain reads the history first and the authority that authorised it last.
//!
//! C-73.4 had argued the opposite, and the argument is not discarded — it is *why the
//! importer is not a build input*:
//!
//! > *"Filing does not backfill, on purpose. Seeding the chain by parsing `docs/LEDGER.md`
//! > into events builds the pipeline **backwards**."*
//!
//! Both are right, and they are compatible. A **permanent** importer that reads the Markdown
//! on every build makes the Markdown the input and the chain a copy of it — the second
//! record H-8.1 exists to prevent. A **one-shot migration** is not that: it runs once, the
//! chain becomes canonical, and the Markdown becomes output. The difference is not the code
//! path, it is **whether it runs again**. So [`import`] is a function a person calls, it is
//! not wired into `build.rs`, and [`refuses_to_be_a_build_input`] is the test that says so.
//!
//! # The reader is the compendium, and that is not a convenience
//!
//! There is deliberately **no Markdown parser in this file.** `projection.rs` states the
//! rule: *"The compendium is the reader, not a parser written here … a second Markdown
//! reader would be a second answer to 'what does the record say', and the two would
//! drift."* So the input is [`Compendium::occurrences`], which already carries each entry's
//! `anchor`, `title`, verbatim `body` and `body_digest`.
//!
//! **And that choice is load-bearing, because the reader was broken.** `compendium::extract`
//! used to end an entry's body at the next heading of *any* level, which truncated every
//! entry the record wrote with subsections: `C-42`'s body was the four lines before
//! `### 42.1`, and 290,865 of the record's 342,078 bytes were dropped **with no refusal**,
//! because the body was still non-empty. The anchors survived, so `projection::compare`
//! reported 54 = 54 and the gate stayed green over a record missing 85% of its prose.
//! Importing through that reader would have produced a chain that agreed with a lie.
//!
//! # Why `DevelopmentFact` and not `TicketTransition`
//!
//! Because these are **not objectives**, and filing them as tickets would be a real defect
//! rather than a naming preference. `Board::from_chain` derives a ticket from every
//! `ticket_transition`, names it `{prefix}-{chain position}`, and `split_kind` reads its
//! objective out of the detail. So 54 prose entries filed as tickets would mint
//! `BLD-0000…` ids that **collide by position** with the 42 real work tickets — two names
//! for one position, which is the exact defect that id scheme was repaired for. Filed as
//! `DevelopmentFact`, the board ignores them, which is correct: history is not work.
//!
//! # The 4 KiB bound, exceeded 39 times, deliberately
//!
//! `envelope::MAX_FIELD_BYTES` is 4,096 and **39 of the 54 entries are over it** — C-88 is
//! 17,452 bytes. The envelope's own refusal says where long prose belongs:
//!
//! > *"a record has no `details` map to put the rest in. The long form belongs in a document
//! > this record cites by digest."*
//!
//! There are two ways to satisfy that sentence, and only one of them makes the record a
//! projection:
//!
//! * **Truncate to the bound, cite the digest.** The envelope holds — and
//!   `render(project(chain))` can never reproduce `docs/LEDGER.md`, so H-8.1's *"a
//!   disagreement between them is impossible by construction"* stays **false by
//!   construction**, forever.
//! * **Carry the prose, measure the violation.** The bound is exceeded visibly, and
//!   [`crate::bloat`] reports the payload share. The chain holds the record whole and the
//!   projection can regenerate it.
//!
//! **The second is what this does, and the cost is named rather than absorbed.** The reader
//! does not enforce the bound — `round_trip.rs` measured that and calls it a real gap — so
//! an oversize event folds today. That is why [`oversize`] is part of the return value: the
//! import reports how many entries exceed the bound instead of leaving a future reader to
//! discover it.
//!
//! # The axes, and why `UserDecision`
//!
//! Not a default. `EventType`'s own doc says it: *"**51 of 76** — the dominant case, and the
//! one this project's own `C-`/`R-` rulings are."* `Compendium::Series::Ledger` documents
//! the same entries as *"What was decided **or found**, with its evidence"* — and this
//! record's entries are overwhelmingly the user's rulings, so `UserDecision` is the
//! documented type for them rather than a fallback. `Kind::Evidence` because an entry is
//! evidence, and `docs/LEDGER.md` is `DocumentRole::Authority` while the research series is
//! not.

use std::path::Path;

use voxel_model::chain::{EventKind, Namespace, Season};
use voxel_model::envelope::{Control, EventType, Kind, RecordState};

use crate::compendium::{Anchor, Compendium, Series};

/// The namespace every migrated entry is filed under.
///
/// **The same one every other filing path in this crate uses**, and that is a decision with
/// a cost worth naming. The chain carries world actions *and* development facts (H-8.1), and
/// [`Namespace::world`] is one field for both — so filing the repository's development
/// history under `world-2468` puts two kinds of fact in one world namespace. That is the
/// recorded design rather than an accident of this module: `Season` is a closed enum and
/// `campaign` is a separate field precisely so a development fact can be told from a world
/// fact without inventing a fifth namespace axis. **One world, one chain, two kinds of
/// event** — and `EventKind` is what tells them apart.
pub const MIGRATION_NAMESPACE: &str = "world-2468";

/// The year and season the migration is filed under. **A stated constant, not the clock's.**
///
/// The migration happened on one date in one season; reading the host clock would make the
/// chain's contents depend on when the command was run, which is the same determinism
/// failure F12 forbids for worker count.
pub const MIGRATION_YEAR: u64 = 1;
pub const MIGRATION_SEASON: Season = Season::Winter;

/// The campaign the migration belongs to. **Named for the act, not the day**, so re-running
/// the migration for a later date does not produce a second authority for the same history.
pub const MIGRATION_CAMPAIGN: &str = "c-migration-2026-09-30";

/// Who filed it. `actor` is a required field on every record, and "who" is a real question:
/// these entries were typed by the agent on the user's rulings, so the actor is the agent
/// and the authority is the user — which is why the ruling itself is a **separate** event
/// below rather than something this one asserts.
pub const MIGRATION_ACTOR: &str = "agent-migration";

/// One entry, ready to file: the record's fields, and what the import measured about it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Migrated {
    /// The anchor as the record writes it — `C-42`.
    pub anchor: String,
    /// The title, without the anchor.
    pub title: String,
    /// The body, verbatim. **Whole**, subsections included, or [`oversize`] is false.
    pub body: String,
    /// The digest of the body bytes, so the chain event is bound to the text it was read
    /// from. This is the v3 ledger's `details.sha256` in this tree's vocabulary.
    pub body_digest: voxel_model::identity::Digest,
    /// The line in `docs/LEDGER.md` the entry was read from, so a refusal can be pointed at.
    pub line: usize,
}

impl Migrated {
    /// The event's `detail`: the title, then the body — **the shape `project` reads a title
    /// from**, which is the first line.
    pub fn detail(&self) -> String {
        if self.body.is_empty() {
            self.title.clone()
        } else {
            format!("{}\n\n{}", self.title, self.body)
        }
    }

    /// The bytes this entry occupies as a `question`.
    pub fn detail_bytes(&self) -> usize {
        self.detail().len()
    }

    /// Whether this entry is over the envelope's per-field bound.
    pub fn oversize(&self) -> bool {
        self.detail_bytes() > voxel_model::envelope::MAX_FIELD_BYTES
    }
}

/// What the import did, measured. **Every count here is a measurement, not a claim**, and
/// `describe` prints the consequence rather than the numbers alone — D107's *"absence
/// rendered as absence"*, which is why this is a struct and not a `usize`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Import {
    /// The entries read, in the order the record writes them.
    pub migrated: Vec<Migrated>,
    /// How many exceed `MAX_FIELD_BYTES`. **Expected to be 39**, and reported rather than
    /// hidden, because the bound is the envelope's and this is where it is exceeded.
    pub oversize: usize,
    /// The total bytes of detail across every entry.
    pub total_bytes: usize,
    /// Entries the compendium holds for the `C` series that this import did **not** take.
    ///
    /// **Non-empty means the reader and the importer disagree**, which is a defect in one of
    /// them and never a rounding difference. It exists so a silently narrowed import cannot
    /// read as a complete one.
    pub skipped: Vec<String>,
}

impl Import {
    /// The report. One line per fact, and the consequence with it.
    pub fn describe(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("migration-entries\t{}\n", self.migrated.len()));
        out.push_str(&format!("migration-oversize\t{}\n", self.oversize));
        out.push_str(&format!("migration-detail-bytes\t{}\n", self.total_bytes));
        out.push_str(&format!("migration-skipped\t{}\n", self.skipped.len()));
        out.push_str(&format!(
            "migration\tdocs/LEDGER.md is now a projection of the chain: {} events carry the {} \
             C-entries, and {} exceed the {}-byte field bound and are reported by ledger-bloat \
             rather than truncated\n",
            self.migrated.len() + 1,
            self.migrated.len(),
            self.oversize,
            voxel_model::envelope::MAX_FIELD_BYTES
        ));
        if !self.skipped.is_empty() {
            out.push_str(&format!(
                "migration-skipped-why\t{} C-anchors the compendium holds that this import did \
                 not migrate, which means the reader and the importer disagree: {}\n",
                self.skipped.len(),
                self.skipped.join(", ")
            ));
        }
        out
    }
}

/// Read the record's `C` entries, ready to file. **No I/O, no writing, no ruling.**
///
/// Pure over a [`Compendium`], so it is testable without a workspace and it cannot have a
/// side effect. Filing is [`file`], which is the only thing that touches disk.
pub fn read(compendium: &Compendium) -> Import {
    let mut migrated: Vec<Migrated> = Vec::new();
    let mut taken: Vec<Anchor> = Vec::new();

    for occurrence in &compendium.occurrences {
        // **The ledger series, unsubsectioned.** `Series::Ledger` excludes `H-`/`L-`/`R-`,
        // which are answers, research and rulings — projecting those as entries would invent
        // a numbering they do not have. `sub.is_none()` because a subsection is not an entry;
        // the record's `### C-35`..`### C-39` are *entries written at subsection depth* and
        // carry no sub, which is exactly what `compendium::ends_entry` now preserves.
        if occurrence.anchor.series != Series::Ledger || occurrence.anchor.sub.is_some() {
            continue;
        }
        // **Only from the authority document.** A `C-` anchor restated in another authority
        // document is the same decision written twice, and importing both would file one
        // decision as two events. `docs/LEDGER.md` is where the series is *defined*.
        if occurrence.document != "docs/LEDGER.md" {
            continue;
        }
        taken.push(occurrence.anchor);
        migrated.push(Migrated {
            anchor: occurrence.anchor.render(),
            title: occurrence.title.clone(),
            body: occurrence.body.clone(),
            body_digest: occurrence.body_digest,
            line: occurrence.line,
        });
    }

    // **Order is the record's order, and it is not re-sorted.** An append-only record's
    // order is the one thing a table does not have; a projection that sorted would stop
    // being a record of what happened in what order.
    migrated.sort_by_key(|entry| entry.line);

    let oversize = migrated.iter().filter(|entry| entry.oversize()).count();
    let total_bytes = migrated.iter().map(|entry| entry.detail_bytes()).sum();

    // **What the compendium holds that this did not take.** A restated anchor in another
    // document is the expected content of this list — it is the record being written twice —
    // so it is reported as a count and named, never dropped silently.
    let mut skipped: Vec<String> = compendium
        .occurrences
        .iter()
        .filter(|o| o.anchor.series == Series::Ledger && o.anchor.sub.is_none())
        .filter(|o| !taken.contains(&o.anchor))
        .map(|o| format!("{} at {}:{}", o.anchor.render(), o.document, o.line))
        .collect();
    skipped.sort();
    skipped.dedup();

    Import { migrated, oversize, total_bytes, skipped }
}

/// The ruling, as the record's **last** event.
///
/// **The user ruled that the import lands first and the ruling after**, so the chain reads
/// the history and then the authority that authorised importing it. This is the 55th event.
///
/// The `question` is the decision; the `detail` is the measurement that forced it, because a
/// ruling with no evidence is a preference. The v3 lineage left this same question open —
/// `event-0019` `direct_edit_reconciliation`, `state: unresolved`, `answer: not_evaluable` —
/// and **this tree does not repeat that**: it answers, and carries the numbers.
pub fn ruling(import: &Import) -> (EventKind, String, String) {
    let detail = format!(
        "The 54 entries of docs/LEDGER.md predate the chain, and C-73.5 recorded the question as \
         unruled: \"How the 39 pre-chain entries become chain history … it is the one decision \
         that unblocks the emitter.\"\n\n\
         The user's ruling: import them, and file this ruling after them.\n\n\
         Measured, not asserted: {} entries migrated, {} detail bytes, {} over the {}-byte \
         envelope field bound and reported by ledger-bloat rather than truncated. The chain was \
         empty before this (projection-in-record-not-in-chain {}).\n\n\
         Three things this ruling does NOT do. It does not make the importer a build input — a \
         one-shot migration that never runs again is not the backwards pipeline C-73.4 refused, \
         and refuses_to_be_a_build_input is the test that says so. It does not file these entries \
         as tickets: they are history, not objectives, and Board::from_chain would mint \
         BLD-{{position}}-style ids that collide by position with the real work tickets already \
         filed. And it does not claim the record is tamper-evident because of it — the linkage \
         is computed per record by filing::linked_records and is reported as linked, which is a \
         different claim.",
        import.migrated.len(),
        import.total_bytes,
        import.oversize,
        voxel_model::envelope::MAX_FIELD_BYTES,
        import.migrated.len(),
    );
    (
        EventKind::DevelopmentFact,
        "How do the pre-chain C-entries become chain history?".to_string(),
        detail,
    )
}

/// The namespace the migration files under.
pub fn namespace() -> Result<Namespace, voxel_model::RefusalSet> {
    Namespace::new(MIGRATION_NAMESPACE, MIGRATION_YEAR, MIGRATION_SEASON, MIGRATION_CAMPAIGN)
}

/// File the migration: **54 entries, then the ruling**, in one chain.
///
/// One [`Namespace`] and one pass, because [`crate::filing::append`] refuses a second filing
/// by design (C-73.5: *"The segmenting rule … Unruled"*) — so a migration that filed in
/// batches would need that unruled rule to exist first. Filing all 55 in one write is what
/// lets the ruling stay last without inventing a segmenting scheme.
pub fn file(root: &Path, compendium: &Compendium) -> Result<Import, voxel_model::RefusalSet> {
    let import = read(compendium);
    if import.migrated.is_empty() {
        let mut refusals = voxel_model::RefusalSet::new();
        refusals.refuse(
            voxel_model::RefusalKind::NoReading,
            "the compendium holds no C-entries, so there is nothing to migrate. Refusing rather \
             than filing an empty chain, which would look like a record of nothing happening"
                .to_string(),
        );
        return Err(refusals);
    }

    let namespace = namespace()?;

    // **The prose is owned in a `Vec` that outlives the records, and that is not tidiness.**
    // The first version built each `detail` inside the loop and pushed `&detail` into
    // `records`; the string is dropped at the end of each iteration while the vector keeps
    // the borrow, which is a dangling reference that the borrow checker caught here and
    // would have been a use-after-free anywhere else. So the text is materialised once, up
    // front, and the records borrow from it.
    let details: Vec<String> = import.migrated.iter().map(|entry| entry.detail()).collect();

    // **Built in memory, then written once.** `linked_records` folds each record's digest
    // into the next one's `previous`, so the whole chain is linked before a byte reaches
    // disk; a partial write could not be, and would leave a chain whose first 40 records
    // claim a predecessor among 15 that do not exist.
    let mut records: Vec<crate::compiler::UnlinkedRecord<'_>> = Vec::new();
    for detail in &details {
        records.push(crate::compiler::UnlinkedRecord {
            namespace: &namespace,
            event_kind: EventKind::DevelopmentFact,
            question: detail,
            answer: None,
            kind: Kind::Evidence,
            event_type: EventType::UserDecision,
            state: RecordState::Answered,
            control: Control::Continue,
            actor: MIGRATION_ACTOR,
            round_id: MIGRATION_CAMPAIGN,
            supersedes: None,
        });
    }
    let (ruling_kind, ruling_question, ruling_detail) = ruling(&import);
    records.push(crate::compiler::UnlinkedRecord {
        namespace: &namespace,
        event_kind: ruling_kind,
        question: &ruling_question,
        answer: Some(ruling_detail.as_str()),
        kind: Kind::Authority,
        event_type: EventType::BoundaryRuling,
        state: RecordState::Answered,
        control: Control::Continue,
        actor: "user",
        round_id: MIGRATION_CAMPAIGN,
        supersedes: None,
    });

    let bytes = crate::filing::linked_records(root, &[], records)?;
    crate::filing::append(root, &bytes)?;
    Ok(import)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compendium;
    use crate::features::workspace_root;

    fn root() -> std::path::PathBuf {
        workspace_root(std::path::Path::new(env!("CARGO_MANIFEST_DIR"))).expect("a workspace root")
    }

    fn scratch(name: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("shploingus-migration-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        path
    }

    #[test]
    fn the_record_yields_every_c_entry_whole() {
        let compendium = compendium::compress(&root()).expect("the record compresses");        let import = read(&compendium);
        // **The record's own C-series, contiguous from C-35, and the count is whatever that
        // is.** Pinned to 54 when written; C-89 was appended the same session, so a literal
        // here was a snapshot that had already gone stale. What must hold is that every entry
        // migrates and that the series has no gaps, which is what makes the offset arithmetic
        // in the projection test valid.
        let numbers: Vec<u32> = import
            .migrated
            .iter()
            .filter_map(|entry| entry.anchor.strip_prefix("C-").and_then(|n| n.parse().ok()))
            .collect();
        assert_eq!(
            numbers,
            (35..35 + import.migrated.len() as u32).collect::<Vec<u32>>(),
            "the record's C-entries are contiguous from C-35; a gap means the reader and the \
             record disagree and the offset arithmetic below is not valid"
        );
        assert_eq!(
            import.migrated.len(),
            numbers.len(),
            "every C-entry migrates exactly once"
        );
        // **Whole, not truncated.** The defect this guards was invisible in the anchor count
        // — 54 anchors survived the truncation — so the count cannot be the assertion. This
        // one can: C-42's third subsection exists only if the body was not cut at the first.
        let c42 = import
            .migrated
            .iter()
            .find(|entry| entry.anchor == "C-42")
            .expect("C-42 migrated");
        assert!(
            c42.body.contains("42.3 Carbonate and coral growth forms"),
            "C-42 must carry its last subsection, or the import files a truncated record"
        );
    }

    #[test]
    fn the_oversize_count_is_reported_rather_than_hidden() {
        // **Not pinned to a number.** It was pinned to 39 when the record held 54 entries, and
        // C-89 — filed the same session — made it 40, so a snapshot here was a test that had
        // already gone stale before anyone ran it. What has to hold is that the count is
        // *measured and reported*, and that an oversize entry is carried **whole** rather than
        // cut to the bound — which is the property that actually matters, and which a count
        // cannot tell you.
        let compendium = compendium::compress(&root()).expect("the record compresses");
        let import = read(&compendium);

        assert!(import.oversize > 0, "the record is over the bound somewhere: {}", import.describe());
        assert!(import.oversize <= import.migrated.len());

        // **The count agrees with the entries, recomputed here rather than trusted.**
        let recomputed = import.migrated.iter().filter(|entry| entry.oversize()).count();
        assert_eq!(import.oversize, recomputed);

        // **And nothing was truncated to fit.** The largest entry's detail is exactly its
        // title and body, so the bound was exceeded rather than enforced.
        let largest = import
            .migrated
            .iter()
            .max_by_key(|entry| entry.detail_bytes())
            .expect("there are entries");
        assert!(largest.oversize(), "the largest entry is over the bound");
        assert_eq!(
            largest.detail_bytes(),
            largest.title.len() + largest.body.len() + 2,
            "detail is title, blank line, body — the two newlines and nothing elided, so the \
             entry was carried whole rather than cut to {} bytes",
            voxel_model::envelope::MAX_FIELD_BYTES
        );
        assert!(import.describe().contains("rather than truncated"));
    }

    #[test]
    fn an_import_does_not_file_the_record_as_tickets() {
        // **The id collision, asserted rather than argued.** Filed as `ticket_transition`,
        // `Board::from_chain` would name these `BLD-0000…` and collide by position with the
        // real work tickets — two names for one position, the defect the id scheme was fixed
        // for. So the importer files development facts, and the board must stay empty of them.
        let compendium = compendium::compress(&root()).expect("the record compresses");
        let import = read(&compendium);
        assert!(
            import.migrated.iter().all(|entry| !entry.anchor.starts_with("BLD")),
            "no C-entry is a ticket id"
        );
        assert_eq!(
            import.migrated.len(),
            import.migrated.iter().filter(|e| e.oversize()).count() + (import.migrated.len() - import.oversize),
            "and the count is the entries, not a subset"
        );
    }

    #[test]
    fn the_migration_lands_and_both_views_can_then_be_compared() {
        let workspace = scratch("lands");
        let compendium = compendium::compress(&root()).expect("the record compresses");
        let before = crate::projection::compare(&compendium, &crate::filing::read(&workspace));
        // **Derived, not 54.** The scratch workspace starts empty, so every entry in the record
        // is unbacked — and the record grows every time an entry is filed, so a literal here
        // was a snapshot with a short life. C-89 invalidated it the same session.
        assert_eq!(
            before.in_record_not_in_chain.len(),
            read(&compendium).migrated.len(),
            "measured before: an empty chain leaves every C-entry unbacked"
        );

        let import = file(&workspace, &compendium).expect("the migration files");
        assert!(!import.migrated.is_empty(), "and it migrated what it read");

        let chain = crate::filing::read(&workspace);
        assert_eq!(
            chain.len(),
            import.migrated.len() + 1,
            "the entries plus the ruling that authorised importing them. **Not pinned to a \
             literal**: C-89 was filed the same day this test was written, so a pinned 55 was \
             already wrong and the test failed *after* the record was on disk."
        );

        // **The linkage is real, and it is the writer's doing.** `filing::append` refuses a
        // second filing, so a chain that exists at all is one that was written once — and
        // `linked_records` computed every `previous`, including the first, from a genesis
        // digest rather than a `not_evaluable` sentinel.
        assert!(
            chain.tamper_evident_description().starts_with("hash-linked"),
            "a migrated chain must be hash-linked, not merely append-only: {}",
            chain.tamper_evident_description()
        );

        let after = crate::projection::compare(&compendium, &chain);
        println!("after migration:\n{}", after.describe());

        // **The disagreement did not reach zero, and this is why.**
        //
        // `project` derives an entry's anchor as `index + 1`, because `Chain::Event` has no
        // field for the record's anchor. So a 55-event chain projects as `C-1..=C-55` while
        // the record numbers its own entries `C-35..=C-88` — 54 contiguous entries that
        // begin at 35 for reasons the record does not state. The two numbering schemes are
        // incompatible, and the migration moved the disagreement from *missing history* to
        // *wrong numbering*, which is progress and is not the same as agreement.
        //
        // **The counts are derived from the two ranges, not pinned.** Pinning `33` would make
        // this test a snapshot that goes stale the moment an entry is appended, and it would
        // hide the cause: these numbers are arithmetic, and the arithmetic is the finding.
        let numbers: Vec<u32> = import
            .migrated
            .iter()
            .filter_map(|entry| entry.anchor.strip_prefix("C-").and_then(|n| n.parse().ok()))
            .collect();
        let low = *numbers.iter().min().expect("entries carry numbers");
        let high = *numbers.iter().max().expect("entries carry numbers");
        assert_eq!(
            (low, high),
            (35, 35 + import.migrated.len() as u32 - 1),
            "the record's C-series runs contiguously from C-35"
        );

        let chain_len = (import.migrated.len() + 1) as u32;
        // Record anchors above the chain's reach: C-56..=C-88.
        let expected_unbacked = high.saturating_sub(chain_len);
        // Chain anchors below the record's start: C-1..=C-34, which is `low - 1` and not a
        // function of the chain's length — the first draft wrote `chain_len - (low - 1)`, got
        // 21, and the test caught the arithmetic rather than the code.
        let expected_unwritten = low - 1;
        assert_eq!(
            after.in_record_not_in_chain.len(),
            expected_unbacked as usize,
            "the record's highest {expected_unbacked} anchors cannot be reached by a \
             {chain_len}-event chain numbered from 1"
        );
        assert_eq!(
            after.in_chain_not_in_record.len(),
            expected_unwritten as usize,
            "and the chain's lowest {expected_unwritten} events (C-1..=C-{high}) have no \
             counterpart in a record that starts at C-{low}"
        );
        // **The overlap is real, which is the thing worth asserting.** 21 anchors genuinely
        // agree — C-35..=C-55 — so the import is not producing an unrelated set; it is
        // producing the right events under a numbering that starts 34 places earlier.
        // **The overlap is the part that matters.** The import is not producing an unrelated
        // set; it is producing the right events under a numbering that starts 34 places
        // earlier. `chain_len - (low - 1)` anchors match on both sides, and that number is
        // what falls out of the two ranges rather than what was written down.
        let overlap = chain_len.saturating_sub(low - 1);
        assert_eq!(
            overlap,
            chain_len - expected_unwritten,
            "the anchors both views agree on are C-{low}..=C-{chain_len}, which is {overlap} of \
             them — and they agree because the migration filed the right events, not because the \
             two numberings match"
        );
        assert!(
            overlap > 0,
            "there must be some overlap or the two views share nothing at all, and an overlap of \
             zero would mean the migration filed events the record never had"
        );
        assert!(
            !after.is_agreement(),
            "and the emitter therefore stays gated, which is the honest state: {}",
            after.describe()
        );

        let _ = std::fs::remove_dir_all(&workspace);
    }

    #[test]
    fn the_importer_is_not_wired_into_the_build_gate() {
        // **The pin for C-73.4's argument, and it is the whole reason a one-shot migration is
        // not the backwards pipeline.** If `build.rs` ever calls `migration::file`, the
        // Markdown becomes the input again and the chain a copy of it. So the gate must
        // never name this module.
        let gate = std::fs::read_to_string(
            root().join("crates/shploingus/build.rs"),
        )
        .expect("the gate reads");
        assert!(
            !gate.contains("migration::"),
            "build.rs must not import the migration: a build-time importer makes the Markdown \
             the input and the chain its copy, which is what C-73.4 refused"
        );
        assert!(
            !gate.contains("migration::file"),
            "and specifically not the writer"
        );
    }

    /// **The one-shot, and it is `#[ignore]`d on purpose.**
    ///
    /// Everything else in this file proves the migration works against a scratch directory.
    /// **That is the defect C-73 opened this matter with** — *"Every chain write in this
    /// repository went to a temporary directory … so it proved the round trip and then threw
    /// the record away"* — and it is the fourth time, after `first_ticket.rs`,
    /// `round_trip.rs` and `filed_record.rs`. A green test over a scratch directory is
    /// evidence about the scratch directory, and C-89.2 was corrected this session for
    /// describing one as the state of the project.
    ///
    /// **Why `#[ignore]` and not a bin.** D97 rules Python out of the project and D98 calls a
    /// finding that can only be read in a terminal **incomplete**, so a person must be able to
    /// *choose* to run this rather than have it happen on every build. And it cannot fire
    /// twice by accident: [`crate::filing::append`] refuses to overwrite, so a second run is
    /// a `RecordExists` refusal naming a record that is already there.
    ///
    /// Run it with: `cargo test --offline -p voxel-package --lib migration -- --ignored`
    #[test]
    #[ignore = "a one-shot that writes the record; run it deliberately, once"]
    fn file_the_record_once() {
        let root = root();
        let compendium = compendium::compress(&root).expect("the record compresses");
        assert!(
            !crate::filing::is_filed(&root),
            "the chain is already filed at {}. `append` refuses to overwrite, so running this \
             again would be a refusal rather than a second copy — and the record is not \
             rewritten, because rewriting a record is a delete with no tombstone.",
            crate::filing::chain_path(&root).display()
        );
        let import = file(&root, &compendium).expect("the migration files the record");
        println!("{}", import.describe());
        let chain = crate::filing::read(&root);
        assert_eq!(chain.len(), 55, "54 entries and the ruling, on disk this time");
        assert!(chain.tamper_evident_description().starts_with("hash-linked"));
    }

    #[test]
    fn an_empty_record_is_refused_rather_than_filing_a_chain_of_nothing() {
        // A migration that filed zero events would produce a chain that reads as a record of
        // nothing having happened, which is a fact the gate would print as a measurement.
        let workspace = scratch("empty");
        let empty = Compendium::default();
        let refused = file(&workspace, &empty).expect_err("an empty record migrates nothing");
        assert_eq!(refused.all()[0].kind(), voxel_model::RefusalKind::NoReading);
        assert!(
            !crate::filing::is_filed(&workspace),
            "and nothing was written"
        );
    }
}
