# Plan, 2026-10-10: the clean room, the api layer, and every line's place

**The user's direction, verbatim:** *"the backwalks are a good framework now, I just want to be sure
that every line of code is backwalked and made sure that it has a place where it belongs, especially
the rendering and doc work, I want to consume all of the old format shit into api layers and have all
the code go through the axis laya von module system so we can regenerate a real clean room version of
the game that has its documentation and moddable api layer for players and agents."*

This document is the program that sentence describes. It is written down because a direction that
lives only in a conversation is a direction nobody can check — the same reason PATCH-017 … PATCH-037
each end with a *not done, and named* list.

## Where the tree actually is, measured

From the gate report of 2026-10-10 (`placement-*` lines, `patch::placement`, PATCH-037):

| kind | files | lines | generated | placed | unplaced |
|---|---|---|---|---|---|
| code (`.rs` under `crates/`) | 239 | 154 538 | 2 files, 7 385 lines | 142 files, 91 418 lines | **95 files, 55 735 lines** |
| shader (`.wgsl`) | 6 | 1 860 | — | **0** | **6 files, 1 860 lines** |
| document (`.md` under `docs/`) | 26 | 25 349 | — | 3 files, 13 761 lines | **23 files, 11 588 lines** |

And the parts, from PATCH-025's join: **12 of 12 parts are named by a record, 16 records name no path
in this tree at all** (`PATCH-001` … `PATCH-016`, targeting `hyperskill/*`).

The document row is the reading **before this file landed**: writing it took documents 26 → 27, placed
3 → 4 and placed lines 13 761 → 13 882 (PATCH-037's evidence). Every other row is unchanged by it.

*"Placed"* means a record names the file. It does not mean the file is understood, and the report says
so in its own `placement-is-not` line. The number that matters for the sentence above is the last
column: **69 183 lines of code, shader and prose that no record names.**

## Lane A — every written line placed, rendering first

The work list is printed on every build, biggest first, so this lane is "take the top line and finish
it" repeated. The order, and why:

1. **Rendering.** `crates/voxel-client/src/shell/mount.rs` 6 696 → `renderer.rs` 2 860 →
   `motion.rs` 1 676 → `skylight.rs` 1 663 → `shell/window.rs` 1 622 → and all six `.wgsl` files.
   This is 1 860 lines of shader plus roughly 12 000 lines of client code, it is the block the user
   named, and it is the code a player sees and a modder will touch. Today **the shaders are placed by
   nothing at all** — the only kind of file in the tree with zero records.
2. **The world's own vocabulary.** `voxel-model/src/shaper.rs` 1 861,
   `civilian/gov/city_reference.rs` 1 823, `civilian/agency/action.rs` 1 576, `materials/schema.rs`
   1 412 — the civilian/government layer, which is the largest unlabelled system inside the model.
3. **The game's own binary.** `shploingus/src/main.rs` 2 425: the thing that runs the game has no
   record of its own.
4. **The documents.** 23 files, starting with `gun-rem-math-skill.md` 2 657, `HYPERGRILLING.md`
   2 635, `LEDGER-COMPACT.md` 1 034, `MODS-MATERIALS.md` 556, `SHELL-PORT.md` 328. Two of those are
   *plans* (`PLAN-2026-10-08-cloud-cubes.md`, `superpowers/plans/…remeshing.md`) — a plan is a record
   of intent and belongs in the same accounting as everything else.

**The rule this lane holds:** a file is placed by a record that says what it is *for*, its evidence
and its rollback — not by a record that mentions it in passing. That distinction is already printed
(the record ids travel with every part line), and the backwalks of PATCH-026 … PATCH-036 are the
template.

## Lane B — the old formats become API layers

*"Consume all of the old format shit into api layers."* What is actually old here, named:

- **Sixteen records for another system** (`PATCH-001` … `PATCH-016`, all targeting `hyperskill/*`) sit
  in this tree's patch directory, count as eleven *owed* patches, and name no artifact in this
  workspace. `patch-unnamed-records 16` prints them on every build. They are a format that this tree
  inherited and never consumed.
- **Formats that describe instead of carrying.** `packages/` holds 526 files at a mean of 246 bytes —
  a package that describes a few UI rows rather than carrying the bytes that run (PATCH-020 measured
  this; upstream's rule is *"a package carries the work"*).
- **A shell port and a materials-mod document** (`SHELL-PORT.md`, `MODS-MATERIALS.md`) that describe
  an interface in prose where an API layer should declare it.

**The lane now has a measured scope, from `cargo ingest` (PATCH-039).** The records are not the
problem: **54 distinct record ids exist across all four drives and this tree carries every one of
them** — its own 38 plus sixteen `hyperskill/*` records that every generation vendored again
(`shploingus` 4 copies, `shploingus-game` 6, `planet-shploingus` 2, the prototype, `AWA3D`).
`ingest-only-there` is 0 everywhere: no unrecorded decision sits on another drive. **What is
unrecorded is material** — 296 documents and ~19 000 source files, including 2 979 code files and
287 documents in the 47 vendored mod repositories. That is what "consume the old formats" has to
answer to, over 10 GB, on a volume at 99 %.

**But none of it has to move, on the user's ruling of 2026-10-10.** Consume here means consume **in
place by reference**: the material is read where it sits and the api layer *declares* a row that
cites a path and a digest as its witness, instead of carrying the bytes. What lands in this tree is
the declaration alone. And the old trees are **superseded generations of this same one** — they are
deleted as the canon settles, not migrated: *"the shploingus clean will have cleaned itself out."*
Pruning a source afterwards stales a witness; it does not lose a decision. So lane B has no import
step and no landing decision — only the question of *which* documents and files become declared
rows, and in which lane.

**The lane's destination is the surface, not a new format**: `voxel_package::apidoc` and
`voxel_package::bridge::TOOLS` already declare what a caller can ask for, and `voxel-mcp` already
serves eight of those rows. So the work is to **declare** each old description as a row with a witness
— the description stays where it is — and to let the served surface be the only interface, which is
what makes it reachable by a player's mod and by an agent through the same door. And because the row
cites its source rather than copying it, the surface is a *reading* of the old material, not a
second copy of it.

## Lane C — every module on the axis, judged through laya and von

*"Have all the code go through the axis laya von module system."* The pieces exist and are not yet
wired to each other:

- `voxel_model::axis` and the census's six axes say what a part *is* (`named`, `moduled`, `tested`,
  `refusing`, `claimed`, `documented`).
- `voxel_model::laya` is the decision layer's vocabulary, `voxel_package::oracle` is the calibrated
  structural judge reached through it, and `voxel_package::ballot` (PATCH-023) is the format a
  question is written in.
- `voxel-von` is the classifier that answers a ballot on CPU.

**And the calibration the user named lives here**: nothing in the tree can yet tell a *decision* from
a *description* — `ingest` can say a foreign file is `docs` and nothing more. The training material
is already on the disk: the sixteen shared `hyperskill/*` ids are documented at five different life
stages across five trees (`proposed`, `designed`, `verified-internal`, `applied-internal`,
`approved`), so each stage is a labelled example of what a decision looks like on its way through a
chain. Calibrating `laya` against those, through `ballot` + `voxel-von`, is what turns "REAL work
into the new branch" from an intention into an instrument.

What is missing is the join, and it is the same shape as the two joins already built this week:
`patch-coverage` joined records to parts, `placement` joined records to files, and this lane joins
**modules to subjects** — a declared subject row per module, a witness per row, an answer per row
through one seam. The audit's own precedent stands: a cell nobody measured is `unchecked`, and an
unchecked cell is never a pass.

## Lane D — regenerate a clean-room version

*"So we can regenerate a real clean room version of the game that has its documentation and moddable
api layer for players and agents."* This is the lane the other three exist to make possible, and it
should not start before they do: regenerating from a record whose coverage is 40 % reproduces the
holes at machine speed.

The shape it takes, given what is already here: the record (record + axis declarations + ballots +
witnesses) is the declaration; `voxel_package::recipe` already compiles a declared source into a
package and refuses it by name; `voxel_package::compendium` already compresses the documents to the
format an evaluator reads. So a clean room is: **the tree that the record implies**, built from the
declarations, with the documentation and the moddable API generated rather than written beside the
code — and every difference between what the declarations produce and what the tree contains is a
finding, not a merge conflict.

## Order, and what blocks what

| step | lane | blocked by |
|---|---|---|
| 1 | A, rendering | nothing — it is a work list and a template |
| 2 | A, documents | nothing |
| 3 | B, the sixteen records | a ruling on whether they are owed here (TODO §9 row 26) |
| 4 | B, packages into the API layer | step 5's surface |
| 5 | C, modules to subjects | a declared subject vocabulary per module — the largest single design decision left |
| 6 | D, the clean room | steps 1–5 |

**Nothing in this plan is a promise to build it**, and every number in it was measured rather than
estimated. The plan's own file is named by PATCH-037, which is how it takes its place on the record
rather than sitting beside it.
