---
patch_id: PATCH-005
title: Governed documentation and diagram tickets
status: proposed
operation: add
target: hyperskill/documentation-and-diagrams
base_ref: orchestrating-ticket-agents-internal
base_digest: sha256:f1fb8e4c20c4445f91a953035b2a924e9e3c26db3f1fe08778d1880b28086b8a
dependencies: [PATCH-001, PATCH-003, PATCH-004]
source: user-approved-conversation
created_at: 2026-09-24
approval: design-approved
live_mutation: false
---

# Change

Make documentation and diagram work first-class governed tickets. Support source-card, claim-map, theory-update, experiment-brief, documentation-draft, diagram-brief, diagram-refresh, and independent-review task kinds.

Parallel documentation work is allowed only when source slices, write sets, and resources are disjoint. Claim-map integration remains coordinator-owned and serialized after its inputs.

# Acceptance

- Every documentation ticket has the seven contract fields and acceptance checks.
- A simple definition may correctly omit a diagram.
- Relational, process, state, or dependency complexity requires a diagram decision.
- No documentation or diagram ticket edits the live skill.
- The theory ledger remains coordinator-owned.

# Evidence

- Candidate fixture `documentation/workstreams.json`.
- Candidate scientific-method specification, documentation-and-diagram section.

# Rollback

Keep generated documentation and diagrams as derived evidence; restore the prior standalone skill without deleting prior artifacts.
