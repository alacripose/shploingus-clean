# PATCH-049 — the runs that stay shaped, and the frame that says nothing

The mission this pass was given, 2026-10-22:

> *"The shell stage is now the biggest thing in the frame — 6.8 ms of a 15.3 ms frame idle and
> 5.7→10.9 ms loaded … Give shaped runs a stable owner: something that knows a run's text, size and
> paint, re-shapes only when one of those changes, and lets a settled frame prepare nothing it
> prepared last frame — with the window and content views still free to place runs wherever they
> like. The output must be pixel-identical: keep the offscreen shell renders as the witness and diff
> them before and after …"*

# Where the 6.6 ms was

The shell stage is compose → `cells::surface` → the hierarchy reading → `HudRenderer::prepare`. The
example's own bench already split the first three (`BENCH=1 shell-shot`: compose **0.04**,
cells **0.08**), so the stage was `prepare`. Splitting `prepare` itself with a throwaway instrument
over 400 live frames of the shipped binary:

| inside `prepare`, per frame | ms |
|---|---|
| shaping 45 runs | **2.79** |
| preparing their glyphs into 18 band renderers | **0.23** |

So the stage was **shaping text that had not changed**. The module's own doc had argued the cost was
unavoidable — *"a counter that changes every frame is text that changes every frame"* — and the
measurement is what that argument was missing: a frame changes **one** readout and reshaped the 44
beside it, and the shell's chrome (titles, tabs, labels, the console's rows) never changes at all.

# What changed

| file | the change |
|---|---|
| `crates/voxel-client/src/shaped.rs` (**new**) | `RunKey` (text, size in px, wrap in px) and `ShapedRuns`: `begin` / `ensure` / `get` / `settle`. The owner of shaped text, and the only place `set_text` + `shape_until_scroll` are called |
| `crates/voxel-client/src/hud.rs` | `placed: Vec<Placed>` (a run's key + the rect, paint, alignment and size it was placed with) replaces `buffers` + `boxes`; `prepare` builds bands into locals and returns untouched when the frame is byte-for-byte the last one; `text_areas` reads the store by key; `shaped_runs()` now counts **shapes**, and `settled_frames()` counts settled frames |
| `crates/voxel-client/src/lib.rs` | one private module declared |
| `crates/voxel-client/tests/settled_frame.rs` (**new**) | the pixel witness, in its own test binary |
| `crates/voxel-client/examples/shell-shot.rs` | the bench also measures *one run changed* — the frame a game actually draws |
| `docs/STATUS.md` | a measured row for the shell stage |
| `conformance/missing-docs.txt` | `voxel-client`'s pin re-taken (191 items, unmoved; only the bytes moved) |

# Two owners instead of one, and why that is the whole design

A run has two halves and they change at different rates:

* **what it says**, at what type size, at what wrap width — the input to shaping, which is where the
  milliseconds are; and
* **where and how it is drawn** — rect, ink, alignment — which is placement, is cheap, and is exactly
  what a window may change at will.

`shaped.rs` owns the first and `hud.rs` owns the second, and the boundary is `RunKey`. A readout that
moves to another window, changes ink or switches to right alignment is the **same** shaped run, so it
costs nothing to re-place; and a settled frame is one where the vertex bytes, the placement list, the
band partition, the scale and the extent are all equal to the last prepared frame's. That frame
returns before it touches a buffer, and `drawn` draws what is still there.

**The atlas is why the skip is all-or-nothing.** `glyphon` frees a glyph that was not used in the
current epoch when a new glyph needs its room, and it marks a glyph used as `TextRenderer::prepare`
reaches it. A frame that prepared *some* bands would leave the others' glyphs unmarked, and a fresh
glyph in a later band could take an atlas slot a reused band still points at. Preparing every band
when anything moved, and none when nothing did, means no band ever points at an entry the atlas
believes is free. The alternative — one band at a time — would have been faster still and wrong in a
way the pictures would show only once the atlas filled.

Retention: a run is released the first frame nobody places it (`settle`), so the store is bounded by
the runs on screen, not by the runs the session has seen. Counters that move every frame are shaped
once per value and dropped; the 34 static runs are shaped once and kept.

# The numbers

Same machine, dev profile, same harness as the last pass; the window settles at 605/605 with one
digest per run.

| | before | after |
|---|---|---|
| shell stage, idle | **6.46–6.77 ms** | **4.21–4.57 ms** (two runs) |
| frame, idle | 14.9–15.9 ms, 65–66 fps — draw 3.7–4.3 | **10.3–11.3 ms, 88–96 fps** — draw 2.6–3.2 |
| shell stage, 8 hogs | **7.53–7.63 ms** | **4.37–4.45 ms** |
| frame, 8 hogs | 18.8–19.4 ms, 51 fps — draw 5.4–5.7 | **12.8–13.4 ms, 75–77 fps** — draw 3.8–4.3 |
| `prepare`, settled (bench, pinned layer) | — (no such path) | **0.056 ms** |
| `prepare`, one run changed (bench) | **2.535 ms** | **0.128 ms** |
| `prepare`, one run changed (console open) | — | **0.166 ms** |

The shell stage fell by **≈ 2.4 ms idle and ≈ 3.2 ms loaded** and the frame by ≈ 4.5 ms idle; nothing
else in the split moved except the draw, which fell as a side effect of the frame being shorter
(4.3 → 2.6 ms idle, the same stage on a less contended queue). **The loaded numbers are steadier than
the last pass's** (75–77 fps across four reports, against 33-vs-48 on identical code that pass), and
they are still the number to trust least: the frame rate on this machine moves with the disk.

# The output is pixel-identical, and the witness is pixels

Six renders of the shipped shell through the game's own draw path (`shell-shot`): pinned layer,
console, and two overlapping windows, each with and without the cell surface. Old code and new code,
compared pixel by pixel:

| render | differing pixels |
|---|---|
| `pinned` / `pinned-cells` | **0** of 921 600 |
| `console` / `console-cells` | **0** of 921 600 |
| `overlap` / `overlap-cells` | **0** of 921 600 |

A single prepare cannot exercise the reuse path, so the durable witness is a test:
`tests/settled_frame.rs` draws the shell's frame, repeats it (**0 shapes, the frame is recognised, the
picture is identical**), then changes one readout and checks the incremental draw against a **renderer
that has never seen the frame** — the same pixels, with **1** run shaped instead of 34. That third
comparison is the point: the first two pass on a cache that never reuses anything.

# What it does not do, named

- **The other ~3.6 ms of the shell stage is untouched.** compose, `cells::surface` and the hierarchy
  reading are not text and were not in this mission; the stage is still the biggest single item in
  the frame (4.4 of 10.6 ms), and the next pass at it has to start there rather than in shaping.
- **A changed frame still re-prepares every band's glyphs** (0.13 ms), by the atlas argument above.
  Only shaping is skipped per run.
- **The keys are content, not position.** A window closing or an order change reuses the other
  windows' runs; an index-keyed cache would have reshaped the frame on every such move.
- **`prepare` still rebuilds every rect's vertices every frame** (a `Vec<u8>` of ~4 KB compared, not
  avoided). That is the cheap half and it is what makes the settled test exact.

# Structure, so the next pass builds with it

`voxel-client/src/hud.rs` is the painter: elements and strokes in, bands of rects and glyphs out, one
GPU pass. `voxel-client/src/shaped.rs` is the text: identity, shaping, retention, and the two
measurements a caller can read (`shaped_runs`, `settled_frames`). `HudRenderer` owns the placement
(`placed`), the uploaded vertices (`uploaded`), the bands and the renderers; `ShapedRuns` owns the
buffers. Nothing in `shell/` knows either exists — the mount still hands over `Vec<Element>` and
places runs wherever it likes, which is why this change touched no window and no view.

# One test that is not green, and it is not this change

`scene::tests::the_fades_cost_is_the_second_trees_records_and_nothing_else` asserts a **wall-clock
ratio** in the parallel unit-test binary, and it fails intermittently on this machine. Attribution,
measured: with this pass reverted and `shaped.rs` out of the tree, the same suite run measured the
cross-fade at **11.57 ms**; with this pass in, **11.77 ms** — the same number. What moves is the
*baseline*: one tree's 40 000-record build measured **1.47 ms** in one run and **9.72 ms** in the
other on the same binary, and the ratio flips with it. In isolation the test passes 3 of 3 runs
(0.9–3.1 ms a fade). The machine is at **98 % disk** (`C:` 12 GB free) and the tree's own
`.cargo/config.toml` already records this volume failing rustc mid-run, so a large allocation's cost
is not reproducible here. The test was not touched, skipped or weakened; it is reported as failing and
unattributable to this patch.

# Verified

- `cargo test -p voxel-client --test settled_frame`: **1 passed** (the witness above).
- `cargo test -p voxel-client --lib`: **511 passed, 0 failed, 2 ignored** — except the timing test
  above, which failed on 2 of 3 full runs and passes alone.
- `cargo test -p voxel-client --lib shaped::`: 4 passed (identity, reuse, retention, the stored
  layout).
- `cargo build -p shploingus`: exit **0**, so the build gate's census, conformance, prose frame
  (`voxel-client` **16 %** of its declared 16 %) and missing-docs pins hold on the final tree.
- The shipped binary, idle and under eight busy cores: shell 4.4 ms, 605/605 chunks,
  `0 stale-but-drawn`, scratch 0.0 MB, exit by `taskkill` only.

## Recipes

```recipe
segment: shell
axis: evidence_grade
ticket: T-0954
site: crates/voxel-client/src/shaped.rs:91
what: keep `ShapedRuns::ensure` the only place text is shaped, so a settled frame shapes nothing it shaped last frame
because: 45 runs were reshaped for one changed readout, 2.79 ms of a 6.8 ms shell stage; with the owner the stage is 4.21-4.57 ms and a settled frame prepares in 0.056 ms
check: cargo test -p voxel-client --lib shaped:: -> failures 0
witness: crates/voxel-client/tests/settled_frame.rs
expects: 0
requires: cargo test -p voxel-client --test settled_frame
```
