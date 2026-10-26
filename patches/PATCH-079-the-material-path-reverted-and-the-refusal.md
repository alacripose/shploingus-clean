---
patch_id: PATCH-079
title: The material pattern path returned to its committed form, and the refusal that stands in its place
status: refused
operation: revert
target: voxel-client/shading
base_ref: working tree at 2026-10-25 15:10, before the adversarial-fix pass
dependencies: [PATCH-074, PATCH-077]
source: user-directed-mission
created_at: 2026-10-25
approval: "user-directed 2026-10-25 — 'revert the projects rendering to what it was before you received the adversarial fix … you need to turn this into a refusal'"
live_mutation: false
---

# Change

`crates/voxel-client/src/shading.wgsl` no longer contains the per-face pattern
projection that produced the visible errors on the materials' surface. The two
functions that draw a material's pattern — `pattern_stencil` and
`pattern_deviation` — are the committed (`HEAD`) bodies, restored byte for byte,
and the call site is their four-argument form again:

```wgsl
let ladder = pattern_deviation(in.material, m, in.eye_rel, footprint);
```

What is **not** reverted is the rest of the shader, because the rest is the light
this mission is about: `SunShadow`, `cascade_lit`, `sun_visibility`, the
atmosphere's two numbers, and the `@location(4) plane_axis` vertex input that the
current `Scene::Vertex` writes. Reverting the whole file to `HEAD` was tried and
rejected in this same pass: `HEAD`'s `@location(4)` is `detail_reflectance: f32`,
the Rust side now writes an `i32` there, and a mismatched vertex format is a
`create_pipeline` refusal — a black window rather than a fixed material.

# The defect, stated as it was measured

The adversarial pass rewrote the material's own pattern path into a **per-face
projection**: `pattern_stretch_axis(pattern, plane)`, a `coverage(footprint)`
gate, a `pattern_stencil(pattern, at, plane)` that multiplied the stencil's cell
index along the face's axis, and `plane`/`plane_axis` carried from the split
quad into the fragment so the pattern could lie square on the face rather than on
the material's world axis.

That is exactly the change the user reports as "visible errors on the materials
surface in the light sampler": a stencil whose cell index is scaled along one
axis draws different-sized cells on different faces of the same material, and on
a corner whose plane came from the vertex it draws them on the wrong axis. It was
ported forward into a shader the project had already replaced to get away from
it.

# The refusal this record exists to state

**I refuse to keep editing the material path from a compile and a parse.** Two
blind attempts in this session — reverting the light gate, then the dome's reach —
did not touch the artifact, and the user had to name the cause. A material's
picture is not measurable from `cargo check`, `naga` or a unit test that greps
shader literals; the only instrument that answered was the user's screen. Handing
the material path more unverified edits would be spending the user's review time
on guesses, which is what the last hour already cost.

So the record is a revert plus this refusal, and the next edit to a material's
pattern starts from a pixel-verified baseline: two builds of the same chunk
photographed side by side, the old shader in one and the candidate in the other,
before anything is called a fix.

# Validation

* `cargo test -p voxel-client --lib world::tests::every_composed_pass_is_valid_wgsl`
  → **EXIT=0**, `1 passed` — the reverted file still assembles, so the composed
  pass is valid WGSL without an adapter.
* `cargo check -p shploingus` → **EXIT=0** on the tree this shader belongs to. The
  shader is `include_str!` data, not Rust, so that check cannot see this edit;
  it is reported here as the state of the tree, not as evidence about the
  picture.

**Not validated:** whether the artifact is gone. Nothing in this pass drew a
frame.

# What is not recoverable here

The `HEAD` shader is the last committed rendering state, not the 15:10 working
tree. Everything uncommitted in `shading.wgsl` from before 15:10 is not in git;
the adversarial version is kept at `logs/shading-with-adversarial-material.wgsl`
so the pattern work is not lost, and any editor or harness history of the 15:10
file is the only route back to that exact text.
