//! **Which rung's address moves when the eye does?** — the question a pixel count cannot answer.
//!
//! The user, 2026-10-08: *"material blending looks great horizontally but it still flickers
//! between multiple different addresses"*. `pattern-flicker` counts pixels that change; it cannot
//! say **which rung** changed them, and that is the whole question — every rung quantises its
//! address to a *different* grid, so "a different address" and "a different dominant rung" are
//! the same statement.
//!
//! ```text
//! cargo run --offline --release -p voxel-client --example rung-attribution
//! ```
//!
//! # What it measures
//!
//! One block, one metre a side, sampled densely on its three visible faces. For each sample the
//! probe computes what the shader computes — the fragment's distance, its face cosine, the
//! footprint from `pattern::eye_footprint_m`, and every rung's `pattern::octave_weight` — and
//! reports **the rung carrying the most weight**. It then moves the eye one walk-sized step
//! **vertically** and again **horizontally**, and counts how often the dominant rung changed.
//!
//! **The horizontal step is the control**, and it is the user's own datum: it is fixed, so
//! whatever the vertical step does beyond it is the defect.
//!
//! # Why it is CPU and not the GPU
//!
//! The shader's own arithmetic is transcribed here from the same three functions the shader calls,
//! and the probe reads a *known* geometry rather than a rendered one. So the numbers carry no
//! rasteriser variance, no dither and no tonemap, and a difference between the two axes cannot be
//! an artefact of the measurement. What it cannot do is prove the shader agrees — that is
//! `renderer::tests::the_pattern_is_drawn_where_the_greedy_mesh_has_no_geometry`'s job, and the two
//! are complementary rather than redundant.
//!
//! # Reading it
//!
//! `top` is the face seen most nearly edge-on, where `cos_face` is small and `FACE_COS_FLOOR`
//! stretches the footprint up to twenty-fold. A vertical step moves the eye *toward* that face, so
//! its footprint sweeps rung thresholds far faster than the same step sideways. If the `top` row
//! dominates, the cause is the footprint's swing with elevation. If `left`/`front` dominate
//! instead, the cause is in the address derivation and the footprint story is wrong.

use std::collections::BTreeMap;

use voxel_client::pattern::{
    eye_footprint_m, octave_cell_m, octave_weight, pixel_angle_rad, octave_span, quantise_cell, FACE_COS_FLOOR,
    OCTAVE_GAIN, OCTAVE_TOP, SUBVOXEL_CELL_M,
};

/// The ladder's floor rung (`pattern::OCTAVE_FLOOR`), which `lib.rs` does not re-export.
const OCTAVE_FLOOR: u32 = 1;

/// One frame of a slow walk, and the only distance that matters to this question.
const STEP_M: f64 = 0.05;

/// Samples a side across, per face. The face is one metre, so this is a 5 mm pitch — well
/// inside a lattice cell, so the counts are not themselves aliased.
const SAMPLES: usize = 200;

/// A block one metre a side, standing on the ground, with the eye two metres above and in front.
const SIDE: f64 = 1.0;
const EYE: [f64; 3] = [0.5, 2.0, -2.0];

/// The three faces a camera above and in front of that block can see.
///
/// Each is `(name, normal, a point on the face)`. The cosine below is derived from these by the
/// same `abs(dot(n, v))` the shader uses, so nothing here is a hand-typed constant.
const FACES: [(&'static str, [f64; 3], [f64; 3]); 3] = [
    ("top", [0.0, 1.0, 0.0], [0.5, SIDE, 0.5]),
    ("left", [-1.0, 0.0, 0.0], [0.0, 0.5, 0.5]),
    ("front", [0.0, 0.0, 1.0], [0.5, 0.5, SIDE]),
];

/// Every rung's weight at this fragment — the shader's own ladder, rung by rung.
fn weights(footprint: f64) -> Vec<(u32, f64)> {
    (OCTAVE_FLOOR..=OCTAVE_TOP)
        .map(|octave| (octave, octave_weight(footprint, octave_cell_m(octave))))
        .collect()
}

/// The rung carrying the ladder, and how much of it.
///
/// **The weight is `octave_weight × OCTAVE_GAIN^(octave − OCTAVE_FLOOR)`, not the weight alone.**
/// `octave_weight` rises towards one as the cell grows, so ranking by it alone always answers
/// `OCTAVE_TOP` — whose cell is 32 m, and whose address therefore never moves for a one-metre
/// block. That is what the first version of this probe did, and it reported zero changes on every
/// face for exactly that reason. The mean is built from weight *and* gain, so that is what is
/// ranked here: octave 11 carries `1 × 0.5^10 ≈ 0.001` of the ladder, and the rung that actually
/// decides the surface is a fine one whose address moves with the fragment.
fn dominant(footprint: f64) -> Option<(u32, f64)> {
    let mut best: Option<(u32, f64)> = None;
    let mut total = 0.0f64;
    for (octave, weight) in weights(footprint) {
        let effective = weight * OCTAVE_GAIN.powi(octave as i32 - OCTAVE_FLOOR as i32);
        total += effective;
        if best.is_none_or(|(_, b)| effective > b) {
            best = Some((octave, effective));
        }
    }
    best.map(|(octave, effective)| (octave, effective / total))
}

fn main() {
    let angle = f64::from(pixel_angle_rad(std::f64::consts::FRAC_PI_3, 1080));
    println!(
        "rung-attribution: one {SIDE} m block, {SAMPLES}² samples a face, \
         {angle:.6} rad a pixel, step {STEP_M} m"
    );

    // **The ladder itself, at the distances the user's frames are at.** "Which rung
    // dominates" is a derived answer and it lied once already; this is the arithmetic.
    // The `pf` column marks the rungs `PREFILTER_SPAN` actually reaches.
    println!("\nthe ladder at a range of footprints (`pf` = prefiltered by the shipped span):");
    for distance in [1.5f64, 2.0, 3.0, 5.0, 10.0, 30.0] {
        for (label, cos) in [("head-on", 0.9f64), ("oblique", 0.3)] {
            let footprint = eye_footprint_m(distance, angle, cos);
            let rows: Vec<(u32, f64, f64)> = weights(footprint)
                .into_iter()
                .map(|(o, w)| {
                    let effective = w * OCTAVE_GAIN.powi(o as i32 - OCTAVE_FLOOR as i32);
                    (o, w, effective)
                })
                .collect();
            let total: f64 = rows.iter().map(|(_, _, e)| *e).sum();
            if total <= 0.0 {
                println!("  {distance:>5.1} m {label:>8}: the ladder is silent (footprint {footprint:.5} m)");
                continue;
            }
            let best = rows.iter().max_by(|a, b| a.2.total_cmp(&b.2)).unwrap();
            let carrying: Vec<String> = rows
                .iter()
                .filter(|(_, _, e)| *e / total > 0.05)
                .map(|(o, _, e)| format!("{}:{:.0}%{}", o, 100.0 * e / total, if octave_span(*o) >= 8 { "*" } else { "" }))
                .collect();
            println!(
                "  {distance:>5.1} m {label:>8}: footprint {footprint:.5} m, carried by [{}], \
                 top octave {}",
                carrying.join(" "),
                best.0
            );
        }
    }

    // **The two steps, per face.** A change from `None` to a rung is the ladder waking up, and
    // one rung to another is a swap — both are "the address moved", and both are counted.
    let mut swaps: BTreeMap<(&str, String), usize> = BTreeMap::new();
    let mut per_face: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    let mut cosines: BTreeMap<&str, f64> = BTreeMap::new();

    for (name, normal, origin) in FACES {
        let mut moved = 0usize;
        let mut total = 0usize;
        let mut cos_sum = 0.0f64;

        for iv in 0..SAMPLES {
            for iu in 0..SAMPLES {
                let (a, b) = (iu as f64 / SAMPLES as f64, iv as f64 / SAMPLES as f64);
                // A point on the face's own plane, spread across it.
                let mut p = origin;
                // Two of the three faces vary on two axes; pick them by which the normal is not.
                let mut axes = [0usize, 1];
                if normal[0] != 0.0 {
                    axes = [1, 2];
                } else if normal[1] != 0.0 {
                    axes = [0, 2];
                }
                p[axes[0]] = a * SIDE;
                p[axes[1]] = b * SIDE;

                let read = |eye: [f64; 3]| -> Option<(u32, f64, [i32; 3])> {
                    let to_eye = [eye[0] - p[0], eye[1] - p[1], eye[2] - p[2]];
                    let distance = to_eye.iter().map(|v| v * v).sum::<f64>().sqrt();
                    let unit = [
                        to_eye[0] / distance,
                        to_eye[1] / distance,
                        to_eye[2] / distance,
                    ];
                    let cos = (normal[0] * unit[0] + normal[1] * unit[1] + normal[2] * unit[2])
                        .abs()
                        .max(FACE_COS_FLOOR);
                    let (octave, share) =
                        dominant(eye_footprint_m(distance, angle, cos))?;
                    // **The rung's own address**, which is the thing the user is watching:
                    // the same rung can stay dominant while the cell it reads jumps.
                    let cell = [
                        (p[0] / SUBVOXEL_CELL_M).floor() as i32,
                        (p[1] / SUBVOXEL_CELL_M).floor() as i32,
                        (p[2] / SUBVOXEL_CELL_M).floor() as i32,
                    ];
                    Some((octave, share, quantise_cell(cell, octave)))
                };

                let here = read(EYE);
                let up = read([EYE[0], EYE[1] + STEP_M, EYE[2]]);
                let side = read([EYE[0] + STEP_M, EYE[1], EYE[2]]);
                cos_sum += {
                    let to_eye = [EYE[0] - p[0], EYE[1] - p[1], EYE[2] - p[2]];
                    let d = to_eye.iter().map(|v| v * v).sum::<f64>().sqrt();
                    (normal[0] * to_eye[0] + normal[1] * to_eye[1] + normal[2] * to_eye[2]).abs() / d
                };

                let label = |v: Option<(u32, f64, [i32; 3])>| match v {
                    Some((octave, share, cell)) => format!("octave {octave} ({share:.0}%) @ {cell:?}"),
                    None => "silent".to_string(),
                };
                let (before, after_v, after_s) = (label(here), label(up), label(side));
                if before != after_v {
                    *swaps.entry((name, format!("vertical: {before} -> {after_v}"))).or_insert(0) += 1;
                    moved += 1;
                }
                if before != after_s {
                    *swaps
                        .entry((name, format!("horizontal: {before} -> {after_s}")))
                        .or_insert(0) += 1;
                }
                total += 1;
            }
        }

        cosines.insert(name, cos_sum / total as f64);
        per_face.insert(name, (moved, total));
    }

    println!("\nmean face cosine as the eye sees it (1.0 is head-on):");
    for (name, cos) in &cosines {
        println!("  {name:>6}: {cos:.4}", );
    }

    println!("\ndominant rung changed, by face:");
    for (name, (moved, total)) in &per_face {
        println!(
            "  {name:>6}: {moved:>6} of {total} samples ({:.1} %) on a vertical step",
            100.0 * *moved as f64 / *total as f64
        );
    }

    println!("\nevery change, ranked:");
    let mut ranked: Vec<_> = swaps.iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(a.1));
    for ((name, what), count) in ranked.iter().take(24) {
        println!("  {count:>6}  {name:>6}  {what}");
    }
}
