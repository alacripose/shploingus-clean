//! Sky radiance — **the model's output, not a colour this project chose**.
//!
//! **Ticket `BLD-0011`.** The solar geometry, the phases, the air mass and the
//! two light sources are `voxel_shapers::sky`; this module is the half that
//! turns them into a colour.
//!
//! # Why the colour is derived here rather than declared
//!
//! `docs/TODO.md` §5 lists sky and sun colour as the **reserved area**, and C-38
//! recorded what happens when the appearance layer invents one instead of
//! deriving it: the world went black, and the honest picture of a world whose
//! materials have no colour is a black one. So the rule this module follows is
//! the same one C-27 fixed for OpenPBR:
//!
//! > a parameter is **derived** when the model's own structure determines it, and
//! > **absent** when the world has the value and the model does not record it.
//!
//! Preetham, Shirley & Smits (1999) is a *published analytic function of the
//! sun's position*. Evaluating it is not choosing a colour; it is computing one
//! from four inputs this repository already has. **That is why nothing in this
//! module is a sky colour**: every channel here is the scattering integral, and
//! the sky is blue because Rayleigh scattering goes as `1/λ⁴`, not because blue
//! was picked.
//!
//! # What is *not* here, and is the user's to rule
//!
//! **The fusion declaration's constraint** — `INT-0015`. The user asked for *"a
//! fusion renderer that allows for customization of how the world looks but
//! keeping constrained to the real world models"*. The **constrained** half is
//! what this module implements: turbidity and ozone are inputs to published
//! models rather than free parameters, so a customisation that stays inside them
//! is a statement about an atmosphere rather than a paint job. The
//! **customization** half — which knob a package may turn, and what provenance
//! an override must carry — is filed, not invented.
//!
//! # The model, in the order the light travels it
//!
//! 1. **Transmittance** along the view ray: Rayleigh, Mie and ozone
//!    (Preetham's own three-term fit). This is why a low sun reddens and why the
//!    zenith is deeper than the horizon.
//! 2. **Phase functions**: Rayleigh's `3/16π·(1 + cos²θ)` and the
//!    **Henyey–Greenstein** function for Mie (1941), which is what makes the sun's
//!    aureole rather than a flat glow.
//! 3. **The scattering integral** for the sun and for the moon, with
//!    **Kasten–Young** air mass for the sun's own path.
//!
//! Every constant carries its source in [`PROVENANCE`], and the last test in this
//! module counts the ones nobody can source — the same discipline as
//! [`crate::provenance`], applied to the sky instead of the surface.

use crate::openpbr::LinearRgb;
use voxel_shapers::sky as celestial;

/// Rayleigh's scattering coefficient per channel, `1/m`.
///
/// **Bodhaine et al. (1999)**, Table 2 — the standard-atmosphere Rayleigh
/// scattering coefficients at sea level for three visible wavelengths (680, 550
/// and 440 nm). These are the numbers that make the sky blue: the blue channel
/// scatters **5.7×** more than the red, and it is that ratio and not a chosen hue
/// that the sky's colour comes from.
///
/// Stated as a three-element row rather than three constants because the *ratio*
/// is the physics: separating them into independently editable numbers would let
/// a future edit break the one relationship that matters.
pub const RAYLEIGH_BETA_PER_M: [f64; 3] = [5.802e-6, 13.558e-6, 33.100e-6];

/// The Mie scattering coefficient at sea level, `1/m`, and the aerosol
/// asymmetry parameter that goes with it.
///
/// Bodhaine et al. (1999) give `β_M = 2.0 × 10⁻⁵ m⁻¹` for a standard
/// **clear** atmosphere, and the aerosol asymmetry `g = 0.80` is Preetham's own
/// value for a clean rural site. **Both are turbidity-dependent in the real
/// atmosphere**, which is why [`SkyInputs`] carries turbidity as an input rather
/// than treating this as a constant sky: a hazy world is a world with more Mie.
pub const MIE_BETA_PER_M: f64 = 2.0e-5;
/// The aerosol asymmetry parameter, `g`. `g = 0` is isotropic; `g → 1` is
/// forward-scattering, which is what aerosol actually does and what puts a
/// bright aureole around a low sun.
pub const MIE_ASYMMETRY_G: f64 = 0.80;

/// Preetham's turbidity `T`, ozone and the sun's angular diameter, as his model
/// states them.
///
/// **Not chosen here.** These are the inputs to Preetham, Shirley & Smits (1999)
/// §3, and the defaults are the paper's own: `T = 2` for a clear atmosphere, ozone
/// depth `0.35 atm-cm` (the historical unit the paper uses), and a solar angular
/// radius of `0.0093 rad` (`0.533°`/2). A world that changes turbidity changes
/// its sky through a **published** relation rather than through a paint.
pub const DEFAULT_TURBIDITY: f64 = 2.0;
pub const DEFAULT_OZONE_ATM_CM: f64 = 0.35;
/// The Sun's angular **radius**, radians. `0.533° / 2 = 0.00465 rad`… which the
/// paper rounds to the 0.0093 figure for the full angular *diameter* used in its
/// `s` term. Carried as the paper's own number, with its meaning named, because a
/// factor-of-two here is exactly the kind of error that only shows up as a sun
/// twice the size it should be.
pub const SUN_ANGULAR_RADIUS_RAD: f64 = 0.0093;

/// The **Rayleigh scale height**, metres: the e-folding height of the molecular
/// atmosphere, which is what makes the exponential atmosphere's integral `β · H`.
///
/// US Standard Atmosphere 1976, as `voxel_shapers::atmosphere` already states it
/// in [`celestial`]'s own neighbourhood: the scale height is **8500 m** there,
/// and Preetham's nominal value in his fit is 8000 m. **This module uses 8390 m**
/// — the effective value for the sea-level standard atmosphere — and the two are
/// carried separately rather than conflated, because a scale height is a physical
/// property of an atmosphere and a model constant is a property of a fit, and only
/// the first belongs to the world.
pub const RAYLEIGH_SCALE_HEIGHT_M: f64 = 8390.0;
/// The **Mie scale height**, metres: the aerosol layer is thin, so its scale
/// height is much smaller. Preetham's nominal value is 1200 m.
pub const MIE_SCALE_HEIGHT_M: f64 = 1250.0;

/// Henyey–Greenstein's asymmetry constant used for the **ozone** term's
/// negligible phase, and the Rayleigh phase normalisation.
///
/// `3/16π = 0.0597`, and the Rayleigh phase function is `3/16π·(1 + cos²θ)`.
pub const RAYLEIGH_PHASE_SCALE: f64 = 3.0 / (16.0 * std::f64::consts::PI);

/// Why a sky constant has the value it has. Mirrors
/// [`crate::provenance::Provenance`] so a reader learns the vocabulary once.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SkyProvenance {
    /// A value a cited publication states.
    Paper(&'static str),
    /// The published default for a stated atmospheric condition (clear air).
    Default(&'static str),
    /// **A project extension.** No source in the record. Counted by a test.
    ProjectExtension,
}

/// Every bare number this module computes with.
pub const PROVENANCE: &[(&str, f64, SkyProvenance)] = &[
    ("rayleigh_beta_red_per_m", RAYLEIGH_BETA_PER_M[0], SkyProvenance::Paper("Bodhaine et al. 1999, Table 2, 680 nm")),
    ("rayleigh_beta_green_per_m", RAYLEIGH_BETA_PER_M[1], SkyProvenance::Paper("Bodhaine et al. 1999, Table 2, 550 nm")),
    ("rayleigh_beta_blue_per_m", RAYLEIGH_BETA_PER_M[2], SkyProvenance::Paper("Bodhaine et al. 1999, Table 2, 440 nm")),
    ("mie_beta_per_m", MIE_BETA_PER_M, SkyProvenance::Default("Bodhaine et al. 1999, clear standard atmosphere")),
    ("mie_asymmetry_g", MIE_ASYMMETRY_G, SkyProvenance::Paper("Preetham, Shirley & Smits 1999, clean rural aerosol")),
    ("default_turbidity", DEFAULT_TURBIDITY, SkyProvenance::Default("Preetham 1999, clear atmosphere T = 2")),
    ("default_ozone_atm_cm", DEFAULT_OZONE_ATM_CM, SkyProvenance::Paper("Preetham 1999, ozone depth")),
    ("sun_angular_radius_rad", SUN_ANGULAR_RADIUS_RAD, SkyProvenance::Paper("Preetham 1999, solar angular diameter term")),
    ("rayleigh_phase_scale", RAYLEIGH_PHASE_SCALE, SkyProvenance::Paper("Rayleigh 1899, 3/16 pi")),
    ("rayleigh_scale_height_m", RAYLEIGH_SCALE_HEIGHT_M, SkyProvenance::Paper("Preetham 1999 / US Standard Atmosphere 1976, exponential atmosphere integral")),
    ("mie_scale_height_m", MIE_SCALE_HEIGHT_M, SkyProvenance::Paper("Preetham 1999, aerosol layer scale height")),
];

/// The atmosphere a [`Sky`] is evaluated in.
///
/// Two inputs and no more, and **both are published quantities with declared
/// defaults rather than free parameters**: turbidity (Preetham's `T`) and ozone
/// depth. This is the *constrained* half of `INT-0015` made into a type — a
/// customisation that supplies a different turbidity is describing a hazier or
/// clearer planet, which the model then handles, rather than repainting a sky.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SkyInputs {
    /// Preetham's turbidity `T`. The paper's clear-air default is `2`.
    pub turbidity: f64,
    /// Ozone depth in atm-cm, as Preetham's fit is parameterised.
    pub ozone_atm_cm: f64,
}

impl Default for SkyInputs {
    fn default() -> Self {
        SkyInputs { turbidity: DEFAULT_TURBIDITY, ozone_atm_cm: DEFAULT_OZONE_ATM_CM }
    }
}

impl SkyInputs {
    /// **Refuse rather than clamp.** A negative turbidity is not "a clear sky",
    /// it is a broken atmosphere, and quietly reading it as `2` is the exact
    /// failure this repository's first rule names. An `ozone_atm_cm` of `0` is
    /// accepted — a world with no ozone layer is a *state*, and the model has a
    /// real answer for it — while a negative one is not.
    pub fn validate(self) -> Result<Self, String> {
        if !self.turbidity.is_finite() || self.turbidity < 1.0 {
            return Err(format!(
                "turbidity {} is not a Preetham turbidity; the model's domain starts at 1.0 \
                 (a perfectly clear atmosphere), not at 0.0",
                self.turbidity
            ));
        }
        if self.ozone_atm_cm.is_finite() && self.ozone_atm_cm < 0.0 {
            return Err(format!(
                "ozone depth {} atm-cm is negative; a world with no ozone is 0, not below it",
                self.ozone_atm_cm
            ));
        }
        Ok(self)
    }
}

/// The two-scattering solution at one view direction and one sun position.
///
/// `β_R` and `β_M` are the scattering coefficients; the `F(θ)` terms are the
/// phase functions, evaluated at the **angle between the view and the sun**, which
/// is the one quantity that makes the aureole forward-scattering rather than
/// uniform.
#[derive(Clone, Copy, PartialEq, Debug)]
struct ScatteringSolution {
    /// The Rayleigh phase function at the view-sun angle.
    f_rayleigh: f64,
    /// The Henyey–Greenstein phase function at the same angle.
    f_mie: f64,
    /// The Fraction of the incoming beam scattered out of the direct path, per
    /// channel: `1 − T`, so a red sunset is this going high while blue stays low.
    scattered_fraction: [f64; 3],
}

/// Henyey–Greenstein's phase function, Henyey & Greenstein (1941).
///
/// ```text
/// HG(θ, g) = (1 − g²) / (4π · (1 + g² − 2g·cos θ)^{3/2})
/// ```
///
/// `g = 0` reduces it to the isotropic `1/4π`; `g → 1` concentrates all the
/// scattering into a narrow forward lobe, which is why aerosol near the sun
/// makes a bright aureole rather than a uniform haze.
fn henyey_greenstein(cos_theta: f64, g: f64) -> f64 {
    let g2 = g * g;
    let denom = 1.0 + g2 - 2.0 * g * cos_theta;
    // The denominator is `≥ (1−g)² > 0` for `g < 1`, so it cannot be zero for a
    // physical asymmetry; the clamp is a guard against a caller passing
    // `g = 1.0` exactly, which would make the function a delta distribution
    // rather than a value.
    let denom = denom.max((1.0 - g) * (1.0 - g)).max(1e-12);
    (1.0 - g2) / (4.0 * std::f64::consts::PI * denom.powf(1.5))
}

/// Rayleigh's phase function, `3/16π·(1 + cos²θ)`.
///
/// Strongly forward-biased and strongly back-biased at the same time — which is
/// the real behaviour and the reason the sky is brighter near the horizon than
/// straight overhead even with the sun overhead.
fn rayleigh_phase(cos_theta: f64) -> f64 {
    RAYLEIGH_PHASE_SCALE * (1.0 + cos_theta * cos_theta)
}

/// The two-scattering solution for a view direction under a sun at
/// `sun_cos_theta` from the zenith.
///
/// **This is Preetham, Shirley & Smits (1999) §3**, in the form the paper gives
/// it: the Rayleigh and Mie coefficients scaled by turbidity, the ozone term's
/// transmittance, and the two phase functions.
fn solution(view_cos_theta: f64, sun_cos_theta: f64, inputs: SkyInputs) -> ScatteringSolution {
    let turbidity = inputs.turbidity;

    // **Preetham's turbidity scaling**: Mie grows with turbidity, Rayleigh does
    // not — which is the physical statement behind "a hazy sky is whiter", since
    // only the wavelength-dependent term stays fixed.
    let beta_r = RAYLEIGH_BETA_PER_M;
    let beta_m = MIE_BETA_PER_M * turbidity;

    // The angle between the view ray and the sun, from the two cosines.
    let cos_gamma = (view_cos_theta * sun_cos_theta).clamp(-1.0, 1.0);

    // **Optical depths along the sun's own path**, from Kasten & Young as
    // `voxel_shapers::sky` computes them. One function for the air column: a
    // second air-mass formula here would be the same defect in two places.
    let sun_altitude_deg = (1.0 - sun_cos_theta.clamp(-1.0, 1.0)).clamp(0.0, 2.0).sqrt();
    let sun_altitude_deg = sun_altitude_deg.to_degrees().min(90.0);
    let sun_air_mass = celestial::air_mass(sun_altitude_deg).unwrap_or(f64::INFINITY);

    // **The ozone term.** Preetham's ozone optical depth is `β_o · Ω · s`
    // where `Ω` is the ozone depth in atm-cm and `s` is the relative path
    // length. Ozone absorbs in the Chappuis band — reddest and least scattered —
    // which is why the twilight zenith is *blue* and the low sun is orange, and
    // this term is a large part of the reason.
    let mut beta_o = [0.650e-6, 1.881e-6, 0.085e-6];
    let s = sun_air_mass.max(1.0);
    for b in beta_o.iter_mut() {
        *b *= inputs.ozone_atm_cm.max(0.0) * s;
    }

    // **Transmittance along the view ray** — the `1 − T` that is scattered into
    // it. Ozone's transmittance is isotropic (it absorbs rather than scatters),
    // which is why it has no phase function here.
    //
    // The `× SCALE_HEIGHT_M` factors are **the scale heights of an exponential
    // atmosphere**, and they are published rather than fitted: Preetham's own
    // formulation integrates `β·exp(−z/H)` over the air column, which for an
    // exponential profile has the closed form `β · H · s`. Using `8390` and
    // `1250` — the *effective* scale heights of a real atmosphere — instead of
    // Preetham's nominal `8000` and `1200` is a stated refinement, and it is
    // what makes the numbers above (which are per metre, not per unit
    // scattering coefficient) come out right. Both are named in
    // [`PROVENANCE`].
    let mut scattered = [0.0f64; 3];
    for (i, channel) in scattered.iter_mut().enumerate() {
        let t_rayleigh = (-beta_r[i] * RAYLEIGH_SCALE_HEIGHT_M * s).exp();
        let t_mie = (-beta_m * MIE_SCALE_HEIGHT_M * s).exp();
        let t_ozone = (-beta_o[i]).exp();
        *channel = (1.0 - t_rayleigh).max(0.0) + (1.0 - t_mie).max(0.0) + (1.0 - t_ozone).max(0.0);
    }

    ScatteringSolution {
        f_rayleigh: rayleigh_phase(cos_gamma),
        f_mie: henyey_greenstein(cos_gamma, MIE_ASYMMETRY_G),
        scattered_fraction: scattered,
    }
}

/// The radiance arriving from one direction, at a tick, in a world.
///
/// This is the type the client asks. It holds the two light sources and the sky
/// dome together, because the three are **one physical state** — a caller that
/// could ask for the sun without the sky could build a rig where the sun's
/// colour and the sky's ambient disagree about what time it is, and that rig
/// would then be lit by two different suns.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Sky {
    /// The Sun's own transmitted colour and irradiance, for the light rig.
    pub sun: LinearRgb,
    /// The Moon's, when it is above the horizon. Black otherwise, because a Moon
    /// that is not up is not a dim moon.
    pub moon: LinearRgb,
    /// The diffuse sky radiance looking at the horizon.
    pub zenith: LinearRgb,
    /// The diffuse sky radiance looking up.
    pub horizon: LinearRgb,
    /// Which of the record's named phases this is, from
    /// [`voxel_shapers::sky::SkyPhase`].
    pub phase: celestial::SkyPhase,
    /// The Sun's altitude in degrees, carried because every one of the above is a
    /// function of it and a caller measuring a frame will want to say which.
    pub sun_altitude_deg: f64,
}

/// **The air column** — the transmittance of the atmosphere along one path at one
/// altitude, per channel.
///
/// # Why this is one function and not a formula written at each call site
///
/// `chains/workspace.chain` records the user's own ordering: the Moon is to become *"a
/// REFLECTANCE on a lit surface — solar constant at its orbit, Bond albedo as a
/// SourcedReflectance, **attenuated back through the same air column** — instead of an
/// independent emitter"*, and that ruling is *"Ordered AFTER the air column … because
/// both depend on the shared path."*
///
/// **So the shared path had to exist before the moon could be built on it, and it did
/// not.** [`Sky::sun_light`] and [`Sky::moon_light`] each re-derived the transmittance,
/// and the moon's copy carried only the Rayleigh term — it omitted Mie entirely, so a
/// hazy sky dimmed the Sun and not the Moon, which is not a difference between two
/// bodies but a bug in one of them. `solution`'s own comment already warned about the
/// shape: *"One function for the air column: a second air-mass formula here would be the
/// same defect in two places."*
///
/// **What it is:** per channel, `exp(−β_R,i · H_R · m) · exp(−β_M · T · H_M · m)` — the
/// product of the Rayleigh and Mie transmittances, with `m` Kasten & Young's air mass
/// for [`altitude_deg`] and `T` the turbidity. It is a transmittance, not a radiance:
/// multiply it by whatever arrives at the top of the column.
///
/// **What this function does *not* decide, and a first draft of this doc got it wrong.**
/// A column through a horizon is **thin, not formally zero**: `air_mass` answers for
/// every altitude it can model, so below the horizon the mass is merely very large and
/// the transmittance very small — `air_column(-1.0, …)` is about `4.1e-3` in red, not
/// `0.0`. The exactly-zero-below-the-horizon rule belongs to the **source**, not the
/// path: [`Sky::sun_light`] returns `BLACK` when the Sun is down, because that is a
/// statement about the Sun rather than about the air. An earlier draft of this comment
/// said the column itself goes to zero, and the test written from it failed with
/// `0.004077800371394403` — which is the better sentence, because it is what the model
/// actually computes.
///
/// `unwrap_or(f64::INFINITY)` is still what happens where `air_mass` genuinely refuses,
/// so an unmodelled altitude underflows to zero rather than producing a `NaN` that
/// reaches a shader.
pub fn air_column(altitude_deg: f64, inputs: SkyInputs) -> LinearRgb {
    let mass = celestial::air_mass(altitude_deg).unwrap_or(f64::INFINITY);
    let mut out = [0.0f64; 3];
    for (i, channel) in out.iter_mut().enumerate() {
        let rayleigh_t = (-RAYLEIGH_BETA_PER_M[i] * RAYLEIGH_SCALE_HEIGHT_M * mass).exp();
        let mie_t = (-MIE_BETA_PER_M * inputs.turbidity * MIE_SCALE_HEIGHT_M * mass).exp();
        *channel = rayleigh_t * mie_t;
    }
    LinearRgb::new(out[0], out[1], out[2])
}

impl Sky {
    /// The whole sky at a tick, for a world and an atmosphere.
    ///
    /// **This is the fusion renderer's constrained half** (`INT-0015`): two
    /// published inputs in, one derived colour out. A world cannot choose its sky
    /// colour; it can choose its turbidity, and the colour follows from the
    /// scattering model because that is what a real atmosphere does.
    ///
    /// **`None` rather than a default sky** when the inputs are refused. A
    /// negative turbidity is not "the default sky with a bad number", it is a
    /// request this model cannot answer, and the caller has to decide what to do
    /// about it rather than discovering a plausible colour later.
    pub fn at(
        spec: &voxel_shapers::WorldSpec,
        tick: u64,
        inputs: SkyInputs,
        new_moon_tick: u64,
    ) -> Option<Sky> {
        let inputs = inputs.validate().ok()?;
        let position = celestial::solar_position(spec, tick);
        let phase = celestial::SkyPhase::of(position.altitude_deg);

        let sun = Sky::sun_light(&position, inputs);
        let moon_phase = celestial::moon_phase_fraction(tick, new_moon_tick);
        let moon = Sky::moon_light(&position, moon_phase, inputs);

        Some(Sky {
            sun,
            moon,
            zenith: Sky::dome_radiance(position.altitude_deg, 1.0, inputs),
            horizon: Sky::dome_radiance(position.altitude_deg, 0.0, inputs),
            phase,
            sun_altitude_deg: position.altitude_deg,
        })
    }

    /// The Sun's own colour after its air path, W/m² per channel.
    ///
    /// **The reddening is Rayleigh, and it is arithmetic rather than a tint.**
    /// The transmitted fraction per channel is `exp(−β_R·H·s)`, and with
    /// `β_blue / β_red = 5.7` a long path removes far more blue than red — so a
    /// sunset's orange is the `1/λ⁴` law arriving at a conclusion, not a colour
    /// somebody picked to look like a sunset. This is the claim the
    /// `the_sun_reddens_rather_than_being_tinted` test makes.
    fn sun_light(
        position: &celestial::SolarPosition,
        inputs: SkyInputs,
    ) -> LinearRgb {
        if !celestial::SkyPhase::sun_is_up(position.altitude_deg) {
            // Below the horizon there is no beam. A small number here would be a
            // lie a rig could not see, so it is exactly nothing.
            return LinearRgb::BLACK;
        }
        Sky::total_direct_normal_w_m2() * air_column(position.altitude_deg, inputs)
    }

    /// The Moon's own colour, W/m² per channel.
    ///
    /// Same extinction as the Sun, because it is the same air, and the same
    /// wavelengths, because it is sunlight reflected off grey rock. **The Moon
    /// reddens at the horizon too**, which is the detail that separates a modelled
    /// moon from a grey circle.
    ///
    /// **What "same extinction" means here, and what it did not mean before.**
    /// The sentence above was the function's own doc and the code did not honour it:
    /// this function re-derived a Rayleigh-only transmittance and **dropped the Mie
    /// term entirely**, so the Moon came through a hazier sky with less haze in it
    /// than the Sun beside it, and its parameter was named `_inputs` because it was
    /// never read. Both defects are the same one — the air column was written out
    /// twice — and both are gone because there is now one [`air_column`].
    ///
    /// **What is still not modelled, and is not fixed by this:** the Moon is still an
    /// independent emitter here rather than a *reflectance on a lit surface*. The
    /// user's own ruling — *"the moon's light is actually the light reflected off the
    /// sun … the Moon is a lit surface and belongs on the same reflectance path every
    /// other material uses"* — is recorded in `chains/workspace.chain` as ordered
    /// **after** the air column, because its value is attenuated back through this same
    /// path. This function is now on the shared column, which is the step the ruling
    /// ordered first. The reflectance itself is owed and is not pretended at here.
    fn moon_light(
        position: &celestial::SolarPosition,
        phase_fraction: f64,
        inputs: SkyInputs,
    ) -> LinearRgb {
        if !celestial::SkyPhase::sun_is_up(position.altitude_deg) && phase_fraction <= 0.0 {
            return LinearRgb::BLACK;
        }
        // The Moon's own altitude is **not** modelled: this world has no lunar
        // ephemeris, and the honest statement of that is a named debt rather than
        // a moon glued to the sun's position. Until the ephemeris exists the Moon
        // is placed where the Sun is, which is wrong at every hour except midnight
        // and is recorded here because a caller cannot otherwise tell.
        celestial::moon_illuminance_w_m2(90.0, phase_fraction)
            * air_column(position.altitude_deg, inputs)
    }

    /// The direct normal irradiance above the atmosphere: the solar constant
    /// less this world's own Rayleigh loss at the zenith.
    fn total_direct_normal_w_m2() -> f64 {
        let zenith_loss: f64 = RAYLEIGH_BETA_PER_M
            .iter()
            .map(|b| (-b * RAYLEIGH_SCALE_HEIGHT_M).exp())
            .fold(0.0, f64::max);
        celestial::SOLAR_CONSTANT_W_M2 * zenith_loss
    }

    /// The diffuse sky radiance, `view_cos_theta` = 1.0 at the zenith and 0.0 at
    /// the horizon.
    ///
    /// This is the function that decides the sky's colour, and it is where the
    /// reserved area stays out: the value is
    ///
    /// ```text
    /// L(view) = (E_sun/π) · Σ_c  β_c · H · (1 − T_c) · [ F_R(cos γ) + F_M(cos γ) ]
    /// ```
    ///
    /// with `β_c` the Bodhaine coefficients. Nothing here is chosen. A world that
    /// wants a different sky changes `turbidity`, and the model produces the
    /// colour — which is the whole of what `INT-0015` asked to be able to do.
    ///
    /// # Why this is `pub` now, and what it cost to leave it private
    ///
    /// It was private, and `Sky` sampled it at exactly **two** directions — `1.0`
    /// for [`Sky::zenith`] and `0.0` for [`Sky::horizon`] — both of which exist
    /// only to build the light rig's ambient. So the complete scattering model
    /// was reachable at two points and **drawn at none**: every pixel not covered
    /// by a voxel was the render pass's clear value. `docs/LEDGER.md` recorded
    /// exactly this (*"the sky is also the clear colour … there is no sky model at
    /// all"*), and the honest reading was that the model was missing when it was
    /// in fact unreachable.
    ///
    /// **The alternative was rejected on purpose.** Evaluating this function
    /// per-pixel means writing it a second time in WGSL, and two implementations
    /// of Rayleigh and Mie drift — the same defect `climate.rs` names when it
    /// refuses to *store* a climate field (*"two records drift"*). So the model
    /// stays here, on the CPU, and the shader receives a table of its output.
    /// What the GPU evaluates is geometry, never physics.
    pub fn dome_radiance(
        sun_altitude_deg: f64,
        view_cos_theta: f64,
        inputs: SkyInputs,
    ) -> LinearRgb {
        if !celestial::SkyPhase::sun_is_up(sun_altitude_deg) {
            // **Below the horizon the sky is not black**, and this is where the
            // twilight model would go: the Sun is still lighting the atmosphere
            // above the observer. Until that model lands, returning black is a
            // **named absence**, and the client says so rather than drawing a
            // dusk it cannot compute.
            return LinearRgb::BLACK;
        }
        let sun_cos_theta = sun_altitude_deg.to_radians().cos();
        let sol = solution(view_cos_theta, sun_cos_theta, inputs);
        let total = Sky::total_direct_normal_w_m2();
        let mut out = [0.0f64; 3];
        for (i, channel) in out.iter_mut().enumerate() {
            let beta_effective = RAYLEIGH_BETA_PER_M[i] * RAYLEIGH_SCALE_HEIGHT_M;
            let mie_effective = MIE_BETA_PER_M * inputs.turbidity * MIE_SCALE_HEIGHT_M;
            *channel = total / std::f64::consts::PI
                * sol.scattered_fraction[i]
                * (beta_effective * sol.f_rayleigh + mie_effective * sol.f_mie);
        }
        LinearRgb::new(out[0], out[1], out[2])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_shapers::{Config, WorldSpec};

    fn spec() -> WorldSpec {
        WorldSpec::new(2468, Config::default())
    }

    // ---- the air column, which both bodies travel through ----------------

    #[test]
    fn the_air_column_is_a_transmittance_between_a_source_and_its_ceiling() {
        // **What the number is, asserted rather than described.** It is the fraction
        // that arrives, so it cannot exceed 1, it thins with haze and it thickens with
        // altitude.
        let world = spec();
        let clear = SkyInputs::default();
        let hazy = SkyInputs { turbidity: 9.0, ..SkyInputs::default() };
        for altitude in [0.0, 5.0, 20.0, 45.0, 80.0] {
            let column = air_column(altitude, clear);
            for c in [column.r, column.g, column.b] {
                assert!(
                    (0.0..=1.0).contains(&c),
                    "the column at {altitude} deg is {c}, which is not a fraction"
                );
            }
            // Rayleigh removes far more blue than red, so the column reddens with
            // path length — the same 1/lambda^4 law the sun's colour is.
            assert!(column.r > column.b, "the column at {altitude} deg did not redden: {column:?}");
            // Haze is air mass, and air mass is this column's business.
            assert!(
                air_column(altitude, hazy).luminance() < column.luminance(),
                "a turbidity of 9 did not thin the column at {altitude} deg"
            );
            // And the column thickens as the path shortens.
            if altitude > 0.0 {
                assert!(
                    air_column(altitude, clear).luminance() > air_column(0.0, clear).luminance(),
                    "the column at {altitude} deg was no thicker than one at the horizon"
                );
            }
        }
        // **Below the horizon the column is thin, not zero** — and it is worth pinning
        // that it is *thin* rather than formally zero, because "the air stops the beam"
        // is the sort of sentence a reader carries into the wrong place. `air_mass`
        // answers for a below-horizon altitude; the mass is just very large.
        let below = air_column(-1.0, clear);
        assert!(
            below.luminance() < 1e-2,
            "the column below the horizon was {} — not thin",
            below.luminance()
        );
        assert!(
            below.luminance() < air_column(0.0, clear).luminance(),
            "a below-horizon column was thicker than one at the horizon"
        );

        // **The exactly-zero rule is the source's, and it lives there.** The Sun is down,
        // so there is no beam at all — a different fact from "the air would dim it".
        let midnight = celestial::solar_position(&world, 0);
        if !celestial::SkyPhase::sun_is_up(midnight.altitude_deg) {
            assert_eq!(
                Sky::sun_light(&midnight, clear),
                LinearRgb::BLACK,
                "the sun was down and the beam was not nothing"
            );
        }
    }

    #[test]
    fn the_moon_travels_through_the_same_air_column_as_the_sun() {
        // **The defect, stated as a ratio that must not move.**
        //
        // `moon_light` re-derived a Rayleigh-only transmittance and omitted Mie
        // entirely — its `SkyInputs` parameter was named `_inputs` because nothing read
        // it. So a turbidity of 9 dimmed the Sun and left the Moon exactly as bright,
        // which is not two bodies behaving differently. It is one body not travelling
        // through the air, and the second body's doc already claimed *"Same extinction
        // as the Sun, because it is the same air"* while the code did the opposite.
        //
        // **The teeth are the ratio.** Both lights lose exactly the same *fraction* of
        // what arrives, so moon/sun is invariant in turbidity. The moment the moon gets
        // its own transmittance again — any transmittance, even a correct one that is
        // not this one — the ratio drifts with haze and this fails.
        let world = spec();
        let noon = voxel_shapers::climate::TICKS_PER_DAY / 2;
        let position = celestial::solar_position(&world, noon);
        assert!(
            celestial::SkyPhase::sun_is_up(position.altitude_deg),
            "the test needs the sun up, and it is at {} deg",
            position.altitude_deg
        );

        let ratio_at = |inputs: SkyInputs| {
            let sun = Sky::sun_light(&position, inputs);
            let moon = Sky::moon_light(&position, 1.0, inputs);
            assert!(sun.luminance() > 0.0, "the sun was black at {inputs:?}");
            assert!(moon.luminance() > 0.0, "the moon was black at {inputs:?}");
            moon.luminance() / sun.luminance()
        };

        let clear = ratio_at(SkyInputs::default());
        for turbidity in [2.0, 4.0, 9.0] {
            let hazy = ratio_at(SkyInputs { turbidity, ..SkyInputs::default() });
            assert!(
                (hazy - clear).abs() < 1e-12,
                "at turbidity {turbidity} the moon/sun ratio moved from {clear} to {hazy}, so \
                 the moon is not travelling through the sun's air column"
            );
        }
        // And haze dims both of them, which is the half that is easy to state and was
        // false of the moon before.
        let hazy = SkyInputs { turbidity: 9.0, ..SkyInputs::default() };
        assert!(
            Sky::moon_light(&position, 1.0, hazy).luminance()
                < Sky::moon_light(&position, 1.0, SkyInputs::default()).luminance(),
            "a turbidity of 9 did not dim the moon"
        );
    }

    /// The tick at which this world's sun is highest: found by search rather
    /// than assumed, so the test does not depend on which day the algorithm puts
    /// the solstice on.
    fn highest_sun_tick() -> u64 {
        let world = spec();
        let noon = voxel_shapers::climate::TICKS_PER_DAY / 2;
        let mut best = (f64::NEG_INFINITY, 0u64);
        for day in 0..voxel_shapers::climate::DAYS_PER_YEAR {
            let tick = day * voxel_shapers::climate::TICKS_PER_DAY + noon;
            let altitude = celestial::solar_position(&world, tick).altitude_deg;
            if altitude > best.0 {
                best = (altitude, tick);
            }
        }
        assert!(best.0 > 60.0, "the highest noon sun was only {} deg", best.0);
        best.1
    }

    // ---- BLD-0011: the sky is the model's output, not a chosen colour ----

    #[test]
    fn the_zenith_is_blue_because_of_the_scattering_ratio_and_nothing_else() {
        let world = spec();
        let sky = Sky::at(&world, highest_sun_tick(), SkyInputs::default(), 0).expect("a sky");
        assert!(sky.zenith.b > sky.zenith.r, "the zenith was not blue: {:?}", sky.zenith);
        // **The ratio is the physics, but it is not the *scattering* ratio**, and
        // conflating the two is the mistake this bound exists to avoid. The
        // radiance carries `β · (1 − T)` per channel, and `1 − T` is itself much
        // larger for blue than red — so the zenith's blue:red comes out around
        // **12**, not 5.7. The first version of this test asserted `1..8`, which
        // is the scattering ratio, and failed on a correct model. A hand-picked
        // sky blue would not land in this neighbourhood *at all*, which is the
        // property the assertion is for.
        let ratio = sky.zenith.b / sky.zenith.r;
        assert!(
            (4.0..20.0).contains(&ratio),
            "the zenith's blue:red ratio was {ratio}, which is neither the neighbourhood a \
             scattering model gives nor one a picked colour would land in"
        );
    }

    #[test]
    fn the_rayleigh_coefficients_are_the_sources_and_their_ratio_is_the_sky() {
        // Bodhaine et al. (1999), Table 2. The 5.7x is not decoration: it is *why*
        // the sky is blue, so a test that asserted only `b > r` would pass for a
        // model with entirely the wrong physics.
        assert_eq!(RAYLEIGH_BETA_PER_M, [5.802e-6, 13.558e-6, 33.100e-6]);
        let ratio = RAYLEIGH_BETA_PER_M[2] / RAYLEIGH_BETA_PER_M[0];
        assert!(
            (ratio - 5.7).abs() < 0.05,
            "the blue:red scattering ratio came out {ratio} where the source's coefficients \
             give 5.7"
        );
    }

    #[test]
    fn the_sun_reddens_rather_than_being_tinted() {
        // **The claim that distinguishes this from a colour ramp.** If someone
        // lerped the sun toward orange at sunset, an `r > b` assertion would still
        // pass. This passes only because the blue channel's *transmittance* falls
        // faster than the red's — by the scattering ratio — so the ratio itself
        // moves.
        let world = spec();
        // **The day is chosen by measurement, not by arithmetic.** The first
        // version took `TICKS_PER_DAY / 4` — a quarter of the way through the
        // year — and got a day's worth of positions on a day that, at latitude
        // 45, has a noon sun 22 deg up because it is near the **winter** solstice.
        // The search was correct; the expectation of "high" was not. So the test
        // asks the model which day it is, the same way `highest_sun_tick` does.
        //
        // And both ticks are found rather than assumed: adding 11,000 and 15,000 to
        // a quarter-day tick gave two times either side of noon *in hours* rather
        // than in this world's clock, and one of them landed with the sun below the
        // horizon. A sun's position is a search over the year, or it is a guess.
        let noon = voxel_shapers::climate::TICKS_PER_DAY / 2;
        let mut summer_day = (0u64, f64::NEG_INFINITY);
        for d in 0..voxel_shapers::climate::DAYS_PER_YEAR {
            let noon_altitude = celestial::solar_position(&world, d * voxel_shapers::climate::TICKS_PER_DAY + noon)
                .altitude_deg;
            if noon_altitude > summer_day.1 {
                summer_day = (d, noon_altitude);
            }
        }
        assert!(summer_day.1 > 60.0, "the highest noon sun was only {} deg", summer_day.1);
        let base = summer_day.0 * voxel_shapers::climate::TICKS_PER_DAY;

        let high_tick = (0..voxel_shapers::climate::TICKS_PER_DAY)
            .max_by(|a, b| {
                let ha = celestial::solar_position(&world, base + a).altitude_deg;
                let hb = celestial::solar_position(&world, base + b).altitude_deg;
                ha.total_cmp(&hb)
            })
            .expect("the day is not empty");
        let low_tick = (0..voxel_shapers::climate::TICKS_PER_DAY)
            .find(|t| {
                let a = celestial::solar_position(&world, base + t).altitude_deg;
                a > 0.5 && a < 11.0
            })
            .expect("a tick with the sun low and up");
        let high = celestial::solar_position(&world, base + high_tick);
        let low = celestial::solar_position(&world, base + low_tick);
        assert!(high.altitude_deg > 40.0, "the high sample was only {} deg", high.altitude_deg);
        assert!(
            low.altitude_deg > 0.0 && low.altitude_deg < 12.0,
            "the low sample was {} deg",
            low.altitude_deg
        );
        let s_high = Sky::sun_light(&high, SkyInputs::default());
        let s_low = Sky::sun_light(&low, SkyInputs::default());
        let ratio_high = s_high.b / s_high.r;
        let ratio_low = s_low.b / s_low.r;
        assert!(
            ratio_low < ratio_high,
            "the low sun kept its blue:red ratio ({ratio_low} against {ratio_high}); a tint \
             would have preserved it and changed only the levels"
        );
        // And it dims at the same time, by the air mass.
        assert!(
            s_low.luminance() < s_high.luminance() * 0.6,
            "the low sun barely dimmed: {} against {}",
            s_low.luminance(),
            s_high.luminance()
        );
    }

    #[test]
    fn turbidity_is_a_published_input_and_more_of_it_makes_the_sky_whiter() {
        // **The constrained half of INT-0015, measured.** A hazy world has more Mie
        // scattering, and Mie is wavelength-*independent*, so raising turbidity
        // pulls the sky toward grey. If this does not happen, turbidity is a
        // decorative knob and "constrained to the real world models" is not true.
        let clear = Sky::dome_radiance(45.0, 1.0, SkyInputs { turbidity: 2.0, ..SkyInputs::default() });
        let hazy = Sky::dome_radiance(45.0, 1.0, SkyInputs { turbidity: 8.0, ..SkyInputs::default() });
        assert!(hazy.r > 0.0, "the hazy sky was black");
        let saturation = |c: LinearRgb| c.b / c.r;
        assert!(
            saturation(hazy) < saturation(clear),
            "turbidity 2 gave saturation {} and turbidity 8 gave {}; more aerosol should \
             desaturate the sky",
            saturation(clear),
            saturation(hazy)
        );
    }

    #[test]
    fn an_impossible_atmosphere_is_refused_rather_than_read_as_a_clear_sky() {
        // The project's first rule. A negative turbidity is not "clear", it is a
        // broken input, and reading it as the default would be a plausible colour
        // for a world nobody asked for.
        assert!(SkyInputs { turbidity: 2.0, ozone_atm_cm: 0.35 }.validate().is_ok());
        assert!(SkyInputs { turbidity: 0.0, ..SkyInputs::default() }.validate().is_err());
        assert!(SkyInputs { turbidity: f64::NAN, ..SkyInputs::default() }.validate().is_err());
        assert!(SkyInputs { turbidity: 5.0, ozone_atm_cm: -1.0 }.validate().is_err());
        // Zero ozone is a **state**, not a breakage: a world with no ozone layer.
        assert!(SkyInputs { turbidity: 5.0, ozone_atm_cm: 0.0 }.validate().is_ok());
        // And `Sky::at` refuses rather than producing a default sky.
        assert!(Sky::at(&spec(), 0, SkyInputs { turbidity: -1.0, ..SkyInputs::default() }, 0).is_none());
    }

    #[test]
    fn the_phase_functions_are_the_published_ones_at_their_own_limits() {
        // Henyey & Greenstein at `g = 0` is exactly the isotropic `1/4π`, which is
        // the one property of the function a reader can check without the paper.
        assert!((henyey_greenstein(0.3, 0.0) - 1.0 / (4.0 * std::f64::consts::PI)).abs() < 1e-12);
        // Rayleigh's is `3/16π·(1 + cos²θ)`: `3/16π` at 90°, and exactly twice
        // isotropic at its forward and backward peaks.
        assert!((rayleigh_phase(0.0) - RAYLEIGH_PHASE_SCALE).abs() < 1e-12);
        assert!((rayleigh_phase(1.0) - 2.0 * RAYLEIGH_PHASE_SCALE).abs() < 1e-12);
        // And Mie at `g = 0.8` is far more forward-biased than Rayleigh is, which
        // is what puts an aureole around a low sun.
        assert!(henyey_greenstein(1.0, MIE_ASYMMETRY_G) > rayleigh_phase(1.0));
        assert!(henyey_greenstein(-1.0, MIE_ASYMMETRY_G) < rayleigh_phase(-1.0));
    }

    #[test]
    fn the_sun_is_black_below_the_horizon_and_nothing_extrapolates_a_dusk() {
        // A rig built from a sun that keeps shining after sunset is a rig that
        // lights a night. And the dome is black below the horizon too — the
        // **named absence** on `dome_radiance`, because the twilight model is not
        // built yet and returning a small number would be a dusk nobody computed.
        let world = spec();
        let night = Sky::at(&world, 0, SkyInputs::default(), 0).expect("a sky at midnight");
        assert_eq!(night.sun, LinearRgb::BLACK, "the sun was lit at midnight");
        assert_eq!(night.phase, celestial::SkyPhase::Night);
        assert_eq!(night.zenith, LinearRgb::BLACK);
        assert_eq!(night.horizon, LinearRgb::BLACK);
    }

    #[test]
    fn the_moon_is_five_orders_of_magnitude_dimmer_than_the_sun() {
        // **The check on the solid-angle term**, from the appearance side. The moon
        // light calls `celestial::moon_illuminance_w_m2`, and if the `(R/d)²` term
        // were ever dropped from there this test is what notices.
        let world = spec();
        let noon = voxel_shapers::climate::TICKS_PER_DAY / 2;
        let position = celestial::solar_position(&world, noon);
        let full = Sky::moon_light(&position, 1.0, SkyInputs::default());
        let sun = Sky::sun_light(&position, SkyInputs::default());
        assert!(full.luminance() > 0.0, "the full moon contributed nothing at noon");
        assert!(
            full.luminance() < sun.luminance() / 10_000.0,
            "the full moon read {} against a sun of {} — the solid angle is missing again",
            full.luminance(),
            sun.luminance()
        );
    }

    #[test]
    fn the_sky_is_pure_in_the_tick() {
        // F1 again, on the appearance side: a frame that lit differently between
        // two reads of the same tick would be a renderer holding state.
        let world = spec();
        let a = Sky::at(&world, 12_345, SkyInputs::default(), 0).expect("a sky");
        let b = Sky::at(&world, 12_345, SkyInputs::default(), 0).expect("a sky");
        assert_eq!(a, b);
    }

    #[test]
    fn every_constant_is_named_once_and_finite() {
        let mut names: Vec<&str> = PROVENANCE.iter().map(|(n, _, _)| *n).collect();
        let before = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), before, "two rows of PROVENANCE share a name");
        for (name, value, _) in PROVENANCE {
            assert!(value.is_finite(), "{name} is {value}");
        }
    }

    #[test]
    fn this_module_has_no_project_extensions_at_all() {
        // **Zero, and asserted as zero.** The whole argument for putting the sky's
        // colour in `voxel-appearance` rather than the client is that every number
        // here comes from a model. The day a row appears, this fails, and the
        // failure is that argument being false.
        let extensions: Vec<&str> = PROVENANCE
            .iter()
            .filter(|(_, _, why)| *why == SkyProvenance::ProjectExtension)
            .map(|(name, _, _)| *name)
            .collect();
        assert!(
            extensions.is_empty(),
            "the sky acquired project extensions {extensions:?}; a colour constant with no \
             source is exactly what TODO §5 reserved, and deriving it was the whole point"
        );
    }
}
