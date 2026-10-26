# PATCH-054 — the pair that is sized before anyone asks

The mission this pass was given, 2026-10-22:

> *"Play the same two conditions again — idle, then loading under eight busy cores — and remove the
> last stall the trace can name: the first scene's corner pair allocating 16 MB inside a live frame
> (77.2 ms, `draw` 43.3), which is the one allocation the earlier fix traded seven growths for. Size
> or pool the pair before the first frame … so no settled-window frame exceeds the stated ceiling in
> either condition, and state what the retained bytes cost in the working-set ledger rather than
> leaving them uncounted. Done means: both runs quote their full distributions against the ceiling,
> the world still settles at 605/605 with digest `7c66a632336cc0a3`, memory is stated with the new
> retained allocation included, and the pin that catches an allocation arriving mid-frame exists and
> fails if it returns."*

# What changed

| file | the change |
|---|---|
| `crates/voxel-client/src/world.rs` | `CORNER_VERTEX_BYTES` / `CORNER_INDEX_BYTES` (32 MiB + 4 MiB) state the capacity **both** pairs are built with, at construction, before the first frame. `CornerPair::ensure_*` now report whether they allocated; the pipeline counts them in `corner_allocations` and states the retained bytes in `corner_retained_bytes`. `a_scene_arriving_allocates_no_corner_buffer` is the pin |
| `crates/shploingus/src/main.rs` | the `upload-split` line states the retained bytes and the allocation count beside the uploads they serve, so 72 MB of GPU memory that neither the mesh residency nor the scratch counts is in the ledger rather than outside it |

**The capacity, and why it is that number.** The shipped window is 605 chunks; its surface at the
spawn eye is **228 700 corners** — 15.6 MB of vertices at the 68-byte stride and 1.4 MB of indices at
six a quad. The capacity is the growth rule's own next power of two above **twice** that surface, so a
window the eye looks down on still fits: 32 MiB of vertices (493 447 corners), 4 MiB of indices
(1 048 576). A pair is 36 MiB; the two pairs are **72 MiB**, retained for the process. That is the
price of removing the last allocation from a live frame, and it is printed on every report interval as
`corner pairs 72.0 MB retained, 4 allocation(s) since start`.

# The numbers

Same machine, dev profile, 16 logical cores, `SHPLOINGUS_STALL_MS=50` (the frame-work ceiling), 8
`yes` processes for the loaded runs. Before = the PATCH-053 build; after = this one.

| | before (PATCH-053) | after |
|---|---|---|
| idle, frames over 50 ms in 40–45 s | 5 (boot 335.2 + four at t9146–t9151, `draw` to 43.3) | **1 — the boot frame only**: 79.9 ms once (`shell` 58.8, `draw` 2.2) and 347.0 ms once (`shell` 312.7, `draw` 13.6) |
| idle, settled window | 77.2 ms worst | **0 frames over 50 ms** for the last 38 s of a 45 s run, mean **8.5–8.6 ms** of 115–117 frames an interval (`shell` 5.2, `draw` 1.1–1.2) |
| loaded, frames over 50 ms | 1 (boot 82.2) at 40 s | 40 s: **1** (boot 83.8). 70–85 s: **119–301**, every one of them in the stream-in phase, the last at t9760–t9824 |
| loaded, settled window (605/605, 0 in flight) | — | **0 frames over 50 ms** over the last 30–44 s of each run; settled mean **8.4–8.5 ms** of 116–118 frames an interval (`shell` 4.5, `draw` 1.3–1.4) |
| loaded worst non-boot stall | 121.3 ms (`draw` 70.8), before PATCH-053 | **69.4–103.8 ms** (t9619: `draw` 57.3, `shell` 16.7, `judge` 10.2, `scene` 9.4, `camera` 7.8); mean split `draw` 30.3 (`records` 14–40 ms, `corners` 0–19.4 ms of a 1.5 MB slice) \| `shell` 11.1–14.3 \| `judge` 5.2 \| `scene` 4.4 \| `camera` 3.9 |
| corner allocations, whole run | one 16–32 MB growth inside a live frame | **4 at construction, 0 after** — 72.0 MB retained |
| world | 605/605, `7c66a632336cc0a3` | **605/605, `7c66a632336cc0a3`**, 57 175 quads, 0 stale-but-drawn |
| light | every corner measured | **80 419 cells held, 0 queued, 0 in flight**, 0 corners rigged |
| memory | 131.6 of 256.0 MB mesh, scratch 0.0 | **130.2–131.7 of 256.0 MB mesh**, scratch 0.0 MB settled, **+72.0 MB corner pairs (GPU, stated separately)** |

**The distribution's own honesty.** The loaded runs are not one distribution: the same binary, same
eight busy cores, gave **1** frame over the ceiling in one 40 s run and **301** in a 70 s run. Their
worst non-boot frames: 69.4, 103.8 and 72.3 ms. The
difference is the machine's other load during the stream-in phase, and it is not the corner buffers:
every run reports `4 allocation(s) since start`, and `corners` never exceeds **19.4 ms** — a bounded
1.5 MB slice, not a whole scene. What the trace names now, when a streaming frame goes over, is
`records`: `write_quads` re-serialising the scene's own record array when a chunk arrives (up to
**39.9 ms** for 57 175 records under contention), with the shell's own work beside it. That is the
next stall to fix, and it is named here rather than folded into this pass.

The boot frame is **not** in this ceiling: 79.9–347.0 ms, and 58.8–312.7 ms of it is `shell` — the
shell's first prepare, exactly the separate concern the mission says to keep named. `draw` on that
frame is 2.2–13.6 ms.

# Verified

- `cargo test -p voxel-client --lib world::tests`: **5 passed**, including the new pin.
- **The pin fails if the defect returns.** With the construction-time sizing removed and the count
  expected at `0`, the same test reports `left: 2` at the scene-arrival assertion — two buffers
  allocated on a live frame — and passes again once the sizing is restored.
- `cargo test -p voxel-client --lib`: **524 passed, 2 ignored, 1 failed** —
  `scene::tests::the_fades_cost_is_the_second_trees_records_and_nothing_else`, a timing-ratio test
  (`joined <= 2.0 * alone`) that fails when the machine is busy and passes when re-run alone. Not
  touched by this pass; it is a measurement, and it is now named as one that is load-sensitive.
- `cargo test -p voxel-work --lib`: 69 passed.
- The gated build (`cargo build -p shploingus`) is green.
- Runs: `C:/tmp/idle-1.log`, `idle-2.log`, `loaded-1.log` (40 s), `loaded-2.log` (85 s),
  `loaded-3.log` (70 s), `loaded-4.log` (70 s).

# What it does not do

- **The record upload is not this pass's.** `write_quads` still serialises the whole record array on
  a frame where a chunk arrived; under eight busy cores that is 14–40 ms and it is what pushes a
  streaming frame over 50 ms. Bounded and named, not fixed.
- **72 MiB of GPU memory for pairs a settled frame does not write.** Pooling or sizing from the
  first scene would remove part of it; neither is knowable before a scene exists, which is why the
  capacity is stated rather than guessed. The count is on the report line so the price is visible.
- **`hold` is dead code** in `world.rs` (the quad/body buffers grew their own `grown`/`hold` path);
  it is warned about by the compiler and was not removed here.
