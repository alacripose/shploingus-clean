//! **The sky the light path produces, as a page.**
//!
//! Run: `cargo run --offline -p voxel-light --example sky-strip`
//!
//! Each row is an hour of the day; each column is a view elevation. The swatch is the
//! chromaticity the solver's own bands give, and the number under it is the radiance's share of
//! the noon sky — so the *shape* is visible at every hour and the magnitude is stated rather than
//! hidden by the display. The last two rows are the same instant beside a fog layer.

use std::fmt::Write as _;

use voxel_light::solver::{
    droplet_extinction_per_m, sky_radiance, twilight_fraction, visible_rgb, Air, SunPath,
};
use voxel_light::spectra::BandTable;

/// The elevations the strip samples, degrees.
const ELEVATIONS: [(f64, &str); 4] = [(1.0, "1°"), (10.0, "10°"), (45.0, "45°"), (85.0, "zenith")];

/// The hours the strip samples, as a fraction of the day from midnight.
const HOURS: [(f64, u32); 8] = [
    (0.0, 0),
    (0.22, 5),
    (0.29, 7),
    (0.40, 10),
    (0.50, 12),
    (0.60, 14),
    (0.71, 17),
    (0.79, 19),
];

fn main() {
    let table = BandTable::computed();
    let clear = Air {
        ground_density_kg_m3: 1.225,
        scale_height_m: 8_500.0,
        droplet_lwc_g_m3: 0.0,
        droplet_layer_depth_m: 0.0,
        cloud_water_path_m: 0.0,
    };
    let fog = Air { droplet_lwc_g_m3: 0.021, droplet_layer_depth_m: 50.0, ..clear };

    let mut page = String::new();
    let _ = write!(
        page,
        "<!doctype html><meta charset=\"utf-8\"><title>the light path's sky</title>\
         <style>body{{background:#111;color:#ddd;font:13px ui-monospace,monospace;margin:24px}}\
         table{{border-collapse:collapse}}td{{padding:2px 6px;text-align:center}}\
         .sw{{width:74px;height:34px;border:1px solid #333}}\
         .h{{color:#888}}caption{{text-align:left;padding-bottom:10px;color:#aaa}}</style>\
         <h1 style=\"font-size:16px\">The sky from the light path — sun's bands through the air</h1>\
         <p class=\"h\">Swatch colour: the solver's own chromaticity. Number: log10 of the band \
         radiance over the noon sky's. Fog row: 0.021 g/m³ in a 50 m layer.</p><table>"
    );
    let _ = write!(page, "<tr><td class=\"h\">solar</td>");
    for (_, label) in ELEVATIONS {
        let _ = write!(page, "<td class=\"h\">{label}</td>");
    }
    let _ = write!(page, "</tr>");

    let mut noon_reference = 0.0f64;
    for (_, hour) in HOURS {
        for (air, tag) in [(&clear, ""), (&fog, " fog")] {
            if !tag.is_empty() && !(hour == 7 || hour == 12) {
                continue;
            }
            let altitude = solar_altitude_deg(hour as f64 / 24.0);
            let _ = write!(
                page,
                "<tr><td class=\"h\">{hour:02}:00{tag}<br>alt {altitude:.1}°</td>"
            );
            for (elevation, _) in ELEVATIONS {
                // **The Sun's own sky, and no Moon.** This page is a strip over hours of a
                // synthetic day rather than over a world's clock, so it carries no orbit and
                // therefore no `MoonPath` — the lunar term needs a body's own position, and
                // inventing one for a page that has no orbit would be the fake the crate's own
                // rule refuses. The Moon's contribution is measured where a real orbit exists:
                // `voxel-client`'s `a_night_has_no_sun_light_and_a_moonlit_ambient` and
                // `examples/measured-light.rs`.
                let bands = sky_radiance(
                    &table,
                    air,
                    &SunPath { altitude_deg: altitude, azimuth_deg: 180.0 },
                    None,
                    elevation,
                    180.0,
                )
                .expect("the sky is computable");
                let total: f64 = bands.iter().sum();
                if tag.is_empty() && hour == 12 && elevation == 85.0 {
                    noon_reference = total;
                }
                let rgb = visible_rgb(&bands);
                let colour = rgb.map(|channel| (channel.clamp(0.0, 1.0) * 255.0) as u8);
                let logarithm = if total > 0.0 { total.log10() } else { f64::NEG_INFINITY };
                let _ = write!(
                    page,
                    "<td><div class=\"sw\" style=\"background:rgb({},{},{})\"></div>\
                     <span class=\"h\">{:.1}</span></td>",
                    colour[0], colour[1], colour[2], logarithm
                );
            }
            let _ = write!(page, "</tr>");
        }
    }
    let _ = write!(page, "</table>");

    // The twilight curve the reference states, so the page carries the fall into night rather than
    // stopping at the last row the table is sampled on.
    let _ = write!(
        page,
        "<h2 style=\"font-size:14px\">The fall into night — the reference's own curve</h2>\
         <table><tr><td class=\"h\">solar altitude</td><td class=\"h\">−4°</td><td class=\"h\">−6°\
         </td><td class=\"h\">−12°</td><td class=\"h\">−18°</td></tr><tr>\
         <td class=\"h\">horizon light</td>"
    );
    for altitude in [-4.0, -6.0, -12.0, -18.0] {
        let _ = write!(
            page,
            "<td>{:.2} lx</td>",
            twilight_fraction(altitude) * 400.0
        );
    }
    let _ = write!(page, "</tr></table>");

    // And the fog's own extinction, so the visibility the droplets imply is on the page too.
    let _ = write!(
        page,
        "<p class=\"h\">droplets at 0.021 g/m³ ⇒ extinction {:.4} /m ⇒ visibility {:.2} km \
         (Koschmieder) — noon reference radiance {:.4}</p>",
        droplet_extinction_per_m(0.021),
        3.912 / (droplet_extinction_per_m(0.021) * 1_000.0),
        noon_reference
    );

    let path = std::path::Path::new("logs/sky-strip.html");
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(path, page).expect("the page is written");
    println!("wrote {}", path.display());
}

/// The sun's altitude for a fraction of the day, at this world's own latitude and declination.
fn solar_altitude_deg(day_fraction: f64) -> f64 {
    let latitude: f64 = 45.0;
    let hour_angle = (day_fraction - 0.5) * 360.0;
    let declination: f64 = 0.0;
    let up = latitude.to_radians().sin() * declination.to_radians().sin()
        + latitude.to_radians().cos()
            * declination.to_radians().cos()
            * hour_angle.to_radians().cos();
    up.clamp(-1.0, 1.0).asin().to_degrees()
}
