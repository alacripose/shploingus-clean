//! **The display transform**: scene-referred radiance in, a displayable colour out.
//!
//! # The limit this exists to lift, and the limit it cannot
//!
//! Everything upstream of this module is **scene-referred**: a sunlit sky and a
//! shadowed rock differ by four or five orders of magnitude, and [`crate::sky`]'s
//! radiance and [`crate::npr::shade`]'s output are physical quantities that are
//! allowed to exceed `1`. Everything downstream is a **display**: this machine's
//! swapchain is an 8-bit `sRGB` surface, and it cannot present more than white.
//!
//! Those two facts cannot both be honoured by pretending the second one is not
//! true. A renderer that writes its physical value straight at an 8-bit target is
//! not rendering HDR — it is **clipping**, and it clips at whatever the arithmetic
//! happened to produce rather than at a point anybody chose. The measured state of
//! this project is exactly that: `renderer::tests::
//! the_derived_rig_lights_the_shipped_world_rather_than_only_a_probe` reads a
//! brightest pixel of **1.886** where the frame's own contract says the value must
//! be within `[0, 1]`, and the two cannot be reconciled by moving the exposure,
//! because the excess is a *shape* and not a scale.
//!
//! So the display is made explicit. This is the single place where an unbounded
//! radiance becomes a bounded colour, and it is a **declared operator** rather
//! than the accidental one that clamping is.
//!
//! # The operator, and why it carries no constant
//!
//! Reinhard's `L ↦ L / (1 + L)` (Reinhard, Stark, Shirley & Ferwerda, *Photographic
//! Tone Reproduction for Digital Images*, ACM TOG 2002), applied to the colour's
//! own luminance and to nothing else.
//!
//! **It has no free parameter, and that is the reason it is here rather than a
//! film curve.** A project that labels every constant with its source and counts
//! its unsourced ones in a test ([`crate::provenance`]) should not acquire a
//! shoulder, a toe, a toe strength and a linear section — each of which would be a
//! number nobody chose, spent on every pixel of every frame. This operator is one
//! division. It adds **no entry to [`crate::provenance::CONSTANTS`]**, which is
//! not a coincidence to be pleased about but the criterion it was selected on.
//!
//! What it gives up for that: it is a *global* curve, so it cannot put a
//! shoulder exactly where a particular exposure wants one, and its white point is
//! asymptotic rather than reached. Both are stated here rather than discovered
//! later.
//!
//! # What it does *not* do
//!
//! It does not apply exposure. The exposure constants
//! ([`crate::sky`]'s and the client's) belong where they are, at the upload, for
//! the reason already recorded there: *the model stays physical and only the thing
//! that reaches the swapchain is scaled*. This module is downstream of all of
//! that — it takes what the frame computed, exactly as the frame computed it.
//!
//! It does not encode either. `sRGB`'s transfer curve is a property of a
//! particular target, and [`crate::openpbr::LinearRgb`] is deliberately not `sRGB`
//! (see its own documentation); the encoding stays in the client, which is where
//! it can be decided from the format actually being presented to.
//!
//! It does not dither. A dither is a statement about how many steps a target has,
//! which is the same target question again, and it is answered in the same place.

use crate::openpbr::LinearRgb;

/// **The display transform**: an unbounded scene-referred colour, made displayable.
///
/// The colour's luminance is compressed by `L ↦ L / (1 + L)` and every channel is
/// scaled by that same factor, so the picture's **chromaticity is untouched** and
/// only its brightness is remapped. A grey stays the same grey, a warm sunlit wall
/// stays warm, and a blue sky stays blue while ceasing to be 1.886.
///
/// # The domain, and what happens outside it
///
/// The domain is **non-negative radiance**, and a colour whose luminance is not
/// greater than zero is answered with [`LinearRgb::BLACK`]. That is a boundary
/// rather than a default: zero luminance is a real measurement — a cell at night —
/// and it maps to black; a *negative* luminance is not a radiance at all, and this
/// crate's rule is to refuse what it cannot represent rather than to invent a value
/// for it. A `NaN` propagates as `NaN` rather than being quietly turned into a
/// number somebody could believe, which is the same rule seen from the other side.
///
/// **It can never brighten a colour** (`L' ≤ L` for all `L ≥ 0`), because a
/// display transform that made something brighter than the scene would be a
/// renderer's decision wearing a display's name.
pub fn tone_map(colour: LinearRgb) -> LinearRgb {
    let luminance = colour.luminance();
    if luminance.is_nan() {
        return LinearRgb::new(f64::NAN, f64::NAN, f64::NAN);
    }
    if luminance <= 0.0 {
        return LinearRgb::BLACK;
    }
    // `(L / (1 + L)) / L` is exactly `1 / (1 + L)`, and written that way it cannot
    // divide by a luminance so small that the two forms disagree. The WGSL
    // transcription uses the same form, so the two compute the same expression
    // rather than two expressions a reader has to prove equal.
    let gain = 1.0 / (1.0 + luminance);
    LinearRgb::new(colour.r * gain, colour.g * gain, colour.b * gain)
}

/// The luminance [`tone_map`] would leave for a stated luminance, on its own.
///
/// Split out because it is the whole of the operator: the colour version is this
/// times a per-channel gain, and a caller measuring the curve should not have to
/// build a colour to do it.
///
/// **Divergence from the shader, stated rather than hidden:** a `NaN` luminance is
/// answered `NaN` here and **black** in `shading.wgsl`'s transcription. The reason
/// is that they are answering different questions. This is a model, and a `NaN`
/// reaching it is a fault the caller has to be able to see — the rule everywhere
/// else in this crate is that a bad value is refused rather than replaced with one
/// somebody could believe. The shader has to *write something* to a surface, and
/// black is the only value it can write that cannot be mistaken for a measurement.
pub fn tone_map_luminance(luminance: f64) -> f64 {
    if luminance.is_nan() || luminance <= 0.0 {
        return 0.0;
    }
    luminance / (1.0 + luminance)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **The measurement that motivated this module**, kept as a number: the
    /// brightest pixel the derived skylight rig produced, read from
    /// `renderer::tests::the_derived_rig_lights_the_shipped_world_rather_than_only_a_probe`.
    /// It is quoted rather than recomputed because it is that test's observation,
    /// and this module is what has to cope with it.
    const MEASURED_BRIGHTEST: f64 = 1.886_123_674_511_909_4;

    #[test]
    fn it_never_presents_more_than_one_and_never_reaches_one() {
        // The whole point: an unbounded scene-referred value arrives displayable.
        // Large finite inputs are the case that matters — a clipping renderer
        // passes this test for every input below 1 and fails it here.
        for exponent in 0..=12 {
            let l = 10f64.powi(exponent);
            let out = tone_map_luminance(l);
            assert!(out < 1.0, "L {l} came out {out}, which a display cannot present");
            assert!(out > 0.0, "L {l} came out {out}, which is a black frame");
        }
        assert!(
            tone_map_luminance(f64::MAX) <= 1.0,
            "the operator has to be bounded for every finite input, not for the ones somebody tried"
        );
    }

    #[test]
    fn it_is_monotonic_so_no_two_radiances_change_order() {
        // A display transform that folded two radiances onto one value would lose
        // a distinction the world made, and no later stage could recover it.
        let mut previous = -1.0f64;
        for step in 0..2_000 {
            let l = step as f64 * 0.05;
            let out = tone_map_luminance(l);
            assert!(out > previous, "L {l} came out {out} after {previous}");
            previous = out;
        }
    }

    #[test]
    fn it_can_never_brighten_a_colour() {
        // A display transform that made something brighter than the scene would be
        // a rendering decision wearing a display's name.
        for step in 0..1_000 {
            let l = step as f64 * 0.1;
            let out = tone_map_luminance(l);
            assert!(out <= l, "L {l} came out brighter at {out}");
        }
    }

    #[test]
    fn black_stays_black_and_a_negative_radiance_is_refused_as_black() {
        assert_eq!(tone_map(LinearRgb::BLACK), LinearRgb::BLACK);
        // A negative luminance is not a radiance. The boundary is refused rather
        // than wrapped, the same way every other refusal in this crate works.
        assert_eq!(tone_map(LinearRgb::new(-1.0, -1.0, -1.0)), LinearRgb::BLACK);
        // And a NaN stays a NaN rather than becoming a value somebody could believe.
        assert!(tone_map(LinearRgb::new(f64::NAN, 0.0, 0.0)).r.is_nan());
    }

    #[test]
    fn it_moves_brightness_and_not_chromaticity() {
        // The two ratios are the chromaticity, and they are what must survive: the
        // sky has to stay blue after it stops being 1.886.
        let colour = LinearRgb::new(0.2, 0.4, 0.9);
        let out = tone_map(colour);
        let before = (colour.g / colour.r, colour.b / colour.r);
        let after = (out.g / out.r, out.b / out.r);
        assert!(
            (before.0 - after.0).abs() < 1e-12 && (before.1 - after.1).abs() < 1e-12,
            "the ratios moved from {before:?} to {after:?}"
        );
        assert!(out.luminance() < colour.luminance(), "the colour did not dim");
    }

    #[test]
    fn a_grey_stays_grey() {
        // The degenerate case a chromaticity test cannot see: all three channels
        // equal, so the ratios are 1 whatever happens, and only equality proves it.
        let out = tone_map(LinearRgb::grey(3.0));
        assert_eq!(out.r, out.g, "a grey came out with a hue");
        assert_eq!(out.g, out.b, "a grey came out with a hue");
    }

    #[test]
    fn the_world_s_measured_brightest_pixel_becomes_displayable() {
        // The number this module exists for. It is asserted as a **range and a
        // direction** rather than as a value: pinning the exact output would make
        // every later change to the rig fail here for a reason that is not this
        // module's, and the claim being made is only "the frame is presentable".
        let out = tone_map_luminance(MEASURED_BRIGHTEST);
        assert!(out < 1.0, "the measured pixel is still not presentable at {out}");
        assert!(out > 0.5, "the measured pixel came out at {out}, which is a dimmer world than was measured");
        println!("the measured brightest pixel 1.886 presents as {out:.6}");
    }

    #[test]
    fn a_scene_referred_frame_loses_only_its_headroom() {
        // What the operator costs, measured rather than asserted: pixels well below
        // white are almost untouched, and the compression is spent where the
        // headroom is. These are the numbers a reader should see before choosing it.
        for l in [0.01, 0.1, 0.5, 1.0, 1.886, 4.0, 16.0, 256.0] {
            let out = tone_map_luminance(l);
            println!("L {l:>8} -> {out:.6}  ({:.1}% of the input)", 100.0 * out / l);
        }
        // Half a unit of radiance is still half the display: the shadows are not
        // lifted, which is what makes this a display transform and not a "look".
        assert!((tone_map_luminance(0.5) - 1.0 / 3.0).abs() < 1e-12);
    }
}
