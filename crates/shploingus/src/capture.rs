//! **The frame recorder**: the game's own frames, read back from the texture it presented.
//!
//! An external screenshot tool samples a window that is being repainted underneath it, so
//! two of its images are not necessarily two presented frames — and a probe built on it can
//! neither prove nor disprove a one-frame change, which is exactly the claim a material LOD
//! defect makes. This reads the **swapchain texture the frame was drawn into**, after the
//! world and the shell have both been recorded into it and before it is handed to the
//! compositor, so consecutive files here are consecutive presented frames.
//!
//! It is off unless `SHPLOINGUS_CAPTURE_DIR` names a directory:
//!
//! ```text
//! SHPLOINGUS_CAPTURE_DIR=target/run cargo run -p shploingus
//! ```
//!
//! Each frame lands as `f-NNNN.ppm` (binary P6, the swapchain's own pixels) and one line in
//! `frames.tsv`: index, wall clock in milliseconds, the eye, the camera's yaw and pitch in
//! degrees, and the tick. Together they pair an image with the state that drew it, without a
//! clock being shared between two processes.
//!
//! # The corners under one piece of ground (`attributes.tsv`)
//!
//! A picture that flips between two frames says *that* the surface changed; it cannot say *which*
//! term moved, and with the eye held still the shader's uniform and the fragment's own arithmetic
//! are the same bytes in both frames — so what is left is what the geometry **states**: the
//! material, the corner sample, the face's own axis, the light bytes, and which rectangles exist at
//! all. [`corner_digest`] reads those off the same [`Scene`] the frame was drawn from, over a fixed
//! piece of ground (a **world** point, so a moving eye does not change which corners are counted),
//! and [`FrameRecorder::record_corners`] writes one line a frame. Two consecutive lines that differ
//! name the term: the axis counts, the sample counts, the record count, or the hash over everything.
//!
//! A readback per frame stalls the GPU on the map and is a measurement, not a shipping
//! path — it exists to be run for a stretch of frames and then switched off.

use std::io::Write;
use std::path::PathBuf;

use voxel_client::scene::Scene;

pub struct FrameRecorder {
    dir: PathBuf,
    trace: std::fs::File,
    attributes: std::fs::File,
    scene: std::fs::File,
    buffer: wgpu::Buffer,
    padded_row_bytes: u32,
    width: u32,
    height: u32,
    bgra: bool,
    index: u64,
}

impl FrameRecorder {
    /// The recorder `SHPLOINGUS_CAPTURE_DIR` asks for, or `None` when it is unset.
    pub fn from_env(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> Option<Self> {
        let dir: PathBuf = std::env::var_os("SHPLOINGUS_CAPTURE_DIR")?.into();
        if let Err(e) = std::fs::create_dir_all(&dir) {
            eprintln!("shploingus: cannot create the capture directory {}: {e}", dir.display());
            return None;
        }
        let padded_row_bytes = (width * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frame capture"),
            size: padded_row_bytes as u64 * height as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let trace = match std::fs::File::create(dir.join("frames.tsv")) {
            Ok(file) => file,
            Err(e) => {
                eprintln!("shploingus: cannot write the capture trace: {e}");
                return None;
            }
        };
        let attributes = match std::fs::File::create(dir.join("attributes.tsv")) {
            Ok(file) => file,
            Err(e) => {
                eprintln!("shploingus: cannot write the corner trace: {e}");
                return None;
            }
        };
        let scene = match std::fs::File::create(dir.join("scene.tsv")) {
            Ok(file) => file,
            Err(e) => {
                eprintln!("shploingus: cannot write the scene trace: {e}");
                return None;
            }
        };
        let bgra = matches!(
            format,
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
        );
        Some(Self {
            dir,
            trace,
            attributes,
            scene,
            buffer,
            padded_row_bytes,
            width,
            height,
            bgra,
            index: 0,
        })
    }

    /// **Record the frame this texture holds**, with the state that drew it.
    pub fn record(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture: &wgpu::Texture,
        eye_m: [f64; 3],
        yaw_rad: f64,
        pitch_rad: f64,
        tick: u64,
    ) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("frame capture"),
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.padded_row_bytes),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(Some(encoder.finish()));

        let slice = self.buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        let _ = device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        });
        if receiver.recv().map(|result| result.is_err()).unwrap_or(true) {
            eprintln!("shploingus: a captured frame could not be mapped");
            return;
        }
        let data = match slice.get_mapped_range() {
            Ok(range) => range,
            Err(e) => {
                eprintln!("shploingus: a captured frame could not be read: {e}");
                return;
            }
        };
        let path = self.dir.join(format!("f-{:04}.ppm", self.index));
        match std::fs::File::create(&path) {
            Ok(mut file) => {
                let _ = write!(file, "P6\n{} {}\n255\n", self.width, self.height);
                let mut row = Vec::with_capacity(self.width as usize * 3);
                for y in 0..self.height as usize {
                    row.clear();
                    let base = y * self.padded_row_bytes as usize;
                    for x in 0..self.width as usize {
                        let i = base + x * 4;
                        let px = if self.bgra {
                            [data[i + 2], data[i + 1], data[i]]
                        } else {
                            [data[i], data[i + 1], data[i + 2]]
                        };
                        row.extend_from_slice(&px);
                    }
                    let _ = file.write_all(&row);
                }
            }
            Err(e) => eprintln!("shploingus: cannot write {}: {e}", path.display()),
        }
        drop(data);
        self.buffer.unmap();

        let ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let _ = writeln!(
            self.trace,
            "{}\t{ms}\t{:.5}\t{:.5}\t{:.5}\t{:.5}\t{:.5}\t{tick}",
            self.index,
            eye_m[0],
            eye_m[1],
            eye_m[2],
            yaw_rad.to_degrees(),
            pitch_rad.to_degrees(),
        );
        let _ = self.trace.flush();
    }

    /// **What the geometry under one piece of ground states**, for the frame of this index.
    ///
    /// Called before [`FrameRecorder::record`] so the line and the image carry the same index. The
    /// point is a **world** one ([`corner_digest`]'s own reason), so the answer does not move when
    /// the eye does: a line that changes while the eye is still is the geometry speaking, which is
    /// precisely the one-frame flip this recorder exists to name.
    pub fn record_corners(&mut self, scene: &Scene, probe: [f64; 3], radius_m: f64) {
        let digest = corner_digest(scene, probe, radius_m);
        let _ = writeln!(
            self.attributes,
            "{}\t{:.3}\t{:.3}\t{:.3}\t{}\t{}\t{}\t{}\t{}\t{:.4}\t{}\t{}\t{}\t{:016x}",
            self.index,
            probe[0],
            probe[1],
            probe[2],
            digest.corners,
            digest.axes[0],
            digest.axes[1],
            digest.axes[2],
            digest.axes[3],
            digest.rough_mean,
            digest.light_absent,
            digest.materials,
            digest.records,
            digest.hash,
        );
        let _ = self.attributes.flush();
    }

    /// **Everything the frame was built from**, in one line — the whole scene, no radius.
    ///
    /// The local digest above answers "did the ground under this point change"; this one answers
    /// "did the frame change at all", and those are different questions. A picture that flips under
    /// a still eye and a still camera can only come from what the frame was built from (a scene, a
    /// rig, a shadow map, a sky) — the camera's own numbers are in `frames.tsv` and the shader reads
    /// nothing else. So a scene hash that holds across the flip clears the scene in one line, and one
    /// that moves names the side the defect is on without another instrument.
    pub fn record_scene(&mut self, scene: &Scene) {
        let digest = corner_digest(scene, [0.0; 3], f64::INFINITY);
        let _ = writeln!(
            self.scene,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:016x}",
            self.index,
            digest.corners,
            digest.axes[0],
            digest.axes[1],
            digest.axes[2],
            digest.axes[3],
            digest.records,
            digest.materials,
            digest.hash,
        );
        let _ = self.scene.flush();
    }

    /// **Close the frame.** Every line the recorder writes for one presented frame shares its
    /// index — the image, the camera's state, the ground's digest and the scene's — so the number
    /// is advanced once, after all of them, rather than by whichever writer happened to run last.
    pub fn next(&mut self) {
        self.index += 1;
    }
}

/// What the corners and rectangles within `radius_m` of a **world** point state, in one line.
///
/// The axis counts are `plane_axis` — `0`, `1`, `2`, and `-1` for a corner that states none, which
/// is the value the shader treats as "no face" and falls back to the material's world axis from. A
/// frame that flips between square and stretched cells on one face is a frame where the `-1` count or
/// the axis of a set of corners changed, and this is the line that shows it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CornerDigest {
    /// Corners of the scene inside the sphere.
    pub corners: usize,
    /// `plane_axis` census: x, y, z, and none.
    pub axes: [usize; 4],
    /// Corners whose quad has no measurement (`light_present == 0`).
    pub light_absent: usize,
    /// Mean `detail_roughness` of those corners.
    pub rough_mean: f64,
    /// Distinct material ids inside the sphere.
    pub materials: usize,
    /// Rectangles whose chunk origin is inside the sphere, widened by a chunk's reach.
    pub records: usize,
    /// One number over everything the corners and rectangles state, quantised so that a rebuild that
    /// states the same thing hashes the same and one that states something else does not.
    pub hash: u64,
}

/// Chunk reach, metres — the widening a rectangle's origin gets before it counts as "under" a point.
pub const CHUNK_REACH_M: f64 = 32.0;

/// Read the world's own digests out of the scene the frame was drawn from.
pub fn corner_digest(scene: &Scene, probe: [f64; 3], radius_m: f64) -> CornerDigest {
    let mut digest = CornerDigest::default();
    let mut materials = std::collections::BTreeSet::new();
    let mut rough_sum = 0.0f64;
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    let fold = |bytes: &[u8], hash: &mut u64| {
        for byte in bytes {
            *hash ^= u64::from(*byte);
            *hash = hash.wrapping_mul(0x1000_0000_01b3);
        }
    };
    for vertex in &scene.vertices {
        let origin = match scene.origins.get(vertex.chunk as usize) {
            Some(origin) => origin,
            // A corner naming an origin the scene does not have is the range a departed chunk left
            // behind (`Vertex::blank`), which covers nothing and is not a statement about the world.
            None => continue,
        };
        let world = [
            origin[0] + f64::from(vertex.position[0]),
            origin[1] + f64::from(vertex.position[1]),
            origin[2] + f64::from(vertex.position[2]),
        ];
        let d = [world[0] - probe[0], world[1] - probe[1], world[2] - probe[2]];
        if d[0] * d[0] + d[1] * d[1] + d[2] * d[2] > radius_m * radius_m {
            continue;
        }
        digest.corners += 1;
        digest.axes[match vertex.plane_axis {
            0 => 0,
            1 => 1,
            2 => 2,
            _ => 3,
        }] += 1;
        if vertex.light_present == 0 {
            digest.light_absent += 1;
        }
        rough_sum += f64::from(vertex.detail_roughness);
        materials.insert(vertex.material);
        // Byte-for-byte the attribute set the vertex stage hands the fragment, in the order the
        // format declares it — a term that moves moves this hash.
        fold(&vertex.material.to_le_bytes(), &mut hash);
        fold(&(vertex.plane_axis as i32).to_le_bytes(), &mut hash);
        fold(&vertex.light_present.to_le_bytes(), &mut hash);
        fold(&vertex.detail_roughness.to_bits().to_le_bytes(), &mut hash);
        for axis in 0..3 {
            // A thousandth of a metre: a rebuild that moves a corner by less than that draws the
            // same fragment, so it is the same statement.
            let quantised = (world[axis] * 1000.0).round() as i64;
            fold(&quantised.to_le_bytes(), &mut hash);
        }
    }
    for axis in 0..3 {
        fold(&(digest.axes[axis] as u32).to_le_bytes(), &mut hash);
    }
    digest.rough_mean = if digest.corners == 0 { 0.0 } else { rough_sum / digest.corners as f64 };
    digest.materials = materials.len();
    // **The rectangles too**: a face that left the record format for triangles, or joined a neighbour
    // in the merge, changes what the world states without moving a corner of the ones still there.
    let reach = radius_m + CHUNK_REACH_M;
    for record in &scene.records {
        let origin = match scene.origins.get(record.chunk as usize) {
            Some(origin) => origin,
            None => continue,
        };
        let d = [origin[0] - probe[0], origin[1] - probe[1], origin[2] - probe[2]];
        if d[0] * d[0] + d[1] * d[1] + d[2] * d[2] > reach * reach {
            continue;
        }
        digest.records += 1;
        fold(&record.face.to_le_bytes(), &mut hash);
        fold(&record.material.to_le_bytes(), &mut hash);
        fold(&record.slice.to_le_bytes(), &mut hash);
        fold(&record.chunk.to_le_bytes(), &mut hash);
        for axis in 0..3 {
            let quantised = (origin[axis] * 1000.0).round() as i64;
            fold(&quantised.to_le_bytes(), &mut hash);
        }
    }
    fold(&(digest.records as u32).to_le_bytes(), &mut hash);
    digest.hash = hash;
    digest
}
