# Tasks: U7 — The decision layer (axis · laya · von)

**Input**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md) | **Record**: [quickstart.md](./quickstart.md)
and `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §25 · `docs/LEDGER.md` **C-187**.

**All tasks ran and are done** in one pass (2026-10-12, fifteenth pass). Where a task's first assumption was
refuted by a measurement, the correction is recorded under the task rather than smoothed away — that is
[T008](#t008) and [T009](#t009) below, and they are the two most important lines in this file.

- [x] **T001** The axis vocabulary, in a new module: `AxisKind` (seven axes), `LadderValues`, the `Ladder`
  trait, the three ladders this build has read (`EvidenceGrade`, `Grade`, `Origin`), the axes it has **not**
  read printed as owed by name, and the three prohibitions — `refuse_interchange`, `refuse_reduction`,
  `refuse_bare` (`AMBIGUOUS` against `UNQUALIFIED`). *Landed in `src/axis.rs`*, 811 lines.
- [x] **T002** The coverage table and its two guards: `CoverageRow`/`COVERAGE`, the compile-time totality pin
  `COVERAGE_IS_TOTAL`, `check_coverage` comparing each row against the axis's own declared values
  (`fabricated_axis`) and refusing an axis with no row (`uncovered_axis`), and `coverage_summary` for the
  binary's line.
- [x] **T003** The asking, in `src/laya.rs`: `Question::{Choice,Score,Noul}` with their floors, the three
  `Verdict` words, `Ballot::declare` and its refusals (`ballot_incomplete`, `floor_undeclared`),
  `Judgement`, `Measurement`, the `StructuralJudge` trait, `Unreachable`, `Answer`/`Evidence` (which carries
  the reason a `not_belongs` reading needs), and `answer` — the **one** place the four rules are applied.
- [x] **T004** The reading, in `src/von.rs`: the **one** fence reader (`regions`, with the BOM stripped in
  one place), the head-field reader, the sixteen labelled readings, the two feature lists (`Declared` for the
  head's facts, `Feature` for the claim), and `Classifier` — the estimator, the folds, the four figures, the
  tie refusal and `carriers`.
- [x] **T005** The declared-word reader: `VOCABULARY_CLAIMS`, `Family`, `Vocabulary`,
  `claimed_vocabulary`/`claimed_vocabulary_at` — the format's words measured over **every** record in the
  tree, with a contradiction refused by name.
- [x] **T006** The refusals, in `src/refusal.rs`'s closed vocabulary: twelve kinds (`axis_interchange_refused`,
  `scalar_reduction_refused`, `bare_token_refused`, `uncovered_axis`, `fabricated_axis`, `ballot_incomplete`,
  `floor_undeclared`, `oracle_unreachable`, `not_established`, `option_uncalibrated`, `reason_absent`),
  with `ALL`, `COUNT` and `name()` moved together. **Cascaded**: `tests/u5_measure.rs`'s pinned count moved
  **65 → 77**, by name.
- [x] **T007** `src/evidence.rs`'s `check_u7` and `src/main.rs`'s join, so the bar is measured by the same
  function the binary prints and a bar not met exits non-zero; `src/systems.rs` carries U7's real status and
  interface.
- [x] **T008** `tests/u7_decision.rs`: seventeen end-to-end tests through the public surface — the axes and
  their three prohibitions, the coverage guards, the ballot answered with `Origin::Oracle`, the four figures,
  the vocabulary over the corpus, the four fail-closed rules by name, a moved label, and the sixteen stored
  labels against the records' own.
- [x] **T009** Every guard proved **live**: the label moved, a word contradicted, a head fact moved, a record
  removed — each refusing by name in the running binary, each restored **byte-exact** and proved by sha256.
- [x] **T010** The cascade: `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §5 row 6 marked landed, §24 corrected in
  place where the refusal vocabulary's count moved, §25 written; **C-187** filed in `docs/LEDGER.md`;
  `clean-room/specs/009-u7-decision-layer/` created by spec-kit's own `create-new-feature.sh`; the stdout line
  count is **464** lines, **measured directly** — the earlier `459 → 464` phrasing was *derived* from §24's
  457 with no capture, and is corrected in plan §26.

## Findings this pass's own measurements produced

1. **The body-only classifier measured 0.0000**, and that is why the design changed: the sixteen claims are
   the same document (measured feature by feature; the matrix is pinned by a test). The stage question reads
   the record's **head**, and `patches/PATCH-039`'s own sentence — *answerable per record, from ids and heads
   alone* — is the document that says so. **T004's first shape was wrong, and the measurement said so.**
2. **The words were validated outside the calibration set.** 8 declared words, measured over 39 records, with
   **24 readings outside** the sixteen the calibration carries — the evidence that the features are the
   format's own rather than a list fitted to the labels.
3. **A tie is a miss.** With five stages over sixteen readings and folds of four and five, ties are common;
   the tie refusal (`DomainTie`, one layer over) is honest and costs the measurement, and both are reported.
4. **Asking the instrument before reading its calibration** is what keeps `oracle_unreachable` distinct from
   `not_established`: an instrument that cannot run reports no readings at all, so checking its figures first
   would answer every missing oracle with the wrong rule's refusal. The order is written in `answer`'s doc
   comment with that reason.
