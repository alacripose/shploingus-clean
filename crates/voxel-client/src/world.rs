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
use crate::shading::{GpuRig, MaterialParams};
use crate::skylight::SKY_LUT_ROWS;
use crate::ClientError;
use voxel_appearance::LinearRgb;

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
    rows: u32,
    /// The sky pass: the table [`voxel_appearance::sky::Sky::dome_radiance`] filled on
    /// the CPU, and the two numbers that turn a pixel into a direction.
    ///
    /// **The table lives here rather than being computed per frame in the shader**
    /// because the scattering model exists once, in Rust. See `sky.wgsl`.
    sky_pipeline: wgpu::RenderPipeline,
    sky_bind_group: wgpu::BindGroup,
    sky_view: wgpu::Buffer,
    sky_table: wgpu::Buffer,
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
                uniform_entry(1, wgpu::ShaderStages::VERTEX),
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
            ],
        });

        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shading"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shading.wgsl").into()),
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

        let view = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("view"),
            size: 64,
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

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("world"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: rig_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: view.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: materials_buffer.as_entire_binding() },
            ],
        });

        // ---- the sky pass -------------------------------------------------
        // Its own bind group and its own pipeline, sharing the render pass and the
        // depth attachment with the world so geometry lands on top of the sky without
        // a second pass or a resolve.
        let sky_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sky"),
            source: wgpu::ShaderSource::Wgsl(include_str!("sky.wgsl").into()),
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
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        // **Zeroed, not left undefined.** A caller that never writes a sky gets a
        // black one, which is what the pass did before it existed — so adding the sky
        // cannot change a frame whose owner has not asked for a sky yet.
        queue.write_buffer(&sky_view, 0, &[0u8; 16]);
        let sky_table = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sky-table"),
            size: (SKY_LUT_ROWS * 16) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&sky_table, 0, &vec![0u8; SKY_LUT_ROWS * 16]);
        let sky_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sky"),
            layout: &sky_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: sky_view.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: sky_table.as_entire_binding() },
            ],
        });

        Ok(WorldPipeline {
            device: device.clone(),
            queue: queue.clone(),
            pipeline,
            bind_group,
            view,
            rig: rig_buffer,
            rows: materials.len() as u32,
            sky_pipeline,
            sky_bind_group,
            sky_view,
            sky_table,
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

    /// The camera as the shader reads it. Kept public so a caller can upload
    /// once and draw twice (a shadow pass, say) without recomputing.
    pub fn write_camera(&self, camera: &Camera) {
        let mut bytes = Vec::with_capacity(64);
        for column in camera.view_projection() {
            for v in column {
                bytes.extend_from_slice(&v.to_le_bytes());
            }
        }
        self.queue.write_buffer(&self.view, 0, &bytes);

        // The sky's geometry, uploaded with the camera rather than separately: it is
        // the same two numbers, and a caller that moved the camera and not the sky
        // would get a sky belonging to a view that is no longer on screen.
        let mut sky_bytes = Vec::with_capacity(16);
        for v in [
            camera.pitch_rad as f32,
            (camera.fov_y_rad / 2.0).tan() as f32,
            SKY_LUT_ROWS as f32,
            0.0f32,
        ] {
            sky_bytes.extend_from_slice(&v.to_le_bytes());
        }
        self.queue.write_buffer(&self.sky_view, 0, &sky_bytes);
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
        if table.is_empty() || table.len() > SKY_LUT_ROWS {
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
        &self,
        encoder: &mut wgpu::CommandEncoder,
        colour: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        scene: &Scene,
        camera: &Camera,
    ) {
        self.write_camera(camera);
        let vertex_buffer =
            self.upload("vertices", &scene.vertex_bytes(), wgpu::BufferUsages::VERTEX);
        let index_buffer =
            self.upload("indices", &scene.index_bytes(), wgpu::BufferUsages::INDEX);

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

        if !scene.indices.is_empty() {
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..scene.indices.len() as u32, 0, 0..1);
        }
    }
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
