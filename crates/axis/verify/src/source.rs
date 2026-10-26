//! **The four sources of evidence, each named with the file it reads.**
//!
//! A verification claim is only as good as where it read. This crate is allowed to consult four
//! things and no fifth, because each of the four is a measurement the tree already takes and nothing
//! here needs to invent one:
//!
//! | source | what it reads | the file |
//! |---|---|---|
//! | [`Source::Workspace`] | the workspace's `members` array and each member's package name | `Cargo.toml` |
//! | [`Source::Census`] | the crate table, the six census axes and the missing-docs pins | `voxel-package/src/census.rs`, `conformance/missing-docs.txt` |
//! | [`Source::Conformance`] | the prose frame, re-measured on this build | `conformance/prose-frame.txt` |
//! | [`Source::Ladder`] | the nine ladders and their coverage rows | `voxel-model/src/axis.rs` |
//!
//! **A source is a name, not an authority.** It never passes or fails anything on its own: a cell's
//! basis quotes the source that produced each of its rows, and a segment no source could witness is
//! declined **by naming the sources that declined**, rather than left blank so it reads as unasked.

/// One holder of evidence this crate is allowed to consult.
///
/// The variants are closed on purpose. A fifth source would be a new claim about where verification
/// comes from, and that is a decision for the record rather than a string a caller invents at a call
/// site — the same rule [`voxel_axis_matrix::Segment`] applies to its seven segments.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Source {
    /// The workspace manifest: which crates exist, and each member's own package name.
    Workspace,
    /// The package census: what each crate was read as, on six axes, and whether the compiler's
    /// documentation count is still about this tree.
    Census,
    /// The conformance files: the prose frame, re-measured every build.
    Conformance,
    /// The axis ladder definitions and their coverage table.
    Ladder,
}

/// How many sources this build knows. **Written down rather than derived**, and pinned by
/// [`SOURCES_ARE_TOTAL`].
pub const SOURCE_COUNT: usize = 4;

impl Source {
    /// Every source, in the order a report prints them: what exists, then what was read, then what
    /// was checked, then the vocabulary the readings are written in.
    pub const ALL: [Source; SOURCE_COUNT] = [
        Source::Workspace,
        Source::Census,
        Source::Conformance,
        Source::Ladder,
    ];

    /// The recorded name. Lowercase, one word, and the word a cell's basis carries.
    pub const fn name(self) -> &'static str {
        match self {
            Source::Workspace => "workspace",
            Source::Census => "census",
            Source::Conformance => "conformance",
            Source::Ladder => "ladder",
        }
    }

    /// The file a reader opens to check this source. **One primary site each**, so a refusal can
    /// name the thing that could not be read rather than the source in the abstract.
    ///
    /// For [`Source::Ladder`] the site is the declaration's home rather than a file opened at run
    /// time: the coverage table is a `const` compiled from that file, so the reading is this
    /// build's own table and a root with no checkout cannot make it unreadable.
    pub const fn site(self) -> &'static str {
        match self {
            Source::Workspace => "Cargo.toml",
            Source::Census => "conformance/missing-docs.txt",
            Source::Conformance => "conformance/prose-frame.txt",
            Source::Ladder => "crates/voxel-model/src/axis.rs",
        }
    }

    /// What this source is for, in one sentence a reader can check against the site.
    pub const fn means(self) -> &'static str {
        match self {
            Source::Workspace => {
                "the workspace's `members` array and each member's `[package] name`, read through \
                 `census::workspace_member_packages` so a grouped folder is not mistaken for a \
                 package"
            }
            Source::Census => {
                "the crate table and the six census axes, with the compiler's documentation count \
                 and the pin that says whether that count is about this tree"
            }
            Source::Conformance => {
                "the prose frame: each crate's plain-comment share against the budget it declared, \
                 re-measured on this build rather than remembered"
            }
            Source::Ladder => {
                "the nine ladders' tokens and the coverage table that must describe each ladder's \
                 own members"
            }
        }
    }
}

impl std::fmt::Display for Source {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.name())
    }
}

/// The table is as wide as the source set. **Compile-checked, not tested**, in the same style as
/// the census's crate count and the matrix's segment count: the value is meaningless, the **type**
/// is the assertion.
pub const SOURCES_ARE_TOTAL: [(); SOURCE_COUNT] = [(); Source::ALL.len()];

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn root() -> std::path::PathBuf {
        voxel_package::features::workspace_root(Path::new(env!("CARGO_MANIFEST_DIR")))
            .expect("this crate sits inside the workspace")
    }

    #[test]
    fn every_source_has_a_distinct_name_a_real_site_and_a_sentence() {
        let mut names: Vec<&str> = Source::ALL.iter().map(|source| source.name()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), SOURCE_COUNT, "two sources share a name");
        for source in Source::ALL {
            assert_eq!(source.name(), source.name().to_lowercase());
            assert!(
                !source.name().contains(' '),
                "{} is two words",
                source.name()
            );
            assert!(
                source.means().len() > 60,
                "{} has no sentence: {:?}",
                source.name(),
                source.means()
            );
            assert!(
                root().join(source.site()).exists(),
                "{} names `{}`, which is not on disk",
                source.name(),
                source.site()
            );
        }
    }

    #[test]
    fn the_table_is_as_wide_as_the_count() {
        // The pin is the compile-time check; this is the same fact stated where a reader looks.
        assert_eq!(Source::ALL.len(), SOURCE_COUNT);
        assert_eq!(SOURCES_ARE_TOTAL.len(), SOURCE_COUNT);
    }
}
