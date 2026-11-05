---
patch_id: PATCH-087
title: The surface ladder — one generator sample a cell, tiers 8 through 28 drawn and measured
status: applied
operation: feature
target: voxel-client/surface
base_ref: working tree at 2026-10-26, after PATCH-086
dependencies: [PATCH-084, PATCH-085, PATCH-086]
source: user-directed-mission
created_at: 2026-10-26
approval: "user-directed 2026-10-26 — 'the single change that makes tier 8 and everything above it affordable: a coarse census that measures the generator's surface once per cell, with the mesh and the band read from that same measurement, so a group face comes from its own cell instead of a three-deep neighbour census ... rungs above 7 must be drawn, measured and lit at their own cell size before this pass ends'"
live_mutation: false
---

# The objection, and what answers it

The pass that stopped the ladder at 7 named the cost honestly: tier 8's cell is larger than a chunk,
a face is drawn only into a measured neighbour, so a ring of groups needs a **three-deep** group
census — 2 304 chunk octrees ≈ 160 s for tier 8 alone, and every rung above it repeats that at a
larger multiplier. No scheduling change fits that in a launch.

This patch does not schedule it better. It **does not build a tree a cell at all**: a rung above a
chunk is measured by calling the generator's own surface law **once per cell**, and the two readers
of that measurement — the mesh and the band — read the same number. A group face therefore comes
from its own cell's sample, and no neighbour census is needed, which is the whole reason the rungs
above 7 were unreachable.

# What shipped

**`crates/voxel-client/src/surface.rs`** — the ladder's tiers above a chunk, tiers **8 through 28**:

* one sample a column from `voxel_shapers::terrain::elevation_m`, the same function the chunk
  builder uses to decide where the ground is;
* the cell's material from `chunk_fields::material_at` at the voxel the surface falls in, with a
  4-probe reach (measured: over every drawn rung's whole disc the topmost solid voxel is the
  sample's own for 727–834 of 1 089 columns and one below for the rest, except one column of one
  disc at tier 16; a column the probe still finds nothing in is unmeasured — drawn as nothing and
  stamped as nothing);
* the sample's **top face stands at the sample's own height** inside its cell, a cell being 256
  sub-cell steps in the record — **at tier 8 that step is exactly one of the octree's own
  subvoxels** (64 m / 256 = 0.25 m);
* the cell's **band word is the share of that same sample above the cell's floor**
  (`FAR_MEASURED | share × 255`), so the face a fragment sees and the light it is given come from
  one number and cannot disagree;
* vertical faces only into a **measured** neighbour; the annulus's own inner and outer edges draw no
  side face, because an unmeasured neighbour is not air.

**The rings tile.** A rung draws `inner ..= 16` cells from the eye's own cell — `6` for tier 8 (384 m,
the outer edge of tier 7's drawn ring, so the ladder begins exactly where the chunk-held rungs end)
and `8` above it. A cell doubling makes `8 × C_t = 16 × C_{t−1}`, so **each rung's inner edge is the
finer rung's outer edge**: no gap, no overlap, 384 m to 1 073 742 km. The whole 33 × 33 disc is
sampled and only the annulus meshed, because a ray from a ring's inner edge crosses nearer ground
and an unstamped cell there would be an invented clear sky.

**One band buffer, stamped by tier, refusal on mismatch.** Each rung's 35 937 words go to
`horizon_coarse` at that tier's own slot (`HorizonStream::stamp_surface`); a ring whose cell is not
the cell of its slot is **refused**, and so is a ring below the ladder's first tier. The view's
`surfaces` table is a uniform array, its width held to the shader's own literal by
`horizon::tests::the_shaders_reach_and_the_hosts_are_one_number`.

**Progressive, nearest first, one rung at a time** (`SurfaceLadder`, its own thread): the first rung
is on screen 12.79 ms after world-open and the ladder is whole a third of a second later. A crossing
re-measures only the rings whose own cell the eye left, and the ring already in hand stays drawn
until the new one lands — its world extent is fixed, so the union of the rings in hand still covers
the ground.

**The app draws it.** `App::surface` starts the ladder at world-open beside the far field, and
`Scene::add_surface` appends each adopted rung's records; the fragment path reads the rung from the
record's own cell (`light_y` carries the rung, `light_z` the sub-cell step) rather than from a
neighbour.

# The top of the ladder, stated

**Tier 31 is the top: a 536 870 912 m ≈ 536 871 km cell; a rung 16 cells across would span
8 589 934 592 m ≈ 8 589 935 km (22 Earth–Moon distances).** No band of that size is filled and no
ground drawn there — it is address.

**The drawn top is tier 28**: a 67 108 864 m cell, outer radius **1 073 741 824 m ≈ 1 073 742 km**.
It is 28 because the generator's material law is stated in `i32` voxels (`material_at` takes `i32`)
and a rung's disc reaches `16 × cell`, so the last tier whose whole disc can be **named** is the last
one with `16 × cell < 2³¹`; tier 29's outer edge is exactly 2 147 483 648 m, the address space's
edge, and 30 and 31 stand past it. `camera::FAR_M` is now `lod::LADDER_FAR_M` = twice the drawn top's
radius = **2 147 483 648 m**, so the plane follows the ladder instead of cutting it; the cloud
field's 12 500 m sits well inside that.

**The ladder is not stacked inside a few kilometres.** The drawn rungs' outer radii are 1 024 m,
2 048 m, 4 096 m, … 1 073 741 824 m, and the frame draws a 384 m rung and a 1 073 742 km rung
together.

# What it costs, measured

`cargo run --release -p voxel-client --example frame-cost`, one process, 2026-10-26:

| | |
|---|---|
| rungs drawn | **21 of 21** (tiers 8–28) |
| ladder quads | **28 478** (the far field draws 386; the streamed scene 57 175) |
| whole ladder fill | **356.3 ms of samples, 368 ms of wall** — 22 869 generator samples on its own thread |
| per rung | **12.34 ms (tier 10) to 19.14 ms (tier 28)**, most 16–18 ms — flat in the cell size, because the work is 1 089 columns whatever the rung |
| first rung on screen | **12.79 ms** after world-open |
| band memory | **3.29 MiB** — 24 slots (tiers 8–31) × 35 937 words × 4 B = 140 KB a rung, dense and the same at every cell size, inside the buffer the coarse bands already shared |
| frame with the ladder | 28.90 ms against **27.73 ms** for the batch measured immediately before it (tiers 6+7, same run) — **the 21 rungs' own price is +1.17 ms**. Quarter frame 12.67 against 13.95, so the cost is fill, not per-pixel |

**Stated with its bound**: this sitting's own unchanged build spread 27.58–36.60 ms across three
runs, so 1.17 ms is smaller than the instrument's drift here; the batches ran baseline → far →
tiers 6+7 → ladder while the machine drifted *downward*, which makes +1.17 ms an over-estimate over
the immediately preceding batch rather than a flattering one. Against the alternative it is not
close: the three-deep group census wanted ~160 s of *work* for tier 8's ring alone, and the whole
drawn ladder's fill is 0.368 s.

For scale, the chunk-held rungs in the same process: tier 5 22.8 s, tier 6 33.3 s, tier 7 40.0 s.

# Found on the way: a shadow test the near cascade had outgrown

`world::tests::the_beam_shadows_the_floor_and_moves_with_the_light_and_the_world` failed before any
of this patch's work: a 0.5 m roof cast an 11 % dimming (0.589 under it against 0.663 beside it)
rather than a shadow. The cause is arithmetic and belongs to the earlier cascade widening, not to
this patch: the near cascade is 160 m over 1024² (31 cm a texel) and the comparison spans 3 × 3, so
its penumbra is ±0.46 m — wider than that roof's whole 0.5 m shadow. The map was right; the test's
caster was smaller than the map can resolve, a geometry written when the near map was 28 m and its
texel 5.5 cm.

The test now uses a 1.5 m roof over a 4 m floor, sampling 0.75 m inside the shadow's edge and 0.6 m
clear of it on the lit side, and **the assertion is unchanged** (`dark < bright * 0.5`). It passes
with a real shadow. The shader, the map and the cascade are untouched by the repair.

# Evidence

* `surface::` 8/8 — `a_rungs_faces_come_from_its_own_samples`, `a_rungs_band_is_its_own_samples`,
  `every_rungs_samples_stand_inside_its_disc`, `a_rungs_cell_is_its_tiers_cell`,
  `a_rung_begins_where_the_finer_one_ends`, `the_drawn_top_is_where_the_worlds_own_address_stops`,
  `the_ladder_fills_nearest_first_and_a_rung_at_a_time`, `a_crossing_only_moves_the_rungs_it_crosses`.
* `horizon::tests::a_surface_rung_stamps_its_own_samples_into_the_ladders_one_buffer`,
  `a_surface_band_refuses_a_ring_at_the_wrong_cell`, `a_surface_band_refuses_a_tier_below_the_ladder`,
  and the existing tier-5/6/7 band tests unchanged.
* `docs/baselines/frame-cost.json` gains `surface_ladder` and moves the `ladder` section's live
  indices; `docs/lod-ladder.md` gains the mechanism, the top of the ladder and the cost table.
* **The fine path's law, re-measured rather than asserted**: `light-at-a-pixel` reports the same
  named pixels as before this pass — `0.2542 · 0.0699 · 0.1880 · 0.1981 · 0.1455` — with 605 drawn,
  27 inside the cube, 27 stamped and 0 unlit. Every already-shipped rung's own test passes:
  `horizon::` 23/23 (including the tier-5/6/7 band stamps and their refusals), `far_terrain::` 12/12,
  `scene::` 32/32, `shading::` 15/15, `lod::` 3/3.
* **The whole library suite, one run**: `cargo test -p voxel-client --lib`, four threads —
  **601 passed, 0 failed, 2 ignored, 300 s, exit 0**. The GPU modules are minutes each on this
  machine (`renderer::` 131 s, `skylight::` 228 s, `game::` 160 s single-threaded), which is why a
  100 s cut-off on a module reads exactly like a deadlock: the earlier full run was killed at 98 of
  603 and was not hung, it was slow. That reading is now in the baseline's `unstable` list so the
  next reviewer does not repeat it.

# Limits, named

* **The drawn top is 29 tiers short of the address top** — tiers 29, 30 and 31 are not drawn, because
  the material law is `i32` metres and a disc at tier 29 reaches exactly that limit. Sampling height
  out there is `f64`; naming a material is not.
* **A rung's disc is dense and its band is the whole disc.** The band is what a ray crossing the ring
  needs, so the memory is 140 KB a rung rather than a per-cell sparse store; it is flat, not
  growing, because the cell size does not enter it.
* **Tiers 3 and 4 (2 m and 4 m) are still not live** — nothing measures or draws at those cells; the
  fine band's 1 m cell carries the octree's own leaf instead.
* **The band is occupancy and a share, not a marched light.** A rung's own light is the share its
  sample stands above its floor, read by the same march as every other rung; the per-cell light
  *texture* the earlier pass wanted is still not built.
