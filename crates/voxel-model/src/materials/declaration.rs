//! The declaration format and its parser — **build-time only**.
//!
//! The declaration files (`materials/*.tsv` beside this crate) are the one place
//! a person edits a row. The generator parses them, runs [`crate::materials::schema::Table::gate`],
//! and refuses to write while any defect stands; the game itself never sees this
//! module's output — it reads the committed [`crate::materials::generated`] table,
//! which is compiled rather than parsed (H-18.1).
//!
//! # The format
//!
//! Tab-separated, one row per line, with a **named header line** first (blank
//! lines and lines starting with `#` are skipped). A cell is either a value or
//! `-`, which means **absent** — and absent is a state, never a zero. Numbers
//! are exact rationals, `n` or `n/d`; anything that parses as a float is refused
//! by name ([`RefusalKind::NonRationalQuantity`]).
//!
//! Every defect carries the row it was found on — the file's label, the line
//! number and the row's name — because `line 41 (flue_gas): density "1.3" is not
//! an exact rational` is a sentence someone can act on.

use crate::identity::Digest;
use crate::material::Ratio;
use crate::materials::schema::{
    parse_ratio, Availability, Family, HeatKind, Hue, Mechanism, Process, Qty, Requires, Sink,
    Source, State, Substance, Tag, Tier, Unit, VocabKind, Vocabulary,
};
use crate::refusal::{RefusalKind, RefusalSet};

/// One parsed data row: the header's columns resolved to cells.
struct Row<'a> {
    line: usize,
    name: String,
    cells: Vec<&'a str>,
}

impl<'a> Row<'a> {
    fn cell(
        &self,
        columns: &[&str],
        column: &str,
        _label: &str,
        _defects: &mut RefusalSet,
    ) -> Option<&'a str> {
        let index = columns.iter().position(|c| *c == column)?;
        match self.cells.get(index).copied() {
            Some("-") | None | Some("") => None,
            Some(value) => Some(value),
        }
    }

    fn required(&self, columns: &[&str], column: &str, label: &str, defects: &mut RefusalSet) -> Option<String> {
        if let Some(value) = self.cell(columns, column, label, defects) {
            return Some(value.to_string());
        }
        defects.refuse(
            RefusalKind::FieldDataMissing,
            format!("{label} line {} ({}): column {column:?} is required", self.line, self.name),
        );
        None
    }
}

/// Split a declaration into a header and its rows, refusing anything malformed
/// about the *shape* of the file (duplicate or unknown columns, ragged rows).
fn read_rows<'a>(
    label: &str,
    text: &'a str,
    known: &[&str],
    required: &[&str],
) -> Result<(Vec<&'a str>, Vec<Row<'a>>), RefusalSet> {
    let mut defects = RefusalSet::new();
    let mut lines = text
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty() && !line.trim_start().starts_with('#'));

    let Some((header_line, header)) = lines.next() else {
        defects.refuse(RefusalKind::FieldDataMissing, format!("{label}: no header line"));
        return Err(defects);
    };
    let columns: Vec<&str> = header.split('\t').map(str::trim).collect();
    for column in &columns {
        if !known.contains(column) {
            defects.refuse(
                RefusalKind::UndeclaredName,
                format!("{label} line {}: {column:?} is not a declared column", header_line + 1),
            );
        }
    }
    for required_column in required {
        if !columns.contains(required_column) {
            defects.refuse(
                RefusalKind::FieldDataMissing,
                format!("{label}: required column {required_column:?} is absent"),
            );
        }
    }

    let mut rows = Vec::new();
    for (line, text) in lines {
        let cells: Vec<&str> = text.split('\t').map(str::trim).collect();
        if cells.len() != columns.len() {
            defects.refuse(
                RefusalKind::MalformedRecord,
                format!(
                    "{label} line {}: {} cells for {} columns",
                    line + 1,
                    cells.len(),
                    columns.len()
                ),
            );
            continue;
        }
        let name = cells
            .first()
            .map(|name| name.trim_start_matches('#').trim().to_string())
            .unwrap_or_default();
        rows.push(Row { line: line + 1, name, cells });
    }
    defects.into_result((columns, rows))
}

fn ratio_or_absent(
    value: Option<&str>,
    label: &str,
    row: &Row<'_>,
    field: &str,
    defects: &mut RefusalSet,
) -> Option<Ratio> {
    value.and_then(|text| match parse_ratio(text) {
        Ok(ratio) => Some(ratio),
        Err(refusal) => {
            defects.refuse(
                refusal.kind(),
                format!("{label} line {} ({}): {field} {:?}: {}", row.line, row.name, text, refusal.detail()),
            );
            None
        }
    })
}

fn ratio_or_zero(
    value: Option<&str>,
    label: &str,
    row: &Row<'_>,
    field: &str,
    defects: &mut RefusalSet,
) -> Ratio {
    ratio_or_absent(value, label, row, field, defects).unwrap_or(Ratio::ZERO)
}

const SUBSTANCE_COLUMNS: &[&str] = &[
    "name", "family", "hue", "unit", "state", "phase_of", "density", "unit_mass", "rot", "tags",
    "source", "sink", "display_unit", "display_grams", "porosity", "fluid_capacity", "provenance",
    "note", "no_consumer",
];
const SUBSTANCE_REQUIRED: &[&str] = &["name", "family", "hue", "unit", "state", "source", "provenance"];

/// Parse the substances declaration.
pub fn parse_substances(label: &str, text: &str) -> Result<Vec<Substance>, RefusalSet> {
    let (columns, rows) = read_rows(label, text, SUBSTANCE_COLUMNS, SUBSTANCE_REQUIRED)?;
    let mut defects = RefusalSet::new();
    let mut substances = Vec::new();
    for row in &rows {
        let name = row.required(&columns, "name", label, &mut defects).unwrap_or_default();
        let family = row
            .required(&columns, "family", label, &mut defects)
            .and_then(|text| match Family::parse(&text) {
                Ok(family) => Some(family),
                Err(refusal) => {
                    defects.refuse(refusal.kind(), format!("{label} line {} ({name}): {text:?} is not a declared family", row.line));
                    None
                }
            });
        let hue = row
            .required(&columns, "hue", label, &mut defects)
            .and_then(|text| match Hue::parse(&text) {
                Ok(hue) => Some(hue),
                Err(refusal) => {
                    defects.refuse(refusal.kind(), format!("{label} line {} ({name}): {text:?} is not a declared hue", row.line));
                    None
                }
            });
        let unit = row
            .required(&columns, "unit", label, &mut defects)
            .and_then(|text| match Unit::parse(&text) {
                Ok(unit) => Some(unit),
                Err(refusal) => {
                    defects.refuse(refusal.kind(), format!("{label} line {} ({name}): {text:?} is not a canonical unit", row.line));
                    None
                }
            });
        let state = row
            .required(&columns, "state", label, &mut defects)
            .and_then(|text| match State::parse(&text) {
                Ok(state) => Some(state),
                Err(refusal) => {
                    defects.refuse(refusal.kind(), format!("{label} line {} ({name}): {text:?} is not a declared state", row.line));
                    None
                }
            });
        let phase_of = row.cell(&columns, "phase_of", label, &mut defects)
            .map(str::to_string);
        let source = row
            .required(&columns, "source", label, &mut defects)
            .and_then(|text| match Source::parse(&text) {
                Ok(source) => Some(source),
                Err(refusal) => {
                    defects.refuse(refusal.kind(), format!("{label} line {} ({name}): {text:?} is not a declared source", row.line));
                    None
                }
            });
        let provenance = row
            .required(&columns, "provenance", label, &mut defects)
            .and_then(|text| match Availability::parse(&text) {
                Ok(provenance) => Some(provenance),
                Err(refusal) => {
                    defects.refuse(refusal.kind(), format!("{label} line {} ({name}): {text:?} is not one of V/D/NS", row.line));
                    None
                }
            });

        let density = ratio_or_absent(row.cell(&columns, "density", label, &mut defects), label, row, "density", &mut defects);
        let unit_mass = ratio_or_absent(row.cell(&columns, "unit_mass", label, &mut defects), label, row, "unit_mass", &mut defects);
        let rot = ratio_or_zero(row.cell(&columns, "rot", label, &mut defects), label, row, "rot", &mut defects);
        let porosity = ratio_or_absent(row.cell(&columns, "porosity", label, &mut defects), label, row, "porosity", &mut defects);
        let fluid_capacity = ratio_or_absent(row.cell(&columns, "fluid_capacity", label, &mut defects), label, row, "fluid_capacity", &mut defects);

        let mut tags = Vec::new();
        if let Some(text) = row.cell(&columns, "tags", label, &mut defects) {
            for tag_text in text.split(',').map(str::trim).filter(|t| !t.is_empty()) {
                match Tag::parse(tag_text) {
                    Ok(tag) => tags.push(tag),
                    Err(refusal) => {
                        defects.refuse(
                            refusal.kind(),
                            format!("{label} line {} ({name}): tag {:?} is not in the closed set", row.line, tag_text),
                        );
                    }
                }
            }
        }

        let sink = match row.cell(&columns, "sink", label, &mut defects) {
            None => Sink::Holding,
            Some(text) => match Sink::parse(text) {
                Ok(sink) => sink,
                Err(refusal) => {
                    defects.refuse(refusal.kind(), format!("{label} line {} ({name}): {text:?} is not a declared sink", row.line));
                    Sink::Holding
                }
            },
        };
        let display_unit = row.required(&columns, "display_unit", label, &mut defects).unwrap_or_default();
        let display_grams = ratio_or_absent(row.cell(&columns, "display_grams", label, &mut defects), label, row, "display_grams", &mut defects);
        let note = row.cell(&columns, "note", label, &mut defects).unwrap_or_default().to_string();
        let no_consumer = row.cell(&columns, "no_consumer", label, &mut defects).map(str::to_string);

        match (family, hue, unit, state, source, provenance) {
            (Some(family), Some(hue), Some(unit), Some(state), Some(source), Some(provenance)) => {
                substances.push(Substance {
                    name,
                    family,
                    hue,
                    unit,
                    state,
                    phase_of,
                    density_g_ml: density,
                    unit_mass_g: unit_mass,
                    rot_per_day: rot,
                    tags,
                    source,
                    sink,
                    display: (display_unit, display_grams),
                    porosity,
                    fluid_capacity,
                    provenance,
                    note,
                    no_consumer,
                });
            }
            _ => continue, // the missing cell already refused
        }
    }
    defects.into_result(substances)
}

/// Parse a `sub*ratio;sub*ratio` list. `-` is an empty side (a gather's inputs).
fn parse_side(
    value: Option<&str>,
    label: &str,
    row: &Row<'_>,
    field: &str,
    defects: &mut RefusalSet,
) -> Vec<Qty> {
    let mut side = Vec::new();
    let Some(text) = value else { return side };
    for item in text.split(';').map(str::trim).filter(|item| !item.is_empty()) {
        let Some((substance, amount)) = item.split_once('*') else {
            defects.refuse(
                RefusalKind::MalformedRecord,
                format!("{label} line {} ({}): {field} item {item:?} is not `substance*amount`", row.line, row.name),
            );
            continue;
        };
        match parse_ratio(amount.trim()) {
            Ok(amount) => side.push(Qty::new(substance.trim(), amount)),
            Err(refusal) => {
                defects.refuse(
                    refusal.kind(),
                    format!("{label} line {} ({}): {field} {amount:?} for {substance:?}: {}", row.line, row.name, refusal.detail()),
                );
            }
        }
    }
    side
}

const PROCESS_COLUMNS: &[&str] = &[
    "name", "tier", "inputs", "outputs", "heat_c", "heat_kind", "hours", "labour_hours", "power_kw",
    "structure", "tool", "skill", "source", "provenance", "note",
];
const PROCESS_REQUIRED: &[&str] = &["name", "tier", "outputs", "source", "provenance"];

/// Parse the processes declaration.
pub fn parse_processes(label: &str, text: &str) -> Result<Vec<Process>, RefusalSet> {
    let (columns, rows) = read_rows(label, text, PROCESS_COLUMNS, PROCESS_REQUIRED)?;
    let mut defects = RefusalSet::new();
    let mut processes = Vec::new();
    for row in &rows {
        let name = row.required(&columns, "name", label, &mut defects).unwrap_or_default();
        let tier = row.required(&columns, "tier", label, &mut defects).unwrap_or_default();
        let inputs = parse_side(row.cell(&columns, "inputs", label, &mut defects), label, row, "inputs", &mut defects);
        let outputs = parse_side(row.cell(&columns, "outputs", label, &mut defects), label, row, "outputs", &mut defects);
        let heat_c = ratio_or_absent(row.cell(&columns, "heat_c", label, &mut defects), label, row, "heat_c", &mut defects);
        let heat_kind = match row.cell(&columns, "heat_kind", label, &mut defects) {
            None => None,
            Some(text) => match HeatKind::parse(text) {
                Ok(kind) => Some(kind),
                Err(refusal) => {
                    defects.refuse(refusal.kind(), format!("{label} line {} ({name}): {text:?} is not material/flame", row.line));
                    None
                }
            },
        };
        let hours = ratio_or_zero(row.cell(&columns, "hours", label, &mut defects), label, row, "hours", &mut defects);
        let labour_hours = ratio_or_zero(row.cell(&columns, "labour_hours", label, &mut defects), label, row, "labour_hours", &mut defects);
        let power_kw = ratio_or_zero(row.cell(&columns, "power_kw", label, &mut defects), label, row, "power_kw", &mut defects);
        let provenance = row
            .required(&columns, "provenance", label, &mut defects)
            .and_then(|text| match Availability::parse(&text) {
                Ok(provenance) => Some(provenance),
                Err(refusal) => {
                    defects.refuse(refusal.kind(), format!("{label} line {} ({name}): {text:?} is not one of V/D/NS", row.line));
                    None
                }
            });
        let requires = Requires {
            structure: row.cell(&columns, "structure", label, &mut defects).map(str::to_string),
            tool: row.cell(&columns, "tool", label, &mut defects).map(str::to_string),
            skill: row.cell(&columns, "skill", label, &mut defects).map(str::to_string),
        };
        let note = row.cell(&columns, "note", label, &mut defects).unwrap_or_default().to_string();
        let source = row.required(&columns, "source", label, &mut defects).unwrap_or_default();

        if let Some(provenance) = provenance {
            processes.push(Process {
                name,
                tier,
                inputs,
                outputs,
                mechanism: Mechanism { heat_c, heat_kind, hours, labour_hours, power_kw },
                requires,
                source,
                provenance,
                note,
            });
        }
    }
    defects.into_result(processes)
}

const TIER_COLUMNS: &[&str] = &["name", "ordinal", "note"];
const TIER_REQUIRED: &[&str] = &["name", "ordinal"];

/// Parse the tiers declaration.
pub fn parse_tiers(label: &str, text: &str) -> Result<Vec<Tier>, RefusalSet> {
    let (columns, rows) = read_rows(label, text, TIER_COLUMNS, TIER_REQUIRED)?;
    let mut defects = RefusalSet::new();
    let mut tiers = Vec::new();
    for row in &rows {
        let name = row.required(&columns, "name", label, &mut defects).unwrap_or_default();
        let ordinal = row
            .required(&columns, "ordinal", label, &mut defects)
            .and_then(|text| match text.parse::<u32>() {
                Ok(ordinal) => Some(ordinal),
                Err(_) => {
                    defects.refuse(
                        RefusalKind::NonRationalQuantity,
                        format!("{label} line {} ({name}): ordinal {text:?} is not a whole number", row.line),
                    );
                    None
                }
            });
        let note = row.cell(&columns, "note", label, &mut defects).unwrap_or_default().to_string();
        if let Some(ordinal) = ordinal {
            tiers.push(Tier { name, ordinal, note });
        }
    }
    defects.into_result(tiers)
}

const VOCABULARY_COLUMNS: &[&str] = &["kind", "name", "note"];
const VOCABULARY_REQUIRED: &[&str] = &["kind", "name"];

/// Parse the vocabulary declaration — the things that must *stand* for work to
/// be possible.
pub fn parse_vocabulary(label: &str, text: &str) -> Result<Vec<Vocabulary>, RefusalSet> {
    let (columns, rows) = read_rows(label, text, VOCABULARY_COLUMNS, VOCABULARY_REQUIRED)?;
    let mut defects = RefusalSet::new();
    let mut entries = Vec::new();
    for row in &rows {
        let name = row.required(&columns, "name", label, &mut defects).unwrap_or_default();
        let kind = row
            .required(&columns, "kind", label, &mut defects)
            .and_then(|text| match VocabKind::parse(&text) {
                Ok(kind) => Some(kind),
                Err(refusal) => {
                    defects.refuse(refusal.kind(), format!("{label} line {} ({name}): {text:?} is not structure/tool/skill", row.line));
                    None
                }
            });
        let note = row.cell(&columns, "note", label, &mut defects).unwrap_or_default().to_string();
        if let Some(kind) = kind {
            entries.push(Vocabulary { kind, name, note });
        }
    }
    defects.into_result(entries)
}

/// The digest of a declaration set: each file's **name and bytes**, in order,
/// domain-separated and length-prefixed by [`Digest::of_fields`]. A renamed file
/// or a spliced pair of files is a different digest, which is what lets
/// `build.rs` check exactly the bytes the generator read.
///
/// `build.rs` cannot call this function (a build script cannot depend on the
/// crate it builds), so it mirrors the scheme; the generator and the mirror are
/// exercised against each other by every build and by the integration test.
pub fn declaration_digest(files: &[(&str, &str)]) -> Digest {
    let mut fields: Vec<Vec<u8>> = Vec::new();
    for (name, text) in files {
        fields.push(name.as_bytes().to_vec());
        fields.push(text.as_bytes().to_vec());
    }
    let borrowed: Vec<&[u8]> = fields.iter().map(|field| field.as_slice()).collect();
    Digest::of_fields("shploingus/materials/declaration", &borrowed)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SUBSTANCES: &str = "\
name\tfamily\thue\tunit\tstate\tphase_of\tdensity\tunit_mass\trot\ttags\tsource\tsink\tdisplay_unit\tdisplay_grams\tporosity\tfluid_capacity\tprovenance\tnote\tno_consumer
timber\torganic\tnatural\tMass\tSolid\t-\t-\t-\t0\tfuel,structure\tgathered\tholding\tkg\t1000\t-\t-\tV\tthe first material a founder touches\t-
water\twater\tnatural\tVolume\tLiquid\t-\t1\t-\t0\t-\tgathered\tholding\tL\t1000\t-\t-\tV\tdensity of 1 g/mL, exact by construction\tthe world draws it
flue_gas\tsoil\tnatural\tGas\tGas\t-\t13/10000\t-\t0\t-\tmade\tatmosphere\tL\t1000\t-\t-\tV\tair's own density at STP\tup the chimney
";

    #[test]
    fn the_declaration_parses_into_typed_rows() {
        let substances = parse_substances("test", SUBSTANCES).expect("a clean declaration");
        assert_eq!(substances.len(), 3);
        let water = &substances[1];
        assert_eq!(water.unit, Unit::Volume);
        assert_eq!(water.state, State::Liquid);
        assert_eq!(water.density_g_ml, Some(Ratio::ONE));
        assert_eq!(water.sink, Sink::Holding);
        let gas = &substances[2];
        assert_eq!(gas.state, State::Gas);
        assert_eq!(gas.sink, Sink::Atmosphere);
        assert_eq!(gas.density_g_ml, Some(Ratio::new(13, 10_000).unwrap()));
        let mut timber = substances[0].clone();
        assert_eq!(timber.tags, vec![Tag::Fuel, Tag::Structure]);
        timber.no_consumer = None;
    }

    #[test]
    fn a_float_is_refused_and_the_row_is_named() {
        let text = SUBSTANCES.replace("13/10000", "1.3");
        let defects = parse_substances("test", &text).expect_err("a float");
        assert_eq!(defects.len(), 1);
        assert_eq!(defects.all()[0].kind(), RefusalKind::NonRationalQuantity);
        assert!(
            defects.all()[0].detail().contains("flue_gas") && defects.all()[0].detail().contains("1.3"),
            "{}",
            defects.all()[0].detail()
        );
    }

    #[test]
    fn every_shallow_thing_wrong_with_a_file_is_refused_at_once() {
        // An unknown column, a missing required column, a ragged row.
        let text = SUBSTANCES.replace("source\tsink", "sauce\tsink") + "ragged\trow\n";
        let defects = parse_substances("test", &text).expect_err("defects");
        let kinds: Vec<RefusalKind> = defects.all().iter().map(|r| r.kind()).collect();
        assert!(kinds.contains(&RefusalKind::UndeclaredName));
        assert!(kinds.contains(&RefusalKind::FieldDataMissing));
        assert!(kinds.contains(&RefusalKind::MalformedRecord));
    }

    #[test]
    fn a_process_side_parses_and_a_bad_item_is_refused() {
        let text = "\
name\ttier\tinputs\toutputs\theat_c\theat_kind\thours\tlabour_hours\tpower_kw\tstructure\ttool\tskill\tsource\tprovenance\tnote
riven haft\thands & stone\ttimber*3000\thaft_blank*2400;timber_offcuts*600\t-\t-\t1/3\t1/3\t0\t-\t-\t-\tthe record's worked chain\tD\triving, not sawing
";
        let processes = parse_processes("test", text).expect("a clean declaration");
        assert_eq!(processes.len(), 1);
        assert_eq!(processes[0].inputs.len(), 1);
        assert_eq!(processes[0].outputs.len(), 2);
        assert!(processes[0].requires.is_empty());
        assert_eq!(processes[0].source, "the record's worked chain");

        let bad = text.replace("timber*3000", "timber*3000;oops");
        let defects = parse_processes("test", &bad).expect_err("a malformed item");
        assert_eq!(defects.all()[0].kind(), RefusalKind::MalformedRecord);
    }

    #[test]
    fn tiers_and_vocabulary_parse() {
        let tiers = parse_tiers(
            "test",
            "name\tordinal\tnote\nhands & stone\t0\thands and stone\nbound & composite\t1\tthings joined\n",
        )
        .expect("tiers");
        assert_eq!(tiers.len(), 2);
        assert_eq!(tiers[1].ordinal, 1);

        let vocabulary = parse_vocabulary(
            "test",
            "kind\tname\tnote\nstructure\tkiln\ta fire under clay\n",
        )
        .expect("vocabulary");
        assert_eq!(vocabulary[0].kind, VocabKind::Structure);
        assert_eq!(vocabulary[0].name, "kiln");
    }
}
