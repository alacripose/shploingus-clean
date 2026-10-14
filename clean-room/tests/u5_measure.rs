//! **U5 proved through the public surface**: one crate, four modules, one world.
//!
//! The instruments read a stock and a boundary; the light solver reads a band table at the same
//! tick; the audio seam reads the same world's own field at the same place. Nothing here reaches
//! inside a module: every value comes back out of the API the binary and a client use.
//!
//! The bar this file holds is the plan's own (`docs/CLEAN-ROOM-PLAN-2026-10-11.md` §5 row 5 and its U5
//! block): `Sensor`/`Band`/`GaugeReading`/`GaugeWall`, `Solver`/`Spectra`, the audio seam, and **a
//! quantity that cannot be weighed is refused by name**.

use shploingus_clean_room::audio::{
    duty, onset, voice, BED_GAIN, LEVEL_CEILING, RAIN_GAIN, SAMPLE_RATE_HZ, VOICE_FRAMES,
    ONSET_CELL_M, ONSET_CELL_TICKS, ONSET_DRYNESS, ONSET_DUTY, ONSET_FLOOR, SILENT_ONSET, WET_ONSET,
};
use shploingus_clean_room::feeling::{
    Family, Feeling, RADIUS_PRESENT, RADIUS_RIM, RADIUS_STRONG,
};
use shploingus_clean_room::light::{
    night_sky_radiance, to_moon_lux, twilight_lux, Droplets, Solver, Spectra, State, CHANNEL_COUNT,
    FOG_THRESHOLD_LWC_G_M3, NIGHT_SKY_ILLUMINANCE_LX,
};
use shploingus_clean_room::material::{MaterialId, MaterialRegistry};
use shploingus_clean_room::measure::{Band, Crossing, GaugeWall, Sensor, Site, WeighingGate};
use shploingus_clean_room::noise::Noise;
use shploingus_clean_room::refusal::RefusalKind;

const QUARTZ: MaterialId = MaterialId(4);
const BASALT: MaterialId = MaterialId(3);
const WATER: MaterialId = MaterialId(5);
const TICK: u64 = 400;

fn stock() -> Site {
    Site::Stock([12, 64, -3])
}

fn band(min: u64, max: u64) -> Band {
    Band::of(min, max).expect("a band from a lower end to a higher one")
}

/// **A boundary audit, and its arithmetic is exact.** Three crossings of three materials, each
/// weighed through its own density; the three raw mass numbers add as rationals with no rounding at
/// any step, which is what *audited to the gram* means.
#[test]
fn the_gate_weighs_every_crossing_exactly_and_the_parts_add_to_the_whole() {
    let registry = MaterialRegistry::starter();
    let gate = WeighingGate::at([12, 65, -3]);
    let crossings = [
        (QUARTZ, 3u64, (7_950_000i64, 1i64)),
        (BASALT, 64, (192_000_000, 1)),
        (WATER, 1, (1_000_000, 1)),
    ];
    let mut total = (0i64, 1i64);
    for (material, count, expected) in crossings {
        let mass = gate
            .weigh(&Crossing { material, count }, &registry)
            .expect("an admitted material at a count above zero weighs");
        assert_eq!(mass.grams(), expected, "material {} at {count} units", material.0);
        total = (total.0 + mass.grams().0, total.1);
    }
    assert_eq!(total, (200_950_000, 1));
    // And the gate's own answer carries no count and no material name: `Mass` has room for grams.
    let again = gate
        .weigh(&Crossing { material: QUARTZ, count: 3 }, &registry)
        .unwrap();
    assert_eq!(again, gate.weigh(&Crossing { material: QUARTZ, count: 3 }, &registry).unwrap());
}

/// **One sensor concept, two consumers.** The wall judges a count against its band; the gate's own
/// answer is a mass, and the wall refuses it *as a mass* — the reading is the only difference between
/// the two consumers, which is what makes the concept one concept.
#[test]
fn the_wall_and_the_gate_are_two_consumers_of_one_reading() {
    let registry = MaterialRegistry::starter();
    let gate = WeighingGate::at([12, 65, -3]);
    let mut wall = GaugeWall::new();
    wall.watch(stock(), QUARTZ, band(32, 96));
    wall.watch(Site::Boundary(gate.boundary()), QUARTZ, band(0, u64::MAX));

    let counted = Sensor::count(stock(), QUARTZ, 64, TICK);
    let judged = wall.read(&counted).expect("the wall watches this item at this site");
    assert!(judged.is_satisfied());
    assert_eq!(judged.band(), band(32, 96));
    assert_eq!(judged.count_of(), Some(64));
    assert_eq!(judged.sensor().tick(), TICK);

    let mass = gate
        .weigh(&Crossing { material: QUARTZ, count: 64 }, &registry)
        .unwrap();
    let weighed = Sensor::mass(gate.boundary(), QUARTZ, mass, TICK);
    assert_eq!(weighed.count_of(), None, "a mass is not a count of anything");
    let refused = wall.read(&weighed).expect_err("a mass is not a band judgement");
    assert_eq!(refused.kind(), RefusalKind::NotACount);
    assert!(
        refused.detail().contains("a band is per item"),
        "the refusal carries the rule's own words: {refused}"
    );
    // The two readings are *different moments of one concept*, and the concept says so.
    assert!(!counted.same_moment(&weighed), "different sites are different moments");
    assert!(
        counted.same_moment(&Sensor::count(stock(), QUARTZ, 12, TICK)),
        "the same site and tick is the same moment, whatever the count says"
    );
}

/// **Every way a quantity can fail to be weighed or judged is refused in its own name**, and the
/// names round-trip through the closed vocabulary.
#[test]
fn every_refusal_is_named_and_the_vocabulary_round_trips() {
    let registry = MaterialRegistry::starter();
    let gate = WeighingGate::at([12, 65, -3]);
    let mut wall = GaugeWall::new();
    wall.watch(stock(), QUARTZ, band(1, 8));

    let cases: [(RefusalKind, String); 5] = [
        (
            RefusalKind::NotWeighable,
            gate.weigh(&Crossing { material: QUARTZ, count: 0 }, &registry)
                .expect_err("nothing crossed")
                .to_string(),
        ),
        (
            RefusalKind::UnadmittedMaterial,
            gate.weigh(&Crossing { material: MaterialId(9999), count: 1 }, &registry)
                .expect_err("no density to weigh it through")
                .to_string(),
        ),
        (
            RefusalKind::NotACount,
            wall.read(&Sensor::mass(
                stock().position(),
                QUARTZ,
                gate.weigh(&Crossing { material: QUARTZ, count: 2 }, &registry).unwrap(),
                TICK,
            ))
            .expect_err("a mass at a band")
            .to_string(),
        ),
        (
            RefusalKind::Unwatched,
            wall.read(&Sensor::count(Site::Stock([0, 0, 0]), QUARTZ, 2, TICK))
                .expect_err("no gauge watches this site")
                .to_string(),
        ),
        (
            RefusalKind::BandInverted,
            Band::of(9, 3).expect_err("a band with no inside").to_string(),
        ),
    ];
    for (kind, message) in cases {
        assert!(
            message.starts_with(kind.name()),
            "the message names its own kind first: `{message}` against `{}`",
            kind.name()
        );
        assert_eq!(RefusalKind::parse(kind.name()), Some(kind));
    }
    // The vocabulary is closed and every name in it round-trips — U5's own four included.
    assert_eq!(RefusalKind::ALL.len(), RefusalKind::COUNT);
    // **65 → 77 on 2026-10-12, and the reason is named rather than the number moved silently**: U7's slice
    // added the twelve kinds an axis or a ballot can refuse with (`axis_interchange_refused`,
    // `scalar_reduction_refused`, `bare_token_refused`, `uncovered_axis`, `fabricated_axis`,
    // `ballot_incomplete`, `floor_undeclared`, `oracle_unreachable`, `not_established`,
    // `option_uncalibrated`, `reason_absent`, `no_reading` — the last of which U6 had already declared).
    // **77 → 80 on 2026-10-12 (the shell pass), for the same reason stated the same way**: U8's slice
    // added the three ways the shell refuses — `constant_unadmitted` (a number that gets drawn and is in
    // none of `docs/UI-SPEC.md` §8's four buckets), `pane_unknown` (a pane operation naming a window the
    // recipe never mounted) and `split_floor_breached` (a split below the declared 40 × 40 floor).
    // **80 → 81 on 2026-10-12 (U9's document form, §28), once more for the same reason**: the write
    // half of a save document added the one way it refuses — `document_unwritable` (the bytes are a
    // save, and they could not be placed at the reference they are declared at).
    // **81 → 86 on 2026-10-12 (the policy chokepoint, §29), the same reason a fifth time**: L-5's four
    // properties and its readable-matrix rule each named one way the chokepoint refuses —
    // `policy_denied` (the decision function denied an act, quoting the list), `grant_unattributed`
    // (a grant or a caller with no author), `grant_unscoped` (a grant whose scope is absent, undeclared
    // or unbounded), `never_store` (a rejection rather than a permission: the material may be used and
    // never persisted) and `action_undescribed` (a matrix row a reader cannot understand).
    // **86 → 90 on 2026-10-12 (U5's voices, §30), the same reason a sixth time**: the feeling wheel's
    // own reading and the voice that reads it each named one way a state is refused rather than voiced
    // — `feeling_absent` (no reading at all), `feeling_undeclared` (a word the wheel does not declare),
    // `feeling_uncalibrated` (a radius past the rim, or a signed drift that walks the peak outside the
    // twelve declared bands — which is the wheel's own `carrying-the-loss`), and
    // `feeling_contradictory` (more than one family named at once, which the wheel refuses to blend).
    // **90 → 92 on 2026-10-13 (U12's first mob, §32), the same reason a seventh time**: the being's own
    // drive named one way the *world* refuses its need and one way its own record does — `water_unreachable`
    // (it walked its whole declared reach along the game's own flow receivers and found no cell where the
    // world's own filled surface stands above the ground) and `stage_below_floor` (its draft reads below the
    // floor its charter declares, so the act is filed rather than taken).
    // **92 → 94 on 2026-10-13 (U1's visible client, §36), the same reason an eighth time**: the client's
    // own frame named the one way a colour cannot be derived and the one way the machine refuses to show
    // one — `no_reflectance` (the column's admitted material carries no reflectance, so the pixel would be
    // a colour nobody recorded: C-39) and `platform_refused` (a window that could not be created, a client
    // area that is not the frame's own size, a blit that copied fewer scan lines than the frame has, or a
    // window interface this platform does not declare) — both refused rather than drawn.
    // **94 → 95 on 2026-10-13 (the faces over the cells, §39), the same reason a ninth time**: the
    // cells' own door named the one way a cell world refuses a request for geometry it does not hold —
    // `geometry_outside_the_world` (a chunk with no column under the world's own patch: the cells
    // declare where the world is, and a request with none of them under it is refused rather than
    // answered with an empty mesh, which would read as smooth ground).
    assert_eq!(RefusalKind::COUNT, 95);
    assert!(
        RefusalKind::parse("geometry_outside_the_world").is_some(),
        "the cells' door's refusal is part of the closed vocabulary"
    );
    for name in ["no_reflectance", "platform_refused"] {
        assert!(RefusalKind::parse(name).is_some(), "{name} is part of the closed vocabulary");
    }
    for name in ["water_unreachable", "stage_below_floor"] {
        assert!(RefusalKind::parse(name).is_some(), "{name} is part of the closed vocabulary");
    }
    for name in ["not_weighable", "not_a_count", "band_inverted", "unwatched"] {
        assert!(RefusalKind::parse(name).is_some(), "{name} is part of the closed vocabulary");
    }
    for name in ["constant_unadmitted", "pane_unknown", "split_floor_breached"] {
        assert!(RefusalKind::parse(name).is_some(), "{name} is part of the closed vocabulary");
    }
    assert!(
        RefusalKind::parse("document_unwritable").is_some(),
        "document_unwritable is part of the closed vocabulary"
    );
    for name in [
        "policy_denied",
        "grant_unattributed",
        "grant_unscoped",
        "never_store",
        "action_undescribed",
    ] {
        assert!(RefusalKind::parse(name).is_some(), "{name} is part of the closed vocabulary");
    }
    for name in [
        "feeling_absent",
        "feeling_undeclared",
        "feeling_uncalibrated",
        "feeling_contradictory",
    ] {
        assert!(RefusalKind::parse(name).is_some(), "{name} is part of the closed vocabulary");
    }
    for name in [
        "axis_interchange_refused",
        "scalar_reduction_refused",
        "bare_token_refused",
        "uncovered_axis",
        "fabricated_axis",
        "ballot_incomplete",
        "floor_undeclared",
        "oracle_unreachable",
        "not_established",
        "option_uncalibrated",
        "reason_absent",
    ] {
        assert!(RefusalKind::parse(name).is_some(), "{name} is part of the closed vocabulary");
    }
}

/// **The light, read at the same world and the same tick.** One band table; the Sun's own bands are
/// calibrated to the published solar constant, pinned as a literal; the day's sky is blue for the
/// extinction's own reason; the beam is exactly zero below the horizon and the night sky is the night
/// sky's own measured value; the Moon's light is the Sun's own, reflected, and lands where the
/// published figures put it.
#[test]
fn the_light_solver_reads_one_table_and_its_numbers_are_the_records_own_claims() {
    let spectra = Spectra::standard();
    let band_total: f64 = spectra.sun_bands().iter().sum();
    assert_eq!(spectra.sun_bands().len(), CHANNEL_COUNT);
    // The calibration's target, pinned as a **literal**: a moved `SOLAR_CONSTANT_W_M2` fails here by
    // name rather than scaling with the assertion.
    assert!(
        (band_total - 1361.0).abs() < 1e-9,
        "the twelve bands are calibrated to the published 1361 W/m²"
    );

    let noon = Solver::new(Spectra::standard(), State::clear(60.0, -30.0, 0.0));
    let sky = noon.sky_radiance(40.0);
    let colour = noon.spectra().response(&sky);
    assert!(colour[2] > colour[0], "a daylight sky is blue: {colour:?}");
    assert!(sky.iter().any(|band| *band > night_sky_radiance()), "a lit sky is above the floor");

    // The beam is not small below the horizon: it is zero — and the sky is not, which is the defect
    // this module exists to end.
    let down = Solver::new(Spectra::standard(), State::clear(-0.5, -30.0, 0.0));
    assert_eq!(down.direct_beam(), [0.0; CHANNEL_COUNT]);
    let night = Solver::new(Spectra::standard(), State::clear(-18.0, -30.0, 0.0));
    let night_sky: f64 = night.sky_radiance(40.0).iter().sum();
    assert!(
        night_sky >= night_sky_radiance() * CHANNEL_COUNT as f64,
        "a night is dim, and it is not an absence"
    );
    assert!(night_sky < night_sky_radiance() * CHANNEL_COUNT as f64 * 1.05);

    // Droplets: the same air, two geometries. No layer declared is no path at all.
    let fogged = Solver::new(
        Spectra::standard(),
        State {
            droplets: Some(Droplets::layer(FOG_THRESHOLD_LWC_G_M3, 0.0)),
            ..State::clear(60.0, -30.0, 0.0)
        },
    );
    assert_eq!(fogged.direct_beam(), noon.direct_beam());
    assert_eq!(fogged.sky_radiance(40.0), sky);
    let dense = Solver::new(
        Spectra::standard(),
        State {
            droplets: Some(Droplets::fog(FOG_THRESHOLD_LWC_G_M3)),
            ..State::clear(60.0, -30.0, 0.0)
        },
    );
    let dense_beam: f64 = dense.direct_beam().iter().sum();
    let dense_sky: f64 = dense.sky_radiance(40.0).iter().sum();
    assert!(dense_beam < band_total && dense_sky > sky.iter().sum::<f64>());

    // The twilight is the reference's own curve, and its floor is the night sky's own figure.
    assert_eq!(twilight_lux(0.0), 400.0);
    assert!((twilight_lux(-4.0) - 13.35).abs() < 0.01);
    assert_eq!(twilight_lux(-18.0), NIGHT_SKY_ILLUMINANCE_LX);
    assert!(twilight_lux(-12.0) > 0.01 && twilight_lux(-12.0) < 0.02);

    // The Moon: a reflectance, with the π its solid angle needs.
    let full = Solver::new(Spectra::standard(), State::clear(0.0, 90.0, 1.0));
    let moonlight: f64 = full.moonlight().iter().sum();
    assert!((2.5e-3..3.5e-3).contains(&moonlight), "the full Moon: {moonlight}");
    assert!((to_moon_lux(moonlight) - 0.3).abs() < 0.1, "the published full-moon figure");
    assert!(full.spectra().moon_solid_angle_sr() > 6.0e-5);
    assert!(
        (full.spectra().moon_solid_angle_sr() / std::f64::consts::PI - 2.0428e-5).abs() < 1e-8,
        "the legacy's own (R/d)², which is π too dim"
    );
}

/// **The audio seam at the same place and tick.** One read: the field's own value, the window it
/// belongs to, and the audio layer's reading of it — plus this build's own measured duty, pinned to
/// the figure it returns and reported against the record's own figure rather than fitted to it.
#[test]
fn the_audio_seam_answers_is_it_raining_right_now() {
    let noise = Noise::new(0x5EED_0A57, 3);
    let at = [12.0, -3.0];
    let weather = onset(&noise, TICK, at);
    assert_eq!(weather.window(), TICK / ONSET_CELL_TICKS);
    // The window is a box of 2 000 ticks: every tick in it reads the same weather, and the tick that
    // leaves it is the next sample.
    assert_eq!(onset(&noise, ONSET_CELL_TICKS - 1, at), weather, "one window, one value");
    assert_ne!(onset(&noise, ONSET_CELL_TICKS, at).onset(), weather.onset());
    assert_eq!(onset(&noise, TICK + ONSET_CELL_TICKS, at).window(), 1);

    // The window is a box in *ticks* and the cell one in *metres*: a place inside the same cell reads
    // the same weather, and a place one cell over does not.
    assert_eq!(onset(&noise, TICK, [ONSET_CELL_M - 1.0, at[1]]), weather, "the same cell, 899 m of it");
    assert_ne!(
        onset(&noise, TICK, [ONSET_CELL_M, at[1]]).onset(),
        weather.onset(),
        "the cell's own edge is where the weather changes"
    );

    // The reading never leaves its own two ends, whatever the field says.
    for step in 0..32 {
        let weather = onset(&noise, step * ONSET_CELL_TICKS, [step as f64 * ONSET_CELL_M, 0.0]);
        assert!((0.0..=1.0).contains(&weather.onset()));
        assert!((0.0..=1.0).contains(&weather.gain()));
        assert_eq!(weather.is_wet(), weather.onset() >= WET_ONSET);
        assert_eq!(weather.is_silent(), weather.onset() <= 0.05);
    }

    let measured = duty(&noise, 32, 8);
    println!(
        "U5 audio: this build's field duty {measured:.4} against the record's own measured figure \
         {ONSET_DUTY}"
    );
    // **Pinned, not banded**: this field is deterministic, so its own duty is reproducible and a
    // moved threshold or a changed field fails here by name. The record's own 0.0997 is the legacy's
    // field and stays quoted beside it; C-94's constants are read as values in the document check
    // below, which is what makes the comparison evidence rather than prose.
    assert!(
        (measured - 0.0482).abs() < 1e-4,
        "this field's own duty moved: {measured}"
    );
}

/// **The voices, through the public surface.** The wheel's own reading drives the bed's level and its
/// own spectral shape, the onset field's own gain drives the rain's, the buffer carries its own
/// measurements, and the bytes it produced are pinned by digest — so a moved band tone, a moved level,
/// a changed field or a changed mix fails here by name rather than sounding almost the same.
#[test]
fn the_voice_renders_a_bounded_buffer_that_the_wheel_and_the_weather_both_move() {
    let noise = Noise::new(0x5EED_0A57, 3);
    let weather = onset(&noise, 7_000, [1_234.0, -55.0]);
    let feeling = Feeling::parse("powerful", RADIUS_STRONG as i64, -1).expect("on the wheel's ladder");
    let voiced = voice(Some(&feeling), &weather, &noise, VOICE_FRAMES).expect("a declared reading");
    assert_eq!(voiced.frames(), VOICE_FRAMES);
    assert_eq!(voiced.channels(), 1, "one channel, and said out loud");
    assert!(
        (voiced.frames() as f64 / SAMPLE_RATE_HZ - 0.042_666_66).abs() < 1e-6,
        "2 048 frames at 48 kHz is 42.67 ms"
    );
    // The bed's level is the radius' own fraction of the bed's ceiling, and the rain's is the seam's
    // own gain — both read out of the values handed in rather than fitted to each other.
    assert_eq!(voiced.bed_level(), feeling.radius() as f64 / RADIUS_RIM as f64 * BED_GAIN);
    assert_eq!(voiced.rain_level(), weather.gain() * RAIN_GAIN);
    // The peak walks one band short of green's own band: a drift of −1 at a strong radius.
    assert_eq!(feeling.band(), 5);
    assert_eq!(feeling.excursion(), -1);
    assert_eq!(feeling.peak_band(), 4);
    let tones = voiced.tones();
    assert!(tones[11] > tones[0], "the tones rise with the band");
    assert_eq!(voiced.sounding_bands(), feeling.sounding_bands());
    assert_eq!(
        voiced.signature(),
        &feeling.signature(),
        "the bed is built from the reading's own signature and nothing else"
    );
    // **Bounded**, and measured rather than promised.
    assert!(voiced.peak() > 0.0 && voiced.peak() <= LEVEL_CEILING, "peak {}", voiced.peak());
    assert!(voiced.rms() < voiced.peak(), "{} against {}", voiced.rms(), voiced.peak());
    assert!((voiced.signature().iter().sum::<f64>() - 1.0).abs() < 1e-12);
    // The same inputs are the same buffer, and the digest is the one this build produces.
    let again = voice(Some(&feeling), &weather, &noise, VOICE_FRAMES).expect("a declared reading");
    assert_eq!(voiced.digest(), again.digest(), "no clock and no state");
    // **Pinned as a literal**, measured on 2026-10-12: the samples are a pure function of the reading,
    // the weather and the field, so a moved band tone (or level, or field) moves this digest and the
    // test fails by name rather than passing on a buffer that sounds almost the same.
    assert_eq!(
        voiced.digest().to_hex(),
        "a33ede013edf0e2f97acfe33bece2af5a9b78d9583ba89972a79c33ed1f754a2"
    );
    // And a different reading is a different buffer: the wheel reaches the samples.
    let other = Feeling::parse("powerful", RADIUS_STRONG as i64, 1).expect("on the wheel's ladder");
    let drifted = voice(Some(&other), &weather, &noise, VOICE_FRAMES).expect("a declared reading");
    assert_ne!(drifted.digest(), voiced.digest());
    assert_eq!(other.peak_band(), 6, "the other direction walks the peak the other way");
}

/// **A feeling the record cannot voice is refused by name rather than voiced as its nearest
/// neighbour** — and the wheel's own worked example is one of the four.
#[test]
fn a_feeling_that_was_never_declared_is_refused_by_name_rather_than_voiced() {
    let noise = Noise::new(0x5EED_0A57, 3);
    let weather = onset(&noise, 7_000, [1_234.0, -55.0]);
    let absent = voice(None, &weather, &noise, VOICE_FRAMES).expect_err("no reading at all");
    assert_eq!(absent.kind(), RefusalKind::FeelingAbsent);
    assert!(absent.detail().contains("never declared"), "{absent}");
    let undeclared = Feeling::parse("furious", RADIUS_PRESENT as i64, 0).expect_err("a tertiary word");
    assert_eq!(undeclared.kind(), RefusalKind::FeelingUndeclared);
    // **The wheel's own example.** `carrying-the-loss` — Sad at the extreme radius with drift −4 — is a
    // reading whose peak *"walks out of the visible into the ultra-violet"*; the declared band table
    // starts at 400 nm and purple's anchor is inside its **first** band, so there is no band below it to
    // voice. The refusal is the honest answer and it quotes the ladder it does have.
    let uncalibrated = Feeling::parse("sad", RADIUS_RIM as i64, -4).expect_err("no band below 400 nm");
    assert_eq!(uncalibrated.kind(), RefusalKind::FeelingUncalibrated);
    assert!(uncalibrated.detail().contains("0..=11"), "{uncalibrated}");
    let contradictory = Feeling::of(&[Family::Mad, Family::Joyful], RADIUS_PRESENT as i64, 0)
        .expect_err("two families at once");
    assert_eq!(contradictory.kind(), RefusalKind::FeelingContradictory);
    assert!(contradictory.detail().contains("mad, joyful"), "{contradictory}");
    // The same reading walked the other way **is** voiced, so the ladder is the reading's own and not a
    // blanket refusal of drift.
    let long = Feeling::parse("sad", RADIUS_RIM as i64, 4).expect("eleven bands of long walk");
    let voiced = voice(Some(&long), &weather, &noise, VOICE_FRAMES).expect("a declared reading");
    assert_eq!(voiced.sounding_bands(), 12, "the rim fills all twelve declared bands");
    // And a period of zero seconds is its own refusal rather than a silent buffer.
    let empty = voice(Some(&long), &weather, &noise, 0).expect_err("no frames");
    assert_eq!(empty.kind(), RefusalKind::ZeroPeriod);
}

/// **The four documents the modules were derived from are on the disk, and each module names its
/// own.** A module whose source document has moved is a module whose provenance is a claim.
#[test]
fn the_derivation_documents_are_where_the_modules_say_they_are() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    for document in [
        "docs/DESIGN-SPEC.md",
        "docs/light-solver.md",
        "docs/missing_atmosphere_deepseek.md",
        "docs/LEDGER.md",
        "docs/CLEAN-ROOM-PLAN-2026-10-11.md",
    ] {
        let path = root.join(document);
        assert!(path.is_file(), "the derivation document is not there: {document}");
        let text = std::fs::read_to_string(&path).expect("the document reads");
        assert!(!text.trim().is_empty(), "the document carries nothing: {document}");
    }
    // **The record's own constants are read as values, not names.** Each row of C-94's table carries
    // the constant between pipes, and the number in that row must be this build's own number — a
    // moved threshold (on either side) fails here by name rather than passing on a name match.
    let ledger = std::fs::read_to_string(root.join("docs/LEDGER.md")).expect("the ledger reads");
    let squash = |text: &str| -> String {
        text.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '.').collect()
    };
    for (name, value) in [
        ("ONSET_CELL_TICKS", ONSET_CELL_TICKS.to_string()),
        ("ONSET_CELL_M", ONSET_CELL_M.to_string()),
        ("ONSET_DRYNESS", ONSET_DRYNESS.to_string()),
        ("ONSET_FLOOR", ONSET_FLOOR.to_string()),
        ("WET_ONSET", WET_ONSET.to_string()),
        ("SILENT_ONSET", SILENT_ONSET.to_string()),
        ("ONSET_DUTY", ONSET_DUTY.to_string()),
    ] {
        let row = ledger
            .lines()
            .find(|line| line.contains(&format!("| `{name}` |")))
            .unwrap_or_else(|| {
                panic!("C-94's table no longer carries {name}: the derivation is a claim")
            });
        assert!(
            squash(row).contains(&squash(&value)),
            "C-94's row for {name} no longer reads {value}: the record and the code disagree: {row}"
        );
    }
    let design = std::fs::read_to_string(root.join("docs/DESIGN-SPEC.md")).expect("the spec reads");
    for slot in ["S-03", "S-12", "S-13"] {
        assert!(design.contains(slot), "the design spec names {slot}");
    }
}
