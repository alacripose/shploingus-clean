//! The notification queue — the client side of the api layer.
//!
//! The shell's own module map names this file as `notification_queue`'s witness.
//! What it holds: the decision records the laya gate produces, one per judged
//! artifact, in the order the gate made them. UI-SPEC §7 rules the surface:
//! *refusals, approvals and anything needing a human are persistent — they live
//! in the notifications window and in the ledger, not in a toast that scrolls
//! away*, and **both read the same records**, so a toast cannot say more than
//! the record it came from. H-12.2: the shell owns the queue — one writer, one
//! rendering per record.
//!
//! **The record format is not defined here.** [`voxel_model::laya`] writes and
//! decodes it; this module is the window's view onto the same records (D-07: one
//! definition on one seam). The queue is **a directory the client watches** —
//! the change channel's own shape (IMG_3365) — and every record is streamed,
//! decoded whole, and checked against the artifact's own bytes before it is
//! shown: `sha256(artifact) == expected`.
//!
//! # The writer is here too, and it is the game that runs it
//!
//! **`logs/laya-decisions/` used to be test residue.** The only callers of
//! `Evaluation::write_queue` were `#[cfg(test)]` code — the judge's own test and the stream
//! test below — so production only ever *read* the queue, and the record's arrows
//! `game -> decision` and `decision -> queue` had no production writer at all. They do now:
//! [`write_the_gate_queue`] is this run's judgement, asked for by the game when the world
//! opens, and written at the world's opening tick; the stream then hands the window **one
//! record per tick** — H-12.2's *one writer, one rendering per record*, at the rate the world
//! runs at rather than the rate the terminal can print.
//!
//! **It judges; it does not invent.** The instrument is `laya::evaluate_project` — full
//! powers, every `.rs` under `crates/`, source and test alike, default fail — and the record
//! shape is the declared `laya::Notification` on the declared codec. A pass beside the judge
//! would be the auditor `forbidden.detached-judge` names, so there is not one.
//!
//! **And the proof slice is the citizens' own rows, not an empty one.** [`mint_the_gate_proofs`]
//! drains the observer's committed acts — `voxel-model`'s own declarations are the contract:
//! the rows are *"the raw material the gate's proof of work is minted from"*
//! (`observer/mod.rs`), and *"the game side drains it to mint the gate's proofs of work (the
//! user's plan, 2026-10-02)"* (`lib.rs`). The grading is `axis.rs`'s own: a row's origin is
//! `ReadBack`, the ticket-evidence ladder's `B` rung, and no grade is invented beside the
//! ladders. An artifact no accepted act names still fails by default — the mint adds proofs,
//! it never lowers the bar.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use voxel_model::identity::Digest;
use voxel_model::laya::{queue_dir, Notification, ProofOfWork};
use voxel_model::observer::recorder::drain_committed_actions;

/// A streamed read of the queue the gate wrote: **one record at a time**, in
/// decision order — the chunk streamer's shape applied to the queue. The whole
/// queue is never held in memory; it grows with every decision and the window
/// scrolls through it.
pub struct NotificationStream {
    records: std::vec::IntoIter<PathBuf>,
}

impl NotificationStream {
    /// The queue the gate wrote under the workspace root. The directory is
    /// listed once and sorted — deterministic order is decision order.
    pub fn open(workspace: &Path) -> Result<Self, String> {
        Self::open_at(&queue_dir(workspace))
    }

    /// **Open a queue by its own path** — the same stream, on the other queue.
    ///
    /// `open` derives the judgement's queue from the workspace root; this takes the directory, so
    /// **the agent's inbox is read by the same streaming system the window reads** (DSL:
    /// `queue -> game` and `queue -> agent` are two readers of one notification system, not two
    /// systems). The order is the same file-name order, the decode is the same musli codec, and
    /// [`Self::next_verified`] re-hashes the artifact the same way — because a recipe the agent is
    /// handed must be the bytes the record was written against or nothing.
    pub fn open_at(dir: &Path) -> Result<Self, String> {
        let entries = std::fs::read_dir(dir)
            .map_err(|failure| format!("{} cannot be read: {failure}", dir.display()))?;
        let mut records: Vec<PathBuf> = Vec::new();
        for entry in entries {
            let path = entry
                .map_err(|failure| format!("{} cannot be listed: {failure}", dir.display()))?
                .path();
            if path.extension().map(|ext| ext == "musli").unwrap_or(false) {
                records.push(path);
            }
        }
        records.sort();
        Ok(NotificationStream {
            records: records.into_iter(),
        })
    }

    /// The next record — **decoded whole and verified** before it is shown:
    /// `sha256(artifact) == expected`, the change channel's rule. A record that
    /// does not decode, or whose artifact no longer hashes to the sha256 it was
    /// judged under, is **refused by name** — never skipped, never shown against
    /// bytes the decision was not made from.
    pub fn next_verified(&mut self, root: &Path) -> Result<Option<Notification>, String> {
        let Some(path) = self.records.next() else {
            return Ok(None);
        };
        let bytes = std::fs::read(&path)
            .map_err(|failure| format!("{} cannot be read: {failure}", path.display()))?;
        let notification = Notification::decode(&bytes)
            .map_err(|failure| format!("{} does not decode: {failure}", path.display()))?;
        let artifact = root.join(&notification.decision.path);
        let file = std::fs::File::open(&artifact)
            .map_err(|failure| format!("{} cannot be opened: {failure}", artifact.display()))?;
        let sha256 = Digest::of_reader(std::io::BufReader::new(file))
            .map_err(|failure| format!("{} cannot be read: {failure}", artifact.display()))?;
        if sha256 != notification.artifact_sha256 {
            return Err(format!(
                "{}: sha256 of {} does not match the record; the decision is refused rather than \
                 shown against bytes it was not made from",
                path.display(),
                notification.decision.path
            ));
        }
        Ok(Some(notification))
    }
}

/// **The proofs of work the citizens mint: the observer's committed rows, one per artifact.**
///
/// `voxel-model`'s own declarations are the whole contract for this function — `lib.rs`:
/// *"until the game side drains it to mint the gate's proofs of work (the user's plan,
/// 2026-10-02)"*; `observer/mod.rs`: the rows are *"the raw material the gate's proof of work
/// is minted from"*; `recorder.rs`: *"The game side drains the rows it has not yet taken"*.
/// This is that minting, and the game's pass is the game side.
///
/// **One row becomes one [`ProofOfWork`] for the artifact whose code committed the act.**
/// `decide` finds a proof by `proof.path == path` and makes no other lookup, so the path is the
/// row's own commit site — read back from the commit point with `#[track_caller]` — with the
/// line dropped and the separators normalized exactly as `evaluate_project` normalizes the
/// artifacts it judges.
///
/// **The grading is `axis.rs`'s and nothing else.** The origin is
/// [`EvidenceOrigin::ReadBack`] — the `B` rung of the ticket-evidence ladder `axis.rs`
/// registers by bridge (`impl Ladder for EvidenceOrigin`) — because the machine looked at its
/// own book and the book answered; `decide` grades it as `(AxisKind::TicketEvidence, B)` on the
/// ladder's own token. No grade, score or support level is invented here.
///
/// **A row is taken once.** The drain is the recorder's own rule: a row handed out never comes
/// back, so one act cannot be counted twice by two passes of the gate.
pub fn mint_the_gate_proofs() -> Vec<ProofOfWork> {
    voxel_model::laya::mint_proofs_from_rows(drain_committed_actions())
}



/// **The judgement this run asks for, written by the game itself.**
///
/// The queue's only writers used to be tests, so `logs/laya-decisions/` was residue from them
/// and the record's `game -> decision` and `decision -> queue` arrows were drawn with nothing
/// behind them in production. This is the writer: the game asks for the run's judgement, the
/// judge (laya) evaluates the whole project, and the queue is written **one notification per
/// decision, never fewer**, through the declared codec.
///
/// **Why the whole project, and why that is not a choice made here.** `evaluate_project` is
/// the judge's own entry point and its scope is the judge's own ruling: the entire project,
/// nothing scoped out, every artifact failing by default until it carries proof of work. A
/// caller that narrowed the subject set would be a second, quieter gate.
///
/// **The proof slice is the citizens' own rows.** [`mint_the_gate_proofs`] drains the
/// observer's committed acts — the machine's single commit point records one row per accepted
/// governed act, and the game side is the side that drains them — so an artifact whose code
/// committed a governed act is judged against that act's receipt, graded on `axis.rs`'s
/// ticket-evidence ladder. An artifact no accepted act names still fails by default: the mint
/// adds proofs, it never lowers the bar.
///
/// **A refusal is returned, never swallowed.** A queue that cannot be written is reported to
/// the caller — the run that asked for it — and the caller says so rather than carrying on
/// with an empty window that would read as a clean pass.
///
/// # The queue is the run's, and the previous run's records are not
///
/// **Record names are run-local** (`laya-0000`, `laya-0001`, …), so two runs write the same
/// names and a record left over from the last run is a record **this run did not make**. The
/// stream's sha256 check does not catch it, and that is the trap: a stale record whose artifact
/// still hashes to what the older decision was made from **verifies**, so the window would show
/// a previous run's decision as this run's. So the writer clears the queue's `.musli` records
/// before it writes this run's — a leftover is not a decision, and a directory is not a record
/// (`forbidden.second-record`: the chain is the record, and this queue is a projection of one
/// pass over it). A record that cannot be removed is refused by name rather than written over,
/// because a write over a file that is still there is exactly how the stale record survives.
pub fn write_the_gate_queue(workspace: &Path) -> Result<voxel_model::laya::Evaluation, String> {
    let proofs = mint_the_gate_proofs();
    let evaluation = voxel_model::laya::evaluate_project(workspace, &proofs);
    let queue = queue_dir(workspace);
    clear_the_previous_run(&queue)?;
    evaluation
        .write_queue(workspace, &queue)
        .map_err(|refusal| format!("the queue at {} was refused: {refusal}", queue.display()))?;
    Ok(evaluation)
}

/// Remove the records a previous run left in the queue — **by name, never silently**, and never
/// by following a link: a queue entry that is not a file whose extension is `musli` is left
/// alone, because a directory of unknown shapes is not this writer's to empty.
///
/// A missing queue directory is not a failure: the write below creates it, and a first run has
/// no previous records to clear. That is a measurement, not an absence handled by luck.
fn clear_the_previous_run(queue: &Path) -> Result<(), String> {
    let Ok(entries) = std::fs::read_dir(queue) else {
        return Ok(());
    };
    for entry in entries {
        let path = entry
            .map_err(|failure| format!("{} cannot be listed: {failure}", queue.display()))?
            .path();
        if path.extension().map(|ext| ext == "musli").unwrap_or(false) {
            std::fs::remove_file(&path).map_err(|failure| {
                format!(
                    "{} is a record from an earlier run and cannot be removed: {failure}. It is \
                     refused rather than overwritten, because a stale record that is written over \
                     is the one way a previous run's decision can pass as this run's",
                    path.display()
                )
            })?;
        }
    }
    Ok(())
}

/// How many ticks a toast is on screen: the declared duration in the world's own ticks.
///
/// **Derived, not restated.** `TOAST_MS` is the design system's value and `TICK_HZ` is the
/// world's; this is the one place they meet, so a duration and a tick rate cannot drift into a
/// toast that lasts a different number of seconds than the record says. The test beside this
/// pins 6 s at 20 Hz = 120, the lineage's own count.
pub fn toast_ticks() -> u64 {
    let hz = crate::clock::TICK_HZ as f64;
    let ticks = (voxel_ui::toast_ms() / 1000.0 * hz).round();
    debug_assert!(
        ticks >= 1.0,
        "a toast shorter than one tick could never be rendered; the declared duration is {ticks} \
         ticks at {hz} Hz"
    );
    ticks.max(1.0) as u64
}

/// How many ticks the toast's entrance ramp lasts: the declared entry duration in the world's
/// ticks — four at 20 Hz. The tick rate is the resolution a ramp has here, which is one honest
/// reason the entrance is an opacity ramp and not a movement: four steps of a slide would read as
/// a stutter, and a motion system with a curve class is still owed (§8.2's open row).
pub fn toast_entry_ticks() -> u64 {
    let hz = crate::clock::TICK_HZ as f64;
    ((voxel_ui::toast_entry_ms() / 1000.0 * hz).round()).max(1.0) as u64
}

/// One toast **as it is rendered**: the line it says and the tick it went on screen.
///
/// **`shown_at`, not `raised_at`.** The record's own time is the console record's (the same words
/// are logged beside this queue); a toast's life is its *screen* life, so a queued message waits
/// its turn and then gets its whole duration rather than inheriting the tick it was enqueued at —
/// which for a burst would otherwise expire the later records before they were ever drawn.
#[derive(Clone, PartialEq, Debug)]
pub struct Toast {
    pub text: String,
    pub shown_at: u64,
}

/// **The shell's toast queue** (H-12.2), and the queue's four properties are the ruling's own:
/// **one writer** ([`Self::enqueue`] is called by the shell, and `Mount::announce` is the one
/// place that calls it), **one rendering per record** (a record is promoted once and drawn until
/// its own time is up), **one at a time** (the queue drains through [`Self::current`]), and the
/// same API that produces a panel produces a toast (C-54.3) — `announce` is that API.
///
/// **Why this lives here and not in the mount.** The oracle's known answers name this file for
/// `enqueue(toast);` — *"the shell owns a queue and a toast is a record in it"* — and refuse
/// `widget.raise(from_call_site)`: a queue kept beside the window manager would be the call-site
/// shape under a different name.
///
/// **Nothing here reads a clock.** [`Self::advance`] takes the tick, so a toast's lifetime is a
/// function of the same clock every record is stamped with, and advancing twice in one tick
/// changes nothing (idempotent), which is what lets a caller advance once per frame while the
/// queue still moves at the world's rate.
#[derive(Default)]
pub struct ToastQueue {
    current: Option<Toast>,
    pending: VecDeque<String>,
    raised: u64,
}

impl ToastQueue {
    /// Enqueue one record. **The shell is the only caller** — one writer, so ordering is a
    /// property of the shell and not of who called first. The queue is unbounded on purpose:
    /// every record here is also on the console, and the toast is the transient view of a message
    /// the record already holds, so a burst delays the queue rather than losing it.
    pub fn enqueue(&mut self, text: impl Into<String>) {
        self.pending.push_back(text.into());
    }

    /// Advance to `tick`: release a finished record, promote the next one, and give it its own
    /// six seconds from this tick. Idempotent within a tick.
    pub fn advance(&mut self, tick: u64) {
        let expired = self
            .current
            .as_ref()
            .is_some_and(|toast| tick.saturating_sub(toast.shown_at) >= toast_ticks());
        if expired {
            self.current = None;
        }
        if self.current.is_none() {
            if let Some(text) = self.pending.pop_front() {
                self.current = Some(Toast { text, shown_at: tick });
                self.raised += 1;
            }
        }
    }

    /// The record on screen, if one is.
    pub fn current(&self) -> Option<&Toast> {
        self.current.as_ref()
    }

    /// How many records have been rendered. One per promotion, so two readings of one record
    /// would show up here as a count that moved twice for one message.
    pub fn raised(&self) -> u64 {
        self.raised
    }

    /// The current toast's remaining life: `(ticks left, fraction of the whole)`. `None` when
    /// nothing is showing — absence rendered as absence, not as a zero-length bar.
    pub fn remaining(&self, tick: u64) -> Option<(u64, f64)> {
        let toast = self.current.as_ref()?;
        let life = toast_ticks();
        let left = life.saturating_sub(tick.saturating_sub(toast.shown_at));
        Some((left, left as f64 / life as f64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_model::laya::{commit_site_artifact, evaluate_project};
    use voxel_model::ticket::EvidenceOrigin;

    fn root() -> PathBuf {
        voxel_package::features::workspace_root(Path::new(env!("CARGO_MANIFEST_DIR")))
            .expect("this crate sits inside the workspace")
    }

    /// The observer's book is process-global (`recorder.rs`), and every test in this module runs
    /// in one process. A mint and a record-then-mint sequence must not interleave with another
    /// test's drain, or a row would be taken by the wrong test.
    fn the_observer_book() -> std::sync::MutexGuard<'static, ()> {
        static BOOK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        BOOK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[test]
    fn the_queue_streams_every_decision_the_gate_made() {
        let root = root();
        let evaluation = evaluate_project(&root, &[]);
        assert!(
            !evaluation.decisions.is_empty(),
            "the workspace has artifacts to judge; an empty judgement would pass the count below \
             without streaming anything"
        );
        // **A scratch queue, not the shared one.** Record names are run-local (`laya-0000`, …)
        // and `write_queue` does not clear, so records a previous run left in
        // `logs/laya-decisions/` are decisions this run did not make — and their artifacts still
        // hash, so the stream would yield them and this test would index past its own decisions.
        // That is the trap `write_the_gate_queue` clears for; the production queue is the game's
        // to write, and this reads the same stream through `open_at` on a queue of its own.
        let scratch = root
            .join("logs")
            .join("test-workspaces")
            .join(format!("notifications-stream-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&scratch);
        let queue = scratch.join("queue");
        evaluation
            .write_queue(&root, &queue)
            .expect("the gate writes its queue");
        let mut stream = NotificationStream::open_at(&queue).expect("the queue is there");
        let mut seen = 0usize;
        while let Some(notification) = stream
            .next_verified(&root)
            .expect("every record decodes and verifies")
        {
            assert_eq!(
                notification.decision, evaluation.decisions[seen],
                "the decision reached the window whole"
            );
            seen += 1;
        }
        assert_eq!(
            seen,
            evaluation.decisions.len(),
            "every decision reaches the window — one notification per decision"
        );
        let _ = std::fs::remove_dir_all(&scratch);
    }

    #[test]
    fn a_tampered_artifact_is_refused_before_it_is_shown() {
        let root = root();
        let scratch = root
            .join("logs")
            .join("test-workspaces")
            .join(format!("notifications-tampered-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&scratch);
        let queue = scratch.join("queue");
        std::fs::create_dir_all(&queue).expect("the scratch queue exists");

        // One record, judged over one artifact — and then the artifact changes.
        let artifact_rel = "tampered.rs";
        std::fs::write(scratch.join(artifact_rel), b"pub fn judged() {}").expect("the artifact");
        let decision = voxel_model::laya::Decision {
            path: artifact_rel.to_string(),
            question: voxel_model::laya::Question::Choice,
            verdict: voxel_model::laya::Verdict::NotBelongs,
            confidence: 0.0,
            origin: None,
            grades: Vec::new(),
            findings: vec!["no proof of work on record".to_string()],
        };
        let notification = Notification::of(0, &decision, &scratch).expect("the record builds");
        std::fs::write(
            queue.join(notification.file_name()),
            notification.encode().expect("the record encodes"),
        )
        .expect("the record lands");

        let mut stream = NotificationStream { records: vec![queue.join("laya-0000.musli")].into_iter() };
        std::fs::write(scratch.join(artifact_rel), b"pub fn judged() { /* changed */ }")
            .expect("the artifact changes");
        let refused = stream.next_verified(&scratch).expect_err("the tamper is caught");
        assert!(
            refused.contains("does not match"),
            "the refusal names the sha256 mismatch: {refused}"
        );
    }

    /// **The agent's read half, exercised as the agent uses it.**
    ///
    /// DSL: `queue -> agent`. The game notifies the judge of the package it emitted, the judge
    /// writes the record into `logs/agent-inbox/`, and this opens that queue with the **same**
    /// stream the notifications window uses — so the agent's inbox is the notification system
    /// rather than a second one beside it.
    ///
    /// Teeth: the record must verify against the package's own bytes, and a package whose bytes
    /// changed after the notification must be **refused**, not shown. A recipe implemented against
    /// bytes nobody judged is the failure this binding exists to make impossible.
    #[test]
    fn the_agent_inbox_streams_the_recipe_laya_wrote_for_the_package() {
        let root = root();
        let scratch = root
            .join("logs")
            .join("test-workspaces")
            .join(format!("agent-inbox-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&scratch);
        let packages = scratch.join("packages");
        std::fs::create_dir_all(&packages).expect("the scratch packages directory");
        let package = packages.join("from-ticket.shpkg");
        std::fs::write(&package, b"the bytes the recipe was compiled into")
            .expect("the package lands");
        let digest = Digest::of_reader(std::io::BufReader::new(
            std::fs::File::open(&package).expect("the package opens"),
        ))
        .expect("the artifact hashes");

        let record = voxel_model::laya::notify_the_agent_of(
            &scratch,
            "BLD-0207",
            "packages/from-ticket.shpkg",
            &digest,
            true,
        )
        .expect("the judge writes the recipe into the agent's inbox");
        assert_eq!(
            record, "recipe-BLD-0207.musli",
            "the record is named for the ticket, not for its position: a positional name would be \
             overwritten by the next run's first filing"
        );

        let inbox = voxel_model::laya::agent_inbox_dir(&scratch);
        let mut stream = NotificationStream::open_at(&inbox).expect("the inbox is there");
        let notification = stream
            .next_verified(&scratch)
            .expect("the record verifies")
            .expect("one record was written");
        assert_eq!(
            notification.decision.path, "packages/from-ticket.shpkg",
            "the artifact is named relative to the root, the way the record spells a path"
        );
        assert_eq!(
            notification.decision.verdict,
            voxel_model::laya::Verdict::Belongs,
            "the compile's own round trip cleared it, so the judge's verdict is Belongs"
        );
        assert!(
            notification.decision.findings.is_empty(),
            "and a cleared verdict carries no finding: {:?}",
            notification.decision.findings
        );
        assert_eq!(notification.artifact_sha256, digest);

        // **A package that changed after the notification is refused by the same stream.**
        std::fs::write(&package, b"the bytes changed afterwards").expect("the package changes");
        let mut again = NotificationStream::open_at(&inbox).expect("the inbox is there");
        let refused = again
            .next_verified(&scratch)
            .expect_err("the tamper is caught before the recipe is shown");
        assert!(
            refused.contains("does not match"),
            "the refusal names the sha256 mismatch: {refused}"
        );
        let _ = std::fs::remove_dir_all(&scratch);
    }

    /// **The production writer, exercised as the game uses it.**
    ///
    /// The queue was written only from `#[cfg(test)]` code before this, so the claim here is
    /// about the game's own entry point: one notification per decision, through the declared
    /// codec, verified against the artifact's own bytes as the window streams it.
    ///
    /// **The scratch workspace is a real one** — a `crates/` tree the judge walks, with one
    /// artifact in it — because a writer tested against a mocked walk would hold nothing about
    /// the walk. Teeth: a writer that wrote the decisions without their sha256, or that wrote
    /// fewer records than decisions, fails here rather than in the shipped client.
    #[test]
    fn the_games_own_pass_writes_one_notification_per_decision() {
        let _book = the_observer_book();
        let root = root();
        let scratch = root
            .join("logs")
            .join("test-workspaces")
            .join(format!("laya-writer-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&scratch);
        let crate_src = scratch.join("crates").join("demo").join("src");
        std::fs::create_dir_all(&crate_src).expect("the scratch crate exists");
        std::fs::write(crate_src.join("lib.rs"), b"pub fn judged() {}\n").expect("the artifact");

        let evaluation = write_the_gate_queue(&scratch).expect("the game's pass writes the queue");
        assert_eq!(evaluation.decisions.len(), 1, "one artifact, one decision");
        // **A second run's queue is the second run's.** The first pass's record is cleared, so a
        // stale record cannot verify its way onto the window as a decision this run made.
        let second = write_the_gate_queue(&scratch).expect("a second pass writes the queue");
        assert_eq!(second.decisions.len(), 1);
        let records: Vec<_> = std::fs::read_dir(queue_dir(&scratch))
            .expect("the queue reads")
            .flatten()
            .map(|entry| entry.path())
            .collect();
        assert_eq!(records.len(), 1, "the previous run's record survived the new pass");
        assert!(
            !evaluation.decisions[0].cleared(),
            "an artifact with no proof of work cleared the gate"
        );

        let mut stream = NotificationStream::open(&scratch).expect("the queue is there");
        let mut seen = 0usize;
        while let Some(notification) =
            stream.next_verified(&scratch).expect("every record verifies")
        {
            assert_eq!(
                notification.decision, evaluation.decisions[seen],
                "the decision reached the window whole"
            );
            seen += 1;
        }
        assert_eq!(seen, evaluation.decisions.len(), "one notification per decision");
        let _ = std::fs::remove_dir_all(&scratch);
    }

    /// **The citizens' proof of work is minted from their committed acts — the wiring the run
    /// was missing.**
    ///
    /// `voxel-model`'s own chain is the contract: `lib.rs` — *"until the game side drains it to
    /// mint the gate's proofs of work (the user's plan, 2026-10-02)"*; `observer/mod.rs` — the
    /// rows are *"the raw material the gate's proof of work is minted from"*; `recorder.rs` —
    /// *"The game side drains the rows it has not yet taken"*.
    ///
    /// Teeth: one accepted act becomes exactly one proof, on the artifact whose code committed
    /// it (the caller read back with `Location::caller()`, the line dropped), carrying the row's
    /// claim anchor and the row's own account, graded `ReadBack` — the `B` rung of `axis.rs`'s
    /// ticket-evidence ladder — and the row is not offered twice.
    #[test]
    fn a_committed_act_is_minted_into_one_proof_of_work_on_the_artifact_that_committed_it() {
        let _book = the_observer_book();
        let _ = drain_committed_actions();
        let row = voxel_model::observer::recorder::record_committed_action(
            "dig",
            "civ-1",
            "task-9",
            Some("res-4".to_owned()),
            &["rock 4g substrate -> spoil:civ-1".to_owned()],
            &["transition.committed: the books balanced".to_owned()],
            "transition-digest".to_owned(),
        );
        let proofs = mint_the_gate_proofs();
        assert_eq!(proofs.len(), 1, "one committed act, one proof of work");
        let proof = &proofs[0];
        // `record_committed_action` is `#[track_caller]`: this test is the caller, so the proof
        // lands on this test's own file — the same read the machine's commit point gets.
        assert!(
            proof.path.ends_with("shell/notifications.rs"),
            "the proof names the caller's artifact, not the recorder: {}",
            proof.path
        );
        assert!(!proof.path.contains(':'), "the line is dropped: {}", proof.path);
        assert_eq!(proof.path, commit_site_artifact(&row.site));
        assert_eq!(proof.origin, EvidenceOrigin::ReadBack, "B — the machine read its own book");
        assert_eq!(proof.origin.letter(), "B", "axis.rs's ticket-evidence ladder token");
        assert!(proof.claim.contains("res-4"), "the claim anchor: {}", proof.claim);
        assert_eq!(proof.evidence, row.message(), "the proof quotes the row's own account");
        assert_eq!(
            voxel_model::observer::recorder::pending_committed_actions(),
            0,
            "a row handed out once is never offered again"
        );
    }

    /// **A minted proof reaches the gate on its artifact, and the grading it carries is
    /// `axis.rs`'s.**
    ///
    /// A scratch workspace holds one artifact at exactly the proof's own path, so `decide`'s one
    /// lookup (`proof.path == path`) finds it, and the decision is on the record with the proof's
    /// origin — the `B` rung of the ticket-evidence ladder — instead of the default "no proof of
    /// work" finding. The judge still judges: a proof opens the gate, it does not pass the
    /// artifact.
    #[test]
    fn a_minted_proof_reaches_the_gate_on_its_artifact_with_axis_rs_grading() {
        let _book = the_observer_book();
        let _ = drain_committed_actions();
        let row = voxel_model::observer::recorder::record_committed_action(
            "place",
            "civ-2",
            "-",
            None,
            &[],
            &[],
            "t2".to_owned(),
        );
        let scratch = root()
            .join("logs")
            .join("test-workspaces")
            .join(format!("laya-proofs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&scratch);
        let artifact_rel = commit_site_artifact(&row.site);
        let artifact = scratch.join(&artifact_rel);
        std::fs::create_dir_all(artifact.parent().expect("an artifact has a parent"))
            .expect("the scratch crates tree exists");
        std::fs::write(&artifact, b"pub fn judged() {}\n").expect("the artifact lands");

        let proofs = mint_the_gate_proofs();
        assert_eq!(proofs.len(), 1, "the act was minted before the gate read the book");
        let evaluation = evaluate_project(&scratch, &proofs);
        assert_eq!(evaluation.decisions.len(), 1, "one artifact, one decision");
        let decision = &evaluation.decisions[0];
        assert_eq!(decision.path, artifact_rel, "the proof is looked up by the artifact's path");
        assert_eq!(
            decision.origin,
            Some(EvidenceOrigin::ReadBack),
            "the proof's origin is on the decision, not stamped by the caller"
        );
        assert!(
            decision.grades.contains(&(
                voxel_model::axis::AxisKind::TicketEvidence,
                "B".to_string()
            )),
            "the ticket-evidence ladder carries the origin's own token: {:?}",
            decision.grades
        );
        let _ = std::fs::remove_dir_all(&scratch);
    }

    #[test]
    fn a_toast_lasts_the_duration_the_design_system_declares_in_the_world_s_ticks() {
        // **The relationship, not the number.** `TOAST_MS` is a *setting* — the design
        // system's value — and the user changes settings; what must never drift is the
        // conversion between a duration and the world's tick rate, because a toast whose
        // two halves disagree lives a different number of seconds than the record says.
        // So the expected count is derived from the two declared numbers rather than
        // stated, and changing `TOAST_MS` moves the expectation with it. The previous form
        // of this test pinned 6 s = 120 ticks and went red the moment the user changed his
        // own setting — a setting made unchangeable by a test is clutter, not protection.
        let hz = crate::clock::TICK_HZ as f64;
        let expected = (voxel_ui::toast_ms() / 1000.0 * hz).round().max(1.0) as u64;
        assert_eq!(toast_ticks(), expected, "the duration in ticks is the duration in seconds");
        assert!(toast_ticks() >= 1, "a toast must last at least one renderable tick");
        let entry = (voxel_ui::toast_entry_ms() / 1000.0 * hz).round().max(1.0) as u64;
        assert_eq!(
            toast_entry_ticks(),
            entry,
            "MD1's 200 ms desktop entry, in world ticks — derived, not stated"
        );
    }

    #[test]
    fn a_toast_queue_renders_one_record_at_a_time_and_each_record_once() {
        let mut queue = ToastQueue::default();
        queue.enqueue("first");
        queue.enqueue("second");
        assert!(queue.current().is_none(), "nothing is on screen before the first advance");
        queue.advance(10);
        assert_eq!(queue.current().map(|toast| toast.text.as_str()), Some("first"));
        assert_eq!(queue.raised(), 1);

        // Advancing again inside the same tick changes nothing: the queue moves at the world's
        // rate, not at whatever rate the frame loop asks it to.
        queue.advance(10);
        assert_eq!(queue.current().map(|toast| toast.text.as_str()), Some("first"));
        assert_eq!(queue.raised(), 1);

        let done = 10 + toast_ticks();
        queue.advance(done - 1);
        assert_eq!(
            queue.current().map(|toast| toast.text.as_str()),
            Some("first"),
            "the record keeps its whole turn"
        );
        queue.advance(done);
        assert_eq!(
            queue.current().map(|toast| toast.text.as_str()),
            Some("second"),
            "the next record takes the screen"
        );
        assert_eq!(queue.raised(), 2, "one promotion per record");
        queue.advance(done + toast_ticks());
        assert!(queue.current().is_none(), "the queue is drained");
        assert_eq!(queue.raised(), 2, "and no record was rendered twice");
    }

    #[test]
    fn a_queued_toast_gets_its_whole_life_from_the_tick_it_is_promoted_not_from_its_enqueue_tick() {
        // A record that waited its turn must not inherit the wait: a burst would otherwise expire
        // the later records before they were ever drawn, which is a rendering nobody got.
        // The turn length is the declared setting, not a literal: with `TOAST_MS` at 500 ms
        // the first record's turn is ten ticks, and this is what the test should say.
        let turn = toast_ticks();
        let mut queue = ToastQueue::default();
        queue.enqueue("first");
        queue.enqueue("second");
        queue.advance(0);
        assert_eq!(
            queue.current().map(|t| t.text.as_str()),
            Some("first"),
            "the first record takes the screen"
        );
        queue.advance(turn - 1);
        assert_eq!(
            queue.current().map(|t| t.text.as_str()),
            Some("first"),
            "the first keeps its whole turn"
        );
        queue.advance(turn);
        let toast = queue.current().expect("the second record is now on screen");
        assert_eq!(toast.text, "second");
        assert_eq!(
            toast.shown_at, turn,
            "its life starts where its turn started, not where it waited"
        );
        assert_eq!(queue.remaining(turn), Some((turn, 1.0)));
        assert_eq!(queue.remaining(turn + turn), Some((0, 0.0)));
    }
}
