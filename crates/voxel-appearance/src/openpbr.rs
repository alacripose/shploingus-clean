//! The OpenPBR inputs, **derived from the material's own physics**.
//!
//! `APPEARANCE.md` §1.1 names the twelve OpenPBR parameters this repository
//! implements. The old tree named the same twelve and had **nothing behind
//! them** — the renderer used fixed per-(kind, direction) colours. Here they are
//! the actual shading inputs, and the first question is where their *values* come
//! from, because no record in this repository contains any.
//!
//! # The rule this module applies
//!
//! **A parameter is *derived* when the model's own structure determines it, and
//! *absent* when the world has the value and the model does not record it.**
//!
//! * *Derived* — nothing in the model can declare a conductor, so no material is
//!   metallic (`base_metalness` is 0). Nothing can declare an emitter, so nothing
//!   emits. A transmissive material transmits and an opaque one does not, so
//!   transmission is **never** on a solid. A face whose outward neighbour is
//!   fluid is wet. And `specular_roughness` is the material's **own** density
//!   read against the admitted set's own density range, which is the
//!   `APPEARANCE.md` §1.1 weathering signal: eroded rock is porous, and porous
//!   rock is rough.
//! * *Absent* — `base_color`, `specular_ior`, `specular_weight`, `coat_roughness`,
//!   `transmission_depth` and `subsurface_weight` are **not** defaulted to zero.
//!   `Option::is_none` and `Colour::Unadmitted` are the honest states, and
//!   [`OpenPbrInputs::unavailable`] names every one of them. A zero would be
//!   indistinguishable from an authored value, which is precisely what the one
//!   rule exists to prevent.
//!
//! So a derived rock today has six of twelve parameters and no albedo: it can
//! show its rim and its ambient, and it cannot show a diffuse response, because
//! there is no admitted reflectance for it to show. That is not a defect in this
//! module. It is the measured state of the repository's content, converted from
//! "appearance is blocked" into a **finite, named debt** — six parameters, listed
//! per material.
//!
//! # The approximate family
//!
//! Everything here is `f64`. DESIGN-SPEC §2.1 puts appearance in the
//! *approximate* family and mass, volume and lineage in the *exact* one. This
//! crate has no dependency path to [`voxel_model::conservation`], so no value
//! here can reach an exact quantity — the boundary is a missing dependency and
//! not a convention.

use crate::composition::VoxelComposition;
use crate::detail::DetailSample;
use crate::provenance::{LUMINANCE_BLUE, LUMINANCE_GREEN, LUMINANCE_RED};
use core::ops::{Add, Div, Mul};
use voxel_model::{
    Material, MaterialId, MaterialRegistry, OpticalClass, Refusal, RefusalKind, RefusalSet, Refused,
};

/// A linear RGB triple.
///
/// Linear, and deliberately not `sRGB`: a transfer curve between the shader and
/// the measurement would change the luminance bands the bar counts, so the
/// encoding happens in the client and never here.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct LinearRgb {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

impl LinearRgb {
    pub const BLACK: LinearRgb = LinearRgb { r: 0.0, g: 0.0, b: 0.0 };
    pub const WHITE: LinearRgb = LinearRgb { r: 1.0, g: 1.0, b: 1.0 };

    pub const fn new(r: f64, g: f64, b: f64) -> Self {
        LinearRgb { r, g, b }
    }

    pub const fn grey(v: f64) -> Self {
        LinearRgb { r: v, g: v, b: v }
    }

    /// Relative luminance, Rec. 709 (see [`crate::provenance`]).
    ///
    /// This is the number the FMA bar's *≥3 distinct luminance bands* criterion
    /// counts. Using a different triple would change the band count for the same
    /// picture, which is why the weights are a cited standard.
    pub fn luminance(self) -> f64 {
        LUMINANCE_RED * self.r + LUMINANCE_GREEN * self.g + LUMINANCE_BLUE * self.b
    }

    pub fn is_finite(self) -> bool {
        self.r.is_finite() && self.g.is_finite() && self.b.is_finite()
    }

    pub fn in_unit_range(self) -> bool {
        [self.r, self.g, self.b].iter().all(|c| (0.0..=1.0).contains(c))
    }
}

impl Add for LinearRgb {
    type Output = LinearRgb;
    fn add(self, other: LinearRgb) -> LinearRgb {
        LinearRgb::new(self.r + other.r, self.g + other.g, self.b + other.b)
    }
}

impl Mul<f64> for LinearRgb {
    type Output = LinearRgb;
    fn mul(self, s: f64) -> LinearRgb {
        LinearRgb::new(self.r * s, self.g * s, self.b * s)
    }
}

/// A radiance arriving at the top of the air column, scaled by its transmittance.
///
/// **The left-hand form, because that is how the physical sentence reads**: what comes
/// down is what was up *times* what the air let through. Written the other way round it
/// would be the same arithmetic wearing a coat that argues the column attenuates the
/// light rather than the radiance.
impl Mul<LinearRgb> for f64 {
    type Output = LinearRgb;
    fn mul(self, colour: LinearRgb) -> LinearRgb {
        LinearRgb::new(self * colour.r, self * colour.g, self * colour.b)
    }
}

/// The same normalisation, written as a division rather than a multiplication by a
/// reciprocal.
///
/// **Because "divide out the peak" and "multiply by one over the peak" are the same
/// arithmetic and not the same statement**, and this project records which one it meant.
impl Div<f64> for LinearRgb {
    type Output = LinearRgb;
    fn div(self, d: f64) -> LinearRgb {
        LinearRgb::new(self.r / d, self.g / d, self.b / d)
    }
}

impl Mul<LinearRgb> for LinearRgb {
    type Output = LinearRgb;
    /// Element-wise, which is what modulating an albedo by an incident light is.
    fn mul(self, other: LinearRgb) -> LinearRgb {
        LinearRgb::new(self.r * other.r, self.g * other.g, self.b * other.b)
    }
}

/// A surface colour: admitted, or **absent**.
///
/// Not an `Option<LinearRgb>` because the absence needs a name at every call
/// site: grey is a colour, and a surface with no admitted albedo does not have a
/// grey one, it has none.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Colour {
    /// Albedo an admitted row supplies.
    Admitted(LinearRgb),
    /// **No admitted spectral data.** The diffuse lobe is *absent*, not dim.
    Unadmitted,
}

/// What, if anything, a surface emits.
///
/// `None` is deliberately **not** `luminance: 0.0`. A zero is a value a caller
/// can mistake for an authored one; `None` is the absence of a claim, and it is
/// the true state here because nothing in [`Material`] can declare an emitter.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Emission {
    None,
    Emitted { color: LinearRgb, luminance: f64 },
}

impl Emission {
    pub fn luminance(self) -> f64 {
        match self {
            Emission::None => 0.0,
            Emission::Emitted { luminance, .. } => luminance,
        }
    }
}

/// One of the twelve OpenPBR parameters `APPEARANCE.md` §1.1 names.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Parameter {
    BaseColor,
    BaseMetalness,
    SpecularRoughness,
    SpecularIor,
    SpecularWeight,
    CoatWeight,
    CoatRoughness,
    TransmissionWeight,
    TransmissionDepth,
    SubsurfaceWeight,
    EmissionColor,
    EmissionLuminance,
}

/// Whether a parameter's value is determined by the model's structure, recorded
/// as data, or simply not recorded anywhere.
///
/// The third state is not decoration. `base_color` was `Absent` for the whole
/// life of this crate, and the consequence was measured: the ambient and
/// half-Lambert terms are gated on an admitted `base_color`, so the world drew
/// black (`docs/LEDGER.md` C-38). It is not `Derived` either — the model's
/// *structure* cannot produce a reflectance, only a **sourced measurement** in
/// the record can. So the rule table says which of the two it is per material,
/// and a material whose record has no reflectance is still `Absent`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Availability {
    /// The model determines it. Present, always.
    Derived(&'static str),
    /// Present **when the material's own record carries it**, absent when it does
    /// not. The state for a parameter that depends on measured data.
    Sourced(&'static str),
    /// The world has this value and nothing admits it. **Absent, not zero.**
    Absent(&'static str),
}

impl Parameter {
    /// All twelve, in `APPEARANCE.md` §1.1's order.
    pub const ALL: [Parameter; 12] = [
        Parameter::BaseColor,
        Parameter::BaseMetalness,
        Parameter::SpecularRoughness,
        Parameter::SpecularIor,
        Parameter::SpecularWeight,
        Parameter::CoatWeight,
        Parameter::CoatRoughness,
        Parameter::TransmissionWeight,
        Parameter::TransmissionDepth,
        Parameter::SubsurfaceWeight,
        Parameter::EmissionColor,
        Parameter::EmissionLuminance,
    ];

    /// The OpenPBR name, exactly as the specification spells it.
    pub const fn name(self) -> &'static str {
        match self {
            Parameter::BaseColor => "base_color",
            Parameter::BaseMetalness => "base_metalness",
            Parameter::SpecularRoughness => "specular_roughness",
            Parameter::SpecularIor => "specular_ior",
            Parameter::SpecularWeight => "specular_weight",
            Parameter::CoatWeight => "coat_weight",
            Parameter::CoatRoughness => "coat_roughness",
            Parameter::TransmissionWeight => "transmission_weight",
            Parameter::TransmissionDepth => "transmission_depth",
            Parameter::SubsurfaceWeight => "subsurface_weight",
            Parameter::EmissionColor => "emission_color",
            Parameter::EmissionLuminance => "emission_luminance",
        }
    }

    /// Where this parameter's value comes from — the rule at the top of this
    /// module, one entry per parameter.
    pub const fn availability(self) -> Availability {
        match self {
            Parameter::BaseColor => Availability::Sourced(
                "a sourced, broadband reflectance in the material's own record. A material without one is absent, not grey",
            ),
            Parameter::BaseMetalness => Availability::Derived(
                "nothing in the model can declare a conductor, so no material is metallic",
            ),
            Parameter::SpecularRoughness => Availability::Derived(
                "the weathering signal: the material's density read against the admitted set's own range. Absent for a transmissive material, which has no admitted weathering model",
            ),
            Parameter::SpecularIor => Availability::Absent(
                "no admitted refractive index: the model records what a material is made of, not how it bends light",
            ),
            Parameter::SpecularWeight => Availability::Absent("no admitted dielectric F0"),
            Parameter::CoatWeight => Availability::Derived(
                "hydrology adjacency: a face whose outward neighbour is a transmissive fluid is wet",
            ),
            Parameter::CoatRoughness => Availability::Absent(
                "no admitted film roughness: the model says a face is wet, not how the film is textured",
            ),
            Parameter::TransmissionWeight => Availability::Derived(
                "OpticalClass: a transmissive material transmits and an opaque one does not, so transmission is never on a solid",
            ),
            Parameter::TransmissionDepth => Availability::Absent(
                "no admitted extinction: the model says whether a material transmits, not how far",
            ),
            Parameter::SubsurfaceWeight => Availability::Absent(
                "no admitted scattering: the model carries no subsurface term for soil or snow",
            ),
            Parameter::EmissionColor => Availability::Derived(
                "nothing in the model can declare an emitter, so no material emits",
            ),
            Parameter::EmissionLuminance => Availability::Derived(
                "nothing in the model can declare an emitter, so no material emits",
            ),
        }
    }

    /// The reason text a refusal names when this parameter is unavailable.
    pub const fn reason(self) -> &'static str {
        match self.availability() {
            Availability::Absent(why) => why,
            Availability::Derived(why) => why,
            Availability::Sourced(why) => why,
        }
    }
}

/// The shading inputs, each either present or **named absent**.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct OpenPbrInputs {
    pub base_color: Colour,
    pub base_metalness: f64,
    pub specular_roughness: Option<f64>,
    pub specular_ior: Option<f64>,
    pub specular_weight: Option<f64>,
    pub coat_weight: f64,
    pub coat_roughness: Option<f64>,
    pub transmission_weight: f64,
    pub transmission_depth: Option<f64>,
    pub subsurface_weight: Option<f64>,
    pub emission: Emission,
}

impl OpenPbrInputs {
    /// The parameters this surface has **no admitted value** for, in
    /// `APPEARANCE.md` §1.1's order.
    pub fn unavailable(&self) -> Vec<Parameter> {
        let mut out = Vec::new();
        if self.base_color == Colour::Unadmitted {
            out.push(Parameter::BaseColor);
        }
        if self.specular_roughness.is_none() {
            out.push(Parameter::SpecularRoughness);
        }
        if self.specular_ior.is_none() {
            out.push(Parameter::SpecularIor);
        }
        if self.specular_weight.is_none() {
            out.push(Parameter::SpecularWeight);
        }
        if self.coat_roughness.is_none() {
            out.push(Parameter::CoatRoughness);
        }
        if self.transmission_depth.is_none() {
            out.push(Parameter::TransmissionDepth);
        }
        if self.subsurface_weight.is_none() {
            out.push(Parameter::SubsurfaceWeight);
        }
        out
    }

    /// Every reason this cannot be shaded: a non-finite value, or a value outside
    /// the range OpenPBR declares for it.
    ///
    /// **All** of them, not the first — a material with four bad parameters
    /// reports four problems in one round.
    pub fn validate(&self) -> Result<(), RefusalSet> {
        let mut set = RefusalSet::new();
        if let Colour::Admitted(c) = self.base_color {
            check_colour(&mut set, "base_color", c);
        }
        check_unit(&mut set, "base_metalness", Some(self.base_metalness));
        check_unit(&mut set, "specular_roughness", self.specular_roughness);
        check_unit(&mut set, "specular_weight", self.specular_weight);
        check_unit(&mut set, "coat_weight", Some(self.coat_weight));
        check_unit(&mut set, "coat_roughness", self.coat_roughness);
        check_unit(&mut set, "transmission_weight", Some(self.transmission_weight));
        check_unit(&mut set, "subsurface_weight", self.subsurface_weight);
        if let Some(ior) = self.specular_ior {
            check_non_finite(&mut set, "specular_ior", ior);
            if ior.is_finite() && ior < 1.0 {
                set.refuse(
                    RefusalKind::OutOfDeclaredRange,
                    format!("specular_ior is {ior}, below the dielectric minimum of 1"),
                );
            }
        }
        if let Some(depth) = self.transmission_depth {
            check_non_finite(&mut set, "transmission_depth", depth);
            if depth.is_finite() && depth < 0.0 {
                set.refuse(
                    RefusalKind::OutOfDeclaredRange,
                    format!("transmission_depth is {depth}, and a path length cannot be negative"),
                );
            }
        }
        if let Emission::Emitted { color, luminance } = self.emission {
            check_colour(&mut set, "emission_color", color);
            check_non_finite(&mut set, "emission_luminance", luminance);
            if luminance.is_finite() && luminance < 0.0 {
                set.refuse(
                    RefusalKind::OutOfDeclaredRange,
                    format!("emission_luminance is {luminance}, and emitted light is not negative"),
                );
            }
        }
        set.into_result(())
    }

    /// The same material, **at one address of the detail lattice**
    /// (`APPEARANCE.md` §4: block truth *plus deterministic variation*).
    ///
    /// The variation is one-sided by construction: it moves the value toward the
    /// domain edge it has room for and never past it, so no clamp and no refusal
    /// is needed and the result is still a legal OpenPBR value. A parameter the
    /// material does not have stays absent — detail cannot invent an albedo.
    pub fn with_detail(&self, sample: DetailSample) -> Refused<Self> {
        if !sample.is_finite() {
            return Err(Refusal::owned(
                RefusalKind::NonFiniteValue,
                format!("detail sample {sample:?} is not finite"),
            ));
        }
        let mut out = *self;
        out.specular_roughness = self
            .specular_roughness
            .map(|r| vary(r, sample.roughness_delta));
        out.base_color = match self.base_color {
            Colour::Admitted(c) => {
                let d = sample.reflectance_delta;
                Colour::Admitted(LinearRgb::new(vary(c.r, d), vary(c.g, d), vary(c.b, d)))
            }
            Colour::Unadmitted => Colour::Unadmitted,
        };
        Ok(out)
    }
}

/// Move a value in `[0, 1]` toward the edge it has room for. One-sided by
/// construction, so the result cannot leave OpenPBR's declared domain.
fn vary(value: f64, delta: f64) -> f64 {
    if delta >= 0.0 {
        value + delta * (1.0 - value)
    } else {
        value + delta * value
    }
}

fn check_non_finite(set: &mut RefusalSet, name: &'static str, v: f64) {
    if !v.is_finite() {
        set.refuse(RefusalKind::NonFiniteValue, format!("{name} is {v}"));
    }
}

fn check_unit(set: &mut RefusalSet, name: &'static str, v: Option<f64>) {
    if let Some(v) = v {
        check_non_finite(set, name, v);
        if v.is_finite() && !(0.0..=1.0).contains(&v) {
            set.refuse(
                RefusalKind::OutOfDeclaredRange,
                format!("{name} is {v}, outside OpenPBR's declared [0, 1]"),
            );
        }
    }
}

fn check_colour(set: &mut RefusalSet, name: &'static str, c: LinearRgb) {
    check_non_finite(set, name, c.r);
    check_non_finite(set, name, c.g);
    check_non_finite(set, name, c.b);
    if c.is_finite() && !c.in_unit_range() {
        set.refuse(
            RefusalKind::OutOfDeclaredRange,
            format!("{name} is ({}, {}, {}), outside OpenPBR's declared [0, 1]", c.r, c.g, c.b),
        );
    }
}

/// The shading inputs of one admitted material, derived from block truth.
///
/// `neighbour` is the material on the outward side of the face, which is the
/// whole of the hydrology adjacency signal: a face looking at fluid is wet.
///
/// The registry's own copy of the material is used, never the caller's — a
/// hand-built [`Material`] with the right id must not be able to shade as a
/// material the registry never admitted.
pub fn derive(
    material: &Material,
    registry: &MaterialRegistry,
    neighbour: Option<MaterialId>,
) -> Refused<OpenPbrInputs> {
    let admitted = registry.material(material.id)?;

    // The weathering scale is the admitted set's **own** density range, and it is
    // taken over the **opaque** materials only — the population the claim is about.
    // Two things follow. A liquid's density cannot stretch a rock's scale, and
    // admitting a fluid cannot restyle every rock in the world. Nothing is
    // hard-coded either way: a reference density would be an invented constant.
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    let mut opaque = 0usize;
    for m in registry.materials() {
        if m.optical_class != OpticalClass::Opaque {
            continue;
        }
        let d = m.bulk_density_kg_m3().to_f64();
        min = min.min(d);
        max = max.max(d);
        opaque += 1;
    }
    // An empty set leaves `min`/`max` at their sentinels and a single material
    // leaves them equal, so both cases are the same refusal rather than a
    // division by a zero-width range.
    let has_range = min.is_finite() && max.is_finite() && max > min;
    if !has_range {
        return Err(Refusal::owned(
            RefusalKind::MissingStage,
            format!(
                "the admitted set has no opaque density range to derive a weathering scale from: {opaque} opaque material(s), {min}..{max} kg/m3"
            ),
        ));
    }

    let density = admitted.bulk_density_kg_m3().to_f64();
    if !density.is_finite() {
        return Err(Refusal::owned(
            RefusalKind::NonFiniteValue,
            format!("{} has a non-finite bulk density", admitted.name),
        ));
    }
    // Porous is rough. Denser than the admitted set is smoother than all of it.
    let porosity = ((max - density) / (max - min)).clamp(0.0, 1.0);

    let wet = match neighbour {
        None => false,
        Some(id) => registry.material(id)?.optical_class == OpticalClass::Transmissive,
    };
    let transmits = admitted.optical_class == OpticalClass::Transmissive;

    // **The one parameter this derivation used to refuse outright.** A material
    // that carries a sourced reflectance admits a `base_color`; a material that
    // does not still gets `Colour::Unadmitted`, and that stays representable
    // because inventing a grey for it is the failure mode this field exists to
    // remove. Until this round *every* material was in the second case, and the
    // world drew black — see `docs/LEDGER.md` C-38.
    let base_color = match admitted.reflectance {
        Some(sourced) => {
            let [r, g, b] = sourced.value.linear();
            let rgb = LinearRgb::new(r, g, b);
            // Refuse rather than clamp: a reflectance outside 0..=1 is a bad
            // measurement, and a clamped one looks like a bright surface.
            if rgb.in_unit_range() && rgb.is_finite() {
                Colour::Admitted(rgb)
            } else {
                return Err(Refusal::owned(
                    RefusalKind::OutOfDeclaredRange,
                    format!(
                        "{} carries a reflectance outside 0..=1: {r}, {g}, {b}",
                        admitted.name
                    ),
                ));
            }
        }
        None => Colour::Unadmitted,
    };

    Ok(OpenPbrInputs {
        base_color,
        base_metalness: 0.0,
        specular_roughness: if transmits { None } else { Some(porosity) },
        specular_ior: None,
        specular_weight: None,
        coat_weight: if wet { 1.0 } else { 0.0 },
        coat_roughness: None,
        transmission_weight: if transmits { 1.0 } else { 0.0 },
        transmission_depth: None,
        subsurface_weight: None,
        emission: Emission::None,
    })
}

/// The shading inputs of a whole visible face: each covering material derived,
/// then blended by its share of the face area.
///
/// Returns a refusal for a face with no matter on it (there is nothing to shade)
/// and for any covering material the registry does not admit.
pub fn from_composition(
    composition: &VoxelComposition,
    registry: &MaterialRegistry,
    neighbour: Option<MaterialId>,
) -> Refused<OpenPbrInputs> {
    let mut parts: Vec<(f64, OpenPbrInputs)> = Vec::with_capacity(composition.materials().len());
    for (id, share) in composition.materials() {
        let material = registry.material(*id)?;
        parts.push((share.to_f64(), derive(material, registry, neighbour)?));
    }
    if parts.is_empty() {
        return Err(Refusal::new(
            RefusalKind::MissingStage,
            "a face with no matter on it has nothing to shade",
        ));
    }
    Ok(blend(&parts))
}

/// Blend parts by weight, with one rule that matters: **a parameter that is
/// absent in any part is absent in the blend.**
///
/// The obvious alternative — average over the parts that have it — silently
/// treats a missing input as if the part that lacks it did not exist, which is a
/// default wearing an average's clothes.
pub fn blend(parts: &[(f64, OpenPbrInputs)]) -> OpenPbrInputs {
    let scalar = |f: &dyn Fn(&OpenPbrInputs) -> Option<f64>| -> Option<f64> {
        let mut acc = 0.0;
        let mut total = 0.0;
        for (w, inputs) in parts {
            let v = f(inputs)?;
            acc += w * v;
            total += w;
        }
        if total > 0.0 {
            Some(acc / total)
        } else {
            None
        }
    };
    let required = |f: &dyn Fn(&OpenPbrInputs) -> f64| -> f64 {
        let mut acc = 0.0;
        let mut total = 0.0;
        for (w, inputs) in parts {
            acc += w * f(inputs);
            total += w;
        }
        if total > 0.0 {
            acc / total
        } else {
            0.0
        }
    };

    OpenPbrInputs {
        base_color: blend_colour(parts),
        base_metalness: required(&|i| i.base_metalness),
        specular_roughness: scalar(&|i| i.specular_roughness),
        specular_ior: scalar(&|i| i.specular_ior),
        specular_weight: scalar(&|i| i.specular_weight),
        coat_weight: required(&|i| i.coat_weight),
        coat_roughness: scalar(&|i| i.coat_roughness),
        transmission_weight: required(&|i| i.transmission_weight),
        transmission_depth: scalar(&|i| i.transmission_depth),
        subsurface_weight: scalar(&|i| i.subsurface_weight),
        emission: blend_emission(parts),
    }
}

fn blend_colour(parts: &[(f64, OpenPbrInputs)]) -> Colour {
    let mut acc = LinearRgb::BLACK;
    let mut total = 0.0;
    for (w, inputs) in parts {
        match inputs.base_color {
            // Absent in any part is absent in the blend.
            Colour::Unadmitted => return Colour::Unadmitted,
            Colour::Admitted(c) => {
                acc = acc + c * *w;
                total += w;
            }
        }
    }
    if total > 0.0 {
        Colour::Admitted(acc * (1.0 / total))
    } else {
        Colour::Unadmitted
    }
}

fn blend_emission(parts: &[(f64, OpenPbrInputs)]) -> Emission {
    if parts.iter().all(|(_, i)| i.emission == Emission::None) {
        return Emission::None;
    }
    // A non-emitter contributes no photons, so the colour is averaged over the
    // emitting parts only. That is physical rather than a renormalising default.
    let mut color = LinearRgb::BLACK;
    let mut colour_weight = 0.0;
    let mut luminance = 0.0;
    let mut total = 0.0;
    for (w, inputs) in parts {
        luminance += w * inputs.emission.luminance();
        total += w;
        if let Emission::Emitted { color: c, .. } = inputs.emission {
            color = color + c * *w;
            colour_weight += w;
        }
    }
    let color = if colour_weight > 0.0 { color * (1.0 / colour_weight) } else { LinearRgb::BLACK };
    Emission::Emitted { color, luminance: if total > 0.0 { luminance / total } else { 0.0 } }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_model::{Component, ComponentId, Material, Ratio};
    use voxel_shapers::column::materials::{self, BEDROCK, IGNEOUS, REGOLITH, SEDIMENTARY, WATER};

    fn registry() -> MaterialRegistry {
        materials::registry().unwrap()
    }

    /// A registry whose `IGNEOUS` material carries **no** reflectance and whose
    /// `SEDIMENTARY` one does.
    ///
    /// This is the fixture that holds the `Sourced` rule from both sides: in one
    /// world, the derivation must produce a colour for one material and refuse
    /// one for the other. Using the real registry for both halves would only ever
    /// test the half that is present.
    fn registry_with_one_reflectance_missing() -> MaterialRegistry {
        use voxel_model::{Reflectance, SourcedReflectance};
        let mut r = MaterialRegistry::new();
        r.admit_component(ComponentId(1), "silicate rock");
        let one = |id: MaterialId, name: &str, density: u64, reflectance| Material {
            id,
            name: name.to_string(),
            components: vec![Component {
                id: ComponentId(1),
                ratio: Ratio::ONE,
                density_kg_m3: Ratio::from_u64(density),
            }],
            optical_class: OpticalClass::Opaque,
            reflectance,
        };
        r.admit_material(one(IGNEOUS, "bare rock", 2900, None)).unwrap();
        r.admit_material(one(
            SEDIMENTARY,
            "dressed rock",
            2400,
            Some(SourcedReflectance {
                value: Reflectance::broadband(Ratio::new(17, 100).unwrap()).unwrap(),
                source: "a test fixture, and the field says so",
            }),
        ))
        .unwrap();
        r
    }

    fn one(id: MaterialId) -> Material {
        registry().material(id).unwrap().clone()
    }

    /// A complete set, as an admitted optical table would supply one. Used to
    /// prove the *shading* path: the derivation above cannot produce one, which
    /// is exactly the debt it reports.
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

    #[test]
    fn roughness_comes_from_the_material_s_own_density_and_is_monotone() {
        // The weathering signal: within the admitted set, denser is smoother.
        let reg = registry();
        let mut ordered: Vec<(f64, f64)> = [REGOLITH, IGNEOUS, BEDROCK]
            .into_iter()
            .map(|id| {
                let m = reg.material(id).unwrap();
                (
                    m.bulk_density_kg_m3().to_f64(),
                    derive(m, &reg, None).unwrap().specular_roughness.unwrap(),
                )
            })
            .collect();
        ordered.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        // Ascending density, strictly descending roughness.
        for pair in ordered.windows(2) {
            assert!(
                pair[0].1 > pair[1].1,
                "roughness did not fall as density rose: {pair:?}"
            );
        }
        // The extremes of the **opaque** admitted set are the extremes of the
        // scale: regolith at 1400 kg/m3 is the roughest and bedrock at 3000 the
        // smoothest, so 1400 -> 1.0 and 3000 -> 0.0. Ice (917) is admitted and
        // lighter than either, and it does **not** stretch the scale, because a
        // liquid is not what the weathering claim is about.
        let regolith = derive(reg.material(REGOLITH).unwrap(), &reg, None).unwrap();
        assert_eq!(regolith.specular_roughness, Some(1.0));
        let bedrock = derive(reg.material(BEDROCK).unwrap(), &reg, None).unwrap();
        assert_eq!(bedrock.specular_roughness, Some(0.0));
    }

    #[test]
    fn transmission_is_never_on_a_solid() {
        let reg = registry();
        let rock = derive(reg.material(IGNEOUS).unwrap(), &reg, None).unwrap();
        assert_eq!(rock.transmission_weight, 0.0);
        let water = derive(reg.material(WATER).unwrap(), &reg, None).unwrap();
        assert_eq!(water.transmission_weight, 1.0);
    }

    #[test]
    fn a_transmissive_material_has_no_weathering_scale_to_read() {
        // The porosity relation is a claim about rock. A liquid's surface
        // roughness is a different physical quantity, so it is absent rather than
        // borrowed from a rock's scale.
        let reg = registry();
        let water = derive(reg.material(WATER).unwrap(), &reg, None).unwrap();
        assert_eq!(water.specular_roughness, None);
        assert!(water.unavailable().contains(&Parameter::SpecularRoughness));
    }

    #[test]
    fn a_wet_neighbour_raises_the_coat_and_a_dry_one_does_not() {
        let reg = registry();
        let dry = derive(reg.material(IGNEOUS).unwrap(), &reg, Some(IGNEOUS)).unwrap();
        assert_eq!(dry.coat_weight, 0.0);
        let wet = derive(reg.material(IGNEOUS).unwrap(), &reg, Some(WATER)).unwrap();
        assert_eq!(wet.coat_weight, 1.0);
        let edge = derive(reg.material(IGNEOUS).unwrap(), &reg, None).unwrap();
        assert_eq!(edge.coat_weight, 0.0);
    }

    #[test]
    fn an_unadmitted_neighbour_or_material_is_refused() {
        let reg = registry();
        let m = one(IGNEOUS);
        let r = derive(&m, &reg, Some(MaterialId(900))).unwrap_err();
        assert_eq!(r.kind(), RefusalKind::UnadmittedMaterial);

        // A hand-built material with an id the registry never admitted must not
        // shade as that id.
        let forged = Material {
            id: MaterialId(901),
            name: "forged".into(),
            components: vec![Component {
                id: ComponentId(1),
                ratio: Ratio::ONE,
                density_kg_m3: Ratio::from_u64(1),
            }],
            optical_class: OpticalClass::Opaque,
            reflectance: None,
        };
        assert_eq!(
            derive(&forged, &reg, None).unwrap_err().kind(),
            RefusalKind::UnadmittedMaterial
        );
    }

    #[test]
    fn a_registry_with_no_density_range_is_refused_rather_than_defaulted() {
        // One material is not a range. The alternative is to invent a reference
        // density, which the four-bucket rule forbids.
        let m = one(IGNEOUS);
        let mut single = MaterialRegistry::new();
        single.admit_component(ComponentId(1), "silicate rock");
        single
            .admit_material(Material {
                id: IGNEOUS,
                name: "only".into(),
                components: vec![Component {
                    id: ComponentId(1),
                    ratio: Ratio::ONE,
                    density_kg_m3: Ratio::from_u64(2900),
                }],
                optical_class: OpticalClass::Opaque,
            reflectance: None,
            })
            .unwrap();
        let r = derive(&m, &single, None).unwrap_err();
        assert_eq!(r.kind(), RefusalKind::MissingStage);
        assert!(r.detail().contains("no opaque density range"), "{}", r.detail());
    }

    #[test]
    fn a_rock_derives_exactly_seven_of_the_twelve_and_the_rest_are_named() {
        // **Seven, not six.** `base_color` left this list when the material record
        // gained a sourced reflectance (C-39); before that it was absent for every
        // material, and the world drew black (C-38).
        let reg = registry();
        let inputs = derive(reg.material(IGNEOUS).unwrap(), &reg, None).unwrap();
        assert_eq!(
            inputs.base_color,
            Colour::Admitted(LinearRgb::grey(0.12)),
            "an igneous rock's sourced reflectance is admitted, and it is the value the record carries"
        );
        let absent = inputs.unavailable();
        assert_eq!(
            absent,
            vec![
                Parameter::SpecularIor,
                Parameter::SpecularWeight,
                Parameter::CoatRoughness,
                Parameter::TransmissionDepth,
                Parameter::SubsurfaceWeight,
            ]
        );
        // Seven present, five absent: the twelve are accounted for, not merely some.
        assert_eq!(absent.len(), 5);
        assert_eq!(Parameter::ALL.len(), 12);
    }

    #[test]
    fn a_material_with_no_reflectance_still_derives_no_colour() {
        // The `Sourced` state, held from both sides: the rule says `base_color` is
        // derived **when the record carries it**, so a material without one must
        // still be `Absent` — and that is not a regression, it is the refusal to
        // invent a grey.
        let reg = registry_with_one_reflectance_missing();
        let bare = derive(reg.material(IGNEOUS).unwrap(), &reg, None).unwrap();
        assert_eq!(bare.base_color, Colour::Unadmitted);
        assert!(bare.unavailable().contains(&Parameter::BaseColor));
        // And the same registry's other opaque material does have one, so the two
        // states are distinguishable inside one world.
        let dressed = derive(reg.material(SEDIMENTARY).unwrap(), &reg, None).unwrap();
        assert!(matches!(dressed.base_color, Colour::Admitted(_)));
    }

    #[test]
    fn the_absent_parameters_are_exactly_the_ones_the_rule_calls_absent() {
        // The rule at the top of the module, held to itself: for a material whose
        // parameters are all derivable, `unavailable()` is the `Absent` set.
        let declared: Vec<Parameter> = Parameter::ALL
            .into_iter()
            .filter(|p| matches!(p.availability(), Availability::Absent(_)))
            .collect();
        let reg = registry();
        let inputs = derive(reg.material(IGNEOUS).unwrap(), &reg, None).unwrap();
        let mut got = inputs.unavailable();
        got.sort_by_key(|p| Parameter::ALL.iter().position(|q| q == p).unwrap());
        assert_eq!(got, declared);
        assert_eq!(declared.len(), 5);
    }

    #[test]
    fn every_parameter_has_a_name_and_a_reason_and_no_two_are_the_same() {
        let mut names: Vec<&str> = Parameter::ALL.iter().map(|p| p.name()).collect();
        names.sort_unstable();
        let unique = names.len();
        names.dedup();
        assert_eq!(names.len(), unique);
        for p in Parameter::ALL {
            assert!(!p.reason().is_empty(), "{} has no reason", p.name());
        }
        // The derivations are distinct sentences, so a reader learns something
        // different from each one.
        let mut derived: Vec<&str> = Parameter::ALL
            .iter()
            .filter_map(|p| match p.availability() {
                Availability::Derived(why) => Some(why),
                Availability::Absent(_) | Availability::Sourced(_) => None,
            })
            .collect();
        derived.sort_unstable();
        derived.dedup();
        // metalness, roughness, coat, transmission and the two emission rows —
        // the emission pair legitimately shares one reason.
        assert_eq!(derived.len(), 5);
    }

    #[test]
    fn a_complete_set_validates_and_a_placeholder_does_not_survive_the_ranges() {
        assert!(stated().validate().is_ok());
        let bad = OpenPbrInputs {
            base_metalness: 1.5,
            specular_roughness: Some(-0.1),
            specular_ior: Some(0.5),
            ..stated()
        };
        let err = bad.validate().unwrap_err();
        assert_eq!(err.len(), 3, "every bad parameter must be reported at once: {err}");
    }

    #[test]
    fn a_non_finite_value_is_refused_by_its_own_kind() {
        let bad = OpenPbrInputs { specular_roughness: Some(f64::NAN), ..stated() };
        let err = bad.validate().unwrap_err();
        assert_eq!(err.all()[0].kind(), RefusalKind::NonFiniteValue);
    }

    #[test]
    fn detail_moves_the_value_toward_the_edge_it_has_room_for_and_never_past_it() {
        let base = stated();
        for delta in [-1.0, -0.5, 0.0, 0.5, 1.0] {
            let moved = base
                .with_detail(DetailSample { roughness_delta: delta, reflectance_delta: delta })
                .unwrap();
            assert!(moved.validate().is_ok(), "delta {delta} left the declared domain");
            let r = moved.specular_roughness.unwrap();
            assert!((0.0..=1.0).contains(&r));
            match moved.base_color {
                Colour::Admitted(c) => assert!(c.in_unit_range()),
                Colour::Unadmitted => panic!("a stated colour must survive detail"),
            }
        }
        // Monotone in the delta, and the extreme deltas reach the domain's edges.
        let down = base.with_detail(DetailSample { roughness_delta: -1.0, reflectance_delta: 0.0 }).unwrap();
        let up = base.with_detail(DetailSample { roughness_delta: 1.0, reflectance_delta: 0.0 }).unwrap();
        assert_eq!(down.specular_roughness, Some(0.0));
        assert_eq!(up.specular_roughness, Some(1.0));
        // Delta zero is the identity — the lattice's own mean.
        let none = base.with_detail(DetailSample::NONE).unwrap();
        assert_eq!(none, base);
    }

    #[test]
    fn detail_cannot_invent_an_albedo_a_material_does_not_have() {
        // The 256× lattice varies a surface; it does not *supply* one. So on a
        // material whose record has no reflectance, detail varies the roughness
        // and leaves the colour absent.
        let reg = registry_with_one_reflectance_missing();
        let rock = derive(reg.material(IGNEOUS).unwrap(), &reg, None).unwrap();
        let varied = rock
            .with_detail(DetailSample { roughness_delta: 0.9, reflectance_delta: 0.9 })
            .unwrap();
        assert_eq!(varied.base_color, Colour::Unadmitted);
        assert!(varied.unavailable().contains(&Parameter::BaseColor));
        // But the roughness it does have is varied.
        assert_ne!(varied.specular_roughness, rock.specular_roughness);

        // And where the record *does* carry one, detail varies it rather than
        // replacing it.
        let reg = registry();
        let rock = derive(reg.material(IGNEOUS).unwrap(), &reg, None).unwrap();
        let varied = rock
            .with_detail(DetailSample { roughness_delta: 0.0, reflectance_delta: 0.5 })
            .unwrap();
        match (rock.base_color, varied.base_color) {
            (Colour::Admitted(a), Colour::Admitted(b)) => {
                assert!(a != b, "a reflectance delta of 0.5 changed nothing");
                assert!(b.luminance() > a.luminance(), "a positive delta brightens");
            }
            other => panic!("both should be admitted: {other:?}"),
        }
    }

    #[test]
    fn a_non_finite_detail_sample_is_refused() {
        let r = stated()
            .with_detail(DetailSample { roughness_delta: f64::INFINITY, reflectance_delta: 0.0 })
            .unwrap_err();
        assert_eq!(r.kind(), RefusalKind::NonFiniteValue);
    }

    #[test]
    fn a_blend_that_lacks_a_parameter_anywhere_lacks_it_everywhere() {
        // One part complete, one part a derived rock whose record carries no
        // reflectance. `Absent` in any part means `Absent` in the blend — the
        // rule that stops a missing input from being averaged away.
        let reg = registry_with_one_reflectance_missing();
        let rock = derive(reg.material(IGNEOUS).unwrap(), &reg, None).unwrap();
        let blended = blend(&[(0.5, stated()), (0.5, rock)]);
        assert_eq!(blended.base_color, Colour::Unadmitted);
        // And the parameter both parts *do* have is averaged by weight.
        let only_stated = blend(&[(1.0, stated())]);
        assert_eq!(only_stated.base_color, stated().base_color);

        // With both parts sourced, the blend averages instead of vanishing.
        let reg = registry();
        let rock = derive(reg.material(IGNEOUS).unwrap(), &reg, None).unwrap();
        match blend(&[(0.5, stated()), (0.5, rock)]).base_color {
            Colour::Admitted(c) => {
                let want = (0.4 + 0.12) / 2.0;
                assert!((c.r - want).abs() < 1e-12, "colour was {c:?}, wanted {want}");
            }
            Colour::Unadmitted => panic!("both parts carried a colour"),
        }
    }

    #[test]
    fn a_blend_of_two_stated_materials_is_weighted_by_coverage() {
        let dark = stated();
        let bright = OpenPbrInputs {
            base_color: Colour::Admitted(LinearRgb::grey(0.8)),
            specular_roughness: Some(0.75),
            ..stated()
        };
        let blended = blend(&[(0.25, dark), (0.75, bright)]);
        // 0.4*(1/4) + 0.8*(3/4) = 0.7
        match blended.base_color {
            Colour::Admitted(c) => assert!((c.r - 0.7).abs() < 1e-12, "colour was {c:?}"),
            Colour::Unadmitted => panic!("both parts had a colour"),
        }
        assert!((blended.specular_roughness.unwrap() - 0.65).abs() < 1e-12);
    }

    #[test]
    fn a_face_with_no_matter_is_refused_rather_than_shaded_as_a_hole() {
        use voxel_model::VoxelShape;
        use voxel_model::Face;
        let reg = registry();
        let c = VoxelComposition::of_face(&VoxelShape::empty(), Face::PosZ, &reg).unwrap();
        let r = from_composition(&c, &reg, None).unwrap_err();
        assert_eq!(r.kind(), RefusalKind::MissingStage);
    }

    #[test]
    fn a_two_material_face_blends_both_derivations() {
        use voxel_model::{Face, SubvoxelIndex, VoxelShape};
        let reg = registry();
        let mut shape = VoxelShape::empty();
        for idx in SubvoxelIndex::all() {
            // A zone of soil sitting on igneous rock, so the face sees both.
            shape.set(idx, if idx.z == 3 { REGOLITH } else { IGNEOUS });
        }
        let c = VoxelComposition::of_face(&shape, Face::PosX, &reg).unwrap();
        let blended = from_composition(&c, &reg, Some(WATER)).unwrap();
        // Both are opaque, so both derived roughness; the wet neighbour wets both.
        assert!(blended.specular_roughness.is_some());
        assert_eq!(blended.coat_weight, 1.0);
        // Both carry a sourced reflectance, so the face has a colour that is the
        // coverage-weighted blend of the two — 17 % soil and 12 % igneous rock
        // come out between them, and the face is *not* the darker or the lighter
        // of the pair.
        match blended.base_color {
            Colour::Admitted(c) => {
                assert!(c.r > 0.12 && c.r < 0.17, "the blend is between the two: {c:?}");
            }
            Colour::Unadmitted => panic!("both materials carry a reflectance"),
        }
    }

    #[test]
    fn the_luminance_of_a_grey_is_that_grey() {
        // The weights sum to one, so this is a check that the luminance actually
        // uses them rather than one channel.
        assert!((LinearRgb::grey(0.37).luminance() - 0.37).abs() < 1e-12);
        assert!(LinearRgb::WHITE.luminance() > LinearRgb::BLACK.luminance());
    }

    #[test]
    fn an_emission_free_blend_stays_emission_free() {
        let blended = blend(&[(0.5, stated()), (0.5, stated())]);
        assert_eq!(blended.emission, Emission::None);
    }
}
