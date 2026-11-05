---
patch_id: PATCH-094
title: The hammer in the map window — a brush the pointer sets, a mark, and an edit the survey reads back
status: applied
operation: feature + fix
target: voxel-client/map_view + voxel-client/shell/mount + voxel-work/stream + voxel-stream/world_source + voxel-shapers/earthworks + shploingus/live-instrument
base_ref: working tree at 2026-10-27, after PATCH-093
dependencies: [PATCH-090, PATCH-091, PATCH-092, PATCH-093]
source: user-directed-mission
created_at: 2026-10-27
approval: "user-directed 2026-10-27 — 'hammer editor controls in the map window, expressed through the shell's existing action vocabulary and the map's one camera/selection owner rather than a new parallel editor — a brush the pointer sets and sizes, a select/mark action, and the editing verbs (raise and lower at the brush, apply/clear) that visibly change the ground the survey draws and record the edit so it survives a resample, all drawn in the map's toolbar and gutter with the pinned window hiding them like the rest of the chrome. Prove it in the live game through the scripted pointer'"
live_mutation: false
---

# The map could be steered and could not be changed

PATCH-090 gave the window a camera, a selection, a cursor and a filter; PATCH-091/093 made its ground
the world's own voxels, residency and generator together. Nothing in the window could **change** any of
it. The world's record for changes made by hand already existed
([`earthworks`](crates/voxel-shapers/src/earthworks.rs), the F10 ruling's append-only chain,
`Streamer::edit` the one voxel door) — what did not exist was a map-side way to *aim* one, and the
temptation this patch exists to refuse is the obvious wrong shape: a second editor in the shell with a
state of its own, a vocabulary of its own and a picture of its own.

# The structure now

| module | owns | does not |
|---|---|---|
| [`voxel-client/map_view.rs`](crates/voxel-client/src/map_view.rs) | `MapBrush` — where the brush is, how wide, what it has marked, how far the next stroke lifts — `MapEdit` (the columns and the lift as the world can record them), `columns_within`, the ring's own drawing, `BRUSH_RADII_M` / `MAX_HAMMER_LIFT` | touch the world, own the camera, or know a streamer |
| [`voxel-client/shell/mount.rs`](crates/voxel-client/src/shell/mount.rs) | the hammer's **state and vocabulary**: `map_brush`, `map_edit`, the seven action arms in `map_action`, the brush's placement on a hover and a click, `map_facts().hammer`, the rings drawn in `map_elements`, `take_map_edit`, and the gutter's scroll (`map_controls_scroll`, its clamp, the wheel branch) | record a voxel, or hold a record |
| [`voxel-client/shell/webview.rs`](crates/voxel-client/src/shell/webview.rs) | the two documents, their layout, and **the column's own height** (`Rendered::content_dp`) against one scroll offset — the arithmetic in `child_box`, so the drawing and the measurement cannot drift | decide where a scroll is, or own a window |
| [`voxel-work/stream.rs`](crates/voxel-work/src/stream.rs) | `Streamer::shape_columns` — a stroke's own door: per column, read its own surface, copy the surface's material up or take it away, one `Modification` per voxel, bounded at `MAX_HAMMER_LIFT_M` — and `Shaped`, the counts a stroke reports | know a map, a brush or a ring |
| [`voxel-shapers/earthworks.rs`](crates/voxel-shapers/src/earthworks.rs) | `Earthworks::column_span` — how far up and down the record reaches in one column, read from the entries | keep a second index beside the entries |
| [`voxel-stream/world_source.rs`](crates/voxel-stream/src/world_source.rs) | `WorldSource::column_surface`'s walk **bounded by the record's own reach** where the record has touched the column | trust the law's ceiling for the world's own surface |
| [`shploingus/src/main.rs`](crates/shploingus/src/main.rs) | `apply_map_edit` — the app's whole part: drain the staged stroke and hand it to the streamer — and the scripted `hover` / `map <action>` verbs | derive a footprint, or decide a lift |

```
the pointer
   │  map_hover / map_release      (the sample's own column, not the declared cell's name)
   ▼
MapBrush  ──mark──▶  a frozen footprint ──raise/lower──▶  a staged lift
   │                                                          │
   └──────────── apply ───────────────────────────────────────┘
                              │  MapEdit { columns, lift }  (one take)
                              ▼
App::apply_map_edit ──▶ Streamer::shape_columns ──▶ one Modification per voxel
                              │                            (the player, fill/dig, the map's hammer)
                              ▼
the record moves ──▶ the chunk rebuilds ──▶ MapSurvey's next ground write ──▶ the picture moves
```

**One vocabulary, one owner.** A gutter button's `action` and the console's `map <action>` land on
`Mount::map_action` — the same call the camera's verbs take — so a press and a scripted verb cannot
mean two things, and there is no second place where "wider" or "raise" is defined. The brush is the
window's state beside its camera and its selection, which is why a pinned map wears none of it.

**A stroke is staged and then recorded, and it is recorded once.** `Mount` cannot touch the world (it
is handed a survey to read), so `apply` leaves the stroke in `map_edit`; `App::apply_map_edit` drains
it with `take_map_edit` and the map's next observation reads the changed ground back. A second `apply`
while one is waiting is refused by name rather than overwriting it — the failure a person would never
see and never believe.

# The defects the live run and the full sweep found

Six scripted runs of the live game preceded the one this patch records
([`logs/hammer-run.txt`](../logs/hammer-run.txt) … [`hammer-run6.txt`](../logs/hammer-run6.txt)),
and **not one of them showed the stroke in the map's own ground**. Each failure was a real defect
rather than a problem with the proof; the last two are recorded here, and the run this patch keeps is
[`logs/hammer-run7.txt`](../logs/hammer-run7.txt).

| the defect | how it showed | the fix |
|---|---|---|
| **the brush stood on the declared cell's name.** `map_hover` and `map_release` set the brush at `terrain.world_of(cell)` — the sample's position **snapped to the whole declared cell**, 32 m across and 22 m off the ground the pointer was over | the live run's stroke landed *beside* the ground the readout named: 812 columns raised, and the map's own ground at the struck column read **17.0 m before and 17.0 m after** (`logs/hammer-run5.txt`, `hammer-run6.txt`). The window said the hammer had struck and the picture at that column said nothing had happened — the worst kind of misleading feedback, because both readings came from the same map | the brush is placed on **`MapTerrain::sample_xz(cell)`**, the sample's own column, in both `map_hover` and `map_release` — the ground the pointer is over, which is also the column the readout names |
| **the far probe's walk was bounded by the law's ceiling.** `WorldSource::column_surface` started at the generator's own surface plus the tallest tree; a stroke that raised ground could stand **above** that ceiling and a dig could cut below its floor, so the probe answered with the world as it was before the edit | the survey's read-back test over the live run's own scene (`the_live_maps_own_stroke_reads_back_at_the_column_it_struck`) read the pre-stroke surface: the world held eight metres of new voxels and the column probe could not see one of them | `Earthworks::column_span` and `WorldSource::column_surface` walks from the **record's own reach** where the record has touched the column, because the law's ceiling is not the world's |
| **a stale assertion hid a charge leak.** `voxel-work`'s `a_changed_world_drops_every_mesh_because_the_generator_is_the_only_source` still pinned the pre-PATCH-092 contract — a world change **empties** the window and the bytes come back to zero — which is the behaviour the flash fix deliberately replaced | the whole-library sweep's first real run: `cargo test -p voxel-work --lib` failed `charged() == 0` with **1 701 456 B** still on the ledger (the drawn, demoted meshes). Reading the failure found the leak beside it: `note_world_changed` did `trees.clear()`, and every other place a tree is dropped (`edit`, `pump`) releases its bytes, so the octrees' charge stayed on the ledger for every world change — one window's worth per change, never returned | the demotion releases what it drops (the octrees) and keeps what it draws (the stale meshes); the test is renamed and re-pinned to the live contract — `a_changed_world_demotes_every_mesh_and_the_rebuild_is_the_same_world`: current meshes 0, stale meshes = the window, charge **less** than before, and the rebuild returns the same world, the same window and the same charge |
| **the hammer's own block pushed working controls out of existence.** `webview::stack` stops at the gutter's bottom edge, and the column this patch made ~130 dp longer than it was: at the map window's declared `480 × 440` the layout drew VIEW, ZOOM, HAMMER and one biome button — and **silently drew nothing** for Mountain, Forest, Grassland, Desert, Tundra, the whole TURN block and Recentre. `plan mountain` was *declared and not drawn* | found by asking the honest question of the shipped document at the shipped size — the assertion that every declared control is drawn, not only the seven the test knew by name. It failed with `plan mountain is declared and not drawn`, and the same cut is visible in the live capture ([`target/hammer7-f0045.png`](../target/hammer7-f0045.png)): the gutter ends after `Ocean` | **the gutter scrolls**: `webview::render` takes a scroll offset and reports the document's own height (`Rendered::content_dp`), the mount clamps it (`map_controls_scroll_max`), `Mount::map_wheel` answers by region — the camera over the ground, the column over the gutter — and the wheel moves in whole rows (`MAP_CONTROLS_ROW_DP = 26`) so nothing is ever drawn half over the toolbar. The app routes a wheel over either region to that one call. Every declared control is now drawn at some scroll, which is what the new assertion says |

The live run's *own* proof also had to be made honest twice: the first read-out reported the **cell's**
height (which moves with the ground, so it cannot be read twice) rather than the map's ground at the
**same column**; the scripted `hover` verb now reports `the hammer's ground at <column>`, which is the
same question asked of the same map twice (`shploingus/src/main.rs`).

# The witnesses

| witness | where | what it proves |
|---|---|---|
| the hammer's state and drawing | `map_view::tests::the_hammer_stages_a_disk_and_a_mark_freezes_where_it_lands` | the footprint is the world's lattice; the ladder's ends **stay put**; a resize after a mark does not move the target; `clear` forgets the mark and the lift and not the place; the readout is the toolbar's own words |
| the rings | `map_view::tests::the_hammer_draws_the_brush_and_the_mark_at_their_own_widths` | both rings where both are set, in their own inks, the wider covering more ground, each inside the body |
| the window's hammer end to end | `shell::mount::tests::the_map_hammer_stages_its_ring_and_a_pinned_map_draws_none_of_it` | every button drawn and inside the gutter; `raise` with no place refused by name; the brush on the **sample's own column**; the toolbar reading `4 m +3`; `apply` staging exactly `columns_within` and draining once; the pinned map wearing no ring — and the defect the stronger form of this test found: `plan mountain is declared and not drawn` |
| the stroke reads back | `map_survey::tests::a_hammer_stroke_reads_back_as_the_maps_own_ground` | a real streamer, one column raised three metres: after the chunk rebuilds, the survey's next ground write draws the surface the stroke left |
| the live scene reads back **and survives a resample** | `map_survey::tests::the_live_maps_own_stroke_reads_back_at_the_column_it_struck` | the game's own seed, window and brush (`x = 16, z = −48`, 16 m, +8 m): the sample nearest the strike moves by the lift, 0 refused — and the next climate bucket's resample of every law (`TICK_BUCKET`) draws the stroke's ground again rather than the elevation the generator would have put there |
| the world's own door | `voxel-work::stream::tests::a_hammer_stroke_raises_the_worlds_own_surface_and_records_it` | the surface moves by the lift read back through `column_surface`; a dig moves it down; one entry per voxel in the hammer's words; a stroke of no columns, of zero metres, or past the bound is refused by name |
| the demotion and the ledger | `voxel-work::stream::tests::a_changed_world_demotes_every_mesh_and_the_rebuild_is_the_same_world` | the world change keeps the drawn meshes and their charge, releases the octrees it drops, and the rebuild is the same world, the same window and the same charge |
| the whole column is reachable | `shell::webview::tests::a_taller_document_is_drawn_from_the_offset_and_reaches_its_own_end` | a rectangle shorter than the document: the end is drawn at the document's own height as the offset, no scrolled pass escapes the gutter, and every declared control is drawn at some offset |
| the link from the wheel to the column | `shell::mount::tests::the_map_hammer_stages_its_ring_and_a_pinned_map_draws_none_of_it` | every control the `MAP_CONTROLS` document declares is drawn at some scroll of the gutter, every scrolled pass stays inside the gutter, a wheel over the ground still zooms the camera, and a wheel back up returns the head of the column |

# The live proof

The running game, not the capture fixture: `SHPLOINGUS_SHELL` names frames and verbs, `hover` runs the
same three calls the window's `CursorMoved` makes, and `map <action>` is dispatched through
`Mount::map_action` — the call a gutter button's press lands on. The full record is
[`logs/hammer-run7.txt`](../logs/hammer-run7.txt); 26 verbs fired, exit 0, no panic.

| tick | the verb, in the run's words |
|---|---|
| 81032 | `hover 631,537 — the map's cell: Some((4, 13)) · the cursor says Some([16.0, 17.0, -48.0]) · the hammer's ground at Some([4.0, -60.0]) is Some(17.0)` |
| 81036 / 81038 | `brush 8 m`, `brush 16 m` — the ladder, answering the width |
| 81040 | `marked 16 m at 4,-60` |
| 81042 … 81056 | `lift +1 m` … `lift +8 m` — eight presses to the bound, each answering the staged total |
| 81062 | `apply 812 columns +8 m`, then the app's own line: `hammer: 812 columns raised 8 m — 6496 raised, 0 lowered, 0 refused, 0 with no surface` |
| 81078 | `hover 631,537 — the map's cell: Some((4, 15)) · the cursor says Some([16.0, 17.0, -48.0]) · the hammer's ground at Some([4.0, -60.0]) is Some(25.0)` |
| 81102 | the same read again after two more seconds of frames: `the hammer's ground at Some([4.0, -60.0]) is Some(25.0)` |
| 81078 on | `map-plan pinned` — the minimap form, the run ended clean |

**The picture's own ground moved by the stroke**: the map's ground at the brush's column reads
**17.0 → 25.0 m** for eight metres of lift, and the cell picked by the same pointer moved from
`(4, 13)` to `(4, 15)` because the ground under it rose. `6 496` voxels were raised, `0` refused and
`0` skipped — the whole footprint took, and the record's own counts say so. The climate resampled
later in the same run (`t81126 (map) 209 of 209 samples from the world's own ground — 209 resident, 0
asked of the generator (0 still the law's) · laws resampled`) and the ground stayed the stroke's; the
read-back across a resample is pinned by the survey witness above rather than by a third hover.

Captures at the swapchain's pixels (`target/hammer7-f0024.png` … `f0070.png`): the map window with the
gutter's HAMMER block (`Brush −`, `Brush +`, `Raise`, `Lower`, `Mark`, `Clear`, `Apply`) between the ZOOM
row and the biome list, the toolbar's `HAMMER` fact, the ring on the struck ground before the stroke,
the raised ground after it, and the pinned form with the chrome gone.

Also live in the same run: the survey's own counts, which is the previous pass's claim still holding —
`14400 of 14400 samples from the world's own ground — 16 resident, 14384 asked of the generator (0 still
the law's)` when the map is at the far scale, and `144 of 144` with the residency alone at the near one.

**What the live run does not cover**: the gutter's scroll. It was added after this run, from a
defect the test found rather than the game (the capture below shows the cut it repairs — the gutter
ending after `Ocean`), and a wheel is not in the scripted verb vocabulary. Its coverage is the two
tests named in the witnesses, not a frame of the game.

# Checks run

| check | result |
|---|---|
| `cargo test -p voxel-client --lib` (the whole library, **run to completion in the background**) | **651 passed, 0 failed, 4 ignored** in 263 s ([`logs/lib-suite-094.txt`](../logs/lib-suite-094.txt)) — the first complete run this mission has had, after PATCH-093 had to report it as not re-run. It was taken before the two later edits below (a repair to `voxel-work` and one assertion added to a survey test) |
| the same library on the **final tree**, group by group | `game::` 4 passed (130.9 s), `world::` 12 passed (3.0 s), `skylight::` 39 passed 1 ignored (169.2 s), `renderer::` **22 passed** (60.3 s), `map_survey` 12 passed 1 ignored (17.2 s), `map_view` 11 passed, `shell::` **283 passed 1 ignored** (15.9 s) — every group green. The first four groups were taken before the gutter's scroll landed; the three that render the shell — `shell::`, `map_view`, `map_survey` — were re-run after it, and all green |
| `cargo check -p shploingus` | the app compiles with the wheel routed to the map's own entry point for both of its regions (4 m 51 s; one pre-existing dead-code warning) |
| `cargo test -p voxel-client --lib` again, after the last edits | **not finished**: two background attempts reached 647 and 645 of the suite's ~651 lines and then spent 20+ minutes inside the same slow group (`game::` + `skylight::` + `renderer::` + the five `shell::mount` tests that each take over a minute). The machine's volumes are at 99 % and 100 % full and the run competes with itself, which is where the time goes. The group-by-group row above is the final tree's coverage; this row is the state of the full re-run, not a pass |
| `cargo test -p voxel-work --lib` | **70 passed, 0 failed, 2 ignored** after the repair; the sweep's first run of it was `69 passed, 1 failed` (the stale assertion and the leak above) |
| `renderer::` under load, once | **20 passed, 2 failed** in a run taken while the machine was loaded — the failure lines were lost to a `tail` pipe and the group passed 22 of 22 on two clean re-runs, so this is recorded as **unreproduced and un-named**, not as a pass and not as a defect |
| `voxel-stream`, `voxel-shapers`, `voxel-mesh`, `voxel-model`, `voxel-ui`, `voxel-package` libraries | **not run this pass**: the sweep never got past its first crate. The crates this patch edits are `voxel-client` (above), `voxel-work` (above), `voxel-stream` and `voxel-shapers`; the last two are exercised through `voxel-client`'s own tests (`map_survey`, `world`, `skylight`, `game`) but their own libraries were not re-run. Stated as a gap rather than implied by the greens above |
| the live game, 26 scripted verbs | every verb answered, the stroke recorded `6496 raised, 0 refused`, the map's ground moved 17.0 → 25.0 m and stayed there through the next frames, `map-plan` pinned clean, exit 0 |

The stale assertion is worth stating plainly: **the sweep was not green on the first complete run.** It
was one failure in `voxel-work`, and the failure was the tree's own record of a change it had made
(PATCH-092's demotion) — repaired here, with the leak it was hiding, and the package re-run green.

# What this does not do

* **The minimap's own size and interpolation are untouched**: a pinned map draws the same survey at the
  same camera, coalesced into the HUD's small body.
* **Cuboid fog and the FPS/needs toasts are untouched** — later passes' questions.
* **The camera, the marks, the picking, the filter and the cursor readout are PATCH-090's**, unchanged:
  the hammer is another thing the window points at on the same map rather than a second editor.
* **A stroke is bounded at ±8 m and is per voxel.** The record is per voxel, so a stroke is many
  entries; the bound is what keeps one press a stroke rather than a stall (~6 000 voxels at the widest
  brush).
