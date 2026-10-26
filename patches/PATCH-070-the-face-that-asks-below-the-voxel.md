---
patch_id: PATCH-070
title: The face that asks below the voxel — the frame samples the shadow at the stored subvoxel lattice, and `Spans` owns what one ray crosses
status: applied
operation: fix
target: shploingus/shadow-sampling
base_ref: shploingus-clean@main
base_digest: sha256:cdeb73282b129f8a7b87123918711cd0ec45ef775357880db5f83427c4c3f7dc
dependencies: [PATCH-069]
source: user-directed-conversation
created_at: 2026-10-23
approval: "user-directed 2026-10-23 — the frame must ask for shadow at subvoxel resolution rather than once per quad corner"
live_mutation: false
---

# Change

**A face carried four samples whichever size it was.** `SceneBuild::emit` measured each quad at
its four corners and `shading.wgsl` interpolated `light_direct` across the face, so a merged
rectangle's shadow was a blend between corners a metre or more apart. The field behind it already
answers at `SUBVOXEL_SIDE = 4` per voxel (0.25 m); the *frame* never asked at that resolution, so
no walk, however fine, could put a sub-metre edge in the picture.

**A face is now cut at the light's own lattice.** `Scene::build_with_light_lattice` (and
`SceneBuild::with_shadow_lattice`) take cells per voxel edge: `1` is the once-per-corner path,
byte for byte; `4` (`SHADOW_LATTICE`) is one sample per stored subvoxel. The frame's own
`ResidentScene` carries the lattice and emits every chunk — landed or re-emitted — at it, and
`main.rs` asks for `SHADOW_LATTICE`. The grid's vertices are measured once each (25 asks for a
square metre instead of 4), and `light_present` stays flat per emitted cell.

**What the stored data supports, stated rather than implied:** the finest answer is **0.25 m** —
one `CellField` cell per subvoxel. A finer lattice would ask the same cell twice and could change
only the interpolation between answers, so `with_shadow_lattice` clamps to a divisor of
`SUBVOXEL_SIDE` (`3` is answered by `2`, anything above `4` by `4`). The material **pattern**
lattice (`DETAIL_SIDE = 16` per subvoxel) is finer, but it is detail of colour, not of occupancy or
of the shadow, and sampling it for light would be inventing answers the field does not hold.

**The ownership of "what one ray crosses" is one function.** `Spans` already splits a non-uniform
`Mixed` leaf at its own 4×4×4 grid (`Spans::split`), and `march_tree` in `voxel-stream` spends
`material_at_entry()` per span with no second walk of the leaf. PATCH-069's `charge_shape` is not
in this tree: the split lives in the walk that knows the geometry, and the consumer only asks it.
The duplicate blocking walk that skipped `Matter::medium` is gone with it — `medium` is asked by
`charge` on the stepped path, where the provider has an address to answer it at.

# Evidence — pixels, then numbers

`cargo run --release -p voxel-client --example shadow-lattice` renders one square metre, half in
shadow at 0.5 m, through `WorldPipeline` + `SkyModel`, with the shadow's footprint isolated by
differencing a fully lit render of the same face. Measured on this machine (RTX 2070 SUPER,
Vulkan, 512×288, one sample):

- coarse (`build_with_light`): the shadow's own footprint fades 90 % to 10 % over **136.0 px =
  0.663 m** of the face, which is 205 px across.
- fine (`SHADOW_LATTICE`): the same footprint fades over **41.0 px = 0.200 m** — a quarter-metre
  cell's own shoulder, below the voxel.
- the two frames differ in **41 646 of 147 456 px (28.2 %)**; the pictures are
  `docs/shadow-lattice/shadow-coarse.png` and `docs/shadow-lattice/shadow-fine.png`.
- cost: **6.2× the vertices** (25 shared corners against 4 for a square metre, **1 700 B against
  272 B**), **3.6× the asks** (29 against 8 — the quad's four corners once to decide its flag, then
  the grid's 25 vertices once each; the once-per-corner path asks its own 2 × 2 too), 0.2–0.5 µs an
  ask in a debug build (the ask is a hash lookup in the field, and the field's marches are on its own
  thread in the game).

The example's own assertions hold the claim: the fine shoulder is less than half the coarse one
and below 0.5 m, or it exits non-zero.

# Acceptance

- `scene::tests::a_face_asks_the_shadow_at_the_subvoxel_lattice_not_once_per_corner` — a one-voxel
  face at `SHADOW_LATTICE` is a **5 × 5 grid of 25 shared corners** and **96 indices** (six a cell,
  sixteen cells), not four corners a cell: an interior sample is one vertex for the four cells that
  meet there. Its x samples are exactly `[0, 0.25, 0.5, 0.75, 1.0]`, and the 0.5 m edge falls
  between two samples that disagree. The same face on `build_with_light` is still 4 corners at
  `[0, 1]`, which is the edge it could not draw.
- `scene::tests::the_shadow_lattice_is_clamped_to_a_divisor_of_the_subvoxel_side`.
- `voxel-mesh::raycast::tests::a_leaf_with_a_hole_is_reported_cell_by_cell` and
  `voxel-stream::light::tests::a_mixed_voxels_shadow_follows_its_own_subvoxel_grid` pass untouched.
- Every existing assertion is where it was: the once-per-corner constructor is still four corners
  and the resident-scene test still counts `quads × 4`.

# What this does not touch

The gate, the light walk, the twilight exposure (`EXPOSURE_GRADE`, `MOONLIGHT_GAIN`) and the
material detail lattice: untouched. The refracted (bent) path keeps its own per-vertex sampling.

# Rollback

Delete the `Some(_)` lattice branch in `SceneBuild::emit` and restore the four-corner emission;
`ResidentScene::set_shadow_lattice` and `build_with_light_lattice` become unused and go with it.
The 1 m blend returns.

## Recipes

```recipe
segment: game
axis: evidence_grade
ticket: T-0961
site: crates/voxel-client/src/scene.rs:1015
what: cut every lit face into subvoxel cells and measure one light sample per grid vertex instead of once per quad corner
because: the field stores one cell per 0.25 m subvoxel, but a face asked only its four corners, so a merged rectangle's shadow was a 1 m blend and no occupancy walk could put a sub-metre edge in the picture
check: cargo test -p voxel-client --lib a_face_asks_the_shadow_at_the_subvoxel_lattice -> failures 0
witness: crates/voxel-client/examples/shadow-lattice.rs
expects: 0
requires: cargo test -p voxel-client --lib
requires: cargo test -p voxel-mesh --lib
requires: cargo test -p voxel-stream --lib
```

```recipe
segment: world
axis: evidence_grade
ticket: T-0962
site: crates/voxel-mesh/src/raycast.rs:479
what: report a non-uniform Mixed leaf as the subvoxel cells the ray crosses, so the consumer spends the walk's answer instead of re-walking the leaf with its own blocking law
because: one metre-wide span with its entry cell spoke for all sixty-four subvoxels, and the second implementation in the consumer skipped Matter::medium, so the same question had two answers
check: cargo test -p voxel-mesh --lib a_leaf_with_a_hole_is_reported_cell_by_cell -> failures 0
witness: crates/voxel-client/examples/shadow-lattice.rs
expects: 0
requires: cargo test -p voxel-mesh --lib
requires: cargo test -p voxel-stream --lib
```
