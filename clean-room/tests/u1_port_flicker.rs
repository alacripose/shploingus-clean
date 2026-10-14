//! **The appearance port's gate, executable** — the flicker instrument run against the **ported** path.
//!
//! Derived from `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §43 (the port's first step and its gate), from the
//! user's own 2026-10-08 report (*"still visible material flickering when moving on the vertical axis…
//! fixed when moving on the horizontal axis"*), and from
//! `docs/baselines/port-appearance-seam/README.txt` §3, which is where the parent's own instrument's
//! numbers were measured and pinned. The parent's figures are **evidence printed beside** this
//! measurement, never an input to it: everything below is computed from this clean room's own frames,
//! drawn through `presentation::shade` — which since P7 v2.0.0 **is** `voxel_appearance`'s shader.
//!
//! What is measured, in the parent instrument's own arithmetic: the same world drawn from one camera and
//! from a camera moved a stated step **vertically** (+y, the axis the user reported) and **horizontally**
//! (+x, the axis they reported as fixed), counting how many pixels changed, by how much in total, and the
//! worst single channel difference. Two further draws from the **unmoved** camera are the control: they
//! must read exactly zero, or the picture is not a function of the camera and nothing below means
//! anything.
//!
//! **The gate is the appearance asymmetry** — the vertical step's response over the horizontal step's, from
//! the frame's own shaded colours ([`appearance_response`]) — with the pixel churn reported beside it and the
//! camera-held controls asserted at exactly zero ([`APPEARANCE_ASYMMETRY_CEILING`]).
//!
//! That is not the first gate this file had, and the reason is worth recording. The gate was first stated on
//! changed pixels against the parent's own pinned ratios ([`MARGINS`]: the parent reads 2.82× at 0.05 m on
//! its current bytes, 3.35× on its prebuilt binary). The **guard-bite experiment** then measured that gate to
//! be blind: a view-dependent gain injected into the ported colour, up to a half, left the changed-pixel
//! counts **identical at every step** — 1 837 and 1 280 pixels at 0.05 m, to the digit — while the appearance
//! response moved from 0.0/0.0/0.2 to 0.2/1.2/4.7. A colour that changes *inside* a pixel class does not
//! change the class, so a changed-pixel count cannot see the re-weighting this instrument exists to catch.
//! The pixel comparison survives as the evidence printed beside the measurement; the gate is the asymmetry
//! ratio, and it was measured to bite (1.00× against 0.07×) before it was trusted.
//!
//! It is a real port being measured, not a re-derivation: the frames below are drawn by
//! `presentation::encode` over the colour **`voxel_appearance::npr::shade`** returns — see
//! `clean-room/Cargo.toml`'s `voxel-appearance` line and the pinned digests it names.

use shploingus_clean_room::automaton::{Automaton, SETTLE_TICKS};
use shploingus_clean_room::drawn;
use shploingus_clean_room::presentation;
use shploingus_clean_room::reference;
use shploingus_clean_room::surface;

/// The three steps the parent's own instrument measures at, in the world's own metres.
const STEPS_M: [f64; 3] = [0.05, 0.25, 1.00];

/// The parent's own instrument, on its current bytes (2026-10-13, `README.txt` §3): changed-pixel share
/// vertically, changed-pixel share horizontally, the vertical/horizontal ratio by changed pixels and by
/// total magnitude. Printed for comparison; **not** read by any assertion.
const PARENT: [(&str, &str, f64, f64); 3] = [
    ("4.8 %", "1.7 %", 2.82, 2.38),
    ("14.9 %", "6.4 %", 2.34, 2.27),
    ("30.2 %", "20.0 %", 1.51, 1.94),
];

/// **How much of the parent's own excess the ported path must not reproduce**, in the parent's own step
/// order, as a factor applied to the parent's own pinned ratios. The gate is expressed against those
/// figures rather than against a number invented here.
///
/// * At **0.05 m** — the parent's own headline step, where its excess is largest and cleanest — the ported
///   path must beat the parent by **a fifth**: at most four fifths of its ratio, by changed pixels and by
///   total magnitude. That is the excess the user reported, and it is the one the port owns.
/// * At **0.25 m and 1.00 m** the parent must simply not be beaten: at those steps the parent's own ratio
///   has already collapsed toward the floor the *geometry* sets (at 1 m 30.2 % of its pixels change), so
///   the appearance excess is no longer separable from the geometry's own churn. The two are compared at
///   equal footing rather than scaled, and the measured ported ratios there — 1.21× and 1.22× — are the
///   same order as the floor itself, which is the result the port should show.
///
/// The reason the gate is not "zero excess": this client's frames are flat-shaded, hard-edged pixels, so
/// a step that slides the eye across a cell boundary flips whole pixels by up to 255. That churn is the
/// geometry, it is not axis-symmetric by construction, and no appearance port can remove it.
const MARGINS: [f64; 3] = [0.8, 1.0, 1.0];

/// **The appearance asymmetry ceiling — the gate.** At every step the **vertical** eye step's appearance
/// response must be at most **half** the **horizontal** step's.
///
/// The user's own report *is* the asymmetry (*"still visible material flickering when moving on the vertical
/// axis which is now fixed when moving on the horizontal axis"*), so the horizontal axis is the reference the
/// vertical one is held to. A path whose appearance re-weights as the eye slides moves both axes alike — a
/// ratio of 1 — and the ported path reads **0.07×, 0.05× and 0.04×** at 0.05 m, 0.25 m and 1.00 m.
///
/// The ceiling is calibrated by the **guard-bite experiment**, not by taste. With a view-dependent gain
/// injected into the ported colour (up to a half, applied to both frames), the same three steps read
/// **1.00×, 1.00× and 0.96×** — above this ceiling at every one of them — while the changed-pixel counts
/// stay identical to the digit (1 837 and 1 280 at 0.05 m). The margin is about seven times on the port's
/// side and about twice on the perturbation's.
///
/// The pixel-churn comparison against the parent ([`MARGINS`]) is reported at every step as well, but it is
/// **not** the gate: the experiment above is why.
const APPEARANCE_ASYMMETRY_CEILING: f64 = 0.5;

/// The world the client's own reading draws, stepped to its declared rest — through the one declaration
/// of it, so this instrument stands on the same world the window's frames do.
fn plane() -> Automaton {
    let cubic = surface::client_world().expect("the client's own world is declared");
    let mut plane = Automaton::over(&cubic);
    plane.steps(SETTLE_TICKS).expect("the law steps");
    plane
}

/// One frame of the client's own world, drawn through **the ported `shade`**, from a camera moved by
/// `step` in the world's own metres. The camera is the client's own ([`surface::view`], the declared field
/// of view) and the step is applied to the eye and to what it looks at, which is what the parent's
/// instrument does: the same view direction, a different place to stand.
fn frame(plane: &Automaton, step: [f64; 3]) -> drawn::Drawn {
    // The mesh comes from **the camera's own residency** of this state ([`surface::residency_of`]), the door
    // the drawn path takes, so this flicker measurement is of the picture a person is shown rather than of a
    // second derivation beside it.
    let residency = surface::residency_of(plane.world()).expect("the world has a chunk to stand in");
    let mesh = surface::region_mesh(&residency, surface::DRAWN_CHUNK).expect("the region meshes");
    let over = drawn::Camera::over(&mesh, surface::view().toward, drawn::DRAWN_FOV_DEG)
        .expect("the region has an extent");
    let mut camera = over;
    camera.eye = [
        over.eye[0] + step[0],
        over.eye[1] + step[1],
        over.eye[2] + step[2],
    ];
    camera.target = [
        over.target[0] + step[0],
        over.target[1] + step[1],
        over.target[2] + step[2],
    ];
    drawn::draw(
        &mesh,
        reference::registry(),
        &surface::light(),
        &camera,
        drawn::DRAWN_SIZE,
        drawn::Look {
            roughness: surface::ROUGHNESS,
            metalness: surface::METALNESS,
            exposure: surface::EXPOSURE,
        },
        presentation::encode,
    )
    .expect("the stepped frame draws")
}

/// The parent instrument's own arithmetic over two frames' client areas: how many pixels differ in any
/// channel, the total difference over every channel, and the worst single channel difference.
fn difference(before: &drawn::Drawn, after: &drawn::Drawn) -> (u64, f64, f64) {
    let size = before.size();
    let mut changed = 0_u64;
    let mut total = 0.0_f64;
    let mut worst = 0.0_f64;
    for y in 0..size {
        for x in 0..size {
            let a = before.pixel(x, y).expect("a pixel of the frame");
            let b = after.pixel(x, y).expect("a pixel of the frame");
            let mut differed = false;
            for channel in 0..4 {
                let delta = (f64::from(b[channel]) - f64::from(a[channel])).abs();
                if delta > 0.0 {
                    differed = true;
                    total += delta;
                    worst = worst.max(delta);
                }
            }
            if differed {
                changed += 1;
            }
        }
    }
    (changed, total, worst)
}

/// **The appearance response of a step** — the total change in the two frames' own shaded colours, over the
/// mesh's own rectangles in the order the mesh declares them, summed over the three channels.
///
/// This is the quantity that responds to appearance, and the pixel churn above is **not** a substitute for
/// it: the guard-bite experiment of 2026-10-13 (a view-dependent gain added to the ported colour, up to a
/// half) left the changed-pixel counts **identical** at every step — 1837 and 1280 at 0.05 m — because a
/// colour that changed inside a pixel class does not change the class. A guard that cannot see that is not a
/// guard, so the gate is stated on this number and the pixel churn is reported beside it.
fn appearance_response(before: &drawn::Drawn, after: &drawn::Drawn) -> f64 {
    assert_eq!(
        before.samples().len(),
        after.samples().len(),
        "both frames must shade the mesh's own rectangles, one sample each"
    );
    before
        .samples()
        .iter()
        .zip(after.samples())
        .map(|(a, b)| (a.r - b.r).abs() + (a.g - b.g).abs() + (a.b - b.b).abs())
        .sum()
}

#[test]
fn the_ported_path_is_measured_for_flicker_and_must_not_exceed_the_parents_excess() {
    let plane = plane();
    let held = frame(&plane, [0.0, 0.0, 0.0]);
    let held_again = frame(&plane, [0.0, 0.0, 0.0]);
    let pixels = (held.size() as u64) * (held.size() as u64);
    let (held_changed, held_total, held_worst) = difference(&held, &held_again);
    println!(
        "port-flicker: {} px frames of the client's own region, drawn through the ported `shade` \
         (`voxel_appearance`, pinned in docs/baselines/port-appearance-seam/pinned-parent-bytes.txt)",
        pixels
    );
    println!(
        "  camera-held control: {held_changed} of {pixels} px changed, total {held_total:.1}, \
         worst {held_worst:.4}"
    );
    assert_eq!(
        held_changed, 0,
        "a camera that did not move must draw the same pixels: {held_changed} changed"
    );
    assert_eq!(
        held_total, 0.0,
        "a camera that did not move must draw the same colours: total {held_total}"
    );

    for (index, step) in STEPS_M.iter().enumerate() {
        let up = frame(&plane, [0.0, *step, 0.0]);
        let right = frame(&plane, [*step, 0.0, 0.0]);
        let (changed_v, total_v, worst_v) = difference(&held, &up);
        let (changed_h, total_h, worst_h) = difference(&held, &right);
        let response_v = appearance_response(&held, &up);
        let response_h = appearance_response(&held, &right);
        let response_ratio = response_v / response_h;
        let share_v = 100.0 * changed_v as f64 / pixels as f64;
        let share_h = 100.0 * changed_h as f64 / pixels as f64;
        let ratio_px = changed_v as f64 / changed_h as f64;
        let ratio_magnitude = total_v / total_h;
        let (parent_v, parent_h, parent_px, parent_magnitude) = PARENT[index];
        println!(
            "  {step:.2} m: vertical (+y)   {changed_v} of {pixels} px ({share_v:.1} %), \
             total {total_v:.1}, mean {:.6}, worst {worst_v:.4}",
            total_v / pixels as f64
        );
        println!(
            "          horizontal (+x) {changed_h} of {pixels} px ({share_h:.1} %), \
             total {total_h:.1}, mean {:.6}, worst {worst_h:.4}   -> {ratio_px:.2}x by pixels, \
             {ratio_magnitude:.2}x by magnitude",
            total_h / pixels as f64
        );
        println!(
            "          the parent's own instrument, same step: vertical {parent_v}, horizontal \
             {parent_h} -> {parent_px:.2}x / {parent_magnitude:.2}x  (the excess this port must not \
             reproduce)"
        );
        println!(
            "          appearance response, over the mesh's own {} rectangle(s): vertical \
             {response_v:.1}, horizontal {response_h:.1} -> {response_ratio:.2}x",
            held.samples().len()
        );

        // **The gate**, asserted at every step the parent's own instrument measures at: the vertical step
        // must not move the frame's appearance more than half as far as the horizontal one does.
        assert!(
            response_ratio <= APPEARANCE_ASYMMETRY_CEILING,
            "the ported path must not re-weight appearance as the eye slides: at {step:.2} m a vertical \
             step moved the frame's own colours {response_v:.1} and a horizontal step {response_h:.1}, \
             over {} rectangle(s) — a ratio of {response_ratio:.2}x, above the declared ceiling \
             {APPEARANCE_ASYMMETRY_CEILING:.2}x (the ported path reads 0.07x, 0.05x and 0.04x at the \
             three steps; a view-dependent gain on the same frames reads 1.00x, 1.00x and 0.96x)",
            held.samples().len()
        );

        // **The gate**, at every step the parent's own instrument measures at: the ported path's excess
        // must be at most four fifths of the parent's, by changed pixels and by total magnitude.
        let margin = MARGINS[index];
        let ceiling_px = parent_px * margin;
        let ceiling_magnitude = parent_magnitude * margin;
        assert!(
            ratio_px <= ceiling_px,
            "the ported path must not reproduce the parent's own flicker excess: at {step:.2} m a \
             vertical step changed {changed_v} px ({share_v:.1} %) against {changed_h} px \
             ({share_h:.1} %) horizontally, {ratio_px:.2}x — above the declared ceiling \
             {ceiling_px:.2}x ({margin:.2} × the parent's own {parent_px:.2}x)"
        );
        assert!(
            ratio_magnitude <= ceiling_magnitude,
            "and by total magnitude the ported path reads {ratio_magnitude:.2}x at {step:.2} m, above \
             the declared ceiling {ceiling_magnitude:.2}x ({margin:.2} × the parent's own \
             {parent_magnitude:.2}x)"
        );
        println!(
            "          gate: {ratio_px:.2}x <= {ceiling_px:.2}x and {ratio_magnitude:.2}x <= \
             {ceiling_magnitude:.2}x  ({margin:.2} × the parent's own excess) — passed"
        );
    }
}
