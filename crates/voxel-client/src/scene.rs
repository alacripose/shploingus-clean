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
//! That costs a `to_ne_bytes` per `f32` and buys a vertex layout that is
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
/// The lifetime is the closure's and it is **required rather than defaulted**: a bare
/// `dyn Fn(...)` in a type alias takes the `'static` bound, so a field borrowing the frame's
/// own world could not be passed at all — which is the only kind of field there is. A
/// `'static` alias here would make the measurement available to exactly the callers who do
/// not have one.
pub type LightAt<'a> = dyn Fn(i32, i32, i32) -> Option<[f32; 6]> + 'a;

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
            out.extend_from_slice(&voxel_model::endian::le_bytes(*v));
        }
        out.extend_from_slice(&voxel_model::endian::le_bytes(self.material));
        out.extend_from_slice(&voxel_model::endian::le_bytes(self.detail_roughness));
        out.extend_from_slice(&voxel_model::endian::le_bytes(self.detail_reflectance));
        for v in self.light_direct.iter().chain(self.light_diffuse.iter()) {
            out.extend_from_slice(&voxel_model::endian::le_bytes(*v));
        }
        out.extend_from_slice(&voxel_model::endian::le_bytes(self.light_present));
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
    /// camera. `w` is `0.0` for a chunk of the world and, for a cloud cell, its own transmittance
    /// **signed by which tree a fade is joining it to** (C-179, C-183:
    /// [`crate::cloud_cubes::veil_with_side`]) — the one float of this `vec4` that had no meaning
    /// before clouds. A scene is therefore the same bytes for every frame of a dissolve; the
    /// weights are the shader's, read from one uniform.
    pub origins: Vec<[f64; 4]>,
    /// **The astronomical bodies this frame draws** (C-184), each with its own origin pushed onto
    /// [`Scene::origins`] — a body's centre and half-extent ride the origin the way a cloud's veil
    /// does. They are their own array rather than a third suffix because a body is not a rectangle in
    /// either direction: `vs_body` expands it, and its slots are not a `uv` and an extent.
    pub bodies: Vec<QuadRecord>,
    /// **How many of the trailing records are the cloud** (C-179). The world draws opaque and
    /// first; the cloud draws after it, blended, with no depth write — so the draw needs to
    /// know where its own range begins, and that is `records.len() - cloud_records`. It is the
    /// **sum** of the trees a scene appended: one, or the two a cross-fade holds for a frame.
    pub cloud_records: usize,
    pub quads: usize,
    /// **How many quads left the record format for triangles** — the ones a displacement
    /// law actually bent. A quad whose refinement is the plane it already was keeps its
    /// 24-byte record, so this is the vertex format's honest share of a frame and the
    /// number that answers "is the 256 level costing anything yet" (C-44, C-170).
    pub bent_quads: usize,
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
    /// [`voxel_mesh::refine`], and a bare amplitude translates its plane; a quad the
    /// field does not move **stays a rectangle in the record format**, because the
    /// refinement of a plane is the plane. Pass `None` to leave the stage out — which is
    /// what a caller whose world declares no law yet does.
    ///
    /// **Refinement is therefore charged per quad and not per frame.** A quad that leaves
    /// the record format costs 280 bytes of corners against the record's 24 (C-170), and
    /// [`Scene::bent_quads`] counts them — so a world whose shapers all act at the stored
    /// level pays nothing for a stage that is wired in, and one that declares a law pays
    /// for exactly the faces that law touches.
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
        light: &LightAt<'_>,
        displacement: Option<&voxel_model::DisplacementField>,
        refine_config: Option<&voxel_mesh::RefineConfig>,
    ) -> Scene {
        Scene::build_with(meshes, eye_m, lattice, Some(light), displacement, refine_config)
    }

    fn build_with<'a>(
        meshes: impl IntoIterator<Item = &'a Mesh>,
        eye_m: [f64; 3],
        lattice: &DetailLattice,
        light: Option<&LightAt<'_>>,
        displacement: Option<&voxel_model::DisplacementField>,
        refine_config: Option<&voxel_mesh::RefineConfig>,
    ) -> Scene {
        let mut scene = Scene::default();
        // An empty field is the unwired frame, decided once rather than per quad (C-182).
        let stage = match (displacement, refine_config) {
            (Some(disp), Some(cfg)) if !disp.is_empty() => Some((disp, cfg)),
            _ => None,
        };
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
                // Only a bent quad leaves the record format, and a refusal is never a hole
                // (C-182). The entry is asked for before refining: a face the field does not
                // name has no law to sample.
                let refined: Option<Vec<voxel_mesh::Triangle>> = match stage {
                    Some((disp, cfg)) if disp.entry(quad.material, quad.face).is_some() => {
                        match voxel_mesh::refine(quad, disp, cfg) {
                            Ok(face) => (face.stats.subdivided_quads > 0
                                || face.stats.displaced_vertices > 0)
                                .then_some(face.triangles),
                            Err(_) => None,
                        }
                    }
                    _ => None,
                };
                if refined.is_some() {
                    scene.bent_quads += 1;
                }

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
            bodies: Vec::new(),
            cloud_records: 0,
            quads: 1,
            bent_quads: 0,
        }
    }

    /// **The Sun and the Moon appended as records** (C-184), each on its own origin.
    ///
    /// A body's record carries what it looks like and the direction that lights it; where it is comes
    /// from the origin this pushes, so the body travels in the world's own `f64` metres and is narrowed
    /// only after the eye is subtracted at upload.
    pub fn with_bodies(mut self, bodies: &[crate::quad::BodyRecord]) -> Scene {
        for body in bodies {
            let chunk = self.origins.len() as u32;
            self.origins.push(body.origin());
            self.bodies.push(QuadRecord::of_body(body, chunk));
        }
        self
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
    pub fn with_clouds(self, cubes: &crate::cloud_cubes::CloudCubes) -> Scene {
        self.append_clouds(cubes, false)
    }

    /// **Two trees at once, each carrying its own side** — the newer one appended last.
    ///
    /// A cloud is derived on its own cadence and drawn every frame, so the picture has to be a join
    /// of the two most recent derivations rather than a swap between them. Both are appended to the
    /// same suffix, drawn by one pipeline in one pass, and **neither carries a weight**: the fade
    /// itself is the shader's, from one uniform ([`crate::world::WorldPipeline::set_cloud_fade`]),
    /// so the arrays this builds are the same bytes for every frame of the dissolve and the GPU
    /// interpolates per pixel rather than the frame thread per rectangle ([`Self::append_clouds`]).
    ///
    /// The older tree comes first because the newer one is drawn over it, and the *side* each
    /// rectangle belongs to rides the sign of its veil
    /// ([`crate::cloud_cubes::veil_with_side`]).
    pub fn with_cloud_fade(
        self,
        from: &crate::cloud_cubes::CloudCubes,
        to: &crate::cloud_cubes::CloudCubes,
    ) -> Scene {
        self.append_clouds(from, true).append_clouds(to, false)
    }

    /// Append one tree's rectangles as the cloud suffix, its side written into every veil.
    ///
    /// **The counts accumulate**, because a fade appends two trees in one frame and the draw splits
    /// on one suffix: `cloud_records` is how many *trailing* records are cloud, so the second join
    /// must add to the first rather than replace it.
    fn append_clouds(mut self, cubes: &crate::cloud_cubes::CloudCubes, older: bool) -> Scene {
        use crate::cloud_cubes::veil_with_side;
        let base = self.origins.len() as u32;
        for origin in &cubes.origins {
            self.origins.push([
                origin[0],
                origin[1],
                origin[2],
                veil_with_side(origin[3], older),
            ]);
        }
        for record in &cubes.records {
            let mut record = *record;
            record.chunk += base;
            self.records.push(record);
        }
        self.cloud_records += cubes.records.len();
        self
    }

    /// **No geometry at all, in either format.**
    ///
    /// The quad format writes no index and no vertex, so a check on `indices`
    /// alone would report the world's own scene as empty — and a caller that used
    /// this to decide there was nothing to draw would then skip every rectangle
    /// the game draws. The record count is the other half of the same question.
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty() && self.records.is_empty() && self.bodies.is_empty()
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
                out.extend_from_slice(&voxel_model::endian::le_bytes(i));
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
            out.extend_from_slice(&voxel_model::endian::le_bytes(*i));
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
    use voxel_mesh::{Mesh, Quad, RefineConfig};
    use voxel_model::{DisplacementField, Face, MaterialId, Profile};

    /// One block's own visible face, as the merge emits it: a single rectangle four
    /// subvoxels on a side, standing on plane `slice = 4`.
    fn one_block_face(face: Face, material: MaterialId) -> Mesh {
        Mesh {
            chunk: voxel_model::ChunkPos::new(0, 0, 0),
            quads: vec![Quad { face, slice: 4, uv: [0, 0], extent: [4, 4], material }],
            loops: Vec::new(),
            stats: voxel_mesh::MeshStats {
                solid_voxels: 1,
                visible_faces: 1,
                quads: 1,
                loops: 0,
            },
        }
    }

    /// A round column inscribed in its block: the pillar's own law, whose cap is a
    /// hemisphere of radius half a block.
    fn a_pillar(material: MaterialId, face: Face) -> DisplacementField {
        let mut field = DisplacementField::none();
        field
            .declare_profile(material, face, Profile::RoundColumn { radius: 0.5 })
            .expect("a pillar's cap is inside its own block and is declarable");
        field
    }

    #[test]
    fn a_declared_law_moves_its_own_face_and_every_other_quad_stays_a_record() {
        /// The stage's cost rule as one frame: the declared face pays 32 triangles, the other
        /// stays a 24-byte record, and the scene's own count says so.
        const ROCK: MaterialId = MaterialId(6);
        let field = a_pillar(ROCK, Face::PosY);
        let config = RefineConfig::new(4, 4096).expect("4 samples per block is inside the level");
        let roof = one_block_face(Face::PosY, ROCK);
        let wall = one_block_face(Face::PosZ, ROCK);

        let scene = Scene::build(
            [&roof, &wall],
            [0.0; 3],
            &DetailLattice::new(0),
            Some(&field),
            Some(&config),
        );
        assert_eq!(scene.bent_quads, 1, "the law reached a quad it does not touch");
        assert_eq!(scene.records.len(), 1, "a bent quad was left in the record format");
        assert_eq!(scene.quads, 2);
        assert_eq!(scene.indices.len(), 96, "32 triangles is 96 indices");

        // A bulge inside the rectangle, not a moved rectangle: exact at the four corners,
        // the declared amplitude at the centre, both read from the field.
        let amplitude_m = (field.amplitude(ROCK, Face::PosY).expect("declared") / 4.0) as f32;
        let top = |x: f32, z: f32| {
            scene
                .vertices
                .iter()
                .filter(|v| (v.position[0] - x).abs() < 1e-4 && (v.position[2] - z).abs() < 1e-4)
                .map(|v| v.position[1])
                .fold(f32::NEG_INFINITY, f32::max)
        };
        for (x, z) in [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)] {
            let y = top(x, z);
            assert!((y - 1.0).abs() < 1e-4, "the corner at ({x}, {z}) moved to {y}");
        }
        let centre = top(0.5, 0.5);
        assert!(
            (centre - (1.0 + amplitude_m)).abs() < 1e-3,
            "the cap stands {centre} m against the declared {amplitude_m} m above the plane"
        );
        let inside = scene
            .vertices
            .iter()
            .all(|v| (-1e-4..=1.0 + 1e-4).contains(&v.position[0])
                && (-1e-4..=1.0 + 1e-4).contains(&v.position[2]));
        assert!(inside, "the refinement left its own rectangle in plane");
    }

    #[test]
    fn a_refused_refinement_falls_back_to_the_rectangle_and_not_to_a_hole() {
        /// A refused quad falls back to its rectangle rather than to an empty triangle list:
        /// a hole would look right in every count (C-44.5, C-182).
        const ROCK: MaterialId = MaterialId(6);
        let field = a_pillar(ROCK, Face::PosY);
        // Two triangles cannot hold a subdivided face, so every bend here is refused.
        let refused = RefineConfig::new(4, 2).expect("a cap of two triangles is a cap");
        let roof = one_block_face(Face::PosY, ROCK);
        let scene = Scene::build(
            [&roof],
            [0.0; 3],
            &DetailLattice::new(0),
            Some(&field),
            Some(&refused),
        );
        assert!(!scene.is_empty(), "a refused refinement made the world disappear");
        assert_eq!(scene.bent_quads, 0, "a refused quad claimed the vertex format");
        assert_eq!(scene.records.len(), 1, "the planar fallback is not one rectangle");
        assert!(scene.indices.is_empty(), "a refused quad spent vertices anyway");
    }

    #[test]
    fn wiring_the_stage_with_a_world_that_declares_nothing_costs_nothing() {
        /// The frame the game draws today: a real field and a real config, and not one quad
        /// changes format. This is what makes "wired in" a statement about cost.
        const ROCK: MaterialId = MaterialId(6);
        let field = DisplacementField::none();
        let config = RefineConfig::new(4, 4096).unwrap();
        let roof = one_block_face(Face::PosY, ROCK);
        let wall = one_block_face(Face::PosZ, ROCK);
        let wired = Scene::build(
            [&roof, &wall],
            [0.0; 3],
            &DetailLattice::new(0),
            Some(&field),
            Some(&config),
        );
        let unwired = Scene::build_with_detail([&roof, &wall], [0.0; 3], &DetailLattice::new(0));
        assert_eq!(wired.records.len(), 2, "the empty field moved a quad out of the record");
        assert!(wired.indices.is_empty());
        assert_eq!(wired.bent_quads, 0);
        assert_eq!(wired.records, unwired.records, "wiring the stage changed the frame");
        assert_eq!(wired.origins, unwired.origins);

        // What the wiring costs, as a reading: a thousand quads a scene, since the stage is
        // per quad. Reported, not asserted — a timing assertion is a machine's load.
        let many: Mesh = {
            let mut m = one_block_face(Face::PosY, ROCK);
            m.quads = (0..1_000)
                .map(|i| Quad {
                    face: Face::PosY,
                    slice: 4,
                    uv: [(i % 64) as u16 * 4, (i / 64) as u16 * 4],
                    extent: [4, 4],
                    material: ROCK,
                })
                .collect();
            m
        };
        let n = 100u32;
        let timed = |build: &dyn Fn()| {
            let t0 = std::time::Instant::now();
            for _ in 0..n {
                build();
            }
            t0.elapsed().as_nanos() / n as u128
        };
        let wired = || {
            let built = Scene::build(
                [&many],
                [0.0; 3],
                &DetailLattice::new(0),
                Some(&field),
                Some(&config),
            );
            std::hint::black_box(built.records.len());
        };
        let unwired = || {
            let built = Scene::build_with_detail([&many], [0.0; 3], &DetailLattice::new(0));
            std::hint::black_box(built.records.len());
        };
        // Both orders, minima reported: the first pass of a pair pays for a cold heap, so one
        // pair is a reading of the order the two were taken in.
        let mut wired_ns = u128::MAX;
        let mut unwired_ns = u128::MAX;
        for round in 0..4 {
            if round % 2 == 0 {
                wired_ns = wired_ns.min(timed(&wired));
                unwired_ns = unwired_ns.min(timed(&unwired));
            } else {
                unwired_ns = unwired_ns.min(timed(&unwired));
                wired_ns = wired_ns.min(timed(&wired));
            }
        }
        println!(
            "a 1 000-quad scene: {wired_ns} ns wired ({:.1} ns a quad) against {unwired_ns} ns \
             unwired, {n} builds each, 4 rounds alternating",
            wired_ns as f64 / 1_000.0
        );
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
                light_y: 0,
                light_z: 0,
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
        // **The cell's veil, in the encoding the shader reads back**: `1 + opacity`, positive for
        // the newer of the two trees a fade may join (`cloud_cubes::veil_with_side`), so
        // `abs(w) - 1` is the opacity and `w != 0` is "this is a cloud".
        assert!((scene.origins[1][3] - 1.28).abs() < 1e-12, "{}", scene.origins[1][3]);
    }

    #[test]
    fn the_two_trees_of_a_fade_carry_their_side_in_the_sign_of_their_veils() {
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
        // One cell's worth of cloud, at a stated veil, standing at a stated corner.
        let cell = |veil: f64, x: f64| crate::cloud_cubes::CloudCubes {
            records: vec![QuadRecord {
                face: 3,
                material: MaterialId::EMPTY.0 as u32,
                slice: 0,
                uv: 0,
                extent: 4 | (4 << 16),
                chunk: 0,
                light_y: 0,
                light_z: 0,
            }],
            origins: vec![[x, 500.0, -20.0, veil]],
            cell_m: 4.0,
            subdivisions: 1,
            cells: 1,
            rows_max: 1,
            ..Default::default()
        };
        let older = cell(0.6, 0.0);
        let newer = cell(0.6, 8.0);
        let scene = terrain.clone().with_cloud_fade(&older, &newer);
        assert_eq!(scene.records.len(), 3, "the join is not both trees over the world");
        assert_eq!(scene.cloud_records, 2, "the draw's split is not the whole suffix");
        assert_eq!(
            scene.records[2].chunk, 2,
            "the newer tree did not move to its own origins"
        );
        // **Both are the tree's own transmittance, and only the sign differs.** A veil of 0.6 is
        // an opacity of 0.4, so both carry `1.4`: the older negated, the newer positive. Nothing in
        // the scene is a *weight* — the shader reads those from one uniform
        // (`WorldPipeline::set_cloud_fade`), so these bytes are the same for every frame of the
        // dissolve and a fade costs no rewriting of the sky at all.
        assert!((scene.origins[1][3] + 1.4).abs() < 1e-12, "{}", scene.origins[1][3]);
        assert!((scene.origins[2][3] - 1.4).abs() < 1e-12, "{}", scene.origins[2][3]);

        // **The same scene serves the whole dissolve.** There is no weight to pass, so the pair a
        // frame draws at the start of a cadence and at the end of it is literally one value — and
        // a scene built from the pair is the same one the frame before it held.
        let again = terrain.clone().with_cloud_fade(&older, &newer);
        assert_eq!(scene, again, "the pair's own bytes depend on something but the trees");
        assert_ne!(scene, terrain.clone().with_clouds(&newer), "a lone tree is not a pair");

        // One tree alone carries the newer side, so a frame with nothing in flight draws its own
        // number whole: the shader's weight is `1.0` there and the decode is the identity.
        let entering = terrain.with_clouds(&newer);
        assert_eq!(entering.cloud_records, 1);
        assert!((entering.origins[1][3] - 1.4).abs() < 1e-12, "{}", entering.origins[1][3]);
    }

    #[test]
    fn the_fades_cost_is_the_second_trees_records_and_nothing_else() {
        // What a cross-fade costs on the frame thread, in the unit the frame pays in: a record
        // appended. The cloud is 39,540 rectangles in the shipped field, so a fade draws twice that.
        use crate::cloud_cubes::CloudCubes;
        let count = 40_000usize;
        let tree = |offset: f64| CloudCubes {
            records: (0..count)
                .map(|i| QuadRecord {
                    face: 3,
                    material: MaterialId::EMPTY.0 as u32,
                    slice: 0,
                    uv: (i % 4096) as u32,
                    extent: 4 | (4 << 16),
                    chunk: i as u32,
                    light_y: 0,
                    light_z: 0,
                })
                .collect(),
            origins: (0..count)
                .map(|i| [offset + i as f64, 500.0, 0.0, 0.9])
                .collect(),
            cell_m: 8.0,
            cells: count,
            ..Default::default()
        };
        let older = tree(0.0);
        let newer = tree(8.0);
        let bare = Scene::build_with_detail(Vec::new().iter(), [0.0; 3], &DetailLattice::new(0));

        // Four rounds alternating, the minimum of each: a pair's first pass pays for a cold heap.
        let (mut single, mut fade) = (f64::INFINITY, f64::INFINITY);
        for _ in 0..4 {
            let started = std::time::Instant::now();
            let scene = bare.clone().with_clouds(&newer);
            assert_eq!(scene.cloud_records, count);
            single = single.min(started.elapsed().as_secs_f64());

            let started = std::time::Instant::now();
            let scene = bare.clone().with_cloud_fade(&older, &newer);
            assert_eq!(scene.cloud_records, 2 * count, "the fade drew one tree");
            fade = fade.min(started.elapsed().as_secs_f64());
        }
        // The honest unit is **a record appended**, because that is what the frame pays for: one tree
        // appends its own records, a fade appends both trees'.
        let alone = single * 1e9 / count as f64;
        let joined = fade * 1e9 / (2 * count) as f64;
        println!(
            "one tree of {count} rectangles: {alone:.2} ns a record ({:.2} ms); a cross-fade of two \
             ({:.2} ms for {} records): {joined:.2} ns a record",
            single * 1e3,
            fade * 1e3,
            2 * count,
        );
        // The cost is linear in the records, so the price of a smooth sky is the second tree.
        assert!(
            joined <= 2.0 * alone.max(1.0),
            "a cross-fade cost {joined} ns a record against {alone} for one tree, which is not a copy"
        );
        // **And the frame pays that once, not once per frame.** The pair is built when a tree is
        // adopted; every frame of the dissolve after it draws the same bytes with one uniform
        // changed, so what the dissolve costs the frame thread is nothing at all — the interpolation
        // is `shading.wgsl`'s, per pixel.
        let scene = bare.with_cloud_fade(&older, &newer);
        assert_eq!(scene.records[count].chunk, count as u32, "the newer tree is not last");
        assert!(
            scene.origins[0][3] < 0.0 && scene.origins[count][3] > 0.0,
            "the two trees' sides are not in the signs: {} against {}",
            scene.origins[0][3],
            scene.origins[count][3]
        );
        assert!(
            (scene.origins[0][3] + scene.origins[count][3]).abs() < 1e-12,
            "the two sides are not the same sky: {} against {}",
            scene.origins[0][3],
            scene.origins[count][3]
        );
    }

    #[test]
    fn a_body_is_its_own_array_and_its_own_origin() {
        // **A body is not a rectangle and not a cloud.** It carries its own origin — centre and
        // half-extent — and it is counted by `is_empty`, which a scene of nothing but a body must not
        // report as empty: a frame with the Moon in it and no terrain is a frame with something in it.
        let moon = crate::quad::BodyRecord {
            centre_m: [3.84e8, 0.0, 0.0],
            half_extent_m: 1_737_400.0,
            radiance: [0.125, 0.0, 0.5],
            light_direction: [0.0, -0.25, 1.0],
        };
        let scene = Scene::default().with_bodies(&[moon]);
        assert_eq!(scene.bodies.len(), 1, "the body was not appended");
        assert_eq!(scene.origins, vec![moon.origin()], "the body has no origin of its own");
        assert_eq!(scene.records.len(), 0, "a body is not a rectangle's record");
        assert_eq!(scene.cloud_records, 0, "a body declared itself a cloud");
        assert!(scene.bodies[0].is_body());
        assert!(!scene.is_empty(), "a scene with the Moon in it read as empty");
        assert_eq!(scene.bodies[0].chunk, 0, "the record does not name its own origin");
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

