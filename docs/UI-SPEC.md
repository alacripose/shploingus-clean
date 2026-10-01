# UI-SPEC — the shell, its surfaces, and the design system

> **Status: outline, owed** (H-11.4 / LEDGER C-53.1). Assembled from the outline this project
> already took down — **not** from another tree's decision list. Every value that is not yet
> ruled carries **⚠️** and appears in §10.
>
> **Authority:** `LEDGER.md` (C-35, C-37, C-40, C-49, C-52, C-53), the user's answers in
> `HYPERGRILLING.md`, and **MD1** (a lineage the user selected, so its spec binds).
> **Reference only:** `portal_simulator.py`, `alacripose-landing`, `ala-cities`,
> `shploingus` (dirty), `shploingus-game` — read for *why* and for shape, never cited as a ruling.

---

## 1. What the shell is

**Windows are data the shell mounts** (H-9.7). There is exactly **one** kind of window.
The game's own surfaces — the ledger, the console, the world shaper panel, the debug lanes,
the API viewer — are all shell windows. Nothing else may draw a window.

The **HUD is the pinned subset** of the same system (H-9.8): a pinned window stays in the user's
view while the shell is hidden. So the shell and the HUD are two layers of one window set, not two
window systems.

**Why this is a rule and not a convenience.** The previous shell lost its own surfaces when the
shell was hidden (the reference's own recorded defect: *"shaper UI hidden when esc menu hidden or
f1 toggled"*). With pinning, a surface that must survive the shell being hidden is *declared*
pinnable rather than reimplemented in a second tree.

---

## 2. The declaration — the shape the shell API must have

A window is a **`WindowSpec`**: data, not a class with a draw method. The shell owns state.

| field | meaning | source |
|---|---|---|
| `id` | stable identity, **not a list position** — the reference shell addressed windows as `#3`, which stops meaning anything the moment one closes | C-51 §51.3's lesson, applied |
| `title` | the bar's text | H-9.8 |
| `lane` | the lane class a window reports through, one of the **six** hues (*Frame, Control, Tab, World, Beacon, Motion*) | H-7.25; the six is the ruling |
| `extent_unit` | **`SquarePixels`** or **`CubicWorldUnits`** — declared, not inferred; this is what lets a pinned window be a HUD element or a world-anchored one | ⚠️ H-9.8's *"depends on the declared unit"* |
| `content_kind` | a closed enum — `WorldView · Readout · Table · Log · Console · Ledger · ApiViewer · ShaperPanel · Mods · MapPlan · Tools · Inventory` | H-7.25/H-7.26 |
| `pinnable` | may it live in the HUD | H-9.8 |
| `default_geometry`, `min_size` | screen-space initial placement and the floor | reference: portal_simulator's `WindowGeometry` / 260×160 |
| `opacity`, `theme_role` | pinned windows keep opacity and theming | H-9.8 |
| `layout_policy`, `behavior_policy` | which chrome (min/max/close/title position) and which behaviours (drag/resize/max/min/close) the window is *allowed* | reference: portal_simulator, and it is the better half of its API |
| `record_id` | the window's own line in the one record | C-49 ruling 1 |

**Ownership is the load-bearing half:** the window declares; the shell owns `WindowState`
(normal/minimized/maximized/closed), geometry, focus and z-order. A window that draws its own
chrome is a window the shell cannot theme.

---

## 3. The pane tree — panes are slots, arrangements are trees

* A **pane** is a slot holding a **window id**. A window with no split is one pane.
* `split(pane, h|v)` inserts into an existing splitter when the orientation agrees, **wraps**
  when it opposes, and wraps the root when the target is the root.
* `undock(pane)` produces a new standalone window; `close(pane)` collapses one-child splitters
  back up; a window closes when its last pane does.
* **40 × 40 floor on every splitter and pane** and proportional stretch factors — the reference's
  hard-won detail: without it, nested splitters' `minimumSizeHint`s accumulate and the layout
  locks.
* **Free-floating windows are allowed** (H-7.24): a window may ignore the grid entirely.
* **The arrangement is data**, so it is **a loadable package** — the first thing the Mods window
  has to show. The shell holds only transient state.

---

## 4. Pinning

A pinned window **stays in the user's UI while the shell is hidden** (H-9.8), and keeps the
ported window structure: title bar, controls, split-tree tiling, **opacity** and **theming**.

Pinning is a **layer move**, not a second rendering path: the same window, hosted by the HUD layer
instead of the shell layer, with its declared `extent_unit` deciding whether it is screen-space
pixels or world-anchored cubic units. ⚠️ The exact layer semantics are H-9.8's open half.

---

## 5. The tab set — the bar

The bar carries the set the user named, plus the two that were *proposed and never added*
(H-7.25, H-7.26). **Built as declared rows, not hand-written code** — the reference's own console
help string already drifted from its dispatcher, which is why §7 derives its help.

| # | Tab | Holds | Source |
|---|---|---|---|
| 1 | `inventory / crafting` | dual pane | H-7.25 |
| 2 | `working trees / crafting trees` | dual pane; the recipe trees | H-7.25 |
| 3 | `tickets / governance` | the ledger's tickets, their expectations, their retirements; the observer | H-7.25 |
| 4 | `mods` | **the join point.** Packages, their manifests, digests, provenance, refusal on mismatch, and the arrangement package | H-7.25, H-8.5, H-7.19 |
| 5 | `docs` | the record, generated from the loaded surface | H-7.25 |
| 6 | `settings` | themes, opacity, motion, the shell's own knobs | H-7.25 |
| 7 | `tools` | proposed-but-never-added; the tool surface | H-7.26 |
| 8 | `map / plan` | proposed-but-never-added; the world map and the plan | H-7.26 |

**There is no Tests tab and no separate test surface** (event-0013's ruling, carried): a check is
visible by looking at the thing it checks, and **the lanes on every window are the test surface**.

---

## 6. Windows that open from the bar rather than living in a tab

| window | what it is | source |
|---|---|---|
| **console** | the game's own command surface: world and record commands, not only window management | H-8.2 |
| **log** | the graded event stream, `[SYS] [EVENT] [INFO] [WARN] [ACTION]`, colour-coded with timestamps; levels as lane classes, not as colours picked per message | reference: portal_simulator's console, which proved the levels |
| **ledger** | the one record, rendered — the same record `docs/LEDGER.md` projects | C-49 ruling 1 |
| **api viewer** | the machine-readable API with human-readable text surrounding it, bounded by the token budget, and **rendering whether a reading may be quoted** | reference: D76 / §8.22, read for shape |
| **notifications** | §7 | H-9.7 |

---

## 7. Notifications

**MD1's own rule binds** (MD1 is a selected lineage), and it settles the split the game needs:

> *"Transient result/undo → **Snackbar** — one at a time; not a blocking decision or persistent
> error."*

So:

* **A snackbar is transient, one at a time**, and carries an undo where an undo exists. It is
  *not* the surface for a blocking decision and *not* the surface for a persistent error.
* **Refusals, approvals and anything needing a human are persistent** — they live in the
  notifications window and in the ledger, not in a toast that scrolls away.
* **Both read the same records**, so a toast cannot say more than the record it came from.
* The linter's verdicts are records, and the panel is *fed by the chokepoint* rather than by
  whichever call site remembered to log (C-49, L-5's shape).

⚠️ The panel's grouping, its default pinning, and whether it is a window or a lane inside one
are **H-11.3**.

---

## 8. The design system — four buckets, counted by name

Every constant carries its bucket, exactly as `voxel-ui` already does (C-35): **MD1 rule ·
MD1 example · platform translation · project extension**, and the extensions are **counted by a
test** so the bucket that can grow silently is the one that cannot.

### 8.1 Already ruled and built

| layer | what exists | source |
|---|---|---|
| ink ladder | 100 / 70 / 50 % white steps plus a 12 % divider | MD1 rule |
| contrast floor | 4.5 : 1, measured with WCAG arithmetic on **pixels**, not asserted | MD1 rule + C-37 |
| panel opacity | **0.93**, *derived* as the least opacity keeping the ladder legible over white, pinned by a test | project extension, derived (C-35) |
| palette | **2 declared roles of 8**; a colour outside the palette is unrepresentable; an unadmitted role **refuses by name** | MD1 rule + C-35 |
| shape | 2 dp radius, computed as a rounded-rectangle distance field rather than asserted | MD1 rule + C-37 |
| depth | **raised = lighter**: `ELEV_STEP` 0.05 × level, levels **0..4**, refused above rather than clamped; the same tint is the interaction language (**Hover** 1 step, **Pressed** 2); **no shadow, no blur** | project extension (form from MD1) + C-55 §55.1 |
| 48 dp target floor, 8 dp separation, reduced-motion | enforced by tests, **drawn by nothing yet** | MD1 rule, C-35 |

### 8.2 Named, not yet ruled

| layer | the shape it needs | reference for the shape |
|---|---|---|
| elevation ladder | the ladder itself is **ruled** — `ELEV_STEP` × level **0..4**, above 4 refused (C-55 §55.1). Open: **which surface takes which level**, a *declaration in the arrangement package* rather than values | C-55 §55.2 + H-13.2 |
| type roles | ⚠️ three roles with a stated job — body, label (uppercase, tracked), display — and a glyph set; **roles, not families** | `alacripose-landing` |
| night/day surfaces | ⚠️ a panel whose **opacity and line colour are declared against a named backdrop**, because §4's sky is the backdrop that will vary | `alacripose-landing` |
| motion | ⚠️ one duration and one curve per class of movement, plus a hard `prefers-reduced-motion` kill switch | `alacripose-landing` |
| keyboard states | ⚠️ focus-visible, skip-link, sr-only, selection — the states a keyboard user actually touches | `alacripose-landing` |

**Nothing in §8.2 is adopted by being pretty.** A value becomes a row only by being either an
**MD1 rule** with the rule named, or a **project extension** declared and counted — and C-35's
derivation (search for the least value that still satisfies the floor) is the pattern for the
second.

---

## 9. Gates

| gate | what it does | source |
|---|---|---|
| **the pass** | identity, refusal collection, conformance, provenance — **one execution pass**, run by whatever builds the game | H-10.1, D97's shape read as reference |
| **two readers** | the build gate is the hard stop; **the same findings are a record the notifications panel renders**, so a green build and a legible panel cannot disagree | C-53.3 |
| **a11y probe** | axe-core (WCAG 2.2 A/AA) **plus the checks axe cannot make** — overlap and control reachability | C-53.2 (binding) |
| **render probe** | assert the viewer **rendered pixels**, not that a page loaded | C-53.2 (binding) |
| **lanes are the test surface** | a check is visible by looking at the thing it checks; **no Tests tab** | event-0013, carried |
| **every view has a text form** | a view is never the only place a fact is visible | C-48's reference finding |

---

## 10. Not yet ruled — the ⚠️ list

| # | The open question | Question id |
|---|---|---|
| 1 | The `extent_unit` semantics for a pinned window (screen-space vs world-anchored vs both) | H-9.8 / H-11.1 |
| 2 | The notification panel's grouping, default pinning, and whether it is a window or a lane | H-11.3 |
| 3 | The type roles, the motion constants, the keyboard state set — and, of the ladder, only **which surface takes which level** (a declaration in the arrangement package). *The depth language itself is ruled: C-55 §55.1.* | H-11.9, H-13.2 |
| 4 | The action vocabulary — prerequisite of the permission matrix, not of this spec | H-11.2 |
| 5 | The record's on-disk shape — the outer grouping is the **world's year/season**, never a governance period | H-11.7 |

---

## 11. Reference notes — where the shapes above were read, and what they are for

* `portal_simulator.py` (4,252 lines) — the **window/pane/split/dock/undock** structure, the
  layout and behaviour policies, the console's graded levels, the taskbar and desktop. Its API is
  the better half; its window-addressing by list position is the worse half, and §2 refuses it.
* `alacripose-landing` — a **worked implementation** of the token, type, motion and keyboard-state
  layers, with two binding probes. Reference for shape; its values are re-derived (C-53.2).
* `ala-cities` — the ticket/governance surface and the 20 Hz deferred clock, as *behaviour* to
  learn from; `gov::season` there is named for a governance period and is **not** evidence about
  this game's seasons (C-49 ruling 8).
* `shploingus` (dirty) — the lane vocabulary and the frozen 14-row list, and the shell's tab list.
* `shploingus-game` — the plan's *shape* for a compile-time conformance gate and an API viewer.

None of these is authority here. Where one of them disagrees with this document, **this document
and `LEDGER.md` win**, and the disagreement is recorded rather than normalized.
