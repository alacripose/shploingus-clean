---
name: diffusion-ledger-v3-audit
description: Use when auditing a claim or record against the v3 envelope. Read-only pointer — the substance is in docs/LEDGER.md C-92.
version: 0.3.0
---

# v3 envelope — pointer

**The substance is `docs/LEDGER.md` §92.1–92.4. Read that; it is the record, and this file
exists only so the question has a name to be asked under.**

## The one thing worth carrying without opening the ledger

**There are two ledgers, and only one audits anything.**

| artifact | role |
|---|---|
| `crates/voxel-model/src/envelope.rs` | **the declaration this tree admits — the cross-reference key** |
| `shploingus\ledger\shploingus-slice1.jsonl` | the **source** the shape was measured from — reference, never rules (C-48/C-49) |
| `chains/workspace.chain` | the live chain; linked, genesis = the empty-string digest |

`envelope.rs:5-8` states all of it. **A finding against the source file is not a weaker finding
about this tree. It is a false one wearing a measurement** — which is what §92.1 records.

Also in §92: the declared vocabulary equals the source's observed vocabulary value for value
(5 / 6 / 3), and this tree **sided with the file over the file's own self-description**.

## Why this is a pointer and not a protocol

An earlier version of this skill carried a procedure and a self-grill. Both were deleted
because `docs/TODO.md` §0 already holds them as **C-90** — *"read the record; do not grep it"*
and *"the smallest thing worth detecting — not the smallest edit"* — and because a file that
re-types `envelope.rs`'s own doc comment is volume, which is the defect. **A restatement of a
rule the record already carries is not a second copy of the rule; it is a second thing to drift.**
