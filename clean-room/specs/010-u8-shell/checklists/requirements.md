# Requirements checklist: U8 — The shell (the window, the mount, the design system)

Read against `spec.md`'s FR-001…FR-008 and this pass's measurements (`quickstart.md`). Every box was
checked against a number in the binary's own output or a test that holds it, not against an intention.

## The design system

- [x] **FR-005** `CONSTANTS` holds **16** rows, each with a `source`; `constant(name)` refuses
  `constant_unadmitted` by name; `bucket_counts()` reads `[8, 2, 1, 5]` and the extension count is
  pinned — `the_constants_are_bucketed_and_the_extension_count_is_pinned`,
  `an_unadmitted_constant_is_refused_by_name`.
- [x] **FR-006** `frame` reads its own geometry from the table (`bar 6 cells (48 dp ÷ 8 px) floor 5`) and
  refuses `split_floor_breached` below the pane floor —
  `the_frame_geometry_comes_from_the_table_and_not_from_a_literal`,
  `a_split_below_the_declared_floor_is_refused_by_name`.

## The declaration

- [x] **FR-001** `Recipe::declared()` declares **six** windows; `Mount::of` mounts exactly those and adds
  none — `the_recipe_is_the_only_window_list`.
- [x] **FR-007** Each `Content` reads a built system and returns its lines; a system's refusal is printed
  **by name** in the pane — `the_frame_is_visibly_drawn_and_shows_real_system_state` (which asserts the
  world's own seed, its truth digest's head, the chunk's geometry pane lines — `11054` swept cells and
  `3703` faces → `406` quads (§39; **§40, 2026-10-13:** `1289` faces → `313` quads, because the reading now
  reads the chunk's own ring; the test compares against the pane's own lines rather than these literals),
  the material count and
  `1361.000 W/m²`).

## The pane tree and the layers

- [x] **FR-002** A split puts a **second pane of the same window** beside its target; close collapses what
  the split added; an unknown pane refuses `pane_unknown` —
  `a_split_puts_a_pane_beside_its_target_and_close_collapses_it`, `an_unknown_pane_is_refused_by_name`.
- [x] **FR-003** The two layers are one window set: the shell layer draws all six panes, the HUD layer
  draws the two `pinnable` windows, and the toggle is a move rather than a rebuild —
  `the_layers_toggle_moves_the_pinnable_windows_and_nothing_else`.

## The pointer

- [x] **FR-004** The ownership is **one** value from `read_pointer`, passed **by value** into the drawing
  and the grab; `visible == (owner == Shell)` and a second read of the same state is equal; the drawn
  frame's glyph line tracks that same value —
  `the_pointer_decision_is_one_read_that_governs_both_the_grab_and_the_glyph`.
- [x] **FR-008** The `shell` mode draws the frame and answers `split`/`close`/`focus`/`layer`/`grab`/`quit`,
  refusing an unknown command (`tool_undeclared`), pane (`pane_unknown`) and orientation
  (`argument_malformed`) by name — the pasted session in `quickstart.md`.

## The standing rules

- [x] **P1** Every module header names the document it was derived from, and every constant row names its
  source; the record's own words are quoted rather than paraphrased.
- [x] **P2** The refusal vocabulary is closed and counted (**80**, up from 77, pinned in
  `tests/u5_measure.rs`); a value in no bucket is refused, never drawn.
- [x] **P3** One owner per concern: one window list (the recipe), one pane tree, one pointer decision, one
  constants table, one frame function, one reader per content kind.
- [x] **P7** No `use` of, and no `path` to, any `crates/*` item; no dependency added (`sha2` is still the
  only one); no legacy file modified.

## The suite

- [x] **Gates** `cargo test --offline` **334 passed / 0 failed** (274 library, 60 across nine suites);
  `cargo clippy --offline --all-targets -- -D warnings` exit 0; `cargo run --offline` exit 0 at **467**
  lines of stdout.
- [x] **Guards proved live** an unadmitted constant name, an unknown pane, a floor breach and an unknown
  command each refused **by name** in the running binary, each restored **byte-exact** by sha256.

## Owed, and not ticked

- [ ] **Undock** (`docs/UI-SPEC.md` §3) and the **splitter-drag resize** — named in the module header and
  `research.md` §7.
- [ ] The **console's five graded levels** and the **toast's final form** (`docs/SHELL-PORT.md` §9).
- [ ] The **`gholtzap/lintapple` P8 gate** — **U13's**, per `spec.md`'s own section; this plan's P8 row
  stays `WIP | NEEDS WORK`.
- [ ] The **four U10 rulings**, **`mc-ar-mod`**, the **`noul`/`score` calibration sets** and the **score
  instrument's rungs** — untouched, and owed exactly as they were.
