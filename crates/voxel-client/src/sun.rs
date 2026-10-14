//! **The sun as a body in the world** - a place and an extent, not a direction.
//!
//! Its distance is the astronomical unit and its radius is derived from the band table's
//! own solar dilution, so no number here is stated twice.
//!
//! # And the body is what the sky pass draws
//!
//! `sky.wgsl` is handed this body's centre relative to the eye, its half-extent and its own
//! radiance, and tests every pixel's ray against the cuboid - so the Sun on screen is the
//! same object [`BodyCuboid::visibility`] measures a shadow against, rather than a second
//! shape declared for the picture.
//!
//! **Its orientation is the world's axes**, and that follows from a test rather than from a
//! preference: `the_sun_is_one_object_in_the_world_and_not_one_per_observer` pins the body
//! as *one* object, and a cube that turned to face each eye would be a different object for
//! every eye. The consequence is drawn and named: the silhouette is a square when a face is
//! presented to the eye and turns toward a hexagon as the body crosses the sky, which a
//! disc-or-billboard Sun would not do.

use crate::skylight::{sky_exposed, sun_direction};
use voxel_appearance::openpbr::LinearRgb;
use voxel_appearance::sky::Sky;
use voxel_light::SUN_DILUTION;
use voxel_shapers::sky::SolarPosition;
use voxel_shapers::WorldSpec;

/// One astronomical unit, metres (IAU 2012 definition).
pub const AU_M: f64 = 1.495_978_707e11;

/// The sun's radius, metres: `1 AU x sqrt(SUN_DILUTION)`, so it is the same sun the band
/// table radiates from rather than a second stated number.
pub fn sun_radius_m() -> f64 {
    AU_M * SUN_DILUTION.sqrt()
}

/// The sun's angular radius as seen from `distance_m`, radians.
pub fn angular_radius_rad(distance_m: f64) -> f64 {
    if distance_m <= 0.0 {
        return 0.0;
    }
    (sun_radius_m() / distance_m).atan()
}

/// The sun as a body: a centre in world metres and a radius.
///
/// The body is a **cuboid of half-extent `radius_m`**, and its faces are the world's own
/// axes (`x` east, `y` up, `z` south). See this module's doc for why the orientation is the
/// world's rather than the eye's.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct BodyCuboid {
    pub centre: [f64; 3],
    pub radius_m: f64,
}

/// **The solid angle a face of the body covers from `distance_m`**: the face's area over the
/// square of the distance, `(2R/d)²`.
///
/// This is the divisor that turns the Sun's own **irradiance** into the **radiance** of its
/// faces - power per square metre into power per square metre per steradian - and it is the
/// number a drawn body needs and an irreducible-flux rig does not.
///
/// The body's own solid angle, not a disc's: `4·SUN_DILUTION` against the real solar disc's
/// `π·SUN_DILUTION`, 27 % larger, which is the corner a square has and a circle has not.
pub fn solid_angle_sr(distance_m: f64) -> f64 {
    if distance_m <= 0.0 {
        return 0.0;
    }
    let ratio = sun_radius_m() / distance_m;
    4.0 * ratio * ratio
}

/// Why a sun could not be built, each named so a reader knows what was rejected.
#[derive(Clone, PartialEq, Debug)]
pub enum SunRefusal {
    NotFinite { what: &'static str, value: f64 },
    BelowHorizon { altitude_deg: f64 },
}

impl std::fmt::Display for SunRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SunRefusal::NotFinite { what, value } => {
                write!(f, "the sun's {what} is {value}, which is not a position")
            }
            SunRefusal::BelowHorizon { altitude_deg } => write!(
                f,
                "the sun is {altitude_deg}° below the horizon, so there is no sun to light with"
            ),
        }
    }
}

/// The unit vector **toward** the sun, in the frame the light rig and the sky shader share:
/// `x` east, `y` up, `z` south, the azimuth measured from north and increasing eastwards.
///
/// **This delegates rather than repeats, and it was wrong before it did.** The body's
/// direction used to be formed here as `(cos δ·cos A, sin δ, cos δ·sin A)` - an azimuth
/// measured from `+x` toward `+z`, which is the compass azimuth turned through a quarter
/// turn. Two spellings of one angle is exactly the defect [`crate::skylight::sun_direction`]
/// exists to prevent, and it stayed invisible while nothing drew the body: the rig moved
/// with the tick and the body stood ninety degrees away from it, in the dark.
pub fn direction_to(p: &SolarPosition) -> [f64; 3] {
    let d = sun_direction(p.altitude_deg, p.azimuth_deg);
    [d.x, d.y, d.z]
}

impl BodyCuboid {
    /// **The body's place, geometry only**: one astronomical unit along the direction the two
    /// angles name, from `world_anchor`.
    ///
    /// No horizon rule here - *where the body is* and *whether a shadow can be cast from it*
    /// are two questions, and [`BodyCuboid::at_tick`] is the one that answers the second.
    /// [`DrawnSun::of`] wants the first, because a drawn body exists exactly when the rig has
    /// a light in it, and `SkyPhase::sun_is_up` reaches 0.833° below the horizon for
    /// refraction.
    pub fn at(
        world_anchor: [f64; 3],
        altitude_deg: f64,
        azimuth_deg: f64,
    ) -> Result<Self, SunRefusal> {
        for v in world_anchor.iter().chain([altitude_deg, azimuth_deg].iter()) {
            if !v.is_finite() {
                return Err(SunRefusal::NotFinite { what: "world anchor or angle", value: *v });
            }
        }
        let d = direction_to(&SolarPosition {
            altitude_deg,
            azimuth_deg,
            declination_deg: 0.0,
            hour_angle_deg: 0.0,
        });
        Ok(BodyCuboid {
            centre: [
                world_anchor[0] + d[0] * AU_M,
                world_anchor[1] + d[1] * AU_M,
                world_anchor[2] + d[2] * AU_M,
            ],
            radius_m: sun_radius_m(),
        })
    }

    /// **The sun for this world at this tick**, refused below the horizon: how much of a sun
    /// that has set can a point see is zero, and a body lighting the world from underneath
    /// would be a worse lie than no body.
    pub fn at_tick(world_anchor: [f64; 3], solar: &SolarPosition) -> Result<Self, SunRefusal> {
        if solar.altitude_deg <= 0.0 {
            return Err(SunRefusal::BelowHorizon { altitude_deg: solar.altitude_deg });
        }
        Self::at(world_anchor, solar.altitude_deg, solar.azimuth_deg)
    }

    /// **How much of this sun `from` can see**, in `[0, 1]`. `is_blocked` takes the origin
    /// and a ray, so the same closure serves an octree walk, a coarse grid and a test, and
    /// the penumbra falls out of the sampling rather than being a knob.
    pub fn visibility(
        &self,
        from: [f64; 3],
        samples: usize,
        is_blocked: impl Fn([f64; 3], [f64; 3]) -> bool,
    ) -> f64 {
        if samples == 0 {
            // No samples is not "fully lit": it is no measurement, and a shadow should not be
            // asserted without evidence.
            return 0.0;
        }
        let to_centre = [
            self.centre[0] - from[0],
            self.centre[1] - from[1],
            self.centre[2] - from[2],
        ];
        let distance = (to_centre[0].powi(2) + to_centre[1].powi(2) + to_centre[2].powi(2)).sqrt();
        if distance <= 0.0 {
            return 0.0;
        }
        let forward = [
            to_centre[0] / distance,
            to_centre[1] / distance,
            to_centre[2] / distance,
        ];
        // Two axes perpendicular to `forward`, from the world axis least aligned with it so
        // the cross product cannot degenerate. An earlier version crossed `forward` with
        // **itself**, which is the zero vector, and so every call answered "no sun visible".
        let mut least = 0usize;
        if forward[1].abs() < forward[least].abs() {
            least = 1;
        }
        if forward[2].abs() < forward[least].abs() {
            least = 2;
        }
        let mut other = [0.0f64; 3];
        other[least] = 1.0;
        let mut right = [
            forward[1] * other[2] - forward[2] * other[1],
            forward[2] * other[0] - forward[0] * other[2],
            forward[0] * other[1] - forward[1] * other[0],
        ];
        let mut up = [
            forward[1] * right[2] - forward[2] * right[1],
            forward[2] * right[0] - forward[0] * right[2],
            forward[0] * right[1] - forward[1] * right[0],
        ];
        for v in [&mut right, &mut up] {
            let n = (v[0].powi(2) + v[1].powi(2) + v[2].powi(2)).sqrt();
            if n == 0.0 {
                return 0.0;
            }
            for c in v.iter_mut() {
                *c /= n;
            }
        }
        let r = self.radius_m;
        let mut visible = 0usize;
        for s in 0..samples {
            // A ring rather than a line, so the samples do not all land on one diameter.
            let (radius, angle) = if s == 0 {
                (0.0, 0.0)
            } else {
                (r * (s as f64 / samples as f64).sqrt(), s as f64 * 2.399_963_229_728_653)
            };
            let (c, n) = (angle.cos(), angle.sin());
            let target = [
                self.centre[0] + (right[0] * c + up[0] * n) * radius,
                self.centre[1] + (right[1] * c + up[1] * n) * radius,
                self.centre[2] + (right[2] * c + up[2] * n) * radius,
            ];
            let ray = [target[0] - from[0], target[1] - from[1], target[2] - from[2]];
            if !is_blocked(from, ray) {
                visible += 1;
            }
        }
        visible as f64 / samples as f64
    }

    /// **Whether a ray from `from` meets this body** - the slab test `sky.wgsl` performs,
    /// written here so a test can hold the shader to a number rather than to a description.
    ///
    /// A ray parallel to a slab plane and outside that slab must miss, and it does: the
    /// reciprocal is taken with a floor under the ray's own magnitude so that no division by
    /// zero produces an infinity that `0 · ∞` would turn into a `NaN`, and a parallel axis
    /// then puts a **positive** infinity into the near bound, which no far bound from the
    /// other axes can be greater than. A body one astronomical unit away can never contain
    /// the eye, so "the origin is inside the slab" is not a case this has to answer.
    pub fn ray_hits(&self, from: [f64; 3], ray: [f64; 3]) -> bool {
        let mut near = f64::NEG_INFINITY;
        let mut far = f64::INFINITY;
        for axis in 0..3 {
            let magnitude = ray[axis].abs().max(1.0e-30);
            let sign = if ray[axis] >= 0.0 { 1.0 } else { -1.0 };
            let inv = sign / magnitude;
            let offset = self.centre[axis] - from[axis];
            let lo = (offset - self.radius_m) * inv;
            let hi = (offset + self.radius_m) * inv;
            near = near.max(lo.min(hi));
            far = far.min(lo.max(hi));
        }
        far >= near.max(0.0)
    }

    /// The penumbra's width at an occluder `occluder_distance_m` from the point: two real
    /// numbers in, and what falls out is metres.
    pub fn penumbra_width_m(&self, from: [f64; 3], occluder_distance_m: f64) -> f64 {
        let to_centre = [
            self.centre[0] - from[0],
            self.centre[1] - from[1],
            self.centre[2] - from[2],
        ];
        let distance = (to_centre[0].powi(2) + to_centre[1].powi(2) + to_centre[2].powi(2)).sqrt();
        2.0 * occluder_distance_m * angular_radius_rad(distance)
    }
}

/// **The Sun as a frame draws it** (C-184): the body, and the radiance of its faces — carried into the
/// frame as a [`crate::quad::BodyRecord`] by [`DrawnSun::body_record`] rather than as a uniform.
///
/// # Why the radiance is not simply `Sky::sun`
///
/// [`Sky::sun`] is an **irradiance** - the beam's power arriving per square metre of ground,
/// W/m². A body drawn in the sky carries a **radiance** - power per square metre per
/// steradian - and the two differ by exactly the solid angle the body covers, [`solid_angle_sr`].
/// Skipping the division would draw a face about a hundred thousand times too dim, and the
/// mistake would look right on screen, because the pass's exposure hides the factor: it is a
/// unit error and not a brightness one.
///
/// # What is declared here rather than sourced
///
/// That the faces are **Lambertian** emissive surfaces, so that the irradiance they deliver is
/// their radiance times their own solid angle. The Sun's photosphere is not Lambertian in
/// detail; what this needs from it is only that the body delivers the flux the band table says
/// it delivers, which is the property the conversion preserves.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct DrawnSun {
    /// The body itself - the same object a shadow's penumbra is measured against.
    pub body: BodyCuboid,
    /// The radiance of its faces, **already through the sky pass's exposure**, so the disc and
    /// the sky it sits in share one scale.
    pub radiance: LinearRgb,
}

impl DrawnSun {
    /// The Sun this frame draws, **from the same `Sky` the light rig was built from**, or
    /// `None` when the Sun is not up.
    ///
    /// **`None` is the Sun's own state, named, and it used to be the *luminance's* zero.** The test
    /// here read `sky.sun.luminance() > 0.0`, which was true exactly while the Sun was up *because
    /// `Sky::sun_light` returned `BLACK` below the horizon* - so the drawn body, the light the Moon
    /// is lit by and the rig's key light were all one switch, and the Sun's features went out in the
    /// same update it crossed. Every one of those is now separate: the Sun's own output keeps the
    /// horizon's air column at every hour below the horizon ([`voxel_appearance::sky::Sky::sun_light`]),
    /// the rig takes the twilight share of it ([`crate::skylight::daylight_of`]), and **the body is
    /// drawn while the Sun is up** - [`voxel_shapers::sky::SkyPhase::sun_is_up`], the same rule the
    /// rig's own membership uses, so the picture and the light cannot drift apart again.
    ///
    /// [`BodyCuboid::at_tick`] is *not* called here, and that is the other half of the separation:
    /// its `BelowHorizon` refusal is the **shadow-casting** rule, and where a body is stands apart
    /// from whether a shadow can be measured from it.
    ///
    /// `peak` is the world's brightest sun, W/m² ([`crate::skylight::peak_sun_luminance`]) -
    /// **the same normaliser the sky table is divided by**, so one exposure covers both. A
    /// second normaliser here would be a second dialect of one field, which is what
    /// `chromaticity`'s own doc refuses.
    ///
    /// `anchor_m` is the point the body is placed a unit of distance from: the eye. The body
    /// is one object in the world and not one per observer, so what the anchor decides is one
    /// astronomical unit of parallax - 4e-8 radians from a six-kilometre world - and not the
    /// direction it is seen from.
    pub fn of(sky: &Sky, peak: f64, anchor_m: [f64; 3]) -> Option<Self> {
        // The Sun's own state, and not a reading of what it emits — see this function's doc.
        if !voxel_shapers::sky::SkyPhase::sun_is_up(sky.sun_altitude_deg) {
            return None;
        }
        // Refused by name, not by how a comparison happens to treat a NaN — docs/lighting-sky-and-moon.md
        let irradiance = sky.sun.luminance();
        if !irradiance.is_finite() || !peak.is_finite() || peak <= 0.0 {
            return None;
        }
        let body = BodyCuboid::at(anchor_m, sky.sun_altitude_deg, sky.sun_azimuth_deg).ok()?;
        Some(DrawnSun {
            body,
            radiance: sky_exposed(sky.sun / (peak * solid_angle_sr(AU_M))),
        })
    }

    /// **The record this body travels as** (C-184): the centre and the half-extent in the world's
    /// **engine metres** — `voxel_shapers::scale`'s 1:8 ruling, applied to the offset from the eye
    /// and to the radius, so what the record subtends is the real angle and the distance it is
    /// carried at is this world's own.
    ///
    /// `light_direction` is `[0, 0, 0]`: a Sun is what it emits, and `fs_body` draws a body lit by
    /// nothing as its own radiance — which is why the Moon's record, not this one, is the phase.
    pub fn body_record(&self, eye_m: [f64; 3]) -> crate::quad::BodyRecord {
        self.body_record_graded(eye_m, 1.0)
    }

    /// [`DrawnSun::body_record`] **at a frame's own exposure grade**: the radiance the record carries
    /// is scaled by it, so the Sun is drawn at the same exposure the sky it stands in and the light it
    /// casts are. A record graded apart from its sky would be a disc left at noon over an exposed night.
    pub fn body_record_graded(&self, eye_m: [f64; 3], grade: f64) -> crate::quad::BodyRecord {
        let mut centre_m = [0.0f64; 3];
        for (axis, slot) in centre_m.iter_mut().enumerate() {
            *slot = eye_m[axis] + voxel_shapers::scale::voxels(self.body.centre[axis] - eye_m[axis]);
        }
        crate::quad::BodyRecord {
            centre_m,
            half_extent_m: voxel_shapers::scale::voxels(self.body.radius_m),
            radiance: [
                self.radiance.r as f32 * grade as f32,
                self.radiance.g as f32 * grade as f32,
                self.radiance.b as f32 * grade as f32,
            ],
            light_direction: [0.0; 3],
        }
    }
}

/// The world's scale height in **real metres** — the unit `voxel_light`'s coefficients are in.
///
/// `voxel_shapers::atmosphere::SCALE_HEIGHT_M` is the same air after the 1:8 ruling, so this is the
/// figure that constant scales rather than a second measurement of the atmosphere.
pub const REAL_SCALE_HEIGHT_M: f64 = 8_500.0;


/// **The air a ray crosses**, from the world's own measurements rather than a caller's guess: the
/// shapers' own density and scale height, and the cloud column's water path at `(x, z)`.
///
/// **The droplets are zero here, and that is declared rather than forgotten**: `voxel_shapers::fog`'s
/// near-surface load is not read in this function yet, so a fog dims the ground without dimming the
/// Moon. The cloud's column is real, so one term is missing and it is named.
pub fn air_at(spec: &WorldSpec, x: f64, z: f64, tick: u64) -> voxel_light::Air {
    let sample = voxel_shapers::atmosphere::sample(spec, 0.0);
    voxel_light::Air {
        ground_density_kg_m3: sample.density_kg_m3,
        scale_height_m: REAL_SCALE_HEIGHT_M,
        droplet_lwc_g_m3: 0.0,
        droplet_layer_depth_m: 0.0,
        cloud_water_path_m: voxel_shapers::clouds::column_water_path_m(spec, x, z, tick),
    }
}

/// **The Moon as a frame draws it** (C-184): a body at the Moon's own distance, and the radiance of its
/// **lit face** — carried into the frame as a [`crate::quad::BodyRecord`] by [`DrawnMoon::body_record`].
///
/// # Why the radiance is the solver's, and why the phase is not here
///
/// The face's radiance is `voxel_light`'s own answer — the Sun's spectrum in each band times the band
/// table's declared reflectance, over π by Lambert's law, transmitted back down the same air column
/// the Sun crossed. So the drawn Moon **is** the Sun reflected, and nothing in this file chooses how
/// bright it is.
///
/// Which part of the body the eye sees lit is **geometry, and the shader does it**: each **face** of
/// the body is shaded by its own normal against the Sun's direction, so a new body, a quarter body and
/// a full one are one equation rather than three pictures. The illuminated fraction that reaches the
/// light rig is the same geometry, integrated over the body.
///
/// # A cuboid where the light is measured, a sphere where it is drawn
///
/// [`BodyCuboid`] is the shape the **photometry** is measured against — the solid angle the Sun's
/// irradiance is divided by, and the body a shadow's penumbra marches toward — and it is the Sun's own
/// type, so the two bodies differ only in the numbers.
///
/// What the eye sees is `shading.wgsl`'s `fs_body`, which draws a **sphere** from the record: a phase
/// is a per-pixel normal, and a cuboid has one normal per visible face, so a cube draws a full moon at
/// every elongation. The one place the two shapes part company in flux is that a face-on square covers
/// `4/π` times the disc inscribed in it, so the drawn disc delivers that much less of the body's own
/// irradiance — named in `docs/lighting-sky-and-moon.md` §7 rather than corrected by a second
/// brightness scale here.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct DrawnMoon {
    /// **The body itself.** Its half-extent is `voxel_light::MOON_RADIUS_KM`'s own figure, so the
    /// body subtends the angle the Moon subtends and not an angle chosen for the picture.
    pub body: BodyCuboid,
    /// The radiance of a **fully lit** face, already through the sky pass's exposure.
    pub radiance: LinearRgb,
    /// **The direction toward the Sun** — the light the shader lights the body's own points with.
    ///
    /// Its own field rather than a reading of the drawn Sun's centre, because at night the Sun's body
    /// is below the horizon and is not drawn at all, and it is *precisely then* that the Moon is up.
    pub light_direction: [f64; 3],
}

impl DrawnMoon {
    /// The Moon this frame draws, or `None` when there is no Moon to draw.
    ///
    /// `moon` is **the orbit's own reading** ([`voxel_shapers::sky::moon_position`]): the altitude
    /// and azimuth the body is placed at, and the elongation the phase comes from. `peak` is the
    /// world's brightest sun (`crate::skylight::peak_sun_luminance`) — **the same normaliser the
    /// sky table is divided by**, so one exposure covers the sky, the Sun and the Moon — and
    /// `anchor_m` the eye the body is placed a lunar distance from.
    ///
    /// **`None` when the body is below the horizon, and it is the solver's own refusal rather
    /// than a second test here**: `voxel_light::moon_radiance` returns all twelve bands zero for a
    /// Moon that has set, so the sum below is zero and the body is not drawn. A Moon the eye
    /// cannot see is not a Moon at reduced brightness.
    pub fn of(
        moon: &voxel_shapers::sky::MoonPosition,
        air: &voxel_light::Air,
        peak: f64,
        anchor_m: [f64; 3],
        sun_altitude_deg: f64,
        sun_azimuth_deg: f64,
    ) -> Option<Self> {
        if !moon.phase_fraction.is_finite() || moon.phase_fraction <= 0.0 {
            return None;
        }
        if !peak.is_finite() || peak <= 0.0 {
            return None;
        }
        let path = voxel_light::MoonPath {
            altitude_deg: moon.altitude_deg,
            azimuth_deg: moon.azimuth_deg,
            illuminated_fraction: moon.phase_fraction,
        };
        let table = voxel_light::BandTable::computed();
        let face = voxel_light::moon_radiance(&table, air, &path).ok()?;
        let total: f64 = face.iter().sum();
        if !total.is_finite() || total <= 0.0 {
            return None;
        }
        // The bands carry the magnitude, the response carries the shape, and both are divided by the
        // same normaliser every other radiance in the sky pass is divided by.
        let chromatic = voxel_light::visible_rgb(&face);
        let magnitude = total / peak;
        // **The Moon's own direction function is the Sun's**, because there is one frame here: an
        // azimuth from north increasing eastwards over `x` east, `y` up, `z` south. A second spelling
        // of that convention is the defect this module already paid for once.
        let direction = sun_direction(moon.altitude_deg, moon.azimuth_deg);
        let to_sun = sun_direction(sun_altitude_deg, sun_azimuth_deg);
        let distance_m = voxel_light::MOON_DISTANCE_KM * 1_000.0;
        Some(DrawnMoon {
            body: BodyCuboid {
                centre: [
                    anchor_m[0] + direction.x * distance_m,
                    anchor_m[1] + direction.y * distance_m,
                    anchor_m[2] + direction.z * distance_m,
                ],
                radius_m: voxel_light::MOON_RADIUS_KM * 1_000.0,
            },
            light_direction: [to_sun.x, to_sun.y, to_sun.z],
            radiance: sky_exposed(LinearRgb {
                r: chromatic[0] * magnitude,
                g: chromatic[1] * magnitude,
                b: chromatic[2] * magnitude,
            }),
        })
    }

    /// **The record this body travels as** (C-184), through the same ruling the Sun's is: engine
    /// metres, and the same negative `f64` subtraction before the narrow, because a lunar distance
    /// does not fit a 32-bit metre and a direction does.
    ///
    /// **Its `light_direction` is the Sun's**, and that one field is the whole of the phase: the
    /// record carries the radiance of a *fully lit* face and the direction that lights it, and which
    /// part of the face the eye sees lit is `fs_body`'s sphere law and not a brightness chosen here.
    pub fn body_record(&self, eye_m: [f64; 3]) -> crate::quad::BodyRecord {
        self.body_record_graded(eye_m, 1.0)
    }

    /// [`DrawnMoon::body_record`] **at a frame's own exposure grade**, through the same scale the Sun's
    /// own record takes: the Moon's face and the sky it is seen against are one frame's exposure.
    pub fn body_record_graded(&self, eye_m: [f64; 3], grade: f64) -> crate::quad::BodyRecord {
        let mut centre_m = [0.0f64; 3];
        for (axis, slot) in centre_m.iter_mut().enumerate() {
            *slot = eye_m[axis] + voxel_shapers::scale::voxels(self.body.centre[axis] - eye_m[axis]);
        }
        crate::quad::BodyRecord {
            centre_m,
            half_extent_m: voxel_shapers::scale::voxels(self.body.radius_m),
            radiance: [
                self.radiance.r as f32 * grade as f32,
                self.radiance.g as f32 * grade as f32,
                self.radiance.b as f32 * grade as f32,
            ],
            light_direction: self.light_direction,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skylight::{daylight_of, peak_sun_luminance};
    use voxel_appearance::sky::{SkyInputs, SUN_ANGULAR_DIAMETER_RAD};
    use voxel_shapers::{climate::TICKS_PER_DAY, Config, WorldSpec};

    fn clear(_from: [f64; 3], _ray: [f64; 3]) -> bool {
        false
    }

    fn moon_air() -> voxel_light::Air {
        voxel_light::Air {
            ground_density_kg_m3: voxel_shapers::atmosphere::SEA_LEVEL_DENSITY_KG_M3,
            scale_height_m: REAL_SCALE_HEIGHT_M,
            droplet_lwc_g_m3: 0.0,
            droplet_layer_depth_m: 0.0,
            cloud_water_path_m: 0.0,
        }
    }

    /// **The drawn Moon is the Sun's own light reflected, and what the shader gets is its face.**
    ///
    /// Four claims, and each is one the wiring could get wrong while still compiling:
    ///
    /// * the body stands at the Moon's own distance with the Moon's own radius, so the disc subtends
    ///   the angle the Moon subtends rather than an angle chosen for the picture;
    /// * its radiance is **in the Sun's own units** — the ratio to the drawn Sun's radiance is
    ///   `reflectance/π · Ω`, which is the same `1e-5` the two bodies are apart in the sky;
    /// * a thicker crescent does **not** dim the face, because the phase is geometry the shader
    ///   computes from the ray and not a brightness multiplied in here — the test that would fail if
    ///   someone moved the phase back into the photometry;
    /// * a new moon draws **no body at all**, rather than a black disc over the stars.
    #[test]
    fn the_drawn_moon_is_the_suns_own_light_reflected_off_a_lit_face() {
        let spec = WorldSpec::new(2468, Config::default());
        let inputs = SkyInputs::default();
        // A tick the model itself calls a lit moon — docs/lighting-sky-and-moon.md
        let month_ticks = voxel_shapers::sky::SYNODIC_MONTH_DAYS * TICKS_PER_DAY as f64;
        let tick = (0..month_ticks as u64)
            .step_by(97)
            .find(|t| {
                let moon = voxel_shapers::sky::moon_position(&spec, *t, 0);
                moon.altitude_deg > 10.0 && moon.phase_fraction > 0.33
            })
            .expect("the model's own month has a lit moon above the horizon in it");
        let day = daylight_of(&spec, tick, inputs, 0).expect("a daylight at the moon's own tick");
        let peak = peak_sun_luminance(&spec, inputs);
        let air = moon_air();
        let eye = [0.0, 0.0, 0.0];
        let orbit = voxel_shapers::sky::moon_position(&spec, tick, 0);
        let moon = DrawnMoon::of(
            &orbit,
            &air,
            peak,
            eye,
            day.sky.sun_altitude_deg,
            day.sky.sun_azimuth_deg,
        )
        .expect("a moon to draw");

        // The body is where the model puts it, at the size the Moon is.
        let distance_m =
            (moon.body.centre[0].powi(2) + moon.body.centre[1].powi(2) + moon.body.centre[2].powi(2))
            .sqrt();
        let expected_distance_m = voxel_light::MOON_DISTANCE_KM * 1_000.0;
        assert!(
            (distance_m - expected_distance_m).abs() < 1.0e3,
            "the body stands at {distance_m} m, not the Moon's own {expected_distance_m}"
        );
        let angular_diameter_deg =
            2.0 * (moon.body.radius_m / distance_m).atan().to_degrees();
        println!("the drawn moon subtends {angular_diameter_deg:.3}° and its face reads {:.3e}",
            moon.radiance.luminance());
        assert!(
            (0.45..0.60).contains(&angular_diameter_deg),
            "the drawn disc subtends {angular_diameter_deg}°, which is not the Moon's own half a 
             degree"
        );

        // **The Sun's own units**, which is the check on the whole exposure: the same normaliser the
        // Sun's body is drawn with divides this one too, so the ratio is the physics and not a second
        // scale someone chose.
        let sun_radiance = sky_exposed(day.sky.sun / (peak * solid_angle_sr(AU_M))).luminance();
        let ratio = moon.radiance.luminance() / sun_radiance;
        println!("the moon's face is {ratio:.3e} of the sun's own radiance");
        assert!(
            (1.0e-6..1.0e-4).contains(&ratio),
            "the drawn moon is {ratio} of the drawn sun, which is not a reflectance over π times a 
             solid angle"
        );

        // The phase is the shader's, so it must not move the face's radiance.
        let crescent_orbit = voxel_shapers::sky::MoonPosition {
            phase_fraction: orbit.phase_fraction * 0.5,
            ..orbit
        };
        let crescent = DrawnMoon::of(
            &crescent_orbit,
            &air,
            peak,
            eye,
            day.sky.sun_altitude_deg,
            day.sky.sun_azimuth_deg,
        )
        .expect("a crescent");
        assert!(
            (crescent.radiance.luminance() - moon.radiance.luminance()).abs() < 1.0e-12,
            "a thinner crescent dimmed the lit face ({} against {}), so the phase is in the 
             photometry rather than in the geometry the shader draws",
            crescent.radiance.luminance(),
            moon.radiance.luminance()
        );

        // A new moon has no lit face, so it is refused rather than drawn black — and a Moon that
        // has set is refused by the same rule, because the solver gives a body below the horizon
        // no radiance at all.
        let new_moon = voxel_shapers::sky::MoonPosition { phase_fraction: 0.0, ..orbit };
        assert!(
            DrawnMoon::of(
                &new_moon,
                &air,
                peak,
                eye,
                day.sky.sun_altitude_deg,
                day.sky.sun_azimuth_deg,
            )
            .is_none(),
            "a new moon was handed to the pass as a body"
        );
        let set_moon = voxel_shapers::sky::MoonPosition { altitude_deg: -5.0, ..orbit };
        assert!(
            DrawnMoon::of(
                &set_moon,
                &air,
                peak,
                eye,
                day.sky.sun_altitude_deg,
                day.sky.sun_azimuth_deg,
            )
            .is_none(),
            "a Moon below the horizon was handed to the pass as a body"
        );
    }

    fn noon() -> SolarPosition {
        SolarPosition {
            altitude_deg: 90.0,
            azimuth_deg: 0.0,
            declination_deg: 0.0,
            hour_angle_deg: 0.0,
        }
    }

    fn morning() -> SolarPosition {
        SolarPosition { altitude_deg: 30.0, ..noon() }
    }

    #[test]
    fn the_derived_angular_size_is_the_sky_s_own_diameter() {
        // The radius is *derived* from the band table's dilution; this checks that
        // arithmetic against the atmosphere model's own figure rather than trusting a copy.
        let radius = angular_radius_rad(AU_M);
        assert!(
            (2.0 * radius - SUN_ANGULAR_DIAMETER_RAD).abs() < 1.0e-5,
            "derived angular diameter {} against the sky's own {SUN_ANGULAR_DIAMETER_RAD}",
            2.0 * radius
        );
    }

    #[test]
    fn the_sun_is_about_one_hundred_and_sixty_thousand_kilometres_across() {
        let r = sun_radius_m();
        assert!(
            r > 6.9e8 && r < 7.0e8,
            "the sun's radius should be about 6.96e8 m, got {r}"
        );
    }

    #[test]
    fn the_sun_is_one_object_in_the_world_and_not_one_per_observer() {
        // Two observers a long way apart must see the **same** sun; an earlier version built
        // the cube from the observer's position, and looked fine only because its size was
        // derived from its distance.
        let solar = morning();
        let sun = BodyCuboid::at_tick([0.0; 3], &solar).expect("a sun");
        let far_observer = [1.0e6, 0.0, 0.0];
        let to_far = [
            sun.centre[0] - far_observer[0],
            sun.centre[1] - far_observer[1],
            sun.centre[2] - far_observer[2],
        ];
        let d = direction_to(&solar);
        let length = (to_far[0].powi(2) + to_far[1].powi(2) + to_far[2].powi(2)).sqrt();
        let seen = [to_far[0] / length, to_far[1] / length, to_far[2] / length];
        for axis in 0..3 {
            assert!(
                (d[axis] - seen[axis]).abs() < 1.0e-4,
                "axis {axis}: the tick says {d:?} and a far observer sees {seen:?}"
            );
        }
    }

    #[test]
    fn a_sun_below_the_horizon_is_refused_and_not_a_light_from_underneath() {
        let set = SolarPosition { altitude_deg: -5.0, ..noon() };
        assert!(matches!(
            BodyCuboid::at_tick([0.0; 3], &set),
            Err(SunRefusal::BelowHorizon { .. })
        ));
    }

    #[test]
    fn a_clear_sky_lights_a_whole_surface() {
        let sun = BodyCuboid::at_tick([0.0; 3], &noon()).expect("a sun");
        assert_eq!(sun.visibility([0.0, 0.0, 0.0], 16, clear), 1.0);
    }

    #[test]
    fn a_fully_blocked_sun_is_dark_and_says_so() {
        let wall = |_from: [f64; 3], _ray: [f64; 3]| -> bool { true };
        let sun = BodyCuboid::at_tick([0.0; 3], &noon()).expect("a sun");
        assert_eq!(sun.visibility([0.0, 0.0, 0.0], 16, wall), 0.0);
    }

    #[test]
    fn an_occluder_edge_gives_a_penumbra_and_not_a_step() {
        /// A **finite** slab, 10 m up, covering everything to one side of `x = EDGE`. The
        /// first version of this predicate was an infinite half-space, so a ray leaning a
        /// fraction of a degree sideways "hit" it kilometres up.
        const SLAB_Y: f64 = 10.0;
        const EDGE: f64 = 0.05;
        let wall = |from: [f64; 3], ray: [f64; 3]| -> bool {
            if ray[1] <= 0.0 {
                return false;
            }
            let t = (SLAB_Y - from[1]) / ray[1];
            if t <= 0.0 {
                return false;
            }
            (from[0] + ray[0] * t) < EDGE
        };
        let sun = BodyCuboid::at_tick([0.0; 3], &noon()).expect("a sun");

        assert_eq!(sun.visibility([1.0, 0.0, 0.0], 64, wall), 1.0, "clear of the edge");
        assert_eq!(sun.visibility([-1.0, 0.0, 0.0], 64, wall), 0.0, "behind the slab");

        // **The transition itself, and its width.** Near the edge the sun is half hidden, so
        // the visibility is strictly between; ten metres below a sun of angular radius
        // 0.00465 rad, that distance is about `2 x 10 x 0.00465 = 9 cm`.
        let mut partials = Vec::new();
        let mut step_m = 0.005f64;
        while step_m < 0.5 {
            let v = sun.visibility([EDGE - 0.2 + step_m, 0.0, 0.0], 64, wall);
            if v > 0.0 && v < 1.0 {
                partials.push((step_m, v));
            }
            step_m += 0.005;
        }
        assert!(
            !partials.is_empty(),
            "an occluder's edge must have a partially lit band, not a step"
        );
        let spread = partials.last().unwrap().0 - partials[0].0;
        assert!(
            spread > 0.02 && spread < 0.4,
            "the penumbra at 10 m should be centimetres wide, got {spread} m across {} partial samples",
            partials.len()
        );
    }

    #[test]
    fn the_penumbra_is_centimetres_at_a_ten_metre_occluder() {
        let sun = BodyCuboid::at_tick([0.0; 3], &noon()).expect("a sun");
        let width = sun.penumbra_width_m([0.0, 0.0, 0.0], 10.0);
        assert!(
            width > 0.01 && width < 0.2,
            "a 10 m occluder blurs its edge over centimetres, got {width} m"
        );
    }

    /// **One angle, two consumers, and this is the test that says so.** The rig's first light
    /// and the drawn body must be the same direction, and the comparison is against
    /// `daylight_of`'s own `LightRig` - the thing that lights the world - rather than against a
    /// third computation of the same azimuth. Before this file delegated to
    /// [`crate::skylight::sun_direction`] the two differed by a quarter turn, which nothing
    /// noticed while nothing drew the body.
    #[test]
    fn the_drawn_body_points_where_the_rig_s_light_points() {
        let world = WorldSpec::new(2468, Config::default());
        let inputs = SkyInputs::default();
        let peak = peak_sun_luminance(&world, inputs);
        let (mut compared, mut morning, mut afternoon) = (0, false, false);
        for sample in 0..96u64 {
            let tick = sample * (TICKS_PER_DAY / 96);
            let day = daylight_of(&world, tick, inputs, 0).expect("a sky");
            // Both halves are taken from the **same** `Daylight`: a sample with a light and
            // no body, or a body and no light, is the failure this file's other test names.
            let (Some(light), Some(drawn)) =
                (day.rig.lights.first(), DrawnSun::of(&day.sky, peak, [0.0; 3]))
            else {
                continue;
            };
            let c = drawn.body.centre;
            let length = (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt();
            let seen = [c[0] / length, c[1] / length, c[2] / length];
            let want = [light.direction.x, light.direction.y, light.direction.z];
            for axis in 0..3 {
                assert!(
                    (seen[axis] - want[axis]).abs() < 1.0e-12,
                    "tick {tick}, axis {axis}: the body is at {} and the light at {}",
                    seen[axis],
                    want[axis]
                );
            }
            compared += 1;
            if day.sky.sun_azimuth_deg < 180.0 {
                morning = true;
            } else {
                afternoon = true;
            }
        }
        // The guard is only against a day with no sun in it (which would make every assertion
        // above vacuous): the world's own day puts the sun above the horizon for about a third
        // of these samples.
        assert!(compared >= 24, "only {compared} of 96 samples carried a sun to compare");
        assert!(
            morning && afternoon,
            "the comparison has to cover both sides of noon: morning {morning}, afternoon {afternoon}"
        );
        println!("the drawn body and the rig's light agree at all {compared} lit samples");
    }

    /// **A body is drawn exactly when the rig carries a sun light**, sampled across a whole
    /// day. This is the join between `voxel_appearance`'s `Sky::sun` and the picture: a body
    /// drawn for a sun the rig does not have is a picture of a light that is not falling, and
    /// a sun light with no body drawn is a sun nobody can see.
    #[test]
    fn the_body_is_drawn_exactly_when_the_rig_carries_a_sun() {
        let world = WorldSpec::new(2468, Config::default());
        let inputs = SkyInputs::default();
        let peak = peak_sun_luminance(&world, inputs);
        let (mut day_bodies, mut night, mut sun_lights) = (0, 0, 0);
        for sample in 0..96u64 {
            let tick = sample * (TICKS_PER_DAY / 96);
            let day = daylight_of(&world, tick, inputs, 0).expect("a sky");
            let has_light = day.sun_intensity > 0.0;
            let has_body = DrawnSun::of(&day.sky, peak, [0.0; 3]).is_some();
            assert_eq!(
                has_light, has_body,
                "tick {tick} at altitude {:.3}°: rig light {has_light}, drawn body {has_body}",
                day.sky.sun_altitude_deg
            );
            if has_light {
                sun_lights += 1;
                // And the body is on the *lit* side: a direction below the horizon with a
                // drawn body would be the sun lighting the world from underneath.
                assert!(day.sky.sun_altitude_deg > -1.0, "a body was drawn at {}°", day.sky.sun_altitude_deg);
            } else {
                night += 1;
            }
            if has_body {
                day_bodies += 1;
            }
        }
        assert!(day_bodies > 0 && night > 0, "the day has to have both a sun and a night: {sun_lights} lit, {night} dark");
        println!("the body is drawn for {day_bodies} of 96 samples");
    }

    /// **The face is a square and not a disc.** With a face presented to the eye the body's
    /// edge is at `atan(R/d)` and its **corner** reaches `√2` times that along the face's own
    /// diagonal - which is the one ray a disc of the same radius cannot answer. Both rays
    /// below are well clear of the edge, so this is not a statement about tolerance.
    #[test]
    fn the_drawn_face_is_a_square_and_not_a_disc() {
        let sun = BodyCuboid::at([0.0; 3], 90.0, 0.0).expect("a body at the zenith");
        let half_width = (sun.radius_m / AU_M).atan();
        // A ray tilted `angle` off the body's own direction, `across` radians around it in
        // the world's x-z plane. At the zenith that plane is the face's own plane.
        let ray = |angle: f64, across: f64| {
            let (s, c) = angle.sin_cos();
            [s * across.cos(), c, s * across.sin()]
        };
        assert!(sun.ray_hits([0.0; 3], [0.0, 1.0, 0.0]), "the body's own direction missed it");
        assert!(
            sun.ray_hits([0.0; 3], ray(0.5 * half_width, 0.0)),
            "half the half-width inside the edge missed, so the edge is not at R/d"
        );
        assert!(
            !sun.ray_hits([0.0; 3], ray(1.2 * half_width, 0.0)),
            "a ray 20 % past the edge hit, so this is not a body of half-extent R"
        );
        assert!(
            sun.ray_hits([0.0; 3], ray(1.2 * half_width, std::f64::consts::FRAC_PI_4)),
            "the ray to the face's own corner missed, so the body is not being drawn as a square: \
             a disc of radius R/d reaches the edge and not the corner"
        );
        assert!(
            !sun.ray_hits([0.0; 3], ray(1.6 * half_width, std::f64::consts::FRAC_PI_4)),
            "a ray past the corner hit, so the body is larger than a cube"
        );
    }

    /// The two horizon rules are different rules, and this pins the difference: a **shadow**
    /// needs a body above the horizon, while a **drawn** body exists exactly when the rig has
    /// a light in it - and the sky's own `sun_is_up` reaches 0.833° below the horizon for
    /// refraction.
    #[test]
    fn the_shadow_rule_refuses_a_sun_the_drawn_body_still_places() {
        let set = SolarPosition { altitude_deg: -0.4, ..noon() };
        assert!(matches!(
            BodyCuboid::at_tick([0.0; 3], &set),
            Err(SunRefusal::BelowHorizon { .. })
        ));
        assert!(
            BodyCuboid::at([0.0; 3], set.altitude_deg, set.azimuth_deg).is_ok(),
            "the geometric body was refused for a reason that belongs to the shadow rule"
        );
    }

    /// **What the record's own narrowing costs, measured.** The record carries the centre in the
    /// world's engine metres, so the narrowing happens where `quad::origin_bytes` does it — on the
    /// offset from the eye — and one `f32` step at the Sun's own engine distance is 2048 m out of
    /// 1.87e10, which is what moves the drawn direction. Five orders below one pixel of the shipped
    /// window.
    #[test]
    fn the_engine_centre_costs_under_two_e_minus_seven_radians() {
        let eye = [6_000.0, 200.0, -6_000.0];
        let (altitude, azimuth) = (20.0, 130.0);
        let drawn = DrawnSun {
            body: BodyCuboid::at(eye, altitude, azimuth).expect("a body"),
            radiance: LinearRgb::BLACK,
        };
        let record = drawn.body_record(eye);
        let exact = direction_to(&SolarPosition {
            altitude_deg: altitude,
            azimuth_deg: azimuth,
            declination_deg: 0.0,
            hour_angle_deg: 0.0,
        });
        let seen = [
            (record.centre_m[0] - eye[0]) as f32 as f64,
            (record.centre_m[1] - eye[1]) as f32 as f64,
            (record.centre_m[2] - eye[2]) as f32 as f64,
        ];
        let length = (seen[0] * seen[0] + seen[1] * seen[1] + seen[2] * seen[2]).sqrt();
        assert!(
            length > 1.0e9,
            "the body is not an astronomical distance away: {length} m"
        );
        // The centre itself is a world `f64` and exact; what moves is the narrowed *offset*, whose step
        // at the Sun's engine distance is one `f32` ulp — 2 048 m of 1.87e10, which is 1.1e-7 rad.
        let distance_m = voxel_shapers::scale::voxels(AU_M);
        assert!(
            (length - distance_m).abs() <= distance_m * f32::EPSILON as f64,
            "the record carries the body at {length} m, not the world's own {distance_m} m, by more \
             than one f32 step"
        );
        let mut worst = 0.0f64;
        for axis in 0..3 {
            worst = worst.max((seen[axis] / length - exact[axis]).abs());
        }
        assert!(
            worst < 2.0e-7,
            "the narrowed direction moved {worst:e} rad; one f32 step of an AU is 5.5e-8"
        );
        println!("the narrowed drawn direction is within {worst:e} rad of the model's");
    }

    /// **The unit conversion, pinned in both directions.** The drawn radiance is the sun's own
    /// irradiance over the body's own solid angle - and that solid angle is `4·SUN_DILUTION`
    /// against a disc's `π·SUN_DILUTION`, which is the corner a square has and a circle has
    /// not.
    #[test]
    fn the_drawn_radiance_is_the_irradiance_over_the_body_s_own_solid_angle() {
        let world = WorldSpec::new(2468, Config::default());
        let inputs = SkyInputs::default();
        let peak = peak_sun_luminance(&world, inputs);
        let day = daylight_of(&world, 9_000, inputs, 0).expect("a sky at the game's start tick");
        let drawn = DrawnSun::of(&day.sky, peak, [0.0; 3]).expect("the sun is up at tick 9000");

        let solid_angle = solid_angle_sr(AU_M);
        // The same quantity as `DrawnSun::of` builds, by an arithmetic path that divides and
        // multiplies in the other order.
        let want = sky_exposed(day.sky.sun) / (peak * solid_angle);
        let got = drawn.radiance;
        for (name, a, b) in [("r", got.r, want.r), ("g", got.g, want.g), ("b", got.b, want.b)] {
            assert!(
                (a - b).abs() <= 1.0e-12 * b.abs().max(1.0),
                "channel {name} measured {a} against {b}"
            );
        }
        // And it is a division and not a multiply — docs/lighting-sky-and-moon.md
        for (name, channel, share) in [
            ("r", got.r, day.sky.sun.r / peak),
            ("g", got.g, day.sky.sun.g / peak),
            ("b", got.b, day.sky.sun.b / peak),
        ] {
            let recovered = (channel / crate::skylight::SKY_EXPOSURE) * solid_angle;
            assert!(
                (recovered - share).abs() <= 1.0e-12 * share.abs().max(1.0),
                "channel {name}: multiplying the drawn radiance back through the solid angle gave \
                 {recovered}, and the irradiance share was {share}"
            );
        }
        assert!(
            solid_angle < 1.0e-4,
            "the body's solid angle is {solid_angle} sr, so the conversion is a rounding and this \
             test is not measuring anything"
        );
        let against_disc = solid_angle / (std::f64::consts::PI * SUN_DILUTION);
        assert!(
            (against_disc - 4.0 / std::f64::consts::PI).abs() < 1.0e-12,
            "the body's solid angle is {against_disc} times a disc's, not 4/pi"
        );
        println!(
            "drawn radiance {} W/m2/sr per channel at tick 9000, {} times the disc's solid angle",
            got.r, against_disc
        );
    }

    #[test]
    fn no_samples_is_not_a_claim_of_daylight() {
        let sun = BodyCuboid::at_tick([0.0; 3], &noon()).expect("a sun");
        assert_eq!(
            sun.visibility([0.0, 0.0, 0.0], 0, clear),
            0.0,
            "an unmeasured sun must not read as lit"
        );
    }
}