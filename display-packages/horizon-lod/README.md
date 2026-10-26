horizon-lod — the far ring as a package layer
=============================================

The first display package with a **layer**. It exists to answer one question mechanically: can a
package draw the horizon of a planetary world, or only an island's near ring?

What it declares
----------------

```
package.shspec        row LOD-0001 — the layer named by the package (the recipe has no layer rows)
layers/LOD-0001.layer pipeline terrain-lod, shader src/LOD-0001.wgsl, entry fs_horizon,
                      bind group 1, view_proj:mat4<f32>@64, rung:u32@4, rung_colour:vec4<f32>@16,
                      lod 0, order 10, requires none
layers/LOD-0001.md    the layer's record: Change / Acceptance / Evidence / Rollback
src/LOD-0001.wgsl     the layer's own vertex and fragment stages
```

`voxel_stream::horizons` chooses the rung; this layer draws it. The layer does not choose a look and
does not write `shading.wgsl` — it carries parameters, and colour is the user's (`C-33`, `AR15`).

Why the recipe names it in a row
--------------------------------

`shploingus.recipe/3` has windows, rows and ballots and **no layer rows**, and `R-64.1` says a schema
that would have to change shape is refused rather than extended. So the layer is a sibling artifact
the recipe names by id in a `row` (M10.1). `voxel_package::layer::read_package` refuses a declared
layer no row names, and `voxel-layers` renders the pair together.

Read it
-------

    cargo run -p voxel-package --bin voxel-layers -- display-packages/horizon-lod

The output is the rendered recipe — `api-layer · LOD-0001 · pipeline terrain-lod · …` — followed by
the four sections of `layers/LOD-0001.md`. Two runs are byte-identical, which is M10.2's acceptance
for a human-readable recipe.

What is not here
----------------

* **`claims.shpkg`** — the axis package's claims/pin/reproduce file is M10.3's. Nothing writes one
  today, and a placeholder would be a claim nobody made.
* **The loader.** `Mount::load_display_package` (M10.3) does not exist yet, so this folder is a
  source package that a human reads and a test checks; it does not draw in the game today.
