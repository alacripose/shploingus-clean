---
patch_id: PATCH-037
title: The placement join — every line's place, measured per file, with rendering and the documents named
status: applied
operation: add
target: shploingus/placement
base_ref: shploingus-clean@main
base_digest: sha256:3b4bf1146d2ca4a2b36e53e1c0e906fe0b4f6b195408fe3eda33cc5a568db41c
dependencies: [PATCH-025]
source: user-directed-conversation
created_at: 2026-10-10
approval: user-directed
live_mutation: false
---

# Change

**The user's instruction: *"I just want to be sure that every line of code is backwalked and made
sure that it has a place where it belongs, especially the rendering and doc work."*** PATCH-025 made
*parts* answerable — twelve crates, three covered. That was one level too coarse: the work lands in
**files**, and a part can be "covered" by a record that names one file inside it while 6 696 lines
beside it are in no record at all.

This patch adds `patch::placement(&PatchSet, &Path) -> Placement` and prints it in the gate's section
and on the MCP surface:

- **Three kinds, because the tree's own words are three**: `code` (`.rs` under `crates/`), `shader`
  (`.wgsl` — *"the rendering work"*, and nothing else in the tree accounts for it), and `document`
  (`.md` under `docs/`).
- **Generated is not unwritten.** Two files are a generator's product
  (`materials/generated.rs`, `materials/spectra.rs`, both refused by
  [voxel-model's build script](crates/voxel-model/build.rs) when they stop matching their
  declaration). They are declared in `patch::GENERATED` with their generator and counted apart, so the
  work list is work.
- **A file is placed when a record names it — or names a directory above it**, and the ids are printed
  beside the counts, because a record that *read* a file and a record that *changed* it are different
  facts and the reader has to be able to see which.
- **Every unplaced file is printed, biggest first, and not truncated.** A shortened work list reads as
  a complete one.
- **The instrument's own first bug, found by running it**: `named_paths` extracted only `crates/`, so
  the whole documentation set read as placed in no record, including the three documents that are
  named. It now reads `crates/` and `docs/`, with a test for each.

The program this measures toward — the API layers, the axis/laya/von module system, and the clean-room
regeneration — is written down in
[docs/PLAN-2026-10-10-clean-room-and-the-api-layer.md](docs/PLAN-2026-10-10-clean-room-and-the-api-layer.md)
rather than left in a conversation.

# Acceptance

- Every file of every kind is counted in exactly **one** of generated, placed or unplaced, and every
  line with it — asserted per kind, on the real tree as well as on a fixture tree.
- A set that did not read prints `placement not-measured` with its reason and **no counts**.
- The walk enumerates crates by looking, not by reading `Cargo.toml`, so a member with no census row
  is still measured; symlinks are not followed.
- The gate and the MCP surface both print it from the same function.

# Evidence

- `cargo test -p voxel-package --lib patch::` — **14 passed**, including
  `the_real_tree_is_placed_against_the_real_records`, which holds the counts-add-up invariant on the
  real tree.
- **The first measured run, 2026-10-10, unedited** (`placement-*` lines in the gate report):
  `code files 239 lines 154538 generated-files 2 generated-lines 7385` /
  `placed-files 142 placed-lines 91418` / `unplaced-files 95 unplaced-lines 55735`;
  `shader files 6 lines 1860` / `placed-files 0` / `unplaced-files 6 unplaced-lines 1860`;
  `document files 26 lines 25349` / `placed-files 3 placed-lines 13761` /
  `unplaced-files 23 unplaced-lines 11588`.
- **The rendering block, as the instrument lists it**: `shell/mount.rs` 6 696, `renderer.rs` 2 860,
  `motion.rs` 1 676, `skylight.rs` 1 663, `shell/window.rs` 1 622, `shell/arrangement.rs` 1 594,
  `world.rs` 1 399, `cloud_cubes.rs` 1 338, `cloud_field.rs` 1 291, `pattern.rs` 1 208,
  `shell/console.rs` 1 201, `hud.rs` 1 147, `shading.rs` 1 114 — **and all six shaders, of which
  `shading.wgsl` is 1 205 lines.**
- **The documents with no record**: `gun-rem-math-skill.md` 2 657, `HYPERGRILLING.md` 2 635,
  `LEDGER-COMPACT.md` 1 034, `missing_atmosphere_deepseek.md` 841, `MODS-MATERIALS.md` 556,
  `superpowers/plans/2026-10-07-hyper-greedy-block-level-remeshing.md` 435,
  `PLAN-2026-10-08-cloud-cubes.md` 344, `SHELL-PORT.md` 328, and fifteen more.
- `cargo check -p shploingus` — exit 0, report **885 lines**; `cargo check -p voxel-mcp` — builds.
- The pin is the record's own digest as the gate computed it at reading time
  (`record-sha256 3b4bf114…`). Writing this record and the plan document moves the digest to
  `109ac069…`, so the pinned reading is **stale rather than wrong** — the same rule the gate applies
  to proofs, and the reason the digest is a field of the record rather than a sentence in it.
- The plan document took its own place in the accounting when it landed: document files 26 → 27,
  placed 3 → 4, placed lines 13 761 → 13 882.

# Rollback

The instrument is a read: it changes no bytes of the tree it measures, and deleting `patch::placement`
plus the two call sites restores the report's previous shape. The digest pins the reading, and a
changed record makes this reading stale rather than wrong.

# Not done, and named

- **55 735 lines of code, 1 860 lines of shader and 11 588 lines of documentation are in no record.**
  This patch measures that and does not repair it. The order the plan takes is rendering first, because
  it is the largest contiguous block, it is the one the user named, and its files are the ones a
  player and a modder will touch.
- **Placement is not understanding.** A file named by a record is *placed*; a file named by a record
  that only read it is placed by a reader. This is stated in the report's own `placement-is-not` line
  because the alternative — reading a placed count as a complete record — is exactly the overclaim
  every other gate line in this tree exists to prevent.
- **Nothing refuses.** Placement is evidence: a gate that refused on 55 735 unplaced lines would
  refuse every build until a season of backwalks landed.
- **The work list has no owner.** It is printed on every build, which is what makes it visible; nothing
  yet schedules it, so the numbers move only when someone opens the report and picks the top line.
