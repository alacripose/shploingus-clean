---
patch_id: PATCH-085
title: The far field stacked above and below the eye, so a peak keeps its top
status: applied
operation: feature
target: voxel-client/far-terrain
base_ref: working tree at 2026-10-26, after PATCH-084
dependencies: [PATCH-084]
source: user-directed-mission
created_at: 2026-10-26
approval: "user-directed 2026-10-26 — 'Give the ring vertical extent as a bounded stack of chunks above and below the eye's layer (enough to cover real relief, not the whole column), reusing the census, mesher and coarse stamp that already work ... Prove it on a peak that the one-layer slice clipped'"
live_mutation: false
---

# What was missing

PATCH-084 drew one chunk layer at the eye's chunk and named the hole: a peak that climbs into the
layer above was cut at the boundary, which is the opposite of a distant horizon. The horizon existed
exactly where the ground was level with the player, and vanished where it rose.

# The stack

`FAR_STACK = 1`: the ring is drawn in `−1 ..= +1` layers of the eye's chunk — 312 chunks instead of
104 — and **the census reaches one layer past the stack** at the ring's own columns, because a
drawn chunk's top and bottom faces ask for the chunk above or below it. The horizontal census margin
(one chunk past the ring) is unchanged, and it is paid only in the drawn layers: a diagonal is never
asked for, so it is never censused.

The bound is the world's own: the declared surface spans roughly −22 … +54 m (a 16 m datum, 40 m of
relief), so one layer each side of the eye's chunk — −32 … +64 m while the eye is in the surface
layer — covers its relief without stacking the whole column. The stack re-centres on the eye's chunk,
not on y = 0, so a player who climbs carries it up.

Three pieces are free functions now (`stack_at`, `ring_at`, `census_at`, `census_for`), which is how
the geometry is tested without starting a worker. `set_eye` retains every chunk still in the stack,
so a crossing still pays for the arc that entered — and a vertical crossing for the new layer only;
the worker's census cache answers the rest.

The census, the mesher, the coarse stamp, the scene's far pair and the ladder's `cell_m` are
untouched. Nothing on the fine path changed.

# Evidence

**The range is the range.** `the_stack_reaches_one_layer_above_and_below_the_eye`: 312 positions,
104 a layer, every drawn layer at its own `y`; no layer beyond the bound is drawn; the census
contains the layer above the stack at the ring's columns (the top faces) and the horizontal margin in
every drawn layer (the side faces), and does not pay for the diagonal.

**The peak the one-layer slice clipped.** `the_stack_draws_a_peak_the_single_layer_would_clip` uses
the world's highest column, 54 m at (1792, 1888): its chunk (56, 1, 59) holds 17 891 solid voxels
with matter up to y = 59, and meshes to 14 quads. For a ring centred six chunks west at (50, 0, 59):

* `ring_at(5, centre, 0)` **does not contain it** — the single-layer field never asked for that
  chunk, asserted rather than assumed;
* `stack_at(5, centre)` **does contain it**;
* every one of its 14 drawn faces answers `far_share_at`, sampled half a metre out along its normal,
  after its census is stamped into a `HorizonStream` — **14 of 14 lit**, so the summit the slice used
  to lose is drawn *and* measured, not a black silhouette.

The eye-layer lit test is unchanged and still passes (`a_far_chunks_surface_is_lit_by_the_coarse_band_it_stamped`,
every face measured in front and standing in measured matter).

**The app draws the stack.** With `SHPLOINGUS_PROBE_DEBUG=1` the running binary reports
`far: 312 of 312 chunks, 386 quads` — three layers adopted and drawn by the same frame.

# Cost

`frame-cost`, same scene, camera and machine, with and without the stack, three clean runs:

| frame | 1280x720 | 640x360 | ratio |
|---|---|---|---|
| the window alone | 21.31 / 21.17 / 21.70 ms | 7.75 / 7.78 / 7.79 ms | 2.75 / 2.72 / 2.79 |
| window + far stack (312 chunks, 386 quads) | 21.18 / 21.65 / 21.69 ms | 8.06 / 7.85 / 8.08 ms | 2.63 / 2.76 / 2.69 |

**The stack's frame price is −0.13, +0.48 and −0.01 ms in the three runs, inside the ±0.6 ms band
an unchanged build spreads over.** That is the honest shape of it: 312 meshes of which most are
empty, 386 quads against the window's 57 175.

**Its generation tripled and a little more, and that is not on the frame.** The stack censuses 712
chunks and meshes 312, and took **61.7–62.0 s of one background thread** — 3.9× PATCH-084's 15.6 s.
Each extra layer is a full ring of tree builds (the census walks the 1 m octree before averaging it
to 8 m), and at this eye the extra layers hold **no surface at all**: the quad count is unchanged at
386, because the eye layer's ground is air above and solid below. The stack's geometry appears only
where relief crosses a layer. The first fill arrives as a whole, since the worker completes the job's
census before meshing it, so the app shows the far field after about a minute rather than growing it
in; a crossing still asks only for the entering arc and the new layer.

# Limits, named

* **The stack is the eye's chunk ±1.** A valley more than 32 m below the eye's chunk is not drawn,
  nor is a peak more than a chunk above it (the world's surface tops out at 54 m, so a player in the
  surface layer covers all of it; a player high above the surface does not).
* **The bottom layer's floor is open by omission.** Its downward faces are against a layer one
  further down that is not censused, so they are not drawn; below the surface those faces are
  interior and invisible, but a cave that reaches the stack's floor would lose its mouth.
* **One far rung, no earthworks, the open outer edge, and the coalesced crossing** are PATCH-084's
  limits and are unchanged.
* **The first fill is a minute** on one thread for the three layers; it is declared above rather than
  hidden, and it is the cost the 64 m rung's cheaper census should pay down.

# Tree

`voxel-client` lib tests: `far_terrain::` 5/5 (the new stack geometry test), `horizon::` 15/15 (the
new peak test), `scene::` 32/32. The material golden images pass with no fixture regenerated,
`the_shipped_view_is_not_a_black_screen` passes, and `light-at-a-pixel`'s named pixels are unchanged
(`0.2542 · 0.0699 · 0.1880 · 0.1981 · 0.1455`). `shploingus` builds and its probe reports the full
312-chunk stack.
