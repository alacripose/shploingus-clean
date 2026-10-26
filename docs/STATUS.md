# STATUS — what is measured, and what is not

> **Every number here was measured in this repository, and the command is given.**
> Nothing is inherited from another tree and nothing is recalled. Where something
> is absent, it says absent.

## 1. The measured state

| Measure | Value | Command |
|---|---|---|
| Workspace members | **12** — `voxel-model`, `voxel-noise`, `voxel-shapers`, `voxel-mesh`, `voxel-stream`, `voxel-instrument`, `voxel-appearance`, `voxel-work`, `voxel-client`, `voxel-ui`, `voxel-package`, and the `shploingus` bin | `cat Cargo.toml` |
| Tests | **859 passing, 0 failing, 8 ignored** — 192 in `voxel-model`, 223 in `voxel-package`, 60 in `voxel-client`, 38 in `voxel-ui` | `cargo test --offline` |
| The ignored tests | **8**, and none is a skipped test: 4 that need a **GPU adapter** (the shader-versus-crate comparison, the bar's band count, the shell chrome's legibility on pixels, and the shipped view), 2 settle-memory measurements, 1 pool-throughput measurement, and **1 terrain-relief measurement** (`terrain::tests::measure_the_relief_and_where_the_mountains_are`, C-46's seed-explicit one). Each names its own reason and prints its own numbers. The shipped-view test **failed on purpose** between C-38 and C-39, and passes now. **This row said 7 until C-51 measured 8** | `cargo test --offline --workspace -- --ignored --list` |
| Clippy | **clean at `-D warnings`** | `cargo clippy --offline --all-targets -- -D warnings` |
| External dependencies **named by a headless crate** | **1** — `sha2`, by `voxel-model` and no other. `voxel-client` names three more (`wgpu`, `pollster`, `glyphon`), each with its reason in `crates/voxel-client/Cargo.toml`; **the 12 members are not one dependency set** | `cat crates/*/Cargo.toml` |
| External crates in the resolved tree | `sha2` plus its own 7 (`digest`, `block-buffer`, `crypto-common`, `generic-array`, `typenum`, `cfg-if`, `cpufeatures`) — and the `wgpu` / `pollster` / `glyphon` closures, whose count is **owed**: `cargo tree --offline --edges normal` has not been re-run since the client landed, and a number nobody measured is not a number (C-51) | `cargo tree --offline --edges normal` |
| `unsafe` | **0 code occurrences**; the only matches are `#![forbid(unsafe_code)]` in all **12** crate roots and one doc comment in `voxel-model/src/lib.rs` | `grep -rn "unsafe" crates/` |
| Client layer | **built, offscreen** — `wgpu 30.0.1` renders a `Mesh` with the four Source 2 terms, and reads the pixels back. **No windowing dependency**: that is the bin's job | `ls crates/voxel-client/src/` |
| The window | **runs.** `shploingus` streams 605 chunks through the pool while drawing, with `SHPLOINGUS_WORKERS` setting the worker count so the F12 claim on screen can be falsified by hand. Measured: the settled digest is `121426b8018389e8` at **2** workers and `121426b8018389e8` at **15** | `SHPLOINGUS_WORKERS=2 target/debug/shploingus.exe` |
| **The world on screen** | **visible.** The shipped window renders from the shipped eye with **55.5 %** of its pixels above 0.01 luminance, `max 0.2012`, **10 distinct colours** — measured after the material record gained a sourced reflectance. Before that it was **0.0 %**, `max 0.0000`, 2 pixels: black, and correctly so (C-38, C-39) | `cargo test -p voxel-client the_shipped_view_is_not_a_black_screen -- --ignored --nocapture` |
| Materials that can be shaded | **7 of 7** derive a `base_color` — every value is a **sourced broadband albedo**, achromatic, with its range and its provenance in the value itself (C-39) | `cargo test -p voxel-client the_shading_report_counts_materials` |
| **The shell stage, split** | **shaping was 2.8 of its 3.0 ms `prepare`** (45 runs, 18 bands) and is now paid per changed run: the stage fell **6.5–6.8 → 4.2–4.6 ms idle** and **7.5–7.6 → 4.4 ms under eight CPU hogs**, the frame **15.0 → 10.3–11.3 ms idle and 19 → 13 ms / 51 → 77 fps loaded**. Six offscreen shell renders are **pixel-identical** before and after, and a settled frame shapes nothing (`shaped_runs` 0) and touches no buffer (`settled_frames` 1) | `BENCH=1 cargo run -p voxel-client --example shell-shot`; `cargo test -p voxel-client --test settled_frame`; `SHPLOINGUS_FRAMES=1 target/debug/shploingus.exe` |
| **The atmosphere cadence, off the frame** | the cadence's derivation (the field's solver, the rig, the dome, the two bodies) cost **14.3–14.7 ms of `sky-light` and 4.2–4.3 ms of `sky-dome` on every cadence frame**, on a frame that otherwise draws in 10–11: a **34–44 ms** stall four times a second. It is now derived **one cadence ahead on its own thread** and adopted when the tick, the inputs and the eye are the ones the frame is about to use — **0.096 ms mean, 3.24 ms max**, 273 of 274 cadences adopted idle (the 274th is boot). The light tick no longer moves `SceneKey` at all while the field holds no measurement, which was every cadence frame rebuilding **byte-identical** corners. Settled frame **10.8–11.7 → 7.9–8.2 ms**, **91 → 123 fps**, **0 stalls** over 25 ms in the settled window against 5–6 per 90 frames, and the world digest is unchanged (`7c66a632336cc0a3`) | `ATMOSPHERE_LEDGER=1 SHPLOINGUS_FRAMES=1 target/debug/shploingus.exe`; `cargo test -p voxel-client --lib atmosphere::`; `cargo test -p voxel-client --lib an_empty_field` |
| **The world's own light, measured** | **the field answers, and the frame is lit by it.** The defect was two address laws in one path: the scene asked at `floor(p / 0.25 m)` while the field read those integers as metres, so every corner named a cell four chunks from its own and the binary marched **0** of the 1.5 M corners it rigged. One law now (`VoxelPos::of_point`), and the settled binary reports **82 207 cells held**, **225 984 of 228 700 corners (98.8 %) from the world's own light**, 2 716 (1.2 %) on the rig, **0 hops past a chunk not resident**, **8.4 ms / 117 fps / 0 stalls**, digest `7c66a632336cc0a3` unchanged. The static frame measures all **82 749** cells its corners name in 0.6 s and differs from the rig in **73.5 %** of its pixels; every build's corners are one to two field steps old, which the report states as `stale` rather than hiding | `cargo run --offline --release -p voxel-client --example light-shot`; `SHPLOINGUS_FRAMES=1 target/debug/shploingus.exe`; `cargo test -p voxel-client --lib the_cell_the_scene_asks` |
| Colour in the record | `Reflectance` (three exact channel ratios) plus `SourcedReflectance`, which makes a reflectance **impossible to carry without a source**; `Parameter::BaseColor` is `Availability::Sourced` — present per material, absent per material | `cargo test -p voxel-appearance a_material_with_no_reflectance_still_derives_no_colour` |
| Design system | **built** — `voxel-ui` with **zero dependencies**: the four-bucket `CONSTANTS` table (5 project extensions, pinned by name), a palette of **2 declared roles of 8**, the ink ladder, the contrast floor, and `Panel`/`Readout`/`Element` | `cargo test -p voxel-ui` |
| The HUD's own legibility | measured **on pixels**: the panel reads back `#505050` against the design's `#505050`, the brightest glyph pixel is **8.06 : 1** over the floor, and there are **166 distinct colours** inside the panel | `cargo test -p voxel-client the_hud_draws_legible_text -- --ignored --nocapture` |
| Package intake | **built** — `voxel-package`, 13 tests: integrity and identity are separate types because a checksum is not a signature, `Established` is deliberately not a `bool`, and every problem is collected rather than the first | `cargo test -p voxel-package` |
| The shader is checked against the crate | the WGSL transcription of `npr::shade` disagrees with it by **1.276e-7** at worst, over 8 probe normals — `f32`-versus-`f64` rounding and nothing else | `cargo test -p voxel-client -- --ignored --nocapture` |
| **The bar, on a real frame** | the same 256×256 frame is **2 bands / 3 distinct colours** with every input *derived* (C-27) and **8 of 8 bands / 9,087 colours** with an albedo *stated* — so the debt is now a number on a picture | same |
| The GPU this was measured on | `NVIDIA GeForce RTX 2070 SUPER (Vulkan)`; `wgpu` sees 5 adapters (Vulkan, DX12 ×2, a CPU fallback, GL) | `cargo test -p voxel-client measure_the_adapters -- --nocapture` |
| Work layer | **built** — the worker pool, the streaming window, and the budget, with **no dependency outside the workspace**: threads from `std::thread`, the queue from `std::sync` | `ls crates/voxel-work/src/` |
| The only crate that spawns a thread | `voxel-work` — `cargo tree -p voxel-work --depth 1` shows the 4 workspace crates and nothing else, so the pool cannot depend on a runtime | same |
| Shapers implemented | **15** — tectonics, fields, geology, terrain, hydrology, erosion, climate, atmosphere, weather, caves, substrate, ecology, delta, transition, earthworks | `ls crates/voxel-shapers/src/` |
| **Voxel shapers, two kinds × two levels** | **built** — `voxel-model::shaper`: a **physical** shaper changes occupancy and a **material** one cannot; the stored level is the 4×4×4 lattice and the addressed level is 256. Every shaper's `acts_at` is required, and a shaper that claims the addressed level and declares nothing is **refused by name** | `cargo test -p voxel-model shaper::` |
| The shape laws, not bitmaps | **built** — `voxel-model::profile`: surfaces of revolution with **exact** normals (a column's and a cap's), a stalagmite's `Da` law integrated from Szymczak et al. (2025), and `cross_section_radius`, which is the silhouette and not the base radius — using the base radius was a real bug in three call sites (C-44) | `cargo test -p voxel-model profile::` |
| The voxel shaper roster | **built** — `voxel-shapers::voxel`: `Slab`/`LoopCut` from a **parametric cut** in 256ths (`Band`, `Cut`, `Trim`), `Column`/`pillar`, `stalagmite` (the drip rate derives `Da`), `stalactite`, `cave_column`, `Boulder` (Zingg form × Powers roundness, bladed refused), `Flowstone`, `Coral` (six growth forms, branching as four bodies) | `cargo test -p voxel-shapers voxel::` |
| A cut's plane is exact | a cut at `96/256` keeps cells 0 and 1 and declares **−0.5 subvoxels** on cell 1's own top face — the residual *is* the displacement, and a band thinner than half a cell is **refused by name** | `cargo test -p voxel-shapers the_cut_s_residual` |
| **The addressed level reaches geometry** | **built** — `voxel-mesh::refine`: a displaced quad is subdivided at a declared rate, **split at block boundaries** (a profile is a law about a block), and emitted as triangles with the law's own normal at each vertex. Measured: a `RoundColumn` block face at 4 samples per block costs **32 triangles** against **2.0** undisplaced; the addressed level's own resolution is priced at **131,072** triangles for one block face, and a rate past the cap is **refused** rather than lowered | `cargo test -p voxel-mesh refine:: -- --nocapture` |
| The two bounds, distinguished | a **trim** may not exceed half a subvoxel (it would cross the cell boundary and the cell next door would move — a carve); a **profile** is bounded by the circle inscribed in its block, **2.0** subvoxels. Sharing one number made the largest cap a block can hold undeclarable (C-44) | `cargo test -p voxel-model a_profile_displaces_with_its_own_shape` |
| Mesh layer | **built** — canonical sparse voxel octree, greedy mesh at 128³, DDA raycast | `ls crates/voxel-mesh/src/` |
| Instrument layer | **built** — one sensor concept, a min/max band **per item**, one weighing gate per boundary, a virtual gauge wall | `ls crates/voxel-instrument/src/` |
| A `MaterialId` **is** an item | mass is `count × the admitted density` in exact rationals, so the *audited to the gram* claim is the mass machinery's and not the instrument's | `cargo test -p voxel-instrument` |
| Appearance layer | **built** — OpenPBR inputs derived from block truth, the four Source 2 terms over them, the 256× lattice addressed, the four-bucket rule as a type | `ls crates/voxel-appearance/src/` |
| The 256× lattice stores nothing | `size_of::<DetailLattice>() == size_of::<u64>()`; the dense alternative is **35,184,372,088,832** samples (**2⁴⁵**) for one chunk | `cargo test -p voxel-appearance detail::tests::the_dense_figures` |
| The bar, measured | a **white-factor placeholder** measures **1** band and **fails**; a stated OpenPBR + Source 2 surface measures **8 of 8** bands (luminance 0.524 … 1.311, all four terms drawn) and **passes** | `cargo test -p voxel-appearance response:: -- --nocapture` |
| The measurement saturates | 8 of 8 bands is **≥8**, not 8: the count is capped by its own declared resolution, so it proves the bar's three and resolves nothing finer | same |
| Merging, measured | 34,816 visible subvoxel faces → **6 quads** (a 32×32×1 slab) | `cargo test -p voxel-mesh a_solid_slab` |
| A generated chunk, meshed | **2,060 quads from 665,152 faces** (merge factor **323**) across 27 chunks | `cargo test -p voxel-stream measure_the_cost -- --ignored --nocapture` |
| Generate + mesh, measured | **11.2 ms** + **20.2 ms** (chunk with geometry) / **1.17 ms** (chunk without) | same |
| Optimisation evidence | the resident digest is **identical** before and after a 4.4× speed-up | `docs/LEDGER.md`, C-23/C-24 |
| Removed-row check | `cargo test` runs offline with the registry cache only | — |
| Memory, as a **type** | `Chunk::bytes`, `Mesh::bytes`, `SparseVoxelOctree::heap_bytes`, `Resident::bytes` — every count **re-walks** the structure rather than reading a cached counter | `cargo test -p voxel-stream accounting_counts_capacity_and_the_allocator_s_share` |
| A chunk with terrain in it, measured | **90,481 B** mean both (octree **89,153 B** / mesh **1,327 B**), over 147 chunks around the surface | `cargo test -p voxel-stream measure_the_memory -- --ignored --nocapture` |
| **Which half is the memory** | the octree is **182×** the mesh at the median (93,344 B vs 512 B); over 147 settled chunks the meshes are **1.5%** of the 13.3 MB held | same |
| The streaming engine's own footprint | **0.16 MB resident** for a 147-chunk window — it holds meshes and **no octrees** | `cargo test -p voxel-work measure_the_throughput -- --ignored --nocapture` |
| A mesher's scratch | **4.8 MB** per worker (`(136³ + 128²) * size_of::<MaterialId>()`), **held only while the queue has work**: 72.4 MB at fifteen workers mid-stream, **0 B** at rest — a settled 605-chunk window is building nothing, so it holds nothing | same |
| Throughput, measured | **12 chunks/s** at 1 worker → **71 chunks/s** at 8 (**5.9×**, 73% efficient, sub-linear because the workload is bandwidth-bound) | same |
| **F12, measured at the engine** | one digest `db4e8937…` across **1, 2, 4 and 8 workers** — the worker count is a knob and does not touch output | same |
| What "generator-only" costs | **84.2 ms/chunk** through the pool's `neighborhood` vs **31 ms** for generate + mesh — the pool is slower per chunk and faster per world, and both numbers are stated rather than traded silently | same, `docs/LEDGER.md` C-31 |

## 2. What is **not** built yet, in the order it is needed

A playable world needed five things this repository did not have. Three are built.

1. **The worker pool — built.** `voxel-work` holds the `Pool`, the box-shaped
   `ChunkWindow`, the `Streamer` and the `Budget`. F12 is measured at the engine
   (one digest across 1/2/4/8 workers) and the render thread never does this work:
   the whole point of the pool is that meshing happens somewhere the frame loop is
   not.
2. **`voxel-client` — built, and it draws.** `Scene::build` turns meshes into
   camera-relative vertices, `shading::table` derives a row per admitted material,
   `Renderer` shades them on the GPU, and `Frame::response` measures the result.
   Measured on a real frame: **2 bands derived, 8 of 8 stated** (C-34).
3. **The `shploingus` bin — built, and it runs.** A `winit` window, free flight,
   and the streamer pumped every frame. It is the only crate with a windowing
   dependency. The escape shell is the one thing here that is not: free flight
   means no gravity and no collision, said plainly rather than discovered.
4. **The shell — built, and it runs over the world (C-85).** The upper bar with
   the package's six tabs and its `console` entry, sixteen declared windows with
   declared geometry, move / dock / undock / minimize / maximize / close, focus
   and z-order, tab menus offering the windows filed under them, and the four
   pinned windows that stay while the shell is hidden. `Escape`/`F1` moves
   between the two layers. What is **not** built is named in TODO §7.1 rather
   than implied: the notification queue, the ticket window, the change view, and
   the debug lanes.
4. **The client UI and the package manager — built.** `voxel-ui` is the MD1/HIG
   design system as types (C-35) and `voxel-package` is content intake that
   refuses a package it cannot validate. The HUD is **drawn** on screen — four
   panels, with `glyphon` and a rounded-rect panel pipeline (C-37). What is not
   drawn is any *interactive* control, so the 48 dp target floor and the 8 dp
   separation rule are enforced by tests and have no consumer yet: a debt, named.
5. **Appearance rows — built, and the blocking row is filled.** `base_color` was
   the row that made the world black; it is now a sourced physical property in the
   material record (C-39). What is left is **lighting and sky**: the world is
   visible and *dim* (mean luminance 0.00998) because the standard rig's ambient is
   a fixture, no material derives `specular_weight`, and the sky is the transparent
   clear colour rather than a sky model.

The fourth of the paragraph that follows is **built** and is measured above:
`voxel-appearance`, the OpenPBR inputs plus the Valve Source-2 layer plus the
256× lattice's evaluation.

**The one thing that gates the client, and it is not a code problem.** `C:` has
**~300 MB** free of 465 GB. `F:` has **57 GB**. A `wgpu` + `naga` + `winit` debug
build tree is on the order of 1–2 GB, so the client's build tree has to live on
`F:` (via `CARGO_TARGET_DIR` or `[build] target-dir`) or the build stops at
"no space left on device". Nothing in this repository needs to be deleted to make
room, and nothing has been.

**It has now been seen, the report was "black", and the report was right.** A
person looked at the running window and saw no world, which is the first
observation in this repository's history that came from an eye rather than from a
test — and it was worth more than the test, because the test was measuring the
*offscreen* frame and passing. `the_shipped_view_is_not_a_black_screen` now renders
the shipped world from the shipped eye; it read back **two** distinct pixels over
921,600 (C-38), and after the material record gained a sourced reflectance the same
test on the same world reads **55.5 % of pixels showing the world, 10 distinct
colours, `max 0.2012`** (C-39). Both numbers are in the table above, with the
command, and the assertion that separates them is the one a player stated.

`docs/APPEARANCE.md` still claims **S0**, and this round **did not change it**,
for two reasons rather than one. The first is that the document is the user's.
The second is that "rendered" here means *rendered into a texture and read back
by a test*: there is no window, and **no human being has looked at a picture of
this world**, which is a different claim from "no pixels exist" and a weaker one.
The measurement is real; the stage call is the user's, and it is now a call they
can make with numbers in hand.

**What the picture costs, row by row.** Because OpenPBR's values are derived
rather than invented (C-27), an admitted rock derives **7 of its 12 parameters**
and the other 5 are **named absent**. The first row used to be one of them, and
filling it is what turned a black screen into a world (C-38, C-39):

| Absent, and named | Why the model cannot supply it |
|---|---|
| ~~`base_color`~~ | **filled** — the record now carries a sourced reflectance (C-39) |
| `specular_ior` | the model records what a material is made of, not how it bends light |
| `specular_weight` | no admitted dielectric F0 |
| `coat_roughness` | the model says a face is wet, not how the film is textured |
| `transmission_depth` | the model says whether a material transmits, not how far |
| `subsurface_weight` | no subsurface term for soil or snow |

`cargo test -p voxel-appearance openpbr::tests::a_rock_derives_exactly_seven`
enumerates them per material, and `..._the_absent_parameters_are_exactly` holds
that list to the rule rather than to itself. **So the picture that waits is not
"appearance is blocked" but "five rows of content are owed"** — and the shading
path those rows feed is already tested, so admitting them changes a table and not
an architecture. The two that matter most for a *look* are `specular_weight`
(without it nothing catches a highlight except the rim) and the sky, which is not
one of the twelve at all and is still the clear colour.

## 3. The interpretations the user has now ruled on

Both were inside the area the user reserved (*how the game looks*), so free agency
never covered them; both were put to the user as questions, and a third question
was asked in the same round because its answer moves code:

* **C-2 — confirmed.** "256× sub-subvoxels" is a **256× addressed procedural
  detail lattice**, never stored densely. Same round, the dense figure was found
  to be **low by 64×** in `APPEARANCE.md` §2 ("16,777,216 per voxel" vs per
  *subvoxel*): the honest figure is **2⁴⁵ = 35,184,372,088,832 per chunk**, which
  is a stronger argument for the decision than the number it replaces
  (`docs/LEDGER.md` C-28).
* **C-3 — confirmed.** NPR is **Valve's Source 2 shading model** applied over
  OpenPBR, measured by the FMA bar's *≥3 distinct luminance bands*.
* **Where OpenPBR's values come from — ruled.** **Derive from the material's own
  physics; refuse the rest.** §2 above is the consequence, measured.
* **How colour enters the record — ruled, and built.** Asked as three options
after a player reported a black window, the answer was **a sourced physical
reflectance**: reflectance becomes a property of the material record, carried
with its source, sourced from published mineral data — not a default, and not a
value chosen by the agent. The mechanism and the provenance are built; the seven
numbers are in `docs/LEDGER.md` C-39 and are **put to the user for approval**,
and they are one-line edits in `voxel-shapers` when they change.

## 4. Every shaper names its source

`docs/SHAPERS.md` is the contract and this crate is the implementation of it. There
is no shaper whose behaviour is "some noise": each is a paper, and three of them
(tectonics, fields, geology) sit on the two layers the old record called *"the
largest evidence gap"* — built from first principles plus primary literature,
which is what that record's own ruling says to do.

## 5. The one rule, and where it is enforced

**Refuse, never default.** It is a type (`voxel_model::Refusal`, which collects
**every** refusal rather than the first), it is a test in each module, and it is the
reason the material registry holds eight materials instead of a plausible-looking
filler set: an unadmitted material is a refusal, not a grey cube.

It is also why appearance has a **named absence** rather than a grey default. A
parameter the model does not record is `None` or `Colour::Unadmitted` — never
`0.0`, because a zero is a value a caller can believe. The one constant in the
appearance crate with no source in the record is the **band resolution**, and a
test counts the extensions so the debt cannot grow silently.
