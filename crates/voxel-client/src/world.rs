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
use crate::skylight::{SKY_DOME_AZIMUTHS, SKY_DOME_ELEVATIONS, SKY_DOME_ENTRIES};
use crate::ClientError;

/// The quad format's two storage buffers start at one page each: a scene with no
/// geometry still binds a valid buffer, and `write_quads` doubles them only when
/// a window grows past them.
const QUAD_START_BYTES: u64 = 4096;
use voxel_appearance::LinearRgb;

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
fn pass_source(pass: &str) -> String {
    format!("{}
{}", include_str!("display.wgsl"), pass)
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
    /// The sky pass: the table [`voxel_appearance::sky::Sky::dome_radiance`] filled on
    /// the CPU, and the two numbers that turn a pixel into a direction.
    ///
    /// **The table lives here rather than being computed per frame in the shader**
    /// because the scattering model exists once, in Rust. See `sky.wgsl`.
    sky_pipeline: wgpu::RenderPipeline,
    sky_bind_group: wgpu::BindGroup,
    sky_view: wgpu::Buffer,
    sky_table: wgpu::Buffer,
    /// **The Sun the sky pass draws** (C-176), held as the **world body** rather than as the
    /// eye-relative numbers the uniform carries.
    ///
    /// That is the whole reason this is a field and not an argument: the sky's own uniforms are
    /// uploaded **per frame** with the camera, so the body has to survive from the tick that
    /// derived it to the frames that draw it — and the body's centre relative to a moving eye
    /// is a per-frame number while the body itself is not. `None` draws no sun.
    sun: Option<crate::sun::DrawnSun>,
    /// **The quad format's own pipeline** (C-168): the same fragment shader, with
    /// a vertex stage that pulls a 24-byte rectangle out of a storage buffer and
    /// expands it. See `shading.wgsl`'s `vs_quad`.
    quad_pipeline: wgpu::RenderPipeline,
    /// **The cloud's own pipeline state** (C-179): the *same* shader module, layout and
    /// fragment — what differs is only what a veil needs and a surface must not have. The
    /// blend composites the cloud over the sky standing behind it by the veil the fragment
    /// writes as alpha, and depth is tested but **not written**, because a cell is a
    /// translucent box and a box that wrote depth would hide the cells behind it inside the
    /// same cloud.
    cloud_pipeline: wgpu::RenderPipeline,
    /// The rectangles, and one eye-relative origin per chunk they name. Both are
    /// held rather than created per frame, which is the point of the format: a
    /// rectangle does not change when the camera moves.
    quad_records: wgpu::Buffer,
    quad_origins: wgpu::Buffer,
    quad_record_capacity: u64,
    quad_origin_capacity: u64,
    /// **What is currently on the GPU, held as values rather than as bytes.**
    /// A rectangle is camera-independent and the mesh set changes only when a
    /// chunk arrives, is evicted, or is remeshed, so in the steady state the whole
    /// of the per-frame cost is this comparison — and comparing 24-byte records
    /// directly is a `memcmp`, where comparing their serialisation would mean
    /// building and throwing away 1.13 MB of little-endian bytes every frame to
    /// discover that nothing moved.
    quad_record_last: Vec<QuadRecord>,
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
    /// The world's bind group layout, kept so a grown quad buffer can be
    /// re-bound without reconstructing the pipeline.
    world_layout: wgpu::BindGroupLayout,
}

impl WorldPipeline {
    /// `format` is the **colour target's** format, so a window passes its
    /// surface format and the offscreen renderer passes
    /// [`crate::TARGET_FORMAT`]. The depth format is fixed at
    /// [`crate::DEPTH_FORMAT`] for both, because a depth test that differed
    /// between the two would be exactly the kind of divergence this module
    /// exists to prevent.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        materials: &[MaterialParams],
        rig: &GpuRig,
    ) -> Result<Self, ClientError> {
        if materials.is_empty() {
            return Err(ClientError::UnknownMaterial { id: 0, rows: 0 });
        }

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("world"),
            entries: &[
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
                // **The same dome table the sky pass draws, bound to the world pass**
                // (C-177). A fluid surface *is* the sky, seen off a dielectric boundary: the
                // reflection is most of what water looks like, and until this binding existed
                // the world pass could not see the table that `sky.wgsl` was already
                // declaring a `fluid_f0` uniform for. Both passes bind the same buffer, so
                // there is one sky and one evaluation of it — the reason `sky.wgsl`'s own
                // doc gives for existing at all.
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
            multisample: wgpu::MultisampleState::default(),
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
            multisample: wgpu::MultisampleState::default(),
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
            multisample: wgpu::MultisampleState::default(),
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

        // **The dome table, created before either pass that reads it.** Both the sky pass
        // (which draws it) and the world pass (which reflects it off a fluid, C-177) bind this
        // one buffer, so it is made here rather than inside either pass's own section: a table
        // created for one of them and copied for the other would be two skies.
        //
        // Zeroed, not left undefined: a caller that has not written a sky yet is owed the
        // frame this pass drew before the sky existed, which is black.
        let sky_table = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sky-table"),
            size: (SKY_DOME_ENTRIES * 16) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&sky_table, 0, &vec![0u8; SKY_DOME_ENTRIES * 16]);

        // `write_quads` re-binds through the same function when a buffer grows,
        // so the six entries exist once.
        let bind_group = world_bind_group(
            device,
            &bind_group_layout,
            &rig_buffer,
            &view,
            &materials_buffer,
            &quad_records,
            &quad_origins,
            &sky_table,
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
                    binding: 1,
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
            multisample: wgpu::MultisampleState::default(),
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
            // **Twenty `f32`, and the count is the shader's.** The first eight are
            // the `SkyView` fields `sky.wgsl` was built with; the ninth is its dither
            // flag and the tenth its display flag, the same two the world pass
            // carries (C-166, C-167); the eleventh and twelfth are declared so the
            // first block is a whole number of `vec4`s rather than whatever `naga`
            // would round it up to; and the last eight are the Sun's body and its
            // radiance (C-176) — `vec3<f32>` and an `f32` twice over, so two more
            // whole `vec4`s. `write_camera` writes exactly these bytes and asserts the
            // length, so the shader and this upload cannot drift apart.
            size: 80,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        // **Zeroed, not left undefined.** A caller that never writes a sky gets a
        // black one, which is what the pass did before it existed — so adding the sky
        // cannot change a frame whose owner has not asked for a sky yet.
        queue.write_buffer(&sky_view, 0, &[0u8; 80]);
        let sky_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sky"),
            layout: &sky_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: sky_view.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: sky_table.as_entire_binding() },
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
            sky_pipeline,
            sky_bind_group,
            sky_view,
            sky_table,
            sun: None,
            quad_pipeline,
            cloud_pipeline,
            quad_records,
            quad_origins,
            quad_record_capacity: QUAD_START_BYTES,
            quad_origin_capacity: QUAD_START_BYTES,
            quad_record_last: Vec::new(),
            quad_origin_last: Vec::new(),
            quad_origin_eye: None,
            materials: materials_buffer,
            world_layout: bind_group_layout,
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
    pub fn write_rig(&self, rig: &GpuRig) -> Option<()> {
        if rig.count[0] as usize > crate::shading::RIG_CAPACITY {
            return None;
        }
        self.queue.write_buffer(&self.rig, 0, &rig.bytes());
        Some(())
    }

    /// How many rows the material table has. A scene naming a material beyond
    /// this is not shaded at all — the shader's own guard, not a wrap.
    pub fn rows(&self) -> u32 {
        self.rows
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
                bytes.extend_from_slice(&v.to_le_bytes());
            }
        }
        let phase = crate::pattern::eye_phase(camera.position_m);
        for v in [phase[0], phase[1], phase[2], 0.0f32] {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        let voxel = crate::pattern::eye_voxel(camera.position_m);
        for v in [voxel[0], voxel[1], voxel[2], 0i32] {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        for v in self.lattice {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        for v in self.rates {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        // `screen.xy`: one pixel's angular size, and the dither flag the fragment
        // pass reads before it writes (C-163, C-166). The two reserved slots stay
        // reserved, so the struct's size does not move when one is claimed.
        for v in [pixel_angle_rad, self.dither, self.tonemap, 0.0] {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        // `sky_grid`: the dome table's two sides and the water boundary's own
        // reflectance, which the world's fluid reflection reads (C-177). Sent rather
        // than typed into `shading.wgsl` for the reason `lattice` and `rates` are: the
        // grid is [`crate::skylight`]'s declaration and the reflectance is
        // `voxel_appearance::fluid`'s, and a shader holding a second copy of either is
        // a shader that can disagree with the table it is looking up.
        // **`w` is the cloud's own albedo** (C-179), the fourth number of this group and the
        // last slot `View` had left. A cloud has no material row — it has no surface, so there
        // is no row to have — and the one number its shading needs beyond the rig is how much
        // of the light a water droplet gives back. `cloud_field::CLOUD_ALBEDO` is that number,
        // a real figure for water droplets in the visible, and it is **sent** for the reason
        // every other constant here is: a shader holding a second copy of it could disagree.
        for v in [
            SKY_DOME_AZIMUTHS as f32,
            SKY_DOME_ELEVATIONS as f32,
            voxel_appearance::fluid::WATER_F0 as f32,
            crate::cloud_field::CLOUD_ALBEDO as f32,
        ] {
            bytes.extend_from_slice(&v.to_le_bytes());
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
            SKY_DOME_AZIMUTHS as f32,
            SKY_DOME_ELEVATIONS as f32,
            voxel_appearance::fluid::WATER_F0 as f32,
            // The sky's own dither flag, the same value the world pass carries in
            // `View::screen.y`: the sky is the largest smooth gradient on the
            // screen, so it is where the target's banding is loudest.
            self.dither,
            // And its display flag, `View::screen.z` — the sky is also the brightest
            // thing on the screen, and the one most likely to be more than white.
            self.tonemap,
            // The two the struct declares to close out its third `vec4`. Nothing
            // reads them; they exist so the shader's struct and this upload are the
            // same twelve `f32` without either having to know the other's padding.
            0.0f32,
            0.0f32,
            0.0f32,
        ] {
            sky_bytes.extend_from_slice(&v.to_le_bytes());
        }
        // **The Sun's body, relative to this camera** (C-176). The subtraction is made here
        // rather than at the upload because it is the one part of the Sun that depends on the
        // eye, and a caller that uploaded a body-relative-to-a-previous-eye would draw a sun
        // whose parallax belongs to a camera that is no longer on screen. Zeroed when no body
        // was written, and a zero half-extent is how the shader is told there is none — the
        // same refusal-by-absence the table and the rig already use.
        let (centre, half_extent, radiance) = match &self.sun {
            Some(sun) => (
                sun.centre_relative_to(camera.position_m),
                sun.half_extent_f32(),
                [sun.radiance.r as f32, sun.radiance.g as f32, sun.radiance.b as f32],
            ),
            None => ([0.0f32; 3], 0.0f32, [0.0f32; 3]),
        };
        for v in [centre[0], centre[1], centre[2], half_extent] {
            sky_bytes.extend_from_slice(&v.to_le_bytes());
        }
        for v in [radiance[0], radiance[1], radiance[2], 0.0f32] {
            sky_bytes.extend_from_slice(&v.to_le_bytes());
        }
        // The uniform is exactly the shader's `SkyView`, so the write is exactly the
        // twenty `f32` the struct declares. This is asserted *before* the upload
        // because a short or long write is a `wgpu` bounds refusal at runtime — the
        // one that used to read "32 bytes into a buffer of size 16".
        debug_assert_eq!(sky_bytes.len(), 80, "SkyView is twenty f32");
        self.queue.write_buffer(&self.sky_view, 0, &sky_bytes);
    }

    /// **Put the Sun in the sky the frames that follow draw**, or take it out.
    ///
    /// `None` is *no sun*, and it is what the pass draws until something asks for one — the
    /// same "a frame whose owner has not asked for a sky is unchanged" rule the zeroed table
    /// and the zeroed view already follow, which is what makes adding a body safe for every
    /// existing caller rather than a change they must all opt into.
    ///
    /// The body is kept as a **world** object and narrowed to the eye per frame in
    /// [`WorldPipeline::write_camera`]; see the field's own doc for why.
    pub fn write_sun(&mut self, sun: Option<crate::sun::DrawnSun>) {
        self.sun = sun;
    }

    /// Whether a frame would draw a sun, so a test can state what it expects the last upload
    /// to have decided rather than inferring it from pixels.
    pub fn has_sun(&self) -> bool {
        self.sun.is_some()
    }

    /// **Upload the sky's radiance for every direction**, already scaled by
    /// [`crate::skylight::SKY_EXPOSURE`].
    ///
    /// The exposure is applied **here**, at the upload, for the same reason it is
    /// applied to the rig at its upload and never inside [`crate::skylight::daylight_of`]:
    /// the model stays physical and only the thing that reaches the swapchain is
    /// scaled. A short table is refused rather than padded, because padding it would
    /// mean drawing a sky nobody asked for at the far end of the dome.
    pub fn write_sky(&self, table: &[LinearRgb]) -> Option<()> {
        if table.is_empty() || table.len() > SKY_DOME_ENTRIES {
            return None;
        }
        let mut bytes = Vec::with_capacity(table.len() * 16);
        for c in table {
            let exposed = crate::skylight::sky_exposed(*c);
            bytes.extend_from_slice(&(exposed.r as f32).to_le_bytes());
            bytes.extend_from_slice(&(exposed.g as f32).to_le_bytes());
            bytes.extend_from_slice(&(exposed.b as f32).to_le_bytes());
            bytes.extend_from_slice(&0.0f32.to_le_bytes());
        }
        self.queue.write_buffer(&self.sky_table, 0, &bytes);
        Some(())
    }

    fn upload(&self, label: &str, bytes: &[u8], usage: wgpu::BufferUsages) -> wgpu::Buffer {
        // `write_buffer` needs a multiple of 4, and an empty buffer is not a
        // buffer, so a scene with no geometry still gets something valid to bind
        // rather than a zero-sized allocation.
        let size = ((bytes.len().max(4) as u64) + 3) & !3u64;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size,
            usage: usage | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        if !bytes.is_empty() {
            self.queue.write_buffer(&buffer, 0, bytes);
        }
        buffer
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
        let wanted_records = (scene.records.len() * QuadRecord::SIZE as usize).max(4) as u64;
        if wanted_records > self.quad_record_capacity {
            self.quad_record_capacity = wanted_records.next_power_of_two();
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
            let rebound = world_bind_group(
                &self.device,
                &self.world_layout,
                &self.rig,
                &self.view,
                &self.materials,
                &self.quad_records,
                &self.quad_origins,
                &self.sky_table,
            );
            self.bind_group = rebound;
        }

        if scene.records != self.quad_record_last {
            self.queue.write_buffer(&self.quad_records, 0, &QuadRecord::bytes(&scene.records));
            self.quad_record_last.clear();
            self.quad_record_last.extend_from_slice(&scene.records);
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
    pub fn draw(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        colour: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        scene: &Scene,
        camera: &Camera,
        pixel_angle_rad: f32,
    ) {
        self.draw_impl(encoder, colour, depth, scene, camera, false, pixel_angle_rad);
    }

    /// **The world meshes' own wireframe**: the frame [`WorldPipeline::draw`] would draw, with
    /// the world's own materials **withheld** so the merge's rectangles are the subject.
    ///
    /// On `F4` in the game. The lines are the greedy merge's rectangles themselves —
    /// [`Scene::quad_edge_bytes`]'s four border edges per quad, no triangulation diagonal —
    /// shaded by `wire.wgsl`: a rainbow hue swept by distance (the user's own call, 2026-10-04,
    /// after a first white ink read poorly against the world; `NEAR_M`/`FAR_M`/`NEAR_HUE`/
    /// `FAR_HUE` are that shader's declared range). The sky still draws, so the lines have a
    /// horizon; the solid world does not, so nothing occludes the mesh being read.
    ///
    /// **Measuring is unaffected.** [`Renderer`](crate::Renderer) keeps calling
    /// [`WorldPipeline::draw`], so a measured frame carries no wireframe; while this is on it
    /// uploads the vertex buffer and one edge index buffer, and pays one indexed draw.
    pub fn draw_wireframe(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        colour: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        scene: &Scene,
        camera: &Camera,
        pixel_angle_rad: f32,
    ) {
        self.draw_impl(encoder, colour, depth, scene, camera, true, pixel_angle_rad);
    }

    fn draw_impl(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        colour: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        scene: &Scene,
        camera: &Camera,
        wireframe: bool,
        pixel_angle_rad: f32,
    ) {
        self.write_camera(camera, pixel_angle_rad);
        // **The rectangles and their origins, before the pass begins.** Both are
        // uploaded only when they changed — see [`WorldPipeline::write_quads`].
        self.write_quads(scene, camera.position_m);
        // **And the corner buffers only for a frame that carries corners.** A frame
        // in the quad format has no vertex and no index at all, so building them
        // here would be two `create_buffer` calls and two uploads of nothing per
        // frame — the very cost this format exists to remove. A frame in the vertex
        // format still pays it, and must: a light measurement attaches to a corner
        // and nowhere else, so that format has no chunk-local form to reuse.
        let corners = !scene.indices.is_empty();
        let vertex_buffer = corners
            .then(|| self.upload("vertices", &scene.vertex_bytes(), wgpu::BufferUsages::VERTEX));
        let index_buffer = corners
            .then(|| self.upload("indices", &scene.index_bytes(), wgpu::BufferUsages::INDEX));

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("world"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: colour,
                depth_slice: None,
                resolve_target: None,
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

        if let (Some(vertex_buffer), Some(index_buffer)) = (&vertex_buffer, &index_buffer) {
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..scene.indices.len() as u32, 0, 0..1);
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
        let world_records = scene.records.len() - scene.cloud_records;
        if world_records > 0 {
            pass.set_pipeline(&self.quad_pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.draw(0..6 * world_records as u32, 0..1);
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
        if scene.cloud_records > 0 {
            let first = 6 * world_records as u32;
            pass.set_pipeline(&self.cloud_pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.draw(first..first + 6 * scene.cloud_records as u32, 0..1);
        }
    }

    /// **GPU-side quad merge**: read the raw quads from a storage buffer, merge
    /// adjacent same-material quads into larger rectangles, and draw the result.
    ///
    /// This is the correct architecture for an RTX card: the mesher outputs
    /// quads to a storage buffer, a compute shader merges them, and the render
    /// pass draws the merged result. No CPU-side merging, no per-frame buffer
    /// allocation.
    ///
    /// The merge is done in two passes:
    /// 1. **Row merge**: merge horizontally adjacent quads in the same row
    /// 2. **Column merge**: merge vertically adjacent quads in the same column
    ///
    /// This is the standard GPU greedy meshing algorithm, and it produces
    /// optimal rectangles for axis-aligned quads.
    pub fn draw_merged(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        colour: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        scene: &Scene,
        camera: &Camera,
        pixel_angle_rad: f32,
    ) {
        self.write_camera(camera, pixel_angle_rad);
        let vertex_buffer =
            self.upload("vertices", &scene.vertex_bytes(), wgpu::BufferUsages::VERTEX);
        let index_buffer =
            self.upload("indices", &scene.index_bytes(), wgpu::BufferUsages::INDEX);

        // Upload quads to a storage buffer for the compute shader
        let _quads_buffer = self.upload(
            "quads",
            &scene.vertex_bytes(), // TODO: use actual quad data
            wgpu::BufferUsages::STORAGE,
        );

        // Run the merge compute shader
        let merge_module = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("merge"),
            source: wgpu::ShaderSource::Wgsl(include_str!("merge.wgsl").into()),
        });
        let merge_pipeline = self.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("merge"),
            layout: None,
            module: &merge_module,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("merge"),
            timestamp_writes: None,
        });
        compute_pass.set_pipeline(&merge_pipeline);
        compute_pass.set_bind_group(0, &self.bind_group, &[]);
        compute_pass.dispatch_workgroups((scene.quads as u32 + 63) / 64, 1, 1);
        drop(compute_pass);

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("world"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: colour,
                depth_slice: None,
                resolve_target: None,
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
        pass.set_pipeline(&self.sky_pipeline);
        pass.set_bind_group(0, &self.sky_bind_group, &[]);
        pass.draw(0..3, 0..1);

        if !scene.indices.is_empty() {
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..scene.indices.len() as u32, 0, 0..1);
        }
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
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("world"),
        layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: rig.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: view.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: materials.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 3, resource: records.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 4, resource: origins.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 5, resource: sky.as_entire_binding() },
        ],
    })
}

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

    /// **Every pass that presents a frame**, as `(file, source)`.
    ///
    /// These two write colour into the target, so these two receive the display stage.
    /// The merge compute pass and the wireframe are deliberately absent — and so is the
    /// cloud pass, which no longer exists: the clouds are geometry now.
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
}
