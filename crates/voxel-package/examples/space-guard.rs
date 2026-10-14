//! The size guard on its own: `cargo space`, or `cargo run -p voxel-package --example space-guard`.
//!
//! # Why a bin exists when the project's own rule says measurements belong in the pass that runs
//!
//! `bloat.rs` records the ruling: a measurement that *"can only be read in a terminal"* is
//! incomplete, so the guard proper is a module the build gate calls. This is not that measurement.
//! It is the **escape hatch**, and it exists because the guard's own gate needs the GPU stack to
//! build: when the thing that is broken is the disk, `cargo check -p shploingus` is the build that
//! cannot run. `voxel-package` and its headless dependencies are the cheapest way to ask the same
//! question of the same module, and the report is byte-identical to the gate's section because it
//! is the same `Report`.
//!
//! Exit code is the guard's verdict, so a shell can gate on it: `0` when nothing is refused, `1`
//! when the volume is below the minimum the guard can act in, `2` for a usage error.

use std::path::Path;
use std::process::ExitCode;

use voxel_package::space;

fn main() -> ExitCode {
    let mut floor: Option<(u64, Option<String>)> = None;
    let mut target: Option<String> = None;
    let mut args = std::env::args().skip(1);
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--help" | "-h" => {
                println!(
                    "the build tree's size guard\n\n\
                     USAGE: cargo space [--floor <GiB>] [--target <dir>]\n\n\
                     With no --floor, the guard's own default ({}) and the {env} override apply.\n\
                     With no --target, the workspace's target directory is measured.\n\n\
                     It trims only what cargo regenerates, cheapest first, and reports every\n\
                     removal and every kept artefact with its reason. Exit 1 means the volume is\n\
                     below the minimum and a build there cannot finish.\n",
                    space::human(space::FLOOR),
                    env = space::FLOOR_ENV
                );
                return ExitCode::SUCCESS;
            }
            "--floor" => match args.next().as_deref().map(space::parse_floor) {
                Some(Ok(gib)) => floor = Some((gib * space::GIB, None)),
                Some(Err(text)) => {
                    eprintln!("--floor {text:?} is not a whole number of GiB");
                    return ExitCode::from(2);
                }
                None => {
                    eprintln!("--floor needs a value, in GiB");
                    return ExitCode::from(2);
                }
            },
            "--target" => match args.next() {
                Some(path) => target = Some(path),
                None => {
                    eprintln!("--target needs a path");
                    return ExitCode::from(2);
                }
            },
            other => {
                eprintln!("unknown argument {other:?}; try --help");
                return ExitCode::from(2);
            }
        }
    }

    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the crate sits under the workspace root");
    let tree = match target {
        Some(path) => std::path::PathBuf::from(path),
        None => space::target_dir(workspace),
    };

    let report = match floor {
        Some((floor, refused)) => space::guard_with(&tree, floor, refused),
        None => space::guard(&tree),
    };
    print!("{}", report.describe());
    match report.refusal() {
        Some(refusal) => {
            eprintln!("\nspace-guard refuses: {refusal}");
            ExitCode::FAILURE
        }
        None => ExitCode::SUCCESS,
    }
}

// **No parser here.** The floor's spelling is `space::parse_floor`'s, which is the same function
// the gate's `SHPLOINGUS_SPACE_FLOOR` goes through: two parsers would be two answers to one
// question, and the one that drifted would be the one nobody ran.
