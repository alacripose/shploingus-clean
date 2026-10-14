# Implementation Plan: U10 — The mods-derived gameplay tree

**Branch**: `007-u10-gameplay-tree` | **Date**: 2026-10-12 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/007-u10-gameplay-tree/spec.md`, governed by
`.specify/memory/constitution.md` (P1–P7) and `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §16 (U10, §16.2, §16.3)
and §5's row-10 bar.

**Note**: **this plan was written by a specification-only pass** (*"Change no code in this pass — the
deliverable is the specification with its plan and tasks entries plus the list of sources you used, cited by
path."*), and **the slice it prepares then landed** in the pass recorded here as **Landed** — so the sections
below describe the slice as it was planned and as it was built, and the differences are named in
tasks.md's own record and in `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §23 rather than smoothed away.

## Landed (2026-10-12, thirteenth pass)

The execution order below ran. `src/gameplay.rs` (the disposition vocabulary, the reference and technology
tables, `Entry`, `Technology`, `Tree::of_record/declare/declared/rows/first`, the owed list),
`src/evidence.rs`'s `check_u10`, `src/main.rs` (the bar joins the gate), `src/systems.rs` (U10's real status)
and `tests/u10_gameplay.rs` (six end-to-end tests) are in the tree; no dependency was added, no `crates/*`
file was touched, and no mod source was read. Measured: **106 entries** (mapped 64, open 25, refused 5,
unresolved 12), **43 technology rows** (23 cited by more than one reference, 35 with an owed bar), U6's own
census over the rows reading **177 established / 121 inactive / 0 unchecked / 3 refused — admitted 8 of 43**,
with `licence_refused×3` and `axis_inactive×35` named; `cargo test --offline` 226 passed / 0 failed, clippy
clean, `cargo run --offline` exit 0 at **444 lines** (stdout 278 → 444: two lines changed by the build, three
entry lines added and ten lines — counts and digests — moved by the two corrections named just below).

**Two corrections after the pass (2026-10-12, user), both carried in `docs/MODS-MAP.md` and in the entries.**
The first corrected three rows: `tt20` is a performance mod, `resquake` is the Squake movement model (with
`Minecraft-CameraOverhaul` on the game's player controller, and no row), and the Archipelago reference is
re-keyed to the Modrinth **datapack** whose own terms declare `CC-BY-NC-ND-4.0`, so that entry is `refused`.
The second gave entries to the three references that had none — `TerraFirmaCraft` and `Calculator` (`mapped`
at `smelting`, each read from its own clone and licence file: **EUPL-1.2** and **MIT** © 2018 Ollie Lansdell)
and `Minecraft-CameraOverhaul` (`open`, **GPL-3.0**, read from its own repository) — taking the **smaller
reading** where the harvest's measured passage states a whole processing chain for a reference and the rule
library states rules one at a time. Neither correction adds a row, an axis or a verb, and neither decides any
of the four owed rulings.

**SC-001 settled after that (2026-10-12, user).** The criterion had been reported unmet — the map's header
declared 91 URLs while the tree derived 106 references — and the sources show the two figures count **two
different documents** (the 91-URL file the map was written from, and this repo's 97-URL copy), while the
map's own rows are a third set: **106**, which is exactly the tree's entry count. The header was therefore
the wrong part and is corrected (it now states its own count and the arithmetic), the URL with no row
anywhere (`mc-ar-mod`) is named **owed** rather than invented into an entry, and a new end-to-end test reads
the map in place and holds the equality both ways — and, corrected 2026-10-12, its **per-section split** as
well (`10 §1 + 42 §2 + 19 §3 + 35 §4`), so a row moved across sections fails by name; the map's §2 legend
also dropped its unreproducible "24 of them" for the **nine** marks it carries (the tree derives twelve §2
`open`, the other three named in the legend). The suite is **226 passed / 0 failed** — one test more,
which is that check (it gained assertions, not a function).

**Two things the plan made conditional are now decided by what was built, and named.** The **disposition
vocabulary lives in `src/gameplay.rs`**, not `src/intake.rs` (this plan's Structure Decision already said
*one new module plus the disposition on U6's row*; `tasks.md` T001 named intake.rs and is corrected there).
And the **head row's check is U4's own pasted bar** (`specs/004-u4-mesh-stream-work/quickstart.md`) while the
U3-shaper rows (`flora-placement`, `seasonal-clock`, `biome-triple`, `flocking`, `dense-ore-dials`) take
`docs/SHAPERS.md` §1's paper-per-shaper contract — the carrying systems' own bars, never one invented for the
tree.

## Summary

Give the clean room the **tree**: one entry per reference in `docs/MODS-MAP.md` §1–§5, each carrying the
record's own **disposition** (`mapped`, `open`, `refused`, `unresolved`) and — for a `mapped` one — the
**row U6 already admits**. A row is an *admission*, never a transcription: the tree adds **no** new row
type, **no** new axis and **no** new verb. It adds the one thing the gate does not carry: the **ordered
list**, its **dispositions**, and the **double-cover dedupe** that keeps one row per technology with both
citations. A `refused` reference refuses **by name** (`licence_refused`, already built); an `open` one is
named **owed** and never admitted, because a reference with no feature cannot be a row.

## Technical Context

**Language/Version**: Rust, edition 2021, `rust-version = 1.98` (the clean-room workspace's pinned toolchain)

**Primary Dependencies**: `sha2` only — already present. U10 is a list of declared entries and the digests
that identify the documents behind them; nothing new is needed.

**Storage**: none of its own. The record stays U2's `Chain`; the resident geometry stays U4's; the tree holds
entries and *references* to documents (`path + digest`), never their bytes.

**Testing**: `cargo test --offline` (a unit test walking the tree against `MODS-MAP`'s own entry set,
**plus** a served-transport test that drives a tree row through the real binary as a separate process),
the printed measurements from `src/evidence.rs`, and `cargo clippy --offline --all-targets -- -D warnings`.

**Target Platform**: any (integer parsing, one hash, and a list; no floating point in any identity path)

**Project Type**: library crate with a thin binary (the shape U2–U9 established)

**Performance Goals**: none claimed. What matters is **counts**: entries declared, dispositions counted,
rows admitted, refusals recorded — each printed, never inferred.

**Constraints**: refuse-never-default; one row per technology; a licence verdict per reference and no blanket
grant; no `crates/*` import and no `path` dependency (P7); no mod source re-read (the harvest is the
reading); every module header names its document (P1)

**Scale/Scope**: **one entry per `MODS-MAP` reference (106 references, §1–§5 — the map's own count, whose header reconciles it against the 91-URL file it was written from and this repo's 97-URL copy)**, of which the `mapped` ones are
rows; the four-word disposition; the double-cover dedupe; the served by-reference row it already rides on.
**Not** in scope: the recipe document's row-kind key and version, the duplicate-row refusal's name, a
whole-tree verb, and the per-row family assignment — each named owed below.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Evidence / note |
|---|---|---|
| P1 Fresh code, declared provenance | PASS | every module header will name `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §16 and `docs/MODS-MAP.md`; no `crates/` source is read, and no mod source is read at all (D10) |
| P2 Refuse, never default | PASS | FR-005/FR-006: a refused reference refuses `licence_refused`, a word outside the two verdicts refuses `licence_undeclared`, an absent document `source_absent`, a moved one `source_digest_mismatch`, a missing stage `stage_undeclared`, an unadmitted material `unadmitted_material` — all **by name**, all already built |
| P3 Determinism is a contract | PASS | the tree is ordered and its entry set is derived from the record, so two builds print one tree; a row's answer is U6's pure digest, unchanged |
| P4 Evidence over assertion | PASS | SC-001…SC-007 each name a printed measurement; an `open`/`unresolved` entry is **counted and printed**, never silently absent; the built slice's tasks are ticked only where their check ran (`tasks.md`) |
| P5 One declared interface, two doors | PASS | the row is U6's, admitted through the **one** table (`admit`); no second row type, no eighth axis, no new verb (D4, D5, D9) |
| P6 Smallest sufficient artifact | PASS | the tree is the **tree** — a list plus its dispositions; the four owed parts are named rather than scaffolded |
| P7 Legacy is a witness, never a source | PASS | FR-013; the recipe **format** is cited by `path` (`packages/alpha-shell.shspec`) and the licence verdicts are quoted from `docs/MODS-MAP.md`; nothing is read from `crates/`, and no mod source is read |

**Post-design re-check**: unchanged. Phase 1 added one document set (`specs/007-u10-gameplay-tree/`); it adds
no dependency and no second copy of any owner's state, and it names four parts owed rather than deciding
them.

## Project Structure

### Documentation (this feature)

```text
specs/007-u10-gameplay-tree/
├── spec.md              # /speckit-specify output (this pass's deliverable)
├── plan.md              # this file (/speckit-plan, via setup-plan.sh)
├── research.md          # the decisions, the disagreement taken smaller, and the four refusals
├── data-model.md        # the entities an entry is made of
├── quickstart.md        # how the acceptance bar is checked (measured in the slice's pass)
├── contracts/
│   └── u10-seams.md     # the seams this feature exposes, each stated as the refusal it produces
├── checklists/
│   └── requirements.md  # /speckit-specify's quality checklist
└── tasks.md             # /speckit-tasks shape — **every task unticked**, because none was run
```

### Source Code (repository root)

**Built, with two deviations named in `tasks.md`.** The shape below is the *smallest* one that satisfies
the spec; what landed matches it — `src/gameplay.rs` instead of a `src/intake.rs` vocabulary for the
disposition, and `src/refusal.rs` unchanged (the duplicate-row refusal uses the closest **declared** name and
names the owed word, rather than coining one).

```text
clean-room/
├── Cargo.toml           # unchanged: `sha2` stays the one dependency
├── src/
│   ├── intake.rs        # exists (U6) — the row, the axes, the verdicts. **Extended, not replaced:**
│   │                    #   a disposition beside the row (D5), and the tree's own vocabulary
│   ├── gameplay.rs      # NEW: the tree — the ordered entries, the four-word disposition, the dedupe
│   ├── refusal.rs       # + the names the tree needs that do not exist yet (the duplicate-row refusal is
│   │                    #   **owed** — D7 — so this file gains a name only when the record gives one)
│   ├── evidence.rs      # + the tree's measurement: entries, dispositions counted, rows admitted,
│   │                    #   refusals by name, and the *owed* line
│   ├── lib.rs           # register the tree on the library surface
│   └── systems.rs       # U10's real status, replacing `absent` — and only what was built
└── tests/
    └── u10_gameplay.rs  # NEW: the end-to-end check — the tree against `MODS-MAP`'s own entries, a
                         #   refused reference refused by name, and a tree row served over the real
                         #   binary as a separate process
```

**Structure Decision**: **one** new module (`gameplay.rs`) plus the disposition on U6's row. The tree is one
capability — *an ordered set of declared entries with the record's disposition for each* — and the **row**
belongs to U6 (D4). The recipe **document** format is a format, not a module: it is read by the intake's
existing by-reference seam, so the tree names it and owes its row-kind spelling (D8).

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| — | — | *(no violations)* |

## Execution order for the slice (when it is authorised)

1. **The disposition vocabulary** — the record's four words, taken from `docs/MODS-MAP.md`, with the
   establishment states they map to (D2, D3, D5).
2. **The tree** (`gameplay.rs`) — the ordered entries, one per reference, and the dedupe rule.
3. **The rows** — each `mapped` entry's row built as U6's `Row` and admitted through the one table.
4. **The refusals** — a `refused` reference by name, a word outside the two verdicts, an absent/moved
   document, a missing stage, an unadmitted material.
5. **The evidence** (`src/evidence.rs`) — the counts, the first entry, the refused line, and the *owed*
   line.
6. **The end-to-end check** — the tree against `MODS-MAP`'s own entry set, and one tree row served over the
   real binary as a separate process.
7. **Documents** — `quickstart.md`'s pasted numbers, `tasks.md`'s ticks, `src/systems.rs`, and the governing
   plan's slice-10 row.

## What this slice does **not** land (named, not implied)

Each of these is **owed to the record**, and each is repeated in `spec.md`'s Assumptions, `research.md` and
the binary's own output (SC-006), so a reader meets it four times rather than not at all.

- **The recipe document's row-kind spelling and version.** The one document that exists opens
  `shploingus.recipe/3` and declares `window`/`tab`/`bar`/`filed`/`pin` rows, plus the ticket form's `row`
  (`packages/alpha-shell.shspec`, `packages/BLD-0119-*.shpkg`). A gameplay entry is a **new row kind**, and
  `docs/LEDGER.md` **R-64.1** makes the format *versioned and closed* — so the key and the number are a
  **version decision the record has not made** (D8). No key and no number are invented here.
- **The duplicate-row refusal's name.** `docs/MODS-MATERIALS.md` §8.2 states the double-cover **rule** and
  nothing states the **word** a second row is refused by (D7).
- **A verb that serves the whole tree.** U6 serves **one row per request line**; the tree is a list, and no
  design anywhere declares a verb that returns it (D9). The tree's *rows* do reach a client today, over
  `admit`.
- **The per-row family assignment and the `SUBSTANCES` table shape.** `docs/MODS-MATERIALS.md` §5 ends with
  the assignment still owed (*"per-row family assignment (the ten families) … on the way to the
  `SUBSTANCES` table shape"*), and §8.3's progression model is **a design sketch, falsifiable, to be built
  after the materials Rust**. This spec names the classes and their sources; it does not assign the rows.
- **The other U6 remainder this slice rides on** — the document record and its compression, the structural
  judge with its calibration, the ballot, the crate × axis census over the tree, the permission policy
  matrix, the API document, a pin over a request line — all U6's, all named owed by §16–§22, and none
  re-opened here. *(Corrected 2026-10-12, §29: the **permission policy matrix** and the **census** have
  since landed; the judge, the ballot, the API document, a pin over a request line and the matrix's own
  remainder — the self-describing protocol and the tool-surface narrowing — stand owed.)*
- **The un-re-derived rows** of U6 (§22: the five older rows' answers are not re-established from their
  requests, for want of a read-only re-evaluation seam) — unchanged by this pass and restated so the tree
  does not inherit a silent limit.
**And the census reads that same parse (2026-10-12, user).** The SC-001 check's parse is no longer the
check's own: it is `gameplay::MapRows`, the module's **one** parse of `docs/MODS-MAP.md`'s §1–§5 tables, which
`Tree::of_record` reads **in place** and the census line takes its per-section split from (`render` takes the
four digits as an argument, so nothing on the line restates them). The printed bytes are unchanged — 0 stdout
lines moved (444 → 444) — and the derivation is proved live by moving a §3 row into §4: the line prints
`17 §3 + 37 §4`, the end-to-end check fails by name (`[10, 42, 17, 37]`), and the map was restored
byte-exact. The suite is **228 passed / 0 failed** — the 226 above plus the census's print-is-the-argument check
and the absent map's refusal by name.
