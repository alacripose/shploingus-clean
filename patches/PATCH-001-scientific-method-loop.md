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

Add a self-contained scientific-method loop to the generated skill. The loop is:

question and scope → hypothesis and predicted observation → source set and inclusion criteria → source cards and claim/evidence map → conditional diagram → falsification/documentation/reproduction experiment → independent cross-grill → update, hold, retire, or request more evidence.

The loop is a candidate process, not a guarantee of improvement. A paper is evidence, not authority.

# Acceptance

- The standalone `SKILL.md` contains every loop stage.
- A claim/evidence map is required for every theory project.
- Null, contradictory, and incomplete results remain visible.
- The output uses non-conclusive language and does not promote a candidate without approval.

# Evidence

- `docs/superpowers/specs/2026-09-24-orchestrating-ticket-agents-scientific-method-improvement-design.md`
- `docs/superpowers/plans/2026-09-24-orchestrating-ticket-agents-scientific-method-improvement.md`

# Rollback

Retain the prior `SKILL.md`, its SHA-256, and the prior compiler manifest. Reverting this patch must not delete the scientific record or its evidence.
