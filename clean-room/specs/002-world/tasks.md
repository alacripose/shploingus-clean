# Tasks 002 — U2 World model

- [x] T001 Add the one justified dependency (`sha2`) with its reason beside the line —
      `Cargo.toml` · *check:* `cargo test --offline` resolves it from cache.
- [x] T002 The named-refusal vocabulary: `Refusal`, `RefusalKind`, `RefusalSet` that collects **all**
      — `src/refusal.rs` · *check:* `a_refusal_set_collects_every_problem_not_the_first`.
- [x] T003 [US1] Coordinate algebra: signed 32-bit `VoxelPos`, `ChunkKey`, `WithinChunk`, Euclidean
      chunk mapping — `src/coordinates.rs` · *check:* `coordinates_round_trip_across_the_origin`.
- [x] T004 Subvoxel (`0..4`) and detail (`0..256`) indices, refusing out-of-range by name —
      `src/coordinates.rs` · *check:* `a_subvoxel_or_detail_index_out_of_range_is_refused_by_name`.
- [x] T005 [US1] The 20-level ladder and `RegionKey`, with the corrected arithmetic —
      `src/coordinates.rs` · *check:* `the_ladder_arithmetic_is_the_corrected_one`.
- [x] T006 [US1] Face relations: `chunks_reached_by` = containing chunk + one per boundary face,
      never a diagonal — `src/coordinates.rs` · *check:* `a_corner_voxel_reaches_three_chunks_and_no_diagonal`.
- [x] T007 [US4] Materials: exact rational `Density`, sourced `Reflectance`, one `MaterialRegistry`
      that admits once and refuses by name — `src/material.rs` · *check:*
      `an_unadmitted_material_is_refused_by_name`, `a_density_that_is_not_positive_is_refused`,
      `a_material_without_reflectance_admits_but_derives_no_colour`.
- [x] T008 [US2] `WorldIdentity`, `DeclaredPeriod`, `Tick`, the closed `Domain`, and `World` owning
      identity and residency as separate concerns — `src/world.rs` · *check:*
      `truth_is_a_function_of_seed_tick_and_period`.
- [x] T009 [US2/US3] `truth_digest` (excludes anchor/residency), `residency_digest` (order-free),
      `plan_digest` (order-free aggregate) — `src/world.rs` · *check:*
      `the_cache_digest_does_not_depend_on_insertion_order`, `a_plan_digest_is_order_independent`.
- [x] T010 [US2] `stage_seed(domain, pos)` pure, and different per domain and per position —
      `src/world.rs` · *check:* `stage_seeds_differ_by_domain_and_by_position`.
- [x] T011 Entry point prints the U2 digests and runs the U2 checks — `src/main.rs` ·
      *check:* `cargo run` prints the digest hexes and exits 0.
- [x] T012 The registry gains a `status` column so the program's scope is a printed value —
      `src/systems.rs` · *check:* `the_registry_is_closed_and_ordered` requires a status per system.
- [x] T012b Split the crate into a **library** and a thin binary, so the public API is a real API
      exercised by the tests and the binary rather than `pub` dead code in a bin — `src/lib.rs`,
      `src/main.rs` · *check:* `cargo clippy --all-targets -- -D warnings` is clean.
- [x] T013 The shape laws behind a `Profile` (cylinder, flat cap, frustum/cone, spheroid, and the
      Szymczak et al. 2025 stalagmite — its `Da` law kept by its exact consequences, the cap radius
      and the boundary normals, over a recorded elliptical flank that makes silhouette and normal
      one body), with the **exact** normal, the **silhouette** radius and a malformed shape refused
      by name — `src/profile.rs` · *check:*
      `a_malformed_profile_is_refused_by_name_and_not_accepted`,
      `a_spheroid_uses_its_silhouette_and_not_a_base_radius`,
      `a_stalagmite_keeps_the_laws_exact_consequences_and_its_printed_figure_does_not_reproduce`,
      `a_stalagmite_silhouette_and_normal_belong_to_one_body`.
- [x] T014 `ShaperSet` — `ShaperKind` × `Level`, `acts_at` required, and the stored lattice rule: a
      **physical** shaper changes occupancy, a **material** shaper cannot — `src/shaper.rs` ·
      *check:* `a_physical_shaper_changes_occupancy_and_a_material_shaper_cannot`,
      `a_shaper_that_claims_the_addressed_level_and_declares_nothing_is_refused_by_name`,
      `the_taxonomy_is_two_kinds_by_two_levels_and_every_cell_is_reachable`.
- [x] T015 The append-only `Chain`: a closed `Namespace`, an event that links to the head, tickets as
      a **view** of the chain, and a wire form that round-trips exactly — `src/chain.rs` ·
      *check:* `an_event_links_to_the_head_and_the_chain_verifies`,
      `the_wire_form_round_trips_exactly_and_is_deterministic`,
      `a_tampered_wire_is_refused_by_name`,
      `a_ticket_is_filed_and_resolved_once_and_the_chain_is_the_state`.
- [ ] T016 **Still owed inside U2:** the civilian/government vocabulary (the Axis/Laya ladders) —
      *check:* a bare ladder token is refused by name. Not started: this pass chose depth on T013–T015.
      *(2026-10-13, §32: **still owed**, and now measured in place — U12's first mob landed **without**
      inventing a token: the office it holds is the organisation's own word, and `AxisKind::OrgRank` and
      `AxisKind::Certification` print **owed by name** (`src/axis.rs`). The check the row names already
      bites: a bare ladder token refuses `bare_token_refused` (`AMBIGUOUS`/`UNQUALIFIED`), and the two
      owed ladders are the ones U12's own organisation would read.)*
- [x] T017 **Constitution task:** re-run the checks behind every P1–P7 row of `plan.md` §3 and record
      the output — *check:* `cargo test --offline` = **45 passed**, `cargo clippy --offline
      --all-targets -- -D warnings` = clean, `cargo run --offline` exits 0 with the shape, shaper and
      chain measurements printed.
