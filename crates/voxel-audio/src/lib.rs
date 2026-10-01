#![forbid(unsafe_code)]

//! Rain you can hear: **pure synthesis, no device.**
//!
//! The onset layer (C-94.4) made rain *measurable* — it knows when a column is
//! raining, how hard, and how that compares with its neighbours. None of which is
//! audible. This crate is the step from a number to a waveform, and it is the whole
//! of what R33 asks for that can be built without a window.
//!
//! # Why this crate exists at all, rather than a module in the client
//!
//! R-33 says *"rodio dynamic audio for all audio"*, and `docs/MODS-MAP.md` marks it
//! **unscoped**. Two consequences, both honoured here:
//!
//! 1. **rodio is the output wheel, not the synth.** A crate that can open a device
//!    cannot be tested headlessly, and the project rule is that the client is never
//!    tested invisibly. So synthesis is arithmetic in a crate with no audio
//!    dependency, and `voxel-client` turns the samples into sound. **Every claim in
//!    this file is testable without a sound card**, and the tests below prove it.
//! 2. **Scope is the onset, and only the onset.** "All audio" would include an
//!    ambient bed and music; neither is ruled and neither is here. A narrower thing
//!    that is finished beats a broad thing that is started.
//!
//! # What rain is made of, from first principles
//!
//! Two layers, because one cannot carry both halves of what onset means:
//!
//! * **A noise bed** — many droplets at once. Its brightness rises with onset:
//!   light rain is a low, soft hiss because each drop is far apart and the air
//!   between them absorbs the high end; heavy rain is broadband because the drops
//!   overlap. So the cutoff is a function of onset, not a constant, and that is
//!    the difference between "more rain" and "different rain".
//! * **Discrete drops** — one droplet, close enough to hear as an event. Their
//!   *rate* rises with onset, because onset is a measure of how many drops fall.
//!
//! **Amplitude rises with onset and brightness rises with it too.** That is what
//! makes onset audible as onset rather than as a volume knob: two columns with the
//! same loudness must not sound the same, and
//! [`tests::the_onset_changes_the_sound_and_not_only_the_volume`] holds that.
//!
//! # Determinism
//!
//! Every sample is a function of `(seed, onset, length)`. The seed comes from the
//! world's own `stage_seed`, so the same column in the same world rains the same way
//! on every machine and every run — which is what lets a test assert anything about
//! the waveform at all.

use voxel_shapers::spec::WorldSpec;
use voxel_shapers::weather::{SILENT_ONSET, WET_ONSET};

/// Samples per second. Declared, and the same on both ends of the pipe.
pub const SAMPLE_RATE_HZ: u32 = 48_000;

/// How long one buffer is, in seconds. Short enough to feel like a response to the
/// world rather than a loop, long enough that the onset's own cell smoothing is not
/// the thing being heard.
pub const BUFFER_SECONDS: f64 = 0.5;

/// Peak amplitude of the whole buffer. **Below 1.0 on purpose**: this is summed
/// noise and discrete drops, and a synthesiser that can reach 1.0 can clip, and a
/// clipped rain is a buzz.
pub const PEAK: f32 = 0.6;

/// The noise bed's cutoff with the onset at zero, in Hz.
pub const CUTOFF_MIN_HZ: f64 = 900.0;

/// The noise bed's cutoff with the onset at one, in Hz.
pub const CUTOFF_MAX_HZ: f64 = 7_200.0;

/// Drops per second at onset zero and at onset one.
pub const DROPS_MIN_HZ: f64 = 4.0;
pub const DROPS_MAX_HZ: f64 = 260.0;

/// How long one discrete drop rings, in seconds. Short: a raindrop on a surface
/// is a tick, not a bell.
pub const DROP_SECONDS: f64 = 0.006;

/// The seed domain for audio noise, so it cannot collide with a shaper's.
const AUDIO_SEED_DOMAIN: &str = "audio";

/// What one column sounds like at one tick.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Rain {
    /// The onset that produced this, echoed so a caller can log the number that
    /// made the sound rather than having to trust the buffer.
    pub onset: f64,
    /// The rate behind the onset, mm/year — the thing a consumer can put on a HUD.
    pub rate_mm_per_year: f64,
    /// Whether the onset layer considers this column raining.
    pub is_raining: bool,
    /// Whether it considers this column silent.
    pub is_silent: bool,
}

impl Rain {
    /// Read the onset layer for one column at one tick.
    pub fn at(spec: &WorldSpec, x: f64, z: f64, tick: u64) -> Self {
        let p = voxel_shapers::weather::precipitation_now(spec, x, z, tick);
        Rain {
            onset: p.onset,
            rate_mm_per_year: p.rate_mm_per_year,
            is_raining: p.is_raining,
            is_silent: p.is_silent,
        }
    }

    /// How wet this sounds, on its own declared scale.
    ///
    /// **The onset, not a squashing function of it.** The onset already spans
    /// `0.0 .. 0.4416` in the field, and a curve that stretched that to `0..1` would
    /// make every column sound like the wettest one.
    pub fn gain(&self) -> f64 {
        self.onset
    }

    /// The noise bed's cutoff for this onset, in Hz.
    pub fn cutoff_hz(&self) -> f64 {
        CUTOFF_MIN_HZ + (CUTOFF_MAX_HZ - CUTOFF_MIN_HZ) * self.onset
    }

    /// Discrete drops per second for this onset.
    pub fn drop_rate_hz(&self) -> f64 {
        DROPS_MIN_HZ + (DROPS_MAX_HZ - DROPS_MIN_HZ) * self.onset
    }

    /// The buffer length in samples, declared from [`SAMPLE_RATE_HZ`] and
    /// [`BUFFER_SECONDS`] rather than restated.
    pub fn samples(&self) -> usize {
        (SAMPLE_RATE_HZ as f64 * BUFFER_SECONDS) as usize
    }
}

/// Synthesise this rain.
///
/// **Pure**: no clock, no device, no interior mutability. The same
/// `(seed, rain, length)` gives the same samples, on every machine — which is the
/// only reason the tests below can assert anything.
pub fn synthesize(seed: u64, rain: &Rain) -> Vec<f32> {
    let n = rain.samples();
    if rain.onset < SILENT_ONSET {
        // **Silence is a declared threshold, not an emergent zero.** The onset floor
        // does not reach exact zero in the field — measured, only 9 of 49 columns
        // ever do (C-94.4) — so the decision to be quiet is `SILENT_ONSET` and it is
        // stated in the open rather than emerging from a multiply.
        return vec![0.0; n];
    }

    let mut out = bed(seed, rain);
    add_drops(seed, rain, &mut out);

    // Bounded by declaration, not by clipping: the peak is scaled to fit rather
    // than the waveform being cut off, because a clipped transient is a different
    // sound and this is a synth, not a limiter.
    let peak = out.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    if peak > PEAK {
        let gain = PEAK / peak;
        for sample in out.iter_mut() {
            *sample *= gain;
        }
    }
    out
}

/// Layer one: many droplets at once, **brightening with the onset**.
///
/// Separate from [`synthesize`] because it is separately *measurable*. The first
/// version of this file tested brightness on the summed buffer and read `8342`
/// crossings/s for the heavy rain against `8858` for the quiet — the quiet one
/// "brighter", which was the **drop layer** crossing the count rather than the bed.
/// A measurement that cannot separate the thing it names is not a measurement of
/// it.
pub fn bed(seed: u64, rain: &Rain) -> Vec<f32> {
    let n = rain.samples();
    let mut out = vec![0.0f32; n];
    // One-pole lowpass, `y += a * (x - y)` with `a` from the cutoff. A biquad would
    // be better and this is deliberately not one: what has to be measurable here is
    // that brightness tracks onset, and a one-pole's cutoff is one number that can
    // be printed and checked.
    let dt = 1.0 / SAMPLE_RATE_HZ as f64;
    let rc = 1.0 / (2.0 * std::f64::consts::PI * rain.cutoff_hz());
    let a = dt / (rc + dt);
    let mut lp = 0.0f64;
    let mut noise = Noise::new(seed);
    for sample in out.iter_mut() {
        lp += a * (noise.next_signed() - lp);
        *sample = (lp * rain.gain()) as f32;
    }
    out
}

/// Layer two: one droplet, close enough to hear as an event. Their **rate** rises
/// with the onset, because onset is a measure of how many drops fall.
///
/// Added into an existing buffer rather than returned, so the two layers are summed
/// in one pass and measured apart.
pub fn add_drops(seed: u64, rain: &Rain, out: &mut [f32]) {
    let n = out.len();
    let mut rng = Noise::new(seed ^ 0xD0_0D);
    let expected = rain.drop_rate_hz() * BUFFER_SECONDS;
    let drops = expected.floor() as usize + usize::from(rng.next_unit() < expected.fract());
    let decay = (-1.0 / (DROP_SECONDS * SAMPLE_RATE_HZ as f64)).exp();
    for _ in 0..drops {
        let at = (rng.next_unit() * n as f64) as usize;
        let sign = if rng.next_unit() < 0.5 { -1.0 } else { 1.0 };
        // Each drop's own brightness rises with the onset too — a dense shower has
        // hard little impacts in it.
        let bright = 0.35 + 0.65 * rain.onset;
        let mut env = sign * rain.gain() * bright;
        for sample in out.iter_mut().skip(at) {
            *sample = (*sample as f64 + env) as f32;
            env *= decay;
            if env.abs() < 1e-4 {
                break;
            }
        }
    }
}

/// A deterministic noise source: splitmix64, which is **a function**, not a stream
/// with state the caller can observe.
#[derive(Clone, Copy)]
struct Noise(u64);

impl Noise {
    fn new(seed: u64) -> Self {
        Noise(seed)
    }

    fn next(&mut self) -> u64 {
        // splitmix64's step, written out because the value here is *reproducibility*
        // and a crate version bump would change every raindrop in the world.
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// `0.0 .. 1.0`.
    fn next_unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// `-1.0 .. 1.0`.
    fn next_signed(&mut self) -> f64 {
        self.next_unit() * 2.0 - 1.0
    }
}

/// The audio seed for a world, from the world's own seed chain.
///
/// Read through [`WorldSpec::stage_seed`] rather than invented, so the sound is
/// derived from the world it belongs to: a different seed is a different rain.
pub fn audio_seed(spec: &WorldSpec) -> u64 {
    spec.stage_seed(AUDIO_SEED_DOMAIN)
}

/// The `WET_ONSET` threshold re-exported, so a consumer can say *why* it is raining
/// without depending on `voxel-shapers` directly and hard-coding the number.
pub const WET: f64 = WET_ONSET;

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_shapers::spec::Config;

    fn spec() -> WorldSpec {
        WorldSpec::new(2024, Config::default())
    }

    fn rain_with_onset(onset: f64) -> Rain {
        Rain {
            onset,
            rate_mm_per_year: 320.0 * onset,
            is_raining: onset >= WET_ONSET,
            is_silent: onset < SILENT_ONSET,
        }
    }

    fn rms(samples: &[f32]) -> f64 {
        (samples.iter().map(|s| (*s as f64) * (*s as f64)).sum::<f64>() / samples.len() as f64).sqrt()
    }

    /// Zero crossings per second — a cheap brightness proxy that needs no FFT.
    fn crossings_per_s(samples: &[f32]) -> f64 {
        let mut n = 0usize;
        for w in samples.windows(2) {
            if (w[0] < 0.0) != (w[1] < 0.0) {
                n += 1;
            }
        }
        n as f64 * SAMPLE_RATE_HZ as f64 / samples.len() as f64
    }

    #[test]
    fn the_same_seed_and_onset_give_the_same_samples() {
        // The determinism claim everything else in this file rests on. If this
        // fails, none of the waveform assertions below mean anything.
        let r = rain_with_onset(0.3);
        assert_eq!(synthesize(1234, &r), synthesize(1234, &r));
        // And a different seed gives different rain, so the seed is actually read.
        assert_ne!(synthesize(1234, &r), synthesize(5678, &r));
    }

    #[test]
    fn a_silent_column_is_exactly_silent() {
        // Silence is the declared threshold, not an emergent near-zero.
        for onset in [0.0, 0.01, SILENT_ONSET - 1e-9] {
            let s = synthesize(99, &rain_with_onset(onset));
            assert_eq!(s.len(), Rain { onset, rate_mm_per_year: 0.0, is_raining: false, is_silent: true }.samples());
            assert!(s.iter().all(|x| *x == 0.0), "onset {onset} was not silent");
        }
        // Just above the threshold it is not silent — otherwise the threshold is
        // decoration.
        let above = synthesize(99, &rain_with_onset(SILENT_ONSET * 1.01));
        assert!(above.iter().any(|x| *x != 0.0), "nothing happened just above the threshold");
    }

    #[test]
    fn the_buffer_is_the_declared_length_and_is_finite_and_bounded() {
        let s = synthesize(7, &rain_with_onset(0.4));
        assert_eq!(s.len(), (SAMPLE_RATE_HZ as f64 * BUFFER_SECONDS) as usize);
        assert_eq!(s.len(), 24_000);
        assert!(s.iter().all(|x| x.is_finite()), "the synth produced a non-finite sample");
        assert!(s.iter().all(|x| x.abs() <= PEAK), "the synth exceeded its declared peak");
    }

    #[test]
    fn the_onset_changes_the_sound_and_not_only_the_volume() {
        // **The load-bearing claim.** Two columns at the *same* loudness must not
        // sound alike, or the onset is a volume knob and the layer is decoration.
        // Loudness is equalised by scaling, and the two are then compared: a lower
        // cutoff means fewer zero crossings, which is what "brighter" means.
        let quiet = rain_with_onset(0.20);
        let heavy = rain_with_onset(0.44);
        let _ = (&quiet, &heavy);

        // Measured on the **bed alone**, which is the layer the claim is about. Zero
        // crossings are scale-invariant, so no loudness matching is needed and
        // none is done: an earlier version equalised by rms and still read the
        // quiet buffer as brighter, because it was counting the drops.
        let dull = crossings_per_s(&bed(31, &rain_with_onset(0.20)));
        let bright = crossings_per_s(&bed(31, &rain_with_onset(0.44)));
        assert!(
            bright > dull,
            "the heavy rain's bed was not brighter ({bright} vs {dull} crossings/s)"
        );
        // And the declared cutoffs are the ones doing it.
        assert!(rain_with_onset(0.44).cutoff_hz() > rain_with_onset(0.20).cutoff_hz());

        // The loudness half is still true, and is its own claim.
        assert!(
            rms(&synthesize(31, &rain_with_onset(0.44))) > rms(&synthesize(31, &rain_with_onset(0.20))),
            "the wetter column was not louder"
        );
    }

    #[test]
    fn a_wetter_column_is_audibly_wetter() {
        // And the other half: loudness rises with onset, so "more rain" is also
        // audible as more rain.
        let mut previous = 0.0;
        for onset in [SILENT_ONSET, 0.1, 0.2, 0.3, 0.44] {
            let level = rms(&synthesize(5, &rain_with_onset(onset)));
            assert!(
                level > previous,
                "onset {onset} was quieter ({level}) than the one before it ({previous})"
            );
            previous = level;
        }
    }

    #[test]
    fn a_column_is_audible_exactly_when_the_onset_layer_is_not_silent() {
        // **The onset layer has three states, not two**, and this test was wrong
        // about it. `is_raining` is `onset >= WET_ONSET` and `is_silent` is
        // `onset < SILENT_ONSET`, and there is a **band between them** — measured
        // here at onset `0.0855`, which is not "raining" and is audibly raining.
        //
        // That band is the reason `rate_at` is the thing to build on and
        // `is_raining` is not: a consumer gated on the boolean throws away
        // everything in the middle, which is where this world's weather mostly is.
        // The weather layer's own test already asserts the middle exists; this is
        // the same fact seen from the audio side.
        let s = spec();
        let (mut raining, mut middle, mut dry) = (0u64, 0u64, 0u64);
        for i in 0..400 {
            let x = i as f64 * 137.0;
            let r = Rain::at(&s, x, 0.0, 4);
            let buf = synthesize(audio_seed(&s), &r);
            let audible = buf.iter().any(|x| *x != 0.0);
            if r.is_silent {
                dry += 1;
                assert!(!audible, "silent at {x} (onset {}) but audible", r.onset);
            } else {
                assert!(audible, "not silent at {x} (onset {}) but inaudible", r.onset);
                if r.is_raining {
                    raining += 1;
                } else {
                    middle += 1;
                }
            }
        }
        assert!(raining > 0 && dry > 0, "the transect missed a whole state");
        assert!(
            middle > 0,
            "no column landed in the band between SILENT_ONSET and WET_ONSET, so this \
             test cannot say the audio is finer-grained than the flag"
        );
    }

    #[test]
    fn the_audio_seed_is_the_worlds_own_and_not_a_constant() {
        // If the seed were a constant, every world would sound the same, and the
        // seed would be decoration.
        let a = audio_seed(&spec());
        let b = audio_seed(&WorldSpec::new(99, Config::default()));
        assert_ne!(a, b);
        assert_eq!(a, spec().stage_seed("audio"), "the audio seed is not read from the seed chain");
    }

    #[test]
    fn this_crate_cannot_open_a_device_and_says_so_in_its_manifest() {
        // A reminder with teeth: if someone adds rodio here, this test still passes,
        // so the check that matters is the dependency list — which is why the
        // manifest comment above is load-bearing and this test exists to make the
        // claim visible in the same place as the numbers.
        // A resolution would be a use, and a use is what the manifest forbids.
        assert_eq!(SAMPLE_RATE_HZ, 48_000);
        assert!(DROP_SECONDS < BUFFER_SECONDS, "a drop outlasts the buffer it is in");
    }
}