//! The materials digest gate (H-18.1).
//!
//! The committed table `src/materials/generated.rs` is written by
//! `cargo run -p voxel-model --bin materials-generator` from the declaration
//! files in `materials/`. This build script recomputes both digests and
//! **fails the build** when they disagree with `materials/digest.txt`: a table
//! that no longer matches its declaration is refused rather than compiled, and
//! a hand-edited table cannot pass.
//!
//! It mirrors `materials::declaration::declaration_digest` exactly — a build
//! script cannot depend on the crate it builds. The scheme is
//! [`voxel_model::Digest::of_fields`]'s: tag length + tag + field count +
//! (length + bytes) per field, with each declaration file contributing its name
//! and its bytes in order.
//!
//! The same gate covers the **sampled appearance** (C-140/C-141): the files
//! under `materials/spectra/` (name + bytes in sorted order) and the generated
//! `src/materials/spectra.rs` are recomputed and compared.
//!
//! # Regenerating (the door)
//!
//! The generator is a binary in this same package, so a stale record blocks the
//! build of the very tool that rewrites it. The `materials-regeneration`
//! feature is the deliberate way out: with it on, this script prints a warning
//! and verifies **nothing**, so `cargo run -p voxel-model --bin
//! materials-generator --features materials-regeneration` can rebuild the
//! record. Normal builds and the tamper tests never enable it, so the gate
//! still bites everywhere else.

use std::env;
use std::fs;
use std::path::PathBuf;

use sha2::{Digest as _, Sha256};

const TAG: &str = "shploingus/materials/declaration";
const SPECTRA_TAG: &str = "shploingus/materials/spectra";
const FILES: [&str; 4] = ["tiers.tsv", "substances.tsv", "processes.tsv", "vocabulary.tsv"];
const REGENERATE: &str =
    "run `cargo run -p voxel-model --bin materials-generator` to regenerate the table (add \
     `--features materials-regeneration` when this gate is what blocks that build)";

fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("Cargo manifest directory"));
    let materials = root.join("materials");
    println!("cargo:rerun-if-changed={}", materials.display());
    println!(
        "cargo:rerun-if-changed={}",
        root.join("src/materials/generated.rs").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        root.join("src/materials/spectra.rs").display()
    );

    if env::var_os("CARGO_FEATURE_MATERIALS_REGENERATION").is_some() {
        println!(
            "cargo:warning=the `materials-regeneration` feature is on: this build verifies no \
             digest, so that the generator can rewrite a stale record. Run `cargo run -p \
             voxel-model --bin materials-generator --features materials-regeneration` now; every \
             normal build verifies again."
        );
        return;
    }

    let record = fs::read_to_string(materials.join("digest.txt")).unwrap_or_else(|error| {
        panic!("materials/digest.txt is unreadable ({error}); {REGENERATE}")
    });
    let mut declared_hex = None;
    let mut generated_hex = None;
    let mut spectra_sources_hex = None;
    let mut spectra_generated_hex = None;
    for line in record.lines() {
        let mut parts = line.split_whitespace();
        match (parts.next(), parts.next()) {
            (Some("declaration"), Some(hex)) => declared_hex = Some(hex.to_string()),
            (Some("generated"), Some(hex)) => generated_hex = Some(hex.to_string()),
            (Some("spectra-sources"), Some(hex)) => spectra_sources_hex = Some(hex.to_string()),
            (Some("spectra-generated"), Some(hex)) => spectra_generated_hex = Some(hex.to_string()),
            _ => panic!(
                "materials/digest.txt has a line that is not `declaration <hex>`, `generated \
                 <hex>`, `spectra-sources <hex>` or `spectra-generated <hex>`: {line:?}; \
                 {REGENERATE}"
            ),
        }
    }
    let (
        Some(declared_hex),
        Some(generated_hex),
        Some(spectra_sources_hex),
        Some(spectra_generated_hex),
    ) = (
        declared_hex,
        generated_hex,
        spectra_sources_hex,
        spectra_generated_hex,
    )
    else {
        panic!("materials/digest.txt is missing a digest line; {REGENERATE}")
    };

    let mut fields: Vec<Vec<u8>> = Vec::new();
    for name in FILES {
        let text = fs::read_to_string(materials.join(name))
            .unwrap_or_else(|error| panic!("materials/{name} is unreadable: {error}"));
        fields.push(name.as_bytes().to_vec());
        fields.push(text.into_bytes());
    }
    let borrowed: Vec<&[u8]> = fields.iter().map(|field| field.as_slice()).collect();
    let declaration_digest = fields_digest(TAG, &borrowed);
    if declaration_digest != declared_hex {
        panic!(
            "the materials declaration changed after the table was generated: digest \
             {declaration_digest} does not match the recorded {declared_hex}; {REGENERATE}"
        );
    }

    let spectra_dir = materials.join("spectra");
    let mut names: Vec<String> = Vec::new();
    for entry in fs::read_dir(&spectra_dir).unwrap_or_else(|error| {
        panic!("{} is unreadable ({error}); {REGENERATE}", spectra_dir.display())
    }) {
        let entry = entry.unwrap_or_else(|error| {
            panic!("{} is unreadable ({error}); {REGENERATE}", spectra_dir.display())
        });
        names.push(entry.file_name().to_string_lossy().into_owned());
    }
    names.sort();
    let mut spectra_fields: Vec<Vec<u8>> = Vec::new();
    for name in &names {
        spectra_fields.push(name.clone().into_bytes());
        spectra_fields.push(fs::read(spectra_dir.join(name)).unwrap_or_else(|error| {
            panic!("{name} is unreadable ({error}); {REGENERATE}")
        }));
    }
    let spectra_borrowed: Vec<&[u8]> = spectra_fields
        .iter()
        .map(|field| field.as_slice())
        .collect();
    let spectra_sources = fields_digest(SPECTRA_TAG, &spectra_borrowed);
    if spectra_sources != spectra_sources_hex {
        panic!(
            "the spectra sources under `materials/spectra/` changed after the module was \
             generated: digest {spectra_sources} does not match the recorded \
             {spectra_sources_hex}; {REGENERATE}"
        );
    }

    let generated_path = root.join("src/materials/generated.rs");
    let generated = fs::read(&generated_path)
        .unwrap_or_else(|error| panic!("{} is unreadable ({error}); {REGENERATE}", generated_path.display()));
    let generated_digest = hex(&Sha256::digest(&generated));
    if generated_digest != generated_hex {
        panic!(
            "{} does not match the digest recorded for it: {generated_digest} vs {generated_hex}; \
             {REGENERATE}",
            generated_path.display()
        );
    }

    let spectra_path = root.join("src/materials/spectra.rs");
    let spectra = fs::read(&spectra_path).unwrap_or_else(|error| {
        panic!("{} is unreadable ({error}); {REGENERATE}", spectra_path.display())
    });
    let spectra_digest = hex(&Sha256::digest(&spectra));
    if spectra_digest != spectra_generated_hex {
        panic!(
            "{} does not match the digest recorded for it: {spectra_digest} vs \
             {spectra_generated_hex}; {REGENERATE}",
            spectra_path.display()
        );
    }
}

fn fields_digest(tag: &str, fields: &[&[u8]]) -> String {
    let mut hasher = Sha256::new();
    hasher.update((tag.len() as u64).to_le_bytes());
    hasher.update(tag.as_bytes());
    hasher.update((fields.len() as u64).to_le_bytes());
    for field in fields {
        hasher.update((field.len() as u64).to_le_bytes());
        hasher.update(field);
    }
    hex(&hasher.finalize())
}

fn hex(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}
