# TODO — the work plan, cross-referenced

> **This file is the plan of record.** It is edited as work lands, and every
> entry names the `LEDGER.md` decision it comes from. A step that is done says how
> it was **verified**, not that it was done. A step that is not done says what is
> missing and what blocks it.
>
> Authority: the user's round-6 message (five asks) plus the four **binding
> corrections** recorded in `LEDGER.md` **C-41**. Where a correction supersedes an
> earlier reading, the correction wins and the superseded reading is struck
> through rather than deleted.

Scope authority: `LEDGER.md` C-41 (the four corrections), C-42 (the shape
sources), C-43 (the quad-versus-triangle answer), C-40 (never test the client
invisibly), `docs/DESIGN-SPEC.md`, `docs/SHAPERS.md`.

**High agency mode: declared explicitly for the round-6 invocation. Logged in
`LEDGER.md` C-41. Per-invocation, non-inheriting.**

**Ponytail is not in use for this repository's workflow. Standing, not
per-invocation — `LEDGER.md` C-45.** No "simplest honest design" tie-breakers, no
re-opening choices the user has already made, no diff-shrinking, no narrating a
design space the user never asked to choose from. Read the code; then write the
code; the scope is the user's and it is already declared.

---

## 0. Standing rules this plan must not break

| Rule | Where | Cheapest way to break it |
|---|---|---|
| **Refuse, never default** | every module | clamping an out-of-range value instead of naming it |
| **Determinism is a contract** — world = `f(seed, tick, declared period)` | F1/WG-Q4 | reading a clock, or an RNG, or a resident chunk |
| **Worker count is a perf knob, never output** | F12 | any parallel reduction that sums in thread order |
| **No shaper is an unexplained pile of noise** | `docs/SHAPERS.md` §2 | shipping a shape with a source string that is a shrug |
| **Never test the client invisibly** | C-40 | a green offscreen test called a check on the client |
| **Appearance is the user's** | `docs/APPEARANCE.md` | deciding a colour, a sun colour or a sky colour unilaterally |
| **Ponytail is ignored for this work — standing** | C-41, **C-45** | shrinking scope, re-opening a settled choice, or dressing a smaller diff up as "the simplest honest design" |
| **Read the record; do not grep it** | **C-45** addendum, **C-88.3**, **C-90** | `grep`ping `docs/LEDGER.md` (6,148 lines / 404 KB) and reporting the hit as the state of the record |
| **The smallest thing worth detecting — not the smallest edit** | **C-90** | finding a true finding and then writing the least that makes it sound resolved |
| **"Simpler" is a caught word — the whole or nothing** | **C-97** | re-deriving a system this lineage already designed out (a `bool` for a refusal vocabulary, a wrapper for a chokepoint, a comment for a total pin) and calling it simpler; the user reads the chain of thought live |
| **Load the Rust skills before coding** — the user's standing reminder (2026-10-03) | `.agents/skills/rust-patterns/SKILL.md`, `.agents/skills/rust-testing/SKILL.md` | writing or reviewing Rust here without loading both skills fresh; the user's words: *"before coding load the rust skills"* |
| **Rust may automate the process parts** — the user's authorisation (2026-10-03) | `HYPERGRILLING.md` working ruling | reaching for Python or an ad-hoc script instead of a Rust generator/harvest step verified by the tree's tests |

---

## 1. The user's round-6 asks

| # | The ask | State |
|---|---|---|
| 1 | Log somewhere to **never use `nodraw`** when testing the client | **done** — `README.md` blockquote + `LEDGER.md` **C-40** |
| 2 | **Sky with a day/night cycle and an atmosphere**, modelling real materials and events | **not started** — the biggest open item; see §4 |
| 3 | Claim to investigate: **"the remesher draws quads instead of tris?"** | **answered** — `LEDGER.md` **C-43**. The mesher emits quads because a quad is the merge unit; `Scene::build` already emits 6 indices per quad (measured `130992/21832 = 6.0`); the *displaced* surface (the new 256 stage) is what genuinely needs triangles |
| 4 | **The per-block subvoxels and sub-subvoxels have nothing to them**; two kinds of voxel shapers | **in progress** — see §2 |
| 5 | **Colour's seven reflectance values** put to the user for approval | **not started** — one-line edits in `voxel-shapers::column`; see §5 |

---

## 2. The voxel shapers — the four binding corrections, tracked

### 2.1 The taxonomy: 2 kinds × 2 levels

| Cell | What it means | State | Verified by |
|---|---|---|---|
| Physical @ **4×4×4** (stored) | which of the 64 cells hold matter | **done** | `shaper::tests::a_physical_shaper_changes_occupancy_and_a_material_shaper_cannot` |
| Material @ **4×4×4** (stored) | which material a cell holds, never the silhouette | **done** | same test, plus `replace_refuses_to_change_occupancy_in_either_direction` |
| Physical @ **256** (addressed) | a **surface displacement** — real geometry, not a shading trick | **done at the type level** | `a_displacement_may_not_cross_a_cell_boundary_and_the_refusal_says_why`, `the_taxonomy_is_two_kinds_crossed_with_two_levels_and_every_cell_is_reachable` |
| Material @ **256** (addressed) | a **shading** variation, per material | **done at the type level** | same two tests |

`acts_at()` is required and **verified**: a shaper that claims `Level::Detail` and
then declares nothing is refused **by name**, which is what stops the level being
decorative (`a_shaper_that_claims_the_256_level_and_declares_nothing_is_refused_by_name`).

**Still owed for the 256 level to be real end to end:**
- [x] the **mesh refinement stage** (`voxel-mesh::refine`): a displaced quad is
      subdivided, split at block boundaries, and emitted as **triangles** with the
      normal the law gives at each vertex — the one place `C-43` says triangles are
      genuinely required. Measured: a `RoundColumn` block face at 4 samples per block
      costs **32 triangles** against the undisplaced **2.0**
- [ ] the refinement is **not wired into the client**: `Scene::build` still emits six
      indices per quad, and nothing calls `refine`
- [ ] the **merge must stop at block boundaries for a displaced material** — a quad
      the refinement would have to split is a quad the merge should not have made
- [ ] the **displacement channel** on the detail lattice (`voxel-appearance::detail`),
      so a *bare* amplitude becomes a variation at an address rather than a uniform
      translation: the seam is declared, the ripple is not a ripple yet
- [ ] a **shader path** for the displaced normal so it reaches a pixel, and a picture
      of it, because C-40 says an eye is the check that finds the most

### 2.2 Cuts are parametric (correction 2) — **done, verified**

The first attempt: `Cut { axis, layers ∈ 1..=3, keep_low }` — a **hardcoded 25 % /
50 % / 75 % plane and one plane at most**. Superseded by `Cut { axis, bands }` in
**256ths of a block**, in `voxel-shapers::voxel`.

- [x] `Cut` is a **256ths** coordinate: `slab(0, 96)` and `slab(85, 171)` are worlds
      the old type could not express (`a_cut_is_a_position_in_256ths_and_not_one_of_three_quarters`)
- [x] a cut is **one or more kept bands** (`Band { low, high }`, low-inclusive):
      `keep`, `discard` (a notch is two bands), `half`, `slab`, `planking`
- [x] `Slab` (physical) carves the cells no band covers and **declares the
      residual** as a displacement that trims the survivors' faces to the exact
      plane — corrections 1 and 2 answered by one mechanism
- [x] `LoopCut` (material) seams the same bands with the silhouette untouched, and
      refuses a seam of the empty id
- [x] the amplitude **is** the residual, in subvoxels, and nobody chooses it

**The three rules that had to be derived, not chosen** (each one was wrong first,
and each is now pinned by a test):

1. **A cell survives when a band covers at least half of it** — not centre
   sampling. Half is the only threshold that agrees with the displacement bound: a
   face moves half a subvoxel, so an edge can be trimmed from a cell's own face only
   while it is within half a cell of it. A tie keeps the cell, so a half-cell cut
   keeps both of its cells and they shave toward each other until they meet.
2. **An edge trims the nearest face on the side the material is** — so the residual
   is measured from a *cell face* and never from the block. That is what lets one
   declaration cover a plank stack that sits off the grid: every plank's top face
   shaves by the same amount *relative to its own face*.
3. **A band thinner than half a cell is refused by name.** It covers no cell by
   half and its trim would exceed the bound; rounding it would be a different shape
   than the one asked for (`a_band_thinner_than_half_a_cell_is_refused_by_name_and_not_rounded`).

Also found and fixed: `Slab` claimed `Level::Detail` **unconditionally**, so a
**grid-aligned** cut claimed the addressed level and declared nothing — and
`ShaperSet::declare` refused it, which is the audit working. `acts_at` is now
**derived** from the cut's own residual. And the amplitude was multiplied by
`SUBVOXEL_SIDE`, making a half-cell trim declare 2.0 where the bound is 0.5.

**Named limit:** the displacement field is keyed by `(material, face)` for the whole
world, so a cut that wants two *different* displacements on one face **refuses**
rather than letting the second overlap the first. Two cut regimens on one material
must be folded under different domains.

### 2.3 The column family, and the speleothems (correction 3) — **done, verified**

The first attempt: `Pillar` insetting the eight cube corners of a **cylinder**.
Superseded — a stalagmite is not a cylinder.

- [x] `voxel-model/src/profile.rs` — surfaces of revolution with **analytic**
      profiles, so a shape is a *law* and the 4×4×4 pattern is a **sample** of it
- [x] `Profile::Stalagmite { da, radius }` from the Szymczak et al. (2025) tangent law
      `cos θ(r) = Da·√(1 − (r/R)²)`; the regimes are **derived**, never declared
- [x] `Profile::{Flat, Cylinder, RoundColumn, Frustum, Spheroid}`, exact normals for
      both a **column** (`column_normal`) and a **cap** (`cap_normal`)
- [x] `Column` as a **physical** shaper (carve outside the silhouette, displace the
      survivors with the profile) and as a **material** one (the inset pattern,
      silhouette untouched) — the latter is the `pillar` the user named
- [x] `stalagmite` derives `Da` from the world's drip rate; `stalactite` names the
      Short–Baygents–Goldstein gap; `cave_column` is a **pair** of frustums with a
      waist and refuses when the waist is not thinnest

**The bug this round found:** `Column::is_outside` used `radius_at` — the radius a
*taper* declares — instead of `cross_section_radius`, the radius the body **has** at
that height. It made a stalagmite a cylinder and a spheroid a cylinder. `Profile`
now has `cross_section_radius` (a spheroid is centred; a stalagmite's section is the
inverse of its own cap; a frustum is its taper) and `Column`, `Boulder::carve` and
`Coral::is_inside` all use it. A **taller** speleothem is a stack whose top block
carries the cap and whose lower blocks carry a `RoundColumn` body — the world's job,
stated in the profile's own doc.

The paper's printed flat-cap radius (`R·√(1 − Da⁻¹)`) still disagrees with its own
tangent law (`R·√(1 − Da⁻²)`); the code implements the law it can verify and the
discrepancy is recorded, not resolved in the paper's favour.

### 2.4 The researched roster (correction 4)

`pillar` and `slab` were **examples**, and treating them as the specification was
the error. Each entry names the primary source that shape comes from
(`LEDGER.md` C-42), because `docs/SHAPERS.md` §2 forbids an unexplained shape.

| Shape | Source | State |
|---|---|---|
| `Slab` | Requicha 1980 (half-space intersection); S-16/S-14 | **done** (§2.2) |
| `LoopCut` | S-14 (press and saw *as a shaper tool*) | **done** (§2.2) |
| `Column` / `pillar` (material, cylinder) | Blinn 1978; Catmull & Clark 1978; Maekawa 1999 | **done** — and the fake wrap now *delegates* to `Profile::cap_normal`, so the round-column law has one statement in the workspace |
| `Stalagmite` (`Da` law, Physical and Material) | Szymczak, Ladd, Lipar & Pekarovič 2025, PNAS | **done**, with the regime and the cap shape derived from the world's drip |
| `Stalactite` | Short, Baygents & Goldstein 2005, PRL 94 018501 | **done as a frustum with a declared tip radius**; the exact free-boundary ideal is **not** implemented and the gap is named in the source string |
| `CaveColumn` (stalactite meets stalagmite) | NPS speleothem morphology; Hill & Forti (1997) | **done** as the pair of frustums with a waist, refusing a waist that is not thinnest |
| `Flowstone` ripples (256, physical) | Camporeale et al. 2012, PRL 108 238501 | **declared, level 256 only** — the wavelength is an **input**, and the paper's preferred-wavelength expression is a named gap |
| `Boulder` / clast (physical) | Barrett 1980 (three axes); Zingg 1935 (form); Powers 1953 (roundness) | **done**, with bladed **refused** (not a surface of revolution) rather than approximated |
| `Coral` (branching, columnar, massive, tabulate, laminar, encrusting) | standard reef-survey growth-form list; corallite diameter | **done** as a set of bodies (branching is four), with the branch girth derived so the colony reads as branches |
| `RiverBed` / delta mouth | `docs/SHAPERS.md` `delta` (Shields 1936, Hjulström 1935) | **owed**, and it is where coral is planted |

**What is still owed is not the shapes, it is their plumbing:**

- [ ] the roster is **not wired into the world**: `voxel-stream::world_source::shape_at`
      still returns `VoxelShape::solid(m)`, so nothing in a streamed chunk has a
      shape yet — the vertical axis no longer blocks this (**z is up**, C-138);
      what remains is the material-library seam (materials steps 2–3)
- [x] the **256 level reaches geometry**: `voxel-mesh::refine` subdivides a displaced
      quad and emits displaced, triangulated, correctly-normalled vertices (§3)
- [ ] the refinement is **not yet called by the renderer**, so no pixel shows it
- [ ] a **`Tally`** in a report, so "3 physical, 0 of them at 256" is a printed
      number rather than an inference — `Tally::occupied_cells()` already exists
- [ ] nobody has **looked** at any of these shapes (C-40), which is the one check
      that has caught the most in this repository

**Named limits, found while building** (each is a refusal rather than a rounding):

| Limit | Why it is not a defect |
|---|---|
| a band thinner than **half a cell** is refused | a kept cell needs half a cell of band to sit in, and a face moves half a subvoxel ([`Cut::trims`](crates/voxel-shapers/src/voxel.rs)) |
| a cut wanting **two different displacements on one face** refuses | a displacement field is keyed by `(material, face)` for the world, so a second, different value cannot be honest |
| a **bladed** clast is refused | it is not a surface of revolution and this library has no exact normal for one |
| a **spheroid's** height is derived, never chosen | a boulder is centred in its block; the height is the clast's own axis |
| a **branching colony's** girth is a derived fraction | below `0.7071` of the colony radius the middle of the block stays outside every branch, at every radius a block holds |

---

## 3. Verification debt (what is claimed but not measured)

| Claim | State |
|---|---|
| The 256 level is real | **type level only** — a field declares a displacement; no geometry carries one |
| Shapes look like their source | **not measured.** Nobody has looked at a pillar. Per C-40 this is stated, not implied |
| A displaced face costs more triangles | **measured**: 32 triangles for a `RoundColumn` block face at 4 samples per block, against `2.0` for the undisplaced quad, and the addressed level's own resolution is priced at **131,072** triangles for one block face — which is why the rate is declared and the cap refuses |
| A profile that is the largest cap a block can hold | **measured**: `RoundColumn { radius: 0.5 }` declares exactly `2.0` subvoxels, and the two bounds are distinguished by test (`MAX_PROFILE_SUBVOXELS` vs `MAX_DISPLACEMENT_SUBVOXELS`) |
| The cut's plane is exact | **measured at the field**: `Cut::slab(0, 96)` declares `−0.5` subvoxels on the cell's own top face, and a plank stack declares **one** amplitude that trims all four planks (`4 × 48/256`) — the arithmetic is verified, the *picture* is not |
| A pillar's side face is half insetted | **known and named**: `VoxelShape::face_recess` asks which material covers *most* of a face, and a cylinder's side is exactly half — the tie breaks by material id (`voxel_model`), so the recess is the *complement* of the shaper's insets there. Which half of a tied face should bend is a question for `docs/APPEARANCE.md`, which is the user's; the wrap's only gate is "the face is not uniform" (`a_side_s_recess_is_half_the_face_and_the_tie_is_named`) |
| The two statements of the round-column law | **closed**: `voxel-appearance::wrap` no longer computes the law, it calls `Profile::cap_normal`, and `the_law_here_is_the_profiles_law_and_not_a_second_copy` pins it |
| An **octree wireframe** view (F4's next brick) | **owed to the chunk/materials return, by the user's choice** (2026-10-04: *"note it for the chunk/materials return"*). F4 outlines the meshes' greedy quads today (`LEDGER.md` C-149); drawing the **tree** means retaining what the streamer drops after meshing (~89 kB/chunk) — that belongs with the cubic-chunks/materials work, not before it |

---

## 3.5 The four bonus goals, as located (not restated by the user)

The user gave these on **2026-09-28** and told this round to find them rather than be
reminded. They are in the Freebuff message record
(`~/.config/manicode/message-history.json`, the message beginning *"your first bonus
goal is recreating the create factory guages system"*), quoted here so the plan of
record holds them and no round has to go looking again:

> your first bonus goal is recreating the create factory gauges system
> a bonus goal you can aim for is having the subvoxels on the surface of water
> animate like the water in the lego movie, a second bonus goal is having a full
> progression system inferred from the mods references, the third bonus goal is the
> complete modding and package api with a fully decorated in-game ui with mods+api
> viewer, main menu, the top bar tabs remade. a fourth goal is integrating
> contraptions with physics like create sable/aeronautics

Note the numbering as recorded: **four numbered goals, plus one unnumbered**
("a bonus goal you can aim for", the water surface). Both readings are kept here
rather than tidied into four, because tidying a user's own numbering is how a goal
gets lost.

| # | Goal | State, honestly |
|---|---|---|
| 1 | **Create's factory gauges** | **built at the model level, unseen.** `voxel-instrument` has `GaugeWall`, `GaugeId`, `GaugeReading`, `Band`, `Site`, `Sensor` and `stock` — the instrument slots were built ahead of the plan. **Owed:** a gauge that reaches the screen, and an eye on it (C-40) |
| — | **Water surface animating like the Lego Movie** | **not started.** The Lego Movie's water reads as *many discrete pieces moving together* — which is exactly what an addressed subvoxel surface is for: the animation must be a pure function of `(tick, address)`, never stored (C-2). The detail lattice (`voxel-appearance::detail`) is the substrate; there is no animation on it yet |
| 2 | **A full progression system inferred from the mods references** | **not started.** "Inferred from the mods references" is the binding clause: the progression must come from `docs/MODS-MAP.md` and the reference list (`references.md`), the way the reflectance values came from USGS — not invented here |
| 3 | **The complete modding and package API, with a decorated in-game UI**: mods+API viewer, main menu, top bar tabs remade | **half built.** `voxel-package` (content intake that refuses an unvalidatable package) and `voxel-ui` (the MD1/HIG design system as types) exist; the HUD is **drawn** — four panels, `glyphon` text. **Owed:** every *interactive* control (the 48 dp floor and the 8 dp rule have no consumer yet), the mods+API viewer, the main menu, and the top-bar tabs |
| 4 | **Contraptions with physics, like Create: Sable / Aeronautics** | **not started.** The design record's own words: `S-16` *"Contraptions are ENGINE"* — a contraption is a **record of modifications** (`voxel-shapers::earthworks` has the append-only ledger: *an unrecorded modification is invisible*), so physics over contraptions is a real subsystem and not a renderer trick |

None of the four is attempted this round: the standing correction from the same
message is that the **256 level must be real**, and a goal built on a level that
reaches no geometry would be built on sand. They are recorded here so they are not
lost, and they are the next stretch once §2 is plumbed through.

---

## 4. Sky, day/night, atmosphere (ask 2) — the researched plan

Not started. The sources are chosen and the discipline is the same as C-39's:
**a value with a source, in the record, or absent and named.**

**Note (2026-10-04, C-152):** the atmosphere is now the **payoff of the fluidics work** rather
than a later pass — §8 items 1–3 are the current work, then the atmosphere and the shapers
ticking. Two of the user's constraints bind this section: **clouds are emergent** (no cloud
prop; the cloud reference is `docs/clouds_and_atmospheric_formation.pdf`) and **rain is drawn as
animated 256-level subvoxels**. The researched rows below stand unchanged.

**Note (2026-10-04, C-153):** the fluidics lattice landed with the 256-level fill interface
(`fluidics::Cell::fill_level`, floored from the exact saturation, `coord::DETAIL_SIDE`), so the
rain animation has its fluid input; the animation itself is still §8 item 6.

**Note (2026-10-04, C-155):** the user re-aimed the sky's **rendering**: the dome is a
one-dimensional elevation table (`Sky::dome_radiance` forms the sun–view angle from two zenith
cosines, so it is exact only in the Sun's meridian plane) and `sky.wgsl` draws it with a
fullscreen triangle; the **bodies are not drawn**. Two write_todo items now carry the repair:
**the Sun and the Moon as pixels** (per the chain's reflectance ruling, quoted in `sky.rs`) and a
**directional sky** whose colour comes from the atmospheric state along the view ray — the
renderer reference is Sebastian Lague's *Coding Adventure: Atmosphere*
(`https://www.youtube.com/watch?v=DxfEbulyFcY`); the lit side of the model stays part of it.

**Note (2026-10-04, C-156):** the **cloud simulation landed** (`voxel-shapers/src/clouds.rs`,
C-152.6 item 3): Magnus–Tetens saturation, the LCL as cloud base at real altitudes, and emergent
cumulus/stratus/fog from the climate, the sun, the terrain and the weather's onset — measured on
seed 2468: 34.1 % noon cumulus cover, 49.5 % midnight stratus, fog in the wettest column,
rain→cloud coupling 719/719. The **drawing** is the two C-155 items above (clouds as sky
features, the Sun and Moon as pixels) plus the falling-rain animation; microphysics (CCN,
coalescence, ice) is named absent. The 256-level rain state exists now
(`clouds::rain_fill_at`).

**Note (2026-10-04, C-157):** the model half of the sky repair landed
(`Sky::cos_view_sun`; `Sky::directional_radiance`; `dome_radiance` is now its Δφ = 0
summary; 89 appearance tests). The user's reminder binds the drawing half: *"the sun and it's
moon refletions is atually core to the rendering of the world itself, and the materials"* — the
Sun, the Moon-as-reflectance and their light are the world renderer's and the materials' source,
not a cosmetic sky. Remaining: the 2-D directional table in the client, the discs, then clouds.
The sourced coefficients below stand; the Preetham row is the analytic model, kept as such.

**Note (2026-10-04, C-158):** the **client drawing half landed** — a 64 × 64 elevation ×
azimuth dome table filled from `Sky::directional_radiance` (`skylight::sky_dome_table`), the
yaw in the sky uniform, and `sky.wgsl` rewritten to turn each pixel into a **world ray** before
indexing the table; the fullscreen triangle remains as the rasterisation device only. The same
`Sky` fills the dome and builds the rig (`sky_at` takes the `Daylight`). `SKY_EXPOSURE` was
re-measured 24 → **12.0** against the repaired geometry (the old value exposed the noon sky at
1.4663). GPU run: `sky mean: up 0.08172, down 0.10431, peak 0.52559`. Next on this row: the Sun
and Moon discs, then clouds and the rain animation — and a newly reported walking-triggered
shading artefact (C-158.4) to pin before either.

**Note (2026-10-04, C-159):** the walking report is **pinned and half-drawn**. The user answered
"See-through water" — *"Water/ocean voxels show the sky through them, and what shows through
changes as I move"*. The cause was in the model, not the sky pass: water carries
`base_color` + `transmission_weight = 1.0` + no roughness, so `npr::shade` gives it only the
ambient term and **no view-dependent reflection at all** — a matte near-black sheet
(`[0.00346, 0.00716, 0.02088]` at the game's own eye), while the sea really is drawn geometry.
Landed: `WATER_IOR`/`WATER_F0` in provenance (IAPWS/Fresnel), **`voxel-appearance::fluid`**
(`reflected_ray`, `schlick`, `surface_radiance`), `skylight::sky_direction_of`, the water Fresnel
branch in `shading.wgsl` reading the same dome, group 1 bound in the world renderer, and
`shading::fluid_surface_cpu`. GPU evidence: CPU model vs shader **2.5e-10**; toward the Sun's
azimuth 0.04881 vs away 0.00820. Workspace test **1580 passed, exit 0**; check exit 0. The three
failing ignored GPU tests were all diagnosed as stale claims and fixed (C-159.5): the derived
rig's max 1.43345 was a **stale exposure** — `skylight::EXPOSURE` was measured against the
placeholder greys C-143 replaced, and is re-measured **8.0 → 4.0** (max 0.71672, mean 0.37842; a
power of two, and low enough that the year's highest noon — where the sun's normalised intensity
reaches 1.0 rather than the test's 0.780 — does not clip either); the
HUD legibility guard `> 20` was re-measured to `> 10` against the 15 text runs H-9.8's pinned
chrome actually lays out (contrast 5.94:1 passes); the mount handover assertion `recipes.len() ==
open` measured the append-only inbox's longevity, not the handover, and now asserts every open
ticket has a recipe while printing the 214 historical receipts.

**Note (2026-10-04, C-159.8):** **the world stopped reshuffling.** `examples/tick-over-world.rs`
measured **270 of 270** surface voxels changing when the spec tick moved, which is why the client
pinned its world at tick 0; every site that samples a *field over position* now uses
`WorldSpec::stationary_stage_seed` (terrain ×2, geology ×4, caves ×2, tectonics ×3, climate ×2,
weather ×2, erosion, delta). Re-measured: **0 of 5445 voxels differ when the tick moves**
(`logs/c159-tick-over-world.txt`), shapers 293 tests green. **The world can be handed a moving tick
now**; what still varies with the tick is what should — the sun, the season, the weather onset. The
client still streams the (seed, tick 0) world, so nothing *uses* it yet: next is the scheduler
(`voxel-work::tick`, tests and no production consumer) driving the stages the clock makes due.

**Note (2026-10-04, C-159.7 + the biome census):** the climate envelope was applying latitude
twice — `base_temperature_c = 15` against a `−45·sin²φ` penalty put the world's declared 45° at
**−7.5 °C** sea level, so only tundra/boreal/alpine could exist and the whole surface was one pale
material. Re-calibrated against the published zonal means (`base 26 °C` equatorial, span 36 → 45° at
≈ +8 °C), the same world now carries **8 of 11 formations** (tundra 7035, grassland 2640,
cool_temperate 2510, boreal 1083, desert 754, subtropical_scrub 299, temperate_forest 197, alpine
123; warmest biotemperature 16.677 °C, was 2.81). The three absent ones need a pole (−5 °C) or an
equator (>20 °C) a 45° world does not have. **Still true and next:** a biome selects no material
(`ColumnProfile::material_at` returns REGOLITH for all positive soil depth), the temperature belt
varies at ~400 m so biomes change every few hundred metres instead of region-by-region, MAP reaches
29,510 mm (Earth's wettest ≈ 11,000), and the 256³ detail lattice is drawn nowhere. **The user's next report, measured:**
the world reads as one pale flat surface — the biome census over the whole ±6 km world
(`logs/c159-biome-census.txt`) is **tundra 8360, boreal 6158, alpine 123** with a warmest
biotemperature of only **2.81 °C**, so no warm biome (and its materials) can ever appear; the
spawn surface is playa-mud regolith, and relief is 26.9 % of the declared 40 m. Also owed: the 256³
detail lattice is drawn nowhere (only `openpbr::derive` reads it), the scheduler in
`voxel-work/src/tick.rs` has no production consumer, and water still lacks waves and depth. Owed:
ice's own F0, then the discs/clouds/rain.

| Piece | Source | Notes |
|---|---|---|
| Solar position (`declination`, `hour angle` → altitude, azimuth) | Meeus, *Astronomical Algorithms* (2nd ed. 1998); NOAA Solar Calculator | pure `f(tick, declared day length, latitude)`, no clock |
| Sky radiance | Preetham, Shirley & Smits (1999), *A Practical Analytic Model for Daylight*, SIGGRAPH | the analytic daylight model — **superseded as the rendering direction by the row below** (C-155) |
| Sky rendering (directional) | Sebastian Lague, *Coding Adventure: Atmosphere* (`https://www.youtube.com/watch?v=DxfEbulyFcY`) — single-scattering Rayleigh + Mie along the view ray, transmittance to the Sun | **the user's direction (C-155)**: the sky's colour from the atmospheric state, the Sun and Moon drawn in it; the CPU model stays the one definition |
| Scattering coefficients | Rayleigh + Mie; Bodhaine et al. (1999) for the standard atmosphere; Henyey & Greenstein (1941) phase function | Rayleigh ∝ 1/λ⁴ is why the sky is blue and sunset red |
| Air mass | Kasten & Young (1989), *Revised optical air mass tables* | the low-sun correction that makes twilight work |
| The **events** | the real ones: civil/nautical/astronomical twilight at sun altitude −6°/−12°/−18°, the golden hour, the blue hour, the **Belt of Venus** | a `SkyPhase` enum with the real thresholds |
| Solar constant / irradiance | 1361 W/m² (CERES/Coddington et al. 2016) | the light source's absolute scale |
| Night | the Moon's Bond albedo **0.12** (NASA) as the night source, plus a real lunar phase from `tick` | a night that is not simply "the sun, dimmed" |

- [ ] `SkyPhase` with real thresholds, and tests at the exact boundary angles
- [ ] `SolarPosition` pure in `tick`; test: equinox noon altitude `= 90° − latitude`
- [ ] `Sky: radiance` — Preetham, evaluated for the sun direction
- [ ] every constant carries a `SourcedValue`-style source, exactly as `SourcedReflectance` does
- [ ] wire the sun + sky into `LightRig`, and **ask the user** before fixing a sky
      colour, because colour is the reserved area
- [ ] a window screenshot, because C-40 says so

---

## 5. Owed to the user (questions, not work)

- [x] **The seven reflectance values** (`LEDGER.md` C-39) — **approved as sourced,
      2026-09-29** (`LEDGER.md` **C-47**), no edits requested
- [ ] **Sky/sun colour**: the reserved appearance area, so it is asked rather than decided
- [ ] A verdict on `docs/APPEARANCE.md` still claiming **S0** — a call the user can
      now make with numbers in hand

---

## 7. The v3 program — the debug views and the shell (round 7, 2026-09-29)

Authority: the user's round-7 instructions (the audit before the code; the debug
views in the client must see the debug systems, including the world shapers) and
the answers in `docs/HYPERGRILLING.md` (**H-7.1 … H-7.25**). The old trees are
**reference only, never authority** (`LEDGER.md` C-48). The completion bar for
this branch is the user's three (**H-7.8**): **playable world · debug views ·
packages & mod UI**.

### 7.1 What the user answered, as work

| # | The answer | The work it names | State |
|---|---|---|---|
| 1 | **Vector change view**: a simple outline over the changed area for a small amount of time, working **with the shell closed**, carrying the terrain shaper debug views (H-7.9) | a world-space outline pass + a transient change ring | not started |
| 2 | **Deferred tick, optimised for the new requirements** — global time, one tick does not run everything at once (H-7.2, H-7.14) | a global clock, declared cadences, render interpolation | not started |
| 3 | **Cuboids at three levels** — player interaction, debug lanes, the shapers that modify the world (H-7.3) | the instrument + its three uses | not started |
| 4 | **Rust shell**: the upper tab bar with this tree's design elements + core windows (movable, resizable, min size, focus/z-order, close) (H-7.4, H-7.11, H-7.15) | shell module + a declared window record | **running in the client** (C-85): `shell/mount.rs` + `shell/hud.rs`, 16 declared windows, 6 tabs, `bar console`, 4 pins; `notification_queue` / `ticket_window` / `change_view` still unchecked |
| 5 | Views are **declared data**, one generic renderer (H-7.5) | the view record + renderer | not started |
| 6 | **Seed per world** (H-7.6) | `GAME_SEED` becomes a world property | not started |
| 7 | Lanes **ported per the new requirements** (H-7.7) | six classes, vocabulary, labels at the lane | not started — next ticket; the dirty tree's `debug.rs` is a **reference, not authority** (C-48), and the rule to carry is *refuse rather than clamp* |
| 8 | A **debug console window + a log window**, in the tiled shell surface (H-7.17, H-7.22) | console + event log | **windows exist and open from the bar; the views do not** (C-85) — both hold a `declared_content_panel` that says so |
| 9 | **Every `cargo test` visible in the client** (H-7.13) | a test inventory the client renders + lanes per failing test | not started |
| 10 | The save = **record + seed + tick**, plus a **client-side chunk cache** so ticks do not re-stream (H-7.16, H-7.20) | the record save + resident cache | not started |
| 11 | The **Mods window is the v3 join point**; the arrangement package is the first package it shows (H-7.19, H-7.23) | Mods window + loader route | not started |
| 12 | **Generation with the debug vector view** (H-7.13) | wire the shaper roster into generation; view it | not started |
| 13 | **Nine schemes × colour themes, composed** (H-16.1) — `redmond` (95/98/2000: thick pixelated borders, dithered glyphs) · `bliss` (XP) · `longhorn` · `vista` · `aero` (glass) · `modern` (8/10, closer to 8) · `material` (true MD) · `ios` (iOS 6 SpringBoard, **tab bar is a dock**) · `touchwiz`, with **colour themes over** them | a scheme record (chrome **form**: border geometry, corner radius, glyph rasterisation, bar height and entry width, title-bar height, strip-vs-dock) × a theme record (colour), and the shell's single constants become **scheme-owned rows** | **ruled, not built** (H-16.1) |

### 7.2 Superseded slice order (historical proposal, not current execution order)

The world-first order below is superseded by H-8.5, C-52.1, H-11.5/C-54.2 and C-56. Retained for history; use §7.3.

1. **The instrument and the lane vocabulary** (`voxel-model::selection`,
   `voxel-client::debug`): the cuboid with its refusing `cells()` and its budget,
   the six classes, the frozen vocabulary, `DebugOverlay`. Tests pin it.
2. **The clock**: one global tick, declared cadences, the tick on screen.
3. **The shell core**: the upper tab bar (design-system tokens) + movable windows
   over the still-running world — first looked at in the window (C-40).
4. **The vector change view** + the transient outline, with the terrain shaper
   debug views riding it.
5. **The console + log windows**, and the test inventory made visible.
6. **The save**: the record + seed + tick, the client chunk cache, the atomic write.
7. **The Mods window** as the v3 join point, arrangement as its first package.

The old open-question list is superseded: H-7.24 accepts tiling plus free-floating windows and HUD pinning; H-7.25 approves the tab set; H-7.26 adds tools and map/plan plus the full design integration. Cache/save requirements continue in H-7.28 and H-7.20.

### 7.3 Current alpha execution order (C-56)

1. Record/API/notification contracts from C-55 §55.4, preserving the latest user answers.
2. Strict recipe declarations, versioned package codec, loader and arrangement-package proof. No executable package content.
3. Mandatory game build conformance pass; the same findings are embedded for client readers. No claim that schema validation is a calibrated Laya oracle.
4. Mount the arrangement through the Rust shell API: tabs, window/pane ownership, floating/tiling/pinning, Mods and API inspection, shell-owned notification queue.
5. One namespaced event chain, world seed/clock/cached working set, save/load, vector change and debug/test observations.
6. Workspace tests and clippy, package negative-path checks, optimized artifact, and drawn-client acceptance. Ship-ready does not mean published.

**State:** active; none of this new integration is claimed complete by the plan. The full alpha scope remains H-7.8; bonus goals are not substituted for it.

---

## 6. Round log

| Round | What landed | Verified |
|---|---|---|
| 6, first pass | shaper kinds, `slab`/`loop-cut`/`pillar`, `voxel-ui` HUD wiring | 461 → 474 tests, clippy clean |
| 6, correction pass | `LEDGER.md` C-40…C-43 written; two kinds × **two levels**; `DisplacementField`/`ShadingField`; `VoxelShape::face_recess`; `Tally`; the 256 declaration **verified** | `cargo test -p voxel-model` → **61 passing, 0 failing** |
| 6, plumbing pass | the **parametric cut** finished: `Band`/`Cut`/`Trim`, the half-cell rule, the nearest-face trim, the sliver refusal, `Slab::acts_at` **derived**, the amplitude units fixed; `Profile::cross_section_radius` and the three call sites that needed it; `voxel-appearance::wrap` rewritten to **delegate** to `Profile::cap_normal`; `docs/TODO.md` created and kept current; the four bonus goals **located and recorded** (§3.5) | `cargo test --offline --workspace` → **534 passing, 0 failing, 7 ignored**; clippy clean; per crate: `voxel-model` 76, `voxel-appearance` 74, `voxel-shapers` 148 |
| 6, geometry pass | `voxel-mesh::refine` — the addressed level reaching a **triangle**: split at block boundaries, `2` triangles for a planar or uniformly trimmed quad, a **refused** cap instead of a lowered resolution, and the apex/rim/normal laws pinned against the profile; `LEDGER.md` **C-44** | `cargo test --offline --workspace` → **542 passing, 0 failing**; `cargo clippy --offline --workspace --all-targets -- -D warnings` → **clean** |
| 6, playable-loop pass (this one) | **digging and building under concurrency**: `Pool::edit` (the world behind an `RwLock`, a generation stamped on every built mesh, stale meshes discarded), `Streamer::edit` invalidating exactly the chunks the mesher's one-voxel pad reaches, `Streamer::pick` over one shared `Crossings` traversal, and break / place / material-select wired into the window; `spawn_eye()` derived from the terrain law; `LEDGER.md` **C-45** (ponytail standing, and the run is the user's to read), **C-46** (the wrong-world measurement), **C-47** (the seven reflectance values approved) | `cargo test --offline --workspace` → **552 passing, 0 failing, 8 ignored**; clippy clean; the shipped view measures **56.1 %** of pixels above 0.01 |
| 15, shell ticket | **the shell runs in the client** (C-85): `Mount` (windows, z-order, bar, filed rows, tab menu, focus, drag, dock/undock, per-policy controls), the pinned layer as a filter over the same painter, the corner HUD deleted, the design doctrine carried into `voxel-ui`, and `alpha-shell.shspec` mounted as the arrangement. The visible run found **two defects the tests had not**: the package declared twelve windows too short for the view they host, and one window's refusal took the whole shell off the screen. The package was corrected and the refusal made per-window; **neither was weakened** | `cargo test --offline --workspace` → **859 passing, 0 failing, 8 ignored**; clippy clean; `cargo run --offline -p shploingus` → 16 windows, 6 tabs, 4 pins, 605/605 chunks, Escape/F1 toggles the shell with the pinned windows staying |
| shaper loose ends (2026-10-09) | **`SoilTransport` and `Ecology` bodies** (`LEDGER.md` **C-175**): Roering 1999 nonlinear hillslope diffusion with its discrete operator stated, and Lieth 1975 / Holdridge 1947 carrying-capacity and suitability laws, so `owed_body` now names only `Caves` and `Earthworks`; the roster runs **14 of 16** from the first tick; `process::field_at` routes nine declared-but-unproduced fields through the roster so `chunk_fields::law_of` has a producer; `App::drive_processes` steps the roster once per tick in the client | `cargo test -p voxel-shapers --lib` → **338 passing, 0 failing, 1 ignored**; `cargo run -p voxel-shapers --example process-tick` → tick 0 **14 of 16** ran; `cargo test -p voxel-stream` → **32 passing**; `cargo check -p shploingus` clean |

**Findings from round 6's passes, each now pinned by a test rather than remembered:**

0. **An edit invalidates the chunks the mesher's *pad* reaches, and only those.** The
   mesher reads one voxel past each face, so a voxel on a chunk boundary belongs to
   the neighbour sharing that face — and to *no* diagonal neighbour, because a face is
   never determined by a corner alone. Pinned by `chunks_touching`'s test (3 chunks
   for a two-face voxel, not 4). A first draft that invalidated the 27-chunk cube
   would have rebuilt eight chunks for every placed block.
0b. **A pick cannot be answered from the mesh.** The mesh holds quads, not occupancy,
   so `Streamer::pick` walks the **world** with the same `Crossings` traversal the
   octree raycast uses — one statement of Amanatides & Woo for both, pinned by
   `the_walk_is_the_walk_the_octree_uses`.
0c. **A measurement of the wrong world is worse than no measurement.** The terrain
   measurement ran on seed **31337** (its module's test world) and was read as the
   shipped seed **2468**, at a 64 m step over a field whose finest octave is 25 m. It
   claimed 36 m of land where the world has water at y 15. The client's own column
   probe caught it; `LEDGER.md` C-46 is the record, and the measurement now prints
   its seed in its first line.

1. `Column::is_outside` used `radius_at` where it needed `cross_section_radius`, which
   made a stalagmite and a spheroid into cylinders. Three call sites were affected
   (`Column`, `Boulder::carve`, `Coral::is_inside`).
2. `Slab` claimed `Level::Detail` unconditionally, so a **grid-aligned** cut claimed
   the addressed level and declared nothing — and the set's own audit refused it.
   `acts_at` is now derived from the cut's own residual.
3. The declared amplitude was multiplied by `SUBVOXEL_SIDE`: a half-**cell** trim
   declared 2.0 where the bound is 0.5 **subvoxels**. One cell is one subvoxel.
4. A clamp-free **tie** matters: at exactly half a cell both edges take the cell the
   edge is *inside*, so a half-cell cut keeps both cells and they shave toward each
   other to meet at the plane. The other convention refuses a cut the geometry
   permits.
5. `ZinggForm::of` compares **strictly**, so a clast at exactly `c/b = 2/3` is
   **bladed**, not prolate — and a bladed clast is refused at construction. The
   boundary is now pinned on both sides (`1.02 → prolate`, `1.0 → bladed`).
6. A **pillar's side face is exactly half insetted**, so `face_recess`'s
   majority rule hits a tie there and recesses the *complement*. Named, not hidden.
7. **One bound was doing two jobs.** `MAX_DISPLACEMENT_SUBVOXELS` (half a subvoxel) is
   about a *trim* crossing a cell boundary; a **profile** is bounded by the circle
   inscribed in its block (half a block, `2.0` subvoxels). Sharing the number made
   `RoundColumn { radius: 0.5 }` — the largest cap a block can hold — **undeclarable**,
   so no pillar could have a domed end. Two bounds, two sources, both by test.

## 8. The materials/fluids grilling, Rounds 18–24 — settled, and the rest as best-guess work

**State.** `docs/HYPERGRILLING.md` Rounds 18–24 hold the questions and the user's answers. The
 grilling is **paused at the user's fatigue** (H-24.3/H-24.7); it resumes on their call. The
 remaining open fluid decisions were **delegated** — *"super tired take your best guess and consider
 them all as todo list items"* — so the items below are **best-guess work items, not user rulings**;
 each is falsifiable and revisable.

**Reorder (2026-10-04, `LEDGER.md` C-152).** The user put the **fluidics first**: doing it gets
the atmosphere working and the world shapers ticking *before* the materials system is complete,
and the materials land into world generation later. So items **1–3 below are the current work**;
the content pass (metals tranche, fluids roster rows, the GT set, the §5/§8 roster) resumes when
fluidics, atmosphere and the ticking world have their first landings. Constraints recorded with
the ruling and binding on the build: **no Minecraft fluids** (no level column, no source blocks;
the pressure/porous model is the one model), **clouds are emergent** (no cloud prop; reference:
`docs/clouds_and_atmospheric_formation.pdf`), **rain is drawn as animated 256-level subvoxels**
(pure function of `(tick, address)`, never stored), and **water is not a mineable block** — the
edit path must refuse fluids (filed defect; drinking was withdrawn).

**Two references added 2026-10-10 (C-184), and the work they owe.**
`docs/fog_mist_foam_and_aerosol_dispersion.pdf` binds the **near-surface** systems, and its first
rule is a separation: fog is condensation (cooling *or* moistening), spray is mechanical breakup, foam
is gas-in-liquid that emits aerosol only through an explicit burst — *"do not generate sea fog solely
from wind speed … they should be separate but coupled processes"*. Its handle is Koschmieder's
`V ≈ 3.912 / β` and its budget is vapor/spray → droplets → extinction → evaporation/deposition.
`docs/earth_biomes_subbiomes_continentalness_mountains.pdf` binds the **shapers** side:
continentalness (§2), mountain barriers and rain shadow (§3), the sub-biome catalog (§15) and the
nine-step assignment ruleset (§16).

Owed, in the order the two guides imply:

**Landed 2026-10-10 (C-185), with evidence.** Items 1, 3 and 4 are built, and 2 is half: the
mechanical pathway exists (`spray_g_m3`, independent of saturation, plus `burst_g_m3` as the only
foam→air coupling) but the *sources* it reads are the sea's own surf, not yet waterfalls or rapids
(that needs `hydrology`'s gradient). The near-surface field is `voxel-shapers::fog` — a layer with a
base and a top, addressed in metres, with the guide's separation enforced by tests (`a_wind_alone_
makes_no_fog`, `spray_needs_no_saturation…`, `foam_is_a_surface_state…`, `the_fog_threshold_is_the_
convention_in_mass_units`), and Koschmieder visibility derived from the droplet loading rather than
from humidity. Continentalness is `climate::continentality`, read through the terrain's own
continental field, with its own measurement of how well it tracks the distance to the sea
(`the_continentalness_proxy_tracks_the_distance_to_the_sea` prints and compares). All four of the
new fields are **declared on the plane** (`Continentalness`, `FogWater`, `SprayWater`, `Visibility`,
plus the cloud's `CloudWater`/`CloudBase`/`CloudTop`), so a cubic chunk reads its own address rather
than a column object.

Still owed from the list below: item 2's non-surf sources (falls, rapids, fountains — the guide's own
§11 catalog), item 5's rain-shadow check against §3, and item 6's catalog and ruleset steps. And on
the cloud side, what C-185 names rather than hides: the client's cloud modules are still per-column
callers, and every address read re-derives its column instead of reading a memo.

1. **Near-surface fog as a field** — *landed* as `fog.rs` (Tier 2). Size-binned droplets (Tier 3)
   are still not there: one declared effective radius carries the optics.
2. **Spray and foam as their own source** — surf is in; **falls, rapids and fountains are owed**.
3. **Visibility from the droplet field, not from humidity** — *landed*: `fog::visibility_m` over
   `extinction_per_m`, declared as the `Visibility` field. §8.3's split (volume scattering vs surface
   droplets vs lens) is the client's, and is not touched.
4. **Continentalness** — *landed* in `climate` and *declared* on the plane.
5. **Rain shadow and elevation belts** — the leeward loss checked against §3, which this tree partly
   has already (`weather`'s Smith & Barstad term, funnel through `habitat`/`column`'s elevation axis).
6. **The sub-biome catalog and the ruleset's missing steps** — §15's recognizable sub-biomes and the
   §16 order's steps 2, 5, 6, 8 and 9 (continentalness, water routing, substrate, disturbance, ecotones).

None of this rules on **what is rendered** — how fog, spray or foam look, and how far up the catalog
the world goes, are the user's (C-184.4).

**Settled (pointer, no restatement):** H-18.1 home + Rust generator, digest-gated · H-18.7 no floats
 ever, ala-cities is the reference · H-18.8 the work order (schema+gate+table → levels → patterns →
 shaper wiring → mods materials) · H-19.2 the spine + gate · H-19.3 the ledger exactly zero ·
 H-19.4 geology + patches · H-19.8 the eight starters are mapping seeds only, never library rows ·
 H-20.3/H-20.4 the whole roster incl. the full acoustic columns (transmission/scattering also NPR) ·
 H-20.6 the full shaper ladder writes state through one declared channel · H-20.7 the full content
 target ([NS] stays empty) · H-21.8 one column set, two consumers, OpenPBR · H-22.2 phases wait for
 thermal · H-22.8 the remesh bar (deterministic projection, local invalidation, stable merge) ·
 H-23.2 fluids are substance rows · H-23.3 molten metals: own species + a Metal reservoir account ·
 H-24.1 porosity + fluid_capacity with the existing refusal · H-24.2 one flow model, pipes are
 carriers · H-24.5 both base levels animate.

**Best-guess work items (delegated):**

1. **Exact/approx tolerance declarations.** Before fluid code: declare a tolerance class for each of
   F1's six approximate families in `conservation.rs`'s vocabulary, so `Mass::from_approximate`
   refusals are meaningful. Exact checks never ride approximate pressure (H-23.6, best guess A).
   — **landed** (`conservation.rs`, C-152.5; the fluidics gate reads it at compile time). The
   atmosphere family's crossing was declared with the rain deposit (**C-154**):
   `WeatherAtmosphereClimate => Crossing(1)` in kg/m²/yr (mm of annual rain) —
   `Mass::from_approximate` rounds to whole kilograms, ≤ half a millimetre of annual rain.
2. **Porous-flow mechanics.** Pressure/cellular flow over `FluidAddress`; `porosity`/`fluid_capacity`
   set what a block holds; the ×4 cadence rung moves it (C-88.4); pipes are carriers of the same
   model. Deliverable: the mechanics note + the first acceptance test on a two-block column.
   — **landed as the six-neighbour cellular automaton** (`fluidics.rs`, C-153: the PDF's recipe,
   exact plans, retention/`K(S)` with hysteresis, sourced properties, 22 tests); the v1 column is
   superseded. **Water in the world landed 2026-10-04 (C-154):** ocean / lakes / karst as pure
   derived state (`fluid_state.rs`), the edit path refuses any edit that targets a fluid (the
   filed defect), rain deposits through the weather family's declared crossing. The client's
   `is_water` asks the material record only — lakes are not swum in yet; the seam is named in
   the code.
3. **A/C behavior blocks.** A stays the storage shape; a fluid whose behavior exceeds columns
   declares a behavior block (H-24.6, best guess A). **Gas is in scope** (the fluidics system also
   handles gas).
4. **Acidity/basicity.** Fluids carry exact content; corrosion/dissolution are declared processes.
   **Research lead:** scour the city builder crates/modules/references for the extra material
   sciences before writing any value (H-24.4).
5. **Porosity sourcing.** Provisional rows are review-gated; weigh declared class-average defaults
   (H-24.3 A leaning, B to consider). No invented porosity, ever.
6. **Water animation.** Both bases animate (64 stored, 256 addressed); pure function of
   `(tick, address)`, never stored. **Check whether animation work already exists** before building
   it (H-24.5 lead; §3.5 says not started). — the fluid **input** now exists:
   `fluidics::Cell::fill_level()` floors the exact saturation to the 256 level (C-153), and the
   **rain fill** exists too (`clouds::rain_fill_at`, C-156: a pure function of the address and
   tick, quantised once onto the same lattice); the animation on the appearance side is still
   unstarted.
7. **Progression.** It requires the materials; the tech tree was scoped but never implemented. Mods
   features were never asked because *"don't copy code"* was read as *"never read the source"* —
   the remedy is **per-feature source reading** (study, never copy), which is also the correction
   for the fake-column-over-real-cubic-chunks misread (H-24.8).
8. **Mod-fluid roster.** Grows from the mod references as rows (Create diesel generators etc.); the
   roster stays open (H-23.8).
9. **Tekkit Classic pass (own section).** The Jaffa Factory set is cloned and pinned —
   `MODS-MATERIALS.md` §6: IC2 (jar route: machine ini tables + lang file), BuildCraft (oil
   distillation roster), Railcraft (coke/creosote/slag; read-only licence), RedPower's successors
   (ProjectRed MIT / BluePower GPLv3 — real gems/rocks/metals vs fabricated electrotine/nikolite/
   teslatite/coloured alloys), Forestry (biomass→bio-ethanol, peat, apiculture — the Sjin's Farm
   family), MFR (rubber/plastic; **no licence file**), EE 3 (magic — deferred to the Deadlock
   model). Jaffa specifics: no `jaffa` hit in any clone — the pack jars are the route. Open passes
   are listed in §6.
10. **Roster mapping rules (2026-10-03).** No minecraft progression — roles follow real properties
   (diamond does real diamond things; BuildCraft's gold/diamond gears are refused as mechanics); the
   older mods are double-covered by their Create-addon descendants, so the union **dedupes to one row
   per material** with both citations kept.
11. **The power-model cross-reference (found 2026-10-03).** The earlier asking is
   `HG-20260925-MODDING-LEDGER-TECH` **0231 Q74** (slot S-01 kinetic power: conversion / captured
   named account / player muscle) + **0223 Q36** (named source accounts per medium: wind, water,
   sun). **Confirmed 2026-10-04 (C-141):** *"we'll use our own cellular model, so a shaft would be
   much more like an actual shaft in a machine, like a contraption but we aren't nearly there yet
   on the progression. Machines and tools in the ala cities city builder was discussed we we can
   apply to actual machine blocks and tools"* — the captured sources and the hand crank survive as
   physical parts/drives of machines; no rotational energy as a power model; the ala-cities
   machine/tool discussion applies to real machine blocks/tools; progression later.
12. **Sweep every reference before Rust (2026-10-03).** *"wait, did you go over all the mods?
   you might want to do that before rust invention"* — complete the per-mod sweep first, including
   references not yet cloned (DynamicTrees, Ecologics, Immersive-Weathering, the crops mods…), with
   each mod's materials verdict recorded (found / nothing / jar). **No Rust for the materials library
   until that sweep is done** (H-18.8 step 1 waits behind it). Status table: `MODS-MATERIALS.md` §7.
   **Complete 2026-10-03** (C-131/C-133): A complete, B resolved, C accounted — full 93-line verdict
   table in `_harvest/stage-c-verdicts.md`; named residuals only (TFMG not read by design, optical
   empty, Ratatouille unread page, six CurseForge jar routes).
   The sweep records **technologies too** — each mod's machines/tools/processes beside the
   substances, each with its **real-world counterpart** (§ item 13).
13. **Progression as a cellular automata (2026-10-03).** *"we'll need the actual progressions system
   so we're not just throwing en masse a huge bunch of materials like 'blast furnace' as the system
   without considering their real world counterparts and tools/machines progression as a cellular
   automata"* — the deliverable is a **technology roster** (machine → real counterpart → inputs →
   outputs → mod source) and a progression model of tools/machines as **cellular automata** (states
   and rules transforming matter), not a flat tier dump. Feeds the bonus goal "progression from   the mods" (`TODO.md` §3.5). Design work after the sweep; recorded now so the sweep captures the
   technology data it will need. **Sequencing (user, 2026-10-03):** the tech roster is the **last
   pass** — after A/B/C are complete — because the Create addons re-reference the older tech trees
   (double-cover applies to technologies too); sweeping techs first would mean re-sweeping them.
   Mid-sweep machine/process values are still captured as sweep evidence.
   **Complete 2026-10-03 (C-134):** the one pass ran — `MODS-MATERIALS.md` §8 re-derived as a
   **deduped rule library (41 rows, family by family)**, each row with its real-world counterpart,
   every declaring mod (old + Create-addon descendant), and a measured example; §8.3 the CA
   progression design (cells → rules → reachability); §8.4 the measured vs [NS] line. H-18.8
   step 1 (Rust materials schema + gate + generated table) is now unblocked and is the next item;
   the CA implementation follows the materials table.
   **Step 1 landed 2026-10-03 (C-135):** `voxel-model::materials` carries the full schema + gate,
   the TSV declaration (2 tiers / 21 substances / 11 processes / kiln vocabulary), the Rust
   generator that refuses to write through a defect, the committed `generated.rs`, and a
   build-time digest gate that fails the build on a stale or hand-edited table (tamper test
   measured). 288 lib + 3 integration tests pass. **Step 5 (the mods' feature materials) now
   has the machinery to flow in as declaration rows**; the settled order is steps 2–4 first, then
   step 5, then the return (C-136.3). **The vertical axis is ruled: z is up (C-138)** — step 4's
   remaining hold is the material-library seam (steps 2–3) alone. **Correction (C-139):**
   family × hue × level is the **icon builder's** scheme, not the voxel game's — it does not wire
   into the substance identity; and these 21/11 TSVs are the stone-rung **slice**, not the content
   target — the full set (H-19.6, H-20.7) is the body of the pass, starting with the per-reference
   sweep read-back and the extraction into rows. **Sampled appearance landed 2026-10-04
   (C-143):** whole USGS v7 spectra retained, exact visible-window derivation, digest-gated
   (`crates/voxel-model/materials/spectra/`); six of the seven placeholder rows now carry
   sampled reflectance, ICE held pending a real ice spectrum (the v7 AREF has no visible
   channels).
   **Corpus widened 2026-10-09 (C-172):** the census's work list is closed — 63 rows admitted
   (56 measured samples + 2 channel grids pinned, digest-gated), the 20 rocks / 24 ores / 9 gems
   / 15 made goods composed by C-141's rule (`blend_reflectance`, an exact rational mass-weighted
   blend), `hatchet`'s `Count` density composed to 2030 kg/m³, and 5 substances **refused by
   name** (`bismuthinite`, `tetrahedrite`, `cryolite`, `diamond`, `flue_gas`). The census is
   green: 65 admitted / 5 stated absences / 0 debt. Two pre-existing `voxel-client` sensors
   remain red and are named, not repaired (rim arithmetic on `water`/`ice`/`plant_fibre`/
   `fibre_dust`; five OpenPBR terms unpacked into `MaterialParams`).
   **The corpus reaches the world 2026-10-09 (C-173):** the five generic classes became
   substances — `corpus::rock_of` picks a declared rock per class per 220 m region and
   `corpus::rock_voxel` draws a vein's ore else its host rock, both called from the two
   material paths so they cannot disagree (equivalence test >30k voxels still green).
   Witness `examples/rock-census`: 24 substances drawn — the rock kinds plus all four
   drawable ores (coal, iron_ore, malachite, cassiterite). Drawing the deposit survey 1:1 made **90% of the subsurface
   coal**, so ores are drawn only above `ORE_DRAW_CUTOFF` (0.9, a stated choice). Still
   undrawn by name: gold (no declared row) and gems + made goods (no occurrence or
   placement seam). This is the materials→rendering wiring C-142 named as the next domino.
   **Shapers audited 2026-10-09 (C-174):** caves wired but inert — `cave_threshold` 0.62
   exceeds four of five rock classes' dissolution ceilings (bedrock 0.319 / igneous 0.347 /
   metamorphic 0.376 / sedimentary 0.622), and the **global** water table (sea+2 m) makes
   open caves 0.9 % of vadoze rock vs 22.5 % at the shaper's own 0.40. Spawn stands 2.17 m
   **below sea** (world range 2.25–57.99 m over 10 km, land 45.6 %). **Erosion is absent
   from the terrain** (`erode_patch` has no world caller; droplet dial 0), mountains are
   gated on `Convergent` boundaries measured at 5 of 5,168 samples, and `delta`,
   `transition`, `process` have **no caller anywhere**. Named, not repaired — each is the
   user's to rule on. Evidence: `examples/cave-census` (new), `examples/relief-profile`.
   **The tree declared 2026-10-23:** the technology tree is rows, not prose — **8 tiers,
   146 substances, 79 processes, 32 vocabulary entries**, every process carrying a mandatory
   `source` (the harvested artifact, the §8.1 roster row, or an explicit `declared:` where the
   harvest is silent). `materials::tree` owns reachability: the walk fires processes tier by tier
   from what the world holds and refuses an unreachable tier, an unproduced `made` substance, a
   dead process and an undeclared name; the generator refuses to write through any of them, and
   `the_committed_tree_is_complete_and_reachable` fails loudly on an orphaned node. The **state
   axis** (`Solid | Liquid | Molten | Solution | Gas`) landed with it: a melt or a liquor names the
   solid it is derived from in `phase_of`, and the gate refuses a molten row nothing melts.
   Coverage: **tiers 8/8, substances 146/146 (80 made, 80 produced), processes 79/79**. Not built,
   by the mission's own boundary: inventory, recipe lookup, craft action, and any wiring into the
   appearance registry — the tree is data and its owner, not gameplay. The agricultural families
   (husbandry, composting, dyeing) landed after the state axis, and solvent extraction followed on
   ethanol — the one extractant the harvest names and the tree already makes — taking it to 146 rows
   with extraction rowed in both halves; three of §8.1's 41 rules are still unrowed, named in
   `MODS-MATERIALS.md` §8.5.

14. **The return path (user, 2026-10-03).** *"the whole reason why we're doing this huge ass
   materials pass is because we were on a tangent fixing the shell, which turned into repairing
   the cubic chunks, which turned into an open question about materials themselves, so when the
   materials and processes are done, we have to repair the actual materials rendering+chunk
   system (I've been making you record progress in the ledger)"* — recorded: the chain is
   **shell → cubic chunks → materials → materials+processes pass → back to materials rendering
   + chunk system**. The pass is what the destination waited on; the standing pass/fail for the
   return is the original chunk mandate, verbatim: *"if the chunks keep the incorrect columns
   shit then you will have failed."* Ledger: C-136. **The dominoes (user, 2026-10-04):**
   *"once the materials loop closes it's going to be wiring the materials to the rendering which
   will start knocking shit down domino style, this will help us with fixing the chunks being
   column based which will in turn bring us back to the atmosphere/fluidics system"* — recorded:
   **materials loop → materials→rendering wiring → the column-based chunk knock-down →
   atmosphere/fluidics** (LEDGER C-142).

**Research leads, in order:** city builder material sciences → existing animation work → per-feature
 mod source reading (incl. the Tekkit §6 passes). The fluid rounds resume after that if the user
 wants them.

15. **Visible-ground pass (C-159.10 / C-159.11 / C-160, 2026-10-04).** The user's ordered sequence for
   things he can *see*: *"biome mateirals, then biomes, then sub biomes, then world shapers
   ticking, then trees"*. Delivered: the biome's own sampled ground (five surface covers, USGS
   Chapter V + `sand_dwo3`) in **both** material paths, and **trees** (`flora`, an 8 m lattice,
   deterministic and tick-free, trunk `WOOD` + conifer/leaf canopies). The Holdridge read that
   makes both affordable was split into a column part and a season scalar: **202 µs → 5.6 µs
   per column**. Evidence: `examples/surface-census.rs` (spawn window map — every letter used to
   be `r`), `examples/flora-census.rs` (counts + a slice through a standing conifer),
   `examples/surface-colours.rs`, `examples/biome-cost.rs`; `shploingus.exe` rebuilt 21:22.
   **Sub-biome layer landed (C-160, 2026-10-04):** `crates/voxel-shapers/src/habitat.rs` — five
   terrain expressions (steep / montane / wet / dry / lowland) composed into the cover in **both**
   material paths (`materials::surface_of`) and gating flora (steep: no tree; dry ×0.35; montane
   ×0.7 and conifer). Evidence: `examples/habitat-census.rs` (all five classes present — world
   `{lowland 1176, wet 335, dry 85, montane 59, steep 26}`), re-run `surface-census` (wet
   repaints scrub ground → `grass`; dry takes `leaf` → `dry grass`), `flora-census`,
   `tick-over-world` (0/5445); `shploingus.exe` rebuilt 22:42. Doors still owed inside this
   layer: aspect (`Radiation`) and drainage (`Groundwater`). **Still owed on this sequence:** the
   regional biome belts (the belt's wavelength is still `continent_frequency` ≈ 400 m), MAP
   calibration (still reaches 29 510 mm), the scheduler driving the ticked world (`voxel-work::tick`
   has no production consumer), then species/regions → Gaia (gap analysis in LEDGER C-159.9).

   **Materials rendering: detail, tiling and rate-fighting (C-162, 2026-10-05).** The user's
   eye-check on the first pattern build, in his words: *"the materials rendering is a little buggy
   still and the tiling is very noticeable"* → *"the 256 base is shuffling around the materials and
   z-fighting the 64x base, and doesn't seem to respect the square like nature of the blocks, and
   the trees are oddly the same look as the ground"* → the ruling: *"white noise at the 256 base is
   acceptable with different shaper patterns for the noise, so for sand it'd be speckled, for trees
   it'd be bark, metal could be brushed/shiny."* Four corrections, all in the model and its WGSL
   transcription (`crates/voxel-client/src/pattern.rs`, `shading.wgsl`, `world.rs`): the address is
   the **world** subvoxel (`floor(p / 0.25 m)`) and not the block-local index (a block-local address
   stamps one pattern into every block, which *is* the visible tiling); the eye's world place
   travels as `eye_phase` + `eye_voxel` so an eye-relative fragment can still address the world
   (the earlier uniform write emitted three `i32` for a `vec4<i32>` field — every later field read
   four bytes early, `rates` fell to zero, and the 256 level resolved everywhere at full gain); the
   64 tone and the 256 pattern are now **one field read at two scales** with the tone capped at
   `TONE_SHARE`, never two full-amplitude fields averaged (the z-fighting); and every material
   **shapes** the white noise (`materials::SurfacePattern` — speckle / foliage / bark / brushed,
   a stencil on the address and a curve on the value). Held to the crate by
   `renderer::tests::the_pattern_is_drawn_where_the_greedy_mesh_has_no_geometry` across four GPU
   stages — far 2 m, near 0.15 m, **eye moved** `[3.25, -7.5, 12.125]`, **bark shaper** — worst
   disagreement 1.06e-5 over 266 144 pixels a stage (bound 5e-3), 3 226 / 199 427 / 205 252 /
   105 395 distinct reds; `shploingus.exe` rebuilt 2026-10-05.
   **The fluid door, as the user framed it (2026-10-05):** *"animating water at the 256 base and
   having actual water cells interpolating at the 256 base so molten metals and fluids/gas can
   animate addressed voxels at the 256 base while also still allowing for the non-addressed
   materials properties at 256x base — it might be easier to frame it similar to minecraft water
   shaders: it allows for sea foam on the material surface, caustics through the materials, and
   when we need the material to break out of the block, it looks good."* Recorded as the next
   door: the 256 level is a **live addressed domain**, a fluid's cells interpolate between
   addresses as a pure function of `(tick, address)` (C-91.3), and the solid read stays a static
   pure function — `pattern.rs` now says so in the module doc, `shading.wgsl` names it in the
   pattern section.

   **The base, the ladder and the eye's own footprint (C-163, 2026-10-05).** The 256 base is
   **16 per subvoxel axis** (`DETAIL_SIDE = 16`, a 15.6 mm cell; a subvoxel face carries
   16×16 = 256), corrected by the user from the earlier 256-per-axis reading; fluids keep their
   own resolution (`FILL_SIDE = 64` levels per block axis — *"we definitley don't need fluidics
   to be the 256 actual base"*), and the fluid scale is the one he named (*"as small as
   individual 32x pixels"*). The pattern is one field read in octaves (15.6 mm · 2^j → 4 m),
   fBm-weighted and normalised, each rung smoothstepped to zero at four pixels a cell; the
   placeholder 64-level tone is retired. **The fade stays; its input changed:** `fwidth` (a
   per-2×2-quad estimate — the boil on angled faces, and the quad diagonal painted as a visible
   line) is replaced by `eye_footprint` — distance × one pixel's angle ÷ cos(face) — with the
   pixel angle uploaded in `View.screen` (128 → 144 bytes). GPU agreement worst **2.0e-7** over
   260 100 pixels a stage (bound 5e-3); silence past the ceiling is an equality. The numbers
   ledger's dense rows moved (4,096 per subvoxel, 262,144 per voxel). **Still owed on this row:**
   the library's own base patterns (`MATERIALS.md` 4.9/4.10); the **NPR half of the
   OpenPBR/NPR equation** (the user's channel list: albedo, tint mask, tint colour, scrolling
   textures/UVs/normals, emissive colours/masks/strength, metallic and roughness maps, detail
   masks, NPR transmissives, subsurface scattering, rim lighting, NPR outlines — Deadlock /
   Source-2's shader, the alacripose-landing example); **per-axis (anisotropic) rung selection**
   for faces seen at grazing incidence (the eye's *"the pixels merge... horizontally"*); the
   sky's rain-shadow step (turbidity sampled at the player's column through
   `precipitation_parts`'s upslope branch — diagnosed, the repair left to the user); and the
   black-screen report (the world stops drawing, the shell remains — needs an instrument before
   a cause).

   **The face's own plane in the address (C-163.1, 2026-10-21).** A rectangle's plane is a whole
   number of subvoxels, so on a face the address's own axis *is* the grid and the crate's
   `floor(p / 0.25)` names the cell on the plane's `+` side. The fragment rebuilt that coordinate
   from the eye's two halves in `f32`, and for some eyes it landed a rounding step **below** the
   plane; `floor` then drew the whole face from the cell *beside* the face, and which side an eye
   got depended on where it stood — the pattern on a wall changing as the camera moved, which is
   the shimmer the eye reads as z-fighting. Measured on a 5 m² face of the shipped world at 2 m:
   the drawn colour was the beside cell's **0.2544** against its own **0.2446**, and over a walk
   of sixteen eyes the worst was **5.02e-3** of a channel. **The repair is the record's own
   plane**: `vs_quad` states the face's axis and its world subvoxel cell (exact integers, from
   `slice` and the recovered chunk origin) as a flat varying, and `pattern_deviation` takes that
   axis from it instead of from the interpolated position. Worst disagreement over the same walk
   is now **1.13e-7**; `renderer::tests::a_face_on_a_cell_boundary_draws_its_own_cell_from_every_eye`
   is the guard (5.02e-3 before, 1.13e-7 after), `pattern-flicker`'s 0.05 m totals moved
   339.1 → **332.6** (vertical) and 144.5 → **143.1** (horizontal) with the same worst, and both
   `shploingus.exe` and `surface-format-check` draw through the new varying with no validation
   error. **A corner states no plane** — a refined rectangle's corners are off the plane its
   normal names — so the vertex format keeps the interpolated address; the window draws that path
   whenever a light field is bound. `examples/face-pattern.rs` is the instrument (it writes the
   three faces as PNGs and prints the drawn step spacing per in-plane axis). **Still owed,
   unchanged by this row:** the **per-axis (anisotropic) rung selection** above. Measured
   2026-10-21 on one wall of the shipped world at 2.34 m: the footprint is **5.0 mm** at 30° off
   its normal and **20.9 mm** at 78°, because it is one scalar — `distance × pixel angle ÷ cos` —
   and so coarsens *both* in-plane axes by the same `1/cos`. A face the eye skims therefore loses
   detail across its width as well as along its length, which is the eye's *"the pixels merge...
   horizontally"*. The address law itself is square and eye-independent at that point (cell
   `[192, 58, 245]`, address `(0, 8, 8)` from four eyes: 1.75 m, 2.0 m, 2.25 m, and 0.37 m
   slid in the plane), and no cell of the eighteen-chunk region is covered twice (**0** of
   5 622 quads, **0** planes with two faces), so the grazing filter is the whole of what remains
   here.

   **The grain's basis is the face's, not the world's (C-163.2, 2026-10-21).** The report above
   says the cells are right-sized *"except in certain cases on vertical sides of blocks where they
   stop being squares"*; the address and the shading were both measured square, and the cause was
   one step further out: `SurfacePattern::stencil` — the shaper that turns white noise into bark
   and brushed stone — stretched its axis **by world name** (`Bark` shifted `z`, `Brushed` shifted
   `x`). A stretch keyed to a world axis is swallowed on the face that looks along it, `x` being
   constant across an x-facing wall, so the same material drew square cells on that side and
   four-to-one cells on every other side of the block; a trunk's bark showed grain on two of its
   four faces and none on the other two. **Measured on the stencil at the floor rung**, before:
   bark on a z-facing wall **35.7 mm × 35.7 mm** (square) against **35.7 mm × 250 mm** on an
   x-facing one; brushed the mirror image. The fix is in the two files that own the stencil:
   `SurfacePattern::stretch_axis(plane)` states the axis **in the face's own basis** — bark runs up
   (`y`) on every vertical face and a log's end face shows rings, brushed runs horizontally (`x` on
   a z-facing wall, `z` on an x-facing one) — and `pattern_stencil` in `shading.wgsl` dispatches on
   the same table with the flat `plane` varying the fragment already had. After: every vertical
   face of a directional material is **250 mm × 35.7 mm**, the grain is the same direction on both
   walls of a corner, and the frame is held to the crate at **2.5e-6** median on a 33² grid of the
   bark wall. Guards: `pattern::tests::every_vertical_face_of_a_directional_material_shows_its_grain`
   (it fails on the world-axis law — bark 8 against 8 on a z-facing wall — and passes on this one)
   and the bark stage of `renderer::tests::the_pattern_is_drawn_where_the_greedy_mesh_has_no_geometry`
   (**1.2e-8** worst against the crate, previously 8.9e-8 on the coarse rungs). The instrument is
   `examples/face-square.rs`: one wall and one floor of one material at **one footprint — 5.813 mm
   on both, 2.236 m away at 26.6°** — in one held frame, plus two bark walls at right angles, and
   it prints the drawn cell per in-plane axis, the frame-against-`shade_cpu` median per face
   (**2.2e-5** on the wall, **3.5e-5** on the floor of the speckle pair — the shading path is
   face-blind), and writes `target/face-corner.png` and `target/face-bark.png`.

   **And the refined-corner path states the face too (C-163.2, same day).** The paragraph above left
   one path on the world axis: a refined triangle carries no record, so `vs_main` had no plane to
   state and the fragment fell back to the material's world axis. The repair is the vertex format's
   own reserved slot — the second of the two `f32` the retired lattice sample held, now `plane_axis`
   (`-1` where the geometry has no face), written by `Scene::build_with_light` from the quad's face
   and read by `vs_main` — so the stride stays **64 bytes** and the eight attributes keep their
   offsets. **Measured on the shipped surface**, `examples/face-refined.rs`: a generated world
   (`WorldSource`, 49 chunks, 17 693 quads) with a **displacement law declared for every material**
   (502 bent quads, 0 records — the whole frame on the vertex path, 19 720 corners), and a player's
   eye 2.0 m off each face, 1.70 m above its foot, looking level. Two runs of one binary differ only
   in the `stretch_axis` table — the pre-fix world axes against the face-relative one:

   | face | before (world axis) | after (the face) | frame changed |
   |---|---|---|---|
   | trunk side, z-facing | 35.7 × 35.7 mm | 35.7 × 250 mm | 33.4 % of 1 681 points > 4e-3 |
   | trunk side, x-facing | 250 × 35.7 mm | 35.7 × 250 mm | 34.3 % |
   | dolomite wall, x-facing | 35.7 × 35.7 mm | 250 × 35.7 mm | 78.4 % |
   | dolomite wall, z-facing (control) | 250 × 35.7 mm | 250 × 35.7 mm | 0.0 % |

   The fourth row is the control and the point of the measurement: a face whose world axis is
   already in-plane is untouched, so what changed is the law and not the build. The frames are held
   to the crate on a 41² grid of each face: **8.1e-8** and **9.3e-8** median on the two trunk sides,
   **4.5e-7** on the dolomite wall — and on that wall the same frame is **4.7e-2** away from the
   corner fallback, which is what says the corner stage received the plane rather than falling back.
   Guard: `scene::tests::a_refined_corner_states_the_face_it_was_cut_from` (every corner of a bent
   quad states its quad's axis; a screen-space probe states `-1`). Artifacts:
   `target/face-refined-compare.png` and `target/face-refined-{bark,brushed}-{posz,posx}-{before,after}.png`.
   **Still owed:** the per-axis rung selection above; and `materials::displacement_field` is still
   empty, so the shipped world never bends a quad today and this path is reached only by a world
   that declares relief.

---

## 9. The open work the patches named — kept here so it is not lost

**Why this section exists.** PATCH-017 … PATCH-038 each end with a `# Not done, and named` list,
and those lists are the only place several real components of the system are written down. A list
that only lives at the foot of a patch is a list nobody reads again — and a follow-up offered in
conversation is a clickable box and not a record: the turn ends and it is gone, so an option offered
and then forgotten is a component that was never going to be built. Every item those patches named
is restated here with its source and its **current** state, and this section is appended to whenever
a new named-but-unbuilt piece appears — by a later patch, or in the conversation.

Nothing here is a promise to build it. Each row says what is missing and what blocks it, in the same
shape as §3.

| # | The component, named | Named by | State, honestly |
|---|---|---|---|
| 1 | **A judgment reaches the gate** — the decision layer does not consult a ballot | PATCH-023 | **open.** `laya`'s instrument is still `oracle::lint` and nothing in `evaluate_project` reads a ballot, so a declared question is answerable by the example and by no gate |
| 2 | **`max-age` is enforced** — the staleness window | PATCH-023 | **open, and blocked on a field.** `laya::ProofOfWork` carries neither `tick_span` nor `world_digest`, so a ballot's `max-age` is declared and never checked |
| 3 | **An answer cache keyed by state digest** | PATCH-023 | **open.** Every judgment pays its own ask; nothing caches, and upstream measured the same recompute |
| 4 | **The classifier's own process** | PATCH-023 | **open, a knowingly-taken trade.** The von session is lazy but lives in the MCP process: a judging client pays ~22 s and holds 757 MiB for the process's life. The seam (`voxel_package::ballot`) already supports a split |
| 5 | **Something consumes `Recipe::ballots`** | this round | **open.** The recipe can now carry its questions (PATCH-023 §5), but the classifier still reads a sibling `.ballot` file, so the carrier has no live reader |
| 6 | **The build door is fatal** | PATCH-021, PATCH-022 | **closed this round, in both directions.** `crates/shploingus/build.rs` turns the census it already took into a verdict (via `conformance::of`, so the tree is walked once) and **refuses the build** when anything is rejected or unchecked, naming each refusal and the list to change. Verified by running it: with no acceptance list the gate exits **101** naming all twelve; with `conformance/accepted-refusals.txt` declared it exits **0** and the report still prints every accepted refusal by name (`conformance-refused 12`, `conformance-accepted 12`, `conformance-rejected 0`). An unchecked cell cannot be accepted, so *no instrument ran* is never a pass |
| 17 | **The sixteen crates get documentation of their own, in their own rustdoc** | this round, completing PATCH-022, on the user's ruling | **closed 2026-10-10, and the ruling changed the shape of the answer.** The user's words: *"I would prefer the actual documentation live inside rust docs. So when we study why some bullshit value is the way it is we know exactly what happened."* So the axis no longer asks for a `docs/` heading: the documentation moved into each crate's own **rustdoc** — the `//!` block and the `///` comments on public items, which `cargo doc` renders as that crate's pages and an editor shows on hover. **Measured, all sixteen:** the census table read `documented active` for every crate where all sixteen read `refused` before, and the `documented` gap lines were gone. **Amended the same day, when that `active` was audited: it was a header being read, not documentation being measured.** The `//!` block answered *has this crate made a claim*, and every crate had one — so the axis could only have turned false if a line at the top of a file were deleted, and the sixteen blocks had been written out of the prose the comment headers already carried. `census::Axis::Documented` now reads the compiler's own `missing_docs` count (`conformance/missing-docs.txt`: **2285 undocumented public items across sixteen crates, three crates at 0, `voxel-model` alone at 907**), so the axis is `active` where the count is 0, `inactive` naming the count elsewhere, and `refused` for a crate the reading does not name. The retrofit is unchanged and its next rung is real: writing the rustdoc beside each item lowers the number. Each crate's entry file now carries an audit section written from its own claims rather than from a summary — the game reading the gate (`voxel-package`), the stage ladder with no drivetrain (`voxel-shapers`), the band measurement spanning 1.5% (`voxel-client`), a band table read for one constant (`voxel-light`), two crates read by nobody (`voxel-audio`, `voxel-instrument`), the seam that ends at a tool server nothing calls (`voxel-von` → `voxel-mcp`), and one test behind the whole frame loop (`shploingus`). **What the axis can still refuse**, which is why it still has bite: a crate that is in no measurement at all — a new workspace member arriving with nothing said about itself, or a crate dropped from `conformance/missing-docs.txt`. Three consequences were followed rather than left standing: **rustdoc is measured apart from commentary** (`bloat::Prose::doc_bytes` and `plain_percent`), so writing documentation no longer costs a crate its prose budget, and the ratchet was re-declared over the plain share (row 35); `conformance/accepted-refusals.txt` was **emptied**, because its sixteen keys answered no refusal once the documentation existed; and the gate now **refuses a declared key that no refusal answers**, printed as `conformance-stale-acceptance` beside the key to delete — a key answering nothing is a **standing permission** that re-admits the refusal it names the moment the axis regresses, and it prints nothing while it waits, because `accepted` is computed by asking which refusals the keys match. The markdown draft of this answer (`docs/CRATES.md`) was **deleted rather than kept beside the crates**: its content is in the sixteen `//!` blocks now, and a copy nothing keeps in step is the failure this tree convicts most often. **Amended once more the same day, when the reading itself was audited: the count is a snapshot the gate cannot take from inside its own build script, so each line of `conformance/missing-docs.txt` now carries a pin — the `sha256` of that crate's covered files — and a pin that no longer matches is the axis's own state, `stale`, reported by the gate (`conformance-stale`) and never fatal, because the repair is re-running the recipe rather than an edit.** So the axis has four answers now: `active` at 0 with a matching pin, `inactive` naming the count, `stale` when the bytes moved under the reading, and `refused` for a crate the reading does not name — and the one it used to give silently, *whatever was last measured*, is gone. |
| 7 | **The twelve rejected events are filed on the chain** | PATCH-022 | **open, blocked on a ruling.** `filing` refuses a second filing by design; the segmenting rule is the user's, and a rejection currently lives for one call |
| 8 | **The MCP subset is pinned against the bridge** | PATCH-022 | **open.** `bridge::TOOLS` carries 23 rows and the server serves 8; no test asserts which 8 |
| 9 | **`get_census` / `get_work_list` are not re-deriving the tree per call** | PATCH-021, PATCH-022 | **open, cost unmeasured.** They walk 805 files and the corpus on every call |
| 10 | **A real client calls the surface** | PATCH-021 | **open.** The MCP surface is verified by transcript; nothing in the shell, an editor or CI calls it |
| 11 | **`status` is verified, not trusted** | PATCH-018 | **open.** A record that says `applied` is counted as applied; checking it is a diff against the pinned digest |
| 12 | **A patch is an event on the chain** | PATCH-018 | **open.** A record is read from a file and reported, not filed into `chains/workspace.chain`, so it cannot be cited by digest |
| 13 | **The bloat stage asks a question** | PATCH-017 | **open, and it is the user's call.** No judge call is made; the stage is a measurement, not an audit with a verdict |
| 14 | **The proposed split is measured** | PATCH-017 | **open, blocked on the user.** No von run has happened because no split has been requested |
| 15 | **Rendering has its own audit** | PATCH-017 | **open.** The stage measures test mass and prose mass; a rendering audit needs its own measurement (`voxel-appearance::response`, the client's parity tests) |
| 16 | **The record reads one line-ending form once** — PATCH-019's **candidate 2** | PATCH-019 | **open, a decision deliberately not taken.** The verbatim repair took candidate 1, folding at the comparison, so the parsed body and every digest of it are untouched. Folding once at the single read is the wider reading, it would change every document digest, and `record::DOCUMENTS` would have to agree with it — so it stays owed, and PATCH-019 says so in its own words |
| 18 | **The gate's own numbers are retained as evidence** | PATCH-020 | **closure, not open.** PATCH-020 named that the gate had not been re-run, so its census and bloat numbers were a previous build's. The gate has since been re-run green several times, but its report is written to `target/` — which is gitignored and now trimmed — so the numbers are **read at the time and not kept**. If a reading is meant to outlive the build, it belongs in `logs/` with a date like the other evidence |
| 19 | **The four `unchecked` feature rows, re-derived** | PATCH-020 | **open.** Those rows are quoted from an earlier reading rather than measured again |
| 20 | **The 639 projection disagreements, listed** | PATCH-020 | **open.** They are counted, not enumerated; where each fact lives in the chain is a separate read |
| 21 | **`logs/` is 413 MB and was not cleaned** | PATCH-020 | **open, and it is a decision about retained evidence.** Plus the untracked `logs/test-workspaces/notifications-tampered-*/` debris directories. Removing either is a choice about what evidence survives, so it is named rather than done |
| 22 | **The build tree grows until the volume is full** | PATCH-020 | **closed by row 23, and it had already bitten.** `target/` had grown to **6.9 GB** and `C:` was down to **232 MB free — 100 % full**, which is what made that round's builds fail. It was trimmed to **1.9 GB** by hand, removing the regenerable parts (`target/debug/examples` 2.5 GB, `debug/incremental.kache-auto` 2.3 GB, the `.pdb` debug symbols ~0.3 GB) and keeping `debug/deps` (489 MB) and `target/release` (1.0 GB), both reusable. **The hand trim it asked for is now a guard in the build**, so the row closes with the work landing rather than with an instruction to whoever reads this next. One claim in it was wrong and is corrected in row 24: a same-volume cache hit here does **not** restore as a hardlink |
| 23 | **The build tree's size guard** | this round | **built and verified, and every number in it is measured.** `voxel_package::space` measures the tree, reads the volume, trims four regenerable parts cheapest-first to a byte budget, and reports every removal and every kept artefact with its reason; `crates/shploingus/build.rs` runs it **before it compiles anything**, prints its section, and refuses only below its 1 GiB minimum (8 GiB floor, 4 GiB tree budget — the derivations are in the module doc). Verified: **21 unit tests** on the refusal set, **312** across the crate; on the real tree `cargo space --floor 20` fired, took `doc` + `release/examples` + `debug/incremental.kache-auto` + ten `deps/*.pdb` files and measured **`space-freed 369692581` — 352 MiB** with `shortfall 9415733248` and exit **0**; the gate is **exit 0** with the section in `alpha-shell.gate`. The `symbols` cost was measured twice rather than assumed (a 21 MB `.pdb` deleted, `cargo build` answering *Finished 0.33s* debug and *0.84s* release, no relink). **Two limits are named in PATCH-024 rather than hidden**: it runs when the gate runs, so a **cold** tree can still die before the first build script exists, and there is deliberately no off switch |
| 24 | **The cache doubles its own bytes** | this round, from kache's own advice line | **recorded, and the switch deliberately not taken.** `.cargo/config.toml` claimed a same-volume cache hit *"costs zero extra bytes and no copy at all"*; kache reports the opposite here — *"cache hits are restored by COPY … roughly doubling disk for cached content"* — and names `[cache] windows_hardlink = true` as the dedup switch. It is **not** set, and row 23 is why: kache warns that mode is only safe *"if your build never deletes or rewrites an object output"*, and the size guard deletes object outputs by design. So the 9.6 GiB store and the tree each cost their own bytes, and the guard is what keeps the second from filling the volume. **The repair is worth doing later** — set the switch and measure — but only if the guard's trimming is ever proven never to rewrite an output in place |
| 25 | **The census measures twelve parts and the workspace has sixteen members** | PATCH-025, PATCH-035, PATCH-036 | **closed, and the repair was four rows and a count — plus the check that had been missing.** `CRATE_COUNT` is 16, `CRATES`/`ROWS`/`Crate` carry `voxel-light`, `voxel-audio`, `voxel-von` and `voxel-mcp` in dependency order, and `decision`'s subject vocabulary grew on its own (96 axis subjects, `SUBJECT_COUNT` 152) because it is derived from the crate table rather than listed again. **The count alone was the bug**, so `census::check_against_manifest` now reads the workspace's own `members` array and **refuses by name** on either disagreement — a member with no row (measured by nothing) or a row naming a crate the workspace does not hold — and the gate runs it on every build. **Measured on the real tree:** the receipt carries `crates 16` beside the new `crates-in-manifest 16`, `conformance-tested 16`, and **sixteen** `crate` rows where there were twelve; the door, which is fatal, refused on the first run with four undeclared refusals (`voxel-light`, `voxel-audio`, `voxel-von`, `voxel-mcp`, all on `documented`) and passed once the four keys were declared — **the new-crate path working exactly as row 6 said it should.** `cargo test -p voxel-package --lib`: 334 passed, 0 failed. Two of the four were also caught by name: `voxel-light` is the crate whose `SUN_DILUTION` the client's sun reads |
| 26 | **Sixteen records name no path in this tree** | PATCH-025 | **open, and it is a ruling, not a repair.** `patch-unnamed-records 16` prints **PATCH-001 … PATCH-016** on every build: they target `hyperskill/*` and name no file under `crates/`. The coverage join now makes that visible rather than silent, but whether records for another system are *owed here*, *historical context*, or *belong in a directory of their own* is a decision the record has not taken. Until it is taken, eleven of them also count as `patch-set-owed`, so the tree's own reading is that this project has eleven unfinished patches when it has none |
| 27 | **A shaper's named source is not checked** | PATCH-029 | **open, and it is the axis that crate cannot fail without.** `voxel-shapers`' own header claims every one of its 37 modules *"is developed from a named source — a paper, or a ruling recorded in this project"*, and nothing mechanical reads that claim: a module whose comment cites a ruling and a module whose comment cites nothing are the same bytes to the gate. The check is a per-module citation row, and it is the difference between a documented rule and an enforced one |
| 28 | **69 183 lines are in no record — 55 735 code, 1 860 shader, 11 588 prose** | PATCH-037 | **open, and it is now a work list rather than a worry.** `patch::placement` prints every unplaced file biggest-first, with its lines, on every build: the rendering block first (`shell/mount.rs` 6 696 → `renderer.rs` 2 860 → `motion.rs` 1 676 → `skylight.rs` 1 663 → `shell/window.rs` 1 622 → … → `shading.rs` 1 114), **all six `.wgsl` shaders (1 860 lines, placed by nothing at all)**, then the game's own `main.rs` 2 425, the world's `shaper.rs` 1 861 and `civilian/` (1 823 + 1 576), then 23 documents. The order, the template and the acceptance shape are in [PLAN-2026-10-10](docs/PLAN-2026-10-10-clean-room-and-the-api-layer.md); what blocks it is only that each file needs a record that says what it is *for*, not that it is mentioned. **Placement is not understanding** — the report says so in its own line |
| 29 | **The clean room, and the api layer every old format becomes** | the user, 2026-10-10, written down as PATCH-037 | **open, and it is a program with an order rather than a patch.** The user's sentence: *"consume all of the old format shit into api layers and have all the code go through the axis laya von module system so we can regenerate a real clean room version of the game that has its documentation and moddable api layer for players and agents."* Four lanes and their blockers are written in [PLAN-2026-10-10](docs/PLAN-2026-10-10-clean-room-and-the-api-layer.md): **A** placement (rendering, then the model's vocabulary, then `main.rs`, then the documents); **B** the old formats into declared api rows (the sixteen `hyperskill/*` records, the 526 mean-246-byte packages, `SHELL-PORT.md`, `MODS-MATERIALS.md`) — **blocked on row 26's ruling**; **C** every module declared as a subject and judged through laya's seam and von's classifier — **blocked on a per-module subject vocabulary, the largest design decision left**; **D** the clean-room regeneration — blocked on A–C, because regenerating from a 40 %-covered record reproduces the holes at machine speed |
| 30 | **A parallel build of this tree does not fit the free space on this machine** | PATCH-038, and row 24's un-taken switch | **open, and measured on a real build.** `cargo test -p voxel-package` at the default parallelism failed with `can't find crate for std` / `only metadata stub found for rlib dependency hashbrown` / `internal compiler error: no resolution for an import` while `C:` was at **99 % full with 7.7 GiB free** and the kache store restores hits by **copy**; `cargo space --floor 20` freed 933 MiB and the same suites then ran green at `-j 2`. Those errors are **eviction, not code** — but they are indistinguishable from a broken tree to anyone reading the log, which is the real cost. The repair is row 24's `windows_hardlink` switch (measure it first, it rewrites outputs) or a smaller tree; until then, a full-suite run on this machine means `-j 2` and a freed volume, and PATCH-038 says so rather than letting the next reader rediscover it |
| 31 | **The corpus beyond this tree: 54 ids, all of them here** | PATCH-039 | **measured, and it answers row 26.** `cargo ingest` walked twelve declared roots across four drives in 35 s and read **0.151 %** of 13.1 GB. **54 distinct record ids exist in the whole corpus and this tree carries all of them** — its own 38 plus the sixteen `hyperskill/*` records, which every later generation also carries (`shploingus` 4 vendored copies, `shploingus-game` 6, `planet-shploingus` 2, the prototype 1, `AWA3D` 2). **`ingest-only-there` is 0 for every root**: there is no unrecorded decision sitting on another drive. So the open question moved from *where did these come from* to *why one id is vendored sixteen times* — and the user ruled it on 2026-10-10: the copies are **superseded generations of this same tree, neither needed nor merged**. One canonical home per decision, and the sixteen-fold duplication is **deleted as the canon settles** rather than reconciled |
| 32 | **The corpus's unrecorded material: 296 documents and ~19 000 source files** | PATCH-039 | **open, and it is the real shape of "consume the old format shit".** The records are already all here; what is not here is prose and source. The 47 vendored mod repositories alone hold **2 979 code files and 287 documents**; the sibling trees hold 1 468 and 2 611 documents; `AWA3D` 3 518 code files; `llama-prism` 1 585. **The user's ruling, 2026-10-10: none of it moves.** The material is consumed **in place by reference** — read where it sits, with the declaration citing a path and a digest as its witness — so there is no landing decision and no import job, and what lands here is the *declaration* alone. The old trees are superseded generations of this one, so they are **deleted as the canon settles, not migrated**: *"the shploingus clean will have cleaned itself out"*. Pruning a source later stales a witness; it does not lose a decision. What remains open is only *which* documents and files become declared rows, and in which lane (row 29's A–D) |
| 33 | **Nothing can tell a decision from a description** | the user, 2026-10-10, via PATCH-039 | **open, and it is the calibration the user named.** `ingest` can say a foreign file is `docs`; it cannot say which paragraph of it is a decision — a subject, an evidence, a rollback. That is the question a classifier answers, and `voxel_package::ballot` + `voxel-von` exist to answer questions of exactly that shape with `laya` as the seam. What is missing is the **calibration material and its accepted answers**, and the first corpus for it is already on the disk: the sixteen shared `hyperskill/*` ids are documented at five different life stages across five trees (`proposed`, `designed`, `verified-internal`, `applied-internal`, `approved`), and each stage is a labelled example. Until that is calibrated, "REAL work that goes straight into the new branch" rests on a classifier nobody has asked anything |
| — | **The compendium verbatim test** | PATCH-019 | **closed this round, by candidate 1.** The failure was a `core.autocrlf` checkout, not the record: `compress` joins a body with `\n` and the working copy of `docs/LEDGER.md` carried CRLF, so the assertion was about the checkout. The comparison now normalises line endings on the document side and nothing about the body's characters is loosened |
| — | **The integration tests run** | PATCH-019 | **closed this round.** `cargo test -p voxel-package` (every suite, not `--lib`) is green — 291 lib tests plus the 16 files under `crates/voxel-package/tests/` |

| 34 | **Two libraries in this workspace are read by nothing** | this round, found by writing the check | **open, and it is the dependency discipline's second rule with a measurement under it at last.** Every manifest in this tree repeats *"is it actually USED? (a resolve is not a use)"* in a comment, and **no instrument checked it**: the six census axes say what a crate *is* (named, moduled, tested, refusing, claimed, documented) and none asks who *reads* it, so a library nothing depends on scored exactly like `voxel-model`. `census::orphan_libraries` now reads the workspace's own manifests, treats a member another member points at (`path = "…"`) as used and a member with a binary target as an entry point, and prints the rest as `orphan-library` in **every build receipt**. It finds **`voxel-audio`** (the rain's synthesis, whose manifest answered rule 2 with *"yes — voxel-client plays what this returns"*: false, and false when written, now corrected in the manifest and the module header) and **`voxel-instrument`** (S-03/S-12/S-13 — the sensor concept, the band controller, the gauge wall, 1,417 lines whose only other mentions in the tree are comments citing `voxel_instrument::Sensor`). Both were built ahead of the world that would consume them and C-25 says so in prose; what was missing is that nothing measured the consequence. **Reported, never refused** — wiring one up, deleting it, or ruling a content crate deliberately unconsumed are all decisions for a reader, and a door here would refuse every build until the world catches up |

| 35 | **In-code prose grows unchecked, and the check that exists always says yes** | this round, from the user's own complaint | **built, and it caught its author on its first run.** The user's words: *"agents see the bloat tests and ignore them because somewhere in the project something says that the house rule is heavy docs in the code, despite ruling differently in the ledger."* Both halves are true and one mechanism caused them. `census::PROSE_FRAME_PERCENT` **is 10** and the `documented` axis refuses **sixteen of sixteen crates** against it — and all sixteen are **accepted** in `accepted-refusals.txt`, so the rule printed every build and bit nothing, while the code's own 30–51 % comment headers became the example every later file matched. So the frame is now a **ratchet** (`conformance/prose-frame.txt`, one `crate<TAB>percent` line each, declared from `check_prose_frame`'s own measurement): the share **may fall and may not rise**, a crate over its number **refuses the build**, and a crate under it reports the slack so a reader can see the decision is still open. It asks for no retrofit — the 10 % frame in a document a reader looks in is row 17 — and it refuses exactly the edits this project keeps having to apologise for. **Verified by it firing:** the first run refused `shploingus` at **52 % against a declared 51 %**, because the comments written into `crates/shploingus/build.rs` while wiring this very check had pushed its own crate over the line; the repair was to **cut the prose, not raise the number**, and the gate then printed all sixteen at budget with `slack=0`. `cargo test -p voxel-package --lib census`: 24 passed. **Corrected 2026-10-10, twice.** (a) The documentation retrofit landed (row 17) in each crate's own rustdoc, so `documented` no longer refuses any crate against that frame. (b) On the same ruling the measurement was **split**: rustdoc (`///`, `//!`) is documentation and is no longer counted against this budget at all, so the declared numbers were **re-declared over the plain-comment share** — 3–29 rather than 19–51, which is a change of definition and not progress, and the file's own header says so in those words. This ratchet is now the only check on prose that is not documentation — and it is the check that had the bug the user named: it used to charge a crate for writing its docs where they belong |

| 36 | **Nothing in this record ever asked for an outline** | the user, 2026-10-10 | **open, and it is a decision that was never captured, which is why no gate could report it.** The user's ask: *"make sure the missing materials rendering features are in — deadlock has additive outlines and npr highlights/rim light, subsurface scattering."* Measured, three of the four are in and one was never specified: **rim light** and the **specular warp** highlight are declared in `APPEARANCE.md` §1.2, built in `npr::shade`, drawn by `shading.wgsl` (§2 rim and 2b the coat), and measured — `Terms { half_lambert: true, rim: true, specular_warp: true, ambient: true, subsurface: true }`; **subsurface** was drawn by both the CPU and the shader and **invisible to `Terms`**, so `response` could not tell `subsurface_weight: Some(0.0)` from `None`, which is the rule that struct exists to keep — repaired this round, and the `a_stated_surface` measurement is now five terms rather than four; **outlines are absent**, and not by omission of the code: `APPEARANCE.md` §1.2 declares **four** terms and none is an outline, `npr.rs` has no outline branch, and every occurrence of the word in the client is debug or shell furniture (the `F4` merge rectangles, the change view's transient outline, the HUD lanes). There is no inverted-hull pass and no screen-space edge detect. **So the feature is missing from the record, not from the code**, which is the distinction this whole pass exists to draw: a wish the ledger never recorded cannot be contended with, counted, or reported missing. What a row here needs is the decision, not the code — which of the two techniques (an inverted hull expanded along the normal, per-material and additive, or a screen-space edge from depth and normals), whether it is a fifth term of the stylisation or its own pass, and whether it applies to every material or only the ones that declare it. Until that is decided, this is the one item in §9 whose blocker is a sentence rather than work |
| 37 | **What the compiler says about missing documentation** | the user, 2026-10-10 | **on, and it is the measurement the rustdoc ruling was missing.** `#![warn(missing_docs)]` is declared in all seventeen crate roots — the same place `#![forbid(unsafe_code)]` already stood — and nothing denies warnings anywhere (no `RUSTFLAGS`, no `[lints]`, no `-Dwarnings`), so it is **advisory**: `cargo check --workspace` exits **0** and prints the list. It flags **2 285 public items**, and the shape is the finding: **906 struct fields and 524 enum variants — 63 % of the total — are the bulk**, then 537 methods, 152 associated functions, 94 associated constants, 27 structs, 21 constants, 14 enums, 8 functions and 2 type aliases. So what is missing is not prose about *behaviour*; it is the reason for *data*, which is exactly the thing the rustdoc ruling (row 17) was written about: `voxel-model` **907** (its axis tables), `voxel-shapers` 464, `voxel-package` 340, `voxel-client` 190, `voxel-ui` 155, then `voxel-work` 58, `voxel-appearance` 54, `voxel-mesh` 40, `voxel-instrument` 33, `voxel-stream` 22, `voxel-noise` 15, `voxel-light` 6, `voxel-audio` 1. **Three roots flag nothing** — `voxel-von`, `voxel-mcp` and the `shploingus` binary, which has no public items at all — and the two small crates document every public item they have, which is why zero is a reading here rather than a silence. **Owed, and it is a decision rather than a sweep:** whether this count becomes a *ratchet* (may fall, may not rise, like `conformance/prose-frame.txt`) or stays a target, because nothing bounds it today — and 2 285 warnings on every build is a wall a reader stops reading, which is the failure this tree has already caught itself in twice. The measurement is `logs/missing-docs-check.txt`. **Followed the same day (row 17): every line of `conformance/missing-docs.txt` now carries a pin over the bytes its count was taken over, so the gate reports `stale` when the tree has moved instead of repeating a snapshot as this build's answer — and the gate now watches `conformance/`, which it did not, so a reading edited on disk is re-read rather than the previous answer being rendered as today's.** What the pin does not do is take the count: the recipe is still two commands, and a pin pasted beside an unchanged count is the one failure the file cannot see by itself (stated in its header). |
| 38 | **The light field has never measured a cell** | found by PATCH-047's own instrument, 2026-10-22 | **repaired, 2026-10-22 (PATCH-051), and the first of its two questions is the one that answered it.** Every `light-field` line written by the shipped binary — **375 of them** across `logs/` and every run of PATCH-047 — read `0 cell(s) marched`, with 1.3–1.7 M corners rigged per second. The field existed and was asked; the cause was **two address laws in one path**: the scene converted a corner with `floor(p / 0.25 m)` while `CellField::at` read those integers as **metres**, so every corner named a cell four chunks from its own and `WorldLight::march` found no tree under it. The repair is one law in one place — `VoxelPos::of_point` — with the cell's chunk and its own centre derived from the same type, so the key, the address and the ray's start cannot drift apart again. **Measured on the shipped binary, settled:** **82 207 cells held**, **225 984 of 228 700 corners (98.8 %) drawn from the world's own light** and 2 716 (1.2 %) on the rig, 30 909 cells adopted a second, **0 hops past a chunk not resident**, frame **8.4 ms / 117 fps / 0 stalls**, digest `7c66a632336cc0a3` unchanged; `light-shot`'s static frame measures all **82 749** cells its corners name in 0.6 s and differs from the rig in **73.5 %** of its pixels. What is still open is the row's *second* question, now for a different reason: the frame pays 68 B a corner whether the field answers or not, and at 98.8 % coverage that is the right format — but a window still streaming in spends its first seconds on the rig, and whether that window should build with the 24-byte record format until the field has something to say is a policy change with a visible switch, not a repair. Also named rather than fixed: every build's corners are **between one and two field steps old** (the field's step is 0.5°, the geometry's epoch is 1.0°), so the report reads `0 answered` / `225 984 a step behind` at every build, forever. PATCH-051 states both. |

**How a row leaves this table.** It moves up into the plan proper (§1–§8) when it is scheduled, and it
disappears when the work lands — a row that is done says how it was verified, in this tree's usual
shape, and a row that is closed without being built says who decided not to build it.
