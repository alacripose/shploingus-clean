# Feature Specification: U3 — World generation (the shapers)

**Feature Branch**: `003-u3-shapers`

**Created**: 2026-10-11

**Status**: **US1 landed (2026-10-11), tests green.** The declared fifteen-stage roster, the noise
primitives, the cadence runtime, the `field_at` seam and **thirteen of the fifteen stage bodies** are
implemented with 113 passing tests and a clean clippy run; the tick-0 tally is **13 of 15 stages ran,
2 refused by name** — `caves` (the dissolution field it reads is not one of the declared fields) and
`earthworks` (the record, not a generator) — so every stage with a declared law runs and SC-003's
floor is met as reconciled below. `src/systems.rs` records U3 as *in progress* rather than `absent`.
Measurements are pasted in [quickstart.md](./quickstart.md) and §9 of
`docs/CLEAN-ROOM-PLAN-2026-10-11.md`.

**Input**: User description: "U3 world generation: the shapers — noise primitives and the fifteen terrain shaper stages plus the process/cadence runtime"

**Governed by**: `.specify/memory/constitution.md` (P1–P7, read live) and
`../docs/CLEAN-ROOM-PLAN-2026-10-11.md` §2 *U3 — World generation (the shapers)*, slice 3 of §5.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The world is a function, and the function is named (Priority: P1)

The world a player walks in must be *produced*, not *painted*. Given one identity and one tick, the
terrain, its materials and its shape are the output of a declared set of stages — each stage a
published law with one paper behind it — and running the same identity and tick twice produces the
same world byte for byte.

**Why this priority**: without this, every later system (mesh, light, stream, shell) has nothing
deterministic to stand on, and "the world changed between runs" is unattributable. This is slice 3's
whole point: a rebuild the record can be checked against.

**Independent Test**: digest one `(seed, declared period, tick)` through the declared stage set
twice — including once with a different worker count — and compare the two digests.

**Acceptance Scenarios**:

1. **Given** a declared stage set and one `(seed, declared period, tick)`, **When** it is run twice,
   **Then** the resulting world digest is identical, and worker count changes nothing.
2. **Given** the same seed and period at a different tick, **When** it is run, **Then** the digest
   differs — the clock is an input, not decoration.
3. **Given** any declared stage, **When** it is asked for its source, **Then** it names exactly one
   paper (or, for the two shapers with no citable external reference, names the first-principles
   ruling it was built under) — a stage that cannot name its source is refused, not shipped.

---

### User Story 2 - Every stage that can run, runs; nothing silently does nothing (Priority: P2)

A declared stage that produces nothing is indistinguishable from a stage that is broken. The
runtime must drive the stages by their declared cadence and report, per tick, exactly which stages
ran and which refused — by name, with the missing input named.

**Why this priority**: the legacy tree's measured failure was exactly this — 16 stages declared, **1**
running from tick 0 before the region form existed, then 11, then 14. The *number* is the acceptance
bar, and a stage quietly returning a zero is worse than a stage refusing.

**Independent Test**: at tick 0 over the declared roster, count the stages that ran and the stages
that refused, and read the refusal names.

**Acceptance Scenarios**:

1. **Given** the declared roster at tick 0, **When** a tick is driven, **Then** the runtime reports
   `due`, `ran` and `refused` counts, and the refused stages are named individually.
2. **Given** a stage whose required input is absent at a site, **When** it is asked about that site,
   **Then** it refuses by name rather than returning a zero, a default, or a plausible substitute.
3. **Given** the same tick driven twice, **When** the second drive happens, **Then** it is idempotent
   within the tick and a rewind is refused by name.
4. **Given** a request that can only be answered over a region (a blend, a pair of neighbours),
   **When** it is asked at a lone site, **Then** it refuses by name instead of guessing.

---

### User Story 3 - The stage graph is declared, ordered and inspectable (Priority: P3)

The stages are a graph, not a call chain: a stage reads only the stages above it and writes its own
declared outputs. The set and its order are values the program can print, so "which stages ran and
in what order" is a reading rather than a claim.

**Why this priority**: it makes the two stories above auditable (a stage reading an undeclared input
is a defect that can be *shown*), and it is what the field seam and the cadence runtime are built on.

**Independent Test**: print the declared stage set with each stage's declared outputs and its cadence
rung, and check that every stage a stage reads is declared above it.

**Acceptance Scenarios**:

1. **Given** the declared stage set, **When** it is printed, **Then** every stage appears once with
   its declared outputs and its cadence rung from the closed four-rung ladder.
2. **Given** a stage that reads a stage not declared above it, **When** the set is validated,
   **Then** it is refused by name.
3. **Given** a stage that claims a level or a cadence and declares nothing behind it, **When** it is
   admitted, **Then** it is refused by name (the rule U2's `ShaperSet` already enforces).

### Edge Cases

- A **missing declared layer** is a blocker, not an omission: the stage refuses, and the runtime
  names it in the tick's refusals.
- A **stage that cannot name its paper** is refused at admission (P1), and the refusal quotes the
  stage.
- **Fields that are independent noise instead of stage-coupled**: a field that is not a function of
  the stages above it is refused, so a climate field cannot be "a second unrelated fBm".
- **A region-only law asked at a site** (blend weights, pairwise hillslope flux) refuses by name.
- **Two stages that write the same output**: refused by name rather than resolved by order of
  appearance.
- **A cadence rung outside the closed ladder**: refused by name.
- **An unmeasured tick**: `unchecked`, never a pass (P4). A stage reported as running without a
  printed count is not evidence.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: World generation MUST be a pure function of `(seed, declared period, tick)` and the
  declared stage set. Two runs over the same inputs MUST produce byte-identical output, and worker
  count MUST be a performance knob that never changes output.
- **FR-002**: The declared stage set MUST be a **stage graph**: a stage reads only stages declared
  above it and writes its own declared outputs. Order MUST be a value the program can print.
- **FR-003**: The roster MUST declare exactly **fifteen** terrain shaper stages plus the noise
  primitives they are built from. Each stage MUST name exactly one paper (or, for the two shapers
  with no citable external reference, the first-principles ruling they were built under). A stage
  that cannot name its source MUST be refused at admission.
- **FR-004**: The runtime MUST drive stages by a declared **cadence** drawn from a closed ladder, and
  MUST report per tick how many stages were `due`, how many `ran`, and how many `refused`, naming
  each refusal.
- **FR-005**: Driving a tick MUST be **idempotent within that tick**, and a rewind (driving an
  earlier tick over a later one) MUST be refused by name.
- **FR-006**: An unavailable or unmeasurable input MUST be **refused by name**, collected rather than
  replaced — never clamped, never defaulted, never zero-that-reads-as-absent.
- **FR-007**: The **fields plane** MUST expose one seam (`field_at`) through which a declared field
  is produced by the stage that declares it as an output; a field with no producing stage MUST
  refuse by name rather than answer with independent noise.
- **FR-008**: A law that is only definable over a **region** (blend weights; pairwise hillslope flux)
  MUST refuse a lone-site request by name, and MUST conserve material within a closed region.
- **FR-009**: Nothing in this feature MUST import, `use`, or path-depend on any crate under
  `crates/` (P7). The shapers are re-derived from the documents and papers named below.
- **FR-010**: Every module MUST carry a header naming the document it was derived from, and every
  success criterion below MUST name the check that decides it (P1, P4).

### The shapers, and the one paper each (FR-003's roster)

The roster is closed at fifteen; a sixteenth is a decision for the record, not for a stage list.

| # | Stage | The paper (or ruling) it is built from | What it produces |
|---|---|---|---|
| 1 | `tectonics` | Kreischer, Bommes et al. *Procedural Tectonic Planets*, CGF 2019; Wilson-cycle plate-boundary taxonomy | a plate partition, per-boundary uplift/subsidence, a failure schedule |
| 2 | `fields` | Shepard 1968; the F4 ruling (the 20-level ladder) | the regional field ladder: lithology, elevation, soil depth |
| 3 | `geology` | Allen & Allen, *Basin Analysis*, Wiley 2013 | a stratigraphic layer stack per column; deposit dials |
| 4 | `terrain` | Mandelbrot 1982 (fractal landscapes); Musgrave, Kolb & Mace 1989 | elevation = fBm relief + tectonic uplift − smoothing |
| 5 | `hydrology` | O'Callaghan & Mark 1984 (D8); Strahler 1952; Barnes, Lehman & Mulla 2014 (Priority-Flood) | flow routing, stream order, lake fill, channel velocity |
| 6 | `substrate` | Heimsath, Dietrich, Nishiizumi & Finkel 1997 | soil depth from slope and curvature |
| 7 | `erosion` | Musgrave 1989 (thermal); Hansson 2017 (droplet); Mei, Decaudin & Hu 2007 | sediment transport and deposition |
| 8 | `climate` | Whittaker 1960; Köppen 1884; ICAO/US Standard Atmosphere 1976 | temperature and moisture as processes, never a stored field |
| 9 | `atmosphere` | US Standard Atmosphere 1976; the barometric formula | pressure and density by altitude |
| 10 | `weather` | Smith & Barstad, *A Linear Theory of Orographic Precipitation*, JAS 2004 | orographic precipitation, moisture advection, return periods |
| 11 | `caves` | Ford & Williams, *Karst Hydrogeology and Geomorphology*, Wiley 2007; Nie et al. 2024 | karst dissolution, guided by hydrology's flow |
| 12 | `ecology` | Holdridge 1947; Lieth 1975 (the Miami model); Reynolds 1987 (boids) | a biome triple per column; flocking as an agent behaviour |
| 13 | `delta` | Gilbert 1885; Hjulström 1935 / Shields 1936; Slingerland & Smith 2004 | deposition at river mouths; avulsion |
| 14 | `transition` | Shepard 1968; normalized sparse convolution over a jittered grid (NoisePosti.ng 2021) | a partition of unity so two domains meet without a seam |
| 15 | `earthworks` | the F10 ruling — not a paper: a **conservation ledger** | recorded modification of the world, each entry carrying its mass ledger |

**The two laws named as genuinely absent** (their absence is why U3's bar is *14 of 16* and not 16):

- **Fluvial incision — the stream-power law.** `E = K·A^m·S^n`. Whipple & Tucker, *JGR* 104(B8) 1999;
  Lague, *ESPL* 39, 2014. `m/n ≈ 0.5` reproduces the observed slope–area relation; `K`, `m`, `n` are
  **declared dials**, never invented constants.
- **Hillslope transport — nonlinear diffusion.** `q = K·S / (1 − (S/S_c)²)`. Roering, Kirchner &
  Dietrich, *WRR* 35(3), 1999; composes with the already-cited Heimsath et al. 1997 soil-production
  function. A closed region MUST conserve material exactly (FR-008).

**The two deliberate refusals**, named rather than stubbed:

- `caves` — its occupancy field is **not** one of the declared fields; the answer (open / flooded /
  rock) is a closed vocabulary whose field is owed by name. The threshold and the water table are
  world-shaping and are **the user's**.
- `earthworks` — bodyless **by design**: it is the record, and a recipe that is a mass pump must
  declare itself one. A stage that generated terrain would be the wrong shape of system.

### Key Entities

- **`Declared`** — the world's declared stage set: the ordered stages, what each reads and writes,
  and the cadence rung each runs on. The value that makes "which stages ran" a reading.
- **`Stage`** — one declared producer: its name, its source (the paper or ruling it is built from),
  its required inputs, its declared outputs, and its cadence rung.
- **`Cadence`** — the closed four-rung ladder a stage's schedule is drawn from.
- **`ProcessScheduler`** — the runtime that takes a tick and asks the due stages for their units.
- **`TickDriver`** — the driver that plans a tick, is idempotent within it, refuses a rewind, and
  digests its plan as typed evidence that the tick happened once.
- **`field_at`** — the one seam through which a declared field is produced by its declaring stage.
- **`Region` / `ProcessRequest::unit | ::region`** — the region form: a site alone cannot evaluate a
  blend or a pairwise flux, and says so.
- **`ShaperKind` × `Level`** (already built in U2) — the two kinds × two levels a shaper's effects
  must fit; a shaper that claims the addressed level and declares nothing is refused by name.
- **`Digest`** — the one content identity: one digest for one `(seed, tick)`.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001** One `(seed, declared period, tick)` produces one digest, and the same digest is produced
  across 1, 2, 4 and 8 workers. *Decided by:* a printed digest over the declared roster at a declared
  tick, run at each worker count.
- **SC-002** The same seed at a different tick produces a different digest. *Decided by:* the same
  printed digest at `tick` and `tick + 1`.
- **SC-003** At tick 0, at least **14 of the 16** declared stages run, and each of the remainder
  **refuses by name** with the input it is missing. *Decided by:* the runtime's per-tick
  `due` / `ran` / `refused` line, with the refusal names printed. (The bar is the legacy tree's
  measured figure; it is a *floor*, not a target.)
  **Denominator reconciled at implementation:** this roster is **closed at fifteen** (the roster table
  above, and FR-003), and **two** of the fifteen refuse by *decision* rather than by debt (`caves`
  reads a dissolution field the declaration does not carry; `earthworks` is the record, not a
  generator). The legacy's `14 of 16` was the same rule — everything runs but the deliberate refusals —
  quoted against the legacy's own count, so against this roster's fifteen it reads **13 of 15**, which
  is the measured tick-0 line: `13/15 stages ran, 2 refused by name`. The "of 16" is kept because it is
  the legacy number the bar is quoted against; it is **not** a second stage count, and a sixteenth
  stage is a decision for the record (`docs/CLEAN-ROOM-PLAN-2026-10-11.md` §2).
- **SC-004** Every one of the fifteen shaper stages names exactly one paper or ruling, and a stage
  built with none is refused at admission. *Decided by:* a printed roster check over the declared set.
- **SC-005** Asking a region-only law at a lone site refuses by name, and a closed region conserves
  material to within a stated tolerance. *Decided by:* the refusal's name, plus a conservation
  recount over a closed region.
- **SC-006** A tick driven twice is idempotent, and a rewind is refused by name. *Decided by:* the
  tick receipt, with its digest and its single-drive evidence.
- **SC-007** `cargo test --offline` passes, `cargo clippy --offline --all-targets -- -D warnings` is
  clean, and the binary prints the U3 measurements and exits 0.

## Assumptions

- **The papers are the specification of the laws; this project is the specification of the wiring.**
  Where a paper states a law with a dial (an exponent, a coefficient, a threshold), the dial is
  **declared** here and its value is the user's, named rather than assumed.
- **What the world looks like is the user's.** The look of the terrain is not decided by this
  feature; the mechanism, the sources and the refusals are.
- **The two shapers with no citable external reference** (`fields`, `geology`'s deposit dials) are
  built from first principles plus primary literature *outside* the mod ecosystem — their intended
  sources were refused on licence, and that refusal is recorded rather than worked around.
- **U2's nouns are the vocabulary this feature speaks**: signed coordinates, the subvoxel/detail
  lattices, `ShaperKind × Level`, `Refusal`/`RefusalSet`, and the chain. U3 declares stages; it does
  not redefine those nouns.
- **Nothing in U3 is built in this pass.** This document set is the specification the next pass
  implements, and every success criterion above is `unchecked` until it is run.
