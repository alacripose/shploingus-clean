---
patch_id: PATCH-012
title: Standalone public SKILL.md contract
status: designed
operation: add
target: hyperskill/publication
base_ref: orchestrating-ticket-agents-internal
base_digest: sha256:f1fb8e4c20c4445f91a953035b2a924e9e3c26db3f1fe08778d1880b28086b8a
dependencies: [PATCH-001, PATCH-002, PATCH-003, PATCH-004, PATCH-005, PATCH-006, PATCH-007, PATCH-008, PATCH-009, PATCH-010, PATCH-011]
source: user-clarification
created_at: 2026-09-24
approval: design-approved
live_mutation: false
---

# Change

The public skill artifact is a standalone `SKILL.md`. It must embed the complete core protocol and contain no required `references/`, `assets/`, local absolute paths, project directory, or hidden binding. The public repository may contain only that file and the requested MIT `LICENSE`.

The output must use conditional language: candidates are proposals, evidence may be incomplete, and no universal gain is implied. Users may modify their copy; the license must not require it to match anyone else's copy.

# Acceptance

- A standalone directory containing only `SKILL.md` passes a smoke verifier.
- No required external path appears in the output.
- Core protocol sections are embedded rather than delegated to missing files.
- The public export excludes indexes, manifests, snapshots, and source records.
- The license states that modified copies are allowed.

# Evidence

- User publication clarification.
- `docs/superpowers/specs/2026-09-24-hyperskill-semantic-diffusion-design.md`, standalone publication contract.

# Rollback

Retain the prior public `SKILL.md` and license reference; do not delete user-modified copies.
