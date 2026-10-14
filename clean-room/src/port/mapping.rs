//! **How the two matter models meet** — the port's one place where this clean room's cells become the
//! parent's matter, and the three mappings, declared once, in one place.
//!
//! 1. **A material is handed over as its own number plus one.** The parent's `MaterialId::EMPTY` is 0 and
//!    is never a material; this clean room's ids are its own registry's. `+ 1` is a bijection that keeps
//!    the parent's reserved id reserved, and it is the only place the two id spaces meet.
//! 2. **A cell is a cube.** The parent's matter is 4×4×4 subvoxels per voxel; this clean room's world is
//!    one material per cell. So a solid cell is handed over as `VoxelShape::solid`, an empty one as
//!    `VoxelShape::empty`, and a cell holding water as **a full cube of water** — this is the quantisation
//!    the two models differ by (this clean room's water is a level in whole micrometres, the parent's
//!    matter is 64 subvoxels per cell) and the measurement reports it instead of hiding it.
//! 3. **The axes are handed over unswapped** (x→x, y→y, z→z, in subvoxels: ×4). The parent's
//!    `winding_flips` singles out its **axis 1** and this clean room's world is z-up, so the two
//!    conventions differ; a visibility law is invariant under a permutation of the axes, and the face-set
//!    comparison is what shows that rather than a sentence claiming it.
//!
//! [`Water`] is the declared choice the second mapping leaves open, and it is the *measurement's* subject:
//! the two readings bracket the truth and neither is it, which is why both are run and both are printed.

use super::*;

/// **The parent's matter resolution**: 4×4×4 subvoxels per voxel, so one voxel face is 16 subvoxel faces
/// and one voxel edge is 4 subvoxel edges. Named here because every conversion below is a division by it
/// and asking the parent for it per call would be four calls to say one thing.
pub const SUBVOXELS_PER_VOXEL: i32 = 4;

/// Hand a clean room material over to the parent's id space: **its own number plus one**, so the parent's
/// reserved `EMPTY` (0) stays reserved. The bijection is `n ↦ n + 1` and nothing else touches either space.
pub const fn handed_over(material: MaterialId) -> ParentMaterialId {
    ParentMaterialId(material.0 + 1)
}

/// The clean room's refusal as the parent's, at the one place the two vocabularies meet: the parent's own
/// name for a coordinate outside a declared range, carrying this crate's detail verbatim so nothing the
/// clean room said is lost.
pub(super) fn as_parent(refusal: &Refusal) -> ParentRefusal {
    ParentRefusal::owned(
        ParentRefusalKind::CoordinateOutOfRange,
        format!("[clean-room] {}", refusal.detail()),
    )
}

/// The parent's subvoxel resolution, as a count of layers up the cell: `SUBVOXELS_PER_VOXEL` = 4.
pub const SUBVOXEL_LAYERS: i32 = SUBVOXELS_PER_VOXEL;

/// **A cell's water, expressed in the parent's own matter model.** The bottom `k` of its four subvoxel
/// layers are water, where `k` is the cell's micrometre level rounded to the nearest quarter of a cell —
/// `water_um` is a depth in micrometres out of the `CELL_HOLD_UM` a full cell holds, and one of the
/// parent's four layers is a quarter of that. Rounded rather than truncated, so a cell holding just over a
/// half reads as three quarters and a cell holding just under as a half; the residual this leaves is the
/// quantisation the port is measuring, not a value it invents.
pub(super) fn water_shape(
    material: ParentMaterialId,
    water_um: i64,
    mapping: Water,
) -> ParentRefused<VoxelShape> {
    match mapping {
        Water::FullCube => Ok(VoxelShape::solid(material)),
        Water::AtTheParentsOwnResolution => {
            let hold = crate::chunk::CELL_HOLD_UM;
            let layers = ((water_um * SUBVOXEL_LAYERS as i64 + hold / 2) / hold)
                .clamp(0, SUBVOXEL_LAYERS as i64) as u8;
            let mut shape = VoxelShape::empty();
            for z in 0..layers {
                for y in 0..SUBVOXELS_PER_VOXEL as u8 {
                    for x in 0..SUBVOXELS_PER_VOXEL as u8 {
                        let at = SubvoxelIndex::new(x, y, z).map_err(|refusal| {
                            ParentRefusal::owned(
                                ParentRefusalKind::CoordinateOutOfRange,
                                format!("[clean-room] subvoxel ({x}, {y}, {z}) is not one of the 64: {}", refusal.detail()),
                            )
                        })?;
                        shape.set(at, material);
                    }
                }
            }
            Ok(shape)
        }
    }
}

/// **How a cell holding water is handed over to the parent's model.** The two readings are both declared;
/// the measurement runs both so that the *reason* for the water faces is a measured cause rather than a
/// sentence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Water {
    /// A **full cube** of water, whatever the cell's own level. The coarsest reading: it claims the water
    /// stands to the cell's ceiling.
    FullCube,
    /// The bottom `k` of the parent's **own four subvoxel layers**, where `k` is this cell's micrometre level
    /// rounded to the nearest quarter of a cell. This is the level expressed in the parent's own matter
    /// model, at the parent's own resolution.
    AtTheParentsOwnResolution,
}

impl Water {
    pub const fn name(self) -> &'static str {
        match self {
            Water::FullCube => "a full cube",
            Water::AtTheParentsOwnResolution => "at the parent's own 1/4-cell resolution",
        }
    }
}
