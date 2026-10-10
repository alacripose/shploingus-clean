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
//!
//! # Per-face material composition (C-172)
//!
//! The quad format's rectangle now carries the face composition of each quad's
//! rectangle selection, not just the quad's own material id. The fragment in
//! `shading.wgsl` reads that composition and shades every pixel from its own
//! blend instead of from one material id. A drawn quad is one material by
//! construction, so the composition is the majority id in slot 0 and empty in
//! slot 1 — the same row the fragment used to read — and the change is invisible
//! on the current world.

use voxel_mesh::Mesh;
use voxel_appearance::DetailLattice;

use crate::quad::QuadRecord;

use voxel_model::MaterialId;

/// A callback that gives the world's light at a cell, in band order (B3, B4, B5).
/// `.0` = direct beam W/m², `.1` = diffuse (sky + one-bounce) W/m², per band.
/// This is the world source / shaper layer's job — the renderer does not own it.
///
/// **`None` is "no field here", and it is not a zero.** A cell at night, in
/// shadow, or under a lid has a beam of zero W/m² and that is a *measurement*; a
/// world whose shaper layer does not produce the field at all has no measurement
/// to give. The two reach the fragment differently — see [`Vertex::light_present`] —
/// because reading "no field" as "black" would darken every frame built before the
/// field existed.
pub type LightAt = dyn Fn(i32, i32, i32) -> Option<[f32; 6]>;

/// One vertex. The field order and types are the `VsIn` struct in
/// `shading.wgsl`, and the offsets in [`Vertex::attributes`] are the same
/// statement written a second time.
///
/// The layout is load-bearing: the shader's `VsIn` must match this exactly,
/// and the test [`Vertex::the_stride_matches_the_attributes_that_claim_offsets_into_it`]
/// holds them equal.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Vertex {
    /// Metres, **relative to the eye**. See the module comment.
    pub position: [f32; 3],
    /// The quad's outward normal, straight from `Quad::normal` — an axis unit
    /// vector, so every corner of a quad carries the same normal and the
    /// interpolated one is exact. For a refined (displaced) face the normal
    /// comes from the profile's cap normal at that corner.
    pub normal: [f32; 3],
    /// `MaterialId`, used to index the shader's material table.
    pub material: u32,
    /// The material's detail at this corner's world address: the 256-level
    /// lattice sampled once, as `roughness_delta, reflectance_delta`.
    /// The fragment shader's ladder uses this as the coarsest rung's base and
    /// blends finer rungs from the fragment's own footprint.
    pub detail_roughness: f32,
    pub detail_reflectance: f32,
    /// The world's light at this corner's cell, in band order (B3, B4, B5):
    /// direct beam, then diffuse (sky + one-bounce), in W/m². **Only meaningful
    /// when [`Vertex::light_present`] is 1** — with no field these stay zero
    /// because a vertex is `[f32]` and zero is what a missing `f32` looks like.
    pub light_direct: [f32; 3],
    pub light_diffuse: [f32; 3],
    /// **Whether the world's light field covers this corner at all**, from
    /// [`LightAt`]'s `Option`: 1 when the shaper layer gave a measurement, 0 when
    /// it had none to give. The fragment shades from the band light where this is
    /// 1; where it is 0 the shader falls back to the rig's own lights, which is
    /// the Source 2 model `npr::shade` states and what every frame drawn before
    /// this field existed was lit by. It is a flag rather than a sentinel because
    /// zero W/m² is a real daylight answer (night, shadow, a covered cell) and must
    /// not hand a lit surface back to the rig.
    pub light_present: u32,
}

impl Vertex {
    /// `3*4 + 3*4 + 4 + 4 + 4 + 3*4 + 3*4 + 4`. A test checks this against the
    /// attribute list rather than trusting the arithmetic.
    pub const STRIDE: u64 = 3 * 4 + 3 * 4 + 4 + 4 + 4 + 3 * 4 + 3 * 4 + 4;

    /// The attribute descriptors, and the offsets they claim.
    pub fn attributes() -> [wgpu::VertexAttribute; 8] {
        [
            // position
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 0,
                shader_location: 0,
            },
            // normal
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 12,
                shader_location: 1,
            },
            // material
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Uint32,
                offset: 24,
                shader_location: 2,
            },
            // detail_roughness
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32,
                offset: 28,
                shader_location: 3,
            },
            // detail_reflectance
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32,
                offset: 32,
                shader_location: 4,
            },
            // light_direct (B3, B4, B5)
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 36,
                shader_location: 5,
            },
            // light_diffuse (B3, B4, B5)
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 48,
                shader_location: 6,
            },
            // light_present
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Uint32,
                offset: 60,
                shader_location: 7,
            },
        ]
    }

    fn write_bytes(self, out: &mut Vec<u8>) {
        for v in self.position.iter().chain(self.normal.iter()) {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out.extend_from_slice(&self.material.to_le_bytes());
        out.extend_from_slice(&self.detail_roughness.to_le_bytes());
        out.extend_from_slice(&self.detail_reflectance.to_le_bytes());
        for v in self.light_direct.iter().chain(self.light_diffuse.iter()) {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out.extend_from_slice(&self.light_present.to_le_bytes());
    }
}

/// Triangles ready to draw, and the count of quads they came from.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Scene {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    /// **The world's rectangles, in the quad format** — chunk-local, camera-
    /// independent, expanded by the GPU ([`crate::quad`], `shading.wgsl`'s
    /// `vs_quad`). 24 bytes each against the 280 a four-corner vertex
    /// expansion costs, and a step of the eye does not touch them.
    pub records: Vec<QuadRecord>,
    /// One **world** origin per chunk the records name, in metres, **and the veil in `w`**.
    /// World-space rather than eye-relative: the pipeline subtracts the eye when it uploads
    /// (`quad::origin_bytes`), so the rectangles and their origins both survive a moving
    /// camera. `w` is `0.0` for a chunk of the world and a cloud cell's own transmittance for
    /// a cell (C-179) — the one float of this `vec4` that had no meaning before clouds.
    pub origins: Vec<[f64; 4]>,
    /// **How many of the trailing records are the cloud** (C-179). The world draws opaque and
    /// first; the cloud draws after it, blended, with no depth write — so the draw needs to
    /// know where its own range begins, and that is `records.len() - cloud_records`.
    pub cloud_records: usize,
    pub quads: usize,
}

impl Scene {
    /// Build the scene from meshes, with per-vertex detail and light.
    ///
    /// `lattice` is the world's 256-level detail lattice — the same one the shader
    /// reads, so the per-vertex sample and the fragment's ladder agree.
    ///
    /// **This is the quad format: no light field and no per-corner lattice
    /// sample.** A rectangle travels as a 24-byte [`QuadRecord`] and the GPU
    /// expands it, which is what takes the per-frame cost off the CPU; the
    /// fragment's own ladder is the whole address law for that geometry. A frame
    /// that carries the world's light field (a measurement per cell, which is a
    /// *corner's* quantity and so needs the vertex format) is built by
    /// [`Scene::build_with_light`], and the two are compared on pixels by
    /// `renderer::tests::the_quad_format_and_the_vertex_format_draw_the_same_surface`.
    ///
    /// `displacement` is the world's addressed displacement field. A quad whose
    /// material+face has a profile in it is refined into subdivided triangles by
    /// [`voxel_mesh::refine`]; a bare amplitude translates the plane; an absent
    /// field leaves the quad planar. Pass `None` to skip refinement and emit
    /// every quad as two triangles — the old behaviour, kept so existing callers
    /// that do not yet have a displacement field still compile.
    ///
    /// `refine_config` is the subdivision config. Only used when `displacement`
    /// is `Some`. The cap is refused by name rather than lowered (C-44.5).
    ///
    /// The eye is subtracted in `f64` for the reason the module comment states:
    /// a world-space `f32` at two million metres cannot resolve a block.
    pub fn build<'a>(
        meshes: impl IntoIterator<Item = &'a Mesh>,
        eye_m: [f64; 3],
        lattice: &DetailLattice,
        displacement: Option<&voxel_model::DisplacementField>,
        refine_config: Option<&voxel_mesh::RefineConfig>,
    ) -> Scene {
        Scene::build_with(meshes, eye_m, lattice, None, displacement, refine_config)
    }

    /// The scene **with the world's light field**: the vertex format, because a
    /// corner is where a per-cell measurement attaches.
    ///
    /// `light` is the world's own light field. `Some([direct (B3,B4,B5),
    /// diffuse (B3,B4,B5)])` in W/m² is a measurement; `None` is the world having
    /// no field there, which the fragment tells apart from a zero (see
    /// [`Vertex::light_present`]). This is the world source / shaper layer's job.
    ///
    /// **Why this is a constructor and not a flag.** The two formats carry
    /// different things — 24 camera-independent bytes against 64 eye-relative
    /// ones plus a measurement — so a runtime switch would make a frame's format
    /// depend on data. A named constructor makes it depend on what the caller
    /// asked for, and the two are then comparable instead of one being a branch.
    pub fn build_with_light<'a>(
        meshes: impl IntoIterator<Item = &'a Mesh>,
        eye_m: [f64; 3],
        lattice: &DetailLattice,
        light: &LightAt,
        displacement: Option<&voxel_model::DisplacementField>,
        refine_config: Option<&voxel_mesh::RefineConfig>,
    ) -> Scene {
        Scene::build_with(meshes, eye_m, lattice, Some(light), displacement, refine_config)
    }

    fn build_with<'a>(
        meshes: impl IntoIterator<Item = &'a Mesh>,
        eye_m: [f64; 3],
        lattice: &DetailLattice,
        light: Option<&LightAt>,
        displacement: Option<&voxel_model::DisplacementField>,
        refine_config: Option<&voxel_mesh::RefineConfig>,
    ) -> Scene {
        let mut scene = Scene::default();
        for mesh in meshes {
            let o = mesh.origin_voxel();
            let origin = [o[0] as f64, o[1] as f64, o[2] as f64];
            // One origin entry per mesh, pushed before its own quads, so a
            // record's `chunk` index is the entry standing beside it. **A chunk's veil is
            // zero**: a world chunk is not a cloud, and zero is what this float was before
            // clouds existed.
            let chunk = scene.origins.len() as u32;
            scene.origins.push([origin[0], origin[1], origin[2], 0.0]);
            for quad in &mesh.quads {
                let refined: Option<Vec<voxel_mesh::Triangle>> =
                    displacement.zip(refine_config).map(|(disp, cfg)| {
                        voxel_mesh::refine(quad, disp, cfg)
                            .map(|r| r.triangles)
                            .unwrap_or_default()
                    });

                let push = |scene: &mut Scene,
                            corner_m: [f64; 3],
                            normal: [f32; 3],
                            material: u16| {
                    let world = [
                        origin[0] + corner_m[0],
                        origin[1] + corner_m[1],
                        origin[2] + corner_m[2],
                    ];
                    let cell = world_cell(world);
                    let (present, bands) = match light.and_then(|f| f(cell[0], cell[1], cell[2])) {
                        Some(bands) => (1u32, bands),
                        None => (0u32, [0.0f32; 6]),
                    };
                    let (detail_r, detail_f) = match lattice.sample(
                        voxel_model::MaterialId(material),
                        cell,
                        corner_detail_index(world),
                    ) {
                        Ok(d) => (d.roughness_delta as f32, d.reflectance_delta as f32),
                        Err(_) => (0.0, 0.0),
                    };
                    scene.vertices.push(Vertex {
                        position: [
                            (world[0] - eye_m[0]) as f32,
                            (world[1] - eye_m[1]) as f32,
                            (world[2] - eye_m[2]) as f32,
                        ],
                        normal,
                        material: material as u32,
                        detail_roughness: detail_r,
                        detail_reflectance: detail_f,
                        light_direct: [bands[0], bands[1], bands[2]],
                        light_diffuse: [bands[3], bands[4], bands[5]],
                        light_present: present,
                    });
                };

                match refined {
                    Some(triangles) => {
                        for tri in &triangles {
                            let base = scene.vertices.len() as u32;
                            if base.checked_add(3).is_none() {
                                return scene;
                            }
                            for v in &tri.vertices {
                                push(
                                    &mut scene,
                                    subvoxel_m([
                                        v.position[0] as f64,
                                        v.position[1] as f64,
                                        v.position[2] as f64,
                                    ]),
                                    [v.normal[0], v.normal[1], v.normal[2]],
                                    tri.material.0,
                                );
                            }
                            scene.indices.extend_from_slice(&[base, base + 1, base + 2]);
                        }
                    }
                    None => match light {
                        // **The quad format.** The rectangle travels as itself —
                        // 24 chunk-local bytes — and `shading.wgsl`'s `vs_quad`
                        // expands it into two triangles. No corner is built here,
                        // and no lattice sample is taken: the fragment's own
                        // ladder is the whole address law for this geometry.
                        None => {
                            scene.records.push(QuadRecord::of(quad, chunk));
                        }
                        // **The vertex format**, for a frame that carries a light
                        // measurement: a corner is where one attaches.
                        Some(_) => {
                            let n = quad.normal();
                            let normal = [n[0] as f32, n[1] as f32, n[2] as f32];
                            let base = scene.vertices.len() as u32;
                            if base.checked_add(4).is_none() {
                                return scene;
                            }
                            for c in quad.corners() {
                                push(
                                    &mut scene,
                                    subvoxel_m([c[0] as f64, c[1] as f64, c[2] as f64]),
                                    normal,
                                    quad.material.0,
                                );
                            }
                            scene.indices.extend_from_slice(&[
                                base,
                                base + 1,
                                base + 2,
                                base,
                                base + 2,
                                base + 3,
                            ]);
                        },
                    }
                }
                scene.quads += 1;
            }
        }
        scene
    }



    /// Build a scene from meshes with per-vertex detail, using the world's own
    /// lattice and **no light field**. Displacement is not applied.
    pub fn build_with_detail<'a>(
        meshes: impl IntoIterator<Item = &'a Mesh>,
        eye_m: [f64; 3],
        lattice: &DetailLattice,
    ) -> Scene {
        Scene::build(meshes, eye_m, lattice, None, None)
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
            .map(|position| Vertex {
                position,
                normal,
                material,
                detail_roughness: 0.0,
                detail_reflectance: 0.0,
                light_direct: [0.0; 3],
                light_diffuse: [0.0; 3],
                light_present: 0,
            })
            .collect();
        // The probe is deliberately **the vertex format**: it exists to state a
        // shading normal, and a normal is not a rectangle's to choose.
        Scene {
            vertices,
            indices: vec![0, 1, 2, 0, 2, 3],
            records: Vec::new(),
            origins: Vec::new(),
            cloud_records: 0,
            quads: 1,
        }
    }

    /// **The cloud appended, after everything else** (C-179).
    ///
    /// A cloud cell is a rectangle like any other, so it is appended to the same arrays the
    /// mesher's own records travel in and needs no format of its own. The two things it adds
    /// are its **own origin per cell** — because a cell's veil rides in the origin's `w` and a
    /// chunk origin is shared by every quad on that chunk — and the count the draw splits on.
    ///
    /// The cloud's records must therefore come **last**, which is what makes
    /// [`Scene::cloud_records`] a suffix rather than a set. A caller that appends clouds and
    /// then more terrain would draw the terrain as a veil.
    pub fn with_clouds(mut self, cubes: &crate::cloud_cubes::CloudCubes) -> Scene {
        let base = self.origins.len() as u32;
        for origin in &cubes.origins {
            self.origins.push(*origin);
        }
        for record in &cubes.records {
            let mut record = *record;
            record.chunk += base;
            self.records.push(record);
        }
        self.cloud_records = cubes.records.len();
        self
    }

    /// **No geometry at all, in either format.**
    ///
    /// The quad format writes no index and no vertex, so a check on `indices`
    /// alone would report the world's own scene as empty — and a caller that used
    /// this to decide there was nothing to draw would then skip every rectangle
    /// the game draws. The record count is the other half of the same question.
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty() && self.records.is_empty()
    }

    /// The merged rectangles' own outlines, as a `LineList` index buffer.
    ///
    /// Four border edges per quad — `(0,1) (1,2) (2,3) (3,0)` in the corner order
    /// [`Scene::build`] emitted — and **not** the triangulation's diagonal, so a
    /// wireframe drawn from this is the greedy merge's answer rather than two
    /// triangles per rectangle. Eight `u32` indices per quad.
    pub fn quad_edge_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.quads * 8 * 4);
        for tri in self.indices.as_chunks::<6>().0 {
            let (a, b, c, d) = (tri[0], tri[1], tri[2], tri[5]);
            for i in [a, b, b, c, c, d, d, a] {
                out.extend_from_slice(&i.to_le_bytes());
            }
        }
        out
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

/// A corner in **subvoxels** — `Quad::corners` integers or a
/// [`voxel_mesh::refine`] vertex, which are the same unit — to metres from the
/// chunk's low corner.
fn subvoxel_m(subvoxel: [f64; 3]) -> [f64; 3] {
    let scale = 1.0 / voxel_model::SUBVOXEL_SIDE as f64;
    [
        subvoxel[0] * scale,
        subvoxel[1] * scale,
        subvoxel[2] * scale,
    ]
}

/// The world subvoxel cell a world position sits in: `floor(p / 0.25 m)` per axis.
fn world_cell(world: [f64; 3]) -> [i32; 3] {
    use voxel_model::SUBVOXEL_SIDE;
    let cell_m = 1.0 / SUBVOXEL_SIDE as f64;
    [
        (world[0] / cell_m).floor() as i32,
        (world[1] / cell_m).floor() as i32,
        (world[2] / cell_m).floor() as i32,
    ]
}

/// The detail lattice address at a world position: the in-cell fraction, as the
/// 16-per-axis samples the lattice declares.
fn corner_detail_index(world: [f64; 3]) -> voxel_model::DetailIndex {
    use voxel_model::{DETAIL_SIDE, SUBVOXEL_SIDE};
    let cell_m = 1.0 / SUBVOXEL_SIDE as f64;
    let cell_frac = [
        (world[0] / cell_m) - (world[0] / cell_m).floor(),
        (world[1] / cell_m) - (world[1] / cell_m).floor(),
        (world[2] / cell_m) - (world[2] / cell_m).floor(),
    ];
    let mut at = [0u16; 3];
    for axis in 0..3 {
        at[axis] = (cell_frac[axis] * DETAIL_SIDE as f64).floor() as u16;
        if at[axis] >= DETAIL_SIDE as u16 {
            at[axis] = DETAIL_SIDE as u16 - 1;
        }
    }
    voxel_model::DetailIndex::new(at[0], at[1], at[2]).unwrap_or(voxel_model::DetailIndex::new(0, 0, 0).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_model::{MaterialId, CHUNK_SIDE};

    /// No field, which is not the same as a field of zero: the point of the
    /// `Option`. A test that wants a *dark* cell returns `Some([0.0; 6])`.
    fn empty_light(_x: i32, _y: i32, _z: i32) -> Option<[f32; 6]> {
        None
    }

    fn mesh_at(x: i32, y: i32, z: i32) -> Mesh {
        Mesh {
            chunk: voxel_model::ChunkPos::new(x, y, z),
            quads: Vec::new(),
            loops: Vec::new(),
            stats: voxel_mesh::MeshStats {
                solid_voxels: 0,
                visible_faces: 0,
                quads: 0,
                loops: 0,
            },
        }
    }

    #[test]
    fn the_cloud_is_appended_as_a_suffix_and_its_own_origins_carry_the_veil() {
        // **The wiring's two claims, as facts** (C-179). A cloud's records come **last**, so the
        // draw can split one buffer into an opaque world and a blended veil without a second
        // array; and a cloud record addresses **its own** origin, because a cell's veil rides in
        // the origin's `w` and a chunk origin is shared by every quad standing on that chunk.
        // The world's own veil is zero, which is what that float was before clouds existed — so
        // adding clouds cannot change how a chunk shades.
        use voxel_model::Face;
        use voxel_mesh::{Mesh, Quad};
        let mesh = Mesh {
            chunk: voxel_model::ChunkPos::new(0, 0, 0),
            quads: vec![Quad {
                face: Face::PosY,
                slice: 4,
                uv: [0, 0],
                extent: [4, 4],
                material: MaterialId(6),
            }],
            loops: Vec::new(),
            stats: voxel_mesh::MeshStats {
                solid_voxels: 1,
                visible_faces: 1,
                quads: 1,
                loops: 0,
            },
        };
        let terrain = Scene::build_with_detail([&mesh], [0.0; 3], &DetailLattice::new(0));
        assert_eq!(terrain.records.len(), 1, "the fixture is not one rectangle");
        assert_eq!(terrain.cloud_records, 0, "terrain declared itself a cloud");
        assert_eq!(terrain.origins, vec![[0.0, 0.0, 0.0, 0.0]]);

        let cubes = crate::cloud_cubes::CloudCubes {
            records: vec![QuadRecord {
                face: 3,
                material: MaterialId::EMPTY.0 as u32,
                slice: 0,
                uv: 0,
                extent: 4 | (4 << 16),
                chunk: 0,
            }],
            origins: vec![[10.0, 500.0, -20.0, 0.72]],
            cell_m: 4.0,
            subdivisions: 1,
            cells: 1,
            rows_max: 1,
            ..Default::default()
        };
        let scene = terrain.with_clouds(&cubes);
        assert_eq!(scene.records.len(), 2);
        assert_eq!(scene.cloud_records, 1, "the draw's split is not the suffix");
        assert_eq!(
            scene.records.len() - scene.cloud_records,
            1,
            "the world's own rectangles are not the prefix"
        );
        assert_eq!(
            scene.records[1].chunk, 1,
            "the cloud's record did not move to its own origin"
        );
        assert_eq!(scene.origins.len(), 2);
        assert_eq!(scene.origins[0], [0.0, 0.0, 0.0, 0.0], "a chunk carries a veil");
        assert_eq!(scene.origins[1][3], 0.72, "the cell's veil was lost");
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
            detail_roughness: 0.0,
            detail_reflectance: 0.0,
            light_direct: [0.0; 3],
            light_diffuse: [0.0; 3],
            light_present: 0,
        };
        // `write_bytes` writes into a buffer the caller owns; the old
        // `Vertex::vertex_bytes` returned a fresh one and no longer exists.
        let mut bytes = Vec::new();
        v.write_bytes(&mut bytes);
        assert_eq!(bytes.len(), Vertex::STRIDE as usize);
    }
}

