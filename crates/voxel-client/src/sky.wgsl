//! The sky pass — **the scattering is not in this file, on purpose**.
//!
//! # What this shader is allowed to know
//!
//! Geometry, and nothing else. A pixel becomes a direction; a direction becomes a
//! cosine; the cosine indexes a table that `voxel_appearance::sky::Sky::dome_radiance`
//! filled on the CPU. There is no Rayleigh coefficient, no Mie asymmetry, no phase
//! function and no air mass anywhere below.
//!
//! The reason is drift. Writing Rayleigh and Mie a second time here would put two
//! copies of one equation in a repository whose own `climate.rs` refuses to *store* a
//! second record of a derived truth because "two records drift". Two copies would drift.
//! So the model exists once, in Rust, and this file is told what it says.
//!
//! # Why the pixels are alpha 0
//!
//! Alpha keeps the meaning `world.rs` gives it — **"geometry is drawn here"** — so the
//! frame can still tell a surface from the sky behind it. A sky pass that wrote alpha 1
//! would be indistinguishable from terrain to every measurement that reads the alpha,
//! and `Frame::response` would then be counting sky as if it were shaded stone.
//!
//! The invariant this establishes: **every pixel is either sky or geometry, and none is
//! left undrawn.** That is the whole repair — the old pass cleared to transparent, so
//! every pixel with no voxel on it was nothing at all.

// The camera's projection, and the table's length. `pitch_rad` and
// `tan_half_fov` are the only two numbers the shader needs to turn a pixel into a
// direction; everything physical arrives in `table`.
struct SkyView {
    pitch_rad: f32,
    tan_half_fov: f32,
    rows: f32,
    pad: f32,
};

@group(0) @binding(0) var<uniform> view: SkyView;
// vec4 rather than vec3 only because a storage array's stride is 16 bytes here.
@group(0) @binding(1) var<storage, read> table: array<vec4<f32>>;

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) ndc_y: f32,
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
    out.ndc_y = xy.y;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    // The one piece of arithmetic this file owns: where this pixel is pointing.
    // With no camera roll, elevation depends on the row alone, so the pitch and the
    // half-angle are all the geometry there is.
    let elevation = view.pitch_rad + atan(in.ndc_y * view.tan_half_fov);
    let cos_zenith = sin(elevation);

    // Below the horizon the table stops — the model is defined from the horizon up, and
    // empty space under the world is a void rather than somewhere the sky has an
    // opinion. Clamping says that; extrapolating would invent a colour.
    let c = clamp(cos_zenith, 0.0, 1.0);
    let rows = u32(view.rows);
    let last = f32(rows - 1u);
    let at = c * last;
    let lower = floor(at);
    let i = min(u32(lower), rows - 2u);
    let frac = at - lower;

    // The same interpolation `skylight::sky_lut_lookup` performs, so the two can be
    // compared by a test rather than assumed to match.
    let radiance = mix(table[i].rgb, table[i + 1u].rgb, frac);

    // Alpha 0: sky, not geometry. See the module doc.
    return vec4<f32>(radiance, 0.0);
}
