//! The ingest survey on its own: `cargo ingest`, or
//! `cargo run -p voxel-package --example ingest`.
//!
//! # Why a bin exists when the project's rule says measurements belong in the pass that runs
//!
//! The same reason `cargo space` exists, and the reason is stronger here. A survey of the corpus on
//! *other drives* is not a thing a build gate can afford: `F:` answered a naive walk in 400 seconds
//! in the session that first measured it, and the gate runs on every build. So the survey is a
//! module (`voxel_package::ingest`) with a report that is the same text wherever it is read, and
//! this is the command that asks it — deliberately **outside** the build.
//!
//! Exit code is the survey's own verdict: `0` when the manifest was declared and parsed, `1` when
//! it was not declared or a line of it was refused, `2` for a usage error. It never fails because a
//! root is absent: an absent root is a fact this machine states, not an error.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use voxel_package::ingest::{self, Budget};
use voxel_package::patch;

fn main() -> ExitCode {
    let mut max_files: Option<usize> = None;
    let mut args = std::env::args().skip(1);
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--help" | "-h" => {
                println!(
                    "the corpus survey: what the other trees on these drives hold\n\n\
                     USAGE: cargo ingest [--max-files <n>]\n\n\
                     Walks the roots declared in {} and classifies every file from its name and \
                     its first {} bytes, reporting the share of the corpus it actually read. It \
                     writes nothing: this is the measurement an import would have to answer to.\n\n\
                     --max-files bounds the walk per root (default {}). Exit 1 means the manifest \
                     was not declared or a line of it was refused.\n",
                    ingest::MANIFEST_PATH,
                    ingest::HEAD_BYTES,
                    Budget::DEFAULT.max_files
                );
                return ExitCode::SUCCESS;
            }
            "--max-files" => match args.next().and_then(|value| value.parse().ok()) {
                Some(files) => max_files = Some(files),
                None => {
                    eprintln!("--max-files needs a whole number");
                    return ExitCode::from(2);
                }
            },
            other => {
                eprintln!("unknown argument {other:?}; try --help");
                return ExitCode::from(2);
            }
        }
    }

    let workspace: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the crate sits under the workspace root")
        .to_path_buf();

    let manifest = ingest::Manifest::read(&workspace);
    if !manifest.declared {
        print!("{}", ingest::survey(&manifest, &patch::PatchSet::unmeasured("no manifest"), &Budget::DEFAULT).describe());
        eprintln!("\n{} is not declared, so no root was walked", ingest::MANIFEST_PATH);
        return ExitCode::FAILURE;
    }
    for problem in &manifest.problems {
        eprintln!("{}: {problem}", ingest::MANIFEST_PATH);
    }

    let budget = match max_files {
        Some(files) => Budget {
            max_files: files,
            ..Budget::DEFAULT
        },
        None => Budget::DEFAULT,
    };
    let ours = patch::read_dir(&workspace.join("patches"));
    let survey = ingest::survey(&manifest, &ours, &budget);
    print!("{}", survey.describe());

    if manifest.problems.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
