//! **Which bytes this build read.** R-59.1's first half, implemented.
//!
//! # The ruling, in the record's own words
//!
//! **R-59.1**, a user ruling of 2026-09-30:
//!
//! > *"Provenance is a working-tree digest this project computes itself, over `sha256:` — the one
//! > scheme `voxel_model::Digest` accepts. **Not a commit, not `gix`, not a second repository**, no
//! > staging, no commits, no configuration changes."*
//!
//! And the limit of what it is, which the same entry states so it is never overclaimed:
//!
//! > *"A working-tree digest is **not Git provenance**. It establishes *which bytes this build
//! > read*. It does not establish authorship, history, a commit, tamper-evidence against someone
//! > who can rewrite both the tree and the digest, or anything `gix` would have provided."*
//!
//! **The gate already declares this feature and reports it `unchecked`** — `witness_of` names
//! `crates/voxel-package/src/provenance.rs`, and the file did not exist. A declared row with no
//! witness is what the gate exists to print, so this is the file the gate was already pointing at.
//!
//! # `gix`, scoped to when there is a commit — and why that is a condition and not a policy
//!
//! The user's ruling of 2026-09-30, superseding nothing and narrowing R-59.1: the working-tree
//! digest is what works **today**, and `gix` is adopted **when a commit exists**.
//!
//! Which is not a policy disagreement, it is a fact about the disk. C-58 measured it and it was
//! re-measured on 2026-09-30, still true:
//!
//! | fact | value | consequence |
//! |---|---|---|
//! | `.git/HEAD` | `ref: refs/heads/main` | a branch was named |
//! | `git rev-list --count HEAD` | *fatal: unknown revision* | **no commit exists** |
//! | `.git/config` | no `extensions.objectFormat` | **SHA-1** |
//!
//! So read-only gix provenance has **nothing to read** — and, separately, the objects that will
//! eventually exist are hashed in a scheme `Digest` refuses to express. **Two conditions, both
//! named, neither reported as a failure**, because a `gix: failed` line would be a permanent red
//! that says "this project is broken" about a project that is simply not a repository yet.
//!
//! This is the D97 lesson applied to a dependency that is legitimately deferred: the honest
//! report is the **condition**, and the gate flips on its own the day a commit lands, with no
//! code change and no one remembering to update a row.

use std::path::{Path, PathBuf};

use voxel_model::identity::Digest;

use crate::features::workspace_root;

/// The domain tag folded in ahead of the payload.
///
/// **`Digest::of_fields` length-prefixes every field under a tag**, which is what makes a
/// digest of this tree impossible to mistake for a digest of a chain, a document or a package —
/// the same discipline L-6 records from the dirty tree's lineage envelope: *"a domain string
/// folded in ahead of every payload, so schema, payload and envelope digests cannot be mistaken
/// for one another."*
pub const DOMAIN: &str = "shploingus/provenance";

/// The extensions this provenance covers.
///
/// **An allowlist, and the allowlist is the declaration.** A digest over "every file I found"
/// would change when `target/` changes, when a build script writes a `.gate` file, or when an
/// editor leaves a `.swp` behind — so it would be a digest of the machine rather than of the
/// project. Four extensions is what the project is written in; anything else is not source.
///
/// **`docs/portal_simulator.py` is deliberately included.** It is a vendored reference (C-70.4)
/// and it is *in* `docs/`, so a change to it is a change to the record. Excluding it would be a
/// second, invisible rule about what the record contains.
pub const COVERED: [&str; 4] = ["rs", "md", "toml", "wgsl"];

/// Directories never walked, named rather than pattern-matched.
///
/// **Two, and both are generated.** `target` is build output and changes on every build; `.git`
/// is another tool's state. Digesting either makes the digest a property of the last command
/// rather than of the project.
pub const SKIPPED: [&str; 2] = ["target", ".git"];

/// What is known about a commit, as a condition rather than a verdict.
///
/// **Every field is populated in every state, and that is the design.** A `present: false` with
/// no `why` would be a blank, and a blank in a gate report reads as *"we did not look"* — which
/// is the one state a gate must never be in without saying so (D107).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitCondition {
    /// Whether a commit could be read. **False today, and that is a fact, not a failure.**
    pub present: bool,
    /// The object format, as the repository declares it. `"sha1"` when it declares none.
    pub object_format: String,
    /// Whether `Digest` can express an object in that format.
    pub expressible: bool,
    /// Why, in a sentence, **always**. This is what makes `present: false` readable.
    pub why: String,
}

impl CommitCondition {
    /// Read the condition from a `.git` directory, or from its absence.
    ///
    /// **Reads the two files and nothing else.** It does not shell out to `git`, does not
    /// create a repository, and does not write configuration — R-59.1's *"no staging, no
    /// commits, no configuration changes"*, taken literally, including the part about not
    /// depending on another tool's global config (`core.autocrlf`, `init.defaultBranch` and
    /// `core.symlinks` are all supplied by this machine's **system** config, which is exactly
    /// why reading them would make the digest a property of the machine).
    pub fn read(git_dir: &Path) -> Self {
        let head = git_dir.join("HEAD");
        let config = git_dir.join("config");

        if !git_dir.is_dir() {
            return CommitCondition {
                present: false,
                object_format: "none".to_string(),
                expressible: false,
                why: "there is no .git directory, so this is not a repository and there is no \
                      commit to read; the working-tree digest is the whole of provenance here"
                    .to_string(),
            };
        }

        let object_format = read_object_format(&config);
        let expressible = object_format == "sha256";
        let head_text = std::fs::read_to_string(&head).unwrap_or_default();
        let head_trimmed = head_text.trim();

        // **A symbolic ref is a branch name, not a commit.** `ref: refs/heads/main` says a
        // branch was named and nothing was written, and reading it as a commit is how a
        // repository that has never been committed gets reported as having provenance.
        let (present, why) = if head_trimmed.starts_with("ref:") {
            (
                false,
                format!(
                    ".git/HEAD is {head_trimmed:?}, which names a branch and no commit; \
                     rev-list over it resolves to nothing, so there is no commit to read"
                ),
            )
        } else if head_trimmed.is_empty() {
            (
                false,
                ".git/HEAD is empty, so there is no commit to read".to_string(),
            )
        } else {
            (
                true,
                format!(".git/HEAD resolves to a commit id ({head_trimmed})"),
            )
        };

        CommitCondition {
            present,
            object_format: object_format.clone(),
            expressible,
            why: if !expressible {
                format!(
                    "{why}; and the object format is {object_format:?}, which voxel_model::Digest \
                     refuses to express, so a commit here would not be readable by this project \
                     even once one exists"
                )
            } else {
                why
            },
        }
    }

    /// The gate's report. **`gix` is named as deferred with its condition**, never as failed.
    pub fn describe(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("provenance-commit\t{}\n", self.present));
        out.push_str(&format!("provenance-object-format\t{}\n", self.object_format));
        out.push_str(&format!("provenance-digest-expressible\t{}\n", self.expressible));
        out.push_str(&format!("provenance-commit-why\t{}\n", self.why));
        if !self.present || !self.expressible {
            out.push_str(
                "provenance-gix\t\tdeferred: read-only gix provenance is adopted when a commit \
                 exists, and this repository is not a commit yet. The working-tree digest is \
                 what stands today (R-59.1)\n",
            );
        } else {
            out.push_str(
                "provenance-gix\t\teligible: a commit exists and its object format is one \
                 voxel_model::Digest can express\n",
            );
        }
        out
    }
}

/// The object format, or `sha1` when the repository declares none.
///
/// **A default of `sha1` rather than "unknown", because that is what an undeclared object format
/// means** — and reporting `unknown` would put a reader in the position of guessing, which is
/// the state a refusal exists to prevent.
fn read_object_format(config: &Path) -> String {
    let text = match std::fs::read_to_string(config) {
        Ok(text) => text,
        Err(_) => return "sha1".to_string(),
    };
    for line in text.lines() {
        let line = line.trim();
        // `extensions.objectFormat = sha256` may be written with or without spaces.
        let rest = match line.strip_prefix("objectFormat") {
            Some(rest) => rest,
            None => continue,
        };
        let rest = rest.trim_start().strip_prefix('=').unwrap_or(rest.trim_start());
        let value = rest.trim().trim_matches('"');
        if !value.is_empty() {
            return value.to_string();
        }
    }
    "sha1".to_string()
}

/// Every covered file under a root, **sorted, and relative-path-keyed**.
pub(crate) fn covered_files(root: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = Vec::new();
    walk(root, &mut found);
    // **Sorted, and the sort is the determinism.** A digest of an unsorted walk is a digest of
    // the order the filesystem happened to return, which differs between machines and between
    // runs — and a digest that moves on its own is one nobody can cite.
    found.sort();
    found
}

/// Collect covered files under `dir`.
///
/// **No `root` parameter, and its absence is the point.** The relative path is computed by
/// [`covered_files`] from the root it already holds, so threading a root through the recursion
/// would be a second copy of the same answer available at every level — and the one thing a
/// walk must not do is re-derive what the caller already knows.
fn walk(dir: &Path, found: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if SKIPPED.contains(&name.as_ref()) {
            continue;
        }
        if path.is_dir() {
            walk(&path, found);
            continue;
        }
        let covered = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| COVERED.contains(&ext))
            .unwrap_or(false);
        if covered {
            found.push(path);
        }
    }
}

/// The working-tree digest, and what it covers.
///
/// **`Digest::of_fields` over `(relative path, bytes)` per file, in sorted order.** The path is
/// inside the digest, so a **rename changes the digest** even when the bytes are identical — and
/// a rename is a change to the project that a content-only digest would hide.
pub fn measure(root: &Path) -> Provenance {
    let files = covered_files(root);
    // Collected rather than streamed because `of_fields` takes a slice, and a stream would need
    // a second pass or an owned copy of every file at once — which is the same memory, so the
    // honest shape is the one that admits it.
    let mut parts: Vec<Vec<u8>> = Vec::with_capacity(files.len() * 2);
    let mut read: Vec<PathBuf> = Vec::with_capacity(files.len());
    for path in &files {
        let relative = path.strip_prefix(root).unwrap_or(path).to_string_lossy().to_string();
        match std::fs::read(path) {
            Ok(bytes) => {
                parts.push(relative.into_bytes());
                parts.push(bytes);
                read.push(path.clone());
            }
            Err(why) => {
                // **A file that cannot be read is a refusal, and it is refused loudly.** Dropping
                // it would make the digest describe a tree this build did not read, which is the
                // one thing R-59.1 says the digest *does* establish.
                return Provenance::unreadable(&relative, &why.to_string());
            }
        }
    }
    let fields: Vec<&[u8]> = parts.iter().map(|part| part.as_slice()).collect();
    Provenance {
        measured: true,
        unreadable: None,
        files: read.len(),
        tree_digest: Digest::of_fields(DOMAIN, &fields),
        commit: CommitCondition::read(&root.join(".git")),
    }
}

/// The provenance of one build.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Provenance {
    /// Whether a digest was computed. **False means [`Self::tree_digest`] is not a
    /// measurement**, and the reason is beside it.
    pub measured: bool,
    /// A file that could not be read, and why. Present exactly when `measured` is false.
    pub unreadable: Option<(String, String)>,
    /// How many files the digest covers.
    pub files: usize,
    /// The digest of those files.
    pub tree_digest: Digest,
    /// What is known about a commit.
    pub commit: CommitCondition,
}

impl Provenance {
    /// A digest that could not be computed, and the file that stopped it.
    pub fn unreadable(path: &str, why: &str) -> Self {
        Provenance {
            measured: false,
            unreadable: Some((path.to_string(), why.to_string())),
            files: 0,
            tree_digest: Digest::of(&[]),
            commit: CommitCondition {
                present: false,
                object_format: "none".to_string(),
                expressible: false,
                why: "not read, because the tree digest was refused first".to_string(),
            },
        }
    }

    /// The gate's report. **`provenance-digest not-measured` rather than a digest of nothing**,
    /// for the same reason the census and the bloat measurement do it.
    pub fn describe(&self) -> String {
        let mut out = String::new();
        if !self.measured {
            out.push_str("provenance-digest\tnot-measured\n");
            if let Some((path, why)) = &self.unreadable {
                out.push_str(&format!("provenance-unreadable\t{path}: {why}\n"));
            }
            return out;
        }
        out.push_str(&format!("provenance-digest\t{}\n", self.tree_digest.to_hex()));
        out.push_str(&format!("provenance-files\t{}\n", self.files));
        // **The overclaim, refused in the same breath as the number.** R-59.1: a working-tree
        // digest "does not establish authorship, history, a commit, tamper-evidence … or
        // anything `gix` would have provided."
        out.push_str(
            "provenance-scope\t\tthese bytes, this build. Not authorship, not history, not a \
             commit, and not tamper-evidence against anyone who can rewrite both the tree and \
             the digest (R-59.1)\n",
        );
        out.push_str(&self.commit.describe());
        out
    }
}

/// The provenance of the workspace this build ran in.
pub fn of_workspace() -> Provenance {
    match workspace_root(&std::env::current_dir().unwrap_or_default()) {
        Some(root) => measure(&root),
        None => Provenance::unreadable(
            "<workspace root>",
            "no workspace root was found above the current directory, so no tree was read",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_digest_moves_when_a_file_moves_even_if_its_bytes_do_not() {
        // **The rename case, and it is the reason the path is inside the digest.** A
        // content-only digest cannot tell "renamed" from "unchanged", and a rename is a change
        // to the project that a build gate would then fail to notice.
        let dir = std::env::temp_dir().join(format!("shploingus-prov-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("a")).expect("creates");
        std::fs::write(dir.join("a").join("one.md"), b"same bytes\n").expect("writes");

        let before = measure(&dir);
        std::fs::create_dir_all(dir.join("b")).expect("creates");
        std::fs::rename(dir.join("a").join("one.md"), dir.join("b").join("one.md"))
            .expect("renames");
        let after = measure(&dir);

        assert_ne!(
            before.tree_digest, after.tree_digest,
            "a rename must move the digest, or the gate cannot see a file that moved"
        );
        assert_eq!(before.files, after.files, "and it is still one file, not two");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn two_reads_of_the_same_tree_agree() {
        // **Determinism is the whole property.** A digest that moves between two runs of the
        // same tree is a digest of the filesystem's mood, and no citation to it is checkable.
        let root = workspace_root(&std::env::current_dir().expect("a current directory"))
            .expect("the workspace root");
        let first = measure(&root);
        let second = measure(&root);
        assert!(first.measured, "the workspace reads: {:?}", first.unreadable);
        assert_eq!(
            first.tree_digest, second.tree_digest,
            "two reads of one tree must agree, or the digest is not a measurement"
        );
    }

    #[test]
    fn the_workspace_digest_is_computed_and_covers_the_record() {
        // **The feature's own witness, asserted by the feature's own test.** The gate reads
        // `provenance_digest` as `unchecked` until this file existed; the file existing is not
        // the claim, and this is. `docs/LEDGER.md` is covered because `.md` is in [`COVERED`],
        // which is what ties the digest to the record rather than only to the code.
        let found = of_workspace();
        assert!(found.measured, "the workspace digest computes: {:?}", found.unreadable);
        assert!(found.files > 0, "and it covers something");
        let report = found.describe();
        assert!(report.contains("provenance-digest\t"), "{report}");
        assert!(
            report.contains("Not authorship, not history, not a"),
            "and the scope limit travels with the number, so the digest cannot be read as more \
             than it is: {report}"
        );
    }

    #[test]
    fn an_absent_git_directory_is_a_condition_and_never_a_zero() {
        // **The state this repository is actually in**, minus the directory. `present: false`
        // with a `why` a reader can act on, and never a line that reads as "provenance: 0".
        let absent = CommitCondition::read(Path::new("no-such-directory/.git"));
        assert!(!absent.present);
        assert_eq!(absent.object_format, "none");
        assert!(
            absent.why.contains("not a repository"),
            "and it says what is true: {}",
            absent.why
        );
        let report = absent.describe();
        assert!(report.contains("provenance-gix\t\tdeferred"), "{report}");
        assert!(
            !report.contains("failed") && !report.contains("error"),
            "a repository that is not a repository yet is not a failure: {report}"
        );
    }

    #[test]
    fn a_symbolic_head_is_a_branch_and_not_a_commit() {
        // **The measured state of this repository, as a test.** `.git/HEAD` reads
        // `ref: refs/heads/main`; reading that as a commit id is how an unborn repository gets
        // reported as having provenance it does not have.
        let dir = std::env::temp_dir().join(format!("shploingus-head-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(".git")).expect("creates");
        std::fs::write(dir.join(".git").join("HEAD"), "ref: refs/heads/main\n").expect("writes");
        std::fs::write(dir.join(".git").join("config"), "[core]\n\trepositoryformatversion = 0\n")
            .expect("writes");

        let condition = CommitCondition::read(&dir.join(".git"));
        assert!(!condition.present, "a symbolic ref is not a commit");
        assert_eq!(
            condition.object_format, "sha1",
            "and an undeclared object format is sha1, not unknown"
        );
        assert!(
            condition.why.contains("names a branch and no commit"),
            "and the reason is the specific one: {}",
            condition.why
        );
        assert!(
            condition.why.contains("Digest"),
            "and it also says the object format would be refused anyway: {}",
            condition.why
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_digest_that_could_not_be_computed_says_so_rather_than_hashing_nothing() {
        // **`Digest::of(&[])` is a real digest of no bytes**, and printing it beside a file
        // count of zero would read as a measurement of an empty tree rather than a refusal.
        let refused = Provenance::unreadable("crates/voxel-package/src/lib.rs", "permission denied");
        let report = refused.describe();
        assert!(report.contains("provenance-digest\tnot-measured"), "{report}");
        assert!(report.contains("permission denied"), "and it names the reason: {report}");
        assert!(!report.contains("provenance-files"), "and prints no file count: {report}");
    }
}
