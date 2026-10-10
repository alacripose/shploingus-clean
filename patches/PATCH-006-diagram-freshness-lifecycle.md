---
patch_id: PATCH-006
title: Diagram source-to-image freshness lifecycle
status: designed
operation: add
target: hyperskill/diagram-lifecycle
base_ref: orchestrating-ticket-agents-internal
base_digest: sha256:f1fb8e4c20c4445f91a953035b2a924e9e3c26db3f1fe08778d1880b28086b8a
dependencies: [PATCH-005]
source: user-feedback
created_at: 2026-09-24
approval: design-approved
live_mutation: false
---

# Change

Every produced diagram must follow source → render → inspect → manifest → freshness-check. The canonical source is the diagram JSON/Markdown; the image is derived evidence.

The manifest records source SHA-256, image SHA-256, renderer identity/version, generation time, inspection status, freshness status, and previous-manifest reference. A source or renderer change marks the old image stale and requires rerender and reinspection. Renderer failure records `blocked` or `not_inspected` and never claims visual completion.

# Acceptance

- A diagram-only source is not accepted as a completed visual artifact.
- Stale images are not silently reused.
- Prior image and manifest remain rollback evidence.
- A failed renderer produces a bounded non-conclusive result.
- The standalone verifier does not require the image or manifest at publication time.

# Evidence

- Candidate fixture `diagram-lifecycle.json`.
- Candidate scientific-method specification, diagram lifecycle section.
- Excalidraw rendering and visual inspection record from the candidate workspace.

# Rollback

Restore the prior diagram source, image, and manifest as a preserved candidate; never delete the stale image silently.
