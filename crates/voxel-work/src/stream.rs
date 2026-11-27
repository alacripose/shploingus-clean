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
//! takes whatever has finished, evicts what has gone out of the window, and
//! returns a [`Frame`] report — so the world visibly fills outward from the
//! viewer, the shapers are always running somewhere, and a HUD can say exactly
//! how far along it is. [`Streamer::settle`] is the same loop run to completion,
//! which is what a test or a first-load screen wants.
use voxel_model::endian::LeBytes;

use std::collections::{BTreeMap, BTreeSet};

use voxel_mesh::raycast::{Crossings, Ray};
use voxel_mesh::{Mesh, MeshConfig, SparseVoxelOctree};
use voxel_model::{
    ChunkPos, Digest, Face, MaterialId, Refusal, RefusalKind, Refused, VoxelPos, VoxelShape,
};
use voxel_shapers::earthworks::Modification;
use voxel_stream::{Horizons, WorldSource};

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
    /// Requests that were **already queued** and whose entry was re-keyed with a
    /// better priority — the chunks the picture was behind on. Counted apart from
    /// [`Self::submitted`], which is new work: one adds a job, the other moves one.
    pub promoted: usize,
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

/// **The most one hammer stroke may lift or drop**, metres — the world's own bound, declared here
/// rather than trusted from a caller: the record is per voxel and a stroke is a press.
pub const MAX_HAMMER_LIFT_M: i32 = 8;

/// **What one [`Streamer::shape_columns`] did**, so a stroke is reported rather than assumed.
///
/// Counted in **voxels**, not columns, because that is what the record holds and what the frame
/// paid for: a column can lose its whole stroke to a refusal half-way (a fluid three metres up),
/// and a report that said "one column raised" would hide it. `skipped` is "the world had no
/// solid surface there", `refused` is "the world declined the voxel" — two different answers a
/// person would want apart.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Shaped {
    /// Columns the stroke was aimed at.
    pub columns: usize,
    /// Voxels placed.
    pub raised: usize,
    /// Voxels removed.
    pub lowered: usize,
    /// Voxels the world refused.
    pub refused: usize,
    /// Columns with no solid surface to strike.
    pub skipped: usize,
}

impl Shaped {
    /// Every voxel the record took.
    pub fn voxels(&self) -> usize {
        self.raised + self.lowered
    }
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
    /// 2. **It is bounded by the window.** A stale mesh is dropped the moment its
    ///    chunk leaves the window, so this map can never outlive the streamer's
    ///    declared holding set or grow into a second cache.
    /// 3. **It is not world truth.** [`Streamer::len`] and [`Streamer::digest`]
    ///    count **fresh** meshes only, because a stale mesh is a piece of a
    ///    world that no longer exists and a digest spanning both would be a
    ///    statement about two worlds at once.
    stale: BTreeMap<ChunkPos, Mesh>,
    /// **The version of each drawn chunk's mesh** — the residency counter's value when the mesh it
    /// holds was admitted. A caller that keeps a scene across frames diffs this instead of the
    /// meshes' bytes: a chunk whose version moved was remeshed, and one that is gone left.
    versions: BTreeMap<ChunkPos, u64>,
    /// **The octrees behind the fresh meshes**, kept so the world's own light can be measured
    /// where it stands.
    ///
    /// The mesher needs a chunk's tree and throws it away ([`crate::pool`]'s own note is the
    /// memory argument for a worker); a *light* needs it after the mesh exists. `voxel_client::field`
    /// marches a cell's beam through this tree, so a tree that has been dropped cannot answer and
    /// the cell would fall back to the rig — the one light-per-frame model this exists to leave
    /// behind.
    ///
    /// **Held only while the mesh is fresh**, and dropped on the same two events the mesh is: an
    /// edit that demotes the chunk to [`Streamer::stale`], and eviction from the window. A chunk
    /// being rebuilt therefore has no field for the frames it is stale, and its cells fall back to
    /// the rig — which is the honest answer, because the tree in hand describes a world that has
    /// just been edited away.
    ///
    /// **Charged to the same ceiling the meshes are** (`SparseVoxelOctree::bytes`, about 89 kB a
    /// chunk against a mesh's 90,481 B), so `Frame::bytes` remains the whole of what this crate
    /// holds and not a mesh-only reading of it.
    ///
    /// **Shared rather than borrowed**, because the measurement runs off the frame thread: a
    /// measurer that needs a chunk's tree gets the same allocation through
    /// [`Streamer::tree_shared`] and the two never copy 89 kB at each other. The charge above is
    /// still the streamer's own — a second holder of one tree is one tree.
    trees: BTreeMap<ChunkPos, std::sync::Arc<SparseVoxelOctree>>,
    /// Chunks the generator refused. Remembered so a broken chunk is asked for
    /// once and not every frame — a hot retry loop on a chunk that cannot exist
    /// would spend the whole machine producing the same refusal.
    failed: BTreeSet<ChunkPos>,
    last_refusal: Option<Refusal>,
    evicted: u64,
    discarded: u64,
    refused_for_budget: u64,
    failed_count: u64,
    /// **The drawn world's own generation** — bumped whenever a mesh enters, leaves or is
    /// replaced. [`Streamer::world_generation`] is the *record's* generation and does not move
    /// when a chunk merely arrives, so a caller caching something built from the resident
    /// meshes (a scene, a digest of one) has to key on this too: residency without this number
    /// is a cache that keeps drawing a world the streamer has moved past.
    residency: u64,
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
            trees: BTreeMap::new(),
            stale: BTreeMap::new(),
            versions: BTreeMap::new(),
            failed: BTreeSet::new(),
            last_refusal: None,
            evicted: 0,
            discarded: 0,
            refused_for_budget: 0,
            failed_count: 0,
            residency: 0,
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

    /// **What is drawn, with each chunk's mesh version**: `(position, version, mesh)`.
    ///
    /// The pair a scene that outlives a frame diffs: a position that is new landed, one that is
    /// absent left, and one whose version moved was remeshed. The version is the streamer's own
    /// residency counter, so it is monotone and never repeats for two different meshes.
    pub fn drawn_meshes(&self) -> impl Iterator<Item = (ChunkPos, u64, &Mesh)> {
        let versions = &self.versions;
        self.meshes.iter().chain(self.stale.iter()).map(move |(pos, mesh)| {
            (*pos, versions.get(pos).copied().unwrap_or(0), mesh)
        })
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
    /// **A chunk's own octree**, for a caller that has to measure the world's light where it
    /// stands — `voxel_client::field` is the one there is.
    ///
    /// `None` for a chunk with no fresh mesh: one being rebuilt, one outside the window, and one
    /// the ceiling refused. The absence is the same statement in all three cases — there is no
    /// tree in hand that describes the chunk as it now is — and a caller that gets it falls back
    /// to the rig rather than to a guess.
    pub fn tree(&self, pos: ChunkPos) -> Option<&SparseVoxelOctree> {
        self.trees.get(&pos).map(|tree| tree.as_ref())
    }

    /// **The version of the tree a chunk holds**, or `None` where it holds none.
    ///
    /// The streamer's own residency counter, the same one [`Self::drawn_meshes`] reports: it is
    /// bumped whenever a chunk's mesh or tree enters, leaves or is replaced, so a caller that
    /// keeps a tree's *contents* somewhere else can tell a tree it has already read from one an
    /// edit has rebuilt. `voxel_client::horizon` is the caller there — a lit volume stamped from
    /// the tree of a record that has since been dug is the shadow of a block that is gone.
    pub fn tree_version(&self, pos: ChunkPos) -> Option<u64> {
        self.versions.get(&pos).copied()
    }

    /// **The same tree as a handle**, for a caller measuring on another thread.
    ///
    /// A refcount rather than a copy: `voxel_client::field`'s measurer keeps the trees its
    /// marches cross, and duplicating a chunk's 89 kB for it would put the world's light on the
    /// memory ledger a second time.
    pub fn tree_shared(&self, pos: ChunkPos) -> Option<std::sync::Arc<SparseVoxelOctree>> {
        self.trees.get(&pos).cloned()
    }

    /// **Which chunks have a tree in hand.** A measurer is handed this set with each ask so that
    /// a chunk outside the window is known in one hash lookup rather than a round trip to this
    /// thread: the sky above the world is most of a beam's march, and a request per hop for a
    /// chunk that cannot be there costs a frame each.
    pub fn tree_chunks(&self) -> impl Iterator<Item = ChunkPos> + '_ {
        self.trees.keys().copied()
    }

    /// Bytes held by the resident octrees, **beside** the meshes [`Streamer::digest`] and
    /// [`Frame::bytes`] account for. The charge inside the budget is the sum of the two; this is
    /// the breakdown, for a readout that has to say where the memory went.
    pub fn tree_bytes(&self) -> usize {
        self.trees.values().map(|tree| tree.bytes()).sum()
    }

    pub fn material_at(&self, x: i32, y: i32, z: i32) -> Refused<MaterialId> {
        self.pool.material_at(x, y, z)
    }

    /// **The voxel's own 4×4×4 shape** — the cell record's answer where one is recorded, the
    /// whole block where none is. A break reads this and hands the stepped-down shape back to
    /// [`Streamer::record_profile`], so the two are one record rather than a read beside a write.
    pub fn shape_at(&self, x: i32, y: i32, z: i32) -> Refused<VoxelShape> {
        self.pool.shape_at(x, y, z)
    }

    /// **The world's own surface at one column, whether or not the chunk is resident.**
    ///
    /// The same world the resident chunks were built from, read at a few voxels instead of a
    /// chunk's worth: the generator's own material law with the earthworks record composed
    /// over it, so a placed block or a dug pit shows here exactly as it does in the chunk it
    /// landed in. A caller holding a map of ground far outside the streamed window — which no
    /// chunk has been built for — asks this and gets the world's answer rather than the
    /// elevation law's.
    ///
    /// `None` where nothing solid stands in the probed span. It says nothing about residency:
    /// [`Streamer::chunk_surface`](crate::Streamer::tree) territory is the exact read for a
    /// chunk that is in hand, and it is the one to prefer wherever the chunk is there.
    pub fn column_surface(&self, x: i32, z: i32) -> Refused<Option<f32>> {
        self.pool.column_surface(x, z)
    }

    /// **Raise or lower the ground over a set of columns**, one voxel at a time, recorded.
    ///
    /// This is a stroke reaching the world: a footprint's whole-metre columns and a lift in metres,
    /// handed here by whoever owns the world. The map window used to be that caller and no longer is
    /// — it draws the ground and moves over it, and the terrain's modification is the shapers' over
    /// the world's cadence tick — so this is the door that path takes when it hands a stroke down.
    ///
    /// # What "raise" means, exactly
    ///
    /// Each column is read at **its own surface**, the top of its topmost solid voxel, which is the
    /// surface the map draws (canopy included, fluids skipped — the same rule
    /// [`Streamer::column_surface`] and the survey share). A raise then **places a voxel of the
    /// surface's own material above it**, one per metre, so a grassy hill rises as grass and a
    /// scree slope as scree; a lower **removes the topmost voxel**, one per metre. Nothing is
    /// invented: a column of air is skipped, not filled, and a stroke that meets water is counted
    /// as refused rather than forced (`hand_change` will not mint or delete a fluid).
    ///
    /// # Why per voxel, and why bounded
    ///
    /// **The record is per voxel** ([`voxel_shapers::earthworks`]), so a stroke is many entries
    /// and there is no region door to take instead — pretending there were would be a second
    /// record. The cost is a lock and a ledger per voxel, which is why the lift is bounded here and
    /// the caller's own ladder is so short: a stroke is a press, not a batch job.
    pub fn shape_columns(&mut self, columns: &[[i32; 2]], lift: i32, reason: &str) -> Refused<Shaped> {
        if columns.is_empty() {
            return Err(Refusal::new(
                RefusalKind::OutOfDeclaredRange,
                "a hammer stroke of no columns records nothing",
            ));
        }
        if lift == 0 {
            return Err(Refusal::new(
                RefusalKind::OutOfDeclaredRange,
                "a hammer stroke of 0 metres records nothing",
            ));
        }
        // **One declared bound.** A lift of a hundred metres is not a stroke a person meant, and
        // the failure it would cause is a frame spent recording a hundred thousand voxels.
        if lift.abs() > MAX_HAMMER_LIFT_M {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!(
                    "a hammer stroke of {lift} m is past the {MAX_HAMMER_LIFT_M} m a stroke may lift"
                ),
            ));
        }
        let mut report = Shaped::default();
        for &[x, z] in columns {
            let surface = match self.column_surface(x, z) {
                Ok(surface) => surface,
                Err(_) => {
                    report.refused += 1;
                    continue;
                }
            };
            // A column the world answers nothing for — open water over no floor, a column past the
            // declared span — is **skipped**, not filled from the law: the hammer edits voxels, and
            // there are none here to edit.
            let Some(top_m) = surface else {
                report.skipped += 1;
                continue;
            };
            // **The top voxel, from the top face.** A voxel at `y` owns `[y, y+1)`, so a surface at
            // 12.0 m is voxel 11's top face and a ¾-metre shape at 12.75 m is voxel 12's. The
            // epsilon keeps a whole-metre face from rounding up into the next voxel.
            let top_voxel = (top_m - 1e-4).floor() as i32;
            // **A column is either up or down for the whole stroke**, decided by its own
            // surface: a raise copies the material the surface is made of, and a lower takes it
            // away. Both are recorded as the player's hand at this column.
            if lift > 0 {
                let Ok(material) = self.material_at(x, top_voxel, z) else {
                    report.refused += 1;
                    continue;
                };
                if material.is_empty() {
                    report.skipped += 1;
                    continue;
                }
                for step in 1..=lift {
                    let edit = crate::pool::Edit {
                        x,
                        y: top_voxel + step,
                        z,
                        to: material,
                        reason: format!("{reason} — raised at {x},{z}"),
                        actor: voxel_model::world::ActorId::player(),
                        verb: voxel_model::world::PhysicalVerb::Fill,
                    };
                    match self.edit(edit) {
                        Ok(_) => report.raised += 1,
                        Err(_) => report.refused += 1,
                    }
                }
            } else {
                for step in 0..(-lift) {
                    let edit = crate::pool::Edit {
                        x,
                        y: top_voxel - step,
                        z,
                        to: voxel_shapers::materials::AIR,
                        reason: format!("{reason} — lowered at {x},{z}"),
                        actor: voxel_model::world::ActorId::player(),
                        verb: voxel_model::world::PhysicalVerb::Dig,
                    };
                    match self.edit(edit) {
                        Ok(_) => report.lowered += 1,
                        Err(_) => report.refused += 1,
                    }
                }
            }
        }
        report.columns = columns.len();
        Ok(report)
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
    /// builds from a snapshot taken at its start, so it may finish with the old
    /// world in its hands — and its generation stamp is what keeps that mesh out
    /// of the cache ([`Frame::stale`]). The edit itself never waits for a build:
    /// the world's read lock is held for one snapshot clone, not for the whole
    /// build (measured — see `Pool::build_one`).
    ///
    /// **Who, and what act**, for the same reason [`crate::Pool::edit`] asks: the earthworks
    /// entry names its actor and its verb, and neither is inferable from the material.
    pub fn edit(&mut self, edit: crate::pool::Edit) -> Refused<Modification> {
        let (x, y, z) = (edit.x, edit.y, edit.z);
        let modification = self.pool.edit(edit)?;
        self.demote([x, y, z]);
        Ok(modification)
    }

    /// **Apply the Earthworks stage's recorded cell to the world's surface.**
    ///
    /// The cell is the process roster's output and the pool records it as the voxel's
    /// shape ([`Pool::record_profile`]); this is the half the picture needs: the chunks
    /// that voxel reaches are demoted, exactly as an edit's are, and the pump rebuilds
    /// them from the record. The shape is the cell the stage computed — there is no
    /// column carrier and no second writer beside the roster.
    ///
    /// `true` says the record and a chunk's rebuild moved; a cell the record already
    /// held costs neither.
    pub fn record_profile(
        &mut self,
        x: i32,
        y: i32,
        z: i32,
        shape: voxel_model::VoxelShape,
    ) -> bool {
        if !self.pool.record_profile(x, y, z, shape) {
            return false;
        }
        self.demote([x, y, z]);
        true
    }

    /// **Demote the chunks one voxel reaches** — the chunk's mesh is moved to the stale
    /// set (still drawn, still charged) and its octree dropped, so the pump's next ask
    /// rebuilds from the new record. Shared by a material edit and a shaper's cell, so
    /// the two write paths cannot disagree about what an invalidation is.
    fn demote(&mut self, voxel: [i32; 3]) {
        for pos in chunks_touching(voxel) {
            // Demoted, not dropped: the mesh keeps its charge and keeps being
            // drawn, and `absorb` releases it when the replacement lands. A
            // second edit while the first replacement is still building finds the
            // chunk already stale and leaves the existing stale mesh alone —
            // re-demoting it would be a no-op that still cost a lookup.
            if let Some(mesh) = self.meshes.remove(&pos) {
                self.stale.entry(pos).or_insert(mesh);
                self.residency = self.residency.wrapping_add(1);
            }
            // **And its tree goes now rather than with the mesh**, because a field measured
            // through it would be a measurement of the world the edit just removed: a dug hole
            // would keep casting the shadow of the block that was there. Unconditionally, so a
            // second edit before the replacement lands finds nothing left to release.
            if let Some(tree) = self.trees.remove(&pos) {
                self.budget.release(tree.bytes());
            }
            // It may be inside again later with the *new* world; a remembered
            // failure would keep the hole the edit just made.
            self.failed.remove(&pos);
        }
    }

    /// **The world record's generation** — bumped by every recorded modification.
    ///
    /// A cheap watch for "the record moved": a caller that reads [`Streamer::modifications`]
    /// every frame would copy the whole record even when nothing changed, and this is the number
    /// that says whether there is anything new to read.
    pub fn world_generation(&self) -> u64 {
        self.pool.generation()
    }

    /// **The drawn world's generation** — bumped whenever a chunk's mesh enters, leaves or is
    /// replaced, unlike [`Streamer::world_generation`] which is the *record's* and moves only on
    /// an edit.
    ///
    /// A caller that keeps something built from the resident meshes between frames keys on both:
    /// the record says the world's substance changed, this says which pieces of it are in hand.
    pub fn residency_generation(&self) -> u64 {
        self.residency
    }

    /// The world's modification record, as a snapshot. See [`Pool::modifications`].
    pub fn modifications(&self) -> Vec<Modification> {
        self.pool.modifications()
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
        t.extend_from_slice(&(self.meshes.len() as u64).into_le_bytes());
        for (pos, mesh) in &self.meshes {
            t.extend_from_slice(&pos.x.into_le_bytes());
            t.extend_from_slice(&pos.y.into_le_bytes());
            t.extend_from_slice(&pos.z.into_le_bytes());
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
        // (see [`Self::ask_priority`] for what a *stale* candidate is asked with; the
        // sort here is the offering order, and the pool re-orders by priority.)
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

    /// **The offset that puts a chunk drawn from a stale mesh ahead of every
    /// routine ask.**
    ///
    /// Routine priorities are squared distances from the viewer, so they are never
    /// negative and never exceed `2·radius² + 4·vertical²`. Subtracting a bound
    /// larger than any window can produce makes every boosted priority negative —
    /// below every routine one — and because the subtraction is monotone, the
    /// nearest stale chunk is still asked for first *among them*.
    const STALE_PRIORITY_BASE: i64 = 1 << 40;

    /// The priority a chunk is asked with. **Lower is sooner**, and a chunk drawn
    /// from a stale mesh asks with [`Self::STALE_PRIORITY_BASE`] subtracted, so an
    /// edit's replacement does not wait behind the horizon fill the chunk happened
    /// to be slotted into. See [`Request::priority`] and [`Submitted::Promoted`].
    fn ask_priority(&self, centre: ChunkPos, pos: ChunkPos) -> i64 {
        let routine = self.window.priority(centre, pos);
        if self.stale.contains_key(&pos) {
            routine - Self::STALE_PRIORITY_BASE
        } else {
            routine
        }
    }

    /// One frame of streaming: cancel, evict, ask, take.
    pub fn pump(&mut self) -> Refused<Frame> {
        let centre = self.centre;
        let window = self.window;

        // 1. Queued work that the viewer has moved off is dropped now.
        self.pool.cancel_where(&|p| window.contains(centre, p));

        // 2. Meshes outside **the window** go, and their charge comes back. The window is what
        // this streamer asks for, admits and draws; the residency horizon is a concept reach
        // (radius 24 is 117 649 chunks), and keying eviction on it held every chunk a step had
        // left behind — 660 for a window of 605 at rest. Both sets are swept, so a stale mesh
        // cannot outlive the window that asked for it.
        let mut outside: Vec<ChunkPos> = self
            .meshes
            .keys()
            .copied()
            .chain(self.stale.keys().copied())
            .filter(|p| !self.window.contains(centre, *p))
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
                self.residency = self.residency.wrapping_add(1);
                self.versions.remove(&pos);
                evicted += 1;
            } else if let Some(mesh) = self.stale.remove(&pos) {
                self.budget.release(mesh.bytes());
                self.residency = self.residency.wrapping_add(1);
                self.versions.remove(&pos);
                evicted += 1;
            }
            // The tree goes with the mesh it belongs to, so a chunk outside the window costs
            // nothing: not its mesh and not its octree.
            if let Some(tree) = self.trees.remove(&pos) {
                self.budget.release(tree.bytes());
            }
        }
        // A chunk that failed and has left the window is forgotten: it may be
        // inside again later, and a permanent blacklist would leave a hole in the
        // world forever.
        self.failed.retain(|p| self.window.contains(centre, *p));
        // And **room was freed**, so what did not fit is worth asking again.
        // Without this a mesh the budget once refused would stay refused for as
        // long as the viewer kept it in the window, even after the eviction of
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
        let mut promoted = 0usize;
        let mut queue_full = 0u64;
        for pos in missing {
            let request = Request { pos, priority: self.ask_priority(centre, pos) };
            match self.pool.submit(request) {
                Submitted::Queued => submitted += 1,
                Submitted::Promoted => promoted += 1,
                Submitted::QueueFull => queue_full += 1,
                Submitted::AlreadyQueued => {}
            }
        }

        // 4. Take what finished.
        let Collected { meshes, failures } = self.pool.collect();
        let applied = self.absorb(meshes, failures);

        self.evicted += evicted;
        Ok(self.report(submitted, promoted, queue_full, applied))
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
            let Built { generation: built_from, stamp, mesh, tree } = built;
            let pos = mesh.chunk;
            if built_from != generation || stamp != self.pool.chunk_stamp(pos) {
                // An edit, or a cell recorded at a voxel this chunk reads, landed
                // while the chunk was being meshed. It is not a cache miss; it is a
                // piece of a world that no longer exists.
                apply.stale += 1;
                continue;
            }
            if !self.window.contains(centre, pos) {
                apply.discarded += 1;
                continue;
            }
            // **The tree is charged beside the mesh**, because both are held: a chunk is not
            // affordable if only one of its two artifacts fits, and a reading that counted the
            // mesh alone would report a streamer holding half of what it holds.
            let bytes = mesh.bytes() + tree.bytes();
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
            self.trees.insert(pos, std::sync::Arc::new(tree));
            self.residency = self.residency.wrapping_add(1);
            self.versions.insert(pos, self.residency);
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

    fn report(
        &self,
        submitted: usize,
        promoted: usize,
        queue_full: u64,
        applied: Apply,
    ) -> Frame {
        Frame {
            wanted: self.window.count() as usize,
            resident: self.meshes.len(),
            stale_drawn: self.stale.len(),
            submitted,
            promoted,
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
            last = self.report(0, 0, 0, applied);
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
    /// **Every mesh is out of date, and every mesh is still a picture of the world.** The record
    /// does not say which chunks the change reached, so every fresh mesh is *demoted to the stale
    /// set*: still drawn, still charged, and replaced one landing at a time as the pool answers —
    /// the same rule [`Streamer::edit`] states for one chunk, applied to the window because that is
    /// how far the change reaches.
    ///
    /// # Why not clear them, measured
    ///
    /// Clearing is what the whole window used to do, and a world drawn from a cleared window is a
    /// world that is *absent* until the rebuild reaches each chunk. From the client's own witness
    /// (`voxel-client/tests/the_world_does_not_vanish.rs`, a settled window of 92 chunks):
    ///
    /// ```text
    /// settled        10 024 quads drawn
    /// after          0 quads drawn, and still 0 twenty frames later
    /// ```
    ///
    /// and in a live run at the user's eye height the same shape read `57 188 -> 36 quads` with
    /// **428 frames** before the world was back — the player's report, twice: *"the terrain that's
    /// supposed to render near the player stops rendering instead of switching to a lower lod"*.
    ///
    /// **Nothing is copied and nothing is re-walked**: the demotion is a move between two maps, so
    /// the frame after a world change draws the world the frame before it drew, at the version it
    /// already had. A chunk's *replacement* moves its version, and that is what the scene diffs.
    /// The trees *are* dropped: a tree is the input to the next build, and the one in hand describes
    /// the world the change removed.
    pub fn note_world_changed(&mut self) {
        let fresh = std::mem::take(&mut self.meshes);
        for (pos, mesh) in fresh {
            // `or_insert`, for the same reason `edit` uses it: a chunk already stale holds a mesh
            // at least as good as this one, and re-demoting it would be a no-op that still cost a
            // lookup — and would drop the mesh it was holding.
            self.stale.entry(pos).or_insert(mesh);
        }
        // The versions stay put, deliberately: the bytes a chunk is drawn from have not changed
        // yet, so a scene that diffs them must see no change. When the replacement lands, `absorb`
        // charges it, releases the stale mesh and moves the version — one chunk at a time.
        //
        // **A dropped octree's charge goes back with it.** The trees are the one thing here that
        // is not kept, and every other place a tree is dropped releases its bytes
        // ([`Streamer::edit`], [`Streamer::pump`]); `clear` alone would drop them and keep
        // charging for them, so every world change would leave a window's worth of octrees on the
        // ledger forever — measured by
        // `a_changed_world_demotes_every_mesh_and_the_rebuild_is_the_same_world`.
        for (_, tree) in std::mem::take(&mut self.trees) {
            self.budget.release(tree.bytes());
        }
        self.failed.clear();
        self.last_refusal = None;
    }

    /// Stop the workers. Consuming, so a stopped streamer cannot be pumped.
    pub fn shutdown(self) {
        self.pool.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_shapers::materials;
    use voxel_shapers::{Config, WorldSpec};
    use voxel_stream::Horizon;

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

    /// **The hammer's stroke moves the world's own surface and the record holds it**, which is the
    /// whole contract the map's ring promises: what it drew is what the world now says.
    #[test]
    fn a_hammer_stroke_raises_the_worlds_own_surface_and_records_it() {
        let mut s = streamer(2, 64 * 1024 * 1024);
        s.settle().expect("settles");
        let spec = WorldSpec::new(2468, Config::default());
        let sea = spec.config.sea_level_m as f32;
        // **A bare column above the sea**: its surface is the generated ground's own top face, not
        // a canopy standing over it and not a lake's floor. Without that, a stroke that follows the
        // contract exactly still reads as a different height — the test would be measuring a tree.
        let [x, z] = (0..32)
            .flat_map(|x| (0..32).map(move |z| [x, z]))
            .find(|[x, z]| {
                let surface = voxel_shapers::terrain::elevation_m(
                    &spec,
                    f64::from(*x) + 0.5,
                    f64::from(*z) + 0.5,
                );
                let ground = (surface - 0.5).floor();
                s.column_surface(*x, *z).ok().flatten().is_some_and(|top| {
                    top > sea + 1.0 && (f64::from(top) - 1.0 - ground).abs() < 1e-6
                })
            })
            .expect("a bare column above the sea");
        let before = s
            .column_surface(x, z)
            .expect("the world answers")
            .expect("solid ground");
        let generation = s.world_generation();

        let shaped = s
            .shape_columns(&[[x, z]], 2, "a test's hammer")
            .expect("a stroke");
        assert_eq!(shaped.columns, 1);
        assert_eq!(shaped.raised, 2, "{shaped:?}");
        assert_eq!(shaped.refused + shaped.skipped, 0, "{shaped:?}");
        assert!(s.world_generation() > generation, "the record did not move");
        let after = s
            .column_surface(x, z)
            .expect("the world answers")
            .expect("solid ground");
        assert!(
            (after - (before + 2.0)).abs() < 1e-4,
            "two metres of lift read back as {after}, from {before}"
        );

        // **And down again, below where it started**: the stroke is the same door in both
        // directions, and the surface follows it exactly.
        let shaped = s
            .shape_columns(&[[x, z]], -3, "a test's hammer")
            .expect("a stroke");
        assert_eq!(shaped.lowered, 3, "{shaped:?}");
        let lowered = s
            .column_surface(x, z)
            .expect("the world answers")
            .expect("solid ground");
        assert!(
            (lowered - (before - 1.0)).abs() < 1e-4,
            "three metres of dig read back as {lowered}, from {after}"
        );
        // **The record says so per voxel, in the hammer's words** — a stroke is entries, not a
        // summary, and the map's next read is the record's own answer.
        let entries = s.modifications();
        assert_eq!(
            entries.iter().filter(|entry| entry.reason.contains("a test's hammer")).count(),
            5,
            "{entries:?}"
        );

        // A stroke that means nothing is refused by name rather than recording nothing.
        assert!(s.shape_columns(&[], 1, "a test's hammer").is_err());
        assert!(s.shape_columns(&[[x, z]], 0, "a test's hammer").is_err());
        assert!(s
            .shape_columns(&[[x, z]], MAX_HAMMER_LIFT_M + 1, "a test's hammer")
            .is_err());
    }

    #[test]
    fn the_drawn_generation_moves_when_the_drawn_world_does() {
        // The distinction this exists for: `world_generation` is the *record's* and does not move
        // when a chunk merely arrives, so a scene cache keyed on it alone would keep drawing a
        // window the streamer had already filled.
        let mut s = streamer(4, 64 * 1024 * 1024);
        let record = s.world_generation();
        let before = s.residency_generation();
        s.settle().unwrap();
        assert!(s.residency_generation() > before, "no chunk's arrival was recorded");
        assert_eq!(s.world_generation(), record, "nothing was edited");
        // A settled world pumps and changes nothing drawn, so the number holds: that is what lets
        // a frame keep the scene it built.
        let settled = s.residency_generation();
        s.pump().unwrap();
        assert_eq!(s.residency_generation(), settled, "a settled pump moved the drawn world");
        // An edit demotes its chunk to stale, which is a change to what is drawn. The cell is a
        // solid one in the centre chunk, so the edit reaches a mesh the window holds.
        let c = s.centre();
        let (x, z) = (c.x * 32 + 16, c.z * 32 + 16);
        let y = (c.y * 32..c.y * 32 + 32)
            .rev()
            .find(|y| !s.material_at(x, *y, z).unwrap().is_empty())
            .expect("the centre chunk's middle column is empty");
        s.edit(crate::pool::Edit {
            x,
            y,
            z,
            to: materials::AIR,
            reason: "the test dug here".into(),
            actor: voxel_model::world::ActorId::player(),
            verb: voxel_model::world::PhysicalVerb::Dig,
        })
        .unwrap();
        assert!(s.residency_generation() > settled, "an edit left the drawn world unmoved");
    }

    #[test]
    fn settle_builds_the_whole_window_and_reports_full_progress() {
        let mut s = streamer(4, 64 * 1024 * 1024);
        let frame = s.settle().unwrap();
        assert_eq!(frame.resident, window().count() as usize);
        assert_eq!(frame.wanted, window().count() as usize);
        assert!((frame.progress() - 1.0).abs() < 1e-12);
        // The invariant that makes the budget meaningful: what is charged is
        // **exactly what is held** — and since the octrees are held beside the meshes
        // (`Streamer::tree`), the sum is both or the invariant is not the one stated.
        assert!(s.tree_bytes() > 0, "no chunk's octree was kept, so nothing could be lit");
        let held: usize = s.meshes().map(Mesh::bytes).sum::<usize>() + s.tree_bytes();
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
    fn the_held_set_is_the_window_and_not_the_wider_residency_horizon() {
        // The held set is the window on every pump of a walk, not only after a jump.
        let mut s = streamer(4, 64 * 1024 * 1024);
        s.settle().unwrap();
        let want = window().count() as usize;
        assert_eq!(s.len(), want, "the settled window is not the window");
        let start = s.centre();
        for step in 1..=4 {
            // One chunk east, and a chunk down every other step: a walk a player can
            // produce, each move far inside the residency horizon.
            s.set_centre(ChunkPos::new(start.x + step, start.y - step / 2, start.z));
            for _ in 0..8 {
                let frame = s.pump().unwrap();
                assert!(
                    s.len() <= want,
                    "step {step}: {} chunks held for a window of {want}",
                    s.len()
                );
                assert!(frame.bytes <= frame.ceiling);
            }
            s.settle().unwrap();
            assert_eq!(
                s.len(),
                want,
                "step {step}: the window did not refill to its own size"
            );
        }
        // And what is held is exactly the window's own positions, not a superset of them.
        let centre = s.centre();
        for pos in s.meshes().map(|m| m.chunk) {
            assert!(
                window().contains(centre, pos),
                "{pos:?} is held but is outside the window"
            );
        }
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
    fn the_engine_reports_the_scratch_it_actually_holds() {
        // **The number a HUD reads is what the pool holds, not what it could.** A caller
        // budgeting 32 MB for the world still has to set aside four workers' ~19 MB while
        // it streams (`scratch_ceiling`), and the frame reports what is held *now*: a
        // settled window is building nothing, so it holds nothing.
        let mut s = streamer(4, 64 * 1024 * 1024);
        let bound = 4 * voxel_mesh::Mesher::new().scratch_bytes();
        let mut frame = s.pump().unwrap();
        assert!(frame.scratch_bytes <= bound, "a frame over-reported its scratch");
        s.settle().unwrap();
        for _ in 0..500 {
            frame = s.pump().unwrap();
            if frame.scratch_bytes == 0 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert!(frame.scratch_bytes <= bound);
        assert_eq!(
            frame.scratch_bytes, 0,
            "a settled engine kept {} B of scratch",
            frame.scratch_bytes
        );
        assert!(bound > frame.ceiling / 8, "scratch is not the small part");
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
        s.edit(crate::pool::Edit {
            x,
            y,
            z,
            to: materials::AIR,
            reason: "the test dug here".into(),
            actor: voxel_model::world::ActorId::player(),
            verb: voxel_model::world::PhysicalVerb::Dig,
        })
        .unwrap();

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

        s.edit(crate::pool::Edit {
            x,
            y,
            z,
            to,
            reason: "the test dug here".into(),
            actor: voxel_model::world::ActorId::player(),
            verb: voxel_model::world::PhysicalVerb::Dig,
        })
        .unwrap();

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
        // The mesh that is about to become stale, measured **before** it moves: the charge that
        // must come back is this one, and only this one.
        let replaced_bytes = s.mesh(target).expect("found above").bytes();
        // And the octree that goes **now** rather than with the mesh: the field is measured through
        // it, and a field measured through the world before the edit would keep the dug block's
        // shadow standing for the frames the chunk is stale.
        let replaced_tree_bytes = s.tree(target).expect("the chunk had a tree").bytes();

        s.edit(crate::pool::Edit {
            x,
            y,
            z,
            to: materials::AIR,
            reason: "the test dug here".into(),
            actor: voxel_model::world::ActorId::player(),
            verb: voxel_model::world::PhysicalVerb::Dig,
        })
        .unwrap();
        assert_eq!(
            s.budget().charged(),
            charged_before - replaced_tree_bytes,
            "a stale mesh is still held — its charge must not silently vanish — while the tree it \
             was lit through must come back"
        );
        assert!(
            s.tree(target).is_none(),
            "the edited chunk's octree outlived the world it described"
        );

        s.settle().unwrap();
        assert_eq!(s.stale_len(), 0, "the replacement should have released it");
        assert!(s.mesh(target).is_some(), "and the chunk should be current again");
        // **The charge back, said exactly.** This line used to assert that the total returned to
        // `charged_before` — which is only true when the replacement is the same size as the mesh
        // it replaced, and *digging changes the mesh*: removing a voxel exposes its neighbours'
        // faces, so the new mesh may be larger or smaller than the old by any amount. That made a
        // claim about the budget's arithmetic depend on the world's geometry, and the assertion
        // broke the day the terrain under it changed — 150 bytes, which is a handful of faces, not
        // a leaked entry.
        //
        // Rule 1 is about the **entries**, so the identity is exact and holds for any content: the
        // stale entry's bytes come back, and the replacement is charged.
        let replacement_bytes = s.mesh(target).expect("checked above").bytes();
        assert_eq!(
            s.budget().charged(),
            charged_before - replaced_bytes + replacement_bytes,
            "the stale mesh's {replaced_bytes} bytes came back and the replacement's \
             {replacement_bytes} were charged, so the budget moved by exactly that exchange"
        );
        // And it holds nothing else: the budget is the current meshes **and their octrees**,
        // re-walked. This is the half the old assertion was reaching for, and it is
        // content-independent.
        let held: usize = s.meshes().map(voxel_mesh::Mesh::bytes).sum::<usize>() + s.tree_bytes();
        assert_eq!(
            s.budget().charged(),
            held,
            "the budget holds exactly the current meshes and their trees, and nothing stale is \
             still charged"
        );
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
        s.edit(crate::pool::Edit {
            x,
            y,
            z,
            to: materials::AIR,
            reason: "the test dug here".into(),
            actor: voxel_model::world::ActorId::player(),
            verb: voxel_model::world::PhysicalVerb::Dig,
        })
        .unwrap();
        assert_eq!(s.stale_len(), 1, "the edit made something stale");

        // Walk the viewer far enough away that the edited chunk is out of window.
        s.set_centre(ChunkPos::new(c.x + 10_000, c.y, c.z));
        s.pump().unwrap();
        assert_eq!(s.stale_len(), 0, "a stale mesh outlived its residency horizon");
        s.shutdown();
    }

    #[test]
    fn a_stale_chunk_asks_ahead_of_every_routine_one() {
        // **The caller's half of the promotion contract.** `Request::priority`
        // says a chunk the picture is behind on is submitted with a lower number;
        // an edited chunk is drawn from its old mesh until its replacement lands,
        // and that replacement must not queue behind the horizon fill the chunk
        // happened to be slotted into.
        let mut s = streamer(2, 64 * 1024 * 1024);
        s.settle().unwrap();
        let c = s.centre();
        let dig = |s: &mut Streamer, cx: i32, cz: i32| {
            let (x, z) = (cx * 32 + 16, cz * 32 + 16);
            let y = (c.y * 32..c.y * 32 + 32)
                .rev()
                .find(|y| !s.material_at(x, *y, z).unwrap().is_empty())
                .expect("the centre chunk's middle column is empty");
            s.edit(crate::pool::Edit {
                x,
                y,
                z,
                to: materials::AIR,
                reason: "the test dug here".into(),
                actor: voxel_model::world::ActorId::player(),
                verb: voxel_model::world::PhysicalVerb::Dig,
            })
            .unwrap();
        };
        dig(&mut s, c.x, c.z); // the near stale chunk: distance 0
        dig(&mut s, c.x + 1, c.z); // the far stale chunk: distance 1
        assert_eq!(s.stale_len(), 2, "both edits must leave a stale mesh");

        let near = ChunkPos::new(c.x, c.y, c.z);
        let far = ChunkPos::new(c.x + 1, c.y, c.z);
        let fresh = ChunkPos::new(c.x, c.y, c.z + 1);

        let routine = s.ask_priority(c, fresh);
        assert!(routine >= 0, "a routine ask's priority is a squared distance: {routine}");
        assert!(
            s.ask_priority(c, near) < 0 && s.ask_priority(c, far) < 0,
            "a stale ask must be ahead of every routine ask"
        );
        assert!(
            s.ask_priority(c, near) < s.ask_priority(c, far),
            "the boost must stay monotone in distance: {near:?} at {} vs {far:?} at {}",
            s.ask_priority(c, near),
            s.ask_priority(c, far)
        );
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
        s.edit(crate::pool::Edit {
            x,
            y,
            z,
            to: materials::AIR,
            reason: "the test dug here".into(),
            actor: voxel_model::world::ActorId::player(),
            verb: voxel_model::world::PhysicalVerb::Dig,
        })
        .unwrap();

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

        let impact = s.edit(crate::pool::Edit {
            x,
            y,
            z,
            to,
            reason: "the test dug here".into(),
            actor: voxel_model::world::ActorId::player(),
            verb: voxel_model::world::PhysicalVerb::Dig,
        })
        .unwrap();
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
    fn a_recorded_cell_demotes_only_the_chunk_it_reaches_and_the_window_keeps_streaming() {
        // The reason the pool stamps the record **per chunk**: an Earthworks tick records a
        // cell, and a global generation bump would discard every build in flight (measured:
        // 288 of 605 chunks resident with 340 builds thrown away). A cell reaches one chunk's
        // mesh, and one chunk is what moves.
        let mut s = streamer(2, 64 * 1024 * 1024);
        s.settle().unwrap();
        let before = s.len();
        let c = s.centre();
        let (x, z) = (c.x * 32 + 16, c.z * 32 + 16);
        let y = (c.y * 32..c.y * 32 + 32)
            .rev()
            .find(|y| !s.material_at(x, *y, z).unwrap().is_empty())
            .expect("the centre chunk's middle column is empty");
        let shape = voxel_shapers::earthworks::voxel_shape(
            s.material_at(x, y, z).unwrap(),
            &voxel_model::profile32::Cell::floor(0.5),
        );
        let generation = s.world_generation();
        let drawn = s.digest();

        assert!(s.record_profile(x, y, z, shape), "the record did not take the cell");
        assert_eq!(
            s.world_generation(),
            generation,
            "a cell moved the material record and discarded the window's builds"
        );
        assert_eq!(s.len(), before - 1, "the cell invalidated more than the chunk it reaches");
        assert_eq!(s.stale_len(), 1, "and it went stale, not missing");

        s.settle().unwrap();
        assert_eq!(s.len(), before, "the reached chunk did not come back");
        assert_ne!(s.digest(), drawn, "the rebuilt window is the old window: the cell was not drawn");
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

    /// **A world change demotes the window, it does not empty it** — and the ledger is exactly the
    /// window's own at every step of it.
    ///
    /// The old contract this test used to pin ("drops every mesh", charge back to zero) is the
    /// behaviour the flash fix replaced: a dropped window is a world that is *absent* until the
    /// rebuild reaches each chunk, which read `57 188 -> 36 quads` with 428 frames of hole in a
    /// live run. What holds now is the demotion: every current mesh moves to the stale set, still
    /// drawn and still charged, and its replacement releases it one chunk at a time. The octrees
    /// are the exception — the change goes *into* them, so they are dropped, and with them their
    /// charge.
    #[test]
    fn a_changed_world_demotes_every_mesh_and_the_rebuild_is_the_same_world() {
        let mut s = streamer(4, 64 * 1024 * 1024);
        s.settle().unwrap();
        let before = s.digest();
        let charged = s.budget().charged();
        let drawn = s.len();
        assert!(drawn > 0, "the window settled empty");
        s.note_world_changed();
        assert_eq!(s.len(), 0, "a mesh stayed current through a world change");
        assert_eq!(
            s.stale_len(),
            drawn,
            "the demotion lost a mesh rather than staling it"
        );
        // What is drawn is charged, so the demotion cannot give the window's bytes back — but what
        // it dropped, it pays for: a world change that kept the octrees' charge would leave a
        // window's worth of them on the ledger at every change.
        assert!(
            s.budget().charged() < charged,
            "the dropped octrees are still charged: {} B of {charged} B",
            s.budget().charged()
        );
        // And it rebuilds to the same world, because the generator did not change. That is what
        // makes demoting safe: nothing was lost, because nothing was stored that could not be
        // recomputed.
        s.settle().unwrap();
        assert_eq!(s.digest(), before, "the rebuilt world was not the same world");
        assert_eq!(s.len(), drawn, "the window did not come back whole");
        assert_eq!(
            s.budget().charged(),
            charged,
            "the rebuilt window is not the window's own charge: the trees leaked"
        );
        s.shutdown();
    }

    #[test]
    #[ignore = "measurement, not an assertion: run with `cargo test <filter> -- --ignored --nocapture`"]
    fn measure_the_edit_hitch_and_where_it_comes_from() {
        use std::time::{Duration, Instant};

        // 1. One chunk, straight from the generator — the unit of work a worker
        //    holds the world's read lock for.
        let src = source();
        let pos = centre();
        let t0 = Instant::now();
        let tree = src.build_chunk(pos).unwrap();
        println!(
            "build_chunk: {:?} ({} solid voxels of {})",
            t0.elapsed(),
            tree.solid_count(),
            voxel_model::CHUNK_VOLUME
        );

        // 2. One voxel through the live record: what a pick and a move step
        //    cost, and where a column's per-(x, z) work is paid per voxel
        //    instead.
        let s = streamer(1, 64 * 1024 * 1024);
        let t0 = Instant::now();
        let calls = 2_000u32;
        for i in 0..calls {
            let _ = s.material_at((i % 32) as i32, centre().y * 32 + 16, 16).unwrap();
        }
        println!("material_at: {:?} per call", t0.elapsed() / calls);
        s.shutdown();

        // 3. A settled pool, so the same edit can be timed against an **idle**
        //    world first.
        let mut idle = streamer(8, 64 * 1024 * 1024);
        idle.settle().unwrap();
        let c = idle.centre();
        let (x, z) = (c.x * 32 + 16, c.z * 32 + 16);
        let y = (c.y * 32..c.y * 32 + 32)
            .rev()
            .find(|y| !idle.material_at(x, *y, z).unwrap().is_empty())
            .expect("the centre chunk's middle column is empty");

        // A pick: the ray a click casts before it edits.
        let ray = Ray::new([x as f64 + 0.5, y as f64 + 1.5, z as f64 + 0.5], [0.0, -1.0, 0.0]);
        let t0 = Instant::now();
        let picked = idle.pick(&ray, 6.0).unwrap().map(|p| p.voxel);
        println!("pick: {:?} ({picked:?})", t0.elapsed());

        let t0 = Instant::now();
        let _ = idle
            .edit(crate::pool::Edit {
                x,
                y,
                z,
                to: materials::AIR,
                reason: "idle-baseline dig".into(),
                actor: voxel_model::world::ActorId::player(),
                verb: voxel_model::world::PhysicalVerb::Dig,
            })
            .unwrap();
        println!("edit (idle pool): {:?}", t0.elapsed());

        // 4. The same edit against a **busy** pool — the state the game is in
        //    while the window streams, which is most of the first minute and
        //    every time the viewer moves.
        let w = ChunkWindow { radius_chunks: 6, vertical_chunks: 1 };
        let mut busy = Streamer::new(source(), config(), 8, 512 * 1024 * 1024)
            .unwrap()
            .with_window(w)
            .unwrap();
        busy.set_centre(centre());
        busy.pump().unwrap();
        std::thread::sleep(Duration::from_millis(20));
        for i in 0..6 {
            // A different column each time, so no two edits share a transition
            // identity and none is refused as a replay.
            let (ex, ez) = (x + 1 + i * 3, z + 1);
            let ey = (c.y * 32..c.y * 32 + 32)
                .rev()
                .find(|y| !busy.material_at(ex, *y, ez).unwrap().is_empty())
                .expect("the measure column is empty");
            let t0 = Instant::now();
            let _ = busy
                .edit(crate::pool::Edit {
                    x: ex,
                    y: ey,
                    z: ez,
                    to: materials::AIR,
                    reason: format!("busy-hitch dig {i}"),
                    actor: voxel_model::world::ActorId::player(),
                    verb: voxel_model::world::PhysicalVerb::Dig,
                })
                .unwrap();
            println!("edit (busy pool): {:?}", t0.elapsed());
            std::thread::sleep(Duration::from_millis(250));
        }

        // 5. How long the **picture** takes to catch up with the record. Collision
        //    is instant (it reads the record), but the frame shows the chunk's
        //    mesh, and the mesh is re-built by a worker: this is the latency the
        //    player sees, and it is a different number from the click's own cost.
        let (ex, ez) = (x + 1 + 6 * 3, z + 1);
        let ey = (c.y * 32..c.y * 32 + 32)
            .rev()
            .find(|y| !busy.material_at(ex, *y, ez).unwrap().is_empty())
            .expect("the measure column is empty");
        let target = chunks_touching([ex, ey, ez])[0];
        busy
            .edit(crate::pool::Edit {
                x: ex,
                y: ey,
                z: ez,
                to: materials::AIR,
                reason: "the latency edit".into(),
                actor: voxel_model::world::ActorId::player(),
                verb: voxel_model::world::PhysicalVerb::Dig,
            })
            .unwrap();
        let t0 = Instant::now();
        let mut pumps = 0u32;
        while busy.mesh(target).is_none() && pumps < 2_000 {
            busy.pump().unwrap();
            std::thread::sleep(Duration::from_millis(1));
            pumps += 1;
        }
        println!(
            "edit → fresh mesh (busy pool): {:?} after {pumps} pump round(s); drawn from the stale mesh throughout: {}",
            t0.elapsed(),
            busy.is_stale(target)
        );
        busy.shutdown();
        idle.shutdown();
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
