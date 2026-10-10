//! The material's own pattern, as a fragment of it is drawn: the address the
//! fragment has inside its block, and the deviation the 256-level lattice gives it
//! at the rungs a pixel can resolve.
//!
//! # Why a pattern at all, and why it is not geometry
//!
//! The greedy mesher merges a flat region of one material into one rectangle
//! (C-17). That is what makes a chunk cheap to draw, and it is also what removes
//! every per-block detail from the picture: a merged quad carries one material and
//! no coordinates, so the block grid it spans is invisible. The appearance layer's
//! 256× detail lattice is **addressed, never stored** (C-2) and is exactly the
//! thing that can be read per fragment from the fragment's own position — the
//! material slot's own pattern, drawn **without one extra quad** (C-122, C-91).
//!
//! # The octave ladder
//!
//! `APPEARANCE.md` §2 gives a material two resolutions: the **256** detail lattice —
//! 16 samples per subvoxel axis, a 15.6 mm cell, the base named 256 because a
//! subvoxel *face* carries 16×16 samples — and the 4×4×4 **subvoxel** grid (a 0.25 m
//! cell). One addressed field can be read at either: quantising the lattice address
//! to a multiple of `2^j` per axis reads the same hash in cells of `15.6 mm · 2^j`,
//! and past the subvoxel rung the **world cell** is what coarsens, because a feature
//! bigger than a subvoxel cannot be addressed inside one.
//!
//! So a fragment's pattern is the field read **at the scale its own footprint can
//! draw** — octave `j` for `j = `[`OCTAVE_FLOOR`]`..=`[`OCTAVE_TOP`], from
//! [`MIN_FEATURE_M`] (31.25 mm — **32 samples per block axis**, the scale the user's
//! own eye settled on) up to [`MAX_FEATURE_M`] (4 m), each shaped by the material's
//! shaper at the octave's own cell — and the octaves are **summed with fBm gains and
//! normalised**, which is the cross-fade the eye-check liked (*"I love the way it has
//! an LOD style appearance to it"*). The bounds are **constraints**, declared and
//! tested tight: the addressed base's own cell (15.6 mm) is *never drawn* — the user
//! asked for nothing below 32 per block (*"if there's stuff happening below that, then
//! I don't want it"*, C-164) — and nothing at all is drawn past the top rung.
//!
//! # The blend stays; its *input* had to change (the third eye-check, C-163)
//!
//! A blend of independent rungs is a function of **(position, footprint)**, and the
//! first footprint this code used was `fwidth` — a *derivative*, estimated per 2×2
//! quad. On any surface the camera sees at an angle the estimate steps from quad to
//! quad, the weights move with it, the drawn value changes with them, and the face
//! *boils*: the user's *"flickering mess over the blocks"*, and *"it doesn't flicker
//! when the camera is still but the patterns are still there"* — a per-quad mix is a
//! static noise field that only a moving footprint animates.
//!
//! The fix is not to drop the fade but to feed it a **smooth** quantity. The shader
//! now derives its footprint from the eye's own geometry — distance × one pixel's
//! angle ÷ cos(face), `shading.wgsl`'s `eye_footprint` — which varies smoothly across
//! a face, so the weights do too and the boil is gone (C-163).
//!
//! # This file is the model; `shading.wgsl` is its transcription
//!
//! The shader computes the same address and the same deviation, in WGSL, in `f32`,
//! with the lattice's hash written a second time (WGSL has no `u64`, so the shader
//! carries splitmix64 in `u32` pairs). That duplication is an **obligation with a
//! test on it**:
//! `renderer::tests::the_pattern_is_drawn_where_the_greedy_mesh_has_no_geometry`
//! renders a stated surface and compares every pixel against this file's `f64`
//! answer, so the day the two disagree the test says so rather than the picture.
//!
//! # The corrections the eye-check forced (C-162)
//!
//! **The address is the world's cell, not the block's index.** A block-local
//! address is the same number in every block, so every block drew the same
//! pattern: a one-metre stamp the eye reads as tiling (the user's *"the tiling is
//! very noticeable"*). The cell is now `floor(p / 0.25 m)`, which is C-91's own
//! ruling read one level down — the pattern is an instance field, and the instance
//! is the world subvoxel.
//!
//! **White noise is not yet a pattern, and now it is shaped.** The user's ruling:
//! *"white noise at the 256 base is acceptable with different shaper patterns for
//! the noise, so for sand it'd be speckled, for trees it'd be bark, metal could be
//! brushed/shiny."* The shaper is
//! [`voxel_shapers::column::materials::SurfacePattern`] — a stencil on the address
//! and a curve on the value — and it is the same hash underneath, so nothing is
//! stored.
//!
//! **A placeholder tone was invented at the 64 level, and it is retired (C-163).**
//! The two-rate form read the subvoxel's own centre as a coarse field capped at a
//! declared share of the gain, and the eye-check's second look read it back as *"the
//! 256 subvoxels … still extremely, like buggy"*: quarter-metre cells of independent
//! white noise are a patchwork, not composition. C-122 §122.1 makes the 64 level the
//! library's **stored slots** (`SlotWriter`), so the invented tone is deleted and the
//! **ladder** below replaces it; `MATERIALS.md` 4.9 stays the owed content.
//!
//! # What this does not do
//!
//! The **library's own base patterns** — the structured motifs C-122 calls "the
//! material slots that allow for per block details" — are still owed content
//! (`MATERIALS.md` 4.9/4.10). What exists here is the record's own mechanism: the
//! lattice's mean-zero deviation, shaped per material role, scaled by the
//! material's declared gains ([`voxel_model::ShadingField`]).
//!
//! The **fluid half of the 256 level** is the named next door, and the user's
//! framing of it is the spec (2026-10-05): *"animating water at the 256 base and
//! having actual water cells interpolating at the 256 base so molten metals and
//! fluids/gas can animate addressed voxels at the 256 base while also still
//! allowing for the non-addressed materials properties at 256x base — it might be
//! easier to frame it similar to minecraft water shaders: it allows for sea foam
//! on the material surface, caustics through the materials, and when we need the
//! material to break out of the block, it looks good."*
//!
//! So the 256 level is a **live addressed domain**: a fluid's cells are real
//! values *interpolated* between addresses and read as a pure function of
//! `(tick, address)` (C-91.3), which is what gives foam on the surface, caustics
//! through the body, and a shaded edge where a material leaves its block — while
//! a solid's non-addressed properties stay exactly the static read they are now.
//! Nothing in this file reads a tick yet, and nothing needs to: an added argument
//! is a pure function's new axis, and that axis is declared here rather than
//! pre-built, so the fluid pass opens the door instead of refactoring around it.

use voxel_appearance::{DetailLattice, DetailSample};
use voxel_model::{
    DetailIndex, MaterialId, Refusal, RefusalKind, Refused, ShadingField, DETAIL_SIDE,
    SUBVOXEL_SIDE,
};
use voxel_shapers::column::materials::SurfacePattern;

/// One block edge, in metres. The generator's own scale (`C-6`: 1 m voxels).
const VOXEL_M: f64 = 1.0;

/// One subvoxel cell across, in metres — the **64** of "the 64 base".
pub const SUBVOXEL_CELL_M: f64 = VOXEL_M / SUBVOXEL_SIDE as f64;

/// One lattice cell across, in metres — the **256** of "the 256 base", per
/// subvoxel axis.
pub const DETAIL_CELL_M: f64 = SUBVOXEL_CELL_M / DETAIL_SIDE as f64;

/// The finest rung the ladder draws: octave 1, **31.25 mm** — 32 samples per block
/// axis — which is the scale the user's own eye settled on:
/// *"I don't want blocks to actually have 256x pixels detail, I like the 32x per side
/// of block, so if there's stuff happening below that, then I don't want it."*
///
/// The addressed base's own cell ([`DETAIL_CELL_M`], 15.6 mm, octave 0) is still what
/// the lattice is *addressed* with and is **never drawn**: the ladder starts one rung
/// above it, so no scale below the one the eye asked for can appear.
pub const OCTAVE_FLOOR: u32 = 1;

/// The smallest feature the ladder draws: octave [`OCTAVE_FLOOR`]'s cell, 31.25 mm.
pub const MIN_FEATURE_M: f64 = DETAIL_CELL_M * (1u64 << OCTAVE_FLOOR) as f64;

/// The largest feature the ladder draws: 32 m. Past it the field is silent — a
/// surface that far away has a 32 m feature under two pixels, where a pattern can
/// only be guessed, and the honest answer is the average.
///
/// Extended from 4 m to stop the moire the vertex detail produced when the ladder
/// went silent past 4 m. The vertex detail is one lattice sample per corner,
/// interpolated across a 32 m merged quad — a 15.6 mm signal sampled four times
/// over 32 m, which reads as moire. With the ladder drawing to 32 m, the vertex
/// detail is only a fallback past ~64 km, where the quad is sub-pixel anyway.
pub const MAX_FEATURE_M: f64 = 32.0;

/// The coarsest rung: octave 11 is 2048 base cells per axis, 32 m.
///
/// The ladder used to stop at octave 8 (4 m), and past its own floor the shader
/// fell back to the per-vertex detail — one lattice sample per corner, interpolated
/// across a 32 m merged quad. That is a 15.6 mm signal sampled four times over
/// 32 m, which reads as moire. Extending the ladder to 32 m means the vertex
/// detail is only a fallback past ~64 km, where the quad is sub-pixel anyway.
pub const OCTAVE_TOP: u32 = 11;

/// The octave of the subvoxel itself: `2^4 = 16` base cells = 0.25 m. Octaves above
/// it have subvoxel-sized and larger features, so their address is the **world
/// cell** coarsened — the composition base the library's stored slots sit at
/// (C-122) and the rung the ladder's cell quantisation starts from.
pub const SUBVOXEL_OCTAVE: u32 = 4;

/// What each coarser rung weighs against the one below it. The ladder is an fBm:
/// fine detail carries the texture, and coarser rungs add their structure at half the
/// amplitude, which is what keeps the composition from becoming one scale's pattern at
/// the expense of the others.
pub const OCTAVE_GAIN: f64 = 0.5;

/// One rung's cell across, in metres: `DETAIL_CELL_M · 2^octave`, so octave 0 is the
/// addressed base's cell (15.6 mm, never drawn), [`OCTAVE_FLOOR`] is
/// [`MIN_FEATURE_M`] (31.25 mm) and [`OCTAVE_TOP`] is 4 m.
pub fn octave_cell_m(octave: u32) -> f64 {
    DETAIL_CELL_M * (1u64 << octave) as f64
}

/// The coarse cell's span in subvoxels per axis, per rung: `2^(octave − SUBVOXEL_OCTAVE)`.
pub fn octave_span(octave: u32) -> i32 {
    1i32 << octave.saturating_sub(SUBVOXEL_OCTAVE)
}

/// **The smallest span worth pre-filtering**, in subvoxels per axis.
///
/// Below this a rung's coarse cell is one or two subvoxels across, and the blend
/// between its neighbours already varies smoothly — there is nothing for an
/// average to fix, and paying eight lattice reads to average four would be a cost
/// with no return.
pub const PREFILTER_SPAN: i32 = 2;

/// Where inside its own coarse cell a pre-filtered rung samples, per axis.
///
/// **Centres, not corners**: `span/4` and `3·span/4` are the centres of the eight
/// equal sub-cells, so the average is over what the rung actually stands for.
pub fn prefilter_offsets(span: i32) -> [i32; 2] {
    [span / 4, 3 * span / 4]
}

/// How sharply a rung's weight moves, as a power on the fade.
///
/// **One is the plain smoothstep and one is what ships.** A larger value was measured
/// (`examples/pattern-flicker`, 2026-10-08) and it is **worse on every number**: at width 4 the
/// vertical total rose 473.4 → 498.3, the changed-pixel count rose 14,155 → 18,147, and the
/// horizontal rose with them. Widening the plateau keeps *more* rungs sitting at nearly full
/// weight at once, so more addresses compete in the mean and a rung entering or leaving moves it
/// further than it did when the fade was spread across the range.
///
/// So the lever that made the blend calmer was not this. "Wider" as C-171 meant it is **more
/// rungs overlapping at comparable weight**, which is [`OCTAVE_GAIN`] — the fBm weighting — and
/// not the shape of any one rung's fade. That is the untested candidate, and this constant is
/// kept at one because the number above is the number it ships.
pub const OCTAVE_FADE_WIDTH: f64 = 1.0;

/// How much of one rung a pixel can draw: **one** while its footprint is well below
/// the rung's cell, falling to **zero at a quarter of the cell** — a stricter floor
/// than Nyquist's half, and a smoothstep rather than a straight line.
///
/// Two eye-checks set this. *Nyquist's half* (C-163) stopped the streaks, but a cell
/// two pixels wide is still hard-edged white noise; four pixels a cell is where a
/// hard cell stops reading as grain and starts reading as a speck, and the smoothstep
/// makes the weight's derivative zero at both ends, so a rung fades in and out
/// without a step at either edge.
///
/// The width is [`OCTAVE_FADE_WIDTH`]: the fade keeps its shape and its floor, and gains a
/// plateau in front of them.
pub fn octave_weight(footprint_m: f64, cell_m: f64) -> f64 {
    let t = (1.0 - 4.0 * footprint_m / cell_m).clamp(0.0, 1.0);
    let t = 1.0 - (1.0 - t).powf(OCTAVE_FADE_WIDTH);
    t * t * (3.0 - 2.0 * t)
}

/// [`octave_weight`] fed a **mip-staged** footprint: the footprint is raised to the
/// rung's own Nyquist floor — `cell_m / 4` exactly, the point where the plain
/// weight first reaches 1 — and the weight is then the plain weight of that
/// **whole-or-nothing stage**.
///
/// Why: the C-171 A/B measured the vertical shimmer's asymmetry living in this
/// function's smoothstep — a vertical step re-weights every rung on the horizontal
/// faces that fill the view. Under staging the smoothstep has a single drawn
/// point: a rung is either **fully on** (footprint strictly below `cell/4`) or
/// **fully off** (at or above it — the plain weight is already exactly 0 at the
/// floor, so the step and the smoothstep agree at the boundary), and the weight
/// moves only when the eye's step pushes the footprint across that one
/// threshold.
///
/// The tests held both halves of the reason for the exact threshold. A *floored*
/// footprint is smaller than the true one and raises rung weights — the ladder's
/// silence past the ceiling broke (the GPU agreement test). A *ceiling to
/// quarter-cells or to powers of two* is strictly larger than `cell/4` whenever it
/// changes anything, where the plain weight is already below 1 and collapsing
/// toward 0 — so staging blanked rungs the plain weight draws (at 3.9 mm against
/// the 31.25 mm rung's floor the plain weight is 0.5; a power-of-two ceiling
/// stages to `cell/4 · 2`, where the weight is 0). The exact-floor stage keeps
/// every silence (`staged ≥ cell/4 ⇒ weight 0`) and draws wherever the plain
/// weight draws (`staged = cell/4 ⇒ weight 1`); the price is that the fade is now
/// a step per rung, not a ramp — the C-163 boil is gone and this is deliberate:
/// the boil was a *derivative* artifact (per 2×2 quad), a staged step is a
/// *footprint* threshold and the footprint is smooth, so a rung flips as a
/// coherent region crossing the eye's distance, not as per-quad noise.
pub fn octave_weight_staged(footprint_m: f64, cell_m: f64) -> f64 {
    let floor = cell_m / 4.0;
    if footprint_m < floor {
        1.0
    } else {
        0.0
    }
}

/// The floor on a face's cosine, so a surface seen nearly edge-on does not divide
/// its footprint by zero.
///
/// Declared here rather than clamped only in the shader (`shading.wgsl`'s
/// `FACE_COS_FLOOR` is this number) so the behaviour at grazing angles is a stated
/// choice: the stretch is bounded at `1/0.05 = 20×`, the coarser rungs answer at a
/// larger cell, and a floor or wall the eye skims goes quiet rather than aliasing.
pub const FACE_COS_FLOOR: f64 = 0.05;

/// **One pixel's angular size**, in radians: the camera's vertical field of view over
/// the render target's own pixel height.
///
/// The ladder needs it because a footprint is a *view* quantity — the same surface at
/// the same distance has a smaller footprint in a taller frame — and a fragment
/// cannot derive it: WGSL carries the field of view only inside the projection
/// matrix, and a pixel has no world-space size until a target states one. So the
/// renderer computes it and uploads it with the camera (`View::screen.x`).
pub fn pixel_angle_rad(fov_y_rad: f64, height_px: u32) -> f32 {
    (2.0 * (fov_y_rad / 2.0).tan() / height_px as f64) as f32
}

/// **One pixel's footprint on a surface**, in metres — the shader's `eye_footprint`,
/// and the number [`Pattern::deviation`] is asked about.
///
/// `distance_m` is the fragment's own distance from the eye, `pixel_angle_rad` the
/// frame's own pixel angle ([`pixel_angle_rad`]), and `face_cos` the cosine between
/// the face's normal and the direction to the eye. The footprint is the pixel's
/// angular width carried out to that distance — `distance × angle` — stretched by the
/// obliquity `1 / cos`, floored by [`FACE_COS_FLOOR`].
///
/// This replaces `fwidth`, and the reason is the whole of C-163: a screen-space
/// derivative is estimated per 2×2 quad, so on a face the camera sees at an angle the
/// estimate steps from quad to quad, the ladder's weights step with it, and the face
/// **boils** as the camera moves — the eyepiece's *"flickering mess over the blocks"*.
/// Every input here is the fragment's own state, so the footprint varies smoothly
/// across a face and the fade between rungs survives.
pub fn eye_footprint_m(distance_m: f64, pixel_angle_rad: f64, face_cos: f64) -> f64 {
    distance_m * pixel_angle_rad / face_cos.abs().max(FACE_COS_FLOOR)
}

/// [`octave_weight`] with the footprint **staged** to quarter-cell ceilings
/// (`octave_weight_staged`): C-171's repair, route (a).
/// same lattice in cells that many times coarser. It is applied **after** the
/// shaper's stencil (C-164): the stencil shifts the stretched axis into a coarser
/// address space and the rung's mask then coarsens it further, so the shaper's
/// character survives at every rung rather than only at the floor — the order is
/// load-bearing and both sides state it. At and above [`SUBVOXEL_OCTAVE`] every
/// address inside a subvoxel quantises to the same one, which is what makes those
/// rungs subvoxel-featured.
pub fn quantise(at: DetailIndex, octave: u32) -> DetailIndex {
    let mask = if octave >= SUBVOXEL_OCTAVE { 0u16 } else { !((1u16 << octave) - 1) };
    DetailIndex { x: at.x & mask, y: at.y & mask, z: at.z & mask }
}

/// A **world cell** quantised to a multiple of `2^(octave − SUBVOXEL_OCTAVE)` per
/// axis — the rungs past the subvoxel, whose features are bigger than one. Two's
/// complement masking is floor-quantisation, so negative cells answer correctly
/// and a rung's cells tile the world without a seam at the origin.
pub fn quantise_cell(cell: [i32; 3], octave: u32) -> [i32; 3] {
    if octave <= SUBVOXEL_OCTAVE {
        return cell;
    }
    let mask = !((1i32 << (octave - SUBVOXEL_OCTAVE)) - 1);
    [cell[0] & mask, cell[1] & mask, cell[2] & mask]
}

/// The fraction inside one cell `side` samples across, as the shader computes it.
///
/// `floor(v * side)` is clamped to the last sample: the lattice is a grid of
/// samples and not a half-open range (`DetailIndex::from_unit` says the same), and
/// in `f32` a value one rounding step below an integer can still round *onto* it.
/// Clamping rather than wrapping is the honest fix — the shader has no refusal
/// channel — and the model's refusal stays where it is, at [`DetailIndex::new`].
fn cell_of(fraction: f64, side: u32) -> (u32, f64) {
    let scaled = fraction * side as f64;
    let whole = scaled.floor().min((side - 1) as f64);
    (whole as u32, scaled - whole)
}

/// Where a fragment sits in its material's pattern: the **world subvoxel** it
/// belongs to and its address in the 256 lattice within that cell.
///
/// `p_m` is the fragment's world position, in `f64` as the renderer holds it. The
/// cell is `floor(p / 0.25 m)` per axis — the world's own cell, not the subvoxel's
/// index inside its block, because a block-local address is the same number in
/// every block and stamps the same pattern across the world (C-162). The shader
/// computes the same cell from the eye's two halves (`eye_phase` + `eye_voxel`);
/// the test that holds the two forms together is the GPU one.
pub fn address_at(p_m: [f64; 3]) -> Refused<([i32; 3], DetailIndex)> {
    check_finite(p_m)?;
    let mut cell = [0i32; 3];
    let mut at = [0u16; 3];
    for axis in 0..3 {
        let g = p_m[axis] / SUBVOXEL_CELL_M;
        let whole = g.floor();
        if !(i32::MIN as f64..=i32::MAX as f64).contains(&whole) {
            return Err(Refusal::owned(
                RefusalKind::CoordinateOutOfRange,
                format!("subvoxel cell {whole} at axis {axis} does not fit i32"),
            ));
        }
        cell[axis] = whole as i32;
        let (a, _) = cell_of(g - whole, DETAIL_SIDE);
        at[axis] = a as u16;
    }
    Ok((cell, DetailIndex::new(at[0], at[1], at[2])?))
}

/// Whether a point's address is far enough from a cell boundary that the shader's
/// `f32` arithmetic cannot land in the neighbouring cell.
///
/// The address axes are `f32` on the GPU and `f64` here, so a fragment whose
/// fraction sits within a rounding step of a boundary may be assigned to either
/// cell. Only axes the fragment actually moves along are checked: an axis whose
/// footprint is zero sits at one address for the whole probe, and both sides land
/// on the same side of a boundary there — a plane at a whole metre is Level, not
/// ambiguous. `footprint_m` is one pixel step per axis, and `margin` is a fraction
/// of one cell of the **finest** rate; a GPU test excludes unsettled pixels and
/// *prints how many*, rather than loosening a tolerance until the difference hides.
pub fn address_is_settled(p_m: [f64; 3], footprint_m: [f64; 3], margin: f64) -> bool {
    for axis in 0..3 {
        if footprint_m[axis] <= 0.0 {
            continue;
        }
        let scaled = p_m[axis] / DETAIL_CELL_M;
        let frac = scaled - scaled.floor();
        // One check covers both boundaries: a fine cell's edge changes the
        // address, and a subvoxel's own edge (every 256 fine cells) changes the
        // lattice domain — both are `frac` near zero.
        if frac < margin || frac > 1.0 - margin {
            return false;
        }
    }
    true
}

/// The eye as the shader needs it, **first half**: its fractional metre offset,
/// per axis.
///
/// The shader adds this to the eye-relative vertex position, and the result is
/// the fragment's position relative to the eye's own *voxel*: `fract(position +
/// phase)` is `fract(p_world)` because the phase differs from the eye by a whole
/// number of metres, and the integer part is exactly what `fract` discards.
pub fn eye_phase(eye_m: [f64; 3]) -> [f32; 3] {
    let mut out = [0.0f32; 3];
    for axis in 0..3 {
        out[axis] = (eye_m[axis] - eye_m[axis].floor()) as f32;
    }
    out
}

/// The eye as the shader needs it, **second half**: the world voxel it is in,
/// `floor(eye)` per axis, as `view.eye_voxel` carries it.
///
/// The fragment rebuilds its world subvoxel cell from the two halves:
/// `cell = floor((p_rel + phase) * 4) + eye_voxel * 4`, which is `floor(p_world / 0.25)`
/// for every position a signed 32-bit world can hold (C-6). Saturating rather than
/// refusing: a camera outside ±2³¹ metres is already outside the coordinate
/// contract, and the projection would have refused the frame first.
pub fn eye_voxel(eye_m: [f64; 3]) -> [i32; 3] {
    let mut out = [0i32; 3];
    for axis in 0..3 {
        out[axis] = eye_m[axis].floor().clamp(i32::MIN as f64, i32::MAX as f64) as i32;
    }
    out
}



/// The pattern a world's field and lattice make of a material.
#[derive(Clone, Copy, Debug)]
pub struct Pattern<'a> {
    lattice: &'a DetailLattice,
    field: &'a ShadingField,
}

impl<'a> Pattern<'a> {
    pub fn new(lattice: &'a DetailLattice, field: &'a ShadingField) -> Self {
        Pattern { lattice, field }
    }

    /// The lattice this pattern addresses.
    pub fn lattice(&self) -> &DetailLattice {
        self.lattice
    }

    /// The deviation a fragment of `material` at `(cell, at)` shows, with the
    /// material's own gains applied and its **shaper** shaping the noise — exactly
    /// what the shader computes before it modulates the row.
    ///
    /// `surface` is the material's declared shaper
    /// ([`voxel_shapers::column::materials::pattern_of`]), which the shader reads
    /// from the row's `pattern.z`. A material the field does not vary returns
    /// [`DetailSample::NONE`] **exactly** and without touching the lattice: "not
    /// varied at this level at all" is a different claim from "varied by zero",
    /// and the cheap path is the honest one.
    pub fn deviation(
        &self,
        material: MaterialId,
        cell: [i32; 3],
        at: DetailIndex,
        footprint_m: f64,
        surface: SurfacePattern,
    ) -> Refused<DetailSample> {
        let Some((roughness_gain, reflectance_gain)) = self.field.gain(material) else {
            return Ok(DetailSample::NONE);
        };
        // The ladder: the same field read in cells of `2^octave`, weighted by what
        // the pixel can draw and by the fBm gain, then **normalised** — so the
        // composition is a weighted mean of values in `[-1, 1]` and the deviation
        // stays inside the range `with_detail` promises, at every footprint (C-163).
        let mut weight_sum = 0.0f64;
        let mut roughness = 0.0f64;
        let mut reflectance = 0.0f64;
        let mut gain = 1.0f64;
        for octave in OCTAVE_FLOOR..=OCTAVE_TOP {
            let weight = octave_weight(footprint_m, octave_cell_m(octave));
            if weight > 0.0 {
                // **A coarse rung is the mean of the grains it stands for.** Point
                // sampling one address inside the cell is what makes a rung swap
                // wholesale as the footprint crosses into it — the blend then
                // shifts between decorrelated values and the surface reads as
                // re-rolling rather than settling. Averaging over the cell makes
                // neighbouring coarse cells agree, so crossing one is a small
                // change instead of a new pattern.
                let span = octave_span(octave);
                // One offset when the rung is not pre-filtered, two when it is — so an
                // unfiltered rung reads the lattice once, as it always did.
                let offsets: Vec<i32> = if span >= PREFILTER_SPAN {
                    prefilter_offsets(span).to_vec()
                } else {
                    vec![0]
                };
                let at_rung = quantise(surface.stencil(at), octave);
                // **The offsets are measured from the rung's own coarse cell, not from
                // `cell`.** A fragment sits wherever it sits inside that cell, so
                // offsetting from it walks into the neighbouring cells and slides the
                // average's centre as the fragment moves — which is the re-rolling this
                // pass exists to remove. Measured from the origin, the average is over
                // one cell and is constant across it, as an area average must be.
                let origin = quantise_cell(cell, octave);
                let (mut taps_r, mut taps_f, mut taps) = (0.0f64, 0.0f64, 0usize);
                for &du in &offsets {
                    for &dv in &offsets {
                        for &dw in &offsets {
                            let sample = self.lattice.sample(
                                material,
                                [origin[0] + du, origin[1] + dv, origin[2] + dw],
                                at_rung,
                            )?;
                            taps_r += surface.curve(sample.roughness_delta);
                            taps_f += surface.curve(sample.reflectance_delta);
                            taps += 1;
                        }
                    }
                }
                roughness += taps_r / taps as f64 * weight * gain;
                reflectance += taps_f / taps as f64 * weight * gain;
                weight_sum += weight * gain;
            }
            gain *= OCTAVE_GAIN;
        }
        if weight_sum <= 0.0 {
            // No rung is drawable at this footprint: the field has nothing to say
            // here, which is a different claim from "varied by zero" — the same
            // distinction `DetailSample::NONE` already carries.
            return Ok(DetailSample::NONE);
        }
        Ok(DetailSample {
            roughness_delta: roughness / weight_sum * roughness_gain,
            reflectance_delta: reflectance / weight_sum * reflectance_gain,
        })
    }

    /// [`Pattern::deviation`] from a world position, with the address derived
    /// first — the whole of what a fragment's pattern is.
    pub fn at(
        &self,
        material: MaterialId,
        p_m: [f64; 3],
        footprint_m: f64,
        surface: SurfacePattern,
    ) -> Refused<DetailSample> {
        let (cell, at) = address_at(p_m)?;
        self.deviation(material, cell, at, footprint_m, surface)
    }
}

/// The lattice's flat address, as `voxel-appearance::detail` flattens it: `x` is
/// the fast axis, and the axes are packed at the side's own bit width — four bits
/// for the 16-per-axis base, which is what makes `(16, 0, 0)` and `(0, 1, 0)` the
/// same address, the aliasing `DetailIndex::new` refuses.
pub fn flat_address(at: DetailIndex) -> u32 {
    let shift = DETAIL_SIDE.trailing_zeros();
    at.x as u32 | ((at.y as u32) << shift) | ((at.z as u32) << (2 * shift))
}


/// A refusal for a point the pattern cannot be asked about — a non-finite world
/// position. Refused rather than defaulted, the same rule as everywhere else: a
/// `NaN` cast to an index silently lands on cell zero, which is a claim about a
/// block nobody asked about.
fn check_finite(p_m: [f64; 3]) -> Refused<()> {
    for (axis, v) in p_m.iter().enumerate() {
        if !v.is_finite() {
            return Err(Refusal::owned(
                RefusalKind::NonFiniteValue,
                format!("pattern position axis {axis} is {v}"),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_model::MaterialId;

    fn lattice() -> DetailLattice {
        DetailLattice::new(0x5A17_0000_0000_0001)
    }

    fn field_with(material: MaterialId) -> ShadingField {
        let mut field = ShadingField::none();
        field.declare(material, 0.5, 0.5).expect("a declared gain");
        field
    }

    #[test]
    fn a_point_addresses_to_its_own_world_subvoxel_and_lattice_cell() {
        let (cell, at) = address_at([0.3, 0.7, 1.2]).expect("a finite point");
        // 0.3 m is cell 1 of the 0.25 m grid (0.2 of the way along it → sample 3
        // of the 16 the base declares); 0.7 is cell 2, 0.8 along → 12; and 1.2 is
        // cell 4, past the block boundary at 1.0 — the world's cell, not the block's.
        assert_eq!(cell, [1, 2, 4]);
        assert_eq!((at.x, at.y, at.z), (3, 12, 12));
    }

    #[test]
    fn two_points_a_block_apart_share_their_lattice_address_and_not_their_cell() {
        // The correction in one assertion: the same offset inside two blocks is
        // the same `at` and a different `cell`, which is what stops one block's
        // pattern from being every block's pattern (C-162).
        let a = address_at([0.2, 0.9, 0.4]).expect("a finite point");
        let b = address_at([1.2, 0.9, 0.4]).expect("a finite point");
        assert_eq!(a.1, b.1, "the lattice address must not move with the block");
        assert_ne!(a.0, b.0, "the world cell must move with the block");
    }

    #[test]
    fn the_shader_recovers_the_world_cell_from_the_eye_s_two_halves() {
        // The shader has no world position: it has `p_rel`, `eye_phase` (the eye's
        // fraction of a metre) and `eye_voxel` (its integer part). The two halves
        // rebuild the eye to within an `f32` rounding step — far below a lattice
        // cell, and the GPU test skips the pixels a step could flip — so
        // `floor((p_rel + phase) * 4) + eye_voxel * 4` is the same cell this file
        // computes from the world position.
        let rel = [0.3, 0.55, 1.2];
        for eye in [[5.3, -2.7, 100.1], [-0.5, 0.25, 1234.75], [2_000_000.125, 0.0, -17.5]] {
            let (true_cell, true_at) = address_at([
                eye[0] + rel[0],
                eye[1] + rel[1],
                eye[2] + rel[2],
            ])
            .expect("a finite point");
            let voxel = eye_voxel(eye);
            let phase = eye_phase(eye);
            let mut cell = [0i32; 3];
            let mut at = [0u16; 3];
            for axis in 0..3 {
                // The fragment's own arithmetic: `q` is the position relative to
                // the eye's voxel, and the voxel's own place is added back.
                let q = rel[axis] as f32 + phase[axis];
                let scaled = q * SUBVOXEL_SIDE as f32;
                cell[axis] = scaled.floor() as i32 + voxel[axis] * SUBVOXEL_SIDE as i32;
                let (a, _) = cell_of((scaled - scaled.floor()) as f64, DETAIL_SIDE);
                at[axis] = a as u16;
            }
            assert_eq!(
                (cell, at),
                (true_cell, [true_at.x, true_at.y, true_at.z]),
                "eye {eye:?}"
            );
        }
    }

    #[test]
    fn the_footprint_is_one_pixel_carried_out_to_the_surface() {
        // A 720-line frame: the pixel's angle is under two milliradians, so at 2 m
        // the footprint is a couple of millimetres — the scale the GPU test quotes,
        // computed here rather than stated twice.
        let a = pixel_angle_rad(1.2, 720) as f64;
        assert!(a > 0.0 && a < 2.0e-3, "pixel angle {a}");
        // Head-on it is distance × angle, linear in distance and therefore smooth.
        assert!((eye_footprint_m(2.0, a, 1.0) - 2.0 * a).abs() < 1e-12);
        assert!((eye_footprint_m(4.0, a, 1.0) - 2.0 * eye_footprint_m(2.0, a, 1.0)).abs() < 1e-12);
        // Obliquity stretches it, and the floor bounds the stretch at 20×.
        assert!((eye_footprint_m(2.0, a, 0.5) - 4.0 * a).abs() < 1e-12);
        assert!((eye_footprint_m(2.0, a, 0.0) - 2.0 * a / FACE_COS_FLOOR).abs() < 1e-12);
        // A taller frame has smaller pixels, so the same surface draws finer rungs:
        // the footprint halves when the height doubles.
        assert!((pixel_angle_rad(1.2, 1440) as f64 - a / 2.0).abs() < 1e-12);
        // And it is continuous in the fragment's own state: a pixel's step in
        // distance moves the footprint by that step times the angle — never by a
        // quad's worth, which is the boil's whole cause.
        let step = eye_footprint_m(2.0 + a, a, 1.0) - eye_footprint_m(2.0, a, 1.0);
        assert!((step - a * a).abs() < 1e-15, "a pixel moved the footprint by {step}");
    }

    #[test]
    fn the_ladder_s_geometry_is_the_lattice_s_own() {
        // The rungs are powers of two of the addressed cell: the **drawn base** is
        // octave [`OCTAVE_FLOOR`] ([`MIN_FEATURE_M`] — 32 samples per block axis),
        // octave [`SUBVOXEL_OCTAVE`] is the subvoxel, and [`OCTAVE_TOP`] is
        // [`MAX_FEATURE_M`]. Both bounds are tight: one rung finer is the addressed
        // base's own cell and is never drawn, one rung coarser is past the ceiling.
        assert_eq!(octave_cell_m(OCTAVE_FLOOR), MIN_FEATURE_M);
        assert_eq!(MIN_FEATURE_M, SUBVOXEL_CELL_M / 8.0, "32 samples per block axis");
        assert!(octave_cell_m(OCTAVE_FLOOR - 1) < MIN_FEATURE_M, "the floor is not tight");
        assert_eq!(octave_cell_m(SUBVOXEL_OCTAVE), SUBVOXEL_CELL_M);
        // The ceiling is now 32 m (octave 11), extended from 4 m to stop the moire
        // the vertex detail produced when the ladder went silent past 4 m.
        assert_eq!(octave_cell_m(OCTAVE_TOP), 32.0);
        assert!(octave_cell_m(OCTAVE_TOP + 1) > 32.0, "the ceiling is not tight");
        // Quantising clears the bits a rung cannot see; at the subvoxel rung it
        // pins the whole address, because those features are subvoxel-sized.
        assert_eq!(quantise(DetailIndex::centre(), 0), DetailIndex::centre());
        let one = DetailIndex::new(12, 5, 9).unwrap();
        assert_eq!(quantise(one, 2), DetailIndex::new(12, 4, 8).unwrap());
        assert_eq!(
            quantise(DetailIndex::centre(), SUBVOXEL_OCTAVE),
            DetailIndex::new(0, 0, 0).unwrap()
        );
        // And the cell quantisation for the rungs past the subvoxel: floor on two's
        // complement, so negative cells tile without a seam.
        assert_eq!(quantise_cell([-8, 7, 16], SUBVOXEL_OCTAVE), [-8, 7, 16]);
        assert_eq!(quantise_cell([-8, 7, 16], SUBVOXEL_OCTAVE + 1), [-8, 6, 16]);
        // The flat packing is the side's own width: four bits an axis, which is
        // what makes the out-of-range address below alias.
        assert_eq!(flat_address(DetailIndex::centre()), 8 | (8 << 4) | (8 << 8));
        assert_eq!(
            flat_address(DetailIndex { x: 16, y: 0, z: 0 }),
            flat_address(DetailIndex::new(0, 1, 0).unwrap())
        );
    }

    #[test]
    fn a_material_the_field_does_not_vary_shows_no_deviation() {
        let lattice = lattice();
        let varied = MaterialId(6);
        let other = MaterialId(7);
        let field = field_with(varied);
        let pattern = Pattern::new(&lattice, &field);
        let (cell, at) = address_at([0.3, 0.7, 1.2]).unwrap();
        let none = pattern
            .deviation(other, cell, at, 0.0, SurfacePattern::Speckle)
            .expect("a sample");
        assert_eq!(none, DetailSample::NONE);
    }

    #[test]
    fn a_rung_s_weight_does_not_move_under_a_walk_sized_step() {
        // C-171's repair, pinned: the *staged* weight is constant across any step
        // smaller than the rung's own doubling stage, on both axes — a 0.05 m
        // frame of a walk cannot re-weight a rung. The un-staged weight did move
        // (that is what the A/B measured), so the two disagree somewhere in a
        // stage, which is what makes this test able to fail.
        for cell in [octave_cell_m(1), octave_cell_m(4), octave_cell_m(8), octave_cell_m(11)] {
            let stage = cell / 4.0;
            for i in 0..40 {
                let a = stage * 1.1 + 1e-4 * i as f64;
                let b = a + 0.05;
                // The step crossed a doubling — the weight is allowed to move.
                if (b / stage).log2().ceil() != (a / stage).log2().ceil() {
                    continue;
                }
                assert_eq!(
                    octave_weight_staged(a, cell),
                    octave_weight_staged(b, cell),
                    "a 0.05 m step moved a rung's staged weight at cell {cell}"
                );
            }
        }
        // And the staged weight is not the plain weight — the staging changed
        // something, or this function is decorative.
        let cell = octave_cell_m(4);
        assert!(
            (0..64).any(|i| octave_weight_staged(1e-4 + i as f64 * 1e-3, cell)
                != octave_weight(1e-4 + i as f64 * 1e-3, cell)),
            "staging changed no weight anywhere in the first 64 mm"
        );
    }

    #[test]
    fn the_staged_weight_keeps_the_ladder_s_silence_and_its_draw() {
        // The three properties the staging has to preserve, or the ladder's own
        // guarantees break. **Silence**: at and past the rung's Nyquist floor the
        // plain weight is exactly 0, and the staged weight must be too — the
        // first staging attempt raised weights and drew rungs past the ceiling
        // (the GPU agreement test held it). **Draw**: strictly inside the floor
        // the plain weight is positive and the staged weight is its full 1 — the
        // power-of-two ceiling attempt staged to a larger footprint where the
        // plain weight was 0 and blanked the whole ladder (the footprint tests
        // held it). **Boundary**: the floor's own point stays silent, so the
        // step and the smoothstep agree exactly where the ladder ends.
        for cell in [octave_cell_m(1), octave_cell_m(4), octave_cell_m(11)] {
            let floor = cell / 4.0;
            // Silence: the plain weight is 0 at and past the floor, and the
            // staged weight is exactly 0 there too.
            for footprint in [floor, floor * 1.001, floor * 2.0, floor * 100.0] {
                assert_eq!(octave_weight(footprint, cell), 0.0, "plain weight drew at {footprint}");
                assert_eq!(
                    octave_weight_staged(footprint, cell),
                    0.0,
                    "staged weight drew past the rung's floor, at {footprint} m, cell {cell}"
                );
            }
            // Draw: strictly inside the floor the plain weight is positive and
            // the staged weight is its full 1 — the rung the pixel can draw is
            // drawn, not blanked.
            for footprint in [1e-6, 1e-4, floor * 0.5, floor * 0.999] {
                assert!(octave_weight(footprint, cell) > 0.0, "plain weight silent at {footprint}");
                assert_eq!(
                    octave_weight_staged(footprint, cell),
                    1.0,
                    "staged weight blank where plain draws, at {footprint} m, cell {cell}"
                );
            }
        }
    }

    #[test]
    fn a_prefiltered_rung_does_not_slide_within_its_own_coarse_cell() {
        // **The property the area average has to have**, and the one the CPU/GPU
        // agreement test cannot see: both sides make the identical mistake, so they
        // agree perfectly while the ladder is wrong.
        //
        // A rung's average is taken over its own coarse cell, so every fragment in
        // that cell must read the same value. Offsetting the taps from the fragment
        // instead of from the cell walks into its neighbours and slides the average's
        // centre as the fragment moves — which is the re-rolling the pass exists to
        // remove, and it is worst on exactly the large side faces where a prefiltered
        // rung carries the whole surface.
        let lattice = lattice();
        let material = MaterialId(6);
        let field = field_with(material);
        let pattern = Pattern::new(&lattice, &field);
        let surface = SurfacePattern::Speckle;
        let mut seen = std::collections::BTreeSet::new();
        // **A footprint where only prefiltered rungs draw.** Between the 6th rung's
        // floor (0.25 m) and the 7th's (0.5 m) the ladder is carried entirely by the
        // prefiltered rungs. Anywhere else this test is vacuous: a footprint past the
        // top rung's floor leaves every rung silent, `deviation` answers `NONE`
        // everywhere, and one distinct value is read whether the averaging is right
        // or not. That is what the first version of this test did.
        let footprint = (octave_cell_m(6) + octave_cell_m(7)) / 8.0;
        // The **finest** prefiltered rung's own cell, so walking across it crosses no
        // coarser rung's boundary and the only value that can move is the one under test.
        let first_prefiltered = PREFILTER_SPAN.trailing_zeros() + SUBVOXEL_OCTAVE;
        let origin = quantise_cell([512i32, 512, 512], first_prefiltered);
        for step in 0..PREFILTER_SPAN {
            for axis in 0..3 {
                let mut cell = origin;
                cell[axis] += step;
                let (_, at) = address_at([
                    cell[0] as f64 / 4.0,
                    cell[1] as f64 / 4.0,
                    cell[2] as f64 / 4.0,
                ])
                .expect("a finite point");
                let d = pattern
                    .deviation(material, cell, at, footprint, surface)
                    .expect("a sample");
                seen.insert((d.roughness_delta.to_bits(), d.reflectance_delta.to_bits()));
            }
        }
        assert_eq!(
            seen.len(),
            1,
            "a prefiltered rung read {} different values inside one coarse cell, so it \
             slides with the fragment: {seen:?}",
            seen.len()
        );
    }

    #[test]
    fn the_blend_cannot_tell_the_vertical_axis_from_the_horizontal_one() {
        // **The user's report, stated as an invariant:** *"lod material rendering and
        // smooth step blending isn't applying to the vertical axis"*. The ladder's only
        // eye-dependent input is [`eye_footprint_m`], whose distance is
        // `length(p_rel)` — the fragment's own **three-dimensional** distance from the
        // eye. So a fragment a stated distance out answers the same however that
        // distance is laid along an axis, and no axis can be quietly switched off.
        //
        // Pinned per axis rather than asserted once, because the failure this guards is
        // a *missing* term: an axis that never reached the weight would agree with
        // itself perfectly.
        let angle = f64::from(pixel_angle_rad(std::f64::consts::FRAC_PI_3, 360));
        // A face seen at an angle, so this is not quietly the `cos_face = 1` case where
        // an axis could be dropped without showing.
        const FACE_COS: f64 = 0.7;
        for &d in &[2.0f64, 10.0, 40.0] {
            let weights = |p: [f64; 3]| {
                let distance = p.into_iter().map(|v| v * v).sum::<f64>().sqrt();
                let footprint = eye_footprint_m(distance, angle, FACE_COS);
                (OCTAVE_FLOOR..=OCTAVE_TOP)
                    .map(|octave| octave_weight(footprint, octave_cell_m(octave)))
                    .collect::<Vec<f64>>()
            };
            let along_x = weights([d, 0.0, 0.0]);
            let along_y = weights([0.0, d, 0.0]);
            let along_z = weights([0.0, 0.0, d]);
            assert_eq!(along_x, along_y, "the vertical axis is weighted differently at {d} m");
            assert_eq!(along_x, along_z, "the depth axis is weighted differently at {d} m");
            // And the ladder is not silent there, or all three agree because none of
            // them drew anything.
            assert!(
                along_x.iter().any(|w| *w > 0.0),
                "no rung drew at {d} m, so no axis was actually compared"
            );
        }
    }

    #[test]
    fn no_rung_s_weight_has_a_step_in_it() {
        // **The blend has to stay a blend**, and this is the property that says so.
        //
        // C-171 measured the vertical shimmer's asymmetry living in `octave_weight`'s
        // smoothstep, and named three routes to repair it. Route (a) — staging each
        // rung whole-or-nothing at `cell / 4`, `octave_weight_staged` — is written and
        // tested in this file and was measured on the GPU
        // (`examples/pattern-flicker.rs`, 2026-10-08): it takes the pixels that change
        // down 45% and *raises* the total on **both** axes (573.1 → 673.0 vertical,
        // 234.0 → 265.2 horizontal), because a staged rung is drawn at its full 1.0
        // where the fade drew it at a partial weight. The smooth LOD blend reads as
        // harsh and broken, so it was not taken.
        //
        // What route (a) gives up is *continuity*: a rung that goes 0 → 1 across a
        // vanishing change of footprint is a switch, whatever it is called. That is
        // asserted here rather than the route, so a future fade may change shape and
        // width freely and only has to stay smooth.
        //
        // The bound is the smoothstep's own steepest slope, not a tuned constant: `t`
        // moves `4 / cell` per metre and `smoothstep`'s derivative peaks at 1.5, so
        // `6 / cell` per metre is the fastest a fade may ever climb. A staged weight
        // clears it by five orders of magnitude at the very threshold it switches on.
        const SAMPLES: usize = 4096;
        for octave in OCTAVE_FLOOR..=OCTAVE_TOP {
            let cell = octave_cell_m(octave);
            let floor = cell / 4.0;
            // A vanishing step, and the probes that straddle the rung's own floor —
            // which is where a staged weight puts its switch, and the one place a
            // uniform sweep is most likely to step around.
            let h = cell * 1.0e-6;
            let mut probes = (0..=SAMPLES)
                .map(|i| floor * i as f64 / SAMPLES as f64)
                .collect::<Vec<f64>>();
            probes.extend((0..=4).map(|k| (floor - k as f64 * h).max(0.0)));
            for f in probes {
                let a = octave_weight(f, cell);
                let b = octave_weight(f + h, cell);
                assert!(
                    (b - a).abs() <= 6.0 * OCTAVE_FADE_WIDTH * h / cell + 1.0e-12,
                    "the rung at octave {octave} (cell {cell} m) jumps by {} at a footprint \
                     of {f} m: that is a switch, not a fade",
                    b - a
                );
            }
        }
    }

    #[test]
    fn the_ladder_handed_a_vanishing_step_does_not_jump() {
        // Route (a) changed the ladder's **call site**, not the weight function, so a
        // test on `octave_weight` alone would not have caught it — this one goes
        // through [`Pattern::deviation`], which is the path the CPU takes and which
        // `shading.wgsl` transcribes. A rung switching on under the ladder is a jump in
        // the drawn material, whatever the weight function is called.
        let lattice = lattice();
        let material = MaterialId(6);
        let field = field_with(material);
        let pattern = Pattern::new(&lattice, &field);
        let surface = SurfacePattern::Speckle;
        let (_, at) = address_at([0.37, 0.61, 0.83]).expect("a finite point");
        let cell = [3i32, 9, 14];
        // **The interior of the ladder, where at least two rungs still draw.** Past
        // the second-coarsest rung's floor exactly one rung is left, and a ladder with
        // one rung is a weighted mean of a single number: its `weight_sum` reaches zero
        // *together with* its numerator, so the normalised mean does not fade toward
        // nothing — it holds its last value and then returns [`DetailSample::NONE`].
        // That discontinuity at the ladder's own ceiling is real, it is not this
        // cross-fade, and it is left named rather than asserted away here.
        let two_rungs = octave_cell_m(OCTAVE_TOP - 1) / 4.0;
        let mut probed = 0usize;
        for octave in OCTAVE_FLOOR..=OCTAVE_TOP {
            let rung = octave_cell_m(octave);
            let h = rung * 1.0e-6;
            // Straddle this rung's floor and its half-way point, where the finer
            // rungs are drawing and a switch in this one is visible on its own.
            for f in [rung / 4.0, rung / 8.0] {
                if f >= two_rungs {
                    continue;
                }
                probed += 1;
                for k in 0..=4 {
                    let f = (f - k as f64 * h).max(0.0);
                    let a = pattern
                        .deviation(material, cell, at, f, surface)
                        .expect("a sample");
                    let b = pattern
                        .deviation(material, cell, at, f + h, surface)
                        .expect("a sample");
                    assert!(
                        (a.roughness_delta - b.roughness_delta).abs() < 1.0e-6
                            && (a.reflectance_delta - b.reflectance_delta).abs() < 1.0e-6,
                        "the ladder jumped by ({}, {}) at a footprint of {f} m — the rung at \
                         octave {octave} changed weight inside a vanishing step",
                        a.roughness_delta - b.roughness_delta,
                        a.reflectance_delta - b.reflectance_delta
                    );
                }
            }
        }
        assert!(
            probed >= 8,
            "only {probed} rungs were probed, so the cross-fade was barely compared"
        );
    }

    #[test]
    fn a_footprint_between_the_rungs_is_carried_by_a_coarser_one() {
        // The base's own cell is 15.6 mm; a pixel whose footprint is 9 mm cannot
        // draw it (its four-pixel floor is 7.8 mm), and it must not go blank there:
        // the coarser rungs carry it — which is what makes a surface past arm's
        // length textured rather than flat (C-163, floored at C-164).
        let lattice = lattice();
        let material = MaterialId(6);
        let field = field_with(material);
        let pattern = Pattern::new(&lattice, &field);
        let surface = SurfacePattern::Speckle;
        assert_eq!(octave_weight(0.009, MIN_FEATURE_M), 0.0);
        let (_, at) = address_at([0.0, 0.0, 0.0]).unwrap();
        let mut moved = 0usize;
        for i in 0..256i32 {
            let cell = [i % 16, (i / 16) % 16, i / 256];
            let d = pattern
                .deviation(material, cell, at, 0.009, surface)
                .expect("a sample");
            if d != DetailSample::NONE {
                moved += 1;
            }
        }
        assert!(moved > 0, "the ladder went blank where a coarser rung carries it");
    }

    #[test]
    fn below_the_resolvable_rung_movement_changes_nothing() {
        // The anti-aliasing property in one assertion: at a 9 mm footprint the drawn
        // base (31.25 mm) is below its own four-pixel floor, so two addresses that
        // differ only in the bits below the drawn floor give the *same* pattern — the
        // sub-pixel shimmer the eye-check saw as streaks (C-163). And an address
        // across a rung that *is* drawn does change it, so the silence is the
        // ladder's, not the field's.
        let lattice = lattice();
        let material = MaterialId(6);
        let field = field_with(material);
        let pattern = Pattern::new(&lattice, &field);
        let surface = SurfacePattern::Speckle;
        let one = DetailIndex::new(1, 1, 1).unwrap();
        let zero = DetailIndex::new(0, 0, 0).unwrap();
        let at_one = pattern.deviation(material, [0, 0, 0], one, 0.009, surface).unwrap();
        let at_zero = pattern.deviation(material, [0, 0, 0], zero, 0.009, surface).unwrap();
        assert_eq!(at_one, at_zero, "an unresolvable rung changed the drawn pattern");
        let across = DetailIndex::new(4, 4, 4).unwrap();
        let far = pattern
            .deviation(material, [0, 0, 0], across, 0.009, surface)
            .unwrap();
        assert!(far != at_one, "a rung that is drawn changed nothing");
    }

    #[test]
    fn the_ladder_is_never_flat_while_a_rung_resolves() {
        // *"Not be totally flat"* measured: for every footprint the ladder can draw
        // at, some address shows a nonzero deviation. The flat patches the eye-check
        // saw came from one white-noise value per subvoxel; every rung here varies
        // within its own cells.
        let lattice = lattice();
        let material = MaterialId(6);
        let field = field_with(material);
        let pattern = Pattern::new(&lattice, &field);
        let surface = SurfacePattern::Speckle;
        let (_, at) = address_at([0.0, 0.0, 0.0]).unwrap();
        for footprint in [0.0, 1e-4, 1e-3, 1e-2, 0.25] {
            let mut moved = 0usize;
            for i in 0..256i32 {
                let cell = [i % 16, (i / 16) % 16, i / 256];
                let d = pattern
                    .deviation(material, cell, at, footprint, surface)
                    .expect("a sample");
                if d != DetailSample::NONE {
                    moved += 1;
                }
            }
            assert!(moved > 0, "the ladder drew nothing at a footprint of {footprint} m");
        }
        // Past the top rung's own floor (a quarter of 32 m = 8 m) the ladder is
        // honestly empty — an average, not a guess — and the test does not pretend
        // otherwise.
        assert_eq!(
            pattern
                .deviation(material, [0, 0, 0], at, 9.0, surface)
                .unwrap(),
            DetailSample::NONE
        );
    }

    #[test]
    fn each_shaper_stretches_the_noise_along_the_axis_its_material_reads() {
        // The user's ruling is that each material *shapes* the same white noise:
        // bark runs along the tree's axis (z), brushed grain runs across it (x). The
        // shaper clears low address bits on the stretched axis, so at the ladder's
        // **floor rung** — C-164's 32 samples per block axis — a stroll of the same
        // length crosses four times more cells across the grain than along it.
        //
        // **Measured on the stencil the ladder itself applies**, not on the blended
        // value: a blend's change count is a weighted sum crossing rounding steps
        // rather than cells crossed, and since the floor masks the base's finest bit
        // that count is diluted (3 along grain against 7 across, where the cells are
        // 1 against 7). The cells are the claim; the blend is asserted below as the
        // direction only.
        let cell_changes = |surface: SurfacePattern, axis: usize| {
            let mut n = 0usize;
            let mut prev: Option<DetailIndex> = None;
            for i in 0..(DETAIL_SIDE as u16) {
                let mut a = [(DETAIL_SIDE as u16 / 2); 3];
                a[axis] = i;
                let s = quantise(
                    surface.stencil(DetailIndex::new(a[0], a[1], a[2]).unwrap()),
                    OCTAVE_FLOOR,
                );
                if let Some(p) = prev {
                    if p != s {
                        n += 1;
                    }
                }
                prev = Some(s);
            }
            n
        };
        let along_z = cell_changes(SurfacePattern::Bark, 2);
        let across_z = cell_changes(SurfacePattern::Bark, 0);
        assert!(
            along_z * 4 <= across_z && along_z < across_z,
            "bark crossed {along_z} cells along its grain and {across_z} across it"
        );
        let along_x = cell_changes(SurfacePattern::Brushed, 0);
        let across_x = cell_changes(SurfacePattern::Brushed, 1);
        assert!(
            along_x * 4 <= across_x && along_x < across_x,
            "brushed crossed {along_x} cells along its grain and {across_x} across it"
        );
        // And the drawn blend still shows the grain: the same stroll on `deviation`
        // changes less often along the stretched axis than across it.
        let lattice = lattice();
        let material = MaterialId(13);
        let field = field_with(material);
        let pattern = Pattern::new(&lattice, &field);
        let blend_changes = |surface: SurfacePattern, axis: usize| {
            let mut n = 0usize;
            let mut prev: Option<DetailSample> = None;
            for i in 0..(DETAIL_SIDE as u16 - 1) {
                let mut a = [(DETAIL_SIDE as u16 / 2); 3];
                a[axis] = i;
                let d = pattern
                    .deviation(
                        material,
                        [0, 0, 0],
                        DetailIndex::new(a[0], a[1], a[2]).unwrap(),
                        0.0,
                        surface,
                    )
                    .unwrap();
                if let Some(p) = prev {
                    if p != d {
                        n += 1;
                    }
                }
                prev = Some(d);
            }
            n
        };
        let b_along_z = blend_changes(SurfacePattern::Bark, 2);
        let b_across_z = blend_changes(SurfacePattern::Bark, 0);
        assert!(
            b_along_z < b_across_z,
            "bark changed {b_along_z} times along its grain and {b_across_z} across it"
        );
        let b_along_x = blend_changes(SurfacePattern::Brushed, 0);
        let b_across_x = blend_changes(SurfacePattern::Brushed, 1);
        assert!(
            b_along_x < b_across_x,
            "brushed changed {b_along_x} times along its grain and {b_across_x} across it"
        );
    }

    #[test]
    fn every_deviation_stays_inside_the_range_the_model_states() {
        // `with_detail`'s promise — a legal value stays legal — holds only inside
        // `[-1, 1)`, so the drawn deviation has to stay there too.
        let lattice = lattice();
        let material = MaterialId(8);
        let field = field_with(material);
        let pattern = Pattern::new(&lattice, &field);
        let mut seen = 0usize;
        for surface in [
            SurfacePattern::Speckle,
            SurfacePattern::Foliage,
            SurfacePattern::Bark,
            SurfacePattern::Brushed,
        ] {
            for i in 0..64u64 {
                let p = [(i as f64) * 0.137, (i as f64) * 0.291 + 0.5, (i as f64) * 0.053];
                for footprint in [0.0, 1e-4, 0.001, 0.01, 0.1, 1.0] {
                    let d = pattern.at(material, p, footprint, surface).expect("a sample");
                    for v in [d.roughness_delta, d.reflectance_delta] {
                        assert!((-1.0..1.0).contains(&v), "deviation {v} at {p:?} {surface:?}");
                    }
                    if d != DetailSample::NONE {
                        seen += 1;
                    }
                }
            }
        }
        assert!(seen > 0, "no address produced any deviation at all");
    }
}
