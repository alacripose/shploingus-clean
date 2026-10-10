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
///
/// **The user's target golden seed**, given in the conversation as
/// `-9028097967753437981`. [`WorldSpec::new`] takes a `u64`, and these are the same
/// 64 bits — a two's-complement pair — so the world is identical either way round.
/// That is not an assertion: `relief-profile` takes a seed in *both* forms and the
/// two runs are byte-identical below the header line, which prints which world it
/// measured.
///
/// ## Why this seed, measured
///
/// The seed is a determinism contract ([`GAME_TICK`] is the other half), so it does
/// not move on taste. It moved because a **colliding** body asks a question the
/// floating observer never had to: is there ground to stand on?
///
/// | measured, at `(0.5, -95.5)` | seed 2468 | **this seed** |
/// |---|---|---|
/// | surface at the spawn | 13.83 m — **2.17 m below sea level** | **17.70 m — 1.70 m above** |
/// | nearest dry column | 38.9 m away | **1.4 m away** |
/// | relief over the streamed window | 10.77 m (26.9 % of declared) | **14.01 m (35.0 %)** |
/// | land fraction over 10 km | 45.6 % | 47.1 % |
/// | Convergent cells in 4 km | **5** | **68** |
///
/// Two of those are the reasons. The first is that on seed 2468 the spawn stands in
/// 2.17 m of water, so a body that obeys gravity comes to rest on the sea floor with
/// its head below the waterline — the world was fine and the *camera* was the thing
/// that had been cheating. The second is that mountains are gated on Convergent plate
/// boundaries (`voxel_shapers::terrain`), and 5 of 5,168 cells against 68 of 5,168 is
/// why this world's mountains were 1,275 m away and out of the render horizon.
///
/// This supersedes C-46.3's seed-2468 table, which is left standing as the record of
/// what that world was rather than edited into this one.
pub const GAME_SEED: u64 = 9_418_646_105_956_113_635;

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
/// **What does not move with the tick is the terrain — and the reason is not that the
/// shapers ignore the tick. Measured 2026-10-03 (LEDGER C-117): they take it.**
/// `WorldSpec::stage_seed` mixes `tick` into every stage's seed and the generators
/// (terrain, mountains, caves, geology, moisture, tectonics) all read it — the same seed
/// at tick 0 and tick 24000 differs in **1985 of 5445** sampled voxels, with elevations
/// moving metres (`voxel-shapers/examples/tick-over-world.rs`). What holds the world still
/// is that **nothing ever moves the generator's tick**: `World::shipped()` is the (seed,
/// tick 0) world and the streamer's `WorldSource` is built from it once, so the voxels are
/// the tick-0 world while the light runs on the live clock. That split is the question
/// C-92.0 item 3 owes an answer to, and the measurement names the constraint: handing the
/// live tick to the generator would **reshuffle** the terrain, not age it. This constant
/// is a *starting tick* and must not be read as "the tick"; the live value is
/// `Clock::tick`, which is what the HUD prints.
pub const GAME_TICK: u64 = voxel_shapers::climate::TICKS_PER_DAY * 3 / 8;

/// **A world: a seed, a tick and a configuration, held together.**
///
/// H-7.6 asks for the seed to be a property of a world rather than a constant, and
/// the reason is not tidiness. A save is **record + seed + tick** (H-7.16), and a
/// collider that asks about the world has to be able to say *which* world. Three
/// loose values — a `GAME_SEED` const, a `Clock` and a `Config` — cannot be written
/// down or compared as one thing, and the first symptom of that is the one this
/// session found: the body and the mesh each held their own copy of a spec at tick
/// zero, and neither could be asked what world it was standing in.
///
/// So this is the value that gets threaded, and [`World::spec`] is deliberately a
/// **borrow** rather than a field: the existing shaper calls take `&WorldSpec` and
/// there is no reason to restate them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct World {
    spec: WorldSpec,
}

impl World {
    /// The world the client ships: the target golden seed at tick zero.
    pub fn shipped() -> Self {
        World { spec: WorldSpec::new(GAME_SEED, Config::default()) }
    }

    /// The world's seed — the first half of what a save has to name.
    pub fn seed(&self) -> u64 {
        self.spec.seed
    }

    /// The world's tick — the second half.
    pub fn tick(&self) -> u64 {
        self.spec.tick
    }

    /// The same world at a later tick.
    pub fn at_tick(mut self, tick: u64) -> Self {
        self.spec = self.spec.at_tick(tick);
        self
    }

    /// The underlying spec, for the shapers that already take one.
    pub fn spec(&self) -> &WorldSpec {
        &self.spec
    }

    /// This world asked about one voxel at a time, with the live record in front of
    /// the generated one where there is one.
    pub fn voxels<'a>(
        &'a self,
        streamer: Option<&'a voxel_work::Streamer>,
        earthworks: &'a voxel_shapers::earthworks::Earthworks,
    ) -> Voxels<'a> {
        Voxels { streamer, spec: &self.spec, earthworks }
    }
}

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
/// The eye the client opens at: [`crate::motion::eye`] of [`spawn_feet`], so the
/// eye is derived from a body standing on a real voxel rather than stored.
pub fn spawn_eye(world: &World) -> [f64; 3] {
    crate::motion::eye(spawn_feet(world))
}

/// The world's own answer to "does this voxel block movement", for [`crate::motion`].
///
/// **The live record first, the generator second — and the order is the whole
/// point.** `WorldSource` keeps its *own* [`Earthworks`], and that is the record
/// [`voxel_work::Streamer::edit`] writes a dig or a placed block into. An adapter
/// holding a *different*, never-mutated `Earthworks` therefore asks about a world
/// that stopped being true the moment the player touched it: the body walks
/// straight through a block just placed, and stands on air where one was just dug.
/// That is not a stale cache, it is the wrong record, and no amount of
/// recalculating fixes it.
///
/// So when the streamer is up it is the authority, because it holds the edits. The
/// generator is the **fallback**, for the frames before the world exists.
///
/// The solidity rule itself is [`column::blocks_movement`] — one function, not two
/// copies — so "solid" means the same thing to the mesh, the raycast and the feet.
///
/// A **final refusal is answered `false`**: the voxel does not block. That is the
/// permissive direction, chosen so a player is not stranded at the edge of the
/// window, and it is commented rather than left implicit because it is a policy
/// rather than a derivation.
pub struct Voxels<'a> {
    /// The live record, when the world is up.
    pub streamer: Option<&'a voxel_work::Streamer>,
    /// The generated world, for the frames before the streamer exists and for any
    /// voxel the live record declines to describe.
    pub spec: &'a WorldSpec,
    /// The record layered over [`Voxels::spec`]. Empty in the client, because every
    /// edit goes through the streamer's own record instead.
    pub earthworks: &'a voxel_shapers::earthworks::Earthworks,
}

impl crate::motion::Solidity for Voxels<'_> {
    fn is_solid(&self, x: i32, y: i32, z: i32) -> bool {
        if let Some(streamer) = self.streamer {
            if let Ok(m) = streamer.material_at(x, y, z) {
                return voxel_shapers::column::blocks_movement(m);
            }
        }
        voxel_shapers::column::is_solid(self.spec, self.earthworks, x, y, z).unwrap_or(false)
    }

    /// **Water, the movement's own question, answered from the same record.** The
    /// sharking rule asks whether the *body* touches water; the inherent method above
    /// asks the same world about a point (the eye). One implementation, so the two
    /// cannot disagree about where the sea is.
    fn is_water(&self, x: i32, y: i32, z: i32) -> bool {
        Voxels::is_water(self, x, y, z)
    }
}

impl Voxels<'_> {
    /// Whether a voxel is **water**, from the live record where there is one.
    ///
    /// Asked about the eye rather than derived from the body's motion, because a
    /// body can be in water and not be moving (standing on the sea floor) and can
    /// be moving and not be in it (falling past a waterfall). It is a question
    /// about the world, so it belongs here and not in the HUD, which renders state
    /// rather than going looking for it.
    ///
    /// The rule is [`voxel_shapers::column::is_fluid`] — one spelling of "water",
    /// shared with the edit refusal and the movement check — rather than a second
    /// `m == WATER` comparison. It asks the live **material** record: a lake is
    /// fluid state above the terrain (`voxel_shapers::fluid_state`) and is not a
    /// material voxel, so a body does not swim in a lake here yet — the query that
    /// knows about lakes builds a hydrology region, which is an edit-time price and
    /// not a per-frame one. When lakes are drawn, this is the seam that grows a
    /// cached lake test rather than a second lake rule.
    pub fn is_water(&self, x: i32, y: i32, z: i32) -> bool {
        match self.streamer.and_then(|s| s.material_at(x, y, z).ok()) {
            Some(m) => voxel_shapers::column::is_fluid(m),
            None => voxel_shapers::column::material_at(self.spec, self.earthworks, x, y, z)
                .map(voxel_shapers::column::is_fluid)
                .unwrap_or(false),
        }
    }
}

/// Where the player's **feet** start: standing on the topmost solid voxel of the
/// spawn column.
///
/// This is a different question from the one [`spawn_eye`] used to ask, and the
/// difference is the whole reason it exists. A floating camera wants *a point above
/// the terrain*; a colliding body wants *a surface to stand on*, and those are not
/// the same number — the old derivation put the eye [`GAME_SPAWN_CLEARANCE_M`] above
/// a continuous `f64` elevation, which is a place no voxel boundary has ever been.
///
/// So the feet are found the way the renderer finds the ground: by asking the
/// world which voxel is solid. [`GAME_SPAWN_CLEARANCE_M`] survives as **where the
/// search starts** — a probe ceiling above the surface — which is the role that
/// cannot go stale when the terrain moves, and is the only thing it was ever good
/// for.
pub fn spawn_feet(world: &World) -> [f64; 3] {
    use crate::motion::Solidity;
    let earthworks = voxel_shapers::earthworks::Earthworks::new();
    let world_view = world.voxels(None, &earthworks);
    let spec = world.spec();
    let surface = voxel_shapers::terrain::elevation_m(spec, GAME_SPAWN_XZ.0, GAME_SPAWN_XZ.1);
    let (cx, cz) = (GAME_SPAWN_XZ.0.floor() as i32, GAME_SPAWN_XZ.1.floor() as i32);
    // Descending from the clearance, and never further than 8 m below the surface:
    // a spawn with no floor under it at all is a broken world, and returning the
    // continuous elevation is the honest "there is nothing to stand on" rather than
    // a number that pretends otherwise.
    let mut y = (surface + GAME_SPAWN_CLEARANCE_M).floor() as i32;
    while y as f64 > surface - 8.0 {
        if world_view.is_solid(cx, y, cz) {
            // The top face of voxel `y` is `y + 1`, and the voxel above it is air by
            // construction — it is the highest solid one, which is what makes this
            // the floor rather than a guess at one.
            return [GAME_SPAWN_XZ.0, y as f64 + 1.0, GAME_SPAWN_XZ.1];
        }
        y -= 1;
    }
    [GAME_SPAWN_XZ.0, surface, GAME_SPAWN_XZ.1]
}

/// The starting pitch, in radians. Nearly level: the eye is over a coast, and
/// the horizon is where the terrain is.
pub const GAME_PITCH_RAD: f64 = -0.08;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::motion::{Player, Solidity, Wish, PLAYER_STEP};

    /// The world every test here measures against: the one the client ships.
    fn shipped() -> World {
        World::shipped()
    }

    #[test]
    fn the_spawn_is_ground_the_body_can_stand_on() {
        let world = shipped();
        let spec = world.spec();
        let earthworks = voxel_shapers::earthworks::Earthworks::new();
        let w = world.voxels(None, &earthworks);
        let feet = spawn_feet(&world);

        // The dry-spawn claim, which is the reason the seed changed. On seed 2468
        // this assertion fails by 2.17 m, because the spawn stands in the sea.
        assert!(
            feet[1] > spec.config.sea_level_m,
            "spawn feet at {} m are not above the {} m waterline — the body would be submerged",
            feet[1],
            spec.config.sea_level_m
        );

        // The feet rest on something solid.
        let (cx, cz) = (feet[0].floor() as i32, feet[2].floor() as i32);
        let below = feet[1] as i32 - 1;
        assert!(
            w.is_solid(cx, below, cz),
            "nothing solid under the spawn at voxel y {below}"
        );
        // And the body is not standing inside the hill.
        assert!(!w.is_solid(cx, feet[1] as i32, cz), "the feet are inside solid ground");
        assert!(
            !w.is_solid(cx, (feet[1] + crate::motion::BODY_HEIGHT - 0.01) as i32, cz),
            "there is no head clearance at the spawn"
        );

        // And the decisive one: a body dropped there does not fall. This is the
        // claim the old floating camera could not make and never had to.
        let mut player = Player::new(feet);
        for _ in 0..180 {
            player.advance(&Wish::STILL, 0.0, PLAYER_STEP, &w);
        }
        assert!(
            (player.motion.feet[1] - feet[1]).abs() < 1e-9,
            "the body fell from {} m to {} m",
            feet[1],
            player.motion.feet[1]
        );
        assert!(player.motion.grounded, "the body is not standing on the ground at spawn");
        println!(
            "seed {} at tick {}: spawn feet {feet:?}, eye {:?}, sea level {} m",
            world.seed(),
            world.tick(),
            player.eye(),
            spec.config.sea_level_m
        );
    }

    /// **The collider must see the player's own edits.**
    ///
    /// This is the reported bug, and it was not a stale cache: the adapter was
    /// holding a *different* `Earthworks` from the one `Streamer::edit` writes, so
    /// the body walked through blocks the player had just placed and stood on air
    /// where one had just been dug. No amount of recalculating fixed it, because
    /// the record being consulted was never the record being changed.
    ///
    /// So the test does the real thing — a real streamer, a real dig, a real place —
    /// and asks the collider what it thinks afterwards. Teeth: an adapter that
    /// ignores the streamer fails at the first assertion after the dig.
    #[test]
    fn the_collider_sees_the_players_own_edits() {
        use crate::motion::Solidity;
        use voxel_shapers::column::materials;

        let world = shipped();
        let earthworks = voxel_shapers::earthworks::Earthworks::new();
        let source = voxel_stream::WorldSource::new(*world.spec());
        let registry = materials::registry().expect("the registry");
        let mesh_config = voxel_mesh::MeshConfig::from_registry(&registry).expect("a mesh config");
        let mut streamer = voxel_work::Streamer::new(source, mesh_config, 1, crate::GAME_CEILING_BYTES)
            .and_then(|s| s.with_window(GAME_WINDOW))
            .expect("the streamer");
        let feet = spawn_feet(&world);
        let (cx, cz) = (feet[0].floor() as i32, feet[2].floor() as i32);
        let ground = feet[1] as i32 - 1;
        streamer.set_centre(voxel_model::ChunkPos::new(
            (feet[0] / 32.0).floor() as i32,
            (feet[1] / 32.0).floor() as i32,
            (feet[2] / 32.0).floor() as i32,
        ));
        streamer.settle().expect("the window settles");

        // A macro, not a closure or a binding: the view borrows the streamer
        // immutably and the edits below need it mutably, so the borrow has to end at
        // the end of each statement. A closure would hold it for the closure's whole
        // life, which is exactly the span the edits fall in.
        macro_rules! live {
            () => {
                Voxels { streamer: Some(&streamer), spec: world.spec(), earthworks: &earthworks }
            };
        }
        macro_rules! generated {
            () => {
                Voxels { streamer: None, spec: world.spec(), earthworks: &earthworks }
            };
        }

        // Before any edit the two records agree — otherwise this test would be
        // measuring a permanent disagreement rather than the effect of an edit.
        assert!(live!().is_solid(cx, ground, cz), "the ground is not solid to begin with");
        assert!(generated!().is_solid(cx, ground, cz));

        // **Dig it.** The collider must stop believing there is anything to stand on.
        streamer
            .edit(voxel_work::pool::Edit {
                x: cx,
                y: ground,
                z: cz,
                to: materials::AIR,
                reason: "the player dug here".into(),
                actor: voxel_model::world::ActorId::player(),
                verb: voxel_model::world::PhysicalVerb::Dig,
            })
            .expect("a dig is recorded");
        assert!(
            !live!().is_solid(cx, ground, cz),
            "the collider still sees a block the player just dug"
        );
        // And this is the disagreement that caused the bug: the generator, which
        // knows nothing of the edit, still reports it solid. That is exactly why the
        // live record has to be asked first.
        assert!(
            generated!().is_solid(cx, ground, cz),
            "the generator should still report the dug block — if it does not, this test is not testing the bug"
        );

        // And the body agrees, **while the hole is still open**: it falls, because the
        // floor it is told about is the one that exists now. Checked here rather than
        // at the end because the placement below goes in at the body's own foot
        // level, and a body standing on a block it just placed is not falling into
        // anything. Six seconds of it — 120 steps at the movement's 20 Hz — because a
        // single step of the Squake gravity is 78 mm and asserting on one step would
        // pass for a body that never moved.
        let mut player = Player::new([feet[0], feet[1] + 1.0, feet[2]]);
        for _ in 0..120 {
            player.advance(&Wish::STILL, 0.0, PLAYER_STEP, &live!());
        }
        assert!(
            player.motion.feet[1] < feet[1] - 0.5,
            "the body did not fall into the hole the player dug: {}",
            player.motion.feet[1]
        );

        // **Place one.** The collider must start believing there is something in the way.
        let air = feet[1] as i32;
        assert!(!live!().is_solid(cx, air, cz), "the spawn column is not clear to begin with");
        streamer
            .edit(voxel_work::pool::Edit {
                x: cx,
                y: air,
                z: cz,
                to: materials::REGOLITH,
                reason: "the player built here".into(),
                actor: voxel_model::world::ActorId::player(),
                verb: voxel_model::world::PhysicalVerb::Place,
            })
            .expect("a placement is recorded");
        assert!(
            live!().is_solid(cx, air, cz),
            "the collider walks through a block the player just placed"
        );
    }

    #[test]
    fn the_eye_is_the_eye_height_above_the_spawn_feet() {
        let world = shipped();
        assert_eq!(spawn_eye(&world)[1], spawn_feet(&world)[1] + crate::motion::EYE_HEIGHT);
    }

    /// **A world is a seed, a tick and a configuration, and the tick is part of it.**
    ///
    /// Asserted because the value was introduced to be *written down* — a save is
    /// record + seed + tick — and a struct that quietly dropped the tick on the way
    /// to a spec would make every save claim false in the same way.
    #[test]
    fn a_world_carries_its_seed_and_its_tick() {
        let world = shipped();
        assert_eq!(world.seed(), GAME_SEED);
        assert_eq!(world.tick(), 0, "a fresh world starts at tick zero");
        let later = world.at_tick(9_000);
        assert_eq!(later.seed(), world.seed(), "the tick must not disturb the seed");
        assert_eq!(later.tick(), 9_000);
        assert_eq!(later.spec().tick, 9_000, "the spec the shapers see carries it too");
    }
}
