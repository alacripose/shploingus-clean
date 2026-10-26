# PATCH-046 — the cells and the Android 4 schema

The user's sentence, 2026-10-21, is this pass's specification:

> *"I want all shell interface elements to use the package designer shell cells while themed to the
> android 4 schema"*

The reference is `ala_package_designer`'s shell — `crates/foundry-shell/shell/`, a Go program that
draws its floating windows as an **8 × 16 cell surface** between the action bar and the system bar.
Its `place.go` header states the division this patch keeps: *"One function decides it and nothing
here draws."* Placement is a package fact; paint is a raster.

# What changed

| file | the change |
|---|---|
| `crates/voxel-client/src/shell/cells.rs` | **new**: the cell grid, the raster, the emitter, and the schema's twelve colours |
| `crates/voxel-client/src/shell/mod.rs` | the module is declared, so the draw boundary and its tests share one path |
| `crates/shploingus/src/main.rs` | one call: `shell_cells::surface(&elements, viewport_dp)` at the element boundary |
| `crates/voxel-client/src/hud.rs` | glyphs are shaped in **Monospace**, one family named at the one place text is shaped |

# The raster, in one paragraph

`Grid::of(elements, cols, rows)` walks the element list and writes cells: a **surface** fills every
cell its rect touches with its paper; a **rule** writes `─` across the cells of its first row,
because a rule is a horizontal line and a cell cannot be thinner than its row; **text** starts at
the cell its alignment and left edge snap to and is clipped to the element's own cell width, not
only to the grid. `Grid::elements(origin)` reads the grid back as runs: paper runs become
`Element::Surface`, glyph runs become `Element::Text` at 12.8 sp. The renderer never learns that a
grid existed — the list it receives is the same kind it received before.

**Nothing grew.** Out-of-range writes are dropped. A readout longer than its window is cut, not
laid out past the pane; the test `a_text_run_is_clipped_to_its_own_rect_not_only_the_grid` is the
witness, and it exists because the first version clipped at the grid edge and let a value column
paint through the column beside it.

# The schema

**Android 4 Holo Dark**, declared as twelve values and nothing else:

| value | hex | where it lands |
|---|---|---|
| background | `#000000` | the schema's black |
| surface | `#1A1A1A` | a window body or panel |
| chrome | `#0D0D0D` | the bar and title chrome |
| blue | `#33B5E5` | the accent, and the selected step |
| blue dark | `#0099CC` | the accent's pressed step |
| text | `#FFFFFF` | primary ink |
| text dim | `#B3B3B3` | secondary ink |
| text disabled | `#808080` | disabled ink |
| divider | `#333333` | rules and separators |
| red | `#FF4444` | status |
| green | `#99CC00` | status |
| orange | `#FFBB33` | status |

Every paint that reaches the raster is **retinted to the nearest value**, distance measured in the
bytes the paint carries. Near-blues become Holo blue, near-greys become its divider or its chrome,
and a red level badge stays red rather than being flattened into the ink ladder. A role is still a
place (`surface`, `hud`, `quiet`); the schema is what this build answers those places with.

# Why the choice is at the boundary and not in the palette

`voxel-ui`'s `Palette` is the design record's eight roles and five of them are deliberately
unadmitted. The shell's paint already resolves roles through `shell/theme.rs` and the HUD's ink
ladder; changing those would be a second theme system. The raster is applied **after** composition
and **before** the HUD's prepare, so the two decisions a paint can still make — which cell, which
schema colour — are made once for the whole interface, and the pointer's hit-testing keeps the
declared rectangles it was given. A paint does not move a window.

# What is not changed, and why

* **Placement, drag, resize and z-order.** The designer shell's `place.go` is a placement algorithm,
  and this tree already has one (`mount.rs`, `window.rs`, `desktop.rs`). Importing a second would be
  two window systems, the failure UI-SPEC §1 names.
* **The shell package.** Windows are still the declared `alpha-shell.shspec` rows; the cells do not
  invent geometry.
* **The dead modules the balance census names.** `desktop` and `taskbar` remain unrendered and
  counted by `PATCH-045`; painting the live elements is not a claim that the dead ones are wired.

# Verification

* `cargo test -p voxel-client --lib shell::cells` — six tests: the schema maps to itself, a surface
  and a rule become whole cells, text is one glyph per cell and clipped by both its rect and the
  grid, an empty grid draws nothing, the same elements produce the same cells, and a paint keeps its
  alpha through the retint.
* `cargo test -p voxel-client --lib shell::` — 245 tests, unchanged.
* `cargo check -p shploingus` — the bin builds with the boundary call in place.
* The missing-docs pins for `voxel-client` and `shploingus` are re-taken; the counts did not move
  (191 and 0), because every item this patch adds is documented.

# Superseded in part, the same day: the raster is not the draw

The user's sentence on seeing it run was *"you replaced window drawing itself with the cells when it's
supposed to be the windows content views for wherever it shows glyphs"*, and the complaint is right
at the glyph: `Grid::of` re-places every run on the 8 × 16 lattice, so a title bar's rule became a row
of `─` cells and a readout's value moved to the cell its rect rounded to. That is a second layout, and
it overwrote the one the views chose. The screenshots made it plain — dotted rules where the chrome
drew lines, glyphs at a cell's advance rather than the view's type size.

**What the draw path asks of this module now is `retinted`**: the schema, and only the schema. Every
element keeps the rect the view gave it. `Grid`, `surface` and their tests stay — the raster is a
capability this module still owns and still proves — but nothing in the client draws through it, so a
later pass may delete it outright. The renderer lost the one thing it had taken on for the grid's
sake, the measured monospace advance that solved a run's type size to one cell; the family stayed
monospace, because the shell's rows are columns and a fixed advance is what holds them.

Measured after the change, on the offscreen path (`examples/shell-shot.rs`, three views): compose
0.04–0.07 ms a frame including the retint, against 0.17 ms with the grid. `cargo test -p
voxel-client --lib` — 503 passed, 2 ignored.

## Recipes

```recipe
segment: shell
axis: evidence_grade
ticket: T-0954
site: crates/voxel-client/src/shell/cells.rs:123
what: keep `retinted` the only schema step on the draw path, with the grid kept and proved but unwired
because: the grid re-placed every run on its 8 x 16 lattice and overwrote the views' own layout; the retint alone composes at 0.04-0.07 ms a frame against 0.17 ms with the grid
check: cargo test -p voxel-client --lib shell::cells -> failures 0
witness: crates/voxel-client/src/shell/cells.rs
expects: 0
requires: cargo check -p shploingus
```
