#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! **The light field: full-spectrum radiometry over declared bands.**
//!
//! # What this crate is for
//!
//! This world is lit by a *photometric* sky (`voxel_shapers::sky`) and a *chromaticity*
//! (`skylight::chromaticity(sky.sun)`), and neither of them is a sun. Measured over a
//! year at the shipped time of day, that chromaticity is red-dominant on **every sampled
//! day**. A chromaticity has a shape and no magnitude; the shape is whatever the
//! atmosphere did to a number that was never split into wavelengths.
//!
//! This crate is the alternative, ported from `planet-shploingus`'s `planet-light`: light
//! is **float, full-spectrum radiometry** over a **small declared set of bands**, the sun
//! is a **real body** whose spectrum is Planck at its effective temperature, and each band
//! declares **how it travels** rather than only how bright it is.
//!
//! # Why this is a separate crate
//!
//! Because the integer simulation must not depend on it. The coupling runs **one way**:
//! light produces quantized joules and quantized band buckets, and the simulation reads
//! those. Nothing in the world model links against a float solver, and keeping the solver
//! behind a crate boundary is what makes that a compile-time fact rather than a rule
//! someone remembers.
//!
//! # What is checked in so far
//!
//! [`spectra`] — the band table and the real sun. Twelve bands from X-ray to radio,
//! Planck at 5778 K, Rayleigh/aerosol/ozone/water extinction, a lunar reflectance curve,
//! and transport classes, with the canonical byte encoding and its refusals.
//!
//! [`solver`] — **the transport**: the bands crossed with the air, the fog and the cloud, so the
//! sky is measured rather than drawn and a night is dim rather than absent.
//!
//! **Not yet here:** the field, the geometry, and the quantizers that hand light to the
//! simulation. The ported tree has them in `field.rs`, `optics.rs`, `geometry.rs` and
//! `quantize.rs`, and they arrive as separate checkpoints rather than as one landing, because each
//! of them is a thing that can be wrong on its own.
//!
//! # The dependency rule for this crate
//!
//! Nothing but `sha2`, and not even that yet. A solver that reaches for a linear-algebra
//! crate before it has a measured reason to is a solver whose units stopped being
//! checkable by reading it.
//!
//! **The audit, 2026-10-10.** 906 lines, 2 modules, 30 public items, 17 tests. Read by **one** crate:
//! `voxel-client`, which takes `SUN_DILUTION` and nothing else. So a crate whose stated purpose is to
//! *replace* the chromaticity path the game lights itself with is, in the game, a single constant.
//! The full-spectrum band table — `SCALE_HEIGHT_M`, `SOLAR_CONSTANT`, `SUN_TEMPERATURE_K`,
//! `examples/bands.rs` — has no consumer at all. **Owed:** either the sky reads the band table, or
//! this crate says on the record that it is a measurement waiting for a renderer. Today the game
//! lights itself the way this crate's header argues against, and no instrument says so.

pub mod solver;
pub mod spectra;

pub use solver::{
    air_mass, cloud_extinction_per_m, direct_beam, droplet_extinction_per_m, mixed_phase,
    CLOUD_EXTINCTION_PER_M,
    moon_radiance, moon_sky_power, moonlight, night_sky_radiance_w_m2_sr, path_in_layer,
    phase_weighted_scatter, rgb_response, rgb_weights, sky_radiance, sun_visible_fraction,
    transmittance, twilight_fraction, visible_rgb, Air, MoonPath, SolverError, SunPath,
    ASTRONOMICAL_TWILIGHT_DEG, DROPLET_ASYMMETRY_G, DROPLET_SINGLE_SCATTER_ALBEDO,
    GRAZING_AIR_MASS, LUMENS_PER_WATT_AT_555NM, MIE_ASYMMETRY_G, MOON_ANGULAR_RADIUS_RAD,
    MOON_DISTANCE_KM, MOON_RADIUS_KM, MOON_SOLID_ANGLE_SR, NIGHT_SKY_ILLUMINANCE_LX,
    SUN_ANGULAR_RADIUS_RAD, TWILIGHT_DECAY_PER_DEG,
};
pub use spectra::{
    aerosol_optical_depth, bands, clear_sky, planck_w_per_m2_per_m, rayleigh_optical_depth, Band,
    BandTable, SpectraError, Transport, BAND_COUNT, SCALE_HEIGHT_M, SOLAR_CONSTANT,
    SPECTRA_VERSION, SUN_DILUTION, SUN_TEMPERATURE_K,
};
