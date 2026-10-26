// The witness's own probe stage — `sun_shadow`'s `the_cascade_seam_is_a_ramp` test, and nothing
// else. It draws no world: the fragment addresses a lattice of points on a ground plane by its
// own pixel, evaluates the **shipped** `sun_visibility` there, and puts the law it replaced beside
// it. Both read the same uniform and the same two depth maps, drawn in the same pass, so the two
// numbers differ by the handover and by nothing else.
//
// `x` is the level a step-selector answers — one cascade chosen by `split.x`, no band — and `y`
// the shipped function's. `z` is the sample's distance from the eye and `w` the far cascade's
// share, so the report can say where the ramp is and how wide.

struct Probe {
    // `(x from, x step, distance from, distance step)`, metres, eye-relative.
    grid: vec4<f32>,
    // `x` is the ground plane's height in eye-relative space — negative, the eye is above it.
    ground: vec4<f32>,
};
@group(1) @binding(0) var<uniform> probe: Probe;

// A full-screen triangle: three vertices, two of them past the frame, which is the cheapest way
// to cover a target without a vertex buffer.
@vertex
fn vs_probe(@builtin(vertex_index) at: u32) -> @builtin(position) vec4<f32> {
    let x = f32((at << 1u) & 2u);
    let y = f32(at & 2u);
    return vec4<f32>(x * 2.0 - 1.0, y * 2.0 - 1.0, 0.0, 1.0);
}

@fragment
fn fs_probe(@builtin(position) at: vec4<f32>) -> @location(0) vec4<f32> {
    // The lattice is addressed by pixel index, not by pixel centre: row zero is the nearest
    // distance and column zero the leftmost offset, and the report reads it back in that order.
    let col = floor(at.x);
    let row = floor(at.y);
    let x = probe.grid.x + probe.grid.y * col;
    let dist = probe.grid.z + probe.grid.w * row;
    let ground = probe.ground.x;
    // The point on the ground at that lateral offset and that distance from the eye. The sweep
    // keeps every sample inside both cascades' boxes, so the square root is over a positive
    // number by construction (`d >= 20`, `|x| <= 5`, eye 1.7 m up).
    let z = sqrt(max(dist * dist - x * x - ground * ground, 0.0));
    let rel = vec3<f32>(x, ground, z);
    let n = vec3<f32>(0.0, 1.0, 0.0);

    let near = cascade_lit(rel, n, false);
    let far = cascade_lit(rel, n, true);
    // **The law as it was**: one cascade, chosen by the split, with no band. This is the whole of
    // what the previous shader varied — the per-cascade arithmetic around it is `cascade_lit`.
    let before = select(near.x, far.x, dist > beam_shadow.split.x);
    // **The law as it is**: the shipped function, unmodified.
    let after = sun_visibility(rel, n);

    let far_texel_m = 2.0 * beam_shadow.cascade.y / beam_shadow.cascade.w;
    let band = SHADOW_BLEND_TEXELS * far_texel_m;
    let share = smoothstep(beam_shadow.split.x - band, beam_shadow.split.x + band, dist);
    return vec4<f32>(before, after, dist, share);
}
