//! **The addressed level, reaching a triangle.**
//!
//! [`crate::mesh`]'s quads are planar rectangles, and every one of them is exact:
//! a rectangle is two triangles and nothing is lost. A **displaced** face is not
//! planar — its corners have moved by different amounts, each following a law
//! sampled at an address — so four corners cannot represent it at all, and the
//! surface has to be **subdivided and triangulated**. This module is that stage, and
//! it is the third layer of C-43's answer: the merge emits rectangles, the submission
//! emits triangles, and the *displaced* surface is where the triangles become
//! load-bearing rather than a formality.
//!
//! # Three facts, and the rule each one forces
//!
//! 1. **A profile is a law about a *block*, not about a quad.** `Profile::RoundColumn`
//!    describes the whole 4×4×4 face of one block, so a merged quad spanning several
//!    blocks has to be **split at block boundaries** before the law can be sampled.
//!    This module does the split; it is not offered as a refusal, because the split is
//!    the law's own domain and a quad that ignored it would bend a shape across a
//!    block it does not own.
//! 2. **A bare amplitude is uniform per `(material, face)`** — that is what a
//!    `DisplacementField` *is* — so a bare amplitude translates a whole merged
//!    rectangle without bending it. It stays **two triangles**, and it is still
//!    *exact*: this is the mechanism a cut's residual uses, and it is why a cut at
//!    96/256 is at 96/256 without a triangle being spent on it.
//! 3. **The addressed level is 256 per block**, and a block face at that resolution is
//!    65,536 cells — 131,072 triangles for one block face. Nobody wants that GPU-side,
//!    so the subdivision rate is **declared**, and a rate whose result would exceed the
//!    declared cap is **refused by name** rather than silently clamped to something the
//!    caller did not ask for.
//!
//! # What is deliberately not here yet
//!
//! A bare amplitude has a magnitude but **no shape of its own**: the field says how
//! much, the *lattice* says which way (`voxel-appearance::detail`, C-2: addressed,
//! never stored). This stage moves a bare amplitude's plane uniformly, which is exactly
//! right for a cut and is **not yet a ripple** — a `Flowstone`'s wavelength needs the
//! lattice's own height at an address, and that seam is named here rather than faked.
//!
//! # Determinism
//!
//! The order is fixed: blocks ascending in `u` then `v`, then the grid, then the two
//! triangles of each cell. Nothing here iterates a hash map, and a refined face's
//! digest therefore means something (F1).

use voxel_model::{
    DisplacementField, Face, MaterialId, Refusal, RefusalKind, Refused, SUBVOXEL_SIDE,
};

use crate::mesh::{winding_flips, Quad};

/// Subvoxels along one edge of a block — the unit a profile's law is a law *about*.
const SUBVOXELS_PER_BLOCK: f64 = SUBVOXEL_SIDE as f64;

/// How finely displaced geometry is subdivided, and the bound that keeps it sane.
///
/// There is no `Default`: the rate is a **declared** property of the pipeline, the
/// same discipline sea level and the worker count are held to. A default would be a
/// number nobody chose being spent on every block face of a world.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RefineConfig {
    /// Vertices along one edge of a **block's** face, so `n` cells and `n + 1`
    /// samples per side. `4` is a vertex every subvoxel; `1` is a vertex per block
    /// face, which cannot show a cap and is allowed only so the coarse end of the
    /// range is reachable and testable.
    pub samples_per_block: u16,
    /// The most triangles one quad may refine to. Past it the quad is **refused**,
    /// with its extent in the message, because the honest answer to "this face would
    /// cost 131,072 triangles" is to say so rather than to halve the resolution.
    pub max_triangles_per_quad: u32,
}

impl RefineConfig {
    /// The finest subdivision there is: one sample per addressed cell, 256 per block.
    pub const ADDRESSED_RESOLUTION: u16 = 256;

    pub fn new(samples_per_block: u16, max_triangles_per_quad: u32) -> Refused<Self> {
        if samples_per_block == 0 || samples_per_block > Self::ADDRESSED_RESOLUTION {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!(
                    "a subdivision of {samples_per_block} samples per block face is outside 1..={}; zero samples has no samples and the addressed level has no finer one",
                    Self::ADDRESSED_RESOLUTION
                ),
            ));
        }
        if max_triangles_per_quad < 2 {
            return Err(Refusal::new(
                RefusalKind::OutOfDeclaredRange,
                "a cap below two triangles cannot hold even a planar quad, so it is not a cap",
            ));
        }
        Ok(RefineConfig { samples_per_block, max_triangles_per_quad })
    }

    /// Triangles one whole block face costs at this rate: `2·n²`.
    pub fn triangles_per_block_face(&self) -> u64 {
        2 * self.samples_per_block as u64 * self.samples_per_block as u64
    }
}

/// A triangle of the surface, in **chunk-local subvoxels**, with the normal the
/// displaced law gives at each vertex.
///
/// Both are `f32` because this is geometry headed for a vertex buffer; the chunk origin
/// is applied per chunk by the renderer and never folded into a world-space `f32`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Triangle {
    pub material: MaterialId,
    pub vertices: [Vertex; 3],
}

/// What the refinement did, so "the 256 level costs triangles" is a measured number.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct RefineStats {
    /// Quads handed in — always 1 for [`refine`], kept so a batch can add these up.
    pub quads: u64,
    pub triangles: u64,
    /// Block faces the quad was split into. One for a quad inside one block, and more
    /// when a merged quad crossed a block boundary.
    pub block_faces: u64,
    /// Quads that needed more than the two triangles a plane needs.
    pub subdivided_quads: u64,
    /// Vertices whose position differs from the undisplaced quad's corner.
    pub displaced_vertices: u64,
}

impl RefineStats {
    /// Triangles per quad in, against the undisplaced cost of exactly `2.0`.
    pub fn triangles_per_quad(&self) -> f64 {
        if self.quads == 0 {
            0.0
        } else {
            self.triangles as f64 / self.quads as f64
        }
    }

    pub fn is_subdivided(&self) -> bool {
        self.subdivided_quads > 0
    }
}

/// A quad's refined geometry.
#[derive(Clone, PartialEq, Debug)]
pub struct RefinedFace {
    pub triangles: Vec<Triangle>,
    pub stats: RefineStats,
}

/// **Refine one quad**: two triangles if nothing displaces it, and a subdivided,
/// displaced, triangulated surface if something does.
pub fn refine(quad: &Quad, field: &DisplacementField, config: &RefineConfig) -> Refused<RefinedFace> {
    let Some(entry) = field.entry(quad.material, quad.face) else {
        return Ok(planar(quad, [0.0; 4]));
    };
    if entry.amplitude_subvoxels == 0.0 && entry.profile.is_none() {
        return Ok(planar(quad, [0.0; 4]));
    }
    let Some(profile) = entry.profile else {
        // A bare amplitude is uniform across the quad, so the surface stays a plane and
        // stays two triangles. Exact, and the reason a cut costs no subdivision.
        let offset = entry.amplitude_subvoxels;
        return Ok(planar(quad, [offset; 4]));
    };

    let offset = |fu: f64, fv: f64| -> Refused<(f64, [f64; 3])> {
        let relief = entry.displacement_at(fu, fv)?;
        let normal = profile.cap_normal(quad.face, fu, fv)?;
        Ok((relief, normal))
    };

    // The blocks the quad covers, along each in-plane axis, and the pieces of this
    // quad that fall inside each of them.
    let pieces_u = pieces(quad.uv[0], quad.extent[0]);
    let pieces_v = pieces(quad.uv[1], quad.extent[1]);
    let mut cells_u = Vec::with_capacity(pieces_u.len());
    for piece in &pieces_u {
        cells_u.push(cells_for(piece, config.samples_per_block));
    }
    let mut cells_v = Vec::with_capacity(pieces_v.len());
    for piece in &pieces_v {
        cells_v.push(cells_for(piece, config.samples_per_block));
    }

    let total: u64 = pieces_u
        .iter()
        .zip(&cells_u)
        .map(|(_, cu)| 2 * *cu as u64)
        .sum::<u64>()
        * pieces_v.iter().zip(&cells_v).map(|(_, cv)| *cv as u64).sum::<u64>();
    if total > config.max_triangles_per_quad as u64 {
        return Err(Refusal::owned(
            RefusalKind::OutOfDeclaredRange,
            format!(
                "refining a {}x{} subvoxel {:?} face at {} samples per block would cost {total} triangles, past the declared cap of {}; the cap is refused rather than lowered, because lowering it would change the shape the caller asked for",
                quad.extent[0],
                quad.extent[1],
                quad.face,
                config.samples_per_block,
                config.max_triangles_per_quad
            ),
        ));
    }

    let flip = winding_flips(quad.face);
    let mut triangles = Vec::new();
    let mut displaced = 0u64;
    let mut block_faces = 0u64;

    for (piece_u, &cu) in pieces_u.iter().zip(&cells_u) {
        for (piece_v, &cv) in pieces_v.iter().zip(&cells_v) {
            block_faces += 1;
            // The piece's grid, in the block's own fractions. The endpoints are inside
            // `[0, 1]` by construction, and only they are interpolated — so no clamped
            // coordinate ever reaches the profile.
            let mut grid = Vec::with_capacity((cu as usize + 1) * (cv as usize + 1));
            for i in 0..=cu {
                let fu = lerp(piece_u.f0, piece_u.f1, i, cu);
                for j in 0..=cv {
                    let fv = lerp(piece_v.f0, piece_v.f1, j, cv);
                    let (relief, normal) = offset(fu, fv)?;
                    if relief != 0.0 {
                        displaced += 1;
                    }
                    grid.push(Vertex {
                        position: position(quad, piece_u.block, piece_v.block, fu, fv, relief),
                        normal: [normal[0] as f32, normal[1] as f32, normal[2] as f32],
                    });
                }
            }
            let stride = cu as usize + 1;
            for i in 0..cu as usize {
                for j in 0..cv as usize {
                    let base = [
                        i * stride + j,
                        (i + 1) * stride + j,
                        (i + 1) * stride + j + 1,
                        i * stride + j + 1,
                    ];
                    let order = if flip {
                        [base[3], base[2], base[1], base[0]]
                    } else {
                        base
                    };
                    for [a, b, c] in [[order[0], order[1], order[2]], [order[0], order[2], order[3]]] {
                        triangles.push(Triangle {
                            material: quad.material,
                            vertices: [grid[a], grid[b], grid[c]],
                        });
                    }
                }
            }
        }
    }

    Ok(RefinedFace {
        stats: RefineStats {
            quads: 1,
            triangles: triangles.len() as u64,
            block_faces,
            subdivided_quads: 1,
            displaced_vertices: displaced,
        },
        triangles,
    })
}

/// The two triangles every planar quad becomes, optionally offset along its own normal.
///
/// The corner order is [`Quad::corners`]'s, which is already wound outwards, so this
/// emits the same `0,1,2 / 0,2,3` the submission path already emits — an undisplaced
/// face refines to *exactly* the geometry it had, which is what makes the refined and
/// unrefined paths comparable rather than merely similar.
fn planar(quad: &Quad, offsets: [f64; 4]) -> RefinedFace {
    let n = normal_f64(quad.face);
    let corners = quad.corners();
    let vertices: Vec<Vertex> = corners
        .iter()
        .zip(offsets)
        .map(|(corner, offset)| Vertex {
            position: [
                (corner[0] as f64 + n[0] * offset) as f32,
                (corner[1] as f64 + n[1] * offset) as f32,
                (corner[2] as f64 + n[2] * offset) as f32,
            ],
            normal: [n[0] as f32, n[1] as f32, n[2] as f32],
        })
        .collect();
    let displaced = offsets.iter().filter(|o| **o != 0.0).count() as u64;
    RefinedFace {
        triangles: vec![
            Triangle { material: quad.material, vertices: [vertices[0], vertices[1], vertices[2]] },
            Triangle { material: quad.material, vertices: [vertices[0], vertices[2], vertices[3]] },
        ],
        stats: RefineStats {
            quads: 1,
            triangles: 2,
            block_faces: 1,
            subdivided_quads: 0,
            displaced_vertices: displaced,
        },
    }
}

/// One block's share of a quad's span along one in-plane axis: which block, and which
/// part of it, as block-local fractions.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Piece {
    block: u16,
    f0: f64,
    f1: f64,
}

/// The blocks a `lo..lo + width` subvoxel span covers, and the fraction of each it
/// covers. A quad inside one block yields exactly one piece.
fn pieces(lo: u16, width: u16) -> Vec<Piece> {
    let mut out = Vec::new();
    if width == 0 {
        return out;
    }
    let per = SUBVOXEL_SIDE as u16;
    let first = lo / per;
    let last = (lo + width - 1) / per;
    for block in first..=last {
        let block_lo = block * per;
        let a0 = lo.max(block_lo);
        let a1 = (lo + width).min(block_lo + per);
        out.push(Piece {
            block,
            f0: (a0 - block_lo) as f64 / SUBVOXELS_PER_BLOCK,
            f1: (a1 - block_lo) as f64 / SUBVOXELS_PER_BLOCK,
        });
    }
    out
}

/// How many cells one piece is sampled in: the declared density, clipped to the piece,
/// rounded **up** so a piece is never empty and a partial block face keeps the density
/// of a full one.
fn cells_for(piece: &Piece, samples_per_block: u16) -> u16 {
    let covered = (piece.f1 - piece.f0) * SUBVOXELS_PER_BLOCK;
    let cells = (covered * samples_per_block as f64 / SUBVOXELS_PER_BLOCK).ceil();
    (cells as u16).max(1)
}

fn lerp(a: f64, b: f64, i: u16, n: u16) -> f64 {
    if n == 0 {
        return a;
    }
    a + (b - a) * (i as f64 / n as f64)
}

/// A vertex's position: the quad's plane, in the block-local fraction the law was
/// sampled at, moved along the face's normal by the displacement.
fn position(quad: &Quad, block_u: u16, block_v: u16, fu: f64, fv: f64, relief: f64) -> [f32; 3] {
    let (au, av) = in_plane_axes_of(quad.face);
    let mut p = [0.0f64; 3];
    p[quad.face.axis() as usize] = quad.slice as f64;
    for axis in 0..3 {
        p[axis] += au[axis] * (block_u as f64 * SUBVOXELS_PER_BLOCK + fu * SUBVOXELS_PER_BLOCK);
        p[axis] += av[axis] * (block_v as f64 * SUBVOXELS_PER_BLOCK + fv * SUBVOXELS_PER_BLOCK);
    }
    let n = normal_f64(quad.face);
    for axis in 0..3 {
        p[axis] += n[axis] * relief;
    }
    [p[0] as f32, p[1] as f32, p[2] as f32]
}

/// The `(u, v)` axis vectors of a face, matching [`voxel_model::in_plane_axes`]: axis 0
/// is `(y, z)`, axis 1 is `(x, z)` and axis 2 is `(x, y)`. Pinned by a test, because the
/// mesh's `cell()` and the profile's law have to agree about which way is `u`.
fn in_plane_axes_of(face: Face) -> ([f64; 3], [f64; 3]) {
    match face.axis() {
        0 => ([0.0, 1.0, 0.0], [0.0, 0.0, 1.0]),
        1 => ([1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
        _ => ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
    }
}

fn normal_f64(face: Face) -> [f64; 3] {
    let (x, y, z) = face.normal();
    [x as f64, y as f64, z as f64]
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_model::Profile;

    const ROCK: MaterialId = MaterialId(3);
    /// A round column inscribed in a block: the pillar's own profile, whose cap is a
    /// hemisphere of radius 0.5 blocks.
    const PILLAR: Profile = Profile::RoundColumn { radius: 0.5 };

    fn quad(face: Face, slice: u16, uv: [u16; 2], extent: [u16; 2]) -> Quad {
        Quad { face, slice, uv, extent, material: ROCK }
    }

    fn config(samples: u16) -> RefineConfig {
        RefineConfig::new(samples, 4096).unwrap()
    }

    /// Every vertex a refined face emitted, in order.
    fn vertices(face: &RefinedFace) -> Vec<Vertex> {
        face.triangles.iter().flat_map(|t| t.vertices.iter().copied()).collect()
    }

    fn vertex_at(face: &RefinedFace, x: f32, y: f32) -> Vertex {
        *vertices(face)
            .iter()
            .find(|v| (v.position[0] - x).abs() < 1e-4 && (v.position[1] - y).abs() < 1e-4)
            .unwrap_or_else(|| panic!("no vertex at ({x}, {y})"))
    }

    #[test]
    fn an_undisplaced_face_is_two_triangles_and_that_is_the_measure_to_beat() {
        let field = DisplacementField::none();
        let q = quad(Face::PosZ, 8, [4, 4], [4, 4]);
        let out = refine(&q, &field, &config(4)).unwrap();
        assert_eq!(out.stats.triangles, 2);
        assert_eq!(out.stats.triangles_per_quad(), 2.0);
        assert!(!out.stats.is_subdivided());
        assert_eq!(out.stats.block_faces, 1);
        assert_eq!(out.stats.displaced_vertices, 0);
        // And the triangles are the quad's own corners, in the order the submission path
        // already emits (0,1,2 / 0,2,3), with the face's own normal.
        let corners = q.corners();
        for (i, corner) in corners.iter().enumerate() {
            let want = [corner[0] as f32, corner[1] as f32, corner[2] as f32];
            let got = if i < 3 {
                out.triangles[0].vertices[i]
            } else {
                out.triangles[1].vertices[2]
            };
            assert_eq!(got.position, want, "corner {i}");
            assert_eq!(got.normal, [0.0, 0.0, 1.0]);
        }
    }

    #[test]
    fn a_bare_amplitude_translates_a_plane_and_still_costs_two_triangles() {
        // The cut's mechanism. A residual is uniform per `(material, face)` — that is
        // what a displacement field *is* — so a whole merged rectangle moves by it and
        // the surface stays a plane. A cut at 96/256 costs no subdivision, and it is
        // still exact.
        let mut field = DisplacementField::none();
        field.declare(ROCK, Face::NegZ, -0.5).unwrap();
        let q = quad(Face::NegZ, 8, [0, 0], [128, 128]);
        let out = refine(&q, &field, &config(4)).unwrap();
        assert_eq!(out.stats.triangles, 2);
        assert!(!out.stats.is_subdivided());
        assert_eq!(out.stats.displaced_vertices, 4);
        for t in &out.triangles {
            for v in &t.vertices {
                assert_eq!(v.normal, [0.0, 0.0, -1.0], "a translated plane is still a plane");
            }
        }
        // Inward on a negative face is `+z`, and the move is exactly the amplitude.
        let corner = q.corners()[0];
        let got = out.triangles[0].vertices[0].position;
        assert!((got[2] - (corner[2] as f32 + 0.5)).abs() < 1e-6, "{got:?}");
    }

    #[test]
    fn a_profiled_face_subdivides_and_the_law_is_sampled_and_not_approximated() {
        let mut field = DisplacementField::none();
        field.declare_profile(ROCK, Face::PosZ, PILLAR).unwrap();
        let q = quad(Face::PosZ, 8, [4, 4], [4, 4]);
        let out = refine(&q, &field, &config(4)).unwrap();
        assert_eq!(out.stats.triangles, 32, "4x4 cells per block face, two triangles each");
        assert_eq!(out.stats.block_faces, 1);
        assert!(out.stats.is_subdivided());
        assert_eq!(out.stats.triangles_per_quad(), 32.0);

        // The apex is the profile's own cap height at the axis, in subvoxels: the
        // hemisphere stands `cap_height(0) · 4 = 2` subvoxels above the plane at the
        // block face's centre, and the cap's normal there *is* the face's normal.
        let apex = vertex_at(&out, 6.0, 6.0);
        assert!((apex.position[2] - 10.0).abs() < 1e-6, "{apex:?}");
        assert_eq!(apex.normal, [0.0, 0.0, 1.0]);

        // The quad's own corners are outside the inscribed circle, where the law has
        // already fallen away: they stay on the plane and their normal has turned to
        // face sideways, which is the limit the profile gives and not a clamp.
        let rim = vertex_at(&out, 4.0, 4.0);
        assert!((rim.position[2] - 8.0).abs() < 1e-6, "{rim:?}");
        assert!(rim.normal[2].abs() < 1e-6, "{rim:?}");
        let radial = (rim.normal[0] * rim.normal[0] + rim.normal[1] * rim.normal[1]).sqrt();
        assert!((radial - 1.0).abs() < 1e-6, "{rim:?}");

        // And the grid's second corner is the quad's second corner, which is what pins
        // the mesh's `(u, v)` order and the profile's together: transpose the axes and
        // this vertex would be at (4, 8) instead.
        let corner = q.corners()[1];
        let second = vertex_at(&out, corner[0] as f32, corner[1] as f32);
        assert_eq!(second.position, [corner[0] as f32, corner[1] as f32, corner[2] as f32]);
    }

    #[test]
    fn a_merged_quad_is_split_at_block_boundaries_because_the_law_is_per_block() {
        let mut field = DisplacementField::none();
        field.declare_profile(ROCK, Face::PosZ, PILLAR).unwrap();
        // Two blocks wide, one block deep: the merge crossed a boundary the law does not
        // span, so the refinement splits it into two block faces — each with its own
        // apex, rather than one bend stretched across both.
        let q = quad(Face::PosZ, 8, [4, 4], [8, 4]);
        let out = refine(&q, &field, &config(4)).unwrap();
        assert_eq!(out.stats.block_faces, 2);
        assert_eq!(out.stats.triangles, 64);
        // One apex per block face and not one bend stretched across both: the apexes are
        // distinct column positions, `6` and `10` subvoxels apart by four.
        let mut apexes: Vec<(i32, i32)> = vertices(&out)
            .iter()
            .filter(|v| v.position[2] > 9.999)
            .map(|v| (v.position[0] as i32, v.position[1] as i32))
            .collect();
        apexes.sort_unstable();
        apexes.dedup();
        assert_eq!(apexes, vec![(6, 6), (10, 6)], "one apex per block face");
    }

    #[test]
    fn a_resolution_past_the_cap_is_refused_by_name_and_not_lowered() {
        let mut field = DisplacementField::none();
        field.declare_profile(ROCK, Face::PosZ, PILLAR).unwrap();
        // A whole chunk's floor at 8 samples per block: 32×32 block faces of 128
        // triangles each. The honest answer is the number, not a quieter resolution.
        let q = quad(Face::PosZ, 8, [0, 0], [128, 128]);
        let err = refine(&q, &field, &config(8)).unwrap_err();
        assert_eq!(err.kind(), RefusalKind::OutOfDeclaredRange);
        assert!(err.detail().contains("131072"), "{}", err.detail());
        assert!(err.detail().contains("128x128"), "{}", err.detail());
    }

    #[test]
    fn a_config_refuses_a_rate_with_no_samples_and_a_cap_that_cannot_hold_a_triangle() {
        assert!(RefineConfig::new(0, 64).is_err());
        assert!(RefineConfig::new(257, 64).is_err());
        assert!(RefineConfig::new(4, 1).is_err());
        assert_eq!(RefineConfig::new(4, 32).unwrap().triangles_per_block_face(), 32);
        // The addressed level's own resolution, priced rather than described.
        assert_eq!(
            RefineConfig::new(256, 200_000).unwrap().triangles_per_block_face(),
            131_072
        );
    }

    #[test]
    fn the_meshs_face_axes_and_the_profiles_law_agree_about_which_way_is_u() {
        // Two statements of one convention, which is one statement too many unless they
        // are checked: a transposed `(u, v)` rotates every shape ninety degrees and looks
        // like a lighting bug for a week.
        for face in Face::ALL {
            assert_eq!(in_plane_axes_of(face), voxel_model::in_plane_axes(face), "{face:?}");
        }
    }

    #[test]
    fn every_refined_normal_is_a_unit_vector_and_leans_outward() {
        for face in Face::ALL {
            let mut field = DisplacementField::none();
            field.declare_profile(ROCK, face, PILLAR).unwrap();
            let q = quad(face, 8, [4, 4], [4, 4]);
            let out = refine(&q, &field, &config(4)).unwrap();
            let n = normal_f64(face);
            for t in &out.triangles {
                for v in &t.vertices {
                    let length = (v.normal[0] * v.normal[0]
                        + v.normal[1] * v.normal[1]
                        + v.normal[2] * v.normal[2])
                        .sqrt();
                    assert!((length - 1.0).abs() < 1e-6, "{face:?} normal {v:?}");
                    let dot = v.normal[0] as f64 * n[0]
                        + v.normal[1] as f64 * n[1]
                        + v.normal[2] as f64 * n[2];
                    assert!(dot >= -1e-9, "{face:?} normal leans into the rock: {dot}");
                }
            }
        }
    }
}
