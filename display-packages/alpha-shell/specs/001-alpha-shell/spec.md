001 — The shell as a package
============================

What this is
------------

The alpha shell is the game's first **display package**: the windows, tabs and pins the user sees at
launch, declared in the recipe grammar rather than constructed in Rust. The requirement is the user's
own — *"the updated rendering package will need to support … the new mount v2 package format"* — and
the constraint that makes it a package rather than a table is UI-SPEC §2: a window is a declared
`WindowSpec`, and the shell owns geometry, focus and z-order at runtime.

What it must express
--------------------

* **Six tabs**, in strip order: `inventory`, `trees`, `mods`, `docs`, `settings`, `map-plan`. The
  record's eight were cut to six by the user; `governance` and `tools` are console-opened.
* **One bar entry that is not a tab**: `console`, the command surface (H-8.2).
* **Six filed windows**, each offered by exactly one tab's menu (H-15.3): tools → inventory,
  api-viewer → mods, refusals → docs, notifications → settings, determinism → settings, world →
  map-plan.
* **Three pins**: `streamer`, `determinism`, `refusals` live in the HUD while the shell is hidden
  (H-9.8).
* **Every window's home is declared**, never inferred from position or lane hue. A declared window
  with none of tab/bar/filed is console-opened, and the console opens any declared window by id.

What it must not do
-------------------

* Fork the constitution. This folder cites `clean-room/.specify/memory/constitution.md`; it does not
  copy it (M10.0).
* Own a shader. The look lives in `shading.wgsl` and a package may not write it (`C-33`, `AR15`).
* Invent a window. Every id here is one the client already opens, and adding one is a patch to this
  tree rather than a package edit.

Acceptance
----------

`cargo build -p shploingus` compiles this folder's `package.shspec`, loads it back through the codec,
and embeds the bytes; the port is refused by path when `package.shspec` is missing. The recipe parses
with 16 windows, 6 tabs, 1 bar entry, 6 filed entries and 3 pins, and the client's own mount tests
read the same bytes.
