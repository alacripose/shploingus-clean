//! **The measurement the port stands on** — the parent's mesher and this clean room's own derivation, on
//! the same world, the same chunk and the same level, compared **face by face**.
//!
//! The two meshes' digests are not comparable and are not compared: what *is* comparable — and what decides
//! whether the parent's derivation is the same geometry — is the set of unit voxel faces each mesh covers
//! ([`BlockFace`]), and that is what is accounted for, by named reason ([`Divergence`]).
//!
//! **What the differential found, and why nothing was replaced.** The two derivations agree on every unit
//! face whose both sides this world owns, and the whole of the rest of the divergence is
//! [`Divergence::AtTheWorldsOwnBound`] — faces the parent drew against a place the world never claimed,
//! which its own `ChunkNeighborhood` documentation calls a hole (*"the alternative is claiming air, which
//! would punch a halo of visible faces around every loaded chunk"*) and which this clean room's extent
//! contract calls an edge. The mesh digests cannot be reproduced either: the parent merges at subvoxel
//! resolution and this crate merges at block resolution, so one face set has two rectangle
//! decompositions (the measurement prints both faces/quad figures). A digest match would need the clean
//! room's own merge — the thing the port would be replacing — which is exactly why the gate is stated on
//! the **unit face set**. So the port's gate does not pass as *equality* and nothing was replaced; the
//! decision is recorded in `docs/baselines/port-appearance-seam/README.txt` §7.3 rather than assumed here.

use super::*;

/// **One unit voxel face** — the unit both meshers' rectangles are made of, named the way this clean room's
/// own geometry names a face, so two rectangles covering the same faces compare equal whatever their
/// extents. `u` and `v` are world coordinates on the two axes [`Axis::spans`] names for the face's axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BlockFace {
    pub layer: Layer,
    pub axis: Axis,
    pub facing: i8,
    pub plane: i32,
    pub u: i32,
    pub v: i32,
}

/// The clean room's own rectangles, expanded to the unit faces they stand for.
pub fn block_faces_of_mesh(mesh: &Mesh) -> BTreeSet<BlockFace> {
    let mut out = BTreeSet::new();
    for quad in mesh.quads() {
        for du in 0..quad.span_u as i32 {
            for dv in 0..quad.span_v as i32 {
                out.insert(BlockFace {
                    layer: quad.layer,
                    axis: quad.axis,
                    facing: quad.facing,
                    plane: quad.plane,
                    u: quad.u + du,
                    v: quad.v + dv,
                });
            }
        }
    }
    out
}

/// A parent rectangle expanded to the same unit faces, and whether every one of its corners landed on a
/// whole voxel. A subvoxel-quantised rectangle that is **not** voxel-aligned is this clean room's own
/// vocabulary's absence, not a rounding: it is counted and reported rather than rounded away.
fn parent_block_faces(
    quad: &ParentQuad,
    origin: VoxelPos,
    water: ParentMaterialId,
    out: &mut BTreeSet<BlockFace>,
) -> bool {
    let corners = quad.corners();
    let axis_index = quad.face.axis() as usize;
    let axis = [Axis::X, Axis::Y, Axis::Z][axis_index];
    let facing: i8 = if quad.face.is_positive() { 1 } else { -1 };
    let layer = if quad.material == water { Layer::Water } else { Layer::Solid };

    // The two axes the face spans, in this clean room's own order: the lower-numbered axis is `u`.
    let others: Vec<usize> = (0..3).filter(|a| *a != axis_index).collect();
    let (au, av) = (others[0], others[1]);

    // **Every corner on a whole voxel** — the quantisation check, over the three axes of each of the four
    // corners. It is what makes a subvoxel-quantised rectangle visible as such instead of rounded away.
    let aligned = corners
        .iter()
        .all(|corner| corner.iter().all(|at| at % SUBVOXELS_PER_VOXEL == 0));

    let low = |a: usize| corners.iter().map(|c| c[a]).min().unwrap_or(0);
    let high = |a: usize| corners.iter().map(|c| c[a]).max().unwrap_or(0);
    let to_voxel = |v: i32| v.div_euclid(SUBVOXELS_PER_VOXEL);
    let plane_axis = [origin.x, origin.y, origin.z][axis_index];
    let plane = plane_axis + to_voxel(low(axis_index));
    let u0 = [origin.x, origin.y, origin.z][au] + to_voxel(low(au));
    let v0 = [origin.x, origin.y, origin.z][av] + to_voxel(low(av));
    let su = to_voxel(high(au) - low(au));
    let sv = to_voxel(high(av) - low(av));
    for du in 0..su.max(0) {
        for dv in 0..sv.max(0) {
            out.insert(BlockFace { layer, axis, facing, plane, u: u0 + du, v: v0 + dv });
        }
    }
    aligned
}

/// **What lies outside the chunk being meshed** — the question the parent's mesher asks its neighbourhood,
/// and the one this clean room's world can answer in two different ways.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outside {
    /// **Ask the world, and refuse where it cannot answer.** The parent's own contract, verbatim. On a
    /// bounded patch no chunk passes this: every chunk has at least one side the world does not own.
    World,
    /// **Ask the world where it owns the place, and answer nothing where it does not.** This is what this
    /// clean room's own chunk door does with its one-ring halo (`MESH_RING`): it culls against the world's
    /// real neighbours and lets its own patch's edge be an edge. The queries the world could not answer are
    /// counted, so the assertion is disclosed rather than hidden.
    WorldOrNothing,
    /// **Never ask the world.** The parent's own `AllAir`: every boundary face is emitted, which its
    /// documentation calls correct for an isolated chunk — *"an isolated chunk really does have nothing on
    /// the other side"*.
    Nothing,
}

/// The two cells a unit face lies between: the one **showing** it and the one its `facing` points at.
///
/// [`Quad::plane`](crate::mesh::Quad) says the plane is the one **just outside** the cell that shows it (a
/// cell at `z` shows its `+z` face on the plane `z + 1`), so the showing cell is `plane - 1` when the face
/// looks the way the axis grows and `plane` when it looks the other way, and the far cell is the other of
/// the two. `None` for a coordinate that cannot be expressed at all (`i32::MIN` one step past the floor),
/// which is a place the world certainly does not own.
fn face_cells(face: &BlockFace) -> Option<(VoxelPos, VoxelPos)> {
    let low = face.plane.checked_sub(1)?;
    let axis = |at: i32| match face.axis {
        Axis::X => VoxelPos::new(at, face.u, face.v),
        Axis::Y => VoxelPos::new(face.u, at, face.v),
        Axis::Z => VoxelPos::new(face.u, face.v, at),
    };
    if face.facing > 0 {
        Some((axis(low), axis(face.plane)))
    } else {
        Some((axis(face.plane), axis(low)))
    }
}

/// One unit face, named the way the two derivations' own vocabulary does.
pub fn face_name(face: &BlockFace) -> String {
    format!(
        "{}/{}{} plane {} u {} v {}",
        face.layer.name(),
        if face.facing > 0 { "+" } else { "-" },
        face.axis.name(),
        face.plane,
        face.u,
        face.v,
    )
}

/// **Why one divergence is there** — three reasons the port knows of by construction, and `Unnamed` for
/// anything they do not cover. `Unnamed` must be zero, and that is what the gate asserts: every divergence
/// is *named*, so there is no residual wearing a reason's clothes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Divergence {
    /// The far side is a place **this world does not own**: outside its patch, or outside its own vertical
    /// span. The parent drew a boundary face against a place it was told was air, and this clean room's
    /// derivation culled it because its own extent contract makes the world's own bound an edge. A
    /// divergence about *what is beyond the world*, not about what the world contains.
    AtTheWorldsOwnBound,
    /// The far side **is** owned, it is in a **different chunk** from the one being meshed, and the run's own
    /// declared halo answered `AllAir` there ([`Outside::Nothing`]) — so the parent drew a boundary face
    /// against the air it was handed, and this clean room's derivation culled it against the neighbour the
    /// world really holds. A divergence about *the halo this run declared*, not about either derivation.
    TheDeclaredHaloSaidAir,
    /// One of the face's two cells **holds water**. The parent's matter is a 4×4×4 subvoxel occupancy set
    /// and this world's water is a level in whole micrometres, so how a wetted cell is handed over
    /// ([`Water`]) decides these faces. A divergence about *the declared mapping* between the two models.
    TheWaterLevelQuantised,
    /// Neither of the above: a disagreement about geometry both derivations can see, which is the thing the
    /// port's gate exists to catch. Must be zero.
    Unnamed,
}

impl Divergence {
    pub const fn name(self) -> &'static str {
        match self {
            Divergence::AtTheWorldsOwnBound => "the world's own bound",
            Divergence::TheDeclaredHaloSaidAir => "the declared halo said air",
            Divergence::TheWaterLevelQuantised => "the water level quantised",
            Divergence::Unnamed => "UNNAMED — a real disagreement",
        }
    }
}

/// The reason one divergence is there, decided from what the world says about both sides of the face and
/// from which halo the run declared.
fn reason(cube: &CubicWorld, chunk: ChunkKey, outside: Outside, face: &BlockFace) -> Divergence {
    let Some((showing, far)) = face_cells(face) else {
        return Divergence::AtTheWorldsOwnBound;
    };
    if cube.cell(far).is_err() {
        return Divergence::AtTheWorldsOwnBound;
    }
    if outside == Outside::Nothing && far.chunk() != chunk {
        return Divergence::TheDeclaredHaloSaidAir;
    }
    let wetted = |at: VoxelPos| cube.cell(at).map(|cell| cell.water_um > 0).unwrap_or(false);
    if face.layer == Layer::Water || wetted(showing) || wetted(far) {
        return Divergence::TheWaterLevelQuantised;
    }
    Divergence::Unnamed
}

/// **The parent's four declared horizons, as this port reads them** — `crates/voxel-stream/src/horizons.rs`,
/// `Horizons::declared()`, pinned by path + sha256 in
/// `docs/baselines/port-appearance-seam/pinned-parent-bytes.txt`. `(name, radius in chunks)`; a horizon's
/// extent is the **cube** `(2r + 1)³` of chunks, because [`HorizonRule::contains`] is a cube and not a
/// sphere (the parent's own reason: *"a sphere would make the boundary depend on the rounding of a square
/// root"*). Named here rather than depended upon, because a `voxel-stream` dependency would be the
/// generator's dependency too — which is the blocker this measurement exists to show.
///
/// [`HorizonRule::contains`]: the parent's `voxel_stream::horizons::HorizonRule::contains`
pub const PARENT_HORIZONS: [(&str, i32); 4] =
    [("render", 8), ("simulation", 12), ("residency", 24), ("field", 96)];

/// The chunks one of [`PARENT_HORIZONS`]' rows names around a centre: `(2r + 1)³`.
pub fn horizon_chunks(radius_chunks: i32) -> u64 {
    let side = 2 * radius_chunks as i64 + 1;
    (side * side * side) as u64
}

/// **The measurement the port stands on**: the parent's mesher and this clean room's own derivation, on the
/// same world, the same chunk and the same level, compared face by face.
///
/// The two meshes' **digests are not comparable** and are not compared: the parent's is over its own
/// subvoxel quads with its own canonical form, this crate's is over block faces with this crate's
/// (`shploingus.mesh/2`). What *is* comparable — and what decides whether the parent's derivation is the
/// same geometry — is the set of unit voxel faces each mesh covers, and that is what is compared.
#[derive(Clone, Debug)]
pub struct Differential {
    pub chunk: ChunkKey,
    pub outside: Outside,
    /// How a cell holding water was handed over for this run ([`Water`]).
    pub water: Water,
    /// Halo queries the world could not answer: the places this measurement treated as empty because the
    /// world does not own them.
    pub unanswered: u64,
    pub parent_solid_voxels: u64,
    pub parent_subvoxel_faces: u64,
    pub parent_quads: u64,
    pub parent_loops: u64,
    pub parent_unaligned_quads: u64,
    pub parent_faces_per_quad: f64,
    pub parent_block_faces: usize,
    pub clean_quads: u64,
    pub clean_faces: u64,
    pub clean_block_faces: usize,
    /// The faces only the parent drew, counted by reason and named by example.
    pub only_parent: Divergences,
    /// The faces only this clean room drew, counted by reason and named by example.
    pub only_clean: Divergences,
    pub only_parent_total: usize,
    pub only_clean_total: usize,
    /// Unit faces the parent covers whose far side this world owns.
    pub parent_faces_the_world_owns: usize,
}

/// **The divergence, counted by kind and named by example** — the whole accounting, so that "every
/// divergence named with measured numbers and the reason" is a table rather than an assertion. The counts
/// are exact; the examples are capped so a report stays readable.
#[derive(Clone, Debug, Default)]
pub struct Divergences {
    counts: BTreeMap<Divergence, usize>,
    examples: BTreeMap<Divergence, Vec<BlockFace>>,
}

/// How many examples of one kind a report names.
pub const EXAMPLE_CAP: usize = 8;

impl Divergences {
    fn add(&mut self, kind: Divergence, face: &BlockFace) {
        *self.counts.entry(kind).or_insert(0) += 1;
        let list = self.examples.entry(kind).or_default();
        if list.len() < EXAMPLE_CAP {
            list.push(*face);
        }
    }

    /// How many divergences of one kind.
    pub fn of(&self, kind: Divergence) -> usize {
        self.counts.get(&kind).copied().unwrap_or(0)
    }

    /// Every kind's count, in the order a reader needs them — `Unnamed` first, because it is the one that
    /// must be zero — then each kind's first up to [`EXAMPLE_CAP`] examples, named the way the geometry does.
    pub fn lines(&self) -> Vec<String> {
        let mut out = Vec::new();
        for kind in [
            Divergence::Unnamed,
            Divergence::AtTheWorldsOwnBound,
            Divergence::TheDeclaredHaloSaidAir,
            Divergence::TheWaterLevelQuantised,
        ] {
            let count = self.of(kind);
            if count == 0 {
                continue;
            }
            out.push(format!("    {}: {count}", kind.name()));
            for face in self.examples.get(&kind).into_iter().flatten() {
                out.push(format!("      {}", face_name(face)));
            }
        }
        out
    }

    /// **Every divergence is one of the three named kinds** — there is no `Unnamed` remainder.
    pub fn every_divergence_is_named(&self) -> bool {
        self.of(Divergence::Unnamed) == 0
    }

    /// The total this accounting covers; it must equal the difference between the two face sets.
    pub fn total(&self) -> usize {
        self.counts.values().sum()
    }
}

impl Differential {
    /// Whether the two derivations cover **exactly** the same unit faces: the claim a *replacement* would
    /// need, and one this bridge does not make.
    pub fn agrees(&self) -> bool {
        self.only_parent_total == 0 && self.only_clean_total == 0
    }

    /// **The port's gate, stated where it can bite:** every divergence between the parent's derivation and
    /// this crate's, on the same world and the same halo, is one of the three **named** reasons
    /// ([`Divergence`]) — the world's own bound, the halo this run declared, the water mapping. An
    /// `Unnamed` divergence is a disagreement about geometry both derivations can see, which is the only
    /// kind that could justify replacing one with the other; there must be none, and the accounting must be
    /// total (the counts must sum to the difference between the sets).
    pub fn the_port_gate_passes(&self) -> bool {
        self.only_parent.every_divergence_is_named()
            && self.only_clean.every_divergence_is_named()
            && self.only_parent.total() == self.only_parent_total
            && self.only_clean.total() == self.only_clean_total
    }

    /// The divergence, named by kind with its counts and its examples — one readable block.
    pub fn divergence_lines(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.only_parent_total > 0 {
            out.push(format!(
                "  the parent only ({} face(s)):",
                self.only_parent_total
            ));
            out.extend(self.only_parent.lines());
        }
        if self.only_clean_total > 0 {
            out.push(format!("  this clean room only ({} face(s)):", self.only_clean_total));
            out.extend(self.only_clean.lines());
        }
        out
    }

    /// One line, the whole measurement, for a test's own print (P4).
    pub fn line(&self) -> String {
        format!(
            "mesh port differential, chunk ({}, {}, {}), outside = {:?}, water handed over as {}: parent {} \
             solid voxel(s) — {} subvoxel face(s), {} quad(s) ({:.2} faces/quad), {} loop(s), {} unaligned, {} \
             block face(s) | clean room {} quad(s), {} face(s), {} block face(s) | agree = {} (parent only {}, \
             clean room only {}) | **the port's gate = {}** (unnamed parent-only {}, unnamed clean-room-only {}) \
             | halo queries the world could not answer: {}",
            self.chunk.x,
            self.chunk.y,
            self.chunk.z,
            self.outside,
            self.water.name(),
            self.parent_solid_voxels,
            self.parent_subvoxel_faces,
            self.parent_quads,
            self.parent_faces_per_quad,
            self.parent_loops,
            self.parent_unaligned_quads,
            self.parent_block_faces,
            self.clean_quads,
            self.clean_faces,
            self.clean_block_faces,
            self.agrees(),
            self.only_parent_total,
            self.only_clean_total,
            self.the_port_gate_passes(),
            self.only_parent.of(Divergence::Unnamed),
            self.only_clean.of(Divergence::Unnamed),
            self.unanswered,
        )
    }
}

/// Run both derivations on one world and one chunk, and return the comparison.
pub fn differential(
    cube: &CubicWorld,
    chunk: ChunkKey,
    ladder: Ladder,
    roster: &Declared,
    mapping: Water,
    outside: Outside,
) -> Result<Differential, Refusal> {
    let water = ladder.water();
    let level = Level::new(0)?;
    let residency = ResidencyHorizon::m(256.0)?;

    // ---- this crate's own derivation, through the cells door ----
    let mesher = Mesher::new(roster, ladder, residency);
    let mut resident = crate::mesh::Resident::new();
    let mut ledger = WorkLedger::default();
    let own: Mesh = mesher.derive_cubic(
        cube,
        chunk,
        level,
        Effort::new(Budget::unlimited(), 1),
        &mut resident,
        &mut ledger,
    )?;
    let clean_block_faces = block_faces_of_mesh(&own);

    // ---- the parent's derivation, through the parent's own two traits ----
    let origin = chunk.origin().map_err(|kind| {
        Refusal::new(
            kind,
            format!("chunk ({}, {}, {}) has no expressible origin", chunk.x, chunk.y, chunk.z),
        )
    })?;
    let pos = ChunkPos { x: chunk.x, y: chunk.y, z: chunk.z };
    let source = HandedOver { cube, water, mapping };
    let mut parent_chunk = ParentChunk::new(pos);
    parent_chunk.generate(&source).map_err(|refusal| {
        Refusal::new(
            RefusalKind::GeometryOutsideTheWorld,
            format!("the parent's own source refused the chunk: {}", refusal.detail()),
        )
    })?;
    // The face law: air plus exactly the one material this world draws through — water. Nothing else is
    // see-through, which is what makes a rock/rock contact cost nothing and a rock/water contact a face.
    let config = MeshConfig::air_only().with_transparent(handed_over(water));
    let halo = Halo::new(cube, pos, water, mapping, outside);
    let nothing = voxel_mesh::source::AllAir;
    let neighbours: &dyn ChunkNeighborhood = match outside {
        Outside::World | Outside::WorldOrNothing => &halo,
        Outside::Nothing => &nothing,
    };
    let mut parent_mesher = ParentMesher::new();
    let parent = parent_mesher
        .build(&parent_chunk, &config, Some(neighbours))
        .map_err(|refusal| {
            Refusal::new(
                RefusalKind::GeometryOutsideTheWorld,
                format!("the parent's own mesher refused the chunk: {}", refusal.detail()),
            )
        })?;

    let water_parent = handed_over(water);
    let mut parent_faces = BTreeSet::new();
    let mut unaligned = 0_u64;
    for quad in &parent.quads {
        if !parent_block_faces(quad, origin, water_parent, &mut parent_faces) {
            unaligned += 1;
        }
    }

    let only_parent_owned: Vec<BlockFace> =
        parent_faces.difference(&clean_block_faces).copied().collect();
    let only_clean_owned: Vec<BlockFace> =
        clean_block_faces.difference(&parent_faces).copied().collect();

    // **Every divergence is named by the reason it is there** ([`Divergence`]), so the accounting is a table
    // of named causes rather than a count of differences. `Unnamed` must be zero, and that is the gate.
    let mut only_parent = Divergences::default();
    for face in &only_parent_owned {
        only_parent.add(reason(cube, chunk, outside, face), face);
    }
    let mut only_clean = Divergences::default();
    for face in &only_clean_owned {
        only_clean.add(reason(cube, chunk, outside, face), face);
    }
    let parent_faces_the_world_owns = parent_faces
        .iter()
        .filter(|face| face_cells(face).map(|(_, far)| cube.cell(far).is_ok()).unwrap_or(false))
        .count();

    Ok(Differential {
        chunk,
        outside,
        water: mapping,
        unanswered: halo.unanswered.get(),
        parent_solid_voxels: parent.stats.solid_voxels,
        parent_subvoxel_faces: parent.stats.visible_faces,
        parent_quads: parent.stats.quads,
        parent_loops: parent.stats.loops,
        parent_unaligned_quads: unaligned,
        parent_faces_per_quad: parent.stats.faces_per_quad(),
        parent_block_faces: parent_faces.len(),
        clean_quads: own.quads().len() as u64,
        clean_faces: own.area(),
        clean_block_faces: clean_block_faces.len(),
        only_parent,
        only_clean,
        only_parent_total: only_parent_owned.len(),
        only_clean_total: only_clean_owned.len(),
        parent_faces_the_world_owns,
    })
}
