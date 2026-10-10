# Handoff — clouds as cellular-automaton cubes, and the crash they cause

Updated: 2026-10-08.
Goal: **the cloud is drawn as cubes built from the fluidics cellular automaton**, and the
freeze-then-crash the wispy cloud pass causes is gone.

This file is **derived, not authority** (`record::DocumentRole::Derived`, the same rule
`docs/HANDOFF.md` is under). It points at the record; it does not extend it.

## Read first

- `vows.md` (repo root) — the build protocol: gather **all** context, produce **one** written
  plan, **ask the user for approval before the first edit**, implement only on approval.
  Docstrings are at most 1–2 sentences. No scaffolds, no mocks, no shortcuts, no deciding
  without asking.
- `docs/HANDOFF.md` — the standing authority list. `docs/LEDGER.md` — the current record.
  User rulings and the ledger win; outside projects (including `F:\planet-shploingus`) are
  reference for mechanisms, **not authority** (C-48/C-49).
- **Run tests exactly as the docs write them — no `| grep`, no `| tail`** (C-45). A piped run
  hides the exit status and is not a run.
- This tree has **large uncommitted changes** (the user's own work in progress:
  `crates/voxel-client/src/shading.wgsl`, `sky.wgsl`, `world.rs`, `renderer.rs`,
  `crates/shploingus/src/main.rs`, …). **Never revert, stash or discard any of it.** Several
  changes described below are also uncommitted; they are listed so they are not redone.

## What the user asked for, in their words

- *"I WANT THE CLOUDS TO BE CUBES."*
- The clouds are *"the wispy pieces of non-cuboid shit that cause the crash."*
- *"we have the cube system in the game already … it's a cellular automata and you invented a
  cloud shader that doesn't render using the cellular automata's cubes."*

## The defect as reported

Since the cloud pass landed, the game **freezes and then crashes** once clouds are drawn.

Two screenshots the user attached (temporary paths; copy them into the repo if they should be
kept):

- `C:\Users\ala\AppData\Local\Temp\freebuff-desktop-pastes\paste-1791467548293-27996.png` —
  the frame **after the crash**: scattered shaded squares. This is the post-crash surface, in
  the user's words; it is not the cloud render and must not be read as one.
- `C:\Users\ala\AppData\Local\Temp\freebuff-desktop-pastes\paste-1791468280614-27996.png` —
  the game **before** the crash, showing the wispy cloud streaks in the sky.

Leading hypothesis, **unproven**: the fullscreen march hangs the GPU (Vulkan device loss →
freeze → crash), and the post-crash squares are the stale/undefined surface. Prove it before
repairing it: run the game with a **cleared** cloud field (`write_clouds` with a clear
`CloudField`) — if it stops crashing, the pass is the cause.

Per-pixel cost as it stands, for scale: up to 96 steps (`MAX_MARCH_STEPS`, 64 m each), each
step a 4-tap bilinear (`column_at`) plus a 4-step `beam_to_sun` — of the order of **1,600
buffer taps per pixel, per frame**, at the window's own resolution.

## The cube system that already exists — use it, do not build a second one

| what | where | what it gives |
|---|---|---|
| the cellular automaton | `crates/voxel-shapers/src/fluidics.rs` | `Lattice`, `Cell`, `settle(dt)`, `FillLevel`, `FILL_LEVELS = FILL_SIDE` |
| the cells are cubes | `crates/voxel-model/src/coord.rs` | `CELL_EDGE_M = 1 m`, `FILL_SIDE = 32`, `CHUNK_SIDE = 32`, `ChunkPos { x, y, z }` (already 3-D) |
| a cell's cube **drawn** | `crates/voxel-client/src/shell/changes.rs` | `boundary_faces()` / `outline_edges()` — six faces, twelve edges per isolated cell; the change view's own geometry |
| the cloud as a **gas** in the material system | `crates/voxel-shapers/src/clouds.rs::rain_fill_at` | returns a `FillLevel` on the fluidics lattice — the CA encoding the clouds were meant to use |
| fluids' *surface radiance* | `crates/voxel-appearance/src/fluid.rs` | a water-surface model only; **not** a cube renderer |

## The cloud render today — the thing to replace

- `crates/voxel-client/src/clouds.wgsl` — the fullscreen march: composites the sky itself, does
  its own bilinear on the field, and owns a copy of the display transform.
- `crates/voxel-client/src/cloud_field.rs` — the CPU field. `CloudField::sample` asks the model
  on a declared **128 m** sample lattice (`CLOUD_SAMPLE_STRIDE = 2`) and interpolates between
  samples. `CLOUD_COLUMN_M = 64`, `CLOUD_COLUMNS = 128`, span 8,192 m, `CLOUD_BLOCK_M = 2,048`.
- `crates/voxel-client/src/world.rs` — the pass, its layout (`cloud_layout`: uniform, columns,
  sky table), and `write_clouds`.
- `crates/shploingus/src/main.rs` — `Clouds { field, sun }`, `cloud_field_of`, written on the
  atmosphere cadence (~line 982 and ~1447).

## The cube arithmetic the design has to settle (**ask the user; do not decide alone**)

The field is 8,192 m across, ~28 % of its columns hold a cloud (measured), and the slab runs
~500 m to ~2,300 m at the shipped tick.

- **1 m CA cells** (the lattice's own edge): ~16,384 columns × ~1,700 rows ≈ 28 M cells →
  unusable as geometry.
- **64 m cubes** (one cube = one field column, one row thick): ~442 k cells → ~124 k cubes →
  ~744 k faces. Likely too many for a per-frame quad upload.
- **128 m cubes — recommended**: 64 × 64 = 4,096 columns × ~13 rows ≈ 53 k cells → ~15 k cubes
  → ~90 k faces. This is exactly the lattice `CloudField::sample` now uses, so the sample
  lattice and the cube lattice are one lattice, 4 chunks per cube.
- **256 m cubes**: ~7 k cells → ~2 k cubes → ~12 k faces. Cheap and very blocky.

Open questions for the user, to be asked with these numbers in front of them:

1. Cube size: 128 m (recommended) or 256 m?
2. Does a cube's vertical extent come from the model's `base_m`/`top_m` rounded to cube rows?
3. Does `FillLevel` drive the cube's **drawn size** (the fluidics idiom: the level is the drawn
   fraction of a cell) or only its shade?
4. Cubes as merged geometry per field, or instanced?
5. Do clouds share the world's material/pattern path (so they are lit and patterned like terrain)
   or are they flat-shaded?

## Landed this session (uncommitted; verified)

- `crates/voxel-client/src/display.wgsl` — **new**: the one display stage (`srgb_encode`,
  `srgb_decode`, `bayer8`, `dithered`, `tone_map`). `world::pass_source` concatenates it in
  front of each presenting pass, and the three copies were deleted from `shading.wgsl`,
  `sky.wgsl` and `clouds.wgsl` (132 / 87 / 59 lines).
  Witnesses: `cargo test -p voxel-client --lib world::` → 2 passed, including a **no-GPU WGSL
  parse** of every composed pass through wgpu's own naga front end.
- `crates/voxel-client/src/cloud_field.rs` — the model is asked on the 128 m lattice and the
  texels between samples are interpolated (coverage from the nearest sample; density and layer
  re-weighted over the wet corners; a class is never averaged).
  Measured, printed on every run: **213.26 ms → 53.16 ms** per fill; 4,225 asks against 16,384;
  coverage 4,604 model-cloudy texels against 4,649 in the field; worst |Δwater| 0.4657, worst
  layer boundary moved 712 m.
  Witnesses: `cargo test -p voxel-client --lib cloud_field` → 9 passed.
- At that time the full suite was `cargo test -p voxel-client --lib` → **391 passed, 0 failed,
  17 ignored** (the ignored ones need a GPU adapter).

## Known red, and open questions for the user

- `cargo test -p voxel-client --lib -- --ignored` → **13 passed, 4 failed** on the user's
  machine: `the_pattern_is_drawn_where_the_greedy_mesh_has_no_geometry` (4225 pixels — every
  pixel of the probe — "still carry a pattern past the ladder's ceiling"),
  `a_frame_of_the_world_passes_the_bars_band_count`,
  `the_derived_rig_lights_the_shipped_world_rather_than_only_a_probe`,
  `the_water_surface_reflects_the_sky_the_model_says_it_does`.
  - The pattern failure is a **premise problem and needs the user's ruling**:
    `pattern.rs::octave_weight` is silent only when `footprint ≥ cell/4`, and the top rung's
    cell is 32 m, so silence begins at an **8 m** footprint. The test's own comment calls the
    frame's **2.15 m** footprint "past the 32 m rung's own 8 m floor", which is not
    arithmetically true. Either the model gains a footprint ceiling or the test's premise is
    stale. **Do not choose alone, and never weaken an assertion to go green.**
  - The other three failures are unexplained; they may or may not share a cause.
- The **fill still runs on the frame thread** — 53 ms per refresh on the atmosphere's cadence is
  a visible hitch by itself.
- **Nothing has been verified in the game path.** `crates/voxel-client/examples/surface-format-check.rs`
  already builds the pipeline on `Bgra8UnormSrgb` and draws a cloud frame — but with an **empty
  scene**. Extending it to carry the game's meshed world (and write a PNG) is the first
  concrete step, and it is what the user means by *"your offscreen renderer is perfect … when
  it's actually displaying shit"*.

## The plan (the user approves before the first edit — vows 5–7)

0. **Reproduce in the game's own path**: extend `examples/surface-format-check.rs` to draw the
   game's scene at the swapchain format and write a PNG; run the game with a cleared field to
   test the crash hypothesis.
1. **Remove the crash**: the march's unbounded per-pixel work goes. The cube path makes the
   march unnecessary rather than merely cheaper.
2. **Clouds as the CA's cubes**: the cloud becomes `FillLevel` cells, and each cube is drawn
   with the change view's own face set (`boundary_faces()`), not a new cube system and not a
   raymarcher.
3. **Move the fill off the frame thread** (or lower the cadence) and re-measure in the running
   client, not only in a unit test.

## Commands

```text
cargo test -p voxel-client --lib
cargo test -p voxel-client --lib -- --ignored      # GPU; 13 pass, 4 fail, as above
cargo test -p voxel-stream                          # 32 passed when last run
cargo run --offline -p voxel-client --example surface-format-check
cargo run --offline --release -p shploingus         # the game itself
```

## Session prompt (paste this to start the new session)

> Read `docs/HANDOFF-2026-10-08-clouds.md` and `vows.md`. The clouds must be drawn as **cubes
> built from the existing fluidics cellular automaton**, and the wispy cloud pass
> (`crates/voxel-client/src/clouds.wgsl`) is crashing the game and must go. Follow the build
> protocol: gather all context, write one plan, ask me for approval before the first edit, then
> implement and verify it in the game's own path — not only offscreen. Do not touch the other
> uncommitted changes in this tree.
