---
patch_id: PATCH-026
title: The backwalk of voxel-model — the world's own vocabulary, recorded
status: applied
operation: add
target: shploingus/voxel-model
base_ref: shploingus-clean@main
base_digest: sha256:3b4bf1146d2ca4a2b36e53e1c0e906fe0b4f6b195408fe3eda33cc5a568db41c
dependencies: []
source: user-directed-conversation
created_at: 2026-10-10
approval: user-directed
live_mutation: false
---

# Change

**This is a backwalk, not an application.** The work below is already in the tree; this record names
it, with the measurements the gate takes of it, because until PATCH-025 ran nothing in the record
named a single file under `crates/voxel-model/`. `patch-coverage` reported it `uncovered`, and this is
the answer to that line.

`voxel-model` is the crate that holds **everything true about the world and nothing about drawing
it** — in its own header's words: *"There is no GPU type, no file handle and no clock"*. It is the
workspace's deepest dependency: every other part either builds on it or is downstream of something
that does.

| what it holds | where |
|---|---|
| the record itself: events, digests, tickets | [chain.rs](crates/voxel-model/src/chain.rs), [ticket.rs](crates/voxel-model/src/ticket.rs) |
| identity that a hash cannot establish | [identity.rs](crates/voxel-model/src/identity.rs), [envelope.rs](crates/voxel-model/src/envelope.rs), [grant.rs](crates/voxel-model/src/grant.rs) |
| the decision layer's vocabulary | [laya.rs](crates/voxel-model/src/laya.rs), [axis.rs](crates/voxel-model/src/axis.rs), [plan.rs](crates/voxel-model/src/plan.rs) |
| refusal as a type rather than a message | [refusal.rs](crates/voxel-model/src/refusal.rs) |
| materials, declared and generated | [material.rs](crates/voxel-model/src/material.rs), [matter/](crates/voxel-model/src/matter) |
| subvoxel geometry, coordinates, conservation | [subvoxel.rs](crates/voxel-model/src/subvoxel.rs), [coord.rs](crates/voxel-model/src/coord.rs), [conservation.rs](crates/voxel-model/src/conservation.rs) |
| the wire format an artifact travels in | [wire.rs](crates/voxel-model/src/wire.rs) |

# Acceptance

- Every path named above exists at the digest this record is pinned to, and the crate is listed in the
  workspace's own members (`[workspace] members` in the root `Cargo.toml`) rather than assumed.
- The crate's size and test mass are the numbers in `# Evidence`, re-derived by the commands quoted
  there, not carried forward from an earlier reading.
- The record does not claim that anything changed: the bytes predate it, and `# Rollback` says what the
  pin means.

# Evidence

- **Measured 2026-10-10**: 49 `.rs` files, **35,209 lines**, **320 `#[test]`** attributes; 21
  `pub` items declared by `src/lib.rs`.
- The census's own row for this part, from the gate report:
  `crate\tvoxel-model\tactive\tactive\tactive\tactive\tactive\trefused` — five axes established, the
  sixth the tree's own prose rule.
- The one gap, quoted: `crate-gap voxel-model axis=documented state=refused — 19% of voxel-model's
  source over 50 of 50 covered file(s) is comment, and no document under docs/ defines it — no heading
  there begins with its name.`
- Feature rows that this part witnesses, from the gate report:
  `event_chain active — witness present at crates/voxel-model/src/chain.rs`; and the
  `materials` table is generated from this crate's declarations by
  [build.rs](crates/voxel-model/build.rs), which refuses the build when a generated table stops
  matching its declaration.
- The pin is the record's own digest as the gate computed it (`record-sha256 3b4bf114…`).

# Rollback

This record changes no bytes, so there is nothing to restore: the tree is identical before and after
it. The digest above pins the *reading* to the record it was taken against, and a record that changes
makes this one **stale rather than wrong**. Undoing the crate itself is not a patch operation — it is
deleting a directory that eight other crates depend on, and this record is deliberately not the vector
for that.

# Not done, and named

- **No `docs/` document defines this part.** The `documented` axis is refused on a measured 19 % of
  comment prose and the absence of a heading that begins with its name; the axis's own reason is that
  prose inside a code artifact is *"documentation written where a reader does not look for it"*. This
  record is a patch, which is a different place again — the owed thing is a section of `docs/`.
- **The materials table and the spectra module are generated, and the generator is a binary in this
  same crate.** A stale record therefore blocks the build of the tool that rewrites it, and the
  escape is a Cargo feature — recorded because it is the shape of every future generated table here.
- **Nothing in this record is a session's memory of building the part.** It is what the gate can see:
  files, lines, tests, cells and gaps. What the builder knew is owed from whoever built it.
