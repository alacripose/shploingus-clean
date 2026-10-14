# Phase 1 — Quickstart: validating U10

**Feature**: `007-u10-gameplay-tree` | **Date**: 2026-10-12 | **Spec**: [spec.md](./spec.md) | **Plan**:
[plan.md](./plan.md)

**Read this first: the slice is built, and every number below was measured in the pass that built it** — the
thirteenth pass, whose plan is `../docs/CLEAN-ROOM-PLAN-2026-10-11.md` §23. The pass before it wrote this
specification and measured nothing; nothing of *that* pass's state survives here except the bar, which is
quoted rather than paraphrased.

```bash
cd clean-room                     # the clean-room crate, beside the legacy tree
cargo test --offline              # exit 0: 226 passed, 0 failed
cargo clippy --offline --all-targets -- -D warnings   # exit 0, no diagnostics
cargo run --offline               # exit 0, 444 lines of stdout (was 278)
find ../crates -type f -newer src/gameplay.rs         # 0 files: crates/ untouched (P7)
```

**Prerequisites**: no network and no mod source. The harvest (`docs/MODS-MATERIALS.md`, and the `_harvest/*`
artifacts it names under `F:/shploingus-mod-sources/`) is the reading, already done and HEAD-pinned; the
crate's only dependency is `sha2` (`cargo tree --offline`).

**What the numbers are, in one line each** (the binary prints them, and this file copies them):

| Measured | Value |
|---|---|
| entries, one per reference in `docs/MODS-MAP.md` §1–§5 | **106** (10 §1 + 42 §2 + 19 §3 + 35 §4; §5's list and the map's repeats fold to one entry per reference) |
| dispositions (the record's own four words) | `mapped=64 open=25 refused=5 unresolved=12` — **summing to 106** |
| the record's own figures, printed beside them, un-averaged | **106 references named in `docs/MODS-MAP.md` §1–§5** (10 §1 + 42 §2 + 19 §3 + 35 §4) — i.e. **the map's own count is the entry count above**, which is SC-001 measured rather than asserted; the file the map was written from (`references/shploingus_dirty/references.md`) **91 URLs**; this repo's copy (`docs/references.md`) **97 URLs**; **9 §2 `*` marks** (the tree derives twelve §2 `open`: the nine plus `optical`, `create-new-age` and `EE 3`, each named in the legend); **43 `open` rows** (the prior disposition file) |
| rows, one per technology | **43**, of which **23 carry more than one citation** (the double-cover rule) and **35 declare their own bar as owed** |
| U6's own census over those rows | 43 rows over 7 axes (**301 cells**): **177 established, 121 inactive, 0 unchecked, 3 refused — 8 of 43 admitted** |
| the refusals, by the owner's own names | `licence_refused×3`, `axis_inactive×35` |
| entries that are owed (no row, never admitted) | **37 of 106** |
| tests / clippy / run | 226 passed · clean · exit 0 |
| `crates/` files touched | **0** |

**Corrected 2026-10-12 (user), after the pass above.** Rows of `docs/MODS-MAP.md` were wrong and are carried
corrected in its header and in the entries: `tt20` is a **performance mod** (a TPS fixer), not the
network/tick budget; `resquake` is the **Squake movement model** — with `Minecraft-CameraOverhaul` added to
the game's player controller (`docs/LEDGER.md` C-150), **not in the clean room** — so its entry leaves
`unresolved` for `open` with no row assigned; and the Archipelago row is **re-keyed** to the artifact the
reference points at — the Modrinth **datapack** `archipelago` (`modrinth.com/datapack/archipelago`), world
generation (island/archipelago terrain) — whose own terms declare `CC-BY-NC-ND-4.0` (non-commercial, **no
derivatives**), so its entry moves from `open` to **`refused`** and `MinecraftFabricArchipelagoMod` is
demoted to the replaced URL it is: the Fabric client that interacts with the Archipelago randomiser, a
different artifact. That correction moved eleven stdout lines — named line by line in §23 of
`docs/CLEAN-ROOM-PLAN-2026-10-11.md`.

**Corrected again 2026-10-12 (user), after that.** Three references had no entry of their own, and now have
one, each with its evidence stated in the map's own row for it: `TerraFirmaCraft` and `Calculator` (both in
`docs/references.md`, cited all through `docs/MODS-MATERIALS.md` §8.1, but never in the map's §2 slot table)
and `Minecraft-CameraOverhaul` (whose only trace was inside the `resquake` note, although `docs/LEDGER.md`
C-150 names it as part of the player controller). Read from each artifact's own source: `TerraFirmaCraft` →
**`mapped` — `smelting`** (clone `6e77733c`, `LICENSE.txt` **EUPL-1.2**), `Calculator` → **`mapped` —
`smelting`** (clone `c796f1dc`, `LICENSE` **MIT**, © 2018 Ollie Lansdell), and `Minecraft-CameraOverhaul` →
**`open`** (**GPL-3.0**, `LICENSE.md` in its own repository — adopted into the game's player controller with
`resquake`, not in the clean room, so the record assigns it no row, as with `resquake`). Where a description
and a name disagree the **smaller reading** is taken: both `mapped` entries are filed under each reference's
**first** §8.1 rule (furnace firing / smelting), because the harvest's passage states a whole processing
chain for a reference while the rule library states rules one at a time — a feature is a rule. Counts:
entries 103 → **106**, `mapped` 62 → **64**, `open` 24 → **25**, owed `36 of 103` → **`37 of 106`**; every row,
the census over the rows and the four owed rulings are **untouched** (no new row, axis or verb). **Ten
stdout lines moved and three were added** (441 → **444**), named line by line in §23.

**Settled 2026-10-12 (user) — SC-001's "91 versus 106".** The criterion was reported unmet because the map's
header declared 91 URLs while the tree derived 106 references, with the header itself declining to say why.
Counted from the sources, the two numbers turn out to describe **two different documents**, and the map's own
rows are a third set that is larger than both: the header's **91** is the count of the file the map was
written from (`references/shploingus_dirty/references.md`, in the sibling `shploingus-game` tree — 91 URL
lines, verified); **this repo's** `docs/references.md` holds **97** URL lines (the same file minus three —
`C2ME`, `CubicChunks` and the replaced `MinecraftFabricArchipelagoMod` — and plus nine, of which six are
given rows here and three are the two non-mod links and `mc-ar-mod`); and the map's own **106** rows are the
94 URLs the copy carries a row for (91 by exact slug, 3 by spelling) plus the 12 rows it does not carry
two dropped from the copy and the ten Tekkit mods a dated ruling added. **The header was the wrong part and
is corrected** — it now states its own count (106, with the arithmetic of all three files) — so SC-001 is
**measured true**, and `mc-ar-mod`, the one mod URL no section of the map names, is named **owed** in the
map's §6 rather than being invented into an entry or dropped. A new test reads the map in place and holds the
equality both ways.
**Five stdout lines moved and none was added** (444 → **444**): the `[U10]` registry witness, `U10
documents`'s map digest, `U10 census`'s record-figures tail, `U10 bar`'s head digest, and `U10 in place`'s
tampered digest pair — named line by line in §23.

**Corrected once more 2026-10-12 (user) — two trailing figures, and the split guarded.** The map's §2 legend
said `(24 of them)`; no source reproduces 24 — the §2 marks are **nine**, the tree derives **twelve** §2
`open` entries (the nine, plus `optical`, `create-new-age` and `EE 3`, each for the reason its own entry
states), and the prior disposition file's own figure is the **43 `open`** rows printed beside it. The legend
now states the nine and names the three, with the correction dated in place, and the census prints **9 §2
rows carry the record's own `*`** where it printed **24 `open` in §2**. §5's heading records that one of its
eight slots was re-keyed, and the README line now describes the map's own scope (106 references, reconciled
in its header) rather than the copy alone. The SC-001 check, which guarded only the total, now re-derives the
header's **per-section split** (`10 §1 + 42 §2 + 19 §3 + 35 §4`) **in place**: moving §3's `shuttfup`/`Stfu`
row into §4 failed by name (`[10, 42, 17, 37]` against the stated split), and the map was restored
byte-exact. **Four stdout lines moved and none was added** (444 → **444**): `U10 documents`'s map digest,
`U10 census`'s §2 figure, `U10 bar`'s head digest, and `U10 in place`'s declared + tampered digest pair.
Nothing else in the 444 lines differs, and the suite stays **226 passed / 0 failed** — the SC-001 check
gained assertions, not a function.

**The census's split, now derived (2026-10-12, user).** The one figure on the census line that was still a
literal — `10 §1 + 42 §2 + 19 §3 + 35 §4` — is now read off `docs/MODS-MAP.md`'s own rows at runtime:
`gameplay::MapRows` is the module's **one** parse of the map's tables (the parse this slice's SC-001 check
used to carry beside it), `Tree::of_record` reads it **in place** and carries the digits, and the census line
takes them as an argument. **The printed bytes are unchanged** — 0 stdout lines moved (444 → **444**, `cmp`
byte-identical), and the census paste below still diffs byte-equal to the run. The derivation is proved
**live** by moving §3's `shuttfup`/`Stfu` row into §4: the line then prints `17 §3 + 37 §4` and the check
fails by name (`[10, 42, 17, 37]`), after which the map was restored byte-exact (sha256 `11873cad…`). The
suite is **228 passed / 0 failed**.
**The census's seam declared; the rulings asked (2026-10-12, user).** `contracts/u10-seams.md` now carries **C6** —
the census's interface, the split passed to `render` instead of stated on the print path, `MapRows` as the module's
one parse, and the reader's own non-verification of the pin with *where* the pin is enforced instead (the gate at
admission, and the digest test) — and the file's stale seam count (*"three"* beside five sections) now names what
it declares. The four owed rulings and `mc-ar-mod` were put to the user as questions the same day and no ruling was
returned, so they stand owed unchanged.

## The bar (quoted, not paraphrased)

`docs/CLEAN-ROOM-PLAN-2026-10-11.md` §5 row 10:

> *every row names its document, its licence verdict and the system that carries it; a refused reference is
> refused by name; the tree's first row is playable*

## Scenario 1 — the tree declares every reference, and counts what it declares (SC-001, SC-002)

```bash
cargo run --offline | grep "^U10"
```

```text
U10 tree: 106 entries, one per reference in `docs/MODS-MAP.md` §1–§5 in the record's own order, and 43 rows, one per technology
U10 documents: `docs/MODS-MAP.md` (11873cad81c2af8d) · `docs/MODS-MATERIALS.md` (6ddffde0fa7be61e) · `docs/SHAPERS.md` (426b58d7f518aef6) · `clean-room/specs/004-u4-mesh-stream-work/quickstart.md` (ca00f1bef771d739) — each row reads its own where it lives, never a copy
U10 census: entries=106 mapped=64 open=25 refused=5 unresolved=12 (mapped+open+refused+unresolved = 106) — the record's own figures, un-averaged: 106 references named in `docs/MODS-MAP.md` §1–§5 (10 §1 + 42 §2 + 19 §3 + 35 §4) = the entry count above; the file the map was written from (`references/shploingus_dirty/references.md`) 91 URLs; this repo's copy (`docs/references.md`) 97 URLs; 9 §2 rows carry the record's own `*`; 43 `open` rows in the prior disposition file
U10 entry: MOD-c2me mapped feature=chunk-infrastructure licence=permitted carrier=U4 source=docs/MODS-MAP.md check=clean-room/specs/004-u4-mesh-stream-work/quickstart.md
U10 entry: MOD-cubicchunks mapped feature=vertical-extent licence=permitted carrier=U4 source=docs/MODS-MAP.md check=clean-room/specs/004-u4-mesh-stream-work/quickstart.md
U10 entry: distant-horizons mapped feature=lod-horizon licence=permitted carrier=U4 source=docs/MODS-MAP.md check=clean-room/specs/004-u4-mesh-stream-work/quickstart.md
U10 entry: voxy mapped feature=lod-horizon licence=permitted carrier=U4 source=docs/MODS-MAP.md check=clean-room/specs/004-u4-mesh-stream-work/quickstart.md
U10 entry: MOD-chunky mapped feature=pre-generation licence=permitted carrier=U3 source=docs/MODS-MAP.md check=owed
U10 entry: DynamicTrees mapped feature=flora-placement licence=permitted carrier=U3 source=docs/MODS-MAP.md check=docs/SHAPERS.md
U10 entry: DynamicTreesPlus mapped feature=flora-placement licence=permitted carrier=U3 source=docs/MODS-MAP.md check=docs/SHAPERS.md
U10 entry: MOD-tidal open (owed: unreviewed; candidate for `voxel-shapers::hydrology` (tides))
U10 entry: BigGlobe refused feature=world-shape licence=refused carrier=U3 source=docs/MODS-MAP.md check=owed (ARR + CC-BY-NC on earlier versions, explicit no-AI clause: readable to understand a problem, and nothing derived may be implemented)
U10 entry: lithostitched refused feature=world-structure licence=refused carrier=U3 source=docs/MODS-MAP.md check=owed (no licence declared (verified 2026-09-25))
… one line per reference, §1 first, then §2, §3 and §4 — 106 lines in all (the last is `archipelago refused feature=world-shape licence=refused …`) …
U10 rows: 43 technologies, one row each; 23 carry more than one citation (the record's own double-cover rule) and 35 declare their own acceptance check as owed
U10 row: chunk-infrastructure carrier=U4 licence=permitted check=clean-room/specs/004-u4-mesh-stream-work/quickstart.md cited_by=MOD-c2me
… one line per technology, 43 in all, each naming its carrier, its verdict, its check and the record's citations …
U10 row: coking carrier=U3 licence=permitted check=owed cited_by=Railcraft+Create-Pantographs-and-Wires
U10 row: steam-generation carrier=U3 licence=permitted check=owed cited_by=SteamPowered+Railcraft+MFR+GregTech-Modern refused_citations=MFR
```

**(§39, 2026-10-13 — U4's quickstart moved):** the quoted line above carries U4's document digest, and that
document gained §39's dated note on the mesh's unit (the faces between a solid cell and air), so the
declared, read-in-place digest moved `481412a537b29391…` → `b2bad61036a1af84…` in `gameplay.rs`'s
`U4_QUICKSTART_DIGEST`. The guard that made the move visible is
`the_documents_the_rows_cite_are_the_ones_on_the_disk`, and it is named here rather than left to a reader of
the 2026-10-12 capture.

**(§40, 2026-10-13 — U4's quickstart moved again):** §40's dated note on the extent contract (a footprint's
own columns and span, with the ring that decides its faces) moved the same document's bytes once more, so the
quoted line above now carries `ca00f1bef771d739…` where §39 wrote `b2bad61036a1af84…`, and
`U4_QUICKSTART_DIGEST` in `gameplay.rs` was updated in the same commit as the note.

**How it is decided.** `N = 106` is the count the map's own §1–§5 name, and **SC-001's equality is met**:
the header states the same 106 (`**The map's own reference count: 106 references**`, 10 §1 + 42 §2 + 19 §3 +
35 §4), and a test re-derives it **from the document in place** — reading the rows, folding the spelling
variants the map prints, and comparing the two sets **in both directions** — so an entry the map does not
name, or a row the tree drops, fails by name. The three file-level counts are a different question and are
printed as such, never soldered into the equality: the file the map was written from holds **91 URL lines**
(the figure the header used to quote, correct for *that* file), this repo's copy holds **97**, and the
reconciliation is written out in the header — **88** URLs both files carry, **+6** the copy adds that this
map gives rows, **+3** spelling variants folding onto one row each = **97**; and **94** rows carry a URL in
the copy, **+12** rows it does not carry (`C2ME`, `CubicChunks`, and the ten Tekkit mods the dated 2026-10-03
ruling added) = **106**. `mapped+open+refused+unresolved = 106` — so an `open` entry can never be read as a
`mapped` one (tasks.md deviation 4, now a resolution). The header's per-section split — **10 §1 + 42 §2 + 19 §3 + 35 §4** — is re-derived in place too, from the section that first names each reference, so a row moved across sections fails by name.

## Scenario 2 — a refused reference refuses by name, and an owed one is never admitted (SC-003)

```bash
cargo test --offline --test u10_gameplay a_refused_reference
cargo test --offline --test u10_gameplay an_absent_document
```

```text
U10 bar: the refused reference `BigGlobe` is refused by name (`licence_refused`) = true; the permitted head `MOD-c2me` admits = true
U10 owed references: 37 of 106 entries are named owed and carry no row (never admitted = true)
U10 in place: the same row over the same bytes admits = true; one byte moves and the same row refuses `source_digest_mismatch` = true (… the bytes at docs/MODS-MAP.md hash to 610f13c7…, and the row declares 11873cad… — these are not the bytes the row describes); a reference that is not there refuses `source_absent` = true (… the declared reference `docs/not-a-document.md` could not be read in place: no such document)
```

**How it is decided.** `licence_refused` is the gate's own name raised at the licence axis
(`clean-room/src/intake.rs`), passed through rather than re-worded; refused rows carry no cargo (nothing
required of the world, and their own bar owed). `feature` is a **required** argument of the `admit` row, which
is why a reference with no feature **cannot** be a row; and a licence word outside the declared two refuses
`licence_undeclared` **with the declared two in the message** (the test walks that fault too).

## Scenario 3 — a tree row reaches a client over the served wire form (SC-005)

```bash
cargo test --offline --test u10_gameplay a_tree_row_crosses
```

The test spawns the real binary in `serve` mode **in the record's root** as a separate process, sends one
`admit` line for the tree's first row, and checks the served answer against the one derived here over the same
line: the answer digest is equal, the seven axes are
`[active, active, active, active, active, inactive, inactive]` (identity and conformance are **reported**,
never counted as passes), the effect appends nothing, and the record stays at 0 events. `licence=maybe` on the
same line refuses `licence_undeclared` and is recorded once.

## Scenario 4 — the bar itself (SC-006, SC-007)

```bash
cargo test --offline                                   # exit 0: 226 passed, 0 failed
cargo clippy --offline --all-targets -- -D warnings     # exit 0, no diagnostics
cargo run --offline                                     # exit 0: 444 lines, U10's block + the four OWED lines
find ../crates -type f -newer src/gameplay.rs           # 0 files — crates/ untouched
```

The four owed parts are printed (`grep "^U10 OWED"`), each appears again in this document set — `spec.md`'s
Assumptions, `plan.md`'s *What this slice does not land*, `research.md`'s refusals and the module header — and
SC-006's check is a diff of the lists, not a reading of one.

## What the documentation pass before this one could not measure

It measured nothing, and said so: no tree existed, so no count was taken. That pass's own text is preserved in
`tasks.md`'s *What the specification pass did*; what it prepared is what this pass ran.
