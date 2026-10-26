---
patch_id: PATCH-073
title: The shoulder the shipped frame draws — a real 6.00 × 2.00 m panel carries a 0.103 m shadow edge at `SHADOW_LATTICE`, and the lattice costs 8 452 asks a chunk
status: applied
operation: report
target: shploingus/shadow-sampling
base_ref: shploingus-clean@main
base_digest: sha256:cdeb73282b129f8a7b87123918711cd0ec45ef775357880db5f83427c4c3f7dc
dependencies: [PATCH-070, PATCH-072]
source: user-directed-conversation
created_at: 2026-10-23
approval: "user-directed 2026-10-23 — the audit's gap: the 0.200 m shoulder was measured only on shadow-lattice's synthetic square metre, so measure the same shoulder on the scene the binary draws"
live_mutation: false
---

# Change

**The shadow's shoulder had one measurement, and it was on a face this world does not build.**
PATCH-070's 0.200 m was measured on a **synthetic** one-square-metre face placed in an otherwise
empty scene, with the field's own 0.5 m edge. That is the witness for the *lattice* and not for the
**shipped frame**: the binary draws a streamed 605-chunk window, merged rectangles metres wide,
refinement and displacement on the faces that declare them, and a `CellField` holding whatever cells
the worker has answered. Nothing in the corpus measured the edge a player stands in front of.

**The same example now measures it.** `shadow-lattice`'s second half streams the game's own window
(`GAME_SEED`, `GAME_WINDOW`, the spawn eye) at `GAME_TICK`, binds the game's own `CellField` with the
same `exposed_bands(bands, grade)` the binary uses, and scans the +Y faces within two chunks of the
eye for a panel whose lattice light varies along one axis. A face whose light is a gradient rather
than a shadow is refused: the scan demands the two ends differ by at least half the face's own
brightness, and that the edge runs along the face (three rows across it smear by no more than 35 % of
the drop). The chosen panel is measured once at 24 × 8 lattice samples; then the whole scene is built
through `ResidentScene` — the binary's own residency — at `SHADOW_LATTICE` and at lattice `1`, with
the **field's own `at`** as the light (not a test closure: the frame's real ask), and drawn through
`WorldPipeline` with the panel filling the view. A third build, with every sample at the panel's
brightest, is the clear frame; the difference is the shadow's own footprint, and the 90 % to 10 %
width is measured over 25 rows across the panel, in pixels and in metres.

**The number, and the claim it settles:** on a real **6.00 × 2.00 m** panel at (-48.0, 17.0, -105.0),
in the shipped window at the shipped hour (Sun 10.6°), the edge fades over **7.1 px = 0.103 m** at
`SHADOW_LATTICE` against **284.2 px = 4.146 m** at lattice `1` — the same view, the same field, the
same panel. The sub-metre structure the old path could not produce is in the shipped frame, and the
coarse comparison is measured there rather than inferred from the synthetic face. The example prints
both, beside the synthetic 41.0 px = 0.200 m against 136.0 px = 0.663 m.

**The cost is stated as the frame pays it.** The fine window is **242 060 corners / 411 954 indices**
against **228 700 / 343 050**, and it asks the field **5 113 375** times against **457 400** — **8 452
asks a chunk against 756**, at **0.28 ms against 0.05 ms a chunk** (median of five full-window syncs:
166.4 ms against 27.5 ms). The binary lands and re-emits chunks under its quad budget, so the
per-chunk line is the unit a landing or a light-cadence re-emit pays; the full-window figure is the
cold sync the game spreads over frames.

# Acceptance

- `cargo run --release -p voxel-client --example shadow-lattice` exits **0** and prints the shipped
  fine and coarse shoulders; its assertions fail the run if the fine edge is not positive, not less
  than half the coarse edge, or not below 0.5 m.
- The synthetic face's numbers are unchanged by the addition (41.0 px = 0.200 m against 136.0 px =
  0.663 m; 41 646 of 147 456 px differ), so the shipped measurement is an addition and not a
  replacement.
- The panel is found on the shipped seed and hour without tuning: 3 431 field asks and 2 632 marches
  in 0.02 s, and the scan refuses gradients by the half-brightness rule.
- `cargo test -p voxel-client` — **543 passed, 0 failed, 2 ignored** in the lib, the integration test
  1 passed, 0 doc tests; `cargo check --workspace --all-targets` — **Finished**, no errors.

# Evidence

`cargo run --release -p voxel-client --example shadow-lattice` (RTX 2070 SUPER, Vulkan, 512 × 288,
one sample), 2026-10-23:

```
shipped: 605 of 605 chunk(s), 66.3 MB, window ChunkWindow { radius_chunks: 5, vertical_chunks: 2 } at tick 9000
shipped: sun 10.6 deg up, grade 1.000; the field is marched on this thread
shipped: panel 6.00 x 2.00 m at (-48.0, 17.0, -105.0) — 24 x 8 lattice sample(s); 3431 ask(s), 2632 march(es), 0.02 s
shipped fine: the shadow's own footprint fades 90 % to 10 % over 7.1 px = 0.103 m (the panel is 411.3 px across), across 21 level(s) of grey
shipped coarse: the shadow's own footprint fades 90 % to 10 % over 284.2 px = 4.146 m (the panel is 411.3 px across), across 48 level(s) of grey
shipped cost: 242060 corner(s) / 411954 index(es) at the lattice in 166.4 ms (median of 5, 153.9..166.9) and 5113375 field ask(s), against 228700 / 343050 at lattice 1 in 27.5 ms (26.6..28.4) and 457400 ask(s); the clear frame is 228700 corners in 46.7 ms
shipped cost, per chunk: 0.28 ms and 8452 ask(s) a chunk at the lattice against 0.05 ms and 756 at lattice 1 (605 chunk(s))
```

- The fine frame's difference profile is **21 grey levels** deep against the coarse frame's **48**;
  both are measured over the same rows of the same panel. The fine frame reaches less of the shadow's
  depth at the sampled rows; it is reported rather than smoothed, because the 10–90 crossings are far
  above quantisation either way.
- The panel's own share of the corner growth is **225 against 4** (a 24 × 8 grid of shared samples);
  the window's **+13 360 corners** include other faces whose lattice light the scan's measured cells
  crossed, so the geometry delta is the window's and not one panel's.
- Single full-window samples on this machine ranged 151 ms to 201 ms; the record quotes the median of
  five because one wall-clock sample is not a measurement.

# What this does not prove

- **Not geometric sharpness.** The 0.103 m is the *rendered* shoulder, interpolated across the
  field's 0.25 m cells; a finer occupancy answer would move it. It is a statement about the frame.
- **One panel, one seed, one hour.** The scan found a shadow crossing a 6 × 2 m face two chunks from
  the spawn at tick 9000 and the Sun at 10.6°; it is not a claim about every face, every hour or every
  seed. A population would need a sweep, and this record is one face.
- **Not a fully lit window.** Only the cells the scan measured are the field's; everywhere else the
  scene falls back to the rig (`None`), exactly as the binary does for an unanswered cell. The cost is
  the lattice's sampling and the corners it emits, not the cost of a window whose every cell is held.
- **Not the steady-state frame.** The times are a cold sync of all 605 chunks with `usize::MAX`; the
  binary's own budget lands fewer chunks a frame. The per-chunk figure is the unit, not the frame's
  total.
- **Not a test the tree runs.** It needs a GPU and lives in an example; it asserts and exits non-zero,
  but a machine without a device cannot execute it, and no CI job here does.

# What this does not touch

The gate, the walk, the exposure (`EXPOSURE_GRADE`, `MOONLIGHT_GAIN`), `SHADOW_LATTICE`, the arena
budget and every assertion: untouched. This record adds no behaviour; it adds a measurement and its
witness.

# Rollback

Delete the `shipped` function and its call at the end of `main` in
`crates/voxel-client/examples/shadow-lattice.rs`. The synthetic face's measurement remains; the
shipped frame's edge is unmeasured again.

## Recipes

```recipe
segment: game
axis: evidence_grade
ticket: T-0964
site: crates/voxel-client/examples/shadow-lattice.rs:246
what: measure the 90-to-10 percent shadow shoulder on a real 6 x 2 m panel of the streamed shipped scene through ResidentScene and WorldPipeline, at SHADOW_LATTICE and at lattice 1 on the identical view
because: the only shoulder in the corpus was measured on a synthetic square metre, so nothing measured the edge in the frame the player sees; the shipped panel reads 7.1 px = 0.103 m against 284.2 px = 4.146 m, and the lattice asks the field 8452 times a chunk against 756
check: cargo run --release -p voxel-client --example shadow-lattice -> exit 0
witness: crates/voxel-client/examples/shadow-lattice.rs
expects: 0
requires: cargo test -p voxel-client --lib
requires: cargo check --workspace --all-targets
```
