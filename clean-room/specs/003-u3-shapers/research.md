# Phase 0 — Research: U3 world generation (the shapers)

**Feature**: `003-u3-shapers` | **Date**: 2026-10-11 | **Spec**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

**What this phase had to resolve.** The spec names behaviours; this file records the decisions that
had to be *made* before a design could be written, each with its rationale and the alternative that
was rejected. All decisions are re-derived from documents this repository already holds
(`docs/SHAPERS.md`, `docs/MISSING-SYSTEMS.md`, `docs/DESIGN-SPEC.md`) and from the papers those
documents cite — **no legacy source was read** (P7), and no new external source was needed.

## D1 — Where the laws come from, and what a "source" is

- **Decision**: a stage is admitted only with one named paper (or one named first-principles ruling)
  as its source. The roster and its citations are `docs/SHAPERS.md` §0 (the primitives) and §1 (the
  fifteen shapers), extended by `docs/MISSING-SYSTEMS.md` for the laws that document names as owed.
- **Rationale**: the repository's rule is that no shaper exists as "an unexplained pile of noise".
  A stage whose law has no citable source is a stage nobody can re-derive or audit.
- **Alternatives considered**: (a) naming a mod or an ecosystem reference — rejected, since the two
  affected shapers' intended references were refused on licence and the ruling is to build from
  first principles plus primary literature *outside* the ecosystem; (b) accepting a stage with a
  "TODO: find paper" note — rejected, that is exactly the placeholder P6 forbids.

## D2 — The determinism contract, and why the tick is not state

- **Decision**: a stage is a pure function of `(seed, declared period, tick)` and the stages above
  it. The tick is a **schedule**, never stored state: driving a tick is idempotent within that tick,
  a rewind is refused by name, and the tick's plan is digested as typed evidence.
- **Rationale**: the record's determinism claim (a ticked world and a rebuilt world produce one
  digest, whatever the worker count) is only checkable if the tick leaves no residue. A driver that
  remembered would be a second simulation.
- **Alternatives considered**: (a) mutating the world per tick and digesting the result — rejected,
  it makes "same inputs, same output" untestable in the presence of a schedule; (b) leaving the tick
  implicit in wall-clock time — rejected outright (no wall clock in the identity path).

## D3 — The two genuinely absent laws, and the bar that follows

- **Decision**: implement the stage graph so that at tick 0 the runtime reports the measured
  `due` / `ran` / `refused`; the stream-power law (Whipple & Tucker 1999; Lague 2014) and nonlinear
  hillslope diffusion (Roering et al. 1999) are the terrain terms that may still be unwired, and
  `caves` and `earthworks` refuse by name. The acceptance bar is therefore **at least 14 of 16**.
- **Rationale**: the legacy tree's own measured history is 1 → 11 → 14 of 16, and the plan adopts the
  figure as a **floor**. Reporting 16 by quietly returning zeros would be the failure mode this
  program exists to remove.
- **Alternatives considered**: (a) claiming all sixteen by letting refusing stages emit zeros —
  rejected: a zero that reads as a value is the exact defect P2 names; (b) deferring the runtime and
  shipping laws only — rejected: an unwired law is indistinguishable from a missing one, which is
  how the legacy tree ended up with an inert roster.

## D4 — Region-only laws: refuse at a site rather than guess

- **Decision**: a law definable only over a region (blend weights; pairwise hillslope flux) takes a
  region request and **refuses a lone-site request by name**. A closed region is conserved exactly:
  a pair exchanges equal and opposite material.
- **Rationale**: the alternative is guessing a neighbourhood, which makes a boundary seam a matter of
  luck, and conservation a claim instead of an invariant.
- **Alternatives considered**: (a) implementing a site fallback with a lerp — rejected in the source
  documents for producing grid-aligned creases *and* here for breaking the partition of unity;
  (b) storing a coarse blend field — rejected, that is a second source of truth for the same answer.

## D5 — Fields are stage-coupled, never independent noise

- **Decision**: the field plane is one seam (`field_at`) that routes a declared field to the stage
  whose declared outputs include it. A field with no declaring stage refuses by name.
- **Rationale**: "a climate field must be a function of the terrain above it, not a second unrelated
  fBm" — otherwise two systems answer one question and the wrong one is found in the world.
- **Alternatives considered**: (a) sampling each field from its own fBm at the call site — rejected
  for the reason above; (b) a single mega-function returning every field — rejected: it removes the
  declaration that makes "which stage produces this field" checkable.

## D6 — The dials belong to the user

- **Decision**: every law's coefficient, exponent and threshold (`K`, `m`, `n`, `S_c`, the karst
  threshold, the water table, the talus angle) is a **declared dial** with no invented default, and a
  stage asked to run with a dial unset refuses by name.
- **Rationale**: dials change what the world looks like, and what the world looks like is the user's
  call. The repository's rule is to name them rather than assume them.
- **Alternatives considered**: (a) publishing the legacy constants as defaults — rejected: they are
  the legacy tree's choices, and copying them would carry a decision from a source P7 forbids
  reading; (b) silently using the paper's example values — rejected, they are examples, not dials.

## Open questions carried into the next pass (named, not guessed)

- Whether the stream-power and nonlinear-diffusion **terrain terms** are wired through to elevation
  in slice 3 or deferred to slice 4's remesh pass — the plan's slice table names the *bar*, not the
  wiring order; this is a scoping decision for the implement pass, and it is recorded here rather
  than hidden.
- The exact tolerance for SC-005's region conservation recount is not yet stated: it must be an
  integer (or exact rational) claim over a closed region, not a floating-point "close enough".
