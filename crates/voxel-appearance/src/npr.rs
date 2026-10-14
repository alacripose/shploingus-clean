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
//! * **It never replaces the BSDF.** Every term is added to a colour the OpenPBR
//!   inputs produced, so the `≥3 distinct luminance bands` bar is a property of
//!   the shading the client *draws*, not of a path only the test uses.
//! * **It never invents a term's input.** A term whose input is **absent** is
//!   absent, and [`Terms`] records that: an absent `base_color` is no diffuse
//!   term rather than a black one. The rim — whose inputs are only the light and
//!   the view — survives every absent input, which is why an *unadmitted* albedo
//!   still leaves a surface that reads.
//!
//! # Where the numbers come from
//!
//! The rim's fresnel exponent is Schlick's (published). The specular warp's
//! threshold is **the material's own roughness** — a smooth surface wants a
//! tight highlight and a rough one wants a broad one — so no constant is
//! invented for it. Every constant this module does use is in
//! [`crate::provenance`], with its source.

use crate::openpbr::{Colour, Emission, LinearRgb, OpenPbrInputs};
use crate::provenance::{COAT_F0, RIM_SCHLICK_EXPONENT};
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
/// The rim is the one term whose inputs are only the light and the view, so a lit
/// rig draws it for every material that has a surface at all.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Terms {
    pub half_lambert: bool,
    pub rim: bool,
    pub specular_warp: bool,
    pub ambient: bool,
    /// The **admitted** `subsurface_weight` modulated the diffuse wrap. Admitted and not
    /// *non-zero*: `Some(0.0)` and `None` compute the same number and are different facts.
    pub subsurface: bool,
}

impl Terms {
    pub const fn none() -> Self {
        Terms {
            half_lambert: false,
            rim: false,
            specular_warp: false,
            ambient: false,
            subsurface: false,
        }
    }

    /// How many were drawn: the four stylisations, plus the admitted OpenPBR inputs that
    /// changed the result.
    pub const fn count(self) -> usize {
        self.half_lambert as usize
            + self.rim as usize
            + self.specular_warp as usize
            + self.ambient as usize
            + self.subsurface as usize
    }

    /// The terms either of two evaluations used — the honest summary of a sweep,
    /// since a term that responded at one view *was* drawn.
    pub const fn union(self, other: Terms) -> Terms {
        Terms {
            half_lambert: self.half_lambert || other.half_lambert,
            rim: self.rim || other.rim,
            specular_warp: self.specular_warp || other.specular_warp,
            ambient: self.ambient || other.ambient,
            subsurface: self.subsurface || other.subsurface,
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
    //
    //    **`transmission_depth` is how far through the material the light has to
    //    come**, so a deep one contributes less of the sky than a shallow one. An
    //    absent depth is zero thickness, which is the gain this has always had.
    if let Colour::Admitted(base) = inputs.base_color {
        let depth = inputs.transmission_depth.unwrap_or(0.0);
        let transmitted = inputs.transmission_weight * (1.0 - depth);
        let gain = (1.0 + transmitted) * (1.0 - frame.open);
        colour = colour + base * (rig.ambient * gain);
        terms.ambient = true;
    }

    for light in &rig.lights {
        let toward_light = light.direction.normalise();
        let ndotl = normal.dot(toward_light);

        // 1. Half-Lambert: the wrap is what keeps a face turned away from the
        //    light readable, and it is the first term `APPEARANCE.md` §1.2 names.
        //
        //    **Subsurface widens the wrap** toward full Lambert: light entering a
        //    translucent material leaves it from further round than a purely
        //    surface-scattering one, so a face turned away stays lit for longer.
        //    At zero it is exactly the half-Lambert above, which is what an absent
        //    `subsurface_weight` means.
        //    **And the boundary's share comes out of it.** `OpenPBR_Energy_Conservation.pdf`
        //    §6: "a diffuse term is not a separate reservoir of energy from the specular
        //    term. For a dielectric surface, Fresnel-reflected light never enters the body;
        //    only the remaining transmitted portion can feed the diffuse response behind the
        //    interface." The rim below is that reflected light, so it is taken out here rather
        //    than added on top: `(1 − F(θ_L))` is the spec's own `(1 − E[f_specular])` under
        //    this model's stated approximation, and the same quantity drives both, so the two
        //    can never spend the same light twice.
        //
        //    **The coat is a layer, not a second highlight** (§9): an upper interface
        //    reflects some light and the rest is what reaches the base, so the coat's own
        //    reflectance is subtracted here as well.
        //
        //    **What is *not* subtracted is the substrate's own Fresnel share, and that is a
        //    declared departure.** The spec's `(1 − E[f_specular])` needs an *integrated*
        //    directional albedo, and this model has no integral: `wrap` is the stylisation
        //    (`APPEARANCE.md` §1.2's half-Lambert), and its floor is exactly the light the
        //    interface never passed — the thing the user's look is built on, pinned by
        //    `half_lambert_keeps_a_back_lit_face_visible_where_lambert_gives_zero`. What is
        //    **not** stylised is the boundary's *own* reflection: the rim's energy is the
        //    share of this beam the interface actually reflected, at the beam's own angle, so
        //    the model cannot conjure a highlight the light did not supply.
        //
        //    **The sky is deliberately left whole.** It arrives from every direction, so a
        //    single-angle Fresnel cannot state its share; the spec's `E[f_specular]` is an
        //    *integrated* directional albedo and says so ("not simply the Fresnel value at
        //    the view angle"), and this model has no integral to divide it by. Stating that
        //    is the honest form of not doing it.
        //
        //    **And the angle the rim is read at is the *beam's*, not the view's.**
        //    `F(θ_L)` is the share of *this beam* the interface reflects instead of
        //    transmitting (`OpenPBR_Energy_Conservation.pdf` §5), which is exactly what the
        //    boundary's own reflection must be drawn from. The first form of this law used the
        //    view angle, and the two are different quantities: that version made a face seen
        //    at a grazing angle lose its diffuse **whatever the Sun was doing**, so the
        //    world stopped depending on where the Sun was — the very defect the user reported
        //    as *"the blocks are just lit by themselves"*. `clamp(ndotl, 0, 1)` because a
        //    light at or below the horizon is not "a negative reflection": the beam arrives
        //    at or past grazing, so the interface reflects all of it and the body receives
        //    none.
        let beam_share = (1.0 - ndotl.clamp(0.0, 1.0)).powi(RIM_SCHLICK_EXPONENT);
        // **The silhouette shape**, which is what makes the rim a rim. The same power as the
        // beam's share, so the two terms are one law read at two angles rather than two laws.
        let silhouette = (1.0 - ndotv).powi(RIM_SCHLICK_EXPONENT);
        // The coat layer's own reflectance, at the beam's angle for the same reason, and
        // gated on both its weight and its admitted roughness (a highlight with no spread is
        // not a coat). Nothing the world admits sets `coat_weight` today, which is why this
        // is a law with no number in it yet.
        let coat = match (inputs.coat_weight, inputs.coat_roughness) {
            (weight, Some(roughness)) if weight > 0.0 => {
                COAT_F0 * weight * (1.0 - roughness) * beam_share
            }
            _ => 0.0,
        };
        // **One stack, one bookkeeping convention** (§9): the substrate keeps what the coat
        // layer above it passed. Adding coat weight therefore cannot increase the
        // full-strength energy available to both the coat and the base.
        let substrate_share = 1.0 - coat;

        if let Colour::Admitted(base) = inputs.base_color {
            let half_lambert = ndotl * 0.5 + 0.5;
            let lambert_term = ndotl.clamp(0.0, 1.0);
            let subsurface = inputs.subsurface_weight.unwrap_or(0.0);
            // Recorded as drawn: the wrap was computed for it, whether or not it moved the colour.
            terms.subsurface = inputs.subsurface_weight.is_some();
            let wrap = half_lambert + subsurface * (lambert_term - half_lambert);
            let lambert = wrap
                * light.intensity
                * (1.0 - inputs.base_metalness)
                * (1.0 - inputs.transmission_weight)
                * substrate_share;
            colour = colour + base * (light.colour * lambert);
            terms.half_lambert = true;
        }

        // 2. Rim: fresnel-shaped (Schlick's exponent), view-facing — so it survives a
        //    light that is *behind* the surface, which is the whole reason the silhouette
        //    reads at distance — and **multiplied by the block's own reflectance**.
        //
        //    **Two defects met here, and they were different mistakes.**
        //
        //    The rim used to be multiplied by `(1 − roughness)`, with `roughness` the
        //    derived porosity, and that factor is zero at both ends of the model: a
        //    transmissive material derives no roughness at all (no admitted weathering
        //    model), and the lightest opaque one derives porosity exactly `1.0`. Four
        //    admitted materials had a rim of zero **at every angle, by arithmetic** — the
        //    defect `voxel-client`'s `no_material_loses_its_rim_to_arithmetic` names. The
        //    factor was also a category error: porosity is a claim about rock weathering,
        //    and this is a boundary term.
        //
        //    Dropping it left the rim reading *nothing* of the material — a full-strength
        //    highlight in the light's own colour, laid over every surface identically.
        //    That is the second defect, and it is the one the user's
        //    `docs/OpenPBR_Energy_Conservation.pdf` §§5–6 states: what reaches the eye at a
        //    boundary is that boundary's reflectance, and a rim light that **overrides the
        //    block's reflectance** — spending the same incident light a second time on top
        //    of an untouched diffuse layer — is that document's own §6 entry under
        //    *incorrect construction*: "diffuse = albedo, plus independently tuned
        //    specular". A rim is not a white light. It is the Sun's light off *this* block,
        //    so it carries this block's reflectance.
        //
        //    The **shape** is still Source 2's silhouette power (`(1 − cos θ)⁵`): the
        //    shape of the term is NPR's, and what this model was missing is *whose*
        //    reflectance it is. `Colour::Unadmitted` — a block nobody has a spectrum for —
        //    reflects the light itself rather than an invented grey, which is the same "an
        //    absent parameter delivers the whole weight" rule `specular_f0`'s
        //    `unwrap_or(1.0)` follows. `Terms::rim` is set whenever a light reached the
        //    surface, so a rig with no lights draws no rim, which is the only absence this
        //    term has.
        //
        //    **And the rim spends the beam's reflected share instead of adding a second
        //    energy source.** Its radiance is `reflectance × F(θ_L) × silhouette ×
        //    intensity`: the share of *this beam* the interface reflected, at the angle the
        //    beam arrived at, shaped by how close to the silhouette it is being seen. A
        //    backlit face (the beam at or past grazing, so `beam_share = 1`) is therefore all
        //    rim, and a face the Sun is in front of gets only the thin edge the interface
        //    genuinely reflects — no rim light can conjure light the beam did not supply.
        let rim_reflectance = match inputs.base_color {
            Colour::Admitted(base) => base,
            Colour::Unadmitted => LinearRgb::WHITE,
        };
        colour = colour + rim_reflectance * (light.colour * (beam_share * silhouette * light.intensity));
        terms.rim = true;

        // 3. The **coat**: a smooth dielectric layer over the surface, which is
        //    OpenPBR FC-8.6's clear coat and what wetness lands in. It adds its own
        //    fresnel-shaped highlight at `COAT_F0`, scaled by `coat_weight` and gated
        //    by the coat's own roughness — so a wet surface gets a gloss *layer*
        //    rather than a darker colour, which is what water on stone looks like.
        // **The coat only exists when its roughness is stated.** An absent
        // `coat_roughness` means nobody has characterised the layer, and a highlight
        // without a spread is not a coat — it is the warp under another name. So the
        // two are read together, and `coat_weight` alone cannot draw one.
        if coat > 0.0 {
            colour = colour + LinearRgb::WHITE * (coat * silhouette * light.intensity);
            terms.rim = true;
        }

        // 4. The specular warp: a step, not a lobe. `(1 − facing) ≤ roughness`
        //    means a smooth surface catches a tight highlight and a rough one
        //    catches a broad one, and the threshold is the material's own
        //    roughness rather than a number this crate picked.
        //
        //    **The step is the whole `specular_weight`, deliberately** — see
        //    `the_specular_warp_is_a_step_and_not_a_gradient`. What `specular_ior`
        //    scales is the *reflectance the step delivers*, not the step's threshold or
        //    its extent, so the stylisation is untouched and a mineral's own index still
        //    decides how bright it glints.
        if let (Some(roughness), Some(weight)) =
            (inputs.specular_roughness, inputs.specular_weight)
        {
            let half = (toward_light + view).normalise();
            let facing = normal.dot(half).clamp(0.0, 1.0);
            if (1.0 - facing) <= roughness {
                // **How much the material reflects at all** comes from its own
                // `specular_ior`. The step is unchanged — it still fires at the whole
                // `specular_weight`, which is the stylisation — but two minerals at
                // different indices now differ in brightness instead of being identical.
                let f0 = inputs
                    .specular_ior
                    .map(crate::provenance::specular_f0)
                    .unwrap_or(1.0);
                colour = colour + light.colour * (weight * f0 * light.intensity);
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
        // Five: the four stylisations, plus the `subsurface_weight` the fixture admits.
        assert_eq!(s.terms.count(), 5, "terms were {:?}", s.terms);
        assert!(s.terms.half_lambert && s.terms.rim && s.terms.specular_warp && s.terms.ambient);
        assert!(s.terms.subsurface);
    }

    /// **Every term the OpenPBR model declares must change the shading.**
    ///
    /// Probed behaviourally: two inputs differing only in one term must shade to different
    /// colours. A field nothing reads fails here whatever it is declared to be, so this
    /// catches a term that is present in the struct, filled in by `derive`, carried to the
    /// GPU — and read by nobody on either side.
    #[test]
    fn every_declared_openpbr_term_changes_the_picture() {
        let rig = LightRig::standard();
        let frame = SurfaceFrame {
            normal: Vec3::new(0.0, 1.0, 0.0),
            view: Vec3::new(0.0, 0.3, 0.954).normalise(),
            open: 0.0,
        };

        /// One declared term's probe: its name, and two setters whose difference must move the
        /// picture — a term whose two arms shade identically is a term nothing reads.
        type Probe = (&'static str, fn(&mut OpenPbrInputs), fn(&mut OpenPbrInputs));

        let probes: [Probe; 11] = [
            ("base_color", |i| i.base_color = Colour::Admitted(LinearRgb::WHITE), |i| i.base_color = Colour::Admitted(LinearRgb::grey(0.4))),
            ("base_metalness", |i| i.base_metalness = 1.0, |i| i.base_metalness = 0.0),
            ("specular_roughness", |i| i.specular_roughness = Some(0.9), |i| i.specular_roughness = Some(0.1)),
            ("specular_ior", |i| i.specular_ior = Some(2.4), |i| i.specular_ior = Some(1.2)),
            ("specular_weight", |i| i.specular_weight = Some(0.9), |i| i.specular_weight = Some(0.1)),
            ("coat_weight", |i| i.coat_weight = 1.0, |i| i.coat_weight = 0.0),
            // **Roughness only acts on a coat that exists**, so both arms carry a coat weight.
            // Varying it on a surface with no coat is a real no-op, and reporting that as
            // "the term is dead" would be the census lying about itself.
            ("coat_roughness", |i| { i.coat_weight = 1.0; i.coat_roughness = Some(0.9); }, |i| { i.coat_weight = 1.0; i.coat_roughness = Some(0.05); }),
            ("transmission_weight", |i| i.transmission_weight = 1.0, |i| i.transmission_weight = 0.0),
            // **Depth only acts on a transmissive surface**, so both arms of this probe
            // carry a transmission weight — varying depth on an opaque material is a real
            // no-op, and calling that "the term is dead" would be the census lying.
            ("transmission_depth", |i| { i.transmission_weight = 1.0; i.transmission_depth = Some(0.9); }, |i| { i.transmission_weight = 1.0; i.transmission_depth = Some(0.05); }),
            ("subsurface_weight", |i| i.subsurface_weight = Some(0.9), |i| i.subsurface_weight = Some(0.05)),
            ("emission", |i| i.emission = Emission::Emitted { color: LinearRgb::WHITE, luminance: 2.0 }, |i| i.emission = Emission::None),
        ];

        let mut unread: Vec<&str> = Vec::new();
        for (name, set_a, set_b) in probes {
            let mut a = stated();
            set_a(&mut a);
            let mut b = stated();
            set_b(&mut b);
            let ca = shade(&a, &frame, &rig).expect("a shades").colour;
            let cb = shade(&b, &frame, &rig).expect("a shades").colour;
            if ca == cb {
                unread.push(name);
            }
        }
        assert!(
            unread.is_empty(),
            "{} term(s) are declared on `OpenPbrInputs` and read by nothing in `shade` — they \
             are filled in, carried to the GPU, and dead on both sides: {unread:?}",
            unread.len()
        );
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
    fn the_rim_carries_the_share_of_the_beam_the_interface_reflected() {
        // **`OpenPBR_Energy_Conservation.pdf` §§5–6, asserted as identities.** `F(θ_L)` is
        // the share of *this beam* the interface reflects instead of transmitting, so the rim
        // — the boundary's reflection of that beam, seen at the silhouette — is worth
        // `F(θ_L) × silhouette × reflectance` and nothing more. A rim light cannot conjure
        // light the beam did not supply, and it cannot appear where the interface reflected
        // nothing.
        //
        // **What this catches, and why no single-angle assertion can.** Every one of these
        // numbers was *plausible* in the defect: a full-strength rim over an untouched
        // diffuse layer is the spec's own §6 entry for an incorrect construction ("diffuse =
        // albedo, plus independently tuned specular"), it carried the light's colour instead
        // of the block's reflectance, and it did not read the light's angle at all — which is
        // why a scene lit by it rendered `3.13` on a `[0, 1]` target. The warp is silenced
        // (`weight = Some(0.0)`) so these two terms are the only contributors and every
        // equality below is exact rather than approximate.
        let inputs = OpenPbrInputs { specular_weight: Some(0.0), ..stated() };
        let normal = Vec3::new(0.0, 1.0, 0.0);
        let rig_at = |direction: Vec3| LightRig {
            lights: vec![Light { direction, colour: LinearRgb::WHITE, intensity: 1.0 }],
            ambient: LinearRgb::BLACK,
        };
        let frame = |view: Vec3| SurfaceFrame { normal, view, open: 0.0 };
        let base = match inputs.base_color {
            Colour::Admitted(c) => c,
            Colour::Unadmitted => unreachable!("`stated` admits a colour"),
        };
        let grazing_view = Vec3::new(1.0, 0.02, 0.0).normalise();
        let silhouette = (1.0 - normal.dot(grazing_view)).powi(RIM_SCHLICK_EXPONENT);

        // **A beam along the normal reflects nothing into the silhouette.** With the light
        // overhead and a grazing view, the face is the substrate's whole wrap and the rim is
        // worth exactly zero — the rim's own geometry is at its strongest here, and it still
        // draws nothing, because `F(θ_L) = 0` for a beam that arrives head-on.
        let overhead = shade(&inputs, &frame(grazing_view), &rig_at(normal)).unwrap();
        assert!(
            (overhead.colour.luminance() - base.luminance()).abs() < 1e-12,
            "a beam along the normal should leave the substrate its whole light and the rim \
             nothing: {} against {}",
            overhead.colour.luminance(),
            base.luminance()
        );

        // **A beam at grazing incidence, seen at the silhouette.** The interface reflects all
        // of that beam, so the picture is the wrap at `ndotl = 0` (`0.5`) *plus* the whole
        // reflected share, scaled by the block's own reflectance.
        let horizontal = Vec3::new(1.0, 0.0, 0.0);
        let reflected = shade(&inputs, &frame(grazing_view), &rig_at(horizontal)).unwrap();
        let expected = base.luminance() * (0.5 + silhouette);
        assert!(
            (reflected.colour.luminance() - expected).abs() < 1e-12,
            "a beam at grazing incidence seen at the silhouette is {} where the wrap plus the \
             reflected share is {}",
            reflected.colour.luminance(),
            expected
        );

        // **And head-on that reflection is not in the eye at all**: the beam leaves along the
        // mirror direction, so the silhouette shape is zero and only the wrap remains. A rim
        // that ignored the beam's own angle would paint this face.
        let missed = shade(&inputs, &frame(normal), &rig_at(horizontal)).unwrap();
        assert!(
            (missed.colour.luminance() - base.luminance() * 0.5).abs() < 1e-12,
            "a beam reflected away from the eye lit the face anyway: {}",
            missed.colour.luminance()
        );
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

        // **And what it adds is the whole weight, scaled by the material's own reflectance.**
        // The step is the stylisation and is unchanged — it fires at 1/2 of a unit, not at
        // a fraction *of* 1/2 — but `stated()` declares `specular_ior = 1.5`, so the light
        // delivered is that weight times the boundary's 4 %. Two assertions, because both
        // halves are claims: the step fired, and the reflectance set how bright it was.
        let a = shade(&loud, &on_axis, &rig).unwrap();
        let b = shade(&silent, &on_axis, &rig).unwrap();
        let f0 = crate::provenance::specular_f0(stated().specular_ior.expect("a stated index"));
        assert!(
            (f0 - 0.04).abs() < 1e-3,
            "n = 1.5 should reflect about 4 % head-on, got {f0}"
        );
        assert!(
            (a.colour.luminance() - b.colour.luminance() - 0.5 * f0).abs() < 1e-12,
            "the warp added {} and the weight times reflectance is {}",
            a.colour.luminance() - b.colour.luminance(),
            0.5 * f0
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
        assert!(
            s.terms.rim,
            "the rim reads the light and the view, so an absent albedo cannot remove it"
        );
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
