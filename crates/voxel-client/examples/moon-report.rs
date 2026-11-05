//! Temporary probe: why the drawn Moon is black at the night the GPU test picks.
use voxel_appearance::sky::SkyInputs;
use voxel_client::sun::air_at;
use voxel_shapers::{climate::TICKS_PER_DAY, Config, WorldSpec};

fn main() {
    let spec = WorldSpec::new(2468, Config::default());
    let inputs = SkyInputs::default();
    let ticks_per_day = TICKS_PER_DAY;
    // The GPU test's own search, transcribed.
    let mut night = None;
    'year: for day in 0..voxel_shapers::climate::DAYS_PER_YEAR {
        let start = day * ticks_per_day;
        if voxel_shapers::sky::moon_phase_fraction(start, 0) < 0.5 {
            continue;
        }
        for t in start..start + ticks_per_day {
            let sun = voxel_shapers::sky::solar_position(&spec, t).altitude_deg;
            let moon = voxel_shapers::sky::moon_position(&spec, t, 0);
            if (-22.0..-18.0).contains(&sun) && moon.altitude_deg > 0.0 {
                night = Some(t);
                break 'year;
            }
        }
    }
    let tick = night.expect("a night");
    let sun = voxel_shapers::sky::solar_position(&spec, tick);
    let moon = voxel_shapers::sky::moon_position(&spec, tick, 0);
    println!("tick {tick}");
    println!(
        "  sun  alt {:7.2}  az {:6.1}",
        sun.altitude_deg, sun.azimuth_deg
    );
    println!(
        "  moon alt {:7.2}  az {:6.1}  elongation {:.1}  phase {:.3}",
        moon.altitude_deg, moon.azimuth_deg, moon.elongation_deg, moon.phase_fraction
    );

    let to_body = voxel_shapers::sky::direction_of(moon.altitude_deg, moon.azimuth_deg);
    let to_sun = voxel_shapers::sky::direction_of(sun.altitude_deg, sun.azimuth_deg);
    let dot = to_body[0] * to_sun[0] + to_body[1] * to_sun[1] + to_body[2] * to_sun[2];
    println!("  dot(to_body, to_sun) = {dot:+.4}");
    // The three eye-facing faces of the body and their own incidences: the phase a cube can show.
    let axes = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let mut lit_faces = 0;
    for (i, a) in axes.iter().enumerate() {
        let facing_eye = a[0] * to_body[0] + a[1] * to_body[1] + a[2] * to_body[2];
        let n = if facing_eye < 0.0 {
            [-a[0], -a[1], -a[2]]
        } else {
            *a
        };
        let inc = (n[0] * to_sun[0] + n[1] * to_sun[1] + n[2] * to_sun[2]).max(0.0);
        if facing_eye < 0.0 && inc > 0.0 {
            lit_faces += 1;
        }
        println!("  axis {i}: facing_eye {facing_eye:+.3} incidence {inc:.3}");
    }
    println!("  visible faces of the body lit: {lit_faces} of 3");

    // And the single pixel the shader's entry-face rule gives for the body's own centre ray.
    let mut best: f64 = -1.0;
    for d in 0..64u32 {
        let frac = d as f64 / 64.0;
        let _ = frac;
    }
    let air = air_at(&spec, 0.0, 0.0, tick);
    let _ = air;
    let peak = voxel_client::skylight::peak_sun_luminance(&spec, inputs);
    println!("  night illuminance peak {peak:.3e}");
    // The entry face the shader picks for the centre ray: the dominant axis of `to_body`.
    let dominant = (0..3)
        .max_by(|a, b| to_body[*a].abs().partial_cmp(&to_body[*b].abs()).unwrap())
        .unwrap();
    let n = {
        let mut n = [0.0; 3];
        n[dominant] = -to_body[dominant].signum();
        n
    };
    let inc = (n[0] * to_sun[0] + n[1] * to_sun[1] + n[2] * to_sun[2]).max(0.0);
    best = best.max(inc);
    let verdict = if inc > 0.0 { "LIT" } else { "BLACK" };
    println!(
        "  entry face (axis {dominant}) incidence {inc:.3}  -> the whole visible face is {verdict}"
    );
}
