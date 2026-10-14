# Requirements checklist: U7 — The decision layer (axis · laya · von)

Read against `spec.md`'s FR-001…FR-011 and this pass's measurements (`quickstart.md`). Every box was checked
against a number in the binary's own output or a test that holds it, not against an intention.

## The axes

- [x] **FR-001** `AxisKind` declares seven axes; three ladders are read and four are printed **owed by name**
  (`certification`, `org_rank`, `gate_status`, `evidence_level`) — the binary's own line, and
  `axis::tests::the_owed_ladders_are_named_and_never_invented`.
- [x] **FR-002** Interchange, reduction and bare tokens refused by name; `A` is `AMBIGUOUS` across three
  declared ladders, `SSS` is `UNQUALIFIED`, and a token no ladder claims is not this rule's business —
  `tests/u7_decision.rs::a_bare_ladder_token_is_refused_by_its_own_name`; 14 reductions refused (7 × 2).
- [x] **FR-003** `COVERAGE` describes each axis once; `COVERAGE_IS_TOTAL` pins the length at compile time;
  `check_coverage` refuses `fabricated_axis` and `uncovered_axis` —
  `the_coverage_table_describes_every_axis_exactly_once_and_refuses_a_row_that_is_untrue`.

## The asking

- [x] **FR-004** `Question::{Choice,Score,Noul}`, the three `Verdict` words, `Ballot` with its ten fields;
  empty fields, fewer than two terms, `max-age` of zero, a floor outside its terms and a duplicate ballot id
  each refused by name — `laya::tests`.
- [x] **FR-005** Every floor is declared inside the ballot, before any answer exists; a `noul` threshold
  outside `(0, 1]` is `floor_undeclared`.
- [x] **FR-006** `answer` applies the four rules in one place, each with its own name, and the order is
  written down with its reason (asking first keeps *unreachable* distinct from *uncalibrated*).
- [x] **FR-007** The origin is set from the seam and no ballot field can set it; a non-reading carries
  `Verdict::NoReading` and `passed()` refuses to read a floor off it.

## The reading

- [x] **FR-008** `KNOWN_ANSWERS` is a `const` of sixteen labelled readings; a label outside the options is
  `option_uncalibrated`, an option with no reading is the same refusal, and a stored label that no longer
  matches its record is `claim_unverified` — **stale rather than wrong**.
- [x] **FR-009** One fence reader (`regions`), one head-field reader; the label's key is read by no feature,
  and a moved label cannot move the reading — `a_moved_label_cannot_move_the_reading`.
- [x] **FR-010** Four figures printed on one line (in sample 0.7500, leave-one-out 0.5000, absence 0.4375,
  claim only 0.0000) against the majority class 0.3125, read strictly; the set's digest travels with them.
- [x] **FR-011** The eight declared words are read over 39 records in place, with 24 readings outside the
  calibration set and no contradiction — `claimed_vocabulary` and the binary's own vocabulary line.

## The standing rules

- [x] **P1** Every module header names the document it was derived from, and the record's own words are quoted
  rather than paraphrased (`measure.rs`'s precedent, followed here in `axis`/`laya`/`von`).
- [x] **P2** The refusal vocabulary is closed and counted (**77**, up from 65, pinned in
  `tests/u5_measure.rs`); the coverage table's totality is a compile-time pin.
- [x] **P3** One owner per concern: one fence reader, one head-field reader, one ballot grammar, one place the
  four rules are applied, one place a column becomes a name.
- [x] **P7** No `use` of, and no `path` to, any `crates/*` item; no dependency added (`sha2` is still the
  only one); no legacy file modified.

## The suite

- [x] **Gates** `cargo test --offline` **323 passed / 0 failed** (263 library, 60 across nine suites);
  `cargo clippy --offline --all-targets -- -D warnings` exit 0; `cargo run --offline` exit 0 at **464** lines
  of stdout.
- [x] **Guards proved live** the label moved, a declared word contradicted, a head fact moved, a labelled
  record removed — each refusing by name in the running binary, each restored **byte-exact** by sha256.

## Owed, and not ticked

- [ ] The **score question's rungs**: no instrument in this tree reads rungs, so a score question is a
  non-reading (`no_reading`) rather than a verdict.
- [ ] The **`noul` and `score` calibration sets**: only the stage question has labelled examples here, so the
  three-part question is exercised but not calibrated, and its answers are not quotable under R-59.5.
- [ ] A **model-backed instrument** behind `StructuralJudge` (the legacy ONNX session's slot).
- [ ] **U6's remainder**, the **four U10 rulings** and **`mc-ar-mod`** — untouched, and decidable only by the
  record (`docs/CLEAN-ROOM-PLAN-2026-10-11.md` §16.3, §23, §24).
