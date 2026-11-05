//! **THROWAWAY PROBE.** Is the biome field over a map sheet a smooth landscape or
//! per-cell noise, and which of the two Holdridge inputs carries it?
//!
//! Reads the world exactly as the map window's `biome_at` does at the plan's own
//! declared cells (32 m), over the three camera centres the screenshots named.

use voxel_shapers::ecology::{self, Biome};
use voxel_shapers::spec::{Config, WorldSpec};
use voxel_shapers::terrain;

const SEED: u64 = 2468;
const CELL_M: f64 = 32.0;
const COLUMNS: usize = 28;
const ROWS: usize = 32;

fn letter(biome: Biome) -> char {
    match biome {
        Biome::PolarDesert => 'P',
        Biome::Tundra => 'T',
        Biome::BorealForest => 'B',
        Biome::CoolTemperateForest => 'C',
        Biome::TemperateForest => 'F',
        Biome::Grassland => 'G',
        Biome::Desert => 'D',
        Biome::SubtropicalScrub => 'S',
        Biome::TropicalDryForest => 'X',
        Biome::TropicalRainforest => 'R',
        Biome::Alpine => 'A',
    }
}

fn sheet(spec: &WorldSpec, label: &str, centre: (f64, f64)) {
    println!("\n=== {label}: centre {centre:?}, {COLUMNS}x{ROWS} cells of {CELL_M} m ===");
    let mut kinds: Vec<Option<Biome>> = Vec::new();
    let mut biot: Vec<f64> = Vec::new();
    let mut precip: Vec<f64> = Vec::new();
    let mut ups: Vec<f64> = Vec::new();
    for row in 0..ROWS {
        for column in 0..COLUMNS {
            let x = centre.0 + (column as f64 + 0.5 - COLUMNS as f64 / 2.0) * CELL_M;
            let z = centre.1 + (row as f64 + 0.5 - ROWS as f64 / 2.0) * CELL_M;
            let elevation = terrain::elevation_m(spec, x, z);
            let (bt, p) = ecology::holdridge_inputs(spec, x, z, elevation, 0);
            let above = (elevation - spec.config.sea_level_m).max(0.0);
            kinds.push(if above <= 0.0 {
                None
            } else {
                Some(ecology::biome_at(spec, elevation, bt, p))
            });
            biot.push(bt);
            precip.push(p);
            ups.push(elevation);
        }
    }
    for row in 0..ROWS {
        let line: String = (0..COLUMNS)
            .map(|column| match kinds[row * COLUMNS + column] {
                None => '~',
                Some(biome) => letter(biome),
            })
            .collect();
        println!("{line}");
    }
    let mut flips = 0usize;
    let mut pairs = 0usize;
    for row in 0..ROWS {
        for column in 0..COLUMNS {
            if column + 1 < COLUMNS {
                pairs += 1;
                if kinds[row * COLUMNS + column] != kinds[row * COLUMNS + column + 1] {
                    flips += 1;
                }
            }
            if row + 1 < ROWS {
                pairs += 1;
                if kinds[row * COLUMNS + column] != kinds[(row + 1) * COLUMNS + column] {
                    flips += 1;
                }
            }
        }
    }
    let span = |values: &[f64]| {
        let lo = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let hi = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        (lo, hi)
    };
    let (blo, bhi) = span(&biot);
    let (plo, phi) = span(&precip);
    let (elo, ehi) = span(&ups);
    println!(
        "neighbour cells differing in biome: {flips}/{pairs} ({:.0}%)",
        100.0 * flips as f64 / pairs as f64
    );
    println!("biotemperature {blo:.1}..{bhi:.1} C  precip {plo:.0}..{phi:.0} mm  elevation {elo:.1}..{ehi:.1} m");

    // The upslope sign, which is the precipitation's cliff: does it flip between
    // neighbouring cells?
    let mut sign_flips = 0usize;
    let mut sign_pairs = 0usize;
    let mut signs = Vec::new();
    for row in 0..ROWS {
        for column in 0..COLUMNS {
            let x = centre.0 + (column as f64 + 0.5 - COLUMNS as f64 / 2.0) * CELL_M;
            let z = centre.1 + (row as f64 + 0.5 - ROWS as f64 / 2.0) * CELL_M;
            signs.push(voxel_shapers::weather::sample(spec, x, z, 0).upslope);
        }
    }
    for row in 0..ROWS {
        for column in 0..COLUMNS {
            if column + 1 < COLUMNS {
                sign_pairs += 1;
                if (signs[row * COLUMNS + column] > 0.0)
                    != (signs[row * COLUMNS + column + 1] > 0.0)
                {
                    sign_flips += 1;
                }
            }
            if row + 1 < ROWS {
                sign_pairs += 1;
                if (signs[row * COLUMNS + column] > 0.0)
                    != (signs[(row + 1) * COLUMNS + column] > 0.0)
                {
                    sign_flips += 1;
                }
            }
        }
    }
    println!(
        "neighbour cells differing in upslope sign: {sign_flips}/{sign_pairs} ({:.0}%)",
        100.0 * sign_flips as f64 / sign_pairs as f64
    );
}

/// **The same sheet, with the upslope read at a landform's own step**: the mosaic the
/// map draws, if `gradient`'s baseline were a real-world distance through `scale::voxels`
/// rather than a 2 engine metre point sample.
fn sheet_at_step(spec: &WorldSpec, label: &str, centre: (f64, f64), real_step_m: f64) {
    let h = voxel_shapers::scale::voxels(real_step_m);
    let mut kinds: Vec<Option<Biome>> = Vec::new();
    for row in 0..ROWS {
        for column in 0..COLUMNS {
            let x = centre.0 + (column as f64 + 0.5 - COLUMNS as f64 / 2.0) * CELL_M;
            let z = centre.1 + (row as f64 + 0.5 - ROWS as f64 / 2.0) * CELL_M;
            let elevation = terrain::elevation_m(spec, x, z);
            let (bt, _) = ecology::holdridge_inputs(spec, x, z, elevation, 0);
            let m = voxel_shapers::climate::moisture(spec, x, z);
            let bearing = voxel_shapers::weather::wind_bearing_rad(spec, x, z);
            let wind = (bearing.sin(), bearing.cos());
            let dx = (terrain::elevation_m(spec, x + h, z) - terrain::elevation_m(spec, x - h, z))
                / (2.0 * h);
            let dz = (terrain::elevation_m(spec, x, z + h) - terrain::elevation_m(spec, x, z - h))
                / (2.0 * h);
            let upslope = dx * wind.0 + dz * wind.1;
            let precip = if upslope <= 0.0 {
                320.0 * m * 0.35
            } else {
                320.0 * m + 26_000.0 * m * upslope
            };
            let above = (elevation - spec.config.sea_level_m).max(0.0);
            kinds.push(if above <= 0.0 {
                None
            } else {
                Some(ecology::biome_at(spec, elevation, bt, precip.max(0.0)))
            });
        }
    }
    let mut lines = String::new();
    for row in 0..ROWS {
        for column in 0..COLUMNS {
            lines.push(match kinds[row * COLUMNS + column] {
                None => '~',
                Some(biome) => letter(biome),
            });
        }
        lines.push('\n');
    }
    print!("{lines}");
    let mut flips = 0usize;
    let mut pairs = 0usize;
    for row in 0..ROWS {
        for column in 0..COLUMNS {
            if column + 1 < COLUMNS {
                pairs += 1;
                if kinds[row * COLUMNS + column] != kinds[row * COLUMNS + column + 1] {
                    flips += 1;
                }
            }
            if row + 1 < ROWS {
                pairs += 1;
                if kinds[row * COLUMNS + column] != kinds[(row + 1) * COLUMNS + column] {
                    flips += 1;
                }
            }
        }
    }
    let mut seen: Vec<char> = kinds
        .iter()
        .flatten()
        .map(|biome| letter(*biome))
        .collect();
    seen.sort_unstable();
    seen.dedup();
    println!(
        "{label}: step {h:.1} engine m ({real_step_m:.0} real m) -> {} biomes {seen:?}, neighbour diffs {flips}/{pairs} ({:.0}%)",
        seen.len(),
        100.0 * flips as f64 / pairs as f64
    );
}

/// **The record's catalog rows, generated from the mounted package** so the artifact and the
/// catalog cannot be transcribed apart. The text it writes is the record's `rows` block.
fn dump_library() {
    let catalog = voxel_shapers::biome_library::library();
    let end = |value: f64| {
        if value.is_finite() { format!("{value:.1}") } else { "\"open\"".to_string() }
    };
    let mut rows = Vec::new();
    for row in &catalog.envelopes {
        rows.push(format!(
            "    {{ \"name\": \"{}\", \"role\": \"envelope\", \"system\": \"{}\", \"biotemperature_c\": [{}, {}], \"precipitation_mm\": [{:.0}, {}], \"biome\": \"{}\", \"must\": \"equal\", \"note\": \"{}\" }}",
            row.name,
            row.system,
            end(row.bands.biotemperature_c[0]),
            end(row.bands.biotemperature_c[1]),
            row.bands.precipitation_mm[0],
            end(row.bands.precipitation_mm[1]),
            row.biome.name(),
            row.source
        ));
    }
    for row in &catalog.types {
        rows.push(format!(
            "    {{ \"name\": \"{}\", \"role\": \"type\", \"envelope\": \"{}\", \"hydrology\": \"{:?}\", \"position\": \"{:?}\", \"structure\": \"{:?}\", \"must\": \"equal\", \"note\": \"{}\" }}",
            row.name, row.envelope, row.hydrology, row.position, row.structure, row.source
        ));
    }
    for row in &catalog.overlays {
        rows.push(format!(
            "    {{ \"name\": \"{}\", \"role\": \"overlay\", \"hydrology\": \"{:?}\", \"position\": \"{:?}\", \"must\": \"equal\", \"note\": \"{}\" }}",
            row.name, row.hydrology, row.position, row.source
        ));
    }
    for (name, reason) in &catalog.deferred {
        rows.push(format!(
            "    {{ \"name\": \"{}\", \"role\": \"deferred\", \"waits_on\": \"{}\", \"must\": \"equal\" }}",
            name, reason
        ));
    }
    for row in rows {
        println!("{row},");
    }
}

fn main() {
    // **The folder's own package, mounted** — the record this probe writes is the file the game
    // mounts, not the crate's embedded copy (the two are the same bytes, and the test says so).
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../display-packages/biomes/package.shspec");
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|why| panic!("{} cannot be read ({why})", path.display()));
    voxel_shapers::biome_library::mount(&source)
        .unwrap_or_else(|why| panic!("{} does not mount: {why}", path.display()));
    if std::env::args().any(|arg| arg == "library") {
        dump_library();
        return;
    }
    let spec = WorldSpec::new(SEED, Config::default());
    sheet(&spec, "mountain view", (6824.0, -27426.0));
    sheet(&spec, "forest view", (1416.0, -46452.0));
    sheet(&spec, "tundra view", (2446.0, -48808.0));
    println!("\n=== the same sheets with a landform-step upslope ===");
    for real_step_m in [133.0, 267.0, 533.0] {
        for (label, centre) in [
            ("mountain", (6824.0, -27426.0)),
            ("forest", (1416.0, -46452.0)),
            ("tundra", (2446.0, -48808.0)),
        ] {
            sheet_at_step(&spec, label, centre, real_step_m);
        }
    }
}
