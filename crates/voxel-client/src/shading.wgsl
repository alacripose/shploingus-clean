// The Valve Source 2 stylisation, as `voxel-appearance`'s `npr::shade` states
// it -- written a second time, in WGSL, because a shader cannot call Rust.
//
// This file is a *transcription*, and it is held to the original by
// `renderer::tests::the_gpu_shades_a_surface_exactly_as_the_crate_does`, which
// renders stated normals and compares the pixels against `npr::shade`. The
// branch order, the exact expressions, and the `_present` flags are all copied
// deliberately: a "nicer" formulation here would be a second model, and two
// models is how shading drifts.
//
// The one line that must NOT be simplified: `if (base_present > 0.5)`. An
// absent albedo removes the diffuse lobe. Reading it as black instead would
// change `Terms`, and `Terms` is what the FMA bar's band count is reported
// against.

struct Light {
    direction: vec4<f32>,   // xyz toward the light, w intensity
    colour: vec4<f32>,
};

struct Rig {
    ambient: vec4<f32>,
    count: vec4<u32>,       // x = how many of `lights` are live
    lights: array<Light, 4>,
};

struct View {
    vp: mat4x4<f32>,
};

struct Material {
    base_color_and_present: vec4<f32>,
    rough_metal: vec4<f32>,
    spec_emission: vec4<f32>,
    emission_rgb_open: vec4<f32>,
};

@group(0) @binding(0) var<uniform> rig: Rig;
@group(0) @binding(1) var<uniform> camera: View;
@group(0) @binding(2) var<storage, read> materials: array<Material>;

struct VsIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) material: u32,
};

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) to_eye: vec3<f32>,
    @location(2) @interpolate(flat) material: u32,
};

@vertex
fn vs_main(in: VsIn) -> VsOut {
    var out: VsOut;
    out.clip = camera.vp * vec4<f32>(in.position, 1.0);
    out.normal = in.normal;
    // Geometry arrives already relative to the eye, so the eye is the origin
    // here and the view vector is just the way back to it. No uniform, and so
    // no opportunity for a uniform to disagree with the geometry.
    out.to_eye = -in.position;
    out.material = in.material;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    if (in.material >= arrayLength(&materials)) {
        // A quad naming a material outside the table is not shaded at all. It
        // is not shaded grey: no admitted input means no picture.
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    let m = materials[in.material];
    let n = normalize(in.normal);
    let v = normalize(in.to_eye);
    let ndotv = clamp(dot(n, v), 0.0, 1.0);

    let base = m.base_color_and_present.xyz;
    let base_present = m.base_color_and_present.w;
    let roughness = m.rough_metal.x;
    let rough_present = m.rough_metal.y;
    let metalness = m.rough_metal.z;
    let transmission = m.rough_metal.w;
    let weight = m.spec_emission.x;
    let weight_present = m.spec_emission.y;
    let emission_luminance = m.spec_emission.z;
    let emission_present = m.spec_emission.w;
    let emission_rgb = m.emission_rgb_open.xyz;
    let open = m.emission_rgb_open.w;

    var colour = vec3<f32>(0.0);

    // 4. Ambient, first, because it is the term that survives when little else
    //    does. A transmissive surface receives light from every direction, and
    //    a face with holes receives less of everything.
    if (base_present > 0.5) {
        let gain = (1.0 + transmission) * (1.0 - open);
        colour += base * (rig.ambient.rgb * gain);
    }

    let count = rig.count.x;
    for (var i = 0u; i < count; i += 1u) {
        let light = rig.lights[i];
        let toward_light = normalize(light.direction.xyz);
        let intensity = light.direction.w;
        let ndotl = dot(n, toward_light);

        // 1. Half-Lambert diffuse wrap.
        if (base_present > 0.5) {
            let wrap = ndotl * 0.5 + 0.5;
            let lambert = wrap * intensity * (1.0 - metalness) * (1.0 - transmission);
            colour += base * (light.colour.rgb * lambert);
        }

        // 2. Rim: Schlick's exponent, view-facing.
        if (rough_present > 0.5) {
            let fresnel = pow(1.0 - ndotv, 5.0);
            let rim = fresnel * (1.0 - roughness) * intensity;
            colour += light.colour.rgb * rim;
        }

        // 3. The specular warp: a step, and the threshold is the material's own
        //    roughness rather than a number this file picked.
        if (rough_present > 0.5 && weight_present > 0.5) {
            let half_v = normalize(toward_light + v);
            let facing = clamp(dot(n, half_v), 0.0, 1.0);
            if ((1.0 - facing) <= roughness) {
                colour += light.colour.rgb * (weight * intensity);
            }
        }
    }

    // Emission is not a Source 2 term. It is the OpenPBR input a dynamic block
    // emitter carries, and it is added last because it depends on no light.
    if (emission_present > 0.5) {
        colour += emission_rgb * emission_luminance;
    }

    return vec4<f32>(colour, 1.0);
}
