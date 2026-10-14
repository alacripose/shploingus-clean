//! **What colour the light is, through the year.**
//!
//! The world is lit by `chromaticity(sky.sun)` — the atmosphere's own transmitted sun,
//! normalised so its strongest channel is 1 — plus the sky dome as an ambient. So "the
//! world looks brown" has two candidate causes that multiply: the **albedo** (a material
//! fact, printed by `--example surface-colours`) and the **illuminant** (an atmosphere
//! fact, printed here). This separates them.
//!
//! The season's only path into the client is `climate::season_phase`, which
//! [`voxel_shapers::climate::season_phase`] derives **from the tick**, so sweeping a year
//! sweeps the season and the time of day separately: every row below is the shipped time
//! of day on a different day of the year.
//!
//! ```text
//! cargo run --offline -p voxel-client --example daylight-hue
//! ```
//!
//! The last column is **arithmetic on the printed numbers, not a render**: it is the
//! albedo multiplied by the beam and the ambient together, at a lambert of 1, which is a
//! surface facing the sun directly. It is here to show how the two factors compound, and
//! it is not a claim about any pixel.

use voxel_appearance::openpbr::LinearRgb;
use voxel_appearance::sky::SkyInputs;
use voxel_client::game::GAME_SEED;
use voxel_client::skylight::daylight_of;
use voxel_shapers::climate::{TICKS_PER_DAY, YEAR_TICKS};
use voxel_shapers::{Config, WorldSpec};

/// The grass surface material's measured albedo, from `--example surface-colours`.
/// Hard-coded here as a *comparison value* rather than derived, so this probe keeps
/// working if the registry changes and the difference is visible.
///
/// **The red channel is 0.1492 since the red window was narrowed to the visible
/// band (it was 0.2086 while the window ran to 0.75 µm and caught the red edge).**
/// Read beside `r/g`: 1.50 then, 1.07 now, and the `r/g` column below shows what
/// the shipped daylight does to that material over a year.
const GRASS: LinearRgb = LinearRgb { r: 0.1492, g: 0.1393, b: 0.0654 };

/// The shipped time of day: `GAME_TICK = TICKS_PER_DAY * 3 / 8`.
const TOD: u64 = TICKS_PER_DAY * 3 / 8;

fn ratio(c: LinearRgb) -> f64 {
    if c.g > 0.0 {
        c.r / c.g
    } else {
        f64::NAN
    }
}

fn main() {
    let spec = WorldSpec::new(GAME_SEED, Config::default());
    println!(
        "grass albedo ({:.4}, {:.4}, {:.4})  r/g = {:.3}   <- the material, before any light",
        GRASS.r,
        GRASS.g,
        GRASS.b,
        ratio(GRASS)
    );
    println!();
    println!(
        "{:>9} {:>7} {:>7} {:>7} | {:>7} {:>7} {:>7} {:>6} | {:>7} {:>7} {:>7} | {:>7}",
        "tick", "day", "alt", "beam", "sun r", "sun g", "sun b", "r/g", "amb r", "amb g",
        "amb b", "lit r/g"
    );

    // A year in twelve steps, each at the shipped time of day.
    let days = YEAR_TICKS / TICKS_PER_DAY;
    let mut warm = 0;
    let mut rows = 0;
    for i in 0..12 {
        let day = days * i / 12;
        let tick = day * TICKS_PER_DAY + TOD;
        let Some(daylight) = daylight_of(&spec, tick, SkyInputs::default(), 0) else {
            println!("{tick:>9} {day:>7}   the atmosphere was refused");
            continue;
        };
        let sun = daylight
            .rig
            .lights
            .first()
            .map(|l| l.colour)
            .unwrap_or(LinearRgb { r: 0.0, g: 0.0, b: 0.0 });
        let ambient = daylight.rig.ambient;
        // Arithmetic on the two printed factors: albedo × (beam + ambient), lambert 1.
        let lit = LinearRgb {
            r: GRASS.r * (sun.r * daylight.sun_intensity + ambient.r),
            g: GRASS.g * (sun.g * daylight.sun_intensity + ambient.g),
            b: GRASS.b * (sun.b * daylight.sun_intensity + ambient.b),
        };
        rows += 1;
        if ratio(sun) > 1.0 {
            warm += 1;
        }
        println!(
            "{tick:>9} {day:>7} {:>7.2} {:>7.4} | {:>7.4} {:>7.4} {:>7.4} {:>6.3} | \
             {:>7.4} {:>7.4} {:>7.4} | {:>7.3}",
            daylight.sun_altitude_deg,
            daylight.sun_intensity,
            sun.r,
            sun.g,
            sun.b,
            ratio(sun),
            ambient.r,
            ambient.g,
            ambient.b,
            ratio(lit),
        );
    }
    println!();
    println!(
        "the sun's chromaticity is red-dominant (r/g > 1) on {warm} of {rows} sampled days"
    );
    println!(
        "**Read the two r/g columns apart.** The albedo's {:.3} is the material and the \
         atmosphere cannot change it; the lit column is what the two factors do together.",
        ratio(GRASS)
    );
}
