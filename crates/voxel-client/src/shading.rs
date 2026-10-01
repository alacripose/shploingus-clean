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
use voxel_model::{MaterialRegistry, Refusal};
use voxel_noise::Vec3;

/// See the module comment. Zero because a merged quad has no hole it can name —
/// **not** because a hole would not matter.
pub const MESH_DOES_NOT_CARRY_OPEN: f64 = 0.0;

/// How many lights the rig uniform can hold. A rig with more is refused rather
/// than truncated, because a dropped light silently changes every pixel.
pub const RIG_CAPACITY: usize = 4;

/// One material's twelve OpenPBR inputs, packed as four `vec4`s.
///
/// The grouping is not cosmetic. A WGSL `array<T>` element stride must be a
/// multiple of 16 bytes, so four `vec4<f32>` (64 bytes, alignment 16) is the
/// layout that needs no padding constant invented to explain it. The field
/// names match `Material` in `shading.wgsl`.
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
        }
    }
}

impl MaterialParams {
    pub const SIZE: u64 = 64;

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
        MaterialParams {
            base_color_and_present: [base[0], base[1], base[2], base_present],
            rough_metal: [roughness, rough_present, inputs.base_metalness as f32, inputs.transmission_weight as f32],
            spec_emission: [weight, weight_present, luminance, emission_present],
            emission_rgb_open: [emission[0], emission[1], emission[2], open as f32],
        }
    }

    pub fn bytes(&self) -> [u8; 64] {
        let mut out = [0u8; 64];
        let mut i = 0usize;
        for group in [
            self.base_color_and_present,
            self.rough_metal,
            self.spec_emission,
            self.emission_rgb_open,
        ] {
            for v in group {
                out[i..i + 4].copy_from_slice(&v.to_le_bytes());
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
            out[i..i + 4].copy_from_slice(&v.to_le_bytes());
            i += 4;
        }
        for v in self.count {
            out[i..i + 4].copy_from_slice(&v.to_le_bytes());
            i += 4;
        }
        for light in self.lights {
            for v in light.direction.iter().chain(light.colour.iter()) {
                out[i..i + 4].copy_from_slice(&v.to_le_bytes());
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
pub fn table(registry: &MaterialRegistry) -> Result<Vec<MaterialParams>, Refusal> {
    let mut highest = 0u16;
    for m in registry.materials() {
        highest = highest.max(m.id.0);
    }
    let mut out = vec![MaterialParams::default(); highest as usize + 1];
    for m in registry.materials() {
        let inputs = openpbr::derive(m, registry, None)?;
        out[m.id.0 as usize] = MaterialParams::from_inputs(&inputs, MESH_DOES_NOT_CARRY_OPEN);
    }
    Ok(out)
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

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_model::MaterialId;

    fn registry() -> MaterialRegistry {
        voxel_shapers::column::materials::registry().expect("the admitted registry builds")
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
    }

    #[test]
    fn bytes_are_little_endian_in_field_order() {
        let p = MaterialParams {
            base_color_and_present: [1.0, 2.0, 3.0, 1.0],
            rough_metal: [0.0; 4],
            spec_emission: [0.0; 4],
            emission_rgb_open: [0.0; 4],
        };
        let b = p.bytes();
        assert_eq!(b.len(), 64);
        assert_eq!(&b[0..4], &1.0f32.to_le_bytes());
        assert_eq!(&b[8..12], &3.0f32.to_le_bytes());
        assert_eq!(&b[12..16], &1.0f32.to_le_bytes());
    }

    #[test]
    fn every_admitted_material_gets_a_row_and_the_rest_are_absent() {
        let reg = registry();
        let t = table(&reg).unwrap();
        let mut admitted = 0;
        for m in reg.materials() {
            // The row must be this material's *derived* inputs, not a default.
            // Asserting equality with the packer is the honest form of "the row
            // was filled in": it cannot pass for a row that was never written.
            let inputs = inputs_for(&reg, m.id).unwrap();
            assert_eq!(
                t[m.id.0 as usize],
                MaterialParams::from_inputs(&inputs, MESH_DOES_NOT_CARRY_OPEN),
                "material {} has a defaulted row",
                m.id.0
            );
            admitted += 1;
        }
        // **Seven, not eight.** The registry declares eight `MaterialId`
        // constants — AIR through BEDROCK — and `AIR` is never admitted, because
        // `MaterialId::EMPTY` is "the absence of matter... never admitted". So
        // the table is eight rows wide (index 0 exists and is never read) and
        // seven of them are derived. Written down because the first version of
        // this assertion said eight.
        assert_eq!(admitted, 7, "the admitted set is {admitted}");
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
        let t = table(&reg).unwrap();
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
        let table = table(&registry()).expect("the table");
        let report = ShadingReport::of(&table);
        assert_eq!(report.materials, 7, "the seven admitted materials");
        assert_eq!(report.with_base_colour, 7, "every one carries a reflectance now");
        assert!(report.shades());

        // And the empty table is zero of zero rather than a false debt.
        let none = ShadingReport::of(&[]);
        assert_eq!((none.materials, none.with_base_colour), (0, 0));
        assert!(!none.shades());
    }
}
