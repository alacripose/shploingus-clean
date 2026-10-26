---
patch_id: PATCH-061
title: The record that compiles into recipes — a machine-readable act block, the matrix's own compile, and the MCP route an agent calls
status: applied
operation: plan
target: voxel-package/patch + voxel-mcp/axis
base_ref: shploingus-clean@main
base_digest: sha256:6c1c7a471f3192e70cc7aaaca3878953de45f143142707f06987ba704148700c
dependencies: [PATCH-042]
source: user-directed-conversation
created_at: 2026-10-23
approval: "user-approved 2026-10-23 — the recipe block, the matrix compile, and the MCP route"
live_mutation: false
---

# Change

**A patch is now a compile target, not only a narrative.** A record may declare `recipe` blocks; each
block names one `(segment, axis)` cell, one `file:line`, what to type there, the check that must move,
the witness, and the gates the tree requires. `patch::recipes` parses them; every defect is carried in
`PatchRecord.problems`, so a malformed act refuses the record rather than passing as prose.

The compile is the matrix's, not a second renderer: `get_patch_recipes` maps the record's segment and
axis through `Segment` and `AxisKind`, builds the matrix's own `Fix`/`Site`, and calls
`Matrix::recipe` — so a block for a cell the walk did not measure is refused by name. The served text
is the calibration matrix's recipe line, unchanged: `# segment=… axis=… star=… tier=… ticket=…`, then
`fix`, `because`, `check`, `witness`.

This patch's two recipes name the surfaces this pass actually exposed: the compile route itself and
the shell's dynamic loader. The third surface the pass named — routing the classifier's answers into
the axis layer — is not in the tree, and it is stated in **Not done** below rather than carried as a
compiled block that points at a function which is not the act.

# Acceptance

- `patch::recipes` returns one entry per block, in file order, each carrying its defects; an unknown
  key, a missing field, a bad `expects`, a site that is not `path:line`, and an unclosed fence are
  each named.
- A record carries its recipes and their problems; two blocks on one `(segment, axis)` are refused as
  two answers to one question.
- `get_patch_recipes` compiles the two blocks below against the measured matrix and prints their
  recipe lines plus every `requires` gate; a patch with no block is refused as prose.
- At the original act, no gate, walk, count pin, dependency or module structure changed. The
  2026-10-25 correction edits `crates/voxel-mcp/tests/axis_route.rs`, so `voxel-mcp`'s missing-docs
  pin moves with that test; its compiler count stays **0**. The only new public surface is
  `PatchRecipe`, `RECIPE_KEYS`, `recipes()`, and one MCP tool.

# Evidence

```
cargo test -p voxel-package --lib patch::     19 passed
cargo test -p voxel-mcp --test axis_route    transcript: get_patch_recipes returns 2 recipe lines
cargo test -p voxel-client --lib shell::mount::tests::the_loader_reads_the_path_it_is_given_and_refuses_rather_than_serving_the_last_package  1 passed
cargo check --workspace                      exit 0
```

`base_digest` is the sha256 of `crates/voxel-package/src/patch.rs` at `HEAD` (`git show
HEAD:crates/voxel-package/src/patch.rs`), the file this record's primary change was read against.

# Rollback

Restore `crates/voxel-package/src/patch.rs` and `crates/voxel-mcp/src/lib.rs` from `HEAD`, delete this
record and `patches/FORMAT.md`; the recipe blocks then parse as ordinary prose fences.

## Recipes

```recipe
segment: axis
axis: evidence_grade
ticket: T-0954
site: crates/voxel-mcp/src/lib.rs:861
what: keep `get_patch_recipes` the only path that turns a record's blocks into recipe lines
because: a second renderer would be a second place a star, a tier or a refusal is spelled
check: cargo test -p voxel-mcp --test axis_route -> failures 0
witness: crates/voxel-mcp/tests/axis_route.rs
expects: 0
requires: cargo check --workspace
requires: cargo test -p voxel-package --lib patch::
```

```recipe
segment: shell
axis: evidence_grade
ticket: T-0954
site: crates/voxel-client/src/shell/mount.rs:2378
what: expose the dynamic loader's entry so a mounted package is listed and run without an editor
because: the loader exists behind the mount and nothing outside the pane can enumerate what it can run
check: cargo test -p voxel-client --lib shell::mount::tests::the_loader_reads_the_path_it_is_given_and_refuses_rather_than_serving_the_last_package -> failures 0
witness: crates/voxel-client/src/shell/mount.rs
expects: 0
requires: cargo check --workspace
```

# Not done, and named

**The classifier seam is not routed into the axis layer.** This record used to carry a third recipe
at `crates/voxel-von/src/lib.rs:91` claiming to route the classifier's answers to `laya` as oracle-rung
evidence on the axis surface. That line is `Von::answers`, the model call, not a routing act:
`voxel-von::judge::decide` returns a `laya::Decision` carrying `EvidenceOrigin::Oracle`, but nothing in
`voxel-package` or `crates/axis` consumes it. PATCH-023's own *Not done* says the decision layer still
reads `oracle::lint` and does not ask the classifier. Wiring it would make the axis layer depend on
the model-bearing `voxel-von` — a dependency PATCH-023 deliberately kept out of the gate — so it is a
separate record with its own dependency decision, not a line in this one. The block was removed rather
than re-sited to a function that is not the act.

# Update 2026-10-25 — the prose route is now part of the compile

The blocks above are still compiled by `get_patch_recipes`, and the MCP route still refuses a record
with no block. What changed is the corpus walk: `patch_recipe_coverage` now calls its derivation for
every record, not only for records with no block, and a record that states `The act is at …` has its
sourced coordinates compared with each declared block. On this tree `PATCH-045` derives an act from
prose and `PATCH-060`'s block matches its prose on segment, site, check and witness — so the compile
is no longer a claim that only a hand-written fence can satisfy.

The record's blocks were also audited against the functions they named on 2026-10-25. Block 1 now points
at `patch_recipes` (line 861), which performs the compile, rather than at `axis_recipe`; block 2 points
at `load_package` (line 2378), the loader's entry, rather than at a line inside `read_the_board`. The
third block was not an act in this tree and is named in **Not done** above instead of being re-sited.

