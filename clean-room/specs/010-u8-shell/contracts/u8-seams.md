# U8 seams — the contracts this slice holds, one owner each

> **Derived from** `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §2 (U8) and §5 row 7, `docs/SHELL-PORT.md` §4
> (the rules that bind every module) and `docs/UI-SPEC.md` §1–§4 and §8. Each contract names **one**
> owner; a second owner of any of them is the defect the contract exists to prevent.

## C1 — The recipe is the only window list

A window is a `WindowSpec`: `id` (a **stable identity**, never a list position), `title`, `lane` (one of
the six hues), `content` (a closed enum of readers) and `pinnable`. A `Recipe` is a list of them, and
`Mount::of(&Recipe)` mounts exactly that list: **it adds no window it was not handed**. The test
`the_recipe_is_the_only_window_list` asserts the mount's window set is the recipe's — a mount that
invented a window would fail it.

*Owner:* `shell::Recipe` / `shell::Mount::windows`. *Refuses:* nothing here; the concern is composition,
not validation — a `WindowSpec` is data whose `source` is the document that admits it.

## C2 — The pane tree, and how it changes

The tree is `Node::{Pane(PaneId), Split{orientation, a, b}}`.

* `split(pane, orientation)` inserts a **second pane of the same window** beside its target; the new pane
  takes focus; the pane count rises by exactly one.
* `close(pane)` removes a pane and **collapses** a splitter left with one child; a window whose last pane
  closes goes with it.
* A pane is addressed by its `PaneId`. An unknown pane refuses **`pane_unknown`** — never a silent index
  landing on a neighbour (`docs/UI-SPEC.md` §2, C-51 §51.3).

*Owner:* `shell::Mount::{split, close, focus_window, panes}`. *Tests:*
`a_split_puts_a_pane_beside_its_target_and_close_collapses_it`, `an_unknown_pane_is_refused_by_name`.

## C3 — The two layers are one window set

`Layer::{Shell, Hud}`; `toggle_layer` flips it. The shell layer draws every pane; the HUD layer draws the
windows whose `WindowSpec` declares `pinnable`. Toggling **moves** windows between the layers; it does not
build a second window set (`docs/UI-SPEC.md` §1, H-9.8). A text frame names both layers on the bar
(`docs/UI-SPEC.md` §9: every view has a text form).

*Owner:* `shell::Mount::{toggle_layer, visible_panes}`. *Test:*
`the_layers_toggle_moves_the_pinnable_windows_and_nothing_else`.

## C4 — One pointer-ownership decision, read once

`Mount::read_pointer() -> Pointer { owner, visible }` is the **only** place the ownership is decided:
the shell owns the pointer iff its layer is shown, a pane exists, and a grab is in effect; otherwise the
world does. The value is **passed by value** into `shell::frame` and into the event handling, so neither
consumer can recompute it — the signature, not a comment, is what makes "read once" true — and `visible`
is a consequence of the *same* read, so a frame cannot show a glyph it does not own or own one it does
not show.

*Owner:* `shell::Mount::read_pointer` / `shell::Pointer`. *Test:*
`the_pointer_decision_is_one_read_that_governs_both_the_grab_and_the_glyph`.

## C5 — Every drawn value is admitted, and an unadmitted one is refused

`CONSTANTS` is the table: `name`, `value`, `bucket`, `source`. The four buckets are
`md1-rule · md1-example · platform-translation · project-extension` (`docs/UI-SPEC.md` §8). `constant(name)`
is the **one** door: an unadmitted name refuses **`constant_unadmitted`**. `bucket_counts()` counts the
buckets, and the **extension** count is pinned by a test, because that is the bucket that can grow
silently. The frame's own geometry reads the table (`bar_cells`, `floor_cells`) and the bar prints the
derivation, so a literal cannot hide in the drawing.

*Owner:* `shell::{CONSTANTS, constant, bucket_counts}`. *Tests:*
`the_constants_are_bucketed_and_the_extension_count_is_pinned`,
`an_unadmitted_constant_is_refused_by_name`,
`the_frame_geometry_comes_from_the_table_and_not_from_a_literal`.

## C6 — The frame is drawn, and the floor is refused rather than clamped

`shell::frame(&Mount, Area, Pointer) -> Result<String, Refusal>` draws a rectangular canvas with
box-drawing borders, the bar, one titled pane per visible window, and the pointer line when the glyph is
visible. `Mount::layout` refuses **`split_floor_breached`** when a split would leave a visible pane below
the declared **40 × 40 floor** (`docs/UI-SPEC.md` §3) — refused, because a silently smaller pane is a
layout the person asked for and did not get. Each pane's body is its `Content`'s own read of a built
system, or that system's refusal **by name**.

*Owner:* `shell::{Canvas, frame}` / `shell::Mount::layout`. *Tests:*
`a_split_below_the_declared_floor_is_refused_by_name`, `the_frame_is_visibly_drawn_and_shows_real_system_state`.
