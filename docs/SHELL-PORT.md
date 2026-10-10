# SHELL-PORT — the plan for porting the reference's window machinery

> **Status: the plan C-110 §110.5 owes, and it lands before the first port module.**
>
> **Authority:** `LEDGER.md` C-110 (the user's ruling, §110.2's inventory, §110.3's survivors,
> §110.4's order), `UI-SPEC.md` §1–§7, H-9.7 / H-9.8 / H-7.15 / H-7.24.
> **Reference:** `docs/portal_simulator.py` (4,252 lines) — read for the machinery's shape, never
> cited as a ruling (UI-SPEC §11). The dirty tree's `PORT-MAP.md` is **not** authority here
> (C-48 / C-49).

---

## 1. What the port is

The user's ruling, 2026-10-03: *"it would be better to remove it and port portal simulator
wholesale to rust without the apps we don't need."* The shell's derived core — the window
drawing and management in `mount.rs` and the bar strip — is retired in favour of a **port of
`docs/portal_simulator.py`'s own window machinery**, and the simulator's product apps are not
ported.

**"Wholesale" means the machinery, not the simulator.** The reference is a Qt program: its
`QSplitter`, `QPushButton`, stylesheet and signal/slot wiring are **not** portable and are not
what is ported. What is ported is the structure the tree's renderer can host:

* the **constructs** — `PortalWindow`, `WindowPane`, `PaneHeader`, `PanePlaceholder`,
  `PortalSplitter`, `DropPreviewOverlay`, `PortalDesktop`, `PortalTaskbar`, `Theme`, `Portal`
  (the registry) — each read line by line from the reference;
* the **interactions** — split (three cases), dock with a drop preview, undock into a standalone
  window, close with splitter cleanup, title-bar drag with edge triggers, edge resize;
* the **proportions and field lists** — as *candidates*: every value that gets drawn is admitted
  into this tree's four-bucket vocabulary (§4), never copied as a literal.

The drawing target is `voxel-ui`'s `Element`/`Panel` renderer, and the declarations come from
this tree's own **package** (`voxel-package`'s `Recipe` / `WindowSpec`), so the ported machinery
is driven by what `packages/alpha-shell.shspec` declares — not by the simulator's app registry.

---

## 2. What is not ported (the apps we don't need)

The reference's product apps are **absent by ruling**: `HomeApplication` (276), `SettingsApplication`
(302), `FileManagerWidget` / `FileManagerApplication` (367, 475), `AboutApplication` (486),
`GridWidget` / `GridApplication` (510, 591), `BusinessSupervisor*` (1623, 2102),
`BusinessDatabase*` (2118, 2306), `ModelRouter*` (2322, 2411), `ExplorerView` (125),
`DetailsView` (195).

This game's windows come from its package (H-9.7, H-15.2). Where the simulator hard-codes
`self.apps = {…}` (3747–3770) and `populate_desktop`'s app-id tuple (3797–3803), this tree
substitutes the loaded `Recipe`'s windows.

**`ExplorerView`/`DetailsView` get a named decision rather than a silent absence** — D-3 in §6.

---

## 2.1 The reference's own design record, and what it says beyond the shipped file

Checked 2026-10-03 at `F:/Documents/simulators` — the simulator's own repository, **reference,
never authority** (C-48 / C-49).

* Its `portal_simulator.py` is **byte-identical** to this tree's `docs/portal_simulator.py`
  (`md5 d7fb4c45a9ba2d0b81185c0d8842b801`, 4,252 lines), so §3's line ranges port the final
  shipped file; `desktop_portal.py` there is the same bytes under an older name.
* `docs/design/window_splitting_and_tiling.md` and
  `docs/implementation_plans/task_splitting_and_grid_engine.md` — the design and plan **of the
  machinery §3 ports** (40×40 floor, stretch factors, the three `split_pane` cases,
  `_cleanup_splitter`, `undock_pane`, `dock_window`, the drag trio, `<24 px` outer-edge docking).
* `docs/walkthroughs/task_splitting_and_grid_engine.md` — its verification report: four offscreen
  suites passed (`test_handle_drag.py`, `test_grid_split.py`, `test_comprehensive.py`,
  `test_split.py`); the suites themselves are not in the repository.
* `AGENT.md` / `GEMINI.md` (identical) — the repository's rules, including the **six multi-split
  edge cases** it required every layout change to test (H/V splits; nested T/asymmetrical;
  handle drags; drop docking; sequential close to removal; undock). §7's eye check walks the
  same six.
* `docs/implementation_plans/task_viewer_and_console.md` — the design behind §3's owed
  `task_viewer.rs` and the console toolbar (the level set `[SYS] [EVENT] [INFO] [WARN] [ACTION]`,
  the command vocabulary, the stats header).
* `docs/logs/copilot_all_prompts_2026-09-01T09-57-36.json` — five prompts from the simulator's
  last session, and **two directions beyond the shipped file, neither executed**:

  1. **A tabbed-multitasking design** (prompt 1, quoted in full in the log): the main window
     getting the same `TitleBarElement` controls; a browser-style top **tab shell** (a custom
     `PortalTabBar`, tear-off by vertical escape, `＋` new-instance tab); **`TabbedPaneGroup`**
     in-window tab groups with `◧ / ⬓ / ⇱ / ×`; **merge-as-tab** centre drop
     (`⊞ Merge as Tab`); bezier-spline animations 120–220 ms; console
     `tabs` / `tab` / `untab` / `tabmerge`; a `window_animations` setting. Never saved to
     `implementation_plans/`, never implemented — the shipped file has no `TabbedPaneGroup`,
     `PortalTabBar`, `main_titlebar` or `cubic_bezier`.
  2. **The orb direction** (prompts 3–4, the user's own words): *"remove the fake desktop and
     fake applications. The user should log in to a floating desktop concept… In place of the
     traditional window, a floating orb that fades out at the edges with a dissolve or dither or
     gaussian blur effect with the background, these floating orbs should change their size
     according to their contents, and provide the normal windowing controls as before but within
     their render frame"*, refined: *"more like light spheres… the distant orbs that are out of
     focus should be dithered, the entire 'desktop'… navigatable like a 2D scene that you can
     zoom into, but the windows and the background should appear with depth."* The design
     document it asked for was never written (`orbs.md` is empty, 4 bytes).

**D-4 and D-5 (§6): whether either direction enters this port is asked, not assumed.** C-110
ports the shipped machinery; the reference's own next steps are material, and this plan records
them rather than smuggling them in.

---

## 3. The module map, in landing order

Each entry names the reference range it ports from, what it replaces, and what it must not
re-derive. Modules are new files under `crates/voxel-client/src/shell/`; `mount.rs` keeps what
is this tree's own (the one-writer chokepoint `announce`, the console's records and behaviour,
`ContentViews`, the command surface, package loading) until §8's retirement applies.

### Step 1 — the window and its parts (C-110 §110.4 item 1)

| module | ports from `portal_simulator.py` | replaces |
|---|---|---|
| `shell/titlebar.rs` | `WindowElement` 2428–2432, `ControlButtonGroupElement` 2434–2454, `TitleBarElement` 2455–2515 (title text and icon 2464–2474, `set_title` 2476, press/move/release 2479–2505, double-click-maximize 2506–2515) | the title-bar pass of `mount.rs::window_elements` and `Mount::control_lanes` (3396) |
| `shell/pane.rs` | `PaneHeader` 2516–2582 (title 2522, `◧`/`⬓` split 2534–2550, `⇱` undock 2552–2560, `×` close 2562–2570, `update_title` 2576–2582), `WindowPane` 2653–2718 (header slot 2666–2675, content-or-placeholder 2677–2685, floor 2686–2688, click-to-focus 2712–2718) | the pane drawing inside `window_elements` |
| `shell/placeholder.rs` | `PanePlaceholder` 2583–2631 | **nothing — it does not exist today**; the reference's ten hard-coded app buttons (2611–2618) become this tree's loaded package's window list |
| `shell/splitter.rs` | `PortalSplitter` 2632–2652 (4 dp handles, 40×40 `minimumSizeHint`, stretch factors) | **retired in C-116**: every layout resolves through `splitter::layout` (`PortalWindow::layout_at`); `mount.rs::layout_panes`/`pane_at`/`pane_rect_of` are deleted |
| `shell/drop.rs` | `DropPreviewOverlay` 2719–2777 (geometry per zone 2744–2774) **and** the drop-zone rule from `continue_window_drag` 3190–3246 (outer 24 px edge first, then nearest-edge dominance inside a pane) | the drag's preview and landing: `Mount::drop_target_for`/`dock_target` (C-116) |
| `shell/window.rs` | `PortalWindow` 2778–3473: the composite (2784–2886), `get_all_panes` 2890–2904, `split` 2905–2909, `split_pane` 2919–3012, `dock_window` 3013–3039, `undock_pane` 3040–3085, `close_pane` 3086–3110, `_cleanup_splitter` 3111–3140, `update_titlebar_and_taskbar` 3141–3164, drag 3165–3269, `_position_edge_triggers`/show/hide 3270–3296, focus visual 3312–3320, mouse 3321–3419, `_edges_at`/`_cursor_for_edges`/`_resize_from` 3420–3473 | **wired in C-116**: `Mount`'s pointer is the port's own verdicts (`press_at`, `DragState`, `resize_from`, `edge_triggers_at`, `drop_target_at`) and `Mount::dock`/`dock_target` delegate to `Window::dock`; the lifecycle (`open`/`focus`/`minimize`/`toggle_maximize`/`close`/`pane_rect`) is `Mount`'s, the state §2 gives it |

**Status, 2026-10-03:** **step 1 is landed** — all six modules above, with `window.rs`
carrying the lifecycle, the composite and the mouse machine (C-113, C-114) — and **step 2's
modules are landed** (`desktop.rs`, `taskbar.rs`, `theme.rs`; C-115). **The drawing is wired**
(`shell/hud.rs::frame` → `shell_elements` → `window_elements` → `elements_at`) and **the pointer
is wired through the port** (C-116): every press is a `WindowPress` verdict, every move a
`DragState`/resize/hover pass, every release lands on the preview `drop_target_at` showed, and
the derived layout helpers are deleted. The **C-40 eye check is owed on this wiring** — the
first shell the port draws *and answers* in the client — and the split controls and the
empty-slot offers refuse by name until a **slot-identity rule** is ruled (LEDGER C-116 §116.4).

**`split_pane`'s three cases (2919–3012) are the tree's data model** in
`arrangement.rs::Arrangement::split` — root-is-pane wraps; **the target's parent splitter**
inserts *beside the target* when the orientation agrees and lifts **the target alone** into a
fresh sub-splitter when it disagrees, restoring the parent's stretch (`parent.setSizes(old_sizes)`).
**Corrected 2026-10-03, and the correction is on the record (LEDGER C-112):** the first
implementation wrapped the *child subtree* and inserted only when the target was itself a
same-orientation splitter, so `H[a, b]` split at `a` became a nesting instead of `H[a, new, b]`
— `splitter.rs`'s three-across floor test caught it, and C-110 §110.2's "faithful in the data"
claim was true of the tree's *description* and not of its code. Three tests now pin the
reference's cases. The port **draws that tree**; it does not build a second pane tree.

**`_cleanup_splitter` (3111–3140) is already `Arrangement::close`'s collapse.** One
implementation, in `arrangement.rs`; the port calls it.

### Step 2 — the desktop, the taskbar, the theme (item 2)

| module | ports from | replaces |
|---|---|---|
| `shell/desktop.rs` | `PortalDesktop` 3475–3597: windows list, `add_window` 3508, `remove_window` 3516, `focus_window` 3529, `toggle_maximize` 3540, `minimize` 3566, `restore` 3575, `clear_focus` 3587, wallpaper `paintEvent` 3494–3507 | `Mount`'s window list and focus |
| `shell/taskbar.rs` | `PortalTaskbar` 3598–3729: construction 3599–3636, clock 3657–3662, `refresh` 3663–3690, `task_clicked` 3691–3698 | the bar strip — **D-2 ruled: one top surface, the clock kept; no Start, no bottom strip** |
| `shell/theme.rs` | `Theme` 23–44 (17 fields), `THEMES` 46–74 (five), `apply_theme` 3850–4240 (the selector→field map) | `Palette`'s single `Theme::ONLY` |

**What `theme.rs` ports and what it re-derives.** The reference's field list is the *coverage a
theme must state* — desktop, window body, window border, title active/inactive/text, text,
secondary text, button / hover / pressed, accent, panel, desktop text, taskbar / border. The
tree's vocabulary is how a value is admitted: `Role` + `Ink` + the elevation tint, each value in
one of the four buckets (§4), with unadmitted roles **refusing by name** (`Palette::get`) rather
than being filled. The reference's five named themes are candidates; **MD1 binds**, and
`apply_theme`'s Qt stylesheet text is translated, never copied.

### Step 3 — this game's windows as the port's panes (item 3)

| module | ports from | notes |
|---|---|---|
| `shell/portal.rs` | `Portal` 3730–3850: `log` 3786–3796, `populate_desktop` 3797–3803, `launch` 3804–3826, `status` 4241–4243 | `log` routes through `Mount::announce` (one writer, H-12.2); `launch` opens a **declared** window at its declared geometry, clamped — the registry is the package's window list |

When this lands and the shipped `packages/alpha-shell.shspec` draws through the port, §8's
retirement applies — and not before (C-40).

### Step 4 — the ported shell's transient surface (item 4)

| module | ports from | notes |
|---|---|---|
| `shell/toast.rs` | **no reference class** — the reference has no toast | the queue and records stay (`notifications.rs`: one at a time, `TOAST_MS`, visible timer); the user ruled the current form wrong (*"toasts arent themselves actual window toast notifications like you would see on android?"*) and §110.4 item 4 says the form is *"a small floating surface that hugs its line"*, decided and drawn here, against MD1's snackbar (UI-SPEC §7) |

### Outside the four steps — owed constructs from §110.2

| module | ports from | why owed |
|---|---|---|
| `shell/task_viewer.rs` | `TaskViewerWidget` 607–893: table 609–708, `populate_tasks` 718–790, the six actions 791–854, context menu 855–877 | §80.5's four-surface API names it; the tree's *subject* is the ledger's tickets/board, so the reference supplies the shape (columns, Focus·Min·Max·Undock·Split·Close), not the nouns |
| console toolbar (in `shell/console.rs`) | `ConsoleWidget.__init__` 896–968 (filter / search / clear / copy / auto-scroll), `render_logs` 986–1028 | the console's *behaviour* already landed (scrollback, 2026-10-03); the toolbar is the ported pane's remaining half |

---

## 4. Rules that bind every module

1. **No literals.** Every number or colour that gets drawn is an MD1 rule, an MD1 example, a
   **platform translation**, or a **project extension** — and an extension is a *counted* act
   (`voxel-ui`'s pinned count test). The reference's values are candidates with a provenance,
   never values to copy.
2. **One pane tree.** `arrangement.rs`'s `Arrangement` is the tree. The port draws it and mutates
   it; a second tree is the defect the *D-07 collision* warnings exist for.
3. **One writer.** The ported `Portal`-equivalent logs through `Mount::announce` — H-12.2's
   chokepoint, C-54.3's one API for console/toast/panel.
4. **One pointer feed.** C-108's split stands: look reads `DeviceEvent::MouseMotion`; the shell
   pointer reads `CursorMoved` and is hidden/shown with the grab. Drag, edge triggers and resize
   consume the shell pointer — they must not open a second source.
5. **Windows are data.** `WindowSpec` (recipe.rs) is the only declaration of a window's title,
   policies, geometry, content, lane and elevation. The port adds no second registry.
6. **C-40.** The client is looked at, never mid-flight: each module lands with its tests, and the
   derived core is deleted only when the port draws the shipped package (§8).

---

## 5. The defect the port fixes by construction — pane rects

The reference's hit tests and its drawing resolve against **the same widgets Qt laid out**:
`dock_window(target_pane=…)` (3013), `show_drop_preview` (3264), `_edges_at` (3420) all use the
pane geometry the splitter gave them.

This tree's derived shell has the mismatch named in C-104/C-107's owed list: `dock`/`undock`/
right-click/`pane_rect` resolve against the **full window rect**, while the drawing lays out
inside `window_body` (mount.rs:229). The port keeps **one rect arithmetic** — `window_body` — and
every pane rect, drop preview and edge trigger position is derived from the pass that draws it.
A test pins drawing and hit-testing to the same rects (the class of defect `pane_at` 3535 exists
to prevent).

---

## 6. Conflicts this plan does not silently decide

**D-1 — the control size floor vs the reference's compactness vs "elements of ui way too large".**
The reference's chrome is desktop-compact: title bar 34 tall (`WindowLayoutPolicy.title_height`,
90), title-bar controls 34 wide (2450), pane header 26 tall with 22×20 buttons (2522–2570), edge
triggers 24×24 (2857), taskbar 48 (3602). This tree's ruled numbers are `MIN_TARGET_DP` **48**
(MD1 rule — the source says *"a touch target is at least 48 dp"*, MD1 and HIG **agreeing about
touch**), `TITLE_BAR_DP` **48** (justified in its source by the control floor), and today's
derived shell draws control lanes 48×48. The port cannot draw a control without a ruled size,
and the user's too-large finding (§110.5) sits exactly here. Options:

* **(a) the floor binds both axes** — pane headers become ≥48; nothing gets smaller; the
  too-large finding is answered elsewhere (density, not target size).
* **(b) a platform translation** — the 48 dp floor is a touch target; **this client's shell is
  pointer-driven** (C-108), so pointer-surface targets take a translated floor, declared as its
  own counted bucket row with a named source (value and source owed with the ruling).
* **(c) split by role** — primary window controls stay ≥48, secondary pane chrome draws compact.

**Ruled 2026-10-03: (b), the pointer translation.** The value and source are no longer owed —
**MD1's own dense-pointer pair**: `POINTER_TARGET_DP` **40** and `POINTER_GLYPH_DP` **20**
(`material-design-1` §4.5: *"dense pointer layouts may use a 20 dp icon in a 40 dp target"*),
landed in `voxel-ui` as `Bucket::Md1Example` rows and pinned by
`the_pointer_translation_is_md1s_dense_pointer_pair_and_narrower_than_touch`. The reference's own
chrome (34 dp title row, 22×20 pane buttons) is denser still; **the floor wins over the literal**,
and every place that happens says so.

**D-2 — the bar vs the taskbar (H-7.15).** `UI-SPEC.md` §5: the bar carries the declared tab set,
top, 48 dp (*"the shell is the upper bar"*). The reference puts a 48-tall taskbar at the
**bottom** carrying Start + the running list + the clock, with auto-hide opacity. Options:
(i) two surfaces — upper bar = tabs/entries, ported taskbar = running windows + clock + start;
(ii) one surface — the upper bar carries tabs *and* running entries, no bottom strip; (iii) the
taskbar wins entirely, tabs included. Named at `taskbar.rs`'s landing.

**Ruled 2026-10-03: (ii) — one surface, top, and the clock kept.** The upper bar carries the tab
set *and* the running-window entries; there is no bottom strip; the reference's clock
(`PortalTaskbar.update_clock`, 3657–3662) is kept in the bar, while its Start menu and auto-hide
opacity are not ported (the launcher is the package's windows plus the bar, §5).

**Glyphs — ruled 2026-10-03, by the user's word:** the shell's marks are **Unicode glyphs** (the
reference's ◧ ⬓ ⇱ × and the dock preview's ◫), not an icon asset. The port extends the tree's
declared glyph set (`GLYPHS`) with the reference's marks rather than introducing an icon pipeline.

**D-3 — the desktop's own icons. RULED 2026-10-03, and it closed the wallpaper too.** The user's
words: *"So i'm gonna let you down easy here the desktop background over the game would be
dumb."* **Neither the wallpaper (3494–3507) nor `ExplorerView`'s icon list (125) is ported.** The
rendered world is the background; the desktop draws nothing, and `theme.rs`'s `desktop` field is
`FieldSource::World` — resolved to `Ok(None)`, deliberately not a refusal, because *a debt and a
ruling are different facts*. `Role::Canvas` stays unadmitted by design. Landed in C-115 §115.1.

**D-4 — the tabbed-multitasking design** (§2.1 item 1). Carried into the port, or kept as
reference only? It moves the port's scope: a top tab shell, pane tab groups, centre-drop
merge-as-tab, tear-off, and its animation vocabulary.

**D-5 — the orb direction** (§2.1 item 2). *"In place of the traditional window, a floating
orb"* replaces the frame §3 step 1 ports — the machinery (panes, split, dock, undock, close)
stays, but the window's own surface is a light sphere on a zoomable desktop with depth. Carried
as the port's target form, sequenced after the machinery, or kept as reference only?

---

## 7. Gates each module lands through

1. **Unit tests in the module** — the tree's convention. Behaviour that exists today (pane tree,
   console, notifications, toast queue) keeps its witnesses; the drawing that moves gets tests
   in the module it moved to.
2. **`cargo test --offline --workspace`** re-run, with the count compared to the previous run.
3. **`cargo clippy -p voxel-client --no-deps -- -D warnings`** (the workspace-wide run fails on
   a pre-existing `voxel-model` finding, named in the record).
4. **The C-40 eye check** on the running client at each step's end: the shell shown, the shipped
   package's windows drawn and interacted with by hand (drag, dock, undock, split, close,
   resize) — the check is the point, not a formality.
5. **A ledger entry per landed step**, naming the modules and the reference ranges they ported
   from, plus anything the step refused or left owed.

---

## 8. Retirement — what is deleted, and only when

Deleted **only when the port draws the shipped `alpha-shell.shspec`** through step 3 (C-40).
Named now so "remove it" cannot mean "remove nothing" or "remove early":

* `Mount::window_elements` (the derived window drawing) and its title-bar/pane passes;
* `Mount::bar_elements`, `bar_rects`, `menu_elements`, `menu_rows` — D-2: the strip is redrawn
  by the port with the clock, and nothing is built below it;
* `Mount::control_lanes`, `is_on_title_bar`, the window branches of
  `pointer_pressed`/`pointer_moved`/`pointer_released`, the `Drag` struct;
* `Mount::focus`/`minimize`/`toggle_maximize`/`close`/`undock`/`dock`/`pane_rect`/`rect_of`/
  `pane_titles`/`title_of` — replaced by `desktop.rs`/`window.rs`;
* the helpers `layout_panes` (3489) / `pane_at` (3535) / `pane_rect_of` (3565).

**Kept:** `Mount::new`/`adopt`/`reload`/`log`/`announce`/`set_tick`, the console's records and
scrollback, `ContentViews::panel_for` and the notifications reader, the command surface, package
loading — and every contract C-110 §110.3 says survives.

---

## 9. Owed beyond this plan

* **D-1 / D-2** — ruled 2026-10-03 (§6); D-1's constants landed in `voxel-ui`.
* **D-3** — the desktop's own icons; named at `desktop.rs`'s landing.
* **D-4 / D-5** — the reference's two post-shipped directions (§2.1); still open. C-110's scope
  (the shipped machinery) is what the port proceeds on until a ruling says otherwise.
* **The toast's final form** — step 4.
* **The UI scale's second half** — D-1 is the target-size half; text/density rides UI-SPEC
  §8.2's owed type roles.
* **`TaskViewerWidget`'s subject** — the ledger's tickets/board, not the simulator's business
  tasks; named at its module's landing.
* **The notifications window's placement** in the ported shell (H-11.3's open half).
