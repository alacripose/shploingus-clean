//! **The tree, read once, with every failure kept rather than dropped.**
//!
//! [`Evidence::read`] is infallible on purpose: a source that cannot be read is a *fact about this
//! build*, and the report has to print it rather than stop. Each source therefore returns a
//! [`SourceReading`] — the reading, or the reason there is no reading — and the matrix's cells are
//! built from whichever readings exist.
//!
//! # One reader per question
//!
//! Every read here is a call into the crate that owns the instrument, never a second parser:
//!
//! * the manifest through [`voxel_package::census::workspace_member_packages`], which reads each
//!   member's own `[package] name` — a grouped folder and a package name are not the same string;
//! * the census through [`voxel_package::census::census`], which is the package's own six-axis
//!   reading, pins included;
//! * the prose frame through [`voxel_package::census::check_prose_frame`], which re-measures every
//!   crate instead of trusting the number in the file;
//! * the ladders through [`voxel_model::axis::check_coverage`], which compares each coverage row
//!   against the axis's own tokens.
//!
//! **No file is opened for writing, and nothing here is cached between roots.** The evidence is a
//! projection of a tree as it stands; [`voxel_axis_matrix::Matrix`] already states why a second
//! store beside the chain is refused.

use std::path::{Path, PathBuf};

use voxel_axis_coverage::{check_coverage, COVERAGE};
use voxel_model::refusal::RefusalSet;
use voxel_package::census::{self, Census, ProseStanding};

use crate::source::Source;

/// What one source returned: the reading, or the reason it could not produce one.
///
/// **The negative arm is not an empty reading.** An empty `Vec` or a zero would read as a
/// measurement, so a source that could not run says so in its own sentence, and the report prints
/// that sentence beside the source's name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceReading<T> {
    /// The source ran and produced this.
    Read(T),
    /// The source did not run, and this is why. `not measured` is not a pass.
    Refused {
        /// The refusal, with the file and the reason in it.
        detail: String,
    },
}

impl<T> SourceReading<T> {
    /// The reading, when there is one.
    pub fn read(&self) -> Option<&T> {
        match self {
            SourceReading::Read(value) => Some(value),
            SourceReading::Refused { .. } => None,
        }
    }

    /// Whether the source ran.
    pub fn is_read(&self) -> bool {
        matches!(self, SourceReading::Read(_))
    }

    /// Why there is no reading, when there is not one.
    pub fn refusal(&self) -> Option<&str> {
        match self {
            SourceReading::Read(_) => None,
            SourceReading::Refused { detail } => Some(detail),
        }
    }
}

/// The tree's evidence, one field per [`Source`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Evidence {
    /// The root every path in this value is relative to.
    pub root: PathBuf,
    /// The workspace's member package names, in manifest order.
    pub workspace: SourceReading<Vec<String>>,
    /// The package census, as the census itself decided it.
    pub census: SourceReading<Census>,
    /// Every crate's prose standing, re-measured on this read.
    pub conformance: SourceReading<Vec<ProseStanding>>,
    /// Whether every coverage row still describes its ladder. **`()` is the whole reading**: the
    /// gate either holds or refuses with the reason.
    pub ladder: SourceReading<()>,
}

impl Evidence {
    /// Read every source at `root`. **Never fails**: each arm records its own refusal.
    pub fn read(root: &Path) -> Self {
        Evidence {
            root: root.to_path_buf(),
            workspace: read_workspace(root),
            census: read_census(root),
            conformance: read_conformance(root),
            ladder: read_ladder(),
        }
    }

    /// The source that produced a reading, when it did. **`None` is not a pass.**
    pub fn reading_of(&self, source: Source) -> bool {
        match source {
            Source::Workspace => self.workspace.is_read(),
            Source::Census => self.census.is_read(),
            Source::Conformance => self.conformance.is_read(),
            Source::Ladder => self.ladder.is_read(),
        }
    }

    /// Every source that refused, with its reason — the report's own accounting, in
    /// [`Source::ALL`] order.
    pub fn refusals(&self) -> Vec<(Source, String)> {
        let mut out = Vec::new();
        for source in Source::ALL {
            let detail = match source {
                Source::Workspace => self.workspace.refusal(),
                Source::Census => self.census.refusal(),
                Source::Conformance => self.conformance.refusal(),
                Source::Ladder => self.ladder.refusal(),
            };
            if let Some(detail) = detail {
                out.push((source, detail.to_string()));
            }
        }
        out
    }
}

fn read_workspace(root: &Path) -> SourceReading<Vec<String>> {
    let manifest = root.join("Cargo.toml");
    match std::fs::read_to_string(&manifest) {
        Ok(text) => {
            let packages = census::workspace_member_packages(root, &text);
            if packages.is_empty() {
                SourceReading::Refused {
                    detail: format!(
                        "{} was read and its `members` array named nothing, so no crate was \
                         compared. An empty member list passes every check it is handed",
                        manifest.display()
                    ),
                }
            } else {
                SourceReading::Read(packages)
            }
        }
        Err(error) => SourceReading::Refused {
            detail: format!(
                "{} could not be read ({error}), so which crates the workspace declares is \
                 unknown. A workspace nobody read is not an empty one",
                manifest.display()
            ),
        },
    }
}

fn read_census(root: &Path) -> SourceReading<Census> {
    match census::census(root) {
        Ok(census) => SourceReading::Read(census),
        Err(refusals) => SourceReading::Refused {
            detail: joined(&refusals),
        },
    }
}

fn read_conformance(root: &Path) -> SourceReading<Vec<ProseStanding>> {
    match census::check_prose_frame(root) {
        Ok(standings) => SourceReading::Read(standings),
        Err(refusals) => SourceReading::Refused {
            detail: joined(&refusals),
        },
    }
}

fn read_ladder() -> SourceReading<()> {
    match check_coverage(&COVERAGE) {
        Ok(()) => SourceReading::Read(()),
        Err(refusals) => SourceReading::Refused {
            detail: joined(&refusals),
        },
    }
}

/// Every refusal in a set, as one sentence a report can carry.
fn joined(refusals: &RefusalSet) -> String {
    let details: Vec<&str> = refusals
        .all()
        .iter()
        .map(|refusal| refusal.detail())
        .collect();
    if details.is_empty() {
        // A refusal set with no refusal in it is a set that refused nothing, and a report that
        // printed an empty reason would read as "no problem". Name the absence instead.
        "the instrument refused without naming a reason".to_string()
    } else {
        details.join("; ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn real_root() -> PathBuf {
        voxel_package::features::workspace_root(Path::new(env!("CARGO_MANIFEST_DIR")))
            .expect("this crate sits inside the workspace")
    }

    #[test]
    fn the_real_tree_is_read_by_all_four_sources() {
        let evidence = Evidence::read(&real_root());
        assert!(evidence.workspace.is_read(), "{:?}", evidence.workspace);
        assert!(evidence.census.is_read(), "{:?}", evidence.census);
        assert!(evidence.conformance.is_read(), "{:?}", evidence.conformance);
        assert!(evidence.ladder.is_read(), "{:?}", evidence.ladder);
        assert!(evidence.refusals().is_empty(), "{:?}", evidence.refusals());

        let members = evidence.workspace.read().expect("read");
        assert!(
            members.iter().any(|member| member == "voxel-axis-verify"),
            "the verifier's own crate is a workspace member: {members:?}"
        );
        assert!(
            evidence
                .census
                .read()
                .expect("read")
                .reading_of(voxel_package::census::Crate::Verify)
                .is_some(),
            "and the census has a row for it"
        );
    }

    #[test]
    fn a_root_with_no_manifest_refuses_the_three_tree_sources_and_keeps_the_reasons() {
        let scratch =
            std::env::temp_dir().join(format!("voxel-axis-verify-empty-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&scratch);
        std::fs::create_dir_all(&scratch).expect("an empty scratch tree");

        let evidence = Evidence::read(&scratch);
        for source in [Source::Workspace, Source::Census, Source::Conformance] {
            assert!(
                !evidence.reading_of(source),
                "{} reported a reading from an empty root",
                source.name()
            );
        }
        // **The ladder table is the running build's own**, not a file read at this root: it was
        // compiled from `voxel-model/src/axis.rs`, so an empty directory cannot make it unreadable.
        assert!(evidence.ladder.is_read(), "{:?}", evidence.ladder);

        let refusals = evidence.refusals();
        assert_eq!(refusals.len(), 3, "{refusals:?}");
        for (source, detail) in &refusals {
            assert!(
                detail.contains("could not") && detail.len() > 40,
                "{}'s refusal is not a sentence about a read: {detail}",
                source.name()
            );
        }
        let _ = std::fs::remove_dir_all(&scratch);
    }

    #[test]
    fn a_reading_and_a_refusal_are_different_states() {
        let read: SourceReading<u8> = SourceReading::Read(0);
        assert!(read.is_read());
        assert_eq!(read.read(), Some(&0));
        assert_eq!(read.refusal(), None, "zero is a reading, not an absence");

        let refused: SourceReading<u8> = SourceReading::Refused {
            detail: "not measured".to_string(),
        };
        assert!(!refused.is_read());
        assert_eq!(refused.read(), None);
        assert_eq!(refused.refusal(), Some("not measured"));
    }
}
