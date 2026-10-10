---
patch_id: PATCH-007
title: Retirement record before completion
status: verified-internal
operation: add
target: hyperskill/retirement
base_ref: orchestrating-ticket-agents-internal
base_digest: sha256:77256380e6f950a5164581b07372423850b12887340871e32ae7d241c7a78ea8
dependencies: []
source: failed-eval-and-user-governance
created_at: 2026-09-24
approval: applied-to-internal-base
live_mutation: false
---

# Change

Require a separate coordinator-authored `ticket.retirement` event before any ticket reaches `complete`. The record must include terminal state, retirement reason, grounding, impact, verification, evidence, outcome classification, and governance.

A generic `outcome.recorded`, status flip, worker `DONE`, or `STATUS.md` projection is not retirement. If right-surface verification is unavailable, use `completed_but_unverified` and do not unlock dependents.

# Acceptance

- The retirement event precedes the completion transition in append order.
- Run-local/evaluation tickets are covered.
- The reviewed subject remains blocked when evidence is contradictory.
- The right-surface verification field is explicit.

# Evidence

- Failed pre-fix grading: `iteration-1/eval-2-evidence-cross-grill/with_skill/failed-run-1/grading.json`.
- Passing isolated candidate grade: `hyperskill-semantic-diffusion-prototype/retirement-candidate/.../grading.json`.
- Passing live regression grade: `iteration-1/eval-2-evidence-cross-grill/with_skill/run-1/grading.json`.

# Rollback

Restore the prior `SKILL.md` digest and retain the retirement ledger and evidence; never erase the failed run.

