//! The terrain shapers.
//!
//! Every shaper here is developed from a **named source** — a paper, or a ruling
//! recorded in this project — and every one of them is a pure function of
//! `(seed, tick, declared period)` and the stages above it. `docs/SHAPERS.md` is
//! the contract; this crate is the implementation of it.
//!
//! # The stage graph
//!
//! A shaper never calls another shaper *upward*. It declares what it **reads**,
//! and [`stages::Declared`] refuses a pipeline that does not provide it. That is
//! why [`geology::stratum_at`] takes the surface height as an **argument** rather
//! than calling [`terrain::elevation_m`]: terrain runs *after* geology in the
//! graph, and a call upward would be the cycle the graph exists to make
//! impossible.
//!
//! # The rules every shaper obeys
//!
//! 1. **Determinism.** No clock, no thread order, no RNG state. Worker count is a
//!    performance knob and never changes output (F12).
//! 2. **Refuse, never default.** An absent stage, an out-of-range index and an
//!    impossible configuration are all refusals with names.
//! 3. **Stage-coupled, not independent noise** (WG-Q5). A field is a function of
//!    the field above it, never a second unrelated fractal.
//! 4. **Rebuild reproduces truth with digests** (WG-Q4).
//!
//! # The shapers, and their papers
//!
//! | Module | Source |
//! |---|---|
//! | [`tectonics`] | Kreischer & Bommes 2019; Wilson-cycle boundary taxonomy |
//! | [`fields`] | F4's 20-level ladder; Shepard 1968 |
//! | [`geology`] | Allen & Allen 2013 (basin analysis); S-22 for the deposit dials |
//! | [`terrain`] | Mandelbrot 1982; Musgrave, Kolb & Mace 1989 |
//! | [`hydrology`] | O'Callaghan & Mark 1984 (D8); Barnes, Lehman & Mulla 2014; Strahler 1952 |
//! | [`erosion`] | Musgrave 1989 (thermal); Hansson 2017 (hydraulic); Mei, Decaudin & Hu 2007 |
//! | [`climate`] | Whittaker 1960; Köppen 1884; S-21 seasonal clock |
//! | [`atmosphere`] | US Standard Atmosphere 1976 |
//! | [`sky`] | Meeus 1998 (solar position); Kasten & Young 1989 (air mass); USNO twilight definitions |
//! | [`weather`] | Smith & Barstad 2004 (orographic precipitation) |
//! | [`caves`] | Ford & Williams 2007 (karst) |
//! | [`substrate`] | Heimsath, Dietrich, Nishiizumi & Finkel 1997 |
//! | [`ecology`] | Holdridge 1947; Reynolds 1987 (boids) |
//! | [`delta`] | Shields 1936; Hjulström 1935; Gilbert 1885; Slingerland & Smith 2004 |
//! | [`transition`] | Shepard 1968 |
//! | [`earthworks`] | F10's conservation ruling |
//! | [`column`] | composes all of the above |
//! | [`voxel`] | the **voxel shapers**: `slab` (physical), `loop-cut` and `pillar` (material) |

#![forbid(unsafe_code)]

pub mod atmosphere;
pub mod caves;
pub mod climate;
pub mod column;
pub mod delta;
pub mod earthworks;
pub mod ecology;
pub mod erosion;
pub mod fields;
pub mod geology;
pub mod hydrology;
pub mod spec;
pub mod stages;
pub mod sky;
pub mod substrate;
pub mod tectonics;
pub mod terrain;
pub mod transition;
pub mod voxel;
pub mod water;
pub mod weather;

pub use column::{derive as derive_column, Column};
pub use spec::{Config, WorldSpec};
pub use stages::{Declared, Stage, ALL as ALL_STAGES, STAGE_COUNT};
