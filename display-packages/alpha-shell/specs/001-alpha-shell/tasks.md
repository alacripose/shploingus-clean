001 — Tasks: the rows of this package
=====================================

A task here is a **declaration in `package.shspec`**, not a plan item: this package declares no
`row` lines, because its work is already filed in `patches/` and restating it would be a second copy
of the record. What follows is the list a reviewer checks the file against, one line per declared
thing.

- [x] `package alpha-shell v1-alpha` — the name and version the client reports.
- [x] 16 `window` blocks, each with all thirteen required keys (UI-SPEC §2).
- [x] `tab inventory`, `tab trees`, `tab mods`, `tab docs`, `tab settings`, `tab map-plan` — the six
      the user ruled, in strip order.
- [x] `bar console` — the command surface is a bar entry, not a tab (H-8.2).
- [x] `filed tools inventory`, `filed api-viewer mods`, `filed refusals docs`,
      `filed notifications settings`, `filed determinism settings`, `filed world map-plan` — one home
      per window (H-15.3).
- [x] `pin streamer`, `pin determinism`, `pin refusals` — the three HUD residents (H-9.8).
- [x] `governance` and `tools` carry no tab/bar/filed row: console-opened is a declared state, not a
      missing home.
- [ ] A `claims.shpkg` for this folder — M10.3, and not written by anything today.
- [ ] A layer, if the shell ever needs its own pass — a layer is a package's, and `shading.wgsl` is
      never one (C-33).

How this list is checked
------------------------

    cargo build -p shploingus          # compiles and embeds the folder's recipe
    cargo run -p voxel-package --bin voxel-layers -- display-packages/alpha-shell

The first refuses the build by path if the recipe is missing or refused; the second reports that the
folder declares no layer, and exit 0 means that is what its files say.
