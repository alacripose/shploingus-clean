//! **U1 — Presentation: the appearance contract, re-derived.**
//!
//! # The document this is derived from, and what it is not derived from
//!
//! Derived from `docs/APPEARANCE.md` §1 (the user's own contract: **OpenPBR as the base, Valve's
//! Source 2 shading model as the NPR stylisation over it**, with 4×4×4 subvoxels and a 256× addressed
//! detail lattice), `docs/DESIGN-SPEC.md` §2 F13, and the FMA bar's criterion that every visible
//! surface shows shading response: *≥3 distinct luminance bands*. The reflectance provenance rule is
//! `LEDGER.md` C-39 — a colour is carried **with its source**, never invented.
//!
//! **Ported, not copied** — Constitution P7 as amended to **v2.0.0** on 2026-10-13: the path
//! dependency **is** the port, and the bytes the ported call reads are pinned by path + sha256 in
//! `docs/baselines/port-appearance-seam/pinned-parent-bytes.txt`.
//!
//! Concretely: this module's reduced `Surface`/`Light`/`View` are composed into `voxel_appearance`'s
//! `OpenPbrInputs`/`SurfaceFrame`/`LightRig`, and the colour `shade` returns is **that crate's** own
//! `shade`. The four Source 2 terms named below are the contract's terms; they are implemented
//! **once**, in the ported crate, and no longer re-derived here. Nothing is transcribed: the crate is
//! called, and the call is what the gate in `crate::evidence` measures.
//!
//! The clean room's own vocabulary survives the port: this module still refuses a missing reflectance
//! and a non-unit direction **by name**, and it carries the ported contract's own refusal as
//! [`RefusalKind::AppearanceRefused`] rather than inventing a colour.
//!
//! # The four Source 2 terms, in the order the contract names them
//!
//! 1. **half-Lambert wrap diffuse** — the `N·L · 0.5 + 0.5` remap, so a surface never goes fully
//!    black on its dark side;
//! 2. **rim light** — a fresnel-shaped, view-facing term;
//! 3. **specular warp** — a hard threshold for the stylised highlight;
//! 4. **ambient tint** — a per-material constant.
//!
//! They are applied **over** the OpenPBR inputs, never instead of them, so the stylised look is a
//! stylisation of light transport and not a substitute for it.
//!
//! # Refuse, never default (P2)
//!
//! A `Surface` with no admitted reflectance is **refused by name** rather than shaded grey, and a
//! direction that is not a unit vector is refused rather than silently normalised. An unadmitted
//! value is named absence, not `0.0` — a zero is a value a caller can believe.

use crate::material::{Density, Material, MaterialId, Reflectance};
use voxel_appearance::openpbr::{Colour, Emission, LinearRgb, OpenPbrInputs};
use voxel_appearance::{Light as ParentLight, LightRig, SurfaceFrame};
use voxel_noise::Vec3;

/// The three linear channels of a colour. Linear, not sRGB: the contract's terms compose in linear
/// light and the presentation step is where encoding happens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgb {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

impl Rgb {
    /// Relative luminance (Rec. 709), the quantity the FMA bar counts bands of.
    pub fn luminance(&self) -> f64 {
        0.2126 * self.r + 0.7152 * self.g + 0.0722 * self.b
    }
}

/// The reduced OpenPBR subset this slice implements. Parameters the model does not record are
/// **owed and named absent** (`tasks.md` T008), never defaulted to a plausible number.
///
/// A reflectance here is [`crate::material::Reflectance`] — a material record's own, carried with the source
/// it came from (`LEDGER.md` C-39) and admitted **once**, at the one place a reflectance is admitted. This
/// module used to declare a second, identical struct with its own range and provenance checks; two types for
/// one fact is how a material comes to be judged twice, so it is one type and one admission now.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Surface {
    /// `base_color`. `None` means the material record does not carry a reflectance — a refusal, not
    /// a black surface.
    pub reflectance: Option<Reflectance>,
    /// `specular_roughness`.
    pub roughness: f64,
    /// `base_metalness`.
    pub metalness: f64,
}

/// **The one conversion from an admitted material to the appearance this contract shades** — the material's
/// own **sourced** reflectance and the caller's declared look, or the one reason it derives no colour.
///
/// It is one function because every path that draws a material asks this same question: the plan reading's
/// columns and the drawn reading's rectangles both come here, so neither can start shading a material the
/// other refuses, and the "a material with no reflectance derives no colour" rule is stated once. *Which*
/// material stands at a place is **not** asked here — that is the world's own cell-addressed door
/// ([`crate::chunk::CubicWorld::material_at`]) and this function is handed the record it returned.
///
/// The caller names the **place** the refusal happened (a column, a rectangle); this names the reason.
pub fn surface_of(material: &Material, roughness: f64, metalness: f64) -> Result<Surface, RefusalKind> {
    let Some(reflectance) = material.reflectance else {
        return Err(RefusalKind::NoReflectance);
    };
    Ok(Surface { reflectance: Some(reflectance), roughness, metalness })
}

/// A directional light: the unit direction **toward** the light, and its tint.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Light {
    pub toward: [f64; 3],
    pub tint: [f64; 3],
}

/// The view: the unit direction **toward** the viewer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub toward: [f64; 3],
}

/// The refusal vocabulary. Named, closed, and never a boolean (P2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefusalKind {
    /// The surface carries no admitted reflectance.
    NoReflectance,
    /// A direction that is not a unit vector.
    NotAUnitVector,
    /// **The ported appearance contract refused it.** `voxel_appearance::shade` collects the reasons
    /// it will not shade — a non-finite or out-of-range OpenPBR input among them — and this port does
    /// not paper over one with a plausible colour. The clean room's own vocabulary cannot name every
    /// reason the contract has, so the contract's refusal is carried **as itself**: named, not
    /// collapsed into one of the four above.
    AppearanceRefused,
}

impl RefusalKind {
    pub const fn name(self) -> &'static str {
        match self {
            RefusalKind::NoReflectance => "no_reflectance",
            RefusalKind::NotAUnitVector => "not_a_unit_vector",
            RefusalKind::AppearanceRefused => "appearance_refused",
        }
    }
}

/// The resolution the FMA bar is counted at. **A single named constant**, and the count it produces
/// is a **lower bound**: at finer luminance differences than `1 / BAND_RESOLUTION` the count
/// saturates rather than resolving further (FR-005).
pub const BAND_RESOLUTION: usize = 64;

/// The FMA bar: at least three distinct luminance bands on a visible surface.
pub const BAR_MINIMUM_BANDS: usize = 3;

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn length(v: [f64; 3]) -> f64 {
    dot(v, v).sqrt()
}

/// A direction is a unit vector within a declared tolerance, or it is refused.
pub fn is_unit(v: [f64; 3]) -> bool {
    (length(v) - 1.0).abs() <= 1e-9
}

/// **The one shading entry point** — the seam the client and the measurement share (P5), and **the
/// port** (P7 v2.0.0): the colour returned is computed by `voxel_appearance`'s own `shade`.
///
/// Pure: the result depends only on its arguments (FR-002), which is what makes the client's picture
/// and the judge's number the same picture.
pub fn shade(
    surface: &Surface,
    normal: [f64; 3],
    light: &Light,
    view: &View,
) -> Result<Rgb, RefusalKind> {
    if !is_unit(normal) || !is_unit(light.toward) || !is_unit(view.toward) {
        return Err(RefusalKind::NotAUnitVector);
    }
    let Some(reflectance) = surface.reflectance else {
        return Err(RefusalKind::NoReflectance);
    };

    // **The port.** The clean room's reduced surface becomes the contract's own OpenPBR inputs, and
    // this frame's normal and view become its `SurfaceFrame`. Every parameter this clean room does not
    // record is passed as **absent** (`None`) — named absence, never a plausible number (T008).
    let inputs = OpenPbrInputs {
        base_color: Colour::Admitted(LinearRgb::new(
            reflectance.channels[0],
            reflectance.channels[1],
            reflectance.channels[2],
        )),
        base_metalness: surface.metalness,
        specular_roughness: Some(surface.roughness),
        specular_ior: None,
        specular_weight: None,
        coat_weight: 0.0,
        coat_roughness: None,
        transmission_weight: 0.0,
        transmission_depth: None,
        subsurface_weight: None,
        emission: Emission::None,
    };
    // The scene this clean room draws is closed: no face carries the composition's hole yet, so the
    // open fraction is 0.0 and it is **stated** here rather than left to a default.
    let frame = SurfaceFrame {
        normal: Vec3::new(normal[0], normal[1], normal[2]),
        view: Vec3::new(view.toward[0], view.toward[1], view.toward[2]),
        open: 0.0,
    };
    // The key light is this clean room's own argument; the hemisphere fill is the **parent's** declared
    // rig ambient, so the room both crates shade in is declared in one place. (The parent's own two-light
    // fixture is deliberately *not* used: a rig of its lights would ignore the light the client passes.)
    let rig = LightRig {
        lights: vec![ParentLight {
            direction: Vec3::new(light.toward[0], light.toward[1], light.toward[2]),
            colour: LinearRgb::new(light.tint[0], light.tint[1], light.tint[2]),
            intensity: 1.0,
        }],
        ambient: LightRig::standard().ambient,
    };
    match voxel_appearance::shade(&inputs, &frame, &rig) {
        Ok(shaded) => Ok(Rgb {
            r: shaded.colour.r,
            g: shaded.colour.g,
            b: shaded.colour.b,
        }),
        Err(_) => Err(RefusalKind::AppearanceRefused),
    }
}

/// **The FMA bar as a number** — how many distinct luminance bands the samples show, quantised at
/// [`BAND_RESOLUTION`]. A lower bound, per FR-005.
pub fn band_count(samples: &[Rgb]) -> usize {
    let mut seen = [false; BAND_RESOLUTION];
    for sample in samples {
        let l = sample.luminance().clamp(0.0, 0.999_999);
        let index = (l * BAND_RESOLUTION as f64) as usize;
        seen[index.min(BAND_RESOLUTION - 1)] = true;
    }
    seen.iter().filter(|b| **b).count()
}

/// **The transfer function** — a linear channel to the byte the screen is handed, over the declared
/// exposure. The contract's terms compose in **linear** light and this is the step where encoding happens
/// (the module's own sentence), so it belongs beside them rather than in a renderer: one owner, and both of
/// the client's readings ([`crate::surface`]'s plan reading and [`crate::drawn`]'s drawn reading) put their
/// colours through this one arithmetic. A channel brighter than the byte can hold is **clamped**, never
/// wrapped.
///
/// The curve is sRGB's own, over the declared linear gain: `12.92·x` below the standard's own breakpoint and
/// `1.055·x^(1/2.4) − 0.055` above it.
pub fn encode(colour: Rgb, exposure: f64) -> [u8; 4] {
    let channel = |linear: f64| -> u8 {
        let lit = (linear * exposure).clamp(0.0, 1.0);
        let encoded = if lit <= 0.003_130_8 {
            12.92 * lit
        } else {
            1.055 * lit.powf(1.0 / 2.4) - 0.055
        };
        (encoded * 255.0 + 0.5) as u8
    };
    // BGRA: the order the platform's own bitmaps are handed their bytes in.
    [channel(colour.b), channel(colour.g), channel(colour.r), 255]
}

/// **The contract's own fixture material** — an admitted material like any other rather than a surface
/// assembled by hand, so the sweep below is a sweep over a record a world could hold.
fn fixture_material() -> Material {
    Material {
        id: MaterialId(1),
        name: "regolith",
        density: Density::per_cm3(3, 2).expect("a positive density"),
        reflectance: Some(
            Reflectance::sourced([0.42, 0.36, 0.30], "USGS mineral reflectance, C-39")
                .expect("a sourced reflectance"),
        ),
    }
}

/// **The contract's own fixture surface** — [`surface_of`]'s answer for the fixture material, so the bar is
/// measured through the one conversion a drawn material goes through.
fn fixture_surface() -> Surface {
    surface_of(&fixture_material(), 0.55, 0.0).expect("the fixture material derives a colour")
}

/// The declared fixture: a sphere of orientations shaded under one light and one view. Deterministic
/// and part of the contract, so the bar is measured on the same surface every run (FR-005, P3).
pub fn sweep_fixture() -> Result<Vec<Rgb>, RefusalKind> {
    let surface = fixture_surface();
    let light = Light { toward: [0.0, 0.0, 1.0], tint: [1.0, 0.97, 0.92] };
    let view = View { toward: [0.0, 0.15, 0.99] };
    // The view direction must be a unit vector.
    let v_len = length(view.toward);
    let view = View { toward: [view.toward[0] / v_len, view.toward[1] / v_len, view.toward[2] / v_len] };

    let mut samples = Vec::with_capacity(180);
    for i in 0..=180 {
        let t = i as f64 / 180.0;
        // A great-circle sweep in the y–z plane, so N·L runs its whole range.
        let angle = std::f64::consts::PI * t;
        let normal = [0.0, angle.sin(), angle.cos()];
        samples.push(shade(&surface, normal, &light, &view)?);
    }
    Ok(samples)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_one_conversion_refuses_a_material_that_derives_no_colour() {
        // The one place "a material with no reflectance derives no colour" is decided, exercised on a record
        // that carries none: the caller names the place and this names the reason.
        let unrecorded = Material { reflectance: None, ..fixture_material() };
        assert_eq!(
            surface_of(&unrecorded, 0.55, 0.0).unwrap_err(),
            RefusalKind::NoReflectance
        );
        // And a record that carries one comes back carrying **it**, source and all — not a copy taken apart
        // and put back together.
        let surface = fixture_surface();
        assert_eq!(surface.reflectance, fixture_material().reflectance);
        assert_eq!(surface.roughness, 0.55);
        assert_eq!(surface.metalness, 0.0);
    }

    #[test]
    fn the_fma_bar_measures_at_least_three_bands() {
        let samples = sweep_fixture().expect("the fixture world is admitted");
        let bands = band_count(&samples);
        println!(
            "U1 FMA bar: {bands} distinct luminance bands (>= {BAR_MINIMUM_BANDS} required, \
             resolution {BAND_RESOLUTION})"
        );
        assert!(
            bands >= BAR_MINIMUM_BANDS,
            "the FMA bar requires >={BAR_MINIMUM_BANDS} bands, measured {bands}"
        );
    }

    #[test]
    fn a_surface_without_reflectance_is_refused_by_name() {
        let surface = Surface { reflectance: None, roughness: 0.5, metalness: 0.0 };
        let light = Light { toward: [0.0, 0.0, 1.0], tint: [1.0, 1.0, 1.0] };
        let view = View { toward: [0.0, 0.0, 1.0] };
        let err = shade(&surface, [0.0, 0.0, 1.0], &light, &view).unwrap_err();
        assert_eq!(err, RefusalKind::NoReflectance);
        assert_eq!(err.name(), "no_reflectance");
    }

    #[test]
    fn a_non_unit_normal_is_refused_by_name() {
        let light = Light { toward: [0.0, 0.0, 1.0], tint: [1.0, 1.0, 1.0] };
        let view = View { toward: [0.0, 0.0, 1.0] };
        let err = shade(&fixture_surface(), [0.0, 0.0, 2.0], &light, &view).unwrap_err();
        assert_eq!(err, RefusalKind::NotAUnitVector);
    }

    // A reflectance channel out of range, and a sourced value with no source, are refused where a
    // reflectance is admitted — `crate::material`'s own `Reflectance::sourced`, tested there. This module no
    // longer states that rule a second time.

    #[test]
    fn shading_is_a_pure_function_of_its_inputs() {
        let a = sweep_fixture().unwrap();
        let b = sweep_fixture().unwrap();
        assert_eq!(a, b, "the same inputs must shade identically");

        // Run-order independence: shading each sample alone matches the sweep.
        let surface = fixture_surface();
        let light = Light { toward: [0.0, 0.0, 1.0], tint: [1.0, 0.97, 0.92] };
        let v: [f64; 3] = [0.0, 0.15, 0.99];
        let v_len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        let view = View { toward: [v[0] / v_len, v[1] / v_len, v[2] / v_len] };
        for (i, sample) in a.iter().enumerate() {
            let angle = std::f64::consts::PI * (i as f64 / 180.0);
            let normal = [0.0, angle.sin(), angle.cos()];
            let alone = shade(&surface, normal, &light, &view).unwrap();
            assert_eq!(*sample, alone, "sample {i} differs when shaded alone");
        }
    }

    #[test]
    fn the_band_count_is_never_more_than_the_declared_resolution() {
        let samples: Vec<Rgb> = (0..BAND_RESOLUTION * 4)
            .map(|i| {
                let l = i as f64 / (BAND_RESOLUTION * 4) as f64;
                Rgb { r: l, g: l, b: l }
            })
            .collect();
        assert!(band_count(&samples) <= BAND_RESOLUTION);
    }

    #[test]
    fn a_flat_colour_encodes_to_the_byte_the_screen_is_handed() {
        // The transfer function's own two ends and its own order: BGRA, so the colour's blue is the low
        // byte of the pixel and the platform's own blue is the high one.
        assert_eq!(encode(Rgb { r: 0.0, g: 0.0, b: 0.0 }, 1.0), [0, 0, 0, 255]);
        assert_eq!(encode(Rgb { r: 1.0, g: 1.0, b: 1.0 }, 1.0), [255, 255, 255, 255]);
        assert_eq!(
            encode(Rgb { r: 1.0, g: 0.0, b: 0.0 }, 1.0),
            [0, 0, 255, 255],
            "red encodes to the third byte, because the buffer is BGRA"
        );
        // A channel brighter than the byte can hold is clamped, never wrapped.
        assert_eq!(encode(Rgb { r: 9.0, g: 9.0, b: 9.0 }, 1.0), [255, 255, 255, 255]);
    }

    #[test]
    fn the_port_refuses_a_surface_the_parent_contract_will_not_shade() {
        // The port's own refusal path, reached **through** the ported call rather than beside it: a
        // roughness outside OpenPBR's declared range is refused by the parent's `validate`, and this
        // module carries that refusal as its own named kind instead of clamping the number.
        let surface = Surface { roughness: 9.0, ..fixture_surface() };
        let light = Light { toward: [0.0, 0.0, 1.0], tint: [1.0, 1.0, 1.0] };
        let view = View { toward: [0.0, 0.0, 1.0] };
        let err = shade(&surface, [0.0, 0.0, 1.0], &light, &view).unwrap_err();
        assert_eq!(err, RefusalKind::AppearanceRefused);
        assert_eq!(err.name(), "appearance_refused");
    }
}
