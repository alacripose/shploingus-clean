---
patch_id: PATCH-034
title: The backwalk of voxel-ui — a design system in one file, with four axes to answer
status: applied
operation: add
target: shploingus/voxel-ui
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
coverage join named this part `uncovered`. It is a leaf crate with the most unusual shape in the
workspace: **one file, 2,684 lines**, and its own header states the rule it is built on —

> *"The design system: **Material Design 1** with the **Human Interface Guidelines** overlay, and the
> four-bucket rule for every constant in it."* — [lib.rs](crates/voxel-ui/src/lib.rs)

The four-bucket rule is the part's whole discipline: every constant belongs to one of four kinds, and a
constant that belongs to none is the defect the rule exists to catch. A design system whose numbers
are not typed cannot be restyled, only rewritten.

# Acceptance

- The path named above exists at this pin, and the crate is a workspace member.
- The part's stated purpose is quoted from its own header rather than paraphrased, so the four-bucket
  rule can be checked against the source.
- **All four gaps** the census measures for this part are quoted unchanged, including the three it
  reports as `inactive` *states* rather than refusals — a one-file leaf crate is legal, and the census
  says so in its own words.

# Evidence

- **Measured 2026-10-10**: 1 `.rs` file, **2,684 lines**, **41 `#[test]`** attributes; 41 `pub` items
  declared by [lib.rs](crates/voxel-ui/src/lib.rs) — the highest declaration count per file in the
  workspace.
- The census's row: `crate\tvoxel-ui\tactive\tinactive\tactive\tinactive\tinactive\trefused` — the
  lowest row in the tree, and the only one with three inactive cells.
- The gaps, quoted: `crate-gap voxel-ui axis=moduled state=inactive — voxel-ui declares no pub mod.
  That is legal — a leaf crate can be one file — so this is reported as inactive, not refused`;
  `axis=refusing state=inactive — voxel-ui names no RefusalKind`; `axis=claimed state=inactive — the
  spec's declaration tables (docs/DESIGN-SPEC.md) never name voxel-ui`;
  `axis=documented state=refused — 34% of voxel-ui's source over 2 of 2 covered file(s) is comment.`
- The pin is the record's own digest as the gate computed it (`record-sha256 3b4bf114…`).

# Rollback

No bytes change. The digest pins the reading; a changed record makes it stale rather than wrong.

# Not done, and named

- **`moduled` is inactive and the census explicitly declines to call it a fault** — *"a leaf crate can
  be one file"*. At 2,684 lines it is also the largest single file in the workspace, so the honest
  reading is neither: the axis is satisfied by a state, and the file is at the size where the next
  screen makes the question real.
- **`claimed` is inactive**: the spec's declaration tables never name this part. A design system the
  game cannot be styled without has no row saying it is needed.
- **No `docs/` document defines this part**, on a measured 34 % comment share over two files — and the
  four-bucket rule is exactly the kind of thing a document can state once instead of a comment stating
  it once per constant.
- **Nothing checks the rule mechanically.** The header declares four buckets; no axis, test or gate
  asserts that every constant in the file is in one of them. That is the one check this part most
  needs and does not have.
