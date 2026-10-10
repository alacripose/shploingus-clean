---
patch_id: PATCH-004
title: Adaptive context budgets
status: proposed
operation: add
target: hyperskill/memory-and-context
base_ref: orchestrating-ticket-agents-internal
base_digest: sha256:f1fb8e4c20c4445f91a953035b2a924e9e3c26db3f1fe08778d1880b28086b8a
dependencies: [PATCH-001, PATCH-002, PATCH-003]
source: user-approved-conversation
created_at: 2026-09-24
approval: design-approved
live_mutation: false
---

# Change

Require a declared context budget for every coordinator, worker, reviewer, and child lane. Calculate:

`required_context = selected skill slice + relevant source cards + relevant claim-map slice + relevant diagram source + ticket contract + required evidence + output reserve`

Dispatch only when the declared load is at most the measured lane window. If measurement is unavailable, use a conservative declared bound and retain the uncertainty.

# Acceptance

- The over-budget paper collection is narrowed or refused.
- Unselected papers remain file-backed.
- Frontier, wait, derived, and dispatch records copy the filed budget exactly.
- A changed budget requires a versioned contract revision and new preflight.
- No scalar memory or performance score is produced.

# Evidence

- Candidate fixture `paper-collection.json`.
- Candidate scientific-method specification, memory-and-context section.

# Rollback

Restore the prior context policy and retain the budget trace; do not silently reinterpret old runs under the new budget.
