# Plan 001 — U1 Presentation

## 1. Summary

Implement the appearance contract as one pure, dependency-free Rust module: `Surface`/`Light`/`View`
declared types, the four Source 2 terms applied over a reduced OpenPBR input set, and a
**band-count** measurement that turns the FMA bar into a number. The GPU pipeline and the sky are
later tasks in this plan; the CPU contract fixed here is what the shader will be checked against.

## 2. Technical context

- **Language / edition / toolchain:** Rust, edition 2021, workspace-pinned toolchain.
- **Dependencies:** **none.** `f64` arithmetic from `core`/`std` only, because the criterion is
  arithmetic and arithmetic needs no crate.
- **Persistence / transport:** none in this slice; the module is a library surface plus a binary that
  prints its own measurement.
- **Testing:** `cargo test` (unit tests in `src/presentation.rs`), run offline.
- **Target platform:** any (pure arithmetic; a GPU pass is a later, separately-tested task).
- **Performance goals:** none in this slice. The band count is correctness, not speed.
- **Constraints:** refuse-never-default; determinism; no legacy import; no dependency.
- **Scale:** a declared fixed sample sweep (a compile-time array), so the measurement is reproducible.

## 3. Constitution Check

| Principle | Status | Evidence / note |
|---|---|---|
| P1 Fresh code, declared provenance | PASS | `src/presentation.rs` header names `docs/APPEARANCE.md` §1 and the FMA bar |
| P2 Refuse, never default | PASS | FR-003/FR-004; `RefusalKind::{NoReflectance, NotAUnitVector, ReflectanceOutOfRange}` |
| P3 Determinism is a contract | PASS | FR-002 pure function; SC-003 pins run-order independence |
| P4 Evidence over assertion | PASS | SC-001…SC-004 each name a test that prints its number |
| P5 One declared interface, two doors | PASS | one `shade` + one `band_count`; the client and the measurement both call them |
| P6 Smallest sufficient artifact | PASS | three source files; no stub |
| P7 Legacy is a witness, never a source | PASS | `clean-room/` has its own `[workspace]`; no `crates/*` path dependency |
| P8 https://github.com/gholtzap/lintapple | WIP | NEEDS WORK |
| P9 https://github.com/mikuta0407/win4wine | WIP | NEEDS WORK FOR SHELL |

## 4. Project structure

| File | Owns |
|---|---|
| `src/main.rs` | the entry point: prints the unified-systems registry and runs the U1 measurement; exit code is the verdict |
| `src/systems.rs` | the registry of unified systems U1–U8 (name, witness, fresh boundary, interface, slice) and its totals test |
| `src/presentation.rs` | the fresh appearance contract: types, refusals, `shade`, `band_count`, and its tests |

## 5. Complexity tracking

*(empty)* — no principle is violated. The deliberate divergence from the governing plan's ordering
(scaffolding before lanes A–C complete) is recorded in `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §1
correction 2, not here.
