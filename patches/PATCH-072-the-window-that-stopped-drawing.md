---
patch_id: PATCH-072
title: The window that stopped drawing — the lattice is spent only where the light is known to bend, the arena is held to a quarter of its holes, and a corner buffer past the device's ceiling is refused by name
status: applied
operation: fix
target: shploingus/shadow-sampling
base_ref: shploingus-clean@main
base_digest: sha256:cdeb73282b129f8a7b87123918711cd0ec45ef775357880db5f83427c4c3f7dc
dependencies: [PATCH-070]
source: user-directed-conversation
created_at: 2026-10-23
approval: "user-directed 2026-10-23 — the shipped window must keep drawing; this regression outranks any further polish"
live_mutation: false
---

# Change

**The shipped window stopped drawing within seconds.** `SHPLOINGUS_FRAMES=1` on the built binary
printed `Buffer size 536870912 is greater than the maximum buffer size (268435456)` for
`corner-vertices`, every later write to that buffer failed, and the world stopped being drawn. Three
mistakes met in one frame, and each now has one owner.

**1. A face is refined only where its light is *known* to bend.** Since PATCH-070 every face whose
four corner samples disagreed was cut into a `(cells + 1)²` grid, and an unmeasured corner counted as
a disagreement. That is the `quad_present` rule one step too coarse: where a corner has no answer the
face is the rig's, and the grid measures a shadow the fragment will not draw. The condition is now
`step.is_some() && quad_present == 1 && samples differ` — the same rule as the quad's own light, one
step finer. Measured on the settled window: this alone moved the live count only from **2 071 105 to
2 050 887** corners, because the field answers a stale cell for most corners and so most faces are
*known*. It is the right rule for a frontier face; it is not the explanation of the window's size.
The size is the lattice's own price at 0.25 m on merged rectangles, and the next two changes are
what make it fit.

**2. The arena is bounded by the live set, not by a multiple of it.** The free lists serve a landing
from the smallest hole that fits, and a light refresh changes a chunk's slice size (a face that stops
varying falls from a grid to four corners), so the holes accumulate. The old bound compacted at
*twice* the live elements — a bound relative to the live set, not on the array. At 2.07 M live it
allowed 4.15 M corners, past the 3.95 M a 256 MiB buffer holds, so the compactor would have fired only
after the device had already been asked for a buffer it cannot make. The bound is now **a quarter**:
`dead > live / 4 + 4096`, so the array peaks at 1.25 × live (2 402 372 corners, 163.4 MB) and the
capacity is a bound on the geometry rather than a coincidence.

**3. A corner buffer past the device's ceiling is refused, not attempted.** `wgpu` answers an
oversized `create_buffer` with an *invalid* buffer and a validation error: the call succeeds, every
later write fails, and the window stops drawing with no refusal of its own. `CornerPair::ensure_*`
now reads `device.limits().max_buffer_size` first and returns `CornerRefusal { array, asked, limit }`
instead; the pipeline keeps drawing the pair it had and `main.rs` prints the refusal once, with both
numbers, as `refused a corner buffer — <array> would need X MB, past this device's Y MB maximum
buffer size`.

**The declared capacity moved with the lattice.** The old declaration — 128 MiB of vertices and
32 MiB of indices a pair — was a reading of a frame that no longer exists (it was measured at
1 405 673 corners before the light had reached the window). A five-minute run of the shipped window
settles at **2 050 887 live corners and 11 055 720 indices**, with the arena at 2 402 372 corners;
the growth rule's next power of two above the *array* is **256 MiB** of vertices (3 947 808 corners)
and **64 MiB** of indices (16 777 216). Two pairs are **640 MiB retained**, stated in `upload-split`.
256 MiB is `wgpu`'s default `max_buffer_size`, so it is the last number this can reach: on a device
below it the pair is refused by name rather than made invalid.

# Evidence — the shipped binary, not a probe

`SHPLOINGUS_FRAMES=1`, the built `shploingus.exe`, five minutes, 605/605 chunks, 57 175 quads:

- **0 validation errors, 0 corner-buffer refusals.** `corner pairs 640.0 MB retained, 4 allocation(s)
  since start, 6 compaction(s), 0 refused` — the pairs are sized once, before the first frame, and no
  frame pays for an allocation after that.
- **The corners are bounded and stay bounded**: 1 289 453 live at the first sample, 2 050 887 at
  five minutes, with the array between 1.08 × and 1.17 × live at every sample taken (1 395 947 →
  2 402 372). Every 60–90 s the compactor rewrites the arrays in origin order and the holes are
  gone; the count is in the report now (`{} compaction(s)`), so a run says whether the bound was
  exercised.
- **The frame still shows the world.** The shipped-view test reports mean luminance **0.10828**,
  **62.9 % of pixels above 0.01**, max **0.3007**, 3 luminance bands over geometry, 578 205 distinct
  colours (1280 × 720).
- **The subvoxel edge from PATCH-070 is unchanged**: `shadow-lattice` prints a 90 → 10 % shoulder of
  **41.0 px = 0.200 m** at `SHADOW_LATTICE` against **136.0 px = 0.663 m** coarse, 41 646 of 147 456
  pixels differing between the two frames. The example's own assertions pass.
- **The rebuilt binary, after the gate read this record**, 90 s: 605/605 chunks, live 1 272 610,
  `640.0 MB retained, 4 allocation(s) since start, 3 compaction(s), 0 refused`, 0 validation errors.
- Stall timing is unchanged by the compactor: scene p50 27.8 ms / p99 73.6 ms / max 184 ms, against
  28.4 / 72.5 / 207 ms before it (debug build; the compactions are not the tail).
- **The wall, stated**: a 256 MiB vertex buffer holds 3 947 808 corners, so the quarter bound freezes
  the scene only if the live geometry passes **≈3.15 M** corners. The settled window is 2.05 M —
  35 % of headroom under the wall. Past it the refusal is named and the frame keeps the pair it had;
  it does not create an invalid buffer.

# Acceptance

- `scene::tests::a_face_with_an_unmeasured_corner_keeps_four_corners_and_the_rig` — a one-voxel face
  measured on one half and unmeasured on the other is **4 corners, 6 indices, `light_present == 0`**;
  before this change it was a grid.
- `scene::tests::a_face_asks_the_shadow_at_the_subvoxel_lattice_not_once_per_corner` passes untouched:
  a fully measured face whose corners disagree is still the 25-corner grid with the 0.5 m edge
  between two samples.
- `scene::tests::a_walking_window_does_not_grow_the_corner_arena_past_its_bound` — the bound is the
  quarter one and the walk still crosses it (compactions > 0).
- `world::tests::a_scene_arriving_allocates_no_corner_buffer` — four allocations, capacities
  `[(256 MiB, 64 MiB); 2]`, and a 2 402 372-corner / 11 055 720-index scene lands with **no growth**.
- `world::tests::a_corner_buffer_past_the_device_limit_is_refused_by_name` — the refusal names
  `corner-vertices`, the device's own limit, an ask past it, and leaves no buffer behind.
- `cargo test -p voxel-client --lib` — **543 passed, 0 failed, 2 ignored**.
- `cargo check --workspace --all-targets` — clean.

**Three expectations moved, and each because the thing it stated is superseded**:

1. `a_walking_window_does_not_grow_the_corner_arena_past_its_bound` asserted `ARENA_SLACK * live`
   (twice); it now asserts `live + live / 4`. The bound itself is the change; the test still walks
   the same churn and still requires a compaction to have run.
2. `a_scene_arriving_allocates_no_corner_buffer` carried the old measurement (457 400 corners against
   228 700 at the spawn eye); it now hands the pipeline the settled window's own array —
   2 402 372 corners and 11 055 720 indices — and asserts the same thing: no allocation.
3. `corner_capacities` asserts 256 MiB / 64 MiB a pair instead of 128 MiB / 32 MiB, because the
   declared capacity is derived from that measurement and the old one no longer held the window.

# What this does not touch

The shadow lattice itself, the exposure constants (`EXPOSURE_GRADE`, `MOONLIGHT_GAIN`), the gate, the
walk's behaviour and the cell-shell refusal record: untouched. The three expectations above are the
only ones rewritten, and none of them weakens a claim: the bound is tighter, the capacity is larger,
and the test scene is the one this record measured.

# Rollback

Restore `ARENA_SLACK = 2` and the `crossed` condition without `quad_present == 1`, and declare
`CORNER_VERTEX_BYTES = 128 MiB` / `CORNER_INDEX_BYTES = 32 MiB`. The array then grows past the 256 MiB
buffer between 240 s and 600 s, the device refuses the growth, and the window freezes — which is the
state this record exists to end.

## Recipes

```recipe
segment: game
axis: evidence_grade
ticket: T-0963
site: crates/voxel-client/src/scene.rs:1083
what: refine a face at the shadow lattice only when all four corners are measured and disagree, so an unmeasured corner is the rig's four corners instead of a full grid
because: a corner without an answer counted as a disagreement, so at four per cent coverage the frontier's faces paid (cells + 1)² to draw rig light, and the shipped window's corner buffers outgrew the device within seconds
check: cargo test -p voxel-client --lib a_face_with_an_unmeasured_corner -> failures 0
witness: crates/voxel-client/src/scene.rs
expects: 0
requires: cargo test -p voxel-client --lib
requires: cargo check --workspace --all-targets
```
