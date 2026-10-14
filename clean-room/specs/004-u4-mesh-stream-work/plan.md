# Implementation Plan: U4 — Meshing, streaming and work

**Branch**: `004-u4-mesh-stream-work` | **Date**: 2026-10-11 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/004-u4-mesh-stream-work/spec.md`, governed by
`.specify/memory/constitution.md` (P1–P7) and `./docs/CLEAN-ROOM-PLAN-2026-10-11.md` §2 U4 / §5 slice 4.

**Note**: this file was created by spec-kit's own workflow — `.specify/scripts/bash/setup-plan.sh`
copied `.specify/templates/plan-template.md` here, and the sections below are filled from the spec.
Unlike spec 003's plan (documents only), this plan **is** executed in the same pass: the slices below
are ordered so that the first one can be landed completely.

## Summary

Re-derive meshing as a **derivation of the world's declared fields** — no second terrain function —
and keep the result **coherent with the world it came from**. A chunk becomes greedy-merged rectangles
(a quad is the merge's unit, C-43); each resident entry remembers the world state it was derived from
and is never served stale (SPEC-3); invalidation removes exactly what the old state produced and
nothing else. The work is counted where it happens, so the counts equal an independent recount of the
geometry rather than a formula, and a request that would exceed its declared budget is refused by name
with both numbers. The four horizons are four types (SPEC-5), and every printed number says which
world it is of (C-46).

## Technical Context

**Language/Version**: Rust, edition 2021, `rust-version = 1.98` (the clean-room workspace's pinned
toolchain)

**Primary Dependencies**: `sha2 = "0.10"` only — already present. No new dependency: meshing is index
arithmetic and the geometry identity is U2's existing `Digest`.

**Storage**: none. Resident geometry lives in memory for the life of the process; nothing is persisted.

**Testing**: `cargo test --offline` (unit tests per module **plus** one integration test that drives the
public API end to end), the printed measurements from `src/main.rs`, and
`cargo clippy --offline --all-targets -- -D warnings`.

**Target Platform**: any (integer block heights and one hash; no floating-point in the identity path —
the fields are sampled as `f64` and **rounded to blocks** once, at the mesh boundary, and the rounding
rule is declared).

**Project Type**: library crate with a thin binary (the shape U2 and U3 established)

**Performance Goals**: none claimed as a target. The merge factor (columns → quads) and the per-chunk
counts are *measured* and printed; a throughput claim without a measurement is not made.

**Constraints**: refuse-never-default; determinism; one owner per concern (U2 owns truth and residency,
U3 owns the fields — U4 owns geometry and its work); no `crates/*` import or `path` dependency (P7);
every module header names its document (P1)

**Scale/Scope**: one 32³ chunk per request; the render ladder at the block level; the four typed
horizons; the resident set with exact invalidation; the work ledger; the budget

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Evidence / note |
|---|---|---|
| P1 Fresh code, declared provenance | PASS | the header of every new module names `docs/DESIGN-SPEC.md` §4 SPEC-3/SPEC-5, the plan's U4 entry and `LEDGER.md` C-43/C-46; no meshing source is read from `crates/` |
| P2 Refuse, never default | PASS | FR-004/FR-005/FR-009/FR-010: an unadmitted material, a stale entry, a blown budget and a crossed horizon are each refused **by name**, with the numbers; nothing is truncated or clamped |
| P3 Determinism is a contract | PASS | FR-002/FR-003/FR-007; SC-001 (one digest at 1/2/4/8 workers) and SC-002 (a moved world invalidates exactly its own geometry) |
| P4 Evidence over assertion | PASS | SC-001…SC-006 each name a printed measurement, and SC-004 requires an **independent recount** rather than a formula's own answer |
| P5 One declared interface, two doors | PASS | one `field_at` seam (U3's) and one world truth (U2's); U4 adds no second terrain function and no second cache of the world |
| P6 Smallest sufficient artifact | PASS | one new module (`mesh.rs`) plus the refusal kinds it needs; the displaced/256× level and the multi-chunk stitch are named owed rather than scaffolded (**§40, 2026-10-13:** the stitch landed as one footprint over a rectangle, so the owed item is a *region's own residency*) |
| P7 Legacy is a witness, never a source | PASS | FR-012; the reasoning is taken from `LEDGER.md` C-43/C-46 and `DESIGN-SPEC.md` §4, which are documents, and no `.rs` under `crates/voxel-mesh|stream|work` is read |

**Post-design re-check**: unchanged. Phase 1 added one module and four refusal kinds; no dependency, no
second world copy, no legacy reference.

## Project Structure

### Documentation (this feature)

```text
specs/004-u4-mesh-stream-work/
├── spec.md              # /speckit-specify output (through create-new-feature.sh)
├── plan.md              # this file (/speckit-plan, via setup-plan.sh)
├── research.md          # the decisions, and the alternatives rejected
├── data-model.md        # the entities the geometry and its ledger are made of
├── quickstart.md        # how the acceptance bar is checked, with the pasted measurements
├── contracts/           # the seams this feature must expose
│   └── u4-seams.md
├── checklists/
│   └── requirements.md  # /speckit-specify's quality checklist
└── tasks.md             # /speckit-tasks shape, ticked only where the check was run
```

### Source Code (repository root)

```text
clean-room/
├── Cargo.toml           # unchanged: `sha2` stays the one dependency
├── src/
│   ├── mesh.rs          # NEW: the field-derived surface mesh, the level ladder, the four horizons,
│   │                    #      the resident set with exact invalidation, the work ledger, the budget
│   ├── lib.rs           # register `mesh` on the library surface
│   ├── refusal.rs       # + the geometry refusal kinds (`level_off_ladder`, `geometry_budget_exceeded`,
│   │                    #   `geometry_out_of_residency`, `horizon_mixed`)
│   ├── main.rs          # + `check_u4`: the printed measurements and the bar
│   ├── systems.rs       # U4's real status, replacing `absent`
│   ├── world.rs         # exists (U2) — consumed: identity, tick, anchor, residency, truth digest
│   ├── field.rs         # exists (U3) — consumed: the one `field_at` seam
│   ├── stage.rs         # exists (U3) — consumed: `FieldId`, `Patch`, `ProcessRequest`, `Refusal`
│   ├── process.rs       # exists (U3) — consumed: the roster and the evaluation
│   ├── coordinates.rs   # exists (U2) — consumed: `ChunkKey`, `VoxelPos`, chunk arithmetic
│   └── material.rs      # exists (U2) — consumed: `MaterialId` and the admitted registry
└── tests/
    └── u4_geometry.rs   # NEW: the end-to-end check that drives the public API and asserts the
                         #      lifecycle (derive → hit → move the tick → exact invalidation → budget)
```

**Structure Decision**: **one** new module. The four things this slice needs — the merge, the level
ladder, the resident set and the work ledger — are one capability (geometry coherent with its world),
and splitting them into four files would spread one contract's invariants over four places. The
horizon types live beside them because they are the *types* of the same derivation, and the refusal
kinds live in the existing closed vocabulary rather than in a local enum (P5).

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| — | — | *(no violations)* |

## Execution order for this pass

1. **The refusal kinds** — named, in `src/refusal.rs`, so every later rule has a name to refuse with.
2. **The merge and the level ladder** (`mesh.rs`) — the geometry itself, derived from the fields.
3. **The four horizons** — four types, and the one derived check that can be caught at run time.
4. **The resident set** — the key, the hit/miss rule, and invalidation that removes exactly its own.
5. **The work ledger and the budget** — counted where the work happens; refused by name when exceeded.
6. **The binary and the integration test** — the printed measurements, and the lifecycle driven through
   the public API.
7. **Documents** — `quickstart.md`'s pasted numbers, `tasks.md`'s ticks, `src/systems.rs`, and the
   governing plan's slice-4 row.

## What this slice does **not** land (named, not implied)

- **The displaced 256× surface** (a non-planar quad must be subdivided and triangulated with normals
  from the displaced geometry — C-43). This slice meshes the block level, where a rectangle *is* two
  triangles and nothing is lost.
- **The multi-chunk stitched mesh** and the seam's marching rule: this slice derives one chunk per
  request from its own columns and declares that ownership. **(§40, 2026-10-13:** the stitch **lands** as one
  footprint over a rectangle of chunks (`Mesher::derive_region`), one sweep and one merge, so a boundary inside
  the rectangle is the world's own inside rather than an edge; what remains owed is a stitched mesh **wider**
  than one rectangle, with a residency entry of its own.**)**
- **The strided field request** (a coarse level that does not compute columns nobody reads). It belongs
  in U3's patch vocabulary, not here: a patch is a dense grid because every landscape law needs its
  neighbours, so a coarse level decimates the full patch and the ledger reports both numbers.
- **Persistence and a streamed horizon-driven residency policy**: the resident set is in memory, and the
  policy that decides *which* chunks to keep is U5/U6's ("instrumentation, then the gate").
