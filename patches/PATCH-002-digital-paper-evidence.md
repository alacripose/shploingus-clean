---
patch_id: PATCH-002
title: Full digital-paper evidence
status: proposed
operation: add
target: hyperskill/paper-evidence
base_ref: orchestrating-ticket-agents-internal
base_digest: sha256:f1fb8e4c20c4445f91a953035b2a924e9e3c26db3f1fe08778d1880b28086b8a
dependencies: [PATCH-001]
source: user-approved-conversation
created_at: 2026-09-24
approval: design-approved
live_mutation: false
---

# Change

Require a source record for every digital paper, preprint, dataset, specification, or reproduction artifact. The record must include `source_id`, `artifact_kind`, `locator`, `retrieved_at`, `version`, `digest`, `access`, `method_summary`, `relevant_claims`, `limitations`, and `reproduction_status`.

Workers receive bounded source slices, not an unbounded paper collection. Metadata-only sources cannot support full-text claims.

# Acceptance

- Every material claim links to a source card and exact locator.
- Missing full text is recorded as `not_evaluable` or `not_applicable`.
- Missing digests are not invented.
- Limitations and reproduction status travel with the claim.
- Unselected papers remain file-backed and recoverable.

# Evidence

- Candidate scientific-method specification, paper-evidence section.
- Digital-paper fixtures `paper-a.md`, `paper-b.md`, and `metadata-only.md`.

# Rollback

Remove the branch from the generated candidate while retaining source records and prior generated digests.
