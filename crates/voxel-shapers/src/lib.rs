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
//! | [`scale`] | the user's 1:8 ruling of 2026-10-10 — one voxel stands for eight real metres |
//! | [`tectonics`] | Kreischer & Bommes 2019; Wilson-cycle boundary taxonomy |
//! | [`fields`] | F4's 20-level ladder; Shepard 1968 |
//! | [`geology`] | Allen & Allen 2013 (basin analysis); S-22 for the deposit dials |
//! | [`terrain`] | Mandelbrot 1982; Musgrave, Kolb & Mace 1989 |
//! | [`hydrology`] | O'Callaghan & Mark 1984 (D8); Barnes, Lehman & Mulla 2014; Strahler 1952 |
//! | [`erosion`] | Musgrave 1989 (thermal); Hansson 2017 (hydraulic); Mei, Decaudin & Hu 2007 |
//! | [`climate`] | Whittaker 1960; Köppen 1884; S-21 seasonal clock; `earth_biomes_subbiomes_continentalness_mountains.pdf` (continentalness) |
//! | [`atmosphere`] | US Standard Atmosphere 1976 |
//! | [`sky`] | Meeus 1998 (solar position); Kasten & Young 1989 (air mass); USNO twilight definitions |
//! | [`weather`] | Smith & Barstad 2004 (orographic precipitation) |
//! | [`clouds`] | `clouds_and_atmospheric_formation.pdf`, `cloud_types_fronts_and_altitudes_guide.pdf`; Magnus–Tetens (Tetens 1930; Alduchov & Eskridge 1996); `fog_mist_foam_and_aerosol_dispersion.pdf` for the near-surface systems (C-184) |
//! | [`fog`] | `fog_mist_foam_and_aerosol_dispersion.pdf` — the near-surface droplet field; Koschmieder's `V ≈ 3.912/β`; the cloud parcel and the weather's own wind, not a second pair |
//! | [`caves`] | Ford & Williams 2007 (karst) |
//! | [`substrate`] | Heimsath, Dietrich, Nishiizumi & Finkel 1997 |
//! | [`ecology`] | Holdridge 1947; Reynolds 1987 (boids); `earth_biomes_subbiomes_terrain.pdf`, `earth_biomes_subbiomes_continentalness_mountains.pdf` |
//! | [`biome_library`] | the same two atlases, as the `display-packages/biomes` catalog: their §5 systems, §15 catalog and §15b overlays, with the envelope bands Holdridge gives |
//! | [`habitat`] | `earth_biomes_subbiomes_terrain.pdf`; `earth_biomes_subbiomes_continentalness_mountains.pdf` (§3, §15, §16) |
//! | [`delta`] | Shields 1936; Hjulström 1935; Gilbert 1885; Slingerland & Smith 2004 |
//! | [`transition`] | Shepard 1968 |
//! | [`earthworks`] | F10's conservation ruling |
//! | [`materials`] | the admitted rows, their 256-level shapers, and the fluid/solid predicates |
//! | [`voxel`] | the **voxel shapers**: `slab` (physical), `loop-cut` and `pillar` (material) |
//!
//! **The audit, 2026-10-10 — the one that explains "nothing happens in the world".** 25 050 lines
//! over 31 modules, 687 public items, sixteen stages on a four-rung ladder. At runtime the only stage
//! values the game consults are `Weather` and `Climate`, and their output is the sky's `SkyInputs`.
//! The only function in the workspace that mutates the world is `voxel_stream::Streamer::edit`, which
//! is the player's dig. **Stages produce values and nothing converts a stage's answer into an edit**,
//! so the ladder is a schedule with no drivetrain — and `fluidics`, which owns a 256-level lattice and
//! declares its own cadence, is read by exactly one thing: `clouds.rs`, through `FillLevel`. **Owed:
//! one stage → edit path.** It is a single interface, and it is what makes sixteen existing stages
//! real rather than scheduled.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod atmosphere;
pub mod biome_library;
pub mod caves;
pub mod chunk_fields;
pub mod climate;
pub mod clouds;
pub mod corpus;
pub mod delta;
pub mod earthworks;
pub mod ecology;
pub mod erosion;
pub mod field_sampler;
pub mod fields;
pub mod flora;
pub mod fluid_state;
pub mod fluidics;
pub mod fog;
pub mod geology;
pub mod habitat;
pub mod hydrology;
pub mod materials;
pub mod process;
pub mod scale;
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

pub use chunk_fields::{material_of, material_at as field_material_at, ChunkFieldSource, FieldQuery, FieldSource};
pub use field_sampler::{
    ChunkFieldSampler, DraftChunkFieldSampler, DraftFieldSample, DraftFieldSampler, DraftFieldSpec,
    FieldId, FieldResolution, FieldSample, FieldSampler, FieldSamplerIdentity, FieldSpec, FieldValue,
    FixedValue, ResolutionLevel, SamplingPolicy, SeamPolicy, Unit, VerticalDependence,
};
pub use spec::{Config, WorldSpec};
pub use stages::{Declared, Stage, ALL as ALL_STAGES, STAGE_COUNT};
