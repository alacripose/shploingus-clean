//! **The civilians' evidence, and the ladders it is graded on.**
//!
//! `architecture/workspace.dsl` states the binding sentence for this module's whole reason to
//! exist: the civilian layer's *"evidence is the proof of work for solutions, **graded against
//! `axis.rs`**"*. This is the reader half of the seam pair — it **reports** and
//! [`crate::decision::rule_for_civilian`] **decides**, exactly as `census::observe` reports the
//! crate grid and `rule_for_axis` decides it, and as `features::observe` reports the rows and
//! `rule_for_feature` decides them. *"Read, then rule"*, and the rule lives behind the seam
//! rather than here.
//!
//! # Why the observation is the **record**, and not the citizens
//!
//! The civilians themselves are staged but not yet declared (`crates/voxel-model/src/civilian/`,
//! eleven files), so their rows have no readable representation yet. What **is** landed is the
//! governed record they will write through: `voxel_model::world`'s book, transition, change and
//! govern modules, and `voxel_shapers::earthworks`' widened `Modification`.
//!
//! So this observes **whether the governed record carries each ladder's witness** — whether the
//! civilians have somewhere on that ladder to put evidence at all. That is a real measurement
//! per ladder (seven distinct file-and-token readings, not one fact graded seven ways), it is
//! the same method `census::observe` already uses on the crate grid, and it is the half that can
//! be measured honestly today.
//!
//! # The second reading, and it landed with the rows
//!
//! The first reading grades the **instrument**: whether the governed record has anywhere on a
//! ladder to put evidence at all. The second reads the **rows** — and the rows are now on the
//! record: every ticket filing is a `ticket_transition` event whose `answer` is the evidence it
//! carries, and the game's ledger (`docs/LEDGER.md`) is filed as one `CSE` ticket per entry whose
//! answer is a **citation**: `docs/LEDGER.md lines A..=B; sha256:<hex>`.
//!
//! So the ticket-evidence ladder ([`AxisKind::TicketEvidence`]) is read twice: its witness (the
//! record has somewhere to put a ticket's evidence) **and its rows** — one [`TicketRow`] per
//! ticket the record carries a citation for, with the cited document opened and the cited
//! `sha256` checked against the bytes still on disk. A row that does not verify is **drift**
//! made visible: the document moved since the ticket was filed, or the citation never parsed.
//! Whether that is a refusal is decided by [`crate::decision::rule_for_civilian`], not here —
//! this module reads, the layer rules.
//!
//! # What this does **not** claim, stated here rather than left to be discovered
//!
//! **The civilians' own books are still not read.** The rows above are the *board's* tickets —
//! the game's governance record — not a citizen's claim; `world::book`'s books are in memory
//! until the civilians are declared and a record is readable from disk, and when that lands this
//! reader gains a third reading rather than being quietly widened. Grading rows by instrument
//! presence would be the defect `census.rs` already names: *"a second vocabulary for 'we did not
//! look' is how a gate comes to read as greener than it is."*

use std::path::Path;

use voxel_model::axis::AxisKind;
use voxel_model::identity::Identity;
use voxel_model::wire::Value;

/// One ladder, and whether the governed record carries its witness.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LadderObservation {
    /// Which ladder this reading is about. **One reading per ladder**, never one reading
    /// summarised onto seven.
    pub ladder: AxisKind,
    /// The file the witness lives in, relative to the workspace root.
    pub witness: &'static str,
    /// The token that must be in that file for the witness to hold.
    pub token: &'static str,
    /// Whether the file was readable at all.
    ///
    /// Separate from `present`: *"the record could not be read"* and *"the record does not carry
    /// this"* are different facts, which is the same distinction `census::CrateObservation`
    /// keeps between `None` and `Some(false)`.
    pub readable: bool,
    /// Whether the token is in the file.
    pub present: bool,
    /// **The second reading: the ticket rows this ladder carries.** One per ticket filing whose
    /// record carries an `answer` — the evidence the ticket stands on.
    ///
    /// Filled for [`AxisKind::TicketEvidence`] and for no other ladder: a citation is a
    /// ticket-evidence fact, and copying it onto seven ladders would be one measurement wearing
    /// seven names. The other six ladders carry an empty list, and that emptiness is a fact
    /// about this reading rather than about the record.
    pub rows: Vec<TicketRow>,
    /// Why the rows are not graded, when the record itself could not be read back.
    ///
    /// **Separate from an empty `rows`**, for the same reason `readable` is separate from
    /// `present`: *"no ticket carries a citation"* and *"the reader could not look"* are
    /// different facts, and only one of them is a finding about the tickets.
    pub rows_unread: Option<String>,
}

/// **One ticket's citation, as the record carries it.**
///
/// The row is what the reader saw, not what it means: the answer verbatim, the document it
/// parses to, the line range, and whether the bytes there still hash to the cited digest. The
/// rule in [`crate::decision::rule_for_civilian`] decides what a non-verifying row *is*.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TicketRow {
    /// The ticket's id, by the board's own rule (`{prefix}-{position:04}`), so a row and a
    /// ticket in the console or the board report are the same ticket by name.
    pub ticket: String,
    /// The record's `answer` field, verbatim. Never reworded.
    pub citation: String,
    /// The document the citation names, when it parses.
    pub document: Option<String>,
    /// The line range the citation names, when it parses.
    pub lines: Option<(usize, usize)>,
    /// Whether the cited bytes still hash to the cited `sha256`.
    ///
    /// `None` when the citation did not parse or the document could not be opened — which is
    /// **not** the same as `Some(false)`, the same distinction [`LadderObservation::readable`]
    /// keeps. `Some(false)` is drift: the document moved.
    pub holds: Option<bool>,
    /// Why the row does not hold, in one line, with the digests that disagree. A row that fails
    /// without a reason is an assertion.
    pub reason: Option<String>,
}

/// **What carries each ladder, witness by witness.**
///
/// Every entry is a **landed** declaration in the governed record, chosen so that no two ladders
/// rest on the same surface — a table where seven rows read one file would be one measurement
/// wearing seven names.
pub const WITNESSES: [(AxisKind, &str, &str); 7] = [
    // Who acted, on every entry — a change the record cannot attribute is not evidence.
    (
        AxisKind::TicketEvidence,
        "crates/voxel-shapers/src/earthworks.rs",
        "pub actor: ActorId",
    ),
    // The claim, declared before the change — the strength of the support a change stands on.
    (
        AxisKind::SupportLevel,
        "crates/voxel-shapers/src/earthworks.rs",
        "pub reservation: Option<ReservationId>",
    ),
    // The gate: a world write cannot happen without the decision layer's permission.
    (
        AxisKind::ProceduralGate,
        "crates/voxel-model/src/world/governed.rs",
        "pub struct WritePermission",
    ),
    // The grade: the commit writes a row a reader can find by reservation or by transition.
    (
        AxisKind::EvidenceGrade,
        "crates/voxel-model/src/world/book.rs",
        "pub fn for_transition",
    ),
    // The unit's grade: the books balance or the change is refused, and the audit says which.
    (
        AxisKind::Grade,
        "crates/voxel-model/src/world/book.rs",
        "pub struct ConservationAudit",
    ),
    // Certification: the whole change as one receipt, never a maximum over its parts.
    (
        AxisKind::CertificationTier,
        "crates/voxel-model/src/world/governed.rs",
        "pub struct TransitionReceipt",
    ),
    // The office that acted. An actor is a persistent identity, not a session.
    (
        AxisKind::OrgRank,
        "crates/voxel-model/src/world/transition.rs",
        "pub struct ActorId",
    ),
];

/// Read the record, once per ladder — and the rows, for the ladder they belong to.
///
/// **Every ladder is reported, always** — a ladder whose file could not be read comes back with
/// `readable: false` rather than being omitted, because an omitted row is an absence a reader
/// has to notice and `refusal.rs`'s rule is that a recorded absence is a *renderable* state.
pub fn observe(root: &Path) -> Vec<LadderObservation> {
    // **The rows are read once, for the one ladder that grades them.** A citation is a fact
    // about a ticket's evidence, so it lands on [`AxisKind::TicketEvidence`] and nowhere else.
    let (rows, rows_unread) = citations(root);
    WITNESSES
        .iter()
        .map(|(ladder, witness, token)| {
            let path = root.join(witness);
            let for_this_ladder = *ladder == AxisKind::TicketEvidence;
            match std::fs::read_to_string(&path) {
                Ok(source) => LadderObservation {
                    ladder: *ladder,
                    witness,
                    token,
                    readable: true,
                    present: source.contains(token),
                    rows: if for_this_ladder { rows.clone() } else { Vec::new() },
                    rows_unread: if for_this_ladder {
                        rows_unread.clone()
                    } else {
                        None
                    },
                },
                Err(_) => LadderObservation {
                    ladder: *ladder,
                    witness,
                    token,
                    readable: false,
                    present: false,
                    rows: if for_this_ladder { rows.clone() } else { Vec::new() },
                    rows_unread: if for_this_ladder {
                        rows_unread.clone()
                    } else {
                        None
                    },
                },
            }
        })
        .collect()
}

/// **The record's ticket rows, and why they could not be read when they could not.**
///
/// The tickets come from the chain the way the board reads them — one `ticket_transition` event
/// is one ticket, its id is `{prefix}-{position:04}`, and its `answer` is the evidence filed
/// against it — except that the answer is kept, because [`voxel_model::ticket::Board`] folds the
/// events and the envelope's descriptive fields (the answer among them) do not cross that fold.
/// The projections are the record's own bytes decoded, so a reader who wants the answer reads it
/// here rather than finding it gone.
fn citations(root: &Path) -> (Vec<TicketRow>, Option<String>) {
    let chain_path = crate::filing::chain_path(root);
    let read = match crate::compiler::read_back(&chain_path) {
        Ok(read) => read,
        Err(refusals) => return (Vec::new(), Some(refusals.to_string())),
    };
    let mut rows = Vec::new();
    for (position, projection) in read.projections.iter().enumerate() {
        let text = |name: &str| projection.get(name).and_then(Value::as_text);
        // Only a ticket filing can carry a ticket's citation; a development fact is not a row.
        if text("event_kind") != Some("ticket_transition") {
            continue;
        }
        // **No answer is not a row here.** A ticket with no evidence is a hole in the ticket,
        // not a citation this reader can grade; the board already names every ticket.
        let Some(answer) = text("answer") else {
            continue;
        };
        let question = text("question").unwrap_or_default();
        let (kind, _) = voxel_model::ticket::split_kind(question);
        rows.push(grade(
            root,
            format!("{}-{:04}", kind.prefix(), position),
            answer.to_string(),
        ));
    }
    (rows, None)
}

/// **A bounded excerpt, for a reason that has to stay one line.** The full text stays on the row
/// (`TicketRow::citation`); a diagnostic that quotes a whole ledger entry is a diagnostic nobody
/// reads, and the report is line-oriented: a reason is one cell of one line.
fn excerpt(text: &str, limit: usize) -> String {
    let characters = text.chars().count();
    if characters <= limit {
        return text.to_string();
    }
    let head: String = text.chars().take(limit).collect();
    format!("{head}… [{} more char(s)]", characters - limit)
}

/// Parse one citation and check it against the document it names.
///
/// The shape is the ledger filer's own: `{document} lines {first}..={last}; sha256:{hex}`. Any
/// other text is a row that **did not parse**, reported with its reason rather than skipped —
/// an answer nobody can follow is exactly the kind of row this ladder exists to call out.
fn grade(root: &Path, ticket: String, citation: String) -> TicketRow {
    // **A row that did not parse is a row, not an omission** — it carries its own citation
    // verbatim and the reason it could not be followed. Once the answer names a document, the
    // row carries that document whether or not the rest checks out: a claimed read-back that
    // cannot be checked is not the same fact as an answer that never claimed one.
    let row = |document: Option<String>,
               lines: Option<(usize, usize)>,
               holds: Option<bool>,
               why: Option<String>| TicketRow {
        ticket: ticket.clone(),
        citation: citation.clone(),
        document,
        lines,
        holds,
        reason: why,
    };
    let Some((document, tail)) = citation.split_once(" lines ") else {
        return row(
            None,
            None,
            None,
            Some(format!(
                "the answer is not a citation this reader understands: it carries no \" lines \" \
                 range — {:?}",
                excerpt(&citation, 160)
            )),
        );
    };
    // **Owned before the moves that follow**: a borrow of the citation cannot outlive the answer
    // it came from, and the document is carried by every failure past this point.
    let document = document.to_string();
    let Some((range, cited)) = tail.split_once("; sha256:") else {
        return row(
            Some(document),
            None,
            None,
            Some(format!(
                "the citation names lines but carries no \"; sha256:\" digest — {:?}",
                excerpt(tail, 160)
            )),
        );
    };
    let Some((first, last)) = range.split_once("..=") else {
        return row(
            Some(document),
            None,
            None,
            Some(format!(
                "the citation's line range {range:?} is not written `first..=last`"
            )),
        );
    };
    let (Ok(first), Ok(last)) = (first.trim().parse::<usize>(), last.trim().parse::<usize>()) else {
        return row(
            Some(document),
            None,
            None,
            Some(format!(
                "the citation's line range {range:?} does not parse as two line numbers"
            )),
        );
    };
    // **The tree's own sha256, parsed rather than string-compared.** `sha256:{hex}` is
    // [`Identity`]'s text form; the comparison is digest to digest, so a citation with the wrong
    // prefix or a truncated body is refused where it should be rather than compared as text.
    let wanted = match Identity::parse(&format!("{}{cited}", Identity::PREFIX)) {
        Ok(identity) => identity.digest(),
        Err(refusal) => {
            return row(
                Some(document),
                Some((first, last)),
                None,
                Some(format!("the citation's digest is not a sha256 digest: {refusal}")),
            );
        }
    };
    let Ok(text) = std::fs::read_to_string(root.join(&document)) else {
        let why = format!(
            "the cited document {document:?} could not be opened at the workspace root"
        );
        return row(Some(document), Some((first, last)), None, Some(why));
    };
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    if first == 0 || last < first || last > lines.len() {
        return row(
            Some(document),
            Some((first, last)),
            None,
            Some(format!(
                "the citation names lines {first}..={last} and the document holds {} line(s)",
                lines.len()
            )),
        );
    }
    let cited_bytes: Vec<u8> = lines[first - 1..last].concat().into_bytes();
    let found = voxel_model::identity::Digest::of(&cited_bytes);
    let why = if found == wanted {
        None
    } else {
        Some(format!(
            "the bytes at {document} lines {first}..={last} hash to sha256:{} and the citation \
             says sha256:{} — the document moved since the ticket was filed",
            found.to_hex(),
            wanted.to_hex()
        ))
    };
    row(Some(document), Some((first, last)), Some(found == wanted), why)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::UnlinkedRecord;

    fn workspace_root(from: &Path) -> Option<std::path::PathBuf> {
        let mut here = from.to_path_buf();
        loop {
            if here.join("Cargo.toml").is_file() && here.join("crates").is_dir() {
                return Some(here);
            }
            if !here.pop() {
                return None;
            }
        }
    }

    fn root() -> std::path::PathBuf {
        workspace_root(&std::env::current_dir().expect("a current directory"))
            .expect("the workspace root")
    }

    #[test]
    fn every_ladder_has_exactly_one_witness_and_they_are_distinct() {
        let mut witness_files: Vec<&str> = WITNESSES.iter().map(|(_, file, _)| *file).collect();
        witness_files.sort();
        witness_files.dedup();
        assert!(
            witness_files.len() > 3,
            "seven rows over {} file(s) would be one measurement wearing seven names",
            witness_files.len()
        );
        // One row per ladder, and every ladder this build knows is covered.
        let mut ladders: Vec<AxisKind> = WITNESSES.iter().map(|(ladder, _, _)| *ladder).collect();
        ladders.sort();
        ladders.dedup();
        assert_eq!(ladders.len(), AxisKind::ALL.len());
        assert_eq!(ladders.len(), WITNESSES.len(), "a ladder with two rows, or none");
    }

    #[test]
    fn the_landed_record_carries_every_witness() {
        // The measured state of this tree, not a hope: each of the seven declarations landed
        // with the governed record, so the arm reports seven present witnesses today.
        let observations = observe(&root());
        assert_eq!(observations.len(), 7);
        for observation in &observations {
            assert!(observation.readable, "{} is unreadable", observation.witness);
            assert!(
                observation.present,
                "{} does not carry {:?} for {}",
                observation.witness,
                observation.token,
                observation.ladder.name()
            );
        }

        // **The second reading, on the real record.** The ledger's filings are the rows: every
        // ticket answer that parses as a citation is checked against the document on disk, and
        // the rule's answer for this ladder is exactly what the gate prints (and the client
        // reads). The counts are printed because the run is the user's to read.
        let evidence = observations
            .iter()
            .find(|observation| observation.ladder == AxisKind::TicketEvidence)
            .expect("the ticket-evidence ladder is reported");
        assert!(
            evidence.rows_unread.is_none(),
            "the real record reads back: {:?}",
            evidence.rows_unread
        );
        assert!(
            !evidence.rows.is_empty(),
            "the ledger filed the rows; a ladder with none read nothing"
        );
        let verifying = evidence
            .rows
            .iter()
            .filter(|row| row.holds == Some(true))
            .count();
        let (state, reason) = crate::decision::rule_for_civilian(evidence);
        println!(
            "the ticket-evidence ladder: {} row(s), {verifying} verifying — {state:?}: {}",
            evidence.rows.len(),
            reason.as_deref().unwrap_or("no reason")
        );
        // **The state and the rows cannot disagree**: the ladder holds while no read-back is
        // broken, and a broken one is refused by name — the gate's callout. Self-report rows
        // (`document: None`) are graded, not failures, so they are not what decides the state.
        let broken = evidence
            .rows
            .iter()
            .filter(|row| row.holds != Some(true) && row.document.is_some())
            .count();
        println!("broken read-back row(s): {broken}");
        if broken == 0 {
            assert_eq!(state, crate::features::Establishment::Active);
        } else {
            assert_eq!(state, crate::features::Establishment::Refused);
        }
    }

    #[test]
    fn an_unreadable_record_is_not_a_record_that_lacks_the_witness() {
        let observations = observe(Path::new("a/root/that/is/not/here"));
        assert_eq!(observations.len(), 7, "every ladder is still reported");
        for observation in &observations {
            assert!(!observation.readable);
            assert!(!observation.present);
        }
        // **And the rows are not silently empty.** An unreadable chain is `rows_unread` with a
        // reason, which is a different fact from a record that carries no ticket.
        let ladder = |kind: AxisKind| {
            observations
                .iter()
                .find(|observation| observation.ladder == kind)
                .expect("every ladder is reported")
        };
        let evidence = ladder(AxisKind::TicketEvidence);
        assert!(evidence.rows.is_empty());
        assert!(
            evidence.rows_unread.is_some(),
            "a chain that cannot be read must not read as a record with no tickets"
        );
        // The other six ladders carry no rows at all — the reading is the ticket ladder's.
        for observation in &observations {
            if observation.ladder != AxisKind::TicketEvidence {
                assert!(observation.rows.is_empty());
                assert!(observation.rows_unread.is_none());
            }
        }
    }

    // ---- the second reading --------------------------------------------------

    /// A workspace this test may write, inside the project (SES-05).
    fn scratch(tag: &str) -> std::path::PathBuf {
        let workspace = crate::features::workspace_root(std::path::Path::new(env!(
            "CARGO_MANIFEST_DIR"
        )))
        .expect("this crate sits inside the workspace");
        let path = workspace
            .join("logs")
            .join("test-workspaces")
            .join(format!("civilians-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        path
    }

    /// One citation naming lines of a document; the digest is of exactly those lines' bytes.
    fn citation(document: &str, text: &str, first: usize, last: usize) -> String {
        let lines: Vec<&str> = text.split_inclusive('\n').collect();
        let bytes: Vec<u8> = lines[first - 1..last].concat().into_bytes();
        format!(
            "{document} lines {first}..={last}; sha256:{}",
            voxel_model::identity::Digest::of(&bytes).to_hex()
        )
    }

    /// The document the fixture cites.
    const DOCUMENT: &str = "docs/LEDGER.md";
    /// The fixture's document: two entries, so a citation of `1..=4` is the first entry's bytes.
    const DOCUMENT_TEXT: &str = "## C-1 — a case\n\nanswer text\n\n## C-2 — another\n\nmore\n";

    /// File a scratch chain holding one development fact and the given tickets.
    fn filed_workspace(
        tag: &str,
        tickets: [(&'static str, Option<String>); 3],
    ) -> std::path::PathBuf {
        use voxel_model::chain::{EventKind, Namespace, Season};
        use voxel_model::envelope::{Control, EventType, Kind, RecordState};

        let workspace = scratch(tag);
        std::fs::create_dir_all(workspace.join("docs")).expect("the document's directory");
        std::fs::write(workspace.join(DOCUMENT), DOCUMENT_TEXT).expect("the cited document");

        let namespace = Namespace::new("world-2468", 1, Season::Winter, "c-2026-10-02")
            .expect("a namespace");
        let mut records: Vec<UnlinkedRecord<'_>> = Vec::new();
        for (question, answer) in &tickets {
            records.push(UnlinkedRecord {
                namespace: &namespace,
                event_kind: EventKind::TicketTransition,
                question,
                answer: answer.as_deref(),
                kind: Kind::Authority,
                event_type: EventType::UserDecision,
                state: RecordState::Answered,
                control: Control::Continue,
                actor: "agent",
                round_id: "c-2026-10-02",
                supersedes: None,
            });
        }

        let first = crate::filing::linked_records(
            &workspace,
            &[],
            [UnlinkedRecord {
                namespace: &namespace,
                event_kind: EventKind::DevelopmentFact,
                question: "the workspace opened",
                answer: None,
                kind: Kind::Evidence,
                event_type: EventType::UserDecision,
                state: RecordState::Answered,
                control: Control::Continue,
                actor: "agent",
                round_id: "c-2026-10-02",
                supersedes: None,
            }],
        )
        .expect("the first record links");
        crate::filing::append(&workspace, &first).expect("the chain is created");
        let before = std::fs::read(crate::filing::chain_path(&workspace)).expect("the chain reads");
        crate::filing::grow(&workspace, &before, records).expect("the tickets are filed");
        workspace
    }

    #[test]
    fn the_tickets_rows_verify_against_the_documents_they_cite() {
        // One whose citation verifies, one whose document moved (`e3b0c442…` is sha256 of the
        // empty input, so it cannot match these bytes), and one whose answer is not a citation.
        let workspace = filed_workspace(
            "rows",
            [
                ("[CSE] C-1 — a case", Some(citation(DOCUMENT, DOCUMENT_TEXT, 1, 4))),
                (
                    "[CSE] C-2 — another",
                    Some(
                        "docs/LEDGER.md lines 1..=4; \
                         sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                            .to_string(),
                    ),
                ),
                ("[CSE] C-3 — a third", Some("saw it with my own eyes".to_string())),
            ],
        );
        let observations = observe(&workspace);
        let evidence = observations
            .iter()
            .find(|observation| observation.ladder == AxisKind::TicketEvidence)
            .expect("the ladder is reported");
        assert_eq!(
            evidence.rows.len(),
            3,
            "one row per ticket that carries an answer: {:?}",
            evidence.rows
        );
        assert_eq!(evidence.rows[0].ticket, "CSE-0001", "the board's own id rule");
        assert_eq!(evidence.rows[1].ticket, "CSE-0002");
        assert_eq!(evidence.rows[2].ticket, "CSE-0003");
        // **Verified**: the document's bytes at the cited lines still hash to the citation.
        assert_eq!(evidence.rows[0].holds, Some(true), "{:?}", evidence.rows[0]);
        assert_eq!(evidence.rows[0].lines, Some((1, 4)));
        assert_eq!(evidence.rows[0].document.as_deref(), Some("docs/LEDGER.md"));
        // **Moved**: the digest names bytes that are not there — the document moved since filing.
        assert_eq!(evidence.rows[1].holds, Some(false));
        assert!(
            evidence.rows[1]
                .reason
                .as_deref()
                .is_some_and(|reason| reason.contains("the document moved")),
            "a non-verifying row carries the reason it does not hold: {:?}",
            evidence.rows[1]
        );
        // **Unparsed**: the answer is not a citation, and that is a row with a reason, not an
        // omission.
        assert_eq!(evidence.rows[2].holds, None);
        assert!(
            evidence.rows[2]
                .reason
                .as_deref()
                .is_some_and(|reason| reason.contains("not a citation")),
            "{:?}",
            evidence.rows[2]
        );
        assert!(evidence.rows_unread.is_none(), "the record read fine");

        // The rule, not the reader, decides what that means: **one broken row** — the moved
        // citation — and one prose answer, which is `self-report (C)` on the ladder rather than a
        // failure: the rung exists for evidence exactly like it.
        let (state, reason) = crate::decision::rule_for_civilian(evidence);
        assert_eq!(state, crate::features::Establishment::Refused);
        let reason = reason.expect("a refusal carries its reason");
        assert!(reason.contains("1 of 3 ticket row(s) are broken"), "{reason}");
        assert!(reason.contains("CSE-0002"), "the moved row is named: {reason}");
        assert!(
            !reason.contains("CSE-0003"),
            "a self-report row is graded, not called broken: {reason}"
        );
        let _ = std::fs::remove_dir_all(&workspace);
    }

    #[test]
    fn a_record_whose_rows_all_hold_says_so_in_its_reason() {
        // Every citation verifying: the rule holds the ladder **and says what it graded** — a
        // catch carried as a reason on a holding subject, printed by `Pass::report`.
        let good = citation(DOCUMENT, DOCUMENT_TEXT, 1, 4);
        let workspace = filed_workspace(
            "rows-hold",
            [
                ("[CSE] C-1 — a case", Some(good.clone())),
                ("[CSE] C-2 — another", Some(good.clone())),
                ("[CSE] C-3 — a third", Some(good)),
            ],
        );
        let observations = observe(&workspace);
        let evidence = observations
            .iter()
            .find(|observation| observation.ladder == AxisKind::TicketEvidence)
            .expect("the ladder is reported");
        assert_eq!(evidence.rows.len(), 3);
        assert!(evidence.rows.iter().all(|row| row.holds == Some(true)));
        let (state, reason) = crate::decision::rule_for_civilian(evidence);
        assert_eq!(state, crate::features::Establishment::Active);
        let reason = reason.expect("a holding subject may still owe the reader a fact");
        assert!(reason.contains("3 ticket row(s) on this ladder"), "{reason}");
        assert!(reason.contains("3 read back"), "{reason}");
        assert!(reason.contains("ticket-evidence B"), "{reason}");
        assert!(reason.contains("0 self-report"), "{reason}");
        assert!(reason.contains("none broken"), "{reason}");
        let _ = std::fs::remove_dir_all(&workspace);
    }
}
