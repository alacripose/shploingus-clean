# Phase 1 — Interface contracts: U6's transport

**Feature**: `005-u6-record-api` | **Date**: 2026-10-11 | **Spec**: [spec.md](../spec.md) | **Plan**: [plan.md](../plan.md)

The transport is a **way in**, not a second owner: it frames the table's own line, hands it to the gate,
and writes back what the gate handed out. It owns no truth, no record and no geometry, and the type
signatures make that impossible — `serve` takes `&mut Gate` and has nothing else to answer from.

## C5 — One line in, one response record out

```text
serve(gate: &mut Gate, input: &mut impl BufRead, output: &mut impl Write) -> Result<Session, Refusal>
reference::serve_stdio(roster, workers, input, output) -> Result<Session, Refusal>  // `src/reference.rs`, the one declared world
reference::gate(roster, workers) -> Result<Gate, Refusal>

request(line) -> Result<String, Refusal>     // the line and its terminator, framed
send(output, line) -> Result<(), Refusal>    // write and flush, so a waiting server sees it
read_response(input) -> Result<Option<Response>, Refusal>   // None at a clean end
parse_response(lines: &[String]) -> Result<Response, Refusal>

Session { requests, answered, refused }
Response::Ok  { receipt: Receipt, record: ServedRecord }
Response::Err { request: String, kind: String, detail: String, record: ServedRecord }
Response::request()   -> &str        // the line the response is about: the receipt's canonical
                                    // request, or the echo (`-` when it could not be framed)
Response::is_about(line) -> bool     // the rule read_response_for enforces
read_response_for(input, expected) -> Result<Option<Response>, Refusal>
ServedRecord { events: usize, head: Option<Digest> }
```

**The declared line protocol.** A request is **one line**; a response is a bounded group of
`key ⇥ value` lines closed by a blank line:

```text
status  ⇥ ok | err
request ⇥ <the line as received>            (- when the line could not be framed)
record  ⇥ <events>:<head hex | ->           the record **after** the call
receipt ⇥ <receipt wire>                    ok only — the whole receipt, one field
kind    ⇥ <refusal kind name>               err only
detail  ⇥ <the refusal's own words>         err only
<blank line>                                end of response
```

**What a request line may not carry.** The characters the response's framing and the receipt wire use —
tab, newline, carriage return, `~`, `;`, `|` — are refused `payload_not_wirable` by the transport
**before** the gate sees the line: the table never sees it, so it is not an event about the world and
writes nothing. That refusal is the transport's own, and its response echoes `-`, because the line is
exactly the one it cannot quote without breaking its own framing.

**Contract — the response's record is the record *after* the call.** Every response carries
`ServedRecord`. A world-level refusal is recorded by the gate, so the client sees the record grow by
exactly one; a line that did not parse leaves it standing. This is how a served session preserves the
table's discipline: the difference between a refusal that counts and a line that is not an event is
visible from outside.

**Contract — the reader never guesses.** An unknown response key, a key given twice, a missing required
key, a status that is neither `ok` nor `err`, a receipt that does not read back against the declared
wire form (including a receipt whose request is not the table's own canonical line), or a stream that
ends inside a response is refused `malformed_wire` rather than half-read. A client that cannot frame a
request refuses it before writing anything.

**Contract — a response is about the line it quotes.** The echo is part of the response's meaning, not
decoration: an `ok` answer is anchored to the receipt's own **canonical** request, and a refusal must
quote the line it answers — `-` when the transport itself could not frame it, and otherwise a line that
*could* have crossed. A refusal whose echo carries the framing's own characters is refused
`malformed_wire`, and a refusal with no echo at all the same: a response that names no line cannot be
read as an answer to one. `transport::read_response_for(input, expected)` is the client-side half of
that rule — a response about a **different** line is refused `malformed_wire` with **both** lines in the
message, so an answer to someone else's request can never be mistaken for this caller's. An `ok` answer
still counts as this line's own when the client sent a non-canonical spelling: the table canonicalizes,
and the echo the receipt carries is the canonical form.

**Behaviour, declared rather than inherited: the blank line.** A blank request line is a **request**
that names no verb — counted in the session, refused `tool_undeclared` ("an empty request line names no
verb"), and it writes nothing. It is not ignored, and it is not an end-of-session marker: only the end
of the input is. A client that appends a trailing newline therefore sees one refusal, and a client that
wants to know the session ended reads the response stream to its end. This is kept as declared rather
than changed: silence would hide a line the caller sent, and the count on stderr stays honest.

**Contract — one reference declaration, and it is not the framing's.** `reference::gate(roster,
workers)` is the one place the reference world (palette, ladder, residency horizon, identity) is
written down, so the binary's `serve` mode, its own evidence and its tests cannot drift into three
similar worlds. It lives in `src/reference.rs` — the framing never learns which world it serves — and
`reference::serve_stdio` hands the built gate to `serve`. The world is still the caller's: a session's
record opens **empty**, so what a client reads back is exactly what its own requests recorded.

**Contract — correspondence, not trust.** A served receipt is the value the gate handed out, not a
summary of it: a client re-checks every claim against owners it supplies itself (`Receipt::verify`), and
the reference declaration makes that comparison meaningful rather than notional. The tests and the
binary's own evidence both do it — they replay the same calls in-process and verify the served
receipts, answer digests, record length and head against their own — so a served answer is believed
because it **corresponds**, never because it crossed a pipe.

**Contract — a row crosses by reference, and the root it resolves against is the server's own.**
The table's sixth row is reachable over the same line: `admit` declares its eight arguments — `path`,
`digest`, `feature`, `licence`, `claim`, `carrier`, `check`, `check_digest` — and the document and its
acceptance check each arrive as a **reference** (`path + digest`), never as bytes:

```text
admit path=source.md digest=<hex> feature=a-declared-row licence=permitted claim=checksum_only \
      carrier=U4 check=check.md check_digest=<hex>
```

A reference is a **relative** path, refused by *shape* (`argument_malformed`, and the line writes
nothing) when it leaves the root, names a `..` segment, or carries a character the receipt's own fields
use (`=`, `~`, `;`, `,`, `|`) — a path that could be answered but not written back is refused before any
world is read. Otherwise the row is read **where it lives**, relative to the directory the server runs
in (`Gate::set_intake_root` pins a different root for a caller that needs one), and the answer says what
it became: `admitted,feature=…,claim=checksum_only,signer=-,found=<hex>,axes=feature:active|…` — every
declared axis's state **by name**, `identity` reported as `inactive` because a checksum establishes
integrity and never origin (`Claim::establishes_identity()` is `false`, and no `bool` carries it).

Because such a line **parses**, its refusals are world-level: an absent document is refused
`source_absent` and a document whose bytes have moved `source_digest_mismatch` — each with the path (or
both digests) in the message, and each recorded once, so the client sees the record grow. `admit`
declares `changes_record: false` and its receipt claims nothing: the evidence it carries is the digest
the seam read in place, and a file is not one of the owners a claim is re-checked against.

**What is still owed on this surface.** The bijection of the served table against the legacy action
vocabulary: it is a witness check, and P7 forbids reading `crates/voxel-mcp` for it, so it is named owed
rather than approximated. A **pin** still cannot cross a request line (a `claim=pinned` word is refused
`claim_undeclared`, because a pin names a signer and a digest an operator states out of band), and the
row's conformance against the world's own declarations remains owed as `spec.md`'s Assumptions record.
