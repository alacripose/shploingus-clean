//! **The parent-facing seam** — this clean room's world seen through the parent's own two traits.
//!
//! [`HandedOver`] is the parent's `VoxelSource` (one voxel in, one shape out) and [`Halo`] is its
//! `ChunkNeighborhood` (the voxels one past each face of the chunk being meshed, which is the question the
//! parent's mesher asks and the one this world answers with [`crate::coordinates::CHUNK_SIDE`] arithmetic).
//! Nothing here owns world state: both structs borrow the world, and the shapes they answer with are built
//! by the mappings in [`super::mapping`].

use super::*;

/// **This clean room's world, seen through the parent's own `VoxelSource`.** One method, as the parent's
/// trait intends: it asks the world what is at a voxel and hands the answer over as a shape.
pub struct HandedOver<'a> {
    pub cube: &'a CubicWorld,
    /// The clean room's own water material, so a cell holding water is handed over as water.
    pub water: MaterialId,
    /// How a cell holding water is expressed in the parent's matter model.
    pub mapping: Water,
}

impl HandedOver<'_> {
    /// The shape at one world voxel: a solid cell is a full cube of its material, a cell holding water is
    /// water by the declared [`Water`] mapping (a solid cell that also holds water is its material, because
    /// the material is what stands there), and anything else is empty.
    ///
    /// A voxel **outside the world's own patch** is refused through the parent's own refusal: the parent's
    /// contract says a neighbourhood that cannot answer must refuse, *"because the alternative is claiming
    /// air, which would punch a halo of visible faces around every loaded chunk and look exactly like a bug
    /// in the mesher"*, and that is this crate's rule too.
    pub fn shape_at(&self, at: VoxelPos) -> ParentRefused<VoxelShape> {
        match self.cube.cell(at) {
            Ok(cell) => match (cell.material, cell.water_um) {
                (Some(material), _) => Ok(VoxelShape::solid(handed_over(material))),
                (None, um) if um > 0 => water_shape(handed_over(self.water), um, self.mapping),
                _ => Ok(VoxelShape::empty()),
            },
            Err(refusal) => Err(as_parent(&refusal)),
        }
    }
}

impl VoxelSource for HandedOver<'_> {
    fn voxel(&self, chunk: ChunkPos, local: LocalPos) -> ParentRefused<VoxelShape> {
        let at = VoxelPos::new(
            chunk.x * CHUNK_SIDE + local.x as i32,
            chunk.y * CHUNK_SIDE + local.y as i32,
            chunk.z * CHUNK_SIDE + local.z as i32,
        );
        self.shape_at(at)
    }
}

/// **The voxels just outside one chunk, through the same door** — the parent's `ChunkNeighborhood`, which
/// may be asked for `-1` and `CHUNK_SIDE`, so it is asked of the world and not of the chunk.
///
/// Where the world cannot answer — outside its own patch, or outside its own vertical span — what happens is
/// the caller's declared choice ([`Outside`]), and the count of those queries is kept ([`Halo::unanswered`])
/// so a reader of the measurement can see how much of the neighbourhood was a place the world does not own
/// rather than a place it said was empty.
pub struct Halo<'a> {
    pub cube: &'a CubicWorld,
    pub chunk: ChunkPos,
    pub water: MaterialId,
    pub mapping: Water,
    pub outside: Outside,
    pub unanswered: std::cell::Cell<u64>,
}

impl Halo<'_> {
    pub fn new(
        cube: &CubicWorld,
        chunk: ChunkPos,
        water: MaterialId,
        mapping: Water,
        outside: Outside,
    ) -> Halo<'_> {
        Halo { cube, chunk, water, mapping, outside, unanswered: std::cell::Cell::new(0) }
    }
}

impl ChunkNeighborhood for Halo<'_> {
    fn shape(&self, local: [i32; 3]) -> ParentRefused<VoxelShape> {
        let at = VoxelPos::new(
            self.chunk.x * CHUNK_SIDE + local[0],
            self.chunk.y * CHUNK_SIDE + local[1],
            self.chunk.z * CHUNK_SIDE + local[2],
        );
        let source =
            HandedOver { cube: self.cube, water: self.water, mapping: self.mapping };
        match source.shape_at(at) {
            Ok(shape) => Ok(shape),
            Err(refusal) => match self.outside {
                // The parent's own contract: a neighbourhood that cannot answer must refuse.
                Outside::World => Err(refusal),
                // This clean room's own chunk door's answer at its patch's edge: the world *declares* that
                // nothing is beyond its own span (nothing is invented above or below it), so the halo says
                // empty and the count of such answers is kept.
                Outside::Nothing | Outside::WorldOrNothing => {
                    self.unanswered.set(self.unanswered.get() + 1);
                    Ok(VoxelShape::empty())
                }
            },
        }
    }
}
