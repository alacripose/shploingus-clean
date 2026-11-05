---
patch_id: PATCH-083
title: The sky share cannot be a per-cell lookup, measured before any pipeline was built
status: refused
operation: measure
target: voxel-client/shading
base_ref: working tree at 2026-10-26, after PATCH-082
dependencies: [PATCH-077, PATCH-082]
source: user-directed-mission
created_at: 2026-10-26
approval: "user-directed 2026-10-26 — 'the tick writes the measured sky share per cell into a buffer the fragment samples once ... if they cannot, revert cleanly and say what made equivalence impossible'"
live_mutation: false
---

# What was proposed

The three sky rays are 2.38 ms of a 21 ms frame — the largest term left that could be *replaced*
rather than shaved (`docs/baselines/frame-cost.json`). The proposal: a cadence tick marches the
three rays once per fine cell, writes the share into a buffer, and the fragment samples it — the
same mechanism distant terrain will want. The fragment would keep the split law and the bounce
complement.

# What was measured, and why nothing was built

The binding constraint is **equivalence**, not the pipeline, so the sampling law was measured
first: one scratch line makes `world_sky` read at the fragment's **own cell centre** instead of at
the fragment — exactly the value a per-cell (1 m) grid holds and returns.

| pose | shipped | cell-centre sample | named pixel's share |
|---|---|---|---|
| sunward open | 0.2542 | **0.2542** | 1.000 → 1.000 |
| away from the beam | 0.0699 | **0.0699** | 1.000 → 1.000 |
| under a canopy | 0.1880 | **0.2038** (+8.4 %) | 0.667 → **1.000** |
| against a wall | 0.1981 | **0.2150** (+8.5 %) | 0.667 → **1.000** |
| into a wall | 0.1455 | 0.1455 | 0.430 → 0.430 |

**The lookup erases the occlusion it was meant to carry.** At the canopy and wall poses the named
pixel's share jumps from `0.667` to `1.000`: the pixel loses its shadow entirely, because the
thing that blocks one of its three rays — a trunk, a crown, a wall face a metre and a half away —
stands **inside the cell the fragment is in**, and a sample at that cell's centre does not see it.

Interpolation does not recover this. Trilinear blending of the eight neighbours moves a value
*toward* the neighbours' mean; the neighbours here are `1.000`, so it pulls the answer further
from `0.667`, not closer. A step that falls **inside** one cell is not representable by any
sampling of that cell's corners.

The grain that would be needed is the one the march already reads: the horizon is a 1 m cell of
4×4×4 subvoxels, and the three-ray answer turns at that 0.25 m grain. A grid at that grain over
the 96 m cube is `384³` = 56.6 M samples, 226 MB — not a candidate, and the fetch would cost more
than the march it replaced.

**The two poses where the lookup is exact are the two where the term is free to get right**: all
three rays clear, share `1.000` everywhere nearby. That is the shape of the result: a lookup is
exact precisely where there is nothing to measure.

# What was therefore done

Nothing was wired. The scratch line was reverted, and `light-at-a-pixel` reads `0.2542 · 0.0699 ·
0.1880 · 0.1981 · 0.1455` again — the shipped numbers — and `frame-cost` reads 20.77 ms
(48.1 fps), 7.59 ms at a quarter of the pixels, ratio 2.74, unchanged by the experiment.

A half-wired path was not left behind: no buffer, no compute pipeline, no write in `horizon.rs`.

# What would make the architecture possible

The lookup becomes equivalent when the *answer* stops depending on the fragment's position inside
its cell — that is, when the sky share is a property of the cell rather than of the subvoxels in
it. That happens one of two ways, and both are a change to the light law rather than to the
plumbing:

* the occluders are pushed to the coarse rung (8 m cells, where a cell is bigger than the grid),
  which makes the share a coarse field and moves the picture far more than 8 %; or
* the ladder of bands gains a rung between the metre and the eight metres, and the fine share is
  defined *at* that rung's cells.

Either is a decision about what a distant chunk is lit by, and belongs in the light pass, not in a
buffer.
