001 — Plan: one layer, three uniforms, one join
===============================================

The package is deliberately one layer. A second layer would add a second question before the first
one is answered, and the question here is whether the package format can carry a terrain pass at all.

The pieces, and where each lives
--------------------------------

| piece | file | why there |
|---|---|---|
| the layer's declaration | `layers/LOD-0001.layer` | the machine part: pipeline, shader, entry, bind group, uniforms, rung, order, requirements. |
| the layer's record | `layers/LOD-0001.md` | the human part: what changed, what must hold, where the measurement is kept, how to undo it. |
| the package's name for it | `package.shspec`'s `row LOD-0001` | the recipe has no layer rows (R-64.1), so the row is the join. |
| the shader | `src/LOD-0001.wgsl` | the layer owns its pass; the look stays in `shading.wgsl` (C-33). |
| the reader | `voxel_package::layer` | parses, refuses and renders — one renderer, no second copy of the fields. |
| the command | `cargo run -p voxel-package --bin voxel-layers` | the caller M10.2 needs; the baseline quotes it. |

The uniform block
-----------------

| name | type | bytes | why |
|---|---|---|---|
| `view_proj` | `mat4<f32>` | 64 | the camera's matrix, passed rather than read from a global. |
| `rung` | `u32` | 4 | which rung this draw is; the shader lifts by it rather than by a constant. |
| `rung_colour` | `vec4<f32>` | 16 | the rung's declared colour, so the pass reads its record. |

**Declared, not inferred.** The size is in the file because a size this module computed from a type
string would be a second, unchecked answer. Laying the buffer out is the client's job, and M10.3 owns
it.

The order of the work
---------------------

1. The grammar and its refusals (`layer.rs`), with the look refused by name.
2. The renderer over the pair (`.layer` + `.md`), byte-stable.
3. The folder: this package, its row, and the alpha-shell source folder beside it.
4. The command, and the baseline record it writes its output into.
5. **Not here:** `Mount::load_display_package` (M10.3) and the horizon sweep's drawn witness. The
   package is source until the loader lands, and this plan does not claim a pixel it has not drawn.

Measurements
------------

* `package.shspec`: 1 `row`, 0 windows; `Recipe::parse` accepts it.
* `LOD-0001.layer`: 1 `layer`, 3 `uniform`, `lod 0`, `order 10`, `requires none`.
* `cargo test -p voxel-package layer::` — the grammar's refusals and the byte-stability witness.
* `docs/baselines/mount-v2/layer-recipes-2026-10-24.txt` — the command, the output, and the reading.
