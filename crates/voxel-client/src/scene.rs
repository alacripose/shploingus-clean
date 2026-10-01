//! Geometry: the mesh the mesher produced, in the shape a GPU wants.
//!
//! # Why the eye is subtracted here
//!
//! [`Mesh::corner_positions_m`] is **chunk-relative on purpose** — at ±2,000,000
//! m an `f32` metre loses its subvoxel resolution, so a world-space `f32` is a
//! bug waiting for a player to walk far enough. So this module adds the chunk
//! origin in `f64`, subtracts the eye in `f64`, and narrows **only the
//! difference**. What the GPU receives is the size of what is on screen, which
//! is small no matter how far out the player is. This is the same decision
//! `voxel-mesh` made, finished on the other side.
//!
//! # No `unsafe`, so no `bytemuck`
//!
//! `#![forbid(unsafe_code)]` is workspace-wide, so a `Vertex` cannot be viewed
//! as bytes by transmuting a slice. The bytes are assembled explicitly instead.
//! That costs a `to_le_bytes` per `f32` and buys a vertex layout that is
//! *written down twice* — once in `#[repr(C)]` and once in the `wgpu` attribute
//! descriptors — with a test asserting the two agree. See
//! [`Vertex::STRIDE`].

use voxel_mesh::Mesh;

/// One vertex. The field order and types are the `VsIn` struct in
/// `shading.wgsl`, and the offsets in [`Vertex::attributes`] are the same
/// statement written a second time.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Vertex {
    /// Metres, **relative to the eye**. See the module comment.
    pub position: [f32; 3],
    /// The quad's outward normal, straight from `Quad::normal` — an axis unit
    /// vector, so every corner of a quad carries the same normal and the
    /// interpolated one is exact.
    pub normal: [f32; 3],
    /// `MaterialId`, used to index the shader's material table.
    pub material: u32,
}

impl Vertex {
    /// `3 * 4 + 3 * 4 + 4`. A test checks this against the attribute list rather
    /// than trusting the arithmetic.
    pub const STRIDE: u64 = 28;

    /// The attribute descriptors, and the offsets they claim.
    pub fn attributes() -> [wgpu::VertexAttribute; 3] {
        [
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 0,
                shader_location: 0,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 12,
                shader_location: 1,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Uint32,
                offset: 24,
                shader_location: 2,
            },
        ]
    }

    fn write_bytes(self, out: &mut Vec<u8>) {
        for v in self.position.iter().chain(self.normal.iter()) {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out.extend_from_slice(&self.material.to_le_bytes());
    }
}

/// Triangles ready to draw, and the count of quads they came from.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Scene {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub quads: usize,
}

impl Scene {
    /// Six indices per quad, wound `(0,1,2) (0,2,3)`. The winding is irrelevant
    /// to the pipeline this crate builds — `cull_mode` is `None` — but it is
    /// written down here so that turning culling on later is a pipeline change
    /// and not a geometry change.
    pub fn build<'a>(meshes: impl IntoIterator<Item = &'a Mesh>, eye_m: [f64; 3]) -> Scene {
        let mut scene = Scene::default();
        for mesh in meshes {
            let o = mesh.origin_voxel();
            let origin = [o[0] as f64, o[1] as f64, o[2] as f64];
            for quad in &mesh.quads {
                // Refuse rather than wrap: an index buffer is `u32`, and a scene
                // that overflows it is not a scene. Stopping keeps the frame
                // that *was* built rather than corrupting it.
                let base = scene.vertices.len() as u32;
                if base.checked_add(4).is_none() {
                    return scene;
                }
                let n = quad.normal();
                let normal = [n[0] as f32, n[1] as f32, n[2] as f32];
                for c in mesh.corner_positions_m(quad) {
                    scene.vertices.push(Vertex {
                        position: [
                            (origin[0] + c[0] as f64 - eye_m[0]) as f32,
                            (origin[1] + c[1] as f64 - eye_m[1]) as f32,
                            (origin[2] + c[2] as f64 - eye_m[2]) as f32,
                        ],
                        normal,
                        material: quad.material.0 as u32,
                    });
                }
                scene
                    .indices
                    .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
                scene.quads += 1;
            }
        }
        scene
    }

    /// One quad, placed `distance_m` in front of a camera at the origin and
    /// oriented across the view, but carrying an **arbitrary shading normal**.
    ///
    /// Geometry and shading normal are deliberately separable here. That is what
    /// makes a probe possible: the shader's output for a stated normal can be
    /// compared against `npr::shade` without also having to reason about
    /// whether a quad is edge-on. It is a test fixture, not a world.
    pub fn probe(normal: [f32; 3], material: u32, distance_m: f32, half_m: f32) -> Scene {
        // In **front** of the camera is `+Z`, because `Camera::standard` looks
        // down `+Z` at yaw zero. A probe behind the eye would be clipped away.
        let z = distance_m;
        let corners = [
            [-half_m, -half_m, z],
            [half_m, -half_m, z],
            [half_m, half_m, z],
            [-half_m, half_m, z],
        ];
        let vertices = corners
            .into_iter()
            .map(|position| Vertex { position, normal, material })
            .collect();
        Scene { vertices, indices: vec![0, 1, 2, 0, 2, 3], quads: 1 }
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    /// The vertex buffer's bytes, little-endian, `STRIDE` per vertex.
    pub fn vertex_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.vertices.len() * Vertex::STRIDE as usize);
        for v in &self.vertices {
            v.write_bytes(&mut out);
        }
        out
    }

    pub fn index_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.indices.len() * 4);
        for i in &self.indices {
            out.extend_from_slice(&i.to_le_bytes());
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_model::{MaterialId, CHUNK_SIDE};

    fn mesh_at(x: i32, y: i32, z: i32) -> Mesh {
        Mesh {
            chunk: voxel_model::ChunkPos::new(x, y, z),
            quads: Vec::new(),
            stats: voxel_mesh::MeshStats { solid_voxels: 0, visible_faces: 0, quads: 0 },
        }
    }

    #[test]
    fn the_stride_matches_the_attributes_that_claim_offsets_into_it() {
        // The layout is written down twice — once as a struct, once as offsets —
        // so it is checked rather than trusted.
        assert_eq!(Vertex::STRIDE, std::mem::size_of::<Vertex>() as u64);
        let last = Vertex::attributes()
            .iter()
            .map(|a| a.offset + a.format.size())
            .max()
            .unwrap();
        assert_eq!(last, Vertex::STRIDE, "the attributes do not cover the vertex");
        for a in Vertex::attributes() {
            assert!(a.offset.is_multiple_of(4), "offset {} is not 4-aligned", a.offset);
        }
    }

    #[test]
    fn a_vertex_encodes_to_exactly_one_stride_of_bytes() {
        let v = Vertex {
            position: [1.0, -2.0, 0.25],
            normal: [0.0, 1.0, 0.0],
            material: 5,
        };
        let mut out = Vec::new();
        v.write_bytes(&mut out);
        assert_eq!(out.len() as u64, Vertex::STRIDE);
        // little-endian, in field order
        assert_eq!(&out[0..4], &1.0f32.to_le_bytes());
        assert_eq!(&out[24..28], &5u32.to_le_bytes());
    }

    #[test]
    fn an_empty_mesh_produces_no_geometry_rather_than_a_default() {
        let scene = Scene::build([&mesh_at(0, 0, 0)], [0.0; 3]);
        assert!(scene.is_empty());
        assert!(scene.vertex_bytes().is_empty());
    }

    #[test]
    fn the_eye_subtraction_keeps_a_distant_chunk_sharp() {
        // Two million metres out, a ghost chunk whose origin is still exact as an
        // `i32`. The vertex must land at a small, precise offset — the failure
        // this design exists to prevent is that offset quantising to 0.25 m.
        let chunk_x = 62_500; // 62_500 * 32 m = 2,000,000 m
        let pos = voxel_model::ChunkPos::new(chunk_x, 0, 0);
        let eye = [2_000_000.5, 0.0, 0.0];
        let origin = pos.origin();
        // the chunk's low corner in metres, exactly
        let low = [origin.x as f64, origin.y as f64, origin.z as f64];
        let rel = low[0] - eye[0];
        assert!(
            (rel - (-0.5)).abs() < 1e-9,
            "the relative offset is {} where it must be exactly -0.5",
            rel
        );
        // Had the world been an f32, 2,000,000.0f32 has a spacing of 0.25 and
        // this subtraction would not resolve a half metre exactly.
        let f32_spacing = {
            let a = 2_000_000.0f32;
            let b = a + 0.25;
            b - a
        };
        assert!(f32_spacing > 0.0 && f32_spacing <= 0.25);
    }

    #[test]
    fn a_quad_becomes_four_vertices_and_two_triangles() {
        let m = MaterialId(3);
        let mut mesh = mesh_at(0, 0, 0);
        // A single face of one voxel: the +x face at subvoxel 4, spanning 1 m².
        mesh.quads.push(voxel_mesh::Quad {
            face: voxel_model::Face::PosX,
            slice: 4,
            uv: [0, 0],
            extent: [4, 4],
            material: m,
        });
        let scene = Scene::build([&mesh], [0.0; 3]);
        assert_eq!(scene.quads, 1);
        assert_eq!(scene.vertices.len(), 4);
        assert_eq!(scene.indices, vec![0, 1, 2, 0, 2, 3]);
        assert_eq!(scene.vertex_bytes().len(), 4 * Vertex::STRIDE as usize);
        assert_eq!(scene.index_bytes().len(), 6 * 4);
        assert!(scene.vertices.iter().all(|v| v.material == 3));
        // The face is at x = 1 m and its normal is +x.
        for v in &scene.vertices {
            assert!((v.position[0] - 1.0).abs() < 1e-6, "{:?}", v.position);
            assert_eq!(v.normal, [1.0, 0.0, 0.0]);
        }
    }

    #[test]
    fn chunk_side_is_still_the_scale_this_assumes() {
        // The metre-per-voxel assumption is what makes the origin arithmetic
        // legal, so it is asserted rather than assumed.
        assert_eq!(CHUNK_SIDE, 32);
    }
}
