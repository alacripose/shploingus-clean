//! The greedy visible mesh, at subvoxel resolution.
//!
//! # Why 128³ and not 32³
//!
//! A voxel is a 4×4×4 grid, so a chunk's surface is the surface of a **128³**
//! grid. Meshing at 32³ and then drawing shapes *on top* would be two passes
//! that disagree at every boundary between them; this is one pass
//! ([`crate::lib`] says why that matters, and C-9 records it).
//!
//! # The algorithm
//!
//! 1. **Rasterise** the chunk to a flat 128³ array of [`MaterialId`], padded by
//!    four subvoxels on every side ([`Mesher::build`]). The padding is a whole
//!    **voxel** wide, not one subvoxel, so a neighbouring chunk's boundary voxel
//!    fits in it whole and no corner case has to be special-cased.
//! 2. For each of the six faces, and each of the 128 slices along that face's
//!    axis, build a 128×128 **mask** of the faces that are actually visible, and
//!    greedy-merge equal materials in it into rectangles.
//!
//! Step 2 is where uniform worlds become cheap without anyone writing a
//! fast path: a plane of one material merges to a single rectangle, however
//! large it is. The same code makes a stair's four subvoxel risers four separate
//! quads. There is no switch between those behaviours, and therefore no seam.
//!
//! # T-junctions
//!
//! Greedy merging to rectangles of different sizes produces corners where one
//! quad's edge ends part-way along its neighbour's — a **T-junction**. On a
//! rasteriser with enough precision it is invisible; at glancing angles and with
//! interpolation it can show as a hairline crack. The fix is vertex stitching
//! (splitting the long edge at the T), which is **not** in this mesh and is
//! recorded as a known limitation rather than papered over.
//!
//! # The winding, which nothing else will catch for you
//!
//! A quad emitted with the wrong corner order renders as a hole and type-checks
//! perfectly. So the order is derived once, in [`Quad::corners`], and *proved*
//! by test: the cross product of the quad's own edges must equal the face's
//! declared normal, for all six faces.

use std::collections::BTreeSet;

use voxel_model::{
    ChunkPos, Digest, Face, MaterialId, MaterialRegistry, OpticalClass, Refusal, RefusalKind,
    Refused, SubvoxelIndex, CHUNK_SIDE, SUBVOXEL_SIDE,
};

use crate::chunk::Chunk;
use crate::octree::{Node, SUBVOXEL_SIDE_I32};
use crate::source::ChunkNeighborhood;
use crate::SUBVOXEL_CHUNK_SIDE;

/// The raster's padding, in subvoxels. A whole voxel, deliberately.
pub const PAD: i32 = SUBVOXEL_SIDE_I32;

/// Subvoxels along a chunk edge.
pub const INTERIOR: usize = SUBVOXEL_CHUNK_SIDE as usize;

/// The raster's edge length, in subvoxels, including padding.
pub const FIELD_SIDE: usize = INTERIOR + 2 * PAD as usize;

/// Which the two off-axis coordinates `u` and `v` are, for a face on `axis`.
///
/// This choice is what makes the winding come out right, so it is a function
/// with a test rather than a convention in a comment.
const fn axes(axis: usize) -> (usize, usize) {
    match axis {
        // u = y, v = z  ⇒  u × v = +x
        0 => (1, 2),
        // u = x, v = z  ⇒  u × v = −y
        1 => (0, 2),
        // u = x, v = y  ⇒  u × v = +z
        _ => (0, 1),
    }
}

/// The raster index of a subvoxel coordinate. Coordinates are chunk-local
/// subvoxels, so `0..128` is inside the chunk and `-4..0` / `128..132` is pad.
fn index_of(c: [i32; 3]) -> usize {
    let x = (c[0] + PAD) as usize;
    let y = (c[1] + PAD) as usize;
    let z = (c[2] + PAD) as usize;
    x + FIELD_SIDE * (y + FIELD_SIDE * z)
}

/// The raster strides, in cells, of a face's two off-axis coordinates `u` and
/// `v`. Hoisted out of the mask loop so the loop multiplies by a constant
/// instead of recomputing a 3-D index twice per cell.
const fn field_strides(axis: usize) -> (usize, usize) {
    match axis {
        // u = y, v = z
        0 => (FIELD_SIDE, FIELD_SIDE * FIELD_SIDE),
        // u = x, v = z
        1 => (1, FIELD_SIDE * FIELD_SIDE),
        // u = x, v = y
        _ => (1, FIELD_SIDE),
    }
}

/// The position along `axis` is `slice`; `u` and `v` are the off-axis ones.
const fn cell(axis: usize, slice: i32, u: i32, v: i32) -> [i32; 3] {
    let (au, av) = axes(axis);
    let mut c = [0i32; 3];
    c[axis] = slice;
    c[au] = u;
    c[av] = v;
    c
}

/// The cubic region a leaf covers, as a subvoxel origin and a side.
fn leaf_region(origin: [i32; 3], level: u32) -> ([i32; 3], i32) {
    let voxels = 1i32 << level;
    (
        [origin[0] * SUBVOXEL_SIDE_I32, origin[1] * SUBVOXEL_SIDE_I32, origin[2] * SUBVOXEL_SIDE_I32],
        voxels * SUBVOXEL_SIDE_I32,
    )
}

/// Fill an axis-aligned cube of the raster.
fn fill_cube(field: &mut [MaterialId], origin: [i32; 3], side: i32, material: MaterialId) {
    for z in origin[2]..origin[2] + side {
        for y in origin[1]..origin[1] + side {
            let base = index_of([origin[0], y, z]);
            for slot in field[base..base + side as usize].iter_mut() {
                *slot = material;
            }
        }
    }
}

/// Which materials a face can be seen through.
///
/// The set comes from the **material registry**, not from a hardcoded id list,
/// because "is water see-through" is a property of water and not of the number
/// the mesher happens to have stored. [`MaterialId::EMPTY`] is always
/// transparent and is never in the set.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct MeshConfig {
    transparent: BTreeSet<MaterialId>,
}

impl MeshConfig {
    /// Air, and nothing else. The default, and a world where nothing but air
    /// is see-through is a perfectly good world.
    pub fn air_only() -> Self {
        Self::default()
    }

    /// Air plus every material the registry calls transmissive.
    pub fn from_registry(registry: &MaterialRegistry) -> Refused<Self> {
        let mut transparent = BTreeSet::new();
        for m in registry.materials() {
            if m.optical_class == OpticalClass::Transmissive {
                transparent.insert(m.id);
            }
        }
        if let Some(bad) = transparent.iter().find(|m| m.is_empty()) {
            return Err(Refusal::owned(
                RefusalKind::UnadmittedMaterial,
                format!("a transmissive material has the reserved empty id {bad:?}"),
            ));
        }
        Ok(MeshConfig { transparent })
    }

    /// Add one see-through material.
    pub fn with_transparent(mut self, material: MaterialId) -> Self {
        self.transparent.insert(material);
        self
    }

    pub fn is_transparent(&self, material: MaterialId) -> bool {
        material.is_empty() || self.transparent.contains(&material)
    }

    pub fn is_opaque(&self, material: MaterialId) -> bool {
        !self.is_transparent(material)
    }

    /// Whether the face between `a` and its neighbour `b` is drawn.
    ///
    /// The rule is `a` is not air, `b` differs from `a`, and `b` is not opaque.
    /// Every clause earns its place:
    ///
    /// * *`a` is not air* — air has no surface;
    /// * *`b` differs from `a`* — two cells of the same material have no face
    ///   between them, which is what makes solid rock cost nothing inside;
    /// * *`b` is not opaque* — this is the clause that stops a rock/soil contact
    ///   being drawn **from both sides** and z-fighting. The visible one is the
    ///   rock face where rock meets water, and the water surface where water
    ///   meets air — which is what a diver and a boat respectively see.
    pub fn face_is_visible(&self, a: MaterialId, b: MaterialId) -> bool {
        !a.is_empty() && a != b && !self.is_opaque(b)
    }
}

/// One merged, axis-aligned rectangle of a voxel face.
///
/// **Canonical**: this is the rectangle, not four corners. Two runs over the
/// same voxels produce byte-identical quads, which is what lets a mesh digest
/// mean something. Corners are derived, in [`Quad::corners`].
///
/// `slice` is the face **plane**'s position along the face axis, in subvoxels
/// from the chunk's low corner, so it is in `0..=128` — `0` is the chunk's low
/// boundary and `128` its high one.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Quad {
    pub face: Face,
    pub slice: u16,
    /// The rectangle's low corner in the face's own `(u, v)` basis.
    pub uv: [u16; 2],
    /// The rectangle's size in the face's own `(u, v)` basis.
    pub extent: [u16; 2],
    pub material: MaterialId,
}

/// One face the specification says must be drawn.
///
/// **A face is identified by its own cell and which way it faces**, not by the
/// plane it happens to land on: the plane is derived from the two, so storing it
/// would let the two disagree. `cell` is the subvoxel coordinate of the **solid**
/// cell — the one whose material is drawn — in chunk-local subvoxels.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct VisibleFace {
    pub cell: [i32; 3],
    pub face: Face,
    pub plane: u16,
    pub material: MaterialId,
}

impl VisibleFace {
    /// The single quad this face would be drawn as, with no merging.
    pub fn quad(&self) -> Quad {
        let axis = self.face.axis() as usize;
        let (au, av) = axes(axis);
        // The face's own `(u, v)` are the two coordinates that are *not* along
        // the axis and not the sign, which is exactly what `axes` names.
        let mut uv = [0i32; 2];
        let (mut i, mut j) = (0, 1);
        if au == axis {
            i = 1;
        }
        if av == axis {
            j = 1;
        }
        uv[i] = self.cell[au];
        uv[j] = self.cell[av];
        Quad {
            face: self.face,
            slice: self.plane,
            uv: [uv[0] as u16, uv[1] as u16],
            extent: [1, 1],
            material: self.material,
        }
    }
}

/// True when the base corner order has to be reversed to face outwards.
///
/// `axes()` was chosen so the base order `(+u, +u+v, +v)` has a normal of `+x`
/// on axis 0, `−y` on axis 1 and `+z` on axis 2. So axis 1 is the odd one out.
pub const fn winding_flips(face: Face) -> bool {
    face.is_positive() != (face.axis() != 1)
}

impl Quad {
    /// The four corners, in subvoxel coordinates local to the chunk's low
    /// corner, wound so that [`Quad::normal`] is the outward normal.
    pub fn corners(&self) -> [[i32; 3]; 4] {
        let axis = self.face.axis() as usize;
        let s = self.slice as i32;
        let (u0, v0) = (self.uv[0] as i32, self.uv[1] as i32);
        let (w, h) = (self.extent[0] as i32, self.extent[1] as i32);
        let base = [
            cell(axis, s, u0, v0),
            cell(axis, s, u0 + w, v0),
            cell(axis, s, u0 + w, v0 + h),
            cell(axis, s, u0, v0 + h),
        ];
        if winding_flips(self.face) {
            [base[3], base[2], base[1], base[0]]
        } else {
            base
        }
    }

    /// The outward normal, as an integer vector of unit length.
    pub fn normal(&self) -> [i32; 3] {
        let (x, y, z) = self.face.normal();
        [x, y, z]
    }

    /// The rectangle's area in subvoxels — how many faces merged into this one.
    pub fn area(&self) -> u32 {
        self.extent[0] as u32 * self.extent[1] as u32
    }
}

/// What the mesher did, so that "it merged" is a measurement and not a claim.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct MeshStats {
    /// Voxels holding matter, read from the octree.
    pub solid_voxels: u64,
    /// Subvoxel faces that are visible, before merging.
    pub visible_faces: u64,
    /// Quads actually emitted.
    pub quads: u64,
}

impl MeshStats {
    /// Visible faces per emitted quad. Zero when nothing was emitted — there is
    /// no ratio without a denominator, and inventing one would be a default.
    pub fn faces_per_quad(&self) -> f64 {
        if self.quads == 0 {
            0.0
        } else {
            self.visible_faces as f64 / self.quads as f64
        }
    }
}

/// A chunk's visible surface, in canonical quads.
///
/// Positions are **chunk-local**, in subvoxels. Converting to world metres is
/// [`Mesh::corner_positions_m`], which stays relative to the chunk origin on
/// purpose: at ±2 million metres an `f32` metre loses its subvoxel resolution,
/// so the chunk origin is applied per-chunk and never folded into a world-space
/// `f32`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Mesh {
    pub chunk: ChunkPos,
    pub quads: Vec<Quad>,
    pub stats: MeshStats,
}

impl Mesh {
    /// The chunk's low corner, in voxels.
    pub fn origin_voxel(&self) -> [i32; 3] {
        let o = self.chunk.origin();
        [o.x, o.y, o.z]
    }

    /// The four corners of a quad in metres from the chunk's low corner.
    pub fn corner_positions_m(&self, quad: &Quad) -> [[f32; 3]; 4] {
        let scale = 1.0 / SUBVOXEL_SIDE as f32;
        let mut out = [[0.0f32; 3]; 4];
        for (i, corner) in quad.corners().iter().enumerate() {
            out[i] = [
                corner[0] as f32 * scale,
                corner[1] as f32 * scale,
                corner[2] as f32 * scale,
            ];
        }
        out
    }

    /// Bytes this mesh holds.
    ///
    /// **Capacity, not length**, because the memory is what the allocator was
    /// asked for and not what is currently used — a `Vec` that grew to 4,096 and
    /// now holds 10 is holding 4,096 quads' worth of the machine.
    pub fn bytes(&self) -> usize {
        std::mem::size_of::<Self>() + self.quads.capacity() * std::mem::size_of::<Quad>()
    }

    /// A content digest over the emitted quads, in emission order.
    pub fn digest(&self) -> Digest {
        let mut t = Vec::with_capacity(self.quads.len() * 13 + 16);
        t.extend_from_slice(&self.chunk.x.to_le_bytes());
        t.extend_from_slice(&self.chunk.y.to_le_bytes());
        t.extend_from_slice(&self.chunk.z.to_le_bytes());
        t.extend_from_slice(&(self.quads.len() as u64).to_le_bytes());
        for q in &self.quads {
            t.push(face_index(q.face));
            t.extend_from_slice(&q.slice.to_le_bytes());
            t.extend_from_slice(&q.uv[0].to_le_bytes());
            t.extend_from_slice(&q.uv[1].to_le_bytes());
            t.extend_from_slice(&q.extent[0].to_le_bytes());
            t.extend_from_slice(&q.extent[1].to_le_bytes());
            t.extend_from_slice(&q.material.0.to_le_bytes());
        }
        Digest::of_fields("voxel-mesh/mesh", &[b"v1", t.as_slice()])
    }
}

const fn face_index(face: Face) -> u8 {
    match face {
        Face::NegX => 0,
        Face::PosX => 1,
        Face::NegY => 2,
        Face::PosY => 3,
        Face::NegZ => 4,
        Face::PosZ => 5,
    }
}

/// The mesher, and its scratch space.
///
/// The raster is `136³` [`MaterialId`]s — about 5 MB — and it is **held**, not
/// allocated per chunk, because a mesher that allocates 5 MB per chunk is a
/// mesher that spends its time in the allocator. One `Mesher` per worker; the
/// scratch is never shared, which is why the mesher needs no locking and why
/// worker count is a performance knob and never a change in output (F12).
pub struct Mesher {
    field: Vec<MaterialId>,
    mask: Vec<MaterialId>,
}

impl Default for Mesher {
    fn default() -> Self {
        Self::new()
    }
}

impl Mesher {
    pub fn new() -> Self {
        Mesher {
            field: vec![MaterialId::EMPTY; FIELD_SIDE * FIELD_SIDE * FIELD_SIDE],
            mask: vec![MaterialId::EMPTY; INTERIOR * INTERIOR],
        }
    }

    /// How many bytes of scratch one mesher holds. Stated so a caller planning
    /// N worker threads can multiply.
    pub fn scratch_bytes(&self) -> usize {
        (self.field.len() + self.mask.len()) * core::mem::size_of::<MaterialId>()
    }

    /// Mesh a chunk.
    ///
    /// `neighbors` is optional: without one every boundary face is emitted,
    /// which is the correct answer for an isolated chunk. [`AllAir`] says the
    /// same thing explicitly.
    pub fn build(
        &mut self,
        chunk: &Chunk,
        config: &MeshConfig,
        neighbors: Option<&dyn ChunkNeighborhood>,
    ) -> Refused<Mesh> {
        self.raster(chunk, neighbors)?;

        let side = SUBVOXEL_CHUNK_SIDE as usize;
        let mut quads: Vec<Quad> = Vec::new();
        let mut stats = MeshStats {
            solid_voxels: chunk.tree().solid_count(),
            visible_faces: 0,
            quads: 0,
        };

        // **The uniform fast path.** If every cell of the padded raster holds
        // the same material then no cell differs from its neighbour, and the
        // face rule requires them to differ — so there is not one face to find.
        // This is not a special case bolted on: it is the face rule's own
        // consequence, and it is what makes a sky chunk or a buried chunk cost a
        // raster fill instead of 12.6 million mask cells. **Measured:** most of
        // a vertical stack of chunks is sky or deep rock, so most of the work a
        // naive mesher would do is work that cannot produce a pixel.
        if !self.field_is_uniform() {
            for face in Face::ALL {
                let axis = face.axis() as usize;
                let positive = face.is_positive();
                let step: i32 = if positive { 1 } else { -1 };
                let (su, sv) = field_strides(axis);

                for slice in 0..SUBVOXEL_CHUNK_SIDE {
                    // The two planes, pre-addressed once per slice.
                    let base_a = index_of(cell(axis, slice, 0, 0));
                    let base_b = index_of(cell(axis, slice + step, 0, 0));
                    let mut any = false;
                    let mut slot = 0usize;
                    for v in 0..side {
                        let (row_a, row_b) = (base_a + v * sv, base_b + v * sv);
                        for u in 0..side {
                            let a = self.field[row_a + u * su];
                            let b = self.field[row_b + u * su];
                            let m = if config.face_is_visible(a, b) {
                                a
                            } else {
                                MaterialId::EMPTY
                            };
                            self.mask[slot] = m;
                            slot += 1;
                            if !m.is_empty() {
                                any = true;
                            }
                        }
                    }
                    if !any {
                        continue;
                    }
                    // The plane: a negative face lies on the cell's own low
                    // plane, a positive face on its high one.
                    let plane = (if positive { slice + 1 } else { slice }) as u16;
                    self.merge(face, plane, &mut quads, &mut stats);
                }
            }
        }

        stats.quads = quads.len() as u64;
        Ok(Mesh { chunk: chunk.pos, quads, stats })
    }

    /// Build the same mesh **without merging**: one quad per visible face.
    ///
    /// The reference implementation, and the definition of correct. It is slower and it
    /// is what the greedy mesher is measured against, in coverage and in cost. It walks
    /// [`Mesher::visible_faces`], so the two cannot disagree about which faces exist:
    /// what the coverage test checks is that **merging** them loses nothing, which is
    /// the only claim the greedy mesher makes.
    ///
    /// **It is a test oracle, not a production path**, and it is deliberately much
    /// slower — one quad per face is what a 128³ raster produces on its own. It is
    /// `#[cfg(test)]`-adjacent in spirit and only ever called by the coverage test.
    pub fn build_naive(
        &mut self,
        chunk: &Chunk,
        config: &MeshConfig,
        neighbors: Option<&dyn ChunkNeighborhood>,
    ) -> Refused<Mesh> {
        // **The same neighbours as the greedy path.** A pad built from `None`
        // and a pad built from `AllAir` are the same only while every fixture's
        // neighbours happen to be air; the comparison would then pass for the
        // wrong reason and fail on the first world with a real neighbour.
        self.raster(chunk, neighbors)?;
        let faces = self.visible_faces(config);
        let quads: Vec<Quad> = faces.iter().map(VisibleFace::quad).collect();
        let visible_faces = quads.len() as u64;
        Ok(Mesh {
            chunk: chunk.pos,
            stats: MeshStats { solid_voxels: chunk.tree().solid_count(), visible_faces, quads: visible_faces },
            quads,
        })
    }

    /// **The specification: every face the mesher is supposed to draw.**
    ///
    /// A triple loop over the chunk's subvoxels, reading the same padded raster
    /// and applying the same [`MeshConfig::face_is_visible`] rule the greedy path
    /// applies — but with **no merging and no slicing**. That is the whole point:
    /// this is the *definition of correct*, and the greedy pass is the thing
    /// measured against it.
    ///
    /// **Why this exists at all.** The greedy mesher's mask is built slice by
    /// slice along each axis, which is a completely different traversal from
    /// this one, so the two can only be trusted if something checks that they
    /// agree. A mesher that drops a face does so **silently**: the world simply
    /// has a hole in it and every count still looks right. The reference tree's
    /// `the_greedy_mesh_covers_exactly_the_faces_the_naive_mesher_draws` is the
    /// test that catches it, and this is the specification that test needs.
    ///
    /// **The set, not a count.** A `BTreeSet` so a face appears once however the
    /// loops reached it, and in a total order, so a mesh is reproducible from the
    /// set alone.
    ///
    /// **It walks the field directly, not the mask**, so it stays honest if the
    /// mask indexing ever changes: a specification written in terms of the code
    /// under test would agree with that code's bugs.
    pub fn visible_faces(&self, config: &MeshConfig) -> BTreeSet<VisibleFace> {
        let mut faces = BTreeSet::new();
        for face in Face::ALL {
            let axis = face.axis() as usize;
            let step: i32 = if face.is_positive() { 1 } else { -1 };
            // The face's own plane: negative faces sit on the cell's low plane,
            // positive ones on its high plane. This is the same convention
            // `build` uses to name a quad's slice.
            for slice in 0..SUBVOXEL_CHUNK_SIDE {
                let plane = if face.is_positive() { slice + 1 } else { slice };
                for v in 0..SUBVOXEL_CHUNK_SIDE {
                    for u in 0..SUBVOXEL_CHUNK_SIDE {
                        let here = cell(axis, slice, u, v);
                        let next = cell(axis, slice + step, u, v);
                        let a = self.field[index_of(here)];
                        let b = self.field[index_of(next)];
                        if config.face_is_visible(a, b) {
                            faces.insert(VisibleFace {
                                cell: here,
                                face,
                                plane: plane as u16,
                                material: a,
                            });
                        }
                    }
                }
            }
        }
        faces
    }

    /// Whether every cell of the padded raster holds the same material.
    ///
    /// Short-circuits on the first difference, so a field that is *not* uniform
    /// costs almost nothing; a field that is uniform costs one pass, which is
    /// still far less than one mask slice.
    fn field_is_uniform(&self) -> bool {
        match self.field.first() {
            None => true,
            Some(first) => self.field.iter().all(|m| m == first),
        }
    }

    /// Rasterise the chunk and one voxel of its neighbours into [`Self::field`].
    fn raster(&mut self, chunk: &Chunk, neighbors: Option<&dyn ChunkNeighborhood>) -> Refused<()> {
        self.field.fill(MaterialId::EMPTY);

        // The chunk itself, one leaf at a time. A `Uniform` leaf fills a whole
        // cube in one go, which is why a solid chunk rasterises in a handful of
        // writes rather than 2.1 million.
        {
            let field = &mut self.field;
            chunk.tree().visit_leaves(&mut |origin, level, node| {
                let (base, side) = leaf_region(origin, level);
                match node {
                    Node::Empty => {}
                    Node::Uniform(m) => fill_cube(field, base, side, *m),
                    Node::Mixed(shape) => {
                        for at in SubvoxelIndex::all() {
                            let m = shape.get(at);
                            if m.is_empty() {
                                continue;
                            }
                            field[index_of([
                                base[0] + at.x as i32,
                                base[1] + at.y as i32,
                                base[2] + at.z as i32,
                            ])] = m;
                        }
                    }
                    Node::Branch(_) => {}
                }
            });
        }

        // One voxel of neighbour, on each of the six faces. Only the layer of
        // the neighbour adjacent to the boundary is written, because that is
        // the only part of it the mask can read: the mask's `u` and `v` are
        // always interior.
        if let Some(neighbors) = neighbors {
            for axis in 0..3usize {
                for side_world in [-1i32, CHUNK_SIDE] {
                    for a in 0..CHUNK_SIDE {
                        for b in 0..CHUNK_SIDE {
                            let mut local = [0i32; 3];
                            local[axis] = side_world;
                            local[(axis + 1) % 3] = a;
                            local[(axis + 2) % 3] = b;
                            let shape = neighbors.shape(local)?;
                            if shape.is_empty() {
                                continue;
                            }
                            // The subvoxel of the neighbour touching the
                            // boundary: index 3 on the low side, 0 on the high.
                            let layer = if side_world < 0 { SUBVOXEL_SIDE_I32 - 1 } else { 0 };
                            let other = axes(axis);
                            for u in 0..SUBVOXEL_SIDE_I32 {
                                for v in 0..SUBVOXEL_SIDE_I32 {
                                    let mut at = [0u8; 3];
                                    at[axis] = layer as u8;
                                    at[other.0] = u as u8;
                                    at[other.1] = v as u8;
                                    let Ok(index) = SubvoxelIndex::new(at[0], at[1], at[2]) else {
                                        continue;
                                    };
                                    let m = shape.get(index);
                                    if m.is_empty() {
                                        continue;
                                    }
                                    let mut c = [0i32; 3];
                                    c[axis] = local[axis] * SUBVOXEL_SIDE_I32 + layer;
                                    c[other.0] = local[other.0] * SUBVOXEL_SIDE_I32 + u;
                                    c[other.1] = local[other.1] * SUBVOXEL_SIDE_I32 + v;
                                    self.field[index_of(c)] = m;
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Greedy-merge the current mask into rectangles and clear it.
    fn merge(&mut self, face: Face, plane: u16, out: &mut Vec<Quad>, stats: &mut MeshStats) {
        let mask = &mut self.mask;
        for v in 0..SUBVOXEL_CHUNK_SIDE {
            for u in 0..SUBVOXEL_CHUNK_SIDE {
                let material = mask[u as usize + v as usize * INTERIOR];
                if material.is_empty() {
                    continue;
                }

                // Grow along u.
                let mut w = 1i32;
                while u + w < SUBVOXEL_CHUNK_SIDE
                    && mask[(u + w) as usize + v as usize * INTERIOR] == material
                {
                    w += 1;
                }

                // Grow along v, but only over rows that are entirely this
                // material for the full width — a rectangle, not a comb.
                let mut h = 1i32;
                while v + h < SUBVOXEL_CHUNK_SIDE {
                    let row = (v + h) as usize * INTERIOR + u as usize;
                    let whole_row = mask[row..row + w as usize].iter().all(|m| *m == material);
                    if !whole_row {
                        break;
                    }
                    h += 1;
                }

                stats.visible_faces += (w * h) as u64;
                for dv in 0..h as usize {
                    let row = (v as usize + dv) * INTERIOR + u as usize;
                    for slot in mask[row..row + w as usize].iter_mut() {
                        *slot = MaterialId::EMPTY;
                    }
                }

                out.push(Quad {
                    face,
                    slice: plane,
                    uv: [u as u16, v as u16],
                    extent: [w as u16, h as u16],
                    material,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use crate::octree::local_from;
    use crate::source::AllAir;
    use voxel_model::{Component, ComponentId, Ratio, VoxelShape};

    const STONE: MaterialId = MaterialId(6);
    const SOIL: MaterialId = MaterialId(3);
    const WATER: MaterialId = MaterialId(1);

    fn chunk_with(voxels: &[([i32; 3], VoxelShape)], pos: ChunkPos) -> Chunk {
        let mut c = Chunk::new(pos);
        for (local, shape) in voxels {
            c.set_voxel(local_from(*local).unwrap(), *shape);
        }
        c
    }

    fn one_voxel(shape: VoxelShape) -> Chunk {
        chunk_with(&[([1, 1, 1], shape)], ChunkPos::new(0, 0, 0))
    }

    /// A slab of rows whose **widths differ**, which is the shape that breaks a
    /// merge that grows along `u` first and then hopes the result is a rectangle.
    ///
    /// Each row is one voxel taller or shorter than the one below, so every
    /// candidate rectangle has to be checked against a full row — the case the
    /// greedy code's `whole_row` test exists for, and the case a subtle off-by-one
    /// in that test would silently lose.
    fn jagged() -> (Chunk, MeshConfig) {
        let mut c = Chunk::new(ChunkPos::new(0, 0, 0));
        for x in 0..16i32 {
            // A staircase: the height depends on x, so every column is a
            // different height and no two rows share a width.
            let height = 2 + (x % 5) + (x % 3);
            for y in 0..height {
                for z in 0..height {
                    c.set_voxel(local_from([x, y, z]).unwrap(), VoxelShape::solid(STONE));
                }
            }
        }
        (c, MeshConfig::air_only())
    }

    /// The worlds the coverage invariant is checked over.
    ///
    /// **Each one exercises a different branch of the face rule**, because a guard
    /// checked against one shape is a guard checked against one shape: uniform
    /// (no faces at all), a lone voxel (six faces), an L (concave corners), water
    /// against rock (the `b is not opaque` clause, which is the one that stops a
    /// contact being drawn from both sides), and **negative chunk coordinates**
    /// (where a truncating index would drop a whole face).
    fn coverage_worlds() -> Vec<(&'static str, Chunk, MeshConfig)> {
        // `MeshConfig` is not `Copy` and carries a registry, so each world gets
        // its own rather than sharing one — which is also what a real caller
        // does, and what keeps this fixture honest about the type.
        let air = || MeshConfig::air_only();

        // A solid block, so two materials meet.
        let mixed = {
            let mut c = Chunk::new(ChunkPos::new(0, 0, 0));
            c.set_voxel(local_from([2, 2, 2]).unwrap(), VoxelShape::solid(STONE));
            c.set_voxel(local_from([3, 2, 2]).unwrap(), VoxelShape::solid(SOIL));
            c
        };

        // Water against rock: exercises the opaque clause from both sides.
        let wet = {
            let mut c = Chunk::new(ChunkPos::new(0, 0, 0));
            for x in 0..4 {
                for y in 0..4 {
                    for z in 0..4 {
                        c.set_voxel(
                            local_from([x, y, z]).unwrap(),
                            VoxelShape::solid(if z == 0 { STONE } else { WATER }),
                        );
                    }
                }
            }
            c
        };

        // Negative coordinates, at a chunk that is not the origin.
        let negative = {
            let mut c = Chunk::new(ChunkPos::new(-2, -1, -3));
            c.set_voxel(local_from([0, 0, 0]).unwrap(), VoxelShape::solid(STONE));
            c.set_voxel(local_from([5, 7, 9]).unwrap(), VoxelShape::solid(SOIL));
            c
        };

        // An L, for concave corners where a merge must not span a gap.
        let ell = {
            let mut c = Chunk::new(ChunkPos::new(0, 0, 0));
            for i in 0..6i32 {
                c.set_voxel(local_from([i, 0, 0]).unwrap(), VoxelShape::solid(STONE));
                c.set_voxel(local_from([0, i, 0]).unwrap(), VoxelShape::solid(STONE));
            }
            c
        };

        vec![
            ("uniform rock", {
                let mut c = Chunk::new(ChunkPos::new(0, 0, 0));
                for x in 0..8 { for y in 0..8 { for z in 0..8 {
                    c.set_voxel(local_from([x, y, z]).unwrap(), VoxelShape::solid(STONE));
                }}}
                c
            }, air()),
            ("empty", Chunk::new(ChunkPos::new(0, 0, 0)), air()),
            ("a lone voxel", one_voxel(VoxelShape::solid(STONE)), air()),
            ("two materials", mixed, air()),
            ("water against rock", wet, air()),
            ("negative chunk", negative, air()),
            ("an L", ell, air()),
        ]
    }

    fn mesh(chunk: &Chunk) -> Mesh {
        Mesher::new()
            .build(chunk, &MeshConfig::air_only(), Some(&AllAir))
            .unwrap()
    }

    /// Every subvoxel cell the greedy merge claims to draw, with how many times.
    ///
    /// A face covered **twice** is as much a bug as one covered zero times: two
    /// overlapping quads z-fight, which looks like the flicker or the seam rather
    /// than the hole a missing face gives. Both are checked.
    fn covered_faces(mesh: &Mesh) -> BTreeMap<([i32; 3], Face), usize> {
        let mut covered: BTreeMap<([i32; 3], Face), usize> = BTreeMap::new();
        for quad in &mesh.quads {
            let axis = quad.face.axis() as usize;
            let (au, av) = axes(axis);
            for dv in 0..quad.extent[1] as i32 {
                for du in 0..quad.extent[0] as i32 {
                    let mut c = [0i32; 3];
                    c[axis] = quad.slice as i32;
                    c[au] = quad.uv[0] as i32 + du;
                    c[av] = quad.uv[1] as i32 + dv;
                    // A face sits on a *plane*, and a cell on the positive side of
                    // it is the solid one. Both conventions are collapsed onto the
                    // low cell so a positive and a negative face of the same plane
                    // cannot count as two different faces.
                    let solid = if quad.face.is_positive() { c[axis] - 1 } else { c[axis] };
                    c[axis] = solid;
                    *covered.entry((c, quad.face)).or_insert(0) += 1;
                }
            }
        }
        covered
    }

    /// The specification's set, keyed the same way so the two can be compared.
    ///
    /// **It rasterises the chunk first.** `visible_faces` reads `self.field`, and
    /// a `Mesher::new()` has an empty one — so asking a fresh mesher what it can
    /// see answers *nothing can be seen*, which is a specification that agrees
    /// with nothing. The first version of this helper did exactly that and
    /// reported every real face as `invented`.
    fn wanted_faces(config: &MeshConfig, chunk: &Chunk) -> BTreeSet<([i32; 3], Face)> {
        let mut mesher = Mesher::new();
        mesher.raster(chunk, Some(&AllAir)).expect("the raster refuses nothing here");
        mesher
            .visible_faces(config)
            .into_iter()
            .map(|f| (f.cell, f.face))
            .collect()
    }

    /// A short, countable tail of a long mismatch list.
    ///
    /// A coverage failure over a 128³ raster can name tens of thousands of faces,
    /// and a message that long is unreadable and floods the test log. Five is
    /// enough to see the shape; the count is the actual information.
    fn sample<T: std::fmt::Debug>(label: &str, items: &[T]) -> String {
        let shown = items.len().min(5);
        format!("{}: {} total, first {} {:?}", label, items.len(), shown, &items[..shown])
    }

    #[test]
    fn the_greedy_mesh_covers_exactly_the_faces_the_naive_mesher_draws() {
        // **The load-bearing assertion.** A mesher that drops a face does it
        // *silently*: the world gets a hole in it and every count still looks
        // right. This diffs the greedy mesh against one-quad-per-face built from
        // the same face rule, over worlds chosen to exercise every branch —
        // uniform, a lone voxel, an L, a water body against rock, and negative
        // chunk coordinates.
        for (name, chunk, config) in coverage_worlds() {
            let mut greedy = Mesher::new();
            let built = greedy.build(&chunk, &config, Some(&AllAir)).unwrap();

            let mut naive_mesher = Mesher::new();
            let naive = naive_mesher.build_naive(&chunk, &config, Some(&AllAir)).unwrap();

            let wanted = wanted_faces(&config, &chunk);
            assert_eq!(
                built.stats.visible_faces, naive.stats.visible_faces,
                "{name}: the greedy pass counted a different number of visible faces than \
                 the naive one, so the two are not talking about the same thing"
            );

            let covered = covered_faces(&built);
            let overlaps: Vec<_> = covered
                .iter()
                .filter(|(_, times)| **times != 1)
                .map(|(face, times)| (*face, *times))
                .collect();
            assert!(
                overlaps.is_empty(),
                "{name}: {overlaps:?} are covered more than once, so two quads overlap and \
                 the region z-fights where it should not"
            );

            let got: BTreeSet<([i32; 3], Face)> = covered.keys().copied().collect();
            let missing: Vec<_> = wanted.difference(&got).copied().collect();
            let extra: Vec<_> = got.difference(&wanted).copied().collect();
            assert!(
                missing.is_empty() && extra.is_empty(),
                "{name}: the greedy mesh is not the naive mesh — {}, {}",
                sample("missing", &missing),
                sample("invented", &extra)
            );
        }
    }

    #[test]
    fn no_greedy_quad_covers_a_face_twice_on_a_jagged_mask() {
        // The same invariant on the one shape built to break rectangle
        // extraction: rows of **different lengths**, so every merge candidate has
        // to be tested against a whole rectangle rather than a run.
        let (chunk, config) = jagged();
        let mut mesher = Mesher::new();
        let built = mesher.build(&chunk, &config, Some(&AllAir)).unwrap();
        let covered = covered_faces(&built);
        let overlaps: Vec<_> = covered
            .iter()
            .filter(|(_, times)| **times != 1)
            .map(|(face, times)| (*face, *times))
            .collect();
        assert!(overlaps.is_empty(), "{overlaps:?} are covered more than once");
        assert!(
            covered.len() > 400,
            "the fixture has to be jagged enough to matter, and it only produced {} faces",
            covered.len()
        );

        let wanted = wanted_faces(&config, &chunk);
        let got: BTreeSet<_> = covered.keys().copied().collect();
        assert_eq!(got, wanted, "the jagged fixture is where a merge loses a face");
    }

    #[test]
    fn the_coverage_guard_has_teeth_and_would_catch_a_mesher_that_drops_a_face() {
        // **A guard that cannot fail is not a guard.** So: take a real mesh,
        // delete one quad from it, and assert the comparison *notices*. Without
        // this, a broken specification that returned the greedy mesh's own
        // answer would pass forever.
        let chunk = one_voxel(VoxelShape::solid(STONE));
        let config = MeshConfig::air_only();
        let mut mesher = Mesher::new();
        let built = mesher.build(&chunk, &config, Some(&AllAir)).unwrap();
        assert_eq!(built.quads.len(), 6, "a lone voxel is six quads");
        assert_eq!(built.stats.visible_faces, 6 * 16, "each is 4×4 subvoxel faces");

        let wanted = wanted_faces(&config, &chunk);
        assert_eq!(
            covered_faces(&built).len(),
            wanted.len(),
            "the honest mesh agrees with the specification"
        );

        // Now lose one face the way a real bug loses one: the last quad goes.
        let mut broken = built.clone();
        broken.quads.pop();
        let got: BTreeSet<([i32; 3], Face)> = covered_faces(&broken).keys().copied().collect();
        let missing: Vec<_> = wanted.difference(&got).copied().collect();
        assert_eq!(missing.len(), 16, "a dropped quad is 16 missing faces, and the diff names them");
    }

    #[test]
    fn a_lone_voxel_emits_exactly_six_quads_one_per_face() {
        let m = mesh(&one_voxel(VoxelShape::solid(STONE)));
        assert_eq!(m.quads.len(), 6, "quads: {:?}", m.quads);
        assert_eq!(m.stats.solid_voxels, 1);
        assert_eq!(m.stats.visible_faces, 6 * 16);

        let mut seen = [false; 6];
        for q in &m.quads {
            seen[face_index(q.face) as usize] = true;
            assert_eq!(q.material, STONE);
            assert_eq!(q.extent, [4, 4]);
            // `uv` is in **chunk-local** subvoxels, so a voxel at voxel 1 starts
            // at subvoxel 4. It is not relative to the rectangle — a rectangle
            // is its origin and its size in the chunk's own frame, which is
            // what makes the quad canonical.
            assert_eq!(q.uv, [4, 4]);
        }
        assert!(seen.iter().all(|s| *s), "a face was missed");
    }

    #[test]
    fn the_plane_of_each_lone_voxel_face_is_where_it_should_be() {
        // The voxel is at (1, 1, 1), so subvoxels 4..8. A negative face lies on
        // the voxel's low plane and a positive one on its high plane.
        let m = mesh(&one_voxel(VoxelShape::solid(STONE)));
        for q in &m.quads {
            let expected = if q.face.is_positive() { 8 } else { 4 };
            assert_eq!(q.slice, expected, "wrong plane for {:?}", q.face);
        }
    }

    #[test]
    fn the_winding_of_every_face_points_outwards() {
        // Nothing else catches this. A quad with reversed corners type-checks,
        // digests consistently, and renders as a hole.
        let m = mesh(&one_voxel(VoxelShape::solid(STONE)));
        for q in &m.quads {
            let c = q.corners();
            let e1 = [c[1][0] - c[0][0], c[1][1] - c[0][1], c[1][2] - c[0][2]];
            let e2 = [c[3][0] - c[0][0], c[3][1] - c[0][1], c[3][2] - c[0][2]];
            let cross = [
                e1[1] * e2[2] - e1[2] * e2[1],
                e1[2] * e2[0] - e1[0] * e2[2],
                e1[0] * e2[1] - e1[1] * e2[0],
            ];
            let n = q.normal();
            let dot = cross[0] * n[0] + cross[1] * n[1] + cross[2] * n[2];
            assert!(
                dot > 0,
                "face {:?} winds inwards: cross {cross:?}, normal {n:?}",
                q.face
            );
            // And the corners must actually be a rectangle of the right size.
            assert_eq!(cross[0].abs() + cross[1].abs() + cross[2].abs(), 16);
        }
    }

    #[test]
    fn a_solid_slab_merges_to_six_quads() {
        // The measurement that proves the mesher merges at all. A 32×32×1 slab
        // is 1,024 voxels and 65,536 subvoxels, and it becomes six rectangles —
        // and the same code, on a mixed voxel, does not merge what it must not.
        // (Corrected 2026-09-29: this comment used to read "32,768 voxels and
        // 2.1 M subvoxels", which is a full *chunk*'s count — 32³ voxels. The
        // slab's own assertions below have always said 32 * 32. See LEDGER C-51.)
        let mut voxels = Vec::new();
        for x in 0..CHUNK_SIDE {
            for y in 0..CHUNK_SIDE {
                voxels.push(([x, y, 0], VoxelShape::solid(STONE)));
            }
        }
        let c = chunk_with(&voxels, ChunkPos::new(0, 0, 0));
        let m = mesh(&c);

        assert_eq!(m.stats.solid_voxels, 32 * 32);
        // 128×128 on the two z faces, 128×4 on each of the four others.
        assert_eq!(m.stats.visible_faces, 2 * 128 * 128 + 4 * 128 * 4);
        assert_eq!(m.quads.len(), 6, "quads: {:?}", m.quads);
        assert!(m.stats.faces_per_quad() > 5000.0);
        for q in &m.quads {
            assert_eq!(q.material, STONE);
        }
    }

    #[test]
    fn a_half_soil_voxel_shows_its_subvoxel_detail() {
        // Bottom two subvoxel layers stone, top two soil: the case a block-id
        // renderer physically cannot draw, and the reason meshing is at 128³.
        let mut shape = VoxelShape::empty();
        for at in SubvoxelIndex::all() {
            shape.set(at, if at.z < 2 { STONE } else { SOIL });
        }
        let m = mesh(&one_voxel(shape));

        // A solid voxel meshes to six quads. This one meshes to ten: the four
        // faces whose own basis includes the z axis split in two, because the
        // material changes half-way up them. The two z faces lie entirely in
        // one material each and do not split.
        assert_eq!(m.quads.len(), 10, "quads: {:?}", m.quads);
        let split: Vec<&Quad> = m.quads.iter().filter(|q| q.extent == [4, 2]).collect();
        assert_eq!(split.len(), 8, "four faces should each split in two");
        assert_eq!(split.iter().filter(|q| q.material == STONE).count(), 4);
        assert_eq!(split.iter().filter(|q| q.material == SOIL).count(), 4);

        let tops: Vec<&Quad> = m.quads.iter().filter(|q| q.face == Face::PosZ).collect();
        assert_eq!(tops.len(), 1, "the +z face is all soil and must not split");
        assert_eq!(tops[0].material, SOIL);
        assert_eq!(tops[0].extent, [4, 4]);
        let bottoms: Vec<&Quad> = m.quads.iter().filter(|q| q.face == Face::NegZ).collect();
        assert_eq!(bottoms.len(), 1, "the -z face is all stone and must not split");
        assert_eq!(bottoms[0].material, STONE);
    }

    fn solid_chunk() -> Chunk {
        let mut c = Chunk::new(ChunkPos::new(0, 0, 0));
        for z in 0..CHUNK_SIDE {
            for y in 0..CHUNK_SIDE {
                for x in 0..CHUNK_SIDE {
                    c.set_voxel(local_from([x, y, z]).unwrap(), VoxelShape::solid(STONE));
                }
            }
        }
        c
    }

    #[test]
    fn a_solid_chunk_is_six_quads_and_a_buried_one_is_none() {
        // The pad, measured. An isolated solid chunk has six faces; the same
        // chunk surrounded by solid rock has none, because none of them are
        // visible from anywhere and the mesher should not draw them.
        let c = solid_chunk();

        let isolated = Mesher::new()
            .build(&c, &MeshConfig::air_only(), None)
            .unwrap();
        assert_eq!(isolated.quads.len(), 6, "quads: {:?}", isolated.quads);

        let buried = Mesher::new()
            .build(
                &c,
                &MeshConfig::air_only(),
                Some(&|_local: [i32; 3]| Ok(VoxelShape::solid(STONE))),
            )
            .unwrap();
        assert_eq!(buried.quads.len(), 0, "a buried chunk still drew faces");
        // The buried case still reports the chunk's own voxels; it just draws
        // none of them, which is a different statement and both are needed.
        assert_eq!(buried.stats.solid_voxels, isolated.stats.solid_voxels);
    }

    #[test]
    fn a_neighbour_only_culls_the_faces_that_touch_it() {
        // Rock at the -x neighbour's boundary layer, and nothing anywhere else.
        // The voxel at local x = 0 has a -x face on the chunk boundary, so that
        // one face goes; its other five stay.
        let c = chunk_with(&[([0, 1, 1], VoxelShape::solid(STONE))], ChunkPos::new(0, 0, 0));
        let m = Mesher::new()
            .build(
                &c,
                &MeshConfig::air_only(),
                Some(&|local: [i32; 3]| {
                    Ok(if local[0] == -1 {
                        VoxelShape::solid(STONE)
                    } else {
                        VoxelShape::empty()
                    })
                }),
            )
            .unwrap();
        assert!(
            !m.quads.iter().any(|q| q.face == Face::NegX),
            "the -x face should have been culled by the neighbour"
        );
        assert_eq!(m.quads.len(), 5, "quads: {:?}", m.quads);
        assert_eq!(m.stats.visible_faces, 5 * 16);
    }

    #[test]
    fn water_is_see_through_and_rock_is_not() {
        let config = MeshConfig::air_only().with_transparent(WATER);
        // Air shows the water surface; water shows the rock under it; the rock
        // does not draw a face into the water, which is what stops two
        // coincident quads z-fighting.
        assert!(config.face_is_visible(WATER, MaterialId::EMPTY));
        assert!(config.face_is_visible(STONE, WATER));
        assert!(!config.face_is_visible(WATER, STONE));
        // Two cells of the same material never have a face between them.
        assert!(!config.face_is_visible(WATER, WATER));
        assert!(!config.face_is_visible(STONE, STONE));
        assert!(!config.face_is_visible(MaterialId::EMPTY, STONE));
    }

    #[test]
    fn the_visible_set_comes_from_the_registry_not_from_a_hardcoded_id() {
        let mut r = MaterialRegistry::new();
        r.admit_component(ComponentId(2), "water/ice");
        r.admit_material(voxel_model::Material {
            id: WATER,
            name: "water".into(),
            components: vec![Component {
                id: ComponentId(2),
                ratio: Ratio::ONE,
                density_kg_m3: Ratio::from_u64(1000),
            }],
            optical_class: OpticalClass::Transmissive,
            reflectance: None,
        })
        .unwrap();
        let config = MeshConfig::from_registry(&r).unwrap();
        assert!(config.is_transparent(WATER));
        assert!(config.is_transparent(MaterialId::EMPTY));
        assert!(config.is_opaque(STONE));
    }

    #[test]
    fn meshing_the_same_chunk_twice_produces_the_same_digest() {
        let voxels: Vec<([i32; 3], VoxelShape)> = (0..CHUNK_SIDE)
            .step_by(3)
            .flat_map(|x| {
                (0..CHUNK_SIDE)
                    .step_by(5)
                    .map(move |y| ([x, y, 4 + (x % 3)], VoxelShape::solid(STONE)))
            })
            .collect();
        let c = chunk_with(&voxels, ChunkPos::new(0, 0, 0));
        let a = mesh(&c);
        let b = mesh(&c);
        assert_eq!(a.digest(), b.digest());
        assert_eq!(a.quads, b.quads);

        // A different chunk position is a different mesh.
        let d = chunk_with(&voxels, ChunkPos::new(1, 0, 0));
        assert_ne!(a.digest(), mesh(&d).digest());
    }

    #[test]
    fn corner_positions_are_chunk_local_metres() {
        let m = mesh(&one_voxel(VoxelShape::solid(STONE)));
        let q = m.quads.iter().find(|q| q.face == Face::NegX).unwrap();
        let p = m.corner_positions_m(q);
        // The face is at subvoxel 4, and a subvoxel is a quarter of a metre, so
        // the whole quad lies at x = 1 m; the other two axes span 1 m to 2 m.
        // The corners are checked as a bounding box rather than in order,
        // because the winding reverses the order and that is not the point here.
        for (axis, expected) in [(1usize, 1.0f32), (2, 1.0)] {
            let lo = p.iter().map(|c| c[axis]).fold(f32::INFINITY, f32::min);
            let hi = p.iter().map(|c| c[axis]).fold(f32::NEG_INFINITY, f32::max);
            assert!((lo - expected).abs() < 1e-6, "axis {axis} low was {lo}");
            assert!((hi - (expected + 1.0)).abs() < 1e-6, "axis {axis} high was {hi}");
        }
        for c in p {
            assert!((c[0] - 1.0).abs() < 1e-6, "a corner left the face plane: {c:?}");
        }
        assert_eq!(m.origin_voxel(), [0, 0, 0]);
    }

    #[test]
    fn the_winding_rule_is_the_one_the_corners_use() {
        for face in Face::ALL {
            let flips = winding_flips(face);
            let expected = match face {
                Face::NegX | Face::NegY | Face::NegZ => true,
                Face::PosX | Face::PosY | Face::PosZ => false,
            } != (face.axis() == 1);
            assert_eq!(flips, expected, "{face:?}");
        }
    }

    #[test]
    fn the_mesher_holds_its_scratch_rather_than_allocating_per_chunk() {
        let m = Mesher::new();
        assert_eq!(m.field.len(), FIELD_SIDE.pow(3));
        assert_eq!(m.mask.len(), INTERIOR * INTERIOR);
        assert!(m.scratch_bytes() > 4_000_000);
    }

    #[test]
    fn an_empty_chunk_meshes_to_nothing_without_refusing() {
        let m = mesh(&Chunk::new(ChunkPos::new(-3, 2, 5)));
        assert!(m.quads.is_empty());
        assert_eq!(m.stats.quads, 0);
        assert_eq!(m.stats.faces_per_quad(), 0.0);
        assert_eq!(m.origin_voxel(), [-96, 64, 160]);
    }
}
