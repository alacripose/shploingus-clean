# PATCH-053 — the upload that was paying for its own buffer

The mission this pass was given, 2026-10-22:

> *"Play the shipped binary the way the mission describes — settled idle, then loading while eight
> busy cores compete — and fix the stalls that actually appear rather than the ones predicted. The
> budget the last pass introduced holds idle (worst streaming frame 48.3 ms inside the 50 ms ceiling)
> but 287 loaded frames exceed it, worst 121.3 ms, and the largest unbudgeted piece is the cloud field
> regenerating and uploading 46 000 records every frame; bring that owner inside the same ceiling,
> without a second generation when nothing changed …"*

# What the playtest found, and what the premise got wrong

Instrumented at the call (`WorldPipeline::upload_split`, printed as `upload-split` every second), the
loaded frame's `draw` split like this:

| | measured |
|---|---|
| `write_quads` — the records, cloud included | **0.4–8.1 ms**, and `0.37–0.48 ms` once the cloud's trees stand still |
| `write_corners` — the corner slice | **0.00 ms** on every sampled frame (the scene was already uploaded) |

So the 46 000 cloud records are **not** regenerated on the frame and **not** uploaded every frame:
the cloud worker derives a tree once per cadence, `Scene::with_cloud_fade` appends the two trees to
the record array (a copy), and `write_quads` compares the values against the last upload and skips
the serialisation and the copy when they are equal (`memcmp` over 1.38 MB, tenths of a millisecond).
The record upload's 8 ms is paid on the frames a **new tree arrives**, four times a second — a
cadence cost, not a per-frame one, and inside the ceiling on its own.

What was actually costing 45–70 ms of `draw` was the pass's own upload: **the back pair was sized to
each slice's end.** A 20 MB scene sent 1.5 MB at a time grew its buffers six or seven times, and a
grown buffer is a new allocation — so every byte already written into it was lost, and every growth
paid a fresh 16 MB GPU allocation under eight busy cores. The picture was a torn scene assembled from
the last slice onward, and the stall was the reallocations.

# What changed

| file | the change |
|---|---|
| `crates/voxel-client/src/work.rs` | `CornerUpload::capacity(stride)` — the buffers an upload needs, from the scene's own totals. Doc states the defect it names. `the_capacity_is_the_whole_scene_and_not_a_slice` pins it |
| `crates/voxel-client/src/world.rs` | the back pair is sized **once** from `capacity` before the first slice lands, never from a slice's end; `upload_split()` exposes the two uploads' cost to the report |
| `crates/shploingus/src/main.rs` | the `upload-split` report line, so the split is read rather than inferred from `draw` |

# The numbers

Same machine, dev profile, `SHPLOINGUS_STALL_MS=20`, 40 s runs.

| | before (PATCH-052) | after |
|---|---|---|
| **loaded** frames over 50 ms, 40 s | **287** (50 s window) | **1 — the boot frame (82.2 ms)** |
| loaded worst streaming frame | 121.3 ms (`draw` 70.8) | **≤ 50 ms**; the next-worst streaming frame in the window is under the line |
| loaded settled mean | 7.7 ms | **7.7–11.4 ms** across the run's seconds (the 11.4 is a second that was still streaming) |
| **idle** frames over 50 ms, 40 s | 1 (the boot frame) | **5** — the boot frame (335.2, `shell` 311.3) and four frames at t9146–t9151 (59.0–77.2) |
| idle worst streaming frame | 48.3 ms | **77.2 ms** (`draw` 43.3, `shell` 12.2, `judge` 9.1, `stream` 8.2) |
| idle settled mean | 8.3–8.6 ms | **8.5 ms** |
| world | 605/605, `7c66a632336cc0a3` | **605/605, `7c66a632336cc0a3`** |
| light | every corner measured | **80 311 cells held, 0 queued**, same corner counts |
| memory | 132.0 of 256.0 MB mesh | **131.6 of 256.0 MB**, scratch 0.0 MB, 0 stale-but-drawn |

**The idle line is a real trade and is stated rather than hidden.** The first upload of the big scene
now pays **one** 16 MB allocation instead of seven growths, so that frame is larger (77.2 ms instead of
48.3) while every frame around it is smaller. The ceiling is met by the loaded distribution (one frame
over it, the boot frame) and missed by the idle one on five frames of the first two seconds.

# Verified

- `cargo test -p voxel-client --lib --offline work::tests`: **4 passed**, including
  `the_capacity_is_the_whole_scene_and_not_a_slice` — a capacity taken from a slice grows mid-upload,
  reallocates and loses the prefix, which is the defect this pass repaired.
- The gated build (`cargo build -p shploingus`) is green with the census, conformance, prose frame and
  missing-docs pins holding.
- Two runs: `target/run-split-load.txt` (loaded, with the split) and `target/run-fix-idle.txt` (idle);
  the run they are compared against is `target/run-work-load.txt` from PATCH-052.

# What it does not do

- **The five idle frames over the ceiling are the first scene's allocation and the first two cloud
  trees arriving on the same frames as the shell's own first work**, not a periodic stall: there is no
  frame over the line after t9151 in the idle run, and the loaded run's only one is boot.
- **The corner pair is allocated per upload, not pooled.** A third buffer reused across uploads would
  remove even the one allocation; it costs 16 MB of GPU memory and is not in this pass's memory
  budget.
- **`draw` still carries the pass and the pipeline state**; the two uploads inside it are now named
  and measured, and the rest is one draw call whose cost under eight busy cores is the GPU driver's.

## Recipes

```recipe
segment: world
axis: evidence_grade
ticket: T-0954
site: crates/voxel-client/src/work.rs:152
what: keep the back pair sized once from `CornerUpload::capacity` before the first slice lands, never from a slice's end
because: a 20 MB scene sent 1.5 MB at a time grew its buffers six or seven times and every growth lost the bytes already written; sizing once leaves 1 loaded frame over 50 ms in 40 s against 287
check: cargo test -p voxel-client --lib work::tests -> failures 0
witness: crates/voxel-client/src/work.rs
expects: 0
requires: cargo check --workspace
```
