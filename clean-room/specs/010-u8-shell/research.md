# U8 research — the decisions, each taken from a document rather than from taste

> Every entry names the document that decided it. Where the record left something open, this slice says
> **owed** rather than filling it (P2).

## 1. The drawing target is a character grid

`docs/SHELL-PORT.md` §3 says the drawing target is `voxel-ui`'s `Element`/`Panel` renderer. That renderer
is legacy code this clean room may not import (P7), and adding a GPU/windowing dependency would break the
crate's offline, one-dependency discipline. The bar itself only asks for *"a **visibly** drawn frame"*, so
the frame is drawn into a character `Canvas` and printed: visible, interactive, dependency-free. The step
that makes this honest is `docs/UI-SPEC.md` §8's own rule — a value this build's target forces is a
**platform translation**, declared rather than implied — so the cell's assumed pixel size is a row in the
constants table (`cell_px_assumed`), and the bar prints `bar 6 cells (48 dp ÷ 8 px)`.

*Cost:* the frame cannot show real opacity or colour. *Recorded as owed:* nothing — text is
`docs/UI-SPEC.md` §9's own rule for every view (*"every view has a text form"*).

## 2. The two layers, and the defect they fix

`docs/UI-SPEC.md` §1 records the reference's own defect — *"shaper UI hidden when esc menu hidden or f1
toggled"* — and the fix: *"the shell and the HUD are two layers of one window set, not two window
systems"*. So `Layer` has exactly two values, `toggle_layer` moves between them, and the HUD draws the
windows declared `pinnable` rather than re-implementing them. HANDOFF §1 names the same toggle at the
same keys (`Escape`/`F1`), which the bar prints.

## 3. Pointer ownership, read once

`docs/CLEAN-ROOM-PLAN-2026-10-11.md` §2 (U8) requires *"one pointer-ownership decision read once and
applied to both grab and visibility"*. The decision is read into a `Pointer` value and **passed by
value** into the drawing and the event handling; the signature is the mechanism (a function that was
handed a value cannot read a second one), and `visible` is computed inside the same read. The same
document's §4 rule 4 (`C-108`) is the neighbouring constraint — *"one pointer feed"* — which this slice
does not violate because it opens no device at all: it consumes a decision.

## 4. The four buckets, and why the extension count is pinned

`docs/UI-SPEC.md` §8: *"Every constant carries its bucket … and the extensions are counted by a test so
the bucket that can grow silently is the one that cannot."* Hence `bucket_counts()` and the pinned
`[8, 2, 1, 5]`. The values themselves are the ones §8.1 already rules (ink ladder, contrast floor 4.5,
corner radius 2 dp, 48 dp target, 8 dp separation, panel opacity 0.93, `ELEV_STEP` 0.05 with ceiling 4)
plus the two `docs/SHELL-PORT.md` §6 D-1 landed (the 40/20 dense-pointer pair) and the two §3/§5 state
(the 40 × 40 pane floor, the 48 dp top bar).

## 5. D-3 — the desktop draws nothing

`docs/SHELL-PORT.md` §6 D-3, ruled by the user's own words: *"the desktop background over the game would
be dumb."* Neither the wallpaper nor `ExplorerView`'s icon list is ported; the rendered world is the
background. In this build there is no world *renderer*, so the frame draws chrome and **readouts** and
names the world (its seed, its truth and residency digests) rather than faking a picture. That is the
same ruling applied one level down, and it is why the closest thing to a "world view" here is the
`world` window's own digests.

## 6. The pane floor, refused rather than clamped

`docs/UI-SPEC.md` §3: *"40 × 40 floor on every splitter and pane"* — *"the reference's hard-won detail:
without it, nested splitters' `minimumSizeHint`s accumulate and the layout locks."* A floor that clamps
would draw a pane the person did not ask for, so `layout` refuses `split_floor_breached` with the numbers
that breached it. The floor is a table row (`pane_floor_dp`), so the refusal quotes the admitted value.

## 7. Owed, named, not silently missing

* **Undock** (`docs/UI-SPEC.md` §3: *"`undock(pane)` produces a new standalone window"*) — owed; the
  split/close pair is landed and the third operation is named here and in the module header.
* **Splitter-drag resize** (`docs/SHELL-PORT.md` §3 step 1's `window.rs` range) — owed; the layout halves
  the area per split, which is the port's default and not its dragged state.
* **The console's five graded levels** and **the toast's final form** (`docs/SHELL-PORT.md` §9) — owed.
* **The `gholtzap/lintapple` P8 gate** — **U13's**, and the spec says so in its own section.
