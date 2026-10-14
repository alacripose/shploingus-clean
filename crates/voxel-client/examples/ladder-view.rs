//! CPU-side view of the material LOD ladder on a flat ground plane, through the
//! exact [`voxel_client::pattern`] math the shader transcribes: per-pixel ray to
//! the plane, [`voxel_client::pattern::eye_footprint_m`] footprint, and
//! [`voxel_client::pattern::Pattern`] deviation, written as a PPM.
//!
//! ```text
//! cargo run --offline -p voxel-client --example ladder-view
//! ```
//!
//! The picture this writes (`target/ladder-view.ppm`) is what the fragment
//! shader's ladder *should* draw for one material on flat ground from a grazing
//! eye: if the streaks and rectangular patches of the reported flicker show up
//! here, the cause lives in the shared address/weight logic and is fixable with
//! CPU tests; if this picture is clean fine grain, the cause lives on the GPU
//! side only (f32 address rebuild, interpolation, or a transcription slip the
//! gated agreement tests cannot catch here).

use voxel_appearance::DetailLattice;
use voxel_client::pattern::{
    eye_footprint_m, pixel_angle_rad, Pattern, OCTAVE_FLOOR, OCTAVE_TOP,
};
use voxel_model::MaterialId;
use voxel_shapers::materials::{pattern_of, shading_field, SAND};

const WIDTH: usize = 640;
const HEIGHT: usize = 360;
const FOV_Y_RAD: f64 = 70f64.to_radians();

fn main() {
    let lattice = DetailLattice::new(0);
    let field = shading_field().expect("the world's shading field declares");
    let pattern = Pattern::new(&lattice, &field);
    let material = MaterialId(SAND.0);
    let surface = pattern_of(material);
    let pixel_angle = pixel_angle_rad(FOV_Y_RAD, HEIGHT as u32) as f64;

    // Steep look-down like the report: ground fills the frame, near plane at
    // the bottom with millimetre footprints, far plane at the top.
    let eye = [0.0f64, 2.5, -6.0];
    let pitch = -1.0f64;
    let dir_down = pitch.sin();
    let dir_fwd = pitch.cos();
    // Camera looks along +z pitched down; aspect-correct ray per pixel.
    let aspect = WIDTH as f64 / HEIGHT as f64;
    let tan_half = (FOV_Y_RAD / 2.0).tan();

    let mut img = vec![0u8; WIDTH * HEIGHT * 3];
    let mut silent = 0usize;
    for y_px in 0..HEIGHT {
        for x_px in 0..WIDTH {
            let ndc_x = (x_px as f64 + 0.5) / WIDTH as f64 * 2.0 - 1.0;
            let ndc_y = 1.0 - (y_px as f64 + 0.5) / HEIGHT as f64 * 2.0;
            // Ray in camera space (x right, y up, z forward), pitched down.
            let rx = ndc_x * tan_half * aspect;
            let ry = ndc_y * tan_half;
            let rz = 1.0;
            // Camera basis pitched down: right stays +x, up tilts forward,
            // forward tilts down. With pitch = -D (D > 0 down):
            // forward = [0, sin p, cos p], up = [0, cos p, -sin p].
            let wy = ry * dir_fwd + rz * dir_down;
            let wz = -ry * dir_down + rz * dir_fwd;
            let ray = [rx, wy, wz];
            // Intersect y=0 plane.
            let t = if ray[1].abs() < 1e-9 {
                f64::INFINITY
            } else {
                -eye[1] / ray[1]
            };
            let idx = (y_px * WIDTH + x_px) * 3;
            if !t.is_finite() || t <= 0.0 {
                img[idx..idx + 3].copy_from_slice(&[0, 0, 0]);
                continue;
            }
            let p = [eye[0] + ray[0] * t, 0.0, eye[2] + ray[2] * t];
            let dist = ((p[0] - eye[0]).powi(2) + eye[1].powi(2) + (p[2] - eye[2]).powi(2)).sqrt();
            // Ground normal up; view vector toward eye.
            let v = [
                (eye[0] - p[0]) / dist,
                eye[1] / dist,
                (eye[2] - p[2]) / dist,
            ];
            let cos_face = v[1].abs();
            let footprint = eye_footprint_m(dist, pixel_angle, cos_face);
            let d = pattern
                .at(material, p, footprint, surface)
                .expect("a finite sample");
            if d.roughness_delta == 0.0 && d.reflectance_delta == 0.0 {
                silent += 1;
            }
            // Map reflectance deviation to grey; silence to pure black so rung
            // drop-outs read as structure, not as grain.
            let g = ((d.reflectance_delta * 0.5 + 0.5).clamp(0.0, 1.0) * 255.0) as u8;
            img[idx..idx + 3].copy_from_slice(&[g, g, g]);
        }
    }
    let total = WIDTH * HEIGHT;
    println!(
        "ladder-view: {WIDTH}x{HEIGHT} grazing ground plane, SAND/speckle; \
         silent {silent}/{total} ({:.1}%); rungs {OCTAVE_FLOOR}..={OCTAVE_TOP}",
        100.0 * silent as f64 / total as f64
    );
    // ---- vertex-path emulation -------------------------------------------
    // What the game actually draws with a light field: `Scene::build_with_light`
    // samples the lattice ONCE PER CORNER on the CPU and the fragment mixes the
    // interpolated corners (`with_a_sample`) with the ladder by coverage:
    // `deviation = mix(vertex, ladder.xy, ladder.z)`, where ladder.z is 1 while
    // any rung draws and 0 past the last rung's floor. Emulate one giant merged
    // quad over [-64, 64]^2 with its four corner samples, bilinearly
    // interpolated, to see whether the corner interpolation + hard ladder/vertex
    // switch is where the streaks and patches come from.
    let corners = [[-64.0f64, -64.0], [64.0, -64.0], [64.0, 64.0], [-64.0, 64.0]];
    let corner_sample = |wx: f64, wz: f64| -> (f64, f64) {
        // Corner world address, exactly as `Scene::build_with` derives it: the
        // world subvoxel cell and the in-cell fraction at DETAIL_SIDE.
        use voxel_model::{DetailIndex, DETAIL_SIDE, SUBVOXEL_SIDE};
        let cell_m = 1.0 / SUBVOXEL_SIDE as f64;
        let cell = [
            (wx / cell_m).floor() as i32,
            0,
            (wz / cell_m).floor() as i32,
        ];
        let frac = |w: f64| {
            let g = w / cell_m;
            let f = g - g.floor();
            ((f * DETAIL_SIDE as f64).floor() as u16).min(DETAIL_SIDE as u16 - 1)
        };
        let at = DetailIndex::new(frac(wx), 0, frac(wz)).expect("in range");
        let s = lattice
            .sample(material, cell, at)
            .expect("a corner sample");
        (s.roughness_delta, s.reflectance_delta)
    };
    let c00 = corner_sample(corners[0][0], corners[0][1]);
    let c10 = corner_sample(corners[1][0], corners[1][1]);
    let c11 = corner_sample(corners[2][0], corners[2][1]);
    let c01 = corner_sample(corners[3][0], corners[3][1]);

    let mut img_v = vec![0u8; WIDTH * HEIGHT * 3];
    for y_px in 0..HEIGHT {
        for x_px in 0..WIDTH {
            let ndc_x = (x_px as f64 + 0.5) / WIDTH as f64 * 2.0 - 1.0;
            let ndc_y = 1.0 - (y_px as f64 + 0.5) / HEIGHT as f64 * 2.0;
            let rx = ndc_x * tan_half * aspect;
            let ry = ndc_y * tan_half;
            let rz = 1.0;
            let wy = ry * dir_fwd + rz * dir_down;
            let wz = -ry * dir_down + rz * dir_fwd;
            let ray = [rx, wy, wz];
            let t = if ray[1].abs() < 1e-9 {
                f64::INFINITY
            } else {
                -eye[1] / ray[1]
            };
            let idx = (y_px * WIDTH + x_px) * 3;
            if !t.is_finite() || t <= 0.0 {
                continue;
            }
            let p = [eye[0] + ray[0] * t, 0.0, eye[2] + ray[2] * t];
            // Bilinear weights over the giant quad.
            let fx = ((p[0] + 64.0) / 128.0).clamp(0.0, 1.0);
            let fz = ((p[2] + 64.0) / 128.0).clamp(0.0, 1.0);
            let interp = |c00: f64, c10: f64, c11: f64, c01: f64| {
                c00 * (1.0 - fx) * (1.0 - fz)
                    + c10 * fx * (1.0 - fz)
                    + c11 * fx * fz
                    + c01 * (1.0 - fx) * fz
            };
            let vertex_dev = [
                interp(c00.0, c10.0, c11.0, c01.0),
                interp(c00.1, c10.1, c11.1, c01.1),
            ];
            let dist = ((p[0] - eye[0]).powi(2) + eye[1].powi(2) + (p[2] - eye[2]).powi(2)).sqrt();
            let v = [
                (eye[0] - p[0]) / dist,
                eye[1] / dist,
                (eye[2] - p[2]) / dist,
            ];
            let footprint = eye_footprint_m(dist, pixel_angle, v[1].abs());
            // Ladder coverage: 1 while any rung draws, else 0 — the shader's
            // `ladder.z`, recomputed here from the weights.
            let mut weight_sum = 0.0f64;
            for octave in OCTAVE_FLOOR..=OCTAVE_TOP {
                let cell_m = voxel_client::pattern::DETAIL_CELL_M * (1u64 << octave) as f64;
                let t = (1.0 - 4.0 * footprint / cell_m).clamp(0.0, 1.0);
                let w = t * t * (3.0 - 2.0 * t);
                weight_sum += w;
                if w <= 0.0 {
                    // keep summing: coverage needs every rung, silence is all-zero
                }
            }
            let coverage = if weight_sum > 0.0 { 1.0 } else { 0.0 };
            let d = pattern
                .at(material, p, footprint, surface)
                .expect("a finite sample");
            let mixed = vertex_dev[1] * (1.0 - coverage) + d.reflectance_delta * coverage;
            let g = ((mixed * 0.5 + 0.5).clamp(0.0, 1.0) * 255.0) as u8;
            img_v[idx..idx + 3].copy_from_slice(&[g, g, g]);
        }
    }
    println!("--- vertex-path emulation (one 128 m quad, interpolated corners) ---");
    const VRAMP: &[u8] = b" .:-=+*#%@";
    let (vcw, vch) = (WIDTH / 160, HEIGHT / 90);
    for by in 0..90 {
        let mut line = String::with_capacity(160);
        for bx in 0..160 {
            let mut sum = 0u64;
            for dy in 0..vch {
                for dx in 0..vcw {
                    sum += img_v[((by * vch + dy) * WIDTH + bx * vcw + dx) * 3] as u64;
                }
            }
            let mean = sum / (vcw * vch) as u64;
            line.push(VRAMP[(mean as usize * (VRAMP.len() - 1)) / 255] as char);
        }
        println!("{line}");
    }
    const RAMP: &[u8] = b" .:-=+*#%@";
    let (cw, ch) = (WIDTH / 160, HEIGHT / 90);
    for by in 0..90 {
        let mut line = String::with_capacity(160);
        for bx in 0..160 {
            let mut sum = 0u64;
            for dy in 0..ch {
                for dx in 0..cw {
                    sum += img[((by * ch + dy) * WIDTH + bx * cw + dx) * 3] as u64;
                }
            }
            let mean = sum / (cw * ch) as u64;
            line.push(RAMP[(mean as usize * (RAMP.len() - 1)) / 255] as char);
        }
        println!("{line}");
    }
    let row_bytes = WIDTH * 3;
    let padding = (4 - row_bytes % 4) % 4;
    let pixel_bytes = (row_bytes + padding) * HEIGHT;
    let mut out = Vec::with_capacity(54 + pixel_bytes);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&(54 + pixel_bytes as u32).to_le_bytes());
    out.extend_from_slice(&[0u8; 4]);
    out.extend_from_slice(&54u32.to_le_bytes());
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(WIDTH as i32).to_le_bytes());
    out.extend_from_slice(&(HEIGHT as i32).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&24u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&(pixel_bytes as u32).to_le_bytes());
    out.extend_from_slice(&[0u8; 16]);
    for y_px in (0..HEIGHT).rev() {
        for x_px in 0..WIDTH {
            let idx = (y_px * WIDTH + x_px) * 3;
            // BMP is BGR.
            out.push(img[idx + 2]);
            out.push(img[idx + 1]);
            out.push(img[idx]);
        }
        out.extend(std::iter::repeat(0u8).take(padding));
    }
    std::fs::create_dir_all("target").expect("target dir");
    std::fs::write("target/ladder-view.bmp", &out).expect("the picture lands");
    println!("wrote target/ladder-view.bmp");
}
