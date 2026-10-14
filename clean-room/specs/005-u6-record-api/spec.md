# Feature Specification: U6 — The record and the API layer (the gate)

**Feature Branch**: `005-u6-record-api`

**Created**: 2026-10-11

**Status**: Draft — **written from the record, not from the legacy code.** The behaviours in this spec
come from `docs/DESIGN-SPEC.md` §2 F1 (*"replay inputs recorded, not derived"*; the world and the
record are one truth), F2 (*"source admission"*), the **U6** entry of
`docs/CLEAN-ROOM-PLAN-2026-10-11.md` §2 (`admit/inspect -> Established`, `bridge::TOOLS`, and
*"`bridge::call` returning a `Receipt` naming which checks ran"*), and slice 5 of its §5. No `.rs`
under `crates/voxel-package/` or `crates/voxel-mcp/` was read. The **one rule** is inherited whole:
*refuse, never default* — so a request that cannot be honoured is refused **by name**, never answered
vaguely.

**Input**: User description: "U6 the record and the API layer — what the record holds, how a request
enters and what a caller receives back, how an answer carries the evidence for itself, and how a
request that cannot be honoured is refused by name rather than answered vaguely."

**Governed by**: `.specify/memory/constitution.md` (P1–P7, read live) and
`docs/CLEAN-ROOM-PLAN-2026-10-11.md` §2 *U6 — The record and the API layer (the gate)*, slice 5 of §5.

## What U6 is, and what it is not

U6 is the **outward door**: the one declared way a caller (a mod, an agent, a tool transport) asks the
world something and gets an answer it can check. Three boundaries are inherited whole:

1. **The record is the chain (SPEC-3, one world authority).** U2 already owns the append-only
   `Chain`; U6 neither wraps it in a second log nor keeps its own state. A call that changes the
   record appends **through the chain's one writer**; a call that changes nothing appends nothing.
2. **The owners answer, the gate dispatches.** World truth is U2's, the fields are U3's stages, the
   geometry is U4's mesh. U6 reads them **through their own seams** — `World::truth_digest`,
   `field_at`, `Mesher::derive`, `Resident::serve` — and holds no second copy of any of them.
3. **An answer carries the evidence for itself.** A `Receipt` names the checks that ran and the
   claims the answer makes, and every claim can be re-checked **against the thing it describes**
   (`Receipt::verify`). An answer that cannot be checked is not an answer this layer hands out.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A request enters, and the answer comes back with its evidence (Priority: P1)

A caller hands the gate **one declared request line** — a verb from the closed table plus its declared
arguments (`truth`, `fields field=elevation at=0,0,0 side=8`, `geometry chunk=0,0,0 level=0`, `held`,
`record`). The gate parses it against the table, dispatches it to the owner whose seam answers it, and
returns a **receipt**: the answer, the checks that ran, the claims the answer makes, and what the call
changed.

*(Corrected 2026-10-12: the table's **sixth row** — `admit`, whose two references arrive as
`path + digest` and are read in place — has since landed, and so has the transport that serves the
whole table. See `tasks.md`'s Phase 9 and §19 of `docs/CLEAN-ROOM-PLAN-2026-10-11.md`; the rest of this
specification is unchanged, and the five rows it declares answer exactly as they did.)*

*(Corrected again 2026-10-12: an admission is judged on **seven** axes, not six — the last is
`conformance`, which asks whether the world declares what the row requires (`requires_stages` against
U3's `Declared`, `requires_materials` against U2's `MaterialRegistry`, both **optional** arguments whose
absence is the declaration *this row requires nothing*). See `tasks.md`'s Phase 10 and §21 of
`docs/CLEAN-ROOM-PLAN-2026-10-11.md`. The six axes the earlier slice declared keep their places and
their answers; a new column is **appended**, because a verdict's axes are positional.)* Nothing about the request, the arguments or the checks is implicit — an interface **declared,
not described**.

**Why this priority**: it is the whole point of the layer. Without a receipt whose claims can be
checked, every later system (U7's decision layer, U8's shell, any transport) stands on answers nobody
can audit.

**Independent Test**: call each verb through the crate's public API, print the answer and its digest,
and run `Receipt::verify` against the same world and chain — every claim holds.

**Acceptance Scenarios**:

1. **Given** a world, a roster, a gate and a chain, **When** `truth` is called, **Then** the answer
   carries U2's own truth digest and the receipt's claim names it.
2. **Given** the same state, **When** `geometry chunk=0,0,0 level=0` is called, **Then** U4 derives the
   chunk (real work, counted), the chain grows by one event that names the mesh digest, and the
   receipt's claims include that the mesh is **held** with those quads and that event.
3. **Given** the answers, **When** `Receipt::verify` runs, **Then** every claim is re-checked against
   the owners' seams and the call refuses nothing.
4. **Given** a receipt from an earlier world, **When** it is verified against a world that has since
   moved, **Then** verification refuses by name (`claim_unverified`) and says both the recorded and
   the found value.

### User Story 2 - One request twice is one answer, and the record shows what changed (Priority: P2)

The same request, twice, produces the **same answer** — the answer digest is a pure function of what
the answer describes, so a cache hit and a derivation agree. What differs is the **effect**: the first
call did the work and appended an event; the second did no work (the geometry is resident) and appended
nothing, because **the record records changes, not reads**.

**Why this priority**: this is what makes the layer usable by an agent loop: an identical question has
one answer, and "what changed" is a separate, checkable fact rather than something a caller infers.

**Independent Test**: call `geometry` twice; assert one `answer_digest`, two different effects (work
columns `1024` then `0`; appended `Some(digest)` then `None`), and that the chain's length moved once.

**Acceptance Scenarios**:

1. **Given** resident geometry, **When** an identical `geometry` request repeats, **Then** the answer
   digest is identical and the effect shows no work and no append.
2. **Given** two spellings of one request (`truth` and `truth ` with noisy whitespace), **When** both
   are parsed, **Then** their canonical lines are one value and their answers are one digest.
3. **Given** a request that changes nothing (`truth`, `held`, `record`), **When** it is called,
   **Then** the ledger's length and head are unchanged and the receipt says so.

### User Story 3 - A request that cannot be honoured is refused by name, and the refusal is on the record (Priority: P3)

Two kinds of refusal, kept apart on purpose. A request that **does not parse** against the declared
table (an undeclared verb, an absent or malformed argument, a duplicate argument) is refused before the
gate ever sees a world — and **writes nothing**, because a malformed line is not an event about the
world. A request that parses but cannot be honoured **by the world** is refused by the world's own name
(`geometry_out_of_residency`, `level_off_ladder`, `field_undeclared`) and that refusal is **recorded**,
because it is a fact about the world.

**Why this priority**: the one rule is *refuse, never default*. A gate that answers vaguely is worse
than one that refuses, and a gate that silently drops a refusal breaks F1's "recorded, not derived".

**Independent Test**: parse a bad line and assert the chain is untouched; call a request the world
cannot honour and assert the chain grew by exactly one `refusal_recorded` event naming the refusal,
then verify the chain still verifies.

**Acceptance Scenarios**:

1. **Given** an undeclared verb, **When** it is parsed, **Then** the refusal is `tool_undeclared`, and
   no world or chain is touched at all.
2. **Given** a chunk outside the declared residency horizon, **When** `geometry` names it, **Then** the
   refusal is `geometry_out_of_residency` (the owner's own name, passed through), the chain grew by
   one event naming it, and nothing was cached.
3. **Given** a claim that does not hold, **When** the receipt is verified, **Then** the refusal is
   `claim_unverified` with the claim named and both values.
4. **Given** a refusal whose text cannot ride the chain's wire form, **When** the gate tries to record
   it, **Then** the returned refusal says the record could not record it **and** still names the
   original refusal — neither fact is dropped.

### Edge Cases

- **An empty request line**: refused (`tool_undeclared`) — it names nothing.
- **An argument the table does not declare**: refused (`argument_undeclared`) with the argument named.
- **A declared argument that is absent**: refused (`argument_absent`).
- **An argument that is not of its declared kind** (`chunk=0,0`, `level=lots`, `field=nope`): refused
  (`argument_malformed` for a shape, `field_undeclared` for a name no stage declares). A declared list
  whose value names nothing (`requires_stages=`) is a shape fault too (`argument_malformed`).
- **A row that requires what the world does not declare**: refused `stage_undeclared` **with the stage
  named** (and the roster's own names in the message) for a stage U3's roster does not run, and
  `unadmitted_material` — the registry's own refusal, id named — for a material U2's registry never
  admitted. A requirement that is **owed**, or declared and left unnamed, is never a pass either:
  `axis_inactive` and `axis_unchecked` respectively, reachable through the intake's seam.
- **A duplicate argument**: refused (`argument_malformed`, naming the duplication) — the table declares
  one value per key.
- **A receipt verified after its geometry was invalidated**: the `MeshHeld` claim does not hold and
  verification refuses by name; it does not silently pass.
- **A verb asked with an argument it does not declare**: refused (`argument_undeclared`), so a verb's
  arity is part of its declaration rather than a habit.
- **The record grows only where a change happened**: reads append nothing (F1's "recorded, not
  derived" cuts both ways — a read is not a change).
- **A ticket's view of the chain**: unchanged from U2; U6 does not restate it.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The tool table MUST be **closed** and bijective: one name per verb, each verb declaring
  its arguments (name, kind, required) and the checks it runs. An undeclared verb MUST be refused by
  name.
- **FR-002**: A request MUST enter as **one declared line**; parsing MUST validate it against the
  table, and the **canonical line** MUST be the request's identity (so two spellings are one request).
- **FR-003**: An answer MUST be derived through the owners' seams — U2's `World` (truth, identity,
  anchor, residency), U3's `field_at`, U4's `Mesher::derive` / `Resident::serve`, U2's `Chain` — and U6
  MUST NOT copy any of their logic or keep a second copy of their state.
- **FR-004**: An answer's digest MUST be a pure function of what the answer **describes** (identity,
  digest, counts), never of the work that produced it: so a derivation and a cache hit answer alike.
- **FR-005**: Every answer MUST carry its **claims**, and verification MUST re-check each claim
  **against the thing it describes**, collecting **every** failure rather than the first.
- **FR-006**: A receipt MUST name **which checks ran**, each with its outcome and its subject.
- **FR-007**: A call that **changes the record** MUST append through the chain's one writer; a call
  that changes nothing MUST append nothing.
- **FR-008**: A world-level refusal MUST be passed through **by name** and recorded; a parse-level
  refusal MUST write nothing.
- **FR-009**: A claim that does not hold MUST be refused by name (`claim_unverified`) with the claim,
  the recorded value and the found value.
- **FR-010**: U6 MUST NOT restate U2's ticket view, U3's roster, U4's ledger or the chain's wire form;
  where it needs a capability below it, the capability is added **at its owner**.
- **FR-011**: `sha2` MUST remain the crate's only dependency, and no `crates/*` path may be imported
  (P7).
- **FR-012**: `cargo test --offline` MUST pass, `cargo clippy --offline --all-targets -- -D warnings`
  MUST be clean, and the binary MUST print U6's measurements and exit 0.
- **FR-013**: An admission MUST be judged on **seven** named axes, the seventh asking whether the
  world's own declarations hold what the row requires — against U3's `Declared` for a stage and U2's
  `MaterialRegistry` for a material, never against a list this layer keeps — refusing **by name with
  the missing item**, and reporting (never counting as a pass) a row that requires nothing, a
  requirement it declares and owes, and one it declares and leaves unnamed.

### Key Entities *(include if this feature involves data)*

- **`Tool`** (new): one row of the closed table — name, declared arguments, what it answers, whether
  it may change the record, and the checks it runs.
- **`Argument`** (new): a declared name, its **kind** (`Chunk`, `Level`, `Field`, `Voxel`, `Side`, and
  — since 2026-10-12 — `Path`, `Digest`, `Name`, the three a by-reference row needs, and `Names`, a
  `+`-joined list) and whether it is required. `admit`'s `requires_stages`/`requires_materials` are the
  table's only two **optional** arguments: a row that requires nothing of the world declares that by
  omitting them.
- **`Request`** (new): the parsed request plus its **canonical line**.
- **`Answer`** (new): what the owner answered — the world's identity and truth, the fields' value
  digest, the mesh's facts, the held geometry's digest, the ledger's state — with a digest that is a
  pure function of those facts.
- **`Claim`** (new): one thing the answer asserts, with the seam that re-checks it.
- **`Check`** (new): one decision the gate made, named, with its outcome and subject.
- **`Effect`** (new): what the call changed — the work U4 counted, the held count before and after, the
  ledger's length and head before and after, and the appended event (if any).
- **`Requirement`/`Requirements`** (new, 2026-10-12): what a row requires of **one** world (a stage
  the roster runs, a material the registry admits — `Named`, or `Owed`) and the pair of those two
  declarations a `Row` carries. `Intake::over(root, WorldDeclarations { roster, registry })` is how the
  owners' seams reach the axis: **borrowed**, never copied.
- **`Receipt`** (new): the answer, its claims, the checks that ran, the effect, and the request that
  produced it — the record of one call, and the unit `verify` checks.
- **`Gate`** (new): the thing a caller hands a request to. It owns U4's `Resident` (not a copy of it)
  and the declared roster/palette/horizon its verbs dispatch through.
- **The chain** (exists, U2): the record. U6 appends to it and reads it; it is never mirrored.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001** Every declared verb is called once through the public API **and** from the binary, and
  every answer verifies. *Decided by:* the printed per-verb lines (answer digest, claims, checks) and
  `verify` returning no failures.
- **SC-002** One request, one answer: the same request twice prints **one** answer digest, while the
  two effects differ (work `1024` then `0` columns; appended `Some` then `None`). *Decided by:* the
  two printed receipts.
- **SC-003** A refused request is named and recorded: the printed refusal carries the owner's own name,
  and the ledger grew by **exactly one** event naming it, still verifying. *Decided by:* the printed
  refusal plus the ledger's before/after length and head.
- **SC-004** An answer is shown to correspond to the world: a receipt verified against the world it was
  made in passes, and the **same receipt** verified against a moved world refuses `claim_unverified`
  with both values. *Decided by:* the two verification lines.
- **SC-005** The record shows what changed: for every call the printed ledger delta says whether the
  call appended (and what) or changed nothing.
- **SC-006** `cargo test --offline` passes, `cargo clippy --offline --all-targets -- -D warnings` is
  clean, and `cargo run --offline` exits 0 with U6's measurements.

## Assumptions

- **The transport** *(corrected 2026-10-12: it has since landed — `src/transport.rs`, `serve` mode,
the declared line protocol in `contracts/u6-transport.md`; the tool table was the contract and is now
also the thing served)*. Serving it over stdio (the legacy's `voxel-mcp` transport) is a separate
capability, and it was named owed rather than stubbed until it was built.
- **The content gate is owed** *(corrected 2026-10-12: its first slice has since landed in
  `src/intake.rs` — `admit`/`inspect` returning `Established`, a declared row consumed in place by
  reference, and the census with an unchecked axis never a pass).* What remains owed: the document
  record's compression, the structural judge, the ballot, the census over the tree, the policy matrix,
  the recipe format and the API document — all U6's, and all named owed in the plan rather than filled
  in. This slice lands the **request/answer/receipt** path, which every one of them would be served
  through. Depth over breadth. *(Corrected 2026-10-12, §29: the **policy matrix** has since landed —
  `src/policy.rs`'s one decision function, the closed action vocabulary, one class per act with no
  wildcard arm, grants refused construction without a granter or a scope, and every denial recorded;
  see [contracts/u6-seams.md](./contracts/u6-seams.md) C5. What remains of it is the **self-describing
  protocol** and the tool-surface narrowing over a mod-supplied tool list.)*
- **What the world looks like is the user's.** The gateway's *mechanism*, its table, its refusals and
  its receipt are this feature's; the palette, the terrain and the horizon declarations are the
  caller's.
- **Verification is by re-reading the owners, with one exception.** A field's values are the only claim
  that is *re-evaluated*, because a field has no stored form: the world is a function of
  `(seed, period, tick)` and a function is checked by evaluating it. Every other claim reads what is
  held (the resident mesh, the chain's events, the world's digest).
