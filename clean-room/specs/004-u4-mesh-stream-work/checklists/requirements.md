# Specification Quality Checklist: U4 — Meshing, streaming and work

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-10-11
**Feature**: [spec.md](../spec.md)

**Note**: this checklist is the one `/speckit-specify` requires (see
`.github/skills/speckit-specify/SKILL.md` § 8a). It reviews **requirements quality**, not
implementation: `[x]` means the criterion was reviewed and satisfied *for the specification*. The
measurements that satisfy it are in [quickstart.md](../quickstart.md).

## Content Quality

- [x] No implementation details leak into the specification — the spec names fields, refusals,
      horizons and measured outcomes; module layout lives in [plan.md](../plan.md)
- [x] Focused on user value and business needs — each user story states the value it carries (geometry
      that cannot drift from its world; work that can be accounted for)
- [x] Written for non-technical stakeholders — the acceptance bar is a merge factor, a hit/miss count
      and two refusals, not a code path
- [x] All mandatory sections completed — User Scenarios & Testing, Requirements, Success Criteria are
      present and filled

## Requirement Completeness

- [x] No `[NEEDS CLARIFICATION]` markers remain — the two open questions (persistence; the streamed
      residency policy) are recorded in [research.md](../research.md) as *named open questions*
      belonging to other systems, not as unresolved scope questions
- [x] Requirements are testable and unambiguous — FR-001…FR-012 each state a checkable obligation
- [x] Success criteria are measurable — SC-001…SC-006 each name the printed measurement that decides
      them, and SC-004 requires an **independent recount** rather than the ledger's own formula
- [x] Success criteria are technology-agnostic where they can be — the criteria are digests, counts,
      ratios and refusal names; the one technology the spec names (`cargo test`/`clippy`) is this
      project's declared gate, quoted because it *is* the acceptance instrument
- [x] All acceptance scenarios are defined — three user stories, each with Given/When/Then scenarios
- [x] Edge cases are identified — eight, each with its disposition (refuse by name, or *owed and named*)
- [x] Scope is clearly bounded — the ladder is closed at level 2; the four things this slice does not
      land are named in [plan.md](../plan.md) § "What this slice does **not** land"
- [x] Dependencies and assumptions identified — the Assumptions section, plus research.md D1–D8

## Feature Readiness

- [x] All functional requirements have clear acceptance criteria — FR-002/FR-003 → SC-001, FR-005/FR-006
      → SC-002, FR-001/FR-004 → SC-003, FR-008 → SC-004, FR-009/FR-010 → SC-005, FR-011/FR-012 →
      SC-006 and the boundary check, FR-007 → US2 AC3's anchor scenario
- [x] User scenarios cover primary flows — US1 (geometry derived from the fields), US2 (residency
      coherence and exact invalidation), US3 (work counted, budgeted and distributed)
- [x] Feature meets measurable outcomes defined in Success Criteria — reviewed after the implementation
      pass (2026-10-11): **SC-001…SC-006 are all measured**, with the pasted output in
      [quickstart.md](../quickstart.md). The merge factor is **7.16** (`1024 columns → 144 quads`,
      `1031 cells`); the first derivative is served free on re-ask (`0 more columns, 0 more quads`);
      invalidation removed **1 of 2** kept entries at tick 1 and kept the current one; `level 2` read
      **64 of 1024** produced columns; `geometry_budget_exceeded`, `geometry_out_of_residency` and
      `horizon_mixed` each print their numbers. No pasted line claims a measurement that was not run. **(§39, 2026-10-13):** the mesh's unit moved
      from the column's height to the cells' own voxel faces, so the dated review above moved with it:
      `1024 columns → 3703 faces → 406 quads` (factor `9.12`) over the reading, `11054 cells swept →
      3047 faces → 399 quads` (factor `7.64`) over the cells' own door, and `level 2` → `64 of 1024`
      columns, `3680` faces, `106` quads. **(§40, 2026-10-13):** the extent contract landed — a footprint's
      own columns and the band they imply, with the ring that decides its faces — so the reading prints
      `1156 produced / 1024 samples → 11054 cells → 1289 faces → 313 quads` (factor `4.12`) and the cells'
      own door `2023 faces → 327 quads` (factor `6.19`); over one world the two doors' lists are
      byte-identical (`U4 one world, two doors: … = true`), the audit's `3703`/`3047` is settled by the three
      worlds that decided it (`2679` / `1289` / `2023`), a two-chunk rectangle stitches into one mesh
      (`8013` faces → `1227` quads, its boundaries meshed once), and `level 2` → `64 of 1156`, `1280` faces,
      `68` quads.
- [x] No implementation details leak into the specification — module names appear only in plan.md's
      Project Structure

## Notes

- **One design fault was found by the acceptance check rather than by review**: with the residency
  horizon checked before the resident set, moving U2's anchor turned a cache *hit* into
  `geometry_out_of_residency`, which contradicts US2 AC3 ("a view is not truth"). The check now runs
  after the lookup, and the alternative is recorded in [research.md](../research.md) D5.
- **A palette shorter than the world's class count** is handled by a declared mapping rather than by a
  refusal or an invented material ([research.md](../research.md) D7); the mapping the binary prints is
  `0->1 1->1 2->2 3->3 4->4 5->6`, so the two classes that render alike are on the record.
- This checklist reviews the *specification*; the pasted measurements are the evidence.
- Items marked incomplete would require spec updates before `/speckit-clarify` or `/speckit-plan`;
  none are.
