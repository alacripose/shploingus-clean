---
description: "Task list for U3 — World generation (the shapers)"
---

# Tasks: U3 — World generation (the shapers)

**Input**: Design documents from `/specs/003-u3-shapers/`

**Prerequisites**: [plan.md](./plan.md) (required), [spec.md](./spec.md) (required — three user
stories), [research.md](./research.md), [data-model.md](./data-model.md),
[contracts/u3-seams.md](./contracts/u3-seams.md), [quickstart.md](./quickstart.md)

**Tests**: included, because this project's rule is that a task is done when its check was run and
its output inspected (Constitution P4): each implementation task names the test that decides it, and
the tests are written first and must fail.

**Status**: **US1 and US2 are landed, and thirteen of the fifteen stage bodies with them.** One digest
for one `(seed, declared period, tick)`, unchanged at 1/2/4/8 workers and moved by the tick
(`460214b1b503d3e6334acabd73a6a654a110394df2de8235c750cf6a331b2fc6` at tick 0,
`7da363a83442494e85baec7864fa3eaddd273e6bc7d115ea943ff3d0d2d8cae5` at tick 1); the tick-0 tally is
**13 of 15 stages ran, 2 refused by name** — `caves` (`body_owed`, the dissolution field is not one of
the declared fields) and `earthworks` (`not_a_generator`, the record and not a generator) — as printed
by `cargo run`.

So **T013's floor is met as reconciled**: the legacy's `14 of 16` was the same rule (everything runs
but the deliberate refusals) against the legacy's own count, and this roster is closed at fifteen with
two deliberate refusals, so the bar reads `13 of 15` — which is the measured line. The laws the first
pass left owed inside landed stages are in too: Lieth 1975's Miami capacity and Reynolds 1987's flocking
(T020), Strahler 1952's stream order and Barnes et al. 2014's lake fill in `hydrology`, the
`hillslope_flux` law with a consumer in `erosion`, and both record bodies (T021). T023 and T024 stay
unchecked for the reason they name — the print gap, not a law. The boxes ticked below had their checks
run and their output inspected (P4).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependencies)
- **[Story]**: `US1` (determinism), `US2` (every runnable stage runs), `US3` (the declared graph)
- Every task names the file it touches and the check that decides it

## Path Conventions

Single project — the clean-room crate at the repository root's `clean-room/`: sources in `src/`,
tests inside each module's `#[cfg(test)]`, documents in `specs/003-u3-shapers/`.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: the vocabulary and provenance the rest of U3 stands on

- [x] T001 Add the named-refusal kinds U3 needs (`no_source`, `output_undeclared`, `reads_below`,
      `duplicate_output`, `cadence_not_on_ladder`, `dial_unset`, `rewind`, `stage_input_absent`,
      `field_undeclared`, `region_required`, `site_alone`) to the existing refusal vocabulary —
      `src/refusal.rs` · *check:* `cargo test --offline` still passes and each new kind is named in
      a test that produces it
- [x] T002 [P] Register the new modules on the library surface so the API is real and exercised —
      `src/lib.rs` · *check:* `cargo clippy --offline --all-targets -- -D warnings` is clean
- [x] T003 [P] Confirm the dependency posture before writing a line of U3: `sha2` stays the only
      dependency and no new crate is needed (the primitives are arithmetic) — `Cargo.toml`,
      `specs/003-u3-shapers/plan.md` · *check:* `cargo tree --offline` lists `sha2` and its
      transitive crates and nothing else

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: the primitives, the stage declaration and the cadence ladder — everything the three
user stories need before any one of them can be implemented

**⚠️ CRITICAL**: no user story can start until this phase is complete

- [x] T004 The noise primitives, each with its paper in the module header: gradient noise (Perlin
      1985/2002), simplex-style gradient (Perlin 2001), cellular (Worley 1996), fBm (Mandelbrot & Van
      Ness 1968), ridged multifractal (Musgrave, Kolb & Mace 1989), domain warp (Quilez 2002), and a
      coherent hash seeded by the splitmix64 finaliser — `src/noise.rs` · *check:*
      `the_primitives_are_pure_functions_of_seed_and_position` (same input twice ⇒ one value)
- [x] T005 `Stage` with its source, `reads`, `writes`, `cadence` and `dials`, and admission that
      refuses a stage with no source, no output, a dial with no value, or a rung off the ladder —
      `src/stage.rs` · *check:* `a_stage_without_a_source_is_refused_by_name`,
      `a_stage_that_declares_no_output_is_refused_by_name`,
      `a_stage_with_an_unset_dial_and_no_default_is_refused_by_name`
- [x] T006 `Declared` — the ordered stage set with its validation: a stage reading a stage declared
      below it refused by name, two stages declaring one output refused by name, and a printable
      order that re-validates — `src/stage.rs` · *check:*
      `a_stage_reading_below_it_is_refused_by_name`,
      `two_stages_declaring_one_output_are_refused_by_name`,
      `a_declared_set_round_trips_through_its_printable_order`
- [x] T007 The closed four-rung `Cadence` ladder and the tick driver with a digesting receipt:
      idempotent within a tick, a rewind refused by name — `src/cadence.rs` · *check:*
      `driving_a_tick_twice_produces_one_receipt_digest`,
      `a_rewind_is_refused_by_name`, `a_rung_off_the_ladder_is_refused_by_name` · **as built:** the
      driver is `process::TickDriver` (not `src/cadence.rs`) and the tests are
      `a_tick_is_idempotent_and_a_rewind_is_refused_by_name` and
      `a_rung_off_the_closed_ladder_is_refused_by_name`

**Checkpoint**: primitives, declaration and cadence exist — the three stories can now proceed in
parallel

---

## Phase 3: User Story 1 - The world is a function, and the function is named (Priority: P1) 🎯 MVP

**Goal**: one digest for one `(seed, declared period, tick)`, unchanged by worker count, changed by
the tick

**Independent Test**: digest the same inputs twice and at each of 1/2/4/8 workers; digest a moved
tick

### Tests for User Story 1 ⚠️

> Write these first, ensure they FAIL before implementing

- [x] T008 [P] [US1] Test: one digest across 1, 2, 4 and 8 workers, and a different digest at
      `tick + 1` — `src/process.rs` · *check:* the test prints both the shared digest and the moved
      one (SC-001, SC-002)
- [x] T009 [P] [US1] Test: `world_generation_digest` is a function of `(seed, period, tick)` and the
      roster only — a moved U2 anchor and a populated residency cache change it **not at all** —
      `src/process.rs` · *check:* the digest is printed with and without a moved anchor (FR-001)

### Implementation for User Story 1

- [x] T010 [US1] `world_generation_digest(seed, period, tick)` over the declared roster, folding each
      stage's contribution in the declared order through the one `Digest` — `src/process.rs` ·
      *check:* T008/T009 pass
- [x] T011 [US1] Worker-count independence: run the stages through the crate's worker knob and prove
      the digest is untouched (the knob is a performance concern and nothing else) — `src/process.rs`
      · *check:* T008 passes at all four worker counts
- [x] T012 [US1] The binary prints the U3 digest and the moved-tick digest and exits 0 — `src/main.rs`
      · *check:* `cargo run --offline` prints both hexes and exits 0 (SC-007)

**Checkpoint**: US1 is independently demonstrable — the world generation is re-derivable and auditable

---

## Phase 4: User Story 2 - Every stage that can run, runs; nothing silently does nothing (Priority: P2)

**Goal**: per-tick `due` / `ran` / `refused` with each refusal named, and at least **14 of 16** stages
running from tick 0

**Independent Test**: drive tick 0 over the declared roster and read the tally and the refusal names

### Tests for User Story 2 ⚠️

> Write these first, ensure they FAIL before implementing

- [x] T013 [P] [US2] Test: the tick-0 tally over the declared roster, printing `due`, `ran` and each
      refusal name, asserting the floor `ran >= 14 of 16` — `src/process.rs` · *check:* the tally and
      the refusal names are printed (SC-003) · **as built:** `the_tally_counts_what_ran_and_names_what_refused`
      asserts `ran == 13` and `refused == ["caves", "earthworks"]`, and `check_u3`'s `LANDED_STAGES`
      bar fails the binary if the count moves — the floor is met against the fifteen-stage roster as
      `13 of 15` (see the Status note above)
- [x] T014 [P] [US2] Test: a stage whose declared input is absent refuses by name and the tick
      reports it — never a zero, never a default — `src/process.rs` · *check:* the refusal names the
      stage and the input (FR-006)
- [x] T015 [P] [US2] Test: a region-only law asked at a lone site is refused by name, and a closed
      region's pairwise flux conserves material exactly — `src/process.rs` · *check:* the refusal name
      plus the conservation recount (FR-008, SC-005)

### Implementation for User Story 2

- [x] T016 [US2] `ProcessScheduler` and `ProcessRequest::{unit, region}`: the per-tick tally, the
      borrowed neighbourhood (the runtime never invents one), the refusal path — `src/process.rs` ·
      *check:* T013/T014 pass
- [x] T017 [P] [US2] The terrain group: `tectonics` (Kreischer & Bommes 2019), `fields` (Shepard
      1968; the F4 ladder ruling), `geology` (Allen & Allen 2013), `terrain` (Mandelbrot 1982;
      Musgrave 1989) — `src/shapers/terrain.rs` · *check:* each names its source and contributes to
      the digest (SC-004)
- [x] T018 [P] [US2] *(completed: the three halves of the routing graph are all landed — `d8_receivers` +
      `accumulate`, `strahler_order`, and Priority-Flood `lake_levels` as `water_level`; and
      `hillslope_flux` now has a consumer stage, `hillslope_divergence` over the closed region, so its
      claim rests on a generated world and not only on a unit test)*
      The water group: `hydrology` (O'Callaghan & Mark 1984; Strahler 1952; Barnes et
      al. 2014), `substrate` (Heimsath et al. 1997), `erosion` (Musgrave 1989; Hansson 2017; Mei      et al. 2007) with the stream-power law (Whipple & Tucker 1999; Lague 2014) and nonlinear hillslope
      diffusion (Roering et al. 1999) as **declared dials** — `src/shapers/water.rs` · *check:*
      a dial with no value for `K`, `m`, `n`, `S_c` · **as built:** the four dials (and the two the
      hillslope law needs) are declared in `src/process.rs` (`EROSION_DIALS`) and an unset one is
      refused at admission by `a_stage_with_an_unset_dial_and_no_default_is_refused_by_name`;
      `the_hillslope_law_refuses_at_the_critical_slope` and `the_stream_power_law_rises_with_area_and_with_slope`
      decide the laws themselves
- [x] T019 [P] [US2] The air group: `climate` (Whittaker 1960; Köppen 1884; ICAO 1976), `atmosphere`
      (US Standard Atmosphere 1976), `weather` (Smith & Barstad 2004) — `src/shapers/air.rs` ·
      *check:* each climate field is stage-coupled to terrain, refused if it is not (FR-007)
- [x] T020 [P] [US2] *(completed: `ecology` produces all three declared readings in one pass —
      Holdridge's province ladder, Lieth's Miami capacity as the minimum of the two single-factor
      curves limited by soil, and Reynolds' boids as the `flock` field — tested by
      `a_wetter_climate_carries_a_wetter_class_and_the_class_never_leaves_its_declared_set`,
      `miami_capacity_is_the_limited_curve_and_the_soil_limits_it`,
      `a_flock_needs_formation_to_carry_it_and_crowding_to_steer_it` and
      `ecology_produces_the_province_the_capacity_and_the_flock_in_one_pass`; `caves` still refuses
      by name)* The life group: `ecology`
      (Holdridge 1947; Lieth 1975; Reynolds 1987) and
      `caves` **refusing by name** — its occupancy field is not one of the declared fields, and the
      threshold and water table are the user's — `src/shapers/life.rs` · *check:*
      `caves_refuses_by_name_and_names_the_missing_field`
- [x] T021 [US2] *(completed: `delta` deposits the load a channel drops where the water stands, capped
      by the accommodation lake fill found, and `transition` blends the two domains a region carries as
      a partition of unity, refusing a tie by name; only the *avulsion* half of `delta` stays owed,
      because it needs a state the record has no home for — `docs/MISSING-SYSTEMS.md` §4.1)*
      The record group: `delta` (Gilbert 1885; Hjulström 1935 / Shields 1936; Slingerland
      & Smith 2004), `transition` (Shepard 1968; normalized sparse convolution over a jittered grid),
      and `earthworks` **bodyless by design** — the record, through U2's `Chain` — `src/shapers/record.rs`
      · *check:* `delta_deposits_at_the_mouth_and_only_where_water_stands`,
      `a_blend_is_a_partition_of_unity_that_crosses_the_boundary_without_a_step`,
      `a_blend_tie_refuses_by_name_rather_than_picking_a_domain`,
      `the_record_refuses_by_name_rather_than_returning_nothing`
- [x] T022 [US2] The field plane's one seam: `field_at` routes a declared field to the stage that
      declares it as an output, and refuses a field with no declaring stage by name — `src/field.rs` ·
      *check:* T019's coupling test passes and `a_field_no_stage_declares_is_refused_by_name` ·
      **as built:** the test is `a_field_no_stage_declares_is_refused_by_name`, beside
      `a_field_comes_from_the_stage_that_declares_it` and
      `an_owed_field_passes_the_declaring_stage_s_refusal_through` (the seam never replaces the
      declaring stage's refusal with its own)

**Checkpoint**: US2 is independently demonstrable — the roster's silence is impossible, and the bar
is a printed count

---

## Phase 5: User Story 3 - The stage graph is declared, ordered and inspectable (Priority: P3)

**Goal**: print the roster with each stage's source, outputs and cadence rung, and refuse a graph
that violates its order

**Independent Test**: print the declared set; check every stage's reads are declared above it

### Tests for User Story 3 ⚠️

- [ ] T023 [P] [US3] Test: the printed roster carries one source per stage, and a graph whose order
      violates its reads is refused by name — `src/stage.rs` · *check:* the printed roster (SC-004) ·
      **still owed, and named:** the order violation is refused and tested
      (`a_stage_reading_below_it_is_refused_by_name`), and the binary prints every stage's rung, reads
      and writes; the per-stage **source** is not in that print yet (it is in the declaration digest,
      not on the line)

### Implementation for User Story 3

- [ ] T024 [US3] `--roster`: print the declared stages with source, reads, writes and cadence rung —
      `src/main.rs` · *check:* T023 passes and the printed roster is the one the digest folds ·
      **owed:** the print exists unconditionally (the binary takes no arguments) and carries the rung,
      the reads and the writes; `--roster` as a flag, and the source on the line, are owed
- [x] T025 [US3] The unified-system registry records U3's real status, replacing `absent` — and only
      what was actually built — `src/systems.rs` · *check:*
      `the_registry_is_closed_and_ordered` requires a status per system

**Checkpoint**: all three stories are independently demonstrable

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: the project's own gates, and the record

- [x] T026 [P] Update `spec.md`'s status, paste each scenario's printed output into `quickstart.md`,
      and re-run `checklists/requirements.md`'s review — `specs/003-u3-shapers/` · *check:* no
      criterion remains `unchecked` without a pasted measurement
- [x] T027 Record the pass in `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §5/§7 and the U3 line of §2,
      naming the measured `ran` count and the digests — `../docs/` · *check:* the documented number
      equals the printed one
- [x] T028 **Constitution task:** re-run the checks behind every P1–P7 row of [plan.md](./plan.md) §3
      and record the output — *check:* `cargo test --offline` (count printed),
      `cargo clippy --offline --all-targets -- -D warnings` (clean), `cargo run --offline` (exit 0,
      U3 measurements printed), and a boundary check that nothing under `../crates/` was read or
      referenced · **run 2026-10-11 (latest pass):** `cargo test --offline` → **112 passed, 0 failed**;
      `cargo clippy --offline --all-targets -- -D warnings` → **exit 0, no output**;
      `cargo run --offline` → **exit 0** with the digests, the `13/15, 2 refused` tally and the law
      readings above (stream order 1–4, 275 columns holding water, deposition 0.136 m over 96 columns,
      blend 0.000–0.995, capacity to 165.1 g m⁻² yr⁻¹, flock to 0.546, hillslope flux cancelling to
      −8.7e−18);
      `cargo tree --offline` → `sha2` and its transitive crates, nothing else; `grep -rn "crates/"
      src/` → 14 hits, **all inside doc comments that name the boundary** (`must_not_copy` strings and
      module headers), zero `use` or `path =` references. No file under `crates/` was read or modified.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: no dependencies
- **Foundational (Phase 2)**: depends on Setup — **blocks all three stories**
- **US1 (Phase 3)**, **US2 (Phase 4)**, **US3 (Phase 5)**: each depends only on Foundational; they
  may run in parallel, though US3's printed roster is most useful once US2's roster exists
- **Polish (Phase 6)**: depends on the stories being complete

### Story Dependencies

- **US1 (P1)**: the MVP. It is the smallest slice that is still worth shipping: the world generation
  is a named, digestible function.
- **US2 (P2)**: independent of US1's digest path; it makes the roster's silence impossible.
- **US3 (P3)**: independent; it turns the declaration into a reading.

### Within Each Story

- Tests first, and they must fail before implementation
- Declaration before producers; producers before the seams that route to them
- A story is complete only when its check has been run and its output inspected (P4)

### Parallel Opportunities

- T002, T003 in Phase 1
- T008, T009 in US1; T013–T015 in US2; T023 in US3
- T017–T020 (the four stage groups) touch four different files and can be written in parallel
- T026 and T027 are independent of each other

---

## Parallel Example: User Story 2

```bash
# The four stage groups are four different files and can be written together:
Task: "terrain group in src/shapers/terrain.rs"
Task: "water group in src/shapers/water.rs"
Task: "air group in src/shapers/air.rs"
Task: "life group in src/shapers/life.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 only)

1. Phase 1 (Setup) → 2. Phase 2 (Foundational) → 3. Phase 3 (US1)
4. **STOP and VALIDATE**: the printed digest is the same at 1/2/4/8 workers and moves with the tick
5. That is already worth recording: world generation is a named function with one identity

### Incremental Delivery

1. US1 → a digestible world generator (MVP)
2. US2 → the roster runs, and its refusals are named (the `14 of 16` bar)
3. US3 → the declaration is printable and self-validating
4. Polish → the project's gates, the pasted measurements, the record

---

## Notes

- `[P]` tasks touch different files and have no incomplete dependency between them
- `[US#]` maps each task to the user story it serves, so a story can be validated alone
- `14 of 16` is a **floor** measured against the legacy tree's figure; it is not a target to reach by
  making a refusing stage emit a zero
- No task in this file was started in this pass: the pass created the specification set only
