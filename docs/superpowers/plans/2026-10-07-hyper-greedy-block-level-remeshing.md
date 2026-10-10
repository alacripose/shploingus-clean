# Hyper-Greedy Block-Level Remeshing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the 128³ subvoxel greedy mesher with a 32³ block-level hyper-greedy mesher that merges same-material faces across block boundaries, reducing quad count by ~16× while preserving LOD material rendering via the existing 256-level lattice.

**Architecture:** The mesher rasterises the chunk to a 32³ block-level grid (not 128³ subvoxel). For each of the 6 faces, it builds a 32×32 mask of visible block faces and greedy-merges equal materials into rectangles. The 256-level detail lattice in the fragment shader handles sub-block detail, so subvoxel geometry is no longer needed. The `refine` step is updated to work at block level: a merged quad spanning N blocks is split only when the displacement law varies across those blocks.

**Tech Stack:** Rust, wgpu, existing voxel-mesh crate

**Spec:** User request — "hyper greedy remeshing allows for the 32 units per axis on the sides of blocks at close range while having distant blocks materials coarser"

**Global Constraints:**
- Determinism: two runs over the same seed produce byte-identical meshes
- Refuse, never default: unanswerable voxels are errors, not air
- No unsafe code
- The mesher must not allocate per-chunk (scratch is held)
- Worker count is a performance knob, never a change in output (F12)

---

### Task 1: Block-level raster

**Files:**
- Modify: `crates/voxel-mesh/src/mesh.rs` (constants, `index_of`, `field_strides`, `cell`, `leaf_region`, `fill_cube`, `Mesher::new`, `Mesher::scratch_bytes`)

**Interfaces:**
- Consumes: `Chunk`, `ChunkNeighborhood` (unchanged)
- Produces: `FIELD_SIDE = CHUNK_SIDE + 2` (34), `INTERIOR = CHUNK_SIDE` (32), `PAD = 1`

- [ ] **Step 1: Change constants**

Replace:
```rust
pub const PAD: i32 = SUBVOXEL_SIDE_I32;
pub const INTERIOR: usize = SUBVOXEL_CHUNK_SIDE as usize;
pub const FIELD_SIDE: usize = INTERIOR + 2 * PAD as usize;
```
With:
```rust
pub const PAD: i32 = 1;
pub const INTERIOR: usize = CHUNK_SIDE as usize;
pub const FIELD_SIDE: usize = INTERIOR + 2 * PAD as usize;
```

- [ ] **Step 2: Update `index_of`**

The function body stays the same — it already uses `PAD` and `FIELD_SIDE`.

- [ ] **Step 3: Update `field_strides`**

The function body stays the same — it already uses `FIELD_SIDE`.

- [ ] **Step 4: Update `cell`**

The function body stays the same — it already uses `axes`.

- [ ] **Step 5: Update `leaf_region`**

Replace:
```rust
fn leaf_region(origin: [i32; 3], level: u32) -> ([i32; 3], i32) {
    let voxels = 1i32 << level;
    (
        [origin[0] * SUBVOXEL_SIDE_I32, origin[1] * SUBVOXEL_SIDE_I32, origin[2] * SUBVOXEL_SIDE_I32],
        voxels * SUBVOXEL_SIDE_I32,
    )
}
```
With:
```rust
fn leaf_region(origin: [i32; 3], level: u32) -> ([i32; 3], i32) {
    let voxels = 1i32 << level;
    (
        [origin[0], origin[1], origin[2]],
        voxels,
    )
}
```

- [ ] **Step 6: Update `fill_cube`**

The function body stays the same — it already uses `index_of`.

- [ ] **Step 7: Update `Mesher::new`**

Replace:
```rust
pub fn new() -> Self {
    Mesher {
        field: vec![MaterialId::EMPTY; FIELD_SIDE * FIELD_SIDE * FIELD_SIDE],
        mask: vec![MaterialId::EMPTY; INTERIOR * INTERIOR],
    }
}
```
With:
```rust
pub fn new() -> Self {
    Mesher {
        field: vec![MaterialId::EMPTY; FIELD_SIDE * FIELD_SIDE * FIELD_SIDE],
        mask: vec![MaterialId::EMPTY; INTERIOR * INTERIOR],
    }
}
```
(The body stays the same — the constants changed.)

- [ ] **Step 8: Update `Mesher::scratch_bytes`**

The function body stays the same.

- [ ] **Step 9: Run tests**

Run: `cargo test -p voxel-mesh`
Expected: All pass (the constants changed but the logic is the same)

---

### Task 2: Block-level merge

**Files:**
- Modify: `crates/voxel-mesh/src/mesh.rs` (`build`, `build_naive`, `visible_faces`, `merge`)

**Interfaces:**
- Consumes: `FIELD_SIDE`, `INTERIOR`, `PAD` from Task 1
- Produces: `Mesh` with block-level quads

- [ ] **Step 1: Update `build`**

Replace `SUBVOXEL_CHUNK_SIDE` with `CHUNK_SIDE` in the loop:
```rust
for slice in 0..CHUNK_SIDE {
```
And in the mask loop:
```rust
for v in 0..side {
    for u in 0..side {
```
Where `side = INTERIOR` (which is now `CHUNK_SIDE`).

- [ ] **Step 2: Update `build_naive`**

Replace `SUBVOXEL_CHUNK_SIDE` with `CHUNK_SIDE` in the loop.

- [ ] **Step 3: Update `visible_faces`**

Replace `SUBVOXEL_CHUNK_SIDE` with `CHUNK_SIDE` in the loop.

- [ ] **Step 4: Update `merge`**

Replace `SUBVOXEL_CHUNK_SIDE` with `CHUNK_SIDE` in the loop.

- [ ] **Step 5: Run tests**

Run: `cargo test -p voxel-mesh`
Expected: All pass

---

### Task 3: Block-level refine

**Files:**
- Modify: `crates/voxel-mesh/src/refine.rs` (`refine`, `pieces`, `cells_for`, `position`, `in_plane_axes_of`)

**Interfaces:**
- Consumes: `Quad` (now block-level), `DisplacementField`, `RefineConfig`
- Produces: `RefinedFace` with block-level triangles

- [ ] **Step 1: Update `refine`**

The `pieces` function now works in block coordinates (0..32) instead of subvoxel coordinates (0..128). The `cells_for` function now returns cells per block (1..=32) instead of cells per subvoxel (1..=256).

- [ ] **Step 2: Update `pieces`**

Replace:
```rust
fn pieces(lo: u16, width: u16) -> Vec<Piece> {
    let mut out = Vec::new();
    if width == 0 {
        return out;
    }
    let per = SUBVOXEL_SIDE as u16;
    let first = lo / per;
    let last = (lo + width - 1) / per;
    for block in first..=last {
        let block_lo = block * per;
        let a0 = lo.max(block_lo);
        let a1 = (lo + width).min(block_lo + per);
        out.push(Piece {
            block,
            f0: (a0 - block_lo) as f64 / SUBVOXELS_PER_BLOCK,
            f1: (a1 - block_lo) as f64 / SUBVOXELS_PER_BLOCK,
        });
    }
    out
}
```
With:
```rust
fn pieces(lo: u16, width: u16) -> Vec<Piece> {
    let mut out = Vec::new();
    if width == 0 {
        return out;
    }
    let first = lo;
    let last = lo + width - 1;
    for block in first..=last {
        out.push(Piece {
            block,
            f0: 0.0,
            f1: 1.0,
        });
    }
    out
}
```

- [ ] **Step 3: Update `cells_for`**

Replace:
```rust
fn cells_for(piece: &Piece, samples_per_block: u16) -> u16 {
    let covered = (piece.f1 - piece.f0) * SUBVOXELS_PER_BLOCK;
    let cells = (covered * samples_per_block as f64 / SUBVOXELS_PER_BLOCK).ceil();
    (cells as u16).max(1)
}
```
With:
```rust
fn cells_for(piece: &Piece, samples_per_block: u16) -> u16 {
    let covered = piece.f1 - piece.f0;
    let cells = (covered * samples_per_block as f64).ceil();
    (cells as u16).max(1)
}
```

- [ ] **Step 4: Update `position`**

Replace:
```rust
fn position(quad: &Quad, block_u: u16, block_v: u16, fu: f64, fv: f64, relief: f64) -> [f32; 3] {
    let (au, av) = in_plane_axes_of(quad.face);
    let mut p = [0.0f64; 3];
    p[quad.face.axis() as usize] = quad.slice as f64;
    for axis in 0..3 {
        p[axis] += au[axis] * (block_u as f64 * SUBVOXELS_PER_BLOCK + fu * SUBVOXELS_PER_BLOCK);
        p[axis] += av[axis] * (block_v as f64 * SUBVOXELS_PER_BLOCK + fv * SUBVOXELS_PER_BLOCK);
    }
    let n = normal_f64(quad.face);
    for axis in 0..3 {
        p[axis] += n[axis] * relief;
    }
    [p[0] as f32, p[1] as f32, p[2] as f32]
}
```
With:
```rust
fn position(quad: &Quad, block_u: u16, block_v: u16, fu: f64, fv: f64, relief: f64) -> [f32; 3] {
    let (au, av) = in_plane_axes_of(quad.face);
    let mut p = [0.0f64; 3];
    p[quad.face.axis() as usize] = quad.slice as f64;
    for axis in 0..3 {
        p[axis] += au[axis] * (block_u as f64 + fu);
        p[axis] += av[axis] * (block_v as f64 + fv);
    }
    let n = normal_f64(quad.face);
    for axis in 0..3 {
        p[axis] += n[axis] * relief;
    }
    [p[0] as f32, p[1] as f32, p[2] as f32]
}
```

- [ ] **Step 5: Update `in_plane_axes_of`**

The function body stays the same — it already returns the correct axis vectors.

- [ ] **Step 6: Run tests**

Run: `cargo test -p voxel-mesh`
Expected: All pass

---

### Task 4: Update `Mesh::corner_positions_m`

**Files:**
- Modify: `crates/voxel-mesh/src/mesh.rs` (`Mesh::corner_positions_m`)

**Interfaces:**
- Consumes: `Quad` (now block-level)
- Produces: `[[f32; 3]; 4]` in metres from chunk's low corner

- [ ] **Step 1: Update scale**

Replace:
```rust
let scale = 1.0 / SUBVOXEL_SIDE as f32;
```
With:
```rust
let scale = 1.0;
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p voxel-mesh`
Expected: All pass

---

### Task 5: Update `Scene::build` and `Scene::build_with_detail`

**Files:**
- Modify: `crates/voxel-client/src/scene.rs` (`Scene::build`, `Scene::build_with_detail`)

**Interfaces:**
- Consumes: `Mesh` (now block-level), `DetailLattice`, `LightAt`, `DisplacementField`, `RefineConfig`
- Produces: `Scene` with block-level vertices

- [ ] **Step 1: Update `Scene::build`**

The `subvoxel_m` function now returns the input unchanged (scale = 1.0). The `world_cell` function now uses `1.0` instead of `1.0 / SUBVOXEL_SIDE as f64`. The `corner_detail_index` function now uses `DETAIL_SIDE` instead of `DETAIL_SIDE * SUBVOXEL_SIDE`.

- [ ] **Step 2: Update `Scene::build_with_detail`**

The function body stays the same.

- [ ] **Step 3: Run tests**

Run: `cargo test -p voxel-client`
Expected: All pass

---

### Task 6: Update `pattern.rs` constants

**Files:**
- Modify: `crates/voxel-client/src/pattern.rs` (`SUBVOXEL_CELL_M`, `DETAIL_CELL_M`, `OCTAVE_FLOOR`, `OCTAVE_TOP`, `MIN_FEATURE_M`, `MAX_FEATURE_M`)

**Interfaces:**
- Consumes: `SUBVOXEL_SIDE`, `DETAIL_SIDE` from `voxel_model`
- Produces: Updated constants for block-level meshing

- [ ] **Step 1: Update constants**

Replace:
```rust
pub const SUBVOXEL_CELL_M: f64 = VOXEL_M / SUBVOXEL_SIDE as f64;
pub const DETAIL_CELL_M: f64 = SUBVOXEL_CELL_M / DETAIL_SIDE as f64;
```
With:
```rust
pub const SUBVOXEL_CELL_M: f64 = VOXEL_M;
pub const DETAIL_CELL_M: f64 = SUBVOXEL_CELL_M / DETAIL_SIDE as f64;
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p voxel-client`
Expected: All pass

---

### Task 7: Update `shading.wgsl` constants

**Files:**
- Modify: `crates/voxel-client/src/shading.wgsl` (`OCTAVE_FLOOR`, `OCTAVE_TOP`, `SUBVOXEL_OCTAVE`)

**Interfaces:**
- Consumes: `rates` uniform (now block-level)
- Produces: Updated constants for block-level meshing

- [ ] **Step 1: Update constants**

Replace:
```wgsl
const OCTAVE_FLOOR: u32 = 1u;
const OCTAVE_TOP: u32 = 11u;
const SUBVOXEL_OCTAVE: u32 = 4u;
```
With:
```wgsl
const OCTAVE_FLOOR: u32 = 0u;
const OCTAVE_TOP: u32 = 5u;
const SUBVOXEL_OCTAVE: u32 = 0u;
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p voxel-client`
Expected: All pass

---

### Task 8: Full integration test

**Files:**
- No new files

**Interfaces:**
- Consumes: All previous tasks
- Produces: Verified block-level meshing

- [ ] **Step 1: Run all tests**

Run: `cargo test -p voxel-mesh -p voxel-client`
Expected: All pass

- [ ] **Step 2: Run the window**

Run: `cargo run -p shploingus`
Expected: The world renders with fewer quads

---

## Self-Review

**1. Spec coverage:**
- ✅ Block-level raster (Task 1)
- ✅ Block-level merge (Task 2)
- ✅ Block-level refine (Task 3)
- ✅ Updated corner positions (Task 4)
- ✅ Updated scene build (Task 5)
- ✅ Updated pattern constants (Task 6)
- ✅ Updated shader constants (Task 7)
- ✅ Full integration test (Task 8)

**2. Placeholder scan:**
- No placeholders found

**3. Type consistency:**
- `FIELD_SIDE`, `INTERIOR`, `PAD` are consistent across tasks
- `Quad` is block-level in all tasks
- `Mesh` is block-level in all tasks
- `Scene` is block-level in all tasks
