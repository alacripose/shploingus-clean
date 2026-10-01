//! Block truth, as the appearance layer reads it.
//!
//! `voxel-appearance` takes a [`VoxelComposition`] and **not a block id**
//! (`APPEARANCE.md` §4). There is no such thing as a block id here: a voxel is a
//! 4×4×4 grid of subvoxels, so the visible face of a voxel is a *mixture*, and
//! the blend is what gets shaded.
//!
//! # Two things a naive mix gets wrong, and this does not
//!
//! 1. **Empty cells are not matter.** A face that is twelve cells of rock and
//!    four of nothing is a face with a **hole** in it. Blending only the matter
//!    and normalising to one would quietly turn the hole into rock — the exact
//!    "plausible substitute" the one rule forbids. So [`VoxelComposition::open`]
//!    carries the hole's share and every material's [`coverage`] is a share of
//!    the *whole* face, so `Σcoverage + open == 1` exactly.
//! 2. **An unadmitted material is not a grey face.** A shape holding a material
//!    the registry never admitted is refused, because a renderer that invents an
//!    appearance for it has answered a question about the world that nobody
//!    answered.
//!
//! [`coverage`]: VoxelComposition::coverage

use voxel_model::{Face, MaterialId, MaterialRegistry, Ratio, Refused, VoxelShape};

/// A face — or a whole voxel — of block truth, resolved into the materials that
/// cover it and the share of it that holds no matter at all.
#[derive(Clone, PartialEq, Debug)]
pub struct VoxelComposition {
    /// Most-covering first; ties broken by material id. Same rule as
    /// [`VoxelShape::face_blend`], and a test holds the two together.
    materials: Vec<(MaterialId, Ratio)>,
    open: Ratio,
}

impl VoxelComposition {
    /// The composition of one face of a voxel.
    pub fn of_face(shape: &VoxelShape, face: Face, registry: &MaterialRegistry) -> Refused<Self> {
        Self::of_cells(&shape.face_cells(face), registry)
    }

    /// The composition of the whole voxel — all 64 cells, not just a surface.
    pub fn of_voxel(shape: &VoxelShape, registry: &MaterialRegistry) -> Refused<Self> {
        Self::of_cells(shape.cells(), registry)
    }

    fn of_cells(cells: &[MaterialId], registry: &MaterialRegistry) -> Refused<Self> {
        let total = cells.len() as u64;
        let mut counts: Vec<(MaterialId, u64)> = Vec::new();
        let mut empty = 0u64;

        for material in cells {
            if material.is_empty() {
                empty += 1;
                continue;
            }
            // *Before* it is counted, so an unadmitted material cannot reach a
            // blend at all — the refusal is not a fallback colour, it is the
            // absence of a composition.
            registry.material(*material)?;
            match counts.iter_mut().find(|(m, _)| m == material) {
                Some(slot) => slot.1 += 1,
                None => counts.push((*material, 1)),
            }
        }

        // Most-covering first, ties by id — the deterministic order, so two
        // callers never see the same face in two orders (F1: never iteration
        // order).
        counts.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

        let mut materials = Vec::with_capacity(counts.len());
        for (material, count) in counts {
            materials.push((material, Ratio::new(count, total)?));
        }

        Ok(VoxelComposition { materials, open: Ratio::new(empty, total)? })
    }

    /// The materials covering this face, most-covering first.
    pub fn materials(&self) -> &[(MaterialId, Ratio)] {
        &self.materials
    }

    /// The share of the face that holds no matter — the hole.
    pub fn open(&self) -> Ratio {
        self.open
    }

    /// True when no cell holds matter, so there is nothing to shade.
    pub fn is_empty(&self) -> bool {
        self.materials.is_empty()
    }

    /// The most-covering material, or `None` if the face is entirely open.
    ///
    /// `None` rather than a plausible material: a completely open face has no
    /// dominant material, and answering one anyway is a default.
    pub fn dominant(&self) -> Option<MaterialId> {
        self.materials.first().map(|(m, _)| *m)
    }

    /// How much of the face a material covers. `0` when it is not present —
    /// a genuine zero, since the material is genuinely absent from the blend.
    pub fn coverage(&self, material: MaterialId) -> Ratio {
        self.materials
            .iter()
            .find(|(m, _)| *m == material)
            .map(|(_, r)| *r)
            .unwrap_or(Ratio::ZERO)
    }

    /// The total matter coverage. `Σcoverage + open` is exactly one.
    pub fn coverage_sum(&self) -> Ratio {
        let mut acc = Ratio::ZERO;
        for (_, r) in &self.materials {
            acc += *r;
        }
        acc
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_model::{MaterialId, RefusalKind, SubvoxelIndex};
    use voxel_shapers::column::materials::{self, IGNEOUS, REGOLITH};

    fn registry() -> MaterialRegistry {
        materials::registry().unwrap()
    }

    fn solid(material: MaterialId) -> VoxelShape {
        VoxelShape::solid(material)
    }

    #[test]
    fn a_solid_voxel_s_face_is_one_material_and_no_hole() {
        let c = VoxelComposition::of_face(&solid(IGNEOUS), Face::PosZ, &registry()).unwrap();
        assert_eq!(c.materials(), &[(IGNEOUS, Ratio::ONE)]);
        assert_eq!(c.open(), Ratio::ZERO);
        assert_eq!(c.dominant(), Some(IGNEOUS));
        assert!(!c.is_empty());
    }

    #[test]
    fn an_empty_voxel_has_no_composition_and_names_no_material() {
        let c = VoxelComposition::of_face(&VoxelShape::empty(), Face::PosZ, &registry()).unwrap();
        assert!(c.is_empty());
        assert_eq!(c.dominant(), None);
        assert_eq!(c.open(), Ratio::ONE);
        assert_eq!(c.coverage_sum(), Ratio::ZERO);
    }

    #[test]
    fn a_quarter_empty_face_reports_the_hole_rather_than_filling_it() {
        // Twelve cells of rock, four of nothing. A blend that normalised over the
        // matter alone would report *all rock* and hide the hole.
        let mut shape = VoxelShape::empty();
        let mut filled = 0;
        for idx in SubvoxelIndex::all() {
            if idx.z == 3 && filled < 12 {
                shape.set(idx, REGOLITH);
                filled += 1;
            }
        }
        let c = VoxelComposition::of_face(&shape, Face::PosZ, &registry()).unwrap();
        assert_eq!(c.coverage(REGOLITH), Ratio::new(12, 16).unwrap());
        assert_eq!(c.open(), Ratio::new(4, 16).unwrap());
        // The three-quarter/one-quarter split, exactly.
        assert_eq!(c.coverage(REGOLITH) + c.open(), Ratio::ONE);
    }

    #[test]
    fn a_half_and_half_face_is_two_materials_of_equal_coverage() {
        let mut shape = VoxelShape::empty();
        for idx in SubvoxelIndex::all() {
            shape.set(idx, if idx.z < 2 { IGNEOUS } else { REGOLITH });
        }
        let c = VoxelComposition::of_face(&shape, Face::PosX, &registry()).unwrap();
        assert_eq!(c.coverage(IGNEOUS), Ratio::new(1, 2).unwrap());
        assert_eq!(c.coverage(REGOLITH), Ratio::new(1, 2).unwrap());
        assert_eq!(c.open(), Ratio::ZERO);
        // An 8/8 tie breaks by material id, so the *lower* id leads: regolith is
        // 3 and igneous is 6. Two callers must never see two orders.
        assert_eq!(c.dominant(), Some(REGOLITH));
    }

    #[test]
    fn coverage_plus_holes_is_exactly_one_on_every_face_of_a_mixed_voxel() {
        // Exact rationals, not floats: this is the invariant the blend rests on.
        let mut shape = VoxelShape::empty();
        for idx in SubvoxelIndex::all() {
            shape.set(
                idx,
                match (idx.x + idx.y + idx.z) % 3 {
                    0 => IGNEOUS,
                    1 => REGOLITH,
                    // The third cell of every three is a hole.
                    _ => MaterialId::EMPTY,
                },
            );
        }
        let registry = registry();
        for face in Face::ALL {
            let c = VoxelComposition::of_face(&shape, face, &registry).unwrap();
            assert_eq!(
                c.coverage_sum() + c.open(),
                Ratio::ONE,
                "face {face:?} did not account for the whole face"
            );
        }
    }

    #[test]
    fn an_unadmitted_material_is_refused_rather_than_blended_as_grey() {
        let r = VoxelComposition::of_face(&solid(MaterialId(600)), Face::PosZ, &registry())
            .unwrap_err();
        assert_eq!(r.kind(), RefusalKind::UnadmittedMaterial);
    }

    #[test]
    fn the_whole_voxel_blends_all_sixty_four_cells() {
        let mut shape = VoxelShape::empty();
        for idx in SubvoxelIndex::all() {
            // One layer of soil over three of rock: 16 of 64 are soil.
            shape.set(idx, if idx.z == 3 { REGOLITH } else { IGNEOUS });
        }
        let c = VoxelComposition::of_voxel(&shape, &registry()).unwrap();
        assert_eq!(c.coverage(REGOLITH), Ratio::new(16, 64).unwrap());
        assert_eq!(c.coverage(IGNEOUS), Ratio::new(48, 64).unwrap());
        assert_eq!(c.open(), Ratio::ZERO);
    }

    #[test]
    fn the_order_matches_the_model_s_own_face_blend_rule() {
        // The model already orders a blend (count desc, then id). This crate
        // cannot use `face_blend` directly because it must split out the empty
        // cells, so the rule is re-derived here — and held to the model's own
        // answer by this test, so the two cannot drift apart.
        let mut shape = VoxelShape::empty();
        for idx in SubvoxelIndex::all() {
            shape.set(
                idx,
                match (idx.x + idx.y) % 4 {
                    0 => IGNEOUS,
                    1 => REGOLITH,
                    _ => MaterialId::EMPTY,
                },
            );
        }
        let c = VoxelComposition::of_face(&shape, Face::PosY, &registry()).unwrap();

        let from_model: Vec<MaterialId> = shape
            .face_blend(Face::PosY)
            .into_iter()
            .filter(|(m, _)| !m.is_empty())
            .map(|(m, _)| m)
            .collect();
        let from_here: Vec<MaterialId> = c.materials().iter().map(|(m, _)| *m).collect();
        assert_eq!(from_here, from_model);
    }
}
