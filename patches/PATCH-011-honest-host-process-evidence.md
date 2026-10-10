---
patch_id: PATCH-011
title: Honest host-process evidence
status: verified-internal
operation: strengthen
target: hyperskill/host-adapter
base_ref: orchestrating-ticket-agents-internal
base_digest: sha256:e08b1b6b0878755da0d0b5db07f099df4be29fbf0738404db40e3700b9953244
dependencies: []
source: failed-eval
created_at: 2026-09-24
approval: applied-to-internal-base
live_mutation: false
---

# Change

If the host cannot provide an independent dispatch/completion trace, record the process claim as `not_evaluable`. A self-authored zero counter is evidence about the report, not proof that a host event did or did not occur.

# Acceptance

- Native dispatch, forbidden dispatch, and completion claims identify their evidence source.
- Missing host traces produce neutral/not-evaluable language.
- No process assertion is marked passed solely from prose.

# Evidence

- Final base eval-7 grading identified self-authored process evidence as insufficient.
- Targeted invariant rerun recorded native process claims as `not_evaluable` when no independent trace existed.

# Rollback

Remove the stricter evidence classification only with a documented host trace; do not turn `not_evaluable` into a pass.
