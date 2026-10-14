# Feature Specification: U7 — The decision layer (axis · laya · von)

> **Derived from documents** (Constitution P1). This file was created by spec-kit's own
> `create-new-feature.sh`, and it is filled from the record rather than from memory:
> `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §2 (U7) and §5 row 6, `docs/PLAN-2026-10-10-clean-room-and-the-api-layer.md`
> Lane C, `docs/LEDGER.md` R-59.5 · §59.5 item 12 · R-59.7 · C-102 §102.2 · C-97.2 · C-73 §73.1,
> `docs/HYPERGRILLING.md` H-14.1, and the two patch records that state the contract's own words —
> `patches/PATCH-023-the-ballot-and-the-von-classifier.md` and
> `patches/PATCH-039-the-corpus-beyond-this-tree.md`.
>
> **The bar this feature must reach**, in the plan's own words (§5 row 6): *a ballot answered with
> `Origin::Oracle`; the calibration accuracy beats the majority class.*
>
> **Measured, this pass** — see `plan.md` and `quickstart.md` for the commands: leave-one-out **0.5000** against
> a majority class of **0.3125** (a margin of **+0.1875**), with the claim-only reading at **0.0000** reported
> beside it because it is the finding that moved the question to the record's head.

## What U7 is, and what it is not

U7 is the layer that answers **questions about work**: *which stage of its life is this record at?*, *is this
text a decision or a description?*, *is this artifact at or above the declared floor?* — and it is the layer
that **refuses to answer** when the instrument behind it is not good enough to be quoted.

* **It is** the seven axes and their three prohibitions (`AxisKind`, `Ladder`, `refuse_interchange`,
  `refuse_reduction`, `refuse_bare`, the coverage table and its anti-fabrication guard); the judge's **three
  typed questions** (`Question::{Choice,Score,Noul}`) and the **three-word** `Verdict`
  (`Belongs`/`NotBelongs`/`NoReading`); the `Ballot` that declares the question, the artifact, the presenter,
  the staleness window **and the floor**; the `StructuralJudge` seam; the **calibrated classifier** that
  answers on this machine with no model; and the **four fail-closed rules**.
* **It is not** a model, a network, a cache or a prompt. The legacy witness is a 757 MiB ONNX session in its
  own crate (`patches/PATCH-023` §2, read as *behaviour*); the clean room re-derives the **contract** —
  questions, verdicts, floors, origin, refusals — and answers it with a calibrated structural reading, because
  a build that needs a model is a build that cannot run offline.
* **It is not** a second parser. The record's fence is read in **one** place (`von::regions`), the head's
  fields by **one** reader (`von::head_field`), and the ballot's grammar lives in `laya` and nowhere else.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - An agent asks a question about a record and gets an answer it may quote (Priority: P1)

A caller declares a `Ballot` over an artifact — *which stage of its life is this record at?* — with the floor
declared **before** the answer exists. The calibrated classifier reads the record's own head, answers with the
option it reads, and the answer comes back carrying `Origin::Oracle` and the evidence it was made of. A caller
may then quote it, because a calibration that does not beat its own majority class cannot produce one.

**Acceptance:** a ballot over `patches/PATCH-015-hypergrilling.md` is answered `belongs`, `origin A`
(`Oracle`), with the head facts it read; the same ballot refused by rule 1 against an instrument whose
calibration only ties its majority class.

### User Story 2 - The axes cannot be read as numbers, across ladders, or bare (Priority: P1)

The seven axes are a **vocabulary of words**, not scales. A caller comparing a grade with an origin, rendering
an axis as a percentage or a score, or writing a ladder token with no ladder named is refused **by name**, and
every axis is described **exactly once** by the coverage table — with a row that is well formed and untrue
caught by a guard that compares the row against the axis's own declared values.

**Acceptance:** `axis_interchange_refused`, `scalar_reduction_refused`, `bare_token_refused` (`AMBIGUOUS` for
`A`, `UNQUALIFIED` for `SSS`), `uncovered_axis`, `fabricated_axis` — each a test; 7 axes × 2 reductions = 14
refusals measured.

### User Story 3 - An instrument that is not good enough writes nothing (Priority: P2)

The same answer is asked of four instruments that cannot give one — an uncalibrated one, a missing one, one
asked a question its shape does not fit, and one whose reading is negative with no human's reason recorded.
Each is refused by its own name, and the gate prints the record's own words (`model-oracle not-established`).

**Acceptance:** `not_established`, `oracle_unreachable`, `no_reading` (and `Answer::passed` refusing a floor off
a non-reading), `reason_absent` — each proved to bite by name in the running binary, each also proved **live**
by perturbing the tree and restoring it byte-exact (`plan.md`).

### Edge Cases

* A document that is not a record at all (no frontmatter fence): no head, no claim, **no reading** — never a
  verdict for some stage, and never a pass.
* A record with a head but no body: not a calibration reading (the second question would have nothing to
  measure), refused by name.
* A record whose head declares none of the facts the instrument reads: a non-reading, not the majority class.
* A record whose head contradicts a word of its own vocabulary (`approval: applied-to-internal-base` and
  `status: designed`): refused **by name**, with the path, the word and the stage.
* A stored label that no longer matches the record's own declared stage: **stale rather than wrong** — the set
  is not edited to fit it.
* A labelled reading that is not there to be read: `source_absent`, naming the path.
* Two options supported equally: a **tie is not an answer** (`DomainTie`, one layer over).
* A `noul` question asked of the shipped structural instrument: this build ships **no known-answer set** for
  it, so the question is **refused by name** (`not_established`) rather than answered on the stage question's
  figures — an answer resting on another question's calibration is a measurement never made.

## Requirements *(mandatory)*

* **FR-001** `axis` shall declare the seven axes, each with a ladder of declared values, and the names of the
  axes whose ladders this build has **not** read shall be printed as *owed* by name rather than given invented
  values. *(Measured: **3 declared, 4 owed** — `certification`, `org_rank`, `gate_status`, `evidence_level`.)*
* **FR-002** `axis` shall refuse an interchange across ladders, a scalar reduction of any axis, and a bare
  ladder token — the last distinguishing `AMBIGUOUS` (two or more declared ladders claim it) from
  `UNQUALIFIED` (one claims it and it is still a bug).
* **FR-003** The coverage table shall describe every axis **exactly once**, be pinned total at compile time,
  and a row claiming values the axis does not declare shall be refused (`fabricated_axis`), as shall an axis
  with no row (`uncovered_axis`).
* **FR-004** `laya` shall declare the three questions (`Choice`, `Score`, `Noul`), the three verdicts as
  **words**, the `Ballot` (id, claim, artifact, kind, instruction, terms, presenter and the floor — its
  `max-age` is declared and checked at declaration but **not carried**, since this build cannot enforce
  staleness and a value nothing reads is not one a ballot should claim to hold), and shall refuse a ballot
  with an empty field, fewer than two terms, a `max-age` of zero, a floor outside its own terms, or two
  ballots sharing an id.
* **FR-005** The **floor shall be declared before the answer exists**: a `choice` names its passing option, a
  `score` its lowest passing rung, a `noul` its threshold.
* **FR-006** `answer` shall apply the four fail-closed rules in one place, each with its own name, and shall
  keep *unreachable* distinct from *uncalibrated*.
* **FR-007** An answer shall carry `Origin::Oracle` and **no ballot field may set it**; a non-reading shall
  carry no verdict a floor can be read off.
* **FR-008** `von` shall declare the **known-answer set** as a `const` of sixteen labelled readings (the
  sixteen shared `hyperskill/*` ids), check each stored label against the record's own declared stage, and
  refuse a label outside the question's options or an option no reading calibrates.
* **FR-009** The classifier shall read the record's **own declared facts** and its **claim** through **one**
  fence reader; the label's own key shall be read by no feature.
* **FR-010** The calibration shall report **in-sample, leave-one-out, the absence variant and the claim-only
  reading**, the bar shall be read **strictly** against the majority class, and a calibration that does not
  beat it shall refuse (`not_established`) — *the recorded defect is the recorded defect, wherever it sits*.
* **FR-011** The declared words the features are made of shall be measured over **every** record in the tree,
  not only the calibration set, and a record contradicting a word's claim shall be refused by name.

## Key Entities

`AxisKind` · `LadderValues`/`Ladder` · `EvidenceGrade`/`Grade`/`Origin` (the three ladders this build has read)
· `Reduction` · `CoverageRow` · `Question` · `Verdict` · `Ballot` · `Judgement` · `Measurement` ·
`StructuralJudge` · `Unreachable` · `Answer`/`Evidence` · `Declared` (the head's facts) · `Feature` (the claim's
features) · `Classifier` · `Vocabulary`/`Family` (the declared words and what they claim).

## Success Criteria *(mandatory)*

* **SC-001** A ballot over a real record is answered with `Origin::Oracle`, and the answer carries what it
  read. *(Measured: `belongs`, token `A`, three head facts.)*
* **SC-002** The calibration's leave-one-out accuracy beats the majority class of the set this project ships.
  *(Measured: **0.5000 > 0.3125**.)*
* **SC-003** The measurement is reported **either way**, including the estimator that loses and the half of the
  record that carries nothing. *(Measured: absence 0.4375, claim-only 0.0000, in sample 0.7500.)*
* **SC-004** Each of the four fail-closed rules bites **by name**, in the running binary, and each is proved
  live by a perturbation of the tree that is restored byte-exact.
* **SC-005** The vocabulary the features are made of is read over the whole corpus. *(Measured: 39 records,
  8 words, **24** readings outside the calibration set, 0 contradictions.)*
* **SC-006** No file under `crates/` is read as code, no dependency is added, and the gate exits 0 with
  `cargo test --offline`, `cargo clippy --offline --all-targets -- -D warnings` and `cargo run --offline`.

## Assumptions

* The sixteen labelled examples are the sixteen `hyperskill/*` records this tree carries (`patches/PATCH-001`
  … `PATCH-016`), each labelled with its **own declared stage** — the label is checked against the record, and
  the set is never edited to manufacture a pass (`docs/LEDGER.md` R-59.5).
* The five stages are the record's own (`patches/PATCH-039`): `proposed`, `designed`, `verified-internal`,
  `applied-internal`, `approved`.
* The stage question is answered from the record's **head** and not its claim, on `PATCH-039`'s own statement
  that this corpus is *"answerable per record, from ids and heads alone"* — and because the claim-only reading
  measures **0.0000** on these sixteen records.
* A model-backed instrument is a **seam** this slice does not fill: `Unreachable` is the shipped witness that a
  missing instrument is refused rather than read as a pass.

## Sources used, cited by path

`docs/CLEAN-ROOM-PLAN-2026-10-11.md` · `docs/PLAN-2026-10-10-clean-room-and-the-api-layer.md` ·
`docs/LEDGER.md` (R-59.5, §59.5 item 12, C-102 §102.2, R-59.7, C-73 §73.1, C-97.2, C-158) ·
`docs/HYPERGRILLING.md` H-14.1 · `patches/PATCH-023-the-ballot-and-the-von-classifier.md` ·
`patches/PATCH-039-the-corpus-beyond-this-tree.md` · the sixteen labelled records in `patches/` ·
`clean-room/.specify/memory/constitution.md` (P1–P7).
