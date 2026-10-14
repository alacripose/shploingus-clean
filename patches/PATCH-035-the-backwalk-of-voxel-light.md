---
patch_id: PATCH-035
title: The backwalk of voxel-light — a workspace member with no census row
status: applied
operation: add
target: shploingus/voxel-light
base_ref: shploingus-clean@main
base_digest: sha256:3b4bf1146d2ca4a2b36e53e1c0e906fe0b4f6b195408fe3eda33cc5a568db41c
dependencies: [PATCH-025]
source: user-directed-conversation
created_at: 2026-10-10
approval: user-directed
live_mutation: false
---

# Change

**This is a backwalk, and it exists because of the instrument's blind spot.** PATCH-025's coverage
join iterates `census::Crate::ALL` — twelve parts — and this crate is **not one of them**, so the join
cannot report on it at all. It is a workspace member with source, tests and a contract:

> *"**The light field: full-spectrum radiometry over declared bands.**"* — [lib.rs](crates/voxel-light/src/lib.rs)

| what it holds | where |
|---|---|
| the light field, over bands rather than channels | [lib.rs](crates/voxel-light/src/lib.rs) |
| the spectra those bands are sampled from | [spectra.rs](crates/voxel-light/src/spectra.rs) |

This is the one record in the backfill that is not an answer to a `patch-uncovered` line, because the
line cannot be printed for it: a part outside the census is outside the instrument. That is stated here
rather than left to be discovered, which is the whole reason this record is hand-written.

# Acceptance

- The paths named above exist at this pin, and the crate is in the workspace's member list in the root
  [Cargo.toml](Cargo.toml) — which is the only place in the tree that says this part exists.
- The record does **not** quote a census row, because there is none: it says so, which is the honest
  equivalent of the `not-measured` line every other gate reading carries.
- The measurements below are re-derivable by the commands in `# Evidence`.

# Evidence

- **Measured 2026-10-10**: 3 `.rs` files, **906 lines**, **17 `#[test]`** attributes.
- **No census row exists for this part.** The census's crate table is fixed at twelve
  (`census::CRATE_COUNT`), and this crate is not in it — so no `crate\tvoxel-light\t…` line and no
  `crate-gap` line is printed for it by the gate, and the six axes the other parts answer are
  unanswered rather than refused.
- The workspace member list is the authority that this part exists: `Cargo.toml`'s `[workspace]
  members` names `crates/voxel-light`.
- The pin is the record's own digest as the gate computed it (`record-sha256 3b4bf114…`).

# Rollback

No bytes change. The digest pins the reading; a changed record makes it stale rather than wrong.

# Not done, and named

- **This part is not measured by the census.** The tree's own completeness check — six axes per crate —
  covers twelve crates while the workspace has sixteen members. Four parts
  (`voxel-light`, `voxel-audio`, `voxel-mcp`, `voxel-von`) are outside it, and adding them is a change
  to `CRATE_COUNT`, the crate table and the gate's shape check rather than a line in a patch. Until
  that lands, a part can be **unmeasurable and unnoticed at the same time**, which is the failure mode
  the census exists to prevent.
- **Whether this crate belongs in the light path with `voxel-appearance`'s sky is not asserted
  anywhere.** `voxel-appearance` carries `sky.rs` and this crate carries the band vocabulary; the seam
  between them is a question no record has answered, and the two names read as if they were one thing.
- **No `docs/` document defines this part** — the same absent heading the other parts are refused on,
  except that this one is not even refused: nothing measures it.
