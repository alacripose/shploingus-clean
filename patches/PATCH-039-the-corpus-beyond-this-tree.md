---
patch_id: PATCH-039
title: The corpus beyond this tree — twelve roots across four drives, measured before a byte is imported
status: applied
operation: add
target: shploingus/corpus-intake
base_ref: shploingus-clean@main
base_digest: sha256:b5a381c0adfbcb42d48fc294b5fe4e9498379b3915a0ca76f6b89bec1ad6b64b
dependencies: [PATCH-018, PATCH-020, PATCH-025, PATCH-037]
source: user-directed-conversation
created_at: 2026-10-10
approval: user-directed
live_mutation: false
---

# Change

**The user's direction:** *"I don't just mean the files in this project, I mean the files from the
other projects across my drives as well being consumed into the new format, so we can see exactly
where and how every decision along the chain was made in a high efficiency manner, and evaluate how
down stream vs up stream changes were made. This way when we regenerate the project from its first
principles we have a comprehensive library awaiting us… This is part of the dynamic classifier
library, using the von laya system to actually calibrate laya to give us REAL work."*

That is an **import**, and an import is a write. So this patch writes the measurement the import has
to answer to, and no record of foreign material:

- **`intake/roots.tsv`** — the corpus, **declared**: 12 roots with a label, a kind
  (`this-tree`, `sibling`, `upstream`, `corpus`, `other`), and a path. A survey that picks its own
  roots is one that silently misses a drive; a scan of a whole volume cannot run in a build, and `F:`
  answered a naive `find` in **400 seconds** in the session that wrote this.
- **`voxel_package::ingest`** — walks those roots, classifies every file from **its name and its
  first 4 096 bytes**, counts what it does *not* read, and reports the share. Frontmatter sits at the
  top of a record, so a head is enough for classification and for an id; **a survey that read every
  body would cost more than the import it exists to price.**
- **`cargo ingest`** — the escape hatch, and deliberately **outside the gate**: a walk of other
  drives is not something a build that runs on every change can afford. The module is the reading and
  the example is the command, exactly as `cargo space` works for the size guard.

# Acceptance

- A manifest that is not declared measures **nothing** and says so; a root that cannot be opened is
  reported **absent**, not as a zero; a walk that hits its budget reports itself **truncated**,
  because a shortened survey otherwise reads as a complete one. Each is a test.
- Records are counted as **files and as distinct ids**, because the difference is the finding.
- Nothing here writes to the corpus or to this tree's record: the report's own `ingest-is-not` line
  says so, and no byte of the corpus is read past its head.

# Evidence — the first complete reading, 2026-10-10, unedited

`cargo ingest --max-files 200000`, **35 seconds**, **13 095 761 192 bytes total, 19 778 514 read —
share 0.151 %**:

| root | kind | records / distinct | duplicate files | chains | docs | code |
|---|---|---|---|---|---|---|
| ours | this-tree | 38 / 38 | 0 | 1 | 348 | 264 |
| shploingus | sibling | 64 / **16** | 48 | 0 | 1 468 | 646 |
| shploingus-game | sibling | 112 / **16** | 96 | 0 | 2 611 | 1 250 |
| planet-shploingus | upstream | 32 / **16** | 16 | 0 | 739 | 157 |
| hyperskill-prototype | other | 16 / 16 | 0 | 0 | 251 | 22 |
| awa3d | other | 32 / **16** | 16 | 0 | 493 | 3 518 |
| mod-sources | corpus | 0 | 0 | 0 | 287 | 2 979 |
| ala-system | other | 0 | 0 | 0 | 82 | 114 |
| llama-prism | other | 0 | 0 | 0 | 124 | 1 585 |
| bonsai-demo | other | 0 | 0 | 0 | 8 | 96 |
| unstoppable-ref | other | 0 | 0 | 0 | 1 | 15 |
| alacripose-landing | other | 0 | 0 | 0 | 0 | 14 |

**54 distinct record ids exist in the whole corpus**, and this tree carries **all of them**: its own
38, plus the sixteen `hyperskill/*` records — which are not orphaned records for another system but
**the oldest layer of this project's own chain**, carried by every later generation (four copies
inside `shploingus`, six inside `shploingus-game`, two in `planet-shploingus`, one in the prototype)
and by `AWA3D` as well. **`ingest-only-there` is zero for every root**: there is no unrecorded
decision sitting on another drive.

**What is not here is not records — it is material.** 296 documents and ~19 000 source files across
the foreign roots carry no record of their own; the 47 vendored mod repositories alone hold 2 979
code files and 287 documents. That is what "consume the old formats into api layers" has to answer
to, and it is a documentation-and-source retrofit, not a records import.

**And `ingest-shared` names the comparison the user asked for**: every one of the sixteen
`hyperskill/*` ids reads `owed-here` here and `their-status proposed|designed|verified-internal|
applied-internal|approved` there — the same id at five different stages of its life, across five
trees. That is the upstream/downstream question, answerable per record, from ids and heads alone.

# Rollback

Two files and one module: delete `intake/roots.tsv`, `voxel_package::ingest`, its example and the
alias, and the tree is byte-identical to before — the module reads other trees and writes nothing
anywhere. The digest pins the reading; a changed record makes it stale rather than wrong.

# Not done, and named

- **No byte of the corpus is imported, and on the user's ruling of 2026-10-10 none ever needs to
  be.** Consumption is **by reference**: a declared row cites a path and a digest as its witness and
  the bytes stay where they are, so what lands in this tree is the declaration alone. The older
  trees are **superseded generations of this one and are deleted as the canon settles, not
  migrated** — *"the shploingus clean will have cleaned itself out"* — which is why pruning a
  source later stales a witness rather than losing a decision. So this is neither the price list
  nor the purchase; it is the map. Which of the 296 documents and 19 000 files become declared
  rows, and in which of the four lanes of `docs/PLAN-2026-10-10-clean-room-and-the-api-layer.md`, is
  the open question.
- **The survey cannot tell a decision from a description.** A foreign document is `docs`; a foreign
  record is a record. Which paragraphs of those 296 documents are decisions — with a subject, an
  evidence, a rollback — is exactly what a **classifier** would have to say, and no classifier has
  been asked. That is the calibration work the user named, and it is not done here.
- **`distinct` counts ids, not content.** Two records with one id and different bodies are one id
  here; comparing bodies is an import-side job.
- **The `corpus` kind is walked as one root**, so the 47 mod repositories are counted together and a
  budget can truncate the group. Per-repository roots are a manifest edit, not a code change.
- **`C:` is at 99 %** (row 30) and this survey adds ~0.15 % of read bytes, not writes: no new
  pressure. The 10 GB the mod corpus and both siblings measure is 10 GB that **need not be fitted
  anywhere** — it is read in place and cited, and the trees that hold it are retired rather than
  copied, which is the one reading this row makes possible and does not yet perform.
