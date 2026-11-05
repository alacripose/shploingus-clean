# The LOD ladder

**One world, thirty-two rungs, and every rung is an address step.** The ladder is not a list of
distances somebody chose; it is the world's own structure read upwards.

## What the rungs are

The world is built from three nested counts of 32:

* **32 subvoxels per voxel per axis** — the octree's `Mixed` leaf, the finest address a fragment or
  a ray can name (`SUBVOXEL_CELL_M` = 0.25 m).
* **32 voxels per chunk per axis** — `CHUNK_SIDE`, the unit the streamer builds, meshes and stamps
  (8 m a voxel, so a chunk is 256 m across).
* **32 chunks per composite per axis** — and so on up, each composite the same shape one octave up.

A rung is therefore **one doubling of the cell**, and the ladder runs from the subvoxel to the top of
the address space:

| rung | cell | is |
|---|---|---|
| 0 | 0.25 m | one subvoxel — the finest thing the octree states |
| 5 | 8 m | one voxel |
| 10 | 256 m | one chunk |
| 15 | 8.2 km | one composite chunk block |
| 20 | 262 km | one super-composite |
| 28 | 67 109 km | the coarsest rung the frame **draws** |
| 31 | 536 871 km | the top of the address — a rung 16 cells across spans 8 589 935 km |

A rung is a **measurement**, not a draw distance: every rung the frame draws is drawn because it was
measured, at its own cell size, from the same number its light comes from. The three tiers above the
drawn top are address — a world is allowed to be larger than one frame can march — and the ladder is
kept whole rather than truncated at the horizon so that "is this cell measured?" does not depend on
how far the player happens to be looking.

## What a rung is for

Each rung is a **band**: occupancy and a light measurement at that cell size. The fragment's own
footprint picks the rung, by the same law the material ladder uses (`pattern::rung_level`): a rung is
full where its cell is four pixels across, and retires where its cell is one pixel across. Two
adjacent rungs cross-fade; no rung is chosen by a face's angle to the eye.

That is what makes **LOD light sampling** and **LOD terrain** the same ladder rather than two:

* a distant hill is drawn at the rung its footprint names, and
* the light it is shaded with is measured at *the same rung* — an 8 m cell's own visibility, not a
  1 m march the fragment cannot afford and does not need.

## The cadence tick

The tick is what separates the measurements. A frame cannot rebuild every rung; the tick rebuilds
**one rung's worth of measurements per beat, inside out** — the rung nearest the eye first, because
it is the one whose error is most visible, and each outer rung costs a quarter of the one inside it.

Two rules hold across a rebuild:

1. **A rung that has not been measured this beat keeps the answer it had.** A band is never emptied
   to be refilled: an empty band is "no measurement", and a frame that drew through it draws a
   surface shaded by a light it invented.
2. **A rung the eye leaves keeps its measurement until no chunk that touches it is drawn.** Leaving
   is not the same as changing.

## Where the tree is today

Three light rungs exist: the fine band at 1 m over a 96 m cube, the coarse band at 8 m over a
512 m box, and — since tier 6 — the wide band at 16 m over a 1024 m box
(`crates/voxel-client/src/horizon.rs`). **The 8 m rung now carries terrain as well as
light** (`crates/voxel-client/src/far_terrain.rs`, PATCH-084): each far chunk's octree is censused
once, the census is cut into a coarse mesh on the rung's own stride, its shares are stamped into
the coarse band, and the mesh is drawn from its own record pair beyond the residency window with
the cell size in the record so the material ladder reads the rung the face was cut at.

**The ladder itself now has one owner** (`crates/voxel-client/src/lod.rs`): `TIERS = 32`,
`cell_m(tier)`, `tier_of(cell)` and `cells_per_chunk(tier)`, with the three cells the tree measures
named as tiers — 0 (subvoxel), 2 (1 m, the fine band) and 5 (8 m, the coarse band *and* the far
terrain). The band constants and the far terrain's stride derive from that module, so the two
readers of "how big is a coarse cell" cannot drift apart.

**A census states the tier it was measured at.** `Census::of(tree, tier)` returns a `side³` census
whose cells each carry the voxel count they were measured out of (`512` at 8 m, `4096` at 16 m), and
`coarse_mesh` cuts it on that tier's own subvoxel stride. The shipped field is still tier 5; a tier
6 census (16 m, `2³` cells, 64-subvoxel stride) is built and meshed by
`far_terrain::tests::a_census_can_be_measured_at_another_tier`, so the geometry path is no longer
written against one grain. `HorizonStream::stamp_far` **refuses** a census that is not the far
tier — an 8 m band cannot hold a 16 m measurement, and resampling it would be a second, invented
reading of the same world.

**Tier 6 is now inhabited end to end.** The same `FarTerrain` takes a tier and a ring offset
(`at_tier(spec, radius, 6, 3, 4)`), so the 16 m rung draws 256–320 m at the shipped 160 m window
while the tier-5 ring (224–256 m) is untouched. Its faces are stamped into a **third band**
(`HorizonStream::stamp_wide`): 64³ cells of 16 m over 1024 m, the same share/measured word and the
same walk as the coarse band, refusing a census that is not tier 6. The shader picks the band from
the face's own cell — `VsOut::cell_m` is 16.0 for these records, 8.0 for the far ring, 0.0 for
streamed geometry — so a tier-6 face is lit and shadowed by a measurement at its own grain rather
than by the finer band it happens to sit inside. **The far ring feeds the wide band too**: its 8 m
census is reduced to 16 m (`Census::aggregate`, the sum of the eight cells, checked against a direct
tier-6 census) and stamped, so a tier-6 beam crossing the nearer ring meets its terrain instead of
an unmeasured cell. `horizon::tests::a_tier_six_surface_is_lit_by_the_wide_band_it_stamped`
walks every face of a real chunk's 16 m mesh and requires a measured cell in front of it and
measured matter behind it; `docs/baselines/frame-cost.json` carries the same-run frame price
(+0.18 ms over 14.31 ms) and the 26.3 s fill.

**Tier 7 is inhabited too** — 32 m cells, one a chunk, drawn at `radius+5 ..= radius+6`
(320–352 m) and stamped into a 32 m band (64³ over 2048 m). A 32 m cell **is** the chunk, so the
face's own cell is the one behind it, and the test samples there: all six faces of a real chunk's
cube, each with a measured 32 m cell. The two finer fields also reduce their censuses into the
one-a-chunk band (factors 4 and 2), so a ray crossing them is occluded.

**Which tier indices are live (2026-10-26).** Terrain drawn **and** measured at its own cell:
tiers **5 (8 m), 6 (16 m), 7 (32 m)** — the chunk-held rungs — and **8 through 28**, the surface
rungs below. Light only, no terrain of its own: **2 (1 m, the fine band)**. Not live: **0, 1, 3, 4**
(no field, no band of its cell size, nothing drawn) and **29, 30, 31**, which are address rather
than ground — see "The top of the ladder".

**The band binding no longer caps the ladder.** The four coarse bands are **one storage buffer**
(`horizon_coarse`, binding 12) and each rung's word offset is the fourth float of its own `offsets`
entry in the view. Adding a rung appends words and writes one offset; it does not spend a binding.
Before this, four bands and the fine one used five of the eight storage buffers a fragment stage is
given, which is a ceiling 32 rungs cannot fit under.

**The fill is progressive.** The far worker no longer censuses a whole ring before meshing any of
it: it takes the ring **eight chunks at a time, nearest first**, censuses each wave and its
neighbours in four lanes, and delivers each wave as it is cut. The field's first geometry is on
screen while the rest of the ring is still being measured; `far_terrain::tests::the_first_far_chunk_
arrives_before_the_ring_is_complete` holds that as a property (the first adoption is fewer than the
ring's own count), and a whole-ring delivery fails it.

## Tiers 8 → 28: one sample a cell, and both readers read it

**Every rung above a chunk is now drawn, measured and lit at its own cell size**
(`crates/voxel-client/src/surface.rs`). The mechanism is not the group census and it is not a bigger
ring: it is the generator's own surface function, sampled **once per cell**.

* the sample is `voxel_shapers::terrain::elevation_m` — the same function the chunk builder uses to
  decide where the ground is — so a rung's skyline is the generated surface rather than a staircase
  of cell tops;
* the cell's **top face stands at the sample's own height** inside its cell (a cell is 256 sub-cell
  steps in the record, so a 64 m cell's step is exactly one of the octree's own subvoxels);
* the cell's **band word is the share of that same sample standing above the cell's floor** —
  `FAR_MEASURED | share × 255` — so the light the fragment receives and the face it sees come from
  one number, and they cannot disagree;
* a vertical face is drawn only where a **measured** neighbour stands lower. The annulus's inner and
  outer edges draw no side face: an unmeasured neighbour is not air.

**Why the group census is not what draws them.** It is still in the tree and still tested —
`Census::of_group(origin, 2, tier 8, trees)` sums the eight chunks of a 64 m cell on the last
chunk-held rung, checked against the chunks' own voxel counts
(`far_terrain::tests::a_group_census_is_the_sum_of_its_chunks`), and `FarTerrain::at_group` schedules
such a field. It is not wired to a rung, and the reason is a law rather than a missing feature: a
face is drawn only into a measured neighbour, so for a cell exactly one group wide **every** face
looks outside the ring being drawn, and a group field that censuses only its ring produces no quads
at all (`a_group_cell_draws_nothing_into_an_unmeasured_neighbour`). Drawing one group ring therefore
needs a three-deep group census: 288 groups, **2 304 chunk builds ≈ 160 s** at the measured 69 ms a
chunk — and 8–31 would each repeat it at a larger multiplier. That cost is what stopped the ladder
at 7, and the sample a cell is what pays it: a rung is 1 089 samples, **12.79 ms** at tier 8 and
**368 ms for all 21 rungs**.

**The ring law.** A rung draws an annulus, `inner ..= 16` cells from the eye's own cell, `inner = 6`
for tier 8 (384 m — the outer edge of tier 7's drawn ring, so the ladder begins where the chunk-held
rungs end) and `8` above it. A cell doubling makes `8 × C_t = 16 × C_{t−1}`, so **each rung's inner
edge is the finer rung's outer edge**: the annuli tile the ground from 384 m to 1 073 742 km with no
gap and no overlap. The whole 33 × 33 disc is sampled and only the annulus meshed, because a ray
from a face at the ring's inner edge crosses ground nearer than the ring and an unstamped cell there
would be an invented clear sky.

**The band is stamped, and a mismatch is refused.** `HorizonStream::stamp_surface` writes the ring's
35 937 words into the ladder's **one coarse buffer** at that tier's own slot —
`horizon::tests::a_surface_rung_stamps_its_own_samples_into_the_ladders_one_buffer` — and it
**refuses** a ring whose cell is not the cell of the slot it claims
(`a_surface_band_refuses_a_ring_at_the_wrong_cell`) and a ring below the ladder's first tier
(`a_surface_band_refuses_a_tier_below_the_ladder`). A 64 m measurement cannot answer for a 128 m
cell, and resampling it would be a second, invented reading of the same world.

**The rest of the ladder's laws are tests rather than prose:**
`surface::tests::a_rungs_faces_come_from_its_own_samples` (no drawn face without a measurement
behind it), `a_rungs_band_is_its_own_samples` (the band word is the sample the face was cut from),
`every_rungs_samples_stand_inside_its_disc`, `a_rungs_cell_is_its_tiers_cell`,
`a_rung_begins_where_the_finer_one_ends`, `the_ladder_fills_nearest_first_and_a_rung_at_a_time`, and
`a_crossing_only_moves_the_rungs_it_crosses` (a crossing re-measures the rings whose cell the eye
left, and the ring already in hand stays drawn until the new one lands).

## The top of the ladder

**Tier 31 is the top: a 536 870 912 m ≈ 536 871 km cell, and a rung 16 cells across, so its outer
radius would be 8 589 934 592 m ≈ 8 589 935 km — 22 Earth–Moon distances.** It is part of the
address, not of the draw: no band of that size is filled and no ground is drawn there.

**The drawn top is tier 28** — a 67 108 864 m cell whose outer radius is **1 073 741 824 m ≈
1 073 742 km** — and it is 28 because that is where the world's own address stops. The generator's
material law is stated in `i32` voxels (`chunk_fields::material_at` takes `i32`) and a rung's disc
reaches `16 × cell` from the eye, so the last tier whose **whole disc can be named** is the last
one with `16 × cell < 2³¹`. Tier 29's cell is 134 217 728 m and its outer edge is exactly
2 147 483 648 m, the address space's edge; 30 and 31 stand past it.
`surface::tests::the_drawn_top_is_where_the_worlds_own_address_stops` holds that boundary.

**And the frame's plane follows the ladder, not the sky.** `camera::FAR_M` is
`lod::LADDER_FAR_M` = twice the drawn top's outer radius = **2 147 483 648 m**; the cloud field's
12 500 m sits well inside it.

## What the ladder costs, measured

From `cargo run --release -p voxel-client --example frame-cost`, the same sixth sitting as
`docs/baselines/frame-cost.json`:

| | value |
|---|---|
| rungs drawn | 21 of 21 (tiers 8–28) |
| quads drawn by the ladder | 28 478 (against the far field's 386) |
| whole ladder fill | **356.3 ms of samples, 368 ms of wall** |
| per rung | **12.34 ms (tier 10) to 19.14 ms (tier 28)**, most 16–18 ms |
| first rung on screen | 12.79 ms after the worker starts (two rungs adopted a frame) |
| band memory | **3.29 MiB** = 24 slots × 35 937 words × 4 B, 140 KB a rung, inside the one coarse buffer |
| frame with the ladder | 28.90 ms against 27.73 ms for the batch before it — the 21 rungs' own ***+1.17 ms*** |

The per-rung fill is **flat in the cell size** — the work is 1 089 columns whatever the rung — so the
ladder is not more expensive the further it reaches. The frame price carries a bound worth stating:
this sitting's unchanged build spread 27.58–36.60 ms over three runs, so 1.17 ms is smaller than the
instrument's own drift, and the batches ran in the order baseline → far → tiers 6+7 → ladder while
the machine drifted downward, which makes that 1.17 ms an over-estimate rather than a flattering one.


The field is **three chunk layers** (PATCH-085): the eye's own and one each side, enough for the
world's declared −22 … +54 m surface, with the ring censused one layer beyond the stack so a
summit's top faces are measured air rather than unknown. That slice is deliberately small and its
edges are named there: a stack bounded at a chunk each side of the eye (a valley more than 32 m
below the eye's chunk is not drawn), one far ring two chunks deep, a half-volume threshold, no
earthworks, and a first fill of about 20 s for the three layers in four census lanes — 35 s to the
first far geometry at launch, measured on the shipped app.

**What is left, in the order the constraints allow it:**

1. **A neighbouring law that lets one annulus ring stand alone.** A rung draws `inner ..= 16` and
   samples its whole discard to the outer edge; a thinner ring would let a rung be re-measured
   without the ground inside it, but the band has to answer for every cell its own rays cross, and
   that is where the disc comes from.
2. **The two rungs below the ladder's first.** Tiers 3 and 4 (2 m and 4 m) sit between the fine band
   and the chunk-held rungs and are not live: nothing measures at 2 m or 4 m and nothing is drawn
   at those cells. Tier 0 and 1 are the octree's own leaf and its doubling, carried by the fine
   band's 1 m cell rather than by a band of their own.
3. **The three tiers above the drawn top.** 29, 30 and 31 are address rather than ground until the
   generator's material law can name a point past `i32` metres (`chunk_fields::material_at` takes
   `i32`), which is a change to the world's addressing and not to the ladder.
4. **A measured per-cell light texture** across the rungs, computed on the cadence tick, so a
   fragment samples one value instead of marching a hundred cells — the per-fragment cost the
   current four-band march pays on every pixel of every frame.
