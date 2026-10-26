//! The world pipeline — **one** of them, shared by the window and by the test.
//!
//! # Why this is its own type
//!
//! The offscreen renderer exists so a frame can be measured. That only means
//! something if the frame a **player** sees is drawn by the same pipeline the
//! measurement used. If the window built its own pipeline, the `shading.wgsl`
//! transcription could pass its test and the game could still draw something
//! else — and nothing in the repository would say so.
//!
//! So the pipeline is separated from the *target*. `Renderer` owns an offscreen
//! target and reads pixels back; the `shploingus` bin owns a surface and
//! presents. Both call [`WorldPipeline::draw`] with the same shader, the same
//! vertex layout, the same depth test and the same bind group. The only thing
//! they may differ in is the colour format, which is why [`WorldPipeline::new`]
//! takes it as an argument rather than assuming [`crate::TARGET_FORMAT`].
//!
//! # Why the target is not an enum here
//!
//! A `cargo`-visible target abstraction would have to know about `Surface`,
//! which would put a windowing concept inside a crate that deliberately has no
//! windowing dependency. Passing a `&TextureView` in keeps the dependency
//! out and makes the sharing explicit at the call site.

use crate::camera::Camera;
use crate::scene::{Scene, Vertex};
use crate::quad::{origin_bytes, QuadRecord, ORIGIN_SIZE};
use crate::shading::{GpuRig, MaterialParams};
use std::collections::BTreeSet;

use crate::skylight::{admit_sky_rows, SkyModel, NO_SKY_ROWS, SKY_MODEL_BYTES, SKY_MODEL_ROWS};
use crate::ClientError;

/// The quad format's two storage buffers start at one page each: a scene with no
/// geometry still binds a valid buffer, and `write_quads` doubles them only when
/// a window grows past them.
const QUAD_START_BYTES: u64 = 4096;

/// **Nothing has been stated about the sky yet** — the value [`WorldPipeline`]'s own record holds
/// until a caller uploads a model or states that there is none. It is not a row count, so it is
/// the one `usize` a row count can never be.
const SKY_NOT_STATED: usize = usize::MAX;

/// **The rows of a stated absence**: the buffer's own layout, all zeros, which the shader's
/// arithmetic answers with black ([`NO_SKY_ROWS`]).
const ZERO_SKY_MODEL: [u8; SKY_MODEL_BYTES] = [0u8; SKY_MODEL_BYTES];


/// **The one place a pass is handed the display stage**, and the reason this is a
/// function rather than three `include_str!`s.
///
/// WGSL has no `include`, and a shader cannot call another shader. That is true of WGSL
/// and it was never true of this build: the host has `include_str!`. So `display.wgsl`
/// holds `srgb_encode`, `srgb_decode`, `bayer8`, `dithered` and `tone_map` **once**, and
/// every pass that presents a frame is compiled with it already in scope.
///
/// Before this, three shaders restated it and each of them said so in a comment — *"WGSL
/// has no `include`"* — which left the display transform as three transcriptions with
/// nothing comparing them. A curve changed in one pass and not in the others is not a
/// compile error: it is a frame whose sky and whose clouds disagree about what white is,
/// and it is visible only as a seam where they meet.
///
/// The passes that do **not** present — the merge compute pass, the wireframe — are not
/// given it, because neither writes a colour into the target.
/// `the_display_stage_is_one_file_and_every_presenting_pass_receives_it` is the witness.
///
/// **The same treatment, and the same reason, for the air.** `atmosphere.wgsl` holds the
/// transport a sky is made of, the single-scatter integral, the phase, the air mass and the
/// twilight's own curve, and the two passes that draw light through air are compiled with it:
/// `sky.wgsl`, which draws it, and `shading.wgsl`, which reflects it off a fluid. The rows it
/// reads are bound at **binding 5 in both**, so the sky a fluid reflects is the sky overhead by
/// construction rather than by agreement.
///
/// **What that replaced was the same defect with a bigger blast radius.** The drawn sky was a
/// 64x64 table of `voxel_appearance::sky`'s fit while the light rig was built from
/// `voxel_light::solver`'s bands, so a frame and the world it stood in were lit by two
/// different skies and nothing compared them. `the_transport_is_one_file_and_both_passes_receive_it`
/// is the witness, beside this one.
pub(crate) fn pass_source(pass: &str) -> String {
    format!(
        "{}\n{}\n{}\n{}",
        include_str!("display.wgsl"),
        include_str!("atmosphere.wgsl"),
        include_str!("horizon.wgsl"),
        pass
    )
}

/// Everything needed to draw the world into a colour target the caller owns.
pub struct WorldPipeline {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    view: wgpu::Buffer,
    /// The lighting rig's uniform, kept so a **later frame can replace it**.
    ///
    /// This did not exist before the sky did. The rig was written once at
    /// construction and never again, which was correct while the rig was
    /// `LightRig::standard()` — a fixture that does not change. Once the sun moves
    /// (BLD-0014), a frame that cannot re-upload its lighting is a pipeline that
    /// renders yesterday's afternoon at midnight, so the buffer is held rather
    /// than dropped after the first `write_buffer`.
    rig: wgpu::Buffer,
    /// The material table, held so a grown quad buffer can be re-bound without a
    /// second statement of the bind group's five entries.
    materials: wgpu::Buffer,
    rows: u32,
    /// The pattern's identity, held as the shader reads it: the lattice's seed and
    /// domain tag (both `u32` pairs) and the two sides in samples per axis. Written
    /// with the camera, because the frame they describe is the frame being drawn
    /// (C-161).
    lattice: [u32; 4],
    rates: [u32; 4],
    /// **Whether this target quantises, and so whether the fragment shades are
    /// dithered** (C-166).
    ///
    /// Set from the colour format at construction, because the format is the only
    /// thing that knows: the window presents an 8-bit `sRGB` surface, which rounds
    /// every shade to one of 256 levels and turns a smooth gradient — the sky is
    /// the loudest — into bands, while the offscreen verification target is
    /// `Rgba32Float` and keeps the value the model computed. A verification can
    /// override it with [`WorldPipeline::set_dither`], because the float target is
    /// exactly where the bands cannot exist, so measuring the dither there means
    /// turning it on by hand.
    dither: f32,
    /// **Whether this target is a display, and so whether a scene-referred value is
    /// compressed into one it can present** (C-167).
    ///
    /// Separate from `dither` on purpose, though the same target sets both today:
    /// the dither answers *how many steps does this target have*, and this answers
    /// *how much range does it have*. An HDR display is a display that quantises
    /// nothing, and a frame that rendered into a float intermediate would set this
    /// on the *present* pass and neither on the world pass. One flag would have to
    /// be split then, and splitting a flag later is how two meanings come to share a
    /// bit. See [`WorldPipeline::set_tonemap`].
    tonemap: f32,
    /// **The cloud's cross-fade weight**: the share of the sky the newer of the two trees a frame
    /// is joining owns, so `1 - this` is the older one's (C-183).
    ///
    /// It is a **frame's** own number, like the pixel angle and unlike the two flags above: the
    /// trees themselves say which side each rectangle is on
    /// ([`crate::cloud_cubes::veil_with_side`]) and the GPU interpolates between them, so a
    /// dissolve costs one uniform write per frame instead of a rebuilt veil per rectangle. `1.0`
    /// is the whole of the sky on the newer side, which is what a frame with one tree in hand —
    /// and every frame before clouds existed — is drawn at.
    cloud_fade: f32,
    /// **Which of the material's layers the world pass draws** (`0` the shaded picture, `1..=4`
    /// the layer views `shading.wgsl`'s `debug_layer` block names). It is a frame's number like
    /// the fade: the geometry and the material table are untouched, so a layer costs one uniform
    /// write and a layer view can be taken of the scene that was just drawn rather than of a
    /// second scene built to be looked at.
    debug_layer: u32,
    /// The sky pass: the transport's rows, and the numbers that turn a pixel into a direction.
    ///
    /// **The rows and the integral both exist once.** The transport is `atmosphere.wgsl`'s — one
    /// transcription, compiled into this pass and into the world pass, which binds the same buffer
    /// at the same binding number — and the rows are the model the CPU assembled from the band
    /// table and the shapers' own air ([`crate::skylight::SkyModel`]). Before this, the pass drew a
    /// 64x64 table of `voxel_appearance::sky`'s fit, so the sky on screen and the sky the light rig
    /// was built from were two models that nothing compared.
    sky_pipeline: wgpu::RenderPipeline,
    sky_bind_group: wgpu::BindGroup,
    sky_view: wgpu::Buffer,
    sky_model: wgpu::Buffer,
    /// **The rows the sky's own buffer holds a sky in**, as the host stated them: `None` until
    /// [`WorldPipeline::write_sky_model`] or [`WorldPipeline::write_no_sky_model`] says otherwise.
    ///
    /// The buffer is the length `atmosphere.wgsl` declares from the moment it is created, so its
    /// bytes cannot say whether a model ever arrived — a host that stated nothing and a host that
    /// stated no sky hold the same zeros. This record is what tells those apart, and
    /// [`WorldPipeline::admit_sky`] refuses a frame drawn out of the first.
    sky_rows: std::sync::atomic::AtomicUsize,
    /// **The astronomical body's own pipeline** (C-184): the same shader module, layout and record
    /// and origin buffers the rectangles use, with `vs_body` and `fs_body`. The bodies are **records
    /// in the scene** rather than fields here, exactly as a rectangle is — which is what lets the
    /// streamer place one and what retired the sky pass's own uniform cuboid.
    body_pipeline: wgpu::RenderPipeline,
    /// **The quad format's own pipeline** (C-168): the same fragment shader, with
    /// a vertex stage that pulls a rectangle out of a storage buffer and
    /// expands it. See `shading.wgsl`'s `vs_quad`.
    quad_pipeline: wgpu::RenderPipeline,
    /// **The cloud's own pipeline state** (C-179): the *same* shader module, layout and
    /// fragment — what differs is only what a veil needs and a surface must not have. The
    /// blend composites the cloud over the sky standing behind it by the veil the fragment
    /// writes as alpha, and depth is tested but **not written**, because a cell is a
    /// translucent box and a box that wrote depth would hide the cells behind it inside the
    /// same cloud.
    cloud_pipeline: wgpu::RenderPipeline,
    /// **The debug wireframe's own pipeline** (F4): the merge's own records outlined as a
    /// `LineList`. The *same* module, layout and record buffers the shaded draw reads, so a line
    /// is the record's border by construction and not by a second expansion that could drift
    /// from it. See `shading.wgsl`'s `vs_wire`/`fs_wire`.
    wire_pipeline: wgpu::RenderPipeline,
    /// The rectangles, and one eye-relative origin per chunk they name. Both are
    /// held rather than created per frame, which is the point of the format: a
    /// rectangle does not change when the camera moves.
    quad_records: wgpu::Buffer,
    quad_origins: wgpu::Buffer,
    quad_record_capacity: u64,
    /// **Whether the record buffer holds a scene at all.** A buffer is allocated at a capacity
    /// before any record is written into it, so "it did not grow this frame" is not the same
    /// question as "it has contents". A resident scene owes the whole array on the first frame
    /// after allocation — the rule the corner path states as `grew ⇒ everything` — and this is
    /// the flag that makes it true when the capacity was reserved up front.
    quad_records_filled: bool,
    quad_origin_capacity: u64,
    /// **The cloud's own record and origin buffers, and their bind group** (2026-10-22). They
    /// used to be a suffix of the terrain arrays, so every chunk that landed or left moved the
    /// sky and rewrote it. A cloud record names its own origin array, and the two are drawn by
    /// the same shader through a bind group whose record and origin entries are these.
    cloud_records: wgpu::Buffer,
    cloud_origins: wgpu::Buffer,
    cloud_record_capacity: u64,
    cloud_origin_capacity: u64,
    cloud_bind_group: wgpu::BindGroup,
    cloud_record_last: Vec<QuadRecord>,
    cloud_origin_last: Vec<[f64; 4]>,
    cloud_origin_eye: Option<[f64; 3]>,
    /// **What is currently on the GPU, held as values rather than as bytes.**
    /// A rectangle is camera-independent and the mesh set changes only when a
    /// chunk arrives, is evicted, or is remeshed, so in the steady state the whole
    /// of the per-frame cost is this comparison — and comparing 24-byte records
    /// directly is a `memcmp`, where comparing their serialisation would mean
    /// building and throwing away 1.13 MB of little-endian bytes every frame to
    /// discover that nothing moved.
    quad_record_last: Vec<QuadRecord>,
    /// The bodies as they were last uploaded, so a frame that changes no body writes nothing.
    body_record_last: Vec<QuadRecord>,
    /// The origins as uploaded, **with the veil in `w`** (C-179) — the array the comparison is
    /// against, and therefore the array whose type has to carry the veil.
    quad_origin_last: Vec<[f64; 4]>,
    /// **The eye the origin buffer's bytes were derived with**, or `None` before the first
    /// upload. The buffer holds `origin − eye`, so a moving eye is a change to it even when
    /// the world-space origins are identical — which is what `write_quads` compares. Keying
    /// the upload on the world-space array alone left the chunks standing in the eye's frame:
    /// the world did not move while its material pattern, addressed by `eye_phase` and
    /// `eye_voxel` (uploaded with the camera every frame), scrolled correctly.
    quad_origin_eye: Option<[f64; 3]>,
    /// **The corner format's two buffers, and the scene build they hold.**
    ///
    /// A corner is chunk-local now ([`crate::scene::Vertex`]), and the light it carries is
    /// answered per cell, so the bytes of a scene change only when the world or the light
    /// does — which the scene's own [`crate::Scene::build`] id states. The buffers therefore
    /// live as long as the pipeline and are re-written only on a new build, by doubling when
    /// they must grow: a frame that walks uploads nothing, and allocates nothing.
    /// **The corner buffers, in two pairs.** One pair is the scene being drawn; the other is the
    /// scene being written, a slice a frame (`crate::work::FrameWork::upload`). Two pairs and not
    /// one because a sliced upload writes into a buffer the frame is still drawing from, and a
    /// half-written buffer is a torn picture: the front pair is untouched until the upload
    /// completes, and the swap is one index.
    corners: [CornerPair; 2],
    /// Which of the two pairs is drawn this frame.
    front: usize,
    /// The upload in flight, if any: its cursor is the remainder the next frame continues from.
    upload: Option<crate::work::CornerUpload>,
    /// Bytes the next `write_corners` may send. `u64::MAX` for a caller that has not stated one
    /// (an offscreen render, a test), which is the whole scene in one frame — the old behaviour.
    upload_budget: u64,
    /// **Record uploads made and the bytes they sent**, and how many of them grew a buffer. The
    /// growth count is the number that separates a transfer from an allocation: a frame that grows
    /// pays `create_buffer` and a bind-group rebuild, and that is a different cost from the copy.
    quad_uploads: u64,
    quad_bytes: u64,
    quad_grows: u64,
    /// **The record path's own split**: the encode and the transfer of the last upload, in
    /// microseconds, and how many records were patched rather than re-encoded. A scene whose
    /// rectangles are the same rectangles with their origin indices moved is **patched** — four
    /// bytes a record — instead of being serialised again, which is the cost this pass removed.
    quad_serialise_us: f64,
    quad_write_us: f64,
    quad_patched: u64,
    quad_encoded: u64,
    /// **The bytes the record buffer last took**, kept across frames so a renumbered scene can be
    /// patched in place rather than serialised from scratch.
    quad_staging: Vec<u8>,
    /// **Corner-buffer allocations made since construction.** Both pairs are allocated once, at
    /// construction, from [`CORNER_VERTICES`]/[`CORNER_INDICES`]; this counter is how a test says
    /// so, and a frame that allocates is a frame that fails it.
    corner_allocations: u64,
    /// **The last corner buffer the device refused to hold**, and how many distinct refusals have
    /// happened. A refusal is named rather than retried into a `create_buffer` the device answers
    /// with an invalid buffer: the pair keeps the bytes it already had and the frame draws them.
    corner_refusal: Option<CornerRefusal>,
    corner_refusals: u64,
    /// **What the last frame's two uploads cost**, in milliseconds: the record path and the corner
    /// path, measured at the call rather than inferred from the draw. The frame's report quotes
    /// them, because "draw" is two uploads and a pass and a budget can only own what it names.
    quads_us: f64,
    corners_us: f64,
    /// **The corner path's own ledger**, and the whole of the report this change owes: how many
    /// uploads it made, how many bytes they were, and how many frames sent nothing — with the
    /// bytes those frames would have sent under the old per-frame upload. Diagnostic only; it
    /// costs four adds on a frame that uploads and four on one that does not.
    corner_traffic: CornerTraffic,
    /// The world's bind group layout, kept so a grown quad buffer can be
    /// re-bound without reconstructing the pipeline.
    world_layout: wgpu::BindGroupLayout,
    /// **The beam's own shadow** — two depth cascades drawn from the light every frame, so a
    /// shadow follows the Sun and Moon and an edit reaches it with the mesh rather than with a
    /// measured cell. See `sun_shadow`.
    shadow: crate::sun_shadow::SunShadow,
    /// **The streamed light horizon** — occupancy stamped from the resident octrees, marched by
    /// the fragment toward the frame's beam and the sky. See [`crate::horizon`].
    horizon: crate::horizon::HorizonStream,
    /// The beam the frame was last given: `(toward the light xyz, intensity)`, straight from
    /// [`WorldPipeline::write_rig`]. The shadow camera and the fragment read the same light.
    sun: Option<[f32; 4]>,
    /// **Whether the frame draws the beam's shadow.** Off until the host asks: a measurement
    /// frame's reference is the model without a shadow pass, and turning it on is the act that
    /// says the frame is a picture rather than a probe. The game asks once, at construction.
    shadows: bool,
    /// How far from the eye the far cascade reaches, metres. Exposed rather than fixed because
    /// it is the host's streamed horizon: a window that reaches farther raises this with it, and
    /// past it the beam is drawn unshadowed but current rather than held at a stale cell.
    shadow_range: f32,
    /// **How many samples a pixel of this pipeline's target has** — the world pass's
    /// multisampling state, held so a caller can build a target the draws agree with
    /// instead of guessing ([`WorldPipeline::samples`]).
    ///
    /// Every pipeline in this module is created at this count, so all six
    /// (world, quads, clouds, wire, bodies, sky) land on the same attachments. It is a
    /// **construction value**: a frame's pass must be handed attachments with this
    /// count, and — for a count above one — the single-sampled target the pass resolves
    /// into ([`WorldPipeline::draw`]'s `resolve`). What the count *is* for a given
    /// colour format is [`crate::world_samples`]'s answer, not a constant here.
    samples: u32,
}

/// **How a frame is drawn, as opposed to what it is drawn from.** The two settings a caller
/// chooses: which of the two views, and the eye's own angular pixel size — a property of the
/// target rather than of the world, which is why the scene cannot state it.
///
/// They travel as one value because they are both *the frame's own settings*, and because the two
/// wrappers below differ in exactly the first field: state them positionally and a reader has to
/// check a literal against the parameter list to see which view is being asked for.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct DrawMode {
    /// `F4`: outline the merge's records and withhold the world's own materials.
    pub wireframe: bool,
    /// One pixel's angular size, in radians — [`crate::pattern::pixel_angle_rad`]'s value.
    pub pixel_angle_rad: f32,
}

impl WorldPipeline {
    /// `format` is the **colour target's** format, so a window passes its
    /// surface format and the offscreen renderer passes
    /// [`crate::TARGET_FORMAT`]. The depth format is fixed at
    /// [`crate::DEPTH_FORMAT`] for both, because a depth test that differed
    /// between the two would be exactly the kind of divergence this module
    /// exists to prevent.
    /// **`samples` is the multisample count of the target the frames are drawn into**
    /// ([`crate::world_samples`] answers it for a format and an adapter), and it must be
    /// the count the caller's own attachments were created with: one, or the count a
    /// frame's [`WorldPipeline::draw`] resolves from. `1` is no multisampling at all —
    /// the frame is drawn exactly as it was before the count existed.
    ///
    /// **Why it is the caller's number and not this module's.** A pipeline validates
    /// its sample count against the *format*, which cannot say what the attachment will
    /// be, so the two have to be decided together by whoever owns the target. The
    /// window's `sRGB` surface supports four samples on every WebGPU adapter; the
    /// measurement target is `Rgba32Float`, whose four samples come from the adapter's
    /// own format features rather than the guaranteed table — so the answer genuinely
    /// differs per target and per adapter, and one constant here would be wrong on one
    /// of them.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        materials: &[MaterialParams],
        rig: &GpuRig,
        samples: u32,
    ) -> Result<Self, ClientError> {
        debug_assert!(
            matches!(samples, 1 | 4),
            "a world pass is drawn at one sample or at four, never at {samples}"
        );
        if materials.is_empty() {
            return Err(ClientError::UnknownMaterial { id: 0, rows: 0 });
        }

        // **The six entries the pass has always had, and the beam's own four.** The shadow
        // textures and sampler are the only additions; a world with no light binds them and the
        // shader's flag answers "no shadow", so the frame is the frame it was.
        let mut entries = vec![
                uniform_entry(0, wgpu::ShaderStages::FRAGMENT),
                // The camera is read by **both** stages now: the vertex projects with
                // `vp`, and the fragment reads the eye's fractional offset to address
                // the pattern (C-161). A layout that hid it from the fragment would
                // be a pipeline wgpu refuses, which is how this line was found.
                uniform_entry(1, wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT),
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // The quad format's two storage buffers: the rectangles, and one
                // eye-relative origin per chunk they name. **Vertex-visible
                // only** — the fragment never reads either, because it addresses
                // the pattern from its own interpolated position (C-161).
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // **The sky's own transport, bound to the world pass** (C-177). A fluid
                // surface *is* the sky, seen off a dielectric boundary: the reflection is most
                // of what water looks like, and until this binding existed the world pass could
                // not see the sky `sky.wgsl` was already drawing. Both passes bind **the same
                // buffer at the same binding number (5)** — the model's rows, which
                // `atmosphere.wgsl` declares once — and both evaluate them with that file's
                // code, so there is one sky, one integral and one exposure rather than an
                // agreement between two.
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
        ];
        entries.extend(crate::sun_shadow::entries());
        entries.extend(crate::horizon::HorizonStream::layout_entries());
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("world"),
            entries: &entries,
        });

        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shading"),
            source: wgpu::ShaderSource::Wgsl(pass_source(include_str!("shading.wgsl")).into()),
        });

        let attributes = Vertex::attributes();
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("world"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("world"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: Vertex::STRIDE,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &attributes,
                })],
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                // **Culling is off, and that is a measured decision rather than an
                // omission** (see `renderer.rs`'s "Why culling is off"). The
                // mesher's quads *are* wound outwards (`mesh::winding_flips`), so
                // culling would be correct — but "correct" rests on a winding
                // convention in another crate, and a frame that comes out empty
                // because it drifted is a confusing failure. Enabling it is what
                // an earlier attempt did, and the probes the verification renders
                // (`Scene::probe`) are wound to face the eye, so a `Back` cull drew
                // them away and four GPU tests read "the probe must actually cover
                // the centre: 0.0". Culling goes back on with a measurement of the
                // fill it saves, not before.
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: crate::DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState { count: samples, ..Default::default() },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            multiview_mask: None,
            cache: None,
        });

        // ---- the quad format's own pipeline --------------------------------
        // **The same fragment shader, a vertex stage that pulls.** `vs_quad` reads
        // the record at `vertex_index / 6` and expands it, so a rectangle costs 24
        // bytes of upload and no CPU corner at all. Its primitive, depth and colour
        // states are the world pipeline's, verbatim — so a frame drawn through this
        // path can differ from one drawn through the other only in what the two
        // vertex stages compute, which is what makes those two comparable.
        let quad_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("world-quads"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_quad"),
                // **No vertex buffers.** The geometry arrives from the storage
                // buffers above, addressed by `vertex_index`.
                buffers: &[],
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: crate::DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState { count: samples, ..Default::default() },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            multiview_mask: None,
            cache: None,
        });

        // ---- the cloud's pipeline state (C-179) -----------------------------
        // **One shader, two states.** The module, the entry points and the layout are the
        // world's, so the cloud cannot drift from the look; what is added is the two things a
        // veil needs. `ALPHA_BLENDING` is the compositing of a translucent box over the sky it
        // stands in, and the source alpha is the veil `fs_main` writes. Depth is tested (`Less`)
        // so terrain in front still hides a cloud, and **not written**, so a second cell of the
        // same cloud behind the first is still drawn.
        // **Blending needs a target that can blend, and a measurement target cannot.** WebGPU
        // makes blending a property of the *format*, and `Rgba32Float` — this client's own
        // verification target, chosen so a disagreement between `shading.wgsl` and `npr::shade`
        // cannot hide inside a rounding step — is not blendable. So the veil is composited on a
        // **display** and stored on a **measurement**, which is the same line `dither` and
        // `tonemap` already draw below. On the measurement target the fragment still writes its
        // alpha, so the veil is *in* the frame as a number `1 − transmittance`; what is absent
        // there is only the compositing of it over the sky, and that is named rather than
        // pretended.
        let veil_blend = if format.is_srgb() {
            Some(wgpu::BlendState::ALPHA_BLENDING)
        } else {
            None
        };
        let cloud_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("world-clouds"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_quad"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: crate::DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState { count: samples, ..Default::default() },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: veil_blend,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            multiview_mask: None,
            cache: None,
        });

        // ---- the debug wireframe's own pipeline (F4) -------------------------
        // **The one debug view of the geometry**: the merge's rectangles, outlined. It is the same
        // module, the same layout and the same record buffers the shaded draw reads — only the
        // entry points and the primitive differ — so the outline is the rectangle rather than a
        // second statement of where the rectangle is.
        //
        // Depth is tested and **not written**: the world's own draws are skipped while this is on,
        // so there is nothing to occlude, and a line that wrote depth would hide the lines behind
        // it inside the same mesh being read.
        let wire_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("world-wire"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_wire"),
                // **No vertex buffers**, exactly as the quad pipeline: `vs_wire` addresses the
                // record at `vertex_index / 8` out of the storage buffers above.
                buffers: &[],
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: crate::DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState { count: samples, ..Default::default() },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_wire"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            multiview_mask: None,
            cache: None,
        });

        let view = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("view"),
            // The camera and the pattern: `vp` (64 bytes), the eye's fractional
            // metre offset and its own voxel (the two halves a fragment adds back
            // to place itself in the world), the lattice's identity and the two
            // rates, one pixel's angular size, and the dome table's grid with the
            // water boundary's own reflectance (C-177 — the fluid reflection reads
            // them). 160 is the `View` struct in `shading.wgsl`, and a test holds the
            // two together.
            size: 160,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let materials_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("materials"),
            size: MaterialParams::SIZE * materials.len() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&materials_buffer, 0, &flatten_materials(materials));

        // ---- the astronomical body's own pipeline (C-184) -------------------
        // **One record, one body, and the phase out of the pixels.** `vs_body` plots a body's record at
        // `camera::BODY_PLOT_M` — inside the plane's own reach, because the depth buffer cannot hold a
        // body's own 1.87e10 m — and `fs_body` gives every pixel the normal of the sphere at that pixel,
        // so a quarter moon is drawn as a lit part of a disc rather than as one face at one incidence.
        //
        // Depth is **written**, unlike the veil's: a body is opaque, and the cloud this pass draws after
        // it has to be tested against it.
        let body_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("world-bodies"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_body"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: crate::DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                // **`LessEqual`, and it is what draws a body at all.** A body is plotted on the far plane,
                // where `vs_body` writes the plane's own depth by hand, and this pass clears depth to
                // `1.0`: `Less` would discard a body exactly where it belongs. Equal is the plane, so a
                // pixel the frame has drawn nothing on takes the body and a hill the terrain wrote a
                // nearer depth on does not. A cloud, drawn after, is nearer than any body and so still
                // veils it.
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState { count: samples, ..Default::default() },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_body"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    // **Written through, not blended**, and the reason is the target rather than a
                    // preference: `Rgba32Float` — the format the offscreen frames are measured in — is not
                    // a blendable format at all, so an additive pipeline would be one wgpu refuses to
                    // create. The alternative, blending only on the window's `sRGB` surface, would make the
                    // composition of a frame depend on where it is being shown: the measurement and the
                    // picture would be two sums, and every test in `renderer.rs` compares against the
                    // measurement. So `fs_body` takes the sum itself — the air this ray carried, from the
                    // same `sky_radiance_rgb` the sky pass uses, plus the body's own radiance through its
                    // incidence — which is why an unlit disc is the sky it stands in and not a black orb.
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            multiview_mask: None,
            cache: None,
        });

        let rig_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rig"),
            size: GpuRig::SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&rig_buffer, 0, &rig.bytes());

        // The quad format's two storage buffers, at a page each to start: a scene
        // with no geometry still binds something valid, and `write_quads` grows
        // them — by doubling — only when a window grows past them.
        let quad_records = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("quad-records"),
            size: QUAD_START_BYTES,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let quad_origins = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("quad-origins"),
            size: QUAD_START_BYTES,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        // **The sky's transport, created before either pass that reads it.** Both the sky pass
        // (which draws it) and the world pass (which reflects it off a fluid, C-177) bind this
        // one buffer, so it is made here rather than inside either pass's own section: a model
        // created for one of them and copied for the other would be two skies.
        //
        // Zeroed, and **that is the whole of what a caller that never states a sky gets**. The
        // rows are the transport's arithmetic over nothing — the black this pass drew before a sky
        // existed — and the record beside them is what says whether a host meant it.
        let sky_model = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sky-model"),
            size: SKY_MODEL_BYTES as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&sky_model, 0, &ZERO_SKY_MODEL);

        // **The beam's own pass**, created once beside the pipeline it shadows. It owns two
        // depth maps and their matrices; the world's bind group below names them.
        let shadow = crate::sun_shadow::SunShadow::new(device);
        // **The light horizon**, created beside it: the streamed occupancy cube the fragment
        // marches toward the beam and the sky, and the small view that puts it in the eye's own
        // frame. A cube with nothing stamped is empty air, which is the frame the pass drew
        // before the horizon existed.
        let horizon = crate::horizon::HorizonStream::new(device);
        // `write_quads` re-binds through the same function when a buffer grows,
        // so the entries exist once.
        let bind_group = world_bind_group(
            device,
            &bind_group_layout,
            &rig_buffer,
            &view,
            &materials_buffer,
            &quad_records,
            &quad_origins,
            &sky_model,
            &shadow,
            &horizon,
        );
        // The cloud's own pair, bound through the same layout: the shader reads a record and an
        // origin from whichever pair its draw's bind group names, so the sky needs no format of
        // its own and a terrain change cannot move it.
        let cloud_records = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cloud-records"),
            size: QUAD_START_BYTES,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let cloud_origins = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cloud-origins"),
            size: QUAD_START_BYTES,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let cloud_bind_group = world_bind_group(
            device,
            &bind_group_layout,
            &rig_buffer,
            &view,
            &materials_buffer,
            &cloud_records,
            &cloud_origins,
            &sky_model,
            &shadow,
            &horizon,
        );

        // ---- the sky pass -------------------------------------------------
        // Its own bind group and its own pipeline, sharing the render pass and the
        // depth attachment with the world so geometry lands on top of the sky without
        // a second pass or a resolve.
        let sky_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sky"),
            source: wgpu::ShaderSource::Wgsl(pass_source(include_str!("sky.wgsl")).into()),
        });
        let sky_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sky"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let sky_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("sky"),
                bind_group_layouts: &[Some(&sky_layout)],
                immediate_size: 0,
            });
        let sky_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sky"),
            layout: Some(&sky_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &sky_module,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: crate::DEPTH_FORMAT,
                // **Drawn first, so it must not claim depth.** It fills every pixel
                // the world does not, and the world then draws over it with `Less`
                // against a depth buffer nothing has written — so a sky that wrote
                // depth here would occlude the terrain behind it.
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState { count: samples, ..Default::default() },
            fragment: Some(wgpu::FragmentState {
                module: &sky_module,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            multiview_mask: None,
            cache: None,
        });

        let sky_view = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sky-view"),
            // **Thirty-two `f32`, and the count is the shader's.** The first eight are
            // the `SkyView` fields `sky.wgsl` was built with; the ninth is its dither
            // flag and the tenth its display flag, the same two the world pass
            // carries (C-166, C-167); the eleventh and twelfth are declared so the
            // first block is a whole number of `vec4`s rather than whatever `naga`
            // would round it up to; the last sixteen are two bodies, each a `vec3<f32>`
            // centre and an `f32` extent followed by a `vec3<f32>` radiance and an `f32`
            // that closes the `vec4` — the Sun (C-176) and the Moon, which the shader
            // lights itself from the third of its own `vec4`s, the direction toward the
            // Sun. `write_camera` writes exactly these bytes and asserts the length, so
            // the shader and this upload cannot drift apart.
            size: 128,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        // **Zeroed, not left undefined.** A caller that never writes a sky gets a
        // black one, which is what the pass did before it existed — so adding the sky
        // cannot change a frame whose owner has not asked for a sky yet.
        queue.write_buffer(&sky_view, 0, &[0u8; 128]);
        let sky_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sky"),
            layout: &sky_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: sky_view.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 5, resource: sky_model.as_entire_binding() },
            ],
        });

        // **The target's own encoding decides, not a preference.** An `sRGB`
        // surface rounds to 256 levels per channel on the way out, so a smooth
        // gradient arrives as bands and a sub-step offset removes them by moving the
        // rounding of neighbouring pixels apart. A float target rounds nothing, so
        // there is no band to remove and the offset would be pure noise laid over an
        // exact measurement — which is why this is `0.0` for the offscreen tests and
        // the whole pass is a no-op there rather than something they must tolerate.
        let dither = if format.is_srgb() { 1.0 } else { 0.0 };
        // **One tree, whole, until a frame says otherwise.** A pipeline that has never been handed
        // a fade draws exactly what it drew before the fade existed: the newer side at its own
        // number, the older side — which a one-tree scene has none of — at nothing.
        let cloud_fade = 1.0;
        // **The shaded picture**, until a caller asks for one of the layers.
        let debug_layer = 0;
        // And the same target is also the one that cannot hold more than white, so
        // it is the one that needs the display transform. An 8-bit `sRGB` surface is
        // a display; `Rgba32Float` is a measurement, and a measurement that had been
        // compressed by a display curve would no longer be the number the model
        // computed — which is what every test in `renderer.rs` compares against.
        let tonemap = if format.is_srgb() { 1.0 } else { 0.0 };

        Ok(WorldPipeline {
            device: device.clone(),
            queue: queue.clone(),
            pipeline,
            bind_group,
            view,
            rig: rig_buffer,
            rows: materials.len() as u32,
            lattice: crate::shading::world_lattice_uniform(),
            rates: crate::shading::rates_uniform(),
            dither,
            tonemap,
            cloud_fade,
            debug_layer,
            sky_pipeline,
            sky_bind_group,
            sky_view,
            sky_model,
            sky_rows: std::sync::atomic::AtomicUsize::new(SKY_NOT_STATED),
            body_pipeline,
            quad_pipeline,
            cloud_pipeline,
            wire_pipeline,
            quad_records,
            quad_origins,
            quad_record_capacity: QUAD_START_BYTES,
            quad_records_filled: false,
            quad_origin_capacity: QUAD_START_BYTES,
            cloud_records,
            cloud_origins,
            cloud_record_capacity: QUAD_START_BYTES,
            cloud_origin_capacity: QUAD_START_BYTES,
            cloud_bind_group,
            cloud_record_last: Vec::new(),
            cloud_origin_last: Vec::new(),
            cloud_origin_eye: None,
            quad_record_last: Vec::new(),
            body_record_last: Vec::new(),
            quad_origin_last: Vec::new(),
            quad_origin_eye: None,
            corners: [CornerPair::default(), CornerPair::default()],
            front: 0,
            upload: None,
            upload_budget: u64::MAX,
            quad_uploads: 0,
            quad_bytes: 0,
            quad_grows: 0,
            quad_serialise_us: 0.0,
            quad_write_us: 0.0,
            quad_patched: 0,
            quad_encoded: 0,
            quad_staging: Vec::new(),
            corner_allocations: 0,
            corner_refusal: None,
            corner_refusals: 0,
            quads_us: 0.0,
            corners_us: 0.0,
            corner_traffic: CornerTraffic::default(),
            materials: materials_buffer,
            world_layout: bind_group_layout,
            shadow,
            horizon,
            // **The construction rig is the first frame's beam**, so a renderer built with a
            // light already in hand casts from it on the first draw rather than waiting for a
            // second `write_rig` nobody makes.
            sun: (rig.count[0] > 0).then_some(rig.lights[0].direction),
            shadows: false,
            shadow_range: crate::sun_shadow::FAR_RANGE_M,
            samples,
        })
        .map(|mut pipeline| {
            // **The corner pairs are sized before the first frame draws.** A pair grown when a
            // scene arrived allocated 16 MB inside a live streaming frame — 77.2 ms, `draw` 43.3 —
            // which is the one cost PATCH-053's fix traded seven growths for. The capacity is the
            // declared window's own corner count with its headroom, so every build of this world
            // fits and no frame allocates; the bytes are stated in the report beside it.
            for i in 0..2 {
                match pipeline.corners[i].ensure_vertices(device, CORNER_VERTEX_BYTES) {
                    Ok(true) => pipeline.corner_allocations += 1,
                    Ok(false) => {}
                    Err(refusal) => pipeline.note_corner_refusal(refusal),
                }
                match pipeline.corners[i].ensure_indices(device, CORNER_INDEX_BYTES) {
                    Ok(true) => pipeline.corner_allocations += 1,
                    Ok(false) => {}
                    Err(refusal) => pipeline.note_corner_refusal(refusal),
                }
            }
            pipeline
        })
    }

    /// **Replace the lighting rig for the frames that follow.**
    ///
    /// `None` when the rig holds more lights than the uniform carries, which is
    /// the same refusal [`crate::shading::GpuRig::from_rig`] makes and for the
    /// same reason: a truncated rig would drop a light and change every pixel in a
    /// way nothing downstream could see. **The old rig is left in place** on
    /// refusal rather than zeroed, so a caller that ignored the `None` draws the
    /// last rig it managed to upload rather than a black frame.
    pub fn write_rig(&mut self, rig: &GpuRig) -> Option<()> {
        if rig.count[0] as usize > crate::shading::RIG_CAPACITY {
            return None;
        }
        self.queue.write_buffer(&self.rig, 0, &rig.bytes());
        // **The same light the pass above draws with.** The shadow camera is derived from this
        // direction and the fragment's choice of cascade from the same frame's matrix, so a Sun
        // that moved between the light and the shadow cannot be two suns.
        self.sun = (rig.count[0] > 0).then_some(rig.lights[0].direction);
        Some(())
    }

    /// **Ask the frame to draw the beam's shadow** — the Sun's or the Moon's, from the light
    /// [`WorldPipeline::write_rig`] last stated. Off by default, because a measurement frame's
    /// reference model has no shadow term and a pass that appeared in every offscreen test would
    /// be a term the reference never claimed. The game turns it on once; a witness turns it on
    /// for the frame it is about.
    pub fn set_shadows(&mut self, on: bool) {
        self.shadows = on;
    }

    /// Whether this pipeline draws shadows, for a report that has to say which frame it is.
    pub fn shadows(&self) -> bool {
        self.shadows
    }

    /// **How far the cast shadow reaches**, metres — set it from the horizon the host streams.
    /// Inside the range the shadow is the map's; past it the beam is unshadowed and current, so
    /// widening a horizon never brings back a cell the light has left behind. A range narrower
    /// than two near cascades is raised to that, which is the smallest pair the split describes.
    pub fn set_shadow_range(&mut self, metres: f32) {
        self.shadow_range = metres.max(crate::sun_shadow::NEAR_RANGE_M * 2.0);
    }

    /// The shadow range this pipeline draws, metres.
    pub fn shadow_range(&self) -> f32 {
        self.shadow_range
    }

    /// **Move the light horizon to the eye's chunk**, clearing it for the chunks that follow.
    ///
    /// Returns whether the cube moved. A host calls this once a frame; a chunk step clears the
    /// cube and the next frames stamp the chunks the streamer holds, which is what makes the
    /// field *stream* rather than wait for a whole window's worth of trees.
    pub fn horizon_recenter(&mut self, eye_m: [f64; 3]) -> bool {
        self.horizon.recenter(eye_m)
    }

    /// **Forget every chunk the resident set no longer holds**, so an evicted chunk's occluder
    /// does not outlive the geometry it was stamped from.
    pub fn horizon_retain(&mut self, resident: &BTreeSet<(i32, i32, i32)>) -> bool {
        self.horizon.retain(resident)
    }

    /// Whether a chunk's cube still needs this version of its tree
    /// (`voxel_work::Streamer::tree_version`).
    pub fn horizon_wants(&self, pos: voxel_model::ChunkPos, version: u64) -> bool {
        self.horizon.wants(pos, version)
    }

    /// **Whether a chunk is in the fine cube** — the one band whose missing cells a frame may
    /// not draw through. A chunk merely in the coarse band is lit at the coarse rung, not refused.
    pub fn horizon_covers(&self, pos: voxel_model::ChunkPos) -> bool {
        self.horizon.covers(pos)
    }

    /// What the fine band holds at a world point: `None` outside the cube or where the slot is
    /// unstamped, `Some(false)` for measured air, `Some(true)` for matter. The readout a frame
    /// uses to say which band answered for a pixel.
    pub fn horizon_solid(&self, point_m: [f64; 3]) -> Option<bool> {
        self.horizon.solid_at(point_m)
    }

    /// The fine band's stamped-slot mask, as the shader reads it.
    pub fn horizon_coverage(&self) -> u32 {
        self.horizon.coverage()
    }

    /// **Stamp one resident chunk's subvoxel octree into the light horizon.**
    ///
    /// Returns the cells the chunk marked. A chunk the cube does not cover, or one whose version
    /// is already stamped, costs nothing and marks nothing.
    pub fn horizon_stamp(
        &mut self,
        pos: voxel_model::ChunkPos,
        tree: &voxel_mesh::SparseVoxelOctree,
        version: u64,
    ) -> usize {
        self.horizon.stamp(pos, tree, version)
    }

    /// The world-voxel corner the cube is anchored on, for a report that has to say what the
    /// frame's horizon covers.
    pub fn horizon_anchor(&self) -> [i32; 3] {
        self.horizon.anchor()
    }



    /// How many rows the material table has. A scene naming a material beyond
    /// this is not shaded at all — the shader's own guard, not a wrap.
    pub fn rows(&self) -> u32 {
        self.rows
    }

    /// **The sample count this pipeline's targets must have** — one, or the count a
    /// frame resolves from. A caller building its own attachment asks rather than
    /// repeating the number, so a pipeline and the target it draws into cannot
    /// disagree about it.
    pub fn samples(&self) -> u32 {
        self.samples
    }

    /// **What the corner path has uploaded, and what it spared** — [`CornerTraffic`].
    pub fn corner_traffic(&self) -> CornerTraffic {
        self.corner_traffic
    }

    /// **Turn the target-quantisation dither on or off for the frames that follow.**
    ///
    /// `0.0` is off; `1.0` spreads **one whole step of an 8-bit channel** across the
    /// pixel by an ordered threshold, so the offset is at most half a step either
    /// way. Set from the colour format at construction; this exists because the
    /// verification target is `Rgba32Float`, which quantises nothing, so the dither
    /// has to be turned on deliberately to be measured there at all.
    ///
    /// An amount outside `0..=1` is clamped rather than refused: this is a dither
    /// amplitude in steps of the target's own step, so `2.0` has no meaning to
    /// refuse *to*, and a caller who passes it wants more dither, not an error.
    pub fn set_dither(&mut self, amount: f32) {
        self.dither = amount.clamp(0.0, 1.0);
    }

    /// **The cloud's cross-fade weight** (C-183): `1.0` is the newer tree whole — the value every
    /// frame with one tree in hand is drawn at — and `0.0` is the older one whole, so the sky's
    /// entrance from clear air and the dissolve between two derivations are one number.
    ///
    /// A caller sets it from the world's ticks, not from a stopwatch: `cloud_cubes::fade_weight`
    /// reads the tick a tree was derived from against the tick being drawn, so the dissolve is
    /// phase-locked to the cadence that produced it and a dropped frame cannot change the sky.
    ///
    /// An amount outside `0..=1` is clamped, and a non-finite amount is read as `1.0` — the newer
    /// tree, which is the honest answer for a caller that could not measure its own fade, where a
    /// NaN would reach every cloud fragment's alpha and make the sky disappear.
    pub fn set_cloud_fade(&mut self, weight: f32) {
        self.cloud_fade = if weight.is_finite() {
            weight.clamp(0.0, 1.0)
        } else {
            1.0
        };
    }

    /// The cross-fade weight a frame would be drawn with, so a test can state its own input.
    pub fn cloud_fade(&self) -> f32 {
        self.cloud_fade
    }

    /// **Draw one of the material's layers instead of the shaded picture.**
    ///
    /// `0` is the shaded picture and every layer number is `shading.wgsl`'s own: `1` the stretch
    /// axis the stencil shifts along, `2` what the corner states of its own, `3` the ladder (its
    /// own share as brightness, the dominant rung as hue), `4` the cell address the pattern is
    /// read at. The geometry, the light and the material table are unchanged, so the two pictures
    /// are of the same fragments; an out-of-range number is clamped to the shaded picture rather
    /// than drawn as a layer this file does not have.
    pub fn set_debug_layer(&mut self, layer: u32) {
        self.debug_layer = if layer <= 4 { layer } else { 0 };
    }

    /// The layer the frames that follow are drawn as.
    pub fn debug_layer(&self) -> u32 {
        self.debug_layer
    }

    /// **Turn the display transform on or off for the frames that follow.**
    ///
    /// `0.0` is off — the frame is the scene-referred value the model computed, which
    /// is what a measurement must be. `1.0` compresses it into a colour a display can
    /// present, because its target cannot hold more than white.
    ///
    /// Set from the colour format at construction. The setter exists for the same
    /// reason [`WorldPipeline::set_dither`]'s does: the verification target does not
    /// quantise *and* does not clip, so neither concern can be measured there without
    /// asking for it.
    pub fn set_tonemap(&mut self, amount: f32) {
        self.tonemap = amount.clamp(0.0, 1.0);
    }

    /// Whether a frame would be display-transformed, so a test can state what it
    /// expects the target's format to have decided.
    pub fn tonemap(&self) -> f32 {
        self.tonemap
    }

    /// The dither amount a frame would be drawn with, so a test can state what it
    /// expects the target's format to have decided.
    pub fn dither(&self) -> f32 {
        self.dither
    }

    /// The camera as the shader reads it. Kept public so a caller can upload
    /// once and draw twice (a shadow pass, say) without recomputing.
    ///
    /// `pixel_angle_rad` is one pixel's angular size ([`crate::pattern::pixel_angle_rad`],
    /// from the camera's own `fov_y_rad` and the target's pixel height). It is the
    /// ladder's footprint scale, and it is a **target** property rather than a world
    /// one — a fragment cannot derive it, so it is passed in and uploaded with the
    /// camera.
    pub fn write_camera(&self, camera: &Camera, pixel_angle_rad: f32) {
        let mut bytes = Vec::with_capacity(160);
        for column in camera.view_projection() {
            for v in column {
                bytes.extend_from_slice(&voxel_model::endian::le_bytes(v));
            }
        }
        let phase = crate::pattern::eye_phase(camera.position_m);
        for v in [phase[0], phase[1], phase[2], 0.0f32] {
            bytes.extend_from_slice(&voxel_model::endian::le_bytes(v));
        }
        let voxel = crate::pattern::eye_voxel(camera.position_m);
        for v in [voxel[0], voxel[1], voxel[2], 0i32] {
            bytes.extend_from_slice(&voxel_model::endian::le_bytes(v));
        }
        for v in self.lattice {
            bytes.extend_from_slice(&voxel_model::endian::le_bytes(v));
        }
        for v in self.rates {
            bytes.extend_from_slice(&voxel_model::endian::le_bytes(v));
        }
        // `screen.xy`: one pixel's angular size, and the dither flag the fragment
        // pass reads before it writes (C-163, C-166). **`screen.w` is now claimed** by the
        // cloud's cross-fade weight (C-183) — the reserved slot the struct left open for exactly
        // this, so the layout and the struct's size do not move — and the display flag is `z`.
        for v in [pixel_angle_rad, self.dither, self.tonemap, self.cloud_fade] {
            bytes.extend_from_slice(&voxel_model::endian::le_bytes(v));
        }
        // `sky_grid`: the water boundary's own reflectance and the cloud's albedo, which
        // the world's fluid reflection and a cloud's own row read (C-177, C-179). Sent
        // rather than typed into `shading.wgsl` for the reason `lattice` and `rates` are:
        // the reflectance is `voxel_appearance::fluid`'s and the albedo is
        // `cloud_field`'s, and a shader holding a second copy of either is a shader that
        // can disagree with the model it is lighting.
        // **`w` is the cloud's own albedo** (C-179), the fourth number of this group and the
        // last slot `View` had left. A cloud has no material row — it has no surface, so there
        // is no row to have — and the one number its shading needs beyond the rig is how much
        // of the light a water droplet gives back. `cloud_field::CLOUD_ALBEDO` is that number,
        // a real figure for water droplets in the visible, and it is **sent** for the reason
        // every other constant here is: a shader holding a second copy of it could disagree.
        for v in [
            // **`x` is how far a drawn astronomical body is plotted** (C-184) — `camera::BODY_PLOT_M`,
            // a distance *inside* the projection's own plane rather than the plane's own reach. **It is
            // not where the body stands**: the record carries the world's own engine distance (4.805e7 m
            // for the Moon, 1.87e10 m for the Sun), and `vs_body` plots that direction here and scales
            // the half-extent by the same ratio, which leaves the subtended angle exactly the world's own.
            //
            // **The plot is inside the plane because a body on it is discarded.** Depth at exactly
            // `FAR_M` is `1.0` in `f32`, the pass clears to `1.0` and tests `Less`, and a body out there
            // fails that test — which is why no frame had a Moon in it when this read `FAR_M`. The margin
            // and its own reasoning are `camera::BODY_PLOT_M`'s doc.
            //
            // It is sent rather than typed into the shader for the reason every constant here is: a
            // shader holding a second copy can disagree with the projection it is drawn through.
            //
            // **The dome's two grid numbers used to be here**, and the second is still a reserve.
            crate::camera::BODY_PLOT_M as f32,
            // **`y` is the layer view's channel now** (`WorldPipeline::set_debug_layer`): the
            // dome's elevation count used to live here and the dome is gone, so the slot was a
            // reserve until the material's own layers needed one. Zero is the shaded picture.
            self.debug_layer as f32,
            voxel_appearance::fluid::WATER_F0 as f32,
            crate::cloud_field::CLOUD_ALBEDO as f32,
        ] {
            bytes.extend_from_slice(&voxel_model::endian::le_bytes(v));
        }
        debug_assert_eq!(bytes.len(), 160, "View is forty f32");
        self.queue.write_buffer(&self.view, 0, &bytes);

        // The sky's geometry, uploaded with the camera rather than separately: it is
        // the same two numbers, and a caller that moved the camera and not the sky
        // would get a sky belonging to a view that is no longer on screen.
        let mut sky_bytes = Vec::with_capacity(32);
        for v in [
            camera.pitch_rad as f32,
            (camera.fov_y_rad / 2.0).tan() as f32,
            camera.yaw_rad as f32,
            camera.aspect as f32,
            // The water boundary's reflectance: written for the world pass's own fluid
            // reflection, and declared here so the two structs stay the same size and
            // layout — the reason that field's own doc gives.
            voxel_appearance::fluid::WATER_F0 as f32,
            // The sky's own dither flag, the same value the world pass carries in
            // `View::screen.y`: the sky is the largest smooth gradient on the
            // screen, so it is where the target's banding is loudest.
            self.dither,
            // And its display flag, `View::screen.z` — the sky is also the brightest
            // thing on the screen, and the one most likely to be more than white.
            self.tonemap,
            // **The five the struct declares to close out its first three `vec4`s.** The
            // first two used to be the dome's grid — its azimuth and elevation counts — and
            // the model carries its own band count now, so nothing reads any of the five:
            // they exist so the shader's struct and this upload are the same twelve `f32`
            // with no padding for either side to work out.
            0.0f32,
            0.0f32,
            0.0f32,
            0.0f32,
            0.0f32,
        ] {
            sky_bytes.extend_from_slice(&voxel_model::endian::le_bytes(v));
        }
        // The uniform is exactly the shader's `SkyView`, so the write is exactly the
        // twelve `f32` the struct declares — the camera and the air, and **no body** (C-184). This is
        // asserted *before* the upload because a short or long write is a `wgpu` bounds refusal at
        // runtime — the one that used to read "32 bytes into a buffer of size 16".
        debug_assert_eq!(sky_bytes.len(), 48, "SkyView is twelve f32");
        self.queue.write_buffer(&self.sky_view, 0, &sky_bytes);
    }

    /// **Upload the sky's transport**: the rows both passes read
    /// ([`crate::skylight::SkyModel`]), with [`crate::skylight::SKY_EXPOSURE`] already folded
    /// into the model's own exposure row.
    ///
    /// The exposure is applied **in the model**, where the CPU assembles the rows, for the same
    /// reason it is applied to the rig at its upload and never inside the solver: the bands stay
    /// physical and only the thing that reaches the swapchain is scaled. The shader then
    /// multiplies the chromaticity by that exposure, which is the one number that makes the sky
    /// this frame draws the sky the light rig was built from.
    ///
    /// **Refused before the write, not after it.** `write_buffer` validates its own range and
    /// panics naming a byte offset; this names the two counts, and a model longer than the buffer
    /// `atmosphere.wgsl` reads is the one way this seam can be short of its layout.
    pub fn write_sky_model(&self, model: &SkyModel) {
        let bytes = model.bytes();
        assert!(
            bytes.len() as u64 <= self.sky_model.size(),
            "the model is {} bytes and the sky's buffer holds {}: the layout atmosphere.wgsl \
             declares is {SKY_MODEL_ROWS} rows of 16 bytes",
            bytes.len(),
            self.sky_model.size()
        );
        self.queue.write_buffer(&self.sky_model, 0, &bytes);
        self.sky_rows.store(model.rows(), std::sync::atomic::Ordering::Relaxed);
    }

    /// **State that this frame has no sky** — the atmosphere refused, say — rather than leaving
    /// the last model standing.
    ///
    /// The rows are zeroed, because the shader reads the buffer and not this record: a stated
    /// absence that left the previous tick's model in place would be that model again. Zero is no
    /// model rather than a short one ([`NO_SKY_ROWS`]), so the frame is the black this pass drew
    /// before a sky existed — and it is **stated**, which is what keeps a host that means it out
    /// of [`WorldPipeline::admit_sky`]'s refusal.
    pub fn write_no_sky_model(&self) {
        self.queue.write_buffer(&self.sky_model, 0, &ZERO_SKY_MODEL);
        self.sky_rows.store(NO_SKY_ROWS, std::sync::atomic::Ordering::Relaxed);
    }

    /// The rows this host has stated, or `None` if it has stated nothing at all.
    fn stated_sky_rows(&self) -> Option<usize> {
        match self.sky_rows.load(std::sync::atomic::Ordering::Relaxed) {
            SKY_NOT_STATED => None,
            rows => Some(rows),
        }
    }

    /// **The one place a frame asks whether there is a sky to read**, and it fails rather than
    /// drawing one that was never stated.
    ///
    /// Both passes that read the rows — the sky pass and the fluid's reflection — do it through
    /// the buffer this record describes, so a host that never called
    /// [`WorldPipeline::write_sky_model`] is caught here, where its own omission is all there is
    /// to see: the shader's `arrayLength` refusal cannot reach it, because the buffer it binds is
    /// the declared length either way.
    fn admit_sky(&self) {
        if let Err(refusal) = admit_sky_rows(self.stated_sky_rows()) {
            panic!("{refusal}");
        }
    }

    /// **The corner format's upload, keyed on the scene's own build, in slices.**
    ///
    /// Corners are chunk-local ([`crate::scene::Vertex`]), so a frame's vertices and
    /// indices are the same bytes while the world and its light are — and the scene's
    /// [`crate::Scene::build`] id is exactly the statement "this scene is that scene".
    /// Walking therefore uploads nothing: the buffers are written when a chunk arrives, is
    /// evicted, is remeshed, or a cell's light is measured, and are handed to the pass untouched
    /// on every frame in between.
    ///
    /// **And a scene that does change is sent `upload_budget` bytes at a time.** A streaming
    /// frame used to send the whole 20 MB — 143 ms under eight busy cores on a frame that
    /// already had a scene to draw. The upload now lives in [`crate::work::CornerUpload`]'s
    /// cursor, the bytes are written into the **back** pair, and the front pair keeps being
    /// drawn until the last slice lands and the two swap. Nothing is staged: each slice is
    /// written out of the scene's own vectors at the offset it belongs at.
    ///
    /// Returns the bytes this frame sent.
    fn write_corners(&mut self, scene: &Scene) -> u64 {
        if scene.indices.is_empty() || scene.vertices.is_empty() {
            return 0;
        }
        if self.corners[self.front].build == scene.build && self.upload.is_none() {
            // **The frame this change is for.** Nothing is written, nothing is allocated, and
            // the bytes that were not sent are counted so the report can state them.
            self.corner_traffic.spared_frames += 1;
            self.corner_traffic.spared_bytes += corner_bytes(scene);
            return 0;
        }
        if !scene.fresh {
            // **Resident: write the ranges the scene changed into the pair the frame draws.**
            // No second pair and no cursor: a chunk's slice is at the same offset it was drawn
            // at last frame, and the queue orders these writes before the pass.
            self.upload = None;
            let stride = crate::Vertex::STRIDE;
            let vertices_at = scene.vertices.len() as u64 * stride;
            let indices_at = scene.indices.len() as u64 * 4;
            let mut refused = None;
            let mut grew = false;
            match self.corners[self.front].ensure_vertices(&self.device, vertices_at) {
                Ok(allocated) => {
                    grew |= allocated;
                    self.corner_allocations += u64::from(allocated);
                }
                Err(refusal) => refused = Some(refusal),
            }
            if refused.is_none() {
                match self.corners[self.front].ensure_indices(&self.device, indices_at) {
                    Ok(allocated) => {
                        grew |= allocated;
                        self.corner_allocations += u64::from(allocated);
                    }
                    Err(refusal) => refused = Some(refusal),
                }
            }
            if let Some(refusal) = refused {
                // **The pair keeps the scene it already draws.** A refused growth uploads nothing:
                // a torn array would be a wrong picture, and the previous scene is a right one.
                self.note_corner_refusal(refusal);
                return 0;
            }
            let vertex_buffer = self.corners[self.front].vertices.clone();
            let index_buffer = self.corners[self.front].indices.clone();
            // **A pair that grew holds none of the scene it was drawing.** The allocation above
            // is a new, empty buffer, so a write of only this frame's ranges would leave every
            // older slice missing — a world with holes. The whole array is owed on that frame,
            // and the written ranges are replaced by it for this one upload.
            let everything = [[0u32, scene.vertices.len() as u32]];
            let written_vertices: &[[u32; 2]] =
                if grew { &everything } else { &scene.written_vertices };
            let everything = [[0u32, scene.indices.len() as u32]];
            let written_indices: &[[u32; 2]] =
                if grew { &everything } else { &scene.written_indices };
            let mut sent = 0u64;
            for [from, to] in written_vertices {
                let from = *from as usize;
                let to = (*to as usize).min(scene.vertices.len());
                if to <= from {
                    continue;
                }
                if let Some(buffer) = vertex_buffer.as_ref() {
                    let bytes = scene.vertex_bytes_of(from, to);
                    self.queue.write_buffer(buffer, from as u64 * stride, &bytes);
                    sent += bytes.len() as u64;
                }
            }
            for [from, to] in written_indices {
                let from = *from as usize;
                let to = (*to as usize).min(scene.indices.len());
                if to <= from {
                    continue;
                }
                if let Some(buffer) = index_buffer.as_ref() {
                    let bytes = scene.index_bytes_of(from, to);
                    self.queue.write_buffer(buffer, from as u64 * 4, &bytes);
                    sent += bytes.len() as u64;
                }
            }
            let pair = &mut self.corners[self.front];
            pair.build = scene.build;
            pair.indices_drawn = scene.indices.len() as u32;
            self.corner_traffic.uploads += 1;
            self.corner_traffic.bytes += sent;
            return sent;
        }
        // A different build than the one in flight means the world moved again: the old cursor
        // names bytes of a scene that is no longer the frame's, so it is dropped rather than
        // finished. The frame still has the pair it was drawing.
        if self.upload.as_ref().map(|u| u.build) != Some(scene.build) {
            self.upload = Some(crate::work::CornerUpload::new(
                scene.build,
                scene.vertices.len(),
                scene.indices.len(),
            ));
            // **The back pair is sized for the whole scene before the first slice lands.** A pair
            // grown to each slice's end would be reallocated as the upload advanced — losing every
            // byte already written into it — and would pay a fresh GPU allocation on six or seven
            // frames of one upload instead of one. The capacity is the upload's total, stated once.
            let stride = crate::Vertex::STRIDE;
            let (vertices, indices) = self
                .upload
                .as_ref()
                .map(|u| u.capacity(stride))
                .unwrap_or((0, 0));
            // The pair was already sized for the declared window at construction, so this is a
            // no-op for every scene this world can hold — and a count when it is not.
            let back = 1 - self.front;
            let mut refused = None;
            match self.corners[back].ensure_vertices(&self.device, vertices) {
                Ok(true) => self.corner_allocations += 1,
                Ok(false) => {}
                Err(refusal) => refused = Some(refusal),
            }
            if refused.is_none() {
                match self.corners[back].ensure_indices(&self.device, indices) {
                    Ok(true) => self.corner_allocations += 1,
                    Ok(false) => {}
                    Err(refusal) => refused = Some(refusal),
                }
            }
            if let Some(refusal) = refused {
                // No pair can hold this scene, so there is nothing to upload into it: the cursor is
                // dropped and the frame keeps drawing the pair it has.
                self.upload = None;
                self.note_corner_refusal(refusal);
                return 0;
            }
        }
        let back = 1 - self.front;
        let stride = crate::Vertex::STRIDE;
        let slice = match self.upload.as_mut() {
            Some(upload) => upload.next(stride, self.upload_budget),
            None => return 0,
        };
        // **Each slice is written out of the scene's own vectors at its own offset**, so nothing
        // is serialised ahead of the frame that sends it and the working set does not grow by a
        // staging copy of the scene.
        let vertices = scene.vertex_bytes_of(slice.vertex_from, slice.vertex_to);
        if !vertices.is_empty() {
            if let Some(buffer) = self.corners[back].vertices.as_ref() {
                self.queue.write_buffer(buffer, slice.vertex_from as u64 * stride, &vertices);
            }
        }
        let indices = scene.index_bytes_of(slice.index_from, slice.index_to);
        if !indices.is_empty() {
            if let Some(buffer) = self.corners[back].indices.as_ref() {
                self.queue.write_buffer(buffer, slice.index_from as u64 * 4, &indices);
            }
        }
        self.corner_traffic.uploads += 1;
        self.corner_traffic.bytes += slice.bytes;
        let finished = self.upload.as_ref().map(|u| u.done()).unwrap_or(false);
        if finished {
            self.corners[back].build = scene.build;
            self.corners[back].indices_drawn = scene.indices.len() as u32;
            self.front = back;
            self.upload = None;
        }
        slice.bytes
    }

    /// **How many bytes the next upload slice may send**, and how many are still owed.
    ///
    /// The frame states the first from its own budget ([`crate::work::FrameWork::upload`]) and
    /// reports the second, so a scene that takes fourteen frames to arrive is a number a reader
    /// can see rather than a lag they feel.
    pub fn set_upload_budget(&mut self, bytes: u64) {
        self.upload_budget = bytes;
    }

    /// **The last corner buffer the device refused to hold**, with the bytes asked for and the
    /// device's own ceiling — `None` when every ask fitted.
    pub fn corner_refusal(&self) -> Option<CornerRefusal> {
        self.corner_refusal
    }

    /// **How many distinct refusals have happened.** A repeated ask for the same buffer is the
    /// same refusal and does not move this count, so the number is what the app logs rather than a
    /// frame-by-frame tally of one condition.
    pub fn corner_refusals(&self) -> u64 {
        self.corner_refusals
    }

    /// Record a refusal, once per distinct ask.
    fn note_corner_refusal(&mut self, refusal: CornerRefusal) {
        if self.corner_refusal != Some(refusal) {
            self.corner_refusals += 1;
            self.corner_refusal = Some(refusal);
        }
    }

    /// **How many corner buffers have been allocated since construction** — four at `new`, and a
    /// number that must not move when a scene arrives.
    pub fn corner_allocations(&self) -> u64 {
        self.corner_allocations
    }

    /// **The retained corner capacity, in bytes, for the working-set report** — what the two
    /// pairs actually hold, summed from their own capacities.
    ///
    /// It is not [`CORNER_VERTEX_BYTES`] + [`CORNER_INDEX_BYTES`] doubled: those are the capacity
    /// both pairs are *built* with, and a scene larger than the declared window grows them. A
    /// report that kept printing the declared number would state 72 MB for a pair that had grown to
    /// 512 MB — the one line the working set is read from, blind exactly when it matters.
    pub fn corner_retained_bytes(&self) -> u64 {
        self.corners
            .iter()
            .map(|pair| pair.vertex_capacity + pair.index_capacity)
            .sum()
    }

    /// **The capacities both pairs hold**, `(vertices, indices)` — what a test states instead of
    /// trusting that the construction sized them.
    pub fn corner_capacities(&self) -> [(u64, u64); 2] {
        self.corners
            .each_ref()
            .map(|pair| (pair.vertex_capacity, pair.index_capacity))
    }

    /// **Records sent, and how much of it was allocation**: `(uploads, bytes, growths)`.
    pub fn quad_traffic(&self) -> (u64, u64, u64) {
        (self.quad_uploads, self.quad_bytes, self.quad_grows)
    }

    /// **How the record upload was made**: `(records patched, arrays re-encoded)`.
    ///
    /// The pair the report quotes, because the two paths differ by an order of magnitude: a patch
    /// is four bytes a record, a re-encode is the whole array's serialisation.
    pub fn quad_path(&self) -> (u64, u64) {
        (self.quad_patched, self.quad_encoded)
    }

    /// **The encode and the transfer of the last record upload**, in milliseconds.
    pub fn quad_split(&self) -> (f64, f64) {
        (self.quad_serialise_us / 1e3, self.quad_write_us / 1e3)
    }

    /// **The last frame's upload split**, in milliseconds: `(records, corners)`.
    pub fn upload_split(&self) -> (f64, f64) {
        (self.quads_us, self.corners_us)
    }

    /// Bytes an upload in flight still owes, or `0` when the frame is drawing the scene it was
    /// given.
    pub fn upload_pending(&self) -> u64 {
        self.upload
            .as_ref()
            .map(|u| u.pending_bytes(crate::Vertex::STRIDE))
            .unwrap_or(0)
    }

    /// **The quad format's upload, and the rule that keeps it off the hot path**
    /// (C-168).
    ///
    /// `scene.records` are chunk-local and camera-independent; `scene.origins` are
    /// world origins and the eye is subtracted here, in `f64`, once per chunk. So a
    /// step of the camera re-derives the small array and leaves the rectangles
    /// exactly as they were — which is what lets the upload be skipped while the
    /// world stands still:
    ///
    /// * the rectangles are uploaded **only when their values change**, which is when
    ///   a chunk arrives, is evicted, or is remeshed. On the shipped window that is
    ///   1.13 MB of rectangles against the 15.40 MB of vertices and indices the
    ///   vertex format rebuilt and re-copied every frame;
    /// * the origins are 16 bytes a chunk — 9.7 kB for 605 chunks — and are compared
    ///   the same way, so a still camera uploads nothing at all.
    ///
    /// **The comparison is on the records, not on the bytes that carry them.** The
    /// serialisation is the expensive half — a brand-new 1.13 MB `Vec<u8>` written
    /// out of 57,673 records — so building it first and comparing afterwards would
    /// leave most of the per-frame cost standing. Comparing the values and
    /// serialising only on a difference is what makes the steady state a `memcmp`
    /// over 1.38 MB of records rather than a serialisation per frame.
    ///
    /// Buffers grow by doubling and are re-bound only when they do, so a window that
    /// grows pays one buffer creation rather than one per frame.
    pub fn write_quads(&mut self, scene: &Scene, eye_m: [f64; 3]) {
        let mut grew = false;
        // **The bodies are the record array's own tail**, so the buffer is sized on both and the vertex
        // index of a body's draw is its index in this one array (C-184).
        let wanted_records =
            ((scene.records.len() + scene.bodies.len()) * QuadRecord::SIZE as usize).max(4) as u64;
        if wanted_records > self.quad_record_capacity {
            self.quad_record_capacity = wanted_records.next_power_of_two();
            self.quad_records_filled = false;
            self.quad_records = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("quad-records"),
                size: self.quad_record_capacity,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            grew = true;
        }
        let wanted_origins = (scene.origins.len() * ORIGIN_SIZE as usize).max(4) as u64;
        if wanted_origins > self.quad_origin_capacity {
            self.quad_origin_capacity = wanted_origins.next_power_of_two();
            self.quad_origins = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("quad-origins"),
                size: self.quad_origin_capacity,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            grew = true;
        }
        if grew {
            self.quad_grows += 1;
            let rebound = world_bind_group(
                &self.device,
                &self.world_layout,
                &self.rig,
                &self.view,
                &self.materials,
                &self.quad_records,
                &self.quad_origins,
                &self.sky_model,
                &self.shadow,
                &self.horizon,
            );
            self.bind_group = rebound;
        }

        // **A body is appended after the rectangles and in the same array**, so one buffer carries both
        // and the body draw's first vertex is its own offset in it — the same trick the cloud's suffix
        // uses, with the roles of "which pipeline" and "which range" swapped.
        if !scene.fresh {
            // **Resident: the ranges the scene says it changed, and the bodies at its end.** No
            // compare, no serialisation of a prefix, no array the frame has to rebuild: a chunk
            // that landed is one write of its own slice.
            self.quad_record_last.clear();
            self.body_record_last.clear();
            self.quad_staging.clear();
            let stride = QuadRecord::SIZE as u64;
            // **A buffer this frame created holds nothing, so a resident scene owes it every
            // record, not only the ranges that moved.** The device allocation above is a new,
            // empty storage buffer; writing just `written_records` into it leaves the rest of
            // the world's rectangles unwritten, and no later resident frame re-sends them — the
            // terrain is blank until a fresh scene happens to be rebuilt. That is the reported
            // "terrain does not draw until something forces a re-mesh", and it is the same
            // `grew ⇒ everything` rule the corner path above already states.
            let everything = [[0u32, scene.records.len() as u32]];
            let whole = grew || !self.quad_records_filled;
            let written: &[[u32; 2]] = if whole { &everything } else { &scene.written_records };
            for [from, to] in written {
                let from = *from as usize;
                let to = (*to as usize).min(scene.records.len());
                if to <= from {
                    continue;
                }
                let bytes = QuadRecord::bytes(&scene.records[from..to]);
                self.queue.write_buffer(&self.quad_records, from as u64 * stride, &bytes);
                self.quad_uploads += 1;
                self.quad_bytes += bytes.len() as u64;
                self.quad_patched += (to - from) as u64;
            }
            if !scene.bodies.is_empty() {
                let at = scene.records.len() as u64 * stride;
                let bytes = QuadRecord::bytes(&scene.bodies);
                self.queue.write_buffer(&self.quad_records, at, &bytes);
                self.quad_uploads += 1;
                self.quad_bytes += bytes.len() as u64;
            }
            self.quad_records_filled = true;
        } else if scene.records != self.quad_record_last || scene.bodies != self.body_record_last {
            // **What moved, decided before anything is written.** A rebuilt scene is very often the
            // same rectangles with their origin indices renumbered — the walk order changed and a
            // record's `chunk` slot with it — and re-serialising 64 000 of them for that costs
            // 13 ms of a frame under eight busy cores. Those four bytes a record are *patched* in
            // the bytes the buffer last took; anything else is serialised whole.
            // **Never onto a buffer the frame just allocated**: a re-created buffer holds nothing,
            // and the staging bytes are not its contents. A growth takes the encode path.
            let mark = std::time::Instant::now();
            let mut bytes = std::mem::take(&mut self.quad_staging);
            // A frame that re-created the buffer has nothing to patch: the staging bytes are not its
            // contents, so the whole array is serialised into it. `previous` empty is that statement.
            let previous: &[QuadRecord] = if grew { &[] } else { &self.quad_record_last };
            let path = update_record_bytes(&mut bytes, previous, &scene.records, &scene.bodies);
            match path {
                RecordPath::Patched { records } => self.quad_patched += records as u64,
                RecordPath::Encoded => self.quad_encoded += 1,
            }
            self.quad_serialise_us = mark.elapsed().as_secs_f64() * 1e6;
            let mark = std::time::Instant::now();
            self.quad_uploads += 1;
            self.quad_bytes += bytes.len() as u64;
            self.queue.write_buffer(&self.quad_records, 0, &bytes);
            self.quad_write_us = mark.elapsed().as_secs_f64() * 1e6;
            // **The buffer goes back**: cleared or patched next time, allocated never.
            self.quad_staging = bytes;
            self.quad_record_last.clear();
            self.quad_record_last.extend_from_slice(&scene.records);
            self.body_record_last.clear();
            self.body_record_last.extend_from_slice(&scene.bodies);
            self.quad_records_filled = true;
        }
        // **The origins follow the eye, so the upload is keyed on the eye as well as on the
        // world-space array.** `scene.origins` are chunk origins in world metres and do not
        // change as the eye walks; the bytes are `origin − eye` and change with every step.
        // Keying this on the array alone is what froze the world: the records are reused (they
        // are chunk-local, and that is the format's whole claim), the origin buffer kept the
        // first eye's bytes, and the geometry therefore stayed in the eye's own frame — the
        // chunks not moving while the material pattern, whose address is `eye_phase` and
        // `eye_voxel`, scrolled correctly past them.
        if scene.origins != self.quad_origin_last || self.quad_origin_eye != Some(eye_m) {
            self.queue.write_buffer(&self.quad_origins, 0, &origin_bytes(&scene.origins, eye_m));
            self.quad_origin_last.clear();
            self.quad_origin_last.extend_from_slice(&scene.origins);
            self.quad_origin_eye = Some(eye_m);
        }
        // ---- the cloud's own pair ------------------------------------------
        let wanted_clouds =
            (scene.clouds.len() * QuadRecord::SIZE as usize).max(4) as u64;
        let wanted_cloud_origins =
            (scene.cloud_origins.len() * ORIGIN_SIZE as usize).max(4) as u64;
        let mut cloud_grew = false;
        if wanted_clouds > self.cloud_record_capacity {
            self.cloud_record_capacity = wanted_clouds.next_power_of_two();
            self.cloud_records = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("cloud-records"),
                size: self.cloud_record_capacity,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            cloud_grew = true;
        }
        if wanted_cloud_origins > self.cloud_origin_capacity {
            self.cloud_origin_capacity = wanted_cloud_origins.next_power_of_two();
            self.cloud_origins = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("cloud-origins"),
                size: self.cloud_origin_capacity,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            cloud_grew = true;
        }
        if cloud_grew {
            self.cloud_bind_group = world_bind_group(
                &self.device,
                &self.world_layout,
                &self.rig,
                &self.view,
                &self.materials,
                &self.cloud_records,
                &self.cloud_origins,
                &self.sky_model,
                &self.shadow,
                &self.horizon,
            );
            self.cloud_record_last.clear();
            self.cloud_origin_last.clear();
            self.cloud_origin_eye = None;
        }
        if scene.clouds != self.cloud_record_last {
            self.queue.write_buffer(&self.cloud_records, 0, &QuadRecord::bytes(&scene.clouds));
            self.cloud_record_last.clear();
            self.cloud_record_last.extend_from_slice(&scene.clouds);
        }
        if scene.cloud_origins != self.cloud_origin_last || self.cloud_origin_eye != Some(eye_m)
        {
            self.queue.write_buffer(
                &self.cloud_origins,
                0,
                &origin_bytes(&scene.cloud_origins, eye_m),
            );
            self.cloud_origin_last.clear();
            self.cloud_origin_last.extend_from_slice(&scene.cloud_origins);
            self.cloud_origin_eye = Some(eye_m);
        }
    }

    /// Draw the sky and then the scene into `colour`, depth-tested against `depth`.
    ///
    /// **The sky goes first and covers every pixel**, which is the whole repair: the
    /// pass used to clear to transparent, so a pixel with no voxel on it carried no
    /// atmosphere at all. It writes **alpha 0**, so alpha keeps meaning \"geometry is
    /// here\" — a compositor or a HUD can still tell a drawn pixel from the sky behind
    /// it, and `Frame::response` still measures the world rather than the backdrop.
    ///
    /// The clear is retained and still transparent: a sky table that was never written
    /// leaves the table black, and black-on-transparent is what this did before.
    ///
    /// **`colour` is the multisampled attachment when there is one, and `resolve` the
    /// single-sampled target the pass ends by resolving into** — the pair a multisampled
    /// frame needs and a plain one does not:
    ///
    /// * `colour` must have [`WorldPipeline::samples`] samples, and must be the same
    ///   count the pipeline was built with;
    /// * `resolve` is `Some` exactly when that count is above one. A pass whose
    ///   attachment has one sample can carry no resolve target, and one with four that
    ///   carries none does not produce a picture — so the pair is stated together, at
    ///   the call, rather than inferred here from a count that cannot see the texture.
    ///
    /// A frame with `resolve: None` and a multisampled pipeline is a validation error
    /// rather than a silently unresolved frame; the two values are the caller's.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        colour: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        resolve: Option<&wgpu::TextureView>,
        scene: &Scene,
        camera: &Camera,
        pixel_angle_rad: f32,
    ) {
        self.draw_impl(
            encoder,
            colour,
            depth,
            resolve,
            scene,
            camera,
            DrawMode { wireframe: false, pixel_angle_rad },
        );
    }

    /// **The world meshes' own wireframe**: the frame [`WorldPipeline::draw`] would draw, with
    /// the world's own materials **withheld** so the merge's rectangles are the subject.
    ///
    /// On `F4` in the game. The lines are the greedy merge's own records — the four border
    /// edges of each rectangle and no triangulation diagonal, expanded by `shading.wgsl`'s
    /// `vs_wire` from the very buffer the shaded draw reads, so a line cannot sit anywhere but on
    /// the rectangle it outlines. Its hue is swept by distance (the user's own call, 2026-10-04,
    /// after a first white ink read poorly against the world; `WIRE_*` are the shader's declared
    /// range). The sky still draws, so the lines have a horizon; the solid world does not, so
    /// nothing occludes the mesh being read.
    ///
    /// **A frame in the vertex format outlines nothing, and that is stated rather than silent**:
    /// the outlines are the *records*, and [`Scene::build_with_light`]'s corner format carries
    /// none. The shipped game draws the record format, which is the one this exists for.
    ///
    /// **Measuring is unaffected.** [`Renderer`](crate::Renderer) keeps calling
    /// [`WorldPipeline::draw`], so a measured frame carries no wireframe; while this is on the
    /// frame pays one line draw and no upload at all.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_wireframe(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        colour: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        resolve: Option<&wgpu::TextureView>,
        scene: &Scene,
        camera: &Camera,
        pixel_angle_rad: f32,
    ) {
        self.draw_impl(
            encoder,
            colour,
            depth,
            resolve,
            scene,
            camera,
            DrawMode { wireframe: true, pixel_angle_rad },
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_impl(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        colour: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        resolve: Option<&wgpu::TextureView>,
        scene: &Scene,
        camera: &Camera,
        mode: DrawMode,
    ) {
        let DrawMode { wireframe, pixel_angle_rad } = mode;
        // **The sky is drawn or the frame is refused**, before any of it is uploaded: a host that
        // never stated a sky has no frame to draw, and the refusal names what to call.
        self.admit_sky();
        self.write_camera(camera, pixel_angle_rad);
        // **The light horizon, before the pass that marches it.** The view is relative to this
        // frame's eye, and the cube is sent only when a chunk or a recentre moved it. An empty
        // cube is the frame the pass drew before the horizon existed: the shader's taps leave
        // the box and answer clear, which is the rig's own light and not a dark world.
        self.horizon.write(&self.queue, camera.position_m);
        // **The rectangles and their origins, before the pass begins.** Both are
        // uploaded only when they changed — see [`WorldPipeline::write_quads`].
        let mark = std::time::Instant::now();
        self.write_quads(scene, camera.position_m);
        self.quads_us = mark.elapsed().as_secs_f64() * 1e3;
        // **And the corner buffers only for a frame that carries corners.** A frame
        // in the quad format has no vertex and no index at all, so building them
        // here would be two `create_buffer` calls and two uploads of nothing per
        // frame — the very cost this format exists to remove. A frame in the vertex
        // format pays for its own bytes once per scene, not once per frame: see
        // [`WorldPipeline::write_corners`].
        let mark = std::time::Instant::now();
        self.write_corners(scene);
        self.corners_us = mark.elapsed().as_secs_f64() * 1e3;
        let front = &self.corners[self.front];
        // **A record-format frame has no corners to draw**, even when a previous frame uploaded
        // some: the pair in hand is then the last vertex-format scene, and drawing it beside the
        // records would draw two worlds at once.
        let corners = !scene.indices.is_empty() && front.indices_drawn > 0;
        let corners_drawn = front.indices_drawn;

        // **The beam's shadow, before the picture it darkens.** The two maps are drawn from the
        // same corners and origins the pass below draws, from the light the rig was last given;
        // an empty or record-format scene has nothing to cast and the shader answers "lit".
        let lit = self.shadows && self.sun.is_some_and(|light| light[3] > 0.0);
        // **A disabled pass states no beam**, so the fragment's own flag answers "lit" even if a
        // previous frame's map is still bound.
        self.shadow.write(
            &self.queue,
            camera,
            if self.shadows { self.sun } else { None },
            self.shadow_range,
        );
        if let (true, Some(vertices), Some(indices)) =
            (corners, &front.vertices, &front.indices)
        {
            self.shadow.render(
                &self.device,
                encoder,
                &self.quad_origins,
                vertices,
                indices,
                corners_drawn,
                lit,
            );
        }

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("world"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: colour,
                depth_slice: None,
                // **The whole of the multisampling's cost and its whole benefit**: the pass
                // draws at its attachment's count and the resolve averages the samples into
                // one pixel at the end. Absent for a one-sample frame, which is then the frame
                // it was before any of this existed.
                resolve_target: resolve,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        // Sky first, unconditionally — including for an empty scene, because an
        // empty scene is exactly the case that used to show nothing at all.
        pass.set_pipeline(&self.sky_pipeline);
        pass.set_bind_group(0, &self.sky_bind_group, &[]);
        pass.draw(0..3, 0..1);

        // **The debug wireframe replaces the material draws below** (F4) rather than being drawn
        // over them: the lines are the subject, and the sky the pass has already drawn is what
        // they are read against. One `if` decides it, so no material draw can leak into the view
        // that exists to replace it.
        if wireframe {
            let rectangles = scene.records.len() as u32;
            // **The pipeline is set once, before either pair**, because a frame whose world has
            // no terrain records and only a cloud still has to leave the sky pipeline behind.
            pass.set_pipeline(&self.wire_pipeline);
            if rectangles > 0 {
                pass.set_bind_group(0, &self.bind_group, &[]);
                // Eight line vertices a rectangle — its four border edges and no diagonal — with
                // no vertex buffer and no index: `vs_wire` addresses the record at
                // `vertex_index / 8`.
                pass.draw(0..rectangles * 8, 0..1);
            }
            // **And the sky's own rectangles**, from its own pair: the debug view draws every
            // rectangle the frame draws, and the cloud is no longer one of the world's records.
            if !scene.clouds.is_empty() {
                pass.set_bind_group(0, &self.cloud_bind_group, &[]);
                pass.draw(0..scene.clouds.len() as u32 * 8, 0..1);
            }
        } else if let (true, Some(vertex_buffer), Some(index_buffer)) =
            (corners, &self.corners[self.front].vertices, &self.corners[self.front].indices)
        {
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            // **What was uploaded, not what the scene holds.** A sliced upload means the front
            // pair is a complete older scene while the back one fills; drawing the new scene's
            // count would read past what has landed.
            pass.draw_indexed(0..corners_drawn, 0, 0..1);
        }

        // **And the quad format's own geometry**: one draw, six vertices a
        // rectangle, no vertex buffer and no index buffer — the expansion is
        // `vs_quad`'s. The pipeline layout, the depth state and the fragment shader
        // are the indexed draw's above; only the vertex stage differs, which is
        // what makes the two comparable (see `renderer.rs`'s parity test).
        // **The world's own rectangles, without the cloud's.** The cloud's records are the
        // suffix of the same array (C-179), and they must not be drawn here: the opaque
        // pipeline writes depth and blends nothing, so a veil drawn through it would be an
        // opaque box — and it would write depth, hiding the cells behind it.
        let world_records = scene.records.len();
        if !wireframe && world_records > 0 {
            pass.set_pipeline(&self.quad_pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.draw(0..6 * world_records as u32, 0..1);
        }

        // **And the astronomical bodies, in the record array's own tail** (C-184). Drawn before the cloud
        // so the veil is tested against a body's depth, and after the terrain so a hill in front of the
        // Moon still hides it. The range begins at the last rectangle's index: the bodies are appended to
        // the same array by `write_quads`, so no second buffer and no second binding is needed to place
        // one — the record and its origin are what a body is.
        if !wireframe && !scene.bodies.is_empty() {
            let first = 6 * scene.records.len() as u32;
            pass.set_pipeline(&self.body_pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.draw(first..first + 6 * scene.bodies.len() as u32, 0..1);
        }

        // **And the cloud, last and blended** (C-179). Last, because the veil composites over
        // what is already in the target — the sky the pass drew first, and the terrain the
        // draw above put there — and because the depth buffer it tests against has to be the
        // world's own. The range is the records' own suffix, and `vs_quad` addresses the record
        // at `vertex_index / 6`, so starting the draw at `6 · world_records` needs no new
        // format, no second buffer and no index: one array, two draws.
        //
        // **A frame that carries no cloud is the frame it was.** The draw is inside this `if`,
        // so an empty cloud set is not a pass with nothing in it — it is no pass at all.
        if !wireframe && !scene.clouds.is_empty() {
            pass.set_pipeline(&self.cloud_pipeline);
            pass.set_bind_group(0, &self.cloud_bind_group, &[]);
            pass.draw(0..6 * scene.clouds.len() as u32, 0..1);
        }
    }
}

/// **What the corner path has sent, and what it did not** — see
/// [`WorldPipeline::write_corners`] and [`WorldPipeline::corner_traffic`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CornerTraffic {
    /// Uploads made: one per scene that carried corners, not one per frame.
    pub uploads: u64,
    /// Bytes those uploads sent — vertices and indices together.
    pub bytes: u64,
    /// Frames that carried corners and sent nothing, because the scene was the one already
    /// held.
    pub spared_frames: u64,
    /// The bytes those frames would have sent under a per-frame upload: `spared_frames` × the
    /// scene's own size, which is the cost this change removes from a frame that walks through
    /// an unchanged world.
    pub spared_bytes: u64,
}

/// **How a new record array reached the buffer.**
///
/// Two states, because there are two: the leading records were reused (patched where their origin
/// index moved, appended where the array grew) or the array was serialised whole. There is
/// deliberately no "nothing changed" state — the caller only calls this when the array's values
/// differ from the ones last sent, so a patch that happens to move no origin slot (a body arriving
/// beside unchanged records, a fade ending) still rebuilt bytes and still has to be counted.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum RecordPath {
    /// The leading records kept their shape, so the buffer's own bytes were reused: `records` of
    /// them had their origin index moved, and the rest of the array was appended after them.
    Patched { records: usize },
    /// Some geometry changed: the array was serialised whole.
    Encoded,
}

/// **Make `bytes` hold exactly `next` then `bodies`**, patching the origin indices when the leading
/// records are the same rectangles in the same order.
///
/// This is the whole incremental upload, and it exists for one measured reason: a rebuilt scene is
/// almost always the **same rectangles renumbered**. A chunk's quads are the chunk's quads, and
/// what moves between two scenes is the index into the origin array, because the walk that built
/// them visited the chunks in a different order — measured over a loaded run, 55 000–64 511 records
/// differed between consecutive uploads and **every one of them kept its shape**. A value diff
/// therefore had nothing to skip; four bytes a record did.
///
/// The invariant is byte equality, not approximation: after this call `bytes` equals
/// [`QuadRecord::bytes`] of the new array followed by the bodies, which the pin
/// `a_patched_upload_is_the_encoded_array_byte_for_byte` states over a renumbered scene, a grown
/// one, and one with a changed rectangle.
fn update_record_bytes(
    bytes: &mut Vec<u8>,
    previous: &[QuadRecord],
    next: &[QuadRecord],
    bodies: &[QuadRecord],
) -> RecordPath {
    if next.is_empty() && bodies.is_empty() {
        bytes.clear();
        return RecordPath::Encoded;
    }
    let stride = QuadRecord::SIZE as usize;
    // The leading records that are the same rectangle: a patch can only start there.
    let shared = previous
        .iter()
        .zip(next.iter())
        .take_while(|(was, now)| {
            was.face == now.face
                && was.material == now.material
                && was.slice == now.slice
                && was.uv == now.uv
                && was.extent == now.extent
                && was.light_y == now.light_y
                && was.light_z == now.light_z
        })
        .count();
    // **Staging must hold the previous array's own bytes** or a patch would write the right four
    // bytes at the wrong record's offset: the length guard is what says it does.
    let patchable = shared > 0 && bytes.len() >= previous.len() * stride;
    if !patchable {
        bytes.clear();
        QuadRecord::write_into(next, bytes);
        QuadRecord::write_into(bodies, bytes);
        return RecordPath::Encoded;
    }
    // **The buffer's own bytes, kept and corrected.** The old array's tail (records past the
    // shared prefix, and the old bodies) is dropped before the new one is appended: a stale tail
    // is a record the draw would read after the array ended. The bodies are re-encoded rather than
    // patched — there are one or two of them, and a body's own origin index moves the same way.
    let mut patched = 0usize;
    for index in 0..shared {
        if previous[index].chunk != next[index].chunk {
            patch_chunk(bytes, index, next[index].chunk);
            patched += 1;
        }
    }
    bytes.truncate(shared * stride);
    QuadRecord::write_into(&next[shared..], bytes);
    QuadRecord::write_into(bodies, bytes);
    RecordPath::Patched { records: patched }
}

/// **Write one record's origin index into the bytes the buffer holds**, at its own stride.
///
/// The slot is `chunk`'s own position in [`QuadRecord::slots`], which is what a patch has to agree
/// with: a pin writes a scene through both this and [`QuadRecord::bytes`] and compares the two
/// byte for byte.
fn patch_chunk(bytes: &mut [u8], index: usize, chunk: u32) {
    let at = index * QuadRecord::SIZE as usize + CHUNK_SLOT * 4;
    bytes[at..at + 4].copy_from_slice(&voxel_model::endian::le_bytes(chunk));
}

/// Where [`QuadRecord::chunk`] sits in a record, in slots.
///
/// Counted once, and held to the record's own field order by a pin that writes a record through
/// both this offset and [`QuadRecord::bytes`] and compares the two: the slot moving without this
/// constant moving would patch the wrong four bytes of every record.
const CHUNK_SLOT: usize = 5;

/// The bytes a scene's corners occupy on the way to the GPU — the vertex stride times the
/// vertices plus four an index. Arithmetic on the lengths, not a serialisation: the whole point
/// is that measuring what was *not* sent costs nothing.
fn corner_bytes(scene: &Scene) -> u64 {
    scene.vertices.len() as u64 * Vertex::STRIDE + scene.indices.len() as u64 * 4
}

/// **The corner capacity both pairs are built with, so that no frame allocates.**
///
/// A pair grown when a scene arrived allocated fresh GPU memory inside a live streaming frame —
/// 77.2 ms, `draw` 43.3 — and PATCH-053's fix traded seven growths of one upload for exactly one
/// such frame. Sizing both pairs once, before the first frame, is the rest of that trade: the bytes
/// are retained for the process, and they are stated in the `upload-split` line beside the uploads
/// they serve rather than left outside the working-set ledger.
///
/// **The numbers are the lit window's, and they moved when the lattice did.** The old declaration
/// counted the record format's four corners a quad — 228 700 for 605 chunks — which was a reading
/// of a frame that no longer exists: since PATCH-070 a lit face is cut at the subvoxel lattice, so
/// its corners are the lattice's samples (a face whose light is uniform, or whose corners are not
/// all measured, keeps four; a face the light crosses pays `(cells + 1)²`). Measured on the
/// settled shipped window on 2026-10-23 (five minutes, 605 chunks, 57 175 quads): **2 050 887 live
/// corners and 11 055 720 indices**, the arrays held at **2 402 372 corners** (1.17× live, the
/// quarter rule's own bound) — 163.4 MB of vertices at the 68-byte stride and 42.2 MB of indices.
/// The capacity is the growth rule's own next power of two above each: **256 MiB** of vertices
/// (3 947 808 corners) and **64 MiB** of indices (16 777 216). A pair is 320 MiB and the two pairs
/// **640 MiB** — the price of never paying for an allocation on a frame, and the number
/// [`WorldPipeline::corner_retained_bytes`] reports as what the pairs actually hold.
///
/// **The vertex capacity is the device's default ceiling**, so it is the last number this can
/// reach: `wgpu`'s default `max_buffer_size` is 256 MiB, and a device below it refuses the pair
/// rather than making an invalid buffer. That is deliberate and stated: the subvoxel lattice's
/// 2.07 M corners are 168 MB at the arena's peak, and declaring less than the next power of two
/// above that would put a growth back on a live frame.
///
/// A scene larger than that still grows, on the frame it arrives, and is **counted**: see
/// [`WorldPipeline::corner_allocations`]. A growth is a new, empty buffer, so that frame uploads
/// the whole scene rather than the ranges that changed; a scene the device cannot hold at all is
/// **refused** ([`CornerRefusal`]) and the frame keeps drawing the pair it had.
pub const CORNER_VERTEX_BYTES: u64 = 256 * 1024 * 1024;
/// The index bytes both pairs hold: 16 777 216 indices, the next power of two above the lit
/// window's 11 055 720.
pub const CORNER_INDEX_BYTES: u64 = 64 * 1024 * 1024;
/// The corners 128 MiB of vertices holds at the shipped stride.
pub const CORNER_VERTICES: usize = (CORNER_VERTEX_BYTES / Vertex::STRIDE) as usize;

/// **A corner buffer the device will not hold.**
///
/// `wgpu` answers a `create_buffer` past [`wgpu::Limits::max_buffer_size`] with an **invalid
/// buffer** and a validation error, not with an `Err`: the call still returns a handle, every
/// later use of it fails, and the frame stops drawing with no refusal of its own to point at.
/// This is that refusal, made before the allocation and reported with both numbers — the bytes
/// the scene asked for and the device's own ceiling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CornerRefusal {
    /// Which array was asked for: `"corner-vertices"` or `"corner-indices"`.
    pub array: &'static str,
    /// The bytes the pair would have held, as it would have allocated them (a power of two).
    pub asked: u64,
    /// [`wgpu::Limits::max_buffer_size`] on the device the buffer was asked of.
    pub limit: u64,
}

impl std::fmt::Display for CornerRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} would need {:.1} MB, past this device's {:.1} MB maximum buffer size",
            self.array,
            self.asked as f64 / (1024.0 * 1024.0),
            self.limit as f64 / (1024.0 * 1024.0),
        )
    }
}

/// **One scene's corner buffers**, drawn or being written.
#[derive(Default)]
struct CornerPair {
    vertices: Option<wgpu::Buffer>,
    indices: Option<wgpu::Buffer>,
    vertex_capacity: u64,
    index_capacity: u64,
    /// The scene build these hold; `0` is "no scene yet".
    build: u64,
    /// How many indices were uploaded — the draw's own range.
    indices_drawn: u32,
}

impl CornerPair {
    /// Grow to hold `bytes` vertices, by doubling, and allocate nothing if it already does.
    ///
    /// Returns whether it allocated: the one thing a frame must never do, and therefore the one
    /// thing [`WorldPipeline::corner_allocations`] counts rather than assumes. A request past the
    /// device's own [`wgpu::Limits::max_buffer_size`] is **refused**, not attempted: `wgpu` would
    /// hand back an invalid buffer whose every later write fails, which is a window that stops
    /// drawing rather than a refusal a reader can see.
    fn ensure_vertices(&mut self, device: &wgpu::Device, bytes: u64) -> Result<bool, CornerRefusal> {
        let wanted = bytes.max(4).next_power_of_two();
        if wanted <= self.vertex_capacity && self.vertices.is_some() {
            return Ok(false);
        }
        let limit = device.limits().max_buffer_size;
        if wanted > limit {
            return Err(CornerRefusal { array: "corner-vertices", asked: wanted, limit });
        }
        self.vertex_capacity = wanted;
        self.vertices = Some(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("corner-vertices"),
            size: wanted,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        Ok(true)
    }

    /// The same for the index buffer.
    fn ensure_indices(&mut self, device: &wgpu::Device, bytes: u64) -> Result<bool, CornerRefusal> {
        let wanted = bytes.max(4).next_power_of_two();
        if wanted <= self.index_capacity && self.indices.is_some() {
            return Ok(false);
        }
        let limit = device.limits().max_buffer_size;
        if wanted > limit {
            return Err(CornerRefusal { array: "corner-indices", asked: wanted, limit });
        }
        self.index_capacity = wanted;
        self.indices = Some(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("corner-indices"),
            size: wanted,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        Ok(true)
    }
}

/// The world's bind group — **one statement of its six entries**, because a grown
/// quad buffer has to be re-bound and a second copy of this list is how one entry
/// drifts from the layout (`write_quads` re-binds through it too).
///
/// `sky` is the **same** buffer the sky pass binds: one dome table, drawn by one pass and
/// reflected by the other, so a fluid cannot show a sky that is not the one overhead.
#[allow(clippy::too_many_arguments)]
fn world_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    rig: &wgpu::Buffer,
    view: &wgpu::Buffer,
    materials: &wgpu::Buffer,
    records: &wgpu::Buffer,
    origins: &wgpu::Buffer,
    sky: &wgpu::Buffer,
    shadow: &crate::sun_shadow::SunShadow,
    horizon: &crate::horizon::HorizonStream,
) -> wgpu::BindGroup {
    let mut entries = vec![
        wgpu::BindGroupEntry { binding: 0, resource: rig.as_entire_binding() },
        wgpu::BindGroupEntry { binding: 1, resource: view.as_entire_binding() },
        wgpu::BindGroupEntry { binding: 2, resource: materials.as_entire_binding() },
        wgpu::BindGroupEntry { binding: 3, resource: records.as_entire_binding() },
        wgpu::BindGroupEntry { binding: 4, resource: origins.as_entire_binding() },
        wgpu::BindGroupEntry { binding: 5, resource: sky.as_entire_binding() },
    ];
    entries.extend(shadow.bind_entries());
    entries.extend(horizon.bind_entries());
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("world"),
        layout,
        entries: &entries,
    })
}

/// **The cloud's field as the buffer holds it, past its two header elements**: one span per record,
/// then the field's texels `[water, base, top, kind]`, indexed `j · columns + i`.
/// The cloud's field in the buffer's own bytes: one `vec4` per element, little-endian.
fn uniform_entry(binding: u32, visibility: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn flatten_materials(materials: &[MaterialParams]) -> Vec<u8> {
    let mut out = Vec::with_capacity(materials.len() * MaterialParams::SIZE as usize);
    for m in materials {
        out.extend_from_slice(&m.bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::pass_source;

    /// **The frame a patched buffer draws is the frame an encoded one draws.**
    ///
    /// The byte pins above hold the update to the encoder's own bytes; this one holds the **picture**
    /// to it, through the real pipeline: two renderers draw the same scene, one that reached it by a
    /// patch and one that serialised it from nothing, and every pixel has to agree. A patch that
    /// wrote the right four bytes at the wrong record, or left the old body behind, changes the
    /// picture and this fails on the frame rather than on a number.
    #[test]
    fn a_patched_record_buffer_draws_the_frame_an_encoded_one_draws() {
        use crate::renderer::Renderer;
        use crate::shading::{GpuRig, MaterialParams};
        use crate::Camera;

        let table = vec![MaterialParams::default(); 2];
        let rig = GpuRig::default();
        let mut patched = Renderer::new(33, 33, &table, &rig).expect("a GPU");
        patched.world_mut().write_no_sky_model();
        let mut encoded = Renderer::new(33, 33, &table, &rig).expect("a GPU");
        encoded.world_mut().write_no_sky_model();
        let camera = Camera::standard([0.0, 0.0, 0.0], 1.0);

        // Two chunks' rectangles in front of the eye, and the origins they name. `flip` renumbers
        // every record and swaps the origins with them: the same world, said the other way round.
        let scene = |flip: bool| {
            let records = (0..48)
                .map(|i| {
                    let mut r = record(i, (i % 2) as u32);
                    if flip {
                        r.chunk = 1 - r.chunk;
                    }
                    r.uv = (i % 6) as u32;
                    r.extent = 1 | (1 << 16);
                    r
                })
                .collect();
            let mut origins = vec![[-1.0, -1.0, 2.0, 0.0], [0.5, -1.0, 2.0, 0.0]];
            if flip {
                origins.swap(0, 1);
            }
            crate::scene::Scene { build: 1, records, origins, ..crate::scene::Scene::default() }
        };

        let first = patched.render(&scene(false), &camera).expect("a frame");
        let after_patch = patched.render(&scene(true), &camera).expect("a frame");
        let (patched_records, _) = patched.world().quad_path();
        assert!(patched_records > 0, "the second scene was not patched at all");

        let from_scratch = encoded.render(&scene(true), &camera).expect("a frame");
        assert_eq!(
            after_patch, from_scratch,
            "the patched buffer drew a different frame from the encoded one"
        );
        // And the first frame was a picture at all, so the comparison is not two blank images.
        assert!(
            first.pixels.iter().any(|p| p[3] > 0.0),
            "nothing was drawn, so the comparison proves nothing"
        );
    }

    /// **A resident scene owes a brand-new record buffer the whole array.**
    ///
    /// The reported defect: the world's terrain does not draw until something forces a re-mesh.
    /// The first frame of a window allocates the record buffer; a resident scene (a chunk arrived)
    /// then says only `written_records` changed, and the upload used to honour that literally — so
    /// the rectangles the buffer was allocated for were never written, and no later resident frame
    /// re-sends them. The fix is the corner path's own rule (`grew ⇒ everything`), and this test is
    /// the frame it is for: the same pipeline, one resident scene carrying **only its newest
    /// chunk**, and every record must reach the buffer.
    #[test]
    fn a_resident_scene_lands_whole_in_a_pipeline_that_has_just_allocated_its_records() {
        use crate::renderer::Renderer;
        use crate::shading::{GpuRig, MaterialParams};
        use crate::Camera;

        let table = vec![MaterialParams::default(); 2];
        let rig = GpuRig::default();
        let mut renderer = Renderer::new(33, 33, &table, &rig).expect("a GPU");
        renderer.world_mut().write_no_sky_model();
        let camera = Camera::standard([0.0, 0.0, 0.0], 1.0);

        let records: Vec<_> = (0..48).map(|i| record(i, (i % 2) as u32)).collect();
        let origins = vec![[-1.0, -1.0, 2.0, 0.0], [0.5, -1.0, 2.0, 0.0]];
        let resident = crate::scene::Scene {
            build: 7,
            fresh: false,
            records: records.clone(),
            origins,
            // One chunk's slice: records 40..48 are the only ones this frame changed.
            written_records: vec![[40, 48]],
            ..crate::scene::Scene::default()
        };
        let frame = renderer.render(&resident, &camera).expect("a frame");
        let (patched, _) = renderer.world().quad_path();
        assert_eq!(
            patched,
            records.len() as u64,
            "the pipeline allocated its record buffer this frame and wrote {patched} of {} \
             rectangles: the rest of the world is missing until a fresh scene rebuilds it",
            records.len()
        );
        assert!(
            frame.pixels.iter().any(|p| p[3] > 0.0),
            "the first resident frame drew nothing at all"
        );
        // **And it is the frame a rebuild would draw**, not merely a frame with something in it:
        // a second pipeline given the same world as one fresh scene must put the same light and
        // the same rectangles in the same pixels.
        let mut rebuilt = Renderer::new(33, 33, &table, &rig).expect("a GPU");
        rebuilt.world_mut().write_no_sky_model();
        let fresh = crate::scene::Scene {
            build: 8,
            fresh: true,
            records,
            origins: vec![[-1.0, -1.0, 2.0, 0.0], [0.5, -1.0, 2.0, 0.0]],
            ..crate::scene::Scene::default()
        };
        let expected = rebuilt.render(&fresh, &camera).expect("a frame");
        let differing = frame
            .pixels
            .iter()
            .zip(&expected.pixels)
            .filter(|(a, b)| a != b)
            .count();
        println!(
            "first resident frame: {patched} record(s) written, {differing} of {} pixels differ \
             from the fresh rebuild",
            frame.pixels.len()
        );
        assert_eq!(
            differing, 0,
            "the first resident frame differs from the same world rebuilt: the streaming path is \
             not what the reload draws"
        );
    }

    /// A record with `chunk` as its only moving part, and a stable rectangle.
    fn record(i: usize, chunk: u32) -> crate::quad::QuadRecord {
        crate::quad::QuadRecord {
            face: (i % 6) as u32,
            material: 1,
            slice: 0,
            uv: (i & 0xffff) as u32,
            extent: 4 | (4 << 16),
            chunk,
            light_y: 0,
            light_z: 0,
        }
    }

    /// **The patched upload is the encoded array, byte for byte.**
    ///
    /// The incremental path exists because a rebuilt scene is the same rectangles renumbered, and it
    /// is only sound if the bytes it leaves are **exactly** the bytes a whole-array serialisation
    /// would have written — no shifted record, no unwritten tail, no stale body. So the pin states
    /// that equality over the four shapes a frame actually produces: a renumbered array, one that
    /// grew, one that shrank (where a stale tail would be read after the end), and one with a
    /// changed rectangle.
    #[test]
    fn a_patched_upload_is_the_encoded_array_byte_for_byte() {
        use super::{update_record_bytes, RecordPath};

        let previous: Vec<_> = (0..1_000).map(|i| record(i, (i % 37) as u32)).collect();
        let mut bytes = crate::quad::QuadRecord::bytes(&previous);
        let encode = |records: &[crate::quad::QuadRecord]| crate::quad::QuadRecord::bytes(records);

        // 1. Renumbered: the same rectangles, every origin index moved.
        let renumbered: Vec<_> = previous.iter().map(|r| record(0, 0)).zip(0..1_000).map(|(_, i)| {
            let mut r = previous[i];
            r.chunk = (i % 37) as u32 + 1;
            r
        }).collect();
        assert_eq!(
            update_record_bytes(&mut bytes, &previous, &renumbered, &[]),
            RecordPath::Patched { records: 1_000 },
            "a renumbered scene was not patched"
        );
        assert_eq!(bytes, encode(&renumbered), "the patched bytes are not the array's bytes");

        // 2. Grown: renumbered, and 250 records the buffer has never held.
        let mut grown = renumbered.clone();
        grown.extend((1_000..1_250).map(|i| record(i, 3)));
        bytes = encode(&renumbered);
        assert!(matches!(
            update_record_bytes(&mut bytes, &renumbered, &grown, &[]),
            RecordPath::Patched { .. }
        ));
        assert_eq!(bytes, encode(&grown), "a grown array's tail is not what was appended");

        // 3. Shrunk: the tail is gone, and the bytes must not still carry it.
        let shrunk = &grown[..400];
        bytes = encode(&grown);
        update_record_bytes(&mut bytes, &grown, shrunk, &[]);
        assert_eq!(bytes, encode(shrunk), "a stale tail survived the shrink");
        assert_eq!(bytes.len(), 400 * crate::quad::QuadRecord::SIZE as usize);

        // 4. A changed rectangle: patching can only carry the array to the record that changed, and
        //    everything from there is serialised — so the bytes are still the array's own.
        let mut edited = shrunk.to_vec();
        edited[200].uv = edited[200].uv.wrapping_add(9);
        bytes = encode(shrunk);
        let path = update_record_bytes(&mut bytes, shrunk, &edited, &[]);
        assert!(
            matches!(path, RecordPath::Patched { .. } | RecordPath::Encoded),
            "a changed rectangle took {path:?}"
        );
        assert_eq!(bytes, encode(&edited), "a changed rectangle left bytes that are not its own");

        // 5. Nothing in common at the front: no patch is possible, and the whole array is encoded.
        let mut reversed: Vec<_> = (0..shrunk.len()).map(|i| record(shrunk.len() - i, 0)).collect();
        reversed.truncate(shrunk.len());
        bytes = encode(shrunk);
        assert_eq!(
            update_record_bytes(&mut bytes, shrunk, &reversed, &[]),
            RecordPath::Encoded,
            "an array with no shared prefix was patched"
        );
        assert_eq!(bytes, encode(&reversed));

        // 6. A body arriving beside unchanged records. The array did not move, so nothing is
        //    patched — and the upload still happened, which is why the path must not read as "the
        //    bytes already there are the array's": the tail past the records is new bytes.
        let body = [record(0, 9)];
        bytes = encode(shrunk);
        assert_eq!(
            update_record_bytes(&mut bytes, shrunk, shrunk, &body),
            RecordPath::Patched { records: 0 },
            "a body arriving beside unchanged records was counted as no upload at all"
        );
        let mut expected = encode(shrunk);
        expected.extend_from_slice(&encode(&body));
        assert_eq!(bytes, expected, "the body is not appended after the array it arrived beside");
    }

    /// **A patch writes the origin slot and nothing else.**
    ///
    /// The other half of the same claim, and the one that holds `CHUNK_SLOT` to the record's own
    /// field order: the bytes a patch leaves differ from the bytes already there **only** inside the
    /// four-byte origin slot of the records whose index moved. If the slot moved in `QuadRecord` and
    /// not here, this fails on the offsets rather than on a picture.
    #[test]
    fn a_patch_touches_the_origin_slot_and_nothing_else() {
        use super::{update_record_bytes, CHUNK_SLOT};

        let previous: Vec<_> = (0..300).map(|i| record(i, 1)).collect();
        let next: Vec<_> = (0..300)
            .map(|i| {
                let mut r = previous[i];
                r.chunk = 2;
                r
            })
            .collect();
        let before = crate::quad::QuadRecord::bytes(&previous);
        let mut bytes = before.clone();
        update_record_bytes(&mut bytes, &previous, &next, &[]);

        let stride = crate::quad::QuadRecord::SIZE as usize;
        assert_eq!(bytes.len(), before.len());
        for at in 0..bytes.len() {
            if bytes[at] == before[at] {
                continue;
            }
            let slot = at % stride;
            assert!(
                (CHUNK_SLOT * 4..CHUNK_SLOT * 4 + 4).contains(&slot),
                "byte {at} (slot {slot} of a record) moved, and a patch may only write the origin slot"
            );
        }
        // And the slot is the record's own: the bytes there are the chunk it was given.
        let at = 7 * stride + CHUNK_SLOT * 4;
        assert_eq!(&bytes[at..at + 4], &voxel_model::endian::le_bytes(next[7].chunk));
    }

    /// **No frame allocates a corner buffer** — the pin PATCH-053's fix left open.
    ///
    /// Both pairs are sized at construction, from [`super::CORNER_VERTEX_BYTES`] and
    /// [`super::CORNER_INDEX_BYTES`], and the write path only grows them for a scene larger than
    /// that. So: a scene the size of the settled shipped window's own array — 2 402 372 corners
    /// and 11 055 720 indices at the subvoxel lattice — arrives, is written into the pairs, and
    /// the allocation count does not move. Delete the pre-size from `new`, or lower the capacity
    /// under the window the streamer is held to, and this fails on the number it is about rather
    /// than on a frame somebody has to notice is 77 ms long.
    #[test]
    fn a_scene_arriving_allocates_no_corner_buffer() {
        use super::{CORNER_INDEX_BYTES, CORNER_VERTEX_BYTES, WorldPipeline, CORNER_VERTICES};
        use crate::renderer::TARGET_FORMAT;
        use crate::scene::{Scene, Vertex};
        use crate::shading::{GpuRig, MaterialParams};

        let (device, queue, _) = crate::renderer::offscreen_device().expect("a GPU");
        let mut world = WorldPipeline::new(
            &device,
            &queue,
            TARGET_FORMAT,
            &[MaterialParams::default()],
            &GpuRig::default(),
            1,
        )
        .expect("the world pipeline");

        // Four buffers — two pairs — for the life of the process.
        assert_eq!(
            world.corner_allocations(),
            4,
            "the corner pairs are not both sized before the first frame draws"
        );
        assert_eq!(
            world.corner_capacities(),
            [(CORNER_VERTEX_BYTES, CORNER_INDEX_BYTES); 2],
            "a pair was built with a capacity other than the stated one"
        );

        // **The lit window's own measured surface** (2026-10-23, the settled shipped window,
        // five minutes): 2 050 887 live corners at the lattice and 11 055 720 indices, with the
        // arena's quarter rule holding the array at 2 402 372 corners. The declared capacity is
        // the next power of two above the array's own peak, so this is the largest scene a frame
        // can be handed without a growth.
        let corners = 2_402_372usize;
        assert!(corners < CORNER_VERTICES, "{corners} corners is not inside the capacity");
        let corner = Vertex {
            position: [0.0; 3],
            normal: [0.0, 1.0, 0.0],
            material: 0,
            detail_roughness: 0.0,
            plane_axis: 1,
            light_direct: [0.0; 3],
            light_diffuse: [0.0; 3],
            light_present: 0,
            chunk: 0,
        };
        let mut scene = Scene { build: 9, ..Scene::default() };
        scene.vertices = vec![corner; corners];
        scene.indices = vec![0u32; 11_055_720];

        // One slice, because the budget is the whole scene.
        world.set_upload_budget(u64::MAX);
        let sent = world.write_corners(&scene);
        assert!(sent > 0, "the scene was not written at all");

        assert_eq!(
            world.corner_allocations(),
            4,
            "a scene arriving inside the stated capacity allocated a corner buffer: the allocation is back on a live frame"
        );
        assert_eq!(world.upload_pending(), 0, "the whole scene did not land in one slice");
    }

    /// **A buffer past the device's own ceiling is refused before it is asked for.** `wgpu`
    /// answers an oversized `create_buffer` with an invalid handle and a validation error, and
    /// every later use of that handle fails — the window stops drawing with no refusal of its
    /// own to point at. This pins the refusal, its numbers, and that no buffer was made.
    #[test]
    fn a_corner_buffer_past_the_device_limit_is_refused_by_name() {
        let (device, _queue, _) = crate::renderer::offscreen_device().expect("a GPU");
        let limit = device.limits().max_buffer_size;
        let mut pair = super::CornerPair::default();
        let refusal = pair
            .ensure_vertices(&device, limit + 1)
            .expect_err("a buffer past the device limit was attempted");
        assert_eq!(refusal.array, "corner-vertices");
        assert_eq!(refusal.limit, limit);
        assert!(
            refusal.asked > limit,
            "the refusal names {} bytes for a {limit}-byte device",
            refusal.asked
        );
        assert!(
            pair.vertices.is_none(),
            "an invalid corner buffer was created and would fail every write"
        );
        // And the next buffer the device can hold is still made: the refusal is not sticky.
        assert!(matches!(pair.ensure_vertices(&device, 4 * 1024 * 1024), Ok(true)));
    }

    /// **Every pass that presents a frame**, as `(file, source)`.
    ///
    /// These two write colour into the target, so these two receive the display stage.
    /// The wireframe is deliberately absent — and so is the cloud pass, which no longer exists:
    /// the clouds are geometry now.
    const PRESENTING: [(&str, &str); 2] = [
        ("shading.wgsl", include_str!("shading.wgsl")),
        ("sky.wgsl", include_str!("sky.wgsl")),
    ];

    /// The five definitions and two constants the stage owns, as they appear in a source.
    const OWNED: [&str; 7] = [
        "const DITHER_STEP",
        "fn srgb_encode",
        "fn srgb_decode",
        "fn bayer8",
        "fn dithered",
        "const LUMINANCE_WEIGHTS",
        "fn tone_map",
    ];

    /// **The display stage exists once, and every presenting pass is compiled with it.**
    ///
    /// What this pins is a **structure**, not a number: `srgb_encode`, `bayer8`,
    /// `dithered` and `tone_map` were written out three times, one per pass, with a
    /// comment in each explaining that WGSL has no `include`. Three transcriptions that
    /// nothing compares is exactly how a sky and the clouds standing in front of it come
    /// to disagree about what white is — and no test could see it, because each copy was
    /// self-consistent. So the assertions run in both directions: the stage holds each
    /// item once, no pass holds any of them, and each pass's composed source holds each
    /// of them once.
    #[test]
    fn the_display_stage_is_one_file_and_every_presenting_pass_receives_it() {
        let stage = include_str!("display.wgsl");
        for item in OWNED {
            assert_eq!(
                stage.matches(item).count(),
                1,
                "display.wgsl should define `{item}` exactly once"
            );
        }
        for (name, pass) in PRESENTING {
            for item in OWNED {
                assert_eq!(
                    pass.matches(item).count(),
                    0,
                    "{name} carries its own copy of `{item}` rather than receiving display.wgsl"
                );
                assert_eq!(
                    pass_source(pass).matches(item).count(),
                    1,
                    "{name} is not compiled with exactly one `{item}`"
                );
            }
        }
    }

    /// **The composed source is valid WGSL**, checked without an adapter.
    ///
    /// Concatenation is a build step, and a build step that produced invalid WGSL would
    /// otherwise fail on the first machine with a GPU: `create_shader_module` runs inside
    /// [`super::WorldPipeline::new`], which needs one. `wgpu` re-exports the front end it
    /// compiles with, so the parse runs here: a pass that no longer assembles is a red
    /// test rather than a black window.
    #[test]
    fn every_composed_pass_is_valid_wgsl() {
        for (name, pass) in PRESENTING {
            if let Err(error) = wgpu::naga::front::wgsl::parse_str(&pass_source(pass)) {
                panic!("{name} does not assemble once the display stage is in front of it:
{error}");
            }
        }
    }

    /// **The row count on both sides of the wire is one number**, read through the shader's own
    /// front end rather than matched as text: `atmosphere.wgsl` declares `SKY_MODEL_ROWS` and the
    /// host indexes the same rows, and a change to either alone is not a build error but a black
    /// sky — the shader's refusal answers a shorter model with no picture at all.
    #[test]
    fn the_sky_layout_is_one_row_count_declared_once_on_each_side() {
        let composed = pass_source(include_str!("shading.wgsl"));
        let module =
            wgpu::naga::front::wgsl::parse_str(&composed).expect("the world pass assembles");
        let (declared_by, declared) = module
            .constants
            .iter()
            .find(|(_, constant)| constant.name.as_deref() == Some("SKY_MODEL_ROWS"))
            .map(|(row_count, constant)| {
                let rows = match &module.global_expressions[constant.init] {
                    wgpu::naga::Expression::Literal(wgpu::naga::Literal::U32(rows)) => {
                        u64::from(*rows)
                    }
                    wgpu::naga::Expression::Literal(wgpu::naga::Literal::AbstractInt(rows)) => {
                        *rows as u64
                    }
                    other => panic!("`SKY_MODEL_ROWS` is not an integer literal: {other:?}"),
                };
                (row_count, rows)
            })
            .expect("atmosphere.wgsl declares the row count its refusal is written against");
        // **Read, not merely declared.** A constant nothing computes is the shape the text a
        // grep can find cannot tell apart from a refusal: what makes `arrayLength(&sky_model) <
        // SKY_MODEL_ROWS` a guard is that a pass's own expressions carry the handle.
        assert!(
            module.functions.iter().any(|(_, function)| function
                .expressions
                .iter()
                .any(|(_, expression)| matches!(
                    expression,
                    wgpu::naga::Expression::Constant(handle) if *handle == declared_by
                ))),
            "no pass computes the row count, so the shader's own refusal is a declaration"
        );
        assert_eq!(
            declared as usize,
            crate::skylight::SKY_MODEL_ROWS,
            "the shader's row count and the host's are two answers to one question"
        );
        assert_eq!(
            crate::skylight::SKY_MODEL_BYTES,
            declared as usize * 16,
            "the buffer the host binds is not the layout the shader's rows describe"
        );
    }

    /// **A shadow is the light's, this frame** — not a corner's remembered answer.
    ///
    /// A 2 m floor and a half-metre roof a metre above its far corner. The beam comes from
    /// `+X`, so the roof's shadow lands on the floor at `x ≈ 1.1`; the same point is lit when
    /// the light is mirrored — the map is redrawn from the new direction, not re-read from a
    /// cell — and lit again when the roof is removed, which is the edit path: new geometry,
    /// same light, new shadow.
    #[test]
    fn the_beam_shadows_the_floor_and_moves_with_the_light_and_the_world() {
        use crate::renderer::Renderer;
        use crate::scene::{Scene, SHADOW_LATTICE};
        use crate::shading::{GpuLight, GpuRig, MaterialParams};
        use voxel_mesh::{Mesh, MeshStats, Quad};
        use voxel_model::{ChunkPos, Face, MaterialId};

        let mesh = |faces: &[(u16, [u16; 2], [u16; 2])]| Mesh {
            chunk: ChunkPos::new(0, 0, 0),
            origin: [0, 0, 0],
            quads: faces
                .iter()
                .map(|(slice, uv, extent)| Quad {
                    face: Face::PosY,
                    slice: *slice,
                    uv: *uv,
                    extent: *extent,
                    material: MaterialId(0),
                })
                .collect(),
            loops: Vec::new(),
            stats: MeshStats {
                solid_voxels: 1,
                visible_faces: faces.len() as u64,
                quads: faces.len() as u64,
                loops: 0,
            },
        };
        let floor = mesh(&[(0, [0, 0], [8, 8])]);
        let roof = mesh(&[(4, [6, 6], [2, 2])]);

        let mut camera = crate::Camera::standard([1.0, 2.0, -1.0], 1.0);
        camera.pitch_rad = -0.55;
        // The field's own bands carry no diffuse, so the picture's only light is the beam and
        // the only thing that can darken it is the shadow map.
        let clear = |_: [f64; 3]| Some([1.0f32, 1.0, 1.0, 0.0, 0.0, 0.0]);
        let scene_with_roof = Scene::build_with_light_lattice(
            [&floor, &roof],
            camera.position_m,
            &clear,
            None,
            None,
            SHADOW_LATTICE,
        );
        let scene_without_roof = Scene::build_with_light_lattice(
            [&floor],
            camera.position_m,
            &clear,
            None,
            None,
            SHADOW_LATTICE,
        );

        let material = MaterialParams {
            base_color_and_present: [0.7, 0.7, 0.7, 1.0],
            rough_metal: [0.9, 1.0, 0.0, 0.0],
            ..MaterialParams::default()
        };
        let rig = |x: f32| {
            let mut rig = GpuRig::default();
            rig.count[0] = 1;
            rig.lights[0] = GpuLight {
                direction: [x, 1.0, 0.0, 1.0],
                colour: [1.0, 1.0, 1.0, 0.0],
            };
            rig
        };
        let (east, west) = (rig(0.5), rig(-0.5));
        let mut renderer = Renderer::new(96, 96, &[material], &east).expect("a GPU");
        // A frame with no sky states so; the subject is the floor, not the background.
        renderer.world_mut().write_no_sky_model();
        renderer.world_mut().set_shadows(true);
        let (width, height) = (96u32, 96u32);
        let at = |point: [f64; 3]| -> (u32, u32) {
            let matrix = camera.view_projection();
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
            let x = ((clip[0] / clip[3] * 0.5 + 0.5) * f64::from(width)) as u32;
            let y = ((1.0 - (clip[1] / clip[3] * 0.5 + 0.5)) * f64::from(height)) as u32;
            (x.min(width - 1), y.min(height - 1))
        };
        // A point under the roof's shadow from the east, and one clear of it. The shadow's edge
        // from this light lies at `x = 1.0`, so the sample sits 0.3 m inside it rather than on
        // the shoulder the 3 × 3 comparison would average.
        let shadowed = at([1.3, 0.0, 1.7]);
        let lit = at([0.3, 0.0, 1.0]);
        let brightest = |frame: &crate::Frame, (x, y): (u32, u32)| -> f64 {
            let mut best = 0.0f64;
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let sx = (x as i32 + dx).clamp(0, width as i32 - 1) as u32;
                    let sy = (y as i32 + dy).clamp(0, height as i32 - 1) as u32;
                    best = best.max(frame.luminance_at(sx, sy).unwrap_or(0.0));
                }
            }
            best
        };

        let frame = renderer.render(&scene_with_roof, &camera).expect("the east frame");
        let (dark, bright) = (brightest(&frame, shadowed), brightest(&frame, lit));
        assert!(bright > 0.0, "the floor's lit point is black: nothing was drawn");
        assert!(
            dark < bright * 0.5,
            "the roof cast no shadow: {dark:.3} under it against {bright:.3} beside it"
        );

        renderer.world_mut().write_rig(&west).expect("the west rig");
        let frame = renderer.render(&scene_with_roof, &camera).expect("the west frame");
        let moved = brightest(&frame, shadowed);
        assert!(
            moved > bright * 0.5,
            "the same point is still dark after the light moved: {moved:.3}"
        );

        renderer.world_mut().write_rig(&east).expect("the east rig");
        let frame = renderer.render(&scene_without_roof, &camera).expect("the open frame");
        let opened = brightest(&frame, shadowed);
        assert!(
            opened > bright * 0.5,
            "removing the roof did not resample the shadow: {opened:.3}"
        );
    }

    /// **The streamed horizon darkens the cell under a roof, with no shadow pass at all.**
    ///
    /// The roof is stamped into the light horizon's occupancy cube and **never drawn**: the
    /// scene is one floor, so neither the shadow map (off) nor the geometry can be what darkens
    /// the point. The only path left is the fragment's march through the cube, which is the
    /// streamed light horizon's real entry point.
    #[test]
    fn the_streamed_horizon_shadows_a_cell_the_shadow_pass_never_saw() {
        use crate::renderer::Renderer;
        use crate::scene::{Scene, SHADOW_LATTICE};
        use crate::shading::{GpuLight, GpuRig, MaterialParams};
        use voxel_mesh::{Mesh, MeshStats, Quad, SparseVoxelOctree};
        use voxel_model::{ChunkPos, Face, MaterialId};

        let floor = Mesh {
            chunk: ChunkPos::new(0, 0, 0),
            origin: [0, 0, 0],
            quads: vec![Quad { face: Face::PosY, slice: 0, uv: [0, 0], extent: [8, 8], material: MaterialId(0) }],
            loops: Vec::new(),
            stats: MeshStats { solid_voxels: 1, visible_faces: 1, quads: 1, loops: 0 },
        };
        // The same eye and the same two floor points the shadow test uses, so the pixels are
        // known to be in frame; the difference is that no shadow pass runs.
        let mut camera = crate::Camera::standard([1.0, 2.0, -1.0], 1.0);
        camera.pitch_rad = -0.55;
        // Until the clock moves, the clear is the field's own answer and the beam is the rig.
        let clear = |_: [f64; 3]| Some([1.0f32, 1.0, 1.0, 0.0, 0.0, 0.0]);
        let scene = Scene::build_with_light_lattice(
            [&floor], camera.position_m, &clear, None, None, SHADOW_LATTICE,
        );
        let material = MaterialParams {
            base_color_and_present: [0.7, 0.7, 0.7, 1.0],
            rough_metal: [0.9, 1.0, 0.0, 0.0],
            ..MaterialParams::default()
        };
        let mut rig = GpuRig::default();
        rig.count[0] = 1;
        rig.lights[0] = GpuLight { direction: [0.0, 1.0, 0.0, 1.0], colour: [1.0, 1.0, 1.0, 0.0] };
        let mut renderer = Renderer::new(96, 96, &[material], &rig).expect("a GPU");
        renderer.world_mut().write_no_sky_model();
        // **No shadow pass**, so the map cannot be the cause of the dark pixel.
        renderer.world_mut().set_shadows(false);
        renderer.world_mut().horizon_recenter(camera.position_m);
        let mut trees = SparseVoxelOctree::new();
        // A 4 x 4 m roof a voxel thick, three metres above the floor, over x,z in 1..5. Laid a
        // voxel at a time because `set_material_region` aligns its origin to its own level.
        for x in 1..5 {
            for z in 1..5 {
                trees.set_voxel(
                    voxel_model::LocalPos::new(x, 3, z).expect("inside the chunk"),
                    voxel_model::VoxelShape::solid(MaterialId(1)),
                );
            }
        }
        assert!(renderer.world_mut().horizon_stamp(ChunkPos::new(0, 0, 0), &trees, 1) > 0);

        let (width, height) = (96u32, 96u32);
        let to_pixel = |point: [f64; 3]| -> (u32, u32) {
            let matrix = camera.view_projection();
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
            let x = ((clip[0] / clip[3] * 0.5 + 0.5) * f64::from(width)) as u32;
            let y = ((1.0 - (clip[1] / clip[3] * 0.5 + 0.5)) * f64::from(height)) as u32;
            (x.min(width - 1), y.min(height - 1))
        };
        let frame = renderer.render(&scene, &camera).expect("a frame");
        let brightest = |point: [f64; 3]| -> f64 {
            let (x, y) = to_pixel(point);
            let mut best = 0.0f64;
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let sx = (x as i32 + dx).clamp(0, width as i32 - 1) as u32;
                    let sy = (y as i32 + dy).clamp(0, height as i32 - 1) as u32;
                    best = best.max(frame.luminance_at(sx, sy).unwrap_or(0.0));
                }
            }
            best
        };
        let roofed = brightest([1.3, 0.0, 1.7]);
        let open = brightest([0.3, 0.0, 1.0]);
        assert!(open > 0.0, "the floor is black: nothing was drawn");
        assert!(
            roofed < open * 0.5,
            "the streamed horizon did not shadow the roofed cell: {roofed:.3} under it against {open:.3} beside it"
        );
    }

    /// **A dug quarter lets its light through, and the CPU's own walk is the reference.**
    ///
    /// The occluder is one voxel with one subvoxel column left standing: three quarters of it have
    /// been taken out, which is the shape a player makes in three clicks. `voxel_stream::light`
    /// measures the two rays on the same tree — `0.0` through the column that is left, `1.0`
    /// through the quarters that are gone — and the picture must follow it, because a volume that
    /// marked the whole metre would shade the dug quarters too and the march would be a second,
    /// coarser shadow than the one this project measures on the CPU.
    #[test]
    fn a_partly_dug_voxel_shadows_its_columns_and_not_its_metre() {
        use crate::renderer::Renderer;
        use crate::scene::{Scene, SHADOW_LATTICE};
        use crate::shading::{GpuLight, GpuRig, MaterialParams};
        use voxel_mesh::{Mesh, MeshStats, Quad, SparseVoxelOctree};
        use voxel_model::{ChunkPos, Face, LocalPos, MaterialId, SubvoxelIndex, VoxelShape};
        use voxel_stream::light::{transmittance_tree, ChunkMatter};

        let floor = Mesh {
            chunk: ChunkPos::new(0, 0, 0),
            origin: [0, 0, 0],
            quads: vec![Quad { face: Face::PosY, slice: 0, uv: [0, 0], extent: [8, 8], material: MaterialId(0) }],
            loops: Vec::new(),
            stats: MeshStats { solid_voxels: 1, visible_faces: 1, quads: 1, loops: 0 },
        };
        let mut camera = crate::Camera::standard([1.0, 2.0, -1.0], 1.0);
        camera.pitch_rad = -0.55;
        let clear = |_: [f64; 3]| Some([1.0f32, 1.0, 1.0, 0.0, 0.0, 0.0]);
        let scene = Scene::build_with_light_lattice(
            [&floor], camera.position_m, &clear, None, None, SHADOW_LATTICE,
        );
        let material = MaterialParams {
            base_color_and_present: [0.7, 0.7, 0.7, 1.0],
            rough_metal: [0.9, 1.0, 0.0, 0.0],
            ..MaterialParams::default()
        };
        let mut rig = GpuRig::default();
        rig.count[0] = 1;
        rig.lights[0] = GpuLight { direction: [0.0, 1.0, 0.0, 1.0], colour: [1.0, 1.0, 1.0, 0.0] };
        let mut renderer = Renderer::new(128, 128, &[material], &rig).expect("a GPU");
        renderer.world_mut().write_no_sky_model();
        // **No shadow pass**, so the map cannot be what darkens a column or lets one through.
        renderer.world_mut().set_shadows(false);
        renderer.world_mut().horizon_recenter(camera.position_m);

        // One voxel three metres up, with only its `x = 0, z = 0` subvoxel column left: three
        // quarters dug out, which is `set_subvoxel` three times and not a region.
        let mut tree = SparseVoxelOctree::new();
        let voxel = LocalPos::new(1, 3, 1).expect("inside the chunk");
        let mut shape = VoxelShape::empty();
        for y in 0..4u8 {
            shape.set(SubvoxelIndex::new(0, y, 0).expect("a subvoxel"), MaterialId(1));
        }
        tree.set_voxel(voxel, shape);
        assert!(renderer.world_mut().horizon_stamp(ChunkPos::new(0, 0, 0), &tree, 1) > 0);

        // **The reference, on the same tree**: the two floor points differ only in which quarter
        // of one voxel stands above them.
        let matter = ChunkMatter::new(&tree, [0, 0, 0]);
        let above = |x: f64, z: f64| {
            transmittance_tree(&matter, &tree, [x, 0.001, z], [0.0, 1.0, 0.0], 8.0)
        };
        assert_eq!(above(1.125, 1.125), 0.0, "the reference let a beam through the standing column");
        assert_eq!(above(1.75, 1.125), 1.0, "the reference shadowed a dug quarter");

        let (width, height) = (128u32, 128u32);
        let to_pixel = |point: [f64; 3]| -> (u32, u32) {
            let matrix = camera.view_projection();
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
            let x = ((clip[0] / clip[3] * 0.5 + 0.5) * f64::from(width)) as u32;
            let y = ((1.0 - (clip[1] / clip[3] * 0.5 + 0.5)) * f64::from(height)) as u32;
            (x.min(width - 1), y.min(height - 1))
        };
        let frame = renderer.render(&scene, &camera).expect("a frame");
        let brightest = |point: [f64; 3]| -> f64 {
            let (x, y) = to_pixel(point);
            let mut best = 0.0f64;
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let sx = (x as i32 + dx).clamp(0, width as i32 - 1) as u32;
                    let sy = (y as i32 + dy).clamp(0, height as i32 - 1) as u32;
                    best = best.max(frame.luminance_at(sx, sy).unwrap_or(0.0));
                }
            }
            best
        };
        let under_column = brightest([1.125, 0.0, 1.125]);
        let under_dug = brightest([1.75, 0.0, 1.125]);
        assert!(under_dug > 0.0, "the floor is black: nothing was drawn");
        assert!(
            under_column < under_dug * 0.5,
            "the standing quarter did not shadow its own column: {under_column:.3} under it \
             against {under_dug:.3} under the dug quarter"
        );
    }
}
