---
patch_id: PATCH-021
title: The MCP read surface, and the gate's verdict served live
status: applied
operation: add
target: shploingus/mcp-surface
base_ref: shploingus-clean@main
base_digest: sha256:b69b66ce6e8a835ec6ce073d8256b7fa035d640ad4b9b0e45c66a6321a368fd1
dependencies: [PATCH-018, PATCH-020]
source: user-directed-conversation
created_at: 2026-10-09
approval: user-directed
live_mutation: false
---

# Change

The gate has measured this tree since the beginning and printed its findings into a build report
nobody reads, because **a refusal that can only be read in a terminal did not happen**. This patch
gives the gate a read surface: `crates/voxel-mcp`, an MCP server over stdio that serves what the gate
already computes and adds no measurement of its own.

Six tools, in served order:

| tool | answers |
|---|---|
| `get_work_list` | every census cell that is not `active`, one line each, naming the crate, the axis, the state and the reason |
| `get_conformance` | the verdict — `active` only when nothing is refused and nothing is unchecked |
| `get_census` | the crate-by-axis table the build gate prints |
| `get_patch_set` | the records, what is applied, what is owed, **and the target each one declares** |
| `get_ownership` | given a path: the crate it is in, and every record that names it |
| `get_test_bloat` | how much of the workspace is test lines, and where they live |

`get_ownership` is the tool the request was for — *"so the agent will know exactly where the code
we're using belongs"*. It searches **both halves of a record**, because they answer different
questions: `target` names the *system* (`shploingus/test-bloat`) and the body names the *artifact*.
The first draft read `target` alone and a test caught it: it reported **0 records** for
`crates/voxel-package/src/bloat.rs`, a file PATCH-017 plainly changed.

Protocol: JSON-RPC 2.0, one object per line, `2025-06-18` (upstream's constant, read rather than
invented). Notifications are never answered; an unknown **method** is `-32601`; a line that is not
JSON is `-32700` and **the stream continues**; an unknown **tool** returns `isError: true` and lists
the tools that exist. No tool writes anything.

# The verdict, served live

Driven as a client drives it — `target/debug/voxel-mcp.exe` on stdin, from the last recorded build:

```
{"id":2,...,"text":"path\tcrates/voxel-package/src/bloat.rs\ncrate\tvoxel-package\n
 patch\tPATCH-017\tapplied\tshploingus/test-bloat\npatch\tPATCH-019\tapplied\tshploingus/gate-timing\n
 ownership\t2 record(s) name this path\n"}
{"id":3,...,"text":"patch-set-records\t20\npatch-set-applied\t9\npatch-set-owed\t11\n
 patch-set-unevaluable\t0\npatch-set-dangling\t0\n..."}
{"id":4,...,"text":"conformance\trefused\nconformance-refused\t12\nconformance-unchecked\t0\n
 conformance-inactive\t11\nconformance-tested\t12\n"}
```

**`conformance refused`, 12 refusals across 12 crates.** That is the retrofit's first honest number
served over the wire: this tree does not conform, and the twelve refusals are the work list. It is
not a build failure yet — and the next section says exactly why that choice is not mine to take.

# What this makes possible, and the one decision it forces

- **Non-conformance can now be rejected rather than printed.** The verdict exists as a value
  (`conformance` / `refused` / `unchecked` / `inactive`), so a door can ask it. Making the **build
  door fatal** is one line in [build.rs](crates/shploingus/build.rs) — and that line immediately
  fails every build in this tree, because 12 refusals are real. That is the retrofit working as
  upstream ruled it should (*"the retrofit audit is expected to refuse"*), but it stops all other
  work until either the refusals are fixed or **recorded as accepted**, which is upstream's own
  shape: record the refusal once, then refuse only what is not on the record.
- **Documentation can be separated from code, measurably.** The `documented` axis already refuses
  every crate that carries prose in a `.rs` file with no document defining it, and
  `get_work_list` now serves the list with the comment share per crate (19 %–41 %). Moving prose into
  `docs/` is a measurable job with a served work list, not a style preference.
- **Debloating has a number and a location.** 51,782 test lines inside production modules, 152 of 227
  files carrying an inline module, largest single module 2,847 lines — and `get_test_bloat` serves
  it. A split can now be proposed per file against a measurement.
- **The patch format is the address book.** Every record declares a target and names its artifact,
  so `get_ownership` answers "where does this belong" for any file — which is what the format was
  adopted for (PATCH-018) rather than a second use of it.

# Acceptance

- The server builds and its ten protocol tests pass, including the transcript cases: notification
  without a reply, `-32601` on an unknown method, `-32700` on a bad line with the stream surviving,
  and `isError` naming the tools on an unknown tool.
- `get_ownership` finds a record by its **body**, not only by `target`, and a test pins both the found
  and the zero case.
- Every tool's output is the gate's own `key\tvalue` lines, so a tool call and the build report
  cannot disagree — the rule C-53.3 already states for the panel.
- No tool writes: the surface is read-only by construction (there is no write path in the crate).
- `serde_json` is declared **once**, in the workspace and in this crate alone, so the crates that
  carry no external dependency still carry none.

# Evidence

- `cargo test --offline -p voxel-mcp` → **10 passed, 0 failed** (protocol.rs).
- `cargo metadata --offline` → resolved `serde_json v1.0.151`, `itoa`, `zmij` offline.
- The live session: `logs/mcp-session.txt` (four replies, `exit=0`, empty stderr).
- The gate report the readings agree with:
  `target/debug/build/shploingus-1b006e32dd437c39/out/alpha-shell.gate` (PATCH-020).

# Rollback

Base bytes: `Cargo.toml` at `shploingus-clean@main`, git blob
`sha256:b69b66ce6e8a835ec6ce073d8256b7fa035d640ad4b9b0e45c66a6321a368fd1`. Rollback is deleting
`crates/voxel-mcp/`, the `crates/voxel-mcp` member line, and the `serde_json` workspace row; the
`Cargo.lock` entries for `serde_json`/`itoa`/`zmij` follow automatically. Nothing existing was
modified: the gate's measurements are untouched, so reverting removes the read surface and not the
refusals it was reporting.

# Not done, and named

- ~~**The build door is not fatal.** The verdict is served and not enforced; the decision and its
  consequence are stated above rather than taken.~~ **Closed after this patch.** The decision and its
  consequence are now both taken: `crates/shploingus/build.rs` refuses the build on any rejected or
  unchecked cell, with the refusals named, and the twelve measured `documented` refusals are declared
  accepted on `conformance/accepted-refusals.txt` — printed on every build rather than hidden by the
  clean exit. "Reject non-conformance" is now true of the build and not only of the read surface.
- **No write tool, no admission, no proofs.** Upstream's `Refusal`/`Admission`/`Proof` record kinds
  are still absent here, so a refusal is served but not filed on the chain.
- **`get_census` and `get_work_list` are slow** — they walk the tree (805 covered files, the corpus
  and the docs) and are not cached between calls. The measurement is right; the cost is unmeasured.
- **No client is configured.** Nothing in this tree's shell, editor or CI calls the server yet, so
  the surface is verified by transcript rather than by a real client.

## Recipes

```recipe
segment: tooling
axis: evidence_grade
ticket: T-0954
site: crates/voxel-mcp/src/lib.rs:766
what: keep `get_ownership` reading both the record's `target` and every path its body names
because: the first draft read `target` alone and answered 0 records for crates/voxel-package/src/bloat.rs, a file PATCH-017 plainly changed
check: cargo test -p voxel-mcp --test protocol -> failures 0
witness: crates/voxel-mcp/src/lib.rs
expects: 0
requires: cargo check --workspace
```
