//! Fractal combinations: fBm, ridged multifractal, and domain warping.
//!
//! * [`Fractal::fbm`] — Mandelbrot, B. & Van Ness, J. *Fractional Brownian
//!   Motions, Fractional Noises and Applications.* SIAM Review 1968. Summed
//!   octaves at a fixed lacunarity and gain.
//! * [`Fractal::ridged`] — Musgrave, Kolb & Mace. *The Synthesis and Rendering of
//!   Eroded Fractal Terrains.* SIGGRAPH 1989. The `1 − |n|` fold that produces
//!   the sharp crests real mountain ranges have and plain fBm does not.
//! * [`Fractal::warp`] / [`Fractal::warped_fbm`] — Perlin 1985 §; Quilez, I.
//!   *Domain warping* (2002). Displacing the sample point by another noise field,
//!   which is what turns "hills made of noise" into "terrain with folded strata".
//!
//! **The normalisation is stated, not implied.** fBm is divided by the sum of its
//! amplitudes, so `fbm` is in `[-1, 1]` by construction rather than by hope.
//! Without that division, adding an octave changes the amplitude of the whole
//! field, and that is exactly the kind of silent coupling that makes a generator
//! impossible to tune.

use crate::noise::{perlin3, value3};
use crate::point::Vec3;

/// Which base noise a fractal is built from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// Gradient noise. Smooth, the default for elevation.
    Perlin,
    /// Value noise. Blockier; several shapers want this on purpose (lithology
    /// beds, soil patches).
    Value,
}

/// The octave stack's parameters.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Fractal {
    pub octaves: u32,
    /// Frequency multiplier per octave. `2.0` is the textbook value.
    pub lacunarity: f64,
    /// Amplitude multiplier per octave. `0.5` is the textbook value; a higher
    /// gain makes a rougher field.
    pub gain: f64,
    pub kind: Kind,
}

impl Default for Fractal {
    fn default() -> Self {
        Fractal { octaves: 5, lacunarity: 2.0, gain: 0.5, kind: Kind::Perlin }
    }
}

impl Fractal {
    pub const fn new(octaves: u32, lacunarity: f64, gain: f64, kind: Kind) -> Self {
        Fractal { octaves, lacunarity, gain, kind }
    }

    #[inline]
    fn base(&self, seed: u64, p: Vec3) -> f64 {
        match self.kind {
            Kind::Perlin => perlin3(seed, p),
            Kind::Value => value3(seed, p),
        }
    }

    /// Standard fBm in `[-1, 1]`.
    pub fn fbm(&self, seed: u64, p: Vec3) -> f64 {
        if self.octaves == 0 {
            return 0.0;
        }
        let (mut freq, mut amp) = (1.0, 1.0);
        let (mut sum, mut norm) = (0.0, 0.0);
        for o in 0..self.octaves {
            // Each octave gets its own seed so octaves do not align into a
            // repeating lattice at the origin.
            sum += amp * self.base(seed ^ (o as u64).wrapping_mul(0x9E37_79B9), p * freq);
            norm += amp;
            freq *= self.lacunarity;
            amp *= self.gain;
        }
        if norm == 0.0 {
            0.0
        } else {
            sum / norm
        }
    }

    /// Ridged multifractal in `[0, 1]`: `0` at a valley, `1` at a crest.
    ///
    /// Musgrave's fold is `1 − |n|`, squared to sharpen, and each octave is
    /// **weighted by the previous octave's value** — that weighting is the
    /// "multifractal" part and it is what makes ridges take their character from
    /// the octave below instead of stacking independently.
    pub fn ridged(&self, seed: u64, p: Vec3) -> f64 {
        if self.octaves == 0 {
            return 0.0;
        }
        let (mut freq, mut amp) = (1.0, 1.0);
        let (mut sum, mut norm) = (0.0, 0.0);
        let mut weight = 1.0;
        for o in 0..self.octaves {
            let mut n = self.base(seed ^ (o as u64).wrapping_mul(0x9E37_79B9), p * freq);
            n = 1.0 - n.abs();
            n *= n;
            n *= weight;
            weight = (n * 2.0).clamp(0.0, 1.0);
            sum += amp * n;
            norm += amp;
            freq *= self.lacunarity;
            amp *= self.gain;
        }
        if norm == 0.0 {
            0.0
        } else {
            (sum / norm).clamp(0.0, 1.0)
        }
    }

    /// The domain-warp displacement: a vector of fBm offsets.
    pub fn warp(&self, seed: u64, p: Vec3, strength: f64) -> Vec3 {
        Vec3::new(
            self.fbm(seed ^ 0xA1, p),
            self.fbm(seed ^ 0xB2, p),
            self.fbm(seed ^ 0xC3, p),
        ) * strength
    }

    /// fBm of a **warped** sample point.
    ///
    /// Kept separate from [`Self::warp`] because the two are useful apart: the
    /// displacement is worth having on its own when a shaper wants to warp
    /// *another* shaper's input rather than its own.
    pub fn warped_fbm(&self, seed: u64, p: Vec3, strength: f64) -> f64 {
        self.fbm(seed, p + self.warp(seed, p, strength))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fbm_box(f: &Fractal, seed: u64) -> (f64, f64) {
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        for x in -20..20 {
            for y in -20..20 {
                for z in 0..20 {
                    let p = Vec3::new(x as f64 * 0.4, y as f64 * 0.4, z as f64 * 0.4);
                    let v = f.fbm(seed, p);
                    assert!(v.is_finite());
                    lo = lo.min(v);
                    hi = hi.max(v);
                }
            }
        }
        (lo, hi)
    }

    #[test]
    fn fbm_is_bounded_by_construction() {
        let f = Fractal::default();
        let (lo, hi) = fbm_box(&f, 1);
        assert!(lo >= -1.0 && hi <= 1.0, "fbm range was [{lo}, {hi}]");
        assert!(hi - lo > 0.5, "fbm was nearly flat: {lo}..{hi}");
    }

    #[test]
    fn adding_an_octave_does_not_rescale_the_field() {
        // The normalisation test: if fBm were summed without dividing by the
        // amplitude total, 2 octaves would have a *larger* range than 1, and
        // every tuned threshold downstream would move when someone added detail.
        let one = Fractal::new(1, 2.0, 0.5, Kind::Perlin);
        let four = Fractal::new(4, 2.0, 0.5, Kind::Perlin);
        let (lo1, hi1) = fbm_box(&one, 5);
        let (lo4, hi4) = fbm_box(&four, 5);
        assert!(hi1 <= 1.0 && hi4 <= 1.0);
        // Both fill most of the range; neither dominates.
        assert!(hi1 - lo1 > 0.8, "1 octave spanned {}..{}", lo1, hi1);
        assert!(hi4 - lo4 > 0.8, "4 octaves spanned {}..{}", lo4, hi4);
    }

    #[test]
    fn zero_octaves_is_zero_rather_than_a_nan_from_dividing_by_zero() {
        let f = Fractal::new(0, 2.0, 0.5, Kind::Perlin);
        assert_eq!(f.fbm(1, Vec3::ZERO), 0.0);
        assert_eq!(f.ridged(1, Vec3::ZERO), 0.0);
    }

    #[test]
    fn ridged_is_in_unit_range_and_reaches_both_ends() {
        let f = Fractal::new(4, 2.0, 0.5, Kind::Perlin);
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        for x in -30..30 {
            for y in -30..30 {
                let p = Vec3::new(x as f64 * 0.3, y as f64 * 0.3, 5.0);
                let v = f.ridged(3, p);
                assert!(v.is_finite() && (0.0..=1.0).contains(&v), "ridged was {v}");
                lo = lo.min(v);
                hi = hi.max(v);
            }
        }
        assert!(hi > 0.6, "ridged never formed a crest: {hi}");
        assert!(lo < 0.2, "ridged never formed a valley: {lo}");
    }

    #[test]
    fn ridged_and_fbm_are_different_fields() {
        // They must not be aliases: a ridge is a fold of the same noise, not the
        // same noise.
        let f = Fractal::default();
        let p = Vec3::new(1.3, 2.7, 0.4);
        assert_ne!(f.fbm(1, p), f.ridged(1, p));
    }

    #[test]
    fn warping_displaces_the_sample_and_changes_the_value() {
        let f = Fractal::default();
        let p = Vec3::new(4.1, 2.2, 8.3);
        let d = f.warp(1, p, 1.0);
        assert!(d.length() > 0.0, "warp did not displace anything");
        assert!(d.is_finite());
        assert_ne!(f.fbm(1, p), f.warped_fbm(1, p, 1.0));
    }

    #[test]
    fn value_and_perlin_kinds_are_different_fields() {
        let p = Vec3::new(0.7, 1.9, 2.4);
        let a = Fractal::new(4, 2.0, 0.5, Kind::Perlin).fbm(1, p);
        let b = Fractal::new(4, 2.0, 0.5, Kind::Value).fbm(1, p);
        assert_ne!(a, b);
    }
}
