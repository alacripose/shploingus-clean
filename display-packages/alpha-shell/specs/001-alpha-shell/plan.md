001 — Plan: what the shell declares, and why each window exists
==============================================================

The arrangement is the recipe's; the reason each window is on the list is the record's. Nothing below
adds a window — every id already exists in the client and in UI-SPEC's table.

The six tabs, in strip order
----------------------------

| tab | why it is a tab |
|---|---|
| `inventory` | the crafting surface; the first thing a player opens, and the tab that owns the `tools` menu. |
| `trees` | the working/crafting trees — the game's progression view (PATCH-043). |
| `mods` | the mod surface, and the home of `api-viewer` (PATCH-042). |
| `docs` | the record as a window; the home of `refusals`, because a refusal nobody can read did not happen. |
| `settings` | the knobs, and the home of `notifications` and `determinism`. |
| `map-plan` | the map/plan view; the home of `world`. M9 later draws the axis's own cells in it. |

The bar
-------

`console` is a **bar** entry, not a tab: it is the command surface (H-8.2), it opens any declared
window by id, and it is the one window that must be reachable from every tab. `governance` and `tools`
are console-opened, not homeless.

The filed windows
-----------------

| window | tab | why the menu and not the strip |
|---|---|---|
| `tools` | inventory | used *while* crafting, so it opens beside the inventory rather than replacing it. |
| `api-viewer` | mods | read next to a mod's declaration; its rows are the api layer's own (PATCH-042). |
| `refusals` | docs | the record's negative space, read with the documents that state the rules. |
| `notifications` | settings | the queue a player turns off; it belongs with the other knobs. |
| `determinism` | settings | a diagnostic, pinned only when someone is measuring. |
| `world` | map-plan | the world window is the map's companion, not a second map. |

The pins
--------

`streamer`, `determinism` and `refusals` are pinnable and pinned: they live in the HUD while the shell
is hidden, because they report on the frame and on the record rather than holding content. A pinned
window's height is the package's declaration and the HUD's parameter — never a constant in code.

Why no layers and no rows
-------------------------

A layer is a GPU pass this package brings; the shell draws with the renderer's own pipelines, so it
declares none. A row is a record the package implements; the shell's work is filed in `patches/` and
the ledger, and copying those rows here would be a second copy of the record — the failure the patch
coverage join exists to prevent.

Measurements
------------

* `package.shspec`: 16 `window` blocks, 6 `tab`, 1 `bar`, 6 `filed`, 3 `pin`.
* `build.rs` compiles it into `OUT_DIR/alpha-shell.shpkg` and embeds it (`main.rs`).
* `cargo run -p voxel-package --bin voxel-layers` over this folder prints `no layers declared`, which
  is the honest state rather than a missing file.
