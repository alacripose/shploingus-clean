---
patch_id: PATCH-031
title: The backwalk of voxel-instrument — readings that are four things and no more
status: applied
operation: add
target: shploingus/voxel-instrument
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
coverage join named this part `uncovered`. It is the part that makes the rest of the tree's numbers
*readings* rather than printed claims:

| what it holds | where |
|---|---|
| the sensor: a reading is `{site, material, measured, tick}` and nothing else | [lib.rs](crates/voxel-instrument/src/lib.rs), [stock.rs](crates/voxel-instrument/src/stock.rs) |
| bands: what a spectrum is measured over | [band.rs](crates/voxel-instrument/src/band.rs) |
| the gauge wall: many readings, one surface | [gauge_wall.rs](crates/voxel-instrument/src/gauge_wall.rs) |

`bloat.rs`'s own header cites this crate's rule when declining to be a sensor: *"A byte count is not a
world reading"*. That is the strongest kind of precedent — a later module refusing to widen this
crate's vocabulary rather than widening it.

# Acceptance

- Every path named above exists at this pin, and the crate is a workspace member.
- The record names what this part is *for* in the tree's own two words (`site`, `material`) rather than
  paraphrasing them, so a reader can check the claim against the source.
- The two gaps the census measures are quoted unchanged, including the one about a claim nobody made.

# Evidence

- **Measured 2026-10-10**: 4 `.rs` files, **1,417 lines**, **33 `#[test]`** attributes; 4 `pub` items
  declared by [lib.rs](crates/voxel-instrument/src/lib.rs).
- The census's row: `crate\tvoxel-instrument\tactive\tactive\tactive\tactive\tinactive\trefused` —
  `claimed` inactive, `documented` refused, everything else established.
- The gaps, quoted: `crate-gap voxel-instrument axis=claimed state=inactive — the spec's declaration
  tables (docs/DESIGN-SPEC.md) never name voxel-instrument. It is **unclaimed**` and
  `crate-gap voxel-instrument axis=documented state=refused — 38% of voxel-instrument's source over 5
  of 5 covered file(s) is comment.`
- The pin is the record's own digest as the gate computed it (`record-sha256 3b4bf114…`).

# Rollback

No bytes change. The digest pins the reading; a changed record makes it stale rather than wrong.

# Not done, and named

- **`claimed` is inactive**: the spec's declaration tables do not name this part. This crate is the
  *reason* several other parts can report a measurement instead of a claim, so the missing feature row
  is a hole in the spec rather than evidence the crate is unnecessary.
- **No `docs/` document defines this part**, on a measured 38 % comment share over five files.
- **The sensor vocabulary is closed to `Units | Mass`, and the tree already wanted to widen it once.**
  `bloat.rs` records the temptation (a byte count) and the refusal; a third measurement that does not
  fit `Units | Mass` will meet the same wall, and where it goes instead is a decision no record has
  taken in advance.
