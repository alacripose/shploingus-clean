# SHAPERS — the terrain shapers and the papers they come from

> The first target is *"a fully playable basic world with all of the terrain
> shapers developed out from their respective research papers."* This document is
> the contract for that: every shaper names the **paper** it is built from, and no
> shaper is allowed to exist as an unexplained pile of noise.
>
> Two shapers (F4's regional fields, F5's deposits) have **zero citable external
> reference** in the old record — their intended references were refused on
> licence. They are therefore built from **first principles plus primary
> literature outside the mod ecosystem**, which is exactly the ruling F5 carries.

---

## 0. The primitives (`voxel-noise`)

Every shaper is built from these. Each is a paper, not a vibe.

| Primitive | Paper |
|---|---|
| Gradient noise | Perlin, K. *An Image Synthesizer.* SIGGRAPH 1985; *Improving Noise.* SIGGRAPH 2002 |
| Simplex-style gradient | Perlin, K. *Noise Hardware.* SIGGRAPH 2001 (Real-Time Shading course) |
| Cellular noise | Worley, S. *A Cellular Texture Basis Function.* SIGGRAPH 1996 |
| Fractal Brownian motion (fBm) | Mandelbrot, B. & Van Ness, J. *Fractional Brownian Motions, Fractional Noises and Applications.* SIAM Review 1968 |
| Ridged multifractal | Musgrave, F.K., Kolb, C., Mace, R. *The Synthesis and Rendering of Eroded Fractal Terrains.* SIGGRAPH 1989 |
| Domain warping | Perlin 1985 §; Quilez, I. *Domain warping* (2002, iquilezles.org) |
| Value noise / coherent hash | Perlin 1985; standard splitmix64 finaliser (Steele et al. 2014) for the seed |

## 1. The shapers (`voxel-shapers`)

Pipeline order is a **stage graph** (F9), not a call chain. A shaper reads only
the stages above it and writes its own declared stage.

| # | Shaper | Paper / source | What it produces |
|---|---|---|---|
| 1 | **`tectonics`** | Kreischer, Bommes et al. *Procedural Tectonic Planets.* CGF 2019; plate-boundary taxonomy from Wilson-cycle literature | a Voronoi **plate partition** + per-boundary **uplift/subsidence** and a **failure schedule** |
| 2 | **`fields`** | F4 ruling (20-level ladder); Shepard, D. *A Two-Dimensional Interpolation Function for Irregularly-Spaced Data.* ACM 1968 | the **regional field ladder**: lithology, elevation, soil depth — multi-resolution, octree-aligned, identity-bound **by effect** |
| 3 | **`geology`** | Allen, P. & Allen, J. *Basin Analysis: Principles and Application to Petroleum Play Assessment.* Wiley 2013 | **stratigraphy**: an ordered layer stack per column, warped by tectonics; dense-ore **deposit dials** (S-22) |
| 4 | **`terrain`** | Mandelbrot 1982 (fractal landscapes); Musgrave 1989 | **elevation** = fBm relief + tectonic uplift − glacial/aeolian smoothing |
| 5 | **`hydrology`** | O'Callaghan & Mark. *The extraction of drainage networks from digital elevation data.* CVGIP 1984 (D8); Strahler 1952; Barnes, Lehman & Mulla. *Priority-Flood.* Computers & Geosciences 2014 | **flow routing**, **stream order**, **lakes** (depression fill), river **velocity** |
| 6 | **`substrate`** | Heimsath, Dietrich, Nishiizumi & Finkel. *The soil production function and landscape equilibrium.* Nature 1997 | **soil depth** from slope/curvature via the soil production function |
| 7 | **`erosion`** | Hansson, H. *Experiments on Hydraulic Erosion.* MSc thesis, Chalmers 2017 (droplet method); Mei, Decaudin & Hu. *Fast Hydraulic Erosion Simulation and Visualization on GPU.* PG 2007; Musgrave 1989 (thermal) | sediment **transport and deposition**, both hydraulic and thermal |
| 8 | **`climate`** | Whittaker, R. *Vegetation of the Siskiyou Mountains.* Ecol. Monogr. 1960; Köppen 1884; ICAO/US Standard Atmosphere 1976 (lapse rate) | **temperature and moisture** as processes, never as a stored field |
| 9 | **`atmosphere`** | US Standard Atmosphere 1976; barometric formula | **pressure / density** by altitude |
| 10 | **`weather`** | Smith, R. & Barstad, I. *A Linear Theory of Orographic Precipitation.* JAS 2004 | **orographic precipitation** + moisture advection; return periods |
| 11 | **`caves`** | Ford, D. & Williams, P. *Karst Hydrogeology and Geomorphology.* Wiley 2007 | **karst dissolution** carving, guided by hydrology's flow |
| 12 | **`ecology`** | Holdridge, L. *Determination of World Plant Formations from Simple Climatic Data.* Science 1947; Reynolds, C. *Flocks, Herds and Schools.* SIGGRAPH 1987 (boids); FC-5.5/5.6/5.7 | **biome triple** per column + **flocking** as a behaviour on any agent |
| 13 | **`delta`** | Gilbert, G.K. *The Topographic Features of Lake Shores.* USGS 1885; Hjulström 1935 / Shields 1936 (transport thresholds); Slingerland & Smith. *River avulsions and their deposits.* 2004 | **sediment deposition** at river mouths; avulsion |
| 14 | **`transition`** | Shepard 1968 (inverse-distance weighting); smooth partition of unity | **domain blending** so two biome/geology regions meet without a seam |
| 15 | **`earthworks`** | F10 ruling. Not a paper: a **conservation ledger.** *A recipe that is a mass pump must declare itself one* | **recorded** player/agent modification of the world, each entry carrying its mass ledger |

## 2. The determinism contract every shaper obeys

1. A shaper is a **pure function** of `(seed, tick, declared period)` and the
   stages above it. No wall clock, no thread order, no RNG state.
2. **Rebuild reproduces truth with digests** (WG-Q4). Two runs are byte-identical.
3. **A declared layer that is absent is a blocker, not an omission** (WG-Q3).
4. Fields are **stage-coupled, not independent noise** (WG-Q5): a climate field
   must be a function of the terrain above it, not a second unrelated fBm.
5. **Worker count is a performance knob** and never changes output (F12).
6. The **six `retire` rows** of `wg-dispositions.jsonl` are honoured, not kept.

## 3. The **voxel** shapers — the per-block level

§1's shapers answer *what is at a position in the world*. This section's answer *what
is a block*: its occupancy at the 4×4×4 level, its materials, and — at the addressed
256 level — the shape of its surface. The same contract applies (`§2`), and the same
rule about sources: **no shaper exists as an unexplained pile of noise.**

### 3.1 Two kinds × two levels, and nothing in between

| | **stored** (`Level::Subvoxel`, the 4×4×4 lattice) | **addressed** (`Level::Detail`, 256 per block) |
|---|---|---|
| **physical** | which of the 64 cells hold matter | a **surface displacement**: real geometry, submitted as triangles |
| **material** | which material a cell holds | a **shading** variation of that material |

The four cells of that table are the whole taxonomy, and three rules hold it honest:

1. **A material shaper cannot change occupancy.** `SlotWriter::set` and `replace`
   refuse an occupancy change in either direction, so a material shaper's block keeps
   its silhouette by construction and not by good intentions.
2. **`acts_at()` is required**, and a shaper that claims the addressed level and then
   declares nothing is **refused by name**. A level claimed with nothing behind it is
   a level that quietly stops being real.
3. **The order of the set is the order of the effects.** The same shapers in a
   different order are a different world, and that is asserted rather than assumed.

### 3.2 No shape is a bitmap

A hand-drawn pattern is a shape somebody picked; a sampled law is a shape with a
**parameter**, and the parameter is derived from the world. `voxel-model::profile`
holds the laws, each with an **exact** normal in closed form (a column's from
`normal ∝ r̂ − R′(z)·ẑ`, a cap's from `normal ∝ n̂ − h′(r)·r̂`), so "fake wrapping" is
correct rather than plausible.

| Law | Source | The one parameter, and where it comes from |
|---|---|---|
| `Stalagmite` — three regimes, flat-top / columnar / conical | Szymczak, Ladd, Lipar & Pekarovič. *Shapes of ideal stalagmites.* PNAS 122(42) e2513263122, 2025 (CC BY) | the **Damköhler number** `Da = πR²k/Q` — derived from the world's drip rate `Q` and base radius `R`. `Da = 1` is **exactly** a hemisphere |
| `Stalactite` | Short, Baygents & Goldstein. *Stalactite growth as a free-boundary problem.* PRL 94 018501 (2005); Phys. Fluids 17 083101 | the tip radius is an **input**, and the free-boundary ideal is **not** implemented — the gap is named in the shaper's own source string |
| `Frustum`, `Cylinder`, `RoundColumn`, `Spheroid` | Requicha 1980 (rigid solid representations); Catmull & Clark 1978; Maekawa 1999 (offsets) | the radius; a spheroid's three Zingg forms |
| Clastic rock: **form × roundness × texture** | Barrett, P. J. *The shape of rock particles, a critical review.* Sedimentology 27(3):291–303, 1980; Zingg 1935 (form, `2/3` boundaries); Powers 1953 (six roundness classes) | three **independent** axes, so a smooth rod and a rough disc are different rocks. Bladed is **refused** — it is not a surface of revolution |
| `Flowstone` ripples (256, physical) | Camporeale et al. *Hydrodynamic-Driven Stability Analysis of Morphological Patterns on Speleothems.* PRL 108 238501, 2012 | the **wavelength is an input**; the paper's preferred-wavelength expression is a named gap |
| `Coral` — six growth forms | reef-survey growth-form classification; the corallite as the skeletal unit | branching is **four bodies**, because one profile cannot be a branch |
| The fake wrap (a shading normal bent without geometry) | Blinn, J. *Simulation of Wrinkled Surfaces.* SIGGRAPH '78 | Blinn's licence: a shading normal may be perturbed **independently of the geometry** |

### 3.3 A cut is a position in 256ths, not three quarters

`Cut` keeps one or more **bands** of a block in 256ths, so an off-centre third, a middle
slice and a plank stack are all expressible. Three rules, each **derived** and each
wrong in the first draft:

1. **A cell survives when a band covers at least half of it** — half is not chosen: a
   displacement moves a face by half a subvoxel, so an edge is trimmable from a cell's
   own face only while it is within half a cell of it. A **tie keeps the cell**, so a
   half-cell cut keeps both cells and they shave toward each other until they meet.
2. **An edge trims the nearest face on the side the material is**, so the residual is
   measured from a *cell face* and never from the block. That is what lets four planks
   off one grid declare **one** amplitude.
3. **A band thinner than half a cell is refused by name**: it covers no cell by half, so
   there is no cell to hold it and the trim would exceed the bound.

### 3.4 Two bounds, because a trim is not a shape

| Bound | Value | What it is about |
|---|---|---|
| `MAX_DISPLACEMENT_SUBVOXELS` | `0.5` subvoxels | a **trim**. Past it the surface would cross the cell boundary and the cell next door would have to move to stay connected — a shape change, and a **carve** at the stored level |
| `MAX_PROFILE_SUBVOXELS` | `2.0` subvoxels (half a block) | a **profile**. The tallest cap a block can hold is the circle inscribed in it, so half a block is the ceiling and it is geometric, not arithmetic |

Sharing one number between the two questions made `RoundColumn { radius: 0.5 }` — the
largest cap a block has, and the pillar the user asked for — **undeclarable**. Two
bounds, two sources, both pinned by test (C-44).

### 3.5 The addressed level reaches a triangle (`voxel-mesh::refine`)

A planar quad is exactly two triangles. A **displaced** quad is not planar, so it is
subdivided at a **declared** rate, **split at block boundaries** (a profile is a law
about a block, and a quad that spanned two blocks would bend one block's shape across
another's), and emitted with the law's own normal at every vertex. A **bare amplitude**
is uniform per `(material, face)`, so it translates a rectangle and stays two triangles
— which is why a cut at `96/256` is exact and costs no subdivision.

The rate is declared and a rate that would explode is **refused with the number in it**:
a block face at the addressed level's own resolution is 65,536 cells — **131,072
triangles** for one block face.

**Owed, named rather than implied:** the merge must stop at block boundaries for a
displaced material (a quad the refinement would have to split is a quad the merge should
not have made); a bare amplitude has a magnitude but no shape of its own, so the
*lattice* supplies the variation and a `Flowstone` ripple is not yet a ripple; and the
roster is **not yet wired into the world** — `world_source::shape_at` still returns a
plain solid.

## 4. What is not here yet, and why

`voxel-shapers` ships **terrain through ecology** first, because those are what a
playable world is made of. `delta`, `earthworks` and the industrial slots are
present as declared stages with their contracts, and their content is scheduled
after the world can be walked in.
