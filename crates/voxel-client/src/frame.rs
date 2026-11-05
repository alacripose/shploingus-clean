//! A rendered frame, and what can be measured off it.
//!
//! # Why this is the point of the crate
//!
//! `docs/APPEARANCE.md` claims no stage above **S0**, and it is right to: the
//! FMA bar's own procedure says that *pixel analysis of a rendered frame* is
//! what advances a stage, and no frame has ever existed. `response::measure`
//! run against a Fibonacci lattice of view directions is an argument about the
//! model; it is not a picture.
//!
//! This module closes that gap with **no new measurement code**: a frame's
//! luminances are handed to `voxel_appearance::response::measure`, which is the
//! same function that measured 8 of 8 bands on stated inputs. So the band count
//! now has two provenances — a lattice and a real rendered image — and they are
//! computed by the same code, which is the only way the two numbers are
//! comparable.

use voxel_appearance::response::{self, ShadingResponse};
use voxel_appearance::{LinearRgb, Terms};
use voxel_model::RefusalSet;

/// One rendered image, read back from the GPU, row-major from the **top left**
/// (which is how a texture copy arrives, not how clip space is oriented).
#[derive(Clone, PartialEq, Debug)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<[f32; 4]>,
}

/// Rec. 709 relative luminance, through the same function the band counter uses
/// (`LinearRgb::luminance`), so a band measured off a frame and a band measured
/// off a lattice cannot disagree about what luminance means.
pub fn luminance(pixel: [f32; 4]) -> f64 {
    LinearRgb::new(pixel[0] as f64, pixel[1] as f64, pixel[2] as f64).luminance()
}

impl Frame {
    pub fn pixel(&self, x: u32, y: u32) -> Option<[f32; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        self.pixels.get((y * self.width + x) as usize).copied()
    }

    pub fn luminance_at(&self, x: u32, y: u32) -> Option<f64> {
        self.pixel(x, y).map(luminance)
    }

    pub fn luminances(&self) -> Vec<f64> {
        self.pixels.iter().map(|p| luminance(*p)).collect()
    }

    /// True when this pixel carries **geometry**.
    ///
    /// The sky pass writes **alpha 0** and the world writes alpha 1, so alpha is the
    /// one bit of the frame that says which pixels the renderer actually shaded. This
    /// is the whole reason the sky is not allowed to claim alpha: a backdrop that
    /// looked like terrain would be counted as terrain by every measurement below.
    pub fn is_geometry(pixel: [f32; 4]) -> bool {
        pixel[3] > 0.5
    }

    /// The luminances of the pixels that were **drawn**, ignoring the sky.
    ///
    /// # Why the shading bars must not see the backdrop
    ///
    /// [`Frame::response`] used every pixel. That was correct while every pixel *was*
    /// geometry, and it stops being correct the moment a sky exists: a bright gradient
    /// across the top half of the frame would raise the band count, the mean luminance
    /// and the distinct-colour count for reasons that have nothing to do with how well
    /// a surface is shaded.
    ///
    /// The failure this prevents is the one `the_shipped_view_is_not_a_black_screen`
    /// already warns about in its own comment — *"a single flat clear colour would pass
    /// the test above and be no more a picture than the black screen was"*. A sky is not
    /// flat, so it would have passed **more** convincingly than a flat clear ever could.
    ///
    /// A frame of pure sky measures nothing and says so: `response` then refuses,
    /// because a shading response taken over zero samples is not a measurement.
    pub fn geometry_luminances(&self) -> Vec<f64> {
        self.pixels
            .iter()
            .filter(|p| Self::is_geometry(**p))
            .map(|p| luminance(*p))
            .collect()
    }

    /// How many pixels are sky and how many are geometry. **They sum to the frame.**
    ///
    /// The invariant worth asserting is that a rendered frame leaves nothing undrawn,
    /// which is what the sky pass exists to guarantee.
    pub fn split(&self) -> (usize, usize) {
        let geometry = self
            .pixels
            .iter()
            .filter(|p| Self::is_geometry(**p))
            .count();
        (self.pixels.len() - geometry, geometry)
    }

    /// The FMA bar's *≥3 distinct luminance bands* criterion, measured on the
    /// pixels that were actually drawn.
    ///
    /// `terms` is passed through rather than inferred: `Terms` is a statement
    /// about which Source 2 terms **could** be drawn, and a frame cannot tell
    /// you that — a black pixel is a black pixel whether the diffuse lobe was
    /// absent or merely unlit. Reporting it as an input keeps that from being
    /// guessed.
    pub fn response(&self, terms: Terms) -> Result<ShadingResponse, RefusalSet> {
        response::measure(&self.geometry_luminances(), terms)
    }

    /// How many distinct pixels are in the frame, for sanity rather than
    /// judgement: a frame of one colour is a picture of nothing.
    pub fn distinct_colours(&self) -> usize {
        let mut seen: Vec<[u32; 4]> = self
            .pixels
            .iter()
            .map(|p| {
                [
                    p[0].to_bits(),
                    p[1].to_bits(),
                    p[2].to_bits(),
                    p[3].to_bits(),
                ]
            })
            .collect();
        seen.sort_unstable();
        seen.dedup();
        seen.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pixels.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_appearance::npr::Terms;

    fn frame_of(values: &[f32]) -> Frame {
        Frame {
            width: values.len() as u32,
            height: 1,
            pixels: values.iter().map(|v| [*v, *v, *v, 1.0]).collect(),
        }
    }

    #[test]
    fn luminance_goes_through_the_crates_own_function() {
        // A grey pixel's luminance is its value, and a pure-green one is the
        // Rec. 709 green weight — which is the point: the frame does not get its
        // own opinion about what luminance is.
        assert!((luminance([0.5, 0.5, 0.5, 1.0]) - 0.5).abs() < 1e-12);
        let g = luminance([0.0, 1.0, 0.0, 1.0]);
        assert!((g - 0.7152).abs() < 1e-12, "green luminance was {g}");
    }

    #[test]
    fn indexing_is_row_major_from_the_top_left_and_out_of_range_is_none() {
        let f = Frame {
            width: 2,
            height: 2,
            pixels: vec![
                [1.0, 0.0, 0.0, 1.0],
                [0.0, 1.0, 0.0, 1.0],
                [0.0, 0.0, 1.0, 1.0],
                [0.5, 0.5, 0.5, 1.0],
            ],
        };
        assert_eq!(f.pixel(0, 0).unwrap()[0], 1.0);
        assert_eq!(f.pixel(1, 0).unwrap()[1], 1.0);
        assert_eq!(f.pixel(0, 1).unwrap()[2], 1.0);
        assert!(f.pixel(2, 0).is_none());
        assert!(f.pixel(0, 2).is_none());
    }

    #[test]
    fn a_two_tone_frame_passes_the_bars_band_count() {
        // The bar is ≥3 bands, so a two-tone frame must FAIL it. This is the
        // discrimination check: a measurement that passes everything measures
        // nothing.
        let flat = frame_of(&[0.2, 0.2, 0.2, 0.2]);
        let r = flat.response(Terms::none()).unwrap();
        assert_eq!(r.bands, 1);
        assert!(!r.meets_bar());

        let two = frame_of(&[0.2, 0.2, 0.8, 0.8]);
        assert_eq!(two.response(Terms::none()).unwrap().bands, 2);

        let three = frame_of(&[0.1, 0.1, 0.5, 0.5, 0.9, 0.9]);
        let r = three.response(Terms::none()).unwrap();
        assert!(r.bands >= 3, "bands were {}", r.bands);
        assert!(r.meets_bar());
    }

    #[test]
    fn a_frame_of_one_colour_is_not_a_picture_and_says_so() {
        let f = frame_of(&[0.3; 64]);
        assert_eq!(f.distinct_colours(), 1);
        let f = frame_of(&[0.3, 0.4, 0.5]);
        assert_eq!(f.distinct_colours(), 3);
    }

    #[test]
    fn a_non_finite_pixel_is_refused_rather_than_counted() {
        let f = frame_of(&[0.2, f32::NAN, 0.4]);
        assert!(f.response(Terms::none()).is_err());
    }

    #[test]
    fn terms_travel_with_the_measurement_rather_than_being_guessed() {
        let f = frame_of(&[0.1, 0.5, 0.9]);
        let none = f.response(Terms::none()).unwrap();
        assert_eq!(none.terms, Terms::none());
        let all = f
            .response(Terms {
                half_lambert: true,
                rim: true,
                specular_warp: true,
                ambient: true,
                subsurface: false,
            })
            .unwrap();
        assert_eq!(all.terms.count(), 4);
        // and the pixels did not change because of it
        assert_eq!(none.bands, all.bands);
    }
}
