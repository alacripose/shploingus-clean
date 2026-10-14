---
patch_id: PATCH-027
title: The backwalk of voxel-noise — purity as the contract, and the crate that cannot refuse
status: applied
operation: add
target: shploingus/voxel-noise
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
coverage join named this part `uncovered`. Smallest part in the workspace, and the one with the
strongest contract:

> *"No state, no tables, no streaming RNG: every function here is a **pure function of `(seed,
> position)`**. That is not an implementation detail — it is …"* — [lib.rs](crates/voxel-noise/src/lib.rs)

| what it holds | where |
|---|---|
| the fractal field and its octaves | [fractal.rs](crates/voxel-noise/src/fractal.rs) |
| the noise families | [noise.rs](crates/voxel-noise/src/noise.rs) |
| the coordinate contract | [point.rs](crates/voxel-noise/src/point.rs) |
| the seed's own derivation, with no carried state | [rng.rs](crates/voxel-noise/src/rng.rs) |

Purity is what makes the rest of the tree checkable: a world generated twice from the same seed must
produce the same digests, and that claim is only as good as this crate's refusal to keep state.

# Acceptance

- Every path named above exists at this pin, and the crate is a workspace member.
- The purity claim is not asserted here as prose: it is the property the crate's own tests exercise,
  and the count of them is in `# Evidence`.
- The two gaps the census measures for this part are quoted unchanged, including the one this record
  does **not** repair.

# Evidence

- **Measured 2026-10-10**: 5 `.rs` files, **775 lines**, **23 `#[test]`** attributes, 4 `pub` items in
  [lib.rs](crates/voxel-noise/src/lib.rs).
- The census's row: `crate\tvoxel-noise\tactive\tactive\tactive\tinactive\tinactive\trefused` —
  `moduled`, `tested` and `named` established; `refusing` and `claimed` inactive; `documented`
  refused.
- The gaps, quoted:
  `crate-gap voxel-noise axis=refusing state=inactive — voxel-noise names no RefusalKind. A crate that
  cannot refuse returns a plausible value where the record says it should decline; whether this one
  should is a question the record has not answered, so none is asserted here` and
  `crate-gap voxel-noise axis=claimed state=inactive — the spec's declaration tables
  (docs/DESIGN-SPEC.md) never name voxel-noise. It is **unclaimed**: not mis-conformant, and not a
  failure — a thing nobody has claimed yet.`
- The pin is the record's own digest as the gate computed it (`record-sha256 3b4bf114…`).

# Rollback

No bytes change: the tree is identical before and after this record. The digest pins the reading, and
a record that changes makes this reading stale rather than wrong.

# Not done, and named

- **`refusing` is inactive and the gate explicitly refuses to call that a fault**: *"whether this one
  should is a question the record has not answered"*. A pure function of `(seed, position)` has no
  failure to report — its inputs are total — so the honest answer may be that this crate is the one
  part for which the axis is genuinely inapplicable. That answer is owed as a **ruling**, not as a
  refusal added to make a column green.
- **`claimed` is inactive**: the spec's declaration tables never name it. Either a feature row is owed
  or the crate is not needed — the gate's own two options.
- **No `docs/` document defines it**, on a measured 33 % comment share over six covered files.
- **The contract itself is prose.** *"Every function here is a pure function of `(seed, position)`"* is
  written in a comment; nothing mechanical forbids a `static` or a clock being introduced later. An
  axis that checks purity by reading the source would be the mechanical version, and it does not exist.
