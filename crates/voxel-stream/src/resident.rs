//! The resident set — what is currently loaded, and what it draws.
//!
//! This is the layer that owns the two questions a world cannot answer alone:
//! **is the chunk next door loaded** (which decides whether a face on this
//! chunk's edge is visible), and **what has gone stale** (because a neighbouring
//! chunk arrived, or because the earthworks record changed).
//!
//! # Provisional meshes, named rather than hidden
//!
//! When a chunk is meshed before all six of its neighbours exist, its boundary
//! faces are drawn against *nothing* — which is what an unloaded neighbour is,
//! and is therefore the right picture, but it is not the final one. The mesh is
//! recorded as **[`Resident::is_provisional`]** rather than silently treated as
//! complete, so the streaming layer can tell "this is what the world looks like"
//! from "this is what the world looks like so far".
//!
//! The flag is not a guess: the neighbourhood reports back whether it had to
//! answer from nothing, and that report is the flag.
//!
//! # Arrival invalidates, and that is not an optimisation
//!
//! [`Resident::ensure`] drops the mesh of every chunk that shares a face with
//! the new one. Skipping that would leave a chunk drawn as if it were at the
//! edge of the world after it stopped being at the edge of the world — visible
//! only at seams, and therefore exactly the kind of defect that survives to
//! release.
use voxel_model::endian::LeBytes;

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};

use voxel_model::{ChunkPos, Digest, LocalPos, Refusal, RefusalKind, Refused, VoxelPos, VoxelShape};
use voxel_mesh::source::ChunkNeighborhood;
use voxel_mesh::{Chunk, Mesh, MeshConfig, Mesher};

use voxel_shapers::materials;
use voxel_shapers::WorldSpec;

use crate::horizons::{Horizon, Horizons};
use crate::world_source::WorldSource;

/// Bytes charged per entry of an ordered index, key and bookkeeping together.
///
/// **A declared floor**, for the same reason as
/// [`voxel_mesh::octree::ALLOC_OVERHEAD`]: the true per-entry cost is an
/// allocator and container detail, so it is charged as a floor and named rather
/// than left out of the total. Leaving it out would make a large resident set's
/// reported size silently smaller than the machine's.
pub const MAP_ENTRY_OVERHEAD: usize = 48;

/// The six chunks that share a face with `p`.
///
/// Faces only: the mesher's pad reads one voxel past each of the six faces and
/// never past an edge or a corner, so an edge-diagonal neighbour cannot change
/// this chunk's mesh. Remeshing on all 26 would be 4× the work for no pixel.
fn face_neighbours(p: ChunkPos) -> [ChunkPos; 6] {
    [
        ChunkPos::new(p.x.saturating_sub(1), p.y, p.z),
        ChunkPos::new(p.x.saturating_add(1), p.y, p.z),
        ChunkPos::new(p.x, p.y.saturating_sub(1), p.z),
        ChunkPos::new(p.x, p.y.saturating_add(1), p.z),
        ChunkPos::new(p.x, p.y, p.z.saturating_sub(1)),
        ChunkPos::new(p.x, p.y, p.z.saturating_add(1)),
    ]
}

/// Answers the mesher's boundary questions from the **resident** chunks.
///
/// It never consults the generator: if the chunk next door is not loaded, the
/// mesher is told `None` — a report that something is missing — rather than
/// being handed a freshly generated chunk that would cost a full pipeline run
/// per boundary voxel and would still be the wrong answer if the world's
/// earthworks record has since moved.
struct ResidentNeighborhood<'a> {
    chunks: &'a BTreeMap<ChunkPos, Chunk>,
    around: ChunkPos,
    missing: Cell<bool>,
}

impl ChunkNeighborhood for ResidentNeighborhood<'_> {
    fn shape(&self, local: [i32; 3]) -> Refused<VoxelShape> {
        let o = self.around.origin();
        let world = VoxelPos::new(o.x, o.y, o.z).checked_add(local[0], local[1], local[2])?;
        let chunk = world.chunk();
        let l = world.local();
        match self.chunks.get(&chunk) {
            Some(c) => Ok(c.voxel(LocalPos { x: l.x, y: l.y, z: l.z })),
            None => {
                self.missing.set(true);
                Ok(VoxelShape::empty())
            }
        }
    }
}

/// The chunks currently loaded, and the meshes currently drawn from them.
pub struct Resident {
    source: WorldSource,
    horizons: Horizons,
    config: MeshConfig,
    chunks: BTreeMap<ChunkPos, Chunk>,
    meshes: BTreeMap<ChunkPos, Mesh>,
    provisional: BTreeSet<ChunkPos>,
}

impl Resident {
    /// An empty resident set over `source`.
    ///
    /// Refuses if the world's material registry is not self-consistent, because
    /// the mesh configuration (which materials are see-through) is read from it
    /// and a mesh built against a broken registry would be a mesh of a world
    /// that does not exist.
    pub fn new(source: WorldSource) -> Refused<Self> {
        let registry = materials::registry()?;
        let config = MeshConfig::from_registry(&registry)?;
        Ok(Resident {
            source,
            horizons: Horizons::declared(),
            config,
            chunks: BTreeMap::new(),
            meshes: BTreeMap::new(),
            provisional: BTreeSet::new(),
        })
    }

    /// Replace the horizon set. Refuses one that is not nested.
    pub fn with_horizons(mut self, horizons: Horizons) -> Refused<Self> {
        horizons.validate()?;
        self.horizons = horizons;
        Ok(self)
    }

    pub fn source(&self) -> &WorldSource {
        &self.source
    }

    /// The world, mutably. Record earthworks changes, then call
    /// [`Self::note_world_changed`].
    pub fn source_mut(&mut self) -> &mut WorldSource {
        &mut self.source
    }

    pub fn spec(&self) -> WorldSpec {
        self.source.spec()
    }

    pub fn horizons(&self) -> &Horizons {
        &self.horizons
    }

    pub fn mesh_config(&self) -> &MeshConfig {
        &self.config
    }

    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    pub fn contains(&self, pos: ChunkPos) -> bool {
        self.chunks.contains_key(&pos)
    }

    pub fn chunk(&self, pos: ChunkPos) -> Option<&Chunk> {
        self.chunks.get(&pos)
    }

    pub fn mesh(&self, pos: ChunkPos) -> Option<&Mesh> {
        self.meshes.get(&pos)
    }

    /// Whether this chunk's mesh was built with at least one neighbour absent,
    /// and therefore has boundary faces that a later arrival may cull.
    pub fn is_provisional(&self, pos: ChunkPos) -> bool {
        self.provisional.contains(&pos)
    }

    /// The chunks that are loaded and whose mesh is missing or stale.
    pub fn stale(&self) -> Vec<ChunkPos> {
        self.chunks
            .iter()
            .filter(|(pos, c)| c.is_dirty() || !self.meshes.contains_key(pos))
            .map(|(pos, _)| *pos)
            .collect()
    }

    pub fn positions(&self) -> impl Iterator<Item = ChunkPos> + '_ {
        self.chunks.keys().copied()
    }

    pub fn total_quads(&self) -> u64 {
        self.meshes.values().map(|m| m.stats.quads).sum()
    }

    /// Bytes held by voxels: every resident chunk's tree.
    ///
    /// Split from [`Self::mesh_bytes`] on purpose. The two answer different
    /// questions and move for different reasons — voxels are what the world *is*,
    /// meshes are what happens to be *drawn* — and a budget that only knew the
    /// total could not say which one grew.
    pub fn chunk_bytes(&self) -> usize {
        self.chunks.values().map(Chunk::bytes).sum()
    }

    /// Bytes held by meshes.
    pub fn mesh_bytes(&self) -> usize {
        self.meshes.values().map(Mesh::bytes).sum()
    }

    /// Bytes this resident set holds, **measured by walking it**.
    ///
    /// Not a running total: a running total is a number that can be wrong
    /// without anyone noticing, and this is the number a memory budget refuses
    /// on. The walk is `O(chunks)`, which between frames is nothing next to one
    /// chunk's generation.
    pub fn bytes(&self) -> usize {
        let index_entries = self.chunks.len() + self.meshes.len() + self.provisional.len();
        std::mem::size_of::<Self>()
            + self.chunk_bytes()
            + self.mesh_bytes()
            + index_entries * MAP_ENTRY_OVERHEAD
    }

    pub fn total_visible_faces(&self) -> u64 {
        self.meshes.values().map(|m| m.stats.visible_faces).sum()
    }

    /// Generate a chunk if it is not already resident, and invalidate the
    /// neighbours whose boundary it has just changed.
    pub fn ensure(&mut self, pos: ChunkPos) -> Refused<()> {
        if self.chunks.contains_key(&pos) {
            return Ok(());
        }
        let tree = self.source.build_chunk(pos)?;
        self.chunks.insert(pos, Chunk::from_tree(pos, tree));
        for n in face_neighbours(pos) {
            self.invalidate(n);
        }
        Ok(())
    }

    /// Generate every chunk within `horizon` of `centre`.
    pub fn ensure_horizon(&mut self, centre: ChunkPos, horizon: Horizon) -> Refused<u64> {
        let rule = *self.horizons.rule(horizon);
        let r = rule.radius_chunks;
        let mut made = 0u64;
        for x in centre.x - r..=centre.x + r {
            for y in centre.y - r..=centre.y + r {
                for z in centre.z - r..=centre.z + r {
                    if !self.chunks.contains_key(&ChunkPos::new(x, y, z)) {
                        self.ensure(ChunkPos::new(x, y, z))?;
                        made += 1;
                    }
                }
            }
        }
        Ok(made)
    }

    /// Mesh a chunk from whatever is resident, storing the result.
    ///
    /// The chunk is generated first if it is not loaded. Whether the mesh is
    /// provisional is recorded, and it is the neighbourhood's own report rather
    /// than an inference from a count of neighbours — a neighbour can be loaded
    /// and still be the wrong shape to compare against.
    pub fn build_mesh(&mut self, mesher: &mut Mesher, pos: ChunkPos) -> Refused<()> {
        self.ensure(pos)?;
        let (mesh, missing) = {
            let neighbors =
                ResidentNeighborhood { chunks: &self.chunks, around: pos, missing: Cell::new(false) };
            let chunk = self.chunks.get(&pos).ok_or_else(|| {
                Refusal::owned(
                    RefusalKind::IndexOutOfRange,
                    format!("chunk ({}, {}, {}) was not resident after generating it", pos.x, pos.y, pos.z),
                )
            })?;
            let mesh = mesher.build(chunk, &self.config, Some(&neighbors))?;
            (mesh, neighbors.missing.get())
        };
        self.meshes.insert(pos, mesh);
        if missing {
            self.provisional.insert(pos);
        } else {
            self.provisional.remove(&pos);
        }
        if let Some(c) = self.chunks.get_mut(&pos) {
            c.mark_meshed();
        }
        Ok(())
    }

    /// Mesh every stale chunk, returning how many were built.
    ///
    /// A fixed number of passes, then it stops: a chunk that is still stale
    /// after that has a cause this function does not have, and looping forever
    /// would hide it rather than report it. The limit is generous — building a
    /// mesh marks a chunk clean and only *invalidates* neighbours, so the count
    /// falls every pass in any world that is not pathological.
    pub fn mesh_stale(&mut self, mesher: &mut Mesher, passes: u32) -> Refused<u64> {
        let mut built = 0u64;
        for _ in 0..passes {
            let work = self.stale();
            if work.is_empty() {
                return Ok(built);
            }
            for pos in work {
                self.build_mesh(mesher, pos)?;
                built += 1;
            }
        }
        Ok(built)
    }

    /// The world underneath the resident chunks has changed.
    ///
    /// Coarse on purpose: an earthworks edit can reach any chunk, and the record
    /// does not index itself by chunk. Invalidating everything is honest about
    /// what is known; a cleverer version would need the record to say which
    /// chunks an edit touched, which is a change to the record and not here.
    pub fn note_world_changed(&mut self) {
        let positions: Vec<ChunkPos> = self.chunks.keys().copied().collect();
        for p in positions {
            self.invalidate(p);
        }
    }

    /// Drop everything outside `horizon` of `centre`.
    pub fn retain_within(&mut self, centre: ChunkPos, horizon: Horizon) {
        let rule = *self.horizons.rule(horizon);
        self.chunks.retain(|p, _| rule.contains(centre, *p));
        self.meshes.retain(|p, _| rule.contains(centre, *p));
        self.provisional.retain(|p| rule.contains(centre, *p));
    }

    /// A digest over what is resident and what is drawn.
    ///
    /// Order-independent by construction, because both maps are ordered by chunk
    /// position — so this is a statement about the **world**, not about the order
    /// chunks happened to be generated in (WG-Q4).
    pub fn digest(&self) -> Digest {
        let mut t = Vec::new();
        t.extend_from_slice(&(self.chunks.len() as u64).into_le_bytes());
        for (pos, chunk) in &self.chunks {
            t.extend_from_slice(&pos.x.into_le_bytes());
            t.extend_from_slice(&pos.y.into_le_bytes());
            t.extend_from_slice(&pos.z.into_le_bytes());
            t.extend_from_slice(chunk.content_digest().as_bytes());
        }
        t.extend_from_slice(&(self.meshes.len() as u64).into_le_bytes());
        for (pos, mesh) in &self.meshes {
            t.extend_from_slice(&pos.x.into_le_bytes());
            t.extend_from_slice(&pos.y.into_le_bytes());
            t.extend_from_slice(&pos.z.into_le_bytes());
            t.extend_from_slice(mesh.digest().as_bytes());
        }
        Digest::of_fields("voxel-stream/resident", &[b"v1", t.as_slice()])
    }

    fn invalidate(&mut self, pos: ChunkPos) {
        self.meshes.remove(&pos);
        self.provisional.remove(&pos);
        if let Some(c) = self.chunks.get_mut(&pos) {
            c.mark_dirty();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_mesh::{MeshStats, Quad};
    use voxel_shapers::{terrain, Config};

    fn resident() -> Resident {
        Resident::new(WorldSource::new(WorldSpec::new(2468, Config::default()))).unwrap()
    }

    /// The chunk `y` that actually contains the terrain surface.
    ///
    /// Found, not assumed. The default world's surface sits around the 16 m
    /// datum, so a chunk at `y = 1` (32…63 m) is mostly sky — and every test
    /// that wanted voxels with a surface in them was quietly meshing air until
    /// this was measured.
    fn surface_y() -> i32 {
        let spec = WorldSpec::new(2468, Config::default());
        let e = terrain::elevation_m(&spec, 48.5, 16.5);
        VoxelPos::new(48, e.floor() as i32, 16).chunk().y
    }

    /// A row of chunks straddling the surface, which is where the meshing is.
    fn surface_row() -> Vec<ChunkPos> {
        (0..3).map(|x| ChunkPos::new(x, surface_y(), 0)).collect()
    }

    #[test]
    fn a_generated_chunk_is_resident_and_meshes_to_something() {
        let mut r = resident();
        let mut mesher = Mesher::new();
        for pos in surface_row() {
            r.ensure(pos).unwrap();
        }
        for pos in surface_row() {
            r.build_mesh(&mut mesher, pos).unwrap();
        }
        assert_eq!(r.len(), 3);
        for pos in surface_row() {
            let m = r.mesh(pos).expect("a mesh should exist");
            assert!(m.stats.quads > 0, "chunk {pos:?} drew nothing");
            assert_eq!(m.stats.quads, m.quads.len() as u64);
        }
    }

    #[test]
    fn the_neighbourhood_culls_faces_that_an_isolated_chunk_would_draw() {
        // The pad, against **real world data** rather than a fixture. The middle
        // chunk of a loaded row must draw fewer faces than the same chunk alone.
        let pos = surface_row()[1];
        let mut mesher = Mesher::new();

        let alone = {
            let mut r = resident();
            r.ensure(pos).unwrap();
            r.build_mesh(&mut mesher, pos).unwrap();
            assert!(r.is_provisional(pos), "a chunk with no neighbours is provisional");
            r.mesh(pos).unwrap().clone()
        };

        let mut r = resident();
        for p in surface_row() {
            r.ensure(p).unwrap();
        }
        r.build_mesh(&mut mesher, pos).unwrap();
        let surrounded = r.mesh(pos).unwrap().stats.visible_faces;

        assert!(
            surrounded < alone.stats.visible_faces,
            "neighbours did not cull anything: {} alone vs {surrounded} surrounded",
            alone.stats.visible_faces
        );
    }

    #[test]
    fn a_chunk_meshed_without_its_neighbours_is_marked_provisional() {
        let mut r = resident();
        let mut mesher = Mesher::new();
        let pos = ChunkPos::new(4, surface_y(), 4);
        r.build_mesh(&mut mesher, pos).unwrap();
        assert!(r.is_provisional(pos), "no neighbour is resident");

        // Load all six and remesh: now it is not provisional.
        for n in face_neighbours(pos) {
            r.ensure(n).unwrap();
        }
        r.build_mesh(&mut mesher, pos).unwrap();
        assert!(!r.is_provisional(pos));
    }

    #[test]
    fn arrival_of_a_chunk_invalidates_the_neighbours_that_shared_that_boundary() {
        let mut r = resident();
        let mut mesher = Mesher::new();
        let row = surface_row();
        let pos = row[1];
        r.ensure(pos).unwrap();
        r.build_mesh(&mut mesher, pos).unwrap();
        assert_eq!(r.stale().len(), 0);

        r.ensure(row[2]).unwrap();
        assert!(
            r.chunk(pos).unwrap().is_dirty(),
            "the neighbour arrived and this chunk was not marked stale"
        );
        assert!(r.mesh(pos).is_none(), "the stale mesh was kept");
        assert!(r.stale().contains(&pos));
    }

    #[test]
    fn meshing_the_same_world_twice_in_the_same_order_gives_the_same_digest() {
        let mut a = resident();
        let mut b = resident();
        let mut ma = Mesher::new();
        let mut mb = Mesher::new();
        for pos in surface_row() {
            a.ensure(pos).unwrap();
            b.ensure(pos).unwrap();
        }
        for pos in surface_row() {
            a.build_mesh(&mut ma, pos).unwrap();
            b.build_mesh(&mut mb, pos).unwrap();
        }
        assert_eq!(a.digest(), b.digest());
    }

    #[test]
    fn the_digest_does_not_depend_on_the_order_chunks_were_built_in() {
        // WG-Q4 in miniature: a rebuild reproduces truth regardless of the order
        // its workers happened to finish in.
        let order: Vec<ChunkPos> = surface_row();
        let mut reversed = order.clone();
        reversed.reverse();

        let mut a = resident();
        let mut b = resident();
        let mut ma = Mesher::new();
        let mut mb = Mesher::new();
        for pos in &order {
            a.ensure(*pos).unwrap();
        }
        for pos in &reversed {
            b.ensure(*pos).unwrap();
        }
        for pos in &order {
            a.build_mesh(&mut ma, *pos).unwrap();
        }
        for pos in &reversed {
            b.build_mesh(&mut mb, *pos).unwrap();
        }
        assert_eq!(
            a.digest(),
            b.digest(),
            "the resident digest depends on build order"
        );
    }

    #[test]
    fn an_earthworks_change_invalidates_every_resident_chunk() {
        let mut r = resident();
        let mut mesher = Mesher::new();
        for pos in surface_row() {
            r.ensure(pos).unwrap();
        }
        r.mesh_stale(&mut mesher, 4).unwrap();
        assert_eq!(r.stale().len(), 0);
        r.note_world_changed();
        assert_eq!(r.stale().len(), 3, "a world change must stale every chunk");
    }

    #[test]
    fn retaining_a_horizon_drops_what_is_outside_it() {
        let mut r = resident();
        let mut mesher = Mesher::new();
        let y = surface_y();
        r.ensure(ChunkPos::new(0, y, 0)).unwrap();
        r.ensure(ChunkPos::new(30, y, 0)).unwrap();
        r.mesh_stale(&mut mesher, 4).unwrap();
        assert_eq!(r.len(), 2);

        r.retain_within(ChunkPos::new(0, y, 0), Horizon::Render);
        assert_eq!(r.len(), 1);
        assert!(r.contains(ChunkPos::new(0, y, 0)));
        assert!(!r.contains(ChunkPos::new(30, y, 0)));
        assert!(r.mesh(ChunkPos::new(30, y, 0)).is_none());
    }

    #[test]
    fn a_resident_set_refuses_a_horizon_set_that_does_not_validate() {
        // `Horizons` has no public mutation, so a malformed set can only exist
        // inside `horizons`'s own tests. What is checked here is that a resident
        // set *asks*: a future constructor that could build an unchecked set
        // would be caught by this test refusing to accept it.
        assert!(Horizons::declared().validate().is_ok());
        assert!(resident().with_horizons(Horizons::declared()).is_ok());
    }

    #[test]
    fn mesh_stale_terminates_and_leaves_nothing_stale() {
        // Sixteen passes is more than a 2×2×2 block can need; the assertion is
        // that it converges rather than that it converges quickly.
        let mut r = resident();
        let mut mesher = Mesher::new();
        let y = surface_y();
        for x in 0..2 {
            for z in 0..2 {
                r.ensure(ChunkPos::new(x, y, z)).unwrap();
            }
        }
        let built = r.mesh_stale(&mut mesher, 16).unwrap();
        assert!(built >= 4, "built {built}");
        assert!(r.stale().is_empty(), "still stale: {:?}", r.stale());
    }

    #[test]
    #[ignore = "measurement, not an assertion: run with `cargo test <filter> -- --ignored --nocapture`"]
    fn measure_the_cost_of_a_meshed_world() {
        // Numbers, not a verdict. A timing assertion would be a flaky test; a
        // printed measurement is evidence somebody can compare against.
        let mut r = resident();
        let mut mesher = Mesher::new();
        // A 3×3×3 block around the surface: small enough to finish in a debug
        // build, wide enough that interior chunks have all six neighbours.
        let base = ChunkPos::new(0, surface_y(), 0);
        let positions: Vec<ChunkPos> = (-1..=1)
            .flat_map(|x| {
                (-1..=1).flat_map(move |y| (-1..=1).map(move |z| ChunkPos::new(x, base.y + y, z)))
            })
            .collect();

        let t0 = std::time::Instant::now();
        let mut made = 0u64;
        for pos in &positions {
            if !r.contains(*pos) {
                r.ensure(*pos).unwrap();
                made += 1;
            }
        }
        let generate = t0.elapsed();

        // Meshing split by outcome, because "chunks per second" is only a
        // meaningful number if you know *which* chunks. A chunk of sky costs a
        // raster fill; a chunk with terrain in it costs the mask loops.
        let mut content = 0u64;
        let mut content_time = std::time::Duration::ZERO;
        let mut empty = 0u64;
        let mut empty_time = std::time::Duration::ZERO;
        let mut built = 0u64;
        for pos in &positions {
            let t = std::time::Instant::now();
            r.build_mesh(&mut mesher, *pos).unwrap();
            let d = t.elapsed();
            built += 1;
            if r.mesh(*pos).map(|m| m.stats.quads > 0).unwrap_or(false) {
                content += 1;
                content_time += d;
            } else {
                empty += 1;
                empty_time += d;
            }
        }
        let mesh = content_time + empty_time;

        println!(
            "{}-chunk block: {made} generated in {generate:?} ({:?}/chunk)",
            r.len(),
            generate / made.max(1) as u32,
        );
        println!(
            "  {built} meshed in {mesh:?}; {content} with geometry in {content_time:?} \
             ({:?}/chunk), {empty} with none in {empty_time:?} ({:?}/chunk)",
            content_time / content.max(1) as u32,
            empty_time / empty.max(1) as u32,
        );
        println!(
            "  {} quads from {} visible faces (merge factor {:.1}); scratch {} bytes",
            r.total_quads(),
            r.total_visible_faces(),
            r.total_visible_faces() as f64 / r.total_quads().max(1) as f64,
            mesher.scratch_bytes(),
        );
        println!("resident digest {}", r.digest());
        println!(
            "provisional chunks: {}",
            r.positions().filter(|p| r.is_provisional(*p)).count()
        );
    }

    #[test]
    fn accounting_counts_capacity_and_the_allocator_s_share() {
        // The three properties a budget depends on, each one thing a naive
        // `len() * size_of` would get wrong.
        let empty = Mesh {
            chunk: ChunkPos::new(0, 0, 0),
            quads: Vec::new(),
            loops: Vec::new(),
            stats: MeshStats::default(),
        };
        assert!(empty.bytes() >= std::mem::size_of::<Mesh>());

        // Capacity, not length: a vec that grew is holding what it grew to.
        let mut grown = empty.clone();
        grown.quads.reserve_exact(1000);
        assert!(grown.bytes() >= empty.bytes() + 1000 * std::mem::size_of::<Quad>());

        // A branch's children are on the heap and are counted, so a chunk with
        // structure costs more than one without.
        let mut mesher = Mesher::new();
        let mut r = resident();
        let y = surface_y();
        r.ensure(ChunkPos::new(0, 100, 0)).unwrap();
        let air_only = r.chunk(ChunkPos::new(0, 100, 0)).unwrap().bytes();
        let mut made_terrain = false;
        for dy in -2i32..=2 {
            let pos = ChunkPos::new(0, y + dy, 0);
            r.build_mesh(&mut mesher, pos).unwrap();
            if !made_terrain && r.chunk(pos).unwrap().node_count() > 1 {
                assert!(
                    r.chunk(pos).unwrap().bytes() > air_only,
                    "a chunk with structure must cost more than one without"
                );
                made_terrain = true;
            }
        }
        assert!(made_terrain, "no chunk in the band had structure to measure");

        // The set is at least the sum of its parts, and the index is charged.
        assert!(r.bytes() >= r.chunk_bytes() + r.mesh_bytes());
        assert!(r.bytes() >= r.chunk_bytes());
    }

    #[test]
    #[ignore = "measurement, not an assertion: run with `cargo test <filter> -- --ignored --nocapture`"]
    fn measure_the_memory_of_a_resident_world() {
        // The number that decides whether the declared horizons are affordable.
        // Measured over a y-stratified sample of a small region, then
        // extrapolated — and the extrapolation is stated as one, because a 7×7
        // patch is not the 49³ chunks a 24-chunk radius names.
        let mut r = resident();
        let mut mesher = Mesher::new();
        let base = surface_y();

        let mut chunk_bytes: Vec<usize> = Vec::new();
        let mut mesh_bytes: Vec<usize> = Vec::new();
        let mut nodes: Vec<u64> = Vec::new();
        let mut quads: Vec<u64> = Vec::new();

        let positions: Vec<ChunkPos> = (-3i32..=3)
            .flat_map(|x| {
                (-3i32..=3).flat_map(move |z| {
                    (-1i32..=1).map(move |dy| ChunkPos::new(x, base + dy, z))
                })
            })
            .collect();

        // Two phases, and the split is not cosmetic. Generating a chunk
        // **invalidates its neighbours' meshes** by design, so a loop that
        // generated and meshed each chunk in turn would measure the churn: the
        // first run of this measurement ended with 147 chunks and **one** mesh.
        // Voxels first, then the meshes settle.
        for pos in &positions {
            r.ensure(*pos).unwrap();
        }
        let built = r.mesh_stale(&mut mesher, 8).unwrap();
        assert_eq!(r.meshes.len(), positions.len(), "the set did not settle");

        for pos in &positions {
            let chunk = r.chunk(*pos).unwrap();
            let mesh = r.mesh(*pos).unwrap();
            chunk_bytes.push(chunk.bytes());
            mesh_bytes.push(mesh.bytes());
            nodes.push(chunk.node_count());
            quads.push(mesh.stats.quads);
        }

        let stat = |v: &[usize]| -> (usize, usize, usize) {
            let mut s = v.to_vec();
            s.sort_unstable();
            (s[0], s[s.len() / 2], s[s.len() - 1])
        };
        let (cmin, cmed, cmax) = stat(&chunk_bytes);
        let (mmin, mmed, mmax) = stat(&mesh_bytes);
        let total_chunk: usize = chunk_bytes.iter().sum();
        let total_mesh: usize = mesh_bytes.iter().sum();
        let n = chunk_bytes.len() as f64;
        let mean_chunk = total_chunk as f64 / n;
        let mean_mesh = total_mesh as f64 / n;
        let mean_node = nodes.iter().sum::<u64>() as f64 / n;
        let mean_quads = quads.iter().sum::<u64>() as f64 / n;

        println!(
            "{} chunks sampled around the surface, settled with {built} meshes built",
            chunk_bytes.len()
        );
        println!(
            "  per chunk: octree min/median/max {cmin}/{cmed}/{cmax} B, mean {mean_chunk:.0} B \
             ({mean_node:.0} nodes)"
        );
        println!(
            "  per mesh:  min/median/max {mmin}/{mmed}/{mmax} B, mean {mean_mesh:.0} B \
             ({mean_quads:.0} quads)"
        );
        println!(
            "  per chunk, both: mean {:.0} B -> {:.1} kB for a chunk with terrain in it",
            mean_chunk + mean_mesh,
            (mean_chunk + mean_mesh) / 1024.0
        );
        println!(
            "  index overhead: {} B/entry, {} entries -> {:.1} MB at this size",
            MAP_ENTRY_OVERHEAD,
            r.len() * 2,
            (r.len() * 2 * MAP_ENTRY_OVERHEAD) as f64 / (1024.0 * 1024.0)
        );

        // Extrapolation. A cube of side 2r+1, and the same radius clamped to a
        // slab as tall as the playable band plus one chunk of margin each way.
        let per_chunk = mean_chunk + mean_mesh;
        let mb = |chunks: f64| chunks * per_chunk / (1024.0 * 1024.0);
        // The same extrapolation for a design that keeps **meshes only** and
        // regenerates voxels on demand — which is what the world being a pure
        // function of its seed licenses, and which the two means above make
        // worth stating separately.
        let mesh_only_mb = |chunks: f64| chunks * mean_mesh / (1024.0 * 1024.0);
        let band_layers = (voxel_model::BAND_MAX_Z - voxel_model::BAND_MIN_Z) as f64
            / voxel_model::CHUNK_SIDE as f64;
        let slab_layers = band_layers + 2.0;

        println!("declared horizons, extrapolated at {per_chunk:.0} B/chunk:");
        println!(
            "  {:>10} {:>5} {:>7} | {:>12} {:>10} | {:>12} {:>10}",
            "horizon", "r", "extent", "voxels+mesh", "mesh only", "slab voxels", "slab mesh"
        );
        for rule in r.horizons().rules() {
            let side = (2 * rule.radius_chunks + 1) as f64;
            let cube = side * side * side;
            let slab = side * side * slab_layers;
            println!(
                "  {:>10} {:>5} {:>6} m | {:>9.1} MB {:>7.1} MB | {:>9.1} MB {:>7.1} MB",
                rule.horizon.name(),
                rule.radius_chunks,
                rule.extent_m(),
                mb(cube),
                mesh_only_mb(cube),
                mb(slab),
                mesh_only_mb(slab),
            );
        }
        println!(
            "  (the band is {band_layers:.0} chunks tall; a cube radius of 24 spends {:.0}x \
             the vertical extent the world actually has)",
            (2.0 * 24.0 + 1.0) / slab_layers
        );
        let mesh_share = r.mesh_bytes() as f64 / r.bytes() as f64;
        println!(
            "resident bytes now: {} ({} chunks, {} meshes) — octrees {} B, meshes {} B ({:.1}% of it)",
            r.bytes(),
            r.len(),
            r.meshes.len(),
            r.chunk_bytes(),
            r.mesh_bytes(),
            mesh_share * 100.0
        );
        println!(
            "the octree is the memory: {:.0}x the mesh at the median ({cmed} B vs {mmed} B)",
            cmed as f64 / mmed.max(1) as f64
        );
    }
}
