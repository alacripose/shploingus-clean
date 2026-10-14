//! The committed-action recorder: **one row per accepted governed act, and
//! nothing else.**
//!
//! `agency/action.rs`'s single commit point calls [`record_committed_action`]
//! **after** the governed path returned a receipt — never before, and never
//! for a refusal. That ordering is the whole honesty rule of this module: a
//! refused act has no deltas and no authority, so recording it would put a
//! non-event into the evidence the gate is graded against.
//!
//! The row carries **where** it was produced from — the caller's `file`/`line`,
//! read with [`Location::caller`] under `#[track_caller]` so the row names the
//! machine's commit point and not this recorder — because an evidence row that
//! cannot name its own commit site is a claim, not evidence. The game side
//! drains the rows it has not yet taken; a row handed out once never comes
//! back, and what remains is what has not been minted into proof of work yet.

use std::panic::Location;
use std::sync::Mutex;

/// One committed governed act, as the observer holds it.
///
/// Every field is quoted from the receipt the governed path returned — the
/// deltas are the physical change, the evidence rows are the transition's own
/// account of itself, and the site names the commit point in the machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommittedAction {
    /// The verb the act was — the machine's shared physical vocabulary.
    pub verb: String,
    /// Who acted.
    pub actor: String,
    /// The task the act served, or `-` for work that is not a job.
    pub task: String,
    /// The claim the act spent, when it claimed one.
    pub reservation: Option<String>,
    /// How much mass moved — one line per committed delta.
    pub deltas: Vec<String>,
    /// The evidence rows the receipt carried.
    pub evidence: Vec<String>,
    /// The transition's identity, from the receipt.
    pub transition: String,
    /// The commit site: the caller's file and line (`#[track_caller]`), so a
    /// row names the machine's commit point rather than this recorder.
    pub site: String,
}

impl CommittedAction {
    /// The claim anchor this row carries on the record: the reservation it
    /// spent and the task it served, or the transition when the act claims
    /// neither. It is what a proof of work quotes as its `claim` — an anchor,
    /// not a sentence, so a proof always says which claim it stands on.
    pub fn claim(&self) -> String {
        match (&self.reservation, self.task.as_str()) {
            (Some(reservation), "-") => format!("reservation {reservation}"),
            (Some(reservation), task) => format!("reservation {reservation} on task {task}"),
            (None, "-") => format!("transition {}", self.transition),
            (None, task) => format!("transition {} on task {task}", self.transition),
        }
    }

    /// The one-line account a proof of work quotes.
    pub fn message(&self) -> String {
        let deltas = if self.deltas.is_empty() {
            "no deltas".to_string()
        } else {
            format!("{} delta(s)", self.deltas.len())
        };
        format!(
            "{} {} task={} {} at {} — {}",
            self.actor,
            self.verb,
            self.task,
            match &self.reservation {
                Some(reservation) => format!("claim {reservation}"),
                None => "no claim".to_string(),
            },
            self.site,
            deltas
        )
    }
}

fn rows() -> &'static Mutex<Vec<CommittedAction>> {
    static ROWS: Mutex<Vec<CommittedAction>> = Mutex::new(Vec::new());
    &ROWS
}

/// **Test-only lock, so a committed row survives until the test that committed
/// it has read the book back.** The book is process-global, so without this a
/// parallel test can drain a row before the committing test evaluates. Tests
/// that commit-and-read hold this lock; production never sees it.
#[cfg(test)]
pub static BOOK_LOCK: Mutex<()> = Mutex::new(());

/// Record one **accepted** governed act. Called only from the machine's single
/// commit point, only after the receipt came back — see the module header.
///
/// **`#[track_caller]` is load-bearing**: `file!()` and `line!()` expand where
/// the macro is written, so without it every row's site would read
/// `recorder.rs` and the row could not name its commit site at all.
/// [`Location::caller`] reads the caller's file and line instead — the
/// machine's own commit point in `agency/action.rs`.
#[track_caller]
#[allow(clippy::too_many_arguments)]
pub fn record_committed_action(
    verb: &str,
    actor: &str,
    task: &str,
    reservation: Option<String>,
    deltas: &[String],
    evidence: &[String],
    transition: String,
) -> CommittedAction {
    let location = Location::caller();
    let row = CommittedAction {
        verb: verb.to_owned(),
        actor: actor.to_owned(),
        task: task.to_owned(),
        reservation,
        deltas: deltas.to_vec(),
        evidence: evidence.to_vec(),
        transition,
        site: format!("{}:{}", location.file(), location.line()),
    };
    // A poisoned lock is not a reason to drop an act on the floor: the guard
    // recovers the rows that are there and carries on.
    rows().lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .push(row.clone());
    row
}

/// Hand over every row recorded so far and empty the book. The game side mints
/// proof of work from what it takes; a row taken once is never offered again.
pub fn drain_committed_actions() -> Vec<CommittedAction> {
    #[cfg(test)]
    {
        let _book_guard = BOOK_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        return rows()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .drain(..)
            .collect();
    }
    #[cfg(not(test))]
    {
        rows()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .drain(..)
            .collect()
    }
}

/// Drain the book **without** taking [`BOOK_LOCK`]. For tests that hold the
/// lock across a commit-and-read sequence so a parallel test cannot drain the
/// row in between. Production never calls this; it has no parallel readers.
#[cfg(test)]
pub fn drain_committed_actions_locked() -> Vec<CommittedAction> {
    rows()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .drain(..)
        .collect()
}

/// How many rows are waiting. Read, not taken.
pub fn pending_committed_actions() -> usize {
    #[cfg(test)]
    let _book_guard = BOOK_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    rows()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .len()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **A test-only lock, because the book is process-wide.**
    ///
    /// `rows()` is a global static and both tests below take it apart, so run in
    /// parallel they race each other — measured: one read `2` where it asserted `1`,
    /// and a drained book refilled between its drain and its emptiness check. There is
    /// no scratch-book seam here (the recorder is deliberately global: one commit
    /// point, one book), so the two tests serialize on this lock, and each starts from
    /// a **drained** book so its counts are absolute rather than relative to whatever
    /// the other left. Draining at the start is the same move the notification tests
    /// make with their scratch queue: the fixture owns its state, and a row another
    /// test recorded cannot be read as this one's.
    fn exclusive_book() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: Mutex<()> = Mutex::new(());
        let guard = LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let _ = drain_committed_actions();
        guard
    }

    #[test]
    fn a_recorded_action_names_its_commit_site_and_reads_back_whole() {
        let _book = exclusive_book();
        let row = record_committed_action(
            "dig",
            "civ-1",
            "task-9",
            Some("res-4".to_owned()),
            &["rock 4g substrate -> spoil:civ-1".to_owned()],
            &["transition.committed: the books balanced".to_owned()],
            "transition-digest".to_owned(),
        );
        assert_eq!(row.verb, "dig");
        assert_eq!(row.task, "task-9");
        assert!(row.site.contains("recorder.rs"), "{}", row.site);
        assert!(row.message().contains("claim res-4"), "{}", row.message());
        assert_eq!(
            pending_committed_actions(),
            1,
            "the book holds exactly the row just recorded"
        );
    }

    #[test]
    fn a_drained_row_is_not_offered_twice() {
        let _book = exclusive_book();
        record_committed_action("place", "civ-2", "-", None, &[], &[], "t2".to_owned());
        assert_eq!(
            pending_committed_actions(),
            1,
            "the book holds exactly the row just recorded"
        );
        let drained = drain_committed_actions();
        assert_eq!(drained.len(), 1, "the drain hands over the row it held");
        assert_eq!(pending_committed_actions(), 0, "the book drains to empty");
        let again = drain_committed_actions();
        assert!(again.is_empty(), "a drained row is not offered twice");
    }
}
