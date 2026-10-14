//! **Geometry derived from a world's cells — every voxel face a world shows, merged greedily, and the work
//! that produced it.**
//!
//! Derived from `docs/DESIGN-SPEC.md` §4 SPEC-3 (*"one world authority: U2 owns truth, U4 consumes it"*)
//! and SPEC-5 (*"four horizons, computed not copied"*), the **U4 — Meshing, streaming and work** entry of
//! `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §2 and slice 4 of its §5, and the reasoning recorded in
//! `docs/LEDGER.md` **C-43** (a quad is the *merge's* unit — 34,816 visible faces became 6 quads on a flat
//! slab — while the GPU's unit is a triangle and a *displaced* surface is not planar) and **C-46** (a number
//! that does not say which world it is of is not evidence). The binding seams are
//! `specs/004-u4-mesh-stream-work/contracts/u4-seams.md` C1–C4.
//!
//! # The three properties this module exists to make checkable
//!
//! 1. **The faces are the world's own voxels'.** A mesh is no longer a height field's rectangles: the
//!    mesher sweeps the cells a world holds — the solid a voxel is made of, the water it stands — and emits
//!    the **faces** those cells show (a side is a face where its neighbour is not solid; a column's water
//!    shows its top where the cell above holds none). The greedy merge is over those faces, so the world's
//!    own quantisation is the geometry's, and a step in the ground is a **wall**, not a new rectangle. If a
//!    field plane cannot answer, the mesh is refused **by name** — never replaced by an empty mesh, which
//!    would read as a smooth world.
//! 2. **A kept mesh cannot drift.** A resident entry's key names **the world state it was drawn from**
//!    (`GeometryKey { state, chunk, level }`) — U2's truth digest when the world is the column reading, and
//!    the **cells' own identity** ([`CubicWorld::digest`]) when it is a cell world, which is what the water
//!    law's own state digest is (`Automaton::digest` is `CubicWorld::digest`). A mesh of the law's world is
//!    therefore re-keyed **exactly** when a cell's water moves, and `Resident::serve` cannot hand back
//!    geometry from an older state: there is no key to find it under. Invalidation is consequently a
//!    memory-honesty operation (`invalidate` removes **exactly** the unreachable entries) rather than a
//!    correctness gate — which is why it removes a count the caller can print rather than something a
//!    caller must trust.
//! 3. **The work accounting is real.** Every counter is incremented where the work happens — columns read
//!    off the plane, samples read after the level's decimation, cells swept, faces emitted, faces the merge
//!    folded away, rectangles the merge closed — and the counts are checked against an **independent
//!    recount** of the same world ([`recount_faces`], a naive double loop that shares no code with the
//!    sweep) rather than recomputed from the finished mesh, which would only prove the formula.
//! 4. **The extent is declared, not inherited from the world.** What a mesh covers is a [`Footprint`]: a
//!    patch of columns and the vertical band **those** columns imply, both snapped to the chunk grid, with
//!    the ring of cells around it ([`MESH_RING`]) deciding its faces. So a world twice as wide around the
//!    same chunk draws the same chunk; a [region](Mesher::derive_region) of chunks is one footprint of the
//!    same kind, one scale up; and a world that owns no ring shows its own edge as a cliff — a declaration
//!    with a number ([`Footprint::halo_owned`]) rather than an accident of how the world was built.
//!
//! # Two doors, one merge
//!
//! [`Mesher::derive`] is the **reading's** door: the declared fields are read in one evaluation, decimated
//! to the level's tile grid, filled into a [`CubicWorld`] by the one constructor (`from_surface`) and swept.
//! [`Mesher::derive_cubic`] is the **cells'** door: a world that already holds cells is swept directly, with
//! no field read and no decimation — a cell world is not a reading of anything, so the render ladder does not
//! coarsen it and a level above `0` is refused by name rather than served at another resolution.
//! [`Mesher::derive_region`] is the **stitch's** door: one [`Footprint`] over a rectangle of chunks, one
//! sweep and one merge, so the cells across a boundary *inside* the rectangle are the world's own and no face
//! is drawn there. All three doors end in the one [`Mesher::merge_into`], so there is one owner for the faces,
//! the merge and the mesh's own state, and no second mesher.
//!
//! # What this slice does **not** do, said here rather than left to be discovered
//!
//! The block level's faces are planar: a rectangle is exactly two triangles, so the merge's unit is a quad
//! and nothing is lost. A **displaced** (256×) surface is not planar and must be subdivided and triangulated
//! with normals from the displaced geometry (C-43): **owed**. The stitch across a rectangle of chunks
//! **lands** ([`Mesher::derive_region`]): one footprint over the rectangle, swept and merged once, so the
//! boundary between two chunks inside it is the world's own inside rather than an edge. A stitched mesh
//! *wider* than one rectangle — a region with a residency entry of its own and a streamed policy over it — is
//! **owed**, because the resident set's unit is a chunk and a level. The seam rule is that ownership is
//! *declared* (this footprint's own columns) rather than discovered at a shared edge: two neighbouring chunks
//! each own the faces on their side of the shared plane, facing opposite ways. Water shows its **top** face and no sides
//! (the sides of a water column are its ground's own faces): a declared reading, not an accident.

use std::collections::{BTreeMap, BTreeSet};

use crate::chunk::{chunk_band, surface_fields, CubicWorld};
use crate::coordinates::{ChunkKey, VoxelPos, CHUNK_SIDE};
use crate::digest::Digest;
use crate::material::{MaterialId, MaterialRegistry};
use crate::refusal::{Refusal, RefusalKind};
use crate::stage::{Declared, FieldId, Patch, ProcessRequest};
use crate::world::World;

/// The declared render ladder's ceiling: level `L` reads one column per `2^L` and merges rectangles up to
/// `2^L` columns on the **reading's** door. Past it a level is **refused**, never clamped — a clamped level
/// would serve a different world's geometry under the requested name.
pub const MAX_LEVEL: u8 = 2;

// The three fields a surface mesh is derived from are the **world's own declared surface**
// (`crate::chunk::SURFACE_FIELDS`, read through `crate::chunk::surface_fields`): one owner for the list, so
// the mesher, the client and the world cannot read three different surfaces.

/// **The halo a footprint declares**: how far outside the footprint's own columns the visibility question
/// reaches. One cell — a face's neighbour is the next cell — so a world that owns the footprint plus this
/// ring decides the footprint's faces from its own contents and from nothing wider.
///
/// The reading's door *reads* exactly this much world ([`Footprint::halo_patch`]); the cells' door already
/// has it. A world that does not own its footprint's halo shows its own edge there
/// ([`Footprint::halo_owned`]), which is a declaration with a number rather than a missing read.
pub const MESH_RING: i32 = 1;

// ---------------------------------------------------------------------------
// The render ladder.
// ---------------------------------------------------------------------------

/// A rung of the render ladder. Level `L` reads one column per `2^L` and lets a rectangle span up to `2^L`
/// columns, so a coarse level is *less work* over the *same* world — which the ledger reports as fewer
/// samples read rather than hiding it. On the **cells'** door the ladder does not apply: a cell world's
/// faces are its own resolution ([`Mesher::derive_cubic`] refuses a level above `0` by name).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Level(u8);

impl Level {
    pub const ALL: [Level; (MAX_LEVEL as usize) + 1] = [Level(0), Level(1), Level(2)];

    /// A level, or `level_off_ladder`. The refusal names both the requested level and the ladder.
    pub fn new(level: u8) -> Result<Self, Refusal> {
        if level > MAX_LEVEL {
            return Err(Refusal::new(
                RefusalKind::LevelOffLadder,
                format!("level {level} is past the declared render ladder 0..={MAX_LEVEL}"),
            ));
        }
        Ok(Level(level))
    }

    pub const fn get(self) -> u8 {
        self.0
    }

    /// The columns one sample stands for at this level.
    pub const fn tile(self) -> i32 {
        1 << self.0
    }
}

// ---------------------------------------------------------------------------
// The four horizons (SPEC-5), as four types.
// ---------------------------------------------------------------------------

fn positive_radius(what: &str, metres: f64) -> Result<f64, Refusal> {
    if !metres.is_finite() || metres <= 0.0 {
        return Err(Refusal::new(
            RefusalKind::ParameterOutOfRange,
            format!("{what} = {metres} m is not a positive finite radius"),
        ));
    }
    Ok(metres)
}

/// How far the caller renders. Its own type: it is not a residency radius and cannot be passed as one.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct RenderHorizon(f64);

impl RenderHorizon {
    pub fn m(metres: f64) -> Result<Self, Refusal> {
        positive_radius("render horizon", metres).map(RenderHorizon)
    }

    pub const fn radius_m(self) -> f64 {
        self.0
    }
}

/// How far the caller simulates. The simulation horizon is where authority ends, and it is **not** the
/// render horizon: a world can be drawn further than it is simulated.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct SimulationHorizon(f64);

impl SimulationHorizon {
    pub fn m(metres: f64) -> Result<Self, Refusal> {
        positive_radius("simulation horizon", metres).map(SimulationHorizon)
    }

    pub const fn radius_m(self) -> f64 {
        self.0
    }
}

/// How far the caller keeps chunks. The only horizon that is also a **predicate** here: geometry outside it
/// is refused by name rather than derived and thrown away. It bounds the **reading's** door — a cell world
/// is where its own cells are, so the cells' door is bounded by the world's own patch instead.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct ResidencyHorizon(f64);

impl ResidencyHorizon {
    pub fn m(metres: f64) -> Result<Self, Refusal> {
        positive_radius("residency horizon", metres).map(ResidencyHorizon)
    }

    pub const fn radius_m(self) -> f64 {
        self.0
    }

    /// Whether a chunk is inside this horizon of an anchor, measured **between chunk centres** so a chunk is
    /// in or out as a whole. A chunk that cannot be expressed as an origin at all is the world's edge:
    /// outside every horizon, never silently inside one.
    pub fn keeps(self, anchor: ChunkKey, chunk: ChunkKey) -> bool {
        chunk_centre_distance_m(anchor, chunk)
            .map(|distance| distance <= self.0)
            .unwrap_or(false)
    }
}

/// How far the fields are trusted to describe the world — the fourth horizon, and the coarsest: a field
/// beyond it is a declaration about a region nobody declared.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct FieldHorizon(f64);

impl FieldHorizon {
    pub fn m(metres: f64) -> Result<Self, Refusal> {
        positive_radius("field horizon", metres).map(FieldHorizon)
    }

    pub const fn radius_m(self) -> f64 {
        self.0
    }
}

/// The distances between two chunks' centres, in metres. `None` when a key cannot be expressed as a voxel
/// origin — the edge of the coordinate system, where no centre exists.
pub fn chunk_centre_distance_m(a: ChunkKey, b: ChunkKey) -> Option<f64> {
    let half = (CHUNK_SIDE / 2) as f64;
    let (a, b) = (a.origin().ok()?, b.origin().ok()?);
    let (dx, dy, dz) = (
        (a.x as f64 + half) - (b.x as f64 + half),
        (a.y as f64 + half) - (b.y as f64 + half),
        (a.z as f64 + half) - (b.z as f64 + half),
    );
    Some((dx * dx + dy * dy + dz * dz).sqrt())
}

/// The one horizon rule that cannot be a type: rendering geometry the resident set will never keep is the
/// mistake SPEC-5 exists to prevent, so the pair is checked and refused by name with both radii.
pub fn check_horizons(render: RenderHorizon, residency: ResidencyHorizon) -> Result<(), Refusal> {
    if render.radius_m() > residency.radius_m() {
        return Err(Refusal::new(
            RefusalKind::HorizonMixed,
            format!(
                "the render horizon {:.1} m is wider than the residency horizon {:.1} m: geometry \
                 would be drawn from a set that is never kept",
                render.radius_m(),
                residency.radius_m()
            ),
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// The geometry: an axis, a layer, a face rectangle, a mesh identity.
// ---------------------------------------------------------------------------

/// Which axis a face is perpendicular to. Its own three names, because a rectangle's two spans are on the
/// **other two** axes and the frame has to say which is which.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    pub const ALL: [Axis; 3] = [Axis::X, Axis::Y, Axis::Z];

    pub const fn name(self) -> &'static str {
        match self {
            Axis::X => "x",
            Axis::Y => "y",
            Axis::Z => "z",
        }
    }

    /// The two world axes a face on this one spans, in the face frame's own `(u, v)` order.
    pub const fn spans(self) -> (&'static str, &'static str) {
        match self {
            Axis::X => ("y", "z"),
            Axis::Y => ("x", "z"),
            Axis::Z => ("x", "y"),
        }
    }

    const fn tag(self) -> u8 {
        match self {
            Axis::X => 1,
            Axis::Y => 2,
            Axis::Z => 3,
        }
    }
}

/// Which surface a rectangle belongs to. A flooded column shows **two**: the ground it stands on and the
/// water above it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Layer {
    Solid,
    Water,
}

impl Layer {
    pub const ALL: [Layer; 2] = [Layer::Solid, Layer::Water];

    pub const fn name(self) -> &'static str {
        match self {
            Layer::Solid => "solid",
            Layer::Water => "water",
        }
    }

    const fn tag(self) -> u8 {
        match self {
            Layer::Solid => 1,
            Layer::Water => 2,
        }
    }
}

/// **The merge's unit** (C-43): a rectangle of the world's own voxel **faces** on one plane, carrying one
/// material and one layer. In world coordinates — `u` and `v` are the world coordinates it starts at, on
/// the two axes [`Axis::spans`] names for its own axis — so a mesh is usable without knowing where its
/// chunk began.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Quad {
    pub layer: Layer,
    pub axis: Axis,
    /// Which way the face looks: `+1` the way the axis grows, `-1` the way it falls.
    pub facing: i8,
    /// The voxel coordinate of the plane the face lies on — the plane **just outside** the cell that shows
    /// it, so a cell at `z` shows its `+z` face on the plane `z + 1`.
    pub plane: i32,
    /// The first world coordinate of the rectangle on each of the face's own two axes.
    pub u: i32,
    pub v: i32,
    pub span_u: u32,
    pub span_v: u32,
    pub material: MaterialId,
}

impl Quad {
    /// The voxel faces this rectangle stands for: its area in faces, one per unit face.
    pub const fn area(self) -> u64 {
        self.span_u as u64 * self.span_v as u64
    }

    /// The block level is planar, so four corners are the whole rectangle.
    pub const fn vertices(self) -> u64 {
        4
    }

    /// Two triangles per rectangle.
    pub const fn triangles(self) -> u64 {
        2
    }

    /// Six indices per rectangle — stated once, in one place (`six_indices_per_quad_exactly`).
    pub const fn indices(self) -> u64 {
        6
    }

    fn write_canonical(&self, out: &mut Vec<u8>) {
        out.push(self.layer.tag());
        out.push(self.axis.tag());
        out.push(if self.facing > 0 { 1 } else { 0 });
        out.extend_from_slice(&self.plane.to_le_bytes());
        out.extend_from_slice(&self.u.to_le_bytes());
        out.extend_from_slice(&self.v.to_le_bytes());
        out.extend_from_slice(&self.span_u.to_le_bytes());
        out.extend_from_slice(&self.span_v.to_le_bytes());
        out.extend_from_slice(&self.material.0.to_le_bytes());
    }
}

/// What a resident entry is bound to: **the world state it was drawn from**, the chunk, and the level.
/// Because `state` is part of the name, a stale entry is not reachable — there is no key under which it
/// could be served.
///
/// The state is the identity of the world the faces came from, and which identity that is follows the door:
/// [`GeometryKey::new`] takes U2's truth digest (the **column reading**'s own identity, anchor and cache
/// excluded), and [`GeometryKey::of_state`] takes a state digest directly — the cells' own
/// ([`CubicWorld::digest`]) when the world is a cell world, which is the water law's state digest and moves
/// exactly when a cell's water moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GeometryKey {
    pub state: Digest,
    pub chunk: ChunkKey,
    pub level: Level,
}

impl GeometryKey {
    /// The key of geometry drawn from a **column reading** of a world: the reading's own truth digest.
    pub fn new(world: &World, chunk: ChunkKey, level: Level) -> Self {
        GeometryKey::of_state(world.truth_digest(), chunk, level)
    }

    /// The key of geometry drawn from a world already named by its state digest — a cell world's own
    /// identity, which is the law's.
    pub fn of_state(state: Digest, chunk: ChunkKey, level: Level) -> Self {
        GeometryKey { state, chunk, level }
    }
}

/// One chunk's geometry at one level, with the digest that identifies it.
#[derive(Clone, Debug, PartialEq)]
pub struct Mesh {
    key: GeometryKey,
    quads: Vec<Quad>,
    digest: Digest,
}

impl Mesh {
    /// **One mesh of one geometry key** — the declared order and the digest, in one place. `pub(crate)`
    /// since 2026-10-14e: `drawn`'s own camera guards need a mesh of known rectangles to stand a camera
    /// over, and a test beside the module that dresses them is not a second derivation ([`Mesher`] is the
    /// only door that derives one).
    pub(crate) fn of(key: GeometryKey, mut quads: Vec<Quad>) -> Self {
        // The declared order, and the digest's: layer, then the face's own frame — axis, facing, plane,
        // `u`, `v`. Sorted here rather than trusted to the merge's visit order, so two derivations of one
        // world are **the same bytes** rather than two orderings of one set.
        quads.sort_by_key(|quad| (quad.layer, quad.axis, quad.facing, quad.plane, quad.u, quad.v));
        let digest = mesh_digest(key, &quads);
        Mesh { key, quads, digest }
    }

    pub const fn key(&self) -> GeometryKey {
        self.key
    }

    /// The rectangles, in the declared order: `Solid` before `Water`, then axis, facing, plane, `u`, `v`.
    pub fn quads(&self) -> &[Quad] {
        &self.quads
    }

    pub const fn digest(&self) -> Digest {
        self.digest
    }

    pub fn vertices(&self) -> u64 {
        self.quads.iter().map(|quad| quad.vertices()).sum()
    }

    pub fn triangles(&self) -> u64 {
        self.quads.iter().map(|quad| quad.triangles()).sum()
    }

    pub fn indices(&self) -> u64 {
        self.quads.iter().map(|quad| quad.indices()).sum()
    }

    /// The voxel faces the rectangles stand for — the sum of their areas. Recomputed from the geometry, so
    /// it can disagree with the ledger's own face count, which is the point of checking one against the
    /// other.
    pub fn area(&self) -> u64 {
        self.quads.iter().map(|quad| quad.area()).sum()
    }
}

/// The mesh's identity: its key, the number of rectangles, and every rectangle in the declared order. One
/// key, one quad list, one digest — and moving one rectangle moves the digest. The domain is `/2`: the unit
/// changed from a column-height rectangle to a voxel **face**, so an old digest may not be read as one of
/// these.
fn mesh_digest(key: GeometryKey, quads: &[Quad]) -> Digest {
    let mut bytes = Vec::with_capacity(64 + quads.len() * 32);
    bytes.extend_from_slice(b"shploingus.mesh/2");
    bytes.extend_from_slice(key.state.as_bytes());
    bytes.extend_from_slice(&key.chunk.x.to_le_bytes());
    bytes.extend_from_slice(&key.chunk.y.to_le_bytes());
    bytes.extend_from_slice(&key.chunk.z.to_le_bytes());
    bytes.push(key.level.get());
    bytes.extend_from_slice(&(quads.len() as u64).to_le_bytes());
    for quad in quads {
        quad.write_canonical(&mut bytes);
    }
    Digest::of(&bytes)
}

// ---------------------------------------------------------------------------
// The palette: class → material, declared.
// ---------------------------------------------------------------------------

/// The declared palette: which **admitted** material each lithology class renders as, and which material
/// stands water. The palette itself is the user's; this type owns only the refusal — a class the ladder
/// cannot name is refused `unadmitted_material` rather than rendered as a plausible rock.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ladder {
    classes: Vec<MaterialId>,
    water: MaterialId,
}

impl Ladder {
    /// One admitted material per class, in class order. An id the registry has not admitted is refused by
    /// name, so a ladder cannot promise a material the world does not have.
    pub fn declare(
        registry: &MaterialRegistry,
        classes: &[MaterialId],
        water: MaterialId,
    ) -> Result<Self, Refusal> {
        if classes.is_empty() {
            return Err(Refusal::new(
                RefusalKind::ParameterOutOfRange,
                "the ladder declares no classes: a palette with no entries names nothing",
            ));
        }
        for id in classes.iter().chain(std::iter::once(&water)) {
            if !registry.is_admitted(*id) {
                return Err(Refusal::new(
                    RefusalKind::UnadmittedMaterial,
                    format!("the ladder names material {} , which is not admitted", id.0),
                ));
            }
        }
        Ok(Ladder { classes: classes.to_vec(), water })
    }

    /// A palette laid over `class_count` classes **in declared order**. When the palette is narrower than
    /// the class count, neighbouring classes share a material — visible in [`Ladder::mapping`] and printed
    /// by the binary, because a rendering decision that is not visible is a decision nobody made. The
    /// mapping is monotone: a class never maps to a material declared later than a higher class's.
    pub fn over_palette(
        registry: &MaterialRegistry,
        palette: &[MaterialId],
        class_count: usize,
        water: MaterialId,
    ) -> Result<Self, Refusal> {
        if class_count == 0 {
            return Err(Refusal::new(
                RefusalKind::ParameterOutOfRange,
                "a ladder over zero classes names nothing",
            ));
        }
        if palette.is_empty() {
            return Err(Refusal::new(
                RefusalKind::ParameterOutOfRange,
                "a ladder over an empty palette names nothing",
            ));
        }
        let classes: Vec<MaterialId> = (0..class_count)
            .map(|class| palette[(class * palette.len()) / class_count])
            .collect();
        Ladder::declare(registry, &classes, water)
    }

    /// The class count this ladder can name.
    pub fn classes(&self) -> usize {
        self.classes.len()
    }

    pub const fn water(&self) -> MaterialId {
        self.water
    }

    /// The material of a lithology class value, or `unadmitted_material` naming the class.
    pub fn for_class(&self, class: i64) -> Result<MaterialId, Refusal> {
        match usize::try_from(class).ok().filter(|class| *class < self.classes.len()) {
            Some(class) => Ok(self.classes[class]),
            None => Err(Refusal::new(
                RefusalKind::UnadmittedMaterial,
                format!(
                    "lithology class {class} is outside the declared ladder, which names classes \
                     0..{}",
                    self.classes.len().saturating_sub(1)
                ),
            )),
        }
    }

    /// The declared class → material mapping, as printed evidence rather than a private variable.
    pub fn mapping(&self) -> Vec<(usize, MaterialId)> {
        self.classes.iter().copied().enumerate().collect()
    }
}

// ---------------------------------------------------------------------------
// The work: a ledger counted where the work happens, and a budget that refuses.
// ---------------------------------------------------------------------------

/// What a derivation **actually did**. Every counter is incremented at the place the work happens; none is
/// computed from the finished mesh (which would only check the formula).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorkLedger {
    /// World columns the field plane produced for the patch — one per `(x, y)` of the request. Zero on the
    /// cells' door, which reads no plane.
    pub columns_produced: u64,
    /// Columns actually read: the same count at level 0, decimated at a coarse level.
    pub samples_read: u64,
    /// Columns of the request the world owns — the columns the sweep stood on.
    pub columns_swept: u64,
    /// Voxels the sweep found **solid** (a column under water has cells above it that are not).
    pub cells: u64,
    /// Faces the sweep emitted, before any merging — the merge's own raw material.
    pub faces: u64,
    /// Faces the merge folded away: `faces - quads`. A merge that closed nothing leaves this at zero.
    pub merged_away: u64,
    /// Rectangles that grew past a single face — the merge did something here.
    pub rectangles_merged: u64,
    pub quads: u64,
    pub vertices: u64,
    pub indices: u64,
    /// Requests served from the resident set: no field read, no sweep, no merge.
    pub hits: u64,
    /// Requests that had to be derived.
    pub misses: u64,
    /// Refusals returned, counted where they were decided.
    pub refusals: u64,
}

impl WorkLedger {
    /// Add another ledger's work. Integer addition, so the total is order-free and the worker count cannot
    /// move it.
    pub fn add(&mut self, other: &WorkLedger) {
        self.columns_produced += other.columns_produced;
        self.samples_read += other.samples_read;
        self.columns_swept += other.columns_swept;
        self.cells += other.cells;
        self.faces += other.faces;
        self.merged_away += other.merged_away;
        self.rectangles_merged += other.rectangles_merged;
        self.quads += other.quads;
        self.vertices += other.vertices;
        self.indices += other.indices;
        self.hits += other.hits;
        self.misses += other.misses;
        self.refusals += other.refusals;
    }

    /// Everything counted, summed — a single number that is only ever used to say the ledger moved.
    pub fn total(&self) -> u64 {
        self.columns_produced
            + self.samples_read
            + self.columns_swept
            + self.cells
            + self.faces
            + self.merged_away
            + self.rectangles_merged
            + self.quads
            + self.vertices
            + self.indices
            + self.hits
            + self.misses
            + self.refusals
    }

    /// The faces each rectangle carries: the merge factor, from the work rather than the mesh.
    pub fn merge_factor(&self) -> f64 {
        if self.quads == 0 {
            0.0
        } else {
            self.faces as f64 / self.quads as f64
        }
    }
}

/// A ceiling, not a target. Exceeding it is a **refusal with both numbers**, so the caller learns what the
/// request needed as well as what it allowed — and nothing is cached, because a partial mesh is a wrong
/// mesh and a cached partial mesh is a wrong mesh that survives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Budget {
    pub max_columns: u64,
    pub max_quads: u64,
}

impl Budget {
    pub const fn new(max_columns: u64, max_quads: u64) -> Self {
        Budget { max_columns, max_quads }
    }

    /// No ceiling declared. Still declared: this is a choice with a name, not a default.
    pub const fn unlimited() -> Self {
        Budget { max_columns: u64::MAX, max_quads: u64::MAX }
    }

    /// Whether a request that would produce `columns` columns and `quads` rectangles fits.
    pub const fn allows(&self, columns: u64, quads: u64) -> bool {
        columns <= self.max_columns && quads <= self.max_quads
    }

    fn refuse_columns(&self, needed: u64) -> Refusal {
        Refusal::new(
            RefusalKind::GeometryBudgetExceeded,
            format!(
                "this request would produce {needed} columns and the declared budget allows {}",
                self.max_columns
            ),
        )
    }

    fn refuse_quads(&self, needed: u64) -> Refusal {
        Refusal::new(
            RefusalKind::GeometryBudgetExceeded,
            format!(
                "this derivation merged {needed} rectangles and the declared budget allows {}",
                self.max_quads
            ),
        )
    }
}

// ---------------------------------------------------------------------------
// The resident set.
// ---------------------------------------------------------------------------

/// **What a request may cost**: a ceiling and a worker count, grouped because they are the two knobs of one
/// decision. Neither is an input to the geometry — a change of workers moves no quad and no digest, and a
/// change of ceiling moves no mesh that fits — which is why they are not fields of the mesher, where they
/// would look like part of the world.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Effort {
    pub budget: Budget,
    pub workers: usize,
}

impl Effort {
    pub const fn new(budget: Budget, workers: usize) -> Self {
        Effort { budget, workers }
    }

    /// No ceiling and one worker: the most literal way to ask, declared rather than defaulted.
    pub const fn plain() -> Self {
        Effort { budget: Budget::unlimited(), workers: 1 }
    }
}

/// The kept geometry — a **cache of geometry**, never a second copy of the world. U2 owns truth and the
/// residency of chunks; this owns the quads derived from a state that may no longer be current.
#[derive(Clone, Debug, Default)]
pub struct Resident {
    entries: BTreeMap<GeometryKey, Mesh>,
}

impl Resident {
    pub fn new() -> Self {
        Resident::default()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The geometry drawn from the world's **current state**, or nothing. A stale entry cannot be returned:
    /// its key names a different state, so this lookup has nothing to find. Never an `Option` that a caller
    /// could mistake for "no geometry in this world".
    pub fn serve(&self, state: Digest, chunk: ChunkKey, level: Level) -> Option<&Mesh> {
        self.entries.get(&GeometryKey::of_state(state, chunk, level))
    }

    /// Keep a mesh. Returns whether it was newly kept (an identical key is one entry).
    pub fn insert(&mut self, mesh: &Mesh) -> bool {
        self.entries.insert(mesh.key(), mesh.clone()).is_none()
    }

    /// How many kept entries were drawn from a state that is no longer current.
    pub fn stale(&self, state: Digest) -> usize {
        self.entries.keys().filter(|key| key.state != state).count()
    }

    /// Remove **exactly** the entries whose state is no longer current, and keep every entry whose state is
    /// current. Returns how many were removed, so the caller prints a count rather than reporting a hope.
    pub fn invalidate(&mut self, state: Digest) -> usize {
        let before = self.entries.len();
        self.entries.retain(|key, _| key.state == state);
        before - self.entries.len()
    }

    /// The kept keys, ascending.
    pub fn keys(&self) -> Vec<GeometryKey> {
        self.entries.keys().copied().collect()
    }

    /// The resident set's own identity: order-free (the keys are sorted), and sensitive to which meshes are
    /// kept — so "the cache is a set" is a digest rather than a promise.
    pub fn digest(&self) -> Digest {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"shploingus.resident/2");
        for (key, mesh) in &self.entries {
            bytes.extend_from_slice(key.state.as_bytes());
            bytes.extend_from_slice(&key.chunk.x.to_le_bytes());
            bytes.extend_from_slice(&key.chunk.y.to_le_bytes());
            bytes.extend_from_slice(&key.chunk.z.to_le_bytes());
            bytes.push(key.level.get());
            bytes.extend_from_slice(mesh.digest().as_bytes());
        }
        Digest::of(&bytes)
    }
}

// ---------------------------------------------------------------------------
// The extent: one declared footprint, and the halo that decides its faces.
// ---------------------------------------------------------------------------

/// **One extent contract, named.** What is meshed is a **footprint**: a square patch of columns, and the
/// vertical band those columns' own contents occupy, both snapped outwards to the chunk boundary. What
/// decides the footprint's faces is a **halo** ([`MESH_RING`]): the one cell around it that its visible
/// surface can be a boundary with.
///
/// Three consequences, each measured rather than promised:
///
/// * A footprint's own boundary is **interior by declaration**. Below its floor the same ground continues —
///   the band is one of a stack of bands, and the band below holds the very cells this one stands on —
///   and above its ceiling there is air, because the ceiling is strictly above the highest cell the
///   footprint's columns hold. Only the halo is a real edge, so a chunk's faces no longer depend on how
///   much wider world its world happens to be: that is the artifact this type exists to close.
/// * The span is derived from **the footprint's own columns**, never from the world's: the chunk boundary
///   at or below the lowest ground those columns stand on, out to the chunk boundary strictly above the
///   highest cell they hold. It is the same law the world's own cells were filled by
///   ([`chunk_band`](crate::chunk::chunk_band)), one scale down, so a one-chunk footprint and a
///   rectangle-of-chunks footprint are extents of one kind.
/// * Where the world **does** own the halo, the footprint's faces are a function of the world's cells in
///   the footprint and its ring — and of nothing else in it ([`Footprint::halo_owned`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Footprint {
    patch: Patch,
    span: (i32, i32),
}

impl Footprint {
    /// The footprint of `patch` held in `world`: the patch's own columns, and the span **those** columns
    /// imply. Columns the world does not own contribute nothing, and a footprint over a world that holds
    /// none of it is refused by name rather than answered with an empty mesh that would read as flat
    /// ground.
    pub fn over(world: &CubicWorld, patch: Patch) -> Result<Footprint, Refusal> {
        let own = world.patch();
        let origin = patch.origin();
        let side = patch.side() as i32;
        let mut lowest_ground: Option<i32> = None;
        let mut highest_cell: Option<i32> = None;
        for ty in 0..side {
            for tx in 0..side {
                let (x, y) = (origin.x + tx, origin.y + ty);
                if !columns_hold(own, x, y) {
                    continue;
                }
                let (ground, top) = column_span(world, x, y)?;
                if let Some(ground) = ground {
                    lowest_ground = Some(lowest_ground.map_or(ground, |low| low.min(ground)));
                }
                if let Some(top) = top {
                    highest_cell = Some(highest_cell.map_or(top, |high| high.max(top)));
                }
            }
        }
        let (Some(lowest_ground), Some(highest_cell)) = (lowest_ground, highest_cell) else {
            return Err(Refusal::new(
                RefusalKind::GeometryOutsideTheWorld,
                format!(
                    "a {side}×{side} footprint at ({}, {}) holds no column of this world's own patch \
                     (x {}..{}, y {}..{}): there are no cells of this world to mesh",
                    origin.x,
                    origin.y,
                    own.origin().x,
                    own.origin().x + own.side() as i32 - 1,
                    own.origin().y,
                    own.origin().y + own.side() as i32 - 1
                ),
            ));
        };
        let (floor_z, ceiling_z) = chunk_band(lowest_ground, highest_cell);
        Ok(Footprint { patch, span: (floor_z, ceiling_z) })
    }

    /// The footprint of one chunk: its patch, and the span that chunk's own columns imply.
    pub fn of_chunk(world: &CubicWorld, chunk: ChunkKey) -> Result<Footprint, Refusal> {
        let origin = chunk.origin().map_err(|kind| {
            Refusal::new(
                kind,
                format!("chunk ({}, {}, {}) has no expressible origin", chunk.x, chunk.y, chunk.z),
            )
        })?;
        Footprint::over(world, Patch::new(origin, CHUNK_SIDE as u32)?)
    }

    /// The columns the footprint stands on.
    pub const fn patch(self) -> Patch {
        self.patch
    }

    /// The vertical band the footprint covers: `[floor, ceiling)`, both chunk boundaries.
    pub const fn span(self) -> (i32, i32) {
        self.span
    }

    pub const fn floor_z(self) -> i32 {
        self.span.0
    }

    pub const fn ceiling_z(self) -> i32 {
        self.span.1
    }

    /// The columns the footprint declares.
    pub const fn columns(self) -> u64 {
        (self.patch.side() as u64) * (self.patch.side() as u64)
    }

    /// The cells of the footprint's own lattice: its columns by the height of its span.
    pub const fn cells(self) -> u64 {
        self.columns() * ((self.span.1 - self.span.0) as u64)
    }

    /// Whether `(x, y, z)` is a cell of the footprint's own lattice.
    pub fn holds(self, x: i32, y: i32, z: i32) -> bool {
        columns_hold(self.patch, x, y) && z >= self.span.0 && z < self.span.1
    }

    /// The columns the sweep may ask about **outside** the footprint's own patch: the patch grown by
    /// [`MESH_RING`]. The reading's door reads exactly this patch and fills a world over it, which is what
    /// makes its world own the halo.
    pub fn halo_patch(self) -> Result<Patch, Refusal> {
        let origin = self.patch.origin();
        let side = self.patch.side() as i32 + 2 * MESH_RING;
        Patch::new(
            VoxelPos::new(origin.x - MESH_RING, origin.y - MESH_RING, origin.z),
            side as u32,
        )
    }

    /// Whether `world` owns the whole halo: every column of the ring around the footprint's patch, over the
    /// footprint's own span. When this holds, the footprint's faces are decided by the world's cells in this
    /// footprint and its ring, and by nothing else — which is what makes the contract a contract rather than
    /// a habit. When it does not, the faces the world's own edge adds are named by the caller.
    pub fn halo_owned(self, world: &CubicWorld) -> bool {
        let own = world.patch();
        let (floor_z, ceiling_z) = world.span_z();
        if self.span.0 < floor_z || self.span.1 > ceiling_z {
            return false;
        }
        let origin = self.patch.origin();
        let side = self.patch.side() as i32;
        for t in -MESH_RING..side + MESH_RING {
            for (x, y) in [
                (origin.x + t, origin.y - MESH_RING),
                (origin.x + t, origin.y + side),
                (origin.x - MESH_RING, origin.y + t),
                (origin.x + side, origin.y + t),
            ] {
                let inside = x >= origin.x && x < origin.x + side && y >= origin.y && y < origin.y + side;
                if inside {
                    continue;
                }
                if !columns_hold(own, x, y) {
                    return false;
                }
            }
        }
        true
    }

    /// The halo's own columns, counted whether or not the world owns them.
    pub const fn halo_columns(self) -> u64 {
        let side = self.patch.side() as u64 + 2 * (MESH_RING as u64);
        side * side
    }
}

/// One column's own vertical content: the **ground** it stands on — the highest solid cell of its stack —
/// and the highest cell it holds at all, which is its water's surface when it stands water and its ground
/// when it is dry. Both `None` for a column the world holds nothing in. This is what a footprint's span is
/// derived from, so a footprint's extent is a function of its own columns rather than of its world's size.
fn column_span(cube: &CubicWorld, x: i32, y: i32) -> Result<(Option<i32>, Option<i32>), Refusal> {
    let (floor_z, ceiling_z) = cube.span_z();
    let mut ground = None;
    let mut top = None;
    for z in floor_z..ceiling_z {
        let cell = cube.cell(VoxelPos::new(x, y, z))?;
        if cell.water_um > 0 || cell.is_solid() {
            top = Some(z);
        }
        if cell.is_solid() {
            ground = Some(z);
        }
    }
    Ok((ground, top))
}

// ---------------------------------------------------------------------------
// The faces: the world's own voxels, swept.
// ---------------------------------------------------------------------------

/// One voxel face, before the merge: a unit square on a plane, waiting to be folded into its neighbours.
/// The merge's own raw material, and never public — a caller reads the rectangles, and the faces are what a
/// rectangle is *made of*.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Face {
    layer: Layer,
    axis: Axis,
    facing: i8,
    plane: i32,
    u: i32,
    v: i32,
    material: MaterialId,
}

/// **What a sweep found**, counted where it looked: the columns of the request the world owns, the voxels
/// that are solid, and the faces those voxels show. The evidence checks the ledger against
/// [`recount_faces`]' own numbers, so a sweep that began miscounting would show as two numbers rather than
/// as one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FaceCount {
    pub columns: u64,
    pub solid_cells: u64,
    pub solid_faces: u64,
    pub water_faces: u64,
}

impl FaceCount {
    pub const fn total_faces(self) -> u64 {
        self.solid_faces + self.water_faces
    }
}

/// The six declared sides of a cell: which way the face looks, and which neighbour decides whether it is
/// there.
const SIDES: [(Axis, i8, i32, i32, i32); 6] = [
    (Axis::X, -1, -1, 0, 0),
    (Axis::X, 1, 1, 0, 0),
    (Axis::Y, -1, 0, -1, 0),
    (Axis::Y, 1, 0, 1, 0),
    (Axis::Z, -1, 0, 0, -1),
    (Axis::Z, 1, 0, 0, 1),
];

/// The plane a side's face lies on, and the two world coordinates it starts at: `(plane, u, v)`.
fn face_frame(axis: Axis, facing: i8, x: i32, y: i32, z: i32) -> (i32, i32, i32) {
    let outside = i32::from(facing > 0);
    match axis {
        Axis::X => (x + outside, y, z),
        Axis::Y => (y + outside, x, z),
        Axis::Z => (z + outside, x, y),
    }
}

/// Whether a column is inside a patch's own columns. The patch is square and its origin is a corner.
fn columns_hold(patch: Patch, x: i32, y: i32) -> bool {
    let origin = patch.origin();
    let side = patch.side() as i32;
    x >= origin.x && y >= origin.y && x < origin.x + side && y < origin.y + side
}

/// **Whether the cell a face's neighbour is in is solid** — the question the whole extent contract is about,
/// so it is asked of a [`Footprint`] rather than of a world alone.
///
/// Three answers, in declared order:
///
/// 1. **The footprint's own floor is interior.** One cell below it is solid by declaration: the band below
///    holds the ground this band stands on, and a mesh that stopped at a band's floor and called it an edge
///    would draw the inside of the world.
/// 2. Where the world holds the cell — inside the footprint's own lattice, or in the halo around it — the
///    world answers.
/// 3. Where it does not, the answer is air: the world's own edge is visible because there is nothing there,
///    named rather than assumed. That is the rule a world that does not own its footprint's halo is read by,
///    and it is why [`Footprint::halo_owned`] is worth printing.
fn solid_at(cube: &CubicWorld, footprint: Footprint, x: i32, y: i32, z: i32) -> Result<bool, Refusal> {
    if z == footprint.floor_z() - MESH_RING {
        return Ok(true);
    }
    let (floor_z, ceiling_z) = cube.span_z();
    if !columns_hold(cube.patch(), x, y) || z < floor_z || z >= ceiling_z {
        return Ok(false);
    }
    Ok(cube.cell(VoxelPos::new(x, y, z))?.is_solid())
}

/// Whether the cell the world is asked about holds water. Not owned is dry for the same reason not owned is
/// not solid; the footprint is carried so that the two questions are asked of one declaration.
fn water_at(cube: &CubicWorld, _footprint: Footprint, x: i32, y: i32, z: i32) -> Result<bool, Refusal> {
    let (floor_z, ceiling_z) = cube.span_z();
    if !columns_hold(cube.patch(), x, y) || z < floor_z || z >= ceiling_z {
        return Ok(false);
    }
    Ok(cube.cell(VoxelPos::new(x, y, z))?.water_um > 0)
}

/// **The sweep.** Every column of the footprint the world owns, every cell of the footprint's own span,
/// every side: one pass that emits the faces the world's voxels show — and counts, where the work happens,
/// the columns it stood on, the cells it found and the faces it emitted. No merge happens here and no
/// rectangle exists yet: this is the raw material [`merge_faces`] folds.
///
/// The span is the footprint's, not the world's: what a chunk's mesh covers is decided by the chunk's own
/// columns (and a rectangle of chunks by the rectangle's), so a wider world adds no cells to a mesh and
/// takes none away.
fn sweep(cube: &CubicWorld, footprint: Footprint, water: MaterialId) -> Result<(Vec<Face>, FaceCount), Refusal> {
    let (floor_z, ceiling_z) = footprint.span();
    let origin = footprint.patch().origin();
    let side = footprint.patch().side() as i32;
    let mut faces = Vec::new();
    let mut count = FaceCount::default();
    for ty in 0..side {
        for tx in 0..side {
            let (x, y) = (origin.x + tx, origin.y + ty);
            if !columns_hold(cube.patch(), x, y) {
                continue;
            }
            count.columns += 1;
            for z in floor_z..ceiling_z {
                let cell = cube.cell(VoxelPos::new(x, y, z))?;
                // The water's own surface: the top of the water a column stands. A cell under the top shows
                // nothing — water's sides are its ground's own faces — and a cell above it holds none.
                if cell.water_um > 0 && !water_at(cube, footprint, x, y, z + 1)? {
                    let (plane, u, v) = face_frame(Axis::Z, 1, x, y, z);
                    faces.push(Face {
                        layer: Layer::Water,
                        axis: Axis::Z,
                        facing: 1,
                        plane,
                        u,
                        v,
                        material: water,
                    });
                    count.water_faces += 1;
                }
                if !cell.is_solid() {
                    continue;
                }
                count.solid_cells += 1;
                let Some(material) = cell.material else {
                    continue;
                };
                for (axis, facing, dx, dy, dz) in SIDES {
                    if solid_at(cube, footprint, x + dx, y + dy, z + dz)? {
                        continue;
                    }
                    let (plane, u, v) = face_frame(axis, facing, x, y, z);
                    faces.push(Face { layer: Layer::Solid, axis, facing, plane, u, v, material });
                    count.solid_faces += 1;
                }
            }
        }
    }
    Ok((faces, count))
}

/// **An independent recount of the same faces** — the naive double loop the evidence checks the sweep
/// against. It shares no code with the sweep: it asks the world one cell at a time and applies the same two
/// declared rules (a solid's side is a face where its neighbour is not solid; a column's water shows its top
/// where the cell above holds none), so the two numbers are two implementations of one rule rather than one
/// implementation counted twice.
///
/// A position the world does not own is air and dry here for the same reason it is in the sweep, and the
/// footprint's own floor is interior here for the same reason too: below it the same ground continues. A
/// position inside the patch and the span that the world nevertheless refuses to answer for is read as **not
/// solid and dry** rather than as an error — this function measures the faces of the cells a world holds,
/// and a cell it does not hold is not one of them; the sweep is the door that refuses by name when a cell it
/// stands on cannot be read.
pub fn recount_faces(cube: &CubicWorld, footprint: Footprint) -> FaceCount {
    let patch = cube.patch();
    let (floor_z, ceiling_z) = cube.span_z();
    let origin = footprint.patch().origin();
    let side = footprint.patch().side() as i32;
    let held = |x: i32, y: i32, z: i32| -> Option<crate::chunk::Cell> {
        if !columns_hold(patch, x, y) || z < floor_z || z >= ceiling_z {
            return None;
        }
        cube.cell(VoxelPos::new(x, y, z)).ok()
    };
    // The one declaration the sweep makes outside a world's own cells, made here too.
    let solid = |x: i32, y: i32, z: i32| -> bool {
        if z == footprint.floor_z() - MESH_RING {
            return true;
        }
        held(x, y, z).map(|cell| cell.is_solid()).unwrap_or(false)
    };
    let mut count = FaceCount::default();
    for ty in 0..side {
        for tx in 0..side {
            let (x, y) = (origin.x + tx, origin.y + ty);
            if !columns_hold(patch, x, y) {
                continue;
            }
            count.columns += 1;
            for z in footprint.floor_z()..footprint.ceiling_z() {
                let is_solid = solid(x, y, z);
                let water = held(x, y, z).map(|cell| cell.water_um > 0).unwrap_or(false);
                if water && !held(x, y, z + 1).map(|cell| cell.water_um > 0).unwrap_or(false) {
                    count.water_faces += 1;
                }
                if !is_solid {
                    continue;
                }
                count.solid_cells += 1;
                for (_, _, dx, dy, dz) in SIDES {
                    if !solid(x + dx, y + dy, z + dz) {
                        count.solid_faces += 1;
                    }
                }
            }
        }
    }
    count
}

/// What the merge itself did. Counted while it scans and grows, not afterwards.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct MergeTally {
    faces: u64,
    rectangles_merged: u64,
}

/// One merge group: the faces of one plane that a rectangle must agree on — its layer, axis, facing,
/// plane and material — each held as the `(u, v)` corner it stands at. A rectangle can never cross a
/// group's boundary, so the groups are the merge's own unit of work.
type FaceGroup = (Layer, Axis, i8, i32, MaterialId);
type FaceGroupPoints = BTreeMap<FaceGroup, BTreeSet<(i32, i32)>>;

/// **The greedy merge: over the faces themselves.** Faces are grouped by what a rectangle must agree on —
/// layer, axis, facing, plane and material — and inside each group the largest rectangle is taken at each
/// unused face, grown **right, then down**, exactly once per face. Every face belongs to exactly one
/// rectangle, so the rectangles' areas sum to the face count, which is what the ledger's independent
/// recount checks.
fn merge_faces(faces: Vec<Face>) -> (Vec<Quad>, MergeTally) {
    let mut groups = FaceGroupPoints::new();
    for face in faces {
        groups
            .entry((face.layer, face.axis, face.facing, face.plane, face.material))
            .or_default()
            .insert((face.u, face.v));
    }
    let mut quads = Vec::new();
    let mut tally = MergeTally::default();
    for ((layer, axis, facing, plane, material), mut points) in groups {
        tally.faces += points.len() as u64;
        // The scan order, declared: ascending `v`, then ascending `u` — so the rectangles a derivation
        // closes are a function of the faces and not of a hash order.
        let mut ordered: Vec<(i32, i32)> = points.iter().copied().collect();
        ordered.sort_unstable_by_key(|(u, v)| (*v, *u));
        for (u, v) in ordered {
            if !points.contains(&(u, v)) {
                continue;
            }
            let mut width = 1;
            while points.contains(&(u + width, v)) {
                width += 1;
            }
            let mut depth = 1;
            'grow: loop {
                for du in 0..width {
                    if !points.contains(&(u + du, v + depth)) {
                        break 'grow;
                    }
                }
                depth += 1;
            }
            for dv in 0..depth {
                for du in 0..width {
                    points.remove(&(u + du, v + dv));
                }
            }
            if (width as u64) * (depth as u64) > 1 {
                tally.rectangles_merged += 1;
            }
            quads.push(Quad {
                layer,
                axis,
                facing,
                plane,
                u,
                v,
                span_u: width as u32,
                span_v: depth as u32,
                material,
            });
        }
    }
    (quads, tally)
}

// ---------------------------------------------------------------------------
// The derivation.
// ---------------------------------------------------------------------------

/// **The mesher.** One roster (U3's), one declared palette, one declared residency horizon — and nothing
/// else is an input. Which world is being drawn is the *request's* argument rather than a field: the same
/// mesher draws the reading of a world ([`Mesher::derive`]) and the cells of one ([`Mesher::derive_cubic`]),
/// and neither the palette nor the horizon can drift between the two.
pub struct Mesher<'a> {
    roster: &'a Declared,
    ladder: Ladder,
    residency: ResidencyHorizon,
}

impl<'a> Mesher<'a> {
    pub fn new(roster: &'a Declared, ladder: Ladder, residency: ResidencyHorizon) -> Self {
        Mesher { roster, ladder, residency }
    }

    pub fn ladder(&self) -> &Ladder {
        &self.ladder
    }

    pub const fn residency_horizon(&self) -> ResidencyHorizon {
        self.residency
    }

    /// Derive one chunk's geometry at one level from the world's **declared fields**, keeping it in
    /// `resident` and counting the work into `ledger`. A hit does no work at all; a refusal leaves the work
    /// it did in the ledger and caches nothing.
    ///
    /// **The resident set is consulted before the horizon.** Geometry that has already been derived is
    /// served whatever the anchor now says, because the anchor is a *view* and is not an input to the
    /// geometry (FR-007); the residency horizon is a limit on **derivation** — it refuses the work of
    /// building geometry that would not be kept.
    pub fn derive(
        &self,
        world: &World,
        chunk: ChunkKey,
        level: Level,
        effort: Effort,
        resident: &mut Resident,
        ledger: &mut WorkLedger,
    ) -> Result<Mesh, Refusal> {
        let state = world.truth_digest();
        if let Some(mesh) = resident.serve(state, chunk, level) {
            ledger.hits += 1;
            return Ok(mesh.clone());
        }
        if !self.residency.keeps(world.anchor(), chunk) {
            ledger.refusals += 1;
            return Err(out_of_residency(self.residency, world.anchor(), chunk));
        }
        let (result, done) = self.build(world, chunk, level, effort);
        ledger.add(&done);
        result.inspect(|mesh| {
            resident.insert(mesh);
        })
    }

    /// Derive several chunks across `effort.workers` threads, returned in the **declared order** of the
    /// request. The worker count changes no quad and no digest (FR-003): each chunk is a pure function of
    /// the world, and the cache and the ledger are written in declared order afterwards. A refusal in any
    /// chunk is the call's refusal — the work that happened is still counted.
    pub fn derive_many(
        &self,
        world: &World,
        chunks: &[ChunkKey],
        level: Level,
        effort: Effort,
        resident: &mut Resident,
        ledger: &mut WorkLedger,
    ) -> Result<Vec<Mesh>, Refusal> {
        let workers = effort.workers.max(1);
        let state = world.truth_digest();
        let mut out: Vec<Option<Mesh>> = vec![None; chunks.len()];
        let mut missing: Vec<(usize, ChunkKey)> = Vec::new();
        for (index, chunk) in chunks.iter().enumerate() {
            match resident.serve(state, *chunk, level) {
                Some(mesh) => {
                    ledger.hits += 1;
                    out[index] = Some(mesh.clone());
                }
                None => {
                    // Served first, gated second: the horizon refuses *work*, not an answer.
                    if !self.residency.keeps(world.anchor(), *chunk) {
                        ledger.refusals += 1;
                        return Err(out_of_residency(self.residency, world.anchor(), *chunk));
                    }
                    missing.push((index, *chunk));
                }
            }
        }

        if !missing.is_empty() {
            let per_thread = missing.len().div_ceil(workers).max(1);
            let mut built: Vec<(usize, Result<Mesh, Refusal>, WorkLedger)> =
                Vec::with_capacity(missing.len());
            std::thread::scope(|scope| {
                let mut handles = Vec::new();
                for slice in missing.chunks(per_thread) {
                    handles.push(scope.spawn(move || {
                        slice
                            .iter()
                            .map(|(index, chunk)| {
                                let (result, done) = self.build(world, *chunk, level, effort);
                                (*index, result, done)
                            })
                            .collect::<Vec<_>>()
                    }));
                }
                for handle in handles {
                    built.extend(handle.join().expect("a derivation thread does not panic"));
                }
            });
            built.sort_by_key(|(index, _, _)| *index);
            for (index, result, done) in built {
                ledger.add(&done);
                let mesh = result?;
                resident.insert(&mesh);
                out[index] = Some(mesh);
            }
        }

        Ok(out
            .into_iter()
            .map(|mesh| mesh.expect("every chunk was either served or built"))
            .collect())
    }

    /// Derive one chunk's geometry from a world that already **holds cells** — the law's own world, or any
    /// [`CubicWorld`]. No field is read and nothing is decimated: the faces are the cells' own, keyed by the
    /// world's own identity ([`CubicWorld::digest`]), which is the water law's state digest.
    ///
    /// **The render ladder does not coarsen a cell world.** The level's decimation is a reading's device
    /// (one sample per `2^level` columns of a field plane); a cell world is not a reading, so a level above
    /// `0` is refused **by name** rather than served at another resolution.
    ///
    /// **The residency horizon is the world's own patch here.** A cell world declares where it is; the
    /// request's chunk is clipped to that patch's columns, and a chunk with no column under it is refused by
    /// name ([`RefusalKind::GeometryOutsideTheWorld`]) rather than answered with an empty mesh — which would
    /// read as smooth ground.
    ///
    /// **What is meshed is the chunk's footprint** ([`Footprint::of_chunk`]): the chunk's columns, and the
    /// span *those* columns imply — not the world's. A world twice as tall around the same chunk draws the
    /// same chunk.
    pub fn derive_cubic(
        &self,
        cube: &CubicWorld,
        chunk: ChunkKey,
        level: Level,
        effort: Effort,
        resident: &mut Resident,
        ledger: &mut WorkLedger,
    ) -> Result<Mesh, Refusal> {
        let state = cube.digest();
        if let Some(mesh) = resident.serve(state, chunk, level) {
            ledger.hits += 1;
            return Ok(mesh.clone());
        }
        if level != Level::ALL[0] {
            ledger.refusals += 1;
            return Err(cell_level_refusal(level));
        }
        let (result, done) = self.build_faces(cube, chunk, level, effort);
        ledger.add(&done);
        result.inspect(|mesh| {
            resident.insert(mesh);
        })
    }

    /// **One mesh over a rectangle of chunks** — the stitched mesh, from the cells' door: one footprint over
    /// the whole rectangle, one sweep, one merge, one mesh. The boundary between two chunks *inside* the
    /// rectangle is not an edge: the cells across it are the world's own, so no face is emitted there. Each
    /// chunk meshed alone emits the faces its own halo decides, and where the halo is owned those are the
    /// same faces — the evidence checks the region's faces against the sum of the chunks' and names the
    /// boundary's own contribution (nothing).
    ///
    /// The mesh is named by the chunk its footprint starts at, and its extent is the caller's to print: the
    /// resident set's unit is a chunk and a level, so a region mesh is a composition call rather than an
    /// entry, and nothing here is kept.
    pub fn derive_region(
        &self,
        cube: &CubicWorld,
        origin: ChunkKey,
        chunks: u32,
        level: Level,
        effort: Effort,
        ledger: &mut WorkLedger,
    ) -> Result<Mesh, Refusal> {
        let mut done = WorkLedger { misses: 1, ..WorkLedger::default() };
        let result = self.build_region(cube, origin, chunks, level, effort, &mut done);
        ledger.add(&done);
        match result {
            Ok(mesh) => Ok(mesh),
            Err(refusal) => {
                ledger.refusals += 1;
                Err(refusal)
            }
        }
    }

    fn build_region(
        &self,
        cube: &CubicWorld,
        origin: ChunkKey,
        chunks: u32,
        level: Level,
        effort: Effort,
        done: &mut WorkLedger,
    ) -> Result<Mesh, Refusal> {
        if chunks == 0 {
            return Err(Refusal::new(
                RefusalKind::ParameterOutOfRange,
                "a region of no chunks is no region: there is nothing to mesh",
            ));
        }
        let base = origin.origin().map_err(|kind| {
            Refusal::new(
                kind,
                format!("chunk ({}, {}, {}) has no expressible origin", origin.x, origin.y, origin.z),
            )
        })?;
        let side = CHUNK_SIDE as u32 * chunks;
        let declared = u64::from(side) * u64::from(side);
        if declared > effort.budget.max_columns {
            return Err(effort.budget.refuse_columns(declared));
        }
        if level != Level::ALL[0] {
            return Err(cell_level_refusal(level));
        }
        let footprint = Footprint::over(cube, Patch::new(base, side)?)?;
        let (faces, count) = sweep(cube, footprint, self.ladder.water())?;
        self.merge_into(
            GeometryKey::of_state(cube.digest(), origin, level),
            faces,
            count,
            effort,
            done,
        )
    }

    /// One chunk of the reading, from its own columns: read the three fields in **one** evaluation,
    /// decimate to the level's tile grid, fill a [`CubicWorld`] by the one constructor, sweep its faces and
    /// merge them. Returns the finished mesh (or the refusal) **with the work it did**, so a refusal still
    /// reports real numbers.
    fn build(
        &self,
        world: &World,
        chunk: ChunkKey,
        level: Level,
        effort: Effort,
    ) -> (Result<Mesh, Refusal>, WorkLedger) {
        let mut done = WorkLedger { misses: 1, ..WorkLedger::default() };
        let result = self.build_from_fields(world, chunk, level, effort, &mut done);
        match result {
            Ok(mesh) => (Ok(mesh), done),
            Err(refusal) => {
                done.refusals += 1;
                (Err(refusal), done)
            }
        }
    }

    /// One chunk of a world that already holds cells.
    fn build_faces(
        &self,
        cube: &CubicWorld,
        chunk: ChunkKey,
        level: Level,
        effort: Effort,
    ) -> (Result<Mesh, Refusal>, WorkLedger) {
        let mut done = WorkLedger { misses: 1, ..WorkLedger::default() };
        let result = self.faces_of(cube, chunk, level, effort, &mut done);
        match result {
            Ok(mesh) => (Ok(mesh), done),
            Err(refusal) => {
                done.refusals += 1;
                (Err(refusal), done)
            }
        }
    }

    /// One chunk of the reading, from the chunk's own columns **and their halo**: read the three fields in
    /// one evaluation over `32 + 2·MESH_RING` columns square, decimate to the level's tile grid, fill a
    /// [`CubicWorld`] by the one constructor, sweep the footprint's faces and merge them.
    fn build_from_fields(
        &self,
        world: &World,
        chunk: ChunkKey,
        level: Level,
        effort: Effort,
        done: &mut WorkLedger,
    ) -> Result<Mesh, Refusal> {
        let origin = chunk.origin().map_err(|kind| {
            Refusal::new(
                kind,
                format!("chunk ({}, {}, {}) has no expressible origin", chunk.x, chunk.y, chunk.z),
            )
        })?;
        let footprint_patch = Patch::new(origin, CHUNK_SIDE as u32)?;
        // **The halo, read.** The world the reading builds is the chunk's footprint *plus* its ring
        // ([`MESH_RING`]), so it owns the one cell around the chunk that every face of the footprint looks
        // at — and the chunk's own edge stops being a world edge.
        let halo = Patch::new(
            VoxelPos::new(origin.x - MESH_RING, origin.y - MESH_RING, origin.z),
            CHUNK_SIDE as u32 + 2 * MESH_RING as u32,
        )?;
        let side = halo.side();
        let area = u64::from(side) * u64::from(side);
        // The ceiling is checked against what the request *declares* it will produce, before any work: a
        // request that cannot fit is refused for free rather than after the work is done.
        if area > effort.budget.max_columns {
            return Err(effort.budget.refuse_columns(area));
        }
        let request = ProcessRequest::Region { origin: halo.origin(), side };
        // A refusal from the field plane is the answer: passed through by name, never replaced by an empty
        // mesh that would read as smooth ground.
        let values = surface_fields(
            self.roster,
            world.identity(),
            world.tick(),
            request,
            effort.workers,
        )?;
        done.columns_produced += area;

        // The decimation, declared: a coarse level reads the tile's first column and the whole tile stands
        // on what it read. The field plane still produced the whole patch (a landscape law needs its
        // neighbours), which is why the ledger reports produced and read separately.
        let tile = level.tile() as usize;
        let grid = (CHUNK_SIDE as usize) / tile;
        let mut elevation = Vec::with_capacity(area as usize);
        let mut water_level = Vec::with_capacity(area as usize);
        let mut classes = Vec::with_capacity(area as usize);
        for column in 0..(side as usize * side as usize) {
            let (rx, ry) = (column % side as usize, column / side as usize);
            let (x, y) = (halo.origin().x + rx as i32, halo.origin().y + ry as i32);
            let (sample_x, sample_y) = representative_column(x, y, origin, tile);
            let index = (sample_y - halo.origin().y) as usize * side as usize
                + (sample_x - halo.origin().x) as usize;
            elevation.push(values[0][index]);
            water_level.push(values[1][index]);
            classes.push(values[2][index]);
        }
        done.samples_read += (grid * grid) as u64;

        // The surface is read as a block lattice, and a value the lattice cannot carry is refused by name
        // **with its field**: a NaN elevation would otherwise become a plausible-looking hole in the ground.
        for (field, plane) in [(FieldId::Elevation, &elevation), (FieldId::WaterLevel, &water_level)] {
            if let Some(value) = plane.iter().find(|value| !value.is_finite()) {
                return Err(Refusal::new(
                    RefusalKind::ParameterOutOfRange,
                    format!("{} = {value} m at this column: a block height cannot be formed", field.name()),
                ));
            }
        }

        let cube = CubicWorld::from_surface(halo, &elevation, &water_level, &classes, &self.ladder)?;
        let footprint = Footprint::over(&cube, footprint_patch)?;
        let (faces, count) = sweep(&cube, footprint, self.ladder.water())?;
        self.merge_into(GeometryKey::new(world, chunk, level), faces, count, effort, done)
    }

    fn faces_of(
        &self,
        cube: &CubicWorld,
        chunk: ChunkKey,
        level: Level,
        effort: Effort,
        done: &mut WorkLedger,
    ) -> Result<Mesh, Refusal> {
        let origin = chunk.origin().map_err(|kind| {
            Refusal::new(
                kind,
                format!("chunk ({}, {}, {}) has no expressible origin", chunk.x, chunk.y, chunk.z),
            )
        })?;
        let side = CHUNK_SIDE as u32;
        let declared = u64::from(side) * u64::from(side);
        if declared > effort.budget.max_columns {
            return Err(effort.budget.refuse_columns(declared));
        }
        let footprint = Patch::new(origin, side)?;
        // **The world's own reach is the contract's own refusal.** A chunk with no column of this world under
        // it is refused **by name** (`geometry_outside_the_world`, with the world's own patch in the detail)
        // by [`Footprint::over`] — the one owner of which columns a footprint may be read from — rather than by
        // a second count here that would shadow it and leave the contract's own refusal unprovable.
        let footprint = Footprint::over(cube, footprint)?;
        let (faces, count) = sweep(cube, footprint, self.ladder.water())?;
        self.merge_into(GeometryKey::of_state(cube.digest(), chunk, level), faces, count, effort, done)
    }

    /// **The one owner of faces → rectangles → mesh.** Both doors end here: the faces are merged, the merge
    /// and the sweep are counted into `ledger` where they happened, the quad ceiling is checked against the
    /// merge that really happened, and the mesh is named by the key the caller chose.
    fn merge_into(
        &self,
        key: GeometryKey,
        faces: Vec<Face>,
        count: FaceCount,
        effort: Effort,
        done: &mut WorkLedger,
    ) -> Result<Mesh, Refusal> {
        done.columns_swept += count.columns;
        done.cells += count.solid_cells;
        done.faces += count.total_faces();
        let (quads, tally) = merge_faces(faces);
        done.rectangles_merged += tally.rectangles_merged;
        done.merged_away += tally.faces - quads.len() as u64;
        done.quads += quads.len() as u64;
        done.vertices += quads.iter().map(|quad| quad.vertices()).sum::<u64>();
        done.indices += quads.iter().map(|quad| quad.indices()).sum::<u64>();
        if done.quads > effort.budget.max_quads {
            return Err(effort.budget.refuse_quads(done.quads));
        }
        Ok(Mesh::of(key, quads))
    }
}

fn out_of_residency(residency: ResidencyHorizon, anchor: ChunkKey, chunk: ChunkKey) -> Refusal {    let distance = chunk_centre_distance_m(anchor, chunk);
    Refusal::new(
        RefusalKind::GeometryOutOfResidency,
        format!(
            "chunk ({}, {}, {}) is {} from the anchor at ({}, {}, {}), outside the declared \
             residency horizon {:.1} m",
            chunk.x,
            chunk.y,
            chunk.z,
            distance.map(|d| format!("{d:.1} m")).unwrap_or_else(|| "not expressible".to_string()),
            anchor.x,
            anchor.y,
            anchor.z,
            residency.radius_m(),
        ),
    )
}

/// The column a column's values are read from at a level. A column of the **footprint** is read at the
/// level's resolution: the representative of its own tile, rounded down. A column of the **halo** is read
/// as itself: it is one cell wide, and it is what the footprint's edge faces are decided by, so decimating
/// it would decide a footprint's boundary from a value the footprint does not contain.
fn representative_column(x: i32, y: i32, origin: VoxelPos, tile: usize) -> (i32, i32) {
    let t = tile as i32;
    let rounded = |value: i32, at: i32| -> i32 {
        let relative = value - at;
        if !(0..CHUNK_SIDE).contains(&relative) {
            value
        } else {
            at + relative.div_euclid(t) * t
        }
    };
    (rounded(x, origin.x), rounded(y, origin.y))
}

/// A level above the ladder asked of a world that already holds cells: refused **by name**, in one place,
/// because both cell doors ([`Mesher::derive_cubic`] and [`Mesher::derive_region`]) have to answer it the
/// same way.
fn cell_level_refusal(level: Level) -> Refusal {
    Refusal::new(
        RefusalKind::ParameterOutOfRange,
        format!(
            "level {} was asked of a cell world's faces: the render ladder coarsens a **column reading** \
             (one sample per 2^level columns), and a cell world is not read as columns — level 0 meshes it",
            level.get()
        ),
    )
}

/// The columns one chunk request covers: the number the derivation's own budget is checked against.
pub const fn chunk_columns() -> u64 {
    (CHUNK_SIDE as u64) * (CHUNK_SIDE as u64)
}

/// The columns a chunk's **reading** produces: the footprint's own columns plus its halo
/// ([`Footprint::halo_patch`]), because the reading has to read the ring to own it. The cells' door
/// produces nothing — it is handed cells — so this is the reading's own number.
pub const fn ring_columns() -> u64 {
    let side = CHUNK_SIDE as u64 + 2 * (MESH_RING as u64);
    side * side
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automaton::Automaton;
    use crate::stage::{Cadence, Dials, Produced, Stage, StageBody, StageContext};
    use crate::world::{DeclaredPeriod, Tick, WorldIdentity};

    // -----------------------------------------------------------------
    // A cheap declared world: three laws, a pattern the test chooses.
    // -----------------------------------------------------------------

    fn stage_of(name: &'static str, writes: &'static [FieldId], body: StageBody) -> Stage {
        Stage {
            name,
            source: "declared by the U4 fixture, on the record",
            reads: &[],
            writes,
            cadence: Cadence::Detail,
            requires_region: false,
            dials: Dials::new(&[]),
            body,
        }
    }

    /// Flat ground at 3 m, dry, one lithology class.
    fn flat_elevation(ctx: &StageContext<'_>) -> Result<Produced, Refusal> {
        Ok(Produced::one(FieldId::Elevation, vec![3.0; ctx.len()]))
    }

    fn dry_water(ctx: &StageContext<'_>) -> Result<Produced, Refusal> {
        Ok(Produced::one(FieldId::WaterLevel, vec![-10.0; ctx.len()]))
    }

    /// The west half at 3 m, the east half at 9 m.
    fn stepped_elevation(ctx: &StageContext<'_>) -> Result<Produced, Refusal> {
        Ok(Produced::one(
            FieldId::Elevation,
            ctx.columns()
                .map(|(_index, at)| {
                    if at.within_chunk().x < (CHUNK_SIDE / 2) as u8 {
                        3.0
                    } else {
                        9.0
                    }
                })
                .collect(),
        ))
    }

    /// Class 0 everywhere: one material.
    fn one_class(ctx: &StageContext<'_>) -> Result<Produced, Refusal> {
        Ok(Produced::one(FieldId::Lithology, vec![0.0; ctx.len()]))
    }

    /// The west half flooded to 6 m; the east half dry.
    fn flooded_west_water(ctx: &StageContext<'_>) -> Result<Produced, Refusal> {
        Ok(Produced::one(
            FieldId::WaterLevel,
            ctx.columns()
                .map(|(_index, at)| {
                    if at.within_chunk().x < (CHUNK_SIDE / 2) as u8 {
                        6.0
                    } else {
                        -10.0
                    }
                })
                .collect(),
        ))
    }

    /// The near half in class 0, the far half in class 1 — same height, different material.
    fn two_class_lithology(ctx: &StageContext<'_>) -> Result<Produced, Refusal> {
        Ok(Produced::one(
            FieldId::Lithology,
            ctx.columns()
                .map(|(_index, at)| {
                    if at.within_chunk().y < (CHUNK_SIDE / 2) as u8 {
                        0.0
                    } else {
                        1.0
                    }
                })
                .collect(),
        ))
    }

    fn roster_with(elevation: StageBody, water: StageBody, lithology: StageBody) -> Declared {
        Declared::new(vec![
            stage_of("terrain", &[FieldId::Elevation], elevation),
            stage_of("hydrology", &[FieldId::WaterLevel], water),
            stage_of("fields", &[FieldId::Lithology], lithology),
        ])
        .expect("the fixture roster is a valid graph")
    }

    fn flat_roster() -> Declared {
        roster_with(flat_elevation, dry_water, one_class)
    }

    fn registry() -> MaterialRegistry {
        MaterialRegistry::starter()
    }

    /// A ladder that can name two classes, both admitted, plus water.
    fn ladder() -> Ladder {
        Ladder::declare(
            &registry(),
            &[MaterialId(1), MaterialId(2)],
            MaterialId(5),
        )
        .expect("the starter registry admits both")
    }

    fn horizon(metres: f64) -> ResidencyHorizon {
        ResidencyHorizon::m(metres).expect("a positive radius")
    }

    fn world() -> World {
        let period = DeclaredPeriod::seconds(1).expect("a one-second period is not zero");
        World::new(WorldIdentity::new(0x5EED_0004, period))
    }

    fn chunk() -> ChunkKey {
        ChunkKey::new(0, 0, 0)
    }

    fn level(level: u8) -> Level {
        Level::new(level).expect("a level on the ladder")
    }

    /// The **cells** the reading of one chunk stands on: the world the declared fields make over that
    /// chunk's own patch, through the world's one door ([`CubicWorld::from_fields`]) — the world the
    /// independent recount is taken of.
    fn chunk_cube(roster: &Declared, world: &World) -> CubicWorld {
        CubicWorld::from_fields(
            roster,
            world.identity(),
            world.tick(),
            ProcessRequest::Region {
                origin: chunk().origin().expect("the fixture chunk has an origin"),
                side: CHUNK_SIDE as u32,
            },
            &ladder(),
            1,
        )
        .expect("the fixture surface fills a cubic world")
    }

    /// **The world the reading builds for itself**: the chunk's footprint plus its halo ([`MESH_RING`]),
    /// from the same fields — so the ledger is recounted against the very cells the door swept.
    fn read_cube(roster: &Declared, world: &World) -> CubicWorld {
        let origin = chunk().origin().expect("the fixture chunk has an origin");
        let side = CHUNK_SIDE as u32 + 2 * MESH_RING as u32;
        let origin = VoxelPos::new(origin.x - MESH_RING, origin.y - MESH_RING, origin.z);
        CubicWorld::from_fields(
            roster,
            world.identity(),
            world.tick(),
            ProcessRequest::Region { origin, side },
            &ladder(),
            1,
        )
        .expect("the fixture surface fills a cubic world")
    }

    fn derive_one(roster: &Declared, world: &World, rung: u8) -> (Mesh, WorkLedger) {
        let mesher = Mesher::new(roster, ladder(), horizon(512.0));
        let mut resident = Resident::new();
        let mut ledger = WorkLedger::default();
        let mesh = mesher
            .derive(
                world,
                chunk(),
                level(rung),
                Effort::new(Budget::unlimited(), 1),
                &mut resident,
                &mut ledger,
            )
            .expect("the fixture world meshes");
        (mesh, ledger)
    }

    // -----------------------------------------------------------------
    // The ladder, the horizons, the identity.
    // -----------------------------------------------------------------

    #[test]
    fn a_level_past_the_ladder_is_refused_by_name() {
        for level in Level::ALL {
            assert!(level.tile() == 1 << level.get());
        }
        assert_eq!(Level::ALL.len(), (MAX_LEVEL as usize) + 1, "the ladder is closed");
        let refused = Level::new(MAX_LEVEL + 1).expect_err("a level past the ladder is refused");
        assert_eq!(refused.kind(), RefusalKind::LevelOffLadder);
        assert_eq!(refused.kind().name(), "level_off_ladder");
        assert!(refused.detail().contains('3'), "the requested level must be named: {refused}");
        assert!(refused.detail().contains('2'), "the ladder must be named: {refused}");
    }

    #[test]
    fn a_render_horizon_wider_than_residency_is_refused() {
        let render = RenderHorizon::m(64.0).expect("positive");
        let residency = horizon(32.0);
        let refused = check_horizons(render, residency).expect_err("64 m cannot be rendered from 32");
        assert_eq!(refused.kind(), RefusalKind::HorizonMixed);
        assert!(refused.detail().contains("64.0") && refused.detail().contains("32.0"), "{refused}");
        // Narrower, and exactly equal, are both fine: nothing is rendered that is not kept.
        assert!(check_horizons(RenderHorizon::m(16.0).unwrap(), residency).is_ok());
        assert!(check_horizons(RenderHorizon::m(32.0).unwrap(), residency).is_ok());

        // Each horizon refuses a radius that is not a positive finite distance.
        for refused in [
            RenderHorizon::m(0.0).unwrap_err(),
            SimulationHorizon::m(-1.0).unwrap_err(),
            ResidencyHorizon::m(f64::NAN).unwrap_err(),
            FieldHorizon::m(f64::INFINITY).unwrap_err(),
        ] {
            assert_eq!(refused.kind(), RefusalKind::ParameterOutOfRange);
        }
        // The four horizons are four types: this line would not compile with one shared radius.
        assert_eq!(FieldHorizon::m(1.0).unwrap().radius_m(), 1.0);
        assert_eq!(SimulationHorizon::m(2.0).unwrap().radius_m(), 2.0);

        // A chunk is inside or outside as a whole, measured between centres.
        assert!(residency.keeps(chunk(), chunk()));
        assert!(horizon(32.0).keeps(ChunkKey::new(0, 0, 0), ChunkKey::new(1, 0, 0)));
        assert!(!horizon(32.0).keeps(ChunkKey::new(0, 0, 0), ChunkKey::new(2, 0, 0)));
    }

    #[test]
    fn a_ladder_names_every_class_or_refuses_it() {
        let registry = registry();
        let declared = Ladder::declare(&registry, &[MaterialId(1), MaterialId(6)], MaterialId(5))
            .expect("both ground ids are admitted");
        assert_eq!(declared.classes(), 2);
        assert_eq!(declared.for_class(0).unwrap(), MaterialId(1));
        assert_eq!(declared.for_class(1).unwrap(), MaterialId(6));
        assert_eq!(declared.water(), MaterialId(5));
        assert_eq!(declared.mapping(), vec![(0, MaterialId(1)), (1, MaterialId(6))]);

        // A class past the ladder, and a negative class, are both refused by name.
        for class in [2, -1] {
            let refused = declared.for_class(class).expect_err("outside the ladder");
            assert_eq!(refused.kind(), RefusalKind::UnadmittedMaterial);
            assert!(refused.detail().contains(&class.to_string()), "{refused}");
        }
        // A ladder cannot promise a material the registry has not admitted, and it cannot be empty.
        let unadmitted = Ladder::declare(&registry, &[MaterialId(9999)], MaterialId(5))
            .expect_err("unadmitted ground");
        assert_eq!(unadmitted.kind(), RefusalKind::UnadmittedMaterial);
        assert!(unadmitted.detail().contains("9999"), "{unadmitted}");
        assert_eq!(
            Ladder::declare(&registry, &[], MaterialId(5)).unwrap_err().kind(),
            RefusalKind::ParameterOutOfRange
        );

        // A palette shorter than the class count is laid over the classes in declared order, and the
        // mapping is visible: monotone, and two neighbouring classes share a material.
        let wide = Ladder::over_palette(&registry, &[MaterialId(1), MaterialId(2)], 6, MaterialId(5))
            .expect("a two-entry palette over six classes");
        let mapping = wide.mapping();
        assert_eq!(mapping.len(), 6);
        assert!(
            mapping.windows(2).all(|pair| pair[0].1 <= pair[1].1),
            "the mapping must be monotone: {mapping:?}"
        );
        assert_eq!(wide.for_class(0).unwrap(), wide.for_class(1).unwrap());
        assert_ne!(wide.for_class(0).unwrap(), wide.for_class(5).unwrap());
    }

    #[test]
    fn one_key_one_digest_and_a_changed_face_changes_it() {
        let roster = flat_roster();
        let mesher = Mesher::new(&roster, ladder(), horizon(512.0));
        let world = world();
        let mut resident = Resident::new();
        let mut ledger = WorkLedger::default();
        let flat = mesher
            .derive(&world, chunk(), level(0), Effort::new(Budget::unlimited(), 1), &mut resident, &mut ledger)
            .expect("the flat fixture meshes");

        // The same world, the same chunk, the same level: the same digest — derived again from scratch in
        // a fresh cache.
        let mut again = WorkLedger::default();
        let mut fresh = Resident::new();
        let repeated = mesher
            .derive(&world, chunk(), level(0), Effort::new(Budget::unlimited(), 4), &mut fresh, &mut again)
            .expect("the same world meshes");
        assert_eq!(flat.digest(), repeated.digest(), "one key, one digest");
        assert_eq!(flat.quads(), repeated.quads());

        // A different world (another tick), and a coarse level, are different geometry.
        let later = mesher
            .derive(
                &world.with_tick(Tick(1)),
                chunk(),
                level(0),
                Effort::new(Budget::unlimited(), 1),
                &mut fresh,
                &mut again,
            )
            .expect("a later tick meshes");
        assert_ne!(later.digest(), flat.digest(), "the digest must name its world");
        let mut coarse_resident = Resident::new();
        let coarse = mesher
            .derive(
                &world,
                chunk(),
                level(2),
                Effort::new(Budget::unlimited(), 1),
                &mut coarse_resident,
                &mut again,
            )
            .expect("a coarse level meshes");
        assert_ne!(coarse.digest(), flat.digest());

        // And the key names the world: the same chunk at another tick is another key.
        assert_ne!(
            GeometryKey::new(&world, chunk(), level(0)),
            GeometryKey::new(&world.with_tick(Tick(1)), chunk(), level(0))
        );
    }

    // -----------------------------------------------------------------
    // US1 — the geometry the cells imply.
    // -----------------------------------------------------------------

    #[test]
    fn a_flat_worlds_chunk_is_one_plane_and_its_edges_are_the_rings_business() {
        let roster = flat_roster();
        let (mesh, ledger) = derive_one(&roster, &world(), 0);

        // **The extent contract at the module's own scale.** The reading's world is the chunk *and its
        // halo* — the same flat ground — so the chunk's four sides are the ring's business (the same cells
        // stand there, so no wall is drawn) and its floor is interior by declaration (the band below holds
        // this ground, so no underside is drawn). A flat chunk of a flat world is therefore exactly its
        // top: one rectangle, because a uniform plane is one quad (C-43).
        assert_eq!(mesh.quads().len(), 1, "{:?}", mesh.quads());
        assert_eq!(mesh.quads()[0].layer, Layer::Solid);
        assert_eq!(mesh.quads()[0].material, MaterialId(1));
        let read = read_cube(&roster, &world());
        let footprint = Footprint::of_chunk(&read, chunk()).expect("the chunk's own footprint");
        assert!(footprint.halo_owned(&read), "the world the reading reads owns the halo");
        let recount = recount_faces(&read, footprint);
        assert_eq!(ledger.cells, recount.solid_cells, "every solid cell was swept");
        assert_eq!(ledger.faces, recount.total_faces());
        assert_eq!(mesh.area(), ledger.faces, "every face is tiled exactly once");
        assert_eq!(ledger.merged_away, ledger.faces - ledger.quads);
        assert_eq!(ledger.columns_swept, chunk_columns());
        assert_eq!(ledger.columns_produced, ring_columns(), "the reading reads the chunk and its halo");
        assert_eq!(ledger.samples_read, chunk_columns(), "level 0 reads every column");

        // The same chunk in a world that is **only** the chunk: it owns no halo, so its own four walls are
        // faces — the declaration is what decides them, and it is a declaration with a number.
        let only = chunk_cube(&roster, &world());
        let only_footprint = Footprint::of_chunk(&only, chunk()).expect("the chunk's own footprint");
        assert!(!only_footprint.halo_owned(&only), "a world that is only the chunk owns no halo");
        let only_recount = recount_faces(&only, only_footprint);
        assert!(
            only_recount.total_faces() > recount.total_faces(),
            "the chunk's own edge is faces: {} against {}",
            only_recount.total_faces(),
            recount.total_faces()
        );

        // The top of the ground: 3.0 m is the voxel that holds it, so the face stands on the plane above
        // voxel 3 — whatever the world's own floor is.
        let top = mesh.quads()[0];
        assert_eq!(top.axis, Axis::Z);
        assert_eq!(top.facing, 1);
        assert_eq!(top.plane, 4, "3.0 m is the voxel 3, and its face is the plane above it");
        assert_eq!((top.u, top.v, top.span_u, top.span_v), (0, 0, 32, 32));
        assert_eq!(ledger.merge_factor(), ledger.faces as f64, "one rectangle carries every face");
    }

    #[test]
    fn a_step_is_a_wall_and_not_a_hole() {
        let roster = roster_with(stepped_elevation, dry_water, one_class);
        let (mesh, ledger) = derive_one(&roster, &world(), 0);

        // The step at x = 16 is a wall between the low half's top (voxel 3, plane 4) and the high half's
        // (voxel 9, plane 10): the faces of the high columns' west sides, z 4..10.
        let wall = mesh
            .quads()
            .iter()
            .find(|quad| quad.axis == Axis::X && quad.facing < 0 && quad.plane == 16 && quad.v == 4)
            .unwrap_or_else(|| panic!("the step must be a wall: {:?}", mesh.quads()));
        assert_eq!((wall.u, wall.span_u, wall.span_v), (0, 32, 6));
        assert_eq!(wall.material, MaterialId(1), "the wall is the high half's material");

        let read = read_cube(&roster, &world());
        let footprint = Footprint::of_chunk(&read, chunk()).expect("the chunk's own footprint");
        let recount = recount_faces(&read, footprint);
        let mut probe_ledger = WorkLedger::default();
        let probe = Mesher::new(&roster, ladder(), horizon(512.0)).derive_cubic(
            &read,
            chunk(),
            level(0),
            Effort::new(Budget::unlimited(), 1),
            &mut Resident::new(),
            &mut probe_ledger,
        );
        // And the two doors over the world the reading reads for itself: the same faces, the same
        // rectangles — only the name differs (the reading keys by the world's reading, the cells' door by
        // the cells' own state).
        let over_the_reading = probe.expect("the world the reading reads meshes through the cells' door");
        assert_eq!(over_the_reading.quads(), mesh.quads(), "one world, two doors: the same list");
        assert_eq!(over_the_reading.area(), mesh.area());
        assert_ne!(over_the_reading.key().state, mesh.key().state, "and two names");
        let _ = (&mut probe_ledger, footprint);
        assert_eq!(ledger.faces, recount.total_faces());
        assert_eq!(mesh.area(), ledger.faces);
        assert_eq!(ledger.merged_away, ledger.faces - ledger.quads);
    }

    #[test]
    fn two_cells_merge_only_when_layer_plane_and_material_agree() {
        // Same height everywhere, two materials: the merge must stop at the material boundary.
        let roster = roster_with(flat_elevation, dry_water, two_class_lithology);
        let (mesh, _) = derive_one(&roster, &world(), 0);
        let tops: Vec<&Quad> = mesh
            .quads()
            .iter()
            .filter(|quad| quad.axis == Axis::Z && quad.facing > 0 && quad.layer == Layer::Solid)
            .collect();
        assert_eq!(tops.len(), 2, "two materials cannot share one rectangle: {tops:?}");
        assert_eq!(tops[0].material, MaterialId(1));
        assert_eq!(tops[1].material, MaterialId(2));
        assert_eq!(tops[0].span_u, 32);
        assert_eq!(tops[1].span_u, 32);
        for top in tops {
            assert_eq!(top.span_v, 16, "only the half that shares the material");
        }

        // A flooded half shows its water surface too, and the water never merges with the ground.
        let roster = roster_with(flat_elevation, flooded_west_water, one_class);
        let (mesh, _) = derive_one(&roster, &world(), 0);
        let water: Vec<&Quad> = mesh.quads().iter().filter(|q| q.layer == Layer::Water).collect();
        assert_eq!(water.len(), 1, "the water's own top is one rectangle: {water:?}");
        assert_eq!(water[0].axis, Axis::Z);
        assert_eq!(water[0].facing, 1);
        assert_eq!(water[0].plane, 7, "6.0 m over 3.0 m is three voxels of water, shown above voxel 6");
        assert_eq!(water[0].material, MaterialId(5));
        assert_eq!(water[0].span_u, 16, "only the flooded half");
        assert_eq!(water[0].span_v, 32);
    }

    #[test]
    fn a_mesh_never_invents_a_material() {
        let roster = roster_with(flat_elevation, dry_water, two_class_lithology);
        // A ladder that can name **one** class, while the world declares two.
        let narrow = Ladder::declare(&registry(), &[MaterialId(1)], MaterialId(5)).expect("admitted");
        let mesher = Mesher::new(&roster, narrow, horizon(512.0));
        let world = world();
        let mut resident = Resident::new();
        let mut ledger = WorkLedger::default();
        let refused = mesher
            .derive(&world, chunk(), level(0), Effort::new(Budget::unlimited(), 1), &mut resident, &mut ledger)
            .expect_err("class 1 is not in the ladder");
        assert_eq!(refused.kind(), RefusalKind::UnadmittedMaterial);
        assert!(refused.detail().contains('1'), "{refused}");
        assert!(resident.is_empty(), "nothing is cached for a refused chunk");
        assert_eq!(ledger.refusals, 1);
        assert_eq!(ledger.cells, 0, "the refusal happened before any cell was swept");
    }

    /// A body that refuses for its own reason: the mesh must pass it through, not answer with nothing.
    fn refuses_elevation(_ctx: &StageContext<'_>) -> Result<Produced, Refusal> {
        Err(Refusal::new(
            RefusalKind::ParameterOutOfRange,
            "the fixture's own reason for refusing",
        ))
    }

    fn nan_elevation(ctx: &StageContext<'_>) -> Result<Produced, Refusal> {
        Ok(Produced::one(FieldId::Elevation, vec![f64::NAN; ctx.len()]))
    }

    #[test]
    fn a_stages_refusal_passes_through_the_mesh_seam() {
        let roster = roster_with(refuses_elevation, dry_water, one_class);
        let mesher = Mesher::new(&roster, ladder(), horizon(512.0));
        let world = world();
        let mut resident = Resident::new();
        let mut ledger = WorkLedger::default();
        let refused = mesher
            .derive(&world, chunk(), level(0), Effort::new(Budget::unlimited(), 1), &mut resident, &mut ledger)
            .expect_err("the declaring stage refuses");
        assert_eq!(refused.kind(), RefusalKind::ParameterOutOfRange);
        assert!(
            refused.detail().contains("the fixture's own reason"),
            "the stage's own reason must survive: {refused}"
        );
        assert!(resident.is_empty());

        // A field value that is not a number is refused too, rather than becoming a hole in ground.
        let roster = roster_with(nan_elevation, dry_water, one_class);
        let mesher = Mesher::new(&roster, ladder(), horizon(512.0));
        let refused = mesher
            .derive(&world, chunk(), level(0), Effort::new(Budget::unlimited(), 1), &mut resident, &mut ledger)
            .expect_err("a NaN elevation cannot become a block height");
        assert_eq!(refused.kind(), RefusalKind::ParameterOutOfRange);
        assert!(refused.detail().contains("elevation"), "{refused}");
        assert!(refused.detail().contains("NaN"), "{refused}");
        assert_eq!(ledger.cells, 0, "nothing was swept for the refused chunk");
    }

    #[test]
    fn six_indices_per_quad_exactly() {
        let roster = roster_with(stepped_elevation, flooded_west_water, one_class);
        let (mesh, ledger) = derive_one(&roster, &world(), 0);

        let quads = mesh.quads().len() as u64;
        assert_eq!(mesh.vertices(), 4 * quads);
        assert_eq!(mesh.triangles(), 2 * quads);
        assert_eq!(mesh.indices(), 6 * quads);
        assert_eq!(ledger.quads, quads);
        assert_eq!(ledger.vertices, 4 * quads);
        assert_eq!(ledger.indices, 6 * quads);
        assert!(mesh.quads().iter().any(|quad| quad.layer == Layer::Water), "the flooded half stands water");
        assert!(mesh.quads().iter().any(|quad| quad.layer == Layer::Solid));
    }

    // -----------------------------------------------------------------
    // US2 — resident geometry stays coherent with its world.
    // -----------------------------------------------------------------

    #[test]
    fn a_stale_entry_is_rebuilt_and_never_served() {
        let roster = roster_with(stepped_elevation, flooded_west_water, two_class_lithology);
        let mesher = Mesher::new(&roster, ladder(), horizon(512.0));
        let world = world();
        let mut resident = Resident::new();
        let mut ledger = WorkLedger::default();
        let first = mesher
            .derive(&world, chunk(), level(0), Effort::new(Budget::unlimited(), 1), &mut resident, &mut ledger)
            .expect("meshes");
        assert_eq!(ledger.misses, 1);

        // A request in an unchanged world is a hit, and a hit does no work at all.
        let before = ledger;
        let again = mesher
            .derive(&world, chunk(), level(0), Effort::new(Budget::unlimited(), 1), &mut resident, &mut ledger)
            .expect("served");
        assert_eq!(again.digest(), first.digest());
        assert_eq!(ledger.hits, before.hits + 1);
        assert_eq!(ledger.columns_produced, before.columns_produced, "a hit reads no columns");
        assert_eq!(ledger.samples_read, before.samples_read);
        assert_eq!(ledger.faces, before.faces, "a hit sweeps no cells");
        assert_eq!(resident.len(), 1);

        // The world moves: the kept geometry is from another truth, so it is not served — it is rebuilt.
        // The stale entry is now unreachable and counted as stale.
        let later = world.with_tick(Tick(1));
        assert!(
            resident.serve(later.truth_digest(), chunk(), level(0)).is_none(),
            "stale is never served"
        );
        assert_eq!(resident.stale(later.truth_digest()), 1);
        let rebuilt = mesher
            .derive(&later, chunk(), level(0), Effort::new(Budget::unlimited(), 1), &mut resident, &mut ledger)
            .expect("rebuilt");
        assert_eq!(ledger.misses, 2);
        assert_ne!(rebuilt.digest(), first.digest(), "the rebuilt mesh is of the new world");
        assert!(resident.serve(later.truth_digest(), chunk(), level(0)).is_some());
        assert_eq!(resident.len(), 2, "the stale entry is still kept until it is invalidated");
    }

    #[test]
    fn invalidation_removes_exactly_the_stale_entries() {
        let roster = roster_with(stepped_elevation, flooded_west_water, two_class_lithology);
        let mesher = Mesher::new(&roster, ladder(), horizon(512.0));
        let world = world();
        let mut resident = Resident::new();
        let mut ledger = WorkLedger::default();
        let old: Vec<ChunkKey> = (0..3).map(|x| ChunkKey::new(x, 0, 0)).collect();
        let before: Vec<Digest> = mesher
            .derive_many(&world, &old, level(0), Effort::new(Budget::unlimited(), 2), &mut resident, &mut ledger)
            .expect("three chunks")
            .iter()
            .map(Mesh::digest)
            .collect();
        assert_eq!(resident.len(), 3);
        assert_eq!(
            resident.invalidate(world.truth_digest()),
            0,
            "nothing is stale in its own world"
        );

        // The world advances and one chunk of the new world is derived: the old three are stale, the new
        // one is current.
        let later = world.with_tick(Tick(1));
        let current = mesher
            .derive(&later, old[1], level(0), Effort::new(Budget::unlimited(), 1), &mut resident, &mut ledger)
            .expect("one chunk of the new world");
        assert_eq!(resident.len(), 4);
        assert_eq!(resident.stale(later.truth_digest()), 3);
        assert_eq!(resident.invalidate(later.truth_digest()), 3, "exactly the old world's entries");
        assert_eq!(resident.len(), 1);
        assert_eq!(resident.stale(later.truth_digest()), 0);
        assert!(
            resident.serve(later.truth_digest(), old[1], level(0)).is_some(),
            "the current entry stays"
        );
        assert_eq!(
            resident.serve(later.truth_digest(), old[1], level(0)).unwrap().digest(),
            current.digest()
        );
        assert!(
            resident.serve(world.truth_digest(), old[0], level(0)).is_none(),
            "the old entries are gone"
        );
        // The geometry of the new world is still what it was, and the old digests are still distinct.
        assert!(!before.contains(&current.digest()));
    }

    #[test]
    fn the_anchor_is_not_a_geometry_input() {
        let roster = roster_with(stepped_elevation, flooded_west_water, two_class_lithology);
        let mesher = Mesher::new(&roster, ladder(), horizon(512.0));
        let world = world();
        let mut resident = Resident::new();
        let mut ledger = WorkLedger::default();
        let mesh = mesher
            .derive(&world, chunk(), level(0), Effort::new(Budget::unlimited(), 1), &mut resident, &mut ledger)
            .expect("meshes");
        let key = mesh.key();

        // A view is not truth: moving the anchor and filling U2's residency cache changes neither the key
        // nor the served geometry.
        let mut viewed = world.clone();
        viewed.set_anchor(ChunkKey::new(-9, 4, 2));
        assert!(viewed.reside(ChunkKey::new(3, 0, 0)));
        assert_eq!(GeometryKey::new(&viewed, chunk(), level(0)), key);
        assert_eq!(viewed.truth_digest(), world.truth_digest());
        let served = resident
            .serve(viewed.truth_digest(), chunk(), level(0))
            .expect("still a current entry");
        assert_eq!(served.digest(), mesh.digest());

        let before = ledger;
        let again = mesher
            .derive(&viewed, chunk(), level(0), Effort::new(Budget::unlimited(), 1), &mut resident, &mut ledger)
            .expect("a hit in the moved view");
        assert_eq!(again.digest(), mesh.digest());
        assert_eq!(ledger.hits, before.hits + 1);
        assert_eq!(ledger.columns_produced, before.columns_produced);
        assert_eq!(
            resident.invalidate(viewed.truth_digest()),
            0,
            "a moved anchor invalidates nothing"
        );
    }

    #[test]
    fn a_chunk_outside_the_residency_horizon_is_refused() {
        let roster = flat_roster();
        let mesher = Mesher::new(&roster, ladder(), horizon(48.0));
        let world = world();
        let mut resident = Resident::new();
        let mut ledger = WorkLedger::default();

        // One chunk away is 32 m centre-to-centre: kept. Two chunks away is 64 m: refused by name.
        let near = mesher
            .derive(
                &world,
                ChunkKey::new(1, 0, 0),
                level(0),
                Effort::new(Budget::unlimited(), 1),
                &mut resident,
                &mut ledger,
            )
            .expect("inside the horizon");
        // The geometry is in world columns: the near chunk's own faces start at column 32 on the two
        // horizontal axes, and hang on the world's own vertical span (which is not the chunk's key).
        for quad in near.quads() {
            match quad.axis {
                Axis::Y | Axis::Z => assert!((32..64).contains(&quad.u), "{quad:?}"),
                Axis::X => assert!((0..32).contains(&quad.u), "{quad:?}"),
            }
            if quad.axis == Axis::Z {
                assert!((0..32).contains(&quad.v), "{quad:?}");
            }
        }

        let refused = mesher
            .derive(
                &world,
                ChunkKey::new(2, 0, 0),
                level(0),
                Effort::new(Budget::unlimited(), 1),
                &mut resident,
                &mut ledger,
            )
            .expect_err("outside the horizon");
        assert_eq!(refused.kind(), RefusalKind::GeometryOutOfResidency);
        assert_eq!(refused.kind().name(), "geometry_out_of_residency");
        assert!(refused.detail().contains("64.0"), "{refused}");
        assert!(refused.detail().contains("48.0"), "{refused}");
        assert_eq!(resident.len(), 1, "a refused chunk caches nothing");

        // The horizon is checked per chunk of a batch too, before any of it is derived.
        let refused = mesher
            .derive_many(
                &world,
                &[ChunkKey::new(0, 0, 0), ChunkKey::new(4, 0, 0)],
                level(0),
                Effort::new(Budget::unlimited(), 2),
                &mut resident,
                &mut ledger,
            )
            .expect_err("one chunk of the batch is out");
        assert_eq!(refused.kind(), RefusalKind::GeometryOutOfResidency);
        assert_eq!(ledger.refusals, 2);
    }

    // -----------------------------------------------------------------
    // US3 — the work is counted, budgeted and distributed.
    // -----------------------------------------------------------------

    #[test]
    fn the_ledger_equals_an_independent_recount() {
        let roster = roster_with(stepped_elevation, flooded_west_water, two_class_lithology);
        let (mesh, ledger) = derive_one(&roster, &world(), 0);

        // The recount is of the cells the door swept, not of the ledger's formula: the same rules, a second
        // implementation, over the world the reading reads for itself (the chunk and its halo).
        let read = read_cube(&roster, &world());
        let footprint = Footprint::of_chunk(&read, chunk()).expect("the chunk's own footprint");
        let recount = recount_faces(&read, footprint);
        assert_eq!(ledger.columns_swept, recount.columns);
        assert_eq!(ledger.cells, recount.solid_cells);
        assert_eq!(ledger.faces, recount.total_faces());
        assert_eq!(ledger.merged_away, ledger.faces - ledger.quads);
        assert_eq!(ledger.quads, mesh.quads().len() as u64);
        assert_eq!(mesh.area(), ledger.faces, "the rectangles tile every face exactly once");
        assert_eq!(ledger.columns_produced, ring_columns());
        assert_eq!(ledger.samples_read, chunk_columns(), "at level 0 every column is read");
        assert!(recount.water_faces > 0, "the fixture stands water");
        assert!(ledger.faces > ledger.quads, "the merge did something");
        assert_eq!(ledger.misses, 1);
        assert_eq!(ledger.hits, 0);
        assert_eq!(ledger.refusals, 0);
        assert!(ledger.merge_factor() > 1.0, "the merge did something: {}", ledger.merge_factor());
        assert!(ledger.cells > ledger.columns_swept, "a stack deeper than one voxel");
    }

    #[test]
    fn a_coarse_level_reads_fewer_samples_than_were_produced() {
        let roster = flat_roster();
        let mesher = Mesher::new(&roster, ladder(), horizon(512.0));
        let world = world();

        let mut fine_ledger = WorkLedger::default();
        let mut resident = Resident::new();
        mesher
            .derive(&world, chunk(), level(0), Effort::new(Budget::unlimited(), 1), &mut resident, &mut fine_ledger)
            .expect("level 0 meshes");

        let mut coarse_ledger = WorkLedger::default();
        let mut coarse_resident = Resident::new();
        let coarse = mesher
            .derive(
                &world,
                chunk(),
                level(2),
                Effort::new(Budget::unlimited(), 1),
                &mut coarse_resident,
                &mut coarse_ledger,
            )
            .expect("level 2 meshes");

        // Both levels produce the whole patch, and both report honestly what they read.
        assert_eq!(coarse_ledger.columns_produced, fine_ledger.columns_produced);
        assert_eq!(fine_ledger.columns_produced, ring_columns());
        assert_eq!(fine_ledger.samples_read, 1024);
        assert_eq!(coarse_ledger.samples_read, 64, "8x8 tiles");
        assert!(coarse_ledger.samples_read < coarse_ledger.columns_produced);
        assert_eq!(coarse.area(), coarse_ledger.faces, "a coarse level still tiles its faces exactly");
        assert!(coarse_ledger.merge_factor() > 1.0, "and still merges them");
    }

    #[test]
    fn a_budget_that_cannot_be_met_is_refused_with_its_numbers() {
        let roster = flat_roster();
        let mesher = Mesher::new(&roster, ladder(), horizon(512.0));
        let world = world();
        let mut resident = Resident::new();
        let mut ledger = WorkLedger::default();

        // The column ceiling is checked before any work: the reading declares 1156 columns — the chunk's
        // own 1024 plus its halo, which is what it reads.
        let columns = mesher
            .derive(&world, chunk(), level(0), Effort::new(Budget::new(100, 10), 1), &mut resident, &mut ledger)
            .expect_err("1156 columns cannot fit in 100");
        assert_eq!(columns.kind(), RefusalKind::GeometryBudgetExceeded);
        assert!(columns.detail().contains("1156"), "{columns}");
        assert!(columns.detail().contains("100"), "{columns}");
        assert_eq!(ledger.columns_produced, 0, "refused before the fields were read");
        assert!(resident.is_empty());

        // The quad ceiling is checked after the merge — which really happened, and is reported.
        let mut measured = WorkLedger::default();
        let mut measured_resident = Resident::new();
        let free = mesher
            .derive(
                &world,
                chunk(),
                level(0),
                Effort::new(Budget::unlimited(), 1),
                &mut measured_resident,
                &mut measured,
            )
            .expect("the same request without a ceiling");
        let before = ledger;
        let quads = mesher
            .derive(&world, chunk(), level(0), Effort::new(Budget::new(u64::MAX, 0), 1), &mut resident, &mut ledger)
            .expect_err("no rectangle fits in zero");
        assert_eq!(quads.kind(), RefusalKind::GeometryBudgetExceeded);
        assert!(quads.detail().contains(&format!("{} rectangles", free.quads().len())), "{quads}");
        assert!(quads.detail().contains("allows 0"), "{quads}");
        assert_eq!(ledger.columns_produced, before.columns_produced + ring_columns());
        assert_eq!(ledger.quads, before.quads + free.quads().len() as u64, "the work is counted");
        assert_eq!(ledger.refusals, before.refusals + 1);
        assert!(resident.is_empty(), "nothing is cached on a refusal");
    }

    fn stepped_roster() -> Declared {
        roster_with(stepped_elevation, flooded_west_water, two_class_lithology)
    }

    #[test]
    fn the_worker_count_changes_no_quad_and_no_digest() {
        let roster = stepped_roster();
        let mesher = Mesher::new(&roster, ladder(), horizon(512.0));
        let world = world();

        let mut digests = Vec::new();
        for workers in [1_usize, 2, 4, 8] {
            let mut resident = Resident::new();
            let mut ledger = WorkLedger::default();
            let mesh = mesher
                .derive(&world, chunk(), level(0), Effort::new(Budget::unlimited(), workers), &mut resident, &mut ledger)
                .expect("meshes");
            digests.push(mesh.digest());
            assert_eq!(mesh.area(), ledger.faces, "the tiles cover the faces exactly");
        }
        assert!(
            digests.windows(2).all(|pair| pair[0] == pair[1]),
            "the worker count is a knob: {digests:?}"
        );

        // A batch is returned in the declared order, and one digestion for one worker count.
        let chunks = [
            ChunkKey::new(1, 0, 0),
            ChunkKey::new(0, 0, 1),
            ChunkKey::new(-1, 2, 0),
        ];
        let mut batched: Vec<Vec<Digest>> = Vec::new();
        for workers in [1_usize, 4] {
            let mut resident = Resident::new();
            let mut ledger = WorkLedger::default();
            let meshes = mesher
                .derive_many(&world, &chunks, level(0), Effort::new(Budget::unlimited(), workers), &mut resident, &mut ledger)
                .expect("three chunks");
            assert_eq!(meshes.len(), chunks.len());
            for (mesh, chunk) in meshes.iter().zip(&chunks) {
                assert_eq!(mesh.key().chunk, *chunk, "the order of the request is the order of the answer");
            }
            assert_eq!(ledger.misses, 3, "a cold cache derives all three");
            batched.push(meshes.iter().map(Mesh::digest).collect());
        }
        assert_eq!(batched[0], batched[1], "the worker count changes no digest");

        // A warm cache in the same order: three hits, no work, and one resident digest.
        let mut resident = Resident::new();
        let mut ledger = WorkLedger::default();
        let first = mesher
            .derive_many(&world, &chunks, level(0), Effort::new(Budget::unlimited(), 4), &mut resident, &mut ledger)
            .expect("three chunks");
        let cold_digest = resident.digest();
        let warm = mesher
            .derive_many(&world, &chunks, level(0), Effort::new(Budget::unlimited(), 1), &mut resident, &mut ledger)
            .expect("served");
        assert_eq!(ledger.hits, 3);
        assert_eq!(ledger.misses, 3);
        assert_eq!(ledger.columns_produced, 3 * ring_columns());
        assert_eq!(warm.iter().map(Mesh::digest).collect::<Vec<_>>(), first.iter().map(Mesh::digest).collect::<Vec<_>>());
        assert_eq!(resident.len(), 3);
        assert_eq!(resident.digest(), cold_digest, "the resident set is a set");

        // And the empty request is an empty answer, not a refusal.
        assert!(mesher
            .derive_many(&world, &[], level(0), Effort::new(Budget::unlimited(), 2), &mut resident, &mut ledger)
            .expect("nothing asked")
            .is_empty());
    }

    // -----------------------------------------------------------------
    // The cells' own door: the law's world, keyed by its own state.
    // -----------------------------------------------------------------

    fn fixture_cube() -> CubicWorld {
        chunk_cube(&stepped_roster(), &world())
    }

    #[test]
    fn a_cell_world_meshes_its_own_cells_and_keys_them_by_its_own_state() {
        let roster = stepped_roster();
        let mesher = Mesher::new(&roster, ladder(), horizon(512.0));
        let cube = fixture_cube();
        let mut resident = Resident::new();
        let mut ledger = WorkLedger::default();
        let mesh = mesher
            .derive_cubic(
                &cube,
                chunk(),
                level(0),
                Effort::new(Budget::unlimited(), 1),
                &mut resident,
                &mut ledger,
            )
            .expect("the cells mesh");

        // The key is the cells' own identity — the law's state digest — and the ledger agrees with the
        // independent recount of the very cells the world holds.
        assert_eq!(mesh.key().state, cube.digest());
        let footprint = Footprint::of_chunk(&cube, chunk()).expect("the chunk's own footprint");
        let recount = recount_faces(&cube, footprint);
        assert_eq!(ledger.columns_swept, recount.columns);
        assert_eq!(ledger.cells, recount.solid_cells);
        assert_eq!(ledger.faces, recount.total_faces());
        assert_eq!(mesh.area(), ledger.faces);
        assert_eq!(ledger.columns_produced, 0, "the cells' door reads no field plane");
        assert_eq!(ledger.misses, 1);

        // The same world again is a hit with no work; and the resident's identity is its set of keys.
        let before = ledger;
        let again = mesher
            .derive_cubic(&cube, chunk(), level(0), Effort::new(Budget::unlimited(), 1), &mut resident, &mut ledger)
            .expect("served");
        assert_eq!(again.digest(), mesh.digest());
        assert_eq!(ledger.hits, before.hits + 1);
        assert_eq!(ledger.faces, before.faces, "a hit sweeps nothing");
        assert!(resident.serve(cube.digest(), chunk(), level(0)).is_some());
    }

    #[test]
    fn a_level_above_zero_on_a_cell_world_is_refused_by_name() {
        let roster = flat_roster();
        let mesher = Mesher::new(&roster, ladder(), horizon(512.0));
        let cube = chunk_cube(&roster, &world());
        let mut resident = Resident::new();
        let mut ledger = WorkLedger::default();
        let refused = mesher
            .derive_cubic(&cube, chunk(), level(1), Effort::new(Budget::unlimited(), 1), &mut resident, &mut ledger)
            .expect_err("the ladder does not coarsen a cell world");
        assert_eq!(refused.kind(), RefusalKind::ParameterOutOfRange);
        assert!(refused.detail().contains("level 1"), "{refused}");
        assert!(refused.detail().contains("level 0 meshes it"), "{refused}");
        assert!(resident.is_empty(), "a refusal caches nothing");
        assert_eq!(ledger.refusals, 1);
    }

    #[test]
    fn a_chunk_with_no_column_under_the_world_is_refused_by_name() {
        let roster = flat_roster();
        let mesher = Mesher::new(&roster, ladder(), horizon(4096.0));
        let cube = chunk_cube(&roster, &world());
        let mut resident = Resident::new();
        let mut ledger = WorkLedger::default();
        // The fixture world is one chunk (0, 0, 0): the chunk beside it has columns the world does not own.
        let refused = mesher
            .derive_cubic(
                &cube,
                ChunkKey::new(1, 0, 0),
                level(0),
                Effort::new(Budget::unlimited(), 1),
                &mut resident,
                &mut ledger,
            )
            .expect_err("no column of this world is under that chunk");
        assert_eq!(refused.kind(), RefusalKind::GeometryOutsideTheWorld);
        assert!(refused.detail().contains("x 0..31"), "{refused}");
        assert!(resident.is_empty());
    }

    #[test]
    fn the_laws_own_step_re_keys_the_mesh_and_moves_no_ground_face() {
        let roster = stepped_roster();
        let mesher = Mesher::new(&roster, ladder(), horizon(512.0));
        let cube = fixture_cube();
        let mut law = Automaton::over(&cube);
        let state_before = law.digest();

        let mut resident = Resident::new();
        let mut ledger = WorkLedger::default();
        let before = mesher
            .derive_cubic(&cube, chunk(), level(0), Effort::new(Budget::unlimited(), 1), &mut resident, &mut ledger)
            .expect("the cells mesh");
        assert_eq!(before.key().state, state_before);

        // One tick of the declared law, over the cells themselves.
        let tick = law.step().expect("the law's own pour is not refused");
        assert_eq!(tick, Tick(1));
        let state_after = law.digest();
        assert_ne!(state_after, state_before, "the rain moved the water, so the state moved");

        // The world the law stepped is a **different** key: the kept geometry is not served for it.
        assert!(
            resident.serve(state_after, chunk(), level(0)).is_none(),
            "a mesh of the older state is never served for the new one"
        );
        assert_eq!(resident.stale(state_after), 1);
        let after = mesher
            .derive_cubic(
                law.world(),
                chunk(),
                level(0),
                Effort::new(Budget::unlimited(), 1),
                &mut resident,
                &mut ledger,
            )
            .expect("the stepped cells mesh");
        assert_eq!(after.key().state, state_after);
        assert_ne!(after.digest(), before.digest());

        // The ground did not move, so the solid faces are the same set; the water is the law's own state,
        // so its own faces are the world's, counted where they are.
        let solid_before: Vec<Quad> = before
            .quads()
            .iter()
            .copied()
            .filter(|quad| quad.layer == Layer::Solid)
            .collect();
        let solid_after: Vec<Quad> = after
            .quads()
            .iter()
            .copied()
            .filter(|quad| quad.layer == Layer::Solid)
            .collect();
        assert_eq!(solid_before, solid_after, "one tick of rain moves no ground");
        assert_eq!(resident.invalidate(state_after), 1, "the older state's entry is removed");
        assert_eq!(resident.len(), 1);
        assert!(resident.serve(state_after, chunk(), level(0)).is_some());
    }

    // -----------------------------------------------------------------
    // The merge itself, on faces: exactly tiled, whatever the pattern.
    // -----------------------------------------------------------------

    #[test]
    fn merge_tiles_every_group_exactly_whatever_the_pattern() {
        // An L-shaped hole and a striped pattern on one plane, plus a second plane in another material:
        // every face belongs to exactly one rectangle, and no rectangle crosses a difference.
        let mut faces = Vec::new();
        for x in 0..6 {
            for y in 0..6 {
                if x >= 2 && y >= 2 {
                    continue;
                }
                if x >= 2 && y % 2 == 1 {
                    continue;
                }
                faces.push(Face {
                    layer: Layer::Solid,
                    axis: Axis::Z,
                    facing: 1,
                    plane: 5,
                    u: x,
                    v: y,
                    material: MaterialId(1),
                });
            }
        }
        // The same plane in another material is another group; the same material on another plane is one.
        for x in 0..3 {
            faces.push(Face {
                layer: Layer::Solid,
                axis: Axis::Z,
                facing: 1,
                plane: 5,
                u: x,
                v: 9,
                material: MaterialId(2),
            });
            faces.push(Face {
                layer: Layer::Solid,
                axis: Axis::Z,
                facing: 1,
                plane: 7,
                u: x,
                v: 9,
                material: MaterialId(1),
            });
        }
        // A face no neighbour shares stays a rectangle of one, twice over: one beside the pattern,
        // one alone in a material of its own.
        faces.push(Face { layer: Layer::Solid, axis: Axis::Z, facing: 1, plane: 5, u: 8, v: 0, material: MaterialId(1) });
        faces.push(Face { layer: Layer::Water, axis: Axis::Z, facing: 1, plane: 5, u: 8, v: 8, material: MaterialId(5) });
        let total = faces.len() as u64;
        let (quads, tally) = merge_faces(faces);
        assert_eq!(tally.faces, total, "every face entered the merge");

        // Exact tiling: expand every rectangle back into the faces it stands for and compare the sets.
        let mut covered = BTreeSet::new();
        for quad in &quads {
            for du in 0..quad.span_u {
                for dv in 0..quad.span_v {
                    assert!(
                        covered.insert((
                            quad.layer,
                            quad.axis,
                            quad.facing,
                            quad.plane,
                            quad.u + du as i32,
                            quad.v + dv as i32,
                            quad.material
                        )),
                        "two rectangles overlap at {quad:?}"
                    );
                }
            }
        }
        assert_eq!(covered.len() as u64, total, "the rectangles stand for exactly the faces");
        assert_eq!(quads.iter().map(|quad| quad.area()).sum::<u64>(), total);
        assert_eq!(tally.rectangles_merged, quads.iter().filter(|quad| quad.area() > 1).count() as u64);
        // A group of one face is a rectangle of one, and nothing merged it away: the two declared
        // loners stand as their own rectangles.
        let lonely = quads.iter().filter(|quad| quad.area() == 1).count() as u64;
        assert_eq!(lonely, 2, "the two declared loners must not be merged: {quads:?}");
        assert_eq!(tally.faces, quads.iter().map(|quad| quad.area()).sum::<u64>());
    }
}
