//! **The reproduction claim: re-read the files the package names, byte for byte, or refuse.**
//!
//! `PATCH-042` gate 5 is *"applying a package to a clean base reproduces exactly the files it names
//! ... refused on any drift"*. This tree has exactly the base that clause is about — the checkout the
//! statements were measured from — so the claim is checked the only way a file-reading program can
//! check it: every cited file is re-read, hashed, and compared with the digest the package recorded.
//!
//! **Three different failures, three names**, because *"nothing to re-derive here"*, *"the file is
//! gone"* and *"the bytes moved"* send a reader to three different places:
//!
//! | what is wrong | refused as | who is sent where |
//! |---|---|---|
//! | this machine has no tree at the root | `NoReading` | the caller, who named a root |
//! | the package names a file that is not there | `NoReading` | the file, by path |
//! | the file's bytes differ from the recorded digest | `DigestMismatch` | the drift, with both digests |
//!
//! A package with no files, or a machine that cannot read the root, is **refused rather than
//! promised**: an unverifiable reproduction is not a reproduction.

use std::path::Path;

use voxel_model::refusal::{RefusalKind, RefusalSet};
use voxel_model::Identity;

use crate::package::Package;

/// What a successful reproduction proved: how many files were re-read, and one digest over them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reproduced {
    files: usize,
    digest: Identity,
}

impl Reproduced {
    /// How many files the package re-read.
    pub fn files(&self) -> usize {
        self.files
    }

    /// One digest over every `(path, digest)` pair, so two reproductions can be compared.
    pub fn digest(&self) -> Identity {
        self.digest
    }
}

/// **The claim, checked.** Every file the package names is re-read from `root` and compared byte for
/// byte; the first thing that cannot be re-derived is refused with its path and the line in the
/// package that names it.
pub fn reproduce(package: &Package, root: &Path) -> Result<Reproduced, RefusalSet> {
    let mut refusals = RefusalSet::new();
    if !root.is_dir() {
        refusals.refuse(
            RefusalKind::NoReading,
            format!(
                "the package cannot be re-derived on this machine: {} is not a tree, and a \
                 reproduction that was not run is not a reproduction",
                root.display()
            ),
        );
        return Err(refusals);
    }
    if package.files().is_empty() {
        refusals.refuse(
            RefusalKind::NoReading,
            "the package names no file, so applying it would write nothing",
        );
        return Err(refusals);
    }
    let mut reproduced: Vec<(String, String)> = Vec::with_capacity(package.files().len());
    for (index, (path, recorded)) in package.files().iter().enumerate() {
        let at = package.file_line(index);
        let bytes = match std::fs::read(root.join(path)) {
            Ok(bytes) => bytes,
            Err(error) => {
                refusals.refuse(
                    RefusalKind::NoReading,
                    format!(
                        "line {at}: the package names `{path}` and this machine cannot read it \
                         ({error}), so it cannot be re-derived here"
                    ),
                );
                continue;
            }
        };
        let found = Identity::of(&bytes);
        if found != *recorded {
            refusals.refuse(
                RefusalKind::DigestMismatch,
                format!(
                    "line {at}: `{path}` was recorded as {recorded} and its bytes now hash to \
                     {found}; the package describes a tree that no longer exists"
                ),
            );
            continue;
        }
        reproduced.push((path.clone(), found.to_string()));
    }
    let fields: Vec<&[u8]> = reproduced
        .iter()
        .flat_map(|(path, digest)| [path.as_bytes(), digest.as_bytes()])
        .collect();
    refusals.into_result(Reproduced {
        files: package.files().len(),
        digest: Identity::of_fields("voxel-package-reproduction", &fields),
    })
}
