---
patch_id: PATCH-029
title: The backwalk of voxel-shapers — every shaper developed from a named source
status: applied
operation: add
target: shploingus/voxel-shapers
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
coverage join named this part `uncovered`. This is the largest part of the world and the one the
record has been describing all along without ever naming a file in it:

> *"Every shaper here is developed from a **named source** — a paper, or a ruling recorded in this
> project — and every one of them is a pure function of …"* — [lib.rs](crates/voxel-shapers/src/lib.rs)

Thirty-seven modules, and they are not one thing: a world is produced by running them in a declared
order over declared fields.

| group | modules |
|---|---|
| the chain and its order | [stages.rs](crates/voxel-shapers/src/stages.rs), [process.rs](crates/voxel-shapers/src/process.rs), [spec.rs](crates/voxel-shapers/src/spec.rs), [voxel.rs](crates/voxel-shapers/src/voxel.rs) |
| solid ground | [terrain.rs](crates/voxel-shapers/src/terrain.rs), [substrate.rs](crates/voxel-shapers/src/substrate.rs), [geology.rs](crates/voxel-shapers/src/geology.rs), [tectonics.rs](crates/voxel-shapers/src/tectonics.rs), [caves.rs](crates/voxel-shapers/src/caves.rs), [erosion.rs](crates/voxel-shapers/src/erosion.rs), [earthworks.rs](crates/voxel-shapers/src/earthworks.rs), [delta.rs](crates/voxel-shapers/src/delta.rs) |
| water and weather | [hydrology.rs](crates/voxel-shapers/src/hydrology.rs), [water.rs](crates/voxel-shapers/src/water.rs), [fluidics.rs](crates/voxel-shapers/src/fluidics.rs), [fluid_state.rs](crates/voxel-shapers/src/fluid_state.rs), [weather.rs](crates/voxel-shapers/src/weather.rs), [atmosphere.rs](crates/voxel-shapers/src/atmosphere.rs), [climate.rs](crates/voxel-shapers/src/climate.rs), [sky.rs](crates/voxel-shapers/src/sky.rs), [clouds.rs](crates/voxel-shapers/src/clouds.rs) |
| life and land cover | [ecology.rs](crates/voxel-shapers/src/ecology.rs), [flora.rs](crates/voxel-shapers/src/flora.rs), [habitat.rs](crates/voxel-shapers/src/habitat.rs), [transition.rs](crates/voxel-shapers/src/transition.rs) |
| the field machinery they share | [fields.rs](crates/voxel-shapers/src/fields.rs), [field_sampler.rs](crates/voxel-shapers/src/field_sampler.rs), [chunk_fields.rs](crates/voxel-shapers/src/chunk_fields.rs), [column.rs](crates/voxel-shapers/src/column.rs), [corpus.rs](crates/voxel-shapers/src/corpus.rs) |

# Acceptance

- Every path named above exists at this pin, and the module list is the crate's own `src/` listing
  rather than a summary of it.
- The two feature rows this part witnesses are quoted with their witness paths, so the claim "this is
  where world generation lives" is checkable rather than asserted.
- The record does not claim a shaper's *physics* is right — it claims the shapers are here, counted,
  and witnessed.

# Evidence

- **Measured 2026-10-10**: 49 `.rs` files, **25,062 lines**, **339 `#[test]`** attributes; 30 `pub`
  items declared by [lib.rs](crates/voxel-shapers/src/lib.rs).
- The census's row: `crate\tvoxel-shapers\tactive\tactive\tactive\tactive\tactive\trefused`.
- The gap, quoted: `crate-gap voxel-shapers axis=documented state=refused — 37% of voxel-shapers's
  source over 50 of 50 covered file(s) is comment.`
- Feature rows, quoted: `feature world_generation active — witness present at
  crates/voxel-shapers/src/voxel.rs`; `feature stage_cadences active — witness present at
  crates/voxel-shapers/src/stages.rs`.
- The pin is the record's own digest as the gate computed it (`record-sha256 3b4bf114…`).

# Rollback

No bytes change; the tree is identical before and after. The digest pins the reading, and a changed
record makes it stale rather than wrong.

# Not done, and named

- **No `docs/` document defines this part**, on a measured 37 % comment share over fifty covered files.
  Thirty-seven modules with a paper apiece is the largest documentation debt in the tree.
- **A shaper's source is named in a comment, not in a table a gate can read.** *"Developed from a named
  source — a paper, or a ruling"* is the crate's own claim about itself; nothing mechanical checks that
  each module's named source exists in `docs/` or in the record, so a module whose comment cites
  nothing would be indistinguishable from one whose comment cites a ruling. That check is owed and it
  is the axis this crate cannot fail without.
- **The cadence table and the shaper list are held by feature rows and tests, not by one place.** The
  gate reports `stage_cadences` through `stages.rs`; which stages run in what order over what fields is
  still assembled from several files.
