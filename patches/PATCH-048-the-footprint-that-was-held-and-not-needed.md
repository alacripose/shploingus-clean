# PATCH-048 — the set that kept what it walked past, and the scratch that was never idle

The mission this pass was given, 2026-10-22:

> *"Establish first whether the set stabilizes over a multi-minute run or leaks — if it leaks, the
> leak is the defect and its owner comes first; if it does not, cut the two largest declared
> consumers (scratch sizing, over-residency and arena trim on eviction) rather than shaving
> constants. Every change must be argued from the shipped binary's own numbers: working set over time
> idle and under the eight-hog load, and the same fps and stage split the last pass measured … Do not
> tune the shell stage, the draw or the light field in this pass."*

# First: it does not leak

Two runs of the shipped binary, `SHPLOINGUS_FRAMES=1`, sampled every ten seconds by `tasklist`:

| run | length | working set | resident | verdict |
|---|---|---|---|---|
| idle, pre-fix (`target/mem-long2.csv`) | **240 s** | 383 → 413 → 391 → 406 → 392 → 413 MB, oscillation with **no trend** | 605 → 660 after the spawn fell one chunk, then flat | plateaus |
| 8 hogs, post-fix (`target/load-long.csv`) | **130 s** | **298–323 MB**, flat | 605, flat | plateaus |

The audit's *"311 → 370 MB in 32 s"* was therefore **warm-up plus the spawn's fall**, not a leak: the
frame reaches its window in ~15 s and the RSS climbs with the chunks it is charging. What did not
plateau correctly was the *resident set*: 660 chunks held for a window of 605, and 880 after a longer
fall, with the charge at 195.7 MB of 256 — the over-residency the mission names. So the leak path is
closed and the mission's second branch is the one this patch takes.

# What changed

| file | the change |
|---|---|
| `crates/voxel-work/src/stream.rs` | `pump` **cancels, evicts and forgets by the `ChunkWindow`**, not by `Horizon::Residency`; the doc states why |
| `crates/voxel-work/src/pool.rs` | a worker **holds one mesher's raster only while the queue has work** and gives the 4.8 MB back when it drains; `Pool::scratch_bytes` is what is held now, `scratch_ceiling` is one raster a worker |
| `docs/STATUS.md` | the scratch row says held-while-streaming, zero at rest, with both numbers |
| `conformance/missing-docs.txt` | `voxel-work`'s pin re-taken (61 items, unmoved; only the bytes moved) |

# The held set is the window

`ChunkWindow` is what the streamer **asks for**, what `absorb` **admits**, and what the caller
**draws**. Eviction keyed on `Horizon::Residency` instead — radius 24, a 49³ cube of **117 649
chunks**, 21 GB of meshes at the measured 180 kB a chunk — so no run could ever reach the eviction
boundary and every chunk a step left behind stayed charged, resident and drawn. The shipped run is
the witness: **660 for 605** after one chunk of fall, 63 245 quads for a window of 57 175, and 145.7 MB
charged. The same predicate now governs all four decisions (`cancel_where`, the eviction filter, the
failed-set retain, and — already — the ask loop), so the engine's holding set and the set it was
declared to hold cannot drift apart.

**The test that fails without the fix**:
`stream::tests::the_held_set_is_the_window_and_not_the_wider_residency_horizon` walks four steps, one
chunk east and one down every other step, and asserts `len() <= window.count()` after every pump and
`== count()` once settled. Restoring the residency rule as a mutation makes it fail on the first
step (`target` run, 2026-10-22).

# The scratch is held while there is work

A mesher's raster is `(136³ + 128²) × size_of::<MaterialId>()` — **4.8 MB**, one per worker because it
is never shared. The pool allocated one per thread at spawn and kept it for the life of the process,
so a settled world that was meshing nothing still held **15 × 4.8 = 72.4 MB**. The `work` loop now
keeps the raster across jobs and drops it when the queue drains, re-allocating on the next burst:
the streaming case still allocates once, and the settled case holds nothing. `scratch_bytes` became
an atomic the workers maintain, so the HUD and the frame report **what is held**, and
`scratch_ceiling` states the bound a streaming caller must set aside.

Measured in the shipped binary: `scratch 72.4 MB` during the load, `scratch 0.0 MB` on every
settled second of every run after it.

# The numbers

Same machine, dev profile, same harness, window settled at 605/605 and one digest per run:

| | before | after |
|---|---|---|
| resident / wanted | **660 / 605** | **605 / 605** |
| charged | 145.7 MB | **132.0–132.4 MB** |
| quads drawn | 63 245 | **57 175** |
| scratch at rest | **72.4 MB** | **0.0 MB** |
| working set, idle (≥130 s) | 379–413 MB | **298–313 MB** |
| working set, 8 hogs (130 s) | 374 MB (one sample) | **298–323 MB** |
| frame, idle | 15.8–15.9 ms, 63 fps — shell 6.7, draw 4.4 | 15.2–15.3 ms, 65 fps — shell 6.8, draw 4.0 |
| frame, 8 hogs | 30.2 ms, 33 fps — shell 10.9, draw 9.3 | 11.9 ms, 84 fps — shell 5.7, draw 2.9 |

**The working set fell by 90–110 MB (≈ −24 %) and the frame is not worse.** The idle split is
unchanged inside noise (shell 6.7–6.8, draw 4.0–4.4 against the last pass's 6.3–6.8 / 3.7–4.0). The
loaded row is the one to read carefully: the machine's own spread across runs swamps the difference —
identical code and identical geometry produced **33 fps** in one loaded run and **48 fps** in another
in this session, and the shell stage moved **5.7 → 10.9 ms** for the same instructions — so this patch
claims **no speed-up** under load and only that no cost was observed. **The dominant stage is still the shell** (6.8 ms idle, 5.7 ms
loaded), and it is not tuned here; neither are the draw or the light field.

# What it does not do, named

- **The 256 MB ceiling is not shrunk.** It is a cap, and the run never needed more than 132.4 MB of
  it; declaring a smaller cap would change a number on a report without changing a byte of allocation.
- **The GPU arena is not trimmed.** `hold` still grows by doubling and never shrinks; with the
  window fixed at 605 chunks the high-water mark is now the window's own 17 MB of corners, where the
  over-resident run had grown it to 880 chunks' worth first.
- **Walking re-allocates each worker's raster once per burst**, because the queue drains between
  steps. The trade is ~72 MB of alloc/free per step against holding it for minutes of standing
  still; it is accepted, and it is on worker threads, not the frame thread.
- **The light field still measures nothing** (`0 cell(s) marched`, 1.14 M corners rigged): the
  vertex format and the five rebuilds a second are still paid for no measurement. Its own pass.

# One gate that fired, and was obeyed

The first version of these comments pushed `voxel-work` to **16 % plain comment against its declared
15 %**, and the build refused by name — *"the share may fall and may not rise … move the explanation
rather than arguing the number up"*. The comments were cut back (the argument lives here instead) and
the crate measures **15 %** again. The budget was not raised.

# Verified

- `cargo test -p voxel-work --lib`: **69 passed, 0 failed, 2 ignored** (the new eviction test and the
  reworked scratch test included).
- `cargo test -p voxel-client --lib`: **508 passed, 0 failed, 2 ignored**.
- `cargo test -p voxel-mcp --test axis_route`: **10 passed** after the pin re-take.
- `cargo check --workspace`: exit **0**; `cargo build -p shploingus`: exit **0**, so the build gate's
  census, conformance, prose frame and missing-docs pins all hold on the final tree.
- The shipped binary, idle and under eight busy cores: no wgpu validation error, 605/605 chunks,
  `0 stale-but-drawn`, scratch 72.4 MB while loading and **0.0 MB** at rest.

## Recipes

```recipe
segment: world
axis: evidence_grade
ticket: T-0954
site: crates/voxel-work/src/stream.rs:658
what: keep eviction keyed on the `ChunkWindow` — the set the streamer asks for, admits and draws — and not on the residency horizon
because: radius 24 residency is a 49-cube of 117 649 chunks, so 660 stayed charged for a window of 605 and 145.7 MB against the 605/605 and 132.0 MB the window rule settles at
check: cargo test -p voxel-work --lib stream::tests -> failures 0
witness: crates/voxel-work/src/stream.rs
expects: 0
requires: cargo check --workspace
```
