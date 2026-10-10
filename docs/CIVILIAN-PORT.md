# The civilian port — the inventory, and the verdict per file

**BLD-0221.** The civilians' whole process is ported into `voxel-model` and wired into laya; the
scope is not reduced (`event-0127`). This document is the port's own inventory: what is copied,
what is **not** copied, and why — so the next reader does not rediscover either.

**Corrected 2026-10-02 (`event-0140`, `event-0141`).** Three claims below were wrong, and they
are fixed where they occurred rather than silently: the world section said the governed vocabulary
is *"implemented against `voxel-shapers::earthworks`"* — impossible under the dependency direction
(`voxel-shapers → voxel-model`, and `sha2` is named by `voxel-model` and no other crate,
`docs/LEDGER.md:1994`); the set table said 4,719 lines, written before `matter/` was staged; and
the world section's first version of the handle said *"no store of its own"*, written before
`apply_transition` was read — the book is real, and what must not exist is a second record
(`event-0141`). The world section and the status section are the corrected versions.

## The method, in five rules

1. **Inventory before copy.** Before a block is copied, this tree is asked what it already owns.
   `gov/season.rs` was copied before that question was asked and was wrong twice over —
   it is removed, and the season section below has the mapping (`event-0131`, strike 8).
2. **Copied whole, seams only.** The precedent is `shploingus-game`'s `legacyledger/mod.rs`:
   *"Six files copied whole … every line of the state machine, the evidence ladder and the
   boundary enforcement is the original's."* The machine is never rewritten by hand.
3. **One codec.** The port's JSON boundary is rewritten onto `CanonicalCodec`/musli; serde and
   `serde_json` are not admitted (`event-0129`), so §10's zero new deps and D111 both hold.
4. **Staged, then switched.** The port sits undeclared (the tree stays green, measured) and the
   switch is one line — `pub mod civilian;` in `crates/voxel-model/src/lib.rs` — added when the
   set compiles. The failure mode is loud and reversible, not append-only.
5. **Copy the machine, map the world.** The civilians *process* is copied whole. The **world it
   acts through is this tree's own generator** (`crates/voxel-shapers`, contracted by
   `docs/SHAPERS.md`) and is never copied beside it. Where the ported code calls into the world,
   the seam is a capability the generator may be **missing** — read what the source does whole,
   then build the capability here (`event-0133`, strike 9, and the world section below).

## The ala-cities set: copied whole, staged, undeclared

| file | lines | what it is |
|---|---|---|
| `agency/action.rs` | 1407 | `GovernedWorld`, `Blocker` (30 variants), `VerbPolicy`, per-actor-verb authority, the single commit point |
| `agency/actor.rs` | 464 | `Citizen` (needs, role, skills, lifecycle), **`Needs { nourishment, rest, safety }`** — all three `Rational` |
| `agency/evidence.rs` | 55 | `AgencyEvidence`, `ConsequencePolicy`, `WorkConsequence` — owner, consent, compensation |
| `agency/mod.rs` | 26 | the set's own re-exports — where the machine names `crate::world::{PhysicalVerb, WorldTransition, ActorId, ReservationId, TargetRef, TaskId}` |
| `agency/selection.rs` | 380 | `SelectionPlan`, `SelectionBudget`, cuboid selection, estimate-before-work, reserve cells |
| `agency/task.rs` | 64 | `WorkTask`, `ToolId`, `TaskStatus` |
| `gov/mod.rs` | 1823 | the governor: decide-before-the-world-changes, ticket kinds and statuses, retirement reasons, the evidence ladder, late evidence appends a correction |
| `matter/mod.rs` | 135 | the fluid vocabulary a governed change is spelled in (`FluidSpecies`, `Phase`, `ReservoirAccount`, from `fluids.rs:12–72` only) |
| `matter/rational.rs` | 314 | `Rational` — the signed exact `i128` number the machine is written in, copied whole; its refusals are mapped onto `RefusalKind` |
| `sim/citizen.rs` | 157 | the city-facing `Citizen` and `CitizenState` |
| `sim/mod.rs` | 16 | the city-facing module's declaration |
| `sim/task.rs` | 343 | the city-facing task records |

**5,312 lines on disk, 13 files.** The twelve source-copied files total 5,184 (`agency/` 2,396,
`gov/mod.rs` 1,823, `matter/` 449, `sim/` 516); `mod.rs` (128) is this port's own. `gov/season.rs`
(416) was copied in the first pass and removed (`event-0131`); `matter/` was staged when its seam
resolved. **Nothing of the civilians' set is declared in `lib.rs`** — `pub mod civilian;` is absent; only
`matter` is declared, because the governed store names it. So `cargo check -p voxel-model` stays
clean and the port cannot turn the tree red while its seam is being satisfied.

### The seam they import, measured from the sources

| import | where it lands |
|---|---|
| `crate::gov::{Correction, Evidence, Retirement, Ticket}` | inside the set — no work |
| `Rational` (`matter::properties`) | **landed at the crate root** — `src/matter/rational.rs`, copied whole (signed, exact `i128`; refusals mapped onto `refusal::RefusalKind`), plus `matter/mod.rs`'s fluid vocabulary from `matter/fluids.rs:12–72`. Moved out of `civilian/` and **declared** (`lib.rs`: `pub mod matter;`) because the governed store's delta vocabulary names `crate::matter::Rational` — it could not compile behind the switch (`event-0143`). Not a rename of `material::Ratio` — measured: `Ratio` is unsigned (`u64/u64`) and has no `Sub` |
| `crate::materials::generated::{Process, Rational}` | **owed** — one import site (`sim/task.rs:43`, used at 327–341); `materials/*` is 7,405 lines of material science beside this tree's own `voxel_model::material`, read whole before it is mapped (`event-0133`) |
| `crate::world::{…}` — 20 names | **landed and owed, split in the world section below** — the governed-change vocabulary now lives at the crate root, `voxel_model::world` (`event-0138`); the handle and its row types are the next landing |
| `crate::sim::{World, Zone, BuildingKind, SIM_HZ}` | the city sim — the mobs side; read whole before any decision (`event-0133`), never copied beside `voxel-shapers` |

### The world half: the generator already exists, and the vocabulary lands in `voxel-model` (`event-0133`, `event-0138`)

The seam table above once said the `crate::world` block was *to port*. It is not, and the closure
that plan printed shows why: `world/transition.rs` imports `delta`, which imports `matter` and
`fields`; `world/mod.rs` declares fifteen submodules (15,892 lines) and `sim/mod.rs` (5,563) plus
`materials/*` (7,405) come with them — **the world generation**, which the clean tree already has.

| this tree | role |
|---|---|
| `crates/voxel-shapers` | **the world generator** — `docs/SHAPERS.md` is its contract |
| `voxel-shapers::earthworks` | the **commit point**: `Modification { x, y, z, from, to, reason, ledger }` in an append-only `Earthworks` record — *"recorded player/agent modification of the world, each entry carrying its mass ledger"* (SHAPERS.md §1, row 15) |
| `voxel_model::conservation` | the exact `Mass`/`Ledger`/`Source` and the pump declaration |
| `voxel_model::identity::Digest`, `chain` | content identity and the one append-only record |

What ala-cities' `world/transition.rs` does, read whole (825 lines): it is the **governed change of
the world**. `WorldTransition { actor, task, intent (PhysicalVerb), target, reservation, epoch,
nonce }` with a stable digest `id()`; `apply_transition` takes a before-snapshot, refuses a
duplicate, a committed, a missing or an unowned reservation, builds the verb's effects (break and
gather to `carried:actor`, dig to `spoil:actor`, fill and place from `carried:actor`, use and craft
evidence only), audits conservation, reads back `after`, writes evidence rows
(`transition.committed`, `earthworks.delta`, `process.evidence`, `transition.refused`) and returns
**`TransitionReceipt { accepted, deltas, evidence, refusal, audit, read_back, transition_id,
reservation }`**. `World` also carries `reserve`, `has_reservation(_for)`, `cancel_reservation`,
`evidence_for`, `evidence_for_transition`, `apply_delta_batch`.

The civilians call exactly this (`agency/action.rs`, measured): `world.apply` (457),
`world.apply_delta_batch` (993), `world.reserve` (869), `world.evidence_for` (1205),
`world.conservation_snapshot`, `world.has_reservation_for`, `world.cancel_reservation` — and the
receipt's `deltas`/`evidence` are what `ActionReceipt` is built from.

**The gap was measured, and the ruling of 2026-10-02 closed its first half by widening the record
rather than sitting a book beside it.** `Modification` (`voxel-shapers::earthworks`) now carries
the governed fields — `actor: ActorId`, `task: Option<TaskId>`, `verb: PhysicalVerb`,
`reservation: Option<ReservationId>`, `transition: Digest` — and the record itself refuses a zero
transition (`MissingStage`) and a duplicate (`RecordExists`), which is where exactly-once lives. A
hand edit is a governed act by **the player as an actor** (`ActorId::player()`,
`hand_change(HandChange)`, domain `shploingus/earthworks/hand-change/v1`); the seven call sites
were threaded (`voxel-work`'s `pool::Edit` and `stream`, `shploingus`, `voxel-client`,
`voxel-shapers::column`, `voxel-stream::world_source`).

**Where the vocabulary lives.** In `voxel_model::world` —
`crates/voxel-model/src/world/{mod,region,transition}.rs`, **at the crate root**, not under
`civilian/`: the record has to name it (`voxel-shapers` imports it today), and behind the
civilians' switch it would have been unnameable until the whole port compiled. The earlier claim
that the vocabulary was *"implemented against `voxel-shapers::earthworks`"* was wrong twice over —
the crate graph forbids the vocabulary living in the generator, and it described the capability
gap as the permanent map after the widening had closed its first half. Corrected: **the vocabulary
is `voxel-model`'s, the record is the generator's, and the generator imports the vocabulary.**
**The DSL was changed first** (rule 2): `architecture/workspace.dsl` gained a `world` component
under `game`, the arrow `civilians -> world`, a `GameLayer` component view, and the `TicketPipeline`
line — validated 0 errors, 0 warnings, 6 views, 21 relationships (was 20).

**What the machine imports from `crate::world`, measured** — `agency/action.rs:5–9` (20 names) plus
`selection.rs:5`, `evidence.rs:2,6`, `mod.rs:9,26`, `actor.rs:5`, `task.rs:2`:

| landed (`voxel_model::world`; verified) | owed by the handle (the next landing) |
|---|---|
| `ActorId`, `PhysicalVerb`, `ReservationId`, `TargetRef`, `TaskId`, `WorldTransition`, `TransitionRefusal`, `WorldError` | `World` — the 8 methods measured in use: `apply`, `apply_delta_batch`, `reserve`, `has_reservation_for`, `cancel_reservation`, `evidence_for`, `conservation_snapshot` (plus `has_reservation`) |
| `Digest` (re-export of `identity::Digest`), `Coord`/`ChunkCoord` (re-exports of `coord::VoxelPos`/`coord::ChunkPos`), `SignedRegion`, `FIRST_BAND_MIN_Z`/`FIRST_BAND_MAX_Z`, and — from the store's own vocabulary — `Account`, `ComponentId`, `Delta` (3 variants admitted: `MatterTransfer`, `OccupancyChange`, `EvidenceOutput`), `LineageId`, `Occupancy`, `OccupancyProvenance`, `FluidAddress`, `EarthworksEffect` (`world/change.rs`, `event-0143`), plus the book itself: `Evidence`, `ConservationSnapshot` (governed subset), `ConservationAudit`/`ConservationViolation`, `DeltaStore`, `apply_with_id` + the audit (`world/book.rs`, `event-0144`) | the `World` handle + apply pipeline only. `Evidence` was measured against `ticket::Evidence` before it was named: they are different subjects — the ticket's is `{ id, ticket, origin, detail }` (a claim citing a ticket), the world's is `{ kind, message, digest, transition_id, reservation }` (a change's own account of itself). The runtime-parts half (`WorldFixtureParts`/`fixture_parts`/`restore_runtime_parts`) is excluded per H-7.20. |

The source's `World` is split across `world/projection.rs:283` (the struct and `apply`) and
`world/transition.rs:599–666` (`reserve`, `has_reservation(_for)`, `cancel_reservation`,
`evidence_for(_transition)`, `apply_delta_batch`, `apply_earthworks`, `apply_process_outputs`,
`conservation_snapshot`); the reservation book and the evidence rows live in `world/delta.rs`
(1211–1411).

**The handle is the governed book, and the measurement is what says so.** `apply_transition`'s
`build_effects` reads only the store — `account_for_coord`, `component_at`, `component_amount` —
and **never the generator**: the material state the machine decides against is the book's own
accounts and components (`carried:<actor>`, `spoil:<actor>`, the substrate account a coordinate
belongs to), and the voxels are a **projection** of generator + record (`material_at(spec,
earthworks, …)`), which stays the game's side. An earlier sentence here said *"a handle over the
record, with no store of its own"*, written before `apply_transition` was read — the book is real;
what must not land is a **second record** (`forbidden.second-record`). The durable change is the
earthworks entry, decisions are the chain's, and the book is the working set (`event-0141`).
Identity and baseline (`GeneratorIdentity` — seed, ruleset, revision — and `BaselineSnapshot`) are
the **projection's** side, i.e. `voxel-shapers`' `WorldSpec` (seed, tick, config, C-96.1); measured,
the machine never calls `derive` or `project`.

**`region.rs` is the case where the inventory changed the file.** The first draft copied `Coord`,
`ChunkCoord`, `CHUNK_SIZE` and the band constants as second types; the tree already owns all four
(measured: `coord.rs:54`, `61`, `17`, `29`/`31`). The file now re-exports `coord::VoxelPos as Coord`
and `coord::ChunkPos as ChunkCoord`, derives `FIRST_BAND_MIN_Z = BAND_MIN_Z` and
`FIRST_BAND_MAX_Z = BAND_MAX_Z - 1`, and keeps only `SignedRegion` — which **refuses rather than
clamps**, the shape `forbidden.box-selection` requires. The draft's own assertion
`FIRST_BAND_MAX_Z == coord::BAND_MAX_Z` was **false** (the source's 47 is inclusive; the tree's 48
is exclusive) and is now a test that asserts the difference (`event-0139`).

And the observer half, found in the same read: `agency/action.rs:521` calls
`observer::recorder::record_committed_action` at the single commit point with the real receipt —
recording a `GovernedTicket { verb, actor, authority, reservation, deltas, evidence }`, never
recording a refusal as a ticket, and structurally preventing a test event from carrying authority.
**That ticket is the civilians' evidence, and where it lands is the decision layer — measured, and
not `laya::ProofOfWork`.** The earlier sentence here said the ticket *"is the proof-of-work
shape"* and that its landing is `laya::ProofOfWork` plus `write_the_gate_queue`'s `&[]`; the read
that the user demanded before execution overturned it (`event-0145`). `ProofOfWork` is per
**artifact** — `{ path, claim, origin, evidence }` (`laya.rs:499`) — and `decide` looks it up by
`proof.path == path` (`laya.rs:645`) over the `.rs` files under `crates/`, so it is the **code
gate's** proof; `&[]` means *no artifact carries proof of work* and every artifact fails by default
(`shell/notifications.rs:140`). It is not a slot for world transitions, and filling it from
receipts would be a category error of exactly the kind H-14.1 records — *an agent read "the
oracle" as the whole of Laya, built one trait for it*.

The binding statement for this side is already in `architecture/workspace.dsl`: the `civilians`
container's *"evidence is the proof of work for solutions, **graded against `axis.rs`**"*, and the
decision layer is `decision::Decider` — `Subject -> Assessment`, one vocabulary, *"a missing
instrument is a refusal, never a pass"* — with **one notification per decision** into
`logs/laya-decisions/`. So the civilians' work reaches laya as a **subject of the decision seam**,
graded on the axis ladders and carried to the client by the queue; the `&[]` slice stays the code
gate's until the gate is given its own artifact proofs.

### The season: removed, and mapped rather than renamed (`event-0131`)

`voxel_model::chain::Season` (`chain.rs:87`) is the **world's year**, and its own refusal text
rules out what was copied: *"a season is one of winter, spring, summer, autumn — the world's own
year, **never a governance period, a campaign or a historical boundary** (H-10.5)."* The copied
`Season` was that governance period, and it carried a **second record store** beside the chain
(`index.json`, `season.json`, `events.jsonl`, `tickets/`, `retirements/`, `evidence/`,
`corrections/`) — `forbidden.second-record`.

| ala-cities | this tree |
|---|---|
| the world's season | `chain::Season` — already owned |
| the governance period (`SeasonMeta.id`, `campaigns`) | the namespace's **`campaign`** (`chain.rs:166`) |
| the store | **the chain** — `chains/workspace.chain` |

The governor's machine stays byte-identical, so `gov/mod.rs:21`'s `pub mod season;` stands; **the
file at that path is to be written as the chain-backed seam, not copied**. Its call sites are
measured (`gov/mod.rs` 721–752, 827, 847, 868, 886, 894, 915–916, 975, 1112–1124, 1147–1156), and
two behaviours must survive the rewrite: **ids come from the directory, never from a counter**,
and **a corrupt record loses that record, not the file**.

## The `legacyledger` half: inventoried, and **not copied**

### What it actually is

**Not a civilians file.** It is the **Bonsai agent daemon's governance ledger**
(`shploingus-game/crates/diffusion-core/src/legacyledger/`, 3,028 lines), written against a
different stack: `SOURCE = "agent:bonsai-agentd"`, AWSO_BSPT's chat mailbox and activity log, the
`us_2000_government_corporate_lawmaker_orchestration_lesson.md` §16/§17/§18/§22/§30/§32 schema as
encoded by `agent/src/gov_ledger.py`, and constants that point at
`C:\Users\ala\Bonsai-demo\agent_uploads\*.jsonl`.

### The verdicts, file by file

| file | lines | what it is | verdict |
|---|---|---|---|
| `mod.rs` | 47 | the wrapper + re-exports, written so `use super::*` resolves; names the serde question | **reference** — the serde question is answered (`event-0129`) and the wrapper exists only for the glob |
| `gov.rs` | 939 | the §16 contract (`Envelope`, `Principal`, `Objective`, `TimeWindow`, `Scope`, `Completion`, `Contract`, `Ticket`, `Evidence`, `Board`) and the evidence tiers (`a_primary_artifact` … `d_self_report`) | **map the shapes** — the envelope and the tier vocabulary; its `Gov` JSONL store is a record beside the chain, so the code is reference |
| `kinds.rs` | 619 | the obligation book: `Obligation` (chat / question / commission), `KindBook`, and the rule *a new kind may raise what is owed, never lower it* | **map the shapes** — the obligation axis, not `TicketKind` |
| `ticket.rs` | 658 | AWSO_BSPT's chat mailbox off the **server console**, `OPEN`/`HANDLED`, `Tickets` JSONL, `chat_from_console` | **reference** — this tree owns `voxel_model::ticket` (`TicketId` 62, `TicketStatus` 91, `TicketKind` 175, `EvidenceOrigin` 380, `Ticket` 653, `Board` 913) and the chain is the one record |
| `ledger.rs` | 249 | the audit ledger (`Record`, `Ledger`, `trace_summary`) | **reference** — this tree's audit sink is the chain (`Auditor::step` appends every decision, allowed and denied) |
| `store.rs` | 99 | an append-only JSONL book with **tombstones** (a repertoire must be able to forget) | **reference** as code — the rule is already this tree's CLM: *version / promote / supersede / retire, never delete* |
| `resume.rs` | 417 | §18/§19 claims **derived** from evidence (`allowed_strength`, `claims`), and three rules: strength is computed; a claim with no evidence is not a claim; promotion needs new evidence | **reference for this port** — it is the **résumé** layer, not the civilians container; its home is the résumé window and `axis.rs`'s ladders, and it owes its own block |

**Nothing in this half is copied as code.** Two shape-blocks map, one belongs to another layer, and
four are reference.

### The two shape-blocks, and where they land

* **the §16 envelope** (`gov.rs:102–268`) — scope, authority, rate, term, evidence, completion,
  change control — maps onto `voxel_model::{ticket, envelope, grant}` plus `axis.rs` for the tiers.
  The DSL already names it (`contract` component) and cites this file as its source.
* **the obligation book** (`kinds.rs:35–151`) — chat / question / commission, and the refusal to
  let a new kind carry `Work` — maps onto the console's filing path, and its rule travels with it:
  *the way to make the agent do less is not to give it more vocabulary.*

## Status

* **Staged, undeclared:** the ala-cities set — 13 files, 5,312 lines — with one part **declared**: `matter` at the crate root, because the governed store names it.
* **Landed and verified (`event-0138`, `event-0143`, `event-0144`):** the governed vocabulary and the
  governed book (`voxel_model::world` — `mod.rs`, `region.rs`, `transition.rs`, `change.rs`,
  `book.rs`; `voxel-model` 230 tests, 0 failures),
  `matter` **declared at the crate root** (the governed store names `crate::matter::Rational`), the
  `Modification` widening with its five fields and exactly-once at the record, the DSL change
  (validated 0/0, 6 views, 21 relationships), and the seven call sites threaded. Workspace
  `cargo check` 0 errors; clippy lib+tests clean except the pre-existing
  `voxel-shapers/src/sky.rs:579`; full suite 0 failures.
* **Owed, in order:**
  1. **the `World` handle** — `apply`, `apply_delta_batch`, `reserve`/`has_reservation(_for)`/
     `cancel_reservation`, `evidence_for`, `conservation_snapshot`, over the governed book that
     landed in `world/book.rs`, **no second record**. The machine's measured surface
     is `agency/action.rs:5–9` (20 names) plus `selection.rs:5`, `evidence.rs:2,6`, `mod.rs:9,26`,
     `actor.rs:5`, `task.rs:2`; the vocabulary and the store it keys already landed
     (`world/change.rs`, `world/book.rs`). Source map: `world/projection.rs:283` (struct +
     `apply`), `world/transition.rs:185–560` (`apply_transition`, `preflight_refusal`,
     `apply_prepared`, `refused*`, `build_effects`) and `599–666` (the reservation/evidence/
     batch methods), all over `world/delta.rs`'s store (landed). The runtime-parts half stays
     out: the save is record + seed + tick (H-7.20).
  2. **the chain-backed seam at `civilian/gov/season.rs`** — ids from the directory, never a
     counter; a corrupt record loses that record, not the file.
  3. the boundary onto `CanonicalCodec` (4 files, 10 JSON call sites, the `#[serde(default)]`
     attributes).
  4. the switch — `pub mod civilian;` in `lib.rs`.
  5. the laya wiring — `write_the_gate_queue`'s `&[]`.
* **Reference, not to be copied:** all seven `legacyledger` files, with the two shape-blocks above.
