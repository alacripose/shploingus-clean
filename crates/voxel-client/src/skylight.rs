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
use voxel_light::{
    aerosol_optical_depth, air_mass, cloud_extinction_per_m, droplet_extinction_per_m,
    night_sky_radiance_w_m2_sr, path_in_layer, rayleigh_optical_depth, rgb_weights,
    sun_visible_fraction, twilight_fraction, Air, BandTable, MoonPath, SunPath, BAND_COUNT,
    DROPLET_ASYMMETRY_G, DROPLET_SINGLE_SCATTER_ALBEDO, GRAZING_AIR_MASS, MIE_ASYMMETRY_G,
    MOON_SOLID_ANGLE_SR,
};
use voxel_appearance::openpbr::LinearRgb;
use voxel_mesh::octree::SparseVoxelOctree;
use voxel_shapers::atmosphere::AirSample;
use voxel_stream::light::{transmittance_tree, Matter, SKY_STENCIL};
use voxel_appearance::sky::{Sky, SkyInputs};
use voxel_noise::Vec3;
use voxel_shapers::WorldSpec;

/// The share of the sky's own reading a surface takes as ambient, per unit of that reading.
///
/// **A declared rendering choice, not a physical constant.** The sky model
/// computes radiance arriving *from a direction*; a surface integrates it over its
/// hemisphere, and which of that integral this shading model uses is a decision
/// about the stylisation — the same kind of decision as the NPR terms themselves.
/// The number is here, named, and counted as a project extension rather than
/// appearing as a bare `0.5` inside a multiply.
///
/// # Two readings, one share, and the gap between them that is owed
///
/// The **field** reads the dome over [`voxel_stream::light::SKY_STENCIL`] — the zenith and four rays
/// `45°` above the horizon — walked through the world's own octree, so a cell under a roof receives
/// the sky a roof leaves it ([`WorldLight::bands_across`]). The **rig** reads the horizon and the
/// zenith at half each, with nothing in the way ([`daylight_of`]), and that is the reading the
/// shipped day's brightness was measured against.
///
/// They are not interchangeable yet, and the number is large: the days' whole ambient lives in the
/// **horizon band**, because this model's sky is single-scattering ([`daylight_of`]'s own note
/// carries the figures — `0.591` against a zenith of `3.9e-5` at the crossing). The stencil's mean
/// therefore reads **five thousand times** smaller, so a frame that took it as the ambient's only
/// source measured `mean 0.171` against `0.378`. **The stencil is the field's sampler and this
/// constant is the share both readings take**; making the field the ambient's *only* source is owed
/// on the sky away from the horizon, not on this number.
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
    /// **The frame's own exposure grade**: `1.0` where the world's own peak sun is what is
    /// arriving, larger as the light falls, so a moonlit night is exposed for the Moon it has and
    /// not for a noon it does not. The law is [`daylight_of`]'s, and it is continuous in the
    /// tick — both terms are smooth functions of the clock — which is what keeps a sunset from
    /// stepping.
    pub grade: f64,
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
///
/// **And it is a conversion, not the formula.** The convention moved down to
/// [`voxel_shapers::sky::direction_of`], beside the two functions that produce an altitude and an
/// azimuth, when the streamer's object lane became a third consumer: that lane streams the Sun and
/// the Moon as world bodies and cannot reach into this crate, so leaving the trigonometry here
/// would have made the streamed body and the drawn one two statements of one convention.
pub fn sun_direction(altitude_deg: f64, azimuth_deg: f64) -> Vec3 {
    let [x, y, z] = voxel_shapers::sky::direction_of(altitude_deg, azimuth_deg);
    Vec3::new(x, y, z)
}

/// **The frame's own exposure grade.**
///
/// The Sun is the reference while it is up — `peak`, this world's brightest sun, so a noon is exactly
/// the noon every number in this file was measured against — and the Moon becomes the reference once
/// the Sun is down, because the Moon's own light is about `1.3e-6` of the Sun's (the real ratio, and
/// not a fault) and a fixed exposure draws that night black.
///
/// **The crossing is the model's own twilight curve** — `twilight_fraction`'s `exp(-decay·altitude)`
/// below the horizon, as the exponent that carries one grade into the other, `1` while the Sun is up.
/// So the mapping is continuous in the Sun's altitude: no step at the horizon and none under it.
/// `max(1.0)` keeps every frame the Sun lights exactly as bright as it was, which is the half of this
/// that matters to a day nobody asked to change.
///
/// **A step in a frame is therefore not here.** Both bodies' own light is gated to zero at its own
/// horizon — the Sun's beam at `0°` and the Moon's illuminance at its own — and no exposure can hold a
/// frame smooth across a light that steps. That is the next piece of this work, and
/// `the_exposure_grade_is_continuous_in_the_twilight` prints the steps it is fighting.
pub fn exposure_grade(peak: f64, sun_altitude_deg: f64, moon_illuminance: f64) -> f64 {
    let moon_grade = if peak > 0.0 && moon_illuminance > 0.0 {
        peak / moon_illuminance
    } else {
        1.0
    };
    let twilight = voxel_light::twilight_fraction(sun_altitude_deg);
    moon_grade.powf(1.0 - twilight).max(1.0)
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
    // The sky is derived here rather than handed in — docs/lighting-sky-and-moon.md
    let sky = Sky::at(spec, tick, inputs, new_moon_tick)?;
    let sun_altitude_deg = sky.sun_altitude_deg;

    // The sun's direction **toward** it. The convention lives in [`sun_direction`], which
    // the drawn disc also calls — two consumers, one formula.
    let toward_sun = sun_direction(sun_altitude_deg, sky_azimuth_deg(spec, tick));

    // intensity is normalised by this world's own peak sun — docs/lighting-sky-and-moon.md
    let peak = peak_sun_luminance(spec, inputs);
    // **The Moon, read once, from the one orbit the rest of this file reads.** It is needed twice:
    // for its own light, and for the fraction of the Sun's own disc it is standing in front of.
    let moon = voxel_shapers::sky::moon_position(spec, tick, new_moon_tick);
    // **The Sun's beam carries its own visibility**, so a Moon crossing the disc takes the direct
    // light as well as the scattered light. One function decides that, for both, so the shadow and
    // the dimming cannot disagree about an eclipse.
    let sun_visible = voxel_light::sun_visible_fraction(
        &voxel_light::SunPath {
            altitude_deg: sky.sun_altitude_deg,
            azimuth_deg: sky.sun_azimuth_deg,
        },
        &voxel_light::MoonPath {
            altitude_deg: moon.altitude_deg,
            azimuth_deg: moon.azimuth_deg,
            illuminated_fraction: moon.phase_fraction,
        },
    );
    // **The Sun's own output is not this ground's view of it.** `voxel_appearance::sky` keeps the
    // Sun shining below the horizon now (the Moon is lit by it), so the rig takes the **twilight
    // share** of it: exactly `1.0` while the Sun is up, decaying through civil, nautical and
    // astronomical twilight under it, where the code this replaces cut the last `4e-3` of the day's
    // beam to nothing at `0.000°` in one update. Whether the rig *carries* a sun at all is still
    // `sun_is_up`, which is the same rule `WorldLight` switches the field's beam body on — one rule,
    // one place, so the rig and the field cannot disagree about which body is lighting the world.
    let visibility = twilight_fraction(sun_altitude_deg);
    let intensity = if peak > 0.0 && voxel_shapers::sky::SkyPhase::sun_is_up(sun_altitude_deg) {
        sky.sun.luminance() / peak * sun_visible * visibility
    } else {
        0.0
    };

    let mut lights = Vec::with_capacity(2);
    if intensity > 0.0 {
        // the colour is a chromaticity, not a radiance — docs/lighting-sky-and-moon.md
        lights.push(Light {
            direction: toward_sun,
            colour: chromaticity(sky.sun),
            intensity,
        });
    }
    // **The Moon's own light, read once for the whole frame.** Two consumers need it now — the rig's
    // night light below, and the exposure grade, which has to know the Moon's light *before* the Sun
    // sets for the grade to cross a sunset without a step — so a second call here would be a second
    // Moon. All twelve bands are zero for a Moon below the horizon, which is the solver's own reading
    // of "there is no light to add".
    let moon_bands = if peak > 0.0 {
        let air = crate::sun::air_at(spec, 0.0, 0.0, tick);
        let path = voxel_light::MoonPath {
            altitude_deg: moon.altitude_deg,
            azimuth_deg: moon.azimuth_deg,
            illuminated_fraction: moon.phase_fraction,
        };
        voxel_light::moonlight(&voxel_light::BandTable::computed(), &air, &path).ok()
    } else {
        None
    };
    let moon_illuminance = moon_bands
        .map(|bands| bands.iter().sum::<f64>())
        .filter(|total| total.is_finite() && *total > 0.0)
        .unwrap_or(0.0);

    // The moon is a night light, and only a night light — docs/lighting-sky-and-moon.md
    if moon_illuminance > 0.0 && !voxel_shapers::sky::SkyPhase::sun_is_up(sun_altitude_deg) {
        let shape = voxel_light::visible_rgb(&moon_bands.expect("the bands the sum came from"));
        lights.push(Light {
            direction: sun_direction(moon.altitude_deg, moon.azimuth_deg),
            colour: chromaticity(LinearRgb {
                r: shape[0],
                g: shape[1],
                b: shape[2],
            }),
            intensity: moon_illuminance / peak,
        });
    }

    // **The ambient is the sky's own dome radiance**, so a world with a hazier
    // atmosphere is a world whose surfaces are lit by more scattered light. Below
    // the horizon the dome radiance is zero — the named absence on
    // `Sky::dome_radiance` — so a night's ambient is black rather than a dimmed
    // day, and the moon's light is the only thing in it.
    //
    // **And it is the same sample the field measures, with nothing in the way.** [`AMBIENT_FROM_HORIZON`]
    // carries the convention; this loop is that convention's other source, so the rig's ambient for
    // an unoccluded cell is the field's own answer for that cell rather than a differently weighted
    // reading of a different pair of rays.
    //
    // **The two rays this is read from, and why it is still two rays.** This was measured rather
    // than assumed, and the measurement is why the field cannot yet be its only source: at the
    // crossing the model puts the **horizon band**'s red at `0.591` against a **zenith** of
    // `3.9e-5`, and at a `13.9°` sun at `0.298` against `1.4e-5` — so *the whole of this term lives
    // in the horizon band*, because a single-scattering sky away from the horizon is nearly dark.
    // The field's own stencil ([`AMBIENT_FROM_HORIZON`]) therefore reads `2.1e-5` at the crossing
    // against this law's `0.131`, and a frame that took the stencil's mean instead of this reading
    // measured **`mean 0.171` against `0.378`, `max 0.538` against `0.717`** over the shipped world
    // (`renderer::tests::the_derived_rig_lights_the_shipped_world_rather_than_only_a_probe`).
    //
    // So the term stays on the rays that carry it, and the debt is named rather than paid out of
    // the day's brightness: **the sky away from the horizon** — multiple scattering, or a declared
    // floor with a source — is what has to exist before the field's own occluded measurement can
    // replace this, and until it does, a measured corner's sky is the field's (nearly dark) and the
    // rig's ambient is added beside it rather than instead of it (`fs_main`).
    let ambient = if peak > 0.0 {
        // Divided by the same peak the sun's intensity is divided by — docs/lighting-sky-and-moon.md
        let air = crate::sun::air_at(spec, 0.0, 0.0, tick);
        let model = SkyModel::of(&sky, &air, peak, &moon);
        let azimuth = sky_azimuth_deg(spec, tick);
        let horizon = sky_ambient_at(&model, 0.0, azimuth);
        let zenith = sky_ambient_at(&model, 90.0, azimuth);
        (horizon + zenith) * AMBIENT_FROM_HORIZON
    } else {
        LinearRgb::BLACK
    };

    // **The frame's own grade**, from the world's peak sun, the Sun's own altitude and the Moon's own
    // light — the three things that decide it and nothing else, so the mapping can be swept on its own
    // ([`exposure_grade`]) and a step in a frame attributed to the light rather than to this.
    let grade = exposure_grade(peak, sun_altitude_deg, moon_illuminance);

    // **Counted before the rig takes them**, so the readout's `lights` is the rig's own
    // length rather than a second opinion about how many lights there ought to be.
    let count = lights.len();
    Some(Daylight {
        rig: LightRig { lights, ambient },
        sun_altitude_deg,
        sun_intensity: intensity,
        lights: count,
        grade,
        // **The same object the rig was built from**, moved rather than recomputed.
        sky,
    })
}

/// The three bands an eye receives, by index in the solver's band table: 400–500, 500–600 and
/// 600–700 nm — **blue, green, red**, which is the order `shading.wgsl` reverses when it builds
/// `vec3(r, g, b)` out of `light_direct`'s own triple.
pub const VISIBLE_BANDS: [usize; 3] = [3, 4, 5];

/// **The world's own light, cell by cell, marched through the world's own octree.**
///
/// `Scene::build_with_light` attaches a measurement to a corner and `shading.wgsl` reads it as
/// `light_present`; this is that measurement. What it is *not* is a second light model: every
/// number in it comes from the same solver the rig's lights come from and the same walk the
/// world's own picking rays use.
///
/// # One law, two bodies
///
/// The beam is **the Sun's own light while the Sun is up, and the Sun's own light reflected by the
/// Moon while it is not** — `direct_beam` and `moonlight`, the same two calls the rig's first light
/// is built from, so doubling the Sun doubles this beam and an independent emitter could not. The
/// direction is the same body's: toward the Sun by day, toward the Moon by night, which is what
/// makes the field's radiance and the shader's wrap angle agree about where the light comes from.
/// `the_fields_beam_is_the_rigs_own_first_light` holds the two against each other.
///
/// # The walk, and why it is this one
///
/// Every band is multiplied by the transmittance **marched through the world's own octree**
/// ([`transmittance_tree`]), which is the same law the per-voxel walk measures and the hierarchy's
/// own answer to where the matter is: a cell under a roof is dark, a cell in the open is lit, and a
/// solid block costs one charge rather than one per voxel. The dome is walked the same way, ray by
/// ray over [`SKY_STENCIL`], so a cave mouth receives less of the sky than a ridge — the sky's own
/// share of a cell is measured rather than assumed.
///
/// **A beam that nothing stands in front of is the same light the rig gives that cell**, because
/// both are the solver's own bands over the same normaliser; a beam that meets matter is the one
/// the rig cannot give, because the rig has no idea where the cell is.
///
/// # The frame, and the units
///
/// [`WorldLight::bands`] takes the cell **in the tree's own coordinates**, for the reason
/// [`transmittance_tree`] states: a chunk's tree is a shape, not a place.
///
/// Everything is **the peak-relative radiance the rig is in**, per band: the same division by
/// [`peak_sun_luminance`] the rig's intensities and the sky table carry, so a frame lit by this
/// field and a frame lit by the rig are commensurable. **The dome enters as
/// [`AMBIENT_FROM_HORIZON`]'s own share of the stencil's mean** — the convention that constant
/// declares, and the same one [`daylight_of`] reads the rig's ambient with — so the field's sky and
/// the rig's are one law at two sources rather than two quantities wearing one name.
pub struct WorldLight {
    /// The direction toward the body the beam comes from: the Sun, or the Moon.
    toward_beam: [f64; 3],
    /// The beam's own bands at the top of the sky, before the world takes any of it, in the
    /// solver's band order.
    beam: [f64; voxel_light::BAND_COUNT],
    /// The dome's radiance in each of the stencil's own directions, likewise before the world
    /// takes any of it.
    sky: [([f64; 3], [f64; voxel_light::BAND_COUNT]); SKY_STENCIL.len()],
    /// The world's own normaliser, so the bands leave in the rig's own units.
    peak: f64,
    /// **Whether the beam is the Moon's** — the Sun's light, reflected. Named rather than inferred
    /// by a caller comparing directions.
    beam_is_moon: bool,
}

impl WorldLight {
    /// The tick's field, or `None` when the atmosphere is refused — **the same refusal from the
    /// same call** the rig makes.
    pub fn of(spec: &WorldSpec, tick: u64, inputs: SkyInputs, new_moon_tick: u64) -> Option<Self> {
        let daylight = daylight_of(spec, tick, inputs, new_moon_tick)?;
        let peak = peak_sun_luminance(spec, inputs);
        if !(peak > 0.0) || !peak.is_finite() {
            return None;
        }
        let table = voxel_light::BandTable::computed();
        let air = crate::sun::air_at(spec, 0.0, 0.0, tick);
        let sun = voxel_light::SunPath {
            altitude_deg: daylight.sun_altitude_deg,
            azimuth_deg: daylight.sky.sun_azimuth_deg,
        };
        // **One `if`, and both arms are the solver's.** By day the beam is the Sun's disc; by night
        // it is that same disc's light reflected off the Moon's declared bands, which is why the
        // night's light moves when the Sun's spectrum moves and an emitter of its own could not.
        // **The Moon, read once, because both arms and the sky need it.** Its path is what the
        // beam is measured at when the Sun is down, and what the sky's own lunar term is scattered
        // from at every hour — and it is the same reading `sky.wgsl` places the drawn disc from.
        let moon = voxel_shapers::sky::moon_position(spec, tick, new_moon_tick);
        let moon_body = voxel_light::MoonPath {
            altitude_deg: moon.altitude_deg,
            azimuth_deg: moon.azimuth_deg,
            illuminated_fraction: moon.phase_fraction,
        };
        let (toward_beam, beam, beam_is_moon) = if voxel_shapers::sky::SkyPhase::sun_is_up(
            daylight.sun_altitude_deg,
        ) {
            // **The Sun's own beam carries its own visibility**, so a Moon across the disc dims
            // the light on the ground at the same instant it dims the sky.
            let bands = voxel_light::direct_beam(&table, &air, &sun, Some(&moon_body)).ok()?;
            let d = sun_direction(daylight.sun_altitude_deg, daylight.sky.sun_azimuth_deg);
            ([d.x, d.y, d.z], bands, false)
        } else {
            // **The night's beam comes from where the Moon actually is**, and a Moon that has set
            // delivers nothing: `moonlight` is zero below the horizon, so a night with no Moon up
            // is a beam of zero and a field of sky alone rather than a moon glued to the zenith.
            let bands = voxel_light::moonlight(&table, &air, &moon_body).ok()?;
            let d = sun_direction(moon.altitude_deg, moon.azimuth_deg);
            ([d.x, d.y, d.z], bands, true)
        };
        // The dome: the solver's own radiance, in each of the stencil's own directions. The
        // direction-to-angles conversion is [`sky_direction_of`]'s, so a ray of the dome is named
        // the way every other ray in this module is named.
        let mut sky = [([0.0f64; 3], [0.0f64; voxel_light::BAND_COUNT]); SKY_STENCIL.len()];
        for (slot, direction) in sky.iter_mut().zip(SKY_STENCIL) {
            let (elevation_deg, azimuth_deg) =
                sky_direction_of(Vec3::new(direction[0], direction[1], direction[2]));
            *slot = (
                direction,
                voxel_light::sky_radiance(
                    &table,
                    &air,
                    &sun,
                    Some(&moon_body),
                    elevation_deg,
                    azimuth_deg,
                )
                .ok()?,
            );
        }
        Some(WorldLight { toward_beam, beam, sky, peak, beam_is_moon })
    }

    /// **The six numbers a corner reads**: the beam's three bands and the dome's three, in band
    /// order — the busiest of the measurement and the one the shader's own reversal expects.
    ///
    /// `matter` and `tree` are the world's, and so is `from_tree_m`: one tree is one chunk's
    /// cube, so a caller holding a world point subtracts the chunk's origin.
    pub fn bands(
        &self,
        matter: &impl Matter,
        tree: &SparseVoxelOctree,
        from_tree_m: [f64; 3],
        reach_m: f64,
    ) -> [f32; 6] {
        let direct = transmittance_tree(matter, tree, from_tree_m, self.toward_beam, reach_m);
        // The dome, one stencil ray at a time, each weighted by what reached the cell along its
        // own path. **The mean is the stencil's** — five rays rather than an integral — because
        // that is the same estimator [`Sampler::sky`] uses for the rig, and a second, more
        // elaborate one here would make the two disagree about the sky they measure.
        let mut diffuse = [0.0f64; 3];
        for (direction, radiance) in &self.sky {
            let through = transmittance_tree(matter, tree, from_tree_m, *direction, reach_m);
            for (slot, band) in VISIBLE_BANDS.iter().enumerate() {
                diffuse[slot] += radiance[*band] * through;
            }
        }
        self.assemble(direct, diffuse)
    }

    /// **The six numbers a corner reads, with the march going across chunks.**
    ///
    /// [`WorldLight::bands`] charges **one** tree and stops at its own box — the honest answer for
    /// a measurement of one chunk, and the wrong one for a *frame*: a ridge forty metres away
    /// shadows this cell and lives in another chunk's tree, so a single-tree field would light a
    /// valley floor at sunset as if the ridge were not there. This walks the ray **chunk by
    /// chunk**, each resident tree answering for its own 32 m, out to `reach_m`.
    ///
    /// # `None` is "no measurement", and it is not "clear"
    ///
    /// A cell whose own chunk has no resident tree — a chunk being rebuilt, or one outside the
    /// window — has **no field**, so the caller falls back to the rig rather than inheriting a
    /// statement nobody measured. A chunk *further along* the ray that is absent charges nothing
    /// (there is no tree to charge), and the count of those hops comes back beside the bands as
    /// [`WorldLight::bands_across`]'s second value: a measurement that quietly assumed clear air
    /// for a chunk the streamer does not hold would be exactly the plausible lie this project
    /// keeps refusing.
    ///
    /// The first chunk's cell is excluded from its own beam ([`transmittance_tree`]'s rule), and
    /// every hop after it charges **including** the voxel it starts in
    /// ([`transmittance_tree_charging_the_start`]), because that voxel is matter the beam has just
    /// entered — the rule a wall standing on a chunk boundary would otherwise leak through.
    pub fn bands_across(
        &self,
        trees: &dyn crate::field::ChunkTrees,
        from_world_m: [f64; 3],
        reach_m: f64,
    ) -> Option<([f32; 6], usize)> {
        let (direct, absent) = self.march(trees, from_world_m, self.toward_beam, reach_m)?;
        let mut diffuse = [0.0f64; 3];
        let mut unresolved = absent;
        for (direction, radiance) in &self.sky {
            let (through, absent) = self.march(trees, from_world_m, *direction, reach_m)?;
            unresolved += absent;
            for (slot, band) in VISIBLE_BANDS.iter().enumerate() {
                diffuse[slot] += radiance[*band] * through;
            }
        }
        Some((self.assemble(direct, diffuse), unresolved))
    }

    /// **The band arithmetic, once**: a beam share and the dome's own sum in, the six numbers a
    /// corner reads out.
    ///
    /// Extracted so [`WorldLight::bands`] and [`WorldLight::bands_across`] cannot disagree about
    /// the normaliser or the `π` — they are the same law read at two reachs, and a second copy of
    /// this would be a second exposure.
    fn assemble(&self, beam_share: f64, diffuse: [f64; 3]) -> [f32; 6] {
        let mut out = [0.0f32; 6];
        for (slot, band) in VISIBLE_BANDS.iter().enumerate() {
            out[slot] = (self.beam[*band] * beam_share / self.peak) as f32;
        }
        // A hemisphere's cosine-weighted integral of a uniform radiance: `π · L`.
        //
        // **Left as it was, and the comparison it is owed is in [`AMBIENT_FROM_HORIZON`]'s doc.**
        // Rescaling this to the rig's two-ray convention was tried and reverted: the field's sky is
        // already thousands of times darker than the rig's ambient because the model's sky away from
        // the horizon is single-scattering, so a factor this module chooses here cannot reconcile
        // them — the term has to be fixed where it is wrong, and until then this is the field's own
        // reading of its own stencil.
        let per_ray = core::f64::consts::PI / self.sky.len() as f64;
        for (slot, value) in diffuse.iter().enumerate() {
            out[3 + slot] = (value * per_ray / self.peak) as f32;
        }
        out
    }

    /// The transmittance along one direction, **hopping the world's chunks**: `Some((through,
    /// hops whose chunk was absent))`, or `None` when the cell's own chunk is not resident.
    fn march(
        &self,
        trees: &dyn crate::field::ChunkTrees,
        from_world_m: [f64; 3],
        direction: [f64; 3],
        reach_m: f64,
    ) -> Option<(f64, usize)> {
        let length = (direction[0] * direction[0]
            + direction[1] * direction[1]
            + direction[2] * direction[2])
            .sqrt();
        if !length.is_finite() || length <= 0.0 || !reach_m.is_finite() || reach_m <= 0.0 {
            return None;
        }
        let unit = [direction[0] / length, direction[1] / length, direction[2] / length];
        // How far past a chunk's own face a hop steps before the next one is named — docs/lighting-sky-and-moon.md
        const BOUNDARY_EPS_M: f64 = 1.0e-6;
        let side = voxel_model::CHUNK_SIDE as f64;
        let mut hops = (reach_m / side).ceil() as usize + 2;
        let mut through = 1.0f64;
        let mut absent = 0usize;
        let mut from = from_world_m;
        let mut left = reach_m;
        let mut first = true;
        while left > 1.0e-9 && hops > 0 {
            hops -= 1;
            let pos = crate::field::chunk_of(from);
            let Some(tree) = trees.tree_at(pos) else {
                if first {
                    return None;
                }
                absent += 1;
                break;
            };
            let origin = pos.origin();
            let origin_m = [origin.x as f64, origin.y as f64, origin.z as f64];
            let from_tree = [from[0] - origin_m[0], from[1] - origin_m[1], from[2] - origin_m[2]];
            // Where this chunk's own box ends along the ray: the hop. `slab` is the hierarchy's
            // own box test, so the hop and the spans inside it are the same geometry.
            let ray = voxel_mesh::raycast::Ray::new(from_tree, unit);
            let Some((_, far, _)) = voxel_mesh::raycast::slab(
                &ray,
                [0, 0, 0],
                voxel_mesh::OCTREE_LEVELS,
                left,
            ) else {
                break;
            };
            let here = far.clamp(0.0, left);
            if here <= 0.0 {
                break;
            }
            let matter = voxel_stream::light::ChunkMatter::new(tree, [origin.x, origin.y, origin.z]);
            let hop = if first {
                transmittance_tree(&matter, tree, from_tree, unit, here)
            } else {
                voxel_stream::light::transmittance_tree_charging_the_start(
                    &matter, tree, from_tree, unit, here,
                )
            };
            through *= hop;
            if through <= 0.0 {
                return Some((0.0, absent));
            }
            // The step **through** the face rather than onto it: see `BOUNDARY_EPS_M`. The
            // remainder is clamped at zero so a hop that ends the reach cannot make it negative.
            let step = (here + BOUNDARY_EPS_M).min(left);
            from = [
                from[0] + unit[0] * step,
                from[1] + unit[1] * step,
                from[2] + unit[2] * step,
            ];
            left -= step;
            first = false;
        }
        Some((through, absent))
    }

    /// The direction toward the body the beam comes from — the Sun, or the Moon.
    pub fn toward_beam(&self) -> [f64; 3] {
        self.toward_beam
    }

    /// Whether the beam is the Moon's: the Sun's own light, reflected.
    pub fn beam_is_moon(&self) -> bool {
        self.beam_is_moon
    }

    /// The beam's own bands at the top of the sky, in the solver's order — the measurement before
    /// the world takes any of it, for a caller comparing it against the rig's own light.
    pub fn beam_bands(&self) -> [f64; voxel_light::BAND_COUNT] {
        self.beam
    }

    /// The world's own normaliser, as [`peak_sun_luminance`] measured it for this sky.
    pub fn peak(&self) -> f64 {
        self.peak
    }
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
    exposed_at(rig, 1.0)
}

/// The rig as it is uploaded **at a frame's own exposure grade** — [`exposed`] when the grade is
/// `1.0`, which is the day [`EXPOSURE`] was measured against.
///
/// **Both terms take the same scale, grade and all**, for the reason [`exposed`]'s own doc gives: a
/// scale on one term and not the other is the 103× bug, and a grade that lifted the Moon's light
/// without lifting the sky it stands in would be that bug with a longer name.
pub fn exposed_at(rig: &LightRig, grade: f64) -> LightRig {
    let scale = EXPOSURE * grade;
    LightRig {
        lights: rig
            .lights
            .iter()
            .map(|l| Light { direction: l.direction, colour: l.colour, intensity: l.intensity * scale })
            .collect(),
        ambient: rig.ambient * scale,
    }
}

/// **The field's six bands on their way to the shader** — the same [`EXPOSURE`] and the same grade
/// the rig beside them takes, applied at the one place the frame's measurement becomes the frame's
/// light.
///
/// # Why this is not a formality
///
/// [`WorldLight`] leaves its bands **peak-relative and unexposed** — deliberately, and its own doc
/// says the unit it is in is the rig's, exactly (`the_fields_beam_is_the_rigs_own_first_light` holds
/// the two against each other). But the shader is handed the *uploaded* rig, which is
/// `EXPOSURE · grade` times the physical one, so the two numbers the fragment compares are a
/// measured band and a scaled band. Without this, a corner that gained a measurement was lit by a
/// beam `EXPOSURE` times dimmer than the rig's own beam for the same geometry — which is a second
/// reason the world still looked rig-lit wherever the field had spoken, beside the ambient the
/// fragment added on top.
///
/// **The cached bands are not what is scaled.** [`CellField`] keeps the physical measurement, so a
/// frame's grade — which moves through a twilight — cannot make the cache disagree with itself; the
/// scale is applied where the measurement is read, and this function is that statement.
pub fn exposed_bands(bands: [f32; 6], grade: f64) -> [f32; 6] {
    let scale = (EXPOSURE * grade) as f32;
    let mut out = bands;
    for value in out.iter_mut() {
        *value *= scale;
    }
    out
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
    sky_exposed_graded(colour, 1.0)
}

/// One sky radiance on its way to the swapchain **at a frame's own exposure grade** — the name is
/// distinct from `sky_exposed_at`, which reads a *model* at a direction, because a grade is a frame's
/// number and a direction is a pixel's.
///
/// The sky, the drawn bodies and the light the world is lit by must meet one exposure, and the grade
/// is the half of it a frame owns — a graded sky beside an ungraded disc would be two pictures of one
/// night.
pub fn sky_exposed_graded(colour: LinearRgb, grade: f64) -> LinearRgb {
    colour * (SKY_EXPOSURE * grade)
}

/// **The sky's transport, as the GPU reads it**: the rows `atmosphere.wgsl` declares.
///
/// This is what replaced the dome table. The dome was 64×64 samples of
/// `voxel_appearance::sky`'s fit, and the shader interpolated between them; this is the *model* —
/// the band table's twelve coefficients, the air's own two geometries, and the sun's own column —
/// so the sky a pixel carries is the integral the light rig was built from, at that pixel's own
/// direction, with nothing between them to interpolate.
///
/// **One model, three consumers**, which is the whole point of it being rows rather than a table:
/// the sky pass draws it, the world pass reflects it off a fluid, and [`sky_exposed_at`] reads the
/// same rows back on the CPU so a test can hold the two transcriptions of
/// `voxel_light::solver::sky_radiance` to each other — the rule `sun::BodyCuboid::ray_hits` and
/// `sky.wgsl`'s `cuboid_entry` already follow for a body's geometry.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SkyModel {
    rows: [[f32; 4]; SKY_MODEL_ROWS],
}

/// The rows a model carries: **six header rows and three per band**, which is the layout
/// `atmosphere.wgsl` documents and the number its own refusal is written against.
///
/// **The Moon has two rows of its own, and that is the whole of what makes a night a night.** The
/// transport's only source used to be the Sun, so below the horizon its own scattering was the
/// twilight's share of nothing and a midnight sky was the moonless floor beside a Moon that was up
/// and full. Rows 4 and 5 are the Moon's column, its lit fraction and the Sun's own visible
/// fraction — which is the eclipse — and the direction toward it.
pub const SKY_MODEL_ROWS: usize = 6 + 3 * BAND_COUNT;

/// The bytes a model occupies: `SKY_MODEL_ROWS` `vec4`s.
pub const SKY_MODEL_BYTES: usize = SKY_MODEL_ROWS * 16;

/// The rows a host states when a frame has no sky: **zero is no model, not a short one**. The
/// shader reads its bands from row 6 up and takes their count from row 3, so a zeroed buffer is
/// the transport's own arithmetic answering nothing rather than a sky with rows missing.
pub const NO_SKY_ROWS: usize = 0;

/// **Why a frame may not read the sky its host stated.**
///
/// `atmosphere.wgsl` refuses a buffer shorter than its own layout, and the buffer this client
/// binds is always the declared length — so that refusal cannot see the case that matters: a host
/// that never uploaded a model binds `SKY_MODEL_ROWS` rows of zeros, which is the right length
/// and no sky. The host's own record is what tells those two apart, and this is the refusal it is
/// read against.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SkyModelRefusal {
    /// The host has stated nothing: not a model, and not that there is no sky either.
    NothingStated,
    /// A host stated rows that are neither [`NO_SKY_ROWS`] nor [`SKY_MODEL_ROWS`].
    RowCount { stated: usize, declared: usize },
}

impl std::fmt::Display for SkyModelRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkyModelRefusal::NothingStated => write!(
                f,
                "no sky has been stated for this frame: the rows hold the zeros they were \
                 created with, which is the length atmosphere.wgsl declares and no sky inside it. \
                 State one with WorldPipeline::write_sky_model, or \
                 WorldPipeline::write_no_sky_model for a frame that has none"
            ),
            SkyModelRefusal::RowCount { stated, declared } => write!(
                f,
                "the sky was stated at {stated} rows and the layout atmosphere.wgsl declares is \
                 {declared}"
            ),
        }
    }
}

/// **Whether a frame may read the sky, and how many rows it reads**: the host's own record of
/// what it uploaded, against the layout both ends of the wire declare.
///
/// `None` is *nothing stated* and is refused; `Some`[`NO_SKY_ROWS`] is a stated absence, which the
/// shader draws as the black this pass drew before a sky existed; `Some`[`SKY_MODEL_ROWS`] is a
/// model. Any other count is refused rather than padded or truncated — the rule `atmosphere.wgsl`
/// follows one layer down, made to fail here, where a host can see it and a build can read it.
pub fn admit_sky_rows(stated: Option<usize>) -> Result<usize, SkyModelRefusal> {
    match stated {
        None => Err(SkyModelRefusal::NothingStated),
        Some(rows) if rows == NO_SKY_ROWS || rows == SKY_MODEL_ROWS => Ok(rows),
        Some(stated) => Err(SkyModelRefusal::RowCount { stated, declared: SKY_MODEL_ROWS }),
    }
}

/// The share of the night sky's radiance one band carries — `voxel_light::solver`'s own
/// `response_night_share`, flat across the bands that transport light and zero for those that do
/// not.
const NIGHT_SHARE_PER_BAND: f64 = 0.1;

impl SkyModel {
    /// **The model a frame draws**, from the sky, the air and the world's own peak sun.
    ///
    /// Every number here is a measurement this tree already made: the bands are
    /// `voxel_light::spectra`'s, the air is the shapers' density, scale height and droplet load,
    /// the sun's path is `voxel_light::solver`'s own optics, and the exposure is
    /// [`SKY_EXPOSURE`] over [`peak_sun_luminance`] — **the same normaliser the drawn Sun and Moon
    /// are divided by**, so one exposure covers the sky, the disc and the night.
    pub fn of(sky: &Sky, air: &Air, peak: f64, moon: &voxel_shapers::sky::MoonPosition) -> SkyModel {
        Self::of_graded(sky, air, peak, moon, 1.0)
    }

    /// [`SkyModel::of`] **at a frame's own exposure grade** — one sky, one grade, so the night's
    /// exposure is the same number here as in [`exposed_at`] and [`sky_exposed_graded`].
    pub fn of_graded(
        sky: &Sky,
        air: &Air,
        peak: f64,
        moon: &voxel_shapers::sky::MoonPosition,
        grade: f64,
    ) -> SkyModel {
        let table = BandTable::computed();
        // **The Moon as the solver sees it**, from the one reading of its orbit the caller made
        // — and the same one `sun::DrawnMoon` places the body from, so the sky's light and the
        // drawn disc cannot be two different Moons.
        let moon = MoonPath {
            altitude_deg: moon.altitude_deg,
            azimuth_deg: moon.azimuth_deg,
            illuminated_fraction: moon.phase_fraction,
        };
        let toward_moon = sun_direction(moon.altitude_deg, moon.azimuth_deg);
        let moon_elevation = moon.altitude_deg.max(0.0);
        let moon_air_path = air.scale_height_m * air_mass(moon_elevation);
        let moon_drop_path = path_in_layer(moon_air_path, moon_elevation, air.droplet_layer_depth_m);
        let sun_visible = sun_visible_fraction(
            &SunPath {
                altitude_deg: sky.sun_altitude_deg,
                azimuth_deg: sky.sun_azimuth_deg,
            },
            &moon,
        );
        let weights = rgb_weights();
        // A droplet and a vapour molecule take the same light out of the beam, and both are
        // carried by one coefficient here for the reason the solver carries them in one.
        let droplets = droplet_extinction_per_m(air.droplet_lwc_g_m3)
            + cloud_extinction_per_m(air.cloud_water_path_m);
        // The sun's own column: its path length is this model's optics rather than a caller's
        // number, and below the horizon it is the horizon's path — a depressed sun's light does
        // not cross a longer one.
        let sun_elevation = sky.sun_altitude_deg.max(0.0);
        let air_path_sun = air.scale_height_m * air_mass(sun_elevation);
        let drop_path_sun = path_in_layer(air_path_sun, sun_elevation, air.droplet_layer_depth_m);
        let toward_sun = sun_direction(sky.sun_altitude_deg, sky.sun_azimuth_deg);
        let exposure = if peak > 0.0 && peak.is_finite() {
            SKY_EXPOSURE * grade / peak
        } else {
            0.0
        };

        let mut rows = [[0.0f32; 4]; SKY_MODEL_ROWS];
        rows[0] = [
            air.scale_height_m as f32,
            droplets as f32,
            air.droplet_layer_depth_m as f32,
            DROPLET_SINGLE_SCATTER_ALBEDO as f32,
        ];
        rows[1] = [
            air_path_sun as f32,
            drop_path_sun as f32,
            twilight_fraction(sky.sun_altitude_deg) as f32,
            night_sky_radiance_w_m2_sr() as f32,
        ];
        rows[2] = [
            toward_sun.x as f32,
            toward_sun.y as f32,
            toward_sun.z as f32,
            exposure as f32,
        ];
        rows[3] = [
            MIE_ASYMMETRY_G as f32,
            DROPLET_ASYMMETRY_G as f32,
            GRAZING_AIR_MASS as f32,
            BAND_COUNT as f32,
        ];
        // **The Moon's own two rows.** Its column, how much of its disc the Sun is lighting, and
        // the Sun's own visible fraction — the eclipse, carried here as one scalar so that the
        // drawn sky, the CPU's reading of it and the rig's ambient all darken together.
        rows[4] = [
            moon_air_path as f32,
            moon_drop_path as f32,
            moon.illuminated_fraction as f32,
            sun_visible as f32,
        ];
        rows[5] = [toward_moon.x as f32, toward_moon.y as f32, toward_moon.z as f32, 0.0];
        for (index, band) in table.bands().iter().enumerate() {
            // **A band an eye does not receive weighs nothing and still carries light**: it has
            // no colour weight and no night floor of its own (`response_night_share` is zero for
            // it), and its power stays in the sum the magnitude is taken from — which is the rule
            // `sun::DrawnMoon` already follows, `visible_rgb`'s shape against every band's own sum.
            let links = matches!(band.transport, voxel_light::Transport::Links);
            let centre_um = band.centre_nm() * 1.0e-3;
            rows[6 + index * 3] = [
                band.extinction_per_m as f32,
                band.scatter_per_m as f32,
                band.sun_w_per_m2 as f32,
                rayleigh_optical_depth(centre_um) as f32,
            ];
            rows[7 + index * 3] = [
                aerosol_optical_depth(centre_um) as f32,
                if links { 0.0 } else { NIGHT_SHARE_PER_BAND as f32 },
                weights[index][0] as f32,
                weights[index][1] as f32,
            ];
            // **The band's own Moon reflection factor**, precomputed: the reflectance the band
            // table declares, over `pi` by Lambert's law, through the disc's own solid angle. The
            // shader multiplies it by the band's solar power and the lit fraction, so the Moon's
            // light stays the Sun's light reflected rather than a second set of numbers.
            rows[8 + index * 3] = [
                weights[index][2] as f32,
                (band.moon_reflectance / core::f64::consts::PI * MOON_SOLID_ANGLE_SR) as f32,
                0.0,
                0.0,
            ];
        }
        SkyModel { rows }
    }

    /// **The rows this model carries**, which is the layout `atmosphere.wgsl` declares — the
    /// number [`admit_sky_rows`] admits and [`crate::WorldPipeline::write_sky_model`] records.
    pub const fn rows(&self) -> usize {
        SKY_MODEL_ROWS
    }

    /// The model's bytes, exactly the `vec4` rows `atmosphere.wgsl` indexes.
    pub fn bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(SKY_MODEL_BYTES);
        for row in &self.rows {
            for value in row {
                out.extend_from_slice(&voxel_model::endian::le_bytes(*value));
            }
        }
        out
    }

    /// One row, for a test that reads the layout back rather than trusting it.
    pub fn row(&self, index: usize) -> [f32; 4] {
        self.rows[index]
    }
}

/// **The sky's exposed colour at one direction**, from the model the shader reads.
///
/// The CPU's reading of `atmosphere.wgsl`'s `sky_radiance_rgb`: the same rows, the same
/// single-scatter integral in closed form, the same split between the bands' magnitude and the
/// response's shape. It exists so a test can compare a frame's pixels against numbers computed from
/// the model rather than against a second copy of the model, and so `shading::fluid_surface_cpu`
/// reflects the sky the shader reflects.
///
/// **The phase is the reduced form** the shader writes: `mixed_phase` divides a phase by the air's
/// optical depth and multiplies by it again, `r·R + a·H` either way.
pub fn sky_exposed_at(model: &SkyModel, elevation_deg: f64, azimuth_deg: f64) -> LinearRgb {
    let head = model.row(0);
    let sun_row = model.row(1);
    let sun = model.row(2);
    let optics = model.row(3);
    let exposure = sun[3] as f64;
    if !(exposure > 0.0) {
        return LinearRgb::BLACK;
    }
    let (scale_height_m, droplets) = (head[0] as f64, head[1] as f64);
    let (layer_depth_m, albedo) = (head[2] as f64, head[3] as f64);
    let (air_path_sun, drop_path_sun) = (sun_row[0] as f64, sun_row[1] as f64);
    let (twilight, night) = (sun_row[2] as f64, sun_row[3] as f64);
    // **The Moon's own rows**, read the way the shader reads them: its column, its lit fraction,
    // the Sun's own visible fraction, and the direction toward it.
    let moon_row = model.row(4);
    let moon_dir = model.row(5);
    let (moon_air_path, moon_drop_path) = (moon_row[0] as f64, moon_row[1] as f64);
    let (moon_lit, sun_visible) = (moon_row[2] as f64, moon_row[3] as f64);
    let (mie_asymmetry, droplet_asymmetry) = (optics[0] as f64, optics[1] as f64);
    let (grazing, bands) = (optics[2] as f64, optics[3] as usize);

    // The ray's own column, in the one frame `sun_direction` builds the model's sun in.
    let elevation_deg = elevation_deg.clamp(0.0, 90.0);
    let direction = sun_direction(elevation_deg, azimuth_deg);
    let sin_e = elevation_deg.to_radians().sin().abs();
    let view_air_mass = (1.0 / sin_e.max(1.0e-9)).clamp(1.0, grazing);
    let air_path_view = scale_height_m * view_air_mass;
    let drop_path_view = path_in_layer(air_path_view, elevation_deg, layer_depth_m);
    let droplet_scatter = droplets * albedo;
    let cos_theta = direction.x * sun[0] as f64
        + direction.y * sun[1] as f64
        + direction.z * sun[2] as f64;
    let cos_theta_moon =
        direction.x * moon_dir[0] as f64 + direction.y * moon_dir[1] as f64 + direction.z * moon_dir[2] as f64;

    let mut acc = [0.0f64; 3];
    let mut total = 0.0;
    for index in 0..bands.min(BAND_COUNT) {
        let a = model.row(6 + index * 3);
        let b = model.row(7 + index * 3);
        let c = model.row(8 + index * 3);
        let extinction = a[0] as f64;
        let beta_e = extinction + droplets;
        let beta_s = a[1] as f64 + droplet_scatter;
        let tau_sun = extinction * air_path_sun + droplets * drop_path_sun;
        let sun_transmittance = (-tau_sun).exp() * twilight;
        let t_view = (-(extinction * air_path_view + droplets * drop_path_view)).exp();
        let mut single_scatter = 0.0;
        if beta_e > 0.0 {
            let phase = mixed_phase(
                a[3] as f64,
                b[0] as f64,
                droplet_scatter,
                cos_theta,
                mie_asymmetry,
                droplet_asymmetry,
            );
            single_scatter = (beta_s / beta_e) * (1.0 - t_view) * phase;
        }
        // **The Moon's own scattering**, in the shader's own order: the band's solar power, the
        // band's reflection factor, the lit fraction, its own phase angle, its own column. No
        // twilight — the Moon is a body, and the Sun's afterglow is not its light.
        let lunar = if beta_e > 0.0 && moon_lit > 0.0 {
            let moon_phase = mixed_phase(
                a[3] as f64,
                b[0] as f64,
                droplet_scatter,
                cos_theta_moon,
                mie_asymmetry,
                droplet_asymmetry,
            );
            let tau_moon = extinction * moon_air_path + droplets * moon_drop_path;
            a[2] as f64 * c[1] as f64 * moon_lit * (beta_s / beta_e) * (1.0 - t_view) * moon_phase
                * (-tau_moon).exp()
        } else {
            0.0
        };
        // The floor is a radiance: added, never scaled by the band's own power.
        let radiance =
            a[2] as f64 * sun_visible * single_scatter * sun_transmittance + lunar + night * b[1] as f64;
        let weight = [b[2] as f64, b[3] as f64, c[0] as f64];
        for (channel, value) in acc.iter_mut().enumerate() {
            *value += radiance * weight[channel];
        }
        total += radiance;
    }
    // The shape is the response's own sum of the same bands, so the colour and the magnitude are
    // one evaluation rather than two measurements.
    let shape: f64 = acc.iter().sum();
    if !(shape > 0.0) || !(total > 0.0) {
        return LinearRgb::BLACK;
    }
    let scale = exposure * total / shape;
    LinearRgb {
        r: acc[0] * scale,
        g: acc[1] * scale,
        b: acc[2] * scale,
    }
}

/// **The sky's own radiance at one direction, as a ratio against the world's own peak sun** — the
/// unit the light rig's ambient is in.
///
/// Exactly [`sky_exposed_at`] with the exposure taken back out of it, and that is not a trick: the
/// model carries `SKY_EXPOSURE / peak` in its own row, so dividing by [`SKY_EXPOSURE`] is the same
/// arithmetic the exposure itself is. What it buys is that **the ambient and the drawn sky are one
/// evaluation** — the same rows, the same closed form, the same two sources — rather than a
/// summary of one model lighting a frame drawn from another.
pub fn sky_ambient_at(model: &SkyModel, elevation_deg: f64, azimuth_deg: f64) -> LinearRgb {
    sky_exposed_at(model, elevation_deg, azimuth_deg) * (1.0 / SKY_EXPOSURE)
}

/// **The phase function of the air and the droplets together**, weighted by what each scatters —
/// `voxel_light::solver::mixed_phase` in the reduced form `atmosphere.wgsl` and
/// [`sky_exposed_at`] both evaluate.
fn mixed_phase(
    rayleigh_od: f64,
    aerosol_od: f64,
    droplet_scatter_per_m: f64,
    cos_theta: f64,
    mie_asymmetry: f64,
    droplet_asymmetry: f64,
) -> f64 {
    let total = rayleigh_od + aerosol_od + droplet_scatter_per_m;
    if total <= 0.0 {
        return 0.0;
    }
    let rayleigh = 3.0 / (16.0 * core::f64::consts::PI) * (1.0 + cos_theta * cos_theta);
    let air = rayleigh_od * rayleigh
        + aerosol_od * henyey_greenstein(cos_theta, mie_asymmetry);
    let droplets = droplet_scatter_per_m * henyey_greenstein(cos_theta, droplet_asymmetry);
    (air + droplets) / total
}

/// Henyey–Greenstein at one asymmetry: `(1 − g²)/(4π(1 + g² − 2g·cosθ)^1.5)`.
fn henyey_greenstein(cos_theta: f64, g: f64) -> f64 {
    let denominator = 1.0 + g * g - 2.0 * g * cos_theta;
    if denominator <= 0.0 {
        return 0.0;
    }
    (1.0 - g * g) / (4.0 * core::f64::consts::PI * denominator.powf(1.5))
}


/// **A ray's place in the sky's own frame**: `(elevation, azimuth)` in degrees, in the
/// convention [`sun_direction`], [`sky_exposed_at`] and `sky.wgsl` share.
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

    /// **What the day and the night are made of**, as numbers, because the frame is not.
    ///
    /// The reported symptom is that the world stops rendering at nightfall. Every candidate cause
    /// is a quantity that can be printed here — the Sun's own light, the Moon's, the two dome
    /// radiances the sky pass draws, the rig the terrain is lit with, and whether the light field
    /// builds at all — so this measures them at a day tick and a night tick side by side instead of
    /// reasoning about which one is zero. A measurement, not an assertion.
    #[test]
    #[ignore = "measurement, not an assertion: run with `cargo test -p voxel-client --lib measure_the_day_and_the_night -- --ignored --nocapture`"]
    fn measure_the_day_and_the_night() {
        let spec = WorldSpec::new(crate::GAME_SEED, voxel_shapers::Config::default());
        let inputs = SkyInputs::default();
        let peak = peak_sun_luminance(&spec, inputs);
        println!("peak sun luminance (the model's own normaliser): {peak:e}");
        // **The nights of a whole lunar month, at midnight**, which is the hour the Sun is
        // deepest and the Moon is only up near the anti-solar point — so this walks the month
        // from new Moon to new Moon and crosses every phase, rather than sampling two nights
        // and calling a new Moon "the night".
        const TICKS_PER_DAY: u64 = 24_000;
        for day in 0..30u64 {
            let tick = day * TICKS_PER_DAY;
            let sky = Sky::at(&spec, tick, inputs, 0).expect("an atmosphere");
            let moon = voxel_shapers::sky::moon_position(&spec, tick, 0);
            let daylight = daylight_of(&spec, tick, inputs, 0);
            let light = WorldLight::of(&spec, tick, inputs, 0);
            let moon_lum = sky.moon.luminance();
            let ambient = daylight.as_ref().map(|d| d.rig.ambient.luminance()).unwrap_or(f64::NAN);
            let lights = daylight.as_ref().map(|d| d.rig.lights.len()).unwrap_or(0);
            println!(
                "day {day:2} tick {tick:7}  sun {:.2}deg  moon alt {:.2}deg phase {:.3}  \
                 moon_lum {moon_lum:e}  lights {lights}  ambient {ambient:e}  field {}",
                sky.sun_altitude_deg,
                moon.altitude_deg,
                moon.phase_fraction,
                light.as_ref().map(|l| l.beam_is_moon()).unwrap_or(false),
            );
        }

        // **And the two ends a gain has to span.** A night's ambient is only a number beside the
        // day's: the exposure that lifts a night may not touch noon, so both are printed here, and
        // the sky's own exposed radiance beside them — the pass's output, not the ambient's input,
        // because it is the pixels a frame is judged by.
        let (altitude, noon) = highest_noon_tick(&spec);
        let mut sample = |label: &str, tick: u64| {
            let sky = Sky::at(&spec, tick, inputs, 0).expect("an atmosphere");
            let air = crate::sun::air_at(&spec, 0.0, 0.0, tick);
            let moon = voxel_shapers::sky::moon_position(&spec, tick, 0);
            let model = SkyModel::of(&sky, &air, peak, &moon);
            let azimuth = sky_azimuth_deg(&spec, tick);
            let ambient = daylight_of(&spec, tick, inputs, 0)
                .map(|d| d.rig.ambient.luminance())
                .unwrap_or(f64::NAN);
            println!(
                "{label:>18} tick {tick:7}  sun {:.2}deg  ambient {ambient:.4e}  \
                 sky zenith {:.4e}  sky 30deg {:.4e}  sky horizon {:.4e}",
                sky.sun_altitude_deg,
                sky_exposed_at(&model, 90.0, azimuth).luminance(),
                sky_exposed_at(&model, 30.0, azimuth).luminance(),
                sky_exposed_at(&model, 0.0, azimuth).luminance(),
            );
        };
        println!("highest noon {altitude:.2}deg (tick {noon}), and the two midnights:");
        sample("highest noon", noon);
        sample("moonless midnight", 0);
        sample("full-moon midnight", 312_000);
    }
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
        // What the user saw on screen — docs/lighting-sky-and-moon.md
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
        //
        // **The witness is noon against midnight**, and it is a stronger pair than the
        // morning-versus-noon one it replaces. Both sources of the ambient now read one stencil
        // ([`AMBIENT_FROM_HORIZON`]), and a morning and a noon are two *bright* skies in it: which
        // of them wins is a property of the stencil's own directions, not of the law this test is
        // about. The day's own ends are not like that — midnight is the night sky's named floor and
        // noon is the peak of the day, so their ratio is the law's, and a pinned or constant ambient
        // cannot pass it.
        let noon = daylight_of(&world, per_day / 2, Default::default(), 0).expect("noon");
        let midnight = daylight_of(&world, 0, Default::default(), 0).expect("midnight");
        let ratio = noon.rig.ambient.luminance() / midnight.rig.ambient.luminance();
        assert!(
            ratio > 50.0,
            "the ambient ran {ratio} from midnight to noon ({} to {}), so it is not the sky's light \
             and the day and the night are lit by one number",
            midnight.rig.ambient.luminance(),
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
        /// **Several places, not one, and the reason is in the shaper.** `precipitation_mm`
        /// has two branches: a downslope one that returns `BASE * moisture * 0.35` with
        /// **no tick in it at all**, and an upslope one carrying the seasonal term. The
        /// first version of this test sampled `(0, -96)`, where the wind descends — a rain
        /// shadow — and measured a turbidity that varied by exactly `0.0` and concluded
        /// the weather was not reaching the sky. The *mean* was not reaching that column
        /// and still does not; since C-94.7 the onset is what moves it, and the test below
        /// (`the_onset_reaches_the_sky_even_where_the_mean_never_moves`) holds that to a
        /// column of exactly that kind. A sky sampled at one point is still not the
        /// world's sky, which is why the places stay.
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
        // **The whole world, not a patch of it**: a convective column is rare, and the claim is about this world's columns.
        let mut cloudy: Vec<f64> = Vec::new();
        for t in [0u64, quarter, quarter * 2, quarter * 3] {
            for x in (-4_000..4_096).step_by(101) {
                for z in (-4_000..4_096).step_by(101) {
                    let path = voxel_shapers::clouds::column_water_path_m(
                        &world, x as f64, z as f64, t,
                    );
                    if path > 0.0 {
                        cloudy.push(path);
                    }
                }
            }
        }
        cloudy.sort_by(f64::total_cmp);
        assert!(
            !cloudy.is_empty(),
            "no sampled column carried any water, so the clamp was never exercised"
        );
        let deepest = cloudy[cloudy.len() - 1];
        let typical = cloudy[cloudy.len() / 2];
        // **The path the law itself saturates at, from its own three constants** — it was `deepest / 4.0`, a divisor guessed here.
        let saturating = (TURBIDITY_CEILING / CLEAR_AIR_TURBIDITY - 1.0)
            * WATER_PATH_REFERENCE_M
            / WET_TURBIDITY_GAIN;
        println!(
            "turbidity over {} cloudy samples: the typical column is {typical:.1} m (T = {:.2}), \
             the deepest {deepest:.1} m (T = {:.2}); the law saturates at {saturating:.1} m",
            cloudy.len(),
            turbidity_of(typical, &air),
            turbidity_of(deepest, &air),
        );
        assert!(
            deepest > saturating,
            "the world's deepest column is {deepest:.1} m, under the {saturating:.1} m the law \
             saturates at, so the clamp is never reached"
        );
        assert_eq!(
            turbidity_of(deepest, &air),
            TURBIDITY_CEILING,
            "the world's deepest column ({deepest:.1} m) is not held at the ceiling"
        );
        assert!(
            turbidity_of(typical, &air) < TURBIDITY_CEILING,
            "the world's typical cloudy column ({typical:.1} m) sits on the ceiling too, so the \
             clamp is holding the whole sky rather than the deep end of it"
        );
        let clear = turbidity_of(0.0, &air);
        assert!(
            (CLEAR_AIR_TURBIDITY..TURBIDITY_CEILING).contains(&clear),
            "clear air read {clear}, so the clamp is clipping ordinary columns"
        );
    }

    /// **The frame's exposure grade follows the light and crosses a sunset without a step.**
    ///
    /// Three claims, and each is one the eye could not check on its own:
    ///
    /// * while the Sun is up the grade is **exactly `1.0`**, so every noon number in this file is
    ///   untouched — the exposure that lifts a night may not touch noon;
    /// * a night's grade is the ratio of the Sun's light to the Moon's on that same world, which is the
    ///   `~1e-6` the world's own light carries, so the night is exposed for the Moon it has;
    /// * and tick by tick through a sunset, over the horizon and on into the night, **no step is larger
    ///   than a declared fraction of the grade itself**. The worst step is printed, so the bound below
    ///   is a reading rather than a hope, and the two ticks either side of the horizon are printed
    ///   beside the Sun's own illuminance, because a light that steps cannot be tracked smoothly by any
    ///   exposure and that would be this test's finding rather than its silence.
    #[test]
    fn the_exposure_grade_follows_the_light_across_a_sunset_without_a_step() {
        /// The largest step one tick may take in the grade, as a share of the grade's own value.
        ///
        /// **A factor of 1.5 in a tick is a jump and 15 % is not**, and the observed worst step over a
        /// sunset is printed below: this bound is deliberately loose enough that a *fast* twilight passes
        /// and a *step* does not, so the number is not a tolerance tuned until the test went green.
        const GRADE_STEP: f64 = 0.5;

        let world = spec();
        let inputs = SkyInputs::default();
        let day = voxel_shapers::climate::TICKS_PER_DAY;
        let start = crate::game::GAME_TICK;
        let sunset = (1..day)
            .map(|t| start + t)
            .find(|t| {
                let (before, now) = (
                    voxel_shapers::sky::solar_position(&world, t - 1).altitude_deg,
                    voxel_shapers::sky::solar_position(&world, *t).altitude_deg,
                );
                before >= 0.0 && now < 0.0
            })
            .expect("the days after the game's own tick have a sunset in them");

        let noon = daylight_of(&world, start, inputs, 0).expect("a noon");
        assert_eq!(
            noon.grade, 1.0,
            "the Sun is up at the world's own start tick and the grade is {} rather than 1",
            noon.grade
        );

        // **The mapping this function owns, swept densely.** Its argument is the Sun's own altitude, so
        // a step here would be this function's; the observed worst step is printed, and the bound is a
        // real no-jump claim — a factor of 1.5 in one step would be a jump, 15 % is not.
        let mut worst = 0.0f64;
        let mut previous = exposure_grade(1000.0, 2.0, 1.0e-3);
        for step in 0..200_000 {
            let altitude = 2.0 - step as f64 * 0.0001;
            let grade = exposure_grade(1000.0, altitude, 1.0e-3);
            worst = worst.max((grade - previous).abs() / previous.max(grade));
            previous = grade;
        }

        // **And the frame's own light, printed because it is what steps.** Both bodies are gated to zero
        // at their own horizons, so the world's brightness is `light × grade` across a light that cuts:
        // a sunset is a cut to a black world in one tick, which is the defect the user reported.
        let mut night = 0.0f64;
        let mut light_step = 0.0f64;
        let mut previous_light: Option<f64> = None;
        for t in sunset - 60..sunset + day / 4 {
            let daylight = daylight_of(&world, t, inputs, 0).expect("a sky");
            let light = daylight.sky.sun.luminance();
            if let Some(before) = previous_light {
                light_step = light_step.max((light - before).abs() / before.max(light).max(f64::MIN_POSITIVE));
                if (t as i64 - sunset as i64).abs() <= 1 {
                    println!(
                        "the frame's own light at t{t}: sun {:.4} deg, {light:.4e} W/m2 -> the grade \
                         {:.6e}, {} light(s)",
                        voxel_shapers::sky::solar_position(&world, t).altitude_deg,
                        daylight.grade,
                        daylight.lights
                    );
                }
            }
            if daylight.lights > 0 && !voxel_shapers::sky::SkyPhase::sun_is_up(daylight.sun_altitude_deg)
            {
                night = night.max(daylight.grade);
            }
            previous_light = Some(light);
        }
        assert!(
            light_step > 0.0,
            "this window contains no change of light at all, so it is not the test it claims"
        );
        // **The finding this test cannot fix from here, printed rather than hidden.** The Sun's own
        // *light* is what steps: `Sky::sun` is exactly zero for a Sun below the horizon, so one tick
        // carries 8.48 W/m² and the next carries none, and the world's own brightness is `light ×
        // grade` — which no exposure can hold smooth across a light that steps. The grade below is
        // continuous across it, and the frame is not, so the next piece of this work is the beam's own
        // twilight decay: `twilight_fraction` is the model's own curve for it and the sky's scattering
        // already rides it. Until then a sunset is a cut to a black world, which is the defect the
        // user reported as "the world stops rendering materials at night".
        let (before, after) = (
            daylight_of(&world, sunset - 1, inputs, 0).expect("a sky").sky.sun.luminance(),
            daylight_of(&world, sunset, inputs, 0).expect("a sky").sky.sun.luminance(),
        );
        println!(
            "grade: worst step of the mapping over a swept twilight is {worst:.5} of the grade, and the \
             Sun's own light steps {before:.4e} -> {after:.4e} W/m2 across the horizon — the frame's \
             step and not the grade's, which is the beam's own next piece of work"
        );
        assert!(
            worst <= GRADE_STEP,
            "the grade took a step of {worst:.5} of its own value in one tick, over the {GRADE_STEP} \
             the model may take: an exposure that steps is the jump this grade exists to remove"
        );
        assert!(
            night > 1.0,
            "no night in the window lifted the grade above 1, so a moonlit frame is still exposed as \
             a noon it does not have"
        );
        println!("grade: the brightest moonlit night in the window is {night:.6e} times the day's");
    }

    /// **How far the world's own light may move in one rig update, as a share of the day's peak.**
    ///
    /// A declared bound rather than a tolerance tuned to whatever the code happens to do: `5 %` of
    /// the day's brightest ambient in a single four-tick update is roughly the rate a real sunset
    /// dims at, and it is an order of magnitude below the step it exists to catch — the horizon
    /// crossing that removed **36 %** of it in one update, and 100 % of it a few updates later.
    const SUNSET_STEP_OF_PEAK: f64 = 0.05;

    /// **The light a frame is given, tick by tick, from afternoon through the night.**
    ///
    /// A frame that goes black when the Sun reaches the horizon has exactly one number in it that
    /// stepped, and the whole point of this is that the number is *named* rather than guessed at:
    /// the run prints the altitude, the Sun's own light, the ambient, the turbidity and the grade,
    /// and reports which step was the largest.
    #[test]
    fn the_worlds_light_across_a_sunset() {
        let world = spec();
        let mut previous: Option<(u64, f64, f64, f64, f64)> = None;
        // (step, tick, altitude, from, to, which)
        let mut worst = (0.0f64, 0u64, 0.0f64, 0.0f64, 0.0f64, "");
        let mut rows = Vec::new();
        // **At the rig's own cadence**, which is the cadence the player sees: `Stage::Atmosphere`
        // uploads on `Cadence::Water`, every four ticks. A sweep at any other step is measuring a
        // frame rate the game does not run at, in either direction.
        const CADENCE_TICKS: u64 = 4;
        for tick in (14_500..17_400).step_by(CADENCE_TICKS as usize) {
            let inputs = sky_inputs_at(&world, 0.0, 0.0, tick);
            let Some(day) = daylight_of(&world, tick, inputs, 0) else {
                continue;
            };
            let intensity = day.rig.lights.first().map_or(0.0, |l| l.intensity);
            let ambient = day.rig.ambient.luminance();
            // **The field's own reading of the same sky, evaluated beside the rig's**: the stencil
            // walked with nothing in the way, which is what the field would measure for a cell the
            // world does not occlude. It is here so this sweep *measures* the gap between the two
            // sources of the ambient — the assertion below is made of these two numbers, and
            // [`AMBIENT_FROM_HORIZON`]'s own doc carries them.
            let peak = peak_sun_luminance(&world, inputs);
            let air = crate::sun::air_at(&world, 0.0, 0.0, tick);
            let model = SkyModel::of(
                &day.sky,
                &air,
                peak,
                &voxel_shapers::sky::moon_position(&world, tick, 0),
            );
            let mut stencil = LinearRgb::BLACK;
            for direction in SKY_STENCIL {
                let (elevation_deg, azimuth_deg) =
                    sky_direction_of(Vec3::new(direction[0], direction[1], direction[2]));
                stencil = stencil + sky_ambient_at(&model, elevation_deg, azimuth_deg);
            }
            let field_reading = stencil.luminance() * (AMBIENT_FROM_HORIZON / SKY_STENCIL.len() as f64);
            rows.push((
                tick,
                day.sun_altitude_deg,
                intensity,
                ambient,
                inputs.turbidity,
                day.grade,
                day.lights,
                field_reading,
            ));
            if let Some((last_tick, last_alt, last_intensity, last_ambient, last_turbidity)) = previous {
                for (step, from, to, which) in [
                    ((intensity - last_intensity).abs(), last_intensity, intensity, "sun light"),
                    ((ambient - last_ambient).abs(), last_ambient, ambient, "ambient"),
                    ((inputs.turbidity - last_turbidity).abs(), last_turbidity, inputs.turbidity, "turbidity"),
                ] {
                    if step > worst.0 {
                        worst = (step, tick, last_alt, from, to, which);
                    }
                }
                let _ = last_tick;
            }
            previous = Some((tick, day.sun_altitude_deg, intensity, ambient, inputs.turbidity));
        }
        println!("tick  alt_deg   sun_light    ambient  turbidity   grade  lights   field(the stencil)");
        for (tick, alt, intensity, ambient, turbidity, grade, lights, field_reading) in &rows {
            // The horizon band, and every 250 ticks either side of it: the whole question is
            // what these numbers do as `alt` crosses zero.
            let near = alt.abs() < 1.5;
            if near {
                println!(
                    "{tick}  {alt:>7.3}  {intensity:>9.6}  {ambient:>9.6}  {turbidity:>9.4}  \
                     {grade:>6.3}  {lights}  {field_reading:>9.6}"
                );
            }
        }
        // **The two sources of the ambient, measured at the crossing.** The frame goes uniformly
        // orange when the key light steps to zero and the ambient is left alone on every surface, so
        // what the ambient *reads* there is the whole of that wash — and the two readings below are
        // at the same tick and the same sky, which is what makes the gap a measurement rather than a
        // claim about one of them.
        let crossing = rows
            .iter()
            .min_by_key(|row| (row.1.abs() * 1.0e6) as i64)
            .expect("a row nearest the horizon");
        println!(
            "nearest the crossing: the Sun at {:.3} deg, the rig's ambient {:.6} against the field's \
             stencil reading {:.6} ({:.2}x)",
            crossing.1,
            crossing.3,
            crossing.7,
            crossing.3 / crossing.7.max(f64::MIN_POSITIVE)
        );
        // **And the high sun, where the two readings are one law**: the change is the *shape* of the
        // day, not its brightness. The sweep's own brightest row is a high sun, and there the two
        // readings are two samplings of a bright dome — which is the bound the rest of the frame's
        // brightness is held to.
        let highest = rows.iter().max_by(|a, b| a.1.total_cmp(&b.1)).expect("a highest sun");
        let ratio = highest.3 / highest.7.max(f64::MIN_POSITIVE);
        println!(
            "highest sun in the sweep: the Sun at {:.3} deg, the rig's ambient {:.6} against the \
             field's stencil reading {:.6} ({ratio:.2}x)",
            highest.1, highest.3, highest.7
        );
        // **And the gap between the readings, asserted as the thing that is owed**: if these two ever
        // meet, the field's own occluded sky can replace the rig's ambient and `fs_main`'s fallback
        // can go. Until then the assertion below is what says so, and `AMBIENT_FROM_HORIZON`'s own
        // doc carries the same numbers.
        assert!(
            crossing.3 > crossing.7 * 100.0,
            "the rig's ambient reads {:.6} at the crossing against the field's stencil {:.6} — the two \
             readings have met, so the field can be the ambient's only source and `daylight_of`'s note \
             is stale",
            crossing.3,
            crossing.7
        );
        println!(
            "worst step: {} {:.6} -> {:.6} ({:+.6}) at tick {} with the Sun at {:.3} deg",
            worst.5, worst.3, worst.4, worst.4 - worst.3, worst.1, worst.2
        );
        // **The claim, from the two numbers it is made of.** The rig's ambient is the world's whole
        // base light: a frame whose ambient is *exactly* zero is a frame with no light in it at all,
        // and one that loses most of what it had in a single update is the sunset the eye reads as
        // the game stopping. Both were true before `voxel_light::solver::path_in_layer` stopped
        // answering the whole air path at exactly the horizon — an empty droplet layer put an
        // eighteen-deep optical depth on the Sun's light in the one tick it crossed, and the ambient
        // fell from `0.189650` to `1.2e-8` and then printed as zero.
        let lowest = rows.iter().map(|r| r.3).fold(f64::INFINITY, f64::min);
        let highest = rows.iter().map(|r| r.3).fold(f64::NEG_INFINITY, f64::max);
        // **The step is measured against the day's own peak, not against the value it came from.**
        // At deep night the ambient is `1e-4` and a relative bound is meaningless there — a
        // hundredth of a thousandth is a 100 % step and an invisible one. What the eye sees is how
        // far the frame moved against the brightest frame in the sweep, and that is what is bound.
        let mut worst_step = 0.0f64;
        let mut stepped_at = 0u64;
        for pair in rows.windows(2) {
            let step = (pair[0].3 - pair[1].3).abs();
            if step > worst_step {
                worst_step = step;
                stepped_at = pair[1].0;
            }
        }
        let share_of_peak = worst_step / highest.max(f64::EPSILON);
        println!(
            "the ambient runs {lowest:.6} .. {highest:.6}; its worst step in one update is \
             {worst_step:.6} = {:.2}% of the day's peak, at tick {stepped_at}",
            share_of_peak * 100.0
        );
        assert!(
            lowest > 0.0,
            "the world's ambient is exactly zero somewhere in the sweep: the frame has no light in \
             it at all, which is the black sunset"
        );
        assert!(
            share_of_peak <= SUNSET_STEP_OF_PEAK,
            "one update moved the world's ambient by {:.2}% of the day's peak light (at tick \
             {stepped_at}), so the frame's own light steps at the horizon rather than decaying \
             through it",
            share_of_peak * 100.0
        );

        // **The two ticks either side of the crossing, taken apart.** The table says *that* the
        // ambient steps; this says which factor in it did.
        for tick in [16_300u64, 16_325, 16_350] {
            let inputs = sky_inputs_at(&world, 0.0, 0.0, tick);
            let Some(day) = daylight_of(&world, tick, inputs, 0) else {
                continue;
            };
            let peak = peak_sun_luminance(&world, inputs);
            let air = crate::sun::air_at(&world, 0.0, 0.0, tick);
            let moon = voxel_shapers::sky::moon_position(&world, tick, 0);
            let model = SkyModel::of(&day.sky, &air, peak, &moon);
            let visible = voxel_light::sun_visible_fraction(
                &SunPath {
                    altitude_deg: day.sun_altitude_deg,
                    azimuth_deg: day.sky.sun_azimuth_deg,
                },
                &MoonPath {
                    altitude_deg: moon.altitude_deg,
                    azimuth_deg: moon.azimuth_deg,
                    illuminated_fraction: moon.phase_fraction,
                },
            );
            println!(
                "tick {tick}: alt {:.4}  twilight(row) {:.6}  sun_visible {:.6}  \
                 moon alt {:.3} phase {:.5}  sky.sun {:.6e}  night floor {:.6e}",
                day.sun_altitude_deg,
                model.row(1)[2],
                visible,
                moon.altitude_deg,
                moon.phase_fraction,
                day.sky.sun.luminance(),
                night_sky_radiance_w_m2_sr(),
            );
            println!(
                "    rows[1] = {:?}  exposure {:.6e}",
                model.row(1),
                model.row(2)[3]
            );
            println!(
                "    ambient: horizon {:?} zenith {:?}",
                sky_ambient_at(&model, 0.0, day.sky.sun_azimuth_deg),
                sky_ambient_at(&model, 90.0, day.sky.sun_azimuth_deg),
            );
            for index in 0..3 {
                let a = model.row(6 + index * 3);
                println!(
                    "    band {index}: extinction {:.4e} sun_power {:.4e} scatter {:.4e}",
                    a[0], a[2], a[1]
                );
            }
        }
    }

    // An assertion that was here, and why it is not — docs/lighting-sky-and-moon.md

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
    fn a_night_has_no_sun_light_and_a_moonlit_ambient() {
        // The absence is the point, and it is the Sun's absence that is the point — docs/lighting-sky-and-moon.md
        let world = spec();
        let moonless = rig_of(&world, 0, Default::default(), 0)
            .expect("the default atmosphere is a legal one");
        assert!(
            moonless.lights.is_empty(),
            "a light was in the rig at a new-moon midnight: {:?}",
            moonless.lights
        );
        let floor = moonless.ambient.luminance();
        assert!(floor > 0.0, "the night sky vanished again: ambient {floor:e}");

        // **The regression, measured**: a full Moon at its highest midnight is brighter, by
        // orders, than the same midnight with the Moon down and new. `docs/light-solver.md`'s own
        // ratio puts the Moon five or six orders under the Sun; what matters here is that the
        // direction of the difference is the Moon's and not a constant's.
        let full = rig_of(&world, 312_000, Default::default(), 0).expect("a legal atmosphere");
        let lit = full.ambient.luminance();
        println!("midnight ambient: new and down {floor:e}, full and 59° up {lit:e}");
        assert!(
            lit > floor * 10.0,
            "a full Moon at midnight lit nothing: {lit:e} against a moonless {floor:e}"
        );
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
    fn the_model_is_the_solvers_own_transport_at_every_angle() {
        // **The seam that matters for the packing.** The shader's sky and the light rig's sky are
        // two transcriptions of `voxel_light::solver::sky_radiance`, and this holds the rows the
        // shader reads — plus `sky_exposed_at`, the CPU's own reading of them — to the solver
        // itself. A band's power, its extinction, its phase and the response's weights all have to
        // survive the round trip through `f32` rows for the picture to be the model.
        let (spec, tick) = noon_world();
        let inputs = SkyInputs::default();
        let sky = Sky::at(&spec, tick, inputs, 0).expect("a sky at noon");
        let peak = peak_sun_luminance(&spec, inputs);
        let air = crate::sun::air_at(&spec, 0.0, 0.0, tick);
        // **The Moon this model carries is the Moon the solver is given**, and it is why this test
        // means anything: with `None` on the solver's side the comparison passes while the rows are
        // never exercised at all — which is exactly what it did at this tick, where the Moon is new
        // and every one of its rows is zero. `the_lunar_rows_are_the_solvers_own_moonlit_transport`
        // is the tick where they are not.
        let moon = voxel_shapers::sky::moon_position(&spec, tick, 0);
        let moon_body = voxel_light::MoonPath {
            altitude_deg: moon.altitude_deg,
            azimuth_deg: moon.azimuth_deg,
            illuminated_fraction: moon.phase_fraction,
        };
        let model = SkyModel::of(&sky, &air, peak, &moon);
        assert_eq!(model.bytes().len(), SKY_MODEL_BYTES, "the model is not its own row count");
        assert_eq!(
            model.row(3)[3] as usize,
            voxel_light::BAND_COUNT,
            "the model does not carry every band"
        );

        let table = voxel_light::BandTable::computed();
        let sun = voxel_light::SunPath {
            altitude_deg: sky.sun_altitude_deg,
            azimuth_deg: sky.sun_azimuth_deg,
        };
        let exposure = SKY_EXPOSURE / peak;
        let mut worst = 0.0_f64;
        let mut brightest = 0.0_f64;
        for e in 0..=18 {
            let elevation = e as f64 * 5.0;
            for a in (0..360).step_by(15) {
                let azimuth = a as f64;
                let bands =
                    voxel_light::sky_radiance(&table, &air, &sun, Some(&moon_body), elevation, azimuth)
                        .expect("a computable sky");
                // The client's own convention, stated once: the bands carry the magnitude, the
                // response the shape, and one exposure covers the sky, the disc and the night.
                let total: f64 = bands.iter().sum();
                let shape = voxel_light::visible_rgb(&bands);
                let want = LinearRgb {
                    r: shape[0] * total * exposure,
                    g: shape[1] * total * exposure,
                    b: shape[2] * total * exposure,
                };
                let got = sky_exposed_at(&model, elevation, azimuth);
                worst = worst.max((got.luminance() - want.luminance()).abs());
                brightest = brightest.max(want.luminance());
            }
        }
        assert!(
            brightest > 0.0,
            "the model's noon sky is black, so every comparison below would pass for nothing"
        );
        assert!(
            worst < brightest * 1.0e-4,
            "the rows are {worst} of {brightest} away from the solver's own transport, so the \
             shader would draw a sky the rig was not built from"
        );
        println!(
            "the model is within {:.2e} of the solver over 19 elevations × 24 azimuths; the \
             exposed noon sky peaks at {brightest:.4}",
            worst
        );

        // **And it is Rayleigh's sky**: the zenith is blue-dominant, which is the property the
        // twelve bands produce rather than a colour anyone chose.
        let zenith = sky_exposed_at(&model, 85.0, sky.sun_azimuth_deg + 90.0);
        assert!(
            zenith.b > zenith.r,
            "the exposed zenith is ({:.4}, {:.4}, {:.4}), which is not a blue sky",
            zenith.r,
            zenith.g,
            zenith.b
        );
    }

    /// **The lunar rows, at the tick where they are not zero.**
    ///
    /// The Moon's own two rows are the newest thing the shader reads, and the test above cannot
    /// prove they are right: its world's noon is a new moon, so `moon_sky_power` is exactly zero
    /// and the rows cancel out of the comparison. Here the Moon is **59° up and 96.5 % lit** — the
    /// tick `a_night_has_no_sun_light_and_a_moonlit_ambient` measures the ambient peak on — so the
    /// shader's reduced form and the solver's closed form have to agree *through the lunar term*,
    /// with the row's own column, lit fraction and phase angle all live. It asserts the rows are
    /// non-trivial first, so it cannot pass by measuring nothing.
    #[test]
    fn the_lunar_rows_are_the_solvers_own_moonlit_transport() {
        let spec = spec();
        let tick = 312_000u64;
        let inputs = SkyInputs::default();
        let sky = Sky::at(&spec, tick, inputs, 0).expect("a sky");
        let peak = peak_sun_luminance(&spec, inputs);
        let air = crate::sun::air_at(&spec, 0.0, 0.0, tick);
        let moon = voxel_shapers::sky::moon_position(&spec, tick, 0);
        let model = SkyModel::of(&sky, &air, peak, &moon);
        assert!(moon.altitude_deg > 30.0, "the Moon is not up: {}°", moon.altitude_deg);
        assert!(
            model.row(4)[2] > 0.5,
            "the lunar row's lit fraction arrives as {}",
            model.row(4)[2]
        );
        assert!(model.row(4)[0] > 0.0, "the Moon's own air path is empty");
        assert!(model.row(5)[1] > 0.5, "the Moon's own direction is not up");

        let table = voxel_light::BandTable::computed();
        let sun = voxel_light::SunPath {
            altitude_deg: sky.sun_altitude_deg,
            azimuth_deg: sky.sun_azimuth_deg,
        };
        let body = voxel_light::MoonPath {
            altitude_deg: moon.altitude_deg,
            azimuth_deg: moon.azimuth_deg,
            illuminated_fraction: moon.phase_fraction,
        };
        let exposure = SKY_EXPOSURE / peak;
        let mut worst = 0.0_f64;
        let mut brightest = 0.0_f64;
        for elevation in [5.0, 25.0, 60.0, 85.0] {
            for azimuth in [0.0, 90.0, 180.0, 270.0] {
                let bands =
                    voxel_light::sky_radiance(&table, &air, &sun, Some(&body), elevation, azimuth)
                        .expect("a computable sky");
                let total: f64 = bands.iter().sum();
                let shape = voxel_light::visible_rgb(&bands);
                let want = LinearRgb {
                    r: shape[0] * total * exposure,
                    g: shape[1] * total * exposure,
                    b: shape[2] * total * exposure,
                };
                let got = sky_exposed_at(&model, elevation, azimuth);
                worst = worst.max((got.luminance() - want.luminance()).abs());
                brightest = brightest.max(want.luminance());
            }
        }
        assert!(brightest > 0.0, "the moonlit sky is black, so nothing was compared");
        assert!(
            worst < brightest * 1.0e-4,
            "at a moonlit midnight the rows are {worst} of {brightest} away from the solver, so \
             the shader would draw a lunar sky the light model was not built from"
        );
        println!(
            "a moonlit midnight: the model is within {:.2e} of the solver over 16 directions; the \
             sky peaks at {brightest:.6e}",
            worst
        );
    }

    /// **A second transcription is caught by the shader's own text**, and this is where the two
    /// halves of the sky meet: `atmosphere.wgsl` is the only file that carries the transport, and
    /// both passes that draw light are compiled with it exactly once.
    #[test]
    fn the_transport_is_one_file_and_both_passes_receive_it() {
        let transport = include_str!("atmosphere.wgsl");
        for item in ["fn sky_radiance_rgb", "fn mixed_phase", "fn air_mass"] {
            assert_eq!(
                transport.matches(item).count(),
                1,
                "atmosphere.wgsl should define `{item}` once"
            );
        }
        for (name, pass) in [
            ("sky.wgsl", include_str!("sky.wgsl")),
            ("shading.wgsl", include_str!("shading.wgsl")),
        ] {
            for item in ["fn sky_radiance_rgb", "fn mixed_phase", "fn air_mass"] {
                assert_eq!(
                    pass.matches(item).count(),
                    0,
                    "{name} carries its own `{item}` rather than reaching through atmosphere.wgsl"
                );
            }
        }
        assert_eq!(
            transport.matches("@group(0) @binding(5)").count(),
            1,
            "the model's binding should be declared once, in the one file both passes share"
        );
    }

    /// **A host that states nothing is refused, and a stated absence is not a mistake.**
    ///
    /// The shader's own refusal is one layer too far down to be driven from here: the buffer a
    /// host binds is always the declared length, so `arrayLength(&sky_model)` cannot see a host
    /// that never uploaded anything. This drives the decision such a host is read against, and
    /// the model's own row count is what the admitted case is asked through — a constant that
    /// agreed with itself would prove nothing.
    #[test]
    fn a_sky_that_was_never_stated_is_refused_before_a_frame_can_read_it() {
        assert_eq!(admit_sky_rows(None), Err(SkyModelRefusal::NothingStated));
        assert_eq!(admit_sky_rows(Some(NO_SKY_ROWS)), Ok(NO_SKY_ROWS));
        assert_eq!(NO_SKY_ROWS, 0, "zero rows is the absence, not a model's first row");
        for wrong in [SKY_MODEL_ROWS - 1, SKY_MODEL_ROWS + 1] {
            assert_eq!(
                admit_sky_rows(Some(wrong)),
                Err(SkyModelRefusal::RowCount { stated: wrong, declared: SKY_MODEL_ROWS }),
                "{wrong} rows was admitted and the layout is {SKY_MODEL_ROWS}"
            );
        }
        assert!(
            SkyModelRefusal::NothingStated.to_string().contains("no sky has been stated"),
            "the refusal a host reads does not say what is missing"
        );

        // And the admitted count is the one a **real model** states, not the constant beside it.
        let (spec, tick) = noon_world();
        let inputs = SkyInputs::default();
        let sky = Sky::at(&spec, tick, inputs, 0).expect("a sky at noon");
        let air = crate::sun::air_at(&spec, 0.0, 0.0, tick);
        let model = SkyModel::of(
            &sky,
            &air,
            peak_sun_luminance(&spec, inputs),
            &voxel_shapers::sky::moon_position(&spec, tick, 0),
        );
        assert_eq!(admit_sky_rows(Some(model.rows())), Ok(SKY_MODEL_ROWS));
        assert_eq!(
            model.bytes().len(),
            model.rows() * 16,
            "a model's bytes are not the rows it claims to carry"
        );
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
        // And it reads the sky the pass draws: south at 45° is a real sky and not the
        // no-model case, which is black.
        let (spec, tick) = noon_world();
        let inputs = SkyInputs::default();
        let sky = Sky::at(&spec, tick, inputs, 0).expect("a sky");
        let peak = peak_sun_luminance(&spec, inputs);
        let air = crate::sun::air_at(&spec, 0.0, 0.0, tick);
        let model = SkyModel::of(&sky, &air, peak, &voxel_shapers::sky::moon_position(&spec, tick, 0));
        let reflected = sky_exposed_at(&model, elevation, azimuth);
        assert!(reflected.luminance() > 0.0, "the reflection is black at noon");
        // **And with no model, the reflection is nothing at all** — the frame this pass drew
        // before a sky existed, rather than a guess at one.
        assert_eq!(
            sky_exposed_at(
                &SkyModel::of(&sky, &air, 0.0, &voxel_shapers::sky::moon_position(&spec, tick, 0)),
                elevation,
                azimuth
            ),
            LinearRgb::BLACK,
            "a model with no exposure behind it drew a sky"
        );
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

    // ---- the world's own light field --------------------------------------------
    //
    // `WorldLight` is the per-cell measurement the shading model reads as `light_present`.
    // Three of its claims a picture cannot show are pinned here: that its beam is the solver's
    // own (the Sun by day, and the Sun reflected by the Moon at night), that its direction is
    // the **rig's own first light** rather than a second opinion about where the light is, and
    // that the world's own octree is what decides how much of either reaches a cell.

    /// **A night the model itself calls lit**: the Sun down and the Moon **up and past half
    /// phase**, so the beam is the Moon's own light arriving at a body an eye can see.
    ///
    /// **The Moon's altitude is part of the fixture now, and that is what the orbit changed.**
    /// While the Moon was drawn at a declared zenith it was up at every hour of every night, so a
    /// "lit night" needed only the Sun's altitude; a body on an orbit sets, and a night with the
    /// waning crescent below the horizon is a night with no moonlight in it at all. Scanned by day
    /// over the model's own year, so the tick is the world's and not one picked to make the picture
    /// come out.
    fn a_lit_night(world: &WorldSpec) -> u64 {
        let ticks_per_day = voxel_shapers::climate::TICKS_PER_DAY;
        for day in 0..voxel_shapers::climate::DAYS_PER_YEAR {
            let start = day * ticks_per_day;
            for t in start..start + ticks_per_day {
                let sun = voxel_shapers::sky::solar_position(world, t).altitude_deg;
                if sun > -6.0 {
                    continue;
                }
                let moon = voxel_shapers::sky::moon_position(world, t, 0);
                if moon.altitude_deg > 10.0 && moon.phase_fraction > 0.5 {
                    return t;
                }
            }
        }
        panic!("the model's own year has no lit night in it");
    }

    /// A chunk of the shipped world, with the source that answers its addresses — found by asking
    /// the world where its ground is rather than by naming a chunk, the same way
    /// `voxel_stream::light`'s own end-to-end test finds it.
    fn a_generated_chunk() -> (voxel_stream::WorldSource, voxel_model::ChunkPos, SparseVoxelOctree) {
        use voxel_model::CHUNK_SIDE;
        let world = spec();
        let source = voxel_stream::WorldSource::new(world);
        let (x, z) = (0, -95);
        let mut surface_y = None;
        for y in -32..128 {
            let solid = source
                .material_at(x, y, z)
                .map(|material| !material.is_empty())
                .unwrap_or(false);
            if solid {
                surface_y = Some(y);
            }
        }
        let surface_y = surface_y.expect("the spawn column has ground under it");
        let chunk = voxel_model::ChunkPos::new(
            x.div_euclid(CHUNK_SIDE),
            surface_y.div_euclid(CHUNK_SIDE),
            z.div_euclid(CHUNK_SIDE),
        );
        let tree = source.build_chunk(chunk).expect("a chunk of the world");
        (source, chunk, tree)
    }

    #[test]
    fn the_fields_beam_is_the_rigs_own_first_light() {
        let world = spec();
        let inputs = SkyInputs::default();
        for tick in [highest_noon_tick(&world).1, a_lit_night(&world)] {
            let field = WorldLight::of(&world, tick, inputs, 0).expect("a field");
            let daylight = daylight_of(&world, tick, inputs, 0).expect("a rig");
            let first = daylight.rig.lights[0];
            // **The shader wraps the field's beam at `rig.lights[0].direction`.** A field measured
            // along a different direction would be lit from where the light is not, and the two
            // numbers are computed from the same tick, so they may not disagree.
            for axis in 0..3 {
                let rig = [first.direction.x as f64, first.direction.y as f64, first.direction.z as f64];
                assert!(
                    (field.toward_beam()[axis] - rig[axis]).abs() < 1e-6,
                    "the field's beam and the rig's first light point different ways on axis \
                     {axis} at tick {tick}: {:?} against {rig:?}",
                    field.toward_beam()
                );
            }
            let sun_is_up =
                voxel_shapers::sky::SkyPhase::sun_is_up(daylight.sun_altitude_deg);
            assert_eq!(
                field.beam_is_moon(),
                !sun_is_up,
                "the beam's body is not the one the Sun's own state names (tick {tick})"
            );
            // One beam, one light: the field measures a single body's light, so a rig carrying a
            // second would be a light the field has no answer for.
            assert!(
                daylight.rig.lights.len() <= 1,
                "the rig carries {} lights and the field measures one beam",
                daylight.rig.lights.len()
            );
        }
    }

    #[test]
    fn the_nights_beam_is_the_suns_reflected_and_the_octree_says_how_much_arrives() {
        let world = spec();
        let inputs = SkyInputs::default();
        let night = a_lit_night(&world);
        let field = WorldLight::of(&world, night, inputs, 0).expect("a field");
        assert!(field.beam_is_moon(), "the night's beam is not the Moon's");

        // **The beam is the solver's own moonlight**, recomputed here from the same three inputs:
        // the field may not carry a model of the Moon of its own.
        let air = crate::sun::air_at(&world, 0.0, 0.0, night);
        let orbit = voxel_shapers::sky::moon_position(&world, night, 0);
        let body = voxel_light::MoonPath {
            altitude_deg: orbit.altitude_deg,
            azimuth_deg: orbit.azimuth_deg,
            illuminated_fraction: orbit.phase_fraction,
        };
        let expected = voxel_light::moonlight(&voxel_light::BandTable::computed(), &air, &body)
            .expect("the Moon's own light");
        assert_eq!(
            field.beam_bands(),
            expected,
            "the field's night beam is not the solver's moonlight"
        );

        // **And it is reflected sunlight**: five or six orders under the Sun's own beam over the
        // *same air*, which is the published ratio between the two and not a dial on the picture.
        //
        // The comparison is the Sun risen 30° over this cell's own column rather than the Sun at
        // its own noon, and that is on purpose: the noon tick's air may carry a cloud thick enough
        // to extinguish the beam entirely, and a ratio against zero would measure the cloud.
        let risen = voxel_light::direct_beam(
            &voxel_light::BandTable::computed(),
            &air,
            &voxel_light::SunPath { altitude_deg: 30.0, azimuth_deg: 0.0 },
            None,
        )
        .expect("a risen Sun");
        let ratio = expected.iter().sum::<f64>() / risen.iter().sum::<f64>();
        println!(
            "a lit night: the Moon's beam sums to {:.3e}, the same air under a 30° Sun to \
             {:.3e} — a ratio of {ratio:.2e}",
            expected.iter().sum::<f64>(),
            risen.iter().sum::<f64>()
        );
        assert!(
            ratio < 1e-4,
            "the night's beam is {ratio} of the Sun's over the same air, so it is not a reflection"
        );

        // **The world's own octree decides how much of it arrives.** A cell in the open sees the
        // Moon's beam entire (the body is at the zenith in this model, so nothing is between the
        // cell and it); a cell three metres inside the rock sees none of it, and none of the sky
        // either — which is the shadow the rig could not cast, because the rig does not know
        // where a cell is.
        let (source, chunk, tree) = a_generated_chunk();
        let origin = chunk.origin();
        let surface = {
            let mut top = None;
            for y in -32..128 {
                let solid = source
                    .material_at(origin.x + 16, y, origin.z + 16)
                    .map(|material| !material.is_empty())
                    .unwrap_or(false);
                if solid {
                    top = Some(y);
                }
            }
            top.expect("the chunk's centre column has ground under it")
        };
        let cell = |world_y: i32| {
            let world = [(origin.x + 16) as f64 + 0.5, world_y as f64 + 0.5, (origin.z + 16) as f64 + 0.5];
            [
                world[0] - origin.x as f64,
                world[1] - origin.y as f64,
                world[2] - origin.z as f64,
            ]
        };
        let open = field.bands(&source, &tree, cell(surface + 4), 40.0);
        let buried = field.bands(&source, &tree, cell(surface - 3), 40.0);
        println!(
            "a lit night's field in the spawn chunk: open {open:?}\n  three metres into the \
             rock: {buried:?}"
        );
        assert!(
            open[0] > 0.0 && open[1] > 0.0 && open[2] > 0.0,
            "a cell in the open saw none of the Moon's beam: {open:?}"
        );
        assert_eq!(
            [buried[0], buried[1], buried[2]],
            [0.0; 3],
            "a cell inside the rock saw the Moon"
        );
        assert!(
            open[3] > 0.0 && open[4] > 0.0 && open[5] > 0.0,
            "a cell in the open saw none of the dome: {open:?}"
        );
        assert_eq!(
            [buried[3], buried[4], buried[5]],
            [0.0; 3],
            "a cell inside the rock saw the dome"
        );
    }
}