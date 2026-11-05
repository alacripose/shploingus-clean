// **The streamed light horizon**: the two bands the fragment marches, their address laws, and the
// dome. `voxel_client::horizon` owns the state these read — this file owns only the reading of it.
//
// Composed by `world::pass_source` between `atmosphere.wgsl` and the pass that draws with it, so a
// binding number here is that file's neighbour rather than a fact of this one.

// **The rung table's own width**, `voxel_client::lod::SURFACE_RUNG_SLOTS`, held to this literal by
// `horizon::tests::the_shaders_reach_and_the_hosts_are_one_number`: a shader that believed a
// different number than the host wrote would read a neighbour's entry as its own rung.
const SURFACE_RUNG_SLOTS: u32 = 24u;

/// **One surface rung's entry**: where its band's grid starts in this frame's eye-relative frame,
/// and `(cell_m, cells, reach_m, word offset)` — the same four numbers the coarse rungs carry,
/// which is what lets one walk answer for every rung on the ladder.
struct SurfaceRung {
    to_anchor: vec4<f32>,
    params: vec4<f32>,
};

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
    // Tier 6's band: sixteen-metre cells over a kilometre, the grain of the rung past the far ring.
    to_wide: vec4<f32>,
    wide: vec4<f32>,
    // Tier 7's band: one 32 m cell a chunk over two kilometres, the last chunk-held rung.
    to_chunk: vec4<f32>,
    chunk: vec4<f32>,
    // **Where each coarse rung starts in the one band buffer**, in words: far, wide, chunk, and
    // **how far the surface rung table reaches**. Adding a rung appends words and an entry here
    // rather than spending a binding.
    offsets: vec4<f32>,
    // **Which chunk slots the fine band has looked at**: bit `sx + 3·(sy + 3·sz)` of `x`, one per
    // 32 m slot of the 3×3×3 cube. A cell in an unset slot is *unmeasured*, not air — zero means
    // "nothing stands here" and that is a claim only a stamped chunk may make. The coarse band
    // carries its own "stamped" bit per cell instead, because one word there is already spent.
    coverage: vec4<u32>,
    // **The surface rungs' table**, one entry a tier above the chunk-held ones, at the end because
    // a uniform's array is its last member. A rung that has not been measured holds `cell_m` zero,
    // and `offsets.w` says how far the table reaches, so a lookup walks the rungs that exist.
    surfaces: array<SurfaceRung, SURFACE_RUNG_SLOTS>,
};

@group(0) @binding(10) var<storage, read> horizon_cells: array<u32>;
@group(0) @binding(11) var<uniform> horizon: HorizonView;
@group(0) @binding(12) var<storage, read> horizon_coarse: array<u32>;

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
// **The coarse band**: 8 m cells over the whole 512 m box it holds. Reaching the band's own edge is
// the point — the march stops when it leaves the box anyway, and a reach shorter than the data was
// an eighth-metre rung that answered only a third of the ground that exists.
const HORIZON_FAR_REACH_M: f32 = 512.0;
// **The wide band**: sixteen-metre cells, 64 of them a side — the tier-6 rung's own grain and the
// range a rung past the far ring is lit over. Held to `voxel_client::horizon`'s literals by
// `horizon::tests::the_shaders_reach_and_the_hosts_are_one_number` like the two below it.
const HORIZON_WIDE_CELL_M: f32 = 16.0;
const HORIZON_WIDE_REACH_M: f32 = 1024.0;
// **The chunk tier**: 32 m cells, 64 of them a side — the last chunk-held rung.
const HORIZON_CHUNK_CELL_M: f32 = 32.0;
const HORIZON_CHUNK_REACH_M: f32 = 2048.0;
// The coarse word's share and stamped bit, `voxel_client::horizon`'s `FAR_SHARE`/`FAR_MEASURED`.
const FAR_SHARE: u32 = 0xFFu;
const FAR_MEASURED: u32 = 256u;
/// **The share at which an eight-metre cell is in a ray's way**: three quarters of the cell holding
/// matter is a canopy, a wall, or a hillside's own bulk; below that the cell is mostly air the ray
/// crosses, and a surface passing through it is not a wall. See `world_far_share`.
const FAR_BLOCKS: f32 = 0.75;
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

//// **One surface rung's band word at a point of its own grid.** The rung's words live in the same
/// buffer as the three chunk-held bands, at the offset its entry states.
fn horizon_surface_word(rung: i32, i: vec3<i32>) -> u32 {
    let entry = horizon.surfaces[rung];
    let n = i32(entry.params.y);
    return horizon_coarse[u32(entry.params.w) + u32(i.x + n * (i.y + n * i.z))];
}

// One coarse band cell's word: bit 0 matter, bit 1 stamped.
fn horizon_far_word(i: vec3<i32>) -> u32 {
    let n = i32(horizon.far.y);
    return horizon_coarse[u32(horizon.offsets.x) + u32(i.x + n * (i.y + n * i.z))];
}

/// One wide cell's word, the same two fields one rung up.
fn horizon_wide_word(i: vec3<i32>) -> u32 {
    let n = i32(horizon.wide.y);
    return horizon_coarse[u32(horizon.offsets.y) + u32(i.x + n * (i.y + n * i.z))];
}

/// One cell of the 32 m (one-a-chunk) band.
fn horizon_chunk_word(i: vec3<i32>) -> u32 {
    let n = i32(horizon.chunk.y);
    return horizon_coarse[u32(horizon.offsets.z) + u32(i.x + n * (i.y + n * i.z))];
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
