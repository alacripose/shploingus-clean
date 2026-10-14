---
patch_id: PATCH-023
title: The ballot — the package format a classifier can judge, and the von classifier wired to it
status: applied
operation: add
target: shploingus/von-classifier
base_ref: shploingus-clean@main
base_digest: sha256:b69b66ce6e8a835ec6ce073d8256b7fa035d640ad4b9b0e45c66a6321a368fd1
dependencies: [PATCH-022]
source: user-directed-conversation
created_at: 2026-10-09
approval: user-directed
live_mutation: false
---

# Change

A package says **what to build**. It cannot say **what question the work answers**, and a judgment
needs one — so an agent could not judge its own work, and the decision layer had no instrument to
point at an artifact. This patch adds the missing half of the format and the classifier that answers
through it.

## 1. The ballot: what a package must declare to be judgeable

`crates/voxel-package/src/ballot.rs`, header `shploingus.ballot/1`, `key<TAB>value` lines — the shape
every gate report in this tree already uses, so a ballot and a report read the same way.

| field | why it is not optional |
|---|---|
| `ballot` | the id the answer comes back under, and the row a finding cites |
| `claim` | the ticket or ruling the work serves — an unattributed claim is not a claim |
| `artifact` | **the bytes the answer is about**, workspace-relative, so a judgment can be re-derived from them |
| `kind` | `noul`, `choice` or `score` — the instrument, and the decision layer's own three |
| `instruction` | how to judge the state; the model receives this and nothing else about intent |
| `option` / `level` | the terms: a choice's options, a score's ordered rungs |
| `presenter` | who is accountable; upstream refuses a presentation with no accountable actor |
| `max-age` | how many ticks an answer stays current — **stale is refused, never grandfathered** |
| `passes` / `floor` / `threshold` | **where the answer stops passing** |

**The floor is the reason this is a format and not a convention.** An answer is not a verdict:
"the model chose *leave it*" says nothing about whether that is good work until someone declares
which answers count. A floor chosen by the code that reads the answer is a floor chosen to fit it,
so it is declared **before the answer exists** — a `choice` names the passing option, a `score`
names the lowest passing rung, a `noul` may name its threshold (0.5 by default, because a
probability of yes has a natural midpoint and a chosen one needs a reason).

What the parser **refuses**, each with its own reason and all of them collected for one round of
fixes: a choice over fewer than two options, a score over fewer than two rungs, an unknown
instrument, no `passes`/`floor` for the instrument that needs one, a `passes` naming an undeclared
option, a floor above the top rung, an empty claim/artifact/instruction/presenter, an instruction
past 4,096 bytes, `max-age` of 0 (*"not a window"*), two ballots sharing an id, an unknown term, and
a file that declares nothing — which is a **`NoReading`, not a pass**.

## 2. The classifier: `crates/voxel-von`, and it runs

`jigor = { version = "0.1.10", default-features = false }` (CPU `ort`) with upstream's own link fix
copied in — `[patch.crates-io] esaxx-rs = { path = "vendor/esaxx-rs" }`, upstream 0.1.10 with one
line changed, because `esaxx-rs` ships `/MT` objects that cannot link with jigor's prebuilt ONNX
Runtime (`/MD`).

**It is its own crate on purpose.** The gate's crates carry no external dependency; a judgment that
needs a 757 MiB model is a crate you ask for, not weight in the crate that decides. `voxel-von`
holds the backend (`Von`, loaded once and asked many times, because a load inside the first question
would put a 3 s failure in the middle of a judgment), the four question constructors, and
`cached()` — a question asked of the cache in microseconds rather than discovered after a download
attempt inside a judgment.

## 3. The judgment: `voxel-von::judge`

`decide(ballot, answer) -> laya::Decision`, **pure**, so the rule is testable without a model:

- `noul` → `Belongs` at or above the declared threshold, confidence = the model's own probability;
- `choice` → `Belongs` only when the declared passing option was chosen, with every option's
  probability carried, because a summary would hide how close the second choice was;
- `score` → `Belongs` at or above the declared floor, and the finding **names the rung** (`"mostly"`,
  from the model's own legend) rather than the index.

Three rules hold across all of them. **The origin is `EvidenceOrigin::Oracle` and no ballot field can
set it** — upstream's rule that the origin is the proof's to carry, not the claimant's to declare,
which is also why the ladder's `A` rung is what a model's answer gets. **A shape mismatch is
`NoReading`**, never a verdict for the other instrument. And **a missing answer is a non-reading**:
the alternative is a missing answer read as a pass.

## 4. The MCP agent judges its own work

`judge_work` on the MCP surface, on the bridge row **`verify_package`** — which is what judging a
package is, and which the tree's own capability partition covers with **`present`**, not `observe`.
That distinction is the bridge being right and a test being wrong: the read loop asserted `observe`
for every served tool and failed the moment this tool existed, so the test now reads each row's own
capability from the partition instead of asserting a belief.

`{"ballot": "packages/voxel-von.ballot"}` → the declared ballots, judged against the named
artifacts' **own bytes read off disk**, returned in the gate's `key\tvalue` shape. The model loads
**on the first judgment and never at startup**: a client that only reads the gate pays nothing, and a
second judgment reuses the session (measured: the second call's asks cost 3.3 s each, not a second
22 s load).

## 5. The ballot rides the recipe — the sibling file closed over

**The first draft of this patch named this as not done, and it is now done.** A ballot was a sibling
file (`packages/*.ballot`), which meant a package and the question it answers could be separated — a
package judged by a question nobody declared for it. `shploingus.recipe/3` now carries a
`ballot<TAB>id` kind, so the work and the questions it answers are declared in the same bytes.

Three properties, and each is a test rather than a claim:

- **One parser, two entry points.** `read_ballot` hands its lines to `ballot::parse_body` instead of
  re-reading the fields, so a term the ballot grammar refuses is refused here with the ballot
  parser's own words, and a field added there is picked up without an edit to the recipe grammar —
  `ballot::is_field` is the one list.
- **A `ballot` row is both a term and a boundary.** It ends the block so the next question can open,
  which is the ballot module's own declared rule (*"a ballot opens with `ballot` and closes when the
  next one opens"*) rather than a second boundary rule invented here. `ballot` is itself a ballot
  field, so the boundary cannot be "an unknown key" alone: a reader that only stopped at an unknown
  key read the next header as a field of the first question and reported both as one malformed
  question — a plausible refusal over content it never read.
- **Empty is legal, and it is not a pass.** A package that declares no ballot still parses; it simply
  cannot be judged, and nothing conjures a question for it. That is the same rule the ballot parser
  already carries for a file that declares nothing (*"nothing to judge is not nothing wrong"*).

# Acceptance

- The ballot parses and round-trips through its own render; a ballot and its re-parse are equal.
- Every refusal above has a test, each asserting the reason rather than a count.
- The floor tests assert the comparison the floor governs: at-threshold passes, below fails, a
  declared `threshold` of 0.9 fails a 0.7 answer, a floor above the top rung is refused at parse.
- `decide` is tested against all three instruments plus a mismatched answer, with no model present.
- The model runs: `JIGOR_CACHE_DIR=F:/planet-models` loads `Zatsepin/von-onnx-fp16` and answers.
- **The MCP surface judges**: `judge_work` with `present` returns verdicts; with `observe` it is
  refused by the bridge, naming the capability, with no side effect and no model load.

# Evidence

- `cargo test -p voxel-package --lib ballot::tests` → **11 passed, 0 failed**.
- `cargo test -p voxel-package --lib recipe::tests` → **35 passed, 0 failed**, seven of them new and all of
  them the ballot block riding the recipe: read into the package without swallowing the rows above
  it, the block boundary at the next `ballot` row *and* at an unknown key, the ballot parser's own
  refusal naming the term it saw, two ballots sharing an id refused rather than resolved, a package
  that declares no question, and the question surviving `compile`/`load`.
- `cargo test -p voxel-package` → **291 passed, 1 ignored, 0 failed** (all suites, not just the lib).
- **One test in that suite was repaired rather than left failing.**
  `compendium::tests::a_decision_body_is_verbatim…` compared a body `compress` had built by joining
  lines with `\n` against the raw bytes of `docs/LEDGER.md`, which a Windows checkout with
  `core.autocrlf=true` writes with `CRLF` — so the assertion was really *"this working copy is
  byte-identical to that working copy"*, a fact about the checkout rather than about the record, and
  it passed or failed by which editor last saved the file. The comparison now normalises line endings
  on the document side, which is what `compress` does when it reads one; nothing about the body's
  characters is loosened.
- `cargo test -p voxel-von --lib` → **6 passed, 0 failed**.
- `cargo build -p voxel-von` → jigor 0.1.12, ort 2.0.0-rc.13, tokenizers 0.23.2, the vendored
  esaxx-rs; links clean.
- `cargo test -p voxel-mcp` → **18 passed, 0 failed** with `judge_work` added (9 tools; the api
  view and the capability assertions adapt to the bridge rather than to a hardcoded list).
- The live session, `logs/von-classifier-session.txt` (exit 0):
  - `Zatsepin/von-onnx-fp16`, cache `F:/planet-models`, `cached true`, load **22.9 s** from cold,
    three questions in **2.2 s**;
  - `packages/voxel-von.ballot` against the real 6,021-byte `crates/voxel-von/src/lib.rs`:
    `belongs` **0.9651**, `documented` **0.5704** — both `Belongs`, origin **A**, grade
    `ticket_evidence A`, ~3.3 s per ask;
  - the findings carry the probability **and the threshold it was compared against**, so the verdict
    is re-derivable without the model.

# Rollback

Delete `crates/voxel-package/src/ballot.rs`, `crates/voxel-von/`, `packages/voxel-von.ballot` and
`vendor/esaxx-rs`, and remove the member line, the `jigor` workspace row and the `patch.crates-io`
section. Then take the kind back out of the recipe: `Recipe::ballots`, `Recipe::read_ballot`, the
`ballot` dispatch arm and the `ballot` guard in the bar's vocabulary error message in `recipe.rs`, and
the seven tests that hold the block. The tree returns to PATCH-022 with no model dependency, the
recipe grammar back to version 3 as it was, and the ballot files stay readable as text.

# Not done, and named

- **The decision layer does not ask it.** `laya`'s own instrument is still `oracle::lint`; nothing in
  `evaluate_project` consults a ballot, so the classifier's verdicts are served by the example and
  not yet by the gate.
- **The classifier is now in the MCP process, and that is a trade taken knowingly.** It is lazy
  (`OnceLock` + `Mutex`, loaded on the first judgment), so a read-only client pays nothing — but a
  client that judges pays 22 s once and holds a 757 MiB session for the process's life. Upstream
  keeps the von spike in its own binary; if the MCP server is ever long-lived or resource-bounded,
  the same split is the fix, and the seam (`voxel_package::ballot`) already supports it.
- **`tick_span` and `world_digest` are not carried.** The ballot has `max-age` and `presenter`, but
  upstream's `Proof` also carries the ticks the work ran over and the world digest it ended at, and
  `laya::ProofOfWork` has no field for either — so **the staleness window cannot be enforced yet**,
  and a ballot's `max-age` is declared but not checked.
- **No answer cache.** Upstream measured that a repeat ask recomputes (*"content-addressed caching is
  ours to add"*). Every judgment pays its ~3.3 s, and nothing here caches by state digest.
