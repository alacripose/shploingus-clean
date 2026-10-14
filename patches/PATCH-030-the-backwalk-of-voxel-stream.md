---
patch_id: PATCH-030
title: The backwalk of voxel-stream — the link between the shapers and the mesher
status: applied
operation: add
target: shploingus/voxel-stream
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
coverage join named this part `uncovered`.

> *"This crate is the **link** between the shapers and the mesher. Above it, a world is a `WorldSpec`
> and a set of papers; below it, a world is a list of …"* — [lib.rs](crates/voxel-stream/src/lib.rs)

| what it holds | where |
|---|---|
| what a shaper chain is asked for, per chunk | [world_source.rs](crates/voxel-stream/src/world_source.rs) |
| what is resident, and for how long | [resident.rs](crates/voxel-stream/src/resident.rs) |
| how far the eye can see, in levels rather than metres | [horizons.rs](crates/voxel-stream/src/horizons.rs) |
| the light a column carries | [light.rs](crates/voxel-stream/src/light.rs) |

The header's "above it / below it" split is the seam this crate exists to hold: the shapers must not
know about residency, and the mesher must not know about `WorldSpec`.

# Acceptance

- Every path named above exists at this pin, and the crate is a workspace member.
- The residency behaviour is named as a fact about *this* crate with a witness the gate can check, not
  as a claim about the client's memory.
- The census row and the gap are quoted unchanged.

# Evidence

- **Measured 2026-10-10**: 5 `.rs` files, **2,829 lines**, **48 `#[test]`** attributes; 4 `pub` items
  declared by [lib.rs](crates/voxel-stream/src/lib.rs).
- The census's row: `crate\tvoxel-stream\tactive\tactive\tactive\tactive\tactive\trefused`.
- The gap, quoted: `crate-gap voxel-stream axis=documented state=refused — 41% of voxel-stream's
  source over 6 of 6 covered file(s) is comment, and no document under docs/ defines it.` The highest
  comment share of the nine parts this round recorded.
- Feature row, quoted: `feature save_load active — witness present at
  crates/voxel-stream/src/resident.rs`.
- The pin is the record's own digest as the gate computed it (`record-sha256 3b4bf114…`).

# Rollback

No bytes change. The digest pins the reading; a changed record makes it stale rather than wrong.

# Not done, and named

- **No `docs/` document defines this part**, on a measured 41 % comment share over six files — and it
  is the part whose *seam* most needs a document, because its whole job is a boundary between two
  crates that must not know about each other.
- **`save_load` is witnessed by a resident-set file and nothing exercises a browser-restart round
  trip.** The gate's row says the witness is present; it does not say a world written and read back is
  the world that was written, and that is a test rather than a record.
- **The horizon policy and the work budget are decided in different crates.** `horizons.rs` decides how
  far, `voxel-work::budget` decides how much, and nothing asserts that the two agree — a level the
  horizon asks for and the budget never grants is a hole neither part's tests would see.
