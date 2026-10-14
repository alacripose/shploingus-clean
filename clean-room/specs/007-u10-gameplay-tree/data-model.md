# Phase 1 — Data model: U10 the mods-derived gameplay tree

**Feature**: `007-u10-gameplay-tree` | **Date**: 2026-10-12 | **Spec**: [spec.md](./spec.md) | **Plan**:
[plan.md](./plan.md)

## What U10 owns, and what it only consumes

| Concern | Owner | How U10 uses it |
|---|---|---|
| a declared row — feature, source, licence, carrier, acceptance, claim, requirements | U6 (`clean-room/src/intake.rs`'s `Row`) | **the tree's row is this value**, admitted through the `admit` row of `bridge::TOOLS`; U10 adds no field to it (D4) |
| the judgement — the seven axes and their four establishment states | U6 (`intake::Axis`, `intake::Establishment`) | read, never restated; a `refused` verdict is `licence_refused` and an owed check is `axis_inactive` |
| the closed vocabularies — `Licence`, `Carrier`, `Claim`, `Acceptance`, `Requirement` | U6 (`clean-room/src/intake.rs`) | consumed as they are; **no new word is added to any of them** |
| the refusals — `licence_refused`, `licence_undeclared`, `source_absent`, `source_digest_mismatch`, `carrier_undeclared`, `stage_undeclared`, `unadmitted_material`, `axis_inactive`, `axis_unchecked`, `argument_absent`, `argument_malformed`, `claim_undeclared` | U6 (`clean-room/src/refusal.rs`) | consumed as they are; the tree's **one** missing name is owed (D7), not coined |
| the served wire form — one request line, the line protocol, the receipt | U6 (`src/transport.rs`, `specs/005-u6-record-api/contracts/u6-transport.md`) | a tree row crosses exactly as `admit` does (D9); no new framing |
| the declared roots and what a root is *not* | the record (`intake/roots.tsv`) | the mod trees are cited as the `corpus` root `mod-sources` (`F:/shploingus-mod-sources`); *"nothing here is consumed by being named"* |
| the **tree** — the ordered entries, the disposition beside each, the double-cover dedupe, the census of dispositions | **U10** | this feature |

## The entities

| Entity | Fields | Rule |
|---|---|---|
| `Tree` | `entries: Vec<Entry>` (ordered) | one entry per reference, in the record's own order (`docs/MODS-MAP.md` §1–§5); order is data, not an accident of construction (FR-008) |
| `Entry` | `reference` (the map's id — `MOD-c2me`, `voxy`, `Create`, …), `disposition`, `row: Option<Row>`, `waits_on: Option<String>` (what an owed entry waits on, quoted from the map, or `None`) | the unit the tree counts. A `mapped` entry **has** a row; an `open`/`unresolved` entry **has none** and names what it waits on where the record says so (FR-003, FR-004) |
| `Disposition` | `Mapped`, `Open`, `Refused`, `Unresolved` | **the record's own four words** (`docs/MODS-MAP.md` §1's legend, §3/§4's *"Unresolved — recorded, not guessed"*). Closed, because a disposition the map does not give is a decision nobody made (FR-002) |
| `Row` *(exists, U6)* | feature, `Source`, `Licence`, `Carrier`, `Acceptance`, `Claim`, `Requirements` | **unchanged**; the tree's row *is* U6's row (D4) |
| `Source` *(exists, U6)* | `path`, `digest` | a document read **in place**; the tree never carries a source's bytes (FR-010) |
| `Licence` *(exists, U6)* | `Permitted`, `Refused` | the row **states** the verdict its source was given; the gate does not decide a licence (`docs/MODS-MAP.md` §1's *"a verdict is per reference"*) |
| `Carrier` *(exists, U6)* | `U3`, `U4`, `U5`, `U6`, `U7`, `U8` | the closed six the plan's §16 names; an undeclared carrier is `carrier_undeclared` |
| `Acceptance` *(exists, U6)* | `Provided(Source)`, `Owed` | the row's own check — **a row whose bar cannot be run yet is shown by name and is never a pass** (FR-009, D3) |
| `Requirement`/`Requirements` *(exists, U6)* | `Named(Vec<T>)`, `Owed`; `stages`, `materials` | what a row requires of the world's own declarations; **absent = requires nothing**, which the conformance axis *reports* rather than counts |
| `Claim` *(exists, U6)* | `ChecksumOnly` (the only claim a request line can carry), `pinned` (an operator's statement out of band) | a checksum establishes integrity and **never** origin; the answer says so rather than carrying a `bool` |

## The disposition, and what each word obliges

| Disposition | Where the record says it | What the entry carries | Refusal, if any |
|---|---|---|---|
| `mapped` | `MODS-MAP` §1's legend: *"the record maps it to a feature"* | **a row** — feature, source, licence, carrier, acceptance, claim, requirements | the row's own axes; a `refused` licence is `licence_refused` |
| `open` | §1's legend: *"the record explicitly left it undecided"*; §2: *"an `open` mod is not a rejection: it means the record assigned no feature"* | **no row**; named owed, with what it waits on where the map says | none — an owed entry is **reported**, and `Establishment::is_established` makes a report no pass |
| `refused` | §1's legend: *"a licence fact forbids deriving from it"* | a row whose `licence` is `Refused` | **`licence_refused`** — the gate's own, already built |
| `unresolved` | §3/§4: *"**Unresolved** — recorded, not guessed"*; *"purpose unclear from the slug"* | **no row**; named owed, exactly as `open` | none — same shape as `open`, with the record's own word kept |

**The disagreement recorded, not smoothed.** `MODS-MAP`'s header says the prior record had **43** `open`
rows; §2 says **24** mods are `open`. The tree takes each entry's disposition from the **row the map gives
it**, and the census prints the count it derived — so the two figures can be compared rather than averaged.

## The canonical request line (unchanged, U6's)

A tree row crosses exactly as the table already declares it — `admit`'s ten arguments, of which
`requires_stages`/`requires_materials` are the only two optional ones:

```text
admit path=<path> digest=<hex> feature=<name> licence=<permitted|refused> claim=checksum_only \
      carrier=<U3..U8> check=<path> check_digest=<hex> [requires_stages=<a[+b…]>] \
      [requires_materials=<id[+id…]>]
```

`clean-room/src/bridge/table.rs` declares this row; `clean-room/src/bridge/request.rs`'s `Request::row`
builds the `Row` from it, and the **same** builder is what verification re-establishes an admission from
(§22) — so a tree row admitted at a call and re-checked later are built by one piece of code.

## State transitions

```text
the record (docs/MODS-MAP.md §1–§5, one row per reference)
    │
    └─ the tree declares one entry per reference, in order
           │
           ├─ disposition = open | unresolved ──▶ Entry { row: None, waits_on }
           │        └─ counted and printed; **never admitted** (a reference with no
           │           feature cannot be a row — `feature` is a required argument)
           │
           ├─ disposition = refused ──▶ Entry { row: Some(Row { licence: Refused, … }) }
           │        └─ admitted ──▶ **refused `licence_refused`** (the owner's own name),
           │                        recorded like any other world-level refusal
           │
           └─ disposition = mapped ──▶ Entry { row: Some(Row { … }) }
                    │
                    ├─ the source document is not readable ──▶ source_absent
                    ├─ its bytes have moved ───────────────▶ source_digest_mismatch
                    ├─ the licence word is outside the two ─▶ licence_undeclared
                    ├─ the carrier is not U3–U8 ────────────▶ carrier_undeclared
                    ├─ the check is absent ─────────────────▶ Acceptance::Owed ⇒ axis_inactive
                    ├─ a required stage the roster lacks ───▶ stage_undeclared (named)
                    ├─ a required material never admitted ──▶ unadmitted_material (id named)
                    └─ otherwise ───────────────────────────▶ Established (never a bool):
                                                              the digest it read, the claim it was
                                                              admitted under, every axis's state
```

## What the tree's census prints, by disposition

| Printed | What it is | Why it can never be read as a pass |
|---|---|---|
| `entries = N` | one per reference, in the map's order | N is compared against the map's own count (SC-001) |
| `mapped = M`, `open = O`, `refused = R`, `unresolved = U` | the four words, each **counted** | the four counts sum to N, so an `open` entry cannot be read as a `mapped` one (SC-002) |
| `rows admitted = A`, `rows refused by name = …` | the admissions and each refusal **with its own name** | a refusal is named, never a bare negative |
| `first entry: <reference> (<class>)` | the ordered head | SC-004: it must be a §1 world-shape-and-streaming row |
| `owed: …` | the four parts the record has not decided (D7, D8, D9, and the per-row assignment) | an owed thing is printed, so its absence is visible rather than silent (`docs/MODS-MAP.md` §6's own rule) |
