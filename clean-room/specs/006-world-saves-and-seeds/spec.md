# Feature Specification: U9 — Saves and loads, and the world's own seed systems

**Feature Branch**: `006-world-saves-and-seeds`

**Created**: 2026-10-12

**Status**: Draft — **written from the record, not from the legacy code.** This spec exists because the
mission's completeness clause names *"world saves and loads with their own seed systems"* as a system
the plan's eight did not carry; the decision that admits it is
`docs/CLEAN-ROOM-PLAN-2026-10-11.md` **§16** (*U9*, row 9 of §5). Its behaviours come from
`docs/DESIGN-SPEC.md` §1.3 (determinism) and §2 **F1** (*"replay inputs recorded, not derived"*), **SPEC-3**
(one canonical world authority); `LEDGER.md` **C-96.1** (*the seed is a property of a world* — **a save is
record + seed + tick**); the determinism test `docs/MISSING-SYSTEMS.md` §1 states (*a ticked world and a
rebuilt world produce one digest, whatever the worker count*); `docs/CIVILIAN-PORT.md`'s H-7.20 line
(`the save is record + seed + tick`) with its `GeneratorIdentity` (`seed, ruleset, revision`); and
`docs/GAIA.md` §5c/§11 (*a save stays portable across mod changes because each record names its ruleset
identity*). **No `.rs` under `crates/` was read.** The one rule is inherited whole: *refuse, never
default* — a save that cannot be read back exactly is **refused by name**, never loaded approximately.

**Input**: User description: "world saves and loads with their own seed systems — what a world writes
down, what a load must reproduce, and every seed a save must carry to be exact rather than approximate."

**Governed by**: `.specify/memory/constitution.md` (P1–P7, read live) and
`docs/CLEAN-ROOM-PLAN-2026-10-11.md` **§16.1 U9** and **row 9 of §5**.

## What U9 is, and what it is not

U9 is the **written form of a world**: the one declared way a world leaves memory as bytes and comes
back as the same world. Four boundaries are inherited whole and are what keep it from becoming a second
truth:

1. **A save is the record plus identity, never a second world (F1, SPEC-3).** The plan already rules
   that *replay inputs are recorded, not derived*, and that there is **one** canonical world authority
   — U2's append-only `Chain` plus `WorldIdentity`. A save is therefore `identity + record + the seeds
   the world derives from`; it is not a snapshot of state U2 owns, and a load does not create a second
   record.
2. **The seed is a property of a world, and the tick is not the seed (C-96.1).** A world is *"a seed, a
   tick and a configuration held together"*; moving the tick moves the tick and **leaves the seed
   alone**. A save that dropped the tick, or that re-derived its seed from a constant rather than the
   world it was written from, would make every one of its claims false — the drift C-96.1 caught while
   the panel was reporting the world's identity.
3. **A load is exact or it is refused.** The exact/approximate boundary of `DESIGN-SPEC` §2.1 is
   binding: mass, volume, component lineage, fluid volume and mass, and lifecycle counters must
   round-trip **exactly**, and no approximate quantity may be written as though it were one of them.
4. **Worker count is a performance knob (P3).** Determinism is a contract: a world rebuilt from a save
   and a world ticked live to the same tick produce **one digest**, at any worker count.

## What this slice waits on — and what it does not

Stated plainly, because it decides whether this system can start:

- **Hard prerequisite: none is owed.** U2's `WorldIdentity`, `Tick`, `Chain`, `Digest`, `Domain` seeds
  and `Refusal`/`RefusalSet` are built and measured (§7 of the plan), and U3's determinism is built and
  measured (`460214b1…2fc6` at 1/2/4/8 workers). That is the whole of what a save in its **replay
  form** needs.
- **U6's content gate is ADJACENT, not a prerequisite.** `admit`/`inspect → Established`, the document
  record, the structural judge, the ballot, the census, the policy matrix and the API document are
  **not** needed for a save that is record + seed + tick. *(2026-10-12: the gate, the census and the
  **policy matrix** have since landed (§18–§22, §29 of the governing plan); the judge, the ballot, the
  API document and the matrix's own remainder are still owed, and this reading is unchanged.)* The gate becomes **hard** only for the *other*
  form of a save — one that crosses the API as a **document consumed in place by reference**
  (`path + digest`) — which this spec names as owed rather than lands.
- **U4's residency is not involved at all.** A save is the record, not the resident set: the resident
  set is a cache and truth is independent of it (U2's `truth_digest` excludes it).
- **The door is U6's.** Exposing `save`/`load` as rows of the one closed tool table is U6's table, not a
  second one (P5). This slice's surface is the library's, and the door is named owed.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A world written down reads back as the same world (Priority: P1)

A world is written to bytes and read back. The loaded world has the **same identity**, the same record
and the same truth: its `truth_digest` equals the digest of the world it was saved from at the same
tick, and it equals the digest of a world **ticked live** to that tick. Nothing about the world is
recomputed approximately and nothing is dropped.

**Why this priority**: it is the whole point of the system, and it is the instrument every other system
needs — the planet's residency, an organisation's history and the content tree's rows all have to be
pinned to a world before they can be judged.

**Independent Test**: save a world, load it, print both digests and the live-ticked digest; the three
are one value, and the same holds at 1, 2, 4 and 8 workers.

**Acceptance Scenarios**:

1. **Given** a world at `(seed, period, tick)`, **When** it is saved and loaded, **Then**
   `truth_digest(loaded) == truth_digest(saved)`.
2. **Given** the same save, **When** it is loaded at four worker counts, **Then** one digest is printed
   for all four (P3).
3. **Given** a world ticked live to `t`, **When** a save taken at `t` is loaded, **Then** the two
   digests are equal — the rebuilt world is the ticked world.
4. **Given** a loaded world, **When** the chain is verified, **Then** it verifies as the same record:
   it links to its head, no second record was created, and the wire form round-trips byte for byte.

### User Story 2 - A save carries its own seed systems, so a load is exact rather than approximate (Priority: P2)

A save declares **every** seed the world derives from: the world seed, the declared period, one seed
per closed deterministic sub-stream, and the configuration/ruleset identity the world was made under.
Nothing is inferred from a constant, and nothing is defaulted. A save that omits one is refused by
name rather than loaded with a plausible substitute.

**Why this priority**: this is the difference between a save and a guess. `C-96.1`'s defect was exactly
this failure at one remove — a panel that printed a global constant while the world came from somewhere
else — and a save is the artifact where such a drift becomes permanent.

**Independent Test**: save a world, delete one declared seed/hand-mutate one field, and assert the load
is refused by name naming the missing thing; then assert a well-formed save loads with each sub-stream
seed **equal** to the world's own.

**Acceptance Scenarios**:

1. **Given** a save whose header omits a declared sub-stream seed, **When** it is loaded, **Then** the
   refusal is `seed_undeclared`, naming the sub-stream, and no world is produced.
2. **Given** a save whose identity names a seed different from the world it was written from, **When** it
   is loaded against that world, **Then** the refusal is `identity_mismatch` with both seeds.
3. **Given** two saves at consecutive ticks of one world, **When** both are loaded, **Then** both carry
   the **same** seed and the **same** period and differ only in the tick (C-96.1).
4. **Given** a save naming a configuration/ruleset identity, **When** it is loaded under a different
   one, **Then** the refusal is `identity_mismatch` naming both identities — the save is portable *and*
   explainable (`GAIA`).

### User Story 3 - A save that cannot be trusted is refused by name (Priority: P3)

Bytes that are not a save, bytes that have been altered, a truncated save, and a load that cannot be
honoured are each refused **by name**, and the refusal is the world's own name for the problem rather
than a vague failure. A refusal never yields a partially loaded world.

**Why this priority**: the one rule is *refuse, never default*. A loader that half-applies a save is
worse than one that refuses it, because the result is indistinguishable from a correct world.

**Independent Test**: load random bytes, a save with one flipped byte, and a truncated save; assert
three distinct refusal names and that nothing was constructed in any case.

**Acceptance Scenarios**:

1. **Given** bytes that are not a save, **When** they are loaded, **Then** the refusal is `not_a_save`.
2. **Given** a save with a single flipped byte, **When** it is loaded, **Then** the refusal is
   `save_tampered` (the digest), and no world is produced.
3. **Given** a truncated save, **When** it is loaded, **Then** the refusal is `save_truncated`.
4. **Given** a save whose record cannot be replayed to its own tick, **When** it is loaded, **Then** the
   refusal names the record (`record_diverged`) rather than loading a shorter world.

### Edge Cases

- **A save at tick 0** is a save: the record may be empty, and the world is the identity alone.
- **A world with no events yet** saves and loads to the same truth digest; an empty record is not a
  reason to refuse.
- **A seed that is zero** is a seed, not an absence — zero is refused only where the quantity is
  required to be non-zero, and the seed is not one of those.
- **Two saves of the same world at the same tick** produce **identical bytes** (determinism, P3).
- **A load into a process that already holds a world** does not mutate the save's bytes; the bytes are
  input, never a second authority (SPEC-3).
- **A population past its ceiling** refuses rather than drops (F1's third clause) — a load that cannot
  carry its population is refused by name.
- **A save written under one `sha2`-digested encoding and read by a future revision** must refuse
  (`not_a_save` or a named version refusal) rather than reinterpret the bytes.
- **A save that names a sub-stream seed the receiver's roster does not declare** is refused
  (`seed_undeclared`) — a roster that shrank must not silently drop a stream's identity.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: A save MUST be **one declared encoding** of `identity (seed, declared period, tick)`, the
  record, the seed systems' declarations, and the configuration/ruleset identity. The encoding MUST be
  **exact**: save → load → save MUST reproduce identical bytes.
- **FR-002**: A load MUST reproduce the world such that `truth_digest` equals the saved world's, **and**
  equals a world ticked live to the same `(seed, declared period, tick)`.
- **FR-003**: The world identity MUST be **immutable** across a save/load: a load MUST NOT adjust, clamp
  or repair an identity, and MUST NOT depend on worker count.
- **FR-004**: A save MUST declare **every** seed the world derives from — the world seed, the declared
  period, and one seed per deterministic sub-stream — and a save missing one MUST be refused by name
  (`seed_undeclared`), never loaded approximately.
- **FR-005**: A save MUST name the configuration/ruleset identity it was written under; a load under a
  different identity MUST be refused by name (`identity_mismatch`) naming both.
- **FR-006**: Moving the tick MUST leave the seed alone: two saves at consecutive ticks of one world
  MUST share seed and period and differ only in the tick.
- **FR-007**: The quantities `DESIGN-SPEC` §2.1 marks **exact** (mass, volume, component lineage, fluid
  volume and mass, lifecycle counters) MUST round-trip exactly, and no approximate quantity may be
  written as one of them.
- **FR-008**: A refusal MUST be **by name** and MUST NOT yield a partially loaded world; the names
  `not_a_save`, `save_truncated`, `save_tampered`, `seed_undeclared`, `identity_mismatch` and
  `record_diverged` MUST each be reachable, and a set of problems MUST be collected rather than the
  first.
- **FR-009**: The record MUST remain the one `Chain`: a load MUST NOT create a second record, and the
  loaded chain MUST verify (links to head, wire form round-trips).
- **FR-010**: U9 MUST NOT add a second decision matrix, a second permission model or a second verb
  table (P5); serving `save`/`load` through the one declared tool table is U6's door and is named owed.
- **FR-011**: U9 MUST NOT import, `use` or path-depend on any crate under `crates/` (P7), and `sha2`
  MUST remain the crate's only dependency.
- **FR-012**: `cargo test --offline` MUST pass, `cargo clippy --offline --all-targets -- -D warnings`
  MUST be clean, and the binary MUST print U9's measurements and exit 0.

### Key Entities *(include if this feature involves data)*

- **`Save`** (new): the declared written form — identity, record, seed declarations, configuration
  identity — with an exact encoding.
- **`SaveIdentity`** (new): `seed`, declared period, tick, and the configuration/ruleset identity the
  save was written under.
- **`SeedDeclaration`** (new): one named deterministic sub-stream and **its own** seed, as the save
  carries them.
- **`LoadReport`** (new): what a load reproduced — the identity, the record's length and head, and the
  truth digest — the thing the acceptance checks read.
- **`SaveRefusal`** / **`RefusalSet`** (new kinds over the existing vocabulary): the named refusals
  above, collecting every problem rather than the first.
- **The world and its identity** (exist, U2): `WorldIdentity`, `Tick`, `World`, `Chain`, `Digest`,
  `Domain` — U9 reads them through their own seams and keeps no copy.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001** A written world reads back as the same world: save → load prints **one** `truth_digest`,
  equal to the live-ticked world's, and identical at **1/2/4/8** workers. *Decided by:* the printed
  digest lines from the binary and the crate's own test.
- **SC-002** The seed systems are the save's, not a constant's: every declared sub-stream seed in the
  loaded world equals the saved one, and a save missing one is refused by name. *Decided by:* the
  printed per-stream seeds and the `seed_undeclared` refusal.
- **SC-003** Identity is immutable and named: a save loaded under a different ruleset/configuration
  identity is refused `identity_mismatch` with both identities printed. *Decided by:* the refusal line.
- **SC-004** The tick is not the seed: two saves at consecutive ticks share seed and period and differ
  in the tick. *Decided by:* the two printed identities.
- **SC-005** Untrusted bytes are refused: random bytes, one flipped byte and a truncated save each
  produce their own named refusal and **no** world. *Decided by:* the three refusal lines.
- **SC-006** The record is one: the loaded chain verifies, its head and length match the saved ones, and
  the save's bytes are unchanged after a load. *Decided by:* the printed before/after record state.
- **SC-007** `cargo test --offline` passes, `cargo clippy --offline --all-targets -- -D warnings` is
  clean, and `cargo run --offline` exits 0 with U9's measurements.

## Assumptions

- **The replay form is this slice; the document form is owed.** A save that crosses the API as a
  document consumed in place by reference (`path + digest`) is named as a later extension, and **that**
  is where U6's content gate becomes a hard prerequisite — not here. *(Corrected 2026-10-12, §28: the
  document form has since landed. It is read in exactly the place this assumption names — through U6's
  content gate, which is a **hard** prerequisite for it — and it is reached through the table's own
  `save`/`load` rows, so this slice is now the whole of both forms. The sentence above is kept as the
  ruling that placed the work; §28 and `contracts/u9-seams.md` C4 carry what changed.)*
- **The format is this system's to declare.** No legacy save format is the contract (P7); if one ever
  becomes the contract, it is cited by `path + digest`, never carried.
- **What the world looks like is the user's.** The seed values, the period, the configurations and the
  ruleset identities are the caller's; U9's own subject is that they are **carried and checked**.
- **U6's door is not opened here.** The tool table is closed at five verbs today; adding `save`/`load`
  rows is a U6 change, named owed rather than smuggled in (P5). *(Corrected 2026-10-12, §28: those two
  rows **have** since been added — as the table's seventh and eighth, by reference, in U6's own table —
  so it is closed at **eight** verbs and this slice's document form is served through it rather than
  beside it.)*
- **The plan's U5 is untouched.** This spec does not claim instruments, light or audio.
