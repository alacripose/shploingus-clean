//! **The world's own constants, as a package** — the seven numbers the terrain and climate laws
//! read, authored in `display-packages/worldgen/package.shspec` rather than in Rust.
//!
//! # Source
//!
//! * `docs/earth_biomes_subbiomes_continentalness_mountains.pdf` — §1 (the climate engine: heat,
//!   water, seasonality), §2 (continentalness: maritime margins to continental interior) and §3
//!   (mountains: uplift, rain shadows, ecological compression).
//! * `docs/earth_biomes_subbiomes_terrain.pdf` — §1 (the first filter: energy and water), §3
//!   (terrain is a climate modifier: elevation, slope, aspect, relief) and §7 (mountains:
//!   compressed biomes and high topographic diversity).
//!
//! **What the atlas gives and what this package gives.** The atlases say what each number
//! *means* — continentalness is the strength of land-interior influence against the ocean's
//! buffering, relief is the amplitude that changes rainfall, exposure, snow and erosion, and a
//! range reorganizes climate over a short horizontal distance. Neither quotes an engine metre, and
//! this world is modelled eight times smaller than the real one ([`crate::scale`]), so the values
//! are the world's own tuning at that scale and each row's `source` says which section the meaning
//! was read from. That is the same division the biomes catalog makes: the atlas names the row, the
//! world chooses the number.
//!
//! # The package
//!
//! [`SHIPPED`] embeds the file, so a test or a tool with no workspace still builds a world;
//! [`mount`] installs a copy read at runtime, and the game mounts the folder at boot, before the
//! first [`crate::spec::Config::default`]. **The first mount wins** for the process's life: a
//! package that changed under a world already generated would make two columns disagree about
//! how deep the sea is.
//!
//! # The rows
//!
//! `role=constant` rows carry `knob`, `value`, `unit` and `source`. The prose rows of the same
//! package (`OPT`, `ORO`, `WAV`) are the atmosphere's and are skipped here. [`Constants`] holds
//! the typed values and the rows, in the package's order, and
//! [`crate::spec::Config::default`] maps them into the configuration every `WorldSpec` carries —
//! so **the world spec reads this package** instead of a table of literals.

use std::sync::OnceLock;

/// The recipe header every display package starts with.
const HEADER: &str = "shploingus.recipe/3";

/// The shipped package: the file's own bytes, embedded so a world exists without a workspace.
const SHIPPED: &str = include_str!("../../../display-packages/worldgen/package.shspec");

/// **Every knob the terrain and climate laws read.** A package missing one, or declaring one
/// twice, is refused rather than filled in: a world with a silently substituted number is a world
/// nobody can tune.
pub const KNOBS: [&str; 7] = [
    "continental_wavelength_m",
    "relief_m",
    "sea_level_m",
    "datum_m",
    "base_temperature_c",
    "ridge_frequency_scale",
    "ridge_amplitude",
];

/// One `role=constant` row, as the package wrote it.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub id: String,
    pub knob: String,
    pub value: f64,
    pub unit: String,
    pub source: String,
}

/// **The world's constants, typed**: the package's rows, in the package's order, and the seven
/// numbers a `Config` is built from.
#[derive(Debug)]
pub struct Constants {
    pub package: String,
    pub version: String,
    /// One continental cycle, engine metres. The frequency is its reciprocal.
    pub continental_wavelength_m: f64,
    /// Peak-to-trough relief the elevation law can reach, engine metres.
    pub relief_m: f64,
    /// Height of the sea surface above the datum.
    pub sea_level_m: f64,
    /// The datum the whole surface is measured from.
    pub datum_m: f64,
    /// Equatorial sea-level annual mean temperature, °C.
    pub base_temperature_c: f64,
    /// How many continental frequencies the ridged mountain term runs at.
    pub ridge_frequency_scale: f64,
    /// How much of the relief the ridged term raises where plates converge.
    pub ridge_amplitude: f64,
    pub rows: Vec<Row>,
}

static CONSTANTS: OnceLock<Constants> = OnceLock::new();

/// **The constants in force**: the package [`mount`] installed, or the shipped package's rows.
pub fn library() -> &'static Constants {
    CONSTANTS.get_or_init(|| {
        Constants::read(SHIPPED)
            .unwrap_or_else(|why| panic!("the embedded worldgen package does not read: {why}"))
    })
}

/// **Mount the world's constants read at runtime.** The first mount wins for the process's life:
/// the configuration every world is built from is read once, and a second package is refused
/// rather than swapped in under a world that already has columns.
pub fn mount(source: &str) -> Result<&'static Constants, String> {
    if CONSTANTS.get().is_some() {
        return Err("a worldgen package is already mounted; the first mount is the world's"
            .to_string());
    }
    let parsed = Constants::read(source)?;
    let _ = CONSTANTS.set(parsed);
    Ok(CONSTANTS.get().expect("set immediately above"))
}

impl Constants {
    /// Read a package's `role=constant` rows. Every error names the line and the field.
    fn read(source: &str) -> Result<Constants, String> {
        let mut lines = source.lines().enumerate();
        let header = lines
            .next()
            .map(|(_, line)| line)
            .ok_or("the package is empty")?;
        if header != HEADER {
            return Err(format!("the first line is {header:?}, not {HEADER:?}"));
        }
        let mut package = None;
        let mut version = None;
        let mut rows: Vec<Row> = Vec::new();
        for (index, line) in lines {
            let number = index + 1;
            if line.trim().is_empty() || line.trim_start().starts_with('#') {
                continue;
            }
            if let Some(rest) = line.strip_prefix("package\t") {
                let fields: Vec<&str> = rest.split('\t').collect();
                match fields.as_slice() {
                    [name, stated] if package.is_none() => {
                        package = Some((*name).to_string());
                        version = Some((*stated).to_string());
                    }
                    _ => return Err(format!("line {number}: the package line is malformed or twice")),
                }
                continue;
            }
            let fields: Vec<&str> = line.splitn(3, '\t').collect();
            let ["row", id, text] = fields.as_slice() else {
                return Err(format!(
                    "line {number}: {line:?} is not a `package` or `row` line"
                ));
            };
            // The atmosphere's rows are the same package's prose, not constants.
            if !text.starts_with("role=constant") {
                continue;
            }
            let row = constant_row(id, text).map_err(|why| format!("line {number}: {why}"))?;
            if rows.iter().any(|held| held.knob == row.knob) {
                return Err(format!(
                    "line {number}: the knob {} is declared twice",
                    row.knob
                ));
            }
            rows.push(row);
        }
        let package = package.ok_or("the package line is missing")?;
        let version = version.ok_or("the package line is missing")?;
        for knob in KNOBS {
            if !rows.iter().any(|row| row.knob == knob) {
                return Err(format!("the package declares no {knob}"));
            }
        }
        if rows.len() != KNOBS.len() {
            return Err(format!(
                "the package declares {} constants against the {KNOBS:?} the laws read",
                rows.len()
            ));
        }
        let value = |knob: &str| {
            rows.iter()
                .find(|row| row.knob == knob)
                .map(|row| row.value)
                .expect("every knob is present, checked above")
        };
        Ok(Constants {
            package,
            version,
            continental_wavelength_m: value("continental_wavelength_m"),
            relief_m: value("relief_m"),
            sea_level_m: value("sea_level_m"),
            datum_m: value("datum_m"),
            base_temperature_c: value("base_temperature_c"),
            ridge_frequency_scale: value("ridge_frequency_scale"),
            ridge_amplitude: value("ridge_amplitude"),
            rows,
        })
    }
}

/// Parse one `role=constant` row's `key=value` fields.
fn constant_row(id: &str, text: &str) -> Result<Row, String> {
    let mut knob = None;
    let mut value = None;
    let mut unit = None;
    let mut source = None;
    for part in text.split(" | ") {
        let Some((key, field)) = part.split_once('=') else {
            return Err(format!("{id}: {part:?} is not `key=value`"));
        };
        match key {
            "role" => {}
            "knob" => knob = Some(field.to_string()),
            "value" => {
                let parsed: f64 = field
                    .parse()
                    .map_err(|_| format!("{id}: value {field:?} is not a number"))?;
                if !parsed.is_finite() {
                    return Err(format!("{id}: value {field:?} is not finite"));
                }
                value = Some(parsed);
            }
            "unit" => unit = Some(field.to_string()),
            "source" => source = Some(field.to_string()),
            other => return Err(format!("{id}: unknown field {other:?}")),
        }
    }
    Ok(Row {
        id: id.to_string(),
        knob: knob.ok_or_else(|| format!("{id}: no knob"))?,
        value: value.ok_or_else(|| format!("{id}: no value"))?,
        unit: unit.ok_or_else(|| format!("{id}: no unit"))?,
        source: source.ok_or_else(|| format!("{id}: no source"))?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Config;
    use crate::terrain;

    #[test]
    fn the_package_is_the_shipped_source() {
        // The embedded copy and the folder are one declaration, so a build cannot read a package
        // that is not on disk.
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../display-packages/worldgen/package.shspec"
        );
        let on_disk = std::fs::read_to_string(path).expect("display-packages/worldgen/package.shspec");
        assert_eq!(
            SHIPPED, on_disk,
            "the embedded worldgen package and {path} are not the same bytes"
        );
    }

    #[test]
    fn every_knob_is_declared_once_and_every_row_names_its_section() {
        let world = library();
        assert_eq!(world.rows.len(), KNOBS.len(), "a row is missing or extra");
        for knob in KNOBS {
            let row = world
                .rows
                .iter()
                .find(|row| row.knob == knob)
                .unwrap_or_else(|| panic!("{knob} is not declared"));
            assert!(
                row.source.contains("§"),
                "{} does not name the atlas section it was read from: {:?}",
                row.knob,
                row.source
            );
            assert!(!row.unit.is_empty(), "{} has no unit", row.knob);
            assert!(row.value.is_finite(), "{} is not finite", row.knob);
        }
    }

    #[test]
    fn the_default_config_is_the_packages_rows() {
        let world = library();
        let c = Config::default();
        assert_eq!(c.sea_level_m, world.sea_level_m);
        assert_eq!(c.base_elevation_m, world.datum_m);
        assert_eq!(c.relief_m, world.relief_m);
        assert_eq!(c.continent_frequency, 1.0 / world.continental_wavelength_m);
        assert_eq!(c.base_temperature_c, world.base_temperature_c);
        assert_eq!(c.ridge_frequency_scale, world.ridge_frequency_scale);
        assert_eq!(c.ridge_amplitude, world.ridge_amplitude);
    }

    #[test]
    fn the_record_names_the_packages_rows() {
        // **The baselines record is a binding, not a copy**: `docs/baselines/worldgen.json` carries
        // every constant row with its knob, value and unit, in the package's order, and this fails
        // the moment either side moves without the other.
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/baselines/worldgen.json");
        let record = std::fs::read_to_string(path).expect("docs/baselines/worldgen.json");
        let field = |line: &str, key: &str| -> Option<String> {
            let start = line.find(&format!("\"{key}\": \""))? + key.len() + 5;
            let rest = &line[start..];
            Some(rest[..rest.find('"')?].to_string())
        };
        let number = |line: &str, key: &str| -> Option<f64> {
            let start = line.find(&format!("\"{key}\": "))? + key.len() + 4;
            let rest = &line[start..];
            let end = rest.find([',', '}'])?;
            rest[..end].trim().parse().ok()
        };
        let mut recorded: Vec<(String, f64, String)> = Vec::new();
        for line in record.lines() {
            let Some(knob) = field(line, "knob") else { continue };
            let value = number(line, "value")
                .unwrap_or_else(|| panic!("the record's {knob} row has no numeric value"));
            let unit = field(line, "unit").unwrap_or_default();
            recorded.push((knob, value, unit));
        }
        let declared: Vec<(String, f64, String)> = library()
            .rows
            .iter()
            .map(|row| (row.knob.clone(), row.value, row.unit.clone()))
            .collect();
        assert_eq!(
            recorded, declared,
            "docs/baselines/worldgen.json does not carry the package's constants, in order"
        );
    }

    #[test]
    fn a_doctored_row_moves_the_ground() {
        // The experiment the mount exists for: the same code, one number changed on disk.
        let shipped = Config::default();
        let doctored = Config { relief_m: shipped.relief_m * 2.0, ..shipped };
        let a = terrain::elevation_m(&crate::spec::WorldSpec::new(2468, shipped), 0.5, -100.5);
        let b = terrain::elevation_m(&crate::spec::WorldSpec::new(2468, doctored), 0.5, -100.5);
        assert_ne!(a, b, "doubling relief left the ground where it was");
    }
}
