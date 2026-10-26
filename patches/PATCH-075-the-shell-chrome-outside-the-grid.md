---
patch_id: PATCH-075
title: The shell chrome outside the grid — the cell raster restricted to a window's content space
status: implemented
operation: repair
target: shploingus/shell
base_ref: shploingus-clean@aef0971
base_digest: sha256:67379c35af2fa29f7c03384d95efb1f315a57ed3630ebde0a64073a881a9460a
dependencies: [PATCH-071, PATCH-074]
source: user-directed-mission
created_at: 2026-10-24
approval: "user-directed 2026-10-24 — \"forbid the cell format to draw the shell's main drawables, only addressable in the windows content space\"; implemented, not proposed"
live_mutation: false
---

# Change

**The cell raster was drawing the whole shell.** `shell::cells::surface`/`raster` took every
composed element — the bar, the title bars, the borders, the panels' chrome, the console's frame —
and snapped all of it to the 8 × 16 grid. The player's instruction is the opposite: the cell format
is addressable **only inside a window's content space**, and the shell's main drawables are drawn by
the element path.

## What changed, and where

- `crates/voxel-client/src/shell/cells.rs`
  - `surface_in(elements, viewport, content)` is the content-aware raster. An element is content
    only when **its whole rect lies inside one of the declared content rects**
    (`inside_content`, which treats a zero-area rect as chrome because a cell cannot be thinner than
    its row). Content elements are rasterised with the viewport's own cell origin, so a run lands on
    the grid the window's rect was snapped to; the runs are emitted where the **first** content
    element stood, so the content layer keeps the z-position the composer gave it. Everything else is
    cloned through untouched.
  - `surface_at` and `raster` take `content: &[Rect]` beside the version, so version 1 can never be
    asked to draw the chrome.
  - `rect_of` reads `Surface`, `Text` and `Rule` — the three element kinds the composer emits.
- `crates/voxel-client/src/shell/mount.rs` — `Mount::content_space()` states the mask from
  `window_body`, the same arithmetic the window drawing and the pane hit-tests use. Closed and
  minimized windows contribute nothing, a maximized window contributes the body of the rect it is
  actually drawn at (`Mount::rect_of`), and a docked pane's space is its host's.
- `crates/shploingus/src/main.rs` — the frame's one raster call passes `self.shell.content_space()`.
- `crates/voxel-client/examples/shell-shot.rs`, `crates/voxel-client/tests/cell_version_refusal.rs`
  — the same call, so the instrument and the witness measure the drawing the app makes.

## Boundaries, chosen and named

1. **A straddling element is chrome.** An element whose rect crosses a content edge is passed through
   whole; snapping it would move the part that is outside the window, which is the defect the
   restriction exists to stop.
2. **A zero-area rect is chrome even inside a window** — the 1 dp rules belong to the element path.
3. **No content space is no cells.** With `content` empty the shell draws its composed elements;
   there is no fallback to a whole-viewport grid, because that fallback *is* the forbidden drawing.

# Acceptance

- The chrome is never snapped: an identical surface and paint placed outside the content space
  survives the raster byte-for-byte and first in the list.
- A body inside the declared content space is rasterised; its text reaches the grid.
- No cell is painted above the content space.
- With no content space declared, the composed list is returned unchanged.
- The package's declaration still selects the drawing (`cells 0` = elements, `cells 1` = raster), and
  a refused declaration still leaves the drawing in force untouched.

# Evidence

`docs/baselines/mount-v2-shell-cells-2026-10-24/` — `readings-2026-10-24.txt`,
`tests-2026-10-24.txt`, `display-packages-2026-10-24.txt` (this patch's 89-line capture) and
`display-packages-after-2026-10-24.txt` (the re-pin PATCH-076 records).

- `shell::cells::tests` — **15 passed, 0 failed**, including
  `a_frame_draws_its_cells_and_content_without_one_stays_composed` (the content-space
  prohibition; the audit found the name this record first cited did not exist).
- `voxel-client --test cell_version_refusal` — **2 passed, 0 failed**.
- `cargo check -q --workspace --all-targets` — exit 0.
- `voxel-layers display-packages` — **89 lines**,
  `sha256:3f7c92309db59d04aab9bf03ebe7e479434f943627e434715971966ee28c0809` (unchanged by this
  patch: no package artifact was added).
- **Boundary reading, updated the same day by PATCH-076**: the digest above is the reading *before*
  the cell frame artifact existed, and PATCH-076 is the record that adds it. The emit now reads **180
  lines**, `sha256:13d93263cf7dd64cd128fc8b4ad8535b658b480bb41e5895a83c4ba75996b7c4`, with `frames 1`
  for `alpha-shell`. The old digest is kept here rather than overwritten: it is this patch's proof,
  and the pair is the deliberate re-pin recorded in PATCH-076's baseline folders. The later pass that
  put the frame into `SHPKG003` re-captured the same two digests in
  `docs/baselines/mount-v2-packaged-cellframe-2026-10-24/`; the emit did not move, because the
  display package's folder bytes did not.
- The mask this patch built is still the mask: the walk still refuses anything outside a **declared**
  content rect, and PATCH-076 narrowed *which* rects are offered to the raster (only a graded frame's
  window), it did not widen them. The walk was renamed `surface_in` → `frames_in` in that pass (same
  `inside` test, now over the frames' footprints), so this patch's rollback names the function as it
  stood when the patch landed.
- No commit, no staging.

# Rollback

Delete `surface_in`, `inside_content` and `rect_of`; restore `surface_at(elements, viewport,
version)` and `raster(elements, viewport, version)`; remove `Mount::content_space()`; pass the old
argument lists at the four call sites. Nothing else in the tree reads the mask, so the rollback is
those five edits and no data migration.
