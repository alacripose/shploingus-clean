//! **Temporary measurement** — what the ground is drawn at, tick by tick, against the noon.
//!
//! One exposure for every frame (`EXPOSURE_GRADE`), so the number that matters is the **share of
//! noon** a frame's ground reaches and what that share looks like after the tone map. This prints
//! both, plus the Moon's altitude, so a night that is drawn as a day is a row and not an opinion.

use voxel_appearance::sky::SkyInputs;
use voxel_client::skylight::{
    daylight_of, exposed_at, peak_sun_luminance, sky_ambient_at, SkyModel, EXPOSURE,
};
use voxel_shapers::{Config, WorldSpec};

/// A surface's own albedo, so the "display" column is a mid-grey wall's and not a bare radiance.
const ALBEDO: f64 = 0.7;

fn ground(rig: &voxel_appearance::npr::LightRig) -> (f64, f64) {
    let beam = rig
        .lights
        .iter()
        .map(|l| l.colour.luminance() * l.intensity)
        .sum::<f64>();
    (beam, rig.ambient.luminance())
}

fn main() {
    let spec = WorldSpec::new(voxel_client::game::GAME_SEED, Config::default());
    let inputs = SkyInputs::default();
    let epoch = voxel_client::game::GAME_NEW_MOON_TICK;
    let opening = voxel_client::game::GAME_TICK;

    let noon = daylight_of(&spec, opening, inputs, epoch).expect("a sky");
    let rig = exposed_at(&noon.rig, noon.grade);
    let (day_beam, day_ambient) = ground(&rig);
    let day = day_beam + day_ambient;
    println!(
        "noon (t{opening}): sun {:.1}°, grade {:.3}, exposure {EXPOSURE}, beam {day_beam:.3} + \
         ambient {day_ambient:.3} = {day:.3}",
        noon.sun_altitude_deg, noon.grade
    );
    let peak = peak_sun_luminance(&spec, inputs);
    println!();
    println!("  tick   sun°  moon°  grade    beam     amb   of noon  display  lights   sky(zenith)  sky(moon+20°)");

    for tick in (opening..opening + 24_000).step_by(750) {
        let d = daylight_of(&spec, tick, inputs, epoch).expect("a sky");
        let air = voxel_client::sun::air_at(&spec, 0.0, 0.0, tick);
        let orbit = voxel_shapers::sky::moon_position(&spec, tick, epoch);
        let model = SkyModel::of_graded(&d.sky, &air, peak, &orbit, d.grade);
        let zenith_sky = sky_ambient_at(&model, 90.0, d.sky.sun_azimuth_deg).luminance();
        let moon_sky = sky_ambient_at(&model, 20.0, orbit.azimuth_deg).luminance();
        let rig = exposed_at(&d.rig, d.grade);
        let (beam, ambient) = ground(&rig);
        let total = beam + ambient;
        let moon = voxel_shapers::sky::moon_position(&spec, tick, epoch);
        // The display value of a wall: the tone map on the shaded colour, `L / (1 + L)`.
        let lit = total * ALBEDO;
        let display = lit / (1.0 + lit);
        println!(
            "t{tick:6} {:6.2} {:6.1}  {:5.1}  {:7.3}  {:6.3}  {:7.3}%  {:6.3}  {}  {:12.3e}  {:12.3e}",
            d.sky.sun_altitude_deg,
            moon.altitude_deg,
            d.grade,
            beam,
            ambient,
            total / day * 100.0,
            display,
            d.lights,
            zenith_sky,
            moon_sky
        );
    }
}
