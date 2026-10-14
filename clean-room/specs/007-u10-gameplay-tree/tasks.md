---
description: "Task list for U10 — the mods-derived gameplay tree"
---

# Tasks: U10 — The mods-derived gameplay tree

**Input**: Design documents from `/specs/007-u10-gameplay-tree/`

**Prerequisites**: [plan.md](./plan.md) (required), [spec.md](./spec.md) (required — three user stories),
[data-model.md](./data-model.md), [contracts/u10-seams.md](./contracts/u10-seams.md),
[quickstart.md](./quickstart.md)

**Tests**: included, because this project's rule is that a task is done when its check was run and its
output inspected (Constitution P4). Each implementation task names the check that will decide it.

**Status 2026-10-12 — built, and ticked where its check ran.** The file was written by a
**specification-only** pass (the user's instruction was *"Change no code in this pass …"*), and the slice
landed in the **next** pass, whose instruction was to *"build only what spec 007's plan and tasks declare,
in the smallest slice that stands alone"*. Every task below is ticked because the check it names was run and
its output inspected: `cargo test --offline` (226 passed, 0 failed: 189 lib — 9 of them this module's — plus
7 `u10_gameplay`, and the pre-existing 30 across U4/U6/U9), `cargo clippy --offline --all-targets -- -D
warnings` (clean, exit 0) and `cargo run --offline` (**exit 0, 441 lines**, stdout 278 → 441 with 2 lines
changed and 0 removed — accounted line by line in
`../docs/CLEAN-ROOM-PLAN-2026-10-11.md` §23; **the binary prints 444 lines** after the two 2026-10-12
corrections recorded at the end of this file). `find crates -type f -newer clean-room/src/gameplay.rs`
returns nothing: no `crates/*` file was touched (P7).

**T010–T013 are ticked because what they ask *of this slice* — the recording — happened**: the binary's
`U10 OWED:` lines name all four parts, and a test holds each. The **decisions** they name remain owed to the
record, and they are the four rulings the pass's closing message asks the user for. Nothing was coined to
close them.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependencies)
- **[Story]**: `US1` (the tree declares every reference, and each entry says what it is), `US2` (a refused
  reference is refused by name), `US3` (the first entry is playable and streams)
- Every task names the file it will touch and the check that will decide it

## Path Conventions

Single project — the clean-room crate at `clean-room/`: sources in `src/`, module tests in `#[cfg(test)]`,
the end-to-end check in `tests/`, documents in `specs/007-u10-gameplay-tree/`.

---

## Phase 1: Setup

- [x] T001 Add the **disposition** to the tree's vocabulary — the record's own four words (`mapped`,
      `open`, `refused`, `unresolved`, taken from `docs/MODS-MAP.md` §1/§3/§4), each with its `name`,
      `parse` and a `COUNT` pinned to `ALL.len()` — `src/intake.rs` · *check:* a unit test asserts
      `Disposition::ALL.len() == Disposition::COUNT`, that a word outside the four parses to `None`, and
      that each word's `name` is the record's — the same test shape `Axis`/`Licence` already carry
- [x] T002 [P] Register the tree on the library surface and name it in the crate header —
      `src/lib.rs` · *check:* `cargo clippy --offline --all-targets -- -D warnings` is clean

## Phase 2: Foundational

- [x] T003 The **tree as a value**: `Tree::new(entries, order)`, `Tree::entries`, `Tree::first`, and the
      closed-over-reference-set rule (the entry set is derived from `docs/MODS-MAP.md` §1–§5 and its count
      is compared against the map's own) — `src/gameplay.rs` · *check:* a unit test builds the tree and
      asserts one entry per reference, in the map's order, with no reference invented and none dropped
      (FR-001)
- [x] T004 The **entry**: `Entry { reference, disposition, row: Option<Row>, waits_on: Option<String> }`,
      with the invariant that a `mapped` entry **has** a row and an `open`/`unresolved` entry **has none** —
      `src/gameplay.rs` · *check:* a unit test asserts an `open` entry carrying a row is refused, and a
      `mapped` entry without one is refused (FR-003, FR-004)

## Phase 3: US1 — the tree declares every reference, and each entry says what it is

- [x] T005 The **rows** for the `mapped` entries, built as U6's `Row` (feature, `Source` (`path + digest`),
      `Licence`, `Carrier`, `Acceptance`, `Claim`, `Requirements`) — `src/gameplay.rs` · *check:* a test
      asserts every `mapped` entry's row names all five declarations the plan's §16 row 10 requires, and
      that each `source` is the harvest artifact `docs/MODS-MATERIALS.md` names rather than a mod's own
      path (FR-003, FR-010)
- [x] T006 The **dedupe**: one row per technology under the record's double-cover rule
      (`docs/MODS-MATERIALS.md` §8.2) — an older mod and its addon descendant are two citations on one row —
      `src/gameplay.rs` · *check:* a test asserts a descendant does not produce a second row and that both
      citations survive on the one (**the refusal's own name is owed — T010**)
- [x] T007 The **census of dispositions**: `mapped`/`open`/`refused`/`unresolved` each counted, the four
      summing to the entry count, printed beside the two figures `docs/MODS-MAP.md` itself gives for `open`
      (24 in §2, 43 in the header) — `src/gameplay.rs`, `src/evidence.rs` · *check:* the printed block shows
      `M+O+R+U = N` and both map figures, un-averaged (FR-002, SC-001, SC-002)

## Phase 4: US2 — a refused reference is refused by name

- [x] T008 A `refused` entry admits nothing: the row's `licence` is `Refused` and the admission refuses
      **`licence_refused`** (the owner's own name, raised at the licence axis in `src/intake.rs`), and the
      refusal is recorded once like any other world-level refusal — `src/gameplay.rs` · *check:* a test
      builds a `refused` entry (`docs/MODS-MAP.md` §1: BigGlobe, lithostitched) and a `permitted` sibling,
      asserts the first refuses with `licence_refused` and the second admits, and asserts the refused entry
      carries no cargo (FR-005, SC-003)
- [x] T009 The **licence words**: a word outside the declared two refuses `licence_undeclared` **with the
      declared two in the message**; an absent or moved source refuses `source_absent` /
      `source_digest_mismatch` — `src/gameplay.rs` · *check:* a test walks the four faults and asserts each
      name, with the declared verdicts named in the message (FR-006, FR-009)

## Phase 5: US3 — the first entry is playable, and it streams

- [x] T010 **OWED — a decision, not an edit.** The **name of the refusal** for a second row on a descendant
      technology: `docs/MODS-MATERIALS.md` §8.2 states the rule (*"never grow a second row for a
      descendant"*) and **no reference names the word**. Until the record gives one, the tree refuses with
      the closest **declared** name and prints the gap; it does not coin one —
      `src/refusal.rs` · *check:* the binary's **OWED** line names this part, and the spec's Assumptions and
      `research.md` D7 name it too (SC-006)
- [x] T011 **OWED — a decision, not an edit.** The recipe document's **row-kind key and version number**: a
      gameplay entry is a new row kind in a format `docs/LEDGER.md` R-64.1 makes *versioned and closed*, and
      no reference chooses the key or the number. Until the record decides, the tree cites the existing
      document form (`packages/alpha-shell.shspec`, `shploingus.recipe/3`) and prints the gap —
      `src/gameplay.rs` · *check:* the **OWED** line names this part; nothing is invented (SC-006)
- [x] T012 **OWED — a decision, not an edit.** A **verb that serves the whole tree**: U6's table is closed
      and bijective, and no design declares one. The tree's **rows** are served today over `admit`; the list
      is not, and this task records that rather than adding a verb — `src/bridge/table.rs` · *check:* the
      **OWED** line names this part, and the served-row test (T015) proves the rows still cross (SC-006)
- [x] T013 **OWED — a decision, not an edit.** The **per-row family assignment** and the `SUBSTANCES` table
      shape (`docs/MODS-MATERIALS.md` §5, §8.3 — the record calls its own progression model *"a design
      sketch, falsifiable"*) — `src/gameplay.rs` · *check:* the **OWED** line names this part; no family is
      assigned by this pass (SC-006)
- [x] T014 The **order and the head**: the tree is ordered and its first entry is a `MODS-MAP` §1
      (world shape and streaming) row carried by U4 or U3, whose acceptance check is the one
      `docs/SHAPERS.md` §1 already declares — `src/gameplay.rs` · *check:* a test asserts the head is a §1
      row and that its check is the carrying system's own bar, not one invented for the tree (FR-008,
      FR-009, SC-004)

## Phase 6: The served wire, the evidence and the documents

- [x] T015 The tree's **rows cross the served wire form**: one `admit` line per row, over the line protocol
      of `specs/005-u6-record-api/contracts/u6-transport.md`, with the seven axes in declared order —
      `tests/u10_gameplay.rs` · *check:* the test spawns the real binary in `serve` mode as a **separate
      process**, sends one `admit` line for a `mapped` entry, and verifies the served receipt's answer
      digest and axes against owners it built itself (FR-011, FR-012, SC-005)
- [x] T016 The **evidence** the binary prints: the entry and disposition counts, the first entry, the rows
      admitted, each refusal by name, and the **OWED** line — `src/evidence.rs` · *check:* `cargo run
      --offline` exits 0 and the printed numbers equal this spec's documented ones (U1–U4/U6/U9 unchanged,
      with any stdout delta accounted for line by line)
- [x] T017 [P] `quickstart.md`: paste the measured output; tick nothing that did not run — *check:* every
      number pasted has a command above it and was run in the same pass (the file is explicit today that it
      has none)
- [x] T018 `src/systems.rs`: U10's real status, replacing `absent` — and only what was built — *check:*
      `the_registry_is_closed_and_ordered` still holds, and the status names the four owed parts
- [x] T019 The governing plan's slice-10 row and a new section recording the pass and what U10 still owes —
      `../docs/CLEAN-ROOM-PLAN-2026-10-11.md` · *check:* the documented numbers equal the printed ones, and
      every published digest of U1–U4/U6/U9 reappears unchanged in the same run

---

## Dependencies & Execution Order

- **Setup (Phase 1)** → **Foundational (Phase 2)** blocks the three stories: the disposition and the entry
  are the vocabulary every story is written in.
- **US1 (Phase 3)** is the MVP: without a tree whose entries are counted, there is nothing for the refusal
  (US2) or the head (US3) to be measured against.
- **US2 (Phase 4)** and **US3 (Phase 5)** both need US1's entry; T014 needs T003's order.
- **Phase 6** needs all three, and T019 is last because it grades the whole pass.
- **T010–T013 are `OWED` and are decisions rather than code**: they block the parts of the slice they name
  (the duplicate-row refusal, the document's new row kind, the whole-tree verb, the family assignment) and
  nothing else, because the tree's **rows** cross the wire today.

## What this pass leaves owed (named in the plan and the spec, not hidden here)

- The **recipe document's row-kind key and version** (T011) — the format is closed and no reference chooses
  either.
- The **name of the refusal** for a second row on a descendant technology (T010).
- A **verb that serves the whole tree** (T012).
- The **per-row family assignment** and the `SUBSTANCES` table shape (T013).
- U6's own remainder this slice rides on: the document record and its compression, the structural judge
  with its calibration, the ballot, the crate × axis census over the tree, the permission policy matrix, the
  API document, a pin over a request line, and the un-re-derived rows of §22 — **all U6's, none re-opened**.

## What the specification pass did (2026-10-12, the pass that wrote this file)

It wrote eight documents — `spec.md`, `plan.md`, `research.md`, `data-model.md`, `quickstart.md`,
`contracts/u10-seams.md`, `checklists/requirements.md` and this file — and it **changed nothing under
`clean-room/src` or `clean-room/tests`** (verified at the time: every file there was older than that pass).

## What the slice's pass did (2026-10-12, thirteenth pass) — and the deviations, named rather than smoothed

**Built.** `clean-room/src/gameplay.rs` (new: the disposition vocabulary, the reference table, the technology
table, the entry, the tree, the double-cover dedupe, the owed list), `src/evidence.rs` (`check_u10`),
`src/main.rs` (the bar joins the gate), `src/systems.rs` (U10's real status), `src/lib.rs` (the module and
the crate header), and `tests/u10_gameplay.rs` (six end-to-end tests). **No dependency was added** (`sha2`
stays the crate's only one), no `crates/*` file was touched, and no mod source was read.

**Measured, and what the bar now reads.** The tree holds **106 entries** (one per reference in
`docs/MODS-MAP.md` §1–§5, in the record's order: 10 in §1, 42 in §2, 19 in §3 and 35 in §4 — the map's §5
list and its repeated mods fold to one entry each) with the record's own dispositions
(`mapped=64 open=25 refused=5 unresolved=12`, the four summing to 106) — and **43 rows**, one per technology,
**23 of them carrying more than one citation** (the double-cover rule's own shape: `coking` is one row cited
by Railcraft and Create-Pantographs-and-Wires, `flora-placement` one row cited by DynamicTrees and
DynamicTreesPlus, and §8.2's own proof is measured on `cited_by`). Through U6's own census the rows read
**43 declared rows over 7 axes: 177 established, 121 inactive, 0 unchecked, 3 refused — 8 of 43 admitted**,
with every refusal named (`licence_refused×3`, `axis_inactive×35`). The head (`MOD-c2me`'s
`chunk-infrastructure`, carried by U4 on U4's own pasted bar) is admitted; `BigGlobe`, `lithostitched`,
`Create-The_Factory_Must_Grow`, `MFR` and the re-keyed `archipelago` datapack refuse `licence_refused` by the
owner's own name; the absent and
moved documents refuse `source_absent` and `source_digest_mismatch` with both values in the message; an owed
reference is never admitted (37 of 106 entries, all carrying no row).

**Deviations from this task list, each with its reason.**

1. **T001 landed in `src/gameplay.rs`, not `src/intake.rs`.** D5 makes the disposition the **tree's** field,
   and the pass's instruction keeps U6's axes, verbs and answers untouched: adding a U10 vocabulary to the
   intake would have grown U6's surface for a question that is not about a row. The check T001 names
   (the four words, `parse`, `COUNT` pinned to `ALL.len()`) ran, in the tree's own module.
2. **The reference table is this pass's reading, and it is written down in the module header.** Three rules:
   the disposition is the record's own word; a reference's technology is the one the record states for it
   (the harvest's measured passage, else the first §8.1 rule row that cites it, else the map's own class
   row); and `cited_by` is the record's list while the **declaring** entries are what the verdict is read
   from. Where none of the three names a technology, the entry is `open` — a feature is exactly what a
   missing rule row would have named.
3. **`biomemusic`'s disposition is `open`, not `unresolved`.** `spec.md`'s Edge Cases had listed it among
   the `unresolved` examples; `docs/MODS-MAP.md` §4's own Record column says `open` (`unscoped`), and FR-002
   forbids a disposition the map does not give. `spec.md`'s example is corrected, and the record's note is
   carried on the entry.
4. **SC-001's equality — reported unmet by this pass, and settled afterwards (see item 7).** The map's header
   stated **91 URLs** while the map's own §1–§5 rows name the **106** references this tree derives, and the
   header said it could not reconcile the two (*"Nothing in this repository distinguishes those two
   cases, so this map does not pick one"*). The pass therefore printed the derived count **beside** the
   record's own figures — 91 URLs, 24 `open` in §2 (the derived `open` count was also 24 then and is **25**
   after the second correction below, which gave entries to two `mapped` and one `open` reference), 43 `open`
   rows in the prior disposition file — un-averaged,and reported the criterion as unmet on the record's evidence rather than smoothing it. *(Its §2 figure, 24,
   was unsupported by any source and is corrected to nine in item 8.)*
5. **`blast-conversion` is a row no single entry declares.** It is cited by Railcraft, IC2 and
   TerraFirmaCraft (the record's own list) while those entries declare `coking`/`crushing` as their own
   principal: the row is the record's rule with its citations, and the entry is one reference's principal.
   The two relations are separate on purpose, and a test holds that every declaring entry is in its row's
   `cited_by`.
6. **Two corrections after the pass, and neither is smoothed** (2026-10-12, user). The first corrected three
   rows of the record — `tt20` is a **performance mod**, `resquake` is the **Squake movement model** (with
   `Minecraft-CameraOverhaul` on the game's player controller, and no row), and the Archipelago reference is
   **re-keyed** to the Modrinth **datapack** whose own terms declare `CC-BY-NC-ND-4.0` — so that entry is
   `refused` (census `open=24 refused=5`). The second gives entries to the three references that had none:
   `TerraFirmaCraft` and `Calculator` (`mapped`, at `smelting`; each read from its own clone and its own
   licence file — **EUPL-1.2** and **MIT** © 2018 Ollie Lansdell) and `Minecraft-CameraOverhaul` (`open`,
   **GPL-3.0**, read from its own repository: adopted into the game's player controller with `resquake` and
   **not in the clean room**, exactly as `resquake` is). The **smaller reading** is taken and said: each new
   `mapped` entry is filed under its reference's **first** `docs/MODS-MATERIALS.md` §8.1 rule (furnace firing /
   smelting), because the harvest's measured passage states a whole processing chain for a reference while the
   rule library states rules one at a time. Counts: `103` → **106** entries, `mapped` `62` → **`64`**, `open`
   `24` → **`25`**, owed `36 of 103` → **`37 of 106`**; the row count, the census over the rows and the four
   owed rulings are **unchanged**, and the corrections are carried in `docs/MODS-MAP.md` (two dated notes and
   two rows in §2, one in §4) and in the module header. The stdout delta is ten moved lines and three added
   (441 → 444), named in §23.
7. **SC-001 settled from the sources, and the header — not the tree — was the wrong part** (2026-10-12,
   user). The criterion compared the tree's entry count with "`MODS-MAP`'s own reference count", and the two
   figures in play counted **two different documents**. Counted live: the file the map was written from
   (`references/shploingus_dirty/references.md`, sibling `shploingus-game` tree) holds **91 URL lines** — the
   figure the header has always quoted, correct **for that file**; this repo's `docs/references.md` holds
   **97** (that file minus `C2ME`, `CubicChunks` and the replaced `MinecraftFabricArchipelagoMod`, plus nine
   — six of which this map gives rows, five of those added by item 6, and three of which are the two non-mod
   links and `mc-ar-mod`); the prior work the header cites, `design/mod-dispositions.jsonl`, holds **83 rows**
   (**40 `mapped`**, **43 `open`**), verified. The map's **own** rows are a third set: **106** — the **94**
   URLs the copy carries a row for (91 by exact slug + 3 spelling variants: `chunky` → `MOD-chunky`, `tidal`
   → `MOD-tidal`, `stockbridge(refinedstrotage%2Bcreate)` → `stockbridge`) plus the **12** rows the copy does
   not carry (`C2ME`, `CubicChunks`, and the ten Tekkit mods the dated 2026-10-03 ruling added). **So the
   header was corrected, not the entry set**: it now states `**The map's own reference count: 106
   references**` with the per-section split (10 §1 + 42 §2 + 19 §3 + 35 §4) and the whole reconciliation, and
   SC-001 is **measured true** — the census line prints the record's own count against the entry count, and
   a new test (`the_maps_own_reference_count_is_the_trees_entry_count`) reads the document **in place**,
   folds the spelling variants the map prints, and holds the equality **in both directions**, so a dropped
   row or an unnamed entry fails by name. No reference was invented or dropped to make the two numbers meet:
   `mc-ar-mod` — a mod URL in the copy that no section of the map names — is named **owed** in the map's §6,
   and the two non-mod links are named as outside the map's scope. The suite is **226 passed / 0 failed** (one test more, which is that check), and the stdout delta is **five moved lines and none added** (444 → 444),
   named in §23.
8. **Two trailing figures corrected, and the map's split made a checked value** (2026-10-12, user). The map's
   §2 legend stated `(24 of them)` — a figure **no source reproduces**: the §2 marks are **nine**, the tree
   derives **twelve** §2 `open` entries (the nine, plus `optical`, `create-new-age` and `EE 3`, each for the
   reason its own entry states), and the prior disposition file's own figure is the **43 `open`** rows printed
   beside it. The legend now states the nine and names the three, with the correction dated in place; the
   census prints **9 §2 rows carry the record's own `*`** instead of **24 `open` in §2**. §5's heading now
   records that one of its eight slots was re-keyed, and the README line now describes the map's own scope
   (106 references, reconciled in its header) rather than the copy alone. The SC-001 check, which guarded only
   the total, now re-derives the header's **per-section split** (`10 §1 + 42 §2 + 19 §3 + 35 §4`) **in
   place**, and was proved to bite: moving §3's `shuttfup`/`Stfu` row into §4 failed by name
   (`[10, 42, 17, 37]` against the stated `[10, 42, 19, 35]`), and the map was restored byte-exact. The
   stdout delta is **four moved lines and none added** (444 → 444) — `U10 documents`'s map digest, `U10
   census`'s §2 figure, `U10 bar`'s head digest, and `U10 in place`'s declared + tampered digest pair — named
   in §23. No entry, verdict or citation moves; the suite stays **226 passed / 0 failed** (the SC-001 check
   gained assertions, not a function), and the four undecided rulings and `mc-ar-mod` stay owed.
9. **The census's per-section split derived from the map's own rows** (2026-10-12, user). The partition the
   census line prints is no longer a literal: `gameplay::MapRows` is the module's **one** parse of
   `docs/MODS-MAP.md`'s §1–§5 tables (the parse the SC-001 check carried beside it until now), the tree reads
   it **in place** at `of_record` and carries `[10, 42, 19, 35]`, and `Dispositions::render` takes the split as
   an argument — so a row moved across sections moves the printed digits. The digest pins are unchanged and
   stay where they were (the row's declared source, verified by the gate at admission), which is what makes a
   moved row visible in the split instead of hidden behind a digest. The printed bytes **do not move**: 0
   stdout lines added, removed or changed (444 → 444, byte-identical), because the change is where the digits
   come from. Proved live by moving §3's `shuttfup`/`Stfu` row into §4 — the census prints **17 §3 + 37 §4**,
   `the_maps_own_reference_count_is_the_trees_entry_count` fails by name (`[10, 42, 17, 37]` against
   `[10, 42, 19, 35]`), `the_census_prints_the_maps_own_split_rather_than_a_literal` fails by name, and the
   binary exits 1 — then the map was restored byte-exact (sha256 `11873cad…`). Suite **228 passed / 0 failed** (the 226 of item 8 plus the census's print-is-the-argument check and the absent
   map's refusal by name);
   no entry, verdict or citation moved, and the four undecided rulings and `mc-ar-mod` stay owed.
10. **The census's seam declared, the pin's enforcement named, and the four rulings asked** (2026-10-12, user).
    `contracts/u10-seams.md` gains **C6**: the `Dispositions` counts, `Tree::sections()` read in place,
    `Dispositions::render(sections)` taking the split as an argument, and `MapRows` as the module's one parse —
    with two contracts: no literal partition on the print path (a moved row moves the digits, and the pin fails by
    name), and the reader's deliberate non-verification of the digest with the places the pin *is* enforced named
    (the gate at admission, on which the binary's exit status depends, and
    `the_declared_digests_are_the_documents_on_the_disk`; measured by appending one byte — unchanged digits,
    `source_digest_mismatch`, exit 1). The file's header miscounted its seams (*"three"* beside five) and now
    names the six it declares. The four owed rulings and `mc-ar-mod` were put to the user as questions and **no
    ruling was returned**, so all five stand owed unchanged — nothing decided here (FR-014, SC-006) — and the
    contract's owed list now records that they were asked.
