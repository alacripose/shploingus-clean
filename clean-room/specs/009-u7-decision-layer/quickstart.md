# Quickstart: U7 — how to see it run, and what it measured

## Run it

```bash
cd clean-room
cargo test --offline                                  # 323 passed / 0 failed (263 lib, 60 across nine suites)
cargo clippy --offline --all-targets -- -D warnings  # exit 0, no diagnostics
cargo run --offline                                   # exit 0, 464 lines of stdout — U7's seven are 457..463
```

## The bar, quoted

> `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §5 row 6, U7's own acceptance check: **a ballot answered with
> `Origin::Oracle`; the calibration accuracy beats the majority class.**

and the standing honesty rule that governs the second half (`docs/LEDGER.md` R-59.5): *if the structural judge
does not beat the majority class of the known-answer set this project actually ships, then the calibration
refuses, the lint gate fails closed, and the build gate prints `model-oracle not-established`. The known-answer
set may **not** be edited to manufacture a pass. (…) The measurement is reported either way.*

## Scenario 1 — the axes, and the three ways reading one is a bug (SC-004)

```
U7 axes: 7 ladders — 3 declared and 4 owed by name (certification, org_rank, gate_status, evidence_level) —
         the coverage table describes every axis exactly once = true; a forged row refuses `fabricated_axis`
         = true, an axis with no row refuses `uncovered_axis` = true
U7 prohibitions: a cross-ladder comparison refuses `axis_interchange_refused` = true (two members of one
         ladder compare freely = true); 14 reductions refused (7 axes × 2); a bare `A` is AMBIGUOUS across 3
         declared ladders = true, a bare `SSS` is UNQUALIFIED = true, and a token no declared ladder claims
         is not this rule's business = true
```

The four owed ladders are **named, not invented**: an axis with no values read is a debt, and the coverage
guard refuses a table that leaves one out.

## Scenario 2 — the words, read over the whole corpus (SC-005)

```
U7 vocabulary: 8 declared word(s) read over 39 record(s) in place — strengthen×4(+1 outside),
         replace×1(+0 outside), design-approved×8(+0 outside), applied-to-internal-base×5(+0 outside),
         user-confirmed×2(+0 outside), user-selected×1(+0 outside), user-directed×23(+23 outside),
         failed-eval×4(+0 outside); every word's claim held = true, and 24 of the readings are records the
         calibration was **not** taken over
```

`(+N outside)` is the count of records carrying the word that lie **outside** the sixteen the calibration
carries. A claim that only ever held over the calibration set would be a fitted claim; **24** readings —
`user-directed`'s 23 and `strengthen`'s 1 — say those two words' meanings were measured on records the
classifier was not calibrated on, and every other word's claim held with none contradicted.

## Scenario 3 — the calibration, both ways round (SC-002, SC-003)

```
U7 calibration: 16 reading(s), proposed 5 / designed 4 / verified-internal 4 / applied-internal 1 / approved 2
         — accuracy 0.7500 in sample, 0.5000 leave-one-out, 0.4375 leave-one-out with absence, 0.0000
         leave-one-out from the claim only, majority class 0.3125, margin +0.1875 — the bar is the majority
         class, read strictly (leave-one-out 0.5000 > majority 0.3125) = true; the stored labels still
         describe the records = true; the set's own digest d803c643
```

Four figures and the set's digest, all on one line: the number that passes, the estimator that does not
(absence), **the half of the record that carries nothing** (claim only, 0.0000), and the bar. The digest makes
a moved reading a **stale measurement** rather than a silent one.

## Scenario 4 — a ballot answered, and the mechanism named (SC-001)

```
U7 ballot: `u7-life-of-PATCH-015` over `patches/PATCH-015-hypergrilling.md` → `belongs`, origin A (Oracle) =
         true; the reading is `approved`; 3 declared feature(s) held; 5 option(s) scored; the evidence is
         21aa66cd (read in place)
U7 reading: the declared feature(s) that carry a stage are operation_is_add→proposed +0.13,
         operation_is_strengthen→applied-internal +0.44, operation_is_replace→designed +0.22,
         declares_no_dependency→verified-internal +0.39, approval_is_design_approved→proposed +0.36,
         approval_is_applied_to_internal_base→verified-internal +0.50, approval_is_user_selected→designed
         +0.22, approval_is_user_confirmed→approved +0.58, source_is_a_user_instruction→proposed +0.13,
         source_is_a_failed_evaluation→verified-internal +0.39, states_what_is_owed→approved +0.17,
         names_a_path→designed +0.22, reports_a_figure→designed +0.17, long_body→approved +0.58,
         short_body→applied-internal +0.44 — the figure is a reading of 16 record(s), so the mechanism is
         named rather than implied
```

The first three facts are the artifact's own head; the reading is the option `PATCH-015` declares for itself;
the origin is `A` because a model's answer is **evidence, not authority** and no ballot field can set it.

## Scenario 5 — the four rules, each by name (SC-004)

```
U7 fail-closed: an uncalibrated instrument writes nothing [`not_established`] = true; a missing instrument is
         refused [`oracle_unreachable`] = true; a non-reading is never a pass [`no_reading`] = true; a
         `not_belongs` reading needs a human's reason in the same act [`reason_absent`] = true, and with one
         it is filed carrying the reason = true
```

Each was then proved **live**, by perturbing the tree and restoring it byte-exact:

| Perturbation | What refused | sha256 restored |
|---|---|---|
| `PATCH-013` `status: designed` → `approved` | `claim_unverified` — *stale rather than wrong*; binary exit 1 | `1d403eea…1846be` |
| `PATCH-023` `status: applied` → `proposed` | `claim_unverified` naming `user-directed`; binary exit 1 | `7dc2b8fb…b76b76d` |
| `PATCH-015` `approval: user-confirmed` → `design-approved` | the reading moves, so the floor is missed: `reason_absent`; binary exit 1 | `21aa66cd…83ee2c` |
| `PATCH-016` moved out of the tree | `source_absent`, naming the path; binary exit 1 | `bcb935b8…0d8c60` |

## What moved, and by how much

| Thing | Before | After |
|---|---|---|
| stdout lines | (no 459-line capture exists — the figure was *derived* from §24's 457, and is corrected in plan §26) | **464**, measured directly; the correction pass added and removed none |
| closed refusal vocabulary | 65 | **77** (twelve U7 kinds; pinned in `tests/u5_measure.rs`, which moved with it) |
| tests | 260 in a green suite, plus U7's first-draft checks | **323 passed / 0 failed** (263 lib + 60 integration; U7's own 63) |
| new source | — | `src/axis.rs` 811 · `src/laya.rs` 958 · `src/von.rs` 1655 · `tests/u7_decision.rs` 445 (re-measured on the final bytes) |
| dependencies | `sha2` | `sha2` (unchanged — no model, no async runtime, no CLI) |
| legacy files touched | 0 | **0** (`grep` finds `crates/` only in documents) |

## What is owed, named

* The **score question's rungs**: `Question::Score` parses and refuses a floor outside its rungs, but no
  instrument here reads rungs, so a score question is a **non-reading** rather than a verdict.
* The **`noul` and `score` calibration sets**: only the stage question has labelled examples in this tree, so
  the three-part question has **no set to be calibrated against**. It is therefore **refused by name** —
  `not_established`, at the instrument — rather than answered on the *stage* question's figures (R-59.5: an
  uncalibrated instrument writes nothing). `tests/u7_decision.rs` proves the refusal end-to-end.
* A **model-backed instrument**: `Unreachable { what: … }` is the shipped witness that a missing instrument is
  refused; the legacy ONNX session is one implementation of `StructuralJudge` behind the trait.
* **U6's remainder** (the document record and its compression, the policy matrix, the API document, a pin over
  a request line, the crate × axis census) — untouched.
* **The four U10 rulings and `mc-ar-mod`** — put to the user, not ruled on, and **not** decided here.
