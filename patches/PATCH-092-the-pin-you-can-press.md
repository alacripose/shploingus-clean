---
patch_id: PATCH-092
title: The pin you can press — one pin state, a glyph in every bar, and the map's chrome sliding away
status: applied
operation: feature + fix
target: voxel-client/shell/chrome + shploingus/live-instrument
base_ref: working tree at 2026-10-27, after PATCH-091
dependencies: [PATCH-091]
source: user-directed-mission
created_at: 2026-10-27
approval: "user-directed 2026-10-27 — 'build the shell-chrome cluster: a runtime per-window pin state, a pin glyph button in every window's glyph row, a minimal titlebar declared as XML, and the map's control gutter and toolbar sliding away when pinned; then drive the actual shploingus window, not just shell-shot'"
live_mutation: false
---

# Pin was a permission pretending to be a state

The shell could *read* a pinned window — the HUD layer, `pinned_elements`, the layer choice — and
nothing could ever *enter* it. The nearest thing to a pin state was `WindowSpec::pinnable`, a static
bit the package declares; the only windows on the pinned layer were the ones the package's `pin`
rows named at mount. A bar had three controls and none of them was a pin, a pinned window wore the
same bar as any other, and the map spent its body on a readout row and a button column whether or
not it was the small thing a player keeps in view.

| fault | what it cost |
|---|---|
| `spec.pinnable` was the only thing that looked like a pin state | a *permission* is not a state: no press could pin anything, and a test over the spec bit would pass while the button did not exist |
| no pin in the glyph row | the pinned layer's membership was whatever the package said at mount, forever |
| no minimal bar | a pinned window named itself and drew full-size marks — the one form that is small enough to live in the HUD drew chrome it does not need |
| the map's readout toolbar and control gutter were unconditional | a pinned map — a minimap — gave a row and a column of its body to controls it cannot afford |

# The structure now

| module | owns | does not |
|---|---|---|
| [`voxel-ui`](crates/voxel-ui/src/lib.rs) | `GlyphAction` (pin first, the one action no layout policy gates) and the pin's two mark shapes | know a window, a bar or a layer |
| [`shell/window.rs`](crates/voxel-client/src/shell/window.rs) | `PortalWindow::pinned` — **the one copy of the runtime state** — `bar_style()`, `set_pinned`, `toggle_pin`, and `composite(spec, rect, pinned)` | draw, or decide what a layer means |
| [`shell/titlebar.rs`](crates/voxel-client/src/shell/titlebar.rs) | `BarStyle` (declared and minimal, the minimal one parsed from the shell's own XML document), the lanes, the elements, `press` | hold state, or know which layer the window is on |
| [`shell/mount.rs`](crates/voxel-client/src/shell/mount.rs) | `toggle_pin` (the only writer), the two layers and their order, and `map_chrome_shown` — one question with four readers | draw a bar itself, or let a caller disagree with what it drew |
| [`shploingus/src/main.rs`](crates/shploingus/src/main.rs) | the frame loop, and the scripted shell pointer that drives it | reach past the shell's own entry points |

```
package `pin` rows ─> Mount::adopt ─> PortalWindow::pinned <── Mount::toggle_pin <── a lane press
                                             │
              ┌──────────────────────────────┼────────────────────────────────┐
              v                              v                                v
   Layer::Shell / Layer::Pinned     BarStyle::of(pinned)             map_chrome_shown()
   (draw_order, pinned_elements)    (composite → the body's top)     (toolbar, gutter, body)
```

**The declaration seeds it once and then it is the player's.** `Mount::adopt` reads the package's
`pin` rows into `PortalWindow::pinned`; from there the pin lane is the writer and the layers, the
bar style and the map's chrome are readers. `spec.pinnable` stays exactly where the declaration put
it — the recipe's statement that a window *may* live in the HUD, which is what its validator checks
a `pin` row against, and the distinction the old spec-only test could not make.

**The minimal bar is a document, not a second drawing.** `<titlebar form="minimal" height="40"
lane="40" glyph="14" gap="8" title="none"/>` is parsed by the shell's own XML reader
([`shell/xml.rs`](crates/voxel-client/src/shell/xml.rs)) and `BarStyle::of(pinned)` answers the
declared form or the minimal one. The lanes, the elements and the press all take a `BarStyle`, so
the pinned bar is the same three functions at another form's numbers — smaller marks, no title, a
shorter strip — rather than a path of its own.

**The map's chrome is one question.** `map_chrome_shown()` is `!is_pinned("map-plan")` and
`map_view_rect`, `map_toolbar_rect`, `map_controls_rect` and the pointer's hit tests all read it, so
the ground and the controls cannot disagree about whether the chrome is there. Pinned, the ground's
rect **is** the content container — x, y, right and bottom all equal to it — and the controls'
document is not laid out at all, so no lane can answer a click the frame does not draw.

# The defect the proof found

| the defect | how it showed | the fix |
|---|---|---|
| **a capture had never drawn the shell at all** — `if graphics.capture.is_none() && …` skipped the shell's pass whenever `SHPLOINGUS_CAPTURE_DIR` was set | the first live run of the scripted pointer recorded 110 frames of the world with no bar, no window and no glyph in any of them: *the frame loop was never running the chrome the capture was supposed to prove* | the recorder copies the texture the frame was **presented from**, and that texture holds the overlay; the shell is now drawn whenever one exists, and the one case that wants a surface measurement asks for it by name with `SHPLOINGUS_CAPTURE_WORLD_ONLY` |

# The witnesses

| witness | where | what it proves |
|---|---|---|
| one writer, one state | `shell::mount::tests::the_declared_pins_seed_the_runtime_state_and_pinnable_alone_does_not` | the package's `pin` row is the seed; `pinnable` alone is not a state |
| the button, through the lane the frame draws | `shell::mount::tests::the_pin_glyph_toggles_the_layer_and_the_minimal_bar_through_one_press` | a press on the drawn lane pins the window, switches the bar's form and its height, removes the name from the drawn elements, and the same lane puts it back |
| the layers | `shell::mount::tests::a_pinned_window_draws_above_a_floating_one` | a window pinned *by the player* draws over one focused later, the pinned layer holds only pinned windows, and the pointer resolves the one on top |
| the minimal bar is the document it names | `shell::titlebar::tests::the_minimal_bar_document_is_the_style_it_names` | the XML's own numbers are the style's, a malformed document is refused by name, and the minimal form draws no title |
| the same lanes at the minimal numbers | `shell::titlebar::tests::a_pinned_window_draws_no_title_and_smaller_marks_through_the_same_lanes` | one element list, smaller marks, the shorter bar, and the pin answering in both forms |
| the mark states which side it is on | `shell::titlebar::tests::the_pin_mark_states_which_side_the_window_is_on` | ring for unpinned, filled for pinned, in both forms |
| the map's chrome slides and returns | `shell::mount::tests::pinning_the_map_slides_its_chrome_away_and_the_body_takes_the_room` | toolbar and gutter `None`, the ground's rect equal to the content container on all four edges, no control document laid out, and the exact unpinned rect back after a second press |
| the name in the bar, through the shell's own elements | `shell::mount::tests::the_docs_window_draws_its_name_in_the_bar_and_a_pinned_one_does_not` | one "Docs" inside the bar's rect unpinned, none pinned, over the shipped package |

# The live proof

**The running game, not the fixture.** `SHPLOINGUS_SHELL` names frames and verbs and each verb is
dispatched through the same entry point the window's `MouseInput` event uses
(`ShellMount::pointer_pressed` with the lane the frame draws), so the pin path that ran is the one a
hand runs. A malformed entry refuses the whole script rather than skipping half of it.

```
SHPLOINGUS_SHELL="14:open docs,22:pin docs,42:open map-plan,58:pin map-plan,
                  90:unpin map-plan,102:unpin docs,110:quit" \
SHPLOINGUS_CAPTURE_DIR=target/live3 cargo run -p shploingus
```

Every verb landed, in the frame loop, on the current build:

| frame | t | the frame said |
|---|---|---|
| 14 | 81026 | `docs opened` |
| 22 | 81042 | `docs pinned` |
| 42 | 81081 | `map-plan opened` |
| 58 | 81106 | `map-plan pinned` |
| 90 | 81154 | `map-plan unpinned` |
| 102 | 81169 | `docs unpinned` |

The frames themselves — each one the texture that frame was presented from, kept as the PNGs under
`target/`, taken twice (the second run after the last source change, with the same frames firing;
the raw `.ppm` frames are distilled to these and deleted, they are 2.7 MB apiece):

* [`live3bars.png`](../target/live3bars.png) — the docs window's bar across five of its frames:
  **ring and full-size marks** unpinned, **filled dot and smaller marks** while pinned, ring and
  full size again after the unpin;
* [`live3mapbar.png`](../target/live3mapbar.png) — the map window's own bar, the same three states;
* [`live3crop-0042.png`](../target/live3crop-0042.png) /
  [`live3crop-0058.png`](../target/live3crop-0058.png) /
  [`live3crop-0090.png`](../target/live3crop-0090.png) — the map's content container: the readout
  toolbar (`ZOOM`, `CURSOR X/Z/Y`, `VIEW`, `BIOMES`) and the button column present, then **gone**
  with the ground filling all four edges, then present again.

And the shell's own reading of what a frame drew, which is a number and not an opinion — the same
three counts from both runs:

| the map's state | surfaces | ink | covered by a later surface |
|---|---|---|---|
| opened, chrome on | 7 341 | 188 | 44 |
| pinned (minimap) | **11 778** | **140** | 21 |
| unpinned again | 7 341 | 188 | 44 |

The ground gains the toolbar's row and the gutter's column (7 341 → 11 778 cells), the controls' own
ink leaves the frame (188 → 140), and the unpin returns the drawing to the same three counts.

**And the layer, which is what a pin is for.** A second scripted run pins `docs` by its lane, opens
`tools` (which the package does not pin), and hides the shell:

```
SHPLOINGUS_SHELL="14:open docs,22:pin docs,50:open tools,55:shell,80:shell,90:quit"
```

| frame | the frame said | `shell drawing` |
|---|---|---|
| 52 | shell shown, `tools` open | 19 surface(s), 152 ink, 19 covered |
| 65 | **the shell is hidden** | 15 surface(s), 116 ink, 10 covered |
| 85 | the shell is shown | 19 surface(s), 152 ink, 19 covered |

[`live4b-0052.png`](../target/live4b-0052.png) against [`live4b-0065.png`](../target/live4b-0065.png):
the menu bar and the whole unpinned `tools` window are gone, and **the docs window the press pinned
is still drawn** — together with the four the package pins by declaration (`world`, `streamer`,
`determinism`, `refusals`). Hiding the shell is the one thing a pinned window exists to survive, and
it did, in the frame loop.

# What this does not do

* **The map's camera, survey and marks are untouched.** The chrome's rects moved; what draws inside
  them is PATCH-091's work, and the survey still answers from the streamer's residency with the laws
  beyond it.
* **The pin is not persisted.** It is the mount's runtime state, seeded per mount; a run starts with
  the package's `pin` rows and no file records a press.
* Hammer tools, the far field and fog are untouched, as the mission ruled.
* The pin is drawn by every bar that exists (`titlebar::controls` is ungated for it); a *bar-less*
  window (`title-position: none`) has no glyph row and therefore no pin, which is the same rule the
  other three controls already followed.
