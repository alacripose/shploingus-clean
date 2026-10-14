//! The conformance verdict: one rule, askable by every door that wants to refuse.
//! A refusal is rejected unless a declared list has accepted it, and an accepted one is still reported.

use std::path::Path;

use crate::census::{self, Establishment};

/// Where a declared acceptance list lives. Absent is **nothing accepted**, which is the strict default.
pub const ACCEPTED_PATH: &str = "conformance/accepted-refusals.txt";

/// One cell that is not `active`, as the census measured it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Refusal {
    pub crate_: String,
    pub axis: String,
    pub state: String,
    pub why: String,
}

impl Refusal {
    /// The key an acceptance list is written in: `crate/axis`.
    pub fn key(&self) -> String {
        format!("{}/{}", self.crate_, self.axis)
    }
}

/// What the gate says about the whole tree at one moment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Verdict {
    /// Every refusal measured, accepted or not.
    pub refusals: Vec<Refusal>,
    /// The refusals a declared list accepts. **Reported, never hidden.**
    pub accepted: Vec<Refusal>,
    /// Cells where **no instrument ran**. Not a pass, and not an acceptance either.
    pub unchecked: Vec<Refusal>,
    /// Cells whose reading **was taken over bytes that have since changed**: a stale reading, which
    /// is neither a pass nor a refusal.
    ///
    /// **Reported, and deliberately not fatal.** What makes a reading stale is that it cannot be
    /// re-derived here — the count comes from a compiler the build script cannot invoke from inside
    /// itself — so the repair is a command a person runs, not an edit. A door that refused on it
    /// would fail every build after every edit until somebody ran that command, which is exactly how
    /// a gate teaches everyone to route around it. So it is counted and printed at the same weight as
    /// a rejection, and `is_active` does not turn on it.
    pub stale: Vec<Refusal>,
    /// Keys the declared list carries that **no refusal in the tree answers**.
    ///
    /// A stale entry is the list's own failure mode, and it is the mirror of a rejection: a
    /// rejection is work the list does not admit, while a stale entry is an admission with no work
    /// behind it — a permission that re-admits the refusal it names the moment that axis comes
    /// back, and prints nothing while it waits, because a key that answers no refusal answers
    /// quietly.
    pub unmatched: Vec<String>,
    /// Whether an acceptance list was declared at all.
    pub accepted_declared: bool,
    /// Subjects measured.
    pub tested: usize,
}

impl Verdict {
    /// The refusals nothing has accepted: this is the work the door rejects.
    pub fn rejected(&self) -> Vec<&Refusal> {
        self.refusals
            .iter()
            .filter(|refusal| !self.accepted.iter().any(|taken| taken.key() == refusal.key()))
            .collect()
    }

    /// `true` only when nothing is rejected, nothing is unchecked, and **no declared key is
    /// unanswered**.
    ///
    /// [`stale`](Self::stale) does not close the door: a reading about a tree that moved is
    /// re-derived by re-running a recipe, and a fatality here would refuse every build in between.
    /// It is not silence either — `describe` prints every stale cell and its reason.
    pub fn is_active(&self) -> bool {
        self.rejected().is_empty() && self.unchecked.is_empty() && self.unmatched.is_empty()
    }

    /// A state the gate's own vocabulary names, so two callers cannot disagree about the word.
    pub fn state(&self) -> Establishment {
        if self.is_active() {
            Establishment::Active
        } else {
            Establishment::Refused
        }
    }

    /// The report, `key\tvalue` like every other gate line, with each rejected refusal on its own.
    pub fn describe(&self) -> String {
        let rejected = self.rejected();
        let mut out = format!(
            "conformance\t{}\nconformance-tested\t{}\nconformance-refused\t{}\nconformance-accepted\t{}\nconformance-rejected\t{}\nconformance-unchecked\t{}\nconformance-stale\t{}\nconformance-unmatched\t{}\nconformance-accepted-list\t{}\n",
            self.state().slug(),
            self.tested,
            self.refusals.len(),
            self.accepted.len(),
            rejected.len(),
            self.unchecked.len(),
            self.stale.len(),
            self.unmatched.len(),
            if self.accepted_declared { ACCEPTED_PATH } else { "not declared" },
        );
        // **Every stale reading, by name, with its reason.** Printed like a rejection because the
        // reader's action is the same shape — one line that says what to do — even though nothing here
        // gates a build. `conformance-stale-acceptance` below is a *different* staleness (a declared key
        // no refusal answers); the shared word is the tree's: a reading that no longer describes the
        // thing it was taken over.
        for refusal in &self.stale {
            out.push_str(&format!(
                "conformance-stale-reading\t{}\t{}\t{}\n",
                refusal.crate_, refusal.axis, refusal.why
            ));
        }
        for refusal in &rejected {
            out.push_str(&format!(
                "conformance-rejection\t{}\t{}\t{}\n",
                refusal.crate_,
                refusal.axis,
                refusal.why
            ));
        }
        for refusal in &self.accepted {
            out.push_str(&format!(
                "conformance-accepted-by-name\t{}\t{}\n",
                refusal.crate_,
                refusal.axis
            ));
        }
        // **The list naming work it does not have.** Printed one key per line, like a rejection,
        // because the repair is the same shape: a line to delete, named.
        for key in &self.unmatched {
            out.push_str(&format!("conformance-stale-acceptance\t{key}\n"));
        }
        out
    }
}

/// The verdict at `root`, with the acceptance list read from [`ACCEPTED_PATH`] when one exists.
pub fn verdict(root: &Path) -> Result<Verdict, String> {
    let census = census::census(root).map_err(|found| voxel_model::RefusalSet::to_string(&found))?;
    let declared = accepted(root);
    Ok(of(&census, &declared.0, declared.1))
}

/// The verdict over a census already taken, so a caller holding one does not walk the tree twice.
pub fn of(census: &census::Census, accepted_keys: &[String], accepted_declared: bool) -> Verdict {
    let mut refusals = Vec::new();
    let mut unchecked = Vec::new();
    let mut stale = Vec::new();
    for reading in &census.readings {
        for (axis, state, why) in reading.gaps() {
            let refusal = Refusal {
                crate_: reading.crate_.name().to_string(),
                axis: axis.name().to_string(),
                state: state.slug().to_string(),
                why: why.unwrap_or("no reason recorded").to_string(),
            };
            if state == Establishment::Unchecked {
                unchecked.push(refusal);
            } else if state == Establishment::Refused {
                refusals.push(refusal);
            } else if state == Establishment::Stale {
                stale.push(refusal);
            }
        }
    }
    let (accepted, unmatched) = split(accepted_keys, &refusals);
    Verdict {
        refusals,
        accepted,
        unchecked,
        stale,
        unmatched,
        accepted_declared,
        tested: census.readings.len(),
    }
}

/// A declared list split against the refusals a census measured: **what it accepts, and what it
/// names that nothing answers.**
///
/// Both halves are needed, and the second is not bookkeeping. While the sixteen `documented`
/// sections were owed, sixteen keys answered sixteen refusals and the list was the axis's
/// exception; the day the sections landed the same sixteen keys answered nothing — and a list of
/// entries that answer nothing is not an empty list. It is a **standing permission**: the refusal
/// each key names comes straight back if that axis regresses, and nothing prints while it waits.
/// `accepted` alone can never see that, because it is computed by asking which refusals the keys
/// match, a question that never looks at the keys nothing matched.
fn split(accepted_keys: &[String], refusals: &[Refusal]) -> (Vec<Refusal>, Vec<String>) {
    let accepted = refusals
        .iter()
        .filter(|refusal| accepted_keys.iter().any(|key| *key == refusal.key()))
        .cloned()
        .collect();
    let unmatched = accepted_keys
        .iter()
        .filter(|key| !refusals.iter().any(|refusal| refusal.key() == **key))
        .cloned()
        .collect();
    (accepted, unmatched)
}

/// The declared acceptance list, and whether it was declared at all. **An absent list accepts nothing**,
/// because a gate that treats silence as permission is the gate that admits whatever it cannot read.
pub fn accepted(root: &Path) -> (Vec<String>, bool) {
    let path = root.join(ACCEPTED_PATH);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return (Vec::new(), false);
    };
    let keys = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
        .collect();
    (keys, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a_refusal(crate_: &str, axis: &str, state: Establishment) -> Refusal {
        Refusal {
            crate_: crate_.to_string(),
            axis: axis.to_string(),
            state: state.slug().to_string(),
            why: format!("{crate_} {axis} was measured and refused"),
        }
    }

    fn verdict_with(refusals: Vec<Refusal>, accepted: Vec<Refusal>, unchecked: Vec<Refusal>) -> Verdict {
        Verdict {
            refusals,
            accepted,
            unchecked,
            stale: Vec::new(),
            unmatched: Vec::new(),
            accepted_declared: true,
            tested: 12,
        }
    }

    #[test]
    fn a_stale_reading_is_reported_by_name_and_does_not_close_the_door() {
        // **The state a snapshot can be in without being wrong.** The `documented` axis's count is
        // taken by a command a person runs, so the gate can detect that its pin no longer matches the
        // files — and *detecting* it is all the door should do, because the repair is re-running the
        // recipe, not editing the tree. If this were fatal, every edit to any crate would refuse every
        // build until somebody ran a compiler by hand, and the check would be routed around within a
        // day. So: counted, printed with its reason, and not a rejection.
        let stale = a_refusal("voxel-model", "documented", Establishment::Stale);
        let verdict = Verdict {
            refusals: Vec::new(),
            accepted: Vec::new(),
            unchecked: Vec::new(),
            stale: vec![stale],
            unmatched: Vec::new(),
            accepted_declared: true,
            tested: 12,
        };
        assert!(
            verdict.is_active(),
            "a stale reading is not a refusal and must not close the door: {}",
            verdict.describe()
        );
        assert_eq!(verdict.state(), Establishment::Active);
        let report = verdict.describe();
        assert!(report.contains("conformance-stale\t1"), "{report}");
        assert!(
            report.contains("conformance-stale-reading\tvoxel-model\tdocumented\t"),
            "and it names the crate and the axis, so the repair is a command and not an \
             investigation: {report}"
        );
        // The two stalenesses in this vocabulary are different facts and must not print alike.
        assert!(!report.contains("conformance-stale-acceptance\t"), "{report}");
    }

    #[test]
    fn a_stale_reading_is_not_reachable_from_the_acceptance_list() {
        // **A permission cannot be written for a stale reading**, and that is the design rather than
        // an omission: accepting a refusal says *this is known and tolerated*, while a stale reading is
        // a command somebody forgot to run. Keys are matched against refusals only, so a key naming a
        // stale cell is an unmatched declaration — the gate's error, printed as such.
        let (accepted, unmatched) = split(&["voxel-model/documented".to_string()], &[]);
        assert!(accepted.is_empty() && unmatched.len() == 1, "no refusal answers that key");
    }

    #[test]
    fn an_unaccepted_refusal_is_what_the_gate_rejects() {
        let verdict = verdict_with(
            vec![a_refusal("voxel-model", "documented", Establishment::Refused)],
            Vec::new(),
            Vec::new(),
        );
        assert_eq!(verdict.state(), Establishment::Refused);
        assert!(!verdict.is_active());
        let rejected = verdict.rejected();
        assert_eq!(rejected.len(), 1);
        assert_eq!(rejected[0].key(), "voxel-model/documented");
        assert!(verdict.describe().contains("conformance-rejection\tvoxel-model\tdocumented"));
    }

    #[test]
    fn an_accepted_refusal_is_still_reported_and_no_longer_rejected() {
        // The retrofit's shape: the refusals already known are named on a list, and the gate then
        // refuses only what is not on it. **Accepted is not deleted** — the line still prints.
        let refusal = a_refusal("voxel-model", "documented", Establishment::Refused);
        let verdict = verdict_with(vec![refusal.clone()], vec![refusal], Vec::new());
        assert!(verdict.is_active(), "{}", verdict.describe());
        assert_eq!(verdict.refusals.len(), 1, "the refusal is still counted");
        assert!(verdict.describe().contains("conformance-accepted-by-name\tvoxel-model\tdocumented"));
        assert!(!verdict.describe().contains("conformance-rejection\t"));
        // The discriminator for the other half: a key that *is* answered is not a stale entry, so
        // an accepted-and-answered list must print no stale line at all.
        assert!(verdict.unmatched.is_empty(), "{}", verdict.describe());
        assert!(!verdict.describe().contains("conformance-stale-acceptance\t"));
    }

    #[test]
    fn an_unchecked_cell_is_not_a_pass_even_when_nothing_is_refused() {
        let verdict = verdict_with(
            Vec::new(),
            Vec::new(),
            vec![a_refusal("voxel-mesh", "claimed", Establishment::Unchecked)],
        );
        assert!(!verdict.is_active(), "no instrument ran, which is not a pass");
        assert_eq!(verdict.state(), Establishment::Refused);
        assert!(verdict.describe().contains("conformance-unchecked\t1"));
    }

    #[test]
    fn an_absent_acceptance_list_accepts_nothing_and_says_that_it_was_not_declared() {
        let nowhere = Path::new("this/directory/does/not/exist");
        let (keys, declared) = accepted(nowhere);
        assert!(keys.is_empty() && !declared, "silence is not permission");
        // The declared flag is the whole assertion here, so the fixture sets it false rather than
        // taking the helper's `true`: a test that cannot tell the two apart is not testing this.
        let verdict = Verdict {
            refusals: vec![a_refusal("voxel-client", "documented", Establishment::Refused)],
            accepted: Vec::new(),
            unchecked: Vec::new(),
            stale: Vec::new(),
            unmatched: Vec::new(),
            accepted_declared: false,
            tested: 12,
        };
        assert!(verdict.describe().contains("conformance-accepted-list\tnot declared"));
        assert!(
            !verdict.describe().contains("conformance-accepted-list\tconformance/"),
            "an absent list must not print as a declared one: {}",
            verdict.describe()
        );
        assert!(!verdict.is_active());
    }

    #[test]
    fn the_real_tree_has_no_refusal_the_declared_list_does_not_accept() {
        // **The door's state, as a test rather than only as a build-script behaviour.**
        // `crates/shploingus/build.rs` refuses the build when this is not active — but building
        // `shploingus` needs the GPU stack, so a refusal there is invisible to anyone running this
        // crate's tests, which is the cheapest place to see it. A crate added later, or an axis
        // that stops measuring, is a key no line of the declared list carries.
        //
        // The assertion names the file to change, because a fatal door whose message does not say
        // where the decision lives is a door that trains people to edit the gate instead.
        let root = crate::features::workspace_root(std::path::Path::new(env!("CARGO_MANIFEST_DIR")))
            .expect("a workspace root above the crate");
        let verdict = verdict(&root).expect("the tree can be measured");
        assert!(
            verdict.is_active(),
            "the tree is not conformant and the door is fatal. The declaration to change is {}:\n{}",
            ACCEPTED_PATH,
            verdict.describe()
        );
    }

    #[test]
    fn a_declared_key_that_no_refusal_answers_is_a_stale_acceptance_and_is_not_a_pass() {
        // **The retrofit's other half.** `conformance/accepted-refusals.txt` named sixteen
        // `documented` refusals from 2026-10-09 until the sixteen sections landed in
        // each crate's own rustdoc on 2026-10-10. After they landed the refusals were gone and the keys
        // stayed — and a list of keys that answer nothing is not the same as an empty list: each
        // one is a standing permission for a refusal that is no longer there. This is the test
        // that says so, and it is the *only* thing that could: `accepted` is computed by asking
        // which refusals the keys match, so it sees nothing when a key matches nothing.
        let (accepted, unmatched) = split(&["voxel-model/documented".to_string()], &[]);
        assert!(accepted.is_empty(), "no refusal answers this key");
        assert_eq!(unmatched, vec!["voxel-model/documented".to_string()]);
        let verdict = Verdict {
            refusals: Vec::new(),
            accepted,
            unchecked: Vec::new(),
            stale: Vec::new(),
            unmatched,
            accepted_declared: true,
            tested: 12,
        };
        assert!(
            !verdict.is_active(),
            "a declaration with nothing behind it is not a pass: {}",
            verdict.describe()
        );
        assert_eq!(verdict.state(), Establishment::Refused);
        assert!(verdict.describe().contains("conformance-unmatched\t1"));
        assert!(verdict.describe().contains("conformance-stale-acceptance\tvoxel-model/documented"));
    }

    #[test]
    fn the_declared_list_is_read_by_line_and_ignores_comments_and_blank_lines() {
        let mut scratch = std::env::temp_dir();
        scratch.push(format!("shploingus-conformance-accepted-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&scratch);
        std::fs::create_dir_all(scratch.join("conformance")).expect("a scratch workspace");
        std::fs::write(
            scratch.join(ACCEPTED_PATH),
            "# accepted by the retrofit, 2026-10-09\nvoxel-model/documented\n\nvoxel-mesh/documented\n",
        )
        .expect("the declared list");
        let (keys, declared) = accepted(&scratch);
        assert!(declared);
        assert_eq!(keys, vec!["voxel-model/documented", "voxel-mesh/documented"]);
        let _ = std::fs::remove_dir_all(&scratch);
    }
}
