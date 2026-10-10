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
