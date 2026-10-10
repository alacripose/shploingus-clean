# Task 1: Block-level raster

## Context

This is Task 1 of 8 in the hyper-greedy block-level remeshing plan. The goal is to replace the 128³ subvoxel greedy mesher with a 32³ block-level hyper-greedy mesher. This task changes the raster constants and helper functions from subvoxel-level to block-level.

## Requirements

Read the plan file for the full specification: `docs/superpowers/plans/2026-10-07-hyper-greedy-block-level-remeshing.md`

### Step 1: Change constants

In `crates/voxel-mesh/src/mesh.rs`, replace:
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

### Step 2: Update `index_of`

The function body stays the same — it already uses `PAD` and `FIELD_SIDE`.

### Step 3: Update `field_strides`

The function body stays the same — it already uses `FIELD_SIDE`.

### Step 4: Update `cell`

The function body stays the same — it already uses `axes`.

### Step 5: Update `leaf_region`

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

### Step 6: Update `fill_cube`

The function body stays the same — it already uses `index_of`.

### Step 7: Update `Mesher::new`

The function body stays the same — the constants changed.

### Step 8: Update `Mesher::scratch_bytes`

The function body stays the same.

### Step 9: Run tests

Run: `cargo test -p voxel-mesh`
Expected: All pass (the constants changed but the logic is the same)

## Global Constraints

- Determinism: two runs over the same seed produce byte-identical meshes
- Refuse, never default: unanswerable voxels are errors, not air
- No unsafe code
- The mesher must not allocate per-chunk (scratch is held)
- Worker count is a performance knob, never a change in output (F12)

## Report

Write your report to: `.superpowers/sdd/hyper-greedy-block-level-remeshing/task-1-report.md`

The report should include:
- What you changed
- Test results (command run + output)
- Any concerns or observations
