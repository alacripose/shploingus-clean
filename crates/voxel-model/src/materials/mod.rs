//! The materials library — the declared rows and the gate (H-18.8 step 1).
//!
//! **H-18.1:** the declaration table, the generator and the generated table live
//! in this tree; the generator is Rust; the generated table is committed and
//! digest-gated; nothing parses at runtime. The references inform the *shape*
//! only — every row is this tree's own, with its own provenance (`[V]` read,
//! `[D]` declared, `[NS]` not yet sourced).
//!
//! * [`schema`] — the substance/process/tier types and the gate that collects
//!   every defect at once.
//! * [`declaration`] — the line format the declaration files are written in,
//!   and its parser. Build-time only: the game never parses a declaration file.
//! * [`generated`] — the committed table the generator writes from the gated
//!   declaration, compiled rather than parsed.
//! * [`spectra`] — the sampled appearance (C-140/C-141): whole USGS splib07a
//!   spectra, exact, and the visible channels derived from them.

pub mod declaration;
pub mod generated;
pub mod schema;
pub mod spectra;
