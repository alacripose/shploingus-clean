001 — Tasks: the package's rows, one per task
=============================================

- [x] Declare the layer: `layers/LOD-0001.layer`, all eight keys, three uniforms.
- [x] Write the record with the four sections: `layers/LOD-0001.md`.
- [x] Name the layer in the package's recipe: `row LOD-0001` in `package.shspec`.
- [x] Give the layer its own shader: `src/LOD-0001.wgsl`, vertex and fragment stages.
- [x] Add the grammar and its refusals: `crates/voxel-package/src/layer.rs` — wrong header, unknown
      key, duplicate key, missing key, a shader outside `src/`, `shading.wgsl`, a record missing a
      section, a layer its recipe names by no row.
- [x] Add the renderer: `api-layer · <id> · pipeline …` plus the block and the record's four sections.
- [x] Add the command: `cargo run -p voxel-package --bin voxel-layers`.
- [x] Record the reading: `docs/baselines/mount-v2/layer-recipes-2026-10-24.txt`.
- [ ] Register the layer with `WorldPipeline` — M10.3, with the loader.
- [ ] The horizon sweep's drawn witness (the eye at the world's scale, no hole at the handoff) —
      M10.3. **Not claimed by this package.**

How each finished task is checked
---------------------------------

    cargo test -p voxel-package layer::
    cargo run -p voxel-package --bin voxel-layers -- display-packages/horizon-lod

The first is the grammar and the renderer; the second is the same answer a reader gets. The unfinished
tasks have no check here on purpose: a check that cannot run is not evidence, and M10.3 names the
witnesses it owes.
