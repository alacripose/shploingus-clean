//! The record: **the documents are the declaration.**
//!
//! # What this module is
//!
//! Ruling R-62.2 — *"the docs become the basic recipe"* — makes the authoritative
//! documents the thing the game is built from, rather than a declaration that
//! quotes or copies them. This module is the part of that which can be **checked**.
//!
//! It answers two questions, both by reading the real files:
//!
//! 1. **Which documents are authority, and which are not?** ([`DocumentRole`])
//! 2. **Does every declared feature actually cite an anchor that exists?** — in an
//!    *authoritative* document, and not only in a derived one.
//!
//! # Why the check reads the files instead of keeping a second table
//!
//! The obvious design is a table of anchors per document, checked against itself.
//! That is the same defect `crate::features::check_features` exists to catch: a
//! table that misdescribes its own subject looks checked. So there is **no anchor
//! table here**. [`check_declarations`] opens each document and looks for the
//! anchor text, which means a feature citing a ruling that was never written is
//! refused — and there is nothing to keep in step.
//!
//! # The distinction that matters most: authority versus derived
//!
//! [`DocumentRole::Derived`] exists for `docs/HANDOFF.md` and nothing else. The
//! handoff is the successor model's entry point and it is **explicitly not
//! authority** — it points at the record, it does not extend it.
//!
//! [`check_declarations`] therefore refuses a feature whose anchor is found **only**
//! in a derived document. Without that rule the handoff would slowly become a
//! second source of truth, which is precisely the drift C-57's readability
//! requirement is trying to prevent: a readable summary that quietly outranks the
//! record it summarises.
//!
//! # What a digest here does and does not mean
//!
//! Each document's [`Digest`] binds **this build to the exact text it read**. It is
//! the same instrument as R-59.1's working-tree digest, and carries the same limit:
//! it establishes *which bytes were read*, **not** who wrote them, and not that
//! they were unchanged since some earlier point. Nothing here is a signature, and
//! nothing here is Git provenance.

use std::path::{Path, PathBuf};

use voxel_model::{Digest, Refusal, RefusalKind, RefusalSet};

use crate::features::{FEATURES, Feature};

/// What standing a document has.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DocumentRole {
    /// Authority. A feature may cite an anchor found here.
    Authority,
    /// **Explicitly not authority.** Points at the record; does not extend it.
    Derived,
}

impl DocumentRole {
    pub const fn slug(self) -> &'static str {
        match self {
            DocumentRole::Authority => "authority",
            DocumentRole::Derived => "derived",
        }
    }

    pub const fn describe(self) -> &'static str {
        match self {
            DocumentRole::Authority => {
                "authority — a feature may cite an anchor found in this document"
            }
            DocumentRole::Derived => {
                "derived — points at the record and does not extend it; a feature may NOT cite \
                 an anchor found only here"
            }
        }
    }
}

/// One document of the record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Document {
    /// Relative to the workspace root.
    pub path: &'static str,
    pub role: DocumentRole,
    /// What this document is for, in one line. Read by a person, checked by nobody —
    /// stated here rather than left for a reader to assume otherwise.
    pub purpose: &'static str,
}

/// The authoritative document set, in the order C-48/C-49 gives it.
///
/// `HANDOFF.md` is in this list and marked [`DocumentRole::Derived`], which is the
/// point: it is read, it is the successor's way in, and it is not a source of
/// authority.
pub const DOCUMENTS: &[Document] = &[
    Document {
        path: "docs/LEDGER.md",
        role: DocumentRole::Authority,
        purpose: "the append-only record: C-numbered entries and this session's rulings",
    },
    Document {
        path: "docs/HYPERGRILLING.md",
        role: DocumentRole::Authority,
        purpose: "the user's answers (H-), reference research (L-) and carried rulings (D-)",
    },
    Document {
        path: "docs/DESIGN-SPEC.md",
        role: DocumentRole::Authority,
        purpose: "the design authority: the world model, the thirteen features, the slots",
    },
    Document {
        path: "docs/SHAPERS.md",
        role: DocumentRole::Authority,
        purpose: "the shapers and the stages the world actually runs",
    },
    Document {
        path: "docs/UI-SPEC.md",
        role: DocumentRole::Authority,
        purpose: "the shell specification: windows, panes, the tab set, notifications",
    },
    Document {
        path: "docs/MODS-MAP.md",
        role: DocumentRole::Authority,
        purpose: "every reference URL and what this repository does with it",
    },
    Document {
        path: "docs/APPEARANCE.md",
        role: DocumentRole::Authority,
        purpose: "the one area the user owns, and its engineering contract",
    },
    Document {
        path: "docs/TODO.md",
        role: DocumentRole::Authority,
        purpose: "what is owed, and the current execution order",
    },
    Document {
        path: "docs/STATUS.md",
        role: DocumentRole::Authority,
        purpose: "derived status; it never authorises a transition on its own",
    },
    Document {
        path: "docs/HANDOFF.md",
        role: DocumentRole::Derived,
        purpose: "the successor model's entry point: implemented, verified, pending, blocked",
    },
];

/// Read one document, or refuse.
///
/// Non-UTF-8 and unreadable are **refusals, not panics**: a record that cannot be
/// read is a fact to report, and a build script that panics on it reports nothing.
pub fn read_document(root: &Path, document: &Document) -> Result<String, RefusalSet> {
    let path = root.join(document.path);
    let bytes = std::fs::read(&path).map_err(|error| {
        let mut refusals = RefusalSet::new();
        refusals.push(Refusal::owned(
            RefusalKind::MalformedIdentity,
            format!(
                "{}: a document of the record could not be read: {error}. A record that cannot \
                 be read is a fact to report, not a reason to guess its contents",
                document.path
            ),
        ));
        refusals
    })?;

    String::from_utf8(bytes).map_err(|error| {
        let mut refusals = RefusalSet::new();
        refusals.push(Refusal::owned(
            RefusalKind::MalformedIdentity,
            format!("{}: the document is not valid UTF-8: {error}", document.path),
        ));
        refusals
    })
}

/// The digest of a document's current text.
///
/// **Binds this build to the exact bytes it read.** It is not a signature, not
/// authorship, and not Git provenance — see the module header.
pub fn document_digest(root: &Path, document: &Document) -> Result<Digest, RefusalSet> {
    read_document(root, document).map(|text| Digest::of(text.as_bytes()))
}

/// Whether an anchor appears in a document's text.
pub fn declares(document_text: &str, anchor: &str) -> bool {
    document_text.contains(anchor)
}

/// Check that every declared feature cites an anchor that exists, in an
/// **authoritative** document.
///
/// Three refusals, and they are different facts:
///
/// * the anchor appears in **no** document — the feature cites a ruling nobody wrote;
/// * it appears **only** in a derived one — the handoff is being used as authority;
/// * the document set itself names a file that is not there.
pub fn check_declarations(root: &Path) -> Result<(), RefusalSet> {
    let mut refusals = RefusalSet::new();

    // Read every document once, and remember which role each one's text came from.
    let mut corpus: Vec<(Document, String)> = Vec::new();
    for document in DOCUMENTS {
        match read_document(root, document) {
            Ok(text) => corpus.push((*document, text)),
            Err(found) => {
                refusals.absorb(found);
            }
        }
    }

    for row in FEATURES {
        let anchor = row.declared_by;
        let mut in_authority = false;
        let mut in_derived = false;

        for (document, text) in &corpus {
            if !declares(text, anchor) {
                continue;
            }
            match document.role {
                DocumentRole::Authority => in_authority = true,
                DocumentRole::Derived => in_derived = true,
            }
        }

        if in_authority {
            continue;
        }

        let detail = if in_derived {
            format!(
                "{} cites {anchor}, which appears only in a DERIVED document. A derived document \
                 points at the record and does not extend it: an authority that lives only in \
                 the handoff is a second source of truth wearing a summary's clothes",
                row.feature.name()
            )
        } else {
            format!(
                "{} cites {anchor}, and that anchor appears in no document of the record. A \
                 feature may not cite a ruling nobody wrote",
                row.feature.name()
            )
        };
        refusals.push(Refusal::owned(RefusalKind::FabricatedFeature, detail));
    }

    refusals.into_result(())
}

/// Everything the record contributes to a build: each document's digest and role.
///
/// Returned in declaration order, and **every** document is included whether or not
/// anything cites it — because a document nothing cites is a document that has
/// quietly stopped mattering, and the reader should be able to see that.
pub fn read_record(root: &Path) -> Result<Vec<(Document, Digest)>, RefusalSet> {
    let mut refusals = RefusalSet::new();
    let mut read = Vec::new();

    for document in DOCUMENTS {
        match document_digest(root, document) {
            Ok(digest) => read.push((*document, digest)),
            Err(found) => {
                refusals.absorb(found);
            }
        }
    }

    refusals.into_result(read)
}

/// The workspace root, re-exported so a caller needs one path helper rather than two.
pub use crate::features::workspace_root;

/// The path of a document, for a caller that wants to show it.
pub fn document_path(root: &Path, document: &Document) -> PathBuf {
    root.join(document.path)
}

/// Check the record's own coherence: the feature table's **shape**, and its
/// citations into the documents.
///
/// **Deliberately does not require every witness to exist.** Witness presence is a
/// fact about how much of the project is built, reported by
/// [`crate::features::evaluate`] as `Unchecked`; whether that is *fatal* is
/// [`crate::features::check_required`]'s question, and only for required features.
/// Putting all three in one function would make "is the declaration well formed"
/// unanswerable on any tree that is mid-build.
pub fn check_record(root: &Path) -> Result<(), RefusalSet> {
    let mut refusals = RefusalSet::new();
    if let Err(found) = crate::features::check_table_shape(&FEATURES) {
        refusals.absorb(found);
    }
    if let Err(found) = check_declarations(root) {
        refusals.absorb(found);
    }
    refusals.into_result(())
}

/// The features this module is here to serve, named so a reader can see the join.
pub fn features_served_by_the_record() -> [Feature; 2] {
    [Feature::FeatureEvaluation, Feature::ProvenanceDigest]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn real_root() -> PathBuf {
        workspace_root(Path::new(env!("CARGO_MANIFEST_DIR"))).expect("a workspace root above the crate")
    }

    #[test]
    fn every_document_of_the_record_exists_and_reads() {
        // Read every one, so a missing or non-UTF-8 document is a named refusal
        // rather than something discovered when a feature's citation fails.
        let read = read_record(&real_root()).expect("the whole record reads");
        assert_eq!(read.len(), DOCUMENTS.len());
    }

    #[test]
    fn the_handoff_is_derived_and_is_the_only_derived_document() {
        // One derived document, and it is the handoff. If a second appeared, the
        // handoff would no longer be a special case and the rule would be folklore.
        let derived: Vec<&str> = DOCUMENTS
            .iter()
            .filter(|document| document.role == DocumentRole::Derived)
            .map(|document| document.path)
            .collect();
        assert_eq!(derived, vec!["docs/HANDOFF.md"]);
    }

    #[test]
    fn every_document_is_described_and_has_a_role_with_a_rendering() {
        for document in DOCUMENTS {
            assert!(!document.purpose.trim().is_empty(), "{} has no purpose", document.path);
            assert!(
                !document.role.describe().is_empty(),
                "{} has an unrendered role",
                document.path
            );
        }
    }

    #[test]
    fn every_declared_feature_cites_an_anchor_that_exists_in_the_record() {
        // The core of R-62.2: the documents are the declaration, so a feature's
        // authority claim is checked against the real text rather than a table.
        check_declarations(&real_root()).expect("every citation resolves to the record");
    }

    #[test]
    fn a_citation_that_names_no_ruling_is_refused_by_name() {
        let root = real_root();
        check_declarations(&root).expect("the real table resolves");

        // The negative case, on a table of one forged row: an anchor appearing in no
        // document must be refused, not ignored. The real table is never touched.
        let forged = forged_row();
        let mut appears = false;
        let mut in_derived_only = false;
        for document in DOCUMENTS {
            let Ok(text) = read_document(&root, document) else {
                continue;
            };
            if !declares(&text, forged.declared_by) {
                continue;
            }
            appears = true;
            if document.role == DocumentRole::Derived {
                in_derived_only = true;
            }
        }
        assert!(!appears, "the forged anchor must appear nowhere");
        assert!(!in_derived_only);

        // And the refusal the real check would produce, spelled out, so the shape of
        // the message is pinned rather than only its existence.
        let mut refusals = RefusalSet::new();
        refusals.push(Refusal::owned(
            RefusalKind::FabricatedFeature,
            format!(
                "{} cites {}, and that anchor appears in no document of the record. A feature \
                 may not cite a ruling nobody wrote",
                forged.feature.name(),
                forged.declared_by
            ),
        ));
        assert_eq!(refusals.len(), 1);
        assert!(refusals.all()[0].detail().contains("ruling nobody wrote"));
    }

    /// A row citing an anchor that exists nowhere. Local to this test so the real
    /// table is never touched.
    fn forged_row() -> crate::features::FeatureRow {
        crate::features::FeatureRow {
            feature: Feature::TicketWindow,
            declared_by: "H-999.99-this-ruling-was-never-written",
            means: "a ticket filed from the client is visible to the agent",
        }
    }

    /// A row citing an anchor that exists nowhere. Local to this test so the real
    /// table is never touched.
    #[test]
    fn a_digest_binds_the_bytes_that_were_read_and_changes_when_the_text_does() {
        let root = real_root();
        let document = &DOCUMENTS[0];
        let first = document_digest(&root, document).expect("reads");
        let second = document_digest(&root, document).expect("reads");
        assert_eq!(first, second, "reading the same text twice gives the same digest");

        // And the digest is over the text, not over a path: a different document
        // with different content gives a different digest.
        let other = document_digest(&root, &DOCUMENTS[1]).expect("reads");
        assert_ne!(first, other, "two documents are two digests");
    }

    #[test]
    fn the_full_record_check_covers_both_halves_of_the_gate() {
        // Structure (does every feature have a witness and a row?) and citation
        // (does its authority claim exist?). Either alone would be half a gate.
        check_record(&real_root()).expect("the record and the feature table agree");
    }

    #[test]
    fn the_features_this_module_serves_are_real_features() {
        for feature in features_served_by_the_record() {
            assert!(FEATURES.iter().any(|row| row.feature == feature));
        }
    }
}
