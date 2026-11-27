//! Throwaway: the worldgen constants and the elevations the baseline records.

use voxel_shapers::spec::{Config, WorldSpec};
use voxel_shapers::terrain;

fn main() {
    let world = voxel_shapers::worldgen::library();
    println!("package {} {} rows {}", world.package, world.version, world.rows.len());
    for row in &world.rows {
        println!("  {} {} = {} {}", row.id, row.knob, row.value, row.unit);
    }
    let c = Config::default();
    println!(
        "config freq {} relief {} sea {} datum {} base {} ridge x{} at {}",
        c.continent_frequency,
        c.relief_m,
        c.sea_level_m,
        c.base_elevation_m,
        c.base_temperature_c,
        c.ridge_frequency_scale,
        c.ridge_amplitude
    );
    let spec = WorldSpec::new(2468, c);
    for (x, z) in [(0.5, -100.5), (0.5, -1000.5), (6824.5, -27426.5)] {
        println!("elevation({x}, {z}) = {:.4}", terrain::elevation_m(&spec, x, z));
    }
    let mut land = 0;
    let mut n = 0;
    let mut z = -2048.0;
    while z < 2048.0 {
        let mut x = -2048.0;
        while x < 2048.0 {
            n += 1;
            if terrain::elevation_m(&spec, x, z) > c.sea_level_m {
                land += 1;
            }
            x += 64.0;
        }
        z += 64.0;
    }
    println!("land fraction {:.4} of {n} columns", land as f64 / n as f64);
    let doctored = WorldSpec::new(2468, Config { relief_m: c.relief_m * 2.0, ..c });
    for (x, z) in [(0.5, -100.5), (0.5, -1000.5)] {
        println!("doctored elevation({x}, {z}) = {:.4}", terrain::elevation_m(&doctored, x, z));
    }
}
