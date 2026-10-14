//! The patch records: what has been applied, what is owed, what is dangling — and **which parts of
//! the tree no record names at all.**
//!
//! A record that cannot name its base digest or its four sections is `not_evaluable`, never passed.
//!
//! # The join this module was missing
//!
//! `get_ownership` answers *"which record names this path?"* for one path at a time, which is a
//! question nobody asks about a file they have not already opened. The question the tree needed was
//! the other direction — **for every part, which records name it?** — and until it was asked, the
//! answer for most of the game was *none*: the patches named the gate and the package, and eleven of
//! the twelve parts the census measures were named by no record at all.
//!
//! **The join is over `crates/<part>/`, never over a bare name.** A record's table that says
//! `voxel-model` has written a *word*; a record that names `crates/voxel-model/src/matter/mod.rs` has
//! named an artifact. This is the same distinction `census::heading_defines` draws for prose (*"a
//! mention is not a definition"*), applied to records, and it is why the coverage line for a part
//! lists the record ids rather than a count: a reader has to be able to see *which* records, since
//! a record that reads the tree is not a record that changed it.

use std::path::{Path, PathBuf};

use crate::census::Crate;

/// The four sections every record carries. A missing one is named by name.
pub const SECTIONS: [&str; 4] = ["# Change", "# Acceptance", "# Evidence", "# Rollback"];

/// The statuses a record may declare. An unknown one is refused rather than carried.
pub const STATUSES: [&str; 7] = [
    "proposed",
    "designed",
    "approved",
    "applied",
    "applied-internal",
    "verified-internal",
    "superseded",
];

/// Words a record writes when its base digest was never established (PATCH-009).
pub const PLACEHOLDER_DIGESTS: [&str; 4] = ["tbd", "todo", "none", "recorded source digest"];

/// One patch record, as read. `problems` is a list because the first problem is not the only one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatchRecord {
    /// `PATCH-017`, and it must be the name the file carries.
    pub id: String,
    pub title: String,
    pub status: String,
    pub target: String,
    pub base_digest: Option<String>,
    pub dependencies: Vec<String>,
    pub live_mutation: bool,
    /// How many of [`SECTIONS`] the body carries.
    pub sections_present: usize,
    /// **Every workspace path this record names**, sorted and deduplicated. What the coverage join
    /// reads, and what `get_ownership` answers from, so the two cannot disagree about a path.
    pub named_paths: Vec<String>,
    pub problems: Vec<String>,
}

impl PatchRecord {
    /// A record with no problems. Anything else is `not_evaluable`.
    pub fn complete(&self) -> bool {
        self.problems.is_empty()
    }
}

/// Every record under a directory, with the order they were read in.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PatchSet {
    pub measured: bool,
    pub why_unmeasured: Option<String>,
    pub records: Vec<PatchRecord>,
}

impl PatchSet {
    pub fn unmeasured(why: impl Into<String>) -> Self {
        PatchSet { measured: false, why_unmeasured: Some(why.into()), records: Vec::new() }
    }

    /// Dependencies that name a record no file in this set carries.
    pub fn dangling(&self) -> Vec<String> {
        let mut out = Vec::new();
        for record in &self.records {
            for dependency in &record.dependencies {
                if !self.records.iter().any(|candidate| candidate.id == *dependency) {
                    out.push(format!("{} depends on {dependency}", record.id));
                }
            }
        }
        out
    }

    /// Records whose id appears more than once.
    pub fn duplicated(&self) -> Vec<String> {
        let mut out = Vec::new();
        for (index, record) in self.records.iter().enumerate() {
            if self.records[..index].iter().any(|earlier| earlier.id == record.id) {
                out.push(record.id.clone());
            }
        }
        out
    }

    /// The parts of the tree no record names, and the records that name nothing in it.
    ///
    /// **The join this module exists for.** It is not a refusal: a part with no record is a part
    /// whose history was never written down, which is a hole in the record rather than a broken
    /// build, and a gate that refused every hole would refuse every build.
    pub fn coverage(&self, parts: &[Crate]) -> Coverage {
        if !self.measured {
            return Coverage {
                measured: false,
                why_unmeasured: self.why_unmeasured.clone(),
                parts: Vec::new(),
                unnamed_records: Vec::new(),
            };
        }
        let parts = parts
            .iter()
            .map(|part| {
                // `crates/voxel-model/` — the trailing separator is what makes this a join over
                // artifacts rather than over names that happen to share a prefix.
                let inside = format!("{}/", part.path());
                let records: Vec<String> = self
                    .records
                    .iter()
                    .filter(|record| {
                        record
                            .named_paths
                            .iter()
                            .any(|path| path.starts_with(&inside))
                    })
                    .map(|record| record.id.clone())
                    .collect();
                PartCoverage {
                    name: part.name(),
                    path: part.path(),
                    records,
                }
            })
            .collect();
        let unnamed_records = self
            .records
            .iter()
            .filter(|record| record.named_paths.is_empty())
            .map(|record| record.id.clone())
            .collect();
        Coverage {
            measured: true,
            why_unmeasured: None,
            parts,
            unnamed_records,
        }
    }

    /// What the whole set adds up to. **Counts, not a score** — a record is applied or it is not.
    pub fn describe(&self) -> String {
        if !self.measured {
            return format!(
                "patch-set\tnot-measured\npatch-set-why\t{}",
                self.why_unmeasured.as_deref().unwrap_or("no reason recorded")
            );
        }
        let applied = self
            .records
            .iter()
            .filter(|record| record.status == "applied" || record.status.ends_with("-internal"))
            .count();
        let unevaluable = self.records.iter().filter(|record| !record.complete()).count();
        let dangling = self.dangling();
        let mut out = String::new();
        out.push_str(&format!("patch-set-records\t{}\n", self.records.len()));
        out.push_str(&format!("patch-set-applied\t{applied}\n"));
        out.push_str(&format!("patch-set-owed\t{}\n", self.records.len() - applied));
        out.push_str(&format!("patch-set-unevaluable\t{unevaluable}\n"));
        out.push_str(&format!("patch-set-dangling\t{}\n", dangling.len()));
        out
    }
}

/// What kind of line a file carries. Three kinds, because the tree's own words are three.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    /// `.rs` under `crates/`.
    Code,
    /// `.wgsl` under `crates/` — the rendering work, which is a kind of its own because a shader is
    /// neither prose nor a Rust file and nothing else in the tree accounts for it.
    Shader,
    /// `.md` under `docs/`.
    Document,
}

impl Kind {
    pub const ALL: [Kind; 3] = [Kind::Code, Kind::Shader, Kind::Document];

    pub const fn slug(self) -> &'static str {
        match self {
            Kind::Code => "code",
            Kind::Shader => "shader",
            Kind::Document => "document",
        }
    }

    /// Where the files of this kind live, relative to the workspace root.
    const fn root(self) -> &'static str {
        match self {
            Kind::Code | Kind::Shader => "crates",
            Kind::Document => "docs",
        }
    }

    fn matches(self, path: &Path) -> bool {
        let extension = path.extension().and_then(|extension| extension.to_str());
        match self {
            Kind::Code => extension == Some("rs"),
            Kind::Shader => extension == Some("wgsl"),
            Kind::Document => extension == Some("md"),
        }
    }
}

/// The files this tree does not write: a generator does, and **their place is the generator.**
///
/// This list is the one exception to *"every line was written by someone"*, and it is declared
/// rather than inferred, because a report that counted a product as unwritten work would be listing
/// a debt that does not exist. Each entry names the thing that produces it — the same files
/// [voxel-model's build script](crates/voxel-model/build.rs) recomputes and refuses the build over.
pub const GENERATED: [(&str, &str); 2] = [
    (
        "crates/voxel-model/src/materials/generated.rs",
        "written by `cargo run -p voxel-model --bin materials-generator` from `materials/*.tsv`, and \
         refused by crates/voxel-model/build.rs when it stops matching its declaration",
    ),
    (
        "crates/voxel-model/src/materials/spectra.rs",
        "the same generator, from `materials/spectra/`, checked by the same build script",
    ),
];

/// One file, and the records that name it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Placed {
    pub path: String,
    pub lines: usize,
    /// Empty means **unplaced**: no record names this file.
    pub records: Vec<String>,
}

impl Placed {
    pub fn is_placed(&self) -> bool {
        !self.records.is_empty()
    }
}

/// One kind of line, counted both ways.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KindPlacement {
    pub kind: Kind,
    pub files: usize,
    pub lines: usize,
    pub generated_files: usize,
    pub generated_lines: usize,
    pub placed_files: usize,
    pub placed_lines: usize,
    /// Every unplaced file, biggest first. **Not truncated**: a shortened work list reads as a
    /// complete one.
    pub unplaced: Vec<Placed>,
}

impl KindPlacement {
    pub fn unplaced_files(&self) -> usize {
        self.unplaced.len()
    }

    pub fn unplaced_lines(&self) -> usize {
        self.unplaced.iter().map(|file| file.lines).sum()
    }
}

/// **Every line's place.** Whether the file a line is in is named by a record, counted per kind.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Placement {
    pub measured: bool,
    pub why_unmeasured: Option<String>,
    pub kinds: Vec<KindPlacement>,
}

impl Placement {
    pub fn kind(&self, kind: Kind) -> Option<&KindPlacement> {
        self.kinds.iter().find(|found| found.kind == kind)
    }

    pub fn unplaced_lines(&self) -> usize {
        self.kinds.iter().map(KindPlacement::unplaced_lines).sum()
    }

    /// The gate's lines, in the shape every other reading here uses.
    pub fn describe(&self) -> String {
        if !self.measured {
            return format!(
                "placement\tnot-measured\nplacement-why\t{}",
                self.why_unmeasured.as_deref().unwrap_or("no reason recorded")
            );
        }
        let mut out = String::new();
        for found in &self.kinds {
            out.push_str(&format!(
                "placement-{}\tfiles\t{}\tlines\t{}\tgenerated-files\t{}\tgenerated-lines\t{}\n",
                found.kind.slug(),
                found.files,
                found.lines,
                found.generated_files,
                found.generated_lines
            ));
            out.push_str(&format!(
                "placement-{}\tplaced-files\t{}\tplaced-lines\t{}\tunplaced-files\t{}\tunplaced-lines\t{}\n",
                found.kind.slug(),
                found.placed_files,
                found.placed_lines,
                found.unplaced_files(),
                found.unplaced_lines()
            ));
        }
        for found in &self.kinds {
            for file in &found.unplaced {
                out.push_str(&format!(
                    "placement-unplaced\t{}\t{}\t{}\n",
                    found.kind.slug(),
                    file.path,
                    file.lines
                ));
            }
        }
        for (path, why) in GENERATED {
            out.push_str(&format!("placement-generated\t{path}\t{why}\n"));
        }
        out.push_str(
            "placement-is-not\ta claim that a placed line is understood. It is a claim about where a \
             line's file is named: a file no record names is a file whose history is in no record, \
             and a file named by a record that only read it is placed by a reader, not by a change. \
             The record ids are printed with the counts so that difference stays visible.\n",
        );
        out
    }
}

/// **Measure every line's place**: walk each kind of file, count its lines, and ask which records
/// name it.
///
/// The walk is the filesystem's own — the crates are enumerated by looking, not by reading
/// `Cargo.toml` — which mattered here, because four workspace members (`voxel-light`,
/// `voxel-audio`, `voxel-mcp`, `voxel-von`) had no census row and this report was written not to
/// inherit that blind spot. **The census has since grown the four rows, and
/// `census::check_against_manifest` refuses when the table and the manifest disagree**, so the two
/// sets now agree; the filesystem walk stays because a report about files should not depend on a
/// table about crates being right.
///
/// A file is placed when some record names **it**, or names a **directory above it**: a record that
/// says `crates/voxel-shapers/src/` has named what is in it. A generated file is placed by its
/// generator instead, and counted apart.
pub fn placement(set: &PatchSet, root: &Path) -> Placement {
    if !set.measured {
        return Placement {
            measured: false,
            why_unmeasured: set.why_unmeasured.clone(),
            kinds: Vec::new(),
        };
    }
    let kinds = Kind::ALL
        .iter()
        .map(|kind| kind_placement(set, root, *kind))
        .collect();
    Placement {
        measured: true,
        why_unmeasured: None,
        kinds,
    }
}

fn kind_placement(set: &PatchSet, root: &Path, kind: Kind) -> KindPlacement {
    let mut found: Vec<PathBuf> = Vec::new();
    walk_into(&root.join(kind.root()), kind, &mut found);
    found.sort();
    let mut out = KindPlacement {
        kind,
        files: 0,
        lines: 0,
        generated_files: 0,
        generated_lines: 0,
        placed_files: 0,
        placed_lines: 0,
        unplaced: Vec::new(),
    };
    for path in found {
        // `/` on every platform: a record is text, and the paths in it are written one way.
        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .display()
            .to_string()
            .replace('\\', "/");
        let lines = lines_of(&path);
        out.files += 1;
        out.lines += lines;
        if GENERATED.iter().any(|(generated, _)| *generated == relative) {
            out.generated_files += 1;
            out.generated_lines += lines;
            continue;
        }
        let records: Vec<String> = set
            .records
            .iter()
            .filter(|record| {
                record.named_paths.iter().any(|named| {
                    *named == relative || relative.starts_with(&format!("{named}/"))
                })
            })
            .map(|record| record.id.clone())
            .collect();
        if records.is_empty() {
            out.unplaced.push(Placed {
                path: relative,
                lines,
                records,
            });
        } else {
            out.placed_files += 1;
            out.placed_lines += lines;
        }
    }
    // Biggest first: the work list is read from the top, and the top is where the mass is.
    out.unplaced.sort_by(|left, right| {
        right
            .lines
            .cmp(&left.lines)
            .then_with(|| left.path.cmp(&right.path))
    });
    out
}

/// Every file of one kind under `dir`. **Symlinks are not followed**, the same rule the size guard
/// holds: a link is not a line anyone wrote.
fn walk_into(dir: &Path, kind: Kind, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            walk_into(&path, kind, found);
        } else if kind.matches(&path) {
            found.push(path);
        }
    }
}

/// Lines in a file, counted by newline so a file with no trailing newline is not counted short.
fn lines_of(path: &Path) -> usize {
    std::fs::read(path)
        .map(|bytes| bytes.iter().filter(|byte| **byte == b'\n').count())
        .unwrap_or(0)
}

/// One part of the tree, and the records that name a file inside it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartCoverage {
    pub name: &'static str,
    pub path: &'static str,
    /// The ids of every record naming a path inside this part, in the order they were read.
    pub records: Vec<String>,
}

impl PartCoverage {
    /// Whether any record names a file inside this part.
    pub fn covered(&self) -> bool {
        !self.records.is_empty()
    }
}

/// What a record set says about the tree's parts: the join nothing was making.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Coverage {
    pub measured: bool,
    pub why_unmeasured: Option<String>,
    pub parts: Vec<PartCoverage>,
    /// Records that name **no** workspace path — a record about something else, or about nothing.
    pub unnamed_records: Vec<String>,
}

impl Coverage {
    /// The parts no record names.
    pub fn uncovered(&self) -> Vec<&PartCoverage> {
        self.parts.iter().filter(|part| !part.covered()).collect()
    }

    /// The same `key\tvalue` shape every other gate reading uses, with the ids printed rather than
    /// counted — a count of two does not tell a reader whether the two are change records or
    /// readings.
    pub fn describe(&self) -> String {
        if !self.measured {
            return format!(
                "patch-coverage\tnot-measured\npatch-coverage-why\t{}",
                self.why_unmeasured.as_deref().unwrap_or("no reason recorded")
            );
        }
        let uncovered = self.uncovered();
        let mut out = format!(
            "patch-coverage\tmeasured\npatch-coverage-parts\t{}\npatch-coverage-covered\t{}\npatch-coverage-uncovered\t{}\n",
            self.parts.len(),
            self.parts.len() - uncovered.len(),
            uncovered.len()
        );
        for part in &self.parts {
            if part.covered() {
                out.push_str(&format!(
                    "patch-part\t{}\t{}\trecords\t{}\t{}\n",
                    part.name,
                    part.path,
                    part.records.len(),
                    part.records.join(" ")
                ));
            } else {
                out.push_str(&format!(
                    "patch-part\t{}\t{}\tuncovered\n",
                    part.name, part.path
                ));
            }
        }
        if !uncovered.is_empty() {
            out.push_str(&format!(
                "patch-uncovered\t{}\n",
                uncovered
                    .iter()
                    .map(|part| part.name)
                    .collect::<Vec<_>>()
                    .join(" ")
            ));
        }
        out.push_str(&format!(
            "patch-unnamed-records\t{}\t{}\n",
            self.unnamed_records.len(),
            self.unnamed_records.join(" ")
        ));
        out
    }
}

/// Every workspace path a record names, deduplicated and sorted.
///
/// Mechanical on purpose: a path starts at `crates/` and runs while the characters are path
/// characters. Prose punctuation after a path (`).`, `,`, `'`) is not part of it, and a bare crate
/// name is not a path, so nothing here can turn a *mention* into coverage.
pub fn named_paths(text: &str) -> Vec<String> {
    /// The two roots the placement report measures. **Both, or the document kind reads as wholly
    /// unplaced** — which is what the first version of this function did: it extracted only
    /// `crates/`, so a record naming `docs/LEDGER.md` placed nothing, and the report said the whole
    /// documentation set was in no record.
    const PREFIXES: [&str; 2] = ["crates/", "docs/"];
    let mut found: Vec<String> = Vec::new();
    let mut at = 0usize;
    loop {
        // The **earliest** occurrence of either prefix, so the paths come out in the order they
        // are written rather than grouped by root.
        let next = PREFIXES
            .iter()
            .filter_map(|prefix| text[at..].find(prefix).map(|offset| (at + offset, *prefix)))
            .min();
        let Some((start, prefix)) = next else {
            break;
        };
        let mut end = start;
        for (inner, character) in text[start..].char_indices() {
            if character.is_ascii_alphanumeric() || matches!(character, '/' | '_' | '-' | '.') {
                end = start + inner + character.len_utf8();
            } else {
                break;
            }
        }
        let path = text[start..end].trim_end_matches(['.', ',', ':', ';', ')']);
        if path.len() > prefix.len() && !found.iter().any(|candidate| candidate == path) {
            found.push(path.to_string());
        }
        at = start + prefix.len();
    }
    found.sort();
    found
}

/// Read every `*.md` under `dir`. A directory that will not read is a measurement that did not run.
pub fn read_dir(dir: &Path) -> PatchSet {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(why) => return PatchSet::unmeasured(format!("{} could not be read: {why}", dir.display())),
    };
    let mut paths: Vec<std::path::PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    paths.sort();
    let mut records = Vec::new();
    let mut undecodable = 0usize;
    for path in paths {
        if path.extension().and_then(|extension| extension.to_str()) != Some("md") {
            continue;
        }
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                let name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("<unprintable>")
                    .to_string();
                records.push(parse(&name, &text));
            }
            Err(_) => undecodable += 1,
        }
    }
    if records.is_empty() {
        return PatchSet::unmeasured(format!(
            "no patch record was found under {}; nothing was measured",
            dir.display()
        ));
    }
    let mut set = PatchSet { measured: true, why_unmeasured: None, records };
    if undecodable > 0 {
        // A file left out of the set is a set that reads cleaner than the directory is.
        set.records.push(PatchRecord {
            id: "<undecodable>".to_string(),
            title: String::new(),
            status: String::new(),
            target: String::new(),
            base_digest: None,
            dependencies: Vec::new(),
            live_mutation: false,
            sections_present: 0,
            named_paths: Vec::new(),
            problems: vec![format!("{undecodable} record(s) could not be decoded and are in no count above")],
        });
    }
    set
}

/// One record, from its text and the name of the file it came from.
pub fn parse(file_name: &str, text: &str) -> PatchRecord {
    let mut problems = Vec::new();
    let front = frontmatter(text);
    let Some(front) = front else {
        return PatchRecord {
            id: file_name.to_string(),
            title: String::new(),
            status: String::new(),
            target: String::new(),
            base_digest: None,
            dependencies: Vec::new(),
            live_mutation: false,
            sections_present: 0,
            named_paths: Vec::new(),
            problems: vec!["no frontmatter block, so the record declares nothing".to_string()],
        };
    };
    let field = |key: &str| -> Option<String> { front.iter().find_map(|line| value_of(line, key)) };

    let id = field("patch_id").unwrap_or_default();
    let status = field("status").unwrap_or_default();
    let target = field("target").unwrap_or_default();
    let base_digest = field("base_digest").filter(|digest| !digest.trim().is_empty());
    let dependencies = field("dependencies").map(|list| parse_list(&list)).unwrap_or_default();
    let live_mutation = field("live_mutation").map(|flag| flag.trim() == "true").unwrap_or(false);
    let sections_present = SECTIONS.iter().filter(|section| text.contains(**section)).count();

    // The name and the id are one fact written twice; a file whose id is not in its name is a
    // record that cannot be cited from its own path.
    if id.is_empty() {
        problems.push("no patch_id".to_string());
    } else if !file_name.starts_with(&id) {
        problems.push(format!("the file is {file_name} but the record calls itself {id}"));
    }
    if status.is_empty() {
        problems.push("no status".to_string());
    } else if !STATUSES.contains(&status.as_str()) {
        problems.push(format!("status {status} is not one of {STATUSES:?}"));
    }
    if target.is_empty() {
        problems.push("no target, so nothing is named as changed".to_string());
    }
    match &base_digest {
        None => problems.push("no base_digest, so no version can be restored (PATCH-009)".to_string()),
        Some(digest) => {
            let lowered = digest.to_ascii_lowercase();
            if PLACEHOLDER_DIGESTS.iter().any(|word| lowered.contains(word)) {
                problems.push(format!("base_digest {digest} is a placeholder, not a digest"));
            } else if !lowered.starts_with("sha256:") {
                problems.push(format!("base_digest {digest} is not a sha256 digest"));
            }
        }
    }
    if live_mutation {
        problems.push("live_mutation is true, and no record here may claim a live write".to_string());
    }
    if sections_present < SECTIONS.len() {
        for section in SECTIONS {
            if !text.contains(section) {
                problems.push(format!("no `{section}` section"));
            }
        }
    }

    PatchRecord {
        id,
        title: field("title").unwrap_or_default(),
        status,
        target,
        base_digest,
        dependencies,
        live_mutation,
        sections_present,
        named_paths: named_paths(text),
        problems,
    }
}

/// The lines between the first two `---` markers.
fn frontmatter(text: &str) -> Option<Vec<&str>> {
    let mut lines = text.lines();
    let first = lines.next()?;
    if first.trim().trim_start_matches('\u{feff}') != "---" {
        return None;
    }
    let mut front = Vec::new();
    for line in lines {
        if line.trim() == "---" {
            return Some(front);
        }
        front.push(line);
    }
    None
}

/// `key: value`, with the key matched exactly and separated by a colon.
fn value_of(line: &str, key: &str) -> Option<String> {
    let (found, value) = line.split_once(':')?;
    if found.trim() != key {
        return None;
    }
    Some(value.trim().to_string())
}

/// `[A, B]` or `A`, without a YAML reader.
fn parse_list(value: &str) -> Vec<String> {
    value
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(|item| item.trim().trim_matches('"').to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const A_RECORD: &str = concat!(
        "---\n",
        "patch_id: PATCH-101\n",
        "title: A record\n",
        "status: applied\n",
        "operation: add\n",
        "target: shploingus/example\n",
        "base_digest: sha256:8ec257e3dbcb64702f83660849055720c566af4b7b59b998168b4a9a0615cba9\n",
        "dependencies: [PATCH-100]\n",
        "live_mutation: false\n",
        "---\n\n",
        "# Change\n\nSomething.\n\n",
        "# Acceptance\n\nIt holds.\n\n",
        "# Evidence\n\nA run.\n\n",
        "# Rollback\n\nRestore the digest.\n",
    );

    fn without(section: &str) -> String {
        A_RECORD.replace(section, "# Superseded")
    }

    #[test]
    fn a_record_that_carries_every_section_and_a_real_digest_is_complete() {
        let record = parse("PATCH-101-a-record.md", A_RECORD);
        assert!(record.complete(), "{:?}", record.problems);
        assert_eq!(record.id, "PATCH-101");
        assert_eq!(record.dependencies, vec!["PATCH-100"]);
        assert_eq!(record.sections_present, SECTIONS.len());
    }

    #[test]
    fn a_missing_section_is_named_rather_than_counted() {
        // The point is a work list: "3 of 4 sections" cannot be acted on and `# Rollback` can.
        let record = parse("PATCH-101-a-record.md", &without("# Rollback"));
        assert!(!record.complete());
        assert!(
            record.problems.iter().any(|problem| problem.contains("# Rollback")),
            "{:?}",
            record.problems
        );
    }

    #[test]
    fn a_placeholder_or_absent_digest_is_not_evaluable() {
        // PATCH-009: a boolean is not rollback evidence, and a word where a digest belongs is
        // the same defect in the same place.
        let placeholder = parse(
            "PATCH-101-a-record.md",
            &A_RECORD.replace("sha256:8ec257", "recorded source digest # sha256:8ec257"),
        );
        assert!(!placeholder.complete());
        assert!(
            placeholder.problems.iter().any(|problem| problem.contains("placeholder")),
            "{:?}",
            placeholder.problems
        );

        let absent = parse(
            "PATCH-101-a-record.md",
            &A_RECORD.replace("base_digest: sha256:8ec257e3dbcb64702f83660849055720c566af4b7b59b998168b4a9a0615cba9\n", ""),
        );
        assert!(!absent.complete());
        assert!(
            absent.problems.iter().any(|problem| problem.contains("no base_digest")),
            "{:?}",
            absent.problems
        );
    }

    #[test]
    fn a_dependency_that_names_no_record_is_dangling_and_one_that_names_a_record_is_not() {
        let mut set = PatchSet {
            measured: true,
            why_unmeasured: None,
            records: vec![parse("PATCH-101-a-record.md", A_RECORD)],
        };
        assert_eq!(set.dangling(), vec!["PATCH-101 depends on PATCH-100"]);

        set.records.push(parse("PATCH-100-its-base.md", &A_RECORD.replace("PATCH-101", "PATCH-100")));
        assert!(set.dangling().is_empty(), "{:?}", set.dangling());
    }

    #[test]
    fn a_set_that_did_not_read_says_so_and_prints_no_count() {
        let nothing = PatchSet::unmeasured("no directory");
        let report = nothing.describe();
        assert!(report.contains("patch-set\tnot-measured"), "{report}");
        assert!(!report.contains("patch-set-records"), "{report}");
    }

    #[test]
    fn a_path_is_extracted_from_prose_and_prose_punctuation_is_not_part_of_it() {
        let paths = named_paths(
            "Fixed [bloat.rs](crates/voxel-package/src/bloat.rs)'s reading, and \
             `crates/voxel-client/src/shell/tickets.rs`.) Then crates/voxel-package/src/bloat.rs \
             again, and [TODO.md](docs/TODO.md). Not a path: crates/ — and not a crate name: \
             voxel-package",
        );
        assert_eq!(
            paths,
            vec![
                "crates/voxel-client/src/shell/tickets.rs",
                "crates/voxel-package/src/bloat.rs",
                "docs/TODO.md",
            ]
        );
        // **A document is a path too.** The placement report measures `docs/`, and the first
        // version of this extraction read only `crates/` — so every document in the tree was
        // reported as placed in no record, which was the instrument's bug and not the tree's state.
        assert_eq!(named_paths("see docs/LEDGER.md."), vec!["docs/LEDGER.md"]);
    }

    #[test]
    fn coverage_counts_a_record_that_names_an_artifact_and_not_one_that_names_a_word() {
        // The distinction the whole join rests on: `voxel-model` in a table is a word, and
        // `crates/voxel-model/` is the part.
        let set = PatchSet {
            measured: true,
            why_unmeasured: None,
            records: vec![
                parse(
                    "PATCH-101-names-the-part.md",
                    &A_RECORD.replace("Something.", "Changed [mod.rs](crates/voxel-model/src/matter/mod.rs)."),
                ),
                parse(
                    "PATCH-102-names-a-word.md",
                    &A_RECORD.replace("PATCH-101", "PATCH-102").replace("Something.", "The `voxel-model` crate is a word here."),
                ),
            ],
        };
        let coverage = set.coverage(&[Crate::Model, Crate::Noise]);
        assert!(coverage.measured);
        assert_eq!(coverage.parts.len(), 2);
        assert_eq!(coverage.parts[0].name, "voxel-model");
        assert_eq!(coverage.parts[0].records, vec!["PATCH-101"]);
        assert!(!coverage.parts[1].covered(), "{} claims coverage", coverage.parts[1].name);
        assert_eq!(coverage.uncovered().len(), 1);
        // The word-only record names no path at all, and the report says so rather than counting it.
        assert_eq!(coverage.unnamed_records, vec!["PATCH-102"]);
        let report = coverage.describe();
        assert!(report.contains("patch-coverage-parts\t2"), "{report}");
        assert!(report.contains("patch-coverage-covered\t1"), "{report}");
        assert!(report.contains("patch-part\tvoxel-noise\tcrates/voxel-noise\tuncovered"), "{report}");
        assert!(report.contains("patch-uncovered\tvoxel-noise"), "{report}");
        assert!(report.contains("patch-unnamed-records\t1\tPATCH-102"), "{report}");
    }

    #[test]
    fn a_set_that_did_not_read_reports_no_coverage_rather_than_an_empty_tree() {
        let coverage = PatchSet::unmeasured("no directory").coverage(&[Crate::Model]);
        assert!(!coverage.measured);
        assert!(coverage.parts.is_empty());
        let report = coverage.describe();
        assert!(report.contains("patch-coverage\tnot-measured"), "{report}");
        assert!(!report.contains("patch-coverage-parts"), "{report}");
        assert!(!report.contains("patch-part"), "{report}");
    }

    #[test]
    fn a_directory_the_record_names_places_the_files_under_it() {
        let root = a_tree_of_files("by-directory");
        let mut set = PatchSet {
            measured: true,
            why_unmeasured: None,
            records: vec![parse("PATCH-101-by-directory.md", A_RECORD)],
        };
        set.records[0].named_paths = vec!["crates/placed/src".to_string()];
        let placement = placement(&set, &root);
        let code = placement.kind(Kind::Code).expect("code is measured");
        assert_eq!(code.placed_files, 2, "both files are under the named directory");
        assert_eq!(code.unplaced.len(), 0, "{:?}", code.unplaced);
        cleanup(&root);
    }

    #[test]
    fn placement_counts_both_ways_and_keeps_a_generated_file_out_of_the_work_list() {
        let root = a_tree_of_files("counts");
        let set = PatchSet {
            measured: true,
            why_unmeasured: None,
            records: vec![{
                let mut record = parse("PATCH-101-one-file.md", A_RECORD);
                record.named_paths = vec!["crates/placed/src/one.rs".to_string()];
                record
            }],
        };
        let placement = placement(&set, &root);
        assert!(placement.measured);
        // Three kinds, and each counts every file it has.
        assert_eq!(Kind::ALL.len(), 3);
        assert_eq!(placement.kinds.len(), 3);
        let code = placement.kind(Kind::Code).expect("code");
        // Three `.rs` files: one named, one not, and one that is a **generator's product** — which
        // is a code file placed by the thing that writes it, not a hole in the work list.
        assert_eq!((code.files, code.lines), (3, 12));
        assert_eq!((code.generated_files, code.generated_lines), (1, 4));
        assert_eq!((code.placed_files, code.placed_lines), (1, 3));
        assert_eq!((code.unplaced_files(), code.unplaced_lines()), (1, 5));
        assert_eq!(code.unplaced[0].path, "crates/placed/src/two.rs");
        assert!(
            !code
                .unplaced
                .iter()
                .any(|file| file.path == GENERATED[0].0),
            "a generated file reached the work list"
        );
        let shader = placement.kind(Kind::Shader).expect("shader");
        assert_eq!((shader.files, shader.lines), (1, 2));
        assert_eq!(shader.unplaced_files(), 1);
        let document = placement.kind(Kind::Document).expect("document");
        assert_eq!((document.files, document.lines), (1, 1));
        assert_eq!(document.unplaced_files(), 1);
        // **Every file and every line is in exactly one of the three counts**, which is the
        // invariant that makes the reading trustworthy rather than merely printed.
        for found in &placement.kinds {
            assert_eq!(
                found.files,
                found.generated_files + found.placed_files + found.unplaced_files()
            );
            assert_eq!(
                found.lines,
                found.generated_lines + found.placed_lines + found.unplaced_lines()
            );
        }
        let report = placement.describe();
        assert!(report.contains("placement-code\tfiles\t3\tlines\t12"), "{report}");
        assert!(
            report.contains("placement-unplaced\tcode\tcrates/placed/src/two.rs\t5"),
            "{report}"
        );
        assert!(report.contains("placement-generated\t"), "{report}");
        cleanup(&root);
    }

    #[test]
    fn an_unmeasured_set_prints_no_placement() {
        let placement = placement(&PatchSet::unmeasured("no directory"), Path::new("nowhere"));
        assert!(!placement.measured);
        let report = placement.describe();
        assert!(report.contains("placement\tnot-measured"), "{report}");
        assert!(!report.contains("placement-code"), "{report}");
    }

    #[test]
    fn the_real_tree_is_placed_against_the_real_records() {
        // The reading this instrument exists to print, on the real tree, with the invariant that
        // makes it trustworthy: every file is counted in exactly one of the three ways.
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
        let set = read_dir(&root.join("patches"));
        let placement = placement(&set, &root);
        assert!(placement.measured, "{}", placement.why_unmeasured.clone().unwrap_or_default());
        assert_eq!(placement.kinds.len(), Kind::ALL.len());
        for found in &placement.kinds {
            assert_eq!(
                found.files,
                found.generated_files + found.placed_files + found.unplaced_files(),
                "{}: every file is generated, placed or unplaced",
                found.kind.slug()
            );
            assert_eq!(
                found.lines,
                found.generated_lines + found.placed_lines + found.unplaced_lines(),
                "{}: and every line is in one of them",
                found.kind.slug()
            );
            for file in &found.unplaced {
                assert!(file.records.is_empty(), "{} is placed", file.path);
            }
        }
        println!("{}", placement.describe());
    }

    /// A tree with one of each kind, one generated file, and two code files so a directory and a
    /// single file can both be named. Nothing here is a mock: the walk reads real files.
    ///
    /// **Named per test, like `space.rs`'s fixture, because the tests run concurrently.** One
    /// shared `placement-<pid>` directory was a race: two tests each call this, so one removes the
    /// tree while the other is walking it, and `placed_files` comes back short by however much of
    /// the other test's rewrite landed mid-walk — which is exactly what happened, intermittently,
    /// before the name was added.
    fn a_tree_of_files(name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("placement-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let write = |relative: &str, lines: usize| {
            let path = root.join(relative);
            std::fs::create_dir_all(path.parent().expect("a parent")).expect("create");
            let body: String = (0..lines).map(|n| format!("line {n}\n")).collect();
            std::fs::write(path, body).expect("write");
        };
        write("crates/placed/src/one.rs", 3);
        write("crates/placed/src/two.rs", 5);
        write("crates/placed/src/a-shader.wgsl", 2);
        write("docs/a-document.md", 1);
        write(GENERATED[0].0, 4);
        root
    }

    fn cleanup(root: &Path) {
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn the_real_patch_set_is_joined_against_the_real_parts() {
        // The reading this join exists to print, measured on the tree rather than asserted as a
        // count: which parts a record names, which records name nothing here, and that every id a
        // part claims is an id the set actually carries.
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
        let set = read_dir(&root.join("patches"));
        let coverage = set.coverage(&Crate::ALL);
        assert!(coverage.measured, "{}", coverage.why_unmeasured.clone().unwrap_or_default());
        assert_eq!(coverage.parts.len(), Crate::ALL.len());
        for part in &coverage.parts {
            for id in &part.records {
                assert!(
                    set.records.iter().any(|record| record.id == *id),
                    "{id} is claimed by {} and is in no record",
                    part.name
                );
            }
        }
        // A join that covered nothing would pass every assertion above while measuring nothing.
        assert!(
            coverage.parts.iter().any(|part| part.covered()),
            "no part is covered, so this join is measuring the wrong strings: {:#?}",
            coverage
        );
        println!("{}", coverage.describe());
    }

    #[test]
    fn the_patches_directory_reads_and_every_dependency_resolves() {
        // The backwalk's first fact, measured on the real set rather than asserted here.
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
        let set = read_dir(&root.join("patches"));
        assert!(set.measured, "{}", set.why_unmeasured.clone().unwrap_or_default());
        assert!(set.records.len() >= 16, "{} records", set.records.len());
        assert!(set.duplicated().is_empty(), "{:?}", set.duplicated());
        assert!(set.dangling().is_empty(), "{:?}", set.dangling());
        assert!(
            set.records.iter().all(|record| record.id.starts_with("PATCH-")),
            "every record names itself PATCH-NNN: {:#?}",
            set.records.iter().map(|record| record.id.clone()).collect::<Vec<_>>()
        );
        println!("{}", set.describe());
        for record in &set.records {
            if !record.complete() {
                println!("{} {:?}", record.id, record.problems);
            }
        }
    }
}
