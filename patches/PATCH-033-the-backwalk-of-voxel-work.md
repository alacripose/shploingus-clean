---
patch_id: PATCH-033
title: The backwalk of voxel-work — the budget, the pool and the tick
status: applied
operation: add
target: shploingus/voxel-work
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
coverage join named this part `uncovered`. Its header names its own subject in one line — the mesh
residency it feeds — and it is the crate that decides **how much work happens per tick**, which is the
difference between a world that streams and a world that stalls.

| what it holds | where |
|---|---|
| the tick, and what a tick may spend | [tick.rs](crates/voxel-work/src/tick.rs), [budget.rs](crates/voxel-work/src/budget.rs) |
| the pool: work that can be done in any order | [pool.rs](crates/voxel-work/src/pool.rs) |
| the stream of work, in the order it must happen | [stream.rs](crates/voxel-work/src/stream.rs) |

# Acceptance

- Every path named above exists at this pin, and the crate is a workspace member.
- The record states the part's *purpose* (how much work per tick) as the phrasing its own files use
  (`budget`, `tick`), not as a performance claim.
- Both gaps the census measures are quoted unchanged, including the one the gate refuses to call a
  failure.

# Evidence

- **Measured 2026-10-10**: 6 `.rs` files, **3,880 lines**, **64 `#[test]`** attributes; 4 `pub` items
  declared by [lib.rs](crates/voxel-work/src/lib.rs).
- The census's row: `crate\tvoxel-work\tactive\tactive\tactive\tactive\tinactive\trefused` — `claimed`
  inactive, `documented` refused.
- The gaps, quoted: `crate-gap voxel-work axis=claimed state=inactive — the spec's declaration tables
  (docs/DESIGN-SPEC.md) never name voxel-work. It is **unclaimed**` and
  `crate-gap voxel-work axis=documented state=refused — 39% of voxel-work's source over 7 of 7 covered
  file(s) is comment.`
- The pin is the record's own digest as the gate computed it (`record-sha256 3b4bf114…`).

# Rollback

No bytes change. The digest pins the reading; a changed record makes it stale rather than wrong.

# Not done, and named

- **`claimed` is inactive**: the spec's declaration tables never name this part — yet the scheduler is
  the thing that decides whether a streamed world keeps up, so the missing row is a hole in the spec.
- **The scheduler has no production consumer.** The record already says it
  (`voxel-work::tick` has no production consumer, and the ticked world is not driven by it), and this
  backwalk re-states it as a measured fact rather than repairing it: the client's streaming path and
  this crate's budget are not yet the same path.
- **No `docs/` document defines this part**, on a measured 39 % comment share over seven files.
- **No budget is asserted against a frame time.** `budget.rs` decides how much a tick may spend; nothing
  in the tree measures what a tick *did* spend against the frame it had to fit in, so the number the
  part is built around has no reading behind it anywhere.
