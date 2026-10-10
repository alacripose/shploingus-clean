# Plan — the cloud is cubes: LOD cells, cuboid n-gons, OpenPBR + NPR with translucency

Written 2026-10-08, revised the same day after the user's three rulings. **Derived, not authority**
(`record::DocumentRole::Derived`), like `docs/HANDOFF-2026-10-08-clouds.md`.

Goal: **the cloud is drawn as cuboid cubes built from the fluidics cellular automaton**, the wispy
fullscreen march (`crates/voxel-client/src/clouds.wgsl`) is **deleted** and the freeze-then-crash it
causes goes with it, and the cloud is looked at through the one OpenPBR + NPR stack with translucency.
No edit before approval (vows 5–7). Nothing in this tree's uncommitted work is reverted or discarded.

## 1. The ruling, in the user's own words

> "the minimum a cloud cell can be is the same size as the minimum material lod level, where a block
> can have up to 32 pixels per side, but clouds also use the open pbr npr spec with translucency to
> provide n-gon hyper greedy remesh layers"

> "the meshes are n gons but they are still cuboid it is not that hard to understand and planet
> shploingus has the full materials spec"

> "the clouds should be able to use shapers to provide subvoxel shaping"

Read as clauses:

1. **The cell floor is 31.25 mm** — one pixel of a block at **32 per side**, which is this tree's own
   number: `FILL_SIDE`/`FILL_LEVELS` = 32 (C-164: *"I like the 32x per side of block"*), and
   `pattern.rs`'s finest ladder rung `octave_cell_m(1)` = **31.25 mm** (octave 0, 15.625 mm, is the
   addressed-and-never-drawn lattice). So the cloud's cells are on the **material LOD ladder's own
   rungs** — 31.25 mm … 32 m — and coarsen with distance exactly as the material detail does.
2. **The polygons are cuboid.** The greedy merge produces n-gons — polygons with more than four
   corners — but every corner is a lattice corner and every polygon lies in **one of the six
   axis-aligned planes** of the volume. The merge changes the corner count, never the axes.
3. **The look is the OpenPBR + NPR stack, with translucency.** One look, shared with the terrain.
4. **Layers.** The cloud is drawn as its cell rows — the layer stack — with translucency letting the
   layers read through each other.
5. **The cube is shaped below its own cell size by a shaper** — not by a second geometry path. The
   cell says *which* cells hold cloud; a shaper says what the surface *between* them follows, which
   is the (kind, level) taxonomy `voxel-model::shaper` already states: a **physical** shaper at
   `Level::Detail` declares a `Profile` through `DisplacementField`, and `voxel-mesh::refine`
   subdivides the face and samples that law at an address. See §3.7.

**Reference, not authority** (C-48/C-49): `F:\planet-shploingus\docs\ART-DIRECTION.md` (AR1 OpenPBR
clean-room, AR2/AR3 NPR over it with **NPR transmissives**, AR5/AR6 *derive or name absent*, AR9 ≥32
samples per block axis, AR12 the mesher bar *deterministic projection, local invalidation, stable
merge*, AR14 **the LOD ladder is bounded and blended and its rung thresholds are look decisions**,
AR15 *a shader is never held only to itself*, AR18 ≥3 luminance bands) and `RENDERER-PLAN.md` §10. The
mechanisms are carried; nothing is imported from that tree.

## 2. What this tree already has, so nothing is built twice

| what the cloud needs | what already exists | where |
|---|---|---|
| the CA's cells and the fill level | `FillLevel` (32 per block axis), `Lattice`, `Cell` | `voxel-shapers/src/fluidics.rs` |
| the fill ladder the user names | `FILL_SIDE` = 32, `SUBVOXEL_SIDE` = 4, `DETAIL_SIDE` = 256 | `voxel-model/src/coord.rs` |
| the LOD ladder — rungs, footprint, weight | `octave_cell_m`, `octave_weight` (silent at `cell/4`), `eye_footprint_m` | `voxel-client/src/pattern.rs` |
| the cloud's own field and its interpolation | `CloudField::sample` — the model asked on the 128 m lattice, `blend` between samples (the measured 4,225 asks) | `voxel-client/src/cloud_field.rs` |
| the cloud model | `cloud_column` / `cloud_at` (`water · sin(π·through)`), depths fog 120 / stratus 500 / cumulus 1,800 m | `voxel-shapers/src/clouds.rs` |
| the cuboid boundary rule | `boundary_faces()` — six faces per isolated cell, interior faces never drawn | `voxel-client/src/shell/changes.rs` |
| the greedy merge | the clean-room greedy mesher, one rectangle per plane, T-junctions declared (C-17) | `voxel-mesh/src/mesh.rs` |
| the shaper taxonomy | `PhysicalShaper`/`MaterialShaper` × `Level::Subvoxel`/`Level::Detail`, with `SlotWriter` unable to carve | `voxel-model/src/shaper.rs` |
| the 256-level surface law | `Profile` (`Flat`, `Cylinder`, `RoundColumn`, `Frustum`, `Spheroid`, `Stalagmite`), amplitude derived from the profile, bound half a block | `voxel-model/src/profile.rs` |
| the subdivision that samples the law | `RefineConfig` + `refine` — split at block boundaries, refine, **refuse** past the declared triangle cap | `voxel-mesh/src/refine.rs` |
| the worked precedents | `Slab` (physical at **both** levels), `Column`/`Boulder` (`Profile` bodies), `Flowstone` (physical at **256 only**) | `voxel-shapers/src/voxel.rs` |
| the look | `openpbr` (derive-or-refuse, `transmission_weight`), `npr::shade` (four terms: half-Lambert, rim, specular warp, ambient), and its **one WGSL copy** | `voxel-appearance/src/{openpbr,npr}.rs`, `voxel-client/src/shading.wgsl` |
| the rig, and cloud light from it | `CloudSun::from_rig` — the **uploaded** rig's own sun and ambient | `voxel-client/src/cloud_field.rs` |

Absent, and named: there is **no cloud/air material row** (12 admitted materials; appearance is the
user's), **no n-gon record**, and **no second WGSL copy of the look is allowed to exist** (C-33, AR15).

## 3. The design

1. **Cells.** The cloud's cells are the ladder's rungs. For a region, the drawn rung is the finest
   rung the ladder would draw there — the smallest `cell` with `cell > 4 × footprint` (the ladder's
   own silence rule, inverted), floored at **31.25 mm** and capped at **32 m**. Measured with the
   shipped camera (70° vertical FOV, 720p — 1.70 mrad a pixel, `cell > 0.00679 × d`):

   | eye distance to the cloud | cell drawn | rung |
   |---|---|---|
   | 500 m (a cloud's own base, the measured slab 433–2,270 m) | **4 m** | 8 |
   | 1,000 m | 8 m | 9 |
   | 2,000 m | 16 m | 10 |
   | 4,000 m and beyond | 32 m | 11 |
   | inside a cloud (< 4.6 m) | **31.25 mm** | 1 |

   A rung is drawn only if its **refinement fits the declared triangle cap** (§3.7), and the cap is
   met by bounding the merged rectangles' area rather than by dropping the rung — so the rule is the
   one above, and the budget shows up as the merge's own extent bound.

   **The arithmetic, from `voxel-model::coord`, not from prose**: voxel = 1 m, subvoxel = 0.25 m
   (4 per block), the addressed lattice is `DETAIL_SIDE` = **16 per subvoxel axis** — the "256" base,
   because a subvoxel *face* carries 16×16 samples — so an addressed cell is **15.625 mm**, 64 per
   block axis. The drawn floor (`FILL_SIDE` = 32 per block axis, `octave_cell_m(1)`) is **31.25 mm**,
   exactly one rung above it. Both are the user's own numbers (C-164).

2. **A cell is a cube iff `FillLevel ≥ 1`.** The level is the CA's own encoding: `floor(local water ×
   32)`, where the local water is the model's, read from the interpolated column field on the
   analytic vertical profile — so the model is still asked on its 128 m lattice (4,225 asks) and the
   cells at **any** rung below it cost no extra asks. The level is also the cell's **translucency**:
   the fill decides how much light the cube passes, which is the drawn-fraction idiom the fluids
   already use (C-153.2, C-164).

3. **Geometry: cuboid n-gons, merged, in layers.** The cube set's exposed faces — interior faces never
   drawn, the change view's own `boundary_faces` rule, factored out so there is one implementation —
   are merged by the mesher's greedy rule **generalized to n-gons**: two rectangles sharing a full
   edge are stitched into one polygon when the union stays x- and y-monotone (a cuboid staircase), and
   a stitch that would wrap a gap or make the outline non-monotone is **refused**, not approximated —
   the same refusal the mesher's own "an L, for concave corners where a merge must not span a gap"
   test pins. The merge runs within a layer and, where two faces are coplanar, **across the layer
   stack** — that is the "hyper" part, and it is what collapses the count.

4. **Layers and translucency.** The polygons are drawn after the opaque world, bottom layer first,
   depth test on and depth write off, alpha = the cell's own translucency, so the layers read through
   each other. A clear field draws nothing (the identity frame the current tests pin).

5. **The look, through the one WGSL copy.** A second vertex entry point, `vs_cloud_ngon`, is added
   **inside `shading.wgsl`** — it expands a polygon record into a fan and hands the existing
   `fs_main` its inputs. The fragment shader therefore stays the single copy of OpenPBR + NPR
   (AR15/C-33), and the cloud cannot drift from the terrain's look. The cloud's inputs are the
   appearance table in §4; the record carries only what the geometry needs.

6. **The march is deleted**, with its pass, its layout, its buffers, and `CLOUD_STEP_M`,
   `CLOUD_MAX_DISTANCE_M`, `CLOUD_LIGHT_STEPS`. **Landed 2026-10-08**: `clouds.wgsl` is gone, the
   cloud pass is out of `world.rs`, the boot and cadence call sites are out of `main.rs`, the cloud
   pass's test is out of `renderer.rs`, and `examples/cloud-picture.rs` is deleted with it (its
   whole subject was the march's picture). `cargo check --offline --workspace --all-targets` is
   green.

### 3.7 Subvoxel shaping, through the shaper path (the user's third ruling)

**Nothing in this section is new machinery.** The tree already has the whole chain, and the cloud's
subvoxel shape is required to ride it rather than grow a parallel one:

| stage | what it already does | where |
|---|---|---|
| `PhysicalShaper` / `MaterialShaper` | which *kind* may change occupancy, and at which *level* | `voxel-model/src/shaper.rs` |
| `DisplacementWriter::declare_profile` | a face's law, amplitude **derived** from the profile (`cap_height_total × 4 subvoxels/block`), bound `MAX_PROFILE_SUBVOXELS` = half a block | same |
| `Profile` | `Flat`, `Cylinder`, `RoundColumn`, `Frustum`, `Spheroid`, `Stalagmite` — each an analytic law sampled by whoever refines | `voxel-model/src/profile.rs` |
| `RefineConfig` + `refine` | splits a merged face at block boundaries, subdivides, samples the law per vertex, **refuses** past a declared triangle cap | `voxel-mesh/src/refine.rs` |
| the precedent | `Slab` (physical, **both** levels: carves the cells *and* displaces survivors to the exact plane), `Column`/`Boulder` (`Profile`-driven bodies), `Flowstone` (physical at 256 **only**) | `voxel-shapers/src/voxel.rs` |

So the cloud is a **`PhysicalShaper` acting at both levels**, in `voxel-shapers` where content belongs
(`DESIGN-SPEC.md` §6), with a source named as `docs/SHAPERS.md` requires:

- **`Level::Subvoxel`** — the CA's own `FillLevel` decides which of a cell's `4×4×4` sub-cells hold
  cloud, so a half-full cell is half a cube through the *stored* grid, exactly as a cut is a cut.
- **`Level::Detail`** — `declare_profile(CLOUD, face, Profile::…)` makes the surface between the
  cells follow a law. This is the *only* route by which a drawn polygon is not a cube, and the
  fragment shader never learns about it: the refinement stage moves vertices and the existing
  `shading.wgsl` fragment path shades them like any other surface.

**The refinement rate is a derivation, not a number.** A cloud cell's surface is sampled at the
**31.25 mm drawn floor** and never finer (C-164: *"if there's stuff happening below that, then I
don't want it"*), so samples per cell edge = `ceil(cell_m × 32)`: one sample for the 31.25 mm cell
(a cube — the floor cell is *exactly* the floor and cannot be shaped below itself), 2 at 62.5 mm,
32 at 1 m, 128 at 4 m.

The **cap** is therefore a statement about **one merged rectangle's area**, not about a rung: at a
fixed 31.25 mm sampling the triangle count of a face is `2 × area / (31.25 mm)²` whatever the cell
size, so a 4 m rectangle costs 32,768 triangles and a 32 m one costs 2M. `RefineConfig`'s
`max_triangles_per_quad` refuses past the cap, so the merge carries a second declared bound — **a
rectangle is split before it exceeds `cap · (31.25 mm)² / 2` of area** — and no polygon is ever
drawn that cannot be refined. The alternative lever, if the user prefers it, is to stop drawing the
rung (coarser picture, same budget); §8(b) asks which.

**Absent, and named**: a cloud is not a solid, so its profile is a *signed* law at its faces — a
cloud cell's silhouette wants to be convex outward on the exposed faces and concave where it meets
its neighbour, which the current `Profile` set does not express (every profile is a body protruding
from its own plane). Two routes, and this is §8's question: shape with an existing body profile per
exposed face (`Spheroid` gives equant/oblate puffs; the fourth Zingg class, **bladed**, is already
declared unrepresentable in `profile.rs`), or add a cloud profile whose law is the model's own
water — *derived when the model determines it* (AR5/C-27) rather than chosen.

### 3.8 The cloud needs a material row, and that is now a hard dependency

`DisplacementField::insert` **refuses the empty material id** (`UnadmittedMaterial`: *"a displacement
field was declared for the empty id, which has no surface"*). So a cloud shaper at `Level::Detail`
cannot be declared until a **cloud material is admitted to the registry** (12 rows today). That is
not a nicety of the look any more; it is what makes the shaper legal. The row this plan proposes is
the §3 table: achromatic `base_color` 0.9, `transmission_weight` = `level/32`, `transmission_depth`
= the cell's edge, and **no** `roughness`/`specular_weight` (a whole-cube medium has no surface
roughness). Appearance is the user's call, so it is §8's third question.

### The two look decisions this plan proposes for signature (AR14 / appearance is the user's)

**(a) The rung rule and its cap:** *the finest rung with `cell > 4 × footprint`, floored at 31.25 mm,
capped at 32 m*, per the table in §3.1. The alternative — the coarsest rung — is the budget knob.

**(b) The cloud's appearance row**, per C-27/AR5 (*derived when the model determines it, absent when it
does not*, never zero):

| input | value | why |
|---|---|---|
| `base_color` | achromatic `CLOUD_ALBEDO` = 0.9 | the cloud's own single-scattering albedo, sourced (water droplets); the model records it |
| `transmission_weight` | the cell's `level/32` | the translucency the user names; derived from the same fill the cube is made of |
| `transmission_depth` | the cell's edge | a path length the geometry determines |
| `roughness`, `specular_weight` | **absent** | nothing in the cloud model declares a surface; a whole-cube medium has no surface roughness, so the rim and the specular warp are **not drawn** (as `npr::shade` already does when the row is absent) |
| `metalness`, `coat`, `subsurface`, `emission` | 0 / absent | nothing in the model can declare a conductor, a coat, or a subsurface for a cloud |

Every other constant this change introduces is labelled and sourced where it is declared (AR20's
four-bucket rule; `voxel-ui`'s `CONSTANTS` is the tree's own precedent), and any number that cannot be
derived or sourced is **left absent and reported**, never defaulted.

## 4. Files to touch, with why

| file | why |
|---|---|
| `voxel-client/src/clouds.wgsl` | **deleted** — the march, and the crash |
| `voxel-client/src/cloud_field.rs` | keeps the model's field and its interpolation; the column record gains the cell lattice's own reading (`level_of(column, y_m)`) |
| `voxel-client/src/cloud_cubes.rs` *(new)* | the LOD cell set, the `FillLevel` per cell, and the cuboid n-gon merge — the geometry, in one place, pure and testable with no GPU |
| `voxel-shapers/src/voxel.rs` | the **cloud shaper** — a `PhysicalShaper` at both levels, with its source named; the roster's ordered list gains it |
| `voxel-model/src/materials/`, `column.rs` | the **cloud material row**, admitted (the shaper's precondition, §3.8) |
| `voxel-mesh/src/refine.rs` | nothing changes if the rate is passed in; its `ADDRESSED_RESOLUTION` needs the correction §6 names |
| `voxel-client/src/shading.wgsl` | **one new vertex entry point** `vs_cloud_ngon`; the fragment shader is untouched, so the look stays single-copy |
| `voxel-client/src/shading.rs` | the cloud's appearance row (`MaterialParams::from_inputs` with the §3 table), and the polygon record's layout |
| `voxel-client/src/world.rs` | the cloud pass becomes the cube pass: sky → **cubes (blended layers, no depth write)** → terrain; `write_clouds` becomes the polygon upload; the march's constants go |
| `voxel-client/src/shell/changes.rs` | `boundary_faces_of(cells)` factored out of `ChangeRegion::boundary_faces` — one implementation, two consumers |
| `voxel-client/src/renderer.rs` | the cloud test rewritten for the cube path: a clear field is the frame it was; a cube is drawn and lit by the uploaded rig; the level moves the translucency; **terrain in front hides a cloud**; the ladder coarsens with distance |
| `voxel-client/examples/cloud-picture.rs` | re-pointed at the cube path; it is where the picture is looked at, and it writes the PNGs |
| `voxel-client/examples/surface-format-check.rs` | extended to carry the game's own meshed world at the swapchain format and write a PNG — the handoff's step 0 |
| `shploingus/src/main.rs` | `cloud_field_of` returns the cube field; the call sites keep the atmosphere cadence |
| `docs/LEDGER.md` | appended: these rulings, the rung table, the appearance table, the measured counts, and the crash's diagnosis |

## 5. Change list, in order

1. ~~Keep the **reference frame**~~ **already on disk**: `logs/clouds.png` (318 KB, 04:31) and
   `logs/clouds-none.png` (166 KB) are the march's picture and the same frame with the field cleared,
   written by the release run the user kept in `logs/cloud-picture-release.log`. They stay.
2. **The measurement the plan wanted exists, and it does not say what the plan assumed.** From the
   kept release run (`logs/cloud-picture-release.log`, 2026-10-08 04:27, `exit=0`): the same frame
   with the field cleared is **10.4 ms** and with the model's field **10.3 ms** — the pass measured
   **−0.0 ms** at 960×540 on the offscreen float target — while the **column fill is 119.3 ms** of
   the atmosphere cadence (0.2 s), with 2,996 of 16,384 columns cloudy and a 433–2,270 m slab. So:
   the march's cost is **not reproduced** by the example that draws it (the freeze is the game
   window's, which is the user's report and is not something this log can confirm), and the number
   this tree can *measure* is the **119 ms fill** — 60 % of its own cadence, five times a second.
   That is the CPU-side defect the cube path must not inherit, and it is printed, not asserted.
3. ~~Delete the march~~ **done**; add the polygon record, the `vs_cloud_ngon` stage, and the cloud row.
4. The cell set on the ladder, the `FillLevel` per cell, the cuboid n-gon merge (with its refusal),
   and the change view's face rule factored out.
5. Wire the pass, the layers and the examples; the game path takes the cube field.
6. Verify offscreen (PNG + tests), then **in the running game**, watched.
7. Append the ledger; update the cloud handoff with what landed.

## 6. Risks

- **The picture changes, and that is the request.** The march's soft streaks become cuboid layers.
  The old PNG is kept so the difference is looked at.
- **The ladder's seams are declared, not hidden.** Two rungs meeting quantise the water at different
  cell centres, so a level can disagree across a rung boundary by up to a level — the same class of
  consequence the field's own 128 m stride already documents, and it is measured and printed.
- **T-junctions** are the mesher's existing declared debt (C-17), now inherited at rung boundaries.
- **The fill's 119.3 ms on the frame thread is untouched** by this change — measured, in the kept
  release log, inside a 0.2 s cadence. Its cost is printed on every fill and the thread question
  stays open rather than restructured unasked. It is the largest measured cost in this whole story
  and it is *not* the shader.
- **`FillLevel` is a floor**, so thin cloud already disappears at level 0; the count lost per kind is
  reported.
- **`RefineConfig::ADDRESSED_RESOLUTION` (256) disagrees with `coord::DETAIL_SIDE` (16 per subvoxel
  axis = 64 per block axis, 15.625 mm cells), and `refine.rs`'s own doc states "256 per block" and
  "65,536 cells" from the 256 reading.** The user corrected exactly this on 2026-10-05 (*"256 was
  supposed to be 16x16x16 like the 64 base was 4x4x4"*), so the correction landed in `coord.rs` and
  not in `refine.rs`. This change **depends on which is authoritative** and does not silently pick
  one: it is a question in §8, and until it is answered the cloud's rate is stated in metres.
- **Two things the deletion made visible, and neither is mine.** `world::draw_impl`'s `wireframe`
  flag is reported unused **in the 04:27 log, before this change** (at `world.rs:852`) — so `F4`
  already drew the same frame as `F3` with the march still in the tree: `wire.wgsl`,
  `Scene::quad_edge_bytes` and the edge-index draw exist and are called by nothing. That is in-flight
  work, left exactly as it is. **The cube pass is the flag's first real consumer** and will gate the
  cubes on `!wireframe` (a wireframe frame withholds the world's own matter).
- **The crash is the user's report and this tree has no measurement of it.** The one release run of
  the example that draws the march exits **0** and measures the pass at −0.0 ms; what remains is the
  window, and the removal is the fix that needs no reproducer. Said here rather than dressed as a
  measured speedup.

## 7. Validation (run as written — C-45: no pipes, no filtering)

```text
cargo test --offline -p voxel-client --lib
cargo test --offline -p voxel-shapers
cargo test --offline -p voxel-client --lib -- --ignored
cargo run --offline --release -p voxel-client --example surface-format-check
cargo run --offline --release -p voxel-client --example cloud-picture
cargo run --offline --release -p shploingus
```

Acceptance: no march anywhere; the cells are `fluidics::FillLevel` cells on the ladder with the
31.25 mm floor; every drawn polygon is cuboid and lattice-aligned **in the merged form**, and shaped
below the cell only by a declared `Profile` sampled at 31.25 mm or coarser; the look runs through the
single `shading.wgsl` fragment path; the game shows cube clouds and **does not freeze or crash**; the
cells, polygons, merge factor, rung histogram, refinement rate, ask count, fill ms and the frame's
own luminance bands are **printed**, not asserted from a guess.

## 8. What is the user's to sign (2026-10-08, after the third ruling)

| # | question | the plan's proposal |
|---|---|---|
| a | the **shaping law** for a cloud cell's surface | `Spheroid` per exposed face, its `height` derived from the cell's own water (fuller cell → nearer a hemisphere); the alternative is a new `Profile` whose law *is* the model's water |
| b | the **refinement rate and cap** | sample at the 31.25 mm drawn floor (`ceil(cell_m × 32)`), never finer, and meet the cap by **splitting a merged rectangle before it exceeds `cap · (31.25 mm)² / 2` of area**; the alternative is to stop drawing the rung |
| c | the **cloud material row** | admit one (13th), achromatic 0.9, `transmission_weight` = `level/32`, `transmission_depth` = the cell edge, no roughness/specular — without it the shaper is illegal (§3.8) |
| d | **which level is 256** | *answered as a rule rather than a constant*: "we do not go further than 32 pixels per side of block … bigger blocks can be as big as we need" |

## 9. The rulings as given (2026-10-08), and the two things they brought with them

1. **The shaping law: the water-derived profile, conditional on emergent behaviour** — *"B if it's
   the one that gives us the emergent cloud systems we're looking for"*. So the law is the model's
   own water, as a new `Profile` in `voxel-model/src/profile.rs`, and its justification is not that
   it is prettier than `Spheroid` but that the puffs have to *do* something: the profile is what
   turns the CA's fill into a shape the eye reads as a cloud system. If the water-derived law does
   not show that in the picture, the ruling says to fall back to a body profile, and the comparison
   is the PNG, not an argument.
2. **The cap: split the merge before it exceeds it** — as proposed in §3.7.
3. **The cloud row: admitted, with §3's numbers.**
4. **"We do not go further than 32 pixels per side of block / any of those positions in an
   addressed rendered block, bigger blocks can be as big as we need."** The drawn floor is therefore
   **32 per block (31.25 mm)** — nothing finer is ever rendered — and the ladder's **top is open**:
   the LOD cell size is unbounded upward, which is exactly what makes the n-gon merge affordable at
   range. Consequence for `voxel-mesh/src/refine.rs`: its `ADDRESSED_RESOLUTION = 256` and its
   "256 per block… 65,536 cells… 131,072 triangles" doc are **not** the drawn resolution and get
   corrected to the floor this tree draws at (32 per block), with the addressed lattice (16 per
   subvoxel axis, 15.625 mm) named as the *address* it is — which is the same correction the user
   made on 2026-10-05 when the detail base moved from 256-per-axis to 16×16×16.
5. **And the second scope, from the same ruling: the n-gons are not the clouds' alone.** *"these
   cloud n-gons should also be used for the rest of the terrain so we don't have the insane quad
   inflation."* The terrain's rectangles become the same cuboid n-gons: **one merge, two consumers**.
   C-168 took the rectangle from 280 B to 24 B; this takes away the rectangles that should never have
   been separate — the staircase of boxes a rectangle merge cannot span. It needs a polygon record
   (face, material, plane, corner list), the merge generalized to n-gons with its monotone refusal
   (C-17's declared T-junction debt is inherited here rather than added to), and the terrain's own
   digest and parity tests re-pointed at records that are no longer rectangles. Named as scope, with
   no estimate.

### And one thing the ruling did not ask for, which is already fixed (C-170)

The world's chunks not moving while the material scrolled is **found and fixed**: the origin buffer
holds `origin − eye` and its upload was keyed on the **world-space** origin array, which does not
change as the eye walks. One metre of walking now rewrites 16 bytes per chunk; the witness is the
quad/vertex parity test with a metre of eye, which fails by name without the eye key (24,675
geometry pixels against 24,609) and passes with it (24,609 against 24,609).

And the material pattern's vertical flicker is **diagnosed** (C-171), with the ladder's rung
cross-fade named as the cause and the `FACE_COS_FLOOR` clamp cleared — a vertical step of 0.05 m
changes 24,912 pixels (total 573.1) against the same step's 4,636 (234.0) horizontally, removing the
face term makes it *worse*, and pinning every rung's weight halves the asymmetry. It is a shimmer
with a high rate, not a jump, and the three candidate repairs are named in C-171 rather than chosen
here, because a look is the user's.
