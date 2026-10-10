//! **PROTOTYPE — THROWAWAY.** Evidence for the ECO-0001 follow-up.
//!
//! Question: with the biome classified the way `column.rs` now does it — on
//! **biotemperature** and **mean annual precipitation**, sampled from the ticked
//! shapers — what biomes does the default world actually contain?
//!
//! `column::tests::the_pipeline_covers_biomes_and_lithologies_across_a_world` went
//! red after ECO-0001: its 12 km diagonal saw `{Tundra, BorealForest}`. This
//! sweeps the world as a grid and re-derives the warmest columns across ticks —
//! **sampling the shapers, not re-deriving them analytically** — so the test's
//! expectation is decided by what the world says, not by what a formula suggests.

use std::collections::BTreeMap;
use voxel_shapers::climate::YEAR_TICKS;
use voxel_shapers::column::derive;
use voxel_shapers::ecology::{self, Biome};
use voxel_shapers::spec::{Config, WorldSpec};

/// What one pass over positions collected.
struct Census {
    biomes: BTreeMap<Biome, usize>,
    min_biotemperature: f64,
    max_biotemperature: f64,
    min_map_mm: f64,
    max_map_mm: f64,
    /// `(biotemperature °C, MAP mm, x, z, biome)`, for the coldest..warmest table.
    columns: Vec<(f64, f64, f64, f64, Biome)>,
}

impl Census {
    fn new() -> Self {
        Census {
            biomes: BTreeMap::new(),
            min_biotemperature: f64::INFINITY,
            max_biotemperature: f64::NEG_INFINITY,
            min_map_mm: f64::INFINITY,
            max_map_mm: f64::NEG_INFINITY,
            columns: Vec::new(),
        }
    }

    fn push(&mut self, spec: &WorldSpec, x: f64, z: f64, tick: u64, hydro: usize) {
        let c = derive(spec, x, z, tick, hydro);
        let (biotemperature_c, mean_annual_precipitation_mm) =
            ecology::holdridge_inputs(spec, x, z, c.terrain.elevation_m, tick);
        *self.biomes.entry(c.biome).or_insert(0) += 1;
        self.min_biotemperature = self.min_biotemperature.min(biotemperature_c);
        self.max_biotemperature = self.max_biotemperature.max(biotemperature_c);
        self.min_map_mm = self.min_map_mm.min(mean_annual_precipitation_mm);
        self.max_map_mm = self.max_map_mm.max(mean_annual_precipitation_mm);
        self.columns.push((biotemperature_c, mean_annual_precipitation_mm, x, z, c.biome));
    }

    fn report(&mut self, label: &str, warmest: usize) {
        self.columns.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        println!("{label}");
        for (biome, n) in &self.biomes {
            println!("    {:<22} {:>6}", biome.name(), n);
        }
        println!(
            "    biotemperature {:.3} .. {:.3} °C   MAP {:.1} .. {:.1} mm",
            self.min_biotemperature,
            self.max_biotemperature,
            self.min_map_mm,
            self.max_map_mm
        );
        if warmest > 0 {
            println!("    warmest columns:");
            for (bt, map, x, z, biome) in self.columns.iter().take(warmest) {
                println!(
                    "        ({x:>7.0}, {z:>7.0})  biotemperature {bt:>6.3} °C  MAP {map:>8.1} mm  {}",
                    biome.name()
                );
            }
        }
        println!();
    }
}

fn main() {
    let spec = WorldSpec::new(2468, Config::default());
    let hydro = 2;

    // The failing test's probe, exactly.
    let mut line = Census::new();
    let mut x = -6000.0;
    while x < 6000.0 {
        line.push(&spec, x, x * 0.7, 0, hydro);
        x += 300.0;
    }
    line.report(
        "the coverage test's diagonal, x in [-6000, 6000) step 300, z = 0.7x, tick 0:",
        8,
    );

    // The world as a grid.
    let mut grid = Census::new();
    let mut x = -6000.0;
    while x <= 6000.0 {
        let mut z = -6000.0;
        while z <= 6000.0 {
            grid.push(&spec, x, z, 0, hydro);
            z += 300.0;
        }
        x += 300.0;
    }
    grid.report("the grid, x and z in [-6000, 6000] step 300 (41 x 41), tick 0:", 8);

    // A finer sweep. The climate belt is sampled at `continent_frequency` and its
    // own field has structure near 1/0.0025 = 400 m, so a 300 m lattice is at
    // Nyquist for it and can miss the warmest cores. This asks whether the coarse
    // grid's set is the world's or only the sampled lattice's.
    let mut fine = Census::new();
    let mut x = -6000.0;
    while x <= 6000.0 {
        let mut z = -6000.0;
        while z <= 6000.0 {
            fine.push(&spec, x, z, 0, hydro);
            z += 100.0;
        }
        x += 100.0;
    }
    fine.report("the fine grid, x and z in [-6000, 6000] step 100 (121 x 121), tick 0:", 8);

    // The warmest columns re-derived across a year and a later year: the aggregate
    // is sampled from the ticked shapers, so this shows the biome stays put when the
    // tick moves.
    println!("the warmest grid columns re-derived across a year, and a later year:");
    let mut held = grid.columns.clone();
    held.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    for (bt, map, x, z, _) in held.iter().take(5) {
        let mut biomes = Vec::new();
        for tick in [0u64, YEAR_TICKS / 4, YEAR_TICKS / 2, YEAR_TICKS * 3 / 4, YEAR_TICKS * 4 + 7] {
            let c = derive(&spec, *x, *z, tick, hydro);
            biomes.push(c.biome.name());
        }
        println!(
            "    ({x:>7.0}, {z:>7.0})  biotemperature {bt:>6.3} °C  MAP {map:>8.1} mm  biomes: {}",
            biomes.join(", ")
        );
    }
}
