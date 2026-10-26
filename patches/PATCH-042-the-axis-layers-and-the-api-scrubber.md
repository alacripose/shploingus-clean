---
patch_id: PATCH-042
title: The axis layers and the API scrubber — one crate per layer, the prose measure lifted out of bloat, and the document the ApiViewer window was declared for
status: approved
operation: plan
target: shploingus/axis + voxel-client/shell + voxel-package/bloat
base_ref: shploingus-clean@main
base_digest: sha256:c3f884806091f57b141ee3c15c459de6184b2307fcc472e2130eeb0012b56062
dependencies: [PATCH-017, PATCH-022, PATCH-041]
source: user-directed-conversation
created_at: 2026-10-21
approval: "user-approved 2026-10-21 — verification, then reconstruction of the game's features and files"
live_mutation: false
---

# Change

**This record is the deliverable.** The user asked for *"the patch file for everything outlined here"*, so the
applied change is the plan itself: no crate, module or line is written by this record, and nothing under `crates/`
moves until the plan is approved (VOWS: plan first, one deliberate pass, approval before the first edit).

The direction, in the user's own words, is the whole of this patch's authority:

> *"Ask me about the layers we're going to expand with a new axis crate system: Create the api scrubber from
> bloat.rs axis module an an updated axis crate folder for seperating out the individual layers for verification
> with upstream changes from f=planet shploingus for viewing documents and api prose in the api window"*

and the seven rulings given when asked, which this plan implements rather than re-opens (the user then added two
layers and the civilians' store, and those additions are §The security layer and §The MCP layer):

1. **One crate per layer**, not one module folder and not one file per ladder.
2. **More axes**, including anything upstream declares and **extra calibration from the prose-logic work in
   `agent_god_files/`**, used as more axis gates.
3. The scrubber reads **all four sources**: public items + rustdoc, the rendered `apidoc` document, `docs/*.md`,
   and chain prose payloads.
4. Its verdict is not a wall of counts: **the purpose is to fix the many missing pieces**, so every gap is named
   by path and line and the report is a work queue rather than a score.
5. The API window is **computed at mount and drawn in the pane** (no generated file on disk, so no invented
   record beside the chain).
6. `F:\planet-shploingus` is a **reference only** — read, pinned by digest, re-implemented here; no line copied.
7. This pass delivers the patch file, not the code — **it is approved**, and the slices below are
   applied one commit at a time in the order §The layers states, so a build never exists in which a
   crate is half-entered in the census.

# The purpose this serves, in the user's words

Approval was given with the purpose attached, and the purpose is the acceptance criterion this record
has to meet:

> *"yes I approve the intent of this is to verify everything in the game and to allow calibration of
> civilians laya + von to produce human readable code packages in an updated format from god agent
> files planet shploingus and shploingus clean that allows us to securely reconstruct the games
> features and files through the new axis system + security"*

So the layers are not the goal; they are the machinery for a **code package**: a readable artifact
that says what a feature is, cites where in the three sources it came from, and can be applied to a
clean base to reconstruct that feature — with the axis system verifying it and the security layer
pinning it. Five requirements, and each is a gate rather than a style note:

1. **Readable.** The package is plain text, one statement per line, no generated binary blob and no
   opaque identifier a reader must decode. A document that cannot be read by a person is not a
   package; it is a build product with a nicer extension.
2. **Cited.** Every line resolves to a claim, and every claim quotes a `file:line` in
   `agent_god_files/`, `F:\planet-shploingus` or this tree. `voxel-axis-view` refuses a line that
   cannot cite one (A9), so a package cannot contain a sentence no source produced.
3. **Laddered.** Every claim carries a `kind_basis` and a computed `support_level`; nothing asserts
   its own strength (A7/A8), and a token is never reduced to a number (A4). That is what lets
   civilians, laya and von **calibrate** a package — the reading is comparable across sources because
   the vocabulary is one vocabulary.
4. **Pinned.** The package's bytes are hashed and signed by the security layer, and reconstruction is
   refused when the pin is stale (S5). A package that reconstructs from unpinned bytes is a package
   that reconstructs a different feature next week.
5. **Reconstructing.** Applying a package to a clean base reproduces exactly the files it names, and
   the reproduction is diffed against the source — byte for byte, refused on any drift. The three
   sources are read-only references (`P7 v2.0.0`'s rule), so reconstruction writes **this** tree and
   cites theirs.

Those five are why the API window, the scrubber, the ladder set, the security cells and the census's
`Exercised` axis all matter: a package is only reconstructible if the thing it names is *exercised*
by a running system, and the window is where a person reads it.

## The calibration matrix: the purpose restated one level down

Stated again on 2026-10-21, and it narrows the job: the point is a **calibration matrix** held by the
civilians' axis database, which hands an agent or a person **human-readable code in the recipe format**
that can be used to fix the game's own code — built from the **patches and tickets**, not invented at
the keyboard:

> *"create a calibration matrix through civilians axis db that will provide agents or people human
> readable code in the recipe format that can be used to fix the game's code, from the patches and
> tickets which should allow for quickly fixing the many hundreds of warning that happen when
> compiling the shploingus bin, allowing for recursive self improvement when the games ticking laya
> and von compilers systems are running. This will require the full outlined api view in the shell
> window because that is how the game is verified through the 5 stars and the bronze-platinum
> tiers"*

**The matrix's shape.** One row per *(segment, axis)* pair, where a segment is a system the game can
be split into and every segment is in the table exactly as the census's `Crate` variants are:
`shell`, `ui`, `game`, `world`, `pattern`, `axis`, `tooling`. A row carries the ladder reading
(token + basis + witness, from `voxel-axis-ladder`), the **star** it earns (★1…★5, `voxel-axis-coverage`'s
grades), its **tier** (bronze · silver · gold · diamond · platinum), the ticket it was read from, and
the patch that states the rule. A cell with no witness is `unchecked`, never a pass (S5), and a row
whose segment is not in the table is refused by name — the matrix's totality is a compile-time pin,
sixteen → seventeen's lesson applied to segments.

**The DB is a projection, not a second store.** The chain is canonical (`voxel-model::chain`, H-8.1)
and the security layer refuses a second record on disk (S3). So the *axis db* is built at mount from
the chain plus the patch files plus the census, cached with the API report, and dropped when
`set_workspace` moves the root. It is a database in the sense that it is addressed by key and
answered in rows; it is not a file beside the chain, and it holds no fact the chain does not.

**The output is a recipe line, and that is what makes warnings tractable.** `cargo check` on these
bytes reports **954 warnings from `voxel-model` alone** (observed on 2026-10-21 while checking
`voxel-axis-being`; the figure is a count, not an estimate). Each warning becomes a **ticket** — the
build gate already files them — each ticket names a `file:line`, a rule and an axis, and the matrix
renders one recipe line per ticket:

```
# segment=world axis=E_documentation star=3 tier=silver ticket=T-0954
fix  crates/voxel-model/src/world/book.rs:212   add doc comment for `pub fn page_for`
     because  missing-docs (voxel-model: 1 of 954)  and  C-9 §9.2 says a public item states its contract
     check    cargo check -p voxel-model  ->  warnings 954 -> 953
     witness  conformance/missing-docs.txt  (re-taken by the recipe in census.rs's header)
```

Six fields, all readable, none of them an identifier a reader must decode: **what to type**, **why**,
**how it is checked**, **what moved**, and **where the witness lives**. That is the "updated format" — a
recipe a person applies by hand and an agent applies through `voxel-package::recipe`, with the same
text and the same result. Applying it is what raises a star; a star that cannot name the recipe that
raised it is not a reading.

**Why the shell window is mandatory, not a report.** The verification vocabulary is the stars and
the bronze–platinum tiers, so the matrix can only be verified where a person can see it: the full
outlined view (`Asked for · Declares · Gates · Surface · Trust · Gaps`) plus the **star row and tier**
per segment, drawn in `ApiViewer`, first line the title, last line the measured tail. A matrix that
exists only in a test's assertion is not verified by a person's own rule.

**Recursive self-improvement, stated as the loop rather than the ambition.** Laya and von tick; on
each tick the civilians step reads the matrix, picks the ticket with the highest gap, renders the
recipe, and (under a ticket-scoped grant, never a standing one) applies it; the next `cargo check`
is the receipt, the star moves, and the row is rewritten. No step invents a ticket, no step applies
without a grant, and a refused recipe leaves the tree untouched (R2's rule applied to edits).

| # | matrix gate |
|---|---|
| C1 | **totality** — every segment in the table has a row; a segment with none is refused by name, and the pin is `SEGMENT_COUNT` |
| C2 | **no invented ticket** — every recipe line resolves to a ticket in the chain and a `file:line` that exists; a line whose path or line is absent is refused before it is drawn |
| C3 | **the recipe is the change** — a recipe applied to a clean base produces exactly the named edit, diffed byte for byte; drift is refused |
| C4 | **the receipt is the compiler** — a star rises only when the check's warning count falls by the number the recipe claimed, and the count is printed |
| C5 | **one grant per apply** — a step that applies without a ticket-scoped grant is refused and the receipt names `matrix_permits`; a read needs `observe` |
| C6 | **the window draws it** — `ApiViewer`'s panel carries the star row and the tier for every segment, and its last line is the measured tail |

# The finding, in numbers, before anything is proposed

**The API window is declared and has no document.** `recipe::Content::ApiViewer` is a declared window kind
(`recipe.rs:161`, `bucket(): RecordRule`). `voxel-package/src/apidoc.rs` (437 lines) renders the human-readable
document for it, `TITLE = "API view — declared arrangement"`, and calibrates it against the same judge that
convicts a contributor. **But no production caller exists**: `grep -rn "apidoc::" crates` names only
`voxel-package/tests/first_ticket.rs`. In the shell, `ContentViews::panel_for` has no arm for `ApiViewer`; it falls
to `_ => declared_content_panel(spec)`, i.e. the window draws a panel of *its own declaration* — a placeholder, as
the user said. So the producer and the consumer both exist, and nothing joins them.

**The axis vocabulary has four homes, and the newest is the least complete.**

| where | lines | axes | state |
|---|---|---|---|
| `crates/voxel-model/src/axis.rs` | 1 455 | 7 | all seven declared; `Requirement`, `ProceduralGate`, rules 51–55 |
| `crates/voxel-package/src/census.rs` (`Axis`) | 2 582 (module) | 6 | **crate-level reading axes** (Named, Moduled, Tested, Refusing, Claimed, Documented), a different question from the ladders |
| `clean-room/src/axis.rs` | 811 | 7 named | **3 declared, 4 owed** — its own test asserts `declared == 3` |
| `F:\planet-shploingus\crates\planet-axis` | 1 550 (`axis.rs` 1 308 + `refusal.rs` 229 + `lib.rs` 13) | 7 | all seven declared; adds `SupportLevel` (6), `EvidenceOrigin` (4 word-rungs), `UnitStatus`, `BARE_TOKEN_AXES` |

The clean room re-derived an axis module that owes four ladders its own parent **already declares** — the exact
shape PATCH-041's port rule was written for.

## The utilisation gap, which is the large issue

The user's conclusion, and this plan agrees with it: *"the fact that civilians aren't utilized in every crate means
that there is a large issue at hand with the project's ability to verify itself and produce reliable code."* The
numbers say the same thing as the sentence:

* **The civilian systems have no caller outside their own module.** `grep -rn "voxel_model::civilian" crates` → **0**.
  The 5 471 ported lines are compiled, tested by their own inline tests, and **stepped by nothing**: no crate runs the
  agency, the governor, the selection planner or the task system against the governed book.
* **`voxel-package::civilians` reads witnesses, not work.** It answers *"does the file carry a token"* and *"does a
  cited digest still match"* — real readings, and neither of them is *"did a civilian do anything"*.
* **Two crates are read by nobody at all** — `voxel-audio` and `voxel-instrument`, recorded in `docs/TODO.md` row 17's
audit. The record knows this in prose.
* **The gate has no axis for it.** `census::Axis` is Named · Moduled · Tested · Refusing · Claimed · Documented — six
  cells a crate can fill while **nothing ever calls it**. `voxel-audio` can read `active` on all six and still be a
  crate no running system exercises. A check that cannot fire on *"unused"* is exactly the class of defect the
  record keeps convicting, and it is why the civilians' absence is invisible today.

So this patch adds a **seventh census axis, `Exercised`** (inside `census`, which stays in `voxel-package`): *does a
call site exist outside this crate's own sources and tests?* It is measured by the crate's own name appearing as a
path segment in another crate's non-`#[cfg(test)]` code or in the `shploingus` binary, with its limit stated in the
module header — **a re-export is not a call**, and a macro-generated call site is not counted. `voxel-audio` and
`voxel-instrument` are expected to come back `inactive` with **0** callers, which turns TODO row 17 from a sentence
into a cell; the civilians' **0** turns into the first `refused` row the new axis has, and the loop in §The
civilians layer is what moves it. Upstream's genuine deltas over this tree are narrow and are listed in
§The axis set: `EvidenceOrigin`'s four word-rungs where this tree has three letter-rungs, and the
`refuse_maximum_certification` naming. Everything else upstream has, this tree already has — sometimes more
(`Requirement`, `ProceduralGate` as an axis rather than a struct field).

**The prose measure is a finished instrument living in the wrong crate.** `voxel-package/src/bloat.rs` (1 034
lines) owns `Prose` — `measured` / `why_unmeasured`, `comment_bytes` split into **`doc_bytes` (rustdoc)** and
`plain_bytes` (commentary), `plain_percent` as the ratchet's subject — plus `is_comment`, `is_rustdoc`,
`comment_lines` and `measure_prose`. Its design is exactly the scrubber's measurement layer, but it lives in the
admissions gate, and **the API window cannot import the gate without importing the admissions door**. That is the
"api scrubber from bloat.rs" the user named: the measure belongs to a layer the window may depend on.

**What there is to read, measured on the base bytes:**

| source | size |
|---|---|
| `crates/**/*.rs` | 259 files, 173 902 lines; **4 228** lines that declare a `pub` item (a grep, so a floor) |
| `conformance/missing-docs.txt` | **2 285** undocumented public items across **17** crates (voxel-model 907, voxel-shapers 464, voxel-package 340, voxel-client 190) |
| `docs/*.md` | 30 files, 33 073 lines, 2 325 331 bytes |
| `chains/workspace.chain` | 1 532 016 bytes |
| the whole walk (`.rs` + `.toml` + `docs/*.md` + `chains/*.chain`) | **306 files, 12 149 969 bytes**, read in **105.6 ms** on this machine by a cold Python walk — the proxy for the mount budget |

**What the prose-logic work contributes, and it is not a style import.** `agent_god_files/prose_logic` (2 604 lines,
four layers: `artifact.rs`, `unit.rs`, `claim.rs`, `view.rs`) carries four **gates** this tree does not have:
a tiling audit (*"ranges must partition `[0, artifact_chars)` exactly, or the run fails rather than loses text"*);
a claim whose **support is computed, never asserted** (*"there is no field for a candidate's opinion of its own
strength"*); a **kind that records its basis** (`heuristic:<signals>` or `heuristic:default`, *"the quote is always
right, the kind is sometimes wrong"*); and **degradation that keeps content** (*"an unrecognized document degrades
to flat paragraphs, all preserved, all FACT — never to lost content"*). Those four are the scrubber's skeleton, and
its `AtomKind` (8 labels) and the census's `Establishment` (5 states) are the two ladders the axis set is missing.

# The layers: ten crates in one folder

`crates/axis/` is the folder the user asked for; every member inside it is a package of its own, so a layer can be
built, tested and refused without the layers above or below it existing. Every crate root enables
`#![warn(missing_docs)]` like the other sixteen, and every one adds a line to `conformance/missing-docs.txt`.

| # | member | package | layer | depends on | owns | refuses |
|---|---|---|---|---|---|---|
| 1 | `crates/axis/ladder` | `voxel-axis-ladder` | the axis vocabulary | `voxel-model` | 9 ladders, `AxisKind`, `Ladder`, tokens, aliases, `refuse_interchange`, `refuse_reduction`, `refuse_maximum_certification`, `refuse_bare`, `bare_token_axes`, `ambiguous_tokens` | a cross-ladder comparison; any percentage or score of a ladder; a maximum over a certification; a bare token (AMBIGUOUS / UNQUALIFIED) |
| 2 | `crates/axis/coverage` | `voxel-axis-coverage` | coverage and the partition audit | `ladder`, `voxel-model` | `CoverageRow`, `COVERAGE`, `COVERAGE_IS_TOTAL`, `check_coverage`; the tiling audit `tile(chars_total, units)`; the owed-debt table (a ladder with no live producer names its document) | an uncovered axis; a fabricated row (values that are not the axis's own); two rows for one axis; a **gap**, an **overlap**, an **empty unit**, a unit **past the end** — each by position |
| 3 | `crates/axis/artifact` | `voxel-axis-artifact` | the sources, measured | `ladder`, `voxel-model` | **`Prose` moved here from `bloat.rs`** with `measure_prose`, `comment_lines`, `is_comment`, `is_rustdoc`; `Artifact { id, kind, bytes, text }` where the id is `artifact_<sha40>` over the bytes; `ArtifactKind` = PublicItem · ApiDocument · DocPage · ChainEvent; `Sources { measured, why_unmeasured, per_source }` | an input it could not read (`unmeasured`, with the reason, never a zero); a second parse of bytes already read |
| 4 | `crates/axis/unit` | `voxel-axis-unit` | the addressable units | `artifact`, `coverage` | `Unit { id, artifact, index, start, end, unit_ref, text }`; the tiling of each artifact into units (a public item's doc block, a document paragraph, a chain event's detail) | leaving one character of a source unclaimed; tiling a text twice |
| 5 | `crates/axis/claim` | `voxel-axis-claim` | the claims | `unit`, `ladder` | `Claim { id, unit, kind, kind_basis, quote, support, origin }`; `derive`, `verify`; `claim_<sha40>` ids; `AtomKind` as the `prose_kind` ladder; `SupportLevel` reused as the support ladder | a quote that diverges from its unit's text, by even one character; a claim with no `kind_basis`; a support level that was not computed by `derive` |
| 6 | `crates/axis/view` | `voxel-axis-view` | the document | `claim` | `Document`, `Section`, `Line { text, kind, claim }`; the three sections (Asked for · Declares · Gates); the index; deterministic rendering | a line that resolves to no claim; a claim the document carries with no line (so a dropped paragraph cannot render); a section with no claims (it prints `none`, which is a claim about the report) |
| 7 | `crates/axis/verify` | `voxel-axis-verify` | the walk and the report | `view`, `artifact`, `security`, `mcp`, `voxel-package` | `scrub(root) -> Report`; the four readers; the `apidoc` handoff; the chain handoff through `bloat::measure_chain_file`; the gate lines; the upstream pin table | any gate below refusing; a located-gap count that claims completeness it does not have |
| 8 | `crates/axis/security` | `voxel-axis-security` | the trust boundary | `ladder`, `artifact`, `voxel-package` | one `SecurityReading` per check, each cell on the `establishment` ladder: containment, capability, grant, filing, pin, secret, admission | a path that escapes the root; a symlink followed; a second `.chain` file (a second record); an invented record beside the chain (`bloat::invented_stores`); a write without a grant; a read without `observe`; a pin that no longer matches; a private key, a `.env` file or a connection string under `crates/`, `packages/` or `docs/` |
| 9 | `crates/axis/civilians` | `voxel-axis-civilians` | the civilian systems, one crate | `ladder`, `voxel-model` | the ported ala-cities civilian process moved whole out of `voxel-model/src/civilian/` (11 files, 5 471 lines), plus the ladder reader moved out of `voxel-package/src/civilians.rs` | a task whose material was not planned before the work; a task completed by declaration rather than by its verb's check; a selection with no need, no budget or no estimate; a ladder reading with no witness |
| 10 | `crates/axis/mcp` | `voxel-axis-mcp` | the surface agents speak | `ladder`, `security`, `voxel-package`, `serde_json` | the **declared** surface (`bridge::SERVED`, moved out of `voxel-mcp`): per row its `bridge::TOOLS` entry, its `Action`, its `Capability` and `policy::permission_of`'s class; the `Session` (capabilities + `Grant`) and the invariant that every call is `bridge::call`; the client that speaks the server's own stdio framing; the laya / von / civilian handoff named per tool | an unclassified served tool; a served tool with no `TOOLS` row; a tool advertised at a permission the chokepoint does not enforce; a call that bypasses `bridge::call`'s three predicates; a write presented with no grant; a store URL in prose |

**Dependency order is a line, not a graph**: 1 ← 2 ← 3 ← 4 ← 5 ← 6, then 8 ← 3 (the trust boundary reads what
was read), 9 ← 1 (the civilian systems read the ladders), 10 ← 8 (a call is a gated act), and 7 ← 6, 8, 9, 10 (the
report composes every layer). `voxel-model` sits under all of them; **9 is the one layer that also uses the
model's `world` vocabulary** (the governed book, `DeltaStore`, `apply_transition`), because the civilians write
through it and this patch does not move the world to move the civilians. **`voxel-mcp` is deliberately not a dependency of 9**: that crate carries `voxel-von` and the
ONNX runtime, so a window that linked it would pay for a model to draw a list. The client speaks the wire shape;
the **server** stays the one crate that carries the classifier, and 9's *dev*-dependencies may name it so the tests
drive the real `serve` over cursors rather than a stand-in.

**What moves, and what is deleted rather than kept beside:**

* `voxel-model/src/axis.rs` (1 455 lines) **moves** to `crates/axis/ladder/src/` and is deleted from `voxel-model`.
  Its six consumers — `voxel-client/src/shell/notifications.rs`, `voxel-package/src/civilians.rs`,
  `voxel-package/src/decision.rs`, `voxel-package/tests/retire_the_broken_rows.rs`, `voxel-von/src/judge.rs` —
  change `voxel_model::axis::` to `voxel_axis_ladder::`. A re-export from `voxel-model` is **refused**: it would
  make the model depend on the ladder crate, which depends on the model. One owner, and the compiler names every
  call site.
* `voxel-package/src/bloat.rs` **keeps** `Bloat`, `measure_chain_file`, `read_chain`, `measure_dir` and
  `TestBloat`, and **re-exports** `Prose`, `measure_prose` and `comment_lines` from `voxel-axis-artifact`, so
  `census::check_prose_frame` and the build door read the same instrument without a second definition. Its header
  gains one dated line saying where the measure lives now; `TestBloat` stays because it measures tests, not API
  prose, and moving it would be a second subject in the artifact layer.
* `voxel-mcp/src/lib.rs`'s served-tool list (`TOOLS: [&str; 9]` and `row_of`) **moves to `bridge::SERVED`**, and
  `voxel-mcp` re-exports it. **The served surface is a gate fact, not a server fact**: it is what the chokepoint
  permits an agent to call, and the server is one voice for it. This is also what lets the API window read the
  surface without linking the model carrier. `voxel-mcp::api_view` keeps its behaviour and becomes a call to
  `apidoc::render_surface(&SERVED)`, so the document the window shows has **one** owner.
* `voxel-package/src/apidoc.rs` gains `render_surface`, rendering a served row as `tool · row · action ·
  capability · permission class`. It is the same calibration rule as `render`: every word comes from a
  declaration, and `check_against` runs over the surface document too.
* `voxel-model/src/civilian/` (eleven files, 5 471 lines) and `voxel-package/src/civilians.rs` (698 lines) **move**
  to `crates/axis/civilians`; `voxel_model::civilian` is deleted (0 external call sites, measured), `voxel_package::civilians`
  re-exports the moved reader, and `decision::rule_for_civilian` stays in the gate where the rule belongs.
* `voxel-package/src/features.rs`'s `Establishment` moves to `crates/axis/ladder` as the `establishment` ladder and
  is re-exported by `features` as a type alias. Five variants, five tokens; the census's cells keep their values.
* `clean-room/src/axis.rs` is **not** touched here. It is a separate workspace with no path dependency; its port
  is a later unit that will pin these crates by digest, and this record names it rather than silently leaving a
  fourth copy.

# The axis set: nine ladders, and where every token comes from

`AxisKind` grows from 7 to 9. Two are new; one is widened; six are unchanged.

| axis | tokens | source of the declaration |
|---|---|---|
| `evidence_grade` | A B C D | `voxel-model/src/axis.rs` (rules 19.1, 51) |
| `certification_tier` | bronze silver gold diamond platinum | same (`TIER_REQUIREMENTS`) |
| `procedural_gate` | pass fail not_applicable expired blocked | same (`GateStatus` over `ProceduralGate`) |
| `grade` | F E D C B A S SS SSS | same (rule 19.2) |
| `org_rank` | applicant … chief_executive_officer (14) | same (organizations rules 101–108) |
| `support_level` | direct strongly_supported supported inferred self_reported unverified | same (ala-system `support.ts`) **and** upstream `planet-axis` declares the identical six — no merge needed, and the patch records that agreement rather than inventing a seventh |
| `ticket_evidence` | **A B C D** — oracle, read_back, self_report, **derived** | this tree's `ticket::EvidenceOrigin` (A/B/C) **widened by one rung**: `derived` (D) is the scrubber's own origin (`prose_logic` records `origin: "mechanical_derivation"`), and a rung with no producer would be a token nobody can cite. Upstream's `executed` and `declared` are **seen and ruled out** here: nothing in this tree executes or declares evidence, so admitting them would be two tokens with no subject — named in §Not done, not silently dropped |
| **`prose_kind`** (new) | definition obligation prohibition permission fact principle condition procedure | `prose_logic/src/claim.rs` `AtomKind`, 8 labels. **No `Ord`, no order**: it is a kind, like an origin, and the ladder's `DESCRIPTIONS` may not rank one against another |
| **`establishment`** (new) | active inactive stale unchecked refused | `voxel-package/src/features.rs` `Establishment`, 5 states; the census's own cells already carry them, so the axis is a name for a value that exists rather than a new value |

**The pin is compile-checked twice.** `COVERAGE` is `[CoverageRow; 9]`, `COVERAGE_IS_TOTAL: [(); 9] = [(); COVERAGE.len()]`,
and `AxisKind::declared_values` is an exhaustive `match` — a tenth axis with no ladder does not compile, and a ladder
with no coverage row does not compile. `check_coverage` then compares each row against **the axis's own declaration**,
never against itself, and every row names the document that states its tokens.

**The bare-token rule widens with the set.** `bare_token_axes("A")` already names three ladders; it now also has to
answer for `direct`, `fact`, `active`. `ambiguous_tokens()` is the table the gate prints, and the ladder crate's
test asserts one token nobody claims (`gold`? no — `gold` is declared) and one token two ladders claim, by name.

# The scrubber: four readers, three record types, one document

**Reader 1 — public items and their rustdoc.** `voxel-axis-artifact` walks `crates/**/*.rs`, and a line-based
parser (no `syn`; **no new dependency in this patch**) records each `pub` item as `Item { path, line, kind, name,
doc }`. Its limits are declared, not discovered later: it does not see items made by macros, it does not read
`pub(crate)`, and a doc comment attaches to the item that follows it. `//!` blocks attach to the module.
An item with an **empty doc block is not a claim** — a claim needs a quote — it is a **gap** with `path:line`.

**Reader 2 — the rendered API document.** `voxel-axis-verify` calls `voxel_package::apidoc::render` for each
package solution and hands the text to `artifact`. The scrub crate never imports `voxel-package`; the handoff is
a `(name, text)` pair, which is `bloat::measure_prose`'s own convention (*"already read"*), so there is no second
parse of the same bytes.

**Reader 3 — `docs/*.md`.** Whole files, read once, measured by the same `Prose`, tiled by paragraph.

**Reader 4 — chain prose.** `voxel-axis-verify` reads `chains/*.chain` through `bloat::read_chain` /
`measure_chain_file`, so the byte split stays `Bloat`'s (one owner), and hands the event `detail`s to `artifact`
for quoting. The report prints bloat's `ledger-bloat-linkage` caveat beside any link count, verbatim — the number
was never printable alone and the scrubber does not get to print it alone either.

**The record types are `prose_logic`'s, re-derived here** (not copied; the four gates are the specification):

* `Unit` — the addressable slice, with a deterministic `unit_<sha40>` id.
* `Claim` — one per unit, `quote == unit.text` **exactly** (support is `direct` only when it is), `kind` with a
  non-empty `kind_basis`, `origin = derived`, and an id `claim_<sha40(unit, normalised-rule)>`. **There is no field
  for a claim's opinion of its own support**, and `derive` is the only constructor that writes a support level.
* `Document` — the view: sections whose every line cites a claim id. A line that cannot cite one is refused;
  that is `apidoc`'s own lesson (a document that explains itself is a document that has started summarising).

**The partition audit is the "missing pieces" engine.** Every unit tiles its artifact's characters; `tile` refuses
a gap, an overlap, an empty unit and a unit past the end, **by position**, so a dropped paragraph is a named
failure rather than a shorter document. The audit is exercised on a fixture whose middle paragraph is deleted
(bite) and on the real tree (report).

**Gaps are typed, not strings**: `UndocumentedItem { path, line, name }`, `BareToken { line, token, axes }`,
`Unmeasured { source, why }`, `LocatePartial { located, compiler_count }`. The count of undocumented items stays
the **compiler's** (`conformance/missing-docs.txt`, 2 285, pinned): the scrubber **locates** gaps, it does not
recount them, because two instruments on one question is exactly the defect the record keeps convicting. When the
parser locates fewer gaps than the compiler counted, the report says `partial` and prints both numbers.

**The bare-token gate has a subject narrow enough to have no false positives**: only a **structured line** (whose
key is followed by a colon and whose value is a single word) can be refused, and only when that word is a token
`bare_token_axes` names and the key is not that axis's own name. `- grade: SSS` passes; `- SSS` is UNQUALIFIED;
`- A` is AMBIGUOUS. Running `refuse_bare` over free prose would convict the indefinite article; the gate is stated
on structure for that reason, and the reason is in the module header.

# The security layer: the trust boundary, measured as cells

`voxel-axis-security` has **no new ladder**, and that is a decision rather than an omission. Every check it runs has
one of the five `establishment` states as its honest answer, so it produces `SecurityReading { check, cell, at,
why }` on the ladder that already exists. A tenth ladder invented for this layer would be a vocabulary with one
reader.

| check | subject | refuses |
|---|---|---|
| containment | a path handed to `space.rs` / a reader | a path that leaves the workspace root, by position |
| symlink | the walking readers (`artifact`, `bloat`, `census`) | a followed directory symlink, which is an unbounded walk |
| one record | `chains/` | a second `.chain` file, and an invented `.jsonl`/`.ndjson` beside it |
| grant | every write the surface offers | a `Step` act presented with no `Grant`, and a read with no `observe` |
| capability | a `Session` | an act its four capabilities do not cover (`Observe`, `Step`, `Snapshot`, `Present`) |
| pin | `conformance/missing-docs.txt`, `UPSTREAM_PIN` | a pin that no longer matches the bytes: `stale`, never a pass |
| secret | `crates/`, `packages/`, `docs/` | a private-key header, a `.env` file, or a connection string with credentials in it (`mongodb://`, `mongodb+srv://`) |
| admission | `voxel-package::intake` | a package whose claim is `ChecksumOnly` reported as identity proven |

**The secret check prints where and what shape, never the value.** A report that echoed the connection string in
order to convict it would be the leak. The reading carries `path:line` and the shape's name; the value is replaced
before the line is ever stored, and a test asserts the group did not survive.

**The MongoDB sentence, grounded in the reference the user pointed at.** The design reference is
`shploingus-game/references/design_reference` (`GOD_AGENTS.md` 51 273 lines, the eleven `docs/GRILLING-C*.md`
sessions, 15 189 lines), and its own words are narrower than "add a Mongo driver": MongoDB **is** the store layer
that cannot be bypassed by application code (§9.2), the validators are `$jsonSchema` — `bsonType: object`,
`required`, `additionalProperties: false` — and the reference **emulates the server rather than deploying one**:
*"No MongoDB server runs. The schemas are emitted and locally enforced"*, because *"a validator that only exists
inside a deployment is a validator nothing in this repository tests."* The collections are append-only
(`civilians`, `contracts`, `tickets`, `evidence`, `promotions`, `retirements`, `seasons`), discriminated by a
`record` field, and the write guards refuse **unknown kinds, missing required fields, self-authorization,
never-granted actions (`create_account`, `enter_credentials`, `move_money`, `accept_terms`, `sign_contract`), and
any write into a protected doctrine root**. Two enforcement layers, and both are named: the application's
`extra="forbid"` models, field-echo guards, immutable fields and a support level that **discards an asserted
strength**, and the store's schema plus guards. Identity stays **procedural** (rule 100): an `agent_key_id` is a
name in a field, never proof, and C18 (*every key issued once*) stays `NOT_EVALUABLE` from the civilian store
because it records a boolean rather than the key.

So this layer does exactly that, and no more: `JSON_SCHEMAS` is emitted as **data**, `enforce_schema` and
`store_write_guard` apply the same rules locally, and `SecurityStore::guard` is the only path in. **This tree's
civilians are `voxel-model/src/civilian/` (eleven files) and their store today is the governed book**
(`world/book.rs`, `DeltaStore`, `apply_transition`) — `docs/CIVILIAN-PORT.md`'s store row says so in one line:
*the store: the chain*. A real deployment, with a URL from the environment and never from a file in the tree, is
**owed** and named in §Not done; inventing a driver here would be a second store before the first one's seam
exists, and the reference's own ruling is that the schemas are the artifact until a deployment exists.

# The MCP layer: the surface agents speak, and what it may do to laya, von and the civilians

**The surface is 23 rows and 9 served tools, and the difference is a finding.** `bridge::TOOLS` is bijective onto
`Action` (23). `voxel-mcp::TOOLS` serves nine (`get_work_list`, `get_conformance`, `get_census`, `get_patch_set`,
`get_ownership`, `get_test_bloat`, `get_api_view`, `judge_work`, `reject_non_conformance`), each mapped to one
bridge row. `docs/TODO.md` row 8 says *"no test asserts which 8"* — the count moved to 9 and the test did not
exist. This layer is that test, and the 14 unserved rows are printed by name as **owed**, not quietly absent.

**A session is capabilities plus a grant, checked twice.** The client (`Session::call`) refuses an act its
capabilities do not cover **before** it writes a byte; the server re-runs `bridge::call` — coverage, then
capability, then the one matrix — and returns a `Receipt` naming which of the three predicates ran. The client's
check is a courtesy to the agent; the server's is the gate. A test drives a call that skips the client check through
`serve` and asserts the receipt's refusal, so *"the client is not the gate"* is measured rather than promised.

**One rule this layer must not break, and it comes from the being doctrine.** `docs/CIVILIANS.md` §3.2: *"Agents are
autonomous and stateless; MCP carries control and observation of the world, never a puppet wire per living being."*
So there is **no per-being session** and no tool that drives a civilian step by step: the session is the agent's own,
the civilians are stepped by the host crate, and the MCP layer observes or asks — `read_world`, `judge_work`,
`get_api_view`. A tool that would pilot a being over the wire is refused by the layer's own rule, not by taste.

**The CPU agents, named per tool.** `judge_work` is the laya path: `bridge::call` → `verify_package` →
`voxel-von`'s lazy session, and *lazy* is measured — a session that only reads the census never loads the model.
A von answer carries `Origin::Oracle` (the `A` rung) and is evidence, not a verdict; laya decides. The civilians
are reached through `read_governance`/`read_world`, which read the governed record — the book's digest and its
transition count are what the report prints, so *the civil record works* is a measurement.

**Choosing work and organising by need is a gate, not a scheduler here.** The grilling sessions are the
specification: C9 Q19 fixes what a task is — *"nested specification → tasks, work on the task"*, with *"real
verifiable work done atomically on each task"* — and C9 §138 restates it as *"tasks are atomic, nested and visibly
verified"*; Q15 extends needs to *soft needs* (food, death, disease) that a civilian's life carries; C1 line 160
replaces `CitizenState` with *needs, skills, illness, wealth, home, job, life stage*, so **needing** is the
civilian's own state and not an assignment. `civilian/agency/selection.rs` already declares `SelectionPlan`,
`SelectionBudget` and *estimate-before-work*; `agency/task.rs` and `sim/task.rs` carry the tasks. So this layer
**verifies** a selection: every chosen task cites the need it serves, the plan's spend is within its budget, the
estimate exists before the work, and the work is atomic — a task that cannot be verified unit by unit is not a
work item under Q19. It does not choose. A plan missing any of the four is refused by name, which is what makes
*"choose work and organise according to their needs"* checkable instead of aspirational.

**Owed tools, named:** `ask_von`, `decide_laya`, `read_civilian_book`, `apply_civilian_transition`. Each needs an
`Action` variant, a `bridge::TOOLS` row, a `Capability` and a `policy::permission_of` class before it may be served
— and adding a row is a matrix ruling, not a code edit. They are listed here so the gap is a work item rather than a
surprise in the window.

# The civilians layer: one crate for the systems, and not the whole thing

The user's ruling, twice: *"all of those should merge into this one axis crate"* and then *"No the civilians systems
are one crate not the whole thing"*. So `voxel-axis-civilians` is **one crate holding the civilian systems and their
ladder reader**, and nothing else is folded in.

**What moves into it, measured:** `voxel-model/src/civilian/` — eleven files, **5 471 lines** — which `mod.rs`
describes as *"the civilian process, ported whole from ala-cities. Not re-derived"*, copied byte for byte under the
2026-10-02 ruling (*"civilians need the entire process … Reducing the civilians scope is not allowed"*):
`agency/action.rs` 1 576, `actor.rs` 467, `evidence.rs` 55, `selection.rs` 382, `task.rs` 64, `gov/mod.rs` 63,
`gov/city_reference.rs` 1 823, `gov/season.rs` 280, `sim/citizen.rs` 159, `sim/task.rs` 429, plus the module header.
With them goes the **reader** — `voxel-package/src/civilians.rs` (698 lines), `LadderObservation` and `TicketRow`,
which reads each ladder's witness and the board's citations — because it is a reading *of these systems*, not of the
package gate. The move is clean: `grep -rn "voxel_model::civilian" crates` finds **0** call sites outside the
module itself.

**What stays where it is, and why the crate is not "the whole thing":**

* `voxel_model::world` — the governed book, `DeltaStore`, `apply_transition`, `ActorId`, `PhysicalVerb` — stays in
the model. It is the world's truth, not the civilians'; the civilians write **through** it, and moving it would be
moving the world to move the civilians.
* `voxel_model::ticket` (`EvidenceOrigin`), `matter` and `materials` stay; the civilians import them.
* `voxel-package::decision::rule_for_civilian` **stays** in the gate. The seam pair is *read, then rule*: the crate
  reads (`LadderObservation`, `TicketRow`) and the decision layer rules, exactly as `census::observe` reports and
  `rule_for_axis` decides. `voxel-package` gains a dependency on the crate; the crate depends on nothing in the gate.
* `voxel-shapers::earthworks` stays the commit point for a modification, and `voxel-work`/`stream` stay the
  executors. The civilians *plan and select*; they do not mesh, stream or draw.

**The gates this crate exists to state** (each is a refusal, and each has a live subject in the moved code):

| gate | the systems' own rule, quoted |
|---|---|
| work is atomic | `sim/task.rs`: *"Tasks: the atomic, verifiable unit of work a citizen does (C9 round 11)"*, a **closed verb list plus a declared target** — Build, Gather, Make, Maintain today, and *"the verb list is closed on purpose and grows by decision"* |
| material is planned before work | the same header: *"Its material is planned before the work starts … The plan is atomic, so a task that exists can be paid for"* |
| completion is a check, not a claim | *"Completion is a defined check. The verb decides what completion means, and for `Build` it means the material was drawn **and a structure stands**. A task cannot be completed by declaring it complete"* |
| a selection has a need, a budget, an estimate | `agency/selection.rs`: `SelectionPlan`, `SelectionBudget`, `SelectionEstimate`, `estimate_selection_bounded` — *estimate-before-work* |
| a need is the citizen's own state | `sim/citizen.rs`'s state and the C9 Q15 *soft needs*; C1 line 160 replaces `CitizenState` with *needs, skills, illness, wealth, home, job, life stage* |
| every ladder has a witness | `civilians.rs`: one `LadderObservation` per ladder, `readable` separate from `present`, and one `TicketRow` per citation whose cited digest is checked against the bytes on disk |

**And the gap the crate must keep saying.** `civilians.rs`'s own header: *"The civilians' own books are still not
read"* — the rows are the board's tickets, not a citizen's claim, and *"when that lands this reader gains a third
reading rather than being quietly widened"*. The moved crate carries that sentence with it; the third reading is
**owed**, not implied by the second.

**The store.** Today the civilians write through the governed book (`world/book.rs`); the reference's MongoDB is
**emulated** by the security layer's `$jsonSchema` data plus local guards, exactly as the reference does it, and no
server is attached. `voxel-axis-civilians` therefore depends on `voxel-model::world`, not on a database.

## Ghosts are an entity type; the absent subject is a different thing

**Two different things were being called “ghost”, and this record keeps them apart** — the tree's own rule is that
one word may not mean two things (C-73 §73.1).

**1. The ghost is an entity type** — a being, drawn in the world, with its own spiritual energy. The user's ruling:
> *“ghosts again are a concept outlined in planet shploingus it was never put in but it was asked about”* and
> *“ghosts are real entities drawn in the world which are an entity type which has it's own spiritual energy.”*

The reference's being doctrine is `docs/CIVILIANS.md` (372 lines): **being** is the shared living substrate,
**agent** is what acts with intent, and the six faces are civilian · pilot/player · god · mob · plant · incarnate
(*“a face is a specialization, never a privilege”*). A ghost is **not one of the six** — it is the seventh face that
was asked about and never landed, declared here for the first time, and the affect layer is where its energy lives. `docs/AFFECT.md` (120 lines): every
being, plant or agent has an integer wheel position (`angle` on a 432-step turn, `radius` 0…65535) that reads as a
twelve-band spectral `Signature`; there is *no discrete intensity scale*, and *“when you look into a being's eyes you
should be able to read its soul, and if the screen cannot render the part of the spectrum that carries that reading,
the reading is still computed and kept.”* The reference's own capture carries a ghost as a bearer:
`logs/dev/affect-drift.txt` — *“ghost read Sad at tick 0 (#5500FF)”*, band `hard-uv`, radius 40 000, drift −4 — and
the wheel skill states the reading's medium: *“Ghosts and souls are read the same way”* (infra-red). So a ghost's
energy is a **spectral signature**, not a score.

`voxel-axis-civilians` therefore declares:

* `Face` = Civilian · Pilot · God · Mob · Plant · Incarnate · **Ghost**;
* `SpiritualSignature { angle: u16, radius: u16, bands: [u16; 12] }`, integer, on the being's record, and **drawn**:
  the renderer reads the bands through the appearance path, and the signature is kept when a band falls outside the
  screen's gamut — the reading is computed even where it cannot be shown;
* **open, and asked rather than decided:** whether a ghost is an *agent* or only a being (a plant is the precedent for
  being-only). The six faces do not answer it, so this patch does not invent an answer.

**Where these two declarations land, amended by `PATCH-043`.** The seven-face `Face` and the
`SpiritualSignature` are declared in **`voxel-axis-being`**, not here: a ghost is not a civilian and
the first mob is not one either, so every face's shared substrate belongs under every face rather
than inside the civilians' crate, which then depends on it. The doctrine, the gates GH1–GH5 and the
evidence in this section stand unchanged; only the declaration site moves, and `PATCH-043` records
why a tree needed the same crate a ghost did.

**2. The dangling reference is not a ghost.** What the reference calls `RefusedGhostCivilian` (`GOD_AGENTS.md:16252`,
`16630`) is *a record naming a civilian who does not exist* — *“the one that would have been missed, and the one that
matters most”*, because a clicker game's cheapest growth path is to invent a person. Because “ghost” now names an
entity here, the refusal is called **`AbsentSubject`**, and it is held two ways: **structurally**, a record naming an
absent subject emits nothing (no row, not a hole in a list); and **by a named refusal that fires**, because *“the
function returned a shorter list”* is a weaker assertion than *“the refusal fired”*. Upstream's gate states the same
rule at the ledger: *“unknown civilian … no grant on the ledger names them”* (`planet-gate/src/civil.rs:376`).

**“Created and cannot be destroyed by the system” is an absence gate, not a feature.** The reference says it
outright: *“Nothing can kill a being”* (`docs/CIVILIANS.md:118`). This tree already has the shape: `actor.rs`'s
`LifecycleState::Retired`/`Dead` are **states**, `retire(reason)` is an explicit transition, `RetirementReason` has
its fourteen declared reasons, and `city_reference.rs:14` says *“a building is retired, not deleted.”* A grep of the
civilian modules finds `retire_actor`/`retire` and **no** `remove`, `delete` or `destroy`. The guards:

| # | gate |
|---|---|
| GH1 | an absent subject emits **no row** — a selection, ticket or ladder row naming a subject with no creating record produces nothing, and `absent-emitted` prints **0** |
| GH2 | the absent subject is a **named refusal** — `AbsentSubject` exists, fires, and names the subject |
| GH3 | **no destruction path** — an absence suite over the crate's public API: no `remove`/`delete`/`destroy` on a being, `retire` keeps the record, `Dead`/`Retired` are states |
| GH4 | creation precedes reference — every being has a creation record with a digest; a reference without one is an absent subject (this is `Exercised`'s counterpart, **`Created`**) |
| GH5 | a ghost is drawn from its signature — a ghost with no signature is refused by name, a band outside `0..=65535` is refused, and the drawn entity cites the being record it came from |

**Mobs are owed, and named.** The authority is `agent_god_files/reference/CIVILIANS.md` (372 lines): a **mob** is a
being **and** an agent, with *mobility, needs, a life stage*; a **plant** is a being and not an agent. No module in
this tree declares a mob — `grep -rl "Mob" crates` finds only prose — and the clean room's first mob
(`automaton.rs`, `being.rs`, U12) is the existing subject. GH1–GH5 travel with it: a mob is created by a record,
retired explicitly, never removed by the system's bookkeeping.

## The loop the civilians run: mounting and compiling, both directions, in real time

The user's requirement, and it is older than this tree: *civilians are supposed to run the systems and through
itself — backward and forward real-time mounting and compiling*, which is what `planet-shploingus` was attempting.
So the crate is a **host**, not a bag of types: it owns the step that advances the civilian systems over the
governed book, and it owns the two directions of the mount/compile loop.

**What this tree already has, and it is more than the reference got:**

| direction | the mechanism today | where |
|---|---|---|
| forward, build time | `recipe::compile` turns a `.shspec` into package bytes | `recipe.rs:1276` |
| forward, live | `Mount::load_package` + `Mount::reload(&Loaded)` replace a running arrangement | `mount.rs:1119`, `1917` |
| forward, from the record | a filed ticket is back-compiled, rendered, emitted and **the shell runs it** — the audited loop with a per-ticket grant | `mount.rs:1431–1465` (the console's `file`) |
| backward, from the chain | `compiler::read_back` reads the record off disk and `emit` writes a package | `compiler.rs:687`, `860` |

**What is missing, and what `voxel-axis-civilians` adds:** the **live arrangement is not a source**. `Mount` can
*apply* a `Loaded`, but nothing renders a mounted arrangement back into a `Solution`/recipe, so the loop is open at
that end: forward works, backward only starts from the chain, and neither runs continuously. The crate adds
`Arrangement::read(&Mount)` — the live panes, their windows, lanes and content kinds, read through the same
public accessors the drawing uses — and `Arrangement::write(&mut Mount, &Loaded)` as the forward half, so one owned
loop does both. Planet-shploingus attempted this with `planet-shell/src/{mount,main,fetch}.rs` and
`planet-package/src/{frame,shell}.rs`; those bytes are read as reference and pinned in §Evidence, and the loop is
re-implemented here rather than copied (P7 v2.0.0's rule).

**The gates, so "real-time" is a measurement rather than an adjective:**

| # | gate |
|---|---|
| R1 | **round trip is identity**: `read(mount) → write(mount)` leaves every pane's window id, lane, extent and content kind unchanged, byte for byte, for the alpha shell |
| R2 | **forward is atomic**: a package that would drop a pane is refused by name and the running arrangement is untouched; a good one replaces it |
| R3 | **no restart**: the process keeps its windows, focus and frame loop across a mount — the loop is measured on one `Mount`, not by rebuilding it |
| R4 | **forward latency**, `mount-forward-ms` ≤ 250 on the alpha package |
| R5 | **backward latency**, `mount-backward-ms` ≤ 50 for the same arrangement |
| R6 | **no invention**: the backward read cannot name a window the mount does not carry, and every line it emits cites the pane it came from |

**The order is the record's, not a scheduler's.** A forward mount is a gated act (`bridge::call`, `RequiresGrant`,
the grant scoped to the ticket — the console already does this); a backward read is a read and needs `observe`.
The civilians host the loop; they do not mint permission for it.

# The API window: computed at mount, drawn in the pane, cached

* **One document, every line citing its source.** The view's three claim sections — *Asked for*, *Declares*,
  *Gates* — come from the scrubber (`verify`); *Surface* comes from `apidoc::render_surface(&bridge::SERVED)`
  (the MCP layer) and *Trust* from the security layer's cells, as readings beside those sections rather than as a
  fourth and fifth `SectionKind`. The view's rule is the join: a line that cannot cite a claim or a reading is
  refused, so the window cannot draw a sentence no layer produced. **`Gaps` was deleted, not kept as a section no
  producer can fill:** classification reaches every kind it declares, and the tautological test that only restated
  the empty arm went with it. The view layer stays text-only; `voxel-client` converts lines to `Element`s, and the
  view crate depends on no UI crate.
* `ContentViews` gains `api: Option<Result<Report, String>>` and `Mount::set_workspace(root)` becomes the
  invalidation point: a new root drops the report, the first `ApiViewer` mount computes it once, later mounts reuse
  it. Nothing is written to disk, so no invented record appears beside the chain.
* `ContentViews::panel_for` gains `Content::ApiViewer => api_panel(report, pane)`. The placeholder is no longer
  reachable for this kind, and a test asserts the panel's first line is the document title rather than
  `declared_content_panel`'s declaration.
* The panel is bounded to its pane like the console's, and its last line is a **measured** tail: `… N line(s) more`
  where N is the report's own count. No scroll in this pass; the view's three sections are ordered Asked for ·
  Declares · Gates, with Surface and Trust rendered beside them, and the tail names what is not shown, so a bounded
  panel cannot read as a complete document.
* **When the walk cannot run**, the panel's first body line is the reason (no `crates/` under the root; a source
  that would not decode), and no count is printed — D107's rule, and the user asked for a window that shows
  documents rather than a window that shows an empty one.
* **Cost**: the first mount reads 306 files / 12.1 MB in the Python proxy's 105.6 ms; the Rust walk is expected to
  be no slower. The gate records `axis-scrub-mount-ms` and the acceptance is **≤ 250 ms**; if the measurement beats
  that, the fallback is to compute at `set_workspace` (before the first frame) rather than to cache a stale copy.

# What this fixes, and what it deliberately does not

**Fixes, in the user's sense of "the many missing pieces":** the API window has a document; the document is a work
queue (`UndocumentedItem` at `path:line` for the locatable subset of the 2 285); the prose measure has one home the
window may import; the axis vocabulary has one home instead of four; every ladder's tokens are coverage-checked;
the four prose-logic gates (tiling, computed support, recorded basis, degradation that keeps content) become
refusals with bite tests; **the civilian systems are one crate with a host loop** (forward compile-and-mount,
backward read-and-emit, both measured); the trust boundary and the agent surface are layers with named refusals;
**the census gains `Exercised`**, so a crate nothing calls can no longer read `active` on every axis; every new
crate enters the census.

**Does not fix, and says so:** the 2 285 undocumented items are *located*, not written; `missing-docs.txt`'s
`voxel-model` and `voxel-package` lines must be re-taken (their bytes move) and ten lines added (one per new
crate) by the `cargo check` recipe in `census.rs`'s header; `census::CRATE_COUNT` moves 16 → 26, which means ten
`Crate` variants, ten `ROWS` entries (each with its `means` sentence), and `DESIGN-SPEC.md` §2/§3 naming the ten
or the `Claimed` axis reads them `unclaimed`; and `census::AXIS_COUNT` moves 6 → 7 for `Exercised`, which fills one
more cell in every row and is found by the same compile-time pins. Those edits are mechanical, exhaustive matches, and every literal that
pins 16 is found by running the suite rather than by memory.

# Acceptance

Every gate below is stated as a command and a figure, and every guard is proven to bite by perturbing its own
subject — the tree's rule (`PATCH-041` §43), applied to ten new crates. The calibration matrix's gates
**C1–C6** are stated in *The calibration matrix* above and run in the same command as the rest, because a
matrix that is only checked by a separate suite is a second gate with a second opinion.

| # | gate | stated as | bite |
|---|---|---|---|
| A1 | the axis set is closed and total | `COVERAGE.is_empty()` false, `COVERAGE_IS_TOTAL.len() == 9`, `AxisKind::COUNT == 9`; every name round-trips | delete a row → **does not compile**, named |
| A2 | no fabricated row | `check_coverage(&COVERAGE)` is `Ok`; a forged `evidence_grade` row (`A B C E`) is refused by name | the forged row is the test |
| A3 | a bare token is a bug | `refuse_bare("A")` names three ladders; `refuse_bare("SSS")` says UNQUALIFIED; `refuse_bare("gold")` is `Ok` | a fixture line `- grade: SSS` is refused as AMBIGUOUS/UNQUALIFIED per the fixture |
| A4 | a ladder is never a number | `refuse_reduction` for all 9 axes × 2 reductions; `refuse_maximum_certification` names `[A, D, D, D]` | each refusal's `kind()` and a word of its detail |
| A5 | the partition is exact | `tile(chars, units)` is `Ok` on each real source; a deleted middle paragraph is refused `gap at N`; a doubled range is refused `overlap at N` | the fixture with a paragraph removed |
| A6 | the quote is the fact | every claim's `quote == unit.text`; a one-character change to a quote is refused by claim id | the changed quote |
| A7 | support is computed | `derive` is the only producer of a support level; a `Claim` built with `support: Some(...)` does not exist as an API | the constructor's signature is the test; the module says so |
| A8 | a kind carries its basis | every claim's `kind_basis` is `heuristic:<signals>` or `heuristic:default`; an empty basis is refused | the empty basis |
| A9 | the document is traceable | every `Line` resolves to a claim in the report; a line added without a claim is refused | the added line |
| A10 | ids are deterministic | two scrubs of the same bytes produce identical claim ids and an identical report digest; a changed byte changes the digest | the second run |
| A11 | the window draws the document | `ContentViews::panel_for` for `ApiViewer` returns a panel whose first line is `TITLE`; the placeholder's `no declared window is free to place here` is absent | the old `declared_content_panel` path |
| A12 | the walk's cost | `axis-scrub-mount-ms` printed on first mount, **≤ 250**; the figure is in the report, not in a comment | the printed number |
| A13 | gaps are located, not recounted | for each of the 17 crates, `located ≤ compiler_count`; when `located < compiler_count` the report prints `partial` with both numbers | a crate whose count is 0 but whose parser finds one item with an empty doc block → `partial` |
| A14 | upstream was read, not copied | `UPSTREAM_PIN` carries the three sha256 of `planet-axis` and a literal token table; every pinned token is declared here or named in §Not done | delete `derived` from `ticket_evidence` → A14 names it |
| S1 | containment | a reader asked for `../../etc` refuses and names the resolved path; `space.rs`'s own guards still pass | the fixture path |
| S2 | symlinks are never followed | a fixture tree with a directory symlink yields **0 followed, 1 recorded** | the symlink |
| S3 | one record | a scratch `chains/` with two `.chain` files is refused by name; `invented_stores` finds a planted `.jsonl` | the second file |
| S4 | capability and grant | a `Step` act with no grant is refused and the receipt names `matrix_permits`; a read with no `observe` is refused by `capability_held` | the receipt's predicate list |
| S5 | pins | a tampered pin reads `stale`, never `active`; an `unchecked` cell is not a pass | the tampered pin |
| S6 | secrets | a planted `mongodb+srv://user:pass@host` line is refused with `path:line` and the shape's name, and the report does not contain `pass` | the planted line |
| S7 | admission is not identity | a checksum-only package reports `ChecksumOnly` and never says identity was proven | the claim |
| M1 | the served surface is classified | every `SERVED` name has a `TOOLS` row, an `Action`, a `Capability` and a `permission_of` class; the 14 unserved rows are printed by name | delete one row → refused by name |
| M2 | the server serves exactly `SERVED` | `voxel_mcp::TOOLS == bridge::SERVED`, and a tool served but not declared is refused | add a name to the server list |
| M3 | the client is not the gate | a call driven straight into `serve` with an uncovered capability is refused, and the receipt names `capability_held` | the receipt |
| M4 | the model is lazy | a read-only session loads **0** ONNX sessions; the first `judge_work` loads **1** — read from a `voxel-von` counter added by this patch (one `AtomicUsize`) | the counter |
| M5 | a selection is a need, a budget and an estimate | a `SelectionPlan` whose task cites no need, whose spend is over budget, or whose estimate is missing is refused by name | three fixtures, one per refusal |
| X1 | the seventh census axis measures callers | `Axis::Exercised` is `inactive` for `voxel-audio` and `voxel-instrument`, printing **0** callers, and `active` for `voxel-model` with a count above zero | add one call site in another crate → the cell turns `active` |
| X2 | a re-export is not a call | a fixture whose only mention is `pub use other::thing;` reads **0** callers | the fixture |
| X3 | the civilians' absence is a cell, not a sentence | the report prints `civilian-step-callers`, **0** on the base bytes — no crate calls the systems' step — and **1** once the host loop's caller lands; the number is printed, not implied | the printed 0 → 1 |
| GH1 | an absent subject emits nothing | a fixture job naming an absent civilian yields **0** rows and `absent-emitted 0` | the fixture's row count |
| GH2 | the absent subject is a named refusal | `AbsentSubject` exists and fires for that same subject, naming it | the refusal |
| GH3 | the destruction path is absent | the absence suite finds no `remove`/`delete`/`destroy` on a being; `retire` keeps the record and `Dead`/`Retired` are states | the suite's failure when a `delete` is added |
| GH4 | creation precedes reference | every being has a creation record with a digest; a reference without one is an absent subject | a fixture reference with no creating record |
| GH5 | a ghost is a drawn signature | a ghost with no signature is refused; twelve bands in `0..=65535` draw; the drawn entity cites its being record | the missing-signature fixture |
| M6 | the store is measured | the report prints the book's digest and transition count; a store seam with no backend reads `unchecked`, never zero | the printed digest |

Commands, and the figures they must print: `cargo test -p voxel-axis-ladder -p voxel-axis-coverage
-p voxel-axis-artifact -p voxel-axis-unit -p voxel-axis-claim -p voxel-axis-view -p voxel-axis-verify
-p voxel-axis-security -p voxel-axis-mcp -p voxel-axis-civilians` → every guard above by name, 0 failures; `cargo test -p voxel-package -p voxel-client -p voxel-von` → the existing suites green
through the re-exports, with the crate count 26 and the two re-taken pins; `cargo check -p shploingus` → exit 0;
`cargo run -p shploingus` (window) → the ApiViewer pane's first line is the title, the last line the measured tail.

# Evidence

* `base_digest` is `sha256` over the 255 tracked `.rs` files plus the four manifests, **as they stand on disk**
  (`git ls-files 'crates/**/*.rs' … | sha256sum`), not over a commit: the working tree is dirty and a branch name
  is not a version.
* The four homes of the axis vocabulary, the six consumers of `voxel_model::axis`, the 62 mentions of
  `EvidenceOrigin` in 9 files, the 198 mentions of `Establishment`, and the 2 285 missing docs across 17 crates are
  counts from this tree, on these bytes.
* The upstream bytes this plan read are pinned:
  `planet-axis/src/axis.rs d4cd16792756cab71610e8782f785f09a7c7a97cce57ade2ba69340814c61891`,
  `refusal.rs d54bd4f326d5efa6109b94e84402c4ffbc1995d24ba761eb83f9bc86830777ba`,
  `lib.rs 02d0e88e4efa1c80b17c825a4bb38893412eb55f3d7c786ebcbe63120fbbaaab`.
* The prose-logic bytes that supply the four gates and the two new ladders are pinned:
  `artifact.rs 1f71bcc1291653d925a9b6dcffadb493d7a03895d1add61aa5b2ceecc7a63fac`,
  `unit.rs becef9c0fdae60fccd0c6d3af34236db8adb1eda308b46d436cc9cacd564926c`,
  `claim.rs 1277e61100745707b2e4d5b839b54afeb5514e8e5cf68985731ac0fdbbf92521`,
  `view.rs 06f22d071ffe592c07f7c328869852e7d3dd94dd1a36d027b2e90caa514e0925`.
* The walk's cost is a measured proxy, not an estimate: 306 files, 12 149 969 bytes, **105.6 ms**, by a cold
  Python `open(...).read()` over the same four patterns the scrubber will walk.
* `apidoc`'s only consumer is `voxel-package/tests/first_ticket.rs`; `ContentViews::panel_for` has no `ApiViewer`
  arm; `voxel-mcp::TOOLS` serves **9** of `bridge::TOOLS`' **23** rows. All three are `grep` results on these bytes,
  and all three are the finding this plan answers.
* **The design reference the user pointed at is pinned by path and digest**:
  `shploingus-game/references/design_reference/GOD_AGENTS.md` (51 273 lines)
  `a18d307b704bc95c87fd721060b3e2569de9acdcc574eba7785334eab6d1782e`,
  `docs/GRILLING-C9.md` (1 989 lines) `53d184b4a782a617e4af86902a9de8423c9e0bafa49c6d87cb3f58914c228a26`,
  `docs/GRILLING-C1.md` (318 lines) `8ad91bececf2bc6b9fce9d9a9597002e03463594a3bdbb13a6eeaf6b1c3afd93`.
  The quotes this plan leans on are the reference's, at their lines: *"No MongoDB server runs. The schemas are
  emitted and locally enforced"* (`GOD_AGENTS.md:39659`), the `$jsonSchema` substitution (`39615`), the collection
  list and `record` discriminator (`50121`), the write guards (`50145`), rule 100's procedural identity (`39651`),
  C9 Q19's atomic nested work (`docs/GRILLING-C9.md:113` and `138`), and C1's `CitizenState` replacement by needs
  (`docs/GRILLING-C1.md:160`).
* **The being and ghost reference is pinned the same way** (`F:\planet-shploingus`): `docs/CIVILIANS.md`
  `a38f8f1618bfd28b6a0ffd1bfa7a6b4bb924091cdd4132aff0e197fbbc0696a6` (the six faces, *"nothing can kill a being"*,
  the no-puppet-wire rule), `docs/AFFECT.md` `ebd3a0d4289df1cfc16dd4fdec2e4fe5b1e04338e7825f6cf47e2dc84f446a75`
  (the wheel, the spectral read, *"the reading is still computed and kept"*), `planet-affect/src/spectral.rs`
  `c5ba27e1fd378dddfbfc2baf5555070f2506618097609d0fa8907dba7fbfdb26`, `wheel.rs`
  `b4fff13104adfc7e7bf14fec6fcc5f4397f2f07a96596bedb7070ab3ac3e8c15`, `labels.rs`
  `b7e2db76f8f458d8c5600021715a3990f730fa12eddbca877f715d03ca179a5b`, and the ghost bearer's own capture
  `logs/dev/affect-drift.txt` `5eb83df9f3b9925bdedc76cde44c09fa303db079bc8e982ee7c20a77106bbb80` (*"ghost read Sad at
  tick 0 (#5500FF)"*, band `hard-uv`, radius 40 000, drift −4). `agent_god_files/reference/CIVILIANS.md` (372 lines)
  is the mobs authority and was read for the face table.

# Risks

1. **The line-based item parser will disagree with rustc.** That is why A13 compares to the compiler's count and
   says `partial` rather than claiming completeness; a parser that reports fewer items is a locator with a stated
   reach, not a second census.
2. **Moving `Prose` changes `voxel-package`'s bytes and therefore its `missing-docs` pin.** The gate will read
   `stale` until the pin is re-taken; the plan takes two pins and adds ten lines, by the recipe, and does not
   hand-edit a count.
3. **`CRATE_COUNT` 16 → 26 touches every literal that pins sixteen.** They are exhaustive types and tests, so the
   compiler and the suite find them; no assertion is weakened, and any count that changes changes to the new total
   with the reason in the patch.
4. **The mount walk can hitch the first frame.** A12 bounds it at 250 ms and the fallback is `set_workspace`, named
   in advance rather than discovered on a slow machine.
5. **Nine ladders make more tokens ambiguous** (`direct`, `fact`, `active` and the letters). The bare-token gate is
   the rule that turns that from a hazard into a refusal, and A3 exercises both branches.

# Not done, and named

* `clean-room/src/axis.rs`'s 811-line copy is not moved, re-exported or deleted; the clean room is a separate
  workspace and its port is a later unit that pins these crates by digest.
* Upstream's `executed` and `declared` origin rungs are **not** admitted: no instrument in this tree produces
  either, and a rung nobody can cite is a token with no subject. If a later unit builds an executing oracle, the
  rung is added with its producer in the same patch.
* The 2 285 undocumented public items are located, not documented. Writing them is a separate, much larger unit,
  and the report is its work queue.
* No scroll for the API window in this pass; the tail line is the honest bound, and scrolling is the next rung.
* The MCP surface does not expose the scrubber yet; `bridge::TOOLS` stays at 23 rows and the served list at nine.
  The four owed tools (`ask_von`, `decide_laya`, `read_civilian_book`, `apply_civilian_transition`) need a matrix
  ruling before a row exists, and this plan does not make one.
* **No MongoDB deployment.** The `$jsonSchema` documents are data and the guards are local, exactly as the
  reference does it; a real server — URL from the environment, never from a file in the tree — is a later unit.
  Nothing here connects to a database, and no driver is added.
* The MCP client speaks **stdio only**. No listening socket, no remote transport, so there is no new network
  surface for the security layer to guard yet; a network transport is a separate unit with its own threats.
* The civilians' **own books** are still not read: the moved reader's second reading is the board's tickets, and the
  third reading — a citizen's own claim — is owed with `world::book`'s in-memory books, exactly as `civilians.rs`
  says. This patch moves the reader; it does not widen it.
* `census::Axis` and the crate grid **stay in `voxel-package`**. The user's ruling was *one crate for the civilian
  systems, not the whole thing*, and the census is the gate's reading of the crates rather than a civilian system;
  its `Establishment` cells are re-expressed on the ladder's own type, which is the whole of its contact here.
* The selection gate verifies a plan; it does not produce one. Choosing work for a civilian remains the civilian's
  own process (`agency/selection.rs`), and this layer only refuses a selection that cites no need, exceeds its
  budget, lacks an estimate, or names work that cannot be verified atomically.

# Rollback

Delete `crates/axis/` (ten members), remove the ten lines from `[workspace] members`, restore
`voxel-model/src/axis.rs` from the base digest, restore the `Prose` block in `voxel-package/src/bloat.rs` and drop
its re-exports, restore `features.rs`'s `Establishment`, restore `ticket.rs` to three origin rungs, and revert the
`ApiViewer` arm in `mount.rs`, move `bridge::SERVED` back into `voxel-mcp`'s `TOOLS`, drop
`apidoc::render_surface`, and restore `census`'s `AXIS_COUNT` to 6 (dropping the `Exercised` cell from every row).
Re-take the two `missing-docs` pins. Nothing outside those files moved, and the base digest
restores the tree that was measured here.

# Update 2026-10-25 — the caller question is now a census axis (a slice, not the plan)

`census::Axis::Exercised` is landed: `census::exercised_callers` walks the covered Rust files once,
skips a crate's own sources, any inline `#[cfg(test)]` module, comment lines and `pub use`
re-exports, and records the first five call sites per crate. The axis is `active` when a call site
exists, `inactive` when none does, and `refused` when the crate has no readable source. On this tree
it reproduces the finding the plan states: `voxel-audio` and `voxel-instrument` are named, moduled,
tested, documented and **stepped by nothing**. The caller scan is line-based, so a macro-generated
call site is still invisible; that limit is stated on the axis rather than claimed away.

**The plan is not applied by this slice.** Six of its ten crate members remain absent from
`crates/axis/` (`ladder`, `coverage`, `unit`, `claim`, `view`, `civilians`); the `Prose` move, the
nine-ladder `AxisKind`, the `ApiViewer` arm and the `bridge::SERVED` move are still owed. The
`Exercised` cell is one gate of the record, proved through `cargo test -p voxel-package --lib
census::`, not the record's completion.

## Recipes

```recipe
segment: axis
axis: evidence_grade
ticket: T-0954
site: crates/axis/matrix/src/matrix.rs:323
what: keep `Matrix::recipe` refusing a cell the walk did not measure, by name, instead of rendering a line for it
because: a cell with no witness is unchecked and never a pass, and 7 segments plus the artifact format with 8 measured cells and 0 unchecked is the whole set a recipe may name
check: cargo test -p voxel-axis-matrix --lib -> failures 0
witness: crates/axis/matrix/src/matrix.rs
expects: 0
requires: cargo check --workspace
```
# Update 2026-10-25 — the dependency direction, decided, and the six prose layers landed

**The direction is `voxel-model` under every axis crate, not the axis vocabulary above it.** The
layer table and the dependency paragraph above already said *"`voxel-model` sits under all of
them"*, but the move beside it said `voxel-model/src/axis.rs` **moves** to `crates/axis/ladder` — a
direction the model's own consumers refuse. `admission`, `laya` and `proof` read the ladders, so
moving the vocabulary into a crate that depends on the model makes the model depend on that crate:
a cycle, not a layer. The decision is therefore the one that compiles. The line the six landed on
is `ladder ← coverage ← unit`, then `claim ← unit, ladder`,
`view ← claim, ladder`, and `civilians ← ladder, voxel-model`; `artifact` already
exists and is not a member of this line, so `unit` does not wait on it. The vocabulary **stays** in
`voxel-model/src/axis.rs`, and `voxel-axis-ladder` is its axis-layer name: it re-exports
`AxisKind`, the nine ladders, the coverage table and the refusals, and it names the two ladders
PATCH-042 asked for, which the model's vocabulary gained (`ProseKind`, eight labels with no order;
`Establishment`, active · inactive · stale · unchecked · refused), plus the
`EvidenceOrigin::Derived` rung (`D`), which `ticket.rs` already declares. `voxel-package`'s feature
rows re-export the model's one `Establishment` rather than keeping a second five-state enum.

**No axis crate depends on `voxel-von` or `voxel-mcp`.** A classifier's answer arrives as
`EvidenceOrigin::Oracle`, and a window that draws a ladder does not load an ONNX session to do it.
The civilians follow the same rule: the systems stay in `voxel-model::civilian`, and
`voxel-axis-civilians` re-exports and gates them, because the 5 471-line physical move would make
every axis crate depend back on the model's `world` vocabulary while changing no behaviour.

**The six layers are no longer absent.** `crates/axis/{ladder,coverage,unit,claim,view,civilians}`
are workspace members, census rows in `Crate::ALL` order, `DESIGN-SPEC` rows and
`conformance/missing-docs.txt` lines (all six read **0** missing items, re-measured, not asserted).
`ladder` 4 tests, `coverage` 6, `unit` 5, `claim` 6, `view` 6, `civilians` 5 — 32 against the
layer, all green. `voxel-axis-verify::prose::scrub` is the first finished reader: it tiles
`docs/DESIGN-SPEC.md` through `unit`, derives one claim per unit, validates the three-section
`Document`, and renders it twice to prove determinism. The paragraph in the previous update saying
six members "remain absent" was true when it was written; this one supersedes it.

**Where the six are reached.** The five prose crates are consumed by `voxel-axis-verify` in its
library, not only in tests: `evidence.rs:27` reads `voxel_axis_coverage`'s table, and
`prose.rs:25–27` reads `voxel_axis_unit`, `voxel_axis_claim` and `voxel_axis_view`; `voxel-axis-ladder`
is read by `coverage`, `claim`, `civilians` and `verify`. **`voxel-axis-civilians` is now reached by
`voxel-package::decision::rule_for_civilian`**: the reader in `voxel-package/src/civilians.rs`
supplies `readable` and `present`, and the gate consumes the crate's `LadderReading` to decide
`Held` / `Absent` / unreadable rather than deciding a second time. The census therefore no longer
names the crate an orphan; its `Arrangement` half still waits for the client-side `Mount` adapter,
which is the remaining owed edge. The prose reader is called only by its tests today — the API
window draws `apidoc::render`'s document and `verify`'s cells — so it is compiled into the library
but is not yet the window's document. That is an evidence gap, stated rather than counted as
reach.

**What is still owed, named.** The client-side `Mount` adapter for `voxel-axis-civilians`'
`Arrangement` is the owed edge; the ladder reading itself is consumed in the gate now, so the
crate is not an orphan. The prose walk reads one named document, not the whole `docs/` corpus, and states that
rather than counting a sample as the corpus. The physical relocation of the civilian systems is
refused for the cycle above, not postponed. The `Rollback` section's *"restore
`voxel-model/src/axis.rs`"* does not apply to this direction: that file was never moved, so the
rollback for these six is deleting the six directories and their workspace, census and line rows.

**Checks run, and their readings.** The six crates’ own suites — 32 passed, 0 failed
(`ladder` 4, `coverage` 6, `unit` 5, `claim` 6, `view` 6, `civilians` 5).
`cargo test -p voxel-axis-verify` — 21 passed, 0 failed.
`cargo test -p voxel-package --lib` — 392 passed, 0 failed, 1 ignored. `cargo check --workspace
--all-targets` — exit 0. The missing-docs count and pin recipes were both re-run after the last
edit; the six new rows read 0, and the moved pins match their bytes. The one real test failure this
pass found was the census's expected orphan list written out of `orphan_libraries`' own sorted
order; the expected list now follows that order. **`voxel-axis-civilians` left that list** when
`decision::rule_for_civilian` began consuming `LadderReading`; the set it pins is now `voxel-audio`,
`voxel-axis-balance`, `voxel-axis-being`, `voxel-instrument`.
**Re-checked for this record:** the pin recipe was re-run and all **29** filed pins match the bytes
on disk. Six pins moved with this pass's edits — `voxel-axis-unit`, `voxel-axis-claim`,
`voxel-axis-view`, `voxel-axis-civilians`, `voxel-axis-verify` and `voxel-package` — and every
count was re-taken with the compiler command and is unchanged: the six axis crates at 0, the
package at 340. `voxel-patch-recipes` reads **76 records, 29 blocks, 29 compiled, 1 derived,
1 derived-compiled, 1 rederived, 1 rederived-match, 50 gaps** — the prose derivation path is still
one record, and the gaps are the corpus's standing findings rather than new ones from this pass.
