//! The material table the shader reads, and the CPU reference it has to agree
//! with.
//!
//! # Why there is a table at all
//!
//! `npr::shade` is the model. It is `f64`, it lives in `voxel-appearance`, and
//! it is what the FMA bar's band count is measured against. A GPU shader cannot
//! call it, so the four Source-2 terms are written a **second time** in
//! `shading.wgsl` — and a second copy of a model is exactly how two models
//! drift apart.
//!
//! So the copy is not trusted. [`shade_cpu`] is the reference, and
//! `renderer::tests::the_gpu_shades_a_surface_exactly_as_the_crate_does`
//! renders stated normals and compares the pixels against it. The duplication is
//! therefore not a hope; it is an obligation with a test on it.
//!
//! # The `_present` flags, and why they are not booleans in disguise
//!
//! `npr::shade` distinguishes a term whose **input is absent** from one that
//! evaluated to zero — that is what `Terms` is for, and an absent `base_color`
//! removes the diffuse lobe rather than making it black. A shader that read a
//! missing roughness as `0.0` would draw a **mirror** where the crate draws
//! nothing. So each optional input travels with a flag, and the fragment
//! shader's branches are on the flag.
//!
//! # The one value this layer cannot derive, and it is named
//!
//! `SurfaceFrame::open` — the share of a visible face that holds no matter — is
//! a property of a **voxel's shape**. The mesher merges quads across many voxels
//! of the same material (`C-17`), so a merged quad legitimately has no single
//! openness: there is no per-quad value to carry, and the mesh does not carry
//! one. The client therefore shades every quad with
//! [`MESH_DOES_NOT_CARRY_OPEN`], which is 0 — and it is recorded as **debt with
//! a name**, not as a derived value, because the day the mesh carries openness
//! this constant is what has to disappear.

use voxel_appearance::npr::{self, LightRig, SurfaceFrame, Terms};
use voxel_appearance::openpbr::{self, Colour, Emission, LinearRgb, OpenPbrInputs};
use voxel_model::{MaterialRegistry, Refusal, ShadingField};
use voxel_noise::Vec3;

/// See the module comment. Zero because a merged quad has no hole it can name —
/// **not** because a hole would not matter.
pub const MESH_DOES_NOT_CARRY_OPEN: f64 = 0.0;

/// How many lights the rig uniform can hold. A rig with more is refused rather
/// than truncated, because a dropped light silently changes every pixel.
pub const RIG_CAPACITY: usize = 4;

/// One material's twelve OpenPBR inputs, packed as eight `vec4`s.
///
/// The grouping is not cosmetic. A WGSL `array<T>` element stride must be a
/// multiple of 16 bytes, so `vec4`s (alignment 16) are the layout that needs no
/// padding constant invented to explain it. The field names match `Material` in
/// `shading.wgsl`. The fifth `vec4` is C-161's: the **pattern gains** — how much of
/// the 256-level lattice's deviation this material shows, per channel — which the
/// shader reads off the row it already fetched.
///
/// # The three groups at the end, and why they exist at all
///
/// `openpbr::derive` computes every one of the twelve parameters and `npr::shade`
/// reads every one of them, but five had **nowhere in the row to go** — the
/// boundary's own `specular_ior`, the coat's weight and roughness, and the
/// participating medium's depth and scattering. The shader therefore drew the
/// boundary with no reflectance, no coat and no medium, and the CPU's picture and
/// the GPU's were two pictures. Carrying them is what
/// `shading.wgsl`'s `specular_boundary`/`participating_medium`/`medium_flags` read,
/// and the gate that names the gap is
/// `no_openpbr_term_is_silently_dropped_on_the_way_to_the_gpu`.
///
/// **Every value travels with its own presence flag**, the same rule
/// [`MaterialParams::spec_emission`] follows, because an absent parameter is not a
/// zero: a coat with no admitted roughness draws no coat rather than a perfectly
/// smooth one.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct MaterialParams {
    /// `base_color.rgb`, and `.w` is 1 when the colour is **admitted**.
    pub base_color_and_present: [f32; 4],
    /// `roughness`, `roughness present`, `base_metalness`, `transmission_weight`.
    pub rough_metal: [f32; 4],
    /// `specular_weight`, `weight present`, `emission luminance`, `emission present`.
    pub spec_emission: [f32; 4],
    /// `emission.rgb`, and `.w` is the `open` fraction.
    pub emission_rgb_open: [f32; 4],
    /// `roughness gain`, `reflectance gain`, `surface shaper id`, one reserved slot.
    ///
    /// Zero for a material the world's [`ShadingField`] does not vary — which is
    /// what makes `MaterialParams::from_inputs`'s stated rows shade exactly as
    /// they did before the pattern existed, and is why the probe tests could keep
    /// their `f64` references unchanged. The shaper id is
    /// [`voxel_shapers::materials::SurfacePattern::index`], which
    /// `shading.wgsl` dispatches on (C-162).
    pub pattern: [f32; 4],
    /// `specular_f0`, `f0 admitted`, `coat_weight`, `coat_roughness`.
    ///
    /// The reflectance is the **derived** one
    /// ([`voxel_appearance::provenance::specular_f0`] read `specular_ior`), not the
    /// index: the shader cannot call Rust, and copying the Fresnel relation into
    /// WGSL would be a second statement of it. The other three are raw inputs.
    pub specular_boundary: [f32; 4],
    /// `coat roughness admitted`, `transmission_depth`, `depth admitted`,
    /// `subsurface_weight`.
    pub participating_medium: [f32; 4],
    /// `subsurface admitted`, and the room the next admitted parameter takes — the
    /// same reserve [`MaterialParams::pattern`]'s fourth slot keeps.
    pub medium_flags: [f32; 4],
}

/// How much of the world's material table can actually be **shaded**.
///
/// This pair decides whether the world can be drawn at all. The appearance layer
/// refuses to invent a reflectance, so a table with no `base_color` in any row
/// shades nothing but the rim — and `0 of 7` is the difference between "the
/// renderer is broken" and "the material record has no colour in it, by rule".
/// Measured, with the consequence, in `docs/LEDGER.md` C-38.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ShadingReport {
    /// Rows the table holds, one per admitted material.
    pub materials: usize,
    /// Rows whose `base_color` is **admitted**.
    pub with_base_colour: usize,
}

impl ShadingReport {
    /// Count the table's **materials**, which is every row except the first.
    ///
    /// Row 0 is `MaterialParams::default()` — the sentinel for `MaterialId::EMPTY`,
    /// which the shader never reads because air is not drawn. Counting it would
    /// make the HUD report `7 of 8` on a world where every material shades, which
    /// is a debt that does not exist and therefore worse than no readout at all.
    /// The first version of this function did exactly that.
    pub fn of(table: &[MaterialParams]) -> Self {
        let materials = table.iter().skip(1);
        ShadingReport {
            materials: materials.clone().count(),
            with_base_colour: materials
                .filter(|row| row.base_color_and_present[3] != 0.0)
                .count(),
        }
    }

    /// Whether any surface can be drawn with more than its rim.
    pub fn shades(&self) -> bool {
        self.with_base_colour > 0
    }
}

impl Default for MaterialParams {
    /// Every input **absent**. A shader reading this shades nothing, which is
    /// the honest answer for a material id that was never filled in.
    fn default() -> Self {
        MaterialParams {
            base_color_and_present: [0.0; 4],
            rough_metal: [0.0; 4],
            spec_emission: [0.0; 4],
            emission_rgb_open: [0.0; 4],
            pattern: [0.0; 4],
            specular_boundary: [0.0; 4],
            participating_medium: [0.0; 4],
            medium_flags: [0.0; 4],
        }
    }
}

impl MaterialParams {
    pub const SIZE: u64 = 128;

    pub fn from_inputs(inputs: &OpenPbrInputs, open: f64) -> Self {
        let (base, base_present) = match inputs.base_color {
            Colour::Admitted(c) => ([c.r as f32, c.g as f32, c.b as f32], 1.0),
            Colour::Unadmitted => ([0.0, 0.0, 0.0], 0.0),
        };
        let (emission, luminance, emission_present) = match inputs.emission {
            Emission::Emitted { color, luminance } => {
                ([color.r as f32, color.g as f32, color.b as f32], luminance as f32, 1.0)
            }
            Emission::None => ([0.0, 0.0, 0.0], 0.0, 0.0),
        };
        let (roughness, rough_present) = match inputs.specular_roughness {
            Some(v) => (v as f32, 1.0),
            None => (0.0, 0.0),
        };
        let (weight, weight_present) = match inputs.specular_weight {
            Some(v) => (v as f32, 1.0),
            None => (0.0, 0.0),
        };
        // **The index's own reflectance, derived once, here.** `specular_ior` reaches
        // the shader through the relation rather than as itself, so the Fresnel
        // boundary is stated in one language: `provenance::specular_f0`. An absent
        // index carries `0.0` with its flag clear, which is what the shader's own
        // `unwrap_or(1.0)` is applied to — the model's statement, not this row's.
        let (f0, f0_admitted) = match inputs.specular_ior {
            Some(ior) => (voxel_appearance::provenance::specular_f0(ior) as f32, 1.0),
            None => (0.0, 0.0),
        };
        let (coat_roughness, coat_roughness_admitted) = match inputs.coat_roughness {
            Some(v) => (v as f32, 1.0),
            None => (0.0, 0.0),
        };
        let (depth, depth_admitted) = match inputs.transmission_depth {
            Some(v) => (v as f32, 1.0),
            None => (0.0, 0.0),
        };
        let (subsurface, subsurface_admitted) = match inputs.subsurface_weight {
            Some(v) => (v as f32, 1.0),
            None => (0.0, 0.0),
        };
        MaterialParams {
            base_color_and_present: [base[0], base[1], base[2], base_present],
            rough_metal: [roughness, rough_present, inputs.base_metalness as f32, inputs.transmission_weight as f32],
            spec_emission: [weight, weight_present, luminance, emission_present],
            emission_rgb_open: [emission[0], emission[1], emission[2], open as f32],
            // A **stated** row varies nothing: the gains are the world's shading
            // field's, and [`table`] is what fills them in. Zero here is what keeps
            // `from_inputs` the same row it was before the pattern existed.
            pattern: [0.0; 4],
            specular_boundary: [f0, f0_admitted, inputs.coat_weight as f32, coat_roughness],
            participating_medium: [coat_roughness_admitted, depth, depth_admitted, subsurface],
            medium_flags: [subsurface_admitted, 0.0, 0.0, 0.0],
        }
    }

    /// The same row, with the material's pattern declaration set (C-161/C-162):
    /// the gains — how much of the lattice's deviation this material shows — and
    /// the surface shaper, which is how that deviation is shaped into the
    /// material's own character (speckle, foliage, bark, brushed).
    pub fn with_pattern(
        mut self,
        roughness_gain: f64,
        reflectance_gain: f64,
        surface: u32,
    ) -> Self {
        self.pattern = [roughness_gain as f32, reflectance_gain as f32, surface as f32, 0.0];
        self
    }

    pub fn bytes(&self) -> [u8; 128] {
        let mut out = [0u8; 128];
        let mut i = 0usize;
        for group in [
            self.base_color_and_present,
            self.rough_metal,
            self.spec_emission,
            self.emission_rgb_open,
            self.pattern,
            self.specular_boundary,
            self.participating_medium,
            self.medium_flags,
        ] {
            for v in group {
                out[i..i + 4].copy_from_slice(&voxel_model::endian::le_bytes(v));
                i += 4;
            }
        }
        out
    }
}

/// One light, packed for the shader. `.w` of `direction` is the intensity, so a
/// light is `vec4 + vec4` and the array stride is 32 with no padding.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct GpuLight {
    pub direction: [f32; 4],
    pub colour: [f32; 4],
}

impl Default for GpuLight {
    fn default() -> Self {
        GpuLight { direction: [0.0; 4], colour: [0.0; 4] }
    }
}

/// The lighting rig, packed for the shader.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct GpuRig {
    pub ambient: [f32; 4],
    pub count: [u32; 4],
    pub lights: [GpuLight; RIG_CAPACITY],
}

impl Default for GpuRig {
    fn default() -> Self {
        GpuRig { ambient: [0.0; 4], count: [0; 4], lights: [GpuLight::default(); RIG_CAPACITY] }
    }
}

impl GpuRig {
    pub const SIZE: u64 = 160;

    /// `None` when the rig holds more lights than the buffer does. Refusing is
    /// the point: truncating would drop a light and change every pixel in a way
    /// nothing downstream could see.
    pub fn from_rig(rig: &LightRig) -> Option<Self> {
        if rig.lights.len() > RIG_CAPACITY {
            return None;
        }
        let mut lights = [GpuLight::default(); RIG_CAPACITY];
        for (i, light) in rig.lights.iter().enumerate() {
            lights[i] = GpuLight {
                direction: [
                    light.direction.x as f32,
                    light.direction.y as f32,
                    light.direction.z as f32,
                    light.intensity as f32,
                ],
                colour: [light.colour.r as f32, light.colour.g as f32, light.colour.b as f32, 0.0],
            };
        }
        let mut count = [0u32; 4];
        count[0] = rig.lights.len() as u32;
        Some(GpuRig {
            ambient: [rig.ambient.r as f32, rig.ambient.g as f32, rig.ambient.b as f32, 0.0],
            count,
            lights,
        })
    }

    pub fn bytes(&self) -> [u8; 160] {
        let mut out = [0u8; 160];
        let mut i = 0usize;
        for v in self.ambient {
            out[i..i + 4].copy_from_slice(&voxel_model::endian::le_bytes(v));
            i += 4;
        }
        for v in self.count {
            out[i..i + 4].copy_from_slice(&voxel_model::endian::le_bytes(v));
            i += 4;
        }
        for light in self.lights {
            for v in light.direction.iter().chain(light.colour.iter()) {
                out[i..i + 4].copy_from_slice(&voxel_model::endian::le_bytes(*v));
                i += 4;
            }
        }
        out
    }
}

/// The material table, indexed by `MaterialId`.
///
/// Index 0 is the never-filled default and is never read: the mesher's
/// visibility rule is `!a.is_empty() && a != b && !is_opaque(b)`, so a quad's
/// material is never air. A material `derive` refuses **stops the table**: a
/// world that contains a material the appearance layer cannot describe is not a
/// world this client can draw, and shading it grey would hide that.
pub fn table(registry: &MaterialRegistry, shading: &ShadingField) -> Result<Vec<MaterialParams>, Refusal> {
    let mut highest = 0u16;
    for m in registry.materials() {
        highest = highest.max(m.id.0);
    }
    let mut out = vec![MaterialParams::default(); highest as usize + 1];
    for m in registry.materials() {
        let inputs = openpbr::derive(m, registry, None)?;
        // `None` is "not varied at this level at all" (`ShadingField::gain`), which
        // is a different claim from "varied by zero" and reaches the shader as
        // gains of zero — the material draws flat, as declared.
        let (roughness_gain, reflectance_gain) = shading.gain(m.id).unwrap_or((0.0, 0.0));
        let surface = voxel_shapers::materials::pattern_of(m.id).index();
        out[m.id.0 as usize] =
            MaterialParams::from_inputs(&inputs, MESH_DOES_NOT_CARRY_OPEN).with_pattern(
                roughness_gain,
                reflectance_gain,
                surface,
            );
    }
    Ok(out)
}

/// The pattern's identity, as the shader receives it: the lattice's seed and the
/// domain tag, each split into two `u32` because WGSL has no `u64`. Both travel
/// from Rust so the shader cannot hold a second copy of either.
///
/// The seed is the **world's own** declaration ([`materials::shading_field`] sets
/// its domain to [`materials::SHADING_FIELD_DOMAIN`]), read here from the constant
/// rather than from a field the caller happens to hold: one pipeline draws one
/// world, and `table()` built from a *different* field would be a second world
/// whose lattice this sentence would have to grow an argument for.
///
/// [`materials::shading_field`]: voxel_shapers::materials::shading_field
/// [`materials::SHADING_FIELD_DOMAIN`]: voxel_shapers::materials::SHADING_FIELD_DOMAIN
pub fn world_lattice_uniform() -> [u32; 4] {
    let seed = voxel_shapers::materials::SHADING_FIELD_DOMAIN;
    let tag = voxel_appearance::provenance::LATTICE_DOMAIN_TAG;
    [seed as u32, (seed >> 32) as u32, tag as u32, (tag >> 32) as u32]
}

/// The two sides, in samples per axis — the 4×4×4 subvoxel grid and the 256
/// lattice of C-122's two levels — sent as the model's own constants so the shader
/// derives its cell sizes rather than stating them.
///
/// These are sides and not metres on purpose: the metre is [`crate::pattern`]'s
/// declared voxel edge, and a shader carrying `0.25` would be a second copy of a
/// number that must move with `SUBVOXEL_SIDE`.
pub fn rates_uniform() -> [u32; 4] {
    [voxel_model::SUBVOXEL_SIDE, voxel_model::DETAIL_SIDE, 0, 0]
}

/// The same table, but keeping the inputs so a test can compare against the
/// crate directly.
pub fn inputs_for(
    registry: &MaterialRegistry,
    id: voxel_model::MaterialId,
) -> Result<OpenPbrInputs, Refusal> {
    let m = registry.material(id)?;
    openpbr::derive(m, registry, None)
}

/// The reference shading: `voxel-appearance`'s own `npr::shade`, in `f64`.
///
/// This is what the GPU is compared against. It is not a reimplementation, so a
/// disagreement between it and `shading.wgsl` is a disagreement between the
/// shipped model and the GPU copy — which is the only thing worth testing.
pub fn shade_cpu(
    inputs: &OpenPbrInputs,
    normal: Vec3,
    view: Vec3,
    open: f64,
    rig: &LightRig,
) -> Result<(LinearRgb, Terms), voxel_model::RefusalSet> {
    npr::shade(inputs, &SurfaceFrame { normal, view, open }, rig).map(|s| (s.colour, s.terms))
}

/// **A fluid surface, the whole composition**: [`shade_cpu`]'s body, then the sky
/// reflected by Fresnel — the Rust side of what `shading.wgsl` does in its
/// `transmission` branch.
///
/// `model` is the sky's own transport — the rows
/// [`crate::WorldPipeline::write_sky_model`] puts on the GPU, which the shader evaluates with
/// `atmosphere.wgsl`'s `sky_radiance_rgb`. The mirror ray is
/// [`voxel_appearance::fluid::reflected_ray`], the direction convention is
/// [`crate::skylight::sky_direction_of`], the reading of the sky is
/// [`crate::skylight::sky_exposed_at`], and the mix is
/// [`voxel_appearance::fluid::surface_radiance`]. This function only joins them,
/// which is why it can be compared against the shader pixel for pixel — and the sky it
/// reflects is the one the shader reflects, from one model and one exposure.
///
/// **Below the horizon the mirror ray reflects the world, not the sky** — and this
/// model does not draw the world in a reflection yet, so that case is no reflection
/// rather than a bright horizon. The shader makes the same guard.
pub fn fluid_surface_cpu(
    inputs: &OpenPbrInputs,
    normal: Vec3,
    view: Vec3,
    open: f64,
    rig: &LightRig,
    model: &crate::skylight::SkyModel,
) -> Result<(LinearRgb, Terms), voxel_model::RefusalSet> {
    let (body, terms) = shade_cpu(inputs, normal, view, open, rig)?;
    let ray = voxel_appearance::fluid::reflected_ray(view, normal);
    let reflected = if ray.y > 0.0 {
        let (elevation_deg, azimuth_deg) = crate::skylight::sky_direction_of(ray);
        crate::skylight::sky_exposed_at(model, elevation_deg, azimuth_deg)
    } else {
        LinearRgb::BLACK
    };
    let cos_theta = normal.normalise().dot(view.normalise()).clamp(0.0, 1.0);
    Ok((voxel_appearance::fluid::surface_radiance(body, reflected, cos_theta), terms))
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_model::MaterialId;

    /// **A material is not allowed to have its rim silently annihilated by arithmetic.**
    ///
    /// The rim is `reflectance × F(θ_L) × silhouette × intensity` — the share of the beam the
    /// interface reflected, carried by the block's own reflectance (`npr.rs`'s rim block). It
    /// used to be scaled by `(1 - roughness)`, and `openpbr::derive` sets roughness to the
    /// material's **porosity**, its position in the admitted set's *own density range*: the
    /// least dense material landed on `porosity = 1`, so its rim was exactly zero — with
    /// `Terms::rim` still reporting `true`, because the flag is set whether or not the term
    /// contributed. So this asserts the rim's **magnitude** at a grazing view, where a rim is
    /// strongest, and names every material that has none.
    #[test]
    fn no_material_loses_its_rim_to_arithmetic() {
        let registry =
            voxel_shapers::materials::registry().expect("the materials registry");
        let rig = LightRig::standard();

        let normal = Vec3::new(0.0, 1.0, 0.0);
        // Head-on: the fresnel factor is zero there, so **nothing** can differ but the rim.
        let head_on = normal;
        let grazing = Vec3::new(0.0, 0.02, 1.0).normalise();

        let mut no_rim: Vec<(String, &'static str)> = Vec::new();
        for material in registry.materials() {
            let inputs =
                openpbr::derive(material, &registry, None).expect("a material derives");
            let a = shade_cpu(&inputs, normal, head_on, 0.0, &rig).expect("head-on shades");
            let b = shade_cpu(&inputs, normal, grazing, 0.0, &rig).expect("grazing shades");
            if a.0 == b.0 {
                // **The cause the rim can lose to now, named rather than assumed.** The rim
                // carries the *block's* reflectance, so a material nobody has a spectrum for
                // has nothing for it to carry — and that is a named absence
                // (`APPEARANCE.md` §5), not arithmetic. Anything else has no excuse.
                no_rim.push((
                    material.name.clone(),
                    match inputs.base_color {
                        voxel_appearance::openpbr::Colour::Unadmitted => {
                            "no admitted reflectance, so the rim has nothing to carry"
                        }
                        voxel_appearance::openpbr::Colour::Admitted(c) if c.luminance() <= 0.0 => {
                            "a reflectance of zero, so the rim is exactly nothing at every angle"
                        }
                        _ => "unknown: a stated reflectance and still no rim",
                    },
                ));
            }
        }
        assert!(
            no_rim.is_empty(),
            "{} material(s) shade identically head-on and at a grazing view, so `npr.rs`'s rim \
             contributes nothing measurable at any angle: {no_rim:?}",
            no_rim.len()
        );
    }

    /// **The corpus census: every declared substance measured against the registry.**
    ///
    /// It ranges over the whole declaration rather than the world's own materials, and it
    /// **hands back the work list by name**: a substance with no admitted material is
    /// reported with what it is missing — a density, or a sourced reflectance — and one
    /// whose appearance is a *stated absence* is reported with its reason instead of being
    /// silently counted as done.
    ///
    /// Two rules keep it from becoming a rubber stamp. An absence in
    /// [`voxel_shapers::corpus::NO_REFLECTANCE`] with no reason is a defect, and an absence
    /// for a substance that **is** admitted is one too: the reason would have outlived the
    /// thing it explained, which is how a refusal turns into a lie.
    ///
    /// Printed, not only asserted — `cargo test -p voxel-client --lib the_corpus -- --nocapture`.
    #[test]
    fn the_corpus_names_what_each_unadmitted_substance_is_missing() {
        use std::collections::BTreeMap;
        let declared = voxel_model::materials::generated::table();
        let registry =
            voxel_shapers::materials::registry().expect("the materials registry");

        let admitted: BTreeMap<&str, &voxel_model::Material> = registry
            .materials()
            .map(|m| (m.name.as_str(), m))
            .collect();

        let mut covered = 0usize;
        let mut missing_density: Vec<String> = Vec::new();
        let mut missing_source: Vec<String> = Vec::new();
        let mut excused: Vec<String> = Vec::new();
        for (name, s) in &declared.substances {
            if let Some(material) = admitted.get(name.as_str()) {
                // **An admitted material is not the claim; a coloured one is.** The census
                // exists because a declared substance with no reflectance is drawn black by
                // arithmetic (`C-38`), so a name that matches a material carrying no
                // reflectance would be reported as done while staying invisible.
                assert!(
                    material.reflectance.is_some(),
                    "declared substance `{name}` has an admitted material that carries no \
                     reflectance, so it is counted as done while staying unlit"
                );
                covered += 1;
                continue;
            }
            match voxel_shapers::corpus::NO_REFLECTANCE
                .iter()
                .find(|(absent, _)| absent == name)
            {
                Some((_, reason)) => {
                    assert!(!reason.is_empty(), "{name} is excused with no reason");
                    excused.push(format!("{name}: {reason}"));
                }
                None if s.density_g_ml.is_none() => missing_density.push(name.clone()),
                None => missing_source.push(name.clone()),
            }
        }

        // **Every declared substance is in exactly one column.** A count that does not add
        // up is the census lying about which of the three it is, so it is checked before
        // it is printed.
        let accounted = covered + excused.len() + missing_density.len() + missing_source.len();
        assert_eq!(
            accounted,
            declared.substances.len(),
            "the census accounts for {accounted} of {} declared substances",
            declared.substances.len(),
        );

        println!(
            "corpus: {} declared substances; {} have an admitted material carrying a sourced \
             reflectance; {} are stated absences; {} lack a density; {} lack a sourced \
             reflectance",
            declared.substances.len(),
            covered,
            excused.len(),
            missing_density.len(),
            missing_source.len(),
        );
        for name in missing_density.iter().chain(missing_source.iter()) {
            println!("corpus-debt\t{name}");
        }
        for line in &excused {
            println!("corpus-absent\t{line}");
        }

        // **A reason may not outlive the thing it explained.** If one of these substances
        // is admitted after all, its absence entry is stale and this fails.
        for (absent, _) in voxel_shapers::corpus::NO_REFLECTANCE {
            assert!(
                !admitted.contains_key(*absent),
                "{absent} is excused from a reflectance and admitted anyway"
            );
        }

        assert!(
            missing_density.is_empty() && missing_source.is_empty(),
            "{} of {} declared substances have no admitted material: {} lack a density and {} \
             lack a sourced reflectance — see the printed work list",
            missing_density.len() + missing_source.len(),
            declared.substances.len(),
            missing_density.len(),
            missing_source.len(),
        );
    }

    /// **The model half of the census**: no `OpenPbrInputs` term may be absent without a
    /// named reason.
    ///
    /// The list below is a table of **rules**, not of excuses. Five of the six absent terms
    /// are absent for *every* admitted row and for the *same* one reason, so a per-material
    /// entry would be 380 copies of one sentence — and a drifted copy is a lie that is harder
    /// to see than a rule that does not bite. Each rule carries its own scope as a predicate,
    /// so it cannot be stretched to cover a row it does not describe, and the two checks at
    /// the end are the ones that keep it honest: **every** absence must match a rule, and
    /// **every** rule must match an absence — a reason may not outlive the thing it explained.
    #[test]
    fn no_absent_openpbr_term_is_unexplained() {
        /// One absent term: its name, why its absence is legitimate, and the rows it is absent for.
        type Absence = (&'static str, &'static str, fn(&voxel_model::Material) -> bool);

        let stated: [Absence; 6] = [
            (
                "specular_roughness",
                "a transmissive row's look is its Fresnel reflection (C-159.2), and a liquid's \
                 surface roughness is a different physical quantity from a rock's — the record \
                 states neither, so it is absent rather than invented",
                |m| m.optical_class == voxel_model::OpticalClass::Transmissive,
            ),
            (
                "specular_ior",
                "no record in this repository states a refractive index for a row, so it is absent \
                 rather than defaulted to a plausible 1.5 — a default is indistinguishable from \
                 an authored value, which is the one thing C-27's rule exists to prevent",
                |_| true,
            ),
            (
                "specular_weight",
                "no record states a specular weight for a row; C-27 keeps it absent rather than \
                 zero, because zero is a legitimate authored value and would read as one",
                |_| true,
            ),
            (
                "coat_roughness",
                "the model says a face is wet, not how the film is textured: the record carries no \
                 film roughness",
                |_| true,
            ),
            (
                "transmission_depth",
                "no row declares a transmission depth; a solid transmits nothing and the \
                 transmissive pair's own depth is a record the world does not carry",
                |_| true,
            ),
            (
                "subsurface_weight",
                "no row declares subsurface scattering; the record carries no scattering length, so \
                 it is absent rather than zeroed",
                |_| true,
            ),
        ];

        let registry =
            voxel_shapers::materials::registry().expect("the materials registry");
        let mut unexplained: Vec<String> = Vec::new();
        let mut explained = 0usize;
        let mut all: Vec<(&voxel_model::Material, Vec<&'static str>)> = Vec::new();

        for material in registry.materials() {
            let inputs = openpbr::derive(material, &registry, None)
                .unwrap_or_else(|e| panic!("{} derives: {e}", material.name));

            // Each `None` is asked: is there a rule that names it **and** covers this row?
            let mut absences: Vec<&'static str> = Vec::new();
            if inputs.specular_roughness.is_none() {
                absences.push("specular_roughness");
            }
            if inputs.specular_ior.is_none() {
                absences.push("specular_ior");
            }
            if inputs.specular_weight.is_none() {
                absences.push("specular_weight");
            }
            if inputs.coat_roughness.is_none() {
                absences.push("coat_roughness");
            }
            if inputs.transmission_depth.is_none() {
                absences.push("transmission_depth");
            }
            if inputs.subsurface_weight.is_none() {
                absences.push("subsurface_weight");
            }

            for term in &absences {
                let rule = stated
                    .iter()
                    .find(|entry| entry.0 == *term && (entry.2)(material));
                match rule {
                    Some((_, reason, _)) => {
                        assert!(
                            !reason.is_empty(),
                            "{term} is excused for {} with a blank reason",
                            material.name
                        );
                        explained += 1;
                    }
                    None => unexplained.push(format!("{}:{term}", material.name)),
                }
            }
            all.push((material, absences));
        }

        // **A rule may not outlive its absences.** One that matches nothing is a claim about a
        // state the admitted set has left, which is how a refusal turns into a lie.
        for (term, _, applies) in stated {
            let bites = all
                .iter()
                .any(|(m, absences)| applies(m) && absences.contains(&term));
            assert!(
                bites,
                "the rule for `{term}` matches no absence in the admitted set, so it is stale"
            );
        }

        assert!(
            unexplained.is_empty(),
            "{} absent term(s) have no named reason — `openpbr::derive` wrote a literal for \
             each; {explained} absence(s) are properly stated: {unexplained:?}",
            unexplained.len()
        );
    }

    /// **A material census** — the invariant is that no OpenPBR term is silently dropped
    /// between the model and the row the shader reads.
    ///
    /// Each term is probed **behaviourally**: two rows differing only in that term must
    /// differ in bytes. A list of names would only prove the list is stale, whereas this
    /// fails for as long as the term is dropped and starts passing the moment someone
    /// carries it, without being edited.
    #[test]
    fn no_openpbr_term_is_silently_dropped_on_the_way_to_the_gpu() {
        /// One term: its name and two setters whose difference must change the uploaded bytes.
        type Probe = (&'static str, fn(&mut OpenPbrInputs), fn(&mut OpenPbrInputs));

        // Every term `OpenPbrInputs` carries, each with a second value to vary it to.
        let probes: [Probe; 11] = [
            ("base_color", |i| i.base_color = Colour::Admitted(LinearRgb::WHITE), |i| i.base_color = Colour::Unadmitted),
            ("base_metalness", |i| i.base_metalness = 1.0, |i| i.base_metalness = 0.0),
            ("specular_roughness", |i| i.specular_roughness = Some(0.25), |i| i.specular_roughness = None),
            ("specular_ior", |i| i.specular_ior = Some(1.5), |i| i.specular_ior = None),
            ("specular_weight", |i| i.specular_weight = Some(0.5), |i| i.specular_weight = None),
            ("coat_weight", |i| i.coat_weight = 1.0, |i| i.coat_weight = 0.0),
            ("coat_roughness", |i| i.coat_roughness = Some(0.1), |i| i.coat_roughness = None),
            ("transmission_weight", |i| i.transmission_weight = 1.0, |i| i.transmission_weight = 0.0),
            ("transmission_depth", |i| i.transmission_depth = Some(0.3), |i| i.transmission_depth = None),
            ("subsurface_weight", |i| i.subsurface_weight = Some(0.4), |i| i.subsurface_weight = None),
            ("emission", |i| i.emission = Emission::Emitted { color: LinearRgb::WHITE, luminance: 2.0 }, |i| i.emission = Emission::None),
        ];

        let base = OpenPbrInputs {
            base_color: Colour::Admitted(LinearRgb::grey(0.4)),
            base_metalness: 0.0,
            specular_roughness: Some(0.35),
            specular_ior: None,
            specular_weight: None,
            coat_weight: 0.0,
            coat_roughness: None,
            transmission_weight: 0.0,
            transmission_depth: None,
            subsurface_weight: None,
            emission: Emission::None,
        };

        let mut dropped = Vec::new();
        for (name, set_a, set_b) in probes {
            let mut a = base;
            set_a(&mut a);
            let mut b = base;
            set_b(&mut b);
            if MaterialParams::from_inputs(&a, 0.0).bytes()
                == MaterialParams::from_inputs(&b, 0.0).bytes()
            {
                dropped.push(name);
            }
        }
        assert!(
            dropped.is_empty(),
            "OpenPbrInputs carries {} term(s) that MaterialParams ({} bytes) has nowhere \
             to put, so `derive` computes them and the shader never sees them: {dropped:?}",
            dropped.len(),
            MaterialParams::SIZE
        );
    }

    fn registry() -> MaterialRegistry {
        voxel_shapers::materials::registry().expect("the admitted registry builds")
    }

    #[test]
    fn the_packed_sizes_are_the_strides_wgsl_will_use() {
        assert_eq!(MaterialParams::SIZE, std::mem::size_of::<MaterialParams>() as u64);
        assert_eq!(GpuRig::SIZE, std::mem::size_of::<GpuRig>() as u64);
        // A WGSL array stride must be a multiple of 16 for both structs.
        assert_eq!(MaterialParams::SIZE % 16, 0);
        assert_eq!(GpuRig::SIZE % 16, 0);
    }

    #[test]
    fn an_absent_input_packs_as_absent_and_not_as_zero() {
        // The whole reason the flags exist: a material with no roughness must
        // reach the shader as "no roughness", not as a roughness of 0.0, which
        // would draw a mirror.
        let inputs = OpenPbrInputs {
            base_color: Colour::Unadmitted,
            base_metalness: 0.0,
            specular_roughness: None,
            specular_ior: None,
            specular_weight: None,
            coat_weight: 0.0,
            coat_roughness: None,
            transmission_weight: 0.0,
            transmission_depth: None,
            subsurface_weight: None,
            emission: Emission::None,
        };
        let p = MaterialParams::from_inputs(&inputs, MESH_DOES_NOT_CARRY_OPEN);
        assert_eq!(p.base_color_and_present[3], 0.0, "the colour is not admitted");
        assert_eq!(p.rough_metal[1], 0.0, "roughness must be flagged absent");
        assert_eq!(p.spec_emission[1], 0.0, "weight must be flagged absent");
        assert_eq!(p.spec_emission[3], 0.0, "emission must be flagged absent");
        assert_eq!(p.rough_metal[0], 0.0, "and carries no value");
        // The five parameters that had nowhere to go carry their absences too, and an
        // absence is a **cleared flag**, not a value that could be read as one: a
        // cleared `f0` slot would draw a 100 % mirror if it were read as a value.
        for (slot, name) in [
            (p.specular_boundary[1], "f0"),
            (p.participating_medium[0], "coat_roughness"),
            (p.participating_medium[2], "transmission_depth"),
            (p.medium_flags[0], "subsurface_weight"),
        ] {
            assert_eq!(slot, 0.0, "{name} must be flagged absent");
        }
    }

    #[test]
    fn a_present_input_packs_its_value_and_its_flag() {
        let inputs = OpenPbrInputs {
            base_color: Colour::Admitted(LinearRgb::new(0.25, 0.5, 0.75)),
            base_metalness: 0.0,
            specular_roughness: Some(0.4),
            specular_ior: Some(1.5),
            specular_weight: Some(0.125),
            coat_weight: 0.0,
            coat_roughness: None,
            transmission_weight: 0.5,
            transmission_depth: None,
            subsurface_weight: None,
            emission: Emission::Emitted { color: LinearRgb::WHITE, luminance: 2.0 },
        };
        let p = MaterialParams::from_inputs(&inputs, MESH_DOES_NOT_CARRY_OPEN);
        assert_eq!(p.base_color_and_present, [0.25, 0.5, 0.75, 1.0]);
        assert_eq!(p.rough_metal, [0.4, 1.0, 0.0, 0.5]);
        assert_eq!(p.spec_emission, [0.125, 1.0, 2.0, 1.0]);
        assert_eq!(p.emission_rgb_open[3], 0.0, "open is the named debt, still 0");
        // The index travels as the reflectance it *becomes*, and the value is the
        // relation's own: `((1.5 − 1)/(1.5 + 1))² = 0.04`.
        let f0 = voxel_appearance::provenance::specular_f0(1.5) as f32;
        assert_eq!(p.specular_boundary, [f0, 1.0, 0.0, 0.0]);
        assert!((f0 - 0.04).abs() < 1e-6, "n = 1.5 reflects about 4 %, got {f0}");
    }

    /// The world's own shading field — the same declaration the window builds its
    /// table from, so the gains these tests read are the gains the game draws (C-161).
    fn standing() -> ShadingField {
        voxel_shapers::materials::shading_field()
            .expect("the world's shading field declares for the world's own registry")
    }

    #[test]
    fn bytes_are_little_endian_in_field_order() {
        let p = MaterialParams {
            base_color_and_present: [1.0, 2.0, 3.0, 1.0],
            rough_metal: [0.0; 4],
            spec_emission: [0.0; 4],
            emission_rgb_open: [0.0; 4],
            pattern: [0.5, 0.25, 0.0, 0.0],
            specular_boundary: [0.04, 1.0, 0.0, 0.0],
            participating_medium: [0.0, 0.3, 1.0, 0.0],
            medium_flags: [0.0; 4],
        };
        let b = p.bytes();
        assert_eq!(b.len(), 128);
        assert_eq!(&b[0..4], &voxel_model::endian::le_bytes(1.0f32));
        assert_eq!(&b[8..12], &voxel_model::endian::le_bytes(3.0f32));
        assert_eq!(&b[12..16], &voxel_model::endian::le_bytes(1.0f32));
        // The fifth `vec4` is the pattern's gains, and it travels after the four
        // OpenPBR groups rather than inside them.
        assert_eq!(&b[64..68], &voxel_model::endian::le_bytes(0.5f32));
        assert_eq!(&b[68..72], &voxel_model::endian::le_bytes(0.25f32));
        // And the three groups the shader could not read before follow it, in the
        // struct's own order — because a stride that disagreed with `shading.wgsl`
        // would put a coat weight where a medium depth is read.
        assert_eq!(&b[80..84], &voxel_model::endian::le_bytes(0.04f32));
        assert_eq!(&b[84..88], &voxel_model::endian::le_bytes(1.0f32));
        // `participating_medium`'s second and third slots: the medium's depth and
        // whether it is admitted — 100 and 104 bytes in, because the two groups
        // before it are full.
        assert_eq!(&b[100..104], &voxel_model::endian::le_bytes(0.3f32));
        assert_eq!(&b[104..108], &voxel_model::endian::le_bytes(1.0f32));
    }

    #[test]
    fn every_admitted_material_gets_a_row_and_the_rest_are_absent() {
        let reg = registry();
        let field = standing();
        let t = table(&reg, &field).unwrap();
        let mut admitted = 0;
        let mut varied = 0;
        for m in reg.materials() {
            // The row must be this material's *derived* inputs, not a default.
            // Asserting equality with the packer is the honest form of "the row
            // was filled in": it cannot pass for a row that was never written.
            let inputs = inputs_for(&reg, m.id).unwrap();
            let (roughness_gain, reflectance_gain) = field.gain(m.id).unwrap_or((0.0, 0.0));
            let surface = voxel_shapers::materials::pattern_of(m.id).index();
            assert_eq!(
                t[m.id.0 as usize],
                MaterialParams::from_inputs(&inputs, MESH_DOES_NOT_CARRY_OPEN).with_pattern(
                    roughness_gain,
                    reflectance_gain,
                    surface,
                ),
                "material {} has a defaulted row",
                m.id.0
            );
            if roughness_gain > 0.0 || reflectance_gain > 0.0 {
                varied += 1;
            }
            admitted += 1;
        }
        // The field is **wired**, not merely carried: the standing declaration
        // varies every opaque material and none of the transmissive ones, and a
        // table of all-zero gains would pass every assertion above while drawing
        // exactly the flat blocks C-160 reported.
        assert!(varied > 0, "the shading field varies no material at all");
        assert_eq!(
            t[voxel_shapers::materials::REGOLITH.0 as usize].pattern,
            [0.5, 0.5, 0.0, 0.0],
            "regolith's row does not carry the standing gains"
        );
        // A transmissive material carries **no gains** — its look is its Fresnel
        // reflection (C-159.2) — and it still carries a *complete* shaper id,
        // because the mapping is total: the fluid half of the 256 level (foam,
        // caustics, animated addressed cells) will read it, and until then the
        // zero gains are what keep the row flat (C-162).
        let water = voxel_shapers::materials::WATER;
        let water_row = t[water.0 as usize].pattern;
        assert_eq!(water_row[..2], [0.0, 0.0], "a transmissive material was given gains");
        assert_eq!(
            water_row[2],
            voxel_shapers::materials::pattern_of(water).index() as f32,
            "the shaper id is part of every row, even a flat one"
        );
        // **The world's own thirteen, plus the corpus.** The registry declares fourteen
        // `MaterialId` constants — AIR, the fluid pair, the five rock classes, the five
        // surface covers and wood — and `AIR` is never admitted, because
        // `MaterialId::EMPTY` is "the absence of matter... never admitted". So the world's
        // own rows are thirteen, written down because the first version of this assertion
        // said eight and the surfaces (C-159.10) and the trees (C-159.11) moved it twice.
        // The corpus (C-122) then admits one material per declared substance on top of
        // them, so the count is asserted as the **sum** rather than as a new constant: an
        // edited constant hides a row that never arrived, and a sum cannot.
        assert_eq!(
            admitted,
            13 + voxel_shapers::corpus::ROWS.len(),
            "the admitted set is {admitted}"
        );
        assert_eq!(reg.admitted_material_count(), admitted);
        assert!(reg.material(MaterialId(0)).is_err(), "air must not be admitted");
        // and the row that was never filled in says so
        assert_eq!(t[0], MaterialParams::default());
    }

    #[test]
    fn not_every_admitted_material_derives_a_roughness_and_the_table_says_so() {
        // **Found by running it, not by reading it.** The first version of the
        // test above asserted that every admitted material derives a roughness.
        // It failed on material 1, which is WATER. That is the appearance
        // layer's rule working as stated (C-27): a parameter is *derived* when
        // the model's own structure determines it, and *absent* when the world
        // has the value and the model does not record it. So the table's job is
        // not to fill every row — it is to carry the absence.
        let reg = registry();
        let t = table(&reg, &standing()).unwrap();
        let mut without_roughness = Vec::new();
        for m in reg.materials() {
            if t[m.id.0 as usize].rough_metal[1] == 0.0 {
                without_roughness.push(m.name.clone());
            }
        }
        println!("admitted materials with no derived roughness: {without_roughness:?}");
        assert!(
            !without_roughness.is_empty(),
            "every material now derives a roughness — the bar's argument changed, and so did this test's premise"
        );
        // The absence is *shipped*, not hidden: the flag is what the shader
        // branches on, so a material with no roughness draws no rim and no
        // specular warp rather than drawing them at zero.
        for m in reg.materials() {
            let row = t[m.id.0 as usize];
            if row.rough_metal[1] == 0.0 {
                assert_eq!(row.rough_metal[0], 0.0, "{}", m.id.0);
            }
        }
    }

    #[test]
    fn a_rig_with_more_lights_than_the_buffer_holds_is_refused_not_truncated() {
        use voxel_appearance::npr::Light;
        let many = LightRig {
            lights: (0..RIG_CAPACITY + 1)
                .map(|_| Light { direction: Vec3::new(0.0, 1.0, 0.0), colour: LinearRgb::WHITE, intensity: 1.0 })
                .collect(),
            ambient: LinearRgb::BLACK,
        };
        assert!(GpuRig::from_rig(&many).is_none());
        let standard = LightRig::standard();
        let packed = GpuRig::from_rig(&standard).expect("the standard rig fits");
        assert_eq!(packed.count[0], standard.lights.len() as u32);
        assert_eq!(packed.lights[0].direction[3] as f64, standard.lights[0].intensity);
    }

    #[test]
    fn the_cpu_reference_is_the_crate_and_refuses_what_the_crate_refuses() {
        let inputs = OpenPbrInputs {
            base_color: Colour::Admitted(LinearRgb::grey(0.4)),
            base_metalness: 0.0,
            specular_roughness: Some(2.0), // out of OpenPBR's range
            specular_ior: Some(1.5),
            specular_weight: Some(0.05),
            coat_weight: 0.0,
            coat_roughness: Some(0.1),
            transmission_weight: 0.0,
            transmission_depth: Some(0.0),
            subsurface_weight: Some(0.0),
            emission: Emission::None,
        };
        let rig = LightRig::standard();
        assert!(shade_cpu(&inputs, Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.0, 1.0, 0.0), 0.0, &rig).is_err());
    }

    #[test]
    fn the_named_debt_is_zero_and_is_named() {
        // A test on a constant looks silly until the constant is a decision.
        // This one asserts the client is shading closed faces, so that whoever
        // makes the mesh carry openness has to change this line.
        assert_eq!(MESH_DOES_NOT_CARRY_OPEN, 0.0);
        let reg = registry();
        for m in reg.materials() {
            let inputs = inputs_for(&reg, m.id).unwrap();
            let p = MaterialParams::from_inputs(&inputs, MESH_DOES_NOT_CARRY_OPEN);
            assert_eq!(p.emission_rgb_open[3], 0.0);
        }
    }

    #[test]
    fn an_unadmitted_material_is_refused_by_the_table() {
        let reg = registry();
        assert!(reg.material(MaterialId(9999)).is_err());
        assert!(inputs_for(&reg, MaterialId(9999)).is_err());
    }

    #[test]
    fn the_shading_report_counts_materials_and_not_the_never_read_sentinel_row() {
        // Row 0 is `MaterialParams::default()` for `MaterialId::EMPTY`. Air is
        // never drawn, so it is not a material that failed to shade — and the
        // first version of this function counted it, which made the HUD report
        // `7 of 8` on a world where every single material shades.
        let table = table(&registry(), &standing()).expect("the table");
        let report = ShadingReport::of(&table);
        let admitted = 13 + voxel_shapers::corpus::ROWS.len();
        assert_eq!(report.materials, admitted, "the admitted materials");
        // **Every admitted material now carries a base colour, and that is this round's
        // claim.** Before the corpus, a row with no reflectance was drawn black by
        // arithmetic (C-38), and the two counts matched only because `derive` refused to
        // invent one for anybody. They are asserted *equal* rather than both pinned so that
        // a future row admitted without a reflectance fails here as well as in the corpus
        // census, where the absence is named.
        assert_eq!(
            report.with_base_colour, report.materials,
            "a material is admitted with no base colour, so it is counted as shaded while \
             staying unlit"
        );
        assert!(report.shades());

        // And the empty table is zero of zero rather than a false debt.
        let none = ShadingReport::of(&[]);
        assert_eq!((none.materials, none.with_base_colour), (0, 0));
        assert!(!none.shades());
    }
}
