---
patch_id: PATCH-093
title: The map's whole window is the world's own surface — the far ground read at the survey's own LOD
status: applied
operation: feature + fix
target: voxel-client/map_survey + voxel-work/column probe + shploingus/live-instrument
base_ref: working tree at 2026-10-27, after PATCH-092
dependencies: [PATCH-091, PATCH-092]
source: user-directed-mission
created_at: 2026-10-27
approval: "user-directed 2026-10-27 — 'the map's whole visible ground must be the streamer's real chunk surface, not just the 160 x 160 m of residency: the survey should ask the streamer for the columns it needs at its own sample LOD, so zooming out and looking at distance far from the player shows the world's own surface rather than falling back to the elevation law, and prove it by driving the live game, not only the capture fixture'"
live_mutation: false
---

# The residency was the map's whole world

PATCH-091 made the map's heights the streamer's own voxels — **where the streamer held a chunk**. A
resident window is a few hundred metres; a map of a valley is kilometres, and the survey covers the
body at whatever the camera asks for. So the picture was two surfaces stitched at an invisible seam:
the world's own ground in the middle, and `terrain::elevation_m` — the smooth field the voxels were
*placed from*, not the voxels — everywhere else. Zooming out made the seam's territory the picture.

| fault | what it cost |
|---|---|
| the survey had no way to ask about a column nobody had built | every sample outside the residency was the law's field: a smooth re-derivation, disagreeing with the ground by the voxel's own quantisation and by everything that happened to a chunk since it was generated |
| the far question was unanswerable at build cost | building a chunk to read one column costs 136 ms (`build_chunk_from`), and a wide survey holds tens of thousands of columns |
| `Observation` counted one source | a caller could not tell a survey that was the world's own from one that was the law's, so nothing could report the climb or assert it reached whole |

# The structure now

| module | owns | does not |
|---|---|---|
| [`voxel-work/stream.rs`](crates/voxel-work/src/stream.rs) | `Streamer::column_surface` — **the world's own surface at one column, whether or not a chunk is resident**: the pool's generator with the earthworks record composed over it, canopy included, fluids skipped | know a map, a camera or a sample grid |
| [`voxel-client/map_survey.rs`](crates/voxel-client/src/map_survey.rs) | `MapGround` (two questions: `chunk_surface` for a resident column, `column_surface` for one nobody built), `Far` — **the generator's answers and the record generation they were given at** — the three passes of one ground write, and the counts (`from_chunks`, `from_world`) | own a camera, draw, or know a window |
| [`voxel-client/shell/mount.rs`](crates/voxel-client/src/shell/mount.rs) | the map's camera, selection, cursor, filter, and **one** call into the survey per frame (`map_observe`) | hold a terrain, a key, or a raster |
| [`shploingus/src/main.rs`](crates/shploingus/src/main.rs) | `observe_map` — the world as the map may read it — and the line that reports the three counts when they move; the scripted pointer's `map <action>` verb | derive a survey key, or sample a column itself |

```
one ground write (MapSurvey::observe, gated to 4 a second)
  law grid for the plan ──> MapTerrain (the base, and the honest fallback)
        │
        ├─ write_resident   → every resident chunk's own 32 x 32 columns   → from_chunks
        └─ Far::write       → 1 024 new generator columns a write, ring    → from_world
                              order from the middle out; every column ever
                              asked is cached until the *record* moves
```

**The band is what a frame can pay for.** A column probe measured **22 µs**
(`measure_the_far_column_probe`), so the widest window's 36 864 samples are 0.9 s of work in one go.
`FAR_BAND_PROBES` is **1 024** — 22 ms a write — and `Far`'s walk is the whole survey in ring order
every time: a composed terrain starts from the law each write, so the walk **re-establishes** every
column already asked (a lookup, no probe) and spends its band on the columns that are new to it. The
picture therefore sharpens outward from the middle of the view, and the count climbs until
`world_samples() == samples`.

**The cache is keyed on the record, not on residency.** The generator is a pure function of the spec
and the column; the one thing that can change an answer is an earthworks edit. So `Far` empties when
`MapGround::world_generation` moves and never when a chunk merely streams in — otherwise a pan would
throw away every answer it had, at the rate the world streams.

**The law is the fallback, and only that.** A column whose only matter is water, or one the generator
refuses, still reads the law's elevation — the ink below sea level is what draws a sea. What changed is
that this is now the exception the counts can show, not the state of everything beyond 160 m.

# The defects the live run found

The first pass of this work was proved in the capture fixture, which fills the survey in a loop before
it composes. The live frame loop is a different instrument, and it found three things the fixture could
not:

| the defect | how it showed | the fix |
|---|---|---|
| **a chunk answered past its own grid** — `ChunkSurface::top` refused a negative local column but not one ≥ 32, so `[32, z]` read the *wrapped* entry of the row below | the survey's own tally came out **larger than the survey**: 76 of 72 samples on a plan 9 wide whose column 8 is centred exactly on a chunk edge. In the game, that was `attempt to subtract with overflow` in the report line — **the process died at the exact frame the fill first reached whole**, after 7 clean passes of the mission never exercising the frame loop | `top` answers nothing outside its own `32 x 32` columns (the neighbour owns that metre), and a sample reached by two chunks is counted **once** — the resident mask's own tally rather than a tally of writes |
| **a report line that could take the frame loop down** | the same subtraction: `observation.samples - observation.world_samples()` | `saturating_sub`, in both the game and the capture example: a reading must not panic the run |
| **a typo in a verb answered as a wrong view** | `map zoom wheee` refused *"is not a world feature this package draws; the views are all, ocean, …"* — a refusal to the wrong question, printed by the live run | `Mount::map_action`: a one-word action is a view (the console's `plan` token); a phrase that is not one of the verbs is refused as **"not a map action"** |

And one stale test, found by running the whole library: `scene::tests::the_cached_scene_is_the_resident_world_on_every_frame`
still asserted that a world change **empties** the drawn world, which is the behaviour the world-change
demotion deliberately replaced (PATCH-092's client witness: a settled window of 92 chunks went
`10 024 → 0` quads and stayed 0). The assertion now pins the live contract instead — the change keeps
the picture and, because the bytes did not change, the key — without weakening the per-frame claim that
the scene is the resident world.

# The witnesses

| witness | where | what it proves |
|---|---|---|
| the far ground is the generator's | `map_survey::tests::the_far_ground_is_the_generators_own_and_the_residency_wins_where_it_holds_one` | a survey far wider than the one held chunk: the chunk's samples are its sheet, every other sample is the generator's, and the law is nowhere in the picture |
| the fill climbs and reaches whole | `map_survey::tests::the_far_fill_climbs_a_band_at_a_time_to_whole` | one band a write, the count climbing, and `world_samples() == samples` at the end without the plan moving |
| the probe agrees with the chunks it overlaps | `map_survey::tests::the_probe_agrees_with_the_resident_chunks_it_overlaps` | a real streamer, settled: the generator's column answer and the resident chunk's own column agree where both exist — the seam is an agreement, not a boundary between two worlds |
| a boundary sample is one chunk's | `map_survey::tests::a_sample_on_a_chunk_boundary_is_one_chunks_own_and_is_counted_once` | the plan that killed the game: 9 samples of 8 m centred on the origin, column 8 on the chunk edge; the world cannot answer more samples than the survey holds, the fill reaches whole, and the height is the chunk that owns the column's |
| a chunk answers only its own columns | `map_survey::tests::a_chunks_surface_refuses_a_column_outside_its_own_grid` | `[32, 0]`, `[0, 32]` and `[-1, 0]` are nothing, while `[31, 31]` answers |
| an unknown action is refused as one | `shell::mount::tests::an_unknown_map_action_is_refused_as_an_action_and_an_unknown_view_as_a_view` | `zoom wheee` is "not a map action", `plan nowhere` and `nowhere` are refused with the views the package draws, and the real verbs still answer |
| the map in the shell's own layer | `shell::mount::tests::the_map_composes_in_the_shells_layer_from_the_worlds_own_voxels` | one observation places the camera, the survey's heights are the ground's inside the residency and the world's beyond it, the picture fills the body and stays inside the window at 1280 x 720 and 1920 x 1080, and the map's declared frame is withdrawn |
| the scene after a world change | `scene::tests::the_cached_scene_is_the_resident_world_on_every_frame` | the demotion keeps the drawn world, and every frame's scene is the streamer's own meshes |

# The live proof

The running game, not the fixture: `SHPLOINGUS_SHELL` names frames and verbs, and `map <action>` is
dispatched through `Mount::map_action` — the call a gutter button's press lands on — so the controls
that ran are the window's own vocabulary.

```
SHPLOINGUS_SHELL="6:open map-plan,8:map zoom out,10:map zoom out,12:map zoom out,14:map zoom out,520:quit"
```

`game_seed` 9418646105956113, the demo's own world, the map window opened and zoomed to the scale floor
(`-3`; the camera clamps at 0.05 px/m). The map's own line, in the run's words:

| tick | the line |
|---|---|
| 81013 | `1126 of 14400 samples from the world's own ground — 42 resident, 1084 asked of the generator (13274 still the law's) · laws resampled · ground rewritten` |
| 81049 | `5138 of 14400 — 9 resident, 5129 asked of the generator (9262 still the law's)` |
| 81101 | `14354 of 14400 — 16 resident, 14338 asked of the generator (46 still the law's)` |
| **81106** | **`14400 of 14400 — 16 resident, 14384 asked of the generator (0 still the law's)`** |
| 81152 | `14400 of 14400 — 16 resident, 14384 asked of the generator (0 still the law's) · laws resampled · ground rewritten` |

The residency is **16 samples of 14 400** at that scale, and the other 14 384 are the world's own
columns asked of the generator through the streamer — **not one sample is the elevation law's**. The
climate resample at 81152 re-reads the whole survey from the cached columns and loses nothing.

The capture, at the swapchain's own pixels (1280 x 720, `SHPLOINGUS_CAPTURE_DIR`, frame 139, the
frame the fill completed on): [`live-map-f-0139.png`](../target/live-map-f-0139.png) — the map window
with its toolbar (`GRID 28x32`, `ZOOM -3`, `CURSOR X/Z/Y`) over kilometres of the world's own coast and
land, and the ground reaching all four edges of the body.

A second scripted run drove the controls a careless person would: zoom out three times, `plan ocean`,
`zoom reset`, `zoom in`, **pin the map (the minimap form)**, unpin, then two bad inputs.

| the verb | the run said |
|---|---|
| `map plan ocean` | `ocean` — the view's frame, and the survey resampled under it (9 522 samples pinned, 6 125 unpinned) |
| `pin map-plan` / `unpin map-plan` | pinned, the survey became the *content container's* plan and filled while pinned; unpinned, the chrome's plan came back |
| `map zoom wheee` | `"zoom wheee" is not a map action` |
| `map plan bad-token` | `"bad-token" is not a world feature this package draws; the views are all, ocean, mountain, forest, grassland, desert, tundra` |
| `map` (no argument) | `plan needs a feature; this package declares all, ocean, mountain, forest, grassland, desert, tundra` |
| — | the map's counts climbed to `6125 of 6125 … (0 still the law's)` and the run ended clean |

# The readings

[`docs/baselines/map-survey.json`](baselines/map-survey.json), re-measured on this build, has the
fixture at both sizes and the live run beside it:

| plan | samples from the world's own | resident | generator | the law's |
|---|---|---|---|---|
| 1280 x 720, 2 px/m (NEAR) | 6 696 of 6 696 | 400 | 6 296 | 0 |
| 1920 x 1080, 2 px/m (NEAR) | 16 576 of 16 576 | 400 | 16 176 | 0 |
| 1920 x 1080, 0.25 px/m (FAR) | 28 032 of 28 032 | 9 | 28 023 | 0 |
| the game, zoomed to the floor | 14 400 of 14 400 | 16 | 14 384 | 0 |

The wide fixture's survey is 7.1 x 9.3 km at 48.7 m a sample and takes 28 bands (2 018 ms) to fill
once; a later pan over ground it has read pays lookups. The first observation — one band included — is
39.5 ms at 1280 x 720 and 50.3 ms at 1920 x 1080, against the 14.5 / 24.1 ms PATCH-091 recorded for
the residency alone: the band is the price of the rest of the window, and it is paid a quarter of a
second at a time.

# Checks run

| check | result |
|---|---|
| `cargo test -p voxel-client --lib map_survey` | 10 passed, 1 ignored (the probe measurement) |
| `cargo test -p voxel-client --lib map_view` | 9 passed |
| `cargo test -p voxel-client --lib "shell::"` | 280 passed, 1 ignored |
| `cargo test -p voxel-client --lib scene::` | 32 passed |
| `cargo test -p voxel-client --lib skylight::` | 39 passed, 1 ignored |
| the whole library, `cargo test -p voxel-client --lib` | **644 passed, 1 failed** at 11:15 — the stale scene assertion this patch repairs; that test passes on its own after the repair. A later full run did not finish inside ten minutes (`skylight` alone measures 170 s on this machine, and the volumes are at 99 %), so the whole-library suite is **reported as not re-run to completion**, not as green |
| the live game, zoomed to the scale floor | the fill reached `14400 of 14400 … (0 still the law's)` at tick 81106, survived a climate resample at 81152, no panic, exit 0 |
| the live game, the careless run | every verb answered and the refusals named the right question; the fill reached `6125 of 6125 … (0 still the law's)`; exit 0 |
| the capture fixture, `NEAR` and `FAR` at both sizes | 6 696 / 16 576 / 28 032 samples, every one the world's own, 0 the law's |

# What this does not do

* **The minimap's own size and interpolation are untouched.** A pinned map draws the same survey at
  the same camera, coalesced into the HUD's small body; choosing a coarser sample for the small form is
  the next pass's question.
* **The far field keeps streaming** (`far_terrain`, the surface ladder) — this pass reads the world for
  the *map*, and harness hammer tools are untouched.
* **The camera, the marks, the picking, the filter and the cursor readout are PATCH-090's**, unchanged:
  the survey answers a different ground and every one of them reads it through the same projection.
* **The cache is per mount.** A closed and reopened map pays its bands again; nothing is written to disk.
