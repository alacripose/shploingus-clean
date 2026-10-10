---
patch_id: PATCH-013
title: Hyperskill identity and non-conclusive description
status: designed
operation: replace
target: hyperskill/identity
base_ref: orchestrating-ticket-agents-internal
base_digest: sha256:f1fb8e4c20c4445f91a953035b2a924e9e3c26db3f1fe08778d1880b28086b8a
dependencies: [PATCH-001, PATCH-012]
source: user-clarification
created_at: 2026-09-24
approval: user-selected
live_mutation: false
---

# Change

Use the name `hyperskill` for the final public skill and repository. Describe it as an agent-facing candidate loop for self-improvement, evidence-backed documentation, delegation, and task planning.

The description must be exclusive to that role and non-conclusive: it must not promise universal acceleration, automatic authority, or guaranteed self-improvement. It must exclude ordinary single-agent work, lists, and summaries.

# Acceptance

- Frontmatter name is exactly `hyperskill`.
- Description starts with `Use when` and stays under 500 characters.
- Description describes triggering conditions, not internal workflow.
- The body remains usable when copied alone.
- The generated repository name is `hyperskill`.

# Evidence

- User selected `hyperskill` and requested a public repository under the authenticated account.
- `description-candidates/hyperskill-description.md` in the evaluation workspace.

# Rollback

Keep the prior public name and file as history; do not overwrite a user-modified copy without explicit reconciliation.
