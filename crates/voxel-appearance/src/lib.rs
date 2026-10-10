//! What the world **looks like**: OpenPBR inputs derived from block truth, the
//! Valve Source 2 stylisation applied over them, and the 256× detail lattice
//! evaluated on demand.
//!
//! This is the crate `docs/APPEARANCE.md` describes. That document is the
//! **user's** — appearance is the one area of this repository they reserved — and
//! this crate is its engineering. Where the document names a contract, the module
//! that implements it is named here:
//!
//! | `APPEARANCE.md` | Module |
//! |---|---|
//! | §1.1 the twelve OpenPBR parameters | [`openpbr`] |
//! | §1.2 the four Source 2 terms, applied *over* the BSDF | [`npr`] |
//! | §2 the 4×4×4 blend and the 256× lattice | [`composition`], [`detail`] |
//! | §3 the ≥3-luminance-band bar | [`response`] |
//! | §3 the four-bucket rule for constants | [`provenance`] |
//! | §4 appearance derived from block truth, plus variation | [`openpbr::from_composition`], [`openpbr::OpenPbrInputs::with_detail`] |
//!
//! # The two interpretations the user confirmed
//!
//! `LEDGER.md` C-2 and C-3 were marked **OPEN TO CORRECTION** because they are
//! inside the reserved area. Both were put to the user and both were confirmed on
//! 2026-09-29 (`LEDGER.md` C-27), so the seams that made them correctable are
//! still seams but they are no longer guesses:
//!
//! * **C-2** — the 256× detail is **addressed, never stored**. The arithmetic that
//!   forces it is in [`detail`], and it is pinned by a test. Its prose figure in
//!   `APPEARANCE.md` §2 was off by 64×, which is corrected and recorded.
//! * **C-3** — NPR is **Valve's Source 2 model over OpenPBR**, and the bar's
//!   *≥3 distinct luminance bands* is measured rather than asserted.
//!
//! # The third thing the user ruled on, and it is the one that costs
//!
//! Asked where OpenPBR's *values* come from — no record here contains any — the
//! user ruled: **derive from the material's own physics, refuse the rest.** So
//! this crate draws roughness from a material's own density, transmission from
//! its own optical class, wetness from its own hydrology adjacency, and it
//! **refuses** `base_color`, `specular_ior`, `specular_weight`, `coat_roughness`,
//! `transmission_depth` and `subsurface_weight` rather than defaulting them.
//!
//! The consequence is worth stating plainly: **no material in the admitted set
//! can pass the bar yet**, because none has an admitted albedo. That is not a
//! bug in this crate. It turns "appearance is blocked" into a **finite, named
//! debt of six parameters**, listed per material by
//! [`openpbr::OpenPbrInputs::unavailable`] — and the shading path those six feed
//! is tested, on stated inputs, so the day an admitted row exists the bar is
//! already measurable.
//!
//! # The one rule, here
//!
//! **Refuse, never default.** An unadmitted material is refused rather than
//! blended as grey; an `OpenPbrInputs` value outside OpenPBR's own declared range
//! is refused rather than clamped; a detail address outside the lattice is
//! refused rather than wrapped; a measurement over zero views is refused rather
//! than defaulted to one. And a parameter the model does not record is **absent**,
//! with a name — never zero, because zero is a value somebody could believe.

#![forbid(unsafe_code)]

pub mod composition;
pub mod detail;
pub mod fluid;
pub mod npr;
pub mod openpbr;
pub mod provenance;
pub mod response;
pub mod sky;
pub mod tonemap;
pub mod wrap;

pub use composition::VoxelComposition;
pub use detail::{DetailLattice, DetailSample};
pub use fluid::{reflected_ray, schlick, surface_radiance, WATER_F0, WATER_IOR};
pub use npr::{shade, Light, LightRig, Shaded, SurfaceFrame, Terms};
pub use openpbr::{
    blend, derive, from_composition, Availability, Colour, Emission, LinearRgb, OpenPbrInputs,
    Parameter,
};
pub use provenance::{Provenance, CONSTANTS};
pub use response::{measure, shading_response, ShadingResponse, BAR_BANDS};
pub use tonemap::{tone_map, tone_map_luminance};
pub use wrap::{
    is_flat, pillar_height, pillar_tangency, wrapped_normal, wrapped_normal_with_detail,
    INSCRIBED_RADIUS,
};
