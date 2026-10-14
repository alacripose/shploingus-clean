---
patch_id: PATCH-022
title: The MCP surface rides the api layer — the bridge's gate, the chain as the record, the api viewer
status: applied
operation: add
target: shploingus/mcp-api-layer
base_ref: shploingus-clean@main
base_digest: sha256:b69b66ce6e8a835ec6ce073d8256b7fa035d640ad4b9b0e45c66a6321a368fd1
dependencies: [PATCH-021]
source: user-directed-conversation
created_at: 2026-10-09
approval: user-directed
live_mutation: false
---

# Change

PATCH-021 gave the gate a read surface and **skipped the api layer it was supposed to join**. It then
got worse before it got better: the next version filed records into `logs/laya-decisions/`, the
shell's notification stream, and that was wrong for a reason the tree states in its own words.

This patch puts the surface on the three declarations it belongs on, and each replacement is
measurable against the thing it replaced.

## 1. It carried its own action map; the tree already has a gate

`voxel-package::bridge` **is** the tool gate, and it is complete: [`bridge::TOOLS`] is a table
bijective onto the action vocabulary (`TOOLS_ARE_TOTAL`), `Capability` is a partition *over* that
vocabulary (four names, one action belongs to none), `bridge::call` runs **coverage → capability →
matrix** in that order, and every call returns a [`bridge::Receipt`] carrying `checks_ran`, because
*a check that did not run is not a check that passed*.

So `action_of`/`gate` are gone. `row_of` names the bridge row each served tool is a call to — the
five reads to `read_governance`, `get_conformance` to `lint_line`, `get_api_view` to `read_recipe`,
`reject_non_conformance` to `append_event` — and `receipt()` asks the bridge. A tool with no row is
refused by **coverage**, and the whole refusal is the bridge's sentence:

```
get_vibes → "the surface carries no tool named \"get_vibes\"; a missing row is a refusal, never an
            allow (23 tools exist) — checks ran: none"
```

Nothing is implicit, including reading: a request that presents no capability gets
`Authority::none()`, and the refusal names what is missing (`lint_line needs the observe capability
and this session does not hold it; silence is not permission`). Capabilities travel as
`"capabilities": ["observe", "step"]` in the request; filing additionally needs a grant
(`"granter"`, `"scope"`), both halves required.

## 2. It filed into the game's per-run queue

**That queue is a projection, and the game clears it.** `laya::queue_dir` → `Notification` records
read by `shell::notifications::NotificationStream`, which re-hashes every artifact before showing it
— but `write_the_gate_queue` calls `clear_the_previous_run` on **every world open**, and its own
header says why: *"the chain is the record, and this queue is a projection of one pass over it"*.
Record names are positional (`laya-NNNN`), so a leftover is not merely stale; **the next run reuses
its name.** Twelve rejections filed there would have been wiped by the next world open.

So the surface writes nothing into that queue, and the crate does not link `voxel_model::laya` at
all — pinned by a test, because the mistake is easy to make twice.

## 3. The record is the chain, and a refusal has a kind

`EventKind::RefusalRecorded` is declared — *"a refusal nobody can read is a refusal that did not
happen"* — and `bridge::record_refusal` is a declared writer for it. `reject_non_conformance`:

1. reads the chain back from `chains/workspace.chain` (`bloat::read_chain`) — **never builds one
   fresh**, because an event linked to a genesis that is not its predecessor is refused on read-back
   as `ChainBroken`, correctly;
2. appends one `RefusalRecorded` event per rejected cell, each carrying
   `crate/axis`, state, the census's reason, and the evidence origin in the ladder's own words
   (*"B — read back from world state"*) — the `B` rung, because this project read its own tree;
3. asks `filing::append` to persist, and **lets `filing` decide**.

Step 3 is where the honest answer is a refusal rather than a workaround. `filing`'s rule is
**one `.chain` file per workspace**, and it refuses a second filing by design: *"the second append has
nowhere legal to go until a segmenting rule is **ruled** rather than invented."* This workspace's
chain is already filed (785 events, 1.5 MB), so the twelve events are built, linked and readable, and
the run reports:

```
reject-recording	12
refusal-recorded	voxel-model	documented	refused	index 785
... eleven more ...
reject-created	refusal_recorded
reject-chain-head	ef466843dbe15f20e4ea3e9912ba5556a204cfac6e61c82530d8640fc0a0fd76
reject-not-filed	1 refusal(s): record_exists: .../chains/workspace.chain already exists
reject-not-filed-reason	the chain is one file per workspace and segmenting it is a rule the record
                        has not ruled; the events above are built, linked and readable, and
                        persisting them is that ruling's to make
```

**The refusal is the finding, not a defect in the tool.** Filing these twelve on the chain is exactly
the ruling the record has not made — upstream's own shape (*record the refusal once, then refuse only
what is not on the record*) is what that ruling would implement.

## 4. The api viewer gets a producer

`get_api_view` renders the document `window[api-viewer]` is for, **from declarations only**: the
served tools, their bridge **rows**, those rows' actions, and `policy::permission_of`'s class. No
sentence about the surface is written by hand, so the document cannot advertise a permission the gate
does not enforce — and the test asserts each line against `bridge::TOOLS` and `policy`.

# Acceptance

- A name the surface does not carry is refused by **coverage** with `checks ran: none` — a check that
  did not run cannot have passed.
- Reading needs `observe` and nothing else; a request presenting nothing is refused, naming the
  capability.
- Filing is the row `append_event`: `RequiresGrant` in the matrix, and the ungranted refusal is the
  matrix's own sentence. **All three checks ran** before it (coverage, capability, matrix).
- A refused filing **changes nothing on disk**: the test compares the chain file's bytes before and
  after.
- With a grant, one `RefusalRecorded` event is appended per rejected cell, each linked
  (`previous == Linkage::Linked(predecessor.digest)`), each with a distinct digest, and the set of
  cells recorded equals the set the rule refused.
- The crate links neither `voxel_model::laya` nor the shell's queue reader; a test pins it.

# Evidence

- `cargo test --offline -p voxel-mcp` → **18 passed, 0 failed** (8 in `tests/api_layer.rs` written
  against the bridge and the chain, 10 in `tests/protocol.rs`, whose tool calls now present
  `observe` and whose unknown-tool assertion is the coverage sentence).
- The live session, driven through the built binary as a client drives it
  (`logs/mcp-api-layer-session.txt`, `exit=0`, **stderr empty**):
  - no capabilities → `lint_line refused — checks ran: tool_has_a_row — lint_line needs the observe
    capability...`;
  - `observe` → `conformance refused`, 12 refused, 12 rejected, 0 unchecked;
  - `observe` + `step`, no grant → `append_event refused — checks ran: tool_has_a_row,
    capability_held, matrix_permits — append_event is requires_grant and no grant was presented;
    silence is not permission`;
  - `observe` + `step` + `{"granter":"ala","scope":"conformance/reject"}` → 12 `refusal_recorded`
    events on a chain of 797 events, and `reject-not-filed` with `filing`'s own reason.
- The workspace after the session: `chains/workspace.chain` byte-identical, `logs/laya-decisions/`
  back to its own 226 records, no record of this surface's anywhere in it.

# Rollback

Base bytes: `Cargo.toml` at `shploingus-clean@main`, git blob
`sha256:b69b66ce6e8a835ec6ce073d8256b7fa035d640ad4b9b0e45c66a6321a368fd1`. Rollback is deleting
`crates/voxel-mcp/` and `crates/voxel-package/src/conformance.rs` plus the `crates/voxel-package`
member lines; nothing else was written. The chain was never modified, so reverting removes the
surface and not one event of the record.

# Not done, and named

- ~~**The build door is still not fatal.** The verdict is served and refused-on-demand; the build
  passes with 12 refusals.~~ **Closed after this patch, and the decision was taken.**
  `crates/shploingus/build.rs` now refuses the build when anything is rejected or unchecked, reusing
  the census it had already taken (`conformance::of`, so the tree is walked once) and naming each
  refusal with the file to change. **The twelve `documented` refusals are declared accepted** on
  `conformance/accepted-refusals.txt`, in the open: they are still counted and still printed
  (`conformance-refused 12` / `conformance-accepted 12` / `conformance-rejected 0`), so accepting
  them is a refusal somebody owns rather than one nobody sees. That is the half of the decision that
  is still owed — twelve sections of real documentation, one per crate — and it is named in
  `docs/TODO.md` §9 rather than implied by the clean build.
- **The twelve events are not on the chain.** `filing` refuses a second filing by design and the
  segmenting rule is a ruling, not an invention this patch may make. Until that ruling exists, a
  rejection is built, linked and readable for the length of one call.
- **`bridge::TOOLS` carries 23 tools and this server serves 8.** The extra 15 rows are the shell's
  and the world's; the MCP surface is a subset of the bridge, not a bijection onto it, and no test
  pins that subset against the bridge's totality.
- **`get_census` and `get_work_list` are slow** — they walk the tree and are not cached between
  calls. The measurement is right; the cost is unmeasured.
