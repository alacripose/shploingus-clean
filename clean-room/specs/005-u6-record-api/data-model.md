# Phase 1 — Data model: U6 the record and the API layer

**Feature**: `005-u6-record-api` | **Date**: 2026-10-11 | **Spec**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

## What U6 owns, and what it only consumes

| Concern | Owner | How U6 uses it |
|---|---|---|
| world truth — identity, tick, declared period, anchor, residency cache | U2 (`World`) | read through `identity`, `tick`, `truth_digest`; the caller moves it through `World::with_tick`, never through a gate-invented operation |
| the record — the one append-only chain, its tickets and its wire form | U2 (`Chain`) | appended to through `Chain::append` (the one writer); read through `len`, `head`, `events`, `wire`. Never mirrored |
| the declared fields and their values | U3 (`field_at`, `fields_at`) | one `field_at` call per `fields` request, over a `ProcessRequest::Region` |
| geometry, its level ladder, its horizons, its residency and its work | U4 (`Mesher`, `Resident`, `WorkLedger`, `Ladder`, `ResidencyHorizon`, `Budget`) | `Mesher::derive` for a miss, `Resident::serve` for a hit; the ledger is read as a **delta** |
| the closed tool table, the request form, the answer, the claims, the checks, the effect, the receipt | **U6** | this feature |

## The entities

| Entity | Fields | Rule |
|---|---|---|
| `Verb` | `Truth`, `Fields`, `Geometry`, `Held`, `Record` | closed, so dispatch is a total `match`; one row per verb (bijectivity is a test) |
| `ArgKind` | `Chunk`, `Level`, `Field`, `Voxel`, `Side` | knows only a value's **shape**. In-domain is the owner's decision, taken at dispatch |
| `Argument` | `name`, `kind`, `required` | declared once per row; one value per key, and this slice declares every argument required |
| `Tool` | `verb`, `name`, `args`, `answers`, `owner`, `changes_record`, `checks` | one row of the outward door; `changes_record: false` is a promise the effect is checked against |
| `CheckName` | `RequestParsed`, `ArgumentsTyped`, `OwnerAnswered`, `ClaimsHold`, `Recorded` | closed; a row declares the subset it runs |
| `Request` | `tool`, `values` (declared order), `canonical` | private fields, so the only constructor is `parse` — which is what makes "every argument is present and of its kind" a property |
| `Answer` | `Truth`, `Fields`, `Geometry`, `Held`, `Record` | the facts an owner answered. **No** variant carries a worker count, a ledger total or a hit/miss — so `digest()` is about the answer, not the work |
| `Claim` | `TruthIs`, `FieldIs`, `MeshIsHeld`, `HeldIs`, `RecordHeadIs`, `EventIs` | a question the gate can ask the owner **again**; `check` returns `claim_unverified` naming the claim, the recorded value and the found value |
| `Check` | `name`, `subject`, `outcome` | one decision the gate made: named, and about something |
| `Effect` | the ledger **delta**, `held_before/after`, `events_before/after`, `appended` | what the call changed. A read changes nothing and appends nothing |
| `Receipt` | `request`, `answer`, `claims`, `checks`, `effect` | the record of one call, and the unit `verify` checks |
| `Owners` | `world`, `roster`, `resident`, `chain`, `workers` | the borrows a claim is checked against — so a receipt can be verified against a **different** world than the one it was made in |
| `Gate` | roster, mesher, world, chain, resident, ledger, budget, workers | the object a caller hands a line to. It owns the owners' **values** and copies none of their logic |

## The canonical line

```text
<verb> [<arg>=<value>]…          arguments in **declared** order, single-spaced

truth
fields field=elevation at=0,0,0 side=8
geometry chunk=0,0,0 level=0
held
record
```

The canonical line is the request's **identity**: `split_whitespace` at parse and declared order at
build mean two spellings (`fields side=8 at=0,0,0 field=elevation` and `fields field=elevation at=0,0,0
side=8`) are one value, one request and one answer. It is also what a recorded refusal names, so a
reader of the record sees the question that was refused.

## State transitions

```text
line
 │
 ├─ parse against the table ──fault──▶ Err(by name)          nothing written, nothing touched
 │                                      (tool_undeclared, argument_undeclared, argument_absent,
 │                                       argument_malformed)
 └─ dispatch to the owner's seam
        │
        ├─ owner refuses by name ──▶ record `refusal_recorded` ──▶ Err(the owner's own name)
        │      (level_off_ladder, geometry_out_of_residency, geometry_budget_exceeded,
        │       field_undeclared, body_owed, …)
        │
        └─ owner answers
               │
               ├─ the answer's claims are re-checked **now**; a claim that does not hold is
               │   recorded and refused (`claim_unverified`) and no receipt is handed out
               │
               └─ the change is recorded: a `geometry` **miss** appends `geometry_derived`
                  (chunk, level, quads, truth, mesh digest); a **hit** appends nothing; a read
                  (`truth`, `fields`, `held`, `record`) appends nothing at all
                     │
                     └─ Receipt { request, answer, claims, checks, effect }
                            │
                            └─ Receipt::verify(&owners) — every claim, every failure collected
```

## What the record holds after a call, by verb

| Verb | Changed the record? | Ledger work | Held geometry |
|---|---|---|---|
| `truth` | no | none | unchanged |
| `fields` | no | none (U3's work is not U4's ledger) | unchanged |
| `geometry` (miss) | **yes** — one `geometry_derived` | 1024 columns produced, 1 miss | +1 entry |
| `geometry` (hit) | no | 0 columns, 1 hit | unchanged |
| `held` | no | none | unchanged |
| `record` | no | none | unchanged |
| any world-level refusal | **yes** — one `refusal_recorded` | U4's own refusal count, where it was decided | unchanged (nothing cached) |
| any parse fault | **no** | none | unchanged |
