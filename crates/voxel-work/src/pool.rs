//! The pool: N threads, one priority queue, one deterministic answer.
//!
//! # Why a worker needs no shared state at all
//!
//! A worker meshes a chunk **from the generator alone**. Its neighbourhood is
//! [`WorldSource::neighborhood`] — the world's own answer at the boundary, one
//! voxel at a time — so it needs no resident neighbour chunks to compare against,
//! no lock on the hot path, and no neighbour that might not have arrived yet.
//!
//! That is not merely convenient, it is what the memory measurement forces.
//! Answering a boundary question from resident chunks costs a whole voxel chunk
//! per neighbour: measured at **~89 kB each**, against **~1.3 kB** for the
//! finished mesh — 182×. A pool that held its own neighbourhood would spend more
//! memory on bookkeeping than on the world it is drawing.
//!
//! # Determinism, and what it costs
//!
//! F12: worker count is a performance knob and **never** changes output. Three
//! things make that true here: the world is a pure function of its inputs, the
//! mesher's scratch is per-worker so no two chunks can interleave, and
//! [`Pool::collect`] sorts what arrived before anyone looks at it. A test drives
//! the same requests through one worker and eight and compares digests.
//!
//! # Bounded, and honest about what cannot be cancelled
//!
//! The queue has a capacity. A camera that outruns the machine fills it, and
//! [`Submitted::QueueFull`] is the answer — a *report*, not an error, because
//! backpressure is the normal state of a streaming world and turning it into an
//! error would make every frame that is slightly behind a failure.
//!
//! [`Pool::cancel_where`] drops **queued** work immediately. Work already running
//! is **not** interrupted: one [`Mesher`]'s scratch belongs to its worker, is
//! ~4.8 MB of half-filled raster, and cannot be abandoned and reused by another
//! job. So an in-flight chunk finishes and its result is discarded by the caller —
//! bounded waste, chosen over a torn raster.
#[cfg(test)]
use voxel_model::endian::LeBytes;

use std::cmp::Reverse;
use std::collections::{BTreeSet, BinaryHeap};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::thread::JoinHandle;

use voxel_mesh::{Chunk, Mesh, MeshConfig, Mesher};
use voxel_model::{ChunkPos, MaterialId, Refusal, RefusalKind, Refused};
use voxel_shapers::earthworks::{hand_change, Modification};
use voxel_stream::WorldSource;

/// The most requests a pool will hold at once.
///
/// A bound, not a tuning: it is the number that turns "the camera outran the
/// machine" from an unbounded allocation into a value the HUD can show. Four
/// thousand requests at the measured ~1.3 kB per finished mesh is about 5 MB of
/// results, comfortably inside a budget that already charges 4.8 MB per worker.
pub const DEFAULT_QUEUE_CAPACITY: usize = 4096;

/// One unit of work: mesh this chunk.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Request {
    pub pos: ChunkPos,
    /// **Lower is sooner.** Distance-squared from the viewer, so the world fills
    /// outward from where somebody is standing rather than in scan order. A caller
    /// with a chunk that must jump the queue — one the picture is behind on after an
    /// edit — submits it with a lower number and [`Pool::submit`] re-keys the entry;
    /// see [`Submitted::Promoted`].
    pub priority: i64,
}

/// What happened to a submission.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Submitted {
    Queued,
    /// Already in the queue or already being built. Not an error: a caller that
    /// asks for the same chunk every frame is asking correctly.
    AlreadyQueued,
    /// **Already queued, and the new request carries a better priority**, so the
    /// entry was re-keyed. This is what gets a chunk the picture is behind on out
    /// of the queue ahead of the routine streaming it was slotted into: a caller
    /// re-asks for a stale chunk every frame, and the first ask may have been
    /// routine.
    Promoted,
    /// The queue is at capacity. Backpressure, reported.
    QueueFull,
}

/// A finished mesh and **which world** it was built from.
///
/// The generation is what makes an edit safe under concurrency. A worker reads
/// the record and the generation **under the same read lock**, so a mesh's
/// generation says exactly which record it saw; an edit takes the write lock,
/// records, and bumps the generation. A result whose generation is no longer
/// current was built from a world that no longer exists, and the caller
/// discards it rather than letting a stale chunk into a fresh world.
#[derive(Clone, PartialEq, Debug)]
pub struct Built {
    pub generation: u64,
    pub mesh: Mesh,
    /// **The chunk's own octree, handed back rather than dropped.**
    ///
    /// The mesher needs it and discards it — a worker's steady state is one tree plus one mesh —
    /// but the *world's light* needs it too: `voxel_client::field` marches a cell's beam through
    /// this tree, and a tree that has been thrown away cannot answer. It travels with the mesh
    /// because the two are one chunk's answer and a caller that has one and not the other has a
    /// mesh it cannot light (or a tree whose chunk is not on screen).
    ///
    /// **The cost is declared, not hidden**: about **89 kB per chunk** on top of the mesh's
    /// 90,481 B, which is the figure `voxel_work::pool`'s own note already carries. At the game's
    /// 605-chunk window that is about 54 MB more, inside `GAME_CEILING_BYTES`'s 256 MB.
    pub tree: voxel_mesh::SparseVoxelOctree,
}

/// What one [`Pool::collect`] returned.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Collected {
    /// Finished meshes, **sorted by chunk position** so the order is a property
    /// of the world rather than of which worker happened to finish first.
    pub meshes: Vec<Built>,
    /// Chunks the generator refused, with the refusal. Collected rather than
    /// swallowed, because a world with a hole in it should say why.
    pub failures: Vec<(ChunkPos, Refusal)>,
}

impl Collected {
    pub fn is_empty(&self) -> bool {
        self.meshes.is_empty() && self.failures.is_empty()
    }
}

/// A queue entry. `Reverse` because [`BinaryHeap`] is a *max*-heap and the
/// smallest priority must come out first; the position breaks ties so two jobs
/// of equal priority still have one order.
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
struct Entry {
    key: Reverse<(i64, i32, i32, i32)>,
    pos: ChunkPos,
}

struct Queue {
    pending: BinaryHeap<Entry>,
    queued: BTreeSet<ChunkPos>,
    /// Chunks a worker has **taken and not yet finished**. A position moves here
    /// from `queued` in the same critical section that pops it, so a re-ask
    /// while the build runs is answered [`Submitted::AlreadyQueued`] rather than
    /// buying the same chunk a second time.
    building: BTreeSet<ChunkPos>,
    capacity: usize,
    refused: u64,
}

impl Queue {
    /// Classify a submission against the queue's own state — the whole of
    /// [`Pool::submit`]'s decision, on data with no threads in it, so the four
    /// answers are unit-testable without timing.
    fn classify(&mut self, request: Request) -> Submitted {
        if self.queued.contains(&request.pos) {
            // **A better priority re-keys the entry.** The binary heap has no
            // decrement-key, so the old entry is removed and the new one pushed —
            // the queue is bounded by `capacity` and this runs for the edited
            // chunks a caller re-asks for, not for the queue as a whole.
            let key = (request.priority, request.pos.x, request.pos.y, request.pos.z);
            let mut better = false;
            self.pending.retain(|entry| {
                if entry.pos != request.pos {
                    return true;
                }
                better = key < entry.key.0;
                !better
            });
            if better {
                self.pending.push(Entry { key: Reverse(key), pos: request.pos });
                return Submitted::Promoted;
            }
            return Submitted::AlreadyQueued;
        }
        if self.building.contains(&request.pos) {
            // **Already under way.** A priority cannot reorder work that has
            // started, and queueing it again would pay for the same chunk twice
            // while the first build still runs — the duplicate the
            // [`Submitted::AlreadyQueued`] doc has always promised to report.
            return Submitted::AlreadyQueued;
        }
        if self.pending.len() >= self.capacity {
            self.refused += 1;
            return Submitted::QueueFull;
        }
        self.queued.insert(request.pos);
        self.pending.push(Entry {
            key: Reverse((request.priority, request.pos.x, request.pos.y, request.pos.z)),
            pos: request.pos,
        });
        Submitted::Queued
    }

    /// The worker's side of a job: the next entry, moved from `queued` to
    /// `building` **in one critical section**, so there is no instant where a
    /// taken chunk is in neither set.
    fn take(&mut self) -> Option<ChunkPos> {
        let entry = self.pending.pop()?;
        self.queued.remove(&entry.pos);
        self.building.insert(entry.pos);
        Some(entry.pos)
    }

    /// A build finished, either way: the chunk leaves `building`, so the next
    /// ask for it is fresh work — which is what lets a mesh built from a
    /// superseded world be rebuilt after the caller discards it.
    fn finished(&mut self, pos: ChunkPos) {
        self.building.remove(&pos);
    }
}

/// The mutable world: the record every worker reads and the version of it.
///
/// Kept together in one lock **on purpose**. Reading them separately would let a
/// worker see the new generation with the old record, or the old generation with
/// the new record — and either ordering silently breaks the "stale meshes are
/// discarded" rule.
struct World {
    source: WorldSource,
    generation: u64,
}

struct Shared {
    queue: Mutex<Queue>,
    /// Signalled when a request is queued or the pool closes.
    ready: Condvar,
    /// A lock with no data, so [`Pool::wait_until_idle`] can wait without
    /// holding the queue's lock — which would deadlock against the workers that
    /// need it to take their next job.
    idle_lock: Mutex<()>,
    /// Signalled when a worker finishes something or closes.
    idle: Condvar,
    results: Mutex<Vec<Built>>,
    /// Signalled when the caller drains results. With `result_capacity` this is
    /// what makes the pool's own footprint bounded even if nobody collects.
    room: Condvar,
    result_capacity: usize,
    failures: Mutex<Vec<(ChunkPos, Refusal)>>,
    running: AtomicUsize,
    closed: AtomicBool,
    world: RwLock<World>,
    config: MeshConfig,
    /// **The scratch the workers hold right now**, in bytes.
    ///
    /// Not `workers × one raster`: the rasters are held only while there is work
    /// (see [`work`]), so the settled pool's own footprint is zero and the number a
    /// HUD reads is the number the process is holding.
    scratch: AtomicUsize,
}

/// A bounded, deterministic pool of worker threads.
pub struct Pool {
    shared: Arc<Shared>,
    workers: Vec<JoinHandle<()>>,
}

/// A hand edit, **named rather than positional**.
///
/// Same reason as `voxel_shapers::earthworks::HandChange`: three coordinates in a row, two of
/// which share a type, is a call that can be transposed without the compiler noticing. Naming
/// the fields is the difference between a swapped coordinate being a type error and being a
/// hole in the world.
///
/// The `from` material is deliberately **absent**. This door reads it under the write lock, so
/// the record cannot be built from a material the world no longer holds — the chain connects by
/// construction rather than by the caller's care.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Edit {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub to: MaterialId,
    pub reason: String,
    pub actor: voxel_model::world::ActorId,
    pub verb: voxel_model::world::PhysicalVerb,
}

impl Pool {
    /// Start `workers` threads over `source`.
    ///
    /// Refuses zero workers rather than quietly building a pool that accepts
    /// work and never runs it — which is the worst possible failure, because
    /// everything a caller can observe says "queued".
    pub fn new(workers: usize, source: WorldSource, config: MeshConfig) -> Refused<Self> {
        Self::with_capacity(workers, source, config, DEFAULT_QUEUE_CAPACITY)
    }

    pub fn with_capacity(
        workers: usize,
        source: WorldSource,
        config: MeshConfig,
        capacity: usize,
    ) -> Refused<Self> {
        if workers == 0 {
            return Err(Refusal::new(
                RefusalKind::OutOfDeclaredRange,
                "a pool with 0 workers accepts work and never runs it, so it is refused rather than started",
            ));
        }
        let shared = Arc::new(Shared {
            queue: Mutex::new(Queue {
                pending: BinaryHeap::new(),
                queued: BTreeSet::new(),
                building: BTreeSet::new(),
                capacity,
                refused: 0,
            }),
            ready: Condvar::new(),
            idle_lock: Mutex::new(()),
            idle: Condvar::new(),
            results: Mutex::new(Vec::new()),
            room: Condvar::new(),
            result_capacity: capacity,
            failures: Mutex::new(Vec::new()),
            running: AtomicUsize::new(0),
            closed: AtomicBool::new(false),
            scratch: AtomicUsize::new(0),
            world: RwLock::new(World { source, generation: 0 }),
            config,
        });

        let mut handles = Vec::with_capacity(workers);
        for _ in 0..workers {
            let shared = Arc::clone(&shared);
            handles.push(std::thread::spawn(move || work(shared)));
        }
        Ok(Pool { shared, workers: handles })
    }

    /// Ask for a chunk. See [`Submitted`] for the four answers.
    pub fn submit(&self, request: Request) -> Submitted {
        let answer = {
            let mut q = self.shared.queue.lock().unwrap();
            if self.shared.closed.load(Ordering::SeqCst) {
                Submitted::QueueFull
            } else {
                q.classify(request)
            }
        };
        // **Only a new entry wakes a worker.** A promotion re-keys an entry some
        // worker was already signalled for, and a refusal changes nothing.
        if answer == Submitted::Queued {
            self.shared.ready.notify_one();
        }
        answer
    }

    /// Drop every queued request the predicate says is **no longer wanted**,
    /// returning how many were dropped.
    ///
    /// A predicate rather than a set, because the natural predicate — "inside the
    /// residency horizon of the viewer" — covers tens of thousands of chunks and
    /// materialising it every frame to test a few thousand queued ones would cost
    /// more than the cancellation saves.
    pub fn cancel_where(&self, keep: &dyn Fn(ChunkPos) -> bool) -> usize {
        let mut q = self.shared.queue.lock().unwrap();
        let before = q.pending.len();
        q.pending.retain(|e| keep(e.pos));
        q.queued.retain(|p| keep(*p));
        before - q.pending.len()
    }

    /// Queued plus running.
    pub fn in_flight(&self) -> usize {
        let queued = self.shared.queue.lock().unwrap().pending.len();
        queued + self.shared.running.load(Ordering::SeqCst)
    }

    /// Submissions refused because the queue was full, since the pool started.
    pub fn refused_submissions(&self) -> u64 {
        self.shared.queue.lock().unwrap().refused
    }

    pub fn workers(&self) -> usize {
        self.workers.len()
    }

    /// **The scratch the workers hold right now** — one raster each while there is
    /// work, none when the queue has drained. See [`work`] for why it is not
    /// `workers × raster`.
    pub fn scratch_bytes(&self) -> usize {
        self.shared.scratch.load(Ordering::SeqCst)
    }

    /// The most scratch this pool can hold: one raster per worker.
    ///
    /// What a caller budgeting for a *streaming* pool must set aside, which is not
    /// what it is holding at rest.
    pub fn scratch_ceiling(&self) -> usize {
        self.workers.len() * Mesher::new().scratch_bytes()
    }

    pub fn queue_capacity(&self) -> usize {
        self.shared.queue.lock().unwrap().capacity
    }

    /// The material at a world voxel, from the live record.
    pub fn material_at(&self, x: i32, y: i32, z: i32) -> Refused<MaterialId> {
        self.shared.world.read().unwrap().source.material_at(x, y, z)
    }

    /// **The world's own surface at one column**, read under **one** lock.
    ///
    /// The probe asks a handful of voxels of one column, and asking the world's lock per
    /// voxel would be a lock per voxel; the whole read is one question about one place, so
    /// it is taken once. See [`voxel_stream::WorldSource::column_surface`] for the rule.
    pub fn column_surface(&self, x: i32, z: i32) -> Refused<Option<f32>> {
        self.shared.world.read().unwrap().source.column_surface(x, z)
    }

    /// Which version of the world new chunks are built from.
    pub fn generation(&self) -> u64 {
        self.shared.world.read().unwrap().generation
    }

    /// **The world's modification record, as a snapshot** — the record itself, not a summary of it.
    ///
    /// A caller that kept its own list of edits would be a second record, and the two would
    /// disagree the first time one was dropped. The vector change view (the client's
    /// `shell::changes`) is *a view over the chain, not a second record* (C-56), so its regions
    /// are read from here rather than remembered at the edit sites.
    pub fn modifications(&self) -> Vec<Modification> {
        self.shared.world.read().unwrap().source.earthworks().entries().to_vec()
    }

    /// **Change the world.** One voxel, recorded in the earthworks ledger.
    ///
    /// `from` is read under the write lock rather than passed in, so the record
    /// cannot be built from a material the world no longer holds — the chain
    /// connects by construction. The generation is bumped in the same lock, so
    /// every worker either sees the whole edit or none of it.
    ///
    /// **Who, and what act.** The record names its actor and its verb, so this door
    /// asks for both rather than assuming them: `ActorId::player()` for the person at
    /// the console, and the [`PhysicalVerb`](voxel_model::world::PhysicalVerb) that
    /// says *which* act it was. A verb is not inferable from the target material —
    /// breaking stone and digging stone are different acts that replace the same
    /// block — so deriving it here would invent a field the record exists to hold.
    pub fn edit(&self, edit: Edit) -> Refused<Modification> {
        let mut world = self.shared.world.write().unwrap();
        let from = world.source.material_at(edit.x, edit.y, edit.z)?;
        // **A fluid is not a block** (C-152.2). `hand_change` refuses a fluid
        // *material* — the sea, a flooded void. This is the other half: a **lake**
        // is fluid state with **no material voxel**, so an edit into one would
        // delete water the ledger books while the materials all looked like air.
        // One query per edit, and only when the target is not already a fluid
        // material, so the common underground edit pays nothing (`fluid_state`
        // declares the query's cost).
        if !voxel_shapers::materials::is_fluid(from)
            && voxel_shapers::fluid_state::fluid_at(
                &world.source.spec(),
                edit.x,
                edit.y,
                edit.z,
                world.source.hydro_radius(),
            )?
            .is_some()
        {
            return Err(voxel_shapers::fluid_state::refuse_edit(edit.x, edit.y, edit.z));
        }
        let modification = hand_change(voxel_shapers::earthworks::HandChange {
            x: edit.x,
            y: edit.y,
            z: edit.z,
            from,
            to: edit.to,
            reason: edit.reason,
            actor: edit.actor,
            verb: edit.verb,
        })?;
        world
            .source
            .earthworks_mut()
            .record(modification.clone())
            .map_err(|set| {
                Refusal::owned(RefusalKind::OutOfDeclaredRange, format!("the earthworks record refused the change: {set}"))
            })?;
        world.generation += 1;
        Ok(modification)
    }

    /// Take everything that has finished.
    pub fn collect(&self) -> Collected {
        let meshes = {
            let mut r = self.shared.results.lock().unwrap();
            std::mem::take(&mut *r)
        };
        self.shared.room.notify_all();
        let failures = {
            let mut f = self.shared.failures.lock().unwrap();
            std::mem::take(&mut *f)
        };
        let mut meshes = meshes;
        meshes.sort_by_key(|built| (built.mesh.chunk.x, built.mesh.chunk.y, built.mesh.chunk.z));
        let mut failures = failures;
        failures.sort_by_key(|(p, _)| (p.x, p.y, p.z));
        Collected { meshes, failures }
    }

    /// Block until nothing is queued and nothing is running.
    ///
    /// **The caller must keep collecting while it waits**, or a worker blocked on
    /// a full result buffer can never finish and this never returns. Prefer
    /// [`Pool::wait_collecting`], which cannot deadlock because it drains the
    /// buffer itself. This one is for a caller that is already draining on its own
    /// thread, and for tests small enough that the buffer cannot fill.
    pub fn wait_until_idle(&self) {
        let mut guard = self.shared.idle_lock.lock().unwrap();
        loop {
            let busy = self.is_busy();
            if !busy {
                return;
            }
            guard = self.shared.idle.wait(guard).unwrap();
        }
    }

    /// Wait until nothing is in flight, **handing over each batch as it arrives**.
    ///
    /// Draining *inside* the wait is the whole point and not a convenience: a
    /// worker whose result buffer is full blocks in `deliver` and so never
    /// decrements its running count, which means a wait that did not drain could
    /// never end. The wait is a short timeout rather than a plain `wait`, because
    /// the worker that would have notified is precisely the one that is blocked.
    pub fn wait_collecting(&self, drain: &mut dyn FnMut(Collected)) {
        loop {
            let collected = self.collect();
            let had = !collected.is_empty();
            if had {
                drain(collected);
            }
            if !self.is_busy() {
                return;
            }
            let guard = self.shared.idle_lock.lock().unwrap();
            let _ = self.shared.idle.wait_timeout(guard, std::time::Duration::from_millis(1));
        }
    }

    fn is_busy(&self) -> bool {
        let queue = self.shared.queue.lock().unwrap();
        !queue.pending.is_empty() || self.shared.running.load(Ordering::SeqCst) > 0
    }

    /// Stop the workers and join them.
    ///
    /// Takes `self`, so "submitting to a stopped pool" is not a state that can be
    /// written down.
    pub fn shutdown(mut self) {
        self.shared.closed.store(true, Ordering::SeqCst);
        self.shared.ready.notify_all();
        self.shared.room.notify_all();
        self.shared.idle.notify_all();
        // `mem::take` rather than moving the field: `Pool` has a `Drop` impl, so
        // its fields cannot be moved out of it.
        for handle in std::mem::take(&mut self.workers) {
            let _ = handle.join();
        }
    }
}

impl Drop for Pool {
    /// A dropped pool closes. Without this a caller that never called
    /// [`Pool::shutdown`] would leak its threads for the life of the process, and
    /// a test that built pools in a loop would leak one thread set per test.
    fn drop(&mut self) {
        self.shared.closed.store(true, Ordering::SeqCst);
        self.shared.ready.notify_all();
        self.shared.room.notify_all();
    }
}

/// One worker: take a job, mesh it, hand it over. Repeat.
fn work(shared: Arc<Shared>) {
    // **One raster, held only while there is work.** Never shared, which is why there is no
    // lock on the hot path; kept across jobs and given back when the queue drains, so a
    // settled world holds none of the 4.8 MB a raster costs.
    let mut mesher: Option<Mesher> = None;
    while let Some(pos) = next_request(&shared) {
        let raster = mesher.get_or_insert_with(|| {
            let made = Mesher::new();
            shared.scratch.fetch_add(made.scratch_bytes(), Ordering::SeqCst);
            made
        });
        let built = build_one(raster, &shared, pos);
        match built {
            Ok(built) => deliver(&shared, built),
            Err(refusal) => shared.failures.lock().unwrap().push((pos, refusal)),
        }
        // **The chunk is no longer being built**, so the next ask for it is fresh
        // work rather than a report that it is already under way. A mesh built
        // from a superseded world is discarded by the caller as stale, and this is
        // what lets its position be queued afresh instead of the duplicate being
        // bought while the doomed build was still running.
        shared.queue.lock().unwrap().finished(pos);
        // **After** the hand-off, never before. `wait_until_idle` reads this to
        // decide the work is done, so a decrement that happened between finishing
        // and delivering would let a caller look at the results and find them
        // empty while a mesh was in flight.
        shared.running.fetch_sub(1, Ordering::SeqCst);
        shared.idle.notify_all();
        // **The last job hands the scratch back**, checked under the queue's own lock so a
        // submit either woke a worker for a job it still has or rebuilds the raster.
        if let Some(held) = mesher.as_ref() {
            if shared.queue.lock().unwrap().pending.is_empty() {
                let bytes = held.scratch_bytes();
                mesher = None;
                shared.scratch.fetch_sub(bytes, Ordering::SeqCst);
            }
        }
    }
}

fn next_request(shared: &Shared) -> Option<ChunkPos> {
    let mut queue = shared.queue.lock().unwrap();
    loop {
        if shared.closed.load(Ordering::SeqCst) {
            return None;
        }
        if let Some(pos) = queue.take() {
            // Counted as running **inside the lock and in the same instant as the
            // pop and the building mark**, so there is no window in which a job is
            // taken but nothing says it is in flight.
            shared.running.fetch_add(1, Ordering::SeqCst);
            return Some(pos);
        }
        queue = shared.ready.wait(queue).unwrap();
    }
}

/// Hand a finished mesh over, **waiting** when the buffer is full.
///
/// Waiting rather than dropping: dropping would lose work that has already been
/// paid for, and the wait is what bounds the buffer. On close it gives up without
/// blocking, so [`Pool::shutdown`] can always join.
fn deliver(shared: &Shared, built: Built) {
    let mut results = shared.results.lock().unwrap();
    while results.len() >= shared.result_capacity && !shared.closed.load(Ordering::SeqCst) {
        results = shared.room.wait(results).unwrap();
    }
    if shared.closed.load(Ordering::SeqCst) {
        return;
    }
    results.push(built);
}

/// Mesh one chunk **from the generator alone**.
///
/// The chunk is built, meshed, and dropped: nothing but the mesh survives. That
/// is the whole memory argument of this crate in three lines — a worker's
/// steady-state footprint is one chunk's octree (~89 kB) plus one mesh
/// (~1.3 kB), not a resident world.
fn build_one(mesher: &mut Mesher, shared: &Shared, pos: ChunkPos) -> Refused<Built> {
    // **A snapshot, then a lock-free build.** The read guard is held for one
    // clone, not for the whole build — and that is a measured decision, not a
    // preference. The guard used to span `build_chunk`, `neighborhood` and
    // `Mesher::build`, and one `build_chunk` costs **163 ms** (measured:
    // `voxel_work::stream::tests::measure_the_edit_hitch_and_where_it_comes_from`),
    // so every edit's write lock waited for the builds in flight: a click that
    // costs **46 µs** against an idle pool cost **153–525 ms** against a busy
    // one, which is the freeze a player feels while the window streams.
    //
    // The snapshot is consistent by construction: the record and the generation
    // are read under one lock, so a build still sees exactly one world. An edit
    // that lands mid-build is caught by the generation stamp exactly as before —
    // the finished mesh is discarded as stale rather than entering a world it was
    // not built from. The cost is the clone: the spec is small, and the
    // earthworks record is already scanned per voxel by `build_chunk` itself, so
    // one more copy per chunk is not what the history costs.
    let (source, generation) = {
        let world = shared.world.read().unwrap();
        (world.source.clone(), world.generation)
    };
    let tree = source.build_chunk(pos)?;
    let chunk = Chunk::from_tree(pos, tree.clone());
    let neighborhood = source.neighborhood(pos);
    let mesh = mesher.build(&chunk, &shared.config, Some(&neighborhood))?;
    Ok(Built { generation, mesh, tree })
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_model::Digest;
    use voxel_shapers::materials;
    use voxel_shapers::{Config, WorldSpec};

    fn source() -> WorldSource {
        WorldSource::new(WorldSpec::new(2468, Config::default()))
    }

    fn config() -> MeshConfig {
        MeshConfig::from_registry(&materials::registry().unwrap()).unwrap()
    }

    fn pool(workers: usize) -> Pool {
        Pool::new(workers, source(), config()).unwrap()
    }

    /// A small block of chunks near the origin's surface.
    fn requests(n: i32) -> Vec<Request> {
        let mut out = Vec::new();
        for x in 0..n {
            for z in 0..n {
                for y in 0..n {
                    let pos = ChunkPos::new(x, y, z);
                    out.push(Request { pos, priority: (x * x + y * y + z * z) as i64 });
                }
            }
        }
        out.sort_by_key(|r| r.priority);
        out
    }

    /// Drain a pool of a known set of requests, and digest the result.
    fn run(workers: usize, requests: &[Request]) -> (Digest, usize) {
        let pool = pool(workers);
        for r in requests {
            pool.submit(*r);
        }
        let mut meshes = Vec::new();
        // Bounded, so a pool that never settled would fail this test rather than
        // hang the suite.
        for _ in 0..1_000 {
            pool.wait_until_idle();
            let collected = pool.collect();
            if collected.meshes.is_empty() && collected.failures.is_empty() {
                break;
            }
            meshes.extend(collected.meshes.into_iter().map(|built| built.mesh));
        }
        pool.shutdown();
        meshes.sort_by_key(|m| (m.chunk.x, m.chunk.y, m.chunk.z));
        let mut t = Vec::new();
        for m in &meshes {
            t.extend_from_slice(&m.chunk.x.into_le_bytes());
            t.extend_from_slice(&m.chunk.y.into_le_bytes());
            t.extend_from_slice(&m.chunk.z.into_le_bytes());
            t.extend_from_slice(m.digest().as_bytes());
        }
        (Digest::of_fields("voxel-work/pool-test", &[t.as_slice()]), meshes.len())
    }

    #[test]
    fn a_pool_with_no_workers_is_refused_rather_than_started() {
        // `.err()` rather than `.unwrap_err()`, because a `Pool` holds join handles
        // and is deliberately not `Debug` — printing one would say nothing.
        let r = Pool::new(0, source(), config()).err().expect("a pool with no workers was started");
        assert_eq!(r.kind(), RefusalKind::OutOfDeclaredRange);
        assert!(r.detail().contains("never runs it"));
    }

    #[test]
    fn the_same_requests_give_the_same_world_at_one_and_at_eight_workers() {
        // F12, finally as a test rather than as an intention: the worker count is
        // a performance knob. If this ever fails, the world depends on the
        // machine it was built on, and every digest in the ledger stops meaning
        // anything.
        let reqs = requests(3);
        let (one, count_one) = run(1, &reqs);
        let (eight, count_eight) = run(8, &reqs);
        assert_eq!(count_one, count_eight, "different numbers of chunks came back");
        assert_eq!(one, eight, "the world depended on the worker count");
    }

    #[test]
    fn the_same_requests_give_the_same_meshes_twice_at_the_same_worker_count() {
        let reqs = requests(2);
        let (a, _) = run(4, &reqs);
        let (b, _) = run(4, &reqs);
        assert_eq!(a, b);
    }

    fn queue() -> Queue {
        Queue {
            pending: BinaryHeap::new(),
            queued: BTreeSet::new(),
            building: BTreeSet::new(),
            capacity: 8,
            refused: 0,
        }
    }

    #[test]
    fn a_better_priority_re_keys_a_queued_chunk_and_a_worse_one_does_not() {
        let mut q = queue();
        let pos = ChunkPos::new(4, 0, -2);
        assert_eq!(q.classify(Request { pos, priority: 900 }), Submitted::Queued);
        assert_eq!(
            q.classify(Request { pos, priority: 5 }),
            Submitted::Promoted,
            "a better priority must re-key the entry"
        );
        assert_eq!(q.pending.len(), 1, "a promotion holds one entry, not two");
        assert_eq!(
            q.pending.peek().expect("one entry").key.0,
            (5, 4, 0, -2),
            "the entry's key is the promoted one"
        );
        assert_eq!(
            q.classify(Request { pos, priority: 5 }),
            Submitted::AlreadyQueued,
            "the same ask again is not a promotion"
        );
        assert_eq!(q.classify(Request { pos, priority: 900 }), Submitted::AlreadyQueued);
        assert_eq!(q.pending.len(), 1);
    }

    #[test]
    fn a_chunk_already_being_built_is_not_queued_a_second_time() {
        let mut q = queue();
        let pos = ChunkPos::new(1, 0, 1);
        assert_eq!(q.classify(Request { pos, priority: 3 }), Submitted::Queued);
        assert_eq!(q.take(), Some(pos), "the worker takes it");
        assert_eq!(
            q.classify(Request { pos, priority: 0 }),
            Submitted::AlreadyQueued,
            "a running build cannot be reordered, and it must not be bought twice"
        );
        assert!(q.pending.is_empty(), "the answer queued a duplicate");
        q.finished(pos);
        assert_eq!(
            q.classify(Request { pos, priority: 0 }),
            Submitted::Queued,
            "after the build finishes the chunk is queued afresh — the rebuild a discarded stale mesh needs"
        );
    }

    #[test]
    fn a_full_queue_refuses_further_requests_instead_of_growing() {
        let pool = Pool::with_capacity(1, source(), config(), 4).unwrap();
        let mut queued = 0;
        let mut full = 0;
        for req in requests(4) {
            match pool.submit(req) {
                Submitted::Queued => queued += 1,
                Submitted::QueueFull => full += 1,
                Submitted::AlreadyQueued | Submitted::Promoted => {}
            }
        }
        assert!(full > 0, "a 4-slot queue accepted {} requests", queued);
        assert!(queued <= 4, "the queue grew past its capacity: {queued}");
        assert_eq!(pool.refused_submissions(), full);
        pool.shutdown();
    }

    #[test]
    fn asking_twice_for_a_chunk_is_reported_rather_than_duplicated() {
        let pool = Pool::with_capacity(1, source(), config(), 64).unwrap();
        let req = Request { pos: ChunkPos::new(0, 0, 0), priority: 0 };
        assert_eq!(pool.submit(req), Submitted::Queued);
        assert_eq!(pool.submit(req), Submitted::AlreadyQueued);
        pool.shutdown();
    }

    #[test]
    fn a_cancelled_request_is_not_built_and_an_in_flight_one_still_arrives() {
        // With one worker, the first request is running and the rest are queued.
        let pool = Pool::with_capacity(1, source(), config(), 64).unwrap();
        let reqs = requests(4);
        for r in &reqs {
            pool.submit(*r);
        }
        let keep = BTreeSet::from([reqs[0].pos]);
        let dropped = pool.cancel_where(&|p| keep.contains(&p));
        assert!(dropped > 0, "nothing was cancelled");
        pool.wait_until_idle();
        let collected = pool.collect();
        // Whatever was already running may still arrive; nothing else may.
        for built in &collected.meshes {
            assert!(
                keep.contains(&built.mesh.chunk),
                "a cancelled chunk was built anyway: {:?}",
                built.mesh.chunk
            );
        }
        pool.shutdown();
    }

    #[test]
    fn a_pool_holds_its_scratch_only_while_it_has_work() {
        // Two numbers: the ceiling a streaming caller must set aside, and what is held now.
        let pool = pool(4);
        assert_eq!(pool.scratch_bytes(), 0, "an idle pool held a raster");
        assert_eq!(
            pool.scratch_ceiling(),
            4 * Mesher::new().scratch_bytes(),
            "the bound is one raster a worker"
        );
        for r in requests(2) {
            pool.submit(r);
        }
        let mut held = false;
        for _ in 0..200 {
            if pool.scratch_bytes() > 0 {
                held = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert!(held, "no worker held a raster while a build was under way");
        pool.wait_until_idle();
        let _ = pool.collect();
        for _ in 0..500 {
            if pool.scratch_bytes() == 0 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert_eq!(pool.scratch_bytes(), 0, "a settled pool kept its scratch");
        // And the pool still works after giving it back.
        for r in requests(1) {
            pool.submit(r);
        }
        pool.wait_until_idle();
        let again = pool.collect();
        assert!(
            !again.meshes.is_empty() || !again.failures.is_empty(),
            "the pool did not build after re-allocating its raster"
        );
        pool.shutdown();
    }

    #[test]
    fn a_shut_down_pool_joins_its_threads() {
        // The evidence is that this returns at all: `shutdown` joins, so a worker
        // that never noticed the close would hang here rather than fail.
        let pool = pool(4);
        for r in requests(2) {
            pool.submit(r);
        }
        pool.shutdown();
    }

    #[test]
    fn collect_returns_one_order_however_the_workers_finished() {
        let pool = pool(2);
        for r in requests(3) {
            pool.submit(r);
        }
        pool.wait_until_idle();
        let collected = pool.collect();
        let mut sorted = collected.meshes.clone();
        sorted.sort_by_key(|built| (built.mesh.chunk.x, built.mesh.chunk.y, built.mesh.chunk.z));
        assert_eq!(collected.meshes, sorted, "collect must return chunk order");
        pool.shutdown();
    }

    #[test]
    fn an_edit_is_recorded_and_takes_effect_at_once() {
        let pool = pool(2);
        // A voxel deep in the ground, where the generator itself is solid: the
        // edit replaces matter rather than filling air, so `from` is a real
        // material and the chain has something to connect to.
        let (x, y, z) = (2, 0, 3);
        let from = pool.material_at(x, y, z).unwrap();
        assert!(!from.is_empty(), "the test voxel starts empty; pick one in the ground");
        let to = if from == materials::IGNEOUS { materials::REGOLITH } else { materials::IGNEOUS };
        assert_eq!(pool.generation(), 0);

        let impact =        pool.edit(Edit {
            x,
            y,
            z,
            to,
            reason: "the test dug here".into(),
            actor: voxel_model::world::ActorId::player(),
            verb: voxel_model::world::PhysicalVerb::Dig,
        })
        .unwrap();
        assert_eq!(impact.from, from, "the record's `from` must be what the world held");
        assert_eq!(pool.material_at(x, y, z).unwrap(), to);
        assert_eq!(pool.generation(), 1, "an edit must move the generation");
        // The record holds it, and its ledger audits.
        assert!(impact.ledger.audit().is_ok(), "the edit's ledger did not audit");
        pool.shutdown();
    }

    #[test]
    fn an_edit_that_targets_a_fluid_is_refused_and_changes_nothing() {
        // C-152.2's filed defect at the edit path's own door: the sea cannot be
        // broken. The record stays empty and the generation does not move, because
        // a refused edit is not an edit.
        let pool = pool(1);
        let spec = WorldSpec::new(2468, Config::default());
        let mut found = None;
        'search: for x in (-6000..6000).step_by(53) {
            for z in (-6000..6000).step_by(97) {
                if voxel_shapers::terrain::elevation_m(&spec, x as f64 + 0.5, z as f64 + 0.5)
                    < spec.config.sea_level_m - 4.0
                {
                    found = Some((x, z));
                    break 'search;
                }
            }
        }
        let (x, z) = found.expect("this world has no sea");
        let y = (spec.config.sea_level_m - 1.0).floor() as i32;
        assert_eq!(pool.material_at(x, y, z).unwrap(), materials::WATER);
        let before = pool.generation();

        let refusal = pool
            .edit(Edit {
                x,
                y,
                z,
                to: MaterialId::EMPTY,
                reason: "the test broke the sea".into(),
                actor: voxel_model::world::ActorId::player(),
                verb: voxel_model::world::PhysicalVerb::Break,
            })
            .unwrap_err();
        assert_eq!(refusal.kind(), RefusalKind::FluidIsNotABlock);
        assert!(refusal.detail().contains("targets water"), "got {refusal}");

        // The world did not move: same material, same generation, no record.
        assert_eq!(pool.material_at(x, y, z).unwrap(), materials::WATER);
        assert_eq!(pool.generation(), before, "a refused edit moved the generation");
        assert!(pool.modifications().is_empty(), "a refused edit was recorded");
        pool.shutdown();
    }

    #[test]
    fn a_placement_into_a_lake_is_refused_because_the_lake_has_no_material() {
        // The derived half: a lake voxel is **air with fluid state above it**, so
        // `hand_change` cannot see it (it holds no spec) and a placement would
        // silently delete water the ledger books. The pool is where the lake is
        // refused, and this test picks a lake by the world's own priority flood.
        let pool = pool(1);
        let spec = WorldSpec::new(2468, Config::default());
        // The grid and the threshold are the ones `fluid_state`'s own tests measured:
        // over this world's above-sea columns (step 101) the deepest fill is ~2.04 m
        // and thirteen exceed a metre.
        let mut found = None;
        'search: for x in (-6000..6000).step_by(101) {
            for z in (-6000..6000).step_by(101) {
                let height =
                    voxel_shapers::terrain::elevation_m(&spec, x as f64 + 0.5, z as f64 + 0.5);
                if height < spec.config.sea_level_m + 1.0 {
                    continue;
                }
                let sample = voxel_shapers::hydrology::sample(
                    &spec,
                    x as f64 + 0.5,
                    z as f64 + 0.5,
                    voxel_shapers::materials::DEFAULT_HYDRO_RADIUS,
                );
                if sample.water_depth_m > 1.0 {
                    found = Some((x, z, height, sample.water_depth_m));
                    break 'search;
                }
            }
        }
        let (x, z, height, depth) = found.expect("no lake deeper than a metre in the window");
        // The voxel the lake occupies is **above** the ground and holds no material:
        // the first voxel whose centre is above the terrain, which a depth over a
        // metre puts inside the water.
        let y = (height - 0.5).floor() as i32 + 1;
        assert!(
            f64::from(y) + 0.5 > height && f64::from(y) + 0.5 <= height + depth,
            "the chosen voxel is not inside the lake (height {height}, depth {depth}, y {y})"
        );
        assert_eq!(
            pool.material_at(x, y, z).unwrap(),
            MaterialId::EMPTY,
            "a lake voxel should hold no material, or this test measures the material rule"
        );

        let refusal = pool
            .edit(Edit {
                x,
                y,
                z,
                to: materials::REGOLITH,
                reason: "the test filled a lake".into(),
                actor: voxel_model::world::ActorId::player(),
                verb: voxel_model::world::PhysicalVerb::Fill,
            })
            .unwrap_err();
        assert_eq!(refusal.kind(), RefusalKind::FluidIsNotABlock);
        assert!(refusal.detail().contains("targets water"), "got {refusal}");
        assert!(pool.modifications().is_empty(), "a refused edit was recorded");
        pool.shutdown();
    }

    #[test]
    fn a_mesh_built_after_an_edit_carries_the_new_generation() {
        // The mechanism stale-mesh discarding rests on: the stamp travels with
        // the mesh, so a caller can tell which world it belongs to.
        let pool = pool(2);
        let from = pool.material_at(2, 0, 3).unwrap();
        let to = if from == materials::REGOLITH { materials::IGNEOUS } else { materials::REGOLITH };
        pool.edit(Edit {
            x: 2,
            y: 0,
            z: 3,
            to,
            reason: "the test built here".into(),
            actor: voxel_model::world::ActorId::player(),
            verb: voxel_model::world::PhysicalVerb::Place,
        })
        .unwrap();
        let generation = pool.generation();
        pool.submit(Request { pos: ChunkPos::new(0, 0, 0), priority: 0 });
        pool.wait_until_idle();
        let collected = pool.collect();
        assert_eq!(collected.meshes.len(), 1);
        assert_eq!(collected.meshes[0].generation, generation);
        pool.shutdown();
    }

    #[test]
    fn a_world_with_no_ground_still_meshes_every_chunk_it_was_asked_for() {
        // A parcel of pure sky: the meshes are empty, which is a legitimate
        // answer and must not be confused with a missing one.
        let pool = pool(2);
        let reqs: Vec<Request> = (0..2)
            .flat_map(|x| {
                (0..2).map(move |z| Request { pos: ChunkPos::new(x, 99, z), priority: 0 })
            })
            .collect();
        for r in &reqs {
            pool.submit(*r);
        }
        pool.wait_until_idle();
        let collected = pool.collect();
        assert_eq!(collected.meshes.len(), 4);
        assert!(collected.failures.is_empty(), "{:?}", collected.failures);
        for built in &collected.meshes {
            assert_eq!(built.mesh.quads.len(), 0, "sky should emit nothing");
        }
        pool.shutdown();
    }
}
