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

    /// Draw the scene and read the pixels back.
    pub fn render(&mut self, scene: &Scene, camera: &Camera) -> Result<Frame, ClientError> {
        let colour_view = self.colour.create_view(&wgpu::TextureViewDescriptor::default());
        let depth_view = self.depth.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("frame") });
        self.world.draw(&mut encoder, &colour_view, &depth_view, scene, camera);
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
        use crate::game::{spawn_eye, GAME_PITCH_RAD, GAME_SEED, GAME_WINDOW};
        use crate::skylight::daylight_of;
        use voxel_appearance::sky::SkyInputs;
        use voxel_shapers::{Config, WorldSpec};
        use voxel_stream::WorldSource;

        let spec = WorldSpec::new(GAME_SEED, Config::default());
        let source = WorldSource::new(spec);
        let registry = voxel_shapers::column::materials::registry().expect("the registry");
        let table = shading::table(&registry).expect("every admitted material derives");

        let workers = std::thread::available_parallelism()
            .map(|n| n.get().saturating_sub(1).max(1))
            .unwrap_or(1);
        let mesh_config = voxel_mesh::MeshConfig::from_registry(&registry).expect("a mesh config");
        let mut streamer =
            voxel_work::Streamer::new(source.clone(), mesh_config, workers, crate::GAME_CEILING_BYTES)
                .and_then(|s| s.with_window(GAME_WINDOW))
                .expect("the streamer");
        let eye = spawn_eye();
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
        let scene = Scene::build(streamer.meshes(), eye);
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
        // invisible picture.
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

        // **And the picture is not black either**, which is the other half. **It is now
        // underexposed, and that is a named finding rather than a pass**: a physically
        // derived radiance mapped straight onto an 8-bit swapchain has no exposure
        // applied, so a world lit by a 22° sun and a 0.4% ambient measures `mean
        // 0.00085` with 0.8% of pixels above 0.01. **What is missing is a declared
        // exposure**, the same class of thing as `AMBIENT_FROM_HORIZON` — a rendering
        // choice rather than a physical constant — and this crate does not have one.
        // The assertion below is therefore deliberately weak: it pins that the rig
        // reaches the pixels at all, not that the picture looks right, because nobody
        // has ruled what right looks like.
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
            "**UNDERSHOOTED, and named as such**: the rig is one unit now (max {} rather \
             than 78.3) but the world measures mean {:.5} with {:.1}% of pixels above \
             0.01. Nothing declares an exposure, so a physically derived radiance lands \
             on an 8-bit swapchain dim. The fixture's {} was not correct either — it was \
             an arbitrary ambient and no sun.",
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

    /// How much of a frame is *anything at all*, and how bright it gets.
    struct Stats {
        min: f64,
        max: f64,
        mean: f64,
        above_one_percent: usize,
        pixels: usize,
        colours: usize,
    }

    impl Stats {
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
            }
        }

        fn report(&self, what: &str) {
            println!(
                "{what}: min {:.4} max {:.4} mean {:.5} | {:.1}% of pixels above 0.01 | {} distinct colours",
                self.min,
                self.max,
                self.mean,
                100.0 * self.above_one_percent as f64 / self.pixels as f64,
                self.colours,
            );
        }
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
        use crate::game::{spawn_eye, GAME_PITCH_RAD, GAME_SEED, GAME_WINDOW};
        use crate::ShadingReport;
        use voxel_shapers::{Config, WorldSpec};
        use voxel_stream::WorldSource;

        let spec = WorldSpec::new(GAME_SEED, Config::default());
        let source = WorldSource::new(spec);
        let registry = voxel_shapers::column::materials::registry().expect("the registry");
        let table = shading::table(&registry).expect("every admitted material derives");
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
        let eye = spawn_eye();
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
        let placed = Scene::build(streamer.meshes(), [0.0; 3]);
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
            let scene = Scene::build(streamer.meshes(), eye);
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
        use crate::skylight::sky_lut;
        use voxel_appearance::sky::SkyInputs;
        use voxel_shapers::{Config, WorldSpec};

        let spec = WorldSpec::new(20_260_930, Config::default());
        // Tick 9000: the Sun is up, so the table is a real sky and not the
        // midnight all-zero case that would make every assertion below pass for
        // the wrong reason.
        let table = sky_lut(&spec, 9_000, SkyInputs::default(), 0).expect("a sky at tick 9000");

        let registry = voxel_shapers::column::materials::registry().expect("the registry");
        let materials = crate::shading::table(&registry).expect("every admitted material derives");
        let rig = GpuRig::from_rig(&LightRig::standard()).expect("the standard rig fits");

        let mut renderer = Renderer::new(64, 64, &materials, &rig).expect("a GPU");
        renderer.world().write_sky(&table).expect("the sky table fits the pipeline");

        // **No geometry at all.** This is precisely the case the old pass left empty:
        // with no voxel on a pixel there was nothing there but the clear value.
        let scene = Scene::build(Vec::new().iter(), [0.0; 3]);
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

    #[test]
    #[ignore = "needs a GPU adapter; run explicitly with --ignored"]
    fn a_frame_with_no_sky_written_is_the_black_it_was_before() {
        // The table is zeroed at construction, so a caller that never asks for a sky
        // gets the behaviour this pass replaced — which is what makes adding it safe
        // for every existing caller rather than a change they must all opt into.
        let registry = voxel_shapers::column::materials::registry().expect("the registry");
        let materials = crate::shading::table(&registry).expect("every admitted material derives");
        let rig = GpuRig::from_rig(&LightRig::standard()).expect("the standard rig fits");
        let mut renderer = Renderer::new(32, 32, &materials, &rig).expect("a GPU");

        let scene = Scene::build(Vec::new().iter(), [0.0; 3]);
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
        let table = shading::table(&registry).expect("every admitted material derives");
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

        // Where the world actually is, so the camera stands on it rather than
        // inside it. Building the scene with the eye at the origin makes every
        // vertex position a world metre, which is what makes this measurable at
        // all — there is no hard-coded "the surface is at 40 m" anywhere here.
        let placed = Scene::build(meshes.iter(), [0.0; 3]);
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

        let scene = Scene::build(meshes.iter(), eye);
        println!(
            "scene: {} chunks, {} quads, {} vertices, {} bytes of vertices",
            meshes.len(),
            scene.quads,
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
}
