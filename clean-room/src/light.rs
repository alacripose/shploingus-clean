//! **U5's light: one band table, and a solver whose numbers are consequences of it.**
//!
//! Derived from `docs/light-solver.md` (the record's own derivation document — re-derived here, not
//! copied: no line comes from any `.rs` in the legacy tree), the published tables that document
//! cites, and `docs/missing_atmosphere_deepseek.md` §4.2/§4.5, the user's own reference for the
//! twilight curve and the night sky. Every constant carries its source in its own doc comment.
//!
//! # The closed form, and why it is closed
//!
//! For one band, along one ray, the scattered radiance is
//!
//! ```text
//! L = E_sun,band · (β_s / β_e) · (1 − t_view) · P(θ) · T_sun + floor
//! ```
//!
//! `(β_s/β_e)·(1 − t_view)` is the single-scatter integral solved rather than marched: the share of
//! the band the medium scatters, times the share of the path it has not transmitted away. `β_e` is
//! the band's extinction — air **plus** droplets — and `β_s` the share of it that scatters, so fog
//! *loads* a sky as it hides the ground: the light is not lost, it is moved into the line of sight.
//!
//! # Two media, two geometries
//!
//! The air is one scale height deep; the droplets are their own layer, tens of metres deep, so a ray
//! looking up leaves the droplet layer quickly while it is still inside the air. One shared column
//! would extinguish the beam before it arrived and the fog would never brighten anything.
//!
//! # What is not modelled here, and is owed
//!
//! **Ozone** absorption; an **azimuth**, because this solver is one plane through the Sun (a caller
//! with a bearing is asking a question this module does not answer); the **frame format** a per-cell
//! measurement must travel in (`docs/light-solver.md` §6 calls that the remaining step, and it
//! belongs to the renderer rather than the solver); and the record's own **turbidity link**
//! (`WET_TURBIDITY_GAIN = 3.0`), which connects a wet column to a hazier one and is a caller's
//! mapping rather than this solver's arithmetic.

/// The visible span the bands cover, in nanometres.
pub const VISIBLE_LO_NM: f64 = 400.0;
/// The visible span's upper end, in nanometres.
pub const VISIBLE_HI_NM: f64 = 700.0;
/// How many bands the visible span is divided into — the record's own band count, twelve.
pub const CHANNEL_COUNT: usize = 12;
/// Quadrature points across the visible span. The grid is geometric, so this is a point count in
/// `ln λ`, and the Jacobian is `λ` itself.
pub const QUADRATURE_POINTS: usize = 256;

/// The air's own molecular scattering coefficient at the reference wavelength, per metre.
///
/// Bodhaine et al. (1999)'s standard atmosphere at 550 nm: `1.16e-5 m⁻¹` (0.0116 km⁻¹), the published
/// table `docs/TODO.md` names and the legacy light crate cites.
pub const RAYLEIGH_REFERENCE_PER_M: f64 = 1.16e-5;
/// The wavelength that reference coefficient is stated at, in nanometres (Bodhaine's own 550 nm).
pub const RAYLEIGH_REFERENCE_NM: f64 = 550.0;
/// Rayleigh scattering falls as `λ^-4.09` — the record's own exponent (`docs/light-solver.md` §1),
/// which is Bodhaine's tabulated fit rather than the ideal `λ^-4`.
pub const RAYLEIGH_EXPONENT: f64 = 4.09;
/// Ångström's aerosol exponent: `τ(λ) = β · (λ / 1 µm)^-α`. `1.3` is the reference's own default
/// (`docs/missing_atmosphere_deepseek.md` §1.5: "α typically ranges from 0.5 to 2.5, with 1.3 being a
/// common default").
pub const ANGSTROM_EXPONENT: f64 = 1.3;
/// The standard atmosphere's scale height, in metres — the depth the air's column is charged over.
pub const SCALE_HEIGHT_M: f64 = 8_000.0;
/// The droplets' own layer depth, in metres.
///
/// **Declared, not published.** `docs/light-solver.md` §1 says the droplets are "their own layer tens
/// of metres deep" and prints no number; this is that, declared rather than guessed at, and the tests
/// hold its *consequences* (no layer is no path, a deeper layer costs more, a higher Sun crosses
/// less of it).
pub const DROPLET_LAYER_M: f64 = 40.0;
/// Droplets scatter 99% of what they take (`DROPLET_SINGLE_SCATTER_ALBEDO`, `docs/light-solver.md` §1).
pub const DROPLET_SINGLE_SCATTER_ALBEDO: f64 = 0.99;
/// The droplets' Henyey–Greenstein asymmetry: forward-peaked at `g = 0.85` (the record's own figure).
pub const DROPLET_ASYMMETRY_G: f64 = 0.85;
/// The visibility relation's coefficient: `Vis = 0.0219 · LWC^-0.9603` in km for g/m³ of liquid water
/// (the fog/aerosol reference `docs/light-solver.md` §5 cites).
pub const VISIBILITY_COEFFICIENT: f64 = 0.0219;
/// The visibility relation's exponent, and the same source as the coefficient.
pub const VISIBILITY_EXPONENT: f64 = 0.9603;
/// Koschmieder's constant: `β = 3.912 / V`.
pub const KOSCHMIEDER: f64 = 3.912;
/// The fog threshold the record declares: `0.021 g/m³` is about a kilometre of visibility, which is
/// why the threshold is not a rounder number (`docs/light-solver.md` §5).
pub const FOG_THRESHOLD_LWC_G_M3: f64 = 0.021;
/// The night sky's own measured illuminance, in lux — the user's own reference figure
/// (`docs/missing_atmosphere_deepseek.md` §4.5's astronomical-twilight row).
pub const NIGHT_SKY_ILLUMINANCE_LX: f64 = 0.001;
/// The photopic conversion's own maximum, in lumens per watt.
pub const LUMENS_PER_WATT_PHOTOPIC: f64 = 683.0;
/// The Moon's own lumens-per-watt, nearer a reflected-sunlight figure than the photopic maximum —
/// which is why the record prints the 683 lm/W number as a *warning* beside the measurement.
pub const MOON_LUMENS_PER_WATT: f64 = 100.0;
/// The twilight curve's anchor: `400 lx` at the horizon
/// (`docs/missing_atmosphere_deepseek.md` §4.2, `E(alt) = 400 lx · exp(0.85 · alt)`).
pub const TWILIGHT_LUX_AT_HORIZON: f64 = 400.0;
/// The twilight curve's own decay per degree, in the exponent.
pub const TWILIGHT_DECAY_PER_DEG: f64 = 0.85;
/// The Sun's own effective temperature, in kelvin — the body the Sun's bands are drawn from.
pub const SUN_KELVIN: f64 = 5_778.0;
/// The solar constant: the twelve bands' own total, in W/m².
pub const SOLAR_CONSTANT_W_M2: f64 = 1_361.0;
/// The Moon's radius, in metres (published mean radius).
pub const MOON_RADIUS_M: f64 = 1_737_400.0;
/// The Moon's mean distance, in metres (published mean distance).
pub const MOON_DISTANCE_M: f64 = 384_400_000.0;
/// The Moon's disc-integrated albedo at the blue end: the Moon is redder than the Sun, and its
/// reflectance **rises with wavelength** (the record's own statement, `docs/light-solver.md` §3;
/// published lunar geometric albedos run from about 0.07 in the blue to about 0.16 in the red).
pub const MOON_ALBEDO_BLUE: f64 = 0.075;
/// The Moon's disc-integrated albedo at the red end, same source as [`MOON_ALBEDO_BLUE`].
pub const MOON_ALBEDO_RED: f64 = 0.155;

/// The twilight curve of `docs/missing_atmosphere_deepseek.md` §4.2, in lux: `400 · exp(0.85 · alt)`,
/// never below the night sky's own measured `0.001 lx`.
///
/// Above the horizon the anchor is what it is (`400 lx`); below it the curve decays for eighteen
/// degrees, and the floor is what a night is. **A night is dim; it is not an absence** — the defect
/// this module exists to end was a model that returned zero below the horizon, an ambient that
/// followed it to zero, and a world that stopped drawing its materials at sunset.
pub fn twilight_lux(altitude_deg: f64) -> f64 {
    let anchored = if altitude_deg >= 0.0 {
        TWILIGHT_LUX_AT_HORIZON
    } else {
        TWILIGHT_LUX_AT_HORIZON * (TWILIGHT_DECAY_PER_DEG * altitude_deg).exp()
    };
    anchored.max(NIGHT_SKY_ILLUMINANCE_LX)
}

/// The altitude at which the curve's own exponential meets the night sky's floor, in degrees:
/// `ln(0.001 / 400) / 0.85`, derived from the two constants rather than measured from a build.
pub fn twilight_knee_deg() -> f64 {
    (NIGHT_SKY_ILLUMINANCE_LX / TWILIGHT_LUX_AT_HORIZON).ln() / TWILIGHT_DECAY_PER_DEG
}

/// The night sky's own radiance, in W/m²/sr, from its measured illuminance: `lx / 683 / π`.
pub fn night_sky_radiance() -> f64 {
    NIGHT_SKY_ILLUMINANCE_LX / LUMENS_PER_WATT_PHOTOPIC / std::f64::consts::PI
}

/// Kasten & Young (1989)'s relative optical air mass for a body at `altitude_deg`.
///
/// `m = 1 / (cos z + 0.50572 · (96.07995 − z)^-1.6364)` with `z` the zenith angle in degrees. At the
/// zenith it is `0.9997`, and at the horizon `37.92` — which is what the record's own
/// `GRAZING_AIR_MASS = 38` is: the formula's own horizon value, rounded and declared.
///
/// **Below the horizon the formula leaves its domain**, and this returns the horizon's own value
/// rather than a longer path: a depressed Sun's light does not cross a longer column, it crosses the
/// same horizon column in a sky whose lit air is shrinking.
pub fn air_mass(altitude_deg: f64) -> f64 {
    let zenith_deg = (90.0 - altitude_deg).min(90.0);
    let denom = zenith_deg.to_radians().cos() + 0.50572 * (96.07995 - zenith_deg).powf(-1.6364);
    1.0 / denom
}

/// One band of the visible span: what it covers, and nothing else.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Channel {
    lo_nm: f64,
    hi_nm: f64,
}

impl Channel {
    /// The channel's lower end, in nanometres.
    const fn lo_nm(&self) -> f64 {
        self.lo_nm
    }

    /// The channel's upper end, in nanometres.
    const fn hi_nm(&self) -> f64 {
        self.hi_nm
    }

    /// The channel's own centre — the wavelength its power law is read at, and *only* that: a band's
    /// colour is an integral over the band rather than a sample at its centre.
    fn centre_nm(&self) -> f64 {
        0.5 * (self.lo_nm + self.hi_nm)
    }
}

/// Twelve numbers, one per band: a radiance, an irradiance or a transmittance, as the reader's own
/// sentence says.
pub type Bands = [f64; CHANNEL_COUNT];

/// The air's own optics: how much it scatters, over how deep, and how that falls with wavelength.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Air {
    /// The column's depth, in metres.
    pub scale_height_m: f64,
    /// Molecular scattering at [`RAYLEIGH_REFERENCE_NM`], per metre.
    pub rayleigh_reference_per_m: f64,
    /// The exponent the molecular coefficient falls with, as `(λ_ref / λ)^exponent`.
    pub rayleigh_exponent: f64,
    /// Ångström's column loading, dimensionless. `0.0` is a clear column; the reference's own typical
    /// range is `0–0.5`.
    pub angstrom_beta: f64,
    /// Ångström's own exponent.
    pub angstrom_exponent: f64,
}

impl Air {
    /// A clear standard atmosphere: Bodhaine's reference coefficient, the record's exponent, and no
    /// aerosol column at all.
    pub fn standard() -> Self {
        Air {
            scale_height_m: SCALE_HEIGHT_M,
            rayleigh_reference_per_m: RAYLEIGH_REFERENCE_PER_M,
            rayleigh_exponent: RAYLEIGH_EXPONENT,
            angstrom_beta: 0.0,
            angstrom_exponent: ANGSTROM_EXPONENT,
        }
    }

    /// The same air with an aerosol column loaded to `beta` (the reference's own `0.0–0.5`) — the
    /// tests' own air; the shipped table declares no aerosol column.
    #[cfg(test)]
    pub fn hazy(beta: f64) -> Self {
        Air { angstrom_beta: beta, ..Air::standard() }
    }

    /// The molecular scattering coefficient at one wavelength, per metre.
    pub fn rayleigh_per_m(&self, lambda_nm: f64) -> f64 {
        self.rayleigh_reference_per_m * (RAYLEIGH_REFERENCE_NM / lambda_nm).powf(self.rayleigh_exponent)
    }

    /// The aerosol column's optical depth at one wavelength — Ångström's own `β · (λ / 1 µm)^-α`.
    pub fn aerosol_optical_depth(&self, lambda_nm: f64) -> f64 {
        self.angstrom_beta * (lambda_nm / 1000.0).powf(-self.angstrom_exponent)
    }

    /// The aerosol column's own coefficient, per metre: the depth over the column's depth.
    pub fn aerosol_per_m(&self, lambda_nm: f64) -> f64 {
        self.aerosol_optical_depth(lambda_nm) / self.scale_height_m
    }
}

/// The droplets' own loading: how much liquid water, over how deep a layer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Droplets {
    /// Liquid water content, in grams per cubic metre.
    pub lwc_g_m3: f64,
    /// The layer's own depth, in metres. `0.0` is no layer at all, which is no path.
    pub layer_m: f64,
}

impl Droplets {
    /// A load in the record's own layer.
    pub fn fog(lwc_g_m3: f64) -> Self {
        Droplets { lwc_g_m3, layer_m: DROPLET_LAYER_M }
    }

    /// A load in a layer of a stated depth — the knob the tests use to hold the record's own claim
    /// that a beam's droplets are their own layer and not the whole column.
    pub fn layer(lwc_g_m3: f64, layer_m: f64) -> Self {
        Droplets { lwc_g_m3, layer_m }
    }

    /// The visibility this load implies, in kilometres: `0.0219 · LWC^-0.9603`.
    pub fn visibility_km(&self) -> f64 {
        VISIBILITY_COEFFICIENT * self.lwc_g_m3.powf(-VISIBILITY_EXPONENT)
    }

    /// The extinction this load implies, per metre, through Koschmieder's `β = 3.912 / V`.
    ///
    /// The droplets **decide the visibility**; a humidity reading turned into a visibility would be a
    /// second model of the same fact.
    pub fn extinction_per_m(&self) -> f64 {
        KOSCHMIEDER / (self.visibility_km() * 1000.0)
    }
}

/// The atmospheric state a measurement is taken in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    /// The Sun's altitude above the horizon, in degrees.
    pub sun_elevation_deg: f64,
    /// The Moon's altitude above the horizon, in degrees.
    pub moon_elevation_deg: f64,
    /// The Moon's illuminated fraction of its disc, in `[0, 1]`. A tenth-lit crescent is a tenth of
    /// the disc, not a dimmer Moon.
    pub moon_illuminated: f64,
    /// The droplet load, or `None` for a sky with no droplets in it at all.
    pub droplets: Option<Droplets>,
}

impl State {
    /// A clear state, with the Moon wherever the caller says.
    pub fn clear(sun_elevation_deg: f64, moon_elevation_deg: f64, moon_illuminated: f64) -> Self {
        State { sun_elevation_deg, moon_elevation_deg, moon_illuminated, droplets: None }
    }
}

/// The band table: the bands, the Sun's own bands, the Moon's own reflectance and the response.
#[derive(Clone, Debug, PartialEq)]
pub struct Spectra {
    air: Air,
    channels: [Channel; CHANNEL_COUNT],
    sun_bands: Bands,
    moon_reflectance: Bands,
}

impl Spectra {
    /// The declared table: Bodhaine's air, the Sun as a 5 778 K body scaled to the solar constant,
    /// and a Moon whose reflectance rises from [`MOON_ALBEDO_BLUE`] to [`MOON_ALBEDO_RED`].
    pub fn standard() -> Self {
        let mut channels = [Channel { lo_nm: 0.0, hi_nm: 0.0 }; CHANNEL_COUNT];
        let step = (VISIBLE_HI_NM - VISIBLE_LO_NM) / CHANNEL_COUNT as f64;
        for (index, channel) in channels.iter_mut().enumerate() {
            channel.lo_nm = VISIBLE_LO_NM + index as f64 * step;
            channel.hi_nm = channel.lo_nm + step;
        }

        // The Sun's bands: Planck's law at 5 778 K, integrated over each band on the geometric grid,
        // then scaled so the twelve bands sum to the published solar constant. The scale is a
        // **calibration**, said out loud: the shape is Planck's, the total is the Sun's own.
        let mut sun_bands = [0.0f64; CHANNEL_COUNT];
        for (index, channel) in channels.iter().enumerate() {
            sun_bands[index] = integrate(channel.lo_nm(), channel.hi_nm(), |lambda| {
                planck_per_nm(lambda, SUN_KELVIN)
            });
        }
        let total: f64 = sun_bands.iter().sum();
        let scale = SOLAR_CONSTANT_W_M2 / total;
        for band in sun_bands.iter_mut() {
            *band *= scale;
        }

        let mut moon_reflectance = [0.0f64; CHANNEL_COUNT];
        for (index, channel) in channels.iter().enumerate() {
            let t = (channel.centre_nm() - VISIBLE_LO_NM) / (VISIBLE_HI_NM - VISIBLE_LO_NM);
            moon_reflectance[index] = MOON_ALBEDO_BLUE + t * (MOON_ALBEDO_RED - MOON_ALBEDO_BLUE);
        }

        Spectra { air: Air::standard(), channels, sun_bands, moon_reflectance }
    }

    /// The same table read through a different air — the tests' own knob; the shipped table is
    /// [`Air::standard`]'s, and nothing on the binary's path swaps it.
    #[cfg(test)]
    pub fn with_air(mut self, air: Air) -> Self {
        self.air = air;
        self
    }

    /// The same table with the Sun's own power scaled — **the check that the Moon is a reflectance
    /// and not an emitter**, since a dimmer Sun must move the Moon and an independent source must
    /// not.
    pub fn with_sun_factor(mut self, factor: f64) -> Self {
        for band in self.sun_bands.iter_mut() {
            *band *= factor;
        }
        self
    }

    /// The air the table is read through.
    pub fn air(&self) -> &Air {
        &self.air
    }

    /// The twelve bands.
    pub fn channels(&self) -> &[Channel; CHANNEL_COUNT] {
        &self.channels
    }

    /// **Which band of this table holds a wavelength**: the band whose own interval contains it, its
    /// lower end inclusive and its upper end exclusive.
    ///
    /// The one caller is the feeling wheel (`src/feeling.rs`), whose six families are anchored at
    /// **real wavelengths** (645, 600, 575, 545, 480 and 420 nm) and whose twelve declared bands have
    /// to be *this* tree's twelve rather than a second table invented beside it (Constitution P3).
    /// The half-open interval is the reading: an anchor landing exactly on a shared edge belongs to
    /// the band it **opens** — 600 nm and 575 nm land there — and a table with two answers for one
    /// wavelength would let one family be voiced in two places.
    ///
    /// `None` for a wavelength outside the table's own span, which is a **refusal and not a clamp**:
    /// there is no band below [`VISIBLE_LO_NM`] or above [`VISIBLE_HI_NM`] to voice, and a caller that
    /// wanted one is owed the name of that absence rather than the nearest band this table happens to
    /// have (`src/feeling.rs` reads that as a feeling off its own ladder).
    pub fn band_of_nm(&self, nm: f64) -> Option<usize> {
        if !nm.is_finite() || nm < VISIBLE_LO_NM || nm >= VISIBLE_HI_NM {
            return None;
        }
        let step = (VISIBLE_HI_NM - VISIBLE_LO_NM) / CHANNEL_COUNT as f64;
        let index = ((nm - VISIBLE_LO_NM) / step).floor() as usize;
        // The arithmetic above walks the same step `Spectra::standard` builds the table with, and the
        // guard is here so a moved span cannot answer with a band that does not hold the wavelength.
        let band_holds = self.channels[index].lo_nm() <= nm && nm < self.channels[index].hi_nm();
        band_holds.then_some(index)
    }

    /// The Sun's own bands, in W/m², **calibrated** so they sum to [`SOLAR_CONSTANT_W_M2`] — the
    /// scale is a calibration, and the checks pin its target as a literal.
    pub fn sun_bands(&self) -> &Bands {
        &self.sun_bands
    }

    /// The Moon's own reflectance per band. **The Moon is not an emitter**: this is a reflectance,
    /// and the light it delivers is the Sun's own, reflected.
    pub fn moon_reflectance(&self) -> &Bands {
        &self.moon_reflectance
    }

    /// The Moon's disc solid angle in steradians: `π · (R/d)²`.
    ///
    /// **The π is the whole of a real discrepancy.** `docs/light-solver.md` §3 records that
    /// `voxel_shapers::sky`'s `SOLID_ANGLE_DILUTION` is `(R/d)²` alone — the *fraction of the sky* the
    /// disc covers, not the solid angle it subtends — and multiplies a radiance by it as though it
    /// were an angle in steradians, so its full moon is a factor of π too dim. This is the solid
    /// angle, and the legacy discrepancy is named rather than inherited.
    pub fn moon_solid_angle_sr(&self) -> f64 {
        let ratio = MOON_RADIUS_M / MOON_DISTANCE_M;
        std::f64::consts::PI * ratio * ratio
    }

    /// The response: a spectrum's own colour, in the sRGB primaries, each channel `0..=1`.
    ///
    /// Two calibrations, both the record's own (`docs/light-solver.md` §4) and both measured below:
    ///
    /// * a band is a range of wavelengths, so its colour is an **integral over the range** on the
    ///   geometric grid, not the response sampled at the band's centre;
    /// * each channel is divided by **its own integral over the visible range**, because the published
    ///   lobes are not balanced against each other and equal power in every band would come out with a
    ///   cast — a bias in the instrument and not a fact about the world.
    pub fn response(&self, bands: &Bands) -> [f64; 3] {
        let weight: f64 = bands.iter().copied().sum::<f64>().max(f64::MIN_POSITIVE);
        let mut xyz = [0.0f64; 3];
        for (lobe, value) in xyz.iter_mut().enumerate() {
            let whole = integrate(VISIBLE_LO_NM, VISIBLE_HI_NM, |lambda| lobe_at(lambda, lobe));
            if whole <= 0.0 {
                continue;
            }
            let mut part = 0.0;
            for (index, channel) in self.channels.iter().enumerate() {
                let over = integrate(channel.lo_nm(), channel.hi_nm(), |lambda| lobe_at(lambda, lobe));
                part += bands[index] * over / whole;
            }
            *value = part / weight;
        }
        srgb(xyz)
    }

    /// The Sun's own colour, as the table's own check on itself: a 5 778 K body renders near the
    /// model's reference white.
    pub fn sun_response(&self) -> [f64; 3] {
        self.response(&self.sun_bands)
    }
}

/// The solver: one band table, read in one atmospheric state.
#[derive(Clone, Debug, PartialEq)]
pub struct Solver {
    spectra: Spectra,
    state: State,
}

impl Solver {
    /// A solver over a band table, in a state.
    pub fn new(spectra: Spectra, state: State) -> Self {
        Solver { spectra, state }
    }

    /// The table this solver reads.
    pub fn spectra(&self) -> &Spectra {
        &self.spectra
    }

    /// The state it reads it in.
    pub fn state(&self) -> &State {
        &self.state
    }

    /// A column's optical depth at `altitude_deg`, per band — air and droplets, each over its own
    /// geometry: `β_air · scale_height · m + β_drop · layer · m`.
    fn column_tau(&self, altitude_deg: f64) -> Bands {
        let m = air_mass(altitude_deg.max(0.0));
        let mut tau = [0.0f64; CHANNEL_COUNT];
        for (index, channel) in self.spectra.channels.iter().enumerate() {
            let lambda = channel.centre_nm();
            let air = self.spectra.air.rayleigh_per_m(lambda) + self.spectra.air.aerosol_per_m(lambda);
            tau[index] = air * self.spectra.air.scale_height_m * m;
            if let Some(droplets) = self.state.droplets {
                tau[index] += droplets.extinction_per_m() * droplets.layer_m * m;
            }
        }
        tau
    }

    /// The transmittance of the Sun's own column, per band.
    ///
    /// The air-mass formula leaves its domain below the horizon, and the horizon's own value takes
    /// over: the beam is not extinguished by a longer path than exists.
    pub fn sun_transmittance(&self) -> Bands {
        let tau = self.column_tau(self.state.sun_elevation_deg);
        let mut t = [0.0f64; CHANNEL_COUNT];
        for index in 0..CHANNEL_COUNT {
            t[index] = (-tau[index]).exp();
        }
        t
    }

    /// The direct beam, per band, in W/m² — and it is **exactly zero** the moment the planet occludes
    /// the body. Not "small": zero. A beam dimmed by an enormous factor would be a lie about where the
    /// light went.
    pub fn direct_beam(&self) -> Bands {
        if self.state.sun_elevation_deg < 0.0 {
            return [0.0; CHANNEL_COUNT];
        }
        let t = self.sun_transmittance();
        let mut beam = [0.0f64; CHANNEL_COUNT];
        for (index, channel) in self.spectra.sun_bands.iter().enumerate() {
            beam[index] = channel * t[index];
        }
        beam
    }

    /// The sky's scattered radiance, per band, in W/m²/sr, from a ray at `view_elevation_deg`.
    ///
    /// Below the horizon the scattered part is scaled by the reference's own curve and the night
    /// sky's own radiance is what the sky *is*: at −18° the scattered part has decayed to nothing and
    /// the floor is the measurement, which is why a night is dim and never an absence.
    pub fn sky_radiance(&self, view_elevation_deg: f64) -> Bands {
        let m_view = air_mass(view_elevation_deg.max(0.0));
        let m_sun = air_mass(self.state.sun_elevation_deg);
        let t_sun = self.sun_transmittance();
        let theta_deg = (view_elevation_deg - self.state.sun_elevation_deg).abs();
        let twilight = if self.state.sun_elevation_deg >= 0.0 {
            1.0
        } else {
            (TWILIGHT_DECAY_PER_DEG * self.state.sun_elevation_deg).exp()
        };

        let mut sky = [0.0f64; CHANNEL_COUNT];
        for (index, channel) in self.spectra.channels.iter().enumerate() {
            let lambda = channel.centre_nm();
            let air = self.spectra.air.rayleigh_per_m(lambda) + self.spectra.air.aerosol_per_m(lambda);
            let (beta_drop, drop_layer_path, drop_sun_path) = match self.state.droplets {
                None => (0.0, 0.0, 0.0),
                Some(droplets) => (
                    droplets.extinction_per_m(),
                    droplets.layer_m * m_view,
                    droplets.layer_m * m_sun,
                ),
            };
            let tau_air_view = air * self.spectra.air.scale_height_m * m_view;
            let tau_air_sun = air * self.spectra.air.scale_height_m * m_sun;
            let tau_drop_view = beta_drop * drop_layer_path;
            // The scattering share is read off the **column's own optical depths**, so a medium with
            // no path in this geometry contributes no share of it: no droplet layer is no path.
            let tau_view = tau_air_view + tau_drop_view;
            let scattering_share = (tau_air_view + tau_drop_view * DROPLET_SINGLE_SCATTER_ALBEDO)
                / tau_view.max(f64::MIN_POSITIVE);
            let phase = mixed_phase(theta_deg, tau_air_sun, beta_drop * drop_sun_path);
            let single_scatter = scattering_share * (1.0 - (-tau_view).exp());
            sky[index] = self.spectra.sun_bands[index] * single_scatter * phase * t_sun[index] * twilight
                + night_sky_radiance();
        }
        sky
    }

    /// The Moon's delivered light, per band, in W/m².
    ///
    /// **The Moon is a lit surface, and this is the arithmetic of it**: its own reflectance over π,
    /// the disc's own solid angle, the illuminated fraction, and the air on the way back down.
    pub fn moonlight(&self) -> Bands {
        let m = air_mass(self.state.moon_elevation_deg.max(0.0));
        let omega = self.spectra.moon_solid_angle_sr();
        let illuminated = self.state.moon_illuminated.clamp(0.0, 1.0);
        let mut light = [0.0f64; CHANNEL_COUNT];
        for (index, channel) in self.spectra.channels.iter().enumerate() {
            let lambda = channel.centre_nm();
            let air = self.spectra.air.rayleigh_per_m(lambda) + self.spectra.air.aerosol_per_m(lambda);
            let mut tau = air * self.spectra.air.scale_height_m * m;
            if let Some(droplets) = self.state.droplets {
                tau += droplets.extinction_per_m() * droplets.layer_m * m;
            }
            let radiance = self.spectra.sun_bands[index] * self.spectra.moon_reflectance[index]
                / std::f64::consts::PI;
            light[index] = radiance * omega * illuminated * (-tau).exp();
        }
        light
    }
}

/// The photopic conversion, said out loud so a reader sees it is a conversion and not a measurement:
/// `lx = W/m² · 683 lm/W`.
pub fn to_lux(w_m2: f64) -> f64 {
    w_m2 * LUMENS_PER_WATT_PHOTOPIC
}

/// The same conversion at the Moon's own lumens-per-watt — the figure the published full-moon
/// illuminance is quoted against.
pub fn to_moon_lux(w_m2: f64) -> f64 {
    w_m2 * MOON_LUMENS_PER_WATT
}

/// Planck's law, per nanometre of wavelength, in W/m²/sr/nm.
fn planck_per_nm(lambda_nm: f64, kelvin: f64) -> f64 {
    const H: f64 = 6.626_070_15e-34;
    const C: f64 = 299_792_458.0;
    const K: f64 = 1.380_649e-23;
    let lambda = lambda_nm * 1e-9;
    let numerator = 2.0 * H * C * C / lambda.powi(5);
    let denominator = (H * C / (lambda * K * kelvin)).exp() - 1.0;
    numerator / denominator * 1e-9
}

/// The geometric-grid quadrature: `λ = lo · (hi/lo)^t`, so `dλ = λ · ln(hi/lo) · dt` — the Jacobian is
/// the wavelength itself, which is what makes a band's colour an integral over the band.
fn integrate<F>(lo_nm: f64, hi_nm: f64, f: F) -> f64
where
    F: Fn(f64) -> f64,
{
    let log_ratio = (hi_nm / lo_nm).ln();
    let step = 1.0 / QUADRATURE_POINTS as f64;
    let mut total = 0.0;
    for point in 0..=QUADRATURE_POINTS {
        let lambda = lo_nm * (hi_nm / lo_nm).powf(point as f64 * step);
        let weight = if point == 0 || point == QUADRATURE_POINTS { 0.5 } else { 1.0 };
        total += weight * f(lambda) * lambda;
    }
    total * log_ratio * step
}

/// The published single-lobe fit's own shape: a Gaussian with a width on each side of its head
/// (Wyman, Sloan & Shirley 2013, JCGT — the fit's defining form).
fn lobe(lambda_nm: f64, sigma_lo: f64, sigma_hi: f64, head: f64, weight: f64) -> f64 {
    let sigma = if lambda_nm < head { sigma_lo } else { sigma_hi };
    weight * (-0.5 * ((lambda_nm - head) / sigma).powi(2)).exp()
}

/// One of the three published lobes, by index: the CIE 1931 colour-matching functions' single-lobe
/// fit (Wyman, Sloan & Shirley 2013). The same table is quoted in `docs/light-solver.md` §4.
fn lobe_at(lambda_nm: f64, index: usize) -> f64 {
    match index {
        0 => {
            lobe(lambda_nm, 37.9, 31.0, 599.8, 1.056)
                + lobe(lambda_nm, 16.0, 26.7, 442.0, 0.362)
                - lobe(lambda_nm, 20.4, 26.2, 501.1, 0.065)
        }
        1 => lobe(lambda_nm, 46.9, 40.5, 568.8, 0.821) + lobe(lambda_nm, 16.3, 31.1, 530.9, 0.286),
        _ => lobe(lambda_nm, 11.8, 36.0, 437.0, 1.217) + lobe(lambda_nm, 26.0, 13.8, 459.0, 0.681),
    }
}

/// The sRGB transform of a normalised XYZ triple.
///
/// The published matrix is balanced for a particular white, so equality in is not equality out: **each
/// row is divided by its own row sum**, the same calibration the record applies to the lobes one level
/// down. Without it a flat spectrum carries a cast, and a cast is a bias in the instrument rather than
/// a fact about the world.
fn srgb(xyz: [f64; 3]) -> [f64; 3] {
    const M: [[f64; 3]; 3] = [
        [3.240_454_2, -1.537_138_5, -0.498_531_4],
        [-0.969_266_0, 1.876_010_8, 0.041_556_0],
        [0.055_643_4, -0.204_025_9, 1.057_225_2],
    ];
    let mut out = [0.0f64; 3];
    for (row, line) in M.iter().enumerate() {
        let value = line[0] * xyz[0] + line[1] * xyz[1] + line[2] * xyz[2];
        out[row] = value / (line[0] + line[1] + line[2]);
    }
    out
}

/// Rayleigh's `3/(16π)·(1 + cos²θ)` and a Henyey–Greenstein at the droplets' own asymmetry, weighted
/// by the two terms' own column optical depths. The droplets' forward lobe is what makes a fog's sky
/// bright *around the Sun*, which a Rayleigh-only phase cannot produce.
fn mixed_phase(theta_deg: f64, tau_air: f64, tau_drop: f64) -> f64 {
    let cos_theta = theta_deg.to_radians().cos();
    let rayleigh = 3.0 / (16.0 * std::f64::consts::PI) * (1.0 + cos_theta * cos_theta);
    let g = DROPLET_ASYMMETRY_G;
    let hg = (1.0 - g * g)
        / (4.0 * std::f64::consts::PI * (1.0 + g * g - 2.0 * g * cos_theta).powf(1.5));
    let total = tau_air + tau_drop;
    if total <= 0.0 {
        return rayleigh;
    }
    (tau_air * rayleigh + tau_drop * hg) / total
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solver(state: State) -> Solver {
        Solver::new(Spectra::standard(), state)
    }

    fn total(bands: &Bands) -> f64 {
        bands.iter().sum()
    }

    #[test]
    fn the_air_mass_is_the_published_formula_and_the_records_own_horizon_floor() {
        assert!(
            (air_mass(90.0) - 1.0).abs() < 1e-3,
            "at the zenith the relative air mass is one, measured {}",
            air_mass(90.0)
        );
        // Kasten & Young's own horizon value is 37.92, which is what the record's declared
        // `GRAZING_AIR_MASS = 38` rounds.
        let horizon = air_mass(0.0);
        assert!((horizon - 37.92).abs() < 0.05, "the horizon's own column: {horizon}");
        assert!((horizon - 38.0).abs() < 0.2, "the record's own declared floor, measured");
        // Below the horizon the formula leaves its domain, and the horizon's own value is what
        // remains: a depressed Sun's light does not cross a longer path.
        assert_eq!(air_mass(-10.0), horizon);
    }

    #[test]
    fn a_wavelength_lands_in_exactly_one_band_and_outside_the_span_in_none() {
        let spectra = Spectra::standard();
        // Each band's own lower end is inside it, and its upper end belongs to the next band along:
        // the interval is half-open, so no wavelength has two answers.
        for index in 0..CHANNEL_COUNT - 1 {
            let lo = spectra.channels()[index].lo_nm();
            let hi = spectra.channels()[index].hi_nm();
            assert_eq!(spectra.band_of_nm(lo), Some(index), "the lower end {lo} nm");
            assert_eq!(spectra.band_of_nm(hi), Some(index + 1), "the shared edge {hi} nm");
        }
        let last = CHANNEL_COUNT - 1;
        assert_eq!(spectra.band_of_nm(spectra.channels()[last].lo_nm()), Some(last));
        // Outside the span there is no band at all: below the short end and above the long one the
        // answer is `None` rather than the nearest band, which is what lets a caller **refuse** a
        // wavelength this table cannot voice instead of voicing a neighbour.
        assert_eq!(spectra.band_of_nm(VISIBLE_LO_NM - 0.001), None);
        assert_eq!(spectra.band_of_nm(VISIBLE_HI_NM), None);
        assert_eq!(spectra.band_of_nm(f64::NAN), None);
    }

    #[test]
    fn the_twilight_is_the_references_own_curve_and_never_black() {
        assert_eq!(twilight_lux(0.0), TWILIGHT_LUX_AT_HORIZON);
        // The record's own reproduction of the reference's table: 13.35 at −4° (the table prints
        // "~15"), and 2.44 at −6° (the table prints "~2.5"; the record's own prose rounds it to 2.40).
        assert!((twilight_lux(-4.0) - 13.35).abs() < 0.01, "{}", twilight_lux(-4.0));
        assert!((twilight_lux(-6.0) - 2.44).abs() < 0.01, "{}", twilight_lux(-6.0));
        assert!(twilight_lux(-2.0) > 70.0 && twilight_lux(-2.0) < 80.0);
        assert!(twilight_lux(-12.0) < 0.02 && twilight_lux(-12.0) > 0.01);
        // The floor is what a night is, and the knee is where the curve meets it: −15.17°, before
        // astronomical twilight's own −18°.
        assert_eq!(twilight_lux(-18.0), NIGHT_SKY_ILLUMINANCE_LX);
        assert_eq!(twilight_lux(-60.0), NIGHT_SKY_ILLUMINANCE_LX);
        let knee = twilight_knee_deg();
        assert!((knee + 15.17).abs() < 0.02, "the knee: {knee}");
        // Below the knee the floor is what the sky is; above it the curve's own exponential is, and
        // it is only a hair above the floor there.
        assert_eq!(twilight_lux(knee - 0.01), NIGHT_SKY_ILLUMINANCE_LX);
        assert!(twilight_lux(knee + 0.01) > NIGHT_SKY_ILLUMINANCE_LX);
        // The curve's own slope is 0.85 per degree, so a hundredth of a degree above the knee is
        // already 0.85% of the floor: the bound is the slope's, not a guess.
        assert!(twilight_lux(knee + 0.01) < NIGHT_SKY_ILLUMINANCE_LX * 1.01);
    }

    #[test]
    fn the_night_skys_radiance_is_the_references_own_measured_figure() {
        let radiance = night_sky_radiance();
        assert!((radiance - 4.66e-7).abs() < 1e-8, "{radiance}");
        // Round trip: the same figure through the photopic conversion is the reference's 0.001 lx.
        assert!((to_lux(radiance) * std::f64::consts::PI - NIGHT_SKY_ILLUMINANCE_LX).abs() < 1e-9);
    }

    #[test]
    fn the_twelve_bands_sum_to_the_published_solar_constant() {
        let spectra = Spectra::standard();
        assert_eq!(spectra.channels().len(), CHANNEL_COUNT);
        assert_eq!(spectra.channels()[0].lo_nm(), VISIBLE_LO_NM);
        assert_eq!(spectra.channels()[CHANNEL_COUNT - 1].hi_nm(), VISIBLE_HI_NM);
        let sum = total(spectra.sun_bands());
        // **The calibration's own target, pinned as a literal.** The bands are scaled *to*
        // `SOLAR_CONSTANT_W_M2`, so a check against the constant itself would pass for any constant;
        // the published `1 361 W/m²` written here is what makes a moved constant a named failure.
        assert!((sum - 1_361.0).abs() < 1e-9, "the Sun's own total: {sum}");
        // The bands are nearly flat at 5 778 K — the peak of a 5 778 K body is inside the span, at
        // 501 nm — which is why the Sun's own colour comes out near white rather than blue.
        let first = spectra.sun_bands()[0];
        let last = spectra.sun_bands()[CHANNEL_COUNT - 1];
        assert!(first > 0.0 && last > 0.0);
    }

    #[test]
    fn the_response_makes_a_flat_spectrum_neutral_and_the_sun_near_white() {
        let spectra = Spectra::standard();
        let flat = [1.0f64; CHANNEL_COUNT];
        let neutral = spectra.response(&flat);
        // The residual is the quadrature's own seam error — the twelve bands' integrals are taken
        // separately and summed against the whole span's — and it is measured rather than assumed
        // away: the three channels differ here by about 4e-7.
        let spread = neutral.iter().copied().fold(f64::MIN, f64::max)
            - neutral.iter().copied().fold(f64::MAX, f64::min);
        assert!(spread < 1e-5, "a flat spectrum must be neutral in the instrument: {neutral:?}");
        assert!((neutral[0] - 1.0 / CHANNEL_COUNT as f64).abs() < 1e-5, "{neutral:?}");
        let sun = spectra.sun_response();
        let spread = sun.iter().copied().fold(f64::MIN, f64::max)
            - sun.iter().copied().fold(f64::MAX, f64::min);
        assert!(spread < 0.05, "the Sun is near the model's white: {sun:?} (spread {spread})");
        assert!(sun.iter().all(|channel| *channel > 0.0));
    }

    #[test]
    fn the_daylight_sky_is_blue_for_rayleighs_own_reason() {
        let solver = solver(State::clear(60.0, -20.0, 0.0));
        let blue = solver.spectra().air().rayleigh_per_m(450.0);
        let red = solver.spectra().air().rayleigh_per_m(675.0);
        let ratio = blue / red;
        assert!(
            (ratio - 5.25).abs() < 0.01,
            "the exponent is the whole reason: β(450)/β(675) = (675/450)^4.09 = {ratio}"
        );
        assert!(
            (solver.spectra().air().rayleigh_per_m(450.0) / RAYLEIGH_REFERENCE_PER_M - 2.272).abs()
                < 0.01,
            "and (550/450)^4.09 = 2.272 against the reference wavelength"
        );
        let sky = solver.sky_radiance(40.0);
        assert!(sky[0] > sky[CHANNEL_COUNT - 1], "the first band out-scatters the last");
        let colour = solver.spectra().response(&sky);
        assert!(
            colour[2] > colour[0],
            "the sky's own response is blue for the extinction's reason, not a taste: {colour:?}"
        );
        // And nobody picked it: the only wavelength-dependent term is λ^-4.09.
        assert!(blue > red);
    }

    #[test]
    fn the_beam_is_exactly_zero_below_the_horizon_and_positive_above_it() {
        let down = solver(State::clear(-0.1, -20.0, 0.0));
        assert_eq!(down.direct_beam(), [0.0; CHANNEL_COUNT], "not small: zero");
        let up = solver(State::clear(0.1, -20.0, 0.0));
        assert!(total(&up.direct_beam()) > 0.0);
        // The twilight sky, though, keeps falling rather than ending: the day:night ratio is finite.
        let night = solver(State::clear(-18.0, -20.0, 0.0));
        let day_beam = total(&up.direct_beam());
        let night_sky = total(&night.sky_radiance(40.0));
        assert!(night_sky > 0.0);
        assert!(day_beam / night_sky > 1.0e6, "a lit day is not a dimensionless infinity");
        // The night's sky is its own floor: at −18° the scattered part has decayed to a small share
        // of the floor, and what the solver reports *is* the night sky's own measured value.
        let floor_over_hemisphere = night_sky_radiance() * CHANNEL_COUNT as f64;
        println!(
            "U5 light: the sky at −18° reads {night_sky:.6e} against the floor's {floor_over_hemisphere:.6e} \
             (scattered share {:.4}%)",
            100.0 * (night_sky - floor_over_hemisphere) / floor_over_hemisphere
        );
        assert!(night_sky >= floor_over_hemisphere);
        assert!((night_sky - floor_over_hemisphere) < floor_over_hemisphere * 0.05);
    }

    #[test]
    fn more_droplets_put_more_light_into_the_sky_and_take_more_out_of_the_path() {
        let clear = solver(State::clear(40.0, -20.0, 0.0));
        let foggy_state = State {
            droplets: Some(Droplets::fog(FOG_THRESHOLD_LWC_G_M3)),
            ..State::clear(40.0, -20.0, 0.0)
        };
        let foggy = solver(foggy_state);
        let visibility = foggy_state.droplets.unwrap().visibility_km();
        assert!((visibility - 0.895).abs() < 0.01, "the threshold's own kilometre: {visibility}");
        assert!(
            foggy.spectra().air().angstrom_beta == 0.0,
            "the droplets' haze is not an invented aerosol column"
        );
        assert!(total(&foggy.direct_beam()) < total(&clear.direct_beam()));
        assert!(total(&foggy.sky_radiance(40.0)) > total(&clear.sky_radiance(40.0)));
        // And the fog's sky is bright *around* the Sun: the forward lobe at a small scattering angle
        // beats the same load at a wide one.
        let near_sun = total(&foggy.sky_radiance(35.0));
        let away = total(&foggy.sky_radiance(120.0));
        assert!(near_sun > away, "the aureole: {near_sun} vs {away}");
    }

    #[test]
    fn the_beams_droplets_are_their_own_layer_and_not_the_whole_column() {
        let state = |layer_m: f64| State {
            droplets: Some(Droplets::layer(FOG_THRESHOLD_LWC_G_M3, layer_m)),
            ..State::clear(30.0, -20.0, 0.0)
        };
        // No layer declared is no path: the beam is untouched, exactly.
        let none = solver(state(0.0));
        let clear = solver(State::clear(30.0, -20.0, 0.0));
        assert_eq!(none.direct_beam(), clear.direct_beam());
        assert_eq!(none.sky_radiance(40.0), clear.sky_radiance(40.0));
        // A deeper layer costs more.
        let thin = solver(state(40.0));
        let deep = solver(state(400.0));
        assert!(total(&deep.direct_beam()) < total(&thin.direct_beam()));
        // A higher Sun crosses less of it.
        let low = solver(State {
            droplets: Some(Droplets::fog(FOG_THRESHOLD_LWC_G_M3)),
            ..State::clear(5.0, -20.0, 0.0)
        });
        let high = solver(State {
            droplets: Some(Droplets::fog(FOG_THRESHOLD_LWC_G_M3)),
            ..State::clear(60.0, -20.0, 0.0)
        });
        let low_t = low.sun_transmittance();
        let high_t = high.sun_transmittance();
        assert!(high_t[0] > low_t[0], "a higher Sun crosses less of the layer");
    }

    #[test]
    fn the_moon_is_a_reflectance_and_not_an_emitter() {
        let full = solver(State::clear(20.0, 90.0, 1.0));
        let doubled = Solver::new(Spectra::standard().with_sun_factor(2.0), State::clear(20.0, 90.0, 1.0));
        // Doubling the Sun doubles the moonlight; an independent source would not move.
        let a = total(&full.moonlight());
        let b = total(&doubled.moonlight());
        assert!((b - 2.0 * a).abs() < 1e-12, "{a} → {b}");
        // A tenth-lit crescent is exactly a tenth of the disc.
        let crescent = solver(State::clear(20.0, 90.0, 0.1));
        assert!((total(&crescent.moonlight()) - a * 0.1).abs() < 1e-15);
        // An occluded Moon is exactly zero, the same rule the Sun's own beam obeys.
        let occluded = solver(State::clear(20.0, 90.0, 0.0));
        assert_eq!(occluded.moonlight(), [0.0; CHANNEL_COUNT]);
        let below = solver(State::clear(20.0, -40.0, 1.0));
        assert!(
            total(&below.moonlight()) < a,
            "the Moon's own air on the way back down is charged, not skipped"
        );
    }

    #[test]
    fn the_full_moons_illuminance_lands_where_the_published_figures_put_it() {
        let full = solver(State::clear(0.0, 90.0, 1.0));
        let w_m2 = total(&full.moonlight());
        // The record's own measurement on its twelve-band table is 3.0484e-3 W/m²; this build's own
        // table is its own, and lands inside the same published zone rather than fitted to the digit.
        assert!((2.5e-3..3.5e-3).contains(&w_m2), "the full Moon's own irradiance: {w_m2}");
        let photopic = to_lux(w_m2);
        let lunar = to_moon_lux(w_m2);
        assert!((photopic - 2.08).abs() < 0.5, "at 683 lm/W: {photopic}");
        assert!((lunar - 0.3).abs() < 0.1, "at the Moon's own ≈100 lm/W: {lunar} lx");
        assert!(lunar < photopic, "the 683 lm/W figure is a warning beside the measurement");
        // The Sun over the Moon: six orders, as the record pins it.
        let sun = total(full.spectra().sun_bands());
        let ratio = sun / w_m2;
        assert!((2.0e5..1.0e6).contains(&ratio), "sun ÷ moon = {ratio}");
        // And the Moon's own spectral shape is redder than the Sun's: its reflectance rises.
        let reflectance = full.spectra().moon_reflectance();
        assert!(reflectance[CHANNEL_COUNT - 1] > reflectance[0]);
    }

    #[test]
    fn a_hazier_column_is_redder_and_dimmer_by_the_references_own_turbidity() {
        let clear = Solver::new(Spectra::standard(), State::clear(30.0, -20.0, 0.0));
        let hazy = Solver::new(Spectra::standard().with_air(Air::hazy(0.2)), State::clear(30.0, -20.0, 0.0));
        assert!(total(&hazy.direct_beam()) < total(&clear.direct_beam()));
        let ratio = |solver: &Solver| {
            let t = solver.sun_transmittance();
            t[CHANNEL_COUNT - 1] / t[0]
        };
        assert!(ratio(&hazy) > ratio(&clear), "aerosol loading reddens what crosses it");
        // The aerosol term is Ångström's own: β · (λ/1 µm)^-α at the two ends.
        let air = Air::hazy(0.2);
        assert!((air.aerosol_optical_depth(1000.0) - 0.2).abs() < 1e-12);
        assert!(air.aerosol_optical_depth(400.0) > air.aerosol_optical_depth(700.0));
    }
}
