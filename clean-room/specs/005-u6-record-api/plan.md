# Implementation Plan: U6 — The record and the API layer (the gate)

**Branch**: `005-u6-record-api` | **Date**: 2026-10-11 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/005-u6-record-api/spec.md`, governed by
`.specify/memory/constitution.md` (P1–P7) and `./docs/CLEAN-ROOM-PLAN-2026-10-11.md` §2 U6 / §5 slice 5.

**Note**: this file was created by spec-kit's own workflow — `.specify/scripts/bash/setup-plan.sh`
copied `.specify/templates/plan-template.md` here, and `.specify/scripts/bash/setup-tasks.sh` reported
the feature directory, the available docs and the tasks template. The sections below are filled from
the spec, and the first slice is implemented in the same pass.

## Summary

Give the clean room **one outward door**: a closed table of verbs, a request that enters as one
declared line, and a **receipt** that carries the answer, the checks that ran, the claims the answer
makes and what the call changed. The answer is derived through the owners' seams (U2's `World` and its
`Chain`, U3's `field_at`, U4's `Mesher`/`Resident`) and its digest is a pure function of what it
**describes**, so the same request twice is one answer while the **effect** — the work U4 counted and
the event the chain appended — is what changed. Refusals are split deliberately: a request that does
not parse against the table writes nothing, and a request the world cannot honour passes the owner's
own refusal name through **and is recorded**, because a refusal is a fact about the world.

## Technical Context

**Language/Version**: Rust, edition 2021, `rust-version = 1.98` (the clean-room workspace's pinned
toolchain)

**Primary Dependencies**: `sha2` only — already present. The gate is parsing, dispatch and digest
folding; nothing new is needed.

**Storage**: none of its own. The record is U2's `Chain`; the geometry is U4's `Resident`; U6 holds
references and one declared table.

**Testing**: `cargo test --offline` (unit tests for the table, the parser and the claims, **plus** one
integration test that drives every verb through the public API), the printed measurements from
`src/main.rs`, and `cargo clippy --offline --all-targets -- -D warnings`.

**Target Platform**: any (integer parsing and one hash; no floating point in any identity path — the
field claim's only float is a digest of the values' bytes, and the printed low/high are readings).

**Project Type**: library crate with a thin binary (the shape U2–U4 established)

**Performance Goals**: none claimed. What is measured is *work*, by U4's ledger, and printed as part of
each receipt's effect.

**Constraints**: refuse-never-default; determinism; one owner per concern (the chain is the record, the
world owns truth, U4 owns geometry — U6 reads them through their seams); no `crates/*` import or `path`
dependency (P7); every module header names its document (P1)

**Scale/Scope**: five verbs over four owners *(corrected 2026-10-12: a sixth row, `admit`, has since
landed — a content row that arrives **by reference** and is read in place; see `tasks.md`'s Phase 9;
and then a seventh and eighth, `save`/`load`, landed the same day for **U9's document form**, so the
table is closed at **eight** verbs — see `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §28)*;
one declared wire form for a request; the receipt and its claims; the parse/world refusal split; the
ledger delta

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Evidence / note |
|---|---|---|
| P1 Fresh code, declared provenance | PASS | the header of `bridge.rs` names `docs/DESIGN-SPEC.md` §2 F1/F2 and the plan's U6 entry; no `crates/voxel-package` or `crates/voxel-mcp` source is read |
| P2 Refuse, never default | PASS | FR-001/FR-002/FR-008/FR-009: an undeclared verb, an absent/undeclared/malformed argument, a world refusal passed through by name, and a claim that does not hold are each refused **by name** with their numbers; nothing is answered vaguely |
| P3 Determinism is a contract | PASS | FR-002 (a canonical request line is the identity), FR-004 (an answer digest is a pure function of what it describes): the same request twice is one answer, printed |
| P4 Evidence over assertion | PASS | SC-001…SC-006 each name a printed measurement; FR-006 requires the receipt to name **which checks ran**, and FR-005 requires each claim to be re-checked against the thing it describes |
| P5 One declared interface, two doors | PASS | the table is closed and bijective (FR-001); one canonical request form; the owners' seams are consumed, never re-implemented (FR-003), and the chain stays the one writer (FR-007) |
| P6 Smallest sufficient artifact | PASS | one new module plus one event kind at its owner; the transport, the content gate and the other verbs of the plan's U6 list are named owed rather than scaffolded |
| P7 Legacy is a witness, never a source | PASS | FR-011; the design is taken from `DESIGN-SPEC.md` §2 and the plan's U6 entry, which are documents |

**Post-design re-check**: unchanged. Phase 1 added one module (`bridge.rs`), five refusal kinds and one
event kind in U2's chain vocabulary; no dependency, no second copy of any owner's state.

## Project Structure

### Documentation (this feature)

```text
specs/005-u6-record-api/
├── spec.md              # /speckit-specify output (through create-new-feature.sh)
├── plan.md              # this file (/speckit-plan, via setup-plan.sh)
├── research.md          # the decisions, and the alternatives rejected
├── data-model.md        # the entities a receipt is made of
├── quickstart.md        # how the acceptance bar is checked, with the pasted measurements
├── contracts/           # the seams this feature must expose
│   └── u6-seams.md
├── checklists/
│   └── requirements.md  # /speckit-specify's quality checklist
└── tasks.md             # /speckit-tasks shape, ticked only where the check was run
```

### Source Code (repository root)

```text
clean-room/
├── Cargo.toml           # unchanged: `sha2` stays the one dependency
├── src/
│   ├── bridge.rs        # NEW: the closed tool table, the request/wire form, the answer, the claims,
│   │                    #      the checks, the effect, the receipt and the gate
│   ├── lib.rs           # register `bridge` on the library surface
│   ├── refusal.rs       # + the gate's refusal kinds (`tool_undeclared`, `argument_undeclared`,
│   │                    #   `argument_absent`, `argument_malformed`, `claim_unverified`)
│   ├── chain.rs         # + one event kind (`geometry_derived`) at the record's owner
│   ├── main.rs          # + `check_u6`: the five verbs, the repeat, the refusals, the verification
│   ├── systems.rs       # U6's real status, replacing `absent`
│   ├── world.rs         # exists (U2) — consumed: identity, tick, truth digest, anchor, residency
│   ├── chain.rs         # exists (U2) — consumed: the one append-only ledger, its tickets and wire
│   ├── field.rs         # exists (U3) — consumed: the one `field_at` seam
│   ├── stage.rs         # exists (U3) — consumed: `FieldId`, `ProcessRequest`
│   ├── process.rs       # exists (U3) — consumed: the reference roster
│   ├── mesh.rs          # exists (U4) — consumed: `Mesher`, `Resident`, `Ladder`, `Level`, `Budget`
│   └── coordinates.rs   # exists (U2) — consumed: `ChunkKey`, `VoxelPos`
└── tests/
    └── u6_bridge.rs     # NEW: the end-to-end check that drives every verb, the repeat, both refusal
                         #      classes and verification against the world it was made in
```

**Structure Decision**: **one** new module. The table, the request form, the receipt and the gate are
one capability — *a request and its checkable answer* — and splitting them would spread one contract's
invariants over four places. The event kind belongs in `chain.rs` because the chain is the record: an
API layer that invented its own event vocabulary would be a second record.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| — | — | *(no violations)* |

## Execution order for this pass

1. **The refusal kinds and the one event kind** — named, in `refusal.rs` and `chain.rs`, so every rule
   below has a name to refuse with.
2. **The tool table and the request form** (`bridge.rs`) — the closed table, the declared argument
   kinds, the parser and the canonical line.
3. **The answer, the claims and the checks** — the answers the owners give, the claims they make, and
   the decisions the gate records.
4. **The gate** — dispatch to the owners' seams, the effect, the receipt, and the parse/world refusal
   split with the refusal recorded through the chain's one writer.
5. **The binary and the integration test** — the printed measurements, and the lifecycle driven
   through the public API.
6. **Documents** — `quickstart.md`'s pasted numbers, `tasks.md`'s ticks, `src/systems.rs`, and the
   governing plan's slice-5 row.

## What this slice does **not** land (named, not implied)

- **The content gate** *(corrected 2026-10-12: its **first slice** has since landed —
  `clean-room/src/intake.rs`, a declared row **consumed in place by reference**, judged on named axes,
  `admit`/`inspect → Established` and never a bool, with the census rule that an unchecked axis is
  never a pass; recorded in `../docs/CLEAN-ROOM-PLAN-2026-10-11.md` §18)*: what remains owed is the
  document record and its compression, the structural judge with its calibration, the ballot, the
  crate-by-axis census over the tree, the permission policy matrix, the recipe format, the API
  document, a row's conformance against the world's own declarations. This slice lands the path all of
  them would be served through. *(Corrected 2026-10-12: **a row's wire form has since landed** —
  `admit` is a row of `bridge::TOOLS`, served over the transport, citing `path + digest` and read in
  place; the conformance check is what remains of that clause.)* *(Corrected again 2026-10-12:
  **the conformance check has landed too** — the seventh axis, `conformance`, judges a row's
  `requires_stages` against U3's `Declared` and its `requires_materials` against U2's
  `MaterialRegistry`, both borrowed through `Intake::over(root, WorldDeclarations { roster, registry })`
  and reached from a served line by the table's two optional arguments; see `tasks.md`'s Phase 10 and
  §21 of the governing plan. What stays owed of that clause is the **served spelling of an owed or
  unnamed requirement**, which the wire refuses by shape rather than inventing.)* *(Corrected again
  2026-10-12: **the permission policy matrix has landed** — `src/policy.rs`, one decision function
  consulted by the gate at one call site before it dispatches, with the act taken from the row's own
  `Tool::action`; see §29 and `contracts/u6-seams.md` C5. It leaves this clause with its own remainder:
  the **self-describing protocol** and the **tool-surface narrowing over a mod-supplied tool list**,
  neither of which has a caller in this build.)*
- **The transport**: serving the table over stdio (`bridge::TOOLS` to a tool transport) and the
  bijectivity of that surface against the legacy action vocabulary.
- **Byte-carrying requests**: the plan's rule is *consume in place by reference* (`path + digest`), so a
  request never carries a document's bytes; the verb that admits one is part of the owed gate.
