//! **The beam's own shadow, drawn every frame** — the Sun's or the Moon's, whichever the rig
//! says is in the sky.
//!
//! # Why this exists beside the measured field
//!
//! The field answers a cell once and is kept until the beam swings past its own step
//! ([`crate::field::BEAM_STEP_DEG`]). That is the right grain for *ambient* — the sky does not
//! move between two measurements — but it is the wrong grain for a **sun shadow**: a shadow is
//! the world's own silhouette seen from the light, and at a metre's height a tenth of a degree
//! moves its edge by centimetres. Baked into corners, that motion arrives in steps; a chunk
//! edited under it keeps the light it had, because no cell was asked again.
//!
//! This pass draws the scene's own corners from the beam's direction and keeps the depth, so the
//! fragment asks the world what is between it and the light **this frame**. The shadow therefore
//! follows the Sun continuously, an edit changes it in the frame the new mesh is drawn, and no
//! measured cell has to be invalidated to make either true.
//!
//! # The two cascades, and the LOD
//!
//! A 2048² map of the whole shadow range would spend its texels on ground the eye never sees at
//! that size, and a 2048² map of a few metres would cut the shadow off at the player's feet. So
//! there are two:
//!
//! | cascade | range | texels | texel on the ground |
//! |---|---|---|---|
//! | near | 0–160 m | 1024² | 31 cm |
//! | far | 160–320 m | 1024² | 62 cm |
//!
//! The near radius is **the streamed window's own radius** — five chunks at
//! [`CHUNK_SIDE`](voxel_model::CHUNK_SIDE) metres — so every chunk the frame draws is inside the
//! detailed cascade rather than only the ground a few steps away. A pixel at 160 m is about 19 cm
//! across and the near texel is 31 cm (`Camera::standard`'s 70° over 288 rows is 1.2 mrad): about
//! 1.6 texels a pixel, the same ratio the old 28 m cascade held. That is the LOD: the same shadow detail per pixel near and far, rather than the
//! same metre. It is both distance- and projected-size-selected, because the split is a distance
//! and the resolutions were chosen from the pixel angle.
//!
//! # The split is a handover, not a switch
//!
//! Choosing one cascade by distance is a **step**: a surface crossing 160 m moves from 31 cm texels
//! and a 3 × 3 comparison a texel wide to 62 cm texels and one nine times wider, and the shadow
//! term jumps with it — a hard line across the ground at a distance the player can walk to. So the
//! fragment treats the two maps as **levels of one shadow**, the way the materials' pattern ladder
//! treats its rungs (`pattern_deviation`'s `weight_sum`): each cascade returns its comparison mean
//! *and the weight it can claim*, the near one's share falls from 1 to 0 across a band of
//! `SHADOW_BLEND_TEXELS` far-cascade texels either side of the split, and the answer is their
//! normalised mean. A pixel inside that band pays eighteen comparison taps instead of nine; either
//! side of it, nine, because each cascade is branched on its own share.
//! [`tests::the_cascade_seam_is_a_ramp`] is the witness: it renders the real maps, probes them
//! through the shipped function, and reports the
//! largest frame-to-frame change before and after, in shading units, in metres and in rows.
//!
//! **The near cascade reaches the streamed window's own radius** — five chunks, 160 m — so every
//! chunk a frame draws gets its direct shadow from the detailed map. The far cascade takes the
//! ground beyond it to the host's range, at least twice the near radius. Past the far range the
//! beam is drawn **unshadowed but current**, not held at the last cell a build happened to ask
//! about. [`SunShadow::write`]'s range argument is where a host that streams farther raises it.
//!
//! One 1024² depth map is 4 MB, so the pair is 8 MB of a pipeline — a cost a test that builds
//! dozens of them at once has to be able to carry.
//!
//! # What is not here
//!
//! The maps are **one sample per texel** and the fragment does a 3 × 3 comparison, so an edge is
//! one texel of penumbra, not a physically soft Sun. The field's diffuse measurement is still the
//! field's: this pass owns the beam and nothing else.

use crate::scene::Vertex;

/// Edge of one cascade's depth map, in texels.
pub const SHADOW_RESOLUTION: u32 = 1024;
/// Where the near cascade ends, metres from the eye — its **radius** across the ground as well
/// as its forward reach.
///
/// **It is the streamed window's radius**, not a distance chosen for the map: five chunks of
/// [`CHUNK_SIDE`](voxel_model::CHUNK_SIDE) metres is 160 m, and every mesh the frame draws from
/// the streamer is inside it. The map is 1024² over a 320 m box, so its texel is 31 cm — about
/// 1.6 drawn pixels at the edge, the ratio the 28 m cascade held at 5.5 cm. The far cascade takes
/// over beyond it (the host's range, at least twice this one).
pub const NEAR_RANGE_M: f32 = 160.0;
/// **The comparison's bias**, metres: the receiver is lifted along its own normal by this much
/// and the depth is compared as it stands. The shader reads it from `split.z`; a ground edge
/// moves by `SHADOW_BIAS_M / tan(elevation)` — 0.11 m at the shipped Sun's 10.6°. The value is
/// here so a witness can derive that ground term from the frame's own number rather than
/// restating 0.02.
pub const SHADOW_BIAS_M: f32 = 0.02;
/// Where the far cascade ends by default, metres from the eye — the shipped window's horizon.
pub(crate) const FAR_RANGE_M: f32 = 256.0;
/// **The cascade handover's width, in texels of the coarser map** — `shading.wgsl`'s
/// `SHADOW_BLEND_TEXELS`, restated for the witness and held to the shader by
/// [`tests::the_handover_constants_are_the_shaders_own`]. Test-only because nothing outside the
/// witness reads it: the frame reads the shader's own constant.
#[cfg(test)]
pub(crate) const BLEND_TEXELS: f32 = 2.0;
/// How far inside a map's border its weight ramps to zero, in that map's own texels —
/// `shading.wgsl`'s `SHADOW_EDGE_TEXELS`, restated for the same reason.
#[cfg(test)]
pub(crate) const EDGE_TEXELS: f32 = 2.0;
/// The light uniform: two matrices, the split, the cascade sizes, the beam.
const UNIFORM_BYTES: u64 = 176;

/// The four bindings the shadow pass adds to the world's own group.
///
/// **Bindings 6–9**, after the sky's transport at 5: the near depth, the far depth, the
/// comparison sampler and the matrices.
pub(crate) fn entries() -> [wgpu::BindGroupLayoutEntry; 4] {
    let depth = |binding: u32| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Depth,
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    };
    [
        depth(6),
        depth(7),
        wgpu::BindGroupLayoutEntry {
            binding: 8,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
            count: None,
        },
        wgpu::BindGroupLayoutEntry {
            binding: 9,
            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
    ]
}

/// **One frame's shadow, from the beam's direction** — the depth maps, the sampler that reads
/// them, the matrices the pass and the fragment share, and the two pipelines that write them.
pub(crate) struct SunShadow {
    /// **The maps are their views.** A `TextureView` owns a refcounted handle to the texture it
    /// was made from, so the pair is stored once: the pass draws into the view and the fragment
    /// binds the same handle.
    near_view: wgpu::TextureView,
    far_view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    uniform: wgpu::Buffer,
    /// **The shadow pass's own layout**, not the world's. The pass reads the origins and the
    /// matrices and nothing else — and it must not bind the map it is writing, which is exactly
    /// what a shared layout's texture entries would do.
    layout: wgpu::BindGroupLayout,
    near_pipeline: wgpu::RenderPipeline,
    far_pipeline: wgpu::RenderPipeline,
}

impl SunShadow {
    /// The textures, sampler and pipelines, once per pipeline — not once per frame.
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let texture = |label: &str| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: SHADOW_RESOLUTION,
                    height: SHADOW_RESOLUTION,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: crate::DEPTH_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
        };
        let near = texture("sun shadow near");
        let far = texture("sun shadow far");
        let near_view = near.create_view(&wgpu::TextureViewDescriptor::default());
        let far_view = far.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("sun shadow"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            // **`LessEqual` is the shadow test**: a fragment is lit when its own light-space
            // depth is at or in front of what the map holds. `Greater` would invert the world.
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sun shadow"),
            size: UNIFORM_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sun shadow"),
            entries: &[
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
                wgpu::BindGroupLayoutEntry {
                    binding: 9,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sun shadow"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sun shadow"),
            source: wgpu::ShaderSource::Wgsl(
                crate::world::pass_source(include_str!("shading.wgsl")).into(),
            ),
        });
        let attributes = Vertex::attributes();
        let pipeline = |label: &'static str, entry: &'static str| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some(entry),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: Vertex::STRIDE,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &attributes,
                    })],
                    compilation_options: Default::default(),
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    cull_mode: None,
                    ..Default::default()
                },
                // A shadow map is a depth and nothing else: no colour target, no blend, no
                // fragment stage. The bias is the map's own; the shader adds the normal offset
                // that a constant bias cannot express on a sloped face.
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: crate::DEPTH_FORMAT,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::Less),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState {
                        constant: 2,
                        slope_scale: 2.0,
                        clamp: 0.0,
                    },
                }),
                multisample: wgpu::MultisampleState::default(),
                fragment: None,
                multiview_mask: None,
                cache: None,
            })
        };
        Self {
            near_view,
            far_view,
            sampler,
            uniform,
            layout,
            near_pipeline: pipeline("sun shadow near", "vs_shadow_near"),
            far_pipeline: pipeline("sun shadow far", "vs_shadow_far"),
        }
    }

    /// The four entries the world's bind group adds — see [`entries`].
    pub(crate) fn bind_entries(&self) -> [wgpu::BindGroupEntry<'_>; 4] {
        [
            wgpu::BindGroupEntry {
                binding: 6,
                resource: wgpu::BindingResource::TextureView(&self.near_view),
            },
            wgpu::BindGroupEntry {
                binding: 7,
                resource: wgpu::BindingResource::TextureView(&self.far_view),
            },
            wgpu::BindGroupEntry {
                binding: 8,
                resource: wgpu::BindingResource::Sampler(&self.sampler),
            },
            wgpu::BindGroupEntry {
                binding: 9,
                resource: self.uniform.as_entire_binding(),
            },
        ]
    }

    /// **The frame's own beam.** `sun` is `(toward the light xyz, intensity)` from the rig the
    /// frame draws under; `None` or a zero intensity is a frame with no beam to cast, and the
    /// shader is told so rather than left with last frame's direction.
    pub(crate) fn write(
        &self,
        queue: &wgpu::Queue,
        camera: &crate::Camera,
        sun: Option<[f32; 4]>,
        far_range: f32,
    ) {
        // A range narrower than two near cascades would leave the far map smaller than the near
        // one and the split inside the near cascade; the halving is the smallest that still
        // describes two cascades.
        let far_range = far_range.max(NEAR_RANGE_M * 2.0);
        let (toward, enabled) = match sun {
            Some(light)
                if light[3] > 0.0
                    && light[0] * light[0] + light[1] * light[1] + light[2] * light[2] > 1e-8 =>
            {
                (
                    normalise([light[0] as f64, light[1] as f64, light[2] as f64]),
                    1.0f32,
                )
            }
            _ => ([0.0, 1.0, 0.0], 0.0),
        };
        let (_, _, forward) = camera.basis();
        let near = cascade(forward, toward, f64::from(NEAR_RANGE_M));
        let far = cascade(forward, toward, f64::from(far_range));
        let mut bytes = Vec::with_capacity(UNIFORM_BYTES as usize);
        for matrix in [near, far] {
            for column in matrix {
                for value in column {
                    bytes.extend_from_slice(&voxel_model::endian::le_bytes(value));
                }
            }
        }
        for value in [
            NEAR_RANGE_M,
            far_range,
            // **The comparison's bias, in metres along the receiver's normal** — one number at
            // both cascades. It was 1.5 *texels*, which is 0.56 m of the shipped 192 m far
            // cascade and, with the shader's slope term, metres of ground at a grazing beam;
            // the 2 cm normal offset moves the edge by 0.11 m at the shipped 10.6° sun and keeps
            // a surface off its own map. The shader applies it once, as a lift; it does not also
            // subtract it from the depth.
            SHADOW_BIAS_M,
            // The fade's width at the far end, in metres: the shadow reaches 1 across the last
            // 24 m rather than ending on a line the eye can find.
            24.0,
            NEAR_RANGE_M,
            far_range,
            SHADOW_RESOLUTION as f32,
            SHADOW_RESOLUTION as f32,
            toward[0] as f32,
            toward[1] as f32,
            toward[2] as f32,
            enabled,
        ] {
            bytes.extend_from_slice(&voxel_model::endian::le_bytes(value));
        }
        queue.write_buffer(&self.uniform, 0, &bytes);
    }

    /// **Draw the scene's own corners from the beam.** Two depth-only passes, one per cascade,
    /// with the same vertex and index buffers the colour pass draws.
    ///
    /// `enabled` is the frame's beam: a frame with no light draws nothing and the shader's own
    /// flag answers 1, so the world is lit exactly as it was before this pass existed.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        origins: &wgpu::Buffer,
        vertices: &wgpu::Buffer,
        indices: &wgpu::Buffer,
        drawn: u32,
        enabled: bool,
    ) {
        if !enabled || drawn == 0 {
            return;
        }
        // **The pass's own bind group**, with the origins and the matrices: the map being
        // written is not bound, so the near pass and the far pass never read their own target.
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sun shadow"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: origins.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 9,
                    resource: self.uniform.as_entire_binding(),
                },
            ],
        });
        for (label, view, pipeline) in [
            ("sun shadow near", &self.near_view, &self.near_pipeline),
            ("sun shadow far", &self.far_view, &self.far_pipeline),
        ] {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(label),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view,
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
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.set_vertex_buffer(0, vertices.slice(..));
            pass.set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..drawn, 0, 0..1);
        }
    }
}

/// **The light's camera and its box**, eye-relative: the centre is `forward · half/2` in front of
/// the eye, the light stands `2 · half` along `toward` from it, and the orthographic box is
/// `± half` about the centre with depth `0.1 … 4 · half`.
fn cascade(forward: [f64; 3], toward: [f64; 3], half: f64) -> [[f32; 4]; 4] {
    let center = [
        forward[0] * half * 0.5,
        forward[1] * half * 0.5,
        forward[2] * half * 0.5,
    ];
    let eye = [
        center[0] + toward[0] * half * 2.0,
        center[1] + toward[1] * half * 2.0,
        center[2] + toward[2] * half * 2.0,
    ];
    // Looking from the light at the centre: `forward` is the beam's own opposite, and it is
    // never parallel to world up except at the zenith, where `+Z` is the stable substitute.
    let view_forward = [-toward[0], -toward[1], -toward[2]];
    let up = if view_forward[1].abs() > 0.99 {
        [0.0, 0.0, 1.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let right = normalise(cross(view_forward, up));
    let up = cross(right, view_forward);
    let view = [
        [right[0] as f32, up[0] as f32, -view_forward[0] as f32, 0.0],
        [right[1] as f32, up[1] as f32, -view_forward[1] as f32, 0.0],
        [right[2] as f32, up[2] as f32, -view_forward[2] as f32, 0.0],
        [
            -dot(right, eye) as f32,
            -dot(up, eye) as f32,
            dot(view_forward, eye) as f32,
            1.0,
        ],
    ];
    let near = 0.1f64;
    let far = half * 4.0;
    let projection = [
        [(1.0 / half) as f32, 0.0, 0.0, 0.0],
        [0.0, (1.0 / half) as f32, 0.0, 0.0],
        [0.0, 0.0, (-1.0 / (far - near)) as f32, 0.0],
        [0.0, 0.0, (-near / (far - near)) as f32, 1.0],
    ];
    multiply(projection, view)
}

fn normalise(v: [f64; 3]) -> [f64; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    [v[0] / len, v[1] / len, v[2] / len]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Column-major `a · b`, the same multiplication `camera.rs` uses.
fn multiply(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut out = [[0.0f32; 4]; 4];
    for (c, column) in out.iter_mut().enumerate() {
        for (r, cell) in column.iter_mut().enumerate() {
            let mut sum = 0.0f32;
            for k in 0..4 {
                sum += a[k][r] * b[c][k];
            }
            *cell = sum;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clip(matrix: [[f32; 4]; 4], point: [f64; 3]) -> [f64; 3] {
        let mut out = [0.0f64; 4];
        for (row, slot) in out.iter_mut().enumerate() {
            for (column, value) in point.iter().enumerate() {
                *slot += matrix[column][row] as f64 * value;
            }
            *slot += matrix[3][row] as f64;
        }
        [out[0] / out[3], out[1] / out[3], out[2] / out[3]]
    }

    /// **The box is where it says it is**: the centre lands in the middle of the map and a
    /// half-metre off the centre stays inside it.
    #[test]
    fn the_cascade_centres_on_the_eye_and_covers_its_own_half_width() {
        let forward = [0.0, 0.0, 1.0];
        let toward = [0.0, 1.0, 0.0];
        let half = 28.0;
        let matrix = cascade(forward, toward, half);
        let centre = [0.0, 0.0, half * 0.5];
        let ndc = clip(matrix, centre);
        assert!(
            ndc[0].abs() < 1e-3 && ndc[1].abs() < 1e-3,
            "centre is {ndc:?}"
        );
        assert!((0.0..=1.0).contains(&ndc[2]), "centre depth is {}", ndc[2]);
        let edge = [half, 0.0, half * 0.5];
        let ndc = clip(matrix, edge);
        // The light's own right axis is `forward × up`, which lays `+X` at **−1** here: the box
        // is symmetric, so the edge is one of the two sides rather than a named one.
        assert!((ndc[0].abs() - 1.0).abs() < 1e-3, "half a width is {ndc:?}");
        assert!((ndc[1] - 0.0).abs() < 1e-3, "and not up the map: {ndc:?}");
    }

    /// **The light looks down its own beam**: a point in front of the light is nearer the map's
    /// near plane than one behind it.
    #[test]
    fn depth_runs_along_the_beam() {
        let matrix = cascade([0.0, 0.0, 1.0], [0.0, 1.0, 0.0], 28.0);
        let at_the_centre = clip(matrix, [0.0, 0.0, 14.0])[2];
        let toward_the_light = clip(matrix, [0.0, 20.0, 14.0])[2];
        assert!(
            toward_the_light < at_the_centre,
            "{toward_the_light} is not in front of {at_the_centre}"
        );
    }

    /// **The two handover constants are the shader's own**, read out of the WGSL rather than
    /// trusted: the witness sizes its band from the Rust side, and a shader edited alone would
    /// make the report describe a band the frame does not draw.
    #[test]
    fn the_handover_constants_are_the_shaders_own() {
        // **`horizon.wgsl`'s, not `shading.wgsl`'s** — the comment beside the trace in `shading.wgsl`
        // says so, and the declaration lives in the pass that draws the handover. This test read the
        // wrong file for as long as the constant has lived in the other one, which is why it failed
        // with "not declared" rather than with a value: a greps-the-shader test is only as good as
        // the file it names.
        let source = include_str!("horizon.wgsl");
        let declared = |name: &str| -> f32 {
            let at = source
                .find(name)
                .unwrap_or_else(|| panic!("{name} is not declared in horizon.wgsl"));
            let value = source[at..]
                .split('=')
                .nth(1)
                .expect("a declaration has a value")
                .trim_start();
            let digits: String = value
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            digits.parse().expect("the value is a literal")
        };
        assert_eq!(declared("const SHADOW_BLEND_TEXELS"), BLEND_TEXELS);
        assert_eq!(declared("const SHADOW_EDGE_TEXELS"), EDGE_TEXELS);
    }

    // ---- the seam witness -------------------------------------------------------------

    /// The eye's height for the sweep, metres — a standing player's.
    const EYE_M: f32 = 1.7;
    /// The probe lattice, addressed by pixel: `(columns, rows)`.
    const SEAM_COLS: u32 = 96;
    const SEAM_ROWS: u32 = 801;
    /// What the lattice spans: lateral offset from the eye, then distance from it, metres.
    const SEAM_X: (f32, f32) = (-5.0, 2.0);
    const SEAM_D: (f32, f32) = (152.0, 168.0);
    /// The fastest a player crosses the seam, m/s, and the frame rate it is crossed at.
    const WALK_M_PER_S: f64 = 5.6;
    const FRAME_HZ: f64 = 60.0;
    /// **The bound the witness holds the handover to**, in the shadow term per frame — the largest
    /// change a 60 Hz frame at [`WALK_M_PER_S`] may make in a pixel's lit fraction.
    const SEAM_BOUND: f64 = 0.10;

    /// One caster as a closed box in the pass's own vertex format, `y` from the ground to 6 m.
    /// The top edge's shadow lands `6 · tan(elevation)` to the −X side, which is where the
    /// lattice is.
    fn box_caster(x0: f32, x1: f32, z0: f32, z1: f32, out: &mut Vec<u8>, index: &mut Vec<u32>) {
        let (y0, y1) = (0.0f32, 6.0f32);
        let p = [
            [x0, y0, z0],
            [x1, y0, z0],
            [x1, y1, z0],
            [x0, y1, z0],
            [x0, y0, z1],
            [x1, y0, z1],
            [x1, y1, z1],
            [x0, y1, z1],
        ];
        let faces: [([usize; 4], [f32; 3]); 6] = [
            ([0, 3, 2, 1], [0.0, 0.0, -1.0]),
            ([4, 5, 6, 7], [0.0, 0.0, 1.0]),
            ([0, 4, 7, 3], [-1.0, 0.0, 0.0]),
            ([1, 2, 6, 5], [1.0, 0.0, 0.0]),
            ([0, 1, 5, 4], [0.0, -1.0, 0.0]),
            ([3, 7, 6, 2], [0.0, 1.0, 0.0]),
        ];
        for (corners, normal) in faces {
            let base = (out.len() / Vertex::STRIDE as usize) as u32;
            for corner in corners {
                let mut vertex = Vertex::blank();
                vertex.position = p[corner];
                vertex.normal = normal;
                vertex.write_bytes(out);
            }
            index.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }

    /// **The seam, as a number.** Renders the real depth maps through [`SunShadow::render`] for one
    /// eye, then renders a probe lattice of ground points through the **shipped** `sun_visibility`
    /// — the same function `fs_main` calls — beside the law it replaced: one cascade chosen by the
    /// split, no band. Same view, same maps, same uniform; the only difference is the handover.
    ///
    /// The sweep is distance from the eye, which is what a point on the ground sees as the player
    /// walks: a metre of walk is a metre of distance, so a row-to-row step *is* the frame-to-frame
    /// step at that frame's motion.
    ///
    /// The casters are two long boxes whose ground shadows run the whole length of the sweep — so
    /// nothing in the shadow varies along a column except the cascade — with a **slit** between
    /// them: 1.2 m of lit ground that the near map's 31 cm texels resolve and the far map's 62 cm
    /// ones cannot. That disagreement is the pop a cascade switch makes, and it is what the numbers
    /// below are about.
    #[test]
    fn the_cascade_seam_is_a_ramp() {
        let (device, queue, adapter) = crate::renderer::offscreen_device().expect("a GPU");
        println!("adapter: {adapter}");
        let shadow = SunShadow::new(&device);

        let eye = [0.0f64, f64::from(EYE_M), 0.0];
        let camera = crate::Camera::standard(eye, 1.0);
        // The beam: 29° above the horizon and swung a little along +Z, so a top edge 6 m up lays
        // its shadow 3.3 m to the −X side.
        let toward = [0.55f32, 1.0, 0.15];

        let mut vertex_bytes = Vec::new();
        let mut indices = Vec::new();
        box_caster(-0.5, 0.5, 10.0, 170.0, &mut vertex_bytes, &mut indices);
        // The second box is placed so the lit slit between the two shadows is **half a metre** —
        // between the new near map's 31 cm and the far map's 62 cm, which is what a cascade
        // handover is for. At the old 28 m radius the same role was played by a 1.2 m slit.
        box_caster(1.0, 1.6, 10.0, 170.0, &mut vertex_bytes, &mut indices);

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("seam caster"),
            size: vertex_bytes.len() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&vertex_buffer, 0, &vertex_bytes);
        let mut index_bytes = Vec::new();
        for i in &indices {
            index_bytes.extend_from_slice(&voxel_model::endian::le_bytes(*i));
        }
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("seam caster indices"),
            size: index_bytes.len() as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&index_buffer, 0, &index_bytes);
        let mut origin_bytes = Vec::new();
        for v in [0.0f32, -EYE_M, 0.0, 0.0] {
            origin_bytes.extend_from_slice(&voxel_model::endian::le_bytes(v));
        }
        let origins = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("seam origin"),
            size: origin_bytes.len() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&origins, 0, &origin_bytes);

        shadow.write(
            &queue,
            &camera,
            Some([toward[0], toward[1], toward[2], 1.0]),
            FAR_RANGE_M,
        );

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("seam witness"),
        });
        shadow.render(
            &device,
            &mut encoder,
            &origins,
            &vertex_buffer,
            &index_buffer,
            indices.len() as u32,
            true,
        );

        // The probe: the same four bindings the fragment uses, plus its own lattice uniform.
        let grid_x = (SEAM_X.1 - SEAM_X.0) / (SEAM_COLS - 1) as f32;
        let grid_d = (SEAM_D.1 - SEAM_D.0) / (SEAM_ROWS - 1) as f32;
        let mut probe_bytes = Vec::new();
        for v in [SEAM_X.0, grid_x, SEAM_D.0, grid_d, -EYE_M, 0.0, 0.0, 0.0] {
            probe_bytes.extend_from_slice(&voxel_model::endian::le_bytes(v));
        }
        let probe_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("seam probe"),
            size: probe_bytes.len() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&probe_uniform, 0, &probe_bytes);

        let shadow_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("seam probe shadow"),
            entries: &entries(),
        });
        let probe_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("seam probe grid"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("seam probe"),
            bind_group_layouts: &[Some(&shadow_layout), Some(&probe_layout)],
            immediate_size: 0,
        });
        let shadow_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("seam probe shadow"),
            layout: &shadow_layout,
            entries: &shadow.bind_entries(),
        });
        let grid_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("seam probe grid"),
            layout: &probe_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: probe_uniform.as_entire_binding(),
            }],
        });

        let source = format!(
            "{}\n{}",
            crate::world::pass_source(include_str!("shading.wgsl")),
            include_str!("shadow_seam.wgsl")
        );
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("seam probe"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let format = wgpu::TextureFormat::Rgba32Float;
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("seam probe"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_probe"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_probe"),
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

        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("seam probe target"),
            size: wgpu::Extent3d {
                width: SEAM_COLS,
                height: SEAM_ROWS,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("seam probe"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &shadow_bind_group, &[]);
            pass.set_bind_group(1, &grid_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        // 96 columns of 16 bytes is 1,536 — already a 256-byte row, so no padding.
        let row_bytes = SEAM_COLS * 16;
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("seam probe readback"),
            size: (row_bytes * SEAM_ROWS) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &target,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row_bytes),
                    rows_per_image: Some(SEAM_ROWS),
                },
            },
            wgpu::Extent3d {
                width: SEAM_COLS,
                height: SEAM_ROWS,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(Some(encoder.finish()));

        let slice = readback.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .expect("the readback completed");
        receiver
            .recv()
            .expect("the map callback ran")
            .expect("the readback mapped");
        let data = slice.get_mapped_range().expect("the mapped bytes");

        let stride = row_bytes as usize;
        let sample = |col: usize, row: usize, channel: usize| -> f64 {
            let at = row * stride + col * 16 + channel * 4;
            let bytes = [data[at], data[at + 1], data[at + 2], data[at + 3]];
            f64::from(voxel_model::endian::le_from_bytes::<f32>(&bytes))
        };

        let cols = SEAM_COLS as usize;
        let rows = SEAM_ROWS as usize;
        let frame_rows = ((WALK_M_PER_S / FRAME_HZ / f64::from(grid_d)).round() as usize).max(1);
        let far_texel_m = 2.0 * FAR_RANGE_M / SHADOW_RESOLUTION as f32;
        let band_m = 2.0 * BLEND_TEXELS * far_texel_m;
        let band = f64::from(band_m);
        let seam_m = f64::from(NEAR_RANGE_M);
        let dist_of = |row: usize| f64::from(SEAM_D.0 + grid_d * row as f32);

        // **Where the handover is the only thing moving.** A column that is not monotone across
        // the band is one where the map's own texel quantisation is flipping taps under the
        // fragment as the sample point slides through it — a 1/9 step, present before this change
        // and inherent to a nine-tap comparison, not the seam. The handover's own rate is measured
        // on the columns where the term crosses the band one way.
        let band_from =
            ((seam_m - band - f64::from(SEAM_D.0)) / f64::from(grid_d)).max(0.0) as usize;
        let band_to =
            (((seam_m + band - f64::from(SEAM_D.0)) / f64::from(grid_d)) as usize).min(rows - 1);
        let mut quiet = vec![false; cols];
        for (col, flag) in quiet.iter_mut().enumerate() {
            let mut sign = 0.0f64;
            let mut monotone = true;
            for row in band_from..band_to {
                let step = sample(col, row + 1, 1) - sample(col, row, 1);
                if step.abs() > 1e-6 {
                    if sign == 0.0 {
                        sign = step.signum();
                    } else if step.signum() != sign {
                        monotone = false;
                    }
                }
            }
            *flag = monotone;
        }

        let mut per_row = [(0.0f64, 0usize, 0usize); 2];
        let mut per_frame = [(0.0f64, 0usize, 0usize); 2];
        let mut handover = [(0.0f64, 0usize, 0usize); 2];
        let mut outside = [0.0f64; 2];
        for col in 0..cols {
            for row in 0..rows - 1 {
                for law in 0..2 {
                    let step = (sample(col, row + 1, law) - sample(col, row, law)).abs();
                    if step > per_row[law].0 {
                        per_row[law] = (step, col, row);
                    }
                    if row + frame_rows < rows {
                        let step =
                            (sample(col, row + frame_rows, law) - sample(col, row, law)).abs();
                        if step > per_frame[law].0 {
                            per_frame[law] = (step, col, row);
                        }
                        let a = dist_of(row);
                        let b = dist_of(row + frame_rows);
                        if a < seam_m - band && b < seam_m - band
                            || a > seam_m + band && b > seam_m + band
                        {
                            outside[law] = outside[law].max(step);
                        }
                    }
                }
            }
            if quiet[col] {
                for row in band_from..band_to {
                    if row + frame_rows >= rows {
                        break;
                    }
                    let step = (sample(col, row + frame_rows, 1) - sample(col, row, 1)).abs();
                    if step > handover[1].0 {
                        handover[1] = (step, col, row);
                    }
                }
            }
        }
        let pixel_angle = crate::pattern::pixel_angle_rad(70f64.to_radians(), 288);
        let band_rows = band / (seam_m * f64::from(pixel_angle));
        let frame_m = WALK_M_PER_S / FRAME_HZ;
        let x_of = |col: usize| SEAM_X.0 + grid_x * col as f32;
        println!(
            "\nseam at {NEAR_RANGE_M} m, eye {EYE_M} m, lattice {cols} x {rows} ({:.0} mm rows)",
            f64::from(grid_d) * 1000.0
        );
        println!(
            "handover: {band_m:.2} m wide ({} far texels) = {band_rows:.0} rows of a 288-row frame at {NEAR_RANGE_M} m\n\
             sampling: {:.3} m between rows, {frame_m:.3} m per frame at {WALK_M_PER_S} m/s, {FRAME_HZ:.0} Hz",
            2.0 * BLEND_TEXELS,
            f64::from(grid_d),
        );
        println!("\nlaw      worst frame step   at (x, d)");
        println!(
            "before   {:>12.4}     x {:+.2} m, d {:.2} m  — one cascade, chosen by the split",
            per_frame[0].0,
            x_of(per_frame[0].1),
            dist_of(per_frame[0].2),
        );
        println!(
            "after    {:>12.4}     x {:+.2} m, d {:.2} m  — every column",
            per_frame[1].0,
            x_of(per_frame[1].1),
            dist_of(per_frame[1].2),
        );
        println!(
            "after    {:>12.4}     x {:+.2} m, d {:.2} m  — columns where the handover is all that moves",
            handover[1].0,
            x_of(handover[1].1),
            dist_of(handover[1].2),
        );
        println!(
            "the map's own texel floor outside the band: {:.4} before, {:.4} after — a 1/9 tap step,\n\
             which the handover neither causes nor removes",
            outside[0], outside[1]
        );
        assert!(
            (outside[0] - outside[1]).abs() < 1e-6,
            "the handover moved the frame outside its band: {:.6} before, {:.6} after",
            outside[0],
            outside[1]
        );

        // **The seam itself**, on the column where the step is largest: the law as it was is a
        // cliff, and the law as it is crosses the same metres in a ramp.
        let col = per_frame[0].1;
        let seam_row = ((seam_m - f64::from(SEAM_D.0)) / f64::from(grid_d)).round() as usize;
        println!(
            "\ncolumn x {:+.2} m across the seam (before -> after):",
            SEAM_X.0 + grid_x * col as f32
        );
        let mut row = seam_row.saturating_sub(60);
        while row <= seam_row + 60 && row < rows {
            println!(
                "  d {:6.2} m   {:.3}   {:.3}",
                SEAM_D.0 + grid_d * row as f32,
                sample(col, row, 0),
                sample(col, row, 1),
            );
            row += 12;
        }

        // **The cliff is half the term, not the whole of it.** The near map covers 160 m now, so
        // its texel is 31 cm against the far map's 62 cm: a feature between the two is resolved
        // by one cascade and not the other, and the one-rung step is about 0.49 of the shadow
        // term. The old 28 m near map's 5.5 cm texels made the same step larger; the bound is the
        // measured cliff, not a constant from the old radius.
        assert!(
            per_frame[0].0 > 0.45,
            "the witness found no pop to remove: the step law moves {:.4} per frame",
            per_frame[0].0
        );
        assert!(
            handover[1].0 <= SEAM_BOUND,
            "the handover moves {:.4} of the shadow term in one frame, above the bound of \
             {SEAM_BOUND}: a {band_m:.2} m band is too narrow for a {frame_m:.3} m frame step",
            handover[1].0
        );
        assert!(
            handover[1].0 * 4.0 < per_frame[0].0,
            "the handover made the seam no quieter: {:.4} against {:.4}",
            handover[1].0,
            per_frame[0].0
        );
    }
}
