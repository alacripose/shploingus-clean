//! The offscreen renderer: a `Scene` and a `Camera` in, a `Frame` out.
//!
//! # Why the target is float and not 8-bit
//!
//! [`TARGET_FORMAT`] is `Rgba32Float`. That is a **verification** decision, not
//! a quality one. An 8-bit target quantises before anything can be compared, so
//! a real disagreement between `shading.wgsl` and `npr::shade` could hide inside
//! a rounding step. Reading back `f32` means the comparison in
//! [`tests::the_gpu_shades_a_surface_exactly_as_the_crate_does`] is between two
//! `f32` numbers and its tolerance means what it says.
//!
//! # Why culling is off
//!
//! `cull_mode: None`. The mesher's quads are wound outwards
//! (`mesh::winding_flips`), so back-face culling would be correct — but
//! "correct" here rests on a winding convention in another crate, and a frame
//! that comes out empty because a convention drifted is a confusing failure.
//! Culling is left for a change that measures a performance difference, and the
//! winding is already written down in [`crate::Scene`] for when it happens.
//!
//! # The cost this version accepts
//!
//! Vertex and index buffers are created and uploaded **every frame**. A client
//! that reuses them is the obvious next step and it is not taken here, because
//! taking it means an invalidation rule for meshes that arrive and are evicted,
//! and a rule that is not measured is a rule that is wrong. Named, not hidden.

use crate::camera::Camera;
use crate::frame::Frame;
use crate::scene::Scene;
use crate::shading::{GpuRig, MaterialParams};
use crate::world::WorldPipeline;
use crate::ClientError;

/// See the module comment.
pub const TARGET_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba32Float;

/// An adapter and a device with no surface attached — everything an offscreen
/// draw needs.
///
/// Extracted so the world pipeline, the HUD and the offscreen tests all reach
/// the GPU the same way. Two copies of this block would be two chances for one
/// of them to ask for a different power preference and quietly measure a
/// different device.
pub fn offscreen_device() -> Result<(wgpu::Device, wgpu::Queue, String), ClientError> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
        ..Default::default()
    }))
    .map_err(|e| ClientError::NoAdapter { detail: e.to_string() })?;
    let info = adapter.get_info();
    let name = format!("{} ({:?})", info.name, info.backend);
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("voxel-client"),
        ..Default::default()
    }))
    .map_err(|e| ClientError::NoDevice { detail: e.to_string() })?;
    Ok((device, queue, name))
}
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const BYTES_PER_PIXEL: u32 = 16;

/// A device, a pipeline, and a target, all sized for one frame extent.
pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    world: WorldPipeline,
    colour: wgpu::Texture,
    depth: wgpu::Texture,
    readback: wgpu::Buffer,
    width: u32,
    height: u32,
    padded_row_bytes: u32,
    /// **The cloud's cross-fade weight this renderer draws with** (C-183), set by
    /// [`Renderer::set_cloud_fade`] and handed to the pipeline before each frame's draw.
    ///
    /// It lives here rather than on the [`Scene`] because it is **the frame's** number and not the
    /// world's: the scene a fade joins is the same bytes whatever the weight is, so a caller can
    /// hold one scene and draw it at any point of the dissolve — which is the whole reason the
    /// interpolation is the shader's.
    cloud_fade: f32,
    /// What the adapter called itself. Recorded so a frame's provenance
    /// includes which GPU drew it — two band counts from two adapters are not
    /// obviously the same measurement.
    pub adapter: String,
}

impl Renderer {
    pub fn new(
        width: u32,
        height: u32,
        materials: &[MaterialParams],
        rig: &GpuRig,
    ) -> Result<Self, ClientError> {
        if width == 0 || height == 0 {
            return Err(ClientError::ZeroExtent { width, height });
        }
        if materials.is_empty() {
            return Err(ClientError::UnknownMaterial { id: 0, rows: 0 });
        }

        let (device, queue, adapter_name) = offscreen_device()?;

        // The pipeline is built by [`WorldPipeline`] so the window and this
        // offscreen target cannot drift apart. See that module.
        let world = WorldPipeline::new(&device, &queue, TARGET_FORMAT, materials, rig)?;

        let colour = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("colour"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TARGET_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let depth = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        // A texture copy needs every row aligned, and the row is wider than the
        // pixels it carries. The residue is skipped when reading back.
        let unpadded = width * BYTES_PER_PIXEL;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_row_bytes = unpadded.div_ceil(align) * align;
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: padded_row_bytes as u64 * height as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        Ok(Renderer {
            device,
            queue,
            world,
            colour,
            depth,
            readback,
            width,
            height,
            padded_row_bytes,
            // **One tree, whole**, until a caller says otherwise: a renderer that has never been
            // handed a fade draws exactly what it drew before fades existed.
            cloud_fade: 1.0,
            adapter: adapter_name,
        })
    }

    /// **The cloud's cross-fade weight the following frames are drawn at** (C-183) — handed to the
    /// pipeline with each frame, exactly as the pixel angle is, because it is the *frame's* number
    /// and not the scene's. See [`WorldPipeline::set_cloud_fade`].
    pub fn set_cloud_fade(&mut self, weight: f32) {
        self.world.set_cloud_fade(weight);
        self.cloud_fade = self.world.cloud_fade();
    }

    pub fn extent(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn material_rows(&self) -> u32 {
        self.world.rows()
    }

    /// The shared world pipeline. A caller that already owns a device can build
    /// one with [`WorldPipeline::new`] and draw to its own target — which is
    /// exactly what the `shploingus` bin does.
    pub fn world(&self) -> &WorldPipeline {
        &self.world
    }

    /// The shared world pipeline, **mutably** — so a caller can change what a frame
    /// is drawn *with* rather than what it is drawn from. The dither is the only such
    /// thing today ([`WorldPipeline::set_dither`]), and it needs this because the
    /// target's format decides it at construction while the verification target is
    /// deliberately not the window's.
    pub fn world_mut(&mut self) -> &mut WorldPipeline {
        &mut self.world
    }

    /// Draw the scene and read the pixels back.
    pub fn render(&mut self, scene: &Scene, camera: &Camera) -> Result<Frame, ClientError> {
        self.render_with(scene, camera, false)
    }

    /// **Draw the scene as its own wireframe** — the `F4` debug view — and read the pixels back.
    ///
    /// The same frame, target and pixel angle as [`Renderer::render`], through
    /// [`WorldPipeline::draw_wireframe`]. It exists because a view that nothing can *check* is a
    /// view that can be silently wired to the wrong thing — which is what happened to `F4` — and
    /// this is the door the check uses rather than a second renderer that could drift.
    pub fn render_wireframe(
        &mut self,
        scene: &Scene,
        camera: &Camera,
    ) -> Result<Frame, ClientError> {
        self.render_with(scene, camera, true)
    }

    fn render_with(
        &mut self,
        scene: &Scene,
        camera: &Camera,
        wireframe: bool,
    ) -> Result<Frame, ClientError> {
        let colour_view = self.colour.create_view(&wgpu::TextureViewDescriptor::default());
        let depth_view = self.depth.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("frame") });
        // The frame's own pixel angle, from the camera's field of view and this
        // target's height: the pattern ladder's footprint is measured in pixels of
        // *this* view, and the shader has no other way to know how big one is
        // (C-163).
        let pixel_angle = crate::pattern::pixel_angle_rad(camera.fov_y_rad, self.height);
        // The frame's own fade, put where the uniform is written ([`Renderer::set_cloud_fade`]).
        self.world.set_cloud_fade(self.cloud_fade);
        if wireframe {
            self.world.draw_wireframe(
                &mut encoder,
                &colour_view,
                &depth_view,
                scene,
                camera,
                pixel_angle,
            );
        } else {
            self.world
                .draw(&mut encoder, &colour_view, &depth_view, scene, camera, pixel_angle);
        }
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.colour,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.padded_row_bytes),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d { width: self.width, height: self.height, depth_or_array_layers: 1 },
        );
        self.queue.submit(Some(encoder.finish()));

        self.read_pixels()
    }

    fn read_pixels(&self) -> Result<Frame, ClientError> {
        let slice = self.readback.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.device
            .poll(wgpu::PollType::Wait { submission_index: None, timeout: None })
            .map_err(|e| ClientError::Readback { detail: format!("poll: {e}") })?;
        receiver
            .recv()
            .map_err(|e| ClientError::Readback { detail: format!("map callback: {e}") })?
            .map_err(|e| ClientError::Readback { detail: format!("map failed: {e}") })?;

        let data = slice
            .get_mapped_range()
            .map_err(|e| ClientError::Readback { detail: format!("get_mapped_range: {e}") })?;
        // The padded stride is what the copy wrote; the pixels are the first
        // `width * 16` bytes of every row and the residue is skipped.
        let stride = self.padded_row_bytes as usize;
        let mut pixels = Vec::with_capacity(self.width as usize * self.height as usize);
        for y in 0..self.height as usize {
            let row_start = y * stride;
            for x in 0..self.width as usize {
                let i = row_start + x * BYTES_PER_PIXEL as usize;
                let mut v = [0.0f32; 4];
                for (c, cell) in v.iter_mut().enumerate() {
                    let at = i + c * 4;
                    *cell = voxel_model::endian::le_from_bytes::<f32>(&[data[at], data[at + 1], data[at + 2], data[at + 3]]);
                }
                pixels.push(v);
            }
        }
        drop(data);
        self.readback.unmap();
        Ok(Frame { width: self.width, height: self.height, pixels })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shading::{self, GpuRig, MESH_DOES_NOT_CARRY_OPEN};
    use voxel_appearance::npr::LightRig;
    use voxel_appearance::openpbr::{Colour, Emission, LinearRgb, OpenPbrInputs};
    use voxel_noise::Vec3;

    /// The world's own shading field — the same declaration the window builds its
    /// table and its pipeline's lattice from, so a test cannot shade with gains the
    /// shader's own seed does not belong to (C-161).
    fn shading_field() -> voxel_model::ShadingField {
        voxel_shapers::materials::shading_field()
            .expect("the world's shading field declares for the world's own registry")
    }

    /// The same surface `npr::shade`'s own tests use: a stated grey with all
    /// twelve parameters present, so no term is missing and the comparison is
    /// about arithmetic rather than about absence.
    fn stated() -> OpenPbrInputs {
        OpenPbrInputs {
            base_color: Colour::Admitted(LinearRgb::grey(0.4)),
            base_metalness: 0.0,
            specular_roughness: Some(0.35),
            specular_ior: Some(1.5),
            specular_weight: Some(0.05),
            coat_weight: 0.0,
            coat_roughness: Some(0.1),
            transmission_weight: 0.0,
            transmission_depth: Some(0.0),
            subsurface_weight: Some(0.0),
            emission: Emission::None,
        }
    }

    const DISTANCE_M: f32 = 3.0;

    /// Probe normals: the axes, one axis reversed, and three off-axis
    /// directions, so the wrap, the rim and the specular step are each
    /// exercised at a different angle.
    const PROBES: [[f32; 3]; 8] = [
        [0.0, 0.0, -1.0],
        [0.0, 1.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.6, 0.8, 0.0],
        [0.0, 0.0, 1.0],
        [0.3, 0.3, -0.905_538_5],
        [-0.5, 0.5, -std::f32::consts::FRAC_1_SQRT_2],
    ];

    /// **The display transform is the crate's, on the GPU** (C-167).
    ///
    /// The window presents a target that cannot hold more than white, so a
    /// scene-referred frame is compressed before it is written. The compression is
    /// [`voxel_appearance::tonemap`]'s and `shading.wgsl` transcribes it, so this is
    /// the test that holds the transcription to the model — the same job
    /// [`tests::the_gpu_shades_a_surface_exactly_as_the_crate_does`] does for the
    /// shading itself.
    ///
    /// **The expectation is the frame's own pixels, transformed.** The undithered,
    /// untransformed frame is already known to equal `npr::shade` to `1e-5` (that
    /// test), so asking `tone_map` of *those* pixels states the pair's agreement
    /// twice over rather than once: a GPU that had a different curve fails here, and
    /// a GPU that had no curve fails the range assertions below.
    #[test]
    #[ignore = "needs a GPU adapter; run explicitly with --ignored"]
    fn the_gpu_presents_a_radiance_the_crate_says_it_does() {
        let rig = LightRig::standard();
        let gpu_rig = GpuRig::from_rig(&rig).expect("the standard rig fits");
        let inputs = stated();
        // **A stated emitter brighter than white, so the compression is actually
        // exercised.** An emitter is the one thing a world can contain that a target
        // genuinely cannot hold — `hud::BRIGHTEST_BACKDROP` names exactly this case as
        // the thing that would invalidate its own assumption — and a probe that
        // stayed below 1.0 would let a shader with **no** transform at all pass this
        // test, because an absent curve and a present one agree below white.
        let mut emissive = stated();
        emissive.emission = Emission::Emitted {
            color: LinearRgb::grey(1.0),
            luminance: 4.0,
        };
        let table = vec![
            MaterialParams::default(),
            MaterialParams::from_inputs(&inputs, MESH_DOES_NOT_CARRY_OPEN),
            MaterialParams::from_inputs(&emissive, MESH_DOES_NOT_CARRY_OPEN),
        ];

        const SIDE: u32 = 65;
        let mut renderer = Renderer::new(SIDE, SIDE, &table, &gpu_rig).expect("a GPU");
        renderer.world_mut().write_no_sky_model();

        // The measurement target neither quantises nor clips, and that is the
        // guarantee every other test in this file rests on: nothing here is dithered
        // or compressed unless it asked to be.
        assert_eq!(
            renderer.world().tonemap(),
            0.0,
            "the offscreen float target is a measurement, not a display, so its frames \
             must be the scene-referred value the model computed"
        );

        // And the window's target is the one that needs it.
        let (device, queue, _) = offscreen_device().expect("a GPU");
        let srgb = WorldPipeline::new(
            &device,
            &queue,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            &table,
            &gpu_rig,
        )
        .expect("the pipeline for an 8-bit sRGB target");
        assert_eq!(
            srgb.tonemap(),
            1.0,
            "the surface the window presents cannot hold more than white, so it is \
             the target the display transform exists for"
        );

        let camera = Camera::standard([0.0, 0.0, 0.0], 1.0);
        // Row 2 is the emitter, so the probe's own radiance is above white.
        let scene = Scene::probe([0.0, 0.0, -1.0], 2, DISTANCE_M, 2.0);
        let scene_referred = renderer.render(&scene, &camera).expect("a frame");

        renderer.world_mut().set_tonemap(1.0);
        let presented = renderer.render(&scene, &camera).expect("a frame");

        let mut worst = 0.0f64;
        let mut brightest = 0.0f64;
        for row in 0..SIDE {
            for col in 0..SIDE {
                let a = scene_referred
                    .pixel(col, row)
                    .expect("the probe covers the frame");
                let b = presented.pixel(col, row).expect("the probe covers the frame");
                let scene_value = voxel_appearance::LinearRgb::new(a[0] as f64, a[1] as f64, a[2] as f64);
                brightest = brightest.max(scene_value.luminance());
                let want = voxel_appearance::tonemap::tone_map(scene_value);
                for (c, w) in [want.r, want.g, want.b].into_iter().enumerate() {
                    worst = worst.max((b[c] as f64 - w).abs());
                    // **Nothing a display cannot present**, which is the claim the
                    // transform is for. A clipping renderer satisfies the lower
                    // bound and fails the upper one.
                    assert!(
                        (0.0..=1.0).contains(&(b[c] as f64)),
                        "pixel ({col},{row}) presented channel {c} as {} where a display \
                         can hold 0..=1",
                        b[c]
                    );
                }
            }
        }

        println!(
            "the display transform: worst disagreement {worst:.3e} over {} pixels, \
             scene-referred brightest {brightest:.6}",
            SIDE * SIDE
        );
        assert!(
            worst < 1e-5,
            "the GPU and the crate disagree about the display transform by {worst}"
        );
        // **The premise, asserted rather than assumed.** A shader that returned the
        // scene-referred value unchanged agrees with the crate at every value below
        // white, so a dim probe would let the whole transform go missing and still
        // pass. This is the assertion that makes the one above worth something.
        assert!(
            brightest > 1.0,
            "the scene-referred probe only reached {brightest}, so this frame never \
             crossed the display's own white and an absent transform could not show"
        );
        println!("the scene-referred probe reached {brightest:.6}, above white");
    }

    /// **The dither is a sub-step offset and nothing else** (C-166).
    ///
    /// The window presents an 8-bit `sRGB` surface, which rounds a smooth shade to
    /// 256 levels and so draws the sky's gradient as bands; the dither spreads that
    /// rounding across the pixel with an ordered threshold. The verification target
    /// is `Rgba32Float` and quantises nothing, so the amount is `0` there and this
    /// test turns it on by hand — which is the only way to measure a repair for
    /// banding on a target that has no bands.
    ///
    /// **What is asserted is the dither's definition, not its appearance.** It moves
    /// pixels; it moves each one by **at most half a step of an 8-bit channel in the
    /// target's own encoding**; and it leaves the frame's average where it was, which
    /// is what an ordered threshold whose matrix tiles the frame exactly must do.
    /// That the bands are then *gone* is a claim about a quantising target this
    /// offscreen path is not, and no assertion here can make it.
    #[test]
    #[ignore = "needs a GPU adapter; run explicitly with --ignored"]
    fn the_dither_moves_a_shade_by_less_than_a_step_and_not_the_average() {
        /// The same transfer function the shader dithers in, so the assertion can be
        /// made where the quantisation happens rather than where it does not.
        fn srgb_encode(c: f64) -> f64 {
            let l = c.clamp(0.0, 1.0);
            if l <= 0.003_130_8 {
                l * 12.92
            } else {
                1.055 * l.powf(1.0 / 2.4) - 0.055
            }
        }

        let registry = voxel_shapers::materials::registry().expect("the registry");
        let table = crate::shading::table(&registry, &shading_field())
            .expect("every admitted material derives");
        let rig = GpuRig::from_rig(&LightRig::standard()).expect("the standard rig fits");

        /// 64×64 is exactly eight 8×8 tiles, so the Bayer matrix covers the frame a
        /// whole number of times and its mean offset is **zero and not merely small**.
        const SIDE: u32 = 64;
        let mut renderer = Renderer::new(SIDE, SIDE, &table, &rig).expect("a GPU");
        renderer.world_mut().write_no_sky_model();

        // **The float target decided by itself**, and that is the guarantee the rest
        // of the suite rests on: no measurement in this file is dithered unless it
        // asked to be, so the transcription of `npr::shade` is compared against the
        // number the model computed rather than against one plus a threshold.
        assert_eq!(
            renderer.world().dither(),
            0.0,
            "the offscreen float target is not a quantising target, so its frames \
             must be drawn undithered"
        );

        // **And the window's own format turns it on**, which is the decision the game
        // depends on and the only place it is made: an 8-bit `sRGB` surface is the
        // target that quantises, so it is the target the dither is for.
        let (device, queue, _) = offscreen_device().expect("a GPU");
        let srgb = WorldPipeline::new(
            &device,
            &queue,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            &table,
            &rig,
        )
        .expect("the pipeline for an 8-bit sRGB target");
        assert_eq!(
            srgb.dither(),
            1.0,
            "the surface the window presents quantises, so its frames are the ones \
             the dither exists for"
        );

        let camera = Camera::standard([0.0, 0.0, 0.0], 1.0);
        let scene = Scene::probe([0.0, 0.0, -1.0], 1, 3.0, 3.0);
        let plain = renderer.render(&scene, &camera).expect("a frame");

        renderer.world_mut().set_dither(1.0);
        let dithered = renderer.render(&scene, &camera).expect("a frame");

        let mut moved = 0usize;
        let mut worst_encoded = 0.0f64;
        let mut sum_encoded = 0.0f64;
        for row in 0..SIDE {
            for col in 0..SIDE {
                let a = plain.pixel(col, row).expect("the probe covers the frame");
                let b = dithered.pixel(col, row).expect("the probe covers the frame");
                for c in 0..3 {
                    let delta = srgb_encode(b[c] as f64) - srgb_encode(a[c] as f64);
                    if delta != 0.0 {
                        moved += 1;
                    }
                    worst_encoded = worst_encoded.max(delta.abs());
                    sum_encoded += delta;
                }
            }
        }

        assert!(moved > 0, "the dither changed nothing, so it is not being applied");
        println!(
            "dither: {moved} moved, worst {worst_encoded:.6} of a {:.6} step, mean {:.9}",
            1.0 / 255.0,
            sum_encoded / (SIDE * SIDE * 3) as f64
        );
        // Half a step either way is the whole of what a `[-0.5, 0.5)` threshold can
        // add, so one step is the bound that is tight and still tolerant of the
        // round trip through `f32` and the transfer function twice.
        assert!(
            worst_encoded <= 1.0 / 255.0,
            "a shade moved by {worst_encoded} of a channel step, which is more than the \
             one step an ordered threshold may spend"
        );
        assert!(
            (sum_encoded / (SIDE * SIDE * 3) as f64).abs() < 1e-6,
            "the dither moved the frame's average by {}, so it is a brightness change and \
             not a rounding one",
            sum_encoded / (SIDE * SIDE * 3) as f64
        );
    }

    #[test]
    #[ignore = "needs a GPU adapter; run explicitly with --ignored"]
    fn the_gpu_shades_a_surface_exactly_as_the_crate_does() {
        let rig = LightRig::standard();
        let gpu_rig = GpuRig::from_rig(&rig).expect("the standard rig fits");
        let inputs = stated();
        let table = vec![
            MaterialParams::default(),
            MaterialParams::from_inputs(&inputs, MESH_DOES_NOT_CARRY_OPEN),
        ];
        /// **65x65, and that is not cosmetic.** A pixel is sampled at its own
        /// centre, so a frame of even width has no pixel at the frame's centre:
        /// the nearest sample sits half a pixel off, and on a screen-filling
        /// probe quad the view vector there therefore differs from the one this
        /// reference is handed. The first version of this test used 64x64 and
        /// failed by 4.3e-2 on a head-on normal, because the rim term is
        /// `(1 - n·v)^5` and that is exactly where half a pixel of view error is
        /// loudest. An odd extent puts pixel 32's sample at NDC 0 exactly, so
        /// both sides compute the *same* surface.
        const SIDE: u32 = 65;
        const CENTRE: u32 = 32;
        let mut renderer = Renderer::new(SIDE, SIDE, &table, &gpu_rig).expect("a GPU");
        println!("adapter: {}", renderer.adapter);
        renderer.world_mut().write_no_sky_model();

        let camera = Camera::standard([0.0, 0.0, 0.0], 1.0);
        // The probe quad sits at +Z, the eye is the origin, and pixel 32's
        // sample is at NDC (0, 0) — the quad's own centre — so the view vector
        // there is exactly (0, 0, -distance).
        let view = Vec3::new(0.0, 0.0, -DISTANCE_M as f64);

        let mut worst: f64 = 0.0;
        for normal in PROBES {
            let scene = Scene::probe(normal, 1, DISTANCE_M, 2.0);
            let frame = renderer.render(&scene, &camera).expect("a frame");
            let got = frame.pixel(CENTRE, CENTRE).expect("the centre is in range");
            assert_eq!(got[3], 1.0, "the probe must actually cover the centre");

            let (expected, _terms) = shading::shade_cpu(
                &inputs,
                Vec3::new(normal[0] as f64, normal[1] as f64, normal[2] as f64),
                view,
                0.0,
                &rig,
            )
            .expect("the stated surface shades");
            for (c, want) in [expected.r, expected.g, expected.b].into_iter().enumerate() {
                let delta = (got[c] as f64 - want).abs();
                worst = worst.max(delta);
            }
            println!(
                "normal {:?}: gpu {:?} crate {:?}",
                normal,
                [got[0], got[1], got[2]],
                [expected.r, expected.g, expected.b]
            );
        }
        // The tolerances are f32-versus-f64, not slack: the shader and the crate
        // compute the same expressions over the same inputs, so anything above
        // a rounding step is a real difference in the model.
        assert!(worst < 2e-3, "the GPU and the crate disagree by {worst}");
        println!("worst channel disagreement: {worst:e}");
    }

    /// **The material's pattern is drawn where the greedy mesh has no geometry.**
    ///
    /// The claim C-161 exists for: a rectangle the greedy mesher merged out of
    /// hundreds of blocks still shows its material's own detail — the subvoxel grid
    /// and the 256 lattice — because the fragment addresses the lattice itself and
    /// no quad carries it. The probe stands in for the merged rectangle: one quad,
    /// one material slot, and every pixel a different address.
    ///
    /// **Per pixel against the crate.** The probe is parallel to the image plane,
    /// so a pixel's fragment position is the camera's own projection read backwards,
    /// and the footprint the shader derives (`eye_footprint`) is recomputed here from
    /// the same inputs — the fragment's own distance, the probe's normal, and the
    /// frame's pixel angle. Both sides therefore evaluate the *same* surface, and a
    /// hash that diverged between WGSL and Rust could not pass.
    ///
    /// **A stage per rung the footprint can draw.** At 2 m with a 512² probe the
    /// footprint is ~5 mm, so the coarse rungs carry the surface; at 0.15 m — above
    /// the camera's own near plane — it is a fraction of a millimetre and the finer
    /// rungs lead, which is how they are exercised through the shipped pipeline rather
    /// than only in the crate. A 65² probe 400 m out carries a ~8.6 m footprint at its
    /// centre — past the top rung's own floor — and is there to prove the level is
    /// then *silent* rather than aliasing (the equality below).
    ///
    /// Ignored because it needs a GPU adapter; about a second of `f64` hashing.
    #[test]
    #[ignore = "needs a GPU adapter; run explicitly with --ignored"]
    fn the_pattern_is_drawn_where_the_greedy_mesh_has_no_geometry() {
        use crate::pattern::Pattern;
        use voxel_model::ShadingField;
        use voxel_shapers::materials;

        let registry = materials::registry().expect("the registry");
        let field = materials::shading_field().expect("the world's shading field declares");
        let table = shading::table(&registry, &field).expect("every admitted material derives");
        let lattice = voxel_appearance::DetailLattice::new(field.domain());
        let pattern = Pattern::new(&lattice, &field);
        let rig = LightRig::standard();
        let gpu_rig = GpuRig::from_rig(&rig).expect("the standard rig fits");

        // A material the field *does* vary, and whose inputs the crate derives the
        // same way the table does — so the pattern is on and both sides shade the
        // ship's own row for it.
        let material = materials::REGOLITH;
        let surface = materials::pattern_of(material);
        let inputs = shading::inputs_for(&registry, material).expect("regolith derives");
        assert!(
            field.gain(material).is_some(),
            "the standing field does not vary regolith, so this test would pass by drawing nothing"
        );

        // **The bark stage** holds the shader to a shaper that is not the identity:
        // its stencil stretches the grain along z, so the GPU is compared against a
        // pattern whose character (not just its amplitude) differs from speckle.
        let bark_material = materials::WOOD;
        let bark_surface = materials::pattern_of(bark_material);
        let bark_inputs = shading::inputs_for(&registry, bark_material).expect("wood derives");
        assert!(
            field.gain(bark_material).is_some(),
            "the standing field does not vary wood, so the bark stage would draw nothing"
        );

        /// A pixel whose address sits on a rounding edge can land in the next cell on
        /// the GPU, where the arithmetic is `f32`. Those pixels are skipped and
        /// counted; the margin is a fixed fraction of one lattice cell.
        const ADDRESS_MARGIN: f64 = 1.0e-3;

        // A stage whose eye is *not* at the origin: the geometry is eye-relative so
        // the probe does not move on screen, but the pattern's address is anchored
        // to the world — so this only agrees with the crate if `eye_voxel` and
        // `eye_phase` are added back exactly (C-161; the four-byte uniform write
        // that broke this path is C-162).
        let stage = |material: voxel_model::MaterialId,
                     surface: materials::SurfacePattern,
                     inputs: &OpenPbrInputs,
                     eye: [f64; 3],
                     side: u32,
                     distance_m: f32,
                     half_m: f32,
                     what: &str| {
            let mut renderer = Renderer::new(side, side, &table, &gpu_rig).expect("a GPU");
            renderer.world_mut().write_no_sky_model();
            let camera = Camera::standard(eye, 1.0);
            let scene = Scene::probe([0.0, 0.0, -1.0], material.0 as u32, distance_m, half_m);
            let frame = renderer.render(&scene, &camera).expect("a frame");
            let f = 1.0 / (camera.fov_y_rad / 2.0).tan();
            let aspect = camera.aspect;
            let d = distance_m as f64;
            // One pixel step of the interpolated world position, for the address
            // ambiguity check: the probe is parallel to the image plane, so its two
            // on-screen axes step and its depth axis does not.
            let step = [2.0 * aspect * d / (f * side as f64), 2.0 * d / (f * side as f64), 0.0];
            // The footprint the shader computes: one pixel's angle carried out to the
            // fragment's own distance and stretched by the face's obliquity
            // (`pattern::eye_footprint_m`) — per pixel, not one number for the whole
            // probe, because the distance to a corner is not the distance to the
            // centre and C-163's whole point is that the footprint follows the
            // fragment's own geometry smoothly.
            let pixel_angle = crate::pattern::pixel_angle_rad(camera.fov_y_rad, side) as f64;
            let footprint_centre = d * pixel_angle;
            let mut worst = 0.0f64;
            let mut compared = 0usize;
            let mut skipped = 0usize;
            let mut distinct: std::collections::BTreeSet<u32> = Default::default();
            for j in 0..side {
                for i in 0..side {
                    let ndc_x = 2.0 * (i as f64 + 0.5) / side as f64 - 1.0;
                    let ndc_y = 1.0 - 2.0 * (j as f64 + 0.5) / side as f64;
                    let p_rel = [ndc_x * aspect * d / f, ndc_y * d / f, d];
                    let p = [eye[0] + p_rel[0], eye[1] + p_rel[1], eye[2] + p_rel[2]];
                    // The shader's own footprint for this pixel: the probe's normal
                    // is `-z` and the direction to the eye is `-p_rel`, so the face's
                    // cosine is the perpendicular distance over the ray's length.
                    let r =
                        (p_rel[0] * p_rel[0] + p_rel[1] * p_rel[1] + p_rel[2] * p_rel[2]).sqrt();
                    let footprint = crate::pattern::eye_footprint_m(r, pixel_angle, d / r);
                    let got = frame.pixel(i, j).expect("the probe covers the frame");
                    distinct.insert(got[0].to_bits());
                    if !crate::pattern::address_is_settled(p, step, ADDRESS_MARGIN) {
                        skipped += 1;
                        continue;
                    }
                    let (cell, at) = crate::pattern::address_at(p).expect("a finite point");
                    let deviation = pattern
                        .deviation(material, cell, at, footprint, surface)
                        .expect("a finite point");

                    let varied = inputs
                        .with_detail(deviation)
                        .expect("a legal deviation keeps a legal value");
                    let view = [-p_rel[0], -p_rel[1], -p_rel[2]];
                    let norm = (view[0] * view[0] + view[1] * view[1] + view[2] * view[2]).sqrt();
                    let (expected, _) = shading::shade_cpu(
                        &varied,
                        Vec3::new(0.0, 0.0, -1.0),
                        Vec3::new(view[0] / norm, view[1] / norm, view[2] / norm),
                        0.0,
                        &rig,
                    )
                    .expect("the stated surface shades");
                    for (c, want) in [expected.r, expected.g, expected.b].into_iter().enumerate() {
                        worst = worst.max((got[c] as f64 - want).abs());
                    }
                    compared += 1;
                }
            }
            println!(
                "{what}: probe at {distance_m} m, footprint {footprint_centre:.2e} m at the \
                 centre, compared {compared} of {} pixels ({skipped} skipped on a cell edge), \
                 worst disagreement {worst:e}, {} distinct reds",
                side * side,
                distinct.len()
            );
            (worst, compared, skipped, distinct.len())
        };

        // The ladder's stages: a coarse probe whose footprint only the coarse rungs
        // can draw, a fine probe where the finest rungs resolve too, a moved eye,
        // and a shaper that is not the isotropic default. The silent probe is the
        // equality below, past the ladder's ceiling.
        let coarse =
            stage(material, surface, &inputs, [0.0, 0.0, 0.0], 512, 2.0, 2.0, "coarse rungs");
        let near = stage(material, surface, &inputs, [0.0, 0.0, 0.0], 512, 0.15, 0.5, "fine rungs");
        let past =
            stage(material, surface, &inputs, [0.0, 0.0, 0.0], 65, 2.0, 2.0, "mid rungs");
        let moved =
            stage(material, surface, &inputs, [3.25, -7.5, 12.125], 512, 0.15, 0.5, "eye moved");
        let bark = stage(bark_material, bark_surface, &bark_inputs, [0.0, 0.0, 0.0], 512, 0.15, 0.5, "bark shaper");
        assert!(coarse.1 > 200_000, "the coarse stage compared only {} pixels", coarse.1);
        assert!(near.1 > 200_000, "the near stage compared only {} pixels", near.1);
        // **Visible at the scale the pixel can draw.** A flat material would give
        // one red across the whole probe; the ladder has to be drawn, not merely
        // computed, at both the coarse and the fine probe.
        assert!(
            coarse.3 > 16 && near.3 > 16 && moved.3 > 16 && bark.3 > 16,
            "the ladder is not visible: {} / {} / {} / {} distinct reds",
            coarse.3,
            near.3,
            moved.3,
            bark.3
        );
        // **And silent where nothing can be drawn.** Past the ceiling's Nyquist floor
        // the level must draw *nothing* — an honest average that cannot alias —
        // rather than the streaks the eye-check saw. A whole-frame red count cannot
        // say this (the view-dependent terms vary across any probe), so the
        // assertion is an **equality**: the same probe rendered with a field that
        // varies nothing must come out identical. A still-lit pattern — the retired
        // tone, or a rung drawn past its Nyquist floor — separates the two frames.
        //
        // The probe is 400 m out, where a 65² frame's centre footprint is 8.6 m:
        // past the 32 m top rung's own 8 m floor, which is the only place the
        // ladder is empty. (It was written at 100 m — a 2.15 m footprint — when
        // the ceiling was 4 m; the ceiling's extension to 32 m left the probe
        // inside the ladder, where rungs 10–11 draw legitimately, and the
        // equality below failed on the stale fixture rather than on the shader.)
        let (_, at) = crate::pattern::address_at([0.0, 0.0, 100.0]).expect("a finite point");
        assert_eq!(
            pattern
                .deviation(material, [0, 0, 0], at, 9.0, surface)
                .expect("a sample"),
            voxel_appearance::DetailSample::NONE,
            "the model is not silent past the ladder's ceiling"
        );
        let silent = shading::table(&registry, &ShadingField::none()).expect("an empty field's table");
        let mut lit = Renderer::new(65, 65, &table, &gpu_rig).expect("a GPU");
        let mut unlit = Renderer::new(65, 65, &silent, &gpu_rig).expect("a GPU");
        lit.world_mut().write_no_sky_model();
        unlit.world_mut().write_no_sky_model();
        let camera = Camera::standard([0.0, 0.0, 0.0], 1.0);
        let scene = Scene::probe([0.0, 0.0, -1.0], material.0 as u32, 400.0, 80.0);
        let with = lit.render(&scene, &camera).expect("a frame");
        let without = unlit.render(&scene, &camera).expect("a frame");
        let mut differing = 0usize;
        for j in 0..65 {
            for i in 0..65 {
                if with.pixel(i, j).expect("the probe covers the frame")
                    != without.pixel(i, j).expect("the probe covers the frame")
                {
                    differing += 1;
                }
            }
        }
        assert_eq!(
            differing, 0,
            "{differing} pixels still carry a pattern past the ladder's ceiling"
        );
        for (worst, _, skipped, _) in [coarse, near, past, moved, bark] {
            assert!(worst < 5e-3, "the GPU and the crate disagree by {worst}");
            assert!(skipped < 20_000, "{skipped} pixels were skipped as unsettled");
        }
    }

    /// **The sun reaches the frame, measured in pixels rather than asserted.**
    ///
    /// C-40: the client is never tested invisibly. Every other test in this file can be
    /// satisfied by code that is correct and never called, and this one cannot — it
    /// builds the rig the way the window builds it, uploads it through the **same
    /// [`WorldPipeline::write_rig`] the frame loop calls**, and reads the picture back
    /// off the GPU.
    ///
    /// The probe carries `stated()`'s base colour, which the streamed terrain does not
    /// (`ShadingReport { with_base_colour: 0 }`), so this is the only surface in the
    /// project that responds to light at all. That is the whole reason the proof is a
    /// probe and not a frame of the world.
    #[test]
    #[ignore = "needs a GPU adapter; run explicitly with --ignored"]
    fn the_derived_rig_reaches_the_pixels_and_moves_with_the_tick() {
        use crate::skylight::daylight_of;
        use voxel_appearance::sky::SkyInputs;
        use voxel_shapers::{Config, WorldSpec};

        const SIDE: u32 = 65;
        const CENTRE: u32 = 32;

        let spec = voxel_shapers::WorldSpec::new(2468, voxel_shapers::Config::default());
        // **The second tick is searched for, not assumed.** The first attempt was a
        // quarter of a day before noon and measured **−16°**, which is night. That still
        // proved the tick reaches the pixels, but it proved it *by being dark* — the
        // weaker claim, because a rig that dropped the sun entirely would pass it too.
        //
        // So both ticks here are in daylight: the second is found between a third and
        // two thirds of the day's noon altitude and above the horizon. Two times of day,
        // both lit, differing in altitude, colour and intensity, with a pixel difference
        // that cannot be explained by one of them being absent.
        let noon = voxel_shapers::climate::TICKS_PER_DAY / 2;
        let noon_alt = voxel_shapers::sky::solar_position(&spec, noon).altitude_deg;
        let morning = (1..voxel_shapers::climate::TICKS_PER_DAY)
            .map(|t| (noon + t) % voxel_shapers::climate::TICKS_PER_DAY)
            .find(|&t| {
                let a = voxel_shapers::sky::solar_position(&spec, t).altitude_deg;
                a > 0.0 && a > noon_alt * 0.3 && a < noon_alt * 0.65
            })
            .expect("this world has a lower sun that is still up");
        let at = |tick: u64| {
            daylight_of(&spec, tick, SkyInputs::default(), 0).expect("a sky at the default atmosphere")
        };
        let noon_day = at(noon);
        let morning_day = at(morning);
        println!(
            "noon altitude {} deg intensity {}; later altitude {} deg intensity {} (tick {morning})",
            noon_day.sun_altitude_deg,
            noon_day.sun_intensity,
            morning_day.sun_altitude_deg,
            morning_day.sun_intensity
        );

        // **The rig has to differ or the pixel comparison below is vacuous**, and the
        // reason it names the *sun* rather than the rig is that the moon is in here too:
        // a difference in light count alone would not show the sun moving.
        assert!(
            noon_day.sun_altitude_deg > morning_day.sun_altitude_deg + 5.0,
            "the sun was at {} deg at noon and {} deg later",
            noon_day.sun_altitude_deg,
            morning_day.sun_altitude_deg
        );
        // **Both are lit, and neither is the whole claim.** If either intensity were
        // zero the test would have become the day-versus-night test it started as.
        assert!(
            noon_day.sun_intensity > 0.0 && morning_day.sun_intensity > 0.0,
            "one of the two ticks had no sun: noon {} and later {}",
            noon_day.sun_intensity,
            morning_day.sun_intensity
        );
        assert_ne!(noon_day.rig, morning_day.rig, "two daylight ticks produced one rig");

        let inputs = stated();
        let table = vec![MaterialParams::from_inputs(&inputs, MESH_DOES_NOT_CARRY_OPEN)];
        // **Booted on the fixture**, exactly as the window used to be and no longer is,
        // so the first comparison is fixture-versus-derived and cannot be circular.
        let boot = GpuRig::from_rig(&LightRig::standard()).expect("the standard rig fits");
        let mut renderer = Renderer::new(SIDE, SIDE, &table, &boot).expect("a GPU");
        renderer.world_mut().write_no_sky_model();
        let camera = Camera::standard([0.0, 0.0, 0.0], 1.0);
        let normal = [0.0, 1.0, 0.0];
        let scene = Scene::probe(normal, 0, DISTANCE_M, 2.0);
        let centre = |r: &mut Renderer| -> [f32; 4] {
            r.render(&scene, &camera)
                .expect("a frame")
                .pixel(CENTRE, CENTRE)
                .expect("the probe covers the centre")
        };

        let boot_px = centre(&mut renderer);

        let mut worst = 0.0f64;
        let mut shots: Vec<(&str, [f32; 4], &crate::skylight::Daylight)> = Vec::new();
        for (label, day) in [("noon", &noon_day), ("later", &morning_day)] {
            let gpu = GpuRig::from_rig(&day.rig)
                .unwrap_or_else(|| panic!("the {label} rig does not fit the uniform"));
            assert!(
                renderer.world().write_rig(&gpu).is_some(),
                "the {label} rig was refused by the uniform, so nothing was uploaded"
            );
            let px = centre(&mut renderer);
            // **The CPU is asked what it expected**, with the same rig the GPU was
            // given. This is the half that makes it a proof rather than a screenshot:
            // a rig that changed *both* the CPU answer and the picture would still pass
            // a difference check, and would not be shading the surface we think it is.
            let (expected, _terms) = shading::shade_cpu(
                &inputs,
                Vec3::new(normal[0] as f64, normal[1] as f64, normal[2] as f64),
                Vec3::new(0.0, 0.0, -(DISTANCE_M as f64)),
                0.0,
                &day.rig,
            )
            .expect("the stated surface shades");
            for (c, want) in [expected.r, expected.g, expected.b].into_iter().enumerate() {
                worst = worst.max((px[c] as f64 - want).abs());
            }
            shots.push((label, px, day));
        }

        // **The derived rig changed the picture.** Not "the picture is not black" —
        // the specific claim that the sun reaching the uniform reaches the frame.
        assert_ne!(
            boot_px, shots[0].1,
            "the derived rig reached the uniform but the picture did not move"
        );
        assert_ne!(
            shots[0].1, shots[1].1,
            "two daylight ticks shaded identically, so the tick does not reach the pixels"
        );
        for (label, px, _) in &shots {
            println!("{label}: {px:?}");
        }
        assert!(worst < 2e-3, "the GPU and the crate disagree by {worst}");
        println!("worst channel disagreement: {worst:e}");
    }

    /// **The world's own pixels under the derived rig** — C-40's proof taken on the
    /// shipped view rather than on a probe quad.
    ///
    /// **What this is correcting.** For most of this work I reported that the world drew
    /// black because no material carried a reflectance, and therefore that the sun could
    /// not be seen on it. Both halves were wrong: `materials::registry` admits
    /// sourced reflectances, and `the_shipped_view_is_not_a_black_screen` measures
    /// **56.1 % of pixels above 0.01, `max 0.2012`, mean 0.00847** — a visible, *dim*
    /// world. The dimness is the fixture's: `LightRig::standard`'s ambient is
    /// "deliberately dim". So the probe quad was never the only surface that could show
    /// the sun, and this test is the one that should have existed from the start.
    ///
    /// Ignored because it meshes 605 chunks; about twenty seconds.
    #[test]
    #[ignore = "needs a GPU adapter and ~20 s of meshing; run explicitly with --ignored"]
    fn the_derived_rig_lights_the_shipped_world_rather_than_only_a_probe() {
        use crate::game::{spawn_eye, World, GAME_PITCH_RAD, GAME_SEED, GAME_WINDOW};
        use crate::skylight::daylight_of;
        use voxel_appearance::sky::SkyInputs;
        use voxel_shapers::{Config, WorldSpec};
        use voxel_stream::WorldSource;

        let spec = WorldSpec::new(GAME_SEED, Config::default());
        let source = WorldSource::new(spec);
        let registry = voxel_shapers::materials::registry().expect("the registry");
        let table = shading::table(&registry, &shading_field()).expect("every admitted material derives");

        let workers = std::thread::available_parallelism()
            .map(|n| n.get().saturating_sub(1).max(1))
            .unwrap_or(1);
        let mesh_config = voxel_mesh::MeshConfig::from_registry(&registry).expect("a mesh config");
        let mut streamer =
            voxel_work::Streamer::new(source.clone(), mesh_config, workers, crate::GAME_CEILING_BYTES)
                .and_then(|s| s.with_window(GAME_WINDOW))
                .expect("the streamer");
        let eye = spawn_eye(&World::shipped());
        let eye_chunk = voxel_model::ChunkPos::new(
            (eye[0] / 32.0).floor() as i32,
            (eye[1] / 32.0).floor() as i32,
            (eye[2] / 32.0).floor() as i32,
        );
        streamer.set_centre(eye_chunk);
        streamer.settle().expect("the window settles");

        // **Booted on the fixture, which is what the window used for its whole life.**
        let fixture = GpuRig::from_rig(&LightRig::standard()).expect("the standard rig fits");
        let mut renderer = Renderer::new(1280, 720, &table, &fixture).expect("a GPU");
        // **This frame has no sky, and states the absence.** The world pass requires a stated sky — a
        // buffer of zeros is not a sky, it is nothing stated — and the frame this test measures has
        // never had one: its thresholds are the world's own lit pixels, so the absence is declared here
        // rather than a sky being added under the numbers.
        renderer.world().write_no_sky_model();
        let scene = Scene::build_with_detail(streamer.meshes(), eye, &voxel_appearance::DetailLattice::new(0));
        let mut camera = Camera::standard(eye, 1280.0 / 720.0);
        camera.pitch_rad = GAME_PITCH_RAD;
        let measure = |r: &mut Renderer, label: &str| {
            let stats = Stats::of(&r.render(&scene, &camera).expect("a frame"));
            println!(
                "{label}: mean {:.5} max {:.5} above1% {:.1}% colours {}",
                stats.mean,
                stats.max,
                100.0 * stats.above_one_percent as f64 / stats.pixels as f64,
                stats.colours
            );
            stats
        };

        let before = measure(&mut renderer, "fixture LightRig::standard()");

        // **The rig the client now derives**, at the world's own noon.
        let noon = voxel_shapers::climate::TICKS_PER_DAY / 2;
        let day = daylight_of(&spec, noon, SkyInputs::default(), 0).expect("a sky at noon");
        println!(
            "derived noon: sun {:.1} deg intensity {:.3} lights {} ambient {:.5}",
            day.sun_altitude_deg,
            day.sun_intensity,
            day.lights,
            day.rig.ambient.luminance()
        );
        // **Through `exposed`, because that is what the client uploads.** The rig itself
        // is a physical measurement in `[0, 1]`; without the declared exposure it renders
        // at `mean 0.00085` and 0.8% of pixels lit, which is a correct radiance and an
        // invisible picture. **This frame is also the measurement behind
        // `skylight::EXPOSURE`**: at `8.0` the brightest noon pixel measured `1.43345`
        // once C-143's sampled spectra replaced the placeholder greys, and the exposure
        // was re-measured to `4.0` rather than the frame being excused (C-159.6).
        let noon_rig =
            GpuRig::from_rig(&crate::skylight::exposed(&day.rig)).expect("the noon rig fits");
        renderer.world().write_rig(&noon_rig).expect("the noon rig is uploaded");
        let after = measure(&mut renderer, "derived skylight rig at noon");

        // **Not blown out.** The first version of this rig put the sun's raw radiance
        // (~1090 W/m²) into `Light`'s `colour`, which the shader multiplies by
        // `intensity`, and the world rendered **103× too bright**: `mean 0.87457`, a
        // brightest pixel of **78.3** out of a `[0, 1]` frame. `Light` carries a
        // chromaticity in `colour` and a magnitude in `intensity` — that is
        // `LightRig::standard`'s own convention, `(0.85, 0.90, 1.0)` with `1.0` — so
        // the colour is now a chromaticity and the ambient a ratio against the same peak.
        assert!(
            after.max <= 1.0,
            "the world's brightest pixel was {} — the rig is not one unit",
            after.max
        );

        // **And the picture is not black either**, which is the other half. The declared
        // exposure exists now (`skylight::EXPOSURE`, re-measured here), so the assertion
        // below pins that the rig reaches the pixels at all — the brightness itself is
        // measured above, not asserted, because nobody has ruled what right looks like.
        assert!(
            after.max > 0.0,
            "the derived rig left the world black: {}",
            after.max
        );
        assert!(
            after.max != before.max,
            "the derived rig and the fixture drew the same brightest pixel, so nothing \
             about the light reached the world"
        );
        println!(
            "**Measured, not asserted**: the exposed rig keeps the world's brightest \
             noon pixel at {} (under full scale) with mean {:.5} and {:.1}% of pixels \
             above 0.01. The fixture's {} was not correct either — it was an arbitrary \
             ambient and no sun.",
            after.max,
            after.mean,
            100.0 * after.above_one_percent as f64 / after.pixels as f64,
            before.mean
        );
        // **And it went up by two orders of magnitude, which is not a lighting change —
        // it is a units change, and the test says so rather than celebrating it.**
        //
        // The rig normalises the **sun** (`intensity` is a ratio against the world's own
        // peak, so 1.0 at its brightest) but leaves the **ambient** as the sky's own
        // absolute dome radiance, which measured **4.61765**. So the two terms of one
        // rig are in different units: the sun is a ratio and the ambient is a radiance
        // some four and a half times brighter than full scale. That is why `mean` went
        // 0.00847 → 0.87457, and it is also why the world has *fewer* distinct colours
        // than the fixture (6 against 8): an ambient that dominant flattens the
        // material differences the shading was there to show.
        //
        // **So this test asserts one unit and that the light reaches the pixels, and
        // reports the underexposure rather than asserting a brightness nobody has ruled.**
    }

    /// **What six hundred chunks and fifty thousand quads cost the CPU each frame.**
    ///
    /// "50k quads in 600 chunks" and "why are we rasterising on the CPU" are the two
    /// questions this world keeps being asked, and they have one answer between them:
    /// **the drawing is one draw call on the GPU** — every resident chunk's quads go
    /// across in one submission — and what the CPU pays for is *assembling and
    /// uploading* that geometry, every frame, whether or not anything moved.
    ///
    /// This measures that half and only that half. It times no GPU work, no driver and
    /// no swapchain. It measures it **twice**, because `crates/shploingus/src/main.rs`
    /// used to run the second and now runs the first (C-168):
    ///
    /// * **the quad format** — [`Scene::build_with_detail`], which is what the game
    ///   builds per frame now: one 24-byte [`crate::QuadRecord`] per rectangle and one
    ///   16-byte origin per chunk, both chunk-local and camera-independent;
    /// * **the vertex format it replaced** — [`Scene::build_with_light`], the four
    ///   eye-relative corners and six indices per quad the above took over from. The
    ///   cost that made this worth doing, and still the right format for a frame that
    ///   has to state a per-cell light measurement, which is what keeps it alive.
    ///
    /// **What is counted is what the CPU assembles, not what is uploaded**, and for
    /// the quad format those are two different numbers — that is the point of the
    /// format. `WorldPipeline::write_quads` compares the records and the origins and
    /// uploads only on a difference, so while the world stands still the settled
    /// window's upload is *nothing at all*. The vertex format could not do that: its
    /// positions are already relative to the eye, so one step of the camera rewrites
    /// every one of them. The upload figure needs a device and is stated rather than
    /// measured here; the assembled bytes below are measured.
    ///
    /// The second fixture is the shipped meshes **repeated** until the quad count passes
    /// 50 000. Repetition stacks every rectangle on one already there, which is exactly
    /// what makes it fair for this question: assembly is per-quad and per-vertex and
    /// knows nothing about where a quad sits. It deliberately does not measure
    /// *meshing* — that is where the shipped 605 chunks spend their twenty seconds, and
    /// it runs once per dirty chunk in the pool, not once per frame.
    ///
    /// **The times are printed and not asserted.** Nobody has ruled what this pass is
    /// allowed to cost on hardware that is not this laptop, so a threshold here would be
    /// a number I invented wearing an assertion's clothes. The byte counts *are*
    /// asserted, because they are the format rather than a measurement.
    ///
    /// Ignored because it meshes 605 chunks; about twenty seconds.
    #[test]
    #[ignore = "needs a GPU adapter and ~20 s of meshing; run explicitly with --ignored"]
    fn what_six_hundred_chunks_cost_the_cpu_every_frame() {
        use crate::game::{spawn_eye, World, GAME_SEED, GAME_WINDOW};
        use std::time::Instant;
        use voxel_shapers::{Config, WorldSpec};
        use voxel_stream::WorldSource;

        let spec = WorldSpec::new(GAME_SEED, Config::default());
        let source = WorldSource::new(spec);
        let registry = voxel_shapers::materials::registry().expect("the registry");
        let workers = std::thread::available_parallelism()
            .map(|n| n.get().saturating_sub(1).max(1))
            .unwrap_or(1);
        let mesh_config = voxel_mesh::MeshConfig::from_registry(&registry).expect("a mesh config");
        let mut streamer = voxel_work::Streamer::new(
            source.clone(),
            mesh_config,
            workers,
            crate::GAME_CEILING_BYTES,
        )
        .and_then(|s| s.with_window(GAME_WINDOW))
        .expect("the streamer");
        let eye = spawn_eye(&World::shipped());
        let eye_chunk = voxel_model::ChunkPos::new(
            (eye[0] / 32.0).floor() as i32,
            (eye[1] / 32.0).floor() as i32,
            (eye[2] / 32.0).floor() as i32,
        );
        streamer.set_centre(eye_chunk);
        streamer.settle().expect("the window settles");

        let meshes: Vec<&voxel_mesh::Mesh> = streamer.meshes().collect();
        let shipped_quads: usize = meshes.iter().map(|m| m.quads.len()).sum();
        assert!(shipped_quads > 0, "the settled window produced no quads");

        let lattice = voxel_appearance::DetailLattice::new(0);
        // **The two formats, as two constructors of one geometry.** The first is what
        // `crates/shploingus/src/main.rs` runs per frame; the second is what it ran
        // before C-168, kept as the comparison that chose against it.
        let quad_format = |chosen: &[&voxel_mesh::Mesh]| {
            Scene::build_with_detail(chosen.iter().copied(), eye, &lattice)
        };
        let no_light = |_: i32, _: i32, _: i32| None;
        let vertex_format = |chosen: &[&voxel_mesh::Mesh]| {
            Scene::build_with_light(chosen.iter().copied(), eye, &lattice, &no_light, None, None)
        };

        // **The formats first**, because every byte count below is read through them: the
        // strides are the declared ones and the writers write the fields one by one, so
        // the counts are the format rather than a padding accident.
        assert_eq!(crate::Vertex::STRIDE, 64, "the vertex stride moved");
        // **Eight slots, not six**: the record carries a rectangle *or* an astronomical body, and the
        // body's unit is the same stride (radiance in the composition, slice and `uv` slots, the light
        // direction in the extent's and the two after it). A stride pinned to 24 is the format before
        // that unit existed, which is what this assertion said until the body's own draw needed it.
        assert_eq!(crate::QuadRecord::SIZE, 32, "the quad record's stride moved");

        /// One frame of one format, as counts rather than as a stopwatch reading.
        struct Cost {
            chunks: usize,
            quads: usize,
            records: usize,
            origins: usize,
            vertices: usize,
            indices: usize,
            ms: f64,
        }

        impl Cost {
            /// **The bytes the CPU assembles for this frame, in the format the scene
            /// actually carries.** A scene in the quad format has no vertex and no index,
            /// so counting those would report a zero and call it a saving; a scene in the
            /// vertex format has no record and no origin, and for the same reason. One sum
            /// over all four, so neither format can be flattered by the arithmetic.
            fn assembled(&self) -> usize {
                self.records * crate::QuadRecord::SIZE as usize
                    + self.origins * crate::quad::ORIGIN_SIZE as usize
                    + self.vertices * crate::Vertex::STRIDE as usize
                    + self.indices * 4
            }

            /// This frame's share of a 60 Hz frame.
            fn of_a_frame(&self) -> f64 {
                100.0 * self.ms / 16.667
            }
        }

        let time_it = |label: &str,
                       build: &dyn Fn(&[&voxel_mesh::Mesh]) -> Scene,
                       chosen: &[&voxel_mesh::Mesh]| {
            // Once unmeasured, so the allocator and the caches are warm rather than the
            // first frame paying for them, then the best of three.
            let scene = build(chosen);
            assert_eq!(
                scene.vertex_bytes().len(),
                scene.vertices.len() * crate::Vertex::STRIDE as usize,
                "{label}: a vertex is not its declared stride"
            );
            assert_eq!(
                scene.index_bytes().len(),
                scene.indices.len() * 4,
                "{label}: an index is not four bytes"
            );
            assert_eq!(
                crate::QuadRecord::bytes(&scene.records).len(),
                scene.records.len() * crate::QuadRecord::SIZE as usize,
                "{label}: a record is not its declared stride"
            );
            let mut best = f64::INFINITY;
            for _ in 0..3 {
                let started = Instant::now();
                let again = build(chosen);
                std::hint::black_box(&again);
                best = best.min(started.elapsed().as_secs_f64() * 1000.0);
            }
            let cost = Cost {
                chunks: chosen.len(),
                quads: scene.quads,
                records: scene.records.len(),
                origins: scene.origins.len(),
                vertices: scene.vertices.len(),
                indices: scene.indices.len(),
                ms: best,
            };
            println!(
                "{label}: {} chunks, {} quads | {} records ({} B) + {} origins, {} vertices \
                 ({} B) + {} indices ({} B) | {} B assembled ({:.2} MB), {:.2} ms = {:.1} % of \
                 a 60 Hz frame",
                cost.chunks,
                cost.quads,
                cost.records,
                cost.records * crate::QuadRecord::SIZE as usize,
                cost.origins,
                cost.vertices,
                cost.vertices * crate::Vertex::STRIDE as usize,
                cost.indices,
                cost.indices * 4,
                cost.assembled(),
                cost.assembled() as f64 / (1024.0 * 1024.0),
                cost.ms,
                cost.of_a_frame(),
            );
            cost
        };

        let quad_shipped =
            time_it("the quad format, on the settled window", &quad_format, &meshes);
        let vertex_shipped = time_it(
            "the vertex format it replaced, same window",
            &vertex_format,
            &meshes,
        );

        // **The scaled fixture.** The settled window is *already* past fifty thousand quads
        // — that is worth stating, because the first version of this test scaled to
        // "50k" and degenerated into measuring the same scene twice. So the scaling is a
        // fixed factor instead: the same meshes four times over, which is what shows the
        // cost is linear in quads rather than quadratic, and what puts a number on a
        // window four times this one.
        let repeats = 4usize;
        let scaled: Vec<&voxel_mesh::Mesh> = (0..repeats)
            .flat_map(|_| meshes.iter().copied())
            .collect();
        let quad_big =
            time_it("the quad format, the same chunks four times over", &quad_format, &scaled);
        let vertex_big = time_it(
            "the vertex format, the same chunks four times over",
            &vertex_format,
            &scaled,
        );

        // **The per-quad figure, which is what the question was actually asking:** what one
        // rectangle costs the CPU in each format, and what the settled window's share of a
        // 60 Hz frame is. Reported, not asserted, for the reason above.
        println!(
            "per quad: quad format {:.0} B, vertex format {:.0} B — the settled window's {} \
             quads cost {:.2} ms against {:.2} ms, {:.1} % of a 60 Hz frame against {:.1} %",
            quad_shipped.assembled() as f64 / quad_shipped.quads as f64,
            vertex_shipped.assembled() as f64 / vertex_shipped.quads as f64,
            quad_shipped.quads,
            quad_shipped.ms,
            vertex_shipped.ms,
            quad_shipped.of_a_frame(),
            vertex_shipped.of_a_frame(),
        );
        // **And the second half of the answer, which is the half that was asked for.** None
        // of the above is *uploaded* per frame in the quad format: the records are
        // chunk-local and camera-independent, so `WorldPipeline::write_quads` compares them
        // and copies only on a difference. The vertex format had no such choice — every
        // position in it is already relative to the eye, so a step of the camera rewrote
        // all of it — and that is the reason the format changed rather than a wish to
        // shrink a number.
        println!(
            "while the world stands still the settled window uploads nothing (C-168: the \
             records and the origins are compared), where the vertex format re-copied all \
             {} B of it whether or not one voxel moved",
            vertex_shipped.assembled(),
        );

        // **Linearity, asserted on bytes rather than on time**, because it is the claim the
        // extrapolation rests on and it is arithmetic rather than a stopwatch: a repeated
        // mesh costs exactly its own geometry again, in either format.
        assert_eq!(quad_big.quads, quad_shipped.quads * repeats, "the repetition is not exact");
        assert_eq!(
            quad_big.records,
            quad_shipped.records * repeats,
            "the records did not scale with the repetitions"
        );
        assert_eq!(
            quad_big.origins,
            quad_shipped.origins * repeats,
            "the origins did not scale with the repetitions"
        );
        assert_eq!(
            vertex_big.quads,
            vertex_shipped.quads * repeats,
            "the repetition is not exact"
        );
        assert_eq!(
            vertex_big.vertices,
            vertex_shipped.vertices * repeats,
            "the vertices did not scale with the repetitions"
        );
        assert_eq!(
            vertex_big.indices,
            vertex_shipped.indices * repeats,
            "the indices did not scale with the repetitions"
        );
        println!(
            "linearity: quad format {} quads in {:.2} ms ({:.3} ms/1000 quads), {} quads in \
             {:.2} ms ({:.3} ms/1000 quads); vertex format {} quads in {:.2} ms ({:.3} ms/1000), \
             {} quads in {:.2} ms ({:.3} ms/1000)",
            quad_shipped.quads,
            quad_shipped.ms,
            1000.0 * quad_shipped.ms / quad_shipped.quads as f64,
            quad_big.quads,
            quad_big.ms,
            1000.0 * quad_big.ms / quad_big.quads as f64,
            vertex_shipped.quads,
            vertex_shipped.ms,
            1000.0 * vertex_shipped.ms / vertex_shipped.quads as f64,
            vertex_big.quads,
            vertex_big.ms,
            1000.0 * vertex_big.ms / vertex_big.quads as f64,
        );
    }

    /// **The quad format and the vertex format draw the same surface** (C-168).
    ///
    /// Two vertex stages now expand one rectangle: `vs_main`, fed the four corners
    /// and six indices [`Scene::build_with_light`] builds on the CPU, and `vs_quad`,
    /// which pulls the 24-byte record [`Scene::build`] emits and expands it in the
    /// shader. Only one of them can be right about the axis order, the winding, and
    /// which half of `uv` is `u` — so this renders one world through both, from the
    /// same camera, and compares the frames.
    ///
    /// **What is asserted is coverage, and what is reported is colour.** A wrong
    /// expansion moves a corner or flips a face, and either changes *which pixels
    /// carry geometry* — alpha, which this repository's passes already separate
    /// (`Frame::is_geometry`) — so the coverage counts must be equal, exactly. The
    /// colour is a different claim and is not asserted: the quad format carries no
    /// per-vertex lattice sample (32 bytes a rectangle would buy one), so wherever
    /// the fragment's ladder is silent a pixel takes the material's base from this
    /// path and the corner's sample from the other. That difference is printed as a
    /// count of pixels, so the size of the named limit is a number on the record
    /// rather than an impression.
    ///
    /// Ignored because it needs a GPU adapter.
    #[test]
    #[ignore = "needs a GPU adapter"]
    fn the_quad_format_and_the_vertex_format_draw_the_same_surface() {
        use crate::shading;
        use voxel_mesh::{Chunk, MeshConfig, Mesher};
        use voxel_shapers::{Config, WorldSpec};
        use voxel_stream::WorldSource;

        let source = WorldSource::new(WorldSpec::new(2468, Config::default()));
        let registry = voxel_shapers::materials::registry().expect("the registry");
        let table = shading::table(&registry, &shading_field()).expect("every admitted material derives");
        let gpu_rig = GpuRig::from_rig(&LightRig::standard()).expect("the standard rig fits");
        let mut renderer = Renderer::new(256, 256, &table, &gpu_rig).expect("a GPU");
        renderer.world_mut().write_no_sky_model();

        let mut mesher = Mesher::new();
        let config = MeshConfig::from_registry(&registry).expect("a mesh config");
        let mut meshes = Vec::new();
        for cy in 0..2 {
            for cx in 0..3 {
                for cz in 0..3 {
                    let pos = voxel_model::ChunkPos::new(cx, cy, cz);
                    let Ok(tree) = source.build_chunk(pos) else {
                        continue;
                    };
                    let chunk = Chunk::from_tree(pos, tree);
                    let neighborhood = source.neighborhood(pos);
                    if let Ok(mesh) = mesher.build(&chunk, &config, Some(&neighborhood)) {
                        meshes.push(mesh);
                    }
                }
            }
        }
        assert!(!meshes.is_empty(), "the world produced no chunks to draw");

        let lattice = voxel_appearance::DetailLattice::new(0);
        let no_light = |_: i32, _: i32, _: i32| None;
        let corners =
            Scene::build_with_light(meshes.iter(), [0.0; 3], &lattice, &no_light, None, None);
        let lo = |axis: usize| {
            corners.vertices.iter().map(|v| v.position[axis] as f64).fold(f64::INFINITY, f64::min)
        };
        let hi = |axis: usize| {
            corners
                .vertices
                .iter()
                .map(|v| v.position[axis] as f64)
                .fold(f64::NEG_INFINITY, f64::max)
        };
        let eye = [(lo(0) + hi(0)) / 2.0, hi(1) + 6.0, lo(2) - 24.0];
        let mut camera = Camera::standard(eye, 1.0);
        camera.pitch_rad = -0.35;

        let quads = Scene::build(meshes.iter(), eye, &lattice, None, None);
        let vertices = Scene::build_with_light(meshes.iter(), eye, &lattice, &no_light, None, None);
        assert_eq!(quads.records.len(), quads.quads, "a quad is not a record");
        assert!(quads.vertices.is_empty(), "the quad format built a corner");
        assert_eq!(vertices.vertices.len(), 4 * vertices.quads, "a quad is not four corners");

        let from_records = renderer.render(&quads, &camera).expect("a frame");
        let from_vertices = renderer.render(&vertices, &camera).expect("a frame");
        let geometry = |f: &Frame| f.pixels.iter().filter(|p| p[3] >= 0.5).count();
        let differing = from_records
            .pixels
            .iter()
            .zip(from_vertices.pixels.iter())
            .filter(|(a, b)| a != b)
            .count();
        println!(
            "quad format: {} rectangles ({} B of records) vs vertex format: {} vertices \
             ({} B); geometry {} vs {} pixels; {differing} of {} pixels differ in colour",
            quads.records.len(),
            quads.records.len() * crate::QuadRecord::SIZE as usize,
            vertices.vertices.len(),
            vertices.vertex_bytes().len(),
            geometry(&from_records),
            geometry(&from_vertices),
            from_records.pixels.len(),
        );

        // **And with the eye a metre on.** The records are camera-independent — that is this
        // format's whole claim — while the *origins* are `origin − eye` and must be re-derived for
        // the new eye even though the world-space array is identical.
        //
        // **The witness is the geometry count, not the colour count.** Measured on 2026-10-08 with
        // the eye key removed: geometry 24,675 against the rebuilt scene's 24,609, and this
        // assertion fails by name ("the two formats disagree about which pixels are geometry once
        // the eye has moved"). The scroll count is printed rather than asserted, because it moves
        // under either behaviour — a frozen world still lands on different pixels when the camera
        // itself has moved, 8,799 of them, against 23,922 with the origins following the eye — so
        // a `scrolled > 0` assertion would pass on the defect and is not offered as evidence.
        let moved = [eye[0] + 1.0, eye[1], eye[2]];
        let mut moved_camera = Camera::standard(moved, 1.0);
        moved_camera.pitch_rad = camera.pitch_rad;
        let moved_vertices =
            Scene::build_with_light(meshes.iter(), moved, &lattice, &no_light, None, None);
        let records_from_the_new_eye = renderer.render(&quads, &moved_camera).expect("a frame");
        let vertices_from_the_new_eye =
            renderer.render(&moved_vertices, &moved_camera).expect("a frame");
        let geometry_after = geometry(&records_from_the_new_eye);
        let scrolled = records_from_the_new_eye
            .pixels
            .iter()
            .zip(from_records.pixels.iter())
            .filter(|(a, b)| a != b)
            .count();
        println!(
            "one metre of eye: geometry {geometry_after} (rebuilt scene: {}) pixels; {scrolled} of \
             {} pixels differ from the frame before the step, and {differing} did before it",
            geometry(&vertices_from_the_new_eye),
            records_from_the_new_eye.pixels.len(),
        );
        assert_eq!(
            geometry_after,
            geometry(&vertices_from_the_new_eye),
            "the two formats disagree about which pixels are geometry once the eye has moved"
        );
        assert!(
            scrolled > 0,
            "one metre of eye changed nothing at all: no origin followed the eye, so no chunk moved"
        );
        assert!(geometry(&from_records) > 0, "the quad format drew no geometry");
        assert_eq!(
            geometry(&from_records),
            geometry(&from_vertices),
            "the two vertex stages cover different pixels: the expansion is not the CPU's"
        );
    }

    /// How much of a frame is *anything at all*, and how bright it gets.
    struct Stats {
        min: f64,
        max: f64,
        mean: f64,
        above_one_percent: usize,
        pixels: usize,
        colours: usize,
        /// **Distinct luminance bands over geometry pixels** — the Deadlock bar's
        /// "≥3 bands of shading response", which `colours` cannot stand in for.
        bands: usize,
    }

    impl Stats {
        /// How many luminance bands hold a real share of the geometry, in eight buckets
        /// across the observed range. A bucket only counts if at least a tenth of the
        /// geometry falls in it, so a single bright pixel cannot invent a band.
        fn bands(frame: &Frame) -> usize {
            const BUCKETS: usize = 8;
            const SHARE: f64 = 0.1;
            let geometry: Vec<f64> = frame
                .pixels
                .iter()
                .filter(|p| p[3] >= 0.5)
                .map(|p| {
                    0.2126 * p[0] as f64 + 0.7152 * p[1] as f64 + 0.0722 * p[2] as f64
                })
                .collect();
            if geometry.is_empty() {
                return 0;
            }
            let min = geometry.iter().copied().fold(f64::INFINITY, f64::min);
            let max = geometry.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let span = max - min;
            if span <= 0.0 {
                // One flat tone is zero bands whatever the pixel count says.
                return 0;
            }
            let mut counts = [0usize; BUCKETS];
            for l in &geometry {
                let bucket = (((l - min) / span) * (BUCKETS - 1) as f64).round() as usize;
                counts[bucket.min(BUCKETS - 1)] += 1;
            }
            let floor = (SHARE * geometry.len() as f64).ceil() as usize;
            counts.iter().filter(|c| **c >= floor).count()
        }

        fn of(frame: &Frame) -> Self {
            let tones = frame.luminances();
            let pixels = tones.len();
            let min = tones.iter().copied().fold(f64::INFINITY, f64::min);
            let max = tones.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let mean = tones.iter().sum::<f64>() / pixels as f64;
            Stats {
                min,
                max,
                mean,
                above_one_percent: tones.iter().filter(|t| **t > 0.01).count(),
                pixels,
                colours: frame.distinct_colours(),
                bands: Self::bands(frame),
            }
        }

        fn report(&self, what: &str) {
            println!(
                "{what}: min {:.4} max {:.4} mean {:.5} | {:.1}% of pixels above 0.01 | {} distinct colours | {} bands",
                self.min,
                self.max,
                self.mean,
                100.0 * self.above_one_percent as f64 / self.pixels as f64,
                self.colours,
                self.bands,
            );
        }
    }

    /// **The player starts on the ground, and the frame itself says so.**
    ///
    /// The camera used to float, so "where is the player" was a number in a struct
    /// and nothing checked it against the picture. Now the player has a body that
    /// obeys gravity, and this asks the only question that matters about it *in
    /// pixels*: does the bottom of the frame contain the ground the player is
    /// standing on?
    ///
    /// The instrument is [`Frame::is_geometry`], which is the renderer's own
    /// convention — the sky pass writes alpha 0, the world writes alpha 1 — so a
    /// bright sky cannot be mistaken for terrain. Without it this test would be
    /// "is the bottom of the frame not black", which a sunset would pass.
    ///
    /// The body is settled through [`Player::advance`] rather than placed, so this
    /// also proves the shipped spawn is standable: a spawn with no floor under it
    /// would have fallen out of the bottom of the frame before the first render.
    ///
    /// Ignored because it meshes 605 chunks; about twenty seconds.
    #[test]
    #[ignore = "needs a GPU adapter and ~20 s of meshing; run explicitly with --ignored"]
    fn the_player_starts_on_the_ground_and_the_frame_proves_it() {
        use crate::game::{
            spawn_feet, World, GAME_PITCH_RAD, GAME_WINDOW,
        };
        use crate::motion::{Player, Wish, PLAYER_STEP};
        use crate::GAME_CEILING_BYTES;
        use voxel_appearance::sky::SkyInputs;
        use voxel_shapers::earthworks::Earthworks;
        use voxel_stream::WorldSource;

        const W: usize = 1280;
        const H: usize = 720;

        let world = World::shipped();
        let spec = *world.spec();
        let earthworks = Earthworks::new();

        // Two seconds of standing still, through the real path, under real gravity.
        // The streamer is absent here — it is built further down — so this settles
        // against the generator, which is the correct fallback for the frames before
        // the world exists and is the reason that fallback exists at all.
        let mut player = Player::new(spawn_feet(&world));
        let voxels = world.voxels(None, &earthworks);
        for _ in 0..120 {
            player.advance(&Wish::STILL, 0.0, PLAYER_STEP, &voxels);
        }
        let spawn_feet_y = spawn_feet(&world)[1];
        assert!(
            (player.motion.feet[1] - spawn_feet_y).abs() < 1e-9,
            "the body fell from {spawn_feet_y} m to {} m — the spawn has no floor",
            player.motion.feet[1]
        );
        assert!(player.motion.grounded, "the body is not standing on the spawn");

        let eye = player.eye();
        println!("settled eye {eye:?}, feet {:?}, sea level {} m", player.motion.feet, spec.config.sea_level_m);
        assert!(eye[1] > spec.config.sea_level_m, "the eye is below the waterline");

        let source = WorldSource::new(spec);
        let registry = voxel_shapers::materials::registry().expect("the registry");
        let table = shading::table(&registry, &shading_field()).expect("every admitted material derives");
        let workers = std::thread::available_parallelism()
            .map(|n| n.get().saturating_sub(1).max(1))
            .unwrap_or(1);
        let mesh_config = voxel_mesh::MeshConfig::from_registry(&registry).expect("a mesh config");
        let mut streamer =
            voxel_work::Streamer::new(source, mesh_config, workers, GAME_CEILING_BYTES)
                .and_then(|s| s.with_window(GAME_WINDOW))
                .expect("the streamer");
        streamer.set_centre(voxel_model::ChunkPos::new(
            (eye[0] / 32.0).floor() as i32,
            (eye[1] / 32.0).floor() as i32,
            (eye[2] / 32.0).floor() as i32,
        ));
        streamer.settle().expect("the window settles");

        let day = crate::skylight::daylight_of(
            &spec,
            voxel_shapers::climate::TICKS_PER_DAY / 2,
            SkyInputs::default(),
            0,
        )
        .expect("a sky at noon");
        let fixture =
            GpuRig::from_rig(&crate::skylight::exposed(&day.rig)).expect("the noon rig fits");
        let mut renderer = Renderer::new(W as u32, H as u32, &table, &fixture).expect("a GPU");
        renderer.world_mut().write_no_sky_model();
        let scene = Scene::build_with_detail(streamer.meshes(), eye, &voxel_appearance::DetailLattice::new(0));
        let mut camera = Camera::standard(eye, W as f64 / H as f64);
        camera.pitch_rad = GAME_PITCH_RAD;
        let frame = renderer.render(&scene, &camera).expect("a frame");

        // The bottom sixth of the frame is the ground at the player's feet; the top
        // sixth is what the player is looking at over it.
        let band = H / 6;
        let geometry_in = |rows: std::ops::Range<usize>| -> f64 {
            let mut n = 0usize;
            let mut total = 0usize;
            for row in rows {
                for col in 0..W {
                    total += 1;
                    if Frame::is_geometry(frame.pixels[row * W + col]) {
                        n += 1;
                    }
                }
            }
            n as f64 / total as f64
        };
        let bottom = geometry_in(H - band..H);
        let top = geometry_in(0..band);
        println!(
            "geometry fraction: bottom sixth {bottom:.3}, top sixth {top:.3}; \
             frame min {:.4} max {:.4} mean {:.5}",
            Stats::of(&frame).min,
            Stats::of(&frame).max,
            Stats::of(&frame).mean
        );

        assert!(
            bottom > 0.9,
            "only {:.1}% of the bottom sixth is ground — the camera is not standing on it",
            100.0 * bottom
        );
        // And the picture is not black, which is a separate failure and was one.
        assert!(Stats::of(&frame).above_one_percent > frame.pixels.len() / 100);
    }

    /// **Is the window actually showing the world?**
    ///
    /// A player reported a black screen while the HUD reported 21,526 quads, and
    /// those two statements are only compatible in a small number of ways. This
    /// test is the tie-breaker: it streams the **shipped** window (built from
    /// [`crate::GAME_WINDOW`], so it cannot drift from the bin), settles it, and
    /// renders it from the shipped eye with the same pipeline the window uses —
    /// then measures how much of the frame is anything but the clear colour.
    ///
    /// # It failed, and then the defect it named was fixed
    ///
    /// Before `docs/LEDGER.md` C-39 the answer was **0.0 % of pixels, `max 0.0000`,
    /// two distinct raw pixels** (`[0,0,0,0]` and `[0,0,0,1]`): the terrain
    /// rasterised full-frame and every fragment was black, because no material
    /// derived a `base_color` and `regolith` — 83 % of the visible quads — derives
    /// a roughness of exactly 1.0, so its rim is exactly 0 (C-38). Once the
    /// material record carried a sourced reflectance, the same test on the same
    /// world measured **55.5 % of pixels above 0.01, `max 0.2012`, 10 distinct
    /// colours**. So this is now a guard rather than a recorded defect, and it is
    /// the guard that would have caught the black screen a round earlier.
    ///
    /// Ignored because it meshes 605 chunks; about twenty seconds.
    #[test]
    #[ignore = "needs a GPU adapter and ~20 s of meshing; run explicitly with --ignored"]
    fn the_shipped_view_is_not_a_black_screen() {
        use crate::game::{spawn_eye, World, GAME_PITCH_RAD, GAME_SEED, GAME_WINDOW};
        use crate::ShadingReport;
        use voxel_shapers::{Config, WorldSpec};
        use voxel_stream::WorldSource;

        let spec = WorldSpec::new(GAME_SEED, Config::default());
        let source = WorldSource::new(spec);
        let registry = voxel_shapers::materials::registry().expect("the registry");
        let table = shading::table(&registry, &shading_field()).expect("every admitted material derives");
        let rig = LightRig::standard();
        let gpu_rig = GpuRig::from_rig(&rig).expect("the standard rig fits");

        let workers = std::thread::available_parallelism()
            .map(|n| n.get().saturating_sub(1).max(1))
            .unwrap_or(1);
        let mesh_config = voxel_mesh::MeshConfig::from_registry(&registry).expect("a mesh config");
        let mut streamer =
            voxel_work::Streamer::new(source.clone(), mesh_config, workers, crate::GAME_CEILING_BYTES)
                .and_then(|s| s.with_window(GAME_WINDOW))
                .expect("the streamer");
        // The window is centred where the shipped client centres it, which is
        // the eye's own chunk.
        let eye = spawn_eye(&World::shipped());
        println!("spawn eye: {eye:?}");
        let eye_chunk = voxel_model::ChunkPos::new(
            (eye[0] / 32.0).floor() as i32,
            (eye[1] / 32.0).floor() as i32,
            (eye[2] / 32.0).floor() as i32,
        );
        streamer.set_centre(eye_chunk);
        println!("eye chunk: {eye_chunk:?}");
        let settled = streamer.settle().expect("the window settles");
        println!(
            "streamed {} of {} chunks, {} meshes held, {:.1} MB",
            settled.resident,
            settled.wanted,
            streamer.len(),
            settled.bytes as f64 / (1024.0 * 1024.0)
        );

        // Where the meshed geometry actually is, so the numbers below are about
        // the world rather than about a camera that missed it.
        let placed = Scene::build_with_detail(streamer.meshes(), [0.0; 3], &voxel_appearance::DetailLattice::new(0));
        let axis = |i: usize, lo: bool| {
            let f = if lo { f64::min } else { f64::max };
            placed
                .vertices
                .iter()
                .map(|v| v.position[i] as f64)
                .fold(if lo { f64::INFINITY } else { f64::NEG_INFINITY }, f)
        };
        println!(
            "meshed extents: x {}..{}, y {}..{}, z {}..{}",
            axis(0, true), axis(0, false), axis(1, true), axis(1, false), axis(2, true), axis(2, false)
        );

        // ---- why the picture can be black at all --------------------------
        // The shading table is derived, not authored, so it is *inspectable*:
        // which of the two terms that survive without an albedo each material
        // actually gets.
        for material in registry.materials() {
            let inputs = shading::inputs_for(&registry, material.id)
                .expect("every admitted material derives or the table would be short");
            let base = matches!(inputs.base_color, voxel_appearance::openpbr::Colour::Admitted(_));
            println!(
                "  material {:>2} {:<12} {:?} density {:.0} kg/m3 | base colour {} | roughness {:?}",
                material.id.0,
                material.name,
                material.optical_class,
                material.bulk_density_kg_m3().to_f64(),
                if base { "present" } else { "ABSENT" },
                inputs.specular_roughness,
            );
        }
        let mut histogram: std::collections::BTreeMap<u32, usize> = Default::default();
        for mesh in streamer.meshes() {
            for quad in &mesh.quads {
                *histogram.entry(quad.material.0 as u32).or_default() += 1;
            }
        }
        println!("  quads by material: {histogram:?}");

        // The surface the player stands on, so the start position below is
        // placed from the world rather than guessed at.
        for (x, z) in [(0i32, -96i32), (0, -64), (0, 0), (0, 40)] {
            let top = (-16..48).rev().find(|y| {
                source.material_at(x, *y, z).map(|m| !m.is_empty()).unwrap_or(false)
            });
            let what = top.and_then(|y| source.material_at(x, y, z).ok());
            println!("  column x {x}, z {z}: topmost solid y = {top:?} material {what:?}");
        }

        let mut renderer = Renderer::new(1280, 720, &table, &gpu_rig).expect("a GPU");
        renderer.world_mut().write_no_sky_model();
        println!("material table rows: {}", renderer.material_rows());
        let mut look = |name: &str, eye: [f64; 3], pitch: f64| {
            let scene = Scene::build_with_detail(streamer.meshes(), eye, &voxel_appearance::DetailLattice::new(0));
            let mut camera = Camera::standard(eye, 1280.0 / 720.0);
            camera.pitch_rad = pitch;
            let frame = renderer.render(&scene, &camera).expect("a frame");
            println!(
                "  scene: {} chunks, {} quads, {} vertices, {} indices",
                scene.quads,
                scene.quads,
                scene.vertices.len(),
                scene.indices.len()
            );
            let mut seen: std::collections::BTreeSet<[u32; 4]> = Default::default();
            for p in &frame.pixels {
                seen.insert([
                    p[0].to_bits(),
                    p[1].to_bits(),
                    p[2].to_bits(),
                    p[3].to_bits(),
                ]);
            }
            println!("  distinct raw pixels: {}", seen.len());
            for bits in seen.iter().take(4) {
                println!(
                    "    {:?}",
                    bits.map(f32::from_bits)
                );
            }
            for (x, y) in [(640u32, 719u32), (640, 600), (640, 480), (640, 360), (2, 2)] {
                println!("    pixel({x},{y}) = {:?}", frame.pixel(x, y));
            }
            Stats::of(&frame).report(name);
            Stats::of(&frame)
        };

        // The shipped view, then the one it replaced, so the comparison is a
        // measurement rather than a memory of what the old one looked like.
        let shipped = look("shipped eye", eye, GAME_PITCH_RAD);
        look("previous eye [0,45,-160] pitch -0.25", [0.0, 45.0, -160.0], -0.25);

        assert!(settled.resident > 500, "the window did not fill: {}", settled.resident);
        // The requirement a player stated, as an assertion: most of the window
        // must be the world. Before C-39 this was 0.0 %.
        assert!(
            shipped.above_one_percent * 4 > shipped.pixels,
            "fewer than a quarter of the shipped view's pixels show anything: {:.1}% \
             ({} of {} materials carry a base colour)",
            100.0 * shipped.above_one_percent as f64 / shipped.pixels as f64,
            ShadingReport::of(&table).with_base_colour,
            ShadingReport::of(&table).materials,
        );
        // Not just not-black: **shaded**. A single flat clear colour would pass
        // the test above and be no more a picture than the black screen was.
        assert!(shipped.colours > 4, "the shipped view has {} colours", shipped.colours);
        // **The Deadlock bar, which `colours` cannot stand in for**: distinct *colours* can
        // come from hue alone at one luminance, so a frame of one flat tone in many hues
        // passes the assertion above and still shows no shading. Bands are counted over
        // geometry pixels only, so the sky cannot contribute.
        assert!(
            shipped.bands >= 3,
            "the shipped view shows {} luminance band(s) over its geometry; the bar is 3",
            shipped.bands
        );
        assert!(shipped.max > 0.05, "the brightest pixel is {} — that is a black screen", shipped.max);
        // And it is *dim*, which is a named finding rather than a failure: the
        // standard rig's ambient is deliberately dim and no material derives a
        // `specular_weight`, so the only highlight source is the rim, which the
        // most common material's roughness switches off. The next appearance row
        // is lighting, not colour.
        println!("mean luminance {:.5} — the world is visible and dim", shipped.mean);
    }

    #[test]
    #[ignore = "needs a GPU adapter; run explicitly with --ignored"]
    fn the_sky_is_drawn_where_there_is_no_geometry_and_never_claims_to_be_geometry() {
        use voxel_shapers::{Config, WorldSpec};

        let spec = WorldSpec::new(20_260_930, Config::default());
        // Tick 9000: the Sun is up, so this is a real sky and not the midnight all-zero case
        // that would make every assertion below pass for the wrong reason.
        let model = sky_model_of(&spec, 9_000);
        assert_eq!(
            model.bytes().len(),
            crate::skylight::SKY_MODEL_BYTES,
            "the model is not the row count its own layout declares"
        );

        let registry = voxel_shapers::materials::registry().expect("the registry");
        let materials = crate::shading::table(&registry, &shading_field()).expect("every admitted material derives");
        let rig = GpuRig::from_rig(&LightRig::standard()).expect("the standard rig fits");

        let mut renderer = Renderer::new(64, 64, &materials, &rig).expect("a GPU");
        renderer.world().write_sky_model(&model);

        // **No geometry at all.** This is precisely the case the old pass left empty:
        // with no voxel on a pixel there was nothing there but the clear value.
        let scene =
            Scene::build_with_detail(
                Vec::<voxel_mesh::Mesh>::new().iter(),
                [0.0; 3],
                &voxel_appearance::DetailLattice::new(0),
            );
        let mut camera = Camera::standard([0.0, 20.0, 0.0], 1.0);
        camera.pitch_rad = 0.45;

        let frame = renderer.render(&scene, &camera).expect("a frame");
        let (sky_pixels, geometry_pixels) = frame.split();

        assert_eq!(geometry_pixels, 0, "an empty scene drew geometry");
        assert_eq!(
            sky_pixels,
            64 * 64,
            "the sky pass left {} pixels undrawn — the clear is still winning",
            64 * 64 - sky_pixels
        );

        // It is a sky and not the clear colour: the clear was transparent black.
        let brightest = frame.luminances().into_iter().fold(0.0_f64, f64::max);
        assert!(
            brightest > 0.01,
            "every pixel is black, so the pass drew nothing: max {brightest}"
        );

        // **Alpha 0 on all of them.** Sky must never be counted as a shaded surface,
        // or every shading measurement silently includes the backdrop.
        assert!(
            frame.pixels.iter().all(|p| p[3] < 0.5),
            "the sky claimed alpha, so it reads as geometry"
        );

        // And the model is being *read*: looking up is not the same picture as looking down, which
        // clamps to the horizon. A pass that ignored the ray and filled one flat colour would fail
        // exactly here.
        let up_mean = frame.luminances().into_iter().sum::<f64>() / frame.luminances().len() as f64;

        let mut down = Camera::standard([0.0, 20.0, 0.0], 1.0);
        down.pitch_rad = -0.45;
        let down_frame = renderer.render(&scene, &down).expect("a frame looking down");
        let down_mean =
            down_frame.luminances().into_iter().sum::<f64>() / down_frame.luminances().len() as f64;

        assert!(
            (up_mean - down_mean).abs() > 1e-4,
            "looking up ({up_mean:.6}) and looking down ({down_mean:.6}) are the same \
             picture, so the sky is not being evaluated at the ray's own direction"
        );
        println!("sky mean: up {up_mean:.5}, down {down_mean:.5}, peak {brightest:.5}");
    }

    /// **The sky a test hands the pipeline**, from the game's own model: the world's air and the
    /// solver's bands over it, normalised by the world's own peak sun. It is the object
    /// `shploingus` uploads every frame, so a test's expectation and the frame's pixels come from
    /// one model rather than from a fixture beside it.
    fn sky_model_of(spec: &voxel_shapers::WorldSpec, tick: u64) -> crate::skylight::SkyModel {
        let inputs = voxel_appearance::sky::SkyInputs::default();
        let sky = voxel_appearance::sky::Sky::at(spec, tick, inputs, 0).expect("a sky");
        let peak = crate::skylight::peak_sun_luminance(spec, inputs);
        let air = crate::sun::air_at(spec, 0.0, 0.0, tick);
        crate::skylight::SkyModel::of(&sky, &air, peak, &voxel_shapers::sky::moon_position(spec, tick, 0))
    }

    /// **A water surface reflects the directional sky, and the GPU agrees with the
    /// model about it.**
    ///
    /// The user's ruling is why this test exists (`LEDGER.md` C-157.6): *"the sun and
    /// it's moon refletions is atually core to the rendering of the world itself, and the
    /// materials"*. A water probe — the water material's own derived inputs, a **tilted**
    /// normal so the body and the reflection are both visible — is drawn with the real sky
    /// model uploaded the way the window uploads it, and the pixels are compared
    /// against [`crate::shading::fluid_surface_cpu`], which reads the same rows through
    /// `sky_exposed_at`. That comparison is what holds `shading.wgsl`'s transcription of
    /// `voxel_appearance::fluid` **and** of the transport to one model, the same way
    /// `the_gpu_shades_a_surface_exactly_as_the_crate_does` holds the NPR terms.
    ///
    /// The second half is the claim a still cannot make: **the reflection follows the
    /// Sun**. Two normals are chosen to send the mirror ray to the same elevation, one
    /// at the Sun's own azimuth and one opposite; the Sun-side water must be the brighter
    /// of the two, or the table is decoration.
    #[test]
    #[ignore = "needs a GPU adapter; run explicitly with --ignored"]
    fn the_water_surface_reflects_the_sky_the_model_says_it_does() {
        use crate::shading::{fluid_surface_cpu, inputs_for, shade_cpu};
        use crate::skylight::{daylight_of, exposed, peak_sun_luminance};
        use voxel_appearance::sky::SkyInputs;
        use voxel_shapers::materials;
        use voxel_shapers::{Config, WorldSpec};

        const SIDE: u32 = 65;
        const CENTRE: u32 = 32;

        let spec = voxel_shapers::WorldSpec::new(2468, voxel_shapers::Config::default());
        let inputs = SkyInputs::default();
        // Tick 9000 is the shipped client's own start tick: the Sun is up, so the sky
        // has a direction in it and this is not the midnight all-zero case.
        let day = daylight_of(&spec, 9_000, inputs, 0).expect("a sky at the game's start tick");
        let peak = peak_sun_luminance(&spec, inputs);
        let rig = exposed(&day.rig);

        let registry = materials::registry().expect("the registry");
        let water = inputs_for(&registry, materials::WATER).expect("water derives");
        let table = vec![
            MaterialParams::default(),
            MaterialParams::from_inputs(&water, MESH_DOES_NOT_CARRY_OPEN),
        ];
        let gpu_rig = GpuRig::from_rig(&rig).expect("the game's rig fits");
        let mut renderer = Renderer::new(SIDE, SIDE, &table, &gpu_rig).expect("a GPU");
        // **The model exactly as the window sends it**, and the same rows read on this side for the
        // CPU comparison: the exposure is inside the model, so the two are compared through one
        // exposure rather than through two that have to agree.
        let model = sky_model_of(&spec, 9_000);
        renderer.world().write_sky_model(&model);

        let view = Vec3::new(0.0, 0.0, -DISTANCE_M as f64);
        // **The mirror geometry, inverted**: for a stated reflection direction `r`, the
        // normal that produces it from the centre view is the half-vector of the two
        // **unit** directions, `normalize(v̂ + r)` — the un-normalised `view` would tilt
        // the normal and the mirror ray would land somewhere else entirely.
        let normal_for = |r: Vec3| (view.normalise() + r).normalise();

        let probe = |renderer: &mut Renderer, n: Vec3| -> ([f32; 4], LinearRgb, LinearRgb) {
            let nf = [n.x as f32, n.y as f32, n.z as f32];
            let scene = Scene::probe(nf, 1, DISTANCE_M, 2.0);
            let camera = Camera::standard([0.0, 0.0, 0.0], 1.0);
            let frame = renderer.render(&scene, &camera).expect("a frame");
            let got = frame.pixel(CENTRE, CENTRE).expect("the centre is in range");
            assert_eq!(got[3], 1.0, "the probe must cover the centre");
            let (expected, _) = fluid_surface_cpu(&water, n, view, 0.0, &rig, &model)
                .expect("the water shades");
            let (body, _) = shade_cpu(&water, n, view, 0.0, &rig).expect("the body shades");
            (got, expected, body)
        };

        // ---- the transcription is the model ----
        let tilt = Vec3::new(0.0, 0.6, -0.8);
        let (got, expected, body) = probe(&mut renderer, tilt);
        let worst = (0..3)
            .map(|c| (got[c] as f64 - [expected.r, expected.g, expected.b][c]).abs())
            .fold(0.0_f64, f64::max);
        println!(
            "water probe: gpu {got:?}, model ({:.5}, {:.5}, {:.5}), body ({:.5}, {:.5}, {:.5}), \
             worst disagreement {worst:e}",
            expected.r, expected.g, expected.b, body.r, body.g, body.b
        );
        assert!(worst < 2e-3, "the GPU water and the model disagree by {worst}");
        // **And the reflection is what the pixels gained**, or the shader is drawing a
        // matte sheet with a table bound beside it.
        // **And the reflection is in the pixels**, moving them where the model moves them. It need not
        // be *brighter* than the matte body: a fluid composes `colour·(1−F) + reflected·F`, a convex
        // mix, so a sky dimmer than the surface's own shading makes the water dimmer and at this tick
        // the reflected sky (0.019) is just under the body's own shading (0.0192). What the claim can be
        // is that the change is the model's change — same sign, and not nothing — which a shader with no
        // reflection at all fails by leaving the pixel exactly at the body's own value.
        let lum = |c: [f64; 3]| 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
        let got_lum = lum([got[0] as f64, got[1] as f64, got[2] as f64]);
        let (body_lum, model_lum) = (body.luminance(), lum([expected.r, expected.g, expected.b]));
        let (change, model_change) = (got_lum - body_lum, model_lum - body_lum);
        println!(
            "  the reflection moves the pixel by {change:.6} of {got_lum:.6} ({:.1} %), where the \
             model moves it by {model_change:.6}",
            100.0 * change / got_lum
        );
        assert!(
            model_change.abs() > 1e-9,
            "the model's own reflection changes nothing here ({model_lum:.9} against the body's \
             {body_lum:.9}), so this frame has no reflection to find in the pixels"
        );
        assert!(
            change != 0.0 && (change > 0.0) == (model_change > 0.0),
            "the reflection moved the pixel by {change:.9} where the model moves it by \
             {model_change:.9}: the pixels are not carrying the reflection the model gives them"
        );

        // ---- and the reflection follows the Sun ----
        // One elevation, two azimuths: the Sun's own, and the one opposite. The mirror
        // ray at 20° toward the Sun looks through the aureole; the anti-sun ray does not.
        let elevation = 20.0_f64.to_radians();
        let ray_at = |azimuth_deg: f64| {
            let a = azimuth_deg.to_radians();
            Vec3::new(
                a.sin() * elevation.cos(),
                elevation.sin(),
                -a.cos() * elevation.cos(),
            )
        };
        let sun_azimuth = day.sky.sun_azimuth_deg;
        let (toward, _, _) = probe(&mut renderer, normal_for(ray_at(sun_azimuth)));
        let (away, _, _) = probe(&mut renderer, normal_for(ray_at(sun_azimuth + 180.0)));
        let at = |p: [f32; 4]| lum([p[0] as f64, p[1] as f64, p[2] as f64]);
        println!(
            "  water toward the Sun's azimuth ({sun_azimuth:.1}°): {:.5}; away: {:.5}",
            at(toward),
            at(away)
        );
        assert!(
            at(toward) > at(away),
            "the water does not follow the Sun: {:.5} toward, {:.5} away",
            at(toward),
            at(away)
        );
    }

    /// **The cloud as a veil, on real pixels** (C-179).
    ///
    /// Three claims, and each one is a thing a still can settle:
    ///
    /// 1. **The veil reaches the frame.** Over a cloud the target's alpha is `1 − transmittance`
    ///    — the same number `cloud_cubes` put in the origin's `w` and `voxel-stream::light` walks
    ///    in the shadow — so the picture and the shadow are one measurement rather than two.
    /// 2. **More water is more cloud.** Two fields, one wetter than the other, put strictly more
    ///    of the pixel on the cloud: the alpha rises and the pixel moves. A shader that read the
    ///    veil but ignored it would pass the first claim by accident and fail this one.
    /// 3. **A frame with no cloud is the frame it was.** An empty scene's alpha over the sky is
    ///    still what the sky wrote, so adding clouds cannot have changed anything else.
    ///
    /// **The `F4` view is the merge's own rectangles, outlined** — checked where the view is
    /// actually drawn, on the same offscreen target every other shading claim is measured on.
    ///
    /// Three claims, each one a wrong wireframe could fail: the lines are drawn **at all** (the
    /// toggle used to draw the shaded world, so nothing in the frame moved); the materials are
    /// **withheld** (a filled surface would put cloud grey across the rectangles' interiors); and
    /// the depth cue is a **hue**, so a line pixel is saturated while every achromatic thing this
    /// frame can otherwise hold — the sky, a shaded material — has its channels together.
    #[test]
    #[ignore = "needs a GPU adapter; run explicitly with --ignored"]
    fn the_wireframe_outlines_the_records_and_withholds_the_materials() {
        use crate::cloud_cubes::CloudCubes;
        use crate::cloud_field::{CloudField, CLOUD_COLUMNS, CLOUD_COLUMN_M};
        use voxel_appearance::npr::LightRig;

        const SIDE: u32 = 65;
        const I: u32 = 10;
        /// **Near enough that the rectangles' own borders cross the frame.** A cloud 300 m off is
        /// thirteen pixels wide and one tall, and a wireframe of it is a handful of pixels however
        /// well the pass draws; at 60 m the column's front face is wider than the view, so its top
        /// and bottom edges run the frame's whole width and a missing outline cannot be mistaken
        /// for a small one.
        const AT_Z: f64 = 60.0;
        const BASE: f32 = -2.0;
        const TOP: f32 = 2.0;

        // One cloudy column centred on the view axis, so the camera looks straight into a cloud's
        // own cells and their rectangles are what fills the frame.
        let n = CLOUD_COLUMNS as usize;
        let mut columns = vec![[0.0f32; 4]; n * n];
        columns[I as usize * n + I as usize] = [0.8, BASE, TOP, 1.0];
        let field = CloudField {
            origin_m: [
                -((I as f64) + 0.5) * CLOUD_COLUMN_M,
                AT_Z - ((I as f64) + 0.5) * CLOUD_COLUMN_M,
            ],
            columns,
            slab_base_m: BASE as f64,
            slab_top_m: TOP as f64,
            cloudy: 1,
            filled_micros: 0,
        };

        let rig = LightRig::standard();
        let table = vec![MaterialParams::default(); 2];
        let gpu_rig = GpuRig::from_rig(&rig).expect("the standard rig fits");
        let mut renderer = Renderer::new(SIDE, SIDE, &table, &gpu_rig).expect("a GPU");
        // **The world's own sky** rather than a fixture beside it: what these tests measure is how
        // a cloud's veil and a cloud's rectangles land on the sky the game draws.
        let spec = voxel_shapers::WorldSpec::new(2468, voxel_shapers::Config::default());
        renderer.world().write_sky_model(&sky_model_of(&spec, 9_000));

        let camera = Camera::standard([0.0, 0.0, 0.0], 1.0);
        let eye = [0.0, 0.0, 0.0];
        let bare = Scene::build_with_detail(
            Vec::new().iter(),
            eye,
            &voxel_appearance::DetailLattice::new(0),
        );
        let cubes = CloudCubes::build(&field, eye, 1.7e-3);
        assert!(
            !cubes.records.is_empty(),
            "the fixture drew no rectangle to outline: {} cells at {} levels, {:.1} m leaves,              {} rects",
            cubes.cells,
            cubes.subdivisions,
            cubes.cell_m,
            cubes.records.len()
        );
        let scene = bare.clone().with_clouds(&cubes);

        // A pixel that is a line: the wireframe's hue is fully saturated, so one channel sits near
        // zero while another sits well above it. Nothing else in this frame can: the dome is a
        // uniform white and a shaded surface here is achromatic.
        let is_line = |px: [f32; 4]| {
            let most = px[0].max(px[1]).max(px[2]);
            let least = px[0].min(px[1]).min(px[2]);
            most > 0.25 && least < 0.1
        };
        let lines_in = |frame: &Frame| {
            let mut lines = 0;
            for y in 0..SIDE {
                for x in 0..SIDE {
                    if is_line(frame.pixel(x, y).expect("the pixel is in range")) {
                        lines += 1;
                    }
                }
            }
            lines
        };

        let shaded = renderer.render(&scene, &camera).expect("a shaded frame");
        let wire = renderer
            .render_wireframe(&scene, &camera)
            .expect("a wireframe frame");
        let sky = renderer.render(&bare, &camera).expect("a bare frame");
        let (wire_lines, shaded_lines, sky_lines) =
            (lines_in(&wire), lines_in(&shaded), lines_in(&sky));
        println!(
            "F4 wireframe: {wire_lines} line pixels of {}; the shaded cloud frame has              {shaded_lines} and the bare sky {sky_lines}",
            SIDE * SIDE
        );

        // 1. **The outlines are drawn.** This is the claim the unwired toggle failed: it drew the
        //    shaded world, so the frame the debug view showed was the frame it replaced. The
        //    fixture's front face is wider than the view, so its own top and bottom edges each run
        //    the frame's full width: two of those is the least a wired outline can produce.
        assert!(
            wire_lines > 2 * SIDE,
            "the wireframe drew {wire_lines} line pixels of {} — the merge's rectangles are not \
             outlined (two full-width edges alone would be {})",
            SIDE * SIDE,
            2 * SIDE
        );
        // 2. **So a saturated pixel in this test is a line**, which is what makes claim 1 mean
        //    what it says: neither the shaded frame nor the bare sky has one.
        assert_eq!(
            (shaded_lines, sky_lines),
            (0, 0),
            "a frame with no wireframe reported saturated pixels, so a line pixel is not a line"
        );
        // 3. **And the materials are withheld**: the rectangles are outlined and not filled, so the
        //    frame is mostly still the sky the pass drew them against.
        assert!(
            wire_lines < SIDE * SIDE / 2,
            "{wire_lines} of {} pixels are lines — the interiors were filled as well as outlined",
            SIDE * SIDE
        );
    }

    #[test]
    #[ignore = "needs a GPU adapter; run explicitly with --ignored"]
    fn the_cloud_is_a_veil_and_its_alpha_is_the_columns_own_transmittance() {
        use crate::cloud_cubes::CloudCubes;
        use crate::cloud_field::{cloud_transmittance, CloudField, CLOUD_COLUMNS, CLOUD_COLUMN_M};
        use voxel_appearance::LinearRgb;
        use voxel_appearance::npr::LightRig;

        const SIDE: u32 = 65;
        const CENTRE: u32 = 32;
        /// Where the column's centre is put in front of the camera, metres.
        const AT_Z: f64 = 300.0;
        const I: u32 = 10;

        /// A field holding **one cloudy column**, centred on the view axis: the camera looks
        /// down `+Z` at yaw zero, so the column's own centre is placed there, and the layer
        /// brackets the axis so the centre pixel is inside a cell rather than on a seam.
        ///
        /// **The layer is one cell deep on purpose.** A cell carries its own thickness's share of
        /// its column's transmittance (`cloud_cubes`), so it is only for a column that is one cell
        /// that the fragment's veil *is* the column's own number. That is what claim 1 below can
        /// then state without qualification, and the layering's own arithmetic — the product of a
        /// column's cells is its `cloud_transmittance` — is pinned where it is computed.
        const BASE: f32 = -2.0;
        const TOP: f32 = 2.0;
        let field = |water: f32| -> CloudField {
            let n = CLOUD_COLUMNS as usize;
            let mut columns = vec![[0.0f32; 4]; n * n];
            columns[I as usize * n + I as usize] = [water, BASE, TOP, 1.0];
            CloudField {
                origin_m: [
                    -((I as f64) + 0.5) * CLOUD_COLUMN_M,
                    AT_Z - ((I as f64) + 0.5) * CLOUD_COLUMN_M,
                ],
                columns,
                slab_base_m: BASE as f64,
                slab_top_m: TOP as f64,
                cloudy: 1,
                filled_micros: 0,
            }
        };
        let veil_of = |water: f32| {
            cloud_transmittance(water as f64 * 2.0 * (TOP - BASE) as f64 / std::f64::consts::PI)
        };

        let rig = LightRig::standard();
        let table = vec![MaterialParams::default(); 2];
        let gpu_rig = GpuRig::from_rig(&rig).expect("the standard rig fits");
        let mut renderer = Renderer::new(SIDE, SIDE, &table, &gpu_rig).expect("a GPU");
        // **The world's own sky**, sent the way the window sends it, so a pixel's backdrop is the
        // sky the game draws and the blend the veil shows is measured against it rather than
        // against a fixture.
        let spec = voxel_shapers::WorldSpec::new(2468, voxel_shapers::Config::default());
        renderer.world().write_sky_model(&sky_model_of(&spec, 9_000));

        let camera = Camera::standard([0.0, 0.0, 0.0], 1.0);
        let eye = [0.0, 0.0, 0.0];
        let pixel = |renderer: &mut Renderer, cubes: Option<&CloudCubes>| -> [f32; 4] {
            let scene = Scene::build_with_detail(Vec::new().iter(), eye, &voxel_appearance::DetailLattice::new(0));
            let scene = match cubes {
                Some(cubes) => scene.with_clouds(cubes),
                None => scene,
            };
            let frame = renderer.render(&scene, &camera).expect("a frame");
            frame.pixel(CENTRE, CENTRE).expect("the centre is in range")
        };

        let bare = pixel(&mut renderer, None);
        assert_eq!(bare[3], 0.0, "a sky with no geometry wrote alpha — it is not geometry");

        let thin = CloudCubes::build(&field(0.2), eye, 1.7e-3);
        let thick = CloudCubes::build(&field(0.8), eye, 1.7e-3);
        assert!(
            !thin.records.is_empty() && !thick.records.is_empty(),
            "the fixture drew no cloud: thin {} cells / {} rects, thick {} cells / {} rects,              {:.1} m leaves, {} levels",
            thin.cells,
            thin.records.len(),
            thick.cells,
            thick.records.len(),
            thin.cell_m,
            thin.subdivisions
        );
        // **The geometry is a tree over the atmosphere, and the fixture is one column of it.**
        // The retired lattice could say "the lane is one row deep" and let the alpha below be the
        // column's own number; an octree subdivides where the model's profile crosses the cut, so
        // the eye ray crosses a *stack* of cells and the pixel shows the last covering fragment's
        // own veil. What this test still holds is everything that does not depend on how the
        // cells are cut: the alpha is a veil and not a surface, it grows with the water, and the
        // shade is the cloud's own row.
        assert!(
            thin.subdivisions >= 1 && thin.cells > 0 && thick.cells > 0,
            "the fixture built no tree: {} levels, {} and {} cells",
            thin.subdivisions,
            thin.cells,
            thick.cells
        );
        assert_eq!(
            (thin.clipped, thick.clipped),
            (0, 0),
            "the fixture's rectangles left the record format's own range"
        );
        let thin_px = pixel(&mut renderer, Some(&thin));
        let thick_px = pixel(&mut renderer, Some(&thick));
        let (thin_veil, thick_veil) = (veil_of(0.2), veil_of(0.8));
        println!(
            "cloud veil: thin water 0.2 -> the whole column is T {thin_veil:.4} for alpha {:.4}; \
             thick water 0.8 -> T {thick_veil:.4} for alpha {:.4}",
            thin_px[3], thick_px[3]
        );

        // 1. **The alpha is a veil and not a surface**: a share of the pixel is cloud and the rest
        //    is whatever stands behind it, so it is strictly between the sky's own `0` and opaque
        //    — for both columns, whatever cut the tree chose.
        for (name, got) in [("thin", thin_px), ("thick", thick_px)] {
            assert!(
                got[3] > 0.0 && got[3] < 1.0,
                "the {name} cloud's alpha is {}, which is not a veil between sky and opaque",
                got[3]
            );
        }
        // 2. **A wetter column is more cloud**, in the alpha.
        assert!(
            thick_px[3] > thin_px[3],
            "more water did not put more cloud on the pixel: {} against {}",
            thick_px[3],
            thin_px[3]
        );

        // 3. **And the cloud is lit by the same model as everything else.** The fragment's row is
        //    the one `cloud_field::cloud_inputs` states — achromatic `CLOUD_ALBEDO`, and no
        //    roughness, specular boundary, coat or emission because a medium has no surface to
        //    carry any of them — so `shade_cpu` with that row is the reference the pixels must
        //    equal. This is the claim the user made in one sentence (*"the clouds are lit the
        //    same way"*), and it is checked the same way `shading.wgsl`'s terms already are.
        //
        //    **Which wall the pixel lands on is geometry, and the test resolves it rather than
        //    assuming it.** A cell is a closed box, and the cloud state tests depth without
        //    writing it — the plan's own line, so a second cell behind the first still
        //    composites. On this target, which cannot blend, the *last* covering fragment is
        //    therefore what the pixel shows; the eye ray crosses a stack of cells and every one of
        //    them presents only its `NegZ` or its `PosZ` wall
        //    `PosZ`. Both shades are computed and the frame must equal one of them, with the
        //    wall it matched named in the output. Pinning *which* one would be a claim about the
        //    depth state rather than about the shading, and the shading is what this is for.
        use crate::shading::shade_cpu;
        use voxel_noise::Vec3;
        let v = Vec3::new(0.0, 0.0, -1.0);
        let shade_of = |n: Vec3| {
            shade_cpu(&crate::cloud_field::cloud_inputs(), n, v, 0.0, &rig)
                .expect("the cloud's own row shades")
                .0
        };
        let walls = [
            ("the wall the ray entered by (NegZ)", shade_of(Vec3::new(0.0, 0.0, -1.0))),
            ("the wall the ray left by (PosZ)", shade_of(Vec3::new(0.0, 0.0, 1.0))),
        ];
        let worst = |got: [f32; 4], want: voxel_appearance::LinearRgb| {
            (0..3)
                .map(|c| (got[c] as f64 - [want.r, want.g, want.b][c]).abs())
                .fold(0.0_f64, f64::max)
        };
        let resolved = walls
            .iter()
            .find(|(_, want)| worst(thin_px, *want) < 2e-3)
            .unwrap_or_else(|| {
                panic!(
                    "the cloud's pixels ({:.4}, {:.4}, {:.4}) are neither wall's own shade \
                     ({:.4}, {:.4}, {:.4}) nor ({:.4}, {:.4}, {:.4})",
                    thin_px[0], thin_px[1], thin_px[2],
                    walls[0].1.r, walls[0].1.g, walls[0].1.b,
                    walls[1].1.r, walls[1].1.g, walls[1].1.b,
                )
            });
        let (which, expected) = (resolved.0, resolved.1);
        println!(
            "cloud pixel ({:.4}, {:.4}, {:.4}) is {which}, whose own shade is ({:.4}, {:.4}, \
             {:.4}); worst disagreement {:.2e}",
            thin_px[0], thin_px[1], thin_px[2], expected.r, expected.g, expected.b,
            worst(thin_px, expected)
        );
        //    And the veil did not change the colour: the alpha is the only channel the
        //    transmittance moves, which is what makes the picture and the shadow one number.
        //    Checked on the **thick** pixel too, whose veil is three times the thin one's, so a
        //    shader that folded the transmittance into the shade would fail here and pass above.
        for got in [thin_px, thick_px] {
            let d = worst(got, expected);
            assert!(
                d < 2e-3,
                "the veil changed the cloud's own colour by {d}: {got:?}"
            );
        }
    }

    /// **The step between two derivations, on real pixels** (C-179's fade).
    ///
    /// The cloud is derived on the atmosphere's cadence and drawn every frame, so before the fade the
    /// picture was **one step per cadence**: the tree a worker finished was swapped in whole, and every
    /// cell whose water crossed the cut appeared or vanished in that step. Three claims can be settled
    /// on a frame, and a swap would fail the second and third:
    ///
    /// 1. **The pair is one value, and the fade is not in it.** The two trees are appended with the
    ///    side in the sign of each veil and **no weight anywhere in the scene**, so the bytes a frame
    ///    draws are identical for every point of the dissolve: the interpolation is `shading.wgsl`'s,
    ///    from `View::screen.w`, and a frame therefore spends one uniform write on it rather than
    ///    rewriting every rectangle of the sky.
    /// 2. **A derivation arrives over its own cadence and not in one step.** The newer tree's own veil
    ///    is on the frame in four **equal** shares, one per tick of the cadence, and no tick is handed
    ///    the whole of it.
    /// 3. **The uniform is what moves it.** The pixel follows `screen.w` and nothing else: the same
    ///    scene, drawn at each tick's weight, is a different picture per tick.
    ///
    /// **What this frame cannot show is the join itself, and that is named rather than papered over:**
    /// the measurement target is `Rgba32Float`, which cannot blend (`world.rs` says so where the cloud's
    /// own pipeline state is built), so the last fragment drawn to a pixel is the one that is read — the
    /// newer tree's, since it is appended last. At weight `0` that fragment is clear air, so the older
    /// tree's own picture is *in* this frame as records and composited on a display — the window's
    /// `ALPHA_BLENDING` on the sRGB surface `main.rs` prefers — and not on this target, which is the one
    /// that exists to measure the shader rather than the compositor.
    ///
    /// The fixture is one column derived twice — thinner water against thicker — which is the smallest
    /// pair of adjacent derivations a cadence can produce, and the same column the veil's own test
    /// measures so the two numbers are comparable.
    #[test]
    #[ignore = "needs a GPU adapter; run explicitly with --ignored"]
    fn the_cloud_arrives_over_its_cadence_from_one_uniform() {
        use crate::cloud_cubes::{fade_period_ticks, fade_weight, CloudCubes};
        use crate::cloud_field::{CloudField, CLOUD_COLUMNS, CLOUD_COLUMN_M};
        use voxel_appearance::npr::LightRig;

        const SIDE: u32 = 65;
        const CENTRE: u32 = 32;
        const AT_Z: f64 = 300.0;
        const I: u32 = 10;
        const BASE: f32 = -2.0;
        const TOP: f32 = 2.0;
        /// The thinner derivation's water, and the thicker one's: both clear the cut, so the two
        /// trees differ in how much cloud they draw rather than in whether they draw cloud.
        const THIN: f32 = 0.30;
        const THICK: f32 = 0.45;

        let field = |water: f32| -> CloudField {
            let n = CLOUD_COLUMNS as usize;
            let mut columns = vec![[0.0f32; 4]; n * n];
            columns[I as usize * n + I as usize] = [water, BASE, TOP, 1.0];
            CloudField {
                origin_m: [
                    -((I as f64) + 0.5) * CLOUD_COLUMN_M,
                    AT_Z - ((I as f64) + 0.5) * CLOUD_COLUMN_M,
                ],
                columns,
                slab_base_m: BASE as f64,
                slab_top_m: TOP as f64,
                cloudy: 1,
                filled_micros: 0,
            }
        };

        let rig = LightRig::standard();
        let table = vec![MaterialParams::default(); 2];
        let gpu_rig = GpuRig::from_rig(&rig).expect("the standard rig fits");
        let mut renderer = Renderer::new(SIDE, SIDE, &table, &gpu_rig).expect("a GPU");
        // **The world's own sky** rather than a fixture beside it: what these tests measure is how
        // a cloud's veil and a cloud's rectangles land on the sky the game draws.
        let spec = voxel_shapers::WorldSpec::new(2468, voxel_shapers::Config::default());
        renderer.world().write_sky_model(&sky_model_of(&spec, 9_000));

        let camera = Camera::standard([0.0, 0.0, 0.0], 1.0);
        let eye = [0.0, 0.0, 0.0];
        let bare = Scene::build_with_detail(
            Vec::new().iter(),
            eye,
            &voxel_appearance::DetailLattice::new(0),
        );
        // The centre pixel's alpha: the cloud's own share of the pixel, the number the eye reads.
        let alpha = |renderer: &mut Renderer, scene: &Scene| -> f32 {
            renderer
                .render(scene, &camera)
                .expect("a frame")
                .pixel(CENTRE, CENTRE)
                .expect("the centre is in range")[3]
        };

        let older = CloudCubes::build(&field(THIN), eye, 1.7e-3);
        let newer = CloudCubes::build(&field(THICK), eye, 1.7e-3);
        assert!(
            !older.records.is_empty() && !newer.records.is_empty(),
            "the fixture drew no cloud: {} cells / {} rects against {} cells / {} rects",
            older.cells,
            older.records.len(),
            newer.cells,
            newer.records.len()
        );

        // The ends, drawn as the trees themselves: one tree appended, at full weight.
        renderer.set_cloud_fade(1.0);
        let older_alone = alpha(&mut renderer, &bare.clone().with_clouds(&older));
        let newer_alone = alpha(&mut renderer, &bare.clone().with_clouds(&newer));
        assert!(
            newer_alone > older_alone + 1e-3,
            "the fixture's two derivations draw the same cloud ({older_alone} against 
             {newer_alone}), so there is no step for the fade to smooth"
        );

        // **The pair, built once**: the side is in each veil's sign, and no weight is in it.
        let pair = bare.clone().with_cloud_fade(&older, &newer);
        assert_eq!(
            pair.cloud_records,
            older.records.len() + newer.records.len(),
            "the pair drew {} of the {} rectangles the two trees hold",
            pair.cloud_records,
            older.records.len() + newer.records.len()
        );
        assert_eq!(
            pair,
            bare.clone().with_cloud_fade(&older, &newer),
            "the pair's own bytes depend on something but the two trees"
        );

        let period = fade_period_ticks();
        let derived = 40_000u64;
        let mut alphas = Vec::new();
        for step in 0..=period {
            let weight = fade_weight(derived, derived + step);
            renderer.set_cloud_fade(weight as f32);
            // The uniform carried it, on a scene that did not change (asserted above).
            assert_eq!(renderer.world().cloud_fade(), weight as f32);
            alphas.push((weight, alpha(&mut renderer, &pair)));
        }
        let shown: Vec<f32> = alphas.iter().map(|(_, a)| *a).collect();
        let steps: Vec<f32> = shown.windows(2).map(|w| w[1] - w[0]).collect();
        println!(
            "the fade's {period} ticks from one uniform: alpha at each {shown:?}; the newer tree's own \
             step from a swap was {newer_alone:.4}, and the older tree alone draws {older_alone:.4}"
        );

        // 2. The shares the **eased** weight asks for, one a tick, read off real pixels: the
        //    shader's interpolation is what replaces the single step a swap took at the cadence
        //    boundary, and `smoothstep` is the law it is held to (`shading.wgsl`'s `fs_main`).
        let eased = |t: f32| t * t * (3.0 - 2.0 * t);
        for (tick, moved) in steps.iter().enumerate() {
            let (lo, hi) = (
                eased(tick as f32 / period as f32),
                eased((tick + 1) as f32 / period as f32),
            );
            let want = (hi - lo) * newer_alone;
            assert!(
                (moved - want).abs() <= want * 0.02 + 1e-6,
                "tick {tick} moved {moved} where the eased weight asks for {want}: {shown:?}"
            );
        }
        // No tick is handed the whole change: the steepest eased step over this period is well
        // under half of it, and the ends are the trees themselves (asserted below).
        let biggest = steps.iter().cloned().fold(0.0f32, f32::max);
        assert!(
            biggest <= newer_alone * 0.5,
            "one tick moved {biggest} of the {newer_alone} a swap moves: the fade is not spread across \
             its own period"
        );
        // The ends: the newer tree's own number at weight `1`, and clear air at `0` — which on this
        // target is nothing, the older tree being composited on a display and not here.
        assert_eq!(alphas[0].1, 0.0, "weight zero drew cloud");
        assert_eq!(
            alphas[period as usize].1, newer_alone,
            "the fade's end is not the newer tree's own picture"
        );
    }

    /// **The join itself, on the target the window presents** (C-183).
    ///
    /// Every other cloud measurement in this file is taken on `Rgba32Float`, which **cannot blend** —
    /// so what those frames show is the last fragment drawn, which is not what a player sees. The fade
    /// is a compositing question and nothing else: the two trees are drawn by the *same* `ALPHA_BLENDING`
    /// pipeline, and the only thing that says whether the sky is smooth is what the blend leaves in the
    /// eight-bit `sRGB` target `main.rs` prefers.
    ///
    /// So this test builds the world pipeline on that format, draws the pair at each tick's weight, and
    /// reads the centre pixel back. Three claims, and the third is the one no other test can make:
    ///
    /// 1. **The ends are the two trees themselves.** At weight `1` the frame is the newer tree's own
    ///    picture and at `0` the older's — so the dissolve is anchored to two real derivations.
    /// 2. **No tick is handed the whole change**: the middle of the fade is strictly between the two.
    /// 3. **The pair composites**, which is what makes a cross-fade legitimate: the old code swapped
    ///    one tree for the other, and this is the frame where both are *in* the pixel at once.
    #[test]
    #[ignore = "needs a GPU adapter; run explicitly with --ignored"]
    fn the_two_trees_composite_on_the_target_the_window_presents() {
        use crate::cloud_cubes::{fade_period_ticks, fade_weight, CloudCubes};
        use crate::cloud_field::{CloudField, CLOUD_COLUMNS, CLOUD_COLUMN_M};
        use voxel_appearance::npr::LightRig;

        const SIDE: u32 = 65;
        const CENTRE: u32 = 32;
        const AT_Z: f64 = 300.0;
        const I: u32 = 10;
        /// **A deep, wet layer on purpose.** This is the one cloud measurement taken through an
        /// eight-bit target, whose alpha has 255 steps: a wisp's veil of two thousandths cannot be
        /// read there at all, so the fixture is a column thick enough to land well inside the range
        /// the target can resolve — and the two derivations are **half the water apart**, so the
        /// dissolve's own middle sits steps clear of both ends rather than a rounding error from one.
        const BASE: f32 = -40.0;
        const TOP: f32 = 40.0;
        const THIN: f32 = 0.5;
        const THICK: f32 = 1.0;
        let format = wgpu::TextureFormat::Rgba8UnormSrgb;

        let field = |water: f32| -> CloudField {
            let n = CLOUD_COLUMNS as usize;
            let mut columns = vec![[0.0f32; 4]; n * n];
            columns[I as usize * n + I as usize] = [water, BASE, TOP, 1.0];
            CloudField {
                origin_m: [
                    -((I as f64) + 0.5) * CLOUD_COLUMN_M,
                    AT_Z - ((I as f64) + 0.5) * CLOUD_COLUMN_M,
                ],
                columns,
                slab_base_m: BASE as f64,
                slab_top_m: TOP as f64,
                cloudy: 1,
                filled_micros: 0,
            }
        };

        let (device, queue, _) = offscreen_device().expect("a GPU");
        let table = vec![MaterialParams::default(); 2];
        let gpu_rig = GpuRig::from_rig(&LightRig::standard()).expect("the standard rig fits");
        let mut world = WorldPipeline::new(&device, &queue, format, &table, &gpu_rig)
            .expect("the pipeline for the surface the window presents");
        // A target that cannot blend cannot show a join: `world.rs` enables `ALPHA_BLENDING` on
        // `is_srgb()` alone, so a future format change must not take the compositing away quietly.
        assert!(format.is_srgb(), "the presented surface is not the one the veil blends on");
        let spec = voxel_shapers::WorldSpec::new(2468, voxel_shapers::Config::default());
        world.write_sky_model(&sky_model_of(&spec, 9_000));

        let colour = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("presented-colour"),
            size: wgpu::Extent3d { width: SIDE, height: SIDE, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let depth = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("presented-depth"),
            size: wgpu::Extent3d { width: SIDE, height: SIDE, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        // A readback row must be a multiple of 256 bytes, so a 65-pixel row of `Rgba8` is padded.
        let row_bytes = SIDE * 4;
        let padded = row_bytes.div_ceil(256) * 256;
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("presented-readback"),
            size: (padded * SIDE) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let colour_view = colour.create_view(&wgpu::TextureViewDescriptor::default());
        let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());

        let camera = Camera::standard([0.0, 0.0, 0.0], 1.0);
        let eye = [0.0, 0.0, 0.0];
        let bare = Scene::build_with_detail(
            Vec::new().iter(),
            eye,
            &voxel_appearance::DetailLattice::new(0),
        );
        let pixel_angle = crate::pattern::pixel_angle_rad(camera.fov_y_rad, SIDE);
        // The centre pixel's alpha, the number the eye reads, read off the presented target.
        let mut alpha = |weight: f32, scene: &Scene| -> f32 {
            world.set_cloud_fade(weight);
            let mut encoder = device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("present") });
            world.draw(&mut encoder, &colour_view, &depth_view, scene, &camera, pixel_angle);
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: &colour,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: &readback,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(padded),
                        rows_per_image: Some(SIDE),
                    },
                },
                wgpu::Extent3d { width: SIDE, height: SIDE, depth_or_array_layers: 1 },
            );
            queue.submit(Some(encoder.finish()));
            let slice = readback.slice(..);
            let (sender, receiver) = std::sync::mpsc::channel();
            slice.map_async(wgpu::MapMode::Read, move |result| {
                let _ = sender.send(result);
            });
            device
                .poll(wgpu::PollType::Wait { submission_index: None, timeout: None })
                .expect("the readback polls");
            receiver.recv().expect("the map callback runs").expect("the map succeeds");
            let data = slice.get_mapped_range().expect("the range maps");
            let at = (CENTRE * padded) as usize + (CENTRE * 4) as usize + 3;
            let alpha = data[at] as f32 / 255.0;
            drop(data);
            readback.unmap();
            alpha
        };

        let older = CloudCubes::build(&field(THIN), eye, 1.7e-3);
        let newer = CloudCubes::build(&field(THICK), eye, 1.7e-3);
        assert!(
            !older.records.is_empty() && !newer.records.is_empty(),
            "the fixture drew no cloud: {} and {} rectangles",
            older.records.len(),
            newer.records.len()
        );
        let pair = bare.clone().with_cloud_fade(&older, &newer);

        let older_alone = alpha(1.0, &bare.clone().with_clouds(&older));
        let newer_alone = alpha(1.0, &bare.clone().with_clouds(&newer));
        let period = fade_period_ticks();
        let derived = 40_000u64;
        let mut shown = Vec::new();
        for step in 0..=period {
            let weight = fade_weight(derived, derived + step) as f32;
            shown.push((weight, alpha(weight, &pair)));
        }
        println!(
            "composited on the presented target: alpha at each {shown:?}; the older tree alone draws \
             {older_alone:.3} and the newer {newer_alone:.3}"
        );
        assert!(
            newer_alone > older_alone + 0.05,
            "the fixture's two derivations draw the same cloud ({older_alone} against {newer_alone}), \
             so there is no step for the fade to smooth"
        );
        // The ends are the trees themselves, to an eight-bit step.
        let step = 1.0 / 255.0;
        let last = shown.len() - 1;
        assert_eq!(shown[0].0, 0.0);
        assert_eq!(shown[last].0, 1.0);
        assert!(
            (shown[0].1 - older_alone).abs() <= 2.0 * step,
            "the fade's start is {} against the older tree's own {older_alone}",
            shown[0].1
        );
        assert!(
            (shown[last].1 - newer_alone).abs() <= 2.0 * step,
            "the fade's end is {} against the newer tree's own {newer_alone}",
            shown[last].1
        );
        // 2. No tick is handed the whole change, and every tick has more cloud than the last.
        let alphas: Vec<f32> = shown.iter().map(|(_, a)| *a).collect();
        for pair in alphas.windows(2) {
            assert!(
                pair[1] >= pair[0] - step,
                "a tick went backwards: {} then {}",
                pair[0],
                pair[1]
            );
            assert!(
                pair[1] - pair[0] < 0.5 * (newer_alone - older_alone) + step,
                "a tick moved {} of the {} a swap moves: the fade is not spread",
                pair[1] - pair[0],
                newer_alone - older_alone
            );
        }
        // 3. **Both trees are in the pixel**: the middle is strictly between the two derivations.
        let middle = alphas[period as usize / 2];
        assert!(
            middle > older_alone + 2.0 * step && middle < newer_alone - 2.0 * step,
            "the half-weight frame ({middle}) is not a join of {older_alone} and {newer_alone}"
        );
    }

    /// **A stated absence is black, and an unstated sky is not a frame at all.**
    ///
    /// This is the pair the guard exists for. `write_no_sky_model` is a host saying there is no
    /// sky — the atmosphere refused — and the frame is the black this pass drew before a sky
    /// existed. Never calling either method is a host that has said nothing, and the test after
    /// this one is the refusal that is owed to it: the zeroed rows a fresh pipeline holds are the
    /// length `atmosphere.wgsl` declares, so the shader's own refusal cannot catch them and only
    /// the host's own record can.
    #[test]
    #[ignore = "needs a GPU adapter; run explicitly with --ignored"]
    fn a_frame_with_no_sky_stated_is_the_black_it_was_before() {
        let registry = voxel_shapers::materials::registry().expect("the registry");
        let materials = crate::shading::table(&registry, &shading_field()).expect("every admitted material derives");
        let rig = GpuRig::from_rig(&LightRig::standard()).expect("the standard rig fits");
        let mut renderer = Renderer::new(32, 32, &materials, &rig).expect("a GPU");
        renderer.world_mut().write_no_sky_model();
        let scene = Scene::build_with_detail(Vec::new().iter(), [0.0; 3], &voxel_appearance::DetailLattice::new(0));
        let camera = Camera::standard([0.0, 20.0, 0.0], 1.0);
        let frame = renderer.render(&scene, &camera).expect("a frame");

        assert!(
            frame.luminances().into_iter().all(|l| l == 0.0),
            "a stated absence is not black, so the frame is not the one this pass drew before a sky"
        );
        // And the model's own rows are the shader's, read back through the CPU's copy of the
        // layout: `world`'s test pins the shader's own declaration against this number.
        let spec = voxel_shapers::WorldSpec::new(1, voxel_shapers::Config::default());
        assert_eq!(
            sky_model_of(&spec, 9_000).bytes().len(),
            crate::skylight::SKY_MODEL_BYTES,
            "a model came out at a size the shader's own rows do not describe"
        );
    }

    /// **A frame drawn out of a sky nobody stated fails**, which is what makes the guard a guard:
    /// what it replaced was a black picture that read as a night.
    #[test]
    #[ignore = "needs a GPU adapter; run explicitly with --ignored"]
    #[should_panic(expected = "no sky has been stated")]
    fn a_frame_that_never_stated_a_sky_is_refused() {
        let registry = voxel_shapers::materials::registry().expect("the registry");
        let materials = crate::shading::table(&registry, &shading_field()).expect("every admitted material derives");
        let rig = GpuRig::from_rig(&LightRig::standard()).expect("the standard rig fits");
        let mut renderer = Renderer::new(32, 32, &materials, &rig).expect("a GPU");
        let scene = Scene::build_with_detail(Vec::new().iter(), [0.0; 3], &voxel_appearance::DetailLattice::new(0));
        let camera = Camera::standard([0.0, 20.0, 0.0], 1.0);
        let _ = renderer.render(&scene, &camera);
    }

    /// **Parked, and named.** This test is written for the quad format's per-face
    /// composition and needs two things that do not exist yet:
    /// `voxel_mesh::Quad::face_blend` and `Scene::build_with_quad_composition` — the
    /// C-172 build path. It is parked on a `cfg` that can never hold rather than on
    /// `#[ignore]`, because `#[ignore]` still compiles the body and the body names two
    /// functions that are not there. **Restore it when C-172 lands**; do not rewrite it to
    /// fit the current format, because it is the witness for that format's composition.
    #[cfg(any())]
    #[test]
    #[ignore = "needs a GPU adapter; run explicitly with --ignored"]
    fn a_frame_of_the_world_passes_the_bars_band_count() {
        // The FMA bar's criterion, on real pixels. This is the measurement that
        // S0 has been waiting for.
        use crate::shading;
        use voxel_appearance::npr::Terms;
        use voxel_mesh::{Chunk, MeshConfig, Mesher};
        use voxel_shapers::{Config, WorldSpec};
        use voxel_stream::WorldSource;

        let spec = voxel_shapers::WorldSpec::new(2468, voxel_shapers::Config::default());
        let source = WorldSource::new(spec);
        let registry = voxel_shapers::materials::registry().expect("the registry");
        let table = shading::table(&registry, &shading_field()).expect("every admitted material derives");
        let rig = LightRig::standard();
        let gpu_rig = GpuRig::from_rig(&rig).expect("the standard rig fits");

        let mut renderer = Renderer::new(256, 256, &table, &gpu_rig).expect("a GPU");
        let mut mesher = Mesher::new();
        let config = MeshConfig::from_registry(&registry).expect("a mesh config");

        // Mesh chunks exactly the way the pool does: build the chunk from the
        // generator, then mesh it against its own neighbourhood. Two vertical
        // chunks, because chunk y 0 and 1 span 0..64 m and the playable band is
        // -16..48 m — the first version meshed only y 0 and photographed a flat
        // plane, because the whole region it drew was underground.
        let mut meshes = Vec::new();
        for cy in 0..2 {
            for cx in 0..3 {
                for cz in 0..3 {
                    let pos = voxel_model::ChunkPos::new(cx, cy, cz);
                    let Ok(tree) = source.build_chunk(pos) else {
                        continue;
                    };
                    let chunk = Chunk::from_tree(pos, tree);
                    let neighborhood = source.neighborhood(pos);
                    if let Ok(mesh) = mesher.build(&chunk, &config, Some(&neighborhood)) {
                        meshes.push(mesh);
                    }
                }
            }
        }
        assert!(!meshes.is_empty(), "the world produced no chunks to draw");
        let quads: usize = meshes.iter().map(|m| m.quads.len()).sum();
        assert!(quads > 0, "the world produced no quads");

        // Per-face composition for the quad format: the face blend of each quad's
        // rectangle selection. Because a drawn quad is one material by construction,
        // this is the majority id in slot 0 and empty in slot 1 — the same row the
        // fragment used to read. The composition fold is the C-172 build path.
        let composition_for = |quad: &voxel_mesh::Quad| -> [voxel_model::MaterialId; 2] {
            let blend = quad.face_blend(quad.face);
            let first = blend.first().map(|(m, _)| *m).unwrap_or(voxel_model::MaterialId::EMPTY);
            [first, voxel_model::MaterialId::EMPTY]
        };

        // Where the world actually is, so the camera stands on it rather than
        // inside it. Building the scene with the eye at the origin makes every
        // vertex position a world metre, which is what makes this measurable at
        // all — there is no hard-coded "the surface is at 40 m" anywhere here.
        //
        // **The bounds are read through the vertex format**, because the frame
        // below is drawn through the quad format and that one builds no corner at
        // all: `scene.vertices` is empty by construction, so a bounds read taken
        // from it would be `+inf..-inf` and the camera would land nowhere.
        let placed = Scene::build_with_light(
            meshes.iter(),
            [0.0; 3],
            &voxel_appearance::DetailLattice::new(0),
            &|_, _, _| None,
            None,
            None,
        );
        let lo = |axis: usize| {
            placed.vertices.iter().map(|v| v.position[axis] as f64).fold(f64::INFINITY, f64::min)
        };
        let hi = |axis: usize| {
            placed.vertices.iter().map(|v| v.position[axis] as f64).fold(f64::NEG_INFINITY, f64::max)
        };
        println!(
            "meshed region: x {}..{}, y {}..{}, z {}..{} m, {} quads, {} chunks",
            lo(0), hi(0), lo(1), hi(1), lo(2), hi(2), placed.quads, meshes.len()
        );

        let eye = [(lo(0) + hi(0)) / 2.0, hi(1) + 6.0, lo(2) - 24.0];
        let mut camera = Camera::standard(eye, 1.0);
        // Down and along +Z, which is where the terrain was meshed.
        camera.pitch_rad = -0.35;

        let scene = Scene::build_with_quad_composition(
            meshes.iter(),
            eye,
            &voxel_appearance::DetailLattice::new(0),
            composition_for,
        );
        println!(
            "scene: {} chunks, {} quads, {} rectangles ({}), {} vertices, {} bytes of vertices",
            meshes.len(),
            scene.quads,
            scene.records.len(),
            scene.records.len() * crate::QuadRecord::SIZE as usize,
            scene.vertices.len(),
            scene.vertex_bytes().len()
        );
        let frame = renderer.render(&scene, &camera).expect("a frame");
        assert_eq!(frame.pixels.len(), 256 * 256);

        let terms = Terms {
            half_lambert: true,
            rim: true,
            specular_warp: true,
            ambient: true,
            subsurface: false,
        };
        let as_derived = frame.response(terms).expect("a measurable frame");
        println!("derived inputs  : {as_derived:?}");
        println!("derived distinct colours: {}", frame.distinct_colours());

        // The same frame with the **stated** surface `npr`'s own tests use, so
        // the only thing that differs between the two numbers below is whether
        // the model records an albedo. C-27 says six of twelve parameters are
        // absent for a rock and that no admitted material can pass the bar yet;
        // this is that sentence measured in pixels rather than in a lattice.
        let stated_inputs = stated();
        let mut stated_table = table.clone();
        for row in stated_table.iter_mut() {
            *row = MaterialParams::from_inputs(&stated_inputs, MESH_DOES_NOT_CARRY_OPEN);
        }
        let mut stated_renderer = Renderer::new(256, 256, &stated_table, &gpu_rig).expect("a GPU");
        let stated_frame = stated_renderer.render(&scene, &camera).expect("a frame");
        let as_stated = stated_frame.response(terms).expect("a measurable frame");
        println!("stated inputs   : {as_stated:?}");
        println!("stated distinct colours: {}", stated_frame.distinct_colours());

        // The bar, on real pixels — the measurement S0 has been waiting for.
        assert!(
            as_stated.meets_bar(),
            "with an admitted albedo the frame must meet the bar: {as_stated:?}"
        );
        // **This assertion used to be a tautology and now is a measurement.**
        //
        // It read `assert_eq!(samples, 256 * 256)` because `Frame::response` counted
        // *every* pixel, so it could only ever agree — it asserted that the code did
        // what it did. With the sky pass in, `response` counts the pixels that carry
        // geometry, so the same number is now the count of shaded surface and the sky
        // gets the rest. Asserting the tautology is what the alpha filter's teeth
        // showed up against: it failed with `left: 10177, right: 65536`, which is the
        // sky doing its job.
        //
        // So the claim is now the one worth making: **the samples are exactly the
        // geometry, and the geometry is a strict minority of the frame** — a terrain
        // that covered every pixel would mean the sky was drawing nothing.
        let (sky_pixels, geometry_pixels) = frame.split();
        assert_eq!(
            as_derived.samples as usize,
            geometry_pixels,
            "the response counted {} samples but the frame carries {geometry_pixels} \
             geometry pixels",
            as_derived.samples
        );
        assert!(
            geometry_pixels < 256 * 256,
            "every pixel is geometry ({geometry_pixels} of {}), so the sky pass drew nothing",
            256 * 256
        );
        assert_eq!(sky_pixels + geometry_pixels, 256 * 256, "a pixel was counted twice or not at all");

        // **The derived frame is nearly flat, and that is the finding.** With
        // every parameter derived as C-27 leaves it, `base_color` is absent, so
        // the diffuse lobe and the ambient term are *not drawn* rather than
        // drawn dark (`Terms`), and `specular_weight` is absent, so the
        // specular warp never fires either. What is left is the rim alone, and
        // the rim varies with the material's roughness and with ``n·v`` — so a
        // handful of flat tints is the correct picture of this world, not a
        // broken one. The first version of this test asserted more than 32
        // distinct colours and got 3.
        //
        // The comparison is what carries the meaning: same world, same camera,
        // same rig, and the only difference is whether an albedo exists.
        assert!(
            stated_frame.distinct_colours() > frame.distinct_colours(),
            "stating an albedo changed nothing: derived {} colours, stated {}",
            frame.distinct_colours(),
            stated_frame.distinct_colours()
        );
    }

    /// **The Sun is drawn, and it is the body `voxel-client::sun` says it is.**
    ///
    /// **The drawn Sun is the disc the world's own figures say, per pixel.**
    ///
    /// The body travels as a **record in the scene** (C-184) carrying its centre and half-extent in the
    /// world's engine metres, and `vs_body` plots that direction at `camera::FAR_M` with the half-extent
    /// scaled by the same ratio, which is what leaves the angle alone. The claim here is the one a plot
    /// distance cannot fake: a pixel is lit exactly when its ray lies within the Sun's own angular
    /// radius, **taken from the real figures** — the IAU radius over one astronomical unit — rather than
    /// from the record's own numbers, so a wrong half-extent or a wrong 1:8 ruling shows up here.
    ///
    /// A telescope's field rather than a player's: the real Sun is 0.53° across, which at a 70°
    /// field is under one pixel, and a one-pixel disc has no shape to check.
    #[test]
    #[ignore = "needs a GPU adapter; run explicitly with --ignored"]
    fn the_sun_is_drawn_as_the_disc_the_model_says() {
        use crate::skylight::{daylight_of, peak_sun_luminance, sun_direction};
        use crate::sun::{DrawnSun, AU_M};
        use voxel_appearance::sky::SkyInputs;

        const SIDE: u32 = 96;
        let spec = voxel_shapers::WorldSpec::new(2468, voxel_shapers::Config::default());
        let inputs = SkyInputs::default();
        // The shipped client's own start tick: the Sun is up, so this is not the midnight case
        // that would let every assertion below pass for the wrong reason.
        let day = daylight_of(&spec, 9_000, inputs, 0).expect("a sky at tick 9000");
        let peak = peak_sun_luminance(&spec, inputs);
        let eye = [0.0; 3];
        let sun = DrawnSun::of(&day.sky, peak, eye).expect("a body at tick 9000");

        // The camera looks straight at the body, through a telescope.
        let d = sun_direction(day.sky.sun_altitude_deg, day.sky.sun_azimuth_deg);
        let mut camera = Camera::standard(eye, 1.0);
        camera.fov_y_rad = 4f64.to_radians();
        camera.yaw_rad = d.x.atan2(d.z);
        camera.pitch_rad = d.y.clamp(-1.0, 1.0).asin();

        let registry = voxel_shapers::materials::registry().expect("the registry");
        let materials =
            crate::shading::table(&registry, &shading_field()).expect("every admitted material derives");
        let rig = GpuRig::from_rig(&LightRig::standard()).expect("the standard rig fits");
        let mut renderer = Renderer::new(SIDE, SIDE, &materials, &rig).expect("a GPU");
        renderer.world().write_sky_model(&sky_model_of(&spec, 9_000));

        // **Two frames of one scene, and the only difference between them is the body.** A threshold on
        // brightness would be a number somebody chose, and a claim about the exposure's own scale rather
        // than about the body; the model's own answer is that the body's pixels changed **by the radiance
        // the record carries** and that no other pixel changed at all.
        let scene = Scene::build_with_detail(
            Vec::<voxel_mesh::Mesh>::new().iter(),
            eye,
            &voxel_appearance::DetailLattice::new(0),
        );
        let without = renderer.render(&scene, &camera).expect("a frame");
        let record = sun.body_record(eye);
        let with = renderer
            .render(&scene.with_bodies(&[record]), &camera)
            .expect("a frame");
        let radiance = crate::frame::luminance([
            record.radiance[0],
            record.radiance[1],
            record.radiance[2],
            0.0,
        ]);
        assert!(
            radiance > 0.0,
            "the body's own record carries no radiance, so no frame can show it"
        );

        // The same pinhole the shader forms a ray through - `sky.wgsl`'s own three lines - so the
        // comparison means "the transcription is the model" rather than "two things agree roughly".
        //
        // **And the model's own edge, in the quantity `fs_body` tests.** The shader discards a pixel
        // whose ray meets the body's plane further than the half-extent from the centre; the record's
        // half-extent and distance are scaled by the plot in opposite directions, so that test is
        // `tan θ <= radius/distance` at *any* plot distance - and `radius/AU`, the real Sun's own
        // tangent, is what the frame has to hit.
        let tan_half = (camera.fov_y_rad / 2.0).tan();
        let t_edge = sun.body.radius_m / AU_M;
        // One pixel of the tangent plane: a pixel straddling the edge is not a claim either way, and
        // grading it would be grading rounding rather than the model.
        let pitch = 2.0 * tan_half / SIDE as f64;
        let (mut body_pixels, mut graded, mut on_edge, mut disagree) = (0usize, 0usize, 0usize, 0usize);
        let mut brightest = 0.0f64;
        let mut samples: Vec<String> = Vec::new();
        for y in 0..SIDE {
            for x in 0..SIDE {
                let ndc_x = 2.0 * (x as f64 + 0.5) / SIDE as f64 - 1.0;
                let ndc_y = 1.0 - 2.0 * (y as f64 + 0.5) / SIDE as f64;
                let across = ndc_x * tan_half * camera.aspect;
                let up_axis = ndc_y * tan_half;
                let drawn = crate::frame::luminance(with.pixel(x, y).expect("in range"));
                let air = crate::frame::luminance(without.pixel(x, y).expect("in range"));
                brightest = brightest.max(drawn - air);
                // The pixel's own radius in the tangent plane. The camera is aimed at the body, so the
                // frame's centre is the body's direction and this is the quantity the discard tests.
                let radius_t = (across * across + up_axis * up_axis).sqrt();
                if (radius_t - t_edge).abs() < pitch {
                    on_edge += 1;
                    continue;
                }
                graded += 1;
                let model = radius_t <= t_edge;
                if model {
                    body_pixels += 1;
                }
                // **What the frame must hold**: the air this ray carried **plus** the record's own
                // radiance where the model puts the body, and **the air untouched** where it does not —
                // a body stands in the air and does not blank it, which is what `fs_body` takes the sum
                // for. One law, both sides of the edge: the second frame differs from the first by the
                // body and by nothing else, so a pixel with no body on it that moved at all is the pass
                // changing something it was not asked to.
                if model {
                    let expected = air + radiance;
                    if (drawn - expected).abs() > radiance * 0.02 {
                        disagree += 1;
                        if samples.len() < 8 {
                            samples.push(format!(
                                "px ({x},{y}) tan {radius_t:.6e} inside the edge {t_edge:.6e}: the air \
                                 {air:.4e} plus the record's {radiance:.4e} is {expected:.4e} and the \
                                 frame drew {drawn:.4e}"
                            ));
                        }
                    }
                } else if drawn != air {
                    disagree += 1;
                    if samples.len() < 8 {
                        samples.push(format!(
                            "px ({x},{y}) tan {radius_t:.6e} outside the edge {t_edge:.6e}: the frame \
                             moved from {air:.4e} to {drawn:.4e} with no body on it"
                        ));
                    }
                }
            }
        }

        assert!(
            body_pixels > 40,
            "the model puts only {body_pixels} pixels on the body, which is too few to compare"
        );
        assert_eq!(
            disagree, 0,
            "{disagree} of {graded} graded pixels disagree with the model: the body's pixels are not \
             the air the ray carried plus the radiance the record carries\n  {}",
            samples.join("\n  ")
        );
        // **And the body is drawn at the radiance it carries**, which is the claim a coverage test
        // cannot make: the brightest change the frame made is the record's own number.
        assert!(
            (brightest - radiance).abs() <= radiance * 0.02,
            "the frame's brightest change is {brightest:.4e} where the record carries \
             {radiance:.4e}: the body is not drawn at its own radiance"
        );
        println!(
            "sun: {body_pixels} pixels inside the model's own {:.4}° radius, {graded} graded with \
             {disagree} disagreeing and {on_edge} on the edge, each changed by {radiance:.4e} \
             (brightest change {brightest:.4e})",
            t_edge.atan().to_degrees()
        );
    }

    /// **The Moon's phase is drawn, not handed over.**
    ///
    /// The claim this test exists for is the one the wiring could fake: the record carries the radiance
    /// of a **lit face** and the direction that lights it, and `fs_body` gives every pixel the normal of
    /// the **sphere** at that pixel, so a crescent is a terminator the shader found rather than a
    /// brightness the CPU multiplied in. Two checks, both against the model:
    ///
    /// * **per pixel**, the incidence the transcribed sphere law gives — a pixel well inside the lit
    ///   side is bright and a pixel well beyond the terminator is the sky alone, with a band either side
    ///   of the terminator left ungraded rather than given a tolerance;
    /// * **in aggregate**, the fraction of the drawn disc the model lights is the orbit's own
    ///   `moon_phase_fraction`, which is the same `(1 − cos E)/2` a sphere's lit area is.
    ///
    /// **And it is a night frame with no Sun in it.** The Sun's body refuses below the horizon, which is
    /// exactly when the Moon is up: a moon lit by a body the frame does not contain would draw nothing
    /// at all here, and the assertion that there is no sun is what makes that the test.
    #[test]
    #[ignore = "needs a GPU adapter; run explicitly with --ignored"]
    fn the_moons_phase_is_drawn_from_its_own_geometry() {
        use crate::skylight::{daylight_of, peak_sun_luminance};
        use crate::sun::{DrawnMoon, DrawnSun};
        use voxel_appearance::sky::SkyInputs;

        const SIDE: u32 = 96;

        let spec = voxel_shapers::WorldSpec::new(2468, voxel_shapers::Config::default());
        let inputs = SkyInputs::default();
        // **A night the model itself calls lit**: the Sun down, the model's own phase at least
        // half, and **the Moon above the horizon** — because a body on an orbit sets, and a lit
        // phase behind the horizon is not a face to draw (`DrawnMoon::of` refuses it). Scanning the
        // model's own year rather than picking a tick keeps this the model's night and not one
        // chosen to make the picture come out.
        //
        // The search runs by **day**, because the phase moves over a synodic month and not inside a
        // day: a scan of one day either has a lit moon or has none, and the first version of this test
        // scanned a single day and found no night in it.
        let ticks_per_day = voxel_shapers::climate::TICKS_PER_DAY;
        let mut night = None;
        'year: for day in 0..voxel_shapers::climate::DAYS_PER_YEAR {
            let start = day * ticks_per_day;
            if voxel_shapers::sky::moon_phase_fraction(start, 0) < 0.5 {
                continue;
            }
            for t in start..start + ticks_per_day {
                let sun = voxel_shapers::sky::solar_position(&spec, t).altitude_deg;
                let moon = voxel_shapers::sky::moon_position(&spec, t, 0);
                if (-22.0..-18.0).contains(&sun) && moon.altitude_deg > 0.0 {
                    night = Some(t);
                    break 'year;
                }
            }
        }
        let tick = night.expect("a night with a lit moon in the model's own year");
        let day = daylight_of(&spec, tick, inputs, 0).expect("a sky");
        let peak = peak_sun_luminance(&spec, inputs);
        let eye = [0.0; 3];
        let air = crate::sun::air_at(&spec, 0.0, 0.0, tick);
        let orbit = voxel_shapers::sky::moon_position(&spec, tick, 0);
        let moon = DrawnMoon::of(
            &orbit,
            &air,
            peak,
            eye,
            day.sky.sun_altitude_deg,
            day.sky.sun_azimuth_deg,
        )
        .expect("a moon to draw");

        // **The record the frame carries**, and the geometry `vs_body` derives from it: the direction to
        // the body, the plane the frame plots it on, and the half-extent scaled to that plane.
        //
        // The camera looks straight at the body, through a telescope: the Moon's real half a degree
        // would be a pixel and a half at a player's own field.
        let record = moon.body_record(eye);
        let offset = [
            record.centre_m[0] - eye[0],
            record.centre_m[1] - eye[1],
            record.centre_m[2] - eye[2],
        ];
        let distance_m = (offset[0] * offset[0] + offset[1] * offset[1] + offset[2] * offset[2]).sqrt();
        let to_body = [
            offset[0] / distance_m,
            offset[1] / distance_m,
            offset[2] / distance_m,
        ];
        let plotted_m = crate::camera::BODY_PLOT_M;
        let half_m = record.half_extent_m * plotted_m / distance_m;
        let mut camera = Camera::standard(eye, 1.0);
        // **A telescope wide enough to resolve the crescent.** At a player's own field the Moon's half a
        // degree is a pixel and a half; at 4° its disc is six pixels across, and counting lit pixels over
        // disc pixels then measures the grid rather than the phase — the first version of this test read
        // 0.693 where the area fraction is 0.647. The disc fills 44 pixels of this 1.2° field, which is
        // what makes the aggregate below a claim about the model.
        camera.fov_y_rad = 1.2f64.to_radians();
        camera.yaw_rad = to_body[0].atan2(to_body[2]);
        camera.pitch_rad = to_body[1].clamp(-1.0, 1.0).asin();

        let registry = voxel_shapers::materials::registry().expect("the registry");
        let materials = crate::shading::table(&registry, &shading_field())
            .expect("every admitted material derives");
        let rig = GpuRig::from_rig(&LightRig::standard()).expect("the standard rig fits");
        let mut renderer = Renderer::new(SIDE, SIDE, &materials, &rig).expect("a GPU");
        renderer.world().write_sky_model(&sky_model_of(&spec, tick));
        // **No Sun in this frame, and that is asserted rather than assumed**: below the horizon the rig
        // carries no sun light, so the drawn body refuses — and the moon is lit anyway. It is the
        // *record's* refusal too, so the frame's own body list holds the Moon and nothing else.
        assert!(
            DrawnSun::of(&day.sky, peak, eye).is_none(),
            "the Sun was drawn, so this frame is not the night case the claim is about"
        );

        // **Two frames of one night, and the only difference between them is the Moon.** What the model
        // predicts per pixel is the air it stands in plus the record's radiance times the incidence the
        // sphere law gives there — so the phase is graded with no threshold and no ungraded band.
        let scene = Scene::build_with_detail(
            Vec::<voxel_mesh::Mesh>::new().iter(),
            eye,
            &voxel_appearance::DetailLattice::new(0),
        );
        let without = renderer.render(&scene, &camera).expect("a frame");
        let with = renderer
            .render(&scene.with_bodies(&[record]), &camera)
            .expect("a frame");
        let radiance = crate::frame::luminance([
            record.radiance[0],
            record.radiance[1],
            record.radiance[2],
            0.0,
        ]);
        assert!(
            radiance > 0.0,
            "the Moon's record carries no radiance, so no frame can show a crescent"
        );

        // **The shader's own geometry, transcribed**: `body_basis` built the way `vs_body` builds it,
        // the pixel's own point in the body's plane, and the normal `fs_body` gives that point — the
        // sphere law, `√(1 − r²)` along the view axis. A pixel inside the band either side of the
        // terminator is not graded: it is a partly lit point and the model has no single answer for it.
        //
        // `plotted_m` and `half_m` are the two numbers the vertex stage is built from, so the offset in
        // `[-1, 1]` here is the quantity the fragment interpolates and not a second parameterisation of
        // the same disc.
        let light = record.light_direction;
        let light_magnitude = (light[0] * light[0] + light[1] * light[1] + light[2] * light[2]).sqrt();
        assert!(
            light_magnitude > 0.0,
            "the record carries no light direction, so this moon has no phase to check"
        );
        let light = [
            light[0] / light_magnitude,
            light[1] / light_magnitude,
            light[2] / light_magnitude,
        ];
        let cross = |a: [f64; 3], b: [f64; 3]| {
            [
                a[1] * b[2] - a[2] * b[1],
                a[2] * b[0] - a[0] * b[2],
                a[0] * b[1] - a[1] * b[0],
            ]
        };
        let unit = |v: [f64; 3]| {
            let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            [v[0] / n, v[1] / n, v[2] / n]
        };
        let helper = if to_body[1].abs() > 0.9 {
            [1.0, 0.0, 0.0]
        } else {
            [0.0, 1.0, 0.0]
        };
        let basis_right = unit(cross(helper, to_body));
        let basis_up = cross(to_body, basis_right);
        let (right, up, forward) = camera.basis();
        let tan_half = (camera.fov_y_rad / 2.0).tan();
        // The disc's own edge in the tangent plane — `half_m / plotted_m`, which is `radius/distance`
        // whatever the plot is — with one pixel's pitch either side of it left ungraded, the same band
        // the Sun's own test leaves.
        let t_edge = half_m / plotted_m;
        let pitch = 2.0 * tan_half / SIDE as f64;
        let mut lit_model = 0usize;
        let mut disc_model = 0usize;
        let mut on_edge = 0usize;
        let mut disagree = 0usize;
        let mut brightest = 0.0f64;
        let mut samples: Vec<String> = Vec::new();
        for y in 0..SIDE {
            for x in 0..SIDE {
                let ndc_x = 2.0 * (x as f64 + 0.5) / SIDE as f64 - 1.0;
                let ndc_y = 1.0 - 2.0 * (y as f64 + 0.5) / SIDE as f64;
                let across = ndc_x * tan_half * camera.aspect;
                let up_axis = ndc_y * tan_half;
                let radius_t = (across * across + up_axis * up_axis).sqrt();
                if (radius_t - t_edge).abs() < pitch {
                    on_edge += 1;
                    continue;
                }
                let dir = unit([
                    right[0] * across + up[0] * up_axis + forward[0],
                    right[1] * across + up[1] * up_axis + forward[1],
                    right[2] * across + up[2] * up_axis + forward[2],
                ]);
                let drawn = crate::frame::luminance(with.pixel(x, y).expect("in range"));
                let air = crate::frame::luminance(without.pixel(x, y).expect("in range"));
                brightest = brightest.max(drawn - air);
                // The ray's own point in the body's plane, in units of the half-extent the frame drew.
                let toward = dir[0] * to_body[0] + dir[1] * to_body[1] + dir[2] * to_body[2];
                if toward <= 0.0 {
                    continue;
                }
                let t = plotted_m / toward;
                let q = [
                    dir[0] * t - to_body[0] * plotted_m,
                    dir[1] * t - to_body[1] * plotted_m,
                    dir[2] * t - to_body[2] * plotted_m,
                ];
                let r2 = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2]) / (half_m * half_m);
                // **The incidence `fs_body` computes**, or none off the disc: the sphere's own outward
                // normal at the pixel's own point — the sub-eye point faces the eye, which is why the
                // normal is the negative of the offset plus the remaining axis — against the direction
                // the record says lights it.
                // **The incidence, and what the shader actually multiplies the radiance by.** The
                // incidence is the *substrate's* share of the face — the physical phase, which the
                // aggregate claim below counts — and `shaded` is `fs_body`'s own factor: that share
                // **plus the rim**, which is `fs_main`'s two powers at its own `5.0`. The rim is not a
                // decoration and this is the test that says so: it is `F(theta)`'s boundary share, so a
                // light **behind** the body still lights its limb, which is what makes a new moon a
                // disc rather than a hole in the sky.
                let (incidence, shaded) = if r2 > 1.0 {
                    (0.0, 0.0)
                } else {
                    let q0 = (q[0] * basis_right[0] + q[1] * basis_right[1] + q[2] * basis_right[2])
                        / half_m;
                    let q1 = (q[0] * basis_up[0] + q[1] * basis_up[1] + q[2] * basis_up[2]) / half_m;
                    let along = (1.0 - r2).sqrt();
                    let normal = [
                        -(basis_right[0] * q0 + basis_up[0] * q1 + to_body[0] * along),
                        -(basis_right[1] * q0 + basis_up[1] * q1 + to_body[1] * along),
                        -(basis_right[2] * q0 + basis_up[2] * q1 + to_body[2] * along),
                    ];
                    let ndotl = (normal[0] * light[0] + normal[1] * light[1] + normal[2] * light[2])
                        .clamp(-1.0, 1.0);
                    // The sub-eye point faces the eye, so the direction from the body toward the eye is
                    // `-to_body` — the same pair the shader's `eye` and `ndotv` are.
                    let ndotv = (-(normal[0] * to_body[0]
                        + normal[1] * to_body[1]
                        + normal[2] * to_body[2]))
                        .clamp(0.0, 1.0);
                    let substrate = ndotl.max(0.0);
                    let beam_share = (1.0 - ndotl.clamp(0.0, 1.0)).powi(5);
                    let silhouette = (1.0 - ndotv).powi(5);
                    (substrate, substrate + beam_share * silhouette)
                };
                if r2 <= 1.0 {
                    disc_model += 1;
                    if incidence > 0.0 {
                        lit_model += 1;
                    }
                }
                // **The model's own pixel value**: the air the ray carried plus the record's radiance
                // through the incidence there. Off the disc the incidence is `0.0` and the sum is that
                // same air, so this is one law on both sides of the rim and not a special case. Three
                // percent is the transcription's own slack — the fragment interpolates `offset` in `f32`
                // from geometry the pass built, and this writes it in `f64` from the record — and it is a
                // *relative* band, so a crescent's own dim pixels are graded as tightly as a full moon's.
                let expected = air + radiance * shaded;
                if (drawn - expected).abs() > radiance * 0.03 {
                    disagree += 1;
                    if samples.len() < 8 {
                        samples.push(format!(
                            "px ({x},{y}) incidence {incidence:.4}, the model says {expected:.4e} \
                             and the frame drew {drawn:.4e}"
                        ));
                    }
                }
            }
        }

        assert!(
            lit_model > 20,
            "the model puts only {lit_model} pixels on the lit part of the disc, which is too few \
             to compare"
        );
        assert!(
            disc_model > 60,
            "the model puts only {disc_model} pixels on the disc, so this frame has no phase to check"
        );
        assert_eq!(
            disagree, 0,
            "{disagree} graded pixels disagree with the model: the drawn crescent is not the air plus \
             the record's own radiance through the sphere law and the material's own rim\n  {}",
            samples.join("\n  ")
        );
        // **And in aggregate: the lit fraction of the drawn disc is the orbit's own.**
        // `moon_phase_fraction` is `(1 − cos E)/2`, which is exactly the fraction of a sphere's visible
        // disc that a body at the Sun's elongation lights — so this compares the crescent's area against
        // the orbit rather than against a second rendering of it. Two percent is the quantisation: the
        // disc is a dozen pixels across in this field, so a pixel of its rim is about one percent of its
        // own area, and the projection is not quite orthographic either.
        let phase = voxel_shapers::sky::moon_phase_fraction(tick, 0);
        let modelled = lit_model as f64 / disc_model as f64;
        // **The geometry both fractions come from**, printed because a phase disagreement is a
        // disagreement about an angle and nothing here can show *which* angle without it: the orbit's
        // elongation, the separation of the two drawn directions, and the two positions.
        let drawn_separation_deg = (to_body[0] * light[0] + to_body[1] * light[1] + to_body[2] * light[2])
            .clamp(-1.0, 1.0)
            .acos()
            .to_degrees();
        println!(
            "moon geometry: orbit {:.5} deg / lit {:.5} (alt {:.4}, az {:.4}); drawn separation \
             {drawn_separation_deg:.5} deg -> lit {:.5}; sun alt {:.4} az {:.4}; modelled {modelled:.5}",
            orbit.elongation_deg,
            (1.0 - orbit.elongation_deg.to_radians().cos()) * 0.5,
            orbit.altitude_deg,
            orbit.azimuth_deg,
            (1.0 - drawn_separation_deg.to_radians().cos()) * 0.5,
            day.sky.sun_altitude_deg,
            day.sky.sun_azimuth_deg
        );
        assert!(
            (modelled - phase).abs() < 0.02,
            "the model's own lit fraction {modelled} is not the orbit's {phase}"
        );
        println!(
            "moon: {lit_model} of {disc_model} pixels model-lit ({modelled:.4}, the orbit says \
             {phase:.4}), brightest change {brightest:.4e} against a face radiance of {radiance:.4e}, \
             {on_edge} on the edge and ungraded, no sun in the frame, 0 disagreements"
        );
    }
}
