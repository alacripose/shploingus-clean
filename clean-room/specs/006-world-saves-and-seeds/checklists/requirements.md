# Specification Quality Checklist: U9 — saves and loads, and the world's own seed systems

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-10-12
**Feature**: [spec.md](../spec.md)

**Note**: this is the checklist `/speckit-specify` requires (`.github/skills/speckit-specify/SKILL.md`
§ 8a), in the structure that skill prescribes. It reviews **requirements quality**, not implementation:
`[x]` means the criterion was reviewed and satisfied *for the specification*. It was created by the
specify pass on 2026-10-12, when nothing was implemented and the last row was left `[ ]` on purpose;
**that row was then settled by the implementation pass of the same date (§17 of the plan), which ran the
instruments** — see the last row and the Notes for what was measured. Per Constitution **P4**, a row is
ticked only where a check ran and its output was inspected.

## Content Quality

- [x] No implementation details leak into the specification — the spec names identity, seeds, refusals
      and measured outcomes; module layout and file names belong to `plan.md`, which is deliberately not
      written (the *plan* step, not the *specify* step)
- [x] Focused on user value and business needs — each user story states the value it carries (a world
      that reads back as itself; a save that carries its own seeds so a load is exact; a refusal that is
      named rather than vague)
- [x] Written for non-technical stakeholders — the acceptance bar is a round-trip digest, a per-stream
      seed, an identity refusal name and a record delta, not a code path
- [x] All mandatory sections completed — User Scenarios & Testing, Requirements and Success Criteria are
      present and filled

## Requirement Completeness

- [x] No `[NEEDS CLARIFICATION]` markers remain — the two open items are placed, not unresolved: the
      **document form** of a save (owed, and the only place U6's content gate becomes a hard
      prerequisite) and the **`save`/`load` rows in U6's closed table** (owed, U6's door). Both are
      recorded in the spec's "What this slice waits on" and in `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §16
      — *and both have since **landed** (2026-10-12, §28): the rows are the table's seventh and eighth,
      and the document form reads through the gate. The placing above is what the implementation then
      followed; the checklist row stays ticked because the items were placed rather than left open.*
- [x] Requirements are testable and unambiguous — FR-001…FR-012 each state a checkable obligation
- [x] Success criteria are measurable — SC-001…SC-007 each name the printed measurement that decides
      them, including the worker-count invariance (1/2/4/8) rather than a single run
- [x] Success criteria are technology-agnostic where they can be — the criteria are digests, seeds,
      refusal names and record deltas; the one technology named (`cargo test`/`clippy`) is this project's
      declared gate, quoted because it *is* the acceptance instrument
- [x] All acceptance scenarios are defined — three user stories, each with Given/When/Then scenarios
- [x] Edge cases are identified — eight, each with its disposition (refuse by name, record, or *owed and
      named*), including the two this system is most likely to get wrong: a zero seed is a seed, and a
      save missing a sub-stream the receiver's roster no longer declares
- [x] Scope is clearly bounded — the replay form is this slice; the document form, the `save`/`load`
      tool rows and U5's instruments are named owed, and "What U9 is, and what it is not" states the four
      inherited boundaries
- [x] Dependencies and assumptions identified — the Assumptions section plus the dedicated
      "What this slice waits on — and what it does not", which answers the hard-versus-adjacent question
      for U6's content gate, U4's residency and U2's owed T016 explicitly

## Feature Readiness

- [x] All functional requirements have clear acceptance criteria — FR-001/FR-002 → SC-001 and SC-006,
      FR-003/FR-006 → SC-004, FR-004 → SC-002, FR-005 → SC-003, FR-008 → SC-005, FR-009 → SC-006,
      FR-010/FR-011/FR-012 → SC-007
- [x] User scenarios cover primary flows — US1 (a world reads back as the same world), US2 (a save
      carries its own seed systems), US3 (a save that cannot be trusted is refused by name)
- [x] Feature meets measurable outcomes defined in Success Criteria — **measured 2026-10-12** by the
      implementation pass, through the real surface (a file on disk and a loaded world, `src/save.rs`
      and `tests/u9_saves.rs`), with the printed lines pasted in [quickstart.md](../quickstart.md):
      SC-001 the loaded world's `truth_digest` equals the live world's (`32b3c1f8…`) and the previous
      tick differs; SC-002 **five distinct** per-domain stream seeds, carried and checked in both
      directions; SC-003 the loaded record is byte-identical and verifies, and `record_diverged` is
      reachable from a coherent save that declares another record; SC-004 a flipped byte is refused with
      both digests, a 100-byte truncation names the extent, and a non-save is refused by name;
      SC-005 the same generation digest at 1/2/4/8 workers (`c331cf55…`) and byte-identical save files;
      SC-006 one read, no re-derivation (the truth digest is derived on load, never read from the file);
      SC-007 **183 tests pass, 0 fail**; `cargo clippy --offline --all-targets -- -D warnings` and
      `cargo run --offline` both exit 0. **Earned SC-004 case — `seed_undeclared` and `record_diverged`
      are not reachable through the binary's writer**, which refuses to emit a save it would refuse on
      load; they are pinned in `src/save.rs` and printed by name so the six names are not silently two
- [x] No implementation details leak into the specification — the entities named are the system's own
      vocabulary (a save, its identity, its seed declarations, its refusals); the module names appear
      nowhere

## Notes

- **The last row was left unchecked by the specify pass on purpose, and was ticked only by the pass
  that ran the instruments.** Every other row is about the *specification* and can be settled by reading
  it; "meets measurable outcomes" can only be settled by running the instrument, and the pass that wrote
  the spec was told not to implement it. It is ticked now because the checks ran and their output was
  pasted, not because the criteria were written down — and where a named refusal is *not* reachable from
  the binary's writer, the row says which two and where they are pinned instead of claiming six.
- **The one design decision the record forced into the spec**: the seed systems are *declared per
  sub-stream* and carried, rather than re-derived from a constant (`C-96.1`'s defect, at one remove).
  A save that omits one is refused by name, so a roster that shrinks cannot silently drop a stream's
  identity.
- **The prerequisite question is answered in the spec, not deferred**: U6's content gate is **adjacent**
  to this slice — hard only for the owed *document* form of a save; U4's residency is not involved; U2's
  T016 is a prerequisite of **U12**, not of this system.
- **What the plan owed this system, and no longer owes**: `plan.md`, `tasks.md`, `research.md`,
  `data-model.md`, `contracts/u9-seams.md` and `quickstart.md` now exist, made by `/speckit-plan` and
  `/speckit-tasks` as specs 003–005 were made. One decision changed during that step (the per-domain seed
  is a new declared number rather than a re-rooting of `stage_seed`, because re-rooting would have moved
  the published U3/U4 digests — research **D2**).
- **Still owed, and named rather than half-built**: **a save of a resident set**, **compression**, and a
  **versioned migration path**. The first two items this line used to name — the **document form** of a
  save (`path + digest`, the one place U6's content gate is a hard prerequisite) and **`save`/`load` rows
  in U6's one tool table** — **landed on 2026-10-12 (§28)**: the rows are `bridge::TOOLS`' seventh and
  eighth, and `Save::read_document` reads a document only through `Intake::read`. The list above is what
  remains, so the binary's own `U9 owed:` line is the account of it.
- This checklist reviews the *specification*. The spec's `Created` line and this file's are the same
  date; the spec was created through spec-kit's own script
  (`specs/006-world-saves-and-seeds`, `FEATURE_NUM` 006, `.specify/feature.json` pointed at it).
