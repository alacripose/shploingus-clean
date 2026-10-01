// Panels, rules and any other flat surface the HUD needs.
//
// # Why this exists at all
//
// `glyphon` draws glyphs and nothing else. A HUD is glyphs **on a surface** —
// and the surface is not decoration: `voxel-ui` derives its whole ink ladder
// from the contrast between ink and the chrome panel behind it, so drawing the
// text without the panel would be drawing a HUD whose legibility claim is about
// a colour that is not on screen. The panel is a fragment shader and two
// triangles, which is cheaper than being wrong.
//
// # Coordinates and colour
//
// Positions arrive already in clip space (`-1..1`, y up) because the rectangles
// are laid out in device-independent pixels by the CPU side and converted once,
// in one place. Colours arrive **linear** — the same thing `shading.wgsl`
// outputs — because the target is either an sRGB swapchain (where the hardware
// encodes on write) or a float texture in the offscreen test (where it is read
// back and compared in linear, as `voxel-appearance`'s luminance expects).
//
// # The radius
//
// MD1 says a corner radius is 2 dp. A rectangle with a hard corner and a comment
// claiming otherwise is the kind of thing that survives review, so the radius is
// computed: the fragment shader has the local offset, the half extent and the
// radius, and returns the coverage of a rounded rectangle. `radius = 0` gives a
// hard edge, which is what a 1 dp rule wants.

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) colour: vec4<f32>,
    // Drawn pixels from the rect's centre, in pixels.
    @location(1) local: vec2<f32>,
    @location(2) half: vec2<f32>,
    // Flat is unnecessary: every vertex of a rect carries the rect's own radius,
    // so the interpolated value is that radius everywhere.
    @location(3) radius: f32,
};

@vertex
fn vs(
    @location(0) position: vec2<f32>,
    @location(1) colour: vec4<f32>,
    @location(2) local: vec2<f32>,
    @location(3) half: vec2<f32>,
    @location(4) radius: f32,
) -> VertexOut {
    var out: VertexOut;
    out.clip = vec4<f32>(position, 0.0, 1.0);
    out.colour = colour;
    out.local = local;
    out.half = half;
    out.radius = radius;
    return out;
}

@fragment
fn fs(in: VertexOut) -> @location(0) vec4<f32> {
    // A signed distance to the rounded rectangle, in pixels. Negative inside.
    let r = min(in.radius, min(in.half.x, in.half.y));
    let q = abs(in.local) - (in.half - vec2<f32>(r, r));
    let outside = length(max(q, vec2<f32>(0.0, 0.0)));
    let inside = min(max(q.x, q.y), 0.0);
    let distance = outside + inside - r;

    // One pixel of feathering, so the corner is antialiased rather than jagged.
    let coverage = 1.0 - smoothstep(-0.5, 0.5, distance);
    if (coverage <= 0.0) {
        discard;
    }
    return vec4<f32>(in.colour.rgb, in.colour.a * coverage);
}
