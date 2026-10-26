# PATCH-052 — the frame's own work, and the one place that decides it

The mission this pass was given, 2026-10-22:

> *"The frame has no owner of its budget: while chunks stream under eight busy cores the audit
> measured a mean of 18 ms and a worst frame of 437 ms (draw 251 | scene 98 | shell 43), against
> 8.2 ms settled, because scene builds, corner uploads and light asks each happen whenever they fall
> due. Reconstruct the frame's work around that single responsibility — one place decides how much
> geometry, upload and measurement work a frame may do, carries the remainder, and cannot be starved
> by a burst … Done means: the shipped binary, run idle and under eight busy cores through a full
> load, reports a stated frame ceiling with the worst streaming frame inside it (quote the
> distribution, not the mean alone), still settles at 605/605 chunks with the same digest and every
> corner still measured, no regression in settled frame time or in the ~312 MB working set, and a
> test that fails if a burst can again blow past the ceiling."*

# What the trace found

The three classes were **all-or-nothing**, and each was paid by whichever frame happened to be
drawing when the work fell due:

| work | how it was bounded | what a burst cost |
|---|---|---|
| geometry | not at all: `residency_generation` moved on every chunk, and `SceneCache::take` walked all 605 meshes on that frame | **5–6 ms** idle, **69–71 ms** under eight hogs |
| corners | not at all: `write_corners` sent the whole scene when the build id moved | **10–13 ms** idle, **143–149 ms** loaded (inside `draw`) |
| measurement | already bounded: 2 048 adopts and one ask a frame | 2 ms, and it stayed 2 ms |

That is why the loaded frame was 240 ms in this pass's own baseline run (45 s, eight busy cores,
`SHPLOINGUS_STALL_MS=20`): **mean 240.5 ms, scene 69–71, draw 143–149, 5 fps, worst 264 ms**, with
317 frames over the 20 ms line.

# What changed

| file | the change |
|---|---|
| `crates/voxel-client/src/work.rs` (**new**) | **`FrameWork`** — the one owner: a stated ceiling (**50 ms**) and three slices, each independent of the others, with the measured cost of each slice written beside it and a test that holds their sum under the ceiling. **`CornerUpload`** — the upload's cursor and the `UploadSlice` it hands a frame; the remainder is the cursor, and a slice always moves at least one item so a starved class cannot spin |
| `crates/voxel-client/src/scene.rs` | **`SceneBuild`** — the scene walk with a cursor. `step(meshes, light, displacement, refine, quads)` emits at most its allowance and returns whether the scene is complete; the cursor is a **chunk position**, not an index, because the streamer's mesh list gains members while a build runs (`HashMap` order is not a stable address). `Scene::build` is now that builder stepped with `usize::MAX`, so the sliced and unsliced paths are **one implementation** |
| `crates/voxel-client/src/scene.rs` | `SceneCache` keeps a scene for a key and **adopts** completed builds (`key`, `current`, `adopt`); `take` stays as the synchronous path for tests, examples and offscreen renders, and its doc says the frame does not use it |
| `crates/voxel-client/src/world.rs` | the corner buffers are **two pairs** — the front one drawn, the back one written a slice a frame — and the draw range is the **uploaded** index count, so a half-sent scene is never drawn. `set_upload_budget`, `upload_pending`; `Scene::vertex_bytes_of` / `index_bytes_of` serialise a slice instead of the whole scene |
| `crates/shploingus/src/main.rs` | one owner per frame: the build is stepped with `work.geometry()`, the draw is handed `work.upload()`, the measurer `work.measure()`. A build is **never restarted** for a key that moved mid-walk (that is a starved class wearing a budget's clothes): it finishes, is adopted under the frame's key, and the next build covers what arrived. A `frame-work` report line states the ceiling, the slices, a build in flight and the bytes owed |

# The design in one paragraph

**Three rates, one statement.** Every frame, the same three allowances: 1 536 quads of geometry
(≈2.0 ms), 1 500 000 bytes of corner upload (≈11 ms loaded, ≈0.9 ms idle) and 2 048 cells of
measurement (≈2 ms). The remainder is not remembered by `FrameWork` — it lives where the work
lives: the build's chunk cursor, the upload's byte cursor, the field's queue. Nothing is derived
from what the frame has left, so no class can starve another; nothing is abandoned when a burst is
due, so the work resumes; and because no site may do more than its slice, the frame's own cost is
bounded by their sum however much is due. The two properties that make it safe are pins rather
than comments: a scene stepped in 64-quad slices is **byte-identical** to one stepped once
(`a_scene_built_in_slices_is_the_scene_built_in_one_step`), and the three slices' measured cost is
under the ceiling (`the_slices_cost_less_than_the_ceiling`).

# The numbers

## Idle, 45 s, `SHPLOINGUS_FRAMES=1 SHPLOINGUS_STALL_MS=20`

| | before | after |
|---|---|---|
| settled mean frame | 8.4 ms | **8.3–8.6 ms** (no regression) |
| streaming frame, `scene` | 5–6 ms | **≤ 1.8 ms** |
| streaming frame, `draw` | 10–13 ms | 11–13.6 ms (the corner slice, now sent a slice a frame) |
| frames over 20 ms | 487 | **385** |
| frames over 33 ms | — | **26** |
| frames over 50 ms | — | **1 — the boot frame** (t9000) |
| **worst streaming frame** | 28.1 ms (over 20 in the settled window: 0) | **48.3 ms**, and **0 frames over the stated 50 ms ceiling** |
| residency, digest | 605/605, `7c66a632336cc0a3` | **605/605, `7c66a632336cc0a3`** |
| corners measured | 225 984 of 228 700 | **225 984 of 228 700** (81 168 cells held) |

The single frame over the ceiling is **t9000, the boot frame: 397.6 ms, of which `shell` is 362.19**
— the shell's first prepare, which this pass does not own and did not touch (the same frame measured
81.4 ms with `shell` 57.9 in the baseline run of the same tree, so its cost is as variable as it is
outside this budget). Every frame after it is a streaming or settled frame, and the streaming
worst is 48.3.

## Under eight busy cores, 50 s

| | before | after |
|---|---|---|
| settled mean frame | 8.2 ms (PATCH-050) | **7.7 ms** |
| settled window | 0 stalls | **0 stalls**, worst 151.7 ms at the boot frame |
| streaming `scene` | 69–71 ms | **≤ 8.5 ms** (a 1 536-quad slice is 2 ms idle and 8.5 ms when eight cores are busy) |
| streaming `draw` | 143–149 ms | **45–70 ms** |
| streaming mean | 240.5 ms | **not a meaningful number in this run** — the window was still streaming when the run ended |
| frames over 20 / 25 / 33 / 50 ms | 317 over 20 | 719 / 680 / 569 / **287** |
| worst streaming frame | 264 ms | **121.3 ms** (`shell` 17.9, `judge` 8.3, `camera` 8.8, `draw` 70.8) |
| residency, digest | 605/605, `7c66a632…` | **605/605, `7c66a632336cc0a3`**, 132.0 of 256 MB mesh, 0 stale-but-drawn |

**The loaded case does not meet the ceiling, and this is the pass's named gap.** 287 frames exceeded
50 ms in the loaded window. Their split says what the ceiling is made of: `shell` 17–23 ms and
`camera` 8–22 ms (both own stages this pass was told to leave intact, both inflated by CPU
starvation), and `draw` 45–70 ms — of which the corner slice is ~11 ms and the rest is **the
cloud's own record upload**: 46 000 rectangles, 1.1 MB, serialised and sent *every frame* by the
cloud field's design, which is a different owner and a different budget from this one. Slicing it
would mean a cloud that draws a prefix of its own rectangles while the tail arrives, i.e. a change
to the cloud field's contract rather than to this budget; it is the next pass's work and it is
named here rather than hidden in a mean.

# Verified

- `cargo test -p voxel-client --lib`: **522 passed, 1 failed, 2 ignored** on the final tree. The one
  failure is `scene::tests::the_fades_cost_is_the_second_trees_records_and_nothing_else`, the
  wall-clock-ratio test PATCH-049, PATCH-050 and PATCH-051 all already reported as intermittent on
  this machine; it passes alone in **0.03 s** and this pass did not touch it.
- Two defects the suite caught in this pass's own work, both fixed rather than papered over: a
  record-format frame was drawing the previous vertex scene beside its rectangles (the draw now
  requires the scene to carry corners *and* the front pair to hold an upload), and a build over an
  unsorted mesh list underflowed its cursor ordinal (a one-step build may be handed any order; the
  sliced caller sorts, and the ordinal saturates).
- `a_scene_built_in_slices_is_the_scene_built_in_one_step`: a real 2×2×2 chunk of the shipped world
  built in 64-quad slices is **byte-identical** to the one-step build in corners, indices,
  rectangles, origins and terrain counts, no step exceeds its allowance, and the number of steps is
  `quads.div_ceil(slice)`.
- `work::tests`: the three slices' measured cost is under the ceiling; a burst of 57 000 quads and a
  20 MB scene is admitted a slice a frame, never more, and finishes; a slice smaller than one
  corner still moves one item.
- The shipped binary, the numbers above: 605/605 chunks, digest `7c66a632336cc0a3`, **every corner
  still measured** (225 984 of 228 700; 81 168 cells held), `0 stale-but-drawn`, `scratch 0.0 MB`,
  mesh 132.0 of 256.0 MB (the double-buffered corner pairs are GPU buffers, not working set).
- `cargo build -p shploingus`: exit 0 with the build gate's census, conformance, prose frame and
  missing-docs pins holding; `conformance-stale` **0**. The pins of the crates this pass edited were
  re-taken with both commands in the order `conformance/missing-docs.txt` gives.
- The two runs are `target/run-work-idle.txt` and `target/run-work-load.txt`; the baseline runs they
  are compared against are `target/run-base-idle.txt` and `target/run-base-load.txt`.
