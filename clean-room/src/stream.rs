//! **U4 — the camera's resident set: the chunks of the shown world a viewer keeps.**
//!
//! Derived from `docs/DESIGN-SPEC.md` §4 SPEC-3/SPEC-5, the U4 entry of
//! `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §2, and the state the last passes left: the world answers a chunk
//! key on demand ([`CubicWorld::chunk`]) and the client *shows* the world the plane's own law leaves, so
//! this module holds the chunks of **that** world while the camera is near them, drops them when the camera
//! leaves or the state moves, and derives them again when either comes back.
//!
//! # One door, one law, and no second world
//!
//! * **The cells come only from the world's own keyed door.** A residency is filled through
//!   [`CubicWorld::chunk`] over the source it is handed, and its own cells are held in the world's own store
//!   ([`CubicWorld`], the same type the law steps and the drawn frame is meshed from) — never in a parallel
//!   collection of its own. [`Streamer::world`] hands that store to the mesher, so **what the residency holds
//!   is what the frame draws**: the frame's own mesh state is the residency's own identity.
//! * **One membership law, and it is the horizon's own.** A chunk is resident while
//!   [`ResidencyHorizon::keeps`] says the horizon around the camera keeps it — the same typed horizon and the
//!   same chunk-centre distance the geometry residency is bounded by, so streaming and meshing cannot
//!   disagree about what *near the camera* means.
//! * **One key law, and it is the world's.** The candidates are the keys the source itself names
//!   ([`CubicWorld::keys`]). A key the horizon names that this world does not hold is not a candidate at all:
//!   this world is finite, and the counted consequence is printed by the measurement rather than hidden.
//! * **One state, named.** The residency records the source's own identity ([`CubicWorld::digest`]) with the
//!   cells it holds, and a step whose source has moved **releases everything it held** before it derives
//!   anything ([`StreamStep::stale`]): a chunk of a world that is gone is not served, because the picture it
//!   would draw is a picture of the wrong tick — the same rule the mesher's own state key already takes.
//!   A residency is also over **one world's own frame** (patch and span): a source of another shape is
//!   refused by name rather than mixed into it.
//!
//! # What a step does, and what it costs
//!
//! [`Streamer::observe`] is the whole policy, in one call: the camera's chunk is the anchor, the horizon
//! decides membership, chunks of a state that moved or a horizon that no longer keeps them are **evicted**
//! (their cells dropped, not kept "just in case"), chunks the horizon wants that are not held are **derived**
//! through the source's keyed door, and chunks already held **of this state** are **served** with no
//! derivation at all — which is what makes a still camera free and a walk back to a place you left cost one
//! derivation per chunk, not two. The step's own counts come back ([`StreamStep`]) so a caller prints what
//! happened instead of guessing.
//!
//! # What is owed, named here rather than implied
//!
//! * The residency holds **whole chunks**; a budget on how much may be resident at once (a ceiling and a
//!   policy for what to drop first) is owed — the horizon is the only limit today.
//! * A **per-chunk memory figure** is owed by name elsewhere in the record and is deliberately not invented
//!   here: this module counts chunks and cells, never bytes.
//! * The residency is observed **once per state**: while the law steps, every tick's cells are a new state,
//!   so a live window re-derives its chunks each frame and the step's `stale` count says so. Serving across
//!   frames would need the law to stand still, which the combined reading in `src/surface.rs` does.

use crate::chunk::{key_mismatch_refusal, Chunk, CubicWorld};
use crate::coordinates::ChunkKey;
use crate::digest::Digest;
use crate::mesh::ResidencyHorizon;
use crate::refusal::{Refusal, RefusalKind};

/// **What one step of the residency did.** The anchor it was asked about, the horizon it kept, the source's
/// own identity, how many chunks of that state are held now, and what the step had to do to get there:
/// `derived` is work done through the keyed door, `served` is residency reused, `evicted` is cells released
/// because the horizon no longer keeps them, and `stale` is cells released because the state itself moved.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StreamStep {
    pub anchor: ChunkKey,
    /// The horizon this step kept, in metres — carried in the answer so a printed line says which
    /// horizon the numbers are of rather than leaving a reader to remember (`LEDGER.md` C-46).
    pub horizon_m: f64,
    /// The source's own identity at this step ([`CubicWorld::digest`]) — the state the held chunks are of.
    pub state: Digest,
    /// Chunks resident **after** the step: the horizon's members among this state's own chunks.
    pub resident: usize,
    pub derived: usize,
    pub served: usize,
    pub evicted: usize,
    pub stale: usize,
}

impl StreamStep {
    /// Whether the step had to derive nothing and release nothing: a camera that did not move while the
    /// world stood still.
    pub const fn was_free(&self) -> bool {
        self.derived == 0 && self.evicted == 0 && self.stale == 0
    }

}

/// **The camera's resident set.** A horizon, the cells held for the chunks the horizon keeps, and the
/// identity of the state those cells came from.
///
/// It owns no world and cannot build one: the only cells it ever holds came from the source's own keyed door
/// ([`CubicWorld::chunk`]) at a state the source named, and [`Streamer::world`] is that store itself rather
/// than a second copy of it.
pub struct Streamer {
    residency: ResidencyHorizon,
    /// The resident chunks, in the world's own store: membership is the payload's presence, so the two can
    /// never disagree, and the store's own identity is the residency's.
    held: CubicWorld,
    /// The source's own identity with which the held cells were derived. `None` until the first step, or
    /// after [`Streamer::release`]: nothing held is of any state.
    state: Option<Digest>,
}

impl Streamer {
    /// **A residency over one world's own frame** — the same patch and span, holding nothing until a camera
    /// is observed. A residency that started full would be a world materialised by another name.
    pub fn over(source: &CubicWorld, residency: ResidencyHorizon) -> Streamer {
        Streamer { residency, held: source.bare(), state: None }
    }

    /// The horizon this residency keeps.
    pub const fn residency_horizon(&self) -> ResidencyHorizon {
        self.residency
    }

    /// **The cells the frame draws** — the resident chunks as the world's own store, keyed by the world's own
    /// key law. This is what [`crate::surface::region_mesh`] meshes and what a consumer may read cells from.
    pub const fn world(&self) -> &CubicWorld {
        &self.held
    }

    /// The source's own identity the held cells are of, or `None` when nothing is held.
    pub const fn state(&self) -> Option<Digest> {
        self.state
    }

    /// How many chunks are resident now.
    pub fn len(&self) -> usize {
        self.held.chunks()
    }

    pub fn is_empty(&self) -> bool {
        self.held.chunks() == 0
    }

    /// How many cells are held: every resident chunk is whole, so this is the store's own count and not a
    /// second number that could drift.
    pub fn cells(&self) -> usize {
        self.held.cells()
    }

    /// **Whether a chunk is resident** — membership, read off the payload's own presence.
    pub fn holds(&self, key: ChunkKey) -> bool {
        self.held.holds(key)
    }

    /// **The cells held for a key**, or nothing when this chunk is not resident. The chunk a streamed
    /// consumer draws from, and the reason the residency is worth having: a resident chunk of this state
    /// costs no derivation.
    pub fn chunk(&self, key: ChunkKey) -> Option<&Chunk> {
        self.held.chunk(key).ok()
    }

    /// The resident keys, in ascending order — the store's own key law.
    pub fn keys(&self) -> Vec<ChunkKey> {
        self.held.keys()
    }

    /// **The residency's own identity**: the store's own digest, so a residency holding every chunk of a
    /// world has **that world's identity** and a residency holding a part has neither's. The drawn frame's
    /// own mesh state is this number, which is how a printed frame says what it was drawn from.
    pub fn digest(&self) -> Digest {
        self.held.digest()
    }

    /// **One step of the residency, around a camera's own chunk** — the state first (a source that moved
    /// releases everything), then membership (the horizon's law over this state's own keys), then eviction of
    /// what the horizon no longer keeps, then a derivation through the keyed door for each chunk that is
    /// wanted and not held. A chunk already held **of this state** is **served**: no derivation, no work.
    ///
    /// The step is **all or nothing**: a source of another shape, or a chunk whose own key is not the key it
    /// was asked for, is refused **by name** and the residency is left exactly as it was rather than
    /// half-moved to a horizon or a state it does not hold.
    pub fn observe(&mut self, source: &CubicWorld, anchor: ChunkKey) -> Result<StreamStep, Refusal> {
        // One residency streams one world's own frame: a source of another patch or span would fill this
        // store with cells of a world it is not of, and its digest would name a world that does not exist.
        let (source_floor, source_ceiling) = source.span_z();
        if source.patch() != self.held.patch() || source.span_z() != self.held.span_z() {
            let (floor, ceiling) = self.held.span_z();
            return Err(Refusal::new(
                RefusalKind::ParameterOutOfRange,
                format!(
                    "a residency over the patch at ({}, {}) spanning {floor}..{ceiling} cannot be filled \
                     from a world of the patch at ({}, {}) spanning {source_floor}..{source_ceiling}: one \
                     residency streams one world's own frame",
                    self.held.patch().origin().x,
                    self.held.patch().origin().y,
                    source.patch().origin().x,
                    source.patch().origin().y
                ),
            ));
        }

        // The state, named and compared: when the source's own identity moves, every held chunk is of a world
        // that is gone — released, counted, and never served.
        let state = source.digest();
        let mut stale = 0usize;
        if self.state != Some(state) {
            stale = self.held.chunks();
            self.held = source.bare();
            self.state = Some(state);
        }

        // The horizon's members among **this state's** own chunks, in key order, and the cells each of them
        // needs: derived before anything is released, so a refusal here changes nothing at all.
        let wanted: Vec<ChunkKey> = source
            .keys()
            .into_iter()
            .filter(|key| self.residency.keeps(anchor, *key))
            .collect();
        let mut fresh: Vec<(ChunkKey, Chunk)> = Vec::new();
        let mut served = 0usize;
        for key in &wanted {
            if self.held.holds(*key) {
                served += 1;
                continue;
            }
            let chunk = source.chunk(*key)?.clone();
            if chunk.key() != *key {
                // The source answered another chunk's cells for this key: refused with the store's own
                // sentence, and nothing has moved.
                return Err(key_mismatch_refusal(*key, chunk.key()));
            }
            fresh.push((*key, chunk));
        }
        let derived = fresh.len();

        // Then the step is applied: the horizon's leavers first, then the new cells.
        let mut evicted = 0usize;
        for key in self.held.keys() {
            if wanted.binary_search(&key).is_err() {
                self.held.take(key);
                evicted += 1;
            }
        }
        for (key, chunk) in fresh {
            self.held
                .put(key, chunk)
                .expect("the chunk was checked against its key before the step");
        }

        Ok(StreamStep {
            anchor,
            horizon_m: self.residency.radius_m(),
            state,
            resident: self.held.chunks(),
            derived,
            served,
            evicted,
            stale,
        })
    }

    /// **Release everything** — the store emptied, with the count of what was released and no state claimed.
    /// The one other door that moves membership besides [`Streamer::observe`], and it exists so a session's
    /// end is an act with a number rather than a `drop` nobody can read.
    pub fn release(&mut self) -> usize {
        let released = self.held.chunks();
        self.held = self.held.bare();
        self.state = None;
        released
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automaton::Automaton;
    use crate::chunk::ChunkSource;
    use crate::coordinates::CHUNK_SIDE;
    use crate::mesh::chunk_centre_distance_m;
    use crate::process::{declared_request, roster};
    use crate::reference;
    use crate::stage::ProcessRequest;
    use crate::world::Tick;

    /// The client's own world's declared surface — two chunks each way in plan, one chunk deep — through the
    /// world's own door ([`ChunkSource::over`] over the same request the client declares), materialised
    /// through the source's own whole-patch door: the **declared** world, before the law steps it.
    fn declared_world() -> CubicWorld {
        let roster = roster().expect("the declared roster");
        let identity = reference::identity().expect("the reference identity");
        let ladder = reference::ladder().expect("the reference ladder");
        let origin = declared_request().patch().expect("the declared patch").origin();
        let request = ProcessRequest::Region { origin, side: CHUNK_SIDE as u32 * 2 };
        ChunkSource::over(&roster, identity, Tick(0), request, &ladder, 1)
            .expect("the declared surface resolves")
            .world()
    }

    /// **The world the client shows** — the declared world after the plane's own law has run the declared
    /// count of ticks ([`crate::automaton::SETTLE_TICKS`]), which is the state a frame is a picture of.
    fn shown_world(ticks: usize) -> CubicWorld {
        let mut plane = Automaton::over(&declared_world());
        plane.steps(ticks).expect("the law steps");
        plane.world().clone()
    }

    /// The declared residency horizon the run's own drawn reading uses, in metres.
    fn declared_horizon() -> ResidencyHorizon {
        ResidencyHorizon::m(crate::surface::DRAWN_RESIDENCY_M).expect("a positive horizon")
    }

    /// The world's own first chunk: the chunk carrying the surface, which is where a camera starts.
    fn home(world: &CubicWorld) -> ChunkKey {
        world.first_chunk().expect("the world has a chunk")
    }

    /// The farthest distance from an anchor to any chunk **this world holds**, in metres — the horizon at
    /// which the last chunk leaves. `None` when the world holds nothing.
    fn far_corner(world: &CubicWorld, anchor: ChunkKey) -> Option<f64> {
        world
            .keys()
            .into_iter()
            .filter_map(|key| chunk_centre_distance_m(anchor, key))
            .reduce(f64::max)
    }

    #[test]
    fn a_declared_horizon_holds_every_chunk_of_this_world_and_a_narrower_one_evicts() {
        let world = declared_world();
        let anchor = home(&world);
        let far = far_corner(&world, anchor).expect("the world is not empty");
        let mut streamer = Streamer::over(&world, declared_horizon());

        // The declared horizon: everything this world names is kept, and nothing else is asked for.
        let step = streamer.observe(&world, anchor).expect("the world answers");
        assert_eq!(
            step.resident,
            world.chunks(),
            "the declared horizon must hold every chunk this world names"
        );
        assert_eq!(step.derived, world.chunks(), "and it derived each of them once");
        assert_eq!(step.served, 0);
        assert_eq!(step.evicted, 0);
        assert_eq!(streamer.cells(), world.cells(), "every cell of the world is held");
        assert_eq!(step.state, world.digest(), "and the step names the state it holds");

        // And the world's own far corner is what bounds it: at the horizon that just reaches that corner
        // every chunk is held, and one metre inside it the corner chunk is **evicted** — the first
        // eviction, measured rather than declared.
        let reaches = ResidencyHorizon::m(far.ceil()).expect("a positive horizon");
        let mut at_edge = Streamer::over(&world, reaches);
        let at_edge_step = at_edge.observe(&world, anchor).expect("the world answers");
        let inside = ResidencyHorizon::m(far.ceil() - 1.0).expect("a positive horizon");
        let mut inside_world = Streamer::over(&world, inside);
        let inside_step = inside_world.observe(&world, anchor).expect("the world answers");

        assert_eq!(at_edge_step.resident, world.chunks(), "the far corner is still kept at {far:.4} m");
        assert!(
            inside_step.resident < world.chunks(),
            "one metre inside the far corner {far:.4} m, {} of {} chunk(s) are held — the first chunk is \
             evicted",
            inside_step.resident,
            world.chunks()
        );
        assert_eq!(
            inside_step.evicted,
            0,
            "eviction is for chunks that *were* resident and left the horizon, not for chunks never held"
        );
        println!(
            "stream: {} chunk(s) of this world at {:.0} m ({} cell(s), {} derived); the world's own far \
             corner from chunk ({}, {}, {}) is {far:.4} m, so all {} are held at {:.0} m and {} of {} are \
             held at {:.0} m — the first eviction",
            at_edge_step.resident,
            at_edge_step.horizon_m,
            at_edge.cells(),
            at_edge_step.derived,
            anchor.x,
            anchor.y,
            anchor.z,
            world.chunks(),
            reaches.radius_m(),
            inside_step.resident,
            world.chunks(),
            inside.radius_m(),
        );
    }

    #[test]
    fn a_still_camera_derives_once_and_a_second_look_is_served() {
        let world = declared_world();
        let anchor = home(&world);
        let mut streamer = Streamer::over(&world, declared_horizon());
        let first = streamer.observe(&world, anchor).expect("the world answers");
        let held_after_first = streamer.digest();
        let second = streamer.observe(&world, anchor).expect("the world answers");
        assert!(first.derived > 0);
        assert_eq!(second.derived, 0, "a camera that did not move derives nothing again");
        assert_eq!(second.served, first.resident, "every resident chunk is served");
        assert_eq!(second.evicted, 0);
        assert_eq!(second.stale, 0, "and the state has not moved");
        assert!(second.was_free(), "and the step is free by its own count");
        assert_eq!(streamer.digest(), held_after_first, "a served step moves no cells");
        assert_eq!(
            held_after_first,
            world.digest(),
            "a residency holding the whole world has the world's own identity"
        );
        println!(
            "stream: a still camera: step 1 derived {} chunk(s) and held {}; step 2 derived {} and served \
             {} of {} ({} cell(s) held, identity {})",
            first.derived,
            first.resident,
            second.derived,
            second.served,
            second.resident,
            streamer.cells(),
            streamer.digest().to_hex(),
        );
    }

    #[test]
    fn a_chunk_that_leaves_the_horizon_is_evicted_and_derived_again_when_the_camera_returns() {
        let world = declared_world();
        let anchor = home(&world);
        let far = far_corner(&world, anchor).expect("the world is not empty");
        // A horizon one metre inside the world's own far corner: exactly the corner chunk comes and goes
        // as the camera moves, which is the smallest walk that exercises eviction and re-request.
        let tight = ResidencyHorizon::m(far.ceil() - 1.0).expect("a positive horizon");
        let mut streamer = Streamer::over(&world, tight);
        let start = streamer.observe(&world, anchor).expect("the world answers");
        assert_eq!(start.resident, world.keys().len() - 1, "the corner chunk is outside the tight one");

        // One chunk east: a chunk that was kept falls outside the horizon and is released.
        let east = ChunkKey::new(anchor.x + 1, anchor.y, anchor.z);
        let moved = streamer.observe(&world, east).expect("the world answers");
        assert!(moved.evicted >= 1, "moving the camera evicts what it left behind");
        let after_move = streamer.len();

        // And back: the chunk the camera left is not held any more, so it is **derived again** while the
        // chunks that stayed are served — the residency's own re-request, counted.
        let back = streamer.observe(&world, anchor).expect("the world answers");
        assert!(back.derived >= 1, "a released chunk costs a derivation to come back");
        assert!(back.served >= 1, "and the chunks that never left are served");
        assert_eq!(back.resident, start.resident, "the same camera holds the same chunks");
        println!(
            "stream: a walk at {:.0} m: at ({}, {}, {}) {} chunk(s) held; one chunk east: {} evicted, {} \
             held; back: {} derived, {} served, {} held — the released chunk cost one derivation",
            tight.radius_m(),
            anchor.x,
            anchor.y,
            anchor.z,
            start.resident,
            moved.evicted,
            after_move,
            back.derived,
            back.served,
            back.resident,
        );
    }

    #[test]
    fn a_camera_that_leaves_the_world_releases_everything_and_fills_nothing() {
        let world = declared_world();
        let anchor = home(&world);
        let mut streamer = Streamer::over(&world, declared_horizon());
        streamer.observe(&world, anchor).expect("the world answers");
        assert_eq!(streamer.len(), world.keys().len());

        // Far outside this world's own patch: the horizon keeps none of the keys this world holds, so
        // everything is released — and nothing is derived, because a key this world does not name is not
        // a candidate at all. This is the whole of the difference between this world and a generator.
        let away = ChunkKey::new(anchor.x + 9, anchor.y + 9, anchor.z + 7);
        let step = streamer.observe(&world, away).expect("the world answers");
        assert_eq!(step.resident, 0, "a camera outside the world holds nothing");
        assert_eq!(step.derived, 0, "and there is no generator to fill the horizon with");
        assert!(step.evicted > 0, "what it held is released");
        assert_eq!(streamer.cells(), 0);
        println!(
            "stream: a camera 9 chunk(s) away from the world's own patch: {} released, {} held, {} derived \
             — this world owns {} chunk(s) and generates no others",
            step.evicted,
            step.resident,
            step.derived,
            world.keys().len(),
        );
    }

    #[test]
    fn the_resident_chunks_are_the_worlds_own_chunks_cell_for_cell() {
        let world = declared_world();
        let anchor = world.first_chunk().expect("the world has a chunk");
        let mut streamer = Streamer::over(&world, declared_horizon());
        let step = streamer.observe(&world, anchor).expect("the world answers");
        assert_eq!(step.resident, world.chunks());

        let mut differing = 0usize;
        let mut cells = 0usize;
        for key in streamer.keys() {
            let held = streamer.chunk(key).expect("a held key has a payload");
            let ours = world.chunk(key).expect("a held key is one this world owns");
            assert_eq!(held.key(), key, "the payload is the chunk its key names");
            for (cell, (held_cell, world_cell)) in held.cells().iter().zip(ours.cells()).enumerate() {
                cells += 1;
                if held_cell != world_cell {
                    differing += 1;
                    assert_eq!(
                        held_cell, world_cell,
                        "cell {cell} of chunk ({}, {}, {}) is not the world's own",
                        key.x, key.y, key.z
                    );
                }
            }
        }
        assert_eq!(differing, 0, "the streamed cells must be the world's own cells");
        assert_eq!(cells, world.cells(), "and every cell of the world must be covered");
        println!(
            "stream: the resident set against the world it came from: {} of {} cell(s) equal ({} differing) \
             over {} chunk(s); the residency's identity {}",
            cells - differing,
            world.cells(),
            differing,
            streamer.len(),
            streamer.digest().to_hex(),
        );
    }

    #[test]
    fn the_residencys_identity_moves_with_its_membership() {
        let world = declared_world();
        let anchor = home(&world);
        let far = far_corner(&world, anchor).expect("the world is not empty");
        let mut full = Streamer::over(&world, declared_horizon());
        let mut tight =
            Streamer::over(&world, ResidencyHorizon::m(far.ceil() - 1.0).expect("positive"));
        full.observe(&world, anchor).expect("the world answers");
        tight.observe(&world, anchor).expect("the world answers");
        assert_ne!(
            full.digest(),
            tight.digest(),
            "two residencies holding different chunks cannot share one identity"
        );
        assert_eq!(full.digest(), world.digest(), "the whole set is the world's own identity");
        assert_ne!(tight.digest(), world.digest(), "and a part is not the whole");

        // The same membership built in a different order is the same residency: a set, not a history.
        let mut again = Streamer::over(&world, declared_horizon());
        again.observe(&world, anchor).expect("the world answers");
        assert_eq!(full.digest(), again.digest());
        assert_eq!(full.keys(), again.keys());

        // Releasing everything returns the identity to the empty frame's, and claims no state.
        let empty = world.bare();
        let released = full.release();
        assert_eq!(released, again.len());
        assert_eq!(full.digest(), empty.digest(), "an emptied residency is an empty one");
        assert_eq!(full.state(), None, "and it holds no state");
    }

    /// **The state is the residency's key** — the guard that makes *what the residency holds is what the
    /// frame shows* true of a moving world: the same camera, the same horizon, and a source one tick older or
    /// newer is a set of chunks of a world that is gone. Nothing stale is served, and what is held after the
    /// step is the new state's own cells, cell for cell.
    #[test]
    fn a_state_that_moves_makes_every_held_chunk_stale_and_none_is_served() {
        let declared = declared_world();
        let anchor = home(&declared);
        let mut streamer = Streamer::over(&declared, declared_horizon());
        let first = streamer.observe(&declared, anchor).expect("the world answers");
        assert_eq!(first.derived, declared.chunks());

        // One tick of the plane's own law: the same patch, the same keys, other cells.
        let moved = shown_world(1);
        assert_ne!(moved.digest(), declared.digest(), "one tick moves the state");

        let step = streamer.observe(&moved, anchor).expect("the moved world answers");
        assert_eq!(
            step.stale,
            declared.chunks(),
            "every chunk held of the old state is released, not served"
        );
        assert_eq!(step.served, 0, "and nothing of the old state is served");
        assert_eq!(step.derived, moved.chunks(), "the new state's own chunks are derived");
        assert_eq!(step.evicted, 0, "nothing left the horizon: the horizon did not move");
        assert_eq!(step.state, moved.digest(), "the step names the state it now holds");
        assert_eq!(
            streamer.digest(),
            moved.digest(),
            "and the residency holds the new state's own cells"
        );

        // Cell for cell, not by construction: the residency was *filled* from the new state, and every cell
        // of it is that state's own.
        let mut differing = 0usize;
        for key in streamer.keys() {
            let held = streamer.chunk(key).expect("a held key has a payload");
            let ours = moved.chunk(key).expect("a held key is one the moved world holds");
            differing += held.cells().iter().zip(ours.cells()).filter(|(a, b)| a != b).count();
        }
        assert_eq!(differing, 0, "a residency that served a stale chunk would differ here");
        println!(
            "stream: the state moved: {} of {} chunk(s) released as stale, {} derived at the new state, \
             {} differing cell(s) against it — the residency is the state's own",
            step.stale,
            declared.chunks(),
            step.derived,
            differing
        );
    }

    /// **One residency streams one world's own frame** — a source of another patch is refused by name, and
    /// the residency keeps exactly what it held.
    #[test]
    fn a_source_of_another_shape_is_refused_by_name_and_nothing_moves() {
        let world = declared_world();
        let anchor = home(&world);
        let mut streamer = Streamer::over(&world, declared_horizon());
        streamer.observe(&world, anchor).expect("the world answers");
        let before = streamer.digest();
        let held = streamer.len();

        // A world of another patch — three chunks a side rather than two — through the same door.
        let other = {
            let roster = roster().expect("the declared roster");
            let identity = reference::identity().expect("the reference identity");
            let ladder = reference::ladder().expect("the reference ladder");
            let origin = declared_request().patch().expect("the declared patch").origin();
            let request = ProcessRequest::Region { origin, side: CHUNK_SIDE as u32 * 3 };
            ChunkSource::over(&roster, identity, Tick(0), request, &ladder, 1)
                .expect("the declared surface resolves")
                .world()
        };
        assert_ne!(other.patch(), world.patch(), "another patch");

        let refusal = streamer
            .observe(&other, anchor)
            .expect_err("a source of another shape is refused");
        println!("stream: another frame: [{}] {}", refusal.kind().name(), refusal.detail());
        assert_eq!(refusal.kind(), RefusalKind::ParameterOutOfRange);
        assert_eq!(streamer.digest(), before, "the residency is left exactly as it was");
        assert_eq!(streamer.len(), held);
        assert_eq!(streamer.state(), Some(world.digest()), "and claims the state it holds");
    }
}
