# The map window's controls

The map window's chrome is not built in Rust. Two XML documents are parsed by
`voxel_client::shell::webview` and laid out in the window's content container:

| document | root | where it draws |
|---|---|---|
| `MAP_TOOLBAR` | `<map-toolbar>` | a 40 dp strip across the window's full content width, at the top |
| `MAP_CONTROLS` | `<map-controls>` | the right-hand gutter, below the toolbar |

The terrain draws in the rectangle left of the gutter and below the toolbar. The three rectangles
come from one arithmetic in `Mount` (`map_toolbar_rect`, `map_view_rect`, `map_controls_rect`), so a
button's drawn rectangle and the rectangle the pointer resolves against cannot disagree.

**A pinned map is a minimap, and the chrome goes.** `Mount::map_chrome_shown()` is
`!is_pinned("map-plan")` and all three rectangles above read it: pinned, the toolbar and the gutter
have no rect at all, the controls' document is not laid out (so no lane can answer a click the frame
does not draw), and the ground's rect **is** the window's content container — x, y, right and bottom
all equal to it. Pinning is the map's own pattern: the window is HUD chrome the player keeps in
view, so it spends its whole body on the ground. The pin lane on the title bar toggles it and the
chrome returns. See PATCH-092.

**The terrain is shell elements, not a GPU surface.** `MapTerrain::body_elements` draws it — a sky
band, the ground, then the route, the selected cell, the player's marker and any off-screen edge
indicator — through the same `voxel_ui::Element` path every other window uses. There is no second
renderer. What the ground *is* comes from the streamer's resident chunks (`map_survey::MapGround`),
with the laws as the fallback beyond the residency; see PATCH-091.

**The view is pitched, not a plan.** The camera looks down `PITCH_DEG` (24°): each row is compressed
into the lower share of the body, the ground band is backed so the relief does not show slits of sky,
and a cell's own height lifts it above the row it stands on, so the near ground hides the far ground
and a ridge stands over the skyline. That is what makes a distance readable off the picture; a flat
plan loses it. Picking runs through the same projection: `MapTerrain::cell_at_pointer` searches the
cells actually drawn and falls back to the row of the pointer's position when it lands in a relief
sliver, so the toolbar's readout and the cursor outline always name ground.

## `<map-toolbar>`

```xml
<map-toolbar>
  <title text="WORLD MAP"/>
  <fact label="VIEW" field="view"/>
  <fact label="CELL" field="cell"/>
  <fact label="GRID" field="extent"/>
  <fact label="PITCH" field="pitch"/>
  <fact label="ZOOM" field="zoom"/>
  <fact label="CURSOR X/Z/Y" field="cursor"/>
</map-toolbar>
```

* `title` — `text`. Drawn at the left, in full ink. One title per toolbar.
* `fact` — `label` and `field`. The label is the small caption; the value is read from
  `webview::Facts`, which the mount builds from the frame the window is currently showing, its
  own zoom and the cell under the pointer. A field outside this table is a refusal naming it:

| `field` | value |
|---|---|
| `view` | the selected feature token: `all`, `ocean`, `mountain`, … |
| `cell` | one map cell in metres, from the frame's `cell` row |
| `extent` | the frame's declared extent, `columns×rows` |
| `pitch` | the camera's downward tilt in degrees |
| `zoom` | the mount's zoom in doublings (`0` is the default scale) |
| `cursor` | the cell under the pointer as `x/z/y` in metres — world east, world south, ground height — or `—` when the pointer is not on the body |

The cursor's column is weighted two and a half facts wide, because it is the only fact that carries
three numbers; an equal sixth of a narrow map window clipped it to the first value.

The toolbar is chrome (`Role::Chrome`), not the controls' alternative surface, because it is the
window's header rather than another control block.

## `<map-controls>`

```xml
<map-controls>
  <label text="VIEW" class="caption"/>
  <button text="All" action="plan all"/>
  <row>
    <button text="Prev" action="plan prev"/>
    <button text="Next" action="plan next"/>
  </row>
  <spacer height="6"/>
  <label text="ZOOM" class="caption"/>
  <row>
    <button text="−" action="zoom out"/>
    <button text="+" action="zoom in"/>
  </row>
  <button text="Reset" action="zoom reset"/>
  <spacer height="6"/>
  <label text="TURN" class="caption"/>
  <row>
    <button text="◀" action="rotate left"/>
    <button text="▶" action="rotate right"/>
  </row>
  <row>
    <button text="Pitch ▲" action="pitch up"/>
    <button text="Pitch ▼" action="pitch down"/>
  </row>
  <button text="Recentre" action="recenter"/>
  <spacer height="6"/>
  <label text="BIOMES" class="caption"/>
  <button text="Ocean" action="plan ocean"/>
  <button text="Mountain" action="plan mountain"/>
  <button text="Forest" action="plan forest"/>
  <button text="Grassland" action="plan grassland"/>
  <button text="Desert" action="plan desert"/>
  <button text="Tundra" action="plan tundra"/>
  <spacer height="6"/>
  <label text="KEYS" class="caption"/>
  <label text="WASD/ARROWS MOVE" class="caption"/>
  <label text="Q/E TURN" class="caption"/>
  <label text="−/+ ZOOM" class="caption"/>
  <label text="HOME CENTRE" class="caption"/>
</map-controls>
```

**The map is a viewer.** It draws the world's own ground, marks a cell, and is moved and looked at —
a drag, the wheel, and the keys above. It has no control that edits the world: the terrain's own
modification is the shapers' over the world's cadence tick (`voxel_shapers::stages::Stage::Earthworks`),
not a button on a map (see *The map edits nothing*, below).
```

| tag | attributes | layout |
|---|---|---|
| `label` | `text`, `class` (`heading` or `caption`, default caption) | one line; 18 dp heading, 14 dp caption, plus a 4 dp gap |
| `button` | `text`, `action` | full content width, 24 dp tall, plus a 2 dp gap |
| `row` | — | children share the width equally with a 4 dp gap; each is 24 dp tall |
| `spacer` | `height` (default 6) | vertical space only |
| `rule` | — | a 1 dp divider line plus 5 dp |

Children are stacked top to bottom. The gutter is `30 %` of the content width, clamped to
`120..168 dp`; the map window's declared minimum width (`240 dp`) therefore leaves a 119 dp map and
a 120 dp gutter.

**The column is taller than the window, and the wheel over the gutter scrolls it.** The shipped
document is **524 dp** of controls at the map window's declared `480 × 440` size (VIEW, ZOOM, TURN,
BIOMES, Recentre and the key captions), against **342 dp** of column below the toolbar — so a control
below the edge would simply not exist. `webview::render` therefore takes a **scroll offset
in dp** and reports the document's own height (`Rendered::content_dp`), the mount keeps the offset
(`Mount::map_controls_scroll`, clamped by `Mount::map_controls_scroll_max`), and the wheel moves it
in whole rows (`MAP_CONTROLS_ROW_DP`) so **what is drawn is drawn whole**: an element this model has
no scissor for would otherwise paint a halved button over the toolbar above it. The wheel is one
entry point ([`Mount::map_wheel`](crates/voxel-client/src/shell/mount.rs)) answering by region —
over the ground it is the camera's zoom, over the gutter it is the column's scroll — and the last
step reaches the document's own end whatever the row grid says, so the final control is reachable.
A scroll is a fine place for the blocks a person reaches occasionally — the biome views, and the key
captions, which are read once.

## Actions

A button's `action` is a verb string, and the same string is executed by a pointer press and read by
the console's `plan` verb where they overlap. `Mount::map_action` owns the vocabulary:

| action | effect |
|---|---|
| `plan all` | the frame that claims every cell |
| `plan <feature>` | the declared view with that `feature` token; an unknown token is refused |
| `plan next` | the next declared view, cyclically (defaults to the first) |
| `plan prev` | the previous declared view, cyclically (defaults to the last) |
| `zoom in` | one step in on the map camera, clamped at [`MapCamera::MAX_PIXELS_PER_M`] |
| `zoom out` | one step out, clamped at [`MapCamera::MIN_PIXELS_PER_M`] |
| `zoom reset` | back to the default scale, still looking at the same column |
| `rotate left` / `rotate right` | one 15° step of bearing, and the survey resamples under it |
| `pitch up` / `pitch down` | one 6° step of tilt, clamped to `8…84°` |
| `recenter` | the camera back to the frame's declared centre — the survey's centre, unchanged in scale |
| `move up` / `move down` / `move left` / `move right` | **one screen step of the view** (`MAP_MOVE_STEP_DP`, 48 dp), answering the camera's own centre in metres. The ground slides the way the key points — the same gesture as a pointer drag the *other* way (`MapCamera::pan_by_screen`) — and the step is a screen distance rather than a world one, so a key carries the same share of the window at every zoom. Refused with *"the map window is not placed: there is nothing to move over"* when no window is on screen |

The `plan prev` button is why `select_feature` carries a `prev` arm: a drawn button whose action no
handler answers is a control that does nothing. The same is true of the TURN group: every one of
these verbs is answered [by `Mount::map_action`](crates/voxel-client/src/shell/mount.rs), and the
buttons are drawn **after** the raster so a control is never painted over by the map.

**The keys run the same strings.** `main.rs`'s `map_action_of` is one table from a physical key to an
action above — arrows and WASD to the moves, `Q`/`E` to the turns, `−`/`+` to the zooms, `Home` to
`recenter` — so a key and a gutter button cannot become two spellings of the same move, and the
scripted pointer drives the keys' own path (`key <name>`).

## The palette contract

This is the rule the first version broke. The controls and the toolbar may use:

* `Role::SurfaceAlt` for the controls panel;
* `Role::Chrome` for buttons and the toolbar;
* `Role::Chrome` elevated by **one** step for the button of the view that is showing;
* `Ink::Full` for headings, values and the selected button's label;
* `Ink::Secondary` for captions and resting button labels;
* `Ink::Divider` for rules and resting underlines.

They must **not** require `Role::Accent`, `Role::Divider` or `Role::Secondary`. Those roles are
deliberately unadmitted in `Palette::declared()`; `theme.rs` names the accent as this tree's one
measured debt. A renderer that required them could never draw with the declared palette, and the
mount used to turn that refusal into an empty element list — which looked exactly like a window with
no controls. `Paints::of` is now the one place the contract is built, and both documents share it.

## Failures

* An unknown tag, an unknown fact field, a `button` without `action`, a `fact` without `field`, a
  document with more than one root, or an unclosed tag is an **error naming the offending text**.
* A closed or minimized map window is **not** an error: it returns an empty element list, because
  there is nothing to draw.
* `Mount::map_toolbar_elements` and `Mount::map_controls_elements` return
  `Result<Vec<Element>, String>`. The game prints a changed refusal once (stderr), the shell-shot
  example panics, and no caller can silently substitute an empty gutter for a broken document.

## Pointer and selection

`Mount::map_control_at(point)` renders the same document into the same gutter and returns the
`action` of the button containing the point. The pointer handler then calls `Mount::map_action` with
that string. The toolbar has no buttons; it only reads.

**The map's body is itself a pointer surface**, and it is four calls rather than one, because a
drag has to mean a pan and a click has to mean a selection:

* `Mount::map_press` starts a drag at the point;
* `Mount::map_drag` pans the camera by how far the pointer has come, and keeps the distance;
* `Mount::map_release` ends it: a press that travelled more than 4 dp was a pan and selects nothing;
  a press that did not selects the cell — or clears the selection when it is the same cell — and
  records the cursor, because a click is also a hover;
* `Mount::map_wheel` scales the camera about the pointer, so the ground under the cursor stays put;
* `Mount::map_hover` records the cell under the pointer, which the toolbar's `cursor` fact reads and
  which the body outlines in white; `Mount::map_pointer_left` forgets it.

The selection is the window's own state (`Mount::map_selection`, set directly with `Mount::select_map`
for a restored view or a capture). There is one selection, so the route and the toolbar cannot
describe different cells.

## The keys

**The focused window owns the keyboard**, and the map is the second window to claim a share of it —
the console's line edit is the first. `Mount::map_has_focus` is the question the app asks before it
routes a key (`App::map_key`), so while the map holds the keyboard a held `W` walks the *map* and not
the player, and a key the map does not use is not consumed at all.

A person gives the map the keyboard the way they give any window the focus: **a press on its body**,
which `App::shell_press` routes to `Mount::focus_map` — the pan the press starts and the keys it takes
are one gesture. **A press that lands on the world takes it back** (`Mount::blur`): digging is the
plainest statement that the person is done with the windows.

| key | action |
|---|---|
| `↑` `↓` `←` `→`, `W` `A` `S` `D` | `move up` / `move down` / `move left` / `move right` |
| `Q` / `E` | `rotate left` / `rotate right` |
| `−` / `+` (and the numpad's) | `zoom out` / `zoom in` |
| `Home` | `recenter` |

The platform's key repeat is what makes holding one a walk: each repeat arrives as a press and runs
one more step. The keys are drawn in the gutter's `KEYS` block, because a key nobody is told about is
a key that does not exist.

## What the map draws

| mark | drawing |
|---|---|
| the player | `@` on a dark plate, inside an eight-cell cuboid frame (`┌ ─ ┐ │ │ └ ─ ┘`) |
| the heading | the nearest of eight arrow lobes (`▲ ◥ ▶ ◢ ▼ ◣ ◀ ◤`) to the player's compass bearing, one row above the frame. The vocabulary has no rotated glyph, so eight lobes is where the spin stops being legible; `arrow_of_degrees` is the only place it rounds |
| the selected cell | `◇`, in the selection's ink |
| the route | the Bresenham line of cells from the player to the selection, clipped at the map's edge, drawn as inset squares in the route's ink |
| an off-screen player or selection | the point clamped to the map's edge and drawn as `◆` (player, red) or `◇` (selection), with the bearing from the view's centre kept in `MapTerrain::edge` |
| the ground | a raster: six sky bands fill the body, then every `4 dp` column is walked from the bottom of the body up, the inverse of the projection is iterated to the surface, and rows of one colour are merged into runs. Below sea level it is water, deeper blue with depth; a selected view keeps its own ground lit and shades the rest (the ground the selection does not name). **Every rect the raster emits is clipped to the body it is drawn in** — a run's overlap is for a seam inside the picture, and the map does not paint outside the window its package declared for it |

The player is drawn by `MapTerrain::follow`, once per frame: the survey is sampled per view and the
marker rides it, so a step costs one pose write rather than a thousand samples.

## The map camera

**One value is the whole picture** — [`MapCamera`](crates/voxel-client/src/map_view.rs):

| field | meaning |
|---|---|
| `centre_m` | the world column at the centre of the body, metres |
| `pixels_per_m` | how big a metre is. **One scale for both axes** |
| `bearing_deg` | which way is up the screen, clockwise from north |
| `pitch_deg` | how far down the camera looks: `90°` is straight down, `8°` skims the ground |

```
body pixel  =  rect.centre  +  right · px/m
                             +  forward · px/m · sin(pitch)
                             −  height  · px/m · cos(pitch)
```

A metre east and a metre north are the same number of pixels whatever the window does, so widening or
maximising the window **reveals more world instead of stretching it** — the two captures in
[`docs/baselines/map-survey.json`](baselines/map-survey.json)'s recipe were taken at the same
`pixels_per_m`, maximised at 1280 × 720 and at 1920 × 1080: 576 × 744 m of ground in the first and
896 × 1 184 m in the second, at one world scale.

**The pointer is the same projection read backwards.** `project` foreshortens the ground by the pitch
(a metre of forward is `pixels_per_m · sin(pitch)` pixels up the body), so `depth_of` reads a pixel
back by that same ratio and the height refinement adds `h · cot(pitch)` back before picking. Without
that the click landed in a cell metres from the one under the pointer — a 17-sample miss at 8 px/m —
which is what `map_view::tests::the_pointer_picks_the_world_column_a_tilted_camera_is_pointing_at`
now refuses.

`Mount` owns the camera and every control writes to it: the drag (`pan_by_screen`), the wheel and
`zoom in`/`zoom out` (`zoom_by`), `rotate left`/`rotate right` (`rotate_by`),
`pitch up`/`pitch down` (`pitch_by`), `recentre` (`look_at`). `Mount::map_camera_to` sets it outright,
for a restored view or a capture. Nothing else stores a scale.

## The survey

The camera does not draw the world. It asks for a [`SurveyPlan`](crates/voxel-client/src/map_view.rs),
and **[`MapSurvey`](crates/voxel-client/src/map_survey.rs) answers it**: from the streamer's own voxels
where it holds chunks, from the world's own generator at the survey's sample LOD everywhere else, and
from the laws only where the world answers nothing at all:

* the survey is centred where the camera looks, quantised to whole declared cells, so a drag resamples
  once per 32 m rather than once per pixel;
* its span covers **the ground the body can see** — the body's width ÷ `pixels_per_m` across, and its
  height ÷ (`pixels_per_m · sin(pitch)`) down, because the far half of the body is foreshortened
  ground;
* one **sample** is a declared cell divided by `SUB` (4) when the camera is close enough for that to
  mean anything, and coarser when the window shows more ground than `192 × 192` samples of it can
  describe. **The sample coarsens; the picture never stops short of the edge**, which is the rule
  `map_view::tests::the_survey_reaches_the_body_at_every_size_and_scale` asserts over three window
  sizes × four scales × three pitches;
* **the climate is read on the declared cells**, not on the samples, and coarsens with them
  (`MAX_BIOME`): a climate column costs about ten thousand height samples.

A sample is *not* a selection: `MapTerrain::world_of` snaps a sample's position to the declared cell it
falls in, so the toolbar's `cursor` readout, the `◇` mark and the route all name whole map cells. The
sample's own column is `MapTerrain::sample_xz`, which is what a comparison with the law is asked at.

### Where a sample's height comes from

**The picture's heights are the world's own, residency and generator together**, and the law's field
is the fallback where the world answers nothing. Three passes, in this order:

1. **The laws are the base.** `MapTerrain::sample` reads `terrain::elevation_m` and `ecology::biome_at`
   once per distinct plan and holds that grid, so a survey always has *a* picture.
2. **The residency is written over them.** One pass over the chunks the survey's extent covers asks
   each resident chunk for its `32 × 32` column surface (`MapGround::chunk_surface`) and writes the top
   of each column's **topmost solid voxel** over the law's. A canopy, a dug pit and a subvoxel shape's
   own quarter-metre top are what the map draws. This is the count `Observation::from_chunks`.
   **A chunk owns exactly its own `32 × 32` columns** (`ChunkSurface::top` refuses anything outside
   them): the grid is a chunk's, so one past its right edge is the neighbour's metre and not the
   first column of the next row. Answering the *wrapped* entry there is what used to put a second
   chunk's column on a sample that already had an owner — and, when the survey's own grid landed a
   sample's centre exactly on a chunk edge, count that sample twice.
3. **The rest of the window is the generator's own, band by band.** The streamer's window is a few
   hundred metres; a map of a valley is kilometres, and a chunk outside it has never been built.
   [`MapGround::column_surface`](crates/voxel-client/src/map_survey.rs) is the world's own material law
   read at the handful of voxels of **one column**, canopy included and fluids skipped, so the map asks
   the generator itself at the survey's own sample LOD instead of building a chunk to read one column.
   That is `Observation::from_world`, and it is what makes a zoomed-out map show ground rather than a
   smooth re-derivation. A probe measured **22 µs**, and the widest window holds 36 864 samples, so the
   fill reads `FAR_BAND_PROBES` (**1 024**) of them per ground write — one band, ~22 ms — in rectangles
   growing out from the middle of the picture, and **remembers every column it has asked** until the
   world's *record* generation moves (an edit; a chunk arriving cannot change what the generator says
   about a column). Panning back over ground the map has already read therefore costs lookups, and the
   count of the world's own samples climbs until `world_samples() == samples` — the survey is the
   world's own to its edges, and the law's field is left answering only where the world answers
   nothing at all.

The two world sources are named in one place, and the survey reports how many samples came from which
(`Observation::from_chunks`, `Observation::from_world`, `Observation::world_samples`). **The count is a
reading of the sample grid, not of writes**: a sample reached by two chunks is the residency's once.
`main.rs` logs the three numbers when they move, so a run of the game says which part of the world its
map is drawn from.

**A fluid is not the ground.** Water is a voxel material here, so a sea column's topmost *voxel* is its
skin at sea level; drawing that as the surface painted every resident ocean chunk as an island. The
surface is the topmost voxel that is not [`materials::is_fluid`](crates/voxel-shapers/src/materials.rs)
— the one rule the mesh, the movement check and this map all ask — and the sea is drawn by the ink
below sea level, as it always was.

### One owner

`MapSurvey` owns the survey, its keys and its cached ground raster, **beside the residency it reads**
(`map_survey.rs`). `Mount` calls `map_observe(world, pose, dt_s)` once per frame — the world (the
ground, the spec, the tick) and the player's column and heading — and the app holds **no terrain and
no key**: `App::observe_map` is one call, a line of console output when the survey moves, and nothing
else. The camera, the selection and the cursor stay the mount's, because they are the window's state
rather than the world's.

### Where it is drawn

The map is a **window's content**, so the mount draws it inside `window_elements` — the layer's own
window order ([`Mount::map_elements`](crates/voxel-client/src/shell/mount.rs)) — and not stitched in
by the caller after the cell raster. A window drawn after the map covers it, a window drawn before it
is covered by it, and nothing the map emits leaves the window's content container. **The frame's
declared cells are the fallback**: once the survey is live, `Mount::frame_space` withdraws the map's
frame, so the static picture cannot read through the live ground.

**A run overlaps its neighbours by half a row step and its column by half a column**
(`RUN_OVERLAP_DP`). Two rects that merely abut leave one pixel of sky between them wherever a
renderer rounds both edges, and 8 169 pixels of a 1920 × 1080 body were exactly that: a dotted field
of pale dashes in the water. The paint is opaque, so the overlap costs a few tenths of a pixel and
buys a continuous ground.

**A view selection is one filter over the ground, read from the frame's own `feature` token**
(`ViewFilter::of_token`). `all` keeps everything, a biome token keeps that ground and shades the rest,
and `ocean` keeps the water. An unknown token filters nothing, because a control that filters nothing
must not look like it filtered something. The raster's cache key carries the filter, so the picture
can never lag the selection it names in the toolbar.

## The map edits nothing

**The terrain's modification is the shapers', not this window's.** What shapes the world in real
time is the shaper roster over the world's cadence tick — `voxel_shapers::stages::Stage::Earthworks`
is declared `Cadence::EveryTick` — with `profile32`'s subvoxel shapes and the earthworks record
composed last at the coordinates the record names. The map draws that ground and has no control that
writes it: no brush, no lift, no apply. A map window that struck ground was the misreading this
section replaces.

The world-side machinery stands where it was, and is where a stroke belongs if the shapers' own path
takes it up:

* [`Streamer::shape_columns`](crates/voxel-work/src/stream.rs) — the stroke's door into the record:
one `Modification` per raised or lowered voxel, through the streamer's own edit path.
* [`MapBrush` / `MapEdit` / `columns_within`](crates/voxel-client/src/map_view.rs) — the stroke's
**modeller**: where a brush is, how wide, and which whole-metre columns it covers. It is parked in
`map_view` with its own tests, and **nothing on the map stages one** — the app no longer drains a
window's stroke.

What the world does with a stroke, when one is handed to it:

* **Per voxel, because the record is per voxel** ([`earthworks`](crates/voxel-shapers/src/earthworks.rs)):
one `Modification` per raised or lowered voxel, the player as the actor, the verb `fill` or `dig`, and
the reason the caller names (`"the map's hammer"` was the old caller's words). There is no region
door to take instead, and pretending there were would be a second record.
* **The material is the column's own.** A raise reads the surface's topmost non-fluid voxel and places
that material above it, one per metre, so a grassy hill rises as grass; a lower removes the topmost
voxel. A column of air is **skipped**, not filled, and a stroke that meets water is **refused**
(`hand_change` will neither mint nor delete a fluid) — reported as `skipped` and `refused` counts,
not as a silent hole.
* **The picture is the world, so the change is the map.** The survey's next ground write re-reads the
chunk the stroke reached, so the ground the map draws moves by exactly the lift. A column far outside
the residency moves too: the far probe reads the record composed over the generator, and its walk
starts and ends at **the record's own reach** (`Earthworks::column_span`) because the law's ceiling is
not the world's — a raise can stand above the generator's canopy ceiling and a dig can cut below its
floor.

**A pinned map wears none of it.** The toolbar and the gutter are drawn with the rest of the map's
content, so `map_chrome_shown()` (false while pinned) takes them both: the minimap form spends its
whole body on the ground, and the ground's rect *is* the content container.

## Adding a view or a control

1. Add a `row MAP-<id> <description>` to `display-packages/alpha-shell/package.shspec` (and the
   byte-identical `packages/alpha-shell.shspec`).
2. Give its frame a `feature` token and the GPU map keys (`field`, `shaper`, `projection`, `centre`,
   `cell`, `extent`, `pitch`), then run
   `cargo run -p voxel-package --bin voxel-mapframe -- --check` to regenerate the frames.
3. Add `<button text="…" action="plan <token>"/>` to `MAP_CONTROLS`.
4. Point the camera with `Mount::select_feature` / the console's `plan <token>`.

For a new readout, add a field to `webview::Facts`, fill it in `Mount::map_facts`, and add a
`<fact field="…"/>` to `MAP_TOOLBAR`. For a new structural tag, add an arm to `stack` (controls) or
`render_toolbar` (toolbar) — an unhandled tag is a refusal, not a no-op.

## The measured survey

The sample counts, the shares that are the residency's, the generator's and the law's, the observation,
fill and compose times and the composed element count are recorded in
[`docs/baselines/map-survey.json`](baselines/map-survey.json), with the shell-shot recipe that produced
them — the same baselines format the renderer's frame-cost and light-coverage rows use. The recipe
builds a real `Streamer` (seed `GAME_SEED`, radius 2 chunks) and settles it over the frame's declared
column, and it fills the survey the way the game fills it (one ground write per band) before it
composes, so the capture's ground is a world rather than the laws alone. **A run of the game is recorded
in the same file**: `SHPLOINGUS_SHELL="…:open map-plan,…:map zoom out,…"` with the map zoomed out to the
scale floor, where `main.rs`'s own line reports `14 400 of 14 400 samples from the world's own ground —
16 resident, 14 384 asked of the generator (0 still the law's)` — the whole window the world's own, at a
scale where the residency is 16 samples of it.

## Tests

* `shell::webview::tests` — parse, every button has an action, the active view is one elevation and
  full ink, the declared palette draws over an opaque panel, the toolbar reads the facts, and an
  unknown tag or field is refused.
* `shell::webview::tests::a_taller_document_is_drawn_from_the_offset_and_reaches_its_own_end` — a
  rectangle shorter than the document: the end of the column is drawn at the document's own height
  as the offset, nothing a scrolled pass draws escapes the gutter, and **every declared control is
  drawn at some offset** — the claim a scrolling column makes.
* `shell::mount::tests::the_map_controls_draw_and_the_pointer_resolves_the_button_under_it` — the
  toolbar and gutter are drawn from the shipped package, the pointer resolves `plan ocean`, a point
  outside is no control, the toolbar sits above both regions, the map does not paint the gutter, and
  a minimized window is an empty list rather than an error.
* `shell::mount::tests::the_map_composes_in_the_shells_layer_from_the_worlds_own_voxels` — one
  `map_observe` places the camera on the declared column and pitch, the survey's heights are the
  ground's inside the residency and the law's beyond it, the picture fills the body and stays inside
  the window's content container at 1280 × 720 and 1920 × 1080 at one world scale, and the map's
  declared frame is withdrawn once the survey is live.
* `shell::mount::tests::the_map_body_draws_the_player_and_a_click_selects_the_cell_under_it` — the
  marker and its arrow are drawn, a body click selects and re-clicking clears, the toolbar's cursor is
  the clicked cell's world position, a drag pans without selecting or zooming, the wheel zooms about
  the pointer, and a point off the body leaves no stale cursor.
* `shell::mount::tests::the_map_moves_like_its_drag_and_declares_no_control_that_edits_the_world` —
  **no declared control names a hammer or a lift**, in the gutter or in the toolbar's facts;
  `move right` is exactly the drag left and `move up` exactly the drag down, one screen step each
  (so the same key moves less ground when the map is zoomed in); a hover puts the toolbar's cursor on
  the cell under the pointer; **every control the document declares is drawn at some scroll of the
  gutter** (the wheel over the gutter scrolls it a row at a time to the document's end, and nothing
  it draws leaves the gutter); a wheel over the ground still zooms; a pinned map draws no chrome and
  its body takes the gutter's width, with the chrome back on unpin; and a window that is not on
  screen refuses a move by name.
* `shell::webview::tests::a_taller_document_is_drawn_from_the_offset_and_reaches_its_own_end` — the
  document's own last button is the one the end of the column shows, so a control appended to the
  document is covered without the test naming it.
* `map_view::tests::the_hammer_stages_a_disk_and_a_mark_freezes_where_it_lands` — the parked stroke
  modeller: the footprint over the world's lattice, the ladder's ends staying put, the mark's radius
  frozen against a later resize, `clear` forgetting the mark and the lift.
* `map_view::tests::the_hammer_draws_the_brush_and_the_mark_at_their_own_widths` — two rings where
  both are set, in the brush's and the mark's own ink, the wider one covering more ground, and each
  inside the body it is drawn in. No window draws them any more; the modeller is what they cover.
* `map_survey::tests::a_hammer_stroke_reads_back_as_the_maps_own_ground` — a real streamer, one
  column raised three metres through `shape_columns`: after the chunk rebuilds, the survey's next
  ground write draws the surface the stroke left.
* `map_survey::tests::the_live_maps_own_stroke_reads_back_at_the_column_it_struck` — the same claim
  at the live run's own seed, window and brush (`x = 16, z = −48`, 16 m, +8 m): the sample nearest the
  strike moves by the lift, which is why the game's map moves rather than this being a wiring mystery —
  **and the stroke survives a resample of the laws**: the next climate bucket (`TICK_BUCKET`, the moment
  the live run prints `laws resampled`) re-reads every law's field and draws the stroke's ground again
  rather than the elevation the generator would have put there.
* `voxel-work::stream::tests::a_hammer_stroke_raises_the_worlds_own_surface_and_records_it` — the
  stroke's own door: the surface moves by the lift read back through `column_surface`, a dig moves it
  down, the record holds one entry per voxel in the hammer's words, and an empty, zero or over-limit
  stroke is refused by name.
* `map_survey::tests::a_live_streamers_own_voxels_are_what_the_map_draws` — a real streamer, settled,
  walked column by column with an independent read of the same rule: the map's height for a sample is
  the world's own topmost solid (non-fluid) subvoxel, and at least one sample disagrees with the law.
* `map_survey::tests::the_far_ground_is_the_generators_own_and_the_residency_wins_where_it_holds_one`
  — a survey far wider than the residency: the chunk's samples are its sheet, every other sample is
  the generator's, and the law answers nothing in the picture.
* `map_survey::tests::the_far_fill_climbs_a_band_at_a_time_to_whole` — one band a write, the count
  climbing, and `world_samples() == samples` at the end without the plan moving.
* `map_survey::tests::a_sample_on_a_chunk_boundary_is_one_chunks_own_and_is_counted_once` — the plan
  whose sample centre lands exactly on a chunk edge: the world cannot answer more samples than the
  survey holds, the fill reaches whole, and the height is the column's owning chunk's.
* `map_survey::tests::a_chunks_surface_refuses_a_column_outside_its_own_grid` — one past a chunk's
  right or bottom edge is nothing, not the wrapped entry of the next row.
* `shell::mount::tests::an_unknown_map_action_is_refused_as_an_action_and_an_unknown_view_as_a_view`
  — `zoom wheee` is refused as an action, a bare or `plan `-prefixed token as a view.
* `map_view::tests` — the marker and arrow follow the pose, the frame is the eight cells around it
  (and drops the cells at the grid's edge), an off-screen player clamps to the edge and keeps its
  bearing, the route runs from the player to the selection and clips at the edge, and the cursor
  reads a cell's world position.
* `map_view::tests::the_scale_is_one_number_and_a_resize_does_not_change_it` — the same world point
  projects the same distance from the centre of a 320 dp body and a 1280 dp one, and a metre east
  and a metre north are the same number of pixels apart from the pitch.
* `map_view::tests::pan_zoom_rotate_and_pitch_move_the_camera_and_nothing_else` — a pan moves the
  centre and not the scale, a zoom about an anchor keeps the ground under it, a rotate brings east
  up-screen, and both tilt and scale clamp.
* `map_view::tests::the_survey_reaches_the_body_at_every_size_and_scale` — the coverage law above.
* `map_view::tests::a_view_selection_lights_its_own_ground_and_shadows_the_rest` — each view token's
  own vocabulary (an unknown token filters nothing), that a selection changes the raster, that it
  never draws ground brighter than the survey already had, and that the mean dims.
* `map_view::tests::the_raster_covers_every_pixel_of_the_body` — no column leaves a pixel of sky
  below its ground, at two body sizes and three pitches: the seam law as a law.
