# Phase 1 — Quickstart: validating U6

**Feature**: `005-u6-record-api` | **Date**: 2026-10-11 | **Spec**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

**Read this first.** U6's first slice is **built and measured** (2026-10-11): the closed tool table, the
request form with its canonical line, the answer and its claims, the checks, the effect, the receipt,
the gate, and the parse/world refusal split — and its second slice now serves that table over stdio,
from the binary's `serve` mode to a separate client process. Every scenario below carries **pasted
output** from a run of the commands above it. The **content gate's first slice** has since landed
(2026-10-12, `src/intake.rs`: a declared row consumed in place by reference, judged on named axes,
`admit`/`inspect → Established`, and the census rule that an unchecked axis is never a pass — measured in
`../docs/CLEAN-ROOM-PLAN-2026-10-11.md` §18). What is **owed and named** — the rest of the gate, the
bijection of the served table against the legacy action vocabulary, and the verbs that carry a document
by reference — is at the end, and the binary prints it, because an owed capability is a fact about the
build.

## Prerequisites

```bash
cd clean-room                # its own Cargo workspace, beside the legacy tree
cargo --version              # the toolchain the workspace pins
cargo tree --offline         # measured 2026-10-11: shploingus-clean-room -> sha2 v0.10.9, and nothing else
```

No network is required: the crate depends only on `sha2`, already resolved locally (`--offline`). The
gate adds **no** dependency — parsing, dispatch and digest folding are the whole of it.

## Scenario 1 — the table is the surface, and every answer carries its evidence (SC-001, SC-005)

```bash
cargo run --offline | grep "^U6"
# measured 2026-10-11 (the report mode takes no arguments; the world is the reference declaration's):
#   U6 tool table: 5 declared verbs, closed and bijective
#   U6   truth     no arguments                           owner world.truth_digest   record false checks request_parsed+owner_answered+claims_hold
#   U6             answers: the world's identity and its truth digest
#   U6   fields    field:field, at:voxel, side:side       owner field_at             record false checks request_parsed+arguments_typed+owner_answered+claims_hold
#   U6             answers: one declared field's values over a patch, with the values' digest
#   U6   geometry  chunk:chunk, level:level               owner mesher.derive        record true  checks request_parsed+arguments_typed+owner_answered+claims_hold+recorded
#   U6             answers: a chunk's geometry at a level, derived or served, and the record of the derivation
#   U6   held      no arguments                           owner resident.digest      record false checks request_parsed+owner_answered+claims_hold
#   U6             answers: the geometry the gate holds, how much of it is stale, and its digest
#   U6   record    no arguments                           owner chain.head           record false checks request_parsed+owner_answered+claims_hold
#   U6             answers: the chain's length and its head
#   U6 call `truth`
#   U6   answer   seed=1592639215 period=1s tick=0 truth=146ff7bf…381733  [57440bd1…f9ab48]
#   U6   checks   request_parsed[passed] owner_answered[passed] claims_hold[passed]
#   U6   claims   truth_is (seed=1592639215 period=1s tick=0 truth=146ff7bf…381733)
#   U6   effect   work 0/0 columns (0 misses, 0 hits, 0 refusals) -> 0 quads; held 0 -> 0; record 1 -> 1 events, appended nothing
#   U6   verified every claim re-checked against the owners it describes
#   U6 call `fields field=elevation at=0,0,0 side=8`
#   U6   answer   elevation: 64 columns in [74.606894, 75.776636], values=08652d46…a48925  [457f6ec4…03e50c]
#   U6   checks   request_parsed[passed] arguments_typed[passed] owner_answered[passed] claims_hold[passed]
#   U6   claims   field_is (elevation: 64 columns, values=08652d46…a48925)
#   U6   effect   work 0/0 columns (0 misses, 0 hits, 0 refusals) -> 0 quads; held 0 -> 0; record 1 -> 1 events, appended nothing
#   U6   verified every claim re-checked against the owners it describes
#   U6 call `geometry chunk=0,0,0 level=0`
#   U6   answer   chunk=(0, 0, 0) level=0 144 quads over 1031 cells, mesh=0e7358e4…814ed8 of truth=146ff7bf…381733  [508ed951…af678f]
#   U6   checks   request_parsed[passed] arguments_typed[passed] owner_answered[passed] claims_hold[passed] recorded[passed]
#   U6   claims   truth_is (…); mesh_is_held (chunk=(0, 0, 0) level=0 144 quads, mesh=0e7358e4…814ed8); event_is (event 1 = 35a1924f…f20990f)
#   U6   effect   work 1024/1024 columns (1 misses, 0 hits, 0 refusals) -> 144 quads; held 0 -> 1; record 1 -> 2 events, appended 35a1924f…f20990f
#   U6   verified every claim re-checked against the owners it describes
#   U6 call `held`
#   U6   answer   1 entries (0 stale), resident=ba7a97d4…ca094a1  [8e0c5f0f…b63306]
#   U6   effect   work 0/0 columns (0 misses, 0 hits, 0 refusals) -> 0 quads; held 1 -> 1; record 2 -> 2 events, appended nothing
#   U6 call `record`
#   U6   answer   2 events, head=35a1924f…f20990f  [3882292e…4641c70]
#   U6   effect   work 0/0 columns (0 misses, 0 hits, 0 refusals) -> 0 quads; held 1 -> 1; record 2 -> 2 events, appended nothing
```

**Passes when**: all five declared verbs answer through the table, each receipt names **every check its
row declares** (3 for the reads, 4 for `fields`, 5 for `geometry`) with the outcome, every claim
re-checks against the owner it describes, and the effect says what the call changed. **Measured:** all
hold. Two cross-checks are worth naming: the `geometry` receipt's mesh digest
**(§39, 2026-10-13 — the mesh's unit is the cells' own voxel face.)** The block above is the
2026-10-11 pass's own output; the answers it quotes moved with the mesh: the `geometry` answer now prints
`chunk=(0, 0, 0) level=0 406 quads over 3703 faces, mesh=2db62b78…b79c of truth=146ff7bf…381733
[ffde9d44…db32b]`, the event it appends is `a88bae4e…09cc48`, the resident set's digest is
`3167bb20…a3005`, `save`/`load` hash `29f521b2…a691`, the repeat answer is `f6d66a51…d772`
(`723bab9b…d6` over the transport) and the record's head is `a994e069…bc43`.

**(§40, 2026-10-13 — the extent contract moved the mesh again).** The `geometry` answer now prints
`chunk=(0, 0, 0) level=0 313 quads over 1289 faces, mesh=44a682e1…4ffb of truth=146ff7bf…381733
[68f985de…bad6]`, the work it does is `1156/1024 columns` (the reading reads the chunk's ring and decimates
nothing at level 0), the event it appends is `2ffdd09d…0d4e`, the resident set's digest is `3cc0dc04…cedda`,
`save`/`load` hash `83c20c30…3d04`, the repeat answer is `cb7e4eef…9df7e` (`c282e4af…8368` over the
transport) and the record's head is `510be458…74694`.

`0e7358e4…814ed8` was **U4's own printed digest for the same world and chunk** (the face
merge's is `44a682e1…4ffb` since §40; `2db62b78…b79c` was §39's), and the two reads that
touch the record (`truth`, `fields`) print `record 1 -> 1 events, appended nothing` — a read is not a
change.

## Scenario 2 — one request twice is one answer, and the record moves once (SC-002)

```bash
cargo run --offline | grep "U6 seed=1592639215 tick=0 repeat"
# measured 2026-10-11:
#   U6 seed=1592639215 tick=0 repeat [`geometry chunk=1,0,0 level=0`]: one answer digest
#     25a485a5…59324be (identical: true); the first did 1024 columns of work and appended true, the
#     second did 0 and appended false (effects differ: true); the record moved once for the change: true
```

**(§39, 2026-10-13:** the repeat's own answer digest in the block above is `f6d66a51…d772` now; `25a485a5…`
was the height-merged mesh's.) ** (§40, 2026-10-13:** it is `cb7e4eef…9df7e` now (`c282e4af…8368` over the
transport), and the first call's work is `1156` columns: the reading reads the chunk's ring.)

**Passes when**: the two calls print **one** answer digest, the first did the work of a derivation and
appended exactly one event, the second did **no** work and appended nothing, and the record's length
moved once. **Measured:** all hold. The answer digest is also invariant in the worker count —
`tests/u6_bridge.rs` derives the same chunk through gates at 1, 2, 4 and 8 workers and asserts one
answer digest, which is U3's knob read through the outward door.

## Scenario 3 — a parse fault writes nothing; a world refusal is named and recorded (SC-003)

```bash
cargo run --offline | grep "U6 parse refusal\|U6 world refusal"
# measured 2026-10-11:
#   U6 parse refusal by name: [argument_malformed] `chunk` is declared as a chunk: `0,0` is not one
#     (the record is untouched: true)
#   U6 world refusal by name: [geometry_out_of_residency] chunk (9, 9, 0) is 407.3 m from the anchor
#     at (0, 0, 0), outside the declared residency horizon 256.0 m -> exactly one event recorded =
#     true, naming it = true, nothing cached = true
```

**Passes when**: the malformed line is refused and the record's length is **unchanged**; the world
refusal passes U4's own name through, the record grew by **exactly one** `refusal_recorded` event naming
it, and nothing was cached. **Measured:** all hold. `tests/u6_bridge.rs` goes further: a level off the
ladder (`level_off_ladder`), a budget that cannot be met (`geometry_budget_exceeded`, with `1024` in its
detail because the ceiling is checked against the work that really happened), and a field name no field
declares (`field_undeclared`) are each recorded and each cache nothing.

## Scenario 4 — the answer corresponds to the world it names (SC-004)

```bash
cargo run --offline | grep -A2 "U6 correspondence\|U6 read, not recomputed"
# measured 2026-10-11:
#   U6 correspondence: the archived receipt verifies in the world it was made in = true; against
#     tick 1 it refuses [claim_unverified] = true
#   U6   claim_unverified: claim `truth_is`: recorded seed=1592639215 period=1s tick=0, found
#     seed=1592639215 period=1s tick=1
#   U6   claim_unverified: claim `mesh_is_held`: no geometry is held for chunk (1, 0, 0) at level 0
#     in this world
#   U6 read, not recomputed: evicting 2 stale entr(ies) leaves the archived answer's held-geometry
#     claim failing while its truth claim still holds (true)
```

**Passes when**: the archived receipt verifies against the owners it was made in; the **same** receipt
against a world that has moved refuses `claim_unverified` **with both values**; and — because the
geometry claim is **read** and not recomputed — evicting the entry makes that one claim fail while the
truth claim still holds. **Measured:** all hold, and both failures are collected rather than the first
alone (`2 refusal(s)`), because verification collects every one. The one claim that is instead
**re-evaluated** is the field's: `tests/u6_bridge.rs` verifies a `fields` receipt against a moved world
and asserts `field_is` fails with both value digests, which is what "a function is checked by
evaluating it" means as a measurement.

## Scenario 5 — the checks (SC-006)

```bash
cargo test --offline
# measured 2026-10-11 (the transport pass):
#   lib:          145 passed; 0 failed   (12 of them U6's — 7 the gate's, 5 the transport's)
#   u4_geometry:    2 passed; 0 failed
#   u6_bridge:      6 passed; 0 failed   (the reference world, end to end, through the public API)
#   u6_transport:   3 passed; 0 failed   (the real binary as the server, from a separate process)
cargo clippy --offline --all-targets -- -D warnings
# measured 2026-10-11: clean (no diagnostics)
cargo run --offline
# measured 2026-10-11: exit 0, 55 U6 lines printed, "Clean room OK: every built system's acceptance
#   bar is measured and passing."
```

**Passes when**: the tests pass, clippy is clean under `-D warnings`, and the binary exits 0.
**Measured:** all three, on 2026-10-11. The fresh-code boundary was re-measured too:
`cargo tree --offline` shows `sha2` as the only dependency, no file under `../crates/` was written
during this pass (`find crates -type f -newer clean-room/src/transport.rs` → **0**), and the only
`crates/` strings in the crate are the witness documentation its module headers carry.

## Scenario 6 — the table served over stdio, driven as a separate process (the transport)

```bash
cargo test --offline --test u6_transport
# measured 2026-10-11: 3 passed; 0 failed — the real binary runs as the server (`serve`) and every
#   served receipt, answer digest, record length and head is verified against a world the test built.
cargo run --offline | grep "^U6 transport\|^U6   world refusal\|^U6   parse fault\|^U6   repeat\|^U6   correspondence"
# measured 2026-10-11 (the session's own count goes to stderr, so it is seen rather than grepped):
#   serve: 7 request(s): 5 answered, 2 refused by name
#   U6 transport: `serve` — a separate process answered 5/7 request(s) with true correspondence; clean exit = true, every request sent = true
#   U6   world refusal [geometry_out_of_residency] chunk (9, 9, 0) is 407.3 m from the anchor at (0, 0, 0), outside the declared residency horizon 256.0 m -> recorded (the record grew by one: true)
#   U6   parse fault   [tool_undeclared] "abracadabra" is not one of the 5 declared verbs: truth, fields, geometry, held, record -> writes nothing (the record held: true)
#   U6   repeat      one answer digest 4c4da3299efb79daf7252761e5cbacea366b1bc91cdac135f5768892323bff1b (identical: true); the first appended exactly one event, the second did 0 columns and appended none (one append: true)
#   U6   correspondence: every served receipt verified against this process's own world, resident and record = true; the served record is this process's record = true; world refusal named = true, parse fault named = true
```

**(§39, 2026-10-13:** the transport repeat's own answer digest in the block above is `723bab9b…392d6` now;
`4c4da329…` was the height-merged mesh's.)

**Passes when**: a request that succeeds comes back as a receipt whose claims verify against the world
it was made in; a world refusal keeps its owner's name **and** shows the record grow by one; a line
that does not parse the table is refused by name and writes nothing; the same request twice is one
answer with two effects; and the record read off the pipe is the client's own record after the same
calls. **Measured:** all hold, from a real child process — `5/7` requests answered and `2` refused by
name, with `correspondence = true` at every step. The protocol is declared in
[contracts/u6-transport.md](./contracts/u6-transport.md); the client half refuses a request it cannot
frame **before** writing anything, and the server half refuses a line it cannot frame with its own
`payload_not_wirable`, echoing `-` because that line is exactly the one it cannot quote.

## Scenario 7 — a row arrives **by reference**, over the real pipe

```bash
cargo test --offline --test u6_admit
# measured 2026-10-12: 4 passed; 0 failed — the real binary runs as the server in the fixture's own
#   directory, one row is admitted by reference, an absent and a moved document each refuse by name,
#   and a line the table does not declare writes nothing.
cargo run --offline | grep "^U6 call \`admit\|^U6 intake by reference"
# measured 2026-10-12 (the answer, checks, claims and effect lines are elided; the run prints 270
#   lines, nine more than the 261 of 2026-10-11 — two for the new table row, six for its receipt, one
#   for the refusals):
#   U6 call `admit path=source.md digest=05285b94… feature=a-declared-row licence=permitted claim=checksum_only carrier=U4 check=check.md check_digest=bbf67132…`
#   U6   answer   a-declared-row: admitted as checksum_only (identity established = false), the document read in place at 05285b94…; axes feature=active source=active licence=active carrier=active acceptance=active identity=inactive  [5de235e3aa1633249474691f60f1b9d71c3455996a34869440576b1955167bb8]
#   U6   checks   request_parsed[passed] arguments_typed[passed] owner_answered[passed] recorded[passed]
#   U6   claims   none — this row declares no `claims_hold`, and makes no claim nothing here could re-check
#   U6   effect   work 0/0 columns (0 misses, 0 hits, 0 refusals) -> 0 quads; held 1 -> 1; record 2 -> 2 events, appended nothing
#   U6 intake by reference: absent [source_absent] … (the path named: true); the bytes moved [source_digest_mismatch] … (both digests in the message: true); both lines parsed, so both refusals are recorded = true, and the record still verifies = true
```

**Passes when**: a client over the real pipe admits a document by reference and sees what it became —
the digest its bytes hashed to, the claim it was admitted under and every declared axis's state — while
an absent and a moved document each refuse **by name** with the path or both digests in the message, a
line the table does not declare writes nothing, and the same row twice is one answer that moves no
record. **Measured:** all hold, from a real child process started in the fixture's own directory (the
root a relative reference resolves against), with the served answer digest equal to the one derived in
the test's own process.

## Scenario 8 — the policy chokepoint: one decision function, and a denial that is recorded (2026-10-12, §29)

**Given** the reference session, whose actor and grants are declared once in
`src/reference.rs` (`reference::caller`, the same caller the binary and the tests act as), **when** the
binary runs and the served rows are driven, **then** every act is judged by one function before it
dispatches — and the output carries the matrix, the construction rules and the denial itself:

```console
# pasted from `cargo run --offline` after the slice (never retyped):
# U6 policy: 13 act(s), closed (6 of them allowed — the reads and the checks everything depends on; 4 gated; 2 denied unconditionally; 1 a rejection), every row describes itself = true, and the classes account for every act = true
# U6 policy: deny by default — `everything`, `execute`, `*`, `all` are not acts and not scopes, and a missing matrix entry is a **compile error** rather than a runtime deny (the matrix's `match` has no wildcard arm) = true
# U6 policy construction: a grant with no granter [grant_unattributed], a scope that is `everything` [grant_unscoped], a scope nobody declares [grant_unscoped], and a caller with no actor [grant_unattributed] — each refused at construction = true
# U6 policy: the reference `operator` serves every gated act — `save` wrote 496 byte(s), `load` read them back, and `geometry` appended — 6 of 6 calls answered = true
# U6 policy: `reader` holds nothing — its three reads answer = true; its four gated acts refuse [policy_denied, policy_denied, policy_denied, policy_denied] = true, each one **recorded** (the record moved by 4 of 4) = true, and the record still verifies = true
# U6 policy refusal, quoting the list: `reader` may not `append_event`: it requires a grant of the `record` scope and `reader` holds no scopes — the declared scopes are `record`, `content`, `world`
# U6 policy: an actor holding all 3 scopes still cannot `delete_content` [policy_denied] or `write_external` [policy_denied], and `persist_material` is a rejection rather than a permission [never_store] = true
```

**Passes when**: the matrix accounts for every declared act (6 allowed + 4 gated + 2 denied + 1
rejection = 13); an act or a scope nobody declares does not parse; a grant with no granter and a scope
that is `everything` are refused **at construction**; the reference operator is served every gated row;
an actor holding nothing is refused `policy_denied` on **all four** gated rows **before they dispatch**,
with **each denial recorded** and the record still verifying; the refusal **quotes the list** (the act,
the scope it needs, what the caller holds, and the declared scopes); and the two unconditional denials
hold even for an actor holding all three scopes, while `persist_material` is a **rejection rather than a
permission** (`never_store`). **Measured:** all hold, in the binary's own evidence
(`check_u6_policy`), in `src/policy.rs`'s seven unit tests and in `tests/u6_policy.rs`'s six — and each
guard was perturbed and restored byte-exact (§29's guard table).

## What U6 still owes, named rather than implied

- **The content gate's remainder.** Its **first slice** has landed (2026-10-12, `src/intake.rs`, §18 of
  the governing plan): `admit`/`inspect` returning `Established` (never a bool), a declared row consumed
  **in place by reference**, judged on named axes, with the census rule that an unchecked axis is never
  a pass. Still owed, and still unstubbed: the document record and its compression, the calibrated
  structural judge, the ballot, the crate-by-axis census **over the tree**, the permission policy
  matrix, the recipe format, the API document, and a row's conformance against the world's own
  declarations. *(Corrected 2026-10-12: **a row's wire form has since landed** — see Scenario 7; the
  conformance check is what remains of that clause.)* *(Corrected again 2026-10-12, §29: the
  **permission policy matrix** has since landed as the policy chokepoint — see Scenario 8; it leaves
  the clause owing the **self-describing protocol** and the **tool-surface narrowing over a
  mod-supplied tool list**.)* This slice lands **the path all of them are served through** — a request,
  an answer, its claims, its checks and its effect — and none of their bodies. A stub returning a
  plausible `Established` would be the invented default P2 forbids, so there is no stub.
- **The bijection of the served table against the legacy action vocabulary.** The transport now serves
  the table over stdio (one line in, one response record out; declared in
  [contracts/u6-transport.md](./contracts/u6-transport.md)), but whether its six verb names are
  bijective with the legacy `voxel-mcp` actions is a **witness check**, and P7 forbids reading
  `../crates/voxel-mcp` for it — so it is named owed rather than approximated.
- **Requests that carry a document** *(corrected 2026-10-12: the opposite has landed — a row cites
  `path + digest` and the bytes are read **where they live**, never carried; a request that carried
  bytes is still not a thing this table declares, and the plan's rule is why)*. The `Path`, `Digest`
  and `Name` argument kinds a by-reference row needs are declared, and the document is consumed in
  place by `src/intake.rs`, whose own remainder (the document record and its compression, the judge,
  the ballot, the census over the tree, the policy matrix, the recipe format, the API document and a
  row's conformance against the world's declarations) is named above rather than stubbed *(and the
  **matrix**, once owed here, has since landed — §29)*.
- **A streamed residency policy.** Which chunks to keep and when to evict is U5/U6's by the plan's own
  note; U4 owns *exact* invalidation, which the gate exercises (`held` reports the stale count, and
  eviction is `Resident::invalidate`).
