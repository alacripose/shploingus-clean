# Specification Quality Checklist: U6 — the record and the API layer (the gate)

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-10-11
**Feature**: [spec.md](../spec.md)

**Note**: this checklist is the one `/speckit-specify` requires (see
`.github/skills/speckit-specify/SKILL.md` § 8a). It reviews **requirements quality**, not
implementation: `[x]` means the criterion was reviewed and satisfied *for the specification*. The
measurements that satisfy it are in [quickstart.md](../quickstart.md).

## Content Quality

- [x] No implementation details leak into the specification — the spec names verbs, refusals, claims,
      checks and measured outcomes; module layout lives in [plan.md](../plan.md)
- [x] Focused on user value and business needs — each user story states the value it carries (an answer
      a caller can audit; an identical question with one answer; a refusal that is named rather than
      vague)
- [x] Written for non-technical stakeholders — the acceptance bar is a canonical request line, an answer
      digest, a ledger delta and a refusal name, not a code path
- [x] All mandatory sections completed — User Scenarios & Testing, Requirements, Success Criteria are
      present and filled

## Requirement Completeness

- [x] No `[NEEDS CLARIFICATION]` markers remain — the three open questions (the transport; the content
      gate's own bodies; by-reference requests) are recorded in [research.md](../research.md) as *owed
      capabilities belonging to this system's later slices*, not as unresolved scope questions
- [x] Requirements are testable and unambiguous — FR-001…FR-012 each state a checkable obligation
- [x] Success criteria are measurable — SC-001…SC-006 each name the printed measurement that decides
      them, and SC-004 requires the **same** receipt to be refused against a world that moved rather
      than a freshly built one
- [x] Success criteria are technology-agnostic where they can be — the criteria are digests, counts,
      canonical lines and refusal names; the one technology the spec names (`cargo test`/`clippy`) is this
      project's declared gate, quoted because it *is* the acceptance instrument
- [x] All acceptance scenarios are defined — three user stories, each with Given/When/Then scenarios
- [x] Edge cases are identified — ten, each with its disposition (refuse by name, record, or *owed and
      named*)
- [x] Scope is clearly bounded — the table is closed at five verbs; the three things this slice does not
      land are named in [plan.md](../plan.md) § "What this slice does **not** land" *(the table is closed
      at **eight** as of 2026-10-12 — `admit`, then `save`/`load`; the *bound* this row checks is that
      the slice's own scope was named, which it was) *(and the owed list shrank again the same day,
      §29: the **policy matrix** landed, leaving its own remainder — the self-describing protocol and
      the tool-surface narrowing over a mod-supplied tool list — named in its place)**
- [x] Dependencies and assumptions identified — the Assumptions section, plus research.md D1–D8

## Feature Readiness

- [x] All functional requirements have clear acceptance criteria — FR-001/FR-002 → SC-001 and
      `two_spellings_of_one_request_are_one_canonical_line`, FR-003/FR-010 → the owner-by-owner
      cross-checks (`0e7358e4…` is U4's own digest before §39; the face merge's is `2db62b78…` before §40 and
      `44a682e1…` since, with the geometry answer at `68f985de…`), FR-004
      → SC-002, FR-005/FR-006/FR-009 → SC-004,
      FR-007/FR-008 → SC-003, FR-011/FR-012 → SC-006
- [x] User scenarios cover primary flows — US1 (a request enters and the answer carries its evidence),
      US2 (one request twice, and the record shows what changed), US3 (a request that cannot be honoured
      is refused by name)
- [x] Feature meets measurable outcomes defined in Success Criteria — reviewed after the implementation
      pass (2026-10-11; **§39, 2026-10-13:** the digests quoted below moved with the mesh — the repeat's
      own answer digest is `f6d66a51…` where this review reads `25a485a5…`; **§40, 2026-10-13:** they moved
      again with the extent contract — the repeat's is `cb7e4eef…`, the geometry answer's `68f985de…`, and
      the first call's work `1156` columns): **SC-001…SC-006 are all
      measured**, with the pasted output in
      [quickstart.md](../quickstart.md). Five verbs answer with 3–5 named checks each; the repeat prints
      **one** answer digest (`25a485a5…59324be`) with `1024` then `0` columns and appended-then-not; a
      parse fault leaves the record untouched while a world refusal records **exactly one** event naming
      `geometry_out_of_residency`; the same receipt refuses `claim_unverified` against tick 1 with both
      values (`tick=0` / `tick=1`); and eviction fails `mesh_is_held` alone, which is the *read, not
      recomputed* distinction made measurable. No pasted line claims a measurement that was not run.
- [x] No implementation details leak into the specification — module names appear only in plan.md's
      Project Structure

## Notes

- **The one design decision the acceptance check forced**: the claims are re-checked **inside** `call`
  rather than only by `Receipt::verify` afterwards ([research.md](../research.md) D3). A receipt that is
  handed out and *then* found not to hold would make `claims_hold` a decoration; checking it in `call`
  makes the check load-bearing, and the refusal it produces is recorded like any other fact about the
  world.
- **A `hit` makes no claim about the record's length.** The `event_is` claim exists only where an event
  does, so a cache hit cannot assert that the record grew — asserted in `tests/u6_bridge.rs`.
- **The device the mission asked for is present and checked**: the requests that *could* have been
  answered narrowly — a chunk outside the residency horizon, a level off the ladder, an unmet budget, an
  undeclared field — are each refused by the owner's own name, recorded once, and cache nothing.
- This checklist reviews the *specification*; the pasted measurements are the evidence.
- **The chokepoint review (2026-10-12, §29).** The policy slice added no requirement to this
  specification — the matrix is derived from `docs/HYPERGRILLING.md` §L-5 and `docs/LEDGER.md` R-62.3
  and is *owed* here rather than promised — so the checklist's scope row moved and no criterion changed.
  What it satisfies: an act the caller may not perform is **refused by name** (US3's own shape) and
  **recorded** (SC-003's own rule), now decided at one call site rather than per verb.
- Items marked incomplete would require spec updates before `/speckit-clarify` or `/speckit-plan`; none
  are.
