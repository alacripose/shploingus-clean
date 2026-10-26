//! **The package, as a value: what `PATCH-042`'s five requirements mean when they are code.**
//!
//! The patch states a code package in one paragraph, and each clause is a gate here rather than a
//! style note:
//!
//! | requirement | where it is enforced |
//! |---|---|
//! | **readable** — plain text, one statement per line, no blob | [`Package::parse_bytes`] refuses bytes that are not UTF-8 or that carry a control character; [`Package::parse`] refuses any text that is not exactly the rendering of the package it parses to |
//! | **cited** — every line quotes a `file:line` | [`Line`] takes the citation from the cell's own witness and the file's first non-blank line; [`Package::gate`] re-locates it, and refuses a citation the tree does not have |
//! | **laddered** — token, basis and a support that was computed | [`Line::check`] refuses a token that is not a member of its own ladder; [`Package::gate`] re-derives the support and refuses one the machine does not |
//! | **pinned** — bytes hashed and signed, stale refused | [`Package::pin`] hashes every byte above `## pin`; [`Package::sign`] makes the seam's signature; [`Package::gate`] refuses a digest that moved, an absent signature, and a signature the verifier does not accept |
//! | **reconstructing** — the named files, byte for byte | [`Package::reproduce`] re-reads every file and refuses a missing one or drifted bytes |
//!
//! # What a package is made of, and what it is not
//!
//! A statement is **the matrix's own reading**: the cell's segment, star, tier and ticket in the
//! matrix's words, its ladder token and basis unchanged, and its witness file as the citation. This
//! crate authors nothing — it formats what [`voxel_axis_matrix::Cell`] already measured, which is why
//! a package can be re-derived from the same tree and come back identical.
//!
//! ```text
//! # voxel package · T-0954
//! # plain text, one statement per line; the pin covers every byte above it
//! ## claims
//! shell ★★★☆☆ silver T-0954	crates/voxel-client/src/shell/mod.rs:1	evidence_grade B	support_level direct	read back from the tree — …
//! ## pin
//! bytes	sha256:…
//! ## reproduce
//! crates/voxel-client/src/shell/mod.rs	sha256:…
//! ```
//!
//! # The signature is a seam, not a claim
//!
//! [`voxel_package::signing`] already owns the question *"who signed this, and does it check out"*,
//! and this crate names the traits rather than a backend. The shell links `voxel-axis-security`, which
//! holds the machine's key outside the checkout; a build with no backend still produces a package that
//! is pinned, reproducible and **refused at the pin**, so an unsigned document is never read as a
//! signed one. There is no weakening here: no key, no signature, no pass.
//!
//! # Where this sits
//!
//! Beside [`voxel_axis_verify`], which measures the cells, and [`voxel_axis_matrix`], which owns the
//! vocabulary. The shell joins the three: the verifier's cells become this crate's package, and the
//! console prints it. Nothing here opens a file for writing.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod line;
pub mod package;
pub mod reproduce;

pub use line::Line;
pub use package::{Package, Pin, CLAIMS, HEADER, NOTE, PIN, REPRODUCE};
pub use reproduce::{reproduce, Reproduced};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use voxel_model::refusal::RefusalKind;

    fn root() -> PathBuf {
        voxel_package::features::workspace_root(Path::new(env!("CARGO_MANIFEST_DIR")))
            .expect("this crate sits inside the workspace")
    }

    #[test]
    fn the_trees_own_cells_become_a_package_that_reparses_byte_for_byte() {
        let root = root();
        let report = voxel_axis_verify::verify(&root, "T-0954");
        let package = Package::of(&root, "T-0954", report.cells()).expect("the verifier's cells");
        assert_eq!(
            package.lines().len(),
            report.cells().len(),
            "one statement per measured cell, and none invented"
        );
        for line in package.lines() {
            assert!(
                root.join(&line.cite().path).is_file(),
                "{} cites a file the tree does not have",
                line.statement()
            );
            assert!(line.cite().line > 0, "{} cites no line", line.statement());
            assert_eq!(
                line.support().as_str(),
                "direct",
                "every statement in the shipped package is on a witnessed file"
            );
        }
        let text = package.render();
        let parsed = Package::parse_bytes(text.as_bytes()).expect("its own bytes");
        assert_eq!(parsed.render(), text, "the package reads back identically");
        parsed
            .reproduce(&root)
            .expect("every file it names is the tree's");
    }

    #[test]
    fn the_package_is_produced_and_refused_at_the_pin_while_it_is_unsigned() {
        let root = root();
        let report = voxel_axis_verify::verify(&root, "T-0954");
        let package = Package::of(&root, "T-0954", report.cells()).expect("the verifier's cells");
        assert_eq!(
            package.pin().digest,
            voxel_model::Identity::of(package.body().as_bytes()),
            "the pin is over the bytes it says it covers"
        );
        // The shell's real path: no backend is linked, so nothing signs, so the package is unsigned
        // and the pin gate refuses it. `NoSigningBackend` is what a *signed* package gets from that
        // same absent backend, and `package::tests` holds that side.
        let refused = package
            .gate(
                &root,
                &voxel_package::signing::NoSigningBackend {
                    why: "the shell links no voxel-envelope backend".to_string(),
                },
            )
            .expect_err("a missing instrument is not a pass");
        assert!(
            refused
                .all()
                .iter()
                .any(|refusal| refusal.kind() == RefusalKind::UnsignedEnvelope),
            "{refused:?}"
        );
    }
}
