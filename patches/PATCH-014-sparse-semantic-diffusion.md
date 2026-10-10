---
patch_id: PATCH-014
title: Sparse semantic diffusion compiler
status: designed
operation: add
target: hyperskill/semantic-diffusion
base_ref: orchestrating-ticket-agents-internal
base_digest: sha256:f1fb8e4c20c4445f91a953035b2a924e9e3c26db3f1fe08778d1880b28086b8a
dependencies: [PATCH-001, PATCH-004, PATCH-012, PATCH-013]
source: user-approved-design
created_at: 2026-09-24
approval: design-approved
live_mutation: false
---

# Change

Use a standalone Rust prototype to apply reviewed Markdown/YAML patches to a pinned base `SKILL.md`. Model requirements as a sparse semantic tree. Use SHA-256 source, parent, dependency, neighbor, compiler, and output identities. Invalidate only affected branches. Select root, target path, dependencies, and bounded neighbors under a declared budget. Generate a standalone `SKILL.md` for publication.

The sparse index, manifests, and history remain internal. The 3D design package contributes the octree, chunk digest, streaming, and cache patterns; the prototype does not import the game crate or claim GPU inference.

Direct edits to a generated copy are detected and preserved. Reconciliation requires an explicit patch; the tool never silently overwrites a user's modified copy.

# Acceptance

- A new requirement updates the generated standalone file.
- An unrelated requirement is omitted from a focused context selection.
- A changed digest invalidates only the affected closure.
- A budget overflow blocks or narrows explicitly.
- The final export contains only `SKILL.md` plus the MIT license.
- The Rust tests and clippy pass without Python or game dependencies.

# Evidence

- `docs/superpowers/specs/2026-09-24-hyperskill-semantic-diffusion-design.md`.
- `docs/superpowers/plans/2026-09-24-hyperskill-semantic-diffusion-prototype.md`.
- Ala-Cities patterns: `src/octree.rs`, `src/asset_streaming.rs`, and `src/asset_generator.rs`.

# Rollback

Retain the prior generated candidate and digest. A failed or stale build never replaces the last known-good standalone file.
