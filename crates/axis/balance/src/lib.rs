//! **The balance census: how much of this tree is CPU code, how much is GPU code, and which
//! public visual items reach no renderer at all.**
//!
//! The axis crates before this one read *evidence about a system*: the matrix reads cells, the
//! verifier reads the census, the package reads both. This one asks a different question, and it
//! is the user's question of 2026-10-21:
//!
//! > *"the newly defined axis crates system needs an extra crate for verifying the balance of cpu
//! > code vs gpu code and how many items in cpu code have no gpu renderer"*
//!
//! # Two measurements, and they are deliberately different kinds
//!
//! | measurement | where it comes from | what it answers |
//! |---|---|---|
//! | [`measure::Measure`] | the tree, walked | how many files, lines and bytes of Rust and of WGSL each crate carries |
//! | [`inventory::Item`] | declared here, held to the tree | which public visual items exist, and which renderer each one reaches |
//!
//! **The first is a measurement. The second is a declared table, and the crate says so.** A
//! visual item is not something a file walk can find: a name in a `pub struct` says nothing about
//! whether a frame draws it, and a grep for a type in the renderer would count a comment as a
//! renderer. So the inventory is stated in [`inventory::INVENTORY`], and every row is checked
//! against the source it names: the declaration must still be in the file, and the renderer the
//! row names must still be a function that exists. A row that lies is refused, and the reading is
//! not a pass.
//!
//! # What "balance" means here
//!
//! It is a **ratio of lines**, per crate and for the whole workspace: the Rust the CPU executes
//! against the WGSL the GPU executes. It is not a target and it is not a score. A shader is
//! written densely and a test is not a shader, so the ratio is reported with its two operands
//! beside it; a reader who wants a different denominator has the counts to build one.
//!
//! The **unrendered count** is the sharp end: an item declared in the inventory with no renderer
//! is counted, named and printed with its site, because a visual construct the CPU builds and
//! nobody draws is work a player never sees.
//!
//! # What this crate is not
//!
//! * **Not a second census.** [`voxel_package::census`] already says what each crate *is* on six
//!   axes. This crate asks how much of it is CPU and how much is GPU, which is a measurement the
//!   census does not take and does not need to.
//! * **Not a dependency on the client.** It reads the client's files as text and depends on no
//!   UI crate, so the inventory cannot become a second place a window is declared.
//! * **Not a gate that fails a build by itself.** It produces a [`report::Report`]; `verified`
//!   is false when a row could not be witnessed or a crate could not be read, and the caller
//!   decides what that costs. The report prints the unrendered items either way.
//!
//! # Where the rules are recorded
//!
//! `PATCH-045` states the crate, the two measurements, the inventory rule and the refusal table.
//!
//! [`PATCH-045`]: ../../../patches/PATCH-045-the-balance-census.md

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod inventory;
pub mod measure;
pub mod report;

pub use inventory::{check, Inventory, Item, Renderer, RowRefusal, INVENTORY};
pub use measure::Measure;
pub use report::{balance, Report, Row};

use std::path::Path;

/// **Read the tree and answer with the balance.** Nothing is written.
///
/// Every crate in [`voxel_package::census::Crate::ALL`] gets a row, whether or not its directory
/// exists: a missing directory is a refusal in that row's place, not a row that silently vanishes
/// from the total.
pub fn read(root: &Path) -> Report {
    report::balance(root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use voxel_package::census::{Crate, CRATE_COUNT};

    fn real_root() -> PathBuf {
        voxel_package::features::workspace_root(Path::new(env!("CARGO_MANIFEST_DIR")))
            .expect("this crate sits inside the workspace")
    }

    #[test]
    fn the_public_act_reads_the_whole_tree_and_refuses_nothing_it_can_name() {
        let report = read(&real_root());
        assert_eq!(report.rows().len(), CRATE_COUNT);
        assert!(
            report.refusals().is_empty(),
            "the balance census refused: {:#?}",
            report.refusals()
        );
        assert!(report.verified(), "a clean tree is a verified reading");
        assert!(report.rust().files > 0, "no Rust was measured");
        // **Five, because the merge and wire passes were folded into `shading.wgsl`**: the tree
        // holds `atmosphere`, `display`, `panels`, `shading` and `sky`. The threshold is the
        // tree's shader count; what it protects is that the census walked them at all rather
        // than reporting an empty reading.
        assert!(report.wgsl().files >= 5, "the shaders were not measured");
        assert!(
            report.inventory().rendered + report.inventory().unrendered.len()
                == INVENTORY.len(),
            "every item is rendered or counted as unrendered, and none is both"
        );
    }

    #[test]
    fn every_workspace_crate_has_a_row_in_dependency_order() {
        let report = read(&real_root());
        let names: Vec<&str> = report.rows().iter().map(|row| row.crate_.name()).collect();
        let expected: Vec<&str> = Crate::ALL.iter().map(|crate_| crate_.name()).collect();
        assert_eq!(names, expected, "the rows follow the census, not a second list");
    }

    #[test]
    fn a_tree_with_no_manifest_refuses_rather_than_reporting_zero() {
        let scratch = std::env::temp_dir().join(format!("voxel-axis-balance-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&scratch);
        std::fs::create_dir_all(&scratch).expect("a scratch tree");
        let report = read(&scratch);
        assert!(!report.verified());
        assert!(
            report
                .refusals()
                .iter()
                .any(|refusal| refusal.kind() == voxel_model::refusal::RefusalKind::NoReading),
            "an unreadable tree is NoReading, not a zero: {:#?}",
            report.refusals()
        );
        let _ = std::fs::remove_dir_all(&scratch);
    }
}
