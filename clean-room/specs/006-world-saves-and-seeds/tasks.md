---
description: "Task list for U9 — saves and loads, and the world's own seed systems"
---

# Tasks: U9 — Saves and loads, and the world's own seed systems

**Input**: Design documents from `/specs/006-world-saves-and-seeds/`

**Prerequisites**: [plan.md](./plan.md) (required), [spec.md](./spec.md) (required — three user
stories), [research.md](./research.md), [data-model.md](./data-model.md),
[contracts/u9-seams.md](./contracts/u9-seams.md), [quickstart.md](./quickstart.md)

**Tests**: included, because this project's rule is that a task is done when its check was run and its
output inspected (Constitution P4): each implementation task names the check that decides it.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependencies)
- **[Story]**: `US1` (a written world reads back as the same world), `US2` (a save carries its own seed
  systems), `US3` (a save that cannot be trusted is refused by name)
- Every task names the file it touches and the check that decides it

## Path Conventions

Single project — the clean-room crate at `clean-room/`: sources in `src/`, module tests in
`#[cfg(test)]`, the end-to-end check in `tests/`, documents in `specs/006-world-saves-and-seeds/`.

---

**Status 2026-10-12: T001–T020 are done and their checks were run**; the printed output is pasted in
[quickstart.md](./quickstart.md). One decision changed during the pass and is recorded in
[research.md](./research.md): the per-domain seed is a **new declared number** at U2's owner rather than
a re-rooting of `stage_seed`, because re-rooting would have moved the already-published U3/U4 digests
(**D2**). One thing is owed and named at the end of this file.

## Phase 1: Setup

- [x] T001 Add U9's refusal kinds to the existing closed vocabulary — `not_a_save`, `save_truncated`,
      `save_tampered`, `seed_undeclared`, `identity_mismatch`, `record_diverged` — `src/refusal.rs` ·
      *check:* ran — `RefusalKind::COUNT` is 50 and `ALL.len() == COUNT` still holds; each kind is
      asserted in the test that produces it, and the binary prints four of the six names (the other
      two cannot be written through the public surface — see T013)
- [x] T002 [P] Register the new module on the library surface — `src/lib.rs` · *check:* ran —
      `cargo clippy --offline --all-targets -- -D warnings` is clean

## Phase 2: Foundational

- [x] T003 `World::stream_seed(domain)` — the **per-domain** seed, at U2's owner, a pure function of
      `(seed, declared period, tick, domain)`; five distinct values for the five declared domains —
      `src/world.rs` · *check:* ran — `stream_seeds_differ_by_domain_and_are_pure` (five distinct
      values, pure, and the seed, the period and the tick each participate)
- [x] T004 [P] `reference::identity()` — one declaration of the reference world's identity, so the
      binary's evidence, its session and its tests cannot drift into three similar worlds —
      `src/reference.rs` · *check:* ran — `reference::gate` calls `identity()`, and every U9 line and
      test above is about seed `1592639215`

## Phase 3: US1 — a world written down reads back as the same world

- [x] T005 The exact encoding: `Save::of(world, chain, ruleset)` writes the declared layout —
      `src/save.rs` · *check:* ran — `a_save_round_trips_its_bytes_exactly`,
      `two_saves_of_one_world_are_one_byte_string`, and the binary's `465 bytes … = true` line
- [x] T006 The parser: framing, every byte accounted for, integrity before interpretation —
      `src/save.rs` · *check:* ran — `bytes_that_are_not_a_save_are_refused_by_name`,
      `a_truncated_save_is_refused_by_name_and_says_where` (cuts at 20/40/100 bytes),
      `a_flipped_byte_is_refused_as_tampered_with_both_digests`
- [x] T007 The loader and the load report: the world rebuilt from the identity, `truth_digest` derived
      rather than read — `src/save.rs` · *check:* ran —
      `a_loaded_world_carries_the_truth_digest_of_the_world_it_was_written_from`; the binary prints
      `32b3c1f8…` for both the loaded world and the live one, and the previous tick differs
- [x] T008 The record: carried in the chain's **own** wire form, declared by head and length, never
      mirrored — `src/save.rs` · *check:* ran — `the_loaded_record_is_the_same_chain_and_verifies`,
      `a_record_that_is_not_the_declared_one_is_refused_as_diverged` (a coherent save whose
      declaration belongs to another record), and `an_empty_record_is_a_record`

## Phase 4: US2 — a save carries its own seed systems

- [x] T009 The five stream declarations, checked in **both** directions (a missing domain and an
      undeclared one) — `src/save.rs` · *check:* ran —
      `a_save_missing_a_declared_stream_is_refused_by_name` (the missing domain is named),
      `a_stream_the_receiver_does_not_declare_is_refused_by_name` (tag 9 is named)
- [x] T010 A stream seed that is not the world's own is refused with the domain and both values —
      `src/save.rs` · *check:* ran — `a_stream_seed_that_is_not_the_worlds_own_is_refused_with_both_values`
- [x] T011 The ruleset identity: a load under another ruleset or revision is refused with both —
      `src/save.rs` · *check:* ran — `a_load_under_another_ruleset_is_refused_with_both` (ruleset 7
      revision 3 against ruleset 8 revision 3, both named), and the same ruleset then loads
- [x] T012 The tick is not the seed: two saves at consecutive ticks share the seed and the period and
      differ only in the tick — `src/save.rs` · *check:* ran —
      `consecutive_ticks_share_the_seed_and_differ_in_the_tick`, and the same test in `tests/u9_saves.rs`
      through the public surface

## Phase 5: US3 — a save that cannot be trusted is refused by name

- [x] T013 Every one of the six refusal names is reachable and asserted, and a set of faults is
      collected rather than the first — `src/save.rs` · *check:* ran — the six named tests, plus
      `every_fault_in_a_save_is_collected_not_just_the_first` (**4** faults in one pass). **The two the
      binary cannot print** (`seed_undeclared`, `record_diverged`) are pinned here and named in the
      binary's own output, because the public writer will not emit a save it would refuse
- [x] T014 No partial load: `Loaded` exists only when every check passed — `src/save.rs` · *check:* ran —
      `a_refused_load_produces_no_world_and_no_record` (another ruleset, and a declared period of zero
      refused by U2's own rule)

## Phase 6: Polish

- [x] T015 The end-to-end check through the **public API** and a **real file** — `tests/u9_saves.rs` ·
      *check:* ran — **4 tests pass**: the file round trip with one generation digest at 1/2/4/8
      workers, the five stream seeds and the ruleset refusal, consecutive ticks, and the three
      refusals reached from bytes on disk (each test owns its file name)
- [x] T016 `check_u9` in the binary: the file round trip with its byte length, the identity and the
      five stream seeds, the loaded digest against the live one, one generation digest at 1/2/4/8
      workers, and the refusals — with every line naming the world it is of (C-46) —
      `src/evidence.rs`, `src/main.rs` · *check:* ran — `cargo run --offline` prints them and exits 0;
      `cargo run --offline` twice gives **byte-identical** stdout
      (`sha256 5ae54082…2ab71`, 240 lines)
- [x] T017 `src/systems.rs`: U9's real status, and the five mission systems the plan admitted in §16
      carried on the registry rather than left implicit · *check:* ran —
      `the_registry_is_closed_and_ordered` still holds for **13** systems, and the printed registry
      carries `[U9] … (slice 9)` through `[U13]`
- [x] T018 [P] `quickstart.md`: paste the measured output; tick this file only where the check ran ·
      *check:* every tick above has a printed line beside it in [quickstart.md](./quickstart.md)
- [x] T019 The governing plan's §5 row 9 and §16.4 status, recording the pass and what U9 still owes —
      `docs/CLEAN-ROOM-PLAN-2026-10-11.md` (the repository's governing plan, two levels above
      `clean-room/`) · *check:* the documented numbers equal the printed ones
      (`465 bytes`, `32b3c1f8…`, `c331cf55…`, 166 lib + 4 U9 tests)
- [x] T020 **Constitution task:** re-run the checks behind every P1–P7 row of [plan.md](./plan.md) ·
      *check:* ran — `cargo test --offline` **166 lib + 2 U4 + 6 U6 bridge + 5 U6 transport + 4 U9 = 183
      passed, 0 failed**; `cargo clippy --offline --all-targets -- -D warnings` exit 0; `cargo run
      --offline` exit 0 with byte-identical stdout across two runs; `cargo tree --offline` shows `sha2`
      as the only dependency; and `find crates -newermt` returns **nothing** — no file under `crates/`
      was read or written during this pass

---

## Dependencies & Execution Order

- **Setup (Phase 1)** → **Foundational (Phase 2)** blocks the three stories.
- **US1 (Phase 3)** is the MVP: without a written world there is nothing to check seeds or refusals
  against.
- **US2 (Phase 4)** and **US3 (Phase 5)** both need US1's encoding; they are independent of each other.
- **Polish (Phase 6)** needs all three, and T020 is last because it grades the whole pass.

## What this pass leaves owed (named in the plan, not hidden here)

- The **document form** of a save (`path + digest`), which is where U6's content gate becomes a hard
  prerequisite. *(Corrected 2026-10-12, §28: landed — written and read through the gate by the table's
  own `save`/`load` rows, with the gate's `Intake::read` as the only reader.)*
- **`save`/`load` as rows of the one tool table** — U6's door; the table is closed at five verbs today.
  *(Corrected 2026-10-12, §28: landed as the table's seventh and eighth rows, so it is closed at eight.)*
- **A save of a resident set**, compression, and a versioned migration path: none is carried, and none
  is half-built.
- The **streamed residency policy** (U4/U11) and the **planet scale contract** (U11).
