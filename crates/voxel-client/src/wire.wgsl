// The world meshes' own wireframe: the greedy merge's rectangles, outlined.
//
// This is a *debug view of the geometry*, not a second shading model. While it
// is on the world's own materials are withheld (the world draw is skipped), so
// the frame is the sky and these lines, and the gradient is the depth cue.
//
// The user's own call (2026-10-04): a **rainbow gradient by depth** rather than
// the first white ink, which read poorly against the world, and the world
// materials hidden so the merge's rectangles are the subject. The hue is the
// artists' wheel at full saturation and value, near red sweeping to far blue.
//
// The outlines arrive as a `LineList` built by `Scene::quad_edge_bytes`: four
// border edges per merged rectangle and no diagonal, so what is drawn is the
// merge's answer rather than the triangulation's.

struct View {
    vp: mat4x4<f32>,
};

@group(0) @binding(1) var<uniform> camera: View;

// The gradient's declared range and sweep: hue NEAR_HUE at NEAR_M, FAR_HUE at FAR_M.
const NEAR_M: f32 = 4.0;
const FAR_M: f32 = 256.0;
const NEAR_HUE: f32 = 0.0;
const FAR_HUE: f32 = 0.7;

/// The artists' wheel at full saturation and value, as a closed form: red at 0,
/// through yellow, green and cyan, to blue near 2/3. (Inigo Quilez's polynomial,
/// the same one every shader palette uses; written out rather than sampled from
/// a table because there is no table to sample.)
fn hue_rgb(h: f32) -> vec3<f32> {
    let x = h * 6.0 + vec3<f32>(0.0, 4.0, 2.0);
    let k = x - 6.0 * floor(x / 6.0);
    return clamp(abs(k - 3.0) - 1.0, vec3<f32>(0.0), vec3<f32>(1.0));
}

struct VsIn {
    @location(0) position: vec3<f32>,
};

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) to_eye: vec3<f32>,
};

@vertex
fn vs_main(in: VsIn) -> VsOut {
    var out: VsOut;
    out.clip = camera.vp * vec4<f32>(in.position, 1.0);
    // Geometry arrives already relative to the eye, so the eye is the origin
    // here and the view distance is the length of the position itself — the
    // same fact `shading.wgsl` relies on, and no uniform to disagree with it.
    out.to_eye = -in.position;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let distance = length(in.to_eye);
    let t = clamp((distance - NEAR_M) / (FAR_M - NEAR_M), 0.0, 1.0);
    return vec4<f32>(hue_rgb(mix(NEAR_HUE, FAR_HUE, t)), 1.0);
}
