//! The materials generator — `cargo run -p voxel-model --bin materials-generator`.
//!
//! H-18.1: the generator is Rust, in this tree, and **refuses to write while the
//! gate reports a defect**. It reads the declaration files in `materials/`, runs
//! [`Table::gate`], and only then writes:
//!
//! * `src/materials/generated.rs` — the committed, compiled table;
//! * `src/materials/spectra.rs` — the committed, compiled sampled-appearance data
//!   from `materials/spectra/sources.tsv` and the USGS files beside it (C-140/C-141);
//! * `materials/digest.txt` — the declaration digests and each generated file's own
//!   digest, which `build.rs` recomputes on every build.
//!
//! A generator that writes through a defect is a generator that launders it, so
//! the refusal path exits non-zero and touches nothing.
//!
//! **No runtime parse**: this binary is build-time tooling. The game reads the
//! generated module, which is compiled rather than parsed.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use voxel_model::materials::schema::{
    Availability, Family, HeatKind, Hue, Qty, Sink, Source, Table, Tag, Unit, VocabKind,
};
use voxel_model::materials::declaration;
use voxel_model::refusal::RefusalSet;
use voxel_model::Digest;

const DECLARATION_FILES: [&str; 4] = ["tiers.tsv", "substances.tsv", "processes.tsv", "vocabulary.tsv"];

/// The sampled-appearance sources beside the declaration (C-140/C-141).
const SPECTRA_DIR: &str = "spectra";
const SPECTRA_SOURCES: &str = "spectra/sources.tsv";

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let materials = root.join("materials");
    let read = |name: &str| {
        fs::read_to_string(materials.join(name))
            .unwrap_or_else(|error| panic!("materials/{name} must be readable: {error}"))
    };

    let tiers = match declaration::parse_tiers("tiers.tsv", &read("tiers.tsv")) {
        Ok(tiers) => tiers,
        Err(defects) => fail(&defects),
    };
    let substances = match declaration::parse_substances("substances.tsv", &read("substances.tsv")) {
        Ok(substances) => substances,
        Err(defects) => fail(&defects),
    };
    let processes = match declaration::parse_processes("processes.tsv", &read("processes.tsv")) {
        Ok(processes) => processes,
        Err(defects) => fail(&defects),
    };
    let vocabulary =
        match declaration::parse_vocabulary("vocabulary.tsv", &read("vocabulary.tsv")) {
            Ok(vocabulary) => vocabulary,
            Err(defects) => fail(&defects),
        };

    let table = Table::new(tiers, substances, processes, vocabulary);
    let defects = table.gate();
    if !defects.is_empty() {
        fail(&defects);
    }

    println!(
        "gate clean: {} tiers, {} substances, {} processes, {} vocabulary entries",
        table.tiers.len(),
        table.substances.len(),
        table.processes.len(),
        table.vocabulary.len(),
    );
    for reason in table.open_reasons() {
        println!("open: {reason}");
    }

    let spectra = match spectra_module(&materials) {
        Ok(spectra) => spectra,
        Err(defects) => fail_text(&defects),
    };
    println!(
        "spectra clean: {} samples, {} channel grids",
        spectra.samples, spectra.grids
    );

    let generated = emit(&table);
    let mut declaration_files: Vec<(&str, String)> = Vec::new();
    for name in DECLARATION_FILES {
        declaration_files.push((name, read(name)));
    }
    let declaration_refs: Vec<(&str, &str)> = declaration_files
        .iter()
        .map(|(name, text)| (*name, text.as_str()))
        .collect();
    let declaration_digest = declaration::declaration_digest(&declaration_refs);
    let generated_digest = Digest::of(generated.as_bytes());
    let spectra_generated_digest = Digest::of(spectra.text.as_bytes());

    fs::write(root.join("src/materials/generated.rs"), &generated)
        .unwrap_or_else(|error| panic!("src/materials/generated.rs must be writable: {error}"));
    fs::write(root.join("src/materials/spectra.rs"), &spectra.text)
        .unwrap_or_else(|error| panic!("src/materials/spectra.rs must be writable: {error}"));
    fs::write(
        materials.join("digest.txt"),
        format!(
            "declaration {}\ngenerated {}\nspectra-sources {}\nspectra-generated {}\n",
            declaration_digest.to_hex(),
            generated_digest.to_hex(),
            spectra.sources.to_hex(),
            spectra_generated_digest.to_hex()
        ),
    )
    .unwrap_or_else(|error| panic!("materials/digest.txt must be writable: {error}"));

    println!("wrote src/materials/generated.rs, src/materials/spectra.rs and materials/digest.txt");
}

fn fail(defects: &RefusalSet) -> ! {
    eprintln!("the materials declaration is refused; nothing was written:");
    for refusal in defects.all() {
        eprintln!("  {refusal}");
    }
    std::process::exit(1);
}

fn fail_text(defects: &[String]) -> ! {
    eprintln!("the spectra declaration is refused; nothing was written:");
    for defect in defects {
        eprintln!("  {defect}");
    }
    std::process::exit(1);
}

/// The emitted spectra module and the digest of the sources it was built from.
struct SpectraModule {
    text: String,
    sources: Digest,
    samples: usize,
    grids: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SourceKind {
    Sample,
    Wavelengths,
}

#[derive(Clone)]
struct SourceRow {
    id: String,
    kind: SourceKind,
    instrument: String,
    file: String,
    record: String,
    sha256: String,
    url: String,
    note: String,
}

#[derive(Clone, Copy)]
struct SourceValue {
    missing: bool,
    negative: bool,
    mantissa: u64,
    decimals: u32,
    exponent: i32,
}

struct Grid {
    file: String,
    instrument: String,
    scale: u32,
    raw: Vec<u64>,
}

struct Sample {
    row: SourceRow,
    scale: u32,
    raw: Vec<i64>,
    visible: [(u64, u64); 3],
}

struct LoadedRow<'a> {
    row: &'a SourceRow,
    values: Vec<SourceValue>,
}

/// The declared derivation windows as exact fractions of a micrometre: B
/// [0.38, 0.50), G [0.50, 0.60), R [0.60, 0.75). Lower bound inclusive,
/// upper exclusive, so no channel is used twice.
const WINDOWS: [(u64, u64, u64, u64, &str); 3] = [
    (19, 50, 1, 2, "b"),
    (1, 2, 3, 5, "g"),
    (3, 5, 3, 4, "r"),
];

fn spectra_module(materials: &Path) -> Result<SpectraModule, Vec<String>> {
    let dir = materials.join(SPECTRA_DIR);
    let text = fs::read_to_string(materials.join(SPECTRA_SOURCES))
        .map_err(|error| vec![format!("{SPECTRA_SOURCES} must be readable: {error}")])?;

    let mut defects: Vec<String> = Vec::new();
    let mut rows: Vec<SourceRow> = Vec::new();
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for (number, line) in text.lines().enumerate() {
        let line = line.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != 8 {
            defects.push(format!(
                "sources.tsv line {}: expected 8 tab-separated fields (id, kind, instrument, file, \
                 record, sha256, url, note), found {}",
                number + 1,
                fields.len()
            ));
            continue;
        }
        let kind = match fields[1] {
            "sample" => SourceKind::Sample,
            "wavelengths" => SourceKind::Wavelengths,
            other => {
                defects.push(format!(
                    "sources.tsv line {}: kind {other:?} is not `sample` or `wavelengths`",
                    number + 1
                ));
                continue;
            }
        };
        let id = fields[0].to_string();
        if seen.insert(id.clone(), number + 1).is_some() {
            defects.push(format!(
                "sources.tsv line {}: id {id:?} is declared twice",
                number + 1
            ));
        }
        rows.push(SourceRow {
            id,
            kind,
            instrument: fields[2].to_string(),
            file: fields[3].to_string(),
            record: fields[4].to_string(),
            sha256: fields[5].to_string(),
            url: fields[6].to_string(),
            note: fields[7].to_string(),
        });
    }
    if rows.is_empty() {
        defects.push("sources.tsv declares no spectra".to_string());
    }

    let mut loaded: Vec<LoadedRow> = Vec::new();
    for row in &rows {
        let path = dir.join(&row.file);
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) => {
                defects.push(format!("{}: unreadable: {error}", row.file));
                continue;
            }
        };
        let actual = Digest::of(&bytes).to_hex();
        if !actual.eq_ignore_ascii_case(&row.sha256) {
            defects.push(format!(
                "{}: sha256 {actual} does not match the declared {}",
                row.file, row.sha256
            ));
            continue;
        }
        let file_text = String::from_utf8_lossy(&bytes);
        match parse_spectrum_file(&file_text, &row.file) {
            Ok((record, values)) => {
                if record != row.record {
                    defects.push(format!(
                        "{}: the header record {record} does not match the declared {}",
                        row.file, row.record
                    ));
                } else {
                    loaded.push(LoadedRow { row, values });
                }
            }
            Err(error) => defects.push(error),
        }
    }

    let mut grids: BTreeMap<String, Grid> = BTreeMap::new();
    for entry in &loaded {
        if entry.row.kind == SourceKind::Wavelengths {
            match grid_from(entry) {
                Ok(grid) => {
                    grids.insert(entry.row.instrument.clone(), grid);
                }
                Err(error) => defects.push(error),
            }
        }
    }

    let mut samples: Vec<Sample> = Vec::new();
    for entry in &loaded {
        if entry.row.kind == SourceKind::Sample {
            match grid_for(&grids, &entry.row.instrument) {
                Ok(grid) => match sample_from(entry, grid) {
                    Ok(sample) => samples.push(sample),
                    Err(error) => defects.push(error),
                },
                Err(error) => defects.push(format!("{}: {error}", entry.row.file)),
            }
        }
    }

    if !defects.is_empty() {
        return Err(defects);
    }

    let sources = spectra_sources_digest(&dir).map_err(|error| vec![error])?;
    let text = match emit_spectra(&samples, &grids) {
        Ok(text) => text,
        Err(error) => return Err(vec![error]),
    };
    Ok(SpectraModule {
        text,
        sources,
        samples: samples.len(),
        grids: grids.len(),
    })
}

/// The digest of every file under `materials/spectra/`, name and bytes in
/// sorted order — the same scheme `build.rs` mirrors.
fn spectra_sources_digest(dir: &Path) -> Result<Digest, String> {
    let mut names: Vec<String> = Vec::new();
    let entries =
        fs::read_dir(dir).map_err(|error| format!("{} is unreadable: {error}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("{} is unreadable: {error}", dir.display()))?;
        names.push(entry.file_name().to_string_lossy().into_owned());
    }
    names.sort();
    let mut fields: Vec<Vec<u8>> = Vec::new();
    for name in &names {
        fields.push(name.clone().into_bytes());
        fields.push(
            fs::read(dir.join(name)).map_err(|error| format!("{name} is unreadable: {error}"))?,
        );
    }
    let borrowed: Vec<&[u8]> = fields.iter().map(|field| field.as_slice()).collect();
    Ok(Digest::of_fields("shploingus/materials/spectra", &borrowed))
}

fn parse_spectrum_file(text: &str, label: &str) -> Result<(String, Vec<SourceValue>), String> {
    let mut lines = text
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.trim().is_empty());
    let header = lines.next().ok_or_else(|| format!("{label} is empty"))?;
    let record =
        header_record(header).ok_or_else(|| format!("{label}: no `Record=` in the header"))?;
    let mut values = Vec::new();
    for (index, line) in lines.enumerate() {
        values.push(
            parse_source_value(line)
                .map_err(|error| format!("{label} line {}: {error}", index + 2))?,
        );
    }
    Ok((record, values))
}

fn header_record(header: &str) -> Option<String> {
    let rest = header.split("Record=").nth(1)?;
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        None
    } else {
        Some(digits)
    }
}

fn parse_source_value(line: &str) -> Result<SourceValue, String> {
    let text = line.trim();
    if text.is_empty() {
        return Err("an empty value line".to_string());
    }
    let (negative, body) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => match text.strip_prefix('+') {
            Some(rest) => (false, rest),
            None => (false, text),
        },
    };
    let split = body
        .find(['e', 'E'])
        .ok_or_else(|| format!("no exponent in {text:?}"))?;
    let (mantissa_text, exponent_text) = body.split_at(split);
    let exponent: i32 = exponent_text[1..]
        .parse()
        .map_err(|_| format!("bad exponent in {text:?}"))?;
    let (integer_part, fraction_part) = match mantissa_text.split_once('.') {
        Some((integer, fraction)) => (integer, fraction),
        None => (mantissa_text, ""),
    };
    if integer_part.is_empty() {
        return Err(format!("no integer part in {text:?}"));
    }
    let digits = format!("{integer_part}{fraction_part}");
    let mantissa: u64 = digits
        .parse()
        .map_err(|_| format!("bad mantissa in {text:?}"))?;
    Ok(SourceValue {
        missing: exponent >= 20,
        negative,
        mantissa,
        decimals: fraction_part.len() as u32,
        exponent,
    })
}

fn value_scale(values: &[SourceValue]) -> u32 {
    let mut scale = 0u32;
    for value in values {
        if value.missing {
            continue;
        }
        let need = value.decimals as i64 - value.exponent as i64;
        if need > scale as i64 {
            scale = need as u32;
        }
    }
    scale
}

fn scaled_raw(value: &SourceValue, scale: u32, label: &str) -> Result<i64, String> {
    if value.missing {
        return Ok(i64::MIN);
    }
    let shift = scale as i64 + value.exponent as i64 - value.decimals as i64;
    if shift < 0 {
        return Err(format!("{label}: scale {scale} cannot represent this value"));
    }
    let factor = 10u64
        .checked_pow(shift as u32)
        .ok_or_else(|| format!("{label}: a power of ten overflowed"))?;
    let magnitude = value
        .mantissa
        .checked_mul(factor)
        .ok_or_else(|| format!("{label}: the mantissa overflowed"))?;
    if magnitude > i64::MAX as u64 {
        return Err(format!("{label}: the value does not fit an i64"));
    }
    Ok(if value.negative {
        -(magnitude as i64)
    } else {
        magnitude as i64
    })
}

fn grid_from(entry: &LoadedRow) -> Result<Grid, String> {
    let row = entry.row;
    if entry.values.iter().any(|value| value.missing) {
        return Err(format!("{}: the channel grid has missing values", row.file));
    }
    if entry.values.iter().any(|value| value.negative) {
        return Err(format!(
            "{}: the channel grid has a negative wavelength",
            row.file
        ));
    }
    let scale = value_scale(&entry.values);
    let mut raw = Vec::with_capacity(entry.values.len());
    for (index, value) in entry.values.iter().enumerate() {
        let scaled = scaled_raw(value, scale, &format!("{} channel {}", row.file, index + 1))?;
        if scaled < 0 {
            return Err(format!(
                "{}: negative wavelength at channel {}",
                row.file,
                index + 1
            ));
        }
        raw.push(scaled as u64);
    }
    if raw.is_empty() {
        return Err(format!("{}: no channels", row.file));
    }
    Ok(Grid {
        file: row.file.clone(),
        instrument: row.instrument.clone(),
        scale,
        raw,
    })
}

/// Resolve a sample's instrument token to its family's channel grid: the USGS
/// files name the family (`BECK`, `ASD`) and the spectrometer in the sample
/// token (`BECKa`, `ASDFRa`), so the match is the family prefix. Ambiguity is
/// refused rather than guessed.
fn grid_for<'a>(grids: &'a BTreeMap<String, Grid>, instrument: &str) -> Result<&'a Grid, String> {
    let mut matches = grids
        .iter()
        .filter(|(family, _)| instrument.starts_with(family.as_str()))
        .map(|(_, grid)| grid);
    let first = matches
        .next()
        .ok_or_else(|| format!("the instrument {instrument:?} has no wavelength grid"))?;
    if matches.next().is_some() {
        return Err(format!(
            "the instrument {instrument:?} matches more than one wavelength grid"
        ));
    }
    Ok(first)
}

fn sample_from(entry: &LoadedRow, grid: &Grid) -> Result<Sample, String> {
    let row = entry.row;
    if entry.values.len() != grid.raw.len() {
        return Err(format!(
            "{}: {} channels, but {} has {} — the join fails",
            row.file,
            entry.values.len(),
            grid.file,
            grid.raw.len()
        ));
    }
    let scale = value_scale(&entry.values);
    let mut raw = Vec::with_capacity(entry.values.len());
    for (index, value) in entry.values.iter().enumerate() {
        raw.push(scaled_raw(
            value,
            scale,
            &format!("{} line {}", row.file, index + 2),
        )?);
    }
    let visible = visible_channels(&raw, scale, grid)
        .map_err(|error| format!("{}: {error}", row.file))?;
    Ok(Sample {
        row: row.clone(),
        scale,
        raw,
        visible,
    })
}

fn visible_channels(
    values: &[i64],
    value_scale_power: u32,
    grid: &Grid,
) -> Result<[(u64, u64); 3], String> {
    let mut out = [(0u64, 1u64); 3];
    let value_scale = 10u128.pow(value_scale_power);
    let wavelength_scale = 10u128.pow(grid.scale);
    for (index, (low_num, low_den, high_num, high_den, name)) in WINDOWS.iter().enumerate() {
        let mut sum: i128 = 0;
        let mut count: u64 = 0;
        for (channel, value) in values.iter().enumerate() {
            if *value == i64::MIN {
                continue;
            }
            let wavelength = grid.raw[channel] as u128;
            let within_low =
                wavelength * (*low_den as u128) >= (*low_num as u128) * wavelength_scale;
            let within_high =
                wavelength * (*high_den as u128) < (*high_num as u128) * wavelength_scale;
            if within_low && within_high {
                sum += *value as i128;
                count += 1;
            }
        }
        if count == 0 {
            return Err(format!("the {name} window has no channels in this sample"));
        }
        // Declared: a window mean below zero is written as exactly zero; the
        // retained spectrum keeps the measured value.
        let numerator = if sum < 0 { 0u128 } else { sum as u128 };
        let denominator = count as u128 * value_scale;
        if numerator > u64::MAX as u128 || denominator > u64::MAX as u128 {
            return Err(format!("the {name} window mean does not fit an exact rational"));
        }
        out[index] = (numerator as u64, denominator as u64);
    }
    Ok(out)
}

fn sample_source_string(row: &SourceRow) -> String {
    format!(
        "USGS Spectral Library v7 (Kokaly et al. 2017), {} (record {}), {}: {} — the \
         visible-window mean over the sample's own channel grid (B 0.38–0.50, G 0.50–0.60, R \
         0.60–0.75 µm, lower bound inclusive), exact rationals; the whole sampled spectrum is \
         retained",
        row.file, row.record, row.url, row.note
    )
}

fn emit_array<T: core::fmt::Display>(
    out: &mut String,
    doc: &str,
    name: &str,
    element: &str,
    values: &[T],
) {
    out.push_str(&format!(
        "/// {doc}\npub static {name}: [{element}; {}] = [\n",
        values.len()
    ));
    for chunk in values.chunks(12) {
        out.push_str("    ");
        for value in chunk {
            out.push_str(&format!("{value}, "));
        }
        out.push('\n');
    }
    out.push_str("];\n\n");
}

fn emit_spectra(samples: &[Sample], grids: &BTreeMap<String, Grid>) -> Result<String, String> {
    let mut out = String::new();
    out.push_str(
"//! @generated — written by `cargo run -p voxel-model --bin materials-generator`.
//!
//! The **sampled appearance** source (C-140/C-141): whole USGS splib07a spectra,
//! exact, plus the derived visible channels.
//!
//! * Reflectance is exact: `raw × 10^-value_scale`. `i64::MIN` marks a channel
//!   the library marks missing (its `-1.2300000e+034` sentinel); the sentinel is
//!   never turned into a number.
//! * Wavelengths are exact: `raw × 10^-wavelength_scale` micrometres.
//! * `visible` is the declared derivation: the mean over the sample's own
//!   channels whose wavelength lies in [low, high) of each window in
//!   `VISIBLE_WINDOWS`, as an exact rational. A window mean below zero is
//!   written as exactly zero; the retained spectrum keeps every measured value.
//! * The whole spectrum is retained beside the derivation, so later process and
//!   feature reads use the bands, not only the render channels.
//!
//! Do not edit by hand: `build.rs` refuses the build when this file or the
//! sources under `materials/spectra/` disagree with `materials/digest.txt`.
//! Regenerate instead. Nothing here is parsed at runtime.

use crate::material::{Ratio, Reflectance, SourcedReflectance};
use crate::refusal::Refused;

/// One measured sample: the whole spectrum, exact, plus its derived visible
/// channels and the provenance both share.
#[derive(Clone, Copy, Debug)]
pub struct Spectrum {
    pub id: &'static str,
    /// The instrument token the source file names (`BECKa`, `ASDFRa`, …).
    pub instrument: &'static str,
    pub source_file: &'static str,
    pub usgs_record: &'static str,
    pub source_url: &'static str,
    /// The declared note; a **flagged** representative where the placeholder
    /// role has no composition to derive from.
    pub note: &'static str,
    pub wavelengths: &'static [u64],
    pub wavelength_scale: u32,
    /// `raw × 10^-value_scale`; `i64::MIN` marks a missing channel.
    pub values: &'static [i64],
    pub value_scale: u32,
    /// (r, g, b) visible-window means, exact.
    pub visible: (Ratio, Ratio, Ratio),
    /// Provenance for `visible`, ready for `SourcedReflectance`.
    pub source: &'static str,
}

/// The declared derivation windows, as (low, high) exact fractions of a
/// micrometre, in B, G, R order; lower bound inclusive, upper exclusive.
pub const VISIBLE_WINDOWS: [(Ratio, Ratio); 3] = [
    (Ratio::from_parts(19, 50), Ratio::from_parts(1, 2)),
    (Ratio::from_parts(1, 2), Ratio::from_parts(3, 5)),
    (Ratio::from_parts(3, 5), Ratio::from_parts(3, 4)),
];

",
    );
    for (instrument, grid) in grids {
        emit_array(
            &mut out,
            &format!(
                "{} channels of `{}`, micrometres as `raw × 10^-{}`.",
                grid.raw.len(),
                grid.file,
                grid.scale
            ),
            &format!("{}_WAVELENGTHS", instrument.to_uppercase()),
            "u64",
            &grid.raw,
        );
    }
    for sample in samples {
        let row = &sample.row;
        emit_array(
            &mut out,
            &format!(
                "{} channels of `{}` (`{}` grid), reflectance as `raw × 10^-{}`; `i64::MIN` \
                 marks a missing channel.",
                sample.raw.len(),
                row.file,
                row.instrument,
                sample.scale
            ),
            &format!("{}_VALUES", row.id.to_uppercase()),
            "i64",
            &sample.raw,
        );
    }
    out.push_str(
        "/// Every declared sample, in declaration order.\npub static SAMPLES: &[Spectrum] = &[\n",
    );
    for sample in samples {
        let row = &sample.row;
        let source = sample_source_string(row);
        let grid = grid_for(grids, &row.instrument)?;
        out.push_str("    Spectrum {\n");
        out.push_str(&format!("        id: {},\n", lit(&row.id)));
        out.push_str(&format!("        instrument: {},\n", lit(&row.instrument)));
        out.push_str(&format!("        source_file: {},\n", lit(&row.file)));
        out.push_str(&format!("        usgs_record: {},\n", lit(&row.record)));
        out.push_str(&format!("        source_url: {},\n", lit(&row.url)));
        out.push_str(&format!("        note: {},\n", lit(&row.note)));
        out.push_str(&format!(
            "        wavelengths: &{}_WAVELENGTHS,\n",
            grid.instrument.to_uppercase()
        ));
        out.push_str(&format!("        wavelength_scale: {},\n", grid.scale));
        out.push_str(&format!(
            "        values: &{}_VALUES,\n",
            row.id.to_uppercase()
        ));
        out.push_str(&format!("        value_scale: {},\n", sample.scale));
        let (r, g, b) = (sample.visible[2], sample.visible[1], sample.visible[0]);
        out.push_str(&format!(
            "        visible: (Ratio::from_parts({}, {}), Ratio::from_parts({}, {}), \
             Ratio::from_parts({}, {})),\n",
            r.0, r.1, g.0, g.1, b.0, b.1
        ));
        out.push_str(&format!("        source: {},\n", lit(&source)));
        out.push_str("    },\n");
    }
    out.push_str("];\n\n");
    out.push_str(
"/// Look a sample up by its declared id.
pub fn by_id(id: &str) -> Option<&'static Spectrum> {
    SAMPLES.iter().find(|sample| sample.id == id)
}

/// The exact `SourcedReflectance` the declared derivation produced for `sample`.
pub fn reflectance_of(sample: &Spectrum) -> Refused<SourcedReflectance> {
    Ok(SourcedReflectance {
        value: Reflectance::new(sample.visible.0, sample.visible.1, sample.visible.2)?,
        source: sample.source,
    })
}
",
    );
    for (index, sample) in samples.iter().enumerate() {
        out.push_str(&format!(
            "\n/// The derived `SourcedReflectance` for `{}`.\npub fn {}() -> \
             Refused<SourcedReflectance> {{\n    reflectance_of(&SAMPLES[{index}])\n}}\n",
            sample.row.id, sample.row.id
        ));
    }
    Ok(out)
}

fn ratio(value: voxel_model::Ratio) -> String {
    format!("Ratio::from_parts({}, {})", value.numerator(), value.denominator())
}

fn option_ratio(value: Option<voxel_model::Ratio>) -> String {
    match value {
        Some(value) => format!("Some({})", ratio(value)),
        None => "None".to_string(),
    }
}

fn lit(text: &str) -> String {
    format!("{text:?}")
}

fn family(value: Family) -> &'static str {
    match value {
        Family::Metal => "Family::Metal",
        Family::Enamel => "Family::Enamel",
        Family::Glass => "Family::Glass",
        Family::Ceramic => "Family::Ceramic",
        Family::Polymer => "Family::Polymer",
        Family::Paper => "Family::Paper",
        Family::Road => "Family::Road",
        Family::Water => "Family::Water",
        Family::Organic => "Family::Organic",
        Family::Soil => "Family::Soil",
    }
}

fn hue(value: Hue) -> &'static str {
    match value {
        Hue::Red => "Hue::Red",
        Hue::Amber => "Hue::Amber",
        Hue::Yellow => "Hue::Yellow",
        Hue::Green => "Hue::Green",
        Hue::Cyan => "Hue::Cyan",
        Hue::Blue => "Hue::Blue",
        Hue::Violet => "Hue::Violet",
        Hue::Natural => "Hue::Natural",
    }
}

fn unit(value: Unit) -> &'static str {
    match value {
        Unit::Mass => "Unit::Mass",
        Unit::Volume => "Unit::Volume",
        Unit::Count => "Unit::Count",
        Unit::Gas => "Unit::Gas",
    }
}

fn source(value: Source) -> &'static str {
    match value {
        Source::Mined => "Source::Mined",
        Source::Grown => "Source::Grown",
        Source::Gathered => "Source::Gathered",
        Source::Salvaged => "Source::Salvaged",
        Source::Made => "Source::Made",
    }
}

fn sink(value: Sink) -> &'static str {
    match value {
        Sink::Holding => "Sink::Holding",
        Sink::Atmosphere => "Sink::Atmosphere",
    }
}

fn tag(value: Tag) -> &'static str {
    match value {
        Tag::Fuel => "Tag::Fuel",
        Tag::Edible => "Tag::Edible",
        Tag::Tool => "Tag::Tool",
        Tag::Structure => "Tag::Structure",
        Tag::Precious => "Tag::Precious",
        Tag::Salvage => "Tag::Salvage",
    }
}

fn availability(value: Availability) -> &'static str {
    match value {
        Availability::Read => "Availability::Read",
        Availability::Declared => "Availability::Declared",
        Availability::NotYetSourced => "Availability::NotYetSourced",
    }
}

fn heat_kind(value: HeatKind) -> &'static str {
    match value {
        HeatKind::Material => "HeatKind::Material",
        HeatKind::Flame => "HeatKind::Flame",
    }
}

fn vocab_kind(value: VocabKind) -> &'static str {
    match value {
        VocabKind::Structure => "VocabKind::Structure",
        VocabKind::Tool => "VocabKind::Tool",
        VocabKind::Skill => "VocabKind::Skill",
    }
}

fn option_string(value: &Option<String>) -> String {
    match value {
        Some(value) => format!("Some({}.to_string())", lit(value)),
        None => "None".to_string(),
    }
}

fn side(quantities: &[Qty]) -> String {
    if quantities.is_empty() {
        return "vec![]".to_string();
    }
    let items: Vec<String> = quantities
        .iter()
        .map(|qty| format!("Qty::new({}, {})", lit(&qty.substance), ratio(qty.amount)))
        .collect();
    format!("vec![{}]", items.join(", "))
}

/// Emit the committed table as Rust source. Deterministic: the declaration's
/// order, with substances sorted by name (they live in a `BTreeMap`).
fn emit(table: &Table) -> String {
    let mut out = String::new();
    out.push_str(
        "//! @generated — written by `cargo run -p voxel-model --bin materials-generator`.\n//!\n\
         //! Do not edit by hand: `build.rs` refuses the build when this file, the\n\
         //! declaration files beside the crate, or `materials/digest.txt` disagree.\n\
         //! Regenerate instead. Nothing here is parsed at runtime.\n\n",
    );
    out.push_str(
        "use crate::material::Ratio;\nuse crate::materials::schema::{\n    Availability, Family, \
         HeatKind, Hue, Mechanism, Process, Qty, Requires, Sink, Source, Substance, Table, Tag, \
         Tier, Unit, VocabKind, Vocabulary,\n};\n\n",
    );
    out.push_str("/// The committed table, compiled rather than parsed.\npub fn table() -> Table {\n    let mut tiers = Vec::new();\n");
    for tier in &table.tiers {
        out.push_str(&format!(
            "    tiers.push(Tier {{ name: {}.to_string(), ordinal: {}, note: {}.to_string() }});\n",
            lit(&tier.name),
            tier.ordinal,
            lit(&tier.note)
        ));
    }

    out.push_str("    let mut substances = Vec::new();\n");
    for substance in table.substances.values() {
        let tags: Vec<&str> = substance.tags.iter().map(|t| tag(*t)).collect();
        out.push_str("    substances.push(Substance {\n");
        out.push_str(&format!("        name: {}.to_string(),\n", lit(&substance.name)));
        out.push_str(&format!("        family: {},\n", family(substance.family)));
        out.push_str(&format!("        hue: {},\n", hue(substance.hue)));
        out.push_str(&format!("        unit: {},\n", unit(substance.unit)));
        out.push_str(&format!(
            "        density_g_ml: {},\n",
            option_ratio(substance.density_g_ml)
        ));
        out.push_str(&format!(
            "        unit_mass_g: {},\n",
            option_ratio(substance.unit_mass_g)
        ));
        out.push_str(&format!("        rot_per_day: {},\n", ratio(substance.rot_per_day)));
        out.push_str(&format!("        tags: vec![{}],\n", tags.join(", ")));
        out.push_str(&format!("        source: {},\n", source(substance.source)));
        out.push_str(&format!("        sink: {},\n", sink(substance.sink)));
        out.push_str(&format!(
            "        display: ({}.to_string(), {}),\n",
            lit(&substance.display.0),
            option_ratio(substance.display.1)
        ));
        out.push_str(&format!("        porosity: {},\n", option_ratio(substance.porosity)));
        out.push_str(&format!(
            "        fluid_capacity: {},\n",
            option_ratio(substance.fluid_capacity)
        ));
        out.push_str(&format!(
            "        provenance: {},\n",
            availability(substance.provenance)
        ));
        out.push_str(&format!("        note: {}.to_string(),\n", lit(&substance.note)));
        out.push_str(&format!(
            "        no_consumer: {},\n",
            option_string(&substance.no_consumer)
        ));
        out.push_str("    });\n");
    }

    out.push_str("    let mut processes = Vec::new();\n");
    for process in &table.processes {
        out.push_str("    processes.push(Process {\n");
        out.push_str(&format!("        name: {}.to_string(),\n", lit(&process.name)));
        out.push_str(&format!("        tier: {}.to_string(),\n", lit(&process.tier)));
        out.push_str(&format!("        inputs: {},\n", side(&process.inputs)));
        out.push_str(&format!("        outputs: {},\n", side(&process.outputs)));
        out.push_str("        mechanism: Mechanism {\n");
        out.push_str(&format!(
            "            heat_c: {},\n",
            option_ratio(process.mechanism.heat_c)
        ));
        out.push_str(&format!(
            "            heat_kind: {},\n",
            match process.mechanism.heat_kind {
                Some(kind) => format!("Some({})", heat_kind(kind)),
                None => "None".to_string(),
            }
        ));
        out.push_str(&format!(
            "            hours: {},\n",
            ratio(process.mechanism.hours)
        ));
        out.push_str(&format!(
            "            labour_hours: {},\n",
            ratio(process.mechanism.labour_hours)
        ));
        out.push_str(&format!(
            "            power_kw: {},\n",
            ratio(process.mechanism.power_kw)
        ));
        out.push_str("        },\n");
        out.push_str(&format!(
            "        requires: Requires {{ structure: {}, tool: {}, skill: {} }},\n",
            option_string(&process.requires.structure),
            option_string(&process.requires.tool),
            option_string(&process.requires.skill)
        ));
        out.push_str(&format!(
            "        provenance: {},\n",
            availability(process.provenance)
        ));
        out.push_str(&format!("        note: {}.to_string(),\n", lit(&process.note)));
        out.push_str("    });\n");
    }

    out.push_str("    let mut vocabulary = Vec::new();\n");
    for entry in &table.vocabulary {
        out.push_str(&format!(
            "    vocabulary.push(Vocabulary {{ kind: {}, name: {}.to_string(), note: {}.to_string() }});\n",
            vocab_kind(entry.kind),
            lit(&entry.name),
            lit(&entry.note)
        ));
    }

    out.push_str("    Table::new(tiers, substances, processes, vocabulary)\n}\n");
    out
}
