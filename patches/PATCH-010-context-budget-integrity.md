---
patch_id: PATCH-010
title: Exact context-budget preservation
status: verified-internal
operation: strengthen
target: hyperskill/context-budget
base_ref: orchestrating-ticket-agents-internal
base_digest: sha256:e08b1b6b0878755da0d0b5db07f099df4be29fbf0738404db40e3700b9953244
dependencies: [PATCH-004]
source: failed-eval
created_at: 2026-09-24
approval: applied-to-internal-base
live_mutation: false
---

# Change

The filed contract is the context-budget authority. Frontier, wait, derived-status, and dispatch records must copy its component values exactly. If a value changes, file a versioned contract revision and repeat preflight before dispatch.

# Acceptance

- No derived record silently changes a filed budget.
- A changed budget has a new contract version and preflight event.
- Queued tickets retain their original budget until revision.
- Over-budget work is narrowed or refused, never silently truncated.

# Evidence

- Failed final base eval-7 grading found contradictory 3750/2250 versus 3850/2150 budget values.
- Targeted invariant rerun preserved the exact 6000-token budget across contracts, waits, frontiers, and derived records.

# Rollback

Restore the prior context policy and preserve the budget trace; do not reinterpret historical runs.
