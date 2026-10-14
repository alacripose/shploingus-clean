# Phase 1 — Data model: U9 saves, loads and the world's own seed systems

**Feature**: `006-world-saves-and-seeds` | **Date**: 2026-10-12 | **Spec**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

## What U9 owns, and what it only consumes

| Concern | Owner | How U9 uses it |
|---|---|---|
| world identity — `seed`, declared period, tick | U2 (`World`, `WorldIdentity`, `DeclaredPeriod`, `Tick`) | read; carried in the save; never re-derived from a constant |
| the closed set of deterministic sub-streams | U2 (`Domain`, `Domain::ALL`, `Domain::COUNT`) | the save declares **one seed per declared domain**; a domain the loader does not declare is refused |
| the per-domain seed | U2 (`World::stream_seed`, **new**) | read for every declared domain and compared with the save's |
| the record — one append-only ledger | U2 (`Chain`) | carried in its own wire form, framed; never mirrored, never re-implemented |
| content identity | U2 (`Digest`) | the trailer and every declared digest; there is no second hash |
| the refusals | U2 (`RefusalKind`, `Refusal`, `RefusalSet`) | six new kinds in the existing closed vocabulary; every problem collected |
| the roster and the worker-count digest | U3 (`process::roster`, `world_generation_digest_with_workers`) | the acceptance instrument: one digest at 1/2/4/8 workers |
| the written form of a world | **U9** | this feature |

## The entities

| Entity | Fields | Rule |
|---|---|---|
| `Ruleset` | `ruleset: u64`, `revision: u64` | the configuration/ruleset identity a save names; a load under another is refused `identity_mismatch` with both |
| `StreamSeed` | `domain: Domain`, `seed: u64` | **one per declared domain**, carried; a missing or undeclared domain ⇒ `seed_undeclared`; a seed that is not the world's own ⇒ `identity_mismatch` naming the domain and both values |
| `SaveIdentity` | `seed: u64`, `period: u64`, `tick: u64` | immutable across a load: nothing is clamped, repaired or normalised |
| `Save` | `identity`, `ruleset`, `streams: Vec<StreamSeed>`, `record_head: Option<Digest>`, `record_len: u64`, `record_wire: String` | the parsed written form; `bytes()` re-encodes it and must reproduce the input exactly |
| `Loaded` | `world: World`, `chain: Chain`, `report: LoadReport` | what a load returns: the world rebuilt from the identity, the record, and the numbers it was checked with |
| `LoadReport` | `identity`, `tick`, `truth: Digest`, `record_len`, `record_head`, `streams` | what the acceptance checks read; `truth` is **derived** at load time, never read from the save |

## The layout, exactly

Little-endian throughout; variable-length parts framed by their length; every field in this order and
no others. `u32`/`u64` are the widths stated; a digest is 32 raw bytes.

```text
offset  width            field
0       18               magic = b"shploingus.save/1"
18      4                version = 1
22      8                ruleset.ruleset
30      8                ruleset.revision
38      8                identity.seed
46      8                identity.period
54      8                identity.tick
62      4                stream count  (must be Domain::COUNT = 5)
66      count × (1 + 8)  per stream: u8 domain tag, u64 seed       — in Domain::ALL order
        ...              record head:  1 byte present flag, then 32 bytes if present
        ...              8               record length (events)
        ...              8               record wire length, then that many UTF-8 bytes
        last 32          trailer = Digest::of(every byte before it)
```

**Why this shape** (D3): a framed layout makes truncation a refusal rather than a misread; the fixed
order makes the encoding a value; and the trailer turns "one byte changed" into a named fault instead
of a slightly different world. The record rides in the chain's **own** wire form as an opaque framed
blob, so there is exactly one record format in the crate (P5) rather than a save-specific copy of it.

## The fault-to-refusal matrix

| Fault | Refusal | What is named |
|---|---|---|
| bytes that are not a save (wrong magic, wrong version, wrong widths) | `not_a_save` | which field, and what was expected |
| a framed part that runs past the end of the bytes | `save_truncated` | the field and both lengths |
| the trailer digest does not match the bytes before it | `save_tampered` | the recorded and the recomputed digest |
| a declared domain the save omits | `seed_undeclared` | the domain |
| a domain the save names that the loader does not declare, or names twice | `seed_undeclared` | the tag and the reason |
| a declared stream seed that is not the world's own at this identity and tick | `identity_mismatch` | the domain, the saved seed, the world's seed |
| a ruleset/revision different from the one the load declares | `identity_mismatch` | both rulesets, both revisions |
| the parsed record's head or length disagreeing with the save's declaration | `record_diverged` | the declared and the found value |

**Every fault is collected** (`RefusalSet`), so a caller sees all of them in one pass rather than one
per round trip (P2). **No fault produces a world**: `Loaded` exists only when every check passed.

## What the save deliberately does not carry

- **the world truth digest** — derived from the identity (D1), and printed by the load report instead;
- **the resident set / any geometry** — a cache, and U4's to derive;
- **the anchor** — a view choice, invisible to truth by U2's own test;
- **a schema version history** — one version, one layout, and bytes that are not this layout are
  refused rather than migrated (FR-008);
- **anything from `crates/`** — no legacy save format is the contract (P7).
