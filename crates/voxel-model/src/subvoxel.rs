//! The 4×4×4 subvoxel model, and the indices of the 256× detail lattice.
//!
//! A voxel is **not** a cube with one material. It is a 4×4×4 grid of subvoxels,
//! and *that* is what makes a block a shape: a voxel can be half soil and half
//! rock, or a stair, or a corner of ore in stone. The appearance layer reads this
//! grid and blends its visible face; nothing anywhere reads "the block id",
//! because there is no such thing.
//!
//! The second resolution is the **detail lattice** — the base the record calls
//! **256**, because a subvoxel face carries 16×16 samples: **16** per subvoxel axis.
//! It is **addressed, never stored** (C-2): [`DetailIndex`] is the address, and the
//! value at that address is computed on demand by the appearance layer. The dense
//! alternative is 4,096 samples per **subvoxel**, **2³³** of them for one chunk —
//! about 8.6 GB at a byte a sample — which is not a renderer either.
//!
//! (That figure was written here as "per voxel" for one revision, which is low by
//! the subvoxel count, 64×. The corrected arithmetic and the test that pins it are
//! in `voxel-appearance::detail`; `LEDGER.md` C-28 records the correction.)

use crate::coord::{DETAIL_SIDE, SUBVOXEL_COUNT, SUBVOXEL_SIDE};
use crate::material::MaterialId;
use crate::refusal::{Refusal, RefusalKind, Refused};

/// A subvoxel's offset inside its voxel. Always in `0..SUBVOXEL_SIDE`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct SubvoxelIndex {
    pub x: u8,
    pub y: u8,
    pub z: u8,
}

impl SubvoxelIndex {
    /// Refuse an index outside the voxel rather than clamping it.
    ///
    /// Clamping is the classic default: `min(3, 7)` silently answers a question
    /// nobody asked, and the caller never learns it was out of range.
    pub const fn new(x: u8, y: u8, z: u8) -> Refused<Self> {
        if (x as u32) < SUBVOXEL_SIDE && (y as u32) < SUBVOXEL_SIDE && (z as u32) < SUBVOXEL_SIDE {
            Ok(SubvoxelIndex { x, y, z })
        } else {
            Err(Refusal::new(
                RefusalKind::IndexOutOfRange,
                "subvoxel offset outside 0..4",
            ))
        }
    }

    /// The index as a flat `0..64`. `x` is the fast axis.
    pub const fn flat(self) -> usize {
        self.x as usize + SUBVOXEL_SIDE as usize * (self.y as usize + SUBVOXEL_SIDE as usize * self.z as usize)
    }

    /// The inverse of [`Self::flat`].
    pub const fn from_flat(flat: usize) -> Refused<Self> {
        if flat >= SUBVOXEL_COUNT {
            return Err(Refusal::new(
                RefusalKind::IndexOutOfRange,
                "flat subvoxel index outside 0..64",
            ));
        }
        let s = SUBVOXEL_SIDE as usize;
        Ok(SubvoxelIndex {
            x: (flat % s) as u8,
            y: ((flat / s) % s) as u8,
            z: (flat / (s * s)) as u8,
        })
    }

    /// Iterate every subvoxel in index order, which is the order everything else
    /// must agree on.
    pub fn all() -> impl Iterator<Item = SubvoxelIndex> {
        (0..SUBVOXEL_COUNT).filter_map(|i| SubvoxelIndex::from_flat(i).ok())
    }
}

/// A face of a voxel.
///
/// **`Ord` and `Hash` are here so a face can be a map key.** The mesher's
/// coverage guard identifies a drawn face by `(cell, face)` and has to put those
/// in a `BTreeSet` to diff the greedy mesh against a naive one; without an order
/// the guard would have to invent an index, and an invented index is one more
/// thing that can disagree with [`Face::ALL`]'s own order. Deriving it keeps the
/// declaration order the ordering.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Face {
    NegX,
    PosX,
    NegY,
    PosY,
    NegZ,
    PosZ,
}

impl Face {
    pub const ALL: [Face; 6] = [Face::NegX, Face::PosX, Face::NegY, Face::PosY, Face::NegZ, Face::PosZ];

    pub const fn normal(self) -> (i32, i32, i32) {
        match self {
            Face::NegX => (-1, 0, 0),
            Face::PosX => (1, 0, 0),
            Face::NegY => (0, -1, 0),
            Face::PosY => (0, 1, 0),
            Face::NegZ => (0, 0, -1),
            Face::PosZ => (0, 0, 1),
        }
    }

    /// The opposite face.
    pub const fn opposite(self) -> Face {
        match self {
            Face::NegX => Face::PosX,
            Face::PosX => Face::NegX,
            Face::NegY => Face::PosY,
            Face::PosY => Face::NegY,
            Face::NegZ => Face::PosZ,
            Face::PosZ => Face::NegZ,
        }
    }

    /// Which axis this face's normal is on: 0 = x, 1 = y, 2 = z.
    pub const fn axis(self) -> u8 {
        match self {
            Face::NegX | Face::PosX => 0,
            Face::NegY | Face::PosY => 1,
            Face::NegZ | Face::PosZ => 2,
        }
    }

    /// True when the face is on the **high** side of its axis.
    pub const fn is_positive(self) -> bool {
        matches!(self, Face::PosX | Face::PosY | Face::PosZ)
    }
}

/// The 4×4×4 material grid of one voxel.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct VoxelShape {
    cells: [MaterialId; SUBVOXEL_COUNT],
}

impl Default for VoxelShape {
    fn default() -> Self {
        Self::empty()
    }
}

impl VoxelShape {
    /// A voxel with no matter in it.
    pub const fn empty() -> Self {
        VoxelShape { cells: [MaterialId::EMPTY; SUBVOXEL_COUNT] }
    }

    /// A voxel filled solid with one material — the common case, and the one
    /// that must stay cheap.
    pub const fn solid(material: MaterialId) -> Self {
        VoxelShape { cells: [material; SUBVOXEL_COUNT] }
    }

    pub fn get(&self, at: SubvoxelIndex) -> MaterialId {
        self.cells[at.flat()]
    }

    /// Set a subvoxel. Setting a zero id **empties** the cell rather than
    /// admitting a material that does not exist.
    pub fn set(&mut self, at: SubvoxelIndex, material: MaterialId) {
        self.cells[at.flat()] = material;
    }

    /// Every cell, in flat index order.
    pub fn cells(&self) -> &[MaterialId; SUBVOXEL_COUNT] {
        &self.cells
    }

    /// The grid mutably — the handle a **material shaper** writes through (see
    /// `crate::shaper`). Only that path takes this: an occupancy change goes
    /// through [`VoxelShape::set`], which is what a physical shaper is given.
    pub fn cells_mut(&mut self) -> &mut [MaterialId; SUBVOXEL_COUNT] {
        &mut self.cells
    }

    /// True when no cell holds matter.
    pub fn is_empty(&self) -> bool {
        self.cells.iter().all(|m| m.is_empty())
    }

    /// The subvoxels on `face`, in a stable order. A face is a 4×4 grid, so this
    /// is always 16 cells.
    pub fn face_cells(&self, face: Face) -> Vec<MaterialId> {
        let s = SUBVOXEL_SIDE as u8;
        let high = if face.is_positive() { s - 1 } else { 0 };
        let mut out = Vec::with_capacity(16);
        // Iterate the two off-axis coordinates in index order so two callers
        // always see the same face in the same order.
        let (mut u, mut v) = (0u8, 0u8);
        while u < s {
            while v < s {
                let idx = match face.axis() {
                    0 => SubvoxelIndex { x: high, y: u, z: v },
                    1 => SubvoxelIndex { x: u, y: high, z: v },
                    _ => SubvoxelIndex { x: u, y: v, z: high },
                };
                out.push(self.get(idx));
                v += 1;
            }
            v = 0;
            u += 1;
        }
        out
    }

    /// Which of a face's 16 cells are **recessed**.
    ///
    /// A cell is recessed when its material differs from the material covering
    /// most of that face — that is, when the face is not uniform and this cell is
    /// not part of the majority. This is the bridge from *materials* to
    /// *normals*, and it is deliberately **derived rather than stored**:
    ///
    /// * A **material shaper** (`pillar`, say) writes materials and nothing else.
    ///   A pillar is a cube whose face pattern is an inscribed circle — the
    ///   corners fall outside it — so the corner cells hold the shaper's inset
    ///   material and every other cell holds the block's own. Nothing is carved,
    ///   so the block's **silhouette is a cube**, which is the whole point of the
    ///   shaper being a material one.
    /// * The shading then bends its normal around that pattern, which is what
    ///   makes a cube read as round without one triangle of extra geometry
    ///   ("fake wrapping" — see `voxel-appearance::wrap`).
    ///
    /// Deriving it costs one pass over 16 cells and **stores nothing**, which
    /// matters because the 4×4×4 grid is already 128 bytes per voxel and the
    /// octree holding it is 182× the mesh (`STATUS.md`). A stored normal field
    /// would be another 64 bytes on top for a value that is a function of the
    /// bytes already there.
    ///
    /// A uniform face has no recess: nothing differs, so nothing stands out.
    /// Returned in the same `(u, v)` order as [`Self::face_cells`].
    pub fn face_recess(&self, face: Face) -> [bool; SUBVOXEL_COUNT / 4] {
        let cells = self.face_cells(face);
        // The majority material, from the blend the appearance layer already
        // shades. `face_blend` is empty only for a face with no cells, which
        // cannot happen: a face is always 16 cells.
        let majority = self
            .face_blend(face)
            .first()
            .map(|(m, _)| *m)
            .unwrap_or(MaterialId::EMPTY);
        let mut out = [false; SUBVOXEL_COUNT / 4];
        for (slot, cell) in out.iter_mut().zip(cells) {
            *slot = cell != majority;
        }
        out
    }

    /// How many of a face's cells are recessed — the number a test can assert and
    /// a report can print.
    pub fn recessed_cells(&self, face: Face) -> usize {
        self.face_recess(face).iter().filter(|r| **r).count()
    }

    /// The face's material **blend**: how many of the 16 visible cells hold each
    /// material, most-covering first. Ties break by material id so the result is
    /// deterministic (F1: never iteration order).
    ///
    /// This is the composition the appearance layer shades — *derived from block
    /// information*, which is the census's own scope clause.
    pub fn face_blend(&self, face: Face) -> Vec<(MaterialId, u8)> {
        let mut counts: [(MaterialId, u8); SUBVOXEL_COUNT] =
            [(MaterialId::EMPTY, 0); SUBVOXEL_COUNT];
        let mut len = 0usize;
        'cells: for m in self.face_cells(face) {
            for slot in counts.iter_mut().take(len) {
                if slot.0 == m {
                    slot.1 += 1;
                    continue 'cells;
                }
            }
            counts[len] = (m, 1);
            len += 1;
        }
        let mut out: Vec<(MaterialId, u8)> = counts[..len].to_vec();
        out.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        out
    }
}

/// An address in the 256× material detail lattice.
///
/// **Not a stored grid.** The value at an address is computed by the appearance
/// layer from the material and the address, deterministically. See
/// `docs/APPEARANCE.md` §2 and C-2.
///
/// The address lies inside **one subvoxel's** lattice. Which subvoxel it belongs
/// to is deliberately *not* a field here: it is part of the lattice's seed in the
/// appearance layer, so a subvoxel's pattern is not the same pattern tiled.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct DetailIndex {
    pub x: u16,
    pub y: u16,
    pub z: u16,
}

impl DetailIndex {
    /// Refuse an address outside the lattice.
    pub const fn new(x: u16, y: u16, z: u16) -> Refused<Self> {
        if (x as u32) < DETAIL_SIDE && (y as u32) < DETAIL_SIDE && (z as u32) < DETAIL_SIDE {
            Ok(DetailIndex { x, y, z })
        } else {
            Err(Refusal::new(
                RefusalKind::IndexOutOfRange,
                "detail address outside 0..16",
            ))
        }
    }

    /// Centre of the lattice — the address a subvoxel's *average* sample uses.
    pub const fn centre() -> Self {
        DetailIndex { x: DETAIL_SIDE as u16 / 2, y: DETAIL_SIDE as u16 / 2, z: DETAIL_SIDE as u16 / 2 }
    }

    /// The address a per-axis fraction maps to, refusing a fraction outside
    /// `[0, 1]` rather than wrapping it.
    pub fn from_unit(x: f64, y: f64, z: f64) -> Refused<Self> {
        let one = |v: f64, axis: &str| -> Refused<u16> {
            if !(0.0..=1.0).contains(&v) || !v.is_finite() {
                return Err(Refusal::owned(
                    RefusalKind::OutOfDeclaredRange,
                    format!("detail {axis} fraction {v} outside [0, 1]"),
                ));
            }
            // 1.0 maps to DETAIL_SIDE - 1: the lattice is a grid of samples, not
            // a half-open range, so the top edge is the last sample.
            Ok((v * (DETAIL_SIDE - 1) as f64).round() as u16)
        };
        Ok(DetailIndex { x: one(x, "x")?, y: one(y, "y")?, z: one(z, "z")? })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_subvoxel_index_out_of_range_is_refused_not_clamped() {
        assert_eq!(SubvoxelIndex::new(3, 3, 3).unwrap().flat(), 63);
        let r = SubvoxelIndex::new(4, 0, 0).unwrap_err();
        assert_eq!(r.kind(), RefusalKind::IndexOutOfRange);
    }

    #[test]
    fn flat_and_index_round_trip_over_all_64() {
        for i in 0..SUBVOXEL_COUNT {
            let idx = SubvoxelIndex::from_flat(i).unwrap();
            assert_eq!(idx.flat(), i);
        }
        assert!(SubvoxelIndex::from_flat(64).is_err());
    }

    #[test]
    fn all_enumerates_exactly_64_in_index_order() {
        let all: Vec<_> = SubvoxelIndex::all().collect();
        assert_eq!(all.len(), 64);
        assert_eq!(all[0].flat(), 0);
        assert_eq!(all[63].flat(), 63);
    }

    #[test]
    fn an_empty_voxel_has_no_matter_and_a_solid_one_has_no_empties() {
        assert!(VoxelShape::empty().is_empty());
        let solid = VoxelShape::solid(MaterialId(7));
        assert!(!solid.is_empty());
        for cell in solid.cells() {
            assert_eq!(*cell, MaterialId(7));
        }
    }

    #[test]
    fn a_half_and_half_voxel_reports_a_face_blend_of_two() {
        // Bottom two subvoxel layers are stone, top two are soil: a genuinely
        // half-and-half block, which is the case a block-id renderer cannot draw.
        let mut shape = VoxelShape::empty();
        for idx in SubvoxelIndex::all() {
            shape.set(idx, if idx.z < 2 { MaterialId(1) } else { MaterialId(2) });
        }
        // The +Z face is entirely soil (z = 3).
        let top = shape.face_blend(Face::PosZ);
        assert_eq!(top, vec![(MaterialId(2), 16)]);
        // The +X face sees all four layers: 8 cells each.
        let side = shape.face_blend(Face::PosX);
        assert_eq!(side, vec![(MaterialId(1), 8), (MaterialId(2), 8)]);
    }

    #[test]
    fn face_blend_breaks_ties_by_material_id_so_it_is_deterministic() {
        let mut shape = VoxelShape::empty();
        // Alternate two materials across the +Z face: a 8/8 tie.
        for idx in SubvoxelIndex::all() {
            if idx.z == 3 {
                let m = if (idx.x + idx.y) % 2 == 0 { MaterialId(9) } else { MaterialId(2) };
                shape.set(idx, m);
            }
        }
        let blend = shape.face_blend(Face::PosZ);
        assert_eq!(blend, vec![(MaterialId(2), 8), (MaterialId(9), 8)]);
    }

    #[test]
    fn a_uniform_face_has_nothing_recessed() {
        // There is no majority to stand out from, which is the honest answer and
        // not "the whole face is recessed".
        let solid = VoxelShape::solid(MaterialId(6));
        for face in Face::ALL {
            assert_eq!(solid.recessed_cells(face), 0, "{face:?}");
            assert!(solid.face_recess(face).iter().all(|r| !*r));
        }
        // An empty voxel is uniform in the empty material, so it too is flat.
        let empty = VoxelShape::empty();
        assert_eq!(empty.recessed_cells(Face::PosZ), 0);
    }

    #[test]
    fn a_corner_pattern_recesses_exactly_the_corners() {
        // The pillar's own pattern, written by hand: the four corner cells of
        // every face hold the inset material and the other twelve hold the
        // block's. A 4×4 face with its corners knocked out is the discrete
        // version of a circle inscribed in the square, which is what makes the
        // derived normal read as round.
        const SHEATH: MaterialId = MaterialId(6);
        const INSET: MaterialId = MaterialId(5);
        let mut shape = VoxelShape::solid(SHEATH);
        for at in SubvoxelIndex::all() {
            let corner = (at.x == 0 || at.x == 3) && (at.y == 0 || at.y == 3) && (at.z == 0 || at.z == 3);
            if corner {
                shape.set(at, INSET);
            }
        }
        // The +z face is `x` × `y` at z = 3: its four corners are (0,0), (3,0),
        // (0,3), (3,3) in `(u, v)` order — which is exactly `face_cells`' order.
        let recess = shape.face_recess(Face::PosZ);
        assert_eq!(shape.recessed_cells(Face::PosZ), 4);
        // `face_cells` walks `u` on the outside and `v` on the inside, so the
        // flat slot of `(u, v)` is `4·u + v`. Pinned here because a test that
        // used the transposed index would pass on a symmetric pattern and be
        // wrong about every pattern that is not.
        let idx = |u: usize, v: usize| 4 * u + v;
        for (u, v) in [(0usize, 0usize), (3, 0), (0, 3), (3, 3)] {
            assert!(recess[idx(u, v)], "({u},{v}) should be recessed");
        }
        for (u, v) in [(1usize, 0usize), (0, 1), (1, 1), (2, 2)] {
            assert!(!recess[idx(u, v)], "({u},{v}) should not be recessed");
        }
        // The transposed index is a *different* answer, which is the point of
        // pinning it: only the corners are symmetric under `u ↔ v`.
        assert_eq!(recess[idx(0, 1)], recess[idx(1, 0)]);
        // And the whole pattern is symmetric, which is what a pillar is.
        assert_eq!(shape.recessed_cells(Face::PosZ), shape.recessed_cells(Face::NegZ));
        assert_eq!(shape.recessed_cells(Face::PosX), shape.recessed_cells(Face::NegX));
        assert_eq!(shape.recessed_cells(Face::PosY), shape.recessed_cells(Face::NegY));
    }

    #[test]
    fn a_half_and_half_face_recesses_the_side_that_covers_less() {
        // The majority rule, on a tie-free case: three of four columns one way,
        // one the other, on the +z face. The minority column is the recess.
        let mut shape = VoxelShape::solid(MaterialId(6));
        for at in SubvoxelIndex::all() {
            if at.z == 3 && at.x == 0 {
                shape.set(at, MaterialId(5));
            }
        }
        assert_eq!(shape.recessed_cells(Face::PosZ), 4, "the x = 0 column is the minority");
        let recess = shape.face_recess(Face::PosZ);
        // `u` is the fast-varying coordinate on the +z face, so a *column* is
        // constant in `u` and varies in `v`: slots 4·0 + v.
        for v in 0..4usize {
            assert!(recess[v], "the minority column should be recessed at v = {v}");
            assert!(!recess[4 + v], "u = 1 is majority and must not be recessed");
        }
    }

    #[test]
    fn a_tie_recesses_the_material_the_blend_names_second_and_that_is_deterministic() {
        // A tie is where a derived field is most likely to flicker. `face_blend`
        // breaks it by material id, so the majority is the *lowest* id and the
        // recess is the other one — the same answer on every run.
        let mut shape = VoxelShape::empty();
        for at in SubvoxelIndex::all() {
            if at.z == 3 {
                shape.set(at, if (at.x + at.y) % 2 == 0 { MaterialId(4) } else { MaterialId(9) });
            }
        }
        let a = shape.face_recess(Face::PosZ);
        let b = shape.face_recess(Face::PosZ);
        assert_eq!(a, b);
        // 4 beats 9 on the id tiebreak, so the 9s are the recess — 8 of them.
        assert_eq!(shape.recessed_cells(Face::PosZ), 8);
    }

    #[test]
    fn opposite_faces_point_away_from_each_other() {
        for f in Face::ALL {
            let (nx, ny, nz) = f.normal();
            let (ox, oy, oz) = f.opposite().normal();
            assert_eq!((nx + ox, ny + oy, nz + oz), (0, 0, 0));
            assert_eq!(f.axis(), f.opposite().axis());
            assert_ne!(f.is_positive(), f.opposite().is_positive());
        }
    }

    #[test]
    fn a_detail_address_outside_the_lattice_is_refused() {
        assert!(DetailIndex::new(15, 15, 15).is_ok());
        let e = DetailIndex::new(16, 0, 0).unwrap_err();
        assert_eq!(e.kind(), RefusalKind::IndexOutOfRange);
        // The message names the side, so a change of base cannot leave a refusal
        // naming the old one.
        assert!(e.detail().contains("0..16"), "{}", e.detail());
    }

    #[test]
    fn detail_fractions_map_within_range_and_refuse_outside_it() {
        let d = DetailIndex::from_unit(0.0, 0.5, 1.0).unwrap();
        assert_eq!(d.x, 0);
        // The top edge is the last sample of the side: 15 of 16, not of 256.
        assert_eq!(d.z, 15);
        assert_eq!(DetailIndex::from_unit(1.5, 0.0, 0.0).unwrap_err().kind(), RefusalKind::OutOfDeclaredRange);
        assert_eq!(DetailIndex::from_unit(f64::NAN, 0.0, 0.0).unwrap_err().kind(), RefusalKind::OutOfDeclaredRange);
    }
}
