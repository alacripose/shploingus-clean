---
patch_id: PATCH-019
title: The suite finishes, and the failure it was hiding
status: applied
operation: strengthen
target: shploingus/gate-timing
base_ref: shploingus-clean@main
base_digest: sha256:edcf6bcf211161edbb104acc1036b7e9854c824bc9e66e04082bfee284997349
dependencies: [PATCH-017]
source: user-directed-conversation
created_at: 2026-10-09
approval: user-directed
live_mutation: false
---

# Change

Two facts, one patch, because the second was invisible until the first was fixed.

**`bloat::invented_stores` walked the build output.** `collect_invented` recursed into every
directory under the root it was given, and the census gives it the workspace root. Measured on this
tree: **18,248 of its 20,897 files live under `target/`** and `.git`, against 1,288 for everything
else. So every census call — and the census tests call it repeatedly — walked 87 % of the tree as
build output. The walk now skips `target` and `.git` **by name**, and they are the two directories
no invented record may live in.

**The lib suite could not finish.** `cargo test -p voxel-package --lib` did not complete in 300 s
and did not complete in 420 s. After the skip it completes in **189 s**. That is the measurement,
and it is why the second fact had never been seen: a suite that does not finish has no result, and
a failing test inside it reports nothing.

# The failure the fix revealed (named, not fixed)

```
---- compendium::tests::a_decision_body_is_verbatim_so_the_record_cannot_drift_from_the_format ----
panicked at crates\voxel-package\src\compendium.rs:1293:9:
the body is not a substring of its own document, so it is not verbatim
test result: FAILED. 267 passed; 1 failed; 1 ignored; 0 measured; 0 filtered out; finished in 189.16s
```

**Mechanism, established rather than guessed:**

- `compendium.rs:768` builds an anchor's body with `body_lines.join("\n")` — `\n`, always.
- `docs/LEDGER.md` in this working tree has **CRLF** terminators (`core.autocrlf=true`).
- The test compares `document.contains(occurrence.body.trim())` (`compendium.rs:1294`), so any body
  of two or more lines is not a substring of the document it came from.
- It reproduces alone, in **0.01 s**, with no compilation, and `compendium` never reads `patches/`
  (`grep -n patches compendium.rs` is empty) — so the two records this session added cannot have
  caused it. The failure is the tree's, not this session's.

**Two candidate repairs, and this patch does not choose between them:**

1. **Normalise at the comparison** — compare with `\r\n` folded to `\n` on both sides. Local, and
   it leaves the parsed body exactly as it is.
2. **Read the document with its terminator normalised once**, at the single read, so every consumer
   of the record sees one form. Wider, and it is the reading `record::DOCUMENTS` would have to agree
   with.

The choice is a decision about what the record's bytes are, which is why it is written down rather
than taken: option 1 makes the digest reproducible only after folding, and option 2 makes the digest
depend on the fold. A patch that silently picked one would be the defect the format exists to
prevent.

**Correction, recorded after this patch was written: option 1 was taken, and it was taken in the open.**
The comparison in `compendium.rs` now folds `\r\n` to `\n` on the document side before the
`contains`, so the body's own characters are still compared — nothing about the assertion's claim is
loosened, and no `#[ignore]` was used. Option 1 is chosen over option 2 because **it changes what a
test compares and not what the record's bytes are**: the parsed body stays exactly as `compress`
built it and every digest of it is unchanged, where option 2 would make the digest itself depend on
the fold. Option 2 remains unchosen and available, and if the record should read one form once and
for all — the wider reading `record::DOCUMENTS` would then have to agree with — that is a decision
still owed, not a decision this repair pre-empts.

# Why this belongs in the backwalk

The question *"what's in the game's output but likely ignored by something"* has an exact answer
here: **a red test in the gate's own adjudication of the ledger, ignored because the gate could not
reach it in time.** It is not a style finding and not a flake — it fails deterministically in
0.01 s, and it has been failing while `cargo test -p voxel-package` looked like a timeout rather
than a result. Anything reading the suite as green was reading an unfinished run.

# Acceptance

- The census's invented-record walk does not descend into `target/` or `.git`, and the scratch-tree
  test that constructs an invented record still finds it.
- `bloat::tests` (12) pass, including `an_invented_record_is_found_by_extension_and_an_empty_result_is_a_measurement`.
- The lib suite completes and reports a result, in a bounded time, rather than being killed.
- The compendium failure is stated with its mechanism and its line numbers, and **is not** fixed,
  skipped, or suppressed: `#[ignore]` would be exactly the "weakened assertion to make verification
  pass" this project refuses.

# Evidence

- `cargo test --offline -p voxel-package --lib` → **267 passed; 1 failed; 1 ignored** in **189.16 s**;
  `/failures` names the compendium test above. Log: `logs/pkg-lib-2.txt`.
- `cargo test --offline -p voxel-package --lib compendium::tests::a_decision_body_is_verbatim -- --nocapture`
  → 0 passed; 1 failed; **0.01 s**. Reproduces alone.
- `file docs/LEDGER.md` → `with CRLF line terminators`.
- `find . -type f | wc -l` = 20,897; `find ./target -type f | wc -l` = 18,248.
- The timed probe and its log: `logs/pkg-lib-probe.txt`.

# Rollback

Base bytes: `crates/voxel-package/src/bloat.rs` git blob
`sha256:edcf6bcf211161edbb104acc1036b7e9854c824bc9e66e04082bfee284997349` at
`shploingus-clean@main`. The change is one guard clause in `collect_invented`; restoring the blob
restores the slow walk, and with it the ability to claim a timeout is a result. **Reverting this
patch does not fix or worsen the compendium failure** — it only makes it invisible again.

# Not done, and named

- ~~**The compendium failure is open.** No repair has been chosen and no assertion has been touched.~~
  **Closed after this patch by the correction above** — option 1, folding at the comparison. The
  assertion was not weakened, skipped or suppressed; the claim it makes — *the body is the record's own
  characters* — is the claim it still tests.
- **No other module was timed.** 189 s for one crate's lib tests is still the slowest thing measured
  here, and where it goes — the census corpus reads, the doc reads, or the ledger parse — is
  unmeasured. A timing breakdown is a patch of its own.
- ~~**The integration tests never ran.**~~ **Closed after this patch.** `cargo test -p voxel-package`
  (every suite, not `--lib`) now runs green: **291 lib tests** plus all **16 files** under
  `crates/voxel-package/tests/`, 0 failed. The run that first timed out has a result.
