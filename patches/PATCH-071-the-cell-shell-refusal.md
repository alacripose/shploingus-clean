---
patch_id: PATCH-071
title: The cell shell — the schema's rendering features refused, and the 8 × 16 raster that stands in for them
status: applied
operation: report
target: shploingus/shell
base_ref: shploingus-clean@main
base_digest: sha256:69b3490839fdfa3708657d8d298b321a4c1c01f304cfce13afecd2ae96189953
dependencies: []
source: user-directed-conversation
created_at: 2026-10-23
approval: "user-directed 2026-10-23 — the cell shell is a substitution, and it is recorded as a refusal rather than left as an unstated state"
live_mutation: false
---

# Change

**This record changes no source and no test.** `operation: report`, `status: applied` — the applied
change is the report itself. What it files is the refusal PATCH-068 (line 83) and PATCH-069 (line 50)
already name as *"the cell-shell refusal"*, and which until now had no record to point at.

## The substitution, and the one line that makes it

**The shell no longer draws the elements it composes, as it composed them.**
`crates/shploingus/src/main.rs:3160` is the whole act:

```rust
let elements = shell_cells::surface(&elements, viewport_dp);
```

It sits between `shell_hud::frame` (`:3129`) and `graphics.hud.prepare` (`:3194`). Every element of
the interface — the bar, window bodies, title bars, pane chrome, readouts, the console's rows, the API
window's lines and the debug lanes — is composed first, then replaced by a raster of **8 × 16 dp**
cells (`crates/voxel-client/src/shell/cells.rs`: `CELL_W_DP` 8, `CELL_H_DP` 16, `surface` 391 →
`Grid::of` 252 → `Grid::elements`). Every paint in it is retinted to the nearest of the twelve `HOLO`
values (`retint`, 103), measured in squared bytes.

**The module is new and untracked at `aef0971`** (`git ls-files` does not know it), so the file this
record is read against is the draw path it was inserted into: `crates/shploingus/src/main.rs` at
`HEAD`, `sha256:69b34908…` — the same base PATCH-063 declares.

## What is lost on screen, measured

| the element as composed | what the raster draws |
|---|---|
| `size_dp`: body 14 sp, a title 20 sp (`TEXT_SIZE_DP`, `TITLE_SIZE_DP`, used by `titlebar.rs`, `pane.rs`, `placeholder.rs`) | one glyph per 8 dp cell at `CELL_TYPE_DP` **12.8** sp, monospace: the title/body step is **gone**, and a title draws smaller than it was declared |
| a run wider than its rect | elided to `…` (`fit_to`, 419): the declared rect wins over the string |
| a fractional `rect` | cell bounds, `floor` left/top and `ceil` right/bottom (475): a surface grows to whole cells, an inset or gap thinner than a cell closes |
| `Element::Rule`, 1 dp at its edge | one `─` row in the nearest clear row within one cell, **or dropped** (`draw_rule`, 442) |
| the role's paint, and its elevation tint | the nearest of twelve values; **alpha is kept, colour is not** |

**Three of eight roles are admitted, and two of them become one colour.** `Palette::declared()` states
`Surface` `#1B1922`, `Chrome` `#1A1A1A` and `SurfaceAlt` `#332F45`; the other five roles are
unadmitted and refuse by name before the raster sees them. Retinting those three gives `#1A1A1A`,
`#1A1A1A`, `#333333` — so a window's body and a console's toolbar rows, drawn with the same `opacity`
(`mount.rs:703`, `:805`), are **the same paint**, and the alternative surface is drawn as a divider.

**The elevation ladder is five levels and the screen has two values.** `ELEV_STEP` is 0.05 and
`MAX_ELEVATION` is 4; `chrome.elevated(0..4)` reaches `#1A1A1A`, `#252525`, `#313131`, `#3C3C3C`,
`#484848` and retints to `#1A1A1A`, `#1A1A1A`, `#333333`, `#333333`, `#333333`. `voxel_ui::appearance`
makes hover one step and pressed two (`voxel-ui/src/lib.rs:1246`), so **hover is invisible** on a
chrome surface and pressed is the divider grey; the toast at `max_elevation()` (`mount.rs:1297`) draws
the same colour as a pressed row. Only the alpha ramp survives, because `retint` keeps alpha.

**The schema's states and tokens are not drawn.** `docs/android-4-holo-theme-schema.md` §6 names eight
states and §5.2 requires normal, pressed, focused and disabled on every action; §2.1's link inks
`#FF5C5CFF` and `#FF0000EE` are not among the twelve. Focused, selected, checked, error and loading
have no distinct paint at all — the **ink** half of `appearance` survives (white at the ladder's four
alphas), the **tint** half does not.

## Why refused rather than done

1. **A cell is a second layout, and the pointer is not on it.** The module says so itself: *"a cell is
   a second layout: it moves a glyph the views sized and a rule the chrome placed."* The shell's hit
   tests resolve against the declared rectangles (`shell/window.rs`, `mount.rs`), so drawing the
   schema's controls — its `layout.actionBar.height` of 48 dp (§4), state drawables, field outlines,
   9-patch frames — inside the grid would put controls where the verdicts do not look: the defect
   class `docs/SHELL-PORT.md` §5 exists to prevent.
2. **Twelve values is the closure, and it is checkable.** `every_paint_lands_on_the_schema_and_keeps_its_alpha`
   is a law only because the set is closed. Admitting §2.1's remaining tokens turns the set into a
   twenty-value vocabulary with near neighbours in it (`#33B5E5` and `#5C5CFF`), so the shell's colours
   become a choice *within* the schema rather than the schema — a second palette beside
   `voxel_ui::Palette`, which already refuses unadmitted roles by name.
3. **The features are the renderer's, not the shell's.** A state drawable, an anti-aliased 1 dp rule, a
   9-patch frame, a density icon and §6's motion need a textured element pass in the HUD's renderer; the
   shell composes rects, rules and text for one `prepare` call. Building them in the shell would be a
   second renderer, which is the same defect as a second layout.
4. **No record rules the schema in as a rendering target.** No `docs/LEDGER.md` entry names Android 4
   or Holo (both "android" hits are the user's toast sentence quoted in C-110), and
   `Palette::declared()` admits three roles. The substitution is what the shell can state honestly, and
   this record is where it is stated — not a feature set quietly re-derived under the name of a design
   reference.

# Acceptance

- The raster is the only draw path: `grep -rn "cells::surface" crates` → `shploingus/src/main.rs:3160`
  and `examples/shell-shot.rs:91,173`; `grep -rn "cells::retinted" crates` → the example only
  (`:132,175`), so the schema reaches the client through `surface` alone.
- The module's witnesses pass: `cargo test --offline -p voxel-client --lib shell::cells` → **9 passed**.
- The record parses as a record: frontmatter complete, the id in its filename, the four sections present,
  every dependency resolved (none declared).
- **What would falsify this record:** an `Element` carrying a schema token outside the twelve, a
  pressed/focused/selected drawable, or a rule thinner than a cell reaching `prepare` from above the
  raster.

# Evidence

The retint table is the module's own arithmetic applied to the values the tree declares, not a second
model of it: `retint` (cells.rs:103) iterates `HOLO` and keeps the smallest `dr² + dg² + db²`.

```
Palette::declared()      retint
Surface   #1B1922   ->   #1A1A1A   (66)
Chrome    #1A1A1A   ->   #1A1A1A   (0)
SurfaceAlt #332F45  ->   #333333   (340)

chrome.elevated(0)  #1A1A1A -> #1A1A1A
chrome.elevated(1)  #252525 -> #1A1A1A
chrome.elevated(2)  #313131 -> #333333
chrome.elevated(3)  #3C3C3C -> #333333
chrome.elevated(4)  #484848 -> #333333
```

```
$ cargo test --offline -p voxel-client --lib shell::cells
test result: ok. 9 passed; 0 failed
```

# What this record does not touch

The gate, the walk, the twilight exposure (PATCH-067, PATCH-068), the shadow lattice and the shadow
walk (PATCH-069, PATCH-070), the tree mob (PATCH-043), and every assertion, test and source line: this
record is prose.

**Two sentences in the module's header are now false, and are left for their own patch.** It says *"The
shell's own draw path does not cross it"* (lines 12–15), which `main.rs:3160` contradicts; it also says
the schema is *"eleven values"* while `HOLO` is `[Rgb; 12]`. Editing a module's own docs to match its
call site and its constant is a code change with its own head, so this record is the note that it is
owed, not the edit.

# Rollback

Delete this record. Nothing in the tree, the gate, the walk or the tests changes with it. If the refusal
is later ruled the other way, the substitution is removed by deleting `main.rs:3160` — a patch with its
own head, its own acceptance and its own rollback.
