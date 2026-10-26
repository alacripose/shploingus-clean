---
patch_id: PATCH-001
title: Scientific-method candidate loop
status: proposed
operation: add
target: hyperskill/scientific-method
base_ref: orchestrating-ticket-agents-internal
base_digest: sha256:f1fb8e4c20c4445f91a953035b2a924e9e3c26db3f1fe08778d1880b28086b8a
dependencies: []
source: user-approved-conversation
created_at: 2026-09-24
approval: design-approved
live_mutation: false
---

# Change

Add a self-contained scientific-method process to the generated `hyperskill/scientific-method/SKILL.md`. This patch concerns the scientific method only. It does not redefine the general ticket system, agent permissions, delegation, scheduling, orchestration, or other responsibilities covered by separate patches.

The process is a repeatable candidate-inquiry loop, not a guarantee that a theory will improve or that a question can be resolved with available evidence. A paper is a source of evidence, not an authority that substitutes for evaluating the methods, data, scope, and limitations behind its claims.

The following section is the proposed standalone skill content. It must remain usable without requiring the reader to consult the design or plan documents named in the patch's Evidence section.

---

# Scientific-Method Candidate Loop

## 1. Purpose and operating principles

Use this process to investigate a question, assess a proposed explanation, test predictions, document uncertainty, and decide what should happen to the current theory candidate. The method applies to empirical, observational, computational, mathematical, historical, and literature-based inquiries, with the test type chosen to fit the question. It must not pretend that every question can be answered by a laboratory experiment.

The process is:

**question and scope → hypothesis and predicted observation → source set and inclusion criteria → source cards and claim/evidence map → conditional diagram → falsification, documentation, and reproduction experiment → independent cross-grill → update, hold, retire, or request more evidence.**

The process is iterative. A result may send the inquiry back to revise its scope, inspect a missing source class, improve an operational definition, or design a more discriminating test. A loop-back creates a recorded revision; it must not silently rewrite the earlier question, prediction, criteria, or result.

Apply these principles throughout:

1. **Separate claim, prediction, observation, and interpretation.** They are related but not interchangeable.
2. **Make claims testable where possible.** State what evidence would count against the candidate, not only what could be interpreted as support.
3. **Make scope explicit.** A result applies to the population, system, conditions, measurements, and methods actually studied unless additional evidence supports generalization.
4. **Keep provenance.** Preserve source identifiers, search decisions, raw observations where available, analysis steps, versions, exceptions, and reviewer objections.
5. **Represent uncertainty honestly.** Distinguish missing evidence, imprecise evidence, conflicting evidence, evidence against a claim, and evidence consistent with a claim.
6. **Report every material outcome.** Positive, negative, null, contradictory, incomplete, failed, and aborted runs remain part of the record.
7. **Do not overstate inference.** Correlation alone does not establish causation; a simulation is not by itself a measurement of the real world; a failed replication does not automatically show that the original finding was fraudulent or universally false.
8. **Do not promote a candidate automatically.** Completion of the loop may inform a recommendation, but no test result or agent may promote a candidate without the required approval outside this scientific-method process.
9. **Preserve prior states.** Updates append a new version or event. Retiring a candidate does not delete its supporting, opposing, or unresolved evidence.

## 2. Stage A — Question and scope

### Goal

Turn a broad interest or assertion into a question that can be investigated with an identifiable body of evidence.

### Required record

Create a research brief containing:

- `inquiry_id`: stable identifier for the inquiry.
- `question`: one primary question, phrased without assuming its answer.
- `question_type`: descriptive, explanatory/mechanistic, causal, predictive, comparative, evaluative, mathematical/formal, historical, or methodological; select more than one only when needed.
- `target_claims`: claims that prompted the inquiry, each given a stable ID.
- `scope_in`: system or phenomenon, population or units, place/domain, time interval, operating conditions, and context being investigated.
- `scope_out`: explicitly excluded cases and adjacent questions not being answered.
- `operational_definitions`: measurable or otherwise inspectable meanings for key terms; identify terms that are not yet operationalized.
- `assumptions`: premises being provisionally accepted so the inquiry can proceed.
- `decision_relevance`: what the answer may inform. Do not let the desired decision dictate the result.
- `known_constraints`: unavailable instruments, data, expertise, time, access, language, ethical permissions, or computational resources.
- `initial_state`: what is known, unknown, disputed, and merely proposed before this inquiry begins.

### Method

1. Identify the phenomenon and the smallest answerable question that preserves the intended issue.
2. Separate factual questions from value judgments, design preferences, and recommendations. A scientific result may inform a value-based decision without settling the value judgment itself.
3. Define what counts as an observation or evidence item for this question.
4. Identify boundary conditions. State where a proposed answer would and would not be expected to apply.
5. Check whether the question can be tested directly. If not, divide it into narrower questions or state which parts can currently be examined.
6. Record unresolved terms or assumptions instead of silently choosing a convenient definition.

### Gate A — Scope is adequate

Proceed only when the question, key terms, intended scope, and relevant constraints can be understood by a reviewer. A question need not be fully answerable to enter the record, but any untestable component must be marked as such. If the scope is too broad or circular, revise it before selecting evidence.

## 3. Stage B — Hypothesis and predicted observation

### Goal

State candidate explanations and derive observations that should follow if each explanation is adequate within the defined scope.

### Required record

For every candidate hypothesis, record:

- `hypothesis_id` and version.
- `hypothesis`: one concise proposition. Split compound propositions into separately assessable claims.
- `mechanism`: proposed causal or logical pathway, if one is asserted; label it as proposed rather than demonstrated.
- `scope_conditions`: the conditions under which the hypothesis is claimed to apply.
- `predictions`: observable or derivable consequences, each with a stable `prediction_id`.
- `measurement_or_test`: how each predicted observation will be detected or derived.
- `expected_direction_or_pattern`: the predicted direction, magnitude range, ordering, structure, or qualitative pattern, as appropriate.
- `failure_condition`: the observation or result that would count against the prediction.
- `competing_hypotheses`: plausible alternatives, including a suitable null or baseline model where applicable.
- `discriminating_test`: what could distinguish this hypothesis from alternatives rather than merely being compatible with all of them.
- `auxiliary_assumptions`: assumptions needed to connect the hypothesis to the prediction, including instrument validity, model assumptions, and data quality.
- `registration_state`: whether predictions were recorded before observations or analysis. If retrospective, mark them as retrospective.

### Method

1. Derive predictions from the hypothesis rather than writing predictions after seeing the results.
2. State at least one plausible observation that would weaken or contradict the hypothesis within its stated scope.
3. Prefer predictions that separate competing hypotheses. A prediction that every plausible explanation makes is weakly discriminating.
4. Include null, baseline, and alternative explanations where appropriate; do not create a straw-man alternative solely to make the preferred hypothesis look stronger.
5. Record quantitative thresholds, decision rules, outcome measures, and stopping conditions before examining confirmatory results whenever feasible.
6. Mark exploratory predictions separately from confirmatory predictions. A prediction invented after an outcome is known may be useful, but it is not a prospective successful prediction and requires new testing.
7. Avoid making a hypothesis immune to challenge by adding exceptions only after each disconfirming observation. If the scope or auxiliary assumptions change, record the reason and version the hypothesis.

### Gate B — Predictions are discriminating and inspectable

A hypothesis can remain a candidate even when predictions are not yet measurable, but it cannot be presented as empirically tested until a test and interpretation rule are specified. If no possible result could count against it, state that limitation explicitly and do not represent it as a falsifiable empirical hypothesis.

## 4. Stage C — Source set and inclusion criteria

### Goal

Define how the available evidence will be found and selected before allowing the source set to determine the preferred answer.

### Required record

Create a source-search protocol containing:

- research question and claims the search is intended to inform;
- information sources consulted, such as bibliographic databases, scientific repositories, standards bodies, datasets, registries, archives, or authoritative measurement records;
- search date for each source and, where relevant, coverage period;
- exact search terms, query expressions, filters, and search paths, to the extent the source permits recording them;
- inclusion and exclusion criteria for source type, date range, language, population/system, methods, outcomes, and relevance;
- a rationale for consequential restrictions;
- method used to deduplicate records and screen titles, abstracts, summaries, or full texts;
- reasons for exclusion, especially for potentially relevant studies or studies missing outcomes of interest;
- any search limitations, unavailable documents, paywalls, translation limits, or suspected indexing gaps;
- the point at which the search was stopped and why;
- whether the search was systematic, structured but bounded, or exploratory. Do not call a search exhaustive unless that characterization is justified.

### Method

1. Choose sources appropriate to the discipline and question. Prefer primary data or primary reports when evaluating what was directly measured; use reviews and syntheses to discover evidence and understand the broader record.
2. Seek evidence capable of supporting and challenging the candidate. Search for null findings, failed replications, alternative explanations, methodological criticism, and relevant contrary results rather than searching only for confirming wording.
3. Apply the stated inclusion and exclusion criteria consistently. If a criterion changes, record the change, its date, its rationale, and which records are affected.
4. Distinguish a source not found from a source searched for and excluded. Distinguish an inaccessible source from a source that does not exist.
5. Track studies and reports separately when multiple reports concern the same underlying dataset or experiment, so they are not accidentally counted as independent evidence.
6. Do not treat publication, peer review, institutional prestige, citation count, or authorship as proof that an individual claim is correct. Those attributes may be relevant context but do not replace methodological assessment.
7. If this is a systematic review or meta-analysis, use an appropriate discipline-specific reporting guideline where applicable. PRISMA is a reporting guideline for systematic reviews, not a universal method for every scientific question.

### Gate C — Selection is auditable

A reviewer must be able to understand where the evidence came from, what was eligible, why material records were excluded, and what limitations affect coverage. A small source set is acceptable when reported honestly; a selective source set presented as comprehensive is not.

## 5. Stage D — Source cards and claim/evidence map

Both artifacts are required for every theory project. A source list alone does not satisfy this requirement.

### 5.1 Source cards

Create one source card for each source materially used in the reasoning. Include:

- `source_id`;
- full bibliographic citation and a stable identifier or URL, such as DOI, report number, dataset ID, archive locator, or versioned repository reference;
- source type: primary study, data release, systematic review, theoretical paper, method paper, standard, archival record, or other identified type;
- which specific claim(s) it is being used to inform;
- study/question design, population/system, sample or data size where applicable, time and context;
- methods, instruments, controls/comparators, analysis, and outcome definitions relevant to the claim;
- reported result, including effect estimate or uncertainty measures where applicable;
- relevant limitations, assumptions, missing data, exclusions, conflicts of interest, funding, and methodological concerns when available;
- whether data, code, protocol, or supplementary materials can be inspected;
- relationship to other sources, including possible dataset overlap, dependence, correction, retraction, or replication;
- the author's stated conclusion, kept separate from the skill's own interpretation;
- the skill's evidence interpretation: supports, contradicts, qualifies, contextualizes, or does not directly bear on the claim;
- confidence in the card's extraction itself: verified from full text/data, abstract only, secondary description, or not yet verified.

Use accurate paraphrases and only the excerpts needed to locate or verify a claim. Do not invent details absent from the source. When source content cannot be inspected, mark the missing fields as unavailable rather than reconstructing them from the title or abstract.

### 5.2 Required claim/evidence map

Create a map with one row per atomic claim. A theory project must not proceed to a concluded candidate disposition without this map. If no evidence has been found for a claim, include the claim and state `no qualifying evidence identified`; never omit the row to make the map appear stronger.

Required columns:

| Field | Requirement |
|---|---|
| `claim_id` | Stable identifier for one assessable claim. |
| `claim_text` | Exact scoped proposition being evaluated. |
| `claim_type` | Observation, association, causal, mechanism, prediction, mathematical, methodological, or interpretive. |
| `scope_conditions` | Conditions and boundaries applicable to the claim. |
| `evidence_ids` | Source-card, dataset, experiment, or analysis IDs linked to the claim. |
| `relationship` | Supports, contradicts, qualifies, mixed, or no qualifying evidence identified. |
| `directness` | Direct test, indirect inference, background context, or analogy. |
| `method_quality_notes` | Relevant design, measurement, selection, confounding, analysis, and reporting limitations. |
| `alternative_explanations` | Plausible ways to explain the same observation. |
| `independence_notes` | Dependencies, shared datasets, shared instruments, author/lab overlap, or other non-independence. |
| `uncertainty_and_gaps` | Precision limits, missing evidence, unresolved conflicts, and unknowns. |
| `current_assessment` | Qualified statement of what the evidence presently warrants and does not warrant. |
| `next_discriminating_test` | The test or source that could most usefully resolve the remaining uncertainty. |

### 5.3 Evidence interpretation rules

- Do not count papers as if every paper represented an independent experiment. Assess independence and underlying data overlap.
- Do not reduce evidence quality to a single score unless a validated, fit-for-purpose scoring method is specified and its limitations are reported.
- Distinguish evidence of absence from absence of evidence. A null result is informative against an effect only to the extent that the measurement, sensitivity, sample size, design, and uncertainty could have detected the effect in question.
- Interpret statistical results in context. A p-value is not the probability that a hypothesis is true, and crossing a threshold does not by itself settle a scientific claim. Report effect estimates, uncertainty, design limitations, and practical or scientific relevance where applicable.
- Evidence can support a narrow claim while failing to support a broader interpretation attached to it. Map these separately.
- Contradictory findings remain visible. Investigate differences in measurements, populations, protocols, assumptions, and uncertainty rather than automatically averaging or discarding inconvenient findings.
- A paper's conclusion must not be copied as the claim/evidence map's verdict without independent examination of the supporting method and data.

### Gate D — Every theoretical claim is traceable

A reviewer must be able to follow each claim to relevant evidence and understand why that evidence bears on it. Missing, indirect, weak, and contradictory evidence must be explicitly represented. If the map is incomplete, the inquiry remains open or is marked as a request for more evidence.

## 6. Stage E — Conditional diagram

### Goal

Represent the candidate's logic so that assumptions, mechanisms, dependencies, and predicted outcomes can be inspected rather than remaining implicit in prose.

### Required diagram contents

Create a conditional diagram in a readable, versionable format. A text-native format such as Mermaid may be used when supported. The diagram must include, as relevant:

- the question and scoped outcome of interest;
- the candidate hypothesis and competing hypotheses;
- claimed mechanisms or intermediate steps;
- observable indicators and measurement path;
- relevant initial conditions and boundary conditions;
- confounders, alternative causes, moderators, mediators, and selection effects where applicable;
- predictions that would follow under stated conditions;
- findings or tests that could discriminate between explanations;
- assumptions and nodes for which evidence is missing;
- links to claim IDs, prediction IDs, source-card IDs, and experiment IDs.

Label each material node or connection as one of: `observed`, `evidence-supported`, `inferred`, `assumed`, `predicted`, `contradicted`, or `untested`. Define any additional labels in the diagram legend.

### Method

1. Draw conditional relations explicitly: “if condition A holds, then prediction B is expected under hypothesis H,” including relevant qualifications.
2. Do not draw a causal arrow merely because two variables are correlated. If causality is proposed, identify the evidence or assumptions supporting that link and the alternative pathways still possible.
3. Show branches for competing explanations rather than presenting one path as inevitable.
4. Mark uncertainty and missing links; do not use visual completeness to imply evidentiary completeness.
5. Trace at least one plausible failure path: which observation would break a key prediction or mechanism link?
6. Cross-reference diagram elements to the claim/evidence map. The diagram is a reasoning aid, not independent proof.

### Gate E — Logic can be challenged

A reader should be able to point to a link, assumption, condition, or predicted observation and ask what justifies it. If the diagram cannot represent an unresolved dependency honestly, add a note or split the diagram rather than hiding it.

## 7. Stage F — Falsification, documentation, and reproduction experiment

### Goal

Design and carry out a test suited to the question, report the procedure and outcomes transparently, and establish what can be independently checked or reproduced.

### 7.1 Select a suitable test class

Choose and justify the test class that best addresses the prediction:

- controlled or quasi-experimental test;
- observational study or natural experiment;
- measurement or instrument-validation study;
- computational analysis or reanalysis;
- simulation or model comparison;
- formal derivation or mathematical proof attempt;
- archival, historical, or documentary analysis;
- systematic evidence synthesis;
- independent reproduction or replication;
- another explicitly described method.

Do not describe a simulation as empirical confirmation of the external world unless it is validated against relevant real-world evidence. Do not present a formal proof of a mathematical statement as experimental evidence for a separate empirical claim. Where a direct test is not feasible, state the limitation and choose the strongest valid indirect test without disguising it as direct evidence.

### 7.2 Protocol before test

Create a versioned test protocol containing:

- `experiment_id` or `test_id` linked to the relevant hypothesis, predictions, claim IDs, and diagram;
- exact question and prediction being tested;
- method choice and rationale;
- materials, instruments, source data, software, model version, code revision, dependencies, and environment where applicable;
- units, samples, population, selection procedure, experimental unit, and intended sample size where applicable;
- independent/dependent variables or equivalent inputs and outcomes;
- controls, comparators, baselines, negative controls, and positive controls where appropriate;
- calibration and measurement procedure;
- inclusion/exclusion rules, missing-data handling, outlier handling, and stopping rules;
- randomization, blinding, masking, counterbalancing, or a reason these are not feasible, where applicable;
- primary outcomes and secondary or exploratory outcomes;
- analysis methods, assumptions, uncertainty estimates, sensitivity checks, and multiplicity handling where relevant;
- criteria for results that support, weaken, contradict, or leave the prediction unresolved;
- risk, ethics, safety, privacy, and approval requirements relevant to the planned work;
- deviations that would invalidate, qualify, or require rerunning the test;
- what materials, code, data, and instructions an independent reviewer would need to inspect the result.

For quantitative work, justify sample size or explain the practical/technical basis for it. State the precision, detection limit, uncertainty, power, or practical equivalence bounds relevant to interpretation when these concepts apply. Do not invent formal power calculations or thresholds when the data or design cannot support them; document the limitation instead.

Record the confirmatory protocol before examining the confirmatory result whenever feasible. If protocol changes occur after seeing data, preserve the original plan, document the change and reason, identify which analyses are now exploratory, and avoid representing a post hoc prediction as prospectively confirmed.

### 7.3 Execute and preserve the record

During execution, record:

- date/time and operator or process identity as appropriate;
- actual inputs, sample counts, exclusions, missing values, and run conditions;
- raw observations or a stable reference to them, subject to lawful and ethical access limits;
- code and environment versions, configuration, random seeds where meaningful, and checksums for important artifacts where practical;
- commands or procedural steps needed to rerun the analysis;
- deviations, failures, interruptions, anomalies, and unexpected results;
- complete planned outcome results, not only results that favor the candidate;
- whether the run completed, failed, was abandoned, or could not be interpreted, and why.

Do not silently replace raw results with cleaned results. Keep transformations and exclusions reproducible, with the rationale and consequences recorded. Preserve enough detail to distinguish a failed hypothesis from a failed instrument, invalid protocol, broken computation, insufficient sample, or execution error.

### 7.4 Analyze against predeclared predictions

For each prediction, record one provisional outcome category:

- `consistent`: observations are compatible with the prediction within stated uncertainty;
- `weakened`: observations reduce the candidate's plausibility or show a smaller/narrower effect than predicted;
- `contradicted`: observations conflict with a key prediction under conditions where the test was capable of detecting the predicted outcome;
- `inconclusive`: data or design cannot distinguish among relevant possibilities;
- `not_tested`: the protocol did not test this prediction;
- `invalid_run`: execution or measurement failure prevents a valid interpretation.

These categories describe the relationship between a result and a scoped prediction. They do not mechanically label an entire theory as true or false.

Compare observations with the planned prediction and uncertainty, not only with a narrative constructed after the result is known. Report effect magnitude and uncertainty where applicable, check sensitivity to reasonable analysis choices, and examine alternatives and assumptions. Separate primary analyses from exploratory analyses. A result that differs from expectation must not be removed merely because it is inconvenient.

### 7.5 Reproduce and replicate distinctly

Use these terms explicitly in the record:

- **Computational reproducibility:** another person can obtain consistent computational results from the same inputs/data, code, methods, and analysis conditions.
- **Replication:** an independent study uses newly obtained data to test the same or a closely equivalent scientific question and assesses consistency of results, accounting for expected uncertainty and contextual differences.
- **Robustness or sensitivity analysis:** the conclusion is examined under alternative reasonable analysis choices, assumptions, or conditions. This is useful but is not identical to independent replication.

When appropriate and feasible, have an independent party rerun the analysis from preserved materials or repeat the study with new data. If this is not possible, state why and classify the evidence accordingly. A failed replication is a result to investigate—not an automatic verdict—and must not be hidden. Report how protocol deviations, measurement precision, changed conditions, or genuine heterogeneity could affect comparison.

### Gate F — Result is inspectable

The record must let another reviewer identify what was planned, what was done, what happened, how results were analyzed, and which deviations matter. If a required artifact is unavailable, note the exact limitation. If the test was invalid or underpowered to resolve the question, do not use it as conclusive evidence for either side.

## 8. Stage G — Independent cross-grill

### Goal

Subject the full candidate argument to a substantive challenge that is separate from the process that produced the initial interpretation.

The reviewer must not be treated as independent if they simply repeat the original author's conclusion without re-examining the materials. When feasible, the reviewer should inspect the protocol and raw or source-level evidence before reading the proposed disposition, or otherwise document how exposure to the initial conclusion may have affected the review.

### Required cross-grill record

The reviewer produces a challenge report linked to the inquiry version, not an informal approval. It must address:

1. **Question and scope:** Is the question answerable as framed? Are key terms circular, ambiguous, or shifting?
2. **Hypothesis and predictions:** Are predictions derived from the hypothesis? Could the prediction fail? Were thresholds or criteria chosen after seeing the outcome?
3. **Alternatives:** Are plausible competing explanations and null/baseline models fairly represented? Is the test discriminating?
4. **Source selection:** Are source coverage and inclusion criteria defensible? Are contrary, null, unpublished, corrected, or failed-replication results missing or inaccessible?
5. **Source quality:** Do the reported methods and data actually support the claims attributed to each source? Are sources independent, or do multiple reports repeat the same evidence?
6. **Claim/evidence map:** Does every material claim have a row and traceable evidence? Are absence of evidence and evidence against a claim being conflated?
7. **Conditional diagram:** Are causal links, hidden assumptions, confounders, boundary conditions, and untested dependencies visible?
8. **Test validity:** Could measurement error, selection bias, leakage, confounding, missing data, inadequate sensitivity, invalid code, or protocol deviations explain the result?
9. **Analysis integrity:** Are effect sizes and uncertainty represented? Are multiple comparisons, post hoc analyses, stopping rules, and exclusions disclosed where relevant?
10. **Reproduction and replication:** Are the terms used correctly? Can a second party check the analysis? Are inconsistent replications explained rather than suppressed?
11. **Conclusion calibration:** Does the proposed disposition follow from the actual evidence and uncertainty, without stronger language than the result warrants?
12. **Missing next test:** What feasible observation or analysis would most strongly distinguish the remaining explanations?

For every finding, record severity (`blocking`, `material`, or `minor`), evidence or artifact reference, reasoning, and the action needed. The original author responds point by point with `accepted`, `partially accepted`, `disputed`, or `unresolved`, plus a rationale and any changed artifact IDs. Preserve unresolved disagreements. The reviewer may recommend a disposition but cannot make approval requirements disappear.

### Gate G — Challenge is resolved or preserved

A material unresolved criticism prevents the inquiry from being presented as fully reviewed. The inquiry may still conclude `hold` or `request more evidence`, but it must identify the open criticism. Disagreement is not resolved merely by seniority, confidence, number of agents, or repetition.

## 9. Stage H — Disposition: update, hold, retire, or request more evidence

Choose exactly one current disposition for the candidate version. Explain the reasoning and link it to the claim/evidence map, test outcomes, uncertainty, and cross-grill.

### `update`

Use when the evidence justifies revising the hypothesis, its scope, mechanism, assumptions, confidence language, or predictions. Preserve the prior version and record each substantive change. State whether the revised claim is based on a prospective prediction, an exploratory finding, or a post hoc reinterpretation. A materially revised hypothesis requires new discriminating predictions and, where feasible, a fresh independent test; it must not inherit confirmation credit from results used to invent it.

### `hold`

Use when the current evidence does not justify a material change but also does not resolve the question. Examples include mixed findings, limited precision, plausible competing explanations, or an incomplete but informative source set. State what remains uncertain and when a hold should be reconsidered. `Hold` is not a finding that the candidate is true.

### `retire`

Use when the candidate version is no longer worth pursuing in its present form—for example, a critical prediction is contradicted by a valid and sufficiently discriminating test, a required premise fails, or a better-supported explanation makes this version unnecessary. Define the scope of retirement. Retiring a scoped candidate is not a claim that every broader related proposition is universally false. Preserve the candidate, original predictions, results, objections, and retirement rationale. A later revival is a new, linked version with a reason for reconsideration.

### `request_more_evidence`

Use when the next justified step is to obtain specific missing evidence, repeat or repair an invalid test, seek an inaccessible source, increase measurement precision, assess an alternative, or conduct an independent replication. Specify the evidence needed, why it is discriminating, how it could be obtained, and which result would change the disposition. Avoid vague requests such as “do more research.” If no feasible path is known, state that rather than promising resolution.

### Required disposition record

- current hypothesis/version and scope;
- chosen disposition and date;
- strongest evidence consistent with the candidate;
- strongest contrary or limiting evidence;
- null, contradictory, incomplete, and invalid results relevant to the decision;
- unresolved cross-grill findings;
- main uncertainty and limitations;
- rationale connecting the evidence to the disposition;
- next discriminating test or reason no feasible next test is known;
- approval status, recorded separately from scientific assessment;
- links to every artifact and prior version.

### Promotion rule

The output uses calibrated, non-conclusive language such as `currently consistent with`, `provisionally supported within this scope`, `weakened by`, `contradicted under these conditions`, `unresolved`, or `insufficient evidence to distinguish`. Avoid declaring a hypothesis proven by a finite test, and do not promote a candidate solely because it completed the loop, passed one test, received a high source count, or survived an agent review. Candidate promotion requires the applicable explicit approval process outside this patch.

## 10. Scientific record and report format

Every completed loop iteration produces a report containing, at minimum:

1. inquiry ID, version, question, and scope;
2. hypothesis version, alternatives, and predictions;
3. source-search protocol and coverage/selection limitations;
4. source cards and the complete claim/evidence map;
5. conditional diagram with a legend;
6. test protocol, methods, materials/data/software references, and predeclared criteria;
7. actual execution record, deviations, and complete outcome table;
8. analysis, uncertainty, alternative explanations, nulls, contradictions, and failures;
9. reproducibility/replication status, with terms used precisely;
10. independent cross-grill report and point-by-point response;
11. disposition and its rationale;
12. outstanding uncertainty, next discriminating test, and approval status;
13. version history and links/hashes for important artifacts where practical.

The report must distinguish source-reported conclusions from the inquiry's assessment. It must disclose important missing materials rather than implying that they were reviewed. A concise executive summary may precede these sections but must not replace them.

## 11. Iteration and version control

- Each substantive change to the question, scope, hypothesis, prediction, inclusion criteria, test protocol, analysis plan, or disposition creates a dated revision or event.
- Preserve the original plan and make deviations easy to compare with the final plan.
- A source correction, retraction, new dataset, replication, or newly found contradiction updates the evidence map and may trigger a new loop iteration.
- Do not delete superseded claims or failed tests from the audit record. Mark them superseded, invalid, or retired with a reason.
- Do not allow a downstream summary to be stronger than the underlying claim/evidence map and test report.
- When evidence is insufficient, a correctly documented unresolved state is a successful process outcome; fabricated certainty is not.

## 12. Minimal output templates

### Claim/evidence row

```yaml
claim_id: CLAIM-001
claim_text: "One atomic, scoped proposition"
claim_type: causal
scope_conditions: "Population/system, context, and boundary conditions"
evidence_ids: []
relationship: no_qualifying_evidence_identified
directness: not_assessed
method_quality_notes: []
alternative_explanations: []
independence_notes: []
uncertainty_and_gaps: ["Evidence search not yet complete"]
current_assessment: "No assessment beyond the absence of qualifying evidence so far."
next_discriminating_test: "Specify an observation or source that could change the assessment."
```

### Prediction and test outcome

```yaml
prediction_id: PRED-001
hypothesis_id: HYP-001
prediction: "Observable pattern expected under stated conditions"
failure_condition: "Result that would count against this prediction"
registration_state: prospective
 test_id: TEST-001
outcome: inconclusive
observed_result: "Report what was actually observed, including null or contrary results."
uncertainty: "Describe measurement and inferential limits."
deviations: []
interpretation: "State only what the result warrants within the declared scope."
```

### Cross-grill finding

```yaml
finding_id: REVIEW-001
severity: material
area: test_validity
artifact_refs: ["TEST-001"]
challenge: "Specific methodological or inferential concern"
rationale: "Why this concern matters to the claim"
required_action: "Repair, test, qualify, or explain the concern"
author_response: unresolved
response_rationale: "Point-by-point response"
```

### Disposition

```yaml
disposition_id: DISP-001
candidate_version: HYP-001-v1
state: request_more_evidence
rationale: "Evidence does not yet distinguish the main alternatives."
strongest_consistent_evidence: []
strongest_contrary_evidence: []
unresolved_findings: []
next_evidence_needed: "Name the specific source, observation, or test required."
approval_status: not_promoted
supersedes: null
```

## 13. References informing this method

These references inform parts of the workflow; they do not replace domain-specific methods or constitute automatic authority for any candidate:

- National Academies of Sciences, Engineering, and Medicine, *Reproducibility and Replicability in Science* (2019), especially its distinction between computational reproducibility and replication using newly collected data: https://www.nationalacademies.org/read/25303/chapter/3
- National Institutes of Health, *Rigorous Study Design and Transparent Reporting*, including transparent methods, inclusion/exclusion criteria, missing results, uncertainty, and independent validation: https://beta.nih.gov/funding/preparing-your-application/preparing-research-plan/rigorous-study-design-and-transparent-reporting
- PRISMA 2020, guidance for transparent reporting of systematic reviews, including explicit eligibility criteria, information sources, search strategies, selection processes, and exclusions: https://www.bmj.com/content/372/bmj.n71.full-text.print
- Wasserstein, R. L., and Lazar, N. A. (2016), *The ASA's Statement on p-Values: Context, Process, and Purpose*, on avoiding threshold-only scientific conclusions and interpreting statistical results in context: https://doi.org/10.1080/00031305.2016.1154108

These references should be consulted in context. PRISMA is applicable to systematic reviews, NIH study-design guidance is rooted in research-grant and biomedical contexts, and the National Academies' definitions are adopted for clarity in this skill; terminology may vary between disciplines and should be declared when it does.

---

# Acceptance

The patch is acceptable only if all requirements below are met.

## Content requirements

- The standalone `hyperskill/scientific-method/SKILL.md` contains every stage: question and scope; hypothesis and predicted observation; source set and inclusion criteria; source cards and claim/evidence map; conditional diagram; falsification/documentation/reproduction experiment; independent cross-grill; and update, hold, retire, or request more evidence.
- The loop order is explicit, and permitted returns to earlier stages are documented as versioned iterations.
- A claim/evidence map is mandatory for every theory project, including explicit rows for claims with no qualifying evidence.
- Source cards record both source provenance and the relevance/limitations of evidence used for claims.
- The conditional diagram marks assumptions, inferences, predictions, observations, and untested or contradicted links.
- The test protocol distinguishes planned/confirmatory work from exploratory or post hoc work and records relevant controls, exclusions, uncertainty, and deviations.
- Computational reproducibility is distinguished from replication using new data.
- Null, contradictory, incomplete, failed, invalid, and aborted results remain visible in the record and final report.
- The cross-grill includes substantive independent challenges and a response record; unresolved objections remain visible.
- Disposition is one of `update`, `hold`, `retire`, or `request_more_evidence`, with an evidence-linked rationale.
- Outputs use non-conclusive language proportionate to the evidence and do not promote a candidate without approval.
- The scientific-method skill is self-contained. Its use does not require the design/plan files listed under Evidence to fill in missing procedural instructions.
- The change is limited to `hyperskill/scientific-method`; it does not silently absorb responsibilities assigned to other patches.

## Behavioral checks

1. **No evidence yet:** the process creates a claim/evidence map row marked `no qualifying evidence identified`; it does not infer that the claim is false or omit the claim.
2. **Contradictory sources:** both supporting and contrary evidence remain linked to the relevant claim; the disposition discusses the conflict rather than hiding one side.
3. **Null result:** the result is recorded with the test's sensitivity and uncertainty; it is not automatically labeled proof of no effect.
4. **Post hoc hypothesis:** the revision is marked post hoc and is not described as a prospectively confirmed prediction.
5. **Invalid test:** a broken instrument, data pipeline, or protocol results in `invalid_run` or a qualified record, not a claim that the hypothesis was falsified.
6. **Failed replication:** the failed replication remains visible and triggers investigation of uncertainty, method/context differences, and possible genuine disagreement.
7. **No independent reviewer:** the report states that independent cross-grill was not completed; self-review is not mislabeled independent review.
8. **Missing source access:** uninspected source details are marked unavailable; no methods or results are invented from a title or abstract.
9. **Changing criteria:** changed inclusion rules or test criteria are dated, justified, and linked to the affected records.
10. **Candidate promotion:** even a favorable run and successful review leave the candidate unpromoted unless the required explicit approval is recorded.
11. **Rollback:** reverting this patch restores the previous skill and compiler manifest while preserving the scientific record and its evidence.

# Evidence

- `docs/superpowers/specs/2026-09-24-orchestrating-ticket-agents-scientific-method-improvement-design.md`
- `docs/superpowers/plans/2026-09-24-orchestrating-ticket-agents-scientific-method-improvement.md`
- National Academies, *Reproducibility and Replicability in Science* (2019): https://www.nationalacademies.org/read/25303/chapter/3
- NIH, *Rigorous Study Design and Transparent Reporting*: https://beta.nih.gov/funding/preparing-your-application/preparing-research-plan/rigorous-study-design-and-transparent-reporting
- PRISMA 2020 statement: https://www.bmj.com/content/372/bmj.n71.full-text.print
- Wasserstein and Lazar, *The ASA's Statement on p-Values: Context, Process, and Purpose*: https://doi.org/10.1080/00031305.2016.1154108

# Rollback

Retain the prior `SKILL.md`, its SHA-256, and the prior compiler manifest. Reverting this patch must restore the prior skill/manifest state without deleting the scientific record, evidence artifacts, source cards, claim/evidence maps, test results, cross-grill reports, or candidate history generated while the patch was active. Rollback changes the active procedure, not the historical record.
