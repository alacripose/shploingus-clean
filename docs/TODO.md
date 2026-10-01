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
      shape yet
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

| Piece | Source | Notes |
|---|---|---|
| Solar position (`declination`, `hour angle` → altitude, azimuth) | Meeus, *Astronomical Algorithms* (2nd ed. 1998); NOAA Solar Calculator | pure `f(tick, declared day length, latitude)`, no clock |
| Sky radiance | Preetham, Shirley & Smits (1999), *A Practical Analytic Model for Daylight*, SIGGRAPH | the analytic daylight model a renderer uses |
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
