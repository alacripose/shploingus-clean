---
patch_id: PATCH-017
title: Measured test-and-bloat stage
status: applied
operation: add
target: shploingus/test-bloat
base_ref: shploingus-clean@main
base_digest: sha256:8ec257e3dbcb64702f83660849055720c566af4b7b59b998168b4a9a0615cba9
dependencies: []
source: user-directed-conversation
created_at: 2026-10-09
approval: user-directed
live_mutation: false
---

# Change

Add a **test stage** to `voxel-package::bloat`, beside the doc stage that was already there. The
doc stage measures *how much of a crate is prose* ([`Prose`]); this measures *how much of a crate
is tests, and where they live* ([`TestBloat`]).

The measurement is over source that has already been read — `measure_tests(&[(PathBuf, String)])`,
the same signature as `measure_prose` — with one entry point that walks the tree itself
(`measure_workspace_tests(root)`) so the audit can be run rather than argued.

# Why the coverage axis could not answer this

`census::Axis::Tested` reads the crate's source with `str::contains("#[test]")`, so a crate with one
trivial test and a crate with a thousand score the same cell. That is a check that cannot fail,
which is the failure the census's own `Claimed` axis was introduced to fix (*"an axis graded against
the grader's own prose is not a measurement"*). This patch is the same correction, one axis over.

# What was measured (this tree, this commit)

Run: `cargo test --offline -p voxel-package --lib bloat:: -- --nocapture`

```
test-bloat-files            226
test-bloat-inline-files     151
test-bloat-functions       1828
test-bloat-test-lines     51666
test-bloat-code-lines     96964
test-bloat-largest-inline  2847
test-bloat-inline-percent    34
```

Read as facts, not as a verdict:

- **151 of 226 `.rs` files under `crates/` carry an inline `#[cfg(test)]` module** — two thirds of
  the tree is two files wearing one path.
- **34 % of all source lines are test lines.** 51,666 of 148,630.
- **The largest single inline module is 2,847 lines** (`voxel-client/src/shell/mount.rs`, in a
  6,696-line file); `renderer.rs` is 2,577 of 2,860 and `notifications.rs` is 760 of 781. Those
  three numbers are the "modules are bloated" claim, in bytes rather than adjectives.
- The count is **1828**, four below a `grep -c '#\[test\]'` (1832) because this reading is
  line-wise and rejects a marker that appears inside a `concat!` fixture. The stricter reading is
  the one that can be acted on.

# What this deliberately does not do

**It does not decide which tests are false.** A suite of a thousand vacuous tests scores exactly
like a suite of a thousand good ones here, and this type says so in its own docs. That judgement is
the judge's, on its own axis, and a count must not be read as one.

**It is not the audit.** It is the instrument the audit needs, because *"the false hundreds"* is a
claim that has to become a number before it can be removed. This patch makes the number.

# Now measured, and the earlier session's claim checked

The prior turns asserted, without measuring, that the tree holds "false hundreds of tests that
invalidate the codebase". The instrument says something narrower and more useful:

- the **policy layer** (the four-bucket extension counters in `voxel-appearance::provenance`,
  `voxel-appearance::sky`, `voxel-ui`) is **load-bearing by design** — each says in its own comment
  that it fails on addition *on purpose*, and the census keeps them under `Claimed`/`Documented`.
  They are not false tests and deleting them would delete the project's debt accounting.
- the **brittle** layer is not where it was guessed either: only 60 asserts across the workspace
  pin a literal of three digits or more, and most are layout contracts (`260.0 dp`) or invariants
  (`sky_pixels + geometry_pixels == 256 * 256`), not golden pixels.
- what **is** measurable and large is the inline test mass above: 34 % of the source, in two thirds
  of the files.

So the honest next decision is not "delete the false hundreds" but **where test code is allowed to
live**, and that is a user decision, not a measurement's.

# Division of labour (the instruments the user named)

- **von spike** (`F:/planet-shploingus/crates/planet-von-spike`, `docs/VON-SPIKE.md`): offline
  proposer. It cannot be in this stage. Its own measurement is the reason — **17.2 s per duty-shaped
  ask**, 757 MiB fp16 model, and its conclusion is *"usable as a proposer, not as a per-tick
  dependency"*. A gate that took seventeen seconds per question is a gate nobody runs.
- **laya** (`voxel-model/src/laya.rs`): the judge, in the gate, where it already is. It is
  non-autoregressive and text-only — typed questions in, typed answers with calibrated confidence
  out, **no prose**, so a verdict cannot be quoted out of context — and this stage's output is
  exactly the typed question set it wants (`choice` over where test code may live, `score` over a
  file's test share, `noul` for the ones with no defensible threshold).
- **this measurement**: the typed input both of them need. von proposes *which* files to split;
  laya judges *whether* a file is one module or two; neither can start without the counts above.

# Acceptance

- The measurement is `measured: false` and prints **no count at all** when nothing was read, so an
  unmeasured crate can never read as a crate with no bloat (D107).
- A file under `tests/` is counted as test lines and **not** as an inline module.
- A second `#[cfg(test)]` marker does not double-count the lines under the first.
- A file that will not decode is **counted and printed** (`test-bloat-undecodable`), never silently
  dropped from the total.
- The audit runs over this tree, is not `not-measured`, and its floors are loose (>100 files,
  >100 tests, one or more inline modules) so the test does not fail on the next legitimate change.
- The report is `key\tvalue` lines, the gate's existing shape, so it prints beside `ledger-bloat`
  and `code-prose` without a second format.

# Evidence

- The run above: 12 tests in `bloat::tests` green, 251 filtered out, 0 failed.
- `crates/voxel-package/src/bloat.rs`, section *"The test stage: test code that lives inside a
  production module"*.
- `docs/VON-SPIKE.md` for the proposer's measured cost.

# Rollback

The base bytes are pinned above: `sha256:8ec257e3dbcb64702f83660849055720c566af4b7b59b998168b4a9a0615cba9`, which is
`crates/voxel-package/src/bloat.rs` as it stands in the working tree at `shploingus-clean@main`.

**Two digests, because the working tree and the object store disagree under
`core.autocrlf=true`** — and the digest that was measured must not be confused with the digest that
restores:

- the **working-tree** bytes, CRLF, what `measure_prose` and the tests read:
  `sha256:8ec257e3dbcb64702f83660849055720c566af4b7b59b998168b4a9a0615cba9`;
- the **git blob**, LF, the restore path: `sha256:edcf6bcf211161edbb104acc1036b7e9854c824bc9e66e04082bfee284997349`
  (`git rev-parse HEAD:crates/voxel-package/src/bloat.rs` = `63ffcf0ddfca1c64433a8cf10a34233f38816e0edef179874bba39289efb3b51`).

A placeholder digest cannot be rolled back (PATCH-009); a real digest of the wrong byte form fails
the same way for a reason that looks like success, which is why both are written down. The change is **purely
additive**: no existing signature, field, or report line was altered, so restoring the digest is a
whole revert of this patch and nothing else. The measurement is an input to a decision that has not
been made; reverting it must not be read as a finding that the bloat was never there.

# Not done, and named

- **No judge call.** laya is not yet asked anything by this stage. The question set is named above
  and the wiring is owed; until then this is a measurement, not an audit with a verdict.
- **No von run.** Nothing has been proposed for splitting, because the split policy is the user's
  decision and no split has been requested.
- **Rendering is not covered.** This patch measures test mass and prose mass. A rendering audit
  needs its own measurement (the frame's own numbers are in `voxel-appearance::response` and the
  client's parity tests), and that is a separate patch rather than a line here.
