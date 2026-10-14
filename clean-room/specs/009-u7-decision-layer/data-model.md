# Data model: U7 — The decision layer

Two halves, and the split is the design: the **asking** (`laya`, with `axis`'s vocabulary behind it) and the
**reading** (`von`). Everything below is what a caller may depend on; the shapes are the crate's, and the
numbers are this pass's measurements.

## 1. The axis vocabulary (`axis`)

| Shape | What it is | Declared facts |
|---|---|---|
| `AxisKind` | one of the **seven** axes | `ALL`, `COUNT` = 7, `name`, `parse`, `declared_values` (an exhaustive match), `is_declared`, `is_a_quantity` |
| `LadderValues` | the values an axis declares | the four-word/three-word ladders, closed |
| `Ladder` | a token's own type | `EvidenceGrade` (A–D), `Grade` (nine words), `Origin` (A/B/C with `TOKENS`) |
| `Reduction` | the two ways a caller tries to make a number of an axis | `Percentage`, `Score` |
| `CoverageRow` | one axis's representation, with its source | `axis`, `ladder`, `values`, `source` |

* **3 of 7** axes are **declared** (their ladders are read); **4** are **owed by name** — `certification`,
  `org_rank`, `gate_status`, `evidence_level`.
* The coverage table is `[CoverageRow; 7]` with the length pinned by `COVERAGE_IS_TOTAL`, and
  `check_coverage` refuses `fabricated_axis` (a row claiming values the axis does not declare) and
  `uncovered_axis` (an axis with no row, an empty row, two rows for one axis).

## 2. The asking (`laya`)

| Shape | Fields / variants | Notes |
|---|---|---|
| `Question` | `Choice { options, passes }`, `Score { levels, floor }`, `Noul { threshold }` | the floor is declared **before** the answer exists; `choice` refuses fewer than two options, `score` a floor above the top rung, `noul` a threshold outside `(0, 1]` |
| `Verdict` | `Belongs`, `NotBelongs`, `NoReading` | three **words**, no prose between them; `is_a_verdict` separates the third |
| `Ballot` | `id`, `claim`, `artifact`, `question`, `instruction`, `presenter`, `max_age_ticks` | a complete ballot or a refusal: `ballot_incomplete`, `floor_undeclared` |
| `Judgement` | `chosen`, `scores` (every option, in the question's order), `features` (what the instrument read) | a summary would hide how close the second option was |
| `Measurement` | `readings`, `per_option`, `majority`, `in_sample`, `leave_one_out`, `leave_one_out_with_absence`, `leave_one_out_from_the_claim_only`, `known_answers` (digest) | `beats_majority` is **strict**: a tie is not a beat; `quotable` is the gate |
| `Answer` | `verdict`, `origin`, `evidence` | `passed()` returns `Err(no_reading)` for a non-reading |
| `Evidence` | `artifact`, `digest`, `features`, `scores`, `readings`, `majority`, `leave_one_out`, `reason` | the reading is re-derivable without the instrument; the reason is required for a negative reading |

## 3. The reading (`von`)

| Shape | What it is | Count |
|---|---|---|
| `Declared` | the head's own declared facts — one per word of the format's vocabulary | **12** |
| `Feature` | the claim's features — what the body's structure declares | **14** |
| one column space | the head's facts at `0..12`, the claim's features at `12..26` | **26** |
| `Reading` (private) | one labelled example: its stage, its head's facts, its claim's features | 16 |
| `Classifier` | the options, the rows, the figures | — |
| `Vocabulary`/`Family` | the declared words, what each claims, how many records carry it, how many **outside** the calibration set | 8 words |
| `KNOWN_ANSWERS` | the sixteen labelled `hyperskill/*` readings | 16 |

**The estimator**, in one line: the log-posterior of an option is the Laplace-smoothed prior plus the log rate
of each **held** feature, over the fold that excludes the reading being answered (`Estimator::Held`), with
`Estimator::HeldAndAbsent` as the reported alternative and `space` fixing which half of the record may be read
as absence. A tie is a non-reading; it counts as a miss in the measurement.

## 4. The measured shape of the corpus (this pass)

| Figure | Value |
|---|---|
| readings / stages / majority class | 16 / 5 / **0.3125** |
| in sample | 0.7500 |
| leave-one-out (**the bar**) | **0.5000** (margin **+0.1875**) |
| leave-one-out, absence variant | 0.4375 |
| leave-one-out **from the claim only** | **0.0000** — the finding that moved the question to the head |
| corpus read for the vocabulary | **39** records, 8 words, **24** readings outside the calibration set, 0 contradictions |
| the sixteen bodies' structure | all four sections, the record's own vocabulary quoted, 19–53 lines — the matrix is pinned by a test |

## 5. What is *not* in the model

No model weights, no network, no cache, no clock, no filesystem beyond **reading** records in place, no
mutation of any artifact, no second parser of the record (one fence reader, one head-field reader), and no
number anywhere that an axis was reduced into.
