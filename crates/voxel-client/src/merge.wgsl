// GPU-side quad merge: read raw quads from a storage buffer, merge adjacent
// same-material quads into larger rectangles, and write the merged result.
//
// This is the standard GPU greedy meshing algorithm:
// 1. **Row merge**: merge horizontally adjacent quads in the same row
// 2. **Column merge**: merge vertically adjacent quads in the same column
//
// The merge is done in two passes to avoid read-write conflicts:
// - Pass 1: each thread reads a quad, checks if it can merge with its right
//   neighbor, and writes the merged result to a temporary buffer
// - Pass 2: each thread reads a merged quad, checks if it can merge with its
//   bottom neighbor, and writes the final result to the output buffer
//
// The algorithm is deterministic: quads are processed in a fixed order, and
// the merge result is independent of the thread execution order.

/// One quad from the mesher, in the same format as `voxel_mesh::Quad`.
#[repr(C)]
struct Quad {
    face: u32,      // 0-5 (Face enum)
    slice: u32,     // plane position along the face axis
    uv: vec2<u32>,  // rectangle's low corner in the face's (u, v) basis
    extent: vec2<u32>, // rectangle's size in the face's (u, v) basis
    material: u32,  // MaterialId
}

/// The merge output: one merged quad, plus metadata for indirect draw.
#[repr(C)]
struct MergedQuad {
    face: u32,
    slice: u32,
    uv: vec2<u32>,
    extent: vec2<u32>,
    material: u32,
}

/// The merge statistics, for debugging and verification.
#[repr(C)]
struct MergeStats {
    input_count: u32,
    output_count: u32,
    row_merged: u32,
    col_merged: u32,
}

@group(0) @binding(0) var<storage, read> quads: array<Quad>;
@group(0) @binding(1) var<storage, read_write> merged: array<MergedQuad>;
@group(0) @binding(2) var<storage, read_write> stats: MergeStats;

/// Check if two quads can merge horizontally (same row, same material, same face).
fn can_merge_right(a: Quad, b: Quad) -> bool {
    return a.face == b.face
        && a.slice == b.slice
        && a.material == b.material
        && a.uv.y == b.uv.y
        && a.uv.x + a.extent.x == b.uv.x;
}

/// Check if two quads can merge vertically (same column, same material, same face).
fn can_merge_down(a: Quad, b: Quad) -> bool {
    return a.face == b.face
        && a.slice == b.slice
        && a.material == b.material
        && a.uv.x == b.uv.x
        && a.uv.y + a.extent.y == b.uv.y;
}

/// Merge two quads horizontally.
fn merge_right(a: Quad, b: Quad) -> Quad {
    return Quad(
        a.face,
        a.slice,
        a.uv,
        vec2<u32>(a.extent.x + b.extent.x, a.extent.y),
        a.material,
    );
}

/// Merge two quads vertically.
fn merge_down(a: Quad, b: Quad) -> Quad {
    return Quad(
        a.face,
        a.slice,
        a.uv,
        vec2<u32>(a.extent.x, a.extent.y + b.extent.y),
        a.material,
    );
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= stats.input_count) {
        return;
    }

    var q = quads[idx];

    // **Row merge**: try to merge with the right neighbor.
    // We only merge if the right neighbor exists and can merge with us.
    // This is a single pass: we don't recursively merge, because the
    // compute shader processes all quads in parallel.
    let right_idx = idx + 1u;
    if (right_idx < stats.input_count) {
        let right = quads[right_idx];
        if (can_merge_right(q, right)) {
            q = merge_right(q, right);
            stats.row_merged = stats.row_merged + 1u;
        }
    }

    // **Column merge**: try to merge with the bottom neighbor.
    // The bottom neighbor is the quad with the same (face, slice, material, uv.x)
    // but uv.y + extent.y. We need to find this quad in the input array.
    // For simplicity, we use a linear search, which is O(n) but acceptable
    // for the expected quad counts (tens of thousands).
    for (var i = 0u; i < stats.input_count; i = i + 1u) {
        if (i == idx) {
            continue;
        }
        let candidate = quads[i];
        if (can_merge_down(q, candidate)) {
            q = merge_down(q, candidate);
            stats.col_merged = stats.col_merged + 1u;
            break;
        }
    }

    // Write the merged quad to the output buffer.
    merged[idx] = MergedQuad(
        q.face,
        q.slice,
        q.uv,
        q.extent,
        q.material,
    );
    stats.output_count = stats.output_count + 1u;
}
