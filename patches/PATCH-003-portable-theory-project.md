---
patch_id: PATCH-003
title: Portable THEORY-PROJECT record
status: proposed
operation: add
target: hyperskill/theory-project
base_ref: orchestrating-ticket-agents-internal
base_digest: sha256:f1fb8e4c20c4445f91a953035b2a924e9e3c26db3f1fe08778d1880b28086b8a
dependencies: [PATCH-001, PATCH-002]
source: user-approved-conversation
created_at: 2026-09-24
approval: design-approved
live_mutation: false
---

# Change

Add a portable `THEORY-PROJECT` record that works without a repository or project directory. It must contain question, scope, hypothesis, predicted observation, claims, source references, experiments, documentation references, diagrams, open questions, updates, current outcome, retirement, and optional attachments.

The record may point to a project later, but it must not create a hidden binding or silently edit `CONTEXT.md`, ADRs, plans, or repository files.

# Acceptance

- A theory-only request succeeds with `project_directory: null`.
- Attachments are optional and explicit.
- Claim status distinguishes supported, uncertain, contradicted, and unsupported.
- The next scientific step is bounded and independently verifiable.
- Retirement preserves gain, regression, residual failure, and inconclusive outcomes.

# Evidence

- Candidate fixture `portable-project.json`.
- Candidate scientific-method specification, theory-project section.

# Rollback

Retain the prior standalone file and theory record; never delete the portable record merely because no repository is attached.
