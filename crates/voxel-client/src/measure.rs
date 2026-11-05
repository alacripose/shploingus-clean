//! **The world's light, measured on a thread of its own.**
//!
//! [`crate::field::CellField`] answers a corner in a hash lookup, and this is where the answer
//! comes from. One worker owns a [`WorldLight`] derived from the frame's own `(spec, tick,
//! inputs)` triple — the same pure function, so its beams are the frame's beams and not a second
//! opinion about the sky — marches each cell the frame asked for through the resident octrees, and
//! hands back the six bands with the direction they were marched under.
//!
//! # Why it is not the frame's work
//!
//! A march is about ten microseconds (the shipped binary: `measured-light`, 128 cells in 1.2 ms
//! over a 160 m reach), and the resident window's surface is about eighty thousand cells. On the
//! frame that is eight hundred milliseconds of marches, and the draft's version of this spent them
//! a thousand cells at a time — ten milliseconds on a rewrite frame, ten of those a second while
//! the field filled. The frame is held to 8.1 ms settled; a field that costs a frame anything at
//! all while it fills is the defect this module exists to remove.
//!
//! # The trees travel by handle, and the frame answers the door
//!
//! A march crosses chunks, so the worker needs trees the streamer holds — and the streamer lives
//! on the frame thread. The trees are `Arc<SparseVoxelOctree>` ([`voxel_work::Streamer::tree_shared`]),
//! so handing one over is a refcount and no bytes. The worker **asks for the trees it lacks** on
//! one channel and blocks; the frame drops up to a few into the reply channel each frame
//! ([`Measurer::service`]). Nothing here can stall a frame: the frame never waits for the worker,
//! and a frame that answers no requests only makes the worker later.
//!
//! **The worker holds a bounded cache of those handles** ([`CACHE_CHUNKS`]) and clears it when the
//! world's generation or the eye's ask changes. A tree the streamer has since evicted stays alive
//! until then — a bounded number of chunks that the streamer's budget no longer charges, which is
//! why the bound is small and stated rather than generous.
//!
//! # One ask in flight, and the queue is the field's
//!
//! [`Measurer::ask`] is refused by the caller when [`Measurer::busy`]; the field's queue drains in
//! batches ([`Measurer::ask_for`] takes up to [`CELLS_PER_ASK`]), so a beam step or a walk replaces
//! demand at the field rather than racing a second solver in the worker. Results arrive in batches
//! and are adopted by the frame's field; a cell is never adopted as a measurement of a beam it was
//! not marched under.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::sync::Arc;

use voxel_appearance::sky::SkyInputs;
use voxel_mesh::SparseVoxelOctree;
use voxel_model::ChunkPos;
use voxel_shapers::WorldSpec;

use crate::field::{ChunkTrees, SubvoxelPos, REACH_M};
use crate::skylight::WorldLight;

/// How many cells one ask carries.
///
/// About forty milliseconds of marches — a batch small enough that a beam step behind it is a step
/// and not a stall, and large enough that the queue drains at a useful rate.
pub const CELLS_PER_ASK: usize = 4_096;

/// How many results the worker sends in one message.
const BATCH: usize = 512;

/// How many chunks' trees the worker keeps after the marches that needed them.
///
/// **Above the resident window's own count**, and that is the number that matters: a march asks
/// for the chunk it starts in and the chunks its rays cross, so a cache smaller than the window
/// is a cache that misses on the next cell's chunk and pays a frame's round trip for it — the
/// first version held 64 and measured 512 cells a second instead of 50 000. The entries are
/// `Arc` handles on the streamer's own trees, so a full cache is kilobytes of refcounts rather
/// than a second copy of the world; the bound is there so walking into new ground cannot grow it
/// without limit.
const CACHE_CHUNKS: usize = 1_024;

/// One cell measured, and the beam it was measured under.
#[derive(Clone, Copy, Debug)]
pub struct Measured {
    /// The cell, by [`SubvoxelPos`] — the one address law, at the grain the shadow is drawn at.
    pub cell: SubvoxelPos,
    /// The six bands, in the vertex format's order: direct B3/B4/B5, then diffuse.
    pub bands: [f32; 6],
    /// The direction the solver's beam came from when this cell was marched.
    pub toward: [f64; 3],
}

/// What the worker is asked to measure, at the tick the frame's field was derived at.
#[derive(Clone, Copy, Debug)]
pub struct Ask {
    /// The world the beams belong to.
    pub spec: WorldSpec,
    /// The tick the solver is derived at — the atmosphere's cadence tick, not the frame's.
    pub tick: u64,
    /// The sky's two inputs, as the frame's own atmosphere read them.
    pub inputs: SkyInputs,
    /// The new-moon tick the Moon's phase is read at.
    pub new_moon_tick: u64,
    /// The world record's generation: an edit invalidates every tree the worker holds.
    pub generation: u64,
    /// Where the eye is, so the queue is marched from the near field outwards.
    pub eye: [f64; 3],
}

/// The measurer: a worker thread, and the three channels the frame uses to feed it and take its
/// answers.
pub struct Measurer {
    asks: Sender<AskBatch>,
    tree_asks: Receiver<ChunkPos>,
    tree_replies: Sender<(ChunkPos, Option<Arc<SparseVoxelOctree>>)>,
    results: Receiver<Batch>,
    /// Cells asked for and not yet answered — the frame's only reason to hold a second ask.
    in_flight: usize,
}

impl Measurer {
    /// Start the worker. It lives as long as this handle: dropping the handle closes the channels
    /// and the thread ends at its next receive.
    pub fn new() -> Self {
        let (asks, ask_rx) = std::sync::mpsc::channel::<AskBatch>();
        let (tree_tx, tree_asks) = std::sync::mpsc::channel::<ChunkPos>();
        let (tree_replies, tree_rx) =
            std::sync::mpsc::channel::<(ChunkPos, Option<Arc<SparseVoxelOctree>>)>();
        let (result_tx, result_rx) = std::sync::mpsc::channel::<Batch>();
        std::thread::Builder::new()
            .name("light".into())
            .spawn(move || run(ask_rx, tree_tx, tree_rx, result_tx))
            .expect("a thread for the world's light");
        Measurer {
            asks,
            tree_asks,
            tree_replies,
            results: result_rx,
            in_flight: 0,
        }
    }

    /// Whether an ask is still being marched. The caller composes the next one when this is false,
    /// so the worker has one solver and one queue rather than a race between two.
    pub fn busy(&self) -> bool {
        self.in_flight > 0
    }

    /// Cells asked for and not yet answered. The number behind [`Measurer::busy`], reported so a
    /// stalled ask is visible rather than inferred from a field that stopped growing.
    pub fn in_flight(&self) -> usize {
        self.in_flight
    }

    /// Hand the worker a batch of cells to measure, **with the chunks that have a tree in hand**.
    ///
    /// The residency set travels with the ask because the alternative is a round trip per hop:
    /// a beam leaves most cells upward, where the world has no chunk, and a worker that has to
    /// ask this thread about every one of those chunks spends a frame discovering that the sky is
    /// empty. With the set in hand it is a hash lookup, and a resident chunk it does not hold yet
    /// is one request for 89 kB of refcount. Ignored by the caller when [`Measurer::busy`].
    pub fn ask_for(&mut self, ask: Ask, cells: Vec<SubvoxelPos>, resident: Vec<ChunkPos>) {
        if cells.is_empty() {
            return;
        }
        self.in_flight += cells.len();
        let _ = self.asks.send(AskBatch {
            ask,
            cells,
            resident,
        });
    }

    /// **Answer the worker's own questions, up to `budget` trees a frame.**
    ///
    /// `tree` is the frame's provider — the streamer, in the game. A chunk it does not hold is
    /// answered as `None`, which the march counts as an unresolved hop rather than as clear air.
    pub fn service(
        &mut self,
        budget: usize,
        tree: impl Fn(ChunkPos) -> Option<Arc<SparseVoxelOctree>>,
    ) -> usize {
        let mut served = 0;
        for _ in 0..budget {
            match self.tree_asks.try_recv() {
                Ok(pos) => {
                    let _ = self.tree_replies.send((pos, tree(pos)));
                    served += 1;
                }
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
        served
    }

    /// **Take the measurements that have landed**, up to `limit` cells. The remainder stays
    /// queued, so a frame adopts one batch and the next frame adopts the next.
    pub fn adopt(&mut self, limit: usize) -> Vec<Measured> {
        let mut out = Vec::new();
        while out.len() < limit {
            match self.results.try_recv() {
                Ok(batch) => {
                    self.in_flight = self.in_flight.saturating_sub(batch.measured.len());
                    out.extend(batch.measured);
                    // **The end of an ask is stated, not inferred.** A batch whose cells were all
                    // refused (no solver, or no world under the cell) sends nothing measurable, so
                    // a count alone would leave the ask — and the next one — waiting forever.
                    if batch.last {
                        self.in_flight = 0;
                        break;
                    }
                }
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
        out
    }
}

impl Default for Measurer {
    fn default() -> Self {
        Self::new()
    }
}

/// What crosses the ask channel: the beam, the cells, and which chunks this thread can answer for.
///
/// Kept apart from [`Ask`] so the beam stays a plain `Copy` record, and the two lists travel once
/// rather than per cell.
struct AskBatch {
    ask: Ask,
    cells: Vec<SubvoxelPos>,
    resident: Vec<ChunkPos>,
}

/// What crosses the results channel: some measurements, and whether they end the ask.
struct Batch {
    measured: Vec<Measured>,
    last: bool,
}

/// The worker's loop: derive a solver when its arguments change, then march the batch it was
/// given, nearest cell first, and send the answers in bounded pieces.
fn run(
    asks: Receiver<AskBatch>,
    tree_tx: Sender<ChunkPos>,
    tree_rx: Receiver<(ChunkPos, Option<Arc<SparseVoxelOctree>>)>,
    results: Sender<Batch>,
) {
    let cache: RefCell<HashMap<ChunkPos, Arc<SparseVoxelOctree>>> = RefCell::new(HashMap::new());
    let mut solver: Option<WorldLight> = None;
    let mut solver_key: Option<(u64, u64, [u64; 2], u64)> = None;
    let mut generation = u64::MAX;

    while let Ok(mut batch) = asks.recv() {
        let key = (
            batch.ask.tick,
            batch.ask.new_moon_tick,
            [
                batch.ask.inputs.turbidity.to_bits(),
                batch.ask.inputs.ozone_atm_cm.to_bits(),
            ],
            batch.ask.generation,
        );
        if solver_key != Some(key) {
            // **A different beam is a different solver** — and nothing else. The trees are the
            // world's, not the beam's, so the cache survives a tick: it used to be dropped with
            // the solver, which meant every ask re-requested every chunk and each request waits
            // for a frame.
            solver = WorldLight::of(
                &batch.ask.spec,
                batch.ask.tick,
                batch.ask.inputs,
                batch.ask.new_moon_tick,
            );
            solver_key = Some(key);
        }
        if generation != batch.ask.generation {
            // The world record moved (an edit), so every held tree describes a world that is no
            // longer there.
            generation = batch.ask.generation;
            cache.borrow_mut().clear();
        }
        let Some(light) = solver.as_ref() else {
            // A solver the sky refused is the frame's own refusal: nothing is measured, and the
            // frame keeps the rig. The ask is answered with nothing rather than with a guess —
            // and with the end-of-ask message, so the frame may ask again.
            let _ = results.send(Batch {
                measured: Vec::new(),
                last: true,
            });
            continue;
        };
        let toward = light.toward_beam();
        let eye = batch.ask.eye;
        let resident: std::collections::HashSet<ChunkPos> = batch.resident.into_iter().collect();
        // **The cache is pruned to the window the streamer is holding**, once per ask. Without it,
        // a tree the streamer evicted stays alive here — still 89 kB, no longer charged to any
        // budget, and a walk across the world would accumulate them to the cache's own bound. What
        // this thread may hold is exactly what the frame says it has.
        cache.borrow_mut().retain(|pos, _| resident.contains(pos));
        // **Nearest first**, so the part of the world the eye is in is measured first, and so a
        // batch that is overtaken still leaves the near field current.
        let mut cells = std::mem::take(&mut batch.cells);
        cells.sort_unstable_by_key(|cell| {
            let c = cell.centre_m();
            let d = [c[0] - eye[0], c[1] - eye[1], c[2] - eye[2]];
            ((d[0] * d[0] + d[1] * d[1] + d[2] * d[2]) * 16.0) as u64
        });
        let mut out = Vec::with_capacity(BATCH);
        for cell in cells {
            let trees = WorkerTrees {
                trees: &cache,
                resident: &resident,
                ask: &tree_tx,
                reply: &tree_rx,
            };
            if let Some((bands, _absent)) = light.bands_across(&trees, cell.centre_m(), REACH_M) {
                out.push(Measured {
                    cell,
                    bands,
                    toward,
                });
            }
            if out.len() >= BATCH {
                let _ = results.send(Batch {
                    measured: std::mem::replace(&mut out, Vec::with_capacity(BATCH)),
                    last: false,
                });
            }
        }
        // The end of the ask, always: an ask whose cells all read as absent still ends.
        let _ = results.send(Batch {
            measured: out,
            last: true,
        });
    }
}

/// The worker's view of the world: a bounded cache, and a request to the frame on a miss.
struct WorkerTrees<'a> {
    trees: &'a RefCell<HashMap<ChunkPos, Arc<SparseVoxelOctree>>>,
    resident: &'a std::collections::HashSet<ChunkPos>,
    ask: &'a Sender<ChunkPos>,
    reply: &'a Receiver<(ChunkPos, Option<Arc<SparseVoxelOctree>>)>,
}

impl ChunkTrees for WorkerTrees<'_> {
    fn tree_at(&self, pos: ChunkPos) -> Option<Arc<SparseVoxelOctree>> {
        if let Some(tree) = self.trees.borrow().get(&pos) {
            return Some(Arc::clone(tree));
        }
        // **A chunk this thread does not have is absent in one lookup**, not a round trip: the
        // ask carries residency. Most hops leave the world upward, and asking about the sky one
        // frame at a time was the first version's whole cost.
        if !self.resident.contains(&pos) {
            return None;
        }
        // **This thread blocks; the frame does not.** The frame answers on its own clock and the
        // march resumes when the handle arrives.
        self.ask.send(pos).ok()?;
        let (answered, tree) = self.reply.recv().ok()?;
        if answered != pos {
            return None;
        }
        if let Some(tree) = &tree {
            let mut held = self.trees.borrow_mut();
            if held.len() >= CACHE_CHUNKS {
                held.clear();
            }
            held.insert(pos, Arc::clone(tree));
        }
        tree
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_model::VoxelPos;

    /// **A batch crosses the worker and comes back as measurements of the beam the field is
    /// holding** — the round trip the frame depends on, over a fixture world rather than a
    /// frame.
    ///
    /// The fixture is one chunk of clear air, so every cell in it has sky above it and every band
    /// is a measurement; what this holds is the *plumbing*: the ask, the tree handshake, the
    /// sorted batch and the adoption limit.
    #[test]
    fn a_batch_of_cells_crosses_the_worker_and_comes_back_measurable() {
        let mut measurer = Measurer::new();
        let spec = WorldSpec::new(2468, voxel_shapers::Config::default());
        let tick = voxel_shapers::climate::TICKS_PER_DAY / 2;
        let air = Arc::new(SparseVoxelOctree::with_levels(5));
        let chunk = ChunkPos::new(0, 0, 0);
        measurer.ask_for(
            Ask {
                spec,
                tick,
                inputs: SkyInputs::default(),
                new_moon_tick: 0,
                generation: 0,
                eye: [0.0; 3],
            },
            (0..32)
                .map(|i| SubvoxelPos::from(VoxelPos::new(i % 8, 0, i / 8)))
                .collect(),
            vec![chunk],
        );
        // The frame's side of the handshake, until the batch has been answered. A real frame does
        // this once a frame with a small budget; the test does it in a loop with a deadline.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        let mut got = Vec::new();
        while got.len() < 32 && std::time::Instant::now() < deadline {
            measurer.service(64, |pos| (pos == chunk).then(|| Arc::clone(&air)));
            got.extend(measurer.adopt(1_024));
            std::thread::yield_now();
        }
        assert_eq!(
            got.len(),
            32,
            "the worker answered {} of 32 cells",
            got.len()
        );
        assert!(
            !measurer.busy(),
            "the batch was still in flight after it landed"
        );
        // **One cell, one answer**, and its bands are a measurement: a cell under open sky reads
        // more than nothing in its direct bands.
        let first = got
            .iter()
            .find(|m| m.cell == SubvoxelPos::from(VoxelPos::new(0, 0, 0)))
            .expect("the first cell");
        assert!(
            first.bands[0] + first.bands[1] + first.bands[2] > 0.0,
            "an open cell measured no beam: {:?}",
            first.bands
        );
        assert_eq!(first.toward, got[0].toward, "one batch carried two beams");
    }

    /// **A cell whose chunk the frame does not hold is not answered**, rather than answered from
    /// clear air: the march's first rule is that the cell's own chunk is resident.
    /// **A cell the worker refused is offered again**, and lands once its chunk is resident: the
    /// ask carries no refusal arm, so the caller's only lever is to ask again — which is the rule a
    /// run-long in-flight set broke.
    #[test]
    fn a_refused_cell_is_offered_again_and_lands_when_its_chunk_arrives() {
        let spec = WorldSpec::new(2468, voxel_shapers::Config::default());
        let tick = voxel_shapers::climate::TICKS_PER_DAY / 2;
        let chunk = ChunkPos::new(0, 0, 0);
        let air = Arc::new(SparseVoxelOctree::with_levels(5));
        let mut measurer = Measurer::new();
        let mut field = crate::field::CellField::of(&spec, tick, SkyInputs::default(), 0, None)
            .expect("a tick the atmosphere can describe");
        let cell = SubvoxelPos::from(VoxelPos::new(4, 0, 0));
        let ask = Ask {
            spec,
            tick,
            inputs: SkyInputs::default(),
            new_moon_tick: 0,
            generation: 0,
            eye: [0.0; 3],
        };

        // The first build asks for the cell, and the ask is made with no residency: refused.
        field.begin_frame();
        assert_eq!(
            field.at(cell),
            None,
            "an unmeasured cell answered with a light"
        );
        let first = field.wants(CELLS_PER_ASK);
        assert_eq!(first, vec![cell], "the build's demand was not queued");
        measurer.ask_for(ask, first, Vec::new());
        let mut landed = Vec::new();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        while measurer.busy() && std::time::Instant::now() < deadline {
            measurer.service(64, |_| None);
            landed.extend(measurer.adopt(1_024));
            std::thread::yield_now();
        }
        assert!(
            landed.is_empty(),
            "a cell with no world under it was measured: {landed:?}"
        );

        // The next build asks again — still a miss — and now the chunk is in hand.
        field.begin_frame();
        assert_eq!(field.at(cell), None);
        let again = field.wants(CELLS_PER_ASK);
        assert_eq!(again, vec![cell], "a refused cell was never offered again");
        measurer.ask_for(ask, again, vec![chunk]);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        while measurer.busy() && std::time::Instant::now() < deadline {
            measurer.service(64, |pos| (pos == chunk).then(|| Arc::clone(&air)));
            for measured in measurer.adopt(1_024) {
                field.adopt(measured.cell, measured.bands, measured.toward);
            }
            std::thread::yield_now();
        }
        field.begin_frame();
        assert!(
            field.at(cell).is_some(),
            "an answered cell never reached the field"
        );
    }

    #[test]
    fn a_cell_with_no_tree_behind_it_is_not_measured() {
        let mut measurer = Measurer::new();
        let spec = WorldSpec::new(2468, voxel_shapers::Config::default());
        measurer.ask_for(
            Ask {
                spec,
                tick: voxel_shapers::climate::TICKS_PER_DAY / 2,
                inputs: SkyInputs::default(),
                new_moon_tick: 0,
                generation: 0,
                eye: [0.0; 3],
            },
            vec![SubvoxelPos::from(VoxelPos::new(4, 0, 400))],
            // **No residency**: the chunk this cell needs is not in hand, so the ask is answered
            // with nothing rather than with a guess.
            Vec::new(),
        );
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        let mut got = Vec::new();
        while measurer.busy() && std::time::Instant::now() < deadline {
            measurer.service(64, |_| None);
            got.extend(measurer.adopt(1_024));
            std::thread::yield_now();
        }
        assert!(
            got.is_empty(),
            "a cell with no world under it was measured: {got:?}"
        );
    }
}
