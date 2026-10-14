---
description: "Task list for U6 — the record and the API layer (the gate)"
---

# Tasks: U6 — the record and the API layer (the gate)

**Input**: Design documents from `/specs/005-u6-record-api/`

**Prerequisites**: [plan.md](./plan.md) (required), [spec.md](./spec.md) (required — three user
stories), [data-model.md](./data-model.md), [contracts/u6-seams.md](./contracts/u6-seams.md),
[quickstart.md](./quickstart.md)

**Tests**: included, because this project's rule is that a task is done when its check was run and its
output inspected (Constitution P4): each implementation task names the check that decides it.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependencies)
- **[Story]**: `US1` (the answer carries its evidence), `US2` (one request twice is one answer, and the
  record shows what changed), `US3` (a request that cannot be honoured is refused by name)
- Every task names the file it touches and the check that decides it

## Path Conventions

Single project — the clean-room crate at `clean-room/`: sources in `src/`, module tests in
`#[cfg(test)]`, the end-to-end check in `tests/`, documents in `specs/005-u6-record-api/`.

---

**Status 2026-10-11: T001–T027 are done and their checks were run**; the printed output is pasted in
[quickstart.md](./quickstart.md). Three decisions were taken during the gate pass and are recorded in
[research.md](./research.md): the claims are re-checked **inside** `call` (D3 — a receipt is only handed
out when its evidence holds *now*, so `claims_hold` is a load-bearing check rather than a re-verification
afterwards), the geometry event kind belongs in `chain.rs` (D5 — the record's vocabulary is U2's), and a
field's claim is re-evaluated while a mesh's claim is **read** (D3 — that readable difference is what the
spec's eviction edge case measures). The transport followed as its own pass (T021–T027): the table is
now served over stdio by the binary's `serve` mode, and both the binary's evidence and
`tests/u6_transport.rs` drive it as a **separate process** whose answers are verified against their own
process's world, record and resident rather than trusted for arriving over a pipe.

## Phase 1: Setup

- [x] T001 Add U6's refusal kinds to the existing closed vocabulary — `tool_undeclared`,
      `argument_undeclared`, `argument_absent`, `argument_malformed`, `claim_unverified` —
      `src/refusal.rs` · *check:* ran — each kind is asserted in the test that produces it
      (`every_argument_fault_is_refused_by_name`, `a_domain_fault_is_decided_by_the_owner_and_named_in_
      its_vocabulary`, `a_receipt_verified_against_a_moved_world_is_refused_by_name_with_both_values`),
      and the binary prints them in its refusal lines
- [x] T002 [P] Register the new module on the library surface and name it in the crate header —
      `src/lib.rs` · *check:* ran — `cargo clippy --offline --all-targets -- -D warnings` is clean

## Phase 2: Foundational

- [x] T003 The closed table as a value: the `Verb` enum, `ArgKind` with its shape rule, `Argument`,
      `Tool`, `TOOLS`, `TOOL_COUNT` and `tool(name)` — `src/bridge.rs` · *check:* ran —
      `the_table_is_closed_and_bijective` (one name per verb, one verb per name, one value per key,
      every declared check a real one, and an undeclared verb refused by name)
- [x] T004 The request form and its canonical line: `parse`, declared-order construction, and the
      per-kind shape checks — `src/bridge.rs` · *check:* ran —
      `two_spellings_of_one_request_are_one_canonical_line` (three spellings, one line),
      `every_argument_fault_is_refused_by_name` (undeclared, absent, malformed ×8, duplicate, no `=`,
      empty line)
- [x] T005 The typed reads, so the **domain** is decided by the owner: `Request::{chunk, voxel, level,
      field, side}` — `src/bridge.rs` · *check:* ran —
      `a_domain_fault_is_decided_by_the_owner_and_named_in_its_vocabulary` (`level=9` ⇒
      `level_off_ladder` from U4, `field=nope` ⇒ `field_undeclared`, and the in-domain reads agree with
      their kinds)

## Phase 3: US1 — the answer comes back with its evidence

- [x] T006 `Answer` and its **pure** digest: the described facts only, never the work —
      `src/bridge.rs` · *check:* ran —
      `one_request_twice_is_one_answer_and_the_second_changes_nothing` asserts one answer digest across
      a derivation, a hit, and gates at 2, 4 and 8 workers
- [x] T007 `Claim` and `Claim::check`: every claim re-reads its owner, and the detail names the claim,
      the recorded value and the found value — `src/bridge.rs` · *check:* ran —
      `an_answer_is_checked_against_the_world_and_not_against_itself` (a moved world fails `truth_is`
      and `mesh_is_held`; an eviction fails `mesh_is_held` alone), and
      `the_record_is_the_chain_and_not_a_second_copy_of_it` (an `event_is` naming a position the chain
      does not have is refused)
- [x] T008 `CheckName`/`Check`/`CheckOutcome` and the `Receipt`, with the checks a row declares emitted
      in the declared order — `src/bridge.rs` · *check:* ran —
      `every_declared_verb_answers_and_every_answer_verifies` asserts each receipt records exactly the
      checks its row declares, all passed
- [x] T009 The gate: dispatch to the owners' seams (`World::truth_digest`, `field_at`,
      `Mesher::derive`, `Resident`, `Chain`) and the claim re-check inside `call` —
      `src/bridge.rs` · *check:* ran — `every_declared_verb_answers_and_every_answer_verifies`
      (all five verbs, on the reference world, through the public API) and
      `tests/u6_bridge.rs`'s reference-world tests. *(This test's literal argument count has moved with
      the table — five at this slice, 15 once `admit` arrived; the table is closed at **eight** verbs
      after `save`/`load`, §28.)*

## Phase 4: US2 — one request twice is one answer, and the record shows what changed

- [x] T010 `Effort`, the gate's declared `Budget` and `Gate::set_budget`, so U4's ceiling is reachable
      through the outward door — `src/bridge.rs` · *check:* ran —
      `a_parse_fault_writes_nothing_and_a_world_refusal_is_recorded_by_name` refuses
      `geometry_budget_exceeded` with `1024` in its detail and caches nothing
- [x] T011 `Effect` as a **delta** of what the owners hold, and the `recorded` check that reads it —
      `src/bridge.rs` · *check:* ran —
      `one_request_twice_is_one_answer_and_the_second_changes_nothing` (`1024` columns then `0`,
      appended then not, the record moved once) and
      `a_read_appends_nothing_and_a_parse_fault_is_not_an_event` (four reads, no event, no refusal)
- [x] T012 `geometry` appends **one** `geometry_derived` on a miss and nothing on a hit, and the
      receipt claims the event it appended — `src/bridge.rs` · *check:* ran —
      `every_declared_verb_answers_and_every_answer_verifies` (one derivation, one event, and the world's
      opening makes two)

## Phase 5: US3 — a request that cannot be honoured is refused by name

- [x] T013 The split: `parse` writes nothing; a dispatch refusal is recorded by the owner's own name and
      then passed through — `src/bridge.rs` · *check:* ran —
      `a_parse_fault_writes_nothing_and_a_world_refusal_is_recorded_by_name` (three parse faults leaving
      the record untouched; four world refusals each recorded once, naming themselves, caching nothing)
- [x] T014 The unrecordable-refusal edge: a payload the wire form cannot frame is refused
      `payload_not_wirable` and the returned refusal names **both** facts — `src/bridge.rs` · *check:*
      ran — `a_refusal_that_cannot_ride_the_record_names_both_facts` (a tabbed detail; the chain stays
      empty)

## Phase 6: Polish

- [x] T015 [P] The one new event kind at the record's owner, and the namespace it belongs to —
      `src/chain.rs` · *check:* ran — `every_declared_verb_answers_and_every_answer_verifies` counts the
      `geometry_derived` events in the reference world's chain, and
      `a_parse_fault_writes_nothing_and_a_world_refusal_is_recorded_by_name` asserts a recorded refusal
      is a `refusal_recorded` in the `record` namespace
- [x] T016 The end-to-end check through the **public API** — `tests/u6_bridge.rs` · *check:* ran — six
      tests drive every verb, both refusals, the repeat at four worker counts,      the requirement of every declared argument straight from the table, and the correspondence of an
      answer to the world it was made in — the **re-evaluated** field claim (`field_is` on a moved
      world, with both value digests), the **read** geometry claim (eviction fails `mesh_is_held`
      alone) and the record claim — on the **reference world**
- [x] T017 `check_u6` in the binary: the table, the five receipts with their answers, digests, checks,
      claims and effects, the repeat, both refusal classes, the correspondence lines and what U6 owes —
      `src/main.rs` · *check:* ran — `cargo run --offline` prints 50 `U6` lines and exits 0
- [x] T018 `src/systems.rs`: U6's real status, replacing `absent` — and only what was built · *check:*
      ran — `the_registry_is_closed_and_ordered` still holds, and the status names the owed content gate,
      transport and by-reference verbs
- [x] T019 [P] `quickstart.md`: paste the measured output; tick this file only where the check ran ·
      *check:* every tick above has a printed line beside it in [quickstart.md](./quickstart.md)
- [x] T020 The governing plan's slice-5 row and a new section recording the pass and what U6 still owes —
      `../docs/CLEAN-ROOM-PLAN-2026-10-11.md` · *check:* the documented numbers equal the printed ones
      (`5 verbs`, one answer digest `25a485a5…` before §39, `1024` then `0` columns, `407.3 m`, `2 refusal(s)`)

## Phase 7: The transport — the table served over stdio

- [x] T021 The transport module: the line framing in both directions, `Session`, `ServedRecord`,
      `serve`/`serve_stdio`, `read_response`/`parse_response`, and the client's `request`/`send` —
      `src/transport.rs` · *check:* ran — six module tests: a session that round-trips every verb,
      a world refusal that keeps its owner's name **and** shows the record grow by one, a parse fault
      that writes nothing, the transport's own `payload_not_wirable` refusal, the reader refusing every
      form the writer could not make, and the served receipt round-tripping as the same value
- [x] T022 The reference declaration in **one** place: `reference_gate`, so `serve`, the binary's own
      evidence and the tests cannot drift into three similar worlds — `src/transport.rs`, `src/main.rs` ·
      *check:* ran — `check_u6` now builds its gate from `transport::reference_gate`, and every printed
      number is unchanged (the same seed, ladder, horizon and identity)
- [x] T023 The binary's `serve` mode and a **separate-process** round trip inside its own U6 evidence:
      a success, a world refusal, a parse fault, the repeat and the correspondence checks —
      `src/main.rs` · *check:* ran — `cargo run --offline` prints the transport block
      (`5/7 answered`, `correspondence = true`, the refusal lines) and exits 0
- [x] T024 The end-to-end transport check through the **public surface**: `tests/u6_transport.rs`
      spawns the real binary and verifies every served receipt, answer digest, record length and head
      against a world this process built itself — *check:* ran — 3 tests pass
      (`cargo test --offline --test u6_transport`)
- [x] T025 [P] The protocol declared as an interface contract —
      `specs/005-u6-record-api/contracts/u6-transport.md` · *check:* the declared grammar is the one
      the module's own tests hold to (`the_response_reader_refuses_what_the_writer_could_not_have_made`)
- [x] T026 [P] `quickstart.md` and `src/systems.rs`: the transport's status and printed output pasted,
      and only what ran ticked · *check:* the pasted numbers equal the printed ones, and
      `the_registry_is_closed_and_ordered` still holds
- [x] T027 The governing plan's sixth-pass section and the remaining owed list —
      `../docs/CLEAN-ROOM-PLAN-2026-10-11.md` · *check:* the documented numbers equal the printed ones
      (`5/7 answered`, `correspondence = true`, one answer digest `4c4da329…` before §39, `723bab9b…`
      after it)

---

## Phase 7 — the structure pass (2026-10-12)

Shape only: no behaviour changed, and every number above was re-measured after it. The earlier entries
name `src/bridge.rs`, which is now the **`src/bridge/` directory** behind a `mod.rs` façade — the
public surface (`crate::bridge::X`) is byte-for-byte the same list of items, so nothing that read it
had to change.

- [x] T028 `bridge.rs` split along the seams it already had into `bridge/{table,request,receipt,wire,
      gate}.rs` plus `mod.rs` (façade) and `bridge/tests.rs` — the table, the canonical line, the
      answer/claims/effect/receipt, the external form, and the gate each stand alone; a client that
      only reads answers needs `receipt` + `wire`, and only the server side needs `gate`.
      *check:* ran — 156 tests still pass, `clippy --all-targets -- -D warnings` clean, and `cargo run`
      exits 0 with its evidence unchanged
- [x] T029 The reference world moved out of the framing into **`src/reference.rs`** (`gate`,
      `serve_stdio`), so the transport learns nothing about a palette, a horizon or an identity and
      `serve(&mut Gate, …)` is the whole of its contract — *check:* ran — the served session answers
      exactly as before (`truth` digest `146ff7bf…`), and `transport.rs` no longer builds a world
- [x] T030 The receipt wire's separators have **one owner**: `REQUEST_FORBIDDEN` now composes
      `bridge::wire::{WIRE_PART, WIRE_ITEM, WIRE_CHECK_FIELD}` instead of respelling `~ ; |` — a comma
      stays legal, because `chunk=0,0,0` is a request — `src/transport.rs` · *check:* ran — the
      refusal's own words for an unwirable line are unchanged
- [x] T031 The per-slice measurements moved out of the binary into **`src/evidence.rs`**, so `main.rs`
      is its two entry points (78 lines: the evidence, or a served session) and a system's bar is
      measured the same way whoever calls it — *check:* ran — `cargo run` prints all 55 U6 lines and
      every other slice's evidence, exit 0

---

## Phase 8 — the two defects the audit named (2026-10-12)

Each is fixed at its cause and pinned by a test that fails on the old behaviour. No capability was
added, no answer changed: the binary's stdout is byte-for-byte what it was before the fixes (187 lines,
55 of them U6's), and the served session still answers `truth` with digest `146ff7bf…`.

- [x] T032 **The session clients read while they write.** `Client::talk` in `tests/u6_transport.rs`,
      the evidence client in `src/evidence.rs`, and the new `transport::read_response_for` now send one
      request and read its answer before the next — a client that writes a whole session first deadlocks
      as soon as the answers fill the pipe buffer, and a hang is worse than a failure. *check:* ran —
      `a_long_session_completes_instead_of_blocking_on_the_pipe` drives 12 000 requests — 72 KiB of
      requests and megabytes of answers, more than a pipe buffer on either side — through the real
      binary under a bounded 60 s wait, finishing in 1.3 s. A throwaway copy of the **old** client,
      run once and deleted, wrote the same session before its first read and got **0 answers in 20 s**
      (blocked, killed by the bound); the shell measured the same effect end to end: 4000 requests in
      1 s with a client reading as it goes against 9 s with one that defers reading by 8 s
- [x] T033 **The `err` echo is kept and checked.** `Response::Err` carries the line it is about;
      `parse_response` requires the echo and refuses one that could not have crossed (or none at all)
      `malformed_wire`; `Response::is_about` + `transport::read_response_for` refuse a response about a
      **different** line, naming both — *check:* ran —
      `a_response_about_a_different_line_is_refused_by_name_with_both_values`,
      `an_err_response_whose_echo_names_no_line_that_could_have_crossed_is_refused`,
      `a_response_whose_echo_names_another_request_is_refused_by_name`, and
      `every_refusal_the_session_writes_is_about_the_line_it_answers` (the writer's refusals all pass
      the reader's new rule)
- [x] T034 The blank-line behaviour **recorded rather than changed** —
      `contracts/u6-transport.md` · *check:* the contract states it as behaviour (a counted request that
      names no verb), and `a_line_that_does_not_parse_writes_nothing_and_a_line_that_cannot_cross_is_refused`
      already pins it

---

## Dependencies & Execution Order

- **Setup (Phase 1)** → **Foundational (Phase 2)** blocks the three stories: the table and the parser
  are the surface every verb is called through.
- **US1 (Phase 3)** is the MVP: without an answer that carries its evidence, there is nothing for the
  repeat (US2) or the refusal class (US3) to be measured against.
- **US2 (Phase 4)** and **US3 (Phase 5)** both need US1's receipt; US3's unrecordable edge needs US2's
  record-refusal path.
- **Polish (Phase 6)** needs all three, and T020 is last because it grades the whole pass.
- **Phase 7 (the transport)** needs the gate whole (a transport is a way in, and there is nothing to
  serve before all three stories hold); T027 is last because it grades the transport pass.

## What this pass leaves owed (named in the plan, not hidden here)

- The **content gate**: `admit`/`inspect` → `Established` (never a bool), the document record and its
  compression, the structural judge with its calibration, the ballot, the crate-by-axis census, the
  permission policy matrix, the recipe format and the API document. *(Corrected by T031 below: its
  **first slice** — a declared row consumed in place by reference, judged on named axes, with an
  unchecked axis never a pass — has since landed in `src/intake.rs`; the rest of this list is what
  remains.)* *(Corrected again 2026-10-12 by Phase 11 below: the **permission policy matrix** has since
  landed as the policy chokepoint; what remains of it is the self-describing protocol and the
  tool-surface narrowing over a mod-supplied tool list.)*
- The **bijection of the served table against the legacy action vocabulary** — a witness check, and P7
  forbids reading `crates/voxel-mcp` for it, so it is named owed rather than approximated.
- **By-reference requests** *(corrected by Phase 9 below: **landed** — `admit` is a row of the table,
  its document arrives as `path + digest` and is read in place; the argument kinds it needed are the
  `Path`, `Digest` and `Name` of T035)*.
- The **streamed residency policy** (which chunks to keep, when to evict) — U4 owns exact invalidation,
  which the gate exercises but does not decide.

---

## Phase 8 — the content gate's first slice (2026-10-12)

Specified by §2's **U6** entry, §5 row 5's bar and §16 row 10's row form in
`../docs/CLEAN-ROOM-PLAN-2026-10-11.md`, and by `docs/LEDGER.md` §55.3 — the record's own decision on
intake. Everything below was measured by running it.

- [x] T031 The content-intake gate as one module: a declared row (feature, `Source { path, digest }`
      consumed **in place**, the licence verdict it relies on, its carrier, its own acceptance check,
      its `Claim`), six judged axes with the four establishment states, `Established` (never a bool),
      and `census` — `src/intake.rs`, `src/refusal.rs` (+7 names), `src/lib.rs`, `src/systems.rs` ·
      *check:* ran — `cargo test --offline` is **196 passed, 0 failed** (174 lib, +5 of them intake's)
      and `cargo run --offline` exits 0 after printing the intake block
- [x] T032 The end-to-end check through the **public surface** with real files:
      `tests/u6_intake.rs` writes its documents under the system temp directory, admits one, refuses a
      document that changed underneath its row, and proves the undeclared and the unchecked paths are
      two names · *check:* ran — 5 tests pass (`cargo test --offline --test u6_intake`)
- [x] T033 The evidence the binary prints: `check_u6_intake` in `src/evidence.rs`, called from
      `check_u6` so `cargo run` is the second door · *check:* ran — the intake block's numbers are the
      ones §18 of the governing plan records, and the *owed* line now names the gate's remainder
      instead of the gate
- [x] T034 The governing plan's eighth-pass section and the three stale owed lists it corrects —
      `../docs/CLEAN-ROOM-PLAN-2026-10-11.md` §18 · *check:* ran — the documented numbers equal the
      printed ones, and every published digest of U1–U4/U6/U9 reappears unchanged in the same run

---

## Phase 9 — the row's wire form: a document admitted by reference, over the served line (2026-10-12)

Specified by §16 row 10's row form and §5 row 5's bar in `../docs/CLEAN-ROOM-PLAN-2026-10-11.md` (the
*by-reference verbs* the mods-derived tree waits on), and by `src/intake.rs`'s own remainder.
Everything below was measured by running it.

- [x] T035 `admit` as a row of the **closed table**: eight declared arguments (`path`, `digest`,
      `feature`, `licence`, `claim`, `carrier`, `check`, `check_digest`) and the three argument kinds a
      reference needs (`Path`, `Digest`, `Name`, each with its shape rule — a relative path that no
      framing can mistake, refusing separators and `..` by shape, before any world is read) —
      `src/bridge/table.rs`, `src/refusal.rs` (+`claim_undeclared`, the coercion `RefusalSet::joined`)
      · *check:* ran — the table is closed and bijective at **six** rows, and every declared argument is
      required (the crate's own tests walk the table and drop each one; **corrected 2026-10-12 by
      T043**: `admit` now declares **ten**, of which `requires_stages`/`requires_materials` are
      optional — the test that walks the table drops each argument and asserts `argument_absent` for
      the required ones and **admission** for the two optional ones)
- [x] T036 The row dispatched to the **intake's seam and the answer made wire-able**: `Verb::Admit` →
      `intake.inspect`/`verdict` reading the document **where it lives**, the gate holding the root a
      reference resolves against (`.` — the directory the server runs in — pinned by
      `Gate::set_intake_root` where a caller needs another), `Answer::Admitted` with its **pure** digest,
      its `admitted,…` wire form and reader (axes by name, in `Axis::ALL` order), and no claim made
      where no owner could re-check one · *check:* ran — `cargo test --offline` is **202 passed,
      0 failed**; `cargo run --offline` exits 0 at **270 lines**, the nine more accounted for line by
      line (two for the row, six for its receipt, one for the two refusals)
- [x] T037 The end-to-end check **through the binary as a separate process**, over a fixture tree it
      writes itself: `tests/u6_admit.rs` — a row admitted by reference with the served answer equal to
      the one derived in-process, an absent document refused `source_absent` and a moved one
      `source_digest_mismatch` (both recorded, because both lines parsed), a line the table does not
      declare writing nothing, and one row twice being one answer · *check:* ran — 4 tests pass
      (`cargo test --offline --test u6_admit`)
- [x] T038 The **defect this pass found and fixed** rather than worked around: a receipt about an
      **empty** record wrote its absent head as `none`, which its own reader refuses — the writer and
      the reader now share one spelling for an absence (`-`), pinned by
      `every_answer_the_wire_writes_is_an_answer_the_wire_reads` · *check:* ran — the unit test round
      trips an empty record's answer and an admission's, and the served client’s first `record` call
      reads back
- [x] T039 The governing plan's ninth-pass section, the transport contract's by-reference clause,
      **U6's own registry status** (a closed six-row table, `admit` arriving by reference, the wire
      form no longer owed — `src/systems.rs`), and the four stale owed lists this pass corrects
      *(the row count has since moved again: `save`/`load` are the seventh and eighth, §28)* ·
      *check:* ran — the documented numbers equal the printed ones (`cargo run --offline` exit 0, 270
      lines, two runs byte-identical at sha256 `4f520076…`), and `the_registry_is_closed_and_ordered`
      still holds

---

## Phase 10 — a row's conformance against the world's own declarations (2026-10-12)

Specified by §2's **U6** entry and §16 row 10's row form in `../docs/CLEAN-ROOM-PLAN-2026-10-11.md`,
by that plan's §18 clause (*"`requires_stages` against U3's `Declared` and `requires_materials` against
U2's `MaterialRegistry`"*) and by `docs/LEDGER.md` §55.3's ruling — *"conformance is against the world,
not a list of strings"*. Everything below was measured by running it.

- [x] T040 The **seventh axis** in the closed vocabulary: `Axis::Conformance` **appended** (a verdict's
      axes are positional, so an inserted column would relabel every report that exists),
      `Axis::COUNT` 7, and the two refusals it can raise by name — `RefusalKind::StageUndeclared`
      (with the stage named **and the roster's own names** in the message) and
      `RefusalKind::UnadmittedMaterial` (the registry's own refusal, the id named) — `src/intake.rs`,
      `src/refusal.rs` · *check:* ran — `Axis::ALL.len() == Axis::COUNT`; a census grid prints
      `over 7 axes`; `cargo test --offline` **209 passed, 0 failed**
- [x] T041 The row's **declaration about the world**, as one value: `Requirement<T>` (`Named`, or
      `Owed` — the bar the row cannot name yet) and `Requirements` carrying both dimensions, so
      `Row::new` stays inside clippy's arity rule **without a suppression** and no caller can pass one
      dimension without the other being asked for; the row's answers (`requires_stages`,
      `requires_materials`, `requirements`) read them back — `src/intake.rs` · *check:* ran — the
      seam's three non-pass states are **distinct** and none is a pass: requires-nothing `inactive`
      (reported), declared-and-owed `inactive` + `axis_inactive`, declared-and-unnamed `unchecked` +
      `axis_unchecked`
- [x] T042 The **coupling to the owners' seams**, borrowed rather than copied:
      `WorldDeclarations { roster, registry }` held by `Intake::over(root, world)` (with
      `Intake::set_root` moving the *content* without moving the *world*), and `Gate::new(roster,
      registry, ladder, residency, identity, workers)` — a gate cannot be built without a registry,
      because a row that requires materials would otherwise have nothing to be judged against;
      `reference::registry()` is the process's **one** registry (a `OnceLock`), the same declaration
      the ladder's palette is drawn from — `src/intake.rs`, `src/bridge/gate.rs`, `src/reference.rs` ·
      *check:* ran — two declarations are equal by their owners' **digests**, not by pointer; the
      reference gate and the reference registry are one world in the binary, the evidence and every
      test; `find ../crates -type f -newer src/intake.rs` returns **0**
- [x] T043 The **served spelling** of a requirement: two arguments of a new kind (`Names`, a `+`-joined
      list of wire-shaped names, whose shape rule refuses a value that names nothing — *nothing is not
      a value of a declared kind*, the way a zero-column patch is not), `Request::names` /
      `Request::material_ids` (an id that is not a `u16` refused `argument_malformed`, the way a level
      that is not a number is), the gate's row build, and `RECEIPT_WIRE_VERSION` `/1` → `/2` because
      an admission's axes travel **positionally** — `src/bridge/{table,request,gate,wire,mod}.rs` ·
      *check:* ran — over the **served binary** (`tests/u6_admit.rs`): a conforming row's conformance
      axis is `active`; a stage nobody declares refuses `stage_undeclared` with `no-such-stage` in the
      message and is **recorded**; a material the registry never admitted refuses
      `unadmitted_material` naming `4242`; a `requires_stages=` that names nothing is
      `argument_malformed` and writes **nothing** (record = 2 events for the two world refusals); and
      the transport's reader now refuses a receipt tagged `/1` rather than reading its six axes as
      seven
- [x] T044 The **tests, the evidence and the documents** of this pass: `tests/u6_intake.rs`
      (conformance at the seam, public API), `tests/u6_bridge.rs` (the table's optional arguments),
      `src/bridge/tests.rs` (the only optional arguments in the table are the two conformance ones),
      `check_u6_intake` in `src/evidence.rs` (conforming row, the two refusals by name, the three
      states, and the **owed** line), and the five documents this pass corrects (`src/intake.rs`'s
      header, `src/systems.rs`'s U6 entry, `README.md`, this file, and `../docs/CLEAN-ROOM-PLAN-2026-10-11.md` §21) ·
      *check:* ran — `cargo run --offline` exits **0** at **276** lines (was 270: 5 lines changed, 6
      added, **0** outside U6, and every other line byte-identical — §21 accounts for each), and the
      suite is green with the new behaviour and **red** without it (the axis reverted to
      reported-only: 2 tests fail, naming the axis they no longer see judged)

---

## Phase 11 — the policy chokepoint: one decision function every served row passes through (2026-10-12)

Specified by §2's **U6** entry, §5 row 5's bar and §16 row 12's clause in
`../docs/CLEAN-ROOM-PLAN-2026-10-11.md`, and derived from `docs/HYPERGRILLING.md` **§L-5** (*the
single chokepoint*), `docs/LEDGER.md` **R-62.3** and **§67.4** (*the chain write needs authority*).
The legacy `crates/voxel-package/src/policy.rs` is the **witness** and was **not read** (P7).
Everything below was measured by running it.

- [x] T045 The **closed vocabulary and the matrix** in one new module: `Scope` (3 declared) and
      `Action` (13 declared), each with `ALL`/`COUNT`/`name()`/`parse()` pinned to each other by a
      test; `PermissionClass` with `AlwaysAllowed`, `Gated(Scope)`, `AlwaysDenied(&'static str)` and
      `NeverStore`; `permission_of(action) -> PermissionClass` as an exhaustive `match` with **no
      wildcard arm**, so a missing entry is `E0004` rather than a runtime deny; `check_matrix()`
      refusing an act that describes nothing by name (`action_undescribed`) — `src/policy.rs` ·
      *check:* ran — `cargo test --offline` **351 passed, 0 failed**; the matrix prints `13 act(s) …
      6 allowed, 4 gated, 2 denied, 1 rejection` and the classes account for every act
- [x] T046 **Grants scoped and attributed, and one decision function**: `Grant::of(granter, scope)`
      refusing a blank granter (`grant_unattributed`) and a scope that is undeclared or `everything`
      (`grant_unscoped`) **at construction**; `Caller::of(actor, grants)` refusing an unnamed actor;
      `decide(&Caller, Action)` as the one function, whose denial **quotes the list** (the act, the
      scope it needs, the scopes the caller holds with who granted each, and the declared scopes) —
      `src/policy.rs`, `src/refusal.rs` (`RefusalKind::COUNT` **81 → 86**) · *check:* ran — the four
      construction faults are refused by name in the binary and in the tests; `everything`, `execute`,
      `*` and `all` are neither acts nor scopes
- [x] T047 **The one call site, and a denial is recorded**: `Tool::action` on every row of `TOOLS`
      (eight rows, four gated: `geometry`, `admit`, `save`, `load`), `Gate::call` consulting `decide`
      **once** before dispatch with the act from the row itself, `Gate::{caller, set_caller}` and a
      `reference::caller()` declared beside the identity and the ruleset so the binary, its evidence
      and its tests act as **one** actor — a gate cannot be built without one — `src/bridge/{table,
      gate,mod}.rs`, `src/reference.rs` · *check:* ran — the reference `operator` is served **6 of 6**
      calls (`save` wrote **496** bytes, `load` read them back, `geometry` appended); an actor holding
      nothing is refused `policy_denied` on **all four** gated rows **before they dispatch**, each
      denial **recorded** (the record moved by 4 of 4) with the record still verifying
- [x] T048 The **tests, the evidence and the documents** of this pass: `src/policy.rs`'s seven unit
      tests, `tests/u6_policy.rs` (six, through the public surface and a running gate), `check_u6_policy`
      in `src/evidence.rs` (seven printed lines), `README.md`, `src/systems.rs`'s U6 and U12 entries,
      this file, `spec.md`, `plan.md`, `research.md`, `quickstart.md`'s Scenario 8,
      `contracts/u6-seams.md`'s C5, `checklists/requirements.md`, and
      `../docs/CLEAN-ROOM-PLAN-2026-10-11.md` §29 · *check:* ran — `cargo run --offline` exits **0** at
      **494** lines (was 487: 3 lines rewritten in place, 7 added, **0** outside U6's own section and
      the two registry lines it corrects — §29 accounts for each), and each new guard was perturbed
      one value at a time, observed to fail the slice's own bar **by name**, and restored byte-exact
      with the digests recorded (§29's guard table)
