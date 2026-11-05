---
patch_id: PATCH-090
title: The map window you can steer — one camera, a survey that reaches the edge, and runs that leave no seam
status: applied
operation: fix
target: voxel-client/map_view + voxel-client/shell/map-plan
base_ref: working tree at 2026-10-27, after PATCH-089
dependencies: [PATCH-089]
source: user-directed-mission
created_at: 2026-10-27
approval: "user-directed 2026-10-27 — 'again it doesnt stream to the edges of the window', 'the biome selection needs to work', 'dragging and panning should literally change where the window's center streamed chunk is', 'we use the axis record format for the shell cell frames'"
live_mutation: false
---

# Three complaints, three defects, and one of them was two pixels wide

The map window drew a survey sampled from the world's own elevation and climate laws. The user
reported three things about it, and each turned out to be a defect with a nameable cause.

| the report | the defect | the fix |
|---|---|---|
| *"it doesnt stream to the edges of the window"* | `SurveyPlan.columns` was read as a count of **samples** by `MapTerrain::sample` and written as a count of **declared cells** by `MapCamera::survey`, so the survey covered a quarter of the ground the body could see in each direction — the rest of the body was sky | one vocabulary: the plan carries `sample_m` (the survey's own grid) and `declared_m` (what a selection is measured in), and `survey` sizes the sample so the plan covers `width ÷ px/m` across and `height ÷ (px/m · sin pitch)` down, coarsening the sample — never truncating the ground — when the window shows more than `192 × 192` samples of it |
| *"the biome selection needs to work"* | `ground_elements` drew every sample with its own colour and never read the frame's `feature`; `plan ocean` swapped the frame and changed the caption only | `ViewFilter` (from `selected_feature`) keeps a view's own ground lit and shades the rest; the water token keeps the water. The raster's cache key carries the filter, so the picture cannot lag the selection |
| *"there are visible holes between the columns"* | the ground runs **abutted**: a run's rect ended exactly where the next began, and a renderer that rounds two edges to the same pixel leaves one pixel of sky between them. Measured: 8 169 sampled pale pixels in a 1920 × 1080 capture | every run overlaps its neighbour by half a row step, and its column by half a column. Re-measured: **104** — a 98.7 % fall — and the survivor is a faint tint at the water's edge rather than a hole |

# The camera is one value, and the survey is its question

`MapCamera { centre_m, pixels_per_m, bearing_deg, pitch_deg }` is the whole picture: one scale in
both axes, a projection that is its own inverse, and `survey` the only thing that turns it into a
question about the world. `Mount` owns it and every control writes to it — drag, wheel, `zoom in`,
`zoom out`, `zoom reset`, `rotate left`, `rotate right`, `pitch up`, `pitch down`, `recentre` — and
`Mount::map_camera_to` sets it outright for a capture or a restored view.

The declared cell stays the map's own unit: `MapTerrain::world_of` snaps a sample to the declared
cell it falls in, so the toolbar's `CURSOR X/Z/Y`, the `◇` mark and the route all name whole map
cells rather than screen pixels.

# The witnesses

| witness | where | what it proves |
|---|---|---|
| coverage at 3 window sizes × 4 scales × 3 pitches | `map_view::tests::the_survey_reaches_the_body_at_every_size_and_scale` | the survey covers what the body can see, and no sample is finer than a declared cell ÷ `SUB` |
| no sky below the ground, per column, at 2 sizes × 3 pitches | `map_view::tests::the_raster_covers_every_pixel_of_the_body` | the raster leaves no pixel of the body unpainted — the seam law, asserted as a law rather than seen in a capture |
| the same world scale in two bodies | `map_view::tests::the_scale_is_one_number_and_a_resize_does_not_change_it` | a resize reveals more world instead of stretching it |
| pan, zoom, rotate, pitch | `map_view::tests::pan_zoom_rotate_and_pitch_move_the_camera_and_nothing_else` | a pan moves the centre and not the scale; a zoom about an anchor keeps the ground under it; tilt and scale clamp |
| a selection lights only its own ground | `map_view::tests::a_view_selection_lights_its_own_ground_and_shadows_the_rest` | the vocabulary per token, that a selection changes the raster, that it never draws brighter than the survey already was, and that the mean dims |
| the same picture at four sizes and a zoomed-out scale | `shell::mount::tests::the_map_body_draws_the_player_and_a_click_selects_the_cell_under_it` | the window's own raster paints ≥ 98 % of its body at 1280 × 720, 480 × 440, 1920 × 1080 and at ⅛ the default scale |

Captures through the real path (`MAP=1 SIZE=1080 MAX=1 cargo run -p voxel-client --example shell-shot`,
which composes the shipped package and the map body through the shell's own renderer): the 1080p
maximised window fills to all four edges at the same 0.45 px/m as the 480 dp window, and
`PLAN=ocean` / `PLAN=desert` show the selection's own ground lit with the rest in shadow.

# What this does not do

* The survey is sampled from the **world's laws**, not from the streamer's resident chunks, so
  `profile32` displacement and earthworks do not appear in the map. The user's own ruling was that
  this pass leaves streaming out: *"we have chunk streaming already — you'd just be continuing
  earlier work by bringing the chunk streaming into the baselines recipes format"*. The baseline
  keeps this as its `unstable` note.
* **The sample coarsens when the window shows more ground than 192 × 192 samples of it describe.**
  At 1920 × 1080 maximised that is 27 m per sample: one sample per twelve pixels. It costs 46–51 ms
  per distinct view, which is why the number is not raised further here; the next pass's job is to
  answer the survey from streamed chunks, at which point the sample size stops being the map's own
  resolution and starts being the world's.
* Hammer tools, pinned/minimal-titlebar chrome and the ASCII column path's own scope are untouched:
  the column stays the 2D cell renderer's, as the user ruled in PATCH-089.
