//! Ingesting the corpus: what the **other trees on this machine** hold, measured before a byte of
//! it is imported.
//!
//! # Why this module reads heads and never bodies
//!
//! The user's direction is that material from the other projects across these drives is consumed
//! into this record's format, so that *"we can see exactly where and how every decision along the
//! chain was made"* and compare upstream against downstream changes. That is an **import**, and an
//! import is a write. Before one byte can be imported honestly, the corpus has to be *measured* —
//! which trees exist, which of them carry records, chains or documentation, and which of their
//! records this tree already carries.
//!
//! Measuring the whole corpus by reading it would cost more than the import: the vendored mod
//! collection on `F:` is 47 repositories and the sibling trees each hold tens of megabytes of prose.
//! So the walk classifies a file from its **name and its first [`HEAD_BYTES`]**, records the bytes it
//! actually read against the bytes on disk, and prints that share. If the share is a fraction of one
//! percent, the survey is cheap; if a survey ever has to read a whole file, the report says so
//! instead of hiding the cost.
//!
//! # What this is not
//!
//! It is **not the import**. Nothing here writes to the corpus, and nothing here writes a record:
//! the survey's own line says so. It is also **not a scan of everything on the drives** — it walks
//! the roots a **declared manifest** names ([`MANIFEST_PATH`]), because a scan that picks its own
//! roots is a scan that silently misses a drive, and because a walk of a whole volume cannot run in
//! a build gate.
//!
//! # Why it is a module and not a shell script
//!
//! The same reason `space` and `patch` are: the counts have to be *assertable*, and the report has
//! to be the same text wherever it is read. The module is a library function; the example
//! (`cargo ingest`) is the escape hatch that runs it by hand, exactly as `cargo space` is for the
//! size guard.

use std::path::{Path, PathBuf};

use crate::patch::{self, PatchSet};

/// The declared roots, one per line: `label <TAB> kind <TAB> path`. `#` starts a comment.
pub const MANIFEST_PATH: &str = "intake/roots.tsv";

/// How much of a file is read to classify it. Frontmatter lives at the top, so this is where the
/// fields a record declares can be found without touching a body.
pub const HEAD_BYTES: usize = 4096;

/// Directories that are never worth walking: build output, caches, and other tools' databases.
const SKIP_DIRS: &[&str] = &[
    ".git", "target", "node_modules", ".venv", "venv", "__pycache__", ".gradle", "build", "out",
    ".cargo", "dist", "logs",
];

/// What a root *is*, which is the first thing a lineage reading needs: two copies of the same
/// project at different times, a tree that teaches this one, or a corpus of foreign sources.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// This tree. Named in the manifest so a survey can measure it the same way as any other.
    ThisTree,
    /// Another `shploingus` tree: the same project at another time.
    Sibling,
    /// A tree this one learned from — read as reasons, never as requirements.
    Upstream,
    /// A collection of foreign sources, vendored as material rather than as a record.
    Corpus,
    /// A different project with its own decisions to consume.
    Other,
}

impl Kind {
    pub const ALL: [Kind; 5] = [
        Kind::ThisTree,
        Kind::Sibling,
        Kind::Upstream,
        Kind::Corpus,
        Kind::Other,
    ];

    pub fn slug(self) -> &'static str {
        match self {
            Kind::ThisTree => "this-tree",
            Kind::Sibling => "sibling",
            Kind::Upstream => "upstream",
            Kind::Corpus => "corpus",
            Kind::Other => "other",
        }
    }

    pub fn parse(text: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|kind| kind.slug() == text)
    }
}

/// One declared root.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Root {
    pub label: String,
    pub kind: Kind,
    pub path: PathBuf,
}

/// The roots a survey is allowed to walk. **A manifest, not a scan**: an absent manifest measures
/// nothing, which is a state the report prints rather than an empty success.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Manifest {
    pub roots: Vec<Root>,
    pub declared: bool,
    pub problems: Vec<String>,
}

impl Manifest {
    pub fn read(workspace: &Path) -> Manifest {
        let path = workspace.join(MANIFEST_PATH);
        let Ok(text) = std::fs::read_to_string(&path) else {
            return Manifest {
                roots: Vec::new(),
                declared: false,
                problems: Vec::new(),
            };
        };
        let mut roots = Vec::new();
        let mut problems = Vec::new();
        for (number, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let fields: Vec<&str> = line.split('\t').map(str::trim).collect();
            if fields.len() != 3 {
                problems.push(format!(
                    "line {} is not `label <TAB> kind <TAB> path`: {line}",
                    number + 1
                ));
                continue;
            }
            let Some(kind) = Kind::parse(fields[1]) else {
                problems.push(format!(
                    "line {} names the kind {:?}, which is none of {}",
                    number + 1,
                    fields[1],
                    Kind::ALL
                        .iter()
                        .map(|kind| kind.slug())
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
                continue;
            };
            if roots.iter().any(|root: &Root| root.label == fields[0]) {
                problems.push(format!("two roots are labelled {:?}", fields[0]));
                continue;
            }
            roots.push(Root {
                label: fields[0].to_string(),
                kind,
                path: PathBuf::from(fields[2]),
            });
        }
        Manifest {
            roots,
            declared: true,
            problems,
        }
    }
}

/// What a walk is allowed to spend. **A budget, because the corpus is on other drives**: `F:` took
/// 400 seconds to answer a naive `find` in this session, so a walk that cannot be stopped is a walk
/// that cannot be run in a build.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Budget {
    /// Files visited per root before the walk stops.
    pub max_files: usize,
    /// Directory levels below the root.
    pub max_depth: usize,
    /// Bytes read per file.
    pub head_bytes: usize,
}

impl Budget {
    pub const DEFAULT: Budget = Budget {
        max_files: 20_000,
        max_depth: 6,
        head_bytes: HEAD_BYTES,
    };

    /// A small budget, for a test or a first look.
    pub const SMALL: Budget = Budget {
        max_files: 200,
        max_depth: 3,
        head_bytes: HEAD_BYTES,
    };
}

/// What a root holds, counted.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Artifacts {
    /// Files that are a **record**: named `PATCH-*`, or carrying `patch_id:` in the head.
    pub records: usize,
    /// How many *distinct* ids those files carry. **The difference between the two is the finding**:
    /// the corpus holds 294 record files for 54 distinct ids, because every generation of this
    /// project vendored the same sixteen records four times over. A count of files alone reads as a
    /// count of decisions, and it is not one.
    pub distinct_records: usize,
    /// `*.chain` files — another tree's append-only record.
    pub chains: usize,
    /// Markdown that is not a record: documentation, plans, handoffs.
    pub docs: usize,
    /// Source files, by extension. **Counted, never read**: a line count here would need every byte.
    pub code: usize,
    /// Bytes on disk for every file the walk classified. Free: `metadata` carries it.
    pub bytes_total: u64,
    /// Bytes actually read — the heads. This is the survey's own cost, printed.
    pub bytes_read: u64,
    /// Set when the walk stopped early, with its reason. **A truncated survey is never a
    /// complete one**, and this is how a reader knows which they are looking at.
    pub truncated: Option<String>,
}

impl Artifacts {
    /// The share of the corpus this survey actually read, as a percentage.
    pub fn read_share(&self) -> f64 {
        if self.bytes_total == 0 {
            return 0.0;
        }
        100.0 * self.bytes_read as f64 / self.bytes_total as f64
    }
}

/// How one of this tree's records compares with the same id found in another tree — which is the
/// question *"how were downstream vs upstream changes made"* asked of a single record.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Shared {
    pub id: String,
    /// `same`, `newer-there`, `newer-here`, or `diverged` (same date, different state).
    pub verdict: &'static str,
    /// The record's own state, as the other tree declares it.
    pub their_status: String,
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Lineage {
    pub shared: Vec<Shared>,
    /// Record ids this tree does not carry at all.
    pub only_there: Vec<String>,
}

/// One root, measured.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RootSurvey {
    pub label: String,
    pub kind: Kind,
    pub path: PathBuf,
    pub exists: bool,
    pub artifacts: Artifacts,
    pub lineage: Lineage,
}

impl RootSurvey {
    pub fn shared(&self) -> usize {
        self.lineage.shared.len()
    }
}

/// The whole reading.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Survey {
    pub measured: bool,
    pub why_unmeasured: Option<String>,
    pub roots: Vec<RootSurvey>,
}

impl Survey {
    pub fn unmeasured(why: String) -> Survey {
        Survey {
            measured: false,
            why_unmeasured: Some(why),
            roots: Vec::new(),
        }
    }

    pub fn describe(&self) -> String {
        let Some(why) = &self.why_unmeasured else {
            let mut out = String::from("ingest\tmeasured\n");
            out.push_str(&format!("ingest-roots\t{}\n", self.roots.len()));
            let (mut present, mut records, mut bytes_total, mut bytes_read) = (0, 0, 0u64, 0u64);
            for root in &self.roots {
                out.push_str(&format!(
                    "ingest-root\t{}\t{}\t{}\texists\t{}\n",
                    root.label,
                    root.kind.slug(),
                    root.path.display(),
                    if root.exists { "yes" } else { "no" }
                ));
                if !root.exists {
                    continue;
                }
                present += 1;
                let found = &root.artifacts;
                records += found.records;
                bytes_total += found.bytes_total;
                bytes_read += found.bytes_read;
                out.push_str(&format!(
                    "ingest-artifacts\t{}\trecords\t{}\tdistinct\t{}\tduplicate-files\t{}\tchains\t{}\tdocs\t{}\tcode\t{}\n",
                    root.label,
                    found.records,
                    found.distinct_records,
                    found.records.saturating_sub(found.distinct_records),
                    found.chains,
                    found.docs,
                    found.code
                ));
                out.push_str(&format!(
                    "ingest-bytes\t{}\ttotal\t{}\tread\t{}\tshare\t{:.3}\n",
                    root.label,
                    found.bytes_total,
                    found.bytes_read,
                    found.read_share()
                ));
                if let Some(reason) = &found.truncated {
                    out.push_str(&format!("ingest-truncated\t{}\t{reason}\n", root.label));
                }
                out.push_str(&format!(
                    "ingest-lineage\t{}\tshared\t{}\tonly-there\t{}\n",
                    root.label,
                    root.lineage.shared.len(),
                    root.lineage.only_there.len()
                ));
                for shared in &root.lineage.shared {
                    out.push_str(&format!(
                        "ingest-shared\t{}\t{}\t{}\ttheir-status\t{}\n",
                        root.label, shared.id, shared.verdict, shared.their_status
                    ));
                }
                for id in &root.lineage.only_there {
                    out.push_str(&format!("ingest-only-there\t{}\t{id}\n", root.label));
                }
            }
            out.push_str(&format!(
                "ingest-root-count\t{present}\tof\t{}\n",
                self.roots.len()
            ));
            out.push_str(&format!(
                "ingest-total\trecords\t{records}\tbytes-total\t{bytes_total}\tbytes-read\t{bytes_read}\tshare\t{:.3}\n",
                if bytes_total == 0 { 0.0 } else { 100.0 * bytes_read as f64 / bytes_total as f64 }
            ));
            out.push_str(
                "ingest-is-not\tthe import. This is a read of other trees' names and first \
                 pages: it writes no record, changes no byte, and trusts no count it did not \
                 take. A root it cannot open is reported absent, and a walk it cannot finish is \
                 reported truncated — a shortened survey reads as a complete one otherwise.\n",
            );
            out.push_str(
                "ingest-reads-not-bodies\ta record is classified from its first page because \
                 frontmatter sits at the top of one, and the share of the corpus actually read is \
                 printed beside every root; a survey that read every byte would cost more than the \
                 import it exists to price.\n",
            );
            return out;
        };
        format!("ingest\tnot-measured\ningest-why\t{why}\n")
    }
}

/// A manifest that was not declared measures nothing, because silence is not permission.
fn nothing_declared() -> Survey {
    Survey::unmeasured(format!(
        "{MANIFEST_PATH} is not declared, so no root was walked"
    ))
}

/// Walk every declared root and report what it holds, with the lineage of its records against
/// `ours`.
pub fn survey(manifest: &Manifest, ours: &PatchSet, budget: &Budget) -> Survey {
    if !manifest.declared {
        return nothing_declared();
    }
    let mut roots = Vec::new();
    for declared in &manifest.roots {
        let exists = declared.path.is_dir();
        let mut artifacts = Artifacts::default();
        let mut foreign: Vec<(String, String)> = Vec::new();
        if exists {
            foreign = walk(&declared.path, budget, &mut artifacts);
            artifacts.distinct_records = {
                let mut ids: Vec<&str> = foreign.iter().map(|(id, _)| id.as_str()).collect();
                ids.sort_unstable();
                ids.dedup();
                ids.len()
            };
        }
        let lineage = if exists {
            lineage(&foreign, declared, ours)
        } else {
            Lineage::default()
        };
        roots.push(RootSurvey {
            label: declared.label.clone(),
            kind: declared.kind,
            path: declared.path.clone(),
            exists,
            artifacts,
            lineage,
        });
    }
    Survey {
        measured: true,
        why_unmeasured: None,
        roots,
    }
}

/// Walk one root, classifying from names and heads, and return the records it carries — each with
/// the state the other tree declares for it, which is what a lineage comparison needs.
fn walk(root: &Path, budget: &Budget, found: &mut Artifacts) -> Vec<(String, String)> {
    let mut records = Vec::new();
    let mut visited = 0usize;
    let mut stack = vec![(root.to_path_buf(), 0usize)];
    while let Some((directory, depth)) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        let mut children: Vec<PathBuf> = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            if metadata.is_dir() {
                let name = path.file_name().and_then(|name| name.to_str()).unwrap_or("");
                if depth < budget.max_depth && !SKIP_DIRS.contains(&name) {
                    children.push(path);
                }
                continue;
            }
            if visited >= budget.max_files {
                found.truncated = Some(format!(
                    "the walk stopped after {} files at {}; the counts above are a floor",
                    budget.max_files,
                    directory.display()
                ));
                return records;
            }
            visited += 1;
            found.bytes_total += metadata.len();
            classify(&path, metadata.len() as usize, budget, found, &mut records);
        }
        // Sorted so a truncated walk is the same walk twice, and popped so the walk is depth-first
        // in name order rather than in whatever order the filesystem answers.
        children.sort();
        for child in children.into_iter().rev() {
            stack.push((child, depth + 1));
        }
    }
    records
}

/// `PATCH-900-a-record.md` → `PATCH-900`: the id is the first two dash-separated fields, and only a
/// name that starts with `PATCH` has one.
fn id_from_name(name: &str) -> Option<String> {
    let mut fields = name.split('-');
    match (fields.next(), fields.next()) {
        (Some("PATCH"), Some(number)) => Some(format!("PATCH-{number}")),
        _ => None,
    }
}

/// One file: its kind from its name, and — for a record — its declared id and state from its head.
fn classify(
    path: &Path,
    bytes: usize,
    budget: &Budget,
    found: &mut Artifacts,
    records: &mut Vec<(String, String)>,
) {
    let name = path.file_name().and_then(|name| name.to_str()).unwrap_or("");
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if extension == "chain" {
        found.chains += 1;
        return;
    }
    if extension != "md" {
        if matches!(
            extension.as_str(),
            "rs" | "java" | "kt" | "ts" | "tsx" | "js" | "py" | "c" | "cc" | "cpp" | "h" | "hlsl"
        ) {
            found.code += 1;
        }
        return;
    }
    // A record is named like one, or declares an id in its head. The head is read, never the body.
    let head = read_head(path, bytes, budget.head_bytes, found);
    let declared_id = head
        .lines()
        .find_map(|line| line.strip_prefix("patch_id:"))
        .map(|value| value.trim().to_string());
    match declared_id.or_else(|| id_from_name(name)) {
        Some(id) => {
            found.records += 1;
            let status = head
                .lines()
                .find_map(|line| line.strip_prefix("status:"))
                .map(|value| value.trim().to_string())
                .unwrap_or_else(|| "<no status declared>".to_string());
            records.push((id, status));
        }
        None => found.docs += 1,
    }
}

/// The first `head_bytes` of a file, counted as read. A file that cannot be opened contributes no
/// bytes and no record, which is a fact the counts show rather than a failure the walk hides.
fn read_head(path: &Path, bytes: usize, head_bytes: usize, found: &mut Artifacts) -> String {
    use std::io::Read;
    let Ok(mut file) = std::fs::File::open(path) else {
        return String::new();
    };
    let ceiling = head_bytes.min(bytes);
    let mut buffer = vec![0u8; ceiling];
    let read = file.read(&mut buffer).unwrap_or(0);
    found.bytes_read += read as u64;
    String::from_utf8_lossy(&buffer[..read]).to_string()
}

/// Compare the records another tree carries with this tree's own records, by id.
fn lineage(foreign: &[(String, String)], declared: &Root, ours: &PatchSet) -> Lineage {
    let mut shared = Vec::new();
    let mut only_there = Vec::new();
    let mut seen: Vec<&str> = Vec::new();
    for (id, their_status) in foreign {
        if seen.contains(&id.as_str()) {
            continue;
        }
        seen.push(id);
        match ours.records.iter().find(|record| record.id == *id) {
            Some(mine) => shared.push(Shared {
                id: id.clone(),
                verdict: compare(mine, declared),
                their_status: their_status.clone(),
            }),
            None => only_there.push(id.clone()),
        }
    }
    shared.sort_by(|left, right| left.id.cmp(&right.id));
    only_there.sort();
    Lineage { shared, only_there }
}

/// The verdict for an id both trees carry.
///
/// **Deliberately weak, and named so.** The survey reads the other tree's head only, so it can
/// compare what *this* tree holds for the id against what *that* tree declares — and nothing finer.
/// `carried-here` means this tree has it applied; `owed-here` means this tree has it and has not
/// applied it. A claim about the other tree's *content* is the import's job: a survey that answered
/// it would be reading the bodies it exists to avoid reading.
fn compare(mine: &patch::PatchRecord, declared: &Root) -> &'static str {
    if declared.kind == Kind::ThisTree {
        return "this-root";
    }
    if mine.status == "applied" {
        "carried-here"
    } else {
        "owed-here"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fixture corpus, **named per test** so two tests never share a directory: the placement
    /// fixture's race (PATCH-038) is the reason this signature takes a name.
    fn a_corpus(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("ingest-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let write = |relative: &str, body: &str| {
            let path = root.join(relative);
            std::fs::create_dir_all(path.parent().expect("a parent")).expect("create");
            std::fs::write(path, body).expect("write");
        };
        write(
            "sibling/patches/PATCH-900-a-record.md",
            "---\npatch_id: PATCH-900\nstatus: applied\ntarget: another/tree\n---\n\n# Change\n",
        );
        write(
            "sibling/patches/PATCH-901-owed.md",
            "---\npatch_id: PATCH-901\nstatus: owed\n---\n\n# Change\n",
        );
        // The same record vendored a second time under another directory: the corpus is full of
        // this, and a file count that does not say so reads as a decision count.
        write(
            "sibling/patches/second-copy/PATCH-900-a-record.md",
            "---\npatch_id: PATCH-900\nstatus: applied\ntarget: another/tree\n---\n\n# Change\n",
        );
        write("sibling/chains/sibling.chain", "one\ntwo\n");
        write("sibling/docs/A-DOC.md", "# A document\n\nthat a reader would look in.\n");
        write("sibling/src/lib.rs", "pub fn one() {}\n");
        write("sibling/src/deep/nested/thing.rs", "pub fn two() {}\n");
        write("sibling/target/should-be-skipped.rs", "not counted\n");
        write("corpus/Foreign.java", "class Foreign {}\n");
        write("gigantic/HUGE.md", "x\n");
        root
    }

    fn a_manifest(root: &Path, extra: &str) -> Manifest {
        let path = root.join(MANIFEST_PATH);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("create");
        std::fs::write(
            &path,
            format!(
                "ours\tthis-tree\t{}\nsibling\tsibling\t{}\nabsent\tother\t{}/no-such-root\n{extra}",
                root.display(),
                root.join("sibling").display(),
                root.display()
            ),
        )
        .expect("write the manifest");
        Manifest::read(root)
    }

    fn ours() -> PatchSet {
        PatchSet {
            measured: true,
            why_unmeasured: None,
            records: vec![patch::parse(
                "PATCH-900-a-record.md",
                "---\npatch_id: PATCH-900\nstatus: applied\n---\n\n# Change\n",
            )],
        }
    }

    #[test]
    fn a_manifest_that_is_not_declared_measures_nothing_and_says_so() {
        let survey = survey(&Manifest::read(Path::new("nowhere")), &ours(), &Budget::SMALL);
        assert!(!survey.measured);
        let report = survey.describe();
        assert!(report.contains("ingest\tnot-measured"), "{report}");
        assert!(!report.contains("ingest-artifacts"), "{report}");
    }

    #[test]
    fn the_walk_classifies_by_name_and_counts_every_byte_it_does_not_read() {
        let root = a_corpus("classifies");
        let manifest = a_manifest(&root, "");
        let survey = survey(&manifest, &ours(), &Budget::SMALL);
        assert!(survey.measured);
        let sibling = survey
            .roots
            .iter()
            .find(|root| root.label == "sibling")
            .expect("the sibling root");
        assert!(sibling.exists);
        assert_eq!(sibling.artifacts.records, 3, "{:?}", sibling.artifacts);
        // Two ids for three files: the duplicate is counted, not folded away.
        assert_eq!(sibling.artifacts.distinct_records, 2, "{:?}", sibling.artifacts);
        assert_eq!(sibling.artifacts.chains, 1);
        assert_eq!(sibling.artifacts.docs, 1);
        // Only the two `.rs` files: `target/` is skipped, which is the whole point of the list.
        assert_eq!(sibling.artifacts.code, 2);
        // **The efficiency claim, as a number**: the survey read a fraction of the corpus.
        assert!(
            sibling.artifacts.bytes_read < sibling.artifacts.bytes_total,
            "the survey read every byte"
        );
        assert!(sibling.artifacts.truncated.is_none(), "{:?}", sibling.artifacts.truncated);
        let report = survey.describe();
        assert!(report.contains("ingest-root\tsibling\tsibling\t"), "{report}");
        assert!(report.contains("ingest-is-not\tthe import"), "{report}");
        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn an_absent_root_is_reported_absent_rather_than_as_a_zero() {
        let root = a_corpus("absent");
        let manifest = a_manifest(&root, "");
        let survey = survey(&manifest, &ours(), &Budget::SMALL);
        let absent = survey
            .roots
            .iter()
            .find(|root| root.label == "absent")
            .expect("the absent root");
        assert!(!absent.exists);
        let report = survey.describe();
        assert!(
            report.contains("ingest-root\tabsent\tother\t") && report.contains("\texists\tno"),
            "{report}"
        );
        // An absent root contributes no artifact line: zero-and-absent are different facts.
        assert!(!report.contains("ingest-artifacts\tabsent"), "{report}");
        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_walk_that_cannot_finish_reports_itself_truncated() {
        let root = a_corpus("truncated");
        let manifest = a_manifest(&root, "");
        let tiny = Budget {
            max_files: 1,
            max_depth: 3,
            head_bytes: HEAD_BYTES,
        };
        let survey = survey(&manifest, &ours(), &tiny);
        let sibling = survey
            .roots
            .iter()
            .find(|root| root.label == "sibling")
            .expect("the sibling root");
        let reason = sibling
            .artifacts
            .truncated
            .clone()
            .expect("a one-file budget cannot finish this tree");
        assert!(reason.contains("floor"), "{reason}");
        assert!(
            survey.describe().contains("ingest-truncated\tsibling\t"),
            "a truncated walk is reported"
        );
        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn an_id_both_trees_carry_is_reported_carried_here_and_the_rest_only_there() {
        let root = a_corpus("lineage");
        let manifest = a_manifest(&root, "");
        let survey = survey(&manifest, &ours(), &Budget::SMALL);
        let sibling = survey
            .roots
            .iter()
            .find(|root| root.label == "sibling")
            .expect("the sibling root");
        let shared: Vec<&str> = sibling
            .lineage
            .shared
            .iter()
            .map(|shared| shared.id.as_str())
            .collect();
        assert_eq!(shared, vec!["PATCH-900"], "the id this tree also carries");
        assert_eq!(sibling.lineage.only_there, vec!["PATCH-901"], "and the one it does not");
        assert_eq!(sibling.lineage.shared[0].verdict, "carried-here");
        let report = survey.describe();
        assert!(report.contains("ingest-shared\tsibling\tPATCH-900\tcarried-here"), "{report}");
        assert!(report.contains("ingest-only-there\tsibling\tPATCH-901"), "{report}");
        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_manifest_line_that_names_no_kind_is_a_problem_and_not_a_root() {
        let root = a_corpus("manifest");
        let manifest = a_manifest(&root, "bad\tnonsense\t/tmp\nshort\n");
        assert_eq!(manifest.roots.len(), 3, "the three well-formed lines");
        assert_eq!(manifest.problems.len(), 2, "{:?}", manifest.problems);
        assert!(manifest.problems[1].contains("not `label"), "{:?}", manifest.problems);
        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn two_roots_with_one_label_are_refused_because_a_report_cannot_name_them_apart() {
        let root = a_corpus("duplicate");
        let manifest = a_manifest(&root, "sibling\tother\t/tmp\n");
        assert_eq!(manifest.roots.len(), 3);
        assert!(manifest.problems[0].contains("labelled"), "{:?}", manifest.problems);
        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn the_real_manifest_parses_and_the_real_corpus_surveys() {
        // The reading this module exists to print, on the machine it is running on. Invariants
        // rather than counts: this tree's own root must be present, every declared root's presence
        // must be stated, and the survey must not have read a large share of what it measured.
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
        let manifest = Manifest::read(&workspace);
        assert!(manifest.declared, "{} is not declared", MANIFEST_PATH);
        assert!(manifest.problems.is_empty(), "{:?}", manifest.problems);
        assert!(
            manifest.roots.iter().any(|root| root.kind == Kind::ThisTree),
            "the manifest does not name this tree"
        );
        let ours = patch::read_dir(&workspace.join("patches"));
        // **`SMALL` here and `DEFAULT` on the command line.** The suite must not walk other drives:
        // this test asserts the manifest's shape and the report's invariants, and `cargo ingest` is
        // where the full survey is asked. A budget is what makes that difference explicit.
        let survey = survey(&manifest, &ours, &Budget::SMALL);
        assert!(survey.measured);
        let report = survey.describe();
        for line in report.lines().filter(|line| line.starts_with("ingest-root\t")) {
            assert!(line.contains("exists\t"), "{line}");
        }
        println!("{report}");
    }
}
