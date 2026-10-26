//! The patch corpus as recipe lines: compile what is declared, derive what the prose sources,
//! and name every field that cannot be derived instead of inventing it.
//!
//! ```text
//! USAGE: voxel-patch-recipes [--root <dir>] [--ticket <id>] [--write] [--help]
//!
//! Exit 0 — the walk finished; gaps are printed, not fatal.
//! Exit 1 — a record or the matrix could not be read.
//! Exit 2 — a usage error.
//! ```
//!
//! `--write` writes only the drafts that compiled, to
//! `patches/upstream/recipes/<PATCH>.recipe`, plus `coverage.tsv`. It does not edit a patch
//! record: a derived line is a draft about the record, not the record's own words.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use voxel_mcp::patch_recipe_coverage;

fn main() -> ExitCode {
    let mut root: Option<PathBuf> = None;
    let mut ticket = "T-0954".to_string();
    let mut write = false;
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--help" | "-h" => {
                println!(
                    "compile the patch corpus into recipe lines\n\n\
                     USAGE: voxel-patch-recipes [--root <dir>] [--ticket <id>] [--write]\n\n\
                     Declared blocks are compiled through the matrix's own route. A prose-only\n\
                     record gets a draft only where its text sources the site, the command and\n\
                     one segment; everything else is a named gap. --write writes compiled drafts\n\
                     to patches/upstream/recipes/ and never edits a patch record."
                );
                return ExitCode::SUCCESS;
            }
            "--write" => write = true,
            "--root" => match arguments.next() {
                Some(value) => root = Some(PathBuf::from(value)),
                None => {
                    eprintln!("voxel-patch-recipes: --root needs a directory");
                    return ExitCode::from(2);
                }
            },
            "--ticket" => match arguments.next() {
                Some(value) if !value.trim().is_empty() => ticket = value,
                _ => {
                    eprintln!("voxel-patch-recipes: --ticket needs an id");
                    return ExitCode::from(2);
                }
            },
            other => {
                eprintln!("voxel-patch-recipes: `{other}` is not an argument this command takes; try --help");
                return ExitCode::from(2);
            }
        }
    }
    let root = root.unwrap_or_else(|| {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("the crate sits two levels under the workspace")
            .to_path_buf()
    });
    match patch_recipe_coverage(&root, &ticket, write) {
        Ok(out) => {
            print!("{out}");
            ExitCode::SUCCESS
        }
        Err(why) => {
            eprintln!("voxel-patch-recipes: {why}");
            ExitCode::FAILURE
        }
    }
}
