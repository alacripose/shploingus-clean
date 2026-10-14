# Phase 0 — Research: U10 the mods-derived gameplay tree

**Feature**: `007-u10-gameplay-tree` | **Date**: 2026-10-12 | **Spec**: [spec.md](./spec.md) | **Plan**:
[plan.md](./plan.md)

**What this file is.** One entry per decision, each naming the **path** it was decided from, and — where two
references disagree — the **smaller** reading taken and **why**. A decision this pass *refuses* to make is
here too, under a `refused` heading, because a refusal nobody recorded is indistinguishable from an
oversight. `/speckit-specify`'s quality checklist reads this file for its two open notes (see
[checklists/requirements.md](./checklists/requirements.md)).

**Nothing in this file is implemented.** No code was written in this pass; the deliverable is this
specification (the user's instruction: *"Change no code in this pass — the deliverable is the specification
with its plan and tasks entries plus the list of sources you used, cited by path"*).

---

## D1 — What the tree is, in one sentence

**Decision.** The tree is an **ordered list of entries, one per reference in `docs/MODS-MAP.md` §1–§5**,
each carrying the record's **disposition** and — for a `mapped` one — the row U6 already admits.

**Decided from.** `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §16's U10 entry: *"the ordered set of playable
systems the mods references imply … as **one declared tree of rows**, each owed by a document, a licence
verdict and an acceptance check"*, and its interface: *"a **tree of declared rows** — feature, source
document (`path + digest`), the licence verdict it relies on, the system that carries it (U3–U8), and its
own acceptance check — plus a refusal for a row whose source is refused or whose verdict is absent."* The
plan's §16.2 row 10 says its disposition map **already exists** (`MODS-MAP`).

**Rejected.** *A tree of mods* (a port list). `MODS-MAP`'s ruling and `docs/MODS-MATERIALS.md` §0's filter
are both explicit that the mods are read for **ideas**, and `docs/LEDGER.md` C-127 refuses minecraft
progression. A port list would be a licence decision nobody made.

---

## D2 — Three dispositions, not six port verdicts

**The disagreement.** Two vocabularies exist for "what happens to this reference":

- `docs/MODS-MAP.md` §1's legend — **`mapped`** (the record maps it to a feature), **`open`** (*"the record
  explicitly left it undecided"*), **`refused`** (*"a licence fact forbids deriving from it"*), plus §3/§4's
  **`Unresolved`** (*"recorded, not guessed"*). **Four words, in this repository, about these references.**
- `shploingus-game/docs/plan/01-reference-inventory.md` §1 — six **port verdicts** (`PORT`, `ADAPT`,
  `SUPERSEDE`, `EVIDENCE`, `REFUSE`, `ABSENT`). **Six words, in another repository, about another tree.**

**The smaller reading taken: the record's own four.** Reasons, all checkable: (i) `MODS-MAP` is the map the
governing plan names as U10's evidence (*"its disposition map already exists (`MODS-MAP`)"*, §16.2 row 10),
while L1 is `shploingus-game`'s audit of **its own** reference trees and its rows are about *code* verdicts
(*"what we do with a piece of code"*), not about the game features this tree declares; (ii) the tree's unit
is a **reference**, and `MODS-MAP` is one row per reference while L1 is one row per **tree**; (iii) taking
six would put two words (`REFUSE`, `ABSENT`) beside `refused` with no reference deciding which wins.

**Recorded, not erased.** The six port verdicts are cited (spec.md's Sources table) so the choice is
auditable, and a later widening to a per-row *port* verdict is named **owed** rather than pre-empted.

---

## D3 — An `open` reference is not a row, and `Acceptance::Owed` is the shape of "owed"

**Decision.** An entry whose disposition is `open` or `unresolved` carries **no row**; it is named **owed**.
A row whose own acceptance check does not exist yet declares the check **owed** (`Acceptance::Owed`, already
built in `clean-room/src/intake.rs`).

**Decided from.** `MODS-MAP` §2: *"An `open` mod is not a rejection: it means the record assigned no
feature"*, and §6: *"It does not resolve the **43 open** rows by fiat … every one of them is named in §2–§4
so that its absence from the build is visible rather than silent."* `clean-room/src/intake.rs`'s
`Acceptance::{Provided, Owed}` and `Requirement::{Named, Owed}` are the two existing instances of *"declared
and owed"*, and `Establishment::is_established` (*"only `Active` is [a pass]"*) is the rule that makes owed
a **report**, never a pass.

**Why this is not an invention.** A row's `feature` argument is **required** by the table
(`clean-room/src/bridge/table.rs`: `arg("feature", ArgKind::Name, true)`), so a reference with no feature
**cannot** be a row — the built surface already decides this, and the tree only says so.`MODS-MAP` §2's count and §6's (*"43 open"*) differ; the tree takes each **entry's own row** from §1–§5 rather
than either count, and prints the count it derived — the disagreement is recorded, not resolved. *(Corrected
2026-10-12: §2's legend stated `24 of them`, a figure no source reproduces; the marks are nine, the tree
derives twelve §2 `open` entries (the nine plus `optical`, `create-new-age` and `EE 3`), and the legend now
states exactly that.)*

---

## D4 — A row **is** U6's row: nothing new is added to it

**Decision.** The tree's row is `clean-room/src/intake.rs`'s `Row` — feature, `Source` (`path + digest`),
`Licence`, `Carrier`, `Acceptance`, `Claim`, `Requirements` — admitted through the **existing** `admit` row
of `bridge::TOOLS` (ten arguments: `path`, `digest`, `feature`, `licence`, `claim`, `carrier`, `check`,
`check_digest`, and the optional `requires_stages`/`requires_materials`).

**Decided from.** The plan §16 row 10's own hard prerequisite: *"deriving from a reference is an
*admission*, never a transcription, so `admit`/`inspect → Established` and the by-reference verbs must land
first. This system is *the* thing that gate blocks."* The five declarations the plan names for a row map
one-for-one onto the `Row` fields already built; the seventh axis (`conformance`) already judges a row's
requirements against U3's roster and U2's registry (`docs/CLEAN-ROOM-PLAN-2026-10-11.md` §21).

**Rejected.** *A second row type for gameplay* (`GameplayRow`). It would be a second decision matrix for the
one thing the gate already decides, which `docs/LEDGER.md` §55.3's one-pass-two-readers ruling forbids and
the constitution's P5 (*"a second decision matrix … is refused by name"*) refuses.

---

## D5 — The disposition is the **tree's** field, not a new axis

**Decision.** The disposition travels on the **entry**, beside the row — not as an eighth axis of the
verdict.

**Decided from.** `clean-room/src/intake.rs`'s own comment on `Axis::ALL`: *"a new column is **appended**,
because a verdict's axes are positional: an axis inserted in the middle would silently relabel a report that
already exists."* An axis is a question about **a row**, and a disposition is a question about **a
reference** — an `open` reference has no row for an axis to judge. The plan's interface (§16 row 10) lists
**five** things a row declares and no disposition among them.

**Rejected.** *An eighth axis `Disposition`.* It would either be `unchecked` for every reference that is not
a row (an axis that cannot fail — the exact defect `docs/LEDGER.md` C-68 names), or it would force every
`open` reference to become a row it cannot be (D3).

---

## D6 — The first entry is §1 (world shape and streaming), and its check is the shapers' own

**Decision.** The tree is ordered, and its first entry is a `MODS-MAP` §1 row carried by U4 (or U3), whose
acceptance check is the one the carrying system already declares.

**Decided from.** `MODS-MAP` §1's heading: *"World shape and streaming — the load-bearing class. This is the
class the first target ('a fully playable basic world') directly needs."* The plan's §5 row-10 bar: *"the
tree's first row is playable."* `docs/DESIGN-SPEC.md` ruling 3: the first target is *"a fully playable basic
world with all terrain shapers"*. `docs/SHAPERS.md` §1 is the paper-per-shaper contract, and its §2 is the
determinism contract those shapers already obey.

**Rejected.** *Ordering by `MODS-MAP`'s section numbering alone*, which would put §2's industrial slots
after §1 — true today, but not a rule; the rule taken is the plan's bar (first row playable).

---

## D7 — One row per technology: the double-cover rule

**Decision.** Where the record cites an older mod **and** its Create-addon descendant, that is **one row**
with **two citations**, never two rows.

**Decided from.** `docs/MODS-MATERIALS.md` §8.2: *"Double-cover is real and now visible per row … Each is
**one rule** with multiple citations — the roster must never grow a second row for a descendant."* §8.1's
rule library is already deduped that way (*"rows **dedupe to one per technology**, citing every swept mod
that declares it (the old mod *and* its Create-addon descendant — the double-cover rule)"*).

**Refused: the refusal's name.** No reference names the refusal a duplicate row produces. `MODS-MATERIALS`
states the *rule* and nothing states the *word*. Named owed in `plan.md`; **not invented here.**

---

## D8 — The recipe document is the row's declared form, and its new row kind is owed

**Decision.** A tree entry is declared in the existing **versioned, closed recipe document** form, consumed
**in place** by `path + digest` — the `Source` the `admit` row already carries. The document's real shape is
`packages/alpha-shell.shspec`: the version tag `shploingus.recipe/3`, a `package <name> <version>` line, then
rows whose fields are indented beneath them.

**Decided from.** `packages/alpha-shell.shspec` (read: the tag, the `package` line, and the
`window`/`tab`/`bar`/`filed`/`pin` rows); `packages/BLD-0119-*.shpkg` (read: magic `SHPKG002`, then a
`shploingus.recipe/3` body with `package from-ticket v0` and a `row BLD-0119 <prose>` line — the compiled
form of a one-row recipe); `docs/LEDGER.md` **R-64.1**: *"the recipe format is defined by this repository,
versioned, and closed — documents pass through it."* `docs/LEDGER.md` C-64 is the defect that put a
document out of step with the format, which is why the format is *closed*.

**Refused: the new row kind's key and version.** A gameplay-tree entry is a **new row kind** (`rule` /
`entry` / …) and the existing `/3` declares only `window`, `tab`, `bar`, `filed`, `pin` and the ticket
`row`. Since the format is closed, a new kind is a **version question** — and **nothing in the references
decides the key or the number**. Named owed in three places (`plan.md`, `research.md` here, `spec.md`'s
Assumptions), never chosen.

---

## D9 — The tree's rows reach a client over the wire form **now served**; the whole-tree verb is owed

**Decision.** A tree row crosses exactly as `admit` does: one declared request line, served by `serve`, over
the line protocol of `clean-room/specs/005-u6-record-api/contracts/u6-transport.md`. The tree itself (the
ordered list with its dispositions) is **not** served by any verb, and no verb is invented.

**Decided from.** §19's landing (`admit` is a row of `bridge::TOOLS`, served over the transport, its
document arriving as `path + digest`); `contracts/u6-transport.md`'s *"a row crosses by reference"* clause;
and the plan's own owed list, which carries *"the recipe/content format and the API document"* as **owed**
(§16's list, §19's, §20's, §21's, §22's) — i.e. the record has already said the document form is owed, and
this spec must not pretend to have decided it. *(Noted 2026-10-12, §28 of the governing plan: the
document form has since landed, so it is no longer among the things owed — but this spec's own blockers
stand unchanged: the content gate's remainder, the recipe/content format and the API document are still
owed, which is why U10's first slice (§23) names its four owed parts rather than completing the tree.)*

**Rejected.** *A new `tree` verb in this pass.* U6's table is **closed and bijective** and a verb is a
contract change; the plan does not declare one, and inventing it would be choosing where the record names
owed.

---

## D10 — The harvest is the reading; the tree does not re-read a mod

**Decision.** A row's `source` points at the **harvest artifact** (`docs/MODS-MATERIALS.md` §2/§8 and the
`_harvest/*` files it names under `F:/shploingus-mod-sources/`), cited by `path + digest`. Building the tree
reads **no mod source**.

**Decided from.** `docs/MODS-MATERIALS.md` §7: the sweep reads **complete** — *"A complete, B resolved, C
accounted — residual items are named, not omissions. Per C-129 the Rust wait ends here."* — and every clone
is HEAD-pinned (`_harvest/clone-hashes.txt`). §4's avenue ruling authorises **reading** for ideas while
forbidding **derivation**; re-reading to build a row adds nothing to a completed reading and would put the
tree's provenance in a second, unversioned place (the defect `shploingus-game/docs/plan/01-reference-
inventory.md` §4 D-06 names: *"An unversioned input to an authoritative output is a broken provenance
chain"*).

**Noted.** `intake/roots.tsv` declares the mod trees as the root `mod-sources` of kind **`corpus`**
(`F:/shploingus-mod-sources`), and says what a root is *not*: *"an import list. Nothing here is consumed by
being named."* That is the same rule, already in this repository, and the tree obeys it.

---

## D11 — The tree's acceptance bar is the plan's, verbatim

**Decision.** The bar is `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §5 row 10's own words: ***"every row names its
document, its licence verdict and the system that carries it; a refused reference is refused by name; the
tree's first row is playable"***.

**Decided from.** That line. No prose is written for it here, because a bar is quoted or it is a paraphrase.

---

## The sources, by path

Listed in full in `spec.md`'s last section. The reading behind them: `docs/` (this repository's record),
`packages/`/`intake/`/`conformance/` (this repository's own document formats and doctrine),
`clean-room/specs/005-u6-record-api/` and `clean-room/src/` (the built surface), `.specify/` (constitution
and templates), and `shploingus-game/docs/plan/01-reference-inventory.md` (the disagreeing taxonomy, cited
so the smaller reading taken is auditable). The mod sources at `F:/shploingus-mod-sources/` are cited
**through the harvest**, per D10.
