# Implementation Plan: U9 — Saves and loads, and the world's own seed systems

**Branch**: `006-world-saves-and-seeds` | **Date**: 2026-10-12 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/006-world-saves-and-seeds/spec.md`, governed by
`.specify/memory/constitution.md` (P1–P7, read live) and `docs/CLEAN-ROOM-PLAN-2026-10-11.md`
**§16.1 U9** with **§5 row 9**.

**Note**: this file was created by spec-kit's own workflow — `.specify/scripts/bash/setup-plan.sh`
copied `.specify/templates/plan-template.md` here, and the sections below are filled from the spec.
Unlike spec 003's plan (documents only), this plan **is** executed in the same pass: the replay-form
slice below lands completely, and the owed document form is named rather than half-built. *(Corrected
2026-10-12, §28 of the governing plan: the document form, named owed here, has since landed too — see the
paragraph under "What this slice does **not** land" below and `contracts/u9-seams.md` C4.)*

## Summary

A world is **written down as one exact byte string** — its identity (`seed`, declared period, tick),
the five declared sub-streams' own seeds, the configuration/ruleset identity it was made under, and
the one record — and **read back as the same world**: the loaded world's `truth_digest` equals the
live world's, the loaded chain is the same chain and verifies, and the world rebuilt from the save
generates the **same one digest whatever the worker count** (U3's instrument, `460214b1…2fc6`). Nothing
is derived that can be recorded (`F1`: replay inputs are recorded, not derived), and nothing is
defaulted: a save missing a stream, naming another ruleset, carrying another record, or altered by one
byte is **refused by name** with both values.

## Technical Context

**Language/Version**: Rust, edition 2021, `rust-version = 1.98` (the clean-room workspace's pinned
toolchain)

**Primary Dependencies**: `sha2 = "0.10"` only — already present. No new dependency: the save's
integrity is U2's existing `Digest`, and the encoding is integer arithmetic.

**Storage**: **a file, in memory and on disk.** A save is bytes; the evidence writes them to one fixed
path under the system temp directory and reads them back, so the round trip is a real file round trip
rather than a buffer in one process. Nothing else is persisted.

**Testing**: `cargo test --offline` (unit tests in the save subsystem, `src/save/tests.rs` **plus** one
integration test that
drives the public API and a real file), the printed measurements from `src/main.rs`, and
`cargo clippy --offline --all-targets -- -D warnings`.

**Target Platform**: any (fixed little-endian integers and one hash; no floating point anywhere in the
encoding, so two platforms write one byte string).

**Project Type**: library crate with a thin binary (the shape U2, U3, U4 and U6 established)

**Performance Goals**: none claimed. The measured numbers are the byte length of a save, the identity
and record lengths, and one generation digest at 1/2/4/8 workers; a throughput claim is not made.

**Constraints**: refuse-never-default; determinism; one owner per concern (U2 owns identity, the
`Domain` set and the record — U9 owns the written form and nothing else); no `crates/*` import or
`path` dependency (P7); every module header names its document (P1)

**Scale/Scope**: one save of one world at one tick, carrying the whole record; the replay form only

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Evidence / note |
|---|---|---|
| P1 Fresh code, declared provenance | PASS | the header of the new module names `docs/DESIGN-SPEC.md` §1.3/§2 F1/SPEC-3, the plan's §16.1 U9, `LEDGER.md` C-96.1 and the H-7.20 line; no `.rs` under `crates/` is read |
| P2 Refuse, never default | PASS | FR-004/FR-005/FR-008: a missing stream, a mismatched ruleset, a diverged record, a truncated or altered save are each refused **by name**, with the numbers; no partial world is ever produced |
| P3 Determinism is a contract | PASS | FR-001/FR-003/FR-009; SC-001 (save → load → one `truth_digest`, and one generation digest at 1/2/4/8 workers), SC-006 (identical bytes for two saves of one world) |
| P4 Evidence over assertion | PASS | SC-001…SC-007 each name a printed measurement; the file round trip is measured through the filesystem rather than asserted |
| P5 One declared interface, two doors | PASS | one record (U2's `Chain`), one identity (U2's `WorldIdentity`), one hash (U2's `Digest`) and one refusal vocabulary; U9 adds no second table, no second record and no second rule about what "the same seed" means |
| P6 Smallest sufficient artifact | PASS | one new module (`save/`: the vocabulary and its writer/parser, the byte layout, the loader) plus the refusal kinds it needs and one small addition at U2's owner (`World::stream_seed`); the document form is named owed, not scaffolded |
| P7 Legacy is a witness, never a source | PASS | FR-011; the contract is taken from documents (`LEDGER.md` C-96.1, H-7.20, `DESIGN-SPEC.md` §1.3, `GAIA.md` §5c/§11), and no legacy save format is carried |

**Post-design re-check**: unchanged. Phase 1 added one module, six refusal kinds and one method on
U2's `World`; no dependency, no second record, no legacy reference.

## Project Structure

### Documentation (this feature)

```text
specs/006-world-saves-and-seeds/
├── spec.md              # /speckit-specify output (through create-new-feature.sh)
├── plan.md              # this file (/speckit-plan, via setup-plan.sh)
├── research.md          # the decisions, and the alternatives rejected
├── data-model.md        # the entities, and the exact byte layout
├── quickstart.md        # how the acceptance bar is checked, with the pasted measurements
├── contracts/           # the seams this feature must expose
│   └── u9-seams.md
├── checklists/
│   └── requirements.md  # /speckit-specify's quality checklist
└── tasks.md             # /speckit-tasks shape, ticked only where the check was run
```

### Source Code (repository root)

```text
clean-room/
├── Cargo.toml           # unchanged: `sha2` stays the one dependency
├── src/
│   ├── save/mod.rs      # NEW: what a save **is** — the vocabulary and the writer/parser
│   ├── save/layout.rs   # NEW: the declared bytes: `MAGIC`/`VERSION`, the one encoder, the cursor
│   ├── save/load.rs     # NEW: the identity correspondence checks, the loader and the load report
│   ├── save/tests.rs    # NEW: the layout, the parser and the loader driven directly
│   ├── lib.rs           # register `save` on the library surface
│   ├── refusal.rs       # + the save refusal kinds (`not_a_save`, `save_truncated`, `save_tampered`,
│   │                    #   `seed_undeclared`, `identity_mismatch`, `record_diverged`)
│   ├── world.rs         # + `World::stream_seed(domain)` — the per-domain seed a save carries
│   ├── evidence.rs      # + `check_u9`: the printed measurements and the bar
│   ├── main.rs          # + the U9 check in the binary's aggregate verdict
│   ├── reference.rs     # + `identity()`, so the reference world has **one** declaration
│   ├── systems.rs       # U9's real status, and the five mission systems on the registry
│   ├── chain.rs         # exists (U2) — consumed: `append`, `wire`, `from_wire`, `verify`, head/len
│   ├── digest.rs        # exists (U2) — consumed: the one content identity
│   ├── process.rs       # exists (U3) — consumed: the roster and the worker-count digest
│   └── coordinates.rs   # exists (U2) — consumed: nothing (a save names no chunk)
└── tests/
    └── u9_saves.rs      # NEW: the end-to-end check that drives the public API and a real file
```

**Structure Decision**: **one** new module. The encoding, the parser and the identity-correspondence
checks are one contract — "these bytes are that world" — and splitting the writer from the reader is
how a format drifts into two formats (P5). The per-domain seed belongs at U2's owner rather than in
the save subsystem, because *what a world's sub-streams are* is U2's decision and a second answer here
would be a second vocabulary.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| — | — | *(no violations)* |

## Execution order for this pass

1. **The refusal kinds** — named, in `src/refusal.rs`, so every rule below has a name to refuse with.
2. **`World::stream_seed(domain)`** — the per-domain seed at U2's owner, so the save carries five real
   stream seeds rather than one dial renamed.
3. **The encoding and the parser** (`save/layout.rs`) — one declared byte layout, exact and
   length-framed.
4. **The correspondence checks** — ruleset, streams, record head/length, and the loaded truth digest.
5. **The loader and the load report** — what a caller gets back, and what it can print.
6. **The binary and the integration test** — the file round trip, the digests at 1/2/4/8 workers, the
   six refusals; and the public surface driven end to end.
7. **Documents** — `quickstart.md`'s pasted numbers, `tasks.md`'s ticks, `checklists/requirements.md`,
   `src/systems.rs`, and this feature's row in the governing plan.

## What this slice does **not** land (named, not implied)

- **The document form of a save** — a save that crosses the API as a document consumed in place by
  reference (`path + digest`). That is where **U6's content gate** (`admit`/`inspect` → `Established`)
  becomes a **hard** prerequisite, and it is why the gate is *adjacent* to this slice rather than
  in front of it (§16.2's ruling).
- **`save`/`load` as rows of the one tool table** (U6's door). The table is closed at five verbs today;
  adding two is a U6 change. This slice's surface is the library's and the binary's.

**Landed after all, on 2026-10-12 (§28) — the two bullets above, in the order they were named.** The
**document form of a save** now exists: `src/save/document.rs` is the one writer
(`Document::write`, U9's own encoder placing its bytes under the gate's declared root) and the one
reader (`Save::read_document`, which reads **through** U6's `Intake::read` and has no `fs::read` of its
own), so *"consumed in place by reference"* and *"the content gate is hard"* are the shape of the code
rather than a policy. And **`save`/`load` are rows of the one tool table** — its seventh and eighth,
added in U6's own table, carrying the reference (`path` and, for `load`, the `digest` the bytes must
hash to) and never bytes. Both were still owed when this plan was written; the plan step's ruling that
they wait on **U6's content gate** is what made the gate a hard prerequisite for exactly one of them,
and §28 records the measurement.
- **A save of a *resident* set.** A save is the record plus identity; residency is a cache and is
  deliberately not carried (U2 keeps them apart, and F1 records inputs rather than derived state).
- **Compression, chunking or a versioned migration path.** One version, one layout; a byte string that
  is not this layout is refused, not reinterpreted (FR-008).
- **A streamed residency policy** (U4's) and the planet's scale contract (U11's) — a save does not
  decide what stays resident.
