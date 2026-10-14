---
patch_id: PATCH-018
title: The patch format adopted in-game
status: applied
operation: add
target: shploingus/patch-records
base_ref: shploingus-clean@main
base_digest: sha256:d7eb1da0510985bff39469a9378ed4b746f41af7b066583403184036f4cba710
dependencies: []
source: user-directed-conversation
created_at: 2026-10-09
approval: user-directed
live_mutation: false
---

# Change

`patches/` was a folder of records with **no reader**: `grep -rn 'PATCH-0\|patches/' crates docs
architecture chains` returned nothing, so seventeen governed changes sat in the tree in a format
nothing in the game could parse, cite, or gate.

This patch adds `voxel-package::patch`, wired at `lib.rs`:

- `parse(file_name, text)` reads a record's frontmatter and body and **collects every problem**
  rather than returning the first, as `wire.rs` does for a frame.
- `read_dir(dir)` reads the whole set, and a directory that will not read is `not-measured` with a
  reason — never a set of zero records (D107).
- `PatchSet::dangling()` names a dependency on a record no file carries; `duplicated()` names an id
  written twice; `describe()` prints `patch-set-*` lines in the gate's existing `key\tvalue` shape.

The rules are the patch set's own, not a new opinion:

- a record with no `base_digest` is refused, because PATCH-009 rules that *"a boolean such as
  `rollback_present=true` is not rollback evidence"* and *"a missing or placeholder digest makes the
  candidate `not_evaluable` and blocks promotion"*;
- a `base_digest` holding a word instead of a `sha256:` is refused by the same rule, and
  `recorded source digest` is enumerated rather than pattern-matched;
- each of `# Change`, `# Acceptance`, `# Evidence`, `# Rollback` is **named** when missing, because
  a count of sections cannot be acted on and a missing section name can;
- `live_mutation: true` is refused: every record in this folder declares `false`, and PATCH-016's
  safety boundary keeps it that way;
- the file name must begin with the record's own `patch_id`, so a finding can be cited from its path.

# The backwalk's first fact, measured

Run: `cargo test --offline -p voxel-package --lib patch:: -- --nocapture`

```
patch-set-records      17
patch-set-applied       6
patch-set-owed         11
patch-set-unevaluable   0
patch-set-dangling      0
```

Read as facts:

- **Six applied, eleven owed.** The applied set is PATCH-007, 008, 009, 010, 011 (`-internal`) and
  the two added by this session. Everything else is `proposed`, `designed`, or `approved` and has
  no reader-visible consequence in this tree yet.
- **Zero dangling and zero unevaluable**: every dependency resolves to a record present, every
  record names a real `sha256:` base digest, and every record carries all four sections. The format
  is honest even where the work is not done — which is what makes it usable as the backwalk's spine.
- **Zero duplicated ids**, so a citation from a ledger entry cannot resolve to two records.

# The upstream formats this has to meet

`F:/planet-shploingus` carries the same system one generation on, and this patch is written to be
readable by it:

- `docs/AXIS-MODULE.md` (APPROVED 2026-10-05) is the gate over everything: seven ladders, a subject
  census, admission at both doors, ticking proofs, the shell's five-star view, and **the retrofit** —
  *"technically speaking this does make the entire work so far on the planet shploingus not actually
  conformant… so from the very beginning there would be no ambiguity of what really needs to be
  done."*
- The same note records this tree's four defects **verbatim**, and they are the four this session
  has been measuring: minimalization producing *"trees that looked like dirt and rendering bugs that
  were unsolvable"*; comment blocks (*"it even had a bloat checker… and we never got to the part
  where the bloat checker actually fucking made sure that the docs were verifiably real"*); tests
  frozen on tunables (*"even simple things like toast notification duration was actually hard coded
  in, and if the user wanted to change it it would literally fail"*); and prose that is well-formed
  but unverifiable.
- `crates/planet-axis/src/axis.rs` is the recreation: `AxisKind` = `EvidenceGrade`,
  `CertificationTier`, `GateStatus`, `SupportLevel`, `EvidenceOrigin`, `UnitGrade`, `OrgRank` —
  named ladders, against this tree's `ProceduralGate`/`TicketEvidence` naming. `AXIS-MODULE.md` is
  explicit that **nothing is imported from this tree**, so the axis work here is a *diff to be
  reconciled by decision*, not a merge.

# Acceptance

- `parse` collects all problems; a record missing any section names that section.
- A placeholder or absent `base_digest` is `not_evaluable` (PATCH-009), and a test proves both.
- `read_dir` on an unreadable directory prints `not-measured` and **no count**.
- Undecodable files are counted into the set as an explicit `<undecodable>` record, never dropped.
- The real `patches/` directory reads: 17 records, no duplicates, no dangling dependency, every id
  `PATCH-NNN`.
- `dangling()` is exact in both directions: a dependency on a present record is **not** dangling.

# Evidence

- `cargo test --offline -p voxel-package --lib patch::`: **6 passed, 0 failed** (263 filtered out).
- `cargo test --offline -p voxel-package --lib bloat::`: **12 passed, 0 failed** (PATCH-017's set).
- The background probe of the full lib suite, and its result, are reported with this patch rather
  than assumed: `cargo test -p voxel-package --lib` **did not finish within 300 s**, and the probe
  log is `logs/pkg-lib-probe.txt`. That is a finding, not a pass.

# Rollback

Base bytes: `crates/voxel-package/src/lib.rs` at `shploingus-clean@main`, git blob
`sha256:d7eb1da0510985bff39469a9378ed4b746f41af7b066583403184036f4cba710` (the working tree carries
CRLF under `core.autocrlf=true`, so its byte digest differs from the blob's — the blob is the
restore path). The change is one added line in `lib.rs` plus a new file, so rollback is deleting
`src/patch.rs` and that line. A record with no reader is still a record; reverting this must not be
read as the seventeen records ceasing to exist.

# Not done, and named

- **No MCP.** This tree has none; upstream it is `planet-mcp` (a capability-gated tool surface with
  `boundary`, `tcp_concurrency` and `transcripts` tests) over `planet-axis`/`planet-gate`/
  `planet-ledger`. Nothing here can call a tool until that decision is made.
- **No chain link.** A record is read from a file and reported; it is **not** filed into
  `chains/workspace.chain`, so a patch is not yet an event on the record and cannot be cited by
  digest from the ledger.
- **`status` is trusted, not verified.** A record that says `applied` is counted as applied even if
  the artifact it names was changed again afterwards; checking that is a diff against the pinned
  digest, which is the next patch rather than a line in this one.
