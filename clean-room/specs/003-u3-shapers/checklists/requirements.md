# Specification Quality Checklist: U3 — World generation (the shapers)

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-10-11
**Feature**: [spec.md](../spec.md)

**Note**: this checklist is the one `/speckit-specify` requires (see
`.github/skills/speckit-specify/SKILL.md` § 8a). It reviews **requirements quality**, not
implementation: `[x]` means the criterion was reviewed and satisfied *for the specification*.

## Content Quality

- [x] No implementation details leak into the specification — the spec names laws, papers, refusal
      names and measured outcomes; module layout lives in [plan.md](../plan.md)
- [x] Focused on user value and business needs — each user story states the value it carries (a world
      that can be rebuilt and audited; a roster whose silence is impossible)
- [x] Written for non-technical stakeholders — the laws are named by paper, and the acceptance bar is
      a count (`14 of 16`), not a code path
- [x] All mandatory sections completed — User Scenarios & Testing, Requirements, Success Criteria are
      present and filled

## Requirement Completeness

- [x] No `[NEEDS CLARIFICATION]` markers remain — the two genuinely open questions (terrain-term
      wiring order; the conservation tolerance) are recorded in [research.md](../research.md) as
      *named open questions* for the implement pass, not as unresolved scope questions
- [x] Requirements are testable and unambiguous — FR-001…FR-010 each state a checkable obligation
- [x] Success criteria are measurable — SC-001…SC-007 each name the printed measurement that decides
      them
- [x] Success criteria are technology-agnostic where they can be — the criteria are digests, counts
      and refusals; the one technology the spec names (`cargo test`/`clippy`) is this project's
      declared gate, quoted because it *is* the acceptance instrument
- [x] All acceptance scenarios are defined — three user stories, each with Given/When/Then scenarios
- [x] Edge cases are identified — eight, each with its disposition (refuse by name, or `unchecked`)
- [x] Scope is clearly bounded — the roster is closed at fifteen; the two deliberate refusals are
      named; U2's nouns are consumed, not redefined
- [x] Dependencies and assumptions identified — the Assumptions section, plus research.md D1–D6

## Feature Readiness

- [x] All functional requirements have clear acceptance criteria — FR-003 → SC-004, FR-004 → SC-003,
      FR-005 → SC-006, FR-006 → SC-003/SC-005, FR-007 → SC-004's field clause, FR-008 → SC-005,
      FR-001 → SC-001/SC-002, FR-009 → the boundary check, FR-002 → the roster check
- [x] User scenarios cover primary flows — US1 (determinism), US2 (every runnable stage runs and
      nothing silently does nothing), US3 (the declared graph is inspectable)
- [x] Feature meets measurable outcomes defined in Success Criteria — reviewed again after the
      implementation pass (2026-10-11): **SC-001, SC-002, SC-003, SC-004's digest clause, SC-005,
      SC-006 and SC-007 are measured** (digests, refusal names, the law readings and the counts are
      pasted in [quickstart.md](../quickstart.md)). SC-003's tally is **13 of 15 stages ran, 2 refused
      by name**, both refusals being decisions (`caves`: the dissolution field is not declared;
      `earthworks`: the record, not a generator) — the legacy's `14 of 16` read against this
      fifteen-stage roster. No pasted line claims a measurement that was not run.
- [x] No implementation details leak into the specification — module names appear only in plan.md's
      Project Structure

## Notes

- **At the date of writing (2026-10-11, first pass) every success criterion was `unchecked`**, and
  the spec said so. The second pass of the same day built U3's first slice, and the third closed the
  laws that slice left owed inside landed stages (Lieth's capacity, Reynolds' flocking, Strahler's
  order, lake fill, the hillslope consumer) plus the two record bodies. Every criterion now carries a
  pasted measurement in [quickstart.md](../quickstart.md).
- This checklist reviews the *specification*; the pasted measurements are the evidence.
- Items marked incomplete would require spec updates before `/speckit-clarify` or `/speckit-plan`;
  none are.
