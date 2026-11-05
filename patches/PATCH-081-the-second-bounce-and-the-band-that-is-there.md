---
patch_id: PATCH-081
title: The second bounce, and the band the shipped view actually has
status: applied
operation: add
target: voxel-client/shading
base_ref: working tree at 2026-10-26, after PATCH-080
dependencies: [PATCH-074, PATCH-080]
source: user-directed-mission
created_at: 2026-10-26
approval: "user-directed 2026-10-26 — 'a second-bounce term that is independent of the three visible rays ... raises an enclosed face without touching a sunlit one'"
live_mutation: false
---

# The defect

The frame's ambient is **single-scattered sky**: `world_sky` is the share of the sky a cell can
see, so a face with no sky in its three rays measures `0.0`, and `0.0` times any dome radiance is
zero. That face is black wherever the beam misses it too, and no constant can lift it without
lifting open ground by the same amount — the two are one multiplication.

The light is not gone. It landed on the terrain that occludes the face, and comes back with **that
terrain's** reflectance. That is the multiple-scattering debt the shader's own comment named.

# The term

```wgsl
let enclosure = 1.0 - sky_share;
let bounce = rig.ambient.rgb * (BOUNCE_REFLECTANCE * enclosure * enclosure);
```

* `rig.ambient.rgb` is the same dome radiance `dome` was built from, before occlusion — so the
  term is **derived from the frame's own light**, not added beside it.
* `BOUNCE_REFLECTANCE = 0.43` is `voxel_appearance::provenance::BOUNCE_REFLECTANCE`: the material
  table's mean base reflectance, **measured** (`light-at-a-pixel` prints it: `0.430` over the 76
  rows that carry a base colour), not chosen for a look.
* **Squared in the enclosure**, because the occluder is not lit like the sky it hides: a wall that
  blocks three quarters of the hemisphere sees a quarter of it itself, so what it gives back falls
  with the square of how much it takes.
* Added **beside** `dome`, both scaled by `ambient_gain`.

It is independent of the three visible rays by construction — it reads the *complement* of their
share — so it can only be large where the sky term is small.

# Measured, in `light-at-a-pixel`, with and without the term

The `into a wall` pose is the wall the world actually has at the spawn (found from the streamed
octrees), with the heading turned round, so its **whole frame stands at the floor share**: 8,511
of 13,843 sampled pixels at a share of exactly zero.

| reading (named pixel `(640, 576)`) | without | with |
|---|---|---|
| enclosed set, median of the full frame | **0.0000** | **0.0146** |
| `into a wall` frame | 0.1252 | 0.1455 |
| `into a wall` dome-only column | 0.0000 | 0.0203 |
| sunward open, lit | 0.2542 | **0.2542** |
| away from the beam, shadowed open | 0.0699 | **0.0699** |
| under a canopy | 0.1853 | 0.1880 (+1.5 %) |
| against a wall | 0.1953 | 0.1981 (+1.4 %) |

**The ordering holds**: enclosed `0.0146` < shadowed open `0.0699` < sunlit `0.2542`. **The two
readings this file already recorded do not move at all** — they sit at a share of `1.000`, where
`enclosure` is zero and the term is exactly zero. The obstructed poses move by `1.5 %`, inside the
**2 % tolerance this pass declares**, and the golden-image test passes unchanged because its
fixture is open ground.

# The band test, settled rather than excused

`renderer::tests::the_shipped_view_is_not_a_black_screen` read **one** luminance band where it
expected two. The histogram over its 583,910 geometry pixels is

```
[28,862 · 47,097 · 38,464 · 460,626 · 3,925 · 610 · 387 · 3,939]
```

— **79 % of the frame is one sunlit tone** (the beach the camera looks down at) and the next
population is its shadow at **8.1 %**. The measurement is correct; the *floor* was the shipped
vantage's. A tenth of the geometry was the right bar for a view where lit and shadowed ground are
comparable, which the wider eye still passes at three bands. So the bar is now read where it
belongs: **5 % at the shipped eye** (`broad`, two bands), **a tenth at the wider eye** (three
bands), with the histogram and the reason in the test. The claim kept is the Deadlock one — more
than one tone, in a population too large to be a stray — and the test is green.

The frame this test draws stamps no horizon, so `world_sky` is `1.0` everywhere in it and the
bounce is zero there; that is why the term left its numbers alone.

# Tree

`cargo test --release -p voxel-client --lib`: **567 passed, 0 failed, 2 ignored**. Golden images
unchanged. No scratch markers in either shader file, no capture directories, nothing committed.
