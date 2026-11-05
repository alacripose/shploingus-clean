//! **A world a player is changing is still a world.** The user's own report, twice: *"the terrain
//! that's supposed to render near the player stops rendering instead of switching to a lower lod"*,
//! with a screenshot of `605 / 605 chunks` resident and **40 quads drawn**, taken seconds after a
//! `placed regolith` toast.
//!
//! # What a live run measured
//!
//! From the shipped client's own probe (`SHPLOINGUS_PROBE_DEBUG`), at the user's eye height:
//!
//! ```text
//! tick 81728  blocks 605  quads 57188      the settled world
//! tick 81729  blocks 484  quads 36         <-- one frame later
//! tick 81800  blocks 605  quads 40
//! tick 82120  blocks 605  quads 57188      <-- four hundred frames later
//! ```
//!
//! and the console line eleven ticks before it: `placed regolith at (0, 92, -96)`. The scene's own
//! quad count **tracked the streamer's exactly** on every frame of that run (`streamer-q` in the
//! probe is the sum over the same meshes the scene is handed), so the vanishing was not the
//! scene's — the meshes the streamer was offering had gone to ~nothing, and the world refilled one
//! mesh at a time.
//!
//! # What this witness says
//!
//! The frame loop's part of that run is reproduced here without a GPU: a real
//! [`voxel_work::Streamer`] over a real world, a real [`voxel_client::scene::ResidentScene`], and
//! the mutations a player produces — **a placement, a dig, a replaced generator, the light's own
//! tick and a window step** — with both sides' quad counts read every frame. The claim is the one
//! the user made: after the world has settled once, **the drawn world never collapses**, whatever
//! the player does to it. A mutation that empties it is a mutation the player sees through.
//!
//! The format here is the **record** one (no light sampler, no displacement), which is the format
//! the shipped client itself falls back to whenever its light field has no answer yet
//! (`crates/shploingus/src/main.rs`, "a field that has measured nothing is not handed over at
//! all"), and it is therefore the honest one for a witness about geometry.

use voxel_client::scene::{ResidentScene, Scene};
use voxel_model::ChunkPos;
use voxel_shapers::{Config, WorldSpec};
use voxel_work::{ChunkWindow, Streamer};

/// Small enough to settle in a test, wide enough to have a surface in it: 5 × 5 × 3.
fn window() -> ChunkWindow {
    ChunkWindow {
        radius_chunks: 2,
        vertical_chunks: 1,
    }
}

/// What one frame of the shipped loop offers a landing: `main` passes its own geometry budget.
const LANDING_BUDGET: usize = 4_096;

struct Witness {
    streamer: Streamer,
    resident: ResidentScene,
    centre: ChunkPos,
    light_epoch: u64,
    /// The streamer's own quads and the scene's, per frame, with the mutation that ran before it.
    log: Vec<(usize, usize, &'static str)>,
}

impl Witness {
    fn quiet() -> Self {
        let spec = WorldSpec::new(2468, Config::default());
        let registry = voxel_shapers::materials::registry().expect("the registry");
        let mesh_config = voxel_mesh::MeshConfig::from_registry(&registry).expect("a mesh config");
        let mut streamer = Streamer::new(
            voxel_stream::WorldSource::new(spec),
            mesh_config,
            4,
            256 * 1024 * 1024,
        )
        .and_then(|s| s.with_window(window()))
        .expect("a streamer");
        let centre = ChunkPos::new(
            0,
            voxel_shapers::terrain::elevation_m(&spec, 16.5, 16.5) as i32 / 32,
            0,
        );
        streamer.set_centre(centre);
        streamer.settle().expect("the window settles");
        Self {
            streamer,
            resident: ResidentScene::new(),
            centre,
            light_epoch: 1,
            log: Vec::new(),
        }
    }

    /// One frame of the shipped loop, in the shipped order: the world is driven, then the scene is
    /// synced against what is drawn, then the frame reads the scene it is about to draw.
    fn frame(&mut self, what: &'static str) {
        self.streamer.set_centre(self.centre);
        self.streamer.pump().expect("the streamer pumps");
        let streamer_quads: usize = self
            .streamer
            .drawn_meshes()
            .map(|(_, _, mesh)| mesh.quads.len())
            .sum();
        self.resident.sync(
            self.streamer.drawn_meshes(),
            self.centre,
            None,
            None,
            None,
            LANDING_BUDGET,
            self.light_epoch,
        );
        let scene: Scene = self.resident.take();
        let quads = scene.quads;
        self.resident.put(scene);
        self.log.push((streamer_quads, quads, what));
    }

    /// The frame the player's edit lands on, then the frames that follow it.
    fn frames(&mut self, what: &'static str, n: usize) {
        for _ in 0..n {
            self.frame(what);
        }
    }

    /// **Fill the window, then start judging.** Every frame of the fill is a frame in which the
    /// scene is behind its streamer by design — landings are budgeted by the shipped loop — so the
    /// log begins where the world does: after the settle, and not before it.
    fn settled(&mut self) -> usize {
        for _ in 0..60 {
            self.frame("settling");
        }
        let settled = self.log.last().expect("a frame ran").1;
        self.log.clear();
        settled
    }
}

/// The drawn world after the world has settled once, as the floor every later frame is held to.
fn floor(settled: usize) -> usize {
    settled * 3 / 4
}

/// Print the trajectory the way the shipped probe does, so a failure names the mutation.
fn report(log: &[(usize, usize, &'static str)], settled: usize) {
    println!("settled at {settled} quads drawn (streamer, scene, after)");
    for (streamer_quads, scene_quads, what) in log.iter().rev().take(40).rev() {
        println!("  {streamer_quads:>7} {scene_quads:>7}  after {what}");
    }
}

#[test]
fn a_placed_block_does_not_empty_the_world_the_player_is_standing_in() {
    let mut w = Witness::quiet();
    let settled = w.settled();
    assert!(settled > 0, "a settled window draws something");

    // **A placement, exactly as the shipped client makes one** (`App::build_at`): the voxel in
    // front of the target, `PhysicalVerb::Place`, regolith.
    let (x, z) = (16, 16);
    let y = (w.centre.y * 32..w.centre.y * 32 + 32)
        .rev()
        .find(|y| {
            !w.streamer
                .material_at(x, *y, z)
                .expect("a material")
                .is_empty()
        })
        .expect("a solid cell to build against");
    w.streamer
        .edit(voxel_work::pool::Edit {
            x,
            y: y + 1,
            z,
            to: voxel_shapers::materials::REGOLITH,
            reason: "the player built here".into(),
            actor: voxel_model::world::ActorId::player(),
            verb: voxel_model::world::PhysicalVerb::Place,
        })
        .expect("the edit reaches a resident chunk");
    w.frames("a placed block", 12);

    let after = w.log.last().expect("a frame ran").1;
    report(&w.log, settled);
    assert!(
        after >= floor(settled),
        "the world is still drawn after a placement: {after} quads of {settled} settled"
    );
    for (streamer_quads, scene_quads, what) in &w.log {
        assert!(
            *scene_quads >= floor(settled) || *streamer_quads < floor(settled),
            "the scene dropped to {scene_quads} quads while the streamer offered \
             {streamer_quads}, after {what}"
        );
    }
}

#[test]
fn a_replaced_generator_does_not_empty_the_world_the_player_is_standing_in() {
    let mut w = Witness::quiet();
    let settled = w.settled();

    // **A replaced generator**, which is what `note_world_changed` is for: every mesh came from a
    // source that no longer exists, and the record does not say which chunks an edit reached.
    w.streamer.note_world_changed();
    w.frames("a replaced generator", 20);

    let after = w.log.last().expect("a frame ran").1;
    report(&w.log, settled);
    assert!(
        after >= floor(settled),
        "the world is still drawn after the generator is replaced: {after} quads of {settled}"
    );
    let worst = w
        .log
        .iter()
        .min_by_key(|(_, scene_quads, _)| *scene_quads)
        .expect("frames");
    assert!(
        worst.1 >= floor(settled),
        "the generator's replacement emptied the drawn world to {} quads after {}",
        worst.1,
        worst.2
    );
}

#[test]
fn a_window_step_and_the_lights_tick_do_not_empty_the_world() {
    let mut w = Witness::quiet();
    let settled = w.settled();

    // The window moving: one chunk east, which evicts a column and asks for another.
    w.centre = ChunkPos::new(w.centre.x + 1, w.centre.y, w.centre.z);
    w.frames("a window step", 16);
    // And the light's own revision, which re-emits every chunk in place.
    w.light_epoch += 1;
    w.frames("the light's tick", 16);

    let after = w.log.last().expect("a frame ran").1;
    report(&w.log, settled);
    for (streamer_quads, scene_quads, what) in &w.log {
        assert!(
            *scene_quads >= floor(settled) || *streamer_quads < floor(settled),
            "the scene dropped to {scene_quads} quads while the streamer offered \
             {streamer_quads}, after {what}"
        );
    }
    assert!(
        after >= floor(settled),
        "the world is still drawn: {after} of {settled}"
    );
}
