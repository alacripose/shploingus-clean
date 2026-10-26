//! **The recipe line: six fields a person can read and an agent can apply.**
//!
//! This is the crate's output, and the user named its shape on 2026-10-21: *"human readable code in
//! the recipe format that can be used to fix the game's code"*. The format is the patch record's own
//! worked example, kept verbatim:
//!
//! ```text
//! # segment=world axis=evidence_grade star=★★★☆☆ tier=silver ticket=T-0954
//! fix      crates/voxel-model/src/world/book.rs:212  add a doc comment for `pub fn page_for`
//!    because  missing-docs (voxel-model: 1 of 954) and C-9 §9.2 says a public item states its contract
//!    check    cargo check -p voxel-model -> warnings 954 -> 953
//!    witness  conformance/missing-docs.txt
//! ```
//!
//! # Why each field is a gate rather than a convention
//!
//! | field | the rule it carries |
//! |---|---|
//! | `segment` / `axis` / `star` / `tier` / `ticket` | the cell this line belongs to, and the ticket it is the act of (`PATCH-042` C2) |
//! | `fix` | **what to type**, at a `file:line` that exists — [`Site::locate`] reads the file and counts its lines |
//! | `because` | why, in words with the numbers in them, because "fix the warning" is not a reason |
//! | `check` | **the command and the movement it must show** — [`Recipe::verify`] refuses a claim the compiler did not confirm (`PATCH-042` C4) |
//! | `witness` | where the measurement lives afterwards, so the next reader does not re-derive it |
//!
//! # A recipe is a ticket's act
//!
//! [`Recipe::cite`] folds the chain for a [`EventKind::TicketTransition`] whose detail names the
//! ticket. No event, no recipe: *"a recipe a ticket did not ask for is a sentence someone typed"*, and
//! the chain is where the asking is recorded (H-8.1, one chain). The fold is used rather than a slice
//! because `Chain` deliberately hands out no events — C-54.1's long-file rule.
//!
//! # Applying is not this module's job
//!
//! `recipe::compile` in `voxel-package` already turns a package into bytes, and the mount already
//! applies a `Loaded` under a ticket-scoped grant. This module **states** the line and checks the
//! three things that can be checked without writing: the site exists, the ticket exists, the receipt
//! moved. The act itself stays where the grant is.

use std::path::Path;

use voxel_model::axis::CertificationTier;
use voxel_model::chain::{Chain, EventKind};
use voxel_model::refusal::RefusalSet;

use crate::{Cell, MatrixRefusal, Segment, Star};

/// A place in the tree: a file and a line. **Both parts are required** — a path with no line names a
/// file, and a line with no path names nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Site {
    /// Workspace-relative path, exactly as a reader would open it.
    pub path: String,
    /// One-based line number.
    pub line: u32,
}

impl Site {
    /// A site. Nothing is checked here; [`Site::locate`] is the check, because a constructor that
    /// touched the filesystem could not be used to describe a site that is about to exist.
    pub fn new(path: impl Into<String>, line: u32) -> Self {
        Site {
            path: path.into(),
            line,
        }
    }

    /// The site as one word: `crates/voxel-model/src/lib.rs:12`.
    pub fn render(&self) -> String {
        format!("{}:{}", self.path, self.line)
    }

    /// **Read the file and count its lines.** `PATCH-042` C2: a line that points at nothing is a line
    /// nobody can apply, and the refusal carries both numbers so the reader knows how far off it is.
    pub fn locate(&self, root: &Path) -> Result<(), MatrixRefusal> {
        if self.line == 0 {
            return Err(MatrixRefusal::Incomplete {
                field: "site line (lines are one-based)",
            });
        }
        let text = match std::fs::read_to_string(root.join(&self.path)) {
            Ok(text) => text,
            Err(_) => {
                return Err(MatrixRefusal::SiteAbsent {
                    path: self.path.clone(),
                    line: self.line,
                    lines: 0,
                })
            }
        };
        let lines = text.lines().count();
        if self.line as usize > lines {
            return Err(MatrixRefusal::SiteAbsent {
                path: self.path.clone(),
                line: self.line,
                lines,
            });
        }
        Ok(())
    }
}

/// The edit a recipe line describes, and what the check must show afterwards.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fix {
    /// Where to type.
    pub site: Site,
    /// What to type, in one line.
    pub what: String,
    /// Why, with the numbers in it.
    pub why: String,
    /// The command, and the movement it must print.
    pub check: String,
    /// Where the measurement is kept.
    pub witness: String,
    /// **The count the check must reach** after the edit. `PATCH-042` C4.
    pub expects: u32,
}

impl Fix {
    /// An edit and the count it must produce.
    pub fn new(
        site: Site,
        what: impl Into<String>,
        why: impl Into<String>,
        check: impl Into<String>,
        witness: impl Into<String>,
        expects: u32,
    ) -> Self {
        Fix {
            site,
            what: what.into(),
            why: why.into(),
            check: check.into(),
            witness: witness.into(),
            expects,
        }
    }
}

/// One recipe line: the cell's five facts plus the edit's four.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recipe {
    /// The segment the cell belongs to.
    pub segment: Segment,
    /// The cell's axis, by its record.
    pub axis: voxel_model::axis::AxisKind,
    /// The star the cell holds when this line was rendered.
    pub star: Star,
    /// The certification tier the cell's requirements hold.
    pub tier: CertificationTier,
    /// The ticket this line is the act of.
    pub ticket: String,
    /// The edit.
    pub fix: Fix,
}

impl Recipe {
    /// **The only constructor that checks.** A cell with no witness, a star its predicates do not
    /// light, or an empty ticket refuses here — so a recipe that exists is a recipe that was verified
    /// against its own cell before it was drawn ([`Cell::check`], [`crate::star::require`]).
    pub fn of(cell: &Cell, fix: Fix) -> Result<Recipe, RefusalSet> {
        let mut refusals = RefusalSet::new();
        if let Err(refusal) = cell.check() {
            refusals.refuse(refusal.kind(), refusal.detail());
        }
        let tier = match cell.tier() {
            Some(tier) => tier,
            None => {
                refusals.refuse(
                    MatrixRefusal::StarUnsupported {
                        claimed: cell.star.name(),
                        detail: format!(
                            "the cell holds no certification tier: none of {:?} is satisfied, and an \
                             absent tier is a state rather than bronze",
                            cell.requirements
                        ),
                    }
                    .kind(),
                    format!(
                        "{}/{}: the cell's requirements hold no certification tier",
                        cell.segment.name(),
                        cell.reading.axis.name()
                    ),
                );
                CertificationTier::Bronze
            }
        };
        refusals.into_result(Recipe {
            segment: cell.segment,
            axis: cell.reading.axis,
            star: cell.star,
            tier,
            ticket: cell.ticket.clone(),
            fix,
        })
    }

    /// The line, as text. **One statement per field, and no field is optional.**
    pub fn render(&self) -> Result<String, MatrixRefusal> {
        for (field, value) in [
            ("fix", self.fix.what.as_str()),
            ("because", self.fix.why.as_str()),
            ("check", self.fix.check.as_str()),
            ("witness", self.fix.witness.as_str()),
            ("ticket", self.ticket.as_str()),
            ("site path", self.fix.site.path.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(MatrixRefusal::Incomplete { field });
            }
        }
        if self.fix.site.line == 0 {
            return Err(MatrixRefusal::Incomplete {
                field: "site line (lines are one-based)",
            });
        }
        Ok(format!(
            "# segment={} axis={} star={} tier={} ticket={}\n\
             fix      {}  {}\n\
             \x20  because  {}\n\
             \x20  check    {}\n\
             \x20  witness  {}",
            self.segment.name(),
            self.axis.name(),
            self.star,
            self.tier.as_str(),
            self.ticket,
            self.fix.site.render(),
            self.fix.what,
            self.fix.why,
            self.fix.check,
            self.fix.witness,
        ))
    }

    /// **No invented ticket.** The chain is folded for a ticket transition whose detail names this
    /// ticket; a recipe with no such event is refused and the ticket is quoted.
    pub fn cite(&self, chain: &Chain) -> Result<(), MatrixRefusal> {
        let cited = chain.fold(false, |found, event| {
            found
                || (event.kind == EventKind::TicketTransition
                    && event.detail.contains(self.ticket.as_str()))
        });
        if cited {
            Ok(())
        } else {
            Err(MatrixRefusal::UncitedTicket {
                ticket: self.ticket.clone(),
            })
        }
    }

    /// **The receipt is the compiler.** A count that rose, or did not move, refuses the star the cell
    /// claims — and the refusal prints both numbers.
    pub fn verify(&self, measured: u32) -> Result<(), MatrixRefusal> {
        if measured > self.fix.expects {
            return Err(MatrixRefusal::CountDidNotFall {
                claimed: self.fix.expects,
                measured,
            });
        }
        Ok(())
    }

    /// The site exists, the ticket is on the chain, and the count moved. **The three checks a line can
    /// pass without being applied**, in one call so a caller cannot run two of them and call it done.
    pub fn check(&self, root: &Path, chain: &Chain, measured: u32) -> Result<(), RefusalSet> {
        let mut refusals = RefusalSet::new();
        if let Err(refusal) = self.fix.site.locate(root) {
            refusals.refuse(refusal.kind(), refusal.detail());
        }
        if let Err(refusal) = self.cite(chain) {
            refusals.refuse(refusal.kind(), refusal.detail());
        }
        if let Err(refusal) = self.verify(measured) {
            refusals.refuse(refusal.kind(), refusal.detail());
        }
        refusals.into_result(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Predicate, Reading};
    use voxel_model::axis::{AxisKind, CertificationTier, EvidenceGrade};
    use voxel_model::chain::{Namespace, Season};

    /// `crates/axis/matrix` → the workspace root is three levels up, and the test reads the same
    /// root a caller would, rather than a fixture that could pass while the tree is unreachable.
    fn root() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
    }

    fn cell_with_papers() -> Cell {
        Cell::new(
            Segment::World,
            Reading::of_ladder(EvidenceGrade::B, "read back from the file on disk")
                .witnessed("conformance/missing-docs.txt"),
            Star::Three,
            vec![
                Predicate::held("site_exists"),
                Predicate::held("ticket_filed"),
                Predicate::held("count_fell"),
            ],
            "T-0954",
            "PATCH-042",
        )
        .with_requirements(vec!["declared_provenance", "evidence_present"])
    }

    fn fix() -> Fix {
        Fix::new(
            Site::new("crates/voxel-model/src/lib.rs", 1),
            "add a doc comment for the module header",
            "missing-docs: voxel-model reports 954 warnings and this line closes one",
            "cargo check -p voxel-model -> warnings 954 -> 953",
            "conformance/missing-docs.txt",
            953,
        )
    }

    fn chain_citing(ticket: &str) -> Chain {
        let mut chain = Chain::new();
        let namespace = Namespace::new("shploingus", 1, Season::Summer, "warnings").unwrap();
        chain
            .append(namespace, EventKind::TicketTransition, format!("filed {ticket}"))
            .expect("a ticket transition appends");
        chain
    }

    #[test]
    fn a_recipe_renders_six_readable_fields_and_a_header() {
        let recipe = Recipe::of(&cell_with_papers(), fix()).expect("the cell holds three stars");
        let text = recipe.render().expect("every field is present");
        assert!(text.starts_with("# segment=world axis=evidence_grade star=★★★☆☆ tier=silver ticket=T-0954"), "got {text}");
        assert!(text.contains("fix      crates/voxel-model/src/lib.rs:1  add a doc comment"), "got {text}");
        assert!(text.contains("because  missing-docs"), "got {text}");
        assert!(text.contains("check    cargo check -p voxel-model -> warnings 954 -> 953"), "got {text}");
        assert!(text.contains("witness  conformance/missing-docs.txt"), "got {text}");
        assert_eq!(text.lines().count(), 5, "one header and four statements: {text}");
    }

    #[test]
    fn a_missing_field_is_refused_by_its_own_name() {
        let mut broken = Recipe::of(&cell_with_papers(), fix()).unwrap();
        broken.fix.witness = "  ".to_string();
        let refused = broken.render().expect_err("no witness");
        assert!(refused.detail().contains("witness"), "got {refused}");
        broken.fix.witness = "conformance/missing-docs.txt".to_string();
        broken.ticket.clear();
        let refused = broken.render().expect_err("no ticket");
        assert!(refused.detail().contains("ticket"), "got {refused}");
        broken.ticket = "T-0954".to_string();
        broken.fix.site.line = 0;
        let refused = broken.render().expect_err("line zero is not a line");
        assert!(refused.detail().contains("one-based"), "got {refused}");
    }

    #[test]
    fn a_site_that_exists_and_reaches_its_line_locates() {
        Site::new("crates/voxel-model/src/lib.rs", 1)
            .locate(&root())
            .expect("line one exists");
        Site::new("Cargo.toml", 2).locate(&root()).expect("the manifest has lines");
    }

    #[test]
    fn a_site_beyond_the_file_names_both_numbers() {
        let refused = Site::new("crates/voxel-model/src/lib.rs", 100_000)
            .locate(&root())
            .expect_err("the file is shorter than that");
        assert!(refused.detail().contains("100000"), "got {refused}");
        assert!(refused.detail().contains(" lines"), "got {refused}");
    }

    #[test]
    fn a_site_in_a_file_that_does_not_exist_is_refused_with_zero_lines() {
        let refused = Site::new("crates/nope/src/lib.rs", 1)
            .locate(&root())
            .expect_err("no such file");
        assert!(refused.detail().contains("0 lines"), "got {refused}");
        assert!(refused.detail().contains("crates/nope/src/lib.rs"), "got {refused}");
    }

    #[test]
    fn a_recipe_with_no_ticket_on_the_chain_is_refused() {
        let recipe = Recipe::of(&cell_with_papers(), fix()).unwrap();
        recipe.cite(&chain_citing("T-0954")).expect("the chain records it");
        let refused = recipe.cite(&Chain::new()).expect_err("an empty chain cites nothing");
        assert!(refused.detail().contains("T-0954"), "got {refused}");
        assert_eq!(
            refused.kind(),
            voxel_model::refusal::RefusalKind::UnsubstantiatedEvidence
        );
    }

    #[test]
    fn a_ticket_that_was_only_a_refusal_does_not_cite_it() {
        let mut chain = Chain::new();
        let namespace = Namespace::new("shploingus", 1, Season::Autumn, "warnings").unwrap();
        chain
            .append(namespace, EventKind::RefusalRecorded, "T-0954 refused")
            .unwrap();
        let recipe = Recipe::of(&cell_with_papers(), fix()).unwrap();
        assert!(
            recipe.cite(&chain).is_err(),
            "a refusal is not a ticket's transition"
        );
    }

    #[test]
    fn the_receipt_is_the_compiler_and_a_count_that_did_not_fall_is_refused() {
        let recipe = Recipe::of(&cell_with_papers(), fix()).unwrap();
        recipe.verify(953).expect("the count reached the claim");
        recipe.verify(900).expect("falling further than claimed is still a fall");
        let refused = recipe.verify(954).expect_err("the count did not move");
        assert!(refused.detail().contains("953"), "got {refused}");
        assert!(refused.detail().contains("954"), "got {refused}");
    }

    #[test]
    fn the_three_checks_run_together() {
        let recipe = Recipe::of(&cell_with_papers(), fix()).unwrap();
        recipe
            .check(&root(), &chain_citing("T-0954"), 953)
            .expect("site, ticket and count all hold");
        let refusals = recipe
            .check(&root(), &Chain::new(), 954)
            .expect_err("two of the three fail");
        let kinds: Vec<&str> = refusals.all().iter().map(|refusal| refusal.kind().name()).collect();
        assert!(kinds.contains(&"unsubstantiated_evidence"), "got {kinds:?}");
        assert!(refusals.all().len() >= 2, "both failures are reported: {kinds:?}");
    }

    #[test]
    fn a_cell_with_no_tier_refuses_to_become_a_recipe() {
        let cell = Cell::new(
            Segment::Ui,
            Reading::of_ladder(EvidenceGrade::C, "self report").witnessed("docs/UI-SPEC.md"),
            Star::One,
            vec![Predicate::held("declared")],
            "T-1000",
            "PATCH-042",
        );
        let refused = Recipe::of(&cell, fix()).expect_err("no requirement is satisfied");
        assert!(
            refused.all().iter().any(|refusal| refusal.detail().contains("certification tier")),
            "got {:?}",
            refused.all()
        );
        // The axis is carried from the cell, not from the fix.
        let with_tier = cell.with_requirements(vec!["declared_provenance"]);
        let recipe = Recipe::of(&with_tier, fix()).expect("bronze is held");
        assert_eq!(recipe.tier, CertificationTier::Bronze);
        assert_eq!(recipe.axis, AxisKind::EvidenceGrade);
    }
}
