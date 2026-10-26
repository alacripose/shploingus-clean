//! **The corner probe.** One vertical face and one horizontal face of the same material, both
//! 2 m on a side, meeting along the world's x axis at the chunk's low corner, seen from one
//! eye at `(1, 2, 2)`. Both faces are therefore **2.236 m away at 26.6° of obliquity**, and
//! the footprint the ladder is driven by is the same number for each — 2.816 mm — which the
//! probe prints rather than assumes.
//!
//! One frame, both faces, one held camera. For each face and each of its two in-plane axes it
//! prints the pattern's cell pitch **in world metres**, read off the rendered pixels, and the
//! same pitch read off the crate's law along the same line. Three comparisons follow from
//! that, and they are the whole question:
//!
//! * **within a face** — the two in-plane pitches are equal when cells are square;
//! * **between the faces** — a wall and a floor at the same footprint draw the same pitch;
//! * **against the law** — the frame's pitch equals the crate's on the same line, which is
//!   the only way to tell a shading defect from the world's own pattern at that address.
//!
//! It also holds every sampled pixel of both faces to `shading::shade_cpu` over
//! `pattern::Pattern::deviation`, so a shader that draws a different pattern on the wall than
//! the crate says is a number here rather than an impression of a picture.
//!
//! Writes `target/face-corner.png`.
//!
//! Usage: `cargo run --example face-square`

use voxel_appearance::npr::LightRig;
use voxel_appearance::openpbr::OpenPbrInputs;
use voxel_client::camera::Camera;
use voxel_client::pattern::{address_at, eye_footprint_m, pixel_angle_rad, Pattern};
use voxel_client::shading;
use voxel_client::{Frame, GpuRig, Renderer, Scene};
use voxel_mesh::{Mesh, MeshStats, Quad};
use voxel_model::{ChunkPos, Face, MaterialId};
use voxel_noise::Vec3;
use voxel_shapers::materials;

/// 1025 is odd, so pixel 512 is the frame's exact centre. At 100° over 1025 pixels a pixel
/// is 1.7 mrad wide — 3.8 mm on these faces — and the cell the ladder draws at this
/// footprint is 62.5 mm, so a cell is ~16 pixels across.
const SIDE: u32 = 1025;
const FIELD_DEG: f64 = 100.0;
/// The material the world paints its broad ground and rock: the shipped registry's own, so a
/// material the world declines to vary is not what gets measured.
const MATERIAL: MaterialId = MaterialId(3);
/// The two faces are 2 m on a side, four subvoxels a metre.
const EDGE: u16 = 8;
/// The sample band inside a face, in metres: half a metre in from every edge, so no sample
/// reads the other face, a face edge, or the sky.
const BAND_LO: f64 = 0.45;
const BAND_HI: f64 = 1.55;

fn srgb(v: f32) -> u8 {
    let v = v.clamp(0.0, 1.0) as f64;
    let encoded = if v <= 0.003_130_8 { 12.92 * v } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 };
    (encoded * 255.0 + 0.5) as u8
}

fn write_png(frame: &Frame, path: &str) {
    let mut bytes = Vec::with_capacity((SIDE * SIDE * 4) as usize);
    for pixel in frame.pixels.iter() {
        bytes.push(srgb(pixel[0]));
        bytes.push(srgb(pixel[1]));
        bytes.push(srgb(pixel[2]));
        bytes.push(255);
    }
    let file = std::fs::File::create(path).expect("a file to write into target/");
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), SIDE, SIDE);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("a PNG header");
    writer.write_image_data(&bytes).expect("the pixels");
    drop(writer);
    println!("wrote {path}");
}

/// A world point's pixel, through the camera's own matrix — the same `view_projection` the
/// vertex stage multiplies by, so a sample lands where the fragment did.
fn project(camera: &Camera, p: [f64; 3]) -> Option<(f64, f64)> {
    let m = camera.view_projection();
    let rel = [
        p[0] - camera.position_m[0],
        p[1] - camera.position_m[1],
        p[2] - camera.position_m[2],
    ];
    let mut clip = [0.0f64; 4];
    for (r, cell) in clip.iter_mut().enumerate() {
        *cell = (0..4)
            .map(|k| m[k][r] as f64 * if k == 3 { 1.0 } else { rel[k] })
            .sum();
    }
    if clip[3] <= 1.0e-9 {
        return None;
    }
    Some((
        (clip[0] / clip[3] + 1.0) * 0.5 * SIDE as f64,
        (1.0 - clip[1] / clip[3]) * 0.5 * SIDE as f64,
    ))
}

/// Bilinear read of a channel of the frame, or `None` off the edge of the frame.
fn sample(frame: &Frame, x: f64, y: f64, channel: usize) -> Option<f64> {
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let at = |i: f64, j: f64| -> Option<f64> {
        if i < 0.0 || j < 0.0 || i > (SIDE - 1) as f64 || j > (SIDE - 1) as f64 {
            return None;
        }
        frame.pixel(i as u32, j as u32).map(|p| p[channel] as f64)
    };
    let (a, b) = (at(x0, y0)?, at(x0 + 1.0, y0)?);
    let (c, d) = (at(x0, y0 + 1.0)?, at(x0 + 1.0, y0 + 1.0)?);
    let top = a * (1.0 - fx) + b * fx;
    let bottom = c * (1.0 - fx) + d * fx;
    Some(top * (1.0 - fy) + bottom * fy)
}

/// **The pattern's cell pitch along one world direction**, in metres, from a signal sampled
/// every 0.5 mm (the image resolves 3.8 mm a pixel, so the walk oversamples it rather than
/// aliasing it). A boundary is a local maximum of the step between neighbours above a third
/// of the line's own strongest step; two candidates closer than 4 mm are one boundary.
/// `None` when the walk leaves the frame or finds no boundary at all.
fn pitch_of(values: &[f64], len_m: f64) -> Option<(f64, usize)> {
    let step: Vec<f64> = values.windows(2).map(|w| w[1] - w[0]).collect();
    let peak = step.iter().fold(0.0f64, |m, d| m.max(d.abs()));
    if !(peak > 0.0) || !peak.is_finite() {
        return None;
    }
    let threshold = peak / 3.0;
    const SAMPLE_M: f64 = 0.0005;
    let guard = (0.004 / SAMPLE_M) as usize;
    let mut boundaries = 0usize;
    let mut last: Option<usize> = None;
    for i in 1..step.len().saturating_sub(1) {
        let here = step[i].abs();
        if here < threshold || here < step[i - 1].abs() || here < step[i + 1].abs() {
            continue;
        }
        if let Some(previous) = last {
            if i - previous < guard {
                continue;
            }
        }
        boundaries += 1;
        last = Some(i);
    }
    if boundaries == 0 {
        return None;
    }
    Some((len_m / boundaries as f64, boundaries))
}

const SAMPLE_M: f64 = 0.0005;

/// **One face of the probe**, with the two in-plane unit axes the pitch is measured along.
struct Panel {
    name: &'static str,
    quad: Quad,
    normal: [f64; 3],
    /// The face's own low corner in metres — the point its `uv` origin maps to.
    corner: [f64; 3],
    /// The face's in-plane axes, each a unit world direction.
    u: [f64; 3],
    w: [f64; 3],
}

/// `mesh::axes`: axis 0 is (y, z), axis 1 is (x, z), axis 2 is (x, y).
fn face_axes(face: Face) -> ([f64; 3], [f64; 3]) {
    match face {
        Face::NegX | Face::PosX => ([0.0, 1.0, 0.0], [0.0, 0.0, 1.0]),
        Face::NegY | Face::PosY => ([1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
        Face::NegZ | Face::PosZ => ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
    }
}

/// The crate's own reading of one world point on a face: the deviation this fragment shows
/// and the colour `shade_cpu` makes of it. The footprint is `eye_footprint_m`'s, from the
/// point's own distance and its face's own angle to the eye — the same two numbers the
/// shader derives from the fragment.
struct Law<'a> {
    pattern: Pattern<'a>,
    material: MaterialId,
    inputs: OpenPbrInputs,
    surface: materials::SurfacePattern,
    rig: LightRig,
    eye: [f64; 3],
    pixel_angle: f64,
}

impl Law<'_> {
    /// (this point's deviation, the red channel `shade_cpu` makes of it).
    fn point(&self, p: [f64; 3], normal: [f64; 3], plane: Option<u32>) -> Option<(f64, f64)> {
        let d = [p[0] - self.eye[0], p[1] - self.eye[1], p[2] - self.eye[2]];
        let distance = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        if distance <= 0.0 {
            return None;
        }
        let facing = (normal[0] * d[0] + normal[1] * d[1] + normal[2] * d[2]).abs() / distance;
        let footprint = eye_footprint_m(distance, self.pixel_angle);
        let deviation = self
            .pattern
            .at(self.material, p, footprint, self.surface)
            .expect("a finite point");
        let varied = self.inputs.with_detail(deviation).expect("a legal deviation");
        let view = Vec3::new(-d[0] / distance, -d[1] / distance, -d[2] / distance);
        let (want, _) = shading::shade_cpu(
            &varied,
            Vec3::new(normal[0], normal[1], normal[2]),
            view,
            0.0,
            &self.rig,
        )
        .ok()?;
        Some((deviation.roughness_delta, want.r))
    }

    fn red(&self, p: [f64; 3], normal: [f64; 3], plane: Option<u32>) -> Option<f64> {
        self.point(p, normal, plane).map(|(_, red)| red)
    }
}

fn main() {
    let registry = materials::registry().expect("the registry");
    let field = materials::shading_field().expect("the world's shading field declares");
    let table = shading::table(&registry, &field).expect("every admitted material derives");
    let inputs = shading::inputs_for(&registry, MATERIAL).expect("the material derives");
    let surface = materials::pattern_of(MATERIAL);
    let gain = field.gain(MATERIAL).expect("the world varies this material");
    let rig = LightRig::standard();
    let gpu_rig = GpuRig::from_rig(&rig).expect("the standard rig fits");
    let mut renderer = Renderer::new(SIDE, SIDE, &table, &gpu_rig).expect("a GPU");
    renderer.world_mut().write_no_sky_model();
    println!("adapter: {}", renderer.adapter);
    println!(
        "material {} ({:?}), ladder gain ({:.3}, {:.3}), tonemap {} (0 is the measurement target)",
        MATERIAL.0,
        surface,
        gain.0,
        gain.1,
        renderer.world().tonemap()
    );

    // ---- the corner: a wall at z = 0 and a floor at y = 0, both 2 m, both in one chunk ----
    let wall = Panel {
        name: "wall ",
        quad: Quad { face: Face::PosZ, slice: 0, uv: [0, 0], extent: [EDGE, EDGE], material: MATERIAL },
        normal: [0.0, 0.0, 1.0],
        corner: [0.0, 0.0, 0.0],
        u: [1.0, 0.0, 0.0],
        w: [0.0, 1.0, 0.0],
    };
    let floor = Panel {
        name: "floor",
        quad: Quad { face: Face::PosY, slice: 0, uv: [0, 0], extent: [EDGE, EDGE], material: MATERIAL },
        normal: [0.0, 1.0, 0.0],
        corner: [0.0, 0.0, 0.0],
        u: [1.0, 0.0, 0.0],
        w: [0.0, 0.0, 1.0],
    };
    for panel in [&wall, &floor] {
        let (u, w) = face_axes(panel.quad.face);
        assert_eq!(u, panel.u, "{}: the u axis is not `mesh::axes`'s", panel.name);
        assert_eq!(w, panel.w, "{}: the w axis is not `mesh::axes`'s", panel.name);
    }

    let mesh = |quad: Quad| Mesh {
        chunk: ChunkPos::new(0, 0, 0),
        origin: [0, 0, 0],
        quads: vec![quad],
        loops: Vec::new(),
        stats: MeshStats::default(),
    };
    let wall_mesh = mesh(wall.quad);
    let floor_mesh = mesh(floor.quad);

    // The eye the two faces were chosen for: 2.236 m from each centre at 26.6° off each
    // normal, so `eye_footprint` is one number for both.
    let eye = [1.0, 2.0, 2.0];
    let mut camera = Camera::standard(eye, 1.0);
    camera.fov_y_rad = FIELD_DEG.to_radians();
    // Looking at the corner's own bisector: (0, -1, -2) and (0, -2, -1) average to (0, -3, -3).
    camera.yaw_rad = std::f64::consts::PI;
    camera.pitch_rad = -std::f64::consts::FRAC_PI_4;

    let scene = Scene::build([&wall_mesh, &floor_mesh], eye, None, None);
    println!(
        "scene: {} quads, {} records, {} bent",
        scene.quads,
        scene.records.len(),
        scene.bent_quads
    );
    let frame = renderer.render(&scene, &camera).expect("a frame");
    write_png(&frame, "target/face-corner.png");

    let lattice = voxel_appearance::DetailLattice::new(field.domain());
    let pixel_angle = pixel_angle_rad(camera.fov_y_rad, SIDE) as f64;
    let law = Law {
        pattern: Pattern::new(&lattice, &field),
        material: MATERIAL,
        inputs,
        surface,
        rig,
        eye,
        pixel_angle,
    };

    println!("\n=== the ladder's footprint, by the crate's own law ===");
    let mut footprints = Vec::new();
    for panel in [&wall, &floor] {
        let mut centre = panel.corner;
        for axis in 0..3 {
            centre[axis] += panel.u[axis] + panel.w[axis];
        }
        let d = [centre[0] - eye[0], centre[1] - eye[1], centre[2] - eye[2]];
        let distance = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        let facing = (panel.normal[0] * d[0] + panel.normal[1] * d[1] + panel.normal[2] * d[2])
            .abs()
            / distance;
        let footprint = eye_footprint_m(distance, pixel_angle);
        footprints.push(footprint);
        println!(
            "  {:<6}: centre {:?}, {:.3} m away, footprint {:.3} mm",
            panel.name,
            centre.map(|c| (c * 1000.0).round() / 1000.0),
            distance,
            footprint * 1000.0
        );
    }

    // ---- the drawn cell, from the stencil (exact) ---------------------------------------
    //
    // A pitch read off the picture is a noisy instrument — a bilinear sample either side
    // of a cell edge moves the count by one — so the **cell** itself is taken from the
    // stencil, which is the law the frame is held to underneath. One subvoxel of `at` is
    // sixteen base cells, and the floor rung's own mask halves the count, so a stretched
    // axis reads four crossings where a square one reads sixteen.
    let cell_mm = |surface: materials::SurfacePattern, axis: usize, plane: Option<u32>| -> f64 {
        let mut crossings = 0usize;
        let mut previous: Option<(u16, u16, u16)> = None;
        for i in 0..16u16 {
            let mut a = [8u16; 3];
            a[axis] = i;
            let at = voxel_model::DetailIndex::new(a[0], a[1], a[2]).expect("inside the side");
            let s = surface.stencil(at, plane);
            let quantised = (s.x >> 1, s.y >> 1, s.z >> 1);
            if let Some(p) = previous {
                if p != quantised {
                    crossings += 1;
                }
            }
            previous = Some(quantised);
        }
        250.0 / crossings.max(1) as f64
    };
    println!("
=== the drawn cell along each in-plane axis, in world metres ===");
    let axis_of = |dir: [f64; 3]| -> usize {
        if dir[0].abs() > 0.5 {
            0
        } else if dir[1].abs() > 0.5 {
            1
        } else {
            2
        }
    };
    for panel in [&wall, &floor] {
        let plane = Some(panel.quad.face.axis() as u32);
        for (label, dir) in [("along u", panel.u), ("along w", panel.w)] {
            println!(
                "  {:<6} {label}: {:.1} mm ({surface:?})",
                panel.name,
                cell_mm(surface, axis_of(dir), plane)
            );
        }
    }

    // ---- every sampled pixel against the law ---------------------------------------------
    println!("\n=== every sampled point of both faces against `shade_cpu` ===");
    let mut worst = [0.0f64; 2];
    let mut deltas: [Vec<f64>; 2] = [Vec::new(), Vec::new()];
    for (index, panel) in [&wall, &floor].into_iter().enumerate() {
        // A 97² grid of world points across the band: a picture of the face, not one pixel.
        const GRID: usize = 97;
        for i in 0..GRID {
            for j in 0..GRID {
                let a = BAND_LO + (BAND_HI - BAND_LO) * (i as f64 + 0.5) / GRID as f64;
                let b = BAND_LO + (BAND_HI - BAND_LO) * (j as f64 + 0.5) / GRID as f64;
                let mut p = panel.corner;
                for axis in 0..3 {
                    p[axis] += panel.u[axis] * a + panel.w[axis] * b;
                    if panel.normal[axis] != 0.0 {
                        p[axis] = panel.corner[axis];
                    }
                }
                let Some(red) = project(&camera, p).and_then(|(x, y)| sample(&frame, x, y, 0)) else {
                    continue;
                };
                let Some(want) = law.red(p, panel.normal, Some(panel.quad.face.axis() as u32)) else {
                    continue;
                };
                let delta = (red - want).abs();
                worst[index] = worst[index].max(delta);
                deltas[index].push(delta);
            }
        }
        deltas[index].sort_by(|a, b| a.partial_cmp(b).expect("finite"));
        let n = deltas[index].len();
        let pick = |q: f64| deltas[index][((n as f64 - 1.0) * q) as usize];
        println!(
            "  {:<6}: {n} points, median {:.2e}, 90th {:.2e}, worst {:.2e}, over 1e-2: {}",
            panel.name,
            pick(0.5),
            pick(0.9),
            worst[index],
            deltas[index].iter().filter(|d| **d > 1.0e-2).count()
        );
    }

    println!("
=== the comparison ===");
    println!(
        "  the two footprints: {:.3} mm and {:.3} mm — ratio {:.4}",
        footprints[0] * 1000.0,
        footprints[1] * 1000.0,
        footprints[0] / footprints[1]
    );

    // ---- the directional pair: two bark walls at right angles -------------------------
    //
    // The report was about *certain* vertical sides, and this is why: a grain keyed to a
    // world axis vanishes on the face that looks along it. Two walls of the same wood,
    // both 2.5 m away at 36.9° of obliquity — one footprint, one material, only the facing
    // differs.
    println!("\n=== the directional pair: two walls of one bark, at right angles ===");
    let bark: MaterialId = voxel_shapers::materials::WOOD;
    let bark_inputs = shading::inputs_for(&registry, bark).expect("wood derives");
    let bark_surface = materials::pattern_of(bark);
    println!("  material {} ({bark_surface:?})", bark.0);
    let eye = [32.0, 16.0, 30.0];
    let facing_z = Panel {
        name: "wall z",
        quad: Quad {
            face: Face::PosZ,
            slice: 112,
            uv: [118, 60],
            extent: [EDGE, EDGE],
            material: bark,
        },
        normal: [0.0, 0.0, 1.0],
        corner: [29.5, 15.0, 28.0],
        u: [1.0, 0.0, 0.0],
        w: [0.0, 1.0, 0.0],
    };
    let facing_x = Panel {
        name: "wall x",
        quad: Quad {
            face: Face::PosX,
            slice: 120,
            uv: [54, 116],
            extent: [EDGE, EDGE],
            material: bark,
        },
        normal: [1.0, 0.0, 0.0],
        corner: [30.0, 13.5, 29.0],
        u: [0.0, 1.0, 0.0],
        w: [0.0, 0.0, 1.0],
    };
    for panel in [&facing_z, &facing_x] {
        let (u, w) = face_axes(panel.quad.face);
        assert_eq!(u, panel.u, "{}: the u axis is not `mesh::axes`'s", panel.name);
        assert_eq!(w, panel.w, "{}: the w axis is not `mesh::axes`'s", panel.name);
    }
    let z_mesh = mesh(facing_z.quad);
    let x_mesh = mesh(facing_x.quad);
    let bisector = [-3.5f64, -1.5, -2.0];
    let length = (bisector[0] * bisector[0] + bisector[1] * bisector[1] + bisector[2] * bisector[2])
        .sqrt();
    let mut camera = Camera::standard(eye, 1.0);
    camera.fov_y_rad = FIELD_DEG.to_radians();
    camera.yaw_rad = bisector[0].atan2(bisector[2]);
    camera.pitch_rad = (bisector[1] / length).asin();
    let scene = Scene::build([&z_mesh, &x_mesh], eye, None, None);
    let frame = renderer.render(&scene, &camera).expect("a frame");
    write_png(&frame, "target/face-bark.png");
    let law = Law {
        pattern: Pattern::new(&lattice, &field),
        material: bark,
        inputs: bark_inputs,
        surface: bark_surface,
        rig: LightRig::standard(),
        eye,
        pixel_angle,
    };
    for panel in [&facing_z, &facing_x] {
        let plane = Some(panel.quad.face.axis() as u32);
        let mut cells = Vec::new();
        for (label, dir) in [("along u", panel.u), ("along w", panel.w)] {
            let axis = if dir[0].abs() > 0.5 { 0 } else if dir[1].abs() > 0.5 { 1 } else { 2 };
            let cell = cell_mm(bark_surface, axis, plane);
            println!("  {:<6} {label}: cell {cell:.1} mm", panel.name);
            cells.push(cell);
        }
        // And the frame is held to the law on this very wall: a 33² grid of points.
        let mut deltas = Vec::new();
        for i in 0..33 {
            for j in 0..33 {
                let a = BAND_LO + (BAND_HI - BAND_LO) * (i as f64 + 0.5) / 33.0;
                let b = BAND_LO + (BAND_HI - BAND_LO) * (j as f64 + 0.5) / 33.0;
                let mut p = panel.corner;
                for axis in 0..3 {
                    p[axis] += panel.u[axis] * a + panel.w[axis] * b;
                    if panel.normal[axis] != 0.0 {
                        p[axis] = panel.corner[axis];
                    }
                }
                let Some(drawn) = project(&camera, p).and_then(|(x, y)| sample(&frame, x, y, 0))
                else {
                    continue;
                };
                let Some(want) = law.red(p, panel.normal, plane) else { continue };
                deltas.push((drawn - want).abs());
            }
        }
        deltas.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
        let n = deltas.len();
        println!(
            "  {:<6}: {} points against the law, median {:.2e}, 90th {:.2e}, aspect {:.2}:1",
            panel.name,
            n,
            deltas[n / 2],
            deltas[(n as f64 * 0.9) as usize],
            cells[0].max(cells[1]) / cells[0].min(cells[1])
        );
    }
}
