//! The layer recipes on their own: `cargo run -p voxel-package --bin voxel-layers`.
//!
//! # Why a command rather than a function nobody runs
//!
//! M10.2's acceptance is a *rendered* recipe: every layer, every window and every row named, with no
//! path that exists only in the prose, and the same bytes on two runs. A renderer with no caller is a
//! renderer nobody has read, so this is the reader — and it is the command the baseline record
//! `docs/baselines/mount-v2/layer-recipes-2026-10-24.txt` quotes.
//!
//! # What it does
//!
//! Walks `display-packages/` (or the folder it is given), and for every folder holding a
//! `package.shspec`, renders each `layers/<id>.layer` together with `layers/<id>.md`. The layers must
//! be named by a row in that package's recipe, and every record must carry the four patch sections;
//! a layer that fails either is printed **with the file it was read from** and the exit code is 1.
//!
//! ```text
//! USAGE: voxel-layers [<folder>]
//!
//! Exit 0 — every layer declared in every folder was read and rendered.
//! Exit 1 — a layer, a record or a recipe was refused; every refusal is printed.
//! Exit 2 — a usage error.
//! ```
//!
//! **It writes nothing.** The baseline is a record of this command's output, made by the pass that
//! read it, exactly as `docs/baselines/` holds every other reading.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use voxel_model::RefusalSet;

fn main() -> ExitCode {
    let mut root: Option<PathBuf> = None;
    for argument in std::env::args().skip(1) {
        match argument.as_str() {
            "--help" | "-h" => {
                println!(
                    "the layer recipes, rendered from the folders' own declarations\n\n\
                     USAGE: voxel-layers [<folder>]\n\n\
                     With no folder, the workspace's display-packages/ is walked. Every folder that \
                     holds a package.shspec is measured; each layers/<id>.layer is rendered with \
                     its layers/<id>.md, and a layer the recipe names by no row is refused.\n\n\
                     Exit 0 — every layer was rendered. Exit 1 — something was refused, and every \
                     refusal is printed with its file. Exit 2 — a usage error.\n"
                );
                return ExitCode::SUCCESS;
            }
            other if other.starts_with('-') => {
                eprintln!("unknown argument {other:?}; try --help");
                return ExitCode::from(2);
            }
            other => {
                if root.is_some() {
                    eprintln!("one folder at a time; {other:?} is a second");
                    return ExitCode::from(2);
                }
                root = Some(PathBuf::from(other));
            }
        }
    }

    let workspace: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("the crate sits two levels under the workspace")
        .to_path_buf();
    let root = root.unwrap_or_else(|| workspace.join("display-packages"));

    if !root.is_dir() {
        eprintln!("{} is not a directory", root.display());
        return ExitCode::from(2);
    }

    let mut folders: Vec<PathBuf> = match std::fs::read_dir(&root) {
        Ok(entries) => entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_dir() && path.join("package.shspec").is_file())
            .collect(),
        Err(why) => {
            eprintln!("{} could not be read ({why})", root.display());
            return ExitCode::FAILURE;
        }
    };
    folders.sort();

    if folders.is_empty() {
        eprintln!(
            "{} holds no folder with a package.shspec, so nothing was measured; a count of zero \
             here would read as a package that declares no layer",
            root.display()
        );
        return ExitCode::FAILURE;
    }

    let mut refused = false;
    for folder in folders {
        match voxel_package::layer::render_package(&folder) {
            Ok(rendered) => println!("{rendered}"),
            Err(found) => {
                refused = true;
                eprintln!("{}: {}", folder.display(), RefusalSet::to_string(&found));
            }
        }
    }

    if refused {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
