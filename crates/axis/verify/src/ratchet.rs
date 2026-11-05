//! **The baseline ratchet: a moved number is only counted when an act says it moved.**
//!
//! Reads `patches/*.md` recipe acts and `docs/baselines/*.json`, joins them on the ticket, and
//! refuses a baseline measured at another commit and an act with no claim, evidence or proof.

use std::path::{Path, PathBuf};

use serde_json::Value;
use voxel_axis_matrix::Task;
use voxel_model::identity::Digest;
use voxel_model::proof::{Proof, verify_bytes};
use voxel_model::ticket::EvidenceOrigin;

/// The word a row's `must` states, decided once so the ratchet and a reader agree on what a law is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Must {
    /// The value must be this number.
    Equal,
    /// The value may not exceed the recorded one.
    AtMost,
    /// The value may not fall below the recorded one.
    AtLeast,
    /// The value may move, but not further than this percentage.
    Within(String),
    /// A structural claim the row's name states (the corpus spells these `declared` and `holds`).
    Declared(String),
    /// A note, not a law: the corpus spells these `note`, `noise` and `within the noise band`.
    Note,
    /// A word this module does not know, carried rather than dropped.
    Unknown(String),
}

impl Must {
    /// Read a row's `must` word.
    pub fn parse(word: &str) -> Must {
        let word = word.trim();
        match word {
            "equal" => Must::Equal,
            "at most" => Must::AtMost,
            "at least" => Must::AtLeast,
            "note" | "noise" | "within the noise band" => Must::Note,
            "declared" | "holds" => Must::Declared(word.to_string()),
            _ if word.starts_with("within ") && word.ends_with('%') => Must::Within(word.to_string()),
            _ => Must::Unknown(word.to_string()),
        }
    }

    /// Whether a row with this word is a law the ratchet will count.
    pub fn is_law(&self) -> bool {
        !matches!(self, Must::Note | Must::Unknown(_))
    }

    /// The word as `lines` prints it.
    pub fn name(&self) -> &str {
        match self {
            Must::Equal => "equal",
            Must::AtMost => "at most",
            Must::AtLeast => "at least",
            Must::Within(word) => word,
            Must::Declared(word) => word,
            Must::Note => "note",
            Must::Unknown(word) => word,
        }
    }
}

/// One row of a baseline file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BaselineRow {
    /// The row's name, as the file states it.
    pub name: String,
    /// The requirement, parsed.
    pub must: Must,
    /// The unit the value is in, when the row states one.
    pub unit: Option<String>,
}

/// One baseline file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Baseline {
    /// The file, workspace-relative.
    pub path: String,
    /// What the file measures.
    pub what: String,
    /// The commit the reading was taken at, as written (`6049414 + working tree`).
    pub commit: Option<String>,
    /// Whether the commit names the head the caller passed.
    pub current: bool,
    /// Whether the reading was taken over uncommitted bytes.
    pub dirty: bool,
    /// Every row, laws and notes together.
    pub rows: Vec<BaselineRow>,
}

impl Baseline {
    /// The rows the ratchet counts.
    pub fn laws(&self) -> usize {
        self.rows.iter().filter(|row| row.must.is_law()).count()
    }
}

/// One recipe block, read back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Act {
    /// The record the act came from, workspace-relative.
    pub patch: String,
    /// The ticket the act is filed under.
    pub ticket: String,
    /// The matrix segment the act names.
    pub segment: String,
    /// The axis the act names.
    pub axis: String,
    /// `path:line`, as the record states it.
    pub site: String,
    /// Where the measurement is kept afterwards.
    pub witness: String,
    /// The class of the check command, or `None` when it does not place itself.
    pub task: Option<Task>,
    /// Whether the act's proof is complete and its bytes hashed to what it names.
    pub proved: bool,
    /// The one reason the act is not kept, if it is not.
    pub refusal: Option<String>,
}

impl Act {
    /// Whether this act is counted.
    pub fn kept(&self) -> bool {
        self.refusal.is_none()
    }
}

/// How much of the baseline directory is in the format at all.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Coverage {
    /// Every regular file under `docs/baselines/`, recursively.
    pub files: usize,
    /// The files the ratchet could read as baselines (`.json`).
    pub formatted: usize,
}

impl Coverage {
    /// Files that are evidence of something and are in no baseline the ratchet can read.
    pub fn outside(&self) -> usize {
        self.files.saturating_sub(self.formatted)
    }
}

/// The whole reading.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ratchet {
    /// The head the reading was taken against.
    pub head: String,
    /// Every act, in patch order.
    pub acts: Vec<Act>,
    /// Every baseline file, in path order.
    pub baselines: Vec<Baseline>,
    /// What the baseline directory holds, against what it formats.
    pub coverage: Coverage,
    /// Everything wrong with the *files* rather than with one act or row.
    pub refusals: Vec<String>,
}

impl Ratchet {
    /// How many acts are counted.
    pub fn kept(&self) -> usize {
        self.acts.iter().filter(|act| act.kept()).count()
    }

    /// How many acts are refused.
    pub fn refused(&self) -> usize {
        self.acts.iter().filter(|act| !act.kept()).count()
    }

    /// Whether every act and every baseline is current and complete.
    pub fn verified(&self) -> bool {
        self.refusals.is_empty() && self.refused() == 0 && self.acts.iter().all(|act| act.proved)
    }

    /// The acts of one class.
    pub fn of(&self, task: Task) -> impl Iterator<Item = &Act> {
        self.acts.iter().filter(move |act| act.task == Some(task))
    }

    /// The report, one line per act and per baseline, with a tail that states the totals.
    pub fn lines(&self) -> Vec<String> {
        let mut out = vec![format!("ratchet {}", self.head)];
        for act in &self.acts {
            out.push(format!(
                "act {} {} {}/{} {} {} site={} witness={}{}",
                act.patch,
                act.ticket,
                act.segment,
                act.axis,
                act.task.map(Task::name).unwrap_or("unclassified"),
                if act.kept() { "kept" } else { "refused" },
                act.site,
                act.witness,
                act.refusal
                    .as_ref()
                    .map(|why| format!(" — {why}"))
                    .unwrap_or_default(),
            ));
        }
        for baseline in &self.baselines {
            out.push(format!(
                "baseline {} {} commit={} {} ({} row(s), {} law(s))",
                baseline.path,
                baseline.what,
                baseline.commit.as_deref().unwrap_or("none"),
                if baseline.current { "current" } else { "stale" },
                baseline.rows.len(),
                baseline.laws(),
            ));
        }
        for refusal in &self.refusals {
            out.push(format!("ratchet-refused {refusal}"));
        }
        out.push(format!(
            "ratchet: {} act(s), {} kept, {} refused, {} baseline(s) of {} file(s) under \
             docs/baselines ({} outside the format), {} unplaceable command(s)",
            self.acts.len(),
            self.kept(),
            self.refused(),
            self.coverage.formatted,
            self.coverage.files,
            self.coverage.outside(),
            self.acts.iter().filter(|act| act.task.is_none()).count(),
        ));
        out
    }
}

/// Every regular file under `dir`, recursively; a directory that will not read counts zero.
fn count_files(dir: &Path) -> usize {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    let mut total = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            total += count_files(&path);
        } else if path.is_file() {
            total += 1;
        }
    }
    total
}

/// **Read the tree's acts and baselines against `head`.** Nothing is written.
pub fn ratchet(root: &Path, head: &str) -> Ratchet {
    let mut out = Ratchet { head: head.to_string(), ..Ratchet::default() };
    out.coverage.files = count_files(&root.join("docs/baselines"));
    out.baselines = read_baselines(root, head, &mut out.refusals);
    out.coverage.formatted = out.baselines.len();
    out.acts = read_acts(root, head, &mut out.refusals);
    out
}

/// Read every `docs/baselines/*.json`.
fn read_baselines(root: &Path, head: &str, refusals: &mut Vec<String>) -> Vec<Baseline> {
    let dir = root.join("docs/baselines");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        refusals.push(format!("{} could not be read", dir.display()));
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|extension| extension.to_str()) == Some("json"))
        .collect();
    paths.sort();
    let mut out = Vec::new();
    for path in paths {
        let name = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        match std::fs::read_to_string(&path) {
            Ok(text) => match parse_baseline(&name, &text, head) {
                Ok(baseline) => {
                    for row in &baseline.rows {
                        if let Must::Unknown(word) = &row.must {
                            refusals.push(format!(
                                "{name}: row `{}` states `{word}`, which is not a requirement",
                                row.name
                            ));
                        }
                    }
                    if !baseline.current {
                        refusals.push(format!(
                            "{name}: measured at {}, and the head is {head}",
                            baseline.commit.as_deref().unwrap_or("no commit")
                        ));
                    }
                    out.push(baseline);
                }
                Err(why) => refusals.push(format!("{name}: {why}")),
            },
            Err(why) => refusals.push(format!("{name} could not be read: {why}")),
        }
    }
    if out.is_empty() {
        refusals.push("no baseline file was found; nothing was measured".to_string());
    }
    out
}

/// Parse one baseline file's documented shape.
pub fn parse_baseline(path: &str, text: &str, head: &str) -> Result<Baseline, String> {
    let value: Value = serde_json::from_str(text).map_err(|why| format!("not JSON: {why}"))?;
    let object = value.as_object().ok_or("the file is not an object")?;
    let what = object
        .get("what")
        .and_then(Value::as_str)
        .ok_or("no `what`: a baseline must say what it measures")?
        .to_string();
    let commit = object.get("commit").and_then(Value::as_str).map(str::to_string);
    let current = commit
        .as_deref()
        .and_then(|commit| commit.split_whitespace().next())
        .is_some_and(|commit| commit == head);
    let dirty = commit
        .as_deref()
        .is_some_and(|commit| commit.contains("working tree"));
    let rows = object
        .get("rows")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    let name = row.get("name")?.as_str()?.to_string();
                    let must = Must::parse(row.get("must")?.as_str()?);
                    let unit = row.get("unit").and_then(Value::as_str).map(str::to_string);
                    Some(BaselineRow { name, must, unit })
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(Baseline { path: path.to_string(), what, commit, current, dirty, rows })
}

/// Read every `patches/*.md` recipe block.
fn read_acts(root: &Path, head: &str, refusals: &mut Vec<String>) -> Vec<Act> {
    let dir = root.join("patches");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        refusals.push(format!("{} could not be read", dir.display()));
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|extension| extension.to_str()) == Some("md"))
        .collect();
    paths.sort();
    let mut out = Vec::new();
    for path in paths {
        let name = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let Ok(text) = std::fs::read_to_string(&path) else {
            refusals.push(format!("{name} could not be read"));
            continue;
        };
        let record = voxel_package::patch::parse(
            path.file_name().and_then(|name| name.to_str()).unwrap_or(&name),
            &text,
        );
        for recipe in &record.recipes {
            out.push(judge(root, head, &name, &text, &record.id, recipe));
        }
    }
    out
}

/// One act, judged against the tree.
fn judge(
    root: &Path,
    head: &str,
    patch: &str,
    text: &str,
    record_id: &str,
    recipe: &voxel_package::patch::PatchRecipe,
) -> Act {
    let task = Task::classify(&recipe.check);
    let mut act = Act {
        patch: patch.to_string(),
        ticket: recipe.ticket.clone(),
        segment: recipe.segment.clone(),
        axis: recipe.axis.clone(),
        site: recipe.site.clone(),
        witness: recipe.witness.clone(),
        task,
        proved: false,
        refusal: None,
    };
    if let Some(problem) = recipe.problems.first() {
        act.refusal = Some(format!("patch-problem: {problem}"));
        return act;
    }
    if recipe.ticket.trim().is_empty() {
        act.refusal = Some("no ticket".to_string());
        return act;
    }
    if let Some((site, _)) = recipe.site_parts() {
        if !root.join(site).exists() {
            act.refusal = Some(format!("site {site} does not exist"));
        }
    } else {
        act.refusal = Some(format!("site `{}` is not path:line", recipe.site));
    }
    if act.refusal.is_none() {
        let witness = recipe.witness.split(':').next().unwrap_or("").trim();
        if witness.is_empty() || !root.join(witness).exists() {
            act.refusal = Some(format!("witness `{}` does not exist", recipe.witness));
        }
    }
    if act.refusal.is_none() && task.is_none() {
        act.refusal = Some(format!("check `{}` is neither a CPU nor a rendering gate", recipe.check));
    }
    let proof = Proof {
        artifact: patch.to_string(),
        artifact_digest: Digest::of(text.as_bytes()),
        claim: recipe.what.clone(),
        origin: EvidenceOrigin::Derived,
        evidence: recipe.check.clone(),
        tick_span: (0, 0),
        world_digest: Digest::of(head.as_bytes()),
        presenter: record_id.to_string(),
        ticket: recipe.ticket.clone(),
    };
    let proved = !proof.claim.trim().is_empty()
        && !proof.evidence.trim().is_empty()
        && !proof.presenter.trim().is_empty()
        && verify_bytes(root, &proof).is_ok();
    act.proved = proved;
    if act.refusal.is_none() && !proved {
        act.refusal = Some(format!("no proof on record for {patch}"));
    }
    act
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn scratch(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "voxel-axis-ratchet-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("patches")).expect("patches");
        fs::create_dir_all(root.join("docs/baselines")).expect("baselines");
        fs::create_dir_all(root.join("crates/thing/src")).expect("source");
        fs::write(root.join("crates/thing/src/lib.rs"), "// the site\n").expect("site");
        root
    }

    fn record(check: &str) -> String {
        format!(
            "---\npatch_id: PATCH-900\ntitle: A scratch act\nstatus: applied\noperation: add\n\
             target: thing\nbase_ref: working tree\ndependencies: []\nsource: test\n\
             created_at: 2026-10-26\nlive_mutation: false\n---\n\n# A scratch act\n\n\
             ```recipe\nsegment: tooling\naxis: evidence_grade\nticket: T-9000\n\
             site: crates/thing/src/lib.rs:1\nwhat: state the act\nbecause: the ratchet reads it\n\
             check: {check}\nwitness: crates/thing/src/lib.rs\nexpects: 1\n```\n"
        )
    }

    fn baseline(commit: &str, must: &str) -> String {
        format!(
            "{{\"what\": \"scratch\", \"commit\": \"{commit}\", \"rows\": \
             [{{\"name\": \"a row\", \"value\": 1, \"must\": \"{must}\"}}]}}"
        )
    }

    #[test]
    fn a_complete_act_over_a_current_baseline_is_kept() {
        let root = scratch("kept");
        fs::write(root.join("patches/PATCH-900.md"), record("cargo test -p thing --lib")).unwrap();
        fs::write(root.join("docs/baselines/scratch.json"), baseline("abc123", "equal")).unwrap();
        let reading = ratchet(&root, "abc123");
        assert_eq!(reading.acts.len(), 1, "{:#?}", reading.lines());
        assert_eq!(reading.kept(), 1, "{:#?}", reading.lines());
        assert_eq!(reading.acts[0].task, Some(Task::Cpu));
        assert!(reading.acts[0].proved);
        assert_eq!(reading.baselines[0].laws(), 1);
        assert!(reading.refusals.is_empty(), "{:?}", reading.refusals);
        assert!(reading.verified(), "{:#?}", reading.lines());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_launch_is_rendering_and_an_unplaceable_command_is_refused() {
        let root = scratch("class");
        fs::write(
            root.join("patches/PATCH-900.md"),
            record("cargo run --release -p shploingus -> exit 0"),
        )
        .unwrap();
        fs::write(root.join("docs/baselines/scratch.json"), baseline("abc123", "at most")).unwrap();
        assert_eq!(ratchet(&root, "abc123").acts[0].task, Some(Task::Rendering));

        fs::write(root.join("patches/PATCH-900.md"), record("./run.sh")).unwrap();
        let reading = ratchet(&root, "abc123");
        assert_eq!(reading.kept(), 0);
        assert!(
            reading.acts[0]
                .refusal
                .as_deref()
                .unwrap_or("")
                .contains("neither a CPU nor a rendering gate"),
            "{:?}",
            reading.acts[0].refusal
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_baseline_measured_at_another_commit_is_stale_and_counts_for_nothing() {
        let root = scratch("stale");
        fs::write(root.join("patches/PATCH-900.md"), record("cargo test -p thing --lib")).unwrap();
        fs::write(root.join("docs/baselines/scratch.json"), baseline("abc123", "equal")).unwrap();
        let reading = ratchet(&root, "def456");
        assert!(!reading.verified());
        assert!(reading.refusals.iter().any(|why| why.contains("measured at abc123")));
        assert!(!reading.baselines[0].current);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_requirement_word_the_ratchet_does_not_know_is_named_rather_than_counted() {
        assert_eq!(Must::parse("within 5%"), Must::Within("within 5%".to_string()));
        assert_eq!(Must::parse("noise"), Must::Note);
        assert!(!Must::parse("noise").is_law());
        assert_eq!(Must::parse("bogus"), Must::Unknown("bogus".to_string()));
        assert!(!Must::parse("bogus").is_law());
        let root = scratch("must");
        fs::write(root.join("patches/PATCH-900.md"), record("cargo test -p thing --lib")).unwrap();
        fs::write(root.join("docs/baselines/scratch.json"), baseline("abc123", "bogus")).unwrap();
        let reading = ratchet(&root, "abc123");
        assert!(!reading.verified());
        assert!(reading.refusals.iter().any(|why| why.contains("not a requirement")));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_repository_reads_as_acts_that_place_themselves() {
        let root = voxel_package::features::workspace_root(Path::new(env!("CARGO_MANIFEST_DIR")))
            .expect("this crate sits inside the workspace");
        let reading = ratchet(&root, "6049414");
        assert!(!reading.acts.is_empty(), "{:#?}", reading.lines());
        assert!(
            reading.acts.iter().all(|act| act.task.is_some()),
            "an act's gate did not place itself: {:#?}",
            reading
                .acts
                .iter()
                .filter(|act| act.task.is_none())
                .collect::<Vec<_>>()
        );
        assert!(!reading.baselines.is_empty());
        assert!(
            reading.of(Task::Cpu).count() + reading.of(Task::Rendering).count()
                == reading.acts.len()
        );
    }
}
