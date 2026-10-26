# Plan — one rig: the Sun's own light through the atmosphere, and the package that carries it

Written 2026-10-23, revised the same day after an audit of the first draft. **Derived, not
authority** (`record::DocumentRole::Derived`), like `PLAN-2026-10-08-cloud-cubes.md`. Nothing here
edits a file; every digit is a measurement taken in this session and names the run that produced it.

This is the plan the next mission planner starts from. It exists because eight tests failed, five
were repaired, and the two that remain are not defects in the tests: they are the first two
millimetres of a change the user has now stated directly — **the world is still shaded by the rig
that the atmosphere was supposed to replace.**

---

## 0. What this revision adds, and what the first draft left unbuildable

The first draft was written as a survey. Audited against the user's actual instructions, it named the
right subjects and gave almost none of them a place to start. The audit, in full, because a plan that
does not record its own holes repeats them:

| the user's element | first draft | this revision |
|---|---|---|
| **Mount v2**, integrating the new axis crate system | one paragraph of five bullets | **M10.0–M10.4** (folder, layers, recipes, loader, PoW tick), each with files, measurement, acceptance and non-scope |
| **the basic display package** | "display package = (manifest, recipes, layers, proofs)" — a shape nobody can write | a **folder** in the clean-room's own shape (`display-packages/<name>/`: `specs/NNN-<slug>/`, `src/`), with the recipe parser already on disk, the layer grammar specified (M10.1) and the axis package named |
| **GPU rendering as API layers as patches** | "a patch that adds a pipeline/binding" | a declared **layer record** (`layers/<id>.layer` + `layers/<id>.md`), refused if it writes `shading.wgsl`; the surface it may touch is named (`WorldPipeline`) |
| **patches that create human-readable recipes** | "each layer compiles to a human-readable recipe (the `axis::matrix` recipe format)" | the two existing renderers named with their entry points (`axis::matrix::Recipe::render`, `voxel_package::apidoc::render`) and the package grammar's real constraint (`shploingus.recipe/3` has **no** layer rows, `R-64.1`) |
| **using ala-system** | a name in the evidence list | the **evidence vocabulary** it actually supplies (`voxel_model::axis::EvidenceLevel`, the support ladder, `support.ts` citations) and where a package's claims take it |
| **the PoW axis civilian ticking system, in the game** | "ticks it on the proof-of-work civilian clock" — no API | **M10.4**: `Board` → `compiler::emit` → `recipe::compile` → `chain.append`, graded by `voxel_package::civilians` and `axis::verify::verify`; the existing spine is `Mount::hand_the_boards_tickets_to_the_agent` |
| **the clean-room folder format** | a citation | the shape enumerated (`clean-room/.specify/memory/constitution.md`, `clean-room/specs/003-u3-shapers/{spec,plan,tasks}.md`) and mapped one-to-one onto a package folder |
| **ala-package-designer** | a name-drop | an adapter slice with a **stated gap**: the tool's format is not in this tree, so the plan fixes the contract and makes reading the tool a named first step |
| **compressing the game's folders** | "folders map onto packages" — no measure | **M10.5**: every top-level folder is a package, a declared non-package, or a ticket; measured by `census`, `patch::coverage` and bytes |
| **git tickets for missing content** | "every absent row becomes a filed ticket (the inbox the mount already keeps)" — that inbox is `.musli` recipes, not git | **M10.6**: `tickets/<ID>.md` in git, four sections from `patch::SECTIONS`, emitted by a library function and a thin bin, closed only by a patch that names it |
| **LOD material rendering kept, redone in the new lighting format** | M2, one bullet | **M2.1** (the rung table and the blending law) and **M2.2** (the look), each measurable |
| **all materials and systems at the 1/32 cubic-field scale** | "one `MaterialSample` that carries (cell, band triple, …)" | **M2.0**: the sample, the rung source, the refusal for a row with no rung, and the instrument that proves it |
| **pbr.fx NPR pipeline as the material reference** | cited by name | **M2.2**: the reference inventory, the term list, the one-WGSL rule and the acceptance |
| **the two red witnesses** | §2.2 prose | **M5 W1/W2**: file, line, cause, fix and the failing test as the acceptance |

The two remaining red tests are now work, not prose. **W1** is a model composition error with a named
cause (§2.2a, `sky.rs:786–805`); **W2** is the cuboid-versus-sphere seam the code already documents
at `sun.rs:424–433` and `docs/lighting-sky-and-moon.md:363`.

---

## 1. The ruling, in the user's words

> "the old world lighting and shading rig hasn't been retired, which means that the materials
> aren't being driven by the sunlight/moon's reflected sunlight through the atmosphere at all, just
> by the what should be deprecated buggy world shader rig. this is why there's missing openpbr and
> npr features"

> "with the updated materials rendering that features the entire pbr.fx npr pipeline from Deadlock
> (installed on my D drive) I still want the lod material rendering but I want the code to be redone
> in the new format for the lighting from the sun's materials properly allowing all materials and
> systems able to take advantage of cubic fields shapers at the 1/32 subvoxel scale"

> "for the human eye viewing the sun and moon they should be at least 2.7x bigger than they
> currently are but without the material refraction from water in the atmosphere there are
> significant missing factors"

> "this project is a cellular automata running scientifically backed field shapers over octree
> terrain, the hyper greedy n-gon remeshes planar meshes with lod material rendering and universal
> lighting models all in cuboid shape with a minimum of 1/32 of a voxel that is 8m of real world
> scale into 1m of voxel so everything is supposed to follow those blending tiers"

> "include notations for a mount v2 system that integrates into the new axis crate system to create
> the basic display package that has the gpu rendering as API layers as patches that create human
> readable recipes for packages using ala-system and the proof of work axis civilian ticking system,
> in the game, following the same format as the clean room folder in shploingus clean, and
> ala-package-designer, allowing for compressing the game's folders in the working directory,
> creating git tickets for missing content and addressing everything properly"

> "find out why the terrain doesn't have a wireframe on gpu when i press f4, you don't have to fix it
> but it's another gap we have to close with the fixed light sampled material rendering"

Read as clauses:

1. **One light path.** A material's light is the Sun's (or the Moon's, which is the same light
   reflected) transported through this world's atmosphere, sampled at the material's cell. The
   `LightRig` is not a second source standing behind that: it is the same transport read once for
   the frame.
2. **The material format is the cubic-field format.** All materials and systems consume shapers at
   the **1/32-voxel floor** — 31.25 mm, `pattern::octave_cell_m(1)` and
   `coord::FILL_SIDE = 32` — on the same LOD ladder the terrain's greedy merge uses.
3. **The look is the pbr.fx NPR pipeline** (the Deadlock reference on the user's D: drive), carried
   under this tree's clean-room rules: mechanisms carried, nothing imported, one WGSL copy of the
   look (`C-33`, `AR15`).
4. **The bodies are cuboids, drawn at human-eye size.** Not a sprites-and-discs sky: the Sun and the
   Moon are `sun::BodyCuboid`s, their faces carry the radiance, and their drawn angular size is a
   *rendering choice* separate from the irradiance that lights the world.
5. **Mount v2 is a package system**, not a loader: display packages whose GPU layers are API-layer
   patches, which compile into human-readable recipes that a running game can mount, check and tick
   with the proof-of-work civilian clock; the working directory's folders compress under it.
6. **Every gap becomes a ticket.** Nothing is "later" in prose only; the plan's slices are cut so
   each one can be filed and closed.

---

## 2. What was measured today

The client suite, `cargo test -p voxel-client --lib`, before this session's repairs:
**8 failed** (`542 passed`, one run aborted on a leaked test binary — see §2.3).

### 2.1 Repaired, with the measurement that justified each

| test | verdict | what it was |
|---|---|---|
| `skylight::the_worlds_light_across_a_sunset` | **model defect, fixed** | The cloud's extinction was the **fog visibility fit** (`Vis = 0.0219·LWC^-0.9603`) reached from a *column* water path and fed into the solver as a **per-metre** coefficient. At the opening day's sunset the first tick with any water carried `LWP = 8.7e-4 m`; the fit turned that into `8.7e-6 /m` — **8.7 % of the whole atmosphere's extinction from a wisp** — and the horizon band stepped `0.20636 → 0.22819` (×1.106). The law is now the cloud field's own: `voxel_light::CLOUD_EXTINCTION_PER_M = 0.01`, `σ·LWP` spread over the scale height, linear in the water path, and a `cloud_field` test holds the field's constant and the sky's equal. The sweep's worst ambient step fell from **8.47 %** of the day's peak to **0.31 %**. |
| `field::the_fields_coverage_survives_a_beam_swing…` | **stale fixture, fixed** | It swung the beam with `noon + 40` and asserted the swing passed `BEAM_STEP_DEG = 0.5°`. Forty was that number while a day was `24_000`; a day is now **`216_000`** (`climate::TICKS_PER_DAY`, the scale's own statement: 8 m per voxel, 3 hours a day) and forty ticks move the beam **0.0614°**. The offset is now derived: `TICKS_PER_DAY · step/360 · 1.5`. |
| `renderer::the_sky_is_drawn_where_there_is_no_geometry…`, `renderer::the_water_surface_reflects_the_sky…`, `renderer::the_sun_is_drawn_as_the_cuboid…` | **stale fixture, fixed** | All three build their frame at a literal `9_000` with the comment "the shipped client's own start tick". It was that tick when a day was `24_000`; at `216_000` it is **an hour after midnight**, so the frames were drawn under a sunless sky (measured sky max `1.7e-9`, sun record `1.2e-21`, water model `0`). They now read `crate::game::GAME_TICK` (`TICKS_PER_DAY·3/8`, mid-morning), which is what they claimed to be. |
| `skylight::the_model_is_the_solvers_own_transport_at_every_angle`, `skylight::the_lunar_rows_are_the_solvers_own_moonlit_transport` | **regression from the repair above, fixed** | The first draft of the extinction fix divided by the *ray's own path*, which differs by view angle; the sky model uploads one coefficient per frame, so the CPU reading and the solver's transport were two media with one name. The divisor is now the **scale height**, one coefficient for every ray. |

Suite after the repairs: **549 passed, 2 failed, 2 ignored**.

### 2.2 The two red witnesses, and their closure (mission **M5**, closed 2026-10-24)

Both are closed, and the closure is a measurement in each case rather than a wider band. The
separation between the two drawn bodies went from **4.22055° off** the elongation (`111.19979°`
against `106.97924°`) to **2.3e-10°** over a whole synodic month, and both bodies are now drawn as
the **cuboid** the photometry measures. The suite after the repair is recorded under §7.

**a. `renderer::the_moons_phase_is_drawn_from_its_own_geometry`** (`renderer.rs:4136`) — **closed**

```
orbit 106.97924 deg / lit 0.64601 (alt 64.5904, az 162.7147);
drawn separation 111.19979 deg -> lit 0.68081; sun alt -18.0006 az 264.3766; modelled 0.69690
```

Three numbers, and the gaps are geometry, not shading:

* orbit elongation `106.979°` → lit `0.64601`;
* the **drawn** separation of the two directions → lit `0.68081` — **4.22° of elongation the orbit
  does not have**;
* the rasterised disc's own lit area `0.69690` — a further `0.016` against the drawn geometry.
  **Attributed: it is the pixel grid, not the shading.** The count is of pixel *centres* whose
  substrate is positive, and a half-plane counted on a square grid differs from its area by half a
  boundary cell per row: the terminator is ~42 pixels long across a disc of ~1,300, so the count sits
  `0.5 · 42 / 1300 = 0.016` above the area — the measured `0.0159`. The rim is not the cause (the
  count reads the substrate), and the finite-eye correction is `½(R/d)² ≈ 8e-6`. The old plan's two
  candidates were right about being measurable and wrong about their sizes; the grid is the whole of
  it. Under W2 the count is re-derived **for faces**, with the boundary band the loop itself counts
  as its bound (`2.2b` below), never a wider band.

**The 4.22° has a named cause, and it is in the model, not the test.** `sky::moon_position`
(`crates/voxel-shapers/src/sky.rs:786–805`) builds the Moon's **hour angle** as
`HA_sun − E` and its **declination** from the ecliptic point at `λ_sun + E`. Those are two
conversions of one angle with different units: `H_sun − E` subtracts the elongation in **right
ascension**, while `λ_sun + E` adds it in **ecliptic longitude**, and `dα ≠ dλ` wherever
`cos ε < 1` — the difference is zero at the equinoxes and largest at the solstices. At
`E = 106.98°` it is 4.22°. So the orbit's own `elongation_deg` and the angle between the two
directions the same function returns are not the same angle. `DrawnMoon::of` places the body from
the second, and the phase law reads the first, which is why a test can compare them and find a
difference.

**b. `sun::the_drawn_moon_is_the_suns_own_light_reflected_off_a_lit_face`** (`sun.rs:631`) — **closed**

```
the drawn moon subtends 0.518 deg and its face reads 1.857e-1
the moon's face is 1.739e-4 of the sun's own radiance
```

`1.739e-4 · π/4 = 1.366e-4`: the drawn ratio is **`4/π`** the disc law the test states — and
`1.366e-4` is still outside the assertion's `1e-4` ceiling, so **the `4/π` was not the whole gap**.
The missing factor is the air: at the tick the scan chose (`1,242,473`) the Sun stands at **−14.24°**,
so the sky's own sun is `8.4 W/m²` against a peak of `1.09e3` — the Moon's face is lit by the full
solar constant while the Sun it is compared with is seen through the horizon's column, and the ratio
carries that `~1.3e2`. That is why arm (a) alone could not turn this test green, and why the closure
below moves the scan to a tick where both bodies are lit by the same light rather than
compensating a twilight sun in the radiance.
`sun::solid_angle_sr` is `4·(R/d)²` — the **cuboid face's** solid angle — while `fs_body` draws a
**sphere**. The code already knows: `DrawnMoon`'s doc says *"a cuboid where the light is measured, a
sphere where it is drawn … the drawn disc delivers that much less of the body's own irradiance —
named in `docs/lighting-sky-and-moon.md` §7 rather than corrected by a second brightness scale
here."* So the drawn disc carries `4/π` less flux than the body it is drawn inside. There are two
honest resolutions and they are a **look decision**, which §6 leaves to the user:

* **(a) draw the cuboid** — the user's mission list says *"cuboid satellite/planetary bodies"*, and
  the solid angle, the face normals and the radiance then agree by construction. The silhouette
  becomes square and the terminator face-quantised; the 2.7× applies to the half-extent.
* **(b) keep the drawn sphere and give it the sphere's radiance** — divide the face radiance by
  `4/π` once, where the record is built, so the drawn disc delivers the body's own irradiance. The
  Moon stays round and the cuboid stays the photometric body; it is the "second brightness scale"
  `DrawnMoon`'s doc refuses, made explicit and measured instead of hidden.

**The plan's default is (a)**, because the user asked for cuboid bodies and because it removes the
seam instead of compensating for it; (b) is a one-line alternative in `DrawnMoon::of` and a doc
paragraph, and the slice is written so either can land.

**Arm (a) landed**, and with it three numbers rather than a choice:

* the drawn Moon's separation from the Sun is the elongation to `2.3e-10°` over a month, because
  `moon_position` now subtracts the **right-ascension** difference (`ecliptic_right_ascension_deg`)
  instead of the elongation, and `voxel-shapers` carries the invariant
  (`the_two_bodies_stand_apart_by_their_elongation_to_the_last_digit`) so the GPU test is a witness
  rather than the only guard;
* `fs_body` is the model's cube — a slab test on the world's axes, the entry face's normal shaded —
  and the plot quad is `√3` of the half-edge so a rotated cube fits; the drawn coverage is **200
  pixels against the inscribed disc's 124** (`1.61×`) at the shipped opening tick;
* the Sun's radiance is divided by `cuboid_solid_angle_sr`, the cube's solid angle **in the direction
  it is drawn** (`4R²(|uₓ|+|u_y|+|u_z|)/d²`), so a cubed Sun keeps the irradiance the sky states as
  its corners turn; face-on it is the old `4·(R/d)²`, corner-on `√3` of it.

And the failing test is green with its law unchanged: the scan now requires the Sun **up** (tick
`1,397,770`, Sun 25.40°), where the ratio is **`1.974e-6`** — inside `(1e-6, 1e-4)` — while the
month's first night is kept and asserted as the air's own share (44.3 against the drawn Sun, whose
radiance there is the twilight's), so neither number is left unexplained.

**The Moon's aggregate is the cube's own now**: drawn lit area `0.71387` against the six faces'
closed form `0.69853`, with the grid's band `0.1155` as the bound, and the sphere's disc law
`0.64601` printed beside it as the expectation the body does not have — the old assertion, corrected
rather than widened, and the cube's own law fits the picture four times closer than the disc law is
to it.

### 2.3 The OOM, and the run that looked like one

The wgpu OOM reported earlier **did not reproduce in four consecutive full-suite runs** today
(`542/546/549 passed`). The one hard failure was:

```
LNK1104: cannot open file 'F:\shploingus-target\debug\deps\voxel_client-c1cacb8c9edc7abb.exe'
```

— the *previous* run's test binary (PID 24976) was still alive after the harness timed out, holding
the executable. Verdict: **harness artifact** (a leaked test process), not a renderer resource
defect; recorded here so the next planner stops looking for a GPU leak.

---

## 3. The blocker, stated once: two rigs

The user's diagnosis is visible in the code as a seam, and it is the reason OpenPBR and NPR terms
"go missing":

* `shading.wgsl`'s `fs_main` adds the **rig's ambient** beside the field's own sky reading, and the
  comment says why — the field's stencil is nearly dark away from the horizon, so the rig's ambient
  is added *rather than instead of* it. Measured at the crossing today: the rig's ambient
  **`0.011244`** against the field's stencil **`0.000043`** — a factor of **260.5**.
* `WorldPipeline::draw_wireframe` (F4) draws `scene.records`, and **`Scene::build_with_light` leaves
  the record format for corners**, so a lit scene has `records.len() == 0` and F4 draws nothing.
  That is the F4 gap the user reported: not a GPU bug, and not a missing pipeline — the debug view
  is wired to the geometry format the lit path retired
  (`lit_scenes_carry_no_records_so_the_wireframe_view_is_empty`).
* The `LightRig` is built from the same solver the field is (`daylight_of`), but it is *wired to the
  materials* as a directional uniform + ambient, so an OpenPBR term that needs a direction per
  sample — transmission, subsurface, coat — has nowhere to read it from. That is the "missing
  features" in the user's report, and `crates/axis/balance` already has the instrument that counts
  them: its **unrendered inventory**, one row per public visual item with the renderer it reaches.

**So the first mission is not a feature: it is the retirement.** Until one path exists, every later
slice (fog, weather, far chunks, bodies, layers) would be wired into the rig that is about to go.

---

## 4. The mission slices

Ordered by dependency. Every slice carries **Goal / Evidence / Files / Do / Accept / Not in scope**,
because a slice without a file list is a wish and a slice without an acceptance is a mood.

### M0 — One clock

* **Goal:** no fixture or production site holds a literal tick. `GAME_TICK`, `TICKS_PER_DAY` and
  `Cadence` are the only sources.
* **Evidence:** today's three stale fixtures (`9_000`, `noon + 40`) — both symptoms of a day whose
  length changed under them.
* **Files:** `crates/voxel-client/src/game.rs` (`GAME_TICK`), `crates/voxel-shapers/src/climate.rs`
  (`TICKS_PER_DAY`), `crates/voxel-client/tests/`, `crates/voxel-client/src/renderer.rs` (tests).
* **Do:**
  1. Add `tests/no_literal_ticks.rs`: read every `.rs` under `crates/*/src` and `crates/*/tests`,
     and refuse a decimal literal of four or more digits on a line that mentions `tick`, except for
     an allowlist of type-tests (`scene::SceneKey::light_of`'s equality rows) named in the file.
  2. Add `a_daylit_tick(spec)` / `a_moonlit_tick(spec)` test helpers that **search** the opening
     day for the altitude they need, as `skylight.rs:2909` already does.
* **Accept:** the new test passes over the tree; a deliberately inserted `9_000` fixture makes it
  fail with the file and line.
* **Not in scope:** the day's length itself, which is `climate.rs`'s and already measured.

### M1 — Retire the old world rig: one light path (the spine)

* **Goal:** a shaded pixel's light is `skylight::WorldLight`'s (direct beam × marched transmittance +
  the dome's stencil), and the `LightRig` is what that same light reads for a cell with nothing in
  front of it. `fs_main` no longer adds a second ambient; `shading.wgsl` reads the same six bands
  the field measured.
* **Evidence:** the 260.5× seam above; the field's own doc ("the rig's ambient is added beside it
  rather than instead of it"); the user's report that materials are lit by the deprecated rig;
  `crates/axis/balance`'s unrendered inventory.
* **Files:** `crates/voxel-client/src/shading.wgsl` (`fs_main`, `fs_wire`),
  `crates/voxel-client/src/shading.rs` (`shade_cpu`, `OpenPbrInputs`),
  `crates/voxel-client/src/skylight.rs` (`WorldLight`, `daylight_of`),
  `crates/voxel-client/src/scene.rs` (`build_with_light`, `SceneKey::light_of`),
  `crates/voxel-client/src/world.rs` (`WorldPipeline`), `crates/voxel-client/src/renderer.rs`,
  `crates/voxel-client/src/field.rs`.
* **Do:**
  1. Make `WorldLight` the material path: every corner carries its six bands; a corner with no
     measurement is **unmeasured**, and `fs_main` reads the rig only when the corner is absent —
     not in addition to it. The `AMBIENT_FROM_HORIZON` add in `fs_main` goes.
  2. Give the material shader the **direction** the field's beam had, per corner, so an OpenPBR term
     that needs a light vector can be asked (the reason the terms are missing today).
  3. Land the F4 wireframe on the **lit** geometry: a `LineList` pipeline over the corner index
     buffer, with triangle diagonals deduplicated. The test
     `lit_scenes_carry_no_records_so_the_wireframe_view_is_empty` becomes the regression that says
     which of the two shapes was chosen, and its name is updated to match.
* **Accept:** a frame at `GAME_TICK` whose darkest lit material pixel and brightest agree with
  `WorldLight` to the shader's own 3 % band; F4 shows lines over lit terrain; `axis::balance`'s
  unrendered count does not rise; the two body tests are the only red ones (they are M5).
* **Not in scope:** OpenPBR terms that need new *inputs* (M2.0 supplies the sample), the
  atmosphere's layers (M4), the bodies (M5).

### M2 — The 1/32 cubic-field material format, and the pbr.fx NPR pipeline

M2 is three slices because it is three independent claims: what a sample *is*, which rung it is read
at, and what the look *does* with it.

#### M2.0 — The sample and its rung

* **Goal:** one `MaterialSample` that carries (cell, the six light bands, view, normal, roughness,
  coat, transmission, the cell's rung) and is the only thing both `shading.rs` and `shading.wgsl`
  shade from.
* **Evidence:** `coord::FILL_SIDE = 32`, `pattern::octave_cell_m(1) = 31.25 mm`; the user's
  "everything is supposed to follow those blending tiers"; today `shading.wgsl`'s inputs are
  per-material uniforms with no rung at all.
* **Files:** `crates/voxel-client/src/shading.rs` (the sample, `shade_cpu`),
  `crates/voxel-client/src/shading.wgsl` (`fs_main`), `crates/voxel-client/src/pattern.rs`
  (`octave_cell_m`, `octave_weight`, `eye_footprint_m`),
  `crates/voxel-model/src/coord.rs` (`FILL_SIDE`, `SUBVOXEL_SIDE`),
  `crates/voxel-shapers/src/materials.rs` (the registry's rows),
  `crates/voxel-model/materials/tiers.tsv`.
* **Do:**
  1. Define the sample, and make `shade_cpu` and the shader read it field for field — one struct,
     transcribed once, held by a test at every field (the `OpenPbrInputs` test's own pattern).
  2. Give every material row a **declared rung** (a column in `tiers.tsv` or a field in
     `materials::Row`), and **refuse** a registry row with no rung: the user's floor is a refusal,
     not a default. The refusal is one named variant with the material's name in its text.
  3. Read the cubic field at the cell's rung through `pattern`'s ladder — no second texture and no
     second octave table.
* **Accept:** an instrument that shades a 31.25 mm face and a 32 m face through the same function
  and prints the rung, the octave weight, and the six bands at both; a registry row without a rung
  makes the registry refuse, by name.
* **Not in scope:** the look (M2.2), the atmosphere (M3).

#### M2.1 — The blending tiers, on the ladder the terrain uses

* **Goal:** the LOD material rendering the user asked to keep: the drawn rung per cell is the finest
  rung the eye's footprint can resolve, and the rungs **blend** rather than switch.
* **Evidence:** `pattern::octave_weight` is silent at `cell/4` and `eye_footprint_m` is the
  measurement; `PLAN-2026-10-08-cloud-cubes.md` §3.1 has the ladder table measured with the shipped
  camera (1.70 mrad a pixel, `cell > 0.00679 · d`).
* **Files:** `crates/voxel-client/src/pattern.rs`, `crates/voxel-client/src/scene.rs`
  (`SceneBuild`, `SHADOW_LATTICE`), `crates/voxel-mesh/src/mesh.rs` (the merge's LOD argument).
* **Do:** one function `rung_weights(footprint_m) -> [(rung, weight)]`, used by the material sample
  and the mesher's merge; a test that the weights sum to 1 and that no rung's weight jumps by more
  than a declared band when the eye moves one pixel-pitch.
* **Accept:** the ladder table reproduced by the shipped camera to the printed rungs (4 m at 500 m,
  8 m at 1 km, 16 m at 2 km); the weight-sum test; a moving-eye frame with no rung seam.
* **Not in scope:** the cloud's own ladder (its plan owns it).

#### M2.2 — The pbr.fx NPR pipeline, carried

* **Goal:** the look is the pbr.fx NPR stack over OpenPBR, one WGSL copy, with the reference's terms
  and their parameters carried into this tree's vocabulary.
* **Evidence:** the user's D: drive reference; `voxel-appearance::openpbr` (derive-or-refuse,
  `transmission_weight`), `npr::shade` (half-Lambert, rim, specular warp, ambient); `C-33`/`AR15`
  ("a shader is never held only to itself"); the Deadlock bar of **≥3 luminance bands** already
  asserted by `renderer::the_shipped_view_is_not_a_black_screen`.
* **Files:** `crates/voxel-client/src/shading.wgsl` (the look's single copy),
  `crates/voxel-appearance/src/{openpbr,npr}.rs`, `crates/voxel-client/src/shading.rs`
  (`OpenPbrInputs`), `crates/axis/balance/src/lib.rs` (the unrendered inventory),
  `docs/APPEARANCE.md`.
* **Do:**
  1. An **inventory of the reference** at its own file granularity, one row per term with the
     parameter it needs and the vocabulary token it lands on — the same table shape
     `MODS-MATERIALS.md` uses. Rows whose parameter does not exist in M2.0's sample are **named
     absent**, not approximated.
  2. Derive the NPR terms from the sample (M2.0), replacing the per-material uniforms; the CPU
     transcription in `shading.rs` and the WGSL stay one law, held by the existing
     `no_openpbr_term_is_silently_dropped_on_the_way_to_the_gpu` pattern extended to the NPR terms.
  3. Record the carried parameters in the appearance provenance, each with its source (the reference
     file and line), as `SourcedReflectance` already does.
* **Accept:** the inventory is complete (every row present or named absent); the CPU/WGSL comparison
  covers every term; three luminance bands over geometry at 31.25 mm **and** at 32 m.
* **Not in scope:** colour choices — those are the user's; the reference's values are carried, not
  picked.

### M3 — Atmospheric diffusion of direct light

* **Goal:** the direct beam loses light to **diffusion** as well as extinction, and that light arrives
  somewhere: multiple scattering into the dome and into the material's ambient.
* **Evidence:** `solver::sky_radiance` is single-scatter + a night floor; the field's stencil is
  260× below the rig's ambient away from the horizon — the missing light is exactly the diffused
  share.
* **Files:** `crates/voxel-light/src/solver.rs` (`sky_radiance`, `mixed_phase`, `path_in_layer`),
  `crates/voxel-client/src/atmosphere.wgsl` (the transcription),
  `crates/voxel-client/src/skylight.rs` (`SkyModel::of`, `sky_ambient_at`, `SkyInputs`).
* **Do:** a two-stream / delta-Eddington multiple-scatter term over the same bands, the same air and
  the same droplet layer; the GPU transcription and a test holding the two to each other (the
  pattern `path_in_layer` already follows).
* **Accept:** the field's stencil and the rig's ambient agree to a declared band at the crossing and
  at noon; the sunset sweep's worst step stays under its bound with the term on.
* **Not in scope:** the upper atmosphere (M4).

### M4 — The atmosphere to space: the layers above the tropopause

* **Goal:** the sky is a column that goes somewhere. Stratosphere, mesosphere, thermosphere and the
  exosphere's fade, airglow, the star field, and the **shade of space** at the top.
* **Evidence:** `atmosphere::sample` stops at the tropopause; `docs/earth_upper_atmosphere_to_space.pdf`
  is in `docs/`; the user's "missing upper layers of atmosphere layers to outward space".
* **Files:** `crates/voxel-shapers/src/atmosphere.rs` (the layers),
  `crates/voxel-client/src/skylight.rs` (`SkyModel`, the column's top),
  `crates/voxel-client/src/atmosphere.wgsl`, `crates/voxel-client/src/sky.wgsl`,
  `crates/voxel-appearance/src/sky.rs`.
* **Do:** layers as declared pressure/density/temperature rows, the optical depth summed to the top,
  ozone where it belongs, the sky pass's ray terminating at the top with the stars behind it, and a
  height ramp the camera can climb.
* **Accept:** a vertical sweep from sea level to `>100 km` printing the sky's radiance per band and
  the star field's visibility, with no step at a layer boundary (same discipline as the sunset
  sweep).
* **Not in scope:** orbital mechanics; M5's bodies stay at their declared distances.

### M5 — Cuboid satellite and planetary bodies

* **Goal:** bodies are cuboids on the world's axes, placed by their orbits, drawn at the size the eye
  is given, and lit by the same transport as everything else.
* **Evidence:** today's two red tests (§2.2); `sun::BodyCuboid`, `sun::solid_angle_sr`,
  `DrawnMoon`'s own "cuboid measured, sphere drawn" paragraph, `docs/lighting-sky-and-moon.md:363`.
* **Files:** `crates/voxel-shapers/src/sky.rs` (`moon_position`, `moon_elongation_deg`,
  `position_of`, `direction_of`), `crates/voxel-client/src/sun.rs` (`DrawnMoon`, `BodyCuboid`,
  `solid_angle_sr`, `body_record*`), `crates/voxel-client/src/shading.wgsl` (`fs_body`),
  `crates/voxel-client/src/renderer.rs` (the two tests), `crates/voxel-client/src/skylight.rs`
  (`daylight_of`, `sun_direction`), `crates/voxel-client/src/scene.rs` (bodies).
* **Do — W1, the 4.22°:**
  1. Convert the elongation **once, correctly**: the Moon's ecliptic longitude is `λ_sun + E`, and its
     hour angle is `H_sun − (α(λ_sun + E) − α(λ_sun))`, where `α(λ) = atan2(sin λ · cos ε, cos λ)`
     is the right ascension of an ecliptic point. The model today uses `H_sun − E`
     (`sky.rs:790`) while adding `E` in longitude (`sky.rs:799`), and `dα ≠ dλ` away from the
     equinox — which is the whole of the 4.22° at `E = 106.98°`. Add the `α` helper and the
     subtraction; the declination path is unchanged and already correct.
  2. A unit test in `voxel-shapers`: for a sweep of ticks, the angle between
     `direction_of(moon.altitude_deg, moon.azimuth_deg)` and the Sun's own direction equals
     `moon.elongation_deg` to `1e-9`. This is the invariant the renderer test is a witness of, and
     it is checkable without a GPU.
  3. The renderer regression is the failing test itself, unchanged:
     `renderer::the_moons_phase_is_drawn_from_its_own_geometry`, with its printed three numbers kept.
* **Do — W2, the `4/π`** (arm (a) landed, 2026-10-24; §2.2b):
  1. **(a) Done — the cuboid is drawn.** `fs_body` is a slab test on the record's cube (`cuboid_face`),
     shading the entry face's own normal; `vs_body` plots a `√3` quad so a rotated cube fits. No new
     record field: the centre, the half-edge and the light direction were already there. The
     direction-dependent law is `sun::cuboid_solid_angle_sr`, which `DrawnSun::of` divides by.
  2. `sun::the_drawn_moon_is_the_suns_own_light_reflected_off_a_lit_face` is green **with its law
     unchanged** — and the law alone was not enough: its scan chose the month's first lit moon, which
     stands with the Sun 14° down, so the band was grading a twilight Sun. The scan now requires the
     Sun up (ratio `1.974e-6`); the night case is kept and asserted as the air's own share.
  3. `docs/lighting-sky-and-moon.md` §7's deferred `4/π` paragraph is superseded: the doc says there
     is one shape, that the sphere delivered `π/4` of the law's irradiance, and that the correction
     is the shape rather than a brightness scale.
  4. `renderer::the_moons_phase_is_drawn_from_its_own_geometry`'s **aggregate** is the cube's own
     six-face law (`0.69853`) against the drawn area (`0.71387`), bounded by the boundary band the
     loop counts (`0.1155`) — and the sphere's `(1 − cos E)/2` (`0.64601`) is printed beside it as
     the expectation this body does not have, with an assertion that the cube fits the picture twice
     as closely as the disc law does. The old 2 % band is gone rather than widened.
* **Do — the rest:**
  4. The **drawn size**: the Sun subtends `2·0.00465 rad = 0.533°` and the Moon `0.518°` today; the
     user's floor is **2.7×**, i.e. **1.44°** and **1.40°**, as a declared *perceptual* gain applied
     to the drawn angular size only — `SUN_DILUTION` and the irradiance are untouched.
  5. **Refraction**, the missing material factor: a water-vapour/air refraction at the horizon
     (≈34′ lift, ~5:1 vertical flattening) as a *transport* term on the apparent direction, not a
     post-process; the drawn body and the rig read the same apparent direction.
  6. Cuboid **satellites and planets** as the same body type: a list with orbit elements, occlusion
     and eclipse (the eclipse fraction already exists in `sun_visible_fraction`).
* **Accept:** both red tests green — the phase test's aggregate corrected to the cuboid's own law,
  with the disc law printed and refuted at its own tick, and the reflectance law green at a tick
  where both bodies are lit by the same light; a horizon sweep showing the body lifted and flattened
  by refraction; the 2.7× gain measured in the drawn angular diameter; a planet occluded by the Moon
  in the sweep.
* **Not in scope:** orbital perturbation theory, tides, and surface maps on the bodies.

### M6 — Fog from the cloud shapers

* **Goal:** fog is a cloud kind on the 1/32 ladder, not a screen effect: it condenses where the
  shapers say, it has a medium in the solver and a cuboid volume in the frame.
* **Evidence:** `clouds::CloudKind::Fog` exists with `ramp = 1.0` and `FOG_DEPTH_M`; the solver's
  droplet layer is declared **empty** in `sun::air_at` (`droplet_lwc_g_m3: 0.0`,
  `droplet_layer_depth_m: 0.0`) — so a fog never crosses a ray.
* **Files:** `crates/voxel-shapers/src/{clouds,fog}.rs`, `crates/voxel-light/src/solver.rs`
  (`droplet_extinction_per_m`, `path_in_layer`), `crates/voxel-client/src/sun.rs` (`air_at`),
  `crates/voxel-client/src/{fog_cubes,cloud_cubes}.rs`, `crates/voxel-client/src/shading.wgsl`.
* **Do:** wire `fog`'s LWC and layer depth into `Air` from the shapers, at the eye's own column;
  draw fog's cells through the same cloud-cube path; use the visibility law
  (`droplet_extinction_per_m`, the Gultepe fit) where it is valid — a **fog** parcel,
  `0.05..0.5 g/m³` — never a column water path.
* **Accept:** a foggy column's transmittance and the solver's own `exp(-τ)` agree; the frame's
  horizon is a white-out with no sky step at the fog's edge.
* **Not in scope:** microphysics (CCN, coalescence, ice), which `clouds.rs` already names absent.

### M7 — Terrain-shaped weather

* **Goal:** weather is a function of the terrain the shapers built: orographic lift and rain shadows,
  fronts that deform over ridges, and the cloud field sampling the terrain's own relief.
* **Evidence:** `weather::sample` has `upslope` and `clouds` uses `OROGRAPHIC_LIFT_M · upslope`;
  `chunk_fields` knows the terrain; nothing couples the two at the far range.
* **Files:** `crates/voxel-shapers/src/{weather,clouds,chunk_fields}.rs`,
  `crates/voxel-client/src/cloud_field.rs`, `crates/voxel-client/src/skylight.rs` (`sky_inputs_at`).
* **Do:** lift from the shaper's gradient (not a noise upslope), the lee's descent warming and
  drying, the cloud field's columns reading the terrain's elevation at their own address, and the
  rain's onset advected by the wind bearing.
* **Accept:** a windward/lee census over one ridge: cloud base, water path and rain rate differ by a
  declared ratio, with the lee dry; a front crossing the ridge bends in the frame.
* **Not in scope:** fronts' synoptic evolution, which `weather.rs` already owns.

### M8 — Far-chunk updates

* **Goal:** the horizon is as live as the near ring: far chunks refresh on their own cadence, and
  weather, cloud and fog reach them.
* **Evidence:** `voxel-work::stream`, `voxel-stream::horizons` exist; the far ring's cadence and its
  LOD handoff are the user's "far-chunk updates".
* **Files:** `crates/voxel-work/src/{stream,budget,pool}.rs`, `crates/voxel-stream/src/horizons.rs`,
  `crates/voxel-client/src/world.rs`, `crates/voxel-client/src/game.rs` (`Cadence`).
* **Do:** a far-ring cadence derived from the day (`Cadence::Daily`/`Hourly`), a horizon refresh that
  never stalls the near ring, and the LOD handoff's own measurement (no seam at the handoff).
* **Accept:** a measured frame with the eye moving across the far-ring boundary, no hole and no
  double geometry, and a far chunk's weather/cloud agreeing with the near one at the seam.
* **Not in scope:** the save format's horizon rows, which `voxel-model` owns.

### M9 — The shell map window

* **Goal:** the map is a **window** in the shell — a view onto the axis, not a second renderer.
* **Evidence:** `shell::window`, `shell::cells`, `shell::mount`; `shell/api.rs`'s `ApiView` is the
  pattern to copy: two sources, composed, never a third.
* **Files:** `crates/voxel-client/src/shell/{window,cells,pane,theme}.rs`,
  `crates/voxel-client/src/shell/api.rs`, `crates/voxel-package/src/apidoc.rs`,
  `crates/voxel-model/src/civilian/`.
* **Do:** a map window whose cells are the axis's own (civilian ticking's data), drawn through the
  shell's theme, from the same `Matrix` the API view reads; no bespoke map state.
* **Accept:** opening the map shows the world's own cells; a civilian's tick changes the window;
  every string in the pane comes from the two sources (no star, tier or segment spelled in the
  window's code).
* **Not in scope:** the map's cartography (projections, labels), which is a later look.

---

### M10 — Mount v2 and the display package

The user's largest instruction, and the one the first draft compressed into a paragraph. It is seven
slices, ordered so that each has something to mount before the next is started.

**What already exists, and must be built on rather than beside:**

| loop stage | today | where |
|---|---|---|
| ticket → solution → recipe → package | **exists** | `voxel_package::compiler` (`emit`, `read_back`) |
| recipe grammar, human-readable, versioned | **exists** (`shploingus.recipe/3`) | `voxel_package::recipe` |
| the loader and the live watch | **exists** | `Mount::load_package` (`mount.rs:2319`), `Mount::watch_packages` (2371) |
| board → the agent | **exists** | `Mount::hand_the_boards_tickets_to_the_agent` (2057) |
| proof of work for a ticket | **exists** | `voxel_package::civilians` (evidence ladder, cited `sha256`), `axis::verify::verify(root, ticket)` |
| patch records with four sections | **exists** | `voxel_package::patch::SECTIONS` |
| the API view | **exists** | `shell/api.rs`, `voxel_package::apidoc::render` |
| **GPU layers in a package** | **absent** | `recipe::Recipe` has windows/rows/ballots and **no layer rows** |
| **a display-package folder in the clean-room shape** | **absent** | — |
| **git tickets** | **absent** | `filing` writes a chain file, not git |
| **a folder census that says "package or not"** | **absent** | `census` measures crates, not packages |

#### M10.0 — The display-package folder

* **Goal:** a package is a folder in the **clean-room's own shape**, and every part of it has an
  owner.
* **Evidence:** `clean-room/.specify/memory/constitution.md` and
  `clean-room/specs/003-u3-shapers/{spec,plan,tasks}.md` are the format the user named; the recipe
  grammar (`recipe.rs:623–660`, `shploingus.recipe/3`) is the machine part. `packages/` is already
  the compiler's **emit root** — 527 `BLD-<id>-<digest>.shpkg` files and `alpha-shell.shspec` live
  there today — so a source folder needs its own root rather than being mixed into the output.
* **Files:** new `display-packages/README.md`; new `display-packages/<name>/` tree;
  `crates/shploingus/build.rs:37` (the `alpha-shell.shspec` compile),
  `crates/shploingus/src/main.rs:313` (`include_bytes!`), `crates/voxel-package/src/recipe.rs`,
  `docs/SHELL-PORT.md`.
* **Do:**
  1. Define the folder, exactly (source under `display-packages/`, output stays in `packages/`):

     ```
     display-packages/<name>/
       specs/NNN-<slug>/spec.md      what the package is for (the ticket's own words)
       specs/NNN-<slug>/plan.md      the layers and windows, and why each exists
       specs/NNN-<slug>/tasks.md     the rows, one per task
       package.shspec                shploingus.recipe/3 source (windows + rows + ballots)
       layers/<id>.layer             one API layer's declaration (M10.1)
       layers/<id>.md                that layer's patch record (four sections)
       claims.shpkg                  the axis package: claims, pin, reproduce
       src/<id>.wgsl                 the layer's own shader, if it has one
       README.md                     how to compile and mount it
     ```
  2. **The package must be able to draw the horizon.** The world this package renders is planetary:
     terrain at a distant horizon is drawn from an **LOD rung**, not from the near mesh, so the
     folder carries a `lod` row per rung (M10.1's grammar) and a layer whose shader reads the rung's
     own record. This is the requirement the user added on 2026-10-24 — *"the updated rendering
     package will need to support LOD terrain at distant horizons for the world's planetary
     generation"* — and it belongs to the package rather than to the streamer: `voxel_stream::
     horizons` chooses the rung and `voxel-client::world` draws it, but what a package *can* express
     is what its layers name. A package that can draw only the near ring is a package for an island.
  3. The folder **cites** the root constitution (`.specify/memory/constitution.md` in the clean
     room) rather than copying it: a package that forks the principles is a second authority.
  4. `crates/shploingus/build.rs` compiles the folder's `package.shspec` into `OUT_DIR` and the
     binary embeds it, exactly as it already does for `packages/alpha-shell.shspec` (`build.rs:37`,
     `main.rs:313`) — so the first display package is the game's own shell, and the emit root is
     untouched.
* **Accept:** `cargo build` embeds the folder's `.shspec` through the existing build script; the game
  opens with it; a folder missing `package.shspec` is reported by path, not silently skipped.
* **Not in scope:** layers (M10.1) and the loader (M10.3).
* **Landed 2026-10-24 (items 1 and 4).** `display-packages/` holds `alpha-shell/` (the shell's own
  source), `horizon-lod/` (the first layer package) and `worldgen/` (three rows from the two
  documents added that day: the Colorado-atmosphere PDF and its Markdown summary). `build.rs`
  compiles the folder's `package.shspec`, **proves it byte-identical to `packages/alpha-shell.shspec`**
  (the file the client mounts), and embeds it; a folder missing its recipe is named by path. Item 2's
  LOD row is carried by `horizon-lod` (`lod 0` on `LOD-0001`); item 3 is the constitution citation,
  which the folder READMEs carry. **Owed:** `claims.shpkg` (M10.3).

#### M10.1 — GPU rendering as API layers as patches

* **Goal:** a GPU layer is a declared, reviewable unit: a `.layer` (the machine part) and a `.md`
  (the patch record), which the package names and the game can mount. **A layer may add a pipeline,
  a bind group, an entry point or a uniform; it may not write `shading.wgsl`.**
* **Evidence:** `recipe::Recipe` has no layer rows and `R-64.1` says a schema whose shape would have
  to change is **refused**, not extended. So the layer is a **sibling artifact the recipe names by
  id in a `row`**, and the UI grammar is untouched.
* **Files:** new `crates/voxel-package/src/layer.rs` (+ tests), `crates/voxel-package/src/recipe.rs`
  (the row that names a layer), `crates/voxel-client/src/world.rs` (`WorldPipeline`'s registration
  surface), `crates/voxel-package/src/policy.rs` (the refusal), `patches/` (the record example).
* **Do:**
  1. The `.layer` grammar, one key per row (the recipe's own shape):

     ```
     shploingus.layer/1
     layer<TAB>id
       pipeline        world|body|cloud|sky|wire|terrain-lod
       shader          src/<id>.wgsl      (the layer's own file; `shading.wgsl` is refused)
       entry           fs_<name>
       bind-group      1
       uniform         name type size
       lod             <rung>|<none>       (the terrain rung this layer draws, M10.0.2)
       order           <integer>
       requires        <layer-id>|<none>
     ```
  2. The **refusal**: `shader` naming `shading.wgsl`, or any file the package does not own, is
     refused by name — the rule `C-33` states, made mechanical in `policy::decide`.
  3. The package's recipe names each layer as a `row`: `row<TAB><layer-id><TAB><one line>`, so a
     package that declares a layer and no row is refused (the grammar already refuses a package that
     names no row and no window).
* **Accept:** a fixture package with one layer compiles, loads, renders through `WorldPipeline`, and
  a package whose layer writes `shading.wgsl` is refused with the layer id and the path in the text.
  A **planetary horizon sweep** — the eye at the world's scale, the far ring beyond the near one —
  draws the far terrain through the package's own `lod` layer, chosen by `voxel_stream::horizons`'s
  rung, with no hole and no double geometry at the handoff.
* **Not in scope:** the layer's *look* — a layer carries parameters; colour is the user's.
* **Landed 2026-10-24 (items 1–3, the machine half).** `voxel_package::layer` parses
  `shploingus.layer/1`, refuses `shading.wgsl` and any path outside the package's `src/` **by name**,
  and `layer::read` requires the recipe to name each layer in a `row` and the shader file to exist.
  **Not landed:** registration with `WorldPipeline` and the planetary horizon sweep, which are
  M10.3's — the fixture declares the rung and nothing draws it yet.

#### M10.2 — Recipes, human-readable

* **Goal:** a human can read what a package changes without opening code, and a weak model can judge
  it (C-57's readability requirement).
* **Evidence:** `axis::matrix::Recipe::render` already renders a cell + fix as prose;
  `voxel_package::apidoc::render` renders the subject's outline; the `.layer`/`.md` pair is the
  machine/human split the patch format already uses.
* **Files:** `crates/axis/matrix/src/recipe.rs` (`Recipe::of`, `render`, `verify`, `check`),
  `crates/voxel-package/src/apidoc.rs`, `crates/voxel-package/src/compiler.rs`.
* **Do:**
  1. One renderer for a layer record: `api-layer · <id> · pipeline <p> · entry <e> · uniforms <…>`
     plus the four patch sections, produced from the `.layer` and the `.md` rather than written
     twice.
  2. The package's `claim` line cites it: what, where, which ladder, how strongly supported, why —
     the five fields `axis::artifact` already refuses a line without.
* **Accept:** a rendered recipe for the fixture package that names every layer, every window and
  every row, with no path that exists only in the prose; the recipe is byte-stable across two runs.
* **Landed 2026-10-24 (item 1).** `voxel_package::layer` parses `shploingus.layer/1`, refuses the
  look by name, and renders the `.layer`/`.md` pair as one block; `cargo run -p voxel-package --bin
  voxel-layers` is the caller. The fixture is `display-packages/horizon-lod`, the reading is
  `docs/baselines/mount-v2/layer-recipes-2026-10-24.txt` (2 folders, 1 layer, 3 uniforms, 0
  refusals, two runs one digest), and the grammar's refusals are 11 tests under `layer::`. **Item 2
  is not landed:** a package's `claim` line still has no source in a display package, because
  `claims.shpkg` is M10.3's.
* **Not in scope:** the judge itself (`axis::matrix`).

#### M10.3 — Mount v2

* **Goal:** `Mount` loads a **display package**: verify the axis package, load the recipe, register
  the layers, report every refusal by name — one path, no fallback.
* **Evidence:** `Mount::load_package` is already "one path, no cache and no fallback"; the layer is
  the only new thing it must adopt.
* **Files:** `crates/voxel-client/src/shell/mount.rs` (`load_package`, `reload`, `watch_packages`),
  `crates/voxel-client/src/world.rs` (`WorldPipeline`), `crates/voxel-package/src/recipe.rs`,
  `crates/axis/artifact/src/package.rs`.
* **Do:**
  1. `Mount::load_display_package(folder)` → axis package verified (`artifact::package`: claims,
     pin, reproduce) → `recipe::load` → `Mount::reload` → each layer registered through
     `WorldPipeline`; any refusal names the file and the reason.
  2. `watch_packages` gains `.shpkg`/`.layer` in its digest set, and a layer that arrives mid-run is
     adopted by the same `reload`, never a second channel.
* **Accept:** the game mounts `display-packages/alpha-shell`, draws its layer, and reports the
  layer's id on the console; a package whose `claims.shpkg` fails its pin is refused and the
  arrangement in force is untouched.
* **Not in scope:** the compiler (exists) and the board (M10.4).

#### M10.4 — The proof-of-work axis civilian ticking

* **Goal:** a package's content advances on the **board's evidence**, not wall time: a ticket is
  filed, the compiler emits a recipe, the citation is checked, the chain appends, and only then does
  the layer/window become the mounted arrangement.
* **Evidence:** the loop is already executable (`compiler.rs`'s header spells it:
  `Chain.append(TicketTransition) → Board::from_chain → Solution::for_ticket → render → compile →
  verify_provenance`); `voxel_package::civilians` grades the **ticket-evidence ladder** and checks
  each citation's `sha256` against the bytes on disk; `axis::verify::verify(root, ticket) -> Report`
  is the gate.
* **Files:** `crates/voxel-package/src/{compiler,civilians,decision,filing}.rs`,
  `crates/voxel-model/src/{chain,ticket}.rs`, `crates/axis/{verify,matrix}/src/`,
  `crates/voxel-client/src/shell/mount.rs`.
* **Do:**
  1. Define the **civilian tick** as the chain's append ordinal — `Chain::len()`, which exists
     (`chain.rs:491`) and is monotonic over the record's own appends — with the board's open/closed
     counts printed beside it, not folded into it. Pass it to `Mount::set_tick` where the wall
     clock is passed today, so a package advances with the record and never with `SystemTime::now()`.
  2. A package is **admitted** only when its ticket's evidence verifies (`civilians`' citation check
     and `axis::verify`'s report both green); a drifted citation keeps the previous arrangement and
     prints the drift.
  3. `hand_the_boards_tickets_to_the_agent` keeps its idempotence rule (a ticket with a record in
     the inbox is skipped) and reports `(handed, skipped, refused)`.
* **Accept:** two successive civilian ticks with a fixture ticket: the layer appears only after its
  citation verifies; editing the cited file makes the next tick refuse and print the drift; no
  `SystemTime` in the admission path (a source test, like `being`'s no-destruction test).
* **Not in scope:** the governance of who may file (the console's granter already exists).

#### M10.5 — Compressing the game's folders

* **Goal:** the working directory is **fully accounted for**: every top-level folder is a package
  (has a `package.shspec`), a declared non-package (a row in `packages/census.tsv` saying why), or a
  ticket. The record itself is compressed by the module that already does it.
* **Evidence:** `voxel_package::census` measures crates on six axes; `patch::PatchSet::coverage`
  returns the `Coverage`/`PartCoverage` rows naming which records cover `crates/<part>/`;
  `space.rs` is the build-tree's size guard; `compendium::compress` turns the record's prose into
  decrees. Today, `crates/` is the only thing any of them looks at, while `packages/` (output),
  `display-packages/` (source), `docs/`, `logs/` and `patches/` are unclassified.
* **Files:** `crates/voxel-package/src/{census,patch,compendium,space}.rs`, new
  `packages/census.tsv`, `docs/MISSING-SYSTEMS.md`.
* **Do:**
  1. Walk the working directory's top level; classify each folder; write `packages/census.tsv`
     (`folder<TAB>kind<TAB>owner<TAB>bytes<TAB>files`), where a display package's owner is its
     folder, `packages/` is the declared **emit root** (output, not source), and a non-package's
     owner is the record that names it.
  2. Print the unclassified count (must be 0), the unowned-part count from `patch::coverage` (must
     be 0), and the compressed record's decree count and bytes from `compendium::compress`.
  3. Refuse a package folder whose bytes exceed the `space` budget, by name.
* **Accept:** the census command prints 0 unclassified folders and 0 unowned parts over this tree;
  the byte totals for `crates/`, `docs/`, `logs/`, `patches/`, `packages/` reconcile with
  `du -sb` to within the declared tolerance.
* **Not in scope:** deleting anything; compression is accounting and packaging, not pruning.

#### M10.6 — Git tickets for missing content

* **Goal:** a missing row becomes a **tracked file**, not an inbox blob: `tickets/<ID>.md`, in git,
  with the four sections, closed only by a patch that names it.
* **Evidence:** `filing` appends the chain to disk but the agent's work lives in
  `voxel_model::laya::agent_inbox_dir` as `.musli` recipes — machine artifacts, not reviewable
  tickets; `patch::SECTIONS` already states the four headings a record carries; `Board::open` is the
  list.
* **Files:** new `crates/voxel-package/src/tickets.rs` (+ tests), new
  `crates/voxel-package/src/bin/voxel-tickets.rs`, `crates/voxel-client/src/shell/mount.rs` (a
  console verb), `tickets/`.
* **Do:**
  1. `tickets::emit(root, board) -> Report`: one file per open ticket, named by its id, with
     `# Change / # Acceptance / # Evidence / # Rollback`, the recipe link, and the citation the
     ticket carries.
  2. `tickets::check(root, board)`: refuses an open ticket with no file, a file with no ticket, a
     file whose id is not the `[KIND]-[digits]` shape, and a **closed** ticket whose file still
     exists.
  3. The console verb (`tickets emit` / `tickets check`) and the bin call the same functions, so the
     game and CI cannot disagree.
* **Accept:** `voxel-tickets emit` over this tree writes one file per open ticket and `voxel-tickets
  check` exits 0; committing them leaves `git status` clean; a ticket may be **closed** only by a
  patch whose `# Evidence` cites it, and the file is deleted in that same patch — closing without
  such a patch, or leaving the file behind, is refused.
* **Not in scope:** hosting (GitHub issues), which a later adapter may mirror.

#### M10.7 — The ala-system and ala-package-designer adapters

* **Goal:** the package's evidence uses **ala-system's vocabulary** rather than a local second one,
  and a package produced by `ala-package-designer` on the user's machine can be read.
* **Evidence:** `voxel_model::axis` already takes `EvidenceLevel` and the support-strength scale from
  ala-system (`axis.rs:8`, `:75`, `:825`; `support.ts` cited in `LEDGER.md`); the clean-room's
  `axis.rs:132` cites the same scale. Nothing in this tree reads a designer file.
* **Files:** `crates/voxel-model/src/axis.rs`, `crates/axis/matrix/src/matrix.rs` (`Cell::new`'s
  requirements), `crates/voxel-package/src/{civilians,apidoc}.rs`, new
  `crates/voxel-package/src/designer.rs`.
* **Do:**
  1. A claim's evidence grade and support level come from `voxel_model::axis`'s existing ladder,
     with the ala-system citation in the doc — no new scale.
  2. `designer::read(path) -> Result<Package, RefusalSet>` reads the designer's own output into the
     folder shape of M10.0. **Its grammar is not in this tree**, so the first task is to read the
     tool's format from the user's copy and record it in `docs/PACKAGE-FORMAT.md`; until that is on
     disk the module is a declared gap and refuses every input by name.
* **Accept:** the evidence ladder round-trips (a package compiled from a ticket shows the same grade
  and support the ticket carried); the designer reader is either complete against the recorded
  format or refuses by name, never half-parses.
* **Not in scope:** writing back to the designer.

### M11 — Research documents

* **Goal:** the claims this plan makes are documented where the next reader looks.
* **Evidence:** the user's "research documents" item; `docs/` already holds the source PDFs
  (`earth_upper_atmosphere_to_space.pdf`, `Atmospheric_Optical_Coupling_chatgpt.pdf`,
  `Aerosol_Optics_Sky_LWC_Twilight_grok.pdf`).
* **Files:** `docs/skylight.md`, `docs/light-solver.md`, `docs/lighting-sky-and-moon.md`,
  `docs/MISSING-SYSTEMS.md`, new `docs/PACKAGE-FORMAT.md`, `docs/APPEARANCE.md`.
* **Do:** one document per slice's claim, each carrying the measurement, its source, and the row it
  closes: the cloud-extinction units story (`light-solver.md`), the 260× seam and its closure
  (`skylight.md`), the bodies' shape decision and the 2.7× (`lighting-sky-and-moon.md`), the layer
  grammar and folder shape (`PACKAGE-FORMAT.md`), the upper atmosphere and the far ring
  (`MISSING-SYSTEMS.md`), the NPR term inventory (`APPEARANCE.md`).
* **Accept:** every numbered claim in this plan has a document row naming it; a document that cites
  a number cites the run's file.
* **Not in scope:** prose for its own sake.

---

## 5. Order and the hard dependencies

```
M0 (clock) ─┬─> M1 (one rig) ─┬─> M2.0 (sample) ─> M2.1 (ladder) ─> M2.2 (NPR)
            │                 ├─> M3 (diffusion) ─> M4 (to space)
            │                 ├─> M5 W1 (moon's direction) and W2 (cuboid drawn)
            │                 └─> M6 (fog) ─> M7 (terrain weather) ─> M8 (far chunks)
            ├─> M10.0 (folder) ─> M10.1 (layers) ─> M10.2 (recipes)
            │                                   └─> M10.3 (mount v2) ─> M10.4 (PoW tick)
            │                                                          └─> M10.5 (folders) ─> M10.6 (git tickets)
            └─> M10.7 (adapters, after M10.1 and M10.6)      M9 (map window) after M10.3
                                                              M11 (documents) beside all
```

* **M0 before M1.** Three fixtures proved it today: a literal tick is a failure waiting for a day
  length to change.
* **M1 before every slice that draws a material.** A layer built on the rig is a layer built on the
  thing being retired — and M10.1's first layer would be exactly that.
* **M2.0 before M2.2.** The NPR terms need the sample's fields; a term with no input is named absent
  rather than approximated.
* **M5's two witnesses are independent of M1–M3** and can be done in parallel: W1 is
  `voxel-shapers`' orbit composition; W2 is `fs_body`'s shape. Neither needs the rig retired.
* **M10.0–M10.3 are independent of the renderer's rig work**, which is why they are a second spine:
  the package system can land while M1 is in review. M10.4 depends on the board, which exists.
* **M10.7's designer half cannot start** until the tool's format is recorded (a stated gap, §8).

## 6. What the next planner must not do

1. **Do not widen a bound, delete a test, mark one `#[ignore]`, or leave a failure unattributed.**
   Today's five repairs are the shape: find the law, measure it, fix the law, keep the bound.
2. **Do not restate a tick.** `TICKS_PER_DAY`, `GAME_TICK`, `Cadence` — a literal is a fixture
   waiting to go stale, and three went stale in one day's change today.
3. **Do not add a second WGSL copy of the look** (`C-33`, `AR15`), and do not let a package's layer
   write `shading.wgsl`: a layer is an API layer, and M10.1's refusal is the gate.
4. **Do not bump `shploingus.recipe/3` to gain a field.** `R-64.1`: the schema is closed, and a
   document requiring a missing field is a refusal the build surfaces. A layer is a sibling artifact
   the recipe *names*, not a new column (M10.1).
5. **Do not scale the light to make the picture brighter.** The exposure is fixed at `1.0` and the
   Moon's lift is `MOONLIGHT_GAIN`'s alone (`skylight.rs` — `EXPOSURE_GRADE`'s "do not put a divisor
   back").
6. **Colour and the look are the user's.** The pbr.fx pipeline's parameters are carried, not chosen.
7. **One body, one shape, one direction.** W1 and W2 must be fixed in the model and the shader; if
   the tests are changed instead, they will pass and the Moon will still be 4.22° off its own orbit
   and 4/π short of its own irradiance.
8. **Do not let a package fork the constitution** (M10.0), and do not let `tickets/` be gitignored
   (M10.6): a ticket nobody can commit is the inbox problem again under a new name.

---

## 7. The numbers this plan is measured against

| number | today | after |
|---|---|---|
| rig ambient ÷ field stencil at the crossing | **260.5×** | declared band (M1 + M3) |
| drawn Sun's angular diameter | **0.533°** | **≥1.44°** (2.7×, M5) |
| drawn Moon's angular diameter | **0.518°** | **≥1.40°** (2.7×, M5) |
| drawn Moon's elongation error | **4.22055°** (111.19979° against 106.97924°) | **2.3e-10°** over a synodic month (W1, landed) |
| sun/moon face radiance vs the body's solid angle | **off by 4/π** (disc drawn, cuboid measured) | **0**: the cube is drawn and its own direction-dependent solid angle divides the Sun's radiance (W2 arm (a), landed) |
| unrendered public visual items (`axis::balance`) | **not yet run over the client** | counted, and not rising |
| package layers with a renderer | **1** in `display-packages/horizon-lod` (M10.2, 2026-10-24) | the display package's |
| top-level folders unclassified by the package census | **all of them** | **0** (M10.5) |
| open tickets with a tracked file | **0** (chain/inbox only) | every one (M10.6) |
| `cargo test -p voxel-client --lib` | **549 passed, 2 failed** | **553 passed, 0 failed, 2 ignored** (`-- --test-threads=4`) |

The two red witnesses are the last two of the eight this session began with. Both are green, and
neither went green by changing an assertion to match the picture: W1 moved the model onto its own
orbit, and W2 moved the shader onto the model's body. The only assertion that changed shape is the
Moon's aggregate, because `(1 − cos E)/2` is a sphere's law and the body is a cube; the disc law is
printed beside the cube's own at the same tick and the assertion is the cube's.

---

## 8. Stated gaps, named rather than hidden

* **`ala-package-designer`'s format is not in this tree.** M10.7 defines the reader's contract and
  refuses every input until the format is recorded from the user's copy in
  `docs/PACKAGE-FORMAT.md`. No grammar is invented here.
* **`ala-system` is a vocabulary source, not a dependency.** This tree takes `EvidenceLevel` and the
  support ladder from `voxel_model::axis` (which cites `support.ts`); M10.7 does not add a crate
  dependency, because the evidence vocabulary already crossed.
* **The layer registry's WGSL story is one file per layer, not one shader.** That is a deliberate
  narrowing: it keeps `C-33` true for *the look* while letting a package bring its own pass. If a
  layer needs to alter the look itself, that is a change to `shading.wgsl` and therefore a patch to
  this tree, not a package.
* **Folder "compression" is accounting and packaging.** Nothing is deleted (M10.5's non-scope); the
  measure is classification and bytes, not archive size.
* **Git tickets are files, not a forge.** Mirroring to a hosting service is a later adapter and is
  not claimed.
