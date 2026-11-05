//! **Does the pattern hold still when the eye moves?** — fixed world points, two eyes, four numbers.
//!
//! The user's report (2026-10-25): *"slight changes in vertical position have drastic effects on
//! the materials appearance"*, and then the discriminator — *"it is not an issue that happens on the
//! horizontal axis of the blocks' faces unless jumping but always happens on the **vertical** sides
//! of blocks when moving"*.
//!
//! That asymmetry is the whole diagnosis, and this binary is the instrument for it. It asks the
//! ladder one question: **for a point that does not move, how much does the pattern change when the
//! eye does?** Everything else — parallax, silhouettes, the world moving under a pixel — is taken out
//! by holding the point fixed. A pattern that is a function of the surface answers the same; one that
//! is a function of the eye's *angle* to the surface answers differently on a wall than on a floor.
//!
//! ```text
//! cargo run --offline -p voxel-client --example pattern-hold
//! ```
//!
//! # Two faces, two steps, two laws
//!
//! * **the wall** — a vertical face (normal along `x`), the player's eye 2 m off it. Walking
//!   **parallel** to that wall slides the eye along `z`, which swings `|n·v|` from nearly 1 to nearly
//!   0 and back: the face angle moves fastest on exactly the surface the user named.
//! * **the floor** — a horizontal face (normal along `y`) at the player's feet. Walking slides
//!   `|n·v|` by almost nothing; **jumping** moves it, which is the user's other half.
//!
//! Each point is sampled at the still eye and at a **5 cm** step — one frame of the game's own walk —
//! on each axis, and the change in the ladder's two channels is reported. The numbers are printed for
//! two footprint laws:
//!
//! * **shipped** — `eye_footprint_m(distance, pixel_angle)`: distance only, no face term.
//! * **retired** — `distance · angle / max(|n·v|, 0.5)`: the law the frame carried before the fix,
//!   reproduced here rather than remembered, so the two are one run of one binary on one ladder.
//!
//! **What the fix has to show**: under the retired law a wall's points change when the eye walks past
//! (and a floor's when it jumps); under the shipped one both steps are small and the two faces answer
//! alike. The ladder's own content changed at the same time — a rung is interpolated between its
//! cell's corners instead of being one constant per cell — so a retired-law number here is not the
//! old picture pixel for pixel; it is the measurement of the eye-coupling that law imposed.

use voxel_appearance::DetailLattice;
use voxel_client::pattern::{
    eye_footprint_m, footprint_distance_m, pixel_angle_rad, Pattern, SUBVOXEL_CELL_M,
};
use voxel_model::MaterialId;
use voxel_shapers::materials::{pattern_of, shading_field, SAND, WOOD};

/// A player's frame: 70° vertical at 720 lines, so a pixel is about 1.7 mrad.
const FOV_Y_RAD: f64 = 70f64.to_radians();
const LINES: u32 = 720;
/// **One frame of the game's own walk**: 0.3 m/s at 60 Hz, the step `pattern-flicker` uses.
const STEP_M: f64 = 0.05;
/// The player's eye above the floor.
const EYE_M: f64 = 1.7;

/// One sampled point: where it is, which way its face looks, and what it is made of.
#[derive(Clone, Copy)]
struct Point {
    p: [f64; 3],
    normal: [f64; 3],
    material: MaterialId,
    surface: voxel_shapers::materials::SurfacePattern,
}

fn sample(
    pattern: &Pattern,
    point: &Point,
    eye: [f64; 3],
    pixel_angle: f64,
    retired: bool,
) -> (f64, f64) {
    let rel = [
        point.p[0] - eye[0],
        point.p[1] - eye[1],
        point.p[2] - eye[2],
    ];
    let distance = footprint_distance_m(rel);
    // The retired law's own face term, reconstructed from the same two vectors the shader had.
    let ray = (rel[0] * rel[0] + rel[1] * rel[1] + rel[2] * rel[2]).sqrt();
    let v = [rel[0] / ray, rel[1] / ray, rel[2] / ray];
    let cos = (point.normal[0] * v[0] + point.normal[1] * v[1] + point.normal[2] * v[2])
        .abs()
        .max(0.05);
    let footprint = if retired {
        distance * pixel_angle / cos
    } else {
        eye_footprint_m(distance, pixel_angle)
    };
    let d = pattern
        .at(point.material, point.p, footprint, point.surface)
        .expect("a finite sample");
    // The pattern is a tint and a one-sided roughness move; the tint is the visible one, and the two
    // are summed so a change in either counts.
    (
        d.roughness_delta + d.reflectance_delta.abs(),
        d.reflectance_delta,
    )
}

#[derive(Default)]
struct Report {
    n: usize,
    changed: usize,
    total: f64,
    worst: f64,
}

impl Report {
    fn add(&mut self, a: (f64, f64), b: (f64, f64)) {
        let delta = (a.0 - b.0).abs();
        self.n += 1;
        self.total += delta;
        self.worst = self.worst.max(delta);
        if delta > 1.0 / 255.0 {
            self.changed += 1;
        }
    }

    fn print(&self, name: &str) {
        let mean = self.total / self.n.max(1) as f64;
        println!(
            "  {name:<28} {:.1}% changed   mean {:.4}   worst {:.4}",
            100.0 * self.changed as f64 / self.n.max(1) as f64,
            mean,
            self.worst
        );
    }
}

fn main() {
    let lattice = DetailLattice::new(0);
    let field = shading_field().expect("the world's shading field declares");
    let pattern = Pattern::new(&lattice, &field);
    let pixel_angle = pixel_angle_rad(FOV_Y_RAD, LINES) as f64;

    // ---- the two faces ----------------------------------------------------
    // **At the distances the tiles are visible at.** Close up the ladder is saturated on its
    // finest rungs, whose weights sit at 1 and do not move; the mosaic the user photographed is
    // the mid-ground, where 0.25–8 m rungs share the pixel and every weight is mid-fade. So the
    // wall stands 20 m off (the trunk-and-wall range of the screenshots) and the floor runs out
    // to 70 m, with the eye walking forward along `z`.
    let mut wall = Vec::new();
    for i in 0..9 {
        for j in 0..5 {
            let z = -2.0 + i as f64 * 1.5;
            let y = 0.5 + j as f64 * 0.5;
            wall.push(Point {
                p: [20.0, y, z],
                normal: [-1.0, 0.0, 0.0],
                material: MaterialId(WOOD.0),
                surface: pattern_of(MaterialId(WOOD.0)),
            });
        }
    }
    // The floor at `y = 0`, spread ahead of the eye in `z`.
    let mut floor = Vec::new();
    for i in 0..9 {
        for j in 0..5 {
            let x = -6.0 + i as f64 * 1.5;
            let z = 8.0 - j as f64 * 12.0;
            floor.push(Point {
                p: [x, 0.0, z],
                normal: [0.0, 1.0, 0.0],
                material: MaterialId(SAND.0),
                surface: pattern_of(MaterialId(SAND.0)),
            });
        }
    }

    let eye = [0.0, EYE_M, -2.0];
    println!(
        "pattern-hold: {} wall points, {} floor points, {:.6} rad a pixel, step {STEP_M} m, eye {eye:?}",
        wall.len(),
        floor.len(),
        pixel_angle
    );
    println!(
        "a point that does not move, sampled from two eyes: the change in the ladder's value, so \
         parallax is not in these numbers\n"
    );

    for (law, retired) in [("shipped (distance)", false), ("retired (obliquity)", true)] {
        println!("{law}:");
        for (face, points) in [("wall", &wall), ("floor", &floor)] {
            for (step, axis, label) in [
                ("walk", 2usize, "walk along the face"),
                ("jump", 1usize, "jump up"),
            ] {
                let mut moved = eye;
                moved[axis] += STEP_M;
                let mut report = Report::default();
                let mut still_report = Report::default();
                for point in points.iter() {
                    let a = sample(&pattern, point, eye, pixel_angle, retired);
                    let b = sample(&pattern, point, moved, pixel_angle, retired);
                    report.add(a, b);
                    // **The control**: the same point from the same eye twice, which must be exactly
                    // zero — a non-zero here would be the probe's own arithmetic, not the ladder.
                    let c = sample(&pattern, point, eye, pixel_angle, retired);
                    still_report.add(a, c);
                }
                let name = format!("{face}, {label} ({step})");
                report.print(&name);
                assert_eq!(still_report.total, 0.0, "the probe moved the point: {name}");
            }
        }
        println!();
    }

    // ---- and the address itself ------------------------------------------
    // A world point's subvoxel cell is a fact about the world, not about the eye: this states it,
    // because the eye's two halves (`eye_phase` + `eye_voxel`) are how the shader rebuilds it.
    let p = wall[0].p;
    let cell = [
        (p[0] / SUBVOXEL_CELL_M).floor() as i32,
        (p[1] / SUBVOXEL_CELL_M).floor() as i32,
        (p[2] / SUBVOXEL_CELL_M).floor() as i32,
    ];
    println!("wall point {p:?} is subvoxel cell {cell:?} from any eye");
}
