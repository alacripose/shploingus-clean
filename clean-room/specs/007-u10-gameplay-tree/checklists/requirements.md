# Specification Quality Checklist: U10 — The mods-derived gameplay tree

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-10-12
**Feature**: [spec.md](../spec.md)

**Note**: this checklist is the one `/speckit-specify` requires (see
`.github/skills/speckit-specify/SKILL.md` § 8a). It reviews **requirements quality**, not implementation:
`[x]` means the criterion was reviewed and satisfied *for the specification*. The specification-only pass
that wrote it measured nothing, and left the one item that needs a measurement **unchecked with its reason**;
**the slice has since landed** (2026-10-12, thirteenth pass), so that item is now ticked against the numbers
in [quickstart.md](../quickstart.md) — which are the same numbers `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §23
records. What the specification-only pass could not meet is kept in the *Notes* rather than deleted.

## Content Quality

- [x] No implementation details leak into the specification — the spec names entries, dispositions,
      refusals, the row form and measured outcomes; module layout lives in [plan.md](../plan.md)
- [x] Focused on user value and business needs — each user story states the value it carries (a tree whose
      absences are visible; a refusal that is named rather than granted; a first entry that is playable)
- [x] Written for non-technical stakeholders — the acceptance bar is an entry count, a disposition count, a
      refusal name and a served receipt, not a code path
- [x] All mandatory sections completed — User Scenarios & Testing, Requirements, Success Criteria are
      present and filled, and a Sources table names every path each decision came from

## Requirement Completeness

- [x] No `[NEEDS CLARIFICATION]` markers remain — the four things the record does **not** decide are
      recorded as **owed** in [research.md](../research.md) (D7, D8, D9) and in the spec's Assumptions, not
      as unresolved scope questions
- [x] Requirements are testable and unambiguous — FR-001…FR-014 each state a checkable obligation
- [x] Success criteria are measurable — SC-001…SC-007 each name the printed measurement that decides them
- [x] Success criteria are technology-agnostic where they can be — the criteria are counts, dispositions,
      refusal names and one served receipt; the one technology the spec names (`cargo test`/`clippy`) is
      this project's declared gate, quoted because it *is* the acceptance instrument
- [x] All acceptance scenarios are defined — three user stories, each with Given/When/Then scenarios
- [x] Edge cases are identified — fourteen, each with its disposition (refuse by name, report as owed, or
      *owed and named*)
- [x] Scope is clearly bounded — the tree is closed at `docs/MODS-MAP.md`'s reference set; the four things
      this slice does not land are named in [plan.md](../plan.md) § *What this slice does **not** land*
- [x] Dependencies and assumptions identified — the Assumptions section, plus research.md D1–D11

## Feature Readiness

- [x] All functional requirements have clear acceptance criteria — FR-001/FR-002 → SC-001/SC-002,
      FR-005/FR-006 → SC-003, FR-008/FR-009 → SC-004, FR-011/FR-012 → SC-005, FR-014 → SC-006,
      FR-013/FR-012 → SC-007
- [x] User scenarios cover primary flows — US1 (the tree declares every reference and says what each is),
      US2 (a refused reference is refused by name), US3 (the first entry is playable and streams)
- [x] Feature meets measurable outcomes defined in Success Criteria — **ticked by the slice's own pass,
      which ran the commands.** Measured: SC-001/SC-002 by the printed census (`entries=106 mapped=64
      open=25 refused=5 unresolved=12`, summing to 106, beside the record's own 91/97 and 43 figures and the §2 marks (nine)); SC-003 by
      `BigGlobe` refusing `licence_refused` while the permitted head admits; SC-004 by the tree's first entry
      (`MOD-c2me`'s `chunk-infrastructure`, U4, U4's own pasted bar); SC-005 by the served `admit` line over a
      separate process, with the seven axes and the answer digest equal to the one derived in-process;
      SC-006 by the four `U10 OWED:` lines; SC-007 by `cargo test --offline` (226 passed), clippy clean, and
      `cargo run --offline` exit 0 at 444 lines of stdout. **SC-001's equality is met, and it was settled from
      the sources rather than forced** (2026-10-12): the header's 91 counts the file the map was written from,
      this repo's copy holds 97, and the map's own §1–§5 name 106 — the tree's entry count — with the whole
      reconciliation written in the header and held by a test that reads the document in place; the one URL
      with no row anywhere (`mc-ar-mod`) is named owed rather than invented. See `tasks.md` deviation 7
- [x] No implementation details leak into the specification — module names appear only in plan.md's
      Project Structure

## Notes

- **The one decision the record forced, and it is a refusal.** A gameplay-tree entry is a **new row kind**
  in a **closed, versioned** document format (`docs/LEDGER.md` R-64.1), and no reference decides the key or
  the version number. The specification therefore declares the *row* (which U6 already built) and names the
  *document's* new kind **owed**, rather than inventing a `/4` or a key. The same reflex produces the other
  three owed parts: the duplicate-row refusal's name, the whole-tree verb, and the per-row family
  assignment.
- **The disagreement taken smaller, on the record.** This repository has four dispositions
  (`docs/MODS-MAP.md` §1/§3/§4); `shploingus-game`'s plan has six port verdicts
  (`shploingus-game/docs/plan/01-reference-inventory.md` §1). The smaller reading — **four** — is taken, for
  the three reasons in [research.md](../research.md) D2, and the six are cited so the choice is auditable.
- **The §2 marks-versus-file disagreement is corrected, not smoothed** (2026-10-12). `docs/MODS-MAP.md`'s
  §6 states 43 `open` rows (the prior disposition file), and its §2 legend used to state 24 — a figure no
  source reproduces (the marks are nine; the tree derives twelve §2 `open` entries, three of them named in
  the legend for their own reasons). The legend now states the nine and names the three, the census prints
  the marks, and the tree takes each entry's disposition from the row the map gives it.
- **The bar is quoted, not paraphrased** (`docs/CLEAN-ROOM-PLAN-2026-10-11.md` §5 row 10), because a bar
  restated in an author's own words is a bar that has quietly changed.
- **What the specification-only pass did not do**: write code, run a test, cross a pipe, or read a mod
  source. It read documents and wrote documents, and the mod sources are cited **through** the harvest that
  already read them (`docs/MODS-MATERIALS.md`, whose §7 records the sweep as complete).
- **What the slice's pass did, and what it did not**: it built `src/gameplay.rs`, `check_u10`, the registry
  status and six end-to-end tests; it added **no** dependency, read **no** mod source, touched **no**
  `crates/*` file, and coined **no** name for the four parts the record has not decided.
- This checklist reviews the *specification*; the acceptance measurements are the slice's, and are named in
  [quickstart.md](../quickstart.md) without being claimed.
