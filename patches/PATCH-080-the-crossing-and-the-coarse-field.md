---
patch_id: PATCH-080
title: The crossing and the coarse field, asked per chunk and per cell
status: applied
operation: measure
target: voxel-client/horizon
base_ref: working tree at 2026-10-26, after the ambient reading of PATCH-079's successor
dependencies: [PATCH-077]
source: user-directed-mission
created_at: 2026-10-26
approval: "user-directed 2026-10-26 — 'extend the same audit to step the eye across at least two crossings ... and do the same for the coarse field beyond the cube'"
live_mutation: false
---

# Change

Two things the spawn-eye audit could not answer, and now can:

* **the crossing**, which is where the user sees black: `light-at-a-pixel` steps the eye
  one chunk at a time across three boundaries (x+1, x+2, x+2/z+1) and re-issues the
  window's own stamp order at each step — the eye's chunk first and outside the budget,
  then the fine cube's twenty-seven and the coarse band's sixty-four, which are
  `HORIZON_CHUNKS_PER_FRAME` and `HORIZON_FAR_CHUNKS_PER_FRAME`;
* **the coarse field**, which had no CPU readout at all: `World::horizon_far_share` is
  `HorizonStream::far_share_at`, already public on the stream and reachable only through
  it. It answers `Some(share)` for a stamped coarse cell and `None` outside the band or
  where nobody has looked, and the example asks it at the far band's own eight metres.

The survey is one function used by both: drawn chunks inside the cube against
`horizon_lit` (the same bit the window prints as `frame: unlit N of M drawn`), the air
above each drawn chunk's own surface against the fine band at 1, 2, 4 and 8 m, the same
air against the coarse band at 8, 16 and 24 m, and the cell the camera stands in.

# What the crossing reads

```
step             drawn in cube stamped  unlit  settled far none far block  fake    eye
spawn              605      27      27      0        0      810         0     0    air
  coarse band: 810 cell(s) unstamped at the crossing frame, 0 after 7 more frame(s)
cross 1: x+1 z+0    605      27      27      0        0        0         0     0    air
cross 2: x+2 z+0    605      27      27      0        0        0         0     0    air
cross 3: x+2 z+1    605      27      27      0        0        0         0     0    air
crossings: GREEN — every drawn chunk kept its record across every crossing
```

**Green, and the green is arithmetic rather than luck.** A crossing owes the cube at most
nine chunks and the window's fine budget is twenty-seven — the number the cube holds — so
no drawn chunk inside the cube is left without a record at a crossing, in either the
crossing's own frame or the one after it. The fine band never calls open air solid above a
drawn chunk at any of the three steps, and the cell under the camera is air at every one.

The coarse band's silence is the only thing the crossing shows, and it is **brightness,
not black**: `world_far_share` returns `1.0` — clear — for a cell nobody has stamped, so
an unmeasured far cell is a chunk with *less* shadowing than it should have. At the spawn
the band needs eight frames of the window's own sixty-four chunks to cover every sampled
cell above a drawn chunk (810 of 5,445 sampled cells unstamped on the first frame); after
a crossing it needs **zero**, because the band reaches 256 m each way and the drawn window
is 160 m, so a chunk has been coarse-stamped for a long time before it crosses into the
cube. A far cell can return `0.0` only when its share reaches `FAR_BLOCKS` (0.75): one
hillside's cell in the ray's way, never an accumulation and never an absence.

# The tree

`skylight::tests::the_declared_ambient_fraction_is_stated_and_is_counted` now states
`1.4` with the reading that moved it; the old assertion still held `0.5`, which the
ambient change of the previous pass had left red.

`renderer::tests::the_shipped_view_is_not_a_black_screen` was also red and was not this
pass's: the shipped eye read one luminance band over its geometry against a floor of two.
Attributed rather than guessed — with `AMBIENT_FROM_HORIZON` put back to `0.5` in a
scratch build the same frame still read one band, so the ambient was not the cause — and
then settled in PATCH-081 with the histogram that shows why.
