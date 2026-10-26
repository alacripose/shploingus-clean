# PATCH-047 — the geometry gets one owner, and the frame that walks sends nothing

The mission this pass was given, 2026-10-22:

> *"Give the resident geometry one owner: buffers that live across frames and are rewritten only
> when chunks, light or camera-relevant data actually change, with the per-frame path doing no
> allocation and no full-scene upload … Reuse the audit's measurement harness so the result is
> argued, not asserted: stage timings and fps from the shipped binary idle and under load,
> working-set memory, before and after."*

The audit's ranked list put **world draw + per-frame buffer creation at 37–50 ms of a 72 ms frame
(~51 %)** and **scene assembly at 17–25 ms (~23 %)**, both untouched. This patch takes the second
one and half of the first: the creation.

# What changed

| file | the change |
|---|---|
| `crates/voxel-client/src/scene.rs` | corners are **chunk-local** (`Vertex::position` is from the chunk's low corner, `Vertex::chunk` indexes `origins`), `Scene::build` stamps a **build id**, `Scene::begin_frame` strips the frame's cloud/body suffix, `SceneKey` states `world · residency · light`, and `Scene::probe` takes the eye its corners are already relative to |
| `crates/voxel-client/src/world.rs` | **`write_corners`**: the vertex and index buffers live on the pipeline and are re-written only when `Scene::build` moves, doubling when they must grow; **`CornerTraffic`** is the ledger of what was sent and what was spared |
| `crates/voxel-client/src/shading.wgsl` | `vs_main` places a corner with `in.position + origins[in.chunk].xyz` — the same eye-relative origin the record format already reads |
| `crates/shploingus/src/main.rs` | the frame builds through **`SceneCache`** keyed on `(world_generation, residency_generation, rig tick)`, appends the cloud and bodies, draws, and puts the terrain back; a `geometry` line on the second boundary reports builds, uploads, bytes and spared bytes |
| `crates/voxel-work/src/stream.rs` | **`residency_generation`**: the drawn world's own count, bumped when a mesh enters, leaves, is demoted or is rebuilt — `world_generation` is the *record's* and moves only on an edit |
| `docs/DESIGN-SPEC.md` §5.2 | the four owners of the frame's geometry, and the direction data flows between them |

**Nothing about what is drawn changed.** The same corners, the same rectangles, the same camera:
`renderer::tests::the_quad_format_and_the_vertex_format_draw_the_same_surface` still draws one
world through both vertex stages and compares coverage exactly.

# Why the corners had to move into the chunk's frame

A per-frame upload could not be removed while the *bytes* depended on the camera. `Vertex::position`
was `world − eye` in `f32`, so every step invalidated all 15 MB of it, and the buffer had to be
built again. Chunk-local corners are the same bytes while the world and its light are — and the
shader already had the array that could place them, because `vs_quad` reads
`origins[record.chunk]` for exactly this reason. `origins` is `origin − eye`, computed in `f64` once
per chunk at upload, so no precision is given up: `f32` now carries a position *inside one chunk*
(≤ 45 m) instead of a coordinate two million metres out.

**The cost is four bytes a corner.** `Vertex::STRIDE` moved 64 → 68 and the renderer test that pins
it says why. The vertex format still costs 296 B a quad against the record format's 32.

# The measurement

The harness is the one the audit used: `SHPLOINGUS_FRAMES=1` prints the mean stage split once a
second, and `geometry` (this patch) prints what the geometry path did in the same second. Run on the
dev profile (`target/debug/shploingus.exe`), because the release build still stops at the rustc
ICE recorded in `docs/TODO.md` — **every number below is dev-profile**, and the draw stage in this
environment is software-bound, so it is quoted only as a split and not as a frame rate.

**The CPU half, measured by `renderer::tests::what_six_hundred_chunks_cost_the_cpu_every_frame`:**

| | quad format | vertex format (what the light-field frame draws) |
|---|---|---|
| settled window | 605 chunks, 57 175 quads | same |
| assembled | 1.75 MB, **0.89 ms** (5.3 % of 60 Hz) | 16.15 MB, **13.85 ms** (83.1 %) |
| four times over | 7.02 MB, 4.68 ms | 64.60 MB, 57.94 ms |
| per quad | 32 B | 296 B |

**The frame half, from the shipped binary** (`SHPLOINGUS_FRAMES=1`, dev profile, a 150 s run that
settled at 1 083/605 chunks resident, in flight 0, 85 321 quads, 44–46 k cloud rectangles, the same
digest for every second quoted):

| second | frames | scenes built | corner uploads | frames that sent nothing | bytes not sent |
|---|---|---|---|---|---|
| 1 | 36 | 4 | 4 (96.34 MB) | **32** | **770.72 MB** |
| 2 | 34 | 5 | 5 (120.43 MB) | **29** | 698.47 MB |
| 3 | 33 | 4 | 4 (96.34 MB) | **29** | 698.47 MB |
| 4 | 35 | 4 | 4 (96.34 MB) | **31** | 746.64 MB |
| 5 | 35 | 5 | 5 (120.43 MB) | **30** | 722.55 MB |
| 6 | 34 | 4 | 4 (96.34 MB) | **30** | 722.55 MB |

The mean split in the same seconds is **28.6–31.9 ms a frame** (31–35 fps, dev profile):
`shell 9.7 | draw 9.1 | scene 5.1 | judge 1.9 | sky-light 1.6 | camera 0.9`. **The `scene` stage is
5.1 ms because four frames of thirty-five pay it** — about 45 ms a build at this window's 85 321
quads (the microbenchmark above measures the geometry alone at 13.85 ms; the rest is the light
field's own per-corner ask and the cloud suffix) — where before this patch every one of the 35
frames paid it and copied 16.9 MB.

In the earlier, smaller settled window the same harness reported the extremes directly: a kept
frame at `scene 0.00 ms` and `0.00 MB`, a built frame at `scene 1304 ms` and 18.62 MB. So the two
claims are numbers rather than adjectives: **~86 % of settled frames build no scene and send no
bytes**, and the 16.9 MB that used to cross the bus every frame now crosses it when a chunk arrives.

# What it does *not* do, named

- **The draw itself is untouched.** The 37–50 ms the audit measured in the draw is fill, shader and
  (since the MSAA pass) resolve; none of it is CPU geometry. This patch removes the *creation* half
  of that row, not the fill.
- **Corner light still travels per corner**, 24 bytes of the 68. Making the world draw entirely
  through the record format with a measured light per rectangle is the next structural step and it
  is a change to `QuadRecord`'s layout, the merge's wire format and every test that pins it — not a
  thing to land in the same pass as this one.
- **A frame that is still streaming pays the build**, by construction: the key includes residency,
  so a chunk arriving rebuilds. That is the correct trade (a stale scene would draw a window the
  streamer had filled or emptied), and it is why the settled run above is the one quoted.
- **The light field measured nothing in any run on this machine, and that is not this patch.**
  Every `light-field` line in `logs/` and this pass's runs — 375 of them — reads `0 cell(s)
  marched`, with `1.3–1.7 M corner(s) rigged` per second: the field's first hop finds no tree for
  the cell's own chunk, so every corner falls back to the rig while the frame still pays the vertex
  format's 296 B a quad for measurements it does not have. Making the *format* follow the field's
  coverage — build with the record format while nothing is measured — is a policy change with a
  visible switch and it is named here rather than slipped in. The defect behind the zero is its own
  pass.

# Verified

- `cargo test -p voxel-client --lib`: **504 passed, 0 failed, 2 ignored** (includes every GPU test,
  the two-format parity test, the pattern/address tests and the new
  `a_scene_cache_builds_only_when_its_key_moves`).
- `cargo test -p voxel-work --lib`: **68 passed, 0 failed, 2 ignored**, including the new
  `the_drawn_generation_moves_when_the_drawn_world_does`.
- `cargo test -p voxel-mcp --test axis_route`: **10 passed** after re-pinning
  `conformance/missing-docs.txt` for the three crates this patch edited (`shploingus`,
  `voxel-client`, `voxel-work`; the counts did not move, 0 / 191 / 61).
- `cargo check -p voxel-client --examples`: every example compiles.
- The shipped binary runs with no wgpu validation error and the picture is unchanged. A second,
  shorter run after the last edit settled at 1 019 chunks and 31 fps with **27–28 of 31–33 frames a
  second sending nothing — **619–642 MB a second not sent**.
- `cargo check --workspace`: exit **0**.

**One thing the full-suite number does not cover:** the first attempt at it died with
`NoDevice { detail: "Not enough memory left." }` on five GPU tests because the machine was running
this pass's other builds at the same time; the tests that failed there were re-run one at a time and
all passed, and the adopted number is from a full run with the machine otherwise idle. That is an
environment, not a code path, and it is recorded so the number is not read as a clean room.

# The hunt: a world mutated under a running cache

The key is the whole contract — a missed bump draws a world that no longer exists, and a rebuild on
every frame is the cost this patch set out to remove — so it is driven rather than argued.

**`scene::tests::the_cached_scene_is_the_resident_world_on_every_frame`** stands a real
`Streamer` (world 2468, a 3 × 1 × 3 window) under a real `SceneCache` for twenty-four frames and
mutates it the way a player does: `note_world_changed` at frame 2 (which empties the drawn set while
the *record* generation does not move), an arrival on each of frames 3, 5, 7, 8, 9 and 11, a dig at
10, **a one-chunk step east at 6** (the west column evicted, a new one asked for — the window
shrinking before it grows), and the atmosphere's own ticks at 12, 15, 18 and 21. Every frame it
asserts, after the pump, that the scene's quads are the resident set's own
(`scene.quads == Σ mesh.quads`), that a corner exists per corner of that set, that every corner's
light is **this frame's** revision rather than the one the scene was kept under, and that the next
`take` strips the cloud and body suffix the frame appended without moving the terrain or breaking
the index range.

**The test fails when the key is wrong, and that was checked by making the key wrong.** With
`residency` dropped from the frame's `SceneKey` the suite stops at frame 2 with
`the scene drew 4144 quads for a residency of 0` — the exact failure the key exists for, and not a
number the assertion could pass by accident.

**The hunt found one defect, and it was in the seal rather than the key.** `Scene::build` seals
`scene.terrain` on its last line, but the two index-space guards (`checked_add(3)` / `checked_add(4)`)
left the function with `return scene` **above** that line. A scene that stopped there would state
`terrain: [0, 0, 0, 0]`, and `begin_frame` — which trusts the seal — would then strip every corner
the build had just made and draw an empty frame: a partial scene turned into no scene. The guards
now `break 'quads` into the same tail, so every exit seals, and
`every_built_scene_states_the_terrain_a_cache_truncates_to` pins the invariant over the four
constructors rather than waiting for four billion indices to arrive.

**What is not exercised live, stated rather than implied.** The shipped binary has no scripted-input
hook (`SHPLOINGUS_WORKERS`, `SHPLOINGUS_FRAMES`, `SHPLOINGUS_STALL_MS` are the only three
environment variables it reads), so a dig or a step cannot be driven into it from a test. The
mutation half runs in-process against the same `Streamer`, `SceneCache` and `Scene` the binary
calls; the binary contributes the frame rate, the stage split, the working set and the upload
ledger. **Buffer growth under a shrink** is pinned as policy
(`world::tests::a_held_buffer_grows_by_doubling_and_never_shrinks` — a halved window reuses the
bigger buffer, one byte over doubles) and the draw reads its counts from the scene rather than the
buffer (`draw_indexed(0..scene.indices.len())`), so a shrunk buffer cannot over-draw; there is no
device-level shrink-and-redraw test, and this line is where that is admitted.

# The measurement, against the audit's baseline

The audit's condition was the 8-hog load: eight busy cores beside the game, **13 fps / 154 ms**.
The same 16-core machine, dev profile, the window settled at 605/605 chunks and 57 175 quads
(in flight 0, digest constant across every second quoted):

| condition | fps | mean frame | shell | draw | scene | judge | sky-light | working set |
|---|---|---|---|---|---|---|---|---|
| idle | **46–51** | **19.9–21.6 ms** | 7.9–8.4 | 5.9–6.7 | 1.9–2.2 | 1.8–2.0 | 1.3–1.5 | **374 MB** |
| 8 hogs | **31–33** | **29.6–31.6 ms** | 10.9–12.0 | 9.2–9.8 | 3.1–3.5 | 2.4–2.6 | 2.2–2.5 | **374 MB** |
| audit baseline | 13 | 154 ms | — | — | — | — | — | not recorded |

So under the load the mission names, the frame is **about 5× shorter than the baseline and the rate
2.4× higher**, and the working set does not move between idle and loaded: the scene cache holds one
scene, not two.

**The upload ledger, in the same runs.** Idle: 5 builds and 5 uploads (88.43 MB) a second, with
**41–45 of 46–50 frames sending nothing** and 725–796 MB not sent. Under load: 5 builds and 5
uploads (80.70 MB), **29–31 of 34–36 frames sending nothing**, 468–500 MB not sent. The five are not
streaming: they are the **atmosphere cadence** — the key's `light` is the rig's tick, which moves
every fourth tick whether or not the derived light moved, and a light tick invalidates the scene by
design because the field's cells were re-marched at it. At 57 175 quads that is ~16 MB and ~23 ms
five times a second (11 % of the loaded second's CPU). It is the last invalidation left on a
settled world, and it is named here rather than tuned in a pass that is about ownership.

**Which stage is dominant under load: `shell` (10.9–12.0 ms), then `draw` (9.2–9.8 ms).** The
geometry work this patch took — `scene` — is 3.1–3.5 ms means because five frames in thirty-five
pay it. The shell being first is a finding about the shell, not about the geometry, and it is the
next pass's question rather than this one's.

**And the light field still measures nothing on this machine**: every `light-field` line in these
runs reads `0 cell(s) marched` against 1 247 940 corners rigged, so the vertex format's 296 B a quad
and the five builds a second buy no measurement at all. That defect is already named below; the
cadence above is its cost.

# Verified, final tree

- `cargo test -p voxel-client --lib`: **508 passed, 0 failed, 2 ignored** (the run that matters for
  this pass's final bytes; `scene::tests::` alone, 16/16, in 1.3 s).
- `cargo test -p voxel-work --lib`: **68 passed, 0 failed, 2 ignored**.
- `cargo test -p voxel-mcp --test axis_route`: **10 passed** after re-pinning `voxel-client`'s
  missing-docs pin to the new bytes; the count re-read from the compiler at **191**, unmoved.
- `cargo check --workspace`: exit **0**.
- The shipped binary under `SHPLOINGUS_FRAMES=1`, idle and with eight busy cores: no wgpu validation
  error, 605/605 chunks, `0 stale-but-drawn`, one digest for every second quoted.

**One pre-existing test is not deterministic under the full suite, and it is not this patch's.**
`the_fades_cost_is_the_second_trees_records_and_nothing_else` asserts a wall-clock ratio between two
arms (a two-tree fade must cost no more than 2× a one-tree append); two of five full-suite runs
failed it while it passes **3/3 in isolation** and **3/3** with the whole `scene::tests::` group at
16 threads, and the passing in-suite run measured 1.20× against the 2.0× bound. The contention comes
from the GPU tests running beside it, not from the code under test: the ratio's two minima are taken
in different rounds and one of them gets a quiet window. It is recorded rather than re-tuned, because
lowering the bound to make a loaded suite green is the failure mode this project's own PATCH-038 is
about.
