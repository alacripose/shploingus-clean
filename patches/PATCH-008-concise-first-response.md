---
patch_id: PATCH-008
title: Concise first delegated response
status: verified-internal
operation: add
target: hyperskill/worker-output
base_ref: orchestrating-ticket-agents-internal
base_digest: sha256:77256380e6f950a5164581b07372423850b12887340871e32ae7d241c7a78ea8
dependencies: []
source: user-feedback
created_at: 2026-09-24
approval: applied-to-internal-base
live_mutation: false
---

# Change

Every child begins its visible response with exactly this compact envelope, capped at 120 words:

`STATUS`, one-line `SUMMARY`, exact `ARTIFACTS`, bounded `CONCERNS`, and one `NEXT` decision.

Internal planning, chain-of-thought, and progress narration do not belong in the parent UI. Detailed reasoning, tool traces, and evidence belong in assigned artifacts. A host that renders provider-internal reasoning is a separate client/provider concern and is not claimed to be suppressed.

# Acceptance

- The first visible response uses all five fields.
- The envelope is at most 120 words.
- Detailed evidence is written to the assigned artifact/transcript.
- A custom host profile may add a positive step cap; the built-in lane is not falsely described as capped.

# Evidence

- Isolated UX candidate test passed with the exact envelope.
- OpenCode V2 agent documentation confirms custom `system` and `steps` fields for bounded subagents.

# Rollback

Remove the envelope requirement while retaining artifact and evidence requirements; do not alter worker authority or ledger rules.

