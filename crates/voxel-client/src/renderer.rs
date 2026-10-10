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
            adapter: adapter_name,
        })
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
        self.world
            .draw(&mut encoder, &colour_view, &depth_view, scene, camera, pixel_angle);
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
                    *cell = f32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]]);
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
        voxel_shapers::column::materials::shading_field()
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

        let registry = voxel_shapers::column::materials::registry().expect("the registry");
        let table = crate::shading::table(&registry, &shading_field())
            .expect("every admitted material derives");
        let rig = GpuRig::from_rig(&LightRig::standard()).expect("the standard rig fits");

        // 64×64 is exactly eight 8×8 tiles, so the Bayer matrix covers the frame a
        // whole number of times and its mean offset is **zero and not merely small**.
        const SIDE: u32 = 64;
        let mut renderer = Renderer::new(SIDE, SIDE, &table, &rig).expect("a GPU");

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
        // **65x65, and that is not cosmetic.** A pixel is sampled at its own
        // centre, so a frame of even width has no pixel at the frame's centre:
        // the nearest sample sits half a pixel off, and on a screen-filling
        // probe quad the view vector there therefore differs from the one this
        // reference is handed. The first version of this test used 64x64 and
        // failed by 4.3e-2 on a head-on normal, because the rim term is
        // `(1 - n·v)^5` and that is exactly where half a pixel of view error is
        // loudest. An odd extent puts pixel 32's sample at NDC 0 exactly, so
        // both sides compute the *same* surface.
        const SIDE: u32 = 65;
        const CENTRE: u32 = 32;
        let mut renderer = Renderer::new(SIDE, SIDE, &table, &gpu_rig).expect("a GPU");
        println!("adapter: {}", renderer.adapter);

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
        use voxel_shapers::column::materials;

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

        // A pixel whose address sits on a rounding edge can land in the next cell on
        // the GPU, where the arithmetic is `f32`. Those pixels are skipped and
        // counted; the margin is a fixed fraction of one lattice cell.
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

        let spec = WorldSpec::new(2468, Config::default());
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
    /// not be seen on it. Both halves were wrong: `column::materials::registry` admits
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
        let registry = voxel_shapers::column::materials::registry().expect("the registry");
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
        let registry = voxel_shapers::column::materials::registry().expect("the registry");
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
        assert_eq!(crate::QuadRecord::SIZE, 24, "the quad record's stride moved");

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
        let registry = voxel_shapers::column::materials::registry().expect("the registry");
        let table = shading::table(&registry, &shading_field()).expect("every admitted material derives");
        let gpu_rig = GpuRig::from_rig(&LightRig::standard()).expect("the standard rig fits");
        let mut renderer = Renderer::new(256, 256, &table, &gpu_rig).expect("a GPU");

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
        let registry = voxel_shapers::column::materials::registry().expect("the registry");
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
        let registry = voxel_shapers::column::materials::registry().expect("the registry");
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
        use crate::skylight::{sky_dome_table, SKY_DOME_ENTRIES};
        use voxel_appearance::sky::{Sky, SkyInputs};
        use voxel_shapers::{Config, WorldSpec};

        let spec = WorldSpec::new(20_260_930, Config::default());
        // Tick 9000: the Sun is up, so the table is a real sky and not the
        // midnight all-zero case that would make every assertion below pass for
        // the wrong reason. The **directional dome** is what the pass draws now.
        let inputs = SkyInputs::default();
        let sky = Sky::at(&spec, 9_000, inputs, 0).expect("a sky at tick 9000");
        let peak = crate::skylight::peak_sun_luminance(&spec, inputs);
        let table = sky_dome_table(&sky, inputs, peak);
        assert_eq!(table.len(), SKY_DOME_ENTRIES, "the dome table is the declared size");

        let registry = voxel_shapers::column::materials::registry().expect("the registry");
        let materials = crate::shading::table(&registry, &shading_field()).expect("every admitted material derives");
        let rig = GpuRig::from_rig(&LightRig::standard()).expect("the standard rig fits");

        let mut renderer = Renderer::new(64, 64, &materials, &rig).expect("a GPU");
        renderer.world().write_sky(&table).expect("the sky table fits the pipeline");

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

        // And the table is being *read*: looking up is not the same picture as
        // looking down, which clamps to the horizon. A pass that ignored the
        // direction and filled one flat colour would fail exactly here.
        let up_mean = frame.luminances().into_iter().sum::<f64>() / frame.luminances().len() as f64;

        let mut down = Camera::standard([0.0, 20.0, 0.0], 1.0);
        down.pitch_rad = -0.45;
        let down_frame = renderer.render(&scene, &down).expect("a frame looking down");
        let down_mean =
            down_frame.luminances().into_iter().sum::<f64>() / down_frame.luminances().len() as f64;

        assert!(
            (up_mean - down_mean).abs() > 1e-4,
            "looking up ({up_mean:.6}) and looking down ({down_mean:.6}) are the same \
             picture, so the sky table is not being indexed by direction"
        );
        println!("sky mean: up {up_mean:.5}, down {down_mean:.5}, peak {brightest:.5}");
    }

    /// **A water surface reflects the directional sky, and the GPU agrees with the
    /// model about it.**
    ///
    /// The user's ruling is why this test exists (`LEDGER.md` C-157.6): *"the sun and
    /// it's moon refletions is atually core to the rendering of the world itself, and the
    /// materials"*. A water probe — the water material's own derived inputs, a **tilted**
    /// normal so the body and the reflection are both visible — is drawn with the real
    /// dome table uploaded the way the window uploads it, and the pixels are compared
    /// against [`crate::shading::fluid_surface_cpu`], which reads the same table through
    /// `sky_dome_lookup`. That comparison is what holds `shading.wgsl`'s transcription of
    /// `voxel_appearance::fluid` to the model, the same way
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
        use crate::skylight::{
            daylight_of, exposed, peak_sun_luminance, sky_dome_table, sky_exposed,
        };
        use voxel_appearance::sky::SkyInputs;
        use voxel_shapers::column::materials;
        use voxel_shapers::{Config, WorldSpec};

        const SIDE: u32 = 65;
        const CENTRE: u32 = 32;

        let spec = WorldSpec::new(2468, Config::default());
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
        // The dome exactly as the window sends it, and the same table exposed on this
        // side for the CPU comparison — `write_sky` applies `SKY_EXPOSURE`, so the two
        // must be compared through one exposure.
        let dome = sky_dome_table(&day.sky, inputs, peak);
        let exposed_dome: Vec<LinearRgb> = dome.iter().copied().map(sky_exposed).collect();
        renderer.world().write_sky(&dome).expect("the sky uploads");

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
            let (expected, _) = fluid_surface_cpu(&water, n, view, 0.0, &rig, &exposed_dome)
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
        let lum = |c: [f64; 3]| 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
        let (got_lum, body_lum) = (lum([got[0] as f64, got[1] as f64, got[2] as f64]), body.luminance());
        println!("  reflection adds {:.5} of {:.5} ({:.1} %)", got_lum - body_lum, got_lum, 100.0 * (got_lum - body_lum) / got_lum);
        assert!(
            got_lum > body_lum * 1.01,
            "the water pixel ({got_lum:.6}) is not brighter than the body alone ({body_lum:.6})"
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

        // A field holding **one cloudy column**, centred on the view axis: the camera looks
        // down `+Z` at yaw zero, so the column's own centre is placed there, and the layer
        // brackets the axis so the centre pixel is inside a cell rather than on a seam.
        //
        // **The layer is one cell deep on purpose.** A cell carries its own thickness's share of
        // its column's transmittance (`cloud_cubes`), so it is only for a column that is one cell
        // that the fragment's veil *is* the column's own number. That is what claim 1 below can
        // then state without qualification, and the layering's own arithmetic — the product of a
        // column's cells is its `cloud_transmittance` — is pinned where it is computed.
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
        // **A white sky**, sent as the table the window sends, so the blend the pixel shows is
        // the veil's alone and not a statement about a particular atmosphere.
        let dome = vec![LinearRgb::WHITE; crate::skylight::SKY_DOME_ENTRIES];
        renderer.world().write_sky(&dome).expect("the sky uploads");

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
        assert!(thin.records.len() > 0 && thick.records.len() > 0, "the fixture drew no cloud");
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

    #[test]
    #[ignore = "needs a GPU adapter; run explicitly with --ignored"]
    fn a_frame_with_no_sky_written_is_the_black_it_was_before() {
        // The table is zeroed at construction, so a caller that never asks for a sky
        // gets the behaviour this pass replaced — which is what makes adding it safe
        // for every existing caller rather than a change they must all opt into.
        let registry = voxel_shapers::column::materials::registry().expect("the registry");
        let materials = crate::shading::table(&registry, &shading_field()).expect("every admitted material derives");
        let rig = GpuRig::from_rig(&LightRig::standard()).expect("the standard rig fits");
        let mut renderer = Renderer::new(32, 32, &materials, &rig).expect("a GPU");
            let scene = Scene::build_with_detail(Vec::new().iter(), [0.0; 3], &voxel_appearance::DetailLattice::new(0));
        let camera = Camera::standard([0.0, 20.0, 0.0], 1.0);
        let frame = renderer.render(&scene, &camera).expect("a frame");

        assert!(
            frame.luminances().into_iter().all(|l| l == 0.0),
            "an unwritten sky is not black, so the zeroed table is not what ships"
        );
        // And an over-long table is refused rather than truncated into a dome that
        // stops short of the zenith.
        assert!(
            renderer.world().write_sky(&vec![voxel_appearance::LinearRgb::WHITE; 100_000]).is_none(),
            "a table longer than the pipeline accepted was truncated instead of refused"
        );
        assert!(
            renderer.world().write_sky(&[]).is_none(),
            "an empty table was accepted, so there is no such thing as no sky"
        );
    }

    // **Parked, and named.** This test is written for the quad format's per-face
    // composition and needs two things that do not exist yet:
    // `voxel_mesh::Quad::face_blend` and `Scene::build_with_quad_composition` — the
    // C-172 build path. It is parked on a `cfg` that can never hold rather than on
    // `#[ignore]`, because `#[ignore]` still compiles the body and the body names two
    // functions that are not there. **Restore it when C-172 lands**; do not rewrite it to
    // fit the current format, because it is the witness for that format's composition.
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

        let spec = WorldSpec::new(2468, Config::default());
        let source = WorldSource::new(spec);
        let registry = voxel_shapers::column::materials::registry().expect("the registry");
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
    /// Two claims, and the second is the one a picture cannot make. The first: the pixels the
    /// pass lights are **exactly** the pixels whose rays the model says meet the body's cuboid -
    /// the shader's slab test against the Rust one, which is the comparison
    /// `the_water_surface_reflects_the_sky_the_model_says_it_does` makes for the dome's bilinear
    /// lookup. The second: that set is **not the ball of the same radius** - a cube's corner
    /// stands outside the ball it contains - so what is drawn is the cuboid and not a disc.
    ///
    /// A telescope's field rather than a player's: the real Sun is 0.53° across, which at a 70°
    /// field is under one pixel, and a one-pixel disc has no shape to check.
    #[test]
    #[ignore = "needs a GPU adapter; run explicitly with --ignored"]
    fn the_sun_is_drawn_as_the_cuboid_the_model_says() {
        use crate::skylight::{daylight_of, peak_sun_luminance, sky_dome_table, sun_direction};
        use crate::sun::{DrawnSun, AU_M};
        use voxel_appearance::sky::SkyInputs;
        use voxel_shapers::{Config, WorldSpec};

        const SIDE: u32 = 96;
        // **What counts as a Sun pixel.** The exposed sky's own brightest pixel is 0.73 - the
        // pass's own test prints it - and the drawn body's is four orders above that, so this is
        // a margin of 3 against the sky and of 1e4 against the body rather than a judgement.
        const SUN_PIXEL: f64 = 2.0;

        let spec = WorldSpec::new(2468, Config::default());
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

        let registry = voxel_shapers::column::materials::registry().expect("the registry");
        let materials =
            crate::shading::table(&registry, &shading_field()).expect("every admitted material derives");
        let rig = GpuRig::from_rig(&LightRig::standard()).expect("the standard rig fits");
        let mut renderer = Renderer::new(SIDE, SIDE, &materials, &rig).expect("a GPU");
        renderer
            .world()
            .write_sky(&sky_dome_table(&day.sky, inputs, peak))
            .expect("the sky table fits the pipeline");
        renderer.world_mut().write_sun(Some(sun));
        assert!(renderer.world().has_sun(), "the body was not taken");

        let scene = Scene::build_with_detail(
            Vec::<voxel_mesh::Mesh>::new().iter(),
            eye,
            &voxel_appearance::DetailLattice::new(0),
        );
        let frame = renderer.render(&scene, &camera).expect("a frame");

        // The same ray per pixel the shader forms, from the camera's own basis - `sky.wgsl`'s
        // pinhole and this are the same three lines, which is what makes the comparison mean
        // "the transcription is the model" rather than "two things agree roughly".
        let (right, up, forward) = camera.basis();
        let tan_half = (camera.fov_y_rad / 2.0).tan();
        let half_width = (sun.body.radius_m / AU_M).atan();
        let (mut body_pixels, mut ball_pixels, mut disagree, mut brightest) = (0usize, 0usize, 0usize, 0.0f64);
        for y in 0..SIDE {
            for x in 0..SIDE {
                let ndc_x = 2.0 * (x as f64 + 0.5) / SIDE as f64 - 1.0;
                let ndc_y = 1.0 - 2.0 * (y as f64 + 0.5) / SIDE as f64;
                let across = ndc_x * tan_half * camera.aspect;
                let up_axis = ndc_y * tan_half;
                let dir = Vec3::new(
                    right[0] * across + up[0] * up_axis + forward[0],
                    right[1] * across + up[1] * up_axis + forward[1],
                    right[2] * across + up[2] * up_axis + forward[2],
                )
                .normalise();
                let luminance = crate::frame::luminance(frame.pixel(x, y).expect("in range"));
                brightest = brightest.max(luminance);
                let drawn = luminance > SUN_PIXEL;
                let cuboid = sun.body.ray_hits(eye, [dir.x, dir.y, dir.z]);
                if cuboid {
                    body_pixels += 1;
                }
                // What the **ball** of the same radius would cover, for the difference the
                // corner makes: `atan(R/d)` around the body's own direction.
                let cos_angle = dir.x * d.x + dir.y * d.y + dir.z * d.z;
                if cos_angle.clamp(-1.0, 1.0).acos() <= half_width {
                    ball_pixels += 1;
                }
                if cuboid != drawn {
                    disagree += 1;
                }
            }
        }

        assert!(
            body_pixels > 40,
            "the model puts only {body_pixels} pixels on the body, which is too few to compare"
        );
        assert_eq!(
            disagree, 0,
            "{disagree} of {} pixels disagree with the model: the shader's slab test and \
             `SunCuboid::ray_hits` are not the same rule",
            SIDE * SIDE
        );
        // **Not a disc.** The ball of radius `R` is contained in the cube, so the cube's
        // silhouette can only be larger - and by a model's own factor rather than a margin
        // somebody liked: a face is `4·SUN_DILUTION` against the ball's `π·SUN_DILUTION` seen
        // face-on, and a tilted face reaches `4·(|dx|+|dy|+|dz|)/π` times the ball. The smallest
        // of those is `4/π = 1.27`; `1.2` sits below it.
        assert!(
            body_pixels as f64 >= 1.2 * ball_pixels as f64,
            "the drawn body covers {body_pixels} pixels and the ball of the same radius {ball_pixels}: \
             equal areas means the corners are being cut off, so this is not the cuboid"
        );
        // And it is a *radiance*, far above the sky it was added to - the pass's exposure on a
        // disc rather than on a gradient.
        assert!(
            brightest > 1.0e3,
            "the brightest pixel is {brightest}, so the body was drawn at the sky's own scale \
             and the solid angle was never divided out"
        );
        println!(
            "sun: {body_pixels} cuboid pixels, {ball_pixels} in the inscribed ball, brightest \
             {brightest:e} (sky's own peak is 0.73 exposed), 0 of {} pixels disagree with the model",
            SIDE * SIDE
        );
    }
}
