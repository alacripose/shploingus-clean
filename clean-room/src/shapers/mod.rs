//! **The stage bodies, grouped by the medium they act through.**
//!
//! Derived from `docs/SHAPERS.md` §1 (the fifteen shapers and their papers) and §3.1 (a stage obeys
//! one contract whatever it produces). The grouping is by **medium** — earth, water, air, life, the
//! record — rather than one file per stage, because this project prefers a small number of large,
//! well-named files over fifteen stubs (Constitution P6); each file carries the papers of the stages
//! it implements in its own header, so provenance stays per-module (P1).
//!
//! | File | Stages | Landed in this slice |
//! |---|---|---|
//! | [`terrain`] | `tectonics`, `fields`, `geology`, `terrain` | all four |
//! | [`water`] | `hydrology`, `substrate`, `erosion` | all three |
//! | [`air`] | `climate`, `atmosphere`, `weather` | all three |
//! | [`life`] | `ecology`, `caves` | `ecology`; `caves` **refuses by name** (its occupancy field is owed) |
//! | [`record`] | `delta`, `transition`, `earthworks` | none: all three **refuse by name** — two owed bodies and the record |
//!
//! Eleven of the fifteen stages produce values in this slice; four refuse by name, each naming the
//! input, the state or the design that is missing. That is the honest shape of a first slice, and
//! [`crate::process::roster`] is where the whole declaration lives.

pub mod air;
pub mod life;
pub mod record;
pub mod terrain;
pub mod water;
