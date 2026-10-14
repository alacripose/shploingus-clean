//! **The feeling wheel, read as a value: six families, a radius, and a signed drift.**
//!
//! Derived from the feeling-wheel vocabulary — `feeling-wheel/SKILL.md`, the file that calls itself
//! *"the operational vocabulary for continuous agent use"*, and its sibling `reference.md`, which the
//! same file lists under *Reference assets*. Quoted as read:
//!
//! > There are six primary families: **Mad** (Red, 645 nm), **Scared** (Orange, 600 nm), **Joyful**
//! > (Yellow, 575 nm), **Powerful** (Green, 545 nm), **Peaceful** (Blue, 480 nm), **Sad** (Purple,
//! > 420 nm). *"Each anchor is a real wavelength, so a state is a spectral read and not only a word."*
//!
//! > The wheel is integer-only and radial. A position is an **angle** and a **radius** from the centre
//! > (0, resolved) to the rim (65535, extreme).
//!
//! > The family determines the **hue**. The **radius** determines the intensity, and the bearer's
//! > signed **drift** decides how far the spectral peak leaves the visible as they escalate. *"There
//! > is no shade ladder and no discrete 1–6 scale."*
//!
//! > The named marks are reporting conveniences only: `resolved 0`, `calm 13107`, `present 32767`,
//! > `strong 49151`, `extreme 65535`. *"At the centre the signature is one band wide at the family's
//! > own hue. At the rim it fills all twelve declared bands."*
//!
//! > *"A bearer declares a signed drift: how many bands its peak walks at the rim, refused by name if
//! > it is off the ladder. The reading's excursion is the radius' fraction of it, so a calm reading
//! > never leaves the band its family hue sits in, and an escalated one goes as far as the bearer
//! > declares. Positive walks toward the long wavelengths (near-IR to radio), negative toward the
//! > short (soft-UV to ionizing)."*
//!
//! This module is the **reading**: it holds a family, a radius and a drift, and it answers the three
//! questions a voice asks of it — *which band is the peak in*, *how many bands does the signature
//! fill*, and *what weight does each band carry*. It computes **no sound**: `src/audio.rs` owns the
//! voices, and the two meet through [`Feeling::signature`] and nothing else.
//!
//! # The twelve bands are the tree's own twelve
//!
//! The wheel says its signature fills *"all twelve declared bands — ionizing, the visible, and
//! radio"*, and **it names three of the twelve**. This build does not invent the other nine: the tree
//! already declares exactly twelve bands, in `src/light.rs`, over a published span (400–700 nm, the
//! visible), and the six family anchors are wavelengths that fall inside it. So the wheel's twelve
//! declared bands are read as **this tree's twelve** (`light::Spectra::band_of_nm`, which owns the
//! span's arithmetic), the family's own band is the band holding its anchor, and the drift's ladder is
//! what remains inside those twelve. The difference between the wheel's ionizing-to-radio span and the
//! table's visible span is **named here rather than papered over**: a reading whose peak would leave
//! the visible — the wheel's own `carrying-the-loss`, Sad at the extreme radius with drift −4, whose
//! peak *"walks out of the visible into the ultra-violet"* — has **no band in this table to be voiced
//! by**, and this build refuses it as [`RefusalKind::FeelingUncalibrated`] rather than voicing the
//! nearest band it happens to have. The wheel's own worked example is one of the tests.
//!
//! # What is owed, and named rather than silently missing
//!
//! * **Which label count is canonical.** The two files beside the wheel disagree: `SKILL.md` declares
//!   *"6 primaries + 72 secondaries + 432 tertiaries = 510 labels"*, `reference.md` declares *"6
//!   primaries + 36 secondary emotions + 36 tertiary emotions = 78 named emotion labels total"*. Both
//!   agree on the six families, their colours and their anchors, which is what this module reads; the
//!   label counts are a vocabulary question and are **owed** to whoever owns that document.
//! * **The tertiary vocabulary.** This build declares the six families and no more: it does not carry
//!   72 (or 36) secondary states or 432 (or 36) tertiary ones, and a name it does not declare is
//!   [`RefusalKind::FeelingUndeclared`](crate::refusal::RefusalKind::FeelingUndeclared) rather than a
//!   nearest match.
//! * **The hex colours.** `reference.md` carries one approximate hex per family and says so itself:
//!   *"approximate visual color references sampled from the supplied raster image, not claimed
//!   design-specification values."* The six colour **words** are declared in both files and are what
//!   [`Family::hue`] returns; the sampled hexes are not carried, because a value the record calls
//!   approximate is not a value this build should depend on.
//! * **The locus sweep.** *"The locus — the visible projection — also sweeps around the wheel, so a
//!   family's later states lean toward the next family's hue."* That is an angle effect, and this
//!   build carries no angle: it reads the hue, the radius and the drift, which is what the record
//!   needs for a spectral signature. The sweep is owed to the layer that draws the wheel.

use crate::light::{Spectra, CHANNEL_COUNT};
use crate::refusal::{Refusal, RefusalKind};

/// The six primary families, in the wheel's own ring order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Family {
    /// Red, anchored at 645 nm — *"friction, anger, violation, heat."*
    Mad,
    /// Orange, anchored at 600 nm — *"uncertainty, threat, vulnerability."*
    Scared,
    /// Yellow, anchored at 575 nm — *"delight, energy, play, discovery."*
    Joyful,
    /// Green, anchored at 545 nm — *"agency, confidence, competence, importance."*
    Powerful,
    /// Blue, anchored at 480 nm — *"calm, trust, connection, reflection."*
    Peaceful,
    /// Purple, anchored at 420 nm — *"loss, shame, loneliness, depletion."*
    Sad,
}

impl Family {
    /// How many families the wheel declares.
    pub const COUNT: usize = 6;

    /// Every family, in ring order — the closed vocabulary, so a reader that meets a family name
    /// outside this process can turn it back into a family rather than guessing one.
    pub const ALL: [Family; Family::COUNT] = [
        Family::Mad,
        Family::Scared,
        Family::Joyful,
        Family::Powerful,
        Family::Peaceful,
        Family::Sad,
    ];

    /// The family's own word, lower-cased: `mad`, `scared`, `joyful`, `powerful`, `peaceful`, `sad`.
    /// **The name is the contract** — a caller may match on it — so it does not change to make a test
    /// pass.
    pub fn name(&self) -> &'static str {
        match self {
            Family::Mad => "mad",
            Family::Scared => "scared",
            Family::Joyful => "joyful",
            Family::Powerful => "powerful",
            Family::Peaceful => "peaceful",
            Family::Sad => "sad",
        }
    }

    /// The colour **word** the wheel declares for the family — the "Base colour" column of its own
    /// table, and the same in both of its files. The approximate sampled hexes are not carried: see
    /// the module's *owed* list.
    pub fn hue(&self) -> &'static str {
        match self {
            Family::Mad => "red",
            Family::Scared => "orange",
            Family::Joyful => "yellow",
            Family::Powerful => "green",
            Family::Peaceful => "blue",
            Family::Sad => "purple",
        }
    }

    /// The family's anchor, in nanometres — a **real wavelength**, which is what makes a state a
    /// spectral read rather than only a word.
    pub fn anchor_nm(&self) -> f64 {
        match self {
            Family::Mad => 645.0,
            Family::Scared => 600.0,
            Family::Joyful => 575.0,
            Family::Powerful => 545.0,
            Family::Peaceful => 480.0,
            Family::Sad => 420.0,
        }
    }

    /// The family a declared word names, or nothing. Case-insensitive, because the wheel's own tables
    /// capitalise (`Mad`) and its state records do not (`State: Scared / Anxious / Orange`); an
    /// undeclared word is refused by name rather than matched to its nearest neighbour
    /// (*"Do not invent a new hue simply because an emotion feels unusual."*).
    pub fn parse(word: &str) -> Option<Family> {
        let word = word.trim().to_ascii_lowercase();
        Family::ALL.iter().copied().find(|family| family.name() == word)
    }

    /// Every family's own word, in ring order — for a refusal that quotes the closed vocabulary back.
    pub fn words() -> [&'static str; Family::COUNT] {
        [
            Family::Mad.name(),
            Family::Scared.name(),
            Family::Joyful.name(),
            Family::Powerful.name(),
            Family::Peaceful.name(),
            Family::Sad.name(),
        ]
    }

    /// **The band of the tree's declared table that holds this family's anchor.**
    ///
    /// The table owns its own span (`src/light.rs`), so the lookup is one table method and not a
    /// second copy of the span arithmetic (P3). Two anchors (600 nm and 575 nm) land exactly on a
    /// shared edge and the table's interval is half-open, so the answer is still one band rather than
    /// two. `None` means the anchor is outside the table, which is a refusal at construction rather
    /// than a clamped band.
    pub fn band(&self) -> Option<usize> {
        Spectra::standard().band_of_nm(self.anchor_nm())
    }
}

/// The radius' own centre: *"resolved"* — at rest.
pub const RADIUS_RESOLVED: u32 = 0;
/// The radius at *"calm"*.
pub const RADIUS_CALM: u32 = 13_107;
/// The radius at *"present"*.
pub const RADIUS_PRESENT: u32 = 32_767;
/// The radius at *"strong"*.
pub const RADIUS_STRONG: u32 = 49_151;
/// The radius' own rim: *"extreme"*, and the top of the wheel's integer span.
pub const RADIUS_RIM: u32 = 65_535;

/// **A reading of the wheel**: one family, one radius, one signed drift.
///
/// The fields are private and the only ways in are [`Feeling::of`] and [`Feeling::parse`], both of
/// which return a [`Refusal`] rather than a substituted value. So a `Feeling` that exists is a
/// reading the wheel can voice: its peak band is one of the twelve declared bands by construction,
/// and no method on this type can panic for a reason the constructor did not already refuse.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Feeling {
    family: Family,
    radius: u32,
    drift: i32,
    /// Resolved once, at construction, from [`Family::band`] — so the peak's arithmetic below cannot
    /// be reached without a declared band behind it.
    band: usize,
}

impl Feeling {
    /// A reading from the families it names and its two numbers.
    ///
    /// Four refusals, each on its own subject:
    ///
    /// * **no family at all** → [`RefusalKind::FeelingAbsent`]. An absent reading is not a silent
    ///   bed: it is a bed nobody asked for, and this build does not invent one.
    /// * **more than one family** → [`RefusalKind::FeelingContradictory`]. The wheel's own rule is
    ///   *"If multiple families compete, select one dominant family rather than blending hues"*, so a
    ///   reading that names two families is a reading that contradicts itself, not an average.
    /// * **a radius outside the wheel's own span** → [`RefusalKind::FeelingUncalibrated`]. The wheel's
    ///   marks run from the centre to the rim; *"extreme"* is `65535`, and the excursion is the
    ///   radius' **fraction of the rim**, so a radius past it is not a fraction of anything the wheel
    ///   declares.
    /// * **a drift off the ladder** → [`RefusalKind::FeelingUncalibrated`]. The ladder is what keeps
    ///   the peak inside the twelve declared bands at the rim; *"refused by name if it is off the
    ///   ladder"* is the record's own sentence for exactly this case.
    pub fn of(families: &[Family], radius: i64, drift: i64) -> Result<Feeling, Refusal> {
        let family = match families {
            [] => {
                return Err(Refusal::new(
                    RefusalKind::FeelingAbsent,
                    "no family was named: there is no reading here to voice, and a bed is not invented \
                     for a state that was never declared",
                ))
            }
            [family] => *family,
            many => {
                let named: Vec<String> =
                    many.iter().map(|family| family.name().to_string()).collect();
                return Err(Refusal::new(
                    RefusalKind::FeelingContradictory,
                    format!(
                        "{} families are named at once ({}); the wheel selects one dominant family \
                         rather than blending hues",
                        many.len(),
                        named.join(", ")
                    ),
                ));
            }
        };

        if radius < 0 || radius > RADIUS_RIM as i64 {
            return Err(Refusal::new(
                RefusalKind::FeelingUncalibrated,
                format!(
                    "radius {radius} is outside the wheel's own span 0..={RADIUS_RIM}: the excursion \
                     is the radius' fraction of the rim and there is no fraction to take"
                ),
            ));
        }

        let band = match family.band() {
            Some(band) => band,
            None => {
                return Err(Refusal::new(
                    RefusalKind::FeelingUncalibrated,
                    format!(
                        "{} is anchored at {} nm, outside the declared band table's own span: no band \
                         can voice it",
                        family.name(),
                        family.anchor_nm()
                    ),
                ))
            }
        };

        let (low, high) = Feeling::ladder_in(band);
        if drift < low as i64 || drift > high as i64 {
            return Err(Refusal::new(
                RefusalKind::FeelingUncalibrated,
                format!(
                    "drift {drift} bands is off the ladder: {} ({}) sits in band {band} of \
                     {CHANNEL_COUNT} and can walk {low}..={high} bands and still be voiced by a \
                     declared band",
                    family.name(),
                    family.hue()
                ),
            ));
        }

        Ok(Feeling { family, radius: radius as u32, drift: drift as i32, band })
    }

    /// A reading from the family's own **word**, for a caller that holds the wheel's own label rather
    /// than the enum. An undeclared word is [`RefusalKind::FeelingUndeclared`] — the refusal quotes
    /// the six words it does know, so the caller can see what it should have said.
    pub fn parse(word: &str, radius: i64, drift: i64) -> Result<Feeling, Refusal> {
        match Family::parse(word) {
            Some(family) => Feeling::of(&[family], radius, drift),
            None => Err(Refusal::new(
                RefusalKind::FeelingUndeclared,
                format!(
                    "`{word}` is not one of the wheel's six families ({}): an undeclared word is not \
                     matched to its nearest neighbour",
                    Family::words().join(", ")
                ),
            )),
        }
    }

    /// The family the reading names.
    pub const fn family(&self) -> Family {
        self.family
    }

    /// The radius: the intensity, `0` at the centre and `65535` at the rim.
    pub const fn radius(&self) -> u32 {
        self.radius
    }

    /// The bearer's signed drift, in bands at the rim.
    pub const fn drift(&self) -> i32 {
        self.drift
    }

    /// The band the family's own hue occupies.
    pub const fn band(&self) -> usize {
        self.band
    }

    /// **The excursion**: *"the reading's excursion is the radius' fraction of it"* — the signed
    /// drift scaled by the radius' own fraction of the rim, rounded, because the wheel is
    /// *integer-only*. At the centre it is zero (a calm reading never leaves its family's band, which
    /// the record states); at the rim it is the drift entire.
    pub fn excursion(&self) -> i32 {
        let walk = self.drift as f64 * (self.radius as f64 / RADIUS_RIM as f64);
        walk.round() as i32
    }

    /// The band the **peak** sits in: the family's own band walked by the excursion. **Inside the
    /// twelve declared bands by construction** — the ladder refused at construction is what makes that
    /// true — so this is a value and not a `Result`.
    pub fn peak_band(&self) -> usize {
        let peak = self.band as i32 + self.excursion();
        debug_assert!((0..CHANNEL_COUNT as i32).contains(&peak), "the ladder keeps the peak inside");
        peak.clamp(0, CHANNEL_COUNT as i32 - 1) as usize
    }

    /// The drift values a family in this band can declare and still be voiced: `-band ..= 11 - band`.
    ///
    /// Two consequences the record's own examples make concrete. A family sitting at a band's own end
    /// has **no inward walk**: purple's anchor is 420 nm, inside the table's **first** band, so
    /// `carrying-the-loss` — the wheel's own example, Sad at the extreme radius with drift −4, *"peak
    /// walks out of the visible into the ultra-violet"* — is off the ladder and refused by name.
    /// Yellow's walk in the other direction is short for the same reason (`hyped`, drift −3, is on the
    /// ladder because band 7 has seven bands below it).
    pub const fn ladder_in(band: usize) -> (i32, i32) {
        (-(band as i32), CHANNEL_COUNT as i32 - 1 - band as i32)
    }

    /// The reading's own ladder.
    pub const fn ladder(&self) -> (i32, i32) {
        Feeling::ladder_in(self.band)
    }

    /// The signature's **half-width, in bands**: *"at the centre the signature is one band wide … at
    /// the rim it fills all twelve declared bands."*
    ///
    /// So the half-width runs from `1` (the centre: the peak's own band and nothing else) to
    /// `CHANNEL_COUNT` (the rim: every band, including the one eleven bands away), linearly in the
    /// radius. The shape between the two ends is **this build's own declared reading** — the record
    /// fixes both ends and does not draw the line between them — and what follows (the weights below)
    /// is a measurement, not an assertion.
    pub fn width(&self) -> f64 {
        let fill = self.radius as f64 / RADIUS_RIM as f64;
        1.0 + fill * (CHANNEL_COUNT as f64 - 1.0)
    }

    /// **The signature**: one weight per declared band, summing to `1`, hot at the peak and falling
    /// off with the band's distance from it.
    ///
    /// The weights are `max(0, half-width − distance)`, normalised. The two ends the record fixes are
    /// both exact: at `radius = 0` the peak's band is the only positive weight, and at the rim every
    /// one of the twelve is positive. The weights are what a voice reads — `src/audio.rs` multiplies
    /// this by its own gain into one band tone per band — so the wheel's own intensity and its own
    /// width both reach the buffer as numbers rather than as a feeling.
    pub fn signature(&self) -> [f64; CHANNEL_COUNT] {
        let half = self.width();
        let peak = self.peak_band() as f64;
        let mut weights = [0.0f64; CHANNEL_COUNT];
        let mut total = 0.0f64;
        for (band, weight) in weights.iter_mut().enumerate() {
            let distance = (band as f64 - peak).abs();
            *weight = (half - distance).max(0.0);
            total += *weight;
        }
        if total > 0.0 {
            for weight in weights.iter_mut() {
                *weight /= total;
            }
        }
        weights
    }

    /// How many of the twelve bands the signature sounds: `1` at the centre, `12` at the rim.
    pub fn sounding_bands(&self) -> usize {
        self.signature().iter().filter(|weight| **weight > 0.0).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The wheel's own six-row table, read as values: each family's colour word and its anchor.
    #[test]
    fn the_six_families_carry_the_wheels_own_colours_and_anchors() {
        let table = [
            (Family::Mad, "red", 645.0),
            (Family::Scared, "orange", 600.0),
            (Family::Joyful, "yellow", 575.0),
            (Family::Powerful, "green", 545.0),
            (Family::Peaceful, "blue", 480.0),
            (Family::Sad, "purple", 420.0),
        ];
        assert_eq!(table.len(), Family::COUNT);
        assert_eq!(Family::ALL.len(), Family::COUNT);
        for (index, (family, hue, anchor)) in table.iter().enumerate() {
            assert_eq!(Family::ALL[index], *family, "ring order");
            assert_eq!(family.name(), format!("{family:?}").to_ascii_lowercase());
            assert_eq!(family.hue(), *hue);
            assert_eq!(family.anchor_nm(), *anchor);
            assert_eq!(Family::parse(&format!("{family:?}")), Some(*family), "the wheel's own capital");
            assert_eq!(Family::parse(hue), None, "a colour word is not a family name");
        }
        assert_eq!(Family::parse("  sad  "), Some(Family::Sad), "the wheel is read generously");
        assert_eq!(Family::parse("anxious"), None, "a secondary state is not a family");
    }

    /// **Every anchor lands in a declared band, and the bands are six of the twelve** — which is the
    /// whole reason this build can voice the wheel out of the table it already has.
    #[test]
    fn every_anchor_lands_in_a_declared_band() {
        let table = Spectra::standard();
        let mut bands = Vec::new();
        for family in Family::ALL {
            let band = family.band().expect("an anchor inside the table's own span");
            // The band is the table's own answer for that wavelength, and the anchor is inside the
            // table's own published span — read through the table's own API, not through a second
            // copy of its arithmetic. (That the interval is half-open is `light.rs`'s own test: each
            // band's lower end is in it and its upper end belongs to the next.)
            assert_eq!(table.band_of_nm(family.anchor_nm()), Some(band));
            assert!(crate::light::VISIBLE_LO_NM <= family.anchor_nm());
            assert!(family.anchor_nm() < crate::light::VISIBLE_HI_NM);
            bands.push((family.name(), band));
        }
        assert_eq!(
            bands,
            vec![
                ("mad", 9),
                ("scared", 8),
                ("joyful", 7),
                ("powerful", 5),
                ("peaceful", 3),
                ("sad", 0)
            ],
            "600 nm and 575 nm land on a shared edge and belong to the band they open"
        );
        // The symmetry check that follows from those six: only the two ends of the table have a walk
        // in one direction, and every family inside has both.
        for family in Family::ALL {
            let (low, high) = Feeling::ladder_in(family.band().expect("in the table"));
            assert!(high >= 0, "{} can walk long", family.name());
            // Every family but the table's first band can walk short as well; purple sits *in* the
            // first band, which is why its own example is the one this build refuses.
            assert_eq!(low < 0, family.band() != Some(0), "{}", family.name());
        }
    }

    /// The wheel's own worked example, first line: `irritated` — Mad, a strong radius, no drift, whose
    /// **peak** is red's own band.
    ///
    /// The example's prose then says *"one band wide at red"*, and the geometry says otherwise:
    /// *"at the centre the signature is one band wide … at the rim it fills all twelve declared bands"*,
    /// with the width set by the radius between them. For a **strong** radius those two statements
    /// disagree — the prose is the centre's reading — and this build follows the geometry, because that
    /// is where both ends of the width are fixed, and **names the difference** rather than choosing the
    /// tidier sentence.
    #[test]
    fn a_no_drift_mad_reading_peaks_at_red_and_the_width_follows_the_geometry() {
        let feeling = Feeling::parse("Mad", RADIUS_STRONG as i64, 0).expect("the wheel's own example");
        assert_eq!(feeling.family(), Family::Mad);
        assert_eq!(feeling.band(), 9);
        assert_eq!(feeling.peak_band(), 9, "no drift, no excursion");
        assert_eq!(feeling.excursion(), 0);
        let signature = feeling.signature();
        let hottest = signature.iter().copied().fold(f64::MIN, f64::max);
        assert_eq!(signature[9], hottest, "the peak's own band carries the most weight");
        assert!((signature.iter().sum::<f64>() - 1.0).abs() < 1e-12);
        // The geometry's own reading at a strong radius: a wide signature rather than a single band.
        assert!(feeling.width() > 9.0, "{}", feeling.width());
        assert_eq!(feeling.sounding_bands(), 12);
        // The one-band end is the centre, and it is exactly one band wide there.
        let centre = Feeling::parse("mad", RADIUS_RESOLVED as i64, 0).expect("the centre");
        assert_eq!(centre.sounding_bands(), 1);
        assert_eq!(centre.signature()[9], 1.0);
        assert!(centre.signature().iter().enumerate().all(|(band, w)| band == 9 || *w == 0.0));
    }

    /// The wheel's own worked example, second line: `hyped` — Joyful, a strong radius, **drift −3**,
    /// *"peak in the ultra-violet"*. On the ladder (yellow sits in band 7, so it can walk −7), and the
    /// excursion is the radius' own fraction of it: `round(-3 × 49151/65535) = -2`.
    #[test]
    fn a_strong_joyful_reading_walks_its_peak_by_the_radius_own_fraction_of_the_drift() {
        let feeling = Feeling::parse("joyful", RADIUS_STRONG as i64, -3).expect("on the ladder");
        assert_eq!(feeling.ladder(), (-7, 4));
        assert_eq!(feeling.excursion(), -2);
        assert_eq!(feeling.peak_band(), 5, "three bands short of yellow, at the strong radius");
        // At the rim the whole drift is walked, which is what "how many bands its peak walks at the
        // rim" says.
        let rim = Feeling::parse("joyful", RADIUS_RIM as i64, -3).expect("on the ladder");
        assert_eq!(rim.excursion(), -3);
        assert_eq!(rim.peak_band(), 4);
        assert_eq!(rim.sounding_bands(), 12, "the rim fills all twelve declared bands");
        assert!(rim.signature().iter().all(|weight| *weight > 0.0));
        // And the centre never leaves its own band, whatever the drift: the record's own sentence.
        let centre = Feeling::parse("joyful", RADIUS_RESOLVED as i64, -3).expect("on the ladder");
        assert_eq!(centre.excursion(), 0);
        assert_eq!(centre.peak_band(), 7);
        assert_eq!(centre.sounding_bands(), 1);
    }

    /// **The wheel's third worked example is refused by name.** `carrying-the-loss` — Sad at the
    /// extreme radius with drift −4 — is a reading whose peak *"walks out of the visible into the
    /// ultra-violet"*; this tree's declared table starts at 400 nm, purple's anchor sits in its
    /// **first** band, and there is no band below it to voice. The refusal is the honest answer.
    #[test]
    fn the_wheels_own_out_of_visible_example_is_refused_rather_than_voiced_a_band_it_does_not_have() {
        let refusal = Feeling::parse("sad", RADIUS_RIM as i64, -4).expect_err("no band below 400 nm");
        assert_eq!(refusal.kind(), RefusalKind::FeelingUncalibrated);
        assert!(refusal.detail().contains("0..=11"), "the ladder, quoted back: {refusal}");
        // The same reading walked the other way is on the ladder, and the ladder is the reading's own.
        let long = Feeling::parse("sad", RADIUS_RIM as i64, 4).expect("eleven bands of long walk");
        assert_eq!(long.ladder(), (0, 11));
        assert_eq!(long.peak_band(), 4);
    }

    /// The four refusals, each on its own subject, each by name.
    #[test]
    fn an_absent_a_blended_an_uncalibrated_and_an_undeclared_reading_are_each_refused_by_name() {
        let absent = Feeling::of(&[], 0, 0).expect_err("nothing was named");
        assert_eq!(absent.kind(), RefusalKind::FeelingAbsent);
        assert!(absent.detail().contains("no family was named"), "{absent}");

        let blended = Feeling::of(&[Family::Mad, Family::Joyful], RADIUS_PRESENT as i64, 0)
            .expect_err("two families compete");
        assert_eq!(blended.kind(), RefusalKind::FeelingContradictory);
        assert!(blended.detail().contains("mad, joyful"), "{blended}");

        let past_the_rim = Feeling::of(&[Family::Powerful], RADIUS_RIM as i64 + 1, 0)
            .expect_err("past the rim");
        assert_eq!(past_the_rim.kind(), RefusalKind::FeelingUncalibrated);
        assert!(past_the_rim.detail().contains("65535"), "{past_the_rim}");

        let negative = Feeling::of(&[Family::Sad], -1, 0).expect_err("before the centre");
        assert_eq!(negative.kind(), RefusalKind::FeelingUncalibrated);

        let off_the_ladder = Feeling::of(&[Family::Mad], RADIUS_RIM as i64, 3)
            .expect_err("band 9 has two bands of long walk");
        assert_eq!(off_the_ladder.kind(), RefusalKind::FeelingUncalibrated);

        let undeclared = Feeling::parse("furious", RADIUS_PRESENT as i64, 0).expect_err("not a family");
        assert_eq!(undeclared.kind(), RefusalKind::FeelingUndeclared);
        assert!(undeclared.detail().contains("mad, scared, joyful"), "{undeclared}");
    }

    /// The four named marks are the record's own numbers, and a reading at each of them keeps the two
    /// relations the record states: the intensity is the radius, and the width grows with it.
    #[test]
    fn the_named_marks_are_the_records_own_and_the_width_grows_with_the_radius() {
        assert_eq!(
            [RADIUS_RESOLVED, RADIUS_CALM, RADIUS_PRESENT, RADIUS_STRONG, RADIUS_RIM],
            [0, 13_107, 32_767, 49_151, 65_535]
        );
        let mut previous = 0.0;
        let mut previous_bands = 0;
        for radius in [RADIUS_RESOLVED, RADIUS_CALM, RADIUS_PRESENT, RADIUS_STRONG, RADIUS_RIM] {
            let feeling = Feeling::parse("peaceful", radius as i64, 0).expect("on the ladder");
            assert!(feeling.width() >= previous, "the width fell at {radius}");
            assert!(feeling.sounding_bands() >= previous_bands, "the bands fell at {radius}");
            assert!((feeling.signature().iter().sum::<f64>() - 1.0).abs() < 1e-12);
            previous = feeling.width();
            previous_bands = feeling.sounding_bands();
        }
        assert_eq!(previous, 12.0);
        assert_eq!(previous_bands, 12);
    }
}
