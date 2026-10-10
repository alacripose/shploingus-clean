//! **The twelve bands, printed.**
//!
//! ```text
//! cargo run --offline -p voxel-light --example bands
//! ```
//!
//! The sun's power per band is Planck at 5778 K, integrated over the band on a log grid,
//! diluted by the solar disc's solid angle, and scaled so the twelve sum to the solar
//! constant. The optical columns are the declared clear-sky model at each band's
//! geometric centre.

use voxel_light::{BandTable, SCALE_HEIGHT_M, SOLAR_CONSTANT, SUN_TEMPERATURE_K};

fn main() {
    let table = BandTable::computed();
    println!(
        "the sun: Planck at {SUN_TEMPERATURE_K:.0} K, diluted and scaled to \
         {SOLAR_CONSTANT:.0} W/m² over {} bands",
        table.bands().len()
    );
    println!(
        "the sky: a clear column, scale height {SCALE_HEIGHT_M:.0} m, Earth-derived \
         coefficients declared rather than measured"
    );
    println!();
    println!(
        "{:>14} {:>11} {:>9} {:>7} | {:>9} {:>9} {:>9} | {:>9} {:>7}",
        "band (nm)", "centre", "sun W/m²", "% sun", "ext column", "scat column", "abs column",
        "moon refl", "carriage"
    );

    for band in table.bands() {
        let share = 100.0 * band.sun_w_per_m2 / SOLAR_CONSTANT;
        let carriage = if band.transport == voxel_light::Transport::Links {
            "links"
        } else if band.transport == voxel_light::Transport::BeamOnly {
            "beam"
        } else {
            "beam+sky"
        };
        println!(
            "{:>6.3e}–{:<6.3e} {:>11.1} {:>9.2} {:>6.2}% | {:>9.4} {:>9.4} {:>9.4} | \
             {:>9.4} {carriage:>7}",
            band.lo_nm,
            band.hi_nm,
            band.centre_nm(),
            band.sun_w_per_m2,
            share,
            band.column_extinction(),
            band.column_scatter(),
            band.column_absorption(),
            band.moon_reflectance,
        );
    }

    println!();
    let visible: f64 = table
        .bands()
        .iter()
        .filter(|band| band.lo_nm >= 400.0 && band.hi_nm <= 700.0)
        .map(|band| band.sun_w_per_m2)
        .sum();
    println!(
        "the visible triple (400–700 nm) carries {:.1}% of the constant, and the widest \
         band carries more than any visible one — the peak is in the green and the bulk \
         is in the near infrared",
        100.0 * visible / SOLAR_CONSTANT
    );
    println!(
        "sum: {:.6} W/m² against the declared {SOLAR_CONSTANT}",
        table.total_sun_w_per_m2()
    );
}
