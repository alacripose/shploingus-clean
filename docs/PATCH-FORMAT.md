# The patch record's recipe block

A record is prose plus, optionally, **acts**. An act is a fenced block opened with ```` ```recipe ````,
and it is the only part of a patch a machine compiles.

````markdown
```recipe
segment: shell
axis: evidence_grade
ticket: T-0954
site: crates/voxel-client/src/shell/mount.rs:2378
what: expose the dynamic loader's entry
because: the loader exists and nothing outside the pane can list what it can run
check: cargo test -p voxel-client --lib shell::mount::tests::the_loader_reads_the_path_it_is_given_and_refuses_rather_than_serving_the_last_package -> failures 0
witness: crates/voxel-client/src/shell/mount.rs
expects: 0
requires: cargo check --workspace
```
````

**Every key is required except `requires`, which repeats.** The parser refuses an unknown key, a
repeated key, an empty value, a missing key, an `expects` that is not a count, a `site` that is not
`path:line`, and a block with no closing fence. The defects are carried in `PatchRecord.problems`, so
an act that cannot compile makes the record `not_evaluable` instead of passing as prose.

| key | what it is |
|---|---|
| `segment` | one of the calibration matrix's segments (`shell`, `ui`, `game`, `world`, `pattern`, `axis`, `tooling`) |
| `axis` | an axis name (`evidence_grade`, `grade`, …), looked up in `AxisKind::ALL` |
| `ticket` | the ticket the act is filed under |
| `site` | `path:line`, one-based, workspace-relative |
| `what` | the edit, in one statement |
| `because` | why, with the numbers in it |
| `check` | the command, and the movement it must print |
| `witness` | where the measurement is kept afterwards |
| `expects` | the count the check must reach |
| `requires` | a gate the tree demands; repeat the line for each |

Two blocks on one `(segment, axis)` in one record are refused: an act has one answer per cell.

**Compiling.** `get_patch_recipes` (`{patch, ticket}`) parses the record, maps segment and axis
through the matrix's own parsers, builds `Fix`/`Site`, and calls `Matrix::recipe`. A block for a cell
the walk did not measure is refused by name (`recipe-refused`); a block with defects prints them
(`recipe-problem`) and is not compiled. The reply is the matrix's recipe line — the same five lines
`get_axis_recipe` serves — plus one `requires` line per gate and a `compiled N of M` tail.

**What this is not.** It is not a second store and not a second score: the star and the tier come from
the measured cell, the gate commands come from the record, and nothing here writes to the chain.
Records written before this block existed carry no recipes; they are records, not acts, until they are
migrated.

**Frontmatter is not the door.** `patch::parse` reads the body's blocks before it looks at the
frontmatter, so an act is visible in a record that cannot say who it is. The missing frontmatter stays
a named problem — the record is `not_evaluable`, its `target` and `dependencies` are empty, and
`get_patch_recipes` prints `patch-unevaluable <record> <reason>` beside the compiled line instead of
implying that nothing was written.

**One axis, because the walk measured one.** `get_axis_matrix` reports **eight cells — seven
segments and the artifact format — 0 unchecked**, all on `evidence_grade`. Every act in the corpus
names that axis because it is the only one the walk measured; an act on another axis is refused as
`missing_cell`, and no record here pretends otherwise.

## Corpus tally, 2026-10-23

Counted by driving every record through the live tools, not estimated: one `voxel-mcp` session — a
`get_patch_recipes` call for each `patches/*.md` (its `PATCH-NNN` prefix as `patch`) and one
`get_patch_set` — reading the returned `recipe`, `compiled`, `patch-problem` and `patch-unevaluable`
lines. Re-run those calls to re-take it; nothing here is a reading of the source.

| measure | count |
|---|---|
| records under `patches/` | **73** |
| with readable frontmatter | **57** |
| carrying at least one ```` ```recipe ```` block | **21** |
| acts declared | **24** |
| acts compiled | **24** |
| clean: frontmatter valid and every act compiled | **15** |
| act compiled while the record is unevaluable | **6** |
| records declaring no act | **52** |

The fifteen clean records are PATCH-021, PATCH-023, PATCH-025, PATCH-028, PATCH-042, PATCH-063,
PATCH-064, PATCH-065, PATCH-066, PATCH-067, PATCH-068, PATCH-069, PATCH-070, PATCH-072 and PATCH-073;
PATCH-070 carries two acts and the rest one.
The six that compile while unevaluable are PATCH-046, 048, 049, 053, 057 and 061: their acts are
readable and compiled, and `patch-unevaluable` names what keeps the record out of the `applied`/`owed`
counts — missing frontmatter for the first five, and for PATCH-061 the duplicate axis declaration
(`recipe 3: axis/evidence_grade is declared twice in this record`), which still compiles 3 of 3 and is
not clean.

> **Update 2026-10-25.** PATCH-061's third block was removed because it named `Von::answers` rather
> than a routing act that exists; its first two blocks were re-sited to `patch_recipes` and
> `load_package`. It now compiles 2 of 2 and is clean. The table and the six-record list above are the
> 2026-10-23 reading and are kept as that reading, not re-taken.

### The backlog, by patch id

**Records that declare no act (52):** PATCH-001 to PATCH-020, 022, 024, 026, 027, 029 to 041, 043, 044,
045, 047, 050 to 052, 054 to 056, 058 to 060, 062, 071.

Sixteen of those still have no frontmatter: PATCH-045 to PATCH-060. Their acts would now be read if
they carried one, but until the frontmatter lands they are `not_evaluable` — no `target`, no
`dependencies`, no `status` — and are counted outside `patch-set-applied` and `patch-set-owed`.

`get_patch_set` prints the same count from the same set: `patch-set-records 73`, `patch-set-recipes
24`, `patch-set-applied 41`, `patch-set-unevaluable 21`, `patch-set-dangling 0`.
