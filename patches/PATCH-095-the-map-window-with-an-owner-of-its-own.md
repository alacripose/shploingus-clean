---
patch_id: PATCH-095
title: The map window with an owner of its own — state, verbs, rects and drawing out of the mount
status: applied
operation: refactor
target: voxel-client/shell
base_ref: working tree at 2026-10-27, after PATCH-094
dependencies: [PATCH-090, PATCH-091, PATCH-092, PATCH-093, PATCH-094]
source: user-directed-mission
created_at: 2026-10-27
approval: "user-directed 2026-10-27 — 'Give the map window one owner of its own: the map's state, verbs and rects move together out of mount.rs behind the small surface the app and the capture already call … keep the public behavior identical … While you are in the live run, also drive the wheel over the ground and over the gutter to prove the new gutter scroll actually moves the declared controls into view, and state the final-tree result of the whole voxel-client library suite'"
live_mutation: false
---

# Four passes of map had landed in the file that owns windows

`mount.rs` owns the shell: which windows exist, where they are, what the pointer is doing, and the
order the layer composes in. PATCH-090, 091, 092 and 094 each added to that file the state *of one
particular window* — a camera, a survey, a selection, a cursor, a hammer, a staged stroke, a gutter
offset — plus its verbs, its rectangles and half its drawing. The four passes were not wrong
individually and the accumulated shape was: by PATCH-094 the map's behaviour was spread across
nine thousand lines about windows, and the next map defect would have had to be found there.

**The move is a change of address and nothing else.** Same verbs, same answers, same toolbar facts,
same minimap form, same hammer semantics, same rects to the device pixel — the run this patch records
prints the same numbers as [PATCH-094's](../patches/PATCH-094-the-hammer-in-the-map-window.md), which
is what makes the claim checkable rather than asserted.

# The structure now

| module | owns | does not |
|---|---|---|
| [`voxel-client/shell/map_window.rs`](crates/voxel-client/src/shell/map_window.rs) | **the map window**: `MapWindow` — the camera and whether it is placed, the survey, the selection, the cursor cell, `MapBrush`, the staged `MapEdit`, the gutter's scroll — its **rects** (`toolbar_rect`, `view_rect`, `controls_rect`, and `MAP_TOOLBAR_DP` / `MAP_CONTROLS_ROW_DP` / the gutter's width), its **facts**, its **verbs** (`action`: zoom, rotate, pitch, recentre, brush, mark, raise, lower, clear, apply), its **pointer** (`press`, `drag`, `release`, `wheel`, `hover`, `pointer_left`), its **observation** (`observe`) and its **drawing** (`ground_elements` with the rings, `toolbar_elements`, `controls_elements`, `controls`, `control_at`) — plus `MapDecl` (the frame's declaration, copied) and `MapChrome` (all it may read of the shell) | know a window id, a pin, a frame or a pointer route; select a package view; own a palette or a refusal list; or hold a second copy of anything |
| [`voxel-client/shell/mount.rs`](crates/voxel-client/src/shell/mount.rs) | the shell: `map_content` (where the window is, or `None` when closed — pins, placed frames, `window_body`), `map_chrome_shown` (a pinned map is a minimap), **`map_chrome`** — the one place the window is told about the shell — the pointer routing that decides a press or a wheel is the map's, the layer composition in `map_elements`, `select_feature` / `frame_feature` (the package's views), and the thin accessors the app, the capture and the tests already call | a camera, a brush, an edit, a scroll, or a rectangle of the map's own |
| [`voxel-client/map_view.rs`](crates/voxel-client/src/map_view.rs) | the drawing primitives and the world's geometry: `MapCamera` (project, pan, zoom, rotate, pitch, clamp, survey plan), `MapTerrain` (raster, picking, ground), `ViewFilter`, `MapBrush` / `MapEdit` (`columns_within`), the inks | the window, its chrome, or its verbs |
| [`voxel-client/map_survey.rs`](crates/voxel-client/src/map_survey.rs) | the world-facing half: the plan's samples, the residency/generator write over the law's field, the composed `MapTerrain`, the counts | a camera, a pointer or a rectangle |
| [`voxel-client/shell/webview.rs`](crates/voxel-client/src/shell/webview.rs) | the two control documents and their layout, including `Rendered::content_dp` — the column's own height against one scroll offset | where a control lives, or what a wheel means |
| [`shploingus/src/main.rs`](crates/shploingus/src/main.rs) | the frame loop, the world, and the scripted run: `map_observe`, `apply_map_edit`, and the verbs `map` / `hover` / `click` / **`wheel`** | a map rectangle, a brush, or a stroke |

```
Mount (windows, pointer, layout)
  │  map_chrome()  ─────────────▶ MapChrome { content, viewport, chrome, decl, feature, selected }
  │  routes the pointer ▶  MapWindow::press / drag / release / wheel / hover   (regions decided here)
  ▼
MapWindow  ──observe(world)──▶  MapSurvey ──▶ MapTerrain ──▶ MapCamera ──▶ voxel_ui Element
  └── action(verb) ──▶ the window's own answer, or None ──▶ Mount::select_feature (the package's views)
```

**One direction, one owner.** The mount resolves one `MapChrome` per question and hands it to the
window; the window never reaches back for more, and nothing outside it keeps a camera, a selection, a
brush, a staged stroke or a scroll. The rects are the window's own arithmetic, so the picture it draws
and the controls the pointer resolves against cannot disagree about where the window is. The two things
that are genuinely the shell's stay there: **where the window is** (which needs pins and the placed
frames) and **which package view is selected** (which the console's `plan` verb also speaks).

**`MapChrome` owns its values** rather than borrowing the mount's: a verb mutates the window while it
is still holding the answer to "where is my window, what does my frame declare", and a borrow would
have forbidden exactly the calls the split exists to make simple. It is a rectangle, a viewport, a
bool, six numbers and two strings.

# What the move cost, and what it did not

* **`Mount`'s surface is unchanged.** Every accessor the app, the capture fixture and the tests call
  is still there, one line long, and now says where the behaviour went. `Mount::map_chrome` is the
  only new name, and it is private.
* **The docs and the refusal texts are unchanged**: `the map toolbar refused to lay out: …` and
  `the map controls refused to lay out: …` are still the shell's own, one per part, which is why the
  mount still composes the window's three pieces rather than the window returning one blob.
* **The documentation moved with the code.** The reasons — the brush on the sample's column rather
  than the declared cell's name, the pinned map's missing chrome, the gutter's row grid and why the
  last step reaches the document's bottom, the ring order under the toolbar — are stated once, in
  `map_window.rs`, where the behaviour is. The mount's copies are one-line pointers. A doc comment
  left behind on a delegation would have been the same drift in prose.
* **One dead arm went.** `select_feature` held a second, unreachable `"prev"` arm below
  `"next" | "prev"`; the compiler said `unreachable pattern` on every build, and it is now one arm
  with `back` (behaviour identical: the arm could not run).

# The structure's own numbers

| file | before | after |
|---|---|---|
| `shell/mount.rs` | 9 434 lines | 9 090 lines, of which the map's surface is ~215 lines of one-line delegations |
| `shell/map_window.rs` | — | 768 lines (state, rects, facts, verbs, pointer, drawing, the module's own account of why) |

The extraction is small in lines because **the map's behaviour was already written well**: the
camera, the terrain, the survey, the brush and the renderer were separate modules with their own
tests. What was wrong was where the *window* was — the values that are one window's, spread through a
file about all of them. Nine thousand lines to eight hundred is the same shape the shell's other
modules already have (`titlebar.rs` 672, `cells.rs` 1 090, `window.rs` 1 704, `hud.rs` 260).

# The live proof

The running game, not the capture fixture: 36 verbs, exit 0, no panic, full record in
[`logs/mapwin-run.txt`](../logs/mapwin-run.txt); captures at the swapchain's own 1280 × 720 pixels in
[`target/mapwin9`](../target/mapwin9).

**The same numbers as the last pass**, which is what "behaviour identical" is measured by here:

| tick | the verb, in the run's words |
|---|---|
| 81030–81036 | `map zoom in` four times — `zoom 1`, `zoom 2`, `zoom 3`, `zoom 4` |
| 81040 | `hover 631,537 — the map's cell: Some((4, 13)) · the cursor says Some([16.0, 17.0, -48.0]) · the hammer's ground at Some([4.0, -60.0]) is Some(17.0)` |
| 81044 / 81046 | `brush 8 m`, `brush 16 m` — the ladder answering the width |
| 81050 | `marked 16 m at 4,-60` |
| 81052 … 81062 | `lift +1 m` … `lift +8 m` |
| 81054 | `apply 812 columns +8 m`, then the app's own line: `hammer: 812 columns raised 8 m — 6496 raised, 0 lowered, 0 refused, 0 with no surface` |
| 81058 | `hover 631,537 — the map's cell: Some((4, 15)) · the cursor says Some([16.0, 17.0, -48.0]) · the hammer's ground at Some([4.0, -60.0]) is Some(25.0)` |
| 81106 | `map-plan pinned`, then the run ended clean |

Identical to PATCH-094's run — same cell `(4, 13)`, same cursor `16/−48/17`, same **17.0 → 25.0 m**
under the stroke, same `812` columns and `6496 raised, 0 refused`, same cell moving `(4, 13) → (4, 15)`
with the ground.

**And the wheel, which the last pass could not drive.** The new `wheel <x> <y> <lines>` verb runs the
same two steps the window's `MouseWheel` runs — the pointer's position, then the mount's decision about
what a wheel there means — and reports *what the gutter now draws*, not the offset:

| tick | verb | what it answered |
|---|---|---|
| 81066 | `wheel -3 at 631,537` (the ground) | `the map claimed it: true` — the map went `zoom 4 → 3`, the gutter untouched at `0 dp of 220` |
| 81070 | `wheel +3 at 631,537` | `true` — back to `zoom 4` |
| 81074 | `wheel -2 at 400,200` (**outside** the window) | `the map claimed it: false`, and the map did not move — the routing claims only the window's own two regions |
| 81078 | `wheel +3 at 872,480` (the gutter) | `true` — `78 dp of 220`, and the column now draws `zoom out … plan grassland` where it drew `plan all … plan ocean` |
| 81080–81082 | `wheel +3` twice more | `156 dp of 220` — `plan desert, plan tundra` appear — then **`220 dp of 220`**: the column's end, drawing `mark, clear, apply, plan ocean … plan tundra, rotate left, rotate right, pitch up, pitch down, recenter` |
| 81084–81094 | `wheel +3` six more times | `220 dp of 220` every time — the end is the end, whatever the row grid says |
| 81098 | `wheel -40 at 872,480` | back to `0 dp of 220`, drawing the head of the column again |

At the swapchain's pixels: [scroll 0](../target/mapwin9-c0029.png) — the column ends at `Ocean`, which
is PATCH-094's defect; [78 dp](../target/zoom39.png) — the toolbar's readout identical and the column
scrolled under it; [220 dp](../target/zoom41b.png) — `Apply`, every biome, the whole TURN block and
**`Recentre`** drawn, which are the controls that were *declared and drawn nowhere* before the scroll
existed; [back at 0](../target/zoom49.png) — the head restored; [pinned](../target/mapwin9-c0053.png) —
no toolbar, no gutter, no ring, the ground in the whole content container.

# The witnesses

No test was added: this pass changes shape, and the tests it must satisfy are the ones that already
describe the behaviour. All of them are the shell's, the survey's or the view's own — the point of the
split is that they stay where they are.

| witness | where | what it proves |
|---|---|---|
| the map window end to end | `shell::mount::tests::the_map_composes_in_the_shells_layer_from_the_worlds_own_voxels` | the survey inside the window's container at the frame's declared camera, the ground from the world's own voxels, nothing outside the container |
| the body, the picking and the facts | `shell::mount::tests::the_map_body_draws_the_player_and_a_click_selects_the_cell_under_it` | the click selects what the pointer projected onto, the cursor fact names it, the brush stands on the sample's column |
| the verbs and their refusals | `shell::mount::tests::an_unknown_map_action_is_refused_as_an_action_and_an_unknown_view_as_a_view` | a typo in a verb is refused as an action, an unknown view as a view — the two owners of `map_action` still answer to their own question |
| the toolbar, the gutter and the pointer lanes | `shell::mount::tests::the_map_controls_draw_and_the_pointer_resolves_the_button_under_it` | every control drawn, a press resolving to `plan ocean`, the view selection reading back |
| the column's scroll | `shell::webview::tests::a_taller_document_is_drawn_from_the_offset_and_reaches_its_own_end` | no scrolled pass escapes the gutter’s rectangle, and every declared control is drawn at some offset |
| the hammer and the pinned form | `shell::mount::tests::the_map_hammer_stages_its_ring_and_a_pinned_map_draws_none_of_it` | the staged stroke, `columns_within`, one take, and the minimap wearing no ring |
| the world-facing half | `map_survey::tests::*` (12 passed) | the ground is the world's own voxels, the counts, and the stroke reading back through a resample |
| the drawing primitives | `map_view::tests::*` (11 passed) | the camera's projection and clamp, the picking, the brush's geometry and its rings |

# Checks run

| check | result |
|---|---|
| `cargo build -p voxel-client --lib` | clean; no warning names `shell/mount.rs`, `shell/map_window.rs` or `shell/mod.rs` (the crate's 254 remaining warnings are other files' `missing_docs`, unchanged by this pass) |
| `cargo build -p shploingus` | the app compiles with the new verb (exit 0) |
| `cargo build -p voxel-client --example shell-shot` (the capture fixture) | **exit 0** — the fixture compiles against the unmoved `Mount` surface, which is the one caller outside the app that the split had to keep whole |
| `cargo build -p voxel-client --examples` | **fails, and failed before this pass**: `examples/face-refined.rs` imports `voxel_shapers::gaia`, and there is no `gaia.rs` in `crates/voxel-shapers/src/` — a stale example, unrelated to the map window, that stops `--examples` before it reaches `shell-shot`. Recorded, not repaired: it is not this pass's subject and its repair means knowing what took `gaia`'s place |
| `cargo test -p voxel-client --lib -- shell:: map_view:: map_survey::` | **306 passed, 0 failed, 2 ignored** in 22.2 s ([`logs/mapwin-core-groups.txt`](../logs/mapwin-core-groups.txt)) — the three groups this pass must not disturb: `shell::` 283, `map_view::` 11, `map_survey::` 12 |
| `cargo test -p voxel-client --lib`, one process | **did not finish**: 102 tests reported and then 30+ minutes inside one slow group, two cores busy, no further output ([`logs/mapwin-suite.txt`](../logs/mapwin-suite.txt)) — where PATCH-094's two attempts stopped too. Killed; the library was closed group by group instead. The group it was sitting in is `world::`, named by the filters below |
| the same library, **group by group, to completion** (26 module filters, each under a 900 s `timeout`) | [`logs/mapwin-groups.txt`](../logs/mapwin-groups.txt) — every group green but two, and both of those are the machine: `renderer::` **21 passed, 1 failed**, and `world::` killed at 900 s with 9 of its 12 passed |
| what those two were | `renderer::tests::the_sun_is_drawn_as_the_cuboid_the_model_says` panicked with **`wgpu error: Out of Memory`** (`wgpu-30.0.1/src/backend/wgpu_core.rs:1619`) — the named form of the un-named flake PATCH-094 recorded for this group. The three `world::` tests still in flight at the timeout (`a_patched_record_buffer_draws_the_frame_an_encoded_one_draws`, `a_resident_scene_lands_whole_in_a_pipeline_that_has_just_allocated_its_records`, `a_scene_arriving_allocates_no_corner_buffer`) are the same class: GPU allocation under a machine already at 99 % on both volumes with other heavy processes running |
| those four tests, each alone and one at a time | **4 passed, 0 failed**, ~1.5 s each ([`logs/mapwin-flakes.txt`](../logs/mapwin-flakes.txt)) |
| `renderer::` and `world::` again as groups, `--test-threads=1` | **`renderer::` 22 passed, 0 failed** (130.2 s) and **`world::` 12 passed, 0 failed** (10.8 s) ([`logs/mapwin-serial.txt`](../logs/mapwin-serial.txt)) — the whole library is green on this tree; the two failures were GPU contention and nothing in the shell |
| the live game, 36 scripted verbs | every verb answered, the stroke `6496 raised, 0 refused`, the map's ground `17.0 → 25.0 m`, the wheel claimed on both regions and refused outside the window, the gutter scrolled to its end and back, `map-plan` pinned clean, exit 0 |
| `rustfmt --edition 2021` on the new module | applied; the tree was already formatted |

**The library holds 656 tests** (the `--list` total). All of them are green in the runs above: 306 in one
process for the three groups the mission names, the rest as 26 module filters, with `renderer::` and
`world::` re-run one test at a time because the parallel pass exhausted the GPU. **No failing test
remains attributable to this tree**, and none of the four that needed the re-run is in the shell, the
view or the survey.

# What this does not do

* **The minimap's size and interpolation** are untouched, as are **cuboid fog**, the **FPS/needs
  toasts** and the **inventory window** — later passes' questions.
* **The app's own shape is untouched**: `main.rs` still drives the shell through `Mount`'s accessors,
  with the one addition of the scripted `wheel` verb (a test surface, like `hover` and `click`, and
  the only way to drive a wheel at a live frame without a person at the trackpad).
* **`map_view.rs` (2 121 lines) and `map_survey.rs` (1 733) are unchanged.** They are large; they are
  also one subject each — the drawing primitives and the world-facing sampler — with the window's
  state no longer among them. Splitting them further is a different question from this one.
* **`main.rs` (5 513 lines) and `mount.rs` (9 090) are still the two large files.** The map is out of
  the mount's state; the mount's remaining bulk is the shell's window system, arrangement and console,
  which the next pass may have views about.

# Reasoning

The map window was not badly written; it was badly located. Every one of its values was reachable from
the file that owns every other window's chrome, so a defect in the map's rects could only be read
against nine thousand lines of pointer routing and layout, and a reader could not tell which of the
map's fifteen methods were about the window's state and which were about the shell around it.

The boundary chosen is the one the code already implied: the mount knows *where* the window is and
*which* package view is selected; the window knows *what it is*. The one thing that had to be handed
over rather than moved is the shell's own facts — the content rect, the pin, the frame's declaration,
the feature tokens — and that is `MapChrome`: five values, resolved in one place, owned per call. It is
also the reason `MapDecl` is a copy of the frame's declaration rather than a borrow of the frame: the
window's `recenter` verb answers with the place the package named, and answering it while holding a
borrow of the package's frame is exactly the borrow the Rust compiler refused.

The evidence that nothing changed is comparative on purpose: the live run prints PATCH-094's numbers
verb for verb, and the 306 tests of the three groups the mission named are the same tests, unmoved.
Where behaviour *could* have changed silently — the two feature tokens, the refusal texts, the pinned
form, the gutter's clamp — this record names the thing that had to stay the same, so a later reader can
check the claim rather than trust it.
