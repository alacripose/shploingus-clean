001 — Draw the far ring from the rung's own record
=================================================

What this is
------------

The first package layer: a `terrain-lod` pass that draws the terrain rung chosen for the horizon.
It is the smallest package that answers the user's requirement — *"the updated rendering package will
need to support LOD terrain at distant horizons for the world's planetary generation"* — without
touching the look.

Why it is a package and not a renderer change
---------------------------------------------

`voxel_stream::horizons` already chooses the rung, and `voxel-client::world` already draws it. What a
package can **express** is what its layers name: a package with no `terrain-lod` layer is a package
for an island, however good the streamer is. The layer is therefore the unit — declared, reviewable,
and refused when it reaches for something it does not own.

What it must do
---------------

* Draw at the rung `horizons` chose, through `WorldPipeline`'s registration surface (M10.1).
* Carry its uniform block as a declaration: `view_proj` (mat4, 64 bytes), `rung` (u32, 4 bytes),
  `rung_colour` (vec4, 16 bytes), bind group 1.
* At the handoff between the near ring and the far one, leave **no hole and no double geometry**.
* Be readable: the pair `.layer` + `.md` renders as one block, byte-stable across two runs.

What it must not do
-------------------

* Write `shading.wgsl`, or name any file outside the package's `src/`. Refused by name with the
  layer's id and the path in the sentence.
* Declare a layer the recipe names by no row. Refused with both files named.
* Choose a colour. The layer carries parameters; the look is the user's.

Acceptance
----------

`voxel-layers -- display-packages/horizon-lod` exits 0 with the rendered block; `voxel-layers` over
all of `display-packages/` exits 0; `cargo test -p voxel-package layer::` is green, including the
refusal of `shading.wgsl`, of a shader outside `src/`, of an unknown key, of a duplicate key, of a
missing key, and of a record without `# Rollback`. The horizon sweep itself is M10.3's drawn witness
and is **not claimed** here.
