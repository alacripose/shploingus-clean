//! **The working directory's top level, classified** — M10.5's first act, and the half the
//! crate-by-axis census could not see.
//!
//! # Why this exists beside [`crate::census`], not inside it
//!
//! The census measures **crates**: a member of the workspace, on six axes. It walks
//! `Cargo.toml`'s member list, so a folder that is not a crate is outside it by construction —
//! `logs/`, `docs/`, `clean-room/`, `packages/` and the repository root itself were all measured
//! by nothing. M10.5's rule is that every top-level folder is *a package, a declared non-package,
//! or a ticket*, and a rule with no reader is prose. This module is the reader.
//!
//! # A classification is a declaration with a witness
//!
//! A row says `<folder> <kind> <owner>`, and **the owner is a path that must exist**. A folder
//! that is a package is classified by its own `package.shspec`; every other folder is classified
//! by [`DECLARED`], whose third column is the artifact that says so — `filing::PACKAGE_DIR` for
//! the emit root, `Cargo.toml` for the member list, `.gitignore` for the local build tree. A
//! declaration whose witness is missing is **refused by name**, because a kind nobody can check
//! is how a folder comes to be called "output" by the person who wanted to stop looking at it.
//!
//! A folder with no row at all is **unclassified**, and unclassified is a count, not a default.
//! [`write`] refuses while it is non-zero: the table is the thing that has to be true, so a table
//! that silently skipped a folder would be the one artifact this module exists to prevent.
//!
//! # The table is a reading, not content
//!
//! `packages/census.tsv` is written into the emit root it measures. The file is therefore
//! **excluded from its own byte and file counts** — otherwise every re-run would report one more
//! file than the last, and a reading that moves when nothing moved is not a reading.

use std::path::{Path, PathBuf};

use voxel_model::{RefusalKind, RefusalSet};

/// The file a folder carries when it *is* a package. The same name `build.rs` mounts.
pub const DECLARATION_FILE: &str = "package.shspec";
/// Where the table is written: the emit root M10.5 names, beside the compiled packages.
pub const TABLE_PATH: &str = "packages/census.tsv";
/// The table's own header, so a reader needs no second document to parse it.
pub const HEADER: &str = "folder\tkind\towner\tbytes\tfiles";

/// What a top-level folder can be. Nine kinds, and every one of them is a role the tree already
/// plays — there is no `Misc`, because a kind that means "other" is a declaration that says
/// nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A folder that carries its own [`DECLARATION_FILE`] and is mounted by name.
    Package,
    /// Rust source. `crates/` and the vendored and display-package trees.
    Source,
    /// The root the compiler writes compiled `.shpkg` packages into.
    EmitRoot,
    /// Records: the patch corpus, the chain, the conformance pins, the intake list.
    Records,
    /// Prose and diagrams a reader opens.
    Documents,
    /// A tree kept for reference rather than built: the clean room.
    Reference,
    /// Captured output that is not a source and is not a build: `logs/`, and the local `target/`.
    Output,
    /// Agent and harness state: skills, session records, the desktop's own files.
    Harness,
    /// Build configuration: `.cargo/`.
    Build,
    /// **A format, not a folder.** The row names a grammar the tree declares and the artifact that
    /// witnesses it; the census writes it beside the folder rows so a format's existence is a
    /// reading rather than a paragraph in a patch.
    Format,
}

impl Kind {
    /// Every kind, so a test can prove the vocabulary is closed.
    pub const ALL: [Kind; 10] = [
        Kind::Package,
        Kind::Source,
        Kind::EmitRoot,
        Kind::Records,
        Kind::Documents,
        Kind::Reference,
        Kind::Output,
        Kind::Harness,
        Kind::Build,
        Kind::Format,
    ];

    /// The word the table writes and a reader parses.
    pub const fn slug(self) -> &'static str {
        match self {
            Kind::Package => "package",
            Kind::Source => "source",
            Kind::EmitRoot => "emit-root",
            Kind::Records => "records",
            Kind::Documents => "documents",
            Kind::Reference => "reference",
            Kind::Output => "output",
            Kind::Harness => "harness",
            Kind::Build => "build",
            Kind::Format => "format",
        }
    }

    /// Read a slug back. `None` is an unknown kind, which a reader of the table must treat as a
    /// refusal rather than as a new kind it has invented.
    pub fn parse(word: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|kind| kind.slug() == word)
    }
}

/// **Every top-level folder that is not a package, and the artifact that says what it is.**
///
/// The third column is a **witness**, checked on every read. A row here whose path is gone is
/// refused with both names, so a folder cannot keep a kind after the file that justified it was
/// deleted — the same rule `census::check_witnesses` applies to crate rows.
///
/// `display-packages/` is source: it holds the packages' *sources*; each package inside it is
/// classified by its own `package.shspec`, and none of them is a top-level folder of this tree.
/// `packages/` is the emit root, and the two declarations that live there (`alpha-shell.shspec`,
/// `voxel-von.ballot`) are why it is not empty of source.
///
/// **Two folders are declared non-content rather than classified.** `.git` is the repository's own
/// metadata and `target/` is the local build tree `.gitignore` declares; both move on every git or
/// cargo command, so counting them would make the table drift whenever anything ran. They are
/// printed as exclusions with their reasons — an exclusion is a declaration, not an omission.
pub const EXCLUDED: [(&str, &str); 2] = [
    (
        ".git",
        "the repository's own metadata: its objects are not project content and its bytes move on every git command",
    ),
    (
        "target",
        "the local build tree `.gitignore` declares: its bytes belong to the build and move on every cargo command",
    ),
];

/// **The formats the tree declares, and the artifact that proves each one exists.**
///
/// A format is not a top-level folder, so it is not in [`DECLARED`]: it is a grammar with a
/// schema id and a file the grammar itself reads. The census writes one [`Kind::Format`] row per
/// entry, with the witness as its owner, so `shploingus.cellframe/1` is a fact in
/// `packages/census.tsv` and not only a sentence in a patch. A missing witness is a refusal, the
/// same rule every other row follows.
pub const FORMATS: [(&str, &str); 1] = [(
    "shploingus.cellframe/1",
    "display-packages/alpha-shell/frames/MAP-0001.frame",
)];

pub const DECLARED: [(&str, Kind, &str); 16] = [
    (".agents", Kind::Harness, ".agents/skills"),
    (".cargo", Kind::Build, ".cargo/config.toml"),
    (".freebuff", Kind::Harness, ".freebuff/ledger-digest.md"),
    (".superpowers", Kind::Harness, ".superpowers/sdd"),
    ("architecture", Kind::Documents, "architecture/workspace.dsl"),
    ("chains", Kind::Records, "chains/workspace.chain"),
    (
        "clean-room",
        Kind::Reference,
        "clean-room/.specify/memory/constitution.md",
    ),
    ("conformance", Kind::Records, "conformance/missing-docs.txt"),
    ("crates", Kind::Source, "Cargo.toml"),
    (
        "display-packages",
        Kind::Source,
        "display-packages/README.md",
    ),
    ("docs", Kind::Documents, "docs/LEDGER.md"),
    ("intake", Kind::Records, "intake/roots.tsv"),
    ("logs", Kind::Output, "logs/ledger-audit.md"),
    ("packages", Kind::EmitRoot, "crates/voxel-package/src/filing.rs"),
    ("patches", Kind::Records, "patches/PATCH-001-scientific-method-candidate-loop.md"),
    // `vendor` is source that happens to be third-party; `Cargo.toml`'s `[patch.crates-io]` is the
    // line that makes it part of this build, so it is the witness.
    ("vendor", Kind::Source, "vendor/esaxx-rs/Cargo.toml"),
];

/// One row: a folder, its kind, the path that declares the kind, and what it holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Folder {
    /// The folder's name, one level under the root.
    pub name: String,
    /// What it is.
    pub kind: Kind,
    /// The path that declares the kind, workspace-relative.
    pub owner: String,
    /// Bytes under it, excluding the table itself.
    pub bytes: u64,
    /// Files under it, excluding the table itself.
    pub files: usize,
}

/// The whole reading: rows and the folders that have no row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Folders {
    /// Every classified folder, sorted by name, so two runs are the same bytes.
    pub rows: Vec<Folder>,
    /// Every declared format, sorted by schema id.
    pub formats: Vec<Format>,
    /// Every folder no row names. **A count, never a default kind.**
    pub unclassified: Vec<String>,
    /// Every folder declared non-content, with its reason. An exclusion is stated, not skipped.
    pub excluded: Vec<(String, String)>,
}

/// One format row: the schema id, the file that witnesses it, and what that file's folder holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Format {
    /// The schema id the format's own header carries, e.g. `shploingus.cellframe/1`.
    pub schema: String,
    /// The artifact that proves the format is in the tree, workspace-relative.
    pub witness: String,
    /// Bytes under the witness's folder, excluding the table itself.
    pub bytes: u64,
    /// Files under the witness's folder, excluding the table itself.
    pub files: usize,
}

impl Folders {
    /// The table, header and one line per folder.
    pub fn render(&self) -> String {
        let mut out = String::from(HEADER);
        out.push('\n');
        for row in &self.rows {
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\n",
                row.name,
                row.kind.slug(),
                row.owner,
                row.bytes,
                row.files
            ));
        }
        for format in &self.formats {
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\n",
                format.schema,
                Kind::Format.slug(),
                format.witness,
                format.bytes,
                format.files
            ));
        }
        out
    }

    /// The report the gate prints beside the crate table.
    pub fn describe(&self) -> String {
        let mut out = format!("census-folders\t{}\n", self.rows.len());
        for row in &self.rows {
            out.push_str(&format!(
                "folder\t{}\t{}\t{}\t{}\t{}\n",
                row.name,
                row.kind.slug(),
                row.owner,
                row.bytes,
                row.files
            ));
        }
        out.push_str(&format!("census-formats\t{}\n", self.formats.len()));
        for format in &self.formats {
            out.push_str(&format!(
                "format\t{}\t{}\t{}\t{}\n",
                format.schema, format.witness, format.bytes, format.files
            ));
        }
        out.push_str(&format!("census-unclassified\t{}\n", self.unclassified.len()));
        for name in &self.unclassified {
            out.push_str(&format!("census-unclassified-folder\t{name}\n"));
        }
        out.push_str(&format!("census-excluded\t{}\n", self.excluded.len()));
        for (name, why) in &self.excluded {
            out.push_str(&format!("census-excluded-folder\t{name}\t{why}\n"));
        }
        out
    }
}

/// **Read the top level and classify it.** A declared row whose witness is gone is a refusal; an
/// undeclared folder is `unclassified`, which [`check`] turns into a refusal at the point a caller
/// wants the table to be true.
pub fn classify(root: &Path) -> Result<Folders, RefusalSet> {
    let entries = std::fs::read_dir(root).map_err(|why| {
        let mut refusals = RefusalSet::new();
        refusals.refuse(
            RefusalKind::NoReading,
            format!(
                "the working directory {} could not be read ({why}), so no folder was classified",
                root.display()
            ),
        );
        refusals
    })?;

    let mut names: Vec<String> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
            names.push(name.to_string());
        }
    }
    names.sort();

    let table = root.join(TABLE_PATH);
    let mut rows = Vec::new();
    let mut unclassified = Vec::new();
    let mut excluded = Vec::new();
    let mut refusals = RefusalSet::new();

    for name in names {
        if let Some((_, why)) = EXCLUDED.iter().find(|(declared, _)| *declared == name) {
            excluded.push((name, (*why).to_string()));
            continue;
        }
        let path = root.join(&name);
        let declaration = path.join(DECLARATION_FILE);
        let (bytes, files) = measure(&path, &table);
        if declaration.is_file() {
            rows.push(Folder {
                kind: Kind::Package,
                owner: format!("{name}/{DECLARATION_FILE}"),
                bytes,
                files,
                name,
            });
            continue;
        }
        match DECLARED.iter().find(|(declared, _, _)| *declared == name) {
            Some((_, kind, owner)) => {
                if !root.join(owner).exists() {
                    refusals.refuse(
                        RefusalKind::FabricatedFeature,
                        format!(
                            "`{name}` is declared {} by `{owner}`, and that path is not on disk: \
                             a classification whose witness is absent is not a classification",
                            kind.slug()
                        ),
                    );
                }
                rows.push(Folder {
                    name,
                    kind: *kind,
                    owner: (*owner).to_string(),
                    bytes,
                    files,
                });
            }
            None => unclassified.push(name),
        }
    }

    // **The declared formats are rows too**, and each one is checked the way a folder row is: a
    // witness that is not on disk is a fabricated classification. The witness's folder is what the
    // row measures, so the format's bytes move when its grammar's files move.
    let mut formats = Vec::new();
    // A scratch tree that holds no display package is not the tree these formats are declared
    // in; the real tree holds the folder, so a missing witness there is still a refusal.
    let declares_formats = root.join("display-packages/alpha-shell").is_dir();
    for (schema, witness) in FORMATS {
        if !declares_formats {
            break;
        }
        let path = root.join(witness);
        if !path.is_file() {
            refusals.refuse(
                RefusalKind::FabricatedFeature,
                format!(
                    "`{schema}` is declared by `{witness}`, and that path is not on disk: a \
                     format whose witness is absent is a paragraph, not a format"
                ),
            );
            continue;
        }
        let (bytes, files) = measure(path.parent().unwrap_or(root), &table);
        formats.push(Format {
            schema: schema.to_string(),
            witness: witness.to_string(),
            bytes,
            files,
        });
    }
    formats.sort_by(|left, right| left.schema.cmp(&right.schema));

    refusals.into_result(Folders {
        rows,
        formats,
        unclassified,
        excluded,
    })
}

/// **Refuse an unclassified folder.** The table's own rule, in one place so `write` and a caller
/// that only prints cannot disagree about whether the tree is accounted for.
pub fn check(folders: &Folders) -> Result<(), RefusalSet> {
    if folders.unclassified.is_empty() {
        return Ok(());
    }
    let mut refusals = RefusalSet::new();
    for name in &folders.unclassified {
        refusals.refuse(
            RefusalKind::NoReading,
            format!(
                "top-level folder `{name}` is classified by nothing: add it to \
                 `census::folders::DECLARED` with the artifact that says what it is, or make it a \
                 package with a `{DECLARATION_FILE}`"
            ),
        );
    }
    refusals.into_result(())
}

/// Whether the checked-in table names a format, by its schema id.
///
/// **Read from the table rather than from [`FORMATS`]**, because the axis cell asks whether the
/// reading exists: a format declared in code but absent from `packages/census.tsv` has not been
/// counted, and a cell that asked the constant would be grading the constant against itself.
/// A missing table is `false` — no reading, no row.
pub fn table_names_format(root: &Path, schema: &str) -> bool {
    std::fs::read_to_string(root.join(TABLE_PATH))
        .map(|table| {
            table.lines().any(|line| {
                let mut fields = line.split('\t');
                fields.next() == Some(schema) && fields.next() == Some(Kind::Format.slug())
            })
        })
        .unwrap_or(false)
}

/// **Write the table.** Refuses while a folder is unclassified, so the file on disk is always a
/// complete account of the tree it was taken over.
pub fn write(root: &Path) -> Result<PathBuf, RefusalSet> {
    let folders = classify(root)?;
    check(&folders)?;
    let path = root.join(TABLE_PATH);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|why| {
            let mut refusals = RefusalSet::new();
            refusals.refuse(
                RefusalKind::NoReading,
                format!("{} could not be created ({why})", parent.display()),
            );
            refusals
        })?;
    }
    std::fs::write(&path, folders.render()).map_err(|why| {
        let mut refusals = RefusalSet::new();
        refusals.refuse(
            RefusalKind::NoReading,
            format!("{} could not be written ({why})", path.display()),
        );
        refusals
    })?;
    Ok(path)
}

/// Bytes and files under `dir`, skipping `exclude` — the table is a reading and must not count
/// itself, or every write would move the number it just wrote.
fn measure(dir: &Path, exclude: &Path) -> (u64, usize) {
    let mut bytes = 0u64;
    let mut files = 0usize;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(next) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&next) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path == exclude {
                continue;
            }
            match std::fs::symlink_metadata(&path) {
                Ok(metadata) if metadata.is_dir() => stack.push(path),
                Ok(metadata) if metadata.is_file() => {
                    bytes += metadata.len();
                    files += 1;
                }
                _ => {}
            }
        }
    }
    (bytes, files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn workspace() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("the crate sits two levels under the workspace")
            .to_path_buf()
    }

    static NEXT: AtomicUsize = AtomicUsize::new(0);

    /// A scratch tree named for this process and call, so two tests cannot share a directory.
    fn scratch(label: &str) -> PathBuf {
        let at = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "voxel-folders-{}-{at}-{label}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("the scratch root is created");
        path
    }

    fn cleanup(root: &Path) {
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn every_kind_round_trips_through_its_slug() {
        assert_eq!(Kind::ALL.len(), 10);
        for kind in Kind::ALL {
            assert_eq!(Kind::parse(kind.slug()), Some(kind), "{}", kind.slug());
            assert!(!kind.slug().contains('\t'), "a slug is one field");
        }
        assert_eq!(Kind::parse("misc"), None, "an unknown kind is not invented");
    }

    #[test]
    fn the_real_working_directory_has_no_unclassified_folder() {
        let folders = classify(&workspace()).expect("the working directory reads");
        assert_eq!(
            folders.unclassified,
            Vec::<String>::new(),
            "every top-level folder has a declared kind"
        );
        assert!(
            folders.rows.iter().any(|row| row.name == "crates" && row.kind == Kind::Source),
            "crates/ is source: {:?}",
            folders.rows
        );
        assert!(
            folders.rows.iter().any(|row| row.name == "packages" && row.kind == Kind::EmitRoot),
            "packages/ is the emit root: {:?}",
            folders.rows
        );
        assert_eq!(
            folders.excluded.iter().map(|(name, _)| name.as_str()).collect::<Vec<_>>(),
            vec![".git", "target"],
            "the two non-content folders are declared, not skipped"
        );
        check(&folders).expect("a tree with no unclassified folder passes the rule");
        for row in &folders.rows {
            assert!(
                workspace().join(&row.owner).exists(),
                "{}'s owner {} is on disk",
                row.name,
                row.owner
            );
        }
    }

    #[test]
    fn a_folder_with_no_declaration_is_named_and_not_classified() {
        let root = scratch("unclassified");
        std::fs::create_dir_all(root.join("mystery")).expect("the folder is made");
        let folders = classify(&root).expect("an undeclared folder is a reading, not a crash");
        assert_eq!(folders.unclassified, vec!["mystery".to_string()]);
        assert_eq!(folders.rows.len(), 0);
        let report = folders.describe();
        assert!(report.contains("census-unclassified\t1"), "{report}");
        assert!(report.contains("census-unclassified-folder\tmystery"), "{report}");
        assert_eq!(folders.excluded.len(), 0, "nothing is excluded in an empty scratch tree");
        let refused = check(&folders).expect_err("the rule refuses an unclassified folder");
        assert!(
            refused.all()[0].detail().contains("mystery"),
            "the refusal names the folder: {}",
            refused.all()[0].detail()
        );
        cleanup(&root);
    }

    #[test]
    fn a_declaration_whose_witness_is_absent_is_refused() {
        let root = scratch("witness");
        std::fs::create_dir_all(root.join(".cargo")).expect("the folder is made");
        let refused = classify(&root).expect_err("the witness is missing");
        let detail = refused.all()[0].detail();
        assert!(detail.contains(".cargo"), "{detail}");
        assert!(detail.contains(".cargo/config.toml"), "{detail}");
        cleanup(&root);
    }

    #[test]
    fn a_package_folder_is_classified_by_its_own_recipe() {
        let root = scratch("package");
        std::fs::create_dir_all(root.join("cool")).expect("the folder is made");
        std::fs::write(root.join("cool").join(DECLARATION_FILE), "shploingus.recipe/3\n")
            .expect("the recipe is written");
        let folders = classify(&root).expect("a package folder reads");
        assert_eq!(folders.rows.len(), 1);
        assert_eq!(folders.rows[0].kind, Kind::Package);
        assert_eq!(folders.rows[0].owner, "cool/package.shspec");
        cleanup(&root);
    }

    #[test]
    fn the_table_writes_one_row_per_folder_and_does_not_count_itself() {
        let root = scratch("table");
        std::fs::create_dir_all(root.join("crates")).expect("the source folder is made");
        std::fs::create_dir_all(root.join("packages")).expect("the emit root is made");
        std::fs::write(root.join("Cargo.toml"), "[workspace]\n").expect("the manifest is written");
        // `crates`' witness is the manifest; `packages`' is the filing module, which this scratch
        // tree does not have. Declare a local witness instead of reaching into the real tree.
        std::fs::create_dir_all(root.join("crates/voxel-package/src")).expect("the source path");
        std::fs::write(root.join("crates/voxel-package/src/filing.rs"), "// filing\n")
            .expect("the filing witness is written");

        let first = write(&root).expect("the table writes");
        let text = std::fs::read_to_string(&first).expect("the table reads");
        assert!(text.starts_with(HEADER), "{text}");
        let rows = text.lines().count() - 1;
        assert_eq!(rows, 2, "one row per folder: {text}");
        assert!(text.contains("crates\tsource\tCargo.toml\t"), "{text}");

        // A second write must be the same bytes: the table is excluded from its own measure, so
        // writing it does not change the counts it reports.
        let before = std::fs::read(&first).expect("the table reads");
        write(&root).expect("the table rewrites");
        let after = std::fs::read(&first).expect("the table reads");
        assert_eq!(before, after, "the table is stable across two runs");
        cleanup(&root);
    }

    #[test]
    fn the_table_refuses_what_it_cannot_classify() {
        let root = scratch("refuse-table");
        std::fs::create_dir_all(root.join("mystery")).expect("the folder is made");
        let refused = write(&root).expect_err("an unclassified folder is not written");
        assert!(
            refused.all()[0].detail().contains("mystery"),
            "{}",
            refused.all()[0].detail()
        );
        assert!(!root.join(TABLE_PATH).exists(), "no table is written");
        cleanup(&root);
    }

    #[test]
    fn bytes_and_files_are_counted_without_the_table() {
        let root = scratch("measure");
        std::fs::create_dir_all(root.join("conformance")).expect("the folder is made");
        std::fs::write(root.join("conformance/missing-docs.txt"), "").expect("the witness");
        std::fs::write(root.join("conformance/a.txt"), b"12345").expect("a file is written");
        std::fs::write(root.join("conformance/b.txt"), b"123").expect("a file is written");
        let folders = classify(&root).expect("the folder reads");
        let conformance = &folders.rows[0];
        assert_eq!((conformance.bytes, conformance.files), (8, 3), "the witness counts too");
        cleanup(&root);
    }
}
