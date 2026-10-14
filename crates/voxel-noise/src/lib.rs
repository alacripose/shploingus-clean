//! The noise primitives, each from its paper.
//!
//! No state, no tables, no streaming RNG: every function here is a **pure
//! function of `(seed, position)`**. That is not an implementation detail — it is
//! what lets a chunk be generated in any thread, in any order, and still produce
//! the same world (F12), and it is why there is no permutation array to fall out
//! of step between workers.
//!
//! | Module | Papers |
//! |---|---|
//! | [`noise`] | Perlin 1985/2002; Worley 1996 |
//! | [`fractal`] | Mandelbrot & Van Ness 1968; Musgrave, Kolb & Mace 1989; Quilez 2002 |
//! | [`rng`] | Steele, Lea & Flood 2014 (splitmix64) |
//! | [`point`] | — (a vector type, deliberately not a dependency) |
//!
//! **The audit, 2026-10-10.** 775 lines, 5 modules, 27 public items, 23 tests; read by `shapers`,
//! `appearance` and `client`. The smallest surface per line of claim in the tree, and the claim is
//! checkable because there is no state to get out of step: a pure function of `(seed, position)` has
//! nothing to reset between runs. **No finding this pass.**

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod fractal;
pub mod noise;
pub mod point;
pub mod rng;

pub use fractal::{Fractal, Kind};
pub use noise::{perlin3, perlin3_clamped, value3, worley3};
pub use point::Vec3;
pub use rng::{hash3, mix, unit2, unit3};
