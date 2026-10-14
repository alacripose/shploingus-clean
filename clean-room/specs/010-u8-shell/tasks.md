# Tasks: U8 — The shell (the window, the mount, the design system)

**Input**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md) | **Record**: [quickstart.md](./quickstart.md)
and `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §27 · `docs/LEDGER.md` **C-188**.

**All tasks ran and are done** in one pass (2026-10-12, sixteenth pass). Where a first assumption was
refuted by a measurement, the correction is recorded under the task rather than smoothed away.

- [x] **T001** The design system, in a new module: `Bucket` (four), `Constant` with its **source**,
  `CONSTANTS` (16 rows), `constant(name)` refusing `constant_unadmitted`, and `bucket_counts()`. *Landed
  in `src/shell.rs`*, 1406 lines.
- [x] **T002** The declaration: `Lane` (six hues), `Content` (six readers), `WindowId` (a stable
  identity), `WindowSpec`, and `Recipe::declared()` — the six-window declaration, one per built system.
- [x] **T003** The mount: `PaneId`, `Orientation`, `Node`, `Mount::of` and the pane grid, `split`,
  `close`, `focus_window`, `panes`, `window_of`/`spec_of`, and the pane→window map that makes a split a
  second pane **of the same window**.
- [x] **T004** The two layers: `Layer::{Shell, Hud}`, `toggle_layer`, `visible_panes` — one window set,
  the HUD drawing the `pinnable` windows.
- [x] **T005** The one pointer decision: `Pointer`, `PointerOwner`, `read_pointer` — a value handed **by
  value** to the drawing and the grab, with `visible` computed inside the same read.
- [x] **T006** The frame: `Area`, `Canvas` with box-drawing borders, `frame(&Mount, Area, Pointer)`,
  `declared_area()` (100 × 30), `frame_digest`, and the floor refusal in `Mount::layout`.
- [x] **T007** The six readers: the registry, the reference world's identity and digests, the material
  registry, the stage roster, a derived chunk's geometry and the light table — each a call into a system
  already built, each returning that system's refusal **by name** rather than a substituted value.
- [x] **T008** The refusals, in `src/refusal.rs`'s closed vocabulary: three new kinds
  (`constant_unadmitted`, `pane_unknown`, `split_floor_breached`), with `ALL`, `COUNT` and `name()` moved
  together. **Cascaded**: `tests/u5_measure.rs`'s pinned count moved **77 → 80**, by name.
- [x] **T009** `src/evidence.rs`'s `check_u8` and `src/main.rs`'s gate, so the bar is measured by the same
  function the binary prints; `src/systems.rs` carries U8's real status, interface and witness.
- [x] **T010** The binary's **`shell` mode**: it draws the frame and reads one command per line
  (`split`/`close`/`focus`/`layer`/`grab`/`quit`), redrawing after each, refusing an unknown command,
  pane or orientation **by name** — so §5 row 7's check is a thing a person runs and interacts with.
- [x] **T011** Eleven unit tests in `src/shell.rs`: the buckets and the pinned extension count, the
  refused constant name, the table-derived geometry, the recipe as the only window list, the split/close
  pair, the unknown pane, the floor refusal, the one-read pointer, the visibly drawn frame with real
  state, the layer toggle, and the frame's purity.
- [x] **T012** Every guard proved **live**: the constant name, the pane, the floor, the unknown command,
  returned the named refusals in the running binary, each restored **byte-exact** and proved by sha256.
- [x] **T013** The cascade: `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §2 (U8's status) and §5 row 7 marked
  landed, §27 written; **C-188** filed in `docs/LEDGER.md`; the spec set created under
  `clean-room/specs/010-u8-shell/`; `clean-room/README.md` updated; the stdout line count moved **464 →
  467** — three lines added (U8's frame, digest and constants lines) and none removed.

## Findings this pass's own measurements produced

1. **The pane floor decides the initial tree.** A flat stack of the six declared windows leaves each pane
   three cells tall inside a 30-row frame — below the 40 dp floor at 8 px per cell — so `layout` refuses
   it. The correction is a **grid** (columns of three, side by side), which is the smallest change that
   makes the declared windows fit a normal frame. The measurement said so; the first shape was wrong.
2. **"Read once" is a signature, not a promise.** The decision is read into a `Pointer` and both consumers
   take it **by value**, so neither *can* read it again. The test asserts the glyph's presence in the
   drawn frame tracks that same value, which is what makes the property observable rather than asserted.
3. **The digest is the honest report line.** The frame is 30 lines of 100 characters; printing it inside
   the evidence report would bury the report. The evidence prints the frame's **digest** and the two
   booleans, and the `shell` mode is where the frame itself is seen — so a person sees it and a report can
   still say *which* frame was drawn.
4. **The geometry pane's numbers are the system's own.** `chunk (0,0,0): 1289 faces → 313 quads` — 1289
   faces over 11 054 swept cells, because the chunk is meshed over the world's own cells, solid *and* water,
   whose side faces are emitted where a neighbour is not solid, and the reading now reads the chunk's own
   ring (§39 had `3703 faces → 406 quads`; **§40, 2026-10-13** moved the pane's numbers when the extent
   contract landed). The pane prints what the mesher derived; nothing was rounded to look tidier.
