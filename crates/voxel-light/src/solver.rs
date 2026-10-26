//! **The solver — the sun's bands carried through the air, and nothing hand-rolled.**
//!
//! # What this replaces
//!
//! The sky was drawn from a hand-tuned gradient: a photometric dome whose shape was
//! whatever the renderer's author chose, and whose night was a *declared absence* — zero
//! radiance below the horizon, so every surface lost its ambient at sunset and the world
//! stopped rendering its materials. Neither is a measurement, and both are a second
//! model of an atmosphere this tree already has.
//!
//! Here the sunlight is [`crate::spectra`]'s twelve bands, each carrying its own
//! extinction and scattering coefficient, and the air it crosses is supplied by the
//! caller from [`voxel_shapers::atmosphere`]'s density and [`voxel_shapers::fog`]'s
//! droplets. The night is *computed*, not declared: the beam is exactly zero the moment
//! the planet occludes the body, while the scattered term keeps falling smoothly through
//! twilight and lands on the night sky's own measured radiance.
//!
//! # Why the caller supplies the air and the air mass
//!
//! The coupling in this tree runs one way — light is a leaf, the simulation never links
//! a float solver — so this module holds **no dependency at all**, not even on the
//! shapers that own the air. The density, the scale height, the droplet load and the
//! cloud's water path arrive as an [`Air`]; the path lengths arrive as a [`SunPath`].
//! Every one of them is a measurement the shapers already make.
//!
//! # What is declared, and says so
//!
//! The photopic conversion (683 lm/W at 555 nm), the RGB response lobes (Wyman, Sloan &
//! Shirley 2013's single-lobe fits), Mie's asymmetry and the twilight floor are declared
//! constants with sources. Nothing here is a dial turned to taste.
//!
//! # Where the derivations went
//!
//! `docs/light-solver.md`: the closed-form single-scatter term, the two media and their two
//! geometries, the Moon as a reflectance with its own measured numbers, the response
//! calibration, and the droplet optics. The crate's prose share may fall and may not rise, and
//! that document is where a reader looks for a reason.

use crate::spectra::{
    aerosol_optical_depth, rayleigh_optical_depth, BandTable, BAND_COUNT,
};

/// The photopic luminous efficacy at 555 nm, lm/W. A definition, not a measurement.
pub const LUMENS_PER_WATT_AT_555NM: f64 = 683.0;

/// The night sky's own illuminance at astronomical twilight, lux.
///
/// **The user's own reference, and the reason a night is not black.** *"−18°: astronomical
/// ends, ~0.001 lx"* (`docs/missing_atmosphere_deepseek.md` §4.5). The floor this module
/// adds is that number, which is a measurement of the sky rather than a floor chosen so
/// the picture is not black.
pub const NIGHT_SKY_ILLUMINANCE_LX: f64 = 0.001;

/// The solar altitude at which the night sky's own radiance takes over, degrees.
pub const ASTRONOMICAL_TWILIGHT_DEG: f64 = -18.0;

/// Mie's asymmetry parameter, the one `voxel_appearance::sky` also declares.
pub const MIE_ASYMMETRY_G: f64 = 0.80;

/// **How much of the light a water droplet takes is scattered rather than absorbed.**
///
/// A visible droplet is very nearly a conservative scatterer — the standard figure is a
/// single-scatter albedo within a percent of one — which is why fog *loads* a sky as it hides the
/// ground behind it. The one percent that is not is declared here.
pub const DROPLET_SINGLE_SCATTER_ALBEDO: f64 = 0.99;

/// The droplets' own asymmetry: a droplet of this size throws light forward, hard.
pub const DROPLET_ASYMMETRY_G: f64 = 0.85;

/// The reference's own twilight decay, per degree of solar depression.
///
/// *"`E_twilight(alt) = 400 lx · exp(0.85 · alt)` for `alt ≤ 0°`"* —
/// `docs/missing_atmosphere_deepseek.md` §4.2, whose own table it reproduces to the lux
/// (2.4 lx at −6°, against the table's 2.5).
pub const TWILIGHT_DECAY_PER_DEG: f64 = 0.85;

/// A grazing path's air mass, capped so a ray along the horizon is long rather than infinite.
pub const GRAZING_AIR_MASS: f64 = 38.0;

/// **The air the light crosses**, all of it a measurement the shapers already made.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Air {
    /// Air density at the datum, kg/m³, from `voxel_shapers::atmosphere`.
    pub ground_density_kg_m3: f64,
    /// The scale height, **real metres** — the units every coefficient here is in.
    pub scale_height_m: f64,
    /// The near-surface droplet load, g/m³, from `voxel_shapers::fog`.
    pub droplet_lwc_g_m3: f64,
    /// **The droplet layer's own depth, real metres**, from `voxel_shapers::fog`'s base and top.
    ///
    /// It is the layer and not the column that decides how much fog a ray crosses: a fog is tens of
    /// metres deep, so a light ray meets its own depth of it rather than a scale height of it.
    pub droplet_layer_depth_m: f64,
    /// The column's cloud water path, metres, from `voxel_shapers::clouds`.
    pub cloud_water_path_m: f64,
}

/// **Where the sun is.** Its path length through the air is this module's own optics, not an
/// argument: [`air_mass`] is the published formula and a caller that handed in its own could
/// disagree with the sky it is lighting.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SunPath {
    /// The sun's altitude, degrees. **Negative is a sun the planet has occluded.**
    pub altitude_deg: f64,
    /// The sun's azimuth, degrees east-north-up from north.
    pub azimuth_deg: f64,
}

/// A refusal from the solver. A non-finite input is never a number to guess past.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SolverError {
    /// A value was not finite.
    NonFinite { what: &'static str, value: f64 },
    /// A density, scale height or load was negative.
    Negative { what: &'static str, value: f64 },
    /// The atmosphere has no scale height, so no path through it has an answer.
    NoAtmosphere,
}

impl core::fmt::Display for SolverError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NonFinite { what, value } => write!(f, "{what} is not finite: {value}"),
            Self::Negative { what, value } => write!(f, "{what} is negative: {value}"),
            Self::NoAtmosphere => write!(f, "the air has no scale height, so it has no path"),
        }
    }
}

impl std::error::Error for SolverError {}

impl Air {
    /// Refuses an atmosphere this solver cannot carry light through.
    pub fn validate(self) -> Result<Self, SolverError> {
        for (what, value) in [
            ("the ground density", self.ground_density_kg_m3),
            ("the scale height", self.scale_height_m),
            ("the droplet load", self.droplet_lwc_g_m3),
            ("the droplet layer's depth", self.droplet_layer_depth_m),
            ("the cloud water path", self.cloud_water_path_m),
        ] {
            if !value.is_finite() {
                return Err(SolverError::NonFinite { what, value });
            }
            if value < 0.0 {
                return Err(SolverError::Negative { what, value });
            }
        }
        if self.scale_height_m <= 0.0 {
            return Err(SolverError::NoAtmosphere);
        }
        Ok(self)
    }
}

impl SunPath {
    /// Refuses a sun this solver cannot place.
    pub fn validate(self) -> Result<Self, SolverError> {
        for (what, value) in [
            ("the sun's altitude", self.altitude_deg),
            ("the sun's azimuth", self.azimuth_deg),
        ] {
            if !value.is_finite() {
                return Err(SolverError::NonFinite { what, value });
            }
        }
        Ok(self)
    }
}

/// **The air mass a path at this altitude crosses, in scale heights.**
///
/// Kasten & Young (1989), *Revised optical air mass tables and approximation formula*, and the
/// same formula `voxel_shapers::sky::air_mass` carries — duplicated here because this crate links
/// nothing, and a client can hold the two to each other where both are in scope.
///
/// Below the horizon the formula leaves its own domain, and this returns the horizon's own value:
/// a depressed sun's light does not cross a *longer* path, it crosses the same horizon path in a
/// sky whose lit air is shrinking — which is [`twilight_fraction`]'s job and not an extrapolation's.
pub fn air_mass(altitude_deg: f64) -> f64 {
    let z = (90.0 - altitude_deg.max(0.0)).to_radians();
    let denominator = z.cos() + 0.505_72 * (z + 0.004_73 * z.powi(3)).cos();
    (1.0 / denominator.max(1.0 / GRAZING_AIR_MASS)).clamp(1.0, GRAZING_AIR_MASS)
}

/// **How much of the horizon's own light a twilight still delivers.**
///
/// The reference's anchored exponential (§4.2), so the sky's decay after sunset follows a measured
/// curve rather than whatever a single-scatter integral does when its beam is extinguished.
pub fn twilight_fraction(altitude_deg: f64) -> f64 {
    if altitude_deg >= 0.0 {
        1.0
    } else {
        (TWILIGHT_DECAY_PER_DEG * altitude_deg).exp()
    }
}

/// **The droplets' extinction per real metre**, from the guide's own visibility fit.
///
/// `Vis = 0.0219 · LWC^-0.9603` (km, for g/m³) is `docs/fog_mist_foam_and_aerosol_dispersion.pdf`'s
/// empirical relation, and Koschmieder's `V = 3.912 / β` turns it into an extinction. So the
/// droplets decide the visibility rather than a humidity reading, which is the guide's §10.1 rule.
pub fn droplet_extinction_per_m(lwc_g_m3: f64) -> f64 {
    if !(lwc_g_m3 > 0.0) {
        return 0.0;
    }
    let visibility_km = 0.0219 * lwc_g_m3.powf(-0.960_3);
    let extinction_per_km = 3.912 / visibility_km;
    extinction_per_km / 1_000.0
}

/// **The scattered share of a band's extinction against the beam's own direction.**
///
/// Rayleigh's `3/(16π)·(1 + cos²θ)` and a Henyey–Greenstein with the declared asymmetry,
/// mixed by the two terms' own optical depths — the split `spectra::clear_sky` keeps inside its
/// extinction, read back apart at the band's centre.
pub fn phase_weighted_scatter(lambda_um: f64, cos_theta: f64) -> f64 {
    let rayleigh = rayleigh_optical_depth(lambda_um);
    let aerosol = aerosol_optical_depth(lambda_um);
    let total = rayleigh + aerosol;
    if total <= 0.0 {
        return 0.0;
    }
    let rayleigh_phase =
        3.0 / (16.0 * core::f64::consts::PI) * (1.0 + cos_theta * cos_theta);
    let g = MIE_ASYMMETRY_G;
    let denominator = 1.0 + g * g - 2.0 * g * cos_theta;
    let mie_phase = if denominator > 0.0 {
        (1.0 - g * g) / (4.0 * core::f64::consts::PI * denominator.powf(1.5))
    } else {
        0.0
    };
    (rayleigh * rayleigh_phase + aerosol * mie_phase) / total
}

/// **The night sky's own radiance**, W/m²/sr, from the reference's twilight illuminance.
///
/// Photopic lux to watts at 555 nm, then divided by π: a bright sky is a Lambertian surface to
/// within the approximation this whole model already carries.
pub fn night_sky_radiance_w_m2_sr() -> f64 {
    NIGHT_SKY_ILLUMINANCE_LX / LUMENS_PER_WATT_AT_555NM / core::f64::consts::PI
}

/// **The beam: what the sun's disc delivers, band by band, exactly zero when occluded.**
///
/// The sun's body is a sphere and the world is on its far side at night, so the beam is not *dim*
/// below the horizon — it is absent, and the exactness is the point: a twilight that dimmed the
/// beam by an enormous factor would be a lie about where the light went.
///
/// **A Moon in the way dims it rather than removing it**, and by its own geometry: the beam is
/// scaled by [`sun_visible_fraction`], so a total eclipse takes it to exactly zero and a partial
/// one takes part of it. The two occlusions are different facts — one is the planet, the other is a
/// body standing in front of the source — and this is the one place either is applied.
pub fn direct_beam(
    table: &BandTable,
    air: &Air,
    sun: &SunPath,
    moon: Option<&MoonPath>,
) -> Result<[f64; BAND_COUNT], SolverError> {
    let air = air.validate()?;
    let sun = sun.validate()?;
    let mut out = [0.0f64; BAND_COUNT];
    if sun.altitude_deg <= 0.0 {
        return Ok(out);
    }
    let visible = match moon {
        Some(moon) => sun_visible_fraction(&sun, &moon.validate()?),
        None => 1.0,
    };
    if visible <= 0.0 {
        return Ok(out);
    }
    // A droplet and a vapour molecule take the same light out of the beam — **over their own
    // paths**. The air is a scale height deep; the droplets are a layer of their own depth, which
    // is the geometry `sky_radiance` and `moon_radiance` already carry. Carrying the droplet rate
    // over the whole column instead made a cloud thicker than the sky it hides, and made the day's
    // beam and the night's — the same beam, reflected — disagree about the same air:
    // `the_beams_droplets_are_their_own_layer_and_not_the_whole_column`.
    let air_path = air.scale_height_m * air_mass(sun.altitude_deg);
    let droplets = droplet_extinction_per_m(air.droplet_lwc_g_m3)
        + cloud_extinction_per_m(air.cloud_water_path_m, air.scale_height_m);
    let drop_path = path_in_layer(air_path, sun.altitude_deg, air.droplet_layer_depth_m);
    for (index, power) in out.iter_mut().enumerate() {
        let band = &table.bands()[index];
        let tau = band.extinction_per_m * air_path + droplets * drop_path;
        *power = band.sun_w_per_m2 * (-tau).exp() * visible;
    }
    Ok(out)
}

/// **The phase function of the air and the droplets together**, weighted by what each scatters.
///
/// The droplets throw light forward hard, so a fog's sky is bright *around the sun* — a shape that
/// a Rayleigh-only phase cannot produce and that the single-scatter weight would otherwise hide.
pub fn mixed_phase(lambda_um: f64, droplet_scatter_per_m: f64, cos_theta: f64) -> f64 {
    let rayleigh = rayleigh_optical_depth(lambda_um);
    let aerosol = aerosol_optical_depth(lambda_um);
    let air_scatter = rayleigh + aerosol;
    let air_phase = phase_weighted_scatter(lambda_um, cos_theta);
    let total = air_scatter + droplet_scatter_per_m;
    if total <= 0.0 {
        return 0.0;
    }
    let g = DROPLET_ASYMMETRY_G;
    let denominator = 1.0 + g * g - 2.0 * g * cos_theta;
    let droplet_phase = if denominator > 0.0 {
        (1.0 - g * g) / (4.0 * core::f64::consts::PI * denominator.powf(1.5))
    } else {
        0.0
    };
    (air_scatter * air_phase + droplet_scatter_per_m * droplet_phase) / total
}

/// **The cloud's extinction per real metre of liquid water path**, dimensionless per metre.
///
/// **Linear, and that is the whole point.** This is `cloud_field`'s own declared rate —
/// `CLOUD_EXTINCTION_PER_M = 0.01`, a saturated metre of the field's water turning a column opaque
/// in about a kilometre — restated here in the solver's units, because the sky's optics and the
/// cloud field's march are one medium and must not disagree about it. A `voxel-client` test holds
/// the two numbers equal.
///
/// **It used to be the fog visibility fit, and the fit diverges at zero.**
/// [`droplet_extinction_per_m`] carries `Vis = 0.0219 · LWC^-0.9603`, which is right for *fog* —
/// an air parcel whose LWC is a measured, finite quantity — and is quoted for `LWC` in
/// `0.05..0.5 g/m³`. Reaching it from a column water path puts that path, divided by a scale
/// height, into the fit: at **`1.4e-7 m` of water** the exponent turns that into **`5e-6 /m`**, a
/// third of the whole Rayleigh extinction, and the slope there is unbounded. Measured at the
/// opening day's sunset: the horizon band's radiance stepped **`0.20636 → 0.22324` (×1.082)** on
/// the first tick the column had any water at all, and the rig's ambient went **`0.01124 →
/// 0.01314`** over the next four — a visible wash of sky light arriving in one update, with no
/// cloud yet in the frame. Linear in the path, one cloud is one law: the term is a part in `1e7`
/// of the air's own extinction until the column is a real one.
pub const CLOUD_EXTINCTION_PER_M: f64 = 0.01;

/// **A cloud's own extinction per metre**, from a column water path: the cloud field's optical
/// depth, `σ · LWP`, spread over the air's own scale height.
///
/// **The division is the units, and it was missing.** `σ · LWP` is an *optical depth* —
/// dimensionless — and the solver's `droplets` is an extinction **per metre**, carried in the same
/// sum as the air's own. Feeding the depth in as if it were a rate made an eighth of a millimetre
/// of column water weigh as much as a third of the whole atmosphere: measured at the opening
/// day's sunset, the first tick with any water at all carried `LWP = 8.7e-4 m`, and the droplet
/// term came out at `8.7e-6 /m` against the air's `~1e-4` — **8.7 % of the atmosphere's own
/// extinction from a wisp**, which moved the horizon band's radiance `0.20636 → 0.22819` (×1.106)
/// and the rig's ambient with it. Spread over the scale height instead, that wisp is `1.1e-9 /m`
/// (a part in `1e5`), and a real `125 m` column — this world's deepest, from
/// `examples/aerosol-column.rs` — is `1.6e-4 /m`, a veil the sky can see. Linear in the water path,
/// so nothing steps at zero, and **one coefficient for every ray**: the sky model uploads this
/// number in one row and must not vary it with the view, or the CPU reading and the solver's own
/// transport are two media wearing one name (`the_model_is_the_solvers_own_transport_at_every_angle`).
pub fn cloud_extinction_per_m(cloud_water_path_m: f64, scale_height_m: f64) -> f64 {
    if !cloud_water_path_m.is_finite() || cloud_water_path_m <= 0.0 || !(scale_height_m > 0.0) {
        return 0.0;
    }
    CLOUD_EXTINCTION_PER_M * cloud_water_path_m / scale_height_m
}

/// **The sky: what the ray from the eye carries, band by band.**
///
/// Single scattering of the sun's own beam, plus the night sky's measured floor. The twilight is
/// the term doing the work: as the sun sinks, `sun_air_mass` grows without bound so the beam is
/// extinguished long before the *scattered* light is, which is why a sky stays lit after sunset
/// and why the world's materials never lose their ambient.
pub fn sky_radiance(
    table: &BandTable,
    air: &Air,
    sun: &SunPath,
    moon: Option<&MoonPath>,
    view_elevation_deg: f64,
    view_azimuth_deg: f64,
) -> Result<[f64; BAND_COUNT], SolverError> {
    let air = air.validate()?;
    let sun = sun.validate()?;
    let moon = moon.copied();
    let moon = match moon {
        Some(moon) => Some(moon.validate()?),
        None => None,
    };
    if !view_elevation_deg.is_finite() || !view_azimuth_deg.is_finite() {
        return Err(SolverError::NonFinite {
            what: "the view direction",
            value: if view_elevation_deg.is_finite() { view_azimuth_deg } else { view_elevation_deg },
        });
    }

    // The view's own air mass, floored at a grazing path so a horizon ray is long, not infinite.
    let view_air_mass = (1.0 / view_elevation_deg.to_radians().sin().abs()).clamp(1.0, GRAZING_AIR_MASS);
    let droplets = droplet_extinction_per_m(air.droplet_lwc_g_m3)
        + cloud_extinction_per_m(air.cloud_water_path_m, air.scale_height_m);
    let cos_theta = scatter_angle_cosine(sun.altitude_deg, sun.azimuth_deg, view_elevation_deg, view_azimuth_deg);

    // **The floor the reference measures.** Below astronomical twilight the sky is the night
    // sky's own radiance, which is small and is not nothing.
    let night = night_sky_radiance_w_m2_sr();

    // **The second source, and it is the first one reflected.** The Moon's own band power arrives
    // before any air, its own column is read at its own altitude, and its own phase angle is
    // formed once — because the sky scatters the light *arriving from the body*, which is a
    // different quantity from the beam that reaches the ground.
    //
    // **And the Sun's own power carries its own visibility**, so a Moon crossing the Sun's disc
    // takes the scattered light out of the sky at the same instant it takes the beam off the
    // ground. That is the whole of an eclipse: two bodies, one frame, one multiply.
    let sun_visible = match moon {
        Some(moon) => sun_visible_fraction(&sun, &moon),
        None => 1.0,
    };
    let moon_power = moon.map(|moon| moon_sky_power(table, &moon)).unwrap_or([0.0; BAND_COUNT]);
    let (moon_air_path, moon_drop_path, cos_theta_moon) = match moon {
        Some(moon) => {
            let elevation = moon.altitude_deg.max(0.0);
            let air_path = air.scale_height_m * air_mass(elevation);
            let drop_path = path_in_layer(air_path, elevation, air.droplet_layer_depth_m);
            (
                air_path,
                drop_path,
                scatter_angle_cosine(
                    moon.altitude_deg,
                    moon.azimuth_deg,
                    view_elevation_deg,
                    view_azimuth_deg,
                ),
            )
        }
        None => (0.0, 0.0, 0.0),
    };

    let mut out = [0.0f64; BAND_COUNT];
    for (index, value) in out.iter_mut().enumerate() {
        let band = &table.bands()[index];
        let centre_um = band.centre_nm() * 1.0e-3;

        // One medium, two coefficients: the droplets move light into the line of sight.
        let droplet_scatter = droplets * DROPLET_SINGLE_SCATTER_ALBEDO;
        let beta_e = band.extinction_per_m + droplets;
        let beta_s = band.scatter_per_m + droplet_scatter;

        // Two media, two geometries: the air is a scale height, the droplets their own layer.
        let air_path_view = air.scale_height_m * view_air_mass;
        let drop_path_view =
            path_in_layer(air_path_view, view_elevation_deg, air.droplet_layer_depth_m);
        let tau_view = band.extinction_per_m * air_path_view + droplets * drop_path_view;

        // The sun's own column, with the reference's twilight decay below the horizon.
        let sun_elevation = sun.altitude_deg.max(0.0);
        let air_path_sun = air.scale_height_m * air_mass(sun_elevation);
        let drop_path_sun = path_in_layer(air_path_sun, sun_elevation, air.droplet_layer_depth_m);
        let tau_sun = band.extinction_per_m * air_path_sun + droplets * drop_path_sun;
        let sun_transmittance = (-tau_sun).exp()
            * if sun.altitude_deg > 0.0 { 1.0 } else { twilight_fraction(sun.altitude_deg) };

        let t_view = (-tau_view).exp();
        let phase = mixed_phase(centre_um, droplet_scatter, cos_theta);

        // The single-scatter integral in closed form; both media enter through the ratio.
        let single_scatter = if beta_e > 0.0 {
            (beta_s / beta_e) * (1.0 - t_view) * phase
        } else {
            0.0
        };
        let scattered = band.sun_w_per_m2 * sun_visible * single_scatter * sun_transmittance;

        // **The Moon's own scattering of the Sun's own light**, through its own column and at its
        // own phase angle. There is no twilight factor here and there must not be: the twilight
        // curve is the Sun's afterglow, and the Moon is a body.
        let lunar = if moon.is_some() && beta_e > 0.0 {
            let tau_moon = band.extinction_per_m * moon_air_path + droplets * moon_drop_path;
            moon_power[index]
                * (beta_s / beta_e)
                * (1.0 - t_view)
                * mixed_phase(centre_um, droplet_scatter, cos_theta_moon)
                * (-tau_moon).exp()
        } else {
            0.0
        };

        // The floor is a radiance: added, never scaled by the band's power.
        let floor = night * response_night_share(index);
        *value = scattered + lunar + floor;
    }
    Ok(out)
}

/// **How far a ray at this elevation is inside a horizontal slab of this depth above the eye.**
///
/// Looking up, the ray leaves the slab at its top, so the crossing is `depth / sin e` however long
/// the ray is; looking level or down, the ray is inside it for its whole length. This is why fog is
/// a white-out at the horizon and a thin veil overhead, and it is geometry rather than a knob.
///
/// **Public because it is one of the two things a second transcription of this module needs.**
/// `atmosphere.wgsl` carries the transport again for the GPU, and its tests hold the two to each
/// other; a caller assembling what that shader reads (the client's own model upload) needs the same
/// geometry rather than a second guess at it.
pub fn path_in_layer(distance_m: f64, elevation_deg: f64, layer_depth_m: f64) -> f64 {
    // **The floor is where the formula is evaluated, and there is no second branch.** A ray
    // looking level or down is inside a slab of depth `D` for its whole length, which is what the
    // `min` already answers as `sin e -> 0`: `D / sin e` diverges, so the smaller term is
    // `distance_m`. Writing that limit as a floor on `sin e` rather than as an `if` is what makes
    // this **continuous at the horizon**, and the discontinuity was not academic: an empty droplet
    // layer (`D = 0`, which is what an unconfigured world has) answered `0` a hair above the
    // horizon and the *whole* 323 km air path at exactly it, so the Sun's light gained a
    // `droplets x 323 000` optical depth — eighteen, in this world — in the single tick it
    // crossed. `exp(-tau)` fell from `0.86` to `1.2e-8`, the rig's ambient went to zero with it,
    // and the frame went black at the horizon with the exposure then running to `2.8e8`
    // multiplying nothing.
    const SIN_FLOOR: f64 = 1.0e-6;
    let sin_e = elevation_deg.to_radians().sin().max(SIN_FLOOR);
    distance_m.min(layer_depth_m / sin_e)
}

/// Samples per band in the response integral. Declared, because the integral's error is.
pub const RESPONSE_SAMPLES: usize = 64;

/// The visible range the response is defined over, nanometres.
pub const VISIBLE_LO_NM: f64 = 360.0;
/// The visible range's upper end, nanometres.
pub const VISIBLE_HI_NM: f64 = 830.0;

/// **Each lobe's own integral over the visible range**, cached.
///
/// The published lobes are not balanced against each other — the blue one's head is 1.258 against
/// the red one's 1.065 and it is narrower — so a spectrum with equal power in every band came out
/// blue until each channel was divided by its own integral. Without this the whole picture carries
/// a cast, which is a bias in the instrument and not in the world.
fn response_denominator() -> [f64; 3] {
    static DENOMINATOR: std::sync::OnceLock<[f64; 3]> = std::sync::OnceLock::new();
    *DENOMINATOR.get_or_init(|| {
        let (lo, hi) = (VISIBLE_LO_NM.ln(), VISIBLE_HI_NM.ln());
        let step = (hi - lo) / 256.0;
        let mut sums = [0.0f64; 3];
        for sample in 0..256 {
            let nm = (lo + (sample as f64 + 0.5) * step).exp();
            let response = rgb_response(nm);
            for (channel, value) in sums.iter_mut().enumerate() {
                *value += response[channel] * nm * step;
            }
        }
        sums
    })
}

/// The Moon's mean radius, km (IAU WGCCRE). Published, and here because it divides.
pub const MOON_RADIUS_KM: f64 = 1_737.4;

/// The mean Earth–Moon distance, km — the same published source.
pub const MOON_DISTANCE_KM: f64 = 384_400.0;

/// **The solid angle the Moon's disc fills**, sr: `π · (R/d)²`.
///
/// The `π` is the whole of it. `voxel_shapers::sky`'s `SOLID_ANGLE_DILUTION` multiplies a
/// *radiance* by `(R/d)²` alone, which is the fraction of the sky the disc covers and not the
/// solid angle it subtends — a factor of `π` under, and the one term that stands between a
/// Lambertian radiance and an illuminance.
pub const MOON_SOLID_ANGLE_SR: f64 =
    core::f64::consts::PI * (MOON_RADIUS_KM / MOON_DISTANCE_KM) * (MOON_RADIUS_KM / MOON_DISTANCE_KM);

/// **The Sun's own angular radius**, radians: `0.0093 / 2`, the solar angular diameter
/// `voxel_appearance::sky` declares.
///
/// It is here, as a second declaration of a number both modules need, for the reason [`air_mass`]
/// is: this crate links nothing. A client that holds both can check them against each other, and
/// the value is the published one either way — the Sun as seen from the Earth is `0.2665°` in
/// radius, which is why a total eclipse is a knife-edge and not a season.
pub const SUN_ANGULAR_RADIUS_RAD: f64 = 0.004_65;

/// **The Moon's own angular radius**, radians: `R_moon / d_moon` at the mean distance, `0.259°`.
///
/// The Moon and the Sun subtend almost the same angle from the Earth — `0.259°` against `0.2665°` —
/// and everything about an eclipse follows from that near-agreement rather than from a chosen
/// number.
pub const MOON_ANGULAR_RADIUS_RAD: f64 = MOON_RADIUS_KM / MOON_DISTANCE_KM;

/// **How much of the Sun's own disc is still visible**, `[0, 1]`: the eclipse, as geometry.
///
/// Both bodies' directions are known, so the angle between them is a dot product of two unit
/// vectors, and both discs have a published angular radius. Two circles at a known separation
/// cover each other by a known area — the standard lens area
///
/// ```text
/// A = r₁²(θ₁ − sin θ₁ cos θ₁) + r₂²(θ₂ − sin θ₂ cos θ₂)
/// ```
///
/// with `θ` the two half-angles at the intersection points — so the visible fraction is
/// `1 − A/(π r_sun²)`, and it is **exactly** the shape of an eclipse: `1` while the discs do not
/// touch, falling as the Moon's limb crosses the Sun's, `0` when the Moon's disc contains it.
///
/// **This is the term that makes the sky darken at noon.** The Sun's own source power is scaled by
/// it wherever the sky is scattered or the beam is cast, so a total eclipse takes the light out of
/// the sky, the rig, and the ground at once, and a partial one takes part of it — which is the only
/// reason a world with an orbit and a Moon needs the two to be in one frame.
///
/// A Moon the planet has occluded is not in front of the Sun for *this* observer, so a set Moon
/// hides nothing.
pub fn sun_visible_fraction(sun: &SunPath, moon: &MoonPath) -> f64 {
    if moon.altitude_deg <= 0.0 {
        return 1.0;
    }
    let (sin_sa, cos_sa) = sun.altitude_deg.to_radians().sin_cos();
    let (sin_ma, cos_ma) = moon.altitude_deg.to_radians().sin_cos();
    let cos_daz = (sun.azimuth_deg - moon.azimuth_deg).to_radians().cos();
    let separation = (sin_sa * sin_ma + cos_sa * cos_ma * cos_daz)
        .clamp(-1.0, 1.0)
        .acos();

    let (big, small) = if SUN_ANGULAR_RADIUS_RAD >= MOON_ANGULAR_RADIUS_RAD {
        (SUN_ANGULAR_RADIUS_RAD, MOON_ANGULAR_RADIUS_RAD)
    } else {
        (MOON_ANGULAR_RADIUS_RAD, SUN_ANGULAR_RADIUS_RAD)
    };
    if separation >= big + small {
        return 1.0;
    }
    if separation <= big - small {
        return 0.0;
    }
    let d = separation;
    let theta_big = ((d * d + big * big - small * small) / (2.0 * d * big)).clamp(-1.0, 1.0).acos();
    let theta_small = ((d * d + small * small - big * big) / (2.0 * d * small)).clamp(-1.0, 1.0).acos();
    let lens = big * big * (theta_big - theta_big.sin() * theta_big.cos())
        + small * small * (theta_small - theta_small.sin() * theta_small.cos());
    let covered = (lens / (core::f64::consts::PI * SUN_ANGULAR_RADIUS_RAD * SUN_ANGULAR_RADIUS_RAD))
        .clamp(0.0, 1.0);
    (1.0 - covered).clamp(0.0, 1.0)
}

/// **The Moon as a lit surface**: where it is, and how much of its disc the Sun is lighting.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoonPath {
    /// The Moon's altitude, degrees. **Negative is a Moon the planet has occluded.**
    pub altitude_deg: f64,
    /// The Moon's azimuth, degrees east-north-up from north.
    pub azimuth_deg: f64,
    /// The illuminated fraction of the disc the observer sees, `[0, 1]`.
    pub illuminated_fraction: f64,
}

impl MoonPath {
    /// Refuses a Moon this solver cannot place.
    pub fn validate(self) -> Result<Self, SolverError> {
        for (what, value) in [
            ("the moon's altitude", self.altitude_deg),
            ("the moon's azimuth", self.azimuth_deg),
            ("the illuminated fraction", self.illuminated_fraction),
        ] {
            if !value.is_finite() {
                return Err(SolverError::NonFinite { what, value });
            }
        }
        if self.illuminated_fraction < 0.0 {
            return Err(SolverError::Negative {
                what: "the illuminated fraction",
                value: self.illuminated_fraction,
            });
        }
        Ok(self)
    }
}

/// **What the night is lit by: the Sun's own bands, reflected off the Moon.**
///
/// The Moon is not an emitter and this is where that becomes arithmetic: each band is the solar
/// constant's own power in it, times the band's **declared reflectance** (`Band::moon_reflectance`,
/// a reflectance and not a radiance), spread over the disc by Lambert's law, delivered through the
/// Moon's own solid angle, and attenuated back through the same air the Sun crosses. Zero when the
/// planet has occluded the Moon, for the reason the beam is exactly zero at night.
pub fn moonlight(
    table: &BandTable,
    air: &Air,
    moon: &MoonPath,
) -> Result<[f64; BAND_COUNT], SolverError> {
    let moon = moon.validate()?;
    let face = moon_radiance(table, air, &moon)?;
    let illuminated = moon.illuminated_fraction.clamp(0.0, 1.0);
    Ok(core::array::from_fn(|index| {
        face[index] * MOON_SOLID_ANGLE_SR * illuminated
    }))
}

/// **The Moon's own source power for a sky's scattering**, band by band: the lit face's radiance
/// through the disc's own solid angle, and **no air crossed at all**.
///
/// This is the term that was missing. [`moonlight`] is what reaches the *ground* — the reflection
/// down one air column, which is a beam on a surface. A sky does not scatter the beam that reached
/// the ground; it scatters the light arriving from the body, so the sky's lunar term needs the same
/// power *before* the column, which is what this is. Without it the sky had exactly one source, and
/// a night with a full Moon up was lit by nothing at all: the measured symptom was a rig with an
/// ambient of `0.0` at a midnight whose Moon stood 59° high and nearly full.
///
/// Zero for a Moon the planet has occluded and for a Moon whose disc the Sun is not lighting, both
/// by the same arithmetic the beam uses rather than by a branch on the clock.
pub fn moon_sky_power(table: &BandTable, moon: &MoonPath) -> [f64; BAND_COUNT] {
    let lit = moon.illuminated_fraction.clamp(0.0, 1.0);
    if moon.altitude_deg <= 0.0 || lit <= 0.0 {
        return [0.0; BAND_COUNT];
    }
    core::array::from_fn(|index| {
        let band = &table.bands()[index];
        band.sun_w_per_m2 * band.moon_reflectance / core::f64::consts::PI
            * MOON_SOLID_ANGLE_SR
            * lit
    })
}

/// **The radiance of the Moon's lit face**, band by band, W/m²/sr — Lambert's law and the return
/// crossing, before the disc's solid angle.
///
/// The split exists for the shader. A drawn body carries the radiance of a **surface**, and which
/// part of that surface is turned toward the Sun is geometry the shader can compute from the ray;
/// the irradiance the ground receives is this radiance through the disc's own solid angle, which is
/// [`moonlight`]. The same numbers reach the shader and the rig, and the phase is drawn rather than
/// handed over as a number.
pub fn moon_radiance(
    table: &BandTable,
    air: &Air,
    moon: &MoonPath,
) -> Result<[f64; BAND_COUNT], SolverError> {
    let air = air.validate()?;
    let moon = moon.validate()?;
    let mut out = [0.0f64; BAND_COUNT];
    if moon.altitude_deg <= 0.0 {
        return Ok(out);
    }
    let air_path = air.scale_height_m * air_mass(moon.altitude_deg);
    let droplets = droplet_extinction_per_m(air.droplet_lwc_g_m3)
        + cloud_extinction_per_m(air.cloud_water_path_m, air.scale_height_m);
    let drop_path = path_in_layer(air_path, moon.altitude_deg, air.droplet_layer_depth_m);

    for (index, value) in out.iter_mut().enumerate() {
        let band = &table.bands()[index];
        // Lambert's law, then the air on the way back down: the disc's angle is `moonlight`'s.
        let radiance = band.sun_w_per_m2 * band.moon_reflectance / core::f64::consts::PI;
        let tau = band.extinction_per_m * air_path + droplets * drop_path;
        *value = radiance * (-tau).exp();
    }
    Ok(out)
}

/// The share of the night sky's radiance a band carries: the declared response of the night sky's
/// own spectrum, normalised so the twelve bands sum to one.
fn response_night_share(index: usize) -> f64 {
    // Flat across the bands that transport light; this build carries no night spectrum.
    match crate::spectra::bands()[index].transport {
        crate::spectra::Transport::Links => 0.0,
        _ => 1.0 / 10.0,
    }
}

/// The cosine of the angle between the sun and the view, both from the eye's own frame.
fn scatter_angle_cosine(
    sun_altitude_deg: f64,
    sun_azimuth_deg: f64,
    view_elevation_deg: f64,
    view_azimuth_deg: f64,
) -> f64 {
    let (sa, va) = (sun_altitude_deg.to_radians(), view_elevation_deg.to_radians());
    let d_azimuth = (sun_azimuth_deg - view_azimuth_deg).to_radians();
    (sa.sin() * va.sin() + sa.cos() * va.cos() * d_azimuth.cos()).clamp(-1.0, 1.0)
}

/// **The transmittance of a path of light**: what survives the air and the droplets, band by band.
///
/// This is what geometry is drawn through, and it is the same path the sky's light is scattered
/// into — one medium, sampled twice.
pub fn transmittance(
    table: &BandTable,
    air: &Air,
    distance_m: f64,
    elevation_deg: f64,
) -> Result<[f64; BAND_COUNT], SolverError> {
    let air = air.validate()?;
    if !distance_m.is_finite() || distance_m < 0.0 {
        return Err(SolverError::Negative { what: "the path's distance", value: distance_m });
    }
    if !elevation_deg.is_finite() {
        return Err(SolverError::NonFinite { what: "the elevation", value: elevation_deg });
    }
    // `voxel_shapers::atmosphere`'s scaling, read as a path rather than a height.
    let air_mass = (1.0 / elevation_deg.to_radians().sin().abs()).clamp(1.0, GRAZING_AIR_MASS);
    let droplets = droplet_extinction_per_m(air.droplet_lwc_g_m3)
        + cloud_extinction_per_m(air.cloud_water_path_m, air.scale_height_m);
    // The droplets' crossing: the whole distance level, the layer's depth looking up.
    let drop_path = path_in_layer(distance_m, elevation_deg, air.droplet_layer_depth_m);
    let mut out = [0.0f64; BAND_COUNT];
    for (index, value) in out.iter_mut().enumerate() {
        let band = &table.bands()[index];
        let tau = band.extinction_per_m * air_mass * distance_m + droplets * drop_path;
        *value = (-tau).exp();
    }
    Ok(out)
}

/// The red, green and blue response at a wavelength, the published single-lobe fits.
///
/// Wyman, Sloan & Shirley (2013), *Simple Analytic Approximations to the CIE XYZ Color Matching
/// Functions* — the single-lobe forms, which are a cited approximation rather than a taste.
pub fn rgb_response(nm: f64) -> [f64; 3] {
    let lobe = |centre: f64, width: f64| (-0.5 * ((nm - centre) / width).powi(2)).exp();
    [
        1.065 * lobe(595.8, 33.33),
        1.034 * lobe(556.3, 31.4),
        1.258 * lobe(435.7, 31.7),
    ]
}

/// **The tristimulus weight of each band, per channel** — the linear part of [`visible_rgb`],
/// declared once so a renderer holding the bands and no wavelength axis can apply the same numbers.
///
/// A band's colour is an integral over its own range on a log grid, with `nm` as the Jacobian (the
/// quadrature `spectra` uses for Planck), each lobe divided by its own integral over the visible
/// range. That is **linear in the bands**, which is the property this function exists for: the
/// weights are band-by-channel numbers, so a shader multiplies band `i`'s radiance by row `i` and
/// sums, and there is one place where the visible range, the grid, the sample count and the
/// calibration live rather than a second set typed into WGSL.
///
/// A band an eye does not receive ([`Transport::Links`]) weighs exactly nothing — the band that
/// never arrives does not colour the picture — and a caller that wants [`visible_rgb`]'s own
/// magnitude (its chromaticity is a ratio) sums the weights' rows, which is what [`visible_rgb`]
/// does below.
pub fn rgb_weights() -> [[f64; 3]; BAND_COUNT] {
    let mut weights = [[0.0f64; 3]; BAND_COUNT];
    let denominator = response_denominator();
    for (index, row) in weights.iter_mut().enumerate() {
        let band = &crate::spectra::bands()[index];
        // Only the bands an eye receives; the far ones are a beam or nothing.
        if band.transport == crate::spectra::Transport::Links {
            continue;
        }
        // A band is a range, so its colour is an integral over it, not a sample at its centre.
        let (lo, hi) = (band.lo_nm.ln(), band.hi_nm.ln());
        let step = (hi - lo) / RESPONSE_SAMPLES as f64;
        let mut share = [0.0f64; 3];
        for sample in 0..RESPONSE_SAMPLES {
            let nm = (lo + (sample as f64 + 0.5) * step).exp();
            let response = rgb_response(nm);
            for (channel, value) in share.iter_mut().enumerate() {
                *value += response[channel] * nm;
            }
        }
        for (channel, value) in row.iter_mut().enumerate() {
            let scale = if denominator[channel] > 0.0 { 1.0 / denominator[channel] } else { 0.0 };
            *value = share[channel] * step * scale;
        }
    }
    weights
}

/// **The colour of a band spectrum**, as a chromaticity whose channels sum to one.
///
/// The bands carry the magnitude and this carries the shape, which is the division
/// `voxel-light`'s header argues for: a colour has a shape and no magnitude, and a magnitude is
/// not a colour.
pub fn visible_rgb(band_radiance: &[f64; BAND_COUNT]) -> [f64; 3] {
    let weights = rgb_weights();
    let mut out = [0.0f64; 3];
    let mut total = 0.0;
    for (index, power) in band_radiance.iter().enumerate() {
        let magnitude = power.abs();
        for (channel, value) in out.iter_mut().enumerate() {
            *value += weights[index][channel] * magnitude;
            total += weights[index][channel] * magnitude;
        }
    }
    if total > 0.0 {
        for channel in out.iter_mut() {
            *channel /= total;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn air() -> Air {
        Air {
            ground_density_kg_m3: 1.225,
            // Real metres, the unit every coefficient in `spectra` is in.
            scale_height_m: 8_500.0,
            droplet_lwc_g_m3: 0.0,
            droplet_layer_depth_m: 50.0,
            cloud_water_path_m: 0.0,
        }
    }

    fn sun_at(altitude_deg: f64) -> SunPath {
        SunPath { altitude_deg, azimuth_deg: 180.0 }
    }

    #[test]
    fn a_night_is_never_black() {
        // The old model returned zero below the horizon and the world lost its materials.
        let table = BandTable::computed();
        let night = sky_radiance(&table, &air(), &sun_at(-40.0), None, 10.0, 180.0)
            .expect("a night is computable");
        let luminance: f64 = visible_rgb(&night).iter().sum();
        assert!(luminance > 0.0, "the night sky has no colour at all");
        assert!(
            night.iter().all(|band| *band >= 0.0),
            "a band radiance went negative at night"
        );
        assert!(
            night.iter().any(|band| *band > 0.0),
            "every band is dark at night, which is the old absence"
        );

        // And it is *dark*: a night has to be many orders below a noon, or "night" means nothing.
        let noon = sky_radiance(&table, &air(), &sun_at(60.0), None, 10.0, 180.0)
            .expect("a noon is computable");
        let noon_total: f64 = noon.iter().sum();
        let night_total: f64 = night.iter().sum();
        assert!(
            noon_total > 1.0e3 * night_total,
            "noon carries {noon_total} against the night's {night_total}, so the two are the \
             same sky"
        );
    }

    #[test]
    fn the_beam_is_exactly_zero_when_the_planet_occludes_the_body() {
        // Not "dim": absent. A dimmed beam would be a lie about where the light went.
        let table = BandTable::computed();
        let set = direct_beam(&table, &air(), &sun_at(0.0), None).expect("a sun on the horizon");
        assert!(
            set.iter().all(|band| *band == 0.0),
            "the beam survived the planet's own limb"
        );
        let risen = direct_beam(&table, &air(), &sun_at(0.5), None).expect("a risen sun");
        assert!(
            risen.iter().all(|band| *band >= 0.0),
            "a band's beam went negative"
        );
        // The X-ray band never arrives: its Rayleigh column is thousands of depths thick.
        assert_eq!(risen[0], 0.0, "X-rays reached the ground");
        // A rising sun is dim and red because it crosses about 38 air masses.
        let fraction = risen[4] / table.bands()[4].sun_w_per_m2;
        assert!(
            (0.000_5..0.02).contains(&fraction),
            "half a degree above the horizon delivered {fraction} of the green band"
        );
        let high = direct_beam(&table, &air(), &sun_at(45.0), None).expect("a high sun");
        assert!(
            high[4] > 0.7 * table.bands()[4].sun_w_per_m2,
            "a 45° sun delivered only {} of the green band",
            high[4]
        );
    }

    #[test]
    fn the_twilight_falls_smoothly_and_the_beam_does_not() {
        // The beam stops dead at the horizon; the sky keeps falling past it.
        let table = BandTable::computed();
        let mut last = f64::INFINITY;
        for altitude in [0.5, 0.0, -2.0, -6.0, -12.0, -18.0] {
            let sky = sky_radiance(&table, &air(), &sun_at(altitude), None, 5.0, 180.0)
                .expect("a computable sky");
            let total: f64 = sky.iter().sum();
            assert!(
                total < last,
                "the sky at {altitude}° carries {total}, not below {last}"
            );
            last = total;
        }
        // Below astronomical twilight the floor holds.
        let deep = sky_radiance(&table, &air(), &sun_at(-80.0), None, 5.0, 180.0).expect("a deep night");
        let floor: f64 = deep.iter().sum();
        let reference = night_sky_radiance_w_m2_sr();
        assert!(
            floor >= reference,
            "the deep night carries {floor}, under the reference's own {reference}"
        );
        assert!(
            floor < 1.2 * reference,
            "the deep night carries {floor}, which is more than the night sky's own {reference}"
        );
    }

    #[test]
    fn the_twilight_is_the_references_own_curve() {
        // The reference's own curve, against its own table.
        let elapsed = |altitude: f64| twilight_fraction(altitude) * 400.0;
        assert!((elapsed(0.0) - 400.0).abs() < 1.0e-9);
        // The formula is implemented; the bounds allow the table's own rounding.
        assert!((elapsed(-4.0) - 400.0 * (-3.4f64).exp()).abs() < 1.0e-9);
        assert!((elapsed(-4.0) - 15.0).abs() < 2.0, "−4° gave {} lx, not the table's ~15", elapsed(-4.0));
        assert!((elapsed(-6.0) - 2.5).abs() < 0.5, "−6° gave {} lx, not the table's ~2.5", elapsed(-6.0));
        // And above the horizon there is no twilight to apply.
        assert_eq!(twilight_fraction(30.0), 1.0);
    }

    #[test]
    fn the_daylight_sky_is_blue_for_rayleighs_own_reason() {
        // Not chosen: β(λ) falls as λ⁻⁴·⁰⁹, so the sunlit air scatters blue by itself.
        let table = BandTable::computed();
        let sky = sky_radiance(&table, &air(), &sun_at(45.0), None, 40.0, 0.0).expect("a day sky");
        let rgb = visible_rgb(&sky);
        assert!(
            rgb[2] > rgb[0],
            "the sunlit sky's blue is {} against a red of {}, so the sky is not Rayleigh's",
            rgb[2],
            rgb[0]
        );
    }

    #[test]
    fn more_droplets_put_more_light_into_the_sky_and_take_more_out_of_the_path() {
        // Fog is a medium: it scatters and extinguishes, and one function must do both.
        let table = BandTable::computed();
        let clear = air();
        let foggy = Air { droplet_lwc_g_m3: 0.021, ..clear };
        // Level, because the visibility the threshold names is a horizontal one.
        let clear_t = transmittance(&table, &clear, 1_000.0, 0.0).expect("a clear path");
        let foggy_t = transmittance(&table, &foggy, 1_000.0, 0.0).expect("a foggy path");
        assert!(
            foggy_t[4] < clear_t[4],
            "the droplets transmitted {} against the clear air's {}",
            foggy_t[4],
            clear_t[4]
        );
        assert!(
            (foggy_t[4] - (-3.912f64).exp()).abs() < 0.02,
            "a kilometre at the fog threshold transmitted {}, not about {}",
            foggy_t[4],
            (-3.912f64).exp()
        );

        let clear_sky = sky_radiance(&table, &clear, &sun_at(45.0), None, 5.0, 0.0).expect("clear");
        let fog_sky = sky_radiance(&table, &foggy, &sun_at(45.0), None, 5.0, 0.0).expect("foggy");
        let clear_total: f64 = clear_sky.iter().sum();
        let fog_total: f64 = fog_sky.iter().sum();
        assert!(
            fog_total > clear_total,
            "the fog dimmed the sky ({fog_total} against {clear_total}) instead of loading it"
        );
    }

    #[test]
    fn the_beams_droplets_are_their_own_layer_and_not_the_whole_column() {
        // **The beam crosses the droplet layer, not the air column.** The droplets are tens of
        // metres deep, so the beam crosses `depth / sin(alt)` of them — the geometry
        // `sky_radiance`, `moon_radiance` and `transmittance` already use, applied to the beam.
        // Carrying their rate over the whole scale height instead made a cloud thicker than the sky
        // it hides, and made the day's beam and the night's — the same beam, reflected — disagree
        // about the same air.
        let table = BandTable::computed();
        let clear = air();
        let sun = sun_at(30.0);
        let before: f64 = direct_beam(&table, &clear, &sun, None)
            .expect("a clear beam")
            .iter()
            .sum();
        // A cloud's water with **no droplet layer declared** is a column the sky scatters and the
        // beam does not cross: zero depth is a zero path, not a whole scale height.
        let no_layer =
            Air { cloud_water_path_m: 0.05, droplet_layer_depth_m: 0.0, ..clear };
        let through_none: f64 = direct_beam(&table, &no_layer, &sun, None)
            .expect("a beam")
            .iter()
            .sum();
        assert_eq!(
            before, through_none,
            "a cloud with no droplet layer took light out of the beam"
        );
        // Give the droplets their layer and the beam loses light in it — and loses *more* as the
        // layer deepens, which is what says the layer is what is crossed.
        let thin = Air { droplet_layer_depth_m: 40.0, ..no_layer };
        let thick = Air { droplet_layer_depth_m: 400.0, ..no_layer };
        let through_thin: f64 = direct_beam(&table, &thin, &sun, None)
            .expect("a beam")
            .iter()
            .sum();
        let through_thick: f64 = direct_beam(&table, &thick, &sun, None)
            .expect("a beam")
            .iter()
            .sum();
        assert!(
            through_thin < before,
            "forty metres of droplets cost the beam nothing ({through_thin} against {before})"
        );
        assert!(
            through_thick < through_thin,
            "a deeper droplet layer cost the beam no more ({through_thick} against \
             {through_thin})"
        );
        // And the layer's own path is `depth / sin(alt)`: the same droplets under a higher Sun are
        // crossed for less of the beam's length, so the beam keeps more light.
        let low: f64 = direct_beam(&table, &thin, &sun_at(30.0), None)
            .expect("a low beam")
            .iter()
            .sum();
        let high: f64 = direct_beam(&table, &thin, &sun_at(60.0), None)
            .expect("a high beam")
            .iter()
            .sum();
        assert!(
            high > low,
            "a higher Sun lost more light to the layer ({high} against {low})"
        );
    }

    #[test]
    fn the_references_own_visibility_relation_is_the_one_used() {
        // `Vis = 0.0219 · LWC^-0.9603` km gives about a kilometre at the threshold.
        let extinction = droplet_extinction_per_m(0.021);
        let visibility_km = 3.912 / (extinction * 1_000.0);
        assert!(
            (0.5..1.5).contains(&visibility_km),
            "the declared fog threshold put the visibility at {visibility_km} km"
        );
        assert_eq!(droplet_extinction_per_m(0.0), 0.0, "no droplets extinguished light");
        assert_eq!(droplet_extinction_per_m(-1.0), 0.0, "a negative load extinguished light");
    }

    #[test]
    fn the_rgb_response_puts_the_bands_where_an_eye_puts_them() {
        // The fit, checked where it is decidable.
        let blue = rgb_response(450.0);
        let green = rgb_response(555.0);
        let red = rgb_response(620.0);
        assert!(blue[2] > blue[1] && blue[2] > blue[0], "450 nm was not blue: {blue:?}");
        assert!(green[1] > green[0] && green[1] > green[2], "555 nm was not green: {green:?}");
        assert!(red[0] > red[1] && red[0] > red[2], "620 nm was not red: {red:?}");
        let nir = rgb_response(1_500.0);
        assert!(
            nir.iter().all(|channel| *channel < 1.0e-4),
            "the near infrared reached a picture: {nir:?}"
        );
    }

    #[test]
    fn the_response_makes_a_flat_spectrum_neutral() {
        // Equal power in every band must come out neutral: each lobe divided by its own integral.
        let flat = [1.0f64; BAND_COUNT];
        let rgb = visible_rgb(&flat);
        let (red, green, blue) = (rgb[0], rgb[1], rgb[2]);
        assert!(
            (red - green).abs() < 0.08 && (green - blue).abs() < 0.08,
            "a flat spectrum rendered as ({red:.3}, {green:.3}, {blue:.3})"
        );
    }

    #[test]
    fn the_suns_own_spectrum_comes_out_near_white() {
        // The model's reference white: printed as well as bounded.
        let table = BandTable::computed();
        let sun: [f64; BAND_COUNT] = core::array::from_fn(|index| table.bands()[index].sun_w_per_m2);
        let rgb = visible_rgb(&sun);
        println!(
            "the sun's own bands render as ({:.3}, {:.3}, {:.3})",
            rgb[0], rgb[1], rgb[2]
        );
        assert!(
            rgb.iter().all(|channel| (channel - 1.0 / 3.0).abs() < 0.15),
            "the sun came out as ({:.3}, {:.3}, {:.3})",
            rgb[0],
            rgb[1],
            rgb[2]
        );
    }

    #[test]
    fn the_moon_is_a_reflectance_and_not_an_emitter() {
        // The test an emitter cannot pass: a reflected Moon doubles with the Sun.
        let table = BandTable::computed();
        let moon = MoonPath { altitude_deg: 45.0, azimuth_deg: 90.0, illuminated_fraction: 1.0 };
        let full = moonlight(&table, &air(), &moon).expect("a full moon");
        let total: f64 = full.iter().sum();
        println!("a full moon overhead delivers {total:.4e} W/m2 in the twelve bands");

        // A crescent is a fraction of the disc and not a dimmer Moon.
        let crescent = moonlight(
            &table,
            &air(),
            &MoonPath { illuminated_fraction: 0.1, ..moon },
        )
        .expect("a crescent");
        let crescent_total: f64 = crescent.iter().sum();
        assert!(
            (crescent_total - 0.1 * total).abs() < 1.0e-12,
            "a tenth-lit moon gave {crescent_total} against a tenth of {total}"
        );

        // Occluded is exactly nothing, the same rule the Sun's own beam obeys.
        let set = moonlight(&table, &air(), &MoonPath { altitude_deg: 0.0, ..moon })
            .expect("a moon on the horizon");
        assert!(set.iter().all(|band| *band == 0.0), "a set moon lit the world");

        // And it is redder than the Sun: the table's reflectance rises with wavelength.
        let sun: [f64; BAND_COUNT] = core::array::from_fn(|index| table.bands()[index].sun_w_per_m2);
        let (sun_rgb, moon_rgb) = (visible_rgb(&sun), visible_rgb(&full));
        assert!(
            moon_rgb[0] / moon_rgb[2] > sun_rgb[0] / sun_rgb[2],
            "the moonlight is no redder than the sunlight: {moon_rgb:?} against {sun_rgb:?}"
        );
    }

    #[test]
    fn the_full_moons_illuminance_lands_where_the_published_figures_put_it() {
        // A full Moon is about a quarter-lux; the published range is 0.05–0.3 lx.
        let table = BandTable::computed();
        let moon = MoonPath { altitude_deg: 90.0, azimuth_deg: 0.0, illuminated_fraction: 1.0 };
        let full = moonlight(&table, &air(), &moon).expect("a full moon at the zenith");
        let irradiance: f64 = full.iter().sum();
        let lux = irradiance * LUMENS_PER_WATT_AT_555NM;
        println!(
            "full moon at the zenith: {irradiance:.4e} W/m2, {lux:.3} lx at 683 lm/W \
             (the real figure is lower, the Moon's spectrum being red)"
        );
        assert!(
            (1.0e-4..1.0e-2).contains(&irradiance),
            "the full moon delivered {irradiance} W/m2, which is not a moon"
        );
        // Six orders under the Sun, which is `MAT-0012`.
        let sun: f64 = table.bands().iter().map(|band| band.sun_w_per_m2).sum();
        assert!(
            sun / irradiance > 1.0e5,
            "the moon came out at {:.1e} of the sun",
            sun / irradiance
        );
    }

    #[test]
    fn a_refusal_is_a_refusal_and_not_a_guess() {
        let table = BandTable::computed();
        assert!(matches!(
            Air { ground_density_kg_m3: f64::NAN, ..air() }.validate(),
            Err(SolverError::NonFinite { .. })
        ));
        assert!(matches!(
            Air { droplet_lwc_g_m3: -1.0, ..air() }.validate(),
            Err(SolverError::Negative { .. })
        ));
        assert!(matches!(
            Air { scale_height_m: 0.0, ..air() }.validate(),
            Err(SolverError::NoAtmosphere)
        ));
        assert!(matches!(
            transmittance(&table, &air(), -1.0, 5.0),
            Err(SolverError::Negative { .. })
        ));
        assert!(matches!(
            sky_radiance(&table, &air(), &sun_at(45.0), None, f64::INFINITY, 0.0),
            Err(SolverError::NonFinite { .. })
        ));
    }
}
