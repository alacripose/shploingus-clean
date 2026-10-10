//! **The sun as a body in the world** - a place and an extent, not a direction.
//!
//! Its distance is the astronomical unit and its radius is derived from the band table's
//! own solar dilution, so no number here is stated twice.
//!
//! # And the body is what the sky pass draws
//!
//! `sky.wgsl` is handed this body's centre relative to the eye, its half-extent and its own
//! radiance, and tests every pixel's ray against the cuboid - so the Sun on screen is the
//! same object [`SunCuboid::visibility`] measures a shadow against, rather than a second
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
pub struct SunCuboid {
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

impl SunCuboid {
    /// **The body's place, geometry only**: one astronomical unit along the direction the two
    /// angles name, from `world_anchor`.
    ///
    /// No horizon rule here - *where the body is* and *whether a shadow can be cast from it*
    /// are two questions, and [`SunCuboid::at_tick`] is the one that answers the second.
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
        Ok(SunCuboid {
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

/// **The Sun as the sky pass draws it**: the body, and the radiance of its faces.
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
    pub body: SunCuboid,
    /// The radiance of its faces, **already through the sky pass's exposure**, so the disc and
    /// the sky it sits in share one scale.
    pub radiance: LinearRgb,
}

impl DrawnSun {
    /// The Sun this frame draws, **from the same `Sky` the light rig was built from**, or
    /// `None` when there is no sun to draw.
    ///
    /// `None` is the rig's own condition rather than a second one. `Sky::sun_light` is exactly
    /// black when the Sun is not up, so `sky.sun.luminance() > 0.0` is true exactly when the
    /// rig carries a sun light - and a body drawn for a sun the rig does not have would be a
    /// picture of a light that is not falling on the world.
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
        // **Refused by name, not by how a comparison happens to treat a NaN.** `!(x > 0.0)` also
        // refuses a NaN, and that behaviour is wanted — a NaN radiance is not a sun — so the NaN
        // is refused explicitly here, which is the form `skylight::sky_lut` already uses for a
        // normaliser and the one clippy's `neg_cmp_op_on_partial_ord` asks for: `!(x > 0.0)` and
        // `x <= 0.0` differ for a NaN, and a reader cannot tell which was meant.
        let irradiance = sky.sun.luminance();
        if !irradiance.is_finite() || irradiance <= 0.0 || !peak.is_finite() || peak <= 0.0 {
            return None;
        }
        let body = SunCuboid::at(anchor_m, sky.sun_altitude_deg, sky.sun_azimuth_deg).ok()?;
        Some(DrawnSun {
            body,
            radiance: sky_exposed(sky.sun / (peak * solid_angle_sr(AU_M))),
        })
    }

    /// The body's centre **relative to the eye**, as the shader's uniform carries it.
    ///
    /// The subtraction is done in `f64` and only the difference is narrowed, the rule
    /// `camera.rs` states for geometry: at one astronomical unit one `f32` metre is spaced
    /// 8192 m, so narrowing the *world* point - 1.5e11 m - would quantise a body 1.4e9 m
    /// across into a grid 8192 m wide. What remains after the subtraction is a direction
    /// whose narrowing moves it by about 5.5e-8 rad, which is five orders below one pixel of
    /// the shipped window (1.1e-3 rad at 1080 lines and a 70° field).
    pub fn centre_relative_to(&self, eye_m: [f64; 3]) -> [f32; 3] {
        let mut out = [0.0f32; 3];
        for (axis, slot) in out.iter_mut().enumerate() {
            *slot = (self.body.centre[axis] - eye_m[axis]) as f32;
        }
        out
    }

    /// The cuboid's half-extent as the uniform carries it.
    pub fn half_extent_f32(&self) -> f32 {
        self.body.radius_m as f32
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
        let sun = SunCuboid::at_tick([0.0; 3], &solar).expect("a sun");
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
            SunCuboid::at_tick([0.0; 3], &set),
            Err(SunRefusal::BelowHorizon { .. })
        ));
    }

    #[test]
    fn a_clear_sky_lights_a_whole_surface() {
        let sun = SunCuboid::at_tick([0.0; 3], &noon()).expect("a sun");
        assert_eq!(sun.visibility([0.0, 0.0, 0.0], 16, clear), 1.0);
    }

    #[test]
    fn a_fully_blocked_sun_is_dark_and_says_so() {
        let wall = |_from: [f64; 3], _ray: [f64; 3]| -> bool { true };
        let sun = SunCuboid::at_tick([0.0; 3], &noon()).expect("a sun");
        assert_eq!(sun.visibility([0.0, 0.0, 0.0], 16, wall), 0.0);
    }

    #[test]
    fn an_occluder_edge_gives_a_penumbra_and_not_a_step() {
        // A **finite** slab, 10 m up, covering everything to one side of `x = EDGE`. The
        // first version of this predicate was an infinite half-space, so a ray leaning a
        // fraction of a degree sideways "hit" it kilometres up.
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
        let sun = SunCuboid::at_tick([0.0; 3], &noon()).expect("a sun");

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
        let sun = SunCuboid::at_tick([0.0; 3], &noon()).expect("a sun");
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
        let sun = SunCuboid::at([0.0; 3], 90.0, 0.0).expect("a body at the zenith");
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
            SunCuboid::at_tick([0.0; 3], &set),
            Err(SunRefusal::BelowHorizon { .. })
        ));
        assert!(
            SunCuboid::at([0.0; 3], set.altitude_deg, set.azimuth_deg).is_ok(),
            "the geometric body was refused for a reason that belongs to the shadow rule"
        );
    }

    /// **What the `f32` narrowing costs, measured.** One astronomical unit is 1.5e11 m, where
    /// one `f32` step is 8192 m; the body is anchored at the eye so the only error left is that
    /// step, and what it moves is the drawn direction. Five orders below one pixel of the
    /// shipped window.
    #[test]
    fn the_eye_relative_centre_costs_under_two_e_minus_seven_radians() {
        let eye = [6_000.0, 200.0, -6_000.0];
        let (altitude, azimuth) = (20.0, 130.0);
        let drawn = DrawnSun {
            body: SunCuboid::at(eye, altitude, azimuth).expect("a body"),
            radiance: LinearRgb::BLACK,
        };
        let relative = drawn.centre_relative_to(eye);
        let exact = direction_to(&SolarPosition {
            altitude_deg: altitude,
            azimuth_deg: azimuth,
            declination_deg: 0.0,
            hour_angle_deg: 0.0,
        });
        let seen = [relative[0] as f64, relative[1] as f64, relative[2] as f64];
        let length = (seen[0] * seen[0] + seen[1] * seen[1] + seen[2] * seen[2]).sqrt();
        assert!(length > 1.0e11, "the body is not a unit of distance away: {length} m");
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
        // **And it is a division and not a multiply**, said as the inverse: dividing the
        // exposure back out and multiplying the solid angle back in recovers the irradiance
        // share the radiance was made from. A wrong direction of that step fails here rather
        // than looking plausible.
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
        let sun = SunCuboid::at_tick([0.0; 3], &noon()).expect("a sun");
        assert_eq!(
            sun.visibility([0.0, 0.0, 0.0], 0, clear),
            0.0,
            "an unmeasured sun must not read as lit"
        );
    }
}