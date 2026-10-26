The display packages — source folders, one per package
======================================================

A **display package** is what Mount v2 mounts: the recipe grammar the shell already reads
(`shploingus.recipe/3`), the layers it names (`shploingus.layer/1`), and the records that say why
each one exists. This directory is the **source**; `packages/` stays the compiler's **emit root**, so a
`.shpkg` in `packages/` is output and never edited, and a package's source is never written by a
build.

The folder, exactly
-------------------

```
display-packages/<name>/
  specs/NNN-<slug>/spec.md      what the package is for (the ticket's own words)
  specs/NNN-<slug>/plan.md      the layers and windows, and why each exists
  specs/NNN-<slug>/tasks.md     the rows, one per task
  package.shspec                shploingus.recipe/3 source (windows + rows + ballots)
  package.cells                 the drawing the package declares (shploingus.cells/1)
  layers/<id>.layer             one API layer's declaration
  layers/<id>.md                that layer's record, with the four patch sections
  frames/<id>.frame             one cell data frame's declaration
  frames/<id>.md                that frame's record, with the four patch sections
  src/<id>.wgsl                 the layer's own shader, if it has one
  README.md                     how to compile and mount it
```

**The drawing is a declaration too.** `package.cells` states which cell version the package is a
package of — `cells 1` is the Holo Dark raster and `cells 0` is the pre-cells drawing, so *"bring the
old version back"* is one line in a package. **The line is read where the package is mounted**
(`Mount::load_package` reads the folder it is about to adopt), so a package that arrives while the
shell runs selects its drawing with the same act that adopts its arrangement; a package written
under the older declaration header (`shploingus.cells/0`) is refused by
`voxel_client::shell::cells::declare` with the file and the line it was written on — before anything
is adopted, so the arrangement and the drawing already on screen are untouched. A package with no
declaration is a declared absence, not a refusal: the version in force stays.

**A layer is a sibling artifact the recipe names in a `row`**, never a new column in the recipe:
`R-64.1` says the schema is closed, and a document that requires a field the schema does not have is a
refusal the build surfaces (M10.1). `voxel-layers` checks the join — a declared layer the package
names by no row is refused by file and id — and it renders each `.layer` together with its `.md` as
one human-readable block, which is the recipe M10.2 asks for.

**A frame is a sibling artifact on the same terms** — `shploingus.cellframe/1`, named by a `row`, so
the recipe grammar is untouched a second time. It is a grid of `fill` and `run` rows addressed to
**one declared window's content space**: `window` is a window id, the grid must fit that window's
declared extent, and every row must land inside the grid, or the frame is refused by file, id and the
cell that wrote it. A frame the recipe names by no row, a frame whose window the package does not
declare, a frame that draws nothing, an old header, and a record missing one of the four sections are
all refusals `voxel_package::cellframe` makes before the mount adopts anything; `voxel-layers` prints
`frames 1` and the frame's rendered grid, and the shell draws those cells **only** from a frame the
mount graded.

**A frame's address is `(col, row)` inside a window, and there is no world-space form of it.** The
format carries no world position, depth, slice or normal, because a cell frame is a window's content
and the shell's content space is two-dimensional; a world-positioned raster lattice (the map window's
cells placed in 3D) would be a different artifact class with its own declaration, its own grading and
its own address law — not a field added to this one. Documented here so the limit is a decision
rather than a surprise.

**A layer may add a pipeline, a bind group, an entry point or a uniform; it may not write
`shading.wgsl`.** That is `C-33`/`AR15` made mechanical in `voxel_package::layer`: a `shader` naming
the look, or a path outside the package's own `src/`, is refused with the layer's id and the path in
the same sentence.

What the packages are
---------------------

| folder | what it is |
|---|---|
| `alpha-shell/` | the shell's own arrangement — 16 windows, 6 tabs, the console bar entry, 3 pins. `crates/shploingus/build.rs` compiles its `package.shspec` and embeds the bytes, so the game's first display package is its own shell. |
| `horizon-lod/` | the first package with a **layer**: the far ring drawn from the terrain rung's own record, so a planetary horizon is expressible by a package rather than by the streamer alone. |

Reading a package without a GPU
-------------------------------

    cargo run -p voxel-package --bin voxel-layers
    cargo run -p voxel-package --bin voxel-layers -- display-packages/horizon-lod

Exit 0 means every layer in every folder was read, named by its recipe and rendered with all four
sections; exit 1 prints every refusal with the file it was read from. The reading is kept in
`docs/baselines/mount-v2/layer-recipes-2026-10-24.txt`.

These folders **cite** the root constitution (`clean-room/.specify/memory/constitution.md`) rather
than copying it. A package that restates the principles is a second authority, and M10.0 refuses
that.

Not here yet
------------

* `claims.shpkg` (the axis package: claims, pin, reproduce) arrives with M10.3's loader verification;
  nothing writes it today, and a placeholder file would be a claim nobody made.
* `Mount::load_display_package` is M10.3 — the loader's declaration half has landed
  (`Mount::load_package` reads a folder's `package.shspec` **and** its `package.cells`), but the
  package's axis verification, claims and pins are still owed. The shell mounts
  `display-packages/alpha-shell/package.shspec` at boot, after the embedded package that guarantees
  a shell exists; `build.rs` proves the folder and `packages/alpha-shell.shspec` byte-identical on
  every build so the two cannot drift.
