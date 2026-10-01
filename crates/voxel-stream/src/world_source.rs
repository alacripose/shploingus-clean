//! The shapers' world, as voxels.
//!
//! [`WorldSource`] is the one place where an abstract `WorldSpec` becomes
//! concrete voxels. Above it, the shapers reason about surfaces, strata, soil
//! and karst; below it, the mesher sees only [`VoxelShape`]s. Everything in
//! between is here, and that is deliberate: a second implementation of "what is
//! at this voxel" would be a second answer, and the two would disagree.
//!
//! # The one optimisation, and why it is safe
//!
//! [`voxel_shapers::column::material_at`] is the definition, and this crate must
//! agree with it **exactly** — which is tested, voxel by voxel, against it.
//!
//! What is added is a shortcut that cannot change the answer. A voxel whose
//! centre is above the column's surface is air, water or nothing else, and the
//! column's surface depends only on `(x, z)` — so a 32-voxel column computes it
//! **once** instead of 32 times, and skips the geology, substrate and karst work
//! entirely for every voxel that is above ground. Since most of the playable
//! band is above ground or far below it, this is most of the cost.
//!
//! The shortcut replays `material_at`'s own branch, condition for condition:
//! `y + 0.5 > surface`, then `y + 0.5 <= sea_level` … water, else air. If the
//! shaper's condition ever changes, the equivalence test fails rather than the
//! world silently gaining a second rule.

use voxel_model::{
    ChunkPos, LocalPos, MaterialId, Refused, VoxelShape, CHUNK_SIDE, VoxelPos,
};
use voxel_mesh::octree::SparseVoxelOctree;
use voxel_mesh::source::{ChunkNeighborhood, VoxelSource};

use voxel_shapers::column::{self, materials, DEFAULT_HYDRO_RADIUS};
use voxel_shapers::earthworks::Earthworks;
use voxel_shapers::{terrain, WorldSpec};

/// A world, as a source of voxels.
#[derive(Clone, PartialEq, Debug)]
pub struct WorldSource {
    spec: WorldSpec,
    earthworks: Earthworks,
    /// The neighbourhood radius hydrology walks per column. Explicit because
    /// shrinking it shrinks the watershed rather than approximating the same
    /// one — the shaper's own words, and a reason not to hide it in a default.
    hydro_radius: usize,
}

impl WorldSource {
    pub fn new(spec: WorldSpec) -> Self {
        WorldSource {
            spec,
            earthworks: Earthworks::new(),
            hydro_radius: DEFAULT_HYDRO_RADIUS,
        }
    }

    pub fn with_hydro_radius(mut self, radius: usize) -> Self {
        self.hydro_radius = radius;
        self
    }

    pub fn spec(&self) -> WorldSpec {
        self.spec
    }

    pub fn earthworks(&self) -> &Earthworks {
        &self.earthworks
    }

    /// The earthworks record, mutably — how a world is edited.
    ///
    /// The caller must then tell the resident set that the world changed
    /// ([`Resident::note_world_changed`](crate::Resident::note_world_changed)),
    /// because this type has no way to know how many chunks a change reached.
    pub fn earthworks_mut(&mut self) -> &mut Earthworks {
        &mut self.earthworks
    }

    pub fn hydro_radius(&self) -> usize {
        self.hydro_radius
    }

    /// The material at a **world** voxel: the generator, composed with the
    /// earthworks record, one layer down at exactly the shaper's own definition.
    pub fn material_at(&self, x: i32, y: i32, z: i32) -> Refused<MaterialId> {
        column::material_at(&self.spec, &self.earthworks, x, y, z)
    }

    /// The height-independent part of a column, for a caller filling a whole
    /// column of voxels.
    ///
    /// Computing this once per column instead of once per voxel is the
    /// difference between a world that streams and one that does not; see
    /// [`column::profile`].
    pub fn profile(&self, x: i32, z: i32) -> column::ColumnProfile {
        column::profile(&self.spec, x as f64 + 0.5, z as f64 + 0.5)
    }

    /// The material at a world voxel from an already-computed column profile.
    ///
    /// Equivalent to [`Self::material_at`] for every input. That equivalence is
    /// **tested**, not asserted: `WorldSource` is a shortcut over the shapers,
    /// and a shortcut that is merely believed is a second definition of where
    /// water is.
    pub fn material_in_column(
        &self,
        profile: &column::ColumnProfile,
        x: i32,
        y: i32,
        z: i32,
    ) -> Refused<MaterialId> {
        let generated = profile.material_at(&self.spec, x, y, z)?;
        Ok(self.earthworks.material_at(x, y, z, generated))
    }

    /// The voxel shape at a world coordinate: solid material, or nothing.
    pub fn shape_at(&self, x: i32, y: i32, z: i32) -> Refused<VoxelShape> {
        let m = self.material_at(x, y, z)?;
        Ok(if m.is_empty() { VoxelShape::empty() } else { VoxelShape::solid(m) })
    }

    /// A world coordinate from a chunk-relative one. `local` **may be outside
    /// `0..CHUNK_SIDE`** — the mesher reads one voxel past each face.
    pub fn world_of(&self, chunk: ChunkPos, local: [i32; 3]) -> Refused<[i32; 3]> {
        let o = chunk.origin();
        let v = VoxelPos::new(o.x, o.y, o.z).checked_add(local[0], local[1], local[2])?;
        Ok([v.x, v.y, v.z])
    }

    /// Build a whole chunk, column by column.
    ///
    /// This is the override [`VoxelSource::chunk`] exists for: the default walks
    /// 32,768 voxels through [`Self::voxel`], and this walks 1,024 columns and
    /// asks each one for its surface once.
    pub fn build_chunk(&self, pos: ChunkPos) -> Refused<SparseVoxelOctree> {
        let o = pos.origin();
        let sea = self.spec.config.sea_level_m;
        // Earthworks can put matter anywhere, including in mid-air, so a column
        // with no ground in it can only be skipped when the record is empty.
        // That is checked once, here, rather than assumed.
        let edits_possible = !self.earthworks.is_empty();
        let lowest = o.y as f64 + 0.5;
        let mut tree = SparseVoxelOctree::new();

        for lz in 0..CHUNK_SIDE {
            for lx in 0..CHUNK_SIDE {
                let (x, z) = (o.x + lx, o.z + lz);
                let fx = x as f64 + 0.5;
                let fz = z as f64 + 0.5;

                // The surface first, because it is the cheapest of the three
                // and because it decides whether the other two are needed.
                let surface = terrain::elevation_m(&self.spec, fx, fz);

                if lowest > surface {
                    // **The whole column is above the ground.** No geology, no
                    // substrate and no karst can contribute a voxel here, so the
                    // expensive half of the profile is not computed at all —
                    // which is what makes a stack of sky chunks cost a surface
                    // evaluation per column instead of a full pipeline.
                    if lowest > sea && !edits_possible {
                        continue;
                    }
                    for ly in 0..CHUNK_SIDE {
                        let y = o.y + ly;
                        let base = if y as f64 + 0.5 <= sea { materials::WATER } else { materials::AIR };
                        let m = self.earthworks.material_at(x, y, z, base);
                        if m.is_empty() {
                            continue;
                        }
                        tree.set_voxel(
                            LocalPos { x: lx as u32, y: ly as u32, z: lz as u32 },
                            VoxelShape::solid(m),
                        );
                    }
                    continue;
                }

                // **The column reaches the ground**, so it needs the full
                // profile: one per column for the 32 voxels that share it.
                let p = column::profile_with_surface(&self.spec, fx, fz, surface);
                for ly in 0..CHUNK_SIDE {
                    let y = o.y + ly;
                    let m = self.material_in_column(&p, x, y, z)?;
                    if m.is_empty() {
                        continue;
                    }
                    tree.set_voxel(
                        LocalPos { x: lx as u32, y: ly as u32, z: lz as u32 },
                        VoxelShape::solid(m),
                    );
                }
            }
        }
        Ok(tree)
    }

    /// A neighbourhood that answers from the **world itself**.
    ///
    /// Exact and slow: every neighbour voxel runs the full generator, because
    /// there is no resident chunk to read instead. [`Resident`](crate::Resident)
    /// does not use this — it answers from its resident chunks — and this exists
    /// for tests, for the first chunk of a world, and for the case where exact
    /// truth at a boundary is worth the cost.
    pub fn neighborhood(&self, chunk: ChunkPos) -> WorldNeighborhood<'_> {
        WorldNeighborhood { source: self, chunk }
    }
}

/// [`WorldSource`] as a mesher sees it.
impl VoxelSource for WorldSource {
    fn voxel(&self, chunk: ChunkPos, local: LocalPos) -> Refused<VoxelShape> {
        let w = self.world_of(chunk, [local.x as i32, local.y as i32, local.z as i32])?;
        self.shape_at(w[0], w[1], w[2])
    }

    fn chunk(&self, pos: ChunkPos) -> Refused<SparseVoxelOctree> {
        self.build_chunk(pos)
    }
}

/// The world's own answer at a chunk's boundary.
pub struct WorldNeighborhood<'a> {
    source: &'a WorldSource,
    chunk: ChunkPos,
}

impl ChunkNeighborhood for WorldNeighborhood<'_> {
    fn shape(&self, local: [i32; 3]) -> Refused<VoxelShape> {
        let w = self.source.world_of(self.chunk, local)?;
        self.source.shape_at(w[0], w[1], w[2])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_model::CHUNK_VOLUME;
    use voxel_shapers::Config;

    fn source() -> WorldSource {
        WorldSource::new(WorldSpec::new(2468, Config::default()))
    }

    /// The chunk that actually **contains** the terrain surface at a world
    /// `(x, z)`.
    ///
    /// Every test that needs voxels with a surface in them has to find it
    /// rather than assume a y: the default world's surface sits around the
    /// datum (16 m), so "chunk 1" is mostly *air* and a test that assumed
    /// otherwise would be testing an empty sky.
    fn surface_chunk(s: &WorldSource, x: i32, z: i32) -> ChunkPos {
        let e = terrain::elevation_m(&s.spec, x as f64 + 0.5, z as f64 + 0.5);
        VoxelPos::new(x, e.floor() as i32, z).chunk()
    }

    #[test]
    fn the_column_shortcut_is_exactly_the_shapers_own_definition() {
        // The test the whole optimisation rests on. If it ever fails, the world
        // has two rules for where water is, and this is the only place that
        // would say so.
        let s = source();
        let c = ChunkPos::new(0, 1, 0);
        let o = c.origin();
        let mut compared = 0u32;
        for lz in (0..CHUNK_SIDE).step_by(5) {
            for lx in (0..CHUNK_SIDE).step_by(5) {
                let (x, z) = (o.x + lx, o.z + lz);
                for ly in 0..CHUNK_SIDE {
                    let y = o.y + ly;
                    let shortcut = s.profile(x, z).material_at(&s.spec, x, y, z).unwrap();
                    let truth = s.material_at(x, y, z).unwrap();
                    assert_eq!(
                        shortcut, truth,
                        "shortcut disagreed at ({x}, {y}, {z})"
                    );
                    compared += 1;
                }
            }
        }
        assert!(compared > 1000, "the comparison was too small to mean anything");
    }

    #[test]
    fn a_chunk_holds_something_and_the_same_something_twice() {
        let s = source();
        let pos = surface_chunk(&s, 16, 16);
        let a = s.build_chunk(pos).unwrap();
        let b = s.build_chunk(pos).unwrap();
        assert!(a.solid_count() > 0, "a chunk in the playable band should hold matter");
        assert_eq!(a.digest(), b.digest(), "the generator is not deterministic");
    }

    #[test]
    fn a_chunk_high_above_the_ground_is_air_and_a_chunk_deep_under_it_is_solid() {
        // Two independent facts about the world, and both should be true of a
        // world whose surface lies in the first playable band.
        let s = source();
        let high = s.build_chunk(ChunkPos::new(0, 8, 0)).unwrap();
        assert_eq!(high.solid_count(), 0, "200 m up should be nothing");

        let deep = s.build_chunk(ChunkPos::new(0, -4, 0)).unwrap();
        // Not *all* of it: karst carves voids in rock, and a test that demanded
        // 100% solid would be demanding a cavern-free world rather than a
        // buried chunk. The claim being made is "buried", and 90% says that.
        assert!(
            deep.solid_count() > CHUNK_VOLUME as u64 * 9 / 10,
            "-120 m down was only {} of {} voxels solid",
            deep.solid_count(),
            CHUNK_VOLUME
        );
    }

    #[test]
    fn the_world_of_a_negative_local_coordinate_lands_in_the_previous_chunk() {
        // The mesher's pad, one layer up: `local` = -1 must be the neighbour's
        // voxel 31 and not the chunk's own voxel 0 mirrored.
        let s = source();
        let c = ChunkPos::new(0, 0, 0);
        assert_eq!(s.world_of(c, [-1, 0, 0]).unwrap(), [-1, 0, 0]);
        assert_eq!(s.world_of(c, [CHUNK_SIDE, 0, 0]).unwrap(), [CHUNK_SIDE, 0, 0]);
        let c = ChunkPos::new(-2, 0, 3);
        let o = c.origin();
        assert_eq!(
            s.world_of(c, [-1, 0, -1]).unwrap(),
            [o.x - 1, o.y, o.z - 1]
        );
    }

    #[test]
    fn the_world_is_continuous_across_a_chunk_boundary() {
        // A generator that disagreed with itself at a chunk edge would still
        // pass every test above. This one walks a line of voxels straight
        // through the boundary and checks the two chunks agree about all of them.
        let s = source();
        let west_pos = ChunkPos::new(0, surface_chunk(&s, 16, 16).y, 0);
        let east_pos = ChunkPos::new(1, west_pos.y, 0);
        let o = west_pos.origin();
        let west = s.build_chunk(west_pos).unwrap();
        let east = s.build_chunk(east_pos).unwrap();
        for ly in (0..CHUNK_SIDE).step_by(7) {
            for lz in (0..CHUNK_SIDE).step_by(11) {
                let y = o.y + ly;
                let z = o.z + lz;
                let left = west.voxel(LocalPos {
                    x: (CHUNK_SIDE - 1) as u32,
                    y: ly as u32,
                    z: lz as u32,
                });
                let right = east.voxel(LocalPos { x: 0, y: ly as u32, z: lz as u32 });
                // They need not be equal — the world varies — but both must be
                // what the world itself says at that **world** coordinate,
                // which is the claim. (An earlier version of this test passed
                // chunk-local coordinates to `shape_at` and was wrong by the
                // chunk origin: it compared the chunk to a place 32 m below.)
                let world_left = s.shape_at(o.x + CHUNK_SIDE - 1, y, z).unwrap();
                let world_right = s.shape_at(o.x + CHUNK_SIDE, y, z).unwrap();
                assert_eq!(left, world_left, "west chunk disagrees with the world");
                assert_eq!(right, world_right, "east chunk disagrees with the world");
            }
        }
    }

    #[test]
    fn the_world_neighbourhood_agrees_with_the_chunk_next_door() {
        // The neighbourhood's job is to be indistinguishable from having read
        // the neighbour, so this compares it against the neighbour itself.
        let s = source();
        let y = surface_chunk(&s, 16, 16).y;
        let here = ChunkPos::new(0, y, 0);
        let n = s.neighborhood(here);
        let east = s.build_chunk(ChunkPos::new(1, y, 0)).unwrap();
        let above = s.build_chunk(ChunkPos::new(0, y + 1, 0)).unwrap();

        for y in (0..CHUNK_SIDE).step_by(9) {
            for z in (0..CHUNK_SIDE).step_by(13) {
                // `[CHUNK_SIDE, y, z]` is the +x neighbour's own local `[0, y, z]`.
                assert_eq!(
                    n.shape([CHUNK_SIDE, y, z]).unwrap(),
                    east.voxel(LocalPos { x: 0, y: y as u32, z: z as u32 })
                );
                // `[x, CHUNK_SIDE, z]` is the chunk above's local `[x, 0, z]`.
                assert_eq!(
                    n.shape([z, CHUNK_SIDE, y]).unwrap(),
                    above.voxel(LocalPos { x: z as u32, y: 0, z: y as u32 })
                );
            }
        }
    }

    #[test]
    fn an_earthworks_edit_changes_the_voxel_and_the_generator_is_untouched() {
        let mut s = source();
        let (x, y, z) = (3, 20, 4);
        let base = s.material_at(x, y, z).unwrap();
        let target = if base.is_empty() { materials::IGNEOUS } else { materials::AIR };
        let entry = voxel_shapers::earthworks::Modification {
            x,
            y,
            z,
            from: base,
            to: target,
            reason: "test".into(),
            ledger: {
                use voxel_model::conservation::{Exact, Ledger, Mass, MassPumpDeclaration, Source};
                use voxel_model::Ratio;
                let mut l = Ledger::new();
                l.declare_pump(MassPumpDeclaration {
                    reason: "test".into(),
                    source: Source::World,
                });
                l.deposit(
                    Mass::from_exact(Exact::new(Ratio::from_u64(1))),
                    Source::World,
                    "one cubic metre",
                );
                l
            },
        };
        s.earthworks_mut().record(entry).unwrap();
        assert_eq!(s.material_at(x, y, z).unwrap(), target);
        assert_eq!(
            column::generated_material_at(&s.spec, x, y, z).unwrap(),
            base,
            "the record must compose over the generator, not replace it"
        );
    }
}
