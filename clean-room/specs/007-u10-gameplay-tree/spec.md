# Feature Specification: U10 — The mods-derived gameplay tree

**Feature Branch**: `007-u10-gameplay-tree`

**Created**: 2026-10-12

**Status**: **Built (2026-10-12, thirteenth pass).** This document was written from the record, not from any
mod's code, in a specification-only pass; the slice it prepares then landed — `clean-room/src/gameplay.rs`,
`src/evidence.rs`'s `check_u10`, `tests/u10_gameplay.rs` and the registry's U10 status. The measured numbers
are in [quickstart.md](./quickstart.md) and in `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §23: **106 entries**
(mapped 64, open 25, refused 5, unresolved 12), **43 technology rows** (23 double-covered), U6's own census
reading **8 of 43 rows admitted** with zero unchecked axes, and 226 tests / clippy clean / `cargo run` exit 0
with `crates/` untouched. Where this document says *"this pass"* about the specification pass, it is kept as
the record of that pass; the built slice's own record is §23 and `tasks.md`. The record's corrections (2026-10-12, user —
`tt20`, `resquake`, and the Archipelago row re-keyed to the datapack `docs/references.md` points at) are
carried in `docs/MODS-MAP.md`'s header and in the entries, which is why the census reads `open 25` /
`refused 5` / `unresolved 12`: `resquake` leaves `unresolved` for `open` with no row assigned, and the
datapack's own terms (`CC-BY-NC-ND-4.0`, no derivatives) move its entry from `open` to `refused`. **A second
correction (2026-10-12, user) gave entries to the three references that had none** — `TerraFirmaCraft` and
`Calculator` (`mapped`, at `smelting`: clones `6e77733c` / `c796f1dc`, licences **EUPL-1.2** / **MIT**, read
from the clones' own licence files) and `Minecraft-CameraOverhaul` (`open`, **GPL-3.0**, adopted into the
game's player controller with `resquake` and not in the clean room, so the record assigns it no row) — which
moves the counts to **106** entries (`mapped 64`, `open 25`) and **`37 of 106`** owed, with every row, the
census over the rows and the four owed rulings untouched.

The behaviours below come from the **U10** entry of `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §16 (= row
10 of §16's system list, bar 10 of §5), from `docs/MODS-MAP.md` (the disposition map) and
`docs/MODS-MATERIALS.md` (the harvest and its technology roster), from `docs/DESIGN-SPEC.md` ruling 3 and
§3, and from the one recipe document that exists (`packages/alpha-shell.shspec`) plus the recipe ruling
`docs/LEDGER.md` **C-64** / **R-64.1**. **No `.rs` under `crates/` was read** (P7), and **no file under
`F:/shploingus-mod-sources/` was read in this pass** — the harvest is cited as the document that already
recorded every reading (`docs/MODS-MATERIALS.md`), which is what the tree's `source` field then points at.
The one rule is inherited whole: *refuse, never default*.

**Input**: User description: "the full minimal gameplay tree derived from the mods references — what a
gameplay-tree row is and what it may declare, the closed vocabulary and refusal names it needs, its
acceptance bar, how it reaches a client over the wire form now served, and which of the mods' own features
it must reproduce."

**Governed by**: `.specify/memory/constitution.md` (P1–P7, read live) and
`docs/CLEAN-ROOM-PLAN-2026-10-11.md` §16 *U10 — The gameplay tree the mods references imply* (its hard
prerequisite and its interface), §16.2 row 10 (why it sits here) and §5's row-10 bar.

## What U10 is, and what it is not

U10 is the **tree**: the ordered set of playable systems the mods references imply, as **one declared list
of entries**, each naming the document it is owed by, the licence verdict it relies on, the system that
carries it and its own acceptance check. It is **not** a port of any mod (P7), and it is **not** a second
content registry: U6's content gate (`admit`/`inspect → Established`, `clean-room/src/intake.rs`) already
admits a declared row **by reference**, and this feature is the **tree of those rows**, not a new gate.

Four boundaries are inherited whole:

1. **A row is an admission, never a transcription.** Deriving from a reference is an *admission* (plan
   §16 row 10's *"its hard prerequisite is U6's content gate: deriving from a reference is an admission,
   never a transcription"*). So a tree row **is** the row U6 already admits — the same value, the same
   seven axes, the same refusals.
2. **The licence verdict is per reference and the gate does not decide it.** `MODS-MAP` §1's own header:
   *"a verdict is per reference … No blanket rule exists, and no milestone may grant one."* The row states
   the verdict its source was given; a `Refused` one refuses by name (`licence_refused`, already built).
3. **An `open` reference is not a row.** `MODS-MAP`'s own words: *"An `open` mod is not a rejection: it
   means the record assigned no feature"* (§2), and *"every one of them is named in §2–§4 so that its
   absence from the build is visible rather than silent"* (§6). A row must name a feature; a reference with
   no feature, or with a purpose the record explicitly left undecided, therefore **cannot be a row** and is
   declared as an **entry that is owed**, not admitted.
4. **Nothing from a refused reference is implemented.** `MODS-MAP`'s 2026-10-03 ruling: every reference is
   *readable for ideas*, and *"the verdicts govern derivation, never reading. Nothing from a refused
   reference is copied or implemented."*

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The tree declares every reference, and each entry says what it is (Priority: P1)

A reader is handed the tree: one **entry per reference** the record maps (`MODS-MAP` §1–§5, **106 references**
— the header's own count, which is the tree's entry count),
in the record's own order, each carrying the record's **disposition** for it — `mapped`, `open`, `refused`
or `unresolved` — and, for a `mapped` one, the **row** it declares. Nothing is silent: an entry that is owed
says so and says who owes it, and an entry that is refused says so **by name**.

**Why this priority**: it is the whole point of the system. Without a tree whose absences are visible, the
first target (*"a fully playable basic world"*, `docs/DESIGN-SPEC.md` ruling 3) is judged against a system
list nobody can enumerate.

**Independent Test**: walk the tree and print one line per entry — its reference id, its disposition, and
(for a row) its feature, source, licence, carrier and check — and assert that every `MODS-MAP` entry
appears exactly once, that every `mapped` entry has a row, and that no `open`/`unresolved` entry has one.

**Acceptance Scenarios**:

1. **Given** the record's disposition map, **When** the tree is built, **Then** every reference in
   `MODS-MAP` §1–§5 appears exactly once, and the entry count equals the map's own count.
2. **Given** an entry whose disposition is `open` or `unresolved`, **When** it is read, **Then** it carries
   **no row** and names itself (and, where the record says so, what it waits on) as **owed**.
3. **Given** an entry whose disposition is `mapped`, **When** it is read, **Then** it carries a row that
   names its feature, its source (`path + digest`), its licence verdict, its carrier (U3–U8) and its own
   acceptance check.

### User Story 2 - A refused reference is refused by name, and the row carries no code (Priority: P2)

Two references are **refused** for derivation on the record's own evidence — `BigGlobe` (*"ARR + CC-BY-NC
on earlier versions, explicit no-AI clause"*) and `lithostitched` (*"no licence declared"*) — and three more
sit in the no-derivation set for their own facts: `MFR` (*"no licence file anywhere in the tree"*), the
`archipelago` datapack (*"its own terms: CC-BY-NC-ND-4.0 — non-commercial, no derivatives"*), and
`Create-The_Factory_Must_Grow` against the highest caution (*"explicit no-AI clause — not cloned and not
read"*). An entry that declares a refused verdict **refuses by name** rather than being admitted, and the
refusal is the gate's own (`licence_refused`), already built at U6.

**Why this priority**: the refusal is the feature. A tree that admits a refused reference is a tree that
has licensed something nobody decided, and `MODS-MAP`'s ruling leaves no room for a blanket grant.

**Independent Test**: build a tree entry whose licence is `refused` and assert the admission refuses with
**`licence_refused`** (the owner's own name) while a `permitted` sibling admits; assert no refused entry
carries a cargo of any kind.

**Acceptance Scenarios**:

1. **Given** a `refused` entry, **When** it is admitted, **Then** the refusal is `licence_refused` and the
   entry's `licence` axis is `refused`.
2. **Given** a row whose `licence` word is `maybe`, **When** it is parsed, **Then** the refusal is
   `licence_undeclared` with the declared two in the message — a word outside the two is not nearly-right.
3. **Given** a row whose source document is not readable at the path it names, **When** it is admitted,
   **Then** the refusal is `source_absent`; a document whose bytes have moved is `source_digest_mismatch`.

### User Story 3 - The tree's first entry is playable, and it streams (Priority: P3)

The tree is **ordered**, and the first entry is the one the repository's own first target needs: the
load-bearing class `MODS-MAP` §1 names — *"World shape and streaming — the load-bearing class. This is the
class the first target ('a fully playable basic world') directly needs."* Its features are the chunk
infrastructure and the vertical extent (`MOD-C2ME`, `MOD-cubicchunks`, both MIT and *"admissible as
architectural references only"*), carried by U4's residency and U3's stage graph, and its acceptance check
is the bar U3/U4 already measure (`docs/SHAPERS.md` §1's shapers and their papers).

**Why this priority**: the plan's bar for this slice says *"the tree's first row is playable"*; a tree whose
first row is an industrial content slot would be a tree in the wrong order.

**Independent Test**: read the tree's first entry and assert it is a `mapped` row of §1's class, that its
carrier is a declared system, and that its check is the shapers' own measured bar rather than a new one.

**Acceptance Scenarios**:

1. **Given** the tree, **When** its entries are read in order, **Then** the first is a §1 (world shape and
   streaming) row carried by U4 (or U3), not a §2 industrial content slot.
2. **Given** that row, **When** its check is run, **Then** the check is the one the carrying system already
   declares (`docs/SHAPERS.md` §1's paper-per-shaper contract), not a bar invented for the tree.

### Edge Cases

- **A reference with no feature** (`open`, per `MODS-MAP` §2's 24): **not a row**, declared owed; the tree
  counts it and prints it, and never admits it.
- **A reference the record explicitly did not decide** (`unresolved`, e.g. `shuttfup`/`Stfu`,
  `AcceleratedRecoiling`, `consistent-block-drops`): the same disposition as `open` in
  *shape* (named, owed) with the record's own word — **not guessed** (`MODS-MAP` §3/§4: *"Unresolved —
  recorded, not guessed"*).
- **A reference with no disposition row anywhere** (`MODS-MAP`'s three unaccounted URLs: `sound-physics-
  remastered`, `immersivethunder`, `SodiumOptionsAPI`): the entry exists (the map gives them rows), and its
  disposition is whatever the map now says — never inferred from the URL's absence.
- **A reference that appears twice under two names** (the **double-cover rule**: an older mod and its
  Create-addon descendant): **one row**, both citations kept — `MODS-MATERIALS` §8.2 measures this as the
  roster's own rule (*"the roster must never grow a second row for a descendant"*). A second row for a
  descendant is **refused**, and the refusal's **name is owed** — no reference names the word, so none is
  coined here (see Assumptions; `research.md` D7).
- **A row that requires a stage the world does not run**: refused `stage_undeclared` **with the stage
  named** (U6's seventh axis, already built).
- **A row that requires a material the registry never admitted**: refused `unadmitted_material` **with the
  id named**.
- **A row whose own acceptance check does not exist yet**: the check is declared **owed** (`Acceptance::
  Owed`), the acceptance axis is `inactive` (`axis_inactive`), and the row is **not** counted as admitted.
- **A row whose check's document has moved**: `source_digest_mismatch` on the acceptance axis.
- **A row whose carrier is not one of U3–U8**: `carrier_undeclared` — the closed set is what makes that a
  fact rather than a guess.
- **A claim that establishes no identity** (`checksum_only`): admitted **and reported** (`MODS-MAP` is
  silent on claims; U6/LEDGER §55.3 is not: *"`ChecksumOnly` **admits** the package and reports that no
  identity was proven"*), never counted as verified origin.
- **A recipe document that does not declare the tree's version tag**: refused by the document's own reader
  before any world is read (`malformed_wire` or the document format's own name — **owed**, see
  Assumptions).
- **A recipe document that carries bytes instead of a reference**: the tree's rule is *consume in place by
  reference* (`path + digest`); a document that inlines a source's bytes is refused rather than read.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The tree MUST declare **one entry per reference** in `docs/MODS-MAP.md` §1–§5, in the
  record's own order, and MUST NOT invent a reference the map does not carry.
- **FR-002**: Every entry MUST carry the record's **disposition** for its reference, from the closed set
  the record itself uses — `mapped`, `open`, `refused`, `unresolved` — and MUST NOT carry a disposition the
  map does not give it.
- **FR-003**: A `mapped` entry MUST declare the **row** U6 already admits: feature, source (`path +
  digest`, read **in place**), licence verdict, carrier (one of U3–U8), its own acceptance check (`path +
  digest`, or declared **owed**), a claim, and what it requires of the world (U3's roster, U2's registry;
  absent = *requires nothing*).
- **FR-004**: An `open` or `unresolved` entry MUST declare **no row** and MUST name itself **owed**, so its
  absence from the build is visible rather than silent (`MODS-MAP` §6's own rule).
- **FR-005**: A `refused` entry MUST be refused **by name** (`licence_refused`, the gate's own, already
  built) and MUST carry no cargo.
- **FR-006**: A licence word outside the declared two MUST be refused `licence_undeclared` **with the
  declared two in the message** — never read as nearly-permitted.
- **FR-007**: The tree MUST hold **one row per technology** under the record's own double-cover rule
  (`MODS-MATERIALS` §8.2): an older mod and its addon descendant are **two citations on one row**, never
  two rows.
- **FR-008**: The tree MUST be **ordered**, and its first entry MUST be a `MODS-MAP` §1 (world shape and
  streaming) row, because that is the class the repository's first target needs.
- **FR-009**: A row's own acceptance check MUST be the check the **carrying system already declares**
  (`docs/SHAPERS.md` §1's paper-per-shaper contract; U6's `admit` `check`/`check_digest`), and MUST NOT be
  a bar invented for the tree.
- **FR-010**: A row's **source** MUST be the document the harvest **already wrote** (`docs/MODS-
  MATERIALS.md` §2/§7/§8, `_harvest/*` under `F:/shploingus-mod-sources/`), cited by `path + digest`; the
  tree reads no mod source to build itself.
- **FR-011**: The tree MUST reach a client over the **wire form now served** — the `admit` row of
  `bridge::TOOLS`, served by `serve` over the line protocol declared in
  `specs/005-u6-record-api/contracts/u6-transport.md` — and MUST NOT require a second door.
- **FR-012**: A row admitted through the served line MUST carry the same seven axes and the same refusals
  U6 already declares (`feature`, `source`, `licence`, `carrier`, `acceptance`, `identity`, `conformance`);
  no axis is added and no axis is reordered, because a verdict's axes are positional.
- **FR-013**: `sha2` MUST remain the crate's **only** dependency, and no `crates/*` path may be imported
  (P7); **no mod source may be read to build the tree** (the harvest is the reading, already done).
- **FR-014**: Every part of this specification that the record does **not** decide MUST be named **owed**
  in `plan.md`, `research.md` **and** this file's Assumptions, rather than chosen — the recipe document's
  row-kind spelling and version, the duplicate-row refusal name, and the verb that serves the whole tree.

### Key Entities *(include if this feature involves data)*

- **`Entry`** (new): one reference of the tree — the reference id (`MOD-c2me`, `voxy`, …), the record's
  **disposition** for it, and (for a `mapped` one) the row it declares. It is the unit the tree counts.
- **`Disposition`** (new): the record's own four words — `mapped`, `open`, `refused`, `unresolved` — each
  taken from `docs/MODS-MAP.md` (§1's legend, §3/§4's *"Unresolved"*) rather than coined here.
- **`Row`** (exists, U6): `clean-room/src/intake.rs`'s declared row — feature, `Source`, `Licence`,
  `Carrier`, `Acceptance`, `Claim`, `Requirements`. **U10 adds nothing to it**: the tree's row *is* this.
- **`Source`** (exists, U6): a document's `path + digest`, read in place — the one way a row cites anything.
- **`Licence`** (exists, U6): the two verdicts `permitted`/`refused`; `Refused` refuses `licence_refused`.
- **`Carrier`** (exists, U6): the closed six — U3, U4, U5, U6, U7, U8 — the systems that carry a row.
- **`Acceptance`** (exists, U6): the row's own check, `Provided(Source)` or **`Owed`** — so a row whose bar
  cannot be run yet is shown by name and is never a pass.
- **`Requirement`/`Requirements`** (exists, U6): what a row requires of the world's own declarations.
- **The recipe document** (exists, the legacy format): `shploingus.recipe/3`, the versioned, closed document
  form (`packages/alpha-shell.shspec`; ruling `docs/LEDGER.md` **R-64.1**), which is the form the tree's
  entries are written in and consumed in place by `path + digest`.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001** The tree's entry count equals `MODS-MAP`'s own reference count, and every reference appears
  exactly once. *Decided by:* the printed per-entry lines and the count beside the map's own count — and,
  since 2026-10-12, by `the_maps_own_reference_count_is_the_trees_entry_count`, which reads the map **in
  place**, folds the spelling variants the map itself prints, and holds the equality **both ways** (no entry
  the map does not name, no named row the tree drops) against the header's own stated count **and its
  per-section split** (`10 §1 + 42 §2 + 19 §3 + 35 §4`), so a row moved across sections fails by name.
  *(Measured: **106 = 106**, with the three file-level counts — the map's source's 91 URL lines, this repo's
  copy's 97, and the map's 106 rows — reconciled step by step in the map's header rather than averaged; the
  one URL in the copy with no row anywhere, `mc-ar-mod`, is named owed there and derives no entry.)*
- **SC-002** Every entry's disposition is one of the record's four words, and every disposition's **count**
  is printed — so an `open` entry can never be read as a `mapped` one. *Decided by:* the census block.
- **SC-003** A `refused` entry refuses **by name** (`licence_refused`) while a `permitted` sibling admits.
  *Decided by:* the two printed lines, one per entry.
- **SC-004** The tree's first entry is a §1 world-shape-and-streaming row, and its check is the carrying
  system's own. *Decided by:* the first line of the tree print.
- **SC-005** A row reaches a client over the **served `admit` row** with its seven axes intact: a served
  admission's answer digest and axes equal the one derived in-process. *Decided by:* the served session's
  receipt against a world the test built itself.
- **SC-006** Every part the record does not decide is printed as **owed** by the binary and named in this
  spec's Assumptions. *Decided by:* the `owed` line and a diff of the two lists.
- **SC-007** `cargo test --offline` passes, `cargo clippy --offline --all-targets -- -D warnings` is clean,
  and `cargo run --offline` exits 0 with the tree's measurements — **when the slice is built**. *(Measured in
  the slice's pass: 226 passed / 0 failed, clippy exit 0, `cargo run` exit 0 at 444 lines of stdout, with
  `find crates -type f -newer src/gameplay.rs` returning nothing. See `quickstart.md`.)*

## Assumptions

- **The content gate is the prerequisite, and it is built.** `admit`/`inspect → Established` landed in
  `clean-room/src/intake.rs` (§18), the served row in §19 (`admit` is a row of `bridge::TOOLS`), and the
  conformance axis in §21. What is **owed** of U6 — the document record and its compression, the structural
  judge, the ballot, the permission policy matrix, the API document, a pin over a request line — is U6's and
  is **not** re-opened here. *(Corrected 2026-10-12, §29: the **permission policy matrix** has since landed
  too — so this tree's rows are now judged by the policy chokepoint on their way through the gate; what
  stays owed of U6 is the judge, the ballot, the API document, a pin over a request line and the matrix's
  own remainder.)*
- **What U10 owes, named rather than chosen** — each of these appears in `plan.md` § *What this slice does
  not land* and in `research.md`:
  1. **The recipe document's row-kind spelling and version.** The one document that exists
     (`packages/alpha-shell.shspec`) opens `shploingus.recipe/3` and declares `window`, `tab`, `bar`,
     `filed` and `pin` rows, plus the compiled form's ticket `row` (`packages/BLD-*.shpkg`, magic
     `SHPKG002`). A gameplay-tree entry is a **new row kind**, and the ruling `docs/LEDGER.md` **R-64.1**
     says the format is *"defined by this repository, versioned, and closed"* — so the new kind is a
     **version question**, and **nothing in the references decides the number or the key**. Named owed.
  2. **The duplicate-row refusal's name.** `MODS-MATERIALS` §8.2 states the rule (*"never grow a second row
     for a descendant"*) but no reference names the refusal. Named owed; no name is invented here.
  3. **The verb that serves the whole tree.** U6 serves **one row per request line**; the tree is a list,
     and no design anywhere declares a verb that returns it. Named owed. The tree's **rows** do reach a
     client today, over `admit` (FR-011).
  4. **Which of the mods' features are in the first slice.** `MODS-MAP` §2 says the industrial slots are
     *"content scheduled after it"*; `MODS-MATERIALS` §5 ends *"per-row family assignment (the ten
     families) … on the way to the `SUBSTANCES` table shape"*, i.e. the assignment is **owed**. This spec
     names the classes and their sources; the per-row assignment is owed.
- **The mods are read through the harvest, never re-read.** `docs/MODS-MATERIALS.md` §7's sweep is
  complete (A/B/C all accounted) and every clone is HEAD-pinned in
  `F:/shploingus-mod-sources/_harvest/clone-hashes.txt`; the tree cites that document, and re-reading a mod
  to build a row would be re-doing a completed reading.
- **What the world looks like is the user's.** The tree's *mechanism*, its dispositions and its refusals are
  this feature's; the appearance of anything a row carries is the user's (`docs/DESIGN-SPEC.md` ruling 4).
- **Verification is by re-reading the owners.** A tree row's evidence is the claims + the re-establishment
  an admission already gets (`Receipt::verify`); U10 adds no new kind of evidence.

## Sources used, cited by path

Every decision above traces to one of these; nothing is designed from memory.

| Path | What it supplied |
|---|---|
| `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §2 (U6), §5 row 10, §16 (U10, §16.2, §16.3), §18–§22 | the system's definition, its bar (*"every row names its document, its licence verdict and the system that carries it; a refused reference is refused by name; the tree's first row is playable"*), its row form, its hard prerequisite, and every owed list |
| `docs/MODS-MAP.md` §1–§6 | the 91 references, the `mapped`/`open`/`refused` legend, the refusals, the no-licence set, the double-cover and no-minecraft-progression rulings, and §1's *load-bearing class* |
| `docs/MODS-MATERIALS.md` §0, §2, §5, §7, §8.1, §8.2, §8.4 | the earth-model filter, the per-mod harvest, the roster, the complete sweep, the technology rule library, the double-cover proof, and the `[NS]` honesty line |
| `docs/DESIGN-SPEC.md` ruling 3, §2 (F2/F5/F10), §3 (S-01…S-24) | the first target, the feature verdicts, and the twenty-four industrial slots |
| `docs/SHAPERS.md` §1, §2, §4 | the terrain shapers and the paper each is built from, the determinism contract, and what is not here yet |
| `docs/LEDGER.md` §55.3, C-64, R-64.1, C-127 | intake returns `Established` not a bool and collects every problem; the recipe format is versioned and closed; no minecraft progression |
| `packages/alpha-shell.shspec`; `packages/BLD-0119-*.shpkg`; `intake/roots.tsv`; `conformance/accepted-refusals.txt` | the recipe document's real shape, its compiled form's magic and fields, the declared roots (**`mod-sources` = `F:/shploingus-mod-sources`**), and the doctrine that an acceptance list is reported and never hidden |
| `clean-room/src/intake.rs`; `clean-room/src/bridge/{table,request,gate,receipt,wire}.rs`; `clean-room/src/refusal.rs` | the built row, the closed vocabularies (`Licence`, `Carrier`, `Axis`, `Establishment`, `Acceptance`, `Requirement`), the `admit` row's ten arguments, the refusal names, and the served wire form |
| `clean-room/specs/005-u6-record-api/{spec,plan,tasks,data-model,research}.md`, `contracts/u6-{seams,transport}.md`, `checklists/requirements.md` | the shape this document mirrors, and the vocabulary it must not contradict |
| `clean-room/.specify/memory/constitution.md`; `.specify/templates/*.md` | P1–P7 and the spec/plan/tasks/checklist shapes |
| `F:/shploingus-mod-sources/` (via the harvest, not re-read) | the 43 HEAD-pinned trees the harvest cites; the `_harvest/*` artifacts the rows' sources point at |
| `shploingus-game/docs/plan/01-reference-inventory.md` §1 (the six port verdicts), §5, §6 | the *other* plan's verdict taxonomy — cited as a disagreement, whose **smaller** reading is taken (see `research.md` D2) |
