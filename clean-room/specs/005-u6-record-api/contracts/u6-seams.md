# Phase 1 — Interface contracts: U6 the record and the API layer

**Feature**: `005-u6-record-api` | **Date**: 2026-10-11 | **Spec**: [spec.md](../spec.md) | **Plan**: [plan.md](../plan.md)

The clean room is a **library**: its contracts are the seams its callers use. U6 exposes five, and each
is stated as the *refusal* it must produce as well as the value it must return — a seam that cannot
refuse is not a seam (P2). *(C5 was added 2026-10-12, §29: the policy chokepoint, whose contract the
other four are judged through.)*

## C1 — One closed table, and one declared line

```text
pub static TOOLS: [Tool; TOOL_COUNT]        // five rows over four owners at this slice;
                                            // six after `admit`, eight after `save`/`load`
pub fn tool(name: &str) -> Option<&'static Tool>

Tool { verb: Verb, name, args: &[Argument], answers, owner, changes_record, checks: &[CheckName],
       action: policy::Action }            // the act this door performs — the row's own declaration,
                                           // so a door cannot be added without naming its act
Argument { name: &'static str, kind: ArgKind, required: bool }
ArgKind  { Chunk | Level | Field | Voxel | Side }
Verb     { Truth | Fields | Geometry | Held | Record }
CheckName{ RequestParsed | ArgumentsTyped | OwnerAnswered | ClaimsHold | Recorded }

parse(line: &str) -> Result<Request, Refusal>
Request::canonical() -> &str                // the request's identity
Request::{chunk, voxel, level, field, side}(name) -> Result<T, Refusal>

Refusals (all **before** any world or record is touched — a line that does not parse writes nothing):
  - tool_undeclared       no row is named by the first token (or the line is empty)
  - argument_undeclared   the row declares no such argument (it declares: …)
  - argument_absent       a required argument is missing — there is no default
  - argument_malformed    not of its declared kind, or a key given twice, or a token with no `=`
```

**Contract.** The table is **closed and bijective**: one name per verb, one verb per name, one value per
key, and each row names the checks it runs. The canonical line is built from the **declared** order, not
the written order, so two spellings are one request and one answer (P3).

**Domain refusals are the owner's, and are decided at dispatch**: `level_off_ladder` (U4's ladder),
`field_undeclared` (no field declares this name), `geometry_out_of_residency`,
`geometry_budget_exceeded`, and a declaring stage's own refusal passed through. A value that is
*shaped* like its kind but not *in* the world is therefore never a parse fault.

## C2 — Every answer carries its claims, and a claim is a question, not a copy

```text
Answer { Truth{seed,period_s,tick,truth} | Fields{field,columns,values,low,high}
       | Geometry{chunk,level,quads,cells,digest,truth} | Held{entries,stale,digest}
       | Record{events,head} }
Answer::digest() -> Digest        // a pure function of what the answer **describes**

Claim { TruthIs | FieldIs | MeshIsHeld | HeldIs | RecordHeadIs | EventIs }
Claim::check(&Owners) -> Result<(), Refusal>
  - claim_unverified      the claim does not hold; the detail names the claim, the recorded value and
                          the found value
```

**Contract.** No `Answer` variant carries a worker count, a ledger total or a hit/miss flag, so a
derivation and the cache hit that serves the same world print **one** answer digest (FR-004). Every
claim is checked **against the thing it describes**:

| Claim | Re-read from | Notes |
|---|---|---|
| `TruthIs` | `World::identity`, `World::tick`, `World::truth_digest` | identity first (both values), then the digest |
| `FieldIs` | `field_at` at the same request | the **one** claim that is re-evaluated: a field has no stored form |
| `MeshIsHeld` | `Resident::serve(world, chunk, level)` | **read**, never recomputed — so an evicted mesh fails the claim while the truth claim still holds |
| `HeldIs` | `Resident::{len, stale, digest}` | the cache's own identity, and how much of it is stale in this world |
| `RecordHeadIs` | `Chain::{len, head}` | the record is U2's; this claim never restates it |
| `EventIs` | `Chain::events()[seq].digest()` | the event a call appended, re-read at its position |

## C3 — The gate: one line in, one receipt out, and the effect of the call

*(Corrected 2026-10-12: `Gate::new` takes the world's **material registry** beside the roster —
`Gate::new(roster, registry, ladder, residency, identity, workers)` — because an admission's
`conformance` axis judges a row's `requires_materials` against it. The registry is **borrowed**, held
by the intake beside the roster, never copied; the palette is still the caller's `Ladder`. See
`tasks.md`'s Phase 10 and §21 of `docs/CLEAN-ROOM-PLAN-2026-10-11.md`.)*

```text
Gate::new(roster, registry, ladder, residency, identity, workers) -> Gate
Gate::{world, world_mut, chain, chain_mut, resident, resident_mut, ledger, owners}
Gate::set_budget(Budget)

call(line: &str) -> Result<Receipt, Refusal>

Receipt { request, answer, claims, checks, effect }
Receipt::{answer_digest, verify(&Owners) -> Result<(), RefusalSet>, render}
Effect { columns_produced, samples_read, quads, hits, misses, refusals,
         held_before, held_after, events_before, events_after, appended: Option<Digest> }
Owners { world, roster, resident, chain, workers }
```

**Contract — the row's conformance is against the world.** `admit` may declare what the row requires of
the world (`requires_stages` against U3's `Declared`, `requires_materials` against U2's
`MaterialRegistry`); absent means *this row requires nothing*, which the `conformance` axis **reports**
rather than counts. A required stage the roster does not run is refused `stage_undeclared` **with the
stage named**, a material the registry never admitted is refused `unadmitted_material` in the
registry's own words, and a `requires_…` key with nothing after it is refused by the table's shape rule
(`argument_malformed`) — a parse fault, which writes nothing. The registry is the **same** declaration
the ladder's palette was drawn from: two registries would be two worlds (`LEDGER` §55.3).

**Contract.** `call` returns a `Receipt` only when **every** claim it hands out holds right then: the
claims are re-checked inside `call`, and a claim that does not hold is refused `claim_unverified`,
recorded, and handed back as the call's refusal. `Receipt::verify` re-checks them **later**, against
whatever `Owners` the caller supplies, collecting **every** failure rather than the first — which is how
the same receipt is proved to correspond to the world it was made in and refused against a world that
has moved.

**Contract — the effect is a delta.** `Effect` is measured as `after − before` off the owners
themselves (U4's ledger, U4's resident set, U2's chain), never computed from the answer. A read's effect
is all zeros and `appended: None`, and `Receipt::checks` carries that as a **passed** `recorded` check
rather than a silent omission.

**What the gate may mutate, and how.** The gate owns the owners' *values* — U2's `World` and `Chain`,
U4's `Resident` and `WorkLedger` — and exposes `world_mut`/`chain_mut`/`resident_mut`, which are the
owners' own types. A caller moves the world with `World::with_tick`, files a ticket with
`Chain::file_ticket` and evicts with `Resident::invalidate`: the gate has no operation of its own over
any of them, and keeps no second copy of any state.

## C4 — What the record gains, and the two refusal classes

```text
Namespace::Record  ·  EventKind::GeometryDerived   // added at the record's owner (chain.rs)
Namespace::Record  ·  EventKind::RefusalRecorded   // U2's existing kind, now used by the gate

geometry_derived payload: request=<canonical line> chunk=(x, y, z) level=L quads=N truth=<hex> mesh=<hex>
refusal_recorded payload: request=<canonical line> kind=<owner's name>
```

**Contract.** A call appends **exactly** what it changed, through `Chain::append` — the chain's one
writer — and nothing else. A **miss** appends one `geometry_derived`, so a later reader replays the
derivation instead of re-deriving it (F1); a **hit** appends nothing; a read appends nothing. A
world-level refusal appends one `refusal_recorded` naming the owner's own refusal **and** the request
that was refused, because that refusal is a fact about the world. A request that does not parse
**writes nothing at all**, because it is not an event about anything.

**Contract — the record carries the decision, not the explanation.** The payload is the canonical
request and the refusal's **name**, and it carries **no `detail`**, because the chain hashes the
payload into the record's own identity while a refusal's `detail` is prose for the caller: it may quote
the host that produced it (an operating system's error text, a path as that host normalized it). A
detail in a payload would give one world two records depending on which machine refused the same
request. Everything the refusal is *about* the request already carries — the path, the chunk, the level
the row declared — and the words stay in the answer. (Measured 2026-10-12: the same refusal, worded two
ways, was two recorded rows under the `detail=` payload and is one row under this one.)

**Contract — a payload the wire form cannot frame is not a lost refusal.** If the payload cannot ride
the chain's tab-separated wire form, the append is refused `payload_not_wirable` and the returned
refusal names **both** facts — that the record could not record it, and which refusal it was. Neither
is dropped, and the chain is left unchanged. Since the payload is the table's own canonical line and a
closed vocabulary, this path is reached by a caller handing `record_refusal` a request that cannot ride
the wire — the table's own separator can never be part of a value, because a value carrying one is
refused by the argument kind's shape rule before any world is read.

**Contract — a receipt is bound to the row it names.** *(See C5: the row's `action` is checked before
dispatch, so a receipt exists only for an act the caller was permitted.)* The wire form is anchored to the table twice: the
request must be the canonical line of the table's own parse **and** the receipt's evidence must be the
evidence that row declares — the answer is that verb's answer, every claim is one the row's `claims`
declares, a row declaring `claims_hold` carries at least one claim, and the checks are exactly the
declared ones in the declared order. A receipt that does not bind is refused `receipt_mismatch` by both
readers ([`Receipt::from_wire`] and [`Receipt::verify`]) instead of being read back as one that
verifies over nothing. (Measured 2026-10-12: with the claims field emptied, `from_wire` accepted the
receipt and `verify` returned `Ok`.)

## C5 — The policy chokepoint: one decision function, and the act is the row's own declaration

*(Added 2026-10-12, §29. Derived from `docs/HYPERGRILLING.md` §L-5 — *"`src/security/policy.ts` — the
single chokepoint: every action passes through **one decision function**, so the permission matrix is
testable as a matrix rather than per call site, and so the one call site nobody remembered cannot be
the one that fails"* — and from `docs/LEDGER.md` R-62.3 and §67.4. The legacy
`crates/voxel-package/src/policy.rs` is the **witness**, never the source: nothing here was read from
it (P7).)*

```text
pub enum Scope { Record | Content | World }            // closed, counted; "everything" is not a scope
pub enum Action { ReadWorld | AppendEvent | AdmitContent | SaveWorld | LoadWorld | CompileRecipe
                | VerifyPackage | LintLine | RaiseToast | ReadGovernance | DeleteContent
                | WriteExternal | PersistMaterial }    // closed at ACTION_COUNT = 13
pub enum PermissionClass { AlwaysAllowed | AlwaysDenied(&'static str) | Gated(Scope) | NeverStore }

Grant::of(granter: &'static str, scope: &str) -> Result<Grant, Refusal>
Caller::of(actor: &'static str, grants: Vec<Grant>) -> Result<Caller, Refusal>
Caller::{actor, grants, holds(Scope)}
permission_of(action: Action) -> PermissionClass       // the action and **nothing else**
decide(&Caller, Action) -> Result<(), Refusal>         // **the one decision function**
check_matrix() -> Result<(), Refusal>                  // every act must have something to say

Refusals, by name:
  - policy_denied         a gated act with no grant of the needed scope; or an unconditional denial
                          — the detail names the act, the scope, what the caller holds and who granted
                          each, and the declared scopes
  - grant_unattributed    a grant with no granter, or a caller naming no actor (refused at construction)
  - grant_unscoped        a scope that is undeclared, blank, or `everything` (refused at construction)
  - never_store           a rejection rather than a permission: the material may be used, never persisted
  - action_undescribed    an act in the matrix that describes nothing (refused by `check_matrix`)
```

**Contract — deny by default is structural, not a runtime hope.** `Action` and `Scope` are closed
enumerations; `permission_of` is an exhaustive `match` with **no wildcard arm**, so a missing matrix
entry is a compile error (`E0004`) rather than a deny nobody notices; an act or a scope outside the
vocabulary does not parse, and the table's own undeclared verb meets `tool_undeclared` where the line is
parsed — before any dispatch.

**Contract — permission is a property of the action.** `permission_of` takes an `Action` and nothing
else: no tool, no caller, no world. The same act has one class whichever door it is reached through
(`truth` and `held` are both `read_world`), so a permission cannot be widened by reaching it a
different way.

**Contract — grants are scoped and attributed, and an unauthorised grant is not a value.** `Grant::of`
refuses a blank granter and a scope that is undeclared or unbounded; `Caller::of` refuses an unnamed
actor. Both faults are refused **at construction**, so the build cannot hold them.

**Contract — the one call site, and a denial is recorded.** `Gate::call` consults `decide` exactly
**once**, before it dispatches, with the act taken from the row's own `Tool::action` — and a denial is
recorded through the chain's one writer as an event naming the request and the refusal's **name**
(never its detail: the record carries the decision, C4), then handed back. A read is `AlwaysAllowed`
because a permission verification itself needs is a permission that can be withdrawn from verification;
`RaiseToast` is allowed because a matrix in which the only way to speak is gated would make every
refusal silent; `AppendEvent` is **gated** because a chain write needs authority (§67.4's correction);
`DeleteContent` and `WriteExternal` are carried and denied unconditionally, because an act that cannot
be named cannot be shown to be forbidden.

**Measured.** The binary prints the matrix (13 acts: 6 allowed, 4 gated, 2 denied, 1 rejection), the
construction refusals, the operator's six served calls, the reader's four recorded denials with the
quoted list, and the unconditional denials holding for an actor holding all three scopes —
`src/evidence.rs`'s `check_u6_policy`, §29's Scenario 8.
