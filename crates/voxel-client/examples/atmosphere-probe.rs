//! **The atmosphere baseline's rows, measured and printed** — the probe
//! `docs/baselines/atmosphere.json` names in its `recipe`.
//!
//! Every row is read from the code that draws or lights the sky, never typed here: the cloud
//! ladder is [`voxel_client::cloud_cubes`]'s, the transmittance is the shipped model's own
//! columns (`voxel_shapers::clouds`), and the fog extinction is `voxel_light`'s visibility fit.
//! `crates/voxel-client/src/cloud_cubes.rs`'s `the_atmosphere_baseline_is_this_ladder` reads the
//! same file back and fails if a row and the law disagree, so this is the reproduction and that
//! is the binding.
//!
//! Run: `cargo run -p voxel-client --example atmosphere-probe`.

use voxel_client::cloud_cubes::{self, rung_cell_m, rung_for, CLOUD_FINEST_CELL_M, RUNGS};
use voxel_client::cloud_field::{cloud_transmittance, CLOUD_EXTINCTION_PER_M};
use voxel_shapers::{clouds, Config, WorldSpec};

fn main() {
    // ---- the ladder: the surface's own 32 rungs, cells named at its ends and its quarters ----
    println!("rungs\t{RUNGS}");
    for tier in [0u32, 2, 5, 7, 8, 9] {
        println!("rung {tier} cell m\t{:.3}", rung_cell_m(tier));
    }
    println!("cloud finest cell m\t{CLOUD_FINEST_CELL_M:.3}");
    println!(
        "cloud domain m\t{:.1}",
        cloud_cubes::CLOUD_DOMAIN_M
    );

    // ---- the rung the eye resolves, at the game's own pixel angle ----
    let angle = voxel_client::pixel_angle_rad(70.0f64.to_radians(), 720);
    let angle = f64::from(angle);
    println!("pixel angle mrad\t{:.3}", angle * 1_000.0);
    for distance in [500.0f64, 1_000.0, 2_000.0, 4_000.0] {
        let tier = rung_for(angle, distance);
        println!(
            "rung at {distance:.0} m\t{tier}\t{:.3} m",
            rung_cell_m(tier)
        );
    }

    // ---- the veil: the extinction the cubes carry and the model's own water paths ----
    println!("cloud extinction per m\t{CLOUD_EXTINCTION_PER_M:.4}");
    let spec = WorldSpec::new(2468, Config::default());
    let tick = voxel_shapers::climate::TICKS_PER_DAY / 2;
    let mut dimmed: Vec<f64> = Vec::new();
    for x in (-1_024..1_024).step_by(61) {
        for z in (-1_024..1_024).step_by(67) {
            let path = clouds::column_water_path_m(&spec, x as f64, z as f64, tick);
            if path > 0.0 {
                dimmed.push(cloud_transmittance(path));
            }
        }
    }
    dimmed.sort_by(f64::total_cmp);
    println!("cloudy columns\t{}", dimmed.len());
    if !dimmed.is_empty() {
        println!("median transmittance\t{:.3}", dimmed[dimmed.len() / 2]);
        println!("deepest transmittance\t{:.3}", dimmed[0]);
    }

    // ---- fog: the guide's visibility fit, which is the extinction the sky uses for droplets ----
    let lwc = voxel_shapers::fog::FOG_LWC_G_M3;
    let extinction = voxel_light::droplet_extinction_per_m(lwc);
    println!(
        "fog extinction per m at {lwc} g/m3\t{extinction:.5}\tvisibility {:.0} m",
        3_912.0 / (extinction * 1_000.0)
    );
}
