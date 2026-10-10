//! The sampled appearance: the whole spectra, the join, and the declared
//! derivation (C-140/C-141).

use voxel_model::materials::spectra;
use voxel_model::Ratio;

/// Recompute the declared derivation independently of the generator: the mean
/// of the sample's own channels whose wavelength lies in `[low, high)`, with
/// missing channels skipped — as an exact rational.
fn window_mean(sample: &spectra::Spectrum, low: Ratio, high: Ratio) -> Ratio {
    let wavelength_scale = 10u64.pow(sample.wavelength_scale);
    let mut sum: u64 = 0;
    let mut count: u64 = 0;
    for (wavelength, value) in sample.wavelengths.iter().zip(sample.values.iter()) {
        if *value == i64::MIN {
            continue;
        }
        assert!(*value >= 0, "{}: a present channel is negative", sample.id);
        let lambda = Ratio::from_parts(*wavelength, wavelength_scale);
        if low.is_greater_than(lambda) || !high.is_greater_than(lambda) {
            continue;
        }
        sum = sum
            .checked_add(*value as u64)
            .expect("the window sum fits in a u64");
        count += 1;
    }
    assert!(count > 0, "{}: a window with no channels", sample.id);
    Ratio::from_parts(sum, count * 10u64.pow(sample.value_scale))
}

#[test]
fn every_sample_joins_its_spectrum_to_its_instrument_grid() {
    for id in [
        "seawater_open_ocean",
        "limestone_cu02",
        "actinolite_hornfels",
        "basalt_fresh",
        "magnetite_skarn",
        "stonewall_playa_dry_mud",
        "sand_dwo3",
    ] {
        assert!(spectra::by_id(id).is_some(), "{id} must be declared");
    }
    for sample in spectra::SAMPLES {
        assert_eq!(sample.values.len(), sample.wavelengths.len(), "{}", sample.id);
        assert!(sample.wavelength_scale > 0, "{}", sample.id);
        assert!(sample.value_scale > 0, "{}", sample.id);
        assert!(
            sample.usgs_record.chars().all(|digit| digit.is_ascii_digit()),
            "{}",
            sample.id
        );
        assert!(sample.source_url.starts_with("https://"), "{}", sample.id);
    }
}

#[test]
fn the_visible_channels_are_exact_means_of_the_retained_spectrum() {
    let [b, g, r] = spectra::VISIBLE_WINDOWS;
    for sample in spectra::SAMPLES {
        assert_eq!(sample.visible.0, window_mean(sample, r.0, r.1), "{} r", sample.id);
        assert_eq!(sample.visible.1, window_mean(sample, g.0, g.1), "{} g", sample.id);
        assert_eq!(sample.visible.2, window_mean(sample, b.0, b.1), "{} b", sample.id);
    }
}

#[test]
fn the_sentinel_is_never_turned_into_a_number() {
    let basalt = spectra::by_id("basalt_fresh").expect("basalt_fresh is declared");
    assert_eq!(&basalt.values[..5], &[i64::MIN; 5]);
    assert_eq!(
        basalt
            .values
            .iter()
            .filter(|value| **value == i64::MIN)
            .count(),
        27
    );
    let limestone = spectra::by_id("limestone_cu02").expect("limestone_cu02 is declared");
    assert!(limestone.values.iter().all(|value| *value != i64::MIN));
}

#[test]
fn every_reflectance_constructs_within_unity_and_keeps_its_source() {
    for sample in spectra::SAMPLES {
        let sourced = spectra::reflectance_of(sample).expect("a reflectance at most 1");
        assert!(
            sourced.source.contains("USGS Spectral Library v7"),
            "{}",
            sample.id
        );
        assert!(sourced.value.r.to_f64() <= 1.0);
        assert!(sourced.value.g.to_f64() <= 1.0);
        assert!(sourced.value.b.to_f64() <= 1.0);
    }
}

#[test]
fn the_open_ocean_is_chromatic_and_the_stand_in_is_flagged() {
    let seawater = spectra::by_id("seawater_open_ocean").expect("declared");
    assert_ne!(
        seawater.visible.0, seawater.visible.2,
        "the ocean cannot come back achromatic (C-140)"
    );
    let playa = spectra::by_id("stonewall_playa_dry_mud").expect("declared");
    assert!(playa.note.contains("STAND-IN"), "note was {:?}", playa.note);
}
