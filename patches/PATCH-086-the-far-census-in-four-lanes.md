---
patch_id: PATCH-086
title: The far census in four lanes, after the rung's own sampling failed its equivalence test
status: applied
operation: optimise
target: voxel-client/far-terrain
base_ref: working tree at 2026-10-26, after PATCH-085
dependencies: [PATCH-085]
source: user-directed-mission
created_at: 2026-10-26
approval: "user-directed 2026-10-26 — 'measure what that actually costs a launch ... If the delay is real to a player, pay it down ... and prove the same 386 quads and the same lit fraction for a fraction of the time'"
live_mutation: false
---

# What the fill actually costs a launch

Not a thread's clock: the shipped app started cold, its probe's lines timestamped by a wrapper
process, and the frame intervals read between them.

| | one lane (PATCH-085) | four lanes (this patch) |
|---|---|---|
| start → first far geometry | **75.9 s** | **35.3 s** |
| start → stack complete | 76.2 s | 35.7 s |
| frames while adopting the stack | 8.5 ms median, no stalls | 9.5 ms median, p95 15 ms, max 21 ms |
| frames while the census runs | 7.0 ms median, p95 15 ms | 8.0 ms median, p95 35 ms |

The fill was **not** stalling the frame once it arrived: the whole stack landed in a third of a
second. The wait itself was real to a player — over a minute of sky where the horizon should be.

**The lanes are not free to the frame while they run.** Before any geometry exists, the four-lane
run's frame intervals were median 8.0 ms, p95 35 ms, max 131 ms, against the one-lane run's median
7.0 ms, p95 15 ms and a single 702 ms start-up gap. That jitter is four census lanes sharing the
machine with the streamer and the renderer; it ends when the census does, and the adoption frames
themselves never exceeded 21 ms. Placed on the record rather than attributed away.

# The named lever, measured and refused

The lever was to census each far chunk at the 8 m cell instead of building the 1 m octree and
averaging it. It was implemented (`census_sampled`) and compared against the tree census over the
104-chunk eye ring:

| samples a cell | census time, 104 chunks | cells whose solidity differs | quads |
|---|---|---|---|
| the tree (truth) | 9.1 s | — | 375 |
| 2 × 2 × 2 (512 samples a chunk) | 0.16 s | **56 of 6 656** | 450 |
| 4 × 4 × 4 (4 096 samples a chunk) | 1.32 s | **23 of 6 656** | 376 |

The sampled census is 6.9× cheaper at four samples a cell and 57× cheaper at two, but **it is not
the same census**: the tree's share is a volume average and the mesh draws a cell at half, so a cell
whose true share sits on the threshold samples either side of it. Four samples a cell already moves
one rectangle in this ring; two move 75. It cannot prove the same 386 quads, so it was reverted
rather than shipped as an approximation under a claim of equivalence.

# What shipped: the same census, four lanes

The censuses are independent — a chunk's tree is a pure function of the spec — so the worker now
builds the missing positions in **`FAR_CENSUS_LANES = 4`** scoped lanes, each with its own
`WorldSource` and its own map, merged before meshing. No shared mutation, no lock, and **the same
values a single lane would produce**: the lane count is wall-clock and nothing else.

* harness: **61.7–62.0 s → 20.4–21.3 s** (712 censuses + 312 meshes), **3.0×**
* launch: **75.9 s → 35.3 s** to the first far geometry, **2.1×** (the app shares the machine with
  the streamer, which is why it is less than three)
* **312 chunks, 386 quads, the same band words, the same lit fraction** — checked by the same
  tests, not by a new tolerance

# Evidence

* `far_terrain::` 5/5, `horizon::` 15/15, `scene::` 32/32.
* The material golden images pass with no fixture regenerated; `the_shipped_view_is_not_a_black_screen`
  passes; `light-at-a-pixel`'s named pixels are unchanged (`0.2542 · 0.0699 · 0.1880 · 0.1981 ·
  0.1455`) — shading was not touched.
* The app probe reports `far: 312 of 312 chunks, 386 quads` in both lane counts, and the peak test
  still draws and lights the summit the single-layer slice clipped.
* `frame-cost`, two clean runs: window 21.71 / 21.80 ms, with the stack 22.30 / 21.89 ms —
  **+0.59 and +0.09 ms**, inside the ±0.6 ms band. The quarter frame is 8.26 / 7.81 ms.

# Limits, named

* **35 s is still late.** The first fill is 20.5 s of census on four lanes; at launch the streamer
  is competing for the same cores, and the app shows the stack all at once because the worker
  censuses a job before meshing any of it. Making it grow in chunk by chunk is a change of its own.
* **Lanes are CPU, not magic.** Four lanes take four cores for the fill — visible as p95 35 ms
  frames before the geometry appears — and a machine with fewer cores will see a smaller win.
* **The 8 m census remains the tree's average.** The cheaper sampled reader is not in the tree; the
  distance between them is measured above (23 cells in 6 656 at four samples) and is the price of
  the exact quads the far field's tests pin.
* PATCH-085's other limits — the stack's bound, the open outer edge, no earthworks, the coalesced
  crossing — are unchanged.

# Tree

`frame-cost`'s far line now states the lane count it measured. No dead code: `census_sampled` and
its constant were removed with the experiment, and the worker's `WorldSource` is the one it uses.
