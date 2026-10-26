//! **The prose walk: one real document, tiled into units, claimed and rendered.**
//!
//! The scrubber has four readers in PATCH-042's full design. This is the first one that is
//! *finished* rather than declared: a Markdown document is read once, tiled by
//! [`voxel_axis_unit`], every unit gets one claim from [`voxel_axis_claim`], and the claims are
//! placed in the three sections of [`voxel_axis_view`].
//!
//! # What it proves
//!
//! * **The tiling is exact.** [`Unit::tile_text`] refuses a gap or an overlap before a claim is
//!   derived, so a deleted paragraph cannot become a shorter document.
//! * **Every line is traceable.** [`Document::validate`] re-checks each line against its claim's
//!   quote, kind and id.
//! * **The run is deterministic.** The same bytes produce the same unit ids and the same rendered
//!   document; the test runs it twice and compares the whole rendering.
//!
//! # What it does not prove
//!
//! It is **not** the API window and does not render the matrix. The matrix is
//! [`crate::verify`]'s; the shell draws both. It reads one named document, not the whole
//! `docs/` corpus, and it says so rather than counting a sample as the corpus.

use std::path::Path;

use voxel_axis_claim::Claim;
use voxel_axis_unit::Unit;
use voxel_axis_view::Document;
use voxel_model::Identity;

/// The document the first prose walk reads, relative to the workspace root.
pub const DEFAULT_DOCUMENT: &str = "docs/DESIGN-SPEC.md";

/// One real document, its units, its claims and the view built from them.
#[derive(Clone, Debug)]
pub struct Prose {
    /// The artifact id: `artifact_` and forty hex digits over the path and bytes.
    pub artifact: String,
    /// The path that was read.
    pub path: String,
    /// The exact paragraphs.
    pub units: Vec<Unit>,
    /// One claim per unit.
    pub claims: Vec<Claim>,
    /// The three-section view.
    pub document: Document,
}

impl Prose {
    /// Render the document, after validating every line against the claims.
    pub fn render(&self) -> Result<String, String> {
        self.document
            .render_checked(&self.claims)
            .map_err(|refusal| refusal.describe())
    }
}

/// Read and walk `path` under `root`.
///
/// A read failure, a refused tiling, a divergent claim and a view that cannot trace a line are all
/// returned with the source and the position; nothing is defaulted to an empty document.
pub fn scrub(root: &Path, path: &str) -> Result<Prose, String> {
    let target = root.join(path);
    let text = std::fs::read_to_string(&target)
        .map_err(|error| format!("{} could not be read: {error}", target.display()))?;
    let artifact = artifact_id(path, &text);
    let units = Unit::tile_text(&artifact, &text)
        .map_err(|errors| {
            let reasons: Vec<String> = errors.iter().map(|error| error.describe()).collect();
            format!("{path} did not tile exactly: {}", reasons.join("; "))
        })?;
    let claims: Vec<Claim> = units.iter().map(Claim::derive).collect();
    for claim in &claims {
        let unit = units
            .iter()
            .find(|unit| unit.id == claim.unit)
            .expect("every claim was derived from one of these units");
        claim
            .verify(unit)
            .map_err(|refusal| format!("{path}: {}", refusal.detail()))?;
    }
    let document = Document::from_claims("API view — the declared arrangement", &claims);
    document
        .validate(&claims)
        .map_err(|refusal| format!("{path}: {}", refusal.describe()))?;
    Ok(Prose {
        artifact,
        path: path.to_string(),
        units,
        claims,
        document,
    })
}

/// The artifact's own id: the path and the bytes, hashed once.
fn artifact_id(path: &str, text: &str) -> String {
    let digest = Identity::of_fields("shploingus/axis/artifact", &[path.as_bytes(), text.as_bytes()])
        .to_string();
    let body: String = digest
        .trim_start_matches("sha256:")
        .chars()
        .take(40)
        .collect();
    format!("artifact_{body}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> std::path::PathBuf {
        voxel_package::features::workspace_root(Path::new(env!("CARGO_MANIFEST_DIR")))
            .expect("this crate sits inside the workspace")
    }

    #[test]
    fn the_spec_is_tiled_claimed_and_rendered_without_a_gap() {
        let prose = scrub(&root(), DEFAULT_DOCUMENT).expect("the document walks");
        assert!(!prose.units.is_empty(), "a document with no paragraphs is not a document");
        assert_eq!(prose.claims.len(), prose.units.len(), "one claim per unit");
        assert_eq!(
            Unit::concatenate(&prose.units),
            std::fs::read_to_string(root().join(DEFAULT_DOCUMENT)).expect("read"),
            "the walk does not drop or duplicate a character"
        );
        let rendered = prose.render().expect("every line resolves");
        for claim in &prose.claims {
            assert!(rendered.contains(&claim.id), "{} is in the document", claim.id);
        }
    }

    #[test]
    fn the_same_bytes_produce_the_same_units_claims_and_document() {
        let first = scrub(&root(), DEFAULT_DOCUMENT).expect("first walk");
        let second = scrub(&root(), DEFAULT_DOCUMENT).expect("second walk");
        assert_eq!(first.artifact, second.artifact);
        assert_eq!(
            first.units.iter().map(|unit| &unit.id).collect::<Vec<_>>(),
            second.units.iter().map(|unit| &unit.id).collect::<Vec<_>>()
        );
        assert_eq!(first.render().unwrap(), second.render().unwrap());
    }
}
