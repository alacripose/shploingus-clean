//! **Where a block's light comes from**, split into the beam and the sky — the measurement
//! behind *"the blocks are just lit by themselves"*.
//!
//! ```text
//! cargo run --offline -p voxel-client --example rig-balance
//! ```
//!
//! A rig is `lights` (the Sun's beam, and the Moon's) plus `ambient` (the sky's own dome
//! radiance), and those two are added by `npr::shade` with the **same weight**: each is a
//! multiplier on the material's own albedo. So whether a picture reads as *sunlit* or as
//! *self-lit* is the ratio between them, and that ratio is what this prints.
//!
//! The split is measured **by the model, not by a second copy of the model**: the sky's own
//! contribution is the rig with its lights removed and the beam's is the rig with its ambient
//! set black, both through `shade_cpu`.

use voxel_appearance::npr::LightRig;
use voxel_appearance::openpbr::LinearRgb;
use voxel_client::shading::{inputs_for, shade_cpu};
use voxel_client::skylight::{daylight_of, peak_sun_luminance, sky_inputs_at};
use voxel_noise::Vec3;
use voxel_shapers::{Config, WorldSpec};

/// The two terms `npr::shade` adds, in the model's own units: `(sky, beam)`.
fn split(inputs: &voxel_appearance::openpbr::OpenPbrInputs, normal: Vec3, view: Vec3, rig: &LightRig) -> (f64, f64) {
    let sky_only = LightRig { lights: Vec::new(), ambient: rig.ambient };
    let beam_only = LightRig { lights: rig.lights.clone(), ambient: LinearRgb::BLACK };
    let shade = |r: &LightRig| {
        shade_cpu(inputs, normal, view, 0.0, r)
            .expect("a stated rig shades")
            .0
            .luminance()
    };
    (shade(&sky_only), shade(&beam_only))
}

fn main() {
    let spec = WorldSpec::new(2468, Config::default());
    let per_day = voxel_shapers::climate::TICKS_PER_DAY;
    let (x, z) = voxel_client::game::GAME_SPAWN_XZ;
    let registry = voxel_shapers::materials::registry().expect("the admitted registry");
    let rock = inputs_for(&registry, voxel_shapers::materials::REGOLITH)
        .expect("regolith derives");

    println!(
        "the world: seed 2468, a day is {per_day} ticks, spawn column ({x}, {z}), material regolith"
    );
    println!("the game opens at tick {}\n", voxel_client::game::GAME_TICK);

    println!(
        "{:>7}  {:>7}  {:>9}  {:>9}  {:>9}  {:>8}  {:>8}  {:>8}  {:>8}",
        "tick", "alt deg", "peak W/m2", "sun W/m2", "zenith", "beam i", "ambient", "up: sky", "up: beam"
    );
    let up = Vec3::new(0.0, 1.0, 0.0);
    for step in 0..8 {
        let tick = step * (per_day / 8);
        let inputs = sky_inputs_at(&spec, x, z, tick);
        let peak = peak_sun_luminance(&spec, inputs);
        let Some(daylight) = daylight_of(&spec, tick, inputs, 0) else {
            println!("{tick:>7}  the atmosphere was refused");
            continue;
        };
        let rig = &daylight.rig;
        let intensity = rig.lights.first().map_or(0.0, |l| l.intensity);
        let (sky, beam) = split(&rock, up, up, rig);
        println!(
            "{tick:>7}  {:>7.2}  {peak:>9.1}  {:>9.1}  {:>9.1}  {intensity:>8.4}  {:>8.4}  {sky:>8.4}  {beam:>8.4}",
            daylight.sun_altitude_deg,
            daylight.sky.sun.luminance(),
            daylight.sky.zenith.luminance(),
            rig.ambient.luminance(),
        );
    }

    // **The picture's own contrast**, at the tick the client opens on: one face turned to the
    // Sun and one away from it, both through the shipped rig. A surface lit by its own albedo
    // shows the same value on both; a sunlit one does not.
    let tick = voxel_client::game::GAME_TICK;
    let inputs = sky_inputs_at(&spec, x, z, tick);
    let daylight = daylight_of(&spec, tick, inputs, 0).expect("the shipped tick has an atmosphere");
    let rig = &daylight.rig;
    let toward_sun = rig
        .lights
        .first()
        .map(|l| l.direction.normalise())
        .unwrap_or(Vec3::new(0.0, 1.0, 0.0));
    let away = Vec3::new(-toward_sun.x, 0.0, -toward_sun.z).normalise();
    let view = vec3_up();
    let (facing_sky, facing_beam) = split(&rock, toward_sun, view, rig);
    let (away_sky, away_beam) = split(&rock, away, view, rig);
    println!(
        "\nat tick {tick} ({} deg), regolith:\n  face toward the Sun: sky {facing_sky:.4} + beam {facing_beam:.4} = {:.4}\n  face away from it:   sky {away_sky:.4} + beam {away_beam:.4} = {:.4}\n  the Sun's share of the lit face: {:.1}%",
        daylight.sun_altitude_deg,
        facing_sky + facing_beam,
        away_sky + away_beam,
        100.0 * facing_beam / (facing_sky + facing_beam).max(f64::EPSILON),
    );
}

fn vec3_up() -> Vec3 {
    Vec3::new(0.0, 1.0, 0.0)
}
