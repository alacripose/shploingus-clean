// The far ring's own fragment stage — LOD-0001.
//
// This file belongs to the package, not to the renderer: `shading.wgsl` is the look every window
// shares and a layer may not write it (C-33, AR15). What a layer may bring is a pipeline, a bind
// group, an entry point and uniforms, which is exactly what this file declares.
//
// The uniform block is `LOD-0001.layer`'s, in its declaration order:
//   view_proj    mat4<f32>   64 bytes
//   rung         u32          4 bytes
//   rung_colour  vec4<f32>   16 bytes

struct Horizon {
    view_proj: mat4x4<f32>,
    rung: u32,
    rung_colour: vec4<f32>,
};

@group(1) @binding(0) var<uniform> horizon: Horizon;

struct HorizonVertex {
    @location(0) position: vec3<f32>,
    @location(1) colour: vec3<f32>,
};

struct HorizonFragment {
    @builtin(position) position: vec4<f32>,
    @location(0) colour: vec3<f32>,
};

@vertex
fn vs_horizon(vertex: HorizonVertex) -> HorizonFragment {
    var out: HorizonFragment;
    out.position = horizon.view_proj * vec4<f32>(vertex.position, 1.0);
    out.colour = vertex.colour;
    return out;
}

@fragment
fn fs_horizon(fragment: HorizonFragment) -> @location(0) vec4<f32> {
    // The rung lifts the far terrain by its own index rather than by a constant in the renderer:
    // the distance is the package's declaration, and the shader reads the record it was handed.
    let lift = f32(horizon.rung) * 0.0009;
    let colour = mix(fragment.colour, horizon.rung_colour.rgb, 0.35);
    return vec4<f32>(colour * (1.0 + lift), 1.0);
}
