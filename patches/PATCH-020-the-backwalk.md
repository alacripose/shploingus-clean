---
patch_id: PATCH-020
title: The backwalk — what is patched, owed, missing, ignored, realistic, and what the health is
status: applied
operation: add
target: shploingus/backwalk
base_ref: shploingus-clean@main
base_digest: sha256:7aea80984f7a31f00c1f8d09d51d6f6ffff07c0adc6e791d7efb24320d1846ac
dependencies: [PATCH-017, PATCH-018, PATCH-019]
source: user-directed-conversation
created_at: 2026-10-09
approval: user-directed
live_mutation: false
---

# Change

A backwalk of every system this tree has, against the gate that already exists for it, answering six
questions with measurements. It changes no bytes: it is a read, and the digest above is the chain's
own digest **as the gate itself computed it**, so this reading is pinned to the record it read.

The instrument is the tree's, not this session's: [build.rs](crates/shploingus/build.rs) runs a
mandatory gate on every build (*"the game cannot compile without the system"*, C-52.1), writes one
report to `OUT_DIR`, and `main.rs` embeds that same file so a green build and the legible panel
cannot disagree (C-53.3). The report is
`target/debug/build/shploingus-1b006e32dd437c39/out/alpha-shell.gate`, **683 lines**, from the last
recorded build.

# 1. What has been patched

| reading | number |
|---|---|
| patch records present | **19** |
| applied | **8** |
| owed (proposed / designed / approved) | **11** |
| unevaluable (no real digest, missing section, duplicate id, dangling dependency) | **0** |
| ledger chain records | **785**, all 785 carrying a previous-event hash |

Zero unevaluable is the load-bearing fact: the format is honest even where the work is not done, so
it can carry the retrofit without being rewritten first.

# 2. What has not been patched

- **Eleven of nineteen records are owed.** Nothing in this tree reads them yet except
  [patch.rs](crates/voxel-package/src/patch.rs) — the reader landed today.
- **Four declared features are `unchecked`**, each with the witness file the row names and nothing
  provides:
  `signature_verification` → `crates/voxel-envelope/src/lib.rs`;
  `sealed_container` → `crates/voxel-package/src/envelope.rs`;
  `command_line_tool` → `crates/voxelctl/src/main.rs`;
  `ticket_window` → `crates/voxel-client/src/shell/tickets.rs`.
  Fifteen of nineteen feature rows are `active`.
- **Six of twelve crates are `unclaimed`**: `voxel-noise`, `voxel-instrument`, `voxel-work`,
  `voxel-ui`, `voxel-package`, `shploingus`. The census refuses to call that a failure — *"a thing
  nobody has claimed yet"* — but it is also twelve crates where six have no row in the spec's own
  declaration tables.
- **Every crate is `refused` on the `documented` axis**, without exception. The reason is one
  sentence, and it is the prose rule this tree's own gate enforces: *"prose in a code artifact is
  documentation written where a reader does not look for it."* Measured comment share:
  voxel-model 19 %, voxel-noise 33 %, voxel-mesh 36 %, voxel-shapers 37 %, voxel-instrument 38 %,
  voxel-stream 41 %.

# 3. What is missing

- The four witness files above, and the crates that would own two of them.
- **`Refusal`, `Admission` and `Proof` are not record kinds here.** Upstream ruled them on
  2026-10-05 (*"a gate that cannot record a refusal cannot refuse"*), and this tree's chain carries
  785 records of the kinds it already had. So a refusal here is a line in a build report, not a
  record on the record.
- **No MCP and no doors.** Upstream `planet-mcp` is a capability-gated tool surface
  (`get_axis_report`, `get_world_state`, `get_cell`, `step_simulation`, `get_surface_mesh`,
  `get_tick_report`, and `capture` for the shell) over `planet-axis`/`planet-gate`/`planet-ledger`;
  here there is no bridge, so the gate's work list cannot be *served*, only printed.
- **A patch is not an event on the chain.** Nothing files a record into
  [workspace.chain](chains/workspace.chain); the nineteen records are still markdown on disk.

# 4. What is in the output but likely ignored by something

This is the question the gate answers about itself, and the answers are unbecoming:

- **`projection-disagreements    639`** — `projection-in-chain-not-in-record 639`,
  `projection-in-record-not-in-chain 0`. The gate's words: *"the two views DISAGREE, so
  docs/LEDGER.md is currently a second hand-written record rather than a projection of the chain;
  writing the projection now would delete the entries listed above."* 639 recorded facts exist in the
  chain and nowhere in the document the project treats as its ledger.
- **`board-tickets 515`, `board-duplicate-filings 43`, 86 repeat filings.** The board's own caveat:
  *"the ticket count above is NOT a count of distinct pieces of work."* `logs/agent-inbox/` holds
  **515** files, matching the ticket count exactly.
- **`logs/` is 413 MB, of which `logs/test-workspaces/` is 403 MB** across 402 files, including two
  directories that are **untracked debris right now**: `notifications-tampered-15436/` and
  `notifications-tampered-8804/`, written by
  [notifications.rs](crates/voxel-client/src/shell/notifications.rs)'s tamper test. Upstream's
  prohibition is exact: *"`logs/` is evidence of verification only — never a queue, never a data
  channel, and **no test writes there**."* `logs/` is also in no `.gitignore`.
- **`logs/laya-decisions/` holds 226 records, four of them tracked and modified in the working
  tree.** Upstream read the same directory in this same tree and called it *"test residue"*.
- **`packages/`: 526 files, 129,862 bytes total, mean 246 bytes**, of which 511 carry a digest in
  the name and 14 do not. Upstream measured this tree's packages at a mean of **232 bytes** and ruled
  that *"a package carries the work — the bytes that will run — never a pointer to it and never a UI
  description of it."* The frame has not changed: it still carries a description of a few UI rows.
- **A red test in the gate's own adjudication of the ledger**, ignored only because the suite could
  not finish (PATCH-019): `compendium::tests::a_decision_body_is_verbatim…` fails in 0.01 s.

# 5. What is realistic

- The retrofit is **expected to refuse**. Upstream's ruling 6, at approval: *"the retrofit audit is
  expected to refuse. A gate that only grades new work is the defect it exists to prevent."* So the
  `documented` axis being refused on every crate is not a defect to be argued away; it is the audit
  working on its first run.
- The scale is asymmetric and that is the point: **403 MB of test output** against **129 KB of
  packages**, and 51,782 test lines against 97,249 production lines. The instrument that produces
  evidence is 3,000× larger than the artifact that carries work.
- Not every missing thing is buildable now: upstream's own unclaimed list is the same shape (its
  first live run was **94 readings, 13 refusals**, each naming the file that would witness it).
  A refusal naming a file is a work item; a refusal naming nothing is not.

# 6. The health of the game

From its own gate, unedited:

- **features**: 15 of 19 active; 4 unchecked, each naming its missing witness.
- **conformance oracle**: active, **103 of 103** known answers, accuracy **1.0000** against a
  majority class of **0.5631**, inverted accuracy **0.0291**, `oracle-kind structural-deterministic`,
  `oracle-is-laya yes`, `model-oracle established`. Laya is not a model oracle and the report says so
  in its own words.
- **the record**: 10 documents with authority digests; **785 chain records, 1,532,016 bytes, 34 %
  payload**, 785 of 785 linked — with the gate's own caveat beside the number: *"a hash is evidence
  about bytes and order, not permission."*
- **provenance**: 805 files, `provenance-commit false`, `provenance-object-format sha1`,
  `provenance-digest-expressible false`, because `.git/HEAD` names a branch and no commit, and this
  project's `Digest` refuses to express sha1.
- **verdict**: the systems run, the gate runs on every build, and **nothing in the tree is
  documentation-complete**. There is no commit, the ledger is a second hand-written record, the four
  declared-but-unbuilt features are named, and every crate fails the axis the tree itself put there.

# Acceptance

- Every answer above carries a measured number and, where it is a refusal, the file the refusal names.
- The reading is pinned to a digest the gate itself computed, not one this session chose.
- Nothing is claimed as fixed: the backwalk changes no bytes, and no finding here is presented as a
  repair.

# Evidence

- `target/debug/build/shploingus-1b006e32dd437c39/out/alpha-shell.gate` — 683 lines, sections
  *declared features*, *conformance oracle*, *the record*, *what the record costs*, *which bytes this
  build read*, *do the two views of the record agree*, and the per-crate census rows.
- `cargo test -p voxel-package --lib patch::` → 19 records, 8 applied, 11 owed, 0 unevaluable,
  0 dangling.
- `du -sh logs` → 413M; `logs/test-workspaces` → 403M / 402 files; `find packages -printf '%s'` →
  526 files, mean 246 bytes; `logs/agent-inbox` → 515 files.
- Upstream, for the rules these are measured against: `F:/planet-shploingus/docs/AXIS-MODULE.md`
  §§2–5 and `crates/planet-gate/src/subject.rs` (nine subject classes, each declaring its witness) —
  read as **reasons, never requirements**, per that note's own *"no old decision tree is authority
  here."*

# Rollback

A read cannot be rolled back and this record changes no bytes: the tree is byte-identical before and
after. The digest above is the chain's, so a chain that changes makes this reading **stale rather
than wrong** — which is the same rule the gate applies to proofs, and it is why the digest is a field
of the record rather than a sentence in it.

# Not done, and named

- **The gate was not re-run for this reading.** It is the last recorded build's report; `shploingus`
  was not rebuilt (it needs the GPU stack), so the census and bloat numbers are as of that build and
  a fresh run could differ.
- **No fresh census of the four `unchecked` features**: the rows are quoted, not re-derived.
- **The 639 disagreements are counted, not listed.** Where each of those 639 facts lives in the chain
  is a separate read.
- **`logs/` was not cleaned.** 403 MB of test output and two untracked debris directories are still
  on disk, and removing them is a decision about what evidence is retained.
