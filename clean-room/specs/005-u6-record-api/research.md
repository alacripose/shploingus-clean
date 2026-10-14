# Phase 0 — Research: U6 the record and the API layer

**Feature**: `005-u6-record-api` | **Date**: 2026-10-11 | **Spec**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

**What this phase had to resolve.** The spec names three properties — a request enters and its answer
comes back with its evidence; one request twice is one answer while the record shows what changed; a
request that cannot be honoured is refused by name. This file records the decisions that had to be
**made** before a design could be written, each with its rationale and the alternative rejected. All of
them are re-derived from documents this repository already holds (`docs/DESIGN-SPEC.md` §2 F1/F2, the
**U6** entry of `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §2, and §3's fresh-code boundary) and from the
seams U2–U4 already landed. **No legacy source was read** (P7), and no external source was needed.

## D1 — The table is a value, not a `match` on strings

- **Decision**: a verb is a closed enum ([`Verb`](../spec.md)), an argument is a declared
  `(name, kind, required)` triple, and a tool is a row of a `static` table naming its arguments, its
  owner seam, whether it may change the record, and the checks it runs.
- **Rationale**: F2's *"an interface declared, not described"* is only checkable if the declaration is
  a value. With a table, bijectivity (one name per verb, one value per key) is a test, and "which
  checks ran" is the row's own list rather than something a reader has to infer from control flow.
- **Alternatives considered**: (a) `match verb_name { "truth" => …, _ => refuse }` — rejected: a typo in
  a call site is a compile error only for the table, not for the caller, and the arguments would be
  described in prose; (b) a runtime registry built at start-up — rejected: a runtime table can be
  built differently in two runs, and the determinism contract (P3) applies to the gate too.

## D2 — An answer's digest is a pure function of what the answer **describes**

- **Decision**: `Answer::digest()` folds the described facts only — the world's identity and truth, a
  field's values digest and column count, a mesh's digest and counts, the resident set's counts and
  digest, the record's length and head. The worker count, the ledger totals and whether the geometry
  came from a derivation or the resident set are deliberately **absent**.
- **Rationale**: FR-004, and the whole of US2. If the digest carried the work, a cache hit would be a
  different answer from the derivation it served — and an agent loop that asks the same question twice
  would see the world change when nothing did.
- **Alternatives considered**: (a) folding the ledger into the digest — rejected: it makes the answer
  a report about the machine; (b) leaving the answer **without** a digest and comparing the facts by
  hand — rejected: the plan's acceptance check is a `Receipt`, and a receipt whose answer has no
  identity cannot be compared across calls.

## D3 — Claims are re-checked, and a field's values are the one claim that is re-evaluated

- **Decision**: every answer carries claims, and `Claim::check` asks **the owner** again — the world's
  truth digest, the resident set, the chain. A field's values are re-evaluated through `field_at`,
  because a field has no stored form: the world is a function of `(seed, period, tick)`.
- **Rationale**: FR-005/FR-009. Verification must be against the thing the answer describes. Copying
  the field's values into the receipt would be a second copy of the world (SPEC-3), and recomputing a
  mesh would be a second geometry path — so the held mesh is **read** and a field is **evaluated**.
- **Consequence, stated because it is observable**: a receipt whose geometry was evicted fails
  `mesh_is_held` while its `truth_is` claim still holds. That is the difference between *read* and
  *recomputed* made measurable, and it is the spec's own edge case.
- **Alternatives considered**: (a) recomputing the mesh and comparing digests — rejected: it would
  pass after eviction, which is exactly the false confidence the edge case forbids; (b) carrying the
  values — rejected: a second copy, and one that can drift.

## D4 — The refusal split: a parse fault writes nothing, a world refusal is recorded

- **Decision**: `bridge::parse` refuses an undeclared verb, an undeclared argument, an absent required
  argument, a value not of its declared kind and a duplicate key — and `Gate::call` returns before
  touching the world or the record. Anything that comes out of **dispatch** (a level off the ladder, a
  chunk outside the residency horizon, an undeclared field, a stage that owes its body, an unmet
  budget) is recorded as a `refusal_recorded` event and then passed through by name.
- **Rationale**: F1 cuts both ways. A refusal nobody recorded is invisible — an agent that asked for
  geometry and was refused would leave no trace of the question, which is precisely the fact the record
  exists to hold. But a line that does not parse is not an event about the world: it names no chunk, no
  level and no field, so writing it would put non-truth in the world's memory.
- **Alternatives considered**: (a) record both — rejected: a malformed line has no world coordinates to
  record; (b) record neither — rejected: it breaks F1 for the refusals that *are* facts; (c) record the
  parse fault as a *diagnostic* in a separate log — rejected: that is a second record (SPEC-3).

## D5 — The new event vocabulary lives at the record's owner, not in the gate

- **Decision**: the one new event kind (`geometry_derived`) and the existing `refusal_recorded` are
  added to `chain.rs`'s closed `EventKind`, and both live in the chain's `record` namespace. The gate
  names no event kind of its own.
- **Rationale**: the chain **is** the record (SPEC-3). An API layer with its own event table would be a
  second record, and the wire form would have two vocabularies. It also keeps `Bridge` free of any
  durability decision.
- **Alternatives considered**: (a) a `bridge`-owned "log" of calls — rejected: a second record that can
  disagree with the first; (b) reusing `world_opened` for derivations — rejected: a kind that means two
  things is a kind that means nothing.

## D6 — The gate owns the owners' **values**, and mutates them through the owners' own methods

- **Decision**: `Gate` holds U2's `World` and `Chain`, U4's `Resident` and `WorkLedger` (by value) and
  borrows U3's roster and U4's ladder/horizon. It exposes `world_mut`, `chain_mut` and `resident_mut`,
  which are U2's and U4's own types, so a caller moves the world with `World::with_tick`, files a ticket
  with `Chain::file_ticket`, and evicts with `Resident::invalidate` — the owners' own operations.
- **Rationale**: the mission's ownership rule — *the chain is the record, the world owns truth, U4 owns
  geometry* — forbids the gate to keep a parallel copy. Holding the values (rather than a copy of their
  **logic**) is what lets one object answer a request and then be re-verified against the same owners.
- **Alternatives considered**: (a) the gate holding `&mut` references to everything — rejected: the
  receiver would then be unusable for the verification that follows the call; (b) the gate re-deriving
  the truth digest itself — rejected: a second truth path.

## D7 — One coherent path, not a survey: five verbs over four owners

*(Amended 2026-10-12: the table has since widened **to eight** — `admit` by reference (§19), then
`save`/`load` for U9's document form (§28). The decision recorded below is unchanged: this slice lands
**this** path and names the rest owed; what moved is which of those named items have since landed.
Amended again the same day: of the owed list below, the **content gate** (§18–§22) and the **policy
matrix** (§29) have landed; the judge, the ballot, the census, the recipe format, the API document and
the legacy-vocabulary bijection are still owed.)*

- **Decision**: this slice lands `truth`, `fields`, `geometry`, `held` and `record` — the request, the
  answer, the claims, the checks, the effect, the receipt and the two refusal classes — and names the
  rest of U6's list (the content gate, the judge, the ballot, the census, the policy matrix, the recipe
  format, the API document, the transport) as **owed**.
- **Rationale**: P6 (smallest sufficient artifact) and the mission's "depth over breadth". Every one of
  the owed capabilities is *served through* a receipt, so landing the receipt first is the order that
  makes the rest smaller rather than larger.
- **Alternatives considered**: stubbing `admit`/`inspect` so the table "covers" U6's whole list —
  rejected: a stub that returns a plausible `Established` is exactly the invented default P2 forbids,
  and it would put a false row on the outward door.

## D8 — The budget is the gate's declared knob, not a request argument

- **Decision**: `geometry` takes `chunk` and `level` and nothing else; the ceiling on a derivation's
  work is `Gate::set_budget`, defaulting to unlimited.
- **Rationale**: U4 already owns the refusal (`geometry_budget_exceeded`, with both numbers); the gate
  only needs to be *able* to declare a ceiling so that refusal is reachable through the outward door.
  Adding a `budget` argument would make every caller re-declare a policy the gate is the right owner
  of, and the table's rule is one value per key.
- **Alternatives considered**: (a) a `budget` argument on the row — rejected as above; (b) no budget at
  all — rejected: it would make U4's own refusal unreachable from the only outward door, which is how a
  seam quietly stops being exercised.

## Open questions this phase hands to the implement pass

- **The transport** (`bridge::TOOLS` served over stdio, the legacy `voxel-mcp` shape) is owed and named
  rather than stubbed: a transport is a *serialisation* of the table, and the table is the contract.
- **The content gate** — `admit`/`inspect` returning `Established` (never a bool), the document record
  and its compression, the structural judge and its calibration, the ballot, the crate-by-axis census,
  the permission policy matrix, the recipe format and the API document — is owed, and its verbs will be
  rows of this same table. *(2026-10-12: the gate landed (§18–§22) and the matrix landed (§29); a
  **served** row still carries only the requirements its own row declares.)*
- **A request never carries bytes.** The plan's Lane B rule is *consume in place by reference*
  (`path + digest`); the verb that admits a document therefore has an argument kind this slice does not
  declare, and adding it is a decision for that verb's own pass.
