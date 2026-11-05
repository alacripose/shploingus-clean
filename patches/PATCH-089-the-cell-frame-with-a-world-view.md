---
patch_id: PATCH-089
title: The cell frame with a world view — the per-axis 1/32 cell and the map's selectable features
status: applied
operation: feature
target: voxel-model/profile32 + voxel-client/shell/map-plan
base_ref: working tree at 2026-10-26, after PATCH-088
dependencies: [PATCH-087, PATCH-088]
source: user-directed-mission
created_at: 2026-10-26
approval: "user-directed 2026-10-26 — 'Finish patching the profile32 function … the per 3D axis arithmetic format for shaping voxel cells at the 1/32th per axis … Patch in the axis cell frame for the map window for browsing the world map and biomes … the cell frame needs to allow me to browse a selection of biomes or world features like mountains or oceans … mount_v2 is the baslines format, so you can bring in the necessary world features for the cell frame'"
live_mutation: false
---

# Two records, one denominator — and the one the world needs

`profile32` already held `Shaped`, a body of revolution for a trunk or a crown. A terrain cell is
not a surface of revolution: a ramp leans across both horizontal axes, a cliff has a side, and a
corner is three planes at once. `docs/AUDIT-2026-10-24-where-a-package-ends-and-the-game-begins.md` §6
named the gap exactly — *"there is no 1/32 profile shaper — `FILL_SIDE = 32` is state granularity, not
a surface law"* — and the terrain shapers that needed a per-axis cell had nothing to answer with.

The law now has a second record at the same denominator:

```text
Cell      { cuts: [Option<AxisCut>; 3] }          one cut per world axis, or open
AxisCut   { at, inside, lean: [f64; 2] }          a plane with a point-based gradient
```

`plane(p) = at + lean[0]·(p[i] − ½) + lean[1]·(p[j] − ½)`. Coverage is `inside · (p[axis] − plane)`
anti-aliased over **one 1/32 cell measured perpendicular to the plane**, and a cell with three cuts
is their product: outside any one cut is outside the cell. `Cell::normal` returns the normal of the
cut whose coverage is smallest at that point, and a corner is stated as the lowest-axis cut rather
than an invented average. `Cell::volume` is the witness — the mean coverage over the 32³ grid — not
a frame cost.

| witness | value | what it proves |
|---|---|---|
| a solid block | 1.000000 | the denominator's whole |
| a plane through the block's centre | 0.500000 | a half-space is a half |
| a leaning top `y = ½ + ½(x − ½) + ¼(z − ½)` | 0.500000 | the lean is a plane, not 31.25 mm stairs |
| a corner of three cuts | 0.125000 | the intersection is an eighth, not three halves |

**The column is not a 3D shape here.** The user's ruling in this pass was explicit: *"column is only
for the 2d cell renderer."* A column-shaped record belongs to the shell's raster and its map window;
the world is cubic, a block is a cuboid, and `Cell` addresses its interior at 1/32 per axis. Nothing
in the 3D path was bent into a column to make the map easier.

## What this does and does not admit

The **format** the refused terrain shapers were missing now exists, with the tests above. The
**renderer carrier** does not: no mesh carries a `Cell` per voxel and `shading.wgsl` has no arm that
evaluates the three cuts. That is the same owed half `docs/SHAPER-PROFILES.md` already names for
`Shaped`, and this record does not pretend otherwise. A shaper can be asked what it occupies today;
a drawn face cannot yet read that answer.

# The map window is a measurement, not a drawing

`MAP-0001` used to be a hand-drawn lattice with invented contours. It is now generated from the
world's own laws — `terrain::elevation_m` and `ecology::biome_at` on the same Holdridge inputs the
chunk builder uses, at the same seed (`9 418 646 105 956 113 635`) and tick (3/8 of a day). Each
view is the same 56 × 22 frame, the same 54 × 17 measured disc at 32 m a cell.

The frame grammar gained **one optional key**, `feature` (lowercase token, at most 24 characters).
It is optional so every existing frame still parses; it is validated because the token is typed at
the console. Seven frames are declared for `map-plan`:

| frame | feature | centre | selected cells of 918 |
|---|---|---|---|
| `MAP-0001` | `all` | spawn `0.5, −95.5` | 918 |
| `MAP-0002` | `ocean` | nearest water, 128 m from spawn | 578 |
| `MAP-0003` | `mountain` | range at `2488, 1917` | 49 |
| `MAP-0004` | `forest` | stand at `−63, 15` | 89 |
| `MAP-0005` | `grassland` | `128, −95` | 86 |
| `MAP-0006` | `desert` | `727, 154` | 42 |
| `MAP-0007` | `tundra` | `65, −206` | 204 |

**There is no eighth view for tropical rainforest.** The ring search walked 128 m rings to 12.3 km
and found none; the world at this seed has no such biome near spawn. A declared view that selected
zero of 918 cells would be a blank selection, so the absence is stated here and in
`docs/baselines/map-frame.json` rather than papered over with a near-biome threshold.

A selected view keeps the whole world drawn: cells that do not answer the feature are in the
schema's `disabled` ink, and only the selected cells keep their biome's ink. Switching views is a
highlight on one measurement, not a second map with different geography.

The console's existing `plan` verb is the selector — it was in the reference's surface with exactly
this name and one argument, and had been refused for want of a window whose content is a view of the
world:

```text
plan ocean       the map shows the ocean view
plan next        the next declared view, cyclically
plan all         the default view
plan volcano     refused by name, and the drawing does not change
```

The frames are all graded at mount; `plan` never re-reads a file, and a reload keeps a selection only
while the new package still declares that token.

## The world this map reads, named and not re-invented

Nothing in the 1/8 scale, the far horizon or the clouds changed in this pass. They are named so the
record does not read as if they were invented here:

* **1/8 scale** — `voxel_shapers::scale::REAL_METRES_PER_VOXEL = 8.0`: one engine metre stands for
  eight real metres, so a planetary atmosphere fits in the column. The map's 32 m cell is 256 real
  metres, and the legend prints engine metres rather than confusing the two.
* **Distant terrain** — the surface ladder's 32 rungs and the 8 m / 16 m light bands are the horizon
  renderers (`crates/voxel-client/src/lod.rs`, `horizon.rs`, `far_terrain.rs`); the octree's finest
  address is the 0.25 m subvoxel. The map does not draw any of them: it is a 2D reading, one screen
  cell a world square.
* **Clouds over terrain** — `voxel_shapers::clouds` already lifts cloud base by
  `OROGRAPHIC_LIFT_M · upslope` and carries fronts, genera and phases, so a range's cloud is not the
  flat ground's cloud. The biome and cloud PDFs under `docs/` are the sources those laws were
  written from; no PDF was regenerated in this pass.
* **The 1/32 law's carrier** — still owed, as above.

# Evidence

* `cargo test -p voxel-model --lib profile32` — **9 passed, 0 failed**. The per-axis tests are
  `a_plane_shapes_a_cell_at_one_thirty_second_an_axis`,
  `a_leaning_top_is_a_plane_and_not_a_staircase`, and
  `three_axis_cuts_shape_a_corner_in_three_dimensions` (which also covers a side cut and the
  limiting-cut normal); the six tree-profile tests still pass beside them.
* `cargo run -p voxel-model --example profile32-shapes` — the leaning top prints
  `volume: 0.500000 of the block` and the three-cut corner `volume: 0.125000 of the block`, exit 0.
  The same witnesses are rows in `docs/baselines/cell-law.json`, with the one-cell skin at
  `0.03125`, so a later shaper cannot move the denominator without moving a named row.
* `docs/baselines/map-frame.json` — the seven views' selected-cell counts (`918, 578, 49, 89, 86,
  42, 204`) with the zero-rainforest note. Both JSON files parse with `python -m json.tool`.
* `cargo test -p voxel-package --lib cellframe::` — **14 passed**. The new grammar test proves
  `feature` is optional, refused twice, and refused with a token the console could not type; the
  old unknown-key, duplicate-key, missing-key, out-of-grid and no-drawing refusals still pass.
* `cargo test -p voxel-package --bin voxel-mapframe` — **4 passed**. The default and ocean views
  parse, the picture is 56 × 22, all four cardinal letters are in the border, and the ocean view
  keeps land in `disabled` ink with no land biome left in full ink inside the map rows.
* `cargo run -p voxel-package --bin voxel-mapframe -- --check` — all **7 views** are the
  generator's own pictures and records, exit 0.
* `cargo run -p voxel-package --bin voxel-layers -- display-packages` — `frames 7`, seven `row`
  lines and seven one-line frame descriptions including `feature ocean` … `feature tundra`, exit 0.
* `cargo test -p voxel-client --lib shell::mount::tests` — **72 passed, 1 ignored**. The new test
  drives `plan ocean` through `Command { verb: Verb::Plan, .. }`, reads the ocean frame back from
  `frame_of("map-plan")`, proves one placed frame, cycles with `next`, returns with `all`, and
  refuses `volcano` without changing the drawing.
* `cargo test -p voxel-client --test cell_version_refusal` — **2 passed**; the existing contract
  that a mounted package places exactly one frame and that `cells 0` returns the composed elements
  still holds with seven frames declared.
* `cargo test -p shploingus --test packaged_frame` — **1 passed**: the build embeds the seven frames
  and the mount grades them from the package alone.
* One stale `voxel_client` test process from an earlier pass held the linker's output file
  (`LNK1104`); it was terminated by explicit Windows PID 19156 after a plain `kill` could not reach
  it. No server and no user process was signalled.
* The game binary's scene-probe format string was one `disc {}` placeholder short, so every later
  argument was shifted and `shploingus` did not compile. The format string was corrected to
  `tick / why / disc / near / quads / eye-chunk / ground / y / …`; no value, probe or rendering
  changed, and `cargo test -p shploingus --test packaged_frame` now builds the bin.

# Rollback

Delete `display-packages/alpha-shell/frames/MAP-0001..0007.frame` and `.md`, the seven `row MAP-…`
lines in `display-packages/alpha-shell/package.shspec` and its byte-identical copy
`packages/alpha-shell.shspec`, and `docs/baselines/map-frame.json`. The package then declares no
frame; the mount registers none, and every window draws composed elements as it did before the
frames existed. `voxel-mapframe` becomes a generator with no outputs to check. Reverting the
`Cell`/`AxisCut` block in `crates/voxel-model/src/profile32.rs`, the `feature` key in
`crates/voxel-package/src/cellframe.rs`, and the `frame_feature` selection in
`crates/voxel-client/src/shell/mount.rs` returns the code to PATCH-088; no stored world data is
touched, because no cell is stored.
