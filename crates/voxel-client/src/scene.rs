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

use crate::quad::QuadRecord;
#[cfg(test)]
use voxel_appearance::DetailLattice;
use voxel_mesh::Mesh;

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
pub type LightAt<'a> = dyn Fn([f64; 3]) -> Option<[f32; 6]> + 'a;

/// **The finest shadow a face can carry, in cells per voxel edge.**
///
/// `SUBVOXEL_SIDE` is `4`, and it is a fact about the **stored** light, not a preference:
/// [`crate::field::CellField`] measures one cell per subvoxel, so a face cut any finer asks the
/// same cell twice and can only change the *interpolation* between samples, never an answer.
/// A face cut coarser than this cannot resolve a single subvoxel's shadow — which is exactly
/// the complaint a once-per-corner quad cannot answer, because a merged rectangle carries four
/// samples however many metres it spans.
pub const SHADOW_LATTICE: u32 = voxel_model::SUBVOXEL_SIDE as u32;

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
    /// Metres, **relative to the chunk's own low corner**.
    ///
    /// **The shader adds the chunk's `origin − eye`** from the origins buffer (`vs_main`,
    /// binding 4) — the same array the record format reads. So a corner is the same bytes
    /// while the camera walks, and only the 16-bytes-a-chunk origins move with the eye: a
    /// scene built once stays the frame's scene until the world or its light changes.
    pub position: [f32; 3],
    /// The quad's outward normal, straight from `Quad::normal` — an axis unit
    /// vector, so every corner of a quad carries the same normal and the
    /// interpolated one is exact. For a refined (displaced) face the normal
    /// comes from the profile's cap normal at that corner.
    pub normal: [f32; 3],
    /// `MaterialId`, used to index the shader's material table.
    pub material: u32,
    /// Always zero: corners once carried the 256-level lattice sampled at the
    /// corner's world address, but an octave-0 sample interpolated across a
    /// merged rectangle is streaks rather than detail, and the fragment's own
    /// ladder reads every rung the eye asked for from the fragment's address.
    /// The slots stay in the layout so the vertex format does not move under
    /// the shader's `VsIn`.
    pub detail_roughness: f32,
    /// **The face this corner was cut from** — 0, 1, 2, or `-1` where the geometry has none:
    /// the axis the shaper's pattern lies along, so a directional material does not draw square
    /// cells on the face that looks along its world axis and stretched ones on every other side
    /// (C-163.2). It rides the second of the two `f32` the retired corner sample reserved, so the
    /// stride does not move.
    pub plane_axis: i32,
    /// The world's light at this corner's cell, in band order (B3, B4, B5):
    /// direct beam, then diffuse (sky + one-bounce), in W/m². **Only meaningful
    /// when [`Vertex::light_present`] is 1** — with no field these stay zero
    /// because a vertex is `[f32]` and zero is what a missing `f32` looks like.
    pub light_direct: [f32; 3],
    pub light_diffuse: [f32; 3],
    /// **Whether the world's light field covers this corner's quad at all**,
    /// from [`LightAt`]'s `Option`: 1 when every corner of the quad was
    /// measured, 0 when any was not. Quad-uniform **on purpose**: the flag
    /// reaches the fragment flat, and flat takes one corner's value per
    /// triangle, so per-corner flags would split a quad's two triangles down
    /// the diagonal whenever coverage differed across it. A flag is still not
    /// a sentinel — zero W/m² is a real daylight answer (night, shadow, a
    /// covered cell) and must not hand a lit surface back to the rig — but the
    /// granularity of "measured" is the quad, never the corner.
    pub light_present: u32,
    /// The chunk this corner belongs to: an index into `Scene::origins`, whose entry is that
    /// chunk's `origin − eye` for the frame being drawn.
    pub chunk: u32,
}

impl Vertex {
    /// **A corner that covers nothing**, for the range a departed chunk left behind.
    ///
    /// The draw reads one dense array, so the elements of a hole have to be corners the vertex
    /// stage accepts and no triangle refers to — the index buffer's own tombstones point at index
    /// zero, so these never reach a primitive. They name origin zero, which is a live origin
    /// whenever the scene has anything resident at all (`ResidentScene` clears the arrays whole
    /// when the last chunk leaves), and stand at their chunk's own low corner.
    pub const fn blank() -> Self {
        Vertex {
            position: [0.0; 3],
            normal: [0.0, 1.0, 0.0],
            material: 0,
            detail_roughness: 0.0,
            plane_axis: -1,
            light_direct: [0.0; 3],
            light_diffuse: [0.0; 3],
            light_present: 0,
            chunk: 0,
        }
    }

    /// `3*4 + 3*4 + 4 + 4 + 4 + 3*4 + 3*4 + 4 + 4`. A test checks this against the
    /// attribute list rather than trusting the arithmetic.
    pub const STRIDE: u64 = 3 * 4 + 3 * 4 + 4 + 4 + 4 + 3 * 4 + 3 * 4 + 4 + 4;

    /// The attribute descriptors, and the offsets they claim.
    pub fn attributes() -> [wgpu::VertexAttribute; 9] {
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
            // plane_axis — the second reserved `f32`, now the face's own axis
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Sint32,
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
            // chunk — the origins entry this corner is placed from
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Uint32,
                offset: 64,
                shader_location: 8,
            },
        ]
    }

    pub(crate) fn write_bytes(self, out: &mut Vec<u8>) {
        for v in self.position.iter().chain(self.normal.iter()) {
            out.extend_from_slice(&voxel_model::endian::le_bytes(*v));
        }
        out.extend_from_slice(&voxel_model::endian::le_bytes(self.material));
        out.extend_from_slice(&voxel_model::endian::le_bytes(self.detail_roughness));
        out.extend_from_slice(&voxel_model::endian::le_bytes(self.plane_axis));
        for v in self.light_direct.iter().chain(self.light_diffuse.iter()) {
            out.extend_from_slice(&voxel_model::endian::le_bytes(*v));
        }
        out.extend_from_slice(&voxel_model::endian::le_bytes(self.light_present));
        out.extend_from_slice(&voxel_model::endian::le_bytes(self.chunk));
    }
}

/// Triangles ready to draw, and the count of quads they came from.
#[derive(Clone, PartialEq, Debug)]
pub struct Scene {
    /// **The build this scene came out of**, monotonic per process. The GPU compares it to the
    /// id of the corners it last uploaded, so a scene that is the same scene is not re-sent.
    pub build: u64,
    /// The world's own lengths at the end of the build — `records`, `origins`, `vertices`,
    /// `indices` — which is where [`Scene::begin_frame`] truncates the frame's additions back to.
    pub terrain: [usize; 4],
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
    /// **The cloud's own rectangles and origins** (2026-10-22). They used to be a suffix of
    /// `records`/`origins`, which coupled the sky to the ground: every chunk that landed or left
    /// moved the terrain array and therefore every cloud record with it. They are drawn by the
    /// same shader from their own buffers instead, so a residency change touches only terrain.
    /// A cloud record's `chunk` indexes [`Scene::cloud_origins`], never `origins`.
    pub clouds: Vec<QuadRecord>,
    /// One world origin per cloud record, in metres and with the veil in `w`, indexed by the
    /// record's own `chunk`.
    pub cloud_origins: Vec<[f64; 4]>,
    /// **The far field's rectangles** — the world beyond the residency window, cut at the coarse
    /// rung ([`crate::far_terrain`]). Their own pair for the cloud's own reason: a far chunk is
    /// built on the far thread and does not move with a terrain slice, so a chunk landing near the
    /// eye must not renumber what stands at the horizon. A far record's `chunk` indexes
    /// [`Scene::far_origins`], and its redundant slot carries the cell size the material ladder is
    /// told to respect.
    pub far: Vec<QuadRecord>,
    /// One world origin per far chunk, in metres.
    pub far_origins: Vec<[f64; 4]>,
    /// **Whether this scene was built whole** — the upload a rebuild owes — against a resident
    /// scene, which carries only the ranges that changed. `fresh` scenes take the whole-array
    /// path (compare, patch or encode); a resident scene writes `written_*` and nothing else.
    pub fresh: bool,
    /// **The ranges of this frame's arrays a resident scene changed**, in element indices,
    /// merged and sorted. Empty on a `fresh` scene, whose whole array is owed.
    pub written_vertices: Vec<[u32; 2]>,
    /// The index-buffer ranges this frame changed, in element indices.
    pub written_indices: Vec<[u32; 2]>,
    /// The record-buffer ranges this frame changed, in element indices.
    pub written_records: Vec<[u32; 2]>,
    pub quads: usize,
    /// **How many quads left the record format for triangles** — the ones a displacement
    /// law actually bent. A quad whose refinement is the plane it already was keeps its
    /// 24-byte record, so this is the vertex format's honest share of a frame and the
    /// number that answers "is the 256 level costing anything yet" (C-44, C-170).
    pub bent_quads: usize,
}

impl Default for Scene {
    /// **A new scene is a whole one**: the upload expects to send it, and a resident scene says
    /// otherwise by setting `fresh = false` where it is owned ([`ResidentScene`]).
    fn default() -> Self {
        Self {
            build: 0,
            terrain: [0; 4],
            vertices: Vec::new(),
            indices: Vec::new(),
            records: Vec::new(),
            origins: Vec::new(),
            bodies: Vec::new(),
            clouds: Vec::new(),
            cloud_origins: Vec::new(),
            far: Vec::new(),
            far_origins: Vec::new(),
            fresh: true,
            written_vertices: Vec::new(),
            written_indices: Vec::new(),
            written_records: Vec::new(),
            quads: 0,
            bent_quads: 0,
        }
    }
}

impl Scene {
    /// Build the scene from meshes, with per-vertex light.
    ///
    /// **This is the quad format: no light field.** A rectangle travels as a
    /// 24-byte [`QuadRecord`] and the GPU expands it, which is what takes the
    /// per-frame cost off the CPU; the fragment's own ladder is the whole address law for that geometry. A frame
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
    /// **`eye_m` is the frame the caller is building for, and the build no longer needs it.** A
    /// corner is chunk-local and the eye is subtracted once per chunk, in `f64`, when the origins
    /// are uploaded (`crate::quad::origin_bytes`) — which is why this array does not mention the
    /// camera and why a scene built before a step is still the scene after it. It stays in the
    /// signature because a caller is always stating a frame, and the parameter is where that
    /// statement goes; the mechanical removal of it is a follow-up that touches fifty call sites
    /// and changes no behaviour.
    pub fn build<'a>(
        meshes: impl IntoIterator<Item = &'a Mesh>,
        _eye_m: [f64; 3],
        displacement: Option<&voxel_model::DisplacementField>,
        refine_config: Option<&voxel_mesh::RefineConfig>,
    ) -> Scene {
        Scene::build_with(meshes, None, displacement, refine_config, 1)
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
    /// different things — 32 camera-independent bytes against 68 chunk-local ones plus a
    /// measurement — so a runtime switch would make a frame's format depend on data. A named
    /// constructor makes it depend on what the caller asked for, and the two are then comparable
    /// instead of one being a branch.
    ///
    /// `eye_m` is ignored for the reason [`Scene::build`] states.
    pub fn build_with_light<'a>(
        meshes: impl IntoIterator<Item = &'a Mesh>,
        _eye_m: [f64; 3],
        light: &LightAt<'_>,
        displacement: Option<&voxel_model::DisplacementField>,
        refine_config: Option<&voxel_mesh::RefineConfig>,
    ) -> Scene {
        Scene::build_with(meshes, Some(light), displacement, refine_config, 1)
    }

    /// **The scene with the world's light, sampled at `shadow_lattice` cells per voxel edge.**
    ///
    /// This is [`Scene::build_with_light`] with the shadow's own resolution stated. `0` and `1`
    /// are the once-per-corner path — one cell for the whole rectangle, whatever its size — and
    /// any larger value is **cells per voxel edge**: `[`SHADOW_LATTICE`]` is the finest the stored
    /// field can answer, one sample per subvoxel, so a face is cut into `lattice²` cells per
    /// voxel. Values that are not a whole divisor of `SUBVOXEL_SIDE` are lowered to the largest
    /// one that is (`3` is answered by `2`), because a sample on a cell boundary is a coin toss
    /// the address law does not make.
    ///
    /// **Why a separate constructor rather than a flag on the field.** The two formats already
    /// exist as constructors (see [`Scene::build_with_light`]); this is the second *resolution*
    /// of the same vertex format, so it is a second constructor for the same reason — the frame
    /// says what it asked for, and the two can be compared instead of one being a branch.
    pub fn build_with_light_lattice<'a>(
        meshes: impl IntoIterator<Item = &'a Mesh>,
        _eye_m: [f64; 3],
        light: &LightAt<'_>,
        displacement: Option<&voxel_model::DisplacementField>,
        refine_config: Option<&voxel_mesh::RefineConfig>,
        shadow_lattice: u32,
    ) -> Scene {
        Scene::build_with(
            meshes,
            Some(light),
            displacement,
            refine_config,
            shadow_lattice,
        )
    }

    fn build_with<'a>(
        meshes: impl IntoIterator<Item = &'a Mesh>,
        light: Option<&LightAt<'_>>,
        displacement: Option<&voxel_model::DisplacementField>,
        refine_config: Option<&voxel_mesh::RefineConfig>,
        lattice: u32,
    ) -> Scene {
        // **One implementation, stepped to the end.** The frame's own path steps the same builder
        // a slice at a time ([`SceneBuild`]); this is that path with no bound on a step, which is
        // why the two cannot produce different bytes.
        let meshes: Vec<&Mesh> = meshes.into_iter().collect();
        let mut build = SceneBuild::with_shadow_lattice(lattice);
        while !build.step(&meshes, light, displacement, refine_config, usize::MAX) {}
        build.finish()
    }

    /// **Back to the world's own scene**: the previous frame's cloud suffix and bodies are
    /// dropped, and the terrain the cache kept is what remains.
    ///
    /// The counts are the ones [`Scene::build`] left — the terrain's own — so a scene that a cache
    /// hands back twice hands back the same terrain both times, however much the frame appended
    /// to it.
    pub fn begin_frame(&mut self) {
        let [records, origins, vertices, indices] = self.terrain;
        self.records.truncate(records);
        self.origins.truncate(origins);
        self.vertices.truncate(vertices);
        self.indices.truncate(indices);
        self.clouds.clear();
        self.cloud_origins.clear();
        self.far.clear();
        self.far_origins.clear();
        self.bodies.clear();
        self.written_vertices.clear();
        self.written_indices.clear();
        self.written_records.clear();
    }

    /// Build a vertex-format scene from meshes, with **no light field** and no
    /// per-corner detail sample. Displacement is not applied.
    pub fn build_with_detail<'a>(
        meshes: impl IntoIterator<Item = &'a Mesh>,
        eye_m: [f64; 3],
    ) -> Scene {
        Scene::build(meshes, eye_m, None, None)
    }

    /// One quad, placed `distance_m` in front of a camera at `eye_m` and
    /// oriented across the view, but carrying an **arbitrary shading normal**.
    ///
    /// Geometry and shading normal are deliberately separable here. That is what
    /// makes a probe possible: the shader's output for a stated normal can be
    /// compared against `npr::shade` without also having to reason about
    /// whether a quad is edge-on. It is a test fixture, not a world.
    /// **`eye_m` is the camera's own eye, and the probe's chunk stands on it.** A corner is
    /// chunk-local and the vertex stage adds `origin − eye`; a probe whose origin were the world's
    /// zero would be pushed by `−eye` and, for an eye well away from the origin, drawn behind the
    /// camera. Stating the eye as the origin makes the corners already in the eye's frame, which is
    /// what a probe is.
    pub fn probe(
        normal: [f32; 3],
        material: u32,
        distance_m: f32,
        half_m: f32,
        eye_m: [f64; 3],
    ) -> Scene {
        Scene::probe_with(
            normal, material, distance_m, half_m, eye_m, [0.0; 3], [0.0; 3], 0,
        )
    }

    /// **The same probe with a light sample of its own** — the single law's fixture.
    ///
    /// `probe` states a surface with **no measurement**, which the shader lights from the
    /// atmosphere at the face's own normal and by no beam (`shading.wgsl`'s one light path).
    /// This states the other side of the same law: the six bands a corner the sampler measured
    /// carries, in band order (B3, B4, B5) for each triple. It adds no scene format — the
    /// vertices are [`Scene::probe`]'s, and the bands are the ones [`Vertex`] already holds.
    ///
    /// [`Vertex`]: crate::scene::Vertex
    pub fn probe_lit(
        normal: [f32; 3],
        material: u32,
        distance_m: f32,
        half_m: f32,
        eye_m: [f64; 3],
        light_direct: [f32; 3],
        light_diffuse: [f32; 3],
    ) -> Scene {
        Scene::probe_with(
            normal,
            material,
            distance_m,
            half_m,
            eye_m,
            light_direct,
            light_diffuse,
            1,
        )
    }

    fn probe_with(
        normal: [f32; 3],
        material: u32,
        distance_m: f32,
        half_m: f32,
        eye_m: [f64; 3],
        light_direct: [f32; 3],
        light_diffuse: [f32; 3],
        light_present: u32,
    ) -> Scene {
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
                // A screen-space probe is not cut from a block's face; -1 is the
                // "no face" the shader falls back from.
                plane_axis: -1,
                light_direct,
                light_diffuse,
                light_present,
                // The probe's corners are already in the eye's frame, and its one
                // chunk stands on the eye ([`Scene::origins`] entry zero) — so the
                // vertex stage's `origin − eye` is zero and the corners land where
                // they were written.
                chunk: 0,
            })
            .collect();
        // The probe is deliberately **the vertex format**: it exists to state a
        // shading normal, and a normal is not a rectangle's to choose.
        Scene {
            // A probe is a new scene each time it is asked for — a shading test states a
            // normal and reads the pixels — so it states a new build and the GPU sends it.
            build: Scene::next_build(),
            terrain: [0, 1, 4, 6],
            vertices,
            indices: vec![0, 1, 2, 0, 2, 3],
            records: Vec::new(),
            origins: vec![[eye_m[0], eye_m[1], eye_m[2], 0.0]],
            bodies: Vec::new(),
            clouds: Vec::new(),
            cloud_origins: Vec::new(),
            far: Vec::new(),
            far_origins: Vec::new(),
            fresh: true,
            written_vertices: Vec::new(),
            written_indices: Vec::new(),
            written_records: Vec::new(),
            quads: 1,
            bent_quads: 0,
        }
    }

    /// **A fresh build id**, monotonic per process: what the GPU compares to the id of the
    /// corners it last uploaded. A scene that is the same scene is not re-sent.
    fn next_build() -> u64 {
        static BUILDS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        BUILDS.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    /// **Say that this scene's bytes moved**, for a caller that rewrote a scene in place rather
    /// than building one.
    ///
    /// The build id is the pipeline's upload key: a rewritten scene that kept its id is a scene
    /// the frame believes it has already sent, and the buffer it draws is the one from before the
    /// rewrite. Nothing else about the scene changes here — the caller owes the whole array, so
    /// `fresh` is set with it.
    pub fn rebuilt(&mut self) {
        self.build = Self::next_build();
        self.fresh = true;
    }

    /// **The Sun and the Moon appended as records** (C-184), each on its own origin.
    ///
    /// A body's record carries what it looks like and the direction that lights it; where it is comes
    /// from the origin this pushes, so the body travels in the world's own `f64` metres and is narrowed
    /// only after the eye is subtracted at upload.
    pub fn with_bodies(mut self, bodies: &[crate::quad::BodyRecord]) -> Scene {
        self.add_bodies(bodies);
        self
    }

    /// Append the bodies to this scene, the mutating half of [`Scene::with_bodies`].
    pub fn add_bodies(&mut self, bodies: &[crate::quad::BodyRecord]) {
        for body in bodies {
            let chunk = self.origins.len() as u32;
            self.origins.push(body.origin());
            self.bodies.push(QuadRecord::of_body(body, chunk));
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
        self.add_clouds(cubes);
        self
    }

    /// Append one tree's rectangles to this scene, the mutating half of [`Scene::with_clouds`].
    pub fn add_clouds(&mut self, cubes: &crate::cloud_cubes::CloudCubes) {
        self.append_clouds(cubes, false);
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
        mut self,
        from: &crate::cloud_cubes::CloudCubes,
        to: &crate::cloud_cubes::CloudCubes,
    ) -> Scene {
        self.add_cloud_fade(from, to);
        self
    }

    /// Append two trees to this scene, the mutating half of [`Scene::with_cloud_fade`].
    pub fn add_cloud_fade(
        &mut self,
        from: &crate::cloud_cubes::CloudCubes,
        to: &crate::cloud_cubes::CloudCubes,
    ) {
        self.append_clouds(from, true);
        self.append_clouds(to, false);
    }

    /// **Clear the cloud and the bodies** so a resident scene can take the frame's additions
    /// again, leaving the terrain it owns untouched.
    pub fn begin_resident_frame(&mut self) {
        self.begin_frame();
    }

    /// Append one tree's rectangles and their origins, each record naming its own origin.
    ///
    /// **The origins accumulate too**, because a fade appends two trees in one frame and both
    /// are drawn from one cloud buffer: the second tree's origins start where the first's end.
    fn append_clouds(&mut self, cubes: &crate::cloud_cubes::CloudCubes, older: bool) {
        use crate::cloud_cubes::veil_with_side;
        let base = self.cloud_origins.len() as u32;
        for origin in &cubes.origins {
            self.cloud_origins.push([
                origin[0],
                origin[1],
                origin[2],
                veil_with_side(origin[3], older),
            ]);
        }
        for record in &cubes.records {
            let mut record = *record;
            record.chunk += base;
            self.clouds.push(record);
        }
    }

    /// **The far field appended, after the terrain and beside the cloud.**
    ///
    /// Each mesh is one far chunk: its quads are already on the coarse lattice (their planes, `uv`
    /// and extents are multiples of [`crate::far_terrain::FAR_CELL_SUBVOXELS`]), so this is the
    /// conversion the quad format asks for and nothing else — one origin per chunk, one record per
    /// rectangle.
    ///
    /// **The cell size rides the record's spare slot** (`light_y`, which a rectangle leaves zero),
    /// not a new field and not its origin's `w`: the origin's fourth float is the cloud's veil, and
    /// a far chunk is not a cloud. `vs_quad` hands the number to the fragment, and the fragment
    /// uses it as the **floor under its footprint** — a face whose cells are eight metres cannot be
    /// finished by a ladder reading half-metre cells, so the rung it reads is the coarsest rung the
    /// geometry actually holds.
    pub fn with_far<'a>(mut self, meshes: impl IntoIterator<Item = &'a Mesh>) -> Scene {
        self.add_far(meshes);
        self
    }

    /// Append the far field to this scene, the mutating half of [`Scene::with_far`].
    pub fn add_far<'a>(&mut self, meshes: impl IntoIterator<Item = &'a Mesh>) {
        self.add_far_at(meshes, crate::far_terrain::FAR_CELL_M as f32);
    }

    /// **Append a far field cut at its own tier.**
    ///
    /// The cell size is the face's rung and travels in the record's spare slot; a tier-6 mesh
    /// appended through [`Scene::add_far`] would be stamped eight metres and shaded as if its
    /// half-chunk cells were quarter-chunks.
    pub fn add_far_at<'a>(&mut self, meshes: impl IntoIterator<Item = &'a Mesh>, cell_m: f32) {
        assert!(cell_m > 0.0, "a far mesh must state the cell it was cut at");
        for mesh in meshes {
            let origin = mesh.origin_voxel();
            let chunk = self.far_origins.len() as u32;
            self.far_origins
                .push([origin[0] as f64, origin[1] as f64, origin[2] as f64, 0.0]);
            for quad in &mesh.quads {
                let mut record = QuadRecord::of(quad, chunk);
                // The cell size, in the slot a rectangle leaves zero. `light_y` is a record's own
                // spare float; `vs_quad` reads it as the face's rung.
                record.light_y = cell_m.to_bits();
                self.far.push(record);
            }
        }
    }

    /// **Append the surface rungs to this scene** — the ladder's tiers above a chunk.
    ///
    /// A rung's quads are in **sub-cell** units of its own cell
    /// ([`crate::lod::SURFACE_SUB_CELLS`] a cell), so its records state two numbers: the **rung**
    /// in `light_y` — the cell the fragment's light must measure it at — and the **step** in
    /// `light_z`, the metres one coordinate of the rectangle stands for. A rectangle from the fine
    /// path leaves both zero, and a chunk-held far mesh leaves `light_z` zero because its
    /// coordinates are subvoxels.
    pub fn add_surface<'a>(
        &mut self,
        rings: impl IntoIterator<Item = &'a crate::surface::SurfaceRing>,
    ) {
        for ring in rings {
            let origin = ring.origin_m();
            let chunk = self.far_origins.len() as u32;
            self.far_origins
                .push([origin[0], origin[1], origin[2], 0.0]);
            let rung = ring.cell_m as f32;
            let step = ring.step_m() as f32;
            assert!(
                rung > 0.0 && step > 0.0,
                "a surface rung states its cell and its step"
            );
            assert_eq!(
                ring.quads.len(),
                ring.shapes.len(),
                "a rung draws one shaper record a rectangle"
            );
            for (quad, shape) in ring.quads.iter().zip(ring.shapes.iter()) {
                let mut record = QuadRecord::of(quad, chunk);
                record.light_y = rung.to_bits();
                record.light_z = step.to_bits();
                record.shape_lo = shape[0].to_bits();
                record.shape_hi = shape[1].to_bits();
                record.shape_twist = shape[2].to_bits();
                self.far.push(record);
            }
        }
    }

    /// **No geometry at all, in either format.**
    ///
    /// The quad format writes no index and no vertex, so a check on `indices`
    /// alone would report the world's own scene as empty — and a caller that used
    /// this to decide there was nothing to draw would then skip every rectangle
    /// the game draws. The record count is the other half of the same question.
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
            && self.records.is_empty()
            && self.clouds.is_empty()
            && self.far.is_empty()
            && self.bodies.is_empty()
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
        self.vertex_bytes_of(0, self.vertices.len())
    }

    /// **A range of the vertex buffer's bytes**, which is what a sliced upload writes: the
    /// serialisation is the slice's own, so a frame that sends 1.5 MB does not build 20 MB first.
    pub fn vertex_bytes_of(&self, from: usize, to: usize) -> Vec<u8> {
        let to = to.min(self.vertices.len());
        let from = from.min(to);
        let mut out = Vec::with_capacity((to - from) * Vertex::STRIDE as usize);
        for v in &self.vertices[from..to] {
            v.write_bytes(&mut out);
        }
        out
    }

    /// The whole index buffer's bytes, little-endian, four per index.
    pub fn index_bytes(&self) -> Vec<u8> {
        self.index_bytes_of(0, self.indices.len())
    }

    /// A range of the index buffer's bytes, for the same reason.
    pub fn index_bytes_of(&self, from: usize, to: usize) -> Vec<u8> {
        let to = to.min(self.indices.len());
        let from = from.min(to);
        let mut out = Vec::with_capacity((to - from) * 4);
        for i in &self.indices[from..to] {
            out.extend_from_slice(&voxel_model::endian::le_bytes(*i));
        }
        out
    }
}

/// **A scene built in bounded steps**, so the frame owns how much geometry it makes.
///
/// The shipped frame's streaming state used to rebuild every resident mesh on **every frame a
/// chunk arrived** — a full walk of 57 000 quads on the frame, 5–6 ms idle and 70 ms under eight
/// busy cores, while the frame it belonged to already had a scene to draw. That is a burst with
/// no owner: the work was all-or-nothing, so the frame that started it paid for all of it.
///
/// This is the same walk with a cursor. A step emits at most the quads it is given, keeps the
/// half-built scene and the cursor, and reports whether the scene is complete; the frame calls it
/// with the allowance its own budget states and draws the **previous** scene until the new one is
/// finished. Nothing about the output depends on where the steps fall: the quads are visited in
/// the same order, so a build stepped a thousand times is the bytes of a build stepped once —
/// which is what [`Scene::build`] now is, literally (`usize::MAX`).
pub struct SceneBuild {
    scene: Scene,
    /// **Where the walk resumes: the chunk it is at, not an index in a map.** The streamer holds
    /// its meshes in a `HashMap` and gains members while a build runs, so an index would name a
    /// different chunk on the next step. A chunk's own position does not move.
    /// The chunk **and which mesh of it** the walk is in: the streamer can hold two meshes for one
    /// chunk (a fresh one and the stale one it replaced), and an address that named only the chunk
    /// would skip the second one's quads.
    at: Option<(voxel_model::ChunkPos, usize)>,
    quad_at: usize,
    /// **How many cells per voxel edge a face is cut into for its shadow** — `1` is the
    /// once-per-corner path, [`SHADOW_LATTICE`] is one sample per stored subvoxel. See
    /// [`Scene::build_with_light_lattice`].
    shadow_lattice: u32,
}

impl Default for SceneBuild {
    fn default() -> Self {
        Self::new()
    }
}

impl SceneBuild {
    /// A builder that asks each face for its shadow once per corner.
    pub fn new() -> Self {
        Self::with_shadow_lattice(1)
    }

    /// A builder whose vertex-format faces are cut at `shadow_lattice` cells per voxel edge.
    ///
    /// The value is clamped to `1..=SHADOW_LATTICE` and raised to a divisor of
    /// `SUBVOXEL_SIDE`, because a step that is not a whole number of subvoxels would put a
    /// sample on a cell boundary — a coin toss the address law does not make.
    pub fn with_shadow_lattice(shadow_lattice: u32) -> Self {
        let lattice = if shadow_lattice <= 1 {
            1
        } else {
            let side = SHADOW_LATTICE;
            let asked = shadow_lattice.min(side);
            (1..=side)
                .rev()
                .find(|n| side % *n == 0 && *n <= asked)
                .unwrap_or(1)
        };
        Self {
            scene: Scene {
                build: Scene::next_build(),
                fresh: true,
                ..Scene::default()
            },
            at: None,
            quad_at: 0,
            shadow_lattice: lattice,
        }
    }

    /// Emit up to `quads` more quads, and answer **`true` when the scene is complete**.
    ///
    /// `meshes` is sorted by chunk position by the caller, and the cursor is a chunk position
    /// rather than an index — see [`SceneBuild::at`]. A mesh list that gained a chunk since the
    /// last step therefore continues at the right place instead of at the right index.
    ///
    /// A completed build is final: [`SceneBuild::finish`] is the only way to the scene, and the
    /// terrain counts are taken there so a build that ran out of index space leaves by the same
    /// door as one that finished (a scene returned past that line would state `terrain: [0; 4]`
    /// and [`Scene::begin_frame`] would strip the corners the build had just made).
    pub fn step(
        &mut self,
        meshes: &[&Mesh],
        light: Option<&LightAt<'_>>,
        displacement: Option<&voxel_model::DisplacementField>,
        refine_config: Option<&voxel_mesh::RefineConfig>,
        quads: usize,
    ) -> bool {
        // An empty field is the unwired frame, decided once rather than per quad (C-182).
        let stage = match (displacement, refine_config) {
            (Some(disp), Some(cfg)) if !disp.is_empty() => Some((disp, cfg)),
            _ => None,
        };
        let mut emitted = 0usize;
        let key_of = |m: &&Mesh| (m.chunk.x, m.chunk.y, m.chunk.z);
        let mut index = match self.at {
            None => 0,
            Some((pos, ordinal)) => {
                meshes.partition_point(|m| key_of(m) < (pos.x, pos.y, pos.z)) + ordinal
            }
        };
        let mut entered: Option<usize> = None;
        while emitted < quads {
            let Some(mesh) = meshes.get(index) else {
                self.complete();
                return true;
            };
            // **Entering a mesh**: the cursor's own (which may be a fresh mesh for a chunk the
            // build already passed), or the next one. Every mesh gets its origin entry, exactly as
            // the one-step walk does, and its quads start at zero unless the cursor is in it.
            if entered != Some(index) {
                let resumes = match self.at {
                    Some((pos, ordinal)) => {
                        let base = meshes.partition_point(|m| key_of(m) < (pos.x, pos.y, pos.z));
                        mesh.chunk == pos && index == base + ordinal
                    }
                    None => false,
                };
                if !resumes {
                    let o = mesh.origin_voxel();
                    self.scene
                        .origins
                        .push([o[0] as f64, o[1] as f64, o[2] as f64, 0.0]);
                    self.quad_at = 0;
                    // **The ordinal only means something on a sorted list.** A one-step build may
                    // be handed meshes in any order (a test's two faces of one block, say), and the
                    // cursor it leaves is never resumed; saturating keeps that case a value rather
                    // than a panic, and the caller that slices sorts (`main`'s own list).
                    let base = meshes.partition_point(|m| key_of(m) < key_of(&mesh));
                    self.at = Some((mesh.chunk, index.saturating_sub(base)));
                }
                entered = Some(index);
            }
            let Some(quad) = mesh.quads.get(self.quad_at) else {
                index += 1;
                entered = None;
                continue;
            };
            let chunk = (self.scene.origins.len() - 1) as u32;
            if !self.emit(quad, chunk, stage, light) {
                // **Out of index space**, which is the guard's own exit and not a failure: the
                // scene is what fit, and a caller that gets `true` here is told so by the counts.
                self.complete();
                return true;
            }
            self.quad_at += 1;
            emitted += 1;
        }
        false
    }

    /// Quads this build has emitted so far — the cursor's own statement of how much of the world
    /// it has walked, which is what the report prints while a build is in flight.
    pub fn walked(&self) -> usize {
        self.scene.quads
    }

    /// **The finished scene.** A build that has not reported completion is finished here anyway —
    /// the frame may be abandoned (an eye that walked away), and a scene is better than a panic.
    pub fn finish(mut self) -> Scene {
        if self.scene.terrain == [0; 4] {
            self.complete();
        }
        self.scene
    }

    /// **What the world built, and what the frame added.** The terrain is the part a cache may
    /// keep; the cloud and the bodies are appended after it every frame, and this is the
    /// boundary [`Scene::begin_frame`] truncates back to.
    fn complete(&mut self) {
        if self.scene.terrain == [0; 4] {
            self.scene.terrain = [
                self.scene.records.len(),
                self.scene.origins.len(),
                self.scene.vertices.len(),
                self.scene.indices.len(),
            ];
        }
    }

    /// One quad, in the format the caller's light chooses. `false` when the build cannot continue
    /// (the index space is spent).
    fn emit(
        &mut self,
        quad: &voxel_mesh::Quad,
        chunk: u32,
        stage: Option<(&voxel_model::DisplacementField, &voxel_mesh::RefineConfig)>,
        light: Option<&LightAt<'_>>,
    ) -> bool {
        let scene = &mut self.scene;
        // Only a bent quad leaves the record format, and a refusal is never a hole (C-182). The
        // entry is asked for before refining: a face the field does not name has no law to sample.
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
        let origin = scene.origins[chunk as usize];
        let origin = [origin[0], origin[1], origin[2]];
        let push = |scene: &mut Scene,
                    corner_m: [f64; 3],
                    normal: [f32; 3],
                    material: u16,
                    bands: [f32; 6],
                    present: u32| {
            // The face both formats were cut from. A refined triangle is one piece of a quad's
            // face, so it states the same axis the record would.
            let plane_axis = quad.face.axis() as i32;
            // **Chunk-local, like the records**: the shader adds `origins[chunk]`, which is
            // `origin − eye` for the frame being drawn, so this array does not mention the
            // camera at all.
            scene.vertices.push(Vertex {
                position: [corner_m[0] as f32, corner_m[1] as f32, corner_m[2] as f32],
                normal,
                material: material as u32,
                detail_roughness: 0.0,
                // **The face this triangle was cut from, not `-1`.** `-1` is the "no face" a
                // screen-space probe has, and it is not what a refined corner is: the shader
                // reads this as the shaper's stretch axis, so a corner that states none draws
                // the material's pattern stretched along the *world* axis — square cells on the
                // side that looks along it, 4x-long ones on the other two. The same block face
                // therefore changed its pattern whenever the mesh was rebuilt into (or out of)
                // refined triangles: whole faces, one address each, flipping between frames.
                plane_axis,
                light_direct: [bands[0], bands[1], bands[2]],
                light_diffuse: [bands[3], bands[4], bands[5]],
                light_present: present,
                chunk,
            });
        };
        // One corner's light flag and bands, measured **at the corner's own address**. The point
        // is handed over unchanged and `field::SubvoxelPos::of_point` is the only division in the
        // path; the cell it names is a quarter-voxel, so two corners of one face are measured at
        // the two cells the shadow edge actually falls between. A point with a non-finite axis
        // has no cell and reads as unmeasured, which is the rig's own fallback.
        let measure = |world: [f64; 3]| -> (u32, [f32; 6]) {
            match light.and_then(|f| f(world)) {
                Some(bands) => (1u32, bands),
                None => (0u32, [0.0f32; 6]),
            }
        };
        // The world position of a chunk-local subvoxel coordinate.
        let place = |at: [f64; 3]| -> [f64; 3] {
            let m = subvoxel_m(at);
            [origin[0] + m[0], origin[1] + m[1], origin[2] + m[2]]
        };
        // **One rule for a quad's light**, shared by both formats: a quad is field-lit only when
        // every one of its four corners was measured, and each corner carries its own bands.
        // `light_present` reaches the fragment flat, so per-corner flags would split the quad's two
        // triangles down the diagonal whenever coverage differed across it; ANDing keeps both on
        // one branch. The corners are marched once here; a refined triangle's interior vertices are
        // measured at their own positions, because they are not corners.
        let measure_quad = |corners: [[i32; 3]; 4]| -> (u32, [(u32, [f32; 6]); 4]) {
            let mut measured = [(0u32, [0.0f32; 6]); 4];
            for (i, c) in corners.iter().enumerate() {
                measured[i] = measure(place([c[0] as f64, c[1] as f64, c[2] as f64]));
            }
            let present = u32::from(measured.iter().all(|m| m.0 == 1));
            (present, measured)
        };

        match refined {
            Some(triangles) => {
                // The flag comes from the one rule above; subdivided triangles inherit it, so no
                // two triangles of one quad can land on different branches of the fragment's test.
                let (quad_present, _) = measure_quad(quad.corners());
                for tri in &triangles {
                    let base = scene.vertices.len() as u32;
                    if base.checked_add(3).is_none() {
                        return false;
                    }
                    for v in &tri.vertices {
                        let (_, bands) = measure(place([
                            v.position[0] as f64,
                            v.position[1] as f64,
                            v.position[2] as f64,
                        ]));
                        push(
                            scene,
                            subvoxel_m([
                                v.position[0] as f64,
                                v.position[1] as f64,
                                v.position[2] as f64,
                            ]),
                            [v.normal[0], v.normal[1], v.normal[2]],
                            tri.material.0,
                            bands,
                            quad_present,
                        );
                    }
                    scene.indices.extend_from_slice(&[base, base + 1, base + 2]);
                }
            }
            None => match light {
                // **The quad format.** The rectangle travels as itself — 24 chunk-local bytes —
                // and `shading.wgsl`'s `vs_quad` expands it into two triangles. No corner is
                // built here, and no lattice sample is taken: the fragment's own ladder is the
                // whole address law for this geometry.
                None => {
                    scene.records.push(QuadRecord::of(quad, chunk));
                }
                // **The vertex format**, for a frame that carries a light measurement: a corner is
                // where one attaches. The flag and the bands come from the one rule above; bands
                // stay per-corner.
                Some(_) => {
                    let n = quad.normal();
                    let normal = [n[0] as f32, n[1] as f32, n[2] as f32];
                    let corners = quad.corners();
                    // **The face's own rectangle, in subvoxels**: `corners` are in order around
                    // it, so the two edges from the first corner span the face and the bilinear
                    // blend of them is exact (the face is axis-aligned and these are integers).
                    let c0 = [
                        corners[0][0] as f64,
                        corners[0][1] as f64,
                        corners[0][2] as f64,
                    ];
                    let u = [
                        (corners[1][0] - corners[0][0]) as f64,
                        (corners[1][1] - corners[0][1]) as f64,
                        (corners[1][2] - corners[0][2]) as f64,
                    ];
                    let v = [
                        (corners[3][0] - corners[0][0]) as f64,
                        (corners[3][1] - corners[0][1]) as f64,
                        (corners[3][2] - corners[0][2]) as f64,
                    ];
                    let span = |e: [f64; 3]| e.iter().fold(0.0f64, |m, c| m.max(c.abs()));
                    // A cell is `SUBVOXEL_SIDE / lattice` subvoxels on a side; at the finest
                    // lattice that is one subvoxel. `1` is not a lattice at all — it is the
                    // once-per-face path, one cell for the whole rectangle, which is the old
                    // bytes exactly and the thing the two paths are compared against.
                    let step = (self.shadow_lattice > 1)
                        .then(|| (SHADOW_LATTICE / self.shadow_lattice) as f64);
                    let cells = |extent: f64| match step {
                        Some(step) => (extent / step).round().max(1.0) as usize,
                        None => 1,
                    };
                    let nu = cells(span(u));
                    let nv = cells(span(v));
                    let base = scene.vertices.len() as u32;
                    if base.checked_add(4 * (nu * nv) as u32).is_none() {
                        return false;
                    }
                    // **The samples are the grid's own vertices, measured once each.** An
                    // interior sample is shared by four cells; asking per cell would multiply
                    // the frame's asks by four for the same answer.
                    let point = |i: usize, j: usize| -> [f64; 3] {
                        let fi = i as f64 / nu as f64;
                        let fj = j as f64 / nv as f64;
                        [
                            c0[0] + u[0] * fi + v[0] * fj,
                            c0[1] + u[1] * fi + v[1] * fj,
                            c0[2] + u[2] * fi + v[2] * fj,
                        ]
                    };
                    let mut sampled = vec![(0u32, [0.0f32; 6]); (nu + 1) * (nv + 1)];
                    for j in 0..=nv {
                        for i in 0..=nu {
                            sampled[j * (nu + 1) + i] = measure(place(point(i, j)));
                        }
                    }
                    // **A face the light does not change keeps its four corners.** Equal samples
                    // are one answer for the whole rectangle, and four corners are then its exact
                    // geometry: a merged rectangle in open light, or wholly inside one shadow,
                    // costs four corners.
                    //
                    // **The test is the whole lattice, not only the four corners.** The corners
                    // are a merged rectangle's coarsest samples; a branch's shadow, a wall's
                    // edge or any shadow smaller than the rectangle can lie wholly between them
                    // with all four corners lit, and four equal corners draw one flat rectangle —
                    // the shadow is not dark, it is absent. So neighbouring **lattice** samples
                    // are compared, and a face is cut when any adjacent pair differs. The grid
                    // was already sampled above to fill `sampled`, so this costs one pass over
                    // the grid's own adjacencies and **no extra field ask**; what it can add is
                    // vertices, and only on a face whose interior carries an edge: `(nu + 1)(nv
                    // + 1)` corners and `6·nu·nv` indices instead of four and six, i.e. 16 641
                    // corners against four on a 32 m face at the 0.25 m lattice. That is the
                    // rebuild's own cost, and the frame's corner count and the arena's wall are
                    // where a window pays it.
                    //
                    // **Bands are the test; presence is the gate above it.** `quad_present`
                    // already keeps a face with an unmeasured corner on four corners. Inside a
                    // face measured at all four, an unmeasured lattice sample carries zero bands
                    // and is compared like any other value; the flag pass below rigs its
                    // neighbourhood, which is the same rule one step finer rather than a second
                    // crossing rule.
                    let crosses_lattice = || {
                        let along = |row: usize| {
                            let at = row * (nu + 1);
                            (0..nu).any(|i| sampled[at + i].1 != sampled[at + i + 1].1)
                        };
                        let down = |i: usize| {
                            (0..nv).any(|j| {
                                let at = j * (nu + 1) + i;
                                sampled[at].1 != sampled[at + nu + 1].1
                            })
                        };
                        (0..=nv).any(along) || (0..=nu).any(down)
                    };
                    //
                    // **And an unmeasured corner is not a cross.** The grid is the same rule as
                    // `quad_present`, one step finer: where a corner has no answer, the face is
                    // the rig's, and cutting it into `(cells + 1)²` samples measures a shadow
                    // that will be drawn as the rig anyway. **What it is worth, stated:** on the
                    // settled window this condition alone barely moved the live count (2.07 M
                    // against 2.05 M), because the field answers a stale cell for most corners
                    // and so most faces are *known*; it is the right rule for a frontier face and
                    // not the explanation of the window's size. The size is the lattice itself,
                    // and the capacity below is what holds it.
                    let (quad_present, samples) = measure_quad(corners);
                    let crossed = step.is_some() && quad_present == 1 && crosses_lattice();
                    if !crossed {
                        let base = scene.vertices.len() as u32;
                        if base.checked_add(4).is_none() {
                            return false;
                        }
                        for (corner, (_, bands)) in corners.iter().zip(samples) {
                            push(
                                scene,
                                subvoxel_m([corner[0] as f64, corner[1] as f64, corner[2] as f64]),
                                normal,
                                quad.material.0,
                                bands,
                                quad_present,
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
                    } else {
                        // **The grid is shared, not four corners a cell.** A cell's corner is its
                        // neighbour's corner, and pushing one vertex per cell's corner spent four
                        // copies of every interior sample: a 32 m face at the subvoxel lattice cost
                        // 65 536 vertices for a grid of 16 641. One vertex per sample and an index
                        // buffer is the same picture at a quarter of the bytes.
                        let base = scene.vertices.len() as u32;
                        let grid_vertices = (nu + 1) * (nv + 1);
                        if base.checked_add(grid_vertices as u32).is_none() {
                            return false;
                        }
                        // **A shared corner carries the flag its whole neighbourhood earns.**
                        // The flag reaches the fragment flat from a triangle's first vertex, so a
                        // cell's two triangles must read the same bit; a corner is shared by up to
                        // four cells and cannot carry one cell's bit alone. This is the AND over
                        // the samples around it — the cell's own four are always among them, so a
                        // cell with an unmeasured corner rigs all four of its corners through a
                        // single value, and a fully measured cell keeps the field unless a
                        // neighbour's sample was unmeasured. That is the quad rule, one cell
                        // smaller.
                        let mut flags = vec![1u32; sampled.len()];
                        for j in 0..=nv {
                            for i in 0..=nu {
                                let mut flag = 1u32;
                                for dj in j.saturating_sub(1)..=(j + 1).min(nv) {
                                    for di in i.saturating_sub(1)..=(i + 1).min(nu) {
                                        flag &= sampled[dj * (nu + 1) + di].0;
                                    }
                                }
                                flags[j * (nu + 1) + i] = flag;
                            }
                        }
                        for j in 0..=nv {
                            for i in 0..=nu {
                                let at = j * (nu + 1) + i;
                                let (_, bands) = sampled[at];
                                push(
                                    scene,
                                    subvoxel_m(point(i, j)),
                                    normal,
                                    quad.material.0,
                                    bands,
                                    flags[at],
                                );
                            }
                        }
                        for j in 0..nv {
                            for i in 0..nu {
                                let at = j * (nu + 1) + i;
                                // Both triangles of the cell start at its own low corner, so the
                                // flat flag is that corner's and the diagonal cannot split them.
                                let corner = [at, at + 1, at + nu + 2, at + nu + 1];
                                scene.indices.extend_from_slice(&[
                                    (corner[0] + base as usize) as u32,
                                    (corner[1] + base as usize) as u32,
                                    (corner[2] + base as usize) as u32,
                                    (corner[0] + base as usize) as u32,
                                    (corner[2] + base as usize) as u32,
                                    (corner[3] + base as usize) as u32,
                                ]);
                            }
                        }
                    }
                }
            },
        }
        scene.quads += 1;
        true
    }
}

/// **What a scene is a function of**: the world's own generation, its residency, and the
/// light's revision.
///
/// Nothing else. A corner is chunk-local and the eye travels in the origins array, so walking
/// does not move the key; a recorded edit moves `world`; a chunk arriving, leaving, being
/// remeshed or being demoted to stale moves `residency` — which is the one of the two the
/// **corners** are a function of, and the reason both are stated: a key on the record alone
/// would keep drawing a window the streamer had already filled; a cell the light field measured
/// moves `light`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct SceneKey {
    /// The world record's generation — `voxel_work`'s `world_generation`, which moves on an
    /// edit and on nothing else.
    pub world: u64,
    /// The streamer's own count of meshes entering and leaving — `voxel_work`'s
    /// `residency_generation`.
    pub residency: u64,
    /// The light's own revision: the tick the rig and the light field were derived at, so a
    /// frame built in one atmosphere cadence is not kept into the next.
    pub light: u64,
}

impl SceneKey {
    /// **The light's own revision, as the corners are a function of it.**
    ///
    /// A corner carries light only when the field **measured** the cell it stands in: the bands are
    /// per corner and the exposure grade scales them. A field that holds **nothing** — no cell
    /// measured, so every corner falls back to the rig through the uniform — cannot change a
    /// corner's bytes at any tick, and the rig is live through `write_rig` whether the geometry was
    /// rebuilt or not.
    ///
    /// So the cadence moves this key **only when the field has something to say**. Before that rule
    /// the atmosphere's tick moved the key four times a second and every cadence frame walked every
    /// resident mesh and re-uploaded the scene to produce **the same bytes**, because the field it
    /// was rebuilding for measured zero cells. That was 1.4–3.6 ms a frame attributed to the
    /// atmosphere and worth nothing.
    ///
    /// **And it is the tick, not the revision, because a rebuild is also an ask.** A build walks
    /// every corner, and `CellField::at` answers a stale one *and queues it for a re-march* — so the
    /// cadence rebuild is what refreshes the field the frame is drawing. A key on adoptions alone
    /// froze the picture at the light of the last adoption: a scene the key never moves is a scene
    /// that never notices its cells are a step behind.
    pub fn light_of(held_cells: usize, rig_tick: u64) -> u64 {
        if held_cells == 0 {
            0
        } else {
            rig_tick
        }
    }
}

/// **The owner of the frame's terrain scene.** It builds when the key moves and keeps the scene
/// otherwise, so the frame loop never walks a world that has not changed — and because a kept
/// scene carries the build id the GPU last uploaded, neither does the upload.
///
/// The frame takes the scene out ([`SceneCache::take`]) and puts it back ([`SceneCache::put`])
/// after its own cloud and bodies are appended: one value, one owner, no second copy of 14 MB.
#[derive(Default)]
pub struct SceneCache {
    scene: Option<Scene>,
    key: Option<SceneKey>,
}

impl SceneCache {
    /// The world's scene for `key`, built by `build` only when the key has moved.
    ///
    /// `build` returns the scene and whether it may be kept: a build whose corners the field could
    /// not answer yet has to be asked again once a measurement lands, and the field's revision is
    /// what says one has.
    /// **The key the held scene was built for**, or `None` when there is no scene yet. The frame
    /// asks this before it starts a build: a build in progress is a build for a key, and the one
    /// the cache holds is what is being drawn meanwhile.
    pub fn key(&self) -> Option<SceneKey> {
        self.key
    }

    /// The held scene, stripped back to the world's own part, for the frame to draw.
    pub fn current(&mut self) -> Option<Scene> {
        self.scene.take().map(|mut scene| {
            scene.begin_frame();
            scene
        })
    }

    /// **The synchronous path**: the world's scene for `key`, built by `build` when the key has
    /// moved.
    ///
    /// For a caller with no frame to protect — a test, an example, an offscreen render. The
    /// shipped frame does **not** use it: it steps a [`SceneBuild`] under its own budget and
    /// adopts the result with [`SceneCache::adopt`], so a burst never lands on the frame.
    pub fn take(&mut self, key: SceneKey, build: impl FnOnce() -> (Scene, bool)) -> Scene {
        if self.key != Some(key) || self.scene.is_none() {
            let (scene, keep) = build();
            self.scene = Some(scene);
            self.key = keep.then_some(key);
        }
        self.current().expect("just built")
    }

    /// **Adopt a completed build**, which is the only way a new key reaches the cache: the build
    /// itself is stepped by the frame's budget ([`crate::SceneBuild`]), not run here.
    pub fn adopt(&mut self, key: SceneKey, mut scene: Scene) {
        scene.begin_frame();
        self.scene = Some(scene);
        self.key = Some(key);
    }

    /// Put the frame's scene back, its cloud and bodies included; the next [`SceneCache::take`]
    /// strips them and hands back the terrain.
    pub fn put(&mut self, scene: Scene) {
        self.scene = Some(scene);
    }
}

/// **The ranges of one array that no chunk owns any more**, sorted and merged.
///
/// A departure does not shift anything: its slice becomes a hole and the next landing whose slice
/// fits takes it. The alternative — moving the last chunk's slice into the gap — is only right when
/// the two slices are the same size, and a chunk whose refinement bent nothing is a handful of
/// elements beside a neighbour of hundreds. So the arrays are an arena, not a stack, and the draw
/// still reads one dense range because a hole is tombstoned rather than left with stale bytes.
type Free = Vec<[u32; 2]>;

/// Where a slice of `len` elements goes: **the smallest hole that fits**, else the tail.
fn place(free: &mut Free, len: usize, tail: usize) -> u32 {
    let len = len as u32;
    if len > 0 {
        let mut best: Option<usize> = None;
        for (i, r) in free.iter().enumerate() {
            let size = r[1] - r[0];
            if size >= len && best.is_none_or(|b| size < free[b][1] - free[b][0]) {
                best = Some(i);
            }
        }
        if let Some(i) = best {
            let start = free[i][0];
            if free[i][1] - free[i][0] == len {
                free.remove(i);
            } else {
                free[i][0] += len;
            }
            return start;
        }
    }
    tail as u32
}

/// **Take a slice back**: the range joins the holes it touches, so a walk's own footprint stays
/// contiguous and the next window can land into it whole.
fn release(free: &mut Free, range: [u32; 2]) {
    if range[1] == range[0] {
        return;
    }
    let mut range = range;
    let mut at = free.partition_point(|r| r[1] < range[0]);
    if at > 0 && free[at - 1][1] == range[0] {
        at -= 1;
        range[0] = free[at][0];
        free.remove(at);
    }
    while at < free.len() && free[at][0] == range[1] {
        range[1] = free[at][1];
        free.remove(at);
    }
    free.insert(at, range);
}

/// **Give the tail back**: while the last hole reaches the array's end, the array ends where the
/// hole begins. A chunk that leaves at the end shrinks the array instead of leaving a tombstone
/// the draw reads every frame.
///
/// `floor` is where the last **live** slice ends, and it is what makes the rule safe: a chunk with
/// no corners of its own owns an empty range, which may stand at the array's own end, and cutting
/// the array to a hole that merely touches that offset would leave a live chunk's range starting
/// past the array.
fn trim(free: &mut Free, len: usize, floor: usize) -> usize {
    let mut len = len;
    while let Some(last) = free.last() {
        if last[1] as usize == len && last[0] as usize >= floor {
            len = last[0] as usize;
            free.pop();
        } else {
            break;
        }
    }
    len
}

/// Where the last live slice of an array ends — the lowest offset [`trim`] may cut to.
fn live_end(ranges: impl Iterator<Item = [u32; 2]>) -> usize {
    ranges.map(|r| r[1]).max().unwrap_or(0) as usize
}

/// **How much dead space an arena may carry before it is compacted**: a quarter of the live
/// elements, plus this floor so a scene of a few chunks does not compact on every landing.
///
/// The free lists serve a landing from the smallest hole that fits, and a light refresh can change
/// a chunk's own size (a face whose light stops varying falls from a grid to four corners), so the
/// slices' sizes move under the holes. A perfect-fit allocator over changing sizes fragments; the
/// bound is what keeps that fragmentation from being a leak — past it the arrays are rewritten in
/// origin order and the holes are gone in one pass.
///
/// **A quarter, not a doubling.** The old bound was twice the live elements, which was a bound on
/// the array *relative to the live set* and not on the array — and the live set was itself 2.07 M
/// corners, so the window's array was allowed to reach 4.15 M, past the 3.95 M a 256 MiB corner
/// buffer holds. The compactor would then have fired only after the device had already been asked
/// for a buffer it cannot make. The array now peaks at 1.25× live, so the capacity is a bound on
/// the geometry and not a coincidence: 2.6 M corners at the settled window, 178 MB of 256 MiB.
const ARENA_DEAD_IN: usize = 4;
const ARENA_FLOOR: usize = 4_096;

/// **Write a slice at its place**, growing the array when the place is past the end. Nothing else
/// in the array is touched, which is what makes a landing its own cost rather than the world's.
fn put<T: Clone>(array: &mut Vec<T>, base: u32, slice: &[T], blank: impl Fn() -> T) {
    let base = base as usize;
    let end = base + slice.len();
    if end > array.len() {
        array.resize_with(end, blank);
    }
    array[base..end].clone_from_slice(slice);
}

/// **One resident chunk's own slice of the three arrays** — see [`ResidentScene`].
#[derive(Clone, Copy, Debug)]
struct Block {
    version: u64,
    /// The chunk's position in the scene's own chunk order, which is also its origin index.
    origin: u32,
    vertices: [u32; 2],
    indices: [u32; 2],
    records: [u32; 2],
    quads: u32,
    bent: u32,
}

/// What one [`ResidentScene::sync`] did, for the frame's report.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Residency {
    /// How many chunks landed this sync.
    pub landed: usize,
    /// How many chunks left this sync.
    pub left: usize,
    /// How many chunks had their light refreshed.
    pub refreshed: usize,
    /// How many landed chunks the frame's budget did not reach.
    pub pending: usize,
    /// Whether any of the scene's arrays moved.
    pub changed: bool,
}

/// **The scene the player is standing in, kept across frames and changed only where the world
/// changed.**
///
/// [`SceneBuild`] walks every resident mesh when the world moves, and the frame draws the previous
/// scene until the walk finishes — so while the player walks, the ground under them can be absent
/// for the whole build. This owns the arrays instead: a chunk that lands appends its own vertices,
/// indices, records and origin; a chunk that leaves is swap-removed with the last chunk's slice
/// moved into its place, its `chunk` slot rebound. Nothing else is touched, and the frame draws the
/// result in the same beat.
///
/// **The arrays stay dense**, so one `draw_indexed` and one record draw still cover the world: the
/// swap is the whole rebase, and it is O(one chunk) rather than O(the window).
///
/// **Landings are budgeted by distance**, so a window step that lands a hundred and five chunks
/// lands the ones under the eye first and carries the rest to the next frames; the eye's own chunk
/// never waits behind a far one. The light's own refresh is budgeted the same way, round-robin: a
/// measured band is at most one pass behind, and no frame pays for the whole window.
pub struct ResidentScene {
    scene: Option<Scene>,
    blocks: std::collections::BTreeMap<voxel_model::ChunkPos, Block>,
    /// The three arrays' own free space — see [`Free`].
    free_vertices: Free,
    free_indices: Free,
    free_records: Free,
    /// Chunks whose corners the current light revision has not reached yet.
    light_stale: std::collections::BTreeSet<voxel_model::ChunkPos>,
    /// Landed chunks the frame's budget did not reach, nearest first.
    pending: Vec<voxel_model::ChunkPos>,
    light_epoch: u64,
    /// **How many times the arena's dead space was reclaimed** — see
    /// [`ResidentScene::compact_if_needed`]. A growing count on a settled window would be a
    /// scene whose slices keep changing size; a count of zero says the free lists always fitted.
    compactions: u64,
    /// **The shadow lattice every chunk is emitted at** — the frame's own request, kept here so
    /// a chunk that lands later is cut the same way as the ones already drawn. See
    /// [`Scene::build_with_light_lattice`].
    shadow_lattice: u32,
}

impl Default for ResidentScene {
    fn default() -> Self {
        Self::new()
    }
}

impl ResidentScene {
    /// A resident scene with no chunks and the once-per-corner lattice.
    pub fn new() -> Self {
        Self {
            scene: Some(Scene {
                fresh: false,
                ..Scene::default()
            }),
            blocks: std::collections::BTreeMap::new(),
            free_vertices: Free::new(),
            free_indices: Free::new(),
            free_records: Free::new(),
            light_stale: std::collections::BTreeSet::new(),
            pending: Vec::new(),
            light_epoch: 0,
            compactions: 0,
            shadow_lattice: 1,
        }
    }

    /// **Ask every face for its shadow at `lattice` cells per voxel edge.**
    ///
    /// `1` is the once-per-corner path; [`SHADOW_LATTICE`] is one sample per stored subvoxel.
    /// The chunks already drawn were cut at the old lattice, so all of them are marked for
    /// re-emission — the change is a change to the geometry's bytes, not only to its light.
    pub fn set_shadow_lattice(&mut self, lattice: u32) {
        let lattice = SceneBuild::with_shadow_lattice(lattice).shadow_lattice;
        if lattice == self.shadow_lattice {
            return;
        }
        self.shadow_lattice = lattice;
        self.light_stale = self.blocks.keys().copied().collect();
    }

    /// The scene, for the frame that is about to draw it.
    pub fn scene(&self) -> &Scene {
        self.scene.as_ref().expect("held")
    }

    /// **Take the scene for the frame's own additions** — the cloud and the bodies are the frame's
    /// and are stripped again at the next sync.
    pub fn take(&mut self) -> Scene {
        self.scene.take().expect("held")
    }

    /// Put it back after the draw.
    pub fn put(&mut self, scene: Scene) {
        self.scene = Some(scene);
    }

    /// **How many corners the resident scene holds** — live slices and dead holes together, the
    /// number the corner buffers are sized from. The window's report states it so a growth that is
    /// not the world's is a number a reader can see, rather than a validation error after the
    /// device has already refused the buffer.
    pub fn corners(&self) -> usize {
        self.scene
            .as_ref()
            .map(|scene| scene.vertices.len())
            .unwrap_or(0)
    }

    /// **How many indices the resident scene holds** — the draw's own range, and the second half
    /// of the corner buffers' size.
    pub fn indices(&self) -> usize {
        self.scene
            .as_ref()
            .map(|scene| scene.indices.len())
            .unwrap_or(0)
    }

    /// **How many corners the resident chunks' own slices hold** — live geometry, without the
    /// arena's holes. [`ResidentScene::corners`] is the array; the difference is the dead space
    /// [`ResidentScene::compact_if_needed`] bounds, and a report states both so a growth is
    /// attributable to the world or to the arena.
    pub fn live_corners(&self) -> usize {
        self.blocks
            .values()
            .map(|block| (block.vertices[1] - block.vertices[0]) as usize)
            .sum()
    }

    /// **How many times the arena was compacted** — the count [`ResidentScene::compact_if_needed`]
    /// states, so a run can say whether its dead space was reclaimed at all.
    pub fn compactions(&self) -> u64 {
        self.compactions
    }

    /// **Whether a chunk's slice is in the scene being drawn** — the question the demo walk asks
    /// every frame about the chunk the eye stands in.
    pub fn holds(&self, pos: voxel_model::ChunkPos) -> bool {
        self.blocks.contains_key(&pos)
    }

    /// How many chunks the resident scene is drawing, streamed slices and no far field.
    pub fn chunks(&self) -> usize {
        self.blocks.len()
    }

    /// **The arrays and the blocks are one object**, checked after every sync in a debug build.
    ///
    /// A swap-remove is arithmetic on offsets, and an offset that is one element out does not fail
    /// where it is made: it draws a chunk at another chunk's place, or it truncates a slice a
    /// survivor still owns and panics a frame later inside `copy_within` — which is how the walk
    /// in the playtest ended. This states the invariant in the place it can be checked: the array
    /// is the blocks' slices end to end in origin order, each slice is its chunk's own range, and
    /// every element names the origin it stands on.
    #[cfg(debug_assertions)]
    fn check(&self) {
        let scene = self.scene.as_ref().expect("held");
        assert_eq!(
            self.blocks.len(),
            scene.origins.len(),
            "{} chunk(s) and {} origin(s): one of them is unowned",
            self.blocks.len(),
            scene.origins.len()
        );
        if self.blocks.is_empty() {
            assert!(
                scene.vertices.is_empty()
                    && scene.indices.is_empty()
                    && scene.records.is_empty()
                    && self.free_vertices.is_empty()
                    && self.free_indices.is_empty()
                    && self.free_records.is_empty(),
                "a scene with no chunks is still holding their elements"
            );
            return;
        }
        /// **Every element is either a live chunk's or a hole**, exactly once, with no gap: the
        /// arena is the whole array and the draw reads all of it.
        fn accounted(
            live: impl Iterator<Item = [u32; 2]>,
            holes: &[[u32; 2]],
            len: usize,
            what: &str,
        ) {
            // **An empty range owns no element** — a chunk with no corners has one, and it may
            // stand anywhere inside another chunk's range or a hole. It is in the arena's books
            // without being part of its contents, so the sweep is over what occupies bytes.
            let mut ranges: Vec<[u32; 2]> = live.filter(|r| r[1] > r[0]).collect();
            ranges.extend(holes.iter().copied().filter(|r| r[1] > r[0]));
            ranges.sort_unstable();
            let mut cursor = 0usize;
            for r in &ranges {
                assert_eq!(
                    r[0] as usize, cursor,
                    "a {what} range begins at {} where the last ends at {cursor}; ranges {ranges:?} of {len}",
                    r[0]
                );
                assert!(r[1] >= r[0], "a {what} range runs backwards");
                cursor = r[1] as usize;
            }
            assert_eq!(
                cursor, len,
                "{what}s at the end of the array belong to nothing; ranges {ranges:?} of {len}"
            );
        }
        let blocks: Vec<Block> = self.blocks.values().copied().collect();
        accounted(
            blocks.iter().map(|b| b.vertices),
            &self.free_vertices,
            scene.vertices.len(),
            "corner",
        );
        accounted(
            blocks.iter().map(|b| b.indices),
            &self.free_indices,
            scene.indices.len(),
            "index",
        );
        accounted(
            blocks.iter().map(|b| b.records),
            &self.free_records,
            scene.records.len(),
            "rectangle",
        );
        for b in &blocks {
            assert!(
                (b.origin as usize) < scene.origins.len(),
                "a chunk names a forgotten origin"
            );
            for (range, len, what) in [
                (b.vertices, scene.vertices.len(), "corner"),
                (b.indices, scene.indices.len(), "index"),
                (b.records, scene.records.len(), "rectangle"),
            ] {
                assert!(
                    range[0] <= range[1] && range[1] as usize <= len,
                    "a chunk's {what} range {range:?} is outside the array's {len}"
                );
            }
            for v in &scene.vertices[b.vertices[0] as usize..b.vertices[1] as usize] {
                assert_eq!(v.chunk, b.origin, "a corner names another chunk's origin");
            }
            for i in &scene.indices[b.indices[0] as usize..b.indices[1] as usize] {
                assert!(
                    *i >= b.vertices[0]
                        && *i < b.vertices[1]
                        && scene.vertices[*i as usize].chunk == b.origin,
                    "an index points at a corner outside its own chunk"
                );
            }
            for r in &scene.records[b.records[0] as usize..b.records[1] as usize] {
                assert_eq!(
                    r.chunk, b.origin,
                    "a rectangle names another chunk's origin"
                );
            }
        }
        // **A hole covers nothing.** The draw reads these bytes, so a stale corner or a rectangle
        // with an extent would draw a chunk that has left the world.
        for hole in &self.free_indices {
            assert!(
                scene.indices[hole[0] as usize..hole[1] as usize]
                    .iter()
                    .all(|i| *i == 0),
                "an index in a hole is a live chunk's corner"
            );
        }
        for hole in &self.free_records {
            assert!(
                scene.records[hole[0] as usize..hole[1] as usize]
                    .iter()
                    .all(|r| r.extent == 0),
                "a rectangle in a hole has an extent"
            );
        }
    }

    /// **Apply the world's diff**: chunks that left, chunks that landed or were remeshed, and the
    /// light's own refresh, each bounded by `quads` except that the nearest landing always fits.
    ///
    /// `drawn` is the streamer's own drawn set — fresh and stale meshes together, each with a
    /// version that moves when its bytes do — so a chunk whose mesh was replaced is a landing and a
    /// chunk that vanished is a leave, with no whole-window comparison.
    #[allow(clippy::too_many_arguments)]
    pub fn sync<'a>(
        &mut self,
        drawn: impl Iterator<Item = (voxel_model::ChunkPos, u64, &'a Mesh)>,
        centre: voxel_model::ChunkPos,
        light: Option<&LightAt<'_>>,
        displacement: Option<&voxel_model::DisplacementField>,
        refine: Option<&voxel_mesh::RefineConfig>,
        quads: usize,
        light_epoch: u64,
    ) -> Residency {
        let mut report = Residency::default();
        {
            let scene = self.scene.as_mut().expect("held");
            scene.begin_resident_frame();
        }
        let current: std::collections::BTreeMap<_, _> = drawn
            .map(|(pos, version, mesh)| (pos, (version, mesh)))
            .collect();
        if self.light_epoch != light_epoch {
            self.light_epoch = light_epoch;
            self.light_stale = self.blocks.keys().copied().collect();
        }
        // 1. What left the world goes now: removing a block never needs the light.
        let gone: Vec<_> = self
            .blocks
            .keys()
            .filter(|pos| !current.contains_key(pos))
            .copied()
            .collect();
        for pos in gone {
            self.remove(pos);
            report.left += 1;
        }
        // **A chunk whose mesh bytes moved was remeshed** — an edit's replacement, or a stale mesh
        // handed over. Its old slice is dropped and the new mesh lands below; leaving it would draw
        // the world the edit removed.
        let replaced: Vec<_> = current
            .iter()
            .filter(|(pos, (version, _))| self.blocks.get(pos).map(|b| b.version) != Some(*version))
            .map(|(pos, _)| *pos)
            .collect();
        for pos in replaced {
            self.remove(pos);
        }
        // 2. What is still on its way back from a previous frame's budget, nearest first, and
        //    only while it is still drawn.
        self.pending
            .retain(|pos| current.contains_key(pos) && !self.blocks.contains_key(pos));
        let mut landed: Vec<voxel_model::ChunkPos> = current
            .keys()
            .filter(|pos| !self.blocks.contains_key(pos) && !self.pending.contains(pos))
            .copied()
            .collect();
        landed.sort_unstable_by_key(|pos| distance_squared(*pos, centre));
        let mut queue: Vec<voxel_model::ChunkPos> = std::mem::take(&mut self.pending);
        queue.extend(landed);
        // **The whole queue in distance order, carried chunks included.** A chunk the previous
        // frame's budget deferred was nearest *then*; the ghost has moved since, and keeping the
        // old order can put a far carried chunk ahead of the one the eye now stands in — the one
        // arrival the budget must never defer. Sorting a handful of positions is what makes
        // "nearest first" a property of the queue rather than of the frame that built it.
        queue.sort_unstable_by_key(|pos| distance_squared(*pos, centre));
        queue.dedup();
        let mut budget = quads;
        for pos in queue {
            let Some(&(version, mesh)) = current.get(&pos) else {
                continue;
            };
            let cost = mesh.quads.len();
            if report.landed > 0 && cost > budget {
                self.pending.push(pos);
                continue;
            }
            if self.add(pos, version, mesh, light, displacement, refine) {
                report.landed += 1;
                budget = budget.saturating_sub(cost);
            }
        }
        // 3. The light, round-robin: a chunk whose bands are a revision behind is re-emitted in
        //    place, and the pass costs a frame only the chunks it reaches.
        let mut stale: Vec<voxel_model::ChunkPos> = self.light_stale.iter().copied().collect();
        stale.sort_unstable_by_key(|pos| distance_squared(*pos, centre));
        for pos in stale {
            let Some(&(version, mesh)) = current.get(&pos) else {
                self.light_stale.remove(&pos);
                continue;
            };
            if self.blocks.get(&pos).map(|b| b.version) != Some(version) {
                self.light_stale.remove(&pos);
                continue;
            }
            let cost = mesh.quads.len();
            if report.refreshed > 0 && cost > budget {
                break;
            }
            self.reemit(pos, mesh, light, displacement, refine);
            self.light_stale.remove(&pos);
            report.refreshed += 1;
            budget = budget.saturating_sub(cost);
        }
        report.pending = self.pending.len();
        report.changed = report.landed + report.left + report.refreshed > 0;
        // **The arena's dead space has a ceiling.** A landing takes the smallest hole that fits and
        // otherwise appends, and the light can change a chunk's own size, so the arrays accumulate
        // holes nobody can fill. Past a quarter of the live elements they are compacted in origin
        // order; after that the arrays are exactly the live ones and the holes are gone. Without
        // this the shipped window's corners grew from 1.3 M to 14.4 M in fifteen seconds and
        // outgrew the device's largest buffer — and at a doubling it still climbed past the
        // 256 MiB buffer's own 3.95 M while the compactor was waiting for 4.15 M.
        if self.compact_if_needed() {
            report.changed = true;
        }
        {
            let scene = self.scene.as_mut().expect("held");
            scene.quads = self.blocks.values().map(|b| b.quads as usize).sum();
            scene.bent_quads = self.blocks.values().map(|b| b.bent as usize).sum();
            scene.terrain = [
                scene.records.len(),
                scene.origins.len(),
                scene.vertices.len(),
                scene.indices.len(),
            ];
            scene.build = if report.changed {
                Scene::next_build()
            } else {
                scene.build
            };
            scene.fresh = false;
        }
        #[cfg(debug_assertions)]
        self.check();
        report
    }

    /// **The chunk's own slice of the scene**: one walk of one mesh, through the same builder the
    /// whole-scene path uses, then copied into this scene at the next free offsets.
    #[allow(clippy::too_many_arguments)]
    fn add(
        &mut self,
        pos: voxel_model::ChunkPos,
        version: u64,
        mesh: &Mesh,
        light: Option<&LightAt<'_>>,
        displacement: Option<&voxel_model::DisplacementField>,
        refine: Option<&voxel_mesh::RefineConfig>,
    ) -> bool {
        let one = one_chunk(mesh, light, displacement, refine, self.shadow_lattice);
        let scene = self.scene.as_mut().expect("held");
        let origin = scene.origins.len() as u32;
        // **A place, not necessarily the end**: the smallest hole that fits, so a walk that has
        // been through a column lands into the footprint it left instead of growing the array.
        let vertex_base = place(
            &mut self.free_vertices,
            one.vertices.len(),
            scene.vertices.len(),
        );
        let index_base = place(
            &mut self.free_indices,
            one.indices.len(),
            scene.indices.len(),
        );
        let record_base = place(
            &mut self.free_records,
            one.records.len(),
            scene.records.len(),
        );
        // The origin the one-chunk build made, or the mesh's own if the chunk has no geometry.
        let placed = one.origins.first().copied().unwrap_or_else(|| {
            let o = mesh.origin_voxel();
            [o[0] as f64, o[1] as f64, o[2] as f64, 0.0]
        });
        scene.origins.push(placed);
        let corners: Vec<Vertex> = one
            .vertices
            .iter()
            .map(|v| Vertex {
                chunk: origin,
                ..*v
            })
            .collect();
        put(&mut scene.vertices, vertex_base, &corners, Vertex::blank);
        let indices: Vec<u32> = one.indices.iter().map(|i| i + vertex_base).collect();
        put(&mut scene.indices, index_base, &indices, || 0);
        let records: Vec<QuadRecord> = one
            .records
            .iter()
            .map(|r| QuadRecord {
                chunk: origin,
                ..*r
            })
            .collect();
        put(&mut scene.records, record_base, &records, QuadRecord::blank);
        scene
            .written_vertices
            .push([vertex_base, vertex_base + corners.len() as u32]);
        scene
            .written_indices
            .push([index_base, index_base + indices.len() as u32]);
        scene
            .written_records
            .push([record_base, record_base + records.len() as u32]);
        self.blocks.insert(
            pos,
            Block {
                version,
                origin,
                vertices: [vertex_base, vertex_base + corners.len() as u32],
                indices: [index_base, index_base + indices.len() as u32],
                records: [record_base, record_base + records.len() as u32],
                quads: one.quads as u32,
                bent: one.bent_quads as u32,
            },
        );
        true
    }

    /// **Drop a chunk's slice**: its ranges become holes the next landing can take, its origin is
    /// taken by the last chunk whose own corners and rectangles are rebound to it, and the arrays
    /// end where their last hole begins rather than keeping a tail of tombstones.
    ///
    /// **Nothing moves.** The slice this chunk owned is tombstoned — a zero extent draws no pixel
    /// and an all-zero index is a triangle with no area — because the draw still reads one dense
    /// range and a hole in the middle of it must be geometry that covers nothing. The one thing
    /// that *is* rewritten is the moved chunk's own elements, and only their `chunk` field: the
    /// origin array is swap-removed, so the chunk standing on the freed slot has to name it, and
    /// that is exactly one chunk's worth of work rather than the window's.
    fn remove(&mut self, pos: voxel_model::ChunkPos) {
        let Some(block) = self.blocks.remove(&pos) else {
            return;
        };
        let scene = self.scene.as_mut().expect("held");
        // **The origin, freed by swapping in the last one.** A chunk's origin index is what its
        // corners and rectangles name, so the chunk that lands on the freed slot has its own
        // elements rebound to it — its ranges are then owed to the GPU again.
        let last_origin = scene.origins.len() as u32 - 1;
        if block.origin != last_origin {
            let moved_pos = self
                .blocks
                .iter()
                .find(|(_, b)| b.origin == last_origin)
                .map(|(p, _)| *p);
            if let Some(moved_pos) = moved_pos {
                let moved = self.blocks[&moved_pos];
                for v in &mut scene.vertices[moved.vertices[0] as usize..moved.vertices[1] as usize]
                {
                    v.chunk = block.origin;
                }
                for r in &mut scene.records[moved.records[0] as usize..moved.records[1] as usize] {
                    r.chunk = block.origin;
                }
                scene.written_vertices.push(moved.vertices);
                scene.written_records.push(moved.records);
                self.blocks
                    .get_mut(&moved_pos)
                    .expect("the last origin's chunk")
                    .origin = block.origin;
            }
            scene.origins[block.origin as usize] = scene.origins[last_origin as usize];
        }
        scene.origins.pop();
        // **The three slices, tombstoned and given back.** A tombstone names origin zero, which is
        // a live origin whenever anything is still resident; a scene with nothing left is cleared
        // whole below, so the two never disagree.
        let corners = block.vertices[0] as usize..block.vertices[1] as usize;
        for v in &mut scene.vertices[corners.clone()] {
            *v = Vertex::blank();
        }
        let indices = block.indices[0] as usize..block.indices[1] as usize;
        for i in &mut scene.indices[indices.clone()] {
            *i = 0;
        }
        let records = block.records[0] as usize..block.records[1] as usize;
        for r in &mut scene.records[records.clone()] {
            *r = QuadRecord::blank();
        }
        if block.vertices[0] != block.vertices[1] {
            scene.written_vertices.push(block.vertices);
        }
        if block.indices[0] != block.indices[1] {
            scene.written_indices.push(block.indices);
        }
        if block.records[0] != block.records[1] {
            scene.written_records.push(block.records);
        }
        release(&mut self.free_vertices, block.vertices);
        release(&mut self.free_indices, block.indices);
        release(&mut self.free_records, block.records);
        // The tail a hole reached is given back, so a column leaving does not leave the arrays
        // holding bytes for a world that is no longer drawn.
        if self.blocks.is_empty() {
            scene.vertices.clear();
            scene.indices.clear();
            scene.records.clear();
            scene.origins.clear();
            self.free_vertices.clear();
            self.free_indices.clear();
            self.free_records.clear();
            return;
        }
        let vertices = trim(
            &mut self.free_vertices,
            scene.vertices.len(),
            live_end(self.blocks.values().map(|b| b.vertices)),
        );
        scene.vertices.truncate(vertices);
        let indices = trim(
            &mut self.free_indices,
            scene.indices.len(),
            live_end(self.blocks.values().map(|b| b.indices)),
        );
        scene.indices.truncate(indices);
        let records = trim(
            &mut self.free_records,
            scene.records.len(),
            live_end(self.blocks.values().map(|b| b.records)),
        );
        scene.records.truncate(records);
    }

    /// **Whether the arrays carry more dead space than [`ARENA_DEAD_IN`] allows** — and if so,
    /// compact them (see [`ResidentScene::compact`]).
    ///
    /// The live lengths are the blocks' own slices, summed; a `u32` range is always ordered by
    /// construction, and the two are equal right after a compaction, so the next one waits until
    /// the churn has refilled a quarter of the array.
    fn compact_if_needed(&mut self) -> bool {
        let live = |ranges: &mut dyn Iterator<Item = [u32; 2]>| -> usize {
            ranges.map(|r| (r[1] - r[0]) as usize).sum()
        };
        let vertices = live(&mut self.blocks.values().map(|block| block.vertices));
        let indices = live(&mut self.blocks.values().map(|block| block.indices));
        let records = live(&mut self.blocks.values().map(|block| block.records));
        let scene = self.scene.as_ref().expect("held");
        let over =
            |len: usize, live: usize| len.saturating_sub(live) > live / ARENA_DEAD_IN + ARENA_FLOOR;
        if !(over(scene.vertices.len(), vertices)
            || over(scene.indices.len(), indices)
            || over(scene.records.len(), records))
        {
            return false;
        }
        self.compact();
        self.compactions += 1;
        true
    }

    /// **Rewrite the three arrays as the blocks' slices, end to end in origin order.**
    ///
    /// Every offset moves, so every block's ranges are reassigned, every corner and rectangle is
    /// rebound to its chunk's new origin, and every index is shifted by its chunk's own vertex
    /// base (an index names a corner inside its own chunk — [`ResidentScene::check`] is the
    /// invariant). The free lists are cleared: there is no hole left to remember. The whole array
    /// is owed to the GPU, so the written ranges are the arrays themselves.
    fn compact(&mut self) {
        let scene = self.scene.as_mut().expect("held");
        let old_vertices = std::mem::take(&mut scene.vertices);
        let old_indices = std::mem::take(&mut scene.indices);
        let old_records = std::mem::take(&mut scene.records);
        let old_origins = std::mem::take(&mut scene.origins);
        let mut blocks: Vec<(voxel_model::ChunkPos, Block)> = self
            .blocks
            .iter()
            .map(|(pos, block)| (*pos, *block))
            .collect();
        blocks.sort_unstable_by_key(|(_, block)| block.origin);
        let mut vertices = Vec::with_capacity(old_vertices.len());
        let mut indices = Vec::with_capacity(old_indices.len());
        let mut records = Vec::with_capacity(old_records.len());
        let mut origins = Vec::with_capacity(old_origins.len());
        for (origin, (_, block)) in blocks.iter_mut().enumerate() {
            let origin_index = origin as u32;
            let vertex_base = vertices.len() as u32;
            vertices.extend(
                old_vertices[block.vertices[0] as usize..block.vertices[1] as usize]
                    .iter()
                    .map(|v| Vertex {
                        chunk: origin_index,
                        ..*v
                    }),
            );
            let index_base = indices.len() as u32;
            indices.extend(
                old_indices[block.indices[0] as usize..block.indices[1] as usize]
                    .iter()
                    .map(|i| i - block.vertices[0] + vertex_base),
            );
            let record_base = records.len() as u32;
            records.extend(
                old_records[block.records[0] as usize..block.records[1] as usize]
                    .iter()
                    .map(|r| QuadRecord {
                        chunk: origin_index,
                        ..*r
                    }),
            );
            origins.push(old_origins[block.origin as usize]);
            block.origin = origin_index;
            block.vertices = [
                vertex_base,
                vertex_base + block.vertices[1] - block.vertices[0],
            ];
            block.indices = [index_base, index_base + block.indices[1] - block.indices[0]];
            block.records = [
                record_base,
                record_base + block.records[1] - block.records[0],
            ];
        }
        self.blocks = blocks.into_iter().collect();
        scene.vertices = vertices;
        scene.indices = indices;
        scene.records = records;
        scene.origins = origins;
        scene.written_vertices.clear();
        scene.written_indices.clear();
        scene.written_records.clear();
        if !scene.vertices.is_empty() {
            scene
                .written_vertices
                .push([0, scene.vertices.len() as u32]);
        }
        if !scene.indices.is_empty() {
            scene.written_indices.push([0, scene.indices.len() as u32]);
        }
        if !scene.records.is_empty() {
            scene.written_records.push([0, scene.records.len() as u32]);
        }
        scene.build = Scene::next_build();
        self.free_vertices.clear();
        self.free_indices.clear();
        self.free_records.clear();
    }

    /// Re-walk one chunk for the current light and write its slice in place.
    #[allow(clippy::too_many_arguments)]
    fn reemit(
        &mut self,
        pos: voxel_model::ChunkPos,
        mesh: &Mesh,
        light: Option<&LightAt<'_>>,
        displacement: Option<&voxel_model::DisplacementField>,
        refine: Option<&voxel_mesh::RefineConfig>,
    ) {
        let Some(block) = self.blocks.get(&pos).copied() else {
            return;
        };
        let one = one_chunk(mesh, light, displacement, refine, self.shadow_lattice);
        let same = one.vertices.len() as u32 == block.vertices[1] - block.vertices[0]
            && one.indices.len() as u32 == block.indices[1] - block.indices[0]
            && one.records.len() as u32 == block.records[1] - block.records[0];
        if !same {
            // The geometry moved as well (an edit that reshaped the chunk): replace the slice.
            self.remove(pos);
            self.add(pos, block.version, mesh, light, displacement, refine);
            return;
        }
        let scene = self.scene.as_mut().expect("held");
        for (i, v) in one.vertices.iter().enumerate() {
            let mut v = v.clone();
            v.chunk = block.origin;
            scene.vertices[block.vertices[0] as usize + i] = v;
        }
        for (i, index) in one.indices.iter().enumerate() {
            scene.indices[block.indices[0] as usize + i] = index + block.vertices[0];
        }
        for (i, r) in one.records.iter().enumerate() {
            let mut r = *r;
            r.chunk = block.origin;
            scene.records[block.records[0] as usize + i] = r;
        }
        if block.vertices[0] != block.vertices[1] {
            scene.written_vertices.push(block.vertices);
        }
        if block.indices[0] != block.indices[1] {
            scene.written_indices.push(block.indices);
        }
        if block.records[0] != block.records[1] {
            scene.written_records.push(block.records);
        }
    }
}

/// One mesh, through the whole-scene builder: the chunk's own slice of the format, with its
/// lighting and its refinement decided by the same code every other caller uses.
fn one_chunk(
    mesh: &Mesh,
    light: Option<&LightAt<'_>>,
    displacement: Option<&voxel_model::DisplacementField>,
    refine: Option<&voxel_mesh::RefineConfig>,
    shadow_lattice: u32,
) -> Scene {
    let mut build = SceneBuild::with_shadow_lattice(shadow_lattice);
    let meshes = [mesh];
    while !build.step(&meshes, light, displacement, refine, usize::MAX) {}
    build.finish()
}

/// Squared distance between two chunk positions, for the landing order.
fn distance_squared(a: voxel_model::ChunkPos, b: voxel_model::ChunkPos) -> i64 {
    let (dx, dy, dz) = (
        a.x as i64 - b.x as i64,
        a.y as i64 - b.y as i64,
        a.z as i64 - b.z as i64,
    );
    dx * dx + dy * dy + dz * dz
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

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_mesh::{Mesh, MeshConfig, Mesher, Quad, RefineConfig};
    use voxel_model::{DisplacementField, Face, MaterialId, Profile};

    /// One block's own visible face, as the merge emits it: a single rectangle four
    /// subvoxels on a side, standing on plane `slice = 4`.
    fn one_block_face(face: Face, material: MaterialId) -> Mesh {
        Mesh {
            chunk: voxel_model::ChunkPos::new(0, 0, 0),
            origin: [0, 0, 0],
            quads: vec![Quad {
                face,
                slice: 4,
                uv: [0, 0],
                extent: [4, 4],
                material,
            }],
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

        let scene = Scene::build([&roof, &wall], [0.0; 3], Some(&field), Some(&config));
        assert_eq!(
            scene.bent_quads, 1,
            "the law reached a quad it does not touch"
        );
        assert_eq!(
            scene.records.len(),
            1,
            "a bent quad was left in the record format"
        );
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
            assert!(
                (y - 1.0).abs() < 1e-4,
                "the corner at ({x}, {z}) moved to {y}"
            );
        }
        let centre = top(0.5, 0.5);
        assert!(
            (centre - (1.0 + amplitude_m)).abs() < 1e-3,
            "the cap stands {centre} m against the declared {amplitude_m} m above the plane"
        );
        let inside = scene.vertices.iter().all(|v| {
            (-1e-4..=1.0 + 1e-4).contains(&v.position[0])
                && (-1e-4..=1.0 + 1e-4).contains(&v.position[2])
        });
        assert!(inside, "the refinement left its own rectangle in plane");
    }

    #[test]
    fn a_refined_corner_states_the_face_it_was_cut_from() {
        /// **The corner path's plane axis** (C-163.2). A refined triangle carries no record, so
        /// the face has to travel with the vertex: without it `vs_main` could only fall back to
        /// the material's **world** axis, and a directional shaper then drew square cells on the
        /// face that looks along that axis and stretched ones on every other side. The value is
        /// the quad's own `Face::axis`, on every corner of every triangle it was refined into.
        const ROCK: MaterialId = MaterialId(6);
        let config = RefineConfig::new(4, 4096).expect("4 samples per block is inside the level");
        for (face, axis) in [(Face::PosY, 1), (Face::PosZ, 2), (Face::NegX, 0)] {
            let field = a_pillar(ROCK, face);
            let quad = one_block_face(face, ROCK);
            let scene = Scene::build([&quad], [0.0; 3], Some(&field), Some(&config));
            assert_eq!(scene.bent_quads, 1, "the law did not reach {face:?}");
            assert!(!scene.vertices.is_empty(), "a bent quad emitted no corners");
            assert!(
                scene.vertices.iter().all(|v| v.plane_axis == axis),
                "a corner of {face:?} did not state axis {axis}: {:?}",
                scene
                    .vertices
                    .iter()
                    .map(|v| v.plane_axis)
                    .collect::<Vec<_>>()
            );
        }
        // Geometry that is not cut from a block's face states none, and `-1` is what the
        // shader's fallback is keyed on.
        let probe = Scene::probe([0.0, 0.0, 1.0], ROCK.0 as u32, 2.0, 1.0, [0.0; 3]);
        assert!(
            probe.vertices.iter().all(|v| v.plane_axis == -1),
            "a screen-space probe claimed a face"
        );
    }

    #[test]
    fn one_unmeasured_corner_rigs_the_whole_quad_so_no_diagonal_splits_it() {
        /// The flat-flag rule, pinned: `light_present` reaches the fragment
        /// flat (one corner's value per triangle), so a quad whose corners
        /// disagree would shade its two triangles on different branches. ANDing
        /// keeps both triangles together; bands stay per-corner, so no detail
        /// is flattened by it.
        const ROCK: MaterialId = MaterialId(6);
        let roof = one_block_face(Face::PosY, ROCK);
        // Deny exactly one corner's cell: the quad's own first corner.
        let first = roof.quads[0].corners()[0];
        let denied = {
            let scale = 1.0 / voxel_model::SUBVOXEL_SIDE as f64;
            let w = [
                first[0] as f64 * scale,
                first[1] as f64 * scale,
                first[2] as f64 * scale,
            ];
            crate::field::SubvoxelPos::of_point(w).expect("a finite corner")
        };
        // **One corner's own cell**, a quarter-voxel rather than the block's metre: the address is
        // the corner's, so the three corners beside it stay measured and the quad is still rigged.
        let light = move |point: [f64; 3]| {
            if crate::field::SubvoxelPos::of_point(point) == Some(denied) {
                None
            } else {
                Some([0.5f32, 0.5, 0.5, 0.5, 0.5, 0.5])
            }
        };
        let scene = Scene::build_with_light([&roof], [0.0; 3], &light, None, None);
        assert_eq!(scene.vertices.len(), 4, "one quad is four corners");
        for v in &scene.vertices {
            assert_eq!(
                v.light_present, 0,
                "a corner was measured but its quad was rigged: {:?}",
                v.light_direct
            );
        }
        // And the measured corners kept their own bands (only the flag is
        // shared — the light itself is still per-corner).
        assert!(
            scene.vertices.iter().any(|v| v.light_direct != [0.0; 3]),
            "sharing the flag flattened the bands too"
        );

        let full = |_: [f64; 3]| Some([0.5f32; 6]);
        let lit = Scene::build_with_light([&roof], [0.0; 3], &full, None, None);
        assert!(
            lit.vertices.iter().all(|v| v.light_present == 1),
            "a fully measured quad must stay field-lit"
        );
    }

    /// **A face asks the shadow at the subvoxel lattice, not once per corner.**
    ///
    /// The shadow this test draws is the one a once-per-corner face cannot: the light changes at
    /// **0.5 m**, in the middle of the face, where the old path had no vertex at all — its four
    /// corners sat a metre apart and the fragment blended between them, so the edge was a metre
    /// wide whatever the field said. At `SHADOW_LATTICE` the face is cut into 4×4 cells, its
    /// samples are 0.25 m apart (the address the field actually stores), and the edge falls
    /// between two of them.
    ///
    /// **The expectation that moved:** this asserted `4 × 4² = 64` vertices, one four-corner
    /// cell at a time. The grid's corners are now shared — an interior sample is one vertex, not
    /// four cells' copies — so the same face, the same samples and the same picture cost
    /// `(4 + 1)² = 25`. The superseded number was paying four times for the same geometry, and the
    /// shipped window paid it on every lit face (a 32 m face cost 65 536 vertices for a grid of
    /// 16 641) until it outgrew the device's largest buffer.
    ///
    /// `SHADOW_LATTICE` is a fact about the stored data rather than a taste: a finer cut would ask
    /// the same cell twice and could only change the interpolation between answers.
    #[test]
    fn a_face_asks_the_shadow_at_the_subvoxel_lattice_not_once_per_corner() {
        const ROCK: MaterialId = MaterialId(6);
        let roof = one_block_face(Face::PosY, ROCK);
        // Lit on the low half of the face's x axis, dark on the high half: the edge is at 0.5 m.
        let half = |point: [f64; 3]| {
            let light = if point[0] < 0.5 { 1.0 } else { 0.0 };
            Some([light, light, light, 0.0, 0.0, 0.0])
        };
        let coarse = Scene::build_with_light([&roof], [0.0; 3], &half, None, None);
        assert_eq!(coarse.vertices.len(), 4, "the once-per-corner path moved");
        let fine =
            Scene::build_with_light_lattice([&roof], [0.0; 3], &half, None, None, SHADOW_LATTICE);
        let side = SHADOW_LATTICE as usize;
        assert_eq!(
            fine.vertices.len(),
            (side + 1) * (side + 1),
            "a one-voxel face is not one shared SHADOW_LATTICE² grid"
        );
        // The grid's own triangles: two a cell, six indices each, and every index inside the
        // face's corners — sharing moves the vertices, never the picture.
        assert_eq!(
            fine.indices.len(),
            6 * side * side,
            "the shared grid's triangles"
        );
        assert!(
            fine.indices
                .iter()
                .all(|i| (*i as usize) < fine.vertices.len()),
            "an index points past the grid"
        );
        // The samples' own spacing, which is the whole claim: a quarter of a metre, at the
        // address `SubvoxelPos` stores, with no sample in between.
        let mut xs: Vec<f32> = fine.vertices.iter().map(|v| v.position[0]).collect();
        xs.sort_by(f32::total_cmp);
        xs.dedup();
        assert_eq!(
            xs,
            vec![0.0, 0.25, 0.5, 0.75, 1.0],
            "the face's sample grid"
        );
        // And the edge is *in* the grid rather than blended across it: the corners either side of
        // 0.5 m disagree, and the disagreement happens inside the face.
        let at = |x: f32, scene: &Scene| -> f32 {
            let v = scene
                .vertices
                .iter()
                .find(|v| (v.position[0] - x).abs() < 1e-6)
                .expect("a sample at that x");
            v.light_direct[0]
        };
        assert_eq!(at(0.25, &fine), 1.0, "the lit side of the edge");
        assert_eq!(at(0.5, &fine), 0.0, "the edge itself is on the dark side");
        assert_eq!(at(0.75, &fine), 0.0, "the dark side of the edge");
        // **What the coarse path could not produce**: its only differing samples are a metre
        // apart, so the visible edge is the fragment's blend across the whole face.
        let mut coarse_xs: Vec<f32> = coarse.vertices.iter().map(|v| v.position[0]).collect();
        coarse_xs.sort_by(f32::total_cmp);
        coarse_xs.dedup();
        assert_eq!(
            coarse_xs,
            vec![0.0, 1.0],
            "the old path is the corners and nothing else"
        );
    }

    /// **A face the light does not cross keeps its four corners.** The lattice is sampled to
    /// find out, and when every sample agrees the whole rectangle is that one answer: the grid
    /// would be copies of it, which is what the shipped window was paying for on every lit face
    /// — 8.9 M corners for a 605-chunk window until the device refused the buffer.
    #[test]
    fn a_uniformly_lit_face_spends_four_corners_and_not_a_grid() {
        const ROCK: MaterialId = MaterialId(6);
        let roof = one_block_face(Face::PosY, ROCK);
        let even = |_: [f64; 3]| Some([0.5f32; 6]);
        let scene =
            Scene::build_with_light_lattice([&roof], [0.0; 3], &even, None, None, SHADOW_LATTICE);
        assert_eq!(
            scene.vertices.len(),
            4,
            "a face whose samples all agree paid for a grid"
        );
        assert_eq!(scene.indices.len(), 6, "one rectangle is two triangles");
        assert!(
            scene
                .vertices
                .iter()
                .all(|v| v.light_present == 1 && v.light_direct == [0.5; 3]),
            "the face's one answer did not reach every corner"
        );
    }

    /// **An unmeasured corner is not a shadow edge.** The grid is the `quad_present` rule one
    /// step finer: a face whose corners do not all have an answer is the rig's, and cutting it
    /// into a lattice measures samples the frame will not draw. This is the frontier's size, not
    /// the window's: the field answers a stale cell, so most faces in the settled window are
    /// *known*, and this rule moved the live count only from 2.07 M to 2.05 M. It is still the
    /// rule a face at the edge of measurement deserves.
    #[test]
    fn a_face_with_an_unmeasured_corner_keeps_four_corners_and_the_rig() {
        const ROCK: MaterialId = MaterialId(6);
        let roof = one_block_face(Face::PosY, ROCK);
        // Measured on the low half of the face's x axis, unmeasured on the high half: the four
        // corners disagree about coverage, and the interior would disagree about light.
        let half_measured = |point: [f64; 3]| {
            if point[0] < 0.5 {
                Some([1.0f32; 6])
            } else {
                None
            }
        };
        let scene = Scene::build_with_light_lattice(
            [&roof],
            [0.0; 3],
            &half_measured,
            None,
            None,
            SHADOW_LATTICE,
        );
        assert_eq!(
            scene.vertices.len(),
            4,
            "a face with an unmeasured corner paid for a lattice it cannot draw"
        );
        assert_eq!(scene.indices.len(), 6, "one rectangle is two triangles");
        assert!(
            scene.vertices.iter().all(|v| v.light_present == 0),
            "a face with an unmeasured corner claimed the field's light"
        );
    }

    /// **The lattice cannot be finer than the data** — values are clamped to a divisor of
    /// `SUBVOXEL_SIDE`, so no caller can ask for a sample on a cell boundary and get a coin toss.
    #[test]
    fn the_shadow_lattice_is_clamped_to_a_divisor_of_the_subvoxel_side() {
        assert_eq!(SceneBuild::with_shadow_lattice(0).shadow_lattice, 1);
        assert_eq!(SceneBuild::with_shadow_lattice(1).shadow_lattice, 1);
        assert_eq!(SceneBuild::with_shadow_lattice(2).shadow_lattice, 2);
        assert_eq!(
            SceneBuild::with_shadow_lattice(SHADOW_LATTICE).shadow_lattice,
            SHADOW_LATTICE
        );
        // Anything past the finest stored cell is the finest stored cell; three is not a divisor
        // and is answered by the largest one below it.
        assert_eq!(SceneBuild::with_shadow_lattice(3).shadow_lattice, 2);
        assert_eq!(
            SceneBuild::with_shadow_lattice(9).shadow_lattice,
            SHADOW_LATTICE
        );
    }

    #[test]
    fn lit_scenes_carry_no_records_so_the_wireframe_view_is_empty() {
        /// The F4 mechanism, pinned: `draw_wireframe` draws `scene.records` and
        /// nothing else, but a lit quad leaves the record format for corners —
        /// so a game with the light field on draws terrain the wireframe view
        /// cannot see.
        const ROCK: MaterialId = MaterialId(6);
        let roof = one_block_face(Face::PosY, ROCK);
        let light = |_: [f64; 3]| Some([1.0f32; 6]);
        let lit = Scene::build_with_light([&roof], [0.0; 3], &light, None, None);
        assert_eq!(lit.records.len(), 0, "a lit quad left the record format");
        assert!(!lit.vertices.is_empty(), "a lit quad built no corners");
        let plain = Scene::build([&roof], [0.0; 3], None, None);
        assert_eq!(plain.records.len(), 1, "an unlit quad is one rectangle");
        assert!(plain.vertices.is_empty(), "an unlit quad built corners");
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
        let scene = Scene::build([&roof], [0.0; 3], Some(&field), Some(&refused));
        assert!(
            !scene.is_empty(),
            "a refused refinement made the world disappear"
        );
        assert_eq!(
            scene.bent_quads, 0,
            "a refused quad claimed the vertex format"
        );
        assert_eq!(
            scene.records.len(),
            1,
            "the planar fallback is not one rectangle"
        );
        assert!(
            scene.indices.is_empty(),
            "a refused quad spent vertices anyway"
        );
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
        let wired = Scene::build([&roof, &wall], [0.0; 3], Some(&field), Some(&config));
        let unwired = Scene::build_with_detail([&roof, &wall], [0.0; 3]);
        assert_eq!(
            wired.records.len(),
            2,
            "the empty field moved a quad out of the record"
        );
        assert!(wired.indices.is_empty());
        assert_eq!(wired.bent_quads, 0);
        assert_eq!(
            wired.records, unwired.records,
            "wiring the stage changed the frame"
        );
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
            let built = Scene::build([&many], [0.0; 3], Some(&field), Some(&config));
            std::hint::black_box(built.records.len());
        };
        let unwired = || {
            let built = Scene::build_with_detail([&many], [0.0; 3]);
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
    fn the_cloud_has_its_own_records_and_origins_and_they_carry_the_veil() {
        // **The wiring's two claims, as facts** (C-179). A cloud's records come **last**, so the
        // draw can split one buffer into an opaque world and a blended veil without a second
        // array; and a cloud record addresses **its own** origin, because a cell's veil rides in
        // the origin's `w` and a chunk origin is shared by every quad standing on that chunk.
        // The world's own veil is zero, which is what that float was before clouds existed — so
        // adding clouds cannot change how a chunk shades.
        use voxel_mesh::{Mesh, Quad};
        use voxel_model::Face;
        let mesh = Mesh {
            chunk: voxel_model::ChunkPos::new(0, 0, 0),
            origin: [0, 0, 0],
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
        let terrain = Scene::build_with_detail([&mesh], [0.0; 3]);
        assert_eq!(terrain.records.len(), 1, "the fixture is not one rectangle");
        assert_eq!(terrain.clouds.len(), 0, "terrain declared itself a cloud");
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
                shape_lo: 0,
                shape_hi: 0,
                shape_twist: 0,
            }],
            origins: vec![[10.0, 500.0, -20.0, 0.72]],
            cell_m: 4.0,
            subdivisions: 1,
            cells: 1,
            rows_max: 1,
            ..Default::default()
        };
        let scene = terrain.with_clouds(&cubes);
        assert_eq!(
            scene.records.len(),
            1,
            "the cloud is in the terrain's array"
        );
        assert_eq!(scene.clouds.len(), 1, "the cloud's own array is empty");
        assert_eq!(
            scene.clouds[0].chunk, 0,
            "the cloud's record does not name its own origin"
        );
        assert_eq!(
            scene.origins.len(),
            1,
            "the cloud's origin is in the terrain's slots"
        );
        assert_eq!(
            scene.origins[0],
            [0.0, 0.0, 0.0, 0.0],
            "a chunk carries a veil"
        );
        assert_eq!(
            scene.cloud_origins.len(),
            1,
            "the cloud has no origin of its own"
        );
        // **The cell's veil, in the encoding the shader reads back**: `1 + opacity`, positive for
        // the newer of the two trees a fade may join (`cloud_cubes::veil_with_side`), so
        // `abs(w) - 1` is the opacity and `w != 0` is "this is a cloud".
        assert!(
            (scene.cloud_origins[0][3] - 1.28).abs() < 1e-12,
            "{}",
            scene.cloud_origins[0][3]
        );
    }

    #[test]
    fn the_two_trees_of_a_fade_carry_their_side_in_the_sign_of_their_veils() {
        use voxel_mesh::{Mesh, Quad};
        use voxel_model::Face;
        let mesh = Mesh {
            chunk: voxel_model::ChunkPos::new(0, 0, 0),
            origin: [0, 0, 0],
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
        let terrain = Scene::build_with_detail([&mesh], [0.0; 3]);
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
                shape_lo: 0,
                shape_hi: 0,
                shape_twist: 0,
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
        assert_eq!(
            scene.records.len(),
            1,
            "the join is not the world's own rectangle"
        );
        assert_eq!(scene.clouds.len(), 2, "the join is not both trees");
        assert_eq!(
            scene.clouds[1].chunk, 1,
            "the newer tree did not move to its own origins"
        );
        // **Both are the tree's own transmittance, and only the sign differs.** A veil of 0.6 is
        // an opacity of 0.4, so both carry `1.4`: the older negated, the newer positive. Nothing in
        // the scene is a *weight* — the shader reads those from one uniform
        // (`WorldPipeline::set_cloud_fade`), so these bytes are the same for every frame of the
        // dissolve and a fade costs no rewriting of the sky at all.
        assert!(
            (scene.cloud_origins[0][3] + 1.4).abs() < 1e-12,
            "{}",
            scene.cloud_origins[0][3]
        );
        assert!(
            (scene.cloud_origins[1][3] - 1.4).abs() < 1e-12,
            "{}",
            scene.cloud_origins[1][3]
        );

        // **The same scene serves the whole dissolve.** There is no weight to pass, so the pair a
        // frame draws at the start of a cadence and at the end of it is literally one value — and
        // a scene built from the pair is the same one the frame before it held.
        let again = terrain.clone().with_cloud_fade(&older, &newer);
        assert_eq!(
            scene, again,
            "the pair's own bytes depend on something but the trees"
        );
        assert_ne!(
            scene,
            terrain.clone().with_clouds(&newer),
            "a lone tree is not a pair"
        );

        // One tree alone carries the newer side, so a frame with nothing in flight draws its own
        // number whole: the shader's weight is `1.0` there and the decode is the identity.
        let entering = terrain.with_clouds(&newer);
        assert_eq!(entering.clouds.len(), 1);
        assert!(
            (entering.cloud_origins[0][3] - 1.4).abs() < 1e-12,
            "{}",
            entering.cloud_origins[0][3]
        );
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
                    shape_lo: 0,
                    shape_hi: 0,
                    shape_twist: 0,
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
        let bare = Scene::build_with_detail(Vec::new().iter(), [0.0; 3]);

        // Four rounds alternating, the minimum of each: a pair's first pass pays for a cold heap.
        let (mut single, mut fade) = (f64::INFINITY, f64::INFINITY);
        for _ in 0..4 {
            let started = std::time::Instant::now();
            let scene = bare.clone().with_clouds(&newer);
            assert_eq!(scene.clouds.len(), count);
            single = single.min(started.elapsed().as_secs_f64());

            let started = std::time::Instant::now();
            let scene = bare.clone().with_cloud_fade(&older, &newer);
            assert_eq!(scene.clouds.len(), 2 * count, "the fade drew one tree");
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
        // **The claim is the bytes, not the clock.** A cross-fade that re-meshed the second tree
        // would hand back records that are not that tree's own; one that appends it hands back the
        // two trees' records, the second rebased onto the first tree's origins. The timing above is
        // printed rather than asserted: under the whole suite an 80 000-record allocation can land
        // on fresh pages and cost four times what it does alone, which is a fact about the
        // allocator and not about this path.
        let scene = bare.with_cloud_fade(&older, &newer);
        for (i, record) in scene.clouds[count..].iter().enumerate() {
            let source = &newer.records[i];
            assert_eq!(
                (
                    record.face,
                    record.material,
                    record.slice,
                    record.uv,
                    record.extent
                ),
                (
                    source.face,
                    source.material,
                    source.slice,
                    source.uv,
                    source.extent
                ),
                "the second tree's record {i} is not the tree's own: the fade re-meshed it"
            );
            assert_eq!(
                record.chunk,
                source.chunk + count as u32,
                "the second tree's record {i} is not rebased onto the first tree's origins"
            );
        }
        // **And the frame pays that once, not once per frame.** The pair is built when a tree is
        // adopted; every frame of the dissolve after it draws the same bytes with one uniform
        // changed, so what the dissolve costs the frame thread is nothing at all — the interpolation
        // is `shading.wgsl`'s, per pixel.
        assert_eq!(
            scene.clouds[count].chunk, count as u32,
            "the newer tree is not last"
        );
        assert!(
            scene.cloud_origins[0][3] < 0.0 && scene.cloud_origins[count][3] > 0.0,
            "the two trees' sides are not in the signs: {} against {}",
            scene.cloud_origins[0][3],
            scene.cloud_origins[count][3]
        );
        assert!(
            (scene.cloud_origins[0][3] + scene.cloud_origins[count][3]).abs() < 1e-12,
            "the two sides are not the same sky: {} against {}",
            scene.cloud_origins[0][3],
            scene.cloud_origins[count][3]
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
        assert_eq!(
            scene.origins,
            vec![moon.origin()],
            "the body has no origin of its own"
        );
        assert_eq!(scene.records.len(), 0, "a body is not a rectangle's record");
        assert_eq!(scene.clouds.len(), 0, "a body declared itself a cloud");
        assert!(scene.bodies[0].is_body());
        assert!(
            !scene.is_empty(),
            "a scene with the Moon in it read as empty"
        );
        assert_eq!(
            scene.bodies[0].chunk, 0,
            "the record does not name its own origin"
        );
    }

    /// **The cache against a world that is actually changing.** The key is the whole of the
    /// contract — a missed bump draws a world that no longer exists — so this drives a real
    /// streamer through the mutations the game allows (a chunk's arrival, a dig, an eviction by
    /// walking out of the window, the atmosphere's own tick) and asks, on every frame, whether the
    /// scene it drew is the world the streamer holds.
    #[test]
    fn the_cached_scene_is_the_resident_world_on_every_frame() {
        use voxel_shapers::{Config, WorldSpec};
        use voxel_stream::WorldSource;

        let spec = WorldSpec::new(2468, Config::default());
        let registry = voxel_shapers::materials::registry().expect("the registry");
        let mesh_config = voxel_mesh::MeshConfig::from_registry(&registry).expect("a mesh config");
        let mut streamer =
            voxel_work::Streamer::new(WorldSource::new(spec), mesh_config, 2, 64 * 1024 * 1024)
                .and_then(|s| {
                    s.with_window(voxel_work::ChunkWindow {
                        radius_chunks: 1,
                        vertical_chunks: 0,
                    })
                })
                .expect("a streamer");
        let surface = voxel_model::ChunkPos::new(
            0,
            voxel_shapers::terrain::elevation_m(&spec, 16.5, 16.5) as i32 / 32,
            0,
        );
        streamer.set_centre(surface);
        streamer.settle().expect("the window settles");

        let mut cache = SceneCache::default();
        let mut light = 0u64;
        let mut builds = 0u32;
        let mut last_build = 0u64;
        let mut frames = 0u32;
        for frame in 0..24u32 {
            // The mutations, one kind at a time and in an order a player produces them.
            match frame {
                // **The drawn world emptied and refilled** — `note_world_changed` is what a
                // replaced generator calls, and it moves `residency_generation` while the pool's
                // record generation does *not* move. This is the case a key on the record alone
                // cannot see, and it is here so the test can fail for it: a cache that kept the
                // old scene would draw 4 144 quads of a world the streamer no longer holds.
                2 => {
                    // **A world change keeps the picture and, for that reason, the key.** The
                    // meshes are demoted to the stale set rather than dropped
                    // ([`voxel_work::Streamer::note_world_changed`] says why, with the client's
                    // own reading of a world that vanished for 428 frames), so the frame after a
                    // change still draws the world the frame before it drew — and since the bytes
                    // did not change, neither does the key: the replacement moves a mesh's version
                    // one chunk at a time, and *that* is what the scene diffs. Clearing here drew
                    // nothing until the rebuild reached every chunk, which is the defect the
                    // demotion fixed; bumping the key here would rebuild the same bytes.
                    let before = streamer.residency_generation();
                    let drawn_before: usize = streamer.meshes().map(|m| m.quads.len()).sum();
                    assert!(drawn_before > 0, "the world was empty before the change");
                    streamer.note_world_changed();
                    let drawn: usize = streamer.meshes().map(|m| m.quads.len()).sum();
                    assert_eq!(
                        drawn, drawn_before,
                        "a world change emptied the drawn world"
                    );
                    assert_eq!(
                        streamer.residency_generation(),
                        before,
                        "a world change moved the key for bytes that did not change"
                    );
                }
                // The refill: chunks arriving one pump at a time.
                3 | 5 | 7 | 8 | 9 | 11 => drop(streamer.pump().expect("a pump")),
                // A dig in the middle of the window: the record moves, the chunk is demoted to
                // stale, and the replacement arrives some frames later.
                10 => {
                    let (x, z) = (16, 16);
                    let y = (surface.y * 32..surface.y * 32 + 32)
                        .rev()
                        .find(|y| !streamer.material_at(x, *y, z).unwrap().is_empty())
                        .expect("a solid cell to dig");
                    streamer
                        .edit(voxel_work::pool::Edit {
                            x,
                            y,
                            z,
                            to: voxel_shapers::materials::AIR,
                            reason: "the cache test dug here".into(),
                            actor: voxel_model::world::ActorId::player(),
                            verb: voxel_model::world::PhysicalVerb::Dig,
                        })
                        .expect("the edit reaches a resident chunk");
                }
                // The atmosphere's own tick, which is the light's revision.
                12 | 15 | 18 | 21 => light += 1,
                // **And the window itself moving**: a step east evicts the west column and asks
                // for a new one, so the drawn set shrinks before it grows — the case a key on
                // arrivals alone would keep drawing a chunk the streamer had already released.
                6 => streamer.set_centre(voxel_model::ChunkPos::new(
                    surface.x + 1,
                    surface.y,
                    surface.z,
                )),
                _ => {}
            }
            drop(streamer.pump().expect("a pump"));

            let key = SceneKey {
                world: streamer.world_generation(),
                residency: streamer.residency_generation(),
                light,
            };
            // **The light answers with its own revision in the first band**, so a scene built
            // under an older light is visible in the bytes and not only in a build count.
            let bands = move |_: [f64; 3]| Some([light as f32, 0.0, 0.0, 0.0, 0.0, 0.0]);
            let scene = cache.take(key, || {
                (
                    Scene::build_with_light(streamer.meshes(), [0.0; 3], &bands, None, None),
                    true,
                )
            });
            frames += 1;
            if scene.build != last_build {
                builds += 1;
                last_build = scene.build;
            }
            // **The claim**: the scene's quads are the resident set's own, on this frame, after
            // every one of those mutations. A missed bump fails here rather than in a picture.
            let resident: usize = streamer.meshes().map(|m| m.quads.len()).sum();
            let corners: usize = streamer.meshes().map(|m| m.quads.len() * 4).sum();
            assert_eq!(
                scene.quads, resident,
                "frame {frame}: the scene drew {} quads for a residency of {resident}",
                scene.quads
            );
            assert_eq!(
                scene.vertices.len(),
                corners,
                "frame {frame}: a corner is missing"
            );
            // **And the light is the frame's own**, not the one the scene was kept under.
            assert!(
                scene
                    .vertices
                    .iter()
                    .all(|v| v.light_present == 1 && v.light_direct[0] == light as f32),
                "frame {frame}: a corner carries another revision's light"
            );
            // And the frame's own suffix, stripped on the next take the way `main.rs` strips it.
            let mut scene = scene.with_bodies(&[]);
            scene.clouds.push(QuadRecord::of(
                &one_block_face(Face::PosY, MaterialId(0)).quads[0],
                0,
            ));
            scene.cloud_origins.push([0.0, 0.0, 0.0, 0.9]);
            cache.put(scene);
            let scene = cache.take(key, || unreachable!("a kept frame rebuilt"));
            assert_eq!(
                scene.records.len(),
                scene.terrain[0],
                "frame {frame}: a record survived into the next frame"
            );
            assert_eq!(
                scene.clouds.len(),
                0,
                "frame {frame}: the cloud survived the take"
            );
            cache.put(scene);
        }
        assert!(frames >= 24);
        // The world moved, so the cache rebuilt — but not once a frame: a still world between
        // mutations is a kept scene.
        assert!(
            builds < frames,
            "every one of {frames} frames rebuilt: the key is not holding"
        );
        assert!(
            builds >= 4,
            "the mutations did not move the key at all ({builds} builds)"
        );
    }

    /// **The atmosphere cadence does not rebuild a scene the field cannot change.**
    ///
    /// A field that holds no measurement leaves every corner on the rig, and the rig is a uniform —
    /// so the cadence's tick is not a corner's function, and a key that moved with it walked every
    /// resident mesh to produce the same bytes four times a second. This is the pin: four cadences,
    /// one build. Putting the rig's tick back into the key unconditionally makes this four.
    #[test]
    fn a_field_that_holds_nothing_does_not_rebuild_on_the_atmosphere_cadence() {
        let mesh = one_block_face(Face::PosY, MaterialId(0));
        let no_light = |_: [f64; 3]| None;
        let mut cache = SceneCache::default();
        let mut builds = 0;
        for tick in [4u64, 8, 12, 16] {
            let key = SceneKey {
                world: 1,
                residency: 1,
                light: SceneKey::light_of(0, tick),
            };
            let scene = cache.take(key, || {
                builds += 1;
                (
                    Scene::build_with_light(
                        std::iter::once(&mesh),
                        [0.0; 3],
                        &no_light,
                        None,
                        None,
                    ),
                    true,
                )
            });
            cache.put(scene);
        }
        assert_eq!(
            builds, 1,
            "the atmosphere cadence rebuilt a scene it cannot change"
        );
        // **A field with a measurement does move the key**: those bands are the corner's, not the
        // uniform's, so a held cell's light is geometry and the revision that measured it is part
        // of it — and a tick that measured nothing is not.
        assert_eq!(SceneKey::light_of(1, 9_000), 9_000);
        assert_ne!(SceneKey::light_of(1, 9_000), SceneKey::light_of(1, 9_001));
    }

    /// **And the bytes a cadence would have rebuilt are the bytes in hand.**
    ///
    /// The rule above is only sound if a skipped rebuild would have produced what is already there.
    /// This builds the same face at two atmosphere ticks through a real `CellField` that measures
    /// **nothing** — no chunk is resident on a provider that answers `None` — and compares the
    /// corners and indices byte for byte. They are equal because every corner fell back to the rig,
    /// which the shader reads from a uniform: the cadence cannot reach a corner the field did not
    /// measure, which is exactly what the key now says.
    #[test]
    fn an_empty_field_gives_byte_identical_corners_at_every_cadence_tick() {
        let spec = voxel_shapers::WorldSpec::new(2468, voxel_shapers::Config::default());
        let inputs = voxel_appearance::sky::SkyInputs::default();
        let mesh = one_block_face(Face::PosY, MaterialId(0));
        let mut built = Vec::new();
        for tick in [9_000u64, 9_004] {
            let field = crate::field::CellField::of(&spec, tick, inputs, 0, None)
                .expect("a tick the atmosphere can describe");
            let field = std::cell::RefCell::new(field);
            let light = crate::field::light_at(&field);
            let scene =
                Scene::build_with_light(std::iter::once(&mesh), [0.0; 3], &light, None, None);
            assert_eq!(
                field.borrow().held(),
                0,
                "a field measured a cell with no world under it"
            );
            built.push((scene.vertex_bytes(), scene.index_bytes()));
        }
        assert_eq!(
            built[0], built[1],
            "the cadence changed a corner it could not measure"
        );
    }

    /// **The suffix strip cannot reach the corners.** A frame appends its cloud and its bodies to
    /// the terrain's own arrays; `begin_frame` must put back exactly the terrain, and the corner
    /// range must still be self-consistent afterwards — an index that outlived its vertex draws
    /// from a buffer the GPU no longer has.
    #[test]
    fn a_kept_scene_keeps_its_corners_and_drops_the_frames_suffix() {
        let mesh = one_block_face(Face::PosY, MaterialId(0));
        let mut cache = SceneCache::default();
        let key = SceneKey {
            world: 1,
            residency: 1,
            light: 1,
        };
        let no_light = |_: [f64; 3]| None;
        let built = cache.take(key, || {
            (
                Scene::build_with_light(std::iter::once(&mesh), [0.0; 3], &no_light, None, None),
                true,
            )
        });
        let terrain = built.terrain;
        let vertices = built.vertices.clone();
        let indices = built.indices.clone();
        assert_eq!(vertices.len(), 4, "the quad is not four corners");
        cache.put(built);
        // The frame's own additions: a cloud record on its own origin, and a body.
        let mut scene = cache.take(key, || unreachable!("a kept frame rebuilt"));
        scene.clouds.push(QuadRecord::of(
            &one_block_face(Face::PosY, MaterialId(0)).quads[0],
            0,
        ));
        scene.cloud_origins.push([8.0, 0.0, 0.0, 0.9]);
        scene.bodies.push(QuadRecord::of(
            &one_block_face(Face::PosY, MaterialId(0)).quads[0],
            0,
        ));
        scene.origins.push([3.0e8, 0.0, 0.0, 0.0]);
        cache.put(scene);

        let scene = cache.take(key, || unreachable!("a kept frame rebuilt"));
        assert_eq!(scene.terrain, terrain, "the terrain's own counts moved");
        assert_eq!(
            scene.records.len(),
            terrain[0],
            "a cloud record outlived its frame"
        );
        assert_eq!(
            scene.origins.len(),
            terrain[1],
            "an origin outlived its frame"
        );
        assert_eq!(scene.bodies.len(), 0, "a body outlived its frame");
        assert_eq!(scene.clouds.len(), 0, "a cloud record outlived its frame");
        assert_eq!(
            scene.cloud_origins.len(),
            0,
            "a cloud origin outlived its frame"
        );
        assert_eq!(
            scene.vertices, vertices,
            "a corner was stripped or rewritten"
        );
        assert_eq!(scene.indices, indices, "an index was stripped or rewritten");
        let reach = scene.vertices.len() as u32;
        assert!(
            scene.indices.iter().all(|i| *i < reach),
            "an index points past the corners that keep it"
        );
        assert!(
            scene
                .vertices
                .iter()
                .all(|v| (v.chunk as usize) < scene.origins.len()),
            "a corner names an origin the strip removed"
        );
    }

    /// **The cell the scene asks is the voxel the corner stands in** — the one address law, held
    /// against the geometry rather than against itself.
    ///
    /// This is the pin for the defect this pass repaired. The draft asked the field at
    /// **subvoxel** grain (`floor(p / 0.25 m)`) while the field read its integers as metres, so
    /// every corner named a cell four times out: a real cell of this world, in a chunk four times
    /// away, with no tree under it. Restoring that grain makes the first corner fail here.
    #[test]
    fn the_cell_the_scene_asks_is_the_voxel_the_corner_stands_in() {
        let source = voxel_stream::WorldSource::new(voxel_shapers::WorldSpec::new(
            2468,
            voxel_shapers::Config::default(),
        ));
        let registry = voxel_shapers::materials::registry().expect("the registry");
        let config = MeshConfig::from_registry(&registry).expect("a mesh config");
        let mut mesher = Mesher::new();
        let mut meshes = Vec::new();
        for cy in 0..2 {
            for cx in 0..2 {
                for cz in 0..2 {
                    let pos = voxel_model::ChunkPos::new(cx, cy, cz);
                    let Ok(tree) = source.build_chunk(pos) else {
                        continue;
                    };
                    let chunk = voxel_mesh::Chunk::from_tree(pos, tree);
                    let neighbourhood = source.neighborhood(pos);
                    if let Ok(mesh) = mesher.build(&chunk, &config, Some(&neighbourhood)) {
                        meshes.push(mesh);
                    }
                }
            }
        }
        assert!(!meshes.is_empty(), "the world produced no chunk to mesh");
        let asked = std::cell::RefCell::new(std::collections::HashSet::new());
        let record = |point: [f64; 3]| {
            if let Some(cell) = crate::field::SubvoxelPos::of_point(point) {
                asked.borrow_mut().insert(cell);
            }
            None
        };
        let scene = Scene::build_with_light(meshes.iter(), [0.0; 3], &record, None, None);
        assert!(!scene.vertices.is_empty(), "the scene drew no corners");
        let asked = asked.borrow();
        for v in &scene.vertices {
            // With the eye at the origin a corner's world position is its own metres plus the
            // chunk's low corner — the same sum the shader does in reverse.
            let origin = &scene.origins[v.chunk as usize];
            let world = [
                v.position[0] as f64 + origin[0] as f64,
                v.position[1] as f64 + origin[1] as f64,
                v.position[2] as f64 + origin[2] as f64,
            ];
            let cell = crate::field::SubvoxelPos::of_point(world).expect("a finite corner");
            assert!(
                asked.contains(&cell),
                "the corner at {world:?} stands in {cell:?}, which the scene never asked about"
            );
        }
    }

    /// **A scene stepped in slices is the scene stepped once**, and no step builds more than its
    /// allowance.
    ///
    /// This is the pin the frame's budget rests on: the geometry slice is only a budget if the
    /// walk can stop anywhere and resume with the same bytes, and if a step never emits more than
    /// it was given. A regression that makes a step unbounded — the burst this pass removed —
    /// fails on the first assertion; one that makes the resume skip or repeat a quad fails on the
    /// byte comparison.
    #[test]
    fn a_scene_built_in_slices_is_the_scene_built_in_one_step() {
        let source = voxel_stream::WorldSource::new(voxel_shapers::WorldSpec::new(
            2468,
            voxel_shapers::Config::default(),
        ));
        let registry = voxel_shapers::materials::registry().expect("the registry");
        let config = MeshConfig::from_registry(&registry).expect("a mesh config");
        let mut mesher = Mesher::new();
        let mut meshes = Vec::new();
        for cy in 0..2 {
            for cx in 0..2 {
                for cz in 0..2 {
                    let pos = voxel_model::ChunkPos::new(cx, cy, cz);
                    let Ok(tree) = source.build_chunk(pos) else {
                        continue;
                    };
                    let chunk = voxel_mesh::Chunk::from_tree(pos, tree);
                    let neighbourhood = source.neighborhood(pos);
                    if let Ok(mesh) = mesher.build(&chunk, &config, Some(&neighbourhood)) {
                        meshes.push(mesh);
                    }
                }
            }
        }
        meshes.sort_unstable_by_key(|m| (m.chunk.x, m.chunk.y, m.chunk.z));
        let refs: Vec<&Mesh> = meshes.iter().collect();
        let whole = Scene::build(refs.iter().copied(), [0.0; 3], None, None);

        const SLICE: usize = 64;
        let mut build = SceneBuild::new();
        let mut frames = 0;
        let mut walked = 0usize;
        loop {
            let complete = build.step(&refs, None, None, None, SLICE);
            let now = build.walked();
            let emitted = now - walked;
            assert!(
                emitted <= SLICE,
                "a step of {SLICE} quads emitted {emitted} — a burst with no ceiling"
            );
            walked = now;
            frames += 1;
            if complete {
                break;
            }
            assert!(frames < 100_000, "the sliced build never finished");
        }
        let sliced = build.finish();
        assert_eq!(
            frames,
            whole.quads.div_ceil(SLICE).max(1),
            "the sliced build took {frames} steps for {} quads",
            whole.quads
        );
        assert_eq!(sliced.vertices, whole.vertices, "the corners differ");
        assert_eq!(sliced.indices, whole.indices, "the indices differ");
        assert_eq!(sliced.records, whole.records, "the rectangles differ");
        assert_eq!(sliced.origins, whole.origins, "the chunks differ");
        assert_eq!(sliced.terrain, whole.terrain, "the terrain counts differ");
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
        assert_eq!(
            last,
            Vertex::STRIDE,
            "the attributes do not cover the vertex"
        );
        for a in Vertex::attributes() {
            assert!(
                a.offset.is_multiple_of(4),
                "offset {} is not 4-aligned",
                a.offset
            );
        }
    }

    #[test]
    fn a_vertex_encodes_to_exactly_one_stride_of_bytes() {
        let v = Vertex {
            position: [1.0, -2.0, 0.25],
            normal: [0.0, 1.0, 0.0],
            material: 5,
            detail_roughness: 0.0,
            plane_axis: -1,
            light_direct: [0.0; 3],
            light_diffuse: [0.0; 3],
            light_present: 0,
            chunk: 0,
        };
        // `write_bytes` writes into a buffer the caller owns; the old
        // `Vertex::vertex_bytes` returned a fresh one and no longer exists.
        let mut bytes = Vec::new();
        v.write_bytes(&mut bytes);
        assert_eq!(bytes.len(), Vertex::STRIDE as usize);
    }

    /// **Every scene a build hands back states its own terrain**, which is the boundary a cache
    /// truncates to. The exits that matter are the index-space guard's: a build that stops early
    /// still has to seal `terrain`, or `begin_frame` would strip the corners it had already made
    /// and draw an empty frame — so the seal is checked by construction rather than by reaching
    /// four billion indices in a test.
    #[test]
    fn every_built_scene_states_the_terrain_a_cache_truncates_to() {
        const ROCK: MaterialId = MaterialId(6);
        let roof = one_block_face(Face::PosY, ROCK);
        let lit = |_: [f64; 3]| Some([1.0f32; 6]);
        let field = a_pillar(ROCK, Face::PosY);
        let config = RefineConfig::new(4, 4096).expect("inside the level");
        let scenes = [
            Scene::build([&roof], [0.0; 3], None, None),
            Scene::build([&roof], [0.0; 3], Some(&field), Some(&config)),
            Scene::build_with_light([&roof], [0.0; 3], &lit, None, None),
            Scene::build_with_light(Vec::new().iter(), [0.0; 3], &lit, None, None),
        ];
        for (i, scene) in scenes.iter().enumerate() {
            assert_eq!(
                scene.terrain,
                [
                    scene.records.len(),
                    scene.origins.len(),
                    scene.vertices.len(),
                    scene.indices.len()
                ],
                "scene {i} does not agree with its own terrain"
            );
        }
        // And an empty build is a scene with a zero terrain rather than one with no statement.
        assert_eq!(scenes[3].terrain, [0, 0, 0, 0]);
    }

    #[test]
    fn a_scene_cache_builds_only_when_its_key_moves() {
        let mut cache = SceneCache::default();
        let mut builds = 0u32;
        let key = SceneKey {
            world: 7,
            residency: 11,
            light: 0,
        };
        let mut build = |builds: &mut u32| {
            *builds += 1;
            (Scene::probe([0.0, 1.0, 0.0], 0, 2.0, 0.5, [0.0; 3]), true)
        };
        let first = cache.take(key, || build(&mut builds));
        let build_id = first.build;
        cache.put(first);
        // The same key: the scene is kept, and the frame's own additions are stripped back to
        // the terrain the build left.
        let mut second = cache.take(key, || build(&mut builds));
        assert_eq!(builds, 1, "the cache built a scene it already had");
        assert_eq!(second.build, build_id, "a kept scene kept its build id");
        assert_eq!(second.vertices.len(), 4, "the probe's own four corners");
        assert_eq!(second.terrain, [0, 1, 4, 6]);
        second.clouds.push(QuadRecord::of(
            &one_block_face(Face::PosY, MaterialId(0)).quads[0],
            0,
        ));
        second.cloud_origins.push([1.0, 2.0, 3.0, 0.9]);
        cache.put(second);
        let third = cache.take(key, || build(&mut builds));
        assert_eq!(builds, 1);
        assert_eq!(
            third.clouds.len(),
            0,
            "the previous frame's cloud survived the take"
        );
        assert_eq!(third.origins.len(), 1, "and so did its origins");
        cache.put(third);
        // A moved key: the world (or its residency, or the light) changed, so the mesh walk runs
        // again and the new scene states a new build.
        let moved = SceneKey {
            residency: 12,
            ..key
        };
        let fourth = cache.take(moved, || build(&mut builds));
        assert_eq!(builds, 2, "a moved key did not rebuild");
        assert_ne!(
            fourth.build, build_id,
            "a rebuilt scene kept the old build id"
        );
        cache.put(fourth);
        // And a build that refused to be kept — the light field measuring new cells — is asked
        // for again under the very same key, which is the whole reason `take` returns a flag
        // rather than assuming its build is worth keeping.
        let mut refused = 0u32;
        let next = SceneKey {
            residency: 13,
            ..moved
        };
        let _ = cache.take(next, || {
            refused += 1;
            (Scene::probe([0.0, 1.0, 0.0], 0, 2.0, 0.5, [0.0; 3]), false)
        });
        let _ = cache.take(next, || {
            refused += 1;
            (Scene::probe([0.0, 1.0, 0.0], 0, 2.0, 0.5, [0.0; 3]), false)
        });
        assert_eq!(
            refused, 2,
            "a scene the caller did not keep was treated as kept"
        );
    }

    /// **What a scene draws, as placed rectangles**: each record's own shape against the world
    /// origin its `chunk` index names. Two scenes that draw the same world have the same list
    /// however their arrays are ordered, which is the property a resident scene has to keep while
    /// chunks land and leave.
    ///
    /// **A tombstone is not drawn**, so it is not listed: a resident scene leaves a zero-extent
    /// rectangle where a chunk's slice was, and `vs_quad` expands that into a quad of no area.
    /// Listing it would compare a hole against a whole build's absence of one.
    fn placed_records(scene: &Scene) -> Vec<([u64; 3], [u32; 5])> {
        let mut out: Vec<_> = scene
            .records
            .iter()
            .filter(|r| r.extent != 0)
            .map(|r| {
                let o = scene.origins[r.chunk as usize];
                (
                    [o[0].to_bits(), o[1].to_bits(), o[2].to_bits()],
                    [r.face, r.material, r.slice, r.uv, r.extent],
                )
            })
            .collect();
        out.sort_unstable();
        out
    }

    fn one_face_at(pos: voxel_model::ChunkPos) -> Mesh {
        use voxel_model::{Face, MaterialId};
        Mesh {
            chunk: pos,
            origin: {
                let o = pos.origin();
                [o.x, o.y, o.z]
            },
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
        }
    }

    /// **The resident scene is the world it was synced to, after a landing and after a leave.**
    ///
    /// The invariant is the picture's: every live chunk's record names an origin that is that
    /// chunk's own, and the list of placed rectangles equals the list a whole build of the same
    /// surviving meshes makes. A swap-remove that moved a block without rebinding it would draw a
    /// chunk at another chunk's place, and this fails on the first removal.
    #[test]
    fn a_resident_scene_is_the_world_it_was_synced_to() {
        use voxel_model::ChunkPos;
        let a = one_face_at(ChunkPos::new(0, 0, 0));
        let b = one_face_at(ChunkPos::new(1, 0, 0));
        let c = one_face_at(ChunkPos::new(2, 0, 0));
        let mut resident = ResidentScene::new();
        let centre = ChunkPos::new(1, 0, 0);
        fn drawn<'a>(meshes: &[&'a Mesh]) -> Vec<(ChunkPos, u64, &'a Mesh)> {
            meshes.iter().map(|m| (m.chunk, 1u64, *m)).collect()
        }
        let all = [&a, &b, &c];
        let report = resident.sync(drawn(&all).into_iter(), centre, None, None, None, 1_000, 0);
        assert_eq!(
            (report.landed, report.left),
            (3, 0),
            "the first sync did not land three"
        );
        let expected = Scene::build_with_detail([&a, &b, &c], [0.0; 3]);
        assert_eq!(placed_records(resident.scene()), placed_records(&expected));

        // **One leaves**: the survivor of the swap is the point of the test.
        let survivors = [&a, &c];
        let report = resident.sync(
            drawn(&survivors).into_iter(),
            centre,
            None,
            None,
            None,
            1_000,
            0,
        );
        assert_eq!(
            (report.landed, report.left),
            (0, 1),
            "the leave did not happen"
        );
        assert!(
            !resident.holds(b.chunk),
            "the chunk that left is still in the scene"
        );
        let expected = Scene::build_with_detail([&a, &c], [0.0; 3]);
        assert_eq!(
            placed_records(resident.scene()),
            placed_records(&expected),
            "a surviving chunk is drawn at another chunk's origin"
        );
        assert!(
            !resident.scene().written_records.is_empty(),
            "the swap-remove wrote no record range"
        );

        // **And one lands** again, at a place that sorts before the survivors: the arrays must
        // still place every chunk where it stands.
        let again = [&a, &c, &b];
        let report = resident.sync(
            drawn(&again).into_iter(),
            centre,
            None,
            None,
            None,
            1_000,
            0,
        );
        assert_eq!(report.landed, 1);
        let expected = Scene::build_with_detail([&a, &b, &c], [0.0; 3]);
        assert_eq!(placed_records(resident.scene()), placed_records(&expected));

        // **A chunk whose mesh moved is replaced, not left beside its old self.** A version bump
        // with two rectangles must draw two, not three.
        let mut b2 = one_face_at(b.chunk);
        b2.quads.push(Quad {
            face: voxel_model::Face::PosY,
            slice: 8,
            uv: [2, 2],
            extent: [2, 2],
            material: voxel_model::MaterialId(6),
        });
        let remeshed = [(&a, 1u64), (&b2, 2u64), (&c, 1u64)];
        let drawn: Vec<_> = remeshed.iter().map(|(m, v)| (m.chunk, *v, *m)).collect();
        let report = resident.sync(drawn.into_iter(), centre, None, None, None, 1_000, 0);
        assert_eq!(report.left, 0, "a remeshed chunk was counted as leaving");
        assert_eq!(report.landed, 1, "the remeshed chunk did not land again");
        let expected = Scene::build_with_detail([&a, &b2, &c], [0.0; 3]);
        assert_eq!(
            placed_records(resident.scene()),
            placed_records(&expected),
            "the remeshed chunk still draws the world the edit removed"
        );
    }

    /// **A window that walks lands and drops chunks of every size and never misplaces a survivor.**
    ///
    /// The playtest's hole was a swap-remove that was right for equal slices: the arrays were cut
    /// at the moved slice's start, so a departing chunk smaller than the last one left its
    /// neighbour's corners outside the array and the next removal indexed past the end. Chunks
    /// here are one rectangle, two, or none at all — a merge that found nothing to draw is a real
    /// window member and the smallest slice there is — and the walk crosses enough columns that
    /// every chunk is eventually the one that leaves, the one that moves and the one under the
    /// eye. `ResidentScene::check` states the array invariant on every sync of a debug build; this
    /// test adds the picture: the placed rectangles are a whole build of the survivors.
    #[test]
    fn a_walking_window_lands_and_drops_every_size_of_chunk() {
        use voxel_model::ChunkPos;
        fn shape(pos: ChunkPos, quads: usize) -> Mesh {
            let mut mesh = one_face_at(pos);
            let one = mesh.quads[0];
            mesh.quads = (0..quads)
                .map(|i| Quad {
                    slice: 4 + i as u16,
                    ..one
                })
                .collect();
            mesh.stats.quads = quads as u64;
            mesh
        }
        let mut resident = ResidentScene::new();
        let mut live: std::collections::BTreeMap<ChunkPos, (u64, Mesh)> =
            std::collections::BTreeMap::new();
        let (mut landed, mut left) = (0usize, 0usize);
        for step in 0..24i32 {
            let centre = ChunkPos::new(step, 0, 0);
            // The window: two columns behind the eye and three ahead, each chunk's size its own.
            // A chunk is remeshed every fourth step, which is the version bump an edit makes.
            let mut frame: std::collections::BTreeMap<ChunkPos, (u64, Mesh)> =
                std::collections::BTreeMap::new();
            for dx in -2..=3 {
                let pos = ChunkPos::new(step + dx, 0, 0);
                let quads = (pos.x.rem_euclid(4)) as usize;
                let remeshed = (pos.x + step).rem_euclid(4) == 0;
                let version = live.get(&pos).map(|(v, _)| *v).unwrap_or(1)
                    + u64::from(remeshed && live.contains_key(&pos));
                frame.insert(pos, (version, shape(pos, quads)));
            }
            // **The light moves too**: a revision re-emits every chunk in place, which is the
            // other writer of these arrays and the one that must not disturb the arena.
            let report = resident.sync(
                frame.iter().map(|(p, (v, m))| (*p, *v, m)),
                centre,
                None,
                None,
                None,
                1_000,
                step as u64 / 3,
            );
            landed += report.landed;
            left += report.left;
            let expected = Scene::build_with_detail(frame.values().map(|(_, m)| m), [0.0; 3]);
            assert_eq!(
                placed_records(resident.scene()),
                placed_records(&expected),
                "step {step}: the window's rectangles are not where the chunks stand"
            );
            live = frame;
        }
        assert!(landed > 30, "the walk landed {landed} chunk(s)");
        assert!(left > 20, "the walk dropped {left} chunk(s)");
    }

    /// **The arena's dead space is bounded while the window walks.** A landing takes the smallest
    /// hole that fits and otherwise appends, and a light refresh can change a chunk's size (a face
    /// that stops varying falls from a grid to four corners), so sizes move under the holes. The
    /// shipped window grew from 1.3 M corners to 14.4 M in fifteen seconds and outgrew the device's
    /// largest buffer; this walks the same churn far enough for the bound to be crossed and states
    /// it: the arrays never exceed a quarter more than their live slices, plus the floor.
    #[test]
    fn a_walking_window_does_not_grow_the_corner_arena_past_its_bound() {
        use voxel_model::ChunkPos;
        fn shape(pos: ChunkPos, quads: usize) -> Mesh {
            let mut mesh = one_face_at(pos);
            let one = mesh.quads[0];
            mesh.quads = (0..quads)
                .map(|i| Quad {
                    slice: 4 + i as u16,
                    ..one
                })
                .collect();
            mesh.stats.quads = quads as u64;
            mesh
        }
        // **The light is the churn, and it is the shipped window's own.** A face whose samples
        // cross a shadow edge is a grid (25 corners on a one-voxel face at `SHADOW_LATTICE`); a
        // face wholly on one side is four. Each light revision therefore changes the slice's size
        // by a factor of six, which is exactly what the window's own light revisions do.
        let varying: &LightAt = &|point: [f64; 3]| {
            // A smooth field sampled at the face's corners: most faces straddle a sign change,
            // which is the condition the lattice is emitted on.
            let lit = (point[0] * 1.7).sin() > 0.0;
            Some(if lit { [1.0f32; 6] } else { [0.0f32; 6] })
        };
        let even: &LightAt = &|_: [f64; 3]| Some([1.0f32; 6]);
        let mut resident = ResidentScene::new();
        resident.set_shadow_lattice(SHADOW_LATTICE);
        for step in 0..24i32 {
            let centre = ChunkPos::new(step, 0, 0);
            let mut frame: std::collections::BTreeMap<ChunkPos, (u64, Mesh)> =
                std::collections::BTreeMap::new();
            for dx in -4..=4 {
                let pos = ChunkPos::new(step + dx, 0, 0);
                frame.insert(pos, (1u64, shape(pos, 60)));
            }
            let light = if step % 2 == 0 { varying } else { even };
            resident.sync(
                frame.iter().map(|(p, (v, m))| (*p, *v, m)),
                centre,
                Some(light),
                None,
                None,
                100_000,
                step as u64,
            );
            let live: usize = resident
                .blocks
                .values()
                .map(|block| (block.vertices[1] - block.vertices[0]) as usize)
                .sum();
            let held = resident.corners();
            assert!(
                held <= live + live / ARENA_DEAD_IN + ARENA_FLOOR,
                "step {step}: the arena holds {held} corner(s) for {live} live — the bound is \
                 {} and the compaction did not run",
                live + live / ARENA_DEAD_IN + ARENA_FLOOR
            );
        }
        assert!(
            resident.compactions() > 0,
            "the walk never crossed the bound, so the compaction was not exercised"
        );
    }

    /// **The nearest landing fits the first frame, whatever the budget says**, so the chunk under
    /// the eye is never the one a budget defers.
    #[test]
    fn the_chunk_under_the_eye_lands_before_the_far_ones() {
        use voxel_model::ChunkPos;
        let near = one_face_at(ChunkPos::new(0, 0, 0));
        let far = one_face_at(ChunkPos::new(4, 0, 0));
        let mut resident = ResidentScene::new();
        let meshes = [&far, &near];
        let drawn: Vec<_> = meshes.iter().map(|m| (m.chunk, 1u64, *m)).collect();
        let report = resident.sync(
            drawn.into_iter(),
            ChunkPos::new(0, 0, 0),
            None,
            None,
            None,
            1,
            0,
        );
        assert_eq!(
            report.landed, 1,
            "a budget of one quad landed more than one chunk"
        );
        assert!(
            resident.holds(near.chunk),
            "the eye's chunk waited for the far one"
        );
        assert_eq!(report.pending, 1);
        // The carried chunk lands on the next frame's budget, not never.
        let drawn: Vec<_> = meshes.iter().map(|m| (m.chunk, 1u64, *m)).collect();
        let report = resident.sync(
            drawn.into_iter(),
            ChunkPos::new(0, 0, 0),
            None,
            None,
            None,
            1_000,
            0,
        );
        assert_eq!(
            (report.landed, report.pending),
            (1, 0),
            "the carried chunk did not land"
        );
        assert!(resident.holds(far.chunk));
    }

    /// **A chunk the last frame carried does not stand in the eye's way.**
    ///
    /// The landing order is distance, and a carried chunk's distance was measured from the frame
    /// that deferred it. When the eye walks toward it, the scene's **new** centre has to decide —
    /// otherwise a far chunk queued a frame ago lands first and the one under the eye waits, which
    /// is the very frame this path exists to remove.
    #[test]
    fn a_chunk_carried_from_an_older_frame_does_not_hold_the_eye_s_arrival() {
        use voxel_model::ChunkPos;
        let behind = one_face_at(ChunkPos::new(0, 0, 0));
        let arriving = one_face_at(ChunkPos::new(2, 0, 0));
        let far = one_face_at(ChunkPos::new(4, 0, 0));
        let mut resident = ResidentScene::new();
        // One quad of budget lands the nearest chunk and carries the far one to the next frame.
        let first = [&behind, &far];
        let drawn: Vec<_> = first.iter().map(|m| (m.chunk, 1u64, *m)).collect();
        let report = resident.sync(
            drawn.into_iter(),
            ChunkPos::new(0, 0, 0),
            None,
            None,
            None,
            1,
            0,
        );
        assert_eq!(
            (report.landed, report.pending),
            (1, 1),
            "the budget did not defer the far chunk"
        );
        assert!(resident.holds(behind.chunk));
        // The eye has walked to the middle, the carried chunk is still owed, and the chunk it now
        // stands in arrives on this frame. The carried chunk is farther from the new centre, so it
        // must wait another frame — not take the one landing this frame's budget affords.
        let second = [&behind, &arriving, &far];
        let drawn: Vec<_> = second.iter().map(|m| (m.chunk, 1u64, *m)).collect();
        let report = resident.sync(
            drawn.into_iter(),
            ChunkPos::new(2, 0, 0),
            None,
            None,
            None,
            1,
            0,
        );
        assert!(
            resident.holds(arriving.chunk),
            "the chunk under the eye waited behind one carried from an older frame"
        );
        assert_eq!(report.landed, 1, "the frame landed more than its budget");
        assert_eq!(report.pending, 1, "the far chunk was not carried");
    }

    /// **The light's refresh is a pass, not a rebuild**: a moved revision re-emits the chunks it
    /// reaches in place, and the corners it writes stay inside the chunk's own slice.
    #[test]
    fn a_light_revision_refreshes_the_chunk_it_reaches_and_leaves_the_rest() {
        use voxel_model::ChunkPos;
        let a = one_face_at(ChunkPos::new(0, 0, 0));
        let b = one_face_at(ChunkPos::new(1, 0, 0));
        let mut resident = ResidentScene::new();
        let drawn: Vec<_> = [&a, &b].iter().map(|m| (m.chunk, 7u64, *m)).collect();
        let light: &LightAt = &|_: [f64; 3]| Some([1.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
        let report = resident.sync(
            drawn.clone().into_iter(),
            ChunkPos::new(0, 0, 0),
            Some(light),
            None,
            None,
            1_000,
            1,
        );
        assert_eq!(report.landed, 2);
        let scene = resident.scene();
        assert_eq!(
            scene.vertices.len(),
            8,
            "the vertex format is not four corners a quad"
        );
        assert_eq!(
            scene.indices.len(),
            12,
            "two quads are not six indices each"
        );
        assert!(scene
            .indices
            .iter()
            .all(|i| (*i as usize) < scene.vertices.len()));
        // A revision that reaches only the first chunk re-emits one slice and leaves the other.
        let drawn: Vec<_> = [&a, &b].iter().map(|m| (m.chunk, 7u64, *m)).collect();
        let light2: &LightAt = &|_: [f64; 3]| Some([2.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
        let report = resident.sync(
            drawn.into_iter(),
            ChunkPos::new(0, 0, 0),
            Some(light2),
            None,
            None,
            1,
            2,
        );
        assert_eq!(
            report.refreshed, 1,
            "the budget refreshed more than one chunk"
        );
        let scene = resident.scene();
        assert_eq!(
            scene.vertices.len(),
            8,
            "a refresh resized the vertex array"
        );
        assert_eq!(
            scene
                .vertices
                .iter()
                .filter(|v| v.light_direct[0] == 2.0)
                .count(),
            4,
            "the refreshed chunk's corners are not the new light"
        );
    }

    // -----------------------------------------------------------------------------------------
    // **The witness the lit path has never had.**
    //
    // `Scene::build_with_detail` is what these tests exercise, and it is handed no light; the lit
    // vertex path is built by [`ResidentScene`], which is what the window draws from. The helpers
    // below put a face through that second path with a field attached and read the drawn profile
    // back off the vertices, so the sampling rule can be judged without a GPU.
    // -----------------------------------------------------------------------------------------

    /// One merged rectangle `extent` subvoxels on a side, standing on `+Y` at `slice = extent`.
    fn merged_face(extent: u16, material: MaterialId) -> Mesh {
        Mesh {
            chunk: voxel_model::ChunkPos::new(0, 0, 0),
            origin: [0, 0, 0],
            quads: vec![Quad {
                face: Face::PosY,
                slice: extent,
                uv: [0, 0],
                extent: [extent, extent],
                material,
            }],
            loops: Vec::new(),
            stats: voxel_mesh::MeshStats {
                solid_voxels: 1,
                visible_faces: 1,
                quads: 1,
                loops: 0,
            },
        }
    }

    /// The drawn light along the face's mid-`z` line, ordered from low `x` to high `x`, in metres.
    ///
    /// A face still drawn as four corners has no vertex on that line: its light is the corners'
    /// own interpolation, both ends lit, so the row it draws is the two lit ends.
    fn drawn_row(scene: &Scene, face_m: f64, y: f64) -> Vec<(f64, f64)> {
        let z = face_m / 2.0;
        let mut row: Vec<(f64, f64)> = scene
            .vertices
            .iter()
            .filter(|v| {
                (v.position[2] as f64 - z).abs() < 1e-4 && (v.position[1] as f64 - y).abs() < 1e-4
            })
            .map(|v| (v.position[0] as f64, v.light_direct[0] as f64))
            .collect();
        row.sort_by(|a, b| a.0.total_cmp(&b.0));
        row.dedup_by(|a, b| (a.0 - b.0).abs() < 1e-6);
        if row.is_empty() {
            row = vec![(0.0, 1.0), (face_m, 1.0)];
        }
        row
    }

    /// The **90 % → 10 % shoulder** of a drawn row: the width a viewer reads as the edge, in
    /// metres, in pixels, and the drop it spans in W/m².
    ///
    /// A row with no drop has no edge — the shoulder is the whole face, which is the leak stated
    /// as a number rather than as an absence.
    fn shoulder(profile: &[(f64, f64)], face_m: f64, face_px: f64) -> (f64, f64, f64) {
        let high = profile.iter().map(|p| p.1).fold(f64::MIN, f64::max);
        let low = profile.iter().map(|p| p.1).fold(f64::MAX, f64::min);
        let drop = high - low;
        if drop <= 1e-9 {
            return (face_m, face_px, drop);
        }
        let scan: Vec<(f64, f64)> = if profile[0].1 >= profile[profile.len() - 1].1 {
            profile.to_vec()
        } else {
            profile.iter().rev().copied().collect()
        };
        let crossing = |fraction: f64| -> f64 {
            let target = high - fraction * drop;
            for pair in scan.windows(2) {
                let (a, b) = (pair[0], pair[1]);
                if a.1 >= target && b.1 <= target && (a.1 - b.1).abs() > 1e-12 {
                    let u = (a.1 - target) / (a.1 - b.1);
                    return a.0 + u * (b.0 - a.0);
                }
            }
            scan[scan.len() - 1].0
        };
        let width_m = (crossing(0.1) - crossing(0.9)).abs();
        (width_m, width_m / face_m * face_px, drop)
    }

    /// Pixels across the face when it is looked down on with the example's camera and the
    /// example's 512 × 288 target, so a metre can be stated at the resolution it is drawn at.
    fn face_pixels(face_m: f64) -> f64 {
        const WIDTH: f64 = 512.0;
        const HEIGHT: f64 = 288.0;
        let mut camera =
            crate::Camera::standard([face_m / 2.0, 1.0 + face_m, face_m / 2.0], WIDTH / HEIGHT);
        camera.pitch_rad = -crate::camera::MAX_PITCH_RAD;
        let matrix = camera.view_projection();
        let left = project(&camera, &matrix, [0.0, 1.0, face_m / 2.0]);
        let right = project(&camera, &matrix, [face_m, 1.0, face_m / 2.0]);
        ((right[0] - left[0]).powi(2) + (right[1] - left[1]).powi(2)).sqrt()
    }

    /// World metres → pixels, with the eye subtracted in `f64` as the scene builder does.
    fn project(camera: &crate::Camera, matrix: &[[f32; 4]; 4], point: [f64; 3]) -> [f64; 2] {
        let v = [
            point[0] - camera.position_m[0],
            point[1] - camera.position_m[1],
            point[2] - camera.position_m[2],
        ];
        let mut clip = [0.0f64; 4];
        for (row, slot) in clip.iter_mut().enumerate() {
            for (column, value) in v.iter().enumerate() {
                *slot += matrix[column][row] as f64 * value;
            }
            *slot += matrix[3][row] as f64;
        }
        let w = clip[3];
        [
            (clip[0] / w * 0.5 + 0.5) * 512.0,
            (1.0 - (clip[1] / w * 0.5 + 0.5)) * 288.0,
        ]
    }

    /// **A shadow wholly inside a merged face must be drawn, at the same shoulder as one with a
    /// corner on its edge.**
    ///
    /// Three builds through the path the window uses: a 1 m rectangle with a step at its middle,
    /// so two corners disagree; a 2 m rectangle carrying a shadow **one stored cell wide** —
    /// 0.25 m, at 0.5–0.75 m — wholly inside it, with no corner on either edge and both corners
    /// lit; and the same 2 m rectangle in even light as the control. The second is the case the
    /// corner rule cannot see: its four corners agree, so the face is emitted as four corners and
    /// the drawn light is their interpolation — no edge at all. It is also the finest shadow the
    /// stored field can answer: `CellField` holds one value per subvoxel, and a narrower shadow
    /// would not change any sample.
    ///
    /// The measured shoulder is the same in metres in both crossed cases, because it is a
    /// distance on the face and not a property of where the step happens to fall; the control's
    /// own ask count is the proof that the rule spends vertices, not field asks.
    #[test]
    fn a_shadow_wholly_inside_a_merged_face_is_resolved_at_the_subvoxel_lattice() {
        const ROCK: MaterialId = MaterialId(6);
        let build = |extent: u16,
                     light: &dyn Fn([f64; 3]) -> Option<[f32; 6]>,
                     light_epoch: u64|
         -> (Scene, u64) {
            let asks = std::cell::Cell::new(0u64);
            let counted = |point: [f64; 3]| {
                asks.set(asks.get() + 1);
                light(point)
            };
            let mesh = merged_face(extent, ROCK);
            let mut resident = ResidentScene::new();
            resident.set_shadow_lattice(SHADOW_LATTICE);
            resident.sync(
                std::iter::once((mesh.chunk, 1, &mesh)),
                mesh.chunk,
                Some(&counted),
                None,
                None,
                usize::MAX,
                light_epoch,
            );
            (resident.take(), asks.get())
        };
        // A step the corners straddle: a lattice line falls on the edge.
        let corner = |p: [f64; 3]| Some(if p[0] < 0.5 { [1.0f32; 6] } else { [0.0f32; 6] });
        // A shadow one subvoxel wide, wholly inside a 2 m face: both corners are lit, and the
        // lattice's 0.25 m lines are at 0.5 m (dark) and 0.75 m (lit).
        let interior = |p: [f64; 3]| {
            Some(if (0.5..0.75).contains(&p[0]) {
                [0.0f32; 6]
            } else {
                [1.0f32; 6]
            })
        };

        let one = build(4, &corner, 1);
        let two = build(8, &interior, 1);
        // The same 2 m face in even light: the grid is asked for either way, and only the
        // face whose interior carries an edge emits it.
        let even = build(8, &|_: [f64; 3]| Some([1.0f32; 6]), 1);
        let one_profile = drawn_row(&one.0, 1.0, 1.0);
        let two_profile = drawn_row(&two.0, 2.0, 2.0);
        let (one_m, one_px, one_drop) = shoulder(&one_profile, 1.0, face_pixels(1.0));
        let (two_m, two_px, two_drop) = shoulder(&two_profile, 2.0, face_pixels(2.0));

        println!(
            "corner case:   {} corner(s), {} index(es), {} ask(s), shoulder {one_px:.1} px = \
             {one_m:.3} m, drop {one_drop:.1}",
            one.0.vertices.len(),
            one.0.indices.len(),
            one.1
        );
        println!(
            "interior case: {} corner(s), {} index(es), {} ask(s), shoulder {two_px:.1} px = \
             {two_m:.3} m, drop {two_drop:.1}",
            two.0.vertices.len(),
            two.0.indices.len(),
            two.1
        );
        println!(
            "even 2 m face: {} corner(s), {} index(es), {} ask(s) — the grid costs nothing until \
             the light uses it",
            even.0.vertices.len(),
            even.0.indices.len(),
            even.1
        );

        assert_eq!(
            one.0.vertices.len(),
            25,
            "a 1 m face at a 0.25 m lattice is 5 × 5 corners, not {}",
            one.0.vertices.len()
        );
        assert_eq!(
            even.0.vertices.len(),
            4,
            "a face whose light does not change pays for four corners, not {}",
            even.0.vertices.len()
        );
        assert_eq!(
            two.1, even.1,
            "the interior face asked {} time(s) against the even face's {}: the rule is costing \
             asks, not vertices",
            two.1, even.1
        );
        assert!(
            one_drop > 0.5,
            "the corner case's shadow is not drawn: drop {one_drop:.2}"
        );
        assert!(
            (one_m - 0.2).abs() < 0.02,
            "the corner case's shoulder is {one_m:.3} m, not the 0.25 m lattice's four-sample span"
        );

        assert_eq!(
            two.0.vertices.len(),
            81,
            "the interior shadow's face is still {} corner(s): the shadow fell wholly inside a 
             merged rectangle and was never resolved",
            two.0.vertices.len()
        );
        assert_eq!(
            one.0.indices.len(),
            96,
            "a crossed 1 m face is 16 cells of two triangles, not {}",
            one.0.indices.len()
        );
        assert_eq!(
            even.0.indices.len(),
            6,
            "the even face is two triangles, not {}",
            even.0.indices.len()
        );
        assert_eq!(
            two.0.indices.len(),
            384,
            "an 8 × 8 grid is 128 cells, six indices each"
        );
        assert!(
            two_drop > 0.5,
            "the interior shadow is not drawn: drop {two_drop:.2}"
        );
        assert!(
            (two_m - one_m).abs() < 0.02,
            "the interior shoulder is {two_m:.3} m against the corner case's {one_m:.3} m"
        );
    }
    /// One chunk of clear air: enough for a beam to cross, so a measured cell has the march's
    /// answer rather than a refusal.
    struct Air(std::sync::Arc<voxel_mesh::SparseVoxelOctree>);

    impl crate::field::ChunkTrees for Air {
        fn tree_at(
            &self,
            _pos: voxel_model::ChunkPos,
        ) -> Option<std::sync::Arc<voxel_mesh::SparseVoxelOctree>> {
            Some(std::sync::Arc::clone(&self.0))
        }
    }

    /// **A drawn corner's light is the field's march or it is nothing.**
    ///
    /// The two builds below are the same face through the same constructor the frame calls. The
    /// first asks a field that has measured nothing: every corner says `light_present = 0` and
    /// carries zero bands, so a chunk in that state is *unlit and says so* rather than wearing the
    /// rig's number as if it were a measurement. The second measures each corner's own cell from a
    /// resident octree — the worker's march, run here on this thread because a test has no worker —
    /// and every corner then carries that measurement, band for band.
    #[test]
    fn a_corner_carries_the_fields_march_or_it_says_it_has_none() {
        let spec = voxel_shapers::WorldSpec::new(2468, voxel_shapers::Config::default());
        let inputs = voxel_appearance::sky::SkyInputs::default();
        let field = crate::field::CellField::of(&spec, 9_000, inputs, 0, None)
            .expect("a tick the atmosphere can describe");
        let field = std::cell::RefCell::new(field);
        let mesh = one_block_face(Face::PosY, MaterialId(0));

        let unmeasured = {
            let light = crate::field::light_at(&field);
            Scene::build_with_light(std::iter::once(&mesh), [0.0; 3], &light, None, None)
        };
        assert!(!unmeasured.vertices.is_empty(), "the face drew no corners");
        for v in &unmeasured.vertices {
            assert_eq!(v.light_present, 0, "an unmeasured corner claimed a light");
            assert_eq!(
                v.light_direct, [0.0; 3],
                "an unmeasured corner carried bands"
            );
            assert_eq!(v.light_diffuse, [0.0; 3]);
        }

        // **The march.** Every corner's own cell is measured from a resident octree, which is the
        // same call the worker makes; the point handed over is the corner's world position.
        let air = Air(std::sync::Arc::new(voxel_mesh::SparseVoxelOctree::new()));
        let mut measured_bands = Vec::new();
        for v in &unmeasured.vertices {
            let point = [
                v.position[0] as f64,
                v.position[1] as f64,
                v.position[2] as f64,
            ];
            let cell = crate::field::SubvoxelPos::of_point(point).expect("a corner has a cell");
            let (bands, _) = field
                .borrow_mut()
                .measure(&air as &dyn crate::field::ChunkTrees, cell)
                .expect("the field has a beam");
            measured_bands.push(bands);
        }
        let measured = {
            let light = crate::field::light_at(&field);
            Scene::build_with_light(std::iter::once(&mesh), [0.0; 3], &light, None, None)
        };
        assert_eq!(measured.vertices.len(), unmeasured.vertices.len());
        for (v, bands) in measured.vertices.iter().zip(&measured_bands) {
            assert_eq!(v.light_present, 1, "a measured corner disowned its bands");
            assert_eq!(
                [
                    v.light_direct[0],
                    v.light_direct[1],
                    v.light_direct[2],
                    v.light_diffuse[0],
                    v.light_diffuse[1],
                    v.light_diffuse[2]
                ],
                *bands,
                "the corner's bands are not the march's"
            );
        }
        assert!(
            measured_bands.iter().flatten().any(|b| *b > 0.0),
            "the march answered a beam and a sky of zero"
        );
    }
}
