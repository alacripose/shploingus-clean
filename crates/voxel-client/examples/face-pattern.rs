//! **One pattern, three faces.** A wall, a floor, and the same wall seen at a walk-past
//! angle — one material, one distance, the shipped rectangle path. Writes
//! `target/face-vertical.png`, `target/face-horizontal.png` and `target/face-grazing.png`,
//! and prints, for each, the drawn pattern's step spacing **in metres** along the face's
//! two in-plane directions. That is the measurement "the cells stop being squares on
//! vertical sides" is asking for: square cells step at the same spacing both ways, and a
//! face the eye skims is where the ladder's footprint is stretched.
//!
//! It also counts faces covered twice in the meshes — the other way a surface comes out
//! looking like it is z-fighting — and asks the address law for one fixed world point
//! from four eyes, which must answer the same cell every time.
//!
//! The GPU-versus-crate hold on a single pixel is **not** here: a staged ladder weight
//! moves the colour by a few percent at some footprints, and
//! `renderer::tests::a_face_on_a_cell_boundary_draws_its_own_cell_from_every_eye` is the
//! exact instrument for that question.

use voxel_appearance::npr::LightRig;
use voxel_client::camera::Camera;
use voxel_client::pattern::{address_at, eye_footprint_m, pixel_angle_rad, Pattern};
use voxel_client::shading;
use voxel_client::{GpuRig, Renderer, Scene};
use voxel_mesh::{Chunk, Mesh, MeshConfig, MeshStats, Mesher, Quad};
use voxel_model::{ChunkPos, MaterialId, CHUNK_SIDE, SUBVOXEL_SIDE};
use voxel_shapers::{materials, Config, WorldSpec};
use voxel_stream::WorldSource;

/// 513 is odd, so pixel 256 samples the frame's exact centre: the point the camera is
/// aimed at, with no half-pixel of view error.
const SIDE: u32 = 513;
const CENTRE: u32 = SIDE / 2;
/// A field of view a person would recognise as "looking at a block".
const FIELD_DEG: f64 = 51.0;
/// The obliquity the wall and the floor are both viewed at: 30° off the normal, so `n·v`
/// is the same for both and the footprint differs only through the distance.
const OBLIQUE: f64 = 0.5;
/// A step smaller than this in the red channel is the smooth lighting gradient, not the
/// pattern: the view term moves by about a thousandth of a channel per pixel here.
const STEP: f32 = 3.0e-3;

fn srgb(v: f32) -> u8 {
    let v = v.clamp(0.0, 1.0) as f64;
    let encoded = if v <= 0.003_130_8 { 12.92 * v } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 };
    (encoded * 255.0 + 0.5) as u8
}

fn write_png(frame: &voxel_client::Frame, path: &str) {
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

fn normal_of(quad: &Quad) -> [f64; 3] {
    let (x, y, z) = quad.face.normal();
    [x as f64, y as f64, z as f64]
}

/// A quad's four corners in world metres, from its chunk's own origin.
fn corners_m(mesh: &Mesh, quad: &Quad) -> [[f64; 3]; 4] {
    let origin = [
        mesh.chunk.x as f64 * CHUNK_SIDE as f64,
        mesh.chunk.y as f64 * CHUNK_SIDE as f64,
        mesh.chunk.z as f64 * CHUNK_SIDE as f64,
    ];
    let mut out = [[0.0f64; 3]; 4];
    for (i, c) in quad.corners().iter().enumerate() {
        for axis in 0..3 {
            out[i][axis] = c[axis] as f64 / SUBVOXEL_SIDE as f64 + origin[axis];
        }
    }
    out
}

fn centre_of(corners: &[[f64; 3]; 4]) -> [f64; 3] {
    let mut c = [0.0f64; 3];
    for corner in corners {
        for axis in 0..3 {
            c[axis] += corner[axis] / 4.0;
        }
    }
    c
}

/// **Every face covered twice**, across the whole region — the within-chunk count the
/// mesher's own coverage test makes, and the cross-chunk one it cannot see.
fn duplicate_faces(meshes: &[Mesh]) -> (usize, usize, usize) {
    use std::collections::BTreeMap;
    let mut seen: BTreeMap<([i32; 3], u8, bool), usize> = BTreeMap::new();
    let mut coplanar: BTreeMap<([i32; 3], u8), usize> = BTreeMap::new();
    let mut quads = 0usize;
    for mesh in meshes {
        let origin = [
            mesh.chunk.x * CHUNK_SIDE,
            mesh.chunk.y * CHUNK_SIDE,
            mesh.chunk.z * CHUNK_SIDE,
        ];
        for quad in &mesh.quads {
            quads += 1;
            let axis = quad.face.axis() as usize;
            let (au, av) = match axis {
                0 => (1, 2),
                1 => (0, 2),
                _ => (0, 1),
            };
            for dv in 0..quad.extent[1] as i32 {
                for du in 0..quad.extent[0] as i32 {
                    let mut c = [0i32; 3];
                    c[axis] = quad.slice as i32;
                    c[au] = quad.uv[0] as i32 + du;
                    c[av] = quad.uv[1] as i32 + dv;
                    // Collapsed onto the solid cell, exactly as the mesher's own guard does.
                    let solid = if quad.face.is_positive() { c[axis] - 1 } else { c[axis] };
                    c[axis] = solid;
                    for a in 0..3 {
                        c[a] += origin[a] * SUBVOXEL_SIDE as i32;
                    }
                    *seen.entry((c, axis as u8, quad.face.is_positive())).or_insert(0) += 1;
                    *coplanar.entry((c, axis as u8)).or_insert(0) += 1;
                }
            }
        }
    }
    let twice = seen.values().filter(|n| **n > 1).count();
    let coplanar_twice = coplanar.values().filter(|n| **n > 1).count();
    (quads, twice, coplanar_twice)
}

/// Where a pixel's own ray meets the plane through `p` with normal `n`.
fn ray_hit(camera: &Camera, i: u32, j: u32, p: [f64; 3], n: [f64; 3]) -> Option<[f64; 3]> {
    let (right, up, forward) = camera.basis();
    let t = (camera.fov_y_rad / 2.0).tan();
    let ndc_x = 2.0 * (i as f64 + 0.5) / SIDE as f64 - 1.0;
    let ndc_y = 1.0 - 2.0 * (j as f64 + 0.5) / SIDE as f64;
    let mut dir = [0.0f64; 3];
    for axis in 0..3 {
        dir[axis] =
            forward[axis] + right[axis] * ndc_x * camera.aspect * t + up[axis] * ndc_y * t;
    }
    let denom = dir[0] * n[0] + dir[1] * n[1] + dir[2] * n[2];
    if denom.abs() < 1.0e-9 {
        return None;
    }
    let to_plane = (p[0] - camera.position_m[0]) * n[0]
        + (p[1] - camera.position_m[1]) * n[1]
        + (p[2] - camera.position_m[2]) * n[2];
    let s = to_plane / denom;
    Some([
        camera.position_m[0] + dir[0] * s,
        camera.position_m[1] + dir[1] * s,
        camera.position_m[2] + dir[2] * s,
    ])
}

fn unit(v: [f64; 3]) -> [f64; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    [v[0] / len, v[1] / len, v[2] / len]
}

/// The face's own in-plane horizontal, or the world's x for a floor: the direction the
/// eye is slid along to give every face the same obliquity.
fn in_plane(n: [f64; 3]) -> [f64; 3] {
    if n[1].abs() > 0.9 {
        [1.0, 0.0, 0.0]
    } else {
        unit([n[1] * 0.0 - n[2] * 1.0, n[2] * 0.0 - n[0] * 0.0, n[0] * 1.0 - n[1] * 0.0])
    }
}

fn median(v: &mut Vec<f64>) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

fn main() {
    // ---- the region, meshed exactly as the renderer's own tests mesh it -----------------
    let source = WorldSource::new(WorldSpec::new(2468, Config::default()));
    let registry = materials::registry().expect("the registry");
    let field = materials::shading_field().expect("the world's shading field declares");
    let table = shading::table(&registry, &field).expect("every admitted material derives");
    let mut mesher = Mesher::new();
    let config = MeshConfig::from_registry(&registry).expect("a mesh config");
    let mut meshes: Vec<Mesh> = Vec::new();
    for cy in 0..2 {
        for cx in 0..3 {
            for cz in 0..3 {
                let pos = ChunkPos::new(cx, cy, cz);
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
    let (quads, twice, coplanar) = duplicate_faces(&meshes);
    println!(
        "meshed {} chunks, {quads} quads: {twice} faces covered twice in the same direction, \
         {coplanar} cells with more than one face on the same plane",
        meshes.len()
    );

    // ---- one vertical face and one horizontal one, of the same varied material ---------
    let speckle = |m: MaterialId| {
        matches!(materials::pattern_of(m), materials::SurfacePattern::Speckle)
    };
    let mut pairs: std::collections::BTreeMap<MaterialId, (usize, Quad, f64, usize, Quad, f64)> =
        Default::default();
    for (index, mesh) in meshes.iter().enumerate() {
        for quad in &mesh.quads {
            if !speckle(quad.material) || field.gain(quad.material).is_none() {
                continue;
            }
            let area = quad.area() as f64;
            let entry = pairs
                .entry(quad.material)
                .or_insert((usize::MAX, *quad, 0.0, usize::MAX, *quad, 0.0));
            if quad.face.axis() != 1 {
                if area > entry.2 {
                    *entry = (index, *quad, area, entry.3, entry.4, entry.5);
                }
            } else if quad.face.is_positive() && area > entry.5 {
                *entry = (entry.0, entry.1, entry.2, index, *quad, area);
            }
        }
    }
    let best = pairs
        .iter()
        .filter(|(_, p)| p.2 > 0.0 && p.5 > 0.0)
        .max_by(|a, b| a.1 .2.min(a.1 .5).partial_cmp(&b.1 .2.min(b.1 .5)).unwrap());
    let Some((&material, &(vi, vquad, varea, hi, hquad, harea))) = best else {
        println!("the region has no varied speckle face of both orientations; nothing to measure");
        return;
    };
    println!(
        "material {} ({:?}): vertical {:?} {:.2} m² on chunk {:?}, horizontal {:?} {:.2} m² on \
         chunk {:?}",
        material.0,
        materials::pattern_of(material),
        vquad.face,
        varea / 16.0,
        meshes[vi].chunk,
        hquad.face,
        harea / 16.0,
        meshes[hi].chunk,
    );

    let rig = LightRig::standard();
    let gpu_rig = GpuRig::from_rig(&rig).expect("the standard rig fits");
    let mut renderer = Renderer::new(SIDE, SIDE, &table, &gpu_rig).expect("a GPU");
    renderer.world_mut().write_no_sky_model();
    println!("adapter: {}", renderer.adapter);

    let lattice = voxel_appearance::DetailLattice::new(field.domain());
    let pattern = Pattern::new(&lattice, &field);
    let surface = materials::pattern_of(material);
    let pixel_angle = pixel_angle_rad(FIELD_DEG.to_radians(), SIDE) as f64;

    // **Only the face itself is drawn**, so the centre pixel is that face or the sky:
    // nothing else in the region can be between the eye and it.
    let alone = |index: usize, quad: Quad| Mesh {
        chunk: meshes[index].chunk,
        origin: {
            let o = meshes[index].chunk.origin();
            [o.x, o.y, o.z]
        },
        quads: vec![quad],
        loops: Vec::new(),
        stats: MeshStats::default(),
    };
    let wall_only = alone(vi, vquad);
    let floor_only = alone(hi, hquad);

    // Both faces are drawn from the same distance, so the only difference between the two
    // frames is which way the plane faces.
    let distance = {
        let half = (varea.max(harea).sqrt() / 4.0 / 2.0).max(0.25);
        (half / (FIELD_DEG.to_radians() / 2.0).tan()).max(0.4)
    };

    // **And the wall at a walk-past angle.** A vertical side is what the eye sees at
    // grazing incidence, so 78° off its normal is the case the report is about.
    for (name, mesh, quad, area, oblique, png) in [
        ("wall ", &wall_only, &vquad, varea, OBLIQUE, Some("target/face-vertical.png")),
        ("floor", &floor_only, &hquad, harea, OBLIQUE, Some("target/face-horizontal.png")),
        ("wall 78°", &wall_only, &vquad, varea, 0.978, Some("target/face-grazing.png")),
    ] {
        let corners = corners_m(mesh, quad);
        let face_centre = centre_of(&corners);
        let n = normal_of(quad);
        let u = in_plane(n);
        let w = [u[1] * n[2] - u[2] * n[1], u[2] * n[0] - u[0] * n[2], u[0] * n[1] - u[1] * n[0]];
        // **A face's centre is a corner of the address grid**, where eight cells meet and
        // "which one" is a convention rather than a claim; the point measured is half a
        // cell in on both in-plane axes, so only the face's own axis is on a boundary.
        let mut centre = face_centre;
        for axis in 0..3 {
            centre[axis] += u[axis] * 0.125 + w[axis] * 0.125;
        }
        let cos = (1.0 - oblique * oblique).sqrt();
        let eye = [
            centre[0] + n[0] * distance * cos + u[0] * distance * oblique,
            centre[1] + n[1] * distance * cos + u[1] * distance * oblique,
            centre[2] + n[2] * distance * cos + u[2] * distance * oblique,
        ];
        let mut camera = Camera::standard(eye, 1.0);
        camera.fov_y_rad = FIELD_DEG.to_radians();
        let d = [centre[0] - eye[0], centre[1] - eye[1], centre[2] - eye[2]];
        let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        camera.yaw_rad = d[0].atan2(d[2]);
        camera.pitch_rad = (d[1] / len).clamp(-1.0, 1.0).asin();

        let scene = Scene::build(std::iter::once(mesh), eye, None, None);
        let frame = renderer.render(&scene, &camera).expect("a frame");
        if let Some(path) = png {
            write_png(&frame, path);
        }

        // What the law says this pixel is, for the record: the face's own cell on its
        // normal axis, a mid-cell address on the two in-plane axes, and a footprint that
        // grows by `1/cos` as the face turns away from the eye.
        let footprint = eye_footprint_m(len, pixel_angle);
        let deviation = pattern
            .at(material, centre, footprint, surface)
            .expect("a finite point");
        println!(
            "\n{name}: {distance:.2} m away, {:.0} m² face, {oblique:.3} obliquity, footprint \
             {:.1} mm\n  deviation {:.4}",
            area / 16.0,
            footprint * 1000.0,
            deviation.roughness_delta
        );

        // ---- where the visible pattern steps land, in metres --------------------------
        for (label, across) in [("row", true), ("column", false)] {
            let mut spacings: Vec<f64> = Vec::new();
            let mut previous: Option<[f32; 4]> = None;
            let mut previous_hit: Option<[f64; 3]> = None;
            for i in 1..SIDE - 1 {
                let (x, y) = if across { (i, CENTRE) } else { (CENTRE, i) };
                let Some(pixel) = frame.pixel(x, y) else { continue };
                if let Some(before) = previous {
                    if (pixel[0] - before[0]).abs() > STEP {
                        let (ax, ay) = if across { (x - 1, y) } else { (x, y - 1) };
                        if let (Some(pa), Some(pb)) = (
                            ray_hit(&camera, ax, ay, centre, n),
                            ray_hit(&camera, x, y, centre, n),
                        ) {
                            let mid = [
                                (pa[0] + pb[0]) / 2.0,
                                (pa[1] + pb[1]) / 2.0,
                                (pa[2] + pb[2]) / 2.0,
                            ];
                            if let Some(last) = previous_hit {
                                spacings.push(
                                    ((mid[0] - last[0]).powi(2)
                                        + (mid[1] - last[1]).powi(2)
                                        + (mid[2] - last[2]).powi(2))
                                    .sqrt(),
                                );
                            }
                            previous_hit = Some(mid);
                        }
                    }
                }
                previous = Some(pixel);
            }
            println!(
                "  {label:>6}: {} visible steps, median spacing {:.1} mm in world metres",
                spacings.len() + usize::from(!spacings.is_empty()),
                median(&mut spacings) * 1000.0,
            );
        }
    }

    // ---- one world point, four eyes ---------------------------------------------------
    println!("\nthe wall's face centre, asked from four eyes (the law must not move):");
    let mut centre = centre_of(&corners_m(&wall_only, &vquad));
    let n = normal_of(&vquad);
    let u = in_plane(n);
    let w = [u[1] * n[2] - u[2] * n[1], u[2] * n[0] - u[0] * n[2], u[0] * n[1] - u[1] * n[0]];
    for axis in 0..3 {
        centre[axis] += u[axis] * 0.125 + w[axis] * 0.125;
    }
    for (what, distance, slide) in [
        ("head-on, 2 m", 2.0f64, 0.0f64),
        ("head-on, 2.25 m", 2.25, 0.0),
        ("head-on, 1.75 m", 1.75, 0.0),
        ("slid 0.37 m in the face's plane", 2.0, 0.37),
    ] {
        let eye = [
            centre[0] + n[0] * distance + u[0] * slide,
            centre[1] + n[1] * distance + u[1] * slide,
            centre[2] + n[2] * distance + u[2] * slide,
        ];
        let dir = [centre[0] - eye[0], centre[1] - eye[1], centre[2] - eye[2]];
        let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
        let facing = (n[0] * dir[0] + n[1] * dir[1] + n[2] * dir[2]).abs() / len;
        let footprint = eye_footprint_m(len, pixel_angle);
        let deviation = pattern
            .at(material, centre, footprint, surface)
            .expect("a finite point");
        println!(
            "  {what:>34}: footprint {:.1} mm, deviation {:+.4}",
            footprint * 1000.0,
            deviation.roughness_delta
        );
    }
}
