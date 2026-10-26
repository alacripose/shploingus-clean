//! The working directory's folders, classified and written:
//! `cargo run -q -p voxel-package --bin voxel-census`.
//!
//! # What it answers that the crate table cannot
//!
//! `get_census` measures **crates** on six axes, and it walks the member list — so every folder
//! that is not a crate is outside it. This command is the other half: one row per top-level folder,
//! its kind, the artifact that declares the kind, and its bytes and files. The output is printed
//! and written to `packages/census.tsv`, the emit root M10.5 names.
//!
//! ```text
//! USAGE: voxel-census [<folder>]
//!
//! Exit 0 — every folder is classified and packages/census.tsv was written.
//! Exit 1 — a folder is unclassified, or a declaration lost its witness; nothing is written.
//! Exit 2 — a usage error.
//! ```
//!
//! **The table is the only file it writes.** A folder with no declaration is refused by name
//! rather than given a default kind, because a table that silently skipped a folder would be the
//! one artifact this command exists to prevent.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use voxel_model::RefusalSet;
use voxel_package::census::folders;

fn main() -> ExitCode {
    let mut root: Option<PathBuf> = None;
    for argument in std::env::args().skip(1) {
        match argument.as_str() {
            "--help" | "-h" => {
                println!(
                    "the working directory's top level, classified\n\n\
                     USAGE: voxel-census [<folder>]\n\n\
                     Prints one row per folder, then census-unclassified, and writes\n\
                     packages/census.tsv. A folder with no declaration is refused by name.\n\n\
                     Exit 0 — every folder is classified and the table is written.\n\
                     Exit 1 — a folder is unclassified or a declaration lost its witness.\n\
                     Exit 2 — a usage error."
                );
                return ExitCode::SUCCESS;
            }
            other if !other.starts_with('-') && root.is_none() => root = Some(PathBuf::from(other)),
            other => {
                eprintln!("voxel-census: `{other}` is not an argument this command takes; try --help");
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

    let folders = match folders::classify(&root) {
        Ok(folders) => folders,
        Err(found) => {
            eprintln!("{}", RefusalSet::to_string(&found));
            return ExitCode::FAILURE;
        }
    };
    print!("{}", folders.describe());
    match folders::write(&root) {
        Ok(_) => {
            println!("table\t{}", folders::TABLE_PATH);
            ExitCode::SUCCESS
        }
        Err(found) => {
            eprintln!("{}", RefusalSet::to_string(&found));
            ExitCode::FAILURE
        }
    }
}
