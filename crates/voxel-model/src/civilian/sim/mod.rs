//! **The sim half of the civilian port: the citizen record and its tasks.**
//!
//! Two of `sim/`'s six files are the civilian process and are copied whole:
//! `citizen.rs` (157 lines) and `task.rs` (343). The other four — `world`, `rng`, `road` and
//! `terrain` — are the **city**, and they are the seam rather than the port: the civilians'
//! *vocabulary* (`World`, `Zone`, `BuildingKind`, `SIM_HZ`) has to exist for the process to
//! name, but the physical city and the eventual mobs are explicitly out of scope in the user's
//! words (*"just not the physical mobs, probably useful for the laya observer and sound engine
//! though"*). **The seam is one-directional**: the process may name the city, and the city does
//! not own the process.
//!
//! They are declared here so the compiler can name the seam exactly, which is how the port has
//! resolved seams twice already (`legacyledger`'s two types; the six-file copy's glob).

pub mod citizen;
pub mod task;
