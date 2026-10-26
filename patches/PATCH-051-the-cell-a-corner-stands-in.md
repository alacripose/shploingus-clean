# PATCH-051 — the cell a corner stands in

PATCH-050 left this named, in its own words:

> *"**The field still measures zero**, and that is a real defect this patch deliberately did not
> touch: `Scene` asks `CellField::at` at **subvoxel** cells (`world_cell` = `floor(p / 0.25 m)`),
> while `at` treats the cell as **metres** (`centre = cell + 0.5`) and `key_of` divides by
> `CHUNK_SIDE` as if it were voxels. Fixing the address law would make the field answer, which would
> change every measured corner — the opposite of this pass's identity requirement. It is the next
> pass's work, and it is now cheap to reach: the rebuild it needs is the one this pass stopped
> paying for nothing."*

This pass is that repair, and nothing else in the light's path moved: the sun, the solver, the
stencil, the step, the queue's bounds and the shading model are the ones PATCH-050 left standing.

# The defect, in one line

**There were two address laws in one path.** The scene converted a corner's world position with
`floor(p / SUBVOXEL_M)` — a quarter-metre grain — and the field read those integers as **voxels**,
one metre each, whose chunk is `cell / CHUNK_SIDE`. So a corner at 40 m named cell 160, whose chunk
is 5, while its own chunk is 1: a real cell of this world, four chunks away, with no tree under it
(and, for a corner near a chunk boundary, in a chunk the streamer holds and a different one than the
corner's). `WorldLight::march` answers `None` for a cell whose own chunk has no resident tree, and
`None` is the rig's fallback — which is why the frame drew, why nothing failed, and why the binary
could report **1 512 244 corners rigged and 0 cells marched** on every run.

A second, quieter half of the same fault: the scene asked the field for **every corner** of a refined
triangle at that corner's own position, while the flag that decides whether the fragment is lit by
the field at all is `flat`. Two triangles of one quad could therefore land on different branches of
the fragment's light test and split the quad down its diagonal. The rule is now stated once, at the
quad: **a quad is field-lit only when all four of its corners were measured**, and a refined
triangle inherits its parent's flag.

# What changed

| file | the change |
|---|---|
| `crates/voxel-model/src/coord.rs` | **`VoxelPos::of_point`** — the one address law, written down where the type is: `floor(p)` per axis, `None` for a non-finite axis, saturated rather than wrapped off the end of `i32`. Beside it `VoxelPos::centre_m` — the point a march leaves from, the cell's own centre — and `VoxelPos::chunk` — the cell's chunk, Euclidean, so `(-1,-1,-1)` is the last cell of chunk `(-1,-1,-1)`. Pinned by `a_cell_is_the_voxel_a_point_stands_in` |
| `crates/voxel-client/src/field.rs` | the map is **keyed by `VoxelPos`**, so a lookup is the address and not a hash of three integers the caller derived some other way; `at`, `wants`, `adopt`, `measure` all speak one type; `chunk_of` is `of_point(p).chunk()` with the sentinel chunk for a non-finite point. The queue's cap is 65 536 addresses and the doc says what that is in asks |
| `crates/voxel-client/src/scene.rs` | the scene's `measure` closure is `VoxelPos::of_point(world)` and **nothing else**; the per-quad flag rule above; a refined triangle's interior vertices are measured at their own positions (they are not corners, and the flag is the quad's) |
| `crates/shploingus/src/main.rs` | `pump_light` adopts what the worker measured with the beam it was marched under, answers the worker's tree questions from the streamer's own `Arc`s, and asks for the next batch nearest-the-eye first; the report line carries the five counts beside the rate |
| `crates/voxel-client/examples/light-shot.rs` (**new**) | **the picture**: the shipped world, window, eye and tick, meshed by `Streamer`, drawn twice by the same `WorldPipeline` — once from the rig, once from the field — with the corner count, the direct band's range and the pixel difference printed so the picture is not the only evidence |
| `crates/voxel-client/src/field.rs`, `scene.rs` (tests) | `a_cell_is_its_own_key_and_the_chunk_is_the_points_own`; `a_point_off_the_world_still_names_a_chunk`; `the_cell_the_scene_asks_is_the_voxel_the_corner_stands_in`; `one_unmeasured_corner_rigs_the_whole_quad_so_no_diagonal_splits_it` |
| `conformance/missing-docs.txt` | the pins of the crates this pass edited, re-taken by the file's own recipe |
| `docs/STATUS.md`, `docs/TODO.md` | a measured row, and item 38 closed with what closed it |

# The design in one paragraph

**A cell is a voxel, and its address is its key.** A corner's world position names exactly one
voxel; that voxel is the key the field stores its measurement under, the point its shadow ray
leaves from (the cell's own centre), and the chunk whose tree the march needs — one function, three
uses, no conversion between them. Because the key is the type rather than three loose integers,
the two laws cannot drift apart again without the compiler saying so, and the pin holds the
geometry against the law rather than the law against itself: the test walks a real 2×2×2 chunk of
the shipped world, takes every corner the scene emits, computes its own cell from the corner's
world position, and fails if the scene ever asked about a different one.

# The numbers

## The picture, with the field measured in full (`light-shot`, release, 512 × 288)

The static frame settles the field completely before it draws, so this is what the world looks like
when every cell has been measured — the destination, not the steady state of a running game.
Tick 12 000 (the first day's highest sun), **22.0° up**, eye `[0.5, 19.62, -95.5]`, 605 of 605
chunks resident, 66.3 MB of mesh.

| | |
|---|---|
| cells the frame's corners name | **82 749**, all measured |
| the marches | **0.6 s** for all of them on one thread, **0.01 ms a cell** |
| corners | **226 500 of 228 700** carry a measurement (99.0 %); the 2 200 without are corners whose own cell's chunk the march could not answer for |
| the measured direct band | **0.0000 … 1.3203**, mean **0.5330** |
| corners reading nothing at all | **135 058 (59.6 %)** — the canopy's own shadow at a 22° sun, over a 160 m march |
| the two frames | **108 444 of 147 456 pixels differ (73.5 %), mean \|Δ\| 47.2** of 255 over the ones that do |

`target/light-rig.png` and `target/light-measured.png` are the two frames side by side: same
geometry, same camera, same sky, one light each. The rig's frame is the world with no shadows at
all — that is what a rig is — and the measured frame is the same world under a forest at a low sun.

## The shipped binary, settled (dev profile, idle, 100 s)

| | |
|---|---|
| cells held | **82 207** (the static frame at its own eye names 82 749) |
| corners from the world's own light, at a build | **225 984 of 228 700 (98.8 %)** |
| corners still on the rig | **2 716 (1.2 %)** |
| cells adopted from the worker | **30 909 a second** at peak, 1 806 in flight, queue 0 once settled |
| hops past a chunk not resident | **0** on every line — the 160 m march stays inside the window the eye holds |
| frame | **8.4 ms mean, 117 fps**, `scene` **0.01 ms**, **0 stalls** over 25 ms, worst 272.2 ms at the boot frame (t9000) |
| mesh, scratch | 131.6 of 256.0 MB, **0.0 MB** scratch |
| world digest | **`7c66a632336cc0a3`** — unchanged, as it must be: this pass moves the light, not the geometry |

**The one number that reads badly, and what it means.** Every build reports `0 corner(s) answered
from the world's own light` and **225 984 a step behind the light** — every corner, every build.
That is not a mismatch left in the field; it is the two clocks this project already declared. The
field's step is **0.5°**, the solver's own quantisation (≈ 1.1 s of this world's clock, whose sun
moves 0.45° a second); the geometry's epoch is **two** steps (≈ 2.2 s), because a build is 228 000
corner asks and a rebuild per field-step would spend that twice as often for a shadow moved by a
fiftieth of its own length. So a build always lands *between* two steps, and the measurements it
draws are between one and two steps old — drawn, counted, and honest about their age. The
equilibrium the counts show (98.8 % measured, 1.2 % rigged, 0 current, all stale) is exactly what
those two clocks predict, at every build, forever.

# Identity, argued and pinned

- **The geometry does not move.** The scene's corners are the same metres, in the same order, with
  the same normals and materials; the only bytes that changed are the bands and the flag that was
  always going to change (`light_present` was 0 on 1.5 M corners and is now 1 on 98.8 % of them).
  The world digest is unchanged and `0 stale-but-drawn` on every line.
- **The address law is the pin.** `the_cell_the_scene_asks_is_the_voxel_the_corner_stands_in` builds
  real chunks of the shipped world, records every cell the scene asks about, and then recomputes
  each corner's cell from its own world position: a scene that asked at the quarter-metre grain
  fails on the first corner, which is the exact defect this pass repaired.
- **The quad's flag cannot split it.** `one_unmeasured_corner_rigs_the_whole_quad_so_no_diagonal_splits_it`
  holds the rule: one corner missing is the whole quad on the rig, so both triangles take the same
  branch of the fragment's test.
- **A cell and its own centre agree.** `a_cell_is_the_voxel_a_point_stands_in` holds `of_point` and
  `centre_m` against each other for a sweep of points, so the key and the point a ray leaves from
  cannot be two different cells; `a_point_off_the_world_still_names_a_chunk` holds the non-finite
  and past-the-end cases to the sentinel rather than a panic or a zero.

# What it does not do, named

- **The field's sky is still not the fragment's sky.** The shader keeps the rig's `ambient` and adds
  the field's `diffuse` beside it, because the two are still not commensurable — the field's stencil
  measures the sky away from the horizon and the model's sky there is single-scattering, thousands
  of times dimmer. That is a fact about the sky model, not about this address law, and the shader's
  comment says so where the term is added.
- **A corner is measured at its cell's centre, not at the corner.** A corner sits on the boundary
  of up to eight cells, and the face it belongs to opens onto exactly one of them; the march
  therefore starts inside a cell that may be matter (the voxel behind the face), and the solver's
  own rule — the first chunk's starting cell is excluded from its own beam — is what makes that
  answer the face's light rather than a black one. A marching law that leaves from the *air in
  front of the face* would be a second address for the same measurement, and this pass did not
  take that step.
- **Coverage is not instantaneous.** The worker marches about 30 000 cells a second in this profile,
  so a window that has just streamed in spends two or three seconds on the rig while the field fills
  — the 1.2 % above is what is left of that after the world settles. In `--release` the same march
  measured **0.01 ms a cell**, so this is a property of the dev profile this project measures in and
  not of the field.
- **A measure of zero is a real answer.** 59.6 % of the measured frame's corners read nothing in the
  direct band. That is the canopy, and it is not distinguishable in the numbers from a cell that is
  itself in matter; only the picture separates them, which is why the picture exists.

# Verified

- `cargo test -p voxel-client --lib`: **518 passed, 1 failed, 2 ignored** on the final tree. The one
  failure is `scene::tests::the_fades_cost_is_the_second_trees_records_and_nothing_else`, the
  wall-clock-ratio test PATCH-049 and PATCH-050 both already reported as intermittent on this
  machine; it passes alone in **0.03 s** and was not touched by this pass.
- `cargo test -p voxel-client --lib the_cell_the_scene_asks` and `… a_cell_is_the_voxel_a_point_stands_in`:
  one each, both green — the two pins this pass adds.
- `cargo build -p shploingus`: **exit 0**, so the build gate's census, conformance, prose frame and
  missing-docs pins hold on the final tree. `conformance-stale` is **0** and `conformance` is
  `active`; the four crates this pass edited (`voxel-model`, `voxel-work`, `voxel-client`,
  `shploingus`) were re-pinned by the recipe in `conformance/missing-docs.txt`'s header, with both
  commands run in the order it gives, and every count unmoved (**948 / 61 / 191 / 0**).
- `cargo run --release -p voxel-client --example light-shot`: the picture and the counts above;
  `target/light-rig.png` and `target/light-measured.png` written.
- The shipped binary, 100 s idle after the window settled (`SHPLOINGUS_FRAMES=1`): the report rows
  above — `605/605 chunks`, `0 stale-but-drawn`, `scratch 0.0 MB`, digest `7c66a632336cc0a3`,
  `0 stalls` over 25 ms in the settled window; stopped by `kill` on its own PID.
