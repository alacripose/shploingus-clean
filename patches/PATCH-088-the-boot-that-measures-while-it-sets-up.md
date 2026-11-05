---
patch_id: PATCH-088
title: The boot that measures while it sets up — launch cost split from build cost, and the fields started before the window
status: applied
operation: optimise
target: shploingus/boot
base_ref: working tree at 2026-10-26, after PATCH-087
dependencies: [PATCH-087]
source: user-directed-mission
created_at: 2026-10-26
approval: "user-directed 2026-10-26 — 'running the client bin takes a while' … 'distinguish build cost from runtime cost with numbers … start the freshly built client in a bounded way that measures time-to-world-open and time-to-horizon-complete'"
live_mutation: false
---

# The wait was two different waits

`cargo run -p shploingus` was measured on 2026-10-26 as three separate things rather than one
number. All of it is in `docs/baselines/frame-cost.json`'s `launch` section; the shape:

| | |
|---|---|
| build, nothing changed | **1.9 s** |
| build, one client file touched | **19–25 s** (four rebuilds: 19.4, 22.6, 22.7, 24.9) |
| build, after a dependency restore | **7 m 11 s**, once |
| process start → world-open | **4.68 / 7.46 / 6.15 s** across three launches |
| world-open → 21 of 21 rungs drawn | **2.6 s** (was 3.9 s of samples started later) |
| process start → 21 of 21 rungs drawn | **7.25 s** in the run whose world-open was fastest (13.88 s before this patch) |

**The spread is the machine's and it is stated rather than averaged away**: three launches of one
unchanged binary measured world-open at 4.68, 7.46 and 6.15 s. That is why the durable figure here
is **2.6 s from world-open to the whole ladder**, which is measured inside the loop and does not
inherit the setup's noise.

**The seven minutes were not the code under change.** Cargo recompiles `voxel-client` and the bin in
~20 s. The long build is kache restoring every dependency's rlib **by copy** from `C:` to the tree on
`F:` — the project's `.cargo/config.toml` says so in its own words — and it happens once per
invalidated dependency graph, not per edit. A second run of the same binary is `Finished in 1.94s`.

# What the process was actually doing

The frame instrument cannot see any of this: it starts at the first frame. Eight boot phases were
added behind the existing `SHPLOINGUS_PROBE_DEBUG` — one clock set at the top of `main`, printed
where each phase ends. On this machine (`SHPLOINGUS_PROBE_DEBUG=1`, one bounded 20 s launch):

| phase | fastest run | spread over three runs |
|---|---|---|
| gate report | 3 ms | 2–4 ms |
| shell mounted | 2 470 ms | **2 470 / 6 611 / 3 224 ms** — `ShellMount::new`: 45 KB of cell-frame artifacts, 16 windows adopted |
| judgement | 689 ms | 689 / 1 019 ms |
| event loop | 6 ms | — |
| window | 43 ms | 43–49 ms |
| graphics | 1 469 ms | 1 469 / 1 897 ms — device, surface, pipelines, shader compilation |
| world-open | 4 680 ms | **4 680 / 7 456 / 6 153 ms** |

So the first frame is drawn 4.7–7.5 s after the process starts, and the window is on screen ~3.2–7.5 s
in. **Nothing there is the ladder or the far field.** The widest term is the shell mount, and it is
noisy in a way that is the machine rather than the code — the same three launches spread by 2.7×.

# The one fix that was a boot-order fix

The four fields that measure the world — the 8 m far ring, the 16 m and 32 m rings, and the surface
ladder — need **no device and no surface**. They were built after `Graphics::new`, so their threads
started at 5.4 s and then competed with the streamer's 605 chunk builds for the CPU. They are now
built at the top of `resumed`, before the window, and measure *through* the device's own 1.5 s:

| | before | after |
|---|---|---|
| tier 8 on screen | 7 718 ms | **6 677 ms** |
| all 21 rungs, 28 478 quads | 13 883 ms | **7 249 ms** |
| the ladder's own samples | 3 904 ms | **1 608 ms** |

The samples got *faster* as well as earlier because the worker now has the machine to itself during
the device's creation instead of sharing it with the streamer. No rendering changed, no library code
changed, no new field was added: four `Some(...)` constructions moved, and the frame loop sees
exactly what it saw before.

# What is left, named

* **The shell mount (2.47 s) is now the largest term before a frame**, and it is eager: the shell is
  constructed before any frame exists. Making it cheaper means changing *when* it is built or drawn
  — a UI change with real risk — so it is named here rather than attempted under this scope.
* **GPU pipeline creation (1.47 s)** is the next. A `PipelineCache` would repay it on the second and
  later launches of the same shader; it is driver-side work and a change of its own.
* **The judgement (0.69 s)** runs on the game's own tick 0. Its per-frame cost shows up in the stall
  report (`judge 124 ms` in a 619 ms frame at 20 s), which is a frame-cost question and not a boot one.
* **The far rings are still 20–40 s of census** (PATCH-086's measured budget). They are started
  ~2.2 s earlier now, and that is all this patch claims: the long-range field's fill is its own
  problem, and the ladder above it no longer waits on it.

# Evidence

* Three bounded launches, none of them the user's process. Fastest: `boot:` lines at 2.9 / 2 470.5 /
  3 158.8 / 3 165.2 / 3 209.1 / 4 677.7 / 4 680.1 ms, `surface: 2 of 21 rungs` at 6 676.6 ms,
  **`surface: 21 of 21 rungs, 28 478 quads, 1 608.5 ms of samples` at 7 249.1 ms**; the other two
  measured world-open at 7 456 and 6 153 ms. A launch with `SHPLOINGUS_PROBE_DEBUG` unset prints no
  `boot:` line at all, which is the check that the probe costs nothing when nobody asks for it.
* The same launch's frame lines show the ladder's quads drawn in the frame (`28 478` in the surface
  report, `quads` climbing in the fps line), so the measurement is of drawn geometry, not of a
  counter nobody reads.
* Library and shader untouched: `voxel-client`'s 601-test suite (PATCH-087's run) still stands, and
  `cargo build -p shploingus` is clean apart from the file's one pre-existing warning.
* No process was killed: the launches were bounded by the harness's own 20 s deadline, and the game
  instance the user had open was never signalled.
