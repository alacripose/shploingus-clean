# Phase 1 — Data model: U3 world generation (the shapers)

**Feature**: `003-u3-shapers` | **Date**: 2026-10-11 | **Spec**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

This is the entity model the stage graph is built from. It is a **model**, not code: no Rust is
written in this pass. Entities already built in U2 are marked `exists (U2)` and are consumed, not
redefined.

## `Declared` — the world's declared stage set

The value that makes "which stages ran, in what order" a *reading*.

| Field | Type (model level) | Rule |
|---|---|---|
| `stages` | ordered list of `Stage` | each stage appears **once**; the order is the order of the effects |
| `ladder` | the closed four-rung `Cadence` ladder | a rung outside it is refused by name |
| `seed`, `period` | from `WorldIdentity` `exists (U2)` | identity-bound, immutable |

**Validation**: a stage reading a stage declared *below* it is refused by name. Two stages declaring
the same output are refused by name. A stage that names no source is refused by name (FR-003).

## `Stage` — one declared producer

| Field | Meaning | Rule |
|---|---|---|
| `name` | the stage's noun (e.g. `hydrology`) | unique within `Declared` |
| `source` | the **one** paper or ruling it is built from | empty source ⇒ refused at admission (FR-003) |
| `reads` | the stages whose outputs it consumes | each must be declared above it (FR-002) |
| `writes` | its declared outputs (fields, or the stored/addressed lattice) | non-empty: a stage that writes nothing is refused by name |
| `cadence` | its rung on the closed ladder | a rung outside the ladder is refused by name |
| `dials` | the law's coefficients/exponents/thresholds | **no invented default**; unset ⇒ refuse by name (D6) |

**Relationship to U2's `ShaperSet`**: `ShaperKind × Level` (`exists (U2)`) is the taxonomy of
*effects* a stage may have — a **physical** shaper changes occupancy, a **material** shaper cannot.
U3's `Stage` is the *producer*; it declares effects in that taxonomy rather than replacing it.

## `Cadence` — the closed four-rung ladder

- Closed set of four rungs; each `Stage` names one.
- Not a duration in wall-clock time: it is a **schedule** expressed in ticks, so two runs agree.

## `ProcessScheduler` and `ProcessRequest`

| Entity | Fields (model level) | Rule |
|---|---|---|
| `ProcessScheduler` | the `Declared` set, the current tick, the per-tick tally | reports `due`, `ran`, `refused` for a tick, naming each refusal |
| `ProcessRequest::unit` | one site (a coordinate + the fields that site needs) | a law definable only over a region refuses this by name (FR-008) |
| `ProcessRequest::region` | a site plus its neighbours, **borrowed from the caller** | the runtime does not invent a neighbourhood; it consumes the one it was given |

**Idempotence**: driving tick *N* twice produces the same tally and the same digest; driving an
earlier tick over a later one is refused by name (FR-005).

## `TickDriver` / `TickReceipt` — the typed evidence that a tick happened once

| Entity | Fields | Rule |
|---|---|---|
| `TickDriver` | the scheduler, the last driven tick | plans a tick; refuses a rewind |
| `TickReceipt` | the tick, the plan digest, the `due`/`ran`/`refused` tally, the refusal names | a receipt without a digest is not a receipt |

## `field_at` — the field plane's one seam

- **Input**: a declared field id and a site (or region, for region-defined fields).
- **Output**: the field's value, produced by the stage that **declares it as an output**.
- **Refusal**: a field with no declaring stage ⇒ refused by name (D5); a region-only field asked at
  a site ⇒ refused by name.

## The fifteen stages (roster; sources in [spec.md](./spec.md) § *The shapers*)

`tectonics`, `fields`, `geology`, `terrain`, `hydrology`, `substrate`, `erosion`, `climate`,
`atmosphere`, `weather`, `caves`, `ecology`, `delta`, `transition`, `earthworks`.

| Stage | Request form | Status |
|---|---|---|
| `tectonics`, `fields`, `geology`, `terrain` | unit | built |
| `hydrology` | region (routing, accumulation, flood) | built — discharge, stream order (Strahler 1952), lake fill (Barnes et al. 2014) |
| `substrate`, `erosion` | region (pairwise flux) | built — soil depth, stream power, and the hillslope divergence whose exchanges cancel over the closed region |
| `climate`, `atmosphere`, `weather` | unit, stage-coupled to terrain | built |
| `ecology` | unit columns, one pass | built — the province, Lieth's capacity and Reynolds' flock |
| `delta` | region (accumulation + accommodation) | built — deposition where the flow stops carrying and water stands; **avulsion owed** with the state |
| `transition` | region only | built — the partition of unity over the region's rounded domain pair; a tie refuses (`domain_tie`) |
| `caves` | region | **refuses by name** — its occupancy field is not one of the declared fields |
| `earthworks` | — | **bodyless by design**: the record, not a generator |

## `Digest` and the chain (`exists (U2)`, consumed here)

- **`Digest`** — the one content identity. U3's contract is one digest per `(seed, tick)`, and the
  same across worker counts (SC-001).
- **`Chain`** — the append-only record. `earthworks` writes here; U3 does not invent a second ledger.

## State transitions

```text
declared roster ──admit──▶ Stage set (each stage: source, reads, writes, cadence, dials)
      │                          │
      │                          ├── validate: source named? order respected? outputs unique?
      │                          └── refused by name at admission if not
      ▼
tick N ──plan──▶ TickReceipt(digest, due/ran/refused) ──drive──▶ stages run (idempotent)
      │                                                              │
      └── rewind (tick < last) ──▶ refused by name                    ▼
                                                          field_at answers from the
                                                          declaring stage (or refuses)
```
