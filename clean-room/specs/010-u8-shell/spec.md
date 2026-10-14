# Feature Specification: U8 — The game binary and the shell UI (the window, the mount, the design system)

> **Derived from documents** (Constitution P1). This file was created from the record rather than from
> memory: `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §2 (U8) and §5 row 7, `docs/SHELL-PORT.md` §1, §4 and §6
> (the machinery's shape, the four-bucket rule, and the rulings D-1/D-2/D-3), `docs/UI-SPEC.md` §1–§5,
> §8 and §9, `docs/DESIGN-SPEC.md` §3 and §5, and the design-doctrine rulings `docs/UI-SPEC.md` §8 quotes
> from `docs/LEDGER.md` (C-35, C-37, C-55 §55.1).
>
> **The bar this feature must reach**, in the plan's own words (§5 row 7): *a **visibly** drawn frame;
> pointer ownership read once.* The plan's own interface sentence (§2, U8) is the same bar stated as
> obligations: *a `Mount`-shaped pane tree driven by a declared `Recipe` (not a hard-coded app registry),
> the two layers' toggle, one pointer-ownership decision read once and applied to both grab and
> visibility, and engine constants sourced from an admitted table rather than literals.*
>
> **Measured, this pass** — see `plan.md` and `quickstart.md` for the commands: a **100 × 30** frame is
> drawn with a digest of `375c5122…` (**§40, 2026-10-13:** `07adeda4…` was §39's, `79f3885c…` before the
> mesh moved), **6**
> declared windows and **6** panes, the pointer decision read
> once and passed by value into both consumers, and **16** admitted constants across the four buckets
> (md1-rule 8 · md1-example 2 · platform-translation 1 · project-extension 5).

## What U8 is, and what it is not

U8 is **the only window** in this repository: the shell. It is the layer that **mounts declared windows**
and **draws them**, so it reads the world the systems above it built.

* **It is** the `Mount`-shaped pane tree (split, close, focus, collapse), driven by a declared `Recipe`
  of `WindowSpec` data; the **two layers of one window set** (shell and HUD, toggled on `Escape`/`F1`);
  the **one pointer-ownership decision**, read once and applied to both the grab and the glyph's
  visibility; the **four-bucket admitted constants table** every drawn value comes from; and a `Canvas`
  that draws a frame a person can look at and interact with through the binary's `shell` mode.
* **It is not a second window registry.** The recipe is the only window list; `Mount::of` mounts what it
  was handed and adds nothing. `docs/UI-SPEC.md` §1's rule — *windows are data the shell mounts* — is
  what the type shape enforces.
* **It is not a second pane tree.** `docs/SHELL-PORT.md` §4.2 warns that a second tree is *the defect*;
  this module **is** the clean room's tree, and there is no other.
* **It is not a world renderer.** `docs/SHELL-PORT.md` §6 **D-3**, ruled: *"the desktop background over
  the game would be dumb"* — neither the wallpaper nor an icon list is ported, the rendered world is the
  background, and this frame draws chrome and readouts and **names** the world rather than faking one.
* **It is not `lintapple`.** See the next section.

## The `gholtzap/lintapple` gate — who takes it, stated plainly

The mission's own bar is *"a more complete shell that also uses `gholtzap/lintapple`"*. The record
assigns that to a **different slice**, and this specification says so rather than deferring it silently:

* `clean-room/specs/001-presentation/plan.md` §3 carries the row **`P8 … WIP | NEEDS WORK`** — the
  constitution row for the linter; `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §5 row **13** defines **U13** as
  *"the shell completed, and its gholtzap/lintapple gate"*, whose acceptance check is *a visibly drawn
  frame (U1's band count and a screenshot) with every constant admitted, and the **P8** gate passing
  rather than `WIP`*.
* `clean-room/src/systems.rs`'s own U13 entry says it *waits on U1's remainder, U6's admitted constants
  table, U8, and the P8 gate*.

So: **U13 takes the lintapple gate; U8 is a prerequisite of U13 and does not take it.** The reason is
structural, not an excuse — U13 is defined as *the shell **completed*** and is *last by §2's own
reasoning* (*"completed first, it would be judged against nothing"*). Taking the P8 gate here would
either duplicate U13's bar or move U13's definition. What U8 **does** bring that the gate needs is the
half the gate is checked against: **every constant admitted** (§8's four buckets, refused by name
otherwise), which is why the constants table is in this slice and the gate is not.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A person runs the product and sees the world through it (Priority: P1)

A person runs `cargo run --offline -- shell`. The frame is drawn: the bar names the layer, the pointer's
owner and the frame's own geometry derivation; six windows sit in the pane tree, each bordered, each
titled with its lane, and each **reading a system that already exists** — the unified-systems registry,
the reference world's identity and its two digests, the material registry, the stage roster, a derived
chunk's geometry (`11054 cells swept → 3703 faces → 406 quads`, merge factor `9.12` — §39) and the
light table's solar constant
(`1361.000 W/m²`). They type `layer` and the HUD layer draws only the two windows the recipe declares
`pinnable`, because the shell and the HUD are **two layers of one window set**, not two window systems.

### User Story 2 - The pointer's owner is decided once and cannot disagree with itself (Priority: P1)

The pointer decision is read once per frame and **passed by value** into the drawing and the grab. A
frame therefore cannot draw a cursor the shell does not own, nor own one it does not draw: the two
consumers receive the same value because the signature leaves them no way to read it again. Run
`grab off` and the glyph leaves the frame with the grab; toggle to the HUD and the pointer returns to
the world.

### User Story 3 - Every drawn value is admitted, and an unadmitted one is refused (Priority: P1)

Every number the frame draws — the bar's height, the pane floor, the cell's assumed pixel size — is read
out of the constants table, and the bar **prints its own derivation** (`bar 6 cells (48 dp ÷ 8 px)`), so
a reader can check it. A name in none of the four buckets is refused by name
(`constant_unadmitted`) rather than drawn. The extension count — the bucket that can grow silently — is
pinned by a test.

### Edge cases

* A **layout** that would leave a pane below the declared **40 × 40 floor** is refused by name
  (`split_floor_breached`), never clamped: the refusal is `Mount::layout`'s, at the frame where the area
  is known — `split` itself accepts the operation, because the mount does not know the frame's area.
* A pane or window the recipe never declared is refused by name (`pane_unknown`), because a pane is
  addressed by its **own identity**, never by a list position.
* An unknown shell command is refused by name (`tool_undeclared`) and printed where the refusal happened.
* A recipe with no window mounts an empty desktop rather than a fabricated pane.

## Requirements *(mandatory)*

* **FR-001** `shell::Mount::of(&Recipe)` mounts a declared recipe; the mount knows **only** the recipe's
  windows. `Recipe::declared()` is this build's declaration — six windows, one per reader.
* **FR-002** The pane tree is `Node::{Pane, Split{orientation, a, b}}` over `PaneId`s; `split` inserts a
  second pane of the same window **beside** its target, `close` collapses a splitter left with one child,
  and a split adds **one** pane rather than a list entry.
* **FR-003** The **two layers** are `Layer::{Shell, Hud}`; `toggle_layer` moves between them, the HUD
  drawing the windows declared `pinnable`. A text frame cannot colour the layers, so the bar names both.
* **FR-004** **One** pointer-ownership decision: `Mount::read_pointer() -> Pointer{owner, visible}`; the
  value is passed **by value** into `shell::frame` and the event handling, so neither can recompute it.
  `visible` is a consequence of the same read.
* **FR-005** `CONSTANTS` is the admitted table; `constant(name)` returns the value or refuses
  `constant_unadmitted`; `bucket_counts()` counts the four buckets and the extension count is pinned.
* **FR-006** `shell::frame(&Mount, Area, Pointer) -> Result<String, Refusal>` draws a rectangular frame
  with box-drawing borders, a bar, a titled pane per visible window, and the pointer line when the glyph
  is visible; a pane below the floor refuses `split_floor_breached`.
* **FR-007** Each window's `Content` reads a system already built and returns its lines, or the system's
  refusal **by name** — never a fabricated value.
* **FR-008** The binary's `shell` mode draws the frame and reads one command per line
  (`split/close/focus/layer/grab/quit`), redrawing after each, so the bar's check is a thing a person
  runs.

## Success Criteria *(mandatory)*

* **SC-001** `cargo run --offline` exits 0 and the U8 evidence line reports the frame, its digest and the
  four bucket counts.
* **SC-002** `cargo run --offline -- shell` draws the frame and answers commands, refusing an unknown
  command, pane and orientation **by name**.
* **SC-003** `cargo test --offline` passes with the eleven `shell` unit tests among them.
* **SC-004** The constants table's four bucket counts and the refusal vocabulary's count are both pinned;
  the closed refusal round-trip test covers the three U8 kinds.
