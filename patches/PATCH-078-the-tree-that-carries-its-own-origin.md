---
patch_id: PATCH-078
title: The tree that carries its own origin — a being with an identity and a body at its own corner, and the block tree deleted
status: withdrawn
operation: repair
target: voxel-shapers/gaia + voxel-mesh/placement + shploingus/forest
base_ref: post-PATCH-077 working tree, 2026-10-25
dependencies: [PATCH-063, PATCH-074, PATCH-077]
source: user-directed-mission
created_at: 2026-10-25
approval: "user-directed 2026-10-25 — the tree is the first mob: it is an entity with its own identity and origin, it is registered where its crate says what it is made of, and the block tree it replaces is removed"
live_mutation: false
---

# Withdrawn, 2026-10-25: the files this record describes are not on disk

The working tree was reverted after this record was written, so `voxel-shapers::gaia`,
`crates/shploingus/src/forest.rs` and the `flora.rs` removal **do not exist**. The one
surviving piece is `Mesh::origin` / `Mesh::placed_at` in `voxel-mesh`. Do not apply the
.patch beside this file expecting the state it claims; the tree is still the block tree,
and `docs/trees.md` and `docs/SHAPER-PROFILES.md` record what is actually true and what
the tree mob will be drawn with.

# Change

**A tree is a record with an identity and an origin, and no chunk stores one any more.**

`voxel-shapers::flora` is deleted and `voxel-shapers::gaia` stands in its place. The lattice, the
species choice, the formation and habitat gates and the memo are unchanged — what is gone is the
answer to *"what material is at this voxel"*. `Tree` gained the two things a being needs and a
material rule never had:

| field | what it is | why it is the tree's |
|---|---|---|
| `id: u64` | a digest of the world and the cell | identity comes from the record, never a counter; two calls for one cell answer one id |
| `cell_x`, `cell_z` | the lattice cell it belongs to | a census row and a body both name the same cell |
| `origin_voxel()` | the corner the body stands on | the tree's position, not its chunk's |

**The removal.** `chunk_fields::material_of` above ground is sea or air. `Tree::voxel_at`,
`tree_material_at`, `trees_over_column` and the above-ground band constant are gone; the test that
walked a tree's trunk and crown through the plane is replaced by one that asks the same cells for
wood and expects **zero**. The census example and `face-refined` ask `gaia` for trees.

**The body, and the one field it needed.** `Mesh` carries `origin: [i32; 3]` and `Mesh::placed_at`.
The mesher places every chunk mesh at its chunk's own corner, so `Mesh::digest` and every chunk's
bytes are unchanged; a body is placed at the tree's own corner and may stand across a chunk
boundary without lying about which chunk it is in. `crates/shploingus/src/forest.rs` builds one
body a frame for the cells within three cells of the eye — a trunk of wood and a crown sampled on
the record's envelope with a declared 24 % gap fraction — meshed by the same `Mesher`, placed at
the tree's origin, retained while the cell is in range and **absent** rather than stood in for
while it is not.

**The registration row.** `voxel-shapers/src/lib.rs`'s module table — the crate's own list of what
it is made of — gains `gaia`'s row, citing `docs/GAIA_comprehensive.md` §1–§2. The table had no
`flora` row at all, which is the defect in miniature the removal settles: a subject nobody
declared.

# Refused, and why

The new `voxel-gaia` crate, the `Stand` population, the `Drawing` record and the signed build
attestation from PATCH-043 are **not** here. The request named three responsibilities — the entity
with identity and origin, the registration row, and the removal of the path it replaces — and those
are what landed; a new workspace member also moves the crate roster the build census reads, which
is a slice of its own rather than churn inside this one.

# Evidence

- `cargo check -p voxel-shapers`, `-p voxel-mesh`, `-p voxel-client` (lib and examples), `-p voxel-stream`, `-p shploingus` → **0 errors**.
- `cargo test -p voxel-shapers --lib -- gaia::` → **8 passed**, including
  `a_tree_carries_an_identity_and_an_origin_of_its_own`.
- `cargo check -p shploingus` re-runs the prose census; the explanation for all of this lives in
  `docs/trees.md` §"As landed, 2026-10-25", not in the crate.

# The patch

`patches/PATCH-078-the-tree-that-carries-its-own-origin.patch` — **716 lines, fourteen files**
(two new, one deleted). It is **stacked on PATCH-077**: its base is the tree as PATCH-077 left it,
so 077 then 078 reconstructs this checkout. `patches/PATCH-078-export.py` builds it as a `git diff`
between two scratch commits — the base, and the base with this pass's files copied in and
`flora.rs` removed — and all three proofs were run on the shipped bytes:

1. `git apply --check -p1` into a clean copy of the dumped base → **0**;
2. applied for real with `core.autocrlf=false` → **all fourteen files byte-identical**, `flora.rs` deleted;
3. `git apply --check --reverse -C1 --ignore-whitespace` → **0**, so the `+` side is exactly this tree.

Two things were repaired in the exporter rather than worked around: the base must be the **staged**
bytes (`git show :path`), because `git add` normalises line endings and a base dumped from the
working copy refuses hunks whose context is right; and the nine files that gained an `origin:` line
had to be normalised to LF first, or no patch could reproduce them byte for byte.

# Unproven without an eye on the screen

Whether a crown at 24 % occupancy reads as a tree rather than as a cage; whether one body a frame
is fast enough while walking; whether the crown's 1 m voxels are acceptable at ten metres; and
whether the forest's absence at the edge of the three-cell radius is visible. None of that was run
— the game is live on the user's machine and cargo was not to be disturbed.
