---
patch_id: PATCH-036
title: The backwalk of voxel-audio — rain you can hear, and no census row either
status: applied
operation: add
target: shploingus/voxel-audio
base_ref: shploingus-clean@main
base_digest: sha256:3b4bf1146d2ca4a2b36e53e1c0e906fe0b4f6b195408fe3eda33cc5a568db41c
dependencies: [PATCH-025]
source: user-directed-conversation
created_at: 2026-10-10
approval: user-directed
live_mutation: false
---

# Change

**This is a backwalk, and like PATCH-035 it is written by hand because the instrument cannot see this
part.** `census::Crate::ALL` carries twelve parts and this crate is not one of them, so no
`patch-part` line can be printed for it. Its whole contract is one sentence:

> *"Rain you can hear: **pure synthesis, no device.**"* — [lib.rs](crates/voxel-audio/src/lib.rs)

The "no device" half is the part's own testability claim: synthesis produces samples, and a device
plays them, so everything worth testing is on the synthesis side of that line.

| what it holds | where |
|---|---|
| the whole part, one file: synthesis and no device | [lib.rs](crates/voxel-audio/src/lib.rs) |

# Acceptance

- The path named above exists at this pin, and the crate is in the workspace's member list in the root
  [Cargo.toml](Cargo.toml).
- The record states plainly that **no census row exists** rather than omitting the row silently.
- The measurements below are re-derivable by the commands in `# Evidence`.

# Evidence

- **Measured 2026-10-10**: 1 `.rs` file, **431 lines**, **8 `#[test]`** attributes; 5 `pub` items
  declared by [lib.rs](crates/voxel-audio/src/lib.rs).
- **No census row exists for this part**, and no `crate-gap` line: the twelve-part crate table does not
  name it, so the six axes are unanswered rather than refused.
- The workspace member list is the authority: `Cargo.toml`'s `[workspace] members` names
  `crates/voxel-audio`.
- The pin is the record's own digest as the gate computed it (`record-sha256 3b4bf114…`).

# Rollback

No bytes change. The digest pins the reading; a changed record makes it stale rather than wrong.

# Not done, and named

- **This part is not measured by the census** — the same blind spot as PATCH-035, and the same owed
  change (`CRATE_COUNT`, the crate table and the gate's shape check).
- **The smallest crate in the workspace, and nothing consumes it.** 431 lines and eight tests with no
  reader anywhere in the tree means the honest question is the census's own `claimed` question — *either
  a feature row is owed for it, or it is a crate this project does not need* — and the answer has not
  been written down. Muting a game is a decision; so is keeping a synthesiser nothing calls.
- **"No device" is a claim about a boundary that only exists in the source's own organisation.** One
  file means there is no module boundary to enforce it; nothing mechanically prevents a device handle
  from being added beside the synthesis in the same file.
