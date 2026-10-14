# Phase 1 — Interface contracts: U3 world generation (the shapers)

**Feature**: `003-u3-shapers` | **Date**: 2026-10-11 | **Spec**: [spec.md](../spec.md) | **Plan**: [plan.md](../plan.md)

The clean room is a **library**: its contracts are the seams its callers use. U3 exposes four, and
each is stated as the *refusal* it must produce as well as the value it must return — because in this
project a seam that cannot refuse is not a seam (P2). No implementation is written in this pass.

## C1 — Admit a declared stage

```text
Declared::new(stages: Vec<Stage>) -> Result<Declared, RefusalSet>

Refusals (named, collected — not the first):
  - no_provenance        the stage names no paper and no ruling  (FR-003)
  - output_undeclared    the stage writes nothing                (P6: no placeholder)
  - reads_below          it consumes a stage declared below it   (FR-002)
  - duplicate_output     two stages declare the same output      (FR-002)
  - cadence_not_on_ladder its rung is outside the closed four-rung ladder
  - dial_unset           the law's dial is declared with no value and no default exists (D6)
```

**Invariant**: a `Declared` set that admitted successfully can be printed and re-validated — the
order is a value, not an accident of registration.

**Corrected at implementation (with reasons):**

- `Result<Declared, RefusalSet>` rather than `Result<Stage, Refusal>`: admission is over the **set** (a
  duplicate output is a fact about two stages, not one), and it collects every problem, so the type
  that carries the answer has to be the set.
- `no_source` is spelled **`no_provenance`**, and `reads_below`/`duplicate_output`/
  `cadence_not_on_ladder` are spelled `ReadsBelow`/`DuplicateOutput`/`CadenceOffLadder` in
  `src/refusal.rs`. The refusal vocabulary is closed and already records "this value has no source"
  as `no_provenance` for U2's materials (C-39); a second spelling for one fact is exactly the drift
  the closed vocabulary exists to prevent.
- `cadence_not_on_ladder` is **unrepresentable** in a `Stage`: `Cadence` is a closed four-variant enum,
  so there is no fifth rung to admit. The refusal lives where an outside number can still arrive —
  `Cadence::from_rung(n)` — and is tested there. A contract that names an impossible state is weaker
  than the type that makes it impossible.

## C2 — Drive a tick

```text
TickDriver::drive(&roster, identity, tick, workers) -> Result<TickReceipt, Refusal>

TickReceipt { tick, plan_digest, tally: Tally { due, ran, refused: [Refusal] } }

Refusals:
  - rewind               tick is earlier than the last driven tick  (FR-005)
  - cadence_off_ladder   a due stage's rung is not on the ladder
  - stage_input_absent   a due stage's declared input is unavailable — the stage refuses by name,
                         and the tick reports it rather than substituting a zero (FR-006)

Idempotence: drive(N) twice ⇒ the same receipt digest; the second drive is not a second effect.
```

**Corrected at implementation (with reasons):** the three counts are one `tally` field rather than
three flattened ones, and a stage that refuses does not fail the *tick*: the tick completes and the
tally names what refused (`refused: [Refusal]`, each carrying its kind's name). Failing the whole tick
on one body would stop the thirteen runnable stages, which is the opposite of SC-003's bar.

**Measured by**: `due` / `ran` / `refused` printed for tick 0, with refusal names — SC-003's floor is
`ran >= 14` of 16 as the legacy quoted it, which against this fifteen-stage roster with two deliberate
refusals reads `13 of 15` and is the measured line.

## C3 — Ask a field

```text
field_at(&roster, identity, tick, field: FieldId, request: ProcessRequest, workers) -> Result<Vec<f64>, Refusal>

  - unit request    for a field its declaring stage produces at a site
  - region request  for a field its declaring stage produces over a region

Refusals:
  - field_undeclared       no stage declares this field as an output (D5)
  - region_required        the declaring stage defines it only over a region (FR-008)
  - site_alone             a region-only law asked at a lone site — one answer, not a guess (FR-008)
  - dial_unset             the field's law has a dial with no value            (D6)
```

**Invariant**: the field's producer is the *declaring stage*; there is no second path that samples a
field from unrelated noise. A closed region's pairwise flux conserves material exactly (SC-005).

**Corrected at implementation (with reasons):** the seam takes the roster, the identity and the tick
because a field is a value *of a world at a tick*, and it returns the declared samples (`Vec<f64>`, the
one shape U3's fields have) rather than an opaque wrapper. The refusal of the declaring stage is
**passed through**, not replaced by a seam-level one, so an owed field names the field *it* is missing.**Sharpened while implementing the flux law (SC-005):** conservation is exact, and it is measured on
the **generated world**: `hillslope_divergence` moves `q = K·S/(1−(S/S_c)²)` from the higher column of
every unordered pair to the lower — the same amount out of one and into the other — so over a closed
region the field sums to zero (`cargo run` prints the sum and the magnitude: −8.674e−18 against
3.217). The first draft carried a standalone pair operator with a **half-the-difference** bound
(moving `x` from `a` to `b` closes their gap by `2x`, so more than half inverts the pair); it was
removed once the divergence became the measured path, because a second transport law with no consumer
is machinery this project does not keep. The bound's discovery is why the divergence was **not** built
on that operator.

## C4 — Digest a generated world

```text
world_generation_digest(seed, period, tick) -> Digest

Contract: the value depends only on (seed, period, tick) and the declared roster.
  - worker count changes it: NEVER (SC-001)
  - a moved tick changes it:  YES  (SC-002)
  - the anchor / residency cache of U2's `World` is not an input (it is a cache, not truth)
```

**Why a digest is the contract**: it is the one claim a later system can check without trusting a
narrative. The number is printed by the binary, so the check is a reading (P4).

## Deviations recorded while implementing this contract (not silently absorbed)

1. **`ecology` produces three readings from one pass, and its class comes from the climate alone.**
   The province is Holdridge's **humidity-province ladder** — `band = 1 + floor(log2(r /
   driest_ratio))` with `r = P / PET`, `PET = 58.93 · T_bio` — because the paper's ladder is geometric
   in the ratio, and `driest_ratio`/`biome_bands` are declared dials rather than a fitted offset. Soil
   is **not** folded into the ratio: `substrate`'s soil depth decides *establishment* (below a declared
   threshold the column carries the reserved bare-ground class `0`), because a multiplicative soil term
   let a steep hillside refuse a *climate* it can classify perfectly well — a real defect found by the
   reference run. Lieth 1975's Miami capacity in g DM m⁻² yr⁻¹ and Reynolds 1987's boids are now the
   declared fields `carrying_capacity` and `flock`, produced by the same stage: the capacity is the
   **minimum** of the two single-factor curves (temperature, precipitation) limited by a saturating soil
   factor, and the flock is the magnitude of the three weighted boid rules over the vegetated columns,
   read from the step's own noise rather than from stored state. `ECOLOGY_SOURCE` in `src/process.rs`
   cites Holdridge 1947, Lieth 1975, Reynolds 1987 and Heimsath et al. 1997.
2. **`src/cadence.rs` is not a file**: `Cadence` and the tick driver live in `src/stage.rs` and
   `src/process.rs`. The ladder is a property of a stage's declaration, and the driver plans the
   declared roster, so one file owns each fact rather than a third file owning half of both.
3. **The two record bodies landed, and their seam reads are what the contract now says.** `delta` reads
   `elevation`, `discharge`, `erosion_rate` and `water_level` — the load it deposits is the erosion rate
   accumulated over the **same** D8 graph the discharge is counted on, and the accommodation it fills is
   the depth lake fill found — so its declaration in `src/process.rs` is those four fields, not the
   `discharge`/`soil_depth` pair the first pass guessed at. `transition` reads `lithology` and `biome`,
   the pair whose **rounded classes** are its domains, and refuses a tie by name (`domain_tie`: two
   domains weighing the same is a boundary the world must decide, not the implementation). Its
   partition of unity is the crate's one Shepard interpolation (`terrain::shepard_weights`), so the
   property is stated once rather than re-derived.
4. **The reference declaration is a world, and a world is dials.** Making the laws *hold* on the
   reference patch was itself a declaration change, and it is recorded rather than tuned silently: the
   patch is 48 columns wide (a 32-column patch sits inside one F4 ladder cell, so `transition` would
   have no boundary to blend), its relief is 5 m and its plates carry **no** signed bias (a piecewise-
   constant plate bias is a scarp, and a soil law has no finite value on one), and its ladder rung is 0.
   The critical slope stays the physical `1.0`: the world moved, not the law. `check_u3_laws` in
   `src/main.rs` fails the binary if any law stops being visible in the reference world, so this cannot
   drift back without a measurement saying so.

## Non-contracts (explicitly out of scope)

- **No new dependency.** The primitives are arithmetic; `sha2` (already present) is the only hash.
- **No second stage list.** The roster lives in `Declared`; a stage list restated in the binary, in
  a client, or in a document is a defect (P5).
- **No legacy reference.** No `use` of, and no `path` dependency on, anything under `../crates/`
  (P7). The laws cite papers, not the legacy implementation.
- **No stored state per tick.** A per-tick accumulator outside `Earthworks`' record would be a
  second simulation (`docs/MISSING-SYSTEMS.md` §4.1's ruling).
