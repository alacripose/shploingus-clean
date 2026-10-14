# Phase 1 — Interface contracts: U9 saves, loads and the world's own seed systems

**Feature**: `006-world-saves-and-seeds` | **Date**: 2026-10-12 | **Spec**: [spec.md](../spec.md) | **Plan**: [plan.md](../plan.md)

The clean room is a **library**: its contracts are the seams its callers use. U9 exposes three, and each
is stated as the *refusal* it must produce as well as the value it must return — a seam that cannot
refuse is not a seam (P2).

## C1 — Write a world down: the exact encoding

```text
Save::of(world: &World, chain: &Chain, ruleset: Ruleset) -> Result<Vec<u8>, RefusalSet>

Save::bytes(&self) -> Vec<u8>          // re-encode a parsed save; byte-identical to the input

Refusals:
  - chain_broken             the record handed in does not verify — a save never carries a record it
                             could not re-adopt
  - (any refusal the chain's own wire form makes, passed through by name)
```

**Invariant — the encoding is a value.** Two saves of one world at one tick are **identical bytes**, and
`parse(bytes).bytes() == bytes` for every save the writer produced. The layout is the one in
[data-model.md](../data-model.md), and no field depends on the work that produced it.

**Invariant — a save records inputs (F1).** Identity, the five stream seeds, the ruleset identity and
the record. Nothing derived: not the truth digest, not the resident set, not the anchor.

## C2 — Read it back: the parser

```text
Save::parse(bytes: &[u8]) -> Result<Save, RefusalSet>

Refusals:
  - not_a_save       wrong magic, wrong version, or a width the layout does not declare
  - save_truncated   a framed part that runs past the end of the bytes
  - save_tampered    the trailer digest is not the digest of the bytes before it
  - seed_undeclared  a domain the loader declares that the save omits; a domain the save names that
                     the loader does not declare; a domain named twice
```

**Invariant — integrity first.** The trailer is checked before any field is interpreted, so a corrupt
byte string is refused as corrupt rather than parsed into a nearly-right world.

## C3 — Load: the identity correspondence, and what a caller gets

```text
Save::load(&self, expected: Ruleset) -> Result<Loaded, RefusalSet>

Loaded  { world: World, chain: Chain, report: LoadReport }
  world  = World::new(WorldIdentity::new(identity.seed, DeclaredPeriod::seconds(identity.period)?))
           .with_tick(Tick(identity.tick))          // identity rebuilt, then the tick moved onto it
  report = LoadReport { identity, tick, truth: world.truth_digest(), record_len, record_head, streams }

Refusals:
  - identity_mismatch  the ruleset/revision differs from `expected` (both named)
  - identity_mismatch  a declared stream seed is not the world's own for that domain (the domain and
                       both seeds named)
  - record_diverged    the parsed record's head or length disagrees with the save's declaration
  - chain_broken       the record does not verify after parsing (U2's own refusal, passed through)
```

**Invariant — the loaded world is the world.** `report.truth` equals the live world's `truth_digest`,
and a world generated from the loaded identity produces **one** digest at 1/2/4/8 workers — the
instrument U3 already measured (`460214b1…2fc6`) and U4 repeated (`44a682e1…4ffb` — §40's reading, which
reads the chunk's own ring; §39's face merge was `2db62b78…b79c`, and `0e7358e4…4ed8` was the height
merge's own digest, kept dated in plan §17/§38).

**Invariant — no partial load.** `Loaded` is produced only when every check passed; there is no
"loaded with warnings" and no world built from a save that failed a check.

## C4 — What U9 does not expose, and why

*(Corrected 2026-10-12, §28: the first two bullets below have landed — the `save`/`load` rows **are** in
`bridge::TOOLS`, as its seventh and eighth, and the document form exists, read through U6's content gate.
They are kept as written because the ruling they record — that both are **U6's door** and the gate is
hard for the document form — is what the implementation then followed. What landed is stated under them.)*

- **No `save`/`load` verb in the tool table.** The table is closed at five verbs (`bridge::TOOLS`); two
  more is a **U6** change and is named owed (D7). A caller reaches this slice through the library and
  the binary's evidence, not through a second door (P5). **Landed 2026-10-12 (§28):** the two rows are
  the table's seventh and eighth, both by reference (`path`, and for `load` the `digest` the bytes must
  hash to), and the replay form's library surface is unchanged behind them.
- **No document form.** A save that crosses the API as a document consumed in place by reference
  (`path + digest`) is owed, and **that** is where U6's content gate (`admit`/`inspect → Established`)
  becomes a hard prerequisite (§16.2). Nothing here half-builds it. **Landed 2026-10-12 (§28):**
  `src/save/document.rs` holds the one writer (`Document::write`) and the one reader
  (`Save::read_document`), the reader having no `fs::read` of its own — it goes through
  `Intake::read`, so the hard prerequisite is a property of the code rather than a rule a caller could
  forget. This is the file that was written to satisfy this row, not written beside it.
- **No residency policy.** What stays resident is U4's and U11's; a save is the record plus identity.
- **No second record.** `save`/`load` read and carry U2's `Chain`; they never keep their own copy of
  the world's history, and a load never appends (a read is not a change — F1).
