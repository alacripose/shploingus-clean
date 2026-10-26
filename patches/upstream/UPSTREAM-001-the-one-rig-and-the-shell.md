---
upstream_id: UPSTREAM-001
title: The one rig and the shell — one streamed light, one content space, terrain drawn on first load
status: current
seam: shploingus/light-and-shell
operation: repair
base_ref: shploingus-clean@aef0971
base_digest: sha256:67379c35af2fa29f7c03384d95efb1f315a57ed3630ebde0a64073a881a9460a
consumes: [PATCH-071, PATCH-074, PATCH-075, PATCH-076]
baseline: docs/baselines/mount-v2-shell-cells-2026-10-24/
live_mutation: false
---

# Baseline

Reproduce from the working tree; every number below was captured in
`docs/baselines/mount-v2-shell-cells-2026-10-24/` (`readings-2026-10-24.txt`,
`tests-2026-10-24.txt`, `display-packages-2026-10-24.txt` before the cell frame and
`display-packages-after-2026-10-24.txt` after it).

| reading | command | value |
|---|---|---|
| tree digest | `git ls-files -z \| sort -z \| xargs -0 sha256sum \| sha256sum` | `sha256:67379c35…a9460a` over 3148 files |
| display packages | `voxel-package --bin voxel-layers -- display-packages` | **180 lines**, `sha256:13d93263…996b7c4` (**89 lines / `3f7c9230…` before the cell frame**; both captures are in the baseline folder) |
| frame format | `cargo test -p voxel-package --lib cellframe::` | **11 passed, 0 failed** |
| axis seam | `cargo test -p voxel-axis-verify` | **16 passed, 0 failed**, eight cells all ★5 |
| folder census | `voxel-package --bin voxel-census` | **16 folders, 0 unclassified** |
| shell cells | `cargo test -p voxel-client --lib shell::cells::tests` | **15 passed, 0 failed** |
| cell declaration | `cargo test -p voxel-client --test cell_version_refusal` | **2 passed, 0 failed** |
| clock | `cargo test -p voxel-client --lib clock::tests` | **13 passed, 0 failed** |
| renderer | `cargo test -p voxel-client --lib renderer::tests -- --test-threads=1` | **22 passed, 0 failed** |
| workspace | `cargo check --workspace --all-targets` | exit 0 |

Two readings are the seam's own claims rather than a count:

- a resident 48-record world whose frame lists only its newest 8 records writes **8 of 48** under the
  old upload rule and **48 of 48** under `grew || !filled`, and the first frame is **0 of 1089 pixels**
  different from a fresh rebuild;
- a surface under seven normals reads **bit-identical** in 1-chunk, 8-chunk and 768-chunk worlds at
  four distances, with per-corner bands zeroed and absurd (`chunk: 0e0`, `bands: 0e0`, GPU vs crate
  `3.9e-8`).

# Code

| file | what it owns here |
|---|---|
| `crates/voxel-client/src/world.rs` | `write_quads` — the first-fill rule (`grew \|\| !quad_records_filled`); `write_corners`' same rule; `quad_path()` as the count the reading asserts |
| `crates/voxel-client/src/shading.wgsl` | `fs_main` — one material law: `rig.ambient` + the frame's beam gated by `sun_visibility`, and the water boundary's `lod.y` Fresnel |
| `crates/voxel-client/src/atmosphere.rs` | the look-ahead whose refusal is stated; the `x`/`z` wording of the sampled air |
| `crates/voxel-client/src/shell/cells.rs` | `frames_in` — content-space-only raster (the `inside` test is PATCH-075's); `surface_at`/`raster` take the graded frames and no viewport |
| `crates/voxel-client/src/shell/mount.rs` | `content_space()` — the mask from `window_body`; `frame_space()` — the graded frames placed in their windows' content |
| `crates/voxel-package/src/cellframe.rs` | the `shploingus.cellframe/1` grammar, its refusals, its render and its four-section record |
| `display-packages/alpha-shell/frames/` | `MAP-0001` — the map window's 56 × 22 lattice and contours, declared as a frame |
| `crates/shploingus/src/main.rs` | the frame's one raster call passes `self.shell.frame_space()` |
| `crates/voxel-client/src/clock.rs` | `TimeSeek` — the epoch seek, cubic ease-in-out, ticks untouched |

# Consumption

| patch | status | what it contributes |
|---|---|---|
| PATCH-071 | applied | the cell shell refusal — an old declaration header refused by file and line |
| PATCH-074 | implemented | one streamed light law; the water test restored; the compression pass's index and layer |
| PATCH-075 | implemented | the chrome outside the grid; `surface_in` and `content_space` |
| PATCH-076 | implemented | the cell frame artifact (`cellframe/1`), its mount grading, the format's matrix cell, and the deliberate `display-packages` re-pin |

# Boundaries

- **The per-corner bands are not a light.** They remain in the vertex format; `fs_main` does not
  branch on `light_present`. The frame-global sample is newer than any band the demand queue could
  deliver, which is why every chunk agrees.
- **Faces are half-Lambert.** A side turned away from the Sun keeps half the beam; the ambient is the
  dome's frame-global mean and carries no per-face occlusion. That is `npr::shade`'s declared
  stylisation, asserted by the model tests, and changing it is an appearance decision — not a
  streaming one, and not a second light law in water or LOD.
- **A cell frame is a window's content, not a world position.** `shploingus.cellframe/1` exists,
  is graded at mount and is drawn only inside a declared window's content container; it carries no
  world position, depth or slice. A streamed, world-positioned frame (record + origin upload through
  `Scene`) is still a different artifact class and is not specified here.
- **The renderer suite's parallel run is not stable.** The recorded reading is
  `-- --test-threads=1` (22 passed); parallel runs on this machine produced a 6-failure reading once
  and a > 400 s hang once, stated in the baseline folder rather than rounded into a pass.
- **The shells `cells` format is untouched** beyond the declaration's own reader; clouds, trees and
  mobs are untouched by these four records.

# Rollback

Per patch: PATCH-075's mask (delete `frames_in`, `inside` and `rect_of` — `frames_in` was named
`surface_in` when that patch landed — restore the signatures `surface_at`/`raster` had before it,
remove `Mount::content_space()`, restore the call sites); PATCH-076 by deleting
`voxel-package::cellframe`, `display-packages/alpha-shell/frames/` and its recipe row, the mount's
grading and the matrix's `FORMAT` subject and `format_rows`, and leaving `voxel-package::cells` alone; PATCH-074's upload
rule by restoring `let whole = grew;` and its light law by restoring the two-path shader — the
readings in the baseline folder say which direction each goes.
