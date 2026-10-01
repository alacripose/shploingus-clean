//! The streamer: what to build, what to keep, and when to say no.
//!
//! This is the crate's answer to "the world has to be there, continuously, and it
//! must not eat the machine". Three things make that true, and each is a
//! consequence of a measurement rather than a preference:
//!
//! 1. **The generator is the storage.** A chunk is built, meshed and dropped
//!    inside one worker, so nothing but the finished mesh is kept. The octree a
//!    chunk needs costs **~89 kB** and the mesh it produces **~1.3 kB** — keeping
//!    voxels resident to avoid regenerating them is a 182× memory decision to
//!    save 20 ms, and at the declared horizons it is the difference between
//!    **12 MB and 10 GB**.
//! 2. **The window is a box, not a cube.** The first playable band is two chunks
//!    tall; a radius-24 *cube* spends twelve times the vertical extent the world
//!    has. [`ChunkWindow`] keeps a separate vertical extent, so nothing is built
//!    for a viewer who cannot see it.
//! 3. **The budget is charged, not hoped for.** Every resident mesh is charged
//!    against [`Budget`] and a charge that would cross the ceiling is refused,
//!    named, and counted in [`Frame::refused_for_budget`]. The pool's own scratch
//!    is charged too, because at ~4.8 MB per worker it is the largest thing here.
//!
//! # Build-up is the normal state, not a loading screen
//!
//! [`Streamer::pump`] is called every frame. It asks for what the window wants,
//! takes whatever has finished, evicts what has gone out of residency, and
//! returns a [`Frame`] report — so the world visibly fills outward from the
//! viewer, the shapers are always running somewhere, and a HUD can say exactly
//! how far along it is. [`Streamer::settle`] is the same loop run to completion,
//! which is what a test or a first-load screen wants.

use std::collections::{BTreeMap, BTreeSet};

use voxel_mesh::raycast::{Crossings, Ray};
use voxel_mesh::{Mesh, MeshConfig};
use voxel_model::{ChunkPos, Digest, Face, MaterialId, Refusal, RefusalKind, Refused, VoxelPos};
use voxel_shapers::earthworks::Modification;
use voxel_stream::{Horizon, Horizons, WorldSource};

use crate::budget::Budget;
use crate::pool::{Built, Collected, Pool, Request, Submitted};

/// The chunks a viewer's chunk asks for.
///
/// Both extents are declared. `radius_chunks` matches the render horizon's own
/// radius, and `vertical_chunks` is a separate, much smaller number because the
/// world is not as tall as it is wide — the first playable band is 64 voxels, two
/// chunks, and a window that reached as far up as it reaches out would spend most
/// of its work on sky.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ChunkWindow {
    pub radius_chunks: i32,
    pub vertical_chunks: i32,
}

/// The declared default window: the **simulation** horizon's radius, three chunks
/// up and down.
///
/// It was the render horizon's radius, and that was too small for what the window
/// is *for*. A tick stage works over [`Horizon::Simulation`], so at radius 8 it
/// would ask about chunks this window never streams — the neighbourhood the stage
/// reasons over and the neighbourhood the machine holds were not the same place.
/// Radius 12 is the simulation horizon's own number, read rather than retyped, so
/// the window cannot drift from it.
///
/// The cost is declared, not discovered: **4,375** chunks wanted instead of
/// 2,023. That is what [`Budget`] and [`Frame::wanted`] are for, and a test holds
/// the two extents to each other so this line and `Horizon::Simulation` cannot
/// quietly disagree again.
pub const DEFAULT_WINDOW: ChunkWindow = ChunkWindow { radius_chunks: 12, vertical_chunks: 3 };

impl ChunkWindow {
    pub fn validate(&self) -> Refused<()> {
        if self.radius_chunks < 0 || self.vertical_chunks < 0 {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!(
                    "a window of radius {} and vertical {} has a negative extent",
                    self.radius_chunks, self.vertical_chunks
                ),
            ));
        }
        Ok(())
    }

    pub fn contains(&self, centre: ChunkPos, at: ChunkPos) -> bool {
        let d = |a: i32, b: i32| (a as i64 - b as i64).abs();
        d(centre.x, at.x) <= self.radius_chunks as i64
            && d(centre.z, at.z) <= self.radius_chunks as i64
            && d(centre.y, at.y) <= self.vertical_chunks as i64
    }

    /// How many chunks the window holds. A knob a budget can be divided by.
    pub fn count(&self) -> u64 {
        let side = (2 * self.radius_chunks as i64 + 1) as u64;
        let height = (2 * self.vertical_chunks as i64 + 1) as u64;
        side * side * height
    }

    /// Every chunk in the window. Iteration order is scan order, which is not
    /// the build order — [`Self::priority`] is.
    pub fn positions(&self, centre: ChunkPos) -> impl Iterator<Item = ChunkPos> {
        let (radius, vertical) = (self.radius_chunks, self.vertical_chunks);
        (-radius..=radius).flat_map(move |dx| {
            (-vertical..=vertical).flat_map(move |dy| {
                (-radius..=radius).map(move |dz| ChunkPos::new(centre.x + dx, centre.y + dy, centre.z + dz))
            })
        })
    }

    /// **Lower is sooner.** Squared distance from the viewer, so the world fills
    /// outward from where somebody is standing.
    pub fn priority(&self, centre: ChunkPos, at: ChunkPos) -> i64 {
        let dx = (at.x as i64 - centre.x as i64).abs();
        let dy = (at.y as i64 - centre.y as i64).abs();
        let dz = (at.z as i64 - centre.z as i64).abs();
        dx * dx + dy * dy * 4 + dz * dz
    }
}

/// One pump's report — what a HUD shows, and what a test measures.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Frame {
    /// Chunks the window wants.
    pub wanted: usize,
    /// **Current** meshes held. A chunk being rebuilt is not in this number while
    /// its replacement builds, so this is "how much of the world is true".
    pub resident: usize,
    /// Chunks drawn from an **out-of-date** mesh, waiting on a worker.
    ///
    /// Non-zero exactly while something is rebuilding, and the measure of the
    /// flash this crate no longer has: those chunks are covered, not absent.
    pub stale_drawn: usize,
    pub submitted: usize,
    pub completed: usize,
    /// Finished meshes thrown away because the viewer moved off them.
    pub discarded: usize,
    /// Finished meshes thrown away because an edit landed while they were being
    /// built: they belong to a world that no longer exists.
    pub stale: usize,
    /// Submissions the queue's capacity refused. Backpressure, not failure.
    pub queue_full: u64,
    /// Meshes the **budget** refused, cumulatively. The number that must stay
    /// zero for "bounded" to be a claim rather than a hope.
    pub refused_for_budget: u64,
    pub evicted: u64,
    pub failed: u64,
    pub in_flight: usize,
    /// Bytes charged: every resident mesh.
    pub bytes: usize,
    pub ceiling: usize,
    /// Bytes the pool's own threads hold, which no mesh accounts for.
    pub scratch_bytes: usize,
}

impl Frame {
    /// `0.0..=1.0` of the window that is resident.
    pub fn progress(&self) -> f64 {
        if self.wanted == 0 {
            1.0
        } else {
            (self.resident.min(self.wanted) as f64) / self.wanted as f64
        }
    }

    pub fn fill(&self) -> f64 {
        self.bytes as f64 / self.ceiling as f64
    }
}

/// What one batch of finished work changed.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
struct Apply {
    completed: usize,
    discarded: u64,
    stale: usize,
    refused_for_budget: u64,
    failed: u64,
}

/// What a picking ray met, in **world** coordinates.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Pick {
    pub voxel: [i32; 3],
    /// The face the ray came in through; `None` when it began inside the voxel.
    pub entry_face: Option<Face>,
    pub material: MaterialId,
}

/// Every chunk whose mesh depends on a voxel.
///
/// The mesher reads **one voxel past each face** (its `ChunkNeighborhood`), so a
/// voxel on a chunk's low boundary is part of the neighbour below it too, and one
/// on the high boundary is part of the neighbour above. The six face neighbours
/// and no others: a face is determined by voxels that share it, never only a
/// corner, and rebuilding the diagonal chunks for every edit would be work the
/// geometry does not require.
pub fn chunks_touching(voxel: [i32; 3]) -> Vec<ChunkPos> {
    let side = voxel_model::CHUNK_SIDE;
    let base = VoxelPos::new(voxel[0], voxel[1], voxel[2]).chunk();
    let mut out = vec![base];
    for axis in 0..3 {
        let local = voxel[axis].rem_euclid(side);
        if local == 0 {
            let mut at = [base.x, base.y, base.z];
            at[axis] -= 1;
            out.push(ChunkPos::new(at[0], at[1], at[2]));
        }
        if local == side - 1 {
            let mut at = [base.x, base.y, base.z];
            at[axis] += 1;
            out.push(ChunkPos::new(at[0], at[1], at[2]));
        }
    }
    out
}

/// The whole engine: a pool, a mesh residency, and a ceiling.
pub struct Streamer {
    pool: Pool,
    budget: Budget,
    window: ChunkWindow,
    horizons: Horizons,
    centre: ChunkPos,
    meshes: BTreeMap<ChunkPos, Mesh>,
    /// Meshes that are **out of date but still drawn**.
    ///
    /// An edit used to drop the chunk it reached, so between the drop and the
    /// worker's replacement the chunk was *absent from the render* and the
    /// player saw through the hole to whatever was behind it — the "flash" that
    /// made breaking a block hard to aim at. The old mesh is kept here instead,
    /// so the chunk stays covered until something better exists.
    ///
    /// **Three rules hold this honest**, and each is a test:
    ///
    /// 1. **It is still charged.** A stale mesh is still held memory, so it is
    ///    charged exactly like a fresh one and released when it goes. Nothing
    ///    here introduces a cached byte count: the charge is [`Mesh::bytes`],
    ///    which re-walks, and the ledger's "nothing is cached, every count
    ///    re-walks the structure" (LEDGER.md) is what that re-walk is for.
    /// 2. **It is bounded by residency.** A stale mesh is dropped the moment its
    ///    chunk leaves the window, so this map can never outlive the residency
    ///    horizon or grow into a second cache.
    /// 3. **It is not world truth.** [`Streamer::len`] and [`Streamer::digest`]
    ///    count **fresh** meshes only, because a stale mesh is a piece of a
    ///    world that no longer exists and a digest spanning both would be a
    ///    statement about two worlds at once.
    stale: BTreeMap<ChunkPos, Mesh>,
    /// Chunks the generator refused. Remembered so a broken chunk is asked for
    /// once and not every frame — a hot retry loop on a chunk that cannot exist
    /// would spend the whole machine producing the same refusal.
    failed: BTreeSet<ChunkPos>,
    last_refusal: Option<Refusal>,
    evicted: u64,
    discarded: u64,
    refused_for_budget: u64,
    failed_count: u64,
}

impl Streamer {
    pub fn new(
        source: WorldSource,
        config: MeshConfig,
        workers: usize,
        ceiling_bytes: usize,
    ) -> Refused<Self> {
        let budget = Budget::new(ceiling_bytes)?;
        let pool = Pool::new(workers, source, config)?;
        Ok(Streamer {
            pool,
            budget,
            window: DEFAULT_WINDOW,
            horizons: Horizons::declared(),
            centre: ChunkPos::new(0, 0, 0),
            meshes: BTreeMap::new(),
            stale: BTreeMap::new(),
            failed: BTreeSet::new(),
            last_refusal: None,
            evicted: 0,
            discarded: 0,
            refused_for_budget: 0,
            failed_count: 0,
        })
    }

    pub fn with_window(mut self, window: ChunkWindow) -> Refused<Self> {
        window.validate()?;
        self.window = window;
        Ok(self)
    }

    pub fn with_horizons(mut self, horizons: Horizons) -> Refused<Self> {
        horizons.validate()?;
        self.horizons = horizons;
        Ok(self)
    }

    pub fn set_centre(&mut self, centre: ChunkPos) {
        self.centre = centre;
    }

    pub fn centre(&self) -> ChunkPos {
        self.centre
    }

    pub fn window(&self) -> ChunkWindow {
        self.window
    }

    pub fn budget(&self) -> &Budget {
        &self.budget
    }

    pub fn len(&self) -> usize {
        self.meshes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.meshes.is_empty()
    }

    /// The **current** mesh for a chunk, or `None` while it is out of date.
    ///
    /// This is the world as the record currently says it. To ask *what would be
    /// drawn* — which during a rebuild is the stale mesh — use [`Self::drawn`].
    pub fn mesh(&self, pos: ChunkPos) -> Option<&Mesh> {
        self.meshes.get(&pos)
    }

    /// Every mesh that should reach the screen, in chunk order.
    ///
    /// **Fresh and stale together**, because the point of the stale set is that
    /// a chunk waiting on its worker is *covered* rather than transparent. A
    /// stale entry yields `None` from [`Self::mesh`], so a caller that cares
    /// about currency can still tell the two apart.
    pub fn meshes(&self) -> impl Iterator<Item = &Mesh> {
        self.meshes.values().chain(self.stale.values())
    }

    /// What is drawn for a chunk: the current mesh, or the last good one while
    /// a replacement is built.
    pub fn drawn(&self, pos: ChunkPos) -> Option<&Mesh> {
        self.meshes.get(&pos).or_else(|| self.stale.get(&pos))
    }

    /// Chunks currently drawn from an **out-of-date** mesh.
    ///
    /// A HUD number, and the honest measure of the flash: it is non-zero exactly
    /// while something is being rebuilt, and returns to zero when it is done.
    pub fn stale_len(&self) -> usize {
        self.stale.len()
    }

    /// Whether a chunk is being drawn from an out-of-date mesh.
    pub fn is_stale(&self, pos: ChunkPos) -> bool {
        !self.meshes.contains_key(&pos) && self.stale.contains_key(&pos)
    }

    /// The last refusal the generator produced, so a caller can *name* what is
    /// wrong with the world instead of only counting it.
    pub fn last_refusal(&self) -> Option<&Refusal> {
        self.last_refusal.as_ref()
    }

    /// The material at a world voxel, from the live record.
    pub fn material_at(&self, x: i32, y: i32, z: i32) -> Refused<MaterialId> {
        self.pool.material_at(x, y, z)
    }

    /// **Change the world**: record one voxel and mark exactly the chunks
    /// the change reaches as out of date.
    ///
    /// This is a dig, a placed block, anything a player does. The affected chunks
    /// are **demoted to the stale set** — still drawn, still charged — and the
    /// next pumps rebuild them from the new record. Every other resident mesh is
    /// left alone: a dig that blanked the window and rebuilt 605 chunks would be
    /// correct and unplayable, and the difference is this function's whole body.
    ///
    /// **Why they are not dropped.** Dropping them is what produced the flash:
    /// between the drop and the replacement the chunk was in no draw list at all,
    /// so the player saw through the hole while trying to aim the next dig at it.
    /// The mesh a chunk already had is the best available picture of it, so it is
    /// kept until a better one exists. See [`Streamer::stale`].
    ///
    /// A build already **in flight** when the edit lands is not cancelled: it
    /// reads the record under a read lock, so it may finish with the old world in
    /// its hands — and its generation stamp is what keeps that mesh out of the
    /// cache ([`Frame::stale`]).
    pub fn edit(
        &mut self,
        x: i32,
        y: i32,
        z: i32,
        to: MaterialId,
        reason: &str,
    ) -> Refused<Modification> {
        let modification = self.pool.edit(x, y, z, to, reason)?;
        for pos in chunks_touching([x, y, z]) {
            // Demoted, not dropped: the mesh keeps its charge and keeps being
            // drawn, and `absorb` releases it when the replacement lands. A
            // second edit while the first replacement is still building finds the
            // chunk already stale and leaves the existing stale mesh alone —
            // re-demoting it would be a no-op that still cost a lookup.
            if let Some(mesh) = self.meshes.remove(&pos) {
                self.stale.entry(pos).or_insert(mesh);
            }
            // It may be inside again later with the *new* world; a remembered
            // failure would keep the hole the edit just made.
            self.failed.remove(&pos);
        }
        Ok(modification)
    }

    /// The first non-empty voxel along a ray, within `reach` voxels.
    ///
    /// Water counts as a hit: a player can dig into it, and a picking ray that
    /// looked through it would place blocks on the sea floor while the player
    /// aimed at the surface. Refusals from the generator propagate rather than
    /// being read as "nothing there".
    pub fn pick(&self, ray: &Ray, reach: f64) -> Refused<Option<Pick>> {
        let Some(walk) = Crossings::new(ray, reach) else {
            return Ok(None);
        };
        for crossing in walk {
            let material = self.material_at(crossing.voxel[0], crossing.voxel[1], crossing.voxel[2])?;
            if !material.is_empty() {
                return Ok(Some(Pick {
                    voxel: crossing.voxel,
                    entry_face: crossing.entry_face,
                    material,
                }));
            }
        }
        Ok(None)
    }

    /// A digest over the **current** meshes, in chunk order.
    ///
    /// Order-independent by construction — a `BTreeMap` keyed by chunk position —
    /// so this is a statement about the **world** and not about how many workers
    /// produced it (F12).
    ///
    /// **Stale meshes are excluded, deliberately.** A stale mesh is a piece of a
    /// world that no longer exists, so a digest spanning both would be a
    /// statement about two worlds at once — and the whole point of this number
    /// is that a player can watch it settle and know the world has stopped
    /// moving. What is drawn ([`Self::meshes`]) is a wider set than what is
    /// *true*, and those are deliberately different questions.
    pub fn digest(&self) -> Digest {
        let mut t = Vec::new();
        t.extend_from_slice(&(self.meshes.len() as u64).to_le_bytes());
        for (pos, mesh) in &self.meshes {
            t.extend_from_slice(&pos.x.to_le_bytes());
            t.extend_from_slice(&pos.y.to_le_bytes());
            t.extend_from_slice(&pos.z.to_le_bytes());
            t.extend_from_slice(mesh.digest().as_bytes());
        }
        Digest::of_fields("voxel-work/stream", &[t.as_slice()])
    }

    /// Order candidate chunks the way [`Self::pump`] asks for them: **horizon tier
    /// first, then distance, then position.**
    ///
    /// Tier before distance is the ruling, and the reason is backpressure.
    /// [`Pool::submit`] does not block — it refuses the tail when the queue is
    /// full, and reports [`Submitted::QueueFull`]. So the order chunks are offered
    /// in *is* the order they are refused in, and offering in scan order refused
    /// whichever chunk the scan reached last — a corner of the window, the
    /// farthest thing in it — while a chunk beside the viewer went unasked. Ranking
    /// first moves the refusal onto the chunk the viewer would miss least.
    ///
    /// Within one tier this is the old nearest-first order unchanged: every chunk
    /// inside the render horizon has the same tier, so nothing about the near world
    /// changes. The tier only decides between the rings.
    ///
    /// Position last, so the order is **total**: equal tier and equal distance
    /// still submit in one fixed sequence, and two runs of the same world ask in
    /// the same order (F12).
    ///
    /// It is a named method rather than an inline sort because the tests assert
    /// this order, and a tested *copy* of an ordering is not the ordering.
    pub fn rank(&self, candidates: &mut [ChunkPos], centre: ChunkPos) {
        // MUTATION-PROBE REVERTED: the sort is load-bearing, and all three tests here
        // were measured failing when it was removed.
        candidates.sort_unstable_by_key(|p| {
            (
                std::cmp::Reverse(self.horizons.tier(centre, *p)),
                self.window.priority(centre, *p),
                p.x,
                p.y,
                p.z,
            )
        });
    }

    /// One frame of streaming: cancel, evict, ask, take.
    pub fn pump(&mut self) -> Refused<Frame> {
        let centre = self.centre;
        let residency = *self.horizons.rule(Horizon::Residency);

        // 1. Queued work that the viewer has moved off is dropped now.
        self.pool.cancel_where(&|p| residency.contains(centre, p));

        // 2. Meshes outside residency go, and their charge comes back.
        //
        // **Both** sets are swept, which is what bounds the stale map: a stale
        // mesh cannot outlive the window that asked for it, so there is no
        // second cache growing in the corner. A stale mesh is counted as evicted
        // like any other, because that is what the viewer stopped seeing.
        let mut outside: Vec<ChunkPos> = self
            .meshes
            .keys()
            .copied()
            .chain(self.stale.keys().copied())
            .filter(|p| !residency.contains(centre, *p))
            .collect();
        // Dedup: a position cannot be in both sets, but collecting both key
        // spaces into one vector makes that a property of the loop rather than
        // an assumption, and the cost is one sort of a handful of elements.
        outside.sort_unstable();
        outside.dedup();
        let mut evicted = 0u64;
        for pos in outside {
            if let Some(mesh) = self.meshes.remove(&pos) {
                self.budget.release(mesh.bytes());
                evicted += 1;
            } else if let Some(mesh) = self.stale.remove(&pos) {
                self.budget.release(mesh.bytes());
                evicted += 1;
            }
        }
        // A chunk that failed and has left residency is forgotten: it may be
        // inside again later, and a permanent blacklist would leave a hole in the
        // world forever.
        self.failed.retain(|p| residency.contains(centre, *p));
        // And **room was freed**, so what did not fit is worth asking again.
        // Without this a mesh the budget once refused would stay refused for as
        // long as the viewer kept it in residency, even after the eviction of
        // something else made room for it.
        if evicted > 0 {
            self.failed.clear();
        }

        // 3. Ask for what the window wants and does not already have — **in the
        //    order [Self::rank] decides, not in scan order.**
        //
        //    The sort is what makes a full queue refuse the far chunk instead of a
        //    corner, and it is the same call the tests hold, so the order cannot be
        //    right in a test and wrong here.
        let mut missing: Vec<ChunkPos> = self
            .window
            .positions(centre)
            .filter(|p| !self.meshes.contains_key(p) && !self.failed.contains(p))
            .collect();
        self.rank(&mut missing, centre);
        let mut submitted = 0usize;
        let mut queue_full = 0u64;
        for pos in missing {
            let request = Request { pos, priority: self.window.priority(centre, pos) };
            match self.pool.submit(request) {
                Submitted::Queued => submitted += 1,
                Submitted::QueueFull => queue_full += 1,
                Submitted::AlreadyQueued => {}
            }
        }

        // 4. Take what finished.
        let Collected { meshes, failures } = self.pool.collect();
        let applied = self.absorb(meshes, failures);

        self.evicted += evicted;
        Ok(self.report(submitted, queue_full, applied))
    }

    /// Apply a batch of finished work.
    ///
    /// A mesh the budget cannot hold is **refused**, not charged and not stored:
    /// the ceiling is never crossed. It is also remembered as unaffordable, so a
    /// chunk that does not fit is asked for **once** rather than rebuilt every
    /// frame to be refused again — the difference between a bounded world and a
    /// machine spending its whole self producing a refusal.
    fn absorb(&mut self, meshes: Vec<Built>, failures: Vec<(ChunkPos, Refusal)>) -> Apply {
        let centre = self.centre;
        let generation = self.pool.generation();
        let mut apply = Apply::default();
        for built in meshes {
            let Built { generation: built_from, mesh } = built;
            let pos = mesh.chunk;
            if built_from != generation {
                // An edit landed while this chunk was being meshed. It is not a
                // cache miss; it is a piece of a world that no longer exists.
                apply.stale += 1;
                continue;
            }
            if !self.window.contains(centre, pos) {
                apply.discarded += 1;
                continue;
            }
            let bytes = mesh.bytes();
            if self.budget.charge(bytes).is_err() {
                apply.refused_for_budget += 1;
                self.failed.insert(pos);
                continue;
            }
            // The replacement is here, so the stale mesh this position was holding is
            // finished with: its charge comes back. This is the only place a
            // stale mesh is released while the chunk is still resident, and it is
            // why the stale set cannot accumulate across a burst of edits.
            if let Some(previous) = self.stale.remove(&pos) {
                self.budget.release(previous.bytes());
            }
            self.meshes.insert(pos, mesh);
            apply.completed += 1;
        }
        for (pos, refusal) in failures {
            self.failed.insert(pos);
            self.last_refusal = Some(refusal);
            apply.failed += 1;
        }
        self.refused_for_budget += apply.refused_for_budget;
        self.discarded += apply.discarded;
        self.failed_count += apply.failed;
        apply
    }

    fn report(&self, submitted: usize, queue_full: u64, applied: Apply) -> Frame {
        Frame {
            wanted: self.window.count() as usize,
            resident: self.meshes.len(),
            stale_drawn: self.stale.len(),
            submitted,
            completed: applied.completed,
            discarded: applied.discarded as usize,
            stale: applied.stale,
            queue_full,
            refused_for_budget: self.refused_for_budget,
            evicted: self.evicted,
            failed: self.failed_count,
            in_flight: self.pool.in_flight(),
            bytes: self.budget.charged(),
            ceiling: self.budget.ceiling(),
            scratch_bytes: self.pool.scratch_bytes(),
        }
    }

    /// Build the whole window, **waiting** for it.
    ///
    /// Each round asks for what is missing and then waits for the pool, draining
    /// results *while* it waits — which is what keeps a worker blocked on a full
    /// result buffer from stalling the wait forever.
    ///
    /// The bound is derived from the window rather than picked: a round finishes
    /// at least everything already in flight, so `bound` rounds can only be
    /// exhausted by a world that is not arriving, and that case is refused with
    /// the counts rather than looped on.
    pub fn settle(&mut self) -> Refused<Frame> {
        let bound = 8 + self.window.count() / 4;
        let mut last = self.pump()?;
        for _ in 0..bound {
            if last.in_flight == 0 {
                return Ok(last);
            }
            let mut meshes = Vec::new();
            let mut failures = Vec::new();
            self.pool.wait_collecting(&mut |collected| {
                meshes.extend(collected.meshes);
                failures.extend(collected.failures);
            });
            let applied = self.absorb(meshes, failures);
            last = self.report(0, 0, applied);
            if last.in_flight == 0 {
                return Ok(last);
            }
            last = self.pump()?;
        }
        Err(Refusal::owned(
            RefusalKind::MissingStage,
            format!(
                "the window did not settle in {bound} rounds: {} wanted, {} resident, {} in flight",
                last.wanted, last.resident, last.in_flight
            ),
        ))
    }

    /// The generator's world changed.
    ///
    /// Everything is dropped, because **every** mesh came from the generator and
    /// the record does not say which chunks an edit reached. That is coarse on
    /// purpose and matches `voxel_stream::Resident::note_world_changed`: a
    /// cleverer version needs the earthworks record to index itself by chunk,
    /// which is a change to the record and not to this engine.
    pub fn note_world_changed(&mut self) {
        self.meshes.clear();
        // The stale set goes too. Clearing `meshes` alone would drop the fresh
        // meshes while leaving the stale ones held **and still charged** — and
        // the charge is released wholesale on the line below, so the budget would
        // silently stop accounting for memory this crate is still holding. This
        // path is a full world change, so there is nothing worth drawing from it
        // anyway: every chunk is about to be rebuilt from the new source.
        self.stale.clear();
        self.failed.clear();
        self.last_refusal = None;
        self.budget.release(self.budget.charged());
    }

    /// Stop the workers. Consuming, so a stopped streamer cannot be pumped.
    pub fn shutdown(self) {
        self.pool.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_shapers::column::materials;
    use voxel_shapers::{Config, WorldSpec};

    /// Small enough to settle in a test, wide enough to have a surface in it.
    fn window() -> ChunkWindow {
        ChunkWindow { radius_chunks: 1, vertical_chunks: 0 }
    }

    fn source() -> WorldSource {
        WorldSource::new(WorldSpec::new(2468, Config::default()))
    }

    fn config() -> MeshConfig {
        MeshConfig::from_registry(&materials::registry().unwrap()).unwrap()
    }

    /// A streamer centred on the surface, where chunks have geometry in them.
    fn streamer(workers: usize, ceiling: usize) -> Streamer {
        let mut s = Streamer::new(source(), config(), workers, ceiling)
            .unwrap()
            .with_window(window())
            .unwrap();
        s.set_centre(centre());
        s
    }

    fn centre() -> ChunkPos {
        let spec = WorldSpec::new(2468, Config::default());
        let e = voxel_shapers::terrain::elevation_m(&spec, 16.5, 16.5);
        ChunkPos::new(0, e.floor() as i32 / 32, 0)
    }

    #[test]
    fn settle_builds_the_whole_window_and_reports_full_progress() {
        let mut s = streamer(4, 64 * 1024 * 1024);
        let frame = s.settle().unwrap();
        assert_eq!(frame.resident, window().count() as usize);
        assert_eq!(frame.wanted, window().count() as usize);
        assert!((frame.progress() - 1.0).abs() < 1e-12);
        // The invariant that makes the budget meaningful: what is charged is
        // exactly what is held.
        let held: usize = s.meshes().map(Mesh::bytes).sum();
        assert_eq!(s.budget().charged(), held);
        assert_eq!(frame.bytes, held);
        s.shutdown();
    }

    #[test]
    fn the_world_that_arrives_does_not_depend_on_how_many_workers_built_it() {
        // F12 through the whole engine: the pool, the priority queue, the budget
        // and the mesh residency together.
        let mut one = streamer(1, 64 * 1024 * 1024);
        one.settle().unwrap();
        let a = one.digest();
        one.shutdown();

        let mut eight = streamer(8, 64 * 1024 * 1024);
        eight.settle().unwrap();
        let b = eight.digest();
        eight.shutdown();

        assert_eq!(a, b, "the world depended on the worker count");
    }

    #[test]
    fn a_budget_that_cannot_hold_the_window_refuses_meshes_rather_than_crossing_its_ceiling() {
        // 4 kB holds a couple of meshes at the measured ~1.3 kB and not nine.
        let mut s = streamer(2, 4096);
        let frame = s.settle().unwrap();
        assert!(
            frame.refused_for_budget > 0,
            "a 4 kB ceiling held the whole window, so it was not the ceiling that bounded it"
        );
        assert!(frame.bytes <= frame.ceiling);
        assert!(s.budget().charged() <= 4096);
        // And what it did keep it charges for exactly.
        let held: usize = s.meshes().map(Mesh::bytes).sum();
        assert_eq!(s.budget().charged(), held);
        s.shutdown();
    }

    #[test]
    fn a_moving_viewer_evicts_what_it_has_left_behind() {
        let mut s = streamer(4, 64 * 1024 * 1024);
        s.settle().unwrap();
        let before = s.len();
        let bytes_before = s.budget().charged();
        assert!(before > 0);

        // Far enough that nothing in the old window is inside residency.
        let c = s.centre();
        s.set_centre(ChunkPos::new(c.x + 100, c.y, c.z + 100));
        let frame = s.pump().unwrap();
        assert_eq!(frame.resident, 0, "the old window was not evicted");
        assert_eq!(frame.bytes, 0);
        assert!(frame.evicted >= before as u64);
        assert!(bytes_before > 0 && s.budget().charged() == 0);
        s.shutdown();
    }

    #[test]
    fn a_long_run_of_moving_centres_never_crosses_the_ceiling() {
        // The claim this test exists for: bounded is a property of the run, not
        // of one frame. Twenty moves over a small ceiling, checking every pump.
        let ceiling = 32 * 1024;
        let mut s = streamer(2, ceiling);
        let start = s.centre();
        for step in 0..20 {
            s.set_centre(ChunkPos::new(start.x + step, start.y, start.z + step));
            for _ in 0..3 {
                let frame = s.pump().unwrap();
                assert!(
                    frame.bytes <= ceiling,
                    "step {step}: {} bytes of a {ceiling} ceiling",
                    frame.bytes
                );
                assert!(s.budget().charged() <= ceiling);
            }
        }
        s.shutdown();
    }

    #[test]
    fn the_engine_reports_the_scratch_it_costs_as_well_as_the_meshes() {
        // A caller budgeting 32 MB for the world needs to know that four workers
        // are already ~19 MB of it before a single chunk is held.
        let mut s = streamer(4, 64 * 1024 * 1024);
        let frame = s.pump().unwrap();
        assert_eq!(frame.scratch_bytes, 4 * voxel_mesh::Mesher::new().scratch_bytes());
        assert!(frame.scratch_bytes > frame.ceiling / 8, "scratch is not the small part");
        s.shutdown();
    }

    #[test]
    fn the_frame_reports_what_is_drawn_and_what_is_current_as_two_numbers() {
        // A HUD shows `resident`; a reader needs to know whether a chunk missing
        // from that count is *absent* (the flash) or *stale but drawn* (the fix).
        // One number cannot say both, so the frame carries two.
        let mut s = streamer(2, 64 * 1024 * 1024);
        s.settle().unwrap();
        assert_eq!(s.pump().unwrap().stale_drawn, 0, "a settled world has nothing stale");

        let c = s.centre();
        let (x, z) = (c.x * 32 + 16, c.z * 32 + 16);
        let y = (c.y * 32..c.y * 32 + 32)
            .rev()
            .find(|y| !s.material_at(x, *y, z).unwrap().is_empty())
            .expect("the centre chunk's middle column is empty");
        s.edit(x, y, z, materials::AIR, "the test dug here").unwrap();

        let frame = s.pump().unwrap();
        assert_eq!(frame.stale_drawn, 1, "the edited chunk should be reported as drawn-but-stale");
        assert_eq!(frame.resident, frame.wanted - 1, "and as no longer current");
        s.shutdown();
    }

    #[test]
    fn a_voxel_touches_its_own_chunk_and_the_neighbours_whose_pad_reads_it() {
        // The mesher reads one voxel past each face, so the boundary cases are
        // the ones that matter — and `VoxelPos::chunk` must floor, not truncate,
        // or every negative coordinate would land one chunk the wrong way.
        assert_eq!(chunks_touching([16, 16, 16]), vec![ChunkPos::new(0, 0, 0)]);

        let low = chunks_touching([0, 0, 16]);
        assert_eq!(low.len(), 3, "two faces reached, each a face neighbour");
        for at in [
            ChunkPos::new(0, 0, 0),
            ChunkPos::new(-1, 0, 0),
            ChunkPos::new(0, -1, 0),
        ] {
            assert!(low.contains(&at), "{at:?} was not invalidated");
        }
        // The **diagonal** chunk is not reached: a face's geometry is set by
        // voxels that share the face, never by a corner alone. Pinned rather
        // than implied, because including it would rebuild eight chunks a dig.
        assert!(!low.contains(&ChunkPos::new(-1, -1, 0)));

        let high = chunks_touching([31, 31, 16]);
        assert_eq!(high.len(), 3);
        for at in [
            ChunkPos::new(0, 0, 0),
            ChunkPos::new(1, 0, 0),
            ChunkPos::new(0, 1, 0),
        ] {
            assert!(high.contains(&at), "{at:?} was not invalidated");
        }
        assert!(!high.contains(&ChunkPos::new(1, 1, 0)));

        // A negative world coordinate still resolves to the chunk that holds it.
        let middle = chunks_touching([-16, -16, -16]);
        assert_eq!(middle, vec![ChunkPos::new(-1, -1, -1)]);
    }

    #[test]
    fn an_edit_leaves_the_chunk_drawn_from_its_old_mesh_instead_of_a_hole() {
        // **The flash.** Before this rule the reached chunks were dropped, so
        // between the edit and the worker's replacement they were in no draw list
        // at all and the player saw through the hole they had just dug.
        let mut s = streamer(2, 64 * 1024 * 1024);
        s.settle().unwrap();
        let c = s.centre();
        let (x, z) = (c.x * 32 + 16, c.z * 32 + 16);
        let y = (c.y * 32..c.y * 32 + 32)
            .rev()
            .find(|y| !s.material_at(x, *y, z).unwrap().is_empty())
            .expect("the centre chunk's middle column is empty");
        let to = materials::AIR;

        let reached: Vec<ChunkPos> = chunks_touching([x, y, z])
            .into_iter()
            .filter(|p| s.mesh(*p).is_some())
            .collect();
        assert_eq!(reached.len(), 1, "a mid-chunk voxel reaches one resident chunk");
        let target = reached[0];
        let current_before = s.len();
        let drawn_before = s.drawn(target).expect("the chunk was resident").digest();

        s.edit(x, y, z, to, "the test dug here").unwrap();

        // The chunk is still on screen, drawn from the mesh it had before.
        assert!(
            s.drawn(target).is_some(),
            "the edited chunk vanished from the draw list: this is the flash"
        );
        assert_eq!(s.drawn(target).unwrap().digest(), drawn_before, "and it changed under us");
        // While being drawn, it is not claimed to be current.
        assert!(s.is_stale(target));
        assert!(s.mesh(target).is_none(), "a stale mesh is not world truth");
        assert_eq!(s.stale_len(), 1);

        // And it is still true that the world is not settled: `len` counts **current**
        // meshes only, so the reached chunk has left that count even though it is
        // still on screen.
        assert_eq!(s.len(), current_before - 1, "len counts current meshes only");
        s.shutdown();
    }

    #[test]
    fn a_stale_mesh_is_still_charged_and_its_charge_comes_back_when_it_is_replaced() {
        // **Rule 1: it is still held memory.** Held and not hidden — the budget
        // never lies about what it is holding, and this is why no byte count here
        // is cached (LEDGER.md: "nothing is cached, every count re-walks").
        let mut s = streamer(2, 64 * 1024 * 1024);
        s.settle().unwrap();
        let charged_before = s.budget().charged();

        let c = s.centre();
        let (x, z) = (c.x * 32 + 16, c.z * 32 + 16);
        let y = (c.y * 32..c.y * 32 + 32)
            .rev()
            .find(|y| !s.material_at(x, *y, z).unwrap().is_empty())
            .expect("the centre chunk's middle column is empty");
        let target = chunks_touching([x, y, z])
            .into_iter()
            .find(|p| s.mesh(*p).is_some())
            .expect("the edit reaches a resident chunk");

        s.edit(x, y, z, materials::AIR, "the test dug here").unwrap();
        assert_eq!(
            s.budget().charged(),
            charged_before,
            "a stale mesh is still held, so its charge must not silently vanish"
        );

        s.settle().unwrap();
        assert_eq!(s.stale_len(), 0, "the replacement should have released it");
        assert_eq!(s.budget().charged(), charged_before, "and given the charge back");
        assert!(s.mesh(target).is_some(), "and the chunk should be current again");
        s.shutdown();
    }

    #[test]
    fn a_stale_mesh_cannot_outlive_the_residency_it_was_asked_for() {
        // **Rule 2: the stale set is bounded.** If this fails, the stale map is a
        // second cache that grows without limit — the thing it must never be.
        let mut s = streamer(2, 64 * 1024 * 1024);
        s.settle().unwrap();
        let c = s.centre();
        let (x, z) = (c.x * 32 + 16, c.z * 32 + 16);
        let y = (c.y * 32..c.y * 32 + 32)
            .rev()
            .find(|y| !s.material_at(x, *y, z).unwrap().is_empty())
            .expect("the centre chunk's middle column is empty");
        s.edit(x, y, z, materials::AIR, "the test dug here").unwrap();
        assert_eq!(s.stale_len(), 1, "the edit made something stale");

        // Walk the viewer far enough away that the edited chunk is out of window.
        s.set_centre(ChunkPos::new(c.x + 10_000, c.y, c.z));
        s.pump().unwrap();
        assert_eq!(s.stale_len(), 0, "a stale mesh outlived its residency horizon");
        s.shutdown();
    }

    #[test]
    fn the_digest_never_spans_two_worlds() {
        // **Rule 3: stale is not truth.** A digest over fresh *and* stale would
        // be a statement about the old world and the new one together.
        let mut s = streamer(2, 64 * 1024 * 1024);
        s.settle().unwrap();
        let settled = s.digest();

        let c = s.centre();
        let (x, z) = (c.x * 32 + 16, c.z * 32 + 16);
        let y = (c.y * 32..c.y * 32 + 32)
            .rev()
            .find(|y| !s.material_at(x, *y, z).unwrap().is_empty())
            .expect("the centre chunk's middle column is empty");
        s.edit(x, y, z, materials::AIR, "the test dug here").unwrap();

        assert_ne!(
            s.digest(),
            settled,
            "an edited world must not digest as the world that was"
        );
        s.settle().unwrap();
        assert_ne!(
            s.digest(),
            settled,
            "and once rebuilt it is still not the world that was: the dig is real"
        );
        s.shutdown();
    }

    #[test]
    fn an_edit_drops_only_the_chunks_it_reaches_and_the_world_rebuilds() {
        let mut s = streamer(2, 64 * 1024 * 1024);
        s.settle().unwrap();
        let before = s.len();
        assert!(before > 1, "a one-chunk window cannot show selective invalidation");

        let c = s.centre();
        let (x, z) = (c.x * 32 + 16, c.z * 32 + 16);
        let y = (c.y * 32..c.y * 32 + 32)
            .rev()
            .find(|y| !s.material_at(x, *y, z).unwrap().is_empty())
            .expect("the centre chunk's middle column is empty");
        let from = s.material_at(x, y, z).unwrap();
        let to = if from == materials::IGNEOUS { materials::AIR } else { materials::IGNEOUS };

        let reached: Vec<ChunkPos> = chunks_touching([x, y, z])
            .into_iter()
            .filter(|p| s.mesh(*p).is_some())
            .collect();
        assert_eq!(reached.len(), 1, "a mid-chunk voxel reaches one resident chunk");
        let digest_before = s.digest();

        let impact = s.edit(x, y, z, to, "the test dug here").unwrap();
        assert_eq!(impact.from, from);
        // **Selective still holds, in the currency sense:** only the reached
        // chunk stops being current. It is still *drawn* (the flash fix), so the
        // count that moved is `len` — which deliberately excludes stale.
        assert_eq!(s.len(), before - reached.len(), "invalidation was not selective");
        assert_eq!(s.stale_len(), reached.len(), "and what it invalidated went stale, not missing");
        assert_eq!(s.material_at(x, y, z).unwrap(), to, "the record did not take");

        s.settle().unwrap();
        assert_eq!(s.len(), before, "the dropped chunk did not come back");
        assert_ne!(s.digest(), digest_before, "the rebuilt world is the old world");
        s.shutdown();
    }

    #[test]
    fn a_picking_ray_finds_the_first_non_empty_voxel_and_names_the_face() {
        let s = streamer(2, 64 * 1024 * 1024);
        let c = s.centre();
        let (x, z) = (c.x * 32 + 16, c.z * 32 + 16);
        let y = (c.y * 32..c.y * 32 + 32)
            .rev()
            .find(|y| !s.material_at(x, *y, z).unwrap().is_empty())
            .expect("the centre chunk's middle column is empty");

        // Straight down the same column, from one voxel above the topmost solid
        // voxel — air by construction, so the ray passes through it and hits
        // `y`. World height is the **y** axis; moving in `-y` means entering
        // through the voxel's `+y` face.
        let ray = Ray::new([x as f64 + 0.5, y as f64 + 1.5, z as f64 + 0.5], [0.0, -1.0, 0.0]);
        let pick = s
            .pick(&ray, 8.0)
            .unwrap()
            .expect("the ray should find the voxel it starts above");
        assert_eq!(pick.voxel, [x, y, z]);
        assert_eq!(pick.entry_face, Some(Face::PosY));
        assert_eq!(pick.material, s.material_at(x, y, z).unwrap());

        // A ray into the sky finds nothing, and a degenerate one is a refusal to
        // pick rather than a panic.
        let sky = Ray::new([x as f64 + 0.5, y as f64 + 1.5, z as f64 + 0.5], [0.0, 1.0, 0.0]);
        assert_eq!(s.pick(&sky, 4.0).unwrap(), None);
        assert_eq!(s.pick(&Ray::new([0.0; 3], [0.0; 3]), 4.0).unwrap(), None);
        s.shutdown();
    }

    #[test]
    fn a_changed_world_drops_every_mesh_because_the_generator_is_the_only_source() {
        let mut s = streamer(4, 64 * 1024 * 1024);
        s.settle().unwrap();
        let before = s.digest();
        assert!(!s.is_empty());
        s.note_world_changed();
        assert_eq!(s.len(), 0);
        assert_eq!(s.budget().charged(), 0);
        // And it rebuilds to the same world, because the generator did not change.
        // That is the point of dropping everything rather than invalidating
        // selectively: nothing was lost, because nothing was stored that could not
        // be recomputed.
        s.settle().unwrap();
        assert_eq!(s.digest(), before, "the rebuilt world was not the same world");
        s.shutdown();
    }

    #[test]
    #[ignore = "measurement, not an assertion: run with `cargo test <filter> -- --ignored --nocapture`"]
    fn measure_the_throughput_of_the_pool() {
        // What the pool buys, and what it costs. The digest check at the end is
        // the assertion: four worker counts, one world (F12), measured rather
        // than argued.
        let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
        println!("available parallelism: {cores}");
        let w = ChunkWindow { radius_chunks: 3, vertical_chunks: 1 };
        println!("window: {} chunks (radius 3, vertical 1)", w.count());

        let mut digests = Vec::new();
        for workers in [1usize, 2, 4, 8] {
            let mut s = Streamer::new(source(), config(), workers, 512 * 1024 * 1024)
                .unwrap()
                .with_window(w)
                .unwrap();
            s.set_centre(centre());
            let t0 = std::time::Instant::now();
            let frame = s.settle().unwrap();
            let elapsed = t0.elapsed();
            println!(
                "  {workers:>2} worker(s): {} chunks in {elapsed:?} \
                 ({:.0} chunks/s, {:.1} ms/chunk), {:.2} MB resident + {:.1} MB scratch",
                frame.resident,
                frame.resident as f64 / elapsed.as_secs_f64(),
                elapsed.as_secs_f64() * 1000.0 / frame.resident.max(1) as f64,
                frame.bytes as f64 / (1024.0 * 1024.0),
                frame.scratch_bytes as f64 / (1024.0 * 1024.0),
            );
            digests.push((workers, s.digest()));
            s.shutdown();
        }
        let first = digests[0].1;
        for (workers, digest) in &digests {
            assert_eq!(*digest, first, "{workers} workers produced a different world");
        }
        println!("  one world at every worker count: {first}");
    }

    #[test]
    fn a_negative_window_is_refused_rather_than_read_as_empty() {
        let w = ChunkWindow { radius_chunks: -1, vertical_chunks: 0 };
        assert_eq!(w.validate().unwrap_err().kind(), RefusalKind::OutOfDeclaredRange);
    }

    #[test]
    fn the_window_is_a_box_and_the_count_says_so() {
        // A cube of radius 12 would be 15,625 chunks; the declared window is not.
        let w = DEFAULT_WINDOW;
        assert_eq!(w.count(), 25 * 25 * 7);
        assert!(w.count() < 25u64.pow(3));
        assert!(w.contains(ChunkPos::new(0, 0, 0), ChunkPos::new(12, 3, 12)));
        assert!(!w.contains(ChunkPos::new(0, 0, 0), ChunkPos::new(12, 4, 12)));
        assert!(!w.contains(ChunkPos::new(0, 0, 0), ChunkPos::new(13, 0, 0)));
    }

    #[test]
    fn the_window_radius_is_the_simulation_horizon_and_cannot_drift_from_it() {
        // The reason the window was widened, held as an assertion: if the horizon
        // moves and this number does not, the two neighbourhoods separate again
        // and nothing says so.
        let h = Horizons::declared();
        assert_eq!(DEFAULT_WINDOW.radius_chunks, h.rule(Horizon::Simulation).radius_chunks);
        // And it is strictly wider than what is drawn, which was the defect.
        assert!(DEFAULT_WINDOW.radius_chunks > h.rule(Horizon::Render).radius_chunks);
    }

    #[test]
    fn the_viewers_own_chunk_is_asked_for_first_and_the_farthest_chunk_last() {
        // Teeth for [Streamer::rank]. Remove the sort — leave the candidates in
        // scan order — and the first chunk offered becomes the window's
        // `(-r, -v, -r)` corner, because that is where `positions` starts. Every
        // assertion below is chosen to fail on that mutation, because "the pool
        // has a priority queue" is not the claim: the claim is *what gets refused
        // when the queue is full*, and the pool pops in priority order whatever
        // order it was handed.
        let s = streamer(1, 8 * 1024 * 1024).with_window(DEFAULT_WINDOW).unwrap();
        let c = centre();
        let mut candidates: Vec<ChunkPos> = s.window.positions(c).collect();
        let unranked_first = candidates[0];
        s.rank(&mut candidates, c);

        assert_eq!(
            candidates[0],
            c,
            "the chunk the viewer is standing in was not the first one asked for"
        );
        assert_ne!(
            unranked_first, c,
            "the scan happened to start at the centre, so this test cannot fail"
        );

        // The other end, stated as a maximum rather than as a named corner: ties
        // in priority are broken by position, so "the last chunk" is one of the
        // farthest, not a particular one.
        let farthest = candidates.iter().map(|p| s.window.priority(c, *p)).max().unwrap();
        assert_eq!(
            s.window.priority(c, *candidates.last().unwrap()),
            farthest,
            "the last chunk asked for is not one of the farthest"
        );
        s.shutdown();
    }

    #[test]
    fn everything_inside_the_render_horizon_is_asked_for_before_anything_outside_it() {
        // The tier half of the ruling, measured. A nearer chunk outside the render
        // horizon must lose to a farther chunk inside it, because the inner one is
        // the one the viewer can see into.
        let s = streamer(1, 8 * 1024 * 1024).with_window(DEFAULT_WINDOW).unwrap();
        let c = centre();
        let h = Horizons::declared();
        let mut candidates: Vec<ChunkPos> = s.window.positions(c).collect();
        s.rank(&mut candidates, c);

        let mut seen_outside = None;
        for (i, p) in candidates.iter().enumerate() {
            let inside = h.contains(Horizon::Render, c, *p);
            if !inside {
                seen_outside.get_or_insert(i);
            } else if let Some(first_outside) = seen_outside {
                // The rank is total, so once the first outside chunk is reached
                // no inside chunk may appear after it. This is the whole claim.
                assert!(
                    i < first_outside,
                    "chunk {p:?} is inside the render horizon but was ranked at {i}, \
                     after an outside chunk at {first_outside}"
                );
            }
        }
        // And the claim is not vacuous: the window really does straddle the
        // render horizon now, which is only true because it was widened.
        let inside_count =
            candidates.iter().filter(|p| h.contains(Horizon::Render, c, **p)).count();
        // The window is 25×7×25 and the render horizon is a radius-8 cube, which
        // spans all seven of its vertical chunks: so 17×7×17 inside, exactly.
        assert_eq!(candidates.len(), 25 * 25 * 7);
        assert_eq!(inside_count, 17 * 17 * 7);
        assert!(
            candidates.len() - inside_count > 0,
            "no candidate was outside the render horizon, so the tier cannot bite"
        );
        s.shutdown();
    }

    #[test]
    fn ranking_is_total_so_two_runs_ask_in_the_same_order() {
        // F12: an order that depended on iteration order would be a different
        // order on another machine, and the world would build differently there.
        let s = streamer(1, 8 * 1024 * 1024).with_window(DEFAULT_WINDOW).unwrap();
        let c = centre();
        let mut a: Vec<ChunkPos> = s.window.positions(c).collect();
        let mut b: Vec<ChunkPos> = a.clone();
        b.reverse();
        s.rank(&mut a, c);
        s.rank(&mut b, c);
        assert_eq!(a, b, "the same candidates ranked two ways came out in two orders");
        // The order is **lexicographic** in (tier, distance): the tier may drop exactly
        // once per ring boundary, and inside a tier the distance may never go
        // backwards. Stated as a monotone pair rather than "the tier is constant",
        // which would be a false claim — the tier is the primary key, so it is
        // supposed to change at each boundary.
        let h = Horizons::declared();
        for w in a.windows(2) {
            let (t0, t1) = (h.tier(c, w[0]), h.tier(c, w[1]));
            let (p0, p1) = (s.window.priority(c, w[0]), s.window.priority(c, w[1]));
            assert!(
                t0 > t1 || (t0 == t1 && p0 <= p1),
                "{w:?} then {:?} is not a monotone (tier, distance) order: \
                 ({t0}, {p0}) then ({t1}, {p1})",
                w[1]
            );
        }
        // And the tiers really do step down: all four are present in this window.
        let mut tiers: Vec<u8> = a.iter().map(|p| h.tier(c, *p)).collect();
        tiers.dedup();
        assert!(tiers.len() > 1, "the window only reached one tier, so tiering is untested");
        s.shutdown();
    }

    #[test]
    fn the_nearer_chunk_is_asked_for_first() {
        let w = DEFAULT_WINDOW;
        let c = ChunkPos::new(0, 0, 0);
        assert!(w.priority(c, ChunkPos::new(1, 0, 0)) < w.priority(c, ChunkPos::new(2, 0, 0)));
        assert!(w.priority(c, ChunkPos::new(0, 0, 0)) < w.priority(c, ChunkPos::new(0, 1, 0)));
    }
}
