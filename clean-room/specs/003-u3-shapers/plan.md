# Implementation Plan: U3 — World generation (the shapers)

**Branch**: `003-u3-shapers` | **Date**: 2026-10-11 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/003-u3-shapers/spec.md`, governed by
`.specify/memory/constitution.md` (P1–P7) and `../docs/CLEAN-ROOM-PLAN-2026-10-11.md` §2 U3 / §5 slice 3.

**Note**: this file was created by spec-kit's own workflow — `.specify/scripts/bash/setup-plan.sh`
copied `.specify/templates/plan-template.md` here and the sections below were filled from the spec.
It is a **plan only**: no U3 code is written in this pass.

## Summary

Re-derive world generation as one deterministic stage graph: the noise primitives and the fifteen
terrain shaper stages, each naming its paper, driven by a cadence runtime that reports per tick how
many stages were due, ran and refused. The world stays a pure function of `(seed, declared period,
tick)`: one digest for one `(seed, tick)`, unchanged by worker count. Every unavailable input is
refused by name; the two stages that cannot yet run (`caves`, `earthworks`) refuse **by design**,
which is what makes the acceptance bar *14 of 16* rather than a claim of 16.

## Technical Context

**Language/Version**: Rust, edition 2021, `rust-version = 1.98` (the clean-room workspace's pinned
toolchain)

**Primary Dependencies**: `sha2 = "0.10"` only — already present for content identity (U2). No new
dependency: the noise primitives are arithmetic (gradient/simplex/cellular/fBm/ridged/domain-warp),
so they are re-derived, not imported.

**Storage**: none. A generated world is values plus a `Digest`; nothing is persisted in this slice.

**Testing**: `cargo test --offline` (unit tests in each new module) plus the printed measurements in
`src/main.rs`; `cargo clippy --offline --all-targets -- -D warnings`.

**Target Platform**: any (integer/fixed-point stage math and one hash; no floating-point required in
the identity path)

**Project Type**: library crate with a thin binary (the shape U2 already established)

**Performance Goals**: none claimed as a target in this slice. Worker count is a *knob* whose only
contract is that it changes no output (SC-001); a throughput number is not asserted without a
measurement.

**Constraints**: refuse-never-default; determinism; one owner per concern; no `crates/*` import or
`path` dependency (P7); every module header names its document (P1)

**Scale/Scope**: fifteen terrain stages + the primitives they stand on; the closed four-rung cadence
ladder; the 20-level regional field ladder; a 32³ chunk at the stored 4×4×4 lattice with the 256×
addressed level above it

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Evidence / note |
|---|---|---|
| P1 Fresh code, declared provenance | PASS | FR-003/FR-010: every stage names the paper or ruling it is built from; every new module header names the document it came from; a stage that cannot name its source is refused at admission |
| P2 Refuse, never default | PASS | FR-006/FR-007/FR-008; the two deliberate refusals (`caves`, `earthworks`) are named, and `14 of 16` is the honest count |
| P3 Determinism is a contract | PASS | FR-001/FR-002/FR-005; SC-001 (one digest across 1/2/4/8 workers) and SC-002 (a moved tick moves the digest) |
| P4 Evidence over assertion | PASS | SC-001…SC-007 each name a printed measurement; a stage reported as running without a printed count is `unchecked`, never a pass |
| P5 One declared interface, two doors | PASS | one `Declared` roster, one `ProcessScheduler`, one `field_at` seam — no second stage list and no second field path |
| P6 Smallest sufficient artifact | PASS | the roster is closed at fifteen; `earthworks` is bodyless by design rather than stubbed, and nothing is scaffolded to reserve a name |
| P7 Legacy is a witness, never a source | PASS | FR-009: no `crates/voxel-noise` or `crates/voxel-shapers` import or `path` dependency; the laws come from the papers `docs/SHAPERS.md` and `docs/MISSING-SYSTEMS.md` cite |

**Post-design re-check**: unchanged — Phase 1 produced documents only (no code), so no new
dependency, no second roster, and no legacy reference was introduced.

## Project Structure

### Documentation (this feature)

```text
specs/003-u3-shapers/
├── spec.md              # /speckit-specify output (through create-new-feature.sh)
├── plan.md              # this file (/speckit-plan, via setup-plan.sh)
├── research.md          # Phase 0: the decisions, and the alternatives rejected
├── data-model.md        # Phase 1: the entities the stage graph is made of
├── quickstart.md        # Phase 1: how the acceptance bar is checked once built
├── contracts/           # Phase 1: the declared seams this feature must expose
│   └── u3-seams.md
├── checklists/
│   └── requirements.md  # /speckit-specify's quality checklist
└── tasks.md             # Phase 2 (/speckit-tasks shape — no U3 code in this pass)
```

### Source Code (repository root)

```text
clean-room/
├── Cargo.toml           # unchanged: `sha2` stays the one dependency
└── src/
    ├── lib.rs           # refuses-adding nothing here beyond the new modules
    ├── digest.rs        # exists (U2) — the one content identity
    ├── coordinates.rs   # exists (U2) — signed coordinates, lattices, the field ladder's arithmetic
    ├── material.rs      # exists (U2) — materials the shapers place
    ├── shaper.rs        # exists (U2) — ShaperKind × Level, the contract each stage obeys
    ├── chain.rs         # exists (U2) — the record `earthworks` writes into
    ├── noise.rs         # NEW: the primitives (gradient, simplex-style, cellular, fBm, ridged, warp)
    ├── stage.rs         # NEW: `Stage`, `Declared`, the stage graph and its validation
    ├── cadence.rs       # NEW: the closed four-rung ladder and the tick driver's receipt
    ├── process.rs       # NEW: `ProcessScheduler`, unit/region requests, per-tick due/ran/refused
    ├── field.rs         # NEW: the field plane — `field_at` and the declaring stage's routing
    └── shapers/         # NEW: the fifteen stages, one file per group of related laws
        ├── terrain.rs   #   tectonics, fields, geology, terrain
        ├── water.rs     #   hydrology, substrate, erosion
        ├── air.rs       #   climate, atmosphere, weather
        ├── life.rs      #   caves, ecology
        └── record.rs    #   delta, transition, earthworks
```

**Structure Decision**: one module per concern, with the fifteen stages grouped by the medium they
act through rather than one file per stage — the repository's *smallest sufficient artifact* rule
(P6) prefers few large, well-named files over fifteen stubs. Each `shapers/*.rs` file carries the
papers it implements in its header, so provenance stays per-module (P1) without one file per law.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| — | — | *(no violations: this plan adds no dependency, no second roster, no legacy reference, and no stub)* |
