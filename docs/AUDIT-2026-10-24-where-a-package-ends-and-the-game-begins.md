# Where a package ends and the game begins

**An audit of two questions: why the working directory has sixteen top-level folders, and why features
the Mount v2 package format is supposed to own are still implemented in the game's crates.**

Measured 2026-10-24 at `aef0971`, working tree as it stands (307 modified, 3 deleted, 431 untracked
entries, so paths and line numbers are the working tree's, not the commit's). Read-only: this
document was the only file written.

---

## 1. Why sixteen folders

The census table (`packages/census.tsv`, written by `voxel-census`) classifies the top level into
**16 folders, 0 unclassified, 2 declared exclusions**. They are not sixteen modules. Four are source
or product, four are record stores, two are documents, one is a reference tree, one is captured
output, three are agent harness, one is build configuration:

| folder | kind | files | bytes | what actually reads it |
|---|---|---|---|---|
| `crates/` | source | 414 | 10.9 MB | cargo; **23 workspace members** |
| `display-packages/` | source | 20 | 51 KB | `build.rs`, `voxel-layers`, `mount` (8 path joins) |
| `packages/` | emit root | 527 | 130 KB | `filing`, `mount`, `notifications` (8 joins) |
| `vendor/` | source | 10 | 86 KB | cargo, through `Cargo.toml:79` `[patch.crates-io]` |
| `patches/` | records | 74 | 752 KB | `patch`, `census`, `mcp` (12 joins) |
| `conformance/` | records | 3 | 21 KB | conformance pin tests (8 joins) |
| `chains/` | records | 1 | 1.5 MB | `filing::CHAIN_DIR`, `season::CHAIN_DIR` — **declared twice** |
| `intake/` | records | 1 | 1.9 KB | **nothing** — one `roots.tsv` |
| `architecture/` | documents | 1 | 22 KB | **nothing** — cited in comments only |
| `docs/` | documents | 295 | 41 MB | by file name: ledger, plan, baselines |
| `clean-room/` | reference | 1,012 | 144 MB | **nothing** — cited in comments only |
| `logs/` | output | 1,928 | 451 MB | `laya`, `notifications`, `mount`, tests |
| `.agents/` | harness | 343 | 1.5 MB | the agent runtime, not the build |
| `.freebuff/` | harness | 6 | 278 KB | the desktop's own files |
| `.superpowers/` | harness | 2 | 4 KB | a plan directory |
| `.cargo/` | build | 1 | 6.9 KB | cargo's own config |

(`.git` and `target/` are declared exclusions in `census::folders::EXCLUDED`, not kinds: they move on
every git or cargo command.)

**The reason there are sixteen is that the method this tree follows requires every claim to have a
path.** A patch record has to sit somewhere (`patches/`, 74 records); the event chain has to be one
append-only file (`chains/`, 1.5 MB); conformance pins have to be compared against something
(`conformance/`); the design's own diagram is `architecture/workspace.dsl`; the reference tree the
project was written against is `clean-room/` (144 MB); the ledger, plan and every baseline reading are
`docs/` (41 MB). Each is a *store*, and a store cannot be a function. That is why the count grew with
the campaign rather than with the design.

The honest cost, visible in the table: **three folders are never read by anything** (`architecture/`,
`intake/`, `clean-room/` — cited only in comments), and a fourth (`vendor/`) is read by cargo and by
no Rust file; **three hold exactly one file each** (`architecture/workspace.dsl`, `intake/roots.tsv`,
`chains/workspace.chain`); and three are agent tooling rather than project content. `logs/` is 451 MB
— 69% of the tree's bytes — for 1,928 captured runs.

The history explains the shape: **13 commits, three of them whole-tree checkpoints** ("Checkpoint the
working tree as it stands", "Checkpoint the cloud field and its octree geometry", "BACKUP
pre-flicker-rewrite") and one titled "one". Nothing in the history ever paid for a pass that moved a
concern *out* of a folder; the campaign added, then checkpointed. Folder count is what that deposits.

---

## 2. Why the package format's features live in the game's crates

### 2.1 The boundary as it stands

`voxel-package` (29,523 lines over 20 top-level modules plus a `census/` submodule) owns the
**nouns**: `recipe` (1,998), `compiler` (1,906), `filing` (1,127), `layer` (1,182), `policy` (634).
`voxel-client` (50,966 lines) and `shploingus` (`main.rs`, 3,951) own the **verbs**: `shell/` alone is
18,651 lines and `world.rs` is 2,938 — 21,589 lines, 42% of the client, interpreting packages.

| capability the package format declares | where it is implemented |
|---|---|
| `package.cells` — which drawing the package is a package of | **`voxel-package/src/cells.rs`** — constants, `Declaration` and `declare`, moved out of `voxel-client/src/shell/cells.rs`, which now re-exports them (see §3, done) |
| load a package: verify, read recipe, adopt arrangement | **`voxel-client/src/shell/mount.rs:2358`** `load_package` |
| watch the folder for a package that arrives mid-run | **`mount.rs:2444`** `watch_packages` |
| the tick a package advances on | **`mount.rs:1258`** `set_tick`; adoption path in the same file |
| hand the board's tickets to the agent | **`mount.rs:2081`** |
| draw a layer (`shploingus.layer/1`) | **nowhere in the game**: `grep voxel_package::layer` outside `voxel-package` returns **zero hits**; `world.rs`'s `WorldPipeline` (`:87`) has no registration surface |
| windows, rows, console bar, drop, changes, notifications | `window.rs` 1,704, `arrangement.rs` 1,594, `console.rs` 1,201, `notifications.rs` 758, `changes.rs` 720, `drop.rs` 470, `api.rs` 433, `theme.rs` 420 — **all in `voxel-client/src/shell/`** |

`mount.rs` alone is 7,403 lines (68 `pub fn`; tests start at 4,310). The format's own code is 6,847
lines; its interpreter in the game is 21,589. **The interpreter is three times the size of the thing
it interprets.**

### 2.2 The five causes, in order of weight

**1. The format stops at `Loaded`.** `voxel_package::recipe` ends at a parsed, compiled document
(`recipe::Loaded`); everything that *acts* on one needs a window, a GPU or a chain. Mount v2's plan
put the loader in the client explicitly — `docs/PLAN-2026-10-23-the-one-rig.md:680` (M10.3) names
`crates/voxel-client/src/shell/mount.rs` as the file. So the split is not drift; it was decided.
The cost is that the decision was never revisited as that one file grew to 7,403 lines.

**2. The shell is *declared* as data but *drawn* by widgets.** H-9.7/H-9.8 in `docs/HANDOFF.md` say
windows are mounted data and the HUD is pinned shell windows; `shploingus.recipe/3` carries the
windows and rows. But the reading of those rows happens in `shell/window.rs` and `shell/arrangement.rs`
because that is where the renderer is. Each new declaration therefore lands on the verb side: the
newest one, `package.cells`, was a *format header* parsed by `voxel-client::shell::cells`, not by
`voxel-package` — the cleanest single proof of the pattern, and **the seam this audit's first fix
closed**: it now lives in `voxel-package/src/cells.rs` and the client re-exports it.

**3. A closed schema (`R-64.1`) turns every capability into a sibling artifact whose reader is written
where it is needed.** The layer grammar was correctly put in `voxel-package::layer` (M10.1) — and its
consumer was supposed to be `WorldPipeline`. That consumer was never built, so the grammar has **zero
users** outside its own crate and 11 tests. Meanwhile the client grew a reader for `package.cells`
that the format crate does not have. **Each half of the format is missing the other's feature**: the
package crate holds an unwired grammar, the game crate holds an unowned header.

**4. `voxel-package` is not a format crate; it is a governance crate with a recipe module.** Census
(2,740), oracle (2,615), decision (2,122), recipe (1,998), compiler (1,906), compendium (1,606),
space (1,431), patch (1,420). There was never an obvious "package runtime" home, so the first caller
— the mount — became the owner by default. This is the structural reason; the others are its symptoms.

**5. The repository is three bulk checkpoints of a long campaign.** With no feature-level history, no
commit ever had to justify a boundary, and no pass was ever scoped to move one. `display-packages/README.md`'s
own "Not here yet" section is the standing ledger of that: `claims.shpkg` and
`Mount::load_display_package` are both owed, and both are format work currently blocked on client code.

### 2.3 What this costs, concretely

- ~~A package declaration (`package.cells`) cannot be validated without the client crate, so
  `voxel-layers` and any non-GPU reader cannot see it.~~ **Closed:** the declaration is
  `voxel_package::cells`, so a GPU-free reader can check it; `voxel-layers` still does not call it,
  because M10.3 did not ask it to.
- A layer can be declared, compiled, tested and rendered as a recipe — and no game code can draw it.
  The M10.1 acceptance ("a fixture package with one layer compiles, loads, **renders**") is
  unachievable without touching `voxel-client`.
- Any test of the mount path is a test of a 7,403-line file with a 3,100-line suite inside it, so a
  reader looking for "what does mounting do" has to walk 68 public methods to find out.
- The same question ("what does this package say?") has two answer sites: `voxel-package` for
  everything except cells and mounting, `voxel-client` for those two.
- The format's path vocabulary already has two owners: `CHAIN_DIR` is declared in both
  `voxel-package/src/filing.rs:56` and `voxel-model/src/civilian/gov/season.rs:50`. Nothing reads both
  in one place, so a change to the chain's location is a two-crate edit found by grep, not by the
  compiler.

---

## 3. What would move the boundary without inventing scope

Ordered so each step is provable on its own and none requires a GPU:

1. **Move `package.cells` into `voxel-package`** (`cells` or `declaration` module): the constants, the
   header refusal, and the reader. `voxel_client::shell::cells::declare` becomes a re-export. No
   behaviour change; one header, one parser. **Done 2026-10-24:** `voxel-package/src/cells.rs` owns
   `CellVersion`, `CellRefusal`, `Declaration`, `declare` and the three constants; the client
   re-exports them, `Mount::load_package` is unchanged, the refusal text and file/line are unchanged,
   and `voxel-layers -- display-packages` still hashes to `3f7c9230…` (89 lines). Recorded in
   PATCH-074's execution.
2. **Give `WorldPipeline` the registration surface M10.1 promised** and consume
   `voxel_package::layer` from it. That removes the zero-consumer grammar and makes "a package can
   declare a layer the game draws" testable headlessly (registration + refusal), leaving only the
   pixels to the client.
3. **Then decide the loader's home on the record**, not by default: either a `voxel-package-mount`
   crate that takes a `Renderer` trait, or an explicit rule in the plan — *a package declaration's
   reader lives in `voxel-package`; only the drawing lives in the client*. Without one of those two,
   the next declaration lands in `mount.rs` again.

What not to move: `window.rs`, `arrangement.rs`, `console.rs` are the renderer. Their size is the
price of a GPU shell, not a boundary defect. The defect is the **readers**, not the drawing.

---

## 4. Sources

- `packages/census.tsv` — the folder table (generated 2026-10-24 by `voxel-census`).
- `docs/PLAN-2026-10-23-the-one-rig.md:537-726` — M10.0–M10.4, including the "today/where" table and
  the "Not landed" notes for M10.1 and M10.2.
- `display-packages/README.md` — the display-package folder, and "Not here yet".
- `docs/HANDOFF.md:1-40` — H-9.7/H-9.8 (windows are mounted data), the five-part composition, the
  documents-are-the-declaration rule.
- `crates/voxel-client/src/shell/mount.rs:1258, 2081, 2358, 2444` — the verbs.
- `crates/voxel-package/src/cells.rs` — the package declaration's owner since 2026-10-24;
  `crates/voxel-client/src/shell/cells.rs` re-exports it beside the raster.
- `crates/voxel-client/src/world.rs:87` — `WorldPipeline`, with no layer registration surface.
- `crates/voxel-package/src/{recipe,compiler,filing,layer,policy}.rs` — the nouns and their sizes.
- `git log --pretty='%h %ad %s'` — 13 commits, three whole-tree checkpoints.

---

## 5. The other half of this seam: the light, not the package

The same "one owner" question the package format had is being asked of the material light, and the
answer is the same shape. **A frame's light is one law, and it belongs to the atmosphere, not to
whichever consumer read it last.**

- `shading.wgsl::fs_main` now samples two streamed numbers: the **sky** (`rig.ambient`, which
  `skylight::daylight_of` fills from the atmosphere's dome — four horizon azimuths and the zenith at
  `AMBIENT_FROM_HORIZON`, at the frame's `EXPOSURE · grade`) and the **beam** (`rig.lights[0]`, the
  solver's Sun/Moon at the same exposure, gated by the frame's shadow map). The retired path was the
  rig's ambient plus two fixture lights, and the first replacement — `π · sky_radiance_rgb(n)` per
  fragment — was a third reading, not the model's: it sampled the zenith for up-facing surfaces.
- **The measured gap the boundary decision rests on.** The model's sky is single-scattering:
  `0.591` at the horizon against `3.9e-5` at the zenith at the crossing, and the field's five-ray
  stencil mean therefore reads ~5000× darker than the rig's two-ray ambient. That is why the field's
  per-corner bands cannot be the material light yet, and why the sky is streamed once per frame
  instead of integrated per fragment: the integral is frame-global, the per-fragment form costs
  `BAND_COUNT` `exp` evaluations against the shadow map's one tap, and it makes horizontal ground
  black. **Decided: the sky is a frame uniform; the per-corner band is the field's measurement and
  nothing else.** Left as named debt: folding `WorldLight`'s six bands into the one shader format so
  a chunk arrives with its light attached (the player's "one material format" direction), which
  needs multiple scattering (M3) before the two readings can be one.
- **What is recorded but not moved here:** water/cloud material light (its GPU path still disagrees
  with `fluid::surface_cpu` by 5.9e-3 in `the_water_surface_reflects_the_sky_the_model_says_it_does`
  — the next pass), the trees' mob features, the shells `cells` format, LOD material rendering, and
  the 1/32-profile cuboid arithmetic shapers. The package-format half of this audit (`package.cells`)
  is unchanged by this pass; `voxel-layers` still emits the `3f7c9230…` bytes.

---

## 6. Water: what the light law had, and what the material format still owes

- **The 32× error was a dropped term, not a light model.** The shader had omitted
  `npr::shade`'s ambient gain `(1 + transmission·(1−depth))·(1−open)`; with water's
  `transmission = 1` and `opaque = 0`, the body vanished and the GPU read `1.70e-4` where the model
  read `5.50e-3`. Restored, the two agree to `5.5e-10`.
- **Water's LOD reading is wired and inert.** The boundary Fresnel now takes
  `lod_reflectance_delta` (the same function every surface uses), CPU and GPU from one law. It
  evaluates to `0` on this world because the registry gives `WATER` a reflectance gain of zero. The
  boundary *direction* is decided (LOD term on the boundary, not a separate water constant), the
  number is the material row's to state.
- **Three absences are the real gap, named not approximated:** (a) the reflected ray reads the
  atmosphere only, so clouds cannot appear in the water; (b) `fluidics` and `fluid_state` are not
  ticked by the game, so there is no piping or ticking of water; (c) there is no 1/32 *profile
  shaper* — `FILL_SIDE = 32` is state granularity, not a surface law — and no tick axis in the
  shader uniform. A flat sea is the correct picture of that absence.

---

## 7. The streaming first-fill: why terrain waited for a reload, and what the light actually does

- **The terrain that would not draw on its first streamed load was a buffer fill, not a mesh.** In
  `WorldPipeline::write_quads`, a resident scene (what the game hands the frame) lists only the
  ranges that changed. When the frame also *allocated* the record buffer — first fill, or a
  doubling — writing those ranges into a brand-new storage buffer left every other rectangle
  undefined, and no later resident frame re-sent them. Measured on the shipped `Renderer::render`
  path with a 48-record world whose resident frame lists only its newest 8 records: the old rule
  wrote **8 of 48**; `grew || !quad_records_filled` writes **48 of 48**, and that first frame is
  **pixel-identical to a fresh rebuild** (0 of 1089 pixels differ). This is the `grew ⇒ everything`
  rule the corner path already stated, now stated once on both.
- **The light is one streamed sample and every chunk reads it, so nothing is left to resample.**
  Seven normals, drawn as one chunk, eight chunks and 768 chunks at four distances, with the
  per-corner bands zeroed and absurd: every reading is **bit-identical** (`chunk: 0e0`,
  `bands: 0e0`), GPU against the crate within `3.9e-8`. `write_rig` is written on the atmosphere
  cadence and holds a frame-global dome mean and one beam, so a near chunk and a horizon chunk take
  the same light from the same uniform on the same frame. The vertex format's bands are still
  carried and still are **not a light**.
- **What the screenshot still shows, and why it is a different seam.** The beam term carries
  `npr::shade`'s half-Lambert wrap, so a face turned away from the Sun keeps half the beam
  (`0.5 ≤ wrap ≤ 1`); and the ambient is the dome's frame-global mean, which has no per-face
  occlusion. Both are the declared model the golden tests assert; neither is a streaming or upload
  defect, and "fixing" them with a second light law in the water or LOD branch is exactly what this
  pass removed. The dark near ground in the player's frame is the low Sun's cast shadow through
  `sun_visibility`, which is a real per-fragment map tap and does change with distance.
- **Boundary named.** Where a resident upload's ranges conflicted with a frame that had just
  allocated the buffer, the reading chosen is **the whole array**: a buffer that holds nothing owes
  everything, and a frame must draw the world it was given. Nothing about the resident path's
  no-upload steady state changes — only the frame that allocated.
- **Not touched, noted for their own passes:** the shells `cells` format (still the cell format),
  clouds on a reflected ray (the water reflection still reads the atmosphere only), and the
  trees/mob features all stand as recorded in §5–§6.

---

## 8. The shell: the cell format may only address a window's content space

- **The raster was drawing the whole shell.** `cells::surface`/`raster` took every composed element
  — bar, title bars, borders, panel chrome, the console's frame — and snapped it to the 8 × 16 grid,
  so the cell format was the *interface's* drawing and not a window's. That is now forbidden at the
  seam: version 1 rasters **only** elements whose whole rect lies inside a declared window content
  rect. Everything else is passed through as composed, in its own z-position; with no content space
  the shell draws its elements and no cells at all.
- **The mask is the drawing's own arithmetic.** `Mount::content_space()` builds it from
  `window_body`, the same function the window's drawing and the pane hit-tests use, so the masked
  area cannot drift from the area a window actually occupies. Closed and minimized windows declare
  nothing; a maximized window declares the body of the rect it is drawn at.
- **Evidence.** `the_raster_draws_only_inside_a_window_content_space`: an identical surface and
  paint outside the content space survives the raster unchanged and first in the list, the one inside
  is rasterised, nothing is painted above the content space, and an empty content space returns the
  composed list. `shell::cells::tests` 15 passed; `cell_version_refusal` 2 passed; workspace
  `--all-targets` check exit 0.
