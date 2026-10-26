//! **The spiritual signature** — the reading every being carries, whether or not the screen can show
//! it.
//!
//! `F:\planet-shploingus\docs\AFFECT.md`: every being, plant or agent, has an integer **wheel
//! position** — an `angle` on a 432-step turn and a `radius` — that reads as a twelve-band spectral
//! `Signature`. There is *no discrete intensity scale*, and the rule that gives this type its shape
//! is the document's own:
//!
//! > *"when you look into a being's eyes you should be able to read its soul, and if the screen
//! > cannot render the part of the spectrum that carries that reading, the reading is still
//! > computed and kept."*
//!
//! So [`SpiritualSignature::bands_outside`] **counts** what a given gamut cannot show, and nothing
//! in this type deletes or clamps a band to make a picture easier. A signature is computed once, at
//! creation, and travels with the being; the client decides what of it it can draw, and says so.
//!
//! The reference's own capture carries the first ghost as a bearer of one of these:
//! `logs/dev/affect-drift.txt` — *"ghost read Sad at tick 0 (#5500FF)"*, band `hard-uv`, radius
//! 40 000, drift −4 — which is why the numbers here are `u16` and the wheel is 432 rather than 360:
//! the reading is the reference's, not this crate's invention.

use crate::refusal::BeingRefusal;

/// Steps in the wheel's turn. **432, not 360**: the reference's wheel is a 432-step turn, and a
/// second turn length would be a second vocabulary.
pub const WHEEL_STEPS: u16 = 432;

/// Bands in one signature. Twelve: the reference's twelve-band spectral read.
pub const BAND_COUNT: usize = 12;

/// **A being's spectral read**: where on the wheel it sits, how far out, and the twelve bands.
///
/// Integer throughout. A float band would be a reading that drifts between two runs of the same
/// world, and the whole point of the reference's capture is that the reading is *kept* — a kept
/// reading that changes on recomputation is not kept.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SpiritualSignature {
    /// The wheel position, in `0..WHEEL_STEPS`.
    pub angle: u16,
    /// How far from the centre the reading sits, in the reference's own integer units.
    pub radius: u16,
    /// The twelve bands, in band order.
    pub bands: [u16; BAND_COUNT],
}

impl SpiritualSignature {
    /// A signature, refusing an angle outside the wheel.
    ///
    /// **The angle is the only thing there is to refuse.** `radius` and every band already have a
    /// valid range by their type (`u16`), so a range check on them would be a check that cannot
    /// fail, and a check that cannot fail is a claim about safety nobody measured.
    pub fn new(
        angle: u16,
        radius: u16,
        bands: [u16; BAND_COUNT],
    ) -> Result<Self, BeingRefusal> {
        if angle >= WHEEL_STEPS {
            return Err(BeingRefusal::AngleOutsideWheel {
                angle,
                steps: WHEEL_STEPS,
            });
        }
        Ok(SpiritualSignature {
            angle,
            radius,
            bands,
        })
    }

    /// A signature with no bands lit: the reading of a being that has just begun.
    ///
    /// Not a default and not a missing reading — `ZERO` bands are a measurement, and the type has no
    /// `Default` for the same reason `Ratio` has none: a defaulted signature would be a being that
    /// was never read being modelled as one read as nothing.
    pub fn unlit(angle: u16, radius: u16) -> Result<Self, BeingRefusal> {
        Self::new(angle, radius, [0; BAND_COUNT])
    }

    /// One band, by index, or `None` outside `0..BAND_COUNT`.
    pub fn band(&self, index: usize) -> Option<u16> {
        self.bands.get(index).copied()
    }

    /// The index of the strongest band. **First wins on a tie**, so the answer is deterministic
    /// rather than dependent on iteration order.
    pub fn peak_band(&self) -> usize {
        let mut peak = 0usize;
        for index in 1..BAND_COUNT {
            if self.bands[index] > self.bands[peak] {
                peak = index;
            }
        }
        peak
    }

    /// How many bands lie outside `visible`, a gamut given as an inclusive band-index range.
    ///
    /// **This is the "kept even when unshowable" rule as a number.** The client may draw only the
    /// bands its display can represent; this count is what it reports, so a screen that cannot show
    /// a ghost's ultra-violet says *one band unshown* rather than drawing a lesser ghost.
    pub fn bands_outside(&self, visible: std::ops::RangeInclusive<usize>) -> usize {
        (0..BAND_COUNT)
            .filter(|index| !visible.contains(index))
            .filter(|index| self.bands[*index] > 0)
            .count()
    }

    /// The strongest band's value, which is the reading's intensity in the reference's integers.
    pub fn peak(&self) -> u16 {
        self.bands[self.peak_band()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lit(index: usize, value: u16) -> [u16; BAND_COUNT] {
        let mut bands = [0u16; BAND_COUNT];
        bands[index] = value;
        bands
    }

    #[test]
    fn an_angle_past_the_wheel_is_refused_and_names_both_numbers() {
        let refused = SpiritualSignature::new(WHEEL_STEPS, 0, [0; BAND_COUNT])
            .expect_err("the wheel has 432 steps, so 432 is the first one past it");
        assert_eq!(refused.name(), "angle_outside_wheel");
        assert!(refused.describe().contains("432"), "{}", refused.describe());
        assert!(SpiritualSignature::new(WHEEL_STEPS - 1, 0, [0; BAND_COUNT]).is_ok());
    }

    #[test]
    fn the_peak_band_is_the_strongest_and_first_wins_a_tie() {
        let signature = SpiritualSignature::new(0, 40_000, lit(11, 5_500)).expect("valid");
        assert_eq!(signature.peak_band(), 11);
        assert_eq!(signature.peak(), 5_500);

        let tie = SpiritualSignature::new(0, 0, [3, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]).expect("valid");
        assert_eq!(tie.peak_band(), 0, "a tie is decided by order, not by chance");
    }

    #[test]
    fn a_band_the_gamut_cannot_show_is_counted_and_never_deleted() {
        // The reference's ghost: a reading whose carrying band is outside the visible range. The
        // signature keeps it; `bands_outside` says one band is unshown.
        let signature = SpiritualSignature::new(300, 40_000, lit(11, 9_000)).expect("valid");
        assert_eq!(signature.bands[11], 9_000, "the reading is kept");
        assert_eq!(signature.bands_outside(0..=9), 1);
        assert_eq!(signature.bands_outside(0..=11), 0);
    }

    #[test]
    fn an_unlit_signature_is_a_reading_of_zero_and_not_a_default() {
        let unlit = SpiritualSignature::unlit(0, 0).expect("valid");
        assert_eq!(unlit.peak(), 0);
        assert_eq!(unlit.band(BAND_COUNT), None);
        assert_eq!(unlit.bands_outside(0..=11), 0, "nothing lit is nothing unshown");
    }

    #[test]
    fn the_wheel_and_band_count_are_the_reference_numbers() {
        assert_eq!(WHEEL_STEPS, 432);
        assert_eq!(BAND_COUNT, 12);
        assert_eq!([0u16; BAND_COUNT].len(), 12);
    }
}
