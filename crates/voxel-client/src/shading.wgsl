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
    // The eye's fractional metre offset, per axis (`pattern::eye_phase`). The
    // geometry arrives eye-relative, so this plus `eye_voxel` is what lets a
    // fragment place itself in the world without a world-space `f32` — which at
    // two million metres could not resolve a block, let alone a lattice cell.
    eye_phase: vec4<f32>,
    // The eye's own world voxel, `floor(eye)` per axis (`pattern::eye_voxel`) —
    // the integer half of the same split. A fragment's world subvoxel cell is
    // `floor((p_rel + phase) * 4) + eye_voxel * 4` (C-162).
    eye_voxel: vec4<i32>,
    // The lattice's identity, from Rust: seed (lo, hi), then domain tag (lo, hi).
    // `u32` pairs because WGSL has no `u64`; both travel from
    // `voxel-appearance::provenance` and the world's `ShadingField` rather than
    // being written a second time here.
    lattice: vec4<u32>,
    // The two sides, in samples per axis: the 4×4×4 subvoxel grid and the 256
    // lattice per subvoxel axis (`voxel_model::coord`). The shader derives its cell
    // sizes — and the ladder's rungs — from these instead of stating metres.
    rates: vec4<u32>,
    // `x` is one pixel's angular size in radians (`pattern::pixel_angle_rad`), the
    // scale the ladder's footprint is measured with; `y` is how much dither the
    // target's own quantisation calls for (C-166), `0` for a float target and `1`
    // for the 8-bit `sRGB` surface the window presents; `z` is whether this target
    // is a **display**, and so whether a scene-referred value is compressed into
    // one it can present (C-167), `0` for the offscreen measurement target. `w`
    // stays reserved. This is a viewport number rather than a world one, so it
    // travels with the camera.
    //
    // **The two are separate flags on purpose, though the same target sets both
    // today.** Dithering is a statement about how many steps a target has; the
    // display transform is a statement about how much range it has. An HDR display
    // is a display that quantises nothing, and a future frame that renders into a
    // float intermediate before presenting would set this one on the *present* pass
    // and neither on the world pass. One flag would have to be split then, and
    // splitting it later is how two meanings come to share one bit.
    screen: vec4<f32>,
    // **The dome table's own grid, and the water boundary's reflectance** (C-177): `x`
    // azimuths, `y` elevations, `z` `voxel_appearance::fluid::WATER_F0`, `w` reserved.
    //
    // The world pass reads the same table the sky pass draws — a fluid surface *is* the
    // sky, seen off a dielectric boundary — and the two numbers it indexes with are
    // `skylight::SKY_DOME_AZIMUTHS`/`SKY_DOME_ELEVATIONS`, sent rather than typed here for
    // the reason `lattice` and `rates` are. The reflectance likewise: `fluid::schlick` is
    // called with `WATER_F0` on the CPU, and a shader holding its own `0.0204` would be a
    // second statement of the water's boundary.
    sky_grid: vec4<f32>,
};

struct Material {
    base_color_and_present: vec4<f32>,
    rough_metal: vec4<f32>,
    spec_emission: vec4<f32>,
    emission_rgb_open: vec4<f32>,
    // The pattern's gains (C-161): how much of the 256-level lattice's deviation
    // this material shows — `.x` roughness, `.y` reflectance. Zero for a material
    // the world's shading field does not vary, and the two reserved slots are its
    // room to grow without moving a stride.
    pattern: vec4<f32>,
    // ---- the parameters that reached no group before (C-176) -------------------
    //
    // `openpbr::derive` computes all of these; `npr::shade` reads all of them; and
    // until this group existed the shader saw none of them. Field-for-field with
    // `voxel_client::shading::MaterialParams`, the same rule every other slot
    // follows: **a value travels with its own presence flag**, because an absent
    // parameter is not a zero.
    // `specular_f0`, whether it is admitted, `coat_weight`, `coat_roughness`.
    specular_boundary: vec4<f32>,
    // `coat_roughness admitted`, `transmission_depth`, `depth admitted`,
    // `subsurface_weight`.
    participating_medium: vec4<f32>,
    // `subsurface_weight admitted`, and the room the next admitted parameter takes.
    medium_flags: vec4<f32>,
};

@group(0) @binding(0) var<uniform> rig: Rig;
@group(0) @binding(1) var<uniform> camera: View;
@group(0) @binding(2) var<storage, read> materials: array<Material>;
// ---- the quad format (C-168) ------------------------------------------------
//
// The world's rectangles arrive as themselves — 24 chunk-local bytes each — and
// this stage expands one into two triangles. The alternative, which is what this
// replaced, is four 64-byte corners and six indices built on the CPU, **every
// frame, for the whole resident window**: measured on the shipped world, 57,673
// quads came to 15.40 MB of vertex and index data re-created and re-uploaded per
// frame, in 24.22 ms — 145 % of a 60 Hz frame, for geometry that did not move.
//
// The expansion is a **transcription of `voxel_mesh::Quad::corners`**, and it is
// held to it by `renderer::tests::the_quad_format_and_the_vertex_format_draw_the_same_surface`,
// which draws the same surface through this stage and through `vs_main` and
// compares the pixels. The winding is carried exactly as `mesh::winding_flips`
// states it, because culling is off today (see `world.rs`) and the law should not
// be the thing that changes when it is turned on.
struct QuadRecord {
    face: u32,      // Face::ALL's own order: 0 NegX, 1 PosX, 2 NegY, 3 PosY, 4 NegZ, 5 PosZ
    // The rectangle's per-face material composition (C-172): the face blend of the
    // quad's rectangle selection, packed so the fragment can rebuild it from the two
    // most-covering materials the selection carries. A single-material selection is
    // one material in the low slot and zero in the high slot.
    composition: u32,
    slice: u32,     // the plane along the face's axis, in subvoxels from the chunk's low corner
    uv: u32,        // uv.x in the low 16 bits, uv.y in the high
    extent: u32,    // extent.x in the low 16 bits, extent.y in the high
    chunk: u32,     // an index into `origins`
};

@group(0) @binding(3) var<storage, read> quads: array<QuadRecord>;
// One chunk's origin **relative to the eye**, in metres, computed on the CPU in
// `f64` and narrowed only afterwards — the same decision `scene.rs` makes for a
// vertex, and the reason a rectangle can be reused across frames while a vertex
// cannot. A `vec4` because a storage array's stride is 16 bytes.
@group(0) @binding(4) var<storage, read> origins: array<vec4<f32>>;
// **The sky pass's own table, bound to this pass too** (C-177). Same buffer, same
// row-major grid (elevation * azimuths + azimuth), same bilinear read — a fluid reflects
// the sky that is overhead, and the only way for that to be true by construction rather
// than by agreement is for both passes to read one buffer.
@group(0) @binding(5) var<storage, read> sky_table: array<vec4<f32>>;

struct VsIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) material: u32,
    // The material's detail at this corner's world address: the 256-level lattice
    // sampled once, as roughness_delta, reflectance_delta. The fragment shader's
    // ladder uses this as the coarsest rung's base (C-161).
    @location(3) detail_roughness: f32,
    @location(4) detail_reflectance: f32,
    // The world's light at this corner's cell, in band order (B3, B4, B5):
    // direct beam, then diffuse (sky + one-bounce), W/m². Read only where
    // `light_present` is 1 — see that field, and `to_channels` below, which is
    // what turns a band triple into the albedo's own (r, g, b) order.
    @location(5) light_direct: vec3<f32>,
    @location(6) light_diffuse: vec3<f32>,
    // **Whether the world's light field covers this corner at all**: 1 when the
    // shaper layer gave a measurement, 0 when it had none. A flag and not a
    // sentinel, because zero W/m² is a real answer — a cell at night or in shadow
    // is measured as zero — and a lit surface must not be handed back to the rig
    // just because the beam that reached it was nothing.
    @location(7) light_present: u32,
};

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) to_eye: vec3<f32>,
    @location(2) @interpolate(flat) material: u32,
    // The eye-relative position, interpolated: the fragment needs its own place in
    // the world's block grid, and this is it (see `eye_phase`).
    @location(3) eye_rel: vec3<f32>,
    // Per-vertex detail, interpolated: the coarsest rung's contribution from the
    // lattice sample at the corner's world address (C-161).
    @location(4) detail_roughness: f32,
    @location(5) detail_reflectance: f32,
    // Per-vertex light, interpolated: the world's beam and diffuse at the corner's
    // cell, in band order (B3, B4, B5).
    @location(6) light_direct: vec3<f32>,
    @location(7) light_diffuse: vec3<f32>,
    // Flat, because "is there a field here" is a property of the corner's cell and
    // not a quantity to blend: a fragment half-covered by the field would get a
    // half-present light, which is a third state nobody declared.
    @location(8) @interpolate(flat) light_present: u32,
    // **The cloud's veil** (C-178/C-179): the fraction of the beam this cell's own column
    // passes, from `cloud_cubes` through the origin's `w`. `0` for every surface, which is
    // terrain — the world never wrote that float before, so nothing that was drawn before
    // clouds existed can read as one.
    //
    // Flat, for the reason `light_present` is: it is a property of the cell and not a
    // quantity to blend, and every corner of one cell carries the same one anyway.
    @location(9) @interpolate(flat) veil: f32,
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
    out.eye_rel = in.position;
    // Pass through the per-vertex detail and light: the fragment's LOD ladder
    // uses these as the base, reading finer rungs from the fragment's own
    // footprint (C-161).
    out.detail_roughness = in.detail_roughness;
    out.detail_reflectance = in.detail_reflectance;
    out.light_direct = in.light_direct;
    out.light_diffuse = in.light_diffuse;
    out.light_present = in.light_present;
    // **A corner is never a cloud.** The vertex format carries a light measurement, and a
    // measurement attaches to a cell of the world rather than to a medium, so this path is
    // terrain by construction and its veil is zero.
    out.veil = 0.0;
    return out;
}

// ---- the quad format's expansion (C-168) -------------------------------------
//
// Every law below is `voxel_mesh`'s, transcribed: `Face::ALL`'s order and the
// three accessors (`normal`, `axis`, `is_positive`), `mesh::axes`, and
// `mesh::winding_flips` (`is_positive != (axis != 1)`). They are written as
// functions of the discriminant rather than as tables so that the one number
// that has to agree with Rust — the face's own index — is the only thing that
// can disagree.

fn face_axis(face: u32) -> u32 {
    if (face == 0u || face == 1u) { return 0u; }
    if (face == 2u || face == 3u) { return 1u; }
    return 2u;
}

fn face_normal(face: u32) -> vec3<f32> {
    if (face == 0u) { return vec3<f32>(-1.0, 0.0, 0.0); }
    if (face == 1u) { return vec3<f32>(1.0, 0.0, 0.0); }
    if (face == 2u) { return vec3<f32>(0.0, -1.0, 0.0); }
    if (face == 3u) { return vec3<f32>(0.0, 1.0, 0.0); }
    if (face == 4u) { return vec3<f32>(0.0, 0.0, -1.0); }
    return vec3<f32>(0.0, 0.0, 1.0);
}

/// `Quad::corners` reverses the four corners when the winding has to flip.
fn winding_flips(face: u32) -> bool {
    let positive = face == 1u || face == 3u || face == 5u;
    return positive != (face_axis(face) != 1u);
}

@vertex
fn vs_quad(@builtin(vertex_index) vertex_index: u32) -> VsOut {
    let quad = quads[vertex_index / 6u];
    // The two triangles `Scene` used to emit as `[0, 1, 2, 0, 2, 3]`.
    var order = array<u32, 6>(0u, 1u, 2u, 0u, 2u, 3u);
    var corner = order[vertex_index % 6u];
    if (winding_flips(quad.face)) {
        corner = 3u - corner;
    }
    let low = vec2<u32>(quad.uv & 0xFFFFu, quad.uv >> 16u);
    let size = vec2<u32>(quad.extent & 0xFFFFu, quad.extent >> 16u);
    // The corner in the face's own `(u, v)` basis, in subvoxels.
    let u = low.x + select(0u, size.x, corner == 1u || corner == 2u);
    let v = low.y + select(0u, size.y, corner == 2u || corner == 3u);
    // `mesh::cell(axis, slice, u, v)`, spelled out per axis: the three cases are
    // `mesh::axes`'s own table (u = y,v = z · u = x,v = z · u = x,v = y).
    let axis = face_axis(quad.face);
    let plane = f32(quad.slice);
    let su = f32(u);
    let sv = f32(v);
    var sub = vec3<f32>(0.0);
    if (axis == 0u) {
        sub = vec3<f32>(plane, su, sv);
    } else if (axis == 1u) {
        sub = vec3<f32>(su, plane, sv);
    } else {
        sub = vec3<f32>(su, sv, plane);
    }
    // Subvoxels to metres, from the rate the camera already carries — one metre a
    // voxel, `SUBVOXEL_SIDE` cells across it — and then the chunk's own origin.
    let p = sub / f32(camera.rates.x) + origins[quad.chunk].xyz;

    var out: VsOut;
    out.clip = camera.vp * vec4<f32>(p, 1.0);
    out.normal = face_normal(quad.face);
    // The eye is the origin here, exactly as in `vs_main`: the geometry arrived
    // already relative to it.
    out.to_eye = -p;
    // The fragment reads the composition here, not the quad's own material id.
    // The quad's own material is the selected one, so for a single-material quad
    // this is the same id the fragment shades from; for a genuinely mixed selection
    // The fragment reads the composition here, not the quad's own material id.
    // The quad's own material is the selected one, so for a single-material quad
    // this is the same id the fragment shades from; for a genuinely mixed selection
    // the composition is the face blend, and the shader blends the selected materials
    // instead of looking up one id's row.
    out.material = quad.composition & 0xFFFFu;
    out.eye_rel = p;
    // **No per-vertex lattice sample and no per-vertex light.** A record has no
    // room for either, and neither is lost: the fragment's own ladder is the
    // whole address law for this geometry (C-161..C-164), and a light field is a
    // per-cell measurement, which is what `Scene::build_with_light`'s vertex
    // format exists to carry. `light_present = 0` is therefore not "a field of
    // zero here" but "this format carries no field", which is the rig path —
    // what every frame was lit by before a field existed.
    out.detail_roughness = 0.0;
    out.detail_reflectance = 0.0;
    out.light_direct = vec3<f32>(0.0);
    out.light_diffuse = vec3<f32>(0.0);
    out.light_present = 0u;
    // **The veil rides the origin** (C-179). A cloud cell's own origin carries the
    // transmittance of the column it stands in, in the one float of that `vec4` the world
    // never used, so the veil needs no new field in the record and no new binding.
    out.veil = origins[quad.chunk].w;
    return out;
}

// ---- the pattern's own arithmetic (C-161, C-162, laddered by C-163) ----------
//
// A material's 256-level detail is **addressed, never stored** (C-2), so a
// greedy-merged rectangle can still show its blocks: the fragment's own **world
// subvoxel** addresses the lattice, its material's **shaper** shapes the noise
// into that material's pattern, and the result modulates the row the quad's
// material slot already fetched. No geometry is added — that is the whole point
// (C-122, C-91). The first form of this section addressed the subvoxel inside its
// *block*, which stamps the same pattern into every block (the eye-check's
// tiling); the world cell is the correction.
//
// The 256 level is the **live** one: this read is static today, but the address,
// the shaper and the rungs are exactly what a fluid will animate (its cells
// interpolated between addresses) while a solid's non-addressed properties stay
// this same static read — foam on the surface, caustics through the body, and a
// shaded edge where a material leaves its block (the user's ruling, recorded in
// `pattern.rs`).
//
// The rungs are the model's **ladder** (C-163): the same field read in cells of
// `2^octave`, each drawn only while the pixel's footprint is under half its cell,
// summed with fBm gains and normalised — so a surface keeps structure at the scale
// it can draw, and the flat patches of the retired placeholder tone are gone.
//
// This section is a **transcription** of `voxel-appearance/src/detail.rs` and
// `voxel-client/src/pattern.rs`, held to them by
// `renderer::tests::the_pattern_is_drawn_where_the_greedy_mesh_has_no_geometry`.
// WGSL has no `u64`, so splitmix64 travels as (low, high) `u32` pairs; every
// operation below is exact integer arithmetic, and the only difference from the
// Rust is `f32` versus `f64` at the very end.

fn u64_add(a: vec2<u32>, b: vec2<u32>) -> vec2<u32> {
    let lo = a.x + b.x;
    let carry = select(0u, 1u, lo < a.x);
    return vec2<u32>(lo, a.y + b.y + carry);
}

fn u64_shr(a: vec2<u32>, n: u32) -> vec2<u32> {
    return vec2<u32>((a.x >> n) | (a.y << (32u - n)), a.y >> n);
}

// The high 32 bits of `a * b`, from 16-bit partial products: WGSL has no widening
// multiply, and splitmix64 needs the high word of its own products.
fn u32_hi(a: u32, b: u32) -> u32 {
    let a0 = a & 0xFFFFu;
    let a1 = a >> 16u;
    let b0 = b & 0xFFFFu;
    let b1 = b >> 16u;
    let t = a0 * b0;
    let w0 = a0 * b1 + (t >> 16u);
    let w1 = a1 * b0 + (w0 & 0xFFFFu);
    let w2 = a1 * b1 + (w0 >> 16u);
    return w2 + (w1 >> 16u);
}

// The low 64 bits of `a * b`. The high words of the two cross products live above
// bit 64 and are dropped, which is exactly what a `u64` multiply keeps.
fn u64_mul(a: vec2<u32>, b: vec2<u32>) -> vec2<u32> {
    return vec2<u32>(a.x * b.x, u32_hi(a.x, b.x) + a.y * b.x + a.x * b.y);
}

// splitmix64's finaliser (`voxel_noise::rng::mix`).
fn mix64(z: vec2<u32>) -> vec2<u32> {
    var x = u64_add(z, vec2<u32>(0x7F4A7C15u, 0x9E3779B9u));
    x = u64_mul(x ^ u64_shr(x, 30u), vec2<u32>(0x1CE4E5B9u, 0xBF58476Du));
    x = u64_mul(x ^ u64_shr(x, 27u), vec2<u32>(0x133111EBu, 0x94D049BBu));
    return x ^ u64_shr(x, 31u);
}

// `voxel_noise::rng::hash3`: mix(mix(mix(seed) ^ x·C1) ^ y·C2 ^ z·C3).
fn hash3(seed: vec2<u32>, x: vec2<u32>, y: vec2<u32>, z: vec2<u32>) -> vec2<u32> {
    let c1 = vec2<u32>(0x6659FD93u, 0xD6E8FEB8u);
    let c2 = vec2<u32>(0x78BD642Fu, 0xA0761D64u);
    let c3 = vec2<u32>(0xA0B428DBu, 0xE7037ED1u);
    var h = mix64(seed);
    h = mix64(h ^ u64_mul(x, c1));
    h = mix64(h ^ u64_mul(y, c2) ^ u64_mul(z, c3));
    return h;
}

// `voxel_noise::rng::signed3`: a hashed address as a value in [-1, 1).
fn signed3(seed: vec2<u32>, x: vec2<u32>, y: vec2<u32>, z: vec2<u32>) -> f32 {
    let h = u64_shr(hash3(seed, x, y, z), 11u);
    // `(h >> 11) as f64 * 2⁻⁵³`, in f32: the high word carries 2⁻²¹ and the low
    // word 2⁻⁵³. The low term can still move the last bit of the sum, so it is
    // added rather than dropped.
    return (f32(h.y) * 4.76837158203125e-7 + f32(h.x) * 1.1102230246251565e-16) * 2.0 - 1.0;
}

// `DetailLattice::domain`: one (material, **world subvoxel**) pair's own lattice,
// folded through `mix64` rather than added, so neighbours do not share one. The
// cell is the world's, not the block's index — a block-local fold stamps the same
// pattern into every block (C-162) — and the three axes fold in the order the
// Rust does, each cell as 32 bits because WGSL has no 64-bit sign extension.
fn lattice_domain(seed: vec2<u32>, tag: vec2<u32>, material: u32, cell: vec3<i32>) -> vec2<u32> {
    let x = vec2<u32>(bitcast<u32>(cell.x), 0u);
    let y = vec2<u32>(bitcast<u32>(cell.y), 0u);
    let z = vec2<u32>(bitcast<u32>(cell.z), 0u);
    let folded = mix64(x ^ mix64(y ^ mix64(z)));
    let inner = mix64(vec2<u32>(material, 0u) ^ folded);
    let mid = mix64(tag ^ inner);
    return mix64(seed ^ mid);
}

// One channel of the lattice at one address: `channel` is 1 for roughness and 2
// for reflectance (`detail.rs`'s domain tags), `cell` is the world subvoxel, and
// `address` is `x | y << 8 | z << 16` — `x` is the fast axis.
fn lattice_sample(
    seed: vec2<u32>,
    tag: vec2<u32>,
    material: u32,
    cell: vec3<i32>,
    address: u32,
    channel: u32,
) -> f32 {
    let domain = lattice_domain(seed, tag, material, cell);
    return signed3(domain, vec2<u32>(channel, 0u), vec2<u32>(address, 0u), tag);
}

// `SurfacePattern::stencil`: an axis **shifted right by k** reads its values in runs
// 2^k cells long, which is the stretch — bark's grain 4× longer than thick along z,
// brushed grain 4× along x, foliage clumps 2× on every axis. It is a shift and not a
// mask because the rung quantises the address again, and a mask's cleared bits would
// be re-cleared by the rung's own mask (C-164). The ids are the Rust enum's own
// (`SurfacePattern::index`), held together by a test in `voxel-client`.
fn pattern_stencil(pattern: u32, at: vec3<u32>) -> vec3<u32> {
    if (pattern == 1u) {          // Foliage: clumps twice the cell, every axis
        return at >> vec3<u32>(1u);
    } else if (pattern == 2u) {   // Bark: grain four cells along z
        return vec3<u32>(at.x, at.y, at.z >> 2u);
    } else if (pattern == 3u) {   // Brushed: grain four cells along x
        return vec3<u32>(at.x >> 2u, at.y, at.z);
    }
    return at;                    // Speckle, and any id a later row adds
}

// `SurfacePattern::curve`: speckle squares its value (sparse specks, not uniform
// static); the directional shapers keep it, because their character is the
// stretch and not the amplitude.
fn pattern_curve(pattern: u32, s: f32) -> f32 {
    if (pattern == 0u) {
        return s * abs(s);
    }
    return s;
}

// A rate's cell in metres, from the model's own sides: one metre per voxel,
// `SUBVOXEL_SIDE` cells across it, and `DETAIL_SIDE` more across each.
fn cell_m(rates: vec4<u32>, fine: bool) -> f32 {
    let sub_cell = 1.0 / f32(rates.x);
    if fine {
        return sub_cell / f32(rates.y);
    }
    return sub_cell;
}

// The ladder's rungs and their fBm gain (`pattern::OCTAVE_TOP`, `pattern::OCTAVE_FLOOR`,
// `pattern::SUBVOXEL_OCTAVE`, `pattern::OCTAVE_GAIN`). Rung `OCTAVE_TOP` is the
// declared ceiling (32 m — extended from 4 m to stop the moire the vertex detail
// produced when the ladder went silent past 4 m); the floor is `OCTAVE_FLOOR`
// (`pattern::MIN_FEATURE_M`, 31.25 mm — 32 samples per block axis), and the addressed
// base's own 15.6 mm cell is never drawn (C-164).
const OCTAVE_TOP: u32 = 11u;
const OCTAVE_FLOOR: u32 = 1u;
const SUBVOXEL_OCTAVE: u32 = 4u;

/// `pattern::PREFILTER_SPAN`: below this many subvoxels a rung is not averaged.
const PREFILTER_SPAN: i32 = 2;

/// `pattern::OCTAVE_FADE_WIDTH`: the fade's plateau, and the price paid at its edge.
const OCTAVE_FADE_WIDTH: f32 = 1.0;

// `openpbr::PATTERN_TINT`: how much of its own value a fragment's albedo may swing
// with the pattern — a **multiplicative tint**, so the material keeps its hue and its
// identity instead of washing toward white (C-165).
const PATTERN_TINT: f32 = 0.35;
// `voxel_appearance::provenance::COAT_F0`: the coat layer's normal-incidence
// reflectance, `((1.5 - 1)/(1.5 + 1))^2` — the same value the clear coat is
// specified with, so a wet surface's highlight is the layer's own boundary.
const COAT_F0: f32 = 0.04;
const OCTAVE_GAIN: f32 = 0.5;

// One rung's cell in metres (`pattern::octave_cell_m`): the base's own cell times
// `2^octave`.
fn octave_cell_m(rates: vec4<u32>, octave: u32) -> f32 {
    return cell_m(rates, true) * f32(1u << octave);
}

// How much of one rung a pixel can draw: one while its footprint is well below the
// rung's cell, falling to **zero at a quarter** of the cell — a stricter floor
// than Nyquist's half, and a smoothstep rather than a straight line.
fn octave_weight(footprint: f32, cell: f32) -> f32 {
    let t = clamp(1.0 - 4.0 * footprint / cell, 0.0, 1.0);
    // `OCTAVE_FADE_WIDTH`: the ramp pushed toward the Nyquist edge, so a walk-sized
    // step moves a rung's weight far less. The floor is unchanged — still exactly
    // zero at `cell / 4` — and so is the fade's shape. WGSL will not shadow a `let`
    // in the same scope, hence the second name.
    let plateau = 1.0 - pow(1.0 - t, OCTAVE_FADE_WIDTH);
    return plateau * plateau * (3.0 - 2.0 * plateau);
}
// `pattern::octave_weight_staged` (C-171 route a): the smoothstep is replaced by
// its own single drawn point — a rung is **fully on** while the footprint is
// strictly below the rung's Nyquist floor (cell/4, where the plain weight first
// reaches 1) and **fully off** at or above it (the plain weight is already 0 at
// the floor, so step and smoothstep agree at the boundary). A walk-sized step
// moves the weight only across that one threshold, and the footprint is smooth,
// so a rung flips as a coherent region crossing the eye's distance — the C-163
// boil was a per-2×2-quad *derivative* artifact, and there is none of that left
// here. Wider stages were measured to break the ladder: flooring raised weights
// past the ceiling; ceilinging to quarter-cells or to powers of two blanked rungs
// the plain weight still draws.
fn octave_weight_staged(footprint: f32, cell: f32) -> f32 {
    let floor = cell / 4.0;
    return select(0.0, 1.0, footprint < floor);
}

// The address quantised to a multiple of `2^octave` per axis: the rung reads the
// same lattice in cells that many times coarser, and at and above the subvoxel
// rung every address inside a subvoxel quantises to the same one, because those
// features are subvoxel-sized (`pattern::quantise`).
fn quantise(a: vec3<u32>, octave: u32) -> vec3<u32> {
    if (octave >= SUBVOXEL_OCTAVE) {
        return vec3<u32>(0u);
    }
    let mask = ~((1u << octave) - 1u);
    return a & vec3<u32>(mask);
}

// A **world cell** quantised to a multiple of `2^(octave − SUBVOXEL_OCTAVE)` per
// axis — the rungs past the subvoxel, whose features are bigger than one. Two's
// complement masking is floor-quantisation, so negative cells tile without a seam
// at the origin (`pattern::quantise_cell`).
fn quantise_cell(cell: vec3<i32>, octave: u32) -> vec3<i32> {
    if (octave <= SUBVOXEL_OCTAVE) {
        return cell;
    }
    let mask = ~((1 << (octave - SUBVOXEL_OCTAVE)) - 1);
    return cell & vec3<i32>(mask);
}

// `pattern::FACE_COS_FLOOR`: a face seen nearly edge-on has a cosine near zero, and
// an unbounded footprint would divide by it. The floor bounds the stretch at 20×;
// the ladder's ceiling then silences the pattern honestly rather than letting a
// grazing surface alias.
const FACE_COS_FLOOR: f32 = 0.05;

// One pixel's footprint on the surface, in metres (`pattern::eye_footprint_m`): the
// pixel's own angular size (`pattern::pixel_angle_rad`, carried in
// `camera.screen.x`) out to this fragment's distance from the eye, stretched by the
// obliquity of the face. Every input is the fragment's own state — a distance and a
// face angle, not a screen-space derivative — so it varies smoothly across a face
// and the rung weights fade instead of boiling (C-163).
fn eye_footprint(p_rel: vec3<f32>, n: vec3<f32>, v: vec3<f32>) -> f32 {
    let distance = length(p_rel);
    let cos_face = max(abs(dot(n, v)), FACE_COS_FLOOR);
    return distance * camera.screen.x / cos_face;
}

// The deviation this fragment shows, with its material's gains applied and its
// shaper shaping the noise — at the footprint the caller derived from the eye's own
// geometry (`eye_footprint`): the whole of what C-161 draws, as C-162 corrected it
// and C-163 smoothed.
// Returns `(roughness, reflectance, coverage, 0)`. **`coverage` is the third
// component on purpose**: the caller has to know whether any rung could be drawn
// at all, because the answer \"no rung\" is not \"no detail\" — the per-vertex sample
// answers there instead — and a caller that read only the first two components
// could not tell the two apart. It is a `vec4` rather than a struct because WGSL
// returns it by value either way and the padding is explicit.
fn pattern_deviation(material: u32, m: Material, p_rel: vec3<f32>, footprint: f32) -> vec4<f32> {
    let roughness_gain = m.pattern.x;
    let reflectance_gain = m.pattern.y;
    if (roughness_gain <= 0.0 && reflectance_gain <= 0.0) {
        // Not varied at this level at all (`ShadingField::gain`), which is a
        // different claim from \"varied by zero\": skip the hash entirely. Coverage
        // 0 as well, so the vertex sample does not smuggle in a variation the
        // world declined to declare for this material.
        return vec4<f32>(0.0);
    }
    let pattern = u32(m.pattern.z);
    let q = p_rel + camera.eye_phase.xyz;
    // The fragment's **world subvoxel cell**: `floor(q * rates.x)` is the cell
    // relative to the eye's own voxel, and the voxel's own place is added back, so
    // two blocks never share a pattern (C-162).
    let cell = vec3<i32>(floor(q * f32(camera.rates.x)))
        + camera.eye_voxel.xyz * i32(camera.rates.x);
    let f_sub = fract(q * f32(camera.rates.x));
    // The last sample of each rate, not one past it: the lattice is a grid of
    // samples (`DetailIndex::from_unit`), and in `f32` a fraction one rounding step
    // below 1.0 can round onto the top edge.
    let fine_last = f32(camera.rates.y - 1u);
    let a = vec3<u32>(min(floor(f_sub * f32(camera.rates.y)), vec3<f32>(fine_last)));
    // The address's packing width is the side's own log2 (`DetailLattice::flat`),
    // derived from the uniform rather than stated: four bits an axis at the base's
    // 16 samples, eight if a side ever grows.
    let bits = firstLeadingBit(camera.rates.y);

    let seed = vec2<u32>(camera.lattice.x, camera.lattice.y);
    let tag = vec2<u32>(camera.lattice.z, camera.lattice.w);

    // The ladder: the same field read in cells of `2^octave`, weighted by what the
    // pixel can draw and by the fBm gain, then **normalised** — a weighted mean of
    // values in `[-1, 1]`, so the deviation stays inside the range `with_detail`
    // promises at every footprint (C-163, `pattern.rs`).
    var weight_sum = 0.0;
    var roughness = 0.0;
    var reflectance = 0.0;
    var gain = 1.0;
    for (var octave = OCTAVE_FLOOR; octave <= OCTAVE_TOP; octave = octave + 1u) {
        let weight = octave_weight(footprint, octave_cell_m(camera.rates, octave));
        if (weight > 0.0) {
            // **Stencil first, rung second** (C-164): the shift moves the stretched
            // axis into a coarser address space, then the rung coarsens it further,
            // so the shaper's character holds at every rung.
            let at_rung = quantise(pattern_stencil(pattern, a), octave);
            let rung_flat = at_rung.x | (at_rung.y << bits) | (at_rung.z << (2u * bits));
            // **A coarse rung is the mean of the grains it stands for** — the same
            // eight sub-cell centres `pattern::prefilter_offsets` names, read in
            // the same order, or the CPU and GPU disagree.
            // `OCTAVE_FLOOR` is below `SUBVOXEL_OCTAVE`, so the shift is guarded rather
            // than subtracted: an unsigned subtraction there wraps to a huge shift.
            let shift = select(0u, octave - SUBVOXEL_OCTAVE, octave >= SUBVOXEL_OCTAVE);
            let span = 1i << shift;
            // The centres of the eight equal sub-cells: `span/4` and `3·span/4`, which
            // is what `pattern::prefilter_offsets` returns. **Not `lo * 3`**: that is
            // `3·span/4` only when `span` divides by four, and at `span = 2` it
            // collapses both taps onto the same cell — so the two sides average
            // different sets and the parity test fails. `3·span/4` is written out
            // rather than factored for exactly that reason.
            let lo = span / 4;
            let hi = (3 * span) / 4;
            var taps_r = 0.0;
            var taps_f = 0.0;
            var taps = 0.0;
            for (var i = 0u; i < 2u; i = i + 1u) {
                let du = select(0, select(lo, hi, i == 1u), span >= PREFILTER_SPAN);
                for (var j = 0u; j < 2u; j = j + 1u) {
                    let dv = select(0, select(lo, hi, j == 1u), span >= PREFILTER_SPAN);
                    for (var k = 0u; k < 2u; k = k + 1u) {
                        let dw = select(0, select(lo, hi, k == 1u), span >= PREFILTER_SPAN);
                        // Offsets run from the rung's own coarse cell, not from `cell`: a fragment
                        // sits wherever it sits inside that cell, so offsetting from it
                        // would walk into its neighbours and slide the average's centre
                        // as the fragment moves — the re-rolling this pass removes.
                        let cell_rung = quantise_cell(cell, octave);
                        let at = cell_rung + vec3<i32>(du, dv, dw);
                        taps_r = taps_r + pattern_curve(
                            pattern, lattice_sample(seed, tag, material, at, rung_flat, 1u));
                        taps_f = taps_f + pattern_curve(
                            pattern, lattice_sample(seed, tag, material, at, rung_flat, 2u));
                        taps = taps + 1.0;
                    }
                }
            }
            roughness = roughness + (taps_r / taps) * weight * gain;
            reflectance = reflectance + (taps_f / taps) * weight * gain;
            weight_sum = weight_sum + weight * gain;
        }
        gain = gain * OCTAVE_GAIN;
    }
    if (weight_sum <= 0.0) {
        // No rung is drawable at this footprint: the field has nothing to say here,
        // which is not the same as saying zero. Coverage 0 is how that reaches the
        // caller, which fills the silence with the corner's own sample.
        return vec4<f32>(0.0);
    }
    return vec4<f32>(
        roughness / weight_sum * roughness_gain,
        reflectance / weight_sum * reflectance_gain,
        1.0,
        0.0,
    );
}

// `openpbr::with_detail`'s one-sided move for the scalar roughness: toward the edge
// the value has room for, never past it. The albedo is **tinted** instead
// (`PATTERN_TINT`, C-165).
fn vary(value: f32, delta: f32) -> f32 {
    if (delta >= 0.0) {
        return value + delta * (1.0 - value);
    }
    return value + delta * value;
}


const DEGREES_PER_RADIAN: f32 = 57.29577951308232;

/// **Schlick's approximation at one angle**: `F0 + (1 − F0)·(1 − cos θ)⁵` — the reflectance
/// of a boundary, which is also the weight the fluid reflection mixes with.
///
/// The same expression and the same exponent as `voxel_appearance::fluid::schlick` (shared
/// with the rim term through `provenance::RIM_SCHLICK_EXPONENT`), written here because a
/// shader cannot call Rust. A "nicer" formulation would be a second model.
fn schlick(cos_theta: f32, f0: f32) -> f32 {
    let c = clamp(cos_theta, 0.0, 1.0);
    return f0 + (1.0 - f0) * pow(1.0 - c, 5.0);
}

/// **The dome table read at a direction**: `(elevation, azimuth)` in degrees, bilinear with
/// the azimuth axis wrapped — the same interpolation `skylight::sky_dome_lookup` performs on
/// the CPU and `sky.wgsl` draws with in the sky pass.
///
/// `grid` is `(azimuths, elevations)`, which is the table's own shape rather than either
/// reader's idea of it: a table that does not cover the grid it was handed is refused here
/// (no reflection) rather than read past its end, the same "refuse rather than invent" rule
/// the sky pass applies when it has been given no table at all.
fn dome_lookup(elevation: f32, azimuth: f32, grid: vec2<f32>) -> vec3<f32> {
    let azimuths = u32(grid.x);
    let elevations = u32(grid.y);
    if (azimuths < 2u || elevations < 2u || arrayLength(&sky_table) < azimuths * elevations) {
        return vec3<f32>(0.0);
    }
    // Elevation is measured from the horizon up over 90°, and the table's rows run the same
    // way; the azimuth axis is periodic over 360°, which is why it wraps and the elevation
    // axis does not.
    let ef = clamp(elevation, 0.0, 90.0) / 90.0 * (grid.y - 1.0);
    let e0 = min(u32(floor(ef)), elevations - 2u);
    let efrac = ef - floor(ef);
    var af = azimuth - 360.0 * floor(azimuth / 360.0);
    af = af / 360.0 * grid.x;
    let a0 = u32(floor(af)) % azimuths;
    let a1 = (a0 + 1u) % azimuths;
    let afrac = af - floor(af);
    let row0 = e0 * azimuths;
    let row1 = row0 + azimuths;
    let low = mix(sky_table[row0 + a0].rgb, sky_table[row0 + a1].rgb, afrac);
    let high = mix(sky_table[row1 + a0].rgb, sky_table[row1 + a1].rgb, afrac);
    return mix(low, high, efrac);
}

/// The cloud's own row, from the two numbers a **medium** has and a surface does not.
///
/// `albedo` is `CLOUD_ALBEDO` — a real number for water droplets, and the reason a cloud is
/// white rather than grey; it is **sent** in `camera.sky_grid.w` rather than typed here, the
/// same rule `sky_grid.z` follows for the water's boundary. Every parameter a *surface* would
/// carry is absent, which is the medium's own statement: a whole-cube medium has no roughness
/// to reflect with and no specular boundary to reflect off, so `npr::shade` draws neither — and
/// the cloud is then lit by exactly the terms everything else is lit by.
fn cloud_row(albedo: f32) -> Material {
    var m: Material;
    m.base_color_and_present = vec4<f32>(albedo, albedo, albedo, 1.0);
    m.rough_metal = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    m.spec_emission = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    m.emission_rgb_open = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    m.pattern = vec4<f32>(0.0);
    m.specular_boundary = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    m.participating_medium = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    m.medium_flags = vec4<f32>(0.0);
    return m;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    if (in.material >= arrayLength(&materials)) {
        // A quad naming a material outside the table is not shaded at all. It
        // is not shaded grey: no admitted input means no picture.
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    // **A cloud has no row because it has no surface** (C-178). Its record carries the empty
    // material id, so the row is built from the medium's own two numbers — its albedo and its
    // transmittance, both sent — and the *same* terms below shade it. Nothing about the look is
    // written twice; only where the row came from differs.
    var m = materials[in.material];
    if (in.veil > 0.0) {
        m = cloud_row(camera.sky_grid.w);
    }
    let n = normalize(in.normal);
    let v = normalize(in.to_eye);
    let ndotv = clamp(dot(n, v), 0.0, 1.0);

    // The material's own pattern, read at this fragment's address: the block grid
    // and subvoxel grid a greedy-merged quad cannot carry as geometry (C-161).
    // **The footprint is the eye's own geometry, not a derivative.** `fwidth` is
    // estimated per 2×2 quad, so on a face the camera sees at an angle its estimate steps from
    // quad to quad; the ladder's weights stepped with it and the face boiled as the
    // camera moved (C-163). This is a function of the fragment alone.
    //
    // The per-vertex detail is the lattice sampled once at the corner's own world
    // address (C-161), and the ladder below reads rungs from the fragment's own
    // footprint — so a merged quad shows sub-pixel detail while the vertex cost
    // stays constant. Which of the two the fragment shows is decided a few lines
    // down, by what the pixel can actually draw.
    let footprint = eye_footprint(in.eye_rel, n, v);
    // **The ladder speaks where it can, and the vertex speaks where it cannot.**
    // The ladder is a weighted mean over rungs a pixel of this footprint can draw,
    // and it is deliberately *silent* past its ceiling rather than aliasing
    // (C-163): at that distance four shared corners are all the detail there is.
    // The per-vertex sample is that detail — one lattice value per corner, at the
    // corner's own world address, interpolated across the quad — which is the new
    // architecture's contribution to a surface the old one left bare. Adding the
    // two would count one address twice; `mix` at the ladder's own coverage keeps
    // the rung blend exactly as it was wherever the ladder draws, so a material
    // that shows its pattern today shows the same one.
    //
    // Both sides carry the material's own gains (`ShadingField::gain`): the ladder
    // applies them to its normalised mean, and here to the vertex sample, so a
    // material the world does not vary reads as unvaried either way.
    let vertex_deviation = vec2<f32>(in.detail_roughness, in.detail_reflectance)
        * vec2<f32>(m.pattern.x, m.pattern.y);
    let ladder = pattern_deviation(in.material, m, in.eye_rel, footprint);
    let deviation = mix(vertex_deviation, ladder.xy, ladder.z);

    let base_unvaried = m.base_color_and_present.xyz;
    let base_present = m.base_color_and_present.w;
    // **A tint, not a wash** (`openpbr::with_detail`, C-165): the pattern's
    // reflectance channel multiplies the material's own colour — hue preserved, the
    // material stays itself — rather than moving each channel toward white, which
    // painted the same pale speckle over sand, wood, foliage and rock alike.
    let tint = clamp(1.0 + PATTERN_TINT * deviation.y, 0.0, 1.0 + PATTERN_TINT);
    let base = select(
        base_unvaried,
        clamp(base_unvaried * tint, vec3<f32>(0.0), vec3<f32>(1.0)),
        base_present > 0.5,
    );
    let roughness_unvaried = m.rough_metal.x;
    let roughness = select(
        roughness_unvaried,
        vary(roughness_unvaried, deviation.x),
        m.rough_metal.y > 0.5,
    );
    let rough_present = m.rough_metal.y;
    let metalness = m.rough_metal.z;
    let transmission = m.rough_metal.w;
    let weight = m.spec_emission.x;
    let weight_present = m.spec_emission.y;
    let emission_luminance = m.spec_emission.z;
    let emission_present = m.spec_emission.w;
    let emission_rgb = m.emission_rgb_open.xyz;
    let open = m.emission_rgb_open.w;
    // ---- the parameters the CPU's model reads and the GPU had nowhere to put -----
    //
    // `openpbr::derive` computes every one of these and `npr::shade` reads every one,
    // and until this group existed they reached the row and stopped: the shader drew
    // the boundary with no reflectance of its own, no coat, and no medium. The
    // `_admitted` flags are the same "absent is not zero" rule every other slot
    // follows, and each value's *use* is transcribed from `npr::shade` below —
    // including `specular_f0`'s `unwrap_or(1.0)`, which is the model's statement and
    // not this file's.
    let f0_value = m.specular_boundary.x;
    let f0_admitted = m.specular_boundary.y > 0.5;
    let coat_weight = m.specular_boundary.z;
    let coat_roughness = m.specular_boundary.w;
    let coat_admitted = m.participating_medium.x > 0.5;
    let depth_value = m.participating_medium.y;
    let depth_admitted = m.participating_medium.z > 0.5;
    let subsurface_value = m.participating_medium.w;
    let subsurface_admitted = m.medium_flags.x > 0.5;
    // `specular_ior.map(specular_f0).unwrap_or(1.0)`: an absent index delivers the
    // whole weight, which is where this model has always put it.
    let f0 = select(1.0, f0_value, f0_admitted);
    // `transmission_depth.unwrap_or(0.0)` and `subsurface_weight.unwrap_or(0.0)`:
    // an absent depth is zero thickness and an absent scattering is no scattering.
    let depth = select(0.0, depth_value, depth_admitted);
    let subsurface = select(0.0, subsurface_value, subsurface_admitted);

    var colour = vec3<f32>(0.0);

    // 4. Ambient, first, because it is the term that survives when little else
    //    does. A transmissive surface receives light from every direction, and
    //    a face with holes receives less of everything.
    //
    // The ambient always comes from the rig uniform: it is the sky's own term, a
    // property of the atmosphere rather than of one cell, and no field has yet
    // measured an ambient at an address. The directional and diffuse light is
    // either the field's or the rig's — see the branch below.
    if (base_present > 0.5) {
        let transmitted = transmission * (1.0 - depth);
        let gain = (1.0 + transmitted) * (1.0 - open);
        colour += base * (rig.ambient.rgb * gain);
    }

    // ---- which light: the world's field, or the rig --------------------------
    //
    // A corner says whether the shaper layer had a measurement for its cell
    // (`light_present`). Where it did, the surface is lit by **the light that
    // reached it** — the field's own beam and sky, per band — which is what makes
    // a covered cell dark and an open one bright without the shader inventing a
    // shadow. Where it did not, the rig's lights stand in, which is the Source 2
    // model this file transcribes and how every frame was lit before the field
    // existed. The two are one `if`: a frame with no field is bit-for-bit the
    // frame it was yesterday.
    if (in.light_present == 1u) {
        // **A band triple is not yet a colour.** The bands arrive in band order
        // (B3, B4, B5) = blue, green, red, and the albedo is (r, g, b), so the
        // reversal is load-bearing: without it the blue light would multiply the
        // red albedo. `planet-render`'s `to_channels` is this same line.
        let beams = vec3<f32>(in.light_direct.z, in.light_direct.y, in.light_direct.x);
        let sky = vec3<f32>(in.light_diffuse.z, in.light_diffuse.y, in.light_diffuse.x);
        let opaque = (1.0 - metalness) * (1.0 - transmission);

        // 1. Half-Lambert diffuse wrap on the beam, and the sky whole: the sky
        //    arrives from every direction, so there is no angle for it to wrap at.
        //
        //    **The boundary's share comes out of the beam, at the beam's own angle**
        //    (`OpenPBR_Energy_Conservation` §§5–6): `F(θ_L)` is the share of this beam the
        //    interface reflects instead of transmitting, so it is what the substrate is drawn
        //    from and what the rim below spends — together they can never exceed the light
        //    that arrived. Reading that budget at the **view** angle instead would make a face
        //    seen at a grazing angle lose its diffuse *whatever the Sun was doing*, which is
        //    the "blocks are just lit by themselves" defect. The coat is a *layer* (§9), so
        //    its own reflectance comes out of the base too. The sky is left whole on purpose:
        //    it is a hemisphere, not one angle, so a single-angle Fresnel cannot state its
        //    share — the spec's own `E[f_specular]` is an integrated albedo, and says so.
        //
        //    **The field carries radiance and no direction**, so the beam's direction comes
        //    from the rig — the same source the warp below takes its direction from, and the
        //    same world's Sun the shaper layer measured this radiance against. With no rig
        //    light there is no angle to measure, and the view stands in.
        var beam_dir = vec3<f32>(0.0, 1.0, 0.0);
        var beam_known = false;
        if (rig.count.x > 0u) {
            beam_dir = normalize(rig.lights[0].direction.xyz);
            beam_known = true;
        }
        let ndotl = select(ndotv, clamp(dot(n, beam_dir), -1.0, 1.0), beam_known);
        let beam_share = pow(1.0 - clamp(ndotl, 0.0, 1.0), 5.0);
        // **The silhouette shape**, which is what makes the rim a rim — the same power, so
        // the two terms are one law read at two angles.
        let silhouette = pow(1.0 - ndotv, 5.0);
        var coat = 0.0;
        if (coat_weight > 0.0 && coat_admitted) {
            coat = COAT_F0 * coat_weight * (1.0 - coat_roughness) * beam_share;
        }
        // **One stack, one bookkeeping convention** (§9): the substrate keeps what the coat
        // layer above it passed. The base interface's own `F(θ_L)` is *not* subtracted here
        // because `wrap` is a declared stylisation whose floor is exactly the light the
        // interface never passed (`npr::shade` states the departure and the test that pins
        // it); what is not stylised is the rim, whose energy **is** `F(θ_L)` — so a boundary
        // light can never conjure light the beam did not supply.
        let substrate = 1.0 - coat;
        if (base_present > 0.5) {
            let half_lambert = ndotl * 0.5 + 0.5;
            let lambert_term = clamp(ndotl, 0.0, 1.0);
            let wrap = half_lambert + subsurface * (lambert_term - half_lambert);
            colour += base * (beams * (wrap * opaque * substrate) + sky * opaque);
        }

        // 2. Rim: Schlick's exponent, view-facing, scaled by the beam's own
        //    radiance at this cell **and by the block's own reflectance** — what reaches
        //    the eye at a boundary is that boundary's reflectance, so the rim carries the
        //    block's colour instead of overriding it. The `select` is `npr::shade`'s own
        //    rule for a block nobody has a spectrum for: it reflects the light, not an
        //    invented grey. **It spends this beam's budget rather than adding to it**: it is
        //    the reflected share (`beam_share`), shaped by the silhouette — so a backlit face
        //    is all rim and no diffuse, and substrate + rim can never exceed the light.
        let rim_reflectance = select(vec3<f32>(1.0), base, base_present > 0.5);
        colour += rim_reflectance * (beams * (beam_share * silhouette));

        // 2b. The coat, over the rim: a smooth dielectric layer at `COAT_F0`, gated on
        //     both its weight and its admitted roughness (a highlight with no spread
        //     is not a coat). `provenance::COAT_F0` — typed here for the same reason
        //     `PATTERN_TINT` is: a shader cannot call Rust, and the number is named so
        //     the two can be held against each other. Its subtraction from the base is the
        //     layer's pass-through above.
        if (coat > 0.0) {
            colour += beams * (coat * silhouette);
        }

        // 3. The specular warp: a step, and the threshold is the material's own
        //    roughness rather than a number this file picked.
        //
        //    **The direction comes from the rig, the magnitude from the field.**
        //    The rig is this world's skylight — the same sun the shaper layer
        //    measured the beam from — so its direction is the honest one, and the
        //    field supplies how much of it arrived here. A radiance triple is not
        //    a direction: `normalize(direct)` would point at whichever band is
        //    brightest, which is a colour, not a place.
        if (rough_present > 0.5 && weight_present > 0.5 && rig.count.x > 0u) {
            let toward_light = normalize(rig.lights[0].direction.xyz);
            let half_v = normalize(toward_light + v);
            let facing = clamp(dot(n, half_v), 0.0, 1.0);
            if ((1.0 - facing) <= roughness) {
                colour += beams * (weight * f0);
            }
        }
    } else {
        let count = rig.count.x;
        for (var i = 0u; i < count; i += 1u) {
            let light = rig.lights[i];
            let toward_light = normalize(light.direction.xyz);
            let intensity = light.direction.w;
            let ndotl = dot(n, toward_light);

            // 1. Half-Lambert diffuse wrap, widened toward Lambert by the medium's
            //    own scattering — **and the boundary's share comes out of it, at the beam's
            //    own angle**, so the rim and the coat below spend one budget with the
            //    diffuse rather than the same light twice (the spec's §§5–6; see the field
            //    branch above for the full statement, including why the sky is not divided
            //    this way).
            let beam_share = pow(1.0 - clamp(ndotl, 0.0, 1.0), 5.0);
            let silhouette = pow(1.0 - ndotv, 5.0);
            var coat = 0.0;
            if (coat_weight > 0.0 && coat_admitted) {
                coat = COAT_F0 * coat_weight * (1.0 - coat_roughness) * beam_share;
            }
            let substrate = 1.0 - coat;
            if (base_present > 0.5) {
                let half_lambert = ndotl * 0.5 + 0.5;
                let lambert_term = clamp(ndotl, 0.0, 1.0);
                let wrap = half_lambert + subsurface * (lambert_term - half_lambert);
                let lambert = wrap * intensity * (1.0 - metalness) * (1.0 - transmission)
                    * substrate;
                colour += base * (light.colour.rgb * lambert);
            }

            // 2. Rim: Schlick's exponent, view-facing, and scaled by **the block's own
            //    reflectance** — the same block `npr::shade` states, because a rim light
            //    is not a second energy source in the light's own colour. Its energy is the
            //    share of *this* beam the interface reflected, seen at the silhouette.
            let rim_reflectance = select(vec3<f32>(1.0), base, base_present > 0.5);
            colour += rim_reflectance * (light.colour.rgb * (beam_share * silhouette * intensity));

            // 2b. The coat, over the rim: the clear dielectric layer, read together
            //     with its own roughness because an absent one means nobody has
            //     characterised the layer.
            if (coat > 0.0) {
                colour += vec3<f32>(1.0) * (coat * silhouette * intensity);
            }

            // 3. The specular warp: a step, and the threshold is the material's own
            //    roughness rather than a number this file picked. What the step
            //    delivers is scaled by the boundary's own reflectance.
            if (rough_present > 0.5 && weight_present > 0.5) {
                let half_v = normalize(toward_light + v);
                let facing = clamp(dot(n, half_v), 0.0, 1.0);
                if ((1.0 - facing) <= roughness) {
                    colour += light.colour.rgb * (weight * f0 * intensity);
                }
            }
        }
    }

    // Emission is not a Source 2 term. It is the OpenPBR input a dynamic block
    // emitter carries, and it is added last because it depends on no light.
    if (emission_present > 0.5) {
        colour += emission_rgb * emission_luminance;
    }

    // ---- the fluid reflection: the sky, seen off the boundary ------------------
    //
    // **A fluid is not a matte sheet with a sheen on it.** Looking along water, most of
    // what reaches the eye *is* the sky reflected, which is why `voxel_appearance::fluid`
    // exists and why the sea read as a hole without it. `shading::fluid_surface_cpu` is the
    // Rust side of exactly this composition and `fluid::surface_radiance` states the law:
    // the body through the transmitted share, plus the sky's radiance from the mirror
    // direction through the reflected share. This is that expression, transcribed.
    //
    // **What makes a material a fluid here is `transmission`**, its own OpenPBR parameter
    // rather than a list of material ids in a shader: a boundary that transmits is one whose
    // mirror image the eye sees, and a boundary that does not is not. The gate also keeps
    // every stated-material comparison intact, since those carry no transmission.
    //
    // The mirror ray can point below the horizon — a steep face reflects the world, not the
    // sky — and this model has no world in a reflection yet, so that case adds nothing: the
    // reflected radiance is black and only the `(1 − F)` transmission of the body remains. The
    // same guard, in the same words, as `fluid_surface_cpu`'s.
    if (transmission > 0.0 && camera.sky_grid.x > 1.0 && camera.sky_grid.y > 1.0) {
        // `reflect(−v, n)` with WGSL's own arithmetic is `−v + 2·(v·n)·n`, which is
        // `fluid::reflected_ray`'s expression exactly — the same convention (`v` points from
        // the surface toward the eye) and the same result, so there is no sign for the two to
        // disagree about.
        let ray = reflect(-v, n);
        var reflected = vec3<f32>(0.0);
        if (ray.y > 0.0) {
            // The world frame is east-north-up with `x` east and `z` south, so the azimuth is
            // `atan2(x, −z)` — the same mapping `sky.wgsl` turns a pixel into, and the same one
            // `skylight::sky_direction_of` performs for the CPU side.
            let elevation = clamp(asin(clamp(ray.y, -1.0, 1.0)) * DEGREES_PER_RADIAN, 0.0, 90.0);
            var azimuth = atan2(ray.x, -ray.z) * DEGREES_PER_RADIAN;
            if (azimuth < 0.0) {
                azimuth = azimuth + 360.0;
            }
            reflected = dome_lookup(elevation, azimuth, camera.sky_grid.xy);
        }
        let fresnel = schlick(clamp(dot(n, v), 0.0, 1.0), camera.sky_grid.z);
        colour = colour * (1.0 - fresnel) + reflected * fresnel;
    }

    // **The two display concerns, in the only order that works.** The transform
    // first, because it is what makes the value presentable, then the target's own
    // rounding, because a dither is a statement about the steps that remain *after*
    // the range is settled. Reversing them would dither a value that is then
    // rescaled, which is a dither of something that is not on screen.
    //
    // Both are gated on the target's own properties, so a measurement frame is the
    // number the model computed and nothing else — see `View::screen`.
    var presentable = colour;
    if (camera.screen.z > 0.0) {
        presentable = tone_map(colour);
    }
    // **A cloud is a veil and not a surface** (C-179). The alpha is the share of this pixel
    // that is *not* the sky standing behind it: `veil` is the cell's column transmittance, so
    // `1 − veil` is exactly the share the cloud contributes — the same fraction the light
    // sampler walks in `voxel-stream::light`, so the picture and the shadow cannot disagree.
    // Terrain keeps `1.0`, which is alpha's existing meaning here: geometry is on this pixel.
    let alpha = select(1.0, 1.0 - in.veil, in.veil > 0.0);
    return vec4<f32>(dithered(presentable, in.clip.xy, camera.screen.y), alpha);
}
