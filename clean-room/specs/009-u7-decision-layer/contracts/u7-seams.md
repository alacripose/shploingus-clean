# U7's seams — what each surface promises, and what is deliberately not on it

Derived from the record (`docs/CLEAN-ROOM-PLAN-2026-10-11.md` §2 U7 and §5 row 6,
`patches/PATCH-023` §1–§3, `docs/LEDGER.md` R-59.5 · R-59.7 · C-102 §102.2). **Four seams**, one owner each.
A seam here is a surface another module is allowed to depend on; anything not listed is private, and a caller
that needs it is a caller whose need is the record's to decide.

## C1. `axis` — the vocabulary of evaluation, and its three prohibitions

* **Promises:** `AxisKind` (seven, with `ALL`/`COUNT`/`name()`/`parse`), `LadderValues`,
  `AxisKind::declared_values` as an **exhaustive match**; the `Ladder` trait with the three ladders this build
  has read (`EvidenceGrade`, `Grade`, `Origin`, the last carrying its own `TOKENS`); `refuse_interchange`,
  `refuse_reduction`, `refuse_bare`, `bare_token_axes`; `Reduction`; `CoverageRow`/`COVERAGE` with the
  compile-time totality pin `COVERAGE_IS_TOTAL` and the runtime guard `check_coverage`;
  `coverage_summary`.
* **Refuses by name:** `axis_interchange_refused`, `scalar_reduction_refused`, `bare_token_refused`
  (`AMBIGUOUS` where two or more declared ladders claim the token, `UNQUALIFIED` where exactly one does),
  `uncovered_axis`, `fabricated_axis`.
* **Not on it:** no arithmetic between axes, no ordering across ladders, no percentage, no score, no
  serialisation of an axis into a number. A caller who wants a number is asking for a reduction this crate
  refuses on purpose.
* **A consumer may rely on:** a ladder's values are the axis's own (`check_coverage` compares the row against
  the axis, never against itself), and an axis with no row is a **failure**, not a todo.

## C2. `laya` — the asking, the three words, and the four rules

* **Promises:** `Question::{Choice,Score,Noul}` (`choice(options, passes)`, `score(levels, floor)`,
  `noul(threshold)`, with `kind`, `passes`, `terms`), `Verdict` (three words, `ALL`/`name`/`parse`/
  `is_a_verdict`), `Ballot::declare` + its accessors, `Judgement`, `Measurement` (`beats_majority`,
  `quotable`, `render`), the `StructuralJudge` trait (`judge`, `measurement`), `Unreachable`, `Answer`
  (`verdict`, `origin`, `evidence`, `passed`), `Evidence`, and `answer`.
* **The floor is declared before the answer exists**: a choice names its passing option, a score its lowest
  passing rung, a `noul` its threshold (0.5 by default — a probability of yes has a natural midpoint).
* **Refuses by name:** `ballot_incomplete`, `floor_undeclared`, `oracle_unreachable`, `not_established`,
  `reason_absent`, `source_absent`, and `no_reading` (a shape the question does not apply to, or an artifact
  with nothing to read).
* **The three rules the legacy stated and this crate keeps:** the origin is the proof's to carry and **no
  ballot field can set it**; a **shape mismatch is a non-reading**, never a verdict for the other instrument;
  a **missing answer is a non-reading**, because the alternative is a missing answer read as a pass.
* **The order of the four rules is declared and reasoned** in `answer`'s doc comment: bytes read → the
  instrument asked (its own refusal returned as it comes) → its calibration read → the negative reading's
  reason. Asking first is what keeps *unreachable* distinct from *uncalibrated*.
* **Not on it:** no question type beyond the three, no answer that carries prose in place of a word, no
  `Measurement` partial — a caller cannot construct a quotable figure without filling every field, including
  the two losing ones.
* **A consumer may rely on:** `Answer::passed()` returns `Err(no_reading)` for a non-reading rather than
  `Ok(false)`, and a `not_belongs` answer has already been refused unless a reason was recorded **in the same
  act**.

## C3. `von` — the reading

* **Promises:** `regions`/`head_of`/`body_of`/`head_field` (the **one** fence reader, the BOM stripped once),
  `declared_status` (the harness's reader, for the staleness check only), `Declared` (the head's twelve
  facts, `ALL`/`name`/`describes`/`holds`) with `declared_facts`, `Feature` (the claim's fourteen features,
  same shape) with `features_of`, `STAGES`, `KNOWN_ANSWERS`, `Classifier` (`calibrate`, `classified`,
  `options`, `figures`, `readings`, `labels_describe_the_records`, `declared_labels`, `carriers`),
  `claimed_vocabulary`/`claimed_vocabulary_at` with `Vocabulary`/`Family`, and the two column counts
  (`HEAD_COLUMNS`, `CLAIM_COLUMNS` — private, because the space they describe is private).
* **Promises about the label:** the label's own key is read by **no feature**; the head's facts cannot see
  `status`, and the claim's features cannot see the head at all. A moved label cannot move a reading, a score
  or a feature list — the test says so by moving one.
* **Refuses by name:** `source_absent` (a reading that is not there), `option_uncalibrated` (a label outside
  the question's options, or an option no reading calibrates), `no_reading` (a document that is not a record, a
  record with no claim, a head that declares none of the facts, a tie), `claim_unverified` (a stored label
  that no longer matches the record — **stale rather than wrong**; a record contradicting a declared word).
* **Not on it:** no model, no file format of its own, no second parser. The classifier reads the record's head
  through `head_field` and its claim through `body_of`, and nothing else in the crate may parse either.
* **A consumer may rely on:** `figures()` reports **four** accuracies (in sample, leave-one-out, the absence
  variant, the claim-only reading) and the majority class beside them, and `beats_majority` is read strictly —
  a judge that always answers the commonest option ties the bar and does not clear it.

## C4. The seams U7 does **not** touch

U6's table and its transport, U5's instruments and light, U9's saves, U10's tree, and the four U10 rulings
plus `mc-ar-mod` — all unchanged, all named as owed where they were already named. U7 adds **no** tool to
U6's closed table: the classifier is asked in-process by the gate, and a served verb for it is a decision for
the record.

## What a consumer may rely on, in one sentence each

1. A bare token, a cross-ladder comparison or a scalar reduction is refused **by name**, and the refusal says
   which axis and which rule.
2. Every axis is described exactly once, and a row that is well formed and untrue is caught.
3. An answer carries its origin, what it read, the calibration's own figures and the artifact's digest — from
   which the reading can be re-derived without the instrument.
4. A floor is declared before the answer exists, and a non-reading cannot be read as a pass.
5. A calibration that does not beat its own majority class refuses, and the measurement is reported either
   way.
