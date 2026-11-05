---
patch_id: PATCH-091
title: The map's ground is the world's own — the streamer's voxels, one owner, and the window's own layer
status: applied
operation: refactor + fix
target: voxel-client/map_survey + voxel-client/shell/map-plan + shploingus/map
base_ref: working tree at 2026-10-27, after PATCH-090
dependencies: [PATCH-090]
source: user-directed-mission
created_at: 2026-10-27
approval: "user-directed 2026-10-27 — 'the map is column but the game is the sobvoxels and octree cells with subvoxels at the 32^3 space in profile shapers, terrain has multiple pdf documents that are supposed to be real world and resized down the the 1/8th world scale, all of it'"
live_mutation: false
---

# The map was drawing the laws, not the world

Every map frame since PATCH-088 was a field of `terrain::elevation_m` samples. The world the player
walks is chunks of octree cells with `profile32` subvoxel shapes, earthworks composed into them, and
canopies a chunk above the ground — none of which the map could see. The user's report names the
consequence exactly: *"the first mountain I went to was pretty shit"*.

Three structural faults kept it that way:

| fault | what it cost |
|---|---|
| the survey was a **re-derivation of a law**, at one sample per declared cell subdivided, with no path from the streamer to it | the map disagreed with the world, and no amount of zooming made it agree |
| **three owners**: `main.rs` held the terrain and re-derived the survey key every frame, `Mount` held the camera, `map_view.rs` held the camera, the filter, the raster and the marks | a rebuild of the survey was one caller's job and the picture another's; `refused` frames of staleness had no name |
| the map was **stitched into the frame after the cell raster** by `App::redraw` and by the capture example, from two call sites | another window's text read through the map, and the static declared cells and the live ground could both be drawn in the same rect |

# The structure now

| module | owns | does not |
|---|---|---|
| [`map_survey.rs`](crates/voxel-client/src/map_survey.rs) | `MapGround`/`ChunkSurface` (the world as the map may read it), `MapSurvey` (the law field, the composed ground, the keys, the cached raster), `MapWorld`, `Observation` | know a camera, a window or a selection |
| [`map_view.rs`](crates/voxel-client/src/map_view.rs) | `MapCamera`, `SurveyPlan`, `MapTerrain` (a grid and its inks), `ViewFilter`, the marks | read the world, or hold anything between frames |
| [`shell/mount.rs`](crates/voxel-client/src/shell/mount.rs) | the camera, the selection, the cursor, `map_observe`, and `map_elements` | sample the world, or let a caller draw the map |
| [`main.rs`](crates/shploingus/src/main.rs) | the world it stands in: one `observe_map` call per frame | a terrain, a key, or a terrain argument at any call site |

```
streamer ─┐
laws ─────┴─> MapWorld { ground, spec, tick } ─> MapSurvey::observe(plan) ─> MapTerrain
                                                                                │
   camera + rect + filter + selection + cursor ────────────> MapSurvey::body_elements
                                                                                │
                                          window_elements ─> map_elements ─> layer ─> cells raster
```

**The streamer is the ground.** `impl MapGround for Streamer` asks one question per chunk — its
`32 × 32` column surfaces — walking every layer the window holds over that column from the top down, so
a canopy is the surface where there is one. `absorb_surface` reads a uniform leaf at its top face and a
mixed voxel at its own shape's topmost subvoxel (1/4, 1/2 or 3/4 of a metre), and **skips fluids**:
water is a voxel material here, so its skin would otherwise paint every resident ocean chunk as an
island — which the `PLAN=ocean` capture showed as a green patch in the middle of the sea before the
rule was named.

**The map composes where the window is.** `map_elements` is called from `window_elements`, inside the
layer's own window order: a window drawn after the map covers it, a window drawn before it is covered
by it, and nothing the map emits leaves the content container its package declared. `frame_space`
withdraws the map's declared frame once the survey is live, so the fallback picture cannot read
through the live ground.

**The camera is placed by the declaration and then owned by the mount.** `map_observe` puts it on the
frame's declared column and at the frame's declared tilt on the first observation (and after
`recentre`), then never touches it: pan, wheel, rotate, pitch, `zoom reset` and `recentre` are the only
writers.

# The defects the proof found

| the defect | how it showed | the fix |
|---|---|---|
| `depth_of` read a body pixel back as `sin(pitch)/px/m` metres where `project` writes `px/m·sin(pitch)` pixels per metre — the inverse was not the inverse | a pointer on the column at 20 m forward named the column at 3 m: **17 samples out** at 8 px/m, so a click selected and a readout named a place the viewer was not pointing at | `depth_of` reads the pixel back by the projection's own ratio, and `ground_point` takes the forward distance it returns. `the_pointer_picks_the_world_column_a_tilted_camera_is_pointing_at` fails without it |
| the ground raster spilled past its body: a run's padding reached 2.25 dp below the bottom edge and the last sky band 1 dp past it | asserted as containment, failed at 728 dp against a 727 dp container | both are clipped to the body they are drawn in |
| the capture example drew every rect through `cells::retinted`, which snaps each colour to the monochrome palette | the map came out **grey** — #333 and #808080 — while the survey's own colours were grass (138, 153, 114) and water | a map capture uses the game's own path (`cells::frames_in`); the app never retints a map |

# The witnesses

| witness | where | what it proves |
|---|---|---|
| a live streamer walked column by column | `map_survey::tests::a_live_streamers_own_voxels_are_what_the_map_draws` | the map's height for a sample is the world's own topmost solid subvoxel, by an independent read of the same rule; and some samples disagree with the law |
| the two sources, one survey | `map_survey::tests::the_survey_reads_the_worlds_own_voxels_and_the_law_is_only_the_fallback` | inside the residency the height is the ground's; beyond it the law's; a dig reaches the map when the gate opens |
| one observation, one owner | `shell::mount::tests::the_map_composes_in_the_shells_layer_from_the_worlds_own_voxels` | the camera is placed on the declared column and pitch; the body is ≥ 98 % painted at 1280 × 720 and 1920 × 1080 at one world scale; every element is inside the content container; the declared frame is withdrawn |
| the pointer's inverse | `map_view::tests::the_pointer_picks_the_world_column_a_tilted_camera_is_pointing_at` | a projection through a 24° tilt picks back the same column at 8 px/m |
| the selection's own ground | `map_view::tests::a_view_selection_lights_its_own_ground_and_shadows_the_rest` | a selection changes ink and not coverage (counted by covered row steps, so run merging cannot fake it) |

Captures through the real path —
`MAP=1 NEAR=1 MAX=1 SIZE=720|1080 cargo run -p voxel-client --example shell-shot` — build a real
`Streamer` (seed `GAME_SEED`, radius 2 chunks), settle it over the frame's declared column, and compose
the shipped package through the shell's own layer:

| | 1280 × 720 maximised | 1920 × 1080 maximised |
|---|---|---|
| body | 1 109 × 590 dp | 1 749 × 950 dp |
| survey | 72 × 93 samples of 8 m over 576 × 744 m | 112 × 148 samples of 8 m over 896 × 1 184 m |
| from the world's voxels | 400 of 6 696 | 400 of 16 576 |
| observation | 14.5 ms | 24.1 ms |
| composed layer | 31.5 ms · 13 564 elements | 90.2 ms · 41 199 elements |

`PLAN=ocean` at 720 keeps the water lit and shades the land, with the VIEW fact naming `ocean`: the
biome selection works through the same projection the picture uses.

The measured rows are in [`docs/baselines/map-survey.json`](../docs/baselines/map-survey.json); the
controls are documented in [`docs/MAP-CONTROLS.md`](../docs/MAP-CONTROLS.md).

# What this does not do

* **The survey is streamed from whatever the streamer already holds**, not from a survey ring of its
  own. At 2 px/m a maximised 1080p window shows 896 × 1 184 m, of which 160 × 160 m is residency; the
  rest is the laws. Streaming the map's own survey from the streamer is the next pass's job, and the
  seam it leaves is named in the baseline's `unstable` rows.
* **At a wide view the sample is 8 m** (a declared cell ÷ `SUB`), so a 32 m voxel's own shape is finer
  than the picture. The sample coarsens to keep the edges; it does not refine where the residency is.
* Hammer tools, the pin glyph button, the XML minimal titlebar and fog are untouched, as the mission
  ruled. The ASCII column path stays the 2D cell renderer's.
* `cargo test -p voxel-package` fails one pre-existing census test
  (`census::folders::tests::the_real_working_directory_has_no_unclassified_folder`: the top-level
  `tools/` folder is unclassified). It is not this pass's and it is not repaired here.
