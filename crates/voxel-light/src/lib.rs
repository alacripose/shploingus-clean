#![forbid(unsafe_code)]

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
//! **Not yet here:** the solver, the field, the optical properties, the geometry, and the
//! quantizers that hand light to the simulation. The ported tree has them in `solver.rs`,
//! `field.rs`, `optics.rs`, `geometry.rs` and `quantize.rs`, and they arrive as separate
//! checkpoints rather than as one landing, because each of them is a thing that can be
//! wrong on its own.
//!
//! # The dependency rule for this crate
//!
//! Nothing but `sha2`, and not even that yet. A solver that reaches for a linear-algebra
//! crate before it has a measured reason to is a solver whose units stopped being
//! checkable by reading it.

pub mod spectra;

pub use spectra::{
    bands, clear_sky, planck_w_per_m2_per_m, Band, BandTable, SpectraError, Transport,
    BAND_COUNT, SCALE_HEIGHT_M, SOLAR_CONSTANT, SPECTRA_VERSION, SUN_DILUTION, SUN_TEMPERATURE_K,
};
