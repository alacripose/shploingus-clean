---
patch_id: PATCH-028
title: The backwalk of voxel-mesh — what is drawn, from the world's own voxels
status: applied
operation: add
target: shploingus/voxel-mesh
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

> *"This crate answers one question — what is drawn — from one input, the world's own voxels. It
> contains no GPU type, no window and no clock: a `Mesh` …"* — [lib.rs](crates/voxel-mesh/src/lib.rs)

| what it holds | where |
|---|---|
| the mesh itself, and greedy merging | [mesh.rs](crates/voxel-mesh/src/mesh.rs) |
| the octree the world is held in | [octree.rs](crates/voxel-mesh/src/octree.rs) |
| refinement: what a level of detail means | [refine.rs](crates/voxel-mesh/src/refine.rs) |
| chunks and their boundaries | [chunk.rs](crates/voxel-mesh/src/chunk.rs) |
| the source a mesh is produced from | [source.rs](crates/voxel-mesh/src/source.rs) |
| picking: the ray and what it hits | [raycast.rs](crates/voxel-mesh/src/raycast.rs) |

The header's claim — *no GPU type* — is what lets this crate be tested without a device, which is why
its test mass is high for its size.

# Acceptance

- Every path named above exists at this pin, and the crate is a workspace member.
- `octree`, `mesh`, `chunk`, `source` and `raycast` are the module set `ls` reports for
  `crates/voxel-mesh/src/`, so the table above is the crate's own layout rather than a summary of it.
- The census row and the one gap are quoted unchanged.

# Evidence

- **Measured 2026-10-10**: 7 `.rs` files, **4,409 lines**, **70 `#[test]`** attributes; 6 `pub` items
  declared by [lib.rs](crates/voxel-mesh/src/lib.rs).
- The census's row: `crate\tvoxel-mesh\tactive\tactive\tactive\tactive\tactive\trefused` — five axes
  established, only `documented` refused.
- The gap, quoted: `crate-gap voxel-mesh axis=documented state=refused — 37% of voxel-mesh's source
  over 8 of 8 covered file(s) is comment, and no document under docs/ defines it.`
- The pin is the record's own digest as the gate computed it (`record-sha256 3b4bf114…`).

# Rollback

No bytes change. The digest pins the reading; a changed record makes it stale rather than wrong.

# Not done, and named

- **No `docs/` document defines this part**, on a measured 37 % comment share over eight covered
  files.
- **The mesher's output is not checked against the appearance crate's view of the same faces.** The
  client draws patterns where the greedy mesh has no geometry — the parity that
  [PATCH-024](patches/PATCH-024-the-build-tree-size-guard.md)'s own summary called out as a client
  test — but nothing in *this* crate asserts anything about it, and the seam between "what is drawn"
  and "how it is shaded" is held by the client's tests alone.
- **The octree's residency policy is elsewhere.** `voxel-work` owns the budget and `voxel-stream` owns
  what is resident; this crate holds the structure, and which of the two decides a level is a question
  no record has yet answered in one place.
