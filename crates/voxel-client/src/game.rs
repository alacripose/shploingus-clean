//! The world the shipped client streams, as constants.
//!
//! # Why these are here and not in the bin
//!
//! They started in `crates/shploingus/src/main.rs`, which is where a game's
//! settings belong. But then there is no way to *measure* the view the game
//! shows: a test that wants to render what the window renders has to either
//! duplicate these numbers (and drift) or reach into a binary crate (which Rust
//! does not allow). So they live beside the renderer, and the bin imports them.
//!
//! The consequence matters more than the tidiness: the ignored test
//! `renderer::tests::the_shipped_view_is_not_a_black_screen` builds its scene
//! from **these** constants and the frame it measures is the frame the window
//! draws. `docs/LEDGER.md` C-36 is the entry that says so.
//!
//! # What each number is
//!
//! * [`GAME_WINDOW`] — radius 5 is 11×11 chunks, 352 m across, and 2 chunks above
//!   and below the eye. The band is 2 chunks tall, so 5 chunks of vertical extent
//!   is more than the world has on purpose: it is what lets the player see the
//!   whole playable column and the sky above it.
//! * [`GAME_CEILING_BYTES`] — 256 MB against a measured **90,481 B** for a chunk
//!   with terrain in it, so the window's 605 chunks cost about 0.8 MB of mesh and
//!   the ceiling is not the binding constraint. That is the point of measuring
//!   rather than guessing (`docs/LEDGER.md` C-29).
//! * [`GAME_SEED`] and [`GAME_TICK`] — with the declared period, the **entire**
//!   input to the world. The HUD reports both, so a player can see what the
//!   world is a function of.
//! * [`spawn_eye`] — where the player starts. Derived from the terrain law; see
//!   its own documentation for the measurement that forced it.

use voxel_shapers::{Config, WorldSpec};
use voxel_work::ChunkWindow;

/// The chunk window the client streams.
pub const GAME_WINDOW: ChunkWindow = ChunkWindow { radius_chunks: 5, vertical_chunks: 2 };

/// The memory ceiling handed to the streamer.
pub const GAME_CEILING_BYTES: usize = 256 * 1024 * 1024;

/// The world's seed.
pub const GAME_SEED: u64 = 2468;

/// **Where the world's tick starts**, and nothing more — the client's clock advances it.
///
/// It was `const 0` with the reason recorded here: *"nothing simulates yet, and a HUD
/// that showed a moving tick would be reporting a simulation this build does not have."*
/// **That reason is now false, and it was false the moment the light was wired to the
/// tick.** The simulation this build has is a lighting one: `skylight::daylight_of` is a
/// function of `(seed, tick, atmosphere)`, the client advances the tick through
/// [`crate::clock::Clock`], and the sun's altitude, colour and intensity all move with
/// it.
///
/// ## Why three eighths, and why not a quarter
///
/// **Tick 0 is midnight in this world, and a world that opens at midnight opens black.**
/// The user launched the client and reported it: the sun is `-68.03°` below the horizon,
/// the rig carries **zero** lights and the ambient is `0.00000`, so the terrain is drawn
/// unlit and sunrise is 600 real seconds away. The start tick is therefore a *lighting*
/// decision, not an arbitrary origin.
///
/// **A quarter of the day is not morning here, and this was measured rather than
/// assumed.** `solar_position` is a function of the tick and this world's own latitude,
/// so its day is offset from midnight and a clock-face fraction does not name it:
///
/// | tick | fraction of the day | sun altitude |
/// |---|---|---|
/// | 0 | `0` | `-68.03°` |
/// | 6000 | **`1/4`** | **`-16.06°` — night** |
/// | 9000 | **`3/8`** | **`+10.57°` — morning** |
/// | 12000 | `1/2` | `+21.97°` |
/// | 7678 | — | first tick with the sun above the horizon |
///
/// `3/8` is **declared as a fraction of [`voxel_shapers::climate::TICKS_PER_DAY`] rather
/// than restated as `9_000`**, for the same reason `Cadence::Daily.period_ticks()` reads
/// the day instead of writing `24_000`: so the start tick cannot drift away from the day
/// the stages measure. It puts the sun `10.57°` up — low enough for long shadows, which
/// is what a directional light is for, and still night is a quarter of the day away.
///
/// **What still does not move with the tick is the terrain.** The shapers remain
/// tick-independent (C-92.0 item 3 admits that scope and has not landed it), so the
/// voxels are the same at every tick; what the tick drives today is the light. This
/// constant is a *starting tick* and must not be read as "the tick" — the live value is
/// `Clock::tick`, which is what the HUD prints.
pub const GAME_TICK: u64 = voxel_shapers::climate::TICKS_PER_DAY * 3 / 8;

/// The column the shipped client starts above, in world metres.
pub const GAME_SPAWN_XZ: (f64, f64) = (0.5, -95.5);

/// How far above the generator's own **surface** the eye starts, in metres.
///
/// This is the one chosen number in the spawn, and it is a **clearance** — a
/// thing that does not go stale when the terrain moves. Everything else about
/// the spawn comes from the generator.
pub const GAME_SPAWN_CLEARANCE_M: f64 = 3.0;

/// Where the shipped client starts: above the generator's own surface at
/// [`GAME_SPAWN_XZ`], looking nearly level.
///
/// # Why this is derived and not a constant (measured, 2026-09-29)
///
/// It used to be `GAME_EYE_M = [0.0, 18.0, -96.0]`, picked from a measurement
/// that said the column at `x 0` held water at `y 15` (C-37). That measurement
/// is **right**, and the constant was not wrong — it was *unchecked*. Nothing
/// tied it to the generator, so the next time the terrain moved, the eye would
/// have been left behind with no test able to say so.
///
/// The eye now comes from the terrain law itself. Measured on this column at seed
/// [`GAME_SEED`] with the default config: surface **13.83 m**, a shallow sea (the
/// topmost non-empty voxel is water at `y 15`), and the derived eye is **16.83 m**
/// — about half a metre above the waterline, which is where the old constant was
/// trying to be. The coast rises to 17–19 m of regolith within 150 m along `+Z`,
/// and the nearest column carrying a real mountain term is **1275 m** away at
/// `(1250, 250)`, elevation 49.4 m.
///
/// ## The measurement this replaced, and why it was wrong
///
/// The first version of this doc claimed the surface here was **36.5 m** and the
/// eye was twenty metres underground. That number was measured — on seed
/// **31337**, the terrain module's own test world, read as though it described
/// the shipped world (seed 2468). The client's `topmost solid y = 15, water` probe
/// caught it. Recorded as C-46; the measurement now names its seed in its own
/// first line.
///
/// ## The one thing this placement may not do
///
/// It may not be chosen to *flatter* a broken frame. The clearance is the whole
/// choice; the position comes from the generator's own columns, and the same
/// measurement that reports the view also reports whether it is black.
pub fn spawn_eye() -> [f64; 3] {
    let spec = WorldSpec::new(GAME_SEED, Config::default());
    let surface = voxel_shapers::terrain::elevation_m(&spec, GAME_SPAWN_XZ.0, GAME_SPAWN_XZ.1);
    [GAME_SPAWN_XZ.0, surface + GAME_SPAWN_CLEARANCE_M, GAME_SPAWN_XZ.1]
}

/// The starting pitch, in radians. Nearly level: the eye is over a coast, and
/// the horizon is where the terrain is.
pub const GAME_PITCH_RAD: f64 = -0.08;
