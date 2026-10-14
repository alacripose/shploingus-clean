# Phase 1 — Interface contracts: U10 the mods-derived gameplay tree

**Feature**: `007-u10-gameplay-tree` | **Date**: 2026-10-12 | **Spec**: [spec.md](../spec.md) | **Plan**:
[plan.md](../plan.md)

The clean room is a **library**: its contracts are the seams its callers use. U10 declares **six** seams
(C1–C6, below) — *"three"* stood here against the five sections that were already here; the count is
corrected rather than left beside the list, 2026-10-12 — and, like `contracts/u6-seams.md`, each is stated as
the *refusal* it must produce as well as the value it must return (P2). **Two of them are U6's and are not
re-opened** (C2's row and vocabulary, C4's wire): a tree row *is* the row U6 admits, so U10 declares what it
*consumes* rather than a second row type.

## C1 — The tree: one entry per reference, and the record's disposition for each

```text
Tree::new(entries: Vec<Entry>) -> Tree        // in the record's own order (docs/MODS-MAP.md §1–§5)
Tree::entries() -> &[Entry]
Tree::first() -> Option<&Entry>               // must be a §1 world-shape-and-streaming entry (FR-008)

Entry { reference: String, disposition: Disposition, row: Option<Row>, waits_on: Option<String> }

Disposition { Mapped, Open, Refused, Unresolved }
Disposition::ALL: [Mapped, Open, Refused, Unresolved]     // four, and COUNT == ALL.len() by test
Disposition::name() -> &'static str                       // "mapped" | "open" | "refused" | "unresolved"
Disposition::parse(name) -> Option<Disposition>           // a word outside the four is nothing

Refusals:
  - a reference the record does not carry     → the map's own absence, named (an entry is built FROM the
                                                map, so a reference outside it cannot be an entry)
  - a disposition the map does not give       → *nothing*: it is not read as the nearest of the four
  - an entry whose disposition is open or unresolved but which carries a row
                                              → a tree that admits an owed reference; **refused**
                                                (the name is owed — D7's sibling; see "What is owed")
```

**Contract.** The tree is **closed over the reference set**: `MODS-MAP` §1–§5 is the whole corpus, and an
entry count that disagrees with the map's own count is a refusal, not a rounding. Order is **data**: the
first entry is the one the repository's first target needs (`MODS-MAP` §1's load-bearing class, quoted in
FR-008), and a tree whose head is an industrial content slot is refused by the bar rather than by taste.

**Contract — an owed entry is a report, never a silence.** An `open`/`unresolved` entry carries **no row**
and is printed with what it waits on where the record says. `docs/MODS-MAP.md` §6's own rule is the reason,
quoted: *"every one of them is named in §2–§4 so that its absence from the build is visible rather than
silent."* And `Establishment::is_established` (*"only `Active` is [a pass]"*, `clean-room/src/intake.rs`)
is what makes a report no pass.

## C2 — The row is U6's row, and the closed vocabulary is U6's vocabulary

```text
// Consumed, unchanged, from clean-room/src/intake.rs and clean-room/src/bridge/table.rs:
Row::new(feature, Source, Option<Licence>, Option<&str> carrier, Option<Acceptance>, Claim, Requirements)
Source::new(path, digest)                 // read IN PLACE; a row never carries a source's bytes
Licence { Permitted, Refused }            // the verdict the row states; the gate does not decide it
Carrier { U3, U4, U5, U6, U7, U8 }        // the closed six the plan's §16 names
Acceptance { Provided(Source), Owed }     // the row's own check — or owed, shown by name
Requirement<T> { Named(Vec<T>), Owed }    // what it requires of the world's declarations
Requirements { stages: Option<…>, materials: Option<…> }   // absent = requires nothing
Claim { ChecksumOnly, pinned }            // ChecksumOnly admits and reports that no identity was proven

// The one table row every tree row crosses through (clean-room/src/bridge/table.rs):
admit path=… digest=… feature=… licence=… claim=… carrier=… check=… check_digest=…
      [requires_stages=a[+b…]] [requires_materials=id[+id…]]        // the last two are OPTIONAL
```

**Contract — no new word is added to a U6 vocabulary.** The three dispositions are **the record's own**
(`docs/MODS-MAP.md` §1/§3/§4) and live on the *entry*; they are not an eighth `Axis`, because a verdict's
axes are **positional** (`clean-room/src/intake.rs`'s `Axis::ALL` comment: *"an axis inserted in the middle
would silently relabel a report that already exists"*), and an axis is a question about **a row** while a
disposition is a question about **a reference** — an `open` reference has no row to judge.

**The refusals a tree row produces, by name (all already built in `clean-room/src/refusal.rs`):**

| Fact | Refusal name |
|---|---|
| the reference's licence verdict forbids derivation | `licence_refused` |
| a licence word outside the declared two | `licence_undeclared` |
| the source document is not readable at the path it names | `source_absent` |
| the bytes under that path are not the bytes the row describes | `source_digest_mismatch` |
| the carrier is not one of U3–U8 | `carrier_undeclared` |
| the row's own acceptance check does not exist yet | `axis_inactive` (the check declared **owed**) |
| the acceptance document has moved | `source_digest_mismatch` (on the acceptance axis) |
| a required stage the roster does not declare | `stage_undeclared` **with the stage named** |
| a required material the registry never admitted | `unadmitted_material` **with the id named** |
| a claim a request line cannot carry | `claim_undeclared` |
| a required argument absent, or a value not of its kind | `argument_absent` / `argument_malformed` |
| **a second row for a descendant technology** | **owed** — `docs/MODS-MATERIALS.md` §8.2 states the rule and no reference names the word (D7) |

## C3 — The recipe document: the row's declared form, consumed in place by reference

```text
The one document that exists (packages/alpha-shell.shspec):
    shploingus.recipe/3                     # the version tag, one declared line, first
    package <name> <version>                # the document's own identity
    window|tab|bar|filed|pin …              # row kinds /3 declares, fields indented beneath

The compiled form (packages/BLD-0119-*.shpkg):
    magic SHPKG002, then a shploingus.recipe/3 body:
    package from-ticket v0
    row BLD-0119 <prose>                    # a one-row recipe, compiled

A gameplay-tree row is a NEW ROW KIND in this closed format (docs/LEDGER.md R-64.1:
    "the recipe format is defined by this repository, versioned, and closed — documents pass through it").
    Its KEY and its VERSION NUMBER are OWED: nothing in the references decides either (D8).
```

**Contract — a reference, never bytes.** The document is consumed **in place**, by `path + digest` — the
`Source` the `admit` row already carries (`contracts/u6-transport.md`: *"the document and its acceptance
check each arrive as a **reference** (`path + digest`), never as bytes"*). A document that inlines a
source's bytes is refused rather than read.

**Contract — where the roots are, and what they are not.** The mod trees are declared in
`intake/roots.tsv` as the `corpus` root `mod-sources` (`F:/shploingus-mod-sources`), and that file's own
header says what a root is not: *"an import list. Nothing here is consumed by being named — `ingest` reads
names and first pages, writes nothing, and a record only ever lands by a patch that says so."* The tree
obeys that: the mods are cited **through the harvest** (`docs/MODS-MATERIALS.md`, `_harvest/*`), and no mod
source is read to build a row (D10).

**Contract — a document's version is a contract, not a comment.** `docs/LEDGER.md` C-64 is the defect that
put a document out of step with the format (*"`alpha-shell.shspec` did not match UI-SPEC §2"*), and it is
why R-64.1 closes the format. A tree document that declares a version the reader does not know is refused
by the reader rather than half-read.

## C4 — The wire: a tree row reaches a client over the form now served

```text
serve(gate, input, output)                    // exists, U6 (src/transport.rs)
reference::serve_stdio(roster, workers, …)    // exists, the one declared world
read_response_for(input, expected)            // exists, the client's own reader

The line protocol, unchanged (contracts/u6-transport.md):
    status ⇥ ok | err
    request ⇥ <the line as received>
    record ⇥ <events>:<head hex | ->          the record AFTER the call
    receipt ⇥ <receipt wire>                  ok only — the whole receipt, one field
    kind ⇥ <refusal kind name>                err only
    detail ⇥ <the refusal's own words>        err only
    <blank line>                              end of response
```

**Contract — one row per request line, and no second door.** A tree row crosses exactly as `admit` does:
the same ten arguments, the same refusal split (a line that does not parse **writes nothing**; a
world-level refusal is **recorded once** and passed through by name), and the same receipt with its seven
axes **in the declared order**. `admit` declares `changes_record: false`, so a tree row appends nothing and
the record's host-stability is untouched: two hosts with different working directories still produce one
head and one refusal detail per line.

**Contract — the tree itself is not a verb *yet*, and that is named rather than shrugged at.** U6's table is
**closed and bijective** and a verb is a contract change; no design declares a verb that returns the tree,
so none is invented (D9). The tree's **rows** are served today; a whole-tree verb is **owed**.

## C5 — Which of the mods' own features the tree must reproduce

The features come from the record's own columns, not from this spec's judgement: `docs/MODS-MAP.md` §1–§4's
*"What it gives this repo"* column, `docs/MODS-MATERIALS.md` §8.1's deduped **rule library** (a technology is
*"a rule that transforms matter"*), and `docs/DESIGN-SPEC.md` §3's twenty-four slots.

| Class (`MODS-MAP` §) | The features it must reproduce | Carried by (`Carrier`) |
|---|---|---|
| **§1 World shape and streaming** — *"the load-bearing class"*, the first target | chunk infrastructure (parallel generation, thread-safe access: `MOD-c2me`); vertical extent (cubic sections: `MOD-cubicchunks`); LOD horizon (`distant-horizons`, `voxy`); pre-generation (`MOD-chunky`); flora placement (`DynamicTrees`/`DynamicTreesPlus`); tides (`MOD-tidal`, **open**) | U4 (residency, horizons), U3 (stages) |
| **§2 Industrial and content** (S-01…S-24) | the technology rule library of `MODS-MATERIALS` §8.1 — heat (smelting, coking, bloomery, blast conversion, steam), comminution (crushing, milling), sizing (sifting, washing), separation (centrifugation, electrolysis, extraction, distillation, fermentation), forming (casting, pressing, wire drawing, hammering), forging/joining (anvil work, welding), textiles, assembly, agriculture, arboriculture, chemistry, weathering, pyrolysis, masonry, glass, ceramics, woodworking, electronics, instrumentation, storage/logistics, construction | U3 (shapers/stages), U5 (instruments), U6 (the gate), U7/U8 (later) |
| **§3 Presentation, performance and appearance** | **one architecture**, never a per-mod port: instanced/indirect batch submission with per-chunk culling; allocation-free hot paths and bounded caches; dynamic block-emitted light (`SodiumDynamicLights`) | U1/U8 (appearance and shell — already ruled in `docs/APPEARANCE.md`) |
| **§4 UI, audio, ecology, worldgen content** | read-the-world overlay (`Jade`); one index both directions (`JustEnoughItems`, `searchables`); bulk inventory gestures (`MouseTweaks`); pickup merge (`pick-up-notifier`); per-option dirty-state (`reeses-sodium-options`); material-as-a-first-class-property footsteps (`Presence-Footsteps`); per-material rain (`Cool-Rain`); spatialised ambient emitters (`matmos-ambient-sound`); per-material absorption/reverb (`sound-physics-remastered`); seasonal clock (`SereneSeasons`); weathering + named repair (`Immersive-Weathering`); biome triple (`Ecologics`); flocking (`boids-reforged`, `Boids-Birds`); dense-ore dials (`DenseOres`) | U3 (`ecology`, `climate`, `geology`), U5 (audio/instruments), U8 (UI) |

**Contract — the smaller reading wherever two references disagree.** `MODS-MAP`'s own adaptation boundary is
the rule and is quoted, not paraphrased: C2ME and CubicChunks are *"MIT and admissible **as architectural
references only** — *use the ideas; do not import engine-specific code; do not assume their thread model is
correct.*"* Where `MODS-MAP` says a class is **one architecture**, a per-mod row is refused; where
`MODS-MATERIALS` §8.2 says a technology is **one rule with many citations**, a per-descendant row is refused
(the refusal's **name** is owed — C2's last row).

**Contract — the features the record refuses are not reproduced at all.** `MODS-MAP`'s refusals
(`BigGlobe`, `lithostitched`), the no-licence set (`MFR`), the highest-caution reference
(`Create-The_Factory_Must_Grow`: *"not cloned and not read"*), and the excluded minecraft-model set
(`docs/MODS-MATERIALS.md` §5: naquadah, enderium, slime, mythril, the adamantite sets, Create's invented
stone names, the kelp-belt mechanics, the rotational power model) are **named so their absence is visible**,
and the licence verdict is per reference with no blanket grant.

## C6 — The census: the record's counts, and its own split read off its rows

```text
Tree::declared() -> Dispositions                 // the four counts, in the record's own four words
Dispositions::entries() / mapped() / open() / refused() / unresolved()
Dispositions::sums_to_entries() -> bool          // mapped+open+refused+unresolved == entries
Tree::sections() -> [usize; 4]                   // §1..§4, READ FROM docs/MODS-MAP.md IN PLACE
Dispositions::render(sections: [usize; 4]) -> String     // the census line: the split is PASSED IN

MapRows::read() -> Result<MapRows, RefusalSet>   // the module's ONE parse of the map's tables
MapRows::read_at(root) -> Result<MapRows, RefusalSet>
MapRows::parse(text) -> MapRows                  // §1's table, §2's groups + dated blocks, §3/§4's rows
MapRows::named() -> &BTreeSet<String>            // the references the map's own rows name
MapRows::sections() -> [usize; 4]                // first-mention counts, §1..§4
MapRows::names(reference) -> bool                // folding the spellings the map itself prints
MapRows::section_5() -> &[String]                // §5's list — rows already counted above it

Refusals:
  - the map is not there to be read in place     → `source_absent`, the path named
  - a printed split that disagrees with the rows → impossible by construction: `render` takes the
                                                   derived digits and states none of them itself
```

**Contract — the split is an argument, never a literal.** The census line's per-section partition is derived
at runtime from `docs/MODS-MAP.md`'s own rows — `MapRows` is the module's **one** parse, and the SC-001 check
consumes it rather than carrying a second one — and is handed to `render`. No constant on the print path may
state it, so a row moved across sections moves the printed digits, and the check that pins them
(`10 §1 + 42 §2 + 19 §3 + 35 §4`) fails by name.

**Contract — this reader does not verify the pin, and *where* the pin is enforced is named.** `MapRows`
answers one question of whatever bytes are there — *what do the rows say?* — which is exactly what makes a
moved document visible in the split instead of hidden behind a digest. The bytes are proven to be the pinned
document elsewhere: by the row's own declared `Source`, verified by the gate at admission
(`source_digest_mismatch`) — on which the binary's exit status depends — and by
`the_declared_digests_are_the_documents_on_the_disk`. Measured, not asserted: append one byte to the map and
the printed digits are unchanged while the run refuses `source_digest_mismatch` at exit 1.

## What is owed on this surface (named, never invented)

1. The recipe document's **row-kind key and version number** (C3) — the format is closed and no reference
   chooses the key or the number.
2. The **name of the refusal** for a second row on a descendant technology (C2, C5; D7).
3. A **verb that serves the whole tree** (C4) — U6's table is closed and no design declares one.
4. The **per-row family assignment** and the `SUBSTANCES` table shape (`docs/MODS-MATERIALS.md` §5, §8.3) —
   a design sketch the record itself calls *"falsifiable, to be built after the materials Rust"*.

**Put to the user 2026-10-12, and still owed.** All four above — and `mc-ar-mod` (`github.com/mt1006/mc-ar-mod`,
the copy's one URL no section of the map names, for which no licence has been read and no disposition taken) —
were put to the user as questions on 2026-10-12 and **no ruling was returned**, so every one of them **stands
owed**, unchanged. None is decided here: FR-014 and SC-006 require the undecided parts to be printed as owed
rather than invented (D7 for the word, D8 for the key and its number, D9 for the verb).
