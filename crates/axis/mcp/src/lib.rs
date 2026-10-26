//! **The axis MCP layer: the authority an agent-facing surface speaks through.**
//!
//! `crates/voxel-mcp` is a JSON-RPC server — a *surface*. This crate is the layer underneath it,
//! and the reason it is a library rather than a second server: two servers would be two places a
//! tool is declared, two places a refusal is phrased and two places a ticket is honoured. The
//! server, the console, and anything else that wants to put a ticket in front of an agent calls
//! [`Registry`].
//!
//! # Three acts, and the fourth that is not one
//!
//! | act | what it answers | refused when |
//! |---|---|---|
//! | [`Registry::read`] | the verified matrix and its package, over this tree, **without a key** | the walk declined a segment, or the package will not build |
//! | [`Registry::mount`] | the same reading, holding the key that signs and verifies grants | the walk declined a segment, or the package will not build |
//! | [`Registry::recipe`] | the package line for a named ticket and cell | the ticket is not the one measured, the cell is not measured, or the cell names no witness |
//! | [`Registry::grant`] | a [`TicketGrant`] signed by the security layer | the ticket is unnamed (`voxel_model::grant::Grant` refuses it) |
//! | [`Registry::apply`] | **a decision** that this act is authorised, and the checks that ran | no credential, a foreign key, a widened grant, another ticket, another cell, an expired grant, an unwitnessed cell |
//!
//! **`apply` performs nothing.** It is the same shape as `voxel_package::bridge::call`: it decides,
//! names the checks it ran, and returns a receipt. A layer that also wrote the edit would be a second
//! place the act is implemented, and the act belongs to the surface that holds the file.
//!
//! # What is reused rather than restated
//!
//! The refusals are [`voxel_model::refusal::RefusalKind`]; the grant's attribution is
//! [`voxel_model::grant::Grant`]; the segments and cells are `voxel_axis_matrix`'s; the reading is
//! `voxel_axis_verify`'s, produced by the one crate allowed to produce it; the citation and the
//! statement are `voxel_axis_artifact`'s [`Line`]; the signature is `voxel_axis_security`'s. This
//! crate declares **no** refusal vocabulary and **no** axis vocabulary of its own — adding one would
//! make it a second authority.
//!
//! # The unverified matrix
//!
//! [`Registry::of`] exists because a surface may want to serve cells another producer measured. It
//! builds no package, and every act that needs one [`RefusalKind::NoReading`]. The shipped path is
//! [`Registry::mount`], which refuses a matrix with a decline instead of shortening the table.
//!
//! **The wire is the grant's own.** [`TicketGrant::to_wire`] writes one tab-separated line and
//! [`TicketGrant::from_wire`] reads it back; parsing never verifies, and [`Registry::apply`] verifies
//! before any field is used. `voxel-mcp` serves this layer as four tools, so an agent reaches the same
//! registry the shell does rather than a second copy of its rules.
//!
//! # Where its rules are recorded
//!
//! `PATCH-044` §The authority layer names this crate, the three acts, and the refusal each act
//! raises.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod grant;

pub use grant::TicketGrant;

use std::path::Path;

use voxel_axis_artifact::{Line, Package};
use voxel_axis_matrix::{Cell, Matrix, Segment};
use voxel_axis_security::Security;
use voxel_axis_verify::verify;
use voxel_model::axis::AxisKind;
use voxel_model::refusal::{RefusalKind, RefusalSet};

/// The checks [`Registry::apply`] runs, in the order it runs them. A receipt names the ones that
/// passed; the first that fails is the refusal, so a receipt that exists is all six.
pub const CHECKS: [&str; 6] = ["credential", "signature", "ticket", "cell", "expiry", "witness"];

/// **An authority over one ticket: the verified matrix, its package, and the key that signs grants.**
///
/// It holds no state of its own — the cells are a projection, the package is a document, and the key
/// lives where the security crate put it. Nothing here opens a file for writing.
#[derive(Clone, Debug)]
pub struct Registry {
    ticket: String,
    /// The key that signs and verifies grants. **Absent for a read-only registry**, so a surface
    /// that only reads the matrix never mints a key as a side effect of reading.
    security: Option<Security>,
    cells: Vec<Cell>,
    matrix: Matrix,
    package: Option<Package>,
}

impl Registry {
    /// **The shipped path.** Walk the tree, require every segment to be witnessed, build the package
    /// over the cells the walk produced, and serve both.
    ///
    /// A decline is not skipped: a matrix with one declined segment has fewer cells than segments, and
    /// [`Package::of`] would build a document that looks complete. So the decline is the refusal here,
    /// naming the sources that could not witness it.
    pub fn mount(root: &Path, ticket: &str, security: Security) -> Result<Registry, RefusalSet> {
        let mut registry = Registry::read(root, ticket)?;
        registry.security = Some(security);
        Ok(registry)
    }

    /// **The same walk, without a key.** A surface that only serves the matrix and its package has
    /// no business minting a signing key to do it: this registry reads, and [`Registry::grant`] and
    /// [`Registry::apply`] refuse [`RefusalKind::NoSigningBackend`] naming that it holds no signer.
    pub fn read(root: &Path, ticket: &str) -> Result<Registry, RefusalSet> {
        let report = verify(root, ticket);
        let mut refusals = RefusalSet::new();
        for decline in report.declines() {
            refusals.refuse(
                RefusalKind::UncalibratedOracle,
                format!(
                    "the walk declined {} and an unwitnessed segment is not a cell: {}",
                    decline.segment.name(),
                    decline.detail
                ),
            );
        }
        // A gate that failed with no decline is still a refusal: an empty set would be an error with
        // no reason, which a caller cannot print or act on.
        if !report.verified() && refusals.all().is_empty() {
            refusals.refuse(
                RefusalKind::UncalibratedOracle,
                format!(
                    "the matrix's own gate refused this walk: {}",
                    report
                        .matrix()
                        .verify()
                        .err()
                        .map(|error| error.to_string())
                        .unwrap_or_else(|| "no cell was measured".to_string())
                ),
            );
        }
        if !refusals.all().is_empty() {
            return Err(refusals);
        }
        let cells = report.cells().to_vec();
        let package = Package::of(root, ticket, &cells)?;
        if package.lines().len() != cells.len() {
            return Err(refusal(
                RefusalKind::NoReading,
                format!(
                    "the package has {} line(s) and the matrix has {} cell(s): a recipe is the \
                     package's line for the cell it names, and these are not the same table",
                    package.lines().len(),
                    cells.len()
                ),
            ));
        }
        Ok(Registry::assemble(ticket, None, cells, Some(package)))
    }

    /// **A registry over the cells and the package a surface already holds** — for a console that
    /// printed the document before asking to act on it. The package must be the one built from these
    /// cells, one line each in cell order, because [`Registry::recipe`] is indexed by that.
    pub fn over(
        ticket: &str,
        security: Security,
        cells: Vec<Cell>,
        package: Package,
    ) -> Result<Registry, RefusalSet> {
        if package.lines().len() != cells.len() {
            return Err(refusal(
                RefusalKind::NoReading,
                format!(
                    "the package has {} line(s) and the matrix has {} cell(s): a recipe is the \
                     package's line for the cell it names, and these are not the same table",
                    package.lines().len(),
                    cells.len()
                ),
            ));
        }
        Ok(Registry::assemble(ticket, Some(security), cells, Some(package)))
    }

    /// **A registry over cells someone else measured.** No package is built, so [`Registry::recipe`]
    /// refuses; [`Registry::apply`] still runs its own checks, which is why it can refuse an
    /// unwitnessed cell that never reached [`Registry::mount`].
    pub fn of(ticket: &str, security: Security, cells: Vec<Cell>) -> Registry {
        Registry::assemble(ticket, Some(security), cells, None)
    }

    /// The one constructor: the matrix is assembled from the cells and the package is carried, or
    /// not. Private, so a caller cannot pair a key with cells it did not measure.
    fn assemble(
        ticket: &str,
        security: Option<Security>,
        cells: Vec<Cell>,
        package: Option<Package>,
    ) -> Registry {
        Registry {
            ticket: ticket.to_string(),
            security,
            cells: cells.clone(),
            matrix: Matrix::assemble(cells),
            package,
        }
    }

    /// The ticket this registry was measured under.
    pub fn ticket(&self) -> &str {
        &self.ticket
    }

    /// The matrix, cells included — the projection a reader draws.
    pub fn matrix(&self) -> &Matrix {
        &self.matrix
    }

    /// The package, when this registry was mounted. `of` serves none, and says so.
    pub fn package(&self) -> Option<&Package> {
        self.package.as_ref()
    }

    /// **The recipe for a named ticket: the package line for the cell it was measured at.**
    ///
    /// The ticket must be the one the matrix was measured under; a recipe for a cell that is not the
    /// one measured is [`RefusalKind::NoReading`] naming, and an unwitnessed cell is
    /// [`RefusalKind::UncalibratedOracle`] — a recipe is an act, and an unchecked cell is not a
    /// reading to act on.
    pub fn recipe(
        &self,
        ticket: &str,
        segment: Segment,
        axis: AxisKind,
    ) -> Result<&Line, RefusalSet> {
        let mut refusals = RefusalSet::new();
        let Some(package) = self.package.as_ref() else {
            refusals.refuse(
                RefusalKind::NoReading,
                "this registry serves no package: it was built from cells without a walk, and a \
                 recipe is a package line"
                    .to_string(),
            );
            return Err(refusals);
        };
        if ticket != self.ticket {
            refusals.refuse(
                RefusalKind::NoReading,
                format!(
                    "the matrix was measured for {} and this request names {ticket}: a recipe for a \
                     cell that is not the one measured",
                    self.ticket
                ),
            );
            return Err(refusals);
        }
        let Some(cell) = self.matrix.cell(segment, axis) else {
            refusals.refuse(
                RefusalKind::NoReading,
                format!(
                    "no measured cell for {}/{}, so there is no recipe to hand out",
                    segment.name(),
                    axis.name()
                ),
            );
            return Err(refusals);
        };
        if !cell.is_checked() {
            refusals.refuse(
                RefusalKind::UncalibratedOracle,
                format!(
                    "the cell at {}/{} names no witness, and a recipe for an unwitnessed cell is a \
                     guess with a citation",
                    segment.name(),
                    axis.name()
                ),
            );
            return Err(refusals);
        }
        let index = self
            .cells
            .iter()
            .position(|candidate| candidate.key() == (segment, axis))
            .expect("the matrix is a projection over these cells");
        Ok(&package.lines()[index])
    }

    /// **Issue a signed grant** for one ticket on one cell, good until `expires`. The ticket is not
    /// required to be this registry's: an authority signs what it is asked for, and [`Registry::apply`]
    /// is where a grant for another ticket meets the act it does not authorise. An already-expired
    /// bound can be issued for the same reason — the refusal belongs to the act, not the signature.
    pub fn grant(
        &self,
        ticket: &str,
        segment: Segment,
        axis: AxisKind,
        expires: u64,
    ) -> Result<TicketGrant, RefusalSet> {
        let Some(security) = self.security.as_ref() else {
            return Err(refusal(
                RefusalKind::NoSigningBackend,
                "this registry was mounted for reading and holds no signer; a grant is signed, so \
                 a registry with no key cannot issue one"
                    .to_string(),
            ));
        };
        TicketGrant::issue(security, ticket, segment, axis, expires)
    }

    /// **Decide whether this act is authorised**, and return a receipt naming the checks that ran.
    ///
    /// Six checks, in [`CHECKS`] order, and the first that fails is the refusal:
    ///
    /// 1. a credential was presented — silence is not permission ([`RefusalKind::UnsignedEnvelope`]);
    /// 2. its signature holds for a key this authority holds ([`RefusalKind::UntrustedKey`],
    ///    [`RefusalKind::SignatureInvalid`] — the security crate's own refusals);
    /// 3. the grant names this ticket ([`RefusalKind::UntrustedKey`]);
    /// 4. the grant names this cell ([`RefusalKind::UntrustedKey`]);
    /// 5. the act is before the grant's expiry ([`RefusalKind::DestinationInvariantRefused`] — a move
    ///    past a bound the grant itself declared);
    /// 6. the cell was measured and names a witness ([`RefusalKind::NoReading`],
    ///    [`RefusalKind::UncalibratedOracle`]).
    ///
    /// **Nothing is executed by this function: it decides.** The surface that holds the file performs
    /// the edit, and this receipt is the authority it holds while doing so.
    pub fn apply(
        &self,
        credential: Option<&TicketGrant>,
        ticket: &str,
        segment: Segment,
        axis: AxisKind,
        now: u64,
    ) -> Result<Applied, RefusalSet> {
        let mut checks: Vec<&'static str> = Vec::with_capacity(CHECKS.len());
        let Some(grant) = credential else {
            return Err(refusal(
                RefusalKind::UnsignedEnvelope,
                "no grant was presented: an act with no credential is unauthorised, and silence is \
                 not permission"
                    .to_string(),
            ));
        };
        checks.push(CHECKS[0]);

        let Some(security) = self.security.as_ref() else {
            return Err(refusal(
                RefusalKind::NoSigningBackend,
                "this registry was mounted for reading and holds no verifier; a signature nobody \
                 checked is not a credential"
                    .to_string(),
            ));
        };
        grant.verify(security)?;
        checks.push(CHECKS[1]);

        if grant.ticket() != ticket {
            return Err(refusal(
                RefusalKind::UntrustedKey,
                format!(
                    "the grant is for {} and this act is on {ticket}: a ticket-scoped grant \
                     authorizes the ticket it names and no other",
                    grant.ticket()
                ),
            ));
        }
        checks.push(CHECKS[2]);

        if grant.cell() != (segment, axis) {
            return Err(refusal(
                RefusalKind::UntrustedKey,
                format!(
                    "the grant is scoped to {}/{} and this act is on {}/{}",
                    grant.cell().0.name(),
                    grant.cell().1.name(),
                    segment.name(),
                    axis.name()
                ),
            ));
        }
        checks.push(CHECKS[3]);

        if now >= grant.expires() {
            return Err(refusal(
                RefusalKind::DestinationInvariantRefused,
                format!(
                    "the grant expired at tick {} (exclusive) and this act is at tick {now}: a grant \
                     used past its expiry is a move on a terminal unit",
                    grant.expires()
                ),
            ));
        }
        checks.push(CHECKS[4]);

        let Some(cell) = self.matrix.cell(segment, axis) else {
            return Err(refusal(
                RefusalKind::NoReading,
                format!(
                    "no cell was measured at {}/{} for {ticket}, so there is nothing to apply to",
                    segment.name(),
                    axis.name()
                ),
            ));
        };
        if !cell.is_checked() {
            return Err(refusal(
                RefusalKind::UncalibratedOracle,
                format!(
                    "the cell at {}/{} names no witness: applying on an unwitnessed cell is acting \
                     on a reading nobody took",
                    segment.name(),
                    axis.name()
                ),
            ));
        }
        checks.push(CHECKS[5]);

        Ok(Applied {
            ticket: ticket.to_string(),
            segment,
            axis,
            checks,
        })
    }
}

/// **A receipt: this act is authorised, and here is what was checked.** It is not the act.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Applied {
    ticket: String,
    segment: Segment,
    axis: AxisKind,
    checks: Vec<&'static str>,
}

impl Applied {
    /// The ticket the act is on.
    pub fn ticket(&self) -> &str {
        &self.ticket
    }

    /// The cell the act is on.
    pub fn cell(&self) -> (Segment, AxisKind) {
        (self.segment, self.axis)
    }

    /// The checks that ran, in order. A receipt exists only when all of [`CHECKS`] passed.
    pub fn checks(&self) -> &[&'static str] {
        &self.checks
    }

    /// The receipt as one console line.
    pub fn render(&self) -> String {
        format!(
            "apply permitted on {} at {}/{} — checks: {}",
            self.ticket,
            self.segment.name(),
            self.axis.name(),
            self.checks.join(" · ")
        )
    }
}

/// One refusal, in the tree's own vocabulary.
fn refusal(kind: RefusalKind, detail: String) -> RefusalSet {
    let mut refusals = RefusalSet::new();
    refusals.refuse(kind, detail);
    refusals
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use voxel_axis_matrix::{Predicate, Reading, Star};
    use voxel_model::axis::EvidenceGrade;
    use voxel_model::refusal::Refusal;

    const TICKET: &str = "T-0954";

    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(3)
            .expect("this crate sits at crates/axis/mcp inside the workspace")
            .to_path_buf()
    }

    /// A key in a scratch directory **outside** the checkout: the security crate refuses one inside it.
    fn security(tag: &str) -> (PathBuf, Security) {
        let scratch = std::env::temp_dir().join(format!(
            "voxel-axis-mcp-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&scratch);
        std::fs::create_dir_all(scratch.join("checkout")).expect("a scratch checkout");
        let security = Security::open(&scratch.join("checkout"), scratch.join("keys/signing.key"))
            .expect("a machine key");
        (scratch, security)
    }

    fn mounted(tag: &str) -> (PathBuf, Registry) {
        let (scratch, security) = security(tag);
        let registry = Registry::mount(&root(), TICKET, security).expect("the real tree mounts");
        (scratch, registry)
    }

    fn kinds(refused: &RefusalSet) -> Vec<RefusalKind> {
        refused.all().iter().map(Refusal::kind).collect()
    }

    fn one_cell(reading: Reading) -> Cell {
        Cell::new(
            Segment::Shell,
            reading,
            Star::One,
            vec![Predicate::held("the only row")],
            TICKET,
            "PATCH-042",
        )
        .with_requirements(vec!["recorded_reading"])
    }

    #[test]
    fn the_mounted_registry_serves_the_measured_matrix_package_and_recipe() {
        let (scratch, registry) = mounted("mount");
        assert_eq!(registry.matrix().len(), Segment::ALL.len() + 1);
        assert_eq!(
            registry.package().expect("a package").lines().len(),
            Segment::ALL.len() + 1,
            "seven segments and the artifact format"
        );

        let line = registry
            .recipe(TICKET, Segment::Shell, AxisKind::EvidenceGrade)
            .expect("the shell's cell is measured");
        assert!(line.statement().contains("shell"), "{}", line.statement());
        assert!(
            line.cite().path.starts_with("crates/"),
            "a citation is a path in this tree: {}",
            line.render()
        );
        assert!(registry.package().expect("a package").render().contains(TICKET));
        let _ = std::fs::remove_dir_all(&scratch);
    }

    #[test]
    fn a_keyless_registry_reads_the_matrix_but_refuses_to_sign_or_verify() {
        let (scratch, security) = security("reader");
        let registry = Registry::read(&root(), TICKET).expect("reading needs no key");
        assert_eq!(
            registry.matrix().len(),
            Segment::ALL.len() + 1,
            "seven segments and the artifact format"
        );
        assert!(
            registry.package().is_some(),
            "the package is part of the reading"
        );
        let refused = registry
            .grant(TICKET, Segment::Shell, AxisKind::EvidenceGrade, 40)
            .expect_err("no signer");
        assert_eq!(kinds(&refused), vec![RefusalKind::NoSigningBackend]);

        // A real credential in the hands of a reader that holds no verifier: the signature was not
        // checked, so it is not a credential, and the refusal says which fact is missing.
        let credential = TicketGrant::issue(&security, TICKET, Segment::Shell, AxisKind::EvidenceGrade, 100)
            .expect("a key issues it");
        let refused = registry
            .apply(Some(&credential), TICKET, Segment::Shell, AxisKind::EvidenceGrade, 0)
            .expect_err("no verifier");
        assert_eq!(kinds(&refused), vec![RefusalKind::NoSigningBackend]);
        let _ = std::fs::remove_dir_all(&scratch);
    }

    #[test]
    fn a_recipe_for_a_cell_that_was_not_measured_is_refused() {
        let (scratch, registry) = mounted("recipe-refused");
        assert_eq!(
            kinds(&registry
                .recipe("T-0000", Segment::Shell, AxisKind::EvidenceGrade)
                .expect_err("another ticket"))
                ,
            vec![RefusalKind::NoReading]
        );
        assert_eq!(
            kinds(&registry
                .recipe(TICKET, Segment::Shell, AxisKind::SupportLevel)
                .expect_err("no cell on the support ladder here"))
                ,
            vec![RefusalKind::NoReading]
        );
        let _ = std::fs::remove_dir_all(&scratch);
    }

    #[test]
    fn a_package_that_is_not_one_line_per_cell_is_refused_rather_than_indexed_past() {
        let (scratch, security) = security("short-package");
        let report = verify(&root(), TICKET);
        let cells = report.cells().to_vec();
        let count = cells.len();
        let short = Package::of(&root(), TICKET, &cells[..1]).expect("one cited line is a package");
        assert_eq!(short.lines().len(), 1);
        let refused = Registry::over(TICKET, security, cells, short)
            .expect_err(&format!("a package of one line cannot answer for {count} cells"));
        assert_eq!(kinds(&refused), vec![RefusalKind::NoReading]);
        let _ = std::fs::remove_dir_all(&scratch);
    }

    #[test]
    fn a_granted_apply_is_permitted_and_names_every_check() {
        let (scratch, registry) = mounted("apply");
        let grant = registry
            .grant(TICKET, Segment::Shell, AxisKind::EvidenceGrade, 100)
            .expect("a signed grant");
        let applied = registry
            .apply(Some(&grant), TICKET, Segment::Shell, AxisKind::EvidenceGrade, 40)
            .expect("inside the grant");
        assert_eq!(applied.checks(), CHECKS);
        assert!(applied.render().contains("apply permitted on T-0954"));
        let _ = std::fs::remove_dir_all(&scratch);
    }

    #[test]
    fn an_act_with_no_credential_is_refused_and_says_so() {
        let (scratch, registry) = mounted("no-credential");
        let refused = registry
            .apply(None, TICKET, Segment::Shell, AxisKind::EvidenceGrade, 0)
            .expect_err("no grant");
        assert_eq!(kinds(&refused), vec![RefusalKind::UnsignedEnvelope]);
        assert!(refused.all()[0].detail().contains("no grant was presented"));
        let _ = std::fs::remove_dir_all(&scratch);
    }

    #[test]
    fn a_grant_for_another_ticket_is_refused_even_with_a_good_signature() {
        let (scratch, registry) = mounted("other-ticket");
        let grant = registry
            .grant("T-0000", Segment::Shell, AxisKind::EvidenceGrade, 100)
            .expect("an authority signs what it is asked");
        grant
            .verify(registry.security.as_ref().expect("mounted with a key"))
            .expect("the signature itself holds");
        let refused = registry
            .apply(Some(&grant), TICKET, Segment::Shell, AxisKind::EvidenceGrade, 0)
            .expect_err("another ticket");
        assert_eq!(kinds(&refused), vec![RefusalKind::UntrustedKey]);
        assert!(
            refused.all()[0].detail().contains("T-0000")
                && refused.all()[0].detail().contains(TICKET),
            "both tickets are named: {}",
            refused.all()[0].detail()
        );
        let _ = std::fs::remove_dir_all(&scratch);
    }

    #[test]
    fn an_expired_grant_is_refused_at_the_boundary_it_declared() {
        let (scratch, registry) = mounted("expired");
        let grant = registry
            .grant(TICKET, Segment::Shell, AxisKind::EvidenceGrade, 40)
            .expect("a signed grant");
        assert_eq!(
            kinds(&registry
                .apply(Some(&grant), TICKET, Segment::Shell, AxisKind::EvidenceGrade, 40)
                .expect_err("the expiry is exclusive"))
                ,
            vec![RefusalKind::DestinationInvariantRefused]
        );
        registry
            .apply(Some(&grant), TICKET, Segment::Shell, AxisKind::EvidenceGrade, 39)
            .expect("one tick before the bound");
        let _ = std::fs::remove_dir_all(&scratch);
    }

    #[test]
    fn a_grant_scoped_to_another_cell_is_refused() {
        let (scratch, registry) = mounted("other-cell");
        let grant = registry
            .grant(TICKET, Segment::World, AxisKind::EvidenceGrade, 100)
            .expect("a signed grant");
        let refused = registry
            .apply(Some(&grant), TICKET, Segment::Shell, AxisKind::EvidenceGrade, 0)
            .expect_err("another cell");
        assert_eq!(kinds(&refused), vec![RefusalKind::UntrustedKey]);
        assert!(refused.all()[0].detail().contains("world/evidence_grade"));
        let _ = std::fs::remove_dir_all(&scratch);
    }

    #[test]
    fn a_grant_signed_by_a_key_this_authority_does_not_hold_is_untrusted() {
        let (scratch, registry) = mounted("foreign");
        let (other_scratch, other) = security("foreign-other");
        let grant = TicketGrant::issue(&other, TICKET, Segment::Shell, AxisKind::EvidenceGrade, 100)
            .expect("the other key signs its own grant");
        let refused = registry
            .apply(Some(&grant), TICKET, Segment::Shell, AxisKind::EvidenceGrade, 0)
            .expect_err("a foreign key");
        assert_eq!(kinds(&refused), vec![RefusalKind::UntrustedKey]);
        let _ = std::fs::remove_dir_all(&scratch);
        let _ = std::fs::remove_dir_all(&other_scratch);
    }

    #[test]
    fn apply_on_an_unwitnessed_cell_is_refused_even_with_a_good_grant() {
        let (scratch, security) = security("unwitnessed");
        let cells = vec![one_cell(Reading::of_ladder(
            EvidenceGrade::C,
            "the crate's own artifact, read by nobody",
        ))];
        let registry = Registry::of(TICKET, security, cells);
        assert!(
            registry.package().is_none(),
            "a registry over cells serves no package"
        );
        assert_eq!(
            kinds(&registry
                .recipe(TICKET, Segment::Shell, AxisKind::EvidenceGrade)
                .expect_err("no package"))
                ,
            vec![RefusalKind::NoReading]
        );
        let grant = registry
            .grant(TICKET, Segment::Shell, AxisKind::EvidenceGrade, 100)
            .expect("a signed grant");
        let refused = registry
            .apply(Some(&grant), TICKET, Segment::Shell, AxisKind::EvidenceGrade, 0)
            .expect_err("the cell names no witness");
        assert_eq!(kinds(&refused), vec![RefusalKind::UncalibratedOracle]);
        assert!(refused.all()[0].detail().contains("no witness"));
        let _ = std::fs::remove_dir_all(&scratch);
    }

    #[test]
    fn mount_refuses_a_tree_that_witnesses_nothing_instead_of_serving_an_empty_matrix() {
        let scratch = std::env::temp_dir().join(format!(
            "voxel-axis-mcp-empty-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&scratch);
        std::fs::create_dir_all(&scratch).expect("an empty tree");
        let (key_dir, security) = security("mount-empty");
        let refused = Registry::mount(&scratch, TICKET, security).expect_err("nothing to witness");
        assert_eq!(
            refused.all().len(),
            Segment::ALL.len() + 1,
            "every declined segment and the format is named, not summarised"
        );
        assert!(refused
            .all()
            .iter()
            .all(|refusal| refusal.kind() == RefusalKind::UncalibratedOracle));
        let _ = std::fs::remove_dir_all(&scratch);
        let _ = std::fs::remove_dir_all(&key_dir);
    }
}
