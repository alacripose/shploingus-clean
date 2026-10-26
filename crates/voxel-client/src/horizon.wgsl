// **The streamed light horizon**: the two bands the fragment marches, their address laws, and the
// dome. `voxel_client::horizon` owns the state these read — this file owns only the reading of it.
//
// Composed by `world::pass_source` between `atmosphere.wgsl` and the pass that draws with it, so a
// binding number here is that file's neighbour rather than a fact of this one.

// **The streamed light horizon** — one word per metre of the world around the eye, stamped from
// the resident chunks' subvoxel octrees (`voxel-client/src/horizon.rs`) and streamed in as those
// chunks land. `to_anchor` is the cube's world corner in this frame's eye-relative frame, so the
// fragment subtracts a small number instead of a world coordinate; `params` is `(metres per
// cell, cells x, cells y, cells z)`. A cell that is non-zero is solid, and a ray that meets one
// delivers nothing.
struct HorizonView {
    // The fine band's corner in the eye's frame, then its `(metres per cell, cells x, y, z)`.
    to_anchor: vec4<f32>,
    near: vec4<f32>,
    // The coarse band's corner in the eye's frame, then its own cell and dimensions.
    to_far: vec4<f32>,
    far: vec4<f32>,
    // **Which chunk slots the fine band has looked at**: bit `sx + 3·(sy + 3·sz)` of `x`, one per
    // 32 m slot of the 3×3×3 cube. A cell in an unset slot is *unmeasured*, not air — zero means
    // "nothing stands here" and that is a claim only a stamped chunk may make. The coarse band
    // carries its own "stamped" bit per cell instead, because one word there is already spent.
    coverage: vec4<u32>,
};
@group(0) @binding(10) var<storage, read> horizon_cells: array<u32>;
@group(0) @binding(11) var<uniform> horizon: HorizonView;
@group(0) @binding(12) var<storage, read> horizon_far: array<u32>;

// **The host's two numbers, restated because a shader cannot import Rust** —
// `voxel_client::HORIZON_CELL_M` and `HORIZON_REACH_M`, held to these literals by
// `horizon::tests::the_shaders_reach_and_the_hosts_are_one_number`.
const HORIZON_CELL_M: f32 = 1.0;
// **The fine band's whole span**, so the subvoxel answer is used everywhere it exists; the coarse
// band continues past it. A ray leaves the fine cube sooner than this near its faces, and the
// traversal reports where.
const HORIZON_REACH_M: f32 = 96.0;
// **The CPU sampler's own sky reach**, kept because `horizon::tests::the_shaders_reach_and_the_
// hosts_are_one_number` holds the host and this file to one literal. The dome no longer marches
// this far: its rays are as long as the volume, so a corridor's ambient stops at its far end.
const HORIZON_SKY_REACH_M: f32 = 32.0;
// **The coarse band**: 8 m cells over 256 m. It is what lets a beam cross a kilometre at a cost a
// frame can carry, and its grain is the second rung of the chunk mesh's own ladder.
const HORIZON_FAR_REACH_M: f32 = 160.0;
// The coarse word's share and stamped bit, `voxel_client::horizon`'s `FAR_SHARE`/`FAR_MEASURED`.
const FAR_SHARE: u32 = 0xFFu;
const FAR_MEASURED: u32 = 256u;
// The cell's own grain, restated from `voxel_client::HORIZON_SUBVOXEL_SIDE` and held to it by
// the same witness: the octree authors a voxel as 4×4×4 subvoxels and the mask has one bit each.
const HORIZON_SUBVOXEL_SIDE: u32 = 4u;
const HORIZON_SUBVOXEL_M: f32 = HORIZON_CELL_M / f32(HORIZON_SUBVOXEL_SIDE);
// **Cells along one chunk's edge**, `CHUNK_SIDE` restated: the volume's coverage mask is one bit
// per 32 m slot, and `horizon_stamped` divides a cell's index by this.
const CHUNK_SIDE_CELLS: u32 = 32u;
// **A partly-dug voxel costs at most one subvoxel's worth of steps**: 4 subvoxels crossed at
// half-subvoxel stride, with one spare for the boundary. A whole block never enters the loop.
const HORIZON_SUB_STEPS: i32 = 10;
// **The traversal's own bound**, in cells: a ray crossing all three axes of the fine band visits
// fewer than `96 · 3 + 1`, so this is a ceiling and not a budget the march can hit unseen. It was
// 128 when the reach was 32 m; a bound below the geometry's own count turns a long diagonal into
// "unmeasured", which hands the answer back to the shadow map on exactly the rays a corner casts.
const HORIZON_MAX_STEPS: i32 = 320;

// **The cascade handover's width, in texels of the coarser map** — see `sun_visibility`. The
// materials' ladder fades a rung out at a quarter of its cell (`octave_weight`); a cascade is not
// a rung on that ladder but a *level* of one shadow, and the level it hands over to is nine
// texels coarser, so the band is measured in the coarse map's own texels.
const SHADOW_BLEND_TEXELS: f32 = 2.0;
// How far inside a map's border its weight ramps to zero, in that map's own texels: enough that a
// cascade's box edge is a ramp like the split, not a line where one level stops answering.
const SHADOW_EDGE_TEXELS: f32 = 2.0;

/// **One voxel's 4×4×4 occupancy**, as the two words `voxel_client::horizon` stamped.
///
/// `x` is the low word, `y` the high one, and bit `x + 4y + 16z` of the pair is the subvoxel at
/// that address — `SubvoxelIndex::flat` in `voxel-model`, which is the order the octree's leaf
/// shapes are authored in. Two words because WGSL has no `u64`.
fn horizon_mask(i: vec3<i32>) -> vec2<u32> {
    let n = i32(horizon.near.y);
    let index = 2u * u32(i.x + n * (i.y + n * i.z));
    return vec2<u32>(horizon_cells[index], horizon_cells[index + 1u]);
}

/// One coarse band cell's word: bit 0 matter, bit 1 stamped.
fn horizon_far_word(i: vec3<i32>) -> u32 {
    let n = i32(horizon.far.y);
    return horizon_far[u32(i.x + n * (i.y + n * i.z))];
}

/// **Has the volume looked at the chunk slot this cell belongs to?**
///
/// A cube is 96 cells and a chunk is 32, so `i / 32` is the slot on each axis — the anchor is a
/// chunk's own corner, which is what makes that division exact. An unstamped slot is not air: it
/// is a chunk that has not landed, and the march reports no measurement there rather than an
/// unshadowed one.
fn horizon_stamped(i: vec3<i32>) -> bool {
    let slot = i / vec3<i32>(i32(CHUNK_SIDE_CELLS));
    let bit = u32(slot.x + 3 * (slot.y + 3 * slot.z));
    return ((horizon.coverage.x >> bit) & 1u) != 0u;
}

/// **Is the subvoxel this cube-relative point stands in solid?**
///
/// `world` is the point relative to the cube's own corner, in metres. The cell is its floor and
/// the subvoxel is the fractional part at `HORIZON_SUBVOXEL_SIDE` grain, which is the whole of the
/// address law: no second grid, no per-metre boolean. This is the question
/// `voxel_stream::light::transmittance_tree` asks of the same leaves on the CPU.
fn horizon_subvoxel_solid(world: vec3<f32>) -> bool {
    let cell = floor(world / HORIZON_CELL_M);
    let inside = fract(world / HORIZON_CELL_M) * f32(HORIZON_SUBVOXEL_SIDE);
    let at = clamp(vec3<i32>(floor(inside)), vec3<i32>(0), vec3<i32>(i32(HORIZON_SUBVOXEL_SIDE) - 1));
    let mask = horizon_mask(vec3<i32>(cell));
    let ordinal = u32(at.x + i32(HORIZON_SUBVOXEL_SIDE) * (at.y + i32(HORIZON_SUBVOXEL_SIDE) * at.z));
    let word = select(mask.y, mask.x, ordinal < 32u);
    return ((word >> (ordinal % 32u)) & 1u) != 0u;
}

/// **The stretch of a partly-occupied voxel this ray crosses**: `true` when it meets matter.
///
/// A voxel is entered through one subvoxel and left through another, so testing the entry alone
/// would let a beam through a dug corner and out through the rock behind it. This is the
/// cell-by-cell answer `raycast::Spans` gives for a `Mixed` leaf, at the subvoxel's own stride —
/// and it is only ever paid for a voxel whose mask is **partial**: an empty one costs a compare
/// and a full one is an answer, so a world of whole blocks never enters here.
fn horizon_partial_blocked(local: vec3<f32>, d: vec3<f32>, enter: f32, exit: f32) -> bool {
    var t = enter + HORIZON_SUBVOXEL_M * 0.5;
    for (var step = 0; step < HORIZON_SUB_STEPS; step = step + 1) {
        if (t >= exit) {
            break;
        }
        if (horizon_subvoxel_solid(local + d * t - horizon.to_anchor.xyz)) {
            return true;
        }
        t = t + HORIZON_SUBVOXEL_M * 0.5;
    }
    return false;
}

/// **One band's traversal**: `vec4(distance, blocked, left_at, measured)`. The fine band's
/// `blocked` is a hit, `0` or `1`; the coarse band's is the share of the ray it stopped, so it
/// accrues across cells and reads `1 − transmittance` rather than a flag.
///
/// Amanatides & Woo's walk — the same one `raycast::Crossings` and
/// `SparseVoxelOctree::raycast_local` use — so **every cell the ray passes through is visited in
/// order**, and a wall a metre thick cannot fall between two samples however oblique the ray is.
/// `band` is 0 for the octree's own subvoxel grain and 1 for the coarse 8 m summary; the coarse
/// band is what makes a kilometre of tunnel one hundred compares instead of a thousand, and it is
/// the same trade the chunk meshes make one rung out.
///
/// **`from_t` is where the caller's own ray has already walked**, so the bands are one march and
/// not two: the coarse band starts at the distance the fine one stopped measuring.
///
/// **The fragment's own cell is not tested**, because the surface is drawn on its boundary — the
/// rule `transmittance_tree`'s `skip_own_voxel` states, arrived at here by advancing before
/// testing. A cell with no bits costs one compare, a fully-solid cell ends the walk, and a
/// partly-dug one is walked at subvoxel grain.
///
/// **`measured` is what makes the answer usable as an authority**: leaving the box is `left_at`
/// with `measured` clear if the cell there was never stamped, so the caller keeps the coarser
/// shadow map rather than believing an answer the volume does not have.
fn horizon_walk(
    to_anchor: vec3<f32>,
    cells: vec3<f32>,
    cell_m: f32,
    band: u32,
    local: vec3<f32>,
    d: vec3<f32>,
    from_t: f32,
    reach_m: f32,
) -> vec4<f32> {
    let rel = (local + d * from_t - to_anchor) / cell_m;
    if (any(rel < vec3<f32>(0.0)) || any(rel >= cells)) {
        return vec4<f32>(0.0, 0.0, from_t, 0.0);
    }
    var i = vec3<i32>(floor(rel));
    // **The distance to this cell's next face on each axis**, and the stride to the one after it:
    // a zero component is infinity, so the walk never steps on an axis the ray does not cross.
    let f = rel - floor(rel);
    var t_max = vec3<f32>(1e30);
    var walk = vec3<i32>(0);
    for (var axis = 0; axis < 3; axis = axis + 1) {
        if (d[axis] > 0.0) {
            t_max[axis] = (cell_m - f[axis] * cell_m) / d[axis];
            walk[axis] = 1;
        } else if (d[axis] < 0.0) {
            t_max[axis] = (f[axis] * cell_m) / -d[axis];
            walk[axis] = -1;
        }
    }
    let t_delta = vec3<f32>(cell_m) / max(abs(d), vec3<f32>(1e-20));
    // **The coarse band lets its share through**: a cell eight metres wide holds a fraction of
    // matter, so the ray keeps that fraction and walks on. Nothing accumulates in the fine band,
    // so `blocked` there is still a hit and not a share.
    var coarse_clear = 1.0;
    for (var step = 0; step < HORIZON_MAX_STEPS; step = step + 1) {
        // The axis whose next face comes first. Ties are resolved in x, y, z order, which is a
        // corner a ray touches at a point and not a cell it passes through.
        var axis = 0;
        if (t_max.y <= t_max.x && t_max.y <= t_max.z) {
            axis = 1;
        } else if (t_max.z < t_max.x) {
            axis = 2;
        }
        let enter = from_t + t_max[axis];
        if (enter >= reach_m) {
            return vec4<f32>(0.0, 1.0 - coarse_clear, reach_m, 1.0);
        }
        i[axis] = i[axis] + walk[axis];
        let exit = from_t + t_max[axis] + t_delta[axis];
        t_max[axis] = t_max[axis] + t_delta[axis];
        if (any(i < vec3<i32>(0)) || any(i >= vec3<i32>(cells))) {
            return vec4<f32>(0.0, 1.0 - coarse_clear, enter, 1.0);
        }
        if (band == 0u) {
            if (!horizon_stamped(i)) {
                return vec4<f32>(0.0, 0.0, enter, 0.0);
            }
            let mask = horizon_mask(i);
            if (mask.x == 0u && mask.y == 0u) {
                continue;
            }
            if (mask.x == 0xFFFFFFFFu && mask.y == 0xFFFFFFFFu) {
                return vec4<f32>(enter, 1.0, enter, 1.0);
            }
            if (horizon_partial_blocked(local, d, enter, min(exit, reach_m))) {
                return vec4<f32>(enter, 1.0, enter, 1.0);
            }
        } else {
            let word = horizon_far_word(i);
            if ((word & FAR_MEASURED) == 0u) {
                return vec4<f32>(0.0, 1.0 - coarse_clear, enter, 0.0);
            }
            coarse_clear = coarse_clear * (1.0 - f32(word & FAR_SHARE) / 255.0);
        }
    }
    return vec4<f32>(0.0, 1.0 - coarse_clear, reach_m, 0.0);
}

/// **What one direction carries from an eye-relative point**: `x` is **stopped in the fine band**,
/// `y` **whether the fine band walked the whole reach**, `z` stopped in the coarse band, and `w`
/// whether the coarse one walked its whole reach.
///
/// **The fine band answers first and the coarse band carries on from where it stopped.** A cell's
/// beam crosses the subvoxel geometry of the 96 m box and then, if it is still going, as much of
/// 256 m of 8 m summary as the caller's reach allows — so a corridor a hundred metres long is
/// occluded at its own end rather than at the fine box's edge, and the answer is eight times
/// cheaper per metre exactly where one coarse cell is smaller than the pixel looking at it.
///
/// **`y` and `z` are separate because the caller trusts them differently.** Inside the fine band
/// the march is finer than the shadow map's 0.28 m texel and may overrule it; past the band it is
/// coarser, and only a *hit* there is taken — an occluder adds up, but a clear answer eight metres
/// wide is not evidence that the map's half-metre one is wrong.
fn horizon_transmittance(local: vec3<f32>, d: vec3<f32>, reach_m: f32) -> vec4<f32> {
    let fine = horizon_walk(
        horizon.to_anchor.xyz, horizon.near.yzw, horizon.near.x, 0u, local, d, 0.0, reach_m,
    );
    if (fine.y > 0.5) {
        return vec4<f32>(1.0, 1.0, 0.0, 0.0);
    }
    if (fine.w < 0.5) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }
    // **A reach that ends with the fine band never pays for the coarse one.** The beam asks for
    // `HORIZON_REACH_M` and refuses the coarse band's answer, so walking it would be a measurement
    // nobody reads; the dome asks for more and gets both.
    if (reach_m <= fine.z + 0.001 || reach_m <= HORIZON_REACH_M) {
        return vec4<f32>(0.0, 1.0, 0.0, 0.0);
    }
    let far = horizon_walk(
        horizon.to_far.xyz, horizon.far.yzw, horizon.far.x, 1u, local, d, fine.z,
        min(reach_m, HORIZON_FAR_REACH_M),
    );
    // **`z` is the share the coarse band stopped, not a yes**: the two bands compose by
    // transmittance, so the handover is `(1 − fine) · (1 − coarse)` and an eight-metre cell that is
    // a third full dims by a third instead of blocking the ray.
    return vec4<f32>(0.0, 1.0, far.y, far.w);
}

/// **How much of one direction reaches an eye-relative point**: `1` clear, `0` stopped.
///
/// The same answer `voxel_stream::light` gives for a solid material, at the octree's own subvoxel
/// grain and then at the coarse band's. A direction that could not be measured — a degenerate one,
/// or one that leaves a band nothing is stamped in — is `1.0`, the answer the dome had before the
/// volume existed, because an unmeasured ray is not a shadow.
fn horizon_visibility(local: vec3<f32>, dir: vec3<f32>, reach_m: f32) -> f32 {
    let length_dir = length(dir);
    if (length_dir <= 0.0) {
        return 1.0;
    }
    let marched = horizon_transmittance(local, dir / length_dir, reach_m);
    // **Both bands count here, and only here.** This is the dome's question — how much of the sky
    // a cell can see — and a diffuse answer does not need half-metre boundaries: an eight-metre
    // cell that holds matter is a wall a corridor's ambient must stop at, however coarse it is.
    // The two are composed as transmittances, which is what makes the coarse share a blend.
    return select(1.0, 0.0, marched.x > 0.5) * (1.0 - marched.z);
}

/// **The dome's own share at a point**, and whether the stencil was measured: `vec2(view, measured)`.
///
/// **Five directions, and the count is the CPU sampler's own** (`voxel_stream::light::SKY_STENCIL`):
/// up and four at 45°, the same rays at every fragment, so two cells' skies are comparable and a
/// cave mouth reads darker than a ridge. Five is not a hemisphere and this is not an integral; it
/// is a declared stencil, and the bounce below is what carries light past it.
///
/// **The rays end with the fine band, and the coarse band is not asked.** An 8 m cell answering a
/// *dome* is a view fraction quantised to eight metres: it puts a block of ambient light on the
/// ground beside every cliff, which is a false edge on a surface whose own detail is a 1/32 of a
/// block. The coarse band's share is still what a long beam is stopped by, and the beam is the
/// caller that may read it; the dome is not. A corridor past the fine box therefore reads open —
/// the answer this had before the coarse band existed, and the one a quantised dome was hiding.
///
/// **`measured` is a majority, not a unanimity**: a stencil half in the next chunk slot along is
/// still an answer about this cell, and refusing it would put a chunk-corners pattern into every
/// cave's ambient.
fn horizon_dome(local: vec3<f32>) -> vec2<f32> {
    let rays = array<vec3<f32>, 5>(
        vec3<f32>(0.0, 1.0, 0.0),
        vec3<f32>(0.70710677, 0.70710677, 0.0),
        vec3<f32>(0.0, 0.70710677, 0.70710677),
        vec3<f32>(-0.70710677, 0.70710677, 0.0),
        vec3<f32>(0.0, 0.70710677, -0.70710677),
    );
    var total = 0.0;
    var measured = 0.0;
    for (var ray = 0; ray < 5; ray = ray + 1) {
        let answer = horizon_transmittance(local, rays[ray], HORIZON_REACH_M);
        total = total + select(1.0, 0.0, answer.x > 0.5) * (1.0 - answer.z);
        measured = measured + select(0.0, 1.0, answer.y + answer.w > 0.5);
    }
    return vec2<f32>(total / 5.0, select(0.0, 1.0, measured > 2.5));
}

