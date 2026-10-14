<!-- Written by GitHub spec-kit's Copilot integration (`specify init --here`, 2026-10-11) and extended
     by this project. agent-forge was run afterwards (`forge generate`, 2026-10-11) and merged its
     own context files beside this one; see docs/CLEAN-ROOM-PLAN-2026-10-11.md §8.

     **2026-10-13: nothing in this project uses GitHub Copilot.** This file is a vestige of the
     spec-kit integration, and its P7 line was the stricter-than-the-constitution ban that made the
     clean room re-derive ~41 350 lines of the parent's code instead of porting it — the user's
     direction and `patches/PATCH-041-the-reconciliation-with-the-parent-branch.md` record the
     correction. The authority here is `.specify/memory/constitution.md`, read live; this file should
     be deleted rather than maintained. -->

# Clean-room context — read this before editing anything here

This directory is the **clean-room regeneration** of the Shploingus game. It is new code **beside**
the legacy tree at the repository root; it is never a copy of it.

## The documents that are authority, in order

1. [`../.specify/memory/constitution.md`](../.specify/memory/constitution.md) — the project
   principles P1–P7. Read live on every task; it outranks any single spec.
2. [`specs/001-presentation/spec.md`](specs/001-presentation/spec.md) — the first unified system
   (U1 Presentation), its requirements and measurable success criteria.
3. [`specs/003-u3-shapers/spec.md`](specs/003-u3-shapers/spec.md) — the active feature (U3 world
   generation: the shapers), created through the spec-kit workflow; it is **specified, not built**.
4. [`../docs/CLEAN-ROOM-PLAN-2026-10-11.md`](../docs/CLEAN-ROOM-PLAN-2026-10-11.md) — the eight
   unified systems, their fresh-code boundaries, interfaces and ordered slices.

The record (`../docs/LEDGER.md`) is authority over prose; a summary is never authority.

## Hard rules for any change here

- **P1** every module header names the document it was derived from.
- **P2** refuse, never default; collect every refusal rather than the first.
- **P3** the world is a pure function of `(seed, tick, declared period)`.
- **P4** a task is done only when its check was run and its output inspected.
- **P6** no placeholder without a stated purpose; prefer few large files over many stubs.
- **P7** (amended 2026-10-13, constitution v2.0.0) **read** the legacy implementation under
  `../crates/` freely, and **port** the good code into this tree — a port declares its `path`
  dependency and pins the parent bytes it read by `path + digest`. Nothing is transcribed from prose
  in place of a source, and every ported seam lands under this project's convention: measured figures,
  refusals by name, no line that can read false while the run exits 0, guards proven to bite, source
  bytes digested. Nothing is deleted before the port that replaces it passes its gate.

## How work is shaped

spec-kit: the constitution once, then per unified system **spec → plan → tasks**, created from
[`.specify/templates/`](.specify/templates/). Each `plan.md` carries a Constitution Check; each
`tasks.md` ends with a Constitution task that re-runs the checks.
