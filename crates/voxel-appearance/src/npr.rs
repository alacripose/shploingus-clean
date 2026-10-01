//! The Valve Source 2 stylisation, applied **over** the OpenPBR BSDF.
//!
//! `APPEARANCE.md` §1.2 names the four terms, in order, and they are the four
//! branches of [`shade`]:
//!
//! 1. **half-Lambert** diffuse wrap — `N·L · 0.5 + 0.5`, so a face turned away
//!    from the light stays lit instead of falling to black;
//! 2. **rim light** — fresnel-shaped and view-facing;
//! 3. **specular warp** — a **hard threshold** term, which is what makes the
//!    highlight read as a stylised one rather than a rendered one;
//! 4. **ambient tint by material class**.
//!
//! # The two things this layer must not do
//!
//! * **It never replaces the BSDF.** Every term is added to a colour that the
//!   OpenPBR inputs produced, so a non-photoreal look is a *stylisation of
//!   correct light transport* rather than a second lighting model. The bar this
//!   serves (`≥3 distinct luminance bands`) is then a property of *the same*
//!   shading the client draws, not of a separate path that only the test uses.
//! * **It never invents a term's input.** A term whose input is **absent** is
//!   *absent*, and [`Terms`] records that: a missing `base_color` means no
//!   diffuse term, not a black one, and the difference is visible in
//!   [`crate::response::ShadingResponse::terms`] rather than hidden inside a
//!   zero.
//!
//! # Where the numbers come from
//!
//! The rim's fresnel exponent is Schlick's (published). The specular warp's
//! threshold is **the material's own roughness** — a smooth surface wants a
//! tight highlight and a rough one wants a broad one — so no constant is
//! invented for it. Every constant this module does use is in
//! [`crate::provenance`], with its source.

use crate::openpbr::{Colour, Emission, LinearRgb, OpenPbrInputs};
use crate::provenance::RIM_SCHLICK_EXPONENT;
use voxel_model::{RefusalKind, RefusalSet};
use voxel_noise::Vec3;

/// One light. `direction` points **toward** the light.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Light {
    pub direction: Vec3,
    pub colour: LinearRgb,
    pub intensity: f64,
}

/// The set of lights a measurement uses.
#[derive(Clone, PartialEq, Debug)]
pub struct LightRig {
    pub lights: Vec<Light>,
    /// The hemisphere light: the tint that arrives from everywhere, before a
    /// surface's own colour is applied to it.
    pub ambient: LinearRgb,
}

impl LightRig {
    /// The rig this repository's measurements use, **declared so that two
    /// measurements are comparable**.
    ///
    /// This is a fixture and not a claim about the game's lighting: the client's
    /// real rig comes from `voxel-client`, and if it produces a different band
    /// count for the same material, that is a result and not a rounding error.
    /// The FMA bar itself says *"under the room light rig"* and never says what
    /// the room is, so the room is declared here — one key light, one fill, a
    /// dim sky.
    pub fn standard() -> Self {
        LightRig {
            lights: vec![
                Light {
                    direction: Vec3::new(0.45, 0.85, 0.28),
                    colour: LinearRgb::WHITE,
                    intensity: 1.0,
                },
                Light {
                    direction: Vec3::new(-0.62, 0.36, -0.47),
                    colour: LinearRgb::new(0.85, 0.90, 1.0),
                    intensity: 0.25,
                },
            ],
            ambient: LinearRgb::grey(0.10),
        }
    }
}

/// The surface being shaded, in world space.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SurfaceFrame {
    /// Outward normal.
    pub normal: Vec3,
    /// Direction **toward** the eye.
    pub view: Vec3,
    /// The share of the visible face that holds no matter — the composition's
    /// hole. A face that is a quarter empty receives a quarter less sky.
    pub open: f64,
}

/// Which of the four Source 2 terms this surface could show.
///
/// A term is `false` when its **input is absent**, which is a different statement
/// from a term that evaluated to zero. Collapsing the two is how "we did not
/// draw a diffuse lobe" becomes indistinguishable from "the surface is black".
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Terms {
    pub half_lambert: bool,
    pub rim: bool,
    pub specular_warp: bool,
    pub ambient: bool,
}

impl Terms {
    pub const fn none() -> Self {
        Terms { half_lambert: false, rim: false, specular_warp: false, ambient: false }
    }

    /// How many of the four were drawn.
    pub const fn count(self) -> usize {
        self.half_lambert as usize + self.rim as usize + self.specular_warp as usize + self.ambient as usize
    }

    /// The terms either of two evaluations used — the honest summary of a sweep,
    /// since a term that responded at one view *was* drawn.
    pub const fn union(self, other: Terms) -> Terms {
        Terms {
            half_lambert: self.half_lambert || other.half_lambert,
            rim: self.rim || other.rim,
            specular_warp: self.specular_warp || other.specular_warp,
            ambient: self.ambient || other.ambient,
        }
    }
}

/// A shaded colour, and the terms that produced it.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Shaded {
    pub colour: LinearRgb,
    pub terms: Terms,
}

/// Shade one surface point under one rig.
///
/// Refuses rather than shading: a non-finite normal or view, an `open` fraction
/// outside `[0, 1]`, a negative light intensity, and **every** problem with the
/// inputs (`OpenPbrInputs::validate`), collected together rather than one at a
/// time.
pub fn shade(
    inputs: &OpenPbrInputs,
    frame: &SurfaceFrame,
    rig: &LightRig,
) -> Result<Shaded, RefusalSet> {
    let mut set = RefusalSet::new();
    if let Err(problems) = inputs.validate() {
        set.absorb(problems);
    }
    for (name, v) in [("normal", frame.normal), ("view", frame.view)] {
        if !v.is_finite() {
            set.refuse(RefusalKind::NonFiniteValue, format!("the surface {name} is not finite"));
        }
    }
    if !frame.open.is_finite() || !(0.0..=1.0).contains(&frame.open) {
        set.refuse(
            RefusalKind::OutOfDeclaredRange,
            format!("open fraction {} is outside [0, 1]", frame.open),
        );
    }
    for light in &rig.lights {
        if !light.direction.is_finite() {
            set.refuse(RefusalKind::NonFiniteValue, "a light direction is not finite");
        }
        if !light.intensity.is_finite() || light.intensity < 0.0 {
            set.refuse(
                RefusalKind::OutOfDeclaredRange,
                format!("light intensity {} is not a non-negative number", light.intensity),
            );
        }
        if !light.colour.is_finite() {
            set.refuse(RefusalKind::NonFiniteValue, "a light colour is not finite");
        }
    }
    if !set.is_empty() {
        return Err(set);
    }

    let normal = frame.normal.normalise();
    let view = frame.view.normalise();
    // A view below the horizon still has a rim; it does not have a negative one.
    let ndotv = normal.dot(view).clamp(0.0, 1.0);

    let mut colour = LinearRgb::BLACK;
    let mut terms = Terms::none();

    // 4. Ambient, first, because it is the term that survives when little else
    //    does. Two things scale it and both come from block truth: a transmissive
    //    surface receives light from every direction rather than only the sky
    //    above it, and a face with holes in it receives less of everything.
    if let Colour::Admitted(base) = inputs.base_color {
        let gain = (1.0 + inputs.transmission_weight) * (1.0 - frame.open);
        colour = colour + base * (rig.ambient * gain);
        terms.ambient = true;
    }

    for light in &rig.lights {
        let toward_light = light.direction.normalise();
        let ndotl = normal.dot(toward_light);

        // 1. Half-Lambert: the wrap is what keeps a face turned away from the
        //    light readable, and it is the first term `APPEARANCE.md` §1.2 names.
        if let Colour::Admitted(base) = inputs.base_color {
            let wrap = ndotl * 0.5 + 0.5;
            let lambert = wrap
                * light.intensity
                * (1.0 - inputs.base_metalness)
                * (1.0 - inputs.transmission_weight);
            colour = colour + base * (light.colour * lambert);
            terms.half_lambert = true;
        }

        // 2. Rim: fresnel-shaped (Schlick's exponent) and view-facing, so it
        //    survives a light that is *behind* the surface — which is the whole
        //    reason the silhouette reads at distance.
        if let Some(roughness) = inputs.specular_roughness {
            let fresnel = (1.0 - ndotv).powi(RIM_SCHLICK_EXPONENT);
            let rim = fresnel * (1.0 - roughness) * light.intensity;
            colour = colour + light.colour * rim;
            terms.rim = true;
        }

        // 3. The specular warp: a step, not a lobe. `(1 − facing) ≤ roughness`
        //    means a smooth surface catches a tight highlight and a rough one
        //    catches a broad one, and the threshold is the material's own
        //    roughness rather than a number this crate picked.
        if let (Some(roughness), Some(weight)) =
            (inputs.specular_roughness, inputs.specular_weight)
        {
            let half = (toward_light + view).normalise();
            let facing = normal.dot(half).clamp(0.0, 1.0);
            if (1.0 - facing) <= roughness {
                colour = colour + light.colour * (weight * light.intensity);
                terms.specular_warp = true;
            }
        }
    }

    // Emission is not a Source 2 term — it is the OpenPBR input FC-8.6 gives a
    // dynamic block emitter, and it is added last because it does not depend on
    // any light at all.
    if let Emission::Emitted { color, luminance } = inputs.emission {
        colour = colour + color * luminance;
    }

    Ok(Shaded { colour, terms })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openpbr::{Colour, Emission, OpenPbrInputs};

    fn stated() -> OpenPbrInputs {
        OpenPbrInputs {
            base_color: Colour::Admitted(LinearRgb::grey(0.4)),
            base_metalness: 0.0,
            specular_roughness: Some(0.35),
            specular_ior: Some(1.5),
            specular_weight: Some(0.05),
            coat_weight: 0.0,
            coat_roughness: Some(0.1),
            transmission_weight: 0.0,
            transmission_depth: Some(0.0),
            subsurface_weight: Some(0.0),
            emission: Emission::None,
        }
    }

    /// Straight up, viewed from straight up: the surface faces both the eye and
    /// the key light.
    fn overhead() -> SurfaceFrame {
        SurfaceFrame { normal: Vec3::new(0.0, 1.0, 0.0), view: Vec3::new(0.0, 1.0, 0.0), open: 0.0 }
    }

    #[test]
    fn a_stated_surface_draws_all_four_terms() {
        let s = shade(&stated(), &overhead(), &LightRig::standard()).unwrap();
        assert_eq!(s.terms.count(), 4, "terms were {:?}", s.terms);
        assert!(s.terms.half_lambert && s.terms.rim && s.terms.specular_warp && s.terms.ambient);
    }

    #[test]
    fn half_lambert_keeps_a_back_lit_face_visible_where_lambert_gives_zero() {
        // The light is behind the surface with N·L exactly -1/2, so pure Lambert
        // clamps to 0 and the face is black while the wrap gives 1/4. The view is
        // head-on, where the rim's fresnel is exactly zero, so the expected colour
        // is pinned rather than bounded: 0.4 * 0.25 = 0.1.
        let rig = LightRig {
            lights: vec![Light {
                direction: Vec3::new(3.0f64.sqrt() / 2.0, -0.5, 0.0),
                colour: LinearRgb::WHITE,
                intensity: 1.0,
            }],
            ambient: LinearRgb::BLACK,
        };
        let shaded = shade(&stated(), &overhead(), &rig).unwrap();
        assert!(shaded.terms.half_lambert);
        assert!(
            (shaded.colour.luminance() - 0.1).abs() < 1e-12,
            "half-Lambert gave {} where the wrap requires 0.1",
            shaded.colour.luminance()
        );
    }

    #[test]
    fn the_rim_is_view_facing_so_it_survives_a_light_behind_the_surface() {
        // Key light behind, no ambient, no diffuse contribution expected to
        // matter: what is left is the rim, and it must not be zero.
        let rig = LightRig { lights: vec![Light { direction: Vec3::new(0.0, -1.0, 0.0), colour: LinearRgb::WHITE, intensity: 1.0 }], ambient: LinearRgb::BLACK };
        // A grazing view: the rim is strongest at the silhouette.
        let grazing = SurfaceFrame { normal: Vec3::new(0.0, 1.0, 0.0), view: Vec3::new(1.0, 0.05, 0.0), open: 0.0 };
        let head_on = overhead();
        let a = shade(&stated(), &grazing, &rig).unwrap();
        let b = shade(&stated(), &head_on, &rig).unwrap();
        assert!(a.colour.luminance() > b.colour.luminance());
    }

    #[test]
    fn the_specular_warp_is_a_step_and_not_a_gradient() {
        // The stylisation is the *step*: the term either fires or it does not, and
        // when it fires it contributes its **whole** weight. A smooth lobe would
        // instead give a value that slides continuously with the view.
        let rig = LightRig {
            lights: vec![Light { direction: Vec3::new(0.0, 1.0, 0.0), colour: LinearRgb::WHITE, intensity: 1.0 }],
            ambient: LinearRgb::BLACK,
        };
        let loud = OpenPbrInputs { specular_roughness: Some(0.05), specular_weight: Some(0.5), ..stated() };
        // Same geometry and same roughness, with the warp's weight zeroed: the
        // difference between the two is therefore the warp's contribution alone.
        let silent = OpenPbrInputs { specular_weight: Some(0.0), ..loud };
        let on_axis = SurfaceFrame { normal: Vec3::new(0.0, 1.0, 0.0), view: Vec3::new(0.0, 1.0, 0.0), open: 0.0 };
        let off_axis = SurfaceFrame { normal: Vec3::new(0.0, 1.0, 0.0), view: Vec3::new(0.95, 0.31, 0.0), open: 0.0 };

        // The threshold is a step: it fires on axis and does not fire off axis.
        assert!(
            shade(&loud, &on_axis, &rig).unwrap().terms.specular_warp,
            "the mirror-facing view must catch the highlight"
        );
        assert!(
            !shade(&loud, &off_axis, &rig).unwrap().terms.specular_warp,
            "an off-axis view must not"
        );

        // And what it adds is the whole weight — 1/2, not a fraction of it.
        let a = shade(&loud, &on_axis, &rig).unwrap();
        let b = shade(&silent, &on_axis, &rig).unwrap();
        assert!(
            (a.colour.luminance() - b.colour.luminance() - 0.5).abs() < 1e-12,
            "the warp added {} of its weight of 0.5",
            a.colour.luminance() - b.colour.luminance()
        );
    }

    #[test]
    fn a_hole_in_the_face_takes_sky_away_from_it() {
        let inputs = stated();
        let solid = shade(&inputs, &overhead(), &LightRig::standard()).unwrap();
        let quarter_open = shade(
            &inputs,
            &SurfaceFrame { open: 0.25, ..overhead() },
            &LightRig::standard(),
        )
        .unwrap();
        assert!(quarter_open.colour.luminance() < solid.colour.luminance());
    }

    #[test]
    fn an_absent_input_removes_a_term_rather_than_dimming_it() {
        // No albedo: there is no diffuse lobe and no ambient term, and the
        // surface still shows its rim. That is the derived rock's real state.
        let no_colour = OpenPbrInputs { base_color: Colour::Unadmitted, ..stated() };
        let s = shade(&no_colour, &overhead(), &LightRig::standard()).unwrap();
        assert!(!s.terms.half_lambert);
        assert!(!s.terms.ambient);
        assert!(s.terms.rim, "roughness is present, so the rim is drawable");
        assert_eq!(s.terms.count(), 2, "rim and the warp: {:?}", s.terms);
    }

    #[test]
    fn shading_is_deterministic() {
        let a = shade(&stated(), &overhead(), &LightRig::standard()).unwrap();
        let b = shade(&stated(), &overhead(), &LightRig::standard()).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn a_broken_frame_is_refused_with_every_problem_named_at_once() {
        let frame = SurfaceFrame {
            normal: Vec3::new(f64::NAN, 1.0, 0.0),
            view: Vec3::new(0.0, 1.0, 0.0),
            open: 1.5,
        };
        let err = shade(&stated(), &frame, &LightRig::standard()).unwrap_err();
        assert_eq!(err.len(), 2, "{err}");
    }

    #[test]
    fn a_bad_input_is_refused_before_any_shading_happens() {
        let inputs = OpenPbrInputs { specular_roughness: Some(2.0), ..stated() };
        let err = shade(&inputs, &overhead(), &LightRig::standard()).unwrap_err();
        assert_eq!(err.all()[0].kind(), RefusalKind::OutOfDeclaredRange);
    }

    #[test]
    fn emission_adds_light_that_no_lamp_supplied() {
        let dark = LightRig { lights: vec![], ambient: LinearRgb::BLACK };
        let lit = OpenPbrInputs {
            emission: Emission::Emitted { color: LinearRgb::WHITE, luminance: 2.0 },
            ..stated()
        };
        let a = shade(&stated(), &overhead(), &dark).unwrap();
        let b = shade(&lit, &overhead(), &dark).unwrap();
        assert_eq!(a.colour, LinearRgb::BLACK);
        assert!(b.colour.luminance() > 0.0);
    }

    #[test]
    fn a_transmissive_surface_receives_more_ambient_than_a_solid_one() {
        let rig = LightRig { lights: vec![], ambient: LinearRgb::grey(0.5) };
        let solid = shade(&stated(), &overhead(), &rig).unwrap();
        let glass = OpenPbrInputs { transmission_weight: 1.0, ..stated() };
        let wet = shade(&glass, &overhead(), &rig).unwrap();
        assert!(wet.colour.luminance() > solid.colour.luminance());
    }
}
