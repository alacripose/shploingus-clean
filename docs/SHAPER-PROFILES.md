# The 1/32 profile: the surface law a tree is drawn with

**Status: the law is implemented and sampled; the renderer carrier is still owed.**
`voxel_model::profile32` holds **two** records at the same 1/32 denominator — `Shaped`,
a body of revolution, and `Cell`, the per-axis cell a terrain shaper samples. Both are
addressable and tested. No tree is built from `Shaped` yet and no drawn face reads
`Cell` yet: the carrier is the next unit, and this document says plainly which half is
code and which half is owed.

**The column is not a 3D shape here.** A column-shaped record belongs only to the 2D
cell renderer (the shell's cell raster and its map window). The world is cubic: a block
is a cuboid, and its interior is addressed at 1/32 per axis by `Cell`.

## The gap this fills

Three resolutions already exist in this world, and none of them is a tree's surface:

| Level | What it is | Owner |
|---|---|---|
| 1 m voxel | the material and the coarse shape, 4×4×4 cells | `voxel-model::subvoxel` |
| 1/4 m subvoxel | what a physical shaper carves or fills | `voxel-model::shaper` |
| 1/16 m | the **detail lattice** — shading only, addressed, never stored | `voxel-appearance::detail` |

The 4×4×4 grid is 64 cells. A trunk 40 cm across is **one and a half cells**: it cannot
be round, it cannot taper, and its bark does not exist. The 1/16 lattice is a *shading*
lattice with no silhouette — it bends a normal and cannot carve.

`docs/AUDIT-2026-10-24-where-a-package-ends-and-the-game-begins.md` §6 names the
consequence exactly: *"there is no 1/32 profile shaper — `FILL_SIDE = 32` is state
granularity, not a surface law"*. `profile32` is that law.

## Resolution, and why thirty-two

**Thirty-two divisions per block axis: 31.25 mm at 1 m voxels, one eighth of a
subvoxel.** The number is not chosen for looks; it is `coord::FILL_SIDE`, the
denominator the world already uses for a fluid's fill. One denominator for "how full"
and "how much trunk" is what lets a caller compare them without a conversion.

At 31.25 mm a 40 cm trunk is 12.8 cells across — round, taperable, and ridged. A 6 cm
twig is 2 cells, which is the honest limit and is drawn as a 2-cell twig.

Thirty-two is also **not** a storage size. 32³ is 32 768 cells per block — 4 KiB as
bits — and a tree three blocks across and three tall is 27 blocks: 108 KiB of
occupancy for one tree, rebuilt whenever it grows. So the profile is **a function, not
a grid** (C-2): a body is an analytic cross-section plus the numbers that place it, and
`Shaped::coverage` answers for whatever cell is asked about.

## The record

`profile32::Shaped` is the whole per-block cost — a `Profile` by value, six numbers and
a seed. It is what a mesh or an instance buffer carries per profiled block.

```text
profile   the cross-section law: Cylinder / RoundColumn / Frustum / Spheroid / Stalagmite
axis      X, Y or Z — the block frame, no quaternion
centre    where the axis sits in the two in-plane coordinates, in block units
span      how far along the axis the body reaches at stage one (1.0 = the whole block)
girth     the profile's radius at stage one, as a multiple of the law's radius
void      the foliage gap fraction: the share of the envelope that is *not* filled
ridge     the radial flute's amplitude, as a fraction of the radius (bark, not leaves)
seed      the body's stable identity, for flutes and gaps
stage     the developmental stage in 1/1024ths
```

Two channels carry **surface features** and both are deterministic functions of the
cell:

* **`ridge`** flutes the radius in sixteen angular segments, keyed by the seed, so a
  trunk is ridged rather than extruded and the ridges do not crawl between frames.
* **`void`** removes a seeded share of the envelope's cells. A crown is an envelope
  with air in it — `docs/trees.md` records the measured 95.3–98.5 % void fraction of
  five scanned trees — and the air must fall on the *same* cells every time the body is
  drawn or a stationary crown would boil.

## The law

For a body of revolution `r = R(t)` the coverage of a cell is its share of the cell
inside the body, with the boundary anti-aliased over one 1/32 cell:

```text
t, d  = axial fraction along the grown span, distance from the axis
R(t)  = profile.cross_section_radius(t) · girth · stage
cov   = clamp((R(t) − d) / (1/32) + ½, 0, 1) · axial end caps
```

The normal needs no shading trick, because a surface of revolution's normal is exact:

```text
n ∝ r̂ − R′(t)·t̂
```

This is why the profile earns its place beside the material shapers rather than
inside them: a bent normal on a cube is an *approximation* ("fake wrapping"), while
`profile32::Shaped::normal` returns the normal of the surface the coverage describes.
At the resolution where a trunk is 12 cells across, that difference is the difference
between a pipe and a pizza box.

## The per-axis cell: three cuts, one denominator

A tree organ is a surface of revolution; a terrain cell is not. A ramp's top leans in
both horizontal axes, a cliff has a side, and a corner is three planes at once. Bending
`Shaped` into those shapes would be a profile pretending to be something it is not, so
the per-axis case has its own record:

```text
Cell { cuts: [Option<AxisCut>; 3] }
AxisCut { at, inside, lean: [f64; 2] }
```

`AxisCut` is **one plane per axis**, evaluated at the sample's own position:

```text
plane(p) = at + lean[0]·(p[i] − ½) + lean[1]·(p[j] − ½)
inside   = clamp((inside · (p[axis] − plane)) / (1/32 · |∇|) + ½, 0, 1)
```

`lean` is the point-based gradient: a 45° ramp on a Y cut has a lean of `1` along the
axis it rises along, and the anti-aliasing width is measured **perpendicular to the
plane**, so a leaning cut still crosses exactly one 1/32 cell. The cell is the
intersection of its cuts — `coverage` is the product — and `Cell::normal` returns the
normal of the cut the point is nearest to. At a corner that choice is stated rather
than blended: the lowest axis wins, because an averaged normal would be a fourth
surface nobody declared.

The measured witness is `Cell::volume`, the mean coverage over the 32³ grid. A plane at
half the block is `0.5`; one profile cell from the top is `0.03125`; three half-spaces
are `0.125`. It is a witness and not a frame cost — the grid is never walked per frame,
exactly as a profile is never stored. The witnesses are rows in
`docs/baselines/cell-law.json`, so they are re-run rather than quoted.

This is the format the terrain shapers that were refused for want of a per-axis law
were missing; **no drawn face reads it yet**, because the carrier below is still owed.
A `Cell` says *which part of this cuboid*, at the same denominator a fluid's fill uses;
it does not make a column, and it does not change the stored 4×4×4 shape that movement
and collision read.

## Growth: one integer, and a blend

`stage` is developmental progress, not time. It is 1/1024ths so a species can declare a
growth animation without floats in the record, and it scales **both** the body's reach
and its radius: a tree at stage 512 is the same form at half the size. A species whose
girth grows faster than its height declares that in `girth`; it is not a law this
module invents.

Animated growth is therefore cheap and does not rebuild anything:

```text
cov(at) = lerp(cov(stage_from, at), cov(stage_to, at), t)
```

`profile32::blend` is exactly that, and its tests pin the endpoints: at `t = 0` it is
the young stage, at `t = 1` the old one. A renderer animating a season, a sapling or a
felled tree moves one number and evaluates two laws per sample. Nothing is remeshed,
nothing is re-stamped into the light field, and no chunk version moves.

## How a tree mob is built from it

A tree is a **being**, not a decoration: `voxel-shapers::flora` answers *which tree is
in this cell, and what is its identity* (`docs/trees.md`). A tree mob is the drawing of
that being's organs, and it is composed of profiled blocks:

| Organ | Profile | Channel |
|---|---|---|
| trunk below the crown | `Frustum` tapering with the stem's declared allometry | `ridge` |
| branch | `Frustum` along the block's own axis, shorter and thinner per branch order | `ridge` |
| crown envelope | `Spheroid`, one per crown lobe, from the species' slice stack | `void` |
| root flare | `Frustum` with `high_radius` > `low_radius`, below the ground line | `ridge` |

The organ graph (nodes, parentage, age, mass, light) is GAIA §3.2's, and it is *not*
part of the drawing: a profile is where an organ **may** be, and `docs/GAIA_comprehensive.md`
§5.1 keeps that line. Nothing in `profile32` returns mass, and a coverage of one means
only that the cell is inside the declared body.

## Frame-time rules this design is bound by

The frame-time problems this pass's predecessor caused were all **per-frame work over
resident chunks**. The profile system must not repeat that:

1. **No CPU walk of the 32³ grid per frame.** Coverage is evaluated where it is drawn —
   per fragment in the persistent pass — not swept on the CPU to fill a buffer.
2. **No per-frame sweep of the streamer.** A body's profile records are built once,
   when the body is built, and are keyed to the streamer's own residency and version.
   A crossing shifts what is still resident; it does not rebuild the forest.
3. **Growth is a uniform, not a rebuild.** A stage moving is one integer in a buffer
   range the frame already owns.
4. **A body with no profile block is absent from the frame**, not drawn with a
   stand-in, exactly as an unmeshed chunk is.

The nearest thing to a rebuild is a **LOD change**: at distance the crown's void dither
aliases, so a far body should draw the envelope's average coverage instead of its
per-cell gaps. That is a second record — same law, `void = 0`, a lower `stage`
resolution — and it is the one place this design budgets a rebuild.

## What this refuses

* **A stored 32³ grid.** 4 KiB a block for a shape a law answers in four multiplies.
* **A column in the 3D world.** The column record is the 2D cell renderer's; a block
  is a cuboid, and `Cell` is its interior at 1/32.
* **A second ladder.** `FILL_SIDE` is the resolution; no new `PROFILE_LEVELS` table.
* **A growth tick in the renderer.** Stage is data, and who advances it is the being's
  business, not the frame's.
* **Collision from the profile.** Movement reads the 4×4×4 shape; a thin profile cell
  inside a solid block does not make it walkable.
* **Mass from occupancy.** A crown's envelope is not biomass (GAIA §5.1), and this
  module has no ledger.

## What is owed before a tree is on the screen

1. **The mesh carrier.** `Mesh` has `origin` (a body's own corner) and 1 m quads. A
   profiled block needs to carry a `Shaped` record per profiled voxel — the smallest
   form is a parallel array in the chunk's own upload, indexed by voxel — so the
   fragment shader can displace and normal-bend inside a block it already draws. The
   **cell carrier** is the same slot for `Cell`: a per-voxel record the fragment reads
   instead of a flat top. Neither carrier exists yet.
2. **The shader arm.** `shading.wgsl` evaluates the record: radial coverage, the flute,
   the axial taper, the exact normal — and, for a `Cell`, its three cuts and limiting
   plane. This is the pass that makes the trunk round and the cell a ramp instead of a
   staircase at 1/32, and it is a fragment cost, not a geometry cost.
3. **The organ graph and the species slice stacks**, from GAIA §5.6–5.7, in
   `voxel-shapers`, so the records are produced rather than hand-authored.
4. **The light's opinion of a body.** The horizon march and the shadow map still see
   terrain only; a tree casts no shadow and does not shade the ground it stands on.

Until (1) and (2) land, `profile32` is a tested law with no pixels, and saying so is
the point of this section.
