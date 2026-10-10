//! The light rig **derived from the sky**, so the client never authors a light.
//!
//! **Ticket `BLD-0014`.** `voxel_shapers::sky` knows where the sun is;
//! `voxel_appearance::sky` knows what colour the light is. This module is the
//! join, and it exists as its own file because the join is where a decision can be
//! made by accident — and the decision it must not make is *"what colour is the
//! sun"*.
//!
//! # Why the rig is derived and not written
//!
//! Before this, the client called [`voxel_appearance::npr::LightRig::standard`]
//! and that fixture's own doc said it was **"a fixture and not a claim about the
//! game's lighting"**. It was honest, and it was also the whole of the lighting:
//! a key light at `(0.45, 0.85, 0.28)` that never moved, in a world with a sun
//! that now does. **So the rig here is a function of `(tick, atmosphere)` and has
//! no free parameters** — every number in it came out of `Sky::at`.
//!
//! # What a light carries, and why a shadow needs all of it
//!
//! [`voxel_appearance::npr::Light`] is `direction` · `colour` · `intensity`, and
//! the user's requirement is that **shadows carry light properties**. Those three
//! are what a shadow needs: a direction to cast away from, a colour to be *lit*
//! with (a shadow at sunset is not the shadow at noon, and it is orange), and an
//! intensity that the shadow's own contrast is measured against. A rig carrying
//! only a direction cannot express any of that, so this build produces all three
//! from the model and nothing is defaulted.
//!
//! # The one number here that is not from the sky
//!
//! [`AMBIENT_FROM_HORIZON`] — how much of the sky's dome radiance reaches a
//! surface as ambient. The sky gives the *radiance*; the fraction that a surface
//! integrates is a rendering choice, and it is declared as one here rather than
//! buried in a multiply. It is a **project extension** and is counted by a test.

use voxel_appearance::npr::{Light, LightRig};
use voxel_appearance::openpbr::LinearRgb;
use voxel_shapers::atmosphere::AirSample;
use voxel_appearance::sky::{Sky, SkyInputs};
use voxel_noise::Vec3;
use voxel_shapers::WorldSpec;

/// The share of the sky's horizon radiance a surface takes as ambient, per unit
/// of the dome's own average.
///
/// **A declared rendering choice, not a physical constant.** The sky model
/// computes radiance arriving *from a direction*; a surface integrates it over its
/// hemisphere, and which of that integral this shading model uses is a decision
/// about the stylisation — the same kind of decision as the NPR terms themselves.
/// The number is here, named, and counted as a project extension rather than
/// appearing as a bare `0.5` inside a multiply.
pub const AMBIENT_FROM_HORIZON: f64 = 0.5;

/// The tick's lighting, as the rig the shader reads.
///
/// **This is what "the sun reaches the frame" means.** A caller that wants to know
/// what time it is in the world asks for `rig`; a caller that wants to draw asks
/// for `rig` and gets the same answer.
pub fn rig_of(
    spec: &WorldSpec,
    tick: u64,
    inputs: SkyInputs,
    new_moon_tick: u64,
) -> Option<LightRig> {
    daylight_of(spec, tick, inputs, new_moon_tick).map(|daylight| daylight.rig)
}

/// The rig **and** the numbers a readout needs, from **one** evaluation.
///
/// **This exists so a HUD cannot disagree with the light it is describing.** A caller
/// that wanted both used to compute the sky twice — once through [`rig_of`] for the
/// uniform and once for the altitude to print beside it — and those are two evaluations
/// of one atmosphere, so a bug in either shows up as a readout that contradicts the
/// picture. There is one evaluation here and one struct, and the readout fields *are*
/// the rig's fields.
#[derive(Clone, PartialEq, Debug)]
pub struct Daylight {
    /// The rig the shader reads.
    pub rig: LightRig,
    /// The sun's altitude this frame, degrees, as the sky computed it.
    pub sun_altitude_deg: f64,
    /// The sun light's normalised intensity, or **`0.0` when the rig carries no sun** —
    /// below the horizon there is no beam, so this is an absence and not a dimming.
    pub sun_intensity: f64,
    /// How many lights the rig carries: one for the sun, two when the moon is also up,
    /// zero at a new moon with the sun down.
    pub lights: usize,
    /// The sky this rig was built from, **carried rather than re-derived**.
    ///
    /// The background the player sees behind the terrain is this same object, sampled
    /// per direction by [`sky_table`]. It travels here for the reason
    /// [`crate::skylight::daylight_of`]'s own doc gives for deriving the sky at all:
    /// a caller that could ask for the light and separately for the sky would hold two
    /// evaluations of one atmosphere, and two records drift.
    pub sky: Sky,
}

/// The unit vector **toward** the sun, in the east-north-up frame: `x` east, `z` south, so a
/// due-north sun is `−z` and azimuth runs clockwise from north.
///
/// **One function, because there are two consumers.** The light rig's first light and the
/// drawn disc both ask where the sun is, and a second copy of this formula is a second place
/// for the sky to be wrong in — in a different quarter, which nothing else would notice.
pub fn sun_direction(altitude_deg: f64, azimuth_deg: f64) -> Vec3 {
    let (alt, az) = (altitude_deg.to_radians(), azimuth_deg.to_radians());
    let (sin_alt, cos_alt) = alt.sin_cos();
    let (sin_az, cos_az) = az.sin_cos();
    Vec3::new(sin_az * cos_alt, sin_alt, -cos_az * cos_alt)
}

/// The whole of a tick's daylight, or `None` when the atmosphere is refused.
///
/// See [`rig_of`] for why the sky is derived here rather than handed in.
pub fn daylight_of(
    spec: &WorldSpec,
    tick: u64,
    inputs: SkyInputs,
    new_moon_tick: u64,
) -> Option<Daylight> {
    // **The sky is derived here rather than handed in.** The signature used to be
    // `(spec, tick, sky, sun_altitude_deg)` — three free parameters for a function whose
    // own module doc says it "is a function of `(tick, atmosphere)` and has no free
    // parameters" — and the fourth merely restated `sky.sun_altitude_deg`, so a caller
    // could hand in a direction built from one atmosphere and a light built from
    // another. **Two sources of truth for one atmosphere was the defect the signature
    // had**, and it is precisely what made the normaliser below dishonest: the
    // denominator could not see the turbidity the numerator was measured through.
    //
    // `None` rather than an unlit rig. A refused atmosphere is a question this model
    // cannot answer, and a black frame would be an answer nobody asked for.
    let sky = Sky::at(spec, tick, inputs, new_moon_tick)?;
    let sun_altitude_deg = sky.sun_altitude_deg;

    // The sun's direction **toward** it. The convention lives in [`sun_direction`], which
    // the drawn disc also calls — two consumers, one formula.
    let toward_sun = sun_direction(sun_altitude_deg, sky_azimuth_deg(spec, tick));

    // **Intensity is normalised by this world's own peak sun, measured through the
    // same atmosphere it is dividing**, so the rig carries a ratio rather than a number
    // in W/m². The shader multiplies a half-Lambert term by this, and a material's
    // albedo, and an 8-bit swapchain has no use for 1200 W/m².
    //
    // **What was wrong here, and why it read as a small number rather than a wrong
    // one.** The divisor used to be `noon_irradiance_w_m2(spec)` — the *geometric*
    // direct irradiance at the year's highest noon, which never sees the air path or
    // the turbidity — while the dividend was `sky.sun.luminance()`, which is always the
    // sun **after** `exp(−β_R·H·m) · exp(−β_M·T·H·m)`. Both are W/m², so the units
    // matched and nothing looked wrong; the two sides were simply two different
    // measurements, and the missing air column was 9.93% of every intensity this rig
    // has ever produced. That is the sense in which the normaliser normalised nothing:
    // it was normalising this world's sun by a sun that had not been through this
    // world's air.
    let peak = peak_sun_luminance(spec, inputs);
    let intensity = if peak > 0.0 { sky.sun.luminance() / peak } else { 0.0 };

    let mut lights = Vec::with_capacity(2);
    if intensity > 0.0 {
        // **The colour is a chromaticity, not a radiance — and that is the whole of the
        // 103×.**
        //
        // `Light` is `colour` × `intensity`: `npr.rs` multiplies them and `shading.wgsl`
        // computes `base * (light.colour.rgb * lambert)` where `lambert` already carries
        // `light.direction.w` — the intensity. So one of the two carries the magnitude
        // and the other carries only the shape. [`LightRig::standard`] settles which is
        // which: colour `(0.85, 0.90, 1.0)`, intensity `1.0` — a chromaticity whose
        // strongest channel is exactly 1, and a magnitude beside it.
        //
        // **The sky's sun was handing this shader `sky.sun` itself**, which is the
        // transmitted radiance in W/m² and measured about **1090**. So the light was a
        // 1090-magnitude radiance multiplied by a 0.78 ratio, and the world rendered
        // `mean 0.87457` with a brightest pixel of `78.3`. The fix is not a smaller
        // number; it is putting each factor in the place the rig already put it.
        lights.push(Light {
            direction: toward_sun,
            colour: chromaticity(sky.sun),
            intensity,
        });
    }
    // **The moon is a night light, and only a night light.** It is added when the
    // Sun is **down**, which is the whole of `MAT-0012`'s claim: a night lit by a
    // second source rather than by the sun multiplied by a small number.
    //
    // The Moon is also above the horizon at noon — that is a fact about the sky,
    // not about this rig — and a first version added it then too, so noon carried
    // two lights. That is wrong for a reason worth stating: the Moon's light at
    // noon is ~1e-5 of the Sun's, so the second light costs a shader loop
    // iteration to contribute nothing visible, and worse, it is a *lie about the
    // physics of a shadow*: a shadow cast at noon would be lit on one side by the
    // Sun and, invisibly, by the Moon. The rule is therefore the Sun's own state,
    // not the Moon's.
    if sky.moon.luminance() > 0.0 && peak > 0.0 && !voxel_shapers::sky::SkyPhase::sun_is_up(sun_altitude_deg) {
        lights.push(Light {
            direction: toward_sun,
            colour: chromaticity(sky.moon),
            intensity: sky.moon.luminance() / peak,
        });
    }

    // **The ambient is the sky's own dome radiance**, so a world with a hazier
    // atmosphere is a world whose surfaces are lit by more scattered light. Below
    // the horizon the dome radiance is zero — the named absence on
    // `Sky::dome_radiance` — so a night's ambient is black rather than a dimmed
    // day, and the moon's light is the only thing in it.
    let ambient = if peak > 0.0 {
        // **Divided by the same peak the sun's intensity is divided by**, because a rig
        // carrying a ratio in one field and a raw radiance in the other is one rig in two
        // units. The user saw it on screen: the world rendered **103× too bright** —
        // `mean 0.87457` against the fixture's `0.00847`, brightest pixel `78.3` — and it
        // lost colours, 6 against the fixture's 8, because an ambient four and a half
        // times brighter than full scale swamps the material differences the shading
        // exists to show.
        //
        // **So the ambient is now a ratio too**, and it moves the way the sun's does:
        // brighter near sunrise (where the path is long and the dome genuinely scatters
        // more) and dimmer at noon. It measured `17.97` at the client's start tick
        // against `4.62` at noon, which is the physics; the divisor is what makes those
        // two numbers commensurable with an intensity of `0.52` and `0.78`.
        let dome = (sky.horizon * AMBIENT_FROM_HORIZON) + (sky.zenith * AMBIENT_FROM_HORIZON);
        dome / peak
    } else {
        LinearRgb::BLACK
    };

    // **Counted before the rig takes them**, so the readout's `lights` is the rig's own
    // length rather than a second opinion about how many lights there ought to be.
    let count = lights.len();
    Some(Daylight {
        rig: LightRig { lights, ambient },
        sun_altitude_deg,
        sun_intensity: intensity,
        lights: count,
        // **The same object the rig was built from**, moved rather than recomputed.
        sky,
    })
}

/// Preetham's clear-air turbidity, `T = 2`. **Preetham, Shirley & Smits (1999) §3** —
/// the same paper the scattering solution in `voxel_appearance::sky` implements, and the
/// paper that gives `SkyInputs::turbidity` its meaning. This is the floor the world
/// departs from, not a value chosen for a look.
pub const CLEAR_AIR_TURBIDITY: f64 = 2.0;

/// How much the air's own water raises turbidity above [`CLEAR_AIR_TURBIDITY`].
///
/// **A declared project extension**, not the model's — Preetham's `T` is an *aerosol* load he
/// is handed, and no published source maps a cloud's water path onto it, so the claim is only
/// the one this project can support: *air with condensed water in it is hazier*. The reasoning
/// is `docs/skylight.md` §3, the measurement §3.1, and the open defect beside it is §6.1.
pub const WET_TURBIDITY_GAIN: f64 = 3.0;

/// The sky's two inputs, **derived from the world rather than declared beside it**.
///
/// **Where this was, and why it moved.** `SkyInputs` has exactly two fields — Preetham's
/// turbidity and an ozone depth — and every caller passed `SkyInputs::default()`, which
/// is two unsourced floats in a binary crate. But the world already computes what both
/// of them depend on:
///
/// | input | derived from | the stage that owns it |
/// |---|---|---|
/// | `turbidity` | [`voxel_shapers::weather`] precipitation **rate** (the onset layer's answer, C-94.7), [`voxel_shapers::atmosphere`] density | [`voxel_shapers::stages::Stage::Weather`], `Cadence::Water` (×4) |
/// | `ozone_atm_cm` | [`voxel_shapers::climate`] moisture and season | [`voxel_shapers::stages::Stage::Climate`], `Cadence::Daily` |
///
/// **Read at `(x, z)` because the sky is sampled where the player is standing**, not at
/// the origin — a world whose sky is one value everywhere is a sky with no weather in it.
pub fn sky_inputs_at(spec: &WorldSpec, x: f64, z: f64, tick: u64) -> SkyInputs {
    // **The stage's own answer at the tick.** `rate_at` carries the onset and the rate it
    // implies, which is what makes "how much of the rain is falling now" reachable from the
    // sky (C-94.7). Reading `weather::sample` here would be the annual mean again — the
    // figure the onset exists *beside*, not instead of.
    //
    // **Read as the column the light crosses, not at a point** — `docs/skylight.md` §3.
    let water_path_m = voxel_shapers::clouds::column_water_path_m(spec, x, z, tick);
    let climate = voxel_shapers::climate::sample(spec, x, z, tick);
    let air = voxel_shapers::atmosphere::sample(spec, 0.0);
    SkyInputs {
        turbidity: turbidity_of(water_path_m, &air),
        ozone_atm_cm: ozone_of(&climate),
    }
}

/// The column water path one unit of `wet` is measured against, metres.
///
/// **Measured, not chosen** by `examples/aerosol-column.rs`: over 1024 columns at four times
/// of day the path runs `0.0` to `125.0` m with a p90 of `33.5`, so `100.0` puts this world's
/// p90 column at `T = 4.01`.
///
/// **The p90 is inside the model's range and the maximum is not.** Its deepest column reads
/// `9.50`, past the `T = 6` the fit was taken over and past the `T = 10` where Grok §2.2 says the
/// formulae go unstable — so the claim that stood here (*"Preetham's ordinary range, not beyond
/// it"*) was false, and the clamp below is what holds the evaluation inside it. ChatGPT §10 is
/// also explicit about this constant's home: a normalisation length for cloud water, "not an
/// aerosol scale".
pub const WATER_PATH_REFERENCE_M: f64 = 100.0;

/// The lowest effective turbidity the sky is evaluated at.
///
/// **DeepSeek §2.2, Grok §2.2 and Zotti, Wilkie & Purgathofer (2007) all put the model's usable
/// floor here**: below it the horizon glow is a bright zone that does not occur in nature, and
/// zenith luminance can go negative.
pub const TURBIDITY_FLOOR: f64 = 1.8;

/// The highest effective turbidity the sky is evaluated at.
///
/// **Grok §2.3's fallback clause**, addressed to exactly this case — a retained wet-turbidity
/// stylisation — against the `T ~ 10` where the formulae go numerically unstable.
pub const TURBIDITY_CEILING: f64 = 8.0;

/// Preetham's `T` for a world's own weather: the clear-air anchor, raised by **how much
/// of the rain is falling now** and thinned by how thin the air is.
///
/// **The figure is the stage's rate, not the annual mean.** [`WET_TURBIDITY_GAIN`]'s own
/// claim is *"a world in which it is raining has more aerosol in its air than one in which
/// it is not"* — and until C-94.7 was wired, this function read `precipitation_mm`, the
/// long-run mean, so a column whose rain never varied had a sky that never varied either.
/// The rate is `mean · onset / ONSET_DUTY`, so the derived turbidity **conserves the
/// annual mean**: the year's average haze is what it always was, redistributed in time.
///
/// **Both factors are dimensionless ratios of a shaper's measurement against that
/// shaper's own declared baseline**, so neither invents a scale. Rainfall is divided by
/// [`RAINFALL_REFERENCE_MM`] — `weather`'s own `BASE_PRECIPITATION_MM` — and air density
/// by `atmosphere`'s own `SEA_LEVEL_DENSITY_KG_M3`.
fn turbidity_of(water_path_m: f64, air: &AirSample) -> f64 {
    let wet = (water_path_m / WATER_PATH_REFERENCE_M).max(0.0);
    // **Thin air is clearer, and the ratio is `1.0` at sea level**, so an ordinary world
    // is not silently hazed by the altitude factor. The playable band is three orders of
    // magnitude below the Armstrong limit, so this stays near 1 in practice — which is
    // the honest outcome and not a tuning choice.
    let thin = air.density_kg_m3 / voxel_shapers::atmosphere::SEA_LEVEL_DENSITY_KG_M3;
    // **Clamped, because the world leaves the model's range and the model is what breaks
    // there.** `examples/aerosol-column.rs` measures this world's deepest column at `T = 9.50`,
    // and the alternative ChatGPT §9.2 names — upgrade the evaluator — is `docs/skylight.md`
    // §7's open slice rather than something to pretend to have done here.
    (CLEAR_AIR_TURBIDITY * (1.0 + WET_TURBIDITY_GAIN * wet) / thin.max(f64::EPSILON))
        .clamp(TURBIDITY_FLOOR, TURBIDITY_CEILING)
}

/// The ozone depth a world's climate implies, in atm-cm.
///
/// **A declared projection, and the weakest link in this file.** Ozone lives in the
/// stratosphere and this model has no stratosphere: `climate::ClimateSample` carries
/// surface temperature, moisture and a season phase, and none of those *is* ozone. So
/// this maps the season onto the ozone column rather than deriving it, and says so.
///
/// **Why the season at all.** `climate::season_phase` is the one quantity here with a
/// real published seasonal behaviour behind it — total column ozone peaks in the summer
/// hemisphere, which is why it is dosed in the summer. A year on an axis that peaks once
/// is the shape that gives; the amplitude is the declared part.
const OZONE_SEASON_GAIN_ATM_CM: f64 = 0.02;

/// The reference ozone depth, atm-cm. **300 Dobson units is 3 mm of pure ozone at
/// standard temperature and pressure**, and the standard total column is quoted as
/// **300 DU**; both are the figures the Dobson unit exists to express.
const OZONE_REFERENCE_ATM_CM: f64 = 0.03;

fn ozone_of(climate: &voxel_shapers::climate::ClimateSample) -> f64 {
    // **`sin` of the phase, offset so the minimum is the reference and the peak is the
    // reference plus the gain.** `season_phase` is a fraction of a year, so this is one
    // full cycle per world year and no more.
    let cycle = (climate.season_phase * std::f64::consts::TAU).sin();
    OZONE_REFERENCE_ATM_CM * (1.0 + OZONE_SEASON_GAIN_ATM_CM / OZONE_REFERENCE_ATM_CM * cycle)
}

/// **How much light the frame is given before it is drawn.** A declared rendering choice,
/// not a physical constant, and it sits beside [`AMBIENT_FROM_HORIZON`] because it is the
/// same kind of thing: a choice about *stylisation* rather than about the sky.
///
/// ## Why it is needed, measured
///
/// The rig is physical and in `[0, 1]`: the sun's `intensity` is a ratio against this
/// world's own peak, and the ambient a ratio against the same. Mapped straight onto the
/// swapchain that gave **`mean 0.00085`, brightest pixel `0.0785`, and 0.8 % of pixels
/// above 0.01** — a physically correct radiance and an invisible picture, because a
/// radiance in W/m² and an 8-bit display share no scale and nothing between them declared
/// one.
///
/// ## Why the two ends of this history were both wrong
///
/// * **Too bright:** `Light.colour` was the sun's raw radiance (about **1090**) where the
///   shader wanted a chromaticity, and the ambient was `4.61765` where it wanted a ratio.
///   `mean 0.87457`, brightest pixel **78.3** — a white frame.
/// * **Too dark:** with both corrected the picture is `mean 0.00085`. Also wrong, in the
///   other direction, and for a different reason.
///
/// **The exposure is what sits between those two**, and it is applied to the **uploaded
/// rig**, never to [`daylight_of`]'s result — so the rig stays a physical measurement and
/// `the_rig_is_one_unit_so_no_field_of_it_is_a_raw_radiance` keeps its meaning. A scale
/// folded into the rig would be indistinguishable from the bug above, which was a scale
/// folded into the rig.
///
/// **The value is a choice and is stated as one**, and it was **re-measured** against
/// the world as it is now — the same move C-158 made for [`SKY_EXPOSURE`]. `8.0` was
/// chosen against the placeholder greys C-143 later replaced: with sampled spectra on
/// the bright classes, the ignored GPU test
/// `the_derived_rig_lights_the_shipped_world_rather_than_only_a_probe` measures the
/// world's brightest noon pixel at **1.43345** — a clipped frame, not a lit one. The
/// measured value is **`4.0`**: the same frame measures `max 0.71672, mean 0.37842`.
/// `4.0` rather than the largest value that just fits this noon is deliberate — the
/// sun's `intensity` is normalised against **this world's own peak**, so at the year's
/// highest noon it reaches `1.0` where it measured `0.780` in the test, and a value
/// measured against one noon would clip at another. A power of two, so the scale is
/// exact in binary and the tests that compare against `EXPOSURE` exactly can say so. It
/// is counted by a test so a later edit has to say it changed it.
pub const EXPOSURE: f64 = 4.0;

/// The rig as it is **uploaded**: the physical rig scaled by [`EXPOSURE`].
///
/// **Both terms, together, and that is the point.** The 103× bug was a scale applied to one
/// term and not the other; this is the scale applied to both or to neither, and the test
/// below checks the ratio between them survives.
pub fn exposed(rig: &LightRig) -> LightRig {
    LightRig {
        lights: rig
            .lights
            .iter()
            .map(|l| Light { direction: l.direction, colour: l.colour, intensity: l.intensity * EXPOSURE })
            .collect(),
        ambient: exposed_colour(rig.ambient),
    }
}

/// One radiance on its way to the swapchain: the same [`EXPOSURE`] the rig gets.
///
/// **Shared rather than restated, and this is the point of the function.** The sky
/// background and the light that lights the world come from one `Sky`, so they must
/// also meet one exposure — a sky drawn at a different scale from the light it casts
/// is the same class of bug as the 103× one, with the sun and the sky disagreeing
/// about how bright the world is.
pub fn exposed_colour(colour: LinearRgb) -> LinearRgb {
    colour * EXPOSURE
}

/// How many samples the sky table carries, across the visible dome.
///
/// **A declared sampling resolution, with the error it costs measured.** The sky's
/// radiance is a smooth function of one variable — the cosine of the angle between the
/// view ray and the zenith — so a table over that variable reproduces it to within an
/// interpolation error, and `the_sky_table_is_within_its_declared_error_of_the_model`
/// measures what that error actually is rather than assuming it is small.
pub const SKY_LUT_ROWS: usize = 256;

/// The sky's radiance for every direction a camera can see, **sampled on the CPU**.
///
/// # Why the scattering runs here and not in the shader
///
/// [`Sky::dome_radiance`] is a function of exactly two things: the Sun's altitude and
/// the view direction's cosine. The first is one number for the whole frame. So the
/// model can be evaluated once per table entry and uploaded, and the shader needs to do
/// **no physics at all** — only the trigonometry that turns a pixel into a direction.
///
/// The alternative, reimplementing Rayleigh and Mie in WGSL, would put two copies of
/// one equation in a repository whose own `climate.rs` refuses to *store* a second
/// record of a derived truth because *"two records drift"*. This way the model exists
/// once and the GPU is told what it says.
///
/// `None` when the sky itself refuses the inputs, for the same reason
/// [`crate::skylight::daylight_of`] does: a caller must decide what an unmodellable
/// atmosphere means rather than inherit a plausible colour.
pub fn sky_lut(
    spec: &voxel_shapers::WorldSpec,
    tick: u64,
    inputs: SkyInputs,
    new_moon_tick: u64,
) -> Option<Vec<LinearRgb>> {
    let sky = Sky::at(spec, tick, inputs, new_moon_tick)?;
    let peak = peak_sun_luminance(spec, inputs);
    // **Absence is a refusal, not a dark sky.** A non-finite peak is refused explicitly, because
    // `peak <= 0.0` is false for it and would let a NaN through as a colour.
    if !peak.is_finite() || peak <= 0.0 {
        return None;
    }
    Some(sky_table(&sky, inputs, peak))
}

/// The table for a sky that has **already been derived**, normalised against the
/// world's peak sun.
///
/// This is the entry point a frame loop wants: [`Daylight`] carries the `Sky` its rig
/// was built from, so passing that in means the background and the light are two views
/// of one evaluation rather than two evaluations.
///
/// # Why the division is here and not left to the caller
///
/// [`EXPOSURE`] alone is not enough, and the first version of this pass proved it by
/// drawing: the table is raw radiance, so at noon it reached a brightest pixel of
/// **61.97** against a rig whose brightest pixel was 0.63. The rig gets away with a
/// radiance reaching the swapchain because [`chromaticity`] has already put it in
/// `[0, 1]`; the sky has no such step.
///
/// Dividing by `peak` is the *same* normaliser the light rig uses, deliberately: a
/// second one would be a second dialect of the same field, which is the thing
/// [`chromaticity`]'s own doc refuses. The sky then moves with the tick for the real
/// reason — its radiance falls at dusk — rather than being rescaled to the same
/// brightness every frame.
pub fn sky_table(sky: &Sky, inputs: SkyInputs, peak: f64) -> Vec<LinearRgb> {
    (0..SKY_LUT_ROWS)
        .map(|i| {
            let cos_zenith = i as f64 / (SKY_LUT_ROWS - 1) as f64;
            Sky::dome_radiance(sky.sun_altitude_deg, cos_zenith, inputs) / peak
        })
        .collect()
}

/// **How much light the sky itself is given before it is drawn.** A declared rendering
/// choice, in the same family as [`EXPOSURE`] and for the same reason: a radiance in
/// W/m² and an 8-bit display share no scale.
///
/// **It is not [`EXPOSURE`], and that is deliberate rather than an oversight.** A lit
/// *surface* is the Sun's irradiance times an albedo; the *sky* is a radiance, some
/// three orders of magnitude fainter than the Sun's disc, and the two are exposed
/// separately in every photograph ever taken — expose for the sky and the sun blows
/// out. Sharing one constant would force the sky to be either invisible or white.
///
/// The value is a choice and is stated as one, beside the measurement that produced it.
/// With the spherical geometry (C-157) the normalised noon summary peaks at **0.0611**,
/// so `12.0` puts the brightest noon sky at about **0.73** — bright, unclipped, and below
/// the sunlit terrain's own highlights, so the sky reads as sky rather than as a white
/// card.
///
/// **The first value tried was `220.0` and it was wrong by 9×**, which the test caught
/// with `the exposed noon sky peaks at 6.7005`. It is recorded here because a constant
/// that was guessed and then measured is a different kind of number from one that was
/// measured first, and this one is the latter.
///
/// **It moved from `24.0` with C-157, and the test caught that too.** The geometry repair
/// took the meridian's radiance up by about 2× (the old phase angle was not a spherical
/// angle at all), and the exposed peak read `1.4663` — a sky brighter than the card it
/// must not become. The exposure is re-measured against the repaired model rather than the
/// model being dimmed back to an old number.
pub const SKY_EXPOSURE: f64 = 12.0;

/// One sky radiance on its way to the swapchain.
///
/// Deliberately a **separate function** from [`exposed_colour`], so that no caller can
/// reach the rig's exposure by accident and no reader can mistake one for the other.
/// What the two *do* share is the normaliser, and `the_sky_is_a_normalised_ratio_like_
/// the_rig_is` says so.
pub fn sky_exposed(colour: LinearRgb) -> LinearRgb {
    colour * SKY_EXPOSURE
}

/// **The directional dome's sampling grid**: azimuth cells around the compass and
/// elevation rows from the horizon to the zenith.
///
/// The 1-D table above is the **elevation summary** (`Sky::dome_radiance`, the Δφ = 0
/// meridian) and remains what the ambient pair is sampled from. The drawn sky is this
/// one: `Sky::directional_radiance` over both angles, so the aureole sits where the Sun
/// is. The azimuth count is the coarser axis because the sky's azimuthal structure is
/// the broad aureole, not a vertical edge; the elevation axis keeps the horizon→zenith
/// gradient, which is the sharp one.
pub const SKY_DOME_AZIMUTHS: usize = 64;
/// Elevation rows: horizon (0°) to zenith (90°), inclusive endpoints.
pub const SKY_DOME_ELEVATIONS: usize = 64;
/// Entries in the dome table: `elevation * azimuth`, row-major.
pub const SKY_DOME_ENTRIES: usize = SKY_DOME_ELEVATIONS * SKY_DOME_AZIMUTHS;

/// **The drawn sky: a table over (elevation, azimuth), normalised against the world's own
/// peak sun.**
///
/// The client fills this once per tick from one [`Sky`] — the same evaluation the light rig
/// is built from — and uploads it; the shader turns a pixel into a direction and indexes
/// the table, evaluating geometry and never physics. Row-major: index `elevation *
/// SKY_DOME_AZIMUTHS + azimuth`, both counted from zero at the horizon and at north
/// respectively, azimuth increasing eastwards.
pub fn sky_dome_table(sky: &Sky, inputs: SkyInputs, peak: f64) -> Vec<LinearRgb> {
    let mut table = Vec::with_capacity(SKY_DOME_ENTRIES);
    for e in 0..SKY_DOME_ELEVATIONS {
        let elevation = 90.0 * e as f64 / (SKY_DOME_ELEVATIONS - 1) as f64;
        for a in 0..SKY_DOME_AZIMUTHS {
            let azimuth = 360.0 * a as f64 / SKY_DOME_AZIMUTHS as f64;
            let delta = azimuth - sky.sun_azimuth_deg;
            table.push(Sky::directional_radiance(sky.sun_altitude_deg, elevation, delta, inputs) / peak);
        }
    }
    table
}

/// The dome table read back at a direction — **the same bilinear interpolation the shader
/// performs**, azimuth wrapped and below-horizon clamped.
///
/// Written on this side so the two can be compared rather than assumed equal, exactly as
/// [`sky_lut_lookup`] is for the elevation summary.
pub fn sky_dome_lookup(table: &[LinearRgb], elevation_deg: f64, azimuth_deg: f64) -> LinearRgb {
    if table.is_empty() {
        return LinearRgb::BLACK;
    }
    if table.len() < 2 {
        return table[0];
    }
    let el = elevation_deg.clamp(0.0, 90.0) / 90.0 * (SKY_DOME_ELEVATIONS - 1) as f64;
    let e0 = (el.floor() as usize).min(SKY_DOME_ELEVATIONS - 2);
    let ef = el - e0 as f64;
    let az = azimuth_deg.rem_euclid(360.0) / 360.0 * SKY_DOME_AZIMUTHS as f64;
    let a0 = az.floor() as usize % SKY_DOME_AZIMUTHS;
    let af = az - az.floor();
    let a1 = (a0 + 1) % SKY_DOME_AZIMUTHS;
    let at = |e: usize, a: usize| table[e * SKY_DOME_AZIMUTHS + a];
    let low = at(e0, a0) * (1.0 - af) + at(e0, a1) * af;
    let high = at(e0 + 1, a0) * (1.0 - af) + at(e0 + 1, a1) * af;
    low * (1.0 - ef) + high * ef
}

/// **A ray's place in the dome table**: `(elevation, azimuth)` in degrees, in the
/// convention [`sky_dome_lookup`] and `sky.wgsl` index with.
///
/// The world frame is east-north-up with `x` east and `z` south (the same convention
/// `voxel_shapers::sky` and the shaders use), so the azimuth is `atan2(x, −z)`, zero at
/// north, increasing eastwards, and the elevation is `asin(y)`. Written here because a
/// fluid's mirror ray has to be looked up in the same table the sky pass draws, and two
/// spellings of the convention would put the reflection in the wrong part of the sky.
pub fn sky_direction_of(ray: Vec3) -> (f64, f64) {
    let r = ray.normalise();
    let elevation_deg = r.y.clamp(-1.0, 1.0).asin().to_degrees();
    let mut azimuth_deg = r.x.atan2(-r.z).to_degrees();
    if azimuth_deg < 0.0 {
        azimuth_deg += 360.0;
    }
    (elevation_deg, azimuth_deg)
}

/// The table read back at a direction — **the same interpolation the shader performs**.
///
/// Written out on this side so the two can be compared rather than assumed equal: the
/// test asserts they agree at the sample points, and asserts the error *between* them is
/// bounded. A shader that drifted from this would still look like a sky, which is
/// exactly why it needs a number attached.
pub fn sky_lut_lookup(table: &[LinearRgb], cos_zenith: f64) -> LinearRgb {
    if table.is_empty() {
        return LinearRgb::BLACK;
    }
    if table.len() == 1 {
        return table[0];
    }
    // Below the horizon the table stops: the model is defined from the horizon up, and
    // looking down into a world with no geometry in it is a void rather than a place
    // the sky has an opinion about. Clamping says so instead of extrapolating.
    let c = cos_zenith.clamp(0.0, 1.0);
    let at = c * (table.len() - 1) as f64;
    let i = (at.floor() as usize).min(table.len() - 2);
    let f = at - i as f64;
    table[i] * (1.0 - f) + table[i + 1] * f
}

/// A colour with its strongest channel at **1.0** — a chromaticity, carrying shape and
/// no magnitude.
///
/// **Why the strongest channel, rather than the luminance.** Because that is the
/// convention [`LightRig::standard`] already established with `(0.85, 0.90, 1.0)`, and a
/// second normalisation would be a second dialect of the same field. Normalising by
/// luminance would instead be defensible and is not chosen: the record prefers the
/// convention the code already has over one that is merely tidy.
///
/// **Black stays black** rather than dividing by zero — and it stays black *because* a
/// sun below the horizon is nothing at all, which is the fact the guard protects.
fn chromaticity(colour: LinearRgb) -> LinearRgb {
    let peak = colour.r.max(colour.g).max(colour.b);
    if peak > 0.0 && peak.is_finite() {
        colour / peak
    } else {
        LinearRgb::BLACK
    }
}

/// The sun's azimuth at a tick, from the shaper that owns the geometry.
fn sky_azimuth_deg(spec: &WorldSpec, tick: u64) -> f64 {
    voxel_shapers::sky::solar_position(spec, tick).azimuth_deg
}

/// The brightest sun this world and this atmosphere ever deliver, W/m².
///
/// **The normaliser for [`rig_of`], and the whole fix is that it is measured the same
/// way the dividend is.** It goes through [`Sky::at`], so the air path and the
/// turbidity are in it; the version it replaces called
/// `voxel_shapers::sky::direct_irradiance_w_m2`, which is a geometric quantity above
/// the atmosphere and had no atmosphere parameter at all. **Dividing a measurement by
/// a measurement of a different thing is not normalisation**, whatever the units say.
///
/// **Why searching only the year's noons is exact rather than an approximation.** Every
/// channel is `total · exp(−β_c · H · m)` with air mass `m`, and Kasten & Young's `m` is
/// decreasing in altitude, so `luminance` is strictly increasing in altitude. The peak
/// over the year is therefore at the tick of greatest noon altitude — a 96-step search
/// over [`voxel_shapers::climate::DAYS_PER_YEAR`] of cheap solar positions, and **one**
/// [`Sky::at`] evaluation. Not 96 sky solves.
///
/// A hard-coded "1000" here would be a number this project chose wearing a unit it had
/// not earned, which is what the previous geometric divisor was doing with the sign
/// pointed the other way.
pub fn peak_sun_luminance(spec: &WorldSpec, inputs: SkyInputs) -> f64 {
    let noon = voxel_shapers::climate::TICKS_PER_DAY / 2;
    let mut best = (f64::NEG_INFINITY, 0u64);
    for day in 0..voxel_shapers::climate::DAYS_PER_YEAR {
        let tick = day * voxel_shapers::climate::TICKS_PER_DAY + noon;
        let altitude = voxel_shapers::sky::solar_position(spec, tick).altitude_deg;
        if altitude > best.0 {
            best = (altitude, tick);
        }
    }
    // **`new_moon_tick` is irrelevant here and is not silently borrowed**: the peak is a
    // property of the sun alone, so the moon's phase cannot enter it. `0` is the default
    // the tests already use, and passing a different one could only be ignored.
    Sky::at(spec, best.1, inputs, 0).map(|sky| sky.sun.luminance()).unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_shapers::Config;

    fn spec() -> WorldSpec {
        WorldSpec::new(2468, Config::default())
    }

    /// The world's highest noon tick, found by search. **A test that recomputes this
    /// rather than trusting [`peak_sun_luminance`]'s own search is a test that can
    /// disagree with it** — and that disagreement is the one thing worth catching.
    fn highest_noon_tick(world: &WorldSpec) -> (f64, u64) {
        let noon = voxel_shapers::climate::TICKS_PER_DAY / 2;
        let mut best = (f64::NEG_INFINITY, 0u64);
        for d in 0..voxel_shapers::climate::DAYS_PER_YEAR {
            let tick = d * voxel_shapers::climate::TICKS_PER_DAY + noon;
            let a = voxel_shapers::sky::solar_position(world, tick).altitude_deg;
            if a > best.0 {
                best = (a, tick);
            }
        }
        best
    }

    #[test]
    fn the_rig_carries_a_direction_that_moves_and_an_intensity_that_is_one_at_noon() {
        let world = spec();
        // **The two claims "the sun is in the frame" rests on.** If the direction
        // were fixed the world would have no shadows that move; if the intensity
        // were not 1 at its own brightest, then the normalisation is a guess.
        let peak = peak_sun_luminance(&world, Default::default());
        assert!(peak > 500.0, "the brightest sun was {peak} W/m2");

        let (altitude, tick) = highest_noon_tick(&world);
        let rig = rig_of(&world, tick, Default::default(), 0)
            .expect("the default atmosphere is a legal one");
        // **Exactly one light at noon: the Sun.** The Moon is above the horizon at
        // noon too, and a rig that carried both would be paying for a second
        // light that contributes 1e-5 of the first — and would be drawing a noon
        // shadow lit on one side by the Sun and invisibly by the Moon.
        assert_eq!(rig.lights.len(), 1, "noon has {} lights", rig.lights.len());
        assert!(
            (rig.lights[0].intensity - 1.0).abs() < 1e-12,
            "the brightest sun's intensity was {} rather than 1.0, at altitude {altitude}",
            rig.lights[0].intensity
        );
    }

    #[test]
    fn the_normaliser_is_measured_through_the_air_it_normalises_and_not_above_it() {
        // **The defect, stated as a measurement rather than a story.** The old divisor
        // was `direct_irradiance_w_m2`, a geometric quantity above the atmosphere. The
        // sun the rig actually carries has been through Rayleigh and Mie, so the two
        // differ by exactly the air column — about 9.93%, which is the 0.9007 the
        // intensity used to read. **If this ratio ever returns to 1.0, the divisor has
        // gone back to measuring a sun that never came through the air.**
        let world = spec();
        let peak = peak_sun_luminance(&world, Default::default());
        let (_alt, tick) = highest_noon_tick(&world);
        let geometric = voxel_shapers::sky::direct_irradiance_w_m2(
            voxel_shapers::sky::solar_position(&world, tick).altitude_deg,
        );
        let ratio = peak / geometric;
        assert!(
            ratio < 1.0,
            "the sun delivered {peak} W/m2 through the air but the atmosphere-free \
             figure was only {geometric} W/m2, so the air path was not in the measurement"
        );
        // And by roughly the amount the air column takes, which is the difference
        // between a normaliser and a guess.
        assert!(
            (0.85..0.95).contains(&ratio),
            "the air column took {ratio}, which is not a column"
        );
    }

    #[test]
    fn a_hazier_world_normalises_against_its_own_dimmer_sun() {
        // **The turbidity is in the divisor, and that is the whole point.** Two
        // atmospheres, same world, same tick: a hazy world's brightest sun is dimmer,
        // so its normaliser is lower, and its intensities are ratios against *itself*
        // rather than against a clear sky's number.
        let world = spec();
        let hazy = SkyInputs { turbidity: 9.0, ..Default::default() };
        let clear_peak = peak_sun_luminance(&world, Default::default());
        let hazy_peak = peak_sun_luminance(&world, hazy);
        assert!(
            hazy_peak < clear_peak,
            "a hazier world's brightest sun was {hazy_peak} W/m2 against a clear {clear_peak}"
        );
        // And each one's own brightest moment still reads exactly 1.0, which is what
        // "a ratio rather than a number" has to mean for both of them.
        for (inputs, peak) in [(Default::default(), clear_peak), (hazy, hazy_peak)] {
            let (_alt, tick) = highest_noon_tick(&world);
            let sky = Sky::at(&world, tick, inputs, 0).expect("a sky");
            let ratio = sky.sun.luminance() / peak;
            assert!(
                (ratio - 1.0).abs() < 1e-12,
                "turbidity {} read {ratio} at its own brightest, peak {peak}",
                inputs.turbidity
            );
        }
    }

    #[test]
    fn the_readout_fields_are_the_rigs_own_fields_and_not_a_second_computation() {
        // **The guarantee [`Daylight`] exists to make.** A caller wanting the rig *and*
        // the altitude to print beside it used to evaluate the sky twice, so a bug in
        // either would show up as a HUD contradicting the picture. These are equality
        // assertions against the rig's own contents, not ranges.
        //
        // **The mutation this catches** is the one that makes the type pointless:
        // computing `sun_intensity` from a second `Sky::at` rather than from the sky the
        // rig was built out of. Both are correct to twelve places and they still differ
        // in the last bits, so the panel would print a number the uniform does not hold.
        let world = spec();
        let noon = voxel_shapers::climate::TICKS_PER_DAY / 2;
        let day = daylight_of(&world, noon, Default::default(), 0).expect("a sky at noon");
        let sun = day.rig.lights.first().expect("a sun is in the rig at noon");
        assert_eq!(
            day.sun_intensity, sun.intensity,
            "the readout's intensity is not the rig's, so the panel describes another sun"
        );
        assert_eq!(day.lights, day.rig.lights.len(), "the light count is a second opinion");
        let sky = Sky::at(&world, noon, Default::default(), 0).expect("the same sky");
        assert_eq!(
            day.sun_altitude_deg, sky.sun_altitude_deg,
            "the altitude is not the sky's own altitude"
        );
    }

    #[test]
    fn the_sun_moves_across_the_day_and_the_readout_moves_with_it() {
        // **The claim the client now rests on**, checked without a GPU: two ticks in
        // daylight give two different suns, in altitude and in intensity, and the rig
        // differs to match. Before the wiring, `GAME_TICK` was a `const 0` and the sun
        // never moved at all, so this test has nothing to fail against until a caller
        // feeds it a tick.
        let world = spec();
        let noon = voxel_shapers::climate::TICKS_PER_DAY / 2;
        let noon_alt = voxel_shapers::sky::solar_position(&world, noon).altitude_deg;
        let lower = (1..voxel_shapers::climate::TICKS_PER_DAY)
            .map(|t| (noon + t) % voxel_shapers::climate::TICKS_PER_DAY)
            .find(|&t| {
                let a = voxel_shapers::sky::solar_position(&world, t).altitude_deg;
                a > 0.0 && a > noon_alt * 0.3 && a < noon_alt * 0.65
            })
            .expect("a lower sun that is still up");

        let high = daylight_of(&world, noon, Default::default(), 0).expect("noon");
        let low = daylight_of(&world, lower, Default::default(), 0).expect("a lower sun");
        assert!(high.sun_altitude_deg > low.sun_altitude_deg, "the sun did not move");
        assert!(high.sun_intensity > low.sun_intensity, "the light did not dim as it set");
        assert_ne!(high.rig, low.rig, "two daylight ticks produced one rig");
        // Both lit — otherwise this has quietly become the day-versus-night test, which
        // a rig that dropped the sun would also pass.
        assert!(low.sun_intensity > 0.0, "the lower sun was below the horizon: {low:?}");
    }

    #[test]
    fn the_rig_is_one_unit_so_no_field_of_it_is_a_raw_radiance() {
        // **What the user saw on screen.** The sun's intensity was a ratio against this
        // world's own peak — `1.0` at its brightest — and the ambient was the sky's raw
        // dome radiance, which measured **4.61765** at noon and **17.97** at the
        // client's start tick. So the rig carried a ratio in one field and a radiance
        // four to eighteen times brighter than full scale in the other, and the world
        // rendered **103× too bright**: `mean 0.87457` against the fixture's `0.00847`,
        // brightest pixel `78.3`, and *fewer* distinct colours than the fixture (6
        // against 8) because an ambient that dominant swamps the shading.
        //
        // **Every value in a rig is now in `[0, 1]`.** That is the whole claim, and it is
        // checked across a whole day rather than at one tick, because an ambient that is
        // fine at noon and enormous at dawn is still wrong.
        let world = spec();
        let per_day = voxel_shapers::climate::TICKS_PER_DAY;
        let mut worst: f64 = 0.0;
        for tick in (0..per_day).step_by(per_day as usize / 24) {
            let Some(day) = daylight_of(&world, tick, Default::default(), 0) else {
                continue;
            };
            worst = worst.max(day.rig.ambient.luminance());
            for light in &day.rig.lights {
                worst = worst.max(light.intensity).max(light.colour.luminance());
                assert!(
                    (0.0..=1.0).contains(&light.intensity),
                    "a light's intensity at tick {tick} was {} — the rig is two units again",
                    light.intensity
                );
            }
        }
        assert!(
            worst <= 1.0,
            "the brightest value anywhere in the rig over a whole day was {worst}, so the \
             ambient is still a raw radiance beside a ratio"
        );
        // And the ambient still moves with the sky rather than being pinned — a
        // constant here is the same defect as the constant ambient the standard fixture
        // had, one level up.
        let noon = daylight_of(&world, per_day / 2, Default::default(), 0).expect("noon");
        let morning = daylight_of(&world, per_day * 3 / 8, Default::default(), 0).expect("morning");
        assert!(
            morning.rig.ambient.luminance() > noon.rig.ambient.luminance(),
            "the ambient at sunrise ({}) was not brighter than at noon ({})",
            morning.rig.ambient.luminance(),
            noon.rig.ambient.luminance()
        );
    }

    #[test]
    fn the_sky_is_driven_by_the_worlds_weather_and_not_by_a_default() {
        // **What was missing, stated as a thing that would be missed.** Every caller
        // passed `SkyInputs::default()` — Preetham's clear air — so the sun and the moon
        // were lit through a permanent clear atmosphere while the world around them
        // rained. These are the shapers' own numbers, at the client's start tick, over a
        // whole day, so the range is a measurement rather than an assertion.
        let world = spec();
        // **A year, not a day, and the first version of this test swept a day.** The
        // weather's seasonal term is `sin(season_phase · 2π)` and `season_phase` is
        // `(tick % YEAR_TICKS) / YEAR_TICKS` — a fraction of the **year**, 96 days. So a
        // day is 1/96 of a cycle, and a day-long sweep measured a turbidity that varied
        // by `0.0000000` and looked like the weather was not reaching the sky at all. It
        // was; the test was asking the wrong question.
        let year = voxel_shapers::climate::YEAR_TICKS;
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        let mut ozone_lo = f64::INFINITY;
        let mut ozone_hi = f64::NEG_INFINITY;
        // **Several places, not one, and the reason is in the shaper.** `precipitation_mm`
        // has two branches: a downslope one that returns `BASE * moisture * 0.35` with
        // **no tick in it at all**, and an upslope one carrying the seasonal term. The
        // first version of this test sampled `(0, -96)`, where the wind descends — a rain
        // shadow — and measured a turbidity that varied by exactly `0.0` and concluded
        // the weather was not reaching the sky. The *mean* was not reaching that column
        // and still does not; since C-94.7 the onset is what moves it, and the test below
        // (`the_onset_reaches_the_sky_even_where_the_mean_never_moves`) holds that to a
        // column of exactly that kind. A sky sampled at one point is still not the
        // world's sky, which is why the places stay.
        const PLACES: [(f64, f64); 5] =
            [(0.0, -96.0), (-96.0, 0.0), (96.0, 0.0), (-96.0, -96.0), (96.0, 96.0)];
        for &(x, z) in &PLACES {
            for tick in (0..year).step_by(year as usize / 96) {
                let path_m = voxel_shapers::clouds::column_water_path_m(&world, x, z, tick);
                let climate = voxel_shapers::climate::sample(&world, x, z, tick);
                let air = voxel_shapers::atmosphere::sample(&world, 0.0);
                let t = turbidity_of(path_m, &air);
                let o = ozone_of(&climate);
                lo = lo.min(t);
                hi = hi.max(t);
                ozone_lo = ozone_lo.min(o);
                ozone_hi = ozone_hi.max(o);
            }
        }
        println!(
            "turbidity {lo:.3}..{hi:.3} over a year ({year} ticks), ozone \
             {ozone_lo:.4}..{ozone_hi:.4} atm-cm"
        );

        // **The weather moves the turbidity.** A clear-air sky forever was the defect.
        assert!(
            hi > lo * 1.05,
            "turbidity ranged only {lo}..{hi} across the year, so the weather is not \
             reaching the sky"
        );
        // **And it never falls below the published clear-air anchor**, because that anchor
        // is Preetham's own and a derivation that undercut it would be a derivation of
        // something other than turbidity.
        assert!(
            lo >= CLEAR_AIR_TURBIDITY,
            "turbidity fell to {lo}, under Preetham's clear-air T = {CLEAR_AIR_TURBIDITY}"
        );
        // The ozone cycles with the season and stays in a physical band around the
        // standard 3 mm column.
        assert!(
            ozone_hi > ozone_lo,
            "the ozone depth never moved: {ozone_lo}..{ozone_hi} — the climate is not \
             reaching the sky"
        );
        assert!(
            ozone_lo > 0.0 && ozone_hi < 0.1,
            "the ozone column left a physical band: {ozone_lo}..{ozone_hi} atm-cm"
        );
        // **Preetham's domain starts at 1.0**, so a derived turbidity below it would be
        // refused by `Sky::at` and the client would exit. That is the failure this pins.
        assert!(
            lo >= 1.0,
            "a derived turbidity of {lo} is outside Preetham's domain (T >= 1.0), so \
             `Sky::at` would refuse it and the client would exit"
        );
    }

    #[test]
    fn the_onset_reaches_the_sky_even_where_the_mean_never_moves() {
        // **C-94.7, held to the exact columns it names.** "The stage consumes
        // precipitation and nothing yet consumes the onset" — the consumer is this path.
        // `weather.rs` measured that **49.7 % of columns never move**: the downslope
        // branch returns `BASE * moisture * 0.35` with no `tick` term, so their annual
        // figure is one constant forever and, before the wire, their turbidity was too.
        // This finds a column of exactly that kind and pins that the onset — not the mean
        // — is what moves the sky above it.
        let world = spec();
        let air = voxel_shapers::atmosphere::sample(&world, 0.0);
        let year = voxel_shapers::climate::YEAR_TICKS;
        let step = year as usize / 96;

        let mut found = None;
        'search: for x in (-600..600).step_by(73) {
            for z in (-600..600).step_by(97) {
                let (xf, zf) = (x as f64, z as f64);
                let first = voxel_shapers::weather::precipitation_mm(&world, xf, zf, 0);
                let flat = (0..year)
                    .step_by(step)
                    .all(|t| voxel_shapers::weather::precipitation_mm(&world, xf, zf, t) == first);
                if flat {
                    found = Some((xf, zf, first));
                    break 'search;
                }
            }
        }
        let (x, z, mean) = found.expect("no downslope column in the search area");

        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        let mut onset_lo = f64::INFINITY;
        let mut onset_hi = f64::NEG_INFINITY;
        for t in (0..year).step_by(step) {
            let p = voxel_shapers::weather::precipitation_now(&world, x, z, t);
            assert_eq!(
                p.mean_mm_per_year, mean,
                "the mean moved at ({x}, {z}), which is not a downslope column"
            );
            onset_lo = onset_lo.min(p.onset);
            onset_hi = onset_hi.max(p.onset);
            let now = turbidity_of(
                voxel_shapers::clouds::column_water_path_m(&world, x, z, t),
                &air,
            );
            lo = lo.min(now);
            hi = hi.max(now);
        }
        assert!(
            onset_hi > onset_lo,
            "the onset field itself was flat at ({x}, {z}): {onset_lo}..{onset_hi}"
        );
        assert!(
            hi > lo * 1.05,
            "turbidity stayed {lo}..{hi} on a column whose mean is flat at {mean} mm/year, \
             so the onset is not reaching the sky"
        );
    }

    #[test]
    fn the_declared_couplings_are_named_and_counted() {
        // **The two numbers in this file that are not the model's are declared, and this
        // is the same discipline `AMBIENT_FROM_HORIZON` carries**: a stylisation choice
        // that cannot be changed without a test saying so.
        assert_eq!(CLEAR_AIR_TURBIDITY, 2.0, "Preetham's clear-air turbidity moved");
        // **Pinned against their literals, not against `> 0.0`.** A `> 0.0` assertion on a
        // constant is not a pin at all — it passes for every value anyone would ever type,
        // and clippy is right to call it a constant assertion. Pinning the number is what
        // makes a later edit say it changed it.
        assert_eq!(WET_TURBIDITY_GAIN, 3.0, "the wet-turbidity coupling moved");
        assert_eq!(OZONE_SEASON_GAIN_ATM_CM, 0.02, "the ozone seasonal amplitude moved");
        assert_eq!(OZONE_REFERENCE_ATM_CM, 0.03, "the standard 3 mm ozone column moved");
        assert_eq!(
            WATER_PATH_REFERENCE_M,
            100.0,
            "the water-path reference moved; examples/aerosol-column.rs is where it was read"
        );
        // **The clamp is declared and pinned, and it is not this file's invention.** DeepSeek
        // §2.2, Grok §2.2 and Zotti, Wilkie & Purgathofer (2007) put the model's usable floor at
        // `1.8`; Grok §2.3's fallback clause for a retained wet-turbidity stylisation takes the
        // ceiling to `8.0`. A later edit that widens either has to say so here.
        assert_eq!(TURBIDITY_FLOOR, 1.8, "the sky's turbidity floor moved");
        assert_eq!(TURBIDITY_CEILING, 8.0, "the sky's turbidity ceiling moved");
    }

    #[test]
    fn the_clamp_is_what_holds_the_sky_inside_the_models_range() {
        // **The world leaves the model's range, and this is the measurement.** ChatGPT §11
        // calls it Failure 4 — "Preetham is extrapolated far past its fit range" — and the fit
        // was taken over `T = 2..6`. So the clamp has to be reached and *held* on real columns;
        // a clamp nothing reaches is a declaration, not a control.
        let world = spec();
        let air = voxel_shapers::atmosphere::sample(&world, 0.0);
        let quarter = voxel_shapers::climate::YEAR_TICKS / 4;
        let mut deepest = 0.0f64;
        for t in [0u64, quarter, quarter * 2] {
            for x in (-600..600).step_by(37) {
                for z in (-600..600).step_by(41) {
                    let path = voxel_shapers::clouds::column_water_path_m(
                        &world, x as f64, z as f64, t,
                    );
                    deepest = deepest.max(path);
                }
            }
        }
        assert!(
            deepest > 0.0,
            "no sampled column carried any water, so the clamp was never exercised"
        );
        assert_eq!(
            turbidity_of(deepest, &air),
            TURBIDITY_CEILING,
            "the world's deepest column ({deepest:.1} m) is not held at the ceiling"
        );
        assert!(
            turbidity_of(deepest / 4.0, &air) < TURBIDITY_CEILING,
            "every path sampled sat on the ceiling, so the clamp is not what holds the deep one"
        );
        let clear = turbidity_of(0.0, &air);
        assert!(
            clear >= CLEAR_AIR_TURBIDITY && clear < TURBIDITY_CEILING,
            "clear air read {clear}, so the clamp is clipping ordinary columns"
        );
    }

    // **An assertion that was here, and why it is not.** There was a
    // `RAINFALL_REFERENCE_MM > 0.0` check here, and clippy is right that it could never
    // fail for any value either side would plausibly take — a comparison between two
    // constants is not a pin. The pin beside it, `RAINFALL_REFERENCE_MM ==
    // weather::BASE_PRECIPITATION_MM`, is the one that catches drift, and a zero in the
    // shaper is that shaper's own test to catch. **I wrote an assertion to defend an
    // assertion, and deleted the defence when I read it again.**
    //
    // **Both are gone as of the column slice**: turbidity reads a cloud's water path, so
    // weather's rainfall baseline is no longer a number this file depends on at all.

    #[test]
    fn the_exposure_scales_both_terms_or_neither() {
        // **The scale belongs to the upload, not the rig.** `daylight_of` is a physical
        // measurement and stays in `[0, 1]`; `exposed` is the display step. Folding the
        // exposure into `daylight_of` would make it indistinguishable from the bug this
        // file already had once — a scale in the rig that the shader did not expect.
        let world = spec();
        let day = daylight_of(&world, crate::game::GAME_TICK, Default::default(), 0)
            .expect("a sky at the start tick");
        assert!(
            day.rig.ambient.luminance() <= 1.0 && day.sun_intensity <= 1.0,
            "the rig itself left [0, 1] before any exposure was applied"
        );

        let lit = exposed(&day.rig);
        // **Both terms moved, and by the same factor** — a scale on one term alone is the
        // defect that made the world 103× too bright.
        assert_eq!(
            lit.ambient.luminance() / day.rig.ambient.luminance(),
            EXPOSURE,
            "the ambient was scaled by {} rather than the exposure {EXPOSURE}",
            lit.ambient.luminance() / day.rig.ambient.luminance()
        );
        assert!(
            (lit.lights[0].intensity - day.rig.lights[0].intensity * EXPOSURE).abs() < 1e-12,
            "the light and the ambient were not scaled by the same factor"
        );
        // And the shape is untouched: exposure is a scale, not a restyle.
        assert_eq!(
            lit.lights[0].colour,
            day.rig.lights[0].colour,
            "exposure changed a light's colour, so it is a tone curve wearing a scale's name"
        );
        assert_eq!(lit.lights.len(), day.rig.lights.len(), "exposure dropped a light");
        // The declared value, so a later edit has to say it changed it.
        assert_eq!(EXPOSURE, 4.0, "the declared exposure moved");
    }

    #[test]
    fn where_this_worlds_day_actually_is() {
        // **A measurement, because the obvious fractions are wrong.** The ruling is to
        // open at "morning, `TICKS_PER_DAY/4`" — and `TICKS_PER_DAY/4` is 06:00 by a
        // clock face, which is not where this world's sun is. `solar_position` is a
        // function of the tick and the world's own latitude, so the day is offset from
        // midnight. Printing the whole day is the only honest way to name a start tick.
        let world = spec();
        let per_day = voxel_shapers::climate::TICKS_PER_DAY;
        for (label, tick) in [
            ("0 (GAME_TICK)", 0u64),
            ("1/8", per_day / 8),
            ("1/4", per_day / 4),
            ("3/8", per_day * 3 / 8),
            ("1/2 (noon by face)", per_day / 2),
            ("5/8", per_day * 5 / 8),
            ("3/4", per_day * 3 / 4),
            ("7/8", per_day * 7 / 8),
        ] {
            let altitude = voxel_shapers::sky::solar_position(&world, tick).altitude_deg;
            println!("tick {tick:>6} ({label:>18}): sun {altitude:>8.2} deg");
        }
        // And the two crossings, found by search rather than read off a clock.
        let sunrise = (0..per_day).find(|&t| {
            voxel_shapers::sky::solar_position(&world, t).altitude_deg > 0.0
        });
        println!("first tick with the sun up: {sunrise:?}");
    }

    #[test]
    fn the_tick_the_client_opens_at_is_not_a_black_world() {
        // **The client opens at [`crate::game::GAME_TICK`], and this is what is lit
        // there.** A player who launches and sees nothing has been told the truth about
        // the world by the world, and no console message says so.
        let world = spec();
        let tick = crate::game::GAME_TICK;
        let day = daylight_of(&world, tick, Default::default(), 0).expect("a sky at the start tick");
        println!(
            "start tick {tick}: sun {:.2} deg, intensity {:.4}, {} light(s), ambient {:.5}, \
             and noon is tick {} which is {:.0} s of real time away at {} Hz",
            day.sun_altitude_deg,
            day.sun_intensity,
            day.lights,
            day.rig.ambient.luminance(),
            voxel_shapers::climate::TICKS_PER_DAY / 2,
            (voxel_shapers::climate::TICKS_PER_DAY / 2) as f64 / crate::clock::TICK_HZ as f64,
            crate::clock::TICK_HZ
        );
        assert!(
            day.sun_altitude_deg > 0.0,
            "**the client opens with the sun {} deg below the horizon** — no sun, and an \
             ambient of {:.5}, so the world is drawn unlit. A day is {} ticks, which is \
             {:.0} seconds of real time at {} Hz, so a player waits {:.0} s for sunrise.",
            day.sun_altitude_deg,
            day.rig.ambient.luminance(),
            voxel_shapers::climate::TICKS_PER_DAY,
            voxel_shapers::climate::TICKS_PER_DAY as f64 / crate::clock::TICK_HZ as f64,
            crate::clock::TICK_HZ,
            voxel_shapers::climate::TICKS_PER_DAY as f64 / (2.0 * crate::clock::TICK_HZ as f64)
        );
        assert!(
            day.rig.ambient.luminance() > 0.0,
            "the world opens with a black ambient ({})",
            day.rig.ambient.luminance()
        );
    }

    #[test]
    fn an_atmosphere_the_model_refuses_yields_no_rig_rather_than_a_black_one() {
        // **A refused input is a question, not a night.** `Sky::at` refuses a turbidity
        // below 1.0, and a caller that got an empty rig back could not tell that from
        // midnight — so the absence has to be `None`, and it has to be `None` for the
        // whole rig rather than for one light inside it.
        assert!(
            rig_of(&spec(), 0, SkyInputs { turbidity: 0.0, ..Default::default() }, 0).is_none(),
            "a turbidity of 0 produced a rig rather than a refusal"
        );
        assert!(
            daylight_of(&spec(), 0, SkyInputs { turbidity: 0.0, ..Default::default() }, 0).is_none(),
            "the readout has no absence: it returned something for a refused atmosphere"
        );
    }

    #[test]
    fn the_sun_direction_moves_across_the_day_and_points_up_at_noon() {
        let world = spec();
        let noon = voxel_shapers::climate::TICKS_PER_DAY / 2;
        let mut directions = Vec::new();
        for offset in [0u64, 2_000, 4_000, 8_000, 12_000] {
            let tick = noon + offset;
            let rig = rig_of(&world, tick, Default::default(), 0)
                .expect("the default atmosphere is a legal one");
            if let Some(light) = rig.lights.first() {
                directions.push(light.direction);
            }
        }
        assert!(directions.len() >= 3, "too few daytime samples: {}", directions.len());
        // Not one direction repeated: the whole claim of a moving sun.
        let distinct = directions
            .iter()
            .map(|d| (d.x * 1000.0) as i64)
            .collect::<std::collections::BTreeSet<_>>();
        assert!(distinct.len() > 1, "the sun did not move: {directions:?}");
        // And every direction toward the sun points **up** in the northern
        // hemisphere at midday, which a sign error would invert.
        for d in &directions {
            assert!(d.y > 0.0, "a sun above the horizon pointed downward: {d:?}");
        }
    }

    #[test]
    fn a_night_has_no_sun_light_and_a_black_ambient() {
        // **The absence is the point.** A rig that kept the sun's light through the
        // night would light a night as though it were an overcast afternoon, and
        // the difference between those two pictures is the entire reason the sky
        // exists.
        let world = spec();
        let rig = rig_of(&world, 0, Default::default(), 0)
            .expect("the default atmosphere is a legal one");
        assert!(rig.lights.is_empty(), "a light was in the rig at midnight: {:?}", rig.lights);
        assert_eq!(rig.ambient.luminance(), 0.0, "the night had an ambient glow");
    }

    #[test]
    fn the_ambient_is_the_skys_own_dome_radiance_and_not_a_constant() {
        // **A constant ambient is what the standard fixture had**, and it is what
        // this exists to remove. Two atmospheres at the same tick must produce two
        // different ambients, because a hazy world scatters more light onto the
        // ground.
        let world = spec();
        let noon = voxel_shapers::climate::TICKS_PER_DAY / 2;
        let tick = noon;
        let a = rig_of(&world, tick, Default::default(), 0)
            .expect("a clear sky's rig");
        let b = rig_of(&world, tick, SkyInputs { turbidity: 9.0, ..Default::default() }, 0)
            .expect("a hazy sky's rig");
        assert!(
            (a.ambient.luminance() - b.ambient.luminance()).abs() > 1e-12,
            "a clear and a hazy atmosphere gave the same ambient, so the ambient is a constant"
        );
    }

    #[test]
    fn the_declared_ambient_fraction_is_half_and_is_counted() {
        // The one number in this file that is not from the sky, asserted so that a
        // later edit has to say it changed it.
        assert_eq!(AMBIENT_FROM_HORIZON, 0.5);
    }

    #[test]
    fn the_rig_is_pure_in_the_tick() {
        let world = spec();
        let tick = 12_345;
        assert_eq!(
            rig_of(&world, tick, Default::default(), 0),
            rig_of(&world, tick, Default::default(), 0)
        );
    }

    // ---- the sky table -------------------------------------------------

    /// How far the interpolated table may sit from the model it samples.
    ///
    /// **A declared bound, and the test below measures the real number against it.**
    /// The table is an approximation — the model is evaluated at 256 cosines and
    /// linearly interpolated between them — so the only honest thing is to say how
    /// wrong it is allowed to be and check. It is a *fraction of the brightest sky*,
    /// because an absolute dB bound would mean something different at noon than at
    /// dusk, and the table is used at both.
    const SKY_TABLE_TOLERANCE: f64 = 0.01;

    fn noon_world() -> (voxel_shapers::WorldSpec, u64) {
        let spec = voxel_shapers::WorldSpec::new(
            20_260_930,
            voxel_shapers::spec::Config::default(),
        );
        // A tick with the Sun clearly up, so the table is not the night-time
        // all-zero case that would make any tolerance pass trivially.
        (spec, 9_000)
    }

    #[test]
    fn the_sky_table_reproduces_the_model_it_samples() {
        let (spec, tick) = noon_world();
        let inputs = SkyInputs::default();
        let table = sky_lut(&spec, tick, inputs, 0).expect("a sky at noon");

        assert_eq!(table.len(), SKY_LUT_ROWS, "the table is the declared size");
        // **The table is normalised, so it is compared against the model normalised
        // the same way.** Comparing a ratio against a radiance would pass or fail for
        // reasons that have nothing to do with the table.
        let peak = peak_sun_luminance(&spec, inputs);
        let altitude = Sky::at(&spec, tick, inputs, 0).unwrap().sun_altitude_deg;
        let exact_at = |c: f64| Sky::dome_radiance(altitude, c, inputs) / peak;

        // A table of one colour would make the error test below pass for the wrong
        // reason, so assert the model actually varies across the dome first.
        let brightest = table.iter().map(|c| c.luminance()).fold(0.0_f64, f64::max);
        assert!(brightest > 0.0, "the noon sky table is black, so it measures nothing");
        assert!(
            brightest < 1.0,
            "the normalised sky is {brightest}, so it is not a ratio against the sun"
        );

        // At a sample point the table must be the model exactly — no interpolation.
        for i in 0..SKY_LUT_ROWS {
            let c = i as f64 / (SKY_LUT_ROWS - 1) as f64;
            assert!(
                (sky_lut_lookup(&table, c).luminance() - exact_at(c).luminance()).abs() < 1e-12,
                "at cos-zenith {c} the table is not the model"
            );
        }

        // **Between** the samples is where the approximation lives, so that is what
        // the tolerance is about.
        let mut worst: f64 = 0.0;
        for step in 0..=2000 {
            let c = step as f64 / 2000.0;
            let interpolated = sky_lut_lookup(&table, c).luminance();
            let exact = exact_at(c).luminance();
            worst = worst.max((interpolated - exact).abs() / brightest.max(1e-12));
        }
        assert!(
            worst < SKY_TABLE_TOLERANCE,
            "the sky table is {worst:.5} of the peak away from the model, over the \
             declared {SKY_TABLE_TOLERANCE}"
        );
        println!(
            "sky table error: {worst:.6} of peak over {SKY_LUT_ROWS} samples; \
             brightest {brightest:.6} of the sun"
        );
    }

    #[test]
    fn the_dome_table_is_the_directional_model_it_samples() {
        let (spec, tick) = noon_world();
        let inputs = SkyInputs::default();
        let sky = Sky::at(&spec, tick, inputs, 0).expect("a sky at noon");
        let peak = peak_sun_luminance(&spec, inputs);
        let table = sky_dome_table(&sky, inputs, peak);
        assert_eq!(table.len(), SKY_DOME_ENTRIES, "the dome table is the declared size");

        // A table of one colour would make everything below pass for the wrong reason.
        let brightest = table.iter().map(|c| c.luminance()).fold(0.0_f64, f64::max);
        assert!(
            brightest > 0.0 && brightest < 1.0,
            "the normalised dome is {brightest}, so it is not a ratio"
        );

        // At a sample point the table is the model exactly — no interpolation.
        for e in 0..SKY_DOME_ELEVATIONS {
            for a in 0..SKY_DOME_AZIMUTHS {
                let elevation = 90.0 * e as f64 / (SKY_DOME_ELEVATIONS - 1) as f64;
                let azimuth = 360.0 * a as f64 / SKY_DOME_AZIMUTHS as f64;
                let exact = Sky::directional_radiance(
                    sky.sun_altitude_deg,
                    elevation,
                    azimuth - sky.sun_azimuth_deg,
                    inputs,
                ) / peak;
                let looked = sky_dome_lookup(&table, elevation, azimuth);
                let (a_, b_) = (looked.luminance(), exact.luminance());
                assert!(
                    (a_ - b_).abs() < 1e-12,
                    "at ({elevation}°, {azimuth}°) the dome is not the model: {a_} against {b_}"
                );
            }
        }

        // **The dome knows where the Sun is.** At one elevation, the cell at the Sun's own
        // azimuth is brighter than the cell opposite — the azimuthal fact the elevation-only
        // table could not carry.
        let toward = sky_dome_lookup(&table, 30.0, sky.sun_azimuth_deg);
        let away = sky_dome_lookup(&table, 30.0, sky.sun_azimuth_deg + 180.0);
        assert!(
            toward.luminance() > away.luminance(),
            "the dome's sun-side cell is not brighter: {} against {}",
            toward.luminance(),
            away.luminance()
        );

        // Between the samples, the interpolation error is measured rather than assumed; the
        // bound is generous on purpose and the number is printed so a later change can tighten
        // it from a measurement.
        let mut worst = 0.0_f64;
        for e in 0..=900 {
            let elevation = e as f64 / 10.0;
            for a in 0..=359 {
                let azimuth = a as f64;
                let exact = Sky::directional_radiance(
                    sky.sun_altitude_deg,
                    elevation,
                    azimuth - sky.sun_azimuth_deg,
                    inputs,
                ) / peak;
                let looked = sky_dome_lookup(&table, elevation, azimuth);
                worst = worst.max((looked.luminance() - exact.luminance()).abs());
            }
        }
        assert!(
            worst < 0.1 * brightest + 1e-12,
            "the dome interpolation is {worst} off, over a tenth of the peak {brightest}"
        );
        println!("dome table worst interpolation error: {worst:.6} of peak {brightest:.6}");
    }

    #[test]
    fn the_direction_convention_is_the_one_the_shaders_use() {
        // The compass, asserted in the frame the whole repository uses: x east, z
        // south, azimuth zero at north and increasing eastwards.
        let at = |x: f64, y: f64, z: f64| sky_direction_of(Vec3::new(x, y, z));
        let up = at(0.0, 1.0, 0.0);
        assert!((up.0 - 90.0).abs() < 1e-9, "straight up is not the zenith: {up:?}");
        let east = at(1.0, 0.0, 0.0);
        assert!((east.0 - 0.0).abs() < 1e-9 && (east.1 - 90.0).abs() < 1e-9, "east: {east:?}");
        let south = at(0.0, 0.0, 1.0);
        assert!((south.1 - 180.0).abs() < 1e-9, "south: {south:?}");
        let west = at(-1.0, 0.0, 0.0);
        assert!((west.1 - 270.0).abs() < 1e-9, "west: {west:?}");
        let north = at(0.0, 0.0, -1.0);
        assert!(north.1 < 1e-9, "north: {north:?}");
        // And a below-horizon ray reports a negative elevation rather than clamping —
        // the caller decides what a ray into the ground means.
        let down = at(0.0, -1.0, 0.0);
        assert!((down.0 + 90.0).abs() < 1e-9, "straight down: {down:?}");
    }

    #[test]
    fn a_flat_water_surface_reflects_the_sky_in_the_mirror_direction() {
        // The fluid geometry joined to the table, on the one case a player sees every
        // day: level water seen from above. The reflected ray must be **up**, at the
        // same azimuth as the view, so the looked-up radiance is a sky radiance and
        // the reflection is the sky and not a hole.
        let view = Vec3::new(0.0, 1.0, -1.0).normalise(); // toward an eye above and north
        let normal = Vec3::new(0.0, 1.0, 0.0);
        let ray = voxel_appearance::fluid::reflected_ray(view, normal);
        let (elevation, azimuth) = sky_direction_of(ray);
        assert!(elevation > 0.0, "the mirror ray points below the horizon: {elevation}");
        assert!((elevation - 45.0).abs() < 1e-6, "the mirror elevation is {elevation}");
        assert!((azimuth - 180.0).abs() < 1e-6, "the mirror azimuth is {azimuth}");
        // And it reads a real table: south at 45° is brighter than the same table's
        // black-outside case would be.
        let (spec, tick) = noon_world();
        let inputs = SkyInputs::default();
        let sky = Sky::at(&spec, tick, inputs, 0).expect("a sky");
        let peak = peak_sun_luminance(&spec, inputs);
        let table = sky_dome_table(&sky, inputs, peak);
        let reflected = sky_dome_lookup(&table, elevation, azimuth);
        assert!(reflected.luminance() > 0.0, "the reflection is black at noon");
    }

    #[test]
    fn the_sky_is_a_normalised_ratio_like_the_rig_is() {
        // The normaliser is shared on purpose. A second one would be a second dialect
        // of the same field, which is what `chromaticity`'s doc refuses.
        let (spec, _) = noon_world();
        let inputs = SkyInputs::default();
        // The normaliser itself, asserted rather than merely used: a peak that is not
        // positive and finite has no ratio to be taken against it.
        let peak = peak_sun_luminance(&spec, inputs);
        assert!(peak > 0.0 && peak.is_finite(), "the world's peak sun luminance is {peak}");

        let noon = sky_lut(&spec, 9_000, inputs, 0).expect("a sky at noon");
        let brightest = noon.iter().map(|c| c.luminance()).fold(0.0_f64, f64::max);
        assert!(
            brightest < 1.0,
            "the noon sky reaches {brightest} of the sun's peak, which is not a ratio"
        );

        // **And it is a ratio, so it moves with the tick** — the teeth that matter. A
        // table rescaled to its own peak would read 1.0 at every hour of the day,
        // which is a sky that never gets dark.
        let morning = sky_lut(&spec, 7_500, inputs, 0).expect("a sky at 7500");
        let evening = sky_lut(&spec, 10_500, inputs, 0).expect("a sky at 10500");
        let lum = |t: &Vec<LinearRgb>| t.iter().map(|c| c.luminance()).fold(0.0_f64, f64::max);
        assert!(
            lum(&morning) < brightest * 0.75,
            "the sky at 7500 is {} against noon {brightest} — the sun sets, so it must dim",
            lum(&morning)
        );
        assert!(
            lum(&evening) < brightest * 0.75,
            "the sky at 10500 is {} against noon {brightest} — it must dim the other way too",
            lum(&evening)
        );

        // The exposure puts it in the top of the range without clipping.
        let drawn = noon.iter().map(|c| crate::skylight::sky_exposed(*c).luminance());
        let drawn_peak = drawn.clone().fold(0.0_f64, f64::max);
        assert!(
            drawn_peak > 0.3 && drawn_peak < 1.0,
            "the exposed noon sky peaks at {drawn_peak:.4}, which is neither a sky nor a \
             white card"
        );
    }

    #[test]
    fn the_sky_table_is_black_at_night_and_the_sky_below_the_horizon_clamps() {
        let (spec, _) = noon_world();
        // Tick 0 is midnight in this world, so the dome radiance is the named
        // absence `dome_radiance` documents: black, not a dusk it cannot compute.
        let night = sky_lut(&spec, 0, SkyInputs::default(), 0).expect("a sky at midnight");
        assert!(
            night.iter().all(|c| c.luminance() == 0.0),
            "the midnight sky is not black, so the twilight absence has been filled in"
        );

        // Below the horizon the table stops rather than extrapolating: a void under
        // the world is not a direction the sky has an opinion about.
        let noon = sky_lut(&spec, 9_000, SkyInputs::default(), 0).expect("a sky at noon");
        assert_eq!(
            sky_lut_lookup(&noon, -1.0),
            sky_lut_lookup(&noon, 0.0),
            "looking down invented a colour instead of clamping to the horizon"
        );
    }

    #[test]
    fn a_shorter_sky_table_is_still_read_at_its_own_endpoints() {
        // The lookup is written against the table it is given rather than against
        // SKY_LUT_ROWS, so a caller with fewer samples must not index off the end.
        let two = vec![LinearRgb::new(1.0, 0.0, 0.0), LinearRgb::new(0.0, 0.0, 1.0)];
        assert_eq!(sky_lut_lookup(&two, 0.0), two[0]);
        assert_eq!(sky_lut_lookup(&two, 1.0), two[1]);
        assert_eq!(sky_lut_lookup(&two, 2.0), two[1], "above the zenith must clamp");
        assert_eq!(sky_lut_lookup(&[], 0.5), LinearRgb::BLACK, "an empty table is black");
        assert_eq!(sky_lut_lookup(&[LinearRgb::WHITE], 0.5), LinearRgb::WHITE);
    }

    #[test]
    fn the_sky_and_the_rig_that_lights_the_world_share_one_exposure() {
        // The sky background and the light it casts come from one `Sky`. If they met
        // different exposures the world would be lit at one brightness and have a sky
        // at another, which is the 103× bug wearing different clothes.
        assert_eq!(
            exposed_colour(LinearRgb::new(0.5, 0.5, 0.5)).luminance(),
            0.5 * EXPOSURE,
            "exposing a colour is not exposing it by EXPOSURE"
        );
        let world = noon_world().0;
        let rig = rig_of(&world, 9_000, SkyInputs::default(), 0).expect("a rig at noon");
        let exposed = exposed(&rig);
        let ambient = rig.ambient.luminance() * EXPOSURE;
        assert!(
            (exposed.ambient.luminance() - ambient).abs() < 1e-12,
            "the rig's ambient is exposed by something other than the shared exposure"
        );
    }

    #[test]
    fn the_sky_does_not_step_across_ground_while_walking() {
        // Walking 0.86 m per `Cadence::Water` update used to step the frame's light by
        // **0.098316**, with `EXPOSURE` a constant that never moved — `docs/skylight.md` §3.
        let world = spec();
        let mut tick = crate::game::GAME_TICK;
        let mut x = 0.0f64;
        let mut previous: Option<(f64, f64)> = None;
        let (mut worst_turbidity, mut worst_intensity) = (0.0f64, 0.0f64);
        while x < 256.0 {
            let inputs = sky_inputs_at(&world, x, 0.0, tick);
            let turbidity = inputs.turbidity;
            let day = daylight_of(&world, tick, inputs, 0).expect("a sky along the walk");
            let intensity = day.rig.lights.first().map_or(0.0, |light| light.intensity);
            if let Some((was_turbidity, was_intensity)) = previous {
                worst_turbidity = worst_turbidity.max((turbidity - was_turbidity).abs());
                worst_intensity = worst_intensity.max((intensity - was_intensity).abs());
            }
            previous = Some((turbidity, intensity));
            // 0.86 m of ground and `Cadence::Water`'s 4 ticks together, because the player
            // moves on both axes at once and it is the product that is felt.
            x += 0.86;
            tick += 4;
        }

        println!(
            "walking 256 m: worst step per {:.1} s update is turbidity {worst_turbidity:.4} \
             and sun intensity {worst_intensity:.6}",
            crate::clock::tick_seconds() * 4.0
        );

        // The bounds are declared, and they are set against **both** measurements — 0.5693
        // and 0.017493 with the footprint, against 2.8064 and 0.098316 without it — so they
        // catch the defect they exist for rather than merely restating today's number.
        assert!(
            worst_turbidity < 1.0,
            "turbidity stepped {worst_turbidity} across one atmosphere update of a walk"
        );
        assert!(
            worst_intensity < 0.03,
            "the frame's light stepped {worst_intensity} in one {:.1} s atmosphere update of a \
             walk, which is the walking change the footprint was added to stop",
            crate::clock::tick_seconds() * 4.0
        );
        // And the sampling choice is pinned, so a later edit has to say it changed it.
        assert_eq!(WATER_PATH_REFERENCE_M, 100.0, "the water-path reference moved");
    }
}