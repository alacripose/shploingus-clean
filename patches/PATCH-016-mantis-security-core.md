---
patch_id: PATCH-016
title: Full Mantis security lifecycle and whole-dataset coverage
status: approved
operation: add
target: hyperskill/mantis-security-core
base_ref: orchestrating-ticket-agents-internal
base_digest: sha256:f1fb8e4c20c4445f91a953035b2a924e9e3c26db3f1fe08778d1880b28086b8a
dependencies: [PATCH-001, PATCH-002, PATCH-003, PATCH-004, PATCH-005, PATCH-006, PATCH-007, PATCH-008, PATCH-009, PATCH-010, PATCH-011, PATCH-012, PATCH-013, PATCH-014, PATCH-015]
source: user-approved-mantis-security-core
created_at: 2026-09-25
approval: user-confirmed
live_mutation: false
---

# Change

Make the complete Mantis security lifecycle a mandatory core protocol of the standalone `hyperskill` skill. The public artifact embeds the lifecycle and remains platform-agnostic; installed Mantis skills are optional local adapters, not runtime dependencies.

The ordered lifecycle is: architecture and knowledge base; threat model and trust boundaries; security history and lineage; governed campaign plan; evidence research; deduplication and classification; independent validation and review; isolated reproduction; exploit-chain analysis; minimal patch and rollback; risk calibration and report; retirement and reflection.

Every dataset item must receive a stage disposition and evidence locator. Missing adapters, host limitations, or incomplete evidence are `not_evaluable`, never silently treated as a pass. Security records preserve source links, claim/evidence maps, reproduction status, impact, patch and rollback identity, independent review, user adjudication, classification, and retirement.

The patch does not authorize live or public mutation, hosted or remote agents, universal security or improvement claims, or publication. The eventual public repository remains exactly `SKILL.md` and MIT `LICENSE`; internal evidence is not a runtime dependency.

# Safety boundary

Safety, privacy, evidence integrity, current-authority verification, destructive-action consent, retirement, dependency-unlock, and live/public mutation boundaries remain binding. Ordinary disputes are preserved for user adjudication. GOD_AGENTS.md is read-only reference material. Children return coordinator escalation packets; isolated self-review writes only inside the prototype.

# Acceptance

- The source record and public renderer contain all twelve lifecycle stages.
- Full-dataset coverage has no unexplained omissions.
- Security evidence records use `not_evaluable` for unavailable verification.
- Independent review, isolated reproduction, minimal patch/rollback, report, and retirement are explicit.
- The public output has no required local Mantis installation, path, reference, asset, repository, or service.
- The patch set, ledger, build identity, generated artifacts, and review evidence are updated together.

# Evidence

- User-approved design: `docs/superpowers/specs/2026-09-25-hyperskill-mantis-security-core-design.md`.
- User confirmation in the current conversation.
- Whole-dataset Mantis coverage and stage evidence remain in the isolated internal review workspace.

# Rollback

Retain prior patches, ledgers, source records, build identities, and generated artifacts. Reverting this patch must not erase questions, answers, challenges, supersessions, unresolved states, findings, coverage rows, or retirement evidence.
