# Constitution — the shploingus clean room

**Version:** 2.0.0 · **Ratified:** 2026-10-11 · **Last amended:** 2026-10-13

> This is the project-level authority for the clean room at `clean-room/`. It is **written once** and
> read live by every `plan.md` and `tasks.md`. It does not describe a feature; it states the rules
> every feature must satisfy. Its upstream shape is GitHub spec-kit's constitution; its content is
> this repository's own.

## The two sentences everything rests on

1. **Refuse, never default.** A value that cannot be validated is refused and named, never replaced
   by a plausible substitute. A plausible substitute is indistinguishable from a correct one until
   much later — and by then it has been written into a world.
2. **The clean room is new code beside the old.** The legacy tree (`crates/`) is a behavioural
   witness. Its tests and its measured numbers say what the contract is; its source is not read while
   writing clean-room code.

---

## Core principles

### P1 — Fresh code, with declared provenance

Every clean-room module header names the **document** it was derived from (a spec in `docs/`, a
paper, a declared format). A module that cannot name its source does not belong here. Provenance is
per-module and checkable; "it looks right" is not a source.

### P2 — Refuse, never default

Absent, invalid or unmeasurable input is **refused by name**, and every refusal is collected rather
than the first. No clamping, no grey defaults, no zero that a caller could believe. This is the
repository's one rule, inherited whole, and it binds the clean room as it binds the legacy tree.

### P3 — Determinism is a contract

The world is a pure function of `(seed, tick, declared period)`. Two runs over the same inputs
produce byte-identical output, and that is a test, not an aspiration. Worker count is a performance
knob and may never change output.

### P4 — Evidence over assertion

Every claim a spec makes about user value is paired with a **measurable success criterion**. A cell
nobody measured is `unchecked`, and an unchecked cell is never a pass. A slice is done when its check
has been run and its output inspected — not when its code compiles.

### P5 — One declared interface, two doors

The API surface is **declared once** and served through one seam, so a player's mod and an agent
reach the game through the same door. A second decision matrix, a second permission model, or a
verb spelled in two places is refused by name. A served action is permitted only when the checks that
gate it **actually ran**, and the receipt says which.

### P6 — Smallest sufficient artifact

Prefer a small number of large, well-named files over many stubs. **Nothing is a placeholder without a
stated purpose.** A file that exists only to reserve a name is deleted or given a purpose. "Simpler"
is not used to re-open a settled decision or to shrink a requested system into a demo.

### P7 — The legacy tree is a witness **and** an architectural source; a port arrives under the convention

The clean room may read every document, paper, declared format **and implementation file** in the
repository. That is the user's direction of 2026-10-13, verbatim: the branch's goal is *"not
reinventing every single crate but porting the good code, and making it all contiguous"*, and *"the
regenerated crates must still follow the clean room convention to avoid rendering bugs from the
parent project"*. What that obliges, and what it retires:

* **A port declares its `path` dependency** on the parent crate it replaces, and **pins the parent
  bytes it read by `path + digest`** — "this came from that revision of that file" is a checkable
  claim rather than a memory. The old clause *"no `path` dependency on a legacy crate"* is **retired**:
  a port is exactly such a dependency, and what makes it safe is the convention below, not the ban.
* **Nothing is transcribed from prose in place of a source.** A legacy *format* that is the contract
  is still cited by `path + digest`, not carried, and generated legacy files are still re-generated
  from sourced tables rather than copied.
* **Every ported seam lands under the clean room's convention** — the five conditions
  `patches/PATCH-041-the-reconciliation-with-the-parent-branch.md` states: every printed figure
  measured by the running product; every reachable boundary refused by name with a non-zero status;
  no line that can read false while the run exits 0; guards proven to bite by perturbing their own
  subject; source bytes digested with captures standing as dated baselines.
* **Nothing is deleted before the port that replaces it passes its gate**, and the record layer
  (`evidence.rs`, the baselines, the ledger) is not a retirement candidate — it is what the port goes
  *into*.

Read the parent freely; port deliberately; land nothing that fails the convention.

---

## Governance

- **Supremacy.** This constitution outranks any single spec or plan. Where a spec conflicts with it,
  the spec is wrong — or the conflict is recorded under *Complexity Tracking* in that spec's
  `plan.md` with a stated reason, and the plan's **Constitution Check** fails loudly until it is.
- **Amendment.** Amending a principle requires a stated reason, a version bump and an entry in the
  amendment log below. Principles are not edited silently to make a plan pass.
- **Versioning.** `MAJOR` for a removed or redefined principle; `MINOR` for a new principle or a
  materially extended one; `PATCH` for a wording fix that changes no obligation.
- **Compliance review.** Every `plan.md` carries a **Constitution Check** table with one row per
  principle. Every `tasks.md` ends with a **Constitution task** that re-runs the applicable checks.
  An agent may not mark a task complete whose check was not run.

## Amendment log

| Version | Date | Change | Reason |
|---|---|---|---|
| 1.0.0 | 2026-10-11 | ratified | the clean-room program of `docs/PLAN-2026-10-10-clean-room-and-the-api-layer.md`, scaffolded by `docs/CLEAN-ROOM-PLAN-2026-10-11.md` |
| 2.0.0 | 2026-10-13 | **P7 redefined**: the parent is a witness *and* a source; a port declares its `path` dependency and pins the parent bytes it read by `path + digest`; every ported seam lands under the clean-room convention | the user's direction of 2026-10-13 (`patches/PATCH-041-the-reconciliation-with-the-parent-branch.md`): the branch's goal was porting the good code, and the reading ban this principle was enforced with came from an instruction file (`clean-room/.github/copilot-instructions.md`) that was **stricter than this constitution** — it forbade *reading*, where P7 forbade *copying* |

**Version**: 2.0.0 | **Ratified**: 2026-10-11 | **Last Amended**: 2026-10-13
