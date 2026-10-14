//! **U5's audio seam: *is it raining right now*, as a number a mixer can read.**
//!
//! Derived from `docs/LEDGER.md` **C-93.5** and **C-94**, in the record's own words. The problem C-94
//! states is precise: this world's precipitation is **mm/year**, a long-run mean, so nothing in the
//! shapers says when rain *starts* — "an audio layer that wants *is it raining right now* had nothing
//! to read". The ruling was to derive the onset **from tick noise**, and C-94 is the entry that built
//! it, with the constants table this module quotes:
//!
//! | Constant | Value | What it is (C-94's own column) |
//! |---|---|---|
//! | [`ONSET_CELL_TICKS`] | 2 000 | how many ticks one onset cell averages over |
//! | [`ONSET_CELL_M`] | 900 | the cell's edge in metres |
//! | [`ONSET_DRYNESS`] | 2.4 | how sharply a column separates from its neighbours |
//! | [`ONSET_FLOOR`] | 0.22 | the floor below which a cell is not raining at all |
//! | [`WET_ONSET`] | 0.18 | the declared "wet" threshold |
//! | [`SILENT_ONSET`] | 0.05 | the declared silence threshold |
//! | [`ONSET_DUTY`] | 0.0997 | the ensemble mean onset, as a fraction of samples |
//!
//! Measured on this build's own field (2026-10-12), `32 × 32` cells over 8 windows: the duty is
//! **0.0482** — the same order as the record's own 0.0997 and a factor of about two drier, because
//! the field's own noise is this build's. It is printed beside the record's figure rather than fitted
//! to it, and it is **pinned**: the field is a pure function of place and tick, so a moved threshold
//! or a changed field is a named failure. C-94's own constants are read as **values** against the
//! ledger's table rather than matched by name (`tests/u5_measure.rs`).
//!
//! # What this module is, and what is owed
//!
//! The seam is **one read and one number**: [`onset`] answers what the field is at a place and a
//! tick, and [`Weather::gain`] is the audio layer's own reading of it — silence, wetness, or the ramp
//! between them. [`ONSET_DUTY`] is the legacy's own measured figure and is quoted as a bar; this
//! build's own field measures its own duty ([`duty`]) and the two are printed side by side rather
//! than fitted together, because the field's own noise is this build's.
//!
//! # The voices
//!
//! C-94's seam answers *is it raining right now*; the **voices** are what a mixer does with the
//! answer. [`voice`] renders one buffer out of three declared inputs — the feeling wheel's own reading
//! (`src/feeling.rs`), the onset field's own gain, and the field's noise — and it carries **two voices
//! in one buffer**: the **bed**, whose level is the reading's own radius (the record's *intensity*) and
//! whose shape is the reading's own spectral signature, and the **rain**, whose level is the weather's
//! own gain. Nothing here reads a clock, nothing is inherited from the legacy's audio crate (P7), the
//! samples are bounded by the mixer's two declared ceilings, and the buffer's own digest is printed
//! beside its levels by the binary and measured by a test.
//!
//! **Owed, and named rather than silently missing:** the *material's* own voice (rain on basalt is not
//! rain on water, and that needs the acoustic columns the roster's own ruling puts on the material row
//! — H-20.4 — which this build does not carry yet), **wind**, and the **music layer**. What this module
//! does not do is voice a state it cannot read: an absent, undeclared, uncalibrated or contradictory
//! feeling is refused **by name** (`feeling_absent`, `feeling_undeclared`, `feeling_uncalibrated`,
//! `feeling_contradictory`) rather than voiced as its nearest neighbour.

use crate::digest::Digest;
use crate::feeling::{Feeling, RADIUS_RIM};
use crate::light::CHANNEL_COUNT;
use crate::noise::Noise;
use crate::refusal::{Refusal, RefusalKind};

/// How many ticks one onset cell averages over (C-94's own constant).
pub const ONSET_CELL_TICKS: u64 = 2_000;
/// The onset cell's edge, in metres (C-94's own constant).
pub const ONSET_CELL_M: f64 = 900.0;
/// How sharply a column separates from its neighbours: the field's own value is raised to this power,
/// which is what makes a threshold crossing a *separation* rather than a wobble.
pub const ONSET_DRYNESS: f64 = 2.4;
/// The floor below which a cell is not raining at all (C-94's own constant).
pub const ONSET_FLOOR: f64 = 0.22;
/// The declared "wet" threshold (C-94's own constant).
pub const WET_ONSET: f64 = 0.18;
/// The declared silence threshold (C-94's own constant).
pub const SILENT_ONSET: f64 = 0.05;
/// The legacy's own measured ensemble duty — the fraction of samples a wet onset covers. Quoted, not
/// pinned to this build: see [`duty`].
pub const ONSET_DUTY: f64 = 0.0997;
/// How far the field is sampled from one window to the next, in metres.
///
/// **Declared, not measured.** C-94's table fixes the cell's *duration* and its edge; it does not say
/// how the field moves with time, and a field that did not move would hold the same weather forever.
/// One cell per window is the smallest reading that moves it, and the tests hold what follows (the
/// field drifts, so a window's neighbours are not independent samples of nothing).
pub const ONSET_WINDOW_STRIDE_M: f64 = ONSET_CELL_M;
/// The field's own octaves, lacunarity and gain — declared, and the same three numbers for every
/// reader, so the duty below is a property of the field and not of a caller's fBm settings.
pub const ONSET_OCTAVES: u32 = 3;
/// The field's own lacunarity.
pub const ONSET_LACUNARITY: f64 = 2.0;
/// The field's own gain.
pub const ONSET_GAIN: f64 = 0.5;
/// The field's own scale, before the record's thresholds are applied to it.
///
/// **Declared, and the reason is a unit mismatch rather than a taste.** C-94's thresholds are stated in
/// the *field's* own units, and an fBm of gradient noise does not spread over the whole of `[0, 1]`: its
/// own deviation is a fraction of it, so applying a threshold stated for the field to the fBm's raw
/// value would read a world that never rains. `2.0` is the smallest round scale that puts the record's
/// own floor inside the field's own range, and what follows — the duty — is measured and printed rather
/// than asserted.
pub const ONSET_FIELD_SCALE: f64 = 2.0;

/// **The onset field at a place and a tick**: the read an audio layer asks for.
///
/// The cell is a box of [`ONSET_CELL_M`] metres and [`ONSET_CELL_TICKS`] ticks; the value is one point
/// of the field's own time axis inside it, so a tick anywhere in a window reads the same weather. The
/// value is the field's own fBm raised to [`ONSET_DRYNESS`], which is what makes neighbouring columns
/// separate rather than blend.
pub fn onset(noise: &Noise, tick: u64, at: [f64; 2]) -> Weather {
    let window = tick / ONSET_CELL_TICKS;
    let cell_x = (at[0] / ONSET_CELL_M).floor() * ONSET_CELL_M;
    let cell_z = (at[1] / ONSET_CELL_M).floor() * ONSET_CELL_M;
    let drifted_x = cell_x + window as f64 * ONSET_WINDOW_STRIDE_M;
    // **Rain is a positive anomaly of the field, and half the field is dry by construction.** The
    // potential is the tick-noise field's own smoothed value; where it is negative there is no rain
    // at all — a column holds no more water than the field gives it — and where it is positive the
    // value is raised to the record's own dryness, which is C-94's *how sharply a column separates
    // from its neighbours*. Normalising the whole field to `[0, 1]` instead would put the record's
    // thresholds near the field's own centre and rain half the time; this world's precipitation is
    // mm/year, and the duty below is measured rather than assumed either way.
    let potential =
        ONSET_FIELD_SCALE * noise.fbm(drifted_x, cell_z, ONSET_OCTAVES, ONSET_LACUNARITY, ONSET_GAIN);
    let onset = if potential <= 0.0 { 0.0 } else { potential.powf(ONSET_DRYNESS).min(1.0) };
    Weather { onset, window }
}

/// What the field says at one place and tick: the value, the window it came from, and the two
/// questions a consumer may ask of it.
///
/// The two questions are **different questions on one value**, and the record's own numbers make them
/// different: the floor is the *cell's* own condition (is this column raining at all?), while the
/// silence and wet thresholds are the *audio layer's* own reading of it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Weather {
    onset: f64,
    window: u64,
}

impl Weather {
    /// The field's own value, in `[0, 1]`.
    pub const fn onset(&self) -> f64 {
        self.onset
    }

    /// The onset window this value belongs to — `tick / ONSET_CELL_TICKS`.
    pub const fn window(&self) -> u64 {
        self.window
    }

    /// True when the column is raining at all: above [`ONSET_FLOOR`].
    pub fn is_raining(&self) -> bool {
        self.onset > ONSET_FLOOR
    }

    /// True when the audio layer must be silent: at or below [`SILENT_ONSET`].
    pub fn is_silent(&self) -> bool {
        self.onset <= SILENT_ONSET
    }

    /// The audio layer's own reading: `0` at or below [`SILENT_ONSET`], `1` at or above
    /// [`WET_ONSET`], and linear between the two. This is the seam's whole output — one number,
    /// monotone in the field, with both ends of the ramp being the record's own declared thresholds.
    pub fn gain(&self) -> f64 {
        if self.onset <= SILENT_ONSET {
            return 0.0;
        }
        if self.onset >= WET_ONSET {
            return 1.0;
        }
        (self.onset - SILENT_ONSET) / (WET_ONSET - SILENT_ONSET)
    }

    /// True when the audio layer reads the column as wet: at or above [`WET_ONSET`].
    pub fn is_wet(&self) -> bool {
        self.onset >= WET_ONSET
    }
}

/// The ensemble duty of this build's own field: the share of samples that read as wet, over `cells ×
/// cells` onset cells and `windows` windows.
///
/// **Measured, and pinned by the tests** to the figure this field returns — a pure function of place
/// and tick is reproducible, so a threshold drift moves this number and the tests fail by name.
/// C-94's own figure is [`ONSET_DUTY`] = 0.0997 on the legacy's field; this build's field is its own
/// noise, and the honest move is to measure it and print the two beside each other. A duty near zero
/// would be a world where it never rains, and the number is what says so.
pub fn duty(noise: &Noise, cells: i64, windows: u64) -> f64 {
    let mut wet = 0u64;
    let mut samples = 0u64;
    for cell_x in 0..cells {
        for cell_z in 0..cells {
            for window in 0..windows {
                let at = [
                    (cell_x as f64 + 0.5) * ONSET_CELL_M,
                    (cell_z as f64 + 0.5) * ONSET_CELL_M,
                ];
                let tick = window * ONSET_CELL_TICKS;
                if onset(noise, tick, at).is_wet() {
                    wet += 1;
                }
                samples += 1;
            }
        }
    }
    wet as f64 / samples.max(1) as f64
}

/// **The bed's own ceiling**: the loudest the feeling's bed may be, which it reaches at the rim.
///
/// Declared, and the reason is arithmetic rather than taste: two voices share one buffer and each has
/// its own ceiling, so [`LEVEL_CEILING`] — their sum — is what makes *the samples are bounded* a
/// property of the mixer's constants rather than a promise about its arithmetic.
pub const BED_GAIN: f64 = 0.5;
/// **The rain's own ceiling**, which it reaches at or above the wet threshold.
pub const RAIN_GAIN: f64 = 0.5;
/// The buffer's own ceiling: `BED_GAIN + RAIN_GAIN`, and the number the evidence measures `peak`
/// against. One band-set that sums to one, times a level no higher than its ceiling, plus a hiss no
/// louder than its own, cannot leave `±1` — and the print says so rather than assuming it.
pub const LEVEL_CEILING: f64 = BED_GAIN + RAIN_GAIN;
/// The rate the buffer is rendered at, in hertz.
///
/// **Declared.** The record fixes no sample rate and the tree carried none: 48 kHz is the rate a mixer
/// runs at and the one number at which this build's own frames-per-second is stated. Nothing reads a
/// clock, so a caller that renders at another rate gets the same samples with a different frame length
/// — which is what the test that renders two different frame counts proves.
pub const SAMPLE_RATE_HZ: f64 = 48_000.0;
/// The lowest band's own tone, in hertz: A2.
pub const BASE_HZ: f64 = 110.0;
/// How far one declared band steps, in octaves: eleven steps of a quarter-octave, so the twelve bands
/// span two and three-quarter octaves (A2 to F♯5) and one band is a minor third.
///
/// **This build's own declared calibration.** The record fixes the wheel's *light* — each family's own
/// anchor is a real wavelength — and says nothing whatever about sound, so the mapping from the twelve
/// bands to twelve tones is declared here, printed with every voice, and pinned by a digest: a moved
/// base, step or band count is a named failure rather than a sound that quietly changes. The direction
/// is the table's own: a later band holds a longer wavelength, and a longer wavelength is a higher
/// tone here.
pub const BAND_OCTAVE_STEPS: f64 = 0.25;
/// The frame count the evidence renders, and the shortest useful buffer:
/// `2 048` frames at 48 kHz is 42.67 ms, which holds four-and-a-half cycles of the lowest tone (110 Hz
/// is 436 frames) and the whole of the highest (740 Hz is 65). A caller may ask for any count above
/// zero; the samples do not depend on how many are asked for.
pub const VOICE_FRAMES: usize = 2_048;

/// The tone each of the twelve declared bands is voiced at: `110 · 2^(band · 0.25)` — A2 to F♯5 over
/// two and three-quarter octaves, one band per minor third. See [`BAND_OCTAVE_STEPS`] for why this is
/// a declared calibration and not a published one.
pub fn band_hz(band: usize) -> f64 {
    BASE_HZ * (band as f64 * BAND_OCTAVE_STEPS).exp2()
}

/// The twelve bands' own tones, in band order.
pub fn band_tones() -> [f64; CHANNEL_COUNT] {
    let mut tones = [0.0f64; CHANNEL_COUNT];
    for (band, tone) in tones.iter_mut().enumerate() {
        *tone = band_hz(band);
    }
    tones
}

/// **One rendered buffer**: the bed the feeling drives and the rain the onset field drives, mixed.
///
/// The type carries its own measurements rather than leaving them to a caller: the peak and the RMS
/// are read off the samples as rendered, the two levels are the ones the mix used, the signature is
/// the one the bed was built from, and the digest is the identity of the samples' own bytes. A buffer
/// that a reader cannot check the levels of is a buffer a reader has to trust.
#[derive(Clone, Debug, PartialEq)]
pub struct Voice {
    samples: Vec<f64>,
    bed_level: f64,
    rain_level: f64,
    signature: [f64; CHANNEL_COUNT],
    tones: [f64; CHANNEL_COUNT],
    sounding_bands: usize,
    peak: f64,
    rms: f64,
    digest: Digest,
}

impl Voice {
    /// The samples, frame by frame, one channel.
    pub fn samples(&self) -> &[f64] {
        &self.samples
    }

    /// How many frames the buffer holds.
    pub fn frames(&self) -> usize {
        self.samples.len()
    }

    /// How many channels: **one**, and said out loud so a reader does not assume a stereo image this
    /// build does not place.
    pub const fn channels(&self) -> usize {
        1
    }

    /// The bed's own level — the reading's radius as a gain, `0` at the centre and [`BED_GAIN`] at the
    /// rim.
    pub const fn bed_level(&self) -> f64 {
        self.bed_level
    }

    /// The rain's own level — the onset field's own gain, `0` below the silence threshold and
    /// [`RAIN_GAIN`] at or above the wet one.
    pub const fn rain_level(&self) -> f64 {
        self.rain_level
    }

    /// The signature the bed was built from: the feeling's own twelve weights, summing to one.
    pub const fn signature(&self) -> &[f64; CHANNEL_COUNT] {
        &self.signature
    }

    /// The twelve bands' own tones, in band order.
    pub const fn tones(&self) -> &[f64; CHANNEL_COUNT] {
        &self.tones
    }

    /// How many of the twelve bands the bed sounds.
    pub const fn sounding_bands(&self) -> usize {
        self.sounding_bands
    }

    /// The largest absolute sample as rendered.
    pub const fn peak(&self) -> f64 {
        self.peak
    }

    /// The root mean square of the samples as rendered.
    pub const fn rms(&self) -> f64 {
        self.rms
    }

    /// The identity of the samples' own bytes — the buffer's one content identity, so a moved band
    /// tone, a moved level or a moved field is a different buffer and not a different-sounding one.
    pub const fn digest(&self) -> Digest {
        self.digest
    }
}

/// **The voices: one buffer out of the feeling wheel, the weather and the field.**
///
/// Three declared inputs and one refusal of its own:
///
/// * the **reading** ([`Feeling`]) — the bed's level is its radius and the bed's shape is its
///   signature, so the wheel's intensity and its width both reach the samples as numbers;
/// * the **weather** ([`Weather`]) — the rain's level is [`Weather::gain`], the seam's own single
///   number, and the rain's own shape is the field's own unit noise at the frame and the window;
/// * the **field** ([`Noise`]) — read through `unit_at`, so the hiss is a function of the tick and the
///   frame and of nothing else.
///
/// `None` for the reading is [`RefusalKind::FeelingAbsent`] — **an absent feeling is not a silent
/// bed**: a buffer invented for a state nobody declared is a buffer a reader would take for a measured
/// one. Zero frames is [`RefusalKind::ZeroPeriod`], which is that kind's own meaning: a period of zero
/// seconds, of which no voice is a function.
pub fn voice(
    feeling: Option<&Feeling>,
    weather: &Weather,
    noise: &Noise,
    frames: usize,
) -> Result<Voice, Refusal> {
    let feeling = match feeling {
        Some(feeling) => feeling,
        None => {
            return Err(Refusal::new(
                RefusalKind::FeelingAbsent,
                "no feeling-wheel reading was handed to the voice: a bed is not invented for a state \
                 that was never declared",
            ))
        }
    };
    if frames == 0 {
        return Err(Refusal::new(
            RefusalKind::ZeroPeriod,
            format!(
                "0 frames at {SAMPLE_RATE_HZ} Hz is a period of zero seconds, and no voice is a \
                 function of nothing"
            ),
        ));
    }

    // The two levels: the record's own intensity for the bed, the seam's own gain for the rain. Both
    // are read from the values handed in and neither is fitted to the other.
    let bed_level = feeling.radius() as f64 / RADIUS_RIM as f64 * BED_GAIN;
    let rain_level = weather.gain() * RAIN_GAIN;
    let signature = feeling.signature();
    let tones = band_tones();
    let window = weather.window();

    // **Every positive weight is a partial at its band's own tone.** The weights sum to one, so the
    // bed's own sum of sines cannot leave `±1`; the hiss is one unit sample per frame, mapped to
    // `±1`; and each is scaled by a level no higher than its own ceiling. The samples are bounded by
    // construction, and `peak` below measures what was rendered rather than what was intended.
    let mut samples = Vec::with_capacity(frames);
    for frame in 0..frames {
        let seconds = frame as f64 / SAMPLE_RATE_HZ;
        let mut bed = 0.0f64;
        for (band, weight) in signature.iter().enumerate() {
            if *weight > 0.0 {
                bed += weight * (std::f64::consts::TAU * tones[band] * seconds).sin();
            }
        }
        let hiss = (noise.unit_at(frame as i64, window as i64) * 2.0 - 1.0) * rain_level;
        samples.push(bed * bed_level + hiss);
    }

    let peak = samples.iter().fold(0.0f64, |peak, sample| peak.max(sample.abs()));
    let rms = (samples.iter().map(|sample| sample * sample).sum::<f64>() / frames as f64).sqrt();
    let mut bytes = Vec::with_capacity(frames * std::mem::size_of::<f64>());
    for sample in &samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    let digest = Digest::of(&bytes);
    let sounding_bands = signature.iter().filter(|weight| **weight > 0.0).count();

    Ok(Voice {
        samples,
        bed_level,
        rain_level,
        signature,
        tones,
        sounding_bands,
        peak,
        rms,
        digest,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feeling::{RADIUS_CALM, RADIUS_PRESENT, RADIUS_RESOLVED, RADIUS_STRONG};

    fn noise() -> Noise {
        // The climate domain's own tag, so the weather field is not shared with the terrain's or the
        // geology's by accident (`crate::world::Domain`).
        Noise::new(0x5EED_0A57, 3)
    }

    #[test]
    fn a_window_holds_one_value_and_the_next_window_moves_it() {
        let noise = noise();
        let at = [120.0, -4_321.0];
        let first = onset(&noise, 0, at);
        assert_eq!(first.window(), 0);
        // Every tick inside the window reads the same weather: the cell averages over 2 000 ticks.
        assert_eq!(onset(&noise, 1_999, at).onset(), first.onset());
        assert_eq!(onset(&noise, 0, at).onset(), first.onset());
        // The next window is its own sample.
        let next = onset(&noise, ONSET_CELL_TICKS, at);
        assert_eq!(next.window(), 1);
        assert_ne!(next.onset(), first.onset());
    }

    #[test]
    fn a_cell_is_nine_hundred_metres_wide() {
        let noise = noise();
        let a = onset(&noise, 400, [10.0, 10.0]);
        let b = onset(&noise, 400, [899.0, 899.0]);
        assert_eq!(a.onset(), b.onset(), "one cell, one value");
        let next = onset(&noise, 400, [910.0, 10.0]);
        assert_ne!(next.onset(), a.onset(), "the next cell along its own edge");
    }

    #[test]
    fn the_field_is_a_function_of_place_and_tick_and_nothing_else() {
        let noise = noise();
        let one = onset(&noise, 7_000, [-1_234.0, 55.0]);
        let again = onset(&noise, 7_000, [-1_234.0, 55.0]);
        assert_eq!(one, again, "no clock, no state, no order");
    }

    #[test]
    fn the_gain_ramps_between_the_records_own_two_thresholds_and_never_leaves_them() {
        let at = |value: f64| Weather { onset: value, window: 0 };
        assert_eq!(at(0.0).gain(), 0.0);
        assert!(at(SILENT_ONSET).is_silent());
        assert_eq!(at(SILENT_ONSET).gain(), 0.0);
        assert!((at((SILENT_ONSET + WET_ONSET) / 2.0).gain() - 0.5).abs() < 1e-12);
        assert_eq!(at(WET_ONSET).gain(), 1.0);
        assert_eq!(at(0.9).gain(), 1.0, "a storm is not louder than wet");
        // Monotone across the ramp.
        let mut previous = -1.0;
        for step in 0..=40 {
            let value = step as f64 / 40.0;
            let gain = at(value).gain();
            assert!(gain >= previous, "the gain fell at {value}");
            previous = gain;
        }
        // The floor and the wet threshold are different questions on one value, and the record's own
        // numbers make them different: 0.22 is above 0.18.
        assert!(!at(0.20).is_raining(), "below the floor a cell is not raining at all");
        assert!(at(0.20).is_wet(), "and the audio layer already reads it as wet");
        // Every onset the field can hand back is a dry one until the potential is positive, so the
        // floor is reached only where the field's own value was above it.
        assert_eq!(at(0.0).onset(), 0.0);
    }

    /// **The two voices, measured on the buffer.** The bed's level is the reading's own intensity,
    /// the rain's is the seam's own gain, the samples stay inside the mixer's two ceilings, and the
    /// two levels are the ones the mix actually used.
    #[test]
    fn the_voice_mixes_a_bed_and_a_rain_and_its_samples_stay_bounded() {
        let noise = noise();
        let feeling = Feeling::parse("powerful", RADIUS_STRONG as i64, 0).expect("on the ladder");
        let wet = Weather { onset: 0.4, window: 3 };
        let voiced = voice(Some(&feeling), &wet, &noise, VOICE_FRAMES).expect("a declared reading");
        assert_eq!(voiced.frames(), VOICE_FRAMES);
        assert_eq!(voiced.channels(), 1);
        assert!(
            (voiced.bed_level() - 0.375).abs() < 1e-5,
            "49151/65535 of the bed's own ceiling: {}",
            voiced.bed_level()
        );
        assert_eq!(voiced.rain_level(), RAIN_GAIN, "a storm reads wet, and wet is not louder");
        assert!(voiced.peak() > 0.0 && voiced.peak() <= LEVEL_CEILING, "{}", voiced.peak());
        assert!(voiced.rms() > 0.0 && voiced.rms() < voiced.peak());
        // The bed is built from the reading's own signature, and nothing else.
        assert_eq!(voiced.signature(), &feeling.signature());
        assert_eq!(voiced.sounding_bands(), feeling.sounding_bands());
        // A silent column is no rain at all, and then the buffer is the bed alone.
        let dry = voice(Some(&feeling), &Weather { onset: 0.0, window: 3 }, &noise, VOICE_FRAMES)
            .expect("a declared reading");
        assert_eq!(dry.rain_level(), 0.0);
        assert!(
            dry.samples()
                .iter()
                .zip(voiced.samples())
                .all(|(dry, wet)| (wet - dry).abs() <= RAIN_GAIN + f64::EPSILON)
        );
        // And a reading at the centre is a silent bed: the radius **is** the intensity, so resolved
        // is no bed at all — which is a consequence of the record's own words and is said out loud.
        let resolved = Feeling::parse("peaceful", RADIUS_RESOLVED as i64, 0).expect("the centre");
        let quiet = voice(Some(&resolved), &Weather { onset: 0.0, window: 3 }, &noise, VOICE_FRAMES)
            .expect("a declared reading");
        assert_eq!(quiet.bed_level(), 0.0);
        assert_eq!(quiet.peak(), 0.0, "a resolved reading and a dry column: silence, not a guess");
        assert_eq!(quiet.sounding_bands(), 1, "and the signature is still one band wide");
    }

    /// **The buffer is a function of its inputs and nothing else** — no clock, no order, no frame
    /// count. Two renders are the same buffer; a changed reading is a different one; and a shorter
    /// render is a prefix of a longer one, which is what makes the frame count a caller's choice
    /// rather than part of the sound.
    #[test]
    fn the_buffer_is_a_function_of_its_inputs_and_of_nothing_else() {
        let noise = noise();
        let weather = Weather { onset: 0.3, window: 5 };
        let feeling = Feeling::parse("scared", RADIUS_PRESENT as i64, -2).expect("on the ladder");
        let one = voice(Some(&feeling), &weather, &noise, VOICE_FRAMES).expect("declared");
        let again = voice(Some(&feeling), &weather, &noise, VOICE_FRAMES).expect("declared");
        assert_eq!(one.digest(), again.digest(), "no clock and no state");
        assert_eq!(one.samples(), again.samples());
        let short = voice(Some(&feeling), &weather, &noise, 128).expect("declared");
        assert_eq!(short.samples(), &one.samples()[..128], "a shorter render is a prefix");
        assert_ne!(short.digest(), one.digest(), "and a different frame count is a different buffer");
        // A different reading is a different buffer, and so is a different tick's weather: the field
        // and the wheel both reach the samples.
        let other = Feeling::parse("scared", RADIUS_PRESENT as i64, 2).expect("on the ladder");
        assert_ne!(
            voice(Some(&other), &weather, &noise, VOICE_FRAMES).expect("declared").digest(),
            one.digest()
        );
        assert_ne!(
            voice(Some(&feeling), &Weather { onset: 0.3, window: 6 }, &noise, VOICE_FRAMES)
                .expect("declared")
                .digest(),
            one.digest()
        );
    }

    /// **No state, no buffer.** An absent feeling and a zero-frame buffer are each refused by name,
    /// and the two names differ because the two subjects do.
    #[test]
    fn an_absent_feeling_and_an_empty_period_are_refused_by_name() {
        let noise = noise();
        let weather = Weather { onset: 0.3, window: 1 };
        let absent = voice(None, &weather, &noise, VOICE_FRAMES).expect_err("no reading at all");
        assert_eq!(absent.kind(), RefusalKind::FeelingAbsent);
        assert!(absent.detail().contains("never declared"), "{absent}");
        let feeling = Feeling::parse("mad", RADIUS_CALM as i64, 0).expect("on the ladder");
        let empty = voice(Some(&feeling), &weather, &noise, 0).expect_err("no frames");
        assert_eq!(empty.kind(), RefusalKind::ZeroPeriod);
        assert!(empty.detail().contains("zero seconds"), "{empty}");
    }

    /// The twelve tones are the declared calibration, and they are **strictly rising with the band**,
    /// so a drift that walks the peak long walks it *up*: three octaves end to end, a quarter of an
    /// octave a band.
    #[test]
    fn the_bands_own_tones_are_the_declared_calibration_and_rise_with_the_band() {
        let tones = band_tones();
        assert_eq!(tones.len(), CHANNEL_COUNT);
        assert_eq!(tones[0], BASE_HZ);
        // A2 to F♯5: eleven quarter-octave steps, so the top band is `110 · 2^2.75` and the ratio end
        // to end is that same 2^2.75 — 2.75 octaves, which is what the record's own twelve bands are
        // mapped onto.
        assert!((tones[CHANNEL_COUNT - 1] - 739.988_845_6).abs() < 1e-6, "{}", tones[CHANNEL_COUNT - 1]);
        assert!(
            (tones[CHANNEL_COUNT - 1] / tones[0] - 2f64.powf(11.0 * BAND_OCTAVE_STEPS)).abs() < 1e-12,
            "eleven quarter-octave steps"
        );
        for band in 1..CHANNEL_COUNT {
            assert!(tones[band] > tones[band - 1], "band {band} did not rise");
            assert!((tones[band] / tones[band - 1] - 2f64.powf(BAND_OCTAVE_STEPS)).abs() < 1e-12);
        }
    }

    #[test]
    fn the_fields_duty_is_measured_and_not_assumed() {
        let noise = noise();
        let measured = duty(&noise, 32, 8);
        println!(
            "U5 audio: this build's field duty {measured:.4} over 32x32 cells x 8 windows \
             (the record's own measured figure: {ONSET_DUTY})"
        );
        // **Pinned, not banded.** The field is a pure function of place and tick, so its own measured
        // duty is reproducible; a moved threshold (or a changed field) moves it and is a named failure
        // here. The record's 0.0997 is the legacy's own field, quoted beside it, and C-94's constants
        // are read as values in `tests/u5_measure.rs` — the comparison is evidence because this side
        // is pinned and that side is value-read.
        assert!(
            (measured - 0.0482).abs() < 1e-4,
            "this field's own duty moved: {measured}"
        );
    }
}
