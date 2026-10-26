---
patch_id: PATCH-069
title: The shadow that leaked through the dug quarter — a mixed voxel is charged cell by cell, not once at its entry
status: applied
operation: fix
target: shploingus/shadow-sampling
base_ref: shploingus-clean@main
base_digest: sha256:cdeb73282b129f8a7b87123918711cd0ec45ef775357880db5f83427c4c3f7dc
dependencies: [PATCH-066]
source: user-directed-conversation
created_at: 2026-10-23
approval: "user-directed 2026-10-23 — the shadow sampling is not at the subvoxel scale the material rendering has"
live_mutation: false
---

# Change

**A `Mixed` leaf is sixty-four subvoxels, and the tree walk asked it once.** `Spans` reports a
`Mixed` voxel as **one** span — `entry_t` to `exit_t` over the whole metre — and `Span::at` is the
subvoxel the ray *entered* through. `march_tree` charged `material_at_entry()` for that whole metre.
So a beam that entered a half-dug voxel through its empty corner was charged nothing for the rock it
then crossed: **the light leaked through the dug quarter of every half-cut voxel**, and a beam that
entered through the solid corner charged the whole voxel as rock. Both directions of the error are
the same mistake — one address speaking for sixty-four.

**The stretch of a `Mixed` leaf is now walked cell by cell** (`charge_shape`), at `SUBVOXEL_SIDE`
times the coordinates, which leaves `t` in world metres: every crossed cell is charged over its own
segment with the same `Blocking` law the span walk uses. The per-voxel walk
(`transmittance`) has always asked the subvoxel address; this is the tree's walk doing the same.

**What this does not resolve, said here rather than implied:** the world's shapes store `4×4×4`
cells (`voxel-model::subvoxel`, `SUBVOXEL_SIDE = 4`), so this is a **quarter-voxel** march. The `1/32`
the report names is `FILL_SIDE = 32` — the **fluid fill lattice** — and the material *pattern*'s
`DETAIL_SIDE = 16` per subvoxel. Terrain occupancy has no `1/32` cells to ask for; a shadow at that
scale needs the shape to carry `32³` cells (32 768 per voxel), which is a data-model change and not
a walk change.

# Acceptance

- `voxel_stream::light::tests::a_mixed_voxels_shadow_follows_its_own_subvoxel_grid` — a tree with one
  solid subvoxel at the **top** of a voxel, so a beam straight up enters through an empty cell and
  meets the rock 0.75 m later: it is **stopped** (`0.0`), and the beam through the emptied quarter
  of the same voxel reads **`1.0`**. The first assertion fails on the old walk, which answered `1.0`
  for the whole voxel because its entry cell was empty.
- `a_partial_voxel_shadows_only_where_it_is_solid` (the per-voxel walk) passes unchanged.
- `cargo test -p voxel-stream --lib`, `cargo test -p voxel-client --lib`, `cargo check --workspace`.

# Evidence

- `cargo test -p voxel-stream --lib a_mixed_voxels_shadow -- --nocapture` — **1 passed**. The fixture
  is one solid subvoxel at the **top** of a voxel (`SubvoxelIndex::new(0, 3, 0)`), so a beam straight
  up from `y = 3.0` enters the voxel through an empty cell and meets the rock 0.75 m in: it is
  **stopped** (`0.0`), and the beam through the emptied quarter of the same voxel reads **`1.0`**.
  The first number is the regression this record repairs; on the old walk the same beam answered
  `1.0`, because `material_at_entry` spoke for the voxel the beam entered and not for the cells it
  crossed.
- `cargo test -p voxel-stream --lib a_partial_voxel_shadow -- --nocapture` — **1 passed**
  (`a_partial_voxel_shadows_only_where_it_is_solid`, the per-voxel walk).
- `cargo test -p voxel-stream --lib` — **50 passed, 0 failed, 3 ignored**.
- `cargo test -p voxel-client --lib` — **543 passed, 0 failed, 2 ignored**.
- `cargo check --workspace` — **Finished**, no errors.

# What this does not touch

The exposure (PATCH-068), the tree mob, the gate, the walk, the cell-shell refusal and the moon's
orbit: untouched.

# Rollback

Delete the `Node::Mixed` arm in `march_tree` and `charge_shape`/`charge_cell`, so a mixed leaf is
charged from `material_at_entry()` again. The leak returns.

## Recipes

```recipe
segment: game
axis: evidence_grade
ticket: T-0960
site: crates/voxel-stream/src/light.rs:462
what: charge a mixed voxel's own 4x4x4 grid along the ray instead of one subvoxel at the span's entry, so a beam cannot pass through the empty quarter of a half-dug voxel
because: Spans reports a Mixed leaf as one metre-wide span whose `at` is the entry cell, so material_at_entry spoke for all sixty-four and the light leaked through the dug corner
check: cargo test -p voxel-stream --lib a_mixed_voxels_shadow -> failures 0
witness: crates/voxel-stream/src/light.rs
expects: 0
requires: cargo test -p voxel-stream --lib
requires: cargo test -p voxel-client --lib
requires: cargo check --workspace
```
