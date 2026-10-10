---
patch_id: PATCH-009
title: Exact rollback evidence for improvement candidates
status: applied-internal
operation: strengthen
target: hyperskill/improvement-candidate
base_ref: orchestrating-ticket-agents-internal
base_digest: sha256:e08b1b6b0878755da0d0b5db07f099df4be29fbf0738404db40e3700b9953244
dependencies: [PATCH-001]
source: failed-eval
created_at: 2026-09-24
approval: applied-to-internal-base
live_mutation: false
---

# Change

An improvement candidate must record the exact base version, byte-level SHA-256, observation time, and immutable restore path. A boolean such as `rollback_present=true` is not rollback evidence. A missing or placeholder digest makes the candidate `not_evaluable` and blocks promotion.

# Acceptance

- The candidate record contains a real digest, not `recorded source digest` prose.
- The digest matches the exact base bytes at proposal time.
- The restore path exists and points to the prior artifact.
- Description-only presence testing remains separate from full-skill behavior.

# Evidence

- Failed final base eval-4 grading identified the missing concrete rollback reference.
- Targeted invariant rerun is the acceptance gate for this patch.

# Rollback

Restore the prior base digest and candidate snapshot; never delete the failed candidate record.
