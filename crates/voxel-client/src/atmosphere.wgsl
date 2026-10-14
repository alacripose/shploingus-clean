// The air's transport, once, for every pass that draws light through it — docs/lighting-sky-and-moon.md

/// The rows `sky_model` carries: six header rows and three per band.
const SKY_MODEL_ROWS: u32 = 42u;

/// The bands `voxel_light::spectra` declares. A row's band count is checked against it.
const BAND_COUNT: u32 = 12u;

/// The response's weights and the transport's own numbers, as the CPU wrote them.
///
/// **Binding 5 in both passes that hold it**, and the same buffer: the world pass reflects the sky
/// it is standing in, so a second buffer would be a second sky.
@group(0) @binding(5) var<storage, read> sky_model: array<vec4<f32>>;

/// Radians per degree, and the two phase functions' constants.
const DEGREES_PER_RADIAN: f32 = 57.29577951308232;
const RAYLEIGH_PHASE_SCALE: f32 = 3.0 / (16.0 * 3.141592653589793);
const INV_FOUR_PI: f32 = 1.0 / (4.0 * 3.141592653589793);

/// **The zenith angle's air mass**, Kasten & Young (1989) — the same formula
/// `voxel_light::solver::air_mass` and `voxel_shapers::sky::air_mass` carry.
///
/// Below the horizon the formula leaves its own domain and this returns the horizon's own value: a
/// depressed sun's light does not cross a longer path, it crosses the same horizon path in a sky
/// whose lit air is shrinking, which is the twilight share's job and not an extrapolation's.
fn air_mass(altitude_deg: f32, grazing: f32) -> f32 {
    let z = (90.0 - max(altitude_deg, 0.0)) / DEGREES_PER_RADIAN;
    let denominator = cos(z) + 0.50572 * cos(z + 0.00473 * z * z * z);
    return clamp(1.0 / max(denominator, 1.0 / grazing), 1.0, grazing);
}

/// How small `sin e` may be before the crossing is taken as grazing, on this side of the horizon.
const SIN_FLOOR: f32 = 1.0e-6;

/// **How far a ray at this elevation is inside a horizontal slab of this depth above the eye.**
///
/// Looking up, the ray leaves the slab at its top, so the crossing is `depth / sin e` however long
/// the ray is; looking level or down, the ray is inside it for its whole length. This is why fog is
/// a white-out at the horizon and a thin veil overhead.
///
/// **One expression, and the floor is the reason.** The `min` already answers the level case as
/// `sin e -> 0` — `depth / sin e` diverges and `distance_m` is smaller — so a second branch can only
/// *disagree* with the limit it is standing in for, which is what it did: with an empty droplet
/// layer (`depth = 0`) the branch returned the whole 323 km air path where the expression returns
/// zero, and the frame went black in the one tick the Sun crossed the horizon. `voxel_light::solver`
/// carries the same function and the same floor; the two are held to each other by test.
fn path_in_layer(distance_m: f32, elevation_deg: f32, layer_depth_m: f32) -> f32 {
    let sin_e = max(sin(elevation_deg / DEGREES_PER_RADIAN), SIN_FLOOR);
    return min(distance_m, layer_depth_m / sin_e);
}

/// Rayleigh's `3/(16π)·(1 + cos²θ)`.
fn rayleigh_phase(cos_theta: f32) -> f32 {
    return RAYLEIGH_PHASE_SCALE * (1.0 + cos_theta * cos_theta);
}

/// Henyey–Greenstein at the given asymmetry: `(1 − g²)/(4π(1 + g² − 2g·cosθ)^1.5)`.
///
/// A denominator that has gone to zero or below returns nothing rather than a `NaN`: a phase is a
/// share of the light and a share of nothing is zero.
fn henyey_greenstein(cos_theta: f32, g: f32) -> f32 {
    let denominator = 1.0 + g * g - 2.0 * g * cos_theta;
    if (denominator <= 0.0) {
        return 0.0;
    }
    return (1.0 - g * g) * INV_FOUR_PI / pow(denominator, 1.5);
}

/// **The phase function of the air and the droplets together**, weighted by what each scatters.
///
/// `voxel_light::solver::mixed_phase` exactly, with its two ratios reduced: that function divides a
/// phase *by* the air's optical depth and then multiplies by it again
/// (`air_scatter · (r·R + a·H)/(r + a)`), which is `r·R + a·H`. The reduction is written here
/// because it is the same number with one fewer division, and the test that holds this file to the
/// Rust holds the reduced form.
fn mixed_phase(
    rayleigh_od: f32,
    aerosol_od: f32,
    droplet_scatter_per_m: f32,
    cos_theta: f32,
    mie_asymmetry: f32,
    droplet_asymmetry: f32,
) -> f32 {
    let total = rayleigh_od + aerosol_od + droplet_scatter_per_m;
    if (total <= 0.0) {
        return 0.0;
    }
    let air = rayleigh_od * rayleigh_phase(cos_theta)
        + aerosol_od * henyey_greenstein(cos_theta, mie_asymmetry);
    let droplets = droplet_scatter_per_m * henyey_greenstein(cos_theta, droplet_asymmetry);
    return (air + droplets) / total;
}

/// **The sky**, exposed, for one ray: **both** sources' bands carried through this air — the Sun's
/// and the Moon's, the second being the first reflected off the Moon's disc — as the colour the
/// frame draws.
///
/// `direction` is a unit world vector — `x` east, `y` up, `z` south, the frame `sun_direction`
/// builds the row-2 sun in. A ray below the horizon is answered with the horizon's own sky for the
/// reason the dome clamped: the pass draws every pixel, the ground is not this pass's to know, and
/// a reflected ray that got here is above the horizon by its caller's own guard.
///
/// **The magnitude and the shape are split the way the rest of the client splits them**
/// (`voxel_light::visible_rgb`'s own doctrine, which `sun::DrawnMoon` already follows for the Moon's
/// body): the bands carry the magnitude, the response carries the shape, and a chromaticity is a
/// ratio of two sums of the same bands. So this returns `shape · total · exposure`, which is what
/// `skylight::sky_exposed_at` computes on the CPU and what the tests compare pixel for pixel.
fn sky_radiance_rgb(direction: vec3<f32>) -> vec3<f32> {
    // **A refusal, not a padded read.** A model smaller than this file's own layout is not a sky
    // with some rows missing; no admitted input means no picture.
    if (arrayLength(&sky_model) < SKY_MODEL_ROWS) {
        return vec3<f32>(0.0);
    }
    let head = sky_model[0];
    let sun_row = sky_model[1];
    let optics = sky_model[3];
    // The Moon's own rows — docs/lighting-sky-and-moon.md
    let moon_row = sky_model[4];
    let moon_dir_row = sky_model[5];
    let moon_air_path = moon_row.x;
    let moon_drop_path = moon_row.y;
    let moon_lit = moon_row.z;
    let sun_visible = moon_row.w;
    let scale_height_m = head.x;
    let droplets = head.y;
    let droplet_layer_depth_m = head.z;
    let droplet_albedo = head.w;
    let exposure = sky_model[2].w;
    let mie_asymmetry = optics.x;
    let droplet_asymmetry = optics.y;
    let grazing = optics.z;
    let bands = min(u32(optics.w), BAND_COUNT);

    // The view's own column: the air is a scale height deep, the droplets their own layer tens of
    // metres deep — two media, two geometries. One shared column would extinguish the beam before
    // it arrived and a fog would never load a sky.
    let elevation_deg = clamp(asin(clamp(direction.y, -1.0, 1.0)) * DEGREES_PER_RADIAN, 0.0, 90.0);
    let sin_e = abs(sin(elevation_deg / DEGREES_PER_RADIAN));
    let view_air_mass = clamp(1.0 / max(sin_e, 1.0e-9), 1.0, grazing);
    let air_path_view = scale_height_m * view_air_mass;
    let drop_path_view = path_in_layer(air_path_view, elevation_deg, droplet_layer_depth_m);

    // The droplets move light into the line of sight rather than taking it away: a visible droplet
    // is very nearly a conservative scatterer, which is why a fog *loads* a sky as it hides the
    // ground behind it.
    let droplet_scatter = droplets * droplet_albedo;
    let cos_theta = dot(direction, sky_model[2].xyz);

    var acc = vec3<f32>(0.0);
    var total = 0.0;
    for (var index = 0u; index < bands; index = index + 1u) {
        let a = sky_model[6u + index * 3u];
        let b = sky_model[7u + index * 3u];
        let c = sky_model[8u + index * 3u];
        let extinction = a.x;
        let beta_e = extinction + droplets;
        let beta_s = a.y + droplet_scatter;
        // The sun's own column, and the twilight's share of it: as the sun sinks the air path grows
        // without bound, so the beam is extinguished long before the *scattered* light is.
        let tau_sun = extinction * sun_row.x + droplets * sun_row.y;
        let sun_transmittance = exp(-tau_sun) * sun_row.z;
        let t_view = exp(-(extinction * air_path_view + droplets * drop_path_view));
        // The single-scatter integral in closed form; both media enter through the ratio.
        var single_scatter = 0.0;
        if (beta_e > 0.0) {
            let phase = mixed_phase(
                a.w,
                b.x,
                droplet_scatter,
                cos_theta,
                mie_asymmetry,
                droplet_asymmetry,
            );
            single_scatter = (beta_s / beta_e) * (1.0 - t_view) * phase;
        }
        // The Moon's own light, and it is the Sun's light reflected — docs/lighting-sky-and-moon.md
        var lunar = 0.0;
        if (beta_e > 0.0) {
            let tau_moon = extinction * moon_air_path + droplets * moon_drop_path;
            let moon_phase = mixed_phase(
                a.w,
                b.x,
                droplet_scatter,
                dot(direction, moon_dir_row.xyz),
                mie_asymmetry,
                droplet_asymmetry,
            );
            let moon_power = a.z * c.y * moon_lit;
            lunar = moon_power * (beta_s / beta_e) * (1.0 - t_view) * moon_phase * exp(-tau_moon);
        }
        // The floor is a **radiance**: added, never scaled by the band's own power.
        let radiance =
            a.z * sun_visible * single_scatter * sun_transmittance + lunar + sun_row.w * b.y;
        acc = acc + radiance * vec3<f32>(b.z, b.w, c.x);
        total = total + radiance;
    }

    // The shape is the response's own sum of the same bands, so the two are one evaluation and not
    // a colour and a brightness measured separately.
    let shape = acc.r + acc.g + acc.b;
    if (shape <= 0.0 || total <= 0.0) {
        return vec3<f32>(0.0);
    }
    return acc * (exposure * total / shape);
}
