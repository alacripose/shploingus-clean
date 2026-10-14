//! The sky pass — **the scattering is not in this file, on purpose**.
//!
//! # What this shader is allowed to know
//!
//! **The transport is `atmosphere.wgsl`'s, and no body is drawn here.**
//!
//! A pixel becomes a **world ray** (the camera's yaw and pitch travel in `view`) and the ray is
//! handed to `sky_radiance_rgb`, the one transcription of `voxel_light::solver`'s transport that
//! this pass and `shading.wgsl` are both compiled with. There is **no dome table, no grid and no
//! interpolation** between the model and the pixel any more: the sky a pixel carries is the
//! integral the light rig was built from, evaluated at that pixel's own direction. What was here
//! before was a 64×64 tabulation of a *different* model (`voxel_appearance::sky`'s fit), which is
//! two skies — one the frame drew and one the world was lit by — and a grid that sampled a sunset's
//! aureole as a staircase.
//!
//! **The Sun and the Moon are not drawn here** (C-184). They are **astronomical bodies in the
//! record stream** now — a body is one 32-byte record and one origin, placed by the world pass's own
//! `vs_body`/`fs_body`, so the frame's geometry carries them and this pass carries only the air.
//! `docs/lighting-sky-and-moon.md` §7 has why, and what it fixed.
//!
//! # Why the pixels are alpha 0
//!
//! Alpha keeps the meaning `world.rs` gives it — **"geometry is drawn here"** — so the frame
//! can still tell a surface from the sky behind it. A sky pass that wrote alpha 1 would be
//! indistinguishable from terrain to every measurement that reads the alpha, and
//! `Frame::response` would then be counting sky as if it were shaded stone.
//!
//! The invariant this establishes: **every pixel is either sky or geometry, and none is left
//! undrawn.** That is the whole repair — the old pass cleared to transparent, so every pixel
//! with no voxel on it was nothing at all.
//!
//! **What a body's radiance is** is unchanged and still not this file's business: the CPU derives it
//! from the band table — the Sun's own bands over its solid angle for the disc, and the Sun's bands
//! times the Moon's declared reflectance over π for the Moon's lit face — and the drawn body carries
//! that number through the record.

// The camera's geometry. `pitch_rad`, `yaw_rad` and `tan_half_fov` are the only numbers
// needed to turn a pixel into a world direction; **everything physical** — the air, the
// sun's own path, the bands and their response weights — arrives in `sky_model`, which is
// `atmosphere.wgsl`'s binding rather than a field of this struct.
struct SkyView {
    pitch_rad: f32,
    tan_half_fov: f32,
    yaw_rad: f32,
    aspect: f32,
    // Written for the world pass's fluid reflection; declared here so the two
    // structs stay the same size and layout. The sky itself does not read it.
    fluid_f0: f32,
    // **How much dither the target's quantisation calls for** (C-166): `0` for a
    // float target, `1` for the 8-bit `sRGB` surface the window presents. This is
    // `shading.wgsl`'s `View::screen.y`, arriving as its own uniform because the
    // sky has its own bind group.
    //
    // **It is the sky that needs it most.** The world is made of flat quads whose
    // shades are mostly several steps apart, while the sky is one long smooth
    // gradient and therefore the only surface on the screen that spends many pixels
    // inside a single step — which is exactly the condition that draws a band.
    dither: f32,
    // **Whether this target is a display** (C-167): `shading.wgsl`'s `View::screen.z`.
    //
    // The sky needs this one more than anything else does. It is the brightest thing
    // on the screen — the sun's own radiance is not a number an 8-bit target can
    // hold — so it is the pass that spends the most of its range above white, and the
    // one where a clip is largest.
    tonemap: f32,
    // **Declared, unread, and load-bearing**: they make this struct's first block twelve
    // `f32` — three whole `vec4`s — with **no implicit padding**, so the bodies below begin
    // on a `vec4` boundary and `WorldPipeline::write_camera` writes exactly the thirty-two
    // `f32` this struct declares. A struct whose padding a reader has to work out is a
    // struct whose size can drift from the buffer it is bound to, which is the defect that
    // made the sky black in the first place.
    //
    // **The first two of the five used to be the dome's grid** — its azimuth and elevation
    // counts, which `write_camera` sent so the pass could index a table. The table is gone
    // and the model carries its own band count, so they are reserves rather than fields a
    // reader would have to trace to a shader that no longer exists.
    reserved0: f32,
    reserved1: f32,
    reserved2: f32,
    reserved3: f32,
    reserved4: f32,
    // The bodies' five `vec4`s used to close this struct: the Sun's and the Moon's centres, halves,
    // radiances and the Sun's direction as they were handed to this pass (C-176). They are records in
    // the world's own stream now (C-184), so this struct is the twelve `f32` of the camera and the
    // air, and `WorldPipeline::write_camera` uploads exactly those.
};

@group(0) @binding(0) var<uniform> view: SkyView;
// `sky_model` — the air, the sun's own column and the bands — is declared by
// `atmosphere.wgsl`, at binding 5, and **the world pass binds the same buffer**: the sky a
// fluid reflects is the sky this pass draws, which is one buffer and one evaluation.

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) ndc_x: f32,
    @location(1) ndc_y: f32,
};

/// A fullscreen triangle — one primitive, no vertex buffer, three vertices' worth of
/// clip-space positions generated from `vertex_index`.
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VsOut {
    var corners = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    let xy = corners[vertex_index];
    var out: VsOut;
    out.position = vec4<f32>(xy, 0.0, 1.0);
    out.ndc_x = xy.x;
    out.ndc_y = xy.y;
    return out;
}

// `DEGREES_PER_RADIAN` is `atmosphere.wgsl`'s: one constant, one file, both passes.


@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    // A uniform with no model behind it draws black — docs/lighting-sky-and-moon.md
    let sy = sin(view.yaw_rad);
    let cy = cos(view.yaw_rad);
    let sp = sin(view.pitch_rad);
    let cp = cos(view.pitch_rad);
    let forward = vec3<f32>(sy * cp, sp, cy * cp);
    let right = vec3<f32>(cy, 0.0, -sy);
    let up = vec3<f32>(-sp * sy, cp, -sp * cy);

    // The ray for this pixel, in camera space: `ndc · tan(half fov)`, forward `1`.
    let x = in.ndc_x * view.tan_half_fov * view.aspect;
    let y = in.ndc_y * view.tan_half_fov;
    let dir = normalize(right * x + up * y + forward);

    // What this ray carries — docs/lighting-sky-and-moon.md
    let radiance = sky_radiance_rgb(dir);

    // Transform first, then dither — docs/lighting-sky-and-moon.md
    var lit = radiance;

    // Alpha 0: sky, not geometry. See the module doc — a body is drawn by the world pass and is still
    // not a surface, so alpha stays the one channel that says "a mesh is on this pixel".
    var presentable = lit;
    if (view.tonemap > 0.0) {
        presentable = tone_map(lit);
    }
    return vec4<f32>(dithered(presentable, in.position.xy, view.dither), 0.0);
}
