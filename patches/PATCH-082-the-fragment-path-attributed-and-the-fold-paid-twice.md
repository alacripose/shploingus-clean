---
patch_id: PATCH-082
title: The fragment path attributed, and the ladder's second channel paid for twice
status: applied
operation: optimise
target: voxel-client/shading
base_ref: working tree at 2026-10-26, after PATCH-081
dependencies: [PATCH-074, PATCH-081]
source: user-directed-mission
created_at: 2026-10-26
approval: "user-directed 2026-10-26 — 'use your scratch-variant method on the rest of the fragment path ... then replace whichever term owns the most with something cheaper and equivalent'"
live_mutation: false
---

# The baseline, re-established before anything was optimised

`frame-cost` already anchors the horizon cube on the eye and stamps all 605 resident octrees, so
it was measuring a real frame — but the *number* it carried was from an earlier session and no
longer comparable. This session's baseline, measured minutes before every variant below:

```
median of medians: 23.13 ms (43.2 fps) at 1280x720; 8.64 ms at 640x360 (ratio 2.68)
```

The same harness read 18.04 ms a day earlier. That 5 ms is the machine, not the code, and it is
why every comparison here is against a baseline taken in the same sitting.

# The per-term table

One term disabled at a time by a scratch edit to `shading.wgsl`, rebuilt, the whole recipe re-run.
The run-to-run spread of an unchanged build is about ±0.6 ms, so three of these came back
**negative** — that is the noise floor, stated rather than massaged away.

| disabled | median | cost |
|---|---|---|
| nothing (baseline) | 23.13 ms | — |
| **the material ladder** (`pattern_deviation` whole) | 19.18 ms | **3.95 ms** |
| **the three sky rays** (`world_sky`) | 20.75 ms | **2.38 ms** |
| **the specular warp** | 21.05 ms | **2.08 ms** |
| the beam march | 23.02 ms | 0.11 ms |
| the coarse far band | 23.67 ms | noise |
| the subvoxel tests | 23.70 ms | noise |
| the shadow cascades | 23.52 ms | **not measured** — this harness renders no shadow pass, so `sun_visibility` is never called |
| the rim and the coat | 23.70 ms | noise (predicates false for most materials here) |
| the second bounce | 23.50 ms | noise |

**The cost is not spread thin.** Three terms own 8.5 ms of 23.13; everything else is inside the
noise. The remaining ~14.6 ms is not in any of those terms, and the quarter-size frame's own
8.6 ms says how much of the rest is pixel-independent (geometry, uploads, readback).

# The change: one fold, two channels

The ladder's cost is 32 lattice samples per fragment — two live rungs × **two channels** × eight
corners — and each sample rebuilt its corner's `lattice_domain`: five `mix64`s, each three 64-bit
multiplies built from 16-bit partial products. `grid_sample` was called twice per rung, once per
channel, over the *same eight corners*: **every corner's domain fold was taken twice for one
fragment's answer.**

`grid_sample_pair` walks the corners once, folds each once, and reads both channels from that
fold into a `vec2`. The descriptors are per corner and per channel of the same fold, so the inputs
and the arithmetic are unchanged: same corners in the same order, same fold, same `signed3` per
channel, same weighted sum.

```
median of medians: 21.02 ms (47.6 fps) at 1280x720; 7.81 ms at 640x360 (ratio 2.69)
```

**2.11 ms, 9.1 % of the frame, 43.2 → 47.6 fps.** The quarter-size frame fell 8.64 → 7.81 ms and
the full/quarter ratio did not move (2.68 → 2.69), which is what a per-pixel saving against an
unchanged fixed cost looks like — the change is where it claims to be.

# Equivalent, and how that was checked

* **`light-at-a-pixel`, every named pixel, before and after**: `0.2542 · 0.0699 · 0.1880 ·
  0.1981 · 0.1455`, and the dome columns `0.0692 · 0.0699 · 0.0396 · 0.0421 · 0.0203` — the same
  four decimals, unchanged.
* **`the_material_frames_match_their_golden_images`**: passes, no fixture regenerated.
* **The pattern agreement test** (GPU against `voxel_pattern`'s own lattice): passes.

The first attempt at this pass was **not** equivalent and was caught by the same check: folding
the domain once per *fragment* instead of once per *corner* is not the same value — the domain is
a function of the corner's cell — and `light-at-a-pixel` moved by 5 % at the named pixel. It was
measured, rejected and reverted; what shipped is the pairing, which leaves the values alone.

# Tree

The five tests that a full parallel run flagged are green in isolation with this build
(`golden`, `the_shipped_view_is_not_a_black_screen`, the wireframe, the two trees, the player on
the ground). The full suite under parallel load is **flaky in this environment, not with this
change**: two consecutive full runs failed *different* tests, and a control run on the pre-change
shader failed two different ones again — each of those passes alone. Named in
`docs/baselines/frame-cost.json` under `unstable` as a defect of its own.
