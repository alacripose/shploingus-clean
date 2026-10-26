//! **The being substrate** — one record, seven faces, and the rules every living thing shares.
//!
//! # What this crate is, and what it deliberately is not
//!
//! `docs/GAIA.md` and `F:\planet-shploingus\docs\CIVILIANS.md` §3.1 both describe one shape: *"A
//! being is a record with an id, and its state is a set of records, not mutable fields."* This crate
//! is that record, and **nothing that any one face owns**. A civilian's needs, a mob's mobility, a
//! ghost's signature and a tree's mass are here; a civilian's agency, a tree's growth and Gaia's law
//! are in the crates that produce them.
//!
//! The vocabulary is the reference's, and it is a correction rather than a preference:
//!
//! * a **being** is the shared living substrate — *life*, of which something can be made;
//! * an **agent** is a being that **acts with intent** — a civilian, a pilot, a god, a mob;
//! * **a plant is a being and not an agent**, and a ghost is the face the table never settled.
//!
//! So [`BeingFace::agency`] is three-valued rather than a `bool`: [`Agency::Agent`],
//! [`Agency::BeingOnly`], and [`Agency::Undecided`] for the ghost, which the doctrine lists as *"open,
//! and asked rather than decided"* (`PATCH-042` §Ghosts). A boolean would have forced that question
//! to be answered by a default, which is how a doctrine becomes a bug.
//!
//! # Why the word is `BeingFace` and not `Face`
//!
//! `voxel_model::Face` already owns the word for **a cube's face** — `PosX`, `NegZ`, and the five
//! others — and it is used on nearly every line of the mesher and the shader. Two meanings of one
//! word in one project is the defect `C-73` §73.1 names, so the living face is spelled
//! [`BeingFace`] here. The reference table's *face* stays the concept; only the identifier moved.
//!
//! # The refusals, and why two of them are structural
//!
//! * **A being with no signature cannot be constructed**, because [`Being::create`] takes a
//!   [`SpiritualSignature`] and has no default. *"Every being is ensouled"* (`CIVILIANS.md` §3.2)
//!   is not checked at runtime by this type; it is impossible to violate without writing a different
//!   constructor, which is the stronger statement ([`PATCH-042`]'s A7 pattern: *the constructor's
//!   signature is the test*).
//! * **A being with no creation record cannot exist either**, for the same reason, and that is what
//!   makes [`Being::require`]'s `AbsentSubject` a refusal about a *reference* rather than about a
//!   being.
//!
//! Everything else is a named [`BeingRefusal`] with the value that was refused in its text.
//!
//! # There is no destruction path, and the absence is tested
//!
//! *"Nothing can kill a being"* (`CIVILIANS.md:118`), and this tree already keeps the shape in
//! `voxel_model::civilian::actor`: retirement is an explicit transition, `Retired` and `Dead` are
//! **states**, and a building is *retired, not deleted*. This crate has `retire`, `die` and no
//! `remove`, `delete`, `destroy` or `drop` — and `the_crate_has_no_destruction_path` reads the
//! crate's own source to assert it, because a comment claiming an absence is worth less than a test
//! looking for the word.
//!
//! [`PATCH-042`]: ../../../patches/PATCH-042-the-axis-layers-and-the-api-scrubber.md

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod being;
pub mod face;
pub mod life;
pub mod refusal;
pub mod signature;

pub use being::{Being, BeingId, CreationRecord};
pub use face::{Agency, BeingFace, Mobility, FACE_COUNT};
pub use life::{LifeStage, Needs, State, STAGE_COUNT, STATE_COUNT};
pub use refusal::BeingRefusal;
pub use signature::{SpiritualSignature, BAND_COUNT, WHEEL_STEPS};
