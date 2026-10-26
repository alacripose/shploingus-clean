//! **The package: a plain-text document whose every statement is cited, laddered, pinned, and
//! re-derivable.**
//!
//! # The format, in full
//!
//! ```text
//! # voxel package · T-0954
//! # plain text, one statement per line; the pin covers every byte above it
//! ## claims
//! shell ★★★☆☆ silver T-0954<TAB>crates/voxel-client/src/shell/mod.rs:1<TAB>evidence_grade B<TAB>support_level direct<TAB>read back from the tree — workspace: …
//! ## pin
//! bytes<TAB>sha256:…
//! signature<TAB>1<TAB>sha256:…<TAB>…
//! ## reproduce
//! crates/voxel-client/src/shell/mod.rs<TAB>sha256:…
//! ```
//!
//! Three sections, and each is a gate rather than a decoration:
//!
//! | section | holds | refused by |
//! |---|---|---|
//! | `## claims` | one statement per line: what, where, which ladder, how strongly supported, why | a line that is not five fields, a token off its ladder, a citation that does not resolve |
//! | `## pin` | `sha256` of every byte above it, and the signature seam's [`Signature`] when one was made | a digest that does not match, no signature, a signature that does not check out |
//! | `## reproduce` | the digest of every file the statements cite | a file that is missing, or whose bytes drifted |
//!
//! # Refused, never silently shorter
//!
//! [`Package::of`] collects every cell it could not turn into a statement and returns them together;
//! [`Package::gate`] collects every broken rule with the **line number in the rendered document** and
//! the reason. A package that fails a gate is never a shorter package: it is the same document and a
//! refusal that names where it broke.
//!
//! # No cryptography in this file
//!
//! The pin is a content digest ([`Identity`]), and the signature is the seam
//! [`voxel_package::signing`] already declares: this crate names no signing backend, so a build
//! without one gets [`RefusalKind::NoSigningBackend`] — *"a missing instrument is not a pass"* — and
//! the shell shows that refusal rather than an unsigned package that looks signed.
//!
//! # No author
//!
//! Nothing here writes a ticket, a basis or a citation: a statement is the cell's own reading, its
//! own witness and the matrix's own words. The tier and the star come from the matrix; the support is
//! re-derived by [`crate::Line`] and audited at the gate.

use std::path::Path;

use voxel_axis_matrix::Cell;
use voxel_model::refusal::{RefusalKind, RefusalSet};
use voxel_model::Identity;
use voxel_package::signing::{Signature, Signer, Verifier};

use crate::line::{refusal_set, support_of, Line};
use crate::reproduce::Reproduced;

/// The first line of every package: the format's own name, then the package's.
pub const HEADER: &str = "# voxel package · ";
/// The second line: what the reader is holding, and what the pin covers.
pub const NOTE: &str = "# plain text, one statement per line; the pin covers every byte above it";
/// The claims section's heading.
pub const CLAIMS: &str = "## claims";
/// The pin section's heading.
pub const PIN: &str = "## pin";
/// The reproduction section's heading.
pub const REPRODUCE: &str = "## reproduce";

/// The package's bytes, hashed, and the signature over them when one exists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pin {
    /// `sha256` over every byte of the document above `## pin`.
    pub digest: Identity,
    /// The seam's signature, or `None` — which the gate reports as an unsigned package rather than
    /// filling with a plausible value.
    pub signature: Option<Signature>,
}

/// A package: its name, its statements, the files it names, and its pin.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Package {
    name: String,
    lines: Vec<Line>,
    files: Vec<(String, Identity)>,
    pin: Pin,
}

impl Package {
    /// **The one constructor from measured cells.** Every cell must become a statement; if any
    /// cannot, the whole package is refused with that cell's reason named.
    pub fn of(root: &Path, name: &str, cells: &[Cell]) -> Result<Package, RefusalSet> {
        let mut refusals = RefusalSet::new();
        if name.trim().is_empty() || name.contains(['\t', '\n', '\r']) {
            refusals.refuse(
                RefusalKind::MalformedRecord,
                format!("a package name is one non-empty line, and {name:?} is not one"),
            );
        }
        if cells.is_empty() {
            refusals.refuse(
                RefusalKind::NoReading,
                "a package over no cell has no statement and would reconstruct nothing",
            );
        }
        let mut lines = Vec::with_capacity(cells.len());
        for cell in cells {
            match Line::of(root, cell) {
                Ok(line) => lines.push(line),
                Err(set) => {
                    for refusal in set.all() {
                        refusals.push(refusal.clone());
                    }
                }
            }
        }
        if !refusals.is_empty() {
            return Err(refusals);
        }
        let mut paths: Vec<String> = lines.iter().map(|line| line.cite().path.clone()).collect();
        paths.sort_unstable();
        paths.dedup();
        let mut files = Vec::with_capacity(paths.len());
        for path in paths {
            match std::fs::read(root.join(&path)) {
                Ok(bytes) => files.push((path, Identity::of(&bytes))),
                Err(error) => {
                    refusals.refuse(
                        RefusalKind::NoReading,
                        format!(
                            "the package cites `{path}` and this machine cannot read it ({error}), \
                             so it could not be reproduced from these bytes"
                        ),
                    );
                }
            }
        }
        if !refusals.is_empty() {
            return Err(refusals);
        }
        let mut package = Package {
            name: name.to_string(),
            lines,
            files,
            pin: Pin {
                digest: Identity::of(b""),
                signature: None,
            },
        };
        package.pin.digest = Identity::of(package.body().as_bytes());
        Ok(package)
    }

    /// The package's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Its statements, in the order they were measured.
    pub fn lines(&self) -> &[Line] {
        &self.lines
    }

    /// The files it reproduces, each with the digest it recorded.
    pub fn files(&self) -> &[(String, Identity)] {
        &self.files
    }

    /// The pin over its bytes.
    pub fn pin(&self) -> &Pin {
        &self.pin
    }

    /// Sign the bytes the pin covers. **A refusal here is no signature**, not a weak one.
    pub fn sign(&mut self, signer: &dyn Signer) -> Result<(), RefusalSet> {
        let signature = signer.sign(self.body().as_bytes())?;
        self.pin.signature = Some(signature);
        Ok(())
    }

    /// The document, line by line. **The one ordering**: [`Package::render`] joins it, and the line
    /// numbers a refusal can name are positions in this vector.
    fn document(&self) -> Vec<String> {
        let mut document = vec![
            format!("{HEADER}{}", self.name),
            NOTE.to_string(),
            CLAIMS.to_string(),
        ];
        document.extend(self.lines.iter().map(Line::render));
        document.push(PIN.to_string());
        document.push(format!("bytes\t{}", self.pin.digest));
        if let Some(signature) = &self.pin.signature {
            document.push(format!(
                "signature\t{}\t{}\t{}",
                signature.algorithm(),
                signature.key_id().as_str(),
                hex(signature.bytes())
            ));
        }
        document.push(REPRODUCE.to_string());
        document.extend(
            self.files
                .iter()
                .map(|(path, digest)| format!("{path}\t{digest}")),
        );
        document
    }

    /// The document as bytes: plain text, one statement per line, ending in a newline.
    pub fn render(&self) -> String {
        let mut text = self.document().join("\n");
        text.push('\n');
        text
    }

    /// Everything above `## pin` — **the bytes the pin and the signature cover**.
    pub fn body(&self) -> String {
        let mut text = self.document()[..3 + self.lines.len()].join("\n");
        text.push('\n');
        text
    }

    /// The 1-based line of statement `index` in [`Package::render`].
    pub fn line_of(&self, index: usize) -> usize {
        index + 4
    }

    /// The 1-based line of the `bytes` field of the pin.
    pub fn pin_line(&self) -> usize {
        self.lines.len() + 5
    }

    /// The 1-based line where the reproduction section records file `index`.
    pub fn file_line(&self, index: usize) -> usize {
        self.lines.len() + 7 + usize::from(self.pin.signature.is_some()) + index
    }

    /// **Read a package back.** The document's shape is the format, so anything not exactly the
    /// rendering of a package is refused with the line it broke on — including bytes that are not
    /// UTF-8, a carriage return, and a field count no statement has.
    pub fn parse(text: &str) -> Result<Package, RefusalSet> {
        let mut refusals = RefusalSet::new();
        // Scanned over the raw characters rather than `str::lines`, which would strip the `\r` of a
        // carriage return before this could see it.
        let mut at = 1;
        for character in text.chars() {
            if character == '\n' {
                at += 1;
                continue;
            }
            if character != '\t' && character.is_control() {
                refusals.refuse(
                    RefusalKind::MalformedRecord,
                    format!(
                        "line {at}: {:?} is a control character, and a package is plain text with \
                         tabs and newlines only",
                        character
                    ),
                );
            }
        }
        if !refusals.is_empty() {
            return Err(refusals);
        }
        let lines: Vec<&str> = text.lines().collect();
        if !text.ends_with('\n') {
            refusals.refuse(
                RefusalKind::MalformedRecord,
                "a package ends with a newline; this text does not, and rendering it back would add \
                 one",
            );
        }
        let mut claims_at = None;
        let mut pin_at = None;
        let mut reproduce_at = None;
        for (index, line) in lines.iter().enumerate() {
            match *line {
                CLAIMS if claims_at.is_none() => claims_at = Some(index),
                PIN if pin_at.is_none() => pin_at = Some(index),
                REPRODUCE if reproduce_at.is_none() => reproduce_at = Some(index),
                _ => {}
            }
        }
        let name = lines
            .first()
            .and_then(|line| line.strip_prefix(HEADER))
            .filter(|name| !name.trim().is_empty());
        let (Some(claims_at), Some(pin_at), Some(reproduce_at), Some(name)) =
            (claims_at, pin_at, reproduce_at, name)
        else {
            refusals.refuse(
                RefusalKind::MalformedRecord,
                format!(
                    "a package is `{HEADER}<name>`, `{NOTE}`, `{CLAIMS}`, at least one statement, \
                     `{PIN}` and `{REPRODUCE}`, in that order; this text is not one"
                ),
            );
            return Err(refusals);
        };
        if claims_at != 2 || pin_at <= claims_at + 1 || reproduce_at <= pin_at + 1 {
            refusals.refuse(
                RefusalKind::MalformedRecord,
                format!(
                    "the sections are out of order: claims at line {}, pin at line {}, reproduce at \
                     line {}",
                    claims_at + 1,
                    pin_at + 1,
                    reproduce_at + 1
                ),
            );
            return Err(refusals);
        }
        let mut statements = Vec::with_capacity(pin_at - claims_at - 1);
        for (offset, line) in lines[claims_at + 1..pin_at].iter().enumerate() {
            let at = claims_at + offset + 2;
            let fields: Vec<&str> = line.split('\t').collect();
            match Line::parse(&fields) {
                Ok(statement) => statements.push(statement),
                Err(detail) => {
                    refusals.refuse(RefusalKind::MalformedRecord, format!("line {at}: {detail}"));
                }
            }
        }
        let mut digest = None;
        let mut signature = None;
        for (offset, line) in lines[pin_at + 1..reproduce_at].iter().enumerate() {
            let at = pin_at + offset + 2;
            let mut fields = line.split('\t');
            match fields.next() {
                Some("bytes") => {
                    let value = fields.next().unwrap_or_default();
                    match Identity::parse(value) {
                        Ok(identity) if fields.next().is_none() => digest = Some(identity),
                        Ok(_) => {
                            refusals.refuse(
                                RefusalKind::MalformedRecord,
                                format!(
                                    "line {at}: a `bytes` field holds one identity and nothing else"
                                ),
                            );
                        }
                        Err(refusal) => {
                            refusals.refuse(
                                RefusalKind::MalformedRecord,
                                format!("line {at}: {}", refusal.detail()),
                            );
                        }
                    }
                }
                Some("signature") => {
                    let fields: Vec<&str> = fields.collect();
                    match signature_of(&fields) {
                        Ok(value) => signature = Some(value),
                        Err(detail) => {
                            refusals.refuse(
                                RefusalKind::MalformedRecord,
                                format!("line {at}: {detail}"),
                            );
                        }
                    }
                }
                _ => {
                    refusals.refuse(
                        RefusalKind::MalformedRecord,
                        format!(
                            "line {at}: the pin holds `bytes` and, when signed, `signature`; this \
                             is neither"
                        ),
                    );
                }
            }
        }
        let mut files = Vec::with_capacity(lines.len().saturating_sub(reproduce_at + 1));
        for (offset, line) in lines[reproduce_at + 1..].iter().enumerate() {
            let at = reproduce_at + offset + 2;
            let fields: Vec<&str> = line.split('\t').collect();
            if fields.len() != 2 || fields[0].trim().is_empty() {
                refusals.refuse(
                    RefusalKind::MalformedRecord,
                    format!("line {at}: a reproduced file is `path<TAB>sha256:…`"),
                );
                continue;
            }
            match Identity::parse(fields[1]) {
                Ok(identity) => files.push((fields[0].to_string(), identity)),
                Err(refusal) => {
                    refusals.refuse(
                        RefusalKind::MalformedRecord,
                        format!("line {at}: {}", refusal.detail()),
                    );
                }
            }
        }
        let Some(digest) = digest else {
            refusals.refuse(
                RefusalKind::MalformedRecord,
                format!("line {}: the pin records no `bytes` digest", pin_at + 2),
            );
            return Err(refusals);
        };
        let package = Package {
            name: name.to_string(),
            lines: statements,
            files,
            pin: Pin { digest, signature },
        };
        if package.render() != text {
            refusals.refuse(
                RefusalKind::MalformedRecord,
                "this text is not the rendering of the package it parses to: the format has one \
                 spelling, and a document that reads back differently was written by something else",
            );
        }
        refusals.into_result(package)
    }

    /// Read a package from bytes. **Bytes that are not UTF-8 are refused**, because a package is
    /// plain text and a blob that decodes to plausible words is still not one.
    pub fn parse_bytes(bytes: &[u8]) -> Result<Package, RefusalSet> {
        let text = std::str::from_utf8(bytes).map_err(|error| {
            refusal_set(
                RefusalKind::MalformedRecord,
                format!(
                    "line 1: the package is not UTF-8 ({error}); it is plain text, and a binary blob \
                     with a package-shaped prefix is not one"
                ),
            )
        })?;
        Package::parse(text)
    }

    /// **The five gates, over a tree and a verifier.** Refusals carry the line in this document and
    /// the reason; a gate that cannot run is a refusal, never a pass.
    pub fn gate(&self, root: &Path, verifier: &dyn Verifier) -> Result<(), RefusalSet> {
        let mut refusals = RefusalSet::new();
        for (index, line) in self.lines.iter().enumerate() {
            let at = self.line_of(index);
            if let Err(refusal) = line.cite().locate(root) {
                refusals.refuse(
                    RefusalKind::NoReading,
                    format!("line {at}: {}", refusal.detail()),
                );
            }
            if !self.files.iter().any(|(path, _)| *path == line.cite().path) {
                refusals.refuse(
                    RefusalKind::MalformedRecord,
                    format!(
                        "line {at}: the statement cites `{}` and the reproduce section does not name \
                         it, so applying this package would not write the file it quotes",
                        line.cite().path
                    ),
                );
            }
            if let Err((kind, detail)) = line.check() {
                refusals.refuse(kind, format!("line {at}: {detail}"));
            }
            let derived = support_of(root, line.cite(), line.reading());
            if derived != line.support() {
                refusals.refuse(
                    RefusalKind::UncalibratedOracle,
                    format!(
                        "line {at}: the statement carries `support_level {}` and this machine derives \
                         `support_level {}` from the same citation and reading; support is computed, \
                         never asserted",
                        line.support().as_str(),
                        derived.as_str()
                    ),
                );
            }
        }
        let body = self.body();
        let found = Identity::of(body.as_bytes());
        if found != self.pin.digest {
            refusals.refuse(
                RefusalKind::DigestMismatch,
                format!(
                    "line {}: the pin records {} and the bytes above it hash to {}",
                    self.pin_line(),
                    self.pin.digest,
                    found
                ),
            );
        }
        match &self.pin.signature {
            Some(signature) if !signature.is_empty() => {
                if let Err(set) = verifier.verify(body.as_bytes(), signature) {
                    for refusal in set.all() {
                        refusals.push(refusal.clone());
                    }
                }
            }
            _ => {
                refusals.refuse(
                    RefusalKind::UnsignedEnvelope,
                    format!(
                        "line {}: the package carries no signature, so nothing in it is \
                         attributable to anyone. An absent signature is not a weak one",
                        self.pin_line()
                    ),
                );
            }
        }
        refusals.into_result(())
    }

    /// **The reproduction claim.** Re-reads every named file and compares it byte for byte; a package
    /// that cannot be re-derived on this machine is refused rather than promised.
    pub fn reproduce(&self, root: &Path) -> Result<Reproduced, RefusalSet> {
        crate::reproduce::reproduce(self, root)
    }
}

fn signature_of(fields: &[&str]) -> Result<Signature, String> {
    let [algorithm, key, bytes] = fields else {
        return Err(
            "a signature is `signature<TAB><algorithm><TAB><key id><TAB><bytes>`".to_string(),
        );
    };
    let algorithm: u16 = algorithm
        .parse()
        .map_err(|_| format!("{algorithm:?} is not an algorithm identifier"))?;
    let key = voxel_package::signing::KeyId::parse(key).map_err(|_| {
        format!("{key:?} is not a key identity; a key is named `sha256:<64 lowercase hex>`")
    })?;
    let bytes =
        unhex(bytes).ok_or_else(|| format!("{bytes:?} is not an even-length hex string"))?;
    Ok(Signature::new(algorithm, key, bytes))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn unhex(text: &str) -> Option<Vec<u8>> {
    if text.len() % 2 != 0 {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(text.get(at..at + 2)?, 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use voxel_axis_matrix::{Predicate, Reading, Segment, Site, Star};
    use voxel_model::axis::{EvidenceGrade, EvidenceLevel};

    const KEY: &[u8] = b"the test's key, never a real one";

    /// A stand-in for the absent `voxel-envelope` backend: a deterministic digest standing where a
    /// real signature would be. The seam is what is exercised — accept, refuse, and the two kinds.
    struct Sealed;

    impl Signer for Sealed {
        fn sign(&self, payload: &[u8]) -> Result<Signature, RefusalSet> {
            Ok(Signature::new(
                1,
                voxel_package::signing::KeyId::of(KEY),
                Identity::of(payload).digest().as_bytes().to_vec(),
            ))
        }
    }

    impl Verifier for Sealed {
        fn verify(&self, payload: &[u8], signature: &Signature) -> Result<(), RefusalSet> {
            let expected = Signature::new(
                1,
                voxel_package::signing::KeyId::of(KEY),
                Identity::of(payload).digest().as_bytes().to_vec(),
            );
            if &expected == signature {
                Ok(())
            } else {
                Err(refusal_set(
                    RefusalKind::SignatureInvalid,
                    "the signature is not over these bytes by this key".to_string(),
                ))
            }
        }
    }

    fn scratch(tag: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("voxel-axis-artifact-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("notes")).expect("a scratch tree");
        std::fs::write(
            root.join("notes/segment.txt"),
            "the shell mounts the windows\nand the console carries the report\n",
        )
        .expect("a witness");
        root
    }

    fn cells() -> Vec<Cell> {
        let reading = Reading::of_ladder(EvidenceGrade::B, "read back from the tree")
            .witnessed("notes/segment.txt");
        vec![Cell::new(
            Segment::Shell,
            reading,
            Star::Three,
            vec![
                Predicate::held("workspace_member"),
                Predicate::held("census_measured"),
                Predicate::held("declared_witness"),
            ],
            "T-0954",
            "PATCH-042",
        )
        .with_requirements(vec!["declared_provenance", "evidence_present"])]
    }

    fn signed(root: &Path) -> Package {
        let mut package = Package::of(root, "T-0954", &cells()).expect("a measured package");
        package.sign(&Sealed).expect("the test backend signs");
        package
    }

    /// Re-take the pin after a deliberate edit, so a test isolates the gate it is about.
    fn repin(package: &mut Package) {
        package.pin.digest = Identity::of(package.body().as_bytes());
    }

    #[test]
    fn a_package_renders_and_reads_back_to_the_same_bytes() {
        let root = scratch("round-trip");
        let package = signed(&root);
        let text = package.render();
        let parsed = Package::parse_bytes(text.as_bytes()).expect("its own bytes");
        assert_eq!(parsed, package);
        assert_eq!(parsed.render(), text, "one spelling, byte for byte");
        assert_eq!(package.lines().len(), 1);
        assert_eq!(package.files().len(), 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_line_numbers_a_refusal_names_are_the_rendered_lines() {
        let root = scratch("lines");
        let package = signed(&root);
        let text = package.render();
        let rendered: Vec<&str> = text.lines().collect();
        assert_eq!(
            rendered[package.line_of(0) - 1],
            package.lines()[0].render()
        );
        assert_eq!(
            rendered[package.pin_line() - 1],
            format!("bytes\t{}", package.pin().digest)
        );
        assert_eq!(
            rendered[package.file_line(0) - 1],
            format!("{}\t{}", package.files()[0].0, package.files()[0].1)
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_signed_package_passes_every_gate_and_reproduces_its_files() {
        let root = scratch("passes");
        let package = signed(&root);
        package
            .gate(&root, &Sealed)
            .expect("cited, laddered, pinned");
        let reproduced = package.reproduce(&root).expect("the files are here");
        assert_eq!(reproduced.files(), 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn an_unsigned_package_is_refused_at_the_pin_rather_than_read_as_signed() {
        let root = scratch("unsigned");
        let package = Package::of(&root, "T-0954", &cells()).expect("a measured package");
        let refused = package.gate(&root, &Sealed).expect_err("no signature");
        assert!(
            refused
                .all()
                .iter()
                .any(|refusal| refusal.kind() == RefusalKind::UnsignedEnvelope),
            "{refused:?}"
        );
        // And the seam's own default, for a build with no backend at all.
        let mut signed_package = package.clone();
        signed_package.sign(&Sealed).unwrap();
        let refused = signed_package
            .gate(
                &root,
                &voxel_package::signing::NoSigningBackend {
                    why: "this build does not link voxel-envelope".to_string(),
                },
            )
            .expect_err("no backend is not a pass");
        assert!(refused
            .all()
            .iter()
            .any(|refusal| refusal.kind() == RefusalKind::NoSigningBackend));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_statement_that_cites_a_file_this_machine_does_not_have_is_refused_by_its_line() {
        let root = scratch("cited");
        let package = signed(&root);
        let text = package
            .render()
            .replace("notes/segment.txt:1", "notes/missing.txt:1");
        let mut parsed = Package::parse(&text).expect("the shape is intact");
        repin(&mut parsed);
        let refused = parsed
            .gate(&root, &Sealed)
            .expect_err("the citation points at nothing");
        let detail = refused
            .all()
            .iter()
            .map(|refusal| refusal.detail().to_string())
            .collect::<Vec<_>>()
            .join("; ");
        assert!(detail.contains("notes/missing.txt"), "{detail}");
        assert!(
            detail.contains("line 4"),
            "the refusal names the statement's line: {detail}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_support_the_tree_does_not_derive_is_refused_as_asserted() {
        let root = scratch("support");
        let package = signed(&root);
        let text = package
            .render()
            .replace("support_level direct", "support_level inferred");
        let mut parsed = Package::parse(&text).expect("a well-formed lie");
        repin(&mut parsed);
        let refused = parsed
            .gate(&root, &Sealed)
            .expect_err("support is computed");
        assert!(
            refused
                .all()
                .iter()
                .any(|refusal| refusal.kind() == RefusalKind::UncalibratedOracle
                    && refusal.detail().contains("direct")),
            "{refused:?}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_token_off_its_ladder_is_refused_before_the_package_exists() {
        let root = scratch("token");
        let text = signed(&root)
            .render()
            .replace("evidence_grade B", "certification_tier plutonium");
        let refused = Package::parse(&text).expect_err("plutonium is not a tier");
        assert_eq!(refused.all()[0].kind(), RefusalKind::MalformedRecord);
        assert!(
            refused.all()[0].detail().contains("line 4"),
            "{}",
            refused.all()[0].detail()
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_changed_byte_breaks_the_pin_and_names_the_line() {
        let root = scratch("digest");
        let package = signed(&root);
        let text = package.render().replace("T-0954", "T-0955");
        let parsed = Package::parse(&text).expect("the shape is intact");
        let refused = parsed.gate(&root, &Sealed).expect_err("the bytes moved");
        assert!(
            refused
                .all()
                .iter()
                .any(|refusal| refusal.kind() == RefusalKind::DigestMismatch
                    && refusal
                        .detail()
                        .contains(&format!("line {}", package.pin_line()))),
            "{refused:?}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_re_pinned_edit_still_fails_the_signature_it_was_not_signed_with() {
        let root = scratch("signature");
        let package = signed(&root);
        let text = package.render().replace("T-0954", "T-0955");
        let mut parsed = Package::parse(&text).expect("the shape is intact");
        repin(&mut parsed);
        let refused = parsed
            .gate(&root, &Sealed)
            .expect_err("the signature is over other bytes");
        assert!(refused
            .all()
            .iter()
            .any(|refusal| refusal.kind() == RefusalKind::SignatureInvalid));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn bytes_that_are_not_plain_text_are_refused_as_a_document() {
        let refused = Package::parse_bytes(&[0xff, 0xfe, b'\n']).expect_err("not UTF-8");
        assert_eq!(refused.all()[0].kind(), RefusalKind::MalformedRecord);
        let refused =
            Package::parse(&format!("{HEADER}x\n{NOTE}\n{CLAIMS}\n")).expect_err("no pin");
        assert_eq!(refused.all()[0].kind(), RefusalKind::MalformedRecord);
        let refined = signed(&scratch("crlf"));
        let text = refined.render().replace('\n', "\r\n");
        let refused = Package::parse(&text).expect_err("carriage returns are not the format");
        assert!(refused.all()[0].detail().contains("line 1"), "{refused:?}");
    }

    #[test]
    fn a_file_that_drifted_or_vanished_refuses_the_reproduction_claim() {
        let root = scratch("reproduce");
        let package = signed(&root);
        std::fs::write(root.join("notes/segment.txt"), "something else\n").expect("a drift");
        let refused = package.reproduce(&root).expect_err("the bytes moved");
        assert!(refused
            .all()
            .iter()
            .any(|refusal| refusal.kind() == RefusalKind::DigestMismatch
                && refusal.detail().contains("notes/segment.txt")));
        let _ = std::fs::remove_dir_all(&root);
        let refused = package
            .reproduce(&root)
            .expect_err("this machine has no such tree");
        assert!(refused
            .all()
            .iter()
            .any(|refusal| refusal.kind() == RefusalKind::NoReading
                && refusal.detail().contains("cannot be re-derived")));
    }

    #[test]
    fn a_package_signed_by_the_security_layer_passes_the_pin_gate() {
        let root = scratch("security");
        let keys =
            std::env::temp_dir().join(format!("voxel-axis-artifact-keys-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&keys);
        let security = voxel_axis_security::Security::open(&root, keys.join("signing.key"))
            .expect("a machine key");
        let mut package = Package::of(&root, "T-0954", &cells()).expect("a measured package");
        package.sign(&security).expect("signed by the machine key");
        package
            .gate(&root, &security)
            .expect("the pin is over these bytes by this key");

        let text = package.render().replace("T-0954", "T-0955");
        let mut edited = Package::parse(&text).expect("the shape is intact");
        repin(&mut edited);
        let refused = edited
            .gate(&root, &security)
            .expect_err("the signature is over other bytes");
        assert!(
            refused
                .all()
                .iter()
                .any(|refusal| refusal.kind() == RefusalKind::SignatureInvalid),
            "{refused:?}"
        );

        let other = voxel_axis_security::Security::open(&root, keys.join("other.key"))
            .expect("another key");
        let refused = package
            .gate(&root, &other)
            .expect_err("not this verifier's key");
        assert!(
            refused
                .all()
                .iter()
                .any(|refusal| refusal.kind() == RefusalKind::UntrustedKey),
            "{refused:?}"
        );
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&keys);
    }

    #[test]
    fn a_package_over_no_cell_is_refused_rather_than_an_empty_document() {
        let refused =
            Package::of(Path::new("."), "empty", &[]).expect_err("nothing to reconstruct");
        assert_eq!(refused.all()[0].kind(), RefusalKind::NoReading);
    }

    #[test]
    fn a_statement_that_is_not_five_fields_is_refused_with_its_line() {
        let text = format!(
            "{HEADER}x\n{NOTE}\n{CLAIMS}\none field\n{PIN}\nbytes\tsha256:{digest}\n{REPRODUCE}\nfile.txt\tsha256:{digest}\n",
            digest = Identity::of(b"x").digest().to_hex()
        );
        let refused = Package::parse(&text).expect_err("a statement is five fields");
        assert!(refused.all()[0].detail().contains("line 4"), "{refused:?}");
    }

    #[test]
    fn a_site_is_the_first_non_blank_line_and_not_a_number_someone_typed() {
        let root = scratch("site");
        std::fs::write(
            root.join("notes/segment.txt"),
            "\n\nthe shell mounts the windows\n",
        )
        .unwrap();
        let package = Package::of(&root, "T-0954", &cells()).expect("a measured package");
        assert_eq!(package.lines()[0].cite().line, 3);
        assert_eq!(package.lines()[0].support(), EvidenceLevel::Direct);
        let _ = Site::new("notes/segment.txt", 3);
        let _ = std::fs::remove_dir_all(&root);
    }
}
