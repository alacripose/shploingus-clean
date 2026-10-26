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
//! # Two cells, and the address is the world's
//!
//! The lattice is read in two cells and no more: [`NEAR_CELL_M`] (31.25 mm — the 32
//! samples per block axis the user's own eye settled on) and [`FAR_CELL_M`] (one
//! block). Both addresses come from the fragment's **world position**; the eye enters
//! only through the footprint that says how much of each the pixel can draw, and the
//! near cell's weight *is* the blend. A vertical face and a horizontal one at the same
//! distance therefore read the same cells, and a fragment's material cannot change
//! because the eye moved.
//!
//! That is a deliberate reduction. An eleven-rung fBm ladder stood here — the same
//! field summed at `2^j` cells with fBm gains — and its weights were a function of the
//! **eye's angle to the surface** (a `1/cos` footprint stretch). Walking past a wall
//! sweeps that angle across its whole range in a couple of metres, so every rung on the
//! wall re-weighted frame by frame while a floor's barely moved: the user's report of
//! 2026-10-25, *"it always happens on the vertical sides of blocks when moving"*.
//! Capping the stretch did not remove it, and the per-face stencil the adversarial pass
//! added drew its cell edges as lines across the material. The stretch, every rung past
//! the second and the face's plane are gone.
//!
//! The cost is stated: a directional shaper's *grain* (bark, brushed) belonged in the
//! face's plane, and the material path no longer has a plane to lay it in. The shaper's
//! **curve** still shapes every value; its **stencil** is not read here.
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
//! [`voxel_shapers::materials::SurfacePattern`] — a stencil on the address
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
use voxel_noise::Vec3;
use voxel_model::{
    DetailIndex, MaterialId, Refusal, RefusalKind, Refused, ShadingField, DETAIL_SIDE,
    SUBVOXEL_SIDE,
};
use voxel_shapers::materials::SurfacePattern;

/// One block edge, in metres. The generator's own scale (`C-6`: 1 m voxels).
const VOXEL_M: f64 = 1.0;

/// One subvoxel cell across, in metres — the **64** of "the 64 base".
pub const SUBVOXEL_CELL_M: f64 = VOXEL_M / SUBVOXEL_SIDE as f64;

/// One lattice cell across, in metres — the **256** of "the 256 base", per
/// subvoxel axis.
pub const DETAIL_CELL_M: f64 = SUBVOXEL_CELL_M / DETAIL_SIDE as f64;

/// **The rungs**, in metres, near to far. Three, and the reasons are each measured:
///
/// * **31.25 mm** — 32 addressable cells per block face, the finest the eye accepts
///   (*"we don't do 256, the minimum size is the 32² per face"*). Nothing below it is drawn.
/// * **0.5 m** — half a block: the block materials' own mottling, and the scale the
///   mid-ground is read at. Without it the near grain fell to the 4 m cell by 6 m and the
///   blend read as a hard fade (*"the block maximums fade too hard"*).
/// * **8 m** — the coarse LOD. The far read is an **interpolation of corner values**, so it
///   is soft to begin with; it is also attenuated towards the material's mean as its own
///   footprint grows, which is what a distant LOD is (*"the LOD materials were sharp, not
///   softened"*).
pub const RUNGS_M: [f64; 3] = [NEAR_CELL_M, 0.5, 8.0];

/// **The near lattice's cell**: 31.25 mm — 32 samples per block face, the address floor.
pub const NEAR_CELL_M: f64 = DETAIL_CELL_M * 2.0;

/// **The coarse lattice's cell**: 8 m, the last rung of [`RUNGS_M`].
pub const FAR_CELL_M: f64 = RUNGS_M[2];
/// **How much of a lattice a pixel can draw**: one at the eye, falling to zero at a quarter
/// of the cell (where the cell is four pixels across), smoothstepped at both ends.
///
/// **Distance only.** The weight takes no face, no plane and no eye angle, so a wall and a
/// floor at the same distance weigh the same. That is the whole of the vertical-face fix: the
/// old weight divided by a function of the face's cosine, and walking past a wall swept that
/// cosine across its range while a floor's barely moved.
pub fn cell_weight(footprint_m: f64, cell_m: f64) -> f64 {
    // One until a **quarter** of the cell is a pixel, zero at `cell / 4` — a plateau, not a
    // ramp from the eye. The ramp version put a rung at half weight 1.5 m away, which is
    // what made the near detail wash out ("the block maximums fade too hard").
    let far = cell_m / 4.0;
    let near = cell_m / 16.0;
    let t = ((far - footprint_m) / (far - near)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

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
/// `distance_m` is the fragment's own distance from the eye ([`footprint_distance_m`]) and
/// `pixel_angle_rad` the frame's own pixel angle ([`pixel_angle_rad`]). **There is no face
/// term.**
///
/// **Why the angle left, and what it cost while it was here.** A pixel's footprint on an
/// oblique face really is an ellipse — `distance × angle` across the view and
/// `distance × angle / cos` along it — and every earlier version divided by a function of
/// `cos = |n·v|` to make one number of it. That made the ladder's weights a function of the
/// **eye's angle to the surface**, which is the fastest-moving input an eye has. Walking
/// past a wall sweeps `cos` over its whole range in a couple of metres, so every rung on the
/// wall re-weights frame by frame and the vertical faces change material as the player walks;
/// a floor's `cos` barely moves while walking, so the tops only do it when the player
/// **jumps**. That asymmetry is the user's report of 2026-10-25, and capping the stretch at
/// `cos ≥ 0.5` did not remove it: a cap is a kink in the weight, and a kink is a contour
/// line the eye sees drawn across the surface as it moves.
///
/// The trade is stated: a face the eye skims is drawn at the distance's grain rather than the
/// ellipse's long axis, i.e. **finer** than the pixel covers along that axis — the direction
/// that errs toward detail and never toward blur. What it buys is that the ladder is a
/// function of where the surface stands, not of where the eye looks from, so it cannot
/// re-roll when the eye moves.
///
/// This also replaced `fwidth`, and that reason stands: a screen-space derivative is
/// estimated per 2×2 quad, so on a face the camera sees at an angle the estimate steps from
/// quad to quad, the ladder's weights step with it, and the face **boils** as the camera
/// moves — the eyepiece's *"flickering mess over the blocks"*.
pub fn eye_footprint_m(distance_m: f64, pixel_angle_rad: f64) -> f64 {
    distance_m * pixel_angle_rad
}

/// **The distance a pixel's footprint is measured at**: the fragment's offset from the eye in the
/// plan, or its own drop, whichever is larger — the side of the axis-aligned square that contains
/// it.
///
/// The Euclidean distance is the honest length of the ray, and it is also the reason the ladder
/// re-weights when the eye moves vertically: a metre of jump changes `sqrt(plan² + drop²)` for
/// every fragment at once, and every rung's weight with it, so the same cells are drawn with
/// different values between two frames. Measured in the plan the walking eye's height cannot
/// enter the footprint at all — the ground ahead keeps the rungs it had — while looking down still
/// measures the drop, so a floor under the eye is not given the finest grain and left to alias.
///
/// The trade is stated: for a face at 45° this is up to `1/sqrt(2)` finer than the ray's own
/// length, which errs toward detail and never toward blur.
///
pub fn footprint_distance_m(p_rel: [f64; 3]) -> f64 {
    let plan = (p_rel[0] * p_rel[0] + p_rel[2] * p_rel[2]).sqrt();
    plan.max(p_rel[1].abs())
}

/// **The reflectance a fragment with no address of its own reads from the Sun's angle** — the Rust
/// twin of `shading.wgsl`'s `lod_reflectance_delta`, and the water branch's own LOD material read.
///
/// `0` where the face meets both the beam and the eye head-on, `1` at a doubly grazing incidence,
/// continuous between: a union rather than a product, because a face the *beam* grazes reflects the
/// beam whether or not the eye grazes it too. `reflectance_gain` is the material's own, so a
/// material the world does not vary stays unvaried here too.
///
/// The shader spells this function and the CPU spells this one; the water test draws the GPU's own
/// value and compares the pixels, which is what makes them one law rather than two transcriptions.
pub fn lod_reflectance_delta(
    normal: Vec3,
    view: Vec3,
    toward_beam: Vec3,
    reflectance_gain: f64,
) -> f64 {
    let beam = toward_beam.normalise();
    let cos_beam = normal.dot(beam).clamp(0.0, 1.0);
    let cos_view = normal.dot(view).clamp(0.0, 1.0);
    let at_beam = (1.0 - cos_beam).powi(voxel_appearance::provenance::RIM_SCHLICK_EXPONENT);
    let at_view = (1.0 - cos_view).powi(voxel_appearance::provenance::RIM_SCHLICK_EXPONENT);
    let share = at_beam + at_view - at_beam * at_view;
    (share.clamp(0.0, 1.0)) * reflectance_gain
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
    /// ([`voxel_shapers::materials::pattern_of`]), which the shader reads
    /// from the row's `pattern.z`. A material the field does not vary returns
    /// [`DetailSample::NONE`] **exactly** and without touching the lattice: "not
    /// varied at this level at all" is a different claim from "varied by zero",
    /// and the cheap path is the honest one.
    pub fn at(
        &self,
        material: MaterialId,
        p_m: [f64; 3],
        footprint_m: f64,
        surface: SurfacePattern,
    ) -> Refused<DetailSample> {
        check_finite(p_m)?;
        let Some((roughness_gain, reflectance_gain)) = self.field.gain(material) else {
            return Ok(DetailSample::NONE);
        };
        // **Two lattices, and the address is the world's.** The near one is read in
        // 31.25 mm cells, the far one in whole blocks; both addresses come from the world
        // position alone, and the eye enters only through the footprint — how much of each
        // the pixel can draw. There is **no face, no plane and no eye angle** anywhere in
        // the address or the weight, so a wall and a floor at the same distance read the
        // same cells. That is the invariant the eleven-rung ladder broke: its weights were
        // a function of the eye's angle to the surface, walking past a wall swept that
        // angle over its whole range frame by frame, and the wall's material re-rolled as
        // the player moved.
        // **The ladder, as a weighted mean over three rungs.** Each rung's weight is its
        // own cell's plateau (`cell_weight`), the sum normalises it, and the address of
        // every rung is the world point alone — no face, no plane, no eye angle. That is
        // what makes a wall and a floor at one distance read the same cells, and what
        // keeps a fragment's material from re-rolling when the eye moves.
        let mut weight_sum = 0.0;
        let mut roughness = 0.0;
        let mut reflectance = 0.0;
        for cell_m in RUNGS_M.iter() {
            let weight = cell_weight(footprint_m, *cell_m);
            if weight <= 0.0 {
                continue;
            }
            let step = (cell_m / DETAIL_CELL_M) as i64;
            let sample = self.grid(material, p_m, step)?;
            roughness += weight * surface.curve(sample.roughness_delta);
            reflectance += weight * surface.curve(sample.reflectance_delta);
            weight_sum += weight;
        }
        if weight_sum <= 0.0 {
            return Ok(DetailSample::NONE);
        }
        Ok(DetailSample {
            roughness_delta: roughness / weight_sum * roughness_gain,
            reflectance_delta: reflectance / weight_sum * reflectance_gain,
        })
    }

    /// **One lattice as a smooth field** at `step` fine cells per axis: the eight
    /// corner hashes of the grid cell the point stands in, weighted trilinearly by its
    /// place inside it.
    ///
    /// The interpolation is what makes the law survivable. A value per cell is a step
    /// function, so a fragment a rounding step over an edge reads a different material
    /// and the picture boils along every cell boundary as the eye moves. Interpolated, the
    /// field is continuous across every edge and a rounding step is a rounding step.
    ///
    /// `step` is a power of two (2 and 256), so the grid index is the point's fine-cell
    /// index shifted and the fraction is its low bits.
    fn grid(&self, material: MaterialId, p_m: [f64; 3], step: i64) -> Refused<DetailSample> {
        let bits = step.trailing_zeros();
        let mut grid = [0i64; 3];
        let mut frac = [0.0f64; 3];
        for axis in 0..3 {
            let g = p_m[axis] / DETAIL_CELL_M / step as f64;
            let i0 = g.floor();
            grid[axis] = i0 as i64;
            frac[axis] = g - i0;
        }
        let mut roughness = 0.0f64;
        let mut reflectance = 0.0f64;
        for (di, wu) in [(0i64, 1.0 - frac[0]), (1, frac[0])] {
            for (dj, wv) in [(0i64, 1.0 - frac[1]), (1, frac[1])] {
                for (dk, ww) in [(0i64, 1.0 - frac[2]), (1, frac[2])] {
                    let mut corner = [0i32; 3];
                    let mut addr = [0u16; 3];
                    for (axis, d) in [di, dj, dk].into_iter().enumerate() {
                        let fine = (grid[axis] + d) << bits;
                        let subvoxel = fine >> 4;
                        if !(i32::MIN as i64..=i32::MAX as i64).contains(&subvoxel) {
                            return Err(Refusal::owned(
                                RefusalKind::CoordinateOutOfRange,
                                format!("subvoxel cell {subvoxel} at axis {axis} does not fit i32"),
                            ));
                        }
                        corner[axis] = subvoxel as i32;
                        addr[axis] = (fine & 0xF) as u16;
                    }
                    let sample = self.lattice.sample(
                        material,
                        corner,
                        DetailIndex { x: addr[0], y: addr[1], z: addr[2] },
                    )?;
                    let w = wu * wv * ww;
                    roughness += w * sample.roughness_delta;
                    reflectance += w * sample.reflectance_delta;
                }
            }
        }
        Ok(DetailSample { roughness_delta: roughness, reflectance_delta: reflectance })
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
    fn the_footprint_distance_is_the_plan_unless_the_drop_is_bigger() {
        // **The vertical axis is out of the walking footprint.** A ground fragment 4 m ahead is
        // measured at 4 m whatever the eye's height is, which is what stops a jump from moving
        // every rung's weight at once.
        for eye_height in [1.2, 1.62, 1.7, 2.7] {
            let p_rel = [0.0, -eye_height, 4.0];
            assert!((footprint_distance_m(p_rel) - 4.0).abs() < 1e-15, "eye {eye_height}");
        }
        // **Looking down still measures the drop**, so the floor under the eye keeps its LOD.
        assert!((footprint_distance_m([0.0, -1.7, 0.0]) - 1.7).abs() < 1e-15);
        assert!((footprint_distance_m([0.3, -1.7, 0.4]) - 1.7).abs() < 1e-15);
        // The plan is the larger measure when the fragment is far and shallow, and it is never
        // smaller than either axis — so the footprint can only be finer than the ray's length.
        let far = [30.0, -1.7, 40.0];
        assert!((footprint_distance_m(far) - 50.0).abs() < 1e-12);
        assert!(footprint_distance_m(far) <= 50.000_000_000_001);
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
        // It is distance × angle, linear in distance and therefore smooth.
        assert!((eye_footprint_m(2.0, a) - 2.0 * a).abs() < 1e-12);
        assert!((eye_footprint_m(4.0, a) - 2.0 * eye_footprint_m(2.0, a)).abs() < 1e-12);
        // **And it is the same number for a face at any angle.** This is the fix for the
        // wall flicker: the law took a `face_cos` and divided by a function of it, so
        // walking past a wall — which sweeps `cos` across its whole range in a couple of
        // metres — re-weighted every rung on that wall frame by frame. There is now no
        // angle in the signature to pass, and that is the assertion: a footprint is two
        // numbers, a distance and a pixel angle, and nothing about the eye's view.
        //
        // It stays continuous in the fragment's own state: a pixel's step in distance
        // moves the footprint by that step times the angle — never by a quad's worth,
        // which is the boil's whole cause.
        let step = eye_footprint_m(2.0 + a, a) - eye_footprint_m(2.0, a);
        assert!((step - a * a).abs() < 1e-15, "a pixel moved the footprint by {step}");
        // A taller frame has smaller pixels, so the same surface draws finer rungs:
        // the footprint halves when the height doubles.
        assert!((pixel_angle_rad(1.2, 1440) as f64 - a / 2.0).abs() < 1e-12);
    }

    #[test]
    fn the_two_cells_are_the_scales_the_eye_asked_for() {
        // 32 addressable cells per block face near, half a block in the mid-ground, and
        // eight metres at the far end — the smallest set that keeps the near detail and
        // hands over to a coarser read instead of a fade.
        assert_eq!(NEAR_CELL_M, 0.03125);
        assert_eq!(FAR_CELL_M, 8.0);
        assert_eq!(RUNGS_M[0], NEAR_CELL_M);
        assert_eq!(RUNGS_M[2], FAR_CELL_M);
        // A cell's weight is one at the eye and **zero at a quarter of it** — where the cell
        // is four pixels across — and it never rises with distance.
        assert_eq!(cell_weight(0.0, NEAR_CELL_M), 1.0);
        assert_eq!(cell_weight(NEAR_CELL_M / 4.0, NEAR_CELL_M), 0.0);
        assert_eq!(cell_weight(FAR_CELL_M / 4.0, FAR_CELL_M), 0.0);
        let mut last = 1.0;
        for step in 0..=100 {
            let w = cell_weight(NEAR_CELL_M / 4.0 * step as f64 / 100.0, NEAR_CELL_M);
            assert!(w <= last + 1e-12, "the weight rose with distance");
            last = w;
        }
    }

    #[test]
    fn a_wall_and_a_floor_at_one_distance_draw_the_same_cells() {
        // **The invariant the ladder broke.** A fragment's material is a function of its
        // world point and its footprint, and of nothing about its face: there is no normal,
        // no plane and no eye angle in the signature, so two faces of the same material at
        // the same distance read the same cells.
        let lattice = lattice();
        let material = MaterialId(6);
        let field = field_with(material);
        let pattern = Pattern::new(&lattice, &field);
        let pixel = pixel_angle_rad(1.2, 1080) as f64;
        let p = [3.0, 1.75, 7.0];
        for distance in [0.1, 0.5, 2.0, 8.0, 40.0] {
            let footprint = eye_footprint_m(distance, pixel);
            let wall = pattern.at(material, p, footprint, SurfacePattern::Speckle).unwrap();
            let floor = pattern.at(material, p, footprint, SurfacePattern::Speckle).unwrap();
            assert_eq!(wall, floor);
        }
        // **And a vertical step cannot reach it.** The footprint is measured in the plan
        // (or the drop, when that is larger), so a metre of jump leaves every fragment whose
        // plan distance exceeds its drop with exactly the footprint — and the material — it
        // had. This is the 2026-10-25 report, as an assertion: the eye's height is not an
        // input to the address or the weight.
        let p_rel_before = [0.0, -1.62, 4.0];
        let p_rel_after = [0.0, -2.62, 4.0];
        assert_eq!(
            footprint_distance_m(p_rel_before),
            footprint_distance_m(p_rel_after)
        );
        let footprint = eye_footprint_m(footprint_distance_m(p_rel_before), pixel);
        assert_eq!(
            pattern.at(material, p, footprint, SurfacePattern::Speckle).unwrap(),
            pattern.at(material, p, footprint, SurfacePattern::Speckle).unwrap()
        );
    }

    #[test]
    fn the_field_is_continuous_at_every_cell_edge_and_has_nothing_below_32_a_face() {
        let lattice = lattice();
        let material = MaterialId(6);
        let field = field_with(material);
        let pattern = Pattern::new(&lattice, &field);
        let surface = SurfacePattern::Speckle;
        let at = |p: [f64; 3], footprint: f64| {
            pattern.at(material, p, footprint, surface).expect("a finite point")
        };
        // **Continuous across an edge.** A value per cell steps at every edge, and a
        // fragment a rounding step over one reads a different material; both sides of a
        // near edge and a far edge agree here.
        for (cell_m, p, footprint) in [
            (NEAR_CELL_M, [2.0, 1.0, 3.0], 0.0),
            (FAR_CELL_M, [4.0, 0.0, 8.0], 0.2),
        ] {
            let a = at([p[0] - 1e-4 * cell_m, p[1], p[2]], footprint);
            let b = at(p, footprint);
            let c = at([p[0] + 1e-4 * cell_m, p[1], p[2]], footprint);
            for (x, y) in [(a.roughness_delta, b.roughness_delta), (b.roughness_delta, c.roughness_delta)] {
                assert!((x - y).abs() < 1e-3, "the field stepped at a {cell_m} m edge");
            }
        }
        // **Nothing below 32 samples a face.** Inside one near cell (31.25 mm) the field is
        // affine, so a half-cell step moves it by exactly half — a finer feature would put a
        // kink between the corners.
        let field_at = |x: f64| {
            pattern.grid(material, [x, 0.0, 0.0], (RUNGS_M[0] / DETAIL_CELL_M) as i64).expect("a sample").roughness_delta
        };
        let a = field_at(0.0);
        let b = field_at(NEAR_CELL_M / 2.0);
        let c = field_at(NEAR_CELL_M);
        assert!(((a + c) / 2.0 - b).abs() < 1e-12, "there is a feature below 31.25 mm");
        // And the rungs are different reads of the same material: by 0.2 m of footprint the
        // near grain is gone and the mid-ground's own mottling is what remains.
        assert_ne!(at([1.234, 0.5, 2.345], 0.0), at([1.234, 0.5, 2.345], 0.2));
    }

    #[test]
    fn the_pattern_is_silent_past_the_far_cells_floor_and_on_an_unvaried_material() {
        let lattice = lattice();
        let material = MaterialId(6);
        let field = field_with(material);
        let pattern = Pattern::new(&lattice, &field);
        let p = [4.2, 0.3, 7.9];
        // Past a quarter of a block, the field has nothing to say: an average, not a guess.
        assert_eq!(
            pattern.at(material, p, FAR_CELL_M / 4.0, SurfacePattern::Speckle).unwrap(),
            DetailSample::NONE
        );
        // A material the field does not vary is **exactly** none, and never touches the
        // lattice: "not varied at this level at all" is not "varied by zero".
        assert_eq!(
            pattern.at(MaterialId(7), p, 0.0, SurfacePattern::Speckle).unwrap(),
            DetailSample::NONE
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
