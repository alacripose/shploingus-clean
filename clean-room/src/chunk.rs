//! **The cubic world's cells, their one identity, and the column world as a reading of them.**
//!
//! Derived from the **documents**, never from the witness's source:
//!
//! | Source | What it declares here |
//! |---|---|
//! | `docs/DESIGN-SPEC.md` §1.2 and §5 | **"Chunk = 32³ voxels"** on three axes, `z` up, signed coordinates — the settled first-target contract this module's cells live in |
//! | [`crate::coordinates`] (U2, built) | the **one** coordinate algebra: [`VoxelPos`], [`ChunkKey`], [`WithinChunk::flat`] (`x + 32·(y + 32·z)`, `0..32768`). This module does not restate a second one |
//! | `patches/PATCH-040-the-column-world-against-the-cubic-chunk-format.md` (which named the four rulings), ruled in `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §37 and `docs/LEDGER.md` **C-189** | **the shape ruling**: the product is the cubic chunk; the **column** representation is refused **for cause** — the user's own report is that it produced the material flicker in the WGSL path, and that is why this branch exists |
//! | §35 and §36 | **one water truth**: the water's depth is the authority and any surface is derived from it; this module keeps that identity across the quantisation (`water_level = surface + depth`) |
//!
//! # What this module is, and what it is not
//!
//! It **is** the world: one owner for the cells (`CubicWorld`), one identity ([`CubicWorld::digest`]), and
//! one reading of them ([`CubicWorld::column`]) — the **surface reading**, which is exactly what the column
//! world honestly survives as (ruling 1). A single reading of each column's ground, its water and its
//! material is what an existing consumer needs, so [`crate::being::WorldRead::over_cubic`] hands the *same*
//! `WorldRead` struct back out of the cells, and the automaton, the being and the client consume it
//! **unchanged** — no second world model, no second routing law (the receivers are still
//! [`crate::shapers::water::d8_receivers`] over the surface this module returns).
//!
//! It is **not** a second coordinate system, **not** a render path, and **not** a generator: the record's
//! stages produce a **surface** (per-column `elevation`, `water_level` and `lithology`), so a cubic world is
//! **built from** that surface by [`CubicWorld::from_surface`]. That is an **admission**, and it is named
//! here rather than implied: a world whose own stages wrote voxels does not exist yet in this tree, and the
//! two refused stage bodies that would be exactly that (`caves`, `earthworks`) are owed by name in
//! `src/systems.rs`. Until they land, this world is a *volume filled from a surface*.
//!
//! # One concern, one owner
//!
//! | concern | owner |
//! |---|---|
//! | **the cells** | [`CubicWorld`] — `BTreeMap<ChunkKey, Chunk>`, `32³` cells each, in [`WithinChunk::flat`] order |
//! | **the world's one identity** | [`CubicWorld::digest`] — every chunk key and every cell, in a canonical order, byte-framed. The column world's own digest is [`crate::being::surface_digest`], named for what it hashes |
//! | **the reading** | [`CubicWorld::column`] — the surface reading: the top solid cell, the water above it, and the material. It is what the column world *is*, now derived instead of authoritative |
//! | **the build** | [`ChunkSource::of`] (values in) and [`ChunkSource::over`] (the declared roster in: one read through [`surface_fields`], then the same resolution), materialised by [`ChunkSource::world`] — the one admission, with the quantisation it takes declared on the field |
//! | **the cells' own law** | [`column_cell`] — what one column has at one height, applied at two scopes: every chunk at once ([`ChunkSource::world`]) or one key on demand ([`ChunkSource::chunk`]) |
//! | **what a cell holds** | [`Cell`] — an admitted material **or air, named** (`None`, never a default material) plus a water depth in whole micrometres |
//! | **which chunk exists** | [`named_chunks`] — the patch's own footprint crossed with the span's chunk layers: the world's build and the keyed door both take the answer from it, and [`CubicWorld::chunk`] / [`CubicWorld::top_chunk_at`] / [`CubicWorld::first_chunk`] answer from the keys it named |
//! | **the band law** | [`chunk_band`] — the chunk-aligned band a vertical extent implies: the world's own span and a footprint's span are this one law, so one cannot drift from the other |
//! | **the declared surface** | [`SURFACE_FIELDS`] — the three fields a world is made of, and [`surface_fields`], the one read of them |
//!
//! **Chunk existence lives here, not in callers.** The mesh port's measurement asks *this* world for its
//! chunk ([`CubicWorld::chunk`], [`CubicWorld::top_chunk_at`]) rather than duplicating the rule.
//!
//! **The keyed door, landed (2026-10-14).** The per-chunk **provider** the last two passes named as owed is
//! [`ChunkSource`]: the declared surface of one request, resolved once, answering **one** chunk at a key
//! ([`ChunkSource::chunk`]) so a streaming consumer can ask for a chunk without every chunk of the patch
//! being materialised. It is one law and not two — [`CubicWorld::from_surface`] / [`CubicWorld::from_fields`]
//! are this source applied to every key it names ([`ChunkSource::world`]), the key law is
//! [`named_chunks`], and the cell law is [`column_cell`] — so there is no second world model and no second
//! derivation for the two scopes to drift apart in. What the keyed door does **not** remove is the field
//! read: the declared water law answers *over the patch it is read on* and its answer depends on that
//! patch's rim (`stage::Patch::is_edge` — *the rim is where the region is open; water leaves there*), so
//! the three fields are still read in one evaluation over the world's own request.
//! `a_region_read_around_one_chunk_is_not_the_worlds_own_surface` measures the difference a per-chunk read
//! would make, in columns and metres, rather than this doc claiming it.
//!
//! # The quantisation, named
//!
//! A voxel is one metre (`coordinates::VOXEL_SIDE_M`), so a surface at 123.4 m lands in the voxel **123**:
//! the reading returns a surface quantised to the voxel that contains it, and a water depth carried exactly
//! in micrometres. The difference between this reading's surface and the generator's own float elevation is
//! therefore **under one voxel by construction**, and the run prints it beside the depth, which round-trips
//! exactly. Where that changes which neighbour a column drains to, the receivers differ — a consequence the
//! run also prints rather than hides.

use std::collections::BTreeMap;

use crate::coordinates::{ChunkKey, VoxelPos, WithinChunk, CHUNK_SIDE};
use crate::digest::Digest;
use crate::field::fields_at;
use crate::material::MaterialId;
use crate::mesh::Ladder;
use crate::refusal::{Refusal, RefusalKind};
use crate::stage::{Declared, FieldId, Patch, ProcessRequest};
use crate::world::{Tick, WorldIdentity};

/// Cells in one chunk: `32³`. The declared contract's own volume, not a tunable.
pub const CHUNK_CELLS: usize = (CHUNK_SIDE * CHUNK_SIDE * CHUNK_SIDE) as usize;

/// Micrometres in the voxel's own metre — one cell holds at most this much water before the next cell up
/// takes the remainder.
pub const CELL_HOLD_UM: i64 = 1_000_000;

/// **The three declared fields a world's own surface is made of**, in the declared read order. One
/// evaluation answers all three ([`surface_fields`]), so a world's columns cannot come from three runs of
/// the plane; this world's build, the mesher's chunk reading and the client all name them **here** rather
/// than each restating the list.
pub const SURFACE_FIELDS: [FieldId; 3] = [FieldId::Elevation, FieldId::WaterLevel, FieldId::Lithology];

/// **The declared surface of one request, read in one evaluation** — the world's own read of the plane it
/// is built from. A refusal from a declaring stage is the answer and is passed through by name.
pub fn surface_fields(
    roster: &Declared,
    identity: WorldIdentity,
    tick: Tick,
    request: ProcessRequest,
    workers: usize,
) -> Result<Vec<Vec<f64>>, Refusal> {
    fields_at(roster, identity, tick, &SURFACE_FIELDS, request, workers)
}

/// **The band of chunks a vertical extent implies** — the one owner of the shift from a lowest and a
/// highest cell to the chunk-aligned band that contains them: the floor is the chunk boundary at or below
/// the lowest, and the ceiling is the boundary strictly above the highest. A world's own span
/// ([`CubicWorld::from_surface`]) and a footprint's span ([`crate::mesh::Footprint`]) are this same law at
/// two scales, so a footprint is extensible to whole chunks exactly the way a world is, and one cannot
/// drift from the other.
pub fn chunk_band(lowest: i32, highest: i32) -> (i32, i32) {
    (
        lowest.div_euclid(CHUNK_SIDE) * CHUNK_SIDE,
        (highest.div_euclid(CHUNK_SIDE) + 1) * CHUNK_SIDE,
    )
}

/// **The chunk index ranges one patch and one vertical span name** — the one place the key law's arithmetic
/// lives, so *which chunks exist* ([`named_chunks`]) and *is this key one of them* ([`ChunkSource::owns`])
/// cannot drift apart. The plan ranges come from the patch's own first and last column; the layer range is
/// the span's chunk-aligned floor up to the boundary **below** its ceiling (the span's ceiling is
/// exclusive, as [`chunk_band`] leaves it).
fn chunk_extent(patch: Patch, span: (i32, i32)) -> ((i32, i32), (i32, i32), (i32, i32)) {
    let origin = patch.origin();
    let side = patch.side() as i32;
    let (floor_z, ceiling_z) = span;
    (
        (
            origin.x.div_euclid(CHUNK_SIDE),
            (origin.x + side - 1).div_euclid(CHUNK_SIDE),
        ),
        (
            origin.y.div_euclid(CHUNK_SIDE),
            (origin.y + side - 1).div_euclid(CHUNK_SIDE),
        ),
        (
            floor_z.div_euclid(CHUNK_SIDE),
            ceiling_z.div_euclid(CHUNK_SIDE) - 1,
        ),
    )
}

/// **The chunks a patch and a vertical span name**, in key order (x, then y, then z) — the one law of
/// *which chunks exist*, and the list of chunks a world of this patch spans.
fn named_chunks(patch: Patch, span: (i32, i32)) -> Vec<ChunkKey> {
    let ((x0, x1), (y0, y1), (z0, z1)) = chunk_extent(patch, span);
    let mut keys = Vec::new();
    for x in x0..=x1 {
        for y in y0..=y1 {
            for z in z0..=z1 {
                keys.push(ChunkKey::new(x, y, z));
            }
        }
    }
    keys
}

/// The refusal an unowned key gets, in **one** place: [`CubicWorld::chunk`] returns it whole,
/// [`CubicWorld::cell`] prefixes it with the position it was asked about, and [`ChunkSource::chunk`]
/// answers with the same sentence — so no door can name two different counts of what this world owns.
fn unowned_chunk_refusal(key: ChunkKey, owned: usize) -> Refusal {
    Refusal::new(
        RefusalKind::ParameterOutOfRange,
        format!(
            "chunk ({}, {}, {}) is not one of the {} this world owns",
            key.x, key.y, key.z, owned
        ),
    )
}

/// **The cell one column has at one height** — the whole filling law, in one place: **solid** at and below
/// the column's ground, **water** above it at most one voxel's worth a cell ([`CELL_HOLD_UM`]), and air
/// where the column stands no water at all. Only heights the world owns are ever asked of it (a chunk's own
/// `z` range is inside the span [`chunk_band`] gave the world), so the ground is the only boundary this law
/// consults and there is no second place for the floor to be tested.
fn column_cell(ground: i32, depth_um: i64, material: MaterialId, z: i32) -> Cell {
    if z <= ground {
        Cell::solid(material)
    } else {
        let above = (z - ground - 1) as i64;
        Cell { material: None, water_um: (depth_um - above * CELL_HOLD_UM).clamp(0, CELL_HOLD_UM) }
    }
}

/// **One cell.** What is there — a material admitted by the registry, or **air, named** — and the water it
/// holds, in whole micrometres. Air is `None` rather than a default material: a cell nobody filled is not a
/// cell made of something (`LEDGER.md` C-39's rule, read for cells instead of colours).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Cell {
    pub material: Option<MaterialId>,
    pub water_um: i64,
}

impl Cell {
    /// The named empty: no material, no water.
    pub const AIR: Cell = Cell { material: None, water_um: 0 };

    /// A solid cell of an admitted material, holding no water.
    pub const fn solid(material: MaterialId) -> Cell {
        Cell { material: Some(material), water_um: 0 }
    }

    /// Whether anything is here.
    pub const fn is_solid(self) -> bool {
        self.material.is_some()
    }
}

/// **One chunk** — a key and its `32³` cells, nothing else. The key is the unit of ownership and streaming
/// ([`crate::coordinates`]), so a chunk that is not where its key says is not representable.
#[derive(Clone, Debug, PartialEq)]
pub struct Chunk {
    key: ChunkKey,
    cells: Vec<Cell>,
}

impl Chunk {
    fn air(key: ChunkKey) -> Chunk {
        Chunk { key, cells: vec![Cell::AIR; CHUNK_CELLS] }
    }

    pub const fn key(&self) -> ChunkKey {
        self.key
    }

    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    /// The cells, writable — the module's own door, so the two laws that write them (the pour, and a
    /// source deriving a chunk) have one and nothing outside this module can reach one.
    fn cells_mut(&mut self) -> &mut [Cell] {
        &mut self.cells
    }

    /// The cell at an offset within this chunk.
    pub fn cell(&self, at: WithinChunk) -> Cell {
        self.cells[at.flat() as usize]
    }
}

/// **The world's own cells**, keyed by chunk, with the patch its own origin and side name and the whole
/// chunks it spans in `z`.
///
/// The chunks are a `BTreeMap`, so the identity's chunk order is the key order and no insert order can
/// move it; the cells inside a chunk are in [`WithinChunk::flat`] order, which is the coordinate law's own.
#[derive(Clone, Debug, PartialEq)]
pub struct CubicWorld {
    chunks: BTreeMap<ChunkKey, Chunk>,
    patch: Patch,
    floor_z: i32,
    ceiling_z: i32,
}

/// **A column, read out of the cells** — the surface reading the column world survives as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CubicColumn {
    /// The plan position this reading is about.
    pub x: i32,
    pub y: i32,
    /// The voxel that contains the column's ground — the surface, quantised to the metre the world is made of.
    pub surface_z: i32,
    /// The ground's height in micrometres, quantised: `surface_z` metres.
    pub surface_um: i64,
    /// **The water this column stands**, in micrometres — the sum of the cells above the surface, and the
    /// authority the surface is derived from (`water_level = surface + depth`, §35).
    pub water_um: i64,
    /// The material of the top solid cell, or air where the column has no solid cell at all.
    pub material: Option<MaterialId>,
    /// How many cells of this column are solid — a question the column world has no field for.
    pub solid: usize,
}

impl CubicColumn {
    /// The water's own surface, derived from the authority: `surface + depth` (§35's one water truth, kept
    /// across the quantisation).
    pub const fn water_level_um(self) -> i64 {
        self.surface_um + self.water_um
    }

    /// Whether the world's own law stands water here.
    pub const fn water_stands(self) -> bool {
        self.water_um > 0
    }
}

/// **The law's own read of one column** — what a water law needs and nothing beside it: the voxel that
/// holds the column's ground, and the water its cells above hold. [`CubicWorld::column`] answers this plus
/// the material and the count the *reading* carries; a law that moves water has no business with either.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColumnWater {
    /// The highest solid cell of the column — the voxel its ground sits in, and the floor the water above
    /// it stands on. A column with no solid cell at all answers the world's own floor, exactly as
    /// [`CubicWorld::column`] does.
    pub surface_z: i32,
    /// **The water the cells above the ground hold**, in whole micrometres — the sum the law steps.
    pub water_um: i64,
}

/// **The refusal a chunk put under another key gets** — one sentence, shared by the store's own door
/// ([`CubicWorld::put`]) and by a caller that checks before it mutates so that a step is all or nothing
/// ([`crate::stream::Streamer::observe`]). A store that answers one key with another chunk's cells has two
/// answers to one question, so the refusal names both keys rather than rounding the mismatch away.
pub(crate) fn key_mismatch_refusal(key: ChunkKey, own: ChunkKey) -> Refusal {
    Refusal::new(
        RefusalKind::ParameterOutOfRange,
        format!(
            "a chunk of ({}, {}, {}) was put under the key ({}, {}, {}): a chunk is put under its own key, \
             or the store answers one key with another chunk's cells",
            own.x, own.y, own.z, key.x, key.y, key.z
        ),
    )
}

/// The global position of a cell by its flat index inside a chunk — the coordinate law's own order
/// (`x + 32·(y + 32·z)`), widened to `i64` so a chunk key at the edge of the coordinate system cannot
/// wrap on its way out.
fn global_cell(key: ChunkKey, at: usize) -> (i32, i32, i32) {
    let side = CHUNK_SIDE as i64;
    let x = key.x as i64 * side + (at as i64 % side);
    let y = key.y as i64 * side + ((at as i64 / side) % side);
    let z = key.z as i64 * side + (at as i64 / (side * side));
    (x as i32, y as i32, z as i32)
}

/// **One cell's column in a patch's own order**, or `None` for a cell the world owns but the patch does not
/// name — the mapping every sweep of the cells shares, in one place.
fn patch_index(patch: Patch, key: ChunkKey, at: usize) -> Option<usize> {
    let origin = patch.origin();
    let side = patch.side() as i32;
    let (x, y, _) = global_cell(key, at);
    if x < origin.x || y < origin.y || x >= origin.x + side || y >= origin.y + side {
        return None;
    }
    Some(((x - origin.x) + (y - origin.y) * side) as usize)
}

impl CubicWorld {
    /// **Build a cubic world from the declared surface.**
    ///
    /// The one admission this slice takes, unchanged: the record's stages produce per-column fields, so
    /// each column's ground voxel and the solid stack beneath it are filled with the material its
    /// `lithology` class maps to through the reference world's own [`Ladder`] — the *same* mapping the
    /// mesher and the shell use — and the water the column stands is carried in the cells **above** the
    /// surface, at most one voxel's worth per cell ([`CELL_HOLD_UM`]). The world spans **whole chunks**:
    /// from the chunk holding the lowest ground to the chunk holding the highest water, both ends snapped
    /// outwards to the chunk boundary; nothing is invented above or below that span, and nothing is clamped.
    ///
    /// The *whole* of that is now [`ChunkSource`]: this door resolves the fields
    /// ([`ChunkSource::of`]) and materialises every chunk the source names ([`ChunkSource::world`]), so a
    /// world built here and a single chunk derived on demand ([`ChunkSource::chunk`]) are one derivation at
    /// two scopes rather than two derivations that can drift.
    pub fn from_surface(
        patch: Patch,
        elevation: &[f64],
        water_level: &[f64],
        classes: &[f64],
        ladder: &Ladder,
    ) -> Result<CubicWorld, Refusal> {
        Ok(ChunkSource::of(patch, elevation, water_level, classes, ladder)?.world())
    }

    /// **The world the declared fields make over a request** — the one door from the roster to a cubic
    /// world: read the three surface fields in one evaluation ([`surface_fields`]) and materialise the world
    /// over them ([`ChunkSource::over`] → [`ChunkSource::world`]). A caller that names a request and wants
    /// the world over it does not restate those two steps, so the client, the evidence run and the port's own
    /// fixture are one reading of one declaration.
    pub fn from_fields(
        roster: &Declared,
        identity: WorldIdentity,
        tick: Tick,
        request: ProcessRequest,
        ladder: &Ladder,
        workers: usize,
    ) -> Result<CubicWorld, Refusal> {
        Ok(ChunkSource::over(roster, identity, tick, request, ladder, workers)?.world())
    }

    /// **The world's chunk at a key** — the keyed door to what this world owns. Every cell of this world is
    /// one cell of one of these, so a caller that has a key asks for the chunk itself rather than probing a
    /// cell to discover whether one is there.
    pub fn chunk(&self, key: ChunkKey) -> Result<&Chunk, Refusal> {
        self.chunks.get(&key).ok_or_else(|| self.refuse_chunk(key))
    }

    /// The refusal an unowned key gets — the one sentence ([`unowned_chunk_refusal`]) the keyed door and a
    /// [`ChunkSource`] both answer with, carrying the count of what this world owns.
    fn refuse_chunk(&self, key: ChunkKey) -> Refusal {
        unowned_chunk_refusal(key, self.chunks.len())
    }

    /// The cells of one position, or a refusal naming the place no chunk of this world holds.
    pub fn cell(&self, pos: VoxelPos) -> Result<Cell, Refusal> {
        let chunk = self.chunk(pos.chunk()).map_err(|missing| {
            Refusal::new(
                RefusalKind::ParameterOutOfRange,
                format!(
                    "({}, {}, {}) is not in this world: its {}",
                    pos.x,
                    pos.y,
                    pos.z,
                    missing.detail()
                ),
            )
        })?;
        Ok(chunk.cell(pos.within_chunk()))
    }

    /// **What material one cell holds** — the read side of the one door every consumer of a material goes
    /// through, and the reason no class ladder is anywhere in the appearance path: *what is here* is a
    /// cell's own fact ([`Cell::material`]), and this is the read of it. `None` is air, **named** — never a
    /// default material a caller could believe.
    ///
    /// A place no chunk of this world holds is refused by name, through [`CubicWorld::cell`]: the two doors
    /// ask one question and neither restates the other's refusal.
    pub fn material_at(&self, pos: VoxelPos) -> Result<Option<MaterialId>, Refusal> {
        Ok(self.cell(pos)?.material)
    }

    /// **Write one cell's material** — the write side of that door, and the act that makes a cell's material
    /// *the* material at that cell rather than a class's. What a shaper filled in can be changed one cell at
    /// a time, and every reader follows without a second place to update: this read
    /// ([`CubicWorld::material_at`]), the column reading ([`CubicWorld::column`]), the mesher's sweep over the
    /// cells, and both frames drawn from them.
    ///
    /// `None` writes **air, named** — a cell that is not made of anything — and leaves the cell's water
    /// alone: this is the door for a material, and [`CubicWorld::pour_columns`] is the door for water. The
    /// world's own identity covers its cells ([`CubicWorld::digest`]), so a written cell moves the state
    /// every drawn frame is keyed by and nothing keeps the old material beside it.
    ///
    /// A place no chunk of this world holds is refused **by name** (the position, and the world's own count
    /// of what it owns). **The id itself is the reader's to admit**: this world holds no registry, so an
    /// unadmitted material is refused where it is looked up ([`crate::material::MaterialRegistry::material`])
    /// rather than here, and this door promises nothing about a material it was never shown a record for.
    pub fn place_material(&mut self, pos: VoxelPos, material: Option<MaterialId>) -> Result<(), Refusal> {
        let key = pos.chunk();
        let held = self.chunks.len();
        let chunk = self.chunks.get_mut(&key).ok_or_else(|| {
            Refusal::new(
                RefusalKind::ParameterOutOfRange,
                format!(
                    "({}, {}, {}) is not in this world: its {}",
                    pos.x,
                    pos.y,
                    pos.z,
                    unowned_chunk_refusal(key, held).detail()
                ),
            )
        })?;
        chunk.cells_mut()[pos.within_chunk().flat() as usize].material = material;
        Ok(())
    }

    /// **The highest chunk this world owns over one plan position** — the world's own answer to *is there a
    /// chunk here at all*, asked from the top of the world's own span, which is where the surface and the
    /// water are. `None` when the world owns no chunk over that plan position. This is the question the
    /// mesh port's measurement asks before it hands a chunk to the parent's mesher, and it is answered from
    /// the world's own keys rather than by probing a cell.
    pub fn top_chunk_at(&self, cx: i32, cy: i32) -> Option<ChunkKey> {
        let (floor_z, ceiling_z) = self.span_z();
        for chunk_z in (floor_z.div_euclid(CHUNK_SIDE)..ceiling_z.div_euclid(CHUNK_SIDE)).rev() {
            let key = ChunkKey::new(cx, cy, chunk_z);
            if self.chunks.contains_key(&key) {
                return Some(key);
            }
        }
        None
    }

    /// **The world's own first chunk** — the highest chunk over the patch's own corner, which is the chunk
    /// carrying the surface and the water a picture of this world is made of. The world's keys are not
    /// zero-based: [`CubicWorld::from_surface`] snaps the vertical span outwards from the surface's own
    /// lowest and highest point, so where a world's first chunk is is a fact about *that* world.
    pub fn first_chunk(&self) -> Option<ChunkKey> {
        let origin = self.patch.origin();
        self.top_chunk_at(origin.x.div_euclid(CHUNK_SIDE), origin.y.div_euclid(CHUNK_SIDE))
    }

    /// The patch this world was built over.
    pub const fn patch(&self) -> Patch {
        self.patch
    }

    /// How many chunks this world owns.
    pub fn chunks(&self) -> usize {
        self.chunks.len()
    }

    /// How many cells the identity covers — every cell of every chunk, air included.
    pub fn cells(&self) -> usize {
        self.chunks.len() * CHUNK_CELLS
    }

    /// How many of those cells hold a material.
    pub fn solid(&self) -> usize {
        self.chunks
            .values()
            .flat_map(|chunk| chunk.cells())
            .filter(|cell| cell.is_solid())
            .count()
    }

    /// The vertical span, in voxels: the floor (inclusive) and the ceiling (exclusive).
    pub const fn span_z(&self) -> (i32, i32) {
        (self.floor_z, self.ceiling_z)
    }

    /// **This world's own frame, holding nothing** — the same patch, the same span, and no chunk: the cell
    /// store a residency fills chunk by chunk. Its identity is the empty set's of this patch and span
    /// ([`CubicWorld::digest`]), so a residency that holds nothing has nothing's identity and a residency
    /// that holds every chunk of this world has **the world's own** — the two are comparable because both
    /// digests frame the same patch and the same span.
    pub fn bare(&self) -> CubicWorld {
        CubicWorld {
            chunks: BTreeMap::new(),
            patch: self.patch,
            floor_z: self.floor_z,
            ceiling_z: self.ceiling_z,
        }
    }

    /// The keys this world holds, in ascending order — the world's own key law, enumerated.
    pub fn keys(&self) -> Vec<ChunkKey> {
        self.chunks.keys().copied().collect()
    }

    /// Whether this world holds a chunk at a key: the question [`CubicWorld::chunk`] answers, asked without
    /// building a refusal.
    pub fn holds(&self, key: ChunkKey) -> bool {
        self.chunks.contains_key(&key)
    }

    /// **Put one chunk in, at the key it is** — the door a residency derives through. A chunk whose own key
    /// is not the key it is put under is refused **by name**: a store that answers one key with another
    /// chunk's cells has two answers to one question, and the refusal says which key was asked for and which
    /// chunk arrived. Returns the chunk that was there, if any.
    pub fn put(&mut self, key: ChunkKey, chunk: Chunk) -> Result<Option<Chunk>, Refusal> {
        if chunk.key() != key {
            return Err(key_mismatch_refusal(key, chunk.key()));
        }
        Ok(self.chunks.insert(key, chunk))
    }

    /// **Take one chunk out** — the door a residency evicts through: the cells stop being held. Returns the
    /// chunk that was there, if any.
    pub fn take(&mut self, key: ChunkKey) -> Option<Chunk> {
        self.chunks.remove(&key)
    }

    /// **The world's one identity.**
    ///
    /// Every chunk in key order, and inside it every cell in [`WithinChunk::flat`] order, each cell encoded
    /// as **a tag** (`0` air, `1` material — so an absent material is a value and never a sentinel that a
    /// real id could collide with), the material's 16 bits little-endian, and the water's 64 bits
    /// little-endian. The patch's own origin and side are framed in first, so two worlds of the same cells
    /// at different places are two identities.
    pub fn digest(&self) -> Digest {
        let mut bytes = Vec::with_capacity(64 + self.chunks.len() * (12 + CHUNK_CELLS * 11));
        let origin = self.patch.origin();
        bytes.extend_from_slice(&origin.x.to_le_bytes());
        bytes.extend_from_slice(&origin.y.to_le_bytes());
        bytes.extend_from_slice(&origin.z.to_le_bytes());
        bytes.extend_from_slice(&self.patch.side().to_le_bytes());
        bytes.extend_from_slice(&self.floor_z.to_le_bytes());
        bytes.extend_from_slice(&self.ceiling_z.to_le_bytes());
        for (key, chunk) in &self.chunks {
            bytes.extend_from_slice(&key.x.to_le_bytes());
            bytes.extend_from_slice(&key.y.to_le_bytes());
            bytes.extend_from_slice(&key.z.to_le_bytes());
            for cell in chunk.cells() {
                match cell.material {
                    Some(material) => {
                        bytes.push(1);
                        bytes.extend_from_slice(&material.0.to_le_bytes());
                    }
                    None => bytes.push(0),
                }
                bytes.extend_from_slice(&cell.water_um.to_le_bytes());
            }
        }
        Digest::of(&bytes)
    }

    /// **The reading: one column of the cells.** The surface is the highest solid cell of the column, the
    /// water is the sum of what the cells **above** it hold, and the material is the surface cell's own.
    ///
    /// This is what the column world honestly becomes (ruling 1): a *reading* of the cubic world's surface,
    /// with one thing it could never answer before — how much solid stands under the surface (`solid`).
    pub fn column(&self, at: VoxelPos) -> Result<CubicColumn, Refusal> {
        let origin = self.patch.origin();
        let side = self.patch.side() as i32;
        if at.x < origin.x
            || at.y < origin.y
            || at.x >= origin.x + side
            || at.y >= origin.y + side
        {
            return Err(Refusal::new(
                RefusalKind::ParameterOutOfRange,
                format!(
                    "({}, {}) is outside this world's own patch: it spans x {}..{}, y {}..{}",
                    at.x,
                    at.y,
                    origin.x,
                    origin.x + side - 1,
                    origin.y,
                    origin.y + side - 1
                ),
            ));
        }
        let mut solid = 0usize;
        let mut surface_z = None;
        for z in (self.floor_z..self.ceiling_z).rev() {
            if self.cell(VoxelPos::new(at.x, at.y, z))?.is_solid() {
                solid += 1;
            }
        }
        for z in (self.floor_z..self.ceiling_z).rev() {
            if self.cell(VoxelPos::new(at.x, at.y, z))?.is_solid() {
                surface_z = Some(z);
                break;
            }
        }
        let (surface_z, material) = match surface_z {
            Some(z) => (z, self.cell(VoxelPos::new(at.x, at.y, z))?.material),
            None => (self.floor_z, None),
        };
        let mut water_um = 0i64;
        for z in (surface_z + 1)..self.ceiling_z {
            water_um += self.cell(VoxelPos::new(at.x, at.y, z))?.water_um;
        }
        Ok(CubicColumn {
            x: at.x,
            y: at.y,
            surface_z,
            surface_um: surface_z as i64 * 1_000_000,
            water_um,
            material,
            solid,
        })
    }

    /// **The reading of every column of the patch, in the patch's own order** — the same order as the
    /// declared request's fields, row by row (`cell = y · side + x`), so a reading and the field it replaces
    /// describe the same column at the same index.
    pub fn columns(&self) -> Result<Vec<CubicColumn>, Refusal> {
        let origin = self.patch.origin();
        let side = self.patch.side() as i32;
        let mut out = Vec::with_capacity(self.patch.len());
        for cell in 0..self.patch.len() {
            let x = origin.x + (cell % side as usize) as i32;
            let y = origin.y + (cell / side as usize) as i32;
            out.push(self.column(VoxelPos::new(x, y, 0))?);
        }
        Ok(out)
    }

    /// The voxel that holds every patch column's ground, in the patch's own order — the cells' own
    /// ground, read in **one sweep**.
    fn grounds(&self) -> Vec<i32> {
        let mut grounds = vec![self.floor_z; self.patch.len()];
        for chunk in self.chunks.values() {
            let key = chunk.key();
            for (at, cell) in chunk.cells().iter().enumerate() {
                if !cell.is_solid() {
                    continue;
                }
                let Some(index) = self.patch_index(key, at) else {
                    continue;
                };
                let (_, _, z) = global_cell(key, at);
                if z > grounds[index] {
                    grounds[index] = z;
                }
            }
        }
        grounds
    }

    /// **One cell's column in the patch's own order**, or `None` for a cell the world owns but the patch
    /// does not name.
    ///
    /// The world is spanned **whole chunks**, so it owns cells outside the patch's footprint — the ones the
    /// build never reaches and the law never writes. They are part of the cubic world (and of its identity)
    /// but they are **not columns of this patch**, and a sweep that treated them as one would index past the
    /// fields it was handed. This is the one place that part of the mapping lives, so the three sweeps below
    /// cannot disagree about it.
    fn patch_index(&self, key: ChunkKey, at: usize) -> Option<usize> {
        patch_index(self.patch, key, at)
    }

    /// **The law's own read of every column of the patch**, in the patch's own order — the water the
    /// cells hold above their ground, and the ground itself, in **two sweeps** over the chunks' cells.
    ///
    /// It is **total** on the patch's own columns: a world is spanned so that every column the patch
    /// names has a chunk to own it, so there is no place here for a refusal to be about. It is **not** a
    /// second reading either: the ground is the highest solid cell and the water is the sum above it,
    /// which is [`CubicWorld::column`]'s own arithmetic — a guard measures the two against each other at
    /// every column rather than this doc claiming it.
    pub fn water_columns(&self) -> Vec<ColumnWater> {
        let grounds = self.grounds();
        let mut out: Vec<ColumnWater> = grounds
            .iter()
            .map(|surface_z| ColumnWater { surface_z: *surface_z, water_um: 0 })
            .collect();
        for chunk in self.chunks.values() {
            let key = chunk.key();
            for (at, cell) in chunk.cells().iter().enumerate() {
                if cell.water_um == 0 {
                    continue;
                }
                let Some(index) = self.patch_index(key, at) else {
                    continue;
                };
                let (_, _, z) = global_cell(key, at);
                if z > out[index].surface_z {
                    out[index].water_um += cell.water_um;
                }
            }
        }
        out
    }

    /// **The law's own write of every column's water** — the inverse of [`CubicWorld::water_columns`].
    ///
    /// Every cell above a column's ground is written with its share of the water the law pours — at most
    /// one voxel's worth ([`CELL_HOLD_UM`]) per cell, the rest standing in the cells above — so the write
    /// round-trips through the read exactly, in whole micrometres, whatever the depth. A column's ground
    /// is **never** written: this law moves water, and the ground is not its to move.
    ///
    /// Two things are refused **by name** rather than rounded away: a **negative** depth — water is a
    /// quantity, and no law here answers with less than nothing — and water that does **not fit the
    /// world's own vertical span**. The world is whole chunks; a column that would need more cells above
    /// its ground than the span holds is water this world cannot carry, and dropping the remainder
    /// silently would be a second world wearing the first one's name.
    pub fn pour_columns(&mut self, water_um: &[i64]) -> Result<(), Refusal> {
        if water_um.len() != self.patch.len() {
            return Err(Refusal::new(
                RefusalKind::ParameterOutOfRange,
                format!(
                    "the law poured {} column(s) of water into a patch of {}: a pour describes every \
                     column of the patch or it is not a pour",
                    water_um.len(),
                    self.patch.len()
                ),
            ));
        }
        let grounds = self.grounds();
        for (cell, water) in water_um.iter().enumerate() {
            if *water < 0 {
                return Err(Refusal::new(
                    RefusalKind::ParameterOutOfRange,
                    format!(
                        "column {cell} was handed {water} µm of water: water is a quantity, and a \
                         negative depth is not one"
                    ),
                ));
            }
            let headroom = (self.ceiling_z - grounds[cell] - 1) as i64;
            let needed = (*water + CELL_HOLD_UM - 1) / CELL_HOLD_UM;
            if needed > headroom {
                return Err(Refusal::new(
                    RefusalKind::ParameterOutOfRange,
                    format!(
                        "the water poured into column {cell} does not fit this world's own span: {water} \
                         µm needs {needed} cell(s) above the ground at z {}, and the world's own ceiling at \
                         z {} leaves {headroom} cell(s) of room",
                        grounds[cell], self.ceiling_z
                    ),
                ));
            }
        }
        let patch = self.patch;
        for chunk in self.chunks.values_mut() {
            let key = chunk.key();
            for (at, cell) in chunk.cells_mut().iter_mut().enumerate() {
                let Some(index) = patch_index(patch, key, at) else {
                    continue;
                };
                let (_, _, z) = global_cell(key, at);
                if z <= grounds[index] {
                    continue;
                }
                let offset = (z - grounds[index] - 1) as i64;
                cell.water_um = (water_um[index] - offset * CELL_HOLD_UM).clamp(0, CELL_HOLD_UM);
            }
        }
        Ok(())
    }
}

/// **The declared surface a world is made of, resolved — and the one door that derives a chunk of it on
/// demand.**
///
/// It owns **no cells**. It holds what the declared fields say about every column of one patch — the voxel
/// the column's ground sits in, the water it stands, the material its class names — and answers either one
/// chunk at a key ([`ChunkSource::chunk`]) or the whole patch ([`ChunkSource::world`]) through one law
/// ([`column_cell`]). The world's own build asks the whole patch and the seam's own comparison asks one key
/// at a time; both are this same source, so neither is a second derivation of the other. What a **drawn**
/// frame streams is not this declared surface: a client shows the world the plane's own law leaves
/// ([`CubicWorld`]'s own keyed door, `crate::stream`), because the law's state and this surface differ —
/// and the difference is a measured number rather than a promise.
///
/// **What it is not, named here rather than implied.** It does not read the fields one chunk at a time: the
/// three fields are read in **one** evaluation over the world's own request ([`ChunkSource::over`] →
/// [`surface_fields`]), because the declared water law is a relation over the patch it is read on and its
/// answer depends on that patch's own rim ([`crate::stage::Patch::is_edge`] — *the rim is where the region is
/// open; water leaves there*). A 32 × 32 read around one chunk would be a different surface for the columns
/// on that read's rim, and `a_region_read_around_one_chunk_is_not_the_worlds_own_surface` measures how
/// different, in columns and metres, rather than this doc claiming it. What the keyed door removes is the
/// up-front materialisation of every chunk's cells, not the field read. It is not a second world either: it
/// names no cell and answers no cell query, and it cannot disagree with [`CubicWorld`] about which chunks
/// exist because both take that answer from [`named_chunks`].
pub struct ChunkSource {
    patch: Patch,
    floor_z: i32,
    ceiling_z: i32,
    /// Per column, in patch order: the voxel the column's ground sits in, the water it stands in whole
    /// micrometres, and the material its lithology class names — the declared fields, resolved once.
    grounds: Vec<i32>,
    depths: Vec<i64>,
    materials: Vec<MaterialId>,
}

impl ChunkSource {
    /// **The declared surface of one request, read in one evaluation and resolved** — the door from a roster
    /// to a source. A refusal from a declaring stage is the answer and is passed through by name.
    pub fn over(
        roster: &Declared,
        identity: WorldIdentity,
        tick: Tick,
        request: ProcessRequest,
        ladder: &Ladder,
        workers: usize,
    ) -> Result<ChunkSource, Refusal> {
        let values = surface_fields(roster, identity, tick, request, workers)?;
        ChunkSource::of(request.patch()?, &values[0], &values[1], &values[2], ladder)
    }

    /// **The same resolution from values already in hand** — the checks and the meaning the world's own build
    /// has always taken, in the same order and with the same sentences: a field set that does not describe one
    /// patch, a surface that is not finite, a class that cannot be formed, and a material the declared ladder
    /// does not admit are each refused **by name**.
    pub fn of(
        patch: Patch,
        elevation: &[f64],
        water_level: &[f64],
        classes: &[f64],
        ladder: &Ladder,
    ) -> Result<ChunkSource, Refusal> {
        let cells = patch.len();
        if elevation.len() != cells || water_level.len() != cells || classes.len() != cells {
            return Err(Refusal::new(
                RefusalKind::ParameterOutOfRange,
                format!(
                    "a cubic world of {cells} column(s) was handed {} elevation(s), {} water level(s) and \
                     {} class(es): the fields do not describe one patch",
                    elevation.len(),
                    water_level.len(),
                    classes.len()
                ),
            ));
        }
        if cells == 0 {
            return Err(Refusal::new(
                RefusalKind::ParameterOutOfRange,
                "a patch of no columns is no world: there is no cell to own",
            ));
        }
        let lowest = elevation
            .iter()
            .chain(water_level.iter())
            .copied()
            .fold(f64::INFINITY, f64::min);
        let highest = elevation
            .iter()
            .chain(water_level.iter())
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        if !lowest.is_finite() || !highest.is_finite() {
            return Err(Refusal::new(
                RefusalKind::ParameterOutOfRange,
                "the surface this world would be filled from is not finite: there is no ground to start from",
            ));
        }
        // The span is the **band law** ([`chunk_band`]) applied to this surface's own lowest and highest
        // point — the same law a footprint's span is derived by, one scale down.
        let (floor_z, ceiling_z) = chunk_band(lowest.floor() as i32, highest.ceil() as i32);

        let mut grounds = Vec::with_capacity(cells);
        let mut depths = Vec::with_capacity(cells);
        let mut materials = Vec::with_capacity(cells);
        for cell in 0..cells {
            let class = classes[cell];
            if !class.is_finite() {
                return Err(Refusal::new(
                    RefusalKind::ParameterOutOfRange,
                    format!("lithology = {class} at column {cell}: a class cannot be formed"),
                ));
            }
            materials.push(ladder.for_class(class.round() as i64)?);
            grounds.push(elevation[cell].floor() as i32);
            // The water the column stands: the depth the declared surface carries, in whole micrometres.
            depths.push(((water_level[cell] - elevation[cell]).max(0.0) * 1_000_000.0) as i64);
        }
        Ok(ChunkSource { patch, floor_z, ceiling_z, grounds, depths, materials })
    }

    /// **The chunks this source names** — the keys a world of it owns, from the one key law
    /// ([`named_chunks`]).
    pub fn keys(&self) -> Vec<ChunkKey> {
        named_chunks(self.patch, self.span_z())
    }

    /// The vertical span this source's own surface implies: the floor (inclusive) and the ceiling
    /// (exclusive), both chunk-aligned.
    pub const fn span_z(&self) -> (i32, i32) {
        (self.floor_z, self.ceiling_z)
    }

    /// Whether a key is one of the chunks this source names — the same arithmetic [`ChunkSource::keys`]
    /// enumerates, asked of one key instead of all of them.
    pub fn owns(&self, key: ChunkKey) -> bool {
        let ((x0, x1), (y0, y1), (z0, z1)) = chunk_extent(self.patch, self.span_z());
        (x0..=x1).contains(&key.x)
            && (y0..=y1).contains(&key.y)
            && (z0..=z1).contains(&key.z)
    }

    /// **One chunk's cells, derived now** — the keyed door a streaming consumer asks. A key this source does
    /// not name is refused with the world's own sentence ([`unowned_chunk_refusal`]), and a key it *does*
    /// name answers a **whole** chunk: `32³` cells, **air, named** where this patch has no column or the
    /// column's own law says air — which is not the same thing as a chunk this world does not own.
    pub fn chunk(&self, key: ChunkKey) -> Result<Chunk, Refusal> {
        if !self.owns(key) {
            return Err(unowned_chunk_refusal(key, self.keys().len()));
        }
        Ok(self.derived(key))
    }

    /// **The whole patch at once** — every chunk this source names, through the same law the keyed door
    /// applies to one ([`ChunkSource::derived`]). This is the world's own build, and it is the keyed door
    /// asked once per key rather than a second derivation beside it.
    pub fn world(&self) -> CubicWorld {
        let mut chunks = BTreeMap::new();
        for key in self.keys() {
            chunks.insert(key, self.derived(key));
        }
        CubicWorld { chunks, patch: self.patch, floor_z: self.floor_z, ceiling_z: self.ceiling_z }
    }

    /// One chunk's cells, from the resolved columns — the law itself, for keys this source names. A cell over
    /// a place this patch has no column is left **air, named**: this world spans whole chunks, so it owns
    /// cells its own patch never reaches, and they are not columns of it.
    fn derived(&self, key: ChunkKey) -> Chunk {
        let mut chunk = Chunk::air(key);
        for at in 0..CHUNK_CELLS {
            let Some(index) = patch_index(self.patch, key, at) else {
                continue;
            };
            let (_, _, z) = global_cell(key, at);
            chunk.cells_mut()[at] =
                column_cell(self.grounds[index], self.depths[index], self.materials[index], z);
        }
        chunk
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coordinates::VoxelPos;
    use crate::process::{declared_request, roster, DECLARED_PATCH_SIDE};
    use crate::reference;
    use crate::stage::FieldId;
    use crate::world::Tick;

    /// The declared patch's own fields and the reference ladder — the same three fields the mesher and the
    /// client read, so this world is the tree's world and not a fixture.
    fn fields() -> (Patch, Vec<f64>, Vec<f64>, Vec<f64>, Ladder) {
        let roster = roster().expect("the declared roster");
        let identity = reference::identity().expect("the reference identity");
        let request = declared_request();
        let values = crate::field::fields_at(
            &roster,
            identity,
            Tick(0),
            &[FieldId::Elevation, FieldId::WaterLevel, FieldId::Lithology],
            request,
            1,
        )
        .expect("the three declared fields");
        (
            request.patch().expect("the declared patch"),
            values[0].clone(),
            values[1].clone(),
            values[2].clone(),
            reference::ladder().expect("the reference ladder"),
        )
    }

    fn world() -> CubicWorld {
        let (patch, elevation, water_level, classes, ladder) = fields();
        CubicWorld::from_surface(patch, &elevation, &water_level, &classes, &ladder)
            .expect("the declared surface fills a world")
    }

    #[test]
    fn the_world_spans_whole_chunks_over_the_surface_it_was_filled_from() {
        let (_, elevation, water_level, _, _) = fields();
        let world = world();
        let (floor_z, ceiling_z) = world.span_z();
        let lowest = elevation
            .iter()
            .chain(water_level.iter())
            .copied()
            .fold(f64::INFINITY, f64::min);
        let highest = elevation
            .iter()
            .chain(water_level.iter())
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        assert_eq!(floor_z % CHUNK_SIDE, 0, "the floor is a chunk boundary");
        assert_eq!(ceiling_z % CHUNK_SIDE, 0, "the ceiling is a chunk boundary");
        assert!((floor_z as f64) <= lowest, "nothing is below the world's own floor");
        assert!((ceiling_z as f64) > highest, "and the highest water is inside it");
        assert_eq!(world.cells(), world.chunks() * CHUNK_CELLS);
        // A patch of 48 columns, 1 m voxels, 32³ chunks: two chunks each way in x and y at least.
        assert!(world.chunks() >= 4, "the patch is wider than one chunk: {}", world.chunks());
        println!(
            "cubic world: {} chunk(s), {} cell(s) ({} solid), z {}..{}",
            world.chunks(),
            world.cells(),
            world.solid(),
            floor_z,
            ceiling_z
        );
    }

    #[test]
    fn a_column_reads_back_the_water_the_declared_field_stands() {
        let (patch, elevation, water_level, _, _) = fields();
        let world = world();
        let columns = world.columns().expect("every column of the patch reads");
        assert_eq!(columns.len(), patch.len());
        let mut worst = 0.0f64;
        for (cell, column) in columns.iter().enumerate() {
            // The water round-trips exactly: it was poured from the same whole micrometres the field's own
            // depth is read as, so the identity §35 established survives the cubic world.
            let field_um = ((water_level[cell] - elevation[cell]).max(0.0) * 1_000_000.0) as i64;
            assert_eq!(
                column.water_um, field_um,
                "column {cell}: the cubic reading must stand the water the field stands"
            );
            // The surface is quantised to the voxel that contains the ground, and never by a voxel or more.
            let delta = (column.surface_um as f64 / 1_000_000.0 - elevation[cell]).abs();
            worst = worst.max(delta);
            assert!(
                delta < 1.0,
                "column {cell}: the surface moved {delta} m, and one voxel is the most it may move"
            );
        }
        println!("cubic reading: {} column(s), the worst surface quantisation {worst:.4} m (one voxel is 1 m)", columns.len());
        assert!(worst < 1.0);
    }

    #[test]
    fn the_reading_answers_a_question_the_column_world_has_no_field_for() {
        let world = world();
        let origin = world.patch().origin();
        let column = world
            .column(VoxelPos::new(origin.x, origin.y, 0))
            .expect("a corner column of the patch");
        assert!(
            column.solid > 0,
            "the world is a volume: the column stands {} solid cell(s) under its surface",
            column.solid
        );
        assert_eq!(
            column.solid,
            (column.surface_z - world.span_z().0 + 1) as usize,
            "every cell from the floor to the surface is solid, and no column world has this number"
        );
        // And a cell below the surface is that column's own material, readable as a *cell*.
        let below = world
            .cell(VoxelPos::new(column.x, column.y, column.surface_z))
            .expect("the surface cell is in the world");
        assert!(below.is_solid());
        assert_eq!(below.material, column.material);
    }

    #[test]
    fn the_identity_is_the_cells_and_moves_with_any_of_them() {
        let mut world = world();
        let first = world.digest();
        assert_eq!(world.digest(), first, "the identity is a function of the cells");
        // One cell's water changes: one identity becomes another. **The world's whole writer is the pour**
        // ([`CubicWorld::pour_columns`]), so the change is made through it rather than through a second door
        // beside it — and the column at the patch's own corner has the room for one more micrometre that the
        // pour's own fit check asks for.
        let at = world.patch().origin();
        let mut water: Vec<i64> = world.water_columns().iter().map(|column| column.water_um).collect();
        water[0] += 1;
        world
            .pour_columns(&water)
            .expect("one micrometre more, in a column whose own span has room for it");
        assert_ne!(world.digest(), first, "one micrometre in one cell is another world");
        assert_eq!(world.digest(), world.digest(), "and it is still deterministic");
        assert_eq!(
            world.column(at).expect("a column of the patch").water_um,
            water[0],
            "and the cell that moved is the one the column reads"
        );
    }

    #[test]
    fn air_is_named_and_a_place_no_chunk_owns_is_refused_by_name() {
        let world = world();
        let (_, ceiling_z) = world.span_z();
        let origin = world.patch().origin();
        let sky = world
            .cell(VoxelPos::new(origin.x, origin.y, ceiling_z - 1))
            .expect("the top cell of the world is a cell");
        assert_eq!(sky, Cell::AIR, "a cell nobody filled is air by name");
        assert_eq!(sky.material, None, "and its material is absent, never defaulted");
        let refused = world
            .cell(VoxelPos::new(origin.x, origin.y, ceiling_z))
            .expect_err("past the ceiling is not this world");
        assert_eq!(refused.kind(), RefusalKind::ParameterOutOfRange);
        assert!(refused.detail().contains("not in this world"), "{refused}");
    }

    #[test]
    fn a_place_outside_the_patch_is_refused_by_name() {
        let world = world();
        let origin = world.patch().origin();
        let side = world.patch().side() as i32;
        let refused = world
            .column(VoxelPos::new(origin.x + side, origin.y, 0))
            .expect_err("one column past the patch is not a column of it");
        assert_eq!(refused.kind(), RefusalKind::ParameterOutOfRange);
        assert!(refused.detail().contains("outside this world's own patch"), "{refused}");
    }

    #[test]
    fn a_class_that_cannot_be_formed_is_refused_by_name() {
        let (patch, elevation, water_level, mut classes, ladder) = fields();
        classes[7] = f64::NAN;
        let refused = CubicWorld::from_surface(patch, &elevation, &water_level, &classes, &ladder)
            .expect_err("a class that is not a number cannot be formed");
        assert_eq!(refused.kind(), RefusalKind::ParameterOutOfRange);
        assert!(refused.detail().contains("a class cannot be formed"), "{refused}");
    }

    /// §35's identity, as the world's own writer keeps it: **a pour is a set of the water it carries, not a
    /// sum of pours.** The law used to live in a cell-level writer the build reached through; the build now
    /// derives its cells from the resolved columns ([`CubicWorld::from_surface`] → [`ChunkSource`]), so the
    /// pour is the world's whole writer and the law is measured here, where it lives.
    #[test]
    fn a_pour_is_a_set_of_the_water_it_carries_and_not_a_sum_of_pours() {
        let mut world = world();
        let at = world.patch().origin();
        let mut water: Vec<i64> = world.water_columns().iter().map(|column| column.water_um).collect();
        water[0] += 7;
        world.pour_columns(&water).expect("the declared span holds seven more micrometres");
        let once = world.digest();
        world.pour_columns(&water).expect("the same pour again");
        assert_eq!(
            world.digest(),
            once,
            "the same pour twice is one world: a pour is a set, not a sum"
        );
        assert_eq!(
            world.column(at).expect("a column of the patch").water_um,
            water[0],
            "a column's own depth is the number poured in, not a sum of pours"
        );
    }

    /// What this measures, said exactly: a patch one column over moves **every column's placement**, so both the
    /// cells' own bytes inside their chunks and the patch the world is over are different — the identity is not
    /// placement-blind. (The *framing* of the origin inside `digest` is not isolated by any single-subject
    /// perturbation, and the harness names that rather than pretending a case covers it.)
    #[test]
    fn a_patch_one_column_over_is_another_world() {
        let (patch, elevation, water_level, classes, ladder) = fields();
        let origin = patch.origin();
        let elsewhere = Patch::new(
            VoxelPos::new(origin.x + 1, origin.y, origin.z),
            patch.side(),
        )
        .expect("a patch one column over is a patch");
        let here = CubicWorld::from_surface(patch, &elevation, &water_level, &classes, &ladder)
            .expect("the declared surface fills a world");
        let there =
            CubicWorld::from_surface(elsewhere, &elevation, &water_level, &classes, &ladder)
                .expect("the same surface, one column over, fills a world too");
        assert_eq!(here.chunks(), there.chunks(), "the same cells, chunk for chunk");
        assert_ne!(
            here.digest(),
            there.digest(),
            "the same field values at another place are another world"
        );
    }

    /// **The ruling's central claim, as a guard**: a being's read of the cubic world is the *cells'* — one
    /// micrometre written into one cell moves the reading, which a read re-derived from the fields the world
    /// was built from could not do. (Run from this module because the world's own writer is private to it.)
    #[test]
    fn the_reading_a_being_takes_is_sourced_from_the_cells() {
        let (patch, elevation, water_level, classes, ladder) = fields();
        let mut world = CubicWorld::from_surface(patch, &elevation, &water_level, &classes, &ladder)
            .expect("the declared surface fills a world");
        let columns = world.columns().expect("every column of the patch reads");
        let cell = (0..columns.len())
            .max_by_key(|cell| columns[*cell].water_um)
            .expect("the patch has columns");
        let organisation = crate::being::Organisation::first();
        let read = crate::being::WorldRead::over_cubic(organisation, "the first mob", &world)
            .expect("the cells are readable");
        assert_eq!(
            read.depth(cell),
            columns[cell].water_um as f64 / 1_000_000.0,
            "the reading stands the cells' own water"
        );
        assert_eq!(
            read.elevation[cell],
            columns[cell].surface_um as f64 / 1_000_000.0,
            "and the cells' own surface"
        );
        // Seven more micrometres of the world's own water, written through the world's whole writer. The
        // reading is taken of the **cells**, so a change at the cells has to move it — and the pour refuses
        // what the world's span cannot carry, so the column is one whose own span has the room.
        let (_, ceiling_z) = world.span_z();
        let cell = (0..columns.len())
            .filter(|cell| {
                let headroom = (ceiling_z - columns[*cell].surface_z - 1) as i64;
                (columns[*cell].water_um + 7 + CELL_HOLD_UM - 1) / CELL_HOLD_UM <= headroom
            })
            .max_by_key(|cell| columns[*cell].water_um)
            .expect("some column of the declared patch has room for seven more micrometres");
        let mut water: Vec<i64> = world.water_columns().iter().map(|column| column.water_um).collect();
        water[cell] += 7;
        world
            .pour_columns(&water)
            .expect("seven more micrometres, in a column whose own span has room for them");
        let again = crate::being::WorldRead::over_cubic(organisation, "the first mob", &world)
            .expect("the cells are still readable");
        let moved = (columns[cell].water_um + 7) as f64 / 1_000_000.0;
        assert!(
            (again.depth(cell) - moved).abs() < 1e-12,
            "the reading follows the cells, one micrometre for one micrometre: {} against {moved}",
            again.depth(cell)
        );
    }

    #[test]
    fn the_laws_bulk_read_answers_exactly_what_the_one_column_reading_answers() {
        let world = world();
        let bulk = world.water_columns();
        let one = world.columns().expect("every column of the patch reads");
        assert_eq!(bulk.len(), one.len(), "one value per column of the patch");
        for (cell, (bulk, one)) in bulk.iter().zip(&one).enumerate() {
            assert_eq!(
                bulk.surface_z, one.surface_z,
                "column {cell}: the law's ground must be the one-column reading's ground"
            );
            assert_eq!(
                bulk.water_um, one.water_um,
                "column {cell}: the law's water must be the one-column reading's water"
            );
        }
    }

    /// §37 named `CELL_HOLD_UM`'s cap **unexercised**, because no reading could observe it. The law's pour
    /// gives it a subject: a depth past one voxel's worth stands in more than one cell, each at the cap.
    #[test]
    fn a_depth_past_one_voxels_worth_spills_into_the_next_cell() {
        let mut world = world();
        let origin = world.patch().origin();
        let at = VoxelPos::new(origin.x, origin.y, 0);
        let ground = world.column(at).expect("a column of the patch").surface_z;
        let depth = CELL_HOLD_UM + CELL_HOLD_UM / 2;
        let mut water = vec![0_i64; world.patch().len()];
        water[0] = depth;
        world
            .pour_columns(&water)
            .expect("the declared world holds one and a half voxels of water");
        let first = world
            .cell(VoxelPos::new(origin.x, origin.y, ground + 1))
            .expect("a cell above the ground");
        let second = world
            .cell(VoxelPos::new(origin.x, origin.y, ground + 2))
            .expect("the cell above it");
        assert_eq!(first.water_um, CELL_HOLD_UM, "the first cell holds one voxel's worth and no more");
        assert_eq!(second.water_um, CELL_HOLD_UM / 2, "the remainder stands in the cell above");
        assert_eq!(
            world.column(at).expect("the column reads").water_um,
            depth,
            "and the reading sums the cells back to the depth exactly"
        );
        println!(
            "cubic pour: {depth} µm stands over 2 cell(s) ({CELL_HOLD_UM} + {}), summed back exactly",
            CELL_HOLD_UM / 2
        );
    }

    #[test]
    fn a_pour_the_world_cannot_hold_is_refused_by_name() {
        let mut world = world();
        let (_, ceiling_z) = world.span_z();
        let origin = world.patch().origin();
        let ground = world
            .column(VoxelPos::new(origin.x, origin.y, 0))
            .expect("a column of the patch")
            .surface_z;
        let headroom = (ceiling_z - ground - 1) as i64;
        let mut water = vec![0_i64; world.patch().len()];
        water[0] = headroom * CELL_HOLD_UM + 1;
        let refused = world
            .pour_columns(&water)
            .expect_err("one cell's worth past the span cannot fit");
        assert_eq!(refused.kind(), RefusalKind::ParameterOutOfRange);
        assert!(
            refused.detail().contains("does not fit this world's own span"),
            "the refusal names what does not fit: {refused}"
        );
        // And a negative depth is not a quantity: it is refused too, rather than clamped to nothing.
        let mut negative = vec![0_i64; world.patch().len()];
        negative[0] = -1;
        let refused = world
            .pour_columns(&negative)
            .expect_err("water does not run negative");
        assert!(refused.detail().contains("negative depth"), "{refused}");
    }

    #[test]
    fn the_pour_round_trips_the_water_it_is_handed() {
        let mut world = world();
        let columns = world.columns().expect("every column of the patch reads");
        // A state no earlier state was: every column moved by its own index, one micrometre apart.
        let handed: Vec<i64> = columns
            .iter()
            .enumerate()
            .map(|(cell, column)| column.water_um + cell as i64)
            .collect();
        let before = world.digest();
        world.pour_columns(&handed).expect("the declared span holds it");
        let read = world.water_columns();
        for (cell, water) in handed.iter().enumerate() {
            assert_eq!(read[cell].water_um, *water, "column {cell} must read back what was poured");
        }
        assert_ne!(world.digest(), before, "and the world's own identity moved with its water");
        // **And the law's own bound is measured at the cells, not only through the reading.** The reading sums
        // only the cells *above* a column's ground, so water written into the ground itself would be invisible
        // to it while still moving the world's identity — which is why this guard reads the cells directly: the
        // pour moves water, and a column's ground is not the pour's to move.
        let (floor_z, _) = world.span_z();
        for (cell, column) in columns.iter().enumerate() {
            let at = world.patch().position(cell).expect("every patch column has a position");
            for z in [column.surface_z - 1, column.surface_z] {
                if z < floor_z {
                    continue;
                }
                let held = world
                    .cell(VoxelPos::new(at.x, at.y, z))
                    .expect("a cell the world owns")
                    .water_um;
                assert_eq!(
                    held, 0,
                    "column {cell}: the pour wrote {held} µm at or below the ground, at z {z}"
                );
            }
        }
    }

    #[test]
    fn fields_that_do_not_describe_one_patch_are_refused_by_name() {
        let (patch, elevation, water_level, classes, ladder) = fields();
        let short = &classes[1..];
        let refused = CubicWorld::from_surface(patch, &elevation, &water_level, short, &ladder)
            .expect_err("a class per column, or no world");
        assert_eq!(refused.kind(), RefusalKind::ParameterOutOfRange);
        assert!(refused.detail().contains("do not describe one patch"), "{refused}");

        // A declared request's own side is what the world is built over: the patch is the contract's.
        assert_eq!(patch.side(), DECLARED_PATCH_SIDE);
    }

    /// The declared surface over a `chunks × chunks` request, through the source's own door — the read and
    /// the resolution [`CubicWorld::from_fields`] makes, which is what the keyed door is asked of.
    fn source_over(chunks_a_side: u32) -> ChunkSource {
        let roster = roster().expect("the declared roster");
        let identity = reference::identity().expect("the reference identity");
        let ladder = reference::ladder().expect("the reference ladder");
        let origin = declared_request().patch().expect("the declared patch").origin();
        let request = ProcessRequest::Region { origin, side: CHUNK_SIDE as u32 * chunks_a_side };
        ChunkSource::over(&roster, identity, Tick(0), request, &ladder, 1)
            .expect("the declared surface resolves")
    }

    /// The declared request's own surface — the patch the contract names (`48` columns, one and a half
    /// chunks), which is the one world here whose chunks are **not** all inside its own footprint.
    fn declared_source() -> ChunkSource {
        let roster = roster().expect("the declared roster");
        let identity = reference::identity().expect("the reference identity");
        let ladder = reference::ladder().expect("the reference ladder");
        ChunkSource::over(&roster, identity, Tick(0), declared_request(), &ladder, 1)
            .expect("the declared surface resolves")
    }

    /// **The keyed door, over the world it is a door of** — every chunk of a world derived *alone*, cell for
    /// cell against the chunk that world holds, on the client's own (2 × 2) world and on a 3 × 3 one.
    ///
    /// Said plainly, because it matters: the world is materialised **through this same source**
    /// ([`ChunkSource::world`] is [`ChunkSource::derived`] once per key), so this equality is one law applied
    /// at two scopes and cannot come out otherwise — it is a *consistency* gate, not an independent recount.
    /// An independent recount would need the derivation this pass replaced, and the bytes of that are not in
    /// this tree; the byte-level evidence that the world's cells did not move is the run's own output and the
    /// port's measured block, reproduced on the final bytes and recorded in
    /// `docs/baselines/port-appearance-seam/README.txt` §9.
    #[test]
    fn a_chunk_derived_alone_is_the_chunk_the_whole_patch_holds() {
        for (label, chunks_a_side) in [("the client's own world", 2u32), ("a 3 x 3 world", 3)] {
            let source = source_over(chunks_a_side);
            let world = source.world();
            let keys = source.keys();
            assert_eq!(keys.len(), world.chunks(), "{label}: the source names what the world owns");
            let mut cells = 0usize;
            for key in &keys {
                let derived = source.chunk(*key).expect("a key the source names is one it answers");
                assert_eq!(derived.cells().len(), CHUNK_CELLS, "{label}: a derived chunk is whole");
                assert_eq!(derived.key(), *key, "{label}: and it is the chunk that key names");
                let held = world.chunk(*key).expect("a key the world owns is one it holds");
                assert_eq!(
                    &derived, held,
                    "{label}: the chunk at ({}, {}, {}) derived alone is not the world's",
                    key.x, key.y, key.z
                );
                cells += derived.cells().len();
            }
            println!(
                "keyed door: {label}: {} chunk(s) derived one at a time, {cells} cell(s) equal to the world's \
                 own, world digest {}",
                keys.len(),
                world.digest().to_hex()
            );
        }
    }

    /// **The two doors name the same world.** A key the source does not name is refused by the source and by
    /// the world with **one sentence**, so a caller learns the same count of what this world owns from either
    /// door — and a key the source *does* name is answered rather than refused.
    #[test]
    fn a_key_the_world_does_not_name_is_refused_by_the_worlds_own_sentence() {
        let source = source_over(2);
        let world = source.world();
        let origin = world.patch().origin();
        let (floor_z, _) = world.span_z();
        // Nine chunks past the footprint in x: a key the source does not name and the world does not own.
        let far = ChunkKey::new(
            origin.x.div_euclid(CHUNK_SIDE) + 9,
            origin.y.div_euclid(CHUNK_SIDE),
            floor_z.div_euclid(CHUNK_SIDE),
        );
        let ours = source.chunk(far).expect_err("nine chunks past the footprint is not this world");
        let theirs = world.chunk(far).expect_err("and the world's own door agrees");
        assert_eq!(ours.kind(), theirs.kind());
        assert_eq!(ours.detail(), theirs.detail(), "two doors, one sentence");
        assert!(ours.detail().contains("is not one of the"), "{ours}");
        assert!(source.owns(source.keys()[0]), "the source owns what it names");
        assert!(!source.owns(far), "and does not own what it refuses");
    }

    /// **A chunk over the patch's own edge is whole, and names the air beyond it.** The declared patch is
    /// `48` columns — one and a half chunks — so the world it makes owns a chunk whose far columns lie outside
    /// it. That chunk is *whole* (`32³` cells, so a key addresses it) and every cell over a column the patch
    /// does not have is **air, named**: a world spanned whole chunks must never read a column it was not
    /// handed, which is the mistake a provider indexing within-chunk offsets directly would make.
    #[test]
    fn a_chunk_over_the_patches_edge_is_whole_and_names_the_air_beyond_it() {
        let source = declared_source();
        let world = source.world();
        let patch = world.patch();
        assert_eq!(patch.side(), DECLARED_PATCH_SIDE, "the contract's own patch");
        let origin = patch.origin();
        let (floor_z, _) = world.span_z();
        let edge = ChunkKey::new(
            (origin.x + CHUNK_SIDE) .div_euclid(CHUNK_SIDE),
            origin.y.div_euclid(CHUNK_SIDE),
            floor_z.div_euclid(CHUNK_SIDE),
        );
        assert!(source.owns(edge), "the patch's second chunk is one this world owns");
        let derived = source.chunk(edge).expect("a chunk this source owns");
        assert_eq!(derived.cells().len(), CHUNK_CELLS, "the chunk is whole");
        assert_eq!(&derived, world.chunk(edge).expect("the world holds it"));
        // Inside the footprint the cell is the column's own; outside it there is no column at all.
        let inside = world
            .cell(VoxelPos::new(origin.x + CHUNK_SIDE, origin.y, floor_z))
            .expect("a column of the patch");
        assert!(inside.is_solid(), "the chunk's first column is the patch's own ground");
        let mut beyond = 0usize;
        for at in 0..CHUNK_CELLS {
            let (x, y, _) = global_cell(edge, at);
            let lacks = x < origin.x
                || y < origin.y
                || x >= origin.x + patch.side() as i32
                || y >= origin.y + patch.side() as i32;
            if lacks {
                beyond += 1;
                assert_eq!(
                    derived.cells()[at],
                    Cell::AIR,
                    "cell {at} of this chunk is over no column of this patch and must be air by name"
                );
            }
        }
        assert!(beyond > 0, "the patch is one and a half chunks wide, so a chunk straddles its edge");
        println!(
            "keyed door: the declared {}-column patch: chunk ({}, {}, {}) is whole ({} cell(s)) and {beyond} \
             of its cell(s) lie over columns this patch does not have — all of them air, named",
            patch.side(),
            edge.x,
            edge.y,
            edge.z,
            CHUNK_CELLS
        );
    }

    /// **Why the keyed door resolves the world's own read rather than reading the fields around each chunk** —
    /// measured, not asserted. The declared water law answers *over the patch it is read on*, and a patch's rim
    /// is where the region is open (`stage::Patch::is_edge` — *the rim is where the region is open; water leaves
    /// there*). So the same columns read over a narrower request do not necessarily stand the same water. This
    /// reads the declared surface twice — once over the world's own request, once over one chunk's own corner —
    /// and reports the columns that differ, the worst difference in metres, and how many of them stand on the
    /// narrow read's own rim.
    #[test]
    fn a_region_read_around_one_chunk_is_not_the_worlds_own_surface() {
        let roster = roster().expect("the declared roster");
        let identity = reference::identity().expect("the reference identity");
        let declared = declared_request();
        let patch = declared.patch().expect("the declared patch");
        let wide = crate::field::fields_at(&roster, identity, Tick(0), &SURFACE_FIELDS, declared, 1)
            .expect("the world's own read of the declared surface");
        let narrow_patch =
            Patch::new(patch.origin(), CHUNK_SIDE as u32).expect("a chunk's own corner is a patch");
        let narrow = crate::field::fields_at(
            &roster,
            identity,
            Tick(0),
            &SURFACE_FIELDS,
            ProcessRequest::Region { origin: narrow_patch.origin(), side: narrow_patch.side() },
            1,
        )
        .expect("one chunk's own corner reads on its own");
        let side = patch.side() as usize;
        let chunk_side = CHUNK_SIDE as usize;
        let mut differ = 0usize;
        let mut rim = 0usize;
        let mut worst = 0.0f64;
        for y in 0..chunk_side {
            for x in 0..chunk_side {
                let width = narrow[1].len();
                assert_eq!(width, chunk_side * chunk_side, "a chunk's corner is {chunk_side}² column(s)");
                let delta = (wide[1][y * side + x] - narrow[1][y * chunk_side + x]).abs();
                if delta > 0.0 {
                    differ += 1;
                    worst = worst.max(delta);
                    if narrow_patch.is_edge(y * chunk_side + x) {
                        rim += 1;
                    }
                }
            }
        }
        println!(
            "water law: one chunk's own {chunk_side} × {chunk_side} corner read on its own against the \
             world's own {side} × {side} read of the same columns: {differ} of {} column(s) stand a different \
             water (worst {worst:.6} m), {rim} of them on the narrow read's own rim",
            chunk_side * chunk_side
        );
        assert!(
            differ > 0,
            "the declared water law no longer depends on the patch it is read on — {differ} of {} column(s) \
             differ — so a per-chunk read would be the world's own surface and the keyed door could read it",
            chunk_side * chunk_side
        );
    }

    /// **The store's own doors, measured** — the frame a residency fills ([`CubicWorld::bare`]), the keys,
    /// membership, a chunk put at the key it is, a chunk taken out, and the identity of the whole, the part
    /// and the empty: a residency that holds every chunk of a world has **that world's own identity**, and one
    /// that holds a part has neither the world's nor nothing's.
    #[test]
    fn the_stores_own_doors_move_cells_and_identity_together() {
        let source = source_over(2);
        let world = source.world();
        let keys = world.keys();
        let mut bare = world.bare();
        println!(
            "store: the world's own frame holding nothing: {} chunk(s), identity {}; the world: {} chunk(s), \
             identity {}",
            bare.chunks(),
            bare.digest().to_hex(),
            world.chunks(),
            world.digest().to_hex()
        );
        assert_eq!(bare.chunks(), 0, "a bare frame holds nothing");
        assert_eq!(bare.patch(), world.patch(), "and it is this world's own patch");
        assert_eq!(bare.span_z(), world.span_z());
        assert_eq!(bare.keys(), Vec::new());
        for key in &keys {
            assert!(!bare.holds(*key));
        }
        // Every chunk of the world, put in one at a time at the key it is: the whole, and the world's own
        // identity — which is the number a drawn frame's mesh is keyed by.
        for key in &keys {
            let replaced = bare
                .put(*key, source.chunk(*key).expect("a key the source names"))
                .expect("a chunk of its own key");
            assert!(replaced.is_none(), "the frame was empty");
            assert!(bare.holds(*key));
        }
        assert_eq!(bare.keys(), keys, "the keys are the world's own, in key order");
        assert_eq!(bare.chunks(), world.chunks());
        assert_eq!(bare.cells(), world.cells());
        assert_eq!(bare.digest(), world.digest(), "the whole residency is the world's own identity");
        // Taking one chunk out is a **different** identity, and the chunk taken out is the one that was there.
        let taken = bare.take(keys[0]).expect("a key it held");
        assert_eq!(&taken, world.chunk(keys[0]).expect("the world's own chunk"));
        assert!(!bare.holds(keys[0]));
        assert_ne!(bare.digest(), world.digest(), "a part is not the whole");
        // Putting it back restores the identity — the store is a set, not a history.
        bare.put(keys[0], taken).expect("the chunk's own key");
        assert_eq!(bare.digest(), world.digest());
    }

    /// **A chunk put under another key is refused by name** — the store's own invariant, and the guard that
    /// makes *the payload is the chunk its key names* a property of the store rather than of every caller.
    #[test]
    fn a_chunk_put_under_another_key_is_refused_by_name() {
        let source = source_over(2);
        let world = source.world();
        let mut bare = world.bare();
        let key = world.first_chunk().expect("the world has a chunk");
        let other = ChunkKey::new(key.x + 1, key.y, key.z);
        let chunk = source.chunk(key).expect("a key the source names");
        let refusal = bare.put(other, chunk).expect_err("a chunk under another key is refused");
        println!(
            "store: a chunk of its own key put under another: [{}] {}",
            refusal.kind().name(),
            refusal.detail()
        );
        assert_eq!(refusal.kind(), RefusalKind::ParameterOutOfRange);
        assert_eq!(bare.chunks(), 0, "and the store is left as it was");
    }
}
