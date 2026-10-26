---
patch_id: PATCH-076
title: The cell frame inside mount v2 — declared by the package, drawn only in a window's content space
status: implemented
operation: repair
target: shploingus/shell
base_ref: shploingus-clean@aef0971
base_digest: sha256:67379c35af2fa29f7c03384d95efb1f315a57ed3630ebde0a64073a881a9460a
dependencies: [PATCH-071, PATCH-075]
source: user-directed-mission
created_at: 2026-10-24
approval: "user-directed 2026-10-24 — cell frames must be secure inside the mount v2 package format and never draw the shell's chrome; the prohibition and the streamed graded artifact its first version named as owed are both implemented here"
live_mutation: false
---

# Change

**A cell frame is a package's declaration and a window's content, and nothing else.** This record is
the mount-v2 half of PATCH-075: what a package may say about cells, what the shell will refuse, and
where the frame is allowed to draw.

## The declaration (format side, unchanged grammar)

`package.cells` is the declaration, read where the package is mounted:

- header `shploingus.cells/1`; **`cells 0`** is the element drawing and **`cells 1`** is the raster;
- an old header (`shploingus.cells/0`) is **refused by name with file and line**, printed once, and
  the drawing and arrangement already on screen keep drawing — it is never silently substituted;
- an undeclared version is refused by number, and the message lists the versions this build draws
  (`voxel-package::cells::CellVersion::IMPLEMENTED`).

The grammar is the format crate's; `shell::cells` re-exports `declare`, `CellVersion`,
`CellRefusal`, `Declaration` rather than copying them, so the client cannot hold a second opinion
about what a package may declare.

## The frame's address (draw side, enforced)

- A frame may draw **only inside a content container** of an open window, computed by `window_body`
  — the same function the window drawing and hit-tests use — and only where a frame the mount graded
  claims the rect (`Mount::frame_space`).
- Outside that space the shell's main drawables are composed elements. The raster cannot reach the
  bar, a title bar, a border or a panel frame, because `cells::frames_in` classifies whole rects
  against the frames' footprints (`inside`) and passes everything else through (PATCH-075's rule,
  kept; the function was `surface_in` when that patch landed).
- With no graded frame — no package frame declared, no open window, a closed or minimized one — the
  frame draws nothing: `frame_space()` is empty and the raster returns the composed elements
  unchanged. **A window's content is not cells by being a rectangle**; it is cells when a frame claims
  it.
- The mount is asked **per frame**, so a package that arrives mid-run selects its drawing and its
  frames in the act that mounts it, and a refused one changes neither the version, the frames nor one
  raster run.

## The artifact this record owed, now landed

`shploingus.cellframe/1` — `crates/voxel-package/src/cellframe.rs`, one file per frame under the
package's `frames/`, named by a `row` in the recipe exactly as a layer is. The grammar is `layer.rs`'s
discipline rather than a resemblance of it: a `frame` row carrying the id the recipe names, then
five required keys (`window`, `cols`, `rows`, `order`, `requires`), an unknown key refused by the key
and the line, a duplicate key
refused the same way, `fill` and `run` rows refused when they land past the grid, a role outside
`surface`/`chrome`/`text`/`accent`/`divider` refused by word, an empty frame refused by id, an old or
borrowed header refused by number, and a record missing one of the four patch sections refused by the
section's name. **The join refuses nine things beyond the grammar's row checks**: an unreadable
`frames/`, a non-UTF-8 frame, a file stem that is not its frame's id, a frame the recipe names by no
row, a window the package does not declare, a grid larger than that window's declared extent, a
missing record, a record missing one of the four sections, and a stray record with no frame. Each
refusal carries the file, the id and the cell that wrote it.

**The mount is where a frame becomes drawable.** `Mount::new` grades the frames the package carries
(`cellframe::read_artifacts`) before it builds a window, and `Mount::load_package` grades the same
map after compiling a folder's `frames/` into the frame, so the two paths cannot disagree. `adopt` is
only the arrangement step and grades nothing. A refused frame refuses the mount and the frames and
arrangement already on screen keep drawing. `Mount::frame_space()` places every graded frame in its
window's content container — the same `window_body` arithmetic the window drawing and hit-tests use —
and contributes nothing for a closed, minimized, docked or too-small window, because a frame is a
declaration of a whole grid and drawing part of one would be a cut the package did not state. The
shell's one raster call (`shploingus/src/main.rs`) passes that list, so content the compiler composed
becomes cells when, and only when, a graded frame claims the rect.

**The frame travels in the package, not beside it.** `build.rs` reads `display-packages/alpha-shell`'s
`frames/` through `recipe::artifacts_in` and compiles it into `SHPKG003`, whose artifact table follows
the recipe source. `recipe::load` reads that table into `Loaded::artifacts`, and `Mount::new` grades
from it, so the embedded package draws its cell frame without a workspace checkout beside the binary;
a folder compiled mid-run carries the same files through `compile_with_artifacts`. `SHPKG002` is
still read, with no artifacts, so an older emitted package is not refused for being old.

**The format is in the walk and in the matrix.** `voxel-layers display-packages` prints `frames 1`,
the frame's one-line description and its rendered 56 × 22 grid; the census writes one real row for the
format — `shploingus.cellframe/1`, kind `format`, witness
`display-packages/alpha-shell/frames/MAP-0001.frame` — beside its 16 folder rows, with 0 unclassified;
and `voxel-axis-verify` files the format's own cell — `Segment::Tooling` / `AxisKind::Grade`, `grade
B`, ★5, witnessed by `display-packages/alpha-shell/frames/MAP-0001.frame` — through the same
`Subject`/rows path as the seven segments (`subject::FORMAT`, appended to the one measurement loop,
never a second builder). Its six rows are the frame's witness, the format reader's acceptance (which
carries the grammar and the record's four sections), the recipe's row, the declared window, the grid
fit, and `packages/census.tsv` naming the format. **The grade is computed from those rows**: `B` only
when all six lit, `C` when any measured row did not, and no cell at all when the reader refused — a
witness and a census row alone are not a star over a format nothing graded. There is no hardcoded
`A`, and no row is asserted as a constant. That is one cell beside the seven segments' cells, not a
second report.

## What a frame still is not: a world position

A frame's address is **(col, row) inside one declared window's content space**. The format carries no
world position, depth, slice, normal or camera — deliberately, and it is stated here because the map
window's lattice provokes the question. A **world-positioned** raster lattice (the same cells placed
in 3D, or a frame that follows the eye through the world) is a different artifact class with its own
declaration, its own address law and its own grading; adding an `at x y z` key to this one would make
a window's content ambiguous with the world's and would let a package draw outside the content space
by another route. Also still owed, and not simulated: a `voxel-model` frame record streamed through
`Scene`/`WorldPipeline` with glyph runs beside it. What exists is the declaration, the grade, the
placement and the draw; what does not is a world-space frame, and no census row or test claims one.

# Acceptance

- A package's `cells` declaration alone decides the drawing; version 0 returns the composed elements,
  version 1 rasters content only.
- An old or undeclared header is refused by name/number with file and line, and nothing on screen
  moves.
- No cell is drawn outside a window's content container, and none at all when no window is open.
- **A frame is an artifact before it is a drawing**: a frame the recipe names by no row, addresses an
  undeclared window, overruns its grid or carries no record is refused at mount with file, id and
  line, and the shell draws the frames it had.
- **Only graded frames become cells**, and the format has one cell in the axis matrix and one row in
  the folder census.
- **The frame survives the packaged form**: `SHPKG003` carries `frames/` in its artifact table,
  `Mount::new` grades from `Loaded::artifacts`, and a shipped package draws its frame with no
  `display-packages/` on disk — proven by a module test that compiles the folder, loads the frame and
  mounts it through the package alone.
- The format has no world position: nothing here addresses a 3D cell, and the limit is written in the
  package README beside the format.

# Evidence

`docs/baselines/mount-v2-shell-cells-2026-10-24/` — `readings-2026-10-24.txt` (the first re-pin,
with the old and new emit digests), `tests-2026-10-24.txt`, `display-packages-2026-10-24.txt` (the
89-line capture before the class existed) and `display-packages-after-2026-10-24.txt` (the 180-line
capture after it).

`docs/baselines/mount-v2-packaged-cellframe-2026-10-24/` — the re-capture this pass adds:
`display-packages-before-2026-10-24.txt` (the same 89-line / `3f7c9230…` capture, kept unedited),
`display-packages-after-2026-10-24.txt` (180 lines / `13d93263…`), `readings-2026-10-24.txt` (the
format row, the pin re-take and the packaged-package reading) and `tests-2026-10-24.txt`.

- `voxel-client --test cell_version_refusal` — **2 passed, 0 failed** (arriving package selects its
  drawing; refused package changes nothing; version 0 is the composed elements).
- `shell::cells::tests` — **15 passed, 0 failed**, including the content-space prohibition.
- `cargo test -p shploingus --test packaged_frame` — **1 passed, 0 failed**. It reads the build's own
  `alpha-shell.shpkg` (`OUT_DIR`), asserts `Loaded::artifacts` carries `frames/MAP-0001.frame`, and
  mounts it through `Mount::new`, proving the shipped package grades its frame without reading the
  `display-packages/` folder at runtime.
- `shell::mount::tests::a_shipped_cell_frame_grades_and_mounts_from_the_package_alone` and
  `a_package_whose_frame_names_an_undeclared_window_is_refused_at_mount` — **1 + 1 passed**. The
  first compiles the folder with `recipe::artifacts_in`, loads the frame and mounts it from
  `Loaded::artifacts`, then places it in the map window's content space; the second proves an
  ungraded frame refuses the mount.
- `voxel-axis-verify --lib` — **19 passed, 0 failed**, including the format cell's six measured rows,
  `the_format_grade_falls_when_a_measured_row_does_not_light` (the same frame, a census table that
  does not name it, `grade C` at ★5) and
  `a_refused_format_frame_declines_instead_of_earning_a_star` (a record with none of the four
  sections declines the format rather than filing a star).
- `voxel-axis-mcp` — **18 passed, 0 failed**. Its three count assertions were still claiming one
  cell per segment; the format's own cell is the eighth, so they now read `Segment::ALL.len() + 1`
  rather than passing a count the report no longer produces.
- `voxel-package --lib cellframe::` — **13 passed, 0 failed**; `layer::` — **11 passed** (the layer
  grammar was not changed by this record). The cellframe suite now includes the artifact-map read,
  the missing-record refusal and the undeclared-window refusal.
- `voxel-client --lib renderer::tests -- --test-threads=1` — **22 passed, 0 failed** (86.08 s). The
  earlier parallel reading reported six failures once and a hang once; this pass did not re-run the
  parallel harness, so `tests-2026-10-24.txt` records the serial reading and reports the instability
  as un-reproduced rather than as a pass.
- `voxel-axis-verify` — **16 passed, 0 failed**, eight cells all ★5, the format's at `grade B` from
  six measured rows (the five format questions plus the census row that names it).
- `census` — **16 folders, 0 unclassified, 1 format row** naming `shploingus.cellframe/1`;
  `missing-docs` — **23 crates, 0 stale pins, 2 342 undocumented items** after the re-pin this pass
  records. The five moved pins are listed in the new baseline's `readings-2026-10-24.txt`.
- `voxel-layers display-packages` — **89 lines / `3f7c9230…` → 180 lines / `13d93263…`**,
  byte-identical across two runs. The digest did **not** move when the frame moved into the package:
  the emit reads the display package's folder, whose bytes are unchanged. The compiled
  `alpha-shell.shpkg` is 18 352 bytes with magic `SHPKG003`.
- `cargo check --workspace --all-targets` —  exit 0 (5 m 27 s on the final run, including the gate); `cargo build -p
  shploingus` — exit 0. No commit, no staging.

# Rollback

Delete `display-packages/alpha-shell/frames/` (both files) and the `row MAP-0001` line in
`display-packages/alpha-shell/package.shspec` and its copy in `packages/alpha-shell.shspec`; revert
PATCH-075's five edits and this record's `cellframe` wiring. A package with no frame declares none,
the mount registers none, and every window draws composed elements as it did before the class
existed. `conformance/missing-docs.txt`'s five re-pinned crates stay re-pinned — their bytes are the
same bytes either way — and the baseline folder stays as the record of the re-pin.
