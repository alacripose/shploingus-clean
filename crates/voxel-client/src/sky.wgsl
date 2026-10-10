//! The sky pass — **the scattering is not in this file, on purpose**.
//!
//! # What this shader is allowed to know
//!
//! Geometry, and nothing else. A pixel becomes a **world ray** (the camera's yaw and pitch
//! travel in `view`), the ray becomes an elevation and a world azimuth, and the pair indexes
//! a table that `voxel_appearance::sky::Sky::directional_radiance` filled on the CPU. There
//! is no Rayleigh coefficient, no Mie asymmetry, no phase function and no air mass anywhere
//! below.
//!
//! The reason is drift. Writing Rayleigh and Mie a second time here would put two copies of
//! one equation in a repository whose own `climate.rs` refuses to *store* a second record of
//! a derived truth because "two records drift". Two copies would drift. So the model exists
//! once, in Rust, and this file is told what it says.
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
//! # The Sun's body is drawn here, and nothing about it is computed here
//!
//! Each pixel's ray is tested against the Sun's **cuboid** and the body's own radiance is added
//! where it meets one (C-176). What arrives is a shape and a number: the half-extent, the centre
//! measured from the eye, and a radiance the CPU derived — over the body's own solid angle, so
//! the **irradiance** the light rig carries is not mistaken for the radiance a drawn body has.
//! Which means the pass still does no physics. A second derivation of the solar constant, or of
//! the solid angle, would be the drifting copy this file exists to refuse.

// The camera's geometry, and the table's grid. `pitch_rad`, `yaw_rad` and `tan_half_fov`
// are the only numbers needed to turn a pixel into a world direction; everything physical
// arrives in `table`.
struct SkyView {
    pitch_rad: f32,
    tan_half_fov: f32,
    yaw_rad: f32,
    aspect: f32,
    azimuths: f32,
    elevations: f32,
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
    // **Declared, unread, and load-bearing.** They make this struct twelve `f32`: a
    // whole number of `vec4`s, which is the size `WorldPipeline::write_camera`
    // uploads. A struct whose padding a reader has to work out is a struct whose size
    // can drift from the buffer it is bound to, which is the defect that made the sky
    // black in the first place.
    reserved0: f32,
    reserved1: f32,
    // **The Sun's body** (C-176): its centre measured **from the eye**, and the half-extent of
    // its cube. A zero half-extent is how the pass is told there is no sun in this frame, which
    // is the state a caller who never wrote one is in.
    //
    // The offset is taken on the CPU in `f64`, because a world point at one astronomical unit
    // does not fit a 32-bit metre: see `sun::DrawnSun::centre_relative_to`.
    sun_centre_m: vec3<f32>,
    sun_half_extent_m: f32,
    // **The Sun's own radiance**, already through this pass's exposure like every other radiance
    // here, so the disc and the sky behind it share one scale. A *radiance* — power per square
    // metre per steradian — and not the irradiance the light rig carries; the CPU divides by the
    // body's own solid angle and `voxel-client::sun` says why.
    sun_radiance: vec3<f32>,
    // Declared, unread, and load-bearing the way `reserved0` is: it closes this struct's fifth
    // `vec4`, so the size is the twenty `f32` `WorldPipeline::write_camera` uploads.
    sun_reserved: f32,
};

/// Whether a ray from the eye meets the Sun's cuboid.
///
/// A **slab test**, and the same one `voxel_client::sun::SunCuboid::ray_hits` performs on the
/// CPU — written twice so a test can hold this transcription to that function rather than to a
/// description of it, exactly as the dome's bilinear lookup is.
///
/// The reciprocal is taken with a floor under the ray's own magnitude so a ray **parallel** to a
/// slab plane cannot produce the `0 · ∞` that would be a `NaN` in the comparisons; a parallel
/// axis then puts a positive infinity into the near bound, which no far bound can be greater
/// than, and the ray misses. The eye is never inside a body one astronomical unit away, so an
/// origin inside a slab is not a case this has to answer.
fn ray_hits_cuboid(ray: vec3<f32>, centre: vec3<f32>, half_extent: f32) -> bool {
    let magnitude = max(abs(ray), vec3<f32>(1.0e-30));
    let sign = select(vec3<f32>(-1.0), vec3<f32>(1.0), ray >= vec3<f32>(0.0));
    let inv = sign / magnitude;
    let lo = (centre - vec3<f32>(half_extent)) * inv;
    let hi = (centre + vec3<f32>(half_extent)) * inv;
    let near = max(max(min(lo.x, hi.x), min(lo.y, hi.y)), min(lo.z, hi.z));
    let far = min(min(max(lo.x, hi.x), max(lo.y, hi.y)), max(lo.z, hi.z));
    return far >= max(near, 0.0);
}

@group(0) @binding(0) var<uniform> view: SkyView;
// vec4 rather than vec3 only because a storage array's stride is 16 bytes here. Row-major:
// `elevation * azimuths + azimuth`, elevation from the horizon up, azimuth from north
// eastwards.
@group(0) @binding(1) var<storage, read> table: array<vec4<f32>>;

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

const DEGREES_PER_RADIAN: f32 = 57.29577951308232;


@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let azimuths = u32(view.azimuths);
    let elevations = u32(view.elevations);
    // A zeroed uniform — the pass has not been given a sky yet — draws black, the same
    // thing this pass did before the sky existed. Nothing indexes an empty grid.
    if (azimuths < 2u || elevations < 2u) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    // The camera basis, matched to `Camera::basis`: yaw about +Y, pitch about the right
    // axis, no roll (the sky does not roll with the drawn camera).
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

    // The world frame is east-north-up with `x` east and `z` south (the same convention
    // `voxel_shapers::sky` and `skylight::rig_of` use), so the azimuth is `atan2(x, −z)`.
    let elevation = clamp(asin(clamp(dir.y, -1.0, 1.0)) * DEGREES_PER_RADIAN, 0.0, 90.0);
    var azimuth = atan2(dir.x, -dir.z) * DEGREES_PER_RADIAN;
    if (azimuth < 0.0) {
        azimuth = azimuth + 360.0;
    }

    // Bilinear over (elevation, azimuth) with the azimuth axis wrapped — the same
    // interpolation `skylight::sky_dome_lookup` performs on the CPU, so the two can be
    // compared by a test rather than assumed to match.
    let ef = elevation / 90.0 * (view.elevations - 1.0);
    let e0 = min(u32(floor(ef)), elevations - 2u);
    let e1 = e0 + 1u;
    let efrac = ef - floor(ef);
    let af = azimuth / 360.0 * view.azimuths;
    let a0 = u32(floor(af)) % azimuths;
    let a1 = (a0 + 1u) % azimuths;
    let afrac = af - floor(af);
    let row0 = e0 * azimuths;
    let row1 = e1 * azimuths;
    let low = mix(table[row0 + a0].rgb, table[row0 + a1].rgb, afrac);
    let high = mix(table[row1 + a0].rgb, table[row1 + a1].rgb, afrac);
    let radiance = mix(low, high, efrac);

    // **Transform first, then dither**, for the reason `shading.wgsl` gives: the
    // dither is about the steps that remain *after* the range is settled, so
    // dithering a value that is about to be rescaled dithers something that is not
    // on screen. Both are gated on the target, so a measurement frame is the
    // radiance the model computed.
    //
    // The dither is applied to the colour and **not to the alpha**: alpha is this
    // pass's "geometry is here" channel and its value is a fact, not a shade — a
    // dithered alpha would read as a half-covered pixel that no geometry made.
    //
    // **The Sun's body, composited onto the sky's own scattering** (C-176). A line of sight to
    // the Sun carries the air's in-scattered light *and* the body's radiance, and the radiance
    // arrived already transmitted, so the two are added rather than one replacing the other.
    //
    // The test is against the **ray**, not against a direction: what is drawn is the cuboid, so
    // its silhouette is a square when a face is presented to the eye and turns toward a hexagon
    // as the body crosses the sky. A ray that misses the body adds nothing, which is also what a
    // frame with no sun written does — the uniform's half-extent is zero.
    //
    // Added **before** the display transform, for the reason the dither is applied after it: the
    // transform is a function of the pixel's luminance, so a sum that skipped the body would be
    // a different colour than the one the model computed.
    var lit = radiance;
    if (view.sun_half_extent_m > 0.0
        && ray_hits_cuboid(dir, view.sun_centre_m, view.sun_half_extent_m)) {
        lit = lit + view.sun_radiance;
    }

    // Alpha 0: sky, not geometry. See the module doc — the Sun is drawn here and is still not
    // geometry, so alpha stays the one channel that says "a mesh is on this pixel".
    var presentable = lit;
    if (view.tonemap > 0.0) {
        presentable = tone_map(lit);
    }
    return vec4<f32>(dithered(presentable, in.position.xy, view.dither), 0.0);
}
