//! **The reading: one row per crate, the totals, and the items no renderer reaches.**
//!
//! A report is a value, and printing it is [`Report::lines`]. The lines are plain text — the same
//! division `voxel_package::apidoc` keeps — so the shell, the console and a test all read one
//! statement rather than three formattings of it.
//!
//! # The refusal rule
//!
//! A crate whose directory does not exist is [`RefusalKind::MissingFeature`]: the census says a
//! crate is there and the tree disagrees. The workspace manifest itself is
//! [`RefusalKind::NoReading`]: with no root there is no measurement, and a zero would be a number
//! about nothing.
//!
//! **Nothing is dropped because it could not be read.** The row stays, with a zero measure, and
//! the refusal says so; the totals are then the sum of what was read, which is why `verified`
//! exists beside them.

use std::path::Path;

use voxel_model::refusal::{RefusalKind, RefusalSet};
use voxel_package::census::{Crate, CRATE_COUNT};

use crate::inventory::{check, Inventory, INVENTORY};
use crate::measure::Measure;

/// One crate's two measures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Row {
    /// The crate this row measures.
    pub crate_: Crate,
    /// Rust under its `src/`.
    pub rust: Measure,
    /// WGSL under its `src/`.
    pub wgsl: Measure,
}

impl Row {
    /// Rust lines per line of WGSL, or `None` for a crate with no shaders.
    pub fn ratio(self) -> Option<f64> {
        self.rust.per_line_of(self.wgsl)
    }
}

/// **The balance census's answer.** See the crate root for what each part is measured from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    rows: Vec<Row>,
    rust: Measure,
    wgsl: Measure,
    inventory: Inventory,
    refusals: RefusalSet,
}

impl Report {
    /// One row per crate, in [`Crate::ALL`] order.
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// Every crate's Rust, summed.
    pub fn rust(&self) -> Measure {
        self.rust
    }

    /// Every crate's WGSL, summed.
    pub fn wgsl(&self) -> Measure {
        self.wgsl
    }

    /// The whole workspace's Rust lines per line of WGSL.
    pub fn ratio(&self) -> Option<f64> {
        self.rust.per_line_of(self.wgsl)
    }

    /// The visual inventory's reading.
    pub fn inventory(&self) -> &Inventory {
        &self.inventory
    }

    /// What could not be read, in the order it was found.
    pub fn refusals(&self) -> &[voxel_model::refusal::Refusal] {
        self.refusals.all()
    }

    /// Whether every crate was measured and every inventory row witnessed.
    pub fn verified(&self) -> bool {
        self.refusals.is_empty() && self.inventory.refusals.is_empty()
    }

    /// The report as lines: one per crate, the total, the inventory, then each finding.
    pub fn lines(&self) -> Vec<String> {
        let mut lines = Vec::with_capacity(self.rows.len() + 4 + self.inventory.unrendered.len());
        for row in &self.rows {
            lines.push(format!(
                "balance\trow\t{}\trust\t{}\t{}\twgsl\t{}\t{}\tratio\t{}",
                row.crate_.name(),
                row.rust.files,
                row.rust.lines,
                row.wgsl.files,
                row.wgsl.lines,
                ratio(row.ratio())
            ));
        }
        lines.push(format!(
            "balance\ttotal\trust\t{}\t{}\twgsl\t{}\t{}\tratio\t{}",
            self.rust.files,
            self.rust.lines,
            self.wgsl.files,
            self.wgsl.lines,
            ratio(self.ratio())
        ));
        lines.push(format!(
            "balance\tinventory\titems\t{}\trendered\t{}\tunrendered\t{}",
            self.inventory.total,
            self.inventory.rendered,
            self.inventory.unrendered.len()
        ));
        for item in &self.inventory.unrendered {
            let renderer = item.renderer.map(|renderer| renderer.describe());
            lines.push(format!(
                "balance\tunrendered\t{}\t{}\t{}",
                item.name,
                item.site,
                renderer.unwrap_or_else(|| "no renderer names it".to_string())
            ));
        }
        for refusal in self.refusals.all() {
            lines.push(format!(
                "balance\trefusal\t{:?}\t{}",
                refusal.kind(),
                refusal.detail()
            ));
        }
        for refusal in &self.inventory.refusals {
            lines.push(format!(
                "balance\tinventory-refusal\t{}\t{:?}\t{}",
                refusal.item, refusal.kind, refusal.detail
            ));
        }
        lines
    }
}

fn ratio(value: Option<f64>) -> String {
    match value {
        Some(value) => format!("{value:.2}:1"),
        None => "n/a".to_string(),
    }
}

/// **Read the tree.** See [`crate::read`], which is this function's public name.
pub fn balance(root: &Path) -> Report {
    let mut refusals = RefusalSet::new();
    if !root.join("Cargo.toml").is_file() {
        refusals.refuse(
            RefusalKind::NoReading,
            format!(
                "{} has no Cargo.toml, so there is no workspace to measure. A balance of nothing \
                 is not a balance of zero",
                root.display()
            ),
        );
    }

    let mut rows = Vec::with_capacity(CRATE_COUNT);
    let mut rust = Measure::ZERO;
    let mut wgsl = Measure::ZERO;
    for crate_ in Crate::ALL {
        let directory = root.join(crate_.path());
        if !directory.is_dir() {
            refusals.refuse(
                RefusalKind::MissingFeature,
                format!(
                    "{} is in the census and {} is not in the tree",
                    crate_.name(),
                    crate_.path()
                ),
            );
        }
        let row = Row {
            crate_,
            rust: Measure::of_dir(root, &format!("{}/src", crate_.path()), "rs"),
            wgsl: Measure::of_dir(root, &format!("{}/src", crate_.path()), "wgsl"),
        };
        rust = rust.add(row.rust);
        wgsl = wgsl.add(row.wgsl);
        rows.push(row);
    }

    Report { rows, rust, wgsl, inventory: check(root, &INVENTORY), refusals }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn real_root() -> PathBuf {
        voxel_package::features::workspace_root(Path::new(env!("CARGO_MANIFEST_DIR")))
            .expect("this crate sits inside the workspace")
    }

    #[test]
    fn the_lines_name_every_crate_the_total_and_every_unrendered_item() {
        let report = balance(&real_root());
        let lines = report.lines();
        for crate_ in Crate::ALL {
            assert!(
                lines.iter().any(|line| line.contains(&format!("\t{}\t", crate_.name()))),
                "{} has no line",
                crate_.name()
            );
        }
        let inventory_at = lines
            .iter()
            .position(|line| line.starts_with("balance\tinventory"))
            .expect("the inventory line is in the report");
        assert!(
            lines[inventory_at + 1..]
                .iter()
                .all(|line| line.starts_with("balance\tunrendered")),
            "every finding follows the count it belongs to: {lines:#?}"
        );
        let unrendered = lines
            .iter()
            .filter(|line| line.starts_with("balance\tunrendered"))
            .count();
        assert_eq!(unrendered, report.inventory().unrendered.len());
    }

    #[test]
    fn a_directory_the_census_claims_and_the_tree_does_not_have_is_missing_feature() {
        let scratch = std::env::temp_dir().join(format!(
            "voxel-axis-balance-report-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&scratch);
        std::fs::create_dir_all(&scratch).expect("a scratch tree");
        std::fs::write(scratch.join("Cargo.toml"), "[workspace]\nmembers = []\n").expect("a manifest");
        let report = balance(&scratch);
        assert!(!report.verified());
        assert!(
            report.refusals().iter().any(|refusal| {
                refusal.kind() == RefusalKind::MissingFeature
                    && refusal.detail().contains("voxel-model")
            }),
            "{:#?}",
            report.refusals()
        );
        let _ = std::fs::remove_dir_all(&scratch);
    }
}
