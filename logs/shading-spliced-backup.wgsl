// the Valve Source 2 stylisation, transcribed into WGSL — docs/lighting-sky-and-moon.md

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
    // one it can present (C-167), `0` for the offscreen measurement target.
    //
    // `w` is **the cloud's cross-fade weight** (C-183), the newer tree's share of the sky, read
    // off the world's ticks by `cloud_cubes::fade_weight`. The interpolation is here rather than
    // baked into the veils because it is one number a frame (`docs/LEDGER.md` §183), and it is a
    // viewport number rather than a world one, so it travels with the camera.
    //
    // **The two are separate flags on purpose, though the same target sets both
    // today.** Dithering is a statement about how many steps a target has; the
    // display transform is a statement about how much range it has. An HDR display
    // is a display that quantises nothing, and a future frame that renders into a
    // float intermediate before presenting would set this one on the *present* pass
    // and neither on the world pass. One flag would have to be split then, and
    // splitting it later is how two meanings come to share one bit.
    screen: vec4<f32>,
    // **The water boundary's reflectance, and the cloud's albedo** — `z` is
    // `voxel_appearance::fluid::WATER_F0` and `w` `cloud_field::CLOUD_ALBEDO` (C-177, C-179),
    // both sent rather than typed here for the reason `lattice` and `rates` are:
    // `fluid::schlick` is called with `WATER_F0` on the CPU and `cloud_row` with `CLOUD_ALBEDO`,
    // and a shader holding its own `0.0204` or `0.99` would be a second statement of each.
    //
    // **`x` and `y` used to be the dome's grid** — its azimuth and elevation counts, which this
    // pass read to index the sky for a fluid's reflection. The dome is gone: the reflection is
    // `atmosphere.wgsl`'s transport now, evaluated at the mirror ray's own direction, and the
    // transport carries its own band count. The two slots are reserves, and whether a sky is
    // present at all is the model's own row count rather than a positive grid side.
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
    // **A body's light direction, `y` and `z`** (C-184), and a rectangle's two unused slots: for a
    // body the six slots after `face` and `chunk` are its radiance's three channels, then `x`, `y`, `z`
    // of the direction that lights it, each an `f32`'s bits.
    light_y: u32,
    light_z: u32,
};

// The record's own discriminant for a body (C-184): `voxel_client::quad::BODY_KIND`.
const BODY_KIND: u32 = 6u;

@group(0) @binding(3) var<storage, read> quads: array<QuadRecord>;
// One chunk's origin **relative to the eye**, in metres, computed on the CPU in
// `f64` and narrowed only afterwards — the same decision `scene.rs` makes for a
// vertex, and the reason a rectangle can be reused across frames while a vertex
// cannot. A `vec4` because a storage array's stride is 16 bytes.
@group(0) @binding(4) var<storage, read> origins: array<vec4<f32>>;

// **The beam's own shadow** — the Sun's or the Moon's, whichever light the rig names.
// The two maps are the world's own silhouette seen from the light, drawn every frame from
// the same corners this pass draws; `split` is `(near range, far range, bias in metres,
// and the far-end fade's width in metres)` and `cascade` is `(near half, far half, near resolution, far resolution).
// `direction.w` is whether this frame has a beam to cast at all.
@group(0) @binding(6) var near_shadow: texture_depth_2d;
@group(0) @binding(7) var far_shadow: texture_depth_2d;
@group(0) @binding(8) var shadow_sampler: sampler_comparison;

struct SunShadow {
    near_vp: mat4x4<f32>,
    far_vp: mat4x4<f32>,
    split: vec4<f32>,
    cascade: vec4<f32>,
    direction: vec4<f32>,
};
@group(0) @binding(9) var<uniform> beam_shadow: SunShadow;

/// **One cascade's answer, and whether it has one**: `x` is the 3 × 3 comparison's mean over the
/// cascade's own texel, `y` the weight the map can claim there — `0` outside the cascade's box,
/// ramped to `0` across the box's own border, `1` where the map has full say.
///
/// This is the arithmetic `sun_visibility` used to do for one cascade, moved here so the two
/// cascades can be **weighted** instead of chosen: the same normal offset, the same bias, the
/// same nine taps.
fn cascade_lit(rel: vec3<f32>, n: vec3<f32>, far: bool) -> vec2<f32> {
    let res = select(beam_shadow.cascade.z, beam_shadow.cascade.w, far);
    // **The bias is a distance along the normal, not a texel of ground.** It was
    // `1.5 · texel · (1 + 4·slope)` along the normal: 0.56 m at the shipped 192 m far cascade and
    // 2.4 m of ground under the 10.6° sun, where that cascade's texel is 0.375 m. That is not a
    // bias, it is a displaced receiver — the wall's shadow left the frame. Two centimetres, the
    // same at both cascades, moves the edge by `0.02 / tan(10.6°)` = 0.11 m: the same order as
    // the CPU field's own 0.103 m shoulder, and enough to keep a surface out of its own map.
    let bias_m = beam_shadow.split.z;
    let lift = n * bias_m;
    // `select` takes scalars and vectors, not a matrix: one branch rather than a table.
    var vp = beam_shadow.near_vp;
    if (far) {
        vp = beam_shadow.far_vp;
    }
    let clip = vp * vec4<f32>(rel + lift, 1.0);
    let ndc = clip.xyz / clip.w;
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0 || ndc.z < 0.0 || ndc.z > 1.0) {
        return vec2<f32>(0.0, 0.0);
    }
    let edge = min(min(uv.x, 1.0 - uv.x), min(uv.y, 1.0 - uv.y));
    let weight = smoothstep(0.0, SHADOW_EDGE_TEXELS / res, edge);
    let texel = vec2<f32>(1.0 / res);
    // **One application, not two.** The lift above is the whole bias: the receiver is 2 cm off
    // its surface when it is compared. Subtracting the same 2 cm from `ndc.z` as well would be a
    // second offset along the light — a different quantity, whose ground effect is
    // `0.02 / cos(elevation)` = 0.02 m rather than the lift's `0.02 / tan` = 0.11 m — and the
    // frame's edges do not move when it is removed (the witness measured both variants).
    let depth = ndc.z;
    var lit = 0.0;
    for (var y = -1; y <= 1; y = y + 1) {
        for (var x = -1; x <= 1; x = x + 1) {
            let at = uv + vec2<f32>(f32(x), f32(y)) * texel;
            if (far) {
                lit = lit + textureSampleCompare(far_shadow, shadow_sampler, at, depth);
            } else {
                lit = lit + textureSampleCompare(near_shadow, shadow_sampler, at, depth);
            }
        }
    }
    return vec2<f32>(lit / 9.0, weight);
}

/// **Is this point lit by the beam, this frame?** Two cascades are two levels of one shadow, and
/// the level is chosen by **weight, not by branch**: the near map's share falls from 1 to 0
/// across a band either side of the split, the far map's rises with it, and the answer is their
/// normalised mean. A surface crossing the seam therefore ramps between two texel sizes instead
/// of stepping between them, and the `1` of "this map has nothing to say there" is a *weight* of
/// zero rather than a lit answer — the same bookkeeping the materials' pattern ladder keeps for
/// its rungs (`pattern_deviation`'s `weight_sum`).
///
/// Cost: nine comparison taps where one cascade answers, **eighteen inside the band**, where the
/// two do. The two branches are what keep the rest of the frame at nine.
fn sun_visibility(rel: vec3<f32>, n: vec3<f32>) -> f32 {
    if (beam_shadow.direction.w < 0.5) {
        return 1.0;
    }
    // **A face turned away from the beam receives none of it**, so it has nothing to occlude and
    // its own back face is not an occluder. The map draws both sides of every quad (culling is
    // off), and without this a lone probe's back face would shadow its front pixel.
    if (dot(n, beam_shadow.direction.xyz) <= 0.0) {
        return 1.0;
    }
    let dist = length(rel);
    if (dist > beam_shadow.split.y) {
        return 1.0;
    }
    // **The handover's width is the coarse map's own texel size**, so a host that raises the
    // shadow range — and with it the coarse cascade — takes its band with it.
    let far_texel_m = 2.0 * beam_shadow.cascade.y / beam_shadow.cascade.w;
    let band = SHADOW_BLEND_TEXELS * far_texel_m;
    let far_share = smoothstep(beam_shadow.split.x - band, beam_shadow.split.x + band, dist);
    var sum = 0.0;
    var weight = 0.0;
    if (far_share < 1.0) {
        let near = cascade_lit(rel, n, false);
        let share = (1.0 - far_share) * near.y;
        sum = sum + near.x * share;
        weight = weight + share;
    }
    if (far_share > 0.0) {
        let far = cascade_lit(rel, n, true);
        let share = far_share * far.y;
        sum = sum + far.x * share;
        weight = weight + share;
    }
    // Both maps silent: a fragment outside both boxes, which only a host that shrinks the
    // cascades below the eye's own field of view can produce.
    if (weight <= 0.0) {
        return 1.0;
    }
    // **The shadow fades to no-shadow over the last stretch**, rather than ending on a line a
    // player can find. Past the range the beam is drawn unshadowed but current, so a horizon
    // larger than this loses distant cast shadows and never shows stale ones.
    let fade = smoothstep(beam_shadow.split.y - beam_shadow.split.w, beam_shadow.split.y, dist);
    return mix(sum / weight, 1.0, fade);
}

// **The pass that fills the maps**: the same corner and origin the colour pass reads, placed
// by the beam's own camera. Two entry points because each depth map has its own matrix and a
// depth-only pipeline states its vertex stage by name.
@vertex
fn vs_shadow_near(in: VsIn) -> @builtin(position) vec4<f32> {
    return beam_shadow.near_vp * vec4<f32>(in.position + origins[in.chunk].xyz, 1.0);
}

@vertex
fn vs_shadow_far(in: VsIn) -> @builtin(position) vec4<f32> {
    return beam_shadow.far_vp * vec4<f32>(in.position + origins[in.chunk].xyz, 1.0);
}
// **`sky_model`, at binding 5, is `atmosphere.wgsl`'s** — the same buffer the sky pass binds, so
// a fluid reflects the sky that is overhead by construction rather than by agreement (C-177). It
// is *that file's* declaration rather than one here, because this pass and the sky pass hold one
// transport: a second declaration of the array and a second transcription of the integral is how
// the sky a frame draws and the sky a fluid reflects come to be two skies.

struct VsIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) material: u32,
    // Reserved: corners once carried the 256-level lattice sampled at the
    // corner's world address here. An octave-0 sample interpolated across a
    // merged rectangle is streaks rather than detail, and the fragment's own
    // ladder reads every rung the eye asked for — so builders write zeros and
    // the slots stay only so the vertex format does not move.
    @location(3) detail_roughness: f32,
    /// **The face this corner was cut from** (`Scene::Vertex::plane_axis`): 0, 1, 2, or
    /// `-1` where the geometry has no face. It sits in the second of the two `f32` the
    /// retired corner sample reserved, so the vertex format and its stride did not move. A
    /// rectangle states its plane from the record in `vs_quad`; a refined corner has no
    /// record, so the face travels with the vertex — the shaper's stretch needs only the
    /// axis, never the cell (C-163.2).
    @location(4) plane_axis: i32,
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
    // **The chunk this corner belongs to** — an index into `origins`, whose entry is that
    // chunk's `origin − eye` for the frame. A corner is chunk-local, the same way a record is,
    // so the camera reaches the vertex through this index and not by re-writing the array.
    @location(8) chunk: u32,
};

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) to_eye: vec3<f32>,
    @location(2) @interpolate(flat) material: u32,
    // The eye-relative position, interpolated: the fragment needs its own place in
    // the world's block grid, and this is it (see `eye_phase`).
    @location(3) eye_rel: vec3<f32>,
    // Reserved: interpolated corner detail used to arrive here. It carries
    // zeros now (see `VsIn`); the ladder is the whole address law.
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
    // The cloud's veil — docs/lighting-sky-and-moon.md
    @location(9) @interpolate(flat) veil: f32,
    // **Whether this corner carries a lattice sample at all.** Neither format samples the
    // lattice at a corner any more (`VsIn`), so both answer `0`; the flag stays for the day a
    // corner carries a sample again, and `1` is then a different claim from "the lattice said zero", and the fragment
    // has to tell the two apart to know when to read the Sun's angle instead
    // (`lod_reflectance_delta`). Flat, for the reason `light_present` is: it is a property of
    // the vertex's own format and not a quantity to blend.
    @location(10) @interpolate(flat) sample_present: u32,
    // **The rectangle's own plane, exactly** — `x` is the face's axis (0, 1, 2), `y` the
    // world subvoxel cell on it, and `x` is `-1` where the geometry is a corner rather than
    // a rectangle. A rectangle carries its plane in `slice`, an integer number of
    // subvoxels, so the cell on that axis is an integer the vertex stage can state; the
    // fragment's interpolated coordinate on the same axis is that plane through
    // `eye_rel + eye_phase`, rounded, and can land one rounding step *below* it — where
    // `floor` draws the whole face from the cell beside the face, and which side an eye
    // gets depends on where it stands. Flat for the reason `material` is: a plane does not
    // vary across the face it is the plane of.
    @location(11) @interpolate(flat) plane: vec2<i32>,
    /// **The face's own axis, stated by whichever stage built the geometry.** A rectangle
    /// computes it from the record; a corner carries it in the vertex format, because a
    /// refined triangle has no record to state its plane and `-1` would leave the shaper's
    /// stretch on the material's **world** axis — square cells on the face that looks along
    /// that axis, stretched ones on every other side (C-163.2).
    @location(12) @interpolate(flat) plane_axis: i32,
};

@vertex
fn vs_main(in: VsIn) -> VsOut {
    var out: VsOut;
    // **Chunk-local to eye-relative**, in one add: `origins[chunk]` is that chunk's
    // `origin − eye`, written once a frame for each chunk in the scene.
    let position = in.position + origins[in.chunk].xyz;
    out.clip = camera.vp * vec4<f32>(position, 1.0);
    out.normal = in.normal;
    // Geometry is now relative to the eye, so the eye is the origin here and the
    // view vector is just the way back to it. No uniform, and so no opportunity
    // for a uniform to disagree with the geometry.
    out.to_eye = -position;
    out.material = in.material;
    out.eye_rel = position;
    // Pass through the per-vertex light. Detail corners carry zeros (see
    // `VsIn`): the fragment's ladder reads every rung from the fragment's own
    // address.
    out.detail_roughness = in.detail_roughness;
    // The slot at 32 carries the face's axis, not this sample (see `VsIn`).
    out.detail_reflectance = 0.0;
    out.light_direct = in.light_direct;
    out.light_diffuse = in.light_diffuse;
    out.light_present = in.light_present;
    // **A corner is never a cloud.** The vertex format carries a light measurement, and a
    // measurement attaches to a cell of the world rather than to a medium, so this path is
    // terrain by construction and its veil is zero.
    out.veil = 0.0;
    // This format carries no lattice sample either: corners read zeros (see `VsIn`), so
    // where the ladder is silent a fragment falls back to the Sun-angle reading exactly as
    // the quad format does. Answering 1 here would read the zeros as a measured flat albedo.
    out.sample_present = 0u;
    // **A corner states no plane.** A refined rectangle's corners are off the plane its
    // normal names, so the fragment keeps the interpolated address for this format (see
    // `plane`); only `vs_quad` knows a plane to state.
    out.plane = vec2<i32>(-1, 0);
    // The face the refinement cut this corner from. A corner states no *plane* (its surface
    // is off the plane its normal names), but it does state the face's **axis**, which is
    // all the shaper's stretch needs.
    out.plane_axis = in.plane_axis;
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

/// **One corner of one record, in metres and eye-relative** — the quad format's whole
/// expansion, stated once so the shaded rectangle and its own outline cannot disagree about
/// where the rectangle is.
///
/// `corner` is `0..3` in `Quad::corners`' own order, and the winding flip is applied here rather
/// than by either caller.
fn quad_corner_m(quad_index: u32, corner_in: u32) -> vec3<f32> {
    let quad = quads[quad_index];
    var corner = corner_in;
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
    //
    // **The face is the tree's own plane**, so a flat boundary is one rectangle and two derivations of a tree draw the same bytes.
    return sub / f32(camera.rates.x) + origins[quad.chunk].xyz;
}

@vertex
fn vs_quad(@builtin(vertex_index) vertex_index: u32) -> VsOut {
    let quad = quads[vertex_index / 6u];
    // The two triangles `Scene` used to emit as `[0, 1, 2, 0, 2, 3]`.
    var order = array<u32, 6>(0u, 1u, 2u, 0u, 2u, 3u);
    let p = quad_corner_m(vertex_index / 6u, order[vertex_index % 6u]);

    var out: VsOut;
    out.clip = camera.vp * vec4<f32>(p, 1.0);
    out.normal = face_normal(quad.face);
    // **The face's plane, exactly.** `slice` is the plane in whole subvoxels from the
    // chunk's low corner, and a chunk origin is a whole metre, so this cell is an integer
    // the rectangle can state. The origin is recovered from the eye-relative origin here
    // because `rel + eye_voxel + eye_phase` *is* that integer metre up to `f32` rounding —
    // at any distance the world allows the error is far below half a metre, and `round`
    // maps it back exactly.
    let origin_m = round(
        origins[quad.chunk].xyz + vec3<f32>(camera.eye_voxel.xyz) + camera.eye_phase.xyz
    );
    let cells = vec3<i32>(origin_m) * i32(camera.rates.x);
    let plane_axis = face_axis(quad.face);
    out.plane = vec2<i32>(i32(plane_axis), cells[plane_axis] + i32(quad.slice));
    out.plane_axis = i32(plane_axis);
    // The eye is the origin here, exactly as in `vs_main`: the geometry arrived
    // already relative to it.
    out.to_eye = -p;
    // The fragment reads the composition here, not the quad's own material id — docs/lighting-sky-and-moon.md
    out.material = quad.composition & 0xFFFFu;
    out.eye_rel = p;
    // No per-vertex lattice sample and no per-vertex light — docs/lighting-sky-and-moon.md
    out.detail_roughness = 0.0;
    out.detail_reflectance = 0.0;
    out.sample_present = 0u;
    out.light_direct = vec3<f32>(0.0);
    out.light_diffuse = vec3<f32>(0.0);
    out.light_present = 0u;
    // **The veil rides the origin** (C-179). A cloud cell's own origin carries the
    // transmittance of the column it stands in, in the one float of that `vec4` the world
    // never used, so the veil needs no new field in the record and no new binding.
    out.veil = origins[quad.chunk].w;
    return out;
}

// ---- the astronomical body (C-184) ------------------------------------------
//
// A body's record is a rectangle's own stride. `vs_body` plots it at the far plane and `fs_body`
// intersects the **cuboid** the record is — the model's own `sun::BodyCuboid`, faces on the world's
// axes — and shades the face the ray enters by that face's own normal. The plot, the intersection and
// why a sphere was the wrong body are under *The astronomical body* in
// `docs/lighting-sky-and-moon.md`.

struct BodyOut {
    @builtin(position) clip: vec4<f32>,
    // Where in the plotted plane this pixel is, in `[-1, 1]` — the plane the cube is projected onto,
    // not the face of a disc. See `fs_body`.
    @location(0) offset: vec2<f32>,
    @location(1) @interpolate(flat) to_body: vec3<f32>,
    @location(2) @interpolate(flat) radiance: vec3<f32>,
    @location(3) @interpolate(flat) light: vec3<f32>,
    // The numbers the vertex stage plotted with, so the fragment can rebuild this pixel's own world
    // ray, intersect the body's cube and ask the air what it carried before the body. See `fs_body`.
    @location(4) @interpolate(flat) drawn_m: f32,
    @location(5) @interpolate(flat) plot_half_m: f32,
    // The body's own half-edge at the plotted distance — the cube's, which is not the quad's: the
    // quad is `√3` of it so that a rotated cube fits whatever way its corners face.
    @location(6) @interpolate(flat) body_half_m: f32,
};

// The body's own basis: `right`, `up`, and the direction to it. `helper` is the world axis the body
// is furthest from, so a body at the zenith still has one.
fn body_basis(to_body: vec3<f32>) -> mat3x3<f32> {
    let helper = select(vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(1.0, 0.0, 0.0), abs(to_body.y) > 0.9);
    let right = normalize(cross(helper, to_body));
    return mat3x3<f32>(right, cross(to_body, right), to_body);
}

@vertex
fn vs_body(@builtin(vertex_index) vertex_index: u32) -> BodyOut {
    let record = quads[vertex_index / 6u];
    let rel = origins[record.chunk].xyz;
    // The body's own half-extent, scaled to the distance it is drawn at, so what it subtends is what
    // the model says it subtends and not an angle chosen for the picture.
    let distance_m = max(length(rel), 1.0);
    // `sky_grid.x` is a distance **inside** the far plane rather than the plane itself
    // (`camera::BODY_PLOT_M`): a body exactly on the plane is discarded by the depth test, and the
    // reason is that number's own.
    let drawn_m = max(camera.sky_grid.x, 1.0);
    let to_body = rel / distance_m;
    // **The record's half-extent is the cube's half-edge**, so the body's silhouette is not a disc of
    // that radius: a face-on square reaches `√2` of it at its corners, and a cube seen corner-on
    // reaches `√(8/3)`. The plotted quad is `√3` of the half-edge, which contains both; the body
    // itself is the cube of `body_half_m` and `fs_body` intersects that, not the quad.
    let body_half_m = origins[record.chunk].w * drawn_m / distance_m;
    let plot_half_m = body_half_m * 1.7320508;
    let basis = body_basis(to_body);
    var order = array<u32, 6>(0u, 1u, 2u, 0u, 2u, 3u);
    let corner = order[vertex_index % 6u];
    let offset = vec2<f32>(
        select(-1.0, 1.0, corner == 1u || corner == 2u),
        select(-1.0, 1.0, corner == 2u || corner == 3u),
    );
    let centre = to_body * drawn_m + (basis[0] * offset.x + basis[1] * offset.y) * plot_half_m;

    var out: BodyOut;
    let clip = camera.vp * vec4<f32>(centre, 1.0);
    // **The plane's own depth, written rather than computed.** A body is plotted at the far plane
    // (`camera::BODY_PLOT_M`), and the projection's own `z/w` there is `1.0` — the value `camera.vp`'s
    // product only *approximates*, landing a few `f32` steps either side of it, which is how a body came
    // out drawn in part where the rounding fell below the clear value. `clip.z = clip.w` is the plane as
    // exactly as `f32` can hold it, and `fs_body`'s `LessEqual` is what draws it there.
    out.clip = vec4<f32>(clip.x, clip.y, clip.w, clip.w);
    out.offset = offset;
    out.to_body = to_body;
    out.drawn_m = drawn_m;
    out.plot_half_m = plot_half_m;
    out.body_half_m = body_half_m;
    // **The radiance rides the rectangle's own composition slot** (C-184): the record is one stride for
    // both units, so a body's three channels are the three slots `vs_quad` spends on a material id, a
    // slice and a `uv` — read here as `f32` bits rather than as the integers they are there.
    out.radiance = vec3<f32>(
        bitcast<f32>(record.composition),
        bitcast<f32>(record.slice),
        bitcast<f32>(record.uv),
    );
    out.light = vec3<f32>(
        bitcast<f32>(record.extent),
        bitcast<f32>(record.light_y),
        bitcast<f32>(record.light_z),
    );
    return out;
}

/// **The face of a world-axis-aligned cube a ray enters**, or the zero vector for a miss.
///
/// The slab test `sun::BodyCuboid::ray_hits` performs — the body's faces are the world's own axes
/// (`x` east, `y` up, `z` south), so the entry face's **normal is one of six exact vectors** and the
/// phase is which of them the light hits rather than a normal interpolated across a disc. A ray
/// parallel to a slab plane and outside that slab misses, by the same positive infinity the CPU's
/// reciprocal-floor test gets; a ray inside it takes no constraint from that axis.
fn cuboid_face(ray: vec3<f32>, centre: vec3<f32>, half: f32) -> vec3<f32> {
    var near = -1.0e30;
    var far = 1.0e30;
    var normal = vec3<f32>(0.0);
    if (abs(ray.x) > 1.0e-30) {
        let inv = 1.0 / ray.x;
        let a = (centre.x - half) * inv;
        let b = (centre.x + half) * inv;
        let lo = min(a, b);
        if (lo > near) {
            near = lo;
            normal = vec3<f32>(select(1.0, -1.0, ray.x > 0.0), 0.0, 0.0);
        }
        far = min(far, max(a, b));
    } else if (abs(centre.x) > half) {
        return vec3<f32>(0.0);
    }
    if (abs(ray.y) > 1.0e-30) {
        let inv = 1.0 / ray.y;
        let a = (centre.y - half) * inv;
        let b = (centre.y + half) * inv;
        let lo = min(a, b);
        if (lo > near) {
            near = lo;
            normal = vec3<f32>(0.0, select(1.0, -1.0, ray.y > 0.0), 0.0);
        }
        far = min(far, max(a, b));
    } else if (abs(centre.y) > half) {
        return vec3<f32>(0.0);
    }
    if (abs(ray.z) > 1.0e-30) {
        let inv = 1.0 / ray.z;
        let a = (centre.z - half) * inv;
        let b = (centre.z + half) * inv;
        let lo = min(a, b);
        if (lo > near) {
            near = lo;
            normal = vec3<f32>(0.0, 0.0, select(1.0, -1.0, ray.z > 0.0));
        }
        far = min(far, max(a, b));
    } else if (abs(centre.z) > half) {
        return vec3<f32>(0.0);
    }
    if (far < max(near, 0.0)) {
        return vec3<f32>(0.0);
    }
    return normal;
}

@fragment
fn fs_body(in: BodyOut) -> @location(0) vec4<f32> {
    let basis = body_basis(in.to_body);
    // The pixel's own world ray, rebuilt from the plane the vertex stage plotted it on.
    let raydir = in.to_body * in.drawn_m
        + (basis[0] * in.offset.x + basis[1] * in.offset.y) * in.plot_half_m;
    // **The body is the cuboid the model measured** — a cube of the record's half-extent on the
    // world's axes — and the pass intersects that cube rather than inscribing a disc in it. The
    // inscribed sphere was a second shape for one body: a face-on square covers `4/π` times the disc,
    // so the drawn body delivered `π/4` of the irradiance the law divides by its own `4·(R/d)²`, and
    // it drew a curved terminator on a body with six flat faces. No brightness is rescaled here to
    // make the two agree; the shape is what changed.
    let n = cuboid_face(raydir, in.to_body * in.drawn_m, in.body_half_m);
    if (dot(n, n) == 0.0) {
        discard;
    }
    // **A body is shaded by the material's own illumination terms, and this is where it was not.**
    // The line here was `in.radiance * max(dot(n, in.light), 0.0)` — a Lambert term of the body's
    // own, a **second and poorer shading model** for the two astronomical bodies, reached without
    // `materials`, without `npr`, and without the rim. A Lambert term is `0` across the whole
    // visible disc of a **new moon** (the Sun is behind it), so the body's own radiance — the
    // night's only light on the sky — drew a black disc over a black sky, which is the invisibility
    // this repairs. It is the same defect class the frame has paid for twice: one law written out
    // in two places, and the second copy is the one nobody checks.
    //
    // The two terms are `fs_main`'s own, **at its own powers** — the substrate's share and the rim,
    // which is *not* a decoration here: `beam_share` is `F(theta_L)`'s boundary share, so a light
    // **behind** the body (a new moon, exactly) still puts `radiance` on its limb and a body is
    // visible against a sky it is darker than. `fs_main` names them "the same power, so the two
    // terms are one law read at two angles"; this is that law, at a body's **face normal** rather
    // than a quad's.
    //
    // The eye direction is `-to_body`. A face turned at the eye has `ndotv = 1`; a face seen edge-on
    // has `0`, and the silhouette term is the rim along the cube's own edge — a box's chamfer rather
    // than a sphere's limb, which is the shape the record names.
    var lit = in.radiance;
    if (dot(in.light, in.light) > 0.0) {
        let eye = -in.to_body;
        let ndotv = clamp(dot(n, eye), 0.0, 1.0);
        let ndotl = clamp(dot(n, normalize(in.light)), -1.0, 1.0);
        let substrate = clamp(ndotl, 0.0, 1.0);
        let beam_share = pow(1.0 - clamp(ndotl, 0.0, 1.0), 5.0);
        let silhouette = pow(1.0 - ndotv, 5.0);
        lit = in.radiance * (substrate + beam_share * silhouette);
    }
    // **The body stands in air, and the air is not blanked by it.** A body is beyond every metre of the
    // atmosphere, so the pixel is what the ray carried before the body *plus* what the body sends — the
    // same `sky_radiance_rgb` the sky pass and a fluid's reflection evaluate, at this pixel's own ray. A
    // body's radiance alone would leave an unlit disc as a black orb over a lit sky, and `Rgba32Float`
    // has no blend, so the sum is taken here.
    let ray = normalize(raydir);
    var presentable = sky_radiance_rgb(ray) + lit;
    if (camera.screen.z > 0.0) {
        presentable = tone_map(presentable);
    }
    return vec4<f32>(dithered(presentable, in.clip.xy, camera.screen.y), 0.0);
}

// ---- the debug wireframe (F4): the merge's own rectangles, outlined ---------
//
// The debug view of the geometry, and not a second shading model: the caller withholds
// the world's own materials, so the frame is the sky and these lines. Each line is the
// record's own border — the expansion above, so an outline cannot sit anywhere but on
// the rectangle it outlines — and the depth cue is the distance, graded through the
// artists' wheel as the user asked (2026-10-04: a first white ink read poorly against
// the world).
//
// **This lives in the record format's own module, and that is the point.** The shipped
// frame is the quad format, which carries records and no corner vertex at all: a
// wireframe drawn from corner vertices would have had nothing to draw, which is what the
// retired `wire.wgsl` was — a `LineList` over geometry the game never builds.

/// The gradient's declared range and sweep: hue `WIRE_NEAR_HUE` at `WIRE_NEAR_M`,
/// `WIRE_FAR_HUE` `WIRE_OCTAVES` doublings further out.
///
/// **The ramp is in octaves, because one frame carries two scales.** `F4` outlines the terrain
/// *and* the cloud — `Scene::build_with_detail` puts both in the same record list, and the game
/// appends the cloud to it — and they do not live in the same range of distances: a mesa is 4 m to
/// a few hundred metres away and a cumulus is 500 m to 6 km. A **linear** ramp cannot hold both.
/// The retired `wire.wgsl` swept 4 m to 256 m, which suit the mesh the terrain draws and spends
/// itself long before the sky: every cloud line arrived one flat violet, measured on the shipped
/// field at the shipped tick as a frame with no depth cue in it at all. Widening that ramp's far
/// end to the field's 8 km then flattened the *terrain* instead — a hill 30 m away and a ridge
/// 300 m away became the same red — which is the same fault from the other side.
///
/// The ladder is what makes a logarithmic ramp the honest one: **every distance in this view that
/// matters is a power of two from the last**, because the rung the rectangle was drawn at is
/// chosen by distance and doubles (`cloud_cubes::rung_for`, `pattern`'s own ladder). One sweep in
/// octaves therefore reads the rung rather than the metre: red at `WIRE_NEAR_M` where the finest
/// cells are, violet at the far edge of the field where they are coarsest, and a terrain face and
/// a cloud face at the same *rung* share a hue however far apart they are.
const WIRE_NEAR_M: f32 = 4.0;
/// `4 m · 2^11 = 8192 m` — the cloud field's own span, and the far end of the drawn ladder.
const WIRE_OCTAVES: f32 = 11.0;
const WIRE_NEAR_HUE: f32 = 0.0;
const WIRE_FAR_HUE: f32 = 0.7;

/// The artists' wheel at full saturation and value, as a closed form: red at 0,
/// through yellow, green and cyan, to blue near 2/3. (Inigo Quilez's polynomial,
/// the same one every shader palette uses; written out rather than sampled from
/// a table because there is no table to sample.)
fn hue_rgb(h: f32) -> vec3<f32> {
    let x = h * 6.0 + vec3<f32>(0.0, 4.0, 2.0);
    let k = x - 6.0 * floor(x / 6.0);
    return clamp(abs(k - 3.0) - 1.0, vec3<f32>(0.0), vec3<f32>(1.0));
}

struct WireOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) distance_m: f32,
};

/// **Four border edges per rectangle and no diagonal**, so what is outlined is the greedy
/// merge's answer rather than the triangulation's: eight line vertices a record, walking
/// `0-1-1-2-2-3-3-0` around the four corners.
@vertex
fn vs_wire(@builtin(vertex_index) vertex_index: u32) -> WireOut {
    var order = array<u32, 8>(0u, 1u, 1u, 2u, 2u, 3u, 3u, 0u);
    let p = quad_corner_m(vertex_index / 8u, order[vertex_index % 8u]);
    var out: WireOut;
    out.clip = camera.vp * vec4<f32>(p, 1.0);
    // The geometry arrives eye-relative, so the eye is the origin here and the distance is
    // the length of the position itself — the same fact `vs_quad` relies on, and no uniform
    // to disagree with it.
    out.distance_m = length(p);
    return out;
}

@fragment
fn fs_wire(in: WireOut) -> @location(0) vec4<f32> {
    let octaves = log2(max(in.distance_m, WIRE_NEAR_M) / WIRE_NEAR_M);
    let t = clamp(octaves / WIRE_OCTAVES, 0.0, 1.0);
    return vec4<f32>(hue_rgb(mix(WIRE_NEAR_HUE, WIRE_FAR_HUE, t)), 1.0);
}

// the pattern's own arithmetic (C-161, C-162, laddered by C-163) — docs/lighting-sky-and-moon.md

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
// `address` is `x | y << bits | z << (2 * bits)` — `x` is the fast axis, packed at
// the side's own bit width (`DetailLattice::flat`, four bits an axis at 16 samples
// a side), which the caller derives from the uniform rather than stating.
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
// **The hemisphere's cosine-weighted integral of a uniform radiance** — `skylight::WorldLight`'s
// `assemble` applies the same π to its stencil's mean, so a corner with the atmosphere's one ray
// and no stencil reads the same kind of number as a measured corner's diffuse band.
const SKY_IRRADIANCE: f32 = 3.14159265358979;
// `pattern::OCTAVE_GAIN`: what each coarser rung weighs against the one below.
// 0.5 ships with the Rust: a GPU A/B (2026-10-14, `pattern-flicker.rs`) moved
// pixel totals ~1% for 0.7, so the look stays while the flicker is fixed where
// it actually lives (coverage, not the blend).
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
// `pattern::octave_weight_staged`, route a (C-171) — docs/lighting-sky-and-moon.md
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

// the deviation this fragment shows, with its gains applied — docs/lighting-sky-and-moon.md
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

// the LODs: a reflectance that is a function of the Sun's angle — docs/lighting-sky-and-moon.md
const LOD_REFLECTANCE_GAIN: f32 = 1.0;

/// The reflectance deviation a fragment with **no address to read** shows, from the Sun's own
/// angle: `0` where the face meets both the beam and the eye head-on, `1` at a doubly grazing
/// incidence, continuous between. `reflectance_gain` is the material's own, so a material the
/// world does not vary stays unvaried here too (`ShadingField::gain`).
fn lod_reflectance_delta(n: vec3<f32>, v: vec3<f32>, reflectance_gain: f32) -> f32 {
    let beam = normalize(rig.lights[0].direction.xyz);
    let cos_beam = clamp(dot(n, beam), 0.0, 1.0);
    let cos_view = clamp(dot(n, v), 0.0, 1.0);
    let at_beam = pow(1.0 - cos_beam, 5.0);
    let at_view = pow(1.0 - cos_view, 5.0);
    // Union rather than product: a face the *beam* grazes reflects the beam whether or not the
    // eye grazes it too, and a face the *eye* grazes shows more of whatever it reflects.
    let share = at_beam + at_view - at_beam * at_view;
    return clamp(LOD_REFLECTANCE_GAIN * share, 0.0, 1.0) * reflectance_gain;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    if (in.material >= arrayLength(&materials)) {
        // A quad naming a material outside the table is not shaded at all. It
        // is not shaded grey: no admitted input means no picture.
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    // **A cloud's side and its veil in one signed number** (C-183): `±(1 + opacity)`, positive for
    // the newer tree a frame is joining, negative for the older, `0` for terrain — so the side needs
    // no second attribute, and a cloud, whose encoding is never zero, cannot be read as a surface.
    let is_cloud = in.veil != 0.0;
    // **The weight is eased** (C-183), with the built-in `smoothstep`, which is symmetric — so the two sides stay complementary.
    let cloud_weight = select(
        1.0 - smoothstep(0.0, 1.0, camera.screen.w),
        smoothstep(0.0, 1.0, camera.screen.w),
        in.veil > 0.0,
    );
    // **The interpolation, once per pixel**: a veil is a transmittance, so it fades in *opacity* —
    // `veil · w` would move a wisp and a core by one number of thousandths, not one fraction.
    let cloud_veil = 1.0 - (abs(in.veil) - 1.0) * cloud_weight;
    // **A cloud has no row because it has no surface** (C-178). Its record carries the empty
    // material id, so the row is built from the medium's own two numbers — its albedo and its
    // transmittance, both sent — and the *same* terms below shade it. Nothing about the look is
    // written twice; only where the row came from differs.
    var m = materials[in.material];
    if (is_cloud) {
        m = cloud_row(camera.sky_grid.w);
    }
    let n = normalize(in.normal);
    let v = normalize(in.to_eye);
    let ndotv = clamp(dot(n, v), 0.0, 1.0);

    // the material's own pattern, read at this fragment's address — docs/lighting-sky-and-moon.md
    let footprint = eye_footprint(in.eye_rel, n, v);
    // The ladder speaks where it can and fades to the fallback across its top rung's
    // band — docs/lighting-sky-and-moon.md
    let vertex_deviation = vec2<f32>(in.detail_roughness, in.detail_reflectance)
        * vec2<f32>(m.pattern.x, m.pattern.y);
    let ladder = pattern_deviation(in.material, m, in.eye_rel, footprint);
    // **What a corner with no sample of its own shows instead**: the Sun's own angle
    // (`lod_reflectance_delta`). Roughness stays unvaried — it has no angle law here, and a
    // roughness guessed from geometry would be an invention rather than a reading.
    let lod = vec2<f32>(0.0, lod_reflectance_delta(n, v, m.pattern.y));
    let with_a_sample = select(lod, vertex_deviation, in.sample_present != 0u);
    let deviation = mix(with_a_sample, ladder.xy, ladder.z);

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
    // the parameters the CPU's model reads and the GPU had nowhere to put — docs/lighting-sky-and-moon.md
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

    // **The atmosphere lights the material, and it arrives as the frame's own two numbers.**
    // — PATCH-074, PLAN-2026-10-23-the-one-rig.md's M1.
    //
    // `rig.ambient` is not a designer's constant here. `skylight::daylight_of` fills it from the
    // atmosphere's own dome — the horizon ring at four azimuths plus the zenith, at
    // `AMBIENT_FROM_HORIZON` each — and `skylight::exposed_at` scales it with the beam at the
    // frame's own `EXPOSURE · grade`, which is why it can be added to the beam's triple directly.
    // The beam is the frame's one Sun/Moon, the solver's own `beam_rgb` at that same exposure,
    // gated by this frame's shadow map — the same map that already streams for shadows.
    //
    // **This is the frame's light in one uniform and one shadow tap, and that is the point.**
    // PATCH-074 first transcribed the sky as `π · sky_radiance_rgb(n)` per fragment. That samples
    // the model's *zenith* for an up-facing normal — the darkest direction a single-scattering
    // sky has (`0.591` horizon against `3.9e-5` zenith at the crossing, `skylight.rs`'s own
    // figures) — so the ground went black while the trunks' horizon-facing sides stayed lit, which
    // is exactly the picture the player reported. It was also eight band iterations of `exp` per
    // fragment against a shadow map's one tap, which is why it streamed slower than the shadows.
    // The dome's mean is frame-global and the CPU already computes it; the shader samples it.
    //
    // **The per-corner bands are no longer a light.** They are the CPU field's measurement, and
    // that field is a demand queue marched cell by cell; reading it here made the light arrive
    // chunk by chunk, long after the geometry, and made the shading differ between a chunk that
    // had been measured and one that had not. The shader no longer branches on `light_present`,
    // and both branches of the retired law are gone. The vertex format still carries the bands (a
    // mesh is a format's, not this pass's to shrink); nothing in this file reads them.
    var sky = rig.ambient.rgb;
    var beams = vec3<f32>(0.0);
    // **The beam's direction is the frame's one Sun/Moon direction**, read from the rig uniform;
    // it is the direction *of the beam*, not a second light. A per-corner direction is the next
    // slice's input (M1.2/M2.0), and until it exists this is the only direction the frame has.
    var beam_dir = vec3<f32>(0.0, 1.0, 0.0);
    var beam_known = false;
    if (rig.count.x > 0u) {
        beams = rig.lights[0].colour.rgb * rig.lights[0].direction.w;
        beam_dir = normalize(rig.lights[0].direction.xyz);
        beam_known = true;
    }
    // **The map says how much of the beam arrives; it does not replace the beam.** It is drawn
    // from the scene's corners every frame and covers the shipped window, so the beam follows an
    // edit on the frame the new mesh is drawn, past any cell a build last asked about. It is the
    // **far** field's answer, though: a 0.56 m texel with a 2 cm normal bias, which at a grazing
    // beam cuts a straight line across a face the geometry says is lit. The subvoxel march below
    // is finer than that, so it refuses this map wherever it has a measurement of its own.
    var beam_visibility = 1.0;
    if (beam_shadow.direction.w > 0.5) {
        beam_visibility = sun_visibility(in.eye_rel, n);
    }
    // **The streamed horizon, at the fragment's own cell.** The shadow map can say whether the
    // beam is behind the world from one direction; it cannot say how much of the **dome** a cell
    // can see, and it knows nothing about the air the far field crossed. This is the second
    // measurement: the same resident octrees the world is meshed from, stamped a metre at a time
    // as the chunks stream in, marched here toward the beam and along the stencil's three sky
    // rays. The beam keeps the shadow map's finer texels and takes the march as well — a cell
    // behind a wall the map's box does not cover is still dark — and the ambient is the march's
    // own share of the atmosphere's radiance.
    //
    // **The Moon is in the two colours, not in a third ray.** At night `beams` is the solver's
    // `moonlight` — the Sun's bands reflected off the Moon's own declared reflectance — and by day
    // the Moon's scattered term is already inside `sky_radiance_rgb`, which is where `rig.ambient`
    // came from. The march decides how much of either reaches this cell, so a cave under a cliff
    // loses the moonlight it cannot see rather than receiving a uniform night.
    //
    // **Half a cell along the normal** leaves the fragment's own cell, which is the geometry it
    // is drawn on; without it every surface would shadow itself on the first tap.
    let horizon_point = in.eye_rel + n * (HORIZON_CELL_M * 0.5);
    if (beam_known) {
        // **The water the beam crossed, on the beam's own ray.** The air's reddening is already in
        // `lights[0]` (`Sky::sun` is the transmitted band); the droplets above *this* fragment are
        // not, and they are what a beam under a cloud bank loses. Applied before the visibility
        // terms and touching neither of them.
        beams *= beam_cloud_filter(in.eye_rel, beam_dir);
        let marched = horizon_transmittance(horizon_point, beam_dir, HORIZON_REACH_M);
        // **Only the band that is finer than the map may overrule it, in either direction.** The
        // fine band reads the world's own subvoxels as far as its box reaches; inside that range
        // the map's half-metre texel and its 2 cm normal bias are not a second opinion — they are
        // a coarse one, which is what drew a straight edge across a face the geometry says is lit.
        // So a fine hit is a refusal the map cannot argue with, and a fine clear is the beam.
        //
        // **The coarse band is refused here and answers the dome only.** Past the fine box it is
        // an 8 m summary: an occluder it finds is real, but so is the lit ground beside it, and
        // the shadow map still covers that range at 0.28 m. Letting eight metres subtract from
        // the beam is how the far field grew blocks of shadow the geometry does not cast — the
        // regression this rule removes. Unmeasured entirely, the map answers, which is the light
        // the far field had before the volume existed.
        if (marched.x > 0.5) {
            beam_visibility = 0.0;
        } else if (marched.y > 0.5) {
            beam_visibility = 1.0;
        }
    }
    beams *= beam_visibility;
    // **The dome, and the enclosure's own one bounce.** A cell that can see none of the sky is not
    // unlit: the walls it can see are lit by the same dome and reflect their own share of it, so
    // the ambient it receives is `view + (1 − view)·albedo` — the fragment's own measured
    // reflectance, not a constant. That is what carries light down a corridor instead of letting the
    // ambient die at the first metre of cover, and it is why a tunnel mouth's light fades with depth
    // rather than stopping where the rig's floor used to.
    let dome = horizon_dome(horizon_point);
    if (dome.y > 0.5) {
        let reflecting = select(vec3<f32>(1.0), base, base_present > 0.5);
        let enclosure = clamp(dot(reflecting, vec3<f32>(0.2126, 0.7152, 0.0722)), 0.0, 1.0);
        sky *= dome.x + (1.0 - dome.x) * enclosure;
    }
    let opaque = (1.0 - metalness) * (1.0 - transmission);
    // **`npr::shade`'s ambient gain, which this transcription dropped** — PATCH-074. A
    // transmissive surface receives the sky from every direction rather than only above it, and a
    // face with holes receives less of everything: `(1 + transmission·(1−depth)) · (1−open)`. The
    // CPU has always multiplied the ambient by it; the shader did not, so a fluid (transmission 1,
    // opaque 0) lost its whole body term and drew 32× darker than the model — the red water test.
    let transmitted = transmission * (1.0 - depth);
    let ambient_gain = (1.0 + transmitted) * (1.0 - open);
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
        colour += base * (beams * (wrap * opaque * substrate) + sky * ambient_gain);
    }

    // the illumination terms (2 — rim) — docs/lighting-sky-and-moon.md
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

    // the illumination terms (3 — the specular warp) — docs/lighting-sky-and-moon.md
    if (rough_present > 0.5 && weight_present > 0.5 && beam_known) {
        let half_v = normalize(beam_dir + v);
        let facing = clamp(dot(n, half_v), 0.0, 1.0);
        if ((1.0 - facing) <= roughness) {
            colour += beams * (weight * f0);
        }
    }

    // Emission is not a Source 2 term. It is the OpenPBR input a dynamic block
    // emitter carries, and it is added last because it depends on no light.
    if (emission_present > 0.5) {
        colour += emission_rgb * emission_luminance;
    }

    // the fluid reflection: the sky, seen off the boundary — docs/lighting-sky-and-moon.md
    if (transmission > 0.0 && arrayLength(&sky_model) >= SKY_MODEL_ROWS) {
        // `reflect(−v, n)` with WGSL's own arithmetic is `−v + 2·(v·n)·n`, which is
        // `fluid::reflected_ray`'s expression exactly — the same convention (`v` points from
        // the surface toward the eye) and the same result, so there is no sign for the two to
        // disagree about.
        let ray = reflect(-v, n);
        var reflected = vec3<f32>(0.0);
        if (ray.y > 0.0) {
            // **The transport, at the mirror ray's own direction** — one integral, the same one
            // the sky pass evaluates for the pixel a mirror of this one would hold. There is no
            // elevation and no azimuth to convert, because the model takes the world direction
            // both passes already have.
            reflected = sky_radiance_rgb(ray);
        }
        // **The water's boundary carries the material's LOD reading** (`lod.y`, the very
        // `lod_reflectance_delta` every surface gets): the reflectance deviation applied to the one
        // quantity water's appearance is made of. A flat sheet used a constant `WATER_F0` and so
        // had no LOD material rendering at all — the reported gap. `lod.y` and not `deviation.y`,
        // because the ladder's own coverage must not change a boundary's reflectance here: the
        // CPU mirror reads this same function, and the two are held by the water test's pixels.
        let f0_lod = clamp(camera.sky_grid.z + lod.y * (1.0 - camera.sky_grid.z), 0.0, 1.0);
        let fresnel = schlick(clamp(dot(n, v), 0.0, 1.0), f0_lod);
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
    // that is *not* the sky standing behind it: `cloud_veil` is the cell's column transmittance
    // at this frame's fade weight, so `1 − cloud_veil` is exactly the share the cloud contributes
    // — the same fraction the light sampler walks in `voxel-stream::light`, so the picture and the
    // shadow cannot disagree. Terrain keeps `1.0`, which is alpha's existing meaning here:
    // geometry is on this pixel.
    let alpha = select(1.0, 1.0 - cloud_veil, is_cloud);
    return vec4<f32>(dithered(presentable, in.clip.xy, camera.screen.y), alpha);
}
