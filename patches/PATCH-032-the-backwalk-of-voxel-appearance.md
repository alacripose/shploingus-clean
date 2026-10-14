---
patch_id: PATCH-032
title: The backwalk of voxel-appearance — the OpenPBR / NPR equation, half recorded
status: applied
operation: add
target: shploingus/voxel-appearance
base_ref: shploingus-clean@main
base_digest: sha256:3b4bf1146d2ca4a2b36e53e1c0e906fe0b4f6b195408fe3eda33cc5a568db41c
dependencies: []
source: user-directed-conversation
created_at: 2026-10-10
approval: user-directed
live_mutation: false
---

# Change

**This is a backwalk.** The work is in the tree; this is its record, written because PATCH-025's
coverage join named this part `uncovered`. It is the crate that turns a material into light, and the
one with the largest comment share in the tree — 48 % of its source is prose:

| what it holds | where |
|---|---|
| the BRDF: OpenPBR's surface response | [openpbr.rs](crates/voxel-appearance/src/openpbr.rs), [response.rs](crates/voxel-appearance/src/response.rs) |
| the NPR half: stylisation over the physical one | [npr.rs](crates/voxel-appearance/src/npr.rs), [wrap.rs](crates/voxel-appearance/src/wrap.rs) |
| the sky and the light it sends down | [sky.rs](crates/voxel-appearance/src/sky.rs), [detail.rs](crates/voxel-appearance/src/detail.rs) |
| fluids, as an appearance problem | [fluid.rs](crates/voxel-appearance/src/fluid.rs) |
| composition of the layers, and tone mapping | [composition.rs](crates/voxel-appearance/src/composition.rs), [tonemap.rs](crates/voxel-appearance/src/tonemap.rs) |
| where each constant came from | [provenance.rs](crates/voxel-appearance/src/provenance.rs) |

`provenance.rs` is the part of the tree that answers *"which bytes did this come from"* for the
appearance constants — the same question the gate asks of the whole build.

# Acceptance

- Every path named above exists at this pin, and the crate is a workspace member.
- The OpenPBR / NPR split is stated as the crate's own layout (`openpbr.rs` beside `npr.rs`) rather
  than as an architectural claim this record is not placed to make.
- The census row and the gap are quoted unchanged, including the comment share, which is the highest
  of the nine parts this round recorded.

# Evidence

- **Measured 2026-10-10**: 11 `.rs` files, **5,181 lines**, **107 `#[test]`** attributes; 10 `pub`
  items declared by [lib.rs](crates/voxel-appearance/src/lib.rs).
- The census's row: `crate\tvoxel-appearance\tactive\tactive\tactive\tactive\tactive\trefused` — five
  axes established, only `documented` refused.
- The gap, quoted: `crate-gap voxel-appearance axis=documented state=refused — 48% of
  voxel-appearance's source over 12 of 12 covered file(s) is comment, and no document under docs/
  defines it.`
- The pin is the record's own digest as the gate computed it (`record-sha256 3b4bf114…`).

# Rollback

No bytes change. The digest pins the reading; a changed record makes it stale rather than wrong.

# Not done, and named

- **No `docs/` document defines this part**, on a measured **48 %** comment share over twelve files.
  Half the crate is documentation written where the project's own axis says a reader does not look.
- **The NPR half is the part that is known incomplete.** The record's own summary names the owed
  channel list (tint masks, scrolling UVs, emissive masks, rim lighting, outlines, subsurface
  scattering) as the user's ruling; this backwalk measures what is present and does not add a channel.
- **The GPU parity claim lives in the client's tests, not here.** The shader and this crate are checked
  against each other by `voxel-client`'s pixel tests, which means a change to the appearance crate can
  be *correct here and wrong on screen* with the gate still green — the seam is real and one-sided.
