---
patch_id: PATCH-044
title: The authority layer — the verified matrix an agent reads, the recipe it asks for, and the grant that says no
status: approved
operation: plan
target: voxel-axis-mcp + shploingus/shell-console
base_ref: shploingus-clean@main
dependencies: [PATCH-042, PATCH-043]
source: user-directed-conversation
created_at: 2026-10-21
approval: "user-directed 2026-10-21 — the axis mcp layer, and nothing wider: no civilians crate, no asymmetric anchor"
live_mutation: false
---

# Change

**The package now verifies, and nothing outside the game can ask for one.** `PATCH-042` built the
matrix and the walk, and `PATCH-043`'s purpose put a package behind a key — but the only surface that
read either was the shell console, which is the game talking to itself. An agent had no way to ask
*"what does the matrix say"*, *"what is the recipe for this ticket"*, or *"may I apply it"*, and the
mission's axis layer did not exist.

`crates/axis/mcp` (`voxel-axis-mcp`) is that layer. It is **a library, not a second server**:
`crates/voxel-mcp` is the surface, and two JSON-RPC servers would be two places a tool is declared,
two places a refusal is phrased and two places a ticket is honoured. The server, the console, and
anything else that wants to put a ticket in front of an agent call this crate.

Three acts, and the fourth that is not one:

| act | what it answers |
|---|---|
| `Registry::read` | the verified matrix and its package, over this tree — or the decline the walk reported, never a shortened table. **No key**, so reading never mints one |
| `Registry::mount` | the same reading, holding the key that signs and verifies grants |
| `Registry::recipe(ticket, segment, axis)` | the package line for a named ticket and cell |
| `Registry::grant(ticket, segment, axis, expires)` | a `TicketGrant`, signed by the security layer |
| `Registry::apply(grant, ticket, cell, now)` | **a decision**, and the checks that ran. It performs nothing |

**`apply` deciding and not doing is the record's own shape, not an omission.** `voxel-package`'s
`bridge::call` already refuses with `UntrustedKey` when a session does not hold the capability and
says in its own documentation that nothing is executed by it. A layer that also wrote the edit would
be a second implementation of the act, and the act belongs to the surface holding the file.

# The finding: the layer had no refusals to reuse, because it did not exist

Every refusal this patch needs was already in `voxel_model::refusal::RefusalKind`, and the temptation
was to add an `Unauthorized` variant for the layer's own convenience. That would have been the fifth
vocabulary in a tree that keeps asking for one. Instead:

| the attempt | the refusal | why that kind |
|---|---|---|
| no credential presented | `UnsignedEnvelope` | there is no signed thing to verify; silence is not permission |
| a grant signed by another key | `UntrustedKey` | the security crate's own fact: the verifier does not hold that key |
| the grant's fields moved after signing | `SignatureInvalid` | the signature is over other bytes than a reader sees |
| a grant for another ticket | `UntrustedKey` | `bridge::call`'s precedent: a credential that does not authorise this act |
| a grant scoped to another cell | `UntrustedKey` | the same fact, about the cell rather than the ticket |
| an act at or after the grant's expiry | `DestinationInvariantRefused` | a move past the bound the grant itself declared |
| a recipe for a ticket or cell not measured | `NoReading` | there is no reading to hand out |
| a recipe or an apply on an unwitnessed cell | `UncalibratedOracle` | the matrix's own kind for a star lit by a row that did not run |

**Nothing here re-declares the axis vocabulary either.** `Segment`, `AxisKind`, `Cell`, `Reading`,
`Star`, `Line`, `Package` and `Grant` are imported, not copied. The two types this crate does
declare — `TicketGrant` and `Applied` — are new facts (a signed authorization, and a receipt), not a
second spelling of anything already owned.

# The grant

`TicketGrant` wraps `voxel_model::grant::Grant` — which already requires a granter and a non-empty
scope — and adds the four facts a ticket-scoped act needs: the ticket, the cell, the expiry tick and
the signature. `Grant::new` refuses an empty granter or scope, so a grant with no attribution cannot
be constructed to sign.

The signed payload is a domain-separated `Identity::of_fields("voxel-axis-mcp/ticket-grant", …)` over
**every field**, including the granter and the scope. Changing the ticket, the cell or the expiry
after signing is `SignatureInvalid`; presenting a grant signed by another key is `UntrustedKey`. Both
are the security crate's refusals, raised by its own `Verifier`.

`expires` is **exclusive**: a grant good until tick 40 authorizes tick 39 and refuses tick 40. The
boundary is tested at both ticks rather than described.

# The route into the server

`voxel-mcp` — the surface the tree already ships — now calls this layer as four tools. No new action
was declared: each tool is a row of `voxel-package`'s bridge, so the gate that already runs every
other call runs these.

| tool | the act | the row (capability) |
|---|---|---|
| `get_axis_matrix` | `Registry::read` — the cells and the package, with no key | `read_governance` (observe) |
| `get_axis_recipe` | `Registry::recipe` for the named ticket and cell | `read_recipe` (observe) |
| `grant_recipe` | a `TicketGrant`, written as one wire line | `file_ticket` (present) |
| `apply_recipe` | `Registry::apply`, returning the receipt | `resolve_ticket` (present) |

**A call with no credential is refused by the matrix's own class, not by this layer.**
`resolve_ticket` is `RequiresGrant`, so `bridge::call` refuses an apply before the dispatcher is
reached and the receipt says `silence is not permission`. With a credential, the bridge's grant is
derived from the wire line's own attribution — presence is the bridge's check, and **the signature is
the proof**: `Registry::apply` verifies it over the ticket, the cell and the expiry before any field is
used. A wire line that does not parse is `malformed_record`, not "no grant presented"; a line moved
after signing is `signature_invalid`; another key is `untrusted_key`.

**The expiry is measured against the server's clock.** `grant_recipe` signs the bound its caller asks
for, and `apply_recipe` compares it with `SystemTime`, never with a tick the client supplied — a
credential's expiry checked against the clock of the party presenting it is not a bound.

**The wire is the grant's own.** `TicketGrant::to_wire` writes a tag and nine fields on one
`axis-grant/1` line; `from_wire` reconstructs them and does not verify. A ticket carrying a tab is
refused at issue, because a grant that cannot be written on one line cannot be presented.

# What the console shows

The shell's `file` path, after the pin gate passes, mounts a `Registry` over the cells and the package
it already printed and then exercises the layer in front of the player:

- `grant issued: grant sha256:… for T-… on shell/evidence_grade until tick … — signed by sha256:…`
- `apply permitted on T-… at shell/evidence_grade — checks: credential · signature · ticket · cell · expiry · witness`
- `refused: the grant is for T-…-neighbour and this act is on T-…: a ticket-scoped grant authorizes the ticket it names and no other`

The third line is the one that matters. A layer that only ever says yes is not an authority, and the
mount test asserts the refusal rather than only the permission.

# What this patch does not do, and says so

1. **The trust anchor is still symmetric and self-issued.** The same machine signs and verifies, so a
   grant proves this machine authorised the act, not that a named party vouches for it — and a client
   holding `present` can ask for a long bound because the issuer is the verifier. The asymmetric
   anchor is its own pass, and the `Verifier` seam is where it lands.
2. **The civilians crate is untouched**, as the mission ordered.
3. **`apply` does not perform the edit.** It returns the receipt the caller holds while performing it.

# Gates

| gate | evidence |
|---|---|
| the layer's own suite | `cargo test -p voxel-axis-mcp` → 18 passed: mount, read, recipe, permitted apply, the wire round-trip, and each of the six refusals hit by an attempt |
| the route | `cargo test -p voxel-mcp` → 28 passed across `api_layer`, `axis_route` and `protocol`: the four tools listed, the matrix and recipe served, an apply refused with no credential and permitted with one, and the tampered, expired, other-ticket and malformed credentials each refused by kind |
| the harness it sits in | `cargo test -p voxel-axis-artifact`, `-p voxel-axis-security`, `-p voxel-axis-verify`, `-p voxel-axis-matrix` |
| the tree's own gates | `cargo test -p voxel-package --lib`, `-p voxel-client --lib` (the mount test asserts the issued and refused lines) |
| the census | row 22, `voxel-axis-mcp`, witness `crates/axis/mcp/src/lib.rs`; the axis segment measures it; `SUBJECT_COUNT` 182 → 188 |
| the conformance pins | `conformance/prose-frame.txt` declares the measured 0 % plain share; `conformance/missing-docs.txt` carries the count (0) and the pin over the covered files |
| the spec | `DESIGN-SPEC.md` §5.1 names six axis crates and this patch |

# The boundary this patch is held to

**A refusal is proven by an attempt, not by its existence.** Every row in the table above is a test
that presents the credential or names the cell and asserts the kind, and the console asserts the
wrong-ticket line reaches the player. The one refusal not reachable through `Registry::mount` — an
unwitnessed cell — is reached through `Registry::of`, which serves cells without a package precisely
so that the apply path's witness check is live rather than dead code behind a verified mount.
