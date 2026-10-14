---
description: "Task list for U4 — meshing, streaming and work"
---

# Tasks: U4 — Meshing, streaming and work

**Input**: Design documents from `/specs/004-u4-mesh-stream-work/`

**Prerequisites**: [plan.md](./plan.md) (required), [spec.md](./spec.md) (required — three user
stories), [data-model.md](./data-model.md), [contracts/u4-seams.md](./contracts/u4-seams.md),
[quickstart.md](./quickstart.md)

**Tests**: included, because this project's rule is that a task is done when its check was run and its
output inspected (Constitution P4): each implementation task names the check that decides it.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependencies)
- **[Story]**: `US1` (the geometry the fields imply), `US2` (residency coherence), `US3` (work, budget,
  distribution)
- Every task names the file it touches and the check that decides it

## Path Conventions

Single project — the clean-room crate at `clean-room/`: sources in `src/`, module tests in
`#[cfg(test)]`, the end-to-end check in `tests/`, documents in `specs/004-u4-mesh-stream-work/`.

---

**Status 2026-10-11: T001–T019 and T021 are done and their checks were run**; the printed output is
pasted in [quickstart.md](./quickstart.md). Two decisions changed during the pass and are recorded in
[research.md](./research.md): the resident set is consulted **before** the residency horizon (D5, found
by the end-to-end check contradicting US2 AC3), and the ceiling and the worker count travel together as
`Effort` (D8, forced by clippy's argument count rather than silenced).

## Phase 1: Setup

- [x] T001 Add U4's refusal kinds to the existing closed vocabulary — `level_off_ladder`,
      `geometry_budget_exceeded`, `geometry_out_of_residency`, `horizon_mixed` — `src/refusal.rs` ·
      *check:* ran — each kind is asserted in the test that produces it
      (`a_level_past_the_ladder_is_refused_by_name`,
      `a_budget_that_cannot_be_met_is_refused_with_its_numbers`,
      `a_chunk_outside_the_residency_horizon_is_refused`,
      `a_render_horizon_wider_than_residency_is_refused`), and the binary prints all four names
- [x] T002 [P] Register the new module on the library surface — `src/lib.rs` · *check:* ran —
      `cargo clippy --offline --all-targets -- -D warnings` is clean

## Phase 2: Foundational

- [x] T003 The level ladder and the horizons as **types** — `Level` (0..=MAX_LEVEL, past it refused),
      `Solid`/`Water` layers, and `RenderHorizon`/`SimulationHorizon`/`ResidencyHorizon`/`FieldHorizon`
      as four distinct newtypes with the one derived width check — `src/mesh.rs` · *check:* ran —
      `a_level_past_the_ladder_is_refused_by_name`, `a_render_horizon_wider_than_residency_is_refused`
- [x] T004 [P] `GeometryKey` and the mesh identity — `(truth, chunk, level)` folded through U2's
      `Digest`, with the quads folded in a declared order — `src/mesh.rs` · *check:* ran —
      `one_key_one_digest_and_a_changed_quad_changes_it` (one digest for one key at any worker count;
      a later tick and a coarser level each move it)

## Phase 3: US1 — the geometry a world's fields imply

- [x] T005 The derivation: read `elevation`, `water_level` and `lithology` through the field plane's
      seam, turn columns into cells (`round` to blocks, flooded = water above the ground, material
      admitted or refused), and merge greedily right-then-down — `src/mesh.rs` · *check:* ran —
      `a_flat_uniform_chunk_is_one_quad`, `a_stepped_surface_is_one_quad_per_step`,
      `two_cells_merge_only_when_layer_height_and_material_agree`,
      `merge_tiles_the_mask_exactly_whatever_the_pattern`
- [x] T006 The refusal paths of the derivation: no field producer, a stage's own refusal passed through,
      an unadmitted material class, a non-finite field value — `src/mesh.rs` · *check:* ran —
      `a_mesh_never_invents_a_material`, `a_stages_refusal_passes_through_the_mesh_seam` (which also
      covers a `NaN` elevation and asserts the stage's own reason survives)
- [x] T007 [P] [US1] The block-level triangle arithmetic: `vertices = 4 × quads`, `indices = 6 × quads`
      (C-43's unit story, stated once) — `src/mesh.rs` · *check:* ran — `six_indices_per_quad_exactly`

## Phase 4: US2 — resident geometry stays coherent with its world

- [x] T008 `Resident`: serve only a current-truth entry, rebuild a stale one, never serve stale —
      `src/mesh.rs` · *check:* ran — `a_stale_entry_is_rebuilt_and_never_served` (the stale entry is
      unreachable by key, so "never served" is a property rather than a check)
- [x] T009 `invalidate(&world)` removes **exactly** the entries derived from a truth that is no longer
      current, and keeps the current ones — `src/mesh.rs` · *check:* ran —
      `invalidation_removes_exactly_the_stale_entries` (3 removed of 4, the current one kept, a second
      call removing 0)
- [x] T010 U2's rule held by U4: the anchor and the residency cache are not geometry inputs —
      `src/mesh.rs` · *check:* ran — `the_anchor_is_not_a_geometry_input`, and the reference end-to-end
      check repeats it on a real world; **D5** records the ordering this forced
- [x] T011 A chunk outside the residency horizon is refused by name and caches nothing — `src/mesh.rs` ·
      *check:* ran — `a_chunk_outside_the_residency_horizon_is_refused` (a single ask and a batch, each
      keeping the count of refusals)

## Phase 5: US3 — the work is counted, budgeted and distributed

- [x] T012 `WorkLedger` counted where the work happens, with `columns_produced` and `samples_read`
      reported separately — `src/mesh.rs` · *check:* ran — `the_ledger_equals_an_independent_recount`,
      `a_coarse_level_reads_fewer_samples_than_were_produced` (`64 of 1024`); the reference end-to-end
      check recounts the real world's mesh against the ledger
- [x] T013 `Budget`: exceeding it is refused by name with both numbers, and caches nothing —
      `src/mesh.rs` · *check:* ran — `a_budget_that_cannot_be_met_is_refused_with_its_numbers` (the
      column ceiling before the fields are read, the quad ceiling after the merge, with the ledger
      keeping the work the refusal measured), plus a fitting budget as a met one
- [x] T014 `derive_many` across `workers`, in declared order, with one refusal reporting by name —
      `src/mesh.rs` · *check:* ran — `the_worker_count_changes_no_quad_and_no_digest` (1/2/4/8 workers
      on one chunk; 1 and 4 workers on a three-chunk batch, and a warm batch of three hits)
- [x] T015 [P] [US3] Per-chunk accounting printed: the merge factor, the counts and the cells —
      `src/main.rs` · *check:* ran — the printed `1024 columns → 144 quads (1031 cells, merge factor
      7.16)` line equals the ledger's own, and the recount line re-derives it from the mesh ·
      **(§39, 2026-10-13):** the printed line is now `1024 columns read (1024 samples) → 11054 cells
      swept → 3703 faces → 3297 merged away → 406 quads (259 merged, factor 9.12)`, and the recount
      line is `U4 face recount:` over the same cells · **(§40, 2026-10-13):** the line is now `footprint
      32×32 + ring 1 (1156 produced, 1024 samples read), span 64..96, halo owned = true; 11054 cells swept
      → 1289 faces → 313 quads (factor 4.12)`, with the cells' own door beside it (`2023 faces → 327 quads`),
      the doors' agreement over one world (`U4 one world, two doors: … = true`), the three-world extent
      (`2679` / `1289` / `2023`) and the two-chunk rectangle (`8013` faces → `1227` quads, once)

## Phase 6: Polish

- [x] T016 The end-to-end check through the **public API** — `tests/u4_geometry.rs` · *check:* ran —
      two tests drive derive → repeat at 2/4/8 workers → coarse level → budget refusal → residency
      refusal on the **reference world**, and derive → hit → anchor move (still a hit) → tick advance →
      miss → rebuild → exact invalidation, asserting each transition and each digest
- [x] T017 [P] `check_u4` in the binary: the digests at 1/2/4/8 workers, the merge factor, the hit/miss
      and invalidation counts, the refusals — with every line naming its seed, period, tick, chunk and
      level (C-46) — `src/main.rs` · *check:* ran — `cargo run --offline` prints them and exits 0
- [x] T018 `src/systems.rs`: U4's real status, replacing `absent` — and only what was built ·
      *check:* ran — `the_registry_is_closed_and_ordered` still holds, and the status names the four
      owed capabilities
- [x] T019 [P] `quickstart.md`: paste the measured output; tick `tasks.md` only where the check ran ·
      *check:* every tick above has a printed line beside it in [quickstart.md](./quickstart.md)
- [x] T020 The governing plan's slice-4 row and a new section recording the pass, the dials and what U4
      still owes — `../docs/CLEAN-ROOM-PLAN-2026-10-11.md` · *check:* the documented numbers equal the
      printed ones (2026-10-11: `1024 → 144`, merge factor `7.16`, `64 of 1024`, `removed 1 of 2`;
      §39, 2026-10-13: `1024 → 3703 faces → 406 quads`, factor `9.12`, `64 of 1024`, `removed 1 of 2`;
      §40, 2026-10-13: `11054 → 1289 faces → 313 quads`, factor `4.12`, `64 of 1156`, `removed 1 of 2`)
- [x] T021 **Constitution task:** re-run the checks behind every P1–P7 row of [plan.md](./plan.md) ·
      *check:* ran — `cargo test --offline` **132 lib + 2 end-to-end passed, 0 failed**; `cargo clippy
      --offline --all-targets -- -D warnings` clean; `cargo run --offline` exit 0; `cargo tree
      --offline` shows `sha2` as the only dependency; and no file under `../crates/` was written during
      this pass (`find crates -newermt`); the only `crates/` strings in this crate are the witness
      documentation U1–U3 already carry

---

## Dependencies & Execution Order

- **Setup (Phase 1)** → **Foundational (Phase 2)** blocks the three stories.
- **US1 (Phase 3)** is the MVP: without geometry there is nothing to keep resident or to account for.
- **US2 (Phase 4)** and **US3 (Phase 5)** both need US1's mesh; they are independent of each other.
- **Polish (Phase 6)** needs all three, and T021 is last because it grades the whole pass.

## What this pass leaves owed (named in the plan, not hidden here)

- The **displaced 256×** level: a non-planar quad must be subdivided and triangulated (C-43).
- The **multi-chunk stitched mesh** and the seam's marching rule. **(§40, 2026-10-13:** it lands as one
  footprint over a rectangle of chunks (`U4 region:`); a mesh **wider** than one rectangle, with a residency of
  its own, is owed.**)**
- The **strided field request**, which belongs in U3's patch vocabulary rather than here.
- A **residency policy** that decides which chunks to keep (U5/U6's, not U4's), and persistence.
