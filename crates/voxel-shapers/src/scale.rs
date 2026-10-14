//! The world's **scale**: how much real world one voxel stands for.
//!
//! **Source.** The user's ruling of 2026-10-10: the engine's voxel is one metre, but the
//! systems it carries are modelled **eight times smaller than their real-world
//! counterparts**, so a planetary atmosphere has an altitude the world can actually hold.
//! One voxel therefore stands for [`REAL_METRES_PER_VOXEL`] real metres.
//!
//! # Why one constant, and not an eight in every module
//!
//! Before this module the tree held three unrelated scales at once: [`crate::atmosphere`]'s
//! scale height was the published 8 500 real metres, [`crate::spec::Config`]'s `relief_m` was
//! a tuned 40, and `continent_frequency` gave 400 m continents. Each number was defensible
//! alone and the *set* was a world nobody declared — which is why an altitude meant nothing
//! physical, and why the elevation ladder was inert. A real-Earth length is now written as
//! its published value passed through [`voxels`], and the eight lives here only.
//!
//! # The two directions, and which a quantity takes
//!
//! * A **published real length** (a scale height, a tropopause, an orographic lift) maps to
//!   engine metres with [`voxels`] — the number gets *smaller*, which is what "eight times
//!   smaller" means and what lets the atmosphere fit.
//! * A **published real gradient** (the lapse rate, °C per real metre) maps with
//!   [`per_voxel`] — because the column is now eight times shorter in engine metres, the
//!   gradient must be eight times *larger* for the same temperature drop over the same
//!   *real* height. Taking the wrong direction here is the one error that would look fine
//!   and be silently wrong, so both directions have a named function and a test.
//!
//! # What is deliberately *not* here
//!
//! Time. A tick is still a tick and a season is still a season; the ruling moved geometry,
//! and a second guess at the clock would be a second ruling nobody made. Wind speed — a
//! length per time — is therefore left alone until a measurement says it moves moisture
//! wrongly.

/// Real-world metres that one voxel stands for. **Declared**, and the only eight in the tree.
pub const REAL_METRES_PER_VOXEL: f64 = 8.0;

/// An engine length (metres, which are voxels) for a published **real-world** length.
///
/// The real length gets smaller: 8 500 real metres of scale height is 1 062.5 engine metres.
pub const fn voxels(real_m: f64) -> f64 {
    real_m / REAL_METRES_PER_VOXEL
}

/// The real-world length an engine length stands for — the inverse of [`voxels`].
pub const fn real_m(engine_m: f64) -> f64 {
    engine_m * REAL_METRES_PER_VOXEL
}

/// A published **real-world gradient** (a quantity per real metre) as a per-engine-metre one.
///
/// The column is eight times shorter in engine metres, so the gradient is eight times larger
/// for the same change over the same real height: the US Standard lapse of 0.0065 °C per real
/// metre is 0.052 °C per engine metre.
pub const fn per_voxel(real_per_m: f64) -> f64 {
    real_per_m * REAL_METRES_PER_VOXEL
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_two_length_directions_are_inverses() {
        for real in [1.0, 8.0, 8_500.0, 11_000.0] {
            let round_trip = real_m(voxels(real));
            assert!(
                (round_trip - real).abs() < 1e-9,
                "{real} round-tripped to {round_trip}"
            );
        }
    }

    #[test]
    fn a_real_length_gets_smaller_and_a_real_gradient_gets_larger() {
        // The two directions, pinned so neither can be flipped by a later hand. A scale
        // height shrinks; a lapse rate grows. Getting these the same way round is the
        // mistake that would pass every other test in the crate.
        assert!(voxels(8_500.0) < 8_500.0);
        assert!(per_voxel(0.006_5) > 0.006_5);
    }

    #[test]
    fn the_published_anchors_land_where_the_ruling_says() {
        // The numbers this tree actually uses, in engine metres, stated once.
        assert!((voxels(8_500.0) - 1_062.5).abs() < 1e-9, "scale height");
        assert!((voxels(11_000.0) - 1_375.0).abs() < 1e-9, "tropopause");
        assert!((voxels(8_000.0) - 1_000.0).abs() < 1e-9, "orographic lift");
        assert!((per_voxel(0.006_5) - 0.052).abs() < 1e-12, "lapse rate");
    }

    #[test]
    fn a_temperature_drop_survives_the_scale_change() {
        // The property the whole ruling is for: the same *real* column has the same
        // temperature drop whether it is measured in real metres or in engine metres.
        let real_drop = 0.006_5 * 8_500.0;
        let engine_drop = per_voxel(0.006_5) * voxels(8_500.0);
        assert!(
            (real_drop - engine_drop).abs() < 1e-9,
            "the scaled column dropped {engine_drop} °C against the real {real_drop} °C"
        );
    }
}
