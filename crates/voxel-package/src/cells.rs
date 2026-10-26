//! **The package's cell declaration** — `package.cells`, the header that says which drawing of the
//! shell a display package is a package of.
//!
//! # Why this is here and not in the client
//!
//! It is the format's own grammar, and it was the declaration the format did not own: the
//! constants, the refusal and the reader lived in `voxel_client::shell::cells`, so the GPU-free
//! readers (`voxel-layers`, `voxel-census`) could not see a header the format emits. The raster that
//! *draws* version 1 stays in the client, where the UI crate is; it re-exports this module, so every
//! caller keeps its path. **The split is by question, not by convenience**: *what may a package
//! declare, and what is refused with which file and line* is the format's; *what version 1 looks
//! like on screen* is the shell's.
//!
//! A package has to be able to say which drawing of the shell it is a package of, and the recipe
//! grammar is closed (`R-64.1`: a document that requires a field the schema does not have is a
//! refusal, not an extension). So the version is a **sibling artifact**, the same shape
//! `shploingus.layer/1` has, and the file is two lines:
//!
//! ```text
//! shploingus.cells/1
//! cells<TAB>0|1
//! ```
//!
//! **Two versions, and they are not the same question.**
//!
//! * The **header** says which declaration grammar the file is written in. This build reads
//!   `shploingus.cells/1`; `shploingus.cells/0` is refused by file and line
//!   ([`CellRefusal::OldDeclaration`]) rather than read under the newer rules.
//! * The **`cells` line** says which *drawing* the package asks for. `0` is the old drawing (the
//!   composed elements, unchanged) and `1` is the Holo Dark raster. **`cells 0` is how the old
//!   version is brought back in** — one line in the package, not a rebuild of the module.
//!
//! A package with no `package.cells` is a **declared absence**, not a refusal: it predates the
//! declaration, and the shell keeps the version in force. A package that carries one and gets it
//! wrong is refused by name, with the file and line.

/// **Which drawing of the shell's interface a caller is asking for.**
///
/// The raster is a **substitution** for the element drawing (PATCH-071), and a substitution that
/// cannot say which version it is has no way to bring the old one back. So the version is the
/// caller's name for what it wants drawn:
///
/// * [`CellVersion::Elements`] — **version 0**, the shell as it drew before the cells existed: the
///   composed elements are handed on exactly as they were composed, one paint per element and the
///   views' own geometry. This is the old version, and it is kept addressable rather than deleted.
/// * [`CellVersion::Holo`] — **version 1**, this module's 8 × 16 Holo Dark raster.
///
/// A number that is neither is **refused by name** ([`CellRefusal`]) rather than drawn as the
/// nearest guess: an interface drawn at a version the binary does not implement is a picture of a
/// different shell, and silently substituting one for the other is how a version goes missing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellVersion {
    /// Version 0: the elements as composed, with no cell grid (the old drawing).
    Elements,
    /// Version 1: the 8 × 16 Holo Dark cell raster this module is.
    Holo,
}

impl CellVersion {
    /// Every version this shell draws, oldest first. **A version not in this list is not drawn.**
    pub const IMPLEMENTED: [CellVersion; 2] = [CellVersion::Elements, CellVersion::Holo];

    /// The number a caller writes for this version.
    pub fn number(self) -> u16 {
        match self {
            CellVersion::Elements => 0,
            CellVersion::Holo => 1,
        }
    }

    /// The version a number names, or the refusal that says which numbers exist.
    pub fn of(number: u16) -> Result<CellVersion, CellRefusal> {
        match number {
            0 => Ok(CellVersion::Elements),
            1 => Ok(CellVersion::Holo),
            asked => Err(CellRefusal::VersionNotImplemented {
                asked,
                implemented: [CellVersion::Elements.number(), CellVersion::Holo.number()],
            }),
        }
    }
}

/// **A cell drawing this shell does not have**, named so a caller can fix the number rather than
/// guess what happened — or, for a declaration, fix the file and the line it was read from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CellRefusal {
    /// The caller asked for a version outside [`CellVersion::IMPLEMENTED`].
    VersionNotImplemented {
        /// The number the caller wrote.
        asked: u16,
        /// The numbers this shell does draw, oldest first.
        implemented: [u16; 2],
    },
    /// **A package declared the old cell declaration version.** The drawing it names may well be one
    /// this build has (`version 0` is the old drawing); what is refused is the *declaration* — an
    /// old header is not read under a newer grammar, the same rule `shploingus.recipe/3` states.
    OldDeclaration {
        /// The file the header was read from.
        file: std::path::PathBuf,
        /// The one-based line the header sits on.
        line: usize,
        /// The header the package wrote.
        declared: String,
    },
    /// A declaration this build reads, with a line that is not part of its grammar.
    MalformedDeclaration {
        /// The file the line was read from.
        file: std::path::PathBuf,
        /// The one-based line, or 0 when the file itself could not be opened.
        line: usize,
        /// What is wrong, naming the value.
        detail: String,
    },
}

impl std::fmt::Display for CellRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CellRefusal::VersionNotImplemented { asked, implemented } => write!(
                f,
                "cell version {asked} is not one this shell draws; it draws version {} \
                 (the elements as composed, the old drawing) and version {} (Holo Dark cells)",
                implemented[0], implemented[1]
            ),
            CellRefusal::OldDeclaration { file, line, declared } => write!(
                f,
                "{}:{line} declares {declared:?}, an old cell declaration this build does not \
                 read: it reads {DECLARATION_HEADER}, whose `cells` line selects version 0 (the \
                 elements as composed, the old drawing) or version 1 (Holo Dark cells). The \
                 drawing is versioned; the declaration is not converted",
                display_path(file)
            ),
            CellRefusal::MalformedDeclaration { file, line, detail } => {
                write!(f, "{}:{line}: {detail}", display_path(file))
            }
        }
    }
}

impl std::error::Error for CellRefusal {}

/// **The declaration a display package carries: `<folder>/package.cells`.**
///
/// A package has to be able to say *which drawing of the shell it is a package of*, and the recipe
/// grammar is closed (`R-64.1`: a document that requires a field the schema does not have is a
/// refusal, not an extension). So the version is a **sibling artifact**, the same shape
/// `shploingus.layer/1` has, and the file is two lines:
///
/// ```text
/// shploingus.cells/1
/// cells<TAB>0|1
/// ```
///
/// **Two versions, and they are not the same question.**
///
/// * The **header** says which declaration grammar the file is written in. This build reads
///   `shploingus.cells/1`; `shploingus.cells/0` is refused by file and line
///   ([`CellRefusal::OldDeclaration`]) rather than read under the newer rules.
/// * The **`cells` line** says which *drawing* the package asks for. `0` is the old drawing (the
///   composed elements, unchanged) and `1` is the Holo Dark raster. **`cells 0` is how the old
///   version is brought back in** — one line in the package, not a rebuild of the module.
///
/// A package with no `package.cells` is a **declared absence**, not a refusal: it predates the
/// declaration, and the shell keeps the version in force. A package that carries one and gets it
/// wrong is refused by name, with the file and line.
pub const DECLARATION_FILE: &str = "package.cells";
/// The one declaration grammar this build reads.
pub const DECLARATION_HEADER: &str = "shploingus.cells/1";
/// The declaration grammar before it. **Named so the refusal can quote it**, not accepted.
pub const OLD_DECLARATION_HEADER: &str = "shploingus.cells/0";

/// A path as a diagnostic writes it: `/` separators, so the same refusal reads the same on every
/// platform a reader opens it on. A message that changes shape with the OS is not a record.
fn display_path(path: &std::path::Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// A cell version a package declared, and where it said so.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Declaration {
    /// The drawing the package asks for.
    pub version: CellVersion,
    /// The file the `cells` line was read from.
    pub file: std::path::PathBuf,
    /// The one-based line the `cells` value sits on.
    pub line: usize,
}

/// **Read a display package's cell declaration.**
///
/// `Ok(None)` is a package that carries no `package.cells` — a declared absence, so the caller keeps
/// the version in force. `Err` is a declaration that was written and cannot be read, with the file
/// and the line in every variant. Neither case is a silent substitution.
///
/// **The caller is mounting, and only mounting.** The mount reads the folder it is about to adopt,
/// so a package that arrives while the shell runs is refused or selects its drawing by the same act
/// that adopts its arrangement. There is no separate resolver a boot path could call instead — that
/// was the hole: a declaration read at boot was never read for a package that arrived later.
pub fn declare(folder: &std::path::Path) -> Result<Option<Declaration>, CellRefusal> {
    let file = folder.join(DECLARATION_FILE);
    let text = match std::fs::read_to_string(&file) {
        Ok(text) => text,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(why) => {
            return Err(CellRefusal::MalformedDeclaration {
                file,
                line: 0,
                detail: format!("the declaration could not be read ({why})"),
            })
        }
    };

    let lines: Vec<&str> = text
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .collect();
    let header = lines.first().copied().unwrap_or_default();
    if header == OLD_DECLARATION_HEADER {
        return Err(CellRefusal::OldDeclaration {
            file,
            line: 1,
            declared: header.to_string(),
        });
    }
    if header != DECLARATION_HEADER {
        return Err(CellRefusal::MalformedDeclaration {
            file,
            line: 1,
            detail: format!(
                "expected `{DECLARATION_HEADER}`, found {header:?}; a cell declaration is one \
                 header and one `cells` line"
            ),
        });
    }

    let mut version: Option<Declaration> = None;
    for (index, line) in lines.iter().enumerate().skip(1) {
        let number = index + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = trimmed.split('\t').collect();
        match fields.as_slice() {
            ["cells", value] => {
                if version.is_some() {
                    return Err(CellRefusal::MalformedDeclaration {
                        file,
                        line: number,
                        detail: "`cells` is declared twice; a package asks for one drawing".into(),
                    });
                }
                let asked: u16 = value.parse().map_err(|_| CellRefusal::MalformedDeclaration {
                    file: file.clone(),
                    line: number,
                    detail: format!("`cells` takes a whole number, not {value:?}"),
                })?;
                match CellVersion::of(asked) {
                    Ok(version_) => {
                        version = Some(Declaration { version: version_, file: file.clone(), line: number });
                    }
                    Err(refusal) => {
                        // **The refusal carries the site**, because the number alone does not tell
                        // a reader which package wrote it.
                        return Err(CellRefusal::MalformedDeclaration {
                            file,
                            line: number,
                            detail: refusal.to_string(),
                        });
                    }
                }
            }
            other => {
                return Err(CellRefusal::MalformedDeclaration {
                    file,
                    line: number,
                    detail: format!(
                        "unexpected row {other:?}; a declaration carries one `cells<TAB>0|1` line"
                    ),
                })
            }
        }
    }
    match version {
        Some(declaration) => Ok(Some(declaration)),
        None => Err(CellRefusal::MalformedDeclaration {
            file,
            line: 0,
            detail: "the declaration names no drawing: it carries no `cells` line".into(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder of this test's own: the declaration is a file, so it is exercised as one rather
    /// than as a string the parser is handed.
    fn folder(name: &str) -> std::path::PathBuf {
        // **The child is named exactly as the folder the assertion names**, because a refusal's site
        // is read as path components: `voxel-cells-<pid>/old-cell-declaration` keeps the last two
        // components the same on every machine.
        let root = std::env::temp_dir()
            .join(format!("voxel-cells-{}", std::process::id()))
            .join(name);
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("a folder for the declaration");
        root
    }

    /// **A package that predates the declaration is a declared absence, not a refusal**: the caller
    /// keeps the version in force, and nothing is invented for it.
    #[test]
    fn a_package_without_a_declaration_is_a_declared_absence() {
        let folder = folder("absent");
        assert_eq!(declare(&folder), Ok(None));
        let _ = std::fs::remove_dir_all(&folder);
    }

    /// **The refusal the user asked for**: an old cell declaration is refused by name, with the file
    /// and the line it was written on — never read under the newer grammar and never substituted.
    #[test]
    fn an_old_cell_declaration_is_refused_by_file_and_line() {
        let folder = folder("old-cell-declaration");
        std::fs::write(
            folder.join(DECLARATION_FILE),
            format!("{OLD_DECLARATION_HEADER}
cells	0
"),
        )
        .expect("the declaration is written");
        let refusal = declare(&folder).expect_err("an old header is not this build's to read");
        match &refusal {
            CellRefusal::OldDeclaration { file, line, declared } => {
                assert_eq!(*line, 1);
                assert_eq!(declared, OLD_DECLARATION_HEADER);
                assert!(
                    file.ends_with("old-cell-declaration/package.cells"),
                    "{}",
                    file.display()
                );
            }
            other => panic!("expected an old-declaration refusal, found {other:?}"),
        }
        let message = refusal.to_string();
        assert!(message.contains("package.cells:1"), "the site is missing: {message}");
        assert!(message.contains("shploingus.cells/0"), "the declared version is missing: {message}");
        // **Both halves of the sentence are in the message**: the old drawing is still selectable,
        // and the declaration that selects it is named.
        assert!(message.contains("version 0"), "the old drawing is not named: {message}");
        assert!(message.contains(DECLARATION_HEADER), "the accepted grammar is not named: {message}");
        let _ = std::fs::remove_dir_all(&folder);
    }

    /// A declaration with no drawing named is refused at the file, not defaulted to the newest.
    #[test]
    fn a_declaration_with_no_cells_line_is_refused() {
        let folder = folder("missing-cells-line");
        std::fs::write(folder.join(DECLARATION_FILE), format!("{DECLARATION_HEADER}
"))
            .expect("the declaration is written");
        let message = declare(&folder)
            .expect_err("a declaration names a drawing or it is not a declaration")
            .to_string();
        assert!(message.contains("package.cells"), "the file is missing: {message}");
        assert!(message.contains("`cells`"), "the missing line is not named: {message}");
        let _ = std::fs::remove_dir_all(&folder);
    }
}
