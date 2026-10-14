---
patch_id: PATCH-025
title: The patch coverage join — the game names the parts it has no record for
status: applied
operation: add
target: shploingus/patch-coverage
base_ref: shploingus-clean@main
base_digest: sha256:3b4bf1146d2ca4a2b36e53e1c0e906fe0b4f6b195408fe3eda33cc5a568db41c
dependencies: [PATCH-018, PATCH-020]
source: user-directed-conversation
created_at: 2026-10-10
approval: user-directed
live_mutation: false
---

# Change

**The user's instruction, verbatim: *"use the game to patch up the game by finding the parts that
were not given patches."*** The tree could not answer it. `voxel_package::patch` read the records and
reported what was applied, owed, dangling and unevaluable — but nothing joined *the records* to *the
parts of the tree*, and `voxel-mcp`'s `get_ownership` asked the question one path at a time, which is
a question nobody asks about a file they have not already opened. So the hole was invisible: **nine of
the twelve parts the census measures were named by no record at all**, and no line of any report said
so.

This patch is that join.

- **`patch::coverage(&PatchSet, &[Crate]) -> Coverage`** ([patch.rs](crates/voxel-package/src/patch.rs)):
  per part, the ids of every record that names a path inside it, or `uncovered`. It reads
  `census::Crate::ALL` and `Crate::path()` rather than a second hand-written list of crates, so the
  parts it measures are the parts the census measures — one vocabulary, two questions.
- **The join is over `crates/<part>/`, never over a bare name.** A record with `voxel-model` in a
  table has written a *word*; a record naming `crates/voxel-model/src/chain.rs` has named an artifact.
  This is `census::heading_defines`'s distinction (*"a mention is not a definition"*) applied to
  records, and it is why `PatchRecord::named_paths` is extracted once in `parse` and read by both the
  coverage join and the MCP `get_ownership` half that searches the body.
- **The gate prints it on every build**, as the section `# what has a patch`: `patch-set-*`, then
  `patch-coverage-*`, one `patch-part` line per part with its record ids, and `patch-uncovered` naming
  every part no record names. The MCP patch tool answers from the same function, so a served answer and
  a built answer cannot differ.
- **It is evidence and not a door.** A part with no record is a part whose history was never written
  down — a hole in the record, not a broken build. A gate that refused here would refuse every build
  until the backfill landed, which is how a gate teaches everyone to route around it.

**The format also convicted this session's own last record, and that is repaired here.** The reader
reported `patch-set-unevaluable 1`: **PATCH-024** carried `# Change`, a verification section and a
not-done list, and **none of `# Acceptance`, `# Evidence`, `# Rollback`** that the format requires.
Its body now has the four sections in the required names. That is the first time this tree's own
format has refused a record written by the tooling that enforces it, which is the point of it.

# Acceptance

- `patch::coverage` prints, per part, the **ids** of the records naming a path inside it — not a count:
  a count of one does not tell a reader whether the one record changed the part or merely read it.
- A set that did not read reports `patch-coverage not-measured` with its reason and **prints no
  counts and no parts**.
- Every id a part claims exists in the set, and a join that covered nothing fails its own test rather
  than passing vacuously.
- The gate's report carries the section, and `voxel-mcp`'s patch tool carries the same lines from the
  same function.

# Evidence

- `cargo test -p voxel-package --lib patch::` — **10 passed**, including
  `the_real_patch_set_is_joined_against_the_real_parts`, which reads the real `patches/` directory and
  asserts the join is not vacuous.
- **The first measured run, 2026-10-10** (this is the finding, unedited):
  `patch-set-records 24`, `patch-set-applied 13`, `patch-set-owed 11`, `patch-set-unevaluable 1`,
  `patch-set-dangling 0`; `patch-coverage-parts 12`, `patch-coverage-covered 3`,
  `patch-coverage-uncovered 9`.
  Covered: `voxel-package` (8 records), `shploingus` (4), `voxel-client` (1 — PATCH-020, a *reading*,
  so the client is named and not recorded). Uncovered: **voxel-model, voxel-noise, voxel-mesh,
  voxel-shapers, voxel-stream, voxel-instrument, voxel-appearance, voxel-work, voxel-ui.**
  `patch-unnamed-records 16` — every record from PATCH-001 to PATCH-016 names no path in this tree at
  all, because they target `hyperskill/*`.
- `target/debug/build/shploingus-*/out/alpha-shell.gate`, section `# what has a patch`; the gate is
  exit 0 with it, and `cargo check -p voxel-mcp` builds with the served half.
- The pin above is the record's own digest as the gate computed it at this writing
  (`record-sha256 3b4bf114…`), the same convention PATCH-020 used: a reading is pinned to the digest of
  the record it read, so a changed record makes the reading **stale rather than wrong**.

# Rollback

The join is a read and changes no bytes: the tree is byte-identical before and after, and rolling it
back means deleting `patch::coverage`, the gate's `# what has a patch` section and the MCP lines that
print it. The digest above makes this reading stale rather than wrong the moment the record changes —
which is why it is a field of the record rather than a sentence in it.

# Not done, and named

- **The census measures twelve parts and the workspace has sixteen members.** `voxel-mcp`,
  `voxel-von`, `voxel-light` and `voxel-audio` have **no census row**, so this join cannot report on
  them — the instrument's own blind spot, and the reason PATCH-035 and PATCH-036 exist as hand-written
  reads. Adding them is a change to `CRATE_COUNT`, the crate table and the gate's table-shape check,
  not a line in this patch.
- **The nine backfills (PATCH-026 … PATCH-034) are written from this round's measurements, not by the
  sessions that built the parts.** A record written by the builder carries what the builder knew; this
  one carries what the gate can see, which is files, lines, tests, cells and gaps. The difference is
  named in every one of them rather than smoothed over.
- **`get_ownership` searched the raw text and now reads the record's extracted paths.** The two
  agreed on the current set by inspection, not by construction; they are one list of paths now, but the
  test that holds them together is the coverage join's, not a second one of their own.
- **Nothing here refuses.** The three uncovered records the tree would most like to fail on — the nine
  parts and the sixteen unnamed records — are printed, counted and left standing, because the
  alternative is a gate that every build has to route around.
