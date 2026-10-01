//! The bar, as a **measurement**.
//!
//! The FMA bars' Deadlock bullet 2 says: *"no white-factor unshaded placeholders
//! past S2: every visible surface shows shading response (**≥3 distinct luminance
//! bands** across the model under the room light rig)"*. That is a criterion and
//! not an algorithm, so it is measurable — and this module is the measurement.
//!
//! # Two entry points, because the bar names two things
//!
//! * [`shading_response`] shades a surface inside the sandbox: it sweeps views
//!   over the upper hemisphere under a [`LightRig`] and measures what comes back.
//!   This is the version a build can run without a GPU.
//! * [`measure`] takes raw luminances. This is the version a **probe** uses on a
//!   renderer's actual output, which is what the bar's own procedure does (pixel
//!   analysis of a screenshot). Both paths end in the same band count, so a
//!   sandbox measurement and a screenshot measurement are comparable numbers
//!   rather than two opinions.
//!
//! # Why a band count is the right statistic
//!
//! Because it is a **discriminator, and it is tested as one**. A white-factor
//! placeholder answers the same value in every direction, so it measures **one**
//! band however bright it is; a surface with a diffuse wrap, a rim and a warp
//! measures several. `a_white_factor_placeholder_measures_one_band` asserts
//! exactly that, which is what makes this a test of the bar instead of a test
//! that the code runs.

use crate::npr::{shade, LightRig, Shaded, SurfaceFrame, Terms};
use crate::openpbr::OpenPbrInputs;
use crate::provenance::LUMINANCE_BAND_RESOLUTION;
use voxel_model::{RefusalKind, RefusalSet};
use voxel_noise::Vec3;

/// The bar's own figure: **≥3 distinct luminance bands**. Quoted, not chosen.
pub const BAR_BANDS: usize = 3;

/// What a shading measurement found.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ShadingResponse {
    /// Distinct luminance bands the surface showed.
    pub bands: usize,
    /// How many samples the measurement took.
    pub samples: usize,
    /// The band resolution the count is made at (see [`crate::provenance`]).
    pub band_resolution: usize,
    pub min_luminance: f64,
    pub max_luminance: f64,
    /// Which of the four Source 2 terms the surface could draw at all. A surface
    /// can pass the band count on its rim alone, and this is where that is
    /// visible rather than implied.
    pub terms: Terms,
}

impl ShadingResponse {
    /// The FMA bar's criterion: at least three distinct luminance bands.
    pub fn meets_bar(&self) -> bool {
        self.bands >= BAR_BANDS
    }
}

/// Count the distinct luminance bands in a set of measured luminances.
///
/// The band **width** is derived from the observed range and the declared
/// resolution — the bar names a count and not a width, so the width has to come
/// from the measurement itself rather than from a constant somebody liked.
pub fn measure(samples: &[f64], terms: Terms) -> Result<ShadingResponse, RefusalSet> {
    let mut set = RefusalSet::new();
    if samples.is_empty() {
        set.refuse(
            RefusalKind::OutOfDeclaredRange,
            "a response measured over zero samples is not a measurement",
        );
        return Err(set);
    }
    for (i, v) in samples.iter().enumerate() {
        if !v.is_finite() {
            set.refuse(
                RefusalKind::NonFiniteValue,
                format!("sample {i} has luminance {v}"),
            );
        }
    }
    if !set.is_empty() {
        return Err(set);
    }

    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for v in samples {
        min = min.min(*v);
        max = max.max(*v);
    }

    // A surface that answered the same value everywhere has **one** band, which is
    // the degenerate case the bar is written against — not zero bands.
    let bands = if max <= min {
        1
    } else {
        let width = (max - min) / LUMINANCE_BAND_RESOLUTION as f64;
        let mut populated = [false; LUMINANCE_BAND_RESOLUTION];
        for v in samples {
            let index = (((v - min) / width) as usize).min(LUMINANCE_BAND_RESOLUTION - 1);
            populated[index] = true;
        }
        populated.iter().filter(|seen| **seen).count()
    };

    Ok(ShadingResponse {
        bands,
        samples: samples.len(),
        band_resolution: LUMINANCE_BAND_RESOLUTION,
        min_luminance: min,
        max_luminance: max,
        terms,
    })
}

/// Measure a surface by shading it, in the sandbox, over a hemisphere of views.
///
/// `views` is required and must be positive: a measurement over no views is not a
/// measurement, and defaulting it would make an unmeasured surface look measured.
/// The view directions are the **Fibonacci lattice** (deterministic, no RNG
/// state, so two runs agree and so a chunk can be measured on any thread).
///
/// The surface measured is a face with **no holes in it** (`open = 0`), so a
/// material's band count is not confused with the porosity of the composition it
/// happens to sit in.
pub fn shading_response(
    inputs: &OpenPbrInputs,
    rig: &LightRig,
    normal: Vec3,
    views: usize,
) -> Result<ShadingResponse, RefusalSet> {
    if views == 0 {
        let mut set = RefusalSet::new();
        set.refuse(
            RefusalKind::OutOfDeclaredRange,
            "a shading response over zero views is not a measurement",
        );
        return Err(set);
    }
    if !normal.is_finite() {
        let mut set = RefusalSet::new();
        set.refuse(RefusalKind::NonFiniteValue, "the measured normal is not finite");
        return Err(set);
    }

    // The golden angle, computed rather than written down as digits.
    let golden = (3.0 - 5.0f64.sqrt()) * std::f64::consts::PI;
    let mut luminances = Vec::with_capacity(views);
    let mut terms = Terms::none();

    for i in 0..views {
        // An upper hemisphere: `y` runs over `(0, 1]`, so the normal is always
        // visible from a sampled view.
        let y = 1.0 - (i as f64 + 0.5) / views as f64;
        let radius = (1.0 - y * y).max(0.0).sqrt();
        let theta = golden * i as f64;
        let view = Vec3::new(radius * theta.cos(), y, radius * theta.sin());

        let frame = SurfaceFrame { normal, view, open: 0.0 };
        let Shaded { colour, terms: at_view } = shade(inputs, &frame, rig)?;
        // The union, not the last: the warp is a *step*, so it can fire at one
        // view and not at the next, and a term that fired once was drawn.
        terms = terms.union(at_view);
        luminances.push(colour.luminance());
    }

    measure(&luminances, terms)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openpbr::{Colour, Emission, LinearRgb, OpenPbrInputs};

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

    fn up() -> Vec3 {
        Vec3::new(0.0, 1.0, 0.0)
    }

    #[test]
    fn a_white_factor_placeholder_measures_one_band_and_fails_the_bar() {
        // A placeholder answers the same value in every direction — that is what
        // "white-factor, unshaded" means. It is the case the bar exists for, and
        // it must fail here or the bar is not being measured at all.
        let response = measure(&[1.0; 64], Terms::none()).unwrap();
        assert_eq!(response.bands, 1);
        assert!(!response.meets_bar());
        assert_eq!(response.max_luminance, response.min_luminance);
    }

    #[test]
    fn a_stated_surface_under_the_standard_rig_passes_the_bar() {
        let response = shading_response(&stated(), &LightRig::standard(), up(), 256).unwrap();
        // The bar's procedure requires **pasted output** rather than a claim, so
        // this line exists to be captured with `--nocapture`.
        println!("measured: {response:?}");
        assert_eq!(response.samples, 256);
        assert_eq!(response.terms.count(), 4, "terms were {:?}", response.terms);
        assert!(
            response.meets_bar(),
            "measured {} bands over {:.3}..{:.3}",
            response.bands,
            response.min_luminance,
            response.max_luminance
        );
        assert!(response.max_luminance > response.min_luminance);
    }

    #[test]
    fn the_band_count_never_exceeds_the_declared_resolution() {
        let response = shading_response(&stated(), &LightRig::standard(), up(), 512).unwrap();
        assert!(response.bands <= response.band_resolution);
        assert_eq!(response.band_resolution, LUMINANCE_BAND_RESOLUTION);
    }

    #[test]
    fn a_flat_rig_measures_one_band_because_nothing_varies() {
        // Ambient only, no lights: the surface is the same colour from every view,
        // so the honest answer is one band even though the surface is *shaded*.
        let rig = LightRig { lights: vec![], ambient: LinearRgb::grey(0.2) };
        let response = shading_response(&stated(), &rig, up(), 64).unwrap();
        assert_eq!(response.bands, 1);
        assert!(response.terms.ambient && response.terms.count() == 1);
    }

    #[test]
    fn a_rim_only_surface_is_reported_as_rim_only_however_many_bands_it_shows() {
        // The honest failure mode of the derived rock: the band count alone could
        // be satisfied by its rim, so the term count travels with the number.
        let rock = OpenPbrInputs { base_color: Colour::Unadmitted, ..stated() };
        let response = shading_response(&rock, &LightRig::standard(), up(), 256).unwrap();
        assert!(!response.terms.half_lambert);
        assert!(!response.terms.ambient);
        assert!(response.terms.rim);
        assert!(response.bands <= response.band_resolution);
    }

    #[test]
    fn zero_views_is_refused_rather_than_defaulted() {
        let err = shading_response(&stated(), &LightRig::standard(), up(), 0).unwrap_err();
        assert_eq!(err.all()[0].kind(), RefusalKind::OutOfDeclaredRange);
    }

    #[test]
    fn an_empty_or_non_finite_measurement_is_refused() {
        assert!(measure(&[], Terms::none()).is_err());
        let err = measure(&[0.5, f64::NAN], Terms::none()).unwrap_err();
        assert_eq!(err.all()[0].kind(), RefusalKind::NonFiniteValue);
    }

    #[test]
    fn the_measurement_of_a_surface_is_reproducible() {
        let a = shading_response(&stated(), &LightRig::standard(), up(), 128).unwrap();
        let b = shading_response(&stated(), &LightRig::standard(), up(), 128).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn two_bands_of_a_known_step_are_counted_as_two() {
        // The band counter itself, on an input whose answer is known by hand.
        let response = measure(&[0.0, 1.0], Terms::none()).unwrap();
        assert_eq!(response.bands, 2);
        assert_eq!(response.min_luminance, 0.0);
        assert_eq!(response.max_luminance, 1.0);
    }
}
