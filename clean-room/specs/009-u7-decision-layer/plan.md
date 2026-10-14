# Implementation Plan: U7 — The decision layer (axis · laya · von)

> The plan's own order (`docs/CLEAN-ROOM-PLAN-2026-10-11.md` §5 row 6, taken after U5's row 5). The record of
> the pass — the numbers, the moved lines and what is owed — is §25 of that plan; this file is the working
> plan and the constitution check.

## Landed (2026-10-12, fifteenth pass)

**Created**: `src/axis.rs` (811 lines), `src/laya.rs` (958), `src/von.rs` (1655), `tests/u7_decision.rs` (445) — re-measured on the final bytes after the audit's corrections (`docs/CLEAN-ROOM-PLAN-2026-10-11.md` §26).
**Changed**: `src/refusal.rs` (U7's own kinds, the closed vocabulary **65 → 77**), `src/lib.rs` (three
modules), `src/evidence.rs` (`check_u7`, the gate's own lines), `src/main.rs` (the bar joins the gate),
`src/systems.rs` (U7's real status), `tests/u5_measure.rs` (its pinned count moved 65 → 77, by name).

**Nothing under `crates/` was read as code** (P7), no dependency was added (`sha2` remains the only one), and
the witness crates (`voxel-model/src/{axis,laya}.rs`, `voxel-package/src/{oracle,ballot}.rs`, `voxel-von`) were
read for **behaviour only** — the published vocabulary and the measured numbers — with no line copied.

## The bar, and how it is read

* The record's bar (R-59.5, `docs/HYPERGRILLING.md` H-14.1): *the calibration bar is the majority class of the
  known-answer set this project actually ships*, read **strictly**, and *the measurement is reported either
  way*.
* **Measured this pass**: 16 readings, five stages, majority class **0.3125** (5 of 16); in sample **0.7500**;
  leave-one-out **0.5000**; leave-one-out with absence **0.4375**; **leave-one-out from the claim only
  0.0000**; margin **+0.1875**. The bar is met, and the reading that loses is printed beside it.

## The one correction this slice records rather than absorbs

**The first draft read the record's *body*.** It was the honest-looking choice — measure the claim, not the
head — and the measurement refused it: the body-only classifier scores **0.0000** leave-one-out against the
same majority class, because the sixteen claims are structurally the same document (measured: every one
declares the same four sections and quotes the record's own vocabulary; only their length varies, and the
matrix is pinned by a test). The stage is not in the claim. It is in the record's own **life facts**, exactly
as `patches/PATCH-039` says of this corpus — *"the same id at five different stages of its life, across five
trees … answerable per record, from ids and heads alone"* — so the stage question now reads the head, the
claim-only figure is printed as the finding, and the second question (`noul`, *a decision or a description?*)
still reads the claim's three parts.

**What was not done instead**: the known-answer set was not touched, no label was moved, and no feature was
chosen because it made a number bigger. The declared facts are the format's own keys (the corpus of 39 records
is where the format is written down), and the word→stage claims the features rest on are measured over **all
39**, of which **24 readings lie outside** the sixteen the calibration carries.

## Constitution check (P1–P7)

* **P1 documents before code** — every module's header names the document it was derived from and where each
  constant's source is; `docs/LEDGER.md`'s own words are quoted rather than paraphrased. ✓
* **P2 pins, not prose** — the refusal vocabulary is closed and counted (77), the coverage table is total at
  compile time (`COVERAGE_IS_TOTAL`), and the calibration's own figures are `Measurement` fields rather than
  sentences. ✓
* **P3 one owner per concern** — one fence reader (`regions`), one head-field reader, one ballot grammar, one
  place the four rules are applied (`laya::answer`), one place a column becomes a name. ✓
* **P4 the smallest reading that fits the documents** — the `noul` threshold defaults to the natural midpoint
  (0.5) as the ballot's own text says; the stage question reads the head because the corpus says so. ✓
* **P5 measurement where a number is claimed** — the four figures, the corpus count, the words and the
  outside count are all measured and printed. ✓
* **P6 compute the horizon, do not copy it** — the axis coverage table compares each row against the axis's own
  declared values; the vocabulary check compares each record against the word's claim. ✓
* **P7 new code beside the old tree** — no `use` of, or `path` to, any `crates/*` item; `grep` finds the word
  only in documents. ✓

## Structure decisions

1. **`axis.rs` owns the vocabulary of evaluation; `laya.rs` owns the asking; `von.rs` owns the reading.** Three
   concerns, three owners, and the binary is a consumer of all three.
2. **The classifier is a structural reading, not a model.** It is a Laplace-smoothed naive Bayes over a
   *declared* feature list, whose fit is the rows themselves — which is what makes leave-one-out honest: a fold
   removes one reading's row and recounts, with no fitted parameter that could have seen the answer.
3. **One feature space, two halves.** The head's declared facts occupy columns `0..12`, the claim's features
   `12..26`, so the same estimator can be asked both readings and the two figures are comparable line for line.
4. **A tie is not an answer.** `reading` refuses two options supported equally (`DomainTie`, one layer over)
   rather than picking by order, and a tie counts as a miss in the measurement.
5. **`Unreachable` is a shipped judge.** *A missing instrument is not a pass* is a value in the crate, not a
   sentence in a comment, so the second rule has a witness the gate can drive.

## What this slice does not land, and names

* **The four U10 rulings and `mc-ar-mod`** stand owed and untouched (`docs/CLEAN-ROOM-PLAN-2026-10-11.md` §16.3,
  §23, §24) — put to the user and not ruled on, so this pass decides none of them.
* **U6's remainder** is owed and untouched: the document record and its compression, the permission policy
  matrix, the API document, a pin over a request line, the crate × axis census, `save`/`load` as rows.
* **The score question's rungs** are owed by name: `Question::Score` parses and refuses a floor outside its
  rungs, but no instrument in this tree reads rungs, so a score question is a non-reading (`no_reading`) rather
  than a verdict — which is the declared behaviour rather than a hole.
* **A model-backed instrument** is not built: `Unreachable { what: … }` is the seam's own witness, and a caller
  who wants the legacy ONNX classifier behind `StructuralJudge` has the trait to implement.
* **The `score`/`noul` calibration sets** are owed: only the stage question has a labelled set in this tree,
  so a `noul` question is **refused by name** (`not_established`) rather than answered on the stage
  question's figures.
* **`docs/LEDGER.md` is untouched**: the clean room writes beside the legacy record and not into it (P7), and
  this pass added no ledger entry.
