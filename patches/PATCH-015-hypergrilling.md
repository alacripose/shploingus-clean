---
patch_id: PATCH-015
title: Hypergrilling user-control and self-review loop
status: approved
operation: add
target: hyperskill/hypergrilling
base_ref: orchestrating-ticket-agents-internal
base_digest: sha256:f1fb8e4c20c4445f91a953035b2a924e9e3c26db3f1fe08778d1880b28086b8a
dependencies: [PATCH-001, PATCH-012, PATCH-013, PATCH-014]
source: user-confirmed-hypergrilling
created_at: 2026-09-24
approval: user-confirmed
live_mutation: false
---

# Change

Embed a hypergrilling loop in the standalone `SKILL.md`. Hypergrilling is the user-control boundary for material decisions across all project work. It is not a request to expose private reasoning; it is a governed sequence of questions, answers, challenges, evidence, and bounded decisions.

The loop must:

1. identify material decisions using a fixed taxonomy: scope, authority, evidence, publication, destructive writes, dependencies, and unresolved ambiguity;
2. ask the user before a material decision or irreversible action, while allowing safe independent work to continue;
3. preserve unanswered questions and block only dependent work;
4. end each material round with `continue`, `one more round`, `pause`, or `stop`, and record the duration choice;
5. auto-close a round with no material decision;
6. convert child uncertainty into a coordinator escalation packet because children cannot prompt the user through the UI;
7. record every question, answer, challenge, supersession, and unresolved state in an internal append-only ledger;
8. allow conversational answers and explicit patch files as equivalent patch inputs;
9. treat a direct edit to generated `SKILL.md` as a proposed patch and block on divergence until reconciliation;
10. allow autonomous self-review to apply patches only in the isolated prototype, never to the live or public skill;
11. run self-review after each task against the full candidate, patch set, tests, evidence, and context behavior;
12. require user approval for writes outside the approved task/output boundary;
13. treat `high agency mode` as an explicit, logged, per-invocation mode that permits low-risk agent answers but is not silently inherited;
14. send an agent challenge to a fresh native third-party reviewer for an advisory review, after which the user decides;
15. keep the public artifact platform-agnostic, with optional adapters only.

The public `SKILL.md` contains the full protocol. The hypergrilling ledger, question records, reviewer reports, manifests, and snapshots remain internal. `GOD_AGENTS.md` is read-only reference material and cannot be challenged or overridden by this patch.

# Safety boundary

User authority is broad over project scope, patches, experiments, and publication. It cannot override higher-priority safety, privacy, evidence, retirement, destructive-action, or current-authority boundaries. A reviewer may challenge and escalate; no agent may silently rewrite a user decision or publish a candidate without the required decision.

# Acceptance

- All 14 existing patch records and this patch are included in the hypergrilling review set.
- A material uncertainty always creates a user-visible decision ticket.
- Safe independent work continues while a question is open.
- A child cannot ask the user directly; it returns a structured escalation packet.
- Every round has a recorded control state, and clean rounds may auto-close.
- Direct edits and patches both enter explicit reconciliation.
- Self-review can apply isolated prototype patches but cannot mutate the live/public skill.
- `high agency mode` is explicit, scoped, logged, and non-inheriting.
- Disputes use a fresh native advisory reviewer followed by a user decision.
- The ledger remains internal; the public repository contains only standalone `SKILL.md` and the requested MIT `LICENSE`.
- The final protocol uses non-conclusive language and does not promise universal improvement.

# Evidence

- `HYPERGRILLING-DECISION-RECORD.md` in this patch workspace.
- User confirmation in the current conversation.
- `GOD_AGENTS.md` read-only governance reference.
- OpenCode V2 agent documentation for fresh native reviewers and custom output bounds.

# Rollback

Retain the prior standalone `SKILL.md`, patch set, hypergrilling ledger, and decision record. Reverting this patch must not erase prior questions, answers, challenges, or unresolved states.
